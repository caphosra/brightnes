use spin::{Lazy, RwLock};
use x86_64::{
    structures::idt::{InterruptStackFrame, InterruptStackFrameValue},
    VirtAddr,
};

use crate::info;

const STACK_SIZE: usize = 0x1_0000;
const PROC_SIZE: usize = 3;

#[repr(C, align(16))]
pub struct ProcessInfo {
    pub dedicated_stack: [u8; STACK_SIZE],
    entry_point: fn() -> (),
    on_changed: Option<fn() -> ()>,
    pub saved_state: Option<InterruptStackFrameValue>,
}

impl ProcessInfo {
    pub fn new(entry_point: fn() -> (), on_changed: Option<fn() -> ()>) -> Self {
        Self {
            dedicated_stack: [0; STACK_SIZE],
            entry_point,
            on_changed,
            saved_state: None,
        }
    }

    pub fn stack_top(&self) -> VirtAddr {
        let stack_top_raw = unsafe { self.dedicated_stack.as_ptr().add(STACK_SIZE) };
        VirtAddr::from_ptr(stack_top_raw)
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
}

pub static PROCESS_SWITCHER: Lazy<RwLock<ProcessSwitcher>> =
    Lazy::new(|| RwLock::new(ProcessSwitcher::new()));

impl ProcessSwitcher {
    pub fn new() -> Self {
        Self {
            processes: [
                ProcessInfo::new(
                    || {},
                    Some(|| {
                        info!(SYS, "Switched to Log process.");
                    }),
                ),
                ProcessInfo::new(
                    || {},
                    Some(|| {
                        info!(SYS, "Switched to Game process.");
                    }),
                ),
                ProcessInfo::new(
                    || {},
                    Some(|| {
                        info!(SYS, "Switched to Info process.");
                    }),
                ),
            ],
            current_proc: ProcessMode::Log,
        }
    }

    fn stack_top(&self, mode: ProcessMode) -> VirtAddr {
        let mode_idx: usize = mode.into();
        let stack = self.processes[mode_idx].dedicated_stack;
        let stack_top_raw = unsafe { stack.as_ptr().add(STACK_SIZE) };
        VirtAddr::from_ptr(stack_top_raw)
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
            self.processes[old_mode_idx].saved_state = Some(old_state);
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
                let stack_top = self.stack_top(new_proc);
                let entry_point = self.processes[mode_idx].entry_point_addr();
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
