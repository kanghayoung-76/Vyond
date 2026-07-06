#ifndef __EYRIE_CALL_H__
#define __EYRIE_CALL_H__

#define RUNTIME_SYSCALL_UNKNOWN             1000
#define RUNTIME_SYSCALL_OCALL               1001
#define RUNTIME_SYSCALL_SHAREDCOPY          1002
#define RUNTIME_SYSCALL_ATTEST_ENCLAVE      1003
#define RUNTIME_SYSCALL_GET_SEALING_KEY     1004
#define RUNTIME_SYSCALL_MAP_SHM             1005
#define RUNTIME_SYSCALL_UNMAP_SHM           1006
#define RUNTIME_SYSCALL_TRANSLATE_VA        1011  /* VA -> PA for DMA setup        */
#define RUNTIME_SYSCALL_GET_SHM_EIDS        1012  /* query SHM channel EID list    */
#define RUNTIME_SYSCALL_MAP_UTM             1013  /* get pre-mapped UTM base VA    */
#define RUNTIME_SYSCALL_WAIT_SHM            1014  /* suspend until notify_shm(rid) */
#define RUNTIME_SYSCALL_NOTIFY_SHM          1015  /* wake enclave waiting on rid   */
#define RUNTIME_SYSCALL_REGISTER_ENC_CHANNEL 1016 /* enc: bind hash attestation to enc-enc SHM rid */
#define RUNTIME_SYSCALL_FIND_SHM_BY_HASH    1017  /* enc: find enc-enc rid by creator hash */
#define RUNTIME_SYSCALL_GET_MY_HASH         1018  /* enc: get own measurement hash (64 bytes) */
#define RUNTIME_SYSCALL_EXIT                1101

#endif  // __EYRIE_CALL_H__
