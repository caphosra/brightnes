use pc_keyboard::{layouts::Us104Key, HandleControl, KeyCode, KeyState, Keyboard, ScancodeSet1};
use spin::{Lazy, RwLock};

use crate::proc::Process;

pub struct BKeyboard;

static KEYBOARD: Lazy<RwLock<Keyboard<Us104Key, ScancodeSet1>>> = Lazy::new(|| {
    RwLock::new(Keyboard::new(
        ScancodeSet1::new(),
        Us104Key,
        HandleControl::Ignore,
    ))
});

impl BKeyboard {
    pub fn on_event(key: u8) {
        let mut keyboard = KEYBOARD.write();
        if let Ok(Some(key)) = keyboard.add_byte(key) {
            match (key.state, key.code) {
                (KeyState::Down, KeyCode::Tab) => {
                    Process::shift_proc();
                }
                (_, _) => {}
            }
        }
    }
}
