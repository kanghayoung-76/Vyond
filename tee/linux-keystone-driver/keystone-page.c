//******************************************************************************
// Copyright (c) 2018, The Regents of the University of California (Regents).
// All Rights Reserved. See LICENSE for license details.
//------------------------------------------------------------------------------
#include "riscv64.h"
#include <linux/kernel.h>
#include "keystone.h"
#include "keystone-sbi.h"

/* Destroy all memory associated with an EPM */
int epm_destroy(struct epm *epm)
{

  if (!epm->ptr || !epm->size)
    return 0;

  if (epm->is_pool)
  {
    enclave_pool_free(epm->pa, epm->size >> PAGE_SHIFT);
  }
  else
  {
    free_pages(epm->ptr, epm->order);
  }

  return 0;
}

/* Create an EPM and initialize the free list */
int epm_init(struct epm *epm, unsigned int min_pages)
{
  unsigned long order = ilog2(min_pages - 1) + 1;
  unsigned long count = 0x1 << order;
  phys_addr_t pa;
  void *vaddr;

  /* Allocate from the enclave pool (physical above mem= boundary).
   * This ensures the SM's WGC pool-range check passes. */
  pa = enclave_pool_alloc(count);
  if (!pa)
  {
    keystone_err("failed to allocate %lu page(s) from enclave pool\n", count);
    return -ENOMEM;
  }

  /* Install OS_WID setup slot before zeroing pool pages so WGC allows S-mode stores. */
  {
    struct sbiret sret = sbi_sm_prepare_epm(pa, count << PAGE_SHIFT);
    if (sret.error) {
      keystone_err("sbi_sm_prepare_epm failed: %ld\n", sret.error);
      enclave_pool_free(pa, count);
      return -EINVAL;
    }
  }

  vaddr = enclave_pool_phys_to_virt(pa);
  memset(vaddr, 0, count << PAGE_SHIFT);

  epm->root_page_table = (pte_t *)vaddr;
  epm->pa    = pa;
  epm->order = order;
  epm->size  = count << PAGE_SHIFT;
  epm->ptr   = (vaddr_t)vaddr;
  epm->is_cma  = 0;
  epm->is_pool = 1;

  keystone_info("epm: pa=%#llx virt=%p (%lu pages) [pool]\n",
                (unsigned long long)pa, vaddr, count);
  return 0;
}

int utm_destroy(struct utm *utm)
{

  if (utm->ptr != NULL)
  {
    free_pages((vaddr_t)utm->ptr, utm->order);
  }

  return 0;
}

int utm_init(struct utm *utm, size_t untrusted_size)
{
  unsigned long req_pages = 0;
  unsigned long order = 0;
  unsigned long count;
  req_pages += PAGE_UP(untrusted_size) / PAGE_SIZE;
  order = ilog2(req_pages - 1) + 1;
  count = 0x1 << order;

  utm->order = order;

  /* Currently, UTM does not utilize CMA.
   * It is always allocated from the buddy allocator */
  utm->ptr = (void *)__get_free_pages(GFP_HIGHUSER, order);
  if (!utm->ptr)
  {
    keystone_err("failed to allocate UTM (size = %i bytes)\n", (1 << order));
    return -ENOMEM;
  }

  utm->size = count * PAGE_SIZE;
  if (utm->size != untrusted_size)
  {
    /* Instead of failing, we just warn that the user has to fix the parameter. */
    keystone_warn("shared buffer size is not multiple of PAGE_SIZE\n");
  }

  return 0;
}

int shm_init(struct shm *shm, size_t shared_size)
{
  unsigned long count = PAGE_UP(shared_size) / PAGE_SIZE;
  phys_addr_t pa;
  void *vaddr;

  INIT_LIST_HEAD(&shm->list);

  /* All SHM (enclave-to-enclave and enclave SHM) comes from the pool so
   * the OS WGC slot cannot reach it.  Only UTM stays in OS memory. */
  pa = enclave_pool_alloc(count);
  if (!pa)
  {
    keystone_err("failed to allocate %lu page(s) from enclave pool\n", count);
    return -ENOMEM;
  }

  vaddr = enclave_pool_phys_to_virt(pa);

  shm->ptr     = vaddr;
  shm->pa      = pa;
  shm->size    = count * PAGE_SIZE;
  shm->is_cma  = 0;
  shm->is_pool = 1;

  return 0;
}

int shm_destroy(struct shm *shm)
{
  if (!shm->ptr || !shm->size)
    return 0;

  if (shm->is_pool)
  {
    enclave_pool_free(shm->pa, shm->size >> PAGE_SHIFT);
  }
  else
  {
    free_pages((uintptr_t)shm->ptr, shm->order);
  }
  return 0;
}
