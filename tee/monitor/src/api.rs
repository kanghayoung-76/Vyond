use crate::attest;
use crate::cpu;
use crate::dbg;
use crate::enclave;
use crate::trap::TrapFrame;
use crate::Error;

#[no_mangle]
pub extern "C" fn sbi_sm_create_enclave(
    eid: *mut usize,
    create_args: *const enclave::KeystoneSBICreate,
) -> isize {
    dbg!("[create_enclave]");
    let create_args = unsafe { &*create_args };
    let ret = match enclave::create_enclave(create_args) {
        Ok(enclave) => {
            unsafe {
                *eid = enclave.id();
            }
            Error::Success
        }
        Err(err) => {
            dbg!("Failed {:?}", err);
            panic!("Failed {:?}", err);
        }
    };
    ret as isize
}

#[no_mangle]
pub extern "C" fn sbi_sm_destroy_enclave(eid: usize) -> isize {
    dbg!("[destroy_enclave] eid: {:?}", eid);
    let ret = match enclave::destroy_enclave(eid) {
        Ok(_) => Error::Success,
        Err(err) => {
            dbg!("Failed {:?}", err);
            err
        }
    };
    ret as isize
}

#[no_mangle]
pub extern "C" fn sbi_sm_enter_enclave(regs: &mut TrapFrame, eid: usize) -> isize {
    dbg!("[enter_enclave] eid: {:?}", eid);
    let ret = match enclave::enter_enclave(regs, eid) {
        Ok(_) => Error::Success,
        Err(err) => {
            dbg!("Failed {:?}", err);
            panic!("Failed {:?}", err);
        }
    };
    ret as isize
}

#[no_mangle]
pub extern "C" fn sbi_sm_resume_enclave(regs: &mut TrapFrame, eid: usize) -> isize {
    dbg!("[resume_enclave] eid: {:?}", eid);
    let ret = match enclave::resume_enclave(regs, eid) {
        Ok(_) => Error::Success,
        Err(err) => {
            if err != Error::Interrupted && err != Error::EdgeCallHost {
                dbg!("Failed {:?}", err);
                panic!("Failed {:?}", err);
            } else {
                err
            }
        }
    };
    ret as isize
}

#[no_mangle]
pub extern "C" fn sbi_sm_stop_enclave(regs: &mut TrapFrame, request: usize) -> isize {
    let ret = match enclave::stop_enclave(regs, request) {
        Ok(_) => Error::Success,
        Err(err) => {
            if err != Error::Interrupted && err != Error::EdgeCallHost && err != Error::IpiHandled {
                dbg!("Failed {:?}", err);
                panic!("Failed {:?}", err);
            } else {
                err
            }
        }
    };
    ret as isize
}

#[no_mangle]
pub extern "C" fn sbi_sm_exit_enclave(regs: &mut TrapFrame) -> isize {
    dbg!("[exit_enclave] eid {:?}", cpu::get_enclave_id());
    let ret = match enclave::exit_enclave(regs) {
        Ok(_) => Error::Success,
        Err(err) => {
            if err != Error::IpiHandled {
                dbg!("Failed {:?}", err);
                panic!("Failed {:?}", err);
            } else {
                err
            }
        }
    };
    ret as isize
}

#[no_mangle]
pub extern "C" fn sbi_sm_get_enclave_id() -> usize {
    cpu::get_enclave_id()
}

#[no_mangle]
pub extern "C" fn sbi_sm_get_enclave_dram_info(
    eid: usize,
    out_base: *mut usize,
    out_size: *mut usize,
) -> isize {
    match enclave::get_enclave_dram_info(eid) {
        Some((base, size)) => {
            unsafe {
                *out_base = base;
                *out_size = size;
            }
            0
        }
        None => -1,
    }
}

/// Loads the WGC slot for the EPM region that contains fault_addr.
/// Returns 0 if slot loaded (caller should resume at same mepc),
/// returns -1 if not in any registered EPM (caller should exit enclave).
#[no_mangle]
pub extern "C" fn sbi_sm_handle_wgc_fault(eid: usize, fault_addr: usize) -> isize {
    if enclave::load_enclave_slot(eid, fault_addr) { 0 } else { -1 }
}

