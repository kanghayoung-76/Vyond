//******************************************************************************
// Copyright (c) 2018, The Regents of the University of California (Regents).
// All Rights Reserved. See LICENSE for license details.
//------------------------------------------------------------------------------
// Ported 2026-07 to the current edge_call API: the old edge_call_init_internals()
// / register_call() global path was removed. We now keep the shared-buffer
// base/size locally (from getSharedBuffer()) and dispatch OCALLs via a switch,
// exactly like shm-ocall-test/host/host.cpp.
#include <edge_call.h>
#include <keystone.h>
#include <stdio.h>
#include <string.h>
#include <cstddef>

// FPGA static-link workaround (from Vyond-main): glibc IFUNC memcpy
// (R_RISCV_IRELATIVE) is not resolved in our static binary, so the PLT
// self-loops and crashes (SIGILL / host access-fault storm). Providing our own
// memcpy makes the linker drop the IFUNC indirection entirely.
extern "C" void* memcpy(void* dest, const void* src, size_t n) {
  char* d = (char*)dest;
  const char* s = (const char*)src;
  while (n--) *d++ = *s++;
  return dest;
}

#define OCALL_PRINT_STRING 1

/* Shared buffer base/size — set once after enclave init. */
static uintptr_t shm_base = 0;
static size_t    shm_size = 0;

static uintptr_t args_ptr(struct edge_call* ec, size_t* size_out) {
  *size_out = ec->call_arg_size;
  uintptr_t ptr;
  if (edge_call_get_ptr_from_offset(ec->call_arg_offset, ec->call_arg_size,
                                    &ptr, shm_base, shm_size) != 0)
    return 0;
  return ptr;
}

static void set_ret(struct edge_call* ec, const void* data, size_t size) {
  uintptr_t ret_area = shm_base + sizeof(struct edge_call);
  memcpy((void*)ret_area, data, size);
  if (edge_call_setup_ret(ec, (void*)ret_area, size, shm_base, shm_size) != 0)
    ec->return_data.call_status = CALL_STATUS_BAD_PTR;
  else
    ec->return_data.call_status = CALL_STATUS_OK;
}

/* The ocall exposed to the enclave: print a string it sent. */
static unsigned long print_string(const char* str) {
  return printf("Enclave said: \"%s\"\n", str);
}

static void handle_print_string(void* buffer) {
  struct edge_call* ec = (struct edge_call*)buffer;
  size_t arg_len;
  uintptr_t str_ptr = args_ptr(ec, &arg_len);
  if (!str_ptr) {
    ec->return_data.call_status = CALL_STATUS_BAD_OFFSET;
    return;
  }
  char buf[256] = {0};
  size_t copy_len = arg_len < sizeof(buf) - 1 ? arg_len : sizeof(buf) - 1;
  memcpy(buf, (void*)str_ptr, copy_len);
  unsigned long ret = print_string(buf);
  set_ret(ec, &ret, sizeof(ret));
}

static void ocall_dispatch(void* buffer, size_t /*size*/) {
  struct edge_call* ec = (struct edge_call*)buffer;
  switch (ec->call_id) {
    case OCALL_PRINT_STRING: handle_print_string(buffer); break;
    default:
      printf("[HOST] unknown OCALL id=%lu\n", ec->call_id);
      ec->return_data.call_status = CALL_STATUS_BAD_CALL_ID;
  }
}

int
main(int argc, char** argv) {
  if (argc < 4) {
    fprintf(stderr, "usage: %s <eapp> <runtime> <loader>\n", argv[0]);
    return 1;
  }

  Keystone::Enclave enclave;
  Keystone::Params  params;
  params.setFreeMemSize(1024 * 1024);
  params.setUntrustedSize(1024 * 1024);

  /* [TRACE] init()의 반환값은 원래 버려지고 있었다. 적재 실패해도 그대로 run()으로 진행해
   * enclave가 쓰레기 파라미터로 실행되므로, 실패를 여기서 잡는다. */
  Keystone::Error init_err = enclave.init(argv[1], argv[2], argv[3], params);
  printf("[TRACE][APP] init() -> %d (0=Success)\n", (int)init_err);
  if (init_err != Keystone::Error::Success) {
    printf("[TRACE][APP] init FAILED — aborting before run()\n");
    return 1;
  }

  shm_base = (uintptr_t)enclave.getSharedBuffer();
  shm_size = enclave.getSharedBufferSize();
  printf("[HOST] hello-native: SHM base=0x%lx size=%zu\n", shm_base, shm_size);

  enclave.registerOcallDispatch(
      [](void* b, size_t s) { ocall_dispatch(b, s); });

  printf("[HOST] Running enclave...\n");
  enclave.run();
  printf("[HOST] Enclave finished.\n");

  return 0;
}
