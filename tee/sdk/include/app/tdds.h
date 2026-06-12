#ifndef __TDDS_H__
#define __TDDS_H__

#include <stddef.h>
#include "shared/keystone_user.h"
#include "shared/tdds_common.h"

/*
 * Trusted DDS (TDDS) — enclave-side pub/sub API over WorldGuard SHM.
 *
 * Data plane stays inside enclaves; host has no read/write access.
 * Model A (cooperative, single hart): subscribe is non-blocking;
 * caller yields via ocall_tdds_wait() so the host can run the publisher.
 *
 * SHM layout: [ flag/size: uint32_t | data: TDDS_MAX_MSG_SIZE bytes ]
 *   flag == 0: empty;  flag == N: N bytes ready to read
 */

#define TDDS_MAX_MSG_SIZE  4096
#define TDDS_NOT_READY     (-2)

typedef struct {
    rid_t   rid;
    void   *shm_addr;
} tdds_channel_t;

int   tdds_init_channel(tdds_channel_t *ch, rid_t rid);
int   tdds_publish(tdds_channel_t *ch, const void *msg, size_t size);
int   tdds_subscribe(tdds_channel_t *ch, void *buf, size_t size);
void  tdds_destroy_channel(tdds_channel_t *ch);

/* Returns pointer into UTM data area (after edge_call header) for OCALL args.
 * edge_call_setup_call requires data to be within UTM, not on the eapp stack. */
void* tdds_utm_data(size_t *avail_out);

#endif /* __TDDS_H__ */
