use core::ops::Index;

use crate::nes::{pad::PADS, ram::NES_RAM, rom::NES_ROM};

pub struct NESBus;

impl NESBus {
    pub fn read(addr: u16) -> u8 {
        if addr < 0x2000 {
            // RAM
            let ram = NES_RAM.read();
            ram.ram[addr as usize & 0x7FF]
        } else if addr < 0x4000 {
            // PPU
            0
        } else if addr < 0x4016 {
            // APU
            0
        } else if addr == 0x4016 {
            // Pad 1
            let mut pad = PADS.write();
            pad[0].read() as u8
        } else if addr == 0x4017 {
            // Pad 2
            let mut pad = PADS.write();
            pad[1].read() as u8
        } else if addr < 0x8000 {
            // Reserved
            0
        } else {
            // ROM
            let rom = NES_ROM.get().unwrap();
            rom.prg_rom[addr as usize - 0x8000]
        }
    }

    pub fn write(addr: u16, data: u8) {
        if addr < 0x2000 {
            // RAM
            let mut ram = NES_RAM.write();
            ram.ram[addr as usize & 0x7FF] = data;
        } else if addr < 0x4000 {
            // PPU
            ()
        } else if addr < 0x4016 {
            // APU
            ()
        } else if addr == 0x4016 {
            // Pad 1
            let mut pad = PADS.write();
            pad[0].write(data & 1 == 1);
        } else if addr == 0x4017 {
            // Pad 2
            let mut pad = PADS.write();
            pad[1].write(data & 1 == 1);
        } else if addr < 0x8000 {
            // Reserved
            ()
        } else {
            // ROM
            ()
        }
    }
}
