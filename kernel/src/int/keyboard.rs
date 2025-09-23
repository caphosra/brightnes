use pc_keyboard::{layouts::Us104Key, HandleControl, KeyCode, KeyState, Keyboard, ScancodeSet1};
use spin::{Lazy, RwLock};
use x86_64::structures::idt::InterruptStackFrame;

use crate::{
    error, info,
    logger::Logger,
    nes::{
        cartridge::Cartridge,
        cpu::CPU,
        pad::{PadButton, PADS},
        ppu::PPU,
    },
    proc::{ProcessMode, PROCESS_SWITCHER},
    serial::Serial,
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
    pub fn on_event(key: u8, stack_frame: &mut InterruptStackFrame) {
        let mut keyboard = KEYBOARD.write();
        if let Ok(Some(key)) = keyboard.add_byte(key) {
            match (key.state, key.code) {
                (KeyState::Down, KeyCode::Tab) => {
                    let mut switcher = PROCESS_SWITCHER.write();
                    switcher.shift_proc(stack_frame);
                }
                (KeyState::Down, KeyCode::F1) => {
                    let mut switcher = PROCESS_SWITCHER.write();
                    switcher.switch_proc(ProcessMode::Game, stack_frame, false);
                }
                (KeyState::Down, KeyCode::F2) => {
                    let mut switcher = PROCESS_SWITCHER.write();
                    switcher.switch_proc(ProcessMode::Info, stack_frame, false);
                }
                (KeyState::Down, KeyCode::F3) => {
                    let mut switcher = PROCESS_SWITCHER.write();
                    switcher.switch_proc(ProcessMode::Log, stack_frame, false);
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
                (KeyState::Down, KeyCode::Backspace) => {
                    // Load the latest state if requested.
                    if let Err(_) = Serial::communicate(|serial| {
                        let cpu = CPU::get();
                        let ppu = PPU::get();
                        let cartridge = Cartridge::get();

                        serial.load_state(cpu, ppu, cartridge)
                    }) {
                        error!(COM, "Failed to load the latest state.");
                    } else {
                        info!(COM, "Loaded the latest saved state successfully.");
                    }

                    let mut switcher = PROCESS_SWITCHER.write();
                    switcher.reset_main(stack_frame);
                }
                (KeyState::Down, KeyCode::Return) => {
                    if let Err(_) = Serial::communicate(|serial| {
                        let cpu = CPU::get();
                        let ppu = PPU::get();
                        let cartridge = Cartridge::get();

                        serial.save_state(cpu, ppu, cartridge)
                    }) {
                        error!(COM, "Failed to save the current state.");
                    } else {
                        info!(COM, "Saved the current state successfully.");
                    }
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
