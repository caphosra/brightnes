//
// Reference: https://www.nesdev.org/wiki/MMC3
//

use core::ptr::slice_from_raw_parts_mut;

use crate::{
    critical, info, log,
    mem::MemoryAllocator,
    nes::{
        cartridge::CartridgeOperations,
        cpu::{InterruptType, NESCPU},
        Mirroring,
    },
    warn,
};

pub struct Mapper4 {
    prg_rom_size: usize,
    prg_ram: &'static mut [u8],
    prg_rom: &'static [u8],
    chr_rom: &'static mut [u8],

    prg_rom_banks: [usize; 3],
    chr_rom_banks: [usize; 8],
    /// If true, map two 2KB banks at PPU $0000.
    two_banks_first: bool,

    bank_select: u8,

    prg_ram_enabled: bool,
    prg_ram_write_protected: bool,

    irq_counter: u8,
    irq_latch_value: u8,
    irq_reload: bool,
    irq_enabled: bool,
}

impl Mapper4 {
    const PRG_RAM_SIZE: usize = 0x2000;
    const CHR_ROM_SIZE: usize = 0x40000;
    const PRG_ROM_BANK_UNIT: usize = 0x2000;
    const CHR_ROM_BANK_UNIT: usize = 0x400;

    pub fn new(
        prg_rom_size: usize,
        _chr_rom_size: usize,
        prg_rom: &'static [u8],
        _chr_rom: &'static mut [u8],
    ) -> Self {
        // Prepare PRG RAM.
        let prg_ram = MemoryAllocator::alloc(Self::PRG_RAM_SIZE, 1);
        let prg_ram =
            unsafe { slice_from_raw_parts_mut(prg_ram, Self::PRG_RAM_SIZE).as_mut() }.unwrap();

        let bank_size = prg_rom_size / Self::PRG_ROM_BANK_UNIT;
        let prg_rom_banks = [0, 1, bank_size - 2];

        // Prepare CHR RAM.

        let chr_rom_start = MemoryAllocator::alloc(Self::CHR_ROM_SIZE, 1);
        let chr_rom =
            unsafe { slice_from_raw_parts_mut(chr_rom_start, Self::CHR_ROM_SIZE).as_mut() }
                .unwrap();

        let chr_rom_size = Self::CHR_ROM_SIZE;
        info!(CAT, "Prepared CHR RAM ({:#x} bytes)", chr_rom_size);

        Self {
            prg_rom_size,
            prg_ram,
            prg_rom,
            chr_rom,
            prg_rom_banks,
            chr_rom_banks: [0; 8],
            two_banks_first: true,
            bank_select: 0,
            prg_ram_enabled: true,
            prg_ram_write_protected: true,

            irq_counter: 0,
            irq_latch_value: 0,
            irq_reload: false,
            irq_enabled: false,
        }
    }

    fn prg_rom_bank_size(&self) -> usize {
        self.prg_rom_size / Self::PRG_ROM_BANK_UNIT
    }

