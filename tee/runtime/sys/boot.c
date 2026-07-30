#include <asm/csr.h>

#include "util/printf.h"
#include "sys/interrupt.h"
#include "call/syscall.h"
#include "mm/vm.h"
#include "util/string.h"
#include "call/sbi.h"
#include "mm/freemem.h"
#include "mm/mm.h"
#include "sys/env.h"
#include "mm/paging.h"
#include "loader/elf.h"
#include "loader/loader.h"

/* defined in vm.h */
extern uintptr_t shared_buffer;
extern uintptr_t shared_buffer_size;

/* initial memory layout */
uintptr_t utm_base;
size_t utm_size;

/* defined in entry.S */
extern void* encl_trap_handler;

int verify_and_load_elf_file(uintptr_t ptr, size_t file_size, bool is_eapp) {
  int ret = 0;
  // validate elf 
  if (((void*) ptr == NULL) || (file_size <= 0)) {
    return -1; 
  }
  
  // create elf struct
  elf_t elf_file;
  ret = elf_newFile((void*) ptr, file_size, &elf_file);
  if (ret < 0) {
    return ret;
  }

  // parse and load elf file
  ret = loadElf(&elf_file, 1);

  if (is_eapp) { // setup entry point
    uintptr_t entry = elf_getEntryPoint(&elf_file);
    csr_write(sepc, entry);
  }
  return ret;
}

/* initialize free memory with a simple page allocator*/
void
init_freemem()
{
  spa_init(freemem_va_start, freemem_size);
}

/* initialize user stack
 *
 * [2026-07-30] sscratch 쓰기는 반드시 여기(부팅 중)에 있어야 한다. eyrie_boot 맨 끝으로
 * 옮겨봤다가 완주율이 4/10 → 2/10 로 떨어지고 호스트 SIGSEGV 가 1→6 회로 늘어 되돌렸다.
 * 이유는 entry.S 에 있다: `_start` 가 sscratch=0 을 sentinel 로 심어 S-모드 트랩을 구분하는데
 * (`bnez sp, __save_context`), return_to_encl 은 복귀 시 `csrrw sp, sscratch, sp` 로 sscratch 에
 * 핸들러 프레임 주소를 남겨 sentinel 을 파괴한다. 부팅 구간에 트랩(타이머)이 실제로 자주
 * 들어오므로, sscratch 를 user_sp 로 미리 심어두면 그 트랩이 유저 스택 경로로 처리돼
 * 런타임 스택과 sentinel 을 건드리지 않는다. 대신 그 컨텍스트 저장이 PTE_U 페이지를
 * 향하므로 sstatus.SUM 이 그 시점에 이미 1이어야 한다 → eyrie_boot 에서 SUM 을 먼저 켠다. */
static void
init_user_stack_and_env(ELF(Ehdr) *hdr)
{
  void* user_sp = (void*) EYRIE_USER_STACK_START;
  size_t count;
  uintptr_t stack_end = EYRIE_USER_STACK_END;
  size_t stack_count = EYRIE_USER_STACK_SIZE >> RISCV_PAGE_BITS;

  // allocated stack pages right below the runtime
  count = alloc_pages(vpn(stack_end), stack_count,
      PTE_R | PTE_W | PTE_D | PTE_A | PTE_U);

  assert(count == stack_count);

  // setup user stack env/aux
  user_sp = setup_start(user_sp, hdr);

  // prepare user sp
  csr_write(sscratch, user_sp);
}

