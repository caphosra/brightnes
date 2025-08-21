use core::cmp::{max, min};
use core::ptr::copy_nonoverlapping;
use core::slice::from_raw_parts;

use log::{info, warn};
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

#[repr(C)]
struct SectionHeader {
    sh_name: u32,
    sh_type: u32,
    sh_flags: u64,
    sh_addr: u64,
    sh_offset: u64,
    sh_size: u64,
    sh_link: u32,
    sh_info: u32,
    sh_addralign: u64,
    sh_entsize: u64,
}

#[repr(C)]
struct Relocation {
    r_offset: u64,
    r_info: u64,
    r_addend: i64,
}

#[repr(C)]
struct Symbol {
    st_name: u32,
    st_info: u8,
    st_other: u8,
    st_shndx: u16,
    st_value: u64,
    st_size: u64,
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

pub fn resolve_global_offset_table(buffer: &[u8], elf_header: &ELFHeader) {
    let section_header_name_table = unsafe {
        (buffer.as_ptr().add(
            elf_header.e_shoff as usize
                + elf_header.e_shentsize as usize * elf_header.e_shstrndx as usize,
        ) as *const SectionHeader)
            .as_ref()
    }
    .unwrap();
    let section_name_table = unsafe {
        buffer
            .as_ptr()
            .add(section_header_name_table.sh_offset as usize) as *const u8
    };

    let mut symbol_table: Option<*const Symbol> = None;
    let mut rela_dyn_table: Option<&[Relocation]> = None;
    let mut rela_plt_table: Option<&[Relocation]> = None;
    for idx in 0..elf_header.e_shnum {
        let offset = elf_header.e_shoff as usize + elf_header.e_shentsize as usize * idx as usize;
        let section_header =
            unsafe { (buffer.as_ptr().add(offset) as *const SectionHeader).as_ref() }.unwrap();

        let section_name = unsafe { section_name_table.add(section_header.sh_name as usize) };

        if compare_section_name(section_name, b".rela.dyn") {
            info!("Found .rela.dyn section at {:#x}", section_header.sh_offset);
            rela_dyn_table = Some(unsafe {
                from_raw_parts(
                    buffer.as_ptr().add(section_header.sh_offset as usize) as *const Relocation,
                    section_header.sh_size as usize / size_of::<Relocation>(),
                )
            });
        } else if compare_section_name(section_name, b".rela.plt") {
            info!("Found .rela.plt section at {:#x}", section_header.sh_offset);
            rela_plt_table = Some(unsafe {
                from_raw_parts(
                    buffer.as_ptr().add(section_header.sh_offset as usize) as *const Relocation,
                    section_header.sh_size as usize / size_of::<Relocation>(),
                )
            });
        } else if compare_section_name(section_name, b".dynsym") {
            info!("Found .dynsym section at {:#x}", section_header.sh_offset);
            symbol_table = Some(unsafe {
                buffer.as_ptr().add(section_header.sh_offset as usize) as *const Symbol
            });
        }
    }

    match symbol_table {
        Some(symbols) => {
            if let Some(rela_dyn) = rela_dyn_table {
                resolve_relocation(rela_dyn, symbols);
            } else {
                warn!("No .rela.dyn section found");
            }

            if let Some(rela_plt) = rela_plt_table {
                resolve_relocation(rela_plt, symbols);
            } else {
                warn!("No .rela.plt section found");
            }
        }
        _ => {
            warn!("Failed to find .dynsym section");
        }
    }
}

fn resolve_relocation(rel_table: &[Relocation], symbols: *const Symbol) {
    for rel in rel_table {
        let symbol = unsafe { symbols.add(rel.r_info as usize >> 32).as_ref() }.unwrap();
        let dest: *mut usize = rel.r_offset as *mut usize;
        unsafe {
            *dest = symbol.st_value as usize;
        }

        info!(
            "Resolved the address of symbol {} at {:#x} to {:#x}",
            symbol.st_name, dest as usize, symbol.st_value
        );
    }
}

pub fn compare_section_name(name1: *const u8, name2: &[u8]) -> bool {
    let mut i = 0;
    while unsafe { *name1.add(i) } != 0 && i < name2.len() {
        if unsafe { *name1.add(i) } != name2[i] {
            return false;
        }
        i += 1;
    }
    i == name2.len() && unsafe { *name1.add(i) } == 0
}
