savedcmd_keystone-sbi.o := riscv64-linux-gnu-gcc -Wp,-MMD,./.keystone-sbi.o.d -nostdinc -I/data/hykang/RVSS/q-vela/linux/arch/riscv/include -I/data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated -I/data/hykang/RVSS/q-vela/linux/include -I/data/hykang/RVSS/q-vela/linux/include -I/data/hykang/RVSS/q-vela/linux/arch/riscv/include/uapi -I/data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/uapi -I/data/hykang/RVSS/q-vela/linux/include/uapi -I/data/hykang/RVSS/q-vela/linux/include/generated/uapi -include /data/hykang/RVSS/q-vela/linux/include/linux/compiler-version.h -include /data/hykang/RVSS/q-vela/linux/include/linux/kconfig.h -include /data/hykang/RVSS/q-vela/linux/include/linux/compiler_types.h -D__KERNEL__ -std=gnu11 -fshort-wchar -funsigned-char -fno-common -fno-PIE -fno-strict-aliasing -fPIE -mabi=lp64 -march=rv64imac_zicsr_zifencei -mno-save-restore -mcmodel=medany -fno-asynchronous-unwind-tables -fno-unwind-tables -mno-riscv-attribute -Wa,-mno-arch-attr -mstrict-align -fno-delete-null-pointer-checks -O2 -fno-allow-store-data-races -fstack-protector-strong -fno-omit-frame-pointer -fno-optimize-sibling-calls -fno-stack-clash-protection -fzero-call-used-regs=used-gpr -fstrict-flex-arrays=3 -fno-strict-overflow -fno-stack-check -fconserve-stack -fno-builtin-wcslen -Wall -Wextra -Wundef -Werror=implicit-function-declaration -Werror=implicit-int -Werror=return-type -Werror=strict-prototypes -Wno-format-security -Wno-trigraphs -Wno-frame-address -Wno-address-of-packed-member -Wmissing-declarations -Wmissing-prototypes -Wframe-larger-than=2048 -Wno-main -Wno-dangling-pointer -Wvla-larger-than=1 -Wno-pointer-sign -Wcast-function-type -Wno-array-bounds -Wno-stringop-overflow -Wno-alloc-size-larger-than -Wimplicit-fallthrough=5 -Werror=date-time -Werror=incompatible-pointer-types -Werror=designated-init -Wenum-conversion -Wunused -Wno-unused-but-set-variable -Wno-unused-const-variable -Wno-packed-not-aligned -Wno-format-overflow -Wno-format-truncation -Wno-stringop-truncation -Wno-override-init -Wno-missing-field-initializers -Wno-type-limits -Wno-shift-negative-value -Wno-maybe-uninitialized -Wno-sign-compare -Wno-unused-parameter -mstack-protector-guard=tls -mstack-protector-guard-reg=tp -mstack-protector-guard-offset=1568 -I/data/hykang/RVSS/WGTEE_project/Vyond/tee/sdk/install/include/shared  -DMODULE -mno-relax  -DKBUILD_BASENAME='"keystone_sbi"' -DKBUILD_MODNAME='"keystone_driver"' -D__KBUILD_MODNAME=kmod_keystone_driver -c -o keystone-sbi.o keystone-sbi.c  

source_keystone-sbi.o := keystone-sbi.c

