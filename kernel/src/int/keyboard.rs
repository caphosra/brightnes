use pc_keyboard::{layouts::Us104Key, HandleControl, KeyCode, KeyState, Keyboard, ScancodeSet1};
use spin::{Lazy, RwLock};
use x86_64::structures::idt::InterruptStackFrame;

use crate::{
    error,
    fs::FILE_SYSTEM,
    info,
    logger::Logger,
    nes::{
        apu::APU,
        cartridge::Cartridge,
        cpu::{InterruptType, CPU},
        pad::{PadButton, PADS},
        ppu::PPU,
    },
    proc::{ProcessMode, PROCESS_SWITCHER},
    system::SYSTEM,
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
                    switcher.switch_proc(ProcessMode::System, stack_frame, false);
                }
                (KeyState::Down, KeyCode::F2, _) => {
                    let mut switcher = PROCESS_SWITCHER.write();
                    switcher.switch_proc(ProcessMode::Game, stack_frame, false);
                }
                (KeyState::Down, KeyCode::F3, _) => {
                    let mut switcher = PROCESS_SWITCHER.write();
                    switcher.switch_proc(ProcessMode::Info, stack_frame, false);
                }
                (KeyState::Down, KeyCode::F4, _) => {
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
                            let sys = SYSTEM.read();
                            let mut fs = FILE_SYSTEM.write();
                            let cartridge = Cartridge::get();
                            match cartridge.working_ram().ok_or(()) {
                                Ok(ram) => match fs.save_ram(&sys, ram) {
                                    Ok(_) => info!(SYS, "Saved the working RAM successfully."),
                                    Err(_) => error!(
                                        SYS,
                                        "Failed to save the working RAM. Possibly no working RAM saved."
                                    ),
                                },
                                Err(_) => error!(
                                    SYS,
                                    "The cartridge has no working RAM."
                                ),
                            }
                        } else {
                            // Save the current state.
                            let sys = SYSTEM.read();
                            let mut fs = FILE_SYSTEM.write();
                            let cpu = CPU::get();
                            let ppu = PPU::get();
                            let cartridge = Cartridge::get();
                            match fs.save_state(&sys, cpu, ppu, cartridge) {
                                Ok(_) => info!(SYS, "Saved the current state successfully."),
                                Err(_) => error!(SYS, "Failed to save the current state."),
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
                            let sys = SYSTEM.read();
                            let mut fs = FILE_SYSTEM.write();
                            let cartridge = Cartridge::get();
                            match cartridge.working_ram().ok_or(()) {
                                Ok(ram) => match fs.load_ram(&sys, ram) {
                                    Ok(_) => info!(SYS, "Load the working RAM successfully."),
                                    Err(_) => error!(
                                        SYS,
                                        "Failed to load the working RAM. Possibly no working RAM saved."
                                    ),
                                },
                                Err(_) => error!(
                                    SYS,
                                    "The cartridge has no working RAM."
                                ),
                            }
                        } else {
                            // Load the latest state if requested.
                            let sys = SYSTEM.read();
                            let mut fs = FILE_SYSTEM.write();
                            let cpu = CPU::get();
                            let ppu = PPU::get();
                            let cartridge = Cartridge::get();
                            match fs.load_state(&sys, cpu, ppu, cartridge) {
                                Ok(_) => info!(SYS, "Loaded the current state successfully."),
                                Err(_) => error!(SYS, "Failed to load the current state."),
                            }

                            let mut switcher = PROCESS_SWITCHER.write();
                            switcher.reset_game(stack_frame);
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
                    let switcher = PROCESS_SWITCHER.read();
                    if switcher.mode() == ProcessMode::System {
                        let mut sys = SYSTEM.write();
                        sys.move_cursor_back();
                    } else {
                        Logger::scroll(-1);
                    }
                }
                (KeyState::Down, KeyCode::ArrowDown, _) => {
                    let switcher = PROCESS_SWITCHER.read();
                    if switcher.mode() == ProcessMode::System {
                        let mut sys = SYSTEM.write();
                        sys.move_cursor_forward();
                    } else {
                        Logger::scroll(1);
                    }
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
                (KeyState::Down, KeyCode::Return, _) => {
                    let mut switcher = PROCESS_SWITCHER.write();
                    if switcher.mode() == ProcessMode::System {
                        // Load the selected cartridge.
                        {
                            let mut sys = SYSTEM.write();
                            sys.load_selected_cartridge();
                        }

                        // Initialize NES.

                        let cpu = CPU::get();
                        cpu.init();

                        cpu.interrupt(InterruptType::RST);

                        let cartridge = Cartridge::get();
                        cartridge.init();

                        let ppu = PPU::get();
                        ppu.init();

                        let apu = APU::get();
                        apu.init();

                        // Load RAM if available.
                        {
                            let sys = SYSTEM.read();
                            if sys.has_ram() == Some(true) {
                                let mut fs = FILE_SYSTEM.write();
                                let cartridge = Cartridge::get();
                                match cartridge.working_ram().ok_or(()) {
                                    Ok(ram) => match fs.load_ram(&sys, ram) {
                                        Ok(_) => info!(SYS, "Load the working RAM successfully."),
                                        Err(_) => error!(
                                            SYS,
                                            "Failed to load the working RAM. Possibly no working RAM saved."
                                        ),
                                    },
                                    Err(_) => error!(
                                        SYS,
                                        "The cartridge has no working RAM."
                                    ),
                                }
                            }
                        }

                        // Start the game.
                        switcher.switch_proc(ProcessMode::Game, stack_frame, true);
                    }
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
