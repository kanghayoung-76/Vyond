use crate::cpu;
use crate::crypto::{
    self, hash_extend, hash_extend_page, hash_finalize, hash_init, sm_sign, Sha3Ctx, ATTEST_DATA_MAXLEN,
    MDSIZE, PUBLIC_KEY_SIZE, RISCV_PGSIZE, SIGNATURE_SIZE,
};
use crate::enclave::find_enclave;
use crate::Error;

extern "C" {
    // Copy `len` bytes FROM enclave/runtime virtual address `src_va` TO SM buffer `dst`.
    // Uses MPRV to read through S-mode page tables.
    fn sm_mprv_read(dst: *mut u8, src_va: usize, len: usize) -> i32;
    // Copy `len` bytes FROM SM buffer `src` TO enclave/runtime virtual address `dst_va`.
    // Uses MPRV to write through S-mode page tables.
    fn sm_mprv_write(dst_va: usize, src: *const u8, len: usize) -> i32;
}

// Matches enclave_report_t in sdk/include/shared/sm_call.h
#[repr(C)]
pub struct EnclaveReport {
    pub hash: [u8; MDSIZE],
    pub data_len: u64,
    pub data: [u8; ATTEST_DATA_MAXLEN],
    pub signature: [u8; SIGNATURE_SIZE],
}

// Matches sm_report_t in sdk/include/shared/sm_call.h
#[repr(C)]
pub struct SmReport {
    pub hash: [u8; MDSIZE],
    pub public_key: [u8; PUBLIC_KEY_SIZE],
    pub signature: [u8; SIGNATURE_SIZE],
}

// Matches report_t in sdk/include/shared/sm_call.h
#[repr(C)]
pub struct Report {
    pub enclave: EnclaveReport,
    pub sm: SmReport,
    pub dev_public_key: [u8; PUBLIC_KEY_SIZE],
}

// Hash enclave physical memory: loader, runtime, eapp sections page by page.
// Called during first enclave entry after pages are fully loaded.
pub fn validate_and_hash_enclave(
    dram_base: usize,
    runtime_base: usize,
    user_base: usize,
    free_base: usize,
) -> [u8; MDSIZE] {
    let mut ctx = Sha3Ctx::zeroed();
    hash_init(&mut ctx);

    // Hash the section sizes first (matches Keystone's validate_and_hash_epm)
    let sizes: [usize; 3] = [
        runtime_base - dram_base,
        user_base - runtime_base,
        free_base - user_base,
    ];
    hash_extend(&mut ctx, unsafe {
        core::slice::from_raw_parts(
            sizes.as_ptr() as *const u8,
            core::mem::size_of::<[usize; 3]>(),
        )
    });

    // Hash all pages in [dram_base, free_base)
    let mut page = dram_base;
    while page < free_base {
        hash_extend_page(&mut ctx, page as *const u8);
        page += RISCV_PGSIZE;
    }

    let mut hash = [0u8; MDSIZE];
    hash_finalize(&mut hash, &mut ctx);
    hash
}

// Generate attestation report for the currently running enclave.
// report_ptr and data are physical addresses inside the enclave's EPM.
pub fn attest_enclave(report_ptr: usize, data: usize, size: usize) -> Result<(), Error> {
    if size > ATTEST_DATA_MAXLEN {
        return Err(Error::IllegalArgument);
    }

    let eid = cpu::get_enclave_id();
    let enclave = find_enclave(eid).ok_or(Error::InvalidId)?;

    let mut report = Report {
        enclave: EnclaveReport {
            hash: enclave.hash,
            data_len: size as u64,
            data: [0u8; ATTEST_DATA_MAXLEN],
            signature: [0u8; SIGNATURE_SIZE],
        },
        sm: SmReport {
            hash: unsafe { crate::SM_HASH },
            public_key: unsafe { crate::SM_PUBLIC_KEY },
            signature: unsafe { crate::SM_SIGNATURE },
        },
        dev_public_key: unsafe { crate::DEV_PUBLIC_KEY },
    };

    // Copy nonce/user data from enclave virtual memory via MPRV
    if size > 0 {
        unsafe {
            sm_mprv_read(report.enclave.data.as_mut_ptr(), data, size);
        }
    }

    // Sign: hash + data_len + data[..size]  (excludes padding and signature field)
    // Matches Keystone: sizeof(enclave_report) - SIGNATURE_SIZE - ATTEST_DATA_MAXLEN + size
    let sign_len = core::mem::size_of::<EnclaveReport>() - SIGNATURE_SIZE - ATTEST_DATA_MAXLEN + size;
    let mut signature = [0u8; SIGNATURE_SIZE];
    unsafe {
        sm_sign(
            &mut signature,
            core::slice::from_raw_parts(
                &report.enclave as *const EnclaveReport as *const u8,
                sign_len,
            ),
        );
    }
    report.enclave.signature = signature;

    // Copy report back to enclave virtual memory via MPRV
    unsafe {
        sm_mprv_write(
            report_ptr,
            &report as *const Report as *const u8,
            core::mem::size_of::<Report>(),
        );
    }

    Ok(())
}