deps_keystone-sbi.o := \
  /data/hykang/RVSS/q-vela/linux/include/linux/compiler-version.h \
    $(wildcard include/config/CC_VERSION_TEXT) \
  /data/hykang/RVSS/q-vela/linux/include/linux/kconfig.h \
    $(wildcard include/config/CPU_BIG_ENDIAN) \
    $(wildcard include/config/BOOGER) \
    $(wildcard include/config/FOO) \
  /data/hykang/RVSS/q-vela/linux/include/linux/compiler_types.h \
    $(wildcard include/config/DEBUG_INFO_BTF) \
    $(wildcard include/config/PAHOLE_HAS_BTF_TAG) \
    $(wildcard include/config/FUNCTION_ALIGNMENT) \
    $(wildcard include/config/CC_HAS_SANE_FUNCTION_ALIGNMENT) \
    $(wildcard include/config/X86_64) \
    $(wildcard include/config/ARM64) \
    $(wildcard include/config/LD_DEAD_CODE_DATA_ELIMINATION) \
    $(wildcard include/config/LTO_CLANG) \
    $(wildcard include/config/HAVE_ARCH_COMPILER_H) \
    $(wildcard include/config/CC_HAS_ASSUME) \
    $(wildcard include/config/CC_HAS_COUNTED_BY) \
    $(wildcard include/config/CC_HAS_MULTIDIMENSIONAL_NONSTRING) \
    $(wildcard include/config/UBSAN_INTEGER_WRAP) \
    $(wildcard include/config/CFI) \
    $(wildcard include/config/ARCH_USES_CFI_GENERIC_LLVM_PASS) \
    $(wildcard include/config/CC_HAS_ASM_INLINE) \
  /data/hykang/RVSS/q-vela/linux/include/linux/compiler_attributes.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/compiler-gcc.h \
    $(wildcard include/config/ARCH_USE_BUILTIN_BSWAP) \
    $(wildcard include/config/SHADOW_CALL_STACK) \
    $(wildcard include/config/KCOV) \
    $(wildcard include/config/CC_HAS_TYPEOF_UNQUAL) \
  keystone-sbi.h \
  /data/hykang/RVSS/WGTEE_project/Vyond/tee/sdk/install/include/shared/keystone_user.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/ioctl.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/uapi/asm/ioctl.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/ioctl.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/asm-generic/ioctl.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/types.h \
    $(wildcard include/config/HAVE_UID16) \
    $(wildcard include/config/UID16) \
    $(wildcard include/config/ARCH_DMA_ADDR_T_64BIT) \
    $(wildcard include/config/PHYS_ADDR_T_64BIT) \
    $(wildcard include/config/64BIT) \
    $(wildcard include/config/ARCH_32BIT_USTAT_F_TINODE) \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/types.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/uapi/asm/types.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/asm-generic/types.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/int-ll64.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/asm-generic/int-ll64.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/uapi/asm/bitsperlong.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/bitsperlong.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/asm-generic/bitsperlong.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/posix_types.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/stddef.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/stddef.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/uapi/asm/posix_types.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/asm-generic/posix_types.h \
  /data/hykang/RVSS/WGTEE_project/Vyond/tee/sdk/install/include/shared/sm_call.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/sbi.h \
    $(wildcard include/config/RISCV_SBI) \
    $(wildcard include/config/RISCV_SBI_V01) \
    $(wildcard include/config/SMP) \
  /data/hykang/RVSS/q-vela/linux/include/linux/cpumask.h \
    $(wildcard include/config/FORCE_NR_CPUS) \
    $(wildcard include/config/HOTPLUG_CPU) \
    $(wildcard include/config/DEBUG_PER_CPU_MAPS) \
    $(wildcard include/config/CPUMASK_OFFSTACK) \
  /data/hykang/RVSS/q-vela/linux/include/linux/cleanup.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/compiler.h \
    $(wildcard include/config/TRACE_BRANCH_PROFILING) \
    $(wildcard include/config/PROFILE_ALL_BRANCHES) \
    $(wildcard include/config/OBJTOOL) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/asm/rwonce.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/rwonce.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/kasan-checks.h \
    $(wildcard include/config/KASAN_GENERIC) \
    $(wildcard include/config/KASAN_SW_TAGS) \
  /data/hykang/RVSS/q-vela/linux/include/linux/kcsan-checks.h \
    $(wildcard include/config/KCSAN) \
    $(wildcard include/config/KCSAN_WEAK_MEMORY) \
    $(wildcard include/config/KCSAN_IGNORE_ATOMICS) \
  /data/hykang/RVSS/q-vela/linux/include/linux/err.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/uapi/asm/errno.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/asm-generic/errno.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/asm-generic/errno-base.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/args.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/kernel.h \
    $(wildcard include/config/PREEMPT_VOLUNTARY_BUILD) \
    $(wildcard include/config/PREEMPT_DYNAMIC) \
    $(wildcard include/config/HAVE_PREEMPT_DYNAMIC_CALL) \
    $(wildcard include/config/HAVE_PREEMPT_DYNAMIC_KEY) \
    $(wildcard include/config/PREEMPT_) \
    $(wildcard include/config/DEBUG_ATOMIC_SLEEP) \
    $(wildcard include/config/MMU) \
    $(wildcard include/config/PROVE_LOCKING) \
    $(wildcard include/config/TRACING) \
    $(wildcard include/config/DYNAMIC_FTRACE) \
  /data/hykang/RVSS/q-vela/linux/include/linux/stdarg.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/align.h \
  /data/hykang/RVSS/q-vela/linux/include/vdso/align.h \
  /data/hykang/RVSS/q-vela/linux/include/vdso/const.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/const.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/array_size.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/limits.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/limits.h \
  /data/hykang/RVSS/q-vela/linux/include/vdso/limits.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/linkage.h \
    $(wildcard include/config/ARCH_USE_SYM_ANNOTATIONS) \
  /data/hykang/RVSS/q-vela/linux/include/linux/stringify.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/export.h \
    $(wildcard include/config/MODVERSIONS) \
    $(wildcard include/config/GENDWARFKSYMS) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/linkage.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/container_of.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/build_bug.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/bitops.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/bits.h \
  /data/hykang/RVSS/q-vela/linux/include/vdso/bits.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/bits.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/overflow.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/const.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/typecheck.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/kernel.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/sysinfo.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/bitops/generic-non-atomic.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/barrier.h \
    $(wildcard include/config/RISCV_ISA_ZAWRS) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/cmpxchg.h \
    $(wildcard include/config/RISCV_ISA_ZABHA) \
    $(wildcard include/config/RISCV_ISA_ZACAS) \
    $(wildcard include/config/TOOLCHAIN_HAS_ZACAS) \
  /data/hykang/RVSS/q-vela/linux/include/linux/bug.h \
    $(wildcard include/config/GENERIC_BUG) \
    $(wildcard include/config/PRINTK) \
    $(wildcard include/config/BUG_ON_DATA_CORRUPTION) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/bug.h \
    $(wildcard include/config/GENERIC_BUG_RELATIVE_POINTERS) \
    $(wildcard include/config/DEBUG_BUGVERBOSE) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/asm.h \
    $(wildcard include/config/AS_HAS_INSN) \
    $(wildcard include/config/KPROBES) \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/bug.h \
    $(wildcard include/config/BUG) \
  /data/hykang/RVSS/q-vela/linux/include/linux/instrumentation.h \
    $(wildcard include/config/NOINSTR_VALIDATION) \
  /data/hykang/RVSS/q-vela/linux/include/linux/once_lite.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/panic.h \
    $(wildcard include/config/PANIC_TIMEOUT) \
  /data/hykang/RVSS/q-vela/linux/include/linux/printk.h \
    $(wildcard include/config/MESSAGE_LOGLEVEL_DEFAULT) \
    $(wildcard include/config/CONSOLE_LOGLEVEL_DEFAULT) \
    $(wildcard include/config/CONSOLE_LOGLEVEL_QUIET) \
    $(wildcard include/config/EARLY_PRINTK) \
    $(wildcard include/config/PRINTK_INDEX) \
    $(wildcard include/config/DYNAMIC_DEBUG) \
    $(wildcard include/config/DYNAMIC_DEBUG_CORE) \
  /data/hykang/RVSS/q-vela/linux/include/linux/init.h \
    $(wildcard include/config/MEMORY_HOTPLUG) \
    $(wildcard include/config/HAVE_ARCH_PREL32_RELOCATIONS) \
  /data/hykang/RVSS/q-vela/linux/include/linux/kern_levels.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/ratelimit_types.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/param.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/uapi/asm/param.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/param.h \
    $(wildcard include/config/HZ) \
  /data/hykang/RVSS/q-vela/linux/include/uapi/asm-generic/param.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/spinlock_types_raw.h \
    $(wildcard include/config/DEBUG_SPINLOCK) \
    $(wildcard include/config/DEBUG_LOCK_ALLOC) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/asm/spinlock_types.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/spinlock_types.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/qspinlock_types.h \
    $(wildcard include/config/NR_CPUS) \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/qrwlock_types.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/uapi/asm/byteorder.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/byteorder/little_endian.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/byteorder/little_endian.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/swab.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/swab.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/swab.h \
    $(wildcard include/config/TOOLCHAIN_HAS_ZBB) \
    $(wildcard include/config/RISCV_ISA_ZBB) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/cpufeature-macros.h \
    $(wildcard include/config/RISCV_ALTERNATIVE) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/hwcap.h \
    $(wildcard include/config/RISCV_M_MODE) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/uapi/asm/hwcap.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/alternative-macros.h \
    $(wildcard include/config/k) \
    $(wildcard include/config/k_1) \
    $(wildcard include/config/k_2) \
  /data/hykang/RVSS/q-vela/linux/include/uapi/asm-generic/swab.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/byteorder/generic.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/lockdep_types.h \
    $(wildcard include/config/PROVE_RAW_LOCK_NESTING) \
    $(wildcard include/config/LOCKDEP) \
    $(wildcard include/config/LOCK_STAT) \
  /data/hykang/RVSS/q-vela/linux/include/linux/dynamic_debug.h \
    $(wildcard include/config/JUMP_LABEL) \
  /data/hykang/RVSS/q-vela/linux/include/linux/jump_label.h \
    $(wildcard include/config/HAVE_ARCH_JUMP_LABEL_RELATIVE) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/jump_label.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/fence.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/insn-def.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/processor.h \
    $(wildcard include/config/RISCV_ISA_ZICBOP) \
    $(wildcard include/config/RISCV_ISA_V) \
    $(wildcard include/config/RISCV_ISA_SUPM) \
  /data/hykang/RVSS/q-vela/linux/include/linux/cache.h \
    $(wildcard include/config/ARCH_HAS_CACHE_LINE_SIZE) \
  /data/hykang/RVSS/q-vela/linux/include/vdso/cache.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/cache.h \
    $(wildcard include/config/RISCV_DMA_NONCOHERENT) \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/prctl.h \
  /data/hykang/RVSS/q-vela/linux/include/vdso/processor.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/vdso/processor.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/errata_list.h \
    $(wildcard include/config/ERRATA_SIFIVE_CIP_453) \
    $(wildcard include/config/ERRATA_SIFIVE_CIP_1200) \
    $(wildcard include/config/ERRATA_MIPS_P8700_PAUSE_OPCODE) \
    $(wildcard include/config/RISCV_ISA_SVPBMT) \
    $(wildcard include/config/ERRATA_THEAD_MAE) \
    $(wildcard include/config/RISCV_ISA_ZICBOM) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/csr.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/vendorid_list.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/errata_list_vendors.h \
    $(wildcard include/config/ERRATA_ANDES) \
    $(wildcard include/config/ERRATA_SIFIVE) \
    $(wildcard include/config/ERRATA_THEAD) \
    $(wildcard include/config/ERRATA_MIPS) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/vendor_extensions/mips.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/ptrace.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/uapi/asm/ptrace.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/barrier.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/bitops.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/irqflags.h \
    $(wildcard include/config/TRACE_IRQFLAGS) \
    $(wildcard include/config/PREEMPT_RT) \
    $(wildcard include/config/IRQSOFF_TRACER) \
    $(wildcard include/config/PREEMPT_TRACER) \
    $(wildcard include/config/DEBUG_IRQFLAGS) \
    $(wildcard include/config/TRACE_IRQFLAGS_SUPPORT) \
  /data/hykang/RVSS/q-vela/linux/include/linux/irqflags_types.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/irqflags.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/asm/percpu.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/percpu.h \
    $(wildcard include/config/DEBUG_PREEMPT) \
    $(wildcard include/config/HAVE_SETUP_PER_CPU_AREA) \
  /data/hykang/RVSS/q-vela/linux/include/linux/threads.h \
    $(wildcard include/config/BASE_SMALL) \
  /data/hykang/RVSS/q-vela/linux/include/linux/percpu-defs.h \
    $(wildcard include/config/ARCH_MODULE_NEEDS_WEAK_PER_CPU) \
    $(wildcard include/config/DEBUG_FORCE_WEAK_PER_CPU) \
    $(wildcard include/config/AMD_MEM_ENCRYPT) \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/bitops/__ffs.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/bitops/__fls.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/bitops/ffs.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/bitops/fls.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/bitops/ffz.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/bitops/fls64.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/bitops/sched.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/arch_hweight.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/bitops/const_hweight.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/bitops/instrumented-atomic.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/instrumented.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/kmsan-checks.h \
    $(wildcard include/config/KMSAN) \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/bitops/instrumented-lock.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/bitops/non-atomic.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/bitops/non-instrumented-non-atomic.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/bitops/le.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/bitops/ext2-atomic.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/hex.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/kstrtox.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/log2.h \
    $(wildcard include/config/ARCH_HAS_ILOG2_U32) \
    $(wildcard include/config/ARCH_HAS_ILOG2_U64) \
  /data/hykang/RVSS/q-vela/linux/include/linux/math.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/asm/div64.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/div64.h \
    $(wildcard include/config/CC_OPTIMIZE_FOR_PERFORMANCE) \
  /data/hykang/RVSS/q-vela/linux/include/linux/minmax.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/sprintf.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/static_call_types.h \
    $(wildcard include/config/HAVE_STATIC_CALL) \
    $(wildcard include/config/HAVE_STATIC_CALL_INLINE) \
  /data/hykang/RVSS/q-vela/linux/include/linux/instruction_pointer.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/util_macros.h \
    $(wildcard include/config/FOO_SUSPEND) \
  /data/hykang/RVSS/q-vela/linux/include/linux/wordpart.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/bitmap.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/errno.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/errno.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/find.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/string.h \
    $(wildcard include/config/BINARY_PRINTF) \
    $(wildcard include/config/FORTIFY_SOURCE) \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/string.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/string.h \
    $(wildcard include/config/KASAN) \
  /data/hykang/RVSS/q-vela/linux/include/linux/bitmap-str.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/cpumask_types.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/atomic.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/atomic.h \
    $(wildcard include/config/GENERIC_ATOMIC64) \
  /data/hykang/RVSS/q-vela/linux/include/linux/atomic/atomic-arch-fallback.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/atomic/atomic-long.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/atomic/atomic-instrumented.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/gfp_types.h \
    $(wildcard include/config/KASAN_HW_TAGS) \
    $(wildcard include/config/SLAB_OBJ_EXT) \
  /data/hykang/RVSS/q-vela/linux/include/linux/numa.h \
    $(wildcard include/config/NUMA_KEEP_MEMINFO) \
    $(wildcard include/config/NUMA) \
    $(wildcard include/config/HAVE_ARCH_NODE_DEV_GROUP) \
  /data/hykang/RVSS/q-vela/linux/include/linux/nodemask.h \
    $(wildcard include/config/HIGHMEM) \
  /data/hykang/RVSS/q-vela/linux/include/linux/nodemask_types.h \
    $(wildcard include/config/NODES_SHIFT) \
  /data/hykang/RVSS/q-vela/linux/include/linux/random.h \
    $(wildcard include/config/VMGENID) \
  /data/hykang/RVSS/q-vela/linux/include/linux/list.h \
    $(wildcard include/config/LIST_HARDENED) \
    $(wildcard include/config/DEBUG_LIST) \
  /data/hykang/RVSS/q-vela/linux/include/linux/poison.h \
    $(wildcard include/config/ILLEGAL_POINTER_VALUE) \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/random.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/irqnr.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/irqnr.h \

keystone-sbi.o: $(deps_keystone-sbi.o)

$(deps_keystone-sbi.o):
