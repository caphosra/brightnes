use alloc::vec::Vec;
use spin::{Lazy, RwLock};

use crate::{
    frame_buffer::{FrameBuffer, PixelColor},
    log,
    nes::{bus::NESBus, cpu::NESCPU, rom::NES_ROM},
    proc::{Process, ProcessMode},
};

const NES_FRAME_WIDTH: usize = 256;
const NES_FRAME_HEIGHT: usize = 240;

pub struct NESFrameBuffer {
    data: Vec<PixelColor>,
    offset_x: usize,
    offset_y: usize,
    pixel_size: usize,
}

pub static NES_FRAME_BUFFER: Lazy<RwLock<NESFrameBuffer>> = Lazy::new(|| {
    RwLock::new(NESFrameBuffer {
        data: Vec::with_capacity(NES_FRAME_WIDTH * NES_FRAME_HEIGHT),
        offset_x: 0,
        offset_y: 0,
        pixel_size: 0,
    })
});

impl NESFrameBuffer {
    pub fn init(&mut self) {
        let raw_buffer = FrameBuffer::get();

        for _ in 0..NES_FRAME_WIDTH * NES_FRAME_HEIGHT {
            self.data.push(raw_buffer.make_color(0xFF, 0xFF, 0xFF));
        }

        self.pixel_size =
            (raw_buffer.width / NES_FRAME_WIDTH).min(raw_buffer.height / NES_FRAME_HEIGHT);

        log!("[FB] Set pixel size: {}", self.pixel_size);

        self.offset_x = (raw_buffer.width - self.pixel_size * NES_FRAME_WIDTH) / 2;
        self.offset_y = (raw_buffer.height - self.pixel_size * NES_FRAME_HEIGHT) / 2;
    }

    pub fn bg_color(raw_buffer: &FrameBuffer) -> PixelColor {
        raw_buffer.make_color(0x0, 0x0, 0x0)
    }

    pub fn render_all(&self) {
        let raw_buffer = FrameBuffer::get();
        let bg_color = NESFrameBuffer::bg_color(&raw_buffer);

        raw_buffer.clear(bg_color);
        for y in 0..NES_FRAME_HEIGHT {
            for x in 0..NES_FRAME_WIDTH {
                let pixel = self.data[y * NES_FRAME_WIDTH + x];
                raw_buffer.draw_rect(
                    self.offset_x + x * self.pixel_size,
                    self.offset_y + y * self.pixel_size,
                    self.pixel_size,
                    self.pixel_size,
                    pixel,
                );
            }
        }
    }

    pub fn set_color(&mut self, x: usize, y: usize, color: PixelColor) {
        assert!(x < NES_FRAME_WIDTH);
        assert!(y < NES_FRAME_HEIGHT);

        self.data[y * NES_FRAME_WIDTH + x] = color;

        if Process::mode() == ProcessMode::Game {
            let raw_buffer = FrameBuffer::get();
            raw_buffer.draw_rect(
                self.offset_x + x * self.pixel_size,
                self.offset_y + y * self.pixel_size,
                self.pixel_size,
                self.pixel_size,
                color,
            );
        }
    }
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
    pub colors: [u8; 32],
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
    pub reg_scroll_x: u8,
    pub reg_scroll_y: u8,
    pub reg_scroll_is_x: bool,
    pub reg_status: u8,
    pub reg_data: u16,
    pub reg_data_is_lo: bool,

    pub x: u16,
    pub y: u16,

    pub name_table: [NameTable; 4],
    pub attribute_table: [AttributeTable; 4],
    pub bg_palette_table: PaletteTable,
    pub sprite_palette_table: PaletteTable,
    pub oam: OAM,
}

const PPU_CTRL_ADDR: u16 = 0x2000;
const PPU_MASK_ADDR: u16 = 0x2001;
const PPU_STATUS_ADDR: u16 = 0x2002;
const PPU_OAM_ADDR: u16 = 0x2003;
const PPU_OAM_DATA_ADDR: u16 = 0x2004;
const PPU_SCROLL_ADDR: u16 = 0x2005;
const PPU_ADDR: u16 = 0x2006;
const PPU_DATA_ADDR: u16 = 0x2007;
const OAM_DMA_ADDR: u16 = 0x4014;

#[repr(C)]
enum PPUCTRLFlag {
    I = 2,
}

impl NESPPU {
    pub fn read_mem(&self, addr: u16) -> u8 {
        if addr < 0x2000 {
            // CHR ROM
            let rom = NES_ROM.get().unwrap();
            rom.chr_rom[addr as usize]
        } else if addr < 0x3000 {
            let idx = (addr - 0x2000) / 0x400;
            let offset = (addr - 0x2000) % 0x400;
            if offset < NAME_TABLE_SIZE as u16 {
                // Name Table
                self.name_table[idx as usize].palette_ids[offset as usize]
            } else {
                // Attribute Table
                self.attribute_table[idx as usize].attributes
                    [(offset - NAME_TABLE_SIZE as u16) as usize]
            }
        } else if addr < 0x3F00 {
            // Mirrors of $2000-$2EFF
            self.read_mem(addr - 0x1000)
        } else if addr < 0x3F10 {
            // Background Palette
            self.bg_palette_table.colors[addr as usize - 0x3F00]
        } else if addr < 0x3F20 {
            // Sprite Palette
            self.sprite_palette_table.colors[addr as usize - 0x3F10]
        } else if addr < 0x4000 {
            // Mirrors of $3F00-$3F1F
            self.read_mem(addr - 0x20)
        } else {
            log!("[PPU] Invalid address reading: {:#06X}", addr);
            0
        }
    }

