/*
 * dds-tdds-test: host runner (MYDEV DMA flow)
 *
 * Flow: MYDEV DMA → dev-SHM → IRQ → bridge-enc → publish(TDDS SHM)
 *                                               + NOTIFY_SHM → sub-enc wakes
 *
 * Usage: ./dds-runner <bridge-eapp> <sub-eapp> <runtime> <loader>
 */
#include <edge_call.h>
#include <keystone.h>
#include <stdio.h>
#include <string.h>
#include <stdint.h>

#include "SharedMemory.hpp"
#include "shared/tdds_common.h"

#define DEV_WID       29
#define DEV_SHM_SIZE  4096   /* matches MYDEV host_buf_size exactly */

#define OCALL_GET_RID          2
#define OCALL_PRINT            3
/* OCALL_LOAN_DEV_SHM=4 from shared/tdds_common.h */
#define OCALL_GET_RID_OUT      5
#define OCALL_GET_RID_FOR_TOPIC 6  /* libddsc_tdds: topic name → TDDS rid */

static uint32_t                  g_rid_tdds = 0;
static uint32_t                  g_dev_rid  = 0;
static Keystone::SharedMemory   *g_pDevShm  = nullptr;

static uintptr_t g_bridge_shm_base = 0;
static size_t    g_bridge_shm_size = 0;
static uintptr_t g_sub_shm_base    = 0;
static size_t    g_sub_shm_size    = 0;

/* ---- edge_call helpers --------------------------------------------------- */

static void set_ret(struct edge_call *ec, const void *data, size_t data_sz,
                    uintptr_t base, size_t shm_sz)
{
    uintptr_t ret_area = base + sizeof(struct edge_call);
    memcpy((void *)ret_area, data, data_sz);
    if (edge_call_setup_ret(ec, (void *)ret_area, data_sz, base, shm_sz) != 0)
        ec->return_data.call_status = CALL_STATUS_BAD_PTR;
    else
        ec->return_data.call_status = CALL_STATUS_OK;
}

/* Bridge OCALLs */
static void handle_loan_dev_shm(struct edge_call *ec, uintptr_t base, size_t sz)
{
    tdds_dev_shm_t s = { g_dev_rid, (uintptr_t)g_pDevShm->getPA(), DEV_SHM_SIZE };
    set_ret(ec, &s, sizeof(s), base, sz);
}

static void handle_get_rid_out(struct edge_call *ec, uintptr_t base, size_t sz)
{
    set_ret(ec, &g_rid_tdds, sizeof(g_rid_tdds), base, sz);
}

/* Sub OCALLs */
static void handle_get_rid(struct edge_call *ec, uintptr_t base, size_t sz)
{
    set_ret(ec, &g_rid_tdds, sizeof(g_rid_tdds), base, sz);
}

/* libddsc_tdds OCALL: map a topic name to a TDDS channel rid.
 * Currently all topics share the single g_rid_tdds channel.
 * Future: parse topic name from call args and look up per-topic rid. */
static void handle_get_rid_for_topic(struct edge_call *ec, uintptr_t base, size_t sz)
{
    /* The topic name is available in the call arg area, but we currently
     * ignore it and return the global channel rid for all topics. */
    set_ret(ec, &g_rid_tdds, sizeof(g_rid_tdds), base, sz);
}

static void handle_print(struct edge_call *ec, uintptr_t base, size_t sz)
{
    size_t len = ec->call_arg_size;
    uintptr_t ptr = 0;
    edge_call_get_ptr_from_offset(ec->call_arg_offset, len, &ptr, base, sz);
    if (ptr && len > 0) {
        char buf[512] = {0};
        size_t n = (len < sizeof(buf) - 1) ? len : sizeof(buf) - 1;
        memcpy(buf, (void *)ptr, n);
        printf("%s\n", buf);
    }
    ec->return_data.call_status = CALL_STATUS_OK;
    ec->return_data.call_ret_size = 0;
}

/* ---- main ---------------------------------------------------------------- */

