use alloc::vec::Vec;
use spin::{Lazy, RwLock};

use crate::{
    frame_buffer::FrameBuffer,
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
    pub fn new() -> Self {
        System {
            running_cartridge: None,
            cartridges: Vec::new(),
            cursor: 0,
        }
    }

    pub fn update_cartridges(&mut self) {
        let fs = FILE_SYSTEM.write();
        self.cartridges = fs.cartridge_infos();
        self.cursor = 0;
    }

    pub fn render(&mut self) {}
}
