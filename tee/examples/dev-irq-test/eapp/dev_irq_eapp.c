/*
 * dev-irq-test enclave application
 *
 * Demonstrates the secure device-to-enclave data path:
 *   Device DMA → enclave EPM buffer  (host OS has NO access)
 *   Device IRQ  → SM (M-mode)        → enclave resumed directly
 *
 * Per-iteration flow:
 *   1. Configure MYDEV DMA target = our private EPM buffer (physical address)
 *   2. Register the IRQ with SM so M-mode intercepts it (not host Linux)
 *   3. Trigger DMA  (CMD=1 : device → guest)
 *   4. Call wait_dev_data — SM suspends us, resumes when IRQ fires
 *   5. Read data from the buffer (data arrived while we were suspended)
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

/* Ask the runtime to walk the enclave page table and return the PA for va. */
static uintptr_t eapp_translate(uintptr_t va) {
    return SYSCALL_1(RUNTIME_SYSCALL_TRANSLATE_VA, va);
}

/* Tell SM: "route MYDEV_IRQ to this enclave; intercept in M-mode." */
static void eapp_register_dev_irq(uint32_t irq) {
    SYSCALL_1(RUNTIME_SYSCALL_REGISTER_DEV_IRQ, irq);
}

/* Suspend until SM resumes us after the device IRQ fires. */
static void eapp_wait_dev_data(uint32_t irq) {
    SYSCALL_1(RUNTIME_SYSCALL_WAIT_DEV_DATA, irq);
}

/* DMA buffer in the enclave's private EPM — the host cannot touch it. */
static char __attribute__((aligned(4096))) dma_buf[BUF_SIZE];

void EAPP_ENTRY eapp_entry(void)
{
    ocall_print_buffer("=== dev-irq-test enclave started ===\n",
                       sizeof("=== dev-irq-test enclave started ===\n"));

    /* Map MYDEV MMIO into the enclave VA space */
    uintptr_t mydev = eapp_mydev_map();
    if (!mydev) {
        ocall_print_buffer("[eapp] ERROR: mydev_map failed\n",
                           sizeof("[eapp] ERROR: mydev_map failed\n"));
        EAPP_RETURN(1);
    }

    /* Get physical address of our private DMA buffer */
    uintptr_t dma_pa = eapp_translate((uintptr_t)dma_buf);
    ocall_print_buffer("[eapp] DMA buf PA: ", sizeof("[eapp] DMA buf PA: "));
    ocall_print_value(dma_pa);

    /* Configure MYDEV: write DMA target address and length once */
    mmio_write(mydev + MYDEV_OFF_DMA_LOW,  (uint32_t)(dma_pa & 0xffffffffUL));
    mmio_write(mydev + MYDEV_OFF_DMA_HIGH, (uint32_t)(dma_pa >> 32));
    mmio_write(mydev + MYDEV_OFF_DMA_LEN,  BUF_SIZE);

    /* Register IRQ with SM once: enables PLIC M-mode, disables S-mode */
    eapp_register_dev_irq(MYDEV_IRQ);

    for (int i = 0; i < ITERATIONS; i++) {
        ocall_print_buffer("[eapp] --- iteration start ---\n",
                           sizeof("[eapp] --- iteration start ---\n"));

        /*
         * Trigger device→enclave DMA.
         * In QEMU, mydev_dma is synchronous: the write to CMD immediately
         * calls cpu_physical_memory_write(dma_pa, ...) and then raises IRQ 19.
         * The IRQ becomes pending in the PLIC before we even call wait_dev_data.
         */
        mmio_write(mydev + MYDEV_OFF_CMD, 1);

        /*
         * Yield to host OS; the SM will resume this enclave directly when
         * IRQ 19 fires — without involving the host OS at all.
         * Because the IRQ is already pending at this point (QEMU DMA is
         * synchronous), the SM will see MEIP set as soon as it mrets to
         * host and will immediately switch back to us.
         */
        eapp_wait_dev_data(MYDEV_IRQ);

        /* ── We are back: SM intercepted IRQ 19, switched us in directly ── */
        ocall_print_buffer("[eapp] resumed via device IRQ — first 8 bytes: ",
                           sizeof("[eapp] resumed via device IRQ — first 8 bytes: "));
        for (int j = 0; j < 8; j++)
            ocall_print_value((unsigned long)(unsigned char)dma_buf[j]);
    }

    ocall_print_buffer("[eapp] done\n", sizeof("[eapp] done\n"));
    EAPP_RETURN(0);
}
