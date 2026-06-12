#include "app/tdds.h"
#include "app/syscall.h"
#include "app/string.h"
#include "shared/tdds_common.h"
#include "edge/edge_common.h"

/* SHM layout: [ flag/size (uint32_t) | data (TDDS_MAX_MSG_SIZE bytes) ]
 * flag == 0: empty;  flag == N > 0: N bytes published and ready */
typedef struct {
    volatile uint32_t flag;
    uint8_t           data[TDDS_MAX_MSG_SIZE];
} tdds_shm_t;

/* UTM mapping used by tdds_utm_data — mapped once on first use */
static void*  s_utm_base = (void*)0;
static size_t s_utm_size = 0;

static inline void utm_ensure(void) {
    if (!s_utm_base) s_utm_base = map_utm(&s_utm_size);
}

int
tdds_init_channel(tdds_channel_t *ch, rid_t rid) {
    if (verify_shm_channel(rid) != 0) return -1;
    void *addr = map_shm(rid);
    if (!addr) return -1;
    ch->rid = rid;
    ch->shm_addr = addr;
    return 0;
}

int
tdds_publish(tdds_channel_t *ch, const void *msg, size_t size) {
    if (!ch || !ch->shm_addr || !msg) return -1;
    if (size > TDDS_MAX_MSG_SIZE) size = TDDS_MAX_MSG_SIZE;
    tdds_shm_t *shm = (tdds_shm_t *)ch->shm_addr;
    memcpy(shm->data, msg, size);
    __sync_synchronize();
    shm->flag = (uint32_t)size;
    /* Wake any enclave suspended on this SHM channel via WAIT_SHM */
    SYSCALL_1(RUNTIME_SYSCALL_NOTIFY_SHM, ch->rid);
    return 0;
}

int
tdds_subscribe(tdds_channel_t *ch, void *buf, size_t size) {
    if (!ch || !ch->shm_addr || !buf) return -1;
    tdds_shm_t *shm = (tdds_shm_t *)ch->shm_addr;
    uint32_t n = shm->flag;
    if (n == 0) return TDDS_NOT_READY;
    __sync_synchronize();
    if (n > (uint32_t)size) n = (uint32_t)size;
    memcpy(buf, shm->data, n);
    shm->flag = 0;
    return (int)n;
}

void*
tdds_utm_data(size_t *avail_out) {
    utm_ensure();
    if (avail_out)
        *avail_out = s_utm_size > sizeof(struct edge_call) ?
                     s_utm_size - sizeof(struct edge_call) : 0;
    return (void*)((uintptr_t)s_utm_base + sizeof(struct edge_call));
}

void
tdds_destroy_channel(tdds_channel_t *ch) {
    if (!ch || !ch->shm_addr) return;
    unmap_shm(ch->rid, ch->shm_addr, sizeof(tdds_shm_t));
    ch->shm_addr = 0;
}
