// SPDX-License-Identifier: BSD-2-Clause
//
// Simple bitmap-based allocator for the enclave pool region.
// The pool (ENCLAVE_POOL_PHYS .. +ENCLAVE_POOL_SIZE) is physical RAM
// that Linux doesn't manage (excluded by mem=4096M).  We map it once
// with memremap(MEMREMAP_WB) so the driver can zero-initialise pages
// and pass kernel virtual addresses to callers.

#include <linux/kernel.h>
#include <linux/bitmap.h>
#include <linux/spinlock.h>
#include <linux/io.h>
#include <linux/mm.h>
#include "keystone.h"

/* Must match wg::ENCLAVE_POOL_BASE / ENCLAVE_POOL_SIZE in the SM */
#define ENCLAVE_POOL_PHYS  0x180000000UL
#define ENCLAVE_POOL_SIZE  0x100000000UL   /* 4 GiB */
#define POOL_PAGES         (ENCLAVE_POOL_SIZE / PAGE_SIZE)

static DECLARE_BITMAP(pool_bitmap, POOL_PAGES);
static DEFINE_SPINLOCK(pool_lock);
static void *pool_vbase;

int enclave_pool_init(void)
{
	pool_vbase = memremap(ENCLAVE_POOL_PHYS, ENCLAVE_POOL_SIZE, MEMREMAP_WB);
	if (!pool_vbase) {
		pr_err("keystone: failed to memremap enclave pool\n");
		return -ENOMEM;
	}
	bitmap_zero(pool_bitmap, POOL_PAGES);
	pr_info("keystone: enclave pool mapped virt=%p phys=0x%lx size=4GiB\n",
		pool_vbase, ENCLAVE_POOL_PHYS);
	return 0;
}

void enclave_pool_exit(void)
{
	if (pool_vbase) {
		memunmap(pool_vbase);
		pool_vbase = NULL;
	}
}

/* Allocate @count contiguous pages from the pool.
 * Returns physical address on success, 0 on failure. */
phys_addr_t enclave_pool_alloc(unsigned long count)
{
	unsigned long start;
	unsigned long flags;

	spin_lock_irqsave(&pool_lock, flags);
	start = bitmap_find_next_zero_area(pool_bitmap, POOL_PAGES, 0, count, count - 1);
	if (start + count > POOL_PAGES) {
		spin_unlock_irqrestore(&pool_lock, flags);
		pr_err("keystone: enclave pool out of memory (%lu pages)\n", count);
		return 0;
	}
	bitmap_set(pool_bitmap, start, count);
	spin_unlock_irqrestore(&pool_lock, flags);

	return (phys_addr_t)(ENCLAVE_POOL_PHYS + (phys_addr_t)start * PAGE_SIZE);
}

void enclave_pool_free(phys_addr_t pa, unsigned long count)
{
	unsigned long start = (unsigned long)((pa - ENCLAVE_POOL_PHYS) / PAGE_SIZE);
	unsigned long flags;

	spin_lock_irqsave(&pool_lock, flags);
	bitmap_clear(pool_bitmap, start, count);
	spin_unlock_irqrestore(&pool_lock, flags);
}

/* Translate a pool physical address to the kernel virtual address
 * obtained from memremap. */
void *enclave_pool_phys_to_virt(phys_addr_t pa)
{
	return pool_vbase + (unsigned long)(pa - ENCLAVE_POOL_PHYS);
}
