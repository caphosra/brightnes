#![no_main]
#![no_std]
#![feature(abi_x86_interrupt)]

extern crate alloc;

use core::panic::PanicInfo;

use x86_64::instructions::{hlt, interrupts};

use crate::drivers::SoundDeviceDriver;
use crate::font::FontManager;
use crate::fs::FILE_SYSTEM;
use crate::info::InfoProc;
use crate::int::InterruptController;
use crate::int::PANIC_INT_IDX;
use crate::int::SLEEP;
use crate::logger::LOG_FB;
use crate::mem::MemoryAllocator;
use crate::nes::apu::APU;
use crate::nes::cartridge::Cartridge;
use crate::nes::cpu::InterruptType;
use crate::nes::cpu::CPU;
use crate::nes::ppu::GAME_FB;
use crate::nes::ppu::PPU;
use crate::proc::ProcessSwitcher;
use crate::system::SYSTEM;
use crate::system::SYSTEM_FB;

#[no_mangle]
#[inline(never)]
pub extern "C" fn kernel_main() -> ! {
    // Set the stack pointer.
    ProcessSwitcher::reset_main_stack();

    // Initialize the memory allocator.
    MemoryAllocator::init();

    if interrupts::are_enabled() {
        interrupts::disable();
    }

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
        let mut fs = FILE_SYSTEM.write();
        fs.check_root_dir();
    }

    {
        let mut fb = SYSTEM_FB.write();
        fb.flush_all();
    }

    {
        let mut sys = SYSTEM.write();
        sys.update_cartridges();
        sys.render(true);
    }

    InterruptController::init(60);
    interrupts::enable();

    info!(SYS, "Interrupts are enabled.");
    log!(SYS, "It's time to enjoy BRIGHTNES!");

    let cpu = CPU::get();
    cpu.init();

    cpu.interrupt(InterruptType::RST);

    Cartridge::get();

    let ppu = PPU::get();
    ppu.init();

    let apu = APU::get();
    apu.init();

    on_system_switched();
    loop {
        {
            let mut fb = SYSTEM_FB.write();
            fb.flush(false);
        }
        hlt();
    }
}

#[inline(never)]
pub fn on_system_switched() {
    {
        let mut fb = SYSTEM_FB.write();
        fb.flush_all();
    }
    interrupts::without_interrupts(|| {
        let mut sys = SYSTEM.write();
        sys.update_cartridges();
        sys.render(true);
    });
}

pub fn game_main() -> ! {
    let mut sound = SoundDeviceDriver::new();

    let cpu = CPU::get();
    let ppu = PPU::get();
    let apu = APU::get();
    let cartridge = Cartridge::get();

    let mut frame_buffer = GAME_FB.write();

    info!(SYS, "Start the game.");

    let mut total_cycles = 0;

    loop {
        const FRAME_CYCLES: usize = 29780;

        while total_cycles < FRAME_CYCLES {
            interrupts::disable();

            let cycles = cpu.clock(ppu, apu, cartridge);
            ppu.render_bg(cycles as usize * 3, &mut frame_buffer, cpu, cartridge);
            apu.clock(cycles, &mut sound, cpu, ppu, cartridge);

            interrupts::enable();

            total_cycles += cycles as usize;
        }
        total_cycles -= FRAME_CYCLES;

        frame_buffer.flush(false);

        // Sleep until the next interrupt comes.
        while *SLEEP.lock() {
            hlt();
        }
        *SLEEP.lock() = true;
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

    let cpu = CPU::get();
    error!(CPU, "PC: {:#06X}", cpu.reg_pc);
    cpu.report_backtrace();

    interrupts::enable();
    unsafe {
        interrupts::software_interrupt::<PANIC_INT_IDX>();
    }
    loop {}
}

mod drivers;
mod font;
mod frame_buffer;
mod fs;
mod info;
mod int;
mod logger;
mod mem;
mod nes;
mod proc;
#[cfg(feature = "serial")]
mod serial;
mod system;
