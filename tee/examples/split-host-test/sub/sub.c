/*
 * split-host-test: subscriber enclave (enc2)
 *
 * sub-host initializes this enclave and parks it at wait_shm().
 * enc1 (bridge) calls notify_shm() → SM directly resumes enc2 here.
 *
 * Hash-based attestation:
 *   1. create_subscription(rid) — map the TDDS SHM.
 *   2. register_enc_channel(rid, open_hash) — promote to RegionEncEnc.
 *        SM records enc2.hash as creator_hash; enc1 finds it via find_shm_by_hash.
 *        open_hash = {0}: any allowed enclave may subscribe (for testing).
 *   3. wait(&sub) — park; sub-host signals bridge-host.
 *
 * OCALLs:
 *   OCALL_GET_TDDS_RID (5): sub-host returns rid of the TDDS channel
 *   OCALL_PRINT        (3): print a string via sub-host
 */
#include "app/eapp_utils.h"
#include "app/string.h"
#include "app/syscall.h"
#include "tdds_rclcpp.h"

#define OCALL_PRINT        3
#define OCALL_GET_TDDS_RID 5

#define N_ITERS 3

static void eprint(const char *msg)
{
    size_t avail;
    char *dst = (char *)tdds_utm_data(&avail);
    size_t len = strlen(msg) + 1;
    if (len > avail) len = avail;
    for (size_t i = 0; i < len; i++) dst[i] = msg[i];
    ocall(OCALL_PRINT, dst, len, NULL, 0);
}

int main(void)
{
    __asm__ __volatile__(
        ".option push\n\t.option norelax\n\t"
        "la gp, __global_pointer$\n\t"
        ".option pop\n\t" ::: "memory");

    /* Get TDDS channel RID from sub-host */
    uintptr_t rid_ptr = 0;
    ocall(OCALL_GET_TDDS_RID, NULL, 0, &rid_ptr, sizeof(rid_ptr));
    rid_t rid = *(rid_t *)rid_ptr;

    Subscription sub;
    if (create_subscription(&sub, rid) != 0) {
        eprint("[SUB] init fail");
        EAPP_RETURN(1);
    }

    /* Promote TDDS SHM to hash-attested enc-enc channel.
     * SM records enc2.hash as creator_hash.
     * enc1 finds this channel via find_shm_by_hash(EXPECTED_ENC2_HASH).
     * open_hash = {0}: enc1's allowed_hash check is wildcard for testing. */
    static const uint8_t open_hash[64] = {0};
    register_enc_channel(rid, open_hash);

    /* Park: SM suspends enc2, returns WaitingForShm to sub-host.
     * sub-host signals bridge-host. When enc1 calls notify_shm,
     * SM directly resumes enc2. */
    wait(&sub);

    char buf[TDDS_MAX_MSG_SIZE + 1];
    for (int i = 0; i < N_ITERS; i++) {
        /* retry until real data arrives: spurious wakeups return TDDS_NOT_READY */
        int n;
        do {
            n = take(&sub, buf, TDDS_MAX_MSG_SIZE);
            if (n <= 0) wait(&sub);
        } while (n <= 0);
        buf[n] = '\0';
        eprint("[ENC2] received: ");
        eprint(buf);
        if (i < N_ITERS - 1)
            wait(&sub);
    }

    EAPP_RETURN(0);
}
