#[cfg(any(feature = "isolator_pmp", feature = "isolator_hybrid"))]
use crate::pmp;
#[cfg(any(feature = "isolator_wg", feature = "isolator_hybrid"))]
use crate::wg;
use crate::{Error, OnceCell};
use core::sync::atomic::compiler_fence;
use core::sync::atomic::Ordering;

pub const SMM_BASE: usize = 0x80000000;
pub const SMM_SIZE: usize = 0x200000;
pub const OSM_BASE: usize = SMM_BASE + SMM_SIZE;

unsafe impl<T> Sync for OnceCell<T> {}

pub const PAGE_SIZE: usize = 4096;

static SM_INIT_DONE: OnceCell<bool> = OnceCell::new();
static SM_REGION_ID: OnceCell<usize> = OnceCell::new();
static OS_REGION_ID: OnceCell<usize> = OnceCell::new();

pub fn os_region_id() -> usize {
    *OS_REGION_ID.get().unwrap()
}

pub fn sm_region_id() -> usize {
    *SM_REGION_ID.get().unwrap()
}

pub fn sm_init_done() {
    SM_INIT_DONE.set(true);
}

pub fn sm_wait_for_completion() {
    while !SM_INIT_DONE.get().unwrap() {
        compiler_fence(Ordering::Release);
    }
}

/// 성능 카운터를 S/U 모드에서 읽을 수 있게 열고, 실제로 돌게 만든다.
///
/// 2026-07-28 실측: `mcountinhibit = 0x5`(CY·IR 정지)이고 U-mode `rdcycle`이 illegal
/// instruction으로 트랩했다(paper_eval 러너 3종 중 2종이 여기서 SIGILL). OpenSBI가
/// sbi_hart.c에서 mcounteren/-inhibit을 세팅하려 하지만 priv-version 게이트에 걸려
/// 반영되지 않은 상태다. 벤치마크는 전부 사이클 측정이 전제라 SM에서 직접 켠다.
///   mcountinhibit = 0xFFFFFFF8 : hpmcounter3+ 는 정지, CY/TM/IR 은 동작
///   mcounteren    = !0         : S-mode 가 모든 카운터 접근 가능
///   scounteren    = 7          : U-mode 가 CY/TM/IR 접근 가능 (Linux 가 이후 관리)
#[cfg(any(feature = "isolator_wg", feature = "isolator_hybrid"))]
fn enable_perf_counters() {
    csr_write_custom!(0x320, 0xFFFF_FFF8usize); // mcountinhibit
    csr_write_custom!(0x306, usize::MAX);       // mcounteren
    csr_write_custom!(0x106, 7usize);           // scounteren
}

/// Opens all peripheral WGC checker last slots to all WIDs and sets mlwid = OS_WID.
/// Required before any WG-based enclave isolation can be configured.
#[cfg(any(feature = "isolator_wg", feature = "isolator_hybrid"))]
fn init_peripheral_wgc() {
    csr_write_custom!(0x390, wg::OS_WID); // Set mlwid
    // The Vyond-main WGRocket8VCU118 bitstream exposes PLIC/BOOTROM/PERIPHERY
    // WGCheckers (0x6003000/0x6004000/0x6005000), NOT FLASH/UART (0x6001000/0x6002000).
    // Accessing the absent FLASH/UART checker faults (observed: mtval=0x6001008).
    // Open the present checkers NAPOT-all for all worlds (matches Vyond-main SM).
    // [2026-07-29] WGC_SLOT_OFFSET을 0x40으로 바로잡아 idx == HW slot 이 되었으므로,
    // 최우선 슬롯인 HW slot 0에 직접 깐다(예전에는 idx=1이 HW slot0을 가리켰다).
    let perm_all = wg::WGC_ALL_PERM as u64;
    let cfg_napot_all = wg::WGC_CFG_ER | wg::WGC_CFG_EW | wg::WGC_CFG_A_NAPOT;
    for base in [wg::WGC_PLIC_BASE, wg::WGC_BOOTROM_BASE, wg::WGC_PERIPHERY_BASE] {
        let wgc = wg::WGChecker::new(base);
        wgc.set_slot_addr(0, !0u64 >> 1); // NAPOT, covers all
        wgc.set_slot_perm(0, perm_all);
        wgc.set_slot_cfg(0, cfg_napot_all);
    }
}

