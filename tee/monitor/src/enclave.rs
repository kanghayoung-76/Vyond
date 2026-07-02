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

// Complete snapshot of a host (S-mode) context: general-purpose registers +
// M-mode fields from TrapFrame + the S-mode CSRs that Linux context-switches.
// Used to save sub-host's context when it parks in wait_and_resume_for_shm so
// that resume_from_shm_ipi can load it before switch_to_enclave, enabling
// OCALLs inside IPI-context execution.
#[derive(Clone, Copy)]
struct HostContext {
    regs:       TrapFrame,
    sstatus:    usize,
    sie:        usize,
    stvec:      usize,
    scounteren: usize,
    sscratch:   usize,
    sepc:       usize,
    scause:     usize,
    sip:        usize,
    satp:       usize,
}

// Capture the caller's complete host context (tf + live S-mode CSRs).
// Must be called while the S-mode CSRs belong to the host (not the enclave).
fn capture_host_ctx(tf: &TrapFrame) -> HostContext {
    HostContext {
        regs:       *tf,
        sstatus:    csr_read!(sstatus),
        sie:        csr_read!(sie),
        stvec:      csr_read!(stvec),
        scounteren: csr_read!(scounteren),
        sscratch:   csr_read!(sscratch),
        sepc:       csr_read!(sepc),
        scause:     csr_read!(scause),
        sip:        csr_read!(sip),
        satp:       csr_read!(satp),
    }
}

// Write a saved host context back to tf and the live S-mode CSR hardware.
fn restore_host_ctx(ctx: &HostContext, tf: &mut TrapFrame) {
    *tf = ctx.regs;
    csr_write!(sstatus,    ctx.sstatus);
    csr_write!(sie,        ctx.sie);
    csr_write!(stvec,      ctx.stvec);
    csr_write!(scounteren, ctx.scounteren);
    csr_write!(sscratch,   ctx.sscratch);
    csr_write!(sepc,       ctx.sepc);
    csr_write!(scause,     ctx.scause);
    csr_write!(sip,        ctx.sip);
    csr_write!(satp,       ctx.satp);
}

// Per-hart: context of the Linux task that was running when IPI fired.
// Set by resume_from_shm_ipi; cleared (and task restored) when enc2 parks.
static mut IPI_INTERRUPTED: [Option<HostContext>; crate::ipi::MAX_HARTS] =
    [None; crate::ipi::MAX_HARTS];

