pub const MDSIZE: usize = 64;
pub const SIGNATURE_SIZE: usize = 64;
pub const PRIVATE_KEY_SIZE: usize = 64;
pub const PUBLIC_KEY_SIZE: usize = 32;
pub const ATTEST_DATA_MAXLEN: usize = 1024;
pub const RISCV_PGSIZE: usize = 4096;
pub const SEALING_KEY_SIZE: usize = 64;

// Matches sha3_ctx_t in sha3.h: union{u8[200]/u64[25]}, int pt, rsiz, mdlen
// With 8-byte alignment: 200 + 12 bytes of ints = 212, padded to 216.
#[repr(C, align(8))]
pub struct Sha3Ctx {
    pub st: [u8; 200],
    pub pt: i32,
    pub rsiz: i32,
    pub mdlen: i32,
    _pad: i32,
}

impl Sha3Ctx {
    pub const fn zeroed() -> Self {
        Sha3Ctx {
            st: [0u8; 200],
            pt: 0,
            rsiz: 0,
            mdlen: 0,
            _pad: 0,
        }
    }
}

extern "C" {
    fn sha3_init(ctx: *mut Sha3Ctx, mdlen: i32) -> i32;
    fn sha3_update(ctx: *mut Sha3Ctx, data: *const u8, len: usize) -> i32;
    fn sha3_final(md: *mut u8, ctx: *mut Sha3Ctx) -> i32;
    pub fn ed25519_create_keypair(public_key: *mut u8, private_key: *mut u8, seed: *const u8);
    fn ed25519_sign(
        signature: *mut u8,
        message: *const u8,
        message_len: usize,
        public_key: *const u8,
        private_key: *const u8,
    );
    pub fn hkdf_sha3_512(
        salt: *const u8,
        salt_len: i32,
        ikm: *const u8,
        ikm_len: i32,
        info: *const u8,
        info_len: i32,
        okm: *mut u8,
        okm_len: i32,
    ) -> i32;
}

pub fn hash_init(ctx: &mut Sha3Ctx) {
    unsafe { sha3_init(ctx as *mut Sha3Ctx, MDSIZE as i32) };
}

pub fn hash_extend(ctx: &mut Sha3Ctx, data: &[u8]) {
    unsafe { sha3_update(ctx as *mut Sha3Ctx, data.as_ptr(), data.len()) };
}

pub fn hash_extend_page(ctx: &mut Sha3Ctx, page_ptr: *const u8) {
    unsafe { sha3_update(ctx as *mut Sha3Ctx, page_ptr, RISCV_PGSIZE) };
}

pub fn hash_finalize(md: &mut [u8; MDSIZE], ctx: &mut Sha3Ctx) {
    unsafe { sha3_final(md.as_mut_ptr(), ctx as *mut Sha3Ctx) };
}

pub unsafe fn sm_sign(signature: &mut [u8; SIGNATURE_SIZE], data: &[u8]) {
    ed25519_sign(
        signature.as_mut_ptr(),
        data.as_ptr(),
        data.len(),
        crate::SM_PUBLIC_KEY.as_ptr(),
        crate::SM_PRIVATE_KEY.as_ptr(),
    );
}

// Test device keys from sdk/include/verifier/test_dev_key.h
// In production these are provisioned by the sanctum bootloader ROM.
const DEV_SECRET_KEY: [u8; 64] = [
    0x40, 0xa0, 0x99, 0x47, 0x8c, 0xce, 0xfa, 0x3a, 0x06, 0x63, 0xab,
    0xc9, 0x5e, 0x7a, 0x1e, 0xc9, 0x54, 0xb4, 0xf5, 0xf6, 0x45, 0xba,
    0xd8, 0x04, 0xdb, 0x13, 0xe7, 0xd7, 0x82, 0x6c, 0x70, 0x73, 0x57,
    0x6a, 0x9a, 0xb6, 0x21, 0x60, 0xd9, 0xd1, 0xc6, 0xae, 0xdc, 0x29,
    0x85, 0x2f, 0xb9, 0x60, 0xee, 0x51, 0x32, 0x83, 0x5a, 0x16, 0x89,
    0xec, 0x06, 0xa8, 0x72, 0x34, 0x51, 0xaa, 0x0e, 0x4a,
];
const DEV_PUBLIC_KEY_BYTES: [u8; 32] = [
    0x0f, 0xaa, 0xd4, 0xff, 0x01, 0x17, 0x85, 0x83, 0xba, 0xa5, 0x88,
    0x96, 0x6f, 0x7c, 0x1f, 0xf3, 0x25, 0x64, 0xdd, 0x17, 0xd7, 0xdc,
    0x2b, 0x46, 0xcb, 0x50, 0xa8, 0x4a, 0x69, 0x27, 0x0b, 0x4c,
];

