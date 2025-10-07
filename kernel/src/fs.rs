use core::slice::from_raw_parts_mut;

use alloc::vec;
use crc::{Crc, CRC_32_ISCSI};
use fatfs::{Error, FsOptions, Read, Write};
use postcard::{from_bytes_crc32, to_allocvec_crc32};
use spin::{Lazy, RwLock};

use crate::{
    critical,
    drivers::BlockDeviceDriver,
    error, info,
    nes::{cartridge::Cartridge, cpu::CPU, ppu::PPU},
};

type FATFileSystem<T> = fatfs::FileSystem<T>;

pub static FILE_SYSTEM: Lazy<RwLock<FileSystem>> = Lazy::new(|| {
    let fs = FileSystem::new();
    RwLock::new(fs)
});

pub struct FileSystem {
    file_system: FATFileSystem<BlockDeviceDriver<'static>>,
}

unsafe impl Send for FileSystem {}
unsafe impl Sync for FileSystem {}

impl FileSystem {
    const STATE_FILE_NAME: &'static str = "saved.brt";
    const RAM_FILE_NAME: &'static str = "ram.brr";

    pub fn new() -> Self {
        let driver = BlockDeviceDriver::new();
        let option = FsOptions::new().strict(true);
        let file_system = FATFileSystem::new(driver, option).unwrap();
        FileSystem { file_system }
    }

    pub fn load_cartridge(&mut self, path: &str) {
        const BUF_SIZE: usize = 1024;

        let root_dir = self.file_system.root_dir();
        let file = root_dir.open_file(path);
        match file {
            Ok(mut file) => {
                let mut loaded_bytes = 0;
                let mut nes_file_ptr = Cartridge::NES_FILE_ADDR as *mut u8;
                loop {
                    let nes_file_data = unsafe { from_raw_parts_mut(nes_file_ptr, BUF_SIZE) };
                    match file.read(nes_file_data) {
                        Ok(0) => break,
                        Ok(n) => {
                            nes_file_ptr = unsafe { nes_file_ptr.add(n) };
                            loaded_bytes += n;
                        }
                        Err(_) => {
                            critical!(CAT, "Failed to load the cartridge: {}", path);
                        }
                    }
                }
                info!(CAT, "Loaded the cartridge. ({} bytes)", loaded_bytes);
            }
            Err(Error::NotFound) => {
                error!(CAT, "The specified cartridge was not found: {}", path);
            }
            _ => {
                error!(CAT, "Failed to open the cartridge.");
            }
        }
    }

    pub fn check_root_dir(&mut self) {
        let root_dir = self.file_system.root_dir();
        let entries = root_dir.iter();
        for entry in entries {
            if let Ok(entry) = entry {
                info!(SYS, "Found file: {}", entry.file_name());
            }
        }
    }

    pub fn save_state(&mut self, cpu: &CPU, ppu: &PPU, cartridge: &Cartridge) -> Result<(), ()> {
        info!(SYS, "Request to save state");

        let crc = Crc::<u32>::new(&CRC_32_ISCSI);

        let root_dir = self.file_system.root_dir();
        let mut file = root_dir
            .create_file(Self::STATE_FILE_NAME)
            .map_err(|_| ())?;

        // The file should be overwritten.
        file.truncate().map_err(|_| ())?;

        let serialized_cpu = to_allocvec_crc32(cpu, crc.digest());
        let serialized_cpu = serialized_cpu.map_err(|_| {
            error!(SYS, "Failed to serialize CPU state");
            ()
        })?;

        file.write_all(&serialized_cpu.len().to_le_bytes())
            .map_err(|_| ())?;
        file.write_all(&serialized_cpu).map_err(|_| ())?;

        info!(SYS, "Saved CPU state ({} bytes)", serialized_cpu.len());

        let serialized_ppu = to_allocvec_crc32(ppu, crc.digest());
        let serialized_ppu = serialized_ppu.map_err(|_| {
            error!(SYS, "Failed to serialize CPU state");
            ()
        })?;

        file.write_all(&serialized_ppu.len().to_le_bytes())
            .map_err(|_| ())?;
        file.write_all(&serialized_ppu).map_err(|_| ())?;

        info!(SYS, "Saved PPU state ({} bytes)", serialized_ppu.len());

        let serialized_cartridge = to_allocvec_crc32(cartridge, crc.digest());
        let serialized_cartridge = serialized_cartridge.map_err(|_| {
            error!(SYS, "Failed to serialize cartridge state");
            ()
        })?;

        file.write_all(&serialized_cartridge.len().to_le_bytes())
            .map_err(|_| ())?;
        file.write_all(&serialized_cartridge).map_err(|_| ())?;

        info!(
            SYS,
            "Saved cartridge state ({} bytes)",
            serialized_cartridge.len()
        );

        file.flush().map_err(|_| ())?;

        Ok(())
    }

