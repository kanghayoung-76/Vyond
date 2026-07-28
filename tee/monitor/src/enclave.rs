use crate::cpu;
use crate::dbg;

// FPGA teardown-wedge diagnostic: the C side (opensbi) prints "[SM] ..." over the real
// UART via sbi_puts — this is NOT semihosting, so it is safe on the debugger-less FPGA.
// We call it directly to emit destroy_enclave phase markers; the LAST marker printed
// before a silent hang localizes the wedge. Remove once the wedge is fixed.
extern "C" {
    fn sbi_puts(s: *const u8);
}
#[inline(always)]
fn sm_mark(s: &[u8]) {
    // s must be NUL-terminated (b"...\0").
    unsafe { sbi_puts(s.as_ptr()); }
}

// [MARKER] usize를 hex로 출력. label은 NUL 없는 바이트열(b"...")로 넘긴다.
#[inline(never)]
fn sm_mark_hex(label: &[u8], val: usize) {
    let mut buf = [0u8; 64];
    let mut i = 0usize;
    while i < label.len() && i < 40 { buf[i] = label[i]; i += 1; }
    buf[i] = b'0'; i += 1;
    buf[i] = b'x'; i += 1;
    let mut shift: i32 = 60;
    while shift >= 0 {
        let nib = ((val >> shift) & 0xf) as u8;
        buf[i] = if nib < 10 { b'0' + nib } else { b'a' + (nib - 10) };
        i += 1;
        shift -= 4;
    }
    buf[i] = b'\n'; i += 1;
    buf[i] = 0; // NUL 종단
    unsafe { sbi_puts(buf.as_ptr()); }
}

// [PROBE2] M-mode에서 WID를 잠깐 enclave WID로 바꾸고 addr을 1워드 읽어본다.
// enclave의 WID=1 fetch가 L2까지 접수(reqw1 증가)되고도 응답이 안 오는 게 확인됐으므로,
// "WID1 데이터 경로 자체가 죽었는지"를 fetch와 분리해 확인하는 용도.
//
// 중요: RTL(worldguard CSR.scala:355) io.wid = prv==M ? mwid : prv==S ? mlwid : ...
// → M-mode 접근은 mlwid가 아니라 **mwid(0x391)**를 쓴다. 그래서 여기서 바꿔야 하는 건
// mwid다 (mlwid를 건드리면 M-mode 트래픽 WID는 그대로 7이라 아무것도 검증 못 함 — 2026-07-27 실측).
// 주의: mwid=WID1인 동안 SM 자신의 fetch/스택/UART 접근도 WID1로 나간다. 그래서
// 임계구간을 csrw-lw-csrw 3명령으로 최소화하고, 호출 전/후에만 UART로 찍는다.
#[inline(never)]
unsafe fn probe_read_at_wid(addr: usize, wid: usize, restore: usize) -> usize {
    let v: usize;
    core::arch::asm!(
        "fence",
        "csrw 0x391, {w}",
        "lw   {v}, 0({a})",
        "csrw 0x391, {r}",
        "fence",
        w = in(reg) wid,
        a = in(reg) addr,
        r = in(reg) restore,
        v = out(reg) v,
        options(nostack)
    );
    v
}

// [PROBE2-D] WID를 enclave WID로 바꾼 뒤 fence.i로 I$를 무효화해서, 이어지는 명령들이
// 반드시 그 WID로 **재fetch**되게 만든다. enclave가 멈추는 지점이 첫 명령 fetch이므로,
// 여기서 멈추면 "WID1 명령 fetch 경로가 죽었다"는 직접 증거(코어/L1-I 쪽)가 된다.
// 통과하면 M-mode fetch는 WID1로 잘 되는 것 → S-mode/진입 경로 쪽으로 범위가 좁혀진다.
#[inline(never)]
unsafe fn probe_fetch_at_wid(wid: usize, restore: usize) {
    core::arch::asm!(
        "csrw 0x391, {w}",
        "fence.i",
        "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop",
        "csrw 0x391, {r}",
        "fence.i",
        w = in(reg) wid,
        r = in(reg) restore,
        options(nostack)
    );
}

// switch_to_host 진입 시 enclave PC(regs.mepc) 추적. 값이 바뀔 때만 출력해 flood 방지.
// stuck  → "mepc CHANGE" 한 줄 뒤 조용(같은 PC 반복). crawl → 증가하는 값이 계속 출력.
// [MK] 엔트리마다 찍는 hot 마커 on/off. 115200 UART로 엔트리당 ~100자를 뱉으면 한 엔트리가
// ~10ms를 먹어서, Linux 틱(1~4ms)이 매번 이미 만료된 상태로 진입하게 된다(관측자 효과).
// 진행률 측정을 할 때는 반드시 false. 흐름 추적이 필요할 때만 true.
pub const VERBOSE_MK: bool = false;

static mut LAST_MEPC: usize = 0xdead_beef;
static mut STH_CNT: usize = 0;

// [MK] enclave 진입 직전에 찍어두는 값들 (switch_to_host에서 차분 계산용).
static mut ENTRY_MINSTRET: usize = 0;
static mut ENTRY_MTIME: usize = 0;
static mut ENTRY_MTIMECMP: usize = 0;

// CLINT (Rocket/VCU118 기본 배치): mtimecmp hart0 = base+0x4000, mtime = base+0xbff8.
const CLINT_BASE: usize = 0x0200_0000;
const CLINT_MTIMECMP: usize = CLINT_BASE + 0x4000;
const CLINT_MTIME: usize = CLINT_BASE + 0xbff8;

use crate::encoding::*;
use crate::isolator;
use crate::shm;
use crate::spinlock::SpinLock;
use crate::thread;
use crate::trap::TrapFrame;
use crate::Error;

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

