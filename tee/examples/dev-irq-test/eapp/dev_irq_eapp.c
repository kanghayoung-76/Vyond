/*
 * dev-irq-test enclave application
 *
 * Demonstrates the secure device-to-enclave data path:
 *   Device DMA → dev-SHM (host-allocated, device_wid + enclave_wid permitted)
 *   Device IRQ  → SM (M-mode)        → enclave resumed directly
 *
 * Per-iteration flow:
 *   1. Read dev-SHM rid from shared buffer; map it to get VA+PA
 *   2. Configure MYDEV DMA target = dev-SHM physical address
 *   3. Register the IRQ with SM so M-mode intercepts it (not host Linux)
 *   4. Trigger DMA  (CMD=1 : device → guest)
 *   5. Call wait_dev_data — SM suspends us, resumes when IRQ fires
 *   6. Read data from the dev-SHM buffer
 */

#include "app/eapp_utils.h"
#include "app/syscall.h"
#include "app/string.h"
#include "edge_wrapper.h"

/* MYDEV MMIO register offsets (qemu/hw/misc/mydev_dma.c) */
#define MYDEV_BASE          0x6004000UL
#define MYDEV_SIZE          0x1000
#define MYDEV_OFF_CMD       0x00
#define MYDEV_OFF_DMA_LOW   0x08
#define MYDEV_OFF_DMA_HIGH  0x0c
#define MYDEV_OFF_DMA_LEN   0x10
#define MYDEV_OFF_STATUS    0x18

/* IRQ 19 = MYDEV_DMA_IRQ (qemu/include/hw/riscv/virt.h) */
#define MYDEV_IRQ  19

#define ITERATIONS  3
#define BUF_SIZE   64

static inline uint32_t mmio_read(uintptr_t a)         { return *(volatile uint32_t *)a; }
static inline void      mmio_write(uintptr_t a, uint32_t v) { *(volatile uint32_t *)a = v; }

/* Runtime syscall helpers ------------------------------------------------- */

static uintptr_t eapp_mydev_map(void) {
    uintptr_t va = 0;
    SYSCALL(RUNTIME_SYSCALL_MYDEV_MAP, MYDEV_BASE, MYDEV_SIZE, (uintptr_t)&va, 0, 0);
    return va;
}

/* Tell SM: "route MYDEV_IRQ to this enclave; intercept in M-mode." */
static void eapp_register_dev_irq(uint32_t irq) {
    SYSCALL_1(RUNTIME_SYSCALL_REGISTER_DEV_IRQ, irq);
}

/* Suspend until SM resumes us after the device IRQ fires. */
static void eapp_wait_dev_data(uint32_t irq) {
    SYSCALL_1(RUNTIME_SYSCALL_WAIT_DEV_DATA, irq);
}

void EAPP_ENTRY eapp_entry(void)
{
    ocall_print_buffer("=== dev-irq-test enclave started ===\n",
                       sizeof("=== dev-irq-test enclave started ===\n"));

    /* Get dev-SHM info from host (zerocopy: host returns rid+pa+size) */
    shm_t dev_shm = ocall_loan_shm(0);
    ocall_print_buffer("[eapp] dev-SHM rid: ", sizeof("[eapp] dev-SHM rid: "));
    ocall_print_value((unsigned long)dev_shm.rid);
    ocall_print_buffer("[eapp] dev-SHM PA:  ", sizeof("[eapp] dev-SHM PA:  "));
    ocall_print_value(dev_shm.pa);

    /* Map dev-SHM physical pages directly into enclave VA (PTE_U — no copy) */
    char *dma_va = (char *)map_shm(dev_shm.rid);
    if (!dma_va) {
        ocall_print_buffer("[eapp] ERROR: map_shm failed\n",
                           sizeof("[eapp] ERROR: map_shm failed\n"));
        EAPP_RETURN(1);
    }

    /* Map MYDEV MMIO into the enclave VA space */
    uintptr_t mydev = eapp_mydev_map();
    if (!mydev) {
        ocall_print_buffer("[eapp] ERROR: mydev_map failed\n",
                           sizeof("[eapp] ERROR: mydev_map failed\n"));
        EAPP_RETURN(1);
    }

    /* Configure MYDEV: use host-provided PA (no eapp_translate needed) */
    mmio_write(mydev + MYDEV_OFF_DMA_LOW,  (uint32_t)(dev_shm.pa & 0xffffffffUL));
    mmio_write(mydev + MYDEV_OFF_DMA_HIGH, (uint32_t)(dev_shm.pa >> 32));
    mmio_write(mydev + MYDEV_OFF_DMA_LEN,  BUF_SIZE);

    /* Register IRQ with SM once: enables PLIC M-mode, disables S-mode */
    eapp_register_dev_irq(MYDEV_IRQ);

    for (int i = 0; i < ITERATIONS; i++) {
        ocall_print_buffer("[eapp] --- iteration start ---\n",
                           sizeof("[eapp] --- iteration start ---\n"));

        mmio_write(mydev + MYDEV_OFF_CMD, 1);
        eapp_wait_dev_data(MYDEV_IRQ);

        /* ── We are back: SM intercepted IRQ 19, switched us in directly ── */
        ocall_print_buffer("[eapp] resumed via device IRQ — first 8 bytes: ",
                           sizeof("[eapp] resumed via device IRQ — first 8 bytes: "));
        for (int j = 0; j < 8; j++)
            ocall_print_value((unsigned long)(unsigned char)dma_va[j]);
    }

    ocall_print_buffer("[eapp] done\n", sizeof("[eapp] done\n"));
    EAPP_RETURN(0);
}
