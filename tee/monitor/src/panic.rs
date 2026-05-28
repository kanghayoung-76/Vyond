use core::panic::PanicInfo;
use semihosting::heprintln;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(loc) = info.location() {
        heprintln!("[PANIC] {}:{}", loc.file(), loc.line());
    } else {
        heprintln!("[PANIC] (no location)");
    }
    loop {}
}
