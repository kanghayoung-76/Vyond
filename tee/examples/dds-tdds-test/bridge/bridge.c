/*
 * dds-tdds-test: bridge enclave (Stage 3 — MYDEV DMA flow)
 *
 * Flow: MYDEV DMA → dev-SHM → IRQ → enc1 resumes → read(dev-SHM)
 *       → publish(TDDS SHM2) + NOTIFY_SHM → enc2 wakes
 *
 * OCALLs:
 *   OCALL_LOAN_DEV_SHM (4): host returns tdds_dev_shm_t{rid, pa, size} for device SHM
 *   OCALL_GET_RID_OUT  (5): host returns rid of the TDDS enc1↔enc2 channel
 */
#include "app/eapp_utils.h"
#include "app/string.h"
#include "app/syscall.h"
#include "shared/tdds_common.h"
#include "tdds_rclcpp.h"

/* MYDEV MMIO register offsets */
#define MYDEV_BASE         0x6004000UL
#define MYDEV_SIZE         0x1000
#define MYDEV_OFF_CMD      0x00
#define MYDEV_OFF_DMA_LOW  0x08
#define MYDEV_OFF_DMA_HIGH 0x0c
#define MYDEV_OFF_DMA_LEN  0x10

#define MYDEV_IRQ  19

/* OCALL_LOAN_DEV_SHM=4 is defined in shared/tdds_common.h (included via tdds_rclcpp.h) */
#define OCALL_GET_RID_OUT   5

static inline uint32_t mmio_read(uintptr_t a)              { return *(volatile uint32_t *)a; }
static inline void     mmio_write(uintptr_t a, uint32_t v) { *(volatile uint32_t *)a = v; }

int main(void)
{
    __asm__ __volatile__(
        ".option push\n\t.option norelax\n\t"
        "la gp, __global_pointer$\n\t"
        ".option pop\n\t" ::: "memory");

    /* Step 1: get dev-SHM {rid, pa, size} from host via pointer-return OCALL */
    uintptr_t dev_shm_ptr = 0;
    ocall(OCALL_LOAN_DEV_SHM, NULL, 0, &dev_shm_ptr, sizeof(dev_shm_ptr));
    tdds_dev_shm_t *dev_shm = (tdds_dev_shm_t *)dev_shm_ptr;

    /* Step 2: map dev-SHM physical pages into enclave VA */
    char *dma_buf = (char *)map_shm(dev_shm->rid);
    if (!dma_buf) EAPP_RETURN(1);

    /* Step 3: map MYDEV MMIO */
    uintptr_t mydev = 0;
    SYSCALL(RUNTIME_SYSCALL_MYDEV_MAP, MYDEV_BASE, MYDEV_SIZE,
            (uintptr_t)&mydev, 0, 0);
    if (!mydev) EAPP_RETURN(1);

    /* Step 4: configure DMA target = dev-SHM PA */
    mmio_write(mydev + MYDEV_OFF_DMA_LOW,  (uint32_t)(dev_shm->pa & 0xffffffffUL));
    mmio_write(mydev + MYDEV_OFF_DMA_HIGH, (uint32_t)(dev_shm->pa >> 32));
    mmio_write(mydev + MYDEV_OFF_DMA_LEN,  (uint32_t)dev_shm->size);

    /* Step 5: register IRQ with SM (M-mode intercept) */
    SYSCALL_1(RUNTIME_SYSCALL_REGISTER_DEV_IRQ, MYDEV_IRQ);

    /* Step 6: trigger DMA, then suspend until IRQ fires */
    mmio_write(mydev + MYDEV_OFF_CMD, 1);
    SYSCALL_1(RUNTIME_SYSCALL_WAIT_DEV_DATA, MYDEV_IRQ);

    /* Step 7: DMA complete — MYDEV writes raw bytes, no size header */
    uint32_t n = (uint32_t)dev_shm->size;
    if (n > TDDS_MAX_MSG_SIZE) n = TDDS_MAX_MSG_SIZE;

    /* Step 8: get TDDS channel RID for enc1↔enc2 */
    uintptr_t ret_ptr = 0;
    ocall(OCALL_GET_RID_OUT, NULL, 0, &ret_ptr, sizeof(ret_ptr));
    rid_t rid_tdds = *(rid_t *)ret_ptr;

    /* Step 9: publish from dev-SHM → TDDS SHM; loop 3 times.
     * notify_shm in publish() triggers an immediate SM context-switch to enc2.
     * enc1 is suspended until enc2 calls wait_shm again, then SM resumes enc1 here. */
    Publisher pub;
    if (create_publisher(&pub, rid_tdds) != 0) EAPP_RETURN(1);

    for (int i = 0; i < 3; i++) {
        /* Re-trigger DMA each iteration (same PA, same len) */
        mmio_write(mydev + MYDEV_OFF_CMD, 1);
        SYSCALL_1(RUNTIME_SYSCALL_WAIT_DEV_DATA, MYDEV_IRQ);
        publish(&pub, dma_buf, n); /* → notify_shm → SM switches to enc2 */
        /* enc1 resumes here after enc2 calls wait_shm (next iteration or exit) */
    }

    EAPP_RETURN(0);
}
