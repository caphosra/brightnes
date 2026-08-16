use core::slice::from_raw_parts_mut;

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::{format, vec::Vec};
use crc::{Crc, CRC_32_ISCSI};
use fatfs::{Error, FsOptions, Read, Write};
use postcard::{from_bytes_crc32, to_allocvec_crc32};
use spin::{Lazy, RwLock};

use crate::proc::system::System;
use crate::{
    critical,
    drivers::BlockDeviceDriver,
    error, info,
    nes::{
        apu::APU,
        cartridge::Cartridge,
        cpu::{InterruptType, CPU},
        ppu::PPU,
    },
};

type FATFileSystem<T> = fatfs::FileSystem<T>;

pub static FILE_SYSTEM: Lazy<RwLock<FileSystem>> = Lazy::new(|| {
    let fs = FileSystem::new();
    RwLock::new(fs)
});

pub struct FileSystem {
    file_system: FATFileSystem<BlockDeviceDriver<'static>>,
}

#[derive(Clone)]
pub struct CartridgeInfo {
    pub short_name: String,
    pub long_name: String,
    pub has_savedata: bool,
    pub has_ram: bool,
}

unsafe impl Send for FileSystem {}
unsafe impl Sync for FileSystem {}

impl FileSystem {
    const NES_DIR_NAME: &'static str = "nes";
    const SAVEDATA_EXT: &'static str = "BRS";
    const RAM_EXT: &'static str = "BRR";

    pub fn new() -> Self {
        let driver = BlockDeviceDriver::new();
        let option = FsOptions::new().strict(true);
        let file_system = FATFileSystem::new(driver, option).unwrap();
        FileSystem { file_system }
    }

    pub fn cartridge_infos(&self) -> Vec<CartridgeInfo> {
        let root_dir = self.file_system.root_dir();
        let nes_dir = root_dir.open_dir(Self::NES_DIR_NAME).unwrap_or_else(|_| {
            critical!(
                DSK,
                "Failed to find cartridges. The disk might be corrupted."
            );
        });
        let mut infos = Vec::new();
        for file in nes_dir.iter().flatten() {
            if file.is_file() {
                let short_name = file.short_file_name_as_bytes();
                if short_name.ends_with(b".NES") {
                    // Found a NES file.

                    // Remove the extension.
                    let name_without_ext =
                        str::from_utf8(&short_name[..short_name.len() - b".NES".len()]).unwrap();

                    // Look up a file with ".TXT" to retrieve the long name.
                    // If not found, use the short name as the long name.
                    let long_name = match nes_dir.open_file(&format!("{}.TXT", name_without_ext)) {
                        Ok(mut file) => {
                            let mut long_name = Vec::new();
                            let mut buf = [0u8; 64];
                            let mut length = 0;
                            loop {
                                match file.read(&mut buf) {
                                    Ok(0) => break,
                                    Ok(n) => {
                                        long_name.extend_from_slice(&buf[..n]);
                                        length += n;
                                    }
                                    Err(_) => break,
                                }
                            }
                            match str::from_utf8(&long_name[..length]) {
                                Ok(s) => s.to_string(),
                                Err(_) => name_without_ext.to_string(),
                            }
                        }
                        Err(Error::NotFound) => name_without_ext.to_string(),
                        _ => {
                            critical!(
                                DSK,
                                "Failed to find cartridges. The disk might be corrupted."
                            );
                        }
                    };

                    // Check saved files by trying to open those.
                    let has_savedata = root_dir
                        .open_file(&format!("{}.{}", name_without_ext, Self::SAVEDATA_EXT))
                        .is_ok();
                    let has_ram = root_dir
                        .open_file(&format!("{}.{}", name_without_ext, Self::RAM_EXT))
                        .is_ok();

                    info!(
                        DSK,
                        "A cartridge found: {} ({}), savedata={}, ram={}",
                        long_name,
                        name_without_ext,
                        has_savedata,
                        has_ram
                    );

                    infos.push(CartridgeInfo {
                        short_name: name_without_ext.to_string(),
                        long_name,
                        has_savedata,
                        has_ram,
                    });
                }
            }
        }
        infos
    }

