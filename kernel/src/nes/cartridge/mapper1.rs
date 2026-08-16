use alloc::string::ToString;
use heapless::Vec;
use serde::{Deserialize, Serialize};

use crate::{
    critical, error, info, log,
    nes::{
        cartridge::{Cartridge, CartridgeOperations},
        Mirroring,
    },
};

#[derive(Serialize, Deserialize)]
pub struct Mapper1 {
    prg_rom_size: usize,
    prg_ram: Vec<u8, { Mapper1::PRG_RAM_SIZE }>,
    prg_rom: Vec<u8, { Mapper1::PRG_ROM_SIZE }>,
    chr: Vec<u8, { Mapper1::CHR_ROM_SIZE }>,

    prg_rom_banks: [usize; 2],
    chr_banks: [usize; 2],

    shift_register: u8,

    prg_rom_bank_mode: PrgRomBankMode,
    chr_bank_mode: ChrRomBankMode,

    prg_ram_disabled: bool,
    #[serde(skip)]
    wrote_register_this_instruction: bool,

    mirroring: Mirroring,
}

#[repr(u8)]
#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum PrgRomBankMode {
    SwitchAll = 0,
    FixFirst = 2,
    FixLast = 3,
}

#[repr(u8)]
#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum ChrRomBankMode {
    SwitchAll = 0,
    SwitchTwo = 1,
}

impl Mapper1 {
    const PRG_RAM_SIZE: usize = 8 * 1024;

    const PRG_ROM_SIZE: usize = 512 * 1024;
    const PRG_ROM_BANK_UNIT: usize = 16 * 1024;

    const CHR_ROM_SIZE: usize = 128 * 1024;
    const CHR_ROM_BANK_UNIT: usize = 4 * 1024;

    pub fn new(prg_rom_size: usize, chr_size: usize, mirroring: Mirroring) -> Self {
        let (prg_rom, chr_rom_start) = Cartridge::load_prg_rom(prg_rom_size);
        let prg_ram = Cartridge::alloc_prg_ram();
        let chr = Cartridge::load_chr(chr_rom_start, chr_size);

        let bank_size = prg_rom_size / Self::PRG_ROM_BANK_UNIT;
        let prg_rom_banks = [0, bank_size - 1];

        let chr_banks = [0, 1];

        Self {
            prg_rom_size,
            prg_ram,
            prg_rom,
            chr,
            prg_rom_banks,
            chr_banks,
            shift_register: 0x10,
            prg_rom_bank_mode: PrgRomBankMode::FixLast,
            chr_bank_mode: ChrRomBankMode::SwitchTwo,
            prg_ram_disabled: false,
            wrote_register_this_instruction: false,
            mirroring,
        }
    }

    pub fn write_effect(&mut self, addr: u16, data: u8) {
        if addr < 0xA000 {
            // Control
            // CPPMM

            self.mirroring = match data & 0b11 {
                0 => Mirroring::SingleScreenLower,
                1 => Mirroring::SingleScreenUpper,
                2 => Mirroring::Vertical,
                3 => Mirroring::Horizontal,
                _ => {
                    critical!(CAT, "Invalid mirroring mode: {}", data & 0b11);
                }
            };

            info!(CAT, "Mirroring: {}", self.mirroring.to_string());

            let prg_rom_bank_mode = (data >> 2) & 0b11;
            self.prg_rom_bank_mode = match prg_rom_bank_mode {
                0 => PrgRomBankMode::SwitchAll,
                1 => PrgRomBankMode::SwitchAll,
                2 => PrgRomBankMode::FixFirst,
                3 => PrgRomBankMode::FixLast,
                _ => {
                    critical!(CAT, "Invalid PRG ROM bank mode: {}", prg_rom_bank_mode);
                }
            };

            log!(
                CAT,
                "Update PRG ROM bank mode: {}",
                self.prg_rom_bank_mode as u8
            );

            let chr_bank_mode = (data >> 4) & 1;
            self.chr_bank_mode = match chr_bank_mode {
                0 => ChrRomBankMode::SwitchAll,
                1 => ChrRomBankMode::SwitchTwo,
                _ => {
                    critical!(CAT, "Invalid CHR ROM bank mode: {}", chr_bank_mode);
                }
            };

            log!(
                CAT,
                "Update CHR ROM bank mode: {}",
                self.chr_bank_mode as u8
            );
        } else if addr < 0xC000 {
            match self.chr_bank_mode {
                ChrRomBankMode::SwitchAll => {
                    self.chr_banks[0] = (data & 0b11110) as usize;
                    self.chr_banks[1] = self.chr_banks[0] + 1;

                    log!(CAT, "Update 8KB CHR bank at $0000: {}", self.chr_banks[0]);
                }
                ChrRomBankMode::SwitchTwo => {
                    self.chr_banks[0] = (data & 0b11111) as usize;

                    log!(CAT, "Update 4KB CHR bank at $0000: {}", self.chr_banks[0]);
                }
            }
        } else if addr < 0xE000 {
            match self.chr_bank_mode {
                ChrRomBankMode::SwitchAll => {}
                ChrRomBankMode::SwitchTwo => {
                    self.chr_banks[1] = (data & 0b11111) as usize;

                    log!(CAT, "Update 4KB CHR bank at $1000: {}", self.chr_banks[1]);
                }
            }
        } else {
            let bank = (data & 0b1111) as usize;
            match self.prg_rom_bank_mode {
                PrgRomBankMode::SwitchAll => {
                    self.prg_rom_banks[0] = bank & 0b1110;
                    self.prg_rom_banks[1] = (bank & 0b1110) + 1;

                    log!(
                        CAT,
                        "Update 32KB PRG bank at $8000: {}",
                        self.prg_rom_banks[0]
                    );
                }
                PrgRomBankMode::FixFirst => {
                    self.prg_rom_banks[0] = 0;
                    self.prg_rom_banks[1] = bank;

                    log!(
                        CAT,
                        "Update 16KB PRG bank at $C000: {}",
                        self.prg_rom_banks[1]
                    );
                }
                PrgRomBankMode::FixLast => {
                    let bank_size = self.prg_rom_size / Self::PRG_ROM_BANK_UNIT;
                    self.prg_rom_banks[0] = bank;
                    self.prg_rom_banks[1] = bank_size - 1;

                    log!(
                        CAT,
                        "Update 16KB PRG bank at $8000: {}",
                        self.prg_rom_banks[0]
                    );
                }
            }

            self.prg_ram_disabled = (data & 0b10000) != 0;
        }
    }

