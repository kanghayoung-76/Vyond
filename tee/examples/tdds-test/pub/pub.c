#include "eapp_utils.h"
#include "string.h"
#include "syscall.h"
#include "tdds.h"

#define OCALL_GET_RID      2
#define OCALL_PRINT        3

#define MYDEV_BASE         0x6004000UL
#define MYDEV_SIZE         0x1000
#define MYDEV_OFF_CMD      0x00
#define MYDEV_OFF_DMA_LOW  0x08
#define MYDEV_OFF_DMA_HIGH 0x0c
#define MYDEV_OFF_DMA_LEN  0x10
#define MYDEV_IRQ          19
#define BUF_SIZE           64

static inline void mmio_write(uintptr_t a, uint32_t v) { *(volatile uint32_t *)a = v; }

static void eprint(const char *msg) {
    size_t avail;
    char *dst = (char *)tdds_utm_data(&avail);
    size_t len = strlen(msg) + 1;
    if (len > avail) len = avail;
    for (size_t i = 0; i < len; i++) dst[i] = msg[i];
    ocall(OCALL_PRINT, dst, len, NULL, 0);
}

int main() {
    __asm__ __volatile__(
        ".option push\n\t.option norelax\n\t"
        "la gp, __global_pointer$\n\t"
        ".option pop\n\t" ::: "memory");

    /* TDDS channel RID */
    uintptr_t ret_ptr = 0;
    ocall(OCALL_GET_RID, NULL, 0, &ret_ptr, sizeof(ret_ptr));
    rid_t rid = *(rid_t *)ret_ptr;

    /* device SHM info (host fills on request; NULL args bypasses UTM range check) */
    uintptr_t dev_ret_ptr = 0;
    ocall(OCALL_LOAN_DEV_SHM, NULL, 0, &dev_ret_ptr, sizeof(dev_ret_ptr));
    tdds_dev_shm_t *dsi = (tdds_dev_shm_t *)dev_ret_ptr;
    rid_t     dev_rid = dsi->rid;
    uintptr_t dev_pa  = dsi->pa;

    tdds_channel_t ch;
    if (tdds_init_channel(&ch, rid) != 0) { eprint("[PUB] init failed"); EAPP_RETURN(1); }

    char *dma_va = (char *)map_shm(dev_rid);
    if (!dma_va) { eprint("[PUB] map_shm failed"); EAPP_RETURN(1); }

    uintptr_t mydev = (uintptr_t)mydev_map(MYDEV_BASE, MYDEV_SIZE);
    if (!mydev) { eprint("[PUB] mydev_map failed"); EAPP_RETURN(1); }

    mmio_write(mydev + MYDEV_OFF_DMA_LOW,  (uint32_t)(dev_pa & 0xffffffffUL));
    mmio_write(mydev + MYDEV_OFF_DMA_HIGH, (uint32_t)(dev_pa >> 32));
    mmio_write(mydev + MYDEV_OFF_DMA_LEN,  BUF_SIZE);
    SYSCALL_1(RUNTIME_SYSCALL_REGISTER_DEV_IRQ, MYDEV_IRQ);

    mmio_write(mydev + MYDEV_OFF_CMD, 1);
    SYSCALL_1(RUNTIME_SYSCALL_WAIT_DEV_DATA, MYDEV_IRQ);

    if (tdds_publish(&ch, dma_va, BUF_SIZE) != 0) {
        eprint("[PUB] publish failed"); EAPP_RETURN(1);
    }

    tdds_destroy_channel(&ch);
    EAPP_RETURN(0);
}
