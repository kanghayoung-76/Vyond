#include "loader/loader.h"
#include "mm/vm.h"
#include "mm/mm.h"
#include "mm/common.h"
#include "mm/freemem.h"
#include "util/printf.h"
#include <asm/csr.h>

/* root page table */
pte root_page_table_storage[BIT(RISCV_PT_INDEX_BITS)] __attribute__((aligned(RISCV_PAGE_SIZE)));
/* page tables for loading physical memory */
pte load_l2_page_table_storage[BIT(RISCV_PT_INDEX_BITS)] __attribute__((aligned(RISCV_PAGE_SIZE)));
pte load_l3_page_table_storage[BIT(RISCV_PT_INDEX_BITS)] __attribute__((aligned(RISCV_PAGE_SIZE)));

uintptr_t free_base_final = 0;

uintptr_t satp_new(uintptr_t pa)
{
  return (SATP_MODE | (pa >> RISCV_PAGE_BITS));
}

void map_physical_memory(uintptr_t dram_base, uintptr_t dram_size) {
  uintptr_t ptr = EYRIE_LOAD_START;
  /* load address should not override kernel address */
  assert(RISCV_GET_PT_INDEX(ptr, 1) != RISCV_GET_PT_INDEX(RUNTIME_VA_START, 1));
  map_with_reserved_page_table(dram_base, dram_size,
      ptr, load_l2_page_table_storage, load_l3_page_table_storage);
}

int map_untrusted_memory(uintptr_t untrusted_ptr, uintptr_t untrusted_size) {
  uintptr_t va        = EYRIE_UNTRUSTED_START;
  while (va < EYRIE_UNTRUSTED_START + untrusted_size) {
    if (!map_page(vpn(va), ppn(untrusted_ptr), PTE_W | PTE_R | PTE_D | PTE_U)) {
      return -1;
    }
    va += RISCV_PAGE_SIZE;
    untrusted_ptr += RISCV_PAGE_SIZE;
  }
  return 0;
}

int load_runtime(uintptr_t dummy,
                uintptr_t dram_base, uintptr_t dram_size, 
                uintptr_t runtime_base, uintptr_t user_base,
                uintptr_t free_base, uintptr_t untrusted_ptr,
                uintptr_t untrusted_size) {
  int ret = 0;

#ifdef USE_WG_FAULT_TEST
  /* WGChecker slot virtualization test:
   * Access SM memory (physical 0x80000000) which is restricted to WID 7 only.
   * At this point satp=0 (physical addressing) so this is a direct physical access.
   * The enclave's WID (set by mlwid CSR) will be denied by WGChecker,
   * producing CAUSE_LOAD_ACCESS_FAULT -> M-mode trap -> SM handler. */
  volatile uintptr_t *sm_mem = (volatile uintptr_t *)0x80000000UL;
  volatile uintptr_t wg_read = *sm_mem;
  (void)wg_read;
#endif

  /* Loader start probe — always visible if loader and sbi_putchar work */
  sbi_putchar('L'); sbi_putchar('D'); sbi_putchar(':');

  /* [TRACE] 로더가 실제로 받은 인자. SM의 entry 출력과 대조하면 mret 경계에서
   * 값이 유실되는지 바로 드러난다. */
  printf("[TRACE][LDR] dram_base=%p dram_size=%p\n", (void*)dram_base, (void*)dram_size);
  printf("[TRACE][LDR] runtime_base=%p user_base=%p free_base=%p\n",
         (void*)runtime_base, (void*)user_base, (void*)free_base);
  printf("[TRACE][LDR] untrusted_ptr=%p untrusted_size=%p\n",
         (void*)untrusted_ptr, (void*)untrusted_size);

  root_page_table = root_page_table_storage;

  // initialize freemem
  printf("[TRACE][LDR] spa_init(free_base=%p, size=%p)\n",
         (void*)free_base, (void*)(dram_base + dram_size - free_base));
  spa_init(free_base, dram_base + dram_size - free_base);
  printf("[TRACE][LDR] spa_init done\n");

  // validate runtime elf
  size_t runtime_size = user_base - runtime_base;
  printf("[TRACE][LDR] runtime_size=%p\n", (void*)runtime_size);
  if (((void*) runtime_base == NULL) || (runtime_size <= 0)) {
    printf("[TRACE][LDR] FAIL: runtime_base NULL or runtime_size<=0\n");
    return -1;
  }

  // create runtime elf struct
  elf_t runtime_elf;
  printf("[TRACE][LDR] elf_newFile(runtime_base=%p, size=%p)\n",
         (void*)runtime_base, (void*)runtime_size);
  ret = elf_newFile((void*) runtime_base, runtime_size, &runtime_elf);
  printf("[TRACE][LDR] elf_newFile -> %d\n", ret);
  if (ret != 0) {
    printf("[TRACE][LDR] FAIL: elf_newFile\n");
    return ret;
  }

  // map runtime memory
  printf("[TRACE][LDR] loadElf(runtime)\n");
  ret = loadElf(&runtime_elf, 0);
  printf("[TRACE][LDR] loadElf -> %d\n", ret);
  if (ret != 0) {
    printf("[TRACE][LDR] FAIL: loadElf\n");
    return ret;
  }

  // map enclave physical memory, so that runtime will be able to access all memory
  map_physical_memory(dram_base, dram_size);

  printf("[TRACE][LDR] map_physical_memory done\n");

  // map untrusted memory
  ret = map_untrusted_memory(untrusted_ptr, untrusted_size);
  printf("[TRACE][LDR] map_untrusted_memory -> %d\n", ret);
  if (ret != 0) {
    printf("[TRACE][LDR] FAIL: map_untrusted_memory\n");
    return ret;
  }

  free_base_final = dram_base + dram_size - spa_available() * RISCV_PAGE_SIZE;

  return ret;
}

void error_and_exit() {
  sbi_putchar('L'); sbi_putchar('F'); sbi_putchar('!');
  printf("[loader] FATAL: failed to load.\n");
  sbi_exit_enclave(-1);
}