// Fallback flag: set when IPI fires but saved_host is not yet available.
// In this case thread.State holds the interrupted task (old behavior).
// All stop/exit/park calls must return IpiHandled to avoid corrupting tf.
static mut IN_IPI_CTX: [bool; crate::ipi::MAX_HARTS] = [false; crate::ipi::MAX_HARTS];

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum State {
    Stopped,
    Running,
    Destroying,
    WaitingForDevice(u32), // suspended; will be resumed by SM on device IRQ
    WaitingForShm(u32),    // suspended; will be resumed by notify_shm
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RegionType {
    RegionInvalid,
    RegionEPM,
    RegionUTM,
    RegionShared,
    RegionOther,
    RegionEncEnc,  // enc-to-enc channel with hash-based attestation
    RegionDevEnc,  // device-to-enc channel with hash-based attestation
}

pub struct Region {
    id: usize,
    r_type: RegionType,
    paddr: usize,
    size: usize,
    device_wid: Option<u32>,  // Some(wid) for dev-SHM regions; None otherwise
    perm_conf: shm::RegionPermConfig,
    // Hash-based attestation fields (enc-enc and dev-enc channels).
    // Zeroed means "no hash check" (host-enc and legacy paths).
    creator_hash: [u8; 64],  // hash of the enc that registered this channel
    allowed_hash: [u8; 64],  // hash of the enc permitted to subscribe
}

/* TODO: does not support multithreaded enclave yet */
pub const MAX_ENCLAVE_THREADS: usize = 1;
pub const MAX_ENCLAVE_REGIONS: usize = 8;

pub struct RunState {
    count: usize,
    state: State,
    notified_by: Option<usize>,      // eid that switched to us via notify_shm; cleared on next wait_shm
    pending_ocall_enc2: Option<usize>, // enc2's eid when enc2 made OCALL while running under our HOST context
    pending_shm_notify: Option<u32>, // rid queued by notify_shm while enc2 was Running (multicore race fix)
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

    // Hart on which enc_subscriber most recently called wait_shm.
    // IPI targeting uses this to wake the correct hart without host involvement.
    pub last_hart: usize,

    // Sub-host's complete context saved when it parks in wait_and_resume_for_shm.
    // resume_from_shm_ipi loads this before switch_to_enclave so that thread.State
    // holds sub-host's context (not the interrupted Linux task), enabling OCALLs.
    saved_host: Option<HostContext>,
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
                notified_by: None,
                pending_ocall_enc2: None,
                pending_shm_notify: None,
            }),
            threads: [Self::THREAD_INIT; MAX_ENCLAVE_THREADS],
            pa_params,
            hash: [0u8; 64],
            last_wid: crate::wid::ENCLAVE_WID_MIN,
            last_hart: 0,
            saved_host: None,
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
            // MPIE=1 → after MRET, MIE=1 so M-mode timer can preempt loading.
            // a0-a4 are saved by swap_prev_state, so PA params survive preemption.
            regs.mstatus = (1 << crate::encoding::MSTATUS_MPP_SHIFT) | crate::encoding::MSTATUS_MPIE;
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

    /// Switch from enclave to host context.
    ///
    /// `keep_vec`: if true, mtvec is left pointing at `trap_vector_enclave` so that
    /// M-mode SHM IPIs can be caught directly without a host round-trip (wait_shm path).
    /// All other callers pass false to restore the standard OpenSBI `_trap_handler`.
    pub fn switch_to_host(&mut self, regs: &mut TrapFrame, keep_vec: bool) {
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

        if !keep_vec {
            switch_vector_host();
        }

        let pending = csr_read!(mip);

        if (pending & MIP_MTIP) != 0 {
            csr_clear!(mip, MIP_MTIP);
            csr_set!(mip, MIP_STIP);
        }
        if (pending & MIP_MSIP) != 0 {
            // MSIP pending: clear before returning to host.
            // In keep_vec mode (wait_shm) we do NOT convert to SSIP here — the pending
            // IPI will be handled by the M-mode handler (trap_vector_enclave) on the
            // next trap.  If we converted to SSIP, Linux would handle it and the
            // notification would be lost.
            csr_clear!(mip, MIP_MSIP);
            if !keep_vec {
                csr_set!(mip, MIP_SSIP);
            }
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
            creator_hash: [0u8; 64],
            allowed_hash: [0u8; 64],
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

// Bare-metal safe hash comparison — avoids bcmp (not available in riscv64-unknown-elf).
#[inline]
fn hash_eq(a: &[u8; 64], b: &[u8; 64]) -> bool {
    let mut eq = true;
    for i in 0..64 {
        if a[i] != b[i] { eq = false; }
    }
    eq
}

// [0u8; 64] is the "open / wildcard" sentinel — no hash check.
#[inline]
fn is_open_hash(h: &[u8; 64]) -> bool {
    let mut all_zero = true;
    for &b in h.iter() { if b != 0 { all_zero = false; } }
    all_zero
}

// WID → device MMIO base for known devices.
// SM uses this to configure DMA target registers without exposing MMIO to enclaves.
fn device_mmio_base(device_wid: u32) -> Option<usize> {
    match device_wid {
        29 => Some(0x6004000),  // MYDEV (WID=29)
        _  => None,
    }
}

// MMIO register offsets inside MYDEV (matches bridge.c / MYDEV hardware spec).
const MYDEV_OFF_CMD:      usize = 0x00;
const MYDEV_OFF_DMA_LOW:  usize = 0x08;
const MYDEV_OFF_DMA_HIGH: usize = 0x0c;
const MYDEV_OFF_DMA_LEN:  usize = 0x10;

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
                            // Pre-configure all SHM slots accessible by this enclave using
                            // the newly assigned WID.  This prevents a second IOMMU flush
                            // while enc2 is executing — the SHM fault handler's set_shm_perm
                            // call triggers wgchecker_iommu_notify_all which corrupts QEMU's
                            // instruction-fetch TLB and causes "Bad ram pointer" crash.
                            for shm_idx in 0..MAX_SHARED_REGIONS {
                                unsafe {
                                    if let Some(ref shm) = SHARED_MEM[shm_idx] {
                                        if shm.perm_conf.get_perm(eid).is_some() {
                                            let is_dev = shm.device_wid.is_some();
                                            let mut perm: u64 = 0;
                                            for copt in shm.perm_conf.conf_list.iter() {
                                                if let Some(c) = copt {
                                                    if c.eid == 11 {
                                                        if is_dev { continue; }
                                                        perm |= 3u64 << (crate::wg::OS_WID as u64 * 2);
                                                    } else {
                                                        let w = if c.eid == eid { wid } else {
                                                            match find_enclave(c.eid) {
                                                                Some(e) => e.last_wid,
                                                                None => continue,
                                                            }
                                                        };
                                                        perm |= 3u64 << (w as u64 * 2);
                                                    }
                                                }
                                            }
                                            if let Some(dwid) = shm.device_wid {
                                                perm |= 3u64 << (dwid as u64 * 2);
                                            }
                                            if perm != 0 {
                                                let _ = isolator::set_shm_perm(shm.id, perm);
                                            }
                                        }
                                    }
                                }
                            }
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

        // Coroutine OCALL redirect: enc2 made an OCALL while running under enc1's HOST context.
        // Copy enc1's UTM return data back to enc2's UTM, then resume enc2 directly.
        let pending_enc2_eid = runstate.pending_ocall_enc2.take();
        if let Some(enc2_eid) = pending_enc2_eid {
            drop(runstate);
            let enc1_utm = enclave.pa_params.untrusted_base;
            let enc1_utm_size = enclave.pa_params.untrusted_size;
            let enc2_eid_val = enc2_eid;
            if let Some(enc2) = find_enclave(enc2_eid_val) {
                let copy_size = enc1_utm_size.min(enc2.pa_params.untrusted_size);
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        enc1_utm as *const u8,
                        enc2.pa_params.untrusted_base as *mut u8,
                        copy_size,
                    );
                }
                let mut rs2 = enc2.state.lock();
                let resumable = rs2.state == State::Stopped && rs2.count < MAX_ENCLAVE_THREADS;
                if resumable {
                    rs2.count += 1;
                    rs2.state = State::Running;
                }
                drop(rs2);
                if resumable {
                    enc2.switch_to_enclave(tf, false);
                    return Ok(());
                }
            }
            return Err(Error::NotResumable);
        }

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

/// Called by host (sub-host) when enc_subscriber is parked at wait_shm.
/// SM blocks in M-mode using WFI until an IPI (MSIP) arrives for this enclave,
/// then directly switches into enc2 with wait_shm's return pre-set to 0.
///
/// - WaitingForShm: enc2 parked again (caller should loop — one call per IPI cycle)
/// - EdgeCallHost: enc2 made an OCALL (caller should dispatch and resume)
/// - Success: enc2 exited normally
/// - NotResumable / InvalidId: error
pub fn wait_and_resume_for_shm(tf: &mut TrapFrame, eid: usize) -> Result<(), Error> {
    if cpu::is_enclave_context() {
        return Err(Error::Invalid);
    }

    // Quick state check without holding enc alive across resume_enclave call.
    let is_waiting = match find_enclave(eid) {
        Some(enc) => matches!(enc.state.lock().state, State::WaitingForShm(_)),
        None => return Err(Error::InvalidId),
    };

    let this_hart = csr_read!(mhartid) as usize;

    // Deferred-IPI race: notify_shm changes enc2 WaitingForShm→Running AND sends MSIP.
    // If shm_ipi fires while Linux is in S-mode (MPP≠M), it defers by putting the IPI
    // back in PENDING_RESUME without switching to enc2.  enc2 is left in Running state.
    // In this case is_waiting=false, but there IS a pending IPI — we must NOT call
    // resume_enclave (enc2 Running → NOT_RESUMABLE).  Instead fall through to the poll
    // path which handles enc2 Running + pending IPI correctly (advance_past_ecall +
    // switch_to_enclave).
    let has_deferred_ipi = crate::ipi::peek_pending_resume_for(this_hart, eid);
    if !is_waiting && !has_deferred_ipi {
        // Not parked AND no deferred IPI — normal resume (OCALL-return path or error).
        return resume_enclave(tf, eid);
    }

    // Save sub-host's complete context (tf + live S-mode CSRs) into enc.saved_host.
    // When IPI fires in the Linux path (resume_from_shm_ipi), this snapshot is
    // loaded into tf before switch_to_enclave so that thread.State ends up holding
    // sub-host's context — enabling OCALLs from within IPI-woken enc2.
    if let Some(enc) = find_enclave(eid) {
        enc.saved_host = Some(capture_host_ctx(tf));
    }

    loop {
        // Check for a pending IPI before sleeping — handles the race where IPI
        // arrived between the caller getting WaitingForShm and this call, OR the
        // deferred-IPI case where enc2 is Running but PENDING_RESUME is set.
        if crate::ipi::peek_pending_resume_for(this_hart, eid) {
            crate::ipi::clear_msip(this_hart);
            crate::ipi::take_pending_resume(this_hart);

            if let Some(enc) = find_enclave(eid) {
                {
                    let mut rs = enc.state.lock();
                    if matches!(rs.state, State::WaitingForShm(_)) {
                        rs.count += 1;
                        rs.state = State::Running;
                    }
                }
                // Advance enc2's saved mepc past wait_shm ecall and set a0=0 so enc2
                // resumes at wait_shm+4 with a0=0 (success), not re-executing the ecall.
                if let Some(thread) = enc.threads[0].as_mut() {
                    thread.advance_past_ecall_with_interrupted();
                }
                heprintln!("[SM:wait_and_resume] hart={} eid={} IPI → switch_to_enclave (WFI path)", this_hart, eid);
                // WFI path: tf = sub-host's context, switch_to_enclave saves it into
                // thread.State directly.  No IPI_INTERRUPTED needed here.
                enc.switch_to_enclave(tf, false);
                // switch_to_enclave does mret — this return is unreachable.
                return Ok(());
            }
            return Err(Error::InvalidId);
        }

        // No IPI pending yet. Return immediately so Linux can sleep and schedule
        // other tasks — no WFI, no hart monopolization.
        return Err(Error::WaitingForShm);
    }
}

