use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use spin::{Lazy, RwLock};
use x86_64::instructions::interrupts;

use crate::font::{FONT_HEIGHT, FONT_WIDTH};
use crate::frame_buffer::{FrameBuffer, PixelColor};
use crate::proc::{ProcessMode, PROCESS_SWITCHER};

#[derive(Clone, Copy)]
pub enum LogLocation {
    SYS,
    DRV,
    CPU,
    PPU,
    APU,
    CAT,
    BUS,
}

impl LogLocation {
    pub fn to_str(&self) -> &str {
        match self {
            LogLocation::SYS => "SYS",
            LogLocation::DRV => "DRV",
            LogLocation::CPU => "CPU",
            LogLocation::PPU => "PPU",
            LogLocation::APU => "APU",
            LogLocation::CAT => "CAT",
            LogLocation::BUS => "BUS",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Log,
    Info,
    Warn,
    Error,
}

pub struct LogEntry {
    pub location: LogLocation,
    pub level: LogLevel,
    pub message: String,
}

pub struct Logger {
    buffer: Vec<LogEntry>,
    scroll: usize,
}

struct LoggerColor;

#[cfg(feature = "logging")]
#[macro_export]
macro_rules! log {
    ($loc:tt, $($arg:tt)*) => ($crate::logger::Logger::log($crate::logger::LogLocation::$loc, $crate::logger::LogLevel::Log, alloc::format!($($arg)*)));
}

#[cfg(not(feature = "logging"))]
#[macro_export]
macro_rules! log {
    ($loc:tt, $($arg:tt)*) => {};
}

#[macro_export]
macro_rules! info {
    ($loc:tt, $($arg:tt)*) => ($crate::logger::Logger::log($crate::logger::LogLocation::$loc, $crate::logger::LogLevel::Info, alloc::format!($($arg)*)));
}

#[macro_export]
macro_rules! warn {
    ($loc:tt, $($arg:tt)*) => ($crate::logger::Logger::log($crate::logger::LogLocation::$loc, $crate::logger::LogLevel::Warn, alloc::format!($($arg)*)));
}

#[macro_export]
macro_rules! error {
    ($loc:tt, $($arg:tt)*) => ($crate::logger::Logger::log($crate::logger::LogLocation::$loc, $crate::logger::LogLevel::Error, alloc::format!($($arg)*)));
}

#[macro_export]
macro_rules! critical {
    ($loc:tt, $($arg:tt)*) => {
        $crate::logger::Logger::log($crate::logger::LogLocation::$loc, $crate::logger::LogLevel::Error, alloc::format!($($arg)*));
        panic!("Critical error occurred.");
    }
}

static LOGGER: Lazy<RwLock<Logger>> = Lazy::new(|| {
    RwLock::new(Logger {
        buffer: Vec::new(),
        scroll: 0,
    })
});

pub static LOG_FB: Lazy<RwLock<FrameBuffer>> = Lazy::new(|| {
    let (width, height) = FrameBuffer::max_size();
    let mut frame_buffer = FrameBuffer::new(0, 0, width, height, 1);
    frame_buffer.clear(LoggerColor::bg_color());
    RwLock::new(frame_buffer)
});

impl Logger {
    const PREFIX_LEN: usize = 15;

    fn log_internal(&mut self, location: LogLocation, level: LogLevel, message: String) {
        for line in message.split(|c| c == '\n') {
            if line.len() == 0 {
                self.add_buffer(location, level, line.to_string());
            } else {
                // Wrap the line if it's too long.
                // The width is reduced by 5 taking the prefix into account.
                let width = LOG_FB.read().text_width() - Self::PREFIX_LEN;
                for chunk in line.as_bytes().chunks(width) {
                    if self.buffer.len() == self.scroll {
                        self.scroll += 1;
                    }
                    self.add_buffer(location, level, String::from_utf8_lossy(chunk).to_string());
                    self.send_serial(location, level, chunk);
                }
            }
        }
    }

    #[cfg(feature = "serial")]
    fn send_serial(&self, location: LogLocation, level: LogLevel, chunk: &[u8]) {
        use crate::serial::Serial;

        Serial::communicate(|handler| {
            handler.write(b"[");
            handler.write(location.to_str().as_bytes());
            match level {
                LogLevel::Log => handler.write(b"]  LOG: "),
                LogLevel::Info => handler.write(b"] INFO: "),
                LogLevel::Warn => handler.write(b"] WARN: "),
                LogLevel::Error => handler.write(b"]  ERR: "),
            }
            handler.write(chunk);
            handler.write(b"\n");
        });
    }

    #[cfg(not(feature = "serial"))]
    fn send_serial(&self, _location: LogLocation, _level: LogLevel, _chunk: &[u8]) {}

    fn add_buffer(&mut self, location: LogLocation, level: LogLevel, message: String) {
        if self.buffer.len() == self.scroll {
            self.scroll += 1;
        }
        self.buffer.push(LogEntry {
            location,
            level,
            message,
        });
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
        let switcher = PROCESS_SWITCHER.read();
        if switcher.mode() == ProcessMode::Log {
            self.render_internal(before, self.scroll);

            // Flush the frame buffer.
            {
                let mut buffer = LOG_FB.write();
                buffer.flush(false);
            }
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
        let prefix_color = LoggerColor::prefix_color();
        let fg_color = LoggerColor::fg_color();
        let bg_color = LoggerColor::bg_color();
        let height = buffer.text_height();

        if after < height {
            // The screen is not fully filled yet.
            assert!(before <= after);
            for idx in before..after {
                let entry = &self.buffer[idx];

                // Draw the prefix.
                buffer.draw_text(
                    0,
                    idx * FONT_HEIGHT as usize,
                    format!("{:08X} ", idx).as_bytes(),
                    prefix_color,
                    bg_color,
                );

                // Draw the prefix.
                buffer.draw_text(
                    9 * FONT_WIDTH as usize,
                    idx * FONT_HEIGHT as usize,
                    format!("[{}] ", entry.location.to_str()).as_bytes(),
                    LoggerColor::level_color(entry.level),
                    bg_color,
                );

                // Draw the content.
                let text = entry.message.as_bytes();
                buffer.draw_text(
                    Self::PREFIX_LEN * FONT_WIDTH as usize,
                    idx * FONT_HEIGHT as usize,
                    text,
                    if entry.level == LogLevel::Log {
                        LoggerColor::log_color()
                    } else {
                        fg_color
                    },
                    bg_color,
                );
            }
        } else {
            for idx in 0..height {
                let entry = &self.buffer[idx + after - height];

                // Draw the prefix.
                buffer.draw_text(
                    0,
                    idx * FONT_HEIGHT as usize,
                    format!("{:08X} ", idx + after - height).as_bytes(),
                    prefix_color,
                    bg_color,
                );

                // Draw the prefix.
                buffer.draw_text(
                    9 * FONT_WIDTH as usize,
                    idx * FONT_HEIGHT as usize,
                    format!("[{}] ", entry.location.to_str()).as_bytes(),
                    LoggerColor::level_color(entry.level),
                    bg_color,
                );

                // Draw the content.
                let text = entry.message.as_bytes();
                buffer.draw_text(
                    Self::PREFIX_LEN * FONT_WIDTH as usize,
                    idx * FONT_HEIGHT as usize,
                    text,
                    if entry.level == LogLevel::Log {
                        LoggerColor::log_color()
                    } else {
                        fg_color
                    },
                    bg_color,
                );

                if idx + before >= height {
                    let current_text_len = text.len() + Self::PREFIX_LEN;
                    let prev_text_len =
                        self.buffer[idx + before - height].message.len() + Self::PREFIX_LEN;
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

    pub fn log(location: LogLocation, level: LogLevel, message: String) {
        interrupts::without_interrupts(|| {
            let mut logger = LOGGER.write();

            let before = logger.scroll;
            logger.log_internal(location, level, message);

            let after = logger.scroll;
            logger.render_internal(before, after);

            let switcher = PROCESS_SWITCHER.read();
            if switcher.mode() == ProcessMode::Log {
                // Flush the frame buffer.
                let mut buffer = LOG_FB.write();
                buffer.flush(false);
            }
        });
    }
}

impl LoggerColor {
    pub fn bg_color() -> PixelColor {
        FrameBuffer::make_color(0x20, 0x20, 0x20)
    }

    pub fn fg_color() -> PixelColor {
        FrameBuffer::make_color(0xFF, 0xFF, 0xFF)
    }

    pub fn prefix_color() -> PixelColor {
        FrameBuffer::make_color(0xA0, 0xA0, 0xA0)
    }

    pub fn log_color() -> PixelColor {
        FrameBuffer::make_color(0xC0, 0xC0, 0xC0)
    }

    pub fn warn_color() -> PixelColor {
        FrameBuffer::make_color(0xFF, 0xFF, 0x00)
    }

    pub fn error_color() -> PixelColor {
        FrameBuffer::make_color(0xFF, 0x00, 0x00)
    }

    pub fn level_color(level: LogLevel) -> PixelColor {
        match level {
            LogLevel::Log => Self::log_color(),
            LogLevel::Info => Self::fg_color(),
            LogLevel::Warn => Self::warn_color(),
            LogLevel::Error => Self::error_color(),
        }
    }
}
