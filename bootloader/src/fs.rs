use core::{ptr::NonNull, slice::from_raw_parts_mut};

use log::info;
use uefi::{
    boot::{AllocateType, MemoryType},
    proto::media::file::{Directory, File, FileAttribute, FileInfo, FileMode, RegularFile},
    CStr16, Status,
};

pub struct FileHelper<'a> {
    volume: Directory,
    info_buffer: &'a mut [u8],
    info_buffer_pages: usize,
}

impl<'a> FileHelper<'a> {
    pub fn new(default_info_size: usize) -> Self {
        let handle = uefi::boot::image_handle();
        let mut file_system = uefi::boot::get_image_file_system(handle).unwrap();
        let volume = file_system.open_volume().unwrap();

        // Allocate a buffer for the file info.
        let info_buffer_pages = (default_info_size + 0xfff) / 0x1000;
        let info_buffer = uefi::boot::allocate_pages(
            uefi::boot::AllocateType::AnyPages,
            MemoryType::BOOT_SERVICES_DATA,
            info_buffer_pages,
        )
        .unwrap();
        let info_buffer = unsafe { from_raw_parts_mut(info_buffer.as_ptr(), default_info_size) };

        FileHelper {
            volume,
            info_buffer,
            info_buffer_pages,
        }
    }

    pub fn open_file(
        &mut self,
        file_name: &CStr16,
        mode: FileMode,
    ) -> uefi::Result<(RegularFile, u64)> {
        let file = self.volume.open(file_name, mode, FileAttribute::empty())?;
        let mut regular_file = file
            .into_regular_file()
            .ok_or_else(|| uefi::Error::new(Status::NOT_FOUND, ()))?;
        loop {
            match regular_file.get_info::<FileInfo>(self.info_buffer) {
                Ok(info) => {
                    // Successfully retrieved the file info.

                    info!(
                        "Opened a file: {} (size: {:#x})",
                        info.file_name(),
                        info.file_size()
                    );

                    return Ok((regular_file, info.file_size()));
                }
                Err(e) => match e.status() {
                    Status::BUFFER_TOO_SMALL => {
                        info!(
                            "Allocate a new buffer for file info. Old: {:#x} New: {:#x}",
                            self.info_buffer.len(),
                            e.data().unwrap()
                        );

                        // Release the too small buffer.
                        unsafe {
                            uefi::boot::free_pages(
                                NonNull::new(self.info_buffer.as_mut_ptr()).unwrap(),
                                self.info_buffer_pages,
                            )?;
                        }
                        // Allocate a new buffer.
                        let required_size = e.data().unwrap();
                        self.info_buffer_pages = (required_size + 0xfff) / 0x1000;
                        let info_buffer = uefi::boot::allocate_pages(
                            uefi::boot::AllocateType::AnyPages,
                            MemoryType::BOOT_SERVICES_DATA,
                            self.info_buffer_pages,
                        )
                        .unwrap();
                        self.info_buffer =
                            unsafe { from_raw_parts_mut(info_buffer.as_ptr(), required_size) }
                    }
                    _ => return Err(e.to_err_without_payload()),
                },
            }
        }
    }

    pub fn read_file(
        &mut self,
        file_name: &CStr16,
        alloc_ty: AllocateType,
        mem_ty: MemoryType,
    ) -> uefi::Result<&mut [u8]> {
        let (mut file, size) = self.open_file(file_name, FileMode::Read)?;

        // Allocate a buffer for the file content.
        let buffer =
            uefi::boot::allocate_pages(alloc_ty, mem_ty, (size as usize + 0xfff) / 0x1000).unwrap();
        let buffer = unsafe { from_raw_parts_mut(buffer.as_ptr(), size as usize) };

        let size = file.read(buffer)?;

        info!("Read {:#x} bytes to {:#x}", size, buffer.as_ptr() as u64);

        Ok(buffer)
    }
}
