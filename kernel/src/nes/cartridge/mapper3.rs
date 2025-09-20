use crate::{
    critical, log,
    nes::cartridge::{CartridgeOperations, CHR_UNIT},
};

pub struct Mapper3 {
    prg_rom_size: usize,
    chr_rom_size: usize,
    prg_rom: &'static [u8],
    chr_rom: &'static mut [u8],
    bank: usize,
}

impl Mapper3 {
    pub fn new(
        prg_rom_size: usize,
        chr_rom_size: usize,
        prg_rom: &'static [u8],
        chr_rom: &'static mut [u8],
    ) -> Self {
        Self {
            prg_rom_size,
            chr_rom_size,
            prg_rom,
            chr_rom,
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
        let addr = addr as usize + self.bank * CHR_UNIT;
        if addr >= self.chr_rom_size {
            critical!(BUS, "Attempt to read unused CHR area: {:#06X}", addr);
        }
        self.chr_rom[addr]
    }

    fn write_ppu_mem(&mut self, addr: u16, data: u8) {
        let addr = addr as usize + self.bank * CHR_UNIT;
        if addr >= self.chr_rom_size {
            critical!(BUS, "Attempt to write unused CHR area: {:#06X}", addr);
        }
        self.chr_rom[addr] = data;
    }
}
