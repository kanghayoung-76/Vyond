use crate::cpu;
use crate::dbg;
use crate::encoding::*;
use crate::isolator;
use crate::isolator::os_region_id;
#[cfg(any(feature = "isolator_pmp", feature = "isolator_hybrid"))]
use crate::pmp;
use crate::shm;
use crate::spinlock::SpinLock;
use crate::thread;
use crate::trap::TrapFrame;
use crate::Error;
use semihosting::{heprintln, hprint, hprintln};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum State {
    Stopped,
    Running,
    Destroying,
    WaitingForDevice(u32), // suspended; will be resumed by SM on device IRQ
    WaitingForShm(u32),    // suspended; will be resumed by notify_shm(rid)
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RegionType {
    RegionInvalid,
    RegionEPM,
    RegionUTM,
    RegionShared,
    RegionOther,
}

pub struct Region {
    id: usize,
    r_type: RegionType,
    paddr: usize,
    size: usize,
    device_wid: Option<u32>,  // Some(wid) for dev-SHM regions; None otherwise
    perm_conf: shm::RegionPermConfig,
}

/* TODO: does not support multithreaded enclave yet */
pub const MAX_ENCLAVE_THREADS: usize = 1;
pub const MAX_ENCLAVE_REGIONS: usize = 8;

pub struct RunState {
    count: usize,
    state: State,
}

#[repr(C)]
pub struct RuntimeVAParams {
    pub runtime_entry: usize,
    pub user_entry: usize,
    pub untrusted_ptr: usize,
    pub untrusted_size: usize,
    pub num_eapp_pages: usize,
}

#[repr(C)]
pub struct RuntimePAParams {
    pub dram_base: usize,
    pub dram_size: usize,
    pub runtime_base: usize,
    pub user_base: usize,
    pub free_base: usize,
    pub untrusted_base: usize,
    pub untrusted_size: usize,
    pub free_requested: usize,
}

#[repr(C)]
pub struct KeystoneSBIPReigion {
    pub paddr: usize,
    pub size: usize,
}

#[repr(C)]
pub struct KeystoneSBICreate {
    pub epm_region: KeystoneSBIPReigion,
    pub utm_region: KeystoneSBIPReigion,

    pub runtime_paddr: usize,
    pub user_paddr: usize,
    pub free_paddr: usize,
    pub free_requested: usize,
}

pub struct KeystoneSBICreateShm {
    pub paddr: usize,
    pub rid: usize,
    pub size: usize,
}

// enclave metadata
pub struct Enclave {
    eid: usize,                // enclave id
    state: SpinLock<RunState>, // global state of the enclave

    // Physical memory regions associate with this enclave
    regions: [Option<Region>; MAX_ENCLAVE_REGIONS],

    // enclave execution context
    threads: [Option<thread::State>; MAX_ENCLAVE_THREADS],

    pub pa_params: RuntimePAParams,

    // SHA3-512 measurement of enclave memory, computed on first entry
    pub hash: [u8; 64],

    // Last WID successfully used by this enclave (0 = never assigned).
    // Set on every WGC slot assign; read in switch_to_enclave to skip a fault on reuse.
    pub last_wid: usize,
}

impl Enclave {
    const REGION_INIT: Option<Region> = None;
    const THREAD_INIT: Option<thread::State> = None;

    pub fn allocate<'a>(pa_params: RuntimePAParams) -> Result<&'a mut Enclave, Error> {
        for slot in 0..MAX_ENCLAVES {
            if unsafe { ENCLAVES[slot].is_none() } {
                // GC: release WID entries for every inactive slot.
                // If a previous test crashed without calling destroy_enclave it leaves
                // ghost WID entries that block low-numbered slots; sweep them all out
                // so the new enclave gets the lowest available slot (slot=0 → wid=1).
                #[cfg(any(feature = "isolator_wg", feature = "isolator_hybrid"))]
                for dead in 0..MAX_ENCLAVES {
                    if unsafe { ENCLAVES[dead].is_none() } {
                        crate::wid::release_wid_for_eid(dead);
                    }
                }
                unsafe { ENCLAVES[slot] = Some(Enclave::new(slot, pa_params)) };
                return Ok(unsafe { ENCLAVES[slot].as_mut().unwrap() });
            }
        }

        Err(Error::NoFreeResource)
    }

    pub fn new(eid: usize, pa_params: RuntimePAParams) -> Self {
        Enclave {
            eid,
            regions: [Self::REGION_INIT; MAX_ENCLAVE_REGIONS],
            state: SpinLock::new(RunState {
                count: 0,
                state: State::Stopped,
            }),
            threads: [Self::THREAD_INIT; MAX_ENCLAVE_THREADS],
            pa_params,
            hash: [0u8; 64],
            last_wid: crate::wid::ENCLAVE_WID_MIN,
        }
    }

    pub fn compute_hash(&mut self) {
        self.hash = crate::attest::validate_and_hash_enclave(
            self.pa_params.dram_base,
            self.pa_params.runtime_base,
            self.pa_params.user_base,
            self.pa_params.free_base,
        );
    }

    pub fn id(&self) -> usize {
        self.eid
    }

    pub fn free(eid: usize) -> Result<(), Error> {
        unsafe {
            for slot in 0..MAX_ENCLAVES {
                if let Some(ref e) = ENCLAVES[slot] {
                    if e.eid == eid {
                        ENCLAVES[slot] = None;
                        return Ok(());
                    }
                }
            }
        }
        Ok(())
    }

    pub fn switch_to_enclave(&mut self, regs: &mut TrapFrame, load_parameters: bool) {
        /* save host context */
        let thread = &mut self.threads[0].as_mut().unwrap();

        thread.swap_prev_state(regs);
        thread.swap_prev_mepc(regs, regs.mepc);
        thread.swap_prev_mstatus(regs, regs.mstatus);

        let interrupts = 0;
        csr_write!(mideleg, interrupts);

        if load_parameters {
            //csr_write!(sepc, self.params.user_entry);
            regs.mepc = self.pa_params.dram_base - 4; // regs->mepc will be +4 before sbi_ecall_handler return
            regs.mstatus = 1 << crate::encoding::MSTATUS_MPP_SHIFT;
            regs.a1 = self.pa_params.dram_base; // $a1: (PA) DRAM base,
            regs.a2 = self.pa_params.dram_size; // $a2: (PA) DRAM size,
            regs.a3 = self.pa_params.runtime_base; // $a3: (PA) kernel location,
            regs.a4 = self.pa_params.user_base; // $a4: (PA) user location,
            regs.a5 = self.pa_params.free_base; // $a5: (PA) freemem location,
            regs.a6 = self.pa_params.untrusted_base; // $a6: (VA) utm base,
            regs.a7 = self.pa_params.untrusted_size; // $a7: (size_t) utm size

            csr_write!(satp, 0);
        }

        switch_vector_enclave();

        (0..MAX_SHARED_REGIONS).for_each(|memid| {
            unsafe {
                if let Some(region) = get_shm_region_by_idx(memid) {
                    if region.r_type != RegionType::RegionInvalid {
                        if let old_perm = region.perm_conf.get_perm(11 /*untrusted eid*/) {
                            if let new_perm = region.perm_conf.get_perm(self.eid) {

                                // TODO: uncomment below after stablized other flows
                                // if old_perm != new_perm {
                                //     let _ = isolator::set_isolator(region.id, new_perm);
                                // }
                            }
                        }
                    }
                }
            }
        });

        // TODO: Temporarily disabled for new shared memory model
        //#[cfg(any(feature = "isolator_pmp", feature = "isolator_hybrid"))]
        //{
        //    let _ = pmp::set_keystone(os_region_id(), pmp::PMP_NO_PERM);
        //}
        (0..MAX_ENCLAVE_REGIONS).for_each(|memid| {
            if let Some(ref region) = self.regions[memid] {
                if region.r_type == RegionType::RegionEPM {
                    // WGC slot virtualization: EPM slot is NOT loaded eagerly.
                    // The SM ACCESS FAULT handler loads it on-demand.
                } else {
                    let _ = isolator::set_isolator(region.id, false);
                }
            }
        });
        // SHM regions are fault-based (lazy): no eager programming here.

        // Setup any platform specific defenses
        cpu::enter_enclave_context(self.eid, self.last_wid);
    }

    pub fn switch_to_host(&mut self, regs: &mut TrapFrame) {
        (0..MAX_SHARED_REGIONS).for_each(|memid| {
            unsafe {
                if let Some(region) = &SHARED_MEM[memid] {
                    if region.r_type != RegionType::RegionInvalid {
                        if let old_perm = region.perm_conf.get_perm(self.eid) {
                            if let new_perm = region.perm_conf.get_perm(11 /*untrusted eid*/) {
                                // TODO: uncomment after stablized
                                // if old_perm != new_perm {
                                //     let _ = isolator::set_isolator(region.id, new_perm);
                                // }
                            }
                        }
                    }
                }
            }
        });

        // SHM WGC slot is intentionally left active: OS_WID bits remain so
        // the host can read/write the shared buffer during OCALL handling.
        let _ = isolator::set_isolator(os_region_id(), false);

        let interrupts = MIP_SSIP | MIP_STIP | MIP_SEIP;
        csr_write!(mideleg, interrupts);

        let thread = &mut self.threads[0].as_mut().unwrap();

        /* restore host context */
        thread.swap_prev_state(regs);
        thread.swap_prev_mepc(regs, regs.mepc);
        thread.swap_prev_mstatus(regs, regs.mstatus);

        switch_vector_host();

        let pending = csr_read!(mip);

        if (pending & MIP_MTIP) != 0 {
            csr_clear!(mip, MIP_MTIP);
            csr_set!(mip, MIP_STIP);
        }
        if (pending & MIP_MSIP) != 0 {
            csr_clear!(mip, MIP_MSIP);
            csr_set!(mip, MIP_SSIP);
        }
        if (pending & MIP_MEIP) != 0 {
            csr_clear!(mip, MIP_MEIP);
            csr_set!(mip, MIP_SEIP);
        }

        cpu::exit_enclave_context();
    }
}

pub const MAX_ENCLAVES: usize = 16;
pub const MAX_SHARED_REGIONS: usize = 8;

const INIT_VALUE: Option<Enclave> = None;
static mut ENCLAVES: [Option<Enclave>; MAX_ENCLAVES] = [INIT_VALUE; MAX_ENCLAVES];

const INIT_SHM: Option<Region> = None;
static mut SHARED_MEM: [Option<Region>; MAX_SHARED_REGIONS] = [INIT_SHM; MAX_SHARED_REGIONS];

pub fn enclave_exists(enclaves: &[Option<Enclave>], eid: usize) -> bool {
    enclaves.iter().any(|slot| {
        if let Some(ref e) = slot {
            e.eid == eid
        } else {
            false
        }
    })
}

/* This handles creation of a new enclave, based on arguments provided
 * by the untrusted host.
 *
 * This may fail if: it cannot allocate PMP regions, EIDs, etc
 */
pub fn create_enclave<'a>(create_args: &KeystoneSBICreate) -> Result<&'a Enclave, Error> {
    let pa_params = RuntimePAParams {
        dram_base: create_args.epm_region.paddr,
        dram_size: create_args.epm_region.size,
        runtime_base: create_args.runtime_paddr,
        user_base: create_args.user_paddr,
        free_base: create_args.free_paddr,
        untrusted_base: create_args.utm_region.paddr,
        untrusted_size: create_args.utm_region.size,
        free_requested: create_args.free_requested,
    };
    let enclave: &mut Enclave = Enclave::allocate(pa_params)?;

    // TODO: Check if create_args is valid

    // create a PMP region bound to the enclave
    if let Ok(region) = isolator::region_init(
        create_args.epm_region.paddr,
        create_args.epm_region.size,
        enclave.id(),
        false,
    ) {
        //hprintln!("Found unused pmp slot: {}", region);
        enclave.regions[0] = Some(Region {
            id: region,
            r_type: RegionType::RegionEPM,
            paddr: create_args.epm_region.paddr,
            size: create_args.epm_region.size,
            device_wid: None,
            perm_conf: shm::RegionPermConfig {
                owner_id: enclave.id(),
                conf_list: [None; shm::MAX_SHM_SHARERS],
            },
        });

        enclave.threads[0] = Some(thread::State::new(
            create_args.epm_region.paddr - 4,
            (1 << crate::encoding::MSTATUS_MPP_SHIFT) | crate::encoding::MSTATUS_FS,
        ));

        // Create a host-enclave shared memory region for OCALL communication.
        // The physical memory (formerly "utm_region") is registered as a SHM
        // owned by the host (EID 11) and shared with this enclave.
        // The WGC slot starts host-only; enclave bits are added on first fault.
        if create_args.utm_region.size > 0 {
            match create_shared_mem(
                create_args.utm_region.paddr,
                create_args.utm_region.size,
                0, // host-SHM (UTM for ocall)
            ) {
                Ok(rid) => {
                    let _ = share_shm_region(rid, enclave.id(), shm::Perm::FULL);
                }
                Err(err) => {
                    heprintln!("[create_enclave] failed to create SHM for ocall: {:?}", err);
                }
            }
        }

        return Ok(enclave);
    }

    Err(Error::NoFreeResource)
}

pub fn find_enclave<'a>(eid: usize) -> Option<&'a mut Enclave> {
    unsafe {
        for slot in 0..MAX_ENCLAVES {
            if let Some(ref mut enclave) = ENCLAVES[slot] {
                if enclave.eid == eid {
                    return Some(enclave);
                }
            }
        }
    }
    None
}

/// Returns (dram_base, dram_size) for the given EID, or None if not found.
pub fn get_enclave_dram_info(eid: usize) -> Option<(usize, usize)> {
    find_enclave(eid).map(|e| (e.pa_params.dram_base, e.pa_params.dram_size))
}

/// Loads the WGC slot for the EPM region containing fault_addr.
/// Returns true if slot was loaded (resume), false if not found (exit enclave).
pub fn load_enclave_slot(eid: usize, fault_addr: usize) -> bool {
    if let Some(enclave) = find_enclave(eid) {
        let dram_base = enclave.pa_params.dram_base;
        let dram_size = enclave.pa_params.dram_size;
        if fault_addr >= dram_base && fault_addr < dram_base + dram_size {
            for memid in 0..MAX_ENCLAVE_REGIONS {
                if let Some(ref region) = enclave.regions[memid] {
                    if region.r_type == RegionType::RegionEPM {
                        let region_id = region.id;
                        #[cfg(any(feature = "isolator_wg", feature = "isolator_hybrid"))]
                        {
                            let (wid, action) = crate::wid::assign_wid(eid, region_id);
                            match action {
                                crate::wid::WIDAction::Assigned { slot } =>
                                    heprintln!("[WGC:EPM] eid={} fault=0x{:x} | ASSIGN slot={} WID={}", eid, fault_addr, slot, wid),
                                crate::wid::WIDAction::Evicted { slot, evicted_eid } =>
                                    heprintln!("[WGC:EPM] eid={} fault=0x{:x} | EVICT  slot={} WID={} (was eid={}) -> new eid={}", eid, fault_addr, slot, wid, evicted_eid, eid),
                            }
                            enclave.last_wid = wid;
                            let _ = isolator::set_isolator_with_wid(region_id, wid);
                            csr_write_custom!(0x390, wid);
                        }
                        return true;
                    }
                }
            }
        }
    }

    // Check global SHM table: fault may be on a shared memory region this enclave
    // has been granted access to.  Load the WGC slot with OS_WID | enclave_wid so
    // both host and enclave can reach it.
    for memid in 0..MAX_SHARED_REGIONS {
        unsafe {
            if let Some(ref region) = SHARED_MEM[memid] {
                if fault_addr >= region.paddr
                    && fault_addr < region.paddr + region.size
                    && region.perm_conf.get_perm(eid).is_some()
                {
                    #[cfg(any(feature = "isolator_wg", feature = "isolator_hybrid"))]
                    {
                        // Compute perm bitmap from current sharers.
                        // For dev-SHM (device_wid.is_some()):  skip eid=11 (host) to keep OS_WID
                        //   out of the hardware slot; only device_wid + enclave_wid are granted.
                        // For host-enclave SHM:  eid=11 → OS_WID is included normally.
                        let is_dev_shm = region.device_wid.is_some();
                        #[derive(Copy, Clone)]
                        struct PermEntry { wid: usize, eid: usize }
                        let mut entries = [PermEntry { wid: 0, eid: 0 }; 8];
                        let mut n = 0usize;
                        let mut perm: u64 = 0;
                        for conf_opt in region.perm_conf.conf_list.iter() {
                            if let Some(c) = conf_opt {
                                if c.eid == 11 {
                                    if is_dev_shm { continue; } // host must not access dev-SHM
                                    let w = crate::wg::OS_WID as usize;
                                    perm |= 3u64 << (w as u64 * 2);
                                    if n < 8 { entries[n] = PermEntry { wid: w, eid: c.eid }; n += 1; }
                                } else {
                                    let w = match find_enclave(c.eid) {
                                        Some(e) => e.last_wid,
                                        None    => continue,
                                    };
                                    perm |= 3u64 << (w as u64 * 2);
                                    if n < 8 { entries[n] = PermEntry { wid: w, eid: c.eid }; n += 1; }
                                }
                            }
                        }
                        // Add device_wid to perm bitmap for dev-SHM
                        if let Some(dwid) = region.device_wid {
                            perm |= 3u64 << (dwid as u64 * 2);
                        }
                        hprint!("[WGC:SHM] eid={} fault=0x{:x} size=0x{:x} | LOAD",
                            eid, fault_addr, region.size);
                        for i in 0..n {
                            let e = &entries[i];
                            if i == 0 { hprint!(" "); } else { hprint!("+"); }
                            if e.eid == 11 { hprint!("WID={}(OS)", e.wid); }
                            else           { hprint!("WID={}(eid={})", e.wid, e.eid); }
                        }
                        if let Some(dwid) = region.device_wid {
                            hprint!("+WID={}(dev)", dwid);
                        }
                        hprintln!("");
                        let _ = isolator::set_shm_perm(region.id, perm);
                    }
                    return true;
                }
            }
        }
    }

    false
}

/*
* Fully destroys an enclave
* Deallocates EID, clears epm, etc
* Fails only if the enclave isn't running.
*/
pub fn destroy_enclave(eid: usize) -> Result<(), Error> {
    if let Some(enclave) = find_enclave(eid) {
        let mut runstate = enclave.state.lock();
        let destroyable = runstate.state != State::Running && runstate.count == 0;

        /* update the enclave state first so that
         * no SM can run the enclave any longer */
        if destroyable {
            runstate.state = State::Destroying;
        }

        drop(runstate);

        if !destroyable {
            return Err(Error::NotDestroyable);
        }

        for i in 0..MAX_SHARED_REGIONS {
            unsafe {
                let should_free = if let Some(region) = &SHARED_MEM[i] {
                    let owner_match = region.r_type != RegionType::RegionInvalid
                        && region.perm_conf.owner_id == eid;
                    // UTM SHM: host-owned (11), host+enclave both in conf_list
                    let utm_match = region.r_type != RegionType::RegionInvalid
                        && region.perm_conf.owner_id == 11
                        && region.perm_conf.get_perm(11).is_some()
                        && region.perm_conf.get_perm(eid).is_some();
                    if owner_match || utm_match { Some(region.id) } else { None }
                } else {
                    None
                };
                if let Some(region_id) = should_free {
                    let _ = isolator::set_isolator(region_id, true);
                    let _ = isolator::region_free(region_id);
                    SHARED_MEM[i] = None;
                }
            }
        }

        // 1. clear all the data in the enclave pages
        // requires no lock (single runner)
        for i in 0..MAX_ENCLAVE_REGIONS {
            if let Some(region) = &enclave.regions[i] {
                if region.r_type == RegionType::RegionInvalid {
                    continue;
                }
                let rid = region.id;
                let _ = isolator::set_isolator(rid, true);
                let _ = isolator::region_free(rid);
            }
        }

        (0..MAX_ENCLAVE_REGIONS).for_each(|idx| {
            enclave.regions[idx] = None;
        });

        // 2. Release WID slot assignment for this enclave
        #[cfg(any(feature = "isolator_wg", feature = "isolator_hybrid"))]
        crate::wid::release_wid_for_eid(eid);

        // 3. release eid
        let _ = Enclave::free(eid);

        return Ok(());
    }

    Err(Error::InvalidId)
}

pub fn enter_enclave(tf: &mut TrapFrame, eid: usize) -> Result<(), Error> {
    if let Some(enclave) = find_enclave(eid) {
        let mut runstate = enclave.state.lock();
        let runnable = runstate.state == State::Running || runstate.state == State::Stopped;

        if runnable {
            runstate.count += 1;
            runstate.state = State::Running;
        }

        drop(runstate);

        if !runnable {
            return Err(Error::NotRunnable);
        }

        enclave.compute_hash();
        enclave.switch_to_enclave(tf, true);

        return Ok(());
    }

    Err(Error::Invalid)
}

pub fn resume_enclave(tf: &mut TrapFrame, eid: usize) -> Result<(), Error> {
    if cpu::is_enclave_context() {
        return Err(Error::Invalid);
    }
    if let Some(enclave) = find_enclave(eid) {
        let mut runstate = enclave.state.lock();
        let resumable = (runstate.state == State::Running
            || runstate.state == State::Stopped
            || matches!(runstate.state, State::WaitingForDevice(_))
            || matches!(runstate.state, State::WaitingForShm(_)))
            && runstate.count < MAX_ENCLAVE_THREADS;

        if resumable {
            runstate.count += 1;
            runstate.state = State::Running;
        }

        drop(runstate);

        if !resumable {
            return Err(Error::NotResumable);
        }

        enclave.switch_to_enclave(tf, false);

        return Ok(());
    }

    return Err(Error::InvalidId);
}

pub fn stop_enclave(tf: &mut TrapFrame, request: usize) -> Result<(), Error> {
    if let Some(enclave) = find_enclave(cpu::get_enclave_id()) {
        let mut runstate = enclave.state.lock();
        let runnable = runstate.state == State::Running;

        if runnable {
            runstate.count -= 1;
            if runstate.count == 0 {
                runstate.state = State::Stopped;
            }
        }

        drop(runstate);

        if !runnable {
            return Err(Error::NotRunning);
        }

        enclave.switch_to_host(tf);

        let ret = match request {
            0/*StopReason::TimerInterrupt*/ => Err(Error::Interrupted),
            1/*StopReason::EdgeCallHost*/ => Err(Error::EdgeCallHost),
            _ => Err(Error::Unknown),
        };
        return ret;
    }

    return Err(Error::Invalid);
}

/// Enclave suspends itself waiting for a device IRQ.
/// Behaves like stop_enclave but transitions to WaitingForDevice instead of Stopped.
/// The SM will resume this enclave directly when irq_num fires (no host involvement).
pub fn wait_dev_data(tf: &mut TrapFrame, irq_num: u32) -> Result<(), Error> {
    if let Some(_enclave) = find_enclave(cpu::get_enclave_id()) {
        // Fast-path A: IRQ fired before we got here (QEMU sync DMA — M-mode IRQ handler
        // ran while enclave was still Running and saved the pre-fired flag).
        if crate::dev_irq::take_fired_irq(irq_num) {
            return Ok(());
        }

        // Fast-path B: IRQ still pending in PLIC (rare: M-mode interrupts masked briefly).
        let hartid = csr_read!(mhartid) as usize;
        let claimed = crate::dev_irq::plic_claim(hartid);
        if claimed == irq_num {
            crate::dev_irq::plic_complete(hartid, irq_num);
            return Ok(());
        }
        if claimed != 0 {
            crate::dev_irq::plic_complete(hartid, claimed);
        }

        // Slow-path: IRQ not yet pending — suspend enclave, let host run,
        // IRQ_M_EXT handler will call resume_from_dev_irq when IRQ fires.
        let enclave = find_enclave(cpu::get_enclave_id()).ok_or(Error::Invalid)?;
        let mut runstate = enclave.state.lock();
        if runstate.state != State::Running {
            return Err(Error::NotRunning);
        }
        runstate.count -= 1;
        runstate.state = State::WaitingForDevice(irq_num);
        drop(runstate);
        enclave.switch_to_host(tf);
        return Err(Error::WaitingForDevice);
    }
    Err(Error::Invalid)
}

/// Called by the M-mode IRQ handler to resume an enclave that called wait_dev_data.
/// regs currently holds the preempted host context; switch_to_enclave swaps it
/// so mret lands in the enclave.
pub fn resume_from_dev_irq(tf: &mut TrapFrame, eid: usize) -> Result<(), Error> {
    if let Some(enclave) = find_enclave(eid) {
        let mut runstate = enclave.state.lock();
        let resumable = matches!(runstate.state, State::WaitingForDevice(_))
            && runstate.count < MAX_ENCLAVE_THREADS;
        if resumable {
            runstate.count += 1;
            runstate.state = State::Running;
        }
        drop(runstate);
        if !resumable {
            return Err(Error::NotResumable);
        }
        enclave.switch_to_enclave(tf, false);
        return Ok(());
    }
    Err(Error::InvalidId)
}

/// Called by an enclave to suspend itself until another enclave calls notify_shm(rid).
/// Mirrors wait_dev_data but uses WaitingForShm(rid) state instead of WaitingForDevice.
pub fn wait_shm(tf: &mut TrapFrame, rid: u32) -> Result<(), Error> {
    if let Some(enclave) = find_enclave(cpu::get_enclave_id()) {
        let mut runstate = enclave.state.lock();
        if runstate.state != State::Running {
            return Err(Error::NotRunning);
        }
        runstate.count -= 1;
        runstate.state = State::WaitingForShm(rid);
        drop(runstate);
        enclave.switch_to_host(tf);
        return Err(Error::WaitingForShm);
    }
    Err(Error::Invalid)
}

/// Called by an enclave (publisher) to mark the subscriber enclave as ready to resume.
/// Transitions the WaitingForShm(rid) enclave to Stopped so the host can resume it.
/// The calling enclave (enc1) continues running; the host resumes enc2 after enc1 exits.
pub fn notify_shm(rid: u32) -> Result<(), Error> {
    unsafe {
        for slot in 0..MAX_ENCLAVES {
            if let Some(ref enc2) = ENCLAVES[slot] {
                let mut rs2 = enc2.state.lock();
                if matches!(rs2.state, State::WaitingForShm(r) if r == rid) {
                    // Mark enc2 as Stopped (count stays 0); host will call resume_enclave.
                    rs2.state = State::Stopped;
                    return Ok(());
                }
            }
        }
    }
    Err(Error::Invalid)
}

pub fn exit_enclave(tf: &mut TrapFrame) -> Result<(), Error> {
    if let Some(enclave) = find_enclave(cpu::get_enclave_id()) {
        let mut runstate = enclave.state.lock();
        let runnable = runstate.state == State::Running && runstate.count > 0;

        if runnable {
            runstate.count -= 1;
        }

        if runstate.count == 0 {
            runstate.state = State::Stopped;
        }

        drop(runstate);

        enclave.switch_to_host(tf);

        return Ok(());
    }

    Err(Error::Invalid)
}

fn switch_vector_enclave() {
    csr_write!(mtvec, trap_vector_enclave);
}

fn switch_vector_host() {
    csr_write!(mtvec, _trap_handler);
}

extern "C" {
    fn trap_vector_enclave();
}
extern "C" {
    fn _trap_handler();
}

/// Creates a shared memory owned by a given enclve
///
/// # Arguments
///
/// * `eid` - The enclave id
/// * `paddr` - The base physical address of the shared memory
/// * `size` - The size of the shared memory
///
/// # Returns
/// A `usize` value that identification for the created sharhed memory
///
///
//pub fn create_shared_mem(eid: usize, paddr: usize, size: usize) -> Result<usize, Error> {
// device_wid: 0 = host-SHM (eager OS_WID slot), non-zero = dev-SHM (lazy, device_wid | enclave_wid)
pub fn create_shared_mem(paddr: usize, size: usize, device_wid: u32) -> Result<usize, Error> {
    if let Ok(region_idx) = isolator::region_init(paddr, size, 11, true) {
        // Eagerly load WGC slot for host-SHM only; dev-SHM is lazy (loaded on first enclave fault).
        #[cfg(any(feature = "isolator_wg", feature = "isolator_hybrid"))]
        if device_wid == 0 {
            let _ = isolator::set_shm_host_only(region_idx);
        }

        for i in 0..MAX_SHARED_REGIONS {
            if unsafe { SHARED_MEM[i].is_none() } {
                unsafe {
                    let mut region = Region {
                        id: region_idx,
                        r_type: RegionType::RegionShared,
                        paddr,
                        size,
                        device_wid: if device_wid != 0 { Some(device_wid) } else { None },
                        perm_conf: shm::RegionPermConfig {
                            owner_id: 11,
                            conf_list: [None; shm::MAX_SHM_SHARERS],
                        },
                    };
                    region.perm_conf.insert_perm(shm::PermConfig {
                        eid: 11,
                        dyn_perm: shm::Perm::FULL,
                        st_perm: shm::Perm::FULL,
                        maps: 0,
                    });
                    SHARED_MEM[i] = Some(region);
                }
                return Ok(region_idx);
            }
        }
    }
    Err(Error::Invalid)
}

pub fn map_shm_region(regs: &mut TrapFrame, rid: usize) -> Result<(), Error> {
    let caller_eid = if cpu::is_enclave_context() {
        cpu::get_enclave_id()
    } else {
        11
    };
    unsafe {
        if let Some(region) = get_shm_region_by_rid(rid) {
            regs.a2 = region.paddr;
            regs.a3 = region.size;
            if let Some(perm) = region.perm_conf.get_perm_mut(caller_eid) {
                perm.increment_map();
                return Ok(());
            }
            // TDDS enclave-only channel: host EID (11) absent from conf_list.
            // Any enclave may map it — auto-grant on first access.
            if cpu::is_enclave_context() && region.perm_conf.get_perm(11).is_none() {
                region.perm_conf.insert_perm(shm::PermConfig {
                    eid: caller_eid,
                    dyn_perm: shm::Perm::FULL,
                    st_perm: shm::Perm::FULL,
                    maps: 1,
                });
                return Ok(());
            }
            dbg!("[map_shm_region] DENIED rid={} caller_eid={} (no perm entry)", rid, caller_eid);
        } else {
            dbg!("[map_shm_region] rid={} NOT FOUND in SHARED_MEM", rid);
        }
    }
    Err(Error::Invalid)
}

pub fn unmap_shm_region(rid: usize) -> Result<(), Error> {
    let caller_eid = if cpu::is_enclave_context() {
        cpu::get_enclave_id()
    } else {
        11
    };
    unsafe {
        if let Some(region) = get_shm_region_by_rid(rid) {
            if let Some(perm) = region.perm_conf.get_perm_mut(caller_eid) {
                perm.decrement_map();
                return Ok(());
            }
            dbg!("Not found perm info for eid {:?}", caller_eid);
        }
        dbg!("Not found region for rid {:?}", rid);
    }
    Err(Error::Invalid)
}

pub fn change_shm_region(rid: usize, dyn_perm: shm::Perm) -> Result<(), Error> {
    if let Some(region) = get_shm_region_by_rid(rid) {
        if cpu::is_enclave_context() {
            if let Some(enclave) = find_enclave(cpu::get_enclave_id()) {
                if let Some(perm) = region.perm_conf.get_perm_mut(enclave.id()) {
                    if perm.update_dyn_perm(dyn_perm) {
                        //display();
                        return Ok(());
                    }
                    dbg!(
                        "[change_shm_region] failed to update dynamic perm {:?} for rid {:?}",
                        dyn_perm,
                        rid
                    );
                }
                dbg!(
                    "[change_shm_region] perm {:?} not found for rid {:?}",
                    dyn_perm,
                    rid
                );
            }
            dbg!(
                "[change_shm_region] enclave not found for eid {:?} rid {:?}",
                cpu::get_enclave_id(),
                rid
            );
        } else {
            if let Some(perm) = region.perm_conf.get_perm_mut(11 /*host */) {
                if perm.update_dyn_perm(dyn_perm) {
                    //display();
                    return Ok(());
                }
                dbg!(
                    "[change_shm_region] failed to update dynamic perm {:?} for rid {:?}",
                    dyn_perm,
                    rid
                );
            }
            dbg!(
                "[change_shm_region] perm {:?} not found for rid {:?}",
                dyn_perm,
                rid
            );
        }
    }
    dbg!("[change_shm_region] region not found for rid {:?}", rid);
    //display();
    Err(Error::InvalidId)
}

pub fn share_shm_region(rid: usize, eid2share: usize, st_perm: shm::Perm) -> Result<(), Error> {
    let owner_eid = if cpu::is_enclave_context() {
        cpu::get_enclave_id()
    } else {
        11 /*untrusted host */
    };
    if let Some(region) = get_shm_region_by_rid(rid) {
        if region.perm_conf.owner_id == owner_eid {
            if region.perm_conf.insert_perm(shm::PermConfig {
                eid: eid2share,
                dyn_perm: shm::Perm::NULL,
                st_perm,
                maps: 0,
            }) == false
            {
                dbg!("[share_shm_region] conf_list is full");
                return Err(Error::Invalid);
            }
        } else {
            dbg!(
                "[share_shm_region] eid {} is not owner of rid {}",
                owner_eid,
                rid
            );
            return Err(Error::Invalid);
        }
    } else {
        dbg!("[share_shm_region] region rid {:?} not found", rid);
        return Err(Error::Invalid);
    }
    //display();
    Ok(())
}

/// Creates a shared memory region for enclave-to-enclave communication.
/// Unlike create_shared_mem, this does NOT add host EID 11 to perm_conf,
/// so enclaves can verify that the host has no access to the channel.
pub fn create_enclave_shm(paddr: usize, size: usize) -> Result<usize, Error> {
    if let Ok(region_idx) = isolator::region_init(paddr, size, 11, true) {
        for i in 0..MAX_SHARED_REGIONS {
            if unsafe { SHARED_MEM[i].is_none() } {
                unsafe {
                    SHARED_MEM[i] = Some(Region {
                        id: region_idx,
                        r_type: RegionType::RegionShared,
                        paddr,
                        size,
                        device_wid: None,
                        perm_conf: shm::RegionPermConfig {
                            owner_id: 11,
                            conf_list: [None; shm::MAX_SHM_SHARERS],
                        },
                    });
                }
                dbg!("[create_enclave_shm] pa={:x} size={:?} rid={:?}", paddr, size, region_idx);
                return Ok(region_idx);
            }
        }
    }
    Err(Error::Invalid)
}

/// Returns the list of EIDs that have access to the given SHM region.
/// Writes up to max_count EIDs to the physical address buf_pa.
/// Returns the number of EIDs written, or an error.
/// Called by enclave to verify channel membership before mapping.
pub fn get_shm_eids(rid: usize, buf_pa: usize, max_count: usize) -> Result<usize, Error> {
    if let Some(region) = get_shm_region_by_rid(rid) {
        let mut count = 0usize;
        let buf = buf_pa as *mut usize;
        for conf in region.perm_conf.conf_list.iter() {
            if count >= max_count {
                break;
            }
            if let Some(cfg) = conf {
                unsafe { *buf.add(count) = cfg.eid; }
                count += 1;
            }
        }
        Ok(count)
    } else {
        Err(Error::Invalid)
    }
}

pub fn get_shm_region_by_rid(rid: usize) -> Option<&'static mut Region> {
    unsafe {
        for i in 0..MAX_SHARED_REGIONS {
            if let Some(region) = &SHARED_MEM[i] {
                if region.id == rid {
                    return SHARED_MEM[i].as_mut();
                }
            }
        }
    }
    None
}