void
eyrie_boot(uintptr_t dummy, // $a0 contains the return value from the SBI
           uintptr_t dram_base,
           uintptr_t dram_size,
           uintptr_t runtime_paddr,
           uintptr_t user_paddr,
           uintptr_t free_paddr,
           uintptr_t utm_vaddr,
           uintptr_t utm_size)
{
  /* Unconditional boot probe — helps distinguish "eyrie_boot not reached" vs "printf broken" */
  sbi_putchar('Y'); sbi_putchar('B'); sbi_putchar(':');

  /* set initial values */
  load_pa_start = dram_base;
  root_page_table = (pte*) __va(csr_read(satp) << RISCV_PAGE_BITS);
  shared_buffer = EYRIE_UNTRUSTED_START;
  shared_buffer_size = utm_size;
  runtime_va_start = (uintptr_t) &rt_base;
  kernel_offset = runtime_va_start - runtime_paddr;

  debug("ROOT PAGE TABLE: 0x%lx", root_page_table);
  debug("UTM : 0x%lx-0x%lx (%u KB)", utm_vaddr, utm_vaddr+utm_size, utm_size/1024);
  debug("DRAM: 0x%lx-0x%lx (%u KB)", dram_base, dram_base + dram_size, dram_size/1024);
  debug("USER: 0x%lx-0x%lx (%u KB)", user_paddr, free_paddr, (free_paddr-user_paddr)/1024);

  /* set trap vector */
  csr_write(stvec, &encl_trap_handler);

  /* Enable the FPU (FS=dirty) and allow S-mode to access U-mode pages.
   * UTM is mapped with PTE_U for zero-copy eapp access; the runtime also
   * writes to UTM (edge_call header), so SUM must be set.
   *
   * [2026-07-30] 부팅 맨 끝(init_timer 뒤)에 있던 것을 트랩 벡터 설정 직후로 올렸다.
   * init_edge_internals 는 PTE_U 로 매핑된 UTM 에 쓰고, 트랩 핸들러도 PTE_U 인 유저
   * 스택을 건드릴 수 있는데 SUM=0 인 채로 그 구간을 지나면 S-모드 store 가 곧바로
   * page fault 가 된다. SUM 은 S-모드 권한을 넓히기만 하므로 먼저 켜도 안전하다. */
  csr_write(sstatus, csr_read(sstatus) | 0x6000 | 0x40000);

  freemem_va_start = __va(free_paddr);
  freemem_size = dram_base + dram_size - free_paddr;

  debug("FREE: 0x%lx-0x%lx (%u KB), va 0x%lx", free_paddr, dram_base + dram_size, freemem_size/1024, freemem_va_start);

  /* [TRACE] 부팅 단계 마커: memset(NULL) fault가 어느 단계에서 나는지 특정 */
  printf("[TRACE][RT] boot: init_freemem\n");
  init_freemem();
  printf("[TRACE][RT] boot: init_freemem done\n");

  /* load eapp elf */
  printf("[TRACE][RT] boot: load eapp elf (va=0x%lx size=0x%lx)\n",
         (unsigned long)__va(user_paddr), (unsigned long)(free_paddr-user_paddr));
  assert(!verify_and_load_elf_file(__va(user_paddr), free_paddr-user_paddr, true));
  printf("[TRACE][RT] boot: eapp elf loaded\n");

  /* free leaking memory */
  // TODO: clean up after loader -- entire file no longer needed
  // TODO: load elf file doesn't map some pages; those can be re-used. runtime and eapp.

  //TODO: This should be set by walking the userspace vm and finding
  //highest used addr. Instead we start partway through the anon space
  set_program_break(EYRIE_ANON_REGION_START + (1024 * 1024 * 1024));

  #ifdef USE_PAGING
  init_paging(user_paddr, free_paddr);
  #endif /* USE_PAGING */

  /* initialize user stack */
  printf("[TRACE][RT] boot: init_user_stack_and_env\n");
  init_user_stack_and_env((ELF(Ehdr) *) __va(user_paddr));
  printf("[TRACE][RT] boot: user stack ready\n");

  /* prepare edge & system calls */
  printf("[TRACE][RT] boot: init_edge_internals\n");
  init_edge_internals();
  printf("[TRACE][RT] boot: edge internals ready\n");

  /* set timer */
  init_timer();

  /* Enable U-mode access to the cycle/time/instret counters so the eapp can
   * use rdcycle/rdtime/rdinstret for self-timing. mcounteren is already all-1s
   * (set by OpenSBI), so this scounteren write is the only gate left; without
   * it a U-mode rdcycle traps as an illegal instruction. Bits: CY|TM|IR. */
  csr_write(scounteren, 0x7);

  debug("eyrie boot finished. drop to the user land ...");
  /* booting all finished, droping to the user land */
  return;
}
