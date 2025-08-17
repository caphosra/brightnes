use core::mem::transmute;

use log::info;
use uefi::{
    boot::{AllocateType, MemoryType},
    proto::console::gop::{GraphicsOutput, PixelFormat},
};

const FRAME_BUFFER_ADDR: u64 = 0x700000;

#[repr(C)]
pub struct FrameBuffer {
    buffer: *mut u32,
    pub width: usize,
    pub height: usize,
    pub mode: PixelColorMode,
}

#[repr(C)]
pub enum PixelColorMode {
    Rgb = 0,
    Bgr = 1,
}

impl FrameBuffer {
    pub fn new() -> &'static mut Self {
        let frame_buffer = uefi::boot::allocate_pages(
            AllocateType::Address(FRAME_BUFFER_ADDR),
            MemoryType::BOOT_SERVICES_DATA,
            (size_of::<FrameBuffer>() + 0xfff) / 0x1000,
        )
        .unwrap();

        info!(
            "Allocated frame buffer info at: {:#x}",
            frame_buffer.as_ptr() as u64
        );

        unsafe {
            transmute::<_, *mut Self>(frame_buffer.as_ptr())
                .as_mut()
                .unwrap()
        }
    }

    pub fn init(&mut self, gop: &mut GraphicsOutput) {
        let mode_info = gop.current_mode_info();
        let (width, height) = mode_info.resolution();
        let mode = match mode_info.pixel_format() {
            PixelFormat::Rgb => PixelColorMode::Rgb,
            PixelFormat::Bgr => PixelColorMode::Bgr,
            _ => {
                panic!("Unsupported pixel format");
            }
        };

        assert_eq!(gop.frame_buffer().size(), width * height * 4);

        let buffer = gop.frame_buffer().as_mut_ptr() as *mut u32;
        self.buffer = buffer;
        self.width = width;
        self.height = height;
        self.mode = mode;

        info!("Frame buffer: {:#x}", self.buffer as u64);
    }
}
