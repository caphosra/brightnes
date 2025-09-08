#![no_main]
#![no_std]
#![feature(abi_x86_interrupt)]

extern crate alloc;

use core::panic::PanicInfo;

use x86_64::instructions::{hlt, interrupts};

use crate::font::FontManager;
use crate::info::InfoProc;
use crate::int::Interrupt;
use crate::logger::Logger;
use crate::nes::bus::CPUBus;
use crate::nes::cartridge::CARTRIDGE;
use crate::nes::cpu::NES_CPU;
use crate::nes::ppu::{GAME_FB, NES_PPU};
use crate::proc::{Process, ProcessMode};

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

    log!(SYS, "Hello World from the kernel.");
    info!(SYS, "Enabled logging system.");

    {
        let mut cartridge = CARTRIDGE.write();
        cartridge.load();
    }
    info!(SYS, "Loaded the cartridge.");

    {
        let mut cpu = NES_CPU.write();
        let mut cartridge = CARTRIDGE.write();
        let lo = CPUBus::read(0xFFFC, &mut cartridge);
        let hi = CPUBus::read(0xFFFD, &mut cartridge);
        cpu.reg_pc = u16::from_le_bytes([lo, hi]);

        log!(SYS, "Entry Point: {:#06X}", cpu.reg_pc);
    }
    info!(SYS, "Initialized the NES CPU.");

    Interrupt::init();
    interrupts::enable();

    info!(SYS, "Interrupts are enabled.");
    log!(SYS, "It's time to enjoy BRIGHTNES!");

    loop {
        main_loop();
    }
}

fn main_loop() {
    match Process::status() {
        (ProcessMode::Log, true) => {
            Logger::render_all();
            Process::mark_as_switched();
        }
        (ProcessMode::Recovery, true) => {
            Logger::render_all();
            Process::mark_as_switched();
        }
        (ProcessMode::Game, true) => {
            let mut buffer = GAME_FB.write();
            buffer.flush_all();
            Process::mark_as_switched();
        }
        (ProcessMode::Info, true) => {
            InfoProc::render_all();
            Process::mark_as_switched();
        }
        (ProcessMode::Game, _) => {
            const FRAME_CYCLES: usize = 29780;

            let mut cartridge = CARTRIDGE.write();
            let mut cpu = NES_CPU.write();
            let mut frame_buffer = GAME_FB.write();

            let mut cycles = 0;
            while cycles < FRAME_CYCLES {
                let required = cpu.clock(&mut cartridge) as usize;
                {
                    let mut ppu = NES_PPU.write();
                    ppu.render_bg(required * 3, &mut frame_buffer, &mut cartridge);
                }
                cycles += required;
            }
            {
                let mut ppu = NES_PPU.write();
                ppu.complete_rendering(&mut frame_buffer, &mut cartridge);
            }
            frame_buffer.flush(false);
        }
        _ => {
            hlt();
        }
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
