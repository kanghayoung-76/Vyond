use core::arch::asm;

#[macro_export]
macro_rules! csr_read {
    ($csr:ident) => {{
        use core::arch::asm;
        let res: usize;
        unsafe {
            asm!(
                concat!("csrr {reg}, ", stringify!($csr)) ,
                reg = out(reg) res,
            );
        }
        res
    }}
}

#[macro_export]
macro_rules! csr_read_custom {
    ($csr:expr) => {{
        use core::arch::asm;
        let res: usize;
        unsafe {
            asm!(
                "csrr {reg}, {csr}",
                reg = out(reg) res,
                csr = const $csr,
            );
        }
        res
    }}
}

#[macro_export]
macro_rules! csr_write {
    ($csr:ident, $val:expr) => {{
        use core::arch::asm;
        unsafe {
            asm!(
                concat!("csrw ", stringify!($csr), ", {reg}"),
                reg = in(reg) $val,
            );
        }
    }}
}

#[macro_export]
macro_rules! csr_write_custom {
    ($csr:expr , $val:expr) => {{
        use core::arch::asm;
        unsafe {
            asm!(
            "csrw {csr}, {reg}",
            csr = const $csr,
            reg = in(reg) $val,
            );
        }
    }}
}

#[macro_export]
macro_rules! csr_swap {
    ($csr:ident, $val:expr) => {{
        use core::arch::asm;
        let res = val;
        unsafe {
            asm!(
                concat!("csrrw {reg0}, ", stringify!($csr), ", {reg1}"),
                reg0 = out(reg) res,
                reg1 = in(reg) res,
            );
        }
        res
    }}
}

#[macro_export]
macro_rules! csr_read_set {
    ($csr:ident, $val:expr) => {{
        use core::arch::asm;
        let res = $val;
        unsafe {
            asm!(
                concat!("csrrs {reg0}, ", stringify!($csr), ", {reg1}"),
                reg0 = out(reg) res,
                reg1 = in(reg) res,
            );
        }
        res
    }}
}

#[macro_export]
macro_rules! csr_set {
    ($csr:ident, $val:expr) => {{
        use core::arch::asm;
        unsafe {
            asm!(
                concat!("csrs ", stringify!($csr), ", {reg}"),
                reg = in(reg) $val,
            );
        }
    }}
}

#[macro_export]
macro_rules! csr_read_clear {
    ($csr:ident, $val:expr) => {{
        use core::arch::asm;
        let res = $val;
        unsafe {
            asm!(
                concat!("csrrc {reg0}, ", stringify!($csr), ", {reg1}"),
                reg0 = out(reg) res,
                reg1 = in(reg) res,
            );
        }
        res
    }}
}

#[macro_export]
macro_rules! csr_clear {
    ($csr:ident, $val:expr) => {{
        use core::arch::asm;
        unsafe {
            asm!(
                concat!("csrc ", stringify!($csr), ", {reg}"),
                reg = in(reg) $val,
            );
        }
    }}
}

// mlwid CSR address (WorldGuard Machine-mode Local World ID)
const MLWID_CSR: usize = 0x390;
// Must match wg::OS_WID = NWORLDS - 2
const OS_WID: usize = 6;

/* hart state for regulating SBI */
struct CpuState {
    is_enclave: bool,
    eid: usize,
}

const MAX_HARTS: usize = 16;

const INIT: CpuState = CpuState {
    is_enclave: false,
    eid: 0,
};

static mut CPU_STATE: [CpuState; MAX_HARTS] = [INIT; MAX_HARTS];

pub fn is_enclave_context() -> bool {
    let hartid = csr_read!(mhartid) as usize;
    unsafe { CPU_STATE[hartid].is_enclave }
}

pub fn get_enclave_id() -> usize {
    let hartid = csr_read!(mhartid) as usize;
    unsafe { CPU_STATE[hartid].eid }
}

pub fn enter_enclave_context(eid: usize) {
    let hartid = csr_read!(mhartid) as usize;
    unsafe {
        CPU_STATE[hartid].is_enclave = true;
        CPU_STATE[hartid].eid = eid;
    }
    // Switch mlwid to the dynamically assigned WID for this enclave.
    // If no WID has been assigned yet (first entry), use ENCLAVE_WID_MIN (WID 1) as a
    // placeholder so the WGC will deny access and trigger the ACCESS FAULT handler,
    // which calls load_enclave_slot to assign the proper WID via the LRU table.
    #[cfg(any(feature = "isolator_wg", feature = "isolator_hybrid"))]
    {
        if let Some(wid) = crate::wid::get_assigned_wid(eid) {
            // WID already assigned and HW slot still valid — no fault needed.
            semihosting::hprintln!("[WGC:EPM] eid={} | REUSE  WID={}", eid, wid);
            csr_write_custom!(MLWID_CSR, wid);
        } else {
            // No WID yet — use placeholder so WGC triggers ACCESS FAULT → assign path.
            csr_write_custom!(MLWID_CSR, crate::wid::ENCLAVE_WID_MIN);
        }
    }
}

pub fn exit_enclave_context() {
    let hartid = csr_read!(mhartid) as usize;
    unsafe { CPU_STATE[hartid].is_enclave = false };
    // Restore mlwid to OS_WID so the OS/host transactions go out as world 6.
    #[cfg(any(feature = "isolator_wg", feature = "isolator_hybrid"))]
    csr_write_custom!(MLWID_CSR, OS_WID);
}
