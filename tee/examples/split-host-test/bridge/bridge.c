/*
 * split-host-test: bridge enclave (enc1)
 *
 * Scenario: MYDEV DMA → dev-SHM → IRQ → enc1 → publish(TDDS SHM) → enc2 wakes
 *
 * Hash-based attestation (enc1 never writes MMIO):
 *   1. find_dev_shm(&rid_dev)       — SM finds pre-registered RegionDevEnc for WID=29.
 *   2. map_shm(rid_dev)             — SM verifies hash, writes DMA_LOW/HIGH/LEN.
 *   3. trigger_dev(MYDEV_WID)       — SM writes CMD=1 (enc1 never touches MMIO).
 *
 *   4. find_shm_by_hash(EXPECTED_ENC2_HASH, &rid_tdds)
 *        — SM finds the RegionEncEnc published by enc2.
 *        — EXPECTED_ENC2_HASH: enc2's measurement hash, known at build time.
 *          For testing: {0} = wildcard (any enc-enc channel this enclave can subscribe to).
 *          In production: replace with the actual measured hash of the target enc2 binary.
 *
 * OCALLs:
 *   OCALL_DONE (6): signals bridge-host all iterations complete
 */
#include "app/eapp_utils.h"
#include "app/string.h"
#include "app/syscall.h"
#include "tdds_rclcpp.h"

#define MYDEV_WID   29
#define MYDEV_IRQ   19

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

    /* --- Dev SHM: SM finds pre-registered RegionDevEnc for WID=29 --- */
    rid_t rid_dev = 0;
    if (find_dev_shm(&rid_dev) != 0) {
        ocall(OCALL_PRINT, (void*)"[ENC1] find_dev_shm FAILED", 27, NULL, 0);
        EAPP_RETURN(1);
    }
    ocall(OCALL_PRINT, (void*)"[ENC1] find_dev_shm OK", 23, NULL, 0);

    /* map_shm triggers SM to write DMA_LOW/HIGH/LEN registers on our behalf */
    char *dma_buf = (char *)map_shm(rid_dev);
    if (!dma_buf) EAPP_RETURN(1);

    /* --- Register device IRQ with SM --- */
    SYSCALL_1(RUNTIME_SYSCALL_REGISTER_DEV_IRQ, MYDEV_IRQ);

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

    /* --- Create publisher on the enc1↔enc2 TDDS channel --- */
    Publisher pub;
    if (create_publisher(&pub, rid_tdds) != 0) EAPP_RETURN(1);

    /* --- Main loop: trigger DMA → wait IRQ → publish (SM switches to enc2) --- */
    for (int i = 0; i < N_ITERS; i++) {
        /* SM writes CMD=1 to MYDEV MMIO register — enc1 never touches MMIO */
        trigger_dev(MYDEV_WID);
        SYSCALL_1(RUNTIME_SYSCALL_WAIT_DEV_DATA, MYDEV_IRQ);
        /* SM direct-switches enc1→enc2 inside publish() → notify_shm().
         * enc1 resumes here when enc2 calls wait_shm() (next iteration or exit). */
        publish(&pub, dma_buf, TDDS_MAX_MSG_SIZE);
    }

    ocall(OCALL_DONE, NULL, 0, NULL, 0);
    EAPP_RETURN(0);
}
