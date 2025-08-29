use spin::{Lazy, RwLock};

use crate::{info::InfoProc, logger::Logger, nes::ppu::NES_FRAME_BUFFER};

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

static CURRENT_PROC_MODE: Lazy<RwLock<ProcessMode>> = Lazy::new(|| RwLock::new(ProcessMode::Log));

impl Process {
    pub fn switch_proc(mode: ProcessMode) {
        let mut proc_mode = CURRENT_PROC_MODE.write();
        *proc_mode = mode;

        match mode {
            ProcessMode::Log => {
                Logger::render_all();
            }
            ProcessMode::Game => {
                let buffer = NES_FRAME_BUFFER.read();
                buffer.render_all();
            }
            ProcessMode::Info => {
                InfoProc::render_all();
            }
        };
    }

    pub fn shift_proc() {
        let mode = CURRENT_PROC_MODE.read().shift();
        Process::switch_proc(mode);
    }

    pub fn mode() -> ProcessMode {
        *CURRENT_PROC_MODE.read()
    }
}