pub fn smm_init<'a>() -> Result<(), Error> {
    #[cfg(feature = "isolator_pmp")]
    {
        let region = pmp::region_init(SMM_BASE, SMM_SIZE, pmp::Priority::Top, false)?;
        SM_REGION_ID.set(region);
        Ok(())
    }
    #[cfg(feature = "isolator_wg")]
    {
        init_peripheral_wgc();
        enable_perf_counters();
        let region = wg::region_init(SMM_BASE, SMM_SIZE, 3 << (wg::TRUSTED_WID * 2), false)?;
        wg::set_wg(region)?;
        SM_REGION_ID.set(region);
        Ok(())
    }
    #[cfg(feature = "isolator_hybrid")]
    {
        init_peripheral_wgc();
        let region = wg::region_init(SMM_BASE, SMM_SIZE, 3 << (wg::TRUSTED_WID * 2), false)?;
        wg::set_wg(region)?;
        Ok(())
    }
}

pub fn osm_init<'a>() -> Result<(), Error> {
    #[cfg(feature = "isolator_pmp")]
    {
        let region = pmp::region_init(0, usize::MAX, pmp::Priority::Bottom, true)?;
        OS_REGION_ID.set(region);
        Ok(())
    }

    #[cfg(feature = "isolator_wg")]
    {
        // FPGA milestone-1 (boot): use Vyond-main's proven NAPOT catch-all OS region.
        //
        // The bounded-TOR OS region below (start=0x80200000) drives set_wg's TOR path,
        // which on the WGRocket8VCU118 bitstream disturbs the SMM slot so that the M-mode
        // trap handler's own instruction fetch (WID7, inside SMM) gets denied right after
        // the kernel jump -> illegal-instruction loop at _trap_handler (csrrw mscratch,
        // observed mcause=2 mtval=0 mepc=0x800004f0). Vyond-main deliberately avoids TOR:
        // NAPOT-all (start=0) takes the NAPOT path and does NOT clobber the SMM slot; the
        // lower-index SMM slot wins priority so SM stays isolated, and this catch-all grants
        // every other world access elsewhere.
        //
        // PER-ENCLAVE ISOLATION (catch-all all-perm @ LOWEST priority):
        // The catch-all grants every world but sits at HW slot 7 (lowest priority). Each
        // enclave's EPM slot (owner_WID + OS_WID, programmed eager at a HIGHER priority)
        // OVERRIDES it for that EPM, so:
        //   - enclave B (WID_B) hitting enclave A's EPM: A's EPM slot wins and grants only
        //     WID_A + OS, so WID_B is DENIED -> per-enclave EPM isolation holds even though
        //     the catch-all would have granted WID_B (it loses on priority).
        //   - the enclave still runs: for any address WITHOUT a dedicated slot, the catch-all
        //     grants its WID as a fallback (this is why an OS+TRUSTED-only catch-all hung —
        //     the enclave touches regions beyond EPM/UTM that need a WID grant).
        //   - OS (WID6) may reach EPM (threat model allows it; also lets the host's leftover
        //     WID-tagged eapp-load lines release across the boundary -> no coherence deadlock).
        // BOOT-RESTORE: revert to Vyond-main's proven catch-all. The napot_region_init_at
        // (OS catch-all forced to HW slot 7 for per-enclave override) is unproven WIP that
        // hangs cold-boot on this bitstream; region_init auto-picks a slot on the NAPOT path
        // (does not clobber the SMM slot). Re-apply the slot-7 isolation experiment only
        // after boot is confirmed.
        // [2026-07-28] catch-all을 HW 슬롯 7(최하위 우선순위)에 고정한다.
        // WGChecker는 "히트한 슬롯 중 가장 낮은 인덱스가 이긴다"(WGCCtrl.scala Checker의
        // reverse+foldLeft). 자동 할당은 낮은 번호부터 주므로 catch-all이 슬롯 2에 앉았고,
        // 그러면 슬롯 3+에 프로그램되는 per-enclave EPM 슬롯을 all-perm이 매번 덮어써서
        // 격리가 성립하지 않았다(2026-07-28 슬롯 덤프로 확인). 슬롯 7로 내려야 EPM 슬롯이 이긴다.
        const OS_CATCHALL_SLOT: usize = 7;
        let region = wg::napot_region_init_at(
            0, usize::MAX, wg::WGC_ALL_PERM as u64, true, OS_CATCHALL_SLOT)?;
        wg::set_wg(region)?;
        OS_REGION_ID.set(region);
        Ok(())

        // --- B's original bounded-TOR design (kept for restoration) ---
        // let region = wg::region_init(
        //     SMM_BASE + SMM_SIZE, usize::MAX, (3 << (wg::OS_WID * 2)) | 3, false)?;
        // wg::set_wg(region);
        // OS_REGION_ID.set(region);
        // Ok(())
    }
    #[cfg(feature = "isolator_hybrid")]
    {
        let region = wg::region_init(OSM_BASE, usize::MAX, 3 << (wg::OS_WID * 2) | 3, false)?;
        wg::set_wg(region)?;

        let region = pmp::region_init(0, usize::MAX, pmp::Priority::Bottom, true)?;
        OS_REGION_ID.set(region);
        Ok(())
    }
}

