use core::slice::from_raw_parts;

use fatfs::{IoBase, Read, Seek, SeekFrom, Write};
use heapless::Vec;

use crate::{
    drivers::virtio::{block::VirtBlockDevice, sound::VirtSoundDevice, VIRT_QUEUE_SIZE},
    warn,
};

pub struct BlockDeviceDriver<'a> {
    block_device: VirtBlockDevice<'a>,
    position: u64,
    sector_buffer: alloc::vec::Vec<u8>,
}

impl IoBase for BlockDeviceDriver<'_> {
    type Error = ();
}

impl Seek for BlockDeviceDriver<'_> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        match pos {
            SeekFrom::Start(offset) => {
                self.position = offset;
            }
            SeekFrom::End(offset) => {
                let capacity = self.block_device.capacity() as i64;
                self.position = (capacity + offset) as u64;
            }
            SeekFrom::Current(offset) => {
                self.position = ((self.position as i64) + offset) as u64;
            }
        }
        Ok(self.position)
    }
}

impl Read for BlockDeviceDriver<'_> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let end = self.position + (buf.len() as u64);

        let sector_size = self.block_device.sector_size() as u64;
        let start_sector = self.position / sector_size;
        let end_sector = (end + sector_size - 1) / sector_size;

        let buffer_size = (end_sector - start_sector) * sector_size;
        if self.sector_buffer.len() < buffer_size as usize {
            // Increase the buffer size.
            for _ in 0..buffer_size as usize - self.sector_buffer.len() {
                self.sector_buffer.push(0);
            }
        }

        // Read the device.
        self.block_device.read(
            start_sector,
            self.sector_buffer.as_mut_ptr(),
            buffer_size as u32,
        )?;

        // Copy to the buffer.
        let start_pos = (self.position % sector_size) as usize;
        buf.copy_from_slice(&self.sector_buffer[start_pos..start_pos + buf.len()]);

        self.seek(SeekFrom::Current(buf.len() as i64))?;

        Ok(buf.len())
    }
}

impl Write for BlockDeviceDriver<'_> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let end = self.position + (buf.len() as u64);

        let sector_size = self.block_device.sector_size() as u64;
        let start_sector = self.position / sector_size;
        let end_sector = (end + sector_size - 1) / sector_size;

        let buffer_size = (end_sector - start_sector) * sector_size;
        if self.sector_buffer.len() > buffer_size as usize {
            // Increase the buffer size.
            for _ in 0..self.sector_buffer.len() - buffer_size as usize {
                self.sector_buffer.push(0);
            }
        }

        // Read the device.
        // This process is required because we need to fill whole sectors.
        self.block_device.read(
            start_sector,
            self.sector_buffer.as_mut_ptr(),
            buffer_size as u32,
        )?;

        // Copy to the buffer.
        let start_pos = (self.position % sector_size) as usize;
        self.sector_buffer[start_pos..start_pos + buf.len()].copy_from_slice(buf);

        // Write them to the device.
        self.block_device.write(
            start_sector,
            self.sector_buffer.as_mut_ptr(),
            buffer_size as u32,
        )?;

        self.seek(SeekFrom::Current(buf.len() as i64))?;

        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<'a> BlockDeviceDriver<'a> {
    pub fn new() -> Self {
        let block_device = VirtBlockDevice::new().unwrap();
        BlockDeviceDriver {
            block_device,
            position: 0,
            sector_buffer: alloc::vec::Vec::new(),
        }
    }
}

pub struct SoundDeviceDriver<'a> {
    sound_device: VirtSoundDevice<'a>,
    sound_data:
        Vec<Vec<i16, { SoundDeviceDriver::SOUND_DATA_SIZE }>, { SoundDeviceDriver::MAX_REQ_SIZE }>,
    last_used: u16,
    last_sound_data: usize,
}

impl<'a> SoundDeviceDriver<'a> {
    pub const SAMPLING_RATE: u32 = 11025;
    pub const SOUND_DATA_SIZE: usize = VirtSoundDevice::PERIOD_BYTES as usize * 2;
    pub const MAX_REQ_SIZE: usize = VIRT_QUEUE_SIZE / 3 - 1;

    pub fn new() -> Self {
        let mut sound_device = VirtSoundDevice::new().unwrap();
        sound_device.set_params().unwrap();
        sound_device.prepare().unwrap();
        sound_device.start().unwrap();

        let mut sound_data = Vec::new();
        for _ in 0..Self::MAX_REQ_SIZE {
            sound_data.push(Vec::new());
        }
        SoundDeviceDriver {
            sound_device,
            sound_data,
            last_used: 0,
            last_sound_data: 0,
        }
    }

    fn queue_is_full(&self) -> bool {
        let consumed = self.sound_device.tx_consumed_count();
        let remains = self.last_used.wrapping_sub(consumed);
        remains as usize >= Self::MAX_REQ_SIZE
    }

    pub fn add_data(&mut self, left_data: i16, right_data: i16) {
        if self.queue_is_full() {
            warn!(DRV, "Sound device queue is full. Dropping data.");
            return;
        }
        self.sound_data[self.last_sound_data].push(left_data);
        self.sound_data[self.last_sound_data].push(right_data);

        // If the buffer is full, send it to the device.
        if self.sound_data[self.last_sound_data].len() >= Self::SOUND_DATA_SIZE {
            let raw_data = self.sound_data[self.last_sound_data].as_slice().as_ptr() as *const u8;
            let data =
                unsafe { from_raw_parts(raw_data, Self::SOUND_DATA_SIZE * size_of::<i16>()) };

            // Send the data to the device.
            self.sound_device.start();
            self.sound_device
                .write_stream(self.last_sound_data as u16, data);

            // Clear the buffer and move to the next one.
            self.sound_data[self.last_sound_data].clear();
            self.last_sound_data += 1;
            if self.last_sound_data >= Self::MAX_REQ_SIZE {
                self.last_sound_data = 0;
            }

            // Increment the used count.
            self.last_used = self.last_used.wrapping_add(1);
        }
    }
}

pub mod pci;
pub mod virtio;
