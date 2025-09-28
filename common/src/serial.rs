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
    pub volume: f64,
    pub duty_rate: f64,
}

#[derive(Serialize, Deserialize)]
pub struct TriangleRequest {
    pub active: bool,
    pub frequency: f64,
    pub length: f64,
}

#[derive(Serialize, Deserialize)]
pub enum APURequest {
    Pulse(usize, PulseRequest),
    Triangle(TriangleRequest),
}
