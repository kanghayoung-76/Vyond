/*
 * libddsc_tdds.a — CycloneDDS-compatible stub for enclave use
 *
 * Replaces libddsc.so for ROS nodes compiled into a Keystone enclave.
 * ~25 functions have real implementations backed by TDDS SHM channels;
 * the remaining ~65 symbols are stubs that return success/null so the
 * rmw_cyclonedds layer links without pulling in the full DDS stack.
 *
 * Data path (publisher enclave):
 *   dds_write → sertype->serdata_ops->from_sample → to_ser → tdds_publish
 *
 * Data path (subscriber enclave):
 *   WAIT_SHM (dds_waitset_wait) → tdds_subscribe → from_ser_iov → dds_takecdr
 *
 * OCALL_GET_RID_FOR_TOPIC (id=6): host maps topic name → TDDS channel rid.
 */

#include "app/eapp_utils.h"
#include "app/string.h"
#include "app/syscall.h"
#include "app/tdds.h"

/* ── Minimal type definitions (ABI-compatible with CycloneDDS 0.10.5 / 64-bit) ── */

typedef int32_t  dds_entity_t;
typedef int32_t  dds_return_t;
typedef int32_t  dds_domainid_t;
typedef intptr_t dds_attach_t;
typedef int64_t  dds_duration_t;
typedef int64_t  dds_time_t;
typedef uint64_t dds_instance_handle_t;

/* Opaque handles — we never look inside these */
typedef void dds_qos_t;
typedef void dds_listener_t;

/* ddsi_serdata_kind (must match CycloneDDS enum values) */
enum ddsi_serdata_kind { SDK_EMPTY = 0, SDK_KEY = 1, SDK_DATA = 2 };

/* iovec (matches ddsrt_iovec_t) */
typedef struct { void *iov_base; size_t iov_len; } ddsrt_iovec_t;

/* Forward declarations */
struct ddsi_sertype;
struct ddsi_serdata;
struct ddsi_plist;

/*
 * ddsi_serdata_ops — layout MUST match CycloneDDS 0.10.5 exactly.
 * Each slot is one 8-byte function pointer on 64-bit.
 * Indices: [0]=eqkey [1]=get_size [2]=from_ser [3]=from_ser_iov
 *          [4]=from_keyhash [5]=from_sample [6]=to_ser [7]=to_ser_ref
 *          [8]=to_ser_unref [9]=to_sample [10]=to_untyped
 *          [11]=untyped_to_sample [12]=free [13]=print [14]=get_keyhash
 */
struct ddsi_serdata_ops {
    void *eqkey;            /* [0] */
    uint32_t (*get_size)(const struct ddsi_serdata *d);                             /* [1] */
    void *from_ser;         /* [2] */
    struct ddsi_serdata *(*from_ser_iov)(const struct ddsi_sertype *t,              /* [3] */
                                         enum ddsi_serdata_kind k,
                                         size_t niov,
                                         const ddsrt_iovec_t *iov,
                                         size_t sz);
    void *from_keyhash;     /* [4] */
    struct ddsi_serdata *(*from_sample)(const struct ddsi_sertype *t,               /* [5] */
                                        enum ddsi_serdata_kind k,
                                        const void *sample);
    void (*to_ser)(const struct ddsi_serdata *d, size_t off, size_t sz, void *buf); /* [6] */
    void *to_ser_ref;       /* [7] */
    void *to_ser_unref;     /* [8] */
    void *to_sample;        /* [9] */
    void *to_untyped;       /* [10] */
    void *untyped_to_sample;/* [11] */
    void (*free)(struct ddsi_serdata *d);                                            /* [12] */
    void *print;            /* [13] */
    void *get_keyhash;      /* [14] */
};

/*
 * ddsi_sertype — we only access serdata_ops at offset 8.
 * Full struct is larger; we just need to read those two pointer fields.
 */
struct ddsi_sertype {
    void *ops;                               /* offset  0 */
    const struct ddsi_serdata_ops *serdata_ops; /* offset  8 */
    /* remaining fields unused here */
};

/*
 * ddsi_serdata base layout (offsets verified against CycloneDDS 0.10.5 64-bit):
 *   0: ops  8: hash  12: refc  16: kind  20:_pad  24: type
 *  32: timestamp  40: statusinfo  44:_pad  48: twrite
 */
struct ddsi_serdata {
    const struct ddsi_serdata_ops *ops; /* offset  0 */
    uint32_t hash;                      /* offset  8 */
    uint32_t refc_v;                    /* offset 12 — ddsrt_atomic_uint32_t.v */
    uint32_t kind;                      /* offset 16 — enum ddsi_serdata_kind */
    uint32_t _pad;                      /* offset 20 */
    const struct ddsi_sertype *type;    /* offset 24 */
    int64_t  timestamp;                 /* offset 32 — ddsrt_wctime_t.v (ns) */
    uint32_t statusinfo;                /* offset 40 */
    uint32_t _pad2;                     /* offset 44 */
    int64_t  twrite;                    /* offset 48 — ddsrt_mtime_t.v (ns) */
};

