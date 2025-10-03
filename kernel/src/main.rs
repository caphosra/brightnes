#![no_main]
#![no_std]
#![feature(abi_x86_interrupt)]

extern crate alloc;

use core::panic::PanicInfo;
use core::ptr::slice_from_raw_parts_mut;

use x86_64::instructions::{hlt, interrupts};

use crate::drivers::virtio::block::VirtBlockDevice;
use crate::font::FontManager;
use crate::info::InfoProc;
use crate::int::InterruptController;
use crate::int::PANIC_INT_IDX;
use crate::logger::LOG_FB;
use crate::mem::MemoryAllocator;
use crate::nes::apu::APU;
use crate::nes::cartridge::Cartridge;
use crate::nes::cpu::InterruptType;
use crate::nes::cpu::CPU;
use crate::nes::ppu::GAME_FB;
use crate::nes::ppu::PPU;
use crate::proc::ProcessSwitcher;

#[no_mangle]
#[inline(never)]
pub extern "C" fn kernel_main() -> ! {
    // Set the stack pointer.
    ProcessSwitcher::reset_main_stack();

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

    let mut block_device = VirtBlockDevice::new().unwrap();
    let buffer_size = block_device.sector_size() as usize;
    let buffer = unsafe {
        slice_from_raw_parts_mut(MemoryAllocator::alloc_bytes(buffer_size), buffer_size).as_mut()
    }
    .unwrap();

    block_device.read(0, buffer).unwrap();

    buffer[0] = 0x41;
    block_device.write(0, buffer).unwrap();

    InterruptController::init();
    interrupts::enable();

    info!(SYS, "Interrupts are enabled.");
    log!(SYS, "It's time to enjoy BRIGHTNES!");

    let cpu = CPU::get();
    cpu.init();

    cpu.interrupt(InterruptType::RST);

    let cartridge = Cartridge::get();
    cartridge.init();

    let ppu = PPU::get();
    ppu.init();

    let apu = APU::get();
    apu.init();

    game_main();
}

pub fn game_main() -> ! {
    let cpu = CPU::get();
    let ppu = PPU::get();
    let apu = APU::get();
    let cartridge = Cartridge::get();

    let mut frame_buffer = GAME_FB.write();

    info!(SYS, "Start the game.");

    loop {
        const FRAME_CYCLES: usize = 29780;

        let mut cycles = 0;

        while cycles < FRAME_CYCLES {
            interrupts::disable();

            let required = cpu.clock(ppu, apu, cartridge);
            ppu.render_bg(required as usize * 3, &mut frame_buffer, cpu, cartridge);
            apu.clock(cycles, cpu);

            interrupts::enable();

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
mod info;
mod int;
mod logger;
mod mem;
mod nes;
mod proc;
mod serial;
