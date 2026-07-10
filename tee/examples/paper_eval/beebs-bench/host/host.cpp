//******************************************************************************
// Generic host runner for the BEEBS crc32 enclave (paper eval #5). The eapp is
// a plain program; its printf is forwarded to this host via the io_syscall /
// linux_syscall eyrie plugins, so no custom ocall wrapper is needed.
//******************************************************************************
#include "edge/edge_call.h"
#include "host/keystone.h"

using namespace Keystone;

int main(int argc, char** argv) {
  Enclave enclave;
  Params params;

  params.setFreeMemSize(1024 * 1024);
  params.setUntrustedSize(256 * 1024);

  enclave.init(argv[1], argv[2], argv[3], params);

  enclave.registerOcallDispatch(incoming_call_dispatch);

  enclave.run();

  return 0;
}