// Per-hart: context of the Linux task T that was running when IPI fired (always S-mode path).
// Set by resume_from_shm_ipi; cleared when enc2 parks or exits.
static mut IPI_INTERRUPTED: [Option<HostContext>; crate::ipi::MAX_HARTS] =
    [None; crate::ipi::MAX_HARTS];

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum State {
    Stopped,
    Running,
    Destroying,
    WaitingForShm(u32),    // suspended; will be resumed by notify_shm
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RegionType {
    RegionInvalid,
    RegionEPM,
    RegionUTM,
    RegionShared,
    RegionEncEnc,  // enc-to-enc channel with hash-based attestation
}

pub struct Region {
    id: usize,
    r_type: RegionType,
    paddr: usize,
    size: usize,
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

    // Set by stop_enclave when enc2 makes an OCALL in S-mode IPI context (sub-host sleeping).
    // Cleared by wait_and_resume_for_shm which captures a fresh saved_host and returns
    // EdgeCallHost so the driver can wake sub-host and process the OCALL normally.
    pending_ocall_for_host: bool,
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
            pending_ocall_for_host: false,
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
        if VERBOSE_MK { sm_mark(b"[MK] STE-in (host->enclave)\n\0"); }
        /* save host context */
        let thread = &mut self.threads[0].as_mut().unwrap();

        thread.swap_prev_state(regs);
        thread.swap_prev_mepc(regs, regs.mepc);
        thread.swap_prev_mstatus(regs, regs.mstatus);

        let interrupts = 0;
        csr_write!(mideleg, interrupts);

        if load_parameters {
            regs.mepc = self.pa_params.dram_base - 4; // regs->mepc will be +4 before sbi_ecall_handler return
            // FPGA(WG) 적응: 로딩 중 인터럽트 OFF (MPIE 제거). MPIE=1이면 loader가 부트
            // 파라미터(a1-a7)를 저장하기 전 M-mode 타이머가 선점 → 재진입은 load_parameters=false
            // 경로라 a1-a7을 다시 세팅하지 않고 stale thread state를 복원 → loader가 boot-param=0
            // 수신. 인터럽트를 꺼서 mret~loader 구간 선점을 막아 a1-a7이 loader까지 온전히 전달됨.
            regs.mstatus = 1 << crate::encoding::MSTATUS_MPP_SHIFT;
            regs.a1 = self.pa_params.dram_base; // $a1: (PA) DRAM base,
            regs.a2 = self.pa_params.dram_size; // $a2: (PA) DRAM size,
            regs.a3 = self.pa_params.runtime_base; // $a3: (PA) kernel location,
            regs.a4 = self.pa_params.user_base; // $a4: (PA) user location,
            regs.a5 = self.pa_params.free_base; // $a5: (PA) freemem location,
            regs.a6 = self.pa_params.untrusted_base; // $a6: (VA) utm base,
            regs.a7 = self.pa_params.untrusted_size; // $a7: (size_t) utm size

            csr_write!(satp, 0);

            // FPGA(WG) boot-param stash: SD의 prebuilt ros.ke 로더가 부트 파라미터를 고정 슬롯
            // 0x83700000에서 읽는다 (레지스터 a1-a7만으로는 이 로더가 못 받음 — FPGA 실측 확인:
            // stash 제거 시 loader가 dram=0/FATAL). SM이 미리 써두고 아래 EPM flush 전에 내림.
            unsafe {
                let p = BOOT_PARAM_SLOT as *mut usize;
                p.add(0).write_volatile(self.pa_params.dram_base);
                p.add(1).write_volatile(self.pa_params.dram_size);
                p.add(2).write_volatile(self.pa_params.runtime_base);
                p.add(3).write_volatile(self.pa_params.user_base);
                p.add(4).write_volatile(self.pa_params.free_base);
                p.add(5).write_volatile(self.pa_params.untrusted_base);
                p.add(6).write_volatile(self.pa_params.untrusted_size);
            }

            // FPGA(WG) 캐시 코히런스 (관문4): host(OS, WID=OS_WID)가 enclave 코드(runtime+loader+
            // eapp)를 EPM에 써넣은 것이 host-WID 캐시에 갇힘. enclave는 다른 WID로 실행되어 그
            // 라인을 못 보고 stale을 읽음. L2 flush로 host dirty 라인을 메모리로 내린 뒤 fence.i로
            // I캐시 무효화. (QEMU엔 WID-tagged 캐시가 없어 이 처리가 없었음)
            //
            // EPM flush는 wid=0으로 발행한다. wid=0는 onlyTagHit 경로로 **모든 WID 태그 라인**을
            // writeback+invalidate 하므로(호스트 OS_WID 라인 + 그 물리페이지에 남은 다른 WID stale
            // 라인까지) enclave가 어느 WID로 읽어도 코히런트하다. 정확성에 필수.
            // (FPGA 실측 확정: OS_WID로만 flush=선택적→non-OS_WID stale 라인 누락→publisher loadElf
            //  FATAL/SHM=0. wid=0로 되돌리니 ros SHM=0xcafebabe 정상. HW의 FlushWid 레지스터
            //  0x2010218은 남아있으나 미사용 — storm 감소 최적화는 정확성을 깨서 폐기함.)
            if BROAD_FLUSH {
                // enclave 풀 전체를 광역 flush (재사용 stale 라인 근절 진단)
                flush_epm_to_memory(BROAD_FLUSH_BASE, BROAD_FLUSH_SIZE, 0);
            } else {
                flush_epm_to_memory(self.pa_params.dram_base, self.pa_params.dram_size, 0);
            }
            flush_epm_to_memory(BOOT_PARAM_SLOT, CACHE_BLOCK_BYTES, 0);
            unsafe { core::arch::asm!("fence.i", options(nostack)); }

            // [PROBE] loader 코드가 메모리에 제대로 실렸는지 M-mode(WID7)로 dram_base 첫 4워드 확인.
            // 기대: 첫 명령 = `la sp,_estack` = auipc x2 → 하위12비트 0x117 (즉 0x????_?117).
            // 코드가 정상이면 → enclave(WID=1) fetch 경로 문제 확정. 0/garbage면 → 로드/flush 실패.
            // load_parameters=true(초기 run) 경로에서만 실행되므로 run당 1회만 출력.
            unsafe {
                let p = self.pa_params.dram_base as *const u32;
                sm_mark_hex(b"[PROBE] dram_base[0]= ", core::ptr::read_volatile(p) as usize);
                sm_mark_hex(b"[PROBE] dram_base[1]= ", core::ptr::read_volatile(p.add(1)) as usize);
                sm_mark_hex(b"[PROBE] dram_base[2]= ", core::ptr::read_volatile(p.add(2)) as usize);
                sm_mark_hex(b"[PROBE] dram_base[3]= ", core::ptr::read_volatile(p.add(3)) as usize);
                let bp = BOOT_PARAM_SLOT as *const usize;
                sm_mark_hex(b"[PROBE] bootparam[0](dram_base)= ", core::ptr::read_volatile(bp) as usize);
            }
        }

        switch_vector_enclave();

        // ISOLATION EXPERIMENT: program the EPM WGC slot EAGERLY (before entry) with the
        // enclave's WID only, at a HIGHER priority than the OS catch-all (HW slot 7) so
        // OS_WID is denied at the EPM. Host loaded the EPM earlier while the catch-all
        // still granted OS_WID; flush_epm_to_memory (above) evicts those lines so the
        // enclave's fetch refills fresh under its own WID.
        for memid in 0..MAX_ENCLAVE_REGIONS {
            let (rid, is_epm) = match self.regions[memid] {
                Some(ref region) => (region.id, region.r_type == RegionType::RegionEPM),
                None => continue,
            };
            if is_epm {
                #[cfg(any(feature = "isolator_wg", feature = "isolator_hybrid"))]
                {
                    let (wid, _action) = crate::wid::assign_wid(self.eid, rid);
                    self.last_wid = wid;
                    let _ = isolator::set_isolator_with_wid(rid, wid);
                }
            } else {
                let _ = isolator::set_isolator(rid, false);
            }
        }

        // [PROBE2] enclave WID의 "데이터 경로"가 살아있는지 M-mode에서 직접 확인한다.
        // WGC 슬롯 프로그래밍(위 루프) 이후에 실행되므로 enclave가 보는 것과 동일한 체커 설정.
        // 세 지점을 순서대로 읽고, 각 읽기 직전에 START 마커를 찍는다 → 멈춘 지점이 곧 원인.
        //   A: SM 자신의 .bss  → WID1로 SM 코드/데이터 경로가 도는지 (대조군)
        //   B: EPM dram_base   → enclave가 fetch하다 멈추는 바로 그 주소
        //   C: BOOT_PARAM_SLOT → EPM 밖 일반 DRAM (WID1 권한 없음 → 정상이면 access fault)
        // A OK & B hang → EPM/WGChecker 경로가 WID1 요청에 무응답 (memory-side 후보 확정)
        // A hang        → WID1 M-mode 접근 자체가 죽음 (L2/코어 공통 경로)
        if load_parameters {
            let wid = self.last_wid;
            let cur = csr_read_custom!(0x391); // 현재 M-mode WID (기본 nWorlds-1 = 7)
            sm_mark_hex(b"[PROBE2] wid= ", wid);
            sm_mark_hex(b"[PROBE2] mwid_restore= ", cur);
            // 진입 직전 WG CSR 스냅샷: S-mode(enclave)는 mlwid, U-mode는 mwiddeleg==0이면 mlwid를 쓴다.
            sm_mark_hex(b"[PROBE2] csr mwid(0x391)= ", csr_read_custom!(0x391));
            sm_mark_hex(b"[PROBE2] csr mlwid(0x390)= ", csr_read_custom!(0x390));
            sm_mark_hex(b"[PROBE2] csr slwid(0x190)= ", csr_read_custom!(0x190));
            sm_mark_hex(b"[PROBE2] csr mwiddeleg(0x748)= ", csr_read_custom!(0x748));
            // minstret(0xb02)이 실제로 도는지 확인한다. nop 사이 차분이 0이면 카운터가 죽은 것이라
            // "instret delta=0"은 enclave가 안 돈다는 증거가 될 수 없다. mcountinhibit도 같이 본다.
            {
                let i0 = csr_read_custom!(0xb02);
                unsafe {
                    core::arch::asm!("nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop",
                                     "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop",
                                     options(nostack, nomem));
                }
                let i1 = csr_read_custom!(0xb02);
                sm_mark_hex(b"[PROBE2] minstret self-test delta(16 nop)= ", i1.wrapping_sub(i0));
                sm_mark_hex(b"[PROBE2] csr mcountinhibit(0x320)= ", csr_read_custom!(0x320));
            }
            unsafe {
                let sm_addr = core::ptr::addr_of!(LAST_MEPC) as usize;
                sm_mark_hex(b"[PROBE2] A start (SM .bss) addr= ", sm_addr);
                let a = probe_read_at_wid(sm_addr, wid, cur);
                sm_mark_hex(b"[PROBE2] A ok val= ", a);

                sm_mark_hex(b"[PROBE2] B start (EPM dram_base) addr= ", self.pa_params.dram_base);
                let b = probe_read_at_wid(self.pa_params.dram_base, wid, cur);
                sm_mark_hex(b"[PROBE2] B ok val= ", b);

                sm_mark_hex(b"[PROBE2] C start (bootparam slot) addr= ", BOOT_PARAM_SLOT);
                let c = probe_read_at_wid(BOOT_PARAM_SLOT, wid, cur);
                sm_mark_hex(b"[PROBE2] C ok val= ", c);

                sm_mark(b"[PROBE2] D start (fence.i + fetch @wid)\n\0");
                probe_fetch_at_wid(wid, cur);
                sm_mark(b"[PROBE2] D ok (fetch @wid returned)\n\0");
            }
            sm_mark(b"[PROBE2] all done\n\0");
        }

        // Setup any platform specific defenses
        cpu::enter_enclave_context(self.eid, self.last_wid);
        // [MK] 진입 직전 minstret과 타이머 상태를 저장한다. switch_to_host에서 차분을 찍어
        // "enclave가 명령을 하나라도 retire 하는가"를 판정한다. mtimecmp <= mtime이면 mret
        // 직후 타이머가 즉시 다시 걸려 첫 명령이 retire 전에 선점된다(=진행률 0의 다른 설명).
        unsafe {
            ENTRY_MINSTRET = csr_read_custom!(0xb02);
            ENTRY_MTIME = core::ptr::read_volatile(CLINT_MTIME as *const u64) as usize;
            ENTRY_MTIMECMP = core::ptr::read_volatile(CLINT_MTIMECMP as *const u64) as usize;
        }
        if VERBOSE_MK { sm_mark(b"[MK] STE-out (mret->enclave)\n\0"); }
    }

    /// Switch from enclave to host context.
    ///
    /// `keep_vec`: if true, mtvec is left pointing at `trap_vector_enclave` so that
    /// M-mode SHM IPIs can be caught directly without a host round-trip (wait_shm path).
    /// All other callers pass false to restore the standard OpenSBI `_trap_handler`.
    pub fn switch_to_host(&mut self, regs: &mut TrapFrame, keep_vec: bool) {
        if VERBOSE_MK { sm_mark(b"[MK] STH-in (enclave->host)\n\0"); }
        // [MARKER] enclave PC 추적: 값이 바뀔 때만 출력(loop flood 방지).
        // regs.mepc는 아직 host 컨텍스트로 swap되기 전이라 enclave의 PC(타이머 경로는 -4 상태).
        unsafe {
            if regs.mepc != LAST_MEPC {
                sm_mark_hex(b"[MK] mepc CHANGE -> ", regs.mepc);
                LAST_MEPC = regs.mepc;
            }
            // [MARKER] L2 cross-WID 진단 카운터: 512 bounce마다 스냅샷. enclave가 진입 fetch에서
            // 멈춘 동안 oth(cross-WID onlyTagHit)/stall이 어떻게 변하는지로 병목 경로를 조준한다.
            // oth 계속 증가 → acquire가 반복적으로 cross-WID 경로. stall만 증가 → 요청 미수락.
            // 둘 다 정체 → 정지가 L2 밖(memory-side WGChecker/core). base 0x2010000.
            STH_CNT += 1;
            // [MK] 이번 엔트리 동안 retire된 명령 수와 타이머 상태. 진입 시 저장한 값과의 차분.
            // instret 차분이 SM 트랩 진입 오버헤드 수준(수십)에 머물면 enclave는 정말 아무것도
            // 실행하지 않은 것. 수백~수천이면 실행은 되는데 PC가 되감기는 것.
            // t_left = mtimecmp - mtime (진입 시점): 음수/0이면 mret 즉시 재선점.
            if (STH_CNT & 0x1ff) == 0 {
                let now_instret = csr_read_custom!(0xb02);
                sm_mark_hex(b"[MK] instret delta= ", now_instret.wrapping_sub(ENTRY_MINSTRET));
                sm_mark_hex(b"[MK] entry mtime= ", ENTRY_MTIME);
                sm_mark_hex(b"[MK] entry mtimecmp= ", ENTRY_MTIMECMP);
                sm_mark_hex(b"[MK] entry t_left= ", ENTRY_MTIMECMP.wrapping_sub(ENTRY_MTIME));
            }
            if (STH_CNT & 0x1ff) == 0 {
                let oth   = core::ptr::read_volatile(0x0201_0300 as *const u32) as usize;
                let rel   = core::ptr::read_volatile(0x0201_0308 as *const u32) as usize;
                let stall = core::ptr::read_volatile(0x0201_0310 as *const u32) as usize;
                let outa  = core::ptr::read_volatile(0x0201_0318 as *const u32) as usize;
                let outd  = core::ptr::read_volatile(0x0201_0320 as *const u32) as usize;
                let reqw1 = core::ptr::read_volatile(0x0201_0328 as *const u32) as usize;
                sm_mark_hex(b"[MK] STHcnt= ", STH_CNT);
                sm_mark_hex(b"[MK] L2 oth= ", oth);
                sm_mark_hex(b"[MK] L2 rel= ", rel);
                sm_mark_hex(b"[MK] L2 stall= ", stall);
                sm_mark_hex(b"[MK] L2 outa= ", outa);
                sm_mark_hex(b"[MK] L2 outd= ", outd);
                sm_mark_hex(b"[MK] L2 reqw1= ", reqw1);
                // inner 채널(코어↔L2) 총량과, EPM 주소창 요청의 WID별 분포.
                // 판독: EPM 창에 wid=1 요청이 늘어나는데 ind가 안 늘면 L2가 grant를 안 돌려주는 것,
                // 다른 wid 칸이 늘면 fetch가 엉뚱한 WID로 나가는 것, 전부 0이면 요청이 코어 밖으로
                // 안 나가는 것(프론트엔드 정지).
                let ind = core::ptr::read_volatile(0x0201_0330 as *const u32) as usize;
                let ina = core::ptr::read_volatile(0x0201_0338 as *const u32) as usize;
                sm_mark_hex(b"[MK] L2 inD= ", ind);
                sm_mark_hex(b"[MK] L2 inA= ", ina);
                let mut w = 0usize;
                while w < 8 {
                    let c = core::ptr::read_volatile((0x0201_0340 + 8 * w) as *const u32) as usize;
                    // "[MK] EPM wid<n>= 0x...."
                    let label: &[u8] = match w {
                        0 => b"[MK] EPM wid0= ",
                        1 => b"[MK] EPM wid1= ",
                        2 => b"[MK] EPM wid2= ",
                        3 => b"[MK] EPM wid3= ",
                        4 => b"[MK] EPM wid4= ",
                        5 => b"[MK] EPM wid5= ",
                        6 => b"[MK] EPM wid6= ",
                        _ => b"[MK] EPM wid7= ",
                    };
                    sm_mark_hex(label, c);
                    w += 1;
                }
            }
        }
        // FPGA(WG) OCALL coherence: on an enclave->host switch (e.g. an OCALL), write the
        // enclave's edge-call buffer in UTM back to memory so the host reads fresh data.
        // The enclave filled UTM (edge_call struct + args, e.g. the "Hello World" string)
        // under its own WID; the host reads UTM as OS_WID and otherwise sees stale lines.
        // Only the head of UTM holds the edge_call payload, so flush a bounded window
        // (the full multi-MB UTM would crawl). Done ONLY here (not on re-entry) — flushing
        // on switch_to_enclave previously invalidated the host's just-written reply.
        let utm_flush = core::cmp::min(self.pa_params.untrusted_size, OCALL_UTM_FLUSH_BYTES);
        // wid=0 (onlyTagHit) so the enclave-WID edge-call lines are written back regardless of
        // which WID tagged them; small window, so the cross-WID path cost is negligible.
        flush_epm_to_memory(self.pa_params.untrusted_base, utm_flush, 0);

        let _ = isolator::set_isolator(isolator::os_region_id(), false);

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
        if VERBOSE_MK { sm_mark(b"[MK] STH-out (mret->host)\n\0"); }
    }
}

