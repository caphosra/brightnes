use core::{mem::transmute, slice::from_raw_parts};

use spin::{Lazy, RwLock};

#[repr(C)]
pub struct PSFHeader {
    magic: [u8; 4],
    version: u32,
    header_size: u32,
    flags: u32,
    num_glyph: u32,
    bytes_per_glyph: u32,
    height: u32,
    width: u32,
}

const FONT_ADDR: u64 = 0x1_000_000;

pub struct FontManager;

static GLYPH_INDEX_TBL: Lazy<RwLock<[usize; 0x100]>> = Lazy::new(|| RwLock::new([0; 0x100]));

impl FontManager {
    pub fn get_psf_header() -> &'static mut PSFHeader {
        unsafe { (FONT_ADDR as *mut PSFHeader).as_mut().unwrap() }
    }

    pub fn validate_psf_header(header: &PSFHeader) -> bool {
        header.magic == [0x72, 0xb5, 0x4a, 0x86]
            && header.version == 0
            && header.header_size == 0x20
            && header.flags == 1
            && header.bytes_per_glyph == 0x10
            && header.height == 0x10
            && header.width == 0x8
    }

    pub fn init_glyph_index_table() {
        let header = Self::get_psf_header();
        let glyph_tbl: *const u8 = unsafe {
            transmute(
                FONT_ADDR
                    + header.header_size as u64
                    + header.bytes_per_glyph as u64 * header.num_glyph as u64,
            )
        };
        let mut current = 0;
        let mut glyph = 0;
        while glyph < header.num_glyph {
            let first = unsafe { *glyph_tbl.add(current) };
            let next = unsafe { *glyph_tbl.add(current + 1) };
            if first == 0xff {
                // The glyph is not used.
                glyph += 1;
                current += 1;
                continue;
            }
            if next != 0xff {
                // The glyph is for multi-byte characters.
                // We will skip it.
                current += 2;

                // The consumer loop can be never stopped, so add a limit.
                let mut loop_count = 0;
                let max_len = 0x10;
                while unsafe { *glyph_tbl.add(current) } != 0xff && loop_count < max_len {
                    current += 1;
                    loop_count += 1;
                }
                if loop_count >= max_len {
                    panic!("The unicode table is corrupted.");
                }

                glyph += 1;
                current += 1;
                continue;
            }

            // The glyph is a single-byte character.
            GLYPH_INDEX_TBL.write()[first as usize] = glyph as usize;
            glyph += 1;
            current += 2;
        }
        ()
    }

    pub fn get_glyph(index: usize) -> &'static [u8] {
        let header = Self::get_psf_header();
        if header.num_glyph <= index as u32 {
            panic!("The glyph index {} is out of range.", index);
        }

        let offset =
            FONT_ADDR + header.header_size as u64 + header.bytes_per_glyph as u64 * index as u64;
        unsafe { from_raw_parts(offset as *const u8, 0x10) }
    }

    pub fn get_glyph_by_char(c: u8) -> &'static [u8] {
        let index = GLYPH_INDEX_TBL.read()[c as usize];
        if index == 0 {
            panic!("The glyph for character '{}' is not found.", c as char);
        }
        Self::get_glyph(index)
    }
}
