use alloc::format;
use alloc::vec::Vec;
use spin::{Lazy, RwLock};

use crate::{
    font::FontManager,
    frame_buffer::{FrameBuffer, PixelColor},
    fs::{CartridgeInfo, FILE_SYSTEM},
    nes::ppu::{NES_FRAME_HEIGHT, NES_FRAME_WIDTH},
};

pub static SYSTEM_FB: Lazy<RwLock<FrameBuffer>> = Lazy::new(|| {
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

pub static SYSTEM: Lazy<RwLock<System>> = Lazy::new(|| RwLock::new(System::new()));

pub struct System {
    running_cartridge: Option<CartridgeInfo>,
    cartridges: Vec<CartridgeInfo>,
    cursor: usize,
}

impl System {
    const CARTRIDGE_LIST_MAX: usize =
        NES_FRAME_HEIGHT as usize / FontManager::FONT_HEIGHT as usize - 3;

    pub fn new() -> Self {
        System {
            running_cartridge: None,
            cartridges: Vec::new(),
            cursor: 0,
        }
    }

    pub fn has_ram(&self) -> Option<bool> {
        if let Some(cart) = &self.running_cartridge {
            Some(cart.has_ram)
        } else {
            None
        }
    }

    pub fn running_cartridge_name(&self) -> Option<&str> {
        self.running_cartridge
            .as_ref()
            .map(|c| c.short_name.as_str())
    }

    pub fn game_initialized(&self) -> bool {
        self.running_cartridge.is_some()
    }

    pub fn update_cartridges(&mut self) {
        let fs = FILE_SYSTEM.write();
        self.cartridges = fs.cartridge_infos();
        self.cursor = 0;
    }

    pub fn load_selected_cartridge(&mut self) {
        if self.cartridges.len() == 0 {
            self.running_cartridge = None;
        } else {
            let cart = &self.cartridges[self.cursor];
            self.running_cartridge = Some(cart.clone());

            let mut fs = FILE_SYSTEM.write();
            fs.load_cartridge(&cart);
        }
    }

    pub fn move_cursor_forward(&mut self) {
        self.cursor += 1;
        if self.cursor >= self.cartridges.len() {
            self.cursor = 0;
        }
        self.render(false);
    }

    pub fn move_cursor_back(&mut self) {
        if self.cartridges.len() == 0 {
            self.cursor = 0;
            self.render(false);
        } else {
            if self.cursor == 0 {
                self.cursor = self.cartridges.len();
            }
            self.cursor -= 1;
            self.render(false);
        }
    }

    pub fn render(&mut self, clear: bool) {
        let mut fb = SYSTEM_FB.write();
        if clear {
            fb.clear(Self::bg_color());
        }

        let mut y = 0;
        match &self.running_cartridge {
            Some(cart) => {
                fb.draw_text(
                    0,
                    y,
                    format!("Running: {}", cart.long_name).as_bytes(),
                    Self::text_color(),
                    Self::bg_color(),
                );
            }
            None => {
                fb.draw_text(
                    0,
                    y,
                    b"Select a cartridge to run.",
                    Self::text_color(),
                    Self::bg_color(),
                );
            }
        }
        y += FontManager::FONT_HEIGHT as usize * 2;

        let selected_page = self.cursor / Self::CARTRIDGE_LIST_MAX;
        for idx in 0..Self::CARTRIDGE_LIST_MAX {
            let cart_idx = selected_page * Self::CARTRIDGE_LIST_MAX + idx;
            if cart_idx >= self.cartridges.len() {
                break;
            }
            let cart = &self.cartridges[cart_idx];

            const INDICATOR_SIZE: usize = 16;
            const INDICATOR_OFFSET: usize = 4;

            if cart.has_savedata {
                fb.draw_rect(
                    INDICATOR_OFFSET,
                    y + INDICATOR_OFFSET,
                    INDICATOR_SIZE - INDICATOR_OFFSET * 2,
                    INDICATOR_SIZE - INDICATOR_OFFSET * 2,
                    FrameBuffer::make_color(0x00, 0xFF, 0x00),
                );
            }

            if cart.has_ram {
                fb.draw_rect(
                    INDICATOR_SIZE + INDICATOR_OFFSET,
                    y + INDICATOR_OFFSET,
                    INDICATOR_SIZE - INDICATOR_OFFSET * 2,
                    INDICATOR_SIZE - INDICATOR_OFFSET * 2,
                    FrameBuffer::make_color(0x00, 0x00, 0xFF),
                );
            }

            let text_color = if cart_idx == self.cursor {
                Self::bg_color()
            } else {
                Self::text_color()
            };
            let bg_color = if cart_idx == self.cursor {
                Self::text_color()
            } else {
                Self::bg_color()
            };
            fb.draw_text(
                INDICATOR_SIZE * 2,
                y,
                format!("{:02}: {}", cart_idx + 1, cart.long_name).as_bytes(),
                text_color,
                bg_color,
            );
            y += FontManager::FONT_HEIGHT as usize;
        }
        fb.flush(false);
    }

    pub fn text_color() -> PixelColor {
        FrameBuffer::make_color(0xFF, 0xFF, 0xFF)
    }

    pub fn bg_color() -> PixelColor {
        FrameBuffer::make_color(0x00, 0x00, 0x00)
    }
}
