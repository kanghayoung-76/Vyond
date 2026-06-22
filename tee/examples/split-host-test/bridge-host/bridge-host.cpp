/*
 * split-host-test: bridge-host — manages enc1 (publisher/bridge enclave)
 *
 * Run after sub-host has written /tmp/wgtee_sub_ready.
 *
 * Single-core flow (SM direct-switch):
 *   bridgeEnc.run() → enc1 runs → notify_shm(rid_tdds)
 *     → SM suspends enc1, resumes enc2 (within THIS ioctl thread)
 *     → enc2 takes data, calls wait_shm → SM parks enc2, returns WaitingForShm
 *     → bridge-host sees WaitingForShm → resumes enc1 (next iteration)
 *     → ...
 *   After enc2's last take, enc2 exits:
 *     → SM restores bridge-host's Linux ctx → ioctl returns Success
 *     → bridge-host resumes enc1 (enc1 finishes loop, calls OCALL_DONE)
 *     → bridge-host handles OCALL_DONE: sets bridge_done=true
 *     → enc1 calls EAPP_RETURN(0) → ioctl returns Success + bridge_done=true → stop
 *
 * Uses direct ioctl for the run/resume loop because the SDK Enclave::run()
 * treats WaitingForShm as "done" (designed for the old host-driven model).
 */
#include <cstdio>
#include <cstring>
#include <cstdlib>
#include <unistd.h>
#include <fcntl.h>
#include <sys/ioctl.h>
#include <sys/stat.h>

#include <edge_call.h>
#include <keystone.h>
#include "SharedMemory.hpp"
#include "shared/tdds_common.h"
/* keystone_user.h lives in include/shared/ (not include/host/).
 * Use the shared/ path so its internal #include "sm_call.h" resolves correctly. */
#include "shared/keystone_user.h"
#include "shared/sm_err.h"

#define DEV_WID       29
#define DEV_SHM_SIZE  4096

#define OCALL_PRINT        3
#define OCALL_GET_TDDS_RID 5
#define OCALL_DONE         6

#define RID_FILE   "/tmp/wgtee_tdds_rid"
#define READY_FILE "/tmp/wgtee_sub_ready"
#define DONE_FILE  "/tmp/wgtee_done"

#define KEYSTONE_DEV_PATH "/dev/keystone_enclave"

static uint32_t  g_rid_tdds = 0;
static uint32_t  g_dev_rid  = 0;
static uintptr_t g_dev_pa   = 0;
static bool      g_bridge_done = false;

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

static void handle_loan_dev_shm(struct edge_call *ec)
{
    tdds_dev_shm_t s = { g_dev_rid, g_dev_pa, DEV_SHM_SIZE };
    set_ret(ec, &s, sizeof(s), g_shm_base, g_shm_size);
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
        case OCALL_LOAN_DEV_SHM:  handle_loan_dev_shm(ec);  break;
        case OCALL_GET_TDDS_RID:  handle_get_tdds_rid(ec);  break;
        case OCALL_PRINT:         handle_print(ec);          break;
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

/*
 * Custom run loop using direct ioctl.
 *
 * Why not SDK Enclave::run()?
 *   The SDK treats EnclaveWaitingForShm as "enclave parked, host done" and
 *   returns Success immediately.  With the new SM direct-switch, WaitingForShm
 *   means enc2 re-parked after a round — enc1 is still suspended mid-loop and
 *   must be resumed.
 */
static int run_bridge_loop(int kfd, uintptr_t eid, void *shm_buf)
{
    struct keystone_ioctl_run_enclave args = {};
    args.eid = eid;

    /* First call: run (not resume) */
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
            /* enc2 exited (not enc1) — resume enc1 to finish its loop */
            printf("[bridge-host] enc2 exited mid-loop, resuming enc1\n");
            if (ioctl(kfd, KEYSTONE_IOC_RESUME_ENCLAVE, &args) != 0) {
                perror("[bridge-host] ioctl RESUME after enc2 exit");
                return -1;
            }
            break;

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
            /* enc2 re-parked at wait_shm after processing a message.
             * Resume enc1 so it continues to the next DMA iteration. */
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

    /* Wait for sub-host to park enc2 */
    printf("[bridge-host] waiting for sub-host ready signal...\n");
    while (access(READY_FILE, F_OK) != 0) {
        usleep(50000);
    }
    printf("[bridge-host] sub-host ready\n");

    /* Read TDDS rid (and enc2 EID — for reference only) */
    uint32_t rid_tdds = 0;
    int sub_eid = 0;
    {
        FILE *f = fopen(RID_FILE, "r");
        if (!f) { perror("fopen rid"); return 1; }
        if (fscanf(f, "%u %d", &rid_tdds, &sub_eid) != 2) {
            fprintf(stderr, "[bridge-host] bad rid file\n");
            fclose(f); return 1;
        }
        fclose(f);
    }
    g_rid_tdds = rid_tdds;
    printf("[bridge-host] TDDS rid=%u  enc2 EID=%d\n", g_rid_tdds, sub_eid);

    /* Device-accessible SHM for MYDEV DMA */
    Keystone::SharedMemory devShm;
    g_dev_rid = devShm.createDevShm(DEV_SHM_SIZE, DEV_WID);
    if (!g_dev_rid) { fprintf(stderr, "[bridge-host] createDevShm failed\n"); return 1; }
    g_dev_pa = (uintptr_t)devShm.getPA();
    printf("[bridge-host] dev-SHM rid=%u  pa=0x%lx\n", g_dev_rid, g_dev_pa);

    /* Initialize enc1 */
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

    /* Share dev SHM with enc1 */
    devShm.shareShm(g_dev_rid, bridgeEnc.getEID(), 7);

    /* Share existing TDDS SHM (created by sub-host) with enc1.
     * SharedMemory::shareShm takes the rid as a parameter — any process
     * holding the keystone fd can share any RID with any enclave. */
    Keystone::SharedMemory tddsProxy;
    tddsProxy.shareShm(g_rid_tdds, bridgeEnc.getEID(), 7);
    printf("[bridge-host] TDDS SHM (rid=%u) shared with enc1\n", g_rid_tdds);

    /* Open keystone device for direct ioctl run loop */
    int kfd = open(KEYSTONE_DEV_PATH, O_RDWR);
    if (kfd < 0) { perror("open keystone"); return 1; }

    printf("[bridge-host] starting enc1 run loop\n");
    fflush(stdout);

    int rc = run_bridge_loop(kfd, (uintptr_t)bridgeEnc.getEID(),
                             bridgeEnc.getSharedBuffer());
    close(kfd);

    if (rc == 0) {
        printf("[bridge-host] enc1 + enc2 finished successfully\n");
    } else {
        fprintf(stderr, "[bridge-host] run loop failed\n");
    }

    /* Signal sub-host: done */
    close(open(DONE_FILE, O_CREAT | O_WRONLY, 0644));
    printf("[bridge-host] done signal written\n");

    return rc;
}
