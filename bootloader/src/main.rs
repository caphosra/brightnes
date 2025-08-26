#![no_main]
#![no_std]

use core::mem::transmute;

use log::{error, info};
use uefi::boot::{
    get_handle_for_protocol, memory_map, open_protocol_exclusive, AllocateType, MemoryType,
};
use uefi::mem::memory_map::MemoryMap;
use uefi::proto::console::gop::GraphicsOutput;
use uefi::{cstr16, entry, Status};

use crate::elf::{extract_elf_program, load_elf_header, resolve_global_offset_table};
use crate::frame_buffer::FrameBuffer;
use crate::fs::FileHelper;

const FONT_DATA_ADDR: u64 = 0x2_000_000;
const NES_FILE_ADDR: u64 = 0x3_000_000;
const FILE_INFO_SIZE: usize = 0x1000;
const STALL_TIME: usize = 1_000;

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    info!("Hello world (^^)/");

    let mut file_helper = FileHelper::new(FILE_INFO_SIZE);

    let kernel_file = file_helper.read_file(
        cstr16!("kernel"),
        AllocateType::AnyPages,
        MemoryType::BOOT_SERVICES_DATA,
    );
    if kernel_file.is_err() {
        error!("Failed to read the kernel file: {:?}", kernel_file.err());
        return Status::NOT_FOUND;
    }
    let kernel_file = kernel_file.unwrap();

    let elf_header = load_elf_header(kernel_file).unwrap();

    let entry = elf_header.e_entry;
    info!("Found the entry: {:#x}", entry);

    let mem_map = memory_map(MemoryType::LOADER_DATA).unwrap();
    for mem_idx in 0..mem_map.len() {
        let descriptor = mem_map.get(mem_idx).unwrap();
        match descriptor.ty {
            MemoryType::CONVENTIONAL => {
                info!(
                    "Mem: {:?} {:#x}-{:#x} ({:#x} bytes)",
                    descriptor.ty,
                    descriptor.phys_start,
                    descriptor.phys_start + descriptor.page_count * 0x1000,
                    descriptor.page_count * 0x1000,
                );
            }
            _ => {}
        }
    }

    extract_elf_program(kernel_file, elf_header);
    resolve_global_offset_table(kernel_file, elf_header);

    info!("Loaded the kernel");

    let font_file = file_helper.read_file(
        cstr16!("system_font.psf"),
        AllocateType::Address(FONT_DATA_ADDR),
        MemoryType::BOOT_SERVICES_DATA,
    );
    if font_file.is_err() {
        error!("Failed to read the system font file: {:?}", font_file.err());
        return Status::NOT_FOUND;
    }

    info!("Loaded the system font");

    let nes_file = file_helper.read_file(
        cstr16!("game.nes"),
        AllocateType::Address(NES_FILE_ADDR),
        MemoryType::BOOT_SERVICES_DATA,
    );
    if nes_file.is_err() {
        error!("Failed to read the NES file: {:?}", nes_file.err());
        return Status::NOT_FOUND;
    }

    let gop_handle = get_handle_for_protocol::<GraphicsOutput>().unwrap();

    let gop = open_protocol_exclusive::<GraphicsOutput>(gop_handle);
    if gop.is_err() {
        info!("Failed to open GraphicsOutput protocol: {:?}", gop.err());
        return Status::UNSUPPORTED;
    }
    info!("Opened graphic output protocol");

    let mut gop = gop.unwrap();

    let frame_buffer = FrameBuffer::new();
    frame_buffer.init(&mut gop);

    uefi::boot::stall(STALL_TIME);

    unsafe {
        let _ = uefi::boot::exit_boot_services(MemoryType::BOOT_SERVICES_DATA);
    }

    let entry_point: extern "sysv64" fn() -> ! = unsafe { transmute(entry) };
    entry_point();
}

mod elf;
mod frame_buffer;
mod fs;
