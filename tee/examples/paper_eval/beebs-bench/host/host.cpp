//******************************************************************************
// Generic host runner for the BEEBS crc32 enclave. The eapp is a plain program;
// its printf is forwarded here via the io_syscall / linux_syscall eyrie plugins.
//******************************************************************************
#include "edge/edge_call.h"
#include "host/keystone.h"
#include <cstdio>

using namespace Keystone;

int main(int argc, char** argv) {
  Enclave enclave;
  Params params;

  params.setFreeMemSize(1024 * 1024);
  params.setUntrustedSize(256 * 1024);

  enclave.init(argv[1], argv[2], argv[3], params);

  enclave.registerOcallDispatch(incoming_call_dispatch);

  uintptr_t retval = 0;
  enclave.run(&retval);
  // eapp returns 0 if crc32 verify passed, 1 if not (no printf inside enclave,
  // so this host-side print is the only output; it goes to the Linux console).
  printf("[HOST] beebs-crc32 enclave exit=%lu (0=correct, 1=wrong)\n",
         (unsigned long)retval);

  return 0;
}
