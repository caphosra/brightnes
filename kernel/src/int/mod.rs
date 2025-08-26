use spin::Lazy;
use x86_64::instructions::hlt;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use crate::int::pic::PIC;
use crate::log;

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

    log!("Tick!");
    PIC::eoi(INT_TIMER);
}

extern "x86-interrupt" fn keyboard_handler(_stack_frame: InterruptStackFrame) {
    log!("Detected a keydown event");
    PIC::eoi(INT_KEYBOARD);
}

mod pic;
