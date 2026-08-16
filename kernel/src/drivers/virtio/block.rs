use core::ptr::{read_volatile, write_volatile};

use crate::{
    drivers::virtio::{VirtIODevice, VirtQ, VirtQDesc},
    error,
    mem::MemoryAllocator,
};

#[repr(C)]
struct BlockConfig {
    capacity: u64,
    size_max: u32,
    seg_max: u32,

    // Block geometry
    geometry_cylinders: u16,
    geometry_heads: u8,
    geometry_sectors: u8,
    blk_size: u32,

    // Block topology
    // The description of these fields are from VirtIO spec v1.3

    // # of logical blocks per physical block (log2)
    topology_physical_block_exp: u8,
    // offset of first aligned logical block
    topology_alignment_offset: u8,
    // suggested minimum I/O size in blocks
    topology_min_io_size: u16,
    // optimal (suggested maximum) I/O size in blocks
    topology_opt_io_size: u32,

    writeback: u8,
    unused0: u8,
    num_queues: u16,
    max_discard_sectors: u32,
    max_discard_seg: u32,
    discard_sector_alignment: u32,
    max_write_zeroes_sectors: u32,
    max_write_zeroes_seg: u32,
    write_zeroes_may_unmap: u8,
    unused1: [u8; 3],
    max_secure_erase_sectors: u32,
    max_secure_erase_seg: u32,
    secure_erase_sector_alignment: u32,
    zoned_zone_sectors: u32,
    zoned_max_open_zones: u32,
    zoned_max_active_zones: u32,
    zoned_max_append_sectors: u32,
    zoned_write_granularity: u32,
    zoned_model: u8,
    zoned_unused2: [u8; 3],
}

#[repr(C)]
struct BlockRequest {
    ty: BlockRequestType,
    reserved: u32,
    sector: u64,
}

#[repr(u32)]
#[allow(dead_code)]
enum BlockRequestType {
    In = 0,
    Out = 1,
    Flush = 4,
    GetID = 8,
    GetLifetime = 10,
    Discard = 11,
    WriteZeroes = 13,
    SecureErase = 14,
}

pub struct VirtBlockDevice<'a> {
    queue: &'a mut VirtQ,
    notify_addr: *mut u8,
    config: &'a mut BlockConfig,
}

enum BlockDeviceOperation {
    Read,
    Write,
}

impl<'a> VirtBlockDevice<'a> {
    pub const VIRTIO_BLOCK_DEVICE_ID: u16 = 0x1001;

    pub const DEFAULT_SECTOR_SIZE: u32 = 512;

    pub fn new() -> Option<Self> {
        let mut base_driver = VirtIODevice::new(Self::VIRTIO_BLOCK_DEVICE_ID)?;

        let config = unsafe { (base_driver.device_specific as *mut BlockConfig).as_mut() }?;

        let queue = MemoryAllocator::alloc::<VirtQ>();
        let queue = unsafe { queue.as_mut() }?;

        base_driver.init();
        let notify_addr = base_driver.init_queue(0, queue);
        base_driver.driver_ok();

        Some(Self {
            queue,
            notify_addr,
            config,
        })
    }

    pub fn sector_size(&self) -> u32 {
        if self.config.blk_size == 0 {
            Self::DEFAULT_SECTOR_SIZE
        } else {
            self.config.blk_size
        }
    }

    pub fn capacity(&self) -> u64 {
        self.config.capacity * self.sector_size() as u64
    }

    pub fn read(&mut self, sector: u64, data: *mut u8, len: u32) -> Result<(), ()> {
        self.request(sector, data, len, BlockDeviceOperation::Read)
    }

    pub fn write(&mut self, sector: u64, data: *const u8, len: u32) -> Result<(), ()> {
        self.request(sector, data as *mut u8, len, BlockDeviceOperation::Write)
    }

    fn request(
        &mut self,
        sector: u64,
        data: *mut u8,
        len: u32,
        op: BlockDeviceOperation,
    ) -> Result<(), ()> {
        if !len.is_multiple_of(self.sector_size()) {
            error!(DRV, "Length should be multiple of sector size.");
            return Err(());
        }

        let req = BlockRequest {
            ty: match op {
                BlockDeviceOperation::Read => BlockRequestType::In,
                BlockDeviceOperation::Write => BlockRequestType::Out,
            },
            reserved: 0,
            sector,
        };

        // Write the request to the descriptor table.
        unsafe {
            write_volatile(&mut self.queue.desc[0].addr, &req as *const _ as u64);
            write_volatile(
                &mut self.queue.desc[0].len,
                size_of::<BlockRequest>() as u32,
            );
            write_volatile(&mut self.queue.desc[0].flags, VirtQDesc::F_NEXT);
            write_volatile(&mut self.queue.desc[0].next, 1);
        }

        // Write the data buffer to the descriptor table.
        let flags = match op {
            BlockDeviceOperation::Read => VirtQDesc::F_NEXT | VirtQDesc::F_WRITE,
            BlockDeviceOperation::Write => VirtQDesc::F_NEXT,
        };
        unsafe {
            write_volatile(&mut self.queue.desc[1].addr, data as u64);
            write_volatile(&mut self.queue.desc[1].len, len);
            write_volatile(&mut self.queue.desc[1].flags, flags);
            write_volatile(&mut self.queue.desc[1].next, 2);
        }

        // Write the status byte to the descriptor table.
        let status: u8 = 0;
        unsafe {
            write_volatile(&mut self.queue.desc[2].addr, &status as *const _ as u64);
            write_volatile(&mut self.queue.desc[2].len, 1);
            write_volatile(&mut self.queue.desc[2].flags, VirtQDesc::F_WRITE);
            write_volatile(&mut self.queue.desc[2].next, 0);
        }

        self.queue.push(0, self.notify_addr);

        loop {
            // Wait until the device processes the request.
            let used_idx = unsafe { read_volatile(&self.queue.used.idx) };
            if used_idx != self.queue.last_used_idx {
                self.queue.last_used_idx = used_idx;
                break;
            }
        }

        let status = unsafe { read_volatile(&status) };
        if status != 0 {
            error!(DRV, "Block device request failed: status={}", status);
            Err(())
        } else {
            Ok(())
        }
    }
}
