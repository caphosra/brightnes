use alloc::format;
use spin::{Lazy, RwLock};
use x86_64::instructions::interrupts;

use crate::{
    font::FontManager,
    frame_buffer::{FrameBuffer, PixelColor},
    nes::{
        cpu::{StatusFlags, CPU},
        pad::{Pad, PadButton, PADS},
        ppu::PPU,
    },
};

const PADDING: usize = 10;

const PAD_WIDTH: usize = BUTTON_SIZE * 7 + PADDING * 6;
const PAD_HEIGHT: usize = BUTTON_SIZE * 3 + PADDING * 2;
const BUTTON_SIZE: usize = 10;

const CPU_PPU_HEIGHT: usize = FontManager::FONT_HEIGHT as usize * 9;

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
    let half_width = FrameBuffer::max_fb_size().0 / 2;

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
    let half_width = FrameBuffer::max_fb_size().0 / 2;

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
    let max_width = FrameBuffer::max_fb_size().0;

    let offset_x = PADDING;
    let offset_y = PADDING * 3 + PAD_HEIGHT + CPU_PPU_HEIGHT;
    let width = max_width - PADDING * 2;
    let height = CPU::HISTORY_SIZE * FontManager::FONT_HEIGHT as usize;
    RwLock::new(FrameBuffer::new(offset_x, offset_y, width, height, 1))
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

    #[allow(unused_assignments)]
    fn render_cpu(buffer: &mut FrameBuffer, cpu: &CPU) {
        let color = InfoProc::color_text();
        let background = InfoProc::color_background();

        buffer.clear(Self::color_background());

        let mut pos = 0;
        macro_rules! draw_field {
            ($($arg:tt)*) => {
                buffer.draw_text(
                    0,
                    FontManager::FONT_HEIGHT as usize * pos,
                    format!($($arg)*).as_bytes(),
                    color,
                    background,
                );
                pos += 1;
            };
        }

        draw_field!("REG A:  {:#04X}", cpu.reg_a);
        draw_field!("REG X:  {:#04X}", cpu.reg_x);
        draw_field!("REG Y:  {:#04X}", cpu.reg_y);
        draw_field!("REG PC: {:#06X}", cpu.reg_pc);
        draw_field!("REG SP: {:#04X}", cpu.reg_sp);
        draw_field!("REG P:  {:#04X}", cpu.reg_p);
        draw_field!(
            "  C: {:1}, Z: {:1}, I: {:1}, D: {:1}",
            cpu.reg_p.contains(StatusFlags::CARRY),
            cpu.reg_p.contains(StatusFlags::ZERO),
            cpu.reg_p.contains(StatusFlags::INT),
            cpu.reg_p.contains(StatusFlags::DECIMAL),
        );
        draw_field!(
            "  B: {:1}, V: {:1}, N: {:1}",
            cpu.reg_p.contains(StatusFlags::BRK),
            cpu.reg_p.contains(StatusFlags::OVERFLOW),
            cpu.reg_p.contains(StatusFlags::NEG),
        );
        draw_field!("CYCLES: {:#018X}", cpu.cycles);
    }

    #[allow(unused_assignments)]
    fn render_ppu(buffer: &mut FrameBuffer, ppu: &PPU) {
        let color = InfoProc::color_text();
        let background = InfoProc::color_background();

        buffer.clear(Self::color_background());

        let mut pos = 0;
        macro_rules! draw_field {
            ($($arg:tt)*) => {
                buffer.draw_text(
                    0,
                    FontManager::FONT_HEIGHT as usize * pos,
                    format!($($arg)*).as_bytes(),
                    color,
                    background,
                );
                pos += 1;
            };
        }

        draw_field!("CTRL:   {:#010b}", ppu.reg_ctrl);
        draw_field!("MASK:   {:#010b}", ppu.reg_mask);
        draw_field!("OAM A:  {:#010b}", ppu.reg_oam_addr);
        draw_field!("STATUS: {:#010b}", ppu.reg_status);
        draw_field!("DATA:   {:#06X}", ppu.reg_data);
        draw_field!("REG V:  {:#06X}", ppu.reg_v);
        draw_field!("REG T:  {:#06X}", ppu.reg_t);
        draw_field!("REG X:  {:#04X}", ppu.reg_x);
        draw_field!("X, Y:   {:#06X}, {:#06X}", ppu.x, ppu.y);
    }

    #[allow(unused_assignments)]
    fn render_reversing(buffer: &mut FrameBuffer, cpu: &CPU) {
        let color = InfoProc::color_text();
        let background = InfoProc::color_background();

        buffer.clear(Self::color_background());

        let mut pos = 0;
        macro_rules! draw_field {
            ($($arg:tt)*) => {
                buffer.draw_text(
                    0,
                    FontManager::FONT_HEIGHT as usize * pos,
                    format!($($arg)*).as_bytes(),
                    color,
                    background,
                );
                pos += 1;
            };
        }

        cpu.history_summary(|inst| {
            draw_field!("{}", inst);
        });
    }

    pub fn render_all() {
        interrupts::without_interrupts(|| {
            let cpu = CPU::get();

            let mut buffer = CPU_FB.write();
            Self::render_cpu(&mut buffer, cpu);

            buffer.flush_all();

            let mut buffer = PPU_FB.write();
            Self::render_ppu(&mut buffer, &PPU::get());

            // The background is already drawn.
            buffer.flush(true);

            let mut buffer = REV_FB.write();
            Self::render_reversing(&mut buffer, cpu);

            buffer.flush(true);

            // Render pads
            for player in 0..2 {
                let mut buffer = Self::get_pad_frame_buffer(player).write();
                unsafe {
                    PADS.force_write_unlock();
                }
                Self::render_pad(&mut buffer, &PADS.read()[player]);

                buffer.flush(true);
            }
        });
    }
}