/* dds_sample_info_t layout (from dds/dds.h) */
typedef struct {
    uint32_t sample_state;
    uint32_t view_state;
    uint32_t instance_state;
    bool     valid_data;
    int64_t  source_timestamp;
    uint64_t instance_handle;
    uint64_t publication_handle;
    uint32_t disposed_generation_count;
    uint32_t no_writers_generation_count;
    uint32_t sample_rank;
    uint32_t generation_rank;
    uint32_t absolute_generation_rank;
} dds_sample_info_t;

/* DDS return codes */
#define DDS_RETCODE_OK                    0
#define DDS_RETCODE_ERROR                 (-1)
#define DDS_RETCODE_UNSUPPORTED           (-2)
#define DDS_RETCODE_BAD_PARAMETER         (-3)
#define DDS_RETCODE_PRECONDITION_NOT_MET  (-4)
#define DDS_RETCODE_OUT_OF_RESOURCES      (-5)
#define DDS_RETCODE_NOT_ENABLED           (-6)
#define DDS_RETCODE_IMMUTABLE_POLICY      (-7)
#define DDS_RETCODE_INCONSISTENT_POLICY   (-8)
#define DDS_RETCODE_ALREADY_DELETED       (-9)
#define DDS_RETCODE_TIMEOUT               (-10)
#define DDS_RETCODE_NO_DATA               (-11)
#define DDS_RETCODE_ILLEGAL_OPERATION     (-12)
#define DDS_RETCODE_NOT_ALLOWED_BY_SECURITY (-13)

#define DDS_INFINITY  ((dds_duration_t)0x7fffffffffffffffLL)

/* DDS sample/view/instance state masks */
#define DDS_SST_NOT_READ   1u
#define DDS_SST_READ       2u
#define DDS_VST_NEW        1u
#define DDS_IST_ALIVE      1u

/* OCALL IDs */
#define OCALL_GET_RID              2
#define OCALL_PRINT                3
#define OCALL_GET_RID_FOR_TOPIC    6

/* ── Entity table ─────────────────────────────────────────────────────────── */

typedef enum {
    ENTITY_NONE = 0,
    ENTITY_PARTICIPANT,
    ENTITY_PUBLISHER,
    ENTITY_SUBSCRIBER,
    ENTITY_TOPIC,
    ENTITY_WRITER,
    ENTITY_READER,
    ENTITY_WAITSET,
    ENTITY_READCONDITION,
    ENTITY_GUARDCONDITION,
} entity_type_t;

#define MAX_ENTITIES   64
#define MAX_ATTACHMENTS 8

typedef struct {
    entity_type_t type;
    bool          in_use;
    /* For TOPIC / WRITER / READER */
    const struct ddsi_sertype *sertype;
    tdds_channel_t channel;
    /* For WRITER / READER / READCONDITION: link to parent entity */
    dds_entity_t parent;
    /* For WAITSET: attached entities + user attach values */
    int attach_count;
    dds_entity_t attach_ents[MAX_ATTACHMENTS];
    dds_attach_t attach_vals[MAX_ATTACHMENTS];
} entity_t;

static entity_t g_entities[MAX_ENTITIES];

static dds_entity_t entity_alloc(entity_type_t type)
{
    for (int i = 1; i < MAX_ENTITIES; i++) {
        if (!g_entities[i].in_use) {
            g_entities[i].in_use  = true;
            g_entities[i].type    = type;
            g_entities[i].parent  = 0;
            g_entities[i].sertype = NULL;
            g_entities[i].attach_count = 0;
            return (dds_entity_t)i;
        }
    }
    return DDS_RETCODE_OUT_OF_RESOURCES;
}

static entity_t *entity_get(dds_entity_t e)
{
    if (e <= 0 || e >= MAX_ENTITIES || !g_entities[e].in_use)
        return NULL;
    return &g_entities[e];
}

/* Resolve readcondition → reader entity */
static entity_t *resolve_reader(dds_entity_t e)
{
    entity_t *ent = entity_get(e);
    if (!ent) return NULL;
    if (ent->type == ENTITY_READCONDITION) {
        ent = entity_get(ent->parent);
    }
    return ent;
}

/* ── QoS / Listener pools ─────────────────────────────────────────────────── */

#define MAX_QOS       16
#define MAX_LISTENERS  8

static uint64_t g_qos_pool[MAX_QOS];   /* 8-byte dummy objects */
static uint64_t g_lst_pool[MAX_LISTENERS];
static uint8_t  g_qos_used[MAX_QOS];
static uint8_t  g_lst_used[MAX_LISTENERS];

