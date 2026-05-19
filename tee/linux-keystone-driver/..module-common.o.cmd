savedcmd_.module-common.o := riscv64-linux-gnu-gcc -Wp,-MMD,./..module-common.o.d -nostdinc -I/data/hykang/RVSS/q-vela/linux/arch/riscv/include -I/data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated -I/data/hykang/RVSS/q-vela/linux/include -I/data/hykang/RVSS/q-vela/linux/include -I/data/hykang/RVSS/q-vela/linux/arch/riscv/include/uapi -I/data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/uapi -I/data/hykang/RVSS/q-vela/linux/include/uapi -I/data/hykang/RVSS/q-vela/linux/include/generated/uapi -include /data/hykang/RVSS/q-vela/linux/include/linux/compiler-version.h -include /data/hykang/RVSS/q-vela/linux/include/linux/kconfig.h -include /data/hykang/RVSS/q-vela/linux/include/linux/compiler_types.h -D__KERNEL__ -std=gnu11 -fshort-wchar -funsigned-char -fno-common -fno-PIE -fno-strict-aliasing -fPIE -mabi=lp64 -march=rv64imac_zicsr_zifencei -mno-save-restore -mcmodel=medany -fno-asynchronous-unwind-tables -fno-unwind-tables -mno-riscv-attribute -Wa,-mno-arch-attr -mstrict-align -fno-delete-null-pointer-checks -O2 -fno-allow-store-data-races -fstack-protector-strong -fno-omit-frame-pointer -fno-optimize-sibling-calls -fno-stack-clash-protection -fzero-call-used-regs=used-gpr -fstrict-flex-arrays=3 -fno-strict-overflow -fno-stack-check -fconserve-stack -fno-builtin-wcslen -Wall -Wextra -Wundef -Werror=implicit-function-declaration -Werror=implicit-int -Werror=return-type -Werror=strict-prototypes -Wno-format-security -Wno-trigraphs -Wno-frame-address -Wno-address-of-packed-member -Wmissing-declarations -Wmissing-prototypes -Wframe-larger-than=2048 -Wno-main -Wno-dangling-pointer -Wvla-larger-than=1 -Wno-pointer-sign -Wcast-function-type -Wno-array-bounds -Wno-stringop-overflow -Wno-alloc-size-larger-than -Wimplicit-fallthrough=5 -Werror=date-time -Werror=incompatible-pointer-types -Werror=designated-init -Wenum-conversion -Wunused -Wno-unused-but-set-variable -Wno-unused-const-variable -Wno-packed-not-aligned -Wno-format-overflow -Wno-format-truncation -Wno-stringop-truncation -Wno-override-init -Wno-missing-field-initializers -Wno-type-limits -Wno-shift-negative-value -Wno-maybe-uninitialized -Wno-sign-compare -Wno-unused-parameter -mstack-protector-guard=tls -mstack-protector-guard-reg=tp -mstack-protector-guard-offset=1568  -DMODULE -mno-relax  -DKBUILD_BASENAME='".module_common"' -DKBUILD_MODNAME='".module_common.o"' -D__KBUILD_MODNAME=kmod_.module_common.o -c -o .module-common.o /data/hykang/RVSS/q-vela/linux/scripts/module-common.c  

source_.module-common.o := /data/hykang/RVSS/q-vela/linux/scripts/module-common.c