pub const MAX_ENCLAVES: usize = 16;
pub const MAX_SHARED_REGIONS: usize = 8;

/// SiFive InclusiveCache (L2) control register: 64-bit PA를 써넣으면 그 라인을 L2에서
/// flush(writeback+invalidate). MMIO store는 flush 완료까지 블록됨. (기존 A와 동일)
/// [실험 ①] 광역 flush: enclave 진입 시 이 enclave EPM만이 아니라 enclave 메모리 풀 전체
/// (+SHM)를 주소 flush → 재사용 페이지/SHM에 남은 stale 라인까지 다 내림. 반복해서 5/5면
/// 잔존 실패 = cache-staleness 확정. 느리지만(수만 블록) 진단용. false면 원래 좁은 flush.
const BROAD_FLUSH: bool = false;  // ① 실험 결과 악화 → 폐기. 좁은(EPM만) flush로 복원.
/// [② 실험] destroy 시 EPM 페이지를 0-clear → 재사용 stale 근절 가설. FPGA 실측(2026-07-15):
/// **악화(4/5→2/5), 실패가 run1·2로 앞당겨짐.** stale 내용이 원인 아님이 반증됨. 게다가 SM이
/// TRUSTED_WID(7)로 EPM에 씀 → WID7 태그 라인이 다음 enclave(WID1) 로드에 cross-WID 부하 추가
/// (OS_WID flush 개악과 동형). → 폐기. 잔존 원인 = OS↔enclave EPM-로드 비결정 코히런스 race.
const ZERO_ON_DESTROY: bool = false;
/// [③ 실험 2026-07-22] destroy 시 EPM을 0이 아니라 **같은 값으로 다시 덮어쓰기**(read→write same).
/// 가설 검증용: 트리거가 '데이터 변경'인가 'cross-WID 접근 자체'인가를 가른다. WID 검사는 값과
/// 무관(onlyTagHit=태그일치+WID불일치)하므로 예측=zeroing과 동일하거나(오히려 read 추가로) 악화.
/// 만약 wedge가 안 나면 근본원인 해석을 재고해야 함. ZERO_ON_DESTROY와 상호배타(둘 다 true 금지).
const REWRITE_SAME_ON_DESTROY: bool = false;  // 진단 완료(2026-07-22): 값 무관, cross-WID 접근이 트리거 확증. clean baseline 복원.
const BROAD_FLUSH_BASE: usize = 0x8300_0000;
const BROAD_FLUSH_SIZE: usize = 0x0080_0000; // 8MB, enclave 풀(EPM+SHM) 커버
const L2_CTRL_FLUSH64: usize = 0x2010000 + 0x200;
/// WID applied to subsequent L2 flush requests (FlushWid control reg). A flush issued under
/// the WID that owns the dirty lines is a clean same-WID hit; wid=0 over WID-tagged lines
/// takes the heavy cross-WID onlyTagHit path (the per-enclave EPM-flush storm). SW writes
/// this before the flush loop. Default (never written) = 0 = legacy behavior.
const L2_CTRL_FLUSH_WID: usize = 0x2010000 + 0x218;
const CACHE_BLOCK_BYTES: usize = 64;
/// Bounded UTM window flushed on an enclave->host switch for OCALL coherence (the
/// edge_call struct + args live at the head of UTM). Flushing the whole multi-MB UTM
/// every switch would crawl (16k blocking MMIO writes).
const OCALL_UTM_FLUSH_BYTES: usize = 16 * 1024;
/// loader가 부트 파라미터를 복구하는 고정 EPM scratch slot. SD의 prebuilt ros.ke 로더가
/// 이 주소에서 읽으므로 SM이 switch_to_enclave에서 써둔다. (런타임 로더와 주소 일치 필수)
const BOOT_PARAM_SLOT: usize = 0x83700000;

