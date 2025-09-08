use alloc::format;
use spin::{Lazy, RwLock};

use crate::{
    font::FONT_HEIGHT,
    frame_buffer::{FrameBuffer, PixelColor},
    nes::{
        cartridge::{Cartridge, CARTRIDGE},
        cpu::{
            BRK_FLAG, CARRY_FLAG, DECIMAL_FLAG, INT_FLAG, NEG_FLAG, NESCPU, NES_CPU, OVERFLOW_FLAG,
            ZERO_FLAG,
        },
        instr::Instruction,
        pad::{Pad, PadButton, PADS},
        ppu::{NESPPU, NES_PPU},
    },
};

const PADDING: usize = 10;

const PAD_WIDTH: usize = BUTTON_SIZE * 7 + PADDING * 6;
const PAD_HEIGHT: usize = BUTTON_SIZE * 3 + PADDING * 2;
const BUTTON_SIZE: usize = 10;

const CPU_PPU_HEIGHT: usize = FONT_HEIGHT as usize * 8;
const REV_HEIGHT: usize = FONT_HEIGHT as usize * 8;

static PAD1_FB: Lazy<RwLock<FrameBuffer>> =
    Lazy::new(|| RwLock::new(FrameBuffer::new(PADDING, PADDING, PAD_WIDTH, PAD_HEIGHT, 1)));

static PAD2_FB: Lazy<RwLock<FrameBuffer>> = Lazy::new(|| {
    RwLock::new(FrameBuffer::new(
        PADDING * 2 + PAD_WIDTH,
        PADDING,
        PAD_WIDTH,
        PAD_HEIGHT,
        1,
    ))
});

static CPU_FB: Lazy<RwLock<FrameBuffer>> = Lazy::new(|| {
    let half_width = FrameBuffer::max_size().0 / 2;

    let offset_x = PADDING;
    let offset_y = PADDING * 2 + PAD_HEIGHT;
    let width = half_width - PADDING * 3 / 2;
    RwLock::new(FrameBuffer::new(
        offset_x,
        offset_y,
        width,
        CPU_PPU_HEIGHT,
        1,
    ))
});

static PPU_FB: Lazy<RwLock<FrameBuffer>> = Lazy::new(|| {
    let half_width = FrameBuffer::max_size().0 / 2;

    let offset_x = half_width + PADDING / 2;
    let offset_y = PADDING * 2 + PAD_HEIGHT;
    let width = half_width - PADDING * 3 / 2;
    RwLock::new(FrameBuffer::new(
        offset_x,
        offset_y,
        width,
        CPU_PPU_HEIGHT,
        1,
    ))
});

static REV_FB: Lazy<RwLock<FrameBuffer>> = Lazy::new(|| {
    let max_width = FrameBuffer::max_size().0;

    let offset_x = PADDING;
    let offset_y = PADDING * 3 + PAD_HEIGHT + CPU_PPU_HEIGHT;
    let width = max_width - PADDING * 2;
    RwLock::new(FrameBuffer::new(offset_x, offset_y, width, REV_HEIGHT, 1))
});

pub struct InfoProc;

