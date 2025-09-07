use core::alloc::Layout;
use core::ptr::slice_from_raw_parts_mut;

use alloc::alloc::alloc;
use spin::{Lazy, RwLock};

use crate::log;
use crate::nes::Mirroring;

#[repr(C)]
pub struct NESHeader {
    magic: [u8; 4],
    prg_rom_size: u8,
    chr_rom_size: u8,
    flag6: u8,
    flag7: u8,
    flag8: u8,
    flag9: u8,
    flag10: u8,
    padding: [u8; 5],
}

impl NESHeader {
    fn new() -> Self {
        // Load the NES header.
        let nes_header = unsafe { (NES_FILE_ADDR as *const NESHeader).as_ref() }.unwrap();
        if nes_header.magic != NES_MAGIC {
            panic!("The NES file is invalid.");
        }

        NESHeader {
            magic: NES_MAGIC,
            prg_rom_size: nes_header.prg_rom_size,
            chr_rom_size: nes_header.chr_rom_size,
            flag6: nes_header.flag6,
            flag7: nes_header.flag7,
            flag8: nes_header.flag8,
            flag9: nes_header.flag9,
            flag10: nes_header.flag10,
            padding: [0; 5],
        }
    }
}

const NES_FILE_ADDR: usize = 0x3_000_000;

const NES_MAGIC: [u8; 4] = *b"NES\x1A";

const PRG_ROM_UNIT: usize = 0x4000;
const CHR_ROM_UNIT: usize = 0x2000;

pub struct Cartridge {
    header: NESHeader,
    prg_rom_size: usize,
    chr_rom_size: usize,
    pub prg_rom: &'static mut [u8],
    pub chr_rom: &'static mut [u8],
    bank: usize,
}

pub static CARTRIDGE: Lazy<RwLock<Cartridge>> = Lazy::new(|| {
    let header = NESHeader::new();
    let prg_rom_size = header.prg_rom_size as usize * PRG_ROM_UNIT;
    let chr_rom_size = header.chr_rom_size as usize * CHR_ROM_UNIT;
    RwLock::new(Cartridge {
        header,
        prg_rom_size,
        chr_rom_size,
        prg_rom: &mut [],
        chr_rom: &mut [],
        bank: 0,
    })
});

impl Cartridge {
    pub fn load(&mut self) {
        log!("[CTG] Mapper: {}", self.mapper());

        // Load the program ROM.
        let prg_rom_start = unsafe { (NES_FILE_ADDR as *mut u8).add(size_of::<NESHeader>()) };
        self.prg_rom =
            unsafe { slice_from_raw_parts_mut(prg_rom_start, self.prg_rom_size).as_mut() }.unwrap();

        log!("[CTG] Loaded PRG ROM ({:#x} bytes)", self.prg_rom_size);

        // Load the character ROM.
        let chr_rom_start = unsafe { prg_rom_start.add(self.prg_rom_size) };
        self.chr_rom =
            unsafe { slice_from_raw_parts_mut(chr_rom_start, self.chr_rom_size).as_mut() }.unwrap();

        log!("[CTG] Loaded CHR ROM ({:#x} bytes)", self.chr_rom_size);

        if self.chr_rom_size == 0 {
            // Prepare CHR RAM if there are no CHR ROM.

            let chr_rom_start = unsafe { alloc(Layout::from_size_align(CHR_ROM_UNIT, 1).unwrap()) };
            self.chr_rom =
                unsafe { slice_from_raw_parts_mut(chr_rom_start, CHR_ROM_UNIT).as_mut() }.unwrap();

            self.chr_rom_size = CHR_ROM_UNIT;
            log!("[CTG] Prepared CHR RAM ({:#x} bytes)", self.chr_rom_size);
        }
    }

    #[inline(always)]
    pub fn mirroring(&self) -> Mirroring {
        (self.header.flag6 & 1).into()
    }

    #[inline(always)]
    pub fn mapper(&self) -> u8 {
        (self.header.flag6 >> 4) | (self.header.flag7 & 0xF0)
    }

    pub fn read_cpu_mem(&mut self, addr: u16) -> u8 {
        let mapper = self.mapper();
        match mapper {
            0 => {
                if addr < 0x8000 {
                    log!("[BUS] Attempt to read unused area: {:#06X}", addr);
                    return 0;
                }
                let addr = (addr as usize - 0x8000) % self.prg_rom_size;
                self.prg_rom[addr]
            }
            2 => {
                // 0x8000-0xBFFF: Switchable bank
                // 0xC000-0xFFFF: Fixed to the last bank
                if addr < 0x8000 {
                    log!("[BUS] Attempt to read unused area: {:#06X}", addr);
                    return 0;
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
            _ => {
                panic!("Unsupported mapper: {}", self.mapper());
            }
        }
    }

    pub fn write_cpu_mem(&mut self, addr: u16, data: u8) {
        let mapper = self.mapper();
        match mapper {
            0 => {
                if addr < 0x8000 {
                    log!("[BUS] Attempt to write to unused area: {:#06X}", addr);
                    return;
                }
                let addr = (addr as usize - 0x8000) % self.prg_rom_size;
                self.prg_rom[addr] = data;
            }
            2 => {
                if addr < 0x8000 {
                    log!("[BUS] Attempt to write to unused area: {:#06X}", addr);
                    return;
                }
                self.bank = (data & 0b1111) as usize;
                log!("[CTG] Switched to bank {}", self.bank);
            }
            _ => {
                panic!("Unsupported mapper: {}", self.mapper());
            }
        }
    }

    pub fn read_ppu_mem(&mut self, addr: u16) -> u8 {
        let mapper = self.mapper();
        match mapper {
            0 => {
                let addr = addr as usize % self.chr_rom_size;
                self.chr_rom[addr]
            }
            2 => {
                let addr = addr as usize % self.chr_rom_size;
                self.chr_rom[addr]
            }
            _ => {
                panic!("Unsupported mapper: {}", self.mapper());
            }
        }
    }

    pub fn write_ppu_mem(&mut self, addr: u16, data: u8) {
        let mapper = self.mapper();
        match mapper {
            0 => {
                let addr = addr as usize % self.chr_rom_size;
                self.chr_rom[addr] = data;
            }
            2 => {
                let addr = addr as usize % self.chr_rom_size;
                self.chr_rom[addr] = data;
            }
            _ => {
                panic!("Unsupported mapper: {}", self.mapper());
            }
        }
    }
}
