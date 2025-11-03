use bitflags::bitflags;
use heapless::Vec;
use serde::{Deserialize, Serialize};
use spin::{Lazy, Once, RwLock};

use crate::mem::MemoryAllocator;
use crate::nes::ppu::color::NESColorConverter;
use crate::{
    critical,
    frame_buffer::{FrameBuffer, PixelColor},
    nes::{
        cartridge::Cartridge,
        cpu::{InterruptType, CPU},
        ppu::{bus::PPUBus, oam::OAM, vram::VRAM},
    },
};

pub const NES_FRAME_WIDTH: usize = 256;
pub const NES_FRAME_HEIGHT: usize = 240;
const NES_FRAME_TOTAL_SIZE: usize = NES_FRAME_WIDTH * NES_FRAME_HEIGHT;

pub static GAME_FB: Lazy<RwLock<FrameBuffer>> = Lazy::new(|| {
    let (width, height) = FrameBuffer::max_fb_size();

    let pixel_size = (width / NES_FRAME_WIDTH).min(height / NES_FRAME_HEIGHT);
    let offset_x = (width - pixel_size * NES_FRAME_WIDTH) / 2;
    let offset_y = (height - pixel_size * NES_FRAME_HEIGHT) / 2;

    RwLock::new(FrameBuffer::new(
        offset_x,
        offset_y,
        NES_FRAME_WIDTH,
        NES_FRAME_HEIGHT,
        pixel_size,
    ))
});

bitflags! {
    #[repr(transparent)]
    #[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PPUCtrl: u8 {
        const INCREMENT = 0b0000_0100;
        const SPRITE_PATTERN_TABLE = 0b0000_1000;
        const BG_PATTERN_TABLE = 0b0001_0000;
        const SPRITE_SIZE = 0b0010_0000;
        const MASTER_SLAVE = 0b0100_0000;
        const NMI_ENABLE = 0b1000_0000;
    }
}

bitflags! {
    #[repr(transparent)]
    #[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PPUMask: u8 {
        const GREY_SCALE = 0b0000_0001;
        const BG_VISIBLE_LEFT8 = 0b0000_0010;
        const SPRITE_VISIBLE_LEFT8 = 0b0000_0100;
        const BG_VISIBLE = 0b0000_1000;
        const SPRITE_VISIBLE = 0b0001_0000;
        const EMPHASIZE_RED = 0b0010_0000;
        const EMPHASIZE_GREEN = 0b0100_0000;
        const EMPHASIZE_BLUE = 0b1000_0000;
    }
}

bitflags! {
    #[repr(transparent)]
    #[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PPUStatus: u8 {
        const SPRITE_OVERFLOW = 0b0010_0000;
        const SPRITE0_HIT = 0b0100_0000;
        const VBLANK = 0b1000_0000;
    }
}

#[derive(Serialize, Deserialize)]
pub struct PPU {
    pub reg_ctrl: PPUCtrl,
    pub reg_mask: PPUMask,
    pub reg_oam_addr: u8,
    pub reg_status: PPUStatus,
    pub reg_data: u16,
    pub reg_data_is_lo: bool,

    reg_data_buffer: u8,

    pub reg_v: u16,
    pub reg_t: u16,
    pub reg_x: u8,
    reg_w: bool,

    relative_x: u16,

    pub x: u16,
    pub y: u16,

    vram: VRAM,
    pub oam: OAM,

    frame_counter: usize,

