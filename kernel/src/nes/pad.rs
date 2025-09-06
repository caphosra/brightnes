use spin::{Lazy, RwLock};

use crate::{
    frame_buffer::RawFrameBuffer,
    info::InfoProc,
    proc::{Process, ProcessMode},
};

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum PadButton {
    A,
    B,
    Select,
    Start,
    Up,
    Down,
    Left,
    Right,
}

impl PadButton {
    fn from_u8(val: u8) -> PadButton {
        match val {
            0 => PadButton::A,
            1 => PadButton::B,
            2 => PadButton::Select,
            3 => PadButton::Start,
            4 => PadButton::Up,
            5 => PadButton::Down,
            6 => PadButton::Left,
            7 => PadButton::Right,
            _ => panic!("Unknown pad button detected."),
        }
    }
}

const PAD_BUTTON_LEN: usize = 8;

pub struct Pad {
    pub player: usize,
    pub pressed: [bool; PAD_BUTTON_LEN],
    pub selected: PadButton,
    pub strobe_enabled: bool,
}

pub static PADS: Lazy<RwLock<[Pad; 2]>> = Lazy::new(|| RwLock::new([Pad::new(0), Pad::new(1)]));

impl Pad {
    fn new(player: usize) -> Self {
        Pad {
            player,
            pressed: [false; PAD_BUTTON_LEN],
            selected: PadButton::A,
            strobe_enabled: false,
        }
    }

    pub fn press_button(&mut self, button: PadButton) {
        self.pressed[button as usize] = true;

        if Process::mode() == ProcessMode::Info {
            let buffer = RawFrameBuffer::get();
            InfoProc::render_button(buffer, self.player, self, button);
        }
    }

    pub fn release_button(&mut self, button: PadButton) {
        self.pressed[button as usize] = false;

        if Process::mode() == ProcessMode::Info {
            let buffer = RawFrameBuffer::get();
            InfoProc::render_button(buffer, self.player, self, button);
        }
    }

    pub fn read(&mut self) -> bool {
        let out = self.pressed[self.selected as usize];
        if !self.strobe_enabled {
            self.selected =
                PadButton::from_u8(((self.selected as usize + 1) % PAD_BUTTON_LEN) as u8);
        }
        out
    }

    pub fn write(&mut self, strobe: bool) {
        self.strobe_enabled = strobe;
        if strobe {
            self.selected = PadButton::A;
        }
    }
}