static void *pool_alloc(void *pool, uint8_t *used, int n, size_t sz)
{
    for (int i = 0; i < n; i++) {
        if (!used[i]) {
            used[i] = 1;
            return (char *)pool + i * sz;
        }
    }
    return (char *)pool; /* fallback: return slot 0 (already in use) */
}

static void pool_free(void *pool, uint8_t *used, int n, size_t sz, void *ptr)
{
    for (int i = 0; i < n; i++) {
        if ((char *)pool + i * sz == (char *)ptr) {
            used[i] = 0;
            return;
        }
    }
}

/* ── Helper: get rid for topic from host via OCALL ────────────────────────── */

static rid_t get_rid_for_topic(const char *name)
{
    size_t avail;
    char *buf = (char *)tdds_utm_data(&avail);
    size_t name_len = strlen((char *)name) + 1;
    if (name_len > avail) name_len = avail;
    memcpy(buf, name, name_len);

    uintptr_t ret_ptr = 0;
    ocall(OCALL_GET_RID_FOR_TOPIC, buf, name_len, &ret_ptr, sizeof(ret_ptr));
    if (!ret_ptr) return 0;
    return *(rid_t *)ret_ptr;
}

/* ── Participant / Publisher / Subscriber ─────────────────────────────────── */

dds_entity_t dds_create_participant(dds_domainid_t domain,
                                    const dds_qos_t *qos,
                                    const dds_listener_t *listener)
{
    (void)domain; (void)qos; (void)listener;
    return entity_alloc(ENTITY_PARTICIPANT);
}

dds_entity_t dds_create_publisher(dds_entity_t participant,
                                   const dds_qos_t *qos,
                                   const dds_listener_t *listener)
{
    (void)participant; (void)qos; (void)listener;
    return entity_alloc(ENTITY_PUBLISHER);
}

dds_entity_t dds_create_subscriber(dds_entity_t participant,
                                    const dds_qos_t *qos,
                                    const dds_listener_t *listener)
{
    (void)participant; (void)qos; (void)listener;
    return entity_alloc(ENTITY_SUBSCRIBER);
}

/* ── Topic ────────────────────────────────────────────────────────────────── */

dds_entity_t dds_create_topic_sertype(dds_entity_t participant,
                                       const char *name,
                                       struct ddsi_sertype **sertype,
                                       const dds_qos_t *qos,
                                       const dds_listener_t *listener,
                                       const struct ddsi_plist *sedp_plist)
{
    (void)participant; (void)qos; (void)listener; (void)sedp_plist;

    dds_entity_t e = entity_alloc(ENTITY_TOPIC);
    if (e < 0) return e;

    entity_t *ent = &g_entities[e];
    ent->sertype = sertype ? *sertype : NULL;

    rid_t rid = get_rid_for_topic(name);
    if (!rid || tdds_init_channel(&ent->channel, rid) != 0) {
        ent->in_use = false;
        return DDS_RETCODE_ERROR;
    }

    return e;
}

/* ── Writer ───────────────────────────────────────────────────────────────── */

dds_entity_t dds_create_writer(dds_entity_t publisher_or_participant,
                                dds_entity_t topic,
                                const dds_qos_t *qos,
                                const dds_listener_t *listener)
{
    (void)publisher_or_participant; (void)qos; (void)listener;

    entity_t *top = entity_get(topic);
    if (!top || top->type != ENTITY_TOPIC) return DDS_RETCODE_BAD_PARAMETER;

    dds_entity_t e = entity_alloc(ENTITY_WRITER);
    if (e < 0) return e;

    entity_t *ent = &g_entities[e];
    ent->parent  = topic;
    ent->sertype = top->sertype;
    ent->channel = top->channel;

    return e;
}

/* ── Reader ───────────────────────────────────────────────────────────────── */

dds_entity_t dds_create_reader(dds_entity_t subscriber_or_participant,
                                dds_entity_t topic,
                                const dds_qos_t *qos,
                                const dds_listener_t *listener)
{
    (void)subscriber_or_participant; (void)qos; (void)listener;

    entity_t *top = entity_get(topic);
    if (!top || top->type != ENTITY_TOPIC) return DDS_RETCODE_BAD_PARAMETER;

    dds_entity_t e = entity_alloc(ENTITY_READER);
    if (e < 0) return e;

    entity_t *ent = &g_entities[e];
    ent->parent  = topic;
    ent->sertype = top->sertype;
    ent->channel = top->channel;

    return e;
}

/* ── ReadCondition ────────────────────────────────────────────────────────── */

dds_entity_t dds_create_readcondition(dds_entity_t reader, uint32_t mask)
{
    (void)mask;

    entity_t *r = entity_get(reader);
    if (!r || r->type != ENTITY_READER) return DDS_RETCODE_BAD_PARAMETER;

    dds_entity_t e = entity_alloc(ENTITY_READCONDITION);
    if (e < 0) return e;

    g_entities[e].parent = reader;
    return e;
}

