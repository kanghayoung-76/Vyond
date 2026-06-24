/*
 * split-host-test: sub-host — manages enc2 (subscriber enclave)
 *
 * Run order:
 *   1. sub-host <sub-eapp> <runtime> <loader>    (start first)
 *   2. bridge-host <bridge-eapp> <runtime> <loader>
 *
 * sub-host creates the TDDS channel SHM, runs enc2 until it parks at
 * wait_shm, then signals bridge-host via /tmp/wgtee_sub_ready.
 *
 * After enc2 parks, the SM handles all subsequent enc2 scheduling:
 *   Single-core: SM switches directly from enc1→enc2 inside bridge-host's
 *     notify_shm ioctl (same-hart path).  sub-host is not involved.
 *   Multi-core (taskset): SM sends MSIP IPI to sub-host's hart; M-mode
 *     intercepts it via mtvec=trap_vector_enclave (set by SM's keep_vec=true
 *     in switch_to_host) and wakes enc2 transparently.  sub-host is not
 *     involved.
 * In both cases sub-host just polls for bridge-host's DONE_FILE.
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

#define RID_FILE   "/tmp/wgtee_tdds_rid"
#define READY_FILE "/tmp/wgtee_sub_ready"
#define DONE_FILE  "/tmp/wgtee_done"

#define KEYSTONE_DEV_PATH "/dev/keystone_enclave"

static uint32_t         g_rid_tdds = 0;
static uintptr_t        g_shm_base = 0;
static size_t           g_shm_size = 0;

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
        printf("[sub-host] %s\n", buf);
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

/*
 * Drive enc2 through its initial phase until it parks at wait_shm.
 *
 * enc2 calls OCALL_GET_TDDS_RID once to get the TDDS SHM RID, then calls
 * wait_shm() and suspends.  The SM returns WaitingForShm; this function
 * returns 0.  After this point the SM owns enc2 scheduling (single-core
 * direct switch or multicore IPI) — no further RESUME_ENCLAVE needed.
 */
static int run_sub_init(int kfd, uintptr_t eid, void *shm_buf)
{
    struct keystone_ioctl_run_enclave args = {};
    args.eid = eid;

    if (ioctl(kfd, KEYSTONE_IOC_RUN_ENCLAVE, &args) != 0) {
        perror("[sub-host] ioctl RUN");
        return -1;
    }

    /* Handle any OCALLs enc2 makes before it parks at wait_shm */
    while (args.error == SBI_ERR_SM_ENCLAVE_EDGE_CALL_HOST ||
           args.error == SBI_ERR_SM_ENCLAVE_INTERRUPTED) {
        if (args.error == SBI_ERR_SM_ENCLAVE_EDGE_CALL_HOST)
            dispatch_ocall(shm_buf);
        if (ioctl(kfd, KEYSTONE_IOC_RESUME_ENCLAVE, &args) != 0) {
            if (errno == EINTR) continue;
            perror("[sub-host] ioctl RESUME (pre-park)");
            return -1;
        }
    }

    if (args.error == SBI_ERR_SM_ENCLAVE_SUCCESS) {
        /* enc2 exited before parking — nothing for bridge-host to do */
        printf("[sub-host] enc2 exited before parking (degenerate)\n");
        return 0;
    }

    if (args.error != SBI_ERR_SM_ENCLAVE_WAITING_FOR_SHM) {
        fprintf(stderr, "[sub-host] unexpected SM error after initial run: %lu\n",
                args.error);
        return -1;
    }

    printf("[sub-host] enc2 parked at wait_shm\n");
    fflush(stdout);
    return 0;
}

int main(int argc, char **argv)
{
    if (argc < 4) {
        fprintf(stderr, "usage: %s <sub-eapp> <runtime> <loader>\n", argv[0]);
        return 1;
    }

    /* Clean up stale coordination files */
    unlink(RID_FILE);
    unlink(READY_FILE);
    unlink(DONE_FILE);

    printf("[sub-host] creating TDDS SHM (%d bytes)\n", TDDS_SHM_SIZE);

    /* Create enc1↔enc2 TDDS channel SHM (enclave-only, no host access) */
    Keystone::SharedMemory tddsShm;
    g_rid_tdds = tddsShm.createEnclaveShm(TDDS_SHM_SIZE);
    if (!g_rid_tdds) {
        fprintf(stderr, "[sub-host] createEnclaveShm failed\n");
        return 1;
    }
    printf("[sub-host] TDDS SHM rid=%u\n", g_rid_tdds);

    /* Initialize enc2 */
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

    /* Share TDDS SHM with enc2 */
    tddsShm.shareShm(g_rid_tdds, subEnc.getEID(), 7);
    printf("[sub-host] enc2 EID=%d, TDDS rid=%u shared\n", subEnc.getEID(), g_rid_tdds);

    /* Open keystone device for direct ioctl during initial phase */
    int kfd = open(KEYSTONE_DEV_PATH, O_RDWR);
    if (kfd < 0) { perror("[sub-host] open keystone"); return 1; }

    printf("[sub-host] running enc2 → enc2 will park at wait_shm\n");
    fflush(stdout);

    /* Drive enc2 through OCALLs until it parks at wait_shm */
    int rc = run_sub_init(kfd, (uintptr_t)subEnc.getEID(), subEnc.getSharedBuffer());
    close(kfd);

    if (rc != 0) {
        fprintf(stderr, "[sub-host] enc2 init phase failed\n");
        return 1;
    }

    /* Write TDDS RID + EID for bridge-host, then signal ready */
    {
        FILE *f = fopen(RID_FILE, "w");
        if (!f) { perror("fopen rid"); return 1; }
        fprintf(f, "%u %lu\n", g_rid_tdds, (uintptr_t)subEnc.getEID());
        fclose(f);
    }
    close(open(READY_FILE, O_CREAT | O_WRONLY, 0644));
    printf("[sub-host] ready signal written — start bridge-host now\n");
    fflush(stdout);

    /*
     * Wait for bridge-host to finish enc1.
     * The SM wakes enc2 via:
     *   - single-core: direct hart switch inside bridge-host's notify_shm ioctl
     *   - multi-core:  M-mode MSIP IPI handler (transparent to sub-host)
     */
    while (access(DONE_FILE, F_OK) != 0)
        usleep(50000);

    printf("[sub-host] bridge-host done. exiting.\n");
    unlink(RID_FILE);
    unlink(READY_FILE);
    unlink(DONE_FILE);
    return 0;
}
