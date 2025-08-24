use spin::Lazy;
use x86_64::instructions::hlt;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use crate::frame_buffer::FrameBuffer;
use crate::int::pic::PIC;

pub struct Interrupt;

const INT_TIMER: u8 = 0x20;
const INT_KEYBOARD: u8 = 0x21;

static IDT: Lazy<InterruptDescriptorTable> = Lazy::new(|| {
    let mut idt = InterruptDescriptorTable::new();
    idt.double_fault.set_handler_fn(double_fault_handler);
    idt.general_protection_fault
        .set_handler_fn(general_protection_fault_handler);
    idt.page_fault.set_handler_fn(page_fault_handler);
    idt[INT_TIMER].set_handler_fn(timer_handler);
    idt[INT_KEYBOARD].set_handler_fn(keyboard_handler);
    idt
});

impl Interrupt {
    pub fn init() {
        IDT.load();

        PIC::remap_irq1();
    }
}

extern "x86-interrupt" fn double_fault_handler(_stack_frame: InterruptStackFrame, _code: u64) -> ! {
    loop {
        hlt();
    }
}

extern "x86-interrupt" fn general_protection_fault_handler(
    _stack_frame: InterruptStackFrame,
    _code: u64,
) {
    loop {
        hlt();
    }
}

extern "x86-interrupt" fn page_fault_handler(
    _stack_frame: InterruptStackFrame,
    _code: PageFaultErrorCode,
) {
    loop {
        hlt();
    }
}

extern "x86-interrupt" fn timer_handler(_stack_frame: InterruptStackFrame) {
    // Do nothing.

    PIC::eoi(0x0);
}

extern "x86-interrupt" fn keyboard_handler(_stack_frame: InterruptStackFrame) {
    let frame_buffer = FrameBuffer::get();

    let white = frame_buffer.make_color(0xFF, 0xFF, 0xFF);
    frame_buffer.draw_text(0, 48, b"Keyboard", white);
    PIC::eoi(0x1);
}

mod pic;