pub fn stop_enclave(tf: &mut TrapFrame, request: usize) -> Result<(), Error> {
    if let Some(enclave) = find_enclave(cpu::get_enclave_id()) {
        let mut runstate = enclave.state.lock();
        let runnable = runstate.state == State::Running;
        // Peek at notified_by without clearing — wait_shm still needs it after the OCALL returns.
        let enc1_eid_if_ocall = if request == 1 /*EdgeCallHost*/ {
            runstate.notified_by
        } else {
            None
        };

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

        // Coroutine OCALL: enc2 is running under enc1's HOST context.
        // Copy enc2's entire UTM to enc1's UTM so the host dispatcher reads the correct data,
        // then record the pending OCALL so resume_enclave(enc1) redirects back to enc2.
        if let Some(enc1_eid) = enc1_eid_if_ocall {
            let enc2_utm = enclave.pa_params.untrusted_base;
            let enc2_utm_size = enclave.pa_params.untrusted_size;
            let enc2_eid = enclave.eid;
            if let Some(enc1) = find_enclave(enc1_eid) {
                let copy_size = enc2_utm_size.min(enc1.pa_params.untrusted_size);
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        enc2_utm as *const u8,
                        enc1.pa_params.untrusted_base as *mut u8,
                        copy_size,
                    );
                }
                let mut rs1 = enc1.state.lock();
                rs1.pending_ocall_enc2 = Some(enc2_eid);
                drop(rs1);
            }
        }

        enclave.switch_to_host(tf, false);

        // IPI-context path: IPI_INTERRUPTED[hart] is set when enc2 was woken by
        // resume_from_shm_ipi (Linux path) and thread.State held sub-host's context.
        // switch_to_host above restored sub-host into tf correctly.
        //
        // For EdgeCallHost (OCALL): sub-host is now in tf — let normal processing
        // continue.  vyond.c will write EdgeCallHost into tf.a0, which is sub-host's
        // a0 register, and sub-host handles the OCALL normally.
        //
        // For Stopped/Interrupted (enc2 parks or exits): tf holds sub-host's context
        // but we must restore the interrupted Linux task T and return IpiHandled so
        // vyond.c does not touch tf.a0 / tf.mepc.  Sub-host's context is saved back
        // into enc.saved_host for the next IPI cycle.
        let this_hart = csr_read!(mhartid) as usize;
        let is_ipi_ctx = unsafe { IPI_INTERRUPTED[this_hart].is_some() };
        let is_fallback_ipi = unsafe { IN_IPI_CTX[this_hart] };

        // IPI context, non-OCALL (enc2 stops or is preempted): restore task T.
        if is_ipi_ctx && request != 1 /*EdgeCallHost*/ {
            enclave.saved_host = Some(capture_host_ctx(tf));
            if let Some(task_t) = unsafe { IPI_INTERRUPTED[this_hart].take() } {
                restore_host_ctx(&task_t, tf);
            }
            heprintln!("[SM:stop_enclave] IPI ctx stop → restored task T (hart={})", this_hart);
            return Err(Error::IpiHandled);
        }

        // IPI context + EdgeCallHost (OCALL in IPI context):
        // saved_host path (!is_fallback_ipi): sub-host was blocking in
        // wait_and_resume_for_shm — its ioctl context is valid in tf.
        // Deliver the OCALL normally: switch_to_host already restored sub-host into
        // tf, so just return EdgeCallHost.  sub-host handles the OCALL and calls
        // RESUME_ENCLAVE.  When enc2 next parks (wait_shm), is_ipi_ctx=true restores
        // task T from IPI_INTERRUPTED.
        //
        // Fallback path (is_fallback_ipi): sub-host context not available — suppress.
        if is_ipi_ctx && request == 1 /*EdgeCallHost*/ && is_fallback_ipi {
            {
                let mut rs = enclave.state.lock();
                rs.count += 1;
                rs.state = State::Running;
            }
            if let Some(thread) = enclave.threads[0].as_mut() {
                thread.advance_past_ecall_with_interrupted(); // mepc+=4, a0=0
            }
            enclave.saved_host = Some(capture_host_ctx(tf));
            enclave.switch_to_enclave(tf, false);
            heprintln!("[SM:stop_enclave] fallback IPI ctx OCALL suppressed → re-entered enc2 (hart={})", this_hart);
            return Err(Error::IpiHandled);
        }
        // IPI context + EdgeCallHost + saved_host path: deliver OCALL to sub-host.
        // tf already holds sub-host's context (switch_to_host above).
        // Fall through to normal EdgeCallHost return below.
        if is_ipi_ctx && request == 1 /*EdgeCallHost*/ && !is_fallback_ipi {
            heprintln!("[SM:stop_enclave] IPI ctx OCALL → delivering to sub-host (hart={})", this_hart);
            // IPI_INTERRUPTED left intact — restored when enc2 parks.
        }

        // Fallback (IN_IPI_CTX), non-OCALL: switch_to_host already restored task T
        // from thread.State into tf. Clear flag and return IpiHandled.
        if request != 1 && is_fallback_ipi {
            unsafe { IN_IPI_CTX[this_hart] = false; }
            heprintln!("[SM:stop_enclave] fallback IPI ctx → IpiHandled (hart={})", this_hart);
            return Err(Error::IpiHandled);
        }

        // Fallback (IN_IPI_CTX) + EdgeCallHost: tf = task T (from thread.State),
        // thread.State = enc2's OCALL frame. Advance enc2's frame and re-enter enc2.
        // When enc2 parks next (wait_shm), IN_IPI_CTX=false → WaitingForShm path
        // mrets to task T which switch_to_host will restore from thread.State.
        if request == 1 && is_fallback_ipi {
            unsafe { IN_IPI_CTX[this_hart] = false; }
            {
                let mut rs = enclave.state.lock();
                rs.count += 1;
                rs.state = State::Running;
            }
            if let Some(thread) = enclave.threads[0].as_mut() {
                thread.advance_past_ecall_with_interrupted(); // mepc+=4, a0=0
            }
            enclave.saved_host = Some(capture_host_ctx(tf));
            enclave.switch_to_enclave(tf, false);
            heprintln!("[SM:stop_enclave] fallback IPI ctx OCALL suppressed → re-entered enc2 (hart={})", this_hart);
            return Err(Error::IpiHandled);
        }

        let ret = match request {
            0 /*StopReason::TimerInterrupt*/ => Err(Error::Interrupted),
            1 /*StopReason::EdgeCallHost*/   => Err(Error::EdgeCallHost),
            _                                => Err(Error::Unknown),
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
        enclave.switch_to_host(tf, false);
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

/// Called by enc_subscriber to suspend itself until enc_publisher calls notify_shm(rid).
///
/// Key design points:
/// - Saves `last_hart` so notify_shm knows where to send the IPI.
/// - Calls switch_to_host with keep_vec=true so that mtvec remains at
///   trap_vector_enclave while Linux runs.  This lets M-mode catch the SHM MSIP
///   directly without any host process involvement.
/// - After switch_to_host, checks for a race (IPI already posted before handler set up).
/// - If notified_by is set (single-core round-trip), resumes enc_publisher directly.
/// - Returns Err(WaitingForShm) for the normal slow path (wait for IPI).
/// - Returns Err(Interrupted) for the race fast path (already resumed).
/// - Returns Ok(0) for the single-core loop path (enc_publisher resumed → C a0=0).
pub fn wait_shm(tf: &mut TrapFrame, rid: u32) -> Result<(), Error> {
    if let Some(enclave) = find_enclave(cpu::get_enclave_id()) {
        let mut runstate = enclave.state.lock();
        if runstate.state != State::Running {
            return Err(Error::NotRunning);
        }
        // Multicore race fast-path: enc_publisher called notify_shm while we were Running
        // (between IPI wake-up and this wait_shm call). It stored the rid in
        // pending_shm_notify instead of printing "no WaitingForShm match". Consume it
        // and return immediately so enc2 continues without parking.
        if let Some(pending_rid) = runstate.pending_shm_notify.take() {
            if pending_rid == rid {
                drop(runstate);
                heprintln!("[SM:wait_shm] eid={} rid={} → pending notify consumed, skip park",
                           enclave.eid, rid);
                return Err(Error::Interrupted); // C: a0=0, mepc+=4 → enc resumes immediately
            }
            runstate.pending_shm_notify = Some(pending_rid); // put back on rid mismatch
        }

        // Consume enc_publisher eid if this is a single-core loop (resume it after suspend).
        let notified_by_eid = runstate.notified_by.take();
        runstate.count -= 1;
        runstate.state = State::WaitingForShm(rid);
        drop(runstate);

        // Record which hart we're parking on so notify_shm can target the IPI.
        enclave.last_hart = csr_read!(mhartid) as usize;
        heprintln!("[SM:wait_shm] eid={} rid={} last_hart={} → WaitingForShm",
                   enclave.eid, rid, enclave.last_hart);

        // Switch to host context while keeping mtvec = trap_vector_enclave.
        // Linux will run on this hart; any M-mode interrupt goes to our handler,
        // allowing sbi_sm_handle_shm_ipi to resume us without host involvement.
        enclave.switch_to_host(tf, true);

        // saved_host was set in wait_and_resume_for_shm (the sub-host's SBI ecall
        // context, valid while sub-host is in that ioctl).  Do NOT clear it here:
        // sub-host's KEYSTONE_IOC_WAIT_AND_RESUME loop will call SM again immediately,
        // refreshing saved_host for the next IPI cycle.
        // The fast-path race check below handles any IPI that arrives in the gap.

        // --- Fast-path race check ------------------------------------------
        // notify_shm may have stored our eid in PENDING_RESUME and sent MSIP
        // before we got here.  switch_to_host clears any pending MSIP (without
        // forwarding it as SSIP in keep_vec mode), so the IPI would be lost.
        // Detect and handle the race immediately.
        let this_hart = enclave.last_hart;
        if crate::ipi::peek_pending_resume_for(this_hart, enclave.eid) {
            crate::ipi::take_pending_resume(this_hart);
            // State is already Running (notify_shm set it), count already incremented.
            enclave.switch_to_enclave(tf, false);
            // C WAIT_SHM handler detects Interrupted → a0=0, mepc+=4 → enc_subscriber resumes.
            return Err(Error::Interrupted);
        }
        // --- End race check -----------------------------------------------

        // Single-core loop: enc_publisher was suspended inside notify_shm waiting
        // for us to call wait_shm.  Resume it now.
        if let Some(enc1_eid) = notified_by_eid {
            if let Some(enc1) = find_enclave(enc1_eid) {
                let mut rs1 = enc1.state.lock();
                let resumable = rs1.state == State::Stopped && rs1.count == 0;
                if resumable {
                    rs1.count += 1;
                    rs1.state = State::Running;
                }
                drop(rs1);
                if resumable {
                    enc1.switch_to_enclave(tf, false);
                    return Ok(()); // C: a0=0, mepc+=4 → enc_publisher at notify_shm+4 ✓
                }
            }
        }

        // IPI-context park: enc_subscriber called wait_shm after being woken by IPI.
        // switch_to_host above restored sub-host's context into tf.
        // We now:
        //   1. Save sub-host's refreshed context into enc.saved_host (for next IPI).
        //   2. Restore the interrupted Linux task T from IPI_INTERRUPTED[hart].
        //   3. Return IpiHandled so vyond.c mrets to task T cleanly.
        if unsafe { IPI_INTERRUPTED[this_hart].is_some() } {
            // New path: saved_host was available; thread.State held sub-host's context.
            // switch_to_host above restored sub-host into tf.
            // Save sub-host's refreshed context for the next IPI, then restore task T.
            enclave.saved_host = Some(capture_host_ctx(tf));
            if let Some(task_t) = unsafe { IPI_INTERRUPTED[this_hart].take() } {
                restore_host_ctx(&task_t, tf);
            }
            heprintln!("[SM:wait_shm] IPI ctx park → saved sub-host, restored task T (hart={})", this_hart);
            return Err(Error::IpiHandled);
        }

        // Fallback path: saved_host was not available; thread.State held the interrupted
        // task's context (old behavior).  switch_to_host restored that task into tf.
        // Return IpiHandled so vyond.c does not corrupt tf.a0/tf.mepc.
        if unsafe { IN_IPI_CTX[this_hart] } {
            unsafe { IN_IPI_CTX[this_hart] = false; }
            heprintln!("[SM:wait_shm] fallback IPI ctx → IpiHandled (hart={})", this_hart);
            return Err(Error::IpiHandled);
        }

        // Normal slow path: wait for IPI (mtvec = trap_vector_enclave, Linux runs).
        return Err(Error::WaitingForShm);
    }
    Err(Error::Invalid)
}

/// Called by enc_publisher after writing to SHM to wake enc_subscriber.
///
/// Same-hart (single-core):
///   Suspends enc_publisher, directly switches to enc_subscriber.
///   enc_subscriber resumes at wait_shm+4.  When enc_subscriber later calls
///   wait_shm again, the SM resumes enc_publisher (via notified_by).
///   Returns Err(Interrupted) → C: a0=0, mepc+=4, sbi_trap_exit → enc_subscriber.
///
/// Different-hart (multi-core):
///   enc_publisher is NOT suspended (non-blocking notify).
///   Stores enc_subscriber eid in PENDING_RESUME, fires CLINT MSIP on target hart.
///   M-mode IPI handler on that hart (trap_vector_enclave / sbi_trap_handler_keystone_enclave
///   IRQ_M_SOFT host-context path) calls sbi_sm_handle_shm_ipi which directly
///   resumes enc_subscriber without any host process involvement.
///   Returns Ok(()) → enc_publisher's notify_shm SBI call returns 0.
pub fn notify_shm(tf: &mut TrapFrame, rid: u32) -> Result<(), Error> {
    let enc1_eid = cpu::get_enclave_id();
    let current_hart = csr_read!(mhartid) as usize;
    heprintln!("[SM:notify_shm] eid={} rid={} hart={}", enc1_eid, rid, current_hart);

    // Find enc_subscriber waiting on this rid, update its state.
    let found: Option<(usize, usize)> = unsafe {
        let mut res = None;
        for slot in 0..MAX_ENCLAVES {
            if let Some(ref mut enc2) = ENCLAVES[slot] {
                let mut rs2 = enc2.state.lock();
                if matches!(rs2.state, State::WaitingForShm(r) if r == rid) {
                    let enc2_eid    = enc2.eid;
                    let target_hart = enc2.last_hart;
                    rs2.count += 1;
                    rs2.state = State::Running;
                    // Single-core: enc_publisher will be suspended; enc_subscriber must
                    // resume it when it calls wait_shm again (via notified_by).
                    rs2.notified_by = if target_hart == current_hart {
                        Some(enc1_eid)
                    } else {
                        None
                    };
                    drop(rs2);
                    res = Some((enc2_eid, target_hart));
                    break;
                }
                drop(rs2);
            }
        }
        res
    };

    let (enc2_eid, target_hart) = match found {
        Some(t) => t,
        None => {
            // Second-chance scan: close the race where enc2 transitions
            // Running→WaitingForShm between the first scan and now.
            // We check WaitingForShm AND Running in a SINGLE per-slot lock so
            // the two checks are atomic — no gap where enc2 can slip through.
            let retry: Option<(usize, usize)> = unsafe {
                let shm_rgn_opt = get_shm_region_by_rid(rid as usize);
                let mut found2 = None;
                for slot in 0..MAX_ENCLAVES {
                    if let Some(ref mut enc2) = ENCLAVES[slot] {
                        if enc2.eid == enc1_eid { continue; }
                        let mut rs2 = enc2.state.lock();
                        if matches!(rs2.state, State::WaitingForShm(r) if r == rid) {
                            // enc2 transitioned Running→WaitingForShm after the first scan.
                            let enc2_eid    = enc2.eid;
                            let target_hart = enc2.last_hart;
                            rs2.count += 1;
                            rs2.state = State::Running;
                            rs2.notified_by = if target_hart == current_hart {
                                Some(enc1_eid)
                            } else {
                                None
                            };
                            drop(rs2);
                            found2 = Some((enc2_eid, target_hart));
                            break;
                        }
                        if let Some(ref shm_rgn) = shm_rgn_opt {
                            if shm_rgn.perm_conf.get_perm(enc2.eid).is_some()
                                && rs2.state == State::Running
                                && enc2.last_hart != current_hart
                                && rs2.pending_shm_notify.is_none()
                            {
                                heprintln!(
                                    "[SM:notify_shm] rid={} eid={} Running on hart={}, queuing pending notify",
                                    rid, enc2.eid, enc2.last_hart
                                );
                                rs2.pending_shm_notify = Some(rid);
                                drop(rs2);
                                return Ok(());
                            }
                        }
                        drop(rs2);
                    }
                }
                found2
            };
            match retry {
                Some(t) => t,
                None => {
                    heprintln!("[SM:notify_shm] rid={} → no WaitingForShm match", rid);
                    return Ok(());
                }
            }
        }
    };

    if target_hart == current_hart {
        // ---- Single-core: suspend enc_publisher, directly wake enc_subscriber ----
        if let Some(enc1) = find_enclave(enc1_eid) {
            let mut rs1 = enc1.state.lock();
            rs1.count -= 1;
            rs1.state = State::Stopped;
            drop(rs1);
            enc1.switch_to_host(tf, false); // tf = Process A kernel context
        }
        if let Some(enc2) = find_enclave(enc2_eid) {
            enc2.switch_to_enclave(tf, false); // tf = enc_subscriber saved state
        }
        // C detects Interrupted → a0=0, mepc+=4, sbi_trap_exit → enc_subscriber resumes
        return Err(Error::Interrupted);
    } else {
        // ---- Multi-core: fire MSIP on target hart, enc_publisher keeps running ----
        heprintln!("[SM:notify_shm] multicore: IPI to hart={} for eid={}", target_hart, enc2_eid);
        crate::ipi::post_resume(target_hart, enc2_eid); // Release fence + store
        crate::ipi::send_msip(target_hart);             // Assert CLINT MSIP
        return Ok(()); // enc_publisher's notify_shm returns 0 (non-blocking)
    }
}

/// Called by sbi_sm_handle_shm_ipi from the M-mode IRQ handler when MSIP fires
/// on a hart where enc_subscriber is parked (mtvec = trap_vector_enclave, host context).
///
/// Clears the CLINT MSIP, swaps the interrupted Linux context with enc_subscriber's
/// saved state, and sets CPU state to enclave.  C handler then does:
///   regs->a0 = 0; regs->mepc += 4; sbi_trap_exit(regs)
/// so enc_subscriber resumes at wait_shm+4 with a0=0 (success).
/// Returns true if an enclave was resumed, false otherwise.
pub fn resume_from_shm_ipi(tf: &mut TrapFrame) -> bool {
    let this_hart = csr_read!(mhartid) as usize;
    crate::ipi::clear_msip(this_hart); // Deassert CLINT MSIP

    if let Some(eid) = crate::ipi::take_pending_resume(this_hart) {
        heprintln!("[SM:shm_ipi] hart={} resuming eid={}", this_hart, eid);

        // Check whether the interrupted context is M-mode (sub-host actively processing
        // a WAIT_AND_RESUME ecall with valid saved_host) or S/U-mode (Linux running,
        // sub-host sleeping in schedule_timeout_interruptible).
        //
        // mstatus.MPP (bits [12:11]) = 3 → M-mode, 1 → S-mode, 0 → U-mode.
        //
        // If S/U-mode: saved_host is STALE (set during a previous WAIT_AND_RESUME ecall
        // that already returned to user space).  Using it would restore an old kernel sp,
        // rewinding the stack past the sleep frame and corrupting the timer struct —
        // causing kernel panic in call_timer_fn.
        // Defer: put the IPI back in PENDING_RESUME; the next WAIT_AND_RESUME ecall
        // (polled by the driver after ~1 jiffie) will pick it up with a fresh saved_host.
        let mpp = (tf.mstatus >> 11) & 0x3;
        if mpp != 3 /* not M-mode */ {
            crate::ipi::post_resume(this_hart, eid);
            heprintln!("[SM:shm_ipi] hart={} eid={} → S-mode interrupted, deferred (WAIT_AND_RESUME will handle)", this_hart, eid);
            return false;
        }

        if let Some(enc) = find_enclave(eid) {
            // Fix state: enc_subscriber was WaitingForShm; we are now Running it in IPI ctx.
            {
                let mut rs = enc.state.lock();
                if matches!(rs.state, State::WaitingForShm(_)) {
                    rs.count += 1;
                    rs.state = State::Running;
                }
            }

            // Save the interrupted Linux task's complete context (tf + S-mode CSRs).
            // This will be restored when enc2 parks (wait_shm) or exits.
            unsafe { IPI_INTERRUPTED[this_hart] = Some(capture_host_ctx(tf)); }

            // Load sub-host's saved context into tf and the live S-mode CSRs.
            // switch_to_enclave will then store sub-host's context into thread.State
            // (via swap_prev_state), so any subsequent OCALL can restore sub-host
            // correctly — just like a regular RESUME_ENCLAVE call would.
            match enc.saved_host.as_ref() {
                Some(host_ctx) => {
                    restore_host_ctx(host_ctx, tf);
                    heprintln!("[SM:shm_ipi] hart={} eid={} → loaded saved_host, OCALLs enabled", this_hart, eid);
                }
                None => {
                    // saved_host not available (first IPI before wait_and_resume_for_shm ran).
                    // Fall back to old behavior: thread.State gets the interrupted task's context.
                    // Set IN_IPI_CTX so that stop/exit/park return IpiHandled (not WaitingForShm),
                    // which prevents a0/mepc corruption of the interrupted Linux task.
                    heprintln!("[SM:shm_ipi] hart={} eid={} → no saved_host, fallback IPI ctx", this_hart, eid);
                    unsafe { IN_IPI_CTX[this_hart] = true; }
                }
            }

            // switch_to_enclave swaps tf (now sub-host's context) into thread.State
            // and loads enc2's saved registers onto CPU.  After mret, enc2 resumes
            // at wait_shm+4 with a0=0.  The C IPI handler sets a0=0, mepc+=4.
            enc.switch_to_enclave(tf, false);
            return true;
        }
    }
    false
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

        enclave.switch_to_host(tf, false);

        // IPI-context guard: same rationale as stop_enclave (non-OCALL path).
        let this_hart = csr_read!(mhartid) as usize;
        if unsafe { IPI_INTERRUPTED[this_hart].is_some() } {
            enclave.saved_host = Some(capture_host_ctx(tf));
            if let Some(task_t) = unsafe { IPI_INTERRUPTED[this_hart].take() } {
                restore_host_ctx(&task_t, tf);
            }
            heprintln!("[SM:exit_enclave] IPI ctx → restored task T (hart={})", this_hart);
            return Err(Error::IpiHandled);
        }

        // Fallback: saved_host was None; switch_to_host already restored task T into tf.
        if unsafe { IN_IPI_CTX[this_hart] } {
            unsafe { IN_IPI_CTX[this_hart] = false; }
            heprintln!("[SM:exit_enclave] fallback IPI ctx → IpiHandled (hart={})", this_hart);
            return Err(Error::IpiHandled);
        }

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
                        creator_hash: [0u8; 64],
                        allowed_hash: [0u8; 64],
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
            // enc-enc channel: hash-based attestation — allowed_hash must match caller
            // (or be all-zeros = open/wildcard for testing).
            if region.r_type == RegionType::RegionEncEnc {
                if !cpu::is_enclave_context() {
                    heprintln!("[map_shm_region] DENIED: non-enclave caller on enc-enc rid={}", rid);
                    return Err(Error::Invalid);
                }
                if !is_open_hash(&region.allowed_hash) {
                    let caller_hash = match find_enclave(caller_eid) {
                        Some(enc) => enc.hash,
                        None => return Err(Error::Invalid),
                    };
                    if !hash_eq(&caller_hash, &region.allowed_hash) {
                        heprintln!("[map_shm_region] DENIED: hash mismatch on enc-enc rid={} caller_eid={}", rid, caller_eid);
                        return Err(Error::Invalid);
                    }
                } else {
                    heprintln!("[map_shm_region] enc-enc rid={} open channel — any enclave allowed", rid);
                }
                region.perm_conf.insert_perm(shm::PermConfig {
                    eid: caller_eid,
                    dyn_perm: shm::Perm::FULL,
                    st_perm: shm::Perm::FULL,
                    maps: 1,
                });
                return Ok(());
            }
            // dev-enc channel: hash-based attestation — read-only for allowed enc.
            // SM also writes DMA target registers so the enclave never touches MMIO.
            if region.r_type == RegionType::RegionDevEnc {
                if !cpu::is_enclave_context() {
                    return Err(Error::Invalid);
                }
                if !is_open_hash(&region.allowed_hash) {
                    let caller_hash = match find_enclave(caller_eid) {
                        Some(enc) => enc.hash,
                        None => return Err(Error::Invalid),
                    };
                    if !hash_eq(&caller_hash, &region.allowed_hash) {
                        heprintln!("[map_shm_region] DENIED: hash mismatch on dev-enc rid={} caller_eid={}", rid, caller_eid);
                        return Err(Error::Invalid);
                    }
                } else {
                    heprintln!("[map_shm_region] dev-enc rid={} open channel — any enclave allowed", rid);
                }
                // DMA registers are programmed once at boot in init_dev_shm_regions().
                region.perm_conf.insert_perm(shm::PermConfig {
                    eid: caller_eid,
                    dyn_perm: shm::Perm::R,
                    st_perm: shm::Perm::R,
                    maps: 1,
                });
                return Ok(());
            }
            // Standard EID-based path (host-enc, legacy enc-enc via shareShm).
            if let Some(perm) = region.perm_conf.get_perm_mut(caller_eid) {
                perm.increment_map();
                return Ok(());
            }
            // Legacy enc-only channel (RegionShared, host EID absent): auto-grant.
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
                        creator_hash: [0u8; 64],
                        allowed_hash: [0u8; 64],
                    });
                }
                dbg!("[create_enclave_shm] pa={:x} size={:?} rid={:?}", paddr, size, region_idx);
                return Ok(region_idx);
            }
        }
    }
    Err(Error::Invalid)
}

