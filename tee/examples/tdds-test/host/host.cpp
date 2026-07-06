/*
 * tdds-test: host runner
 *
 * Usage: ./tdds-runner tdds-pub tdds-sub eyrie-rt loader.bin
 *
 * Flow (dev→enclave1→enclave2):
 *   1. Create enclave-only TDDS SHM channel (no host EID)
 *   2. Create device SHM (dev WID=29 + pub enclave WID)
 *   3. Init pub enclave; share TDDS SHM + dev SHM with it
 *   4. Init sub enclave; share TDDS SHM with it
 *   5. Broker: OCALL_TDDS_WAIT(rid) from sub → run pub enclave
 *      pub: DMA from device → TDDS channel → sub receives it
 */
#include <edge_call.h>
#include <keystone.h>
#include <stdio.h>
#include <string.h>
#include <stdint.h>

#include "SharedMemory.hpp"
#include "TddsBroker.hpp"
#include "shared/tdds_common.h"   /* OCALL_TDDS_WAIT, OCALL_LOAN_DEV_SHM, tdds_dev_shm_t */

#define OCALL_GET_RID  2
#define OCALL_PRINT    3

#define DEV_WID        5
#define DEV_SHM_SIZE   64

/* Channel RID — global so lambdas can capture by reference */
static uint32_t g_rid = 0;

/* Device SHM — needed for OCALL_LOAN_DEV_SHM handler */
static Keystone::SharedMemory* g_dev_shm = nullptr;

/* ---- Helper: extract args pointer from edge_call ---- */
static uintptr_t
args_ptr(struct edge_call* ec, size_t* sz_out,
         uintptr_t shm_base, size_t shm_size) {
    *sz_out = ec->call_arg_size;
    uintptr_t ptr = 0;
    edge_call_get_ptr_from_offset(ec->call_arg_offset, ec->call_arg_size,
                                   &ptr, shm_base, shm_size);
    return ptr;
}

/* ---- Helper: write return data into edge_call ---- */
static void
set_ret(struct edge_call* ec, const void* data, size_t size,
        uintptr_t shm_base, size_t shm_size) {
    uintptr_t ret_area = shm_base + sizeof(struct edge_call);
    memcpy((void*)ret_area, data, size);
    if (edge_call_setup_ret(ec, (void*)ret_area, size, shm_base, shm_size) != 0)
        ec->return_data.call_status = CALL_STATUS_BAD_PTR;
    else
        ec->return_data.call_status = CALL_STATUS_OK;
}

/* ---- OCALL handlers ---- */
static void
handle_get_rid(struct edge_call* ec, uintptr_t shm_base, size_t shm_size) {
    set_ret(ec, &g_rid, sizeof(g_rid), shm_base, shm_size);
}

static void
handle_print(struct edge_call* ec, uintptr_t shm_base, size_t shm_size) {
    size_t len = 0;
    uintptr_t ptr = args_ptr(ec, &len, shm_base, shm_size);
    if (ptr) {
        char buf[512] = {0};
        size_t n = (len < sizeof(buf) - 1) ? len : sizeof(buf) - 1;
        memcpy(buf, (void*)ptr, n);
        printf("%s\n", buf);
    }
    ec->return_data.call_status = CALL_STATUS_OK;
    ec->return_data.call_ret_size = 0;
}

static void
handle_loan_dev_shm(struct edge_call* ec, uintptr_t shm_base, size_t shm_size) {
    tdds_dev_shm_t info;
    info.rid  = g_dev_shm->getRID();
    info.pa   = (uintptr_t)g_dev_shm->getPA();
    info.size = g_dev_shm->getSize();
    printf("[HOST][PUB] OCALL_LOAN_DEV_SHM: rid=%u pa=0x%lx size=%zu\n",
           info.rid, info.pa, info.size);
    set_ret(ec, &info, sizeof(info), shm_base, shm_size);
}

