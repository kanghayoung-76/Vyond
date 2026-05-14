#include "host/keystone.h"
#include <stdio.h>

using namespace Keystone;

int main(int argc, char** argv)
{
    printf("[HOST] wg-slot-test: launching enclave...\n");
    printf("[HOST] Expected: SM prints enclave EPM info when ACCESS FAULT occurs.\n");

    Enclave enclave;
    Params params;
    params.setFreeMemSize(256 * 1024);
    params.setUntrustedSize(0);

    enclave.init(argv[1], argv[2], argv[3], params);
    enclave.run();

    printf("[HOST] enclave.run() returned.\n");
    return 0;
}
