#include <sbi/sbi_string.h>
#include <sbi/sbi_trap.h>
#include <sbi/sbi_error.h>
#include <sbi/sbi_console.h>
#include <sbi/riscv_asm.h>
#include <sbi/sbi_ecall.h>

#include <vyond.h>
#include <mprv.h>

#include <sbi/riscv_encoding.h>
#include <sbi/sbi_hart.h>
#include <sbi/sbi_illegal_insn.h>
#include <sbi/sbi_ipi.h>
#include <sbi/sbi_misaligned_ldst.h>
#include <sbi/sbi_timer.h>

unsigned long sbi_sm_create_enclave(unsigned long* edi, uintptr_t create_args);
unsigned long sbi_sm_destroy_enclave(unsigned long eid);
unsigned long sbi_sm_enter_enclave(struct sbi_trap_regs *regs, unsigned long eid);
unsigned long sbi_sm_resume_enclave(struct sbi_trap_regs *regs, unsigned long eid);
unsigned long sbi_sm_stop_enclave(struct sbi_trap_regs *regs, unsigned long request);
unsigned long sbi_sm_exit_enclave(struct sbi_trap_regs *regs);
/* WGC slot virtualization helpers */
uintptr_t    sbi_sm_get_enclave_id(void);
long         sbi_sm_handle_wgc_fault(uintptr_t eid, uintptr_t fault_addr);

/*
 * SV39 VA→PA translation executed from M-mode.
 *
 * M-mode loads use physical addresses UNLESS mstatus.MPRV=1, in which
 * case they use the privilege level in mstatus.MPP and go through satp.
 * We must clear MPRV before reading PTEs, otherwise we'd be recursively
 * applying VA translation to what we think are physical PTE addresses —
 * producing completely wrong results.
 *
 * Returns 0 if the address is unmapped or the mode is not SV39.
 */
static uintptr_t sv39_translate(uintptr_t satp, uintptr_t va)
{
    if ((satp >> 60) != 8)   /* mode field: 8 = SV39 */
        return 0;

    /* Save and clear MPRV so our PTE loads are truly physical. */
    uintptr_t orig_mstatus = csr_read(CSR_MSTATUS);
    if (orig_mstatus & MSTATUS_MPRV)
        csr_write(CSR_MSTATUS, orig_mstatus & ~MSTATUS_MPRV);

    uintptr_t pt_pa = (satp & 0x00000FFFFFFFFFFF) << 12;
    uintptr_t result = 0;

    for (int level = 2; level >= 0; level--) {
        int shift = 12 + level * 9;
        uintptr_t vpn = (va >> shift) & 0x1FF;
        uintptr_t pte = *(volatile uint64_t *)(pt_pa + vpn * 8);

        if (!(pte & 0x1))          /* V bit clear: not mapped */
            goto done;
        if (pte & 0xE) {           /* R/W/X set: leaf PTE */
            uintptr_t leaf_ppn = (pte >> 10) & 0x00000FFFFFFFFFFF;
            uintptr_t offset_mask = (1UL << shift) - 1;
            result = (leaf_ppn << 12) | (va & offset_mask);
            goto done;
        }
        /* Non-leaf: descend */
        pt_pa = ((pte >> 10) & 0x00000FFFFFFFFFFF) << 12;
    }

done:
    if (orig_mstatus & MSTATUS_MPRV)
        csr_write(CSR_MSTATUS, orig_mstatus);
    return result;
}

unsigned long copy_enclave_create_args(uintptr_t src, struct keystone_sbi_create_t* dest);
unsigned long sbi_sm_attest_enclave(unsigned long report, unsigned long data, unsigned long size);
unsigned long sbi_sm_create_shm_region(unsigned long *rid, uintptr_t pa, unsigned long size);
unsigned long sbi_sm_map_shm_region(struct sbi_trap_regs *regs, unsigned long rid);
unsigned long sbi_sm_unmap_shm_region(unsigned long rid);
unsigned long sbi_sm_change_shm_region(unsigned long rid, unsigned long dyn_perm);
unsigned long sbi_sm_share_shm_region(unsigned long rid, unsigned long eid2share, unsigned long st_perm);
/* Device IRQ path */
unsigned long sbi_sm_register_dev_irq(uint32_t irq_num);
unsigned long sbi_sm_wait_dev_data(struct sbi_trap_regs *regs, uint32_t irq_num);
long         sbi_sm_handle_dev_irq(struct sbi_trap_regs *regs, uint32_t irq_num);