    fn update_bank(&mut self, bank: u8) {
        let selected_bank = self.bank_select & 0b111;
        let prg_rom_bank_mode = self.bank_select & (1 << 6) != 0;
        self.two_banks_first = self.bank_select & (1 << 7) == 0;
        match selected_bank {
            0 => {
                if bank % 2 != 0 {
                    critical!(BUS, "Expected even CHR bank number but found: {}", bank);
                }

                if self.two_banks_first {
                    // 2KB CHR bank at PPU $0000-$07FF
                    self.chr_rom_banks[0] = bank as usize;
                    self.chr_rom_banks[1] = bank as usize + 1;

                    log!(CAT, "Update 2KB CHR bank at $0000: {}", bank);
                } else {
                    // 2KB CHR bank at PPU $1000-$17FF
                    self.chr_rom_banks[4] = bank as usize;
                    self.chr_rom_banks[5] = bank as usize + 1;

                    log!(CAT, "Update 2KB CHR bank at $1000: {}", bank);
                }
            }
            1 => {
                if bank % 2 != 0 {
                    critical!(BUS, "Expected even CHR bank number but found: {}", bank);
                }

                if self.two_banks_first {
                    // 2KB CHR bank at PPU $0800-$0FFF
                    self.chr_rom_banks[2] = bank as usize;
                    self.chr_rom_banks[3] = bank as usize + 1;

                    log!(CAT, "Update 2KB CHR bank at $0800: {}", bank);
                } else {
                    // 2KB CHR bank at PPU $1800-$1FFF
                    self.chr_rom_banks[6] = bank as usize;
                    self.chr_rom_banks[7] = bank as usize + 1;

                    log!(CAT, "Update 2KB CHR bank at $1800: {}", bank);
                }
            }
            2 => {
                if self.two_banks_first {
                    // 1KB CHR bank at PPU $1000-$13FF
                    self.chr_rom_banks[4] = bank as usize;

                    log!(CAT, "Update 1KB CHR bank at $1000: {}", bank);
                } else {
                    // 1KB CHR bank at PPU $0000-$03FF
                    self.chr_rom_banks[0] = bank as usize;

                    log!(CAT, "Update 1KB CHR bank at $0000: {}", bank);
                }
            }
            3 => {
                if self.two_banks_first {
                    // 1KB CHR bank at PPU $1400-$17FF
                    self.chr_rom_banks[5] = bank as usize;

                    log!(CAT, "Update 1KB CHR bank at $1400: {}", bank);
                } else {
                    // 1KB CHR bank at PPU $0400-$07FF
                    self.chr_rom_banks[1] = bank as usize;

                    log!(CAT, "Update 1KB CHR bank at $0400: {}", bank);
                }
            }
            4 => {
                if self.two_banks_first {
                    // 1KB CHR bank at PPU $1800-$1BFF
                    self.chr_rom_banks[6] = bank as usize;

                    log!(CAT, "Update 1KB CHR bank at $1800: {}", bank);
                } else {
                    // 1KB CHR bank at PPU $0800-$0BFF
                    self.chr_rom_banks[2] = bank as usize;

                    log!(CAT, "Update 1KB CHR bank at $0800: {}", bank);
                }
            }
            5 => {
                if self.two_banks_first {
                    // 1KB CHR bank at PPU $1C00-$1FFF
                    self.chr_rom_banks[7] = bank as usize;

                    log!(CAT, "Update 1KB CHR bank at $1C00: {}", bank);
                } else {
                    // 1KB CHR bank at PPU $0C00-$0FFF
                    self.chr_rom_banks[3] = bank as usize;

                    log!(CAT, "Update 1KB CHR bank at $0C00: {}", bank);
                }
            }
            6 => {
                if prg_rom_bank_mode {
                    // $8000-$9FFF: fixed to the second last bank
                    // $C000-$DFFF: switch 8KB PRG bank
                    self.prg_rom_banks[0] = (self.prg_rom_bank_size() - 2) as usize;
                    self.prg_rom_banks[2] = bank as usize;

                    log!(CAT, "Update PRG ROM bank at $C000: {}", bank);
                } else {
                    // $8000-$9FFF: switch 8KB PRG bank
                    // $C000-$DFFF: fixed to the second last bank
                    self.prg_rom_banks[0] = bank as usize;
                    self.prg_rom_banks[2] = (self.prg_rom_bank_size() - 2) as usize;

                    log!(CAT, "Update PRG ROM bank at $8000: {}", bank);
                }
            }
            7 => {
                // 8KB PRG bank at CPU $A000-$BFFF
                self.prg_rom_banks[1] = bank as usize;

                log!(CAT, "Update PRG ROM bank at $A000: {}", bank);
            }
            _ => {
                critical!(BUS, "Invalid bank select: {}", selected_bank);
            }
        }
    }

    pub fn irq_clock(&mut self) {
        if self.irq_enabled && self.irq_counter == 0 {
            // Trigger IRQ
            log!(CAT, "Trigger IRQ from the cartridge.");
            NESCPU::interrupt(InterruptType::IRQ);
        }

        if self.irq_counter == 0 || self.irq_reload {
            self.irq_counter = self.irq_latch_value;
            self.irq_reload = false;
        } else {
            self.irq_counter -= 1;
        }
    }
}

