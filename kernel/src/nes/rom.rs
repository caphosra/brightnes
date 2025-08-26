use core::ptr::slice_from_raw_parts;

use spin::{Lazy, Once};

use crate::log;

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

const NES_FILE_ADDR: usize = 0x3_000_000;

const NES_MAGIC: [u8; 4] = *b"NES\x1A";

const PRG_ROM_UNIT: usize = 0x4000;
const CHR_ROM_UNIT: usize = 0x2000;

pub struct NESROM {
    pub prg_rom: &'static [u8],
    pub chr_rom: &'static [u8],
}

pub static NES_ROM: Lazy<Once<NESROM>> = Lazy::new(|| Once::new());

impl NESROM {
    pub fn load() {
        NES_ROM.call_once(|| {
            // Load the NES header.
            let nes_header = unsafe { (NES_FILE_ADDR as *const NESHeader).as_ref() }.unwrap();
            if nes_header.magic != NES_MAGIC {
                panic!("The NES file is invalid.");
            }

            // Load the program ROM.
            let prg_rom_start = unsafe { (NES_FILE_ADDR as *const u8).add(size_of::<NESHeader>()) };
            let prg_rom_size = nes_header.prg_rom_size as usize * PRG_ROM_UNIT;
            let prg_rom =
                unsafe { slice_from_raw_parts(prg_rom_start, prg_rom_size).as_ref() }.unwrap();

            log!("[ROM] Loaded PRG ROM ({:#x} bytes)", prg_rom_size);

            // Load the character ROM.
            let chr_rom_start = unsafe { prg_rom_start.add(prg_rom_size) };
            let chr_rom_size = nes_header.chr_rom_size as usize * CHR_ROM_UNIT;
            let chr_rom =
                unsafe { slice_from_raw_parts(chr_rom_start, chr_rom_size).as_ref() }.unwrap();

            log!("[ROM] Loaded CHR ROM ({:#x} bytes)", chr_rom_size);

            NESROM { prg_rom, chr_rom }
        });
    }
}