#[no_mangle]
pub extern "C" fn sbi_sm_attest_enclave(report: usize, data: usize, size: usize) -> isize {
    dbg!("[attest_enclave] eid {:?}", cpu::get_enclave_id());
    let ret = match attest::attest_enclave(report, data, size) {
        Ok(_) => Error::Success,
        Err(err) => {
            dbg!("Failed {:?}", err);
            err
        }
    };
    ret as isize
}

#[no_mangle]
pub extern "C" fn sbi_sm_create_shm_region(rid: *mut usize, pa: usize, size: usize, device_wid: u32) -> isize {
    let ret = match enclave::create_shared_mem(pa, size, device_wid) {
        Ok(id) => {
            unsafe { *rid = id; }
            dbg!("[create_shm_region] pa={:x} size={:?} rid={:?}", pa, size, id);
            Error::Success
        }
        Err(err) => {
            dbg!("Failed {:?}", err);
            panic!("Failed {:?}", err);
        }
    };
    ret as isize
}

#[no_mangle]
pub extern "C" fn sbi_sm_map_shm_region(regs: &mut TrapFrame, rid: usize) -> isize {
    let ret = match enclave::map_shm_region(regs, rid) {
        Ok(_) => Error::Success,
        Err(err) => {
            dbg!("Failed {:?}", err);
            panic!("Failed {:?}", err);
        }
    };
    ret as isize
}

#[no_mangle]
pub extern "C" fn sbi_sm_unmap_shm_region(rid: usize) -> isize {
    let ret = match enclave::unmap_shm_region(rid) {
        Ok(_) => Error::Success,
        Err(err) => {
            dbg!("Failed {:?}", err);
            panic!("Failed {:?}", err);
        }
    };
    ret as isize
}

#[no_mangle]
pub extern "C" fn sbi_sm_change_shm_region(rid: usize, dyn_perm: i8) -> isize {
    let ret = match enclave::change_shm_region(rid, dyn_perm.into()) {
        Ok(_) => Error::Success,
        Err(err) => {
            dbg!("Failed {:?}", err);
            panic!("Failed {:?}", err);
        }
    };
    ret as isize
}

/// Called by the enclave to register which IRQ it wants the SM to intercept.
/// The SM enables irq_num in the PLIC M-mode context (disabling it in S-mode)
/// so host Linux never receives it.
#[no_mangle]
pub extern "C" fn sbi_sm_register_dev_irq(irq_num: u32) -> isize {
    let eid = cpu::get_enclave_id();
    crate::dev_irq::register_irq(irq_num, eid);
    Error::Success as isize
}

/// Called by the enclave to suspend itself until irq_num fires.
/// SM saves enclave context, restores host context (like stop_enclave), and marks
/// the enclave WaitingForDevice so the IRQ handler can resume it without host help.
#[no_mangle]
pub extern "C" fn sbi_sm_wait_dev_data(regs: &mut TrapFrame, irq_num: u32) -> isize {
    dbg!("[wait_dev_data] eid {:?} irq {:?}", cpu::get_enclave_id(), irq_num);
    let ret = match enclave::wait_dev_data(regs, irq_num) {
        Ok(_) => Error::Success,
        Err(err) => err,
    };
    ret as isize
}

/// Called from the M-mode IRQ handler in vyond.c when a device IRQ fires.
/// Finds the enclave waiting for irq_num and switches directly into it.
/// Returns 1 if an enclave was resumed, 0 otherwise.
#[no_mangle]
pub extern "C" fn sbi_sm_handle_dev_irq(regs: &mut TrapFrame, irq_num: u32) -> isize {
    if let Some(eid) = crate::dev_irq::get_eid_for_irq(irq_num) {
        match enclave::resume_from_dev_irq(regs, eid) {
            Ok(_) => 1,
            Err(_) => {
                // IRQ arrived before enclave called wait_dev_data (e.g. QEMU sync DMA).
                // Save the fact so wait_dev_data can detect it and return Ok immediately.
                crate::dev_irq::mark_irq_fired(irq_num);
                0
            }
        }
    } else {
        0
    }
}