/// 주어진 물리범위를 캐시블록 단위로 L2 flush 레지스터에 흘려 메모리로 writeback.
/// enclave 첫 진입 전 1회 호출 → enclave가 host-WID L1에 갇힌 stale 대신 host가 로드한
/// EPM(코드/파라미터)을 메모리에서 올바로 읽게 함. (관문4, 기존 A에서 이식)
fn flush_epm_to_memory(pa_start: usize, size: usize, wid: usize) {
    // Ensure any prior stores (the enclave's just-written edge-call data, etc.) are globally
    // ordered BEFORE we start issuing L2 flushes. Without this leading fence the flush could
    // race an in-flight store and write back a stale line -> non-deterministic cross-WID data
    // (OCALL string missing on some runs).
    unsafe {
        core::arch::asm!("fence", options(nostack));
    }
    // Select the WID for these flushes. Flushing under the WID that owns the dirty lines
    // (e.g. OS_WID for host-loaded EPM) makes each flush a clean same-WID hit instead of a
    // cross-WID onlyTagHit, eliminating the per-enclave flush storm. wid=0 = legacy path.
    unsafe {
        core::ptr::write_volatile(L2_CTRL_FLUSH_WID as *mut u64, wid as u64);
    }
    let mut pa = pa_start & !(CACHE_BLOCK_BYTES - 1);
    let end = pa_start + size;
    while pa < end {
        unsafe {
            core::ptr::write_volatile(L2_CTRL_FLUSH64 as *mut u64, pa as u64);
        }
        pa += CACHE_BLOCK_BYTES;
    }
    unsafe {
        core::arch::asm!("fence", options(nostack));
    }
}

