use core::ptr::slice_from_raw_parts_mut;

use crate::{
    critical, info, log,
    mem::MemoryAllocator,
    nes::cartridge::{CartridgeOperations, CHR_ROM_UNIT, PRG_ROM_UNIT},
};

pub struct Mapper2 {
    prg_rom_size: usize,
    chr_rom_size: usize,
    prg_rom: &'static [u8],
    chr_rom: &'static mut [u8],
    bank: usize,
}

impl Mapper2 {
    pub fn new(
        prg_rom_size: usize,
        chr_rom_size: usize,
        prg_rom: &'static [u8],
        chr_rom: &'static mut [u8],
    ) -> Self {
        if chr_rom_size == 0 {
            // Prepare CHR RAM if there are no CHR ROM.

            let chr_rom_start = MemoryAllocator::alloc(CHR_ROM_UNIT, 1);
            let chr_rom =
                unsafe { slice_from_raw_parts_mut(chr_rom_start, CHR_ROM_UNIT).as_mut() }.unwrap();

            let chr_rom_size = CHR_ROM_UNIT;
            info!(CAT, "Prepared CHR RAM ({:#x} bytes)", chr_rom_size);

            Self {
                prg_rom_size,
                chr_rom_size,
                prg_rom,
                chr_rom,
                bank: 0,
            }
        } else {
            Self {
                prg_rom_size,
                chr_rom_size,
                prg_rom,
                chr_rom,
                bank: 0,
            }
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
            let addr = (addr as usize - 0x8000) + self.bank * PRG_ROM_UNIT;
            self.prg_rom[addr]
        } else {
            let bank_len = self.prg_rom_size / PRG_ROM_UNIT;
            let addr = (addr as usize - 0xC000) + (bank_len - 1) * PRG_ROM_UNIT;
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
        let addr = addr as usize % self.chr_rom_size;
        self.chr_rom[addr]
    }

    fn write_ppu_mem(&mut self, addr: u16, data: u8) {
        let addr = addr as usize % self.chr_rom_size;
        self.chr_rom[addr] = data;
    }
}
