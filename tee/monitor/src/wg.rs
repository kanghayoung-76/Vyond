use crate::isolator::PAGE_SIZE;
use crate::Error;
use semihosting::{hprint, hprintln};
use volatile_register::{RO, RW};

/// General WGC
pub const WGC_SLOT_OFFSET: usize = 0x20;
pub const WGC_SLOT_SIZE: usize = 0x20;

pub const WGC_CFG_A_TOR: u32 = 0x1;
pub const WGC_CFG_A_NAPOT: u32 = 0x3;
pub const WGC_CFG_ER: u32 = 1 << 8;
pub const WGC_CFG_EW: u32 = 1 << 9;
pub const WGC_CFG_IR: u32 = 1 << 10;
pub const WGC_CFG_IW: u32 = 1 << 11;

// TODO: read platform specific configs from dtb
pub const WGC_DRAM_BASE: usize = 0x600_0000;
pub const WGC_FLASH_BASE: usize = 0x600_1000;
pub const WGC_UART_BASE: usize = 0x600_2000;
pub const WGC_MYDEV_BASE: usize = 0x600_5000;

const DRAM_BASE: usize = 0x8000_0000;
// Enclave pool: upper half of DRAM, reserved exclusively for EPM/SHM.
// OS region covers DRAM_BASE..ENCLAVE_POOL_BASE (via a single bounded WGC TOR slot).
// To adjust the split, change ENCLAVE_POOL_BASE and rebuild; the pool boundary
// can also be made runtime-configurable via a DTB property in future work.
pub const ENCLAVE_POOL_BASE: usize = 0x1_8000_0000; // 6 GiB physical (4 GiB into DRAM, -m 8192 split)
pub const ENCLAVE_POOL_SIZE: usize = 0x1_0000_0000; // 4 GiB pool
const FLASH_BASE: usize = 0x20000000;
const FLASH_SIZE: usize = 0x4000000;
const UART_BASE: usize = 0x10000000;
const UART_SIZE: usize = 0x100;
const MYDEV_BASE: usize = 0x600_4000;
const MYDEV_SIZE: usize = 0x1000;

/// WGC for Memory
#[repr(C)]
pub struct WGCRegisterBlock {
    pub vendor: RO<u32>,
    pub impid: RO<u32>,
    pub nslots: RO<u32>,
    reserved: RO<u32>,
    pub errcause: RW<u64>,
    pub erraddr: RW<u64>,
}

#[repr(C)]
pub struct WGCSlot {
    pub addr: RW<u64>,
    pub perm: RW<u64>,
    pub cfg: RW<u32>,
    pub reserved1: RW<u64>,
    pub reserved2: RW<u32>,
}

pub struct WGChecker {
    p_wgc: &'static mut WGCRegisterBlock,
    p_slot_base: usize,
}

impl WGChecker {
    pub fn new(base: usize) -> WGChecker {
        WGChecker {
            p_wgc: unsafe { &mut *(base as *mut WGCRegisterBlock) },
            p_slot_base: base + WGC_SLOT_OFFSET,
        }
    }

    #[inline]
    pub fn get_nslots(&self) -> u32 {
        self.p_wgc.nslots.read()
    }

    #[inline]
    pub fn get_errcause(&self) -> u64 {
        self.p_wgc.errcause.read()
    }

    #[inline]
    pub fn get_erraddr(&self) -> u64 {
        self.p_wgc.erraddr.read()
    }

    #[inline]
    pub fn get_slot_addr(&self, idx: usize) -> u64 {
        let ptr = (self.p_slot_base + idx * WGC_SLOT_SIZE) as *const WGCSlot;
        unsafe { (*ptr).addr.read() }
    }

    #[inline]
    pub fn set_slot_addr(&self, idx: usize, addr: u64) {
        let ptr = (self.p_slot_base + idx * WGC_SLOT_SIZE) as *const WGCSlot;
        unsafe { (*ptr).addr.write(addr) }
    }

    #[inline]
    pub fn get_slot_perm(&self, idx: usize) -> u64 {
        let ptr = (self.p_slot_base + idx * WGC_SLOT_SIZE) as *const WGCSlot;
        unsafe { (*ptr).perm.read() }
    }

    #[inline]
    pub fn set_slot_perm(&self, idx: usize, perm: u64) {
        let ptr = (self.p_slot_base + idx * WGC_SLOT_SIZE) as *const WGCSlot;
        unsafe { (*ptr).perm.write(perm) }
    }