/// Returns true (1) if the calling hart is currently in enclave context.
/// Exported for use by the C trap handler (sbi_trap_handler_keystone_enclave)
/// to distinguish enclave-context vs host-waiting-SHM-context interrupts.
#[no_mangle]
pub extern "C" fn sbi_sm_is_enclave_context() -> bool {
    cpu::is_enclave_context()
}

/// Called by host (sub-host) when enc_subscriber is parked at wait_shm.
/// SM blocks in M-mode with WFI until IPI arrives for eid, then switches directly
/// into enc2 — no repeated RESUME_ENCLAVE calls, no SM log spam.
#[no_mangle]
pub extern "C" fn sbi_sm_wait_and_resume(regs: &mut TrapFrame, eid: usize) -> isize {
    let ret = match enclave::wait_and_resume_for_shm(regs, eid) {
        Ok(_) => Error::Success,
        Err(err) => err,
    };
    ret as isize
}

/// Called from the M-mode IRQ_M_SOFT handler when the hart is in host-context
/// (mtvec = trap_vector_enclave, enc_subscriber parked in wait_shm).
/// Clears CLINT MSIP, finds the pending enc_subscriber, and directly switches
/// into it by swapping the interrupted Linux context with enc_subscriber's state.
/// Returns 1 if an enclave was resumed (caller should do a0=0,mepc+=4,sbi_trap_exit),
/// returns 0 if no pending resume (caller should forward as SSIP to Linux).
#[no_mangle]
pub extern "C" fn sbi_sm_handle_shm_ipi(regs: &mut TrapFrame) -> isize {
    if enclave::resume_from_shm_ipi(regs) { 1 } else { 0 }
}

/// Called by enc2 to suspend itself until enc1 calls notify_shm(rid).
#[no_mangle]
pub extern "C" fn sbi_sm_wait_shm(regs: &mut TrapFrame, rid: u32) -> isize {
    let ret = match enclave::wait_shm(regs, rid) {
        Ok(_) => Error::Success,
        Err(err) => err,
    };
    ret as isize
}

/// Called by enc1 after publishing: immediately suspends enc1 and switches to enc2.
/// Returns Interrupted when switch happened (C handler calls sbi_trap_exit for enc2).
/// Returns Success when enc2 not found (OpenSBI returns enc1 normally).
#[no_mangle]
pub extern "C" fn sbi_sm_notify_shm(regs: &mut TrapFrame, rid: u32) -> isize {
    let ret = match enclave::notify_shm(regs, rid) {
        Ok(_) => Error::Success,
        Err(err) => err,
    };
    ret as isize
}

#[no_mangle]
pub extern "C" fn sbi_sm_share_shm_region(rid: usize, eid2share: usize, st_perm: i8) -> isize {
    let ret = match enclave::share_shm_region(rid, eid2share, st_perm.into()) {
        Ok(_) => Error::Success,
        Err(err) => {
            dbg!("Failed {:?}", err);
            panic!("Failed {:?}", err);
        }
    };
    ret as isize
}

// sbi_sm_create_dev_shm: same as sbi_sm_create_shm_region with device_wid != 0.
// Kept as a separate symbol so vyond.c can dispatch SBI_SM_CREATE_DEV_SHM.
#[no_mangle]
pub extern "C" fn sbi_sm_create_dev_shm(rid: *mut usize, pa: usize, size: usize, device_wid: u32) -> isize {
    crate::dbg!("[create_dev_shm] device_wid={}", device_wid);
    sbi_sm_create_shm_region(rid, pa, size, device_wid)
}

