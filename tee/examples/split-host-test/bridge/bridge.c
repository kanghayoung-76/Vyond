/*
 * split-host-test: bridge enclave (enc1)
 *
 * Scenario: enc1 publishes directly to enc2's TDDS channel (no DMA device).
 *
 * Flow:
 *   1. find_shm_by_hash(EXPECTED_ENC2_HASH, &rid_tdds) — locate enc2's channel
 *   2. create_publisher(&pub, rid_tdds)
 *   3. For N_ITERS: publish "Hello from enc1!" directly into enc2's channel
 *   4. OCALL_DONE to signal completion
 *
 * OCALLs:
 *   OCALL_PRINT (3): print a string via host
 *   OCALL_DONE  (6): signals bridge-host all iterations complete
 */
#include "app/eapp_utils.h"
#include "app/string.h"
#include "app/syscall.h"
#include "tdds_rclcpp.h"

#define OCALL_PRINT 3
#define OCALL_DONE  6

#define N_ITERS 3

/* enc2's measurement hash — known at build time.
 * {0} = wildcard for testing. Replace with actual hash in production. */
static const uint8_t EXPECTED_ENC2_HASH[64] = {0};

int main(void)
{
    __asm__ __volatile__(
        ".option push\n\t.option norelax\n\t"
        "la gp, __global_pointer$\n\t"
        ".option pop\n\t" ::: "memory");

    ocall(OCALL_PRINT, (void*)"[ENC1] main() reached", 22, NULL, 0);

    /* --- Locate enc2's TDDS channel (retry until enc2 publishes it) --- */
    rid_t rid_tdds = 0;
    {
        int tries;
        for (tries = 0; tries < 100000; tries++) {
            if (find_shm_by_hash(EXPECTED_ENC2_HASH, &rid_tdds) == 0)
                break;
            /* enc2 not ready yet — spin briefly then retry */
            volatile int spin;
            for (spin = 0; spin < 50000; spin++) ;
        }
        if (tries == 100000) {
            ocall(OCALL_PRINT, (void*)"[ENC1] find_shm_by_hash timed out", 34, NULL, 0);
            EAPP_RETURN(1);
        }
    }

    /* --- Create publisher on the enc1-enc2 TDDS channel --- */
    Publisher pub;
    if (create_publisher(&pub, rid_tdds) != 0) EAPP_RETURN(1);

    /* --- Main loop: publish directly to enc2 --- */
    for (int i = 0; i < N_ITERS; i++) {
        /* Print iteration separator — data must be in UTM for ocall to work. */
        {
            size_t avail;
            char *utm = (char *)tdds_utm_data(&avail);
            if (utm && avail >= 30) {
                char sep[30] = "[ENC1] ===== iteration X =====";
                sep[23] = '0' + i;
                memcpy(utm, sep, 30);
                ocall(OCALL_PRINT, utm, 30, NULL, 0);
            }
        }

        /* Write message and publish directly to enc2 */
        char msg[TDDS_MAX_MSG_SIZE];
        memset(msg, 0, sizeof(msg));
        memcpy(msg, "Hello from enc1!", 16);
        publish(&pub, msg, TDDS_MAX_MSG_SIZE);
    }

    ocall(OCALL_DONE, NULL, 0, NULL, 0);
    EAPP_RETURN(0);
}