/* PLIC M-mode claim/complete  (QEMU virt: M-mode context = hartid * 2) */
#define PLIC_BASE_ADDR  0xc000000UL
#define PLIC_CLAIM_REG(ctx) ((volatile uint32_t *)(PLIC_BASE_ADDR + 0x200004UL + (ulong)(ctx) * 0x1000UL))

static inline uint32_t plic_claim_m(ulong hartid) {
    return *PLIC_CLAIM_REG(hartid * 2);
}
static inline void plic_complete_m(ulong hartid, uint32_t irq) {
    *PLIC_CLAIM_REG(hartid * 2) = irq;
}

static int sbi_ecall_vyond_monitor_handler(
    unsigned long extid, unsigned long funcid,
    const struct sbi_trap_regs *regs,
    unsigned long *out_val,
    struct sbi_trap_info *out_trap)
{
    uintptr_t retval;

    //sbi_printf("SBI ECALL funcid: %ld\n", funcid);
  
    if (funcid <= FID_RANGE_DEPRECATED) {
        return SBI_ERR_SM_DEPRECATED;
    }

    switch (funcid) {
    case SBI_SM_CREATE_ENCLAVE:
        struct keystone_sbi_create_t create_args_local;
        retval = copy_enclave_create_args((uintptr_t)regs->a0, &create_args_local);
        if (!retval) {
            retval = sbi_sm_create_enclave(out_val, (uintptr_t)&create_args_local);
        }
        break;
    case SBI_SM_DESTROY_ENCLAVE:
        retval = sbi_sm_destroy_enclave(regs->a0);
        break;
    case SBI_SM_ENTER_ENCLAVE:
        retval = sbi_sm_enter_enclave((struct sbi_trap_regs*) regs, regs->a0);
        ((struct sbi_trap_regs *)regs)->mepc += 4;
        sbi_trap_exit(regs);
        break;
    case SBI_SM_RESUME_ENCLAVE:
        retval = sbi_sm_resume_enclave((struct sbi_trap_regs*) regs, regs->a0);
        if (!regs->zero) {
          ((struct sbi_trap_regs *)regs)->a0 = retval;
        }
        ((struct sbi_trap_regs *)regs)->mepc += 4;
        sbi_trap_exit(regs);
        break;
    case SBI_SM_ATTEST_ENCLAVE:
        retval = sbi_sm_attest_enclave(regs->a0, regs->a1, regs->a2);
        break;
    case SBI_SM_RANDOM:
        static uint64_t w = 0, s = 0xb5ad4eceda1ce2a9;
        unsigned long cycles;
        asm volatile ("rdcycle %0" : "=r" (cycles));

        // from Middle Square Weyl Sequence algorithm
        uint64_t x = cycles;
        x *= x;
        x += (w += s);
        *out_val = (x>>32) | (x<<32);
        retval = 0;
        break;
    case SBI_SM_STOP_ENCLAVE:
        retval = sbi_sm_stop_enclave((struct sbi_trap_regs*) regs, regs->a0);
        ((struct sbi_trap_regs *)regs)->a0 = retval;
        ((struct sbi_trap_regs *)regs)->mepc += 4;
        sbi_trap_exit(regs);
        break;
    case SBI_SM_EXIT_ENCLAVE:
        unsigned long prev_reg_a0 = regs->a0;
        retval = sbi_sm_exit_enclave((struct sbi_trap_regs*) regs);
        ((struct sbi_trap_regs *)regs)->a0 = retval;
        ((struct sbi_trap_regs *)regs)->a1 = prev_reg_a0;
        ((struct sbi_trap_regs *)regs)->mepc += 4;
        sbi_trap_exit(regs);
        break;
	case SBI_SM_CREATE_SHM_REGION:
		retval = sbi_sm_create_shm_region(out_val, (uintptr_t)regs->a0, (unsigned long)regs->a1);
		break;
	case SBI_SM_MAP_SHM_REGION:
		retval = sbi_sm_map_shm_region((struct sbi_trap_regs *)regs, (uint32_t)regs->a0);
		break;
	case SBI_SM_UNMAP_SHM_REGION:
		retval = sbi_sm_unmap_shm_region((uint32_t)regs->a0);
		break;
	case SBI_SM_CHANGE_SHM_REGION:
		retval = sbi_sm_change_shm_region(regs->a0, regs->a1);
		break;
	case SBI_SM_SHARE_SHM_REGION:
		retval = sbi_sm_share_shm_region(regs->a0, regs->a1, regs->a2);
		break;
    case SBI_SM_REGISTER_DEV_IRQ:
        retval = sbi_sm_register_dev_irq((uint32_t)regs->a0);
        break;
    case SBI_SM_WAIT_DEV_DATA:
        retval = sbi_sm_wait_dev_data((struct sbi_trap_regs*) regs, (uint32_t)regs->a0);
        ((struct sbi_trap_regs *)regs)->a0 = retval;
        ((struct sbi_trap_regs *)regs)->mepc += 4;
        sbi_trap_exit(regs);
        break;
	default:
		retval = SBI_ERR_SM_NOT_IMPLEMENTED;
		break;
	}

