/* Shared definitions for Trusted DDS OCALL wire format. */
#ifndef __TDDS_COMMON_H__
#define __TDDS_COMMON_H__

#include <stdint.h>
#include <stddef.h>

/* Edge-call IDs */
#define OCALL_TDDS_WAIT        1   /* sub -> host: need publisher to run */
#define OCALL_LOAN_DEV_SHM     4   /* bridge -> host: get dev-SHM {rid,pa,size} */
#define OCALL_GET_RID_FOR_TOPIC 6  /* enc -> host: topic name -> TDDS rid */
#define OCALL_GET_RID_OUT      7   /* bridge -> host: get output channel rid */

/* Descriptor returned by OCALL_LOAN_DEV_SHM */
typedef struct {
    uint32_t  rid;
    uintptr_t pa;
    size_t    size;
} tdds_dev_shm_t;

/* Argument for OCALL_TDDS_WAIT */
struct tdds_ocall_wait_args {
    uint32_t rid;
};

#endif /* __TDDS_COMMON_H__ */
