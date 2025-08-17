#![no_main]
#![no_std]

use core::arch::asm;
use core::panic::PanicInfo;

use crate::frame_buffer::FrameBuffer;

#[no_mangle]
#[inline(never)]
pub extern "C" fn kernel_main() -> ! {
    let frame_buffer = FrameBuffer::get();
    let green = frame_buffer.make_color(0x00, 0xFF, 0x00);
    for y in 0..frame_buffer.height {
        for x in 0..frame_buffer.width {
            frame_buffer.set_pixel(x, y, green);
        }
    }

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

mod frame_buffer;
