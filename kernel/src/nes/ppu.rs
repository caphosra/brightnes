use alloc::vec::Vec;
use spin::{Lazy, RwLock};

use crate::{
    frame_buffer::{FrameBuffer, PixelColor},
    log,
    nes::{
        bus::NESBus,
        cpu::{InterruptType, NESCPU},
        rom::NES_ROM,
        Mirroring, NES_CONFIG,
    },
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
    pub pattern_ids: [u8; NAME_TABLE_SIZE],
}

impl NameTable {
    pub fn new() -> Self {
        NameTable {
            pattern_ids: [0; NAME_TABLE_SIZE],
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

impl PaletteTable {
    pub fn get_encoded_color(&self, palette_idx: usize, color_idx: usize) -> u8 {
        assert!(palette_idx < 8);
        assert!(color_idx < 4);

        self.colors[palette_idx * 4 + color_idx] & 0x3F
    }

    pub fn decode_color(encoded: u8) -> PixelColor {
        let buffer = FrameBuffer::get();
        match encoded {
            0x00 => buffer.make_color(0x62, 0x62, 0x62),
            0x01 => buffer.make_color(0x00, 0x1C, 0x95),
            0x02 => buffer.make_color(0x19, 0x04, 0xAC),
            0x03 => buffer.make_color(0x42, 0x00, 0x9D),
            0x04 => buffer.make_color(0x61, 0x00, 0x6B),
            0x05 => buffer.make_color(0x6E, 0x00, 0x25),
            0x06 => buffer.make_color(0x65, 0x05, 0x00),
            0x07 => buffer.make_color(0x49, 0x1E, 0x00),
            0x08 => buffer.make_color(0x22, 0x37, 0x00),
            0x09 => buffer.make_color(0x00, 0x49, 0x00),
            0x0A => buffer.make_color(0x00, 0x4F, 0x00),
            0x0B => buffer.make_color(0x00, 0x48, 0x16),
            0x0C => buffer.make_color(0x00, 0x35, 0x5E),
            0x0D => buffer.make_color(0x00, 0x00, 0x00),
            0x0E => buffer.make_color(0x00, 0x00, 0x00),
            0x0F => buffer.make_color(0x00, 0x00, 0x00),
            0x10 => buffer.make_color(0xAB, 0xAB, 0xAB),
            0x11 => buffer.make_color(0x0C, 0x4E, 0xDB),
            0x12 => buffer.make_color(0x3D, 0x2E, 0xFF),
            0x13 => buffer.make_color(0x71, 0x15, 0xF3),
            0x14 => buffer.make_color(0x9B, 0x0B, 0xB9),
            0x15 => buffer.make_color(0xB0, 0x12, 0x62),
            0x16 => buffer.make_color(0xA9, 0x27, 0x04),
            0x17 => buffer.make_color(0x89, 0x46, 0x00),
            0x18 => buffer.make_color(0x57, 0x66, 0x00),
            0x19 => buffer.make_color(0x23, 0x7F, 0x00),
            0x1A => buffer.make_color(0x00, 0x89, 0x00),
            0x1B => buffer.make_color(0x00, 0x83, 0x32),
            0x1C => buffer.make_color(0x00, 0x6D, 0x90),
            0x1D => buffer.make_color(0x00, 0x00, 0x00),
            0x1E => buffer.make_color(0x00, 0x00, 0x00),
            0x1F => buffer.make_color(0x00, 0x00, 0x00),
            0x20 => buffer.make_color(0xFF, 0xFF, 0xFF),
            0x21 => buffer.make_color(0x57, 0xA5, 0xFF),
            0x22 => buffer.make_color(0x82, 0x87, 0xFF),
            0x23 => buffer.make_color(0xB4, 0x6D, 0xFF),
            0x24 => buffer.make_color(0xDF, 0x60, 0xFF),
            0x25 => buffer.make_color(0xF8, 0x63, 0xC6),
            0x26 => buffer.make_color(0xF8, 0x74, 0x6D),
            0x27 => buffer.make_color(0xDE, 0x90, 0x20),
            0x28 => buffer.make_color(0xB3, 0xAE, 0x00),
            0x29 => buffer.make_color(0x81, 0xC8, 0x00),
            0x2A => buffer.make_color(0x56, 0xD5, 0x22),
            0x2B => buffer.make_color(0x3D, 0xD3, 0x6F),
            0x2C => buffer.make_color(0x3E, 0xC1, 0xC8),
            0x2D => buffer.make_color(0x4E, 0x4E, 0x4E),
            0x2E => buffer.make_color(0x00, 0x00, 0x00),
            0x2F => buffer.make_color(0x00, 0x00, 0x00),
            0x30 => buffer.make_color(0xFF, 0xFF, 0xFF),
            0x31 => buffer.make_color(0xBE, 0xE0, 0xFF),
            0x32 => buffer.make_color(0xCD, 0xD4, 0xFF),
            0x33 => buffer.make_color(0xE0, 0xCA, 0xFF),
            0x34 => buffer.make_color(0xF1, 0xC4, 0xFF),
            0x35 => buffer.make_color(0xFC, 0xC4, 0xEF),
            0x36 => buffer.make_color(0xFD, 0xCA, 0xCE),
            0x37 => buffer.make_color(0xF5, 0xD4, 0xAF),
            0x38 => buffer.make_color(0xE6, 0xDF, 0x9C),
            0x39 => buffer.make_color(0xD3, 0xE9, 0x9A),
            0x3A => buffer.make_color(0xC2, 0xEF, 0xA8),
            0x3B => buffer.make_color(0xB7, 0xEF, 0xC4),
            0x3C => buffer.make_color(0xB6, 0xEA, 0xE5),
            0x3D => buffer.make_color(0xB8, 0xB8, 0xB8),
            0x3E => buffer.make_color(0x00, 0x00, 0x00),
            0x3F => buffer.make_color(0x00, 0x00, 0x00),
            _ => buffer.make_color(0x00, 0x00, 0x00),
        }
    }

    pub fn get_color(&self, palette_idx: usize, color_idx: usize) -> PixelColor {
        assert!(palette_idx < 8);
        assert!(color_idx < 4);

        let encoded = self.get_encoded_color(palette_idx, color_idx);
        PaletteTable::decode_color(encoded)
    }
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

const PALETTE_BASE_ADDR: u16 = 0x3f00;

const BG_TILE_SIZE: u16 = 8;
const PATTERN_SIZE: u16 = 16;

const PPU_CYCLE: u16 = 341;
const PPU_VBLANK: u16 = 22;

impl NESPPU {
    pub fn ctrl_name_table(&self) -> u8 {
        self.reg_ctrl & 0x03
    }

    pub fn ctrl_increment(&self) -> u16 {
        if self.reg_ctrl & 0x04 != 0 {
            32
        } else {
            1
        }
    }

    pub fn ctrl_sprite_pattern_table(&self) -> u16 {
        if self.reg_ctrl & 0x08 != 0 {
            0x1000
        } else {
            0x0000
        }
    }

    pub fn ctrl_bg_pattern_table(&self) -> u16 {
        if self.reg_ctrl & 0x10 != 0 {
            0x1000
        } else {
            0x0000
        }
    }

    pub fn ctrl_sprite_size(&self) -> u8 {
        if self.reg_ctrl & 0x20 != 0 {
            16
        } else {
            8
        }
    }

    pub fn ctrl_master_slave(&self) -> bool {
        self.reg_ctrl & 0x40 != 0
    }

    pub fn ctrl_nmi_enable(&self) -> bool {
        self.reg_ctrl & 0x80 != 0
    }

    pub fn mask_gley_scale(&self) -> bool {
        self.reg_mask & 0x01 != 0
    }

    pub fn mask_bg_visible_left8(&self) -> bool {
        self.reg_mask & 0x02 != 0
    }

    pub fn mask_sprite_visible_left8(&self) -> bool {
        self.reg_mask & 0x04 != 0
    }

    pub fn mask_bg_visible(&self) -> bool {
        self.reg_mask & 0x08 != 0
    }

    pub fn mask_sprite_visible(&self) -> bool {
        self.reg_mask & 0x10 != 0
    }

    pub fn mask_emphasize_red(&self) -> bool {
        self.reg_mask & 0x20 != 0
    }

    pub fn mask_emphasize_green(&self) -> bool {
        self.reg_mask & 0x40 != 0
    }

    pub fn mask_emphasize_blue(&self) -> bool {
        self.reg_mask & 0x80 != 0
    }

    pub fn read_mem(&self, addr: u16) -> u8 {
        if addr < 0x2000 {
            // CHR ROM
            let rom = NES_ROM.get().unwrap();
            rom.chr_rom[addr as usize]
        } else if addr < 0x3000 {
            let idx = (addr - 0x2000) / 0x400;
            let offset = (addr - 0x2000) % 0x400;

            // Consider mirroring
            let config = NES_CONFIG.read();
            let idx = match config.mirroring {
                Mirroring::Horizontal => idx / 2,
                Mirroring::Vertical => idx % 2,
            };

            if offset < NAME_TABLE_SIZE as u16 {
                // Name Table
                self.name_table[idx as usize].pattern_ids[offset as usize]
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

            // Consider mirroring
            let config = NES_CONFIG.read();
            let idx = match config.mirroring {
                Mirroring::Horizontal => idx / 2,
                Mirroring::Vertical => idx % 2,
            };

            if offset < NAME_TABLE_SIZE as u16 {
                // Name Table
                self.name_table[idx as usize].pattern_ids[offset as usize] = val;
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

            self.reg_data = self.reg_data.wrapping_add(self.ctrl_increment());
        } else if addr == OAM_DMA_ADDR {
            // OAM_DMA
            self.oam.direct_mem_access(val);
        } else {
            log!("[PPU] Invalid register writing: {:#06X}", addr);
        }
    }

    pub fn read_pattern(&self, id: u8, x: u8, y: u8) -> u8 {
        assert!(x < 8);
        assert!(y < 8);

        let lo = self.read_mem(id as u16 * PATTERN_SIZE + y as u16);
        let hi = self.read_mem(id as u16 * PATTERN_SIZE + PATTERN_SIZE / 2 + y as u16);
        let lo_bit = (lo & (1 << x)) >> x;
        let hi_bit = (hi & (1 << x)) >> x;
        (hi_bit << 1) | lo_bit
    }

    pub fn get_bg_color(&self, x: u8, y: u8) -> Option<PixelColor> {
        let global_x = self.reg_scroll_x as u16 + x as u16;
        let global_y = self.reg_scroll_y as u16 + y as u16;
        let x = global_x / BG_TILE_SIZE;
        let y = global_y / BG_TILE_SIZE;

        let horizontal_tiles = NES_FRAME_WIDTH as u16 / BG_TILE_SIZE;
        let vertical_tiles = NES_FRAME_HEIGHT as u16 / BG_TILE_SIZE;

        let x_page = x / horizontal_tiles;
        let y_page = y / vertical_tiles;
        let table_idx = (y_page * 2 + x_page) as usize;

        let x_offset = x % horizontal_tiles;
        let y_offset = y % vertical_tiles;
        let pattern = self.name_table[table_idx].pattern_ids
            [(y_offset * horizontal_tiles + x_offset) as usize];

        let x_offset = x_offset / 2;
        let y_offset = y_offset / 2;
        let attribute = self.attribute_table[table_idx].attributes
            [(y_offset / 2 * (vertical_tiles / 4) + x_offset / 2) as usize];

        let internal_offset = y_offset % 2 * 2 + x_offset % 2;
        let palette_idx =
            ((attribute & (0b11 << (internal_offset * 2))) >> (internal_offset * 2)) as usize;

        let color_idx = self.read_pattern(
            pattern,
            (global_x % BG_TILE_SIZE) as u8,
            (global_y % BG_TILE_SIZE) as u8,
        ) as usize;

        if color_idx == 0 {
            None
        } else {
            Some(self.bg_palette_table.get_color(palette_idx, color_idx))
        }
    }

    pub fn render(&mut self, x: u8, y: u8) {
        let mut buffer = NES_FRAME_BUFFER.write();
        match self.get_bg_color(x, y) {
            Some(color) => {
                buffer.set_color(x as usize, y as usize, color);
            }
            None => {
                let color = PaletteTable::decode_color(self.read_mem(PALETTE_BASE_ADDR));
                buffer.set_color(x as usize, y as usize, color);
            }
        }
    }

    pub fn clock(&mut self) {
        if self.x < NES_FRAME_WIDTH as u16 && self.y < NES_FRAME_HEIGHT as u16 {
            self.render(self.x as u8, self.y as u8);
        }

        if self.x == 0 && self.y == NES_FRAME_HEIGHT as u16 {
            // VBLANK
            NESCPU::interrupt(InterruptType::NMI);

            // Set VBLANK flag
            self.reg_status |= 0x80;
        }

        self.x += 1;
        if self.x >= PPU_CYCLE {
            self.x = 0;
            self.y += 1;
        }
        if self.y >= NES_FRAME_HEIGHT as u16 + PPU_VBLANK {
            self.y = 0;

            // Clear VBLANK flag
            self.reg_status &= 0x7F;
        }
    }
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
