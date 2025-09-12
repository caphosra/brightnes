use spin::{Lazy, RwLock};
use x86_64::{
    structures::idt::{InterruptStackFrame, InterruptStackFrameValue},
    VirtAddr,
};

use crate::info;

const STACK_SIZE: usize = 0x1_0000;
const PROC_SIZE: usize = 3;

///
/// Supports switching between multiple processes.
///
#[repr(C, align(16))]
pub struct ProcessSwitcher {
    stacks: [[u8; STACK_SIZE]; PROC_SIZE],
    entry_points: [VirtAddr; PROC_SIZE],
    saved_states: [Option<InterruptStackFrameValue>; PROC_SIZE],
    current_proc: ProcessMode,
}

pub static PROCESS_SWITCHER: Lazy<RwLock<ProcessSwitcher>> =
    Lazy::new(|| RwLock::new(ProcessSwitcher::new()));

impl ProcessSwitcher {
    pub fn new() -> Self {
        Self {
            stacks: [[0; STACK_SIZE]; PROC_SIZE],
            entry_points: [VirtAddr::zero(); PROC_SIZE],
            saved_states: [None; PROC_SIZE],
            current_proc: ProcessMode::Log,
        }
    }

    fn stack_top(&self, mode: ProcessMode) -> VirtAddr {
        let mode_idx: usize = mode.into();
        let stack = self.stacks[mode_idx];
        let stacl_top_raw = unsafe { stack.as_ptr().add(STACK_SIZE) };
        VirtAddr::from_ptr(stacl_top_raw)
    }

    pub fn switch_proc(
        &mut self,
        new_proc: ProcessMode,
        current_frame: &mut InterruptStackFrame,
        save_state: bool,
    ) {
        let old_mode_idx: usize = self.current_proc.into();
        let mode_idx: usize = new_proc.into();

        if old_mode_idx == mode_idx {
            // No need to switch.
            return;
        }

        if save_state {
            // Save the current state.
            let old_state = current_frame.clone();
            self.saved_states[old_mode_idx] = Some(old_state);
        }

        // Load the new state.
        let new_state = &mut self.saved_states[mode_idx];
        let new_state = match new_state {
            Some(state) => {
                // Restore the saved state.
                *state
            }
            None => {
                // Create a new state.
                let stack_top = self.stack_top(new_proc);
                let entry_point = self.entry_points[mode_idx];
                InterruptStackFrameValue::new(
                    entry_point,
                    current_frame.code_segment,
                    current_frame.cpu_flags,
                    stack_top,
                    current_frame.stack_segment,
                )
            }
        };

        // Switch to the new state.
        let mut current_frame = unsafe { current_frame.as_mut() };
        current_frame.write(new_state);

        self.current_proc = new_proc;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProcessMode {
    Log,
    Game,
    Info,
    Recovery,
}

impl From<ProcessMode> for usize {
    fn from(mode: ProcessMode) -> Self {
        match mode {
            ProcessMode::Log => 0,
            ProcessMode::Game => 1,
            ProcessMode::Info => 2,
            ProcessMode::Recovery => 3,
        }
    }
}

impl ProcessMode {
    pub fn shift(&self) -> ProcessMode {
        match self {
            ProcessMode::Log => ProcessMode::Game,
            ProcessMode::Game => ProcessMode::Info,
            ProcessMode::Info => ProcessMode::Log,
            ProcessMode::Recovery => ProcessMode::Recovery,
        }
    }
}

pub struct Process;

static CURRENT_PROC_MODE: Lazy<RwLock<(ProcessMode, bool)>> =
    Lazy::new(|| RwLock::new((ProcessMode::Log, false)));

impl Process {
    pub fn switch_proc(mode: ProcessMode) {
        let proc_mode = CURRENT_PROC_MODE.try_write();
        match proc_mode {
            Some(mut pm) => {
                if pm.0 != ProcessMode::Recovery {
                    *pm = (mode, true);
                }
            }
            None => {}
        }
    }

    pub fn shift_proc() {
        let (mode, _) = *CURRENT_PROC_MODE.read();
        Process::switch_proc(mode.shift());
    }

    pub fn mode() -> ProcessMode {
        let (mode, _) = *CURRENT_PROC_MODE.read();
        mode
    }

    pub fn status() -> (ProcessMode, bool) {
        *CURRENT_PROC_MODE.read()
    }

    pub fn mark_as_switched() {
        let mut proc_mode = CURRENT_PROC_MODE.write();
        *proc_mode = (proc_mode.0, false);
    }

    pub fn enter_recovery() {
        info!(SYS, "Entering recovery mode");

        Self::switch_proc(ProcessMode::Recovery);
    }
}
