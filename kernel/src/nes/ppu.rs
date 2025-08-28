use spin::{Lazy, RwLock};

use crate::{
    frame_buffer::PixelColor,
    nes::{
        bus::NESBus,
        cpu::{NESCPU, NES_CPU},
    },
};

const NES_FRAME_WIDTH: usize = 256;
const NES_FRAME_HEIGHT: usize = 240;

pub struct NESFrameBuffer {
    pub data: [PixelColor; NES_FRAME_WIDTH * NES_FRAME_HEIGHT],
}

const NAME_TABLE_SIZE: usize = 0x3C0;

pub struct NameTable {
    pub palette_ids: [u8; NAME_TABLE_SIZE],
}

impl NameTable {
    pub fn new() -> Self {
        NameTable {
            palette_ids: [0; NAME_TABLE_SIZE],
        }
    }
}

pub struct AttributeTable {
    pub attributes: [u8; 0x40],
}

impl AttributeTable {
    pub fn new() -> Self {
        AttributeTable {
            attributes: [0; 0x40],
        }
    }
}

pub struct PaletteTable {
    pub colors: [u32; 4],
}

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

    pub fn read(&self, addr: u8) -> u8 {
        let index = (addr / 4) as usize;
        match addr % 4 {
            0 => self.sprites[index].y,
            1 => self.sprites[index].pattern_index,
            2 => self.sprites[index].attributes,
            3 => self.sprites[index].x,
            _ => 0,
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

    pub fn direct_mem_access(&mut self, hi: u8) {
        let base_addr = (hi as u16) << 8;
        for i in 0..=0xFF {
            let addr = base_addr + i as u16;
            self.write(i, NESBus::read(addr));
        }
        NESCPU::stall(OAM_DMA_CYCLES);
    }
}

pub struct NESPPU {
    pub reg_ctrl: u8,
    pub reg_mask: u8,
    pub reg_oam_addr: u8,
    pub reg_oam_data: u8,
    pub reg_scroll: u8,
    pub reg_status: u8,
    pub reg_data: u8,
    pub name_table: [NameTable; 4],
    pub attribute_table: [AttributeTable; 4],
    pub bg_palette_table: PaletteTable,
    pub sprite_palette_table: PaletteTable,
    pub oam: OAM,
}

impl NESPPU {
    pub fn read(&self, addr: u16) -> u8 {
        0
    }
}

pub static NES_PPU: Lazy<RwLock<NESPPU>> = Lazy::new(|| {
    RwLock::new(NESPPU {
        reg_ctrl: 0,
        reg_mask: 0,
        reg_oam_addr: 0,
        reg_oam_data: 0,
        reg_scroll: 0,
        reg_status: 0,
        reg_data: 0,
        name_table: [
            NameTable::new(),
            NameTable::new(),
            NameTable::new(),
            NameTable::new(),
        ],
        attribute_table: [
            AttributeTable::new(),
            AttributeTable::new(),
            AttributeTable::new(),
            AttributeTable::new(),
        ],
        bg_palette_table: PaletteTable { colors: [0; 4] },
        sprite_palette_table: PaletteTable { colors: [0; 4] },
        oam: OAM::new(),
    })
});
