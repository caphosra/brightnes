use core::ptr::slice_from_raw_parts_mut;

use heapless::Vec;
use spin::{Lazy, RwLock};

use crate::nes::cartridge::mapper0::Mapper0;
use crate::nes::cartridge::mapper2::Mapper2;
use crate::nes::cartridge::mapper3::Mapper3;
use crate::nes::cartridge::mapper4::Mapper4;
use crate::nes::cpu::{InterruptType, NESCPU};
use crate::nes::ppu::NESPPU;
use crate::nes::Mirroring;
use crate::{critical, info};

#[repr(C)]
pub struct NESHeader {
    magic: [u8; 4],
    prg_rom_size: u8,
    chr_size: u8,
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
            critical!(CAT, "The NES file is invalid.");
        }

        NESHeader {
            magic: NES_MAGIC,
            prg_rom_size: nes_header.prg_rom_size,
            chr_size: nes_header.chr_size,
            flag6: nes_header.flag6,
            flag7: nes_header.flag7,
            flag8: nes_header.flag8,
            flag9: nes_header.flag9,
            flag10: nes_header.flag10,
            padding: [0; 5],
        }
    }

    #[inline(always)]
    pub fn mapper(&self) -> u8 {
        (self.flag6 >> 4) | (self.flag7 & 0xF0)
    }
}

const NES_FILE_ADDR: usize = 0x3_000_000;

const NES_MAGIC: [u8; 4] = *b"NES\x1A";

const PRG_ROM_UNIT: usize = 0x4000;
const CHR_UNIT: usize = 0x2000;

pub struct Cartridge {
    header: NESHeader,
    kind: CartridgeKind,
}

pub enum CartridgeKind {
    Mapper0(Mapper0),
    Mapper2(Mapper2),
    Mapper3(Mapper3),
    Mapper4(Mapper4),
}

pub static CARTRIDGE: Lazy<RwLock<Cartridge>> = Lazy::new(|| {
    let header = NESHeader::new();
    let prg_rom_size = header.prg_rom_size as usize * PRG_ROM_UNIT;
    let chr_size = header.chr_size as usize * CHR_UNIT;
    RwLock::new(Cartridge::new(header, prg_rom_size, chr_size))
});

impl Cartridge {
    pub fn new(header: NESHeader, prg_rom_size: usize, chr_size: usize) -> Self {
        let mapper = header.mapper();
        info!(CAT, "Mapper: {}", mapper);

        let kind = match mapper {
            0 => CartridgeKind::Mapper0(Mapper0::new(prg_rom_size, chr_size)),
            2 => CartridgeKind::Mapper2(Mapper2::new(prg_rom_size, chr_size)),
            3 => CartridgeKind::Mapper3(Mapper3::new(prg_rom_size, chr_size)),
            4 => CartridgeKind::Mapper4(Mapper4::new(prg_rom_size, chr_size)),
            _ => {
                critical!(CAT, "Unsupported mapper: {}", mapper);
            }
        };

        info!(SYS, "Loaded the cartridge.");

        Cartridge { header, kind }
    }

    pub fn load_prg_rom<const N: usize>(prg_rom_size: usize) -> (Vec<u8, N>, *mut u8) {
        if prg_rom_size > N {
            critical!(
                CAT,
                "Expected {:#x} bytes on PRG ROM size but found {:#x} bytes.",
                N,
                prg_rom_size
            );
        }

        // Load PRG ROM.
        let prg_rom_start = unsafe { (NES_FILE_ADDR as *mut u8).add(size_of::<NESHeader>()) };
        let prg_rom =
            unsafe { slice_from_raw_parts_mut(prg_rom_start, prg_rom_size).as_mut() }.unwrap();

        info!(CAT, "Allocate PRG ROM ({:#x} bytes)", N);
        info!(CAT, "Loaded PRG ROM ({:#x} bytes)", prg_rom_size);

        // Calculate the start address of CHR ROM.
        let chr_rom_start = unsafe { prg_rom_start.add(prg_rom_size) };

        let mut prg_rom = Vec::from_slice(prg_rom).unwrap();
        prg_rom.resize(N, 0).unwrap();
        (prg_rom, chr_rom_start)
    }