// ---------------------------------------------------------------------------
// dev-enc: hardcoded config table
// ---------------------------------------------------------------------------

/// One entry in the boot-time device SHM config table.
/// PA and size describe a pre-allocated physical memory region shared between
/// a hardware device (identified by device_wid) and a trusted enclave node
/// (identified by allowed_hash).  allowed_hash is a placeholder for now;
/// replace with the real SHA3-512 measurement of the target enclave binary.
struct DevShmConfig {
    pa:           usize,
    size:         usize,
    device_wid:   u32,
    allowed_hash: [u8; 64],
}

/// Compile-time device SHM table.  Add one entry per device-enclave channel.
/// PA addresses and WIDs must not overlap with EPM / host memory regions.
static DEV_SHM_TABLE: &[DevShmConfig] = &[
    // my_dev (WID=29): DMA buffer at 0x9000_0000, 4 KiB.
    // allowed_hash=[0u8;64] = open for testing (any enclave may map).
    DevShmConfig {
        pa:           0x9000_0000,
        size:         4096,
        device_wid:   29,
        allowed_hash: [0u8; 64],
    },
];

/// Create a single RegionDevEnc SHM region from a config entry.
/// Called only at cold-boot from init_dev_shm_regions().
fn create_dev_enc_shm(pa: usize, size: usize, device_wid: u32, allowed_hash: [u8; 64])
    -> Result<usize, Error>
{
    // Use the same isolator path as create_shared_mem for device regions.
    if let Ok(region_idx) = isolator::region_init(pa, size, 11, true) {
        for i in 0..MAX_SHARED_REGIONS {
            if unsafe { SHARED_MEM[i].is_none() } {
                unsafe {
                    SHARED_MEM[i] = Some(Region {
                        id: region_idx,
                        r_type: RegionType::RegionDevEnc,
                        paddr: pa,
                        size,
                        device_wid: Some(device_wid),
                        perm_conf: shm::RegionPermConfig {
                            owner_id: 11,
                            conf_list: [None; shm::MAX_SHM_SHARERS],
                        },
                        creator_hash: [0u8; 64],
                        allowed_hash,
                    });
                }
                heprintln!(
                    "[SM:dev_shm] created dev-enc region pa={:#x} size={} wid={} rid={}",
                    pa, size, device_wid, region_idx
                );
                return Ok(region_idx);
            }
        }
    }
    Err(Error::Invalid)
}

