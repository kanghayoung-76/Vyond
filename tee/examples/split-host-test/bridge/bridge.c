/*
 * split-host-test: bridge enclave (enc1)
 *
 * Scenario: MYDEV DMA → dev-SHM → IRQ → enc1 → publish(TDDS SHM) → enc2 wakes
 *
 * With the new SM, notify_shm() directly suspends enc1 and resumes enc2 (same core).
 * enc1 resumes here after enc2 calls wait_shm() for the next round.
 *
 * OCALLs:
 *   OCALL_LOAN_DEV_SHM (4): bridge-host returns {rid, pa, size} of device SHM
 *   OCALL_GET_TDDS_RID  (5): bridge-host returns rid of the enc1↔enc2 TDDS channel
 *   OCALL_DONE          (6): signals bridge-host that all iterations are complete
 */
#include "app/eapp_utils.h"
#include "app/string.h"
#include "app/syscall.h"
#include "tdds_rclcpp.h"
#include "shared/tdds_common.h"

#define MYDEV_BASE         0x6004000UL
#define MYDEV_SIZE         0x1000
#define MYDEV_OFF_CMD      0x00
#define MYDEV_OFF_DMA_LOW  0x08
#define MYDEV_OFF_DMA_HIGH 0x0c
#define MYDEV_OFF_DMA_LEN  0x10
#define MYDEV_IRQ          19

#define OCALL_GET_TDDS_RID  5
#define OCALL_DONE          6

#define N_ITERS 3

static inline void mmio_write(uintptr_t a, uint32_t v) { *(volatile uint32_t *)a = v; }

int main(void)
{
    __asm__ __volatile__(
        ".option push\n\t.option norelax\n\t"
        "la gp, __global_pointer$\n\t"
        ".option pop\n\t" ::: "memory");

    /* --- Dev SHM: get metadata from bridge-host --- */
    uintptr_t dev_shm_ptr = 0;
    ocall(OCALL_LOAN_DEV_SHM, NULL, 0, &dev_shm_ptr, sizeof(dev_shm_ptr));
    tdds_dev_shm_t *dev_shm = (tdds_dev_shm_t *)dev_shm_ptr;

    char *dma_buf = (char *)map_shm(dev_shm->rid);
    if (!dma_buf) EAPP_RETURN(1);

    /* --- MYDEV MMIO: map and configure DMA target --- */
    uintptr_t mydev = 0;
    SYSCALL(RUNTIME_SYSCALL_MYDEV_MAP, MYDEV_BASE, MYDEV_SIZE,
            (uintptr_t)&mydev, 0, 0);
    if (!mydev) EAPP_RETURN(1);

    /* Save n before any further OCALL that may overwrite the UTM return area */
    uint32_t n = (uint32_t)dev_shm->size;
    if (n > TDDS_MAX_MSG_SIZE) n = TDDS_MAX_MSG_SIZE;

    mmio_write(mydev + MYDEV_OFF_DMA_LOW,  (uint32_t)(dev_shm->pa & 0xffffffffUL));
    mmio_write(mydev + MYDEV_OFF_DMA_HIGH, (uint32_t)(dev_shm->pa >> 32));
    mmio_write(mydev + MYDEV_OFF_DMA_LEN,  (uint32_t)dev_shm->size);

    /* --- Register device IRQ with SM --- */
    SYSCALL_1(RUNTIME_SYSCALL_REGISTER_DEV_IRQ, MYDEV_IRQ);

    /* --- Get TDDS channel RID (2nd OCALL — overwrites UTM return area, dev_shm stale after) --- */
    uintptr_t rid_ptr = 0;
    ocall(OCALL_GET_TDDS_RID, NULL, 0, &rid_ptr, sizeof(rid_ptr));
    rid_t rid_tdds = *(rid_t *)rid_ptr;

    /* --- Create publisher on the enc1↔enc2 TDDS channel --- */
    Publisher pub;
    if (create_publisher(&pub, rid_tdds) != 0) EAPP_RETURN(1);

    /* --- Main loop: DMA → wait IRQ → publish (SM switches to enc2) --- */
    for (int i = 0; i < N_ITERS; i++) {
        mmio_write(mydev + MYDEV_OFF_CMD, 1);
        SYSCALL_1(RUNTIME_SYSCALL_WAIT_DEV_DATA, MYDEV_IRQ);
        /* SM direct-switches enc1→enc2 inside publish() → notify_shm().
         * enc1 resumes here when enc2 calls wait_shm() (next iteration or exit). */
        publish(&pub, dma_buf, n);
    }

    /* Signal bridge-host: all iterations done. */
    ocall(OCALL_DONE, NULL, 0, NULL, 0);

    EAPP_RETURN(0);
}
