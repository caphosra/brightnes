use crate::{
    log,
    nes::{
        cartridge::Cartridge,
        cpu::{bus::CPUBus, NESCPU},
    },
};

#[derive(Clone, Copy)]
pub struct Sprite {
    pub y: u8,
    pub pattern_index: u8,
    pub attributes: u8,
    pub x: u8,
}

impl Sprite {
    pub fn new() -> Self {
        Sprite {
            y: 0,
            pattern_index: 0,
            attributes: 0,
            x: 0,
        }
    }

    pub fn palette_idx(&self) -> usize {
        (self.attributes & 0b11) as usize
    }

    pub fn background(&self) -> bool {
        (self.attributes & 1 << 5) != 0
    }

    pub fn flip_horizontal(&self) -> bool {
        (self.attributes & 1 << 6) != 0
    }

    pub fn flip_vertical(&self) -> bool {
        (self.attributes & 1 << 7) != 0
    }
}

const OAM_DMA_CYCLES: u32 = 513;

pub struct OAM {
    pub sprites: [Sprite; 64],
}

impl OAM {
    pub fn new() -> Self {
        OAM {
            sprites: [Sprite::new(); 64],
        }
    }

    pub fn write(&mut self, addr: u8, val: u8) {
        let index = (addr / 4) as usize;
        match addr % 4 {
            0 => {
                self.sprites[index].y = val;
            }
            1 => {
                self.sprites[index].pattern_index = val;
            }
            2 => {
                self.sprites[index].attributes = val;
            }
            3 => {
                self.sprites[index].x = val;
            }
            _ => {}
        }
    }

    pub fn direct_mem_access(&mut self, hi: u8, cartridge: &mut Cartridge) {
        let base_addr = (hi as u16) << 8;
        for i in 0..=0xFF {
            let addr = base_addr + i as u16;
            self.write(i, CPUBus::read(addr, cartridge));
        }
        log!(OAM, "OAM DMA from {:#06X}", base_addr);
        NESCPU::stall(OAM_DMA_CYCLES);
    }
}
