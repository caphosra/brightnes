use heapless::Vec;
use serde::{Deserialize, Serialize};

use crate::{
    critical,
    nes::cartridge::{Cartridge, CartridgeOperations, CHR_ROM_UNIT},
};

#[derive(Serialize, Deserialize)]
pub struct Mapper0 {
    prg_rom_size: usize,
    chr_rom_size: usize,
    prg_rom: Vec<u8, { Mapper0::PRG_ROM_SIZE }>,
    chr_rom: Vec<u8, { Mapper0::CHR_ROM_SIZE }>,
}

impl Mapper0 {
    const PRG_ROM_SIZE: usize = 32 * 1024;
    const CHR_ROM_SIZE: usize = 8 * 1024;

    pub fn new(prg_rom_size: usize, chr_rom_size: usize) -> Self {
        if prg_rom_size != Self::PRG_ROM_SIZE && prg_rom_size != Self::PRG_ROM_SIZE / 2 {
            critical!(CAT, "Unsupported PRG ROM size: {:#x} bytes", prg_rom_size);
        }

        if chr_rom_size != CHR_ROM_UNIT {
            critical!(CAT, "Unsupported CHR ROM size: {:#x} bytes", chr_rom_size);
        }

        let (prg_rom, chr_rom_start) = Cartridge::alloc_prg_rom(prg_rom_size);
        let chr_rom = Cartridge::alloc_chr_rom(chr_rom_start, chr_rom_size);

        Self {
            prg_rom_size,
            chr_rom_size,
            prg_rom,
            chr_rom,
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
        // For 8KB CHR-ROM, mirror it.
        let addr = addr as usize % self.chr_rom_size;
        self.chr_rom[addr]
    }

    fn write_ppu_mem(&mut self, addr: u16, data: u8) {
        // For 8KB CHR-ROM, mirror it.
        let addr = addr as usize % self.chr_rom_size;
        self.chr_rom[addr] = data;
    }
}
