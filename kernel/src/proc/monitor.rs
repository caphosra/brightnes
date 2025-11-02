use alloc::format;
use spin::{Lazy, RwLock};
use x86_64::instructions::{hlt, interrupts};

use crate::{
    font::FontManager,
    frame_buffer::{FrameBuffer, PixelColor},
    mem::{MemStatistics, HEAP_SIZE},
};

pub static MONITOR_FB: Lazy<RwLock<FrameBuffer>> = Lazy::new(|| {
    let (width, height) = FrameBuffer::max_fb_size();

    RwLock::new(FrameBuffer::new(0, 0, width, height, 1))
});

pub struct Monitor;

impl Monitor {
    const PADDING: usize = 10;
    const LINE_HEIGHT: usize = 50;
    const MESSAGE_PADDING: usize = 3;
    const TEXT_HEIGHT: usize = FontManager::FONT_HEIGHT as usize + Self::MESSAGE_PADDING * 2;

    pub fn render_bar(
        buffer: &mut FrameBuffer,
        in_use: usize,
        cached: usize,
        dead: usize,
        padding: usize,
    ) {
        let bar_width = buffer.pixel_width() - Self::PADDING * 2;

        let in_use_width = bar_width * in_use / HEAP_SIZE;
        let cached_width = bar_width * cached / HEAP_SIZE;
        let dead_width = bar_width * dead / HEAP_SIZE;
        let padding_width = bar_width * padding / HEAP_SIZE;

        let mut x = Self::PADDING;
        buffer.draw_rect(
            x,
            Self::PADDING,
            in_use_width,
            Self::LINE_HEIGHT,
            Self::in_use_color(),
        );
        x += in_use_width;

        buffer.draw_rect(
            x,
            Self::PADDING,
            cached_width,
            Self::LINE_HEIGHT,
            Self::cached_color(),
        );
        x += cached_width;

        buffer.draw_rect(
            x,
            Self::PADDING,
            dead_width,
            Self::LINE_HEIGHT,
            Self::dead_color(),
        );
        x += dead_width;

        buffer.draw_rect(
            x,
            Self::PADDING,
            padding_width,
            Self::LINE_HEIGHT,
            Self::padding_color(),
        );
        x += padding_width;

        buffer.draw_rect(
            x,
            Self::PADDING,
            bar_width - (x - Self::PADDING),
            Self::LINE_HEIGHT,
            Self::unused_color(),
        );
    }

    pub fn render_size(
        buffer: &mut FrameBuffer,
        y: usize,
        text: &str,
        size: usize,
        color: PixelColor,
        show_percentage: bool,
    ) {
        let text_height = FontManager::FONT_HEIGHT as usize;

        buffer.draw_rect(
            Self::PADDING,
            y + Self::MESSAGE_PADDING,
            text_height,
            text_height,
            color,
        );

        let percent = size as f64 / HEAP_SIZE as f64 * 100.0;
        let message = if show_percentage {
            format!("{}: {} bytes ({:.2} %)", text, size, percent)
        } else {
            format!("{}: {} bytes", text, size)
        };

        buffer.draw_text(
            Self::PADDING + text_height + Self::MESSAGE_PADDING,
            y + Self::MESSAGE_PADDING,
            message.as_bytes(),
            Self::font_color(),
            Self::bg_color(),
        );
    }

    pub fn render_all() {
        let mut buffer = MONITOR_FB.write();

        let in_use = MemStatistics::in_use();
        let cached = MemStatistics::cached();
        let dead = MemStatistics::dead();
        let padding = MemStatistics::padding();
        let reused = MemStatistics::reused_total();
        let unused = HEAP_SIZE - (in_use + cached + dead + padding);

        Self::render_bar(&mut buffer, in_use, cached, dead, padding);

        let mut y = Self::LINE_HEIGHT + Self::PADDING * 2;
        Self::render_size(&mut buffer, y, "Total", HEAP_SIZE, Self::bg_color(), false);
        y += Self::TEXT_HEIGHT;
        Self::render_size(&mut buffer, y, "In Use", in_use, Self::in_use_color(), true);
        y += Self::TEXT_HEIGHT;
        Self::render_size(&mut buffer, y, "Cached", cached, Self::cached_color(), true);
        y += Self::TEXT_HEIGHT;
        Self::render_size(&mut buffer, y, "Dead", dead, Self::dead_color(), true);
        y += Self::TEXT_HEIGHT;
        Self::render_size(
            &mut buffer,
            y,
            "Padding",
            padding,
            Self::padding_color(),
            true,
        );
        y += Self::TEXT_HEIGHT;
        Self::render_size(&mut buffer, y, "Unused", unused, Self::unused_color(), true);
        y += Self::TEXT_HEIGHT;
        Self::render_size(&mut buffer, y, "Reused", reused, Self::bg_color(), false);

        buffer.flush_all();
    }

    pub fn in_use_color() -> PixelColor {
        FrameBuffer::make_color(28, 255, 81)
    }

    pub fn cached_color() -> PixelColor {
        FrameBuffer::make_color(71, 166, 255)
    }

    pub fn dead_color() -> PixelColor {
        FrameBuffer::make_color(255, 149, 28)
    }

    pub fn padding_color() -> PixelColor {
        FrameBuffer::make_color(196, 211, 255)
    }

    pub fn unused_color() -> PixelColor {
        FrameBuffer::make_color(128, 128, 128)
    }

    pub fn font_color() -> PixelColor {
        FrameBuffer::make_color(255, 255, 255)
    }

    pub fn bg_color() -> PixelColor {
        FrameBuffer::make_color(0, 0, 0)
    }

    pub fn main() -> ! {
        loop {
            hlt();
        }
    }

    pub fn switched() {
        interrupts::without_interrupts(|| {
            Self::render_all();
        });
    }
}