    pub fn load_cartridge(&mut self, info: &CartridgeInfo) {
        const BUF_SIZE: usize = 1024;

        let root_dir = self.file_system.root_dir();
        let nes_dir = root_dir.open_dir(Self::NES_DIR_NAME).unwrap_or_else(|_| {
            critical!(
                DSK,
                "Failed to find cartridges. The disk might be corrupted."
            );
        });
        let file = nes_dir.open_file(&format!("{}.NES", info.short_name));
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
                            critical!(CAT, "Failed to load the cartridge: {}", info.long_name);
                        }
                    }
                }
                info!(CAT, "Loaded the cartridge. ({} bytes)", loaded_bytes);
            }
            Err(Error::NotFound) => {
                error!(
                    CAT,
                    "The specified cartridge was not found: {}", info.long_name
                );
            }
            _ => {
                error!(CAT, "Failed to open the cartridge.");
            }
        }
    }

    pub fn check_root_dir(&mut self) {
        let root_dir = self.file_system.root_dir();
        let entries = root_dir.iter();
        for entry in entries.flatten() {
            info!(SYS, "Found file: {}", entry.file_name());
        }
    }

    pub fn state_file_name(&self, sys: &System) -> Option<String> {
        sys.running_cartridge_name()
            .map(|s| format!("{}.{}", s, Self::SAVEDATA_EXT))
    }

    pub fn ram_file_name(&self, sys: &System) -> Option<String> {
        sys.running_cartridge_name()
            .map(|s| format!("{}.{}", s, Self::RAM_EXT))
    }

    pub fn save_state(
        &mut self,
        sys: &System,
        cpu: &CPU,
        ppu: &PPU,
        apu: &APU,
        cartridge: &Cartridge,
    ) -> Result<(), ()> {
        info!(SYS, "Request to save state");

        let file_name = self.state_file_name(sys);
        if file_name.is_none() {
            error!(SYS, "No game is running. Cannot save state.");
            return Err(());
        }

        let crc = Crc::<u32>::new(&CRC_32_ISCSI);

        let root_dir = self.file_system.root_dir();
        let mut file = root_dir.create_file(&file_name.unwrap()).map_err(|_| ())?;

        // The file should be overwritten.
        file.truncate().map_err(|_| ())?;

        let serialized_cpu = to_allocvec_crc32(cpu, crc.digest());
        let serialized_cpu = serialized_cpu.map_err(|_| {
            error!(SYS, "Failed to serialize CPU state");
        })?;

        file.write_all(&serialized_cpu.len().to_le_bytes())
            .map_err(|_| ())?;
        file.write_all(&serialized_cpu).map_err(|_| ())?;

        info!(SYS, "Saved CPU state ({} bytes)", serialized_cpu.len());

        let serialized_ppu = to_allocvec_crc32(ppu, crc.digest());
        let serialized_ppu = serialized_ppu.map_err(|_| {
            error!(SYS, "Failed to serialize CPU state");
        })?;

        file.write_all(&serialized_ppu.len().to_le_bytes())
            .map_err(|_| ())?;
        file.write_all(&serialized_ppu).map_err(|_| ())?;

        info!(SYS, "Saved PPU state ({} bytes)", serialized_ppu.len());

        let serialized_cartridge = to_allocvec_crc32(cartridge, crc.digest());
        let serialized_cartridge = serialized_cartridge.map_err(|_| {
            error!(SYS, "Failed to serialize cartridge state");
        })?;

        file.write_all(&serialized_cartridge.len().to_le_bytes())
            .map_err(|_| ())?;
        file.write_all(&serialized_cartridge).map_err(|_| ())?;

        info!(
            SYS,
            "Saved cartridge state ({} bytes)",
            serialized_cartridge.len()
        );

        let serialized_apu = to_allocvec_crc32(apu, crc.digest());
        let serialized_apu = serialized_apu.map_err(|_| {
            error!(SYS, "Failed to serialize APU state");
            ()
        })?;

        file.write_all(&serialized_apu.len().to_le_bytes())
            .map_err(|_| ())?;
        file.write_all(&serialized_apu).map_err(|_| ())?;

        info!(SYS, "Saved APU state ({} bytes)", serialized_apu.len());

        file.flush().map_err(|_| ())?;

        Ok(())
    }

    pub fn load_state(
        &mut self,
        sys: &System,
        cpu: &mut CPU,
        ppu: &mut PPU,
        apu: &mut APU,
        cartridge: &mut Cartridge,
    ) -> Result<(), ()> {
        info!(SYS, "Request to load saved state");

        let file_name = self.state_file_name(sys);
        if file_name.is_none() {
            error!(SYS, "No game is running. Cannot save state.");
            return Err(());
        }

        let crc = Crc::<u32>::new(&CRC_32_ISCSI);

        let root_dir = self.file_system.root_dir();
        let mut file = root_dir.open_file(&file_name.unwrap()).map_err(|_| ())?;

        let mut file_size_buf = [0u8; size_of::<usize>()];

        file.read_exact(&mut file_size_buf).map_err(|_| ())?;
        let cpu_size = usize::from_le_bytes(file_size_buf);

        let mut cpu_buf = vec![0; cpu_size];
        file.read_exact(&mut cpu_buf).map_err(|_| ())?;

        info!(SYS, "Received CPU state ({} bytes)", cpu_size);

        *cpu = from_bytes_crc32(&cpu_buf, crc.digest()).map_err(|_| {
            error!(SYS, "Failed to deserialize CPU state");
        })?;

        file.read_exact(&mut file_size_buf).map_err(|_| ())?;
        let ppu_size = usize::from_le_bytes(file_size_buf);

        let mut ppu_buf = vec![0; ppu_size];
        file.read_exact(&mut ppu_buf).map_err(|_| ())?;

        info!(SYS, "Received PPU state ({} bytes)", ppu_size);

        *ppu = from_bytes_crc32(&ppu_buf, crc.digest()).map_err(|_| {
            error!(SYS, "Failed to deserialize PPU state");
        })?;

        file.read_exact(&mut file_size_buf).map_err(|_| ())?;
        let cart_size = usize::from_le_bytes(file_size_buf);

        let mut cart_buf = vec![0; cart_size];
        file.read_exact(&mut cart_buf).map_err(|_| ())?;

        info!(SYS, "Received cartridge state ({} bytes)", cart_size);

        *cartridge = from_bytes_crc32(&cart_buf, crc.digest()).map_err(|_| {
            error!(SYS, "Failed to deserialize cartridge state");
        })?;

        match file.read_exact(&mut file_size_buf) {
            Ok(()) => {}
            Err(Error::UnexpectedEof) => {
                cpu.cancel_interrupt(InterruptType::IRQ);
                info!(SYS, "Loaded legacy state without APU data");
                return Ok(());
            }
            Err(_) => return Err(()),
        }

        let apu_size = usize::from_le_bytes(file_size_buf);

        let mut apu_buf = vec![0; apu_size];
        file.read_exact(&mut apu_buf).map_err(|_| ())?;

        info!(SYS, "Received APU state ({} bytes)", apu_size);

        *apu = from_bytes_crc32(&apu_buf, crc.digest()).map_err(|_| {
            error!(SYS, "Failed to deserialize APU state");
            ()
        })?;

        Ok(())
    }

    pub fn save_ram(&mut self, sys: &System, data: &[u8]) -> Result<(), ()> {
        info!(SYS, "Request to save RAM. ({} bytes)", data.len());

        let file_name = self.ram_file_name(sys);
        if file_name.is_none() {
            error!(SYS, "No game is running. Cannot save state.");
            return Err(());
        }

        let root_dir = self.file_system.root_dir();
        let mut file = root_dir.create_file(&file_name.unwrap()).map_err(|_| ())?;

        // The file should be overwritten.
        file.truncate().map_err(|_| ())?;

        file.write_all(data).map_err(|_| ())?;
        file.flush().map_err(|_| ())?;

        Ok(())
    }

    pub fn load_ram(&mut self, sys: &System, buffer: &mut [u8]) -> Result<(), ()> {
        info!(SYS, "Request to load RAM. ({} bytes)", buffer.len());

        let file_name = self.ram_file_name(sys);
        if file_name.is_none() {
            error!(SYS, "No game is running. Cannot save state.");
            return Err(());
        }

        let root_dir = self.file_system.root_dir();
        let mut file = root_dir.open_file(&file_name.unwrap()).map_err(|_| ())?;

        file.read_exact(buffer).map_err(|_| ())?;

        Ok(())
    }
}
