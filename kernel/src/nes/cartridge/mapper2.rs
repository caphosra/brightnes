use heapless::Vec;

use crate::{
    critical, log,
    nes::cartridge::{Cartridge, CartridgeOperations},
};

pub struct Mapper2 {
    prg_rom: Vec<u8, { Mapper2::PRG_ROM_SIZE }>,
    chr: Vec<u8, { Mapper2::CHR_SIZE }>,
    bank: usize,
}

impl Mapper2 {
    const PRG_ROM_SIZE: usize = 256 * 1024;
    const PRG_ROM_UNIT: usize = 16 * 1024;

    const CHR_SIZE: usize = 8 * 1024;

    pub fn new(prg_rom_size: usize, chr_size: usize) -> Self {
        let (prg_rom, chr_rom_start) = Cartridge::load_prg_rom(prg_rom_size);
        let chr = Cartridge::load_chr(chr_rom_start, chr_size);

        Self {
            prg_rom,
            chr,
            bank: 0,
        }
    }
}

impl CartridgeOperations for Mapper2 {
    fn read_cpu_mem(&mut self, addr: u16) -> u8 {
        // 0x8000-0xBFFF: Switchable bank
        // 0xC000-0xFFFF: Fixed to the last bank
        if addr < 0x8000 {
            critical!(BUS, "Attempt to read unused area: {:#06X}", addr);
        }
        if addr < 0xC000 {
            let addr = (addr as usize - 0x8000) + self.bank * Self::PRG_ROM_UNIT;
            self.prg_rom[addr]
        } else {
            let bank_len = Self::PRG_ROM_SIZE / Self::PRG_ROM_UNIT;
            let addr = (addr as usize - 0xC000) + (bank_len - 1) * Self::PRG_ROM_UNIT;
            self.prg_rom[addr]
        }
    }

    fn write_cpu_mem(&mut self, addr: u16, data: u8) {
        if addr < 0x8000 {
            critical!(BUS, "Attempt to write to unused area: {:#06X}", addr);
        }
        self.bank = (data & 0b1111) as usize;
        log!(CAT, "Switched to bank {}", self.bank);
    }

    fn read_ppu_mem(&mut self, addr: u16) -> u8 {
        self.chr[addr as usize]
    }

    fn write_ppu_mem(&mut self, addr: u16, data: u8) {
        self.chr[addr as usize] = data;
    }
}