	// sbi_printf("Retval = %lx\n", retval);

    return retval;
}
  
//extern struct sbi_ecall_extension ecall_vyond_monitor;
#define SBI_EXT_EXPERIMENTAL_VYOND_MONITOR 0x08424b45 // BKE (Berkeley Keystone Enclave)
  
struct sbi_ecall_extension ecall_vyond_monitor = {
    .extid_start = SBI_EXT_EXPERIMENTAL_VYOND_MONITOR,
    .extid_end = SBI_EXT_EXPERIMENTAL_VYOND_MONITOR,
    .handle = sbi_ecall_vyond_monitor_handler,
};
  
int sm_init(int cold_boot);

int vyond_monitor_init(int cold_boot)
{
    sbi_printf("Initializing vyond monitor\n");

    int ret = sm_init(cold_boot);
    if (ret != 0) {
        return ret;
    }
    sbi_printf("Finished Initializing vyond monitor\n");
    sbi_ecall_register_extension(&ecall_vyond_monitor);

    return ret;
}

// TODO: This function is externally used by sm-sbi.c.
// Change it to be internal (remove from the enclave.h and make static)
/* Internal function enforcing a copy source is from the untrusted world.
 * Does NOT do verification of dest, assumes caller knows what that is.
 * Dest should be inside the SM memory.
 */
unsigned long copy_enclave_create_args(uintptr_t src, struct keystone_sbi_create_t* dest){

  int region_overlap = copy_to_sm(dest, src, sizeof(struct keystone_sbi_create_t));

  if (region_overlap)
    return SBI_ERR_SM_ENCLAVE_REGION_OVERLAPS;
  else
    return SBI_ERR_SM_ENCLAVE_SUCCESS;
}

/* MPRV wrappers callable from Rust (SM) */
int sm_mprv_read(void *dst, uintptr_t src_va, size_t len) {
  return copy_to_sm(dst, src_va, len);
}

int sm_mprv_write(uintptr_t dst_va, const void *src, size_t len) {
  return copy_from_sm(dst_va, (void*)src, len);
}

