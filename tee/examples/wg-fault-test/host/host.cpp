#include "host/keystone.h"

using namespace Keystone;

int main(int argc, char** argv) {
    printf("[HOST] wg-fault-test: launching enclave...\n");
    printf("[HOST] Expected: SM prints 'ACCESS FAULT' before eapp runs.\n");

    Enclave enclave;
    Params params;
    params.setFreeMemSize(256 * 1024);
    params.setUntrustedSize(256 * 1024);

    enclave.init(argv[1], argv[2], argv[3], params);
    enclave.run();

    printf("[HOST] enclave.run() returned.\n");
    printf("[HOST] If ACCESS FAULT appeared above -> WGChecker->SM trap confirmed.\n");
    return 0;
}
