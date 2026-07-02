/*
 * split-host-test: bridge-host — manages enc1 (publisher/bridge enclave)
 *
 * enc1 finds the TDDS channel autonomously via SM hash-based attestation
 * and wakes enc2 via IPI.
 */
#include <cstdio>
#include <cstring>
#include <fcntl.h>
#include <unistd.h>
#include <sys/stat.h>
#include <sys/ioctl.h>

#include <edge_call.h>
#include <keystone.h>
#include "SharedMemory.hpp"
#include "shared/keystone_user.h"
#include "shared/sm_err.h"

#define OCALL_PRINT  3
#define OCALL_DONE   6

#define KEYSTONE_DEV_PATH "/dev/keystone_enclave"

static bool      g_bridge_done = false;
static uintptr_t g_shm_base   = 0;
static size_t    g_shm_size    = 0;

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
        printf("[bridge-host] %s\n", buf);
        fflush(stdout);
    }
    ec->return_data.call_status  = CALL_STATUS_OK;
    ec->return_data.call_ret_size = 0;
}

static void dispatch_ocall(void *buf)
{
    struct edge_call *ec = (struct edge_call *)buf;
    switch (ec->call_id) {
        case OCALL_PRINT: handle_print(ec); break;
        case OCALL_DONE:
            g_bridge_done = true;
            ec->return_data.call_status  = CALL_STATUS_OK;
            ec->return_data.call_ret_size = 0;
            printf("[bridge-host] enc1 signaled DONE\n");
            fflush(stdout);
            break;
        default:
            printf("[bridge-host] unknown OCALL id=%lu\n", ec->call_id);
            ec->return_data.call_status = CALL_STATUS_BAD_CALL_ID;
    }
}

static int run_bridge_loop(int kfd, uintptr_t eid, void *shm_buf)
{
    struct keystone_ioctl_run_enclave args = {};
    args.eid = eid;

    if (ioctl(kfd, KEYSTONE_IOC_RUN_ENCLAVE, &args) != 0) {
        perror("[bridge-host] ioctl RUN");
        return -1;
    }

    while (true) {
        switch (args.error) {
        case SBI_ERR_SM_ENCLAVE_SUCCESS:
            if (g_bridge_done) {
                printf("[bridge-host] enc1 exited cleanly\n");
                return 0;
            }
            fprintf(stderr, "[bridge-host] enc1 exited without OCALL_DONE (early failure)\n");
            return -1;

        case SBI_ERR_SM_ENCLAVE_EDGE_CALL_HOST:
            dispatch_ocall(shm_buf);
            if (ioctl(kfd, KEYSTONE_IOC_RESUME_ENCLAVE, &args) != 0) {
                perror("[bridge-host] ioctl RESUME after OCALL");
                return -1;
            }
            break;

        case SBI_ERR_SM_ENCLAVE_INTERRUPTED:
        case SBI_ERR_SM_ENCLAVE_WAITING_FOR_DEVICE:
            if (ioctl(kfd, KEYSTONE_IOC_RESUME_ENCLAVE, &args) != 0) {
                perror("[bridge-host] ioctl RESUME after interrupt");
                return -1;
            }
            break;

        case SBI_ERR_SM_ENCLAVE_WAITING_FOR_SHM:
            if (ioctl(kfd, KEYSTONE_IOC_RESUME_ENCLAVE, &args) != 0) {
                perror("[bridge-host] ioctl RESUME after WaitingForShm");
                return -1;
            }
            break;

        default:
            fprintf(stderr, "[bridge-host] unexpected SM error: %lu\n", args.error);
            return -1;
        }
    }
}

int main(int argc, char **argv)
{
    if (argc < 4) {
        fprintf(stderr, "usage: %s <bridge-eapp> <runtime> <loader>\n", argv[0]);
        return 1;
    }

    /* enc1 discovers dev-SHM and TDDS channel autonomously via SM */
    Keystone::Enclave bridgeEnc;
    Keystone::Params  params;
    params.setFreeMemSize(1 * 1024 * 1024);
    params.setUntrustedSize(64 * 1024);
    if (bridgeEnc.init(argv[1], argv[2], argv[3], params) != Keystone::Error::Success) {
        fprintf(stderr, "[bridge-host] enc1 init failed\n");
        return 1;
    }
    g_shm_base = (uintptr_t)bridgeEnc.getSharedBuffer();
    g_shm_size = bridgeEnc.getSharedBufferSize();
    printf("[bridge-host] enc1 EID=%d\n", bridgeEnc.getEID());

    int kfd = open(KEYSTONE_DEV_PATH, O_RDWR);
    if (kfd < 0) { perror("open keystone"); return 1; }

    printf("[bridge-host] starting enc1 run loop\n");
    fflush(stdout);

    int rc = run_bridge_loop(kfd, (uintptr_t)bridgeEnc.getEID(),
                             bridgeEnc.getSharedBuffer());
    close(kfd);

    if (rc == 0)
        printf("[bridge-host] enc1 finished successfully\n");
    else
        fprintf(stderr, "[bridge-host] run loop failed\n");

    return rc;
}
