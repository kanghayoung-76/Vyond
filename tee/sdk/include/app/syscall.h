//******************************************************************************
// Copyright (c) 2018, The Regents of the University of California (Regents).
// All Rights Reserved. See LICENSE for license details.
//------------------------------------------------------------------------------
#ifndef __SYSCALL_H__
#define __SYSCALL_H__

#include <stddef.h>
#include <stdint.h>
#include "sealing.h"

#include "shared/eyrie_call.h"
#include "shared/keystone_user.h"

#define SYSCALL(which, arg0, arg1, arg2, arg3, arg4)           \
  ({                                                           \
    register uintptr_t a0 asm("a0") = (uintptr_t)(arg0);       \
    register uintptr_t a1 asm("a1") = (uintptr_t)(arg1);       \
    register uintptr_t a2 asm("a2") = (uintptr_t)(arg2);       \
    register uintptr_t a3 asm("a3") = (uintptr_t)(arg3);       \
    register uintptr_t a4 asm("a4") = (uintptr_t)(arg4);       \
    register uintptr_t a7 asm("a7") = (uintptr_t)(which);      \
    asm volatile("ecall"                                       \
                 : "+r"(a0)                                    \
                 : "r"(a1), "r"(a2), "r"(a3), "r"(a4), "r"(a7) \
                 : "memory");                                  \
    a0;                                                        \
  })

#define SYSCALL_0(which) SYSCALL(which, 0, 0, 0, 0, 0)
#define SYSCALL_1(which, arg0) SYSCALL(which, arg0, 0, 0, 0, 0)
#define SYSCALL_2(which, arg0, arg1) SYSCALL(which, arg0, arg1, 0, 0, 0)
#define SYSCALL_3(which, arg0, arg1, arg2) \
  SYSCALL(which, arg0, arg1, arg2, 0, 0)
#define SYSCALL_4(which, arg0, arg1, arg2, arg3) \
  SYSCALL(which, arg0, arg1, arg2, arg3, 0)
#define SYSCALL_5(which, arg0, arg1, arg2, arg3, arg4) \
  SYSCALL(which, arg0, arg1, arg2, arg3, arg4)

int
copy_from_shared(void* dst, uintptr_t offset, size_t data_len);

int
ocall(
    unsigned long call_id, void* data, size_t data_len, void* return_buffer,
    size_t return_len);
uintptr_t
untrusted_mmap();
int
attest_enclave(void* report, void* data, size_t size);

int
get_sealing_key(
    struct sealing_key* sealing_key_struct, size_t sealing_key_struct_size,
    void* key_ident, size_t key_ident_size);

void*
map_shm(rid_t rid);

int
unmap_shm(rid_t rid, void* addr, size_t size);

/* get_shm_eids: returns EID list for a shared memory region.
 * eids_out: caller-provided buffer, max_count: buffer capacity.
 * Returns number of EIDs written, or -1 on error. */
int
get_shm_eids(rid_t rid, uintptr_t* eids_out, size_t max_count);

/* verify_shm_channel: queries SM for EID list of rid and checks
 * that host EID (11) is absent. Returns 0 if safe, -1 otherwise. */
int
verify_shm_channel(rid_t rid);

/* register_enc_channel: bind hash-based attestation to an existing enc-enc SHM rid.
 * allowed_hash: 64-byte hash of the enclave permitted to subscribe.
 * SM records caller's own hash as creator_hash automatically.
 * Returns 0 on success, -1 on error. */
int
register_enc_channel(rid_t rid, const uint8_t *allowed_hash);

/* find_shm_by_hash: locate an enc-enc SHM channel without a RID_FILE.
 * creator_hash: 64-byte hash of the publisher enclave.
 * rid_out: receives the rid of the matching channel.
 * SM matches creator_hash AND verifies caller's own hash == allowed_hash.
 * Returns 0 on success, -1 if no matching channel found. */
int
find_shm_by_hash(const uint8_t *creator_hash, rid_t *rid_out);

/* get_my_hash: retrieve this enclave's own 64-byte measurement hash from SM. */
int
get_my_hash(uint8_t *hash_out);

/* find_dev_shm: locate the RegionDevEnc channel whose allowed_hash matches
 * this enclave (or is open). rid_out receives the matching rid. */
int
find_dev_shm(rid_t *rid_out);

/* trigger_dev: ask SM to write CMD=1 to the MMIO register of device_wid.
 * The enclave never writes MMIO directly. */
int
trigger_dev(uint32_t device_wid);

void*
mydev_map(uintptr_t base, size_t size);

int
mydev_unmap(void* addr, size_t size);

/* map_utm: return the pre-mapped UTM (host-enclave shared buffer) VA.
 * The loader already mapped UTM at EYRIE_UNTRUSTED_START with PTE_U,
 * so this simply retrieves that address and its size. */
void*
map_utm(size_t* size_out);
#endif /* syscall.h */
