use heapless::Vec;
use serde::{Deserialize, Serialize};

use crate::nes::cpu::NESCPU;

#[derive(Clone, Copy, Serialize, Deserialize)]
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

pub const OAM_DMA_CYCLES: u32 = 513;
const OAM_SPRITE_NUM: usize = 64;

#[derive(Serialize, Deserialize)]
pub struct OAM {
    pub sprites: Vec<Sprite, OAM_SPRITE_NUM>,
    pub dma_request_addr: u16,
}

impl OAM {
    pub fn new() -> Self {
        OAM {
            sprites: Vec::from_array([Sprite::new(); OAM_SPRITE_NUM]),
            dma_request_addr: 0,
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

    pub fn request_dma_transfer(&mut self, hi: u8, cpu: &mut NESCPU) {
        self.dma_request_addr = (hi as u16) << 8;
        cpu.dma_stall();
    }
}