impl InfoProc {
    pub fn get_pad_frame_buffer(player: usize) -> &'static RwLock<FrameBuffer> {
        match player {
            0 => &PAD1_FB,
            1 => &PAD2_FB,
            _ => panic!("Invalid player number for pad frame buffer."),
        }
    }

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

    fn render_pad_base(buffer: &mut FrameBuffer) {
        let color = InfoProc::color_pad_base();
        buffer.draw_rect(0, 0, PAD_WIDTH, PAD_HEIGHT, color);
    }

    pub fn render_button(buffer: &mut FrameBuffer, pad: &Pad, button: PadButton) {
        let x = match button {
            PadButton::A => BUTTON_SIZE * 6 + PADDING * 5,
            PadButton::B => BUTTON_SIZE * 5 + PADDING * 4,
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
        buffer.draw_rect(x, y, BUTTON_SIZE, BUTTON_SIZE, color);

        buffer.flush(false);
    }

    fn render_pad(buffer: &mut FrameBuffer, pad: &Pad) {
        InfoProc::render_pad_base(buffer);
        InfoProc::render_button(buffer, pad, PadButton::A);
        InfoProc::render_button(buffer, pad, PadButton::B);
        InfoProc::render_button(buffer, pad, PadButton::Select);
        InfoProc::render_button(buffer, pad, PadButton::Start);
        InfoProc::render_button(buffer, pad, PadButton::Up);
        InfoProc::render_button(buffer, pad, PadButton::Down);
        InfoProc::render_button(buffer, pad, PadButton::Left);
        InfoProc::render_button(buffer, pad, PadButton::Right);
    }

    fn render_cpu(buffer: &mut FrameBuffer, cpu: &NESCPU) {
        let color = InfoProc::color_text();
        let background = InfoProc::color_background();

        buffer.clear(Self::color_background());

        buffer.draw_text(
            0,
            0,
            format!("REG A: {:#04X}", cpu.reg_a).as_bytes(),
            color,
            background,
        );
        buffer.draw_text(
            0,
            FONT_HEIGHT as usize,
            format!("REG X: {:#04X}", cpu.reg_x).as_bytes(),
            color,
            background,
        );
        buffer.draw_text(
            0,
            FONT_HEIGHT as usize * 2,
            format!("REG Y: {:#04X}", cpu.reg_y).as_bytes(),
            color,
            background,
        );
        buffer.draw_text(
            0,
            FONT_HEIGHT as usize * 3,
            format!("REG PC: {:#06X}", cpu.reg_pc).as_bytes(),
            color,
            background,
        );
        buffer.draw_text(
            0,
            FONT_HEIGHT as usize * 4,
            format!("REG SP: {:#04X}", cpu.reg_sp).as_bytes(),
            color,
            background,
        );
        buffer.draw_text(
            0,
            FONT_HEIGHT as usize * 5,
            format!("REG P: {:#04X}", cpu.reg_p,).as_bytes(),
            color,
            background,
        );
        buffer.draw_text(
            0,
            FONT_HEIGHT as usize * 6,
            format!(
                "C: {:1}, Z: {:1}, I: {:1}, D: {:1}, B: {:1}{:1}, V: {:1}, N: {:1}",
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
            0,
            FONT_HEIGHT as usize * 7,
            format!("CYCLES: {:#018X}", cpu.cycles).as_bytes(),
            color,
            background,
        );
    }

    fn render_ppu(buffer: &mut FrameBuffer, _ppu: &NESPPU) {
        let _color = InfoProc::color_text();
        let _background = InfoProc::color_background();

        buffer.clear(Self::color_background());
    }

    fn render_reversing(buffer: &mut FrameBuffer, _cpu: &NESCPU, _cartridge: &mut Cartridge) {
        let _color = InfoProc::color_text();
        let _background = InfoProc::color_background();

        buffer.clear(Self::color_background());
    }

    pub fn render_all() {
        let mut buffer = CPU_FB.write();
        Self::render_cpu(&mut buffer, &NES_CPU.read());

        buffer.flush_all();

        let mut buffer = PPU_FB.write();
        Self::render_ppu(&mut buffer, &NES_PPU.read());

        // The background is already drawn.
        buffer.flush(true);

        let mut buffer = REV_FB.write();
        let mut cartridge = CARTRIDGE.write();
        Self::render_reversing(&mut buffer, &NES_CPU.read(), &mut cartridge);

        buffer.flush(true);

        // Render pads
        for player in 0..2 {
            let mut buffer = Self::get_pad_frame_buffer(player).write();
            Self::render_pad(&mut buffer, &PADS.read()[player]);

            buffer.flush(true);
        }
    }
}
