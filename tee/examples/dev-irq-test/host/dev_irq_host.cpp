#include <stdio.h>
#include "edge/edge_call.h"
#include "host/keystone.h"
#include "host/SharedMemory.hpp"
#include "edge_wrapper.h"

using namespace Keystone;

#define DEV_WID      6    /* temporary; will change to 5 (dedicated device world) */
#define DEV_SHM_SIZE 64

unsigned long print_buffer(char *str)
{
    printf("[HOST] %s", str);
    return strlen(str);
}

void print_value(unsigned long val)
{
    printf("[HOST] value: %lu (%#lx)\n", val, val);
}

int main(int argc, char **argv)
{
    if (argc < 4) {
        fprintf(stderr, "Usage: %s <eapp> <eyrie-rt> <loader.bin>\n", argv[0]);
        return 1;
    }

    Enclave enclave;
    Params params;
    params.setFreeMemSize(256 * 1024);
    params.setUntrustedSize(256 * 1024);

    enclave.init(argv[1], argv[2], argv[3], params);
    printf("[HOST] enclave initialized, EID=%d\n", enclave.getEID());

    /* Create device-enclave shared memory (lazy WGC slot, device_wid=DEV_WID) */
    SharedMemory devShm;
    rid_t dev_rid = devShm.createDevShm(DEV_SHM_SIZE, DEV_WID);
    if (!dev_rid) {
        fprintf(stderr, "[HOST] ERROR: createDevShm failed\n");
        return 1;
    }
    printf("[HOST] dev-SHM created: rid=%u pa=%p\n", dev_rid, devShm.getPA());

    /* Grant enclave full access to the dev-SHM */
    devShm.shareShm(dev_rid, enclave.getEID(), 7 /* FULL */);
    printf("[HOST] dev-SHM shared with enclave EID=%d\n", enclave.getEID());

    /* Register dev-SHM so loan_shm_wrapper can return it to the eapp */
    set_dev_shm(&devShm);

    edge_init(&enclave);

    enclave.run();
    printf("[HOST] enclave finished\n");
    return 0;
}