    #[inline]
    pub fn get_slot_cfg(&self, idx: usize) -> u32 {
        let ptr = (self.p_slot_base + idx * WGC_SLOT_SIZE) as *const WGCSlot;
        unsafe { (*ptr).cfg.read() }
    }

    #[inline]
    pub fn set_slot_cfg(&self, idx: usize, cfg: u32) {
        let ptr = (self.p_slot_base + idx * WGC_SLOT_SIZE) as *const WGCSlot;
        unsafe { (*ptr).cfg.write(cfg) }
    }
}

pub fn region_init(
    start: usize,
    size: usize,
    perm: u64,
    allow_overlap: bool,
) -> Result<usize, Error> {
    if size == 0 {
        return Err(Error::Invalid);
    }
    /* overlap detection */
    if allow_overlap == false {
        if detect_region_overlap(start, size) {
            return Err(Error::Overlap);
        }
    }

    /* WG granularity check */
    if (size != usize::MAX) && ((size & (PAGE_SIZE - 1)) != 0) {
        return Err(Error::NotPageGranularity);
    }

    if (start & (PAGE_SIZE - 1)) != 0 {
        return Err(Error::NotPageGranularity);
    }

    /* if the address covers the entire RAM or it's NAPOT */
    if (size == usize::MAX && start == 0)
        || (((size & (size - 1)) == 0) && ((start & (size - 1)) == 0))
    {
        return napot_region_init(start, size, perm, allow_overlap);
    } else {
        return tor_region_init(start, size, perm, allow_overlap);
    }
}

pub fn region_free(region_idx: usize) -> Result<(), Error> {
    if !is_wg_region_valid(region_idx) {
        return Err(Error::Invalid);
    }

    let region = unsafe { REGIONS[region_idx].as_ref().unwrap() };
    let reg_idx = region.index();
    unsafe {
        REGION_VALID[region_idx] = false;
        REG_BITMAP &= !(1 << reg_idx);
    }

    unsafe { REGIONS[region_idx] = None }

    Ok(())
}

pub fn detect_region_overlap(addr: usize, size: usize) -> bool {
    let mut region_overlap = false;
    //let input_end = addr + size;
    let input_end = match addr.checked_add(size) {
        Some(sum) => sum,
        None => usize::MAX,
    };

    (1..WG_MAX_N_REGION).for_each(|index| {
        if is_wg_region_valid(index) {
            let region = unsafe { REGIONS[index].as_ref().unwrap() };
            if !region.allows_overlap() {
                let epm_base = region.addr();
                let epm_size = region.size();

                // Only looking at valid regions, no need to check epm_base+size
                region_overlap |= (epm_base < input_end) && (epm_base + epm_size > addr);
            }
        }
    });

    region_overlap
}

pub fn is_wg_region_valid(region_idx: usize) -> bool {
    region_idx < WG_MAX_N_REGION && unsafe { REGION_VALID[region_idx] }
}

pub const WG_MAX_N_REGION: usize = 256;
// Hardware WGC slot count — REG_BITMAP is usize (64-bit), so this must stay <= 64.
// The actual hardware nslots register value is typically 16.
const WGC_HW_SLOTS: usize = 32;
pub const NWORLDS: u64 = 32;
pub const TRUSTED_WID: u64 = NWORLDS - 1; // WID 31: SM
pub const OS_WID: u64 = NWORLDS - 2;      // WID 30: host OS
pub const DEV_WID: u64 = 29;              // WID 29: dedicated device world
const INIT_VALUE: Option<Region> = None;

/* PMP region getter/setters */
static mut REGIONS: [Option<Region>; WG_MAX_N_REGION] = [INIT_VALUE; WG_MAX_N_REGION];
static mut REG_BITMAP: usize = 1;
// REGION_VALID[i] == true means logical region slot i is in use.
// Slot 0 is reserved (never allocated); handled by starting get_free_region_idx at index 1.
static mut REGION_VALID: [bool; WG_MAX_N_REGION] = [false; WG_MAX_N_REGION];

/// PMP region type
pub struct Region {
    size: usize,
    mode: u32,
    addr: usize,
    perm: u64,
    allow_overlap: bool,
    index: usize,
}

