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

static LOG_FB: Lazy<RwLock<FrameBuffer>> = Lazy::new(|| {
    let (width, height) = FrameBuffer::max_size();
    RwLock::new(FrameBuffer::new(0, 0, width, height, 1))
});

impl Logger {
    fn log_internal(&mut self, text: String) {
        for line in text.split(|c| c == '\n') {
            if line.len() == 0 {
                self.add_buffer(line.to_string());
            } else {
                let width = LOG_FB.read().text_width();
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
        let text_height = LOG_FB.read().text_height();
        if self.scroll < text_height {
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
        self.scroll = self.scroll.max(text_height);

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

        let mut buffer = LOG_FB.write();
        let font_color = FrameBuffer::make_color(0xFF, 0xFF, 0xFF);
        let bg_color = Logger::bg_color();
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

                // Flush the frame buffer.
                {
                    let mut buffer = LOG_FB.write();
                    buffer.flush(false);
                }
            }
        });
    }

    fn bg_color() -> PixelColor {
        FrameBuffer::make_color(0x20, 0x20, 0x20)
    }

    pub fn render_all() {
        // Clear the frame buffer.
        {
            let mut buffer = LOG_FB.write();
            buffer.clear(Logger::bg_color());
        }

        // Re-render the log.
        let mut logger = LOGGER.write();
        let scroll = logger.scroll;
        logger.render_internal(0, scroll);

        // Flush the frame buffer.
        {
            let mut buffer = LOG_FB.write();
            buffer.flush_all();
        }
    }
}
