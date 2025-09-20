use core::arch::asm;

use spin::{Lazy, RwLock};
use x86_64::{
    structures::idt::{InterruptStackFrame, InterruptStackFrameValue},
    VirtAddr,
};

use crate::{game_main, info_main, log_main, on_game_switched, on_info_switched, on_log_switched};

const PROC_SIZE: usize = 3;

const MAIN_STACK_BOTTOM: usize = 0x40_000_000;

const GAME_STACK_BOTTOM: *const u8 = 0x50_000_000 as *const u8;
const INFO_STACK_BOTTOM: *const u8 = 0x680_0000 as *const u8;
const LOG_STACK_BOTTOM: *const u8 = 0x700_0000 as *const u8;

#[repr(C, align(16))]
pub struct ProcessInfo {
    pub stack_bottom: VirtAddr,
    entry_point: fn() -> !,
    switched_handler: fn() -> (),
    pub saved_state: Option<InterruptStackFrameValue>,
}

impl ProcessInfo {
    pub fn new(
        stack_bottom: VirtAddr,
        entry_point: fn() -> !,
        switched_handler: fn() -> (),
    ) -> Self {
        Self {
            stack_bottom,
            entry_point,
            switched_handler,
            saved_state: None,
        }
    }

    pub fn entry_point_addr(&self) -> VirtAddr {
        VirtAddr::from_ptr(self.entry_point as *const ())
    }
}

///
/// Supports switching between multiple processes.
///
#[repr(C)]
pub struct ProcessSwitcher {
    processes: [ProcessInfo; PROC_SIZE],
    current_proc: ProcessMode,
    safe_mode: bool,
}

pub static PROCESS_SWITCHER: Lazy<RwLock<ProcessSwitcher>> =
    Lazy::new(|| RwLock::new(ProcessSwitcher::new()));

impl ProcessSwitcher {
    pub fn new() -> Self {
        Self {
            processes: [
                ProcessInfo::new(
                    VirtAddr::from_ptr(GAME_STACK_BOTTOM),
                    game_main,
                    on_game_switched,
                ),
                ProcessInfo::new(
                    VirtAddr::from_ptr(INFO_STACK_BOTTOM),
                    info_main,
                    on_info_switched,
                ),
                ProcessInfo::new(
                    VirtAddr::from_ptr(LOG_STACK_BOTTOM),
                    log_main,
                    on_log_switched,
                ),
            ],
            current_proc: ProcessMode::Game,
            safe_mode: false,
        }
    }

    #[inline(always)]
    pub fn reset_main_stack() {
        unsafe {
            asm!("mov rsp, {x}", x = const MAIN_STACK_BOTTOM);
        }
    }

    pub fn reset_main(&mut self, current_frame: &mut InterruptStackFrame) {
        let game_id = (ProcessMode::Game as u8) as usize;

        // Overwrite the saved state.
        self.processes[game_id].saved_state = Some(InterruptStackFrameValue::new(
            self.processes[game_id].entry_point_addr(),
            current_frame.code_segment,
            current_frame.cpu_flags,
            self.processes[game_id].stack_bottom,
            current_frame.stack_segment,
        ));

        // Switch to the game forcefully.
        // If you do without force, it will not switch if the current process is already the game.
        self.switch_proc(ProcessMode::Game, current_frame, true);
    }

    pub fn switch_proc(
        &mut self,
        new_proc: ProcessMode,
        current_frame: &mut InterruptStackFrame,
        reset_main: bool,
    ) {
        if self.safe_mode {
            // Do not allow switching processes in safety mode.
            return;
        }

        let old_mode_idx: usize = self.current_proc.into();
        let mode_idx: usize = new_proc.into();

        if old_mode_idx == mode_idx && !reset_main {
            // No need to switch.
            return;
        }

        // Save the current state.
        let old_state = current_frame.clone();
        self.processes[old_mode_idx].saved_state = Some(old_state);

        // Call on_changed handler.
        (self.processes[mode_idx].switched_handler)();

        if reset_main {
            // Reset the main stack frame.
            self.processes[mode_idx].saved_state = None;
        }

        // Load the new state.
        let new_state = &mut self.processes[mode_idx].saved_state;
        let new_state = match new_state {
            Some(state) => {
                // Restore the saved state.
                *state
            }
            None => {
                // Create a new state.
                let stack_bottom = self.processes[mode_idx].stack_bottom;
                let entry_point = self.processes[mode_idx].entry_point_addr();
                InterruptStackFrameValue::new(
                    entry_point,
                    current_frame.code_segment,
                    current_frame.cpu_flags,
                    stack_bottom,
                    current_frame.stack_segment,
                )
            }
        };

        // Switch to the new state.
        let mut current_frame = unsafe { current_frame.as_mut() };
        current_frame.write(new_state);

        self.current_proc = new_proc;
    }

    pub fn shift_proc(&mut self, current_frame: &mut InterruptStackFrame) {
        let new_proc = self.current_proc.shift();
        self.switch_proc(new_proc, current_frame, false);
    }

    pub fn enter_safe_mode(&mut self, current_frame: &mut InterruptStackFrame) {
        self.switch_proc(ProcessMode::Log, current_frame, false);
        self.safe_mode = true;
    }

    pub fn mode(&self) -> ProcessMode {
        self.current_proc
    }
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProcessMode {
    Game,
    Info,
    Log,
}

impl From<ProcessMode> for usize {
    fn from(mode: ProcessMode) -> Self {
        match mode {
            ProcessMode::Game => 0,
            ProcessMode::Info => 1,
            ProcessMode::Log => 2,
        }
    }
}

impl ProcessMode {
    pub fn shift(&self) -> ProcessMode {
        match self {
            ProcessMode::Game => ProcessMode::Info,
            ProcessMode::Info => ProcessMode::Log,
            ProcessMode::Log => ProcessMode::Game,
        }
    }
}
