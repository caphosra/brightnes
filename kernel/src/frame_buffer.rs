use core::ptr::copy_nonoverlapping;

use alloc::vec;
use alloc::vec::Vec;

use crate::font::{FontManager, FONT_HEIGHT, FONT_WIDTH};

pub type PixelColor = u32;

const FRAME_BUFFER_ADDR: u64 = 0x2_800_000;

const COLOR_BLACK: PixelColor = 0x0;

pub struct FrameBuffer {
    offset_x: usize,
    offset_y: usize,
    pub width: usize,
    pub height: usize,
    buffer: Vec<PixelColor>,
    dirty: Vec<bool>,
}

impl FrameBuffer {
    pub fn new(offset_x: usize, offset_y: usize, width: usize, height: usize) -> Self {
        FrameBuffer {
            offset_x,
            offset_y,
            width,
            height,
            buffer: vec![COLOR_BLACK; width * height],
            dirty: vec![true; height],
        }
    }

    #[inline(always)]
    pub fn set_pixel(&mut self, x: usize, y: usize, color: PixelColor) {
        assert!(x < self.width && y < self.height);

        let idx = y * self.width + x;
        if self.buffer[idx] != color {
            // Mark as dirty only if the color is changed
            self.buffer[idx] = color;
            self.dirty[y] = true;
        }
    }

    pub fn draw_rect(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        color: PixelColor,
    ) {
        for y_idx in y..(y + height) {
            for x_idx in x..(x + width) {
                self.set_pixel(x_idx, y_idx, color);
            }
        }
    }

    pub fn draw_glyph(
        &mut self,
        x: usize,
        y: usize,
        glyph: &'static [u8],
        color: PixelColor,
        background: PixelColor,
    ) {
        for y_idx in 0..0x10 {
            for x_idx in 0..8 {
                let color = if (glyph[y_idx] & (1 << (7 - x_idx))) != 0 {
                    color
                } else {
                    background
                };
                self.set_pixel(x + x_idx, y + y_idx, color);
            }
        }
    }

    pub fn draw_text(
        &mut self,
        x: usize,
        y: usize,
        text: &[u8],
        color: PixelColor,
        background: PixelColor,
    ) {
        for (i, &c) in text.iter().enumerate() {
            let glyph = FontManager::get_glyph_by_char(c);
            self.draw_glyph(x + i * 8, y, glyph, color, background);
        }
    }

    pub fn flush(&mut self, force: bool) {
        let raw_fb = RawFrameBuffer::get();
        let managed_fb = self.buffer.as_ptr();
        let mut idx = 0;
        for is_dirty in &mut self.dirty {
            if *is_dirty || force {
                let dst = (self.offset_y + idx) * raw_fb.width + self.offset_x;

                unsafe {
                    copy_nonoverlapping(
                        managed_fb.add(idx * self.width),
                        raw_fb.buffer.add(dst),
                        self.width,
                    );
                }
            }
            *is_dirty = false;
            idx += 1;
        }
    }

    pub fn flush_all(&mut self) {
        // Fill the entire raw frame buffer with black
        let raw_fb = RawFrameBuffer::get();
        unsafe {
            raw_fb
                .buffer
                .write_bytes(0x00, raw_fb.width * raw_fb.height);
        }

        self.flush(true);
    }
}

#[repr(C)]
pub struct RawFrameBuffer {
    pub buffer: *mut PixelColor,
    pub width: usize,
    pub height: usize,
    pub mode: PixelColorMode,
}

#[repr(C)]
pub enum PixelColorMode {
    #[allow(dead_code)]
    Rgb = 0,
    #[allow(dead_code)]
    Bgr = 1,
}

impl RawFrameBuffer {
    #[inline(always)]
    pub fn get_raw_ptr() -> *mut PixelColor {
        FRAME_BUFFER_ADDR as *mut PixelColor
    }

    #[inline(always)]
    pub fn render_sequence(dest: usize, src: *const PixelColor, len: usize) {
        unsafe {
            copy_nonoverlapping(src, Self::get_raw_ptr().add(dest), len);
        }
    }

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
    pub fn text_width(&self) -> usize {
        self.width / FONT_WIDTH as usize
    }

    #[inline(always)]
    pub fn text_height(&self) -> usize {
        self.height / FONT_HEIGHT as usize
    }

    #[inline(always)]
    pub fn set_pixel(&mut self, x: usize, y: usize, color: PixelColor) {
        unsafe {
            self.buffer.add(y * self.width + x).write(color);
        }
    }

    pub fn draw_rect(
        &mut self,
        offset_x: usize,
        offset_y: usize,
        width: usize,
        height: usize,
        color: PixelColor,
    ) {
        for y in offset_y..(offset_y + height) {
            for x in offset_x..(offset_x + width) {
                self.set_pixel(x, y, color);
            }
        }
    }

    pub fn clear(&mut self, color: PixelColor) {
        self.draw_rect(0, 0, self.width, self.height, color);
    }

    pub fn draw_glyph(
        &mut self,
        offset_x: usize,
        offset_y: usize,
        glyph: &'static [u8],
        color: PixelColor,
        background: PixelColor,
    ) {
        for y in 0..0x10 {
            for x in 0..8 {
                let color = if (glyph[y] & (1 << (7 - x))) != 0 {
                    color
                } else {
                    background
                };
                self.set_pixel(offset_x + x, offset_y + y, color);
            }
        }
    }

    pub fn draw_text(
        &mut self,
        offset_x: usize,
        offset_y: usize,
        text: &[u8],
        color: PixelColor,
        background: PixelColor,
    ) {
        for (i, &c) in text.iter().enumerate() {
            let glyph = FontManager::get_glyph_by_char(c);
            self.draw_glyph(offset_x + i * 8, offset_y, glyph, color, background);
        }
    }
}