pub fn region_init(start: usize, size: usize, eid: usize, shared: bool) -> Result<usize, Error> {
    #[cfg(feature = "isolator_pmp")]
    {
        if shared {
            pmp::region_init(start, size, pmp::Priority::Bottom, false)
        } else {
            pmp::region_init(start, size, pmp::Priority::Any, false)
        }
    }
    #[cfg(feature = "isolator_wg")]
    {
        // TODO(slot-virt): pWID assignment is temporary (eid+1); replace with dynamic LRU table.
        // WID 0: legacy/untrusted, WID 1-5: enclaves, WID 6: OS, WID 7: SM.
        // Only EPM regions (shared=false) use eid-derived WIDs; SHM regions use OS_WID/custom perms.
        let region_idx = wg::region_init(start, size, 3 << ((eid + 1) * 2), true)?;
        // WGC slot virtualization: do NOT write EPM slot to hardware at create time.
        // The ACCESS FAULT handler loads it on-demand when the enclave first accesses EPM.
        // wg::set_wg(region_idx)?;
        Ok(region_idx)
    }
    #[cfg(feature = "isolator_hybrid")]
    {
        if shared {
            pmp::region_init(start, size, pmp::Priority::Bottom, false)
        } else {
            pmp::region_init(start, size, pmp::Priority::Any, false)
        }
    }
}

pub fn set_isolator(region_idx: usize, destroy: bool) -> Result<(), Error> {
    #[cfg(feature = "isolator_pmp")]
    {
        pmp::set_pmp(region_idx, pmp::PMP_ALL_PERM, destroy)
    }
    #[cfg(feature = "isolator_wg")]
    {
        if destroy {
            wg::reset_wg(region_idx)
        } else {
            wg::set_wg(region_idx)
        }
    }
    #[cfg(feature = "isolator_hybrid")]
    {
        pmp::set_pmp(region_idx, pmp::PMP_ALL_PERM, destroy)
    }
}

// pub fn set_isolator(region_idx: usize, perm_conf: PermConfig) Result<(), Error> {
//     #[cfg(feature = "isolator_pmp")]
//     {
//         pmp::set_keystone(region_idx, pmp::PMP_ALL_PERM)
//     }
//     #[cfg(feature = "isolator_wg")]
//     {
//         Ok(())
//         //wg::set_wg(region_idx)
//     }
//     #[cfg(feature = "isolator_hybrid")]
//     {
//         pmp::set_keystone(region_idx, pmp::PMP_ALL_PERM)
//     }
// }