/* ── WaitSet ──────────────────────────────────────────────────────────────── */

dds_entity_t dds_create_waitset(dds_entity_t participant)
{
    (void)participant;
    return entity_alloc(ENTITY_WAITSET);
}

dds_return_t dds_waitset_attach(dds_entity_t waitset, dds_entity_t entity, dds_attach_t x)
{
    entity_t *ws = entity_get(waitset);
    if (!ws || ws->type != ENTITY_WAITSET) return DDS_RETCODE_BAD_PARAMETER;
    if (ws->attach_count >= MAX_ATTACHMENTS) return DDS_RETCODE_OUT_OF_RESOURCES;

    ws->attach_ents[ws->attach_count] = entity;
    ws->attach_vals[ws->attach_count] = x;
    ws->attach_count++;
    return DDS_RETCODE_OK;
}

dds_return_t dds_waitset_detach(dds_entity_t waitset, dds_entity_t entity)
{
    entity_t *ws = entity_get(waitset);
    if (!ws || ws->type != ENTITY_WAITSET) return DDS_RETCODE_BAD_PARAMETER;

    for (int i = 0; i < ws->attach_count; i++) {
        if (ws->attach_ents[i] == entity) {
            ws->attach_ents[i] = ws->attach_ents[ws->attach_count - 1];
            ws->attach_vals[i] = ws->attach_vals[ws->attach_count - 1];
            ws->attach_count--;
            return DDS_RETCODE_OK;
        }
    }
    return DDS_RETCODE_BAD_PARAMETER;
}

/*
 * dds_waitset_wait: suspend the enclave (WAIT_SHM) until the publisher
 * triggers notify_shm on the corresponding channel.
 * Returns 1 on wakeup (one condition satisfied), 0 on timeout (not used),
 * or < 0 on error.
 */
dds_return_t dds_waitset_wait(dds_entity_t waitset, dds_attach_t *xs, size_t nxs,
                               dds_duration_t reltimeout)
{
    (void)reltimeout;

    entity_t *ws = entity_get(waitset);
    if (!ws || ws->type != ENTITY_WAITSET) return DDS_RETCODE_BAD_PARAMETER;

    /* Find the first attached entity that has a reader channel */
    for (int i = 0; i < ws->attach_count; i++) {
        entity_t *attached = resolve_reader(ws->attach_ents[i]);
        if (!attached) continue;

        rid_t rid = attached->channel.rid;
        if (!rid) continue;

        SYSCALL_1(RUNTIME_SYSCALL_WAIT_SHM, rid);

        if (xs && nxs > 0)
            xs[0] = ws->attach_vals[i];

        return 1;
    }

    /* No valid reader found in waitset */
    return DDS_RETCODE_ERROR;
}

/* ── Write ────────────────────────────────────────────────────────────────── */

/*
 * dds_write: serialize a raw sample using the stored sertype, then publish
 * the CDR bytes (including 4-byte header) via the TDDS channel.
 */
dds_return_t dds_write(dds_entity_t writer, const void *data)
{
    entity_t *w = entity_get(writer);
    if (!w || w->type != ENTITY_WRITER) return DDS_RETCODE_BAD_PARAMETER;
    if (!w->sertype || !data) return DDS_RETCODE_BAD_PARAMETER;

    const struct ddsi_serdata_ops *ops = w->sertype->serdata_ops;
    if (!ops || !ops->from_sample || !ops->get_size || !ops->to_ser || !ops->free)
        return DDS_RETCODE_ERROR;

    struct ddsi_serdata *sd = ops->from_sample(w->sertype, SDK_DATA, data);
    if (!sd) return DDS_RETCODE_ERROR;

    uint32_t sz = ops->get_size(sd);
    if (sz == 0 || sz > TDDS_MAX_MSG_SIZE) {
        ops->free(sd);
        return DDS_RETCODE_ERROR;
    }

    static uint8_t ser_buf[TDDS_MAX_MSG_SIZE];
    ops->to_ser(sd, 0, sz, ser_buf);
    ops->free(sd);

    return (tdds_publish(&w->channel, ser_buf, sz) == 0)
           ? DDS_RETCODE_OK : DDS_RETCODE_ERROR;
}

dds_return_t dds_write_ts(dds_entity_t writer, const void *data, dds_time_t timestamp)
{
    (void)timestamp;
    return dds_write(writer, data);
}

/*
 * dds_forwardcdr: publish a pre-serialized ddsi_serdata directly.
 * Extracts the CDR bytes from the serdata and sends via TDDS.
 */