    pub fn load_chr<const N: usize>(chr_rom_start: *mut u8, chr_size: usize) -> Vec<u8, N> {
        if chr_size > N {
            critical!(
                CAT,
                "Expected {:#x} bytes on CHR size but found {:#x} bytes.",
                N,
                chr_size
            );
        }

        // Load CHR.
        let chr = unsafe { slice_from_raw_parts_mut(chr_rom_start, chr_size).as_mut() }.unwrap();

        info!(CAT, "Allocate CHR ({:#x} bytes)", N);
        info!(CAT, "Loaded CHR ({:#x} bytes)", chr_size);

        let mut chr = Vec::from_slice(chr).unwrap();
        chr.resize(N, 0).unwrap();
        chr
    }

    pub fn alloc_prg_ram<const N: usize>() -> Vec<u8, N> {
        info!(CAT, "Allocate PRG RAM ({:#x} bytes)", N);
        Vec::from_array([0; N])
    }

    #[inline(always)]
    pub fn mirroring(&self) -> Mirroring {
        (self.header.flag6 & 1).into()
    }

    pub fn read_cpu_mem(&mut self, addr: u16) -> u8 {
        match &mut self.kind {
            CartridgeKind::Mapper0(mapper) => mapper.read_cpu_mem(addr),
            CartridgeKind::Mapper2(mapper) => mapper.read_cpu_mem(addr),
            CartridgeKind::Mapper3(mapper) => mapper.read_cpu_mem(addr),
            CartridgeKind::Mapper4(mapper) => mapper.read_cpu_mem(addr),
        }
    }

    pub fn write_cpu_mem(&mut self, addr: u16, data: u8) {
        match &mut self.kind {
            CartridgeKind::Mapper0(mapper) => mapper.write_cpu_mem(addr, data),
            CartridgeKind::Mapper2(mapper) => mapper.write_cpu_mem(addr, data),
            CartridgeKind::Mapper3(mapper) => mapper.write_cpu_mem(addr, data),
            CartridgeKind::Mapper4(mapper) => mapper.write_cpu_mem(addr, data),
        };
    }

    pub fn read_ppu_mem(&mut self, addr: u16) -> u8 {
        match &mut self.kind {
            CartridgeKind::Mapper0(mapper) => mapper.read_ppu_mem(addr),
            CartridgeKind::Mapper2(mapper) => mapper.read_ppu_mem(addr),
            CartridgeKind::Mapper3(mapper) => mapper.read_ppu_mem(addr),
            CartridgeKind::Mapper4(mapper) => mapper.read_ppu_mem(addr),
        }
    }

    pub fn write_ppu_mem(&mut self, addr: u16, data: u8) {
        match &mut self.kind {
            CartridgeKind::Mapper0(mapper) => mapper.write_ppu_mem(addr, data),
            CartridgeKind::Mapper2(mapper) => mapper.write_ppu_mem(addr, data),
            CartridgeKind::Mapper3(mapper) => mapper.write_ppu_mem(addr, data),
            CartridgeKind::Mapper4(mapper) => mapper.write_ppu_mem(addr, data),
        };
    }

    pub fn irq_clock(&mut self, cpu: &mut NESCPU, ppu: &mut NESPPU) {
        if let CartridgeKind::Mapper4(mapper) = &mut self.kind {
            if mapper.irq_clock() {
                cpu.interrupt(InterruptType::IRQ, ppu, self);
            }
        }
    }
}

trait CartridgeOperations {
    fn read_cpu_mem(&mut self, addr: u16) -> u8;
    fn write_cpu_mem(&mut self, addr: u16, data: u8);
    fn read_ppu_mem(&mut self, addr: u16) -> u8;
    fn write_ppu_mem(&mut self, addr: u16, data: u8);
}

mod mapper0;
mod mapper2;
mod mapper3;
mod mapper4;
