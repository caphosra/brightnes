use heapless::Vec;
use serde::{Deserialize, Serialize};

use crate::{
    critical,
    nes::cartridge::{Cartridge, CartridgeOperations},
};

#[derive(Serialize, Deserialize)]
pub struct Mapper0 {
    prg_rom_size: usize,
    prg_rom: Vec<u8, { Mapper0::PRG_ROM_SIZE }>,
    chr: Vec<u8, { Mapper0::CHR_SIZE }>,
}

impl Mapper0 {
    const PRG_ROM_SIZE: usize = 32 * 1024;
    const CHR_SIZE: usize = 8 * 1024;

    pub fn new(prg_rom_size: usize, chr_size: usize) -> Self {
        let (prg_rom, chr_rom_start) = Cartridge::load_prg_rom(prg_rom_size);
        let chr = Cartridge::load_chr(chr_rom_start, chr_size);

        Self {
            prg_rom_size,
            prg_rom,
            chr,
        }
    }
}

impl CartridgeOperations for Mapper0 {
    fn read_cpu_mem(&mut self, addr: u16) -> u8 {
        if addr < 0x8000 {
            critical!(BUS, "Attempt to read unused area: {:#06X}", addr);
        }

        // For 16KB PRG-ROM, mirror it.
        let addr = (addr as usize - 0x8000) % self.prg_rom_size;
        self.prg_rom[addr]
    }

    fn write_cpu_mem(&mut self, addr: u16, _data: u8) {
        critical!(BUS, "Attempt to write to read-only area: {:#06X}", addr);
    }

    fn read_ppu_mem(&mut self, addr: u16) -> u8 {
        self.chr[addr as usize]
    }

    fn write_ppu_mem(&mut self, addr: u16, data: u8) {
        self.chr[addr as usize] = data;
    }
}
