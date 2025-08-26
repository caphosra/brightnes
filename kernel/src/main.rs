#![no_main]
#![no_std]
#![feature(abi_x86_interrupt)]

extern crate alloc;

use core::panic::PanicInfo;

use x86_64::instructions::{hlt, interrupts};

use crate::font::FontManager;
use crate::int::Interrupt;
use crate::logger::Logger;
use crate::nes::cpu::NES_CPU;
use crate::nes::rom::NESROM;

#[no_mangle]
#[inline(never)]
pub extern "C" fn kernel_main() -> ! {
    if interrupts::are_enabled() {
        interrupts::disable();
    }

    // Initialize the frame buffer.
    Logger::render_all();

    // Load the font data.
    // This task is required to render texts on the screen.
    let header = FontManager::get_psf_header();
    if !FontManager::validate_psf_header(header) {
        panic!("Found an invalid PSF header.");
    }
    FontManager::init_glyph_index_table();

    log!("[SYS] Hello World from the kernel.");

    NESROM::load();
    {
        let mut cpu = NES_CPU.write();
        cpu.reg_pc = 0xC000;
    }

    log!("[SYS] Initialized the NES CPU.");

    Interrupt::init();
    interrupts::enable();

    log!("[SYS] Interrupts are enabled.");
    log!("[SYS] It's time to enjoy BRIGHTNES!");

    loop {
        hlt();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

mod font;
mod frame_buffer;
mod info;
mod int;
mod logger;
mod mem;
mod nes;
mod proc;
