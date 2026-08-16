use core::ptr::copy_nonoverlapping;

use alloc::vec;
use alloc::vec::Vec;

use crate::font::FontManager;

pub type PixelColor = u32;

const COLOR_BLACK: PixelColor = 0x0;
const FRAME_BUFFER_ADDR: u64 = 0x2_800_000;

#[repr(C)]
struct RawFrameBuffer {
    pub buffer: *mut PixelColor,
    pub width: usize,
    pub height: usize,
    pub mode: PixelColorMode,
}

#[repr(C)]
#[derive(Clone, Copy)]
enum PixelColorMode {
    #[allow(dead_code)]
    Rgb = 0,
    #[allow(dead_code)]
    Bgr = 1,
}

impl RawFrameBuffer {
    pub fn get() -> &'static mut Self {
        unsafe { (FRAME_BUFFER_ADDR as *mut Self).as_mut().unwrap() }
    }
}

pub struct FrameBuffer {
    offset_x: usize,
    offset_y: usize,
    pub width: usize,
    pub height: usize,

    /// The pixel buffer.
    ///
    /// Since we can copy it to the frame buffer efficiently if horizontal pixels
    /// are duplicated, its size is `(width * chunk_size) * height`.
    buffer: Vec<PixelColor>,

    /// Whether each line is dirty or not.
    dirty: Vec<bool>,
    chunk_size: usize,
}

impl FrameBuffer {
    pub fn new(
        offset_x: usize,
        offset_y: usize,
        width: usize,
        height: usize,
        chunk_size: usize,
    ) -> Self {
        FrameBuffer {
            offset_x,
            offset_y,
            width,
            height,
            buffer: vec![COLOR_BLACK; (width * chunk_size) * height],
            dirty: vec![true; height],
            chunk_size,
        }
    }

    #[inline(always)]
    pub fn make_color(r: u8, g: u8, b: u8) -> PixelColor {
        let mode = RawFrameBuffer::get().mode;
        if cfg!(target_endian = "little") {
            match mode {
                PixelColorMode::Rgb => r as u32 | ((g as u32) << 8) | ((b as u32) << 16),
                PixelColorMode::Bgr => b as u32 | ((g as u32) << 8) | ((r as u32) << 16),
            }
        } else {
            match mode {
                PixelColorMode::Rgb => ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8),
                PixelColorMode::Bgr => ((b as u32) << 24) | ((g as u32) << 16) | ((r as u32) << 8),
            }
        }
    }

    pub fn max_fb_size() -> (usize, usize) {
        let raw_fb = RawFrameBuffer::get();
        (raw_fb.width, raw_fb.height)
    }

    pub fn pixel_width(&self) -> usize {
        self.width * self.chunk_size
    }

    #[inline(always)]
    pub fn max_text_length(&self) -> usize {
        self.width / FontManager::FONT_WIDTH as usize
    }

    #[inline(always)]
    pub fn max_text_lines(&self) -> usize {
        self.height / FontManager::FONT_HEIGHT as usize
    }

    pub fn set_chunk(&mut self, x: usize, y: usize, color: PixelColor) {
        let start_pixel_x = x * self.chunk_size;
        let end_pixel_x = (x + 1) * self.chunk_size;

        if self.buffer[y * self.pixel_width() + start_pixel_x] != color {
            // Mark as dirty only if the color is changed.
            self.dirty[y] = true;

            for x in start_pixel_x..end_pixel_x {
                let idx = y * self.pixel_width() + x;
                self.buffer[idx] = color;
            }
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
                self.set_chunk(x_idx, y_idx, color);
            }
        }
    }

    pub fn clear(&mut self, color: PixelColor) {
        for y_idx in 0..self.height {
            for x_idx in 0..self.width {
                self.set_chunk(x_idx, y_idx, color);
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
        for (y_idx, g) in glyph
            .iter()
            .enumerate()
            .take(FontManager::FONT_HEIGHT as usize)
        {
            for x_idx in 0..FontManager::FONT_WIDTH as usize {
                let color = if (g & (1 << (7 - x_idx))) != 0 {
                    color
                } else {
                    background
                };
                self.set_chunk(x + x_idx, y + y_idx, color);
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
            let x = x + i * FontManager::FONT_WIDTH as usize;
            self.draw_glyph(x, y, glyph, color, background);
        }
    }

    pub fn flush(&mut self, force: bool) {
        let raw_fb = RawFrameBuffer::get();
        let managed_fb = self.buffer.as_ptr();

        for global_y in 0..self.height {
            if self.dirty[global_y] || force {
                for y in (global_y * self.chunk_size)..((global_y + 1) * self.chunk_size) {
                    let dst = (self.offset_y + y) * raw_fb.width + self.offset_x;
                    unsafe {
                        copy_nonoverlapping(
                            managed_fb.add(global_y * self.pixel_width()),
                            raw_fb.buffer.add(dst),
                            self.pixel_width(),
                        );
                    }
                }
            }
            self.dirty[global_y] = false;
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