/// Creates a shared memory region for enclave-to-enclave communication.
/// Host EID 11 is NOT added to perm_conf, allowing enclaves to verify
/// that the host cannot read/write channel data.
/// Called by host (SBI 4007) before running enclaves.
#[no_mangle]
pub extern "C" fn sbi_sm_create_enclave_shm(rid: *mut usize, pa: usize, size: usize) -> isize {
    let ret = match enclave::create_enclave_shm(pa, size) {
        Ok(id) => {
            unsafe { *rid = id; }
            dbg!("[create_enclave_shm] pa={:x} size={:?} rid={:?}", pa, size, id);
            Error::Success
        }
        Err(err) => {
            dbg!("Failed {:?}", err);
            err
        }
    };
    ret as isize
}

/// Binds hash-based attestation to an enc-enc SHM channel.
/// Called by enc1 (publisher, in enclave context) after the host allocated the SHM.
/// SM records enc1's own hash as creator_hash and stores the caller-provided allowed_hash.
/// SBI call 3011 (enclave-callable).
#[no_mangle]
pub extern "C" fn sbi_sm_register_enc_channel(rid: usize, allowed_hash_pa: usize) -> isize {
    let ret = match enclave::register_enc_channel(rid, allowed_hash_pa) {
        Ok(()) => Error::Success,
        Err(err) => {
            dbg!("sbi_sm_register_enc_channel failed {:?}", err);
            err
        }
    };
    ret as isize
}

/// Locates an enc-enc SHM channel by matching creator_hash and caller's own hash.
/// Called by enc2 (subscriber, in enclave context) to discover the rid without RID_FILE.
/// Writes the rid to rid_out_pa on success.
/// SBI call 3012 (enclave-callable).
#[no_mangle]
pub extern "C" fn sbi_sm_find_shm_by_hash(creator_hash_pa: usize, rid_out_pa: usize) -> isize {
    let ret = match enclave::find_shm_by_hash(creator_hash_pa, rid_out_pa) {
        Ok(()) => Error::Success,
        Err(err) => {
            dbg!("sbi_sm_find_shm_by_hash failed {:?}", err);
            err
        }
    };
    ret as isize
}

/// Returns the EID list for a shared memory region into a caller-provided buffer.
/// The enclave uses this to verify no unexpected EIDs (especially host EID 11)
/// have access to the channel before mapping it.
/// Called by enclave (SBI 4008) after receiving RID from host via OCALL.
#[no_mangle]
pub extern "C" fn sbi_sm_get_shm_eids(
    rid: usize,
    buf_pa: usize,
    max_count: usize,
    out_count: *mut usize,
) -> isize {
    let ret = match enclave::get_shm_eids(rid, buf_pa, max_count) {
        Ok(count) => {
            unsafe { *out_count = count; }
            Error::Success
        }
        Err(err) => {
            dbg!("Failed {:?}", err);
            err
        }
    };
    ret as isize
}

/// Called by enclave to retrieve its own measurement hash (SBI 3013).
/// Writes 64 bytes to hash_out_pa (physical address translated by runtime).
#[no_mangle]
pub extern "C" fn sbi_sm_get_my_hash(hash_out_pa: usize) -> isize {
    let ret = match enclave::get_my_hash(hash_out_pa) {
        Ok(_) => Error::Success,
        Err(err) => {
            dbg!("sbi_sm_get_my_hash failed {:?}", err);
            err
        }
    };
    ret as isize
}

/// Called by enclave to locate a RegionDevEnc whose allowed_hash matches
/// the caller (or is open). Writes rid to rid_out_pa (SBI 3014).
#[no_mangle]
pub extern "C" fn sbi_sm_find_dev_shm(rid_out_pa: usize) -> isize {
    let ret = match enclave::find_dev_shm(rid_out_pa) {
        Ok(_) => Error::Success,
        Err(err) => {
            dbg!("sbi_sm_find_dev_shm failed {:?}", err);
            err
        }
    };
    ret as isize
}

/// Called by enclave to ask SM to write CMD=1 to the device's MMIO register (SBI 3015).
#[no_mangle]
pub extern "C" fn sbi_sm_trigger_dev(device_wid: u32) -> isize {
    let ret = match enclave::trigger_dev(device_wid) {
        Ok(_) => Error::Success,
        Err(err) => {
            dbg!("sbi_sm_trigger_dev failed {:?}", err);
            err
        }
    };
    ret as isize
}
