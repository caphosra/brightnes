use crate::{
    log,
    nes::{pad::PADS, ppu::NES_PPU, ram::NES_RAM, rom::NES_ROM},
};

pub struct NESBus;

impl NESBus {
    pub fn read(addr: u16) -> u8 {
        if addr < 0x2000 {
            // RAM
            let ram = NES_RAM.read();
            ram.ram[addr as usize & 0x7FF]
        } else if addr < 0x4000 {
            // PPU
            let mut ppu = NES_PPU.write();
            ppu.read_reg(addr)
        } else if addr < 0x4016 {
            // APU
            log!("[BUS] Attempt to read from APU: {:#06X}", addr);
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
            log!("[BUS] Attempt to read from reserved area: {:#06X}", addr);
            0
        } else {
            // ROM
            let rom = NES_ROM.get().unwrap();
            let prog_addr = (addr as usize - 0x8000) % 0x4000;
            rom.prg_rom[prog_addr]
        }
    }

    pub fn write(addr: u16, data: u8) {
        if addr < 0x2000 {
            // RAM
            let mut ram = NES_RAM.write();
            ram.ram[addr as usize & 0x7FF] = data;
        } else if addr < 0x4000 {
            // PPU
            let mut ppu = NES_PPU.write();
            ppu.write_reg(addr, data);
        } else if addr < 0x4016 {
            // APU
            log!("[BUS] Attempt to write to APU: {:#06X}", addr);
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
            log!("[BUS] Attempt to write to reserved area: {:#06X}", addr);
        } else {
            // ROM
            log!("[BUS] Attempt to write to ROM: {:#06X}", addr);
        }
    }
}