int main(int argc, char **argv)
{
    if (argc < 5) {
        fprintf(stderr, "usage: %s <bridge-eapp> <sub-eapp> <runtime> <loader>\n",
                argv[0]);
        return 1;
    }

    printf("\n===== MYDEV DMA → enc1 → TDDS → enc2 =====\n\n");
    printf("----- [Phase 1] Setup: SHM allocation -----\n");

    /* Step 1: device-accessible SHM (WID=29 allows MYDEV DMA writes) */
    static Keystone::SharedMemory devShm;
    g_pDevShm = &devShm;
    g_dev_rid = devShm.createDevShm(DEV_SHM_SIZE, DEV_WID);
    if (!g_dev_rid) { fprintf(stderr, "[HOST] createDevShm failed\n"); return 1; }
    printf("[HOST] dev-SHM : rid=%u  pa=%p\n", g_dev_rid, devShm.getPA());

    /* Step 2: TDDS channel SHM (enc1 ↔ enc2, no host access needed) */
    static Keystone::SharedMemory tddsShm;
    g_rid_tdds = tddsShm.createEnclaveShm(8192);
    if (!g_rid_tdds) { fprintf(stderr, "[HOST] createEnclaveShm failed\n"); return 1; }
    printf("[HOST] TDDS SHM: rid=%u\n", g_rid_tdds);

    /* Step 3: bridge enclave */
    Keystone::Enclave bridgeEnc;
    Keystone::Params  bridgeParams;
    bridgeParams.setFreeMemSize(1 * 1024 * 1024);
    bridgeParams.setUntrustedSize(64 * 1024);
    if (bridgeEnc.init(argv[1], argv[3], argv[4], bridgeParams)
            != Keystone::Error::Success) {
        fprintf(stderr, "[HOST] bridge enclave init failed\n"); return 1;
    }
    g_bridge_shm_base = (uintptr_t)bridgeEnc.getSharedBuffer();
    g_bridge_shm_size = bridgeEnc.getSharedBufferSize();
    devShm.shareShm(g_dev_rid,   bridgeEnc.getEID(), 7);
    tddsShm.shareShm(g_rid_tdds, bridgeEnc.getEID(), 7);

    bridgeEnc.registerOcallDispatch([](void *buf, size_t) {
        struct edge_call *ec = (struct edge_call *)buf;
        switch (ec->call_id) {
            case OCALL_LOAN_DEV_SHM:
                handle_loan_dev_shm(ec, g_bridge_shm_base, g_bridge_shm_size);
                break;
            case OCALL_GET_RID_OUT:
                handle_get_rid_out(ec, g_bridge_shm_base, g_bridge_shm_size);
                break;
            case OCALL_GET_RID_FOR_TOPIC:
                handle_get_rid_for_topic(ec, g_bridge_shm_base, g_bridge_shm_size);
                break;
            case OCALL_PRINT:
                /* enc2's eprint is routed here during coroutine loop:
                 * SM copies enc2's UTM to enc1's UTM before returning to host. */
                handle_print(ec, g_bridge_shm_base, g_bridge_shm_size);
                break;
            default:
                ec->return_data.call_status = CALL_STATUS_BAD_CALL_ID;
        }
    });

    /* Step 4: sub enclave */
    Keystone::Enclave subEnc;
    Keystone::Params  subParams;
    subParams.setFreeMemSize(1 * 1024 * 1024);
    subParams.setUntrustedSize(64 * 1024);
    if (subEnc.init(argv[2], argv[3], argv[4], subParams)
            != Keystone::Error::Success) {
        fprintf(stderr, "[HOST] sub enclave init failed\n"); return 1;
    }
    g_sub_shm_base = (uintptr_t)subEnc.getSharedBuffer();
    g_sub_shm_size = subEnc.getSharedBufferSize();
    tddsShm.shareShm(g_rid_tdds, subEnc.getEID(), 7);

    subEnc.registerOcallDispatch([](void *buf, size_t) {
        struct edge_call *ec = (struct edge_call *)buf;
        switch (ec->call_id) {
            case OCALL_GET_RID:
                handle_get_rid(ec, g_sub_shm_base, g_sub_shm_size);
                break;
            case OCALL_GET_RID_FOR_TOPIC:
                handle_get_rid_for_topic(ec, g_sub_shm_base, g_sub_shm_size);
                break;
            case OCALL_PRINT:
                handle_print(ec, g_sub_shm_base, g_sub_shm_size);
                break;
            default:
                printf("[HOST][SUB] unknown OCALL id=%lu\n", ec->call_id);
                ec->return_data.call_status = CALL_STATUS_BAD_CALL_ID;
        }
    });

    printf("\n----- [Phase 2] enc2 (sub): run → WAIT_SHM -----\n");
    fflush(stdout);
    subEnc.run();
    printf("[HOST] enc2 suspended at WAIT_SHM\n");
    fflush(stdout);

    printf("\n----- [Phase 3+4] enc1/enc2 loop: DMA→publish→switch→take (×3) -----\n");
    fflush(stdout);
    bridgeEnc.run();
    printf("[HOST] enc1 + enc2 done\n\n");

    return 0;
}
