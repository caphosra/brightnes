#![no_main]
#![no_std]

use core::arch::asm;
use core::panic::PanicInfo;

use crate::font::FontManager;
use crate::frame_buffer::FrameBuffer;

#[no_mangle]
#[inline(never)]
pub extern "C" fn kernel_main() -> ! {
    let frame_buffer = FrameBuffer::get();
    let grey = frame_buffer.make_color(0x20, 0x20, 0x20);
    for y in 0..frame_buffer.height {
        for x in 0..frame_buffer.width {
            frame_buffer.set_pixel(x, y, grey);
        }
    }

    let header = FontManager::get_psf_header();
    FontManager::validate_psf_header(header);
    FontManager::init_glyph_index_table();

    let white = frame_buffer.make_color(0xff, 0xff, 0xff);

    let text = b"Hello World";
    for (i, &c) in text.iter().enumerate() {
        let glyph = FontManager::get_glyph_by_char(c);
        frame_buffer.draw_glyph(i * 8, 0, glyph, white);
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

mod font;
mod frame_buffer;