// Matches the sanctum_sm_size used in verifier/verifier.cpp:compute_expected_sm_hash()
const SANCTUM_SM_SIZE: usize = 0x1ff000;
const FW_TEXT_START: usize = 0x80000000;

// Linker symbol marking start of .data (end of text+rodata).
// OpenSBI writes dynamic values (_load_start, _link_start) into .data before
// sm_init_keys() runs, so .data in memory != .data in the binary file.
// We measure ONLY text+rodata [FW_TEXT_START, _fw_rw_start), which is never
// modified at runtime and matches the binary file byte-for-byte.
extern "C" {
    static _fw_rw_start: u8;
}

pub fn sm_init_keys() {
    let fw_rw_start = unsafe { core::ptr::addr_of!(_fw_rw_start) as usize };
    let measure_end = fw_rw_start;

    let mut ctx = Sha3Ctx::zeroed();
    hash_init(&mut ctx);

    // Phase 1: hash text+rodata [FW_TEXT_START, _fw_rw_start) from memory
    let mut offset = 0usize;
    while FW_TEXT_START + offset < measure_end {
        let remaining = measure_end - FW_TEXT_START - offset;
        let chunk = remaining.min(RISCV_PGSIZE);
        unsafe {
            sha3_update(
                &mut ctx as *mut Sha3Ctx,
                (FW_TEXT_START + offset) as *const u8,
                chunk,
            );
        }
        offset += chunk;
    }

    // Phase 2: feed zeros for [_fw_rw_start, FW_TEXT_START+SANCTUM_SM_SIZE),
    // matching the verifier's zero-padded buffer beyond 0x40000.
    let zeros = [0u8; RISCV_PGSIZE];
    while offset < SANCTUM_SM_SIZE {
        let chunk = (SANCTUM_SM_SIZE - offset).min(RISCV_PGSIZE);
        unsafe {
            sha3_update(&mut ctx as *mut Sha3Ctx, zeros.as_ptr(), chunk);
        }
        offset += chunk;
    }

    unsafe {
        hash_finalize(&mut crate::SM_HASH, &mut ctx);
    }

    // Derive SM keypair from H(SK_D || H_SM) truncated to 32 bytes,
    // matching Keystone bootloader's key derivation (bootloader.c).
    let mut seed_ctx = Sha3Ctx::zeroed();
    hash_init(&mut seed_ctx);
    unsafe {
        sha3_update(
            &mut seed_ctx as *mut Sha3Ctx,
            DEV_SECRET_KEY.as_ptr(),
            DEV_SECRET_KEY.len(),
        );
        sha3_update(
            &mut seed_ctx as *mut Sha3Ctx,
            crate::SM_HASH.as_ptr(),
            MDSIZE,
        );
    }
    let mut seed_full = [0u8; MDSIZE];
    unsafe {
        sha3_final(seed_full.as_mut_ptr(), &mut seed_ctx as *mut Sha3Ctx);
    }
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&seed_full[..32]);

    unsafe {
        ed25519_create_keypair(
            crate::SM_PUBLIC_KEY.as_mut_ptr(),
            crate::SM_PRIVATE_KEY.as_mut_ptr(),
            seed.as_ptr(),
        );

        // Set device public key from hardcoded test key
        crate::DEV_PUBLIC_KEY.copy_from_slice(&DEV_PUBLIC_KEY_BYTES);

        // SM_SIGNATURE = ed25519_sign(SM_HASH || SM_PUBLIC_KEY, dev_secret_key)
        // This matches what checkSignaturesOnly() verifies in Report.cpp
        let mut msg = [0u8; MDSIZE + PUBLIC_KEY_SIZE];
        msg[..MDSIZE].copy_from_slice(&crate::SM_HASH);
        msg[MDSIZE..].copy_from_slice(&crate::SM_PUBLIC_KEY);
        ed25519_sign(
            crate::SM_SIGNATURE.as_mut_ptr(),
            msg.as_ptr(),
            MDSIZE + PUBLIC_KEY_SIZE,
            DEV_PUBLIC_KEY_BYTES.as_ptr(),
            DEV_SECRET_KEY.as_ptr(),
        );
    }
}
