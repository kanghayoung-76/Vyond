/*
 * split-host-test: sub-host — manages enc2 (subscriber enclave)
 *
 * sub-host creates the TDDS SHM, runs enc2 for its full lifetime,
 * and handles all enc2 OCALLs — including those issued after enc1
 * wakes enc2 via IPI in multicore mode.  No file-based signaling:
 *   - bridge-host starts immediately alongside sub-host.
 *   - enc1 polls for enc2's TDDS channel via find_shm_by_hash retry loop.
 */
#include <cstdio>
#include <cstring>
#include <cerrno>
#include <unistd.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <sys/ioctl.h>

#include <edge_call.h>
#include <keystone.h>
#include "SharedMemory.hpp"
#include "shared/keystone_user.h"
#include "shared/sm_err.h"

#define TDDS_SHM_SIZE  8192

#define OCALL_PRINT        3
#define OCALL_GET_TDDS_RID 5

#define KEYSTONE_DEV_PATH "/dev/keystone_enclave"

static uint32_t  g_rid_tdds = 0;
static uintptr_t g_shm_base = 0;
static size_t    g_shm_size = 0;

static void set_ret(struct edge_call *ec, const void *data, size_t dsz,
                    uintptr_t base, size_t bsz)
{
    uintptr_t area = base + sizeof(struct edge_call);
    memcpy((void *)area, data, dsz);
    if (edge_call_setup_ret(ec, (void *)area, dsz, base, bsz) != 0)
        ec->return_data.call_status = CALL_STATUS_BAD_PTR;
    else
        ec->return_data.call_status = CALL_STATUS_OK;
}

static void handle_get_tdds_rid(struct edge_call *ec)
{
    set_ret(ec, &g_rid_tdds, sizeof(g_rid_tdds), g_shm_base, g_shm_size);
}

static void handle_print(struct edge_call *ec)
{
    size_t len = ec->call_arg_size;
    uintptr_t ptr = 0;
    edge_call_get_ptr_from_offset(ec->call_arg_offset, len, &ptr,
                                  g_shm_base, g_shm_size);
    if (ptr && len > 0) {
        char buf[512] = {};
        size_t n = (len < sizeof(buf) - 1) ? len : sizeof(buf) - 1;
        memcpy(buf, (void *)ptr, n);
        printf("[sub-host] OCALL_PRINT len=%zu: '%s'\n", n, buf);
        printf("[sub-host]   hex:");
        for (size_t i = 0; i < (n < 32 ? n : 32); i++)
            printf(" %02x", (unsigned char)buf[i]);
        printf("\n");
        fflush(stdout);
    } else {
        printf("[sub-host] OCALL_PRINT: empty (len=%zu ptr=%p)\n", len, (void *)ptr);
        fflush(stdout);
    }
    ec->return_data.call_status  = CALL_STATUS_OK;
    ec->return_data.call_ret_size = 0;
}

static void dispatch_ocall(void *buf)
{
    struct edge_call *ec = (struct edge_call *)buf;
    switch (ec->call_id) {
        case OCALL_GET_TDDS_RID: handle_get_tdds_rid(ec); break;
        case OCALL_PRINT:        handle_print(ec);        break;
        default:
            fprintf(stderr, "[sub-host] unknown OCALL id=%lu\n", ec->call_id);
            ec->return_data.call_status = CALL_STATUS_BAD_CALL_ID;
    }
}