dds_return_t dds_forwardcdr(dds_entity_t writer, struct ddsi_serdata *serdata)
{
    entity_t *w = entity_get(writer);
    if (!w || w->type != ENTITY_WRITER) return DDS_RETCODE_BAD_PARAMETER;
    if (!serdata || !serdata->ops) return DDS_RETCODE_BAD_PARAMETER;

    const struct ddsi_serdata_ops *ops = serdata->ops;
    if (!ops->get_size || !ops->to_ser) return DDS_RETCODE_ERROR;

    uint32_t sz = ops->get_size(serdata);
    if (sz == 0 || sz > TDDS_MAX_MSG_SIZE) return DDS_RETCODE_ERROR;

    static uint8_t fwd_buf[TDDS_MAX_MSG_SIZE];
    ops->to_ser(serdata, 0, sz, fwd_buf);

    return (tdds_publish(&w->channel, fwd_buf, sz) == 0)
           ? DDS_RETCODE_OK : DDS_RETCODE_ERROR;
}

/* ── Take (CDR) ───────────────────────────────────────────────────────────── */

/*
 * dds_takecdr: receive CDR bytes from TDDS channel, wrap them back into
 * a proper ddsi_serdata via from_ser_iov so rmw_cyclonedds can deserialize.
 * Returns number of samples read (0 or 1).
 */
dds_return_t dds_takecdr(dds_entity_t reader_or_condition,
                          struct ddsi_serdata **buf,
                          uint32_t maxs,
                          dds_sample_info_t *si,
                          uint32_t mask)
{
    (void)mask;
    if (!buf || maxs == 0 || !si) return DDS_RETCODE_BAD_PARAMETER;

    entity_t *r = resolve_reader(reader_or_condition);
    if (!r || r->type != ENTITY_READER) return DDS_RETCODE_BAD_PARAMETER;
    if (!r->sertype) return DDS_RETCODE_ERROR;

    static uint8_t recv_buf[TDDS_MAX_MSG_SIZE];
    int n = tdds_subscribe(&r->channel, recv_buf, sizeof(recv_buf));
    if (n <= 0) return 0;

    const struct ddsi_serdata_ops *ops = r->sertype->serdata_ops;
    if (!ops || !ops->from_ser_iov) return DDS_RETCODE_ERROR;

    ddsrt_iovec_t iov = { .iov_base = recv_buf, .iov_len = (size_t)n };
    struct ddsi_serdata *sd = ops->from_ser_iov(r->sertype, SDK_DATA, 1, &iov, (size_t)n);
    if (!sd) return 0;

    buf[0] = sd;
    si[0].valid_data              = true;
    si[0].sample_state            = DDS_SST_NOT_READ;
    si[0].view_state              = DDS_VST_NEW;
    si[0].instance_state          = DDS_IST_ALIVE;
    si[0].source_timestamp        = 0;
    si[0].instance_handle         = 0;
    si[0].publication_handle      = 0;
    si[0].disposed_generation_count     = 0;
    si[0].no_writers_generation_count   = 0;
    si[0].sample_rank             = 0;
    si[0].generation_rank         = 0;
    si[0].absolute_generation_rank = 0;

    return 1;
}

/* Non-CDR take: deserialize directly into caller's buffer */
dds_return_t dds_take(dds_entity_t reader_or_condition,
                       void **buf, dds_sample_info_t *si,
                       size_t bufsz, uint32_t maxs)
{
    (void)reader_or_condition; (void)buf; (void)si; (void)bufsz; (void)maxs;
    return 0;
}

/* ── Return loan ──────────────────────────────────────────────────────────── */

/*
 * dds_return_loan: release ddsi_serdata objects returned by takecdr.
 * Decrements the reference count; frees when it hits zero.
 */
dds_return_t dds_return_loan(dds_entity_t entity, void **buf, int32_t bufsz)
{
    (void)entity;
    for (int32_t i = 0; i < bufsz; i++) {
        struct ddsi_serdata *sd = (struct ddsi_serdata *)buf[i];
        if (!sd) continue;
        /* Atomic decrement of sd->refc_v; free if it drops to zero */
        uint32_t old = __atomic_fetch_sub(&sd->refc_v, 1u, __ATOMIC_ACQ_REL);
        if (old == 1 && sd->ops && sd->ops->free)
            sd->ops->free(sd);
        buf[i] = NULL;
    }
    return DDS_RETCODE_OK;
}

/* ── Delete ───────────────────────────────────────────────────────────────── */

dds_return_t dds_delete(dds_entity_t entity)
{
    entity_t *ent = entity_get(entity);
    if (!ent) return DDS_RETCODE_ALREADY_DELETED;

    if (ent->type == ENTITY_WRITER || ent->type == ENTITY_READER ||
        ent->type == ENTITY_TOPIC) {
        tdds_destroy_channel(&ent->channel);
    }

    memset(ent, 0, sizeof(*ent));
    return DDS_RETCODE_OK;
}

/* ── QoS ──────────────────────────────────────────────────────────────────── */

