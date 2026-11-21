//! Minimal ELF64 loader for freestanding, statically linked user programs.
//!
//! Supports the subset produced by a `x86_64-unknown-none` Rust/Cargo build
//! with `-C relocation-model=static`:
//!   - ELF64, little endian, current version
//!   - EXEC type (not PIE/DYN)
//!   - PT_LOAD segments only
//!
//! Segments are mapped at their `p_vaddr` as-is, so user programs must be
//! linked for the fixed user base address (`USER_ELF_BASE` in main.rs).

use crate::memory::paging::{map_page, EntryFlags};
use crate::serial_println;

const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
const PT_LOAD: u32 = 1;

/// ELF64 header (64 bytes).
#[repr(C)]
struct Elf64Header {
    e_ident: [u8; 16],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u64,
    e_phoff: u64,
    e_shoff: u64,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

/// ELF64 program header (56 bytes).
#[repr(C)]
struct Elf64Phdr {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

fn read_u64(data: &[u8], off: usize) -> Option<u64> {
    let bytes = data.get(off..off + 8)?;
    Some(u64::from_le_bytes(bytes.try_into().ok()?))
}

/// Validate an ELF64 image and return its entry point.
pub fn validate(elf: &[u8]) -> Option<u64> {
    if elf.len() < 64 || elf[..4] != ELF_MAGIC {
        serial_println!("elf: bad magic or too short");
        return None;
    }
    if elf[4] != 2 || elf[5] != 1 || elf[6] != 1 {
        serial_println!("elf: unsupported class/data/version");
        return None;
    }
    // e_type = 2 (ET_EXEC), e_machine = 62 (EM_X86_64)
    let e_type = u16::from_le_bytes(elf[16..18].try_into().ok()?);
    let e_machine = u16::from_le_bytes(elf[18..20].try_into().ok()?);
    if e_type != 2 || e_machine != 62 {
        serial_println!("elf: not an x86_64 EXEC (type={} machine={})", e_type, e_machine);
        return None;
    }
    Some(read_u64(elf, 24)?)
}

/// Load all PT_LOAD segments of `elf` into freshly mapped user pages.
/// Returns the entry point on success.
///
/// `user_flags` must grant user access. Each segment's pages are mapped at
/// `p_vaddr` (page-aligned down) and the file bytes copied in.
pub fn load(elf: &[u8], user_flags: EntryFlags) -> Option<usize> {
    let entry = validate(elf)?;
    let phoff = read_u64(elf, 32)? as usize;
    let phentsize = u16::from_le_bytes(elf[54..56].try_into().ok()?) as usize;
    let phnum = u16::from_le_bytes(elf[56..58].try_into().ok()?) as usize;

    if phentsize < 56 {
        serial_println!("elf: bad phentsize {}", phentsize);
        return None;
    }

    for i in 0..phnum {
        let ph = phoff + i * phentsize;
        if ph + 56 > elf.len() {
            serial_println!("elf: program header out of range");
            return None;
        }
        let p_type = u32::from_le_bytes(elf[ph..ph + 4].try_into().ok()?);
        if p_type != PT_LOAD {
            continue;
        }
        let p_offset = read_u64(elf, ph + 8)? as usize;
        let p_vaddr = read_u64(elf, ph + 16)? as usize;
        let p_filesz = read_u64(elf, ph + 32)? as usize;
        let p_memsz = read_u64(elf, ph + 40)? as usize;

        if p_filesz > p_memsz || p_offset + p_filesz > elf.len() {
            serial_println!("elf: invalid segment sizes");
            return None;
        }

        let seg_start = p_vaddr & !0xfff;
        let seg_end = (p_vaddr + p_memsz + 0xfff) & !0xfff;
        serial_println!(
            "elf: PT_LOAD vaddr=0x{:x} filesz={} memsz={} pages=0x{:x}-0x{:x}",
            p_vaddr, p_filesz, p_memsz, seg_start, seg_end
        );

        let mut page_vaddr = seg_start;
        while page_vaddr < seg_end {
            let frame = crate::memory::pfa::FRAME_ALLOCATOR
                .lock()
                .allocate()
                .expect("elf: OOM loading segment");
            // Zero the frame before copying file bytes.
            let dst = frame as *mut u8;
            unsafe {
                core::ptr::write_bytes(dst, 0, 4096);
            }
            unsafe { map_page(page_vaddr, frame, user_flags); }

            // Copy the portion of this page covered by p_filesz.
            let page_file_start = if p_vaddr > page_vaddr { p_vaddr - page_vaddr } else { 0 };
            let file_remaining = p_filesz.saturating_sub(page_vaddr.saturating_sub(p_vaddr));
            let copy_len = core::cmp::min(4096 - page_file_start, file_remaining);
            let src_off = p_offset + (page_vaddr.saturating_sub(p_vaddr));
            if copy_len > 0 {
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        elf.as_ptr().add(src_off),
                        dst.add(page_file_start),
                        copy_len,
                    );
                }
            }
            page_vaddr += 0x1000;
        }
    }

    Some(entry as usize)
}
