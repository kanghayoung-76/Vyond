use crate::isolator::PAGE_SIZE;
use crate::Error;
use volatile_register::{RO, RW};

/// General WGC
pub const WGC_SLOT_OFFSET: usize = 0x20;
pub const WGC_SLOT_SIZE: usize = 0x20;

pub const WGC_CFG_A_OFF: u32 = 0x0;
pub const WGC_CFG_A_TOR: u32 = 0x1;
pub const WGC_CFG_A_NA4: u32 = 0x2;
pub const WGC_CFG_A_NAPOT: u32 = 0x3;
pub const WGC_CFG_ER: u32 = 1 << 8;
pub const WGC_CFG_EW: u32 = 1 << 9;
pub const WGC_CFG_IR: u32 = 1 << 10;
pub const WGC_CFG_IW: u32 = 1 << 11;
pub const WGC_CFG_L: u32 = 1 << 31;

pub const WGC_ERRCAUSE_R_SHIFT: u8 = 8;
pub const WGC_ERRCAUSE_W_SHIFT: u8 = 9;
pub const WGC_ERRCAUSE_BE_SHIFT: u8 = 62;
pub const WGC_ERRCAUSE_IP_SHIFT: u8 = 63;

// 2 bits per world; mask exactly NWORLDS worlds so no reserved (nonexistent-world)
// bits are set. At NWORLDS=8 this is 0xFFFF. Matches the HW's widWidth=log2(NWORLDS).
pub const WGC_ALL_PERM: usize = ((1u128 << (NWORLDS * 2)) - 1) as usize;

// TODO: read platform specific configs from dtb
pub const WGC_DRAM_BASE: usize = 0x600_0000;
pub const WGC_FLASH_BASE: usize = 0x600_1000;
pub const WGC_UART_BASE: usize = 0x600_2000;
// WGChecker MMIO layout of the Vyond-main WGRocket8VCU118 bitstream (from its .dts):
// memport(DRAM)@0x6000000, plic@0x6003000, bootrom@0x6004000, periphery@0x6005000.
// (No FLASH/UART checker exist in that bitstream.)
pub const WGC_PLIC_BASE: usize = 0x600_3000;
pub const WGC_BOOTROM_BASE: usize = 0x600_4000;
pub const WGC_PERIPHERY_BASE: usize = 0x600_5000;
const DRAM_BASE: usize = 0x8000_0000;
const FLASH_BASE: usize = 0x20000000;
const FLASH_SIZE: usize = 0x4000000;
const UART_BASE: usize = 0x10000000;
const UART_SIZE: usize = 0x100;

/// WGC for Memory
#[repr(C)]
pub struct WGCRegisterBlock {
    pub vendor: RO<u32>,
    pub impid: RO<u32>,
    pub nslots: RO<u32>,
    reserved: RO<u32>,
    pub errcause: RW<u64>,
    pub erraddr: RW<u64>,
    //pub slots: [WGCSlot; NUM_N_SLOTS + 1],     // FIXME: this does not work.. why?
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

    pub fn from(base: usize, size: usize) -> Result<WGChecker, Error> {
        if FLASH_BASE <= base && base + size < FLASH_BASE + FLASH_SIZE {
            Ok(WGChecker::new(WGC_FLASH_BASE))
        } else if UART_BASE <= base && base + size < UART_BASE + UART_SIZE {
            Ok(WGChecker::new(WGC_UART_BASE))
        } else if DRAM_BASE <= base {
            Ok(WGChecker::new(WGC_DRAM_BASE))
        } else {
            Err(Error::Invalid)
        }
    }

    #[inline]
    pub fn get_vendor(&self) -> u32 {
        self.p_wgc.vendor.read()
    }

