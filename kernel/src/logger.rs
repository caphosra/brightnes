use alloc::string::{String, ToString};
use alloc::vec::Vec;
use spin::{Lazy, RwLock};
use x86_64::instructions::interrupts;

use crate::font::{FONT_HEIGHT, FONT_WIDTH};
use crate::frame_buffer::{FrameBuffer, PixelColor};
use crate::proc::{Process, ProcessMode};

pub struct Logger {
    buffer: Vec<String>,
    scroll: usize,
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => ($crate::logger::Logger::log(alloc::format!($($arg)*)));
}

static LOGGER: Lazy<RwLock<Logger>> = Lazy::new(|| {
    RwLock::new(Logger {
        buffer: Vec::new(),
        scroll: 0,
    })
});

impl Logger {
    fn log_internal(&mut self, text: String) {
        for line in text.split(|c| c == '\n') {
            if line.len() == 0 {
                self.add_buffer(line.to_string());
            } else {
                let width = FrameBuffer::get().text_width();
                for chunk in line.as_bytes().chunks(width) {
                    if self.buffer.len() == self.scroll {
                        self.scroll += 1;
                    }
                    self.add_buffer(String::from_utf8_lossy(chunk).to_string());
                }
            }
        }
    }

    fn add_buffer(&mut self, text: String) {
        if self.buffer.len() == self.scroll {
            self.scroll += 1;
        }
        self.buffer.push(text);
    }

    fn scroll_internal(&mut self, lines: i32) {
        // Cannot scroll if the screen is not fully filled.
        let buffer = FrameBuffer::get();
        if self.scroll < buffer.text_height() {
            return;
        }

        // Update the scroll position.
        let before = self.scroll;
        if lines > 0 {
            self.scroll = (self.scroll + lines as usize).min(self.buffer.len());
        } else if lines < 0 {
            self.scroll = self.scroll.saturating_sub((-lines) as usize);
        }

        // Cannot scroll beyond the screen height.
        self.scroll = self.scroll.max(buffer.text_height());

        // Rerender the screen.
        if Process::mode() == ProcessMode::Log {
            self.render_internal(before, self.scroll);
        }
    }

    pub fn scroll(lines: i32) {
        interrupts::without_interrupts(|| {
            let mut logger = LOGGER.write();
            logger.scroll_internal(lines);
        });
    }

    pub fn reset_scroll() {
        interrupts::without_interrupts(|| {
            let mut logger = LOGGER.write();
            let scroll = logger.buffer.len() - logger.scroll;
            logger.scroll_internal(scroll as i32);
        });
    }

    fn render_internal(&mut self, before: usize, after: usize) {
        // If nothing changed, do nothing.
        if before == after {
            return;
        }

        let buffer = FrameBuffer::get();
        let font_color = buffer.make_color(0xFF, 0xFF, 0xFF);
        let bg_color = Logger::bg_color(buffer);
        let height = buffer.text_height();

        if after < height {
            // The screen is not fully filled yet.
            assert!(before <= after);
            for idx in before..after {
                let text = self.buffer[idx].as_bytes();
                buffer.draw_text(0, idx * FONT_HEIGHT as usize, text, font_color, bg_color);
            }
        } else {
            for idx in 0..height {
                let text = self.buffer[idx + after - height].as_bytes();
                buffer.draw_text(0, idx * FONT_HEIGHT as usize, text, font_color, bg_color);
                if idx + before >= height {
                    let current_text_len = text.len();
                    let prev_text_len = self.buffer[idx + before - height].len();
                    if prev_text_len > current_text_len {
                        // Erase the previous text.
                        buffer.draw_rect(
                            current_text_len * FONT_WIDTH as usize,
                            idx * FONT_HEIGHT as usize,
                            (prev_text_len - current_text_len) * FONT_WIDTH as usize,
                            FONT_HEIGHT as usize,
                            bg_color,
                        );
                    }
                }
            }
        }
    }

    pub fn log(text: String) {
        interrupts::without_interrupts(|| {
            let mut logger = LOGGER.write();

            let before = logger.scroll;
            logger.log_internal(text);

            if Process::mode() == ProcessMode::Log {
                let after = logger.scroll;
                logger.render_internal(before, after);
            }
        });
    }

    fn bg_color(buffer: &FrameBuffer) -> PixelColor {
        buffer.make_color(0x20, 0x20, 0x20)
    }

    pub fn render_all() {
        let buffer = FrameBuffer::get();

        // Clear the frame buffer.
        buffer.clear(Logger::bg_color(buffer));

        // Re-render the log.
        let mut logger = LOGGER.write();
        let scroll = logger.scroll;
        logger.render_internal(0, scroll);
    }
}
