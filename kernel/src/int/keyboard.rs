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
            match (key.state, key.code, keyboard.get_modifiers()) {
                (KeyState::Down, KeyCode::Tab, _) => {
                    let mut switcher = PROCESS_SWITCHER.write();
                    switcher.shift_proc(stack_frame);
                }
                (KeyState::Down, KeyCode::F1, _) => {
                    let mut switcher = PROCESS_SWITCHER.write();
                    switcher.switch_proc(ProcessMode::Game, stack_frame, false);
                }
                (KeyState::Down, KeyCode::F2, _) => {
                    let mut switcher = PROCESS_SWITCHER.write();
                    switcher.switch_proc(ProcessMode::Info, stack_frame, false);
                }
                (KeyState::Down, KeyCode::F3, _) => {
                    let mut switcher = PROCESS_SWITCHER.write();
                    switcher.switch_proc(ProcessMode::Log, stack_frame, false);
                }
                (state, KeyCode::L, _) => {
                    BKeyboard::on_pad_button(state, PadButton::A);
                }
                (state, KeyCode::K, _) => {
                    BKeyboard::on_pad_button(state, PadButton::B);
                }
                (state, KeyCode::W, _) => {
                    BKeyboard::on_pad_button(state, PadButton::Up);
                }
                (state, KeyCode::S, modifiers) => {
                    if modifiers.is_ctrl() {
                        if modifiers.is_shifted() {
                            // Save the working RAM
                            if let Err(_) = Serial::communicate(|serial| {
                                let cartridge = Cartridge::get();
                                let ram = cartridge.working_ram().ok_or(())?;
                                serial.save_ram(ram)
                            }) {
                                error!(
                                    COM,
                                    "Failed to save the working RAM. Possibly no working RAM."
                                );
                            } else {
                                info!(COM, "Saved the working RAM successfully.");
                            }
                        } else {
                            // Save the current state.
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
                    } else {
                        BKeyboard::on_pad_button(state, PadButton::Down);
                    }
                }
                (state, KeyCode::A, _) => {
                    BKeyboard::on_pad_button(state, PadButton::Left);
                }
                (state, KeyCode::D, _) => {
                    BKeyboard::on_pad_button(state, PadButton::Right);
                }
                (KeyState::Down, KeyCode::Z, modifiers) => {
                    if modifiers.is_ctrl() {
                        if modifiers.is_shifted() {
                            // Load the working RAM
                            if let Err(_) = Serial::communicate(|serial| {
                                let cartridge = Cartridge::get();
                                let ram = cartridge.working_ram().ok_or(())?;
                                serial.load_ram(ram)
                            }) {
                                error!(
                                    COM,
                                    "Failed to load the working RAM. Possibly no working RAM."
                                );
                            } else {
                                info!(COM, "Load the working RAM successfully.");
                            }
                        } else {
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
                    }
                }
                (state, KeyCode::Key1, _) => {
                    BKeyboard::on_pad_button(state, PadButton::Select);
                }
                (state, KeyCode::Key2, _) => {
                    BKeyboard::on_pad_button(state, PadButton::Start);
                }
                (KeyState::Down, KeyCode::ArrowUp, _) => {
                    Logger::scroll(-1);
                }
                (KeyState::Down, KeyCode::ArrowDown, _) => {
                    Logger::scroll(1);
                }
                (KeyState::Down, KeyCode::PageUp, _) => {
                    Logger::scroll(-0x100);
                }
                (KeyState::Down, KeyCode::PageDown, _) => {
                    Logger::scroll(0x100);
                }
                (KeyState::Down, KeyCode::ArrowLeft, _) => {
                    Logger::scroll(-0xFFFFFFF);
                }
                (KeyState::Down, KeyCode::ArrowRight, _) => {
                    Logger::reset_scroll();
                }
                (_, _, _) => {}
            }
            keyboard.process_keyevent(key);
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