deps_.module-common.o := \
    $(wildcard include/config/UNWINDER_ORC) \
    $(wildcard include/config/MITIGATION_RETPOLINE) \
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
  /data/hykang/RVSS/q-vela/linux/include/linux/module.h \
    $(wildcard include/config/MODULES) \
    $(wildcard include/config/SYSFS) \
    $(wildcard include/config/MODULES_TREE_LOOKUP) \
    $(wildcard include/config/LIVEPATCH) \
    $(wildcard include/config/STACKTRACE_BUILD_ID) \
    $(wildcard include/config/ARCH_USES_CFI_TRAPS) \
    $(wildcard include/config/MODULE_SIG) \
    $(wildcard include/config/GENERIC_BUG) \
    $(wildcard include/config/KALLSYMS) \
    $(wildcard include/config/SMP) \
    $(wildcard include/config/TRACEPOINTS) \
    $(wildcard include/config/TREE_SRCU) \
    $(wildcard include/config/BPF_EVENTS) \
    $(wildcard include/config/DEBUG_INFO_BTF_MODULES) \
    $(wildcard include/config/JUMP_LABEL) \
    $(wildcard include/config/TRACING) \
    $(wildcard include/config/EVENT_TRACING) \
    $(wildcard include/config/DYNAMIC_FTRACE) \
    $(wildcard include/config/KPROBES) \
    $(wildcard include/config/HAVE_STATIC_CALL_INLINE) \
    $(wildcard include/config/KUNIT) \
    $(wildcard include/config/PRINTK_INDEX) \
    $(wildcard include/config/MODULE_UNLOAD) \
    $(wildcard include/config/CONSTRUCTORS) \
    $(wildcard include/config/FUNCTION_ERROR_INJECTION) \
    $(wildcard include/config/DYNAMIC_DEBUG_CORE) \
  /data/hykang/RVSS/q-vela/linux/include/linux/list.h \
    $(wildcard include/config/LIST_HARDENED) \
    $(wildcard include/config/DEBUG_LIST) \
  /data/hykang/RVSS/q-vela/linux/include/linux/container_of.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/build_bug.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/compiler.h \
    $(wildcard include/config/TRACE_BRANCH_PROFILING) \
    $(wildcard include/config/PROFILE_ALL_BRANCHES) \
    $(wildcard include/config/OBJTOOL) \
    $(wildcard include/config/64BIT) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/asm/rwonce.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/rwonce.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/kasan-checks.h \
    $(wildcard include/config/KASAN_GENERIC) \
    $(wildcard include/config/KASAN_SW_TAGS) \
  /data/hykang/RVSS/q-vela/linux/include/linux/types.h \
    $(wildcard include/config/HAVE_UID16) \
    $(wildcard include/config/UID16) \
    $(wildcard include/config/ARCH_DMA_ADDR_T_64BIT) \
    $(wildcard include/config/PHYS_ADDR_T_64BIT) \
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
  /data/hykang/RVSS/q-vela/linux/include/linux/kcsan-checks.h \
    $(wildcard include/config/KCSAN) \
    $(wildcard include/config/KCSAN_WEAK_MEMORY) \
    $(wildcard include/config/KCSAN_IGNORE_ATOMICS) \
  /data/hykang/RVSS/q-vela/linux/include/linux/poison.h \
    $(wildcard include/config/ILLEGAL_POINTER_VALUE) \
  /data/hykang/RVSS/q-vela/linux/include/linux/const.h \
  /data/hykang/RVSS/q-vela/linux/include/vdso/const.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/const.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/barrier.h \
    $(wildcard include/config/RISCV_ISA_ZAWRS) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/cmpxchg.h \
    $(wildcard include/config/RISCV_ISA_ZABHA) \
    $(wildcard include/config/RISCV_ISA_ZACAS) \
    $(wildcard include/config/TOOLCHAIN_HAS_ZACAS) \
  /data/hykang/RVSS/q-vela/linux/include/linux/bug.h \
    $(wildcard include/config/PRINTK) \
    $(wildcard include/config/BUG_ON_DATA_CORRUPTION) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/bug.h \
    $(wildcard include/config/GENERIC_BUG_RELATIVE_POINTERS) \
    $(wildcard include/config/DEBUG_BUGVERBOSE) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/asm.h \
    $(wildcard include/config/AS_HAS_INSN) \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/bug.h \
    $(wildcard include/config/BUG) \
  /data/hykang/RVSS/q-vela/linux/include/linux/instrumentation.h \
    $(wildcard include/config/NOINSTR_VALIDATION) \
  /data/hykang/RVSS/q-vela/linux/include/linux/once_lite.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/panic.h \
    $(wildcard include/config/PANIC_TIMEOUT) \
  /data/hykang/RVSS/q-vela/linux/include/linux/stdarg.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/printk.h \
    $(wildcard include/config/MESSAGE_LOGLEVEL_DEFAULT) \
    $(wildcard include/config/CONSOLE_LOGLEVEL_DEFAULT) \
    $(wildcard include/config/CONSOLE_LOGLEVEL_QUIET) \
    $(wildcard include/config/EARLY_PRINTK) \
    $(wildcard include/config/DYNAMIC_DEBUG) \
  /data/hykang/RVSS/q-vela/linux/include/linux/init.h \
    $(wildcard include/config/MEMORY_HOTPLUG) \
    $(wildcard include/config/HAVE_ARCH_PREL32_RELOCATIONS) \
  /data/hykang/RVSS/q-vela/linux/include/linux/stringify.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/kern_levels.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/linkage.h \
    $(wildcard include/config/ARCH_USE_SYM_ANNOTATIONS) \
  /data/hykang/RVSS/q-vela/linux/include/linux/export.h \
    $(wildcard include/config/MODVERSIONS) \
    $(wildcard include/config/GENDWARFKSYMS) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/linkage.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/ratelimit_types.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/bits.h \
  /data/hykang/RVSS/q-vela/linux/include/vdso/bits.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/bits.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/overflow.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/limits.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/limits.h \
  /data/hykang/RVSS/q-vela/linux/include/vdso/limits.h \
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
  /data/hykang/RVSS/q-vela/linux/include/linux/jump_label.h \
    $(wildcard include/config/HAVE_ARCH_JUMP_LABEL_RELATIVE) \
  /data/hykang/RVSS/q-vela/linux/include/linux/cleanup.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/err.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/uapi/asm/errno.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/asm-generic/errno.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/asm-generic/errno-base.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/args.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/jump_label.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/fence.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/insn-def.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/processor.h \
    $(wildcard include/config/MMU) \
    $(wildcard include/config/RISCV_ISA_ZICBOP) \
    $(wildcard include/config/RISCV_ISA_V) \
    $(wildcard include/config/RISCV_ISA_SUPM) \
  /data/hykang/RVSS/q-vela/linux/include/linux/cache.h \
    $(wildcard include/config/ARCH_HAS_CACHE_LINE_SIZE) \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/kernel.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/sysinfo.h \
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
  /data/hykang/RVSS/q-vela/linux/include/linux/stat.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/uapi/asm/stat.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/asm-generic/stat.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/stat.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/time.h \
    $(wildcard include/config/POSIX_TIMERS) \
  /data/hykang/RVSS/q-vela/linux/include/linux/math64.h \
    $(wildcard include/config/ARCH_SUPPORTS_INT128) \
  /data/hykang/RVSS/q-vela/linux/include/linux/math.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/asm/div64.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/div64.h \
    $(wildcard include/config/CC_OPTIMIZE_FOR_PERFORMANCE) \
  /data/hykang/RVSS/q-vela/linux/include/vdso/math64.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/time64.h \
  /data/hykang/RVSS/q-vela/linux/include/vdso/time64.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/time.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/time_types.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/time32.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/timex.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/timex.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/timex.h \
  /data/hykang/RVSS/q-vela/linux/include/vdso/time32.h \
  /data/hykang/RVSS/q-vela/linux/include/vdso/time.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/uidgid.h \
    $(wildcard include/config/MULTIUSER) \
    $(wildcard include/config/USER_NS) \
  /data/hykang/RVSS/q-vela/linux/include/linux/uidgid_types.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/highuid.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/buildid.h \
    $(wildcard include/config/VMCORE_INFO) \
  /data/hykang/RVSS/q-vela/linux/include/linux/kmod.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/umh.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/gfp.h \
    $(wildcard include/config/HIGHMEM) \
    $(wildcard include/config/ZONE_DMA) \
    $(wildcard include/config/ZONE_DMA32) \
    $(wildcard include/config/ZONE_DEVICE) \
    $(wildcard include/config/NUMA) \
    $(wildcard include/config/COMPACTION) \
    $(wildcard include/config/CONTIG_ALLOC) \
  /data/hykang/RVSS/q-vela/linux/include/linux/gfp_types.h \
    $(wildcard include/config/KASAN_HW_TAGS) \
    $(wildcard include/config/SLAB_OBJ_EXT) \
  /data/hykang/RVSS/q-vela/linux/include/linux/mmzone.h \
    $(wildcard include/config/ARCH_FORCE_MAX_ORDER) \
    $(wildcard include/config/PAGE_BLOCK_MAX_ORDER) \
    $(wildcard include/config/CMA) \
    $(wildcard include/config/MEMORY_ISOLATION) \
    $(wildcard include/config/ZSMALLOC) \
    $(wildcard include/config/UNACCEPTED_MEMORY) \
    $(wildcard include/config/IOMMU_SUPPORT) \
    $(wildcard include/config/SWAP) \
    $(wildcard include/config/NUMA_BALANCING) \
    $(wildcard include/config/HUGETLB_PAGE) \
    $(wildcard include/config/TRANSPARENT_HUGEPAGE) \
    $(wildcard include/config/LRU_GEN) \
    $(wildcard include/config/LRU_GEN_STATS) \
    $(wildcard include/config/LRU_GEN_WALKS_MMU) \
    $(wildcard include/config/MEMCG) \
    $(wildcard include/config/SPARSEMEM) \
    $(wildcard include/config/MEMORY_FAILURE) \
    $(wildcard include/config/FLATMEM) \
    $(wildcard include/config/PAGE_EXTENSION) \
    $(wildcard include/config/DEFERRED_STRUCT_PAGE_INIT) \
    $(wildcard include/config/HAVE_MEMORYLESS_NODES) \
    $(wildcard include/config/SPARSEMEM_VMEMMAP) \
    $(wildcard include/config/SPARSEMEM_EXTREME) \
    $(wildcard include/config/SPARSEMEM_VMEMMAP_PREINIT) \
    $(wildcard include/config/HAVE_ARCH_PFN_VALID) \
  /data/hykang/RVSS/q-vela/linux/include/linux/spinlock.h \
    $(wildcard include/config/PREEMPTION) \
    $(wildcard include/config/PREEMPT_RT) \
  /data/hykang/RVSS/q-vela/linux/include/linux/typecheck.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/preempt.h \
    $(wildcard include/config/PREEMPT_COUNT) \
    $(wildcard include/config/DEBUG_PREEMPT) \
    $(wildcard include/config/TRACE_PREEMPT_TOGGLE) \
    $(wildcard include/config/PREEMPT_NOTIFIERS) \
    $(wildcard include/config/PREEMPT_DYNAMIC) \
    $(wildcard include/config/PREEMPT_NONE) \
    $(wildcard include/config/PREEMPT_VOLUNTARY) \
    $(wildcard include/config/PREEMPT) \
    $(wildcard include/config/PREEMPT_LAZY) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/asm/preempt.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/preempt.h \
    $(wildcard include/config/HAVE_PREEMPT_DYNAMIC_KEY) \
  /data/hykang/RVSS/q-vela/linux/include/linux/thread_info.h \
    $(wildcard include/config/THREAD_INFO_IN_TASK) \
    $(wildcard include/config/GENERIC_ENTRY) \
    $(wildcard include/config/ARCH_HAS_PREEMPT_LAZY) \
    $(wildcard include/config/HAVE_ARCH_WITHIN_STACK_FRAMES) \
    $(wildcard include/config/SH) \
  /data/hykang/RVSS/q-vela/linux/include/linux/restart_block.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/errno.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/errno.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/current.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/bitops.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/bitops/generic-non-atomic.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/bitops.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/irqflags.h \
    $(wildcard include/config/PROVE_LOCKING) \
    $(wildcard include/config/TRACE_IRQFLAGS) \
    $(wildcard include/config/IRQSOFF_TRACER) \
    $(wildcard include/config/PREEMPT_TRACER) \
    $(wildcard include/config/DEBUG_IRQFLAGS) \
    $(wildcard include/config/TRACE_IRQFLAGS_SUPPORT) \
  /data/hykang/RVSS/q-vela/linux/include/linux/irqflags_types.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/irqflags.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/asm/percpu.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/percpu.h \
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
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/thread_info.h \
    $(wildcard include/config/KASAN) \
    $(wildcard include/config/THREAD_SIZE_ORDER) \
    $(wildcard include/config/VMAP_STACK) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/page.h \
    $(wildcard include/config/XIP_KERNEL) \
    $(wildcard include/config/RISCV_ISA_ZICBOZ) \
    $(wildcard include/config/DEBUG_VIRTUAL) \
  /data/hykang/RVSS/q-vela/linux/include/linux/pfn.h \
  /data/hykang/RVSS/q-vela/linux/include/vdso/page.h \
    $(wildcard include/config/PAGE_SHIFT) \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/memory_model.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/getorder.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/log2.h \
    $(wildcard include/config/ARCH_HAS_ILOG2_U32) \
    $(wildcard include/config/ARCH_HAS_ILOG2_U64) \
  /data/hykang/RVSS/q-vela/linux/include/linux/sizes.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/thread_info_tif.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/bottom_half.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/instruction_pointer.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/lockdep.h \
    $(wildcard include/config/DEBUG_LOCKING_API_SELFTESTS) \
  /data/hykang/RVSS/q-vela/linux/include/linux/smp.h \
    $(wildcard include/config/UP_LATE_INIT) \
    $(wildcard include/config/CSD_LOCK_WAIT_DEBUG) \
  /data/hykang/RVSS/q-vela/linux/include/linux/cpumask.h \
    $(wildcard include/config/FORCE_NR_CPUS) \
    $(wildcard include/config/HOTPLUG_CPU) \
    $(wildcard include/config/DEBUG_PER_CPU_MAPS) \
    $(wildcard include/config/CPUMASK_OFFSTACK) \
  /data/hykang/RVSS/q-vela/linux/include/linux/kernel.h \
    $(wildcard include/config/PREEMPT_VOLUNTARY_BUILD) \
    $(wildcard include/config/HAVE_PREEMPT_DYNAMIC_CALL) \
    $(wildcard include/config/PREEMPT_) \
    $(wildcard include/config/DEBUG_ATOMIC_SLEEP) \
  /data/hykang/RVSS/q-vela/linux/include/linux/align.h \
  /data/hykang/RVSS/q-vela/linux/include/vdso/align.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/array_size.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/hex.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/kstrtox.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/minmax.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/sprintf.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/static_call_types.h \
    $(wildcard include/config/HAVE_STATIC_CALL) \
  /data/hykang/RVSS/q-vela/linux/include/linux/util_macros.h \
    $(wildcard include/config/FOO_SUSPEND) \
  /data/hykang/RVSS/q-vela/linux/include/linux/wordpart.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/bitmap.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/find.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/string.h \
    $(wildcard include/config/BINARY_PRINTF) \
    $(wildcard include/config/FORTIFY_SOURCE) \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/string.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/string.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/bitmap-str.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/cpumask_types.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/atomic.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/atomic.h \
    $(wildcard include/config/GENERIC_ATOMIC64) \
  /data/hykang/RVSS/q-vela/linux/include/linux/atomic/atomic-arch-fallback.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/atomic/atomic-long.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/atomic/atomic-instrumented.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/numa.h \
    $(wildcard include/config/NUMA_KEEP_MEMINFO) \
    $(wildcard include/config/HAVE_ARCH_NODE_DEV_GROUP) \
  /data/hykang/RVSS/q-vela/linux/include/linux/nodemask.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/nodemask_types.h \
    $(wildcard include/config/NODES_SHIFT) \
  /data/hykang/RVSS/q-vela/linux/include/linux/random.h \
    $(wildcard include/config/VMGENID) \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/random.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/ioctl.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/uapi/asm/ioctl.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/ioctl.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/asm-generic/ioctl.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/irqnr.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/irqnr.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/smp_types.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/llist.h \
    $(wildcard include/config/ARCH_HAVE_NMI_SAFE_CMPXCHG) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/smp.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/irqreturn.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/mmiowb.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/mmiowb.h \
    $(wildcard include/config/MMIOWB) \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/mmiowb_types.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/spinlock_types.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/rwlock_types.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/spinlock.h \
    $(wildcard include/config/QUEUED_SPINLOCKS) \
    $(wildcard include/config/RISCV_COMBO_SPINLOCKS) \
    $(wildcard include/config/RISCV_QUEUED_SPINLOCKS) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/asm/ticket_spinlock.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/ticket_spinlock.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/asm/qspinlock.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/qspinlock.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/asm/qrwlock.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/qrwlock.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/rwlock.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/spinlock_api_smp.h \
    $(wildcard include/config/INLINE_SPIN_LOCK) \
    $(wildcard include/config/INLINE_SPIN_LOCK_BH) \
    $(wildcard include/config/INLINE_SPIN_LOCK_IRQ) \
    $(wildcard include/config/INLINE_SPIN_LOCK_IRQSAVE) \
    $(wildcard include/config/INLINE_SPIN_TRYLOCK) \
    $(wildcard include/config/INLINE_SPIN_TRYLOCK_BH) \
    $(wildcard include/config/UNINLINE_SPIN_UNLOCK) \
    $(wildcard include/config/INLINE_SPIN_UNLOCK_BH) \
    $(wildcard include/config/INLINE_SPIN_UNLOCK_IRQ) \
    $(wildcard include/config/INLINE_SPIN_UNLOCK_IRQRESTORE) \
    $(wildcard include/config/GENERIC_LOCKBREAK) \
  /data/hykang/RVSS/q-vela/linux/include/linux/rwlock_api_smp.h \
    $(wildcard include/config/INLINE_READ_LOCK) \
    $(wildcard include/config/INLINE_WRITE_LOCK) \
    $(wildcard include/config/INLINE_READ_LOCK_BH) \
    $(wildcard include/config/INLINE_WRITE_LOCK_BH) \
    $(wildcard include/config/INLINE_READ_LOCK_IRQ) \
    $(wildcard include/config/INLINE_WRITE_LOCK_IRQ) \
    $(wildcard include/config/INLINE_READ_LOCK_IRQSAVE) \
    $(wildcard include/config/INLINE_WRITE_LOCK_IRQSAVE) \
    $(wildcard include/config/INLINE_READ_TRYLOCK) \
    $(wildcard include/config/INLINE_WRITE_TRYLOCK) \
    $(wildcard include/config/INLINE_READ_UNLOCK) \
    $(wildcard include/config/INLINE_WRITE_UNLOCK) \
    $(wildcard include/config/INLINE_READ_UNLOCK_BH) \
    $(wildcard include/config/INLINE_WRITE_UNLOCK_BH) \
    $(wildcard include/config/INLINE_READ_UNLOCK_IRQ) \
    $(wildcard include/config/INLINE_WRITE_UNLOCK_IRQ) \
    $(wildcard include/config/INLINE_READ_UNLOCK_IRQRESTORE) \
    $(wildcard include/config/INLINE_WRITE_UNLOCK_IRQRESTORE) \
  /data/hykang/RVSS/q-vela/linux/include/linux/list_nulls.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/wait.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/seqlock.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/mutex.h \
    $(wildcard include/config/DEBUG_MUTEXES) \
  /data/hykang/RVSS/q-vela/linux/include/linux/osq_lock.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/debug_locks.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/mutex_types.h \
    $(wildcard include/config/MUTEX_SPIN_ON_OWNER) \
  /data/hykang/RVSS/q-vela/linux/include/linux/seqlock_types.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/pageblock-flags.h \
    $(wildcard include/config/HUGETLB_PAGE_SIZE_VARIABLE) \
  /data/hykang/RVSS/q-vela/linux/include/linux/page-flags-layout.h \
  /data/hykang/RVSS/q-vela/linux/include/generated/bounds.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/sparsemem.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/mm_types.h \
    $(wildcard include/config/HAVE_ALIGNED_STRUCT_PAGE) \
    $(wildcard include/config/HUGETLB_PMD_PAGE_TABLE_SHARING) \
    $(wildcard include/config/SLAB_FREELIST_HARDENED) \
    $(wildcard include/config/USERFAULTFD) \
    $(wildcard include/config/ANON_VMA_NAME) \
    $(wildcard include/config/PER_VMA_LOCK) \
    $(wildcard include/config/SCHED_MM_CID) \
    $(wildcard include/config/HAVE_ARCH_COMPAT_MMAP_BASES) \
    $(wildcard include/config/MEMBARRIER) \
    $(wildcard include/config/FUTEX_PRIVATE_HASH) \
    $(wildcard include/config/ARCH_HAS_ELF_CORE_EFLAGS) \
    $(wildcard include/config/AIO) \
    $(wildcard include/config/MMU_NOTIFIER) \
    $(wildcard include/config/SPLIT_PMD_PTLOCKS) \
    $(wildcard include/config/ARCH_WANT_BATCHED_UNMAP_TLB_FLUSH) \
    $(wildcard include/config/IOMMU_MM_DATA) \
    $(wildcard include/config/KSM) \
    $(wildcard include/config/MM_ID) \
    $(wildcard include/config/CORE_DUMP_DEFAULT_ELF_HEADERS) \
  /data/hykang/RVSS/q-vela/linux/include/linux/mm_types_task.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/tlbbatch.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/auxvec.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/auxvec.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/uapi/asm/auxvec.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/kref.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/refcount.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/refcount_types.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/rbtree.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/rbtree_types.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/rcupdate.h \
    $(wildcard include/config/PREEMPT_RCU) \
    $(wildcard include/config/TINY_RCU) \
    $(wildcard include/config/RCU_STRICT_GRACE_PERIOD) \
    $(wildcard include/config/RCU_LAZY) \
    $(wildcard include/config/RCU_STALL_COMMON) \
    $(wildcard include/config/NO_HZ_FULL) \
    $(wildcard include/config/VIRT_XFER_TO_GUEST_WORK) \
    $(wildcard include/config/RCU_NOCB_CPU) \
    $(wildcard include/config/TASKS_RCU_GENERIC) \
    $(wildcard include/config/TASKS_RCU) \
    $(wildcard include/config/TASKS_TRACE_RCU) \
    $(wildcard include/config/TASKS_RUDE_RCU) \
    $(wildcard include/config/TREE_RCU) \
    $(wildcard include/config/DEBUG_OBJECTS_RCU_HEAD) \
    $(wildcard include/config/PROVE_RCU) \
    $(wildcard include/config/ARCH_WEAK_RELEASE_ACQUIRE) \
  /data/hykang/RVSS/q-vela/linux/include/linux/sched.h \
    $(wildcard include/config/VIRT_CPU_ACCOUNTING_NATIVE) \
    $(wildcard include/config/SCHED_INFO) \
    $(wildcard include/config/SCHEDSTATS) \
    $(wildcard include/config/SCHED_CORE) \
    $(wildcard include/config/FAIR_GROUP_SCHED) \
    $(wildcard include/config/RT_GROUP_SCHED) \
    $(wildcard include/config/RT_MUTEXES) \
    $(wildcard include/config/UCLAMP_TASK) \
    $(wildcard include/config/UCLAMP_BUCKETS_COUNT) \
    $(wildcard include/config/KMAP_LOCAL) \
    $(wildcard include/config/MEM_ALLOC_PROFILING) \
    $(wildcard include/config/SCHED_CLASS_EXT) \
    $(wildcard include/config/CGROUP_SCHED) \
    $(wildcard include/config/CFS_BANDWIDTH) \
    $(wildcard include/config/BLK_DEV_IO_TRACE) \
    $(wildcard include/config/MEMCG_V1) \
    $(wildcard include/config/COMPAT_BRK) \
    $(wildcard include/config/CGROUPS) \
    $(wildcard include/config/BLK_CGROUP) \
    $(wildcard include/config/PSI) \
    $(wildcard include/config/PAGE_OWNER) \
    $(wildcard include/config/EVENTFD) \
    $(wildcard include/config/ARCH_HAS_CPU_PASID) \
    $(wildcard include/config/X86_BUS_LOCK_DETECT) \
    $(wildcard include/config/TASK_DELAY_ACCT) \
    $(wildcard include/config/STACKPROTECTOR) \
    $(wildcard include/config/ARCH_HAS_SCALED_CPUTIME) \
    $(wildcard include/config/VIRT_CPU_ACCOUNTING_GEN) \
    $(wildcard include/config/POSIX_CPUTIMERS) \
    $(wildcard include/config/POSIX_CPU_TIMERS_TASK_WORK) \
    $(wildcard include/config/KEYS) \
    $(wildcard include/config/SYSVIPC) \
    $(wildcard include/config/DETECT_HUNG_TASK) \
    $(wildcard include/config/IO_URING) \
    $(wildcard include/config/AUDIT) \
    $(wildcard include/config/AUDITSYSCALL) \
    $(wildcard include/config/DETECT_HUNG_TASK_BLOCKER) \
    $(wildcard include/config/UBSAN) \
    $(wildcard include/config/UBSAN_TRAP) \
    $(wildcard include/config/TASK_XACCT) \
    $(wildcard include/config/CPUSETS) \
    $(wildcard include/config/X86_CPU_RESCTRL) \
    $(wildcard include/config/FUTEX) \
    $(wildcard include/config/COMPAT) \
    $(wildcard include/config/PERF_EVENTS) \
    $(wildcard include/config/RSEQ) \
    $(wildcard include/config/DEBUG_RSEQ) \
    $(wildcard include/config/FAULT_INJECTION) \
    $(wildcard include/config/LATENCYTOP) \
    $(wildcard include/config/FUNCTION_GRAPH_TRACER) \
    $(wildcard include/config/UPROBES) \
    $(wildcard include/config/BCACHE) \
    $(wildcard include/config/SECURITY) \
    $(wildcard include/config/BPF_SYSCALL) \
    $(wildcard include/config/KSTACK_ERASE) \
    $(wildcard include/config/KSTACK_ERASE_METRICS) \
    $(wildcard include/config/X86_MCE) \
    $(wildcard include/config/KRETPROBES) \
    $(wildcard include/config/RETHOOK) \
    $(wildcard include/config/ARCH_HAS_PARANOID_L1D_FLUSH) \
    $(wildcard include/config/RV) \
    $(wildcard include/config/RV_PER_TASK_MONITORS) \
    $(wildcard include/config/USER_EVENTS) \
    $(wildcard include/config/UNWIND_USER) \
    $(wildcard include/config/SCHED_PROXY_EXEC) \
    $(wildcard include/config/MEM_ALLOC_PROFILING_DEBUG) \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/sched.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/pid_types.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/sem_types.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/shm.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/asm/shmparam.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/shmparam.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/kmsan_types.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/plist_types.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/hrtimer_types.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/timerqueue_types.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/timer_types.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/seccomp_types.h \
    $(wildcard include/config/SECCOMP) \
  /data/hykang/RVSS/q-vela/linux/include/linux/resource.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/resource.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/uapi/asm/resource.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/resource.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/asm-generic/resource.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/latencytop.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/sched/prio.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/sched/types.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/signal_types.h \
    $(wildcard include/config/OLD_SIGACTION) \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/signal.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/uapi/asm/signal.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/signal.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/asm-generic/signal.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/asm-generic/signal-defs.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/uapi/asm/sigcontext.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/uapi/asm/siginfo.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/asm-generic/siginfo.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/syscall_user_dispatch_types.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/netdevice_xmit.h \
    $(wildcard include/config/NET_EGRESS) \
    $(wildcard include/config/NET_ACT_MIRRED) \
    $(wildcard include/config/NF_DUP_NETDEV) \
  /data/hykang/RVSS/q-vela/linux/include/linux/task_io_accounting.h \
    $(wildcard include/config/TASK_IO_ACCOUNTING) \
  /data/hykang/RVSS/q-vela/linux/include/linux/posix-timers_types.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/rseq.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/kcsan.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/rv.h \
    $(wildcard include/config/RV_LTL_MONITOR) \
    $(wildcard include/config/RV_REACTORS) \
  /data/hykang/RVSS/q-vela/linux/include/linux/tracepoint-defs.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/static_key.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/unwind_deferred_types.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/asm/kmap_size.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/kmap_size.h \
    $(wildcard include/config/DEBUG_KMAP_LOCAL) \
  /data/hykang/RVSS/q-vela/linux/include/generated/rq-offsets.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/sched/ext.h \
    $(wildcard include/config/EXT_GROUP_SCHED) \
  /data/hykang/RVSS/q-vela/linux/include/linux/context_tracking_irq.h \
    $(wildcard include/config/CONTEXT_TRACKING_IDLE) \
  /data/hykang/RVSS/q-vela/linux/include/linux/rcutree.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/maple_tree.h \
    $(wildcard include/config/MAPLE_RCU_DISABLED) \
    $(wildcard include/config/DEBUG_MAPLE_TREE) \
  /data/hykang/RVSS/q-vela/linux/include/linux/rwsem.h \
    $(wildcard include/config/RWSEM_SPIN_ON_OWNER) \
    $(wildcard include/config/DEBUG_RWSEMS) \
  /data/hykang/RVSS/q-vela/linux/include/linux/completion.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/swait.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/uprobes.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/timer.h \
    $(wildcard include/config/DEBUG_OBJECTS_TIMERS) \
  /data/hykang/RVSS/q-vela/linux/include/linux/ktime.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/jiffies.h \
  /data/hykang/RVSS/q-vela/linux/include/vdso/jiffies.h \
  /data/hykang/RVSS/q-vela/linux/include/generated/timeconst.h \
  /data/hykang/RVSS/q-vela/linux/include/vdso/ktime.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/timekeeping.h \
    $(wildcard include/config/POSIX_AUX_CLOCKS) \
    $(wildcard include/config/GENERIC_CMOS_UPDATE) \
  /data/hykang/RVSS/q-vela/linux/include/linux/clocksource_ids.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/debugobjects.h \
    $(wildcard include/config/DEBUG_OBJECTS) \
    $(wildcard include/config/DEBUG_OBJECTS_FREE) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/uprobes.h \
    $(wildcard include/config/RISCV_ISA_C) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/probes.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/text-patching.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/workqueue.h \
    $(wildcard include/config/DEBUG_OBJECTS_WORK) \
    $(wildcard include/config/FREEZER) \
    $(wildcard include/config/WQ_WATCHDOG) \
  /data/hykang/RVSS/q-vela/linux/include/linux/alloc_tag.h \
    $(wildcard include/config/MEM_ALLOC_PROFILING_ENABLED_BY_DEFAULT) \
  /data/hykang/RVSS/q-vela/linux/include/linux/codetag.h \
    $(wildcard include/config/CODE_TAGGING) \
  /data/hykang/RVSS/q-vela/linux/include/linux/workqueue_types.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/percpu_counter.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/percpu.h \
    $(wildcard include/config/RANDOM_KMALLOC_CACHES) \
    $(wildcard include/config/PAGE_SIZE_4KB) \
    $(wildcard include/config/NEED_PER_CPU_PAGE_FIRST_CHUNK) \
  /data/hykang/RVSS/q-vela/linux/include/linux/mmdebug.h \
    $(wildcard include/config/DEBUG_VM) \
    $(wildcard include/config/DEBUG_VM_IRQSOFF) \
    $(wildcard include/config/DEBUG_VM_PGFLAGS) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/mmu.h \
    $(wildcard include/config/BINFMT_ELF_FDPIC) \
  /data/hykang/RVSS/q-vela/linux/include/linux/page-flags.h \
    $(wildcard include/config/PAGE_IDLE_FLAG) \
    $(wildcard include/config/ARCH_USES_PG_ARCH_2) \
    $(wildcard include/config/ARCH_USES_PG_ARCH_3) \
    $(wildcard include/config/MIGRATION) \
    $(wildcard include/config/HUGETLB_PAGE_OPTIMIZE_VMEMMAP) \
    $(wildcard include/config/DEBUG_KMAP_LOCAL_FORCE_MAP) \
  /data/hykang/RVSS/q-vela/linux/include/linux/local_lock.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/local_lock_internal.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/zswap.h \
    $(wildcard include/config/ZSWAP) \
  /data/hykang/RVSS/q-vela/linux/include/linux/memory_hotplug.h \
    $(wildcard include/config/ARCH_HAS_ADD_PAGES) \
    $(wildcard include/config/MEMORY_HOTREMOVE) \
  /data/hykang/RVSS/q-vela/linux/include/linux/notifier.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/srcu.h \
    $(wildcard include/config/TINY_SRCU) \
    $(wildcard include/config/NEED_SRCU_NMI_SAFE) \
  /data/hykang/RVSS/q-vela/linux/include/linux/rcu_segcblist.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/srcutree.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/rcu_node_tree.h \
    $(wildcard include/config/RCU_FANOUT) \
    $(wildcard include/config/RCU_FANOUT_LEAF) \
  /data/hykang/RVSS/q-vela/linux/include/linux/topology.h \
    $(wildcard include/config/USE_PERCPU_NUMA_NODE_ID) \
    $(wildcard include/config/SCHED_SMT) \
    $(wildcard include/config/GENERIC_ARCH_TOPOLOGY) \
  /data/hykang/RVSS/q-vela/linux/include/linux/arch_topology.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/topology.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/topology.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/sysctl.h \
    $(wildcard include/config/SYSCTL) \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/sysctl.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/elf.h \
    $(wildcard include/config/ARCH_HAVE_EXTRA_ELF_NOTES) \
    $(wildcard include/config/ARCH_USE_GNU_PROPERTY) \
    $(wildcard include/config/ARCH_HAVE_ELF_PROT) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/elf.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/elf.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/elf-em.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/compat.h \
    $(wildcard include/config/ARCH_HAS_SYSCALL_WRAPPER) \
    $(wildcard include/config/X86_X32_ABI) \
    $(wildcard include/config/COMPAT_OLD_SIGACTION) \
    $(wildcard include/config/HARDENED_USERCOPY) \
    $(wildcard include/config/ODD_RT_SIGACTION) \
  /data/hykang/RVSS/q-vela/linux/include/linux/sem.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/sem.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/ipc.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/rhashtable-types.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/ipc.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/uapi/asm/ipcbuf.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/asm-generic/ipcbuf.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/uapi/asm/sembuf.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/asm-generic/sembuf.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/socket.h \
    $(wildcard include/config/PROC_FS) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/uapi/asm/socket.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/asm-generic/socket.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/uapi/asm/sockios.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/asm-generic/sockios.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/sockios.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/uio.h \
    $(wildcard include/config/ARCH_HAS_UACCESS_FLUSHCACHE) \
    $(wildcard include/config/ARCH_HAS_COPY_MC) \
  /data/hykang/RVSS/q-vela/linux/include/linux/ucopysize.h \
    $(wildcard include/config/HARDENED_USERCOPY_DEFAULT_ON) \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/uio.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/socket.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/if.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/libc-compat.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/hdlc/ioctl.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/fs.h \
    $(wildcard include/config/FANOTIFY_ACCESS_PERMISSIONS) \
    $(wildcard include/config/READ_ONLY_THP_FOR_FS) \
    $(wildcard include/config/FS_POSIX_ACL) \
    $(wildcard include/config/CGROUP_WRITEBACK) \
    $(wildcard include/config/IMA) \
    $(wildcard include/config/FILE_LOCKING) \
    $(wildcard include/config/FSNOTIFY) \
    $(wildcard include/config/EPOLL) \
    $(wildcard include/config/UNICODE) \
    $(wildcard include/config/FS_ENCRYPTION) \
    $(wildcard include/config/FS_VERITY) \
    $(wildcard include/config/QUOTA) \
    $(wildcard include/config/FS_DAX) \
    $(wildcard include/config/BLOCK) \
  /data/hykang/RVSS/q-vela/linux/include/linux/vfsdebug.h \
    $(wildcard include/config/DEBUG_VFS) \
  /data/hykang/RVSS/q-vela/linux/include/linux/wait_bit.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/kdev_t.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/kdev_t.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/dcache.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/rculist.h \
    $(wildcard include/config/PROVE_RCU_LIST) \
  /data/hykang/RVSS/q-vela/linux/include/linux/rculist_bl.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/list_bl.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/bit_spinlock.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/lockref.h \
    $(wildcard include/config/ARCH_USE_CMPXCHG_LOCKREF) \
  /data/hykang/RVSS/q-vela/linux/include/linux/stringhash.h \
    $(wildcard include/config/DCACHE_WORD_ACCESS) \
  /data/hykang/RVSS/q-vela/linux/include/linux/hash.h \
    $(wildcard include/config/HAVE_ARCH_HASH) \
  /data/hykang/RVSS/q-vela/linux/include/linux/path.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/list_lru.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/shrinker.h \
    $(wildcard include/config/SHRINKER_DEBUG) \
  /data/hykang/RVSS/q-vela/linux/include/linux/xarray.h \
    $(wildcard include/config/XARRAY_MULTI) \
  /data/hykang/RVSS/q-vela/linux/include/linux/sched/mm.h \
    $(wildcard include/config/MMU_LAZY_TLB_REFCOUNT) \
    $(wildcard include/config/ARCH_HAS_MEMBARRIER_CALLBACKS) \
    $(wildcard include/config/ARCH_HAS_SYNC_CORE_BEFORE_USERMODE) \
  /data/hykang/RVSS/q-vela/linux/include/linux/sync_core.h \
    $(wildcard include/config/ARCH_HAS_PREPARE_SYNC_CORE_CMD) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/sync_core.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/sched/coredump.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/membarrier.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/radix-tree.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/pid.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/capability.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/capability.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/semaphore.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/fcntl.h \
    $(wildcard include/config/ARCH_32BIT_OFF_T) \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/fcntl.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/uapi/asm/fcntl.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/asm-generic/fcntl.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/openat2.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/migrate_mode.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/percpu-rwsem.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/rcuwait.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/sched/signal.h \
    $(wildcard include/config/SCHED_AUTOGROUP) \
    $(wildcard include/config/BSD_PROCESS_ACCT) \
    $(wildcard include/config/TASKSTATS) \
    $(wildcard include/config/STACK_GROWSUP) \
  /data/hykang/RVSS/q-vela/linux/include/linux/signal.h \
    $(wildcard include/config/DYNAMIC_SIGFRAME) \
  /data/hykang/RVSS/q-vela/linux/include/linux/sched/jobctl.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/sched/task.h \
    $(wildcard include/config/HAVE_EXIT_THREAD) \
    $(wildcard include/config/ARCH_WANTS_DYNAMIC_TASK_STRUCT) \
    $(wildcard include/config/HAVE_ARCH_THREAD_STRUCT_WHITELIST) \
  /data/hykang/RVSS/q-vela/linux/include/linux/uaccess.h \
    $(wildcard include/config/ARCH_HAS_SUBPAGE_FAULTS) \
  /data/hykang/RVSS/q-vela/linux/include/linux/fault-inject-usercopy.h \
    $(wildcard include/config/FAULT_INJECTION_USERCOPY) \
  /data/hykang/RVSS/q-vela/linux/include/linux/nospec.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/uaccess.h \
    $(wildcard include/config/CC_HAS_ASM_GOTO_OUTPUT) \
    $(wildcard include/config/HAVE_EFFICIENT_UNALIGNED_ACCESS) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/asm-extable.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/gpr-num.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/cpufeature.h \
    $(wildcard include/config/RISCV_SCALAR_MISALIGNED) \
    $(wildcard include/config/RISCV_MISALIGNED) \
    $(wildcard include/config/RISCV_VECTOR_MISALIGNED) \
    $(wildcard include/config/RISCV_PROBE_UNALIGNED_ACCESS) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/pgtable.h \
    $(wildcard include/config/RELOCATABLE) \
    $(wildcard include/config/PHYS_RAM_BASE) \
    $(wildcard include/config/XIP_PHYS_ADDR) \
    $(wildcard include/config/RISCV_ISA_SVNAPOT) \
    $(wildcard include/config/ARCH_SUPPORTS_PMD_PFNMAP) \
    $(wildcard include/config/ARCH_SUPPORTS_PUD_PFNMAP) \
    $(wildcard include/config/PAGE_TABLE_CHECK) \
    $(wildcard include/config/ARCH_ENABLE_THP_MIGRATION) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/pgtable-bits.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/tlbflush.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/compat.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/compat.h \
    $(wildcard include/config/COMPAT_FOR_U64_ALIGNMENT) \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/pgtable-64.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/page_table_check.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/extable.h \
    $(wildcard include/config/BPF_JIT) \
    $(wildcard include/config/ARCH_RV64I) \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/access_ok.h \
    $(wildcard include/config/ALTERNATE_USER_ADDRESS_SPACE) \
  /data/hykang/RVSS/q-vela/linux/include/linux/cred.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/key.h \
    $(wildcard include/config/KEY_NOTIFICATIONS) \
    $(wildcard include/config/NET) \
  /data/hykang/RVSS/q-vela/linux/include/linux/assoc_array.h \
    $(wildcard include/config/ASSOCIATIVE_ARRAY) \
  /data/hykang/RVSS/q-vela/linux/include/linux/sched/user.h \
    $(wildcard include/config/VFIO_PCI_ZDEV_KVM) \
    $(wildcard include/config/IOMMUFD) \
    $(wildcard include/config/WATCH_QUEUE) \
  /data/hykang/RVSS/q-vela/linux/include/linux/ratelimit.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/posix-timers.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/alarmtimer.h \
    $(wildcard include/config/RTC_CLASS) \
  /data/hykang/RVSS/q-vela/linux/include/linux/hrtimer.h \
    $(wildcard include/config/HIGH_RES_TIMERS) \
    $(wildcard include/config/TIME_LOW_RES) \
    $(wildcard include/config/TIMERFD) \
  /data/hykang/RVSS/q-vela/linux/include/linux/hrtimer_defs.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/timerqueue.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/rcuref.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/rcu_sync.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/delayed_call.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/uuid.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/errseq.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/ioprio.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/sched/rt.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/iocontext.h \
    $(wildcard include/config/BLK_ICQ) \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/ioprio.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/fs_types.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/mount.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/mnt_idmapping.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/slab.h \
    $(wildcard include/config/FAILSLAB) \
    $(wildcard include/config/KFENCE) \
    $(wildcard include/config/SLUB_TINY) \
    $(wildcard include/config/SLUB_DEBUG) \
    $(wildcard include/config/SLAB_BUCKETS) \
    $(wildcard include/config/KVFREE_RCU_BATCHED) \
  /data/hykang/RVSS/q-vela/linux/include/linux/percpu-refcount.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/kasan.h \
    $(wildcard include/config/KASAN_STACK) \
    $(wildcard include/config/KASAN_VMALLOC) \
  /data/hykang/RVSS/q-vela/linux/include/linux/kasan-enabled.h \
    $(wildcard include/config/ARCH_DEFER_KASAN) \
  /data/hykang/RVSS/q-vela/linux/include/linux/kasan-tags.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/rw_hint.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/file_ref.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/unicode.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/fs.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/quota.h \
    $(wildcard include/config/QUOTA_NETLINK_INTERFACE) \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/dqblk_xfs.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/dqblk_v1.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/dqblk_v2.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/dqblk_qtree.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/projid.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/quota.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/aio_abi.h \
  /data/hykang/RVSS/q-vela/linux/include/uapi/linux/unistd.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/unistd.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/uapi/asm/unistd.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/generated/uapi/asm/unistd_64.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/syscall_wrapper.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/uapi/asm/elf.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/cacheinfo.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/cacheinfo.h \
    $(wildcard include/config/ACPI_PPTT) \
    $(wildcard include/config/ARM) \
    $(wildcard include/config/ARCH_HAS_CPU_CACHE_ALIASING) \
  /data/hykang/RVSS/q-vela/linux/include/linux/cpuhplock.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/kobject.h \
    $(wildcard include/config/UEVENT_HELPER) \
    $(wildcard include/config/DEBUG_KOBJECT_RELEASE) \
  /data/hykang/RVSS/q-vela/linux/include/linux/sysfs.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/kernfs.h \
    $(wildcard include/config/KERNFS) \
  /data/hykang/RVSS/q-vela/linux/include/linux/idr.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/kobject_ns.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/moduleparam.h \
    $(wildcard include/config/ALPHA) \
    $(wildcard include/config/PPC64) \
  /data/hykang/RVSS/q-vela/linux/include/linux/rbtree_latch.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/error-injection.h \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/error-injection.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/module.h \
    $(wildcard include/config/MODULE_SECTIONS) \
  /data/hykang/RVSS/q-vela/linux/include/asm-generic/module.h \
    $(wildcard include/config/HAVE_MOD_ARCH_SPECIFIC) \
  /data/hykang/RVSS/q-vela/linux/include/linux/build-salt.h \
    $(wildcard include/config/BUILD_SALT) \
  /data/hykang/RVSS/q-vela/linux/include/linux/elfnote.h \
  /data/hykang/RVSS/q-vela/linux/include/linux/elfnote-lto.h \
    $(wildcard include/config/LTO) \
  /data/hykang/RVSS/q-vela/linux/include/linux/vermagic.h \
    $(wildcard include/config/PREEMPT_BUILD) \
  /data/hykang/RVSS/q-vela/linux/include/generated/utsrelease.h \
  /data/hykang/RVSS/q-vela/linux/arch/riscv/include/asm/vermagic.h \

.module-common.o: $(deps_.module-common.o)

$(deps_.module-common.o):
