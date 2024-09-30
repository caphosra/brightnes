#![no_main]
#![no_std]

use core::arch::asm;
use core::panic::PanicInfo;

use frame_buffer::FrameBuffer;

use crate::frame_buffer::NativeFrameBuffer;

#[no_mangle]
#[inline(never)]
pub extern "C" fn kernel_main(frame_buffer: *mut NativeFrameBuffer) -> ! {
    let mut frame_buffer: FrameBuffer = frame_buffer.into();
    for y in 0..frame_buffer.height {
        for x in 0..frame_buffer.width {
            frame_buffer.set_pixel(x, y, 0x00, 0xff, 0x00);
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
