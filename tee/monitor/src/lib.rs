#![no_std]
#![no_main]

use core::sync::atomic::compiler_fence;
use core::sync::atomic::Ordering;

use once_cell::OnceCell;

#[macro_use]
pub mod cpu;

pub mod api;
pub mod attest;
pub mod crypto;
pub mod enclave;
pub mod ipi;
pub mod encoding;
pub mod isolator;
pub mod log;
pub mod once_cell;
pub mod panic;
#[cfg(any(feature = "isolator_pmp", feature = "isolator_hybrid"))]
pub mod pmp;
pub mod shm;
pub mod spinlock;
pub mod thread;
pub mod trap;
#[cfg(any(feature = "isolator_wg", feature = "isolator_hybrid"))]
pub mod wg;
#[cfg(any(feature = "isolator_wg", feature = "isolator_hybrid"))]
pub mod wid;

// SM key material — populated by sm_init on cold boot.
// In production these should be provisioned by the bootloader (sanctum ROM).
pub static mut SM_HASH: [u8; crypto::MDSIZE] = [0u8; crypto::MDSIZE];
pub static mut SM_SIGNATURE: [u8; crypto::SIGNATURE_SIZE] = [0u8; crypto::SIGNATURE_SIZE];
pub static mut SM_PUBLIC_KEY: [u8; crypto::PUBLIC_KEY_SIZE] = [0u8; crypto::PUBLIC_KEY_SIZE];
pub static mut SM_PRIVATE_KEY: [u8; crypto::PRIVATE_KEY_SIZE] = [0u8; crypto::PRIVATE_KEY_SIZE];
pub static mut DEV_PUBLIC_KEY: [u8; crypto::PUBLIC_KEY_SIZE] = [0u8; crypto::PUBLIC_KEY_SIZE];

#[derive(Debug, PartialEq)]
pub enum Error {
    Success = 0,
    Unknown = 100000,
    InvalidId,
    Interrupted,
    PmpFailure,
    NotRunnable,
    NotDestroyable,
    RegionOverlaps,
    NotAccessible,
    IllegalArgument,
    NotRunning,
    NotResumable,
    EdgeCallHost,
    NotInitialized,
    NoFreeResource,
    SBIProhibited,
    IllegalPTE,
    NotFresh,
    WaitingForShm       = 100018,
    IpiHandled          = 100019,
    TimerInterrupted    = 100027,
    RegionSizeInvalid   = 10020,
    NotPageGranularity,
    NotAligned,
    MaxReached,
    RegionInvalid,
    RegionOverlap,
    ImpossibleTor,
    Deprecated = 100099,
    NotImplemented,
    // Above enum's are identical to sm_err.h in sdk
    Overlap,
    NotSupported,
    Invalid,
}

#[no_mangle]
pub extern "C" fn sm_init(cold_boot: bool) -> isize {
    let hartid = csr_read!(mhartid);

    // initialize SMM
    if cold_boot {
        crate::wid::wid_init();

        if let Err(e) = isolator::smm_init() {
            return -1;
        }

        if let Err(e) = isolator::osm_init() {
            return -1;
        }

        crypto::sm_init_keys();

        isolator::sm_init_done();

        //isolator::display_isolator();
        compiler_fence(Ordering::Release);
    }

    /* wait until cold-boot hart finishes */
    isolator::sm_wait_for_completion();

    /* below are executed by all harts */
    if let Err(e) = isolator::update() {
        return -1;
    }
    isolator::display_isolator();

    0
}
