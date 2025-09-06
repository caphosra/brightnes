use alloc::format;
use spin::{Lazy, RwLock};

use crate::{
    font::FONT_HEIGHT,
    frame_buffer::{FrameBuffer, PixelColor},
    nes::{
        cpu::{
            BRK_FLAG, CARRY_FLAG, DECIMAL_FLAG, INT_FLAG, NEG_FLAG, NESCPU, NES_CPU, OVERFLOW_FLAG,
            ZERO_FLAG,
        },
        pad::{Pad, PadButton, PADS},
    },
};

pub static INFO_FB: Lazy<RwLock<FrameBuffer>> = Lazy::new(|| {
    let (width, height) = FrameBuffer::max_size();
    RwLock::new(FrameBuffer::new(0, 0, width, height, 1))
});

const PADDING: usize = 10;

const PAD_WIDTH: usize = BUTTON_SIZE * 7 + PADDING * 6;
const PAD_HEIGHT: usize = BUTTON_SIZE * 3 + PADDING * 2;
const BUTTON_SIZE: usize = 10;

pub struct InfoProc;

impl InfoProc {
    #[inline(always)]
    fn color_background() -> PixelColor {
        FrameBuffer::make_color(0x20, 0x20, 0x20)
    }

    #[inline(always)]
    fn color_text() -> PixelColor {
        FrameBuffer::make_color(0xFF, 0xFF, 0xFF)
    }

    #[inline(always)]
    fn color_pad_base() -> PixelColor {
        FrameBuffer::make_color(0xF0, 0xF0, 0xF0)
    }

    #[inline(always)]
    fn color_pressed() -> PixelColor {
        FrameBuffer::make_color(0xC2, 0x73, 0x19)
    }

    #[inline(always)]
    fn color_released() -> PixelColor {
        FrameBuffer::make_color(0x10, 0x10, 0x10)
    }

    fn render_pad_base(buffer: &mut FrameBuffer, player: usize) {
        let offset_x = if player == 0 {
            PADDING
        } else {
            PADDING * 2 + PAD_WIDTH
        };
        let offset_y = PADDING;
        let color = InfoProc::color_pad_base();
        buffer.draw_rect(offset_x, offset_y, PAD_WIDTH, PAD_HEIGHT, color);
    }

    pub fn render_button(buffer: &mut FrameBuffer, player: usize, pad: &Pad, button: PadButton) {
        let offset_x = if player == 0 {
            PADDING
        } else {
            PADDING * 2 + PAD_WIDTH
        };
        let offset_y = PADDING;

        let x = match button {
            PadButton::A => BUTTON_SIZE * 5 + PADDING * 4,
            PadButton::B => BUTTON_SIZE * 6 + PADDING * 5,
            PadButton::Select => BUTTON_SIZE * 3 + PADDING * 2,
            PadButton::Start => BUTTON_SIZE * 4 + PADDING * 3,
            PadButton::Up => BUTTON_SIZE + PADDING,
            PadButton::Down => BUTTON_SIZE + PADDING,
            PadButton::Left => PADDING,
            PadButton::Right => BUTTON_SIZE * 2 + PADDING,
        };
        let y = match button {
            PadButton::A => BUTTON_SIZE + PADDING,
            PadButton::B => BUTTON_SIZE + PADDING,
            PadButton::Select => BUTTON_SIZE * 2 + PADDING,
            PadButton::Start => BUTTON_SIZE * 2 + PADDING,
            PadButton::Up => PADDING,
            PadButton::Down => BUTTON_SIZE * 2 + PADDING,
            PadButton::Left => BUTTON_SIZE + PADDING,
            PadButton::Right => BUTTON_SIZE + PADDING,
        };
        let color = if pad.pressed[button as usize] {
            InfoProc::color_pressed()
        } else {
            InfoProc::color_released()
        };
        buffer.draw_rect(offset_x + x, offset_y + y, BUTTON_SIZE, BUTTON_SIZE, color);

        buffer.flush(false);
    }

    fn render_pad(buffer: &mut FrameBuffer, player: usize, pad: &Pad) {
        InfoProc::render_pad_base(buffer, player);
        InfoProc::render_button(buffer, player, pad, PadButton::A);
        InfoProc::render_button(buffer, player, pad, PadButton::B);
        InfoProc::render_button(buffer, player, pad, PadButton::Select);
        InfoProc::render_button(buffer, player, pad, PadButton::Start);
        InfoProc::render_button(buffer, player, pad, PadButton::Up);
        InfoProc::render_button(buffer, player, pad, PadButton::Down);
        InfoProc::render_button(buffer, player, pad, PadButton::Left);
        InfoProc::render_button(buffer, player, pad, PadButton::Right);
    }

    fn render_cpu(buffer: &mut FrameBuffer, cpu: &NESCPU) {
        let offset_x = PADDING;
        let offset_y = PAD_HEIGHT + PADDING * 2;
        let color = InfoProc::color_text();
        let background = InfoProc::color_background();

        buffer.draw_text(
            offset_x,
            offset_y,
            format!("REG A: {:#04X}", cpu.reg_a).as_bytes(),
            color,
            background,
        );
        buffer.draw_text(
            offset_x,
            offset_y + FONT_HEIGHT as usize,
            format!("REG X: {:#04X}", cpu.reg_x).as_bytes(),
            color,
            background,
        );
        buffer.draw_text(
            offset_x,
            offset_y + FONT_HEIGHT as usize * 2,
            format!("REG Y: {:#04X}", cpu.reg_y).as_bytes(),
            color,
            background,
        );
        buffer.draw_text(
            offset_x,
            offset_y + FONT_HEIGHT as usize * 3,
            format!("REG PC: {:#06X}", cpu.reg_pc).as_bytes(),
            color,
            background,
        );
        buffer.draw_text(
            offset_x,
            offset_y + FONT_HEIGHT as usize * 4,
            format!("REG SP: {:#04X}", cpu.reg_sp).as_bytes(),
            color,
            background,
        );
        buffer.draw_text(
            offset_x,
            offset_y + FONT_HEIGHT as usize * 5,
            format!(
                "REG P: {:#04X} (C: {:1}, Z: {:1}, I: {:1}, D: {:1}, B: {:1}{:1}, V: {:1}, N: {:1})",
                cpu.reg_p,
                cpu.get_flag(CARRY_FLAG),
                cpu.get_flag(ZERO_FLAG),
                cpu.get_flag(INT_FLAG),
                cpu.get_flag(DECIMAL_FLAG),
                cpu.get_flag(BRK_FLAG),
                cpu.get_flag(BRK_FLAG + 1),
                cpu.get_flag(OVERFLOW_FLAG),
                cpu.get_flag(NEG_FLAG),
            )
            .as_bytes(),
            color,
            background,
        );
        buffer.draw_text(
            offset_x,
            offset_y + FONT_HEIGHT as usize * 6,
            format!("CYCLES: {:#018X}", cpu.cycles).as_bytes(),
            color,
            background,
        );
    }

    pub fn render_all() {
        let mut buffer = INFO_FB.write();

        // Clear the frame buffer.
        let background_color = InfoProc::color_background();
        buffer.clear(background_color);

        InfoProc::render_cpu(&mut buffer, &NES_CPU.read());
        for player in 0..2 {
            InfoProc::render_pad(&mut buffer, player as usize, &PADS.read()[player]);
        }

        buffer.flush_all();
    }
}
