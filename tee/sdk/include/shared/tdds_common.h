/* Shared definitions for Trusted DDS OCALL wire format.
 * Included by both the enclave app (sdk/include/app/tdds.h) and the
 * host broker (sdk/include/host/TddsBroker.hpp). */
#ifndef __TDDS_COMMON_H__
#define __TDDS_COMMON_H__

#include <stdint.h>
#include <stddef.h>   /* size_t, uintptr_t */

/* Edge-call IDs */
#define OCALL_TDDS_WAIT     1   /* sub -> host: need publisher to run */
#define OCALL_LOAN_DEV_SHM  4   /* pub -> host: get device SHM info */

/* Arguments packed into the edge_call shared buffer. */
struct tdds_ocall_wait_args {
    uint32_t rid;   /* RID of the channel the subscriber is waiting on */
};

/* Device SHM info returned by OCALL_LOAN_DEV_SHM */
typedef struct {
    uint32_t  rid;
    uintptr_t pa;
    size_t    size;
} tdds_dev_shm_t;

#endif /* __TDDS_COMMON_H__ */
