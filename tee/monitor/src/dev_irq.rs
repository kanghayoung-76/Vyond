// Device IRQ → Enclave binding table and PLIC M-mode configuration.
//
// When an enclave calls sbi_wait_dev_data(irq), the SM stores the mapping
// irq → eid here and enables the IRQ in the PLIC M-mode context so the SM
// (not host Linux) receives the interrupt.  On IRQ arrival the M-mode trap
// handler calls handle_dev_irq() which looks up the waiting enclave and
// directly resumes it, bypassing the host OS entirely.


pub const INVALID_EID: usize = usize::MAX;
const MAX_DEV_IRQS: usize = 64;

// irq_table[irq_num] = eid of the enclave waiting for that IRQ
static mut IRQ_TABLE: [usize; MAX_DEV_IRQS] = [INVALID_EID; MAX_DEV_IRQS];

// irq_fired[irq_num] = true if IRQ fired before wait_dev_data was called
// (QEMU synchronous DMA: IRQ fires in M-mode handler while enclave is still Running)
static mut IRQ_FIRED: [bool; MAX_DEV_IRQS] = [false; MAX_DEV_IRQS];

/// Mark irq_num as having fired before the enclave entered WaitingForDevice.
pub fn mark_irq_fired(irq_num: u32) {
    if (irq_num as usize) < MAX_DEV_IRQS {
        unsafe { IRQ_FIRED[irq_num as usize] = true; }
    }
}

/// Consume the pre-fired flag for irq_num. Returns true if it was set.
pub fn take_fired_irq(irq_num: u32) -> bool {
    if (irq_num as usize) < MAX_DEV_IRQS {
        let fired = unsafe { IRQ_FIRED[irq_num as usize] };
        if fired {
            unsafe { IRQ_FIRED[irq_num as usize] = false; }
            return true;
        }
    }
    false
}

pub fn register_irq(irq_num: u32, eid: usize) {
    if (irq_num as usize) < MAX_DEV_IRQS {
        unsafe { IRQ_TABLE[irq_num as usize] = eid; }
        plic_enable_m_mode(irq_num);
    }
}

pub fn get_eid_for_irq(irq_num: u32) -> Option<usize> {
    if (irq_num as usize) < MAX_DEV_IRQS {
        let eid = unsafe { IRQ_TABLE[irq_num as usize] };
        if eid != INVALID_EID { Some(eid) } else { None }
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// PLIC MMIO helpers  (QEMU virt machine layout)
// ---------------------------------------------------------------------------
// Context layout: ctx = hartid * 2  (M-mode),  hartid * 2 + 1  (S-mode)
//
// Priority  : PLIC_BASE + irq * 4
// Enable    : PLIC_BASE + 0x2000 + ctx * 0x80  (32 IRQs per word)
// Threshold : PLIC_BASE + 0x20_0000 + ctx * 0x1000
// Claim/Cmpl: PLIC_BASE + 0x20_0004 + ctx * 0x1000

const PLIC_BASE: usize = 0x0c00_0000;

fn plic_priority_addr(irq: u32) -> *mut u32 {
    (PLIC_BASE + irq as usize * 4) as *mut u32
}

fn plic_enable_word_addr(ctx: usize, irq: u32) -> *mut u32 {
    (PLIC_BASE + 0x2000 + ctx * 0x80 + (irq as usize / 32) * 4) as *mut u32
}

fn plic_threshold_addr(ctx: usize) -> *mut u32 {
    (PLIC_BASE + 0x20_0000 + ctx * 0x1000) as *mut u32
}

fn plic_claim_addr(ctx: usize) -> *mut u32 {
    (PLIC_BASE + 0x20_0004 + ctx * 0x1000) as *mut u32
}

fn m_ctx(hartid: usize) -> usize { hartid * 2 }
fn s_ctx(hartid: usize) -> usize { hartid * 2 + 1 }

/// Claim the highest-pending IRQ for this hart's M-mode PLIC context.
/// Returns 0 if no IRQ is pending (spurious).
pub fn plic_claim(hartid: usize) -> u32 {
    unsafe { plic_claim_addr(m_ctx(hartid)).read_volatile() }
}

/// Signal completion of IRQ handling to the PLIC.
pub fn plic_complete(hartid: usize, irq: u32) {
    unsafe { plic_claim_addr(m_ctx(hartid)).write_volatile(irq); }
}

/// Route irq_num to M-mode (SM) and away from S-mode (host Linux).
fn plic_enable_m_mode(irq: u32) {
    let hartid: usize = csr_read!(mhartid);
    let bit = 1u32 << (irq % 32);

    unsafe {
        // Set IRQ priority > 0 so it can be claimed
        plic_priority_addr(irq).write_volatile(1);

        // Enable in M-mode context
        let m = plic_enable_word_addr(m_ctx(hartid), irq);
        m.write_volatile(m.read_volatile() | bit);

        // Set M-mode threshold to 0 (accept everything above priority 0)
        plic_threshold_addr(m_ctx(hartid)).write_volatile(0);

        // Disable in S-mode context so host Linux never sees this IRQ
        let s = plic_enable_word_addr(s_ctx(hartid), irq);
        s.write_volatile(s.read_volatile() & !bit);
    }
}