static int run_sub_enc2(int kfd, uintptr_t eid, void *shm_buf)
{
    struct keystone_ioctl_run_enclave args = {};
    args.eid = eid;

    if (ioctl(kfd, KEYSTONE_IOC_RUN_ENCLAVE, &args) != 0) {
        perror("[sub-host] ioctl RUN");
        return -1;
    }

    /* Handle OCALLs enc2 makes before it parks at wait_shm */
    while (args.error == SBI_ERR_SM_ENCLAVE_EDGE_CALL_HOST ||
           args.error == SBI_ERR_SM_ENCLAVE_INTERRUPTED) {
        if (args.error == SBI_ERR_SM_ENCLAVE_EDGE_CALL_HOST)
            dispatch_ocall(shm_buf);
        if (ioctl(kfd, KEYSTONE_IOC_RESUME_ENCLAVE, &args) != 0) {
            if (errno == EINTR) continue;
            perror("[sub-host] ioctl RESUME");
            return -1;
        }
    }

    if (args.error != SBI_ERR_SM_ENCLAVE_WAITING_FOR_SHM) {
        fprintf(stderr, "[sub-host] unexpected SM error: %lu\n", args.error);
        return -1;
    }

    /* enc2 parked at wait_shm.  Loop: wait for enc2 to be woken by IPI, handle
     * any OCALLs it makes, then wait for it to park or finish again. */
    printf("[sub-host] enc2 parked — entering WAIT_AND_RESUME loop\n");
    fflush(stdout);

    for (;;) {
        /* Block (with timer-preemptible WFI) until enc2 is woken by IPI and
         * either makes an OCALL or parks again. */
        if (ioctl(kfd, KEYSTONE_IOC_WAIT_AND_RESUME, &args) != 0) {
            if (errno == EINTR) continue;
            perror("[sub-host] ioctl WAIT_AND_RESUME");
            return -1;
        }

        /* Inner loop: handle OCALLs until enc2 parks (WaitingForShm) or exits. */
        while (args.error == SBI_ERR_SM_ENCLAVE_EDGE_CALL_HOST ||
               args.error == SBI_ERR_SM_ENCLAVE_INTERRUPTED) {
            if (args.error == SBI_ERR_SM_ENCLAVE_EDGE_CALL_HOST) {
                dispatch_ocall(shm_buf);
                printf("[sub-host] OCALL handled, resuming enc2\n");
                fflush(stdout);
            }
            if (ioctl(kfd, KEYSTONE_IOC_RESUME_ENCLAVE, &args) != 0) {
                if (errno == EINTR) continue;
                perror("[sub-host] ioctl RESUME");
                return -1;
            }
        }

        if (args.error == SBI_ERR_SM_ENCLAVE_WAITING_FOR_SHM) {
            /* enc2 parked again — loop back to WAIT_AND_RESUME */
            printf("[sub-host] enc2 re-parked, waiting for next IPI\n");
            fflush(stdout);
            continue;
        }

        /* enc2 exited or errored */
        printf("[sub-host] enc2 done (error=%lu)\n", args.error);
        fflush(stdout);
        break;
    }
    return 0;
}

int main(int argc, char **argv)
{
    if (argc < 4) {
        fprintf(stderr, "usage: %s <sub-eapp> <runtime> <loader>\n", argv[0]);
        return 1;
    }

    printf("[sub-host] creating TDDS SHM (%d bytes)\n", TDDS_SHM_SIZE);

    Keystone::SharedMemory tddsShm;
    g_rid_tdds = tddsShm.createEnclaveShm(TDDS_SHM_SIZE);
    if (!g_rid_tdds) {
        fprintf(stderr, "[sub-host] createEnclaveShm failed\n");
        return 1;
    }
    printf("[sub-host] TDDS SHM rid=%u\n", g_rid_tdds);

    Keystone::Enclave subEnc;
    Keystone::Params  params;
    params.setFreeMemSize(1 * 1024 * 1024);
    params.setUntrustedSize(64 * 1024);
    if (subEnc.init(argv[1], argv[2], argv[3], params) != Keystone::Error::Success) {
        fprintf(stderr, "[sub-host] enc2 init failed\n");
        return 1;
    }
    g_shm_base = (uintptr_t)subEnc.getSharedBuffer();
    g_shm_size = subEnc.getSharedBufferSize();

    /* Share TDDS SHM with enc2 so it can map it for create_subscription */
    tddsShm.shareShm(g_rid_tdds, subEnc.getEID(), 7);
    printf("[sub-host] enc2 EID=%d, TDDS rid=%u shared\n", subEnc.getEID(), g_rid_tdds);

    int kfd = open(KEYSTONE_DEV_PATH, O_RDWR);
    if (kfd < 0) { perror("[sub-host] open keystone"); return 1; }

    printf("[sub-host] running enc2 → will park at wait_shm\n");
    fflush(stdout);

    int rc = run_sub_enc2(kfd, (uintptr_t)subEnc.getEID(), subEnc.getSharedBuffer());
    close(kfd);
    return rc != 0 ? 1 : 0;
}
