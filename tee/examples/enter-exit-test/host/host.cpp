// enter-exit-test host runner.
// Creates an enclave, enters it (run), and reports each phase with a marker so
// we can see EXACTLY how far it gets. No ocall dispatch (the eapp makes none).
#include <keystone.h>
#include <cstddef>
#include <cstdio>

// FPGA static-link workaround (from Vyond-main): glibc IFUNC memcpy
// (R_RISCV_IRELATIVE) self-loops in a static binary and crashes; provide a
// plain memcpy so the linker drops the IFUNC indirection.
extern "C" void* memcpy(void* dest, const void* src, size_t n) {
  char* d = (char*)dest; const char* s = (const char*)src;
  while (n--) *d++ = *s++;
  return dest;
}

using namespace Keystone;

int main(int argc, char** argv) {
  if (argc < 4) {
    fprintf(stderr, "usage: %s <eapp> <runtime> <loader>\n", argv[0]);
    return 1;
  }

  Enclave enclave;
  Params  params;
  params.setFreeMemSize(1024 * 1024);
  params.setUntrustedSize(1024 * 1024);

  printf("[HOST] EE1 init begin\n");
  Error e = enclave.init(argv[1], argv[2], argv[3], params);
  printf("[HOST] EE2 init returned err=%d\n", (int)e);
  if (e != Error::Success) { printf("[HOST] EE-FAIL init\n"); return 2; }

  printf("[HOST] EE3 entering enclave (run)...\n");
  uintptr_t ret = 0;
  Error r = enclave.run(&ret);
  printf("[HOST] EE4 run returned err=%d ret=%lu\n", (int)r, (unsigned long)ret);

  printf("[HOST] EE5 DONE cleanly\n");
  return 0;
}
