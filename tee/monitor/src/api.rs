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
            panic!("Failed {:?}", err);
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
pub extern "C" fn sbi_sm_exit_enclave(regs: &mut TrapFrame) -> isize {
    dbg!("[exit_enclave] eid {:?}", cpu::get_enclave_id());
    let ret = match enclave::exit_enclave(regs) {
        Ok(_) => Error::Success,
        Err(err) => {
            dbg!("Failed {:?}", err);
            panic!("Failed {:?}", err);
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

/// Called by enc2 to suspend itself until enc1 calls notify_shm(rid).
#[no_mangle]
pub extern "C" fn sbi_sm_wait_shm(regs: &mut TrapFrame, rid: u32) -> isize {
    let ret = match enclave::wait_shm(regs, rid) {
        Ok(_) => Error::Success,
        Err(err) => err,
    };
    ret as isize
}

/// Called by enc1 after publishing; transitions enc2 from WaitingForShm to Stopped.
/// enc1 continues running; the host resumes enc2 via resume_enclave after enc1 exits.
#[no_mangle]
pub extern "C" fn sbi_sm_notify_shm(rid: u32) -> isize {
    let ret = match enclave::notify_shm(rid) {
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