/// Sets a WGC slot for an enclave EPM region using a dynamically assigned WID.
/// Only available with the `isolator_wg` or `isolator_hybrid` features.
/// Called from the ACCESS FAULT handler (load_enclave_slot) during slot virtualization.
#[cfg(any(feature = "isolator_wg", feature = "isolator_hybrid"))]
pub fn set_isolator_with_wid(region_idx: usize, wid: usize) -> Result<(), Error> {
    wg::set_wg_for_enclave(region_idx, wid)
}

/// Programs a SHM WGC slot accessible by both host (OS_WID) and the given enclave WID.
/// Called from the ACCESS FAULT handler when an enclave first touches a SHM region.
#[cfg(any(feature = "isolator_wg", feature = "isolator_hybrid"))]
pub fn set_shm_for_host_enclave(region_idx: usize, enclave_wid: usize) -> Result<(), Error> {
    wg::set_wg_for_host_enclave_shm(region_idx, enclave_wid)
}

/// Programs a SHM WGC slot accessible by host (OS_WID) only.
/// Called once at create_shared_mem time so the host can populate the buffer
/// before the enclave runs.
#[cfg(any(feature = "isolator_wg", feature = "isolator_hybrid"))]
pub fn set_shm_host_only(region_idx: usize) -> Result<(), Error> {
    wg::set_wg_for_host_shm(region_idx)
}

/// Programs a SHM WGC slot with a raw perm bitmap (used for enclave-enclave SHM).
#[cfg(any(feature = "isolator_wg", feature = "isolator_hybrid"))]
pub fn set_shm_perm(region_idx: usize, perm: u64) -> Result<(), Error> {
    wg::set_wg_for_shm_perm(region_idx, perm)
}

pub fn reset_isolator(region_idx: usize, destroy: bool) -> Result<(), Error> {
    #[cfg(feature = "isolator_pmp")]
    {
        pmp::set_pmp(region_idx, pmp::PMP_NO_PERM, destroy)
    }
    #[cfg(feature = "isolator_wg")]
    {
        wg::reset_wg(region_idx)
    }
    #[cfg(feature = "isolator_hybrid")]
    {
        pmp::set_pmp(region_idx, pmp::PMP_NO_PERM, destroy)
    }
}

pub fn region_free(region_idx: usize) -> Result<(), Error> {
    #[cfg(feature = "isolator_pmp")]
    {
        pmp::region_free(region_idx)
    }
    #[cfg(feature = "isolator_wg")]
    {
        wg::region_free(region_idx)
    }
    #[cfg(feature = "isolator_hybrid")]
    {
        pmp::region_free(region_idx)
    }
}

pub fn display_isolator() {
    #[cfg(feature = "isolator_pmp")]
    {
        pmp::display()
    }
    #[cfg(feature = "isolator_wg")]
    {
        //enclave::display();
        //wg::display_regions();
    }
    #[cfg(feature = "isolator_hybrid")]
    {
        //pmp::display();
        //wg::display()
    }
}

pub fn update() -> Result<(), Error> {
    #[cfg(feature = "isolator_pmp")]
    {
        pmp::reset(pmp::PMP_N_REG);
        let _ = pmp::set_pmp(sm_region_id(), pmp::PMP_NO_PERM, false);
        let _ = pmp::set_pmp(os_region_id(), pmp::PMP_ALL_PERM, false);
        Ok(())
    }
    #[cfg(feature = "isolator_wg")]
    {
        Ok(())
    }
    #[cfg(feature = "isolator_hybrid")]
    {
        pmp::reset(pmp::PMP_N_REG);
        pmp::set_pmp(os_region_id(), pmp::PMP_ALL_PERM, false)?;
        Ok(())
    }
}