impl Region {
    pub fn mode(&self) -> u32 {
        self.mode
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn allows_overlap(&self) -> bool {
        self.allow_overlap
    }

    pub fn addr(&self) -> usize {
        self.addr
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn is_napot(&self) -> bool {
        self.mode == WGC_CFG_A_NAPOT
    }

    pub fn is_tor(&self) -> bool {
        self.mode == WGC_CFG_A_TOR
    }

    pub fn needs_two_entries(&self) -> bool {
        self.is_tor() && self.index > 0
    }

    pub fn is_napot_all(&self) -> bool {
        self.addr == usize::MIN && self.size == usize::MAX
    }

    pub fn wgaddr_val(&self) -> u64 {
        if self.is_napot_all() {
            return !0;
        } else if self.is_napot() {
            return ((self.addr | (self.size / 2 - 1)) >> 2) as u64;
        } else if self.is_tor() {
            if self.size == usize::MAX {
                return u64::MAX >> 3;
            } else {
                return ((self.addr + self.size) >> 2) as u64;
            }
        }

        0
    }
}

pub fn napot_region_init<'a>(
    start: usize,
    size: usize,
    perm: u64,
    allow_overlap: bool,
) -> Result<usize, Error> {
    //find available wg region idx
    let region_idx = get_free_region_idx();
    if region_idx.is_none() {
        return Err(Error::MaxReached);
    }

    let region_idx = region_idx.unwrap();
    let reg_idx = get_free_reg_idx().unwrap();

    if ((unsafe { REG_BITMAP } & (1 << reg_idx)) != 0) || (reg_idx >= WGC_HW_SLOTS) {
        return Err(Error::MaxReached);
    }

    // initialize the region
    unsafe {
        REGIONS[region_idx] = Some(Region {
            size,
            mode: WGC_CFG_A_NAPOT,
            addr: start,
            perm,
            allow_overlap,
            index: reg_idx,
        });
    };

    unsafe {
        REGION_VALID[region_idx] = true;
        REG_BITMAP |= 1 << reg_idx;
    }

    Ok(region_idx)
}

pub fn get_free_region_idx() -> Option<usize> {
    unsafe {
        for i in 1..WG_MAX_N_REGION {
            // start at 1; slot 0 is reserved
            if !REGION_VALID[i] {
                return Some(i);
            }
        }
    }
    None
}

pub fn get_free_reg_idx() -> Option<usize> {
    return search_rightmost_unset(unsafe { REG_BITMAP }, WGC_HW_SLOTS, 0x1);
}

pub fn get_conseq_free_reg_idx() -> Option<usize> {
    return search_rightmost_unset(unsafe { REG_BITMAP }, WGC_HW_SLOTS, 0x3);
}

fn search_rightmost_unset(bitmap: usize, max: usize, mask: usize) -> Option<usize> {
    let mut i = 0;
    let mut mask = mask;

    while mask < (1 << max) {
        if (!bitmap & mask) == mask {
            return Some(i);
        }

        mask = mask << 1;
        i += 1;
    }

    None
}

pub fn tor_region_init<'a>(
    start: usize,
    size: usize,
    perm: u64,
    allow_overlap: bool,
) -> Result<usize, Error> {
    let region_idx = get_free_region_idx();
    if region_idx.is_none() {
        return Err(Error::MaxReached);
    }

    let reg_idx = get_free_reg_idx().ok_or(Error::MaxReached)?;
    if (unsafe { REG_BITMAP } & (1 << reg_idx)) != 0 || reg_idx >= WGC_HW_SLOTS {
        return Err(Error::MaxReached);
    }

    let region_idx = region_idx.unwrap();

    // TOR end slot only; TOR start is derived implicitly from the preceding NAPOT slot
    // (SM region NAPOT end + 1 = SMM_END), so no explicit TOR-start slot is needed.
    unsafe {
        REGIONS[region_idx] = Some(Region {
            size,
            mode: WGC_CFG_A_TOR,
            addr: start,
            perm,
            allow_overlap,
            index: reg_idx,
        });

        REGION_VALID[region_idx] = true;
        REG_BITMAP |= 1 << reg_idx;
    }

    Ok(region_idx)
}

/// Programs the WGC DRAM slot for `region_idx` with the given raw `perm` bitmap.
fn set_wg_slot(region_idx: usize, perm: u64) -> Result<(), Error> {
    if !is_wg_region_valid(region_idx) {
        return Err(Error::Invalid);
    }
    let region = unsafe { REGIONS[region_idx].as_ref().unwrap() };
    let reg_idx = region.index();
    let dram = WGChecker::new(WGC_DRAM_BASE);
    dram.set_slot_cfg(reg_idx, WGC_CFG_ER | WGC_CFG_EW | WGC_CFG_IR | WGC_CFG_IW | region.mode);
    dram.set_slot_addr(reg_idx, region.wgaddr_val());
    dram.set_slot_perm(reg_idx, perm);
    Ok(())
}

