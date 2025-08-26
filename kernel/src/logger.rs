use alloc::string::{String, ToString};
use alloc::vec::Vec;
use spin::{Lazy, RwLock};
use x86_64::instructions::interrupts;

use crate::font::{FONT_HEIGHT, FONT_WIDTH};
use crate::frame_buffer::FrameBuffer;

pub struct Logger {
    buffer: Vec<String>,
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => ($crate::logger::Logger::log(alloc::format!($($arg)*)));
}

static LOGGER: Lazy<RwLock<Logger>> = Lazy::new(|| RwLock::new(Logger { buffer: Vec::new() }));

impl Logger {
    fn log_internal(&mut self, text: String) {
        for line in text.split(|c| c == '\n') {
            if line.len() == 0 {
                self.buffer.push(line.to_string());
            } else {
                let width = FrameBuffer::get().width / FONT_WIDTH as usize;
                for chunk in line.as_bytes().chunks(width) {
                    self.buffer.push(String::from_utf8_lossy(chunk).to_string());
                }
            }
        }
    }

    fn render_internal(&mut self, rendered: usize, added: usize) {
        let buffer = FrameBuffer::get();
        let font_color = buffer.make_color(0xFF, 0xFF, 0xFF);

        let height = buffer.height / FONT_HEIGHT as usize;
        if added >= height {
            // Need to scroll the screen.
            for idx in 0..height {
                let text = self.buffer[idx - height + added].as_bytes();
                buffer.draw_text(0, idx * FONT_HEIGHT as usize, text, font_color);
            }
        } else {
            for idx in rendered..added {
                let text = self.buffer[idx].as_bytes();
                buffer.draw_text(0, idx * FONT_HEIGHT as usize, text, font_color);
            }
        }
    }

    pub fn log(text: String) {
        let int_enabled = interrupts::are_enabled();
        interrupts::disable();

        {
            let mut logger = LOGGER.write();

            let rendered = logger.buffer.len();
            logger.log_internal(text);
            let added = logger.buffer.len();
            logger.render_internal(rendered, added);
        }

        if int_enabled {
            interrupts::enable();
        }
    }
}
