#![no_main]
#![no_std]
#![feature(abi_x86_interrupt)]

extern crate alloc;

use core::panic::PanicInfo;

use x86_64::instructions::{hlt, interrupts};

use crate::font::FontManager;
use crate::info::InfoProc;
use crate::int::InterruptController;
use crate::int::PANIC_INT_IDX;
use crate::logger::LOG_FB;
use crate::nes::cartridge::CARTRIDGE;
use crate::nes::cpu::InterruptType;
use crate::nes::cpu::NES_CPU;
use crate::nes::ppu::{GAME_FB, NES_PPU};

#[no_mangle]
#[inline(never)]
pub extern "C" fn kernel_main() -> ! {
    if interrupts::are_enabled() {
        interrupts::disable();
    }

    // Initialize the frame buffer.
    on_game_switched();

    // Load the font data.
    // This task is required to render texts on the screen.
    let header = FontManager::get_psf_header();
    if !FontManager::validate_psf_header(header) {
        panic!("Found an invalid PSF header.");
    }
    FontManager::init_glyph_index_table();

    log!(SYS, "Hello World from the kernel.");
    info!(SYS, "Enabled logging system.");

    InterruptController::init();
    interrupts::enable();

    info!(SYS, "Interrupts are enabled.");
    log!(SYS, "It's time to enjoy BRIGHTNES!");

    game_main();
}

pub fn game_main() -> ! {
    let mut cartridge = CARTRIDGE.write();
    let mut cpu = NES_CPU.write();
    let mut ppu = NES_PPU.write();
    let mut frame_buffer = GAME_FB.write();

    cpu.interrupt(InterruptType::RST, &mut ppu, &mut cartridge);

    info!(SYS, "Start the game.");

    loop {
        const FRAME_CYCLES: usize = 29780;

        let mut cycles = 0;
        while cycles < FRAME_CYCLES {
            let required = cpu.clock(&mut ppu, &mut cartridge);
            ppu.render_bg(
                required as usize * 3,
                &mut frame_buffer,
                &mut cpu,
                &mut cartridge,
            );

            cycles += required as usize;
        }
        frame_buffer.flush(false);
    }
}

pub fn on_game_switched() {
    unsafe {
        GAME_FB.force_write_unlock();
    }
    let mut buffer = GAME_FB.write();
    buffer.flush_all();
}

pub fn log_main() -> ! {
    loop {
        hlt();
    }
}

pub fn on_log_switched() {
    interrupts::without_interrupts(|| {
        LOG_FB.write().flush_all();
    });
}

pub fn info_main() -> ! {
    loop {
        hlt();
    }
}

pub fn on_info_switched() {
    InfoProc::render_all();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!(SYS, "Kernel panic: {}", info);

    unsafe {
        NES_CPU.force_write_unlock();
    }
    let cpu = NES_CPU.read();
    error!(CPU, "PC: {:#06X}", cpu.reg_pc);
    cpu.report_backtrace();

    interrupts::enable();
    unsafe {
        interrupts::software_interrupt::<PANIC_INT_IDX>();
    }
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
mod serial;
