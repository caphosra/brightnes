pub type PixelColor = u32;

const FRAME_BUFFER_ADDR: u64 = 0x700000;

#[repr(C)]
pub struct FrameBuffer {
    buffer: *mut u32,
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

impl FrameBuffer {
    pub fn get() -> &'static mut Self {
        unsafe { (FRAME_BUFFER_ADDR as *mut Self).as_mut().unwrap() }
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
            self.buffer.add(y * self.width + x).write(color);
        }
    }
}