/// Called once during cold-boot (from sm_init) to register all device SHM
/// regions defined in DEV_SHM_TABLE into SHARED_MEM and program DMA registers.
pub fn init_dev_shm_regions() {
    for cfg in DEV_SHM_TABLE.iter() {
        match create_dev_enc_shm(cfg.pa, cfg.size, cfg.device_wid, cfg.allowed_hash) {
            Ok(rid) => {
                heprintln!("[SM:dev_shm] registered dev-enc channel rid={}", rid);
                // Write DMA target registers at boot — PA is fixed in DEV_SHM_TABLE.
                if let Some(mmio_base) = device_mmio_base(cfg.device_wid) {
                    unsafe {
                        let base = mmio_base as *mut u8;
                        core::ptr::write_volatile(
                            base.add(MYDEV_OFF_DMA_LOW) as *mut u32,
                            (cfg.pa & 0xffff_ffff) as u32,
                        );
                        core::ptr::write_volatile(
                            base.add(MYDEV_OFF_DMA_HIGH) as *mut u32,
                            (cfg.pa >> 32) as u32,
                        );
                        core::ptr::write_volatile(
                            base.add(MYDEV_OFF_DMA_LEN) as *mut u32,
                            cfg.size as u32,
                        );
                    }
                    heprintln!(
                        "[SM:dev_shm] DMA regs set at boot wid={} mmio={:#x} pa={:#x} sz={}",
                        cfg.device_wid, mmio_base, cfg.pa, cfg.size
                    );
                }
            }
            Err(e) => heprintln!("[SM:dev_shm] failed to register dev-enc channel: {:?}", e),
        }
    }
}