/* ---- Main ---- */
int main(int argc, char** argv) {
    if (argc < 5) {
        fprintf(stderr,
                "usage: %s <pub-eapp> <sub-eapp> <runtime> <loader>\n",
                argv[0]);
        return 1;
    }

    const char* pub_eapp = argv[1];
    const char* sub_eapp = argv[2];
    const char* runtime  = argv[3];
    const char* loader   = argv[4];

    printf("[HOST] tdds-test (dev->enclave1->enclave2) starting\n");

    /* --- 1. Create enclave-only TDDS SHM channel (no host EID) --- */
    Keystone::SharedMemory shm;
    g_rid = shm.createEnclaveShm(8192);
    if (g_rid == 0) {
        fprintf(stderr, "[HOST] createEnclaveShm failed\n");
        return 1;
    }
    printf("[HOST] TDDS channel SHM created: RID=%u\n", g_rid);

    /* --- 2. Create device SHM (device WID=29 permitted) --- */
    Keystone::SharedMemory devShm;
    g_dev_shm = &devShm;
    uint32_t dev_rid = devShm.createDevShm(DEV_SHM_SIZE, DEV_WID);
    if (dev_rid == 0) {
        fprintf(stderr, "[HOST] createDevShm failed\n");
        return 1;
    }
    printf("[HOST] device SHM created: RID=%u PA=0x%lx size=%zu\n",
           dev_rid, (uintptr_t)devShm.getPA(), devShm.getSize());

    /* --- 3. Init publisher enclave --- */
    Keystone::Enclave pubEnc;
    Keystone::Params  pubParams;
    pubParams.setFreeMemSize(1 * 1024 * 1024);
    pubParams.setUntrustedSize(64 * 1024);
    if (pubEnc.init(pub_eapp, runtime, loader, pubParams)
            != Keystone::Error::Success) {
        fprintf(stderr, "[HOST] pub enclave init failed\n");
        return 1;
    }
    uintptr_t pub_shm_base = (uintptr_t)pubEnc.getSharedBuffer();
    size_t    pub_shm_size = pubEnc.getSharedBufferSize();
    printf("[HOST] pub enclave ready (EID=%lu shm_base=0x%lx)\n",
           (unsigned long)pubEnc.getEID(), pub_shm_base);

    /* Share TDDS SHM with pub enclave */
    shm.shareShm(g_rid, pubEnc.getEID(), /*perm=*/0);
    /* Share device SHM with pub enclave (full access: device writes, enclave reads) */
    devShm.shareShm(dev_rid, pubEnc.getEID(), /*perm=*/7);

    pubEnc.registerOcallDispatch(
        [&pub_shm_base, &pub_shm_size](void* buf, size_t /*sz*/) {
            struct edge_call* ec = (struct edge_call*)buf;
            switch (ec->call_id) {
                case OCALL_GET_RID:
                    handle_get_rid(ec, pub_shm_base, pub_shm_size);
                    break;
                case OCALL_PRINT:
                    handle_print(ec, pub_shm_base, pub_shm_size);
                    break;
                case OCALL_LOAN_DEV_SHM:
                    handle_loan_dev_shm(ec, pub_shm_base, pub_shm_size);
                    break;
                default:
                    printf("[HOST][PUB] unknown OCALL id=%lu\n", ec->call_id);
                    ec->return_data.call_status = CALL_STATUS_BAD_CALL_ID;
            }
        });

    /* --- 4. Init subscriber enclave --- */
    Keystone::Enclave subEnc;
    Keystone::Params  subParams;
    subParams.setFreeMemSize(1 * 1024 * 1024);
    subParams.setUntrustedSize(64 * 1024);
    if (subEnc.init(sub_eapp, runtime, loader, subParams)
            != Keystone::Error::Success) {
        fprintf(stderr, "[HOST] sub enclave init failed\n");
        return 1;
    }
    uintptr_t sub_shm_base = (uintptr_t)subEnc.getSharedBuffer();
    size_t    sub_shm_size = subEnc.getSharedBufferSize();
    printf("[HOST] sub enclave ready (EID=%lu shm_base=0x%lx)\n",
           (unsigned long)subEnc.getEID(), sub_shm_base);

    /* Share TDDS SHM with sub enclave */
    shm.shareShm(g_rid, subEnc.getEID(), /*perm=*/0);

    /* --- 5. Wire up broker: OCALL_TDDS_WAIT(rid) -> run pub --- */
    Keystone::TddsBroker broker;
    broker.registerChannel(g_rid, [&pubEnc]() {
        printf("[HOST] OCALL_TDDS_WAIT: running publisher enclave (reads device, publishes)\n");
        uintptr_t retval = 0;
        pubEnc.run(&retval);
        printf("[HOST] publisher finished (retval=%lu)\n", retval);
    });

    subEnc.registerOcallDispatch(
        [&broker, &sub_shm_base, &sub_shm_size](void* buf, size_t sz) {
            struct edge_call* ec = (struct edge_call*)buf;
            switch (ec->call_id) {
                case OCALL_GET_RID:
                    handle_get_rid(ec, sub_shm_base, sub_shm_size);
                    break;
                case OCALL_PRINT:
                    handle_print(ec, sub_shm_base, sub_shm_size);
                    break;
                case OCALL_TDDS_WAIT:
                    broker.handleOcall(buf, sz);
                    break;
                default:
                    printf("[HOST][SUB] unknown OCALL id=%lu\n", ec->call_id);
                    ec->return_data.call_status = CALL_STATUS_BAD_CALL_ID;
            }
        });

    /* --- 6. Run subscriber; pub fires automatically on first wait --- */
    printf("[HOST] running subscriber enclave\n");
    uintptr_t retval = 0;
    Keystone::Error err = subEnc.run(&retval);
    if (err != Keystone::Error::Success)
        fprintf(stderr, "[HOST] subscriber enclave returned error\n");
    else
        printf("[HOST] subscriber finished OK (retval=%lu)\n", retval);

    printf("[HOST] tdds-test done\n");
    return (err == Keystone::Error::Success) ? 0 : 1;
}
