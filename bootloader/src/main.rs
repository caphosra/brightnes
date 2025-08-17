#![no_main]
#![no_std]

use core::arch::asm;
use core::mem::transmute;
use core::slice::from_raw_parts_mut;

use frame_buffer::FrameBuffer;
use log::info;
use uefi::boot::{get_handle_for_protocol, open_protocol_exclusive, AllocateType, MemoryType};
use uefi::proto::console::gop::GraphicsOutput;
use uefi::proto::media::file::{File, FileAttribute, FileInfo, FileMode};
use uefi::{cstr16, entry, Status};

use crate::elf::{extract_elf_program, load_elf_header};

const KERNEL_DATA_ADDR: u64 = 0x400000;
const FILE_INFO_SIZE: usize = 0x1000;
const STALL_TIME: usize = 1_000;

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    info!("Hello world (^^)/");

    let handle = uefi::boot::image_handle();
    let mut file_system = uefi::boot::get_image_file_system(handle).unwrap();
    let mut volume = file_system.open_volume().unwrap();

    let mut kernel_file = volume
        .open(cstr16!("kernel"), FileMode::Read, FileAttribute::empty())
        .unwrap()
        .into_regular_file()
        .unwrap();

    let file_info_size = FILE_INFO_SIZE;
    let info_buffer = uefi::boot::allocate_pages(
        AllocateType::AnyPages,
        MemoryType::BOOT_SERVICES_DATA,
        file_info_size / 0x1000,
    )
    .unwrap();
    let info_buffer = unsafe { from_raw_parts_mut(info_buffer.as_ptr(), file_info_size) };
    let info = kernel_file.get_info::<FileInfo>(info_buffer).unwrap();

    info!("Kernel file name: {}", info.file_name());
    info!("Kernel file size: {}", info.file_size());

    let kernel_size = info.file_size() as usize;
    let kernel_buffer = uefi::boot::allocate_pages(
        AllocateType::Address(KERNEL_DATA_ADDR),
        MemoryType::BOOT_SERVICES_DATA,
        (kernel_size + 0xfff) / 0x1000,
    )
    .unwrap();
    let kernel_buffer = unsafe { from_raw_parts_mut(kernel_buffer.as_ptr(), kernel_size) };

    let read_size = kernel_file.read(kernel_buffer).unwrap();
    info!("Read {} bytes to {:p}", read_size, kernel_buffer.as_ptr());

    let elf_header = load_elf_header(kernel_buffer).unwrap();

    info!("Entry: {:#x}", elf_header.e_entry);

    extract_elf_program(kernel_buffer, elf_header);

    info!("Loaded the kernel");

    let gop_handle = get_handle_for_protocol::<GraphicsOutput>().unwrap();

    let mut gop = open_protocol_exclusive::<GraphicsOutput>(gop_handle);
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

    let entry_point: extern "sysv64" fn() -> ! = unsafe { transmute(elf_header.e_entry) };
    entry_point();
}

mod elf;
mod frame_buffer;
