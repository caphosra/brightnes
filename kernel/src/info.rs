use alloc::format;

use crate::{
    font::FONT_HEIGHT,
    frame_buffer::{FrameBuffer, PixelColor},
    nes::{
        cpu::{NESCPU, NES_CPU},
        pad::{Pad, PadButton, PADS},
    },
};

pub struct InfoProc;

const PADDING: usize = 10;

const PAD_WIDTH: usize = BUTTON_SIZE * 7 + PADDING * 6;
const PAD_HEIGHT: usize = BUTTON_SIZE * 3 + PADDING * 2;
const BUTTON_SIZE: usize = 10;

impl InfoProc {
    #[inline(always)]
    pub fn color_background(buffer: &FrameBuffer) -> PixelColor {
        buffer.make_color(0x20, 0x20, 0x20)
    }

    #[inline(always)]
    pub fn color_text(buffer: &FrameBuffer) -> PixelColor {
        buffer.make_color(0xFF, 0xFF, 0xFF)
    }

    #[inline(always)]
    pub fn color_pad_base(buffer: &FrameBuffer) -> PixelColor {
        buffer.make_color(0xF0, 0xF0, 0xF0)
    }

    #[inline(always)]
    pub fn color_pressed(buffer: &FrameBuffer) -> PixelColor {
        buffer.make_color(0xC2, 0x73, 0x19)
    }

    #[inline(always)]
    pub fn color_released(buffer: &FrameBuffer) -> PixelColor {
        buffer.make_color(0x10, 0x10, 0x10)
    }

    pub fn render_pad_base(buffer: &mut FrameBuffer, player: usize) {
        let offset_x = if player == 0 {
            PADDING
        } else {
            PADDING * 2 + PAD_WIDTH
        };
        let offset_y = PADDING;
        let color = InfoProc::color_pad_base(buffer);
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
            InfoProc::color_pressed(buffer)
        } else {
            InfoProc::color_released(buffer)
        };
        buffer.draw_rect(offset_x + x, offset_y + y, BUTTON_SIZE, BUTTON_SIZE, color);
    }

    pub fn render_pad(buffer: &mut FrameBuffer, player: usize, pad: &Pad) {
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

    pub fn render_cpu(buffer: &mut FrameBuffer, cpu: &NESCPU) {
        let offset_x = PADDING;
        let offset_y = PAD_HEIGHT + PADDING * 2;
        let color = InfoProc::color_text(buffer);
        let background = InfoProc::color_background(buffer);

        buffer.draw_text(
            offset_x,
            offset_y,
            format!("REG A: {:#04x}", cpu.reg_a).as_bytes(),
            color,
            background,
        );
        buffer.draw_text(
            offset_x,
            offset_y + FONT_HEIGHT as usize,
            format!("REG X: {:#04x}", cpu.reg_x).as_bytes(),
            color,
            background,
        );
        buffer.draw_text(
            offset_x,
            offset_y + FONT_HEIGHT as usize * 2,
            format!("REG Y: {:#04x}", cpu.reg_y).as_bytes(),
            color,
            background,
        );
        buffer.draw_text(
            offset_x,
            offset_y + FONT_HEIGHT as usize * 3,
            format!("REG PC: {:#06x}", cpu.reg_pc).as_bytes(),
            color,
            background,
        );
        buffer.draw_text(
            offset_x,
            offset_y + FONT_HEIGHT as usize * 4,
            format!("REG SP: {:#04x}", cpu.reg_sp).as_bytes(),
            color,
            background,
        );
    }

    pub fn render_all() {
        let buffer = FrameBuffer::get();

        // Clear the frame buffer.
        let background_color = InfoProc::color_background(buffer);
        let width = buffer.width;
        let height = buffer.height;
        buffer.draw_rect(0, 0, width, height, background_color);

        InfoProc::render_cpu(buffer, &NES_CPU.read());
        for player in 0..2 {
            InfoProc::render_pad(buffer, player as usize, &PADS.read()[player]);
        }
    }
}
