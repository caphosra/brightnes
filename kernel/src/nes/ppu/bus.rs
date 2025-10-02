use crate::{
    critical,
    nes::{cartridge::Cartridge, ppu::vram::VRAM, Mirroring},
};

pub struct PPUBus;

impl PPUBus {
    pub fn read(addr: u16, vram: &VRAM, cartridge: &mut Cartridge) -> u8 {
        if addr < 0x2000 {
            // CHR ROM
            cartridge.read_ppu_mem(addr)
        } else if addr < 0x3000 {
            // Consider mirroring
            let addr = {
                match cartridge.mirroring() {
                    Mirroring::Horizontal => addr & !0x400,
                    Mirroring::Vertical => addr & !0x800,
                }
            };

            vram.read(addr)
        } else if addr < 0x3F00 {
            // Mirrors of $2000-$2EFF
            Self::read(addr - 0x1000, vram, cartridge)
        } else if addr < 0x3F10 {
            // Background Palette
            vram.read(addr)
        } else if addr < 0x3F20 {
            // Sprite Palette
            if addr & 0b11 == 0 {
                Self::read(addr - 0x10, vram, cartridge)
            } else {
                vram.read(addr)
            }
        } else if addr < 0x4000 {
            // Mirrors of $3F00-$3F1F
            Self::read(addr - 0x20, vram, cartridge)
        } else {
            critical!(PPU, "Invalid address reading: {:#06X}", addr);
        }
    }

    pub fn write(addr: u16, val: u8, vram: &mut VRAM, cartridge: &mut Cartridge) {
        if addr < 0x2000 {
            // CHR ROM
            cartridge.write_ppu_mem(addr, val);
        } else if addr < 0x3000 {
            // Consider mirroring
            let addr = {
                match cartridge.mirroring() {
                    Mirroring::Horizontal => addr & !0x400,
                    Mirroring::Vertical => addr & !0x800,
                }
            };

            vram.write(addr, val);
        } else if addr < 0x3F00 {
            // Mirrors of $2000-$2EFF
            Self::write(addr - 0x1000, val, vram, cartridge);
        } else if addr < 0x3F10 {
            // Background Palette
            vram.write(addr, val);
        } else if addr < 0x3F20 {
            // Sprite Palette
            if addr & 0b11 == 0 {
                Self::write(addr - 0x10, val, vram, cartridge);
            } else {
                vram.write(addr, val);
            }
        } else if addr < 0x4000 {
            // Mirrors of $3F00-$3F1F
            Self::write(addr - 0x20, val, vram, cartridge);
        } else {
            critical!(PPU, "Invalid address writing: {:#06X}", addr);
        }
    }
}