    #[inline]
    pub fn get_impid(&self) -> u32 {
        self.p_wgc.impid.read()
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
    pub fn set_errcause(&self, wid: u8, r: bool, w: bool, be: bool, ip: bool) {
        unsafe {
            self.p_wgc.errcause.modify(|v| {
                v | ((wid as u64)
                    | (r as u64) << WGC_ERRCAUSE_R_SHIFT
                    | (w as u64) << WGC_ERRCAUSE_W_SHIFT
                    | (be as u64) << WGC_ERRCAUSE_BE_SHIFT
                    | (ip as u64) << WGC_ERRCAUSE_IP_SHIFT)
            })
        }
    }

    #[inline]
    pub fn get_erraddr(&self) -> u64 {
        self.p_wgc.erraddr.read()
    }

    #[inline]
    pub fn set_erraddr(&self, addr: u64) {
        unsafe { self.p_wgc.erraddr.write(addr) }
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
    let reg_idx = if region.is_tor() {
        region.index() + 1
    } else {
        region.index()
    };
    unsafe {
        REGION_VALID[region_idx] = false;
        REG_BITMAP &= !(1 << reg_idx);
    }
    if region.needs_two_entries() {
        unsafe { REG_BITMAP &= !(1 << (reg_idx - 1)) };
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
pub const NWORLDS: u64 = 8;
pub const TRUSTED_WID: u64 = NWORLDS - 1; // WID 7: SM
pub const OS_WID: u64 = NWORLDS - 2;      // WID 6: host OS
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

/// NAPOT region at a caller-chosen HW slot (reg_idx). WGChecker priority = lowest-index
/// hitting slot wins, so an OS catch-all placed at the HIGHEST index (lowest priority)
/// lets a per-enclave EPM slot at a lower index override it -> isolation.
/// Rust set_slot(idx) writes HW slot idx directly; HW nSlots=8 -> reg_idx in 0..7 valid.
pub fn napot_region_init_at(
    start: usize,
    size: usize,
    perm: u64,
    allow_overlap: bool,
    reg_idx: usize,
) -> Result<usize, Error> {
    let region_idx = match get_free_region_idx() {
        Some(i) => i,
        None => return Err(Error::MaxReached),
    };
    if ((unsafe { REG_BITMAP } & (1 << reg_idx)) != 0) || (reg_idx >= WGC_HW_SLOTS) {
        return Err(Error::MaxReached);
    }
    unsafe {
        REGIONS[region_idx] = Some(Region {
            size,
            mode: WGC_CFG_A_NAPOT,
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

    let reg_idx = get_conseq_free_reg_idx().unwrap();
    if ((unsafe { REG_BITMAP } & (1 << reg_idx)) != 0)
        || ((unsafe { REG_BITMAP } & (1 << reg_idx + 1)) != 0)
        || (reg_idx + 1 > WGC_HW_SLOTS)
    {
        return Err(Error::MaxReached);
    }

    let region_idx = region_idx.unwrap();

    // FIXME: looks incorrect logic below.
    // initialize the region
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
    if reg_idx > 0 {
        unsafe { REG_BITMAP |= 1 << (reg_idx + 1) };
    }

    Ok(region_idx)
}

/// Programs the WGC DRAM slot for `region_idx` with the given raw `perm` bitmap.
fn set_wg_slot(region_idx: usize, perm: u64) -> Result<(), Error> {
    if !is_wg_region_valid(region_idx) {
        return Err(Error::Invalid);
    }
    let region = unsafe { REGIONS[region_idx].as_ref().unwrap() };
    let reg_idx = if region.is_tor() { region.index() + 1 } else { region.index() };
    let dram = WGChecker::new(WGC_DRAM_BASE);
    if region.is_tor() {
        dram.set_slot_cfg(reg_idx - 1, 0x0);
        dram.set_slot_addr(reg_idx - 1, (region.addr() >> 2) as u64);
        dram.set_slot_perm(reg_idx - 1, 0);
    }
    dram.set_slot_cfg(reg_idx, WGC_CFG_ER | WGC_CFG_EW | WGC_CFG_IR | WGC_CFG_IW | region.mode);
    dram.set_slot_addr(reg_idx, region.wgaddr_val());
    dram.set_slot_perm(reg_idx, perm);
    Ok(())
}

/// Sets a WGC slot for an enclave EPM — grants R+W to the owning enclave `wid` AND
/// OS_WID (+ M-mode WID, forced on by HW). Per-enclave isolation: other enclave WIDs
/// are denied; the OS may reach enclave memory (threat model), which also lets the host's
/// leftover WID-tagged eapp-load lines release across the boundary (no coherence deadlock).
pub fn set_wg_for_enclave(region_idx: usize, wid: usize) -> Result<(), Error> {
    // [실험 2026-07-27] EPM 슬롯을 모든 WID에 개방한다. enclave 첫 명령 fetch 정지가
    // "체커가 어떤 WID를 거부해서"인지 확인하는 반증 실험: 다 열었는데도 멈추면 권한 계통은
    // 원인이 아니고 코어↔L2 I-포트의 WID 처리(RTL)만 남는다. 진행되면 fetch가 제시하는 WID가
    // 1도 OS도 아니라는 뜻 → WID를 하나씩 켜서 이분탐색. 결론 나면 이 플래그는 되돌린다.
    if EPM_SLOT_ALL_PERM {
        return set_wg_slot(region_idx, WGC_ALL_PERM as u64);
    }
    set_wg_slot(region_idx, (3u64 << (wid * 2)) | (3u64 << (OS_WID * 2)))
}

/// 위 실험 플래그. 정상 동작(격리 유지)은 false.
/// 2026-07-27 실측: true로 열어도 enclave 정지는 그대로 → 권한 계통은 원인 아님. 원복함.
const EPM_SLOT_ALL_PERM: bool = false;

/// Clears all WGC DRAM hardware slots that carry permission bits for `wid`.
/// Zeros cfg + addr + perm for each matching slot, so the evicted WID loses
/// access to every region it was mapped to (EPM, SHM, etc.).
pub fn invalidate_wid_in_all_slots(wid: usize) {
    let dram = WGChecker::new(WGC_DRAM_BASE);
    // Do NOT trust get_nslots(): on the Vyond-main WGRocket8VCU118 bitstream the DRAM
    // (memport) checker's nslots register returns a bogus large value, so an unbounded
    // loop walks past this checker's 0x1000 MMIO page into the absent FLASH checker
    // (0x6001000) -> load access fault (observed: mcause=5 mtval=0x6001008).
    // Slot i lives at WGC_DRAM_BASE + 0x20 + i*0x20, so clamp to WGC_HW_SLOTS to stay
    // inside the page. Empty/unused slots read perm=0 and are skipped, so over-scanning
    // the real slot count is harmless.
    let nslots = core::cmp::min(dram.get_nslots() as usize, WGC_HW_SLOTS);
    let wid_mask = 3u64 << (wid as u64 * 2);
    // NOTE: this zaps the WHOLE slot (cfg/addr/perm=0) — do NOT change to "clear only
    // this WID's bits". FPGA 실측(2026-07-15): preserving other bits leaves the slot
    // ACTIVE with a restricted perm, which then DENIES the host (OS_WID) for that whole
    // address range instead of letting it fall through to the permissive catch-all (osm
    // all-perm). Result was a ~4800-deep host ACCESS FAULT storm (eid=0, kernel VAs).
    // Fully disabling the slot lets the range fall back to the catch-all -> host OK.
    let all_perm = WGC_ALL_PERM as u64;
    for slot_idx in 0..=nslots {
        let perm = dram.get_slot_perm(slot_idx);
        // OS catch-all(= 모든 world 허용, boot 시 region_init(0, MAX, WGC_ALL_PERM))은 건드리지
        // 않는다. 이 슬롯도 enclave WID 비트를 갖고 있어서 예전에는 함께 0으로 지워졌고, 그
        // 결과 destroy 직후 호스트(OS_WID)를 커버하는 슬롯이 사라져 mret 복귀 직후 정지했다.
        // (FPGA 실측 2026-07-27: destroy 후 슬롯 덤프에 catch-all 부재, 드라이버의 destroy
        //  ecall이 영영 반환되지 않음 = 셸 wedge.)
        if (perm & all_perm) == all_perm {
            continue;
        }
        if (perm & wid_mask) != 0 {
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
    let reg_idx = if region.is_tor() {
        region.index() + 1
    } else {
        region.index()
    };

    let dram = WGChecker::new(WGC_DRAM_BASE);
    if region.is_tor() {
        dram.set_slot_cfg(reg_idx - 1, 0x0);
        dram.set_slot_addr(reg_idx - 1, (region.addr() >> 2) as u64);
        dram.set_slot_perm(reg_idx - 1, 0); // RW for w3 only
    }
    dram.set_slot_cfg(
        reg_idx,
        WGC_CFG_ER | WGC_CFG_EW | WGC_CFG_IR | WGC_CFG_IW | region.mode,
    );
    dram.set_slot_addr(reg_idx, region.wgaddr_val());
    dram.set_slot_perm(reg_idx, region.perm); // RW for w3 only

    Ok(())
}

pub fn reset_wg(region_idx: usize) -> Result<(), Error> {
    if !is_wg_region_valid(region_idx) {
        return Err(Error::Invalid);
    }

    let region = unsafe { REGIONS[region_idx].as_ref().unwrap() };
    let reg_idx = if region.is_tor() {
        region.index() + 1
    } else {
        region.index()
    };

    let dram = WGChecker::new(WGC_DRAM_BASE);
    if region.is_tor() {
        dram.set_slot_cfg(reg_idx - 1, 0);
        dram.set_slot_addr(reg_idx - 1, 0);
        dram.set_slot_perm(reg_idx - 1, 0); // RW for w3 only
    }
    dram.set_slot_cfg(reg_idx, 0);
    dram.set_slot_addr(reg_idx, 0);
    dram.set_slot_perm(reg_idx, 0); // RW for w3 only

    Ok(())
}

pub fn display() {
    let dram = WGChecker::new(WGC_DRAM_BASE);
    let vendor = dram.get_vendor();
    let impid = dram.get_impid();
    // Clamp: the bitstream's DRAM checker nslots register is bogus (see
    // invalidate_wid_in_all_slots); an unbounded scan faults off the MMIO page.
    let nslots = core::cmp::min(dram.get_nslots() as usize, WGC_HW_SLOTS) as u32;
    let errcause = dram.get_errcause();
    let erraddr = dram.get_erraddr();

    for idx in 0..(nslots + 1) {
        let addr = dram.get_slot_addr(idx as usize);
        let cfg = dram.get_slot_cfg(idx as usize);
        let perm = dram.get_slot_perm(idx as usize);

    }
}

pub fn display_regions() {
    unsafe {
    }
    for rid in 0..WG_MAX_N_REGION {
        unsafe {
            if let Some(region) = &REGIONS[rid] {
            }
        }
    }
    //display();
}