    pub fn load_state(
        &mut self,
        cpu: &mut CPU,
        ppu: &mut PPU,
        cartridge: &mut Cartridge,
    ) -> Result<(), ()> {
        info!(SYS, "Request to load saved state");

        let crc = Crc::<u32>::new(&CRC_32_ISCSI);

        let root_dir = self.file_system.root_dir();
        let mut file = root_dir.open_file(Self::STATE_FILE_NAME).map_err(|_| ())?;

        let mut file_size_buf = [0u8; size_of::<usize>()];

        file.read_exact(&mut file_size_buf).map_err(|_| ())?;
        let cpu_size = usize::from_le_bytes(file_size_buf);

        let mut cpu_buf = vec![0; cpu_size];
        file.read_exact(&mut cpu_buf).map_err(|_| ())?;

        info!(SYS, "Received CPU state ({} bytes)", cpu_size);

        *cpu = from_bytes_crc32(&cpu_buf, crc.digest()).map_err(|_| {
            error!(SYS, "Failed to deserialize CPU state");
            ()
        })?;

        file.read_exact(&mut file_size_buf).map_err(|_| ())?;
        let ppu_size = usize::from_le_bytes(file_size_buf);

        let mut ppu_buf = vec![0; ppu_size];
        file.read_exact(&mut ppu_buf).map_err(|_| ())?;

        info!(SYS, "Received PPU state ({} bytes)", ppu_size);

        *ppu = from_bytes_crc32(&ppu_buf, crc.digest()).map_err(|_| {
            error!(SYS, "Failed to deserialize PPU state");
            ()
        })?;

        file.read_exact(&mut file_size_buf).map_err(|_| ())?;
        let cart_size = usize::from_le_bytes(file_size_buf);

        let mut cart_buf = vec![0; cart_size];
        file.read_exact(&mut cart_buf).map_err(|_| ())?;

        info!(SYS, "Received cartridge state ({} bytes)", cart_size);

        *cartridge = from_bytes_crc32(&cart_buf, crc.digest()).map_err(|_| {
            error!(SYS, "Failed to deserialize cartridge state");
            ()
        })?;

        Ok(())
    }

    pub fn save_ram(&mut self, data: &[u8]) -> Result<(), ()> {
        info!(SYS, "Request to save RAM. ({} bytes)", data.len());

        let root_dir = self.file_system.root_dir();
        let mut file = root_dir.create_file(Self::RAM_FILE_NAME).map_err(|_| ())?;

        // The file should be overwritten.
        file.truncate().map_err(|_| ())?;

        file.write_all(data).map_err(|_| ())?;
        file.flush().map_err(|_| ())?;

        Ok(())
    }

    pub fn load_ram(&mut self, buffer: &mut [u8]) -> Result<(), ()> {
        info!(SYS, "Request to load RAM. ({} bytes)", buffer.len());

        let root_dir = self.file_system.root_dir();
        let mut file = root_dir.open_file(Self::RAM_FILE_NAME).map_err(|_| ())?;

        file.read_exact(buffer).map_err(|_| ())?;

        Ok(())
    }
}