dds_qos_t *dds_create_qos(void)
{
    return (dds_qos_t *)pool_alloc(g_qos_pool, g_qos_used, MAX_QOS,
                                    sizeof(g_qos_pool[0]));
}

void dds_delete_qos(dds_qos_t *qos)
{
    pool_free(g_qos_pool, g_qos_used, MAX_QOS, sizeof(g_qos_pool[0]), qos);
}

dds_qos_t *dds_get_qos(dds_entity_t entity, dds_qos_t *qos)
{
    (void)entity;
    return qos;  /* return whatever was passed in, unchanged */
}

/* qset stubs — callers set QoS properties we silently ignore */
void dds_qset_reliability(dds_qos_t *q, uint32_t k, dds_duration_t d) { (void)q;(void)k;(void)d; }
void dds_qset_durability(dds_qos_t *q, uint32_t k) { (void)q;(void)k; }
void dds_qset_durability_service(dds_qos_t *q, dds_duration_t sd, uint32_t hk,
                                  int32_t msd, int32_t msbs, int32_t msibs) {
    (void)q;(void)sd;(void)hk;(void)msd;(void)msbs;(void)msibs;
}
void dds_qset_history(dds_qos_t *q, uint32_t k, int32_t d) { (void)q;(void)k;(void)d; }
void dds_qset_liveliness(dds_qos_t *q, uint32_t k, dds_duration_t d) { (void)q;(void)k;(void)d; }
void dds_qset_deadline(dds_qos_t *q, dds_duration_t d) { (void)q;(void)d; }
void dds_qset_lifespan(dds_qos_t *q, dds_duration_t d) { (void)q;(void)d; }
void dds_qset_userdata(dds_qos_t *q, const void *d, size_t sz) { (void)q;(void)d;(void)sz; }
void dds_qset_ignorelocal(dds_qos_t *q, uint32_t v) { (void)q;(void)v; }
void dds_qset_prop(dds_qos_t *q, const char *n, const char *v) { (void)q;(void)n;(void)v; }
void dds_qset_writer_data_lifecycle(dds_qos_t *q, bool v) { (void)q;(void)v; }

/* qget stubs — return false/0 so caller uses defaults */
bool dds_qget_reliability(const dds_qos_t *q, uint32_t *k, dds_duration_t *d) {
    (void)q; if(k)*k=0; if(d)*d=0; return false;
}
bool dds_qget_durability(const dds_qos_t *q, uint32_t *k) {
    (void)q; if(k)*k=0; return false;
}
bool dds_qget_history(const dds_qos_t *q, uint32_t *k, int32_t *d) {
    (void)q; if(k)*k=0; if(d)*d=0; return false;
}
bool dds_qget_liveliness(const dds_qos_t *q, uint32_t *k, dds_duration_t *d) {
    (void)q; if(k)*k=0; if(d)*d=0; return false;
}
bool dds_qget_deadline(const dds_qos_t *q, dds_duration_t *d) {
    (void)q; if(d)*d=0; return false;
}
bool dds_qget_lifespan(const dds_qos_t *q, dds_duration_t *d) {
    (void)q; if(d)*d=0; return false;
}
bool dds_qget_userdata(const dds_qos_t *q, void **d, size_t *sz) {
    (void)q; if(d)*d=NULL; if(sz)*sz=0; return false;
}

/* ── Listener ─────────────────────────────────────────────────────────────── */

dds_listener_t *dds_create_listener(void *arg)
{
    (void)arg;
    return (dds_listener_t *)pool_alloc(g_lst_pool, g_lst_used, MAX_LISTENERS,
                                         sizeof(g_lst_pool[0]));
}

void dds_delete_listener(dds_listener_t *listener)
{
    pool_free(g_lst_pool, g_lst_used, MAX_LISTENERS, sizeof(g_lst_pool[0]), listener);
}