/// Sets a WGC slot for an enclave EPM — grants R+W to `wid` only.
pub fn set_wg_for_enclave(region_idx: usize, wid: usize) -> Result<(), Error> {
    let region = unsafe { REGIONS[region_idx].as_ref() };
    if let Some(r) = region {
        hprintln!("[EPM] mode={} pa=0x{:x} size=0x{:x} wid={}",
            if r.is_napot() { "NAPOT" } else { "TOR" }, r.addr(), r.size(), wid);
    }
    set_wg_slot(region_idx, 3u64 << (wid * 2))
}

/// Clears all WGC DRAM hardware slots that carry permission bits for `wid`.
/// Zeros cfg + addr + perm for each matching slot, so the evicted WID loses
/// access to every region it was mapped to (EPM, SHM, etc.).
pub fn invalidate_wid_in_all_slots(wid: usize) {
    let dram = WGChecker::new(WGC_DRAM_BASE);
    let nslots = dram.get_nslots() as usize;
    let wid_mask = 3u64 << (wid as u64 * 2);
    for slot_idx in 0..=nslots {
        if (dram.get_slot_perm(slot_idx) & wid_mask) != 0 {
            dram.set_slot_cfg(slot_idx, 0);
            dram.set_slot_addr(slot_idx, 0);
            dram.set_slot_perm(slot_idx, 0);
        }
    }
}

/// Programs a WGC slot for a host-enclave SHM — grants R+W to both OS_WID and enclave_wid.
pub fn set_wg_for_host_enclave_shm(region_idx: usize, enclave_wid: usize) -> Result<(), Error> {
    set_wg_slot(region_idx, (3u64 << (OS_WID * 2)) | (3u64 << (enclave_wid as u64 * 2)))
}

/// Programs a WGC slot for a host-only SHM — grants R+W to OS_WID only.
pub fn set_wg_for_host_shm(region_idx: usize) -> Result<(), Error> {
    set_wg_slot(region_idx, 3u64 << (OS_WID * 2))
}

/// Programs a WGC slot with a caller-supplied raw perm bitmap.
/// Used for enclave-enclave SHM where callers compute perm from all sharers' WIDs.
pub fn set_wg_for_shm_perm(region_idx: usize, perm: u64) -> Result<(), Error> {
    set_wg_slot(region_idx, perm)
}

pub fn set_wg(region_idx: usize) -> Result<(), Error> {
    if !is_wg_region_valid(region_idx) {
        return Err(Error::Invalid);
    }

    let region = unsafe { REGIONS[region_idx].as_ref().unwrap() };
    let reg_idx = region.index();

    let dram = WGChecker::new(WGC_DRAM_BASE);
    dram.set_slot_cfg(
        reg_idx,
        WGC_CFG_ER | WGC_CFG_EW | WGC_CFG_IR | WGC_CFG_IW | region.mode,
    );
    dram.set_slot_addr(reg_idx, region.wgaddr_val());
    dram.set_slot_perm(reg_idx, region.perm);

    Ok(())
}

/// Finds and removes the OS_WID setup slot installed for an EPM PA range by prepare_epm_slot.
/// Clears hardware WGC slot and frees the region structure.
/// Matches only regions whose perm has OS_WID bits set to avoid accidentally removing
/// the enclave-WID lazy EPM region (same addr/size but different perm).
pub fn remove_setup_slot_for_pa(pa: usize, size: usize) -> Result<(), Error> {
    let os_wid_mask = 3u64 << (OS_WID * 2);
    for i in 1..WG_MAX_N_REGION {
        if is_wg_region_valid(i) {
            let (raddr, rsize, rperm) = unsafe {
                let r = REGIONS[i].as_ref().unwrap();
                (r.addr(), r.size(), r.perm)
            };
            if raddr == pa && rsize == size && (rperm & os_wid_mask) != 0 {
                reset_wg(i)?;
                region_free(i)?;
                return Ok(());
            }
        }
    }
    Err(Error::Invalid)
}

