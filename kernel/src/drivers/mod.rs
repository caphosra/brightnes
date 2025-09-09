use alloc::boxed::Box;
use spin::{Lazy, RwLock};

pub trait DiskIODriver {
    fn read(&mut self, pos: usize, buffer: &mut [u8]) -> Result<(), ()>;
    fn write(&mut self, pos: usize, buffer: &[u8]) -> Result<(), ()>;
}

// TODO: Define functions
pub trait AudioDriver {
    fn play_sound(&mut self);
}

pub struct DummyDiskIODriver;

impl DiskIODriver for DummyDiskIODriver {
    fn read(&mut self, _pos: usize, _buffer: &mut [u8]) -> Result<(), ()> {
        Ok(())
    }
    fn write(&mut self, _pos: usize, _buffer: &[u8]) -> Result<(), ()> {
        Ok(())
    }
}

pub struct DummyAudioDriver;

impl AudioDriver for DummyAudioDriver {
    fn play_sound(&mut self) {}
}

pub static DISK_IO_DRIVER: Lazy<RwLock<Box<dyn DiskIODriver + Send + Sync>>> =
    Lazy::new(|| RwLock::new(Box::new(DummyDiskIODriver {})));

pub static AUDIO_DRIVER: Lazy<RwLock<Box<dyn AudioDriver + Send + Sync>>> =
    Lazy::new(|| RwLock::new(Box::new(DummyAudioDriver {})));

pub mod pci;
