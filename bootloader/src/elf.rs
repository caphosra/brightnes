use core::cmp::{max, min};
use core::ptr::copy_nonoverlapping;

use log::info;
use uefi::boot::{AllocateType, MemoryType};

#[repr(C)]
pub struct ELFHeader {
    pub e_ident: [u8; 0x10],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}

#[repr(C)]
struct ProgramHeader {
    p_type: u16,
    p_flags: u16,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

const PT_LOAD: u16 = 1;

pub fn load_elf_header(buffer: &[u8]) -> Option<&ELFHeader> {
    let header = unsafe { (buffer.as_ptr() as *const ELFHeader).as_ref() }?;
    if header.e_ident[0] == 0x7f
        && header.e_ident[1] == b'E'
        && header.e_ident[2] == b'L'
        && header.e_ident[3] == b'F'
    {
        Some(header)
    } else {
        None
    }
}

pub fn extract_elf_program(buffer: &[u8], elf_header: &ELFHeader) {
    let mut ml_addr = u64::MAX;
    let mut ms_addr = 0u64;

    for idx in 0..elf_header.e_phnum {
        let offset = elf_header.e_phoff as usize + elf_header.e_phentsize as usize * idx as usize;
        let header =
            unsafe { (buffer.as_ptr().add(offset) as *const ProgramHeader).as_ref() }.unwrap();

        if header.p_type == PT_LOAD {
            ml_addr = min(header.p_vaddr, ml_addr);
            ms_addr = max(header.p_vaddr + header.p_memsz, ms_addr);
        }
    }

    let start_addr = ml_addr >> 12 << 12;
    let page_num = ((ms_addr + 0xfff >> 12) - (start_addr >> 12)) as usize;
    let _ = uefi::boot::allocate_pages(
        AllocateType::Address(start_addr),
        MemoryType::LOADER_CODE,
        page_num,
    )
    .unwrap();

    info!("Allocated {} pages from {:#x}", page_num, start_addr);

    for idx in 0..elf_header.e_phnum {
        let offset = elf_header.e_phoff as usize + elf_header.e_phentsize as usize * idx as usize;
        let header =
            unsafe { (buffer.as_ptr().add(offset) as *const ProgramHeader).as_ref() }.unwrap();

        if header.p_type == PT_LOAD {
            unsafe {
                let src = buffer.as_ptr().add(header.p_offset as usize);
                let dest = header.p_vaddr as *mut u8;
                let size = header.p_memsz as usize;
                copy_nonoverlapping(src, dest, size);

                info!(
                    "Loaded {:#x} bytes from {:#x} to {:#x}",
                    size, src as usize, dest as usize
                );
            }
        }
    }
}