impl CartridgeOperations for Mapper4 {
    fn read_cpu_mem(&mut self, addr: u16) -> u8 {
        if addr < 0x6000 {
            critical!(BUS, "Attempt to read unused area: {:#06X}", addr);
        } else if addr < 0x8000 {
            // PRG RAM: 0x6000-0x7FFF
            if self.prg_ram_enabled {
                let addr = addr as usize - 0x6000;
                self.prg_ram[addr]
            } else {
                critical!(BUS, "Attempt to read disabled PRG RAM: {:#06X}", addr);
            }
        } else if addr < 0xE000 {
            // RRG ROM switchable:
            // $8000-$9FFF
            // $A000-$BFFF
            // $C000-$DFFF
            let section_num = (addr as usize - 0x8000) / Self::PRG_ROM_BANK_UNIT;
            let section_offset = (addr as usize - 0x8000) % Self::PRG_ROM_BANK_UNIT;
            let bank = self.prg_rom_banks[section_num];
            let addr = bank * Self::PRG_ROM_BANK_UNIT + section_offset;
            self.prg_rom[addr]
        } else {
            // PRG ROM: $E000-$FFFF
            let offset = addr as usize - 0xE000;
            let bank = self.prg_rom_bank_size() - 1;
            let addr = bank * Self::PRG_ROM_BANK_UNIT + offset;
            self.prg_rom[addr]
        }
    }

    fn write_cpu_mem(&mut self, addr: u16, data: u8) {
        if addr < 0x6000 {
            critical!(BUS, "Attempt to write unused area: {:#06X}", addr);
        } else if addr < 0x8000 {
            // PRG RAM: 0x6000-0x7FFF
            if self.prg_ram_enabled {
                let addr = addr as usize - 0x6000;
                self.prg_ram[addr] = data;
            } else {
                critical!(BUS, "Attempt to write disabled PRG RAM: {:#06X}", addr);
            }
        } else if addr < 0xA000 {
            if addr % 2 == 0 {
                // Bank select
                self.bank_select = data;
            } else {
                // Bank data
                self.update_bank(data);
            }
        } else if addr < 0xC000 {
            if addr % 2 == 0 {
                // Mirroring
                let mirroring: Mirroring = (data & 1).into();
                match mirroring {
                    Mirroring::Horizontal => {
                        warn!(BUS, "Change mirroring to horizontal is not supported.");
                    }
                    Mirroring::Vertical => {
                        warn!(BUS, "Change mirroring to vertical is not supported.")
                    }
                }
            } else {
                // PRG RAM protect
                self.prg_ram_enabled = data & (1 << 7) != 0;
                self.prg_ram_write_protected = data & (1 << 6) == 0;
            }
        } else if addr < 0xE000 {
            if addr % 2 == 0 {
                // IRQ latch
                self.irq_latch_value = data;
            } else {
                // IRQ reload
                self.irq_reload = true;
            }
        } else {
            if addr % 2 == 0 {
                // IRQ disable
                self.irq_enabled = false;
            } else {
                // IRQ enable
                self.irq_enabled = true;
            }
        }
    }

    fn read_ppu_mem(&mut self, addr: u16) -> u8 {
        let section_num = addr as usize / Self::CHR_ROM_BANK_UNIT;
        let section_offset = addr as usize % Self::CHR_ROM_BANK_UNIT;
        let bank = self.chr_rom_banks[section_num];
        let addr = bank * Self::CHR_ROM_BANK_UNIT + section_offset;
        self.chr_rom[addr]
    }

    fn write_ppu_mem(&mut self, addr: u16, data: u8) {
        let section_num = addr as usize / Self::CHR_ROM_BANK_UNIT;
        let section_offset = addr as usize % Self::CHR_ROM_BANK_UNIT;
        let bank = self.chr_rom_banks[section_num];
        let addr = bank * Self::CHR_ROM_BANK_UNIT + section_offset;
        self.chr_rom[addr] = data;
    }
}
