use spin::{Lazy, RwLock};
use x86_64::instructions::interrupts;

use crate::{info::InfoProc, logger::Logger};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProcessMode {
    Log,
    Game,
    Info,
}

pub struct Process;

static CURRENT_PROC_MODE: Lazy<RwLock<ProcessMode>> = Lazy::new(|| RwLock::new(ProcessMode::Log));

impl Process {
    pub fn switch_proc(mode: ProcessMode) {
        interrupts::without_interrupts(|| {
            let mut proc_mode = CURRENT_PROC_MODE.write();
            *proc_mode = mode;

            match mode {
                ProcessMode::Log => {
                    Logger::render_all();
                }
                ProcessMode::Game => {}
                ProcessMode::Info => {
                    InfoProc::render_all();
                }
            }
        });
    }

    pub fn mode() -> ProcessMode {
        *CURRENT_PROC_MODE.read()
    }
}
