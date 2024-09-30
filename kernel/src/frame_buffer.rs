use core::slice::from_raw_parts_mut;

#[repr(C)]
pub struct PixelColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    reserved: u8,
}

#[repr(C)]
pub struct FrameBuffer {
    buffer: &'static mut [PixelColor],
    pub width: usize,
    pub height: usize,
    pub mode: PixelColorMode,
}

#[repr(C)]
pub struct NativeFrameBuffer {
    pub buffer: usize,
    pub width: usize,
    pub height: usize,
    pub mode: PixelColorMode,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub enum PixelColorMode {
    Rgb,
    Bgr,
}

impl From<*mut NativeFrameBuffer> for FrameBuffer {
    fn from(native: *mut NativeFrameBuffer) -> Self {
        let native = unsafe { native.as_mut() }.unwrap();
        let buffer = unsafe { from_raw_parts_mut(native.buffer as *mut PixelColor, native.width * native.height) };

        FrameBuffer {
            buffer,
            width: native.width,
            height: native.height,
            mode: native.mode,
        }
    }
}

impl FrameBuffer {
    #[inline]
    pub fn set_pixel(&mut self, x: usize, y: usize, r: u8, g: u8, b: u8) {
        match self.mode {
            PixelColorMode::Rgb => {
                let color = &mut self.buffer[y * self.width + x];
                color.r = r;
                color.g = g;
                color.b = b;
            }
            PixelColorMode::Bgr => {
                let color = &mut self.buffer[y * self.width + x];
                color.r = b;
                color.g = g;
                color.b = r;
            }
        }
    }
}
