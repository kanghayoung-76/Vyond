/*
 * split-host-test: sub-host — manages enc2 (subscriber enclave)
 *
 * Run order:
 *   1. sub-host <sub-eapp> <runtime> <loader>    (start first)
 *   2. bridge-host <bridge-eapp> <runtime> <loader>
 *
 * sub-host creates the TDDS channel SHM, parks enc2 at wait_shm,
 * then signals bridge-host via /tmp/wgtee_sub_ready.
 * bridge-host handles the actual iteration loop (enc1→enc2 switching
 * happens inside bridge-host's ioctl thread via SM direct-switch).
 * sub-host waits for /tmp/wgtee_done, then exits.
 */
#include <cstdio>
#include <cstring>
#include <unistd.h>
#include <fcntl.h>
#include <sys/stat.h>

#include <edge_call.h>
#include <keystone.h>
#include "SharedMemory.hpp"

#define TDDS_SHM_SIZE  8192

#define OCALL_PRINT        3
#define OCALL_GET_TDDS_RID 5

#define RID_FILE   "/tmp/wgtee_tdds_rid"
#define READY_FILE "/tmp/wgtee_sub_ready"
#define DONE_FILE  "/tmp/wgtee_done"

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
    printf("[sub-host] enc2 EID=%d, TDDS rid shared\n", subEnc.getEID());

    subEnc.registerOcallDispatch([](void *buf, size_t) {
        struct edge_call *ec = (struct edge_call *)buf;
        switch (ec->call_id) {
            case OCALL_GET_TDDS_RID: handle_get_tdds_rid(ec); break;
            case OCALL_PRINT:        handle_print(ec);        break;
            default:
                ec->return_data.call_status = CALL_STATUS_BAD_CALL_ID;
        }
    });

    printf("[sub-host] running enc2 → enc2 will park at wait_shm\n");

    /* Run enc2: it calls wait_shm() → SM parks it, SDK run() returns Success */
    subEnc.run();

    printf("[sub-host] enc2 parked at wait_shm (EID=%d)\n", subEnc.getEID());

    /* Write rid to coordination file so bridge-host can share the same SHM */
    {
        FILE *f = fopen(RID_FILE, "w");
        if (!f) { perror("fopen rid"); return 1; }
        fprintf(f, "%u %d\n", g_rid_tdds, subEnc.getEID());
        fclose(f);
    }

    /* Signal bridge-host: enc2 is parked, safe to start */
    close(open(READY_FILE, O_CREAT | O_WRONLY, 0644));
    printf("[sub-host] ready signal written — start bridge-host now\n");
    fflush(stdout);

    /* Wait for bridge-host to finish.
     * enc2 will be resumed/run to completion inside bridge-host's ioctl thread.
     * sub-host just waits here (no more ioctl calls needed for enc2). */
    printf("[sub-host] waiting for bridge-host to complete...\n");
    while (access(DONE_FILE, F_OK) != 0) {
        usleep(50000);  /* 50 ms poll */
    }

    printf("[sub-host] bridge-host done. exiting.\n");
    unlink(RID_FILE);
    unlink(READY_FILE);
    unlink(DONE_FILE);
    return 0;
}
