use alloc::vec;
use alloc::vec::Vec;
use spin::{Lazy, RwLock};

use crate::{
    critical,
    frame_buffer::{FrameBuffer, PixelColor, UNDEF_COLOR},
    log,
    nes::{
        bus::CPUBus,
        cartridge::Cartridge,
        cpu::{InterruptType, NESCPU},
        Mirroring,
    },
};

const NES_FRAME_WIDTH: usize = 256;
const NES_FRAME_HEIGHT: usize = 240;

pub static GAME_FB: Lazy<RwLock<FrameBuffer>> = Lazy::new(|| {
    let (width, height) = FrameBuffer::max_size();

    let pixel_size = (width / NES_FRAME_WIDTH).min(height / NES_FRAME_HEIGHT);
    let offset_x = (width - pixel_size * NES_FRAME_WIDTH) / 2;
    let offset_y = (height - pixel_size * NES_FRAME_HEIGHT) / 2;

    RwLock::new(FrameBuffer::new(
        offset_x, offset_y, width, height, pixel_size,
    ))
});

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

trait NESColorConverter {
    fn from_nes_color(nes_color: u8, grey_scale: bool) -> Self;
}

impl NESColorConverter for PixelColor {
    fn from_nes_color(nes_color: u8, grey_scale: bool) -> Self {
        assert!(nes_color < 0x40);

        // Converts to grey scale if needed
        let nes_color = if grey_scale {
            nes_color & 0x30
        } else {
            nes_color
        };

        match nes_color {
            0x00 => FrameBuffer::make_color(0x62, 0x62, 0x62),
            0x01 => FrameBuffer::make_color(0x00, 0x1C, 0x95),
            0x02 => FrameBuffer::make_color(0x19, 0x04, 0xAC),
            0x03 => FrameBuffer::make_color(0x42, 0x00, 0x9D),
            0x04 => FrameBuffer::make_color(0x61, 0x00, 0x6B),
            0x05 => FrameBuffer::make_color(0x6E, 0x00, 0x25),
            0x06 => FrameBuffer::make_color(0x65, 0x05, 0x00),
            0x07 => FrameBuffer::make_color(0x49, 0x1E, 0x00),
            0x08 => FrameBuffer::make_color(0x22, 0x37, 0x00),
            0x09 => FrameBuffer::make_color(0x00, 0x49, 0x00),
            0x0A => FrameBuffer::make_color(0x00, 0x4F, 0x00),
            0x0B => FrameBuffer::make_color(0x00, 0x48, 0x16),
            0x0C => FrameBuffer::make_color(0x00, 0x35, 0x5E),
            0x0D => FrameBuffer::make_color(0x00, 0x00, 0x00),
            0x0E => FrameBuffer::make_color(0x00, 0x00, 0x00),
            0x0F => FrameBuffer::make_color(0x00, 0x00, 0x00),
            0x10 => FrameBuffer::make_color(0xAB, 0xAB, 0xAB),
            0x11 => FrameBuffer::make_color(0x0C, 0x4E, 0xDB),
            0x12 => FrameBuffer::make_color(0x3D, 0x2E, 0xFF),
            0x13 => FrameBuffer::make_color(0x71, 0x15, 0xF3),
            0x14 => FrameBuffer::make_color(0x9B, 0x0B, 0xB9),
            0x15 => FrameBuffer::make_color(0xB0, 0x12, 0x62),
            0x16 => FrameBuffer::make_color(0xA9, 0x27, 0x04),
            0x17 => FrameBuffer::make_color(0x89, 0x46, 0x00),
            0x18 => FrameBuffer::make_color(0x57, 0x66, 0x00),
            0x19 => FrameBuffer::make_color(0x23, 0x7F, 0x00),
            0x1A => FrameBuffer::make_color(0x00, 0x89, 0x00),
            0x1B => FrameBuffer::make_color(0x00, 0x83, 0x32),
            0x1C => FrameBuffer::make_color(0x00, 0x6D, 0x90),
            0x1D => FrameBuffer::make_color(0x00, 0x00, 0x00),
            0x1E => FrameBuffer::make_color(0x00, 0x00, 0x00),
            0x1F => FrameBuffer::make_color(0x00, 0x00, 0x00),
            0x20 => FrameBuffer::make_color(0xFF, 0xFF, 0xFF),
            0x21 => FrameBuffer::make_color(0x57, 0xA5, 0xFF),
            0x22 => FrameBuffer::make_color(0x82, 0x87, 0xFF),
            0x23 => FrameBuffer::make_color(0xB4, 0x6D, 0xFF),
            0x24 => FrameBuffer::make_color(0xDF, 0x60, 0xFF),
            0x25 => FrameBuffer::make_color(0xF8, 0x63, 0xC6),
            0x26 => FrameBuffer::make_color(0xF8, 0x74, 0x6D),
            0x27 => FrameBuffer::make_color(0xDE, 0x90, 0x20),
            0x28 => FrameBuffer::make_color(0xB3, 0xAE, 0x00),
            0x29 => FrameBuffer::make_color(0x81, 0xC8, 0x00),
            0x2A => FrameBuffer::make_color(0x56, 0xD5, 0x22),
            0x2B => FrameBuffer::make_color(0x3D, 0xD3, 0x6F),
            0x2C => FrameBuffer::make_color(0x3E, 0xC1, 0xC8),
            0x2D => FrameBuffer::make_color(0x4E, 0x4E, 0x4E),
            0x2E => FrameBuffer::make_color(0x00, 0x00, 0x00),
            0x2F => FrameBuffer::make_color(0x00, 0x00, 0x00),
            0x30 => FrameBuffer::make_color(0xFF, 0xFF, 0xFF),
            0x31 => FrameBuffer::make_color(0xBE, 0xE0, 0xFF),
            0x32 => FrameBuffer::make_color(0xCD, 0xD4, 0xFF),
            0x33 => FrameBuffer::make_color(0xE0, 0xCA, 0xFF),
            0x34 => FrameBuffer::make_color(0xF1, 0xC4, 0xFF),
            0x35 => FrameBuffer::make_color(0xFC, 0xC4, 0xEF),
            0x36 => FrameBuffer::make_color(0xFD, 0xCA, 0xCE),
            0x37 => FrameBuffer::make_color(0xF5, 0xD4, 0xAF),
            0x38 => FrameBuffer::make_color(0xE6, 0xDF, 0x9C),
            0x39 => FrameBuffer::make_color(0xD3, 0xE9, 0x9A),
            0x3A => FrameBuffer::make_color(0xC2, 0xEF, 0xA8),
            0x3B => FrameBuffer::make_color(0xB7, 0xEF, 0xC4),
            0x3C => FrameBuffer::make_color(0xB6, 0xEA, 0xE5),
            0x3D => FrameBuffer::make_color(0xB8, 0xB8, 0xB8),
            0x3E => FrameBuffer::make_color(0x00, 0x00, 0x00),
            0x3F => FrameBuffer::make_color(0x00, 0x00, 0x00),
            _ => FrameBuffer::make_color(0x00, 0x00, 0x00),
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

pub struct NESPPU {
    pub reg_ctrl: u8,
    pub reg_mask: u8,
    pub reg_oam_addr: u8,
    pub reg_status: u8,
    pub reg_data: u16,
    pub reg_data_is_lo: bool,

    reg_data_buffer: u8,

    reg_v: u16,
    reg_t: u16,
    reg_x: u8,
    reg_w: bool,

    relative_x: u16,

    pub x: u16,
    pub y: u16,

    pub name_table: [NameTable; 4],
    pub attribute_table: [AttributeTable; 4],
    pub bg_palette_table: PaletteTable,
    pub sprite_palette_table: PaletteTable,
    pub oam: OAM,

    bg_transparent: Vec<bool>,
}

const PPU_CTRL_ADDR: u16 = 0x2000;
const PPU_MASK_ADDR: u16 = 0x2001;
const PPU_STATUS_ADDR: u16 = 0x2002;
const PPU_OAM_ADDR: u16 = 0x2003;
const PPU_OAM_DATA_ADDR: u16 = 0x2004;
const PPU_SCROLL_ADDR: u16 = 0x2005;
const PPU_ADDR: u16 = 0x2006;
const PPU_DATA_ADDR: u16 = 0x2007;
pub const OAM_DMA_ADDR: u16 = 0x4014;

const PALETTE_BASE_ADDR: u16 = 0x3F00;

const PATTERN_SIZE: u16 = 16;

const PPU_CYCLE: u16 = 341;
const PPU_VBLANK: u16 = 22;

impl NESPPU {
    const COARSE_X_MASK: u16 = 0b00000000_00011111;
    const COARSE_Y_MASK: u16 = 0b00000011_11100000;
    const FINE_Y_MASK: u16 = 0b01110000_00000000;
    const NAME_TABLE_MASK: u16 = 0b00001100_00000000;

    #[inline(always)]
    pub fn ctrl_increment(&self) -> u16 {
        if self.reg_ctrl & 0x04 != 0 {
            32
        } else {
            1
        }
    }

    #[inline(always)]
    pub fn ctrl_sprite_pattern_table(&self) -> u16 {
        if self.reg_ctrl & 0x08 != 0 {
            0x1000
        } else {
            0x0000
        }
    }

    #[inline(always)]
    pub fn ctrl_bg_pattern_table(&self) -> u16 {
        if self.reg_ctrl & 0x10 != 0 {
            0x1000
        } else {
            0x0000
        }
    }

    #[inline(always)]
    pub fn ctrl_sprite_size(&self) -> u8 {
        if self.reg_ctrl & 0x20 != 0 {
            16
        } else {
            8
        }
    }

    #[inline(always)]
    pub fn ctrl_nmi_enable(&self) -> bool {
        self.reg_ctrl & 0x80 != 0
    }

    #[inline(always)]
    pub fn mask_grey_scale(&self) -> bool {
        self.reg_mask & 0x01 != 0
    }

    #[inline(always)]
    pub fn mask_bg_visible_left8(&self) -> bool {
        self.reg_mask & 0x02 != 0
    }

    #[inline(always)]
    pub fn mask_sprite_visible_left8(&self) -> bool {
        self.reg_mask & 0x04 != 0
    }

    #[inline(always)]
    pub fn mask_bg_visible(&self) -> bool {
        self.reg_mask & 0x08 != 0
    }

    #[inline(always)]
    pub fn mask_sprite_visible(&self) -> bool {
        self.reg_mask & 0x10 != 0
    }

    fn read_mem(&self, addr: u16, cartridge: &mut Cartridge) -> u8 {
        if addr < 0x2000 {
            // CHR ROM
            cartridge.read_ppu_mem(addr)
        } else if addr < 0x3000 {
            let idx = (addr - 0x2000) / 0x400;
            let offset = (addr - 0x2000) % 0x400;

            // Consider mirroring
            let idx = {
                match cartridge.mirroring() {
                    Mirroring::Horizontal => idx / 2,
                    Mirroring::Vertical => idx % 2,
                }
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
            self.read_mem(addr - 0x1000, cartridge)
        } else if addr < 0x3F10 {
            // Background Palette
            self.bg_palette_table.colors[addr as usize - 0x3F00]
        } else if addr < 0x3F20 {
            // Sprite Palette
            if addr % 4 == 0 {
                self.read_mem(addr - 0x10, cartridge)
            } else {
                self.sprite_palette_table.colors[addr as usize - 0x3F10]
            }
        } else if addr < 0x4000 {
            // Mirrors of $3F00-$3F1F
            self.read_mem(addr - 0x20, cartridge)
        } else {
            critical!(PPU, "Invalid address reading: {:#06X}", addr);
        }
    }

    fn write_mem(&mut self, addr: u16, val: u8, cartridge: &mut Cartridge) {
        if addr < 0x2000 {
            // CHR ROM
            cartridge.write_ppu_mem(addr, val);
        } else if addr < 0x3000 {
            let idx = (addr - 0x2000) / 0x400;
            let offset = (addr - 0x2000) % 0x400;

            // Consider mirroring
            let idx = {
                match cartridge.mirroring() {
                    Mirroring::Horizontal => idx / 2,
                    Mirroring::Vertical => idx % 2,
                }
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
            self.write_mem(addr - 0x1000, val, cartridge);
        } else if addr < 0x3F10 {
            // Background Palette
            self.bg_palette_table.colors[addr as usize - 0x3F00] = val;
        } else if addr < 0x3F20 {
            // Sprite Palette
            if addr % 4 == 0 {
                self.write_mem(addr - 0x10, val, cartridge);
            } else {
                self.sprite_palette_table.colors[addr as usize - 0x3F10] = val;
            }
        } else if addr < 0x4000 {
            // Mirrors of $3F00-$3F1F
            self.write_mem(addr - 0x20, val, cartridge);
        } else {
            critical!(PPU, "Invalid address writing: {:#06X}", addr);
        }
    }

    pub fn read_reg(&mut self, addr: u16, cartridge: &mut Cartridge) -> u8 {
        // Mirroring every 8 bytes.
        let addr = 0x2000 + ((addr - 0x2000) & 0x7);
        if addr == PPU_STATUS_ADDR {
            // PPU_STATUS
            self.reg_w = false;

            self.reg_status
        } else if addr == PPU_DATA_ADDR {
            // PPU_DATA
            let data = self.reg_data_buffer;
            self.reg_data_buffer = self.read_mem(self.reg_data, cartridge);

            self.reg_data = self.reg_data.wrapping_add(self.ctrl_increment());

            data
        } else {
            critical!(PPU, "Invalid register reading: {:#06X}", addr);
        }
    }

    pub fn write_reg(&mut self, addr: u16, val: u8, cartridge: &mut Cartridge) {
        if addr == OAM_DMA_ADDR {
            // OAM_DMA
            self.oam.direct_mem_access(val, cartridge);
            return;
        }

        // Mirroring every 8 bytes.
        let addr = 0x2000 + ((addr - 0x2000) & 0x7);
        if addr == PPU_CTRL_ADDR {
            // PPU_CTRL
            self.reg_ctrl = val;

            self.reg_t = (self.reg_t & !Self::NAME_TABLE_MASK) | (((val as u16) & 0x03) << 10);
        } else if addr == PPU_MASK_ADDR {
            // PPU_MASK
            self.reg_mask = val;

            self.reg_w = false;
        } else if addr == PPU_OAM_ADDR {
            // PPU_OAM_ADDR
            self.reg_oam_addr = val;
        } else if addr == PPU_OAM_DATA_ADDR {
            // PPU_OAM_DATA
            self.oam.write(self.reg_oam_addr, val);
            self.reg_oam_addr = self.reg_oam_addr.wrapping_add(1);
        } else if addr == PPU_SCROLL_ADDR {
            // PPU_SCROLL
            if self.reg_w {
                // Second write configures Y
                self.reg_t = (self.reg_t & (!Self::COARSE_Y_MASK) & (!Self::FINE_Y_MASK))
                    | (((val as u16) >> 3) << 5)
                    | (((val as u16) & 0b111) << 12);
                self.reg_w = false;
            } else {
                // First write configures X
                self.reg_t = (self.reg_t & !Self::COARSE_X_MASK) | ((val as u16) >> 3);
                self.reg_x = val & 0b111;
                self.reg_w = true;
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

            if self.reg_w {
                // Second write
                self.reg_t = (self.reg_t & 0xFF00) | val as u16;
                self.reg_v = self.reg_t;
                self.reg_w = false;
            } else {
                // First write
                self.reg_t = (self.reg_t & 0x00FF) | (((val as u16) & 0b111111) << 8);
                self.reg_w = true;
            }
        } else if addr == PPU_DATA_ADDR {
            let addr = self.reg_data;
            self.write_mem(addr, val, cartridge);

            self.reg_data = self.reg_data.wrapping_add(self.ctrl_increment());
        } else {
            critical!(PPU, "Invalid register writing: {:#06X}", addr);
        }
    }

    #[inline(always)]
    fn tile_addr(&self) -> u16 {
        0x2000 | (self.reg_v & 0x0FFF)
    }

    #[inline(always)]
    fn attribute_addr(&self) -> u16 {
        0x23C0 | (self.reg_v & 0x0C00) | ((self.reg_v >> 4) & 0x38) | ((self.reg_v >> 2) & 0x07)
    }

    fn coarse_x_inc(&mut self) {
        if (self.reg_v & Self::COARSE_X_MASK) == 31 {
            self.reg_v &= !Self::COARSE_X_MASK;

            // Switch name table horizontally
            self.reg_v ^= 0b00000100_00000000;
        } else {
            self.reg_v += 1;
        }
    }

    fn y_inc(&mut self) {
        if (self.reg_v & Self::FINE_Y_MASK) != Self::FINE_Y_MASK {
            self.reg_v += 0x1000;
        } else {
            self.reg_v &= !Self::FINE_Y_MASK;

            let mut y = (self.reg_v & Self::COARSE_Y_MASK) >> 5;
            if y == 29 {
                y = 0;
                // Switch name table vertically
                self.reg_v ^= 0b00001000_00000000;
            } else if y == 31 {
                y = 0;
            } else {
                y += 1;
            }
            self.reg_v = (self.reg_v & !Self::COARSE_Y_MASK) | (y << 5);
        }
    }

    fn update_horizontal_v(&mut self) {
        self.reg_v = (self.reg_v & 0b01111011_11100000) | (self.reg_t & 0b00000100_00011111);
    }

    fn update_vertical_v(&mut self) {
        self.reg_v = (self.reg_v & 0b00000100_00011111) | (self.reg_t & 0b01111011_11100000);
    }

    fn get_palette_idx(&self, tile_id: u16, attribute: u8) -> u8 {
        let internal_offset = (((tile_id >> 6) & 1) << 1) | ((tile_id >> 1) & 1);
        (attribute >> (internal_offset * 2)) & 0b11
    }

    fn get_bg_color(&self, cartridge: &mut Cartridge) -> Option<PixelColor> {
        let tile_addr = self.tile_addr();
        let attribute_addr = self.attribute_addr();

        let attribute = self.read_mem(attribute_addr, cartridge);
        let palette_idx = self.get_palette_idx(tile_addr, attribute);

        let relative_y = (self.reg_v & Self::FINE_Y_MASK) >> 12;

        let pattern_idx = self.read_mem(tile_addr, cartridge);
        let bg_pattern_base_addr = self.ctrl_bg_pattern_table();

        let lo = self.read_mem(
            bg_pattern_base_addr + pattern_idx as u16 * PATTERN_SIZE + relative_y,
            cartridge,
        );
        let hi = self.read_mem(
            bg_pattern_base_addr
                + pattern_idx as u16 * PATTERN_SIZE
                + PATTERN_SIZE / 2
                + relative_y,
            cartridge,
        );
        let lo_bit = (lo & (1 << (7 - self.relative_x))) >> (7 - self.relative_x);
        let hi_bit = (hi & (1 << (7 - self.relative_x))) >> (7 - self.relative_x);
        let color_idx = (hi_bit << 1) | lo_bit;

        if color_idx == 0 {
            // The color is transparent.
            None
        } else {
            let color = self.read_mem(
                PALETTE_BASE_ADDR + (palette_idx as u16 * 4) + color_idx as u16,
                cartridge,
            );
            Some(PixelColor::from_nes_color(color, self.mask_grey_scale()))
        }
    }

    fn is_bg_transparent(&self, x: u16, y: u16) -> bool {
        self.bg_transparent[(y as usize) * NES_FRAME_WIDTH + (x as usize)]
    }

    fn set_bg_transparent(&mut self, x: u16, y: u16, transparent: bool) {
        self.bg_transparent[(y as usize) * NES_FRAME_WIDTH + (x as usize)] = transparent;
    }

    /// Render the background.
    pub fn render_bg(
        &mut self,
        cycles: usize,
        frame_buffer: &mut FrameBuffer,
        cartridge: &mut Cartridge,
    ) {
        let start_x = self.x;
        let start_y = self.y;
        for cycle_num in 0..cycles {
            if self.x == 0 {
                self.relative_x = self.reg_x as u16;
            }

            if self.x < NES_FRAME_WIDTH as u16 && self.y < NES_FRAME_HEIGHT as u16 {
                if self.mask_bg_visible() && (self.x >= 8 || self.mask_bg_visible_left8()) {
                    // Get the color of the background.
                    let color = self.get_bg_color(cartridge);

                    if let Some(color) = color {
                        // The background color is not transparent.
                        self.set_bg_transparent(self.x, self.y, false);

                        frame_buffer.set_chunk(self.x as usize, self.y as usize, color);
                    } else {
                        // The background color is transparent.
                        self.set_bg_transparent(self.x, self.y, true);

                        frame_buffer.set_chunk(self.x as usize, self.y as usize, UNDEF_COLOR);
                    }
                }

                self.relative_x += 1;
                if self.relative_x == 8 {
                    self.relative_x = 0;
                    self.coarse_x_inc();
                }
            }

            if self.x == NES_FRAME_WIDTH as u16 && self.y < NES_FRAME_HEIGHT as u16 {
                self.y_inc();
            }

            if self.x == NES_FRAME_WIDTH as u16 + 1 && self.y < NES_FRAME_HEIGHT as u16 {
                self.update_horizontal_v();
            }
            if 280 <= self.x && self.x <= 304 && self.y == NES_FRAME_HEIGHT as u16 + PPU_VBLANK - 1
            {
                self.update_vertical_v();
            }

            if self.x == 0 && self.y == NES_FRAME_HEIGHT as u16 {
                // Set VBLANK flag
                self.reg_status |= 0x80;

                if self.ctrl_nmi_enable() {
                    NESCPU::interrupt(InterruptType::NMI);
                }
            }

            self.x += 1;
            if self.x >= PPU_CYCLE {
                self.x = 0;
                self.y += 1;
            }
            if self.y >= NES_FRAME_HEIGHT as u16 + PPU_VBLANK {
                self.y = 0;

                // Clear VBLANK flag
                self.reg_status &= !0x80;

                // Clear sprite 0 hit flag
                self.reg_status &= !0x40;

                // This function assumes that it does not cross one frame.
                // So, if it reaches here, just restart it.
                self.render_bg(cycles - cycle_num - 1, frame_buffer, cartridge);
                return;
            }
        }

        let zero_hit = self.sprite_zero_hit(start_x, start_y, self.x, self.y, cartridge);
        if zero_hit {
            // Set sprite 0 hit flag
            self.reg_status |= 0x40;
        }
    }

    fn sprite_zero_hit(
        &self,
        start_x: u16,
        start_y: u16,
        end_x: u16,
        end_y: u16,
        cartridge: &mut Cartridge,
    ) -> bool {
        let sprite = &self.oam.sprites[0];

        let sprite_pattern_base_addr = self.ctrl_sprite_pattern_table();
        let pattern_idx = sprite.pattern_index;

        let top = sprite.y as u16 + 1;
        let left = sprite.x as u16;
        for y in top..top + self.ctrl_sprite_size() as u16 {
            if y >= NES_FRAME_HEIGHT as u16 {
                // Detect overflow.
                break;
            }

            // Calculate the relative Y position.
            let relative_y = if sprite.flip_vertical() {
                self.ctrl_sprite_size() as u16 - 1 + top - y
            } else {
                y - top
            };

            // Read pattern data.
            let lo = self.read_mem(
                sprite_pattern_base_addr + pattern_idx as u16 * PATTERN_SIZE + relative_y as u16,
                cartridge,
            );
            let hi = self.read_mem(
                sprite_pattern_base_addr
                    + pattern_idx as u16 * PATTERN_SIZE
                    + PATTERN_SIZE / 2
                    + relative_y as u16,
                cartridge,
            );

            for x in left..left + 8 {
                if x >= NES_FRAME_WIDTH as u16 {
                    // Detect overflow.
                    break;
                }

                if self.is_bg_transparent(x, y) {
                    // Sprite 0 hit is not occurred if the background is transparent.
                    continue;
                }

                if x < 8 && !self.mask_sprite_visible_left8() {
                    // Left 8 pixels of the screen are not visible.
                    continue;
                }

                // Calculate the relative X position.
                let relative_x = if sprite.flip_horizontal() {
                    7 + left - x
                } else {
                    x - left
                };

                // Read pattern data.
                let lo_bit = (lo & (1 << (7 - relative_x))) >> (7 - relative_x);
                let hi_bit = (hi & (1 << (7 - relative_x))) >> (7 - relative_x);

                let color_idx = (hi_bit << 1) | lo_bit;
                if color_idx != 0 {
                    // Sprite 0 hit is occurred.
                    let start = start_y as u32 * PPU_CYCLE as u32 + start_x as u32;
                    let current_pos = y as u32 * PPU_CYCLE as u32 + x as u32;
                    let end = end_y as u32 * PPU_CYCLE as u32 + end_x as u32;
                    if start <= current_pos && current_pos < end {
                        // Sprite 0 hit is occurred in the given range.
                        return true;
                    }
                }
            }
        }
        return false;
    }

    /// Render a sprite.
    fn render_sprite(
        &self,
        priority: usize,
        frame_buffer: &mut FrameBuffer,
        cartridge: &mut Cartridge,
    ) {
        let sprite = &self.oam.sprites[priority];

        let sprite_pattern_base_addr = self.ctrl_sprite_pattern_table();
        let pattern_idx = sprite.pattern_index;

        let top = sprite.y as u16 + 1;
        let left = sprite.x as u16;
        for y in top..top + self.ctrl_sprite_size() as u16 {
            if y >= NES_FRAME_HEIGHT as u16 {
                // Detect overflow.
                break;
            }

            // Calculate the relative Y position.
            let relative_y = if sprite.flip_vertical() {
                self.ctrl_sprite_size() as u16 - 1 + top - y
            } else {
                y - top
            };

            // Read pattern data.
            let lo = self.read_mem(
                sprite_pattern_base_addr + pattern_idx as u16 * PATTERN_SIZE + relative_y as u16,
                cartridge,
            );
            let hi = self.read_mem(
                sprite_pattern_base_addr
                    + pattern_idx as u16 * PATTERN_SIZE
                    + PATTERN_SIZE / 2
                    + relative_y as u16,
                cartridge,
            );

            for x in left..left + 8 {
                if x >= NES_FRAME_WIDTH as u16 {
                    // Detect overflow.
                    break;
                }

                if sprite.background() && !self.is_bg_transparent(x, y) {
                    // The background has priority.
                    continue;
                }

                if x < 8 && !self.mask_sprite_visible_left8() {
                    // Left 8 pixels of the screen are not visible.
                    continue;
                }

                // Calculate the relative X position.
                let relative_x = if sprite.flip_horizontal() {
                    7 + left - x
                } else {
                    x - left
                };

                // Read pattern data.
                let lo_bit = (lo & (1 << (7 - relative_x))) >> (7 - relative_x);
                let hi_bit = (hi & (1 << (7 - relative_x))) >> (7 - relative_x);

                let color_idx = (hi_bit << 1) | lo_bit;
                if color_idx != 0 {
                    let color = self.read_mem(
                        PALETTE_BASE_ADDR
                            + 0x10
                            + (sprite.palette_idx() as u16 * 4)
                            + color_idx as u16,
                        cartridge,
                    );
                    let color = PixelColor::from_nes_color(color, self.mask_grey_scale());

                    // Render the pixel.
                    frame_buffer.set_chunk(x as usize, y as usize, color);
                }
            }
        }
    }

    fn fill_undef(&mut self, frame_buffer: &mut FrameBuffer, cartridge: &mut Cartridge) {
        // Fill the undefined pixels with the background color.
        let color = PixelColor::from_nes_color(
            self.read_mem(PALETTE_BASE_ADDR, cartridge),
            self.mask_grey_scale(),
        );

        for y in 0..NES_FRAME_HEIGHT {
            for x in 0..NES_FRAME_WIDTH {
                if frame_buffer.get_chunk(x, y) == UNDEF_COLOR {
                    frame_buffer.set_chunk(x, y, color);
                }
            }
        }
    }

    pub fn complete_rendering(
        &mut self,
        frame_buffer: &mut FrameBuffer,
        cartridge: &mut Cartridge,
    ) {
        if self.mask_sprite_visible() {
            // Render sprites reversely.
            for priority in 0..64 {
                self.render_sprite(63 - priority, frame_buffer, cartridge);
            }
        }
        // Fill the undefined pixels with the background color.
        self.fill_undef(frame_buffer, cartridge);
    }
}

pub static NES_PPU: Lazy<RwLock<NESPPU>> = Lazy::new(|| {
    RwLock::new(NESPPU {
        reg_ctrl: 0,
        reg_mask: 0,
        reg_oam_addr: 0,
        reg_status: 0,
        reg_data: 0,
        reg_data_is_lo: false,

        reg_data_buffer: 0,

        reg_v: 0,
        reg_t: 0,
        reg_w: false,
        reg_x: 0,

        relative_x: 0,

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
        bg_transparent: vec![true; NES_FRAME_WIDTH * NES_FRAME_HEIGHT],
    })
});
