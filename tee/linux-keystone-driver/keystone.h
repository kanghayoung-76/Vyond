//******************************************************************************
// Copyright (c) 2018, The Regents of the University of California (Regents).
// All Rights Reserved. See LICENSE for license details.
//------------------------------------------------------------------------------
#ifndef _KEYSTONE_H_
#define _KEYSTONE_H_

#include <asm/sbi.h>
#include <asm/csr.h>
#include <linux/slab.h>
#include <linux/module.h>
#include <linux/uaccess.h>
#include <linux/init.h>
#include <linux/kernel.h>
#include <linux/fs.h>
#include <linux/miscdevice.h>
#include <linux/idr.h>

#include <linux/file.h>

/* IMPORTANT: This code assumes Sv39 */
#include "riscv64.h"

#define PAGE_UP(addr)	(((addr)+((PAGE_SIZE)-1))&(~((PAGE_SIZE)-1)))

typedef uintptr_t vaddr_t;
typedef uintptr_t paddr_t;
typedef uint32_t rid_t;

extern struct miscdevice keystone_dev;
extern struct list_head shm_list;

long keystone_ioctl(struct file* filep, unsigned int cmd, unsigned long arg);
int keystone_release(struct inode *inode, struct file *file);
int keystone_mmap(struct file *filp, struct vm_area_struct *vma);

/* enclave private memory */
struct epm {
  pte_t* root_page_table;
  vaddr_t ptr;
  size_t size;
  unsigned long order;
  paddr_t pa;
  bool is_cma;
  bool is_pool;
};

struct utm {
  pte_t* root_page_table;
  void* ptr;
  size_t size;
  unsigned long order;
};

struct shm
{
  struct list_head list;
  void *ptr;    // kernel address
  paddr_t pa;   // physical address
  uintptr_t va; // user virtual address
  size_t size;
  unsigned long order;
  int is_cma;
  int is_pool;
};

struct mem_mapping
{
  rid_t rid;
  uintptr_t va;
  uintptr_t pa;
  size_t size;
};

extern struct mem_mapping mem_mappings[];
extern int mem_mappings_n;

struct enclave
{
  unsigned long eid;
  int close_on_pexit;
  struct utm* utm;
  struct epm* epm;
  bool is_init;
};

extern struct enclave host_enclave;

struct enclave* get_enclave_by_id(unsigned int ueid);
struct enclave* create_enclave(unsigned long min_pages);
int destroy_enclave(struct enclave* enclave);

unsigned int enclave_idr_alloc(struct enclave* enclave);
struct enclave* enclave_idr_remove(unsigned int ueid);

int epm_destroy(struct epm* epm);
int epm_init(struct epm* epm, unsigned int count);
int utm_destroy(struct utm* utm);
int utm_init(struct utm* utm, size_t untrusted_size);

uintptr_t allocate_shm(struct enclave *enclave, uintptr_t size);
int destroy_shm_by_pa(uintptr_t pa);
int shm_destroy(struct shm *shm);
int shm_init(struct shm *shm, size_t shared_size);

/* Enclave pool allocator (pool region excluded from Linux via mem=) */
int enclave_pool_init(void);
void enclave_pool_exit(void);
phys_addr_t enclave_pool_alloc(unsigned long count);
void enclave_pool_free(phys_addr_t pa, unsigned long count);
void *enclave_pool_phys_to_virt(phys_addr_t pa);

#define keystone_info(fmt, ...) \
  pr_info("keystone_enclave: " fmt, ##__VA_ARGS__)
#define keystone_err(fmt, ...) \
  pr_err("keystone_enclave: " fmt, ##__VA_ARGS__)
#define keystone_warn(fmt, ...) \
  pr_warn("keystone_enclave: " fmt, ##__VA_ARGS__)
#endif
