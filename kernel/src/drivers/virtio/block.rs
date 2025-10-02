use crate::{
    drivers::virtio::{VirtIODevice, VirtQ},
    mem::MemoryAllocator,
};

pub struct VirtBlockDevice<'a> {
    base_driver: VirtIODevice,
    queue: &'a mut VirtQ,
}

impl<'a> VirtBlockDevice<'a> {
    const VIRTIO_BLOCK_DEVICE_ID: u16 = 0x1001;

    pub fn new() -> Option<Self> {
        let base_driver = VirtIODevice::new(Self::VIRTIO_BLOCK_DEVICE_ID)?;

        let queue = MemoryAllocator::alloc::<VirtQ>();
        let queue = unsafe { queue.as_mut() }?;

        Some(Self { base_driver, queue })
    }

    pub fn init(&mut self) {
        self.base_driver.init();
        self.base_driver.init_queue(0, self.queue);
        self.base_driver.driver_ok();
    }
}