pub fn reset_wg(region_idx: usize) -> Result<(), Error> {
    if !is_wg_region_valid(region_idx) {
        return Err(Error::Invalid);
    }

    let region = unsafe { REGIONS[region_idx].as_ref().unwrap() };
    let reg_idx = region.index();

    let dram = WGChecker::new(WGC_DRAM_BASE);
    dram.set_slot_cfg(reg_idx, 0);
    dram.set_slot_addr(reg_idx, 0);
    dram.set_slot_perm(reg_idx, 0);

    Ok(())
}

/// Installs the SM region's HW slot as a TOR address anchor at SMM_END with perm=0.
/// hw_bypass (TRUSTED_WID) handles all SM DRAM access; this slot's only role is to
/// anchor the OS TOR start address so that slot[index+1] (OS TOR end) covers [SMM_END, POOL_BASE).
pub fn set_wg_smm_anchor(region_id: usize) -> Result<(), Error> {
    if !is_wg_region_valid(region_id) {
        return Err(Error::Invalid);
    }
    let region = unsafe { REGIONS[region_id].as_ref().unwrap() };
    let reg_idx = region.index();
    let smm_end = (region.addr() + region.size()) as u64;
    let dram = WGChecker::new(WGC_DRAM_BASE);
    dram.set_slot_cfg(reg_idx, WGC_CFG_A_TOR);
    dram.set_slot_addr(reg_idx, smm_end >> 2);
    dram.set_slot_perm(reg_idx, 0);
    Ok(())
}

/// Arms the hw_bypass slot (slot[nslots]) for lazy EPM slot loading.
/// hw_bypass stays A_TOR (covers [0, DRAM_END)) with perm=TRUSTED_WID only and ER=1.
/// - WID31 (M-mode/SM): perm match → full pool access, no bus error.
/// - Any other WID hitting an unmatched pool address: perm miss + ER=1 →
///   CAUSE_FETCH_ACCESS → SM lazy-installs the EPM WGC slot.
/// Call once from osm_init after the OS region slot is installed.
pub fn arm_hw_bypass_for_lazy_load() {
    let dram = WGChecker::new(WGC_DRAM_BASE);
    let nslots = dram.get_nslots() as usize;
    // ER=1 so wrong-WID fetches to pool generate CAUSE_FETCH_ACCESS (lazy load trigger).
    dram.set_slot_cfg(nslots, WGC_CFG_A_TOR | WGC_CFG_ER);
    dram.set_slot_perm(nslots, 3u64 << (TRUSTED_WID * 2));
}

pub fn display() {
    let dram = WGChecker::new(WGC_DRAM_BASE);
    let nslots = dram.get_nslots();
    let errcause = dram.get_errcause();
    let erraddr = dram.get_erraddr();

    hprintln!(
        "[WGC] mlwid={:#x} mwiddeleg={:#x} nslots={} errcause={:#x} erraddr={:#x}",
        csr_read_custom!(0x390),
        csr_read_custom!(0x748),
        nslots,
        errcause,
        erraddr
    );
    hprintln!("[WGC] {:>4}  {:>5}  {:>18}  {:>18}  ER EW IR IW | perm",
        "slot", "mode", "addr(raw)", "pa");
    hprintln!("[WGC] --------------------------------------------------------------------------------");

    for idx in 0..=(nslots as usize) {
        let raw_addr = dram.get_slot_addr(idx);
        let cfg = dram.get_slot_cfg(idx);
        let perm = dram.get_slot_perm(idx);

        let a = cfg & 0x3;
        let mode_str = match a {
            0 => "OFF  ",
            1 => "TOR  ",
            2 => "NA4  ",
            3 => "NAPOT",
            _ => "?????",
        };
        let er = if (cfg >> 8) & 1 != 0 { 1u32 } else { 0 };
        let ew = if (cfg >> 9) & 1 != 0 { 1u32 } else { 0 };
        let ir = if (cfg >> 10) & 1 != 0 { 1u32 } else { 0 };
        let iw = if (cfg >> 11) & 1 != 0 { 1u32 } else { 0 };

        let pa = (raw_addr as u64) << 2;

        let label = if idx == 0 {
            " (slot0)"
        } else if idx == nslots as usize {
            " (hw_bypass)"
        } else {
            ""
        };

        hprintln!("[WGC] {:>4}  {}  {:>#18x}  {:>#18x}  {}  {}  {}  {} | {:#018x}{}",
            idx, mode_str, raw_addr, pa, er, ew, ir, iw, perm, label);
    }
    hprintln!("[WGC] --------------------------------------------------------------------------------");
}

