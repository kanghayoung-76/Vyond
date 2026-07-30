//******************************************************************************
// Copyright (c) 2018, The Regents of the University of California (Regents).
// All Rights Reserved. See LICENSE for license details.
//------------------------------------------------------------------------------
#include "mm/mm.h"
#include "util/rt_util.h"
#include "util/printf.h"
#include "uaccess.h"
#include "mm/vm.h"
#include "mm/vm_defs.h"
#include "mm/freemem.h"

// Statically allocated copy-buffer
unsigned char rt_copy_buffer_1[RISCV_PAGE_SIZE];
unsigned char rt_copy_buffer_2[RISCV_PAGE_SIZE];

size_t rt_util_getrandom(void* vaddr, size_t buflen){
  size_t remaining = buflen;
  uintptr_t rnd;
  uintptr_t* next = (uintptr_t*)vaddr;
  // Get data
  while(remaining > sizeof(uintptr_t)){
    rnd = sbi_random();
    ALLOW_USER_ACCESS( *next = rnd );
    remaining -= sizeof(uintptr_t);
    next++;
  }
  // Cleanup
  if( remaining > 0 ){
    rnd = sbi_random();
    copy_to_user(next, &rnd, remaining);
  }
  size_t ret = buflen;
  return ret;
}

void rt_util_misc_fatal(){
  //Better hope we can debug it!
  sbi_exit_enclave(-1);
}

void not_implemented_fatal(struct encl_ctx* ctx){
#ifdef FATAL_DEBUG
    unsigned long addr, cause, pc;
    pc = ctx->regs.sepc;
    addr = ctx->sbadaddr;
    cause = ctx->scause;
    printf("[runtime] non-handlable interrupt/exception at 0x%lx on 0x%lx (scause: 0x%lx)\r\n", pc, addr, cause);
#endif

    // Bail to m-mode
    __asm__ volatile("csrr a0, scause\r\nli a7, 1111\r\n ecall");

    return;
}

void rt_page_fault(struct encl_ctx* ctx)
{
#ifdef FATAL_DEBUG
  unsigned long addr, cause, pc;
  pc = ctx->regs.sepc;
  addr = ctx->sbadaddr;
  cause = ctx->scause;
  printf("[runtime] page fault at 0x%lx on 0x%lx (scause: 0x%lx)\r\n", pc, addr, cause);
  /* [2026-07-30 진단] 폴트 주소의 PTE 를 같이 찍는다. 간헐적 BSS store fault 가
   *   (a) PTE 자체가 무효/오염 (소프트웨어 또는 WID-태그 캐시에서 dirty PTE 유실) 인지
   *   (b) PTE 는 정상인데 TLB/하드웨어 문제인지
   * 를 이 한 줄로 가른다. pte=0 → 매핑 유실, pte 에 V|R|W|U 가 다 서 있으면 (b). */
  {
    pte* p = pte_of_va(addr);
    printf("[runtime]   pte@0x%lx = 0x%lx  (V=%d R=%d W=%d U=%d)  spa_avail=%u\r\n",
           (unsigned long)p, p ? (unsigned long)*p : 0UL,
           p ? (int)((*p & PTE_V) != 0) : 0, p ? (int)((*p & PTE_R) != 0) : 0,
           p ? (int)((*p & PTE_W) != 0) : 0, p ? (int)((*p & PTE_U) != 0) : 0,
           spa_available());
  }
#endif

  sbi_exit_enclave(-1);

  /* never reach here */
  assert(false);
  return;
}

void tlb_flush(void)
{
  __asm__ volatile("fence.i\t\nsfence.vma\t\n");
}
