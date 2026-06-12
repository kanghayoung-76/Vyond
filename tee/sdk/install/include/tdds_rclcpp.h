#pragma once
#include "app/tdds.h"
#include "app/syscall.h"
#include "shared/eyrie_call.h"

/* Minimal rclcpp-compatible pub/sub shim backed by TDDS.
 * API names match rclcpp so enclave code can be ported without rename. */

typedef tdds_channel_t Publisher;
typedef tdds_channel_t Subscription;

static inline int create_publisher(Publisher *pub, rid_t rid) {
    return tdds_init_channel(pub, rid);
}

static inline int create_subscription(Subscription *sub, rid_t rid) {
    return tdds_init_channel(sub, rid);
}

/* publish(): matches rclcpp Publisher::publish() */
static inline void publish(Publisher *pub, const void *data, size_t len) {
    tdds_publish(pub, data, len);
}

/* take(): non-blocking receive — returns bytes read, or TDDS_NOT_READY */
static inline int take(Subscription *sub, void *buf, size_t max) {
    return tdds_subscribe(sub, buf, max);
}

/* wait(): suspend enclave until publisher calls notify_shm on this channel's rid */
static inline void wait(Subscription *sub) {
    SYSCALL_1(RUNTIME_SYSCALL_WAIT_SHM, sub->rid);
}
