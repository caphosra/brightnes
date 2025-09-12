use pic8259::ChainedPics;
use spin::{Lazy, Mutex};
use x86_64::instructions::hlt;
use x86_64::instructions::port::Port;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use crate::int::keyboard::BKeyboard;

const PIC_1_OFFSET: u8 = 0x20;
const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[repr(u8)]
pub enum InterruptIdx {
    #[allow(dead_code)]
    Timer = PIC_1_OFFSET,
    Keyboard,
}

static IDT: Lazy<InterruptDescriptorTable> = Lazy::new(|| {
    let mut idt = InterruptDescriptorTable::new();
    idt.double_fault.set_handler_fn(double_fault_handler);
    idt.general_protection_fault
        .set_handler_fn(general_protection_fault_handler);
    idt.page_fault.set_handler_fn(page_fault_handler);
    idt[InterruptIdx::Keyboard as u8].set_handler_fn(keyboard_handler);
    idt
});

pub struct Interrupt;

impl Interrupt {
    pub fn init() {
        IDT.load();

        {
            let mut pics = PICS.lock();
            unsafe { pics.initialize() };
            unsafe { pics.write_masks(0xFD, 0xFF) };
        }
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

extern "x86-interrupt" fn keyboard_handler(_stack_frame: InterruptStackFrame) {
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    BKeyboard::on_event(scancode);

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIdx::Keyboard as u8);
    }
}

mod keyboard;
