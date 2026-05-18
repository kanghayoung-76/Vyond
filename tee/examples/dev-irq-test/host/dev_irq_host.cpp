#include <stdio.h>
#include "edge/edge_call.h"
#include "host/keystone.h"
#include "edge_wrapper.h"

using namespace Keystone;

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

    edge_init(&enclave);

    enclave.run();
    printf("[HOST] enclave finished\n");
    return 0;
}