static void sbi_trap_error(const char *msg, int rc,
				      ulong mcause, ulong mtval, ulong mtval2,
				      ulong mtinst, struct sbi_trap_regs *regs)
{
	u32 hartid = current_hartid();

	if (misa_extension('H')) {
		sbi_printf("%s: hart%d: mtval2=0x%" PRILX
			   " mtinst=0x%" PRILX "\n",
			   __func__, hartid, mtval2, mtinst);
	}
	sbi_printf("%s: hart%d: mepc=0x%" PRILX " mstatus=0x%" PRILX "\n",
		   __func__, hartid, regs->mepc, regs->mstatus);
	sbi_printf("%s: hart%d: %s=0x%" PRILX " %s=0x%" PRILX "\n", __func__,
		   hartid, "ra", regs->ra, "sp", regs->sp);
	sbi_printf("%s: hart%d: %s=0x%" PRILX " %s=0x%" PRILX "\n", __func__,
		   hartid, "gp", regs->gp, "tp", regs->tp);
	sbi_printf("%s: hart%d: %s=0x%" PRILX " %s=0x%" PRILX "\n", __func__,
		   hartid, "s0", regs->s0, "s1", regs->s1);
	sbi_printf("%s: hart%d: %s=0x%" PRILX " %s=0x%" PRILX "\n", __func__,
		   hartid, "a0", regs->a0, "a1", regs->a1);
	sbi_printf("%s: hart%d: %s=0x%" PRILX " %s=0x%" PRILX "\n", __func__,
		   hartid, "a2", regs->a2, "a3", regs->a3);
	sbi_printf("%s: hart%d: %s=0x%" PRILX " %s=0x%" PRILX "\n", __func__,
		   hartid, "a4", regs->a4, "a5", regs->a5);
	sbi_printf("%s: hart%d: %s=0x%" PRILX " %s=0x%" PRILX "\n", __func__,
		   hartid, "a6", regs->a6, "a7", regs->a7);
	sbi_printf("%s: hart%d: %s=0x%" PRILX " %s=0x%" PRILX "\n", __func__,
		   hartid, "s2", regs->s2, "s3", regs->s3);
	sbi_printf("%s: hart%d: %s=0x%" PRILX " %s=0x%" PRILX "\n", __func__,
		   hartid, "s4", regs->s4, "s5", regs->s5);
	sbi_printf("%s: hart%d: %s=0x%" PRILX " %s=0x%" PRILX "\n", __func__,
		   hartid, "s6", regs->s6, "s7", regs->s7);
	sbi_printf("%s: hart%d: %s=0x%" PRILX " %s=0x%" PRILX "\n", __func__,
		   hartid, "s8", regs->s8, "s9", regs->s9);
	sbi_printf("%s: hart%d: %s=0x%" PRILX " %s=0x%" PRILX "\n", __func__,
		   hartid, "s10", regs->s10, "s11", regs->s11);
	sbi_printf("%s: hart%d: %s=0x%" PRILX " %s=0x%" PRILX "\n", __func__,
		   hartid, "t0", regs->t0, "t1", regs->t1);
	sbi_printf("%s: hart%d: %s=0x%" PRILX " %s=0x%" PRILX "\n", __func__,
		   hartid, "t2", regs->t2, "t3", regs->t3);
	sbi_printf("%s: hart%d: %s=0x%" PRILX " %s=0x%" PRILX "\n", __func__,
		   hartid, "t4", regs->t4, "t5", regs->t5);
	sbi_printf("%s: hart%d: %s=0x%" PRILX "\n", __func__, hartid, "t6",
		   regs->t6);

    sbi_sm_exit_enclave((struct sbi_trap_regs*) regs);
}


/**
 * Handle trap/interrupt
 *
 * This function is called by firmware linked to OpenSBI
 * library for handling trap/interrupt. It expects the
 * following:
 * 1. The 'mscratch' CSR is pointing to sbi_scratch of current HART
 * 2. The 'mcause' CSR is having exception/interrupt cause
 * 3. The 'mtval' CSR is having additional trap information
 * 4. The 'mtval2' CSR is having additional trap information
 * 5. The 'mtinst' CSR is having decoded trap instruction
 * 6. Stack pointer (SP) is setup for current HART
 * 7. Interrupts are disabled in MSTATUS CSR
 *
 * @param regs pointer to register state
 */