    pub fn mirroring(&self) -> Mirroring {
        self.mirroring
    }

    pub fn working_ram(&mut self) -> &mut [u8] {
        &mut self.prg_ram
    }

    pub fn begin_cpu_instruction(&mut self) {
        self.wrote_register_this_instruction = false;
    }
}

impl CartridgeOperations for Mapper1 {
    fn read_cpu_mem(&mut self, addr: u16) -> u8 {
        if addr < 0x6000 {
            critical!(BUS, "Attempt to read unused area: {:#06X}", addr);
        } else if addr < 0x8000 {
            // PRG RAM
            if self.prg_ram_disabled {
                error!(CAT, "Attempt to read disabled PRG RAM: {:#06X}", addr);
                0
            } else {
                self.prg_ram[addr as usize - 0x6000]
            }
        } else {
            // Switchable PRG ROM
            let bank_index = (addr as usize - 0x8000) / Self::PRG_ROM_BANK_UNIT;
            let offset = (addr as usize - 0x8000) % Self::PRG_ROM_BANK_UNIT;
            let bank = self.prg_rom_banks[bank_index];
            let addr = bank * Self::PRG_ROM_BANK_UNIT + offset;
            self.prg_rom[addr]
        }
    }

    fn write_cpu_mem(&mut self, addr: u16, data: u8) {
        if addr < 0x6000 {
            critical!(BUS, "Attempt to write to unused area: {:#06X}", addr);
        } else if addr < 0x8000 {
            // PRG RAM
            if self.prg_ram_disabled {
                error!(CAT, "Attempt to write to disabled PRG RAM: {:#06X}", addr);
            } else {
                self.prg_ram[addr as usize - 0x6000] = data;
            }
        } else {
            // Writing to the shift register.

            // MMC1 ignores a write on the CPU cycle immediately following a register write.
            if self.wrote_register_this_instruction {
                return;
            }

            self.wrote_register_this_instruction = true;

            let data_bit = data & 1;
            let reset = (data >> 7) & 1;
            if reset != 0 {
                log!(CAT, "The cartridge reset is requested.");

                // Reset the shift register.
                self.shift_register = 0x10;

                // Reset the PRG ROM bank mapping.
                self.prg_rom_bank_mode = PrgRomBankMode::FixLast;
                let bank_size = self.prg_rom_size / Self::PRG_ROM_BANK_UNIT;
                self.prg_rom_banks[1] = bank_size - 1;
            } else {
                let complete = (self.shift_register & 1) != 0;
                self.shift_register = (self.shift_register >> 1) | (data_bit << 4);
                if complete {
                    // The shift register is full.
                    self.write_effect(addr, self.shift_register);
                    self.shift_register = 0x10;
                }
            }
        }
    }

    fn read_ppu_mem(&mut self, addr: u16) -> u8 {
        let bank_index = addr as usize / Self::CHR_ROM_BANK_UNIT;
        let offset = addr as usize % Self::CHR_ROM_BANK_UNIT;
        let bank = self.chr_banks[bank_index];
        let addr = bank * Self::CHR_ROM_BANK_UNIT + offset;
        self.chr[addr]
    }

    fn write_ppu_mem(&mut self, addr: u16, data: u8) {
        let bank_index = addr as usize / Self::CHR_ROM_BANK_UNIT;
        let offset = addr as usize % Self::CHR_ROM_BANK_UNIT;
        let bank = self.chr_banks[bank_index];
        let addr = bank * Self::CHR_ROM_BANK_UNIT + offset;
        self.chr[addr] = data;
    }
}
