#![no_main]
#![no_std]
#![feature(abi_x86_interrupt)]

extern crate alloc;

use core::arch::asm;
use core::panic::PanicInfo;

use x86_64::instructions::interrupts;

use crate::font::FontManager;
use crate::frame_buffer::FrameBuffer;
use crate::int::Interrupt;

#[no_mangle]
#[inline(never)]
pub extern "C" fn kernel_main() -> ! {
    if interrupts::are_enabled() {
        interrupts::disable();
    }

    // Initialize the frame buffer.
    let frame_buffer = FrameBuffer::get();
    let grey = frame_buffer.make_color(0x20, 0x20, 0x20);
    for y in 0..frame_buffer.height {
        for x in 0..frame_buffer.width {
            frame_buffer.set_pixel(x, y, grey);
        }
    }

    // Load the font data.
    // This task is required to render texts on the screen.
    let header = FontManager::get_psf_header();
    if !FontManager::validate_psf_header(header) {
        panic!("Found an invalid PSF header.");
    }
    FontManager::init_glyph_index_table();

    Interrupt::init();

    let white = frame_buffer.make_color(0xff, 0xff, 0xff);

    let text = b"Hello World from the kernel.";
    frame_buffer.draw_text(0, 0, text, white);

    interrupts::enable();

    let text = b"Interrupts are enabled.";
    frame_buffer.draw_text(0, 16, text, white);

    let text = b"It's time to enjoy BRIGHTNES!";
    frame_buffer.draw_text(0, 32, text, white);

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

mod font;
mod frame_buffer;
mod int;
mod mem;