    sprite0_hit: Vec<bool, NES_FRAME_TOTAL_SIZE>,
    sprites_layer: Vec<Option<SpriteRequest>, NES_FRAME_TOTAL_SIZE>,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct SpriteRequest {
    pub priority: usize,
    pub color: u8,
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

static PPU_PTR: Lazy<Once<usize>> = Lazy::new(|| Once::new());

impl PPU {
    const COARSE_X_MASK: u16 = 0b00000000_00011111;
    const COARSE_Y_MASK: u16 = 0b00000011_11100000;
    const FINE_Y_MASK: u16 = 0b01110000_00000000;
    const NAME_TABLE_MASK: u16 = 0b00001100_00000000;

    const IRQ_CYCLE: u16 = 260;

    #[inline(never)]
    pub fn get() -> &'static mut Self {
        let ppu_raw_ptr = *PPU_PTR.call_once(|| {
            // Allocate memory for PPU.
            let ppu_raw_ptr = MemoryAllocator::alloc_zeroed::<PPU>();
            ppu_raw_ptr as usize
        }) as *mut PPU;
        unsafe { ppu_raw_ptr.as_mut() }.unwrap()
    }

    #[inline(never)]
    pub fn init(&mut self) {
        self.vram = VRAM::new();
        self.oam = OAM::new();
        self.sprite0_hit = Vec::from_array([false; NES_FRAME_WIDTH * NES_FRAME_HEIGHT]);
        self.sprites_layer = Vec::from_array([None; NES_FRAME_WIDTH * NES_FRAME_HEIGHT]);
    }

    pub fn read_reg(&mut self, addr: u16, cpu: &mut CPU, cartridge: &mut Cartridge) -> u8 {
        // Mirroring every 8 bytes.
        let addr = 0x2000 + ((addr - 0x2000) & 0x7);
        if addr == PPU_STATUS_ADDR {
            // PPU_STATUS
            self.reg_w = false;

            let prev_status = self.reg_status.bits();

            // Remove VBLANK flag and cancel NMI if it was set.
            self.reg_status.remove(PPUStatus::VBLANK);
            cpu.cancel_interrupt(InterruptType::NMI);

            prev_status
        } else if addr == PPU_DATA_ADDR {
            // PPU_DATA
            let data = self.reg_data_buffer;
            self.reg_data_buffer = PPUBus::read(self.reg_data, &self.vram, cartridge);

            self.reg_data = self.reg_data.wrapping_add(self.reg_ctrl.increment());

            data
        } else {
            critical!(PPU, "Invalid register reading: {:#06X}", addr);
        }
    }

    pub fn write_reg(&mut self, addr: u16, val: u8, cpu: &mut CPU, cartridge: &mut Cartridge) {
        if addr == OAM_DMA_ADDR {
            // OAM_DMA
            self.oam.request_dma_transfer(val, cpu);
            return;
        }

        // Mirroring every 8 bytes.
        let addr = 0x2000 + ((addr - 0x2000) & 0x7);
        if addr == PPU_CTRL_ADDR {
            // PPU_CTRL
            self.reg_ctrl = PPUCtrl::from_bits_retain(val);

            self.reg_t = (self.reg_t & !Self::NAME_TABLE_MASK) | (((val as u16) & 0x03) << 10);
        } else if addr == PPU_MASK_ADDR {
            // PPU_MASK
            self.reg_mask = PPUMask::from_bits_retain(val);

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
            PPUBus::write(addr, val, &mut self.vram, cartridge);

            self.reg_data = self.reg_data.wrapping_add(self.reg_ctrl.increment());
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
        if !self.reg_mask.contains(PPUMask::BG_VISIBLE)
            || (self.x < 8 && !self.reg_mask.contains(PPUMask::BG_VISIBLE_LEFT8))
        {
            // The background is not visible.
            return None;
        }

        let tile_addr = self.tile_addr();
        let attribute_addr = self.attribute_addr();

        let attribute = PPUBus::read(attribute_addr, &self.vram, cartridge);
        let palette_idx = self.get_palette_idx(tile_addr, attribute);

        let relative_y = (self.reg_v & Self::FINE_Y_MASK) >> 12;

        let pattern_idx = PPUBus::read(tile_addr, &self.vram, cartridge);
        let bg_pattern_base_addr = self.reg_ctrl.bg_pattern_table();

        let lo = PPUBus::read(
            bg_pattern_base_addr + pattern_idx as u16 * PATTERN_SIZE + relative_y,
            &self.vram,
            cartridge,
        );
        let hi = PPUBus::read(
            bg_pattern_base_addr
                + pattern_idx as u16 * PATTERN_SIZE
                + PATTERN_SIZE / 2
                + relative_y,
            &self.vram,
            cartridge,
        );
        let lo_bit = (lo & (1 << (7 - self.relative_x))) >> (7 - self.relative_x);
        let hi_bit = (hi & (1 << (7 - self.relative_x))) >> (7 - self.relative_x);
        let color_idx = (hi_bit << 1) | lo_bit;

        if color_idx == 0 {
            // The color is transparent.
            None
        } else {
            let color = PPUBus::read(
                PALETTE_BASE_ADDR + (palette_idx as u16 * 4) + color_idx as u16,
                &self.vram,
                cartridge,
            );
            Some(PixelColor::from_nes_color(
                color,
                self.reg_mask.contains(PPUMask::GREY_SCALE),
            ))
        }
    }

    /// Render the background.
    pub fn render_bg(
        &mut self,
        cycles: usize,
        frame_buffer: &mut FrameBuffer,
        cpu: &mut CPU,
        cartridge: &mut Cartridge,
    ) {
        let mut bg_color = None;

        for _ in 0..cycles {
            if self.x == 0 {
                self.relative_x = self.reg_x as u16;
            }

            if self.x < NES_FRAME_WIDTH as u16 && self.y < NES_FRAME_HEIGHT as u16 {
                let sprite_req =
                    &self.sprites_layer[self.y as usize * NES_FRAME_WIDTH + self.x as usize];

                if let Some(req) = sprite_req {
                    // A sprite can be visible.

                    if !req.background() {
                        // Sprite is over background.
                        let color = PixelColor::from_nes_color(
                            req.color,
                            self.reg_mask.contains(PPUMask::GREY_SCALE),
                        );
                        frame_buffer.set_chunk(self.x as usize, self.y as usize, color);

                        if self.sprite0_hit[self.y as usize * NES_FRAME_WIDTH + self.x as usize] {
                            // Sprite 0 hit can be occurred.
                            // To check it, we will calculate the background color even though it is not visible.

                            if self.get_bg_color(cartridge).is_some() {
                                // Sprite 0 hit is occurred.
                                self.reg_status.insert(PPUStatus::SPRITE0_HIT);
                            }
                        }
                    } else {
                        let color = self.get_bg_color(cartridge);
                        if let Some(color) = color {
                            // The background color is not transparent.
                            // Sprite is hidden.
                            frame_buffer.set_chunk(self.x as usize, self.y as usize, color);

                            if self.sprite0_hit[self.y as usize * NES_FRAME_WIDTH + self.x as usize]
                            {
                                // Sprite 0 hit is occurred.
                                self.reg_status.insert(PPUStatus::SPRITE0_HIT);
                            }
                        } else {
                            // The background color is transparent.
                            // Sprite is visible.
                            let color = PixelColor::from_nes_color(
                                req.color,
                                self.reg_mask.contains(PPUMask::GREY_SCALE),
                            );
                            frame_buffer.set_chunk(self.x as usize, self.y as usize, color);
                        }
                    }
                } else {
                    // No sprite is visible.

                    let color = self.get_bg_color(cartridge);
                    if let Some(color) = color {
                        // The background color is not transparent.
                        frame_buffer.set_chunk(self.x as usize, self.y as usize, color);
                    } else {
                        // Both background and sprite are transparent or not placed.
                        let bg_color = match bg_color {
                            Some(color) => color,
                            None => {
                                // It is not initialized.
                                let color = PixelColor::from_nes_color(
                                    PPUBus::read(PALETTE_BASE_ADDR, &self.vram, cartridge),
                                    self.reg_mask.contains(PPUMask::GREY_SCALE),
                                );
                                bg_color = Some(color);
                                color
                            }
                        };
                        frame_buffer.set_chunk(self.x as usize, self.y as usize, bg_color);
                    }
                }

                self.relative_x += 1;
                if self.relative_x == 8 {
                    self.relative_x = 0;
                    self.coarse_x_inc();
                }
            }

            if self.x == NES_FRAME_WIDTH as u16 && self.y < NES_FRAME_HEIGHT as u16 {
                // Calculate sprites on the next line.
                self.reflect_next_line_sprites(cartridge);

                self.y_inc();
            }

            if self.x == NES_FRAME_WIDTH as u16 + 1 && self.y < NES_FRAME_HEIGHT as u16 {
                self.update_horizontal_v();
            }
            if 280 <= self.x && self.x <= 304 && self.y == NES_FRAME_HEIGHT as u16 + PPU_VBLANK - 1
            {
                self.update_vertical_v();
            }

            if self.x == Self::IRQ_CYCLE && self.y < NES_FRAME_HEIGHT as u16 {
                if self.reg_ctrl.bg_pattern_table() != self.reg_ctrl.sprite_pattern_table()
                    && (self.reg_mask.contains(PPUMask::BG_VISIBLE)
                        || self.reg_mask.contains(PPUMask::SPRITE_VISIBLE))
                {
                    // https://www.nesdev.org/wiki/MMC3

                    // BG uses $0000, sprite uses $1000 or vice versa.

                    // Clock IRQ counter
                    cartridge.irq_clock(cpu);
                }
            }

            if self.x == 0 && self.y == NES_FRAME_HEIGHT as u16 {
                // Set VBLANK flag
                self.reg_status.insert(PPUStatus::VBLANK);

                if self.reg_ctrl.contains(PPUCtrl::NMI_ENABLE) {
                    cpu.interrupt(InterruptType::NMI);
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
                self.reg_status.remove(PPUStatus::VBLANK);

                // Clear sprite 0 hit flag
                self.reg_status.remove(PPUStatus::SPRITE0_HIT);
                self.clear_sprite0_hit_flags();

                // One frame is rendered.
                self.frame_counter += 1;

                // Initialize the background color.
                bg_color = None;
            }
        }
    }

    pub fn reflect_next_line_sprites(&mut self, cartridge: &mut Cartridge) {
        let target_y = self.y as u16 + 1;
        if target_y >= NES_FRAME_HEIGHT as u16 {
            // No need to reflect sprites.
            return;
        }

        // Clean up the next line.
        for x in 0..NES_FRAME_WIDTH {
            let target = &mut self.sprites_layer[target_y as usize * NES_FRAME_WIDTH + x as usize];
            if let Some(req) = target {
                if req.frame() < self.frame_counter {
                    // The request is from the previous frame. Just remove it.
                    *target = None;
                }
            }
        }

        if !self.reg_mask.contains(PPUMask::SPRITE_VISIBLE) {
            // Sprites are not visible.
            return;
        }

        for sprite_idx in 0..64 {
            let sprite = self.oam.sprites[sprite_idx];
            if sprite.y as u16 != self.y {
                // The sprite is not in the next line.
                continue;
            }

            let top = target_y;
            let left = sprite.x as u16;

            let pattern_idx = sprite.pattern_index;

            let sprite_pattern_base_addr = if self.reg_ctrl.sprite_size() == 16 {
                (pattern_idx as u16 & 1) * 0x1000
            } else {
                self.reg_ctrl.sprite_pattern_table()
            };

            for y in top..top + self.reg_ctrl.sprite_size() as u16 {
                if y >= NES_FRAME_HEIGHT as u16 {
                    // Detect overflow.
                    break;
                }

                // Calculate the relative Y position.
                let relative_y = if sprite.flip_vertical() {
                    self.reg_ctrl.sprite_size() as u16 - 1 + top - y
                } else {
                    y - top
                };

                // Read pattern data.
                let pattern_idx = if self.reg_ctrl.sprite_size() == 16 {
                    (pattern_idx & !0x1) | ((relative_y >= 8) as u8)
                } else {
                    pattern_idx
                };
                let lo = PPUBus::read(
                    sprite_pattern_base_addr
                        + pattern_idx as u16 * PATTERN_SIZE
                        + (relative_y & 0b111),
                    &self.vram,
                    cartridge,
                );
                let hi = PPUBus::read(
                    sprite_pattern_base_addr
                        + pattern_idx as u16 * PATTERN_SIZE
                        + (PATTERN_SIZE >> 1)
                        + (relative_y & 0b111),
                    &self.vram,
                    cartridge,
                );

                for x in left..left + 8 {
                    if x >= NES_FRAME_WIDTH as u16 {
                        // Detect overflow.
                        break;
                    }

                    if x < 8 && !self.reg_mask.contains(PPUMask::SPRITE_VISIBLE_LEFT8) {
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
                        // The pixel is not transparent.

                        if sprite_idx == 0 {
                            // Sprite 0 hit can happen here.
                            self.sprite0_hit[y as usize * NES_FRAME_WIDTH + x as usize] = true;
                        }

                        // Lookup the color.
                        let color = PPUBus::read(
                            PALETTE_BASE_ADDR
                                + 0x10
                                + (sprite.palette_idx() as u16 * 4)
                                + color_idx as u16,
                            &self.vram,
                            cartridge,
                        );

                        // Create a request.
                        let request = SpriteRequest::new(
                            self.frame_counter,
                            sprite_idx,
                            sprite.background(),
                            color,
                        );

                        // Try to overwrite the pixel.
                        let target =
                            &mut self.sprites_layer[y as usize * NES_FRAME_WIDTH + x as usize];
                        if target.is_none() || target.as_ref().unwrap().priority < request.priority
                        {
                            // The target has lower priority.
                            *target = Some(request);
                        }
                    }
                }
            }
        }
    }

    pub fn clear_sprite0_hit_flags(&mut self) {
        for v in self.sprite0_hit.iter_mut() {
            *v = false;
        }
    }
}

impl PPUCtrl {
    pub fn increment(&self) -> u16 {
        if self.contains(PPUCtrl::INCREMENT) {
            32
        } else {
            1
        }
    }

    pub fn sprite_pattern_table(&self) -> u16 {
        if self.contains(PPUCtrl::SPRITE_PATTERN_TABLE) {
            0x1000
        } else {
            0x0000
        }
    }

    pub fn bg_pattern_table(&self) -> u16 {
        if self.contains(PPUCtrl::BG_PATTERN_TABLE) {
            0x1000
        } else {
            0x0000
        }
    }

    pub fn sprite_size(&self) -> u8 {
        if self.contains(PPUCtrl::SPRITE_SIZE) {
            16
        } else {
            8
        }
    }
}

impl SpriteRequest {
    pub fn new(frame: usize, priority: usize, bg: bool, color: u8) -> Self {
        let priority = frame << 7 | (1 - (bg as usize)) << 6 | (63 - priority);
        SpriteRequest { priority, color }
    }

    pub fn background(&self) -> bool {
        ((self.priority >> 6) & 1) == 0
    }

    pub fn frame(&self) -> usize {
        self.priority >> 7
    }
}

pub mod bus;
pub mod color;
pub mod oam;
pub mod vram;
