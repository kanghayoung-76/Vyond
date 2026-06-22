// SHM inter-enclave IPI: per-hart pending resume table + CLINT MSIP helpers.
//
// When notify_shm targets a different hart, it:
//   1. Stores enc_subscriber's eid in PENDING_RESUME[target_hart]  (Release fence)
//   2. Writes 1 to CLINT MSIP[target_hart] to fire the IPI
//
// The M-mode IPI handler on the target hart (sbi_trap_handler_keystone_enclave,
// IRQ_M_SOFT, host-context path) calls sbi_sm_handle_shm_ipi which calls
// resume_from_shm_ipi:
//   1. Clears CLINT MSIP[this_hart]
//   2. Acquires the pending eid
//   3. Calls switch_to_enclave — Linux context saved, enc_subscriber resumes
//
// CLINT MSIP register layout (QEMU virt / SiFive FU540):
//   base = 0x0200_0000,  MSIP[hartid] = base + hartid * 4  (32-bit, write 1 to send)

use core::sync::atomic::{fence, Ordering};

// CLINT base address (QEMU virt / SiFive FU540 / keystone reference platform)
const CLINT_BASE: usize = 0x0200_0000;
pub const MAX_HARTS: usize = 16;

// Per-hart pending enc_subscriber eid.  None = no pending resume.
static mut PENDING_RESUME: [Option<usize>; MAX_HARTS] = [None; MAX_HARTS];

/// Store eid as the pending resume for the given hart.
/// Uses a Release fence so the eid store is visible before the MSIP write.
pub fn post_resume(hart: usize, eid: usize) {
    if hart < MAX_HARTS {
        unsafe { PENDING_RESUME[hart] = Some(eid); }
        fence(Ordering::Release);
    }
}

/// Return the pending eid for this hart if it matches expected_eid, without consuming.
pub fn peek_pending_resume_for(hart: usize, expected_eid: usize) -> bool {
    fence(Ordering::Acquire);
    if hart < MAX_HARTS {
        unsafe { PENDING_RESUME[hart] == Some(expected_eid) }
    } else {
        false
    }
}

/// Consume and return the pending eid for this hart, if any.
pub fn take_pending_resume(hart: usize) -> Option<usize> {
    fence(Ordering::Acquire);
    if hart < MAX_HARTS {
        unsafe { PENDING_RESUME[hart].take() }
    } else {
        None
    }
}

/// Assert MSIP on the target hart (fire IPI).
pub fn send_msip(hart: usize) {
    unsafe {
        ((CLINT_BASE + hart * 4) as *mut u32).write_volatile(1);
    }
}

/// Deassert MSIP on this hart (clear IPI).
pub fn clear_msip(hart: usize) {
    unsafe {
        ((CLINT_BASE + hart * 4) as *mut u32).write_volatile(0);
    }
}
