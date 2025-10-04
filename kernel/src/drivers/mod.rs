use alloc::vec::Vec;
use fatfs::{IoBase, Read, Seek, SeekFrom, Write};

use crate::drivers::virtio::block::VirtBlockDevice;

pub struct BlockDeviceDriver<'a> {
    block_device: VirtBlockDevice<'a>,
    position: u64,
    sector_buffer: Vec<u8>,
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
            sector_buffer: Vec::new(),
        }
    }
}

pub mod pci;
pub mod virtio;