    pub fn write_mem(&mut self, addr: u16, val: u8) {
        if addr < 0x2000 {
            // CHR ROM
            log!("[PPU] Attempt to write to CHR ROM: {:#06X}", addr);
        } else if addr < 0x3000 {
            let idx = (addr - 0x2000) / 0x400;
            let offset = (addr - 0x2000) % 0x400;
            if offset < NAME_TABLE_SIZE as u16 {
                // Name Table
                self.name_table[idx as usize].palette_ids[offset as usize] = val;
            } else {
                // Attribute Table
                self.attribute_table[idx as usize].attributes
                    [(offset - NAME_TABLE_SIZE as u16) as usize] = val;
            }
        } else if addr < 0x3F00 {
            // Mirrors of $2000-$2EFF
            self.write_mem(addr - 0x1000, val);
        } else if addr < 0x3F10 {
            // Background Palette
            self.bg_palette_table.colors[addr as usize - 0x3F00] = val;
        } else if addr < 0x3F20 {
            // Sprite Palette
            self.sprite_palette_table.colors[addr as usize - 0x3F10] = val;
        } else if addr < 0x4000 {
            // Mirrors of $3F00-$3F1F
            self.write_mem(addr - 0x20, val);
        } else {
            log!("[PPU] Invalid address writing: {:#06X}", addr);
        }
    }

    pub fn read_reg(&mut self, addr: u16) -> u8 {
        if addr == PPU_STATUS_ADDR {
            // PPU_STATUS
            self.reg_status
        } else if addr == PPU_DATA_ADDR {
            // PPU_DATA
            self.read_mem(self.reg_data)
        } else {
            log!("[PPU] Invalid register reading: {:#06X}", addr);
            0
        }
    }

    pub fn write_reg(&mut self, addr: u16, val: u8) {
        if addr == PPU_CTRL_ADDR {
            // PPU_CTRL
            self.reg_ctrl = val;
        } else if addr == PPU_MASK_ADDR {
            // PPU_MASK
            self.reg_mask = val;
        } else if addr == PPU_OAM_ADDR {
            // PPU_OAM_ADDR
            self.reg_oam_addr = val;
        } else if addr == PPU_OAM_DATA_ADDR {
            // PPU_OAM_DATA
            self.oam.write(self.reg_oam_addr, val);
            self.reg_oam_addr = self.reg_oam_addr.wrapping_add(1);
        } else if addr == PPU_SCROLL_ADDR {
            // PPU_SCROLL
            if self.reg_scroll_is_x {
                self.reg_scroll_x = val;
                self.reg_scroll_is_x = false;
            } else {
                self.reg_scroll_y = val;
                self.reg_scroll_is_x = true;
            }
        } else if addr == PPU_ADDR {
            // PPU_ADDR
            if self.reg_data_is_lo {
                self.reg_data = (self.reg_data & 0xFF00) | val as u16;
                self.reg_data_is_lo = false;
            } else {
                self.reg_data = ((val as u16) << 8) | (self.reg_data & 0x00FF);
                self.reg_data_is_lo = true;
            }
        } else if addr == PPU_DATA_ADDR {
            let addr = self.reg_data;
            self.write_mem(addr, val);

            if self.reg_ctrl & (PPUCTRLFlag::I as u8) != 0 {
                self.reg_data = self.reg_data.wrapping_add(32);
            } else {
                self.reg_data = self.reg_data.wrapping_add(1);
            }
        } else if addr == OAM_DMA_ADDR {
            // OAM_DMA
            self.oam.direct_mem_access(val);
        } else {
            log!("[PPU] Invalid register writing: {:#06X}", addr);
        }
    }

    pub fn clock(&mut self) {}
}

pub static NES_PPU: Lazy<RwLock<NESPPU>> = Lazy::new(|| {
    RwLock::new(NESPPU {
        reg_ctrl: 0,
        reg_mask: 0,
        reg_oam_addr: 0,
        reg_scroll_x: 0,
        reg_scroll_y: 0,
        reg_scroll_is_x: true,
        reg_status: 0,
        reg_data: 0,
        reg_data_is_lo: false,

        x: 0,
        y: 0,

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
        bg_palette_table: PaletteTable { colors: [0; 32] },
        sprite_palette_table: PaletteTable { colors: [0; 32] },
        oam: OAM::new(),
    })
});
