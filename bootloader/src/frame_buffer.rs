use uefi::proto::console::gop::{GraphicsOutput, PixelFormat};

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
    pub fn new(gop: &mut GraphicsOutput) -> Self {
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

        FrameBuffer {
            buffer,
            width: width as usize,
            height: height as usize,
            mode,
        }
    }
}
