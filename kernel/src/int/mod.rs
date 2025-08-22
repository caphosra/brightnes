use x86_64::instructions::hlt;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use crate::frame_buffer::FrameBuffer;
use crate::int::pic::PIC;

pub struct Interrupt;

pub const INT_TIMER: u8 = 0x20;
pub const INT_KEYBOARD: u8 = 0x21;

pub static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

impl Interrupt {
    pub fn init() {
        unsafe {
            IDT.double_fault.set_handler_fn(double_fault_handler);
            IDT.page_fault.set_handler_fn(page_fault_handler);
            IDT[INT_TIMER].set_handler_fn(timer_handler);
            IDT[INT_KEYBOARD].set_handler_fn(keyboard_handler);
            IDT.load();
        }

        PIC::remap_irq1();
    }
}

extern "x86-interrupt" fn double_fault_handler(
    _stack_frame: InterruptStackFrame,
    _dummy: u64,
) -> ! {
    loop {
        hlt();
    }
}

extern "x86-interrupt" fn page_fault_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: PageFaultErrorCode,
) {
}

extern "x86-interrupt" fn timer_handler(_stack_frame: InterruptStackFrame) {
    let frame_buffer = FrameBuffer::get();

    let white = frame_buffer.make_color(0xFF, 0xFF, 0xFF);
    frame_buffer.draw_text(64, 128, b"TIMER", white);
    PIC::eoi(0x0);
}

extern "x86-interrupt" fn keyboard_handler(_stack_frame: InterruptStackFrame) {
    let frame_buffer = FrameBuffer::get();

    let white = frame_buffer.make_color(0xFF, 0xFF, 0xFF);
    frame_buffer.draw_text(0, 48, b"Keyboard", white);
    PIC::eoi(0x1);
}

mod pic;
