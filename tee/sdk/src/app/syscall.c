//******************************************************************************
// Copyright (c) 2018, The Regents of the University of California (Regents).
// All Rights Reserved. See LICENSE for license details.
//------------------------------------------------------------------------------
#include "syscall.h"

/* this implementes basic system calls for the enclave */

int
ocall(
    unsigned long call_id, void* data, size_t data_len, void* return_buffer,
    size_t return_len) {
  return SYSCALL_5(RUNTIME_SYSCALL_OCALL,
      call_id, data, data_len, return_buffer, return_len);
}

int
copy_from_shared(void* dst, uintptr_t offset, size_t data_len) {
  return SYSCALL_3(RUNTIME_SYSCALL_SHAREDCOPY, dst, offset, data_len);
}

int
attest_enclave(void* report, void* data, size_t size) {
  return SYSCALL_3(RUNTIME_SYSCALL_ATTEST_ENCLAVE, report, data, size);
}

/* returns sealing key */
int
get_sealing_key(
    struct sealing_key* sealing_key_struct, size_t sealing_key_struct_size,
    void* key_ident, size_t key_ident_size) {
  return SYSCALL_4(RUNTIME_SYSCALL_GET_SEALING_KEY,
      sealing_key_struct, sealing_key_struct_size,
      key_ident, key_ident_size);
}

void*
map_shm(rid_t rid) {
  uintptr_t vaddr;
  uintptr_t* vaddr_ptr = &vaddr;
  int ret              = SYSCALL_2(RUNTIME_SYSCALL_MAP_SHM, rid, vaddr_ptr);
  if (ret) vaddr = 0;
  return (void*)vaddr;
}

int
unmap_shm(rid_t rid, void* addr, size_t size) {
  return SYSCALL_3(RUNTIME_SYSCALL_UNMAP_SHM, rid, addr, size);
}


int
get_shm_eids(rid_t rid, uintptr_t* eids_out, size_t max_count) {
  uintptr_t actual_count = 0;
  int ret = SYSCALL_4(RUNTIME_SYSCALL_GET_SHM_EIDS,
                      rid, eids_out, max_count, &actual_count);
  if (ret) return -1;
  return (int)actual_count;
}

void*
map_utm(size_t* size_out) {
  uintptr_t vaddr = 0;
  uintptr_t sz    = 0;
  int ret = SYSCALL_2(RUNTIME_SYSCALL_MAP_UTM, &vaddr, &sz);
  if (ret) return (void*)0;
  if (size_out) *size_out = (size_t)sz;
  return (void*)vaddr;
}

#define HOST_EID 11

int
verify_shm_channel(rid_t rid) {
  uintptr_t eids[8];
  int count = get_shm_eids(rid, eids, 8);
  if (count < 0) return -1;
  for (int i = 0; i < count; i++) {
    if (eids[i] == HOST_EID) return -1;
  }
  return 0;
}

int
register_enc_channel(rid_t rid, const uint8_t *allowed_hash) {
  return (int)SYSCALL_2(RUNTIME_SYSCALL_REGISTER_ENC_CHANNEL, rid, allowed_hash);
}

int
find_shm_by_hash(const uint8_t *creator_hash, rid_t *rid_out) {
  return (int)SYSCALL_2(RUNTIME_SYSCALL_FIND_SHM_BY_HASH, creator_hash, rid_out);
}

int
get_my_hash(uint8_t *hash_out) {
  return (int)SYSCALL_1(RUNTIME_SYSCALL_GET_MY_HASH, hash_out);
}