/* lset stubs — callbacks registered on listeners we ignore */
void dds_lset_data_available_arg(dds_listener_t *l, void (*fn)(dds_entity_t r, void *a), void *a, bool reset) {
    (void)l;(void)fn;(void)a;(void)reset;
}
void dds_lset_inconsistent_topic_arg(dds_listener_t *l, void (*fn)(dds_entity_t t, const void *s, void *a), void *a, bool reset) {
    (void)l;(void)fn;(void)a;(void)reset;
}
void dds_lset_liveliness_changed_arg(dds_listener_t *l, void (*fn)(dds_entity_t r, const void *s, void *a), void *a, bool reset) {
    (void)l;(void)fn;(void)a;(void)reset;
}
void dds_lset_liveliness_lost_arg(dds_listener_t *l, void (*fn)(dds_entity_t w, const void *s, void *a), void *a, bool reset) {
    (void)l;(void)fn;(void)a;(void)reset;
}
void dds_lset_offered_deadline_missed_arg(dds_listener_t *l, void (*fn)(dds_entity_t w, const void *s, void *a), void *a, bool reset) {
    (void)l;(void)fn;(void)a;(void)reset;
}
void dds_lset_offered_incompatible_qos_arg(dds_listener_t *l, void (*fn)(dds_entity_t w, const void *s, void *a), void *a, bool reset) {
    (void)l;(void)fn;(void)a;(void)reset;
}
void dds_lset_publication_matched_arg(dds_listener_t *l, void (*fn)(dds_entity_t w, const void *s, void *a), void *a, bool reset) {
    (void)l;(void)fn;(void)a;(void)reset;
}
void dds_lset_requested_deadline_missed_arg(dds_listener_t *l, void (*fn)(dds_entity_t r, const void *s, void *a), void *a, bool reset) {
    (void)l;(void)fn;(void)a;(void)reset;
}
void dds_lset_requested_incompatible_qos_arg(dds_listener_t *l, void (*fn)(dds_entity_t r, const void *s, void *a), void *a, bool reset) {
    (void)l;(void)fn;(void)a;(void)reset;
}
void dds_lset_sample_lost_arg(dds_listener_t *l, void (*fn)(dds_entity_t r, const void *s, void *a), void *a, bool reset) {
    (void)l;(void)fn;(void)a;(void)reset;
}
void dds_lset_subscription_matched_arg(dds_listener_t *l, void (*fn)(dds_entity_t r, const void *s, void *a), void *a, bool reset) {
    (void)l;(void)fn;(void)a;(void)reset;
}

/* ── Utilities ────────────────────────────────────────────────────────────── */

dds_time_t dds_time(void) { return 0; }

void dds_sleepfor(dds_duration_t ns)
{
    /* No sleep primitive in enclave; busy-spin is not appropriate.
     * A real implementation could use a timed WAIT_SHM when available. */
    (void)ns;
}

void dds_free(void *ptr)
{
    /* No heap allocator in bare enclave — only free objects we allocated
     * ourselves (handled in return_loan / delete_qos / etc.). */
    (void)ptr;
}

const char *dds_strretcode(dds_return_t ret)
{
    switch (ret) {
    case DDS_RETCODE_OK:                   return "ok";
    case DDS_RETCODE_ERROR:                return "error";
    case DDS_RETCODE_UNSUPPORTED:          return "unsupported";
    case DDS_RETCODE_BAD_PARAMETER:        return "bad_parameter";
    case DDS_RETCODE_PRECONDITION_NOT_MET: return "precondition_not_met";
    case DDS_RETCODE_OUT_OF_RESOURCES:     return "out_of_resources";
    case DDS_RETCODE_TIMEOUT:              return "timeout";
    case DDS_RETCODE_NO_DATA:              return "no_data";
    case DDS_RETCODE_ILLEGAL_OPERATION:    return "illegal_operation";
    default:                               return "unknown";
    }
}

void dds_set_log_mask(uint32_t mask) { (void)mask; }

/* ── GuardCondition ───────────────────────────────────────────────────────── */

dds_entity_t dds_create_guardcondition(dds_entity_t participant)
{
    (void)participant;
    return entity_alloc(ENTITY_GUARDCONDITION);
}

dds_return_t dds_set_guardcondition(dds_entity_t guardcond, bool triggered)
{
    (void)guardcond; (void)triggered;
    return DDS_RETCODE_OK;
}

dds_return_t dds_take_guardcondition(dds_entity_t guardcond, bool *triggered)
{
    (void)guardcond;
    if (triggered) *triggered = false;
    return DDS_RETCODE_OK;
}

/* ── Misc status / info stubs ─────────────────────────────────────────────── */

dds_return_t dds_get_status_changes(dds_entity_t entity, uint32_t *status)
{
    (void)entity; if (status) *status = 0; return DDS_RETCODE_OK;
}

dds_return_t dds_set_status_mask(dds_entity_t entity, uint32_t mask)
{
    (void)entity; (void)mask; return DDS_RETCODE_OK;
}

dds_return_t dds_get_instance_handle(dds_entity_t entity, dds_instance_handle_t *ihdl)
{
    if (ihdl) *ihdl = (dds_instance_handle_t)entity;
    return DDS_RETCODE_OK;
}

dds_return_t dds_get_guid(dds_entity_t entity, uint8_t guid[16])
{
    if (guid) memset(guid, 0, 16);
    (void)entity;
    return DDS_RETCODE_OK;
}

dds_entity_t dds_get_topic(dds_entity_t entity)
{
    entity_t *ent = entity_get(entity);
    if (!ent) return DDS_RETCODE_BAD_PARAMETER;
    if (ent->type == ENTITY_WRITER || ent->type == ENTITY_READER)
        return ent->parent;
    return DDS_RETCODE_ILLEGAL_OPERATION;
}

