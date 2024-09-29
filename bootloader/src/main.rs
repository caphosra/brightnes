#![no_main]
#![no_std]

use core::slice::from_raw_parts_mut;

use log::info;
use uefi::boot::{MemoryType, AllocateType};
use uefi::prelude::*;
use uefi::proto::media::file::{File, FileInfo};
use uefi::proto::media::file::FileAttribute;
use uefi::proto::media::file::FileMode;

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    info!("Hello world (^^)/");

    let handle = uefi::boot::image_handle();
    let mut file_system = uefi::boot::get_image_file_system(handle).unwrap();
    let mut volume = file_system.open_volume().unwrap();

    let mut kernel_file = volume.open(cstr16!("kernel"), FileMode::Read, FileAttribute::empty()).unwrap();

    let file_info_size = 0x1000;
    let info_buffer = uefi::boot::allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, file_info_size).unwrap();
    let info_buffer = unsafe {
        from_raw_parts_mut(info_buffer.as_ptr(), file_info_size)
    };
    let info = kernel_file.get_info::<FileInfo>(info_buffer).unwrap();

    info!("Kernel file name: {}", info.file_name());
    info!("Kernel file size: {}", info.file_size());

    boot::stall(10_000_000);

    Status::SUCCESS
}
