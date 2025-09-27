use serde::{Deserialize, Serialize};

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum SerialRequest {
    Active = 1,
    SaveState = 2,
    LoadState = 3,
    SaveRAM = 4,
    LoadRAM = 5,
    PlaySound = 7,
}

#[derive(Serialize, Deserialize)]
pub struct PulseRequest {
    pub active: bool,
    pub frequency: f64,
    pub volume: Volume,
    pub length: f64,
    pub duty_rate: f64,
    pub loop_enabled: bool,

    pub sweep_enabled: bool,
    pub sweep_interval: f64,
    pub sweep_negate: bool,
    pub sweep_shift: u8,
}

#[derive(Serialize, Deserialize)]
pub enum Volume {
    Constant(f64),
    Decreasing(f64),
}

#[derive(Serialize, Deserialize)]
pub enum APURequest {
    Pulse(usize, PulseRequest),
}
