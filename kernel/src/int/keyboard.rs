use pc_keyboard::{layouts::Us104Key, HandleControl, KeyCode, KeyState, Keyboard, ScancodeSet1};
use spin::{Lazy, RwLock};

use crate::{
    logger::Logger,
    nes::pad::{PadButton, PADS},
    proc::{Process, ProcessMode},
};

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
                (KeyState::Down, KeyCode::F1) => {
                    Process::switch_proc(ProcessMode::Game);
                }
                (KeyState::Down, KeyCode::F2) => {
                    Process::switch_proc(ProcessMode::Info);
                }
                (KeyState::Down, KeyCode::F3) => {
                    Process::switch_proc(ProcessMode::Log);
                }
                (state, KeyCode::L) => {
                    BKeyboard::on_pad_button(state, PadButton::A);
                }
                (state, KeyCode::K) => {
                    BKeyboard::on_pad_button(state, PadButton::B);
                }
                (state, KeyCode::W) => {
                    BKeyboard::on_pad_button(state, PadButton::Up);
                }
                (state, KeyCode::S) => {
                    BKeyboard::on_pad_button(state, PadButton::Down);
                }
                (state, KeyCode::A) => {
                    BKeyboard::on_pad_button(state, PadButton::Left);
                }
                (state, KeyCode::D) => {
                    BKeyboard::on_pad_button(state, PadButton::Right);
                }
                (state, KeyCode::Key1) => {
                    BKeyboard::on_pad_button(state, PadButton::Select);
                }
                (state, KeyCode::Key2) => {
                    BKeyboard::on_pad_button(state, PadButton::Start);
                }
                (KeyState::Down, KeyCode::ArrowUp) => {
                    Logger::scroll(-1);
                }
                (KeyState::Down, KeyCode::ArrowDown) => {
                    Logger::scroll(1);
                }
                (KeyState::Down, KeyCode::PageUp) => {
                    Logger::scroll(-0x100);
                }
                (KeyState::Down, KeyCode::PageDown) => {
                    Logger::scroll(0x100);
                }
                (KeyState::Down, KeyCode::ArrowLeft) => {
                    Logger::scroll(-0xFFFFFFF);
                }
                (KeyState::Down, KeyCode::ArrowRight) => {
                    Logger::reset_scroll();
                }
                (_, _) => {}
            }
        }
    }

    pub fn on_pad_button(state: KeyState, button: PadButton) {
        let mut pads = PADS.write();
        match state {
            KeyState::Down => {
                pads[0].press_button(button);
            }
            KeyState::Up => {
                pads[0].release_button(button);
            }
            _ => {}
        }
    }
}
