use spin::{Lazy, RwLock};

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
    pub pressed: [bool; PAD_BUTTON_LEN],
    pub selected: PadButton,
    pub strobe_enabled: bool,
}

pub static PADS: Lazy<RwLock<[Pad; 2]>> = Lazy::new(|| RwLock::new([Pad::new(), Pad::new()]));

impl Pad {
    fn new() -> Self {
        Pad {
            pressed: [false; PAD_BUTTON_LEN],
            selected: PadButton::A,
            strobe_enabled: false,
        }
    }

    pub fn press_button(&mut self, button: PadButton) {
        self.pressed[button as usize] = true;
    }

    pub fn release_button(&mut self, button: PadButton) {
        self.pressed[button as usize] = false;
    }

    pub fn read(&mut self) -> bool {
        let out = self.pressed[self.selected as usize];
        if !self.strobe_enabled {
            self.selected =
                PadButton::from_u8(((self.selected as usize + 1) % PAD_BUTTON_LEN) as u8);
        }
        out
    }
}
