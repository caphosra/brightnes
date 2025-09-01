use spin::{Lazy, RwLock};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProcessMode {
    Log,
    Game,
    Info,
}

impl ProcessMode {
    pub fn shift(&self) -> ProcessMode {
        match self {
            ProcessMode::Log => ProcessMode::Game,
            ProcessMode::Game => ProcessMode::Info,
            ProcessMode::Info => ProcessMode::Log,
        }
    }
}

pub struct Process;

static CURRENT_PROC_MODE: Lazy<RwLock<(ProcessMode, bool)>> =
    Lazy::new(|| RwLock::new((ProcessMode::Log, false)));

impl Process {
    pub fn switch_proc(mode: ProcessMode) {
        let mut proc_mode = CURRENT_PROC_MODE.write();
        *proc_mode = (mode, true);
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
}
