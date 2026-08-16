use core::ptr::slice_from_raw_parts_mut;

use alloc::boxed::Box;
use alloc::string::ToString;
use heapless::Vec;
use serde::{Deserialize, Serialize};
use spin::{Lazy, Once};

use crate::mem::MemoryAllocator;
use crate::nes::cartridge::mapper0::Mapper0;
use crate::nes::cartridge::mapper2::Mapper2;
use crate::nes::cartridge::mapper3::Mapper3;
use crate::nes::cartridge::mapper4::Mapper4;
use crate::nes::cpu::{InterruptType, CPU};
use crate::nes::Mirroring;
use crate::{critical, info};

#[repr(C)]
#[derive(Serialize, Deserialize)]
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
        let nes_header =
            unsafe { (Cartridge::NES_FILE_ADDR as *const NESHeader).as_ref() }.unwrap();
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

    #[inline(always)]
    pub fn mirroring(&self) -> Mirroring {
        (self.flag6 & 1).into()
    }
}

const NES_MAGIC: [u8; 4] = *b"NES\x1A";

const PRG_ROM_UNIT: usize = 0x4000;
const CHR_UNIT: usize = 0x2000;

#[derive(Serialize, Deserialize)]
pub struct Cartridge {
    header: NESHeader,
    kind: CartridgeKind,
}

#[derive(Serialize, Deserialize)]
pub enum CartridgeKind {
    Mapper0(Box<Mapper0>),
    Mapper2(Box<Mapper2>),
    Mapper3(Box<Mapper3>),
    Mapper4(Box<Mapper4>),
}

static CARTRIDGE_PTR: Lazy<Once<usize>> = Lazy::new(Once::new);

impl Cartridge {
    pub const NES_FILE_ADDR: usize = 0x3_000_000;

    pub fn get() -> &'static mut Self {
        let ptr = *CARTRIDGE_PTR.call_once(|| {
            // Allocate memory for the cartridge.
            let cartridge_raw_ptr = MemoryAllocator::alloc_zeroed::<Cartridge>();
            cartridge_raw_ptr as usize
        }) as *mut Cartridge;
        unsafe { ptr.as_mut() }.unwrap()
    }

    pub fn init(&mut self) {
        let header = NESHeader::new();
        let prg_rom_size = header.prg_rom_size as usize * PRG_ROM_UNIT;
        let chr_size = header.chr_size as usize * CHR_UNIT;

        let mapper = header.mapper();
        info!(CAT, "Mapper: {}", mapper);

        let mirroring = header.mirroring();
        info!(CAT, "Mirroring: {}", mirroring.to_string());

        let kind = match mapper {
            0 => CartridgeKind::Mapper0(Box::new(Mapper0::new(prg_rom_size, chr_size))),
            2 => CartridgeKind::Mapper2(Box::new(Mapper2::new(prg_rom_size, chr_size))),
            3 => CartridgeKind::Mapper3(Box::new(Mapper3::new(prg_rom_size, chr_size))),
            4 => CartridgeKind::Mapper4(Box::new(Mapper4::new(prg_rom_size, chr_size))),
            _ => {
                critical!(CAT, "Unsupported mapper: {}", mapper);
            }
        };

        self.header = header;
        self.kind = kind;

        info!(SYS, "Loaded the cartridge.");
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
        let prg_rom_start = unsafe { (Self::NES_FILE_ADDR as *mut u8).add(size_of::<NESHeader>()) };
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

    pub fn irq_clock(&mut self, cpu: &mut CPU) {
        if let CartridgeKind::Mapper4(mapper) = &mut self.kind {
            if mapper.irq_clock() {
                cpu.interrupt(InterruptType::IRQ);
            }
        }
    }

    pub fn working_ram(&mut self) -> Option<&mut [u8]> {
        match &mut self.kind {
            CartridgeKind::Mapper4(mapper) => Some(mapper.working_ram()),
            _ => None,
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