/// Scrub an EPM region on destroy. With WID-tagged caches a plain memset is not
/// enough: enclave-WID dirty secret lines could survive (and write back later),
/// and once the WGC slot is cleared the freed pool pages become host-accessible.
/// So: (1) flush+invalidate any lingering lines to DRAM, (2) zero the DRAM,
/// (3) flush the zeros back. After this the physical pages hold no secrets.
/// (upstream Keystone does sbi_memset on destroy; the PMP path here does it in
/// pmp::region_free, but the WG reset_wg only clears the slot, not the RAM.)
#[cfg(any(feature = "isolator_wg", feature = "isolator_hybrid"))]
fn scrub_epm(pa_start: usize, size: usize) {
    flush_epm_to_memory(pa_start, size);
    unsafe {
        core::ptr::write_bytes(pa_start as *mut u8, 0, size);
    }
    flush_epm_to_memory(pa_start, size);
}

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
        //
        enclave.regions[0] = Some(Region {
            id: region,
            r_type: RegionType::RegionEPM,
            paddr: create_args.epm_region.paddr,
            size: create_args.epm_region.size,
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
            ) {
                Ok(rid) => {
                    let _ = share_shm_region(rid, enclave.id(), shm::Perm::FULL);
                }
                Err(err) => {
                }
            }
        }

        // Measure at create (like upstream Keystone): the host has finished
        // copying loader+runtime+eapp into the EPM before it calls finalize, so
        // the image is present now. WID-tagged caches are not coherent across
        // WIDs, so first flush the host-WID-loaded image down to DRAM, then hash
        // the coherent image. (First-entry re-flushes for boot-param stash +
        // fence.i; the measurement itself no longer happens on the entry path.)
        flush_epm_to_memory(create_args.epm_region.paddr, create_args.epm_region.size);
        enclave.compute_hash();

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
                                crate::wid::WIDAction::Assigned { slot: _ } => (),
                                crate::wid::WIDAction::Evicted { slot: _, evicted_eid: _ } => (),
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
                                            let mut perm: u64 = 0;
                                            for copt in shm.perm_conf.conf_list.iter() {
                                                if let Some(c) = copt {
                                                    if c.eid == 11 {
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
                        // For host-enclave SHM: eid=11 → OS_WID is included normally.
                        #[derive(Copy, Clone)]
                        struct PermEntry { wid: usize, eid: usize }
                        let mut entries = [PermEntry { wid: 0, eid: 0 }; 8];
                        let mut n = 0usize;
                        let mut perm: u64 = 0;
                        for conf_opt in region.perm_conf.conf_list.iter() {
                            if let Some(c) = conf_opt {
                                if c.eid == 11 {
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
                        let _ = (n, entries);
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
        // Refuse ONLY self-destruction (this call executing inside the very enclave
        // being torn down). Otherwise force teardown regardless of Running/count.
        //
        // FPGA(single-hart): destroy_enclave is an SBI call from the host (S-mode);
        // on a single hart no enclave executes at that moment, so a leftover
        // Running / count>0 state just means the enclave is suspended (e.g. a stuck
        // subscriber). The old `state != Running && count == 0` guard refused those
        // and LEAKED the EID slot + WG regions + SHM across repeated runs (freed
        // nothing -> region PA overlap -> the next create_enclave fails). Force-clean
        // instead so successive ros runs are independent.
        if cpu::is_enclave_context() && cpu::get_enclave_id() == eid {
            return Err(Error::NotDestroyable);
        }

        sm_mark(b"[SM] destroy D0 enter\n\0");
        let mut runstate = enclave.state.lock();
        /* update the enclave state first so that
         * no SM can run the enclave any longer */
        runstate.state = State::Destroying;
        runstate.count = 0;
        drop(runstate);

        sm_mark(b"[SM] destroy D1 shm-loop begin\n\0");
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
                    // capture (id, paddr, size) so we can flush the region before freeing
                    if owner_match || utm_match { Some((region.id, region.paddr, region.size)) } else { None }
                } else {
                    None
                };
                if let Some((region_id, paddr, size)) = should_free {
                    // FPGA(WG): write the shared/UTM region back to memory before freeing it,
                    // so the host/Linux reclaims coherent pages (no leftover enclave-WID lines
                    // -> cross-WID wedge on reclaim after destroy).
                    flush_epm_to_memory(paddr, size, 0);
                    let _ = isolator::set_isolator(region_id, true);
                    let _ = isolator::region_free(region_id);
                    SHARED_MEM[i] = None;
                }
            }
        }

        // FPGA(WG): write the enclave's EPM back to memory BEFORE freeing its WGC slot.
        // The enclave's dirty lines are tagged with its WID; once the host/Linux reclaims
        // these pages (buddy allocator) it touches them as OS_WID and would otherwise hit
        // the leftover cross-WID lines -> coherence wedge after destroy. Flushing (with the
        // MSHR onlyTagHit fix) evicts them to DRAM while the slot still permits it.
        // wid=0 (onlyTagHit): after the enclave ran, EPM holds a mix of enclave-WID (stack/
        // heap) and OS_WID lines; the cross-WID path writes back all of them before reclaim.
        // NOTE(2026-07-16): a destroy-time DRAIN (2× EPM+UTM flush as a completion barrier)
        // was tried here to rule out an in-flight-writeback race with the host reclaim — it did
        // NOT fix the wedge (destroy completed D0-D6 with drain, host still hung after). Reverted
        // to the single flush. Residual = HW cross-WID coherence stall (see memory), pursuing ILA.
        // NOTE(2026-07-21): a destroy-time L1 flush (fence.i, both orders vs the L2 flush) was
        // tried and did NOT fix the wedge — the real bug is in the L2 (cross-WID Acquire re-tags
        // the directory out from under a same-set voluntary Release, which then never gets a
        // ReleaseAck). Fixed in MSHR.scala (nestC extension). Reverted the fence.i here for a
        // clean baseline. Keeping the single L2 flush below.
        sm_mark(b"[SM] destroy D2 epm-flush begin\n\0");
        flush_epm_to_memory(enclave.pa_params.dram_base, enclave.pa_params.dram_size, 0);
        sm_mark(b"[SM] destroy D2 epm-flush done\n\0");

        // [② 실험] 재사용 stale 근절: destroy 시 EPM 페이지를 실제로 0-clear.
        // (기존엔 아래 'clear all the data' 주석만 있고 실제로 안 지웠음 → 재사용 페이지에
        //  이전 enclave 내용 잔존 → 반복 시 뒤 run이 garbage 읽음.) 0으로 채운 뒤 flush로
        // DRAM까지 내려 다음 enclave가 clean 페이지를 받게 함. 반복 4/5→5/5면 원인 확정.
        if ZERO_ON_DESTROY {
            unsafe {
                let base = enclave.pa_params.dram_base as *mut usize;
                let words = enclave.pa_params.dram_size / core::mem::size_of::<usize>();
                for i in 0..words {
                    base.add(i).write_volatile(0);
                }
                core::arch::asm!("fence", options(nostack));
            }
            flush_epm_to_memory(enclave.pa_params.dram_base, enclave.pa_params.dram_size, 0);
        }

        // [③ 실험] 0-clear 대신 **같은 값으로 다시 덮어쓰기**: 각 워드를 읽어 그대로 되쓴다.
        // 데이터는 안 바뀌지만 SM(WID7)의 store가 enclave-WID 라인에 cross-WID로 다시 찍힌다.
        // 트리거가 '값 변경'이 아니라 'cross-WID 접근'임을 가리는 진단. (read+write 둘 다 cross-WID)
        if REWRITE_SAME_ON_DESTROY {
            sm_mark(b"[SM] destroy D2b rewrite-same begin\n\0");
            unsafe {
                let base = enclave.pa_params.dram_base as *mut usize;
                let words = enclave.pa_params.dram_size / core::mem::size_of::<usize>();
                for i in 0..words {
                    let v = base.add(i).read_volatile();
                    base.add(i).write_volatile(v);
                }
                core::arch::asm!("fence", options(nostack));
            }
            flush_epm_to_memory(enclave.pa_params.dram_base, enclave.pa_params.dram_size, 0);
            sm_mark(b"[SM] destroy D2b rewrite-same done\n\0");
        }

        // 1. clear all the data in the enclave pages
        // requires no lock (single runner)
        sm_mark(b"[SM] destroy D3 region-free begin\n\0");
        for i in 0..MAX_ENCLAVE_REGIONS {
            if let Some(region) = &enclave.regions[i] {
                if region.r_type == RegionType::RegionInvalid {
                    continue;
                }
                let rid = region.id;
                // Scrub EPM DRAM before releasing so the freed pool pages carry
                // no enclave secrets (WG reset_wg only clears the slot, not RAM).
                #[cfg(any(feature = "isolator_wg", feature = "isolator_hybrid"))]
                scrub_epm(region.paddr, region.size);
                let _ = isolator::set_isolator(rid, true);
                let _ = isolator::region_free(rid);
            }
        }

        (0..MAX_ENCLAVE_REGIONS).for_each(|idx| {
            enclave.regions[idx] = None;
        });

        // 2. Release WID slot assignment for this enclave
        sm_mark(b"[SM] destroy D4 release-wid begin\n\0");
        #[cfg(any(feature = "isolator_wg", feature = "isolator_hybrid"))]
        crate::wid::release_wid_for_eid(eid);

        // [WEDGE] release-wid는 invalidate_wid_in_all_slots로 슬롯을 통째로 0으로 만든다.
        // 과거(2026-07-15) 이 경로가 호스트를 막아 access-fault storm을 낸 전력이 있으므로,
        // 정리 후 DRAM 체커 슬롯 상태를 한 번 덤프해 호스트가 접근 가능한 상태인지 남긴다.
        // 정리 후 OS catch-all을 명시적으로 다시 깐다. enclave→host 전환 경로(switch_to_host)는
        // 매번 이걸 해주지만 destroy는 host 컨텍스트에서 호출되어 그 경로를 타지 않으므로,
        // 슬롯 정리로 catch-all이 훼손된 경우 복구할 기회가 없다.
        let _ = isolator::set_isolator(isolator::os_region_id(), false);

        sm_mark(b"[SM] destroy D4b slot dump (cfg/addr/perm)\n\0");
        {
            let dram = crate::wg::WGChecker::new(crate::wg::WGC_DRAM_BASE);
            let mut i = 0usize;
            while i < 8 {
                let cfg = dram.get_slot_cfg(i) as usize;
                let addr = dram.get_slot_addr(i) as usize;
                let perm = dram.get_slot_perm(i) as usize;
                if cfg != 0 || perm != 0 {
                    sm_mark_hex(b"[SM]   slot idx= ", i);
                    sm_mark_hex(b"[SM]     cfg= ", cfg);
                    sm_mark_hex(b"[SM]     addr= ", addr);
                    sm_mark_hex(b"[SM]     perm= ", perm);
                }
                i += 1;
            }
        }

        // 3. release eid
        sm_mark(b"[SM] destroy D5 free-eid begin\n\0");
        let _ = Enclave::free(eid);

        sm_mark(b"[SM] destroy D6 done ok\n\0");
        return Ok(());
    }

    Err(Error::InvalidId)
}

pub fn enter_enclave(tf: &mut TrapFrame, eid: usize) -> Result<(), Error> {
    sm_mark(b"[MK] ENT-in (sbi enter_enclave)\n\0");
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

        // Measurement was already computed at create_enclave (fair with
        // Keystone). switch_to_enclave still flushes the EPM so the enclave (its
        // own WID) reads the loaded code from DRAM and stashes boot params.
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
            if let Some(enc2) = find_enclave(enc2_eid) {
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

    let this_hart = csr_read!(mhartid) as usize;

    // IPI OCALL path: stop_enclave set pending_ocall_for_host + SSIP; driver woke here.
    // Capture fresh sub-host context from this ecall frame and return EdgeCallHost.
    if let Some(enc) = find_enclave(eid) {
        if enc.pending_ocall_for_host {
            enc.pending_ocall_for_host = false;
            enc.saved_host = Some(capture_host_ctx(tf));
            return Err(Error::EdgeCallHost);
        }
    } else {
        return Err(Error::InvalidId);
    }

    let is_waiting = match find_enclave(eid) {
        Some(enc) => matches!(enc.state.lock().state, State::WaitingForShm(_)),
        None => return Err(Error::InvalidId),
    };
    let has_deferred_ipi = crate::ipi::peek_pending_resume_for(this_hart, eid);
    let in_ipi_ctx = unsafe { IPI_INTERRUPTED[this_hart].is_some() };

    if !is_waiting && !has_deferred_ipi {
        if in_ipi_ctx { return Err(Error::WaitingForShm); }
        return resume_enclave(tf, eid);
    }

    // Save fresh sub-host context; used if IPI fires while we return WaitingForShm.
    if let Some(enc) = find_enclave(eid) {
        enc.saved_host = Some(capture_host_ctx(tf));
    }

    // Race: IPI arrived between previous WaitingForShm return and this call.
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
            if let Some(thread) = enc.threads[0].as_mut() {
                thread.advance_past_ecall_with_interrupted();
            }
            enc.switch_to_enclave(tf, false);
            return Ok(());
        }
        return Err(Error::InvalidId);
    }

    Err(Error::WaitingForShm)
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

        let this_hart = csr_read!(mhartid) as usize;

        // IPI path: MSIP fired while sub-host was sleeping (S-mode).
        // enc2 was switched directly; thread.State held T; switch_to_host restored T into tf.
        if unsafe { IPI_INTERRUPTED[this_hart].is_some() } {
            unsafe { IPI_INTERRUPTED[this_hart] = None; }
            if request == 1 /* EdgeCallHost */ {
                enclave.pending_ocall_for_host = true;
                csr_set!(mip, MIP_SSIP);
            }
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

        // IPI park: enc2 called wait_shm after being IPI-woken (S-mode path).
        // thread.State held T; switch_to_host restored T into tf — mret to T.
        if unsafe { IPI_INTERRUPTED[this_hart].take().is_some() } {
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
                            let is_active = (rs2.state == State::Running
                                || rs2.state == State::Stopped)
                                && enc2.last_hart != current_hart;
                            if shm_rgn.perm_conf.get_perm(enc2.eid).is_some() && is_active {
                                // ROS latest-value semantics: always overwrite pending slot.
                                // Running: enc2 awake between IPI wake and wait_shm.
                                // Stopped: enc2 mid-OCALL; consumed by wait_shm on next park.
                                let state_str = if rs2.state == State::Running { "Running" } else { "Stopped(OCALL)" };
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
    crate::ipi::clear_msip(this_hart);

    if let Some(eid) = crate::ipi::take_pending_resume(this_hart) {
        if let Some(enc) = find_enclave(eid) {
            {
                let mut rs = enc.state.lock();
                if matches!(rs.state, State::WaitingForShm(_)) {
                    rs.count += 1;
                    rs.state = State::Running;
                }
            }
            // sub-host is sleeping (S-mode); save T and switch enc2 directly.
            // OCALLs are delivered via pending_ocall_for_host in wait_and_resume_for_shm.
            unsafe { IPI_INTERRUPTED[this_hart] = Some(capture_host_ctx(tf)); }
            enc.switch_to_enclave(tf, false);
            return true;
        }
    }
    false
}

pub fn exit_enclave(tf: &mut TrapFrame) -> Result<(), Error> {
    sm_mark(b"[MK] EXE-in (sbi exit_enclave)\n\0");
    if let Some(enclave) = find_enclave(cpu::get_enclave_id()) {
        sm_mark(b"[MK] EXE-found -> switch_to_host\n\0");
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

        // IPI exit: tf = T already (switch_to_host loaded T from thread.State).
        let this_hart = csr_read!(mhartid) as usize;
        if unsafe { IPI_INTERRUPTED[this_hart].take().is_some() } {
            return Err(Error::IpiHandled);
        }

        sm_mark(b"[MK] EXE-ok (returning to host)\n\0");
        return Ok(());
    }

    sm_mark(b"[MK] EXE-NOTFOUND (find_enclave failed -> Err/panic)\n\0");
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
pub fn create_shared_mem(paddr: usize, size: usize) -> Result<usize, Error> {
    if let Ok(region_idx) = isolator::region_init(paddr, size, 11, true) {
        #[cfg(any(feature = "isolator_wg", feature = "isolator_hybrid"))]
        let _ = isolator::set_shm_host_only(region_idx);

        for i in 0..MAX_SHARED_REGIONS {
            if unsafe { SHARED_MEM[i].is_none() } {
                unsafe {
                    let mut region = Region {
                        id: region_idx,
                        r_type: RegionType::RegionShared,
                        paddr,
                        size,
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
                    return Err(Error::Invalid);
                }
                if !is_open_hash(&region.allowed_hash) {
                    let caller_hash = match find_enclave(caller_eid) {
                        Some(enc) => enc.hash,
                        None => return Err(Error::Invalid),
                    };
                    if !hash_eq(&caller_hash, &region.allowed_hash) {
                        return Err(Error::Invalid);
                    }
                }
                region.perm_conf.insert_perm(shm::PermConfig {
                    eid: caller_eid,
                    dyn_perm: shm::Perm::FULL,
                    st_perm: shm::Perm::FULL,
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
                    return Ok(());
                }
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
    Ok(())
}