pub fn get_shm_region_by_idx(idx: usize) -> Option<&'static mut Region> {
    unsafe {
        if idx < MAX_SHARED_REGIONS {
            SHARED_MEM[idx].as_mut()
        } else {
            None
        }
    }
}

pub fn remove_region_by_idx(idx: usize) -> bool {
    unsafe {
        if idx < MAX_SHARED_REGIONS {
            SHARED_MEM[idx] = None;
            return true;
        }
    }
    false
}

pub fn display() {
    hprintln!("Display Enclaves");
    for slot in 0..MAX_ENCLAVES {
        if let Some(enclave) = unsafe { ENCLAVES[slot].as_mut() } {
            hprintln!("+--------+--------+----------------+----------------+----------------+----------------+----------------+----------------+----------------+--------+");
            hprintln!("|                                                                 Enclave                                                                         |");
            hprintln!("+--------+--------+----------------+----------------+----------------+----------------+----------------+----------------+----------------+--------+");
            hprintln!("|  eid   |  state |                                              pa_params                                                               |#regions|");
            hprintln!("+--------+--------+----------------+----------------+----------------+----------------+----------------+----------------+----------------+--------+");
            hprintln!("|        |        |   dram_base    |  dram_size     |  user_base     |  free_base     |   ut_base      |  ut_size       |   free_req     |        |");
            hprintln!("+--------+--------+----------------+----------------+----------------+----------------+----------------+----------------+----------------+--------+");
            let runstate = enclave.state.lock();
            hprint!("|{:>8}", enclave.eid);
            let state_id: u8 = match runstate.state {
                State::Stopped => 0,
                State::Running => 1,
                State::Destroying => 2,
                State::WaitingForDevice(_) => 3,
                State::WaitingForShm(_) => 4,
            };
            hprint!("|{:>8}", state_id);
            hprint!("|{:>16x}", enclave.pa_params.dram_base);
            hprint!("|{:>16x}", enclave.pa_params.dram_size);
            hprint!("|{:>16x}", enclave.pa_params.user_base);
            hprint!("|{:>16x}", enclave.pa_params.free_base);
            hprint!("|{:>16x}", enclave.pa_params.untrusted_base);
            hprint!("|{:>16x}", enclave.pa_params.untrusted_size);
            hprint!("|{:>16x}", enclave.pa_params.free_requested);

            let mut region_cnt = 0;
            for rid in 0..MAX_ENCLAVE_REGIONS {
                if let Some(_) = &enclave.regions[rid] {
                    region_cnt += 1;
                }
            }
            hprintln!("|{:>8}|", region_cnt);
            hprintln!("+--------+--------+----------------+----------------+----------------+----------------+----------------+----------------+----------------+--------+");
            drop(runstate);

            for rid in 0..MAX_ENCLAVE_REGIONS {
                if let Some(region) = &enclave.regions[rid] {
                    hprintln!("");
                    hprintln!(
                        "+--------+--------+----------------+----------------+----------------+----------------+"
                    );
                    hprintln!(
                        "|                                           Region                                    |"
                    );
                    hprintln!(
                        "+--------+--------+----------------+----------------+----------------+----------------+"
                    );
                    hprintln!(
                        "|regionid|  type  |      paddr     |      size      |             perm_conf           |"
                    );
                    hprintln!(
                        "+--------+--------+----------------+----------------+----------------+----------------+"
                    );
                    hprintln!(
                        "|--------|--------|----------------|----------------|   owner_id     |   #confs       |"
                    );
                    hprintln!(
                        "+--------+--------+----------------+----------------+----------------+----------------+"
                    );
                    break;
                }
            }

            for rid in 0..MAX_ENCLAVE_REGIONS {
                if let Some(region) = &enclave.regions[rid] {
                    hprint!("|{:>8}", region.id);
                    hprint!("|{:>8?}", region.r_type as u8);
                    hprint!("|{:>16x}", region.paddr);
                    hprint!("|{:>16x}", region.size);
                    hprint!("|{:>16}", region.perm_conf.owner_id);
                    let mut cfg_cnt = 0;
                    for cid in 0..shm::MAX_SHM_SHARERS {
                        if let Some(_) = region.perm_conf.conf_list[cid] {
                            cfg_cnt += 1;
                        }
                    }
                    hprintln!("|{:>16}|", cfg_cnt);
                    hprintln!(
                        "+--------+--------+----------------+----------------+----------------+----------------+"
                    );

                    for cid in 0..shm::MAX_SHM_SHARERS {
                        if let Some(conf) = region.perm_conf.conf_list[cid] {
                            hprintln!("");
                            hprintln!("+--------+--------+--------+");
                            hprintln!("|         PermConf         |");
                            hprintln!("+--------+--------+--------+");
                            hprintln!("|  eid   |st_perm |dyn_perm|");
                            hprintln!("+--------+--------+--------+");
                            break;
                        }
                    }

                    for cid in 0..shm::MAX_SHM_SHARERS {
                        if let Some(conf) = region.perm_conf.conf_list[cid] {
                            hprint!("|{:>8}", conf.eid);
                            hprint!("|{:>8x}", conf.st_perm);
                            hprintln!("|{:>8x}|", conf.dyn_perm);
                            hprintln!("+--------+--------+--------+");
                        }
                    }
                }
            }
        }
    }

    for rid in 0..MAX_SHARED_REGIONS {
        unsafe {
            if let Some(_) = &SHARED_MEM[rid] {
                hprintln!("");
                hprintln!(
                        "+--------+--------+----------------+----------------+----------------+----------------+"
                    );
                hprintln!(
                        "+                                    Shared Region                                    +"
                    );
                hprintln!(
                        "+--------+--------+----------------+----------------+----------------+----------------+"
                    );
                hprintln!(
                        "|regionid|  type  |      paddr     |      size      |             perm_conf           |"
                    );
                hprintln!(
                        "+--------+--------+----------------+----------------+----------------+----------------+"
                    );
                hprintln!(
                        "|--------|--------|----------------|----------------|   owner_id     |   #confs       |"
                    );
                hprintln!(
                        "+--------+--------+----------------+----------------+----------------+----------------+"
                    );
                break;
            }
        }
    }

    for rid in 0..MAX_SHARED_REGIONS {
        unsafe {
            if let Some(region) = &SHARED_MEM[rid] {
                hprint!("|{:>8}", region.id);
                hprint!("|{:>8?}", region.r_type as u8);
                hprint!("|{:>16x}", region.paddr);
                hprint!("|{:>16x}", region.size);
                hprint!("|{:>16}", region.perm_conf.owner_id);
                let mut cfg_cnt = 0;
                for cid in 0..shm::MAX_SHM_SHARERS {
                    if let Some(_) = region.perm_conf.conf_list[cid] {
                        cfg_cnt += 1;
                    }
                }
                hprintln!("|{:>16}|", cfg_cnt);
                hprintln!(
                        "+--------+--------+----------------+----------------+----------------+----------------+"
                    );

                for cid in 0..shm::MAX_SHM_SHARERS {
                    if let Some(conf) = region.perm_conf.conf_list[cid] {
                        hprintln!("");
                        hprintln!("+--------+--------+--------+--------+");
                        hprintln!("|         PermConf                  |");
                        hprintln!("+--------+--------+--------+--------+");
                        hprintln!("|  eid   |st_perm |dyn_perm|   map  |");
                        hprintln!("+--------+--------+--------+--------+");
                        break;
                    }
                }

                for cid in 0..shm::MAX_SHM_SHARERS {
                    if let Some(conf) = region.perm_conf.conf_list[cid] {
                        hprint!("|{:>8}", conf.eid);
                        hprint!("|{:>8x}", conf.st_perm);
                        hprint!("|{:>8x}", conf.dyn_perm);
                        hprintln!("|{:>8x}|", conf.maps);
                        hprintln!("+--------+--------+--------+--------+");
                    }
                }
            }
        }
    }
}
