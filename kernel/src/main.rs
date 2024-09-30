#![no_main]
#![no_std]

use core::arch::asm;
use core::panic::PanicInfo;

#[no_mangle]
#[inline(never)]
pub extern "C" fn kernel_main() -> ! {
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