void sbi_trap_handler_keystone_enclave(struct sbi_trap_regs *regs)
{
	int rc = SBI_ENOTSUPP;
	const char *msg = "trap handler failed";
	ulong mcause = csr_read(CSR_MCAUSE);
	ulong mtval = csr_read(CSR_MTVAL), mtval2 = 0, mtinst = 0;
	struct sbi_trap_info trap;
	
	//sbi_printf("%s: mcause=0x%" PRILX " mtval=0x%" PRILX "\n",
	//	   __func__, mcause, mtval);

	if (misa_extension('H')) {
		mtval2 = csr_read(CSR_MTVAL2);
		mtinst = csr_read(CSR_MTINST);
	}

	if (mcause & (1UL << (__riscv_xlen - 1))) {
		mcause &= ~(1UL << (__riscv_xlen - 1));
		switch (mcause) {
		    case IRQ_M_TIMER: {
                regs->mepc -= 4;
                sbi_sm_stop_enclave(regs, 0/*STOP_TIMER_INTERRUPT*/);
                ((struct sbi_trap_regs *)regs)->a0 = SBI_ERR_SM_ENCLAVE_INTERRUPTED;
                ((struct sbi_trap_regs *)regs)->mepc += 4;
			    break;
            }
		    case IRQ_M_SOFT: {
                regs->mepc -= 4;
                sbi_sm_stop_enclave((struct sbi_trap_regs*) regs, 0/*STOP_TIMER_INTERRUPT*/);
                ((struct sbi_trap_regs *)regs)->a0 = SBI_ERR_SM_ENCLAVE_INTERRUPTED;
                ((struct sbi_trap_regs *)regs)->mepc += 4;
			    break;
            }
		    case IRQ_M_EXT: {
                /* Device IRQ intercepted in M-mode.
                 * Claim the IRQ from the PLIC M-mode context, check whether
                 * an enclave is waiting for it, and if so resume it directly
                 * without involving the host OS. */
                ulong hartid = current_hartid();
                uint32_t irq = plic_claim_m(hartid);
                if (irq != 0) {
                    sbi_sm_handle_dev_irq((struct sbi_trap_regs*) regs, irq);
                    plic_complete_m(hartid, irq);
                }
                break;
            }
		    default:
			    msg = "unhandled external interrupt";
			goto trap_error;
		};
		return;
	}

	switch (mcause) {
	    case CAUSE_ILLEGAL_INSTRUCTION:
		    rc  = sbi_illegal_insn_handler(mtval, regs);
		    msg = "illegal instruction handler failed";
		break;
	    case CAUSE_MISALIGNED_LOAD:
		    rc = sbi_misaligned_load_handler(mtval, mtval2, mtinst, regs);
		    msg = "misaligned load handler failed";
		break;
	    case CAUSE_MISALIGNED_STORE:
		    rc  = sbi_misaligned_store_handler(mtval, mtval2, mtinst, regs);
		    msg = "misaligned store handler failed";
		break;
	    case CAUSE_SUPERVISOR_ECALL:
	    case CAUSE_MACHINE_ECALL:
		    rc  = sbi_ecall_handler(regs);
		    msg = "ecall handler failed";
		break;
	    case CAUSE_FETCH_ACCESS:
	    case CAUSE_LOAD_ACCESS:
	    case CAUSE_STORE_ACCESS: {
		    uintptr_t eid = sbi_sm_get_enclave_id();
		    /* When the enclave MMU is active (satp SV39 mode), mtval is a
		     * virtual address.  Translate it to PA via a software page-table
		     * walk.  In bare mode satp==0 so mtval is already PA. */
		    uintptr_t satp = csr_read(CSR_SATP);
		    uintptr_t phys_addr;
		    if ((satp >> 60) == 8) {
		        phys_addr = sv39_translate(satp, mtval);
		        if (phys_addr == 0) {
		            sbi_printf("[SM] ACCESS FAULT: eid=%lu va=0x%lx translation failed -> exit\n",
		                       eid, mtval);
		            sbi_sm_exit_enclave((struct sbi_trap_regs*) regs);
		            rc = SBI_OK;
		            break;
		        }
		    } else {
		        phys_addr = mtval;  /* bare mode: mtval is PA */
		    }
		    long ret = sbi_sm_handle_wgc_fault(eid, phys_addr);
		    if (ret == 0) {
		        rc = SBI_OK;
		    } else {
		        sbi_printf("[SM] ACCESS FAULT: eid=%lu phys=0x%lx va=0x%lx not in region -> exit\n",
		                   eid, phys_addr, mtval);
		        sbi_sm_exit_enclave((struct sbi_trap_regs*) regs);
		        rc = SBI_OK;
		    }
		    break;
		}
	    default:
		    /* If the trap came from S or U mode, redirect it there */
		    trap.epc = regs->mepc;
		    trap.cause = mcause;
		    trap.tval = mtval;
		    trap.tval2 = mtval2;
		    trap.tinst = mtinst;
		    rc = sbi_trap_redirect(regs, &trap);
		break;
	};

trap_error:
	if (rc)
		sbi_trap_error(msg, rc, mcause, mtval, mtval2, mtinst, regs);
}
