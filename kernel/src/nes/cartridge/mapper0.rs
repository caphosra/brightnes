use crate::{critical, nes::cartridge::CartridgeOperations};

pub struct Mapper0 {
    prg_rom_size: usize,
    chr_rom_size: usize,
    prg_rom: &'static [u8],
    chr_rom: &'static mut [u8],
}

impl Mapper0 {
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
