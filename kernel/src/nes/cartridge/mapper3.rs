use heapless::Vec;
use serde::{Deserialize, Serialize};

use crate::{
    critical, log,
    nes::cartridge::{Cartridge, CartridgeOperations},
};

#[derive(Serialize, Deserialize)]
pub struct Mapper3 {
    prg_rom_size: usize,
    chr_size: usize,
    prg_rom: Vec<u8, { Mapper3::PRG_ROM_SIZE }>,
    chr: Vec<u8, { Mapper3::CHR_SIZE }>,
    bank: usize,
}

impl Mapper3 {
    const PRG_ROM_SIZE: usize = 32 * 1024;

    const CHR_SIZE: usize = 32 * 1024;
    const CHR_BANK_UNIT: usize = 8 * 1024;

    pub fn new(prg_rom_size: usize, chr_size: usize) -> Self {
        let (prg_rom, chr_rom_start) = Cartridge::load_prg_rom(prg_rom_size);
        let chr = Cartridge::load_chr(chr_rom_start, chr_size);

        Self {
            prg_rom_size,
            chr_size,
            prg_rom,
            chr,
            bank: 0,
        }
    }
}

impl CartridgeOperations for Mapper3 {
    fn read_cpu_mem(&mut self, addr: u16) -> u8 {
        if addr < 0x8000 {
            critical!(BUS, "Attempt to read unused area: {:#06X}", addr);
        }
        let addr = (addr as usize - 0x8000) % self.prg_rom_size;
        self.prg_rom[addr]
    }

    fn write_cpu_mem(&mut self, addr: u16, data: u8) {
        if addr < 0x8000 {
            critical!(BUS, "Attempt to write to unused area: {:#06X}", addr);
        }
        let addr = (addr as usize - 0x8000) % self.prg_rom_size;
        self.bank = (self.prg_rom[addr] & data) as usize & 0b11;
        log!(CAT, "Switched to bank {}", self.bank);
    }

    fn read_ppu_mem(&mut self, addr: u16) -> u8 {
        let addr = addr as usize + self.bank * Self::CHR_BANK_UNIT;
        if addr >= self.chr_size {
            critical!(BUS, "Attempt to read unused CHR area: {:#06X}", addr);
        }
        self.chr[addr]
    }

    fn write_ppu_mem(&mut self, addr: u16, data: u8) {
        let addr = addr as usize + self.bank * Self::CHR_BANK_UNIT;
        if addr >= self.chr_size {
            critical!(BUS, "Attempt to write unused CHR area: {:#06X}", addr);
        }
        self.chr[addr] = data;
    }
}
