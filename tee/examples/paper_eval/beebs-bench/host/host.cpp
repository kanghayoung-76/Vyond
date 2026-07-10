//******************************************************************************
// Generic host runner for the BEEBS crc32 enclave (paper eval #5). The eapp is
// a plain program; its printf is forwarded to this host via the io_syscall /
// linux_syscall eyrie plugins, so no custom ocall wrapper is needed.
//******************************************************************************
#include "edge/edge_call.h"
#include "host/keystone.h"
#include <cstdio>
#include <cstdint>

using namespace Keystone;

// Timer source (host, U-mode Linux — no UTM/scounteren issue). Default =
// rdcycle (CPU cycles). For wall-clock LATENCY, comment rdcycle and uncomment
// rdtime (divide the delta by the timebase frequency to get seconds).
static inline uint64_t read_timer(void) {
  uint64_t v;
  asm volatile("rdcycle %0" : "=r"(v));   // CPU cycles (default)
  // asm volatile("rdtime %0" : "=r"(v)); // wall-clock ticks (uncomment for latency)
  return v;
}

int main(int argc, char** argv) {
  Enclave enclave;
  Params params;

  params.setFreeMemSize(1024 * 1024);
  params.setUntrustedSize(256 * 1024);

  enclave.init(argv[1], argv[2], argv[3], params);
  enclave.registerOcallDispatch(incoming_call_dispatch);

  // "Runtime" = host-observed enclave.run() cost (same as Keystone test-runner:
  // enclave enter + eyrie boot + eapp compute + exit). eapp binary unchanged.
  uintptr_t retval = 0;
  uint64_t t0 = read_timer();
  enclave.run(&retval);
  uint64_t t1 = read_timer();

  printf("[HOST] beebs-crc32 run=%lu cyc  exit=%lu (0=correct)\n",
         (unsigned long)(t1 - t0), (unsigned long)retval);

  return 0;
}
