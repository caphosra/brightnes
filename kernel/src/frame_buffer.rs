use core::mem::transmute;

use spin::RwLock;

pub type PixelColor = u32;

const FRAME_BUFFER_ADDR: u64 = 0x700000;

#[repr(C)]
pub struct FrameBuffer {
    buffer: u64,
    pub width: usize,
    pub height: usize,
    pub mode: PixelColorMode,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub enum PixelColorMode {
    Rgb = 0,
    Bgr = 1,
}

pub static FRAME_BUFFER: RwLock<FrameBuffer> = RwLock::new(FrameBuffer {
    buffer: 0,
    width: 0,
    height: 0,
    mode: PixelColorMode::Rgb,
});

impl FrameBuffer {
    pub fn init() {
        let original: &mut FrameBuffer = unsafe {
            transmute::<_, *mut Self>(FRAME_BUFFER_ADDR)
                .as_mut()
                .unwrap()
        };

        let mut frame_buffer = FRAME_BUFFER.write();
        frame_buffer.buffer = original.buffer as u64;
        frame_buffer.width = original.width;
        frame_buffer.height = original.height;
        frame_buffer.mode = original.mode;
    }

    #[inline(always)]
    pub fn make_color(&self, r: u8, g: u8, b: u8) -> PixelColor {
        if cfg!(target_endian = "little") {
            match self.mode {
                PixelColorMode::Rgb => r as u32 | ((g as u32) << 8) | ((b as u32) << 16),
                PixelColorMode::Bgr => b as u32 | ((g as u32) << 8) | ((r as u32) << 16),
            }
        } else {
            match self.mode {
                PixelColorMode::Rgb => ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8),
                PixelColorMode::Bgr => ((b as u32) << 24) | ((g as u32) << 16) | ((r as u32) << 8),
            }
        }
    }

    #[inline(always)]
    pub fn set_pixel(&mut self, x: usize, y: usize, color: PixelColor) {
        unsafe {
            let buffer: *mut u32 = transmute(self.buffer);
            buffer.add(y * self.width + x).write(color);
        }
    }
}