dds_return_t dds_get_name(dds_entity_t topic, char *name, size_t size)
{
    (void)topic;
    if (name && size > 0) name[0] = '\0';
    return DDS_RETCODE_OK;
}

dds_return_t dds_wait_for_acks(dds_entity_t publisher_or_writer, dds_duration_t timeout)
{
    (void)publisher_or_writer; (void)timeout;
    return DDS_RETCODE_OK;
}

dds_return_t dds_assert_liveliness(dds_entity_t entity)
{
    (void)entity;
    return DDS_RETCODE_OK;
}

dds_return_t dds_is_loan_available(dds_entity_t entity, bool *is_available)
{
    (void)entity;
    if (is_available) *is_available = false;
    return DDS_RETCODE_OK;
}

dds_return_t dds_is_shared_memory_available(dds_entity_t entity, bool *is_available)
{
    (void)entity;
    if (is_available) *is_available = false;
    return DDS_RETCODE_OK;
}

/* ── Domain ───────────────────────────────────────────────────────────────── */

dds_entity_t dds_create_domain(dds_domainid_t domain, const char *config)
{
    (void)domain; (void)config;
    return entity_alloc(ENTITY_PARTICIPANT); /* domain is like a participant */
}

/* ── Matched endpoint discovery stubs ────────────────────────────────────── */

dds_return_t dds_get_matched_publications(dds_entity_t reader,
                                           dds_instance_handle_t *rdr_ih,
                                           size_t *n)
{
    (void)reader; (void)rdr_ih; if (n) *n = 0; return DDS_RETCODE_OK;
}

dds_return_t dds_get_matched_subscriptions(dds_entity_t writer,
                                            dds_instance_handle_t *wtr_ih,
                                            size_t *n)
{
    (void)writer; (void)wtr_ih; if (n) *n = 0; return DDS_RETCODE_OK;
}

dds_return_t dds_get_matched_publication_data(dds_entity_t reader,
                                               void *publication_data,
                                               dds_instance_handle_t ih)
{
    (void)reader; (void)publication_data; (void)ih;
    return DDS_RETCODE_UNSUPPORTED;
}

dds_return_t dds_get_matched_subscription_data(dds_entity_t writer,
                                                void *subscription_data,
                                                dds_instance_handle_t ih)
{
    (void)writer; (void)subscription_data; (void)ih;
    return DDS_RETCODE_UNSUPPORTED;
}

/* ── Status stubs ─────────────────────────────────────────────────────────── */

dds_return_t dds_get_inconsistent_topic_status(dds_entity_t topic, void *status) {
    (void)topic; (void)status; return DDS_RETCODE_OK; }
dds_return_t dds_get_liveliness_changed_status(dds_entity_t reader, void *status) {
    (void)reader; (void)status; return DDS_RETCODE_OK; }
dds_return_t dds_get_liveliness_lost_status(dds_entity_t writer, void *status) {
    (void)writer; (void)status; return DDS_RETCODE_OK; }
dds_return_t dds_get_offered_deadline_missed_status(dds_entity_t writer, void *status) {
    (void)writer; (void)status; return DDS_RETCODE_OK; }
dds_return_t dds_get_offered_incompatible_qos_status(dds_entity_t writer, void *status) {
    (void)writer; (void)status; return DDS_RETCODE_OK; }
dds_return_t dds_get_publication_matched_status(dds_entity_t writer, void *status) {
    (void)writer; (void)status; return DDS_RETCODE_OK; }
dds_return_t dds_get_requested_deadline_missed_status(dds_entity_t reader, void *status) {
    (void)reader; (void)status; return DDS_RETCODE_OK; }
dds_return_t dds_get_requested_incompatible_qos_status(dds_entity_t reader, void *status) {
    (void)reader; (void)status; return DDS_RETCODE_OK; }
dds_return_t dds_get_sample_lost_status(dds_entity_t reader, void *status) {
    (void)reader; (void)status; return DDS_RETCODE_OK; }
dds_return_t dds_get_subscription_matched_status(dds_entity_t reader, void *status) {
    (void)reader; (void)status; return DDS_RETCODE_OK; }

/* ── Data allocator stubs ─────────────────────────────────────────────────── */

dds_return_t dds_data_allocator_init(dds_entity_t entity, void *data_allocator) {
    (void)entity; (void)data_allocator; return DDS_RETCODE_OK; }
dds_return_t dds_data_allocator_init_heap(void *data_allocator) {
    (void)data_allocator; return DDS_RETCODE_OK; }
dds_return_t dds_data_allocator_fini(void *data_allocator) {
    (void)data_allocator; return DDS_RETCODE_OK; }
void *dds_data_allocator_alloc(void *data_allocator, size_t size) {
    (void)data_allocator; (void)size; return NULL; }
dds_return_t dds_data_allocator_free(void *data_allocator, void *ptr) {
    (void)data_allocator; (void)ptr; return DDS_RETCODE_OK; }