/// Called by enc1 (publisher) to bind hash-based attestation to an existing SHM region.
/// The region must already exist (created by host via create_enclave_shm).
/// SM records enc1's own hash as creator_hash and the caller-supplied allowed_hash.
/// Caller: must be in enclave context.
pub fn register_enc_channel(rid: usize, allowed_hash_pa: usize) -> Result<(), Error> {
    if !cpu::is_enclave_context() {
        return Err(Error::Invalid);
    }
    let caller_eid = cpu::get_enclave_id();
    let creator_hash = match find_enclave(caller_eid) {
        Some(enc) => enc.hash,
        None => return Err(Error::Invalid),
    };
    let allowed_hash: [u8; 64] = unsafe { *(allowed_hash_pa as *const [u8; 64]) };
    if let Some(region) = get_shm_region_by_rid(rid) {
        region.r_type = RegionType::RegionEncEnc;
        region.creator_hash = creator_hash;
        region.allowed_hash = allowed_hash;
        heprintln!("[SM:register_enc_channel] rid={} creator_eid={}", rid, caller_eid);
        return Ok(());
    }
    Err(Error::Invalid)
}

/// Called by enc2 (subscriber) to locate the enc-enc SHM channel created by enc1.
/// enc2 provides enc1's expected hash; SM finds the region where:
///   creator_hash == provided && allowed_hash == enc2.hash
/// Returns the rid via rid_out_pa.
/// Caller: must be in enclave context.
pub fn find_shm_by_hash(creator_hash_pa: usize, rid_out_pa: usize) -> Result<(), Error> {
    if !cpu::is_enclave_context() {
        return Err(Error::Invalid);
    }
    let caller_eid = cpu::get_enclave_id();
    let caller_hash = match find_enclave(caller_eid) {
        Some(enc) => enc.hash,
        None => return Err(Error::Invalid),
    };
    let creator_hash: [u8; 64] = unsafe { *(creator_hash_pa as *const [u8; 64]) };
    // creator_hash = {0} (all-zeros) is a wildcard: match any creator.
    // In production, set to the actual measured hash of the expected publisher.
    let creator_wildcard = is_open_hash(&creator_hash);
    unsafe {
        for i in 0..MAX_SHARED_REGIONS {
            if let Some(ref region) = SHARED_MEM[i] {
                if region.r_type == RegionType::RegionEncEnc
                    && (creator_wildcard || hash_eq(&region.creator_hash, &creator_hash))
                    && (is_open_hash(&region.allowed_hash) || hash_eq(&region.allowed_hash, &caller_hash))
                {
                    // Skip stale channels from previous runs: require an active waiter.
                    let found_rid = region.id;
                    let has_waiter = ENCLAVES.iter().any(|slot| {
                        if let Some(ref enc) = slot {
                            matches!(enc.state.lock().state, State::WaitingForShm(r) if r == found_rid as u32)
                        } else {
                            false
                        }
                    });
                    if !has_waiter {
                        continue;
                    }
                    *(rid_out_pa as *mut usize) = found_rid;
                    heprintln!("[SM:find_shm_by_hash] found rid={} for caller_eid={}", found_rid, caller_eid);
                    return Ok(());
                }
            }
        }
    }
    heprintln!("[SM:find_shm_by_hash] no matching enc-enc channel for caller_eid={}", caller_eid);
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

#[cfg(feature = "dbg")]
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

/// Called by enclave to retrieve its own measurement hash.
/// Writes 64 bytes to hash_out_pa (physical address).
pub fn get_my_hash(hash_out_pa: usize) -> Result<(), Error> {
    if !cpu::is_enclave_context() {
        return Err(Error::Invalid);
    }
    let caller_eid = cpu::get_enclave_id();
    let hash = match find_enclave(caller_eid) {
        Some(enc) => enc.hash,
        None => return Err(Error::Invalid),
    };
    unsafe { *(hash_out_pa as *mut [u8; 64]) = hash; }
    heprintln!("[SM:get_my_hash] written hash for eid={}", caller_eid);
    Ok(())
}

/// Called by enclave to locate a RegionDevEnc channel whose allowed_hash
/// matches the caller's hash (or is open/wildcard).
/// Writes the rid to rid_out_pa on success.
pub fn find_dev_shm(rid_out_pa: usize) -> Result<(), Error> {
    if !cpu::is_enclave_context() {
        return Err(Error::Invalid);
    }
    let caller_eid = cpu::get_enclave_id();
    let caller_hash = match find_enclave(caller_eid) {
        Some(enc) => enc.hash,
        None => return Err(Error::Invalid),
    };
    unsafe {
        for i in 0..MAX_SHARED_REGIONS {
            if let Some(ref region) = SHARED_MEM[i] {
                if region.r_type == RegionType::RegionDevEnc
                    && (is_open_hash(&region.allowed_hash)
                        || hash_eq(&region.allowed_hash, &caller_hash))
                {
                    *(rid_out_pa as *mut usize) = region.id;
                    heprintln!(
                        "[SM:find_dev_shm] found rid={} for caller_eid={}",
                        region.id, caller_eid
                    );
                    return Ok(());
                }
            }
        }
    }
    heprintln!("[SM:find_dev_shm] no matching dev-enc channel for caller_eid={}", caller_eid);
    Err(Error::Invalid)
}

/// Called by enclave to ask SM to write the CMD register of a device.
/// The enclave never touches MMIO directly; SM validates the device_wid
/// and performs the write on its behalf.
pub fn trigger_dev(device_wid: u32) -> Result<(), Error> {
    if !cpu::is_enclave_context() {
        return Err(Error::Invalid);
    }
    let mmio_base = match device_mmio_base(device_wid) {
        Some(base) => base,
        None => {
            heprintln!("[SM:trigger_dev] unknown device_wid={}", device_wid);
            return Err(Error::Invalid);
        }
    };
    unsafe {
        let base = mmio_base as *mut u8;
        core::ptr::write_volatile(base.add(MYDEV_OFF_CMD) as *mut u32, 1u32);
    }
    heprintln!("[SM:trigger_dev] wid={} CMD=1 written", device_wid);
    Ok(())
}
