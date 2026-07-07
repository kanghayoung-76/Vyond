#ifndef __SM_CALL_H__
#define __SM_CALL_H__

#ifndef __KERNEL__
#include <stddef.h>
#include <stdint.h>
#endif

// BKE (Berkeley Keystone Enclave)
#define SBI_EXT_EXPERIMENTAL_KEYSTONE_ENCLAVE 0x08424b45

#define SBI_SET_TIMER 0
#define SBI_CONSOLE_PUTCHAR 1
#define SBI_CONSOLE_GETCHAR 2

/* 0-1999 are not used (deprecated) */
#define FID_RANGE_DEPRECATED      1999
/* 2000-2999 are called by host */
#define SBI_SM_CREATE_ENCLAVE    2001
#define SBI_SM_DESTROY_ENCLAVE   2002
#define SBI_SM_RUN_ENCLAVE       2003
#define SBI_SM_RESUME_ENCLAVE    2005
#define SBI_SM_WAIT_AND_RESUME   2006  /* host: SM wfi-blocks until enc2 has OCALL */
#define FID_RANGE_HOST           2999

/* 3000-3999 are called by enclave */
#define SBI_SM_RANDOM            3001
#define SBI_SM_ATTEST_ENCLAVE    3002
#define SBI_SM_GET_SEALING_KEY   3003
#define SBI_SM_STOP_ENCLAVE      3004
#define SBI_SM_EXIT_ENCLAVE      3006
#define SBI_SM_REGISTER_DEV_IRQ  3007  /* bind current enclave to a device IRQ */
#define SBI_SM_WAIT_DEV_DATA     3008  /* suspend enclave until that IRQ fires  */
#define SBI_SM_WAIT_SHM          3009  /* suspend enclave until notify_shm(rid) */
#define SBI_SM_NOTIFY_SHM        3010  /* resume enclave waiting on rid         */
#define FID_RANGE_ENCLAVE        3999

/* 4000-4999 are experimental */
#define SBI_SM_CALL_PLUGIN        4000
#define SBI_SM_CREATE_SHM_REGION  4001
#define SBI_SM_MAP_SHM_REGION 	  4002
#define SBI_SM_UNMAP_SHM_REGION   4003
#define SBI_SM_CHANGE_SHM_REGION  4004
#define SBI_SM_SHARE_SHM_REGION   4005
#define SBI_SM_CREATE_DEV_SHM        4006  /* host: create device-enclave SHM (device_wid, lazy WGC) */
#define SBI_SM_CREATE_ENCLAVE_SHM    4007  /* host: create enclave-channel SHM (no host EID in perm_conf) */
#define SBI_SM_GET_SHM_EIDS          4008  /* enclave: query EID list of a channel before mapping */
#define SBI_SM_PREPARE_EPM           4009  /* host: install OS_WID setup slot for pool EPM PA */
#define SBI_SM_REGISTER_ENC_CHANNEL  3011  /* enc: bind creator+allowed hash to an enc-enc SHM rid */
#define SBI_SM_FIND_SHM_BY_HASH      3012  /* enc: find enc-enc rid by creator hash */
#define SBI_SM_GET_MY_HASH           3013  /* enc: retrieve own measurement hash (64 bytes) */
#define SBI_SM_FIND_DEV_SHM          3014  /* enc: find RegionDevEnc matching own hash */
#define SBI_SM_TRIGGER_DEV           3015  /* enc: ask SM to write CMD register of a device */
#define FID_RANGE_CUSTOM          4999

/* Plugin IDs and Call IDs */
#define SM_MULTIMEM_PLUGIN_ID   0x01
#define SM_MULTIMEM_CALL_GET_SIZE 0x01
#define SM_MULTIMEM_CALL_GET_ADDR 0x02

/* Enclave stop reasons requested */
#define STOP_TIMER_INTERRUPT  0
#define STOP_EDGE_CALL_HOST   1
#define STOP_EXIT_ENCLAVE     2
#define STOP_WAITING_DEV_DATA 3  /* enclave is waiting for a device IRQ */
#define STOP_WAITING_SHM      4  /* enclave is waiting for notify_shm   */

/* Structs for interfacing into the SM */
struct runtime_params_t {
  uintptr_t dram_base;
  uintptr_t dram_size;
  uintptr_t runtime_base;
  uintptr_t user_base;
  uintptr_t free_base;
  uintptr_t untrusted_base;
  uintptr_t untrusted_size;
  uintptr_t free_requested; // for attestation
};

struct keystone_sbi_pregion_t {
  uintptr_t paddr;
  size_t size;
};

struct keystone_sbi_create_t {
  struct keystone_sbi_pregion_t epm_region;
  struct keystone_sbi_pregion_t utm_region;

  uintptr_t runtime_paddr;
  uintptr_t user_paddr;
  uintptr_t free_paddr;
  uintptr_t free_requested;
};

#endif  // __SM_CALL_H__
