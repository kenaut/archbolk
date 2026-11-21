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
    // e_type = 2 (ET_EXEC) or 3 (ET_DYN, produced by -C relocation-model=pic
    // but statically linked), e_machine = 62 (EM_X86_64)
    let e_type = u16::from_le_bytes(elf[16..18].try_into().ok()?);
    let e_machine = u16::from_le_bytes(elf[18..20].try_into().ok()?);
    if (e_type != 2 && e_type != 3) || e_machine != 62 {
        serial_println!("elf: not an x86_64 EXEC/DYN (type={} machine={})", e_type, e_machine);
        return None;
    }
    Some(read_u64(elf, 24)?)
}

/// Load all PT_LOAD segments of `elf` into freshly mapped user pages.
/// Returns the relocated entry point on success.
///
/// `base` is the user virtual address to load at. The ELF's lowest PT_LOAD
/// vaddr (its link base) is subtracted from each segment's vaddr and `base`
/// is added instead, so a program linked at a low fixed address can be
/// loaded into any slot. RIP-relative code works unmodified under this
/// scheme; absolute addresses would need relocation (not supported).
pub fn load(elf: &[u8], user_flags: EntryFlags, base: usize) -> Option<usize> {
    let elf_entry = validate(elf)?;
    let phoff = read_u64(elf, 32)? as usize;
    let phentsize = u16::from_le_bytes(elf[54..56].try_into().ok()?) as usize;
    let phnum = u16::from_le_bytes(elf[56..58].try_into().ok()?) as usize;

    if phentsize < 56 {
        serial_println!("elf: bad phentsize {}", phentsize);
        return None;
    }

    // First pass: find the lowest PT_LOAD vaddr (the link base) so we can
    // relocate all segments relative to it.
    let mut link_base: usize = usize::MAX;
    for i in 0..phnum {
        let ph = phoff + i * phentsize;
        if ph + 56 > elf.len() {
            return None;
        }
        let p_type = u32::from_le_bytes(elf[ph..ph + 4].try_into().ok()?);
        if p_type != PT_LOAD {
            continue;
        }
        let p_vaddr = read_u64(elf, ph + 16)? as usize;
        let p_memsz = read_u64(elf, ph + 40)? as usize;
        if p_memsz == 0 {
            continue;
        }
        link_base = core::cmp::min(link_base, p_vaddr & !0xfff);
    }
    if link_base == usize::MAX {
        serial_println!("elf: no PT_LOAD segments");
        return None;
    }

    // Relocated entry point.
    let entry_addr = base + (elf_entry as usize - link_base);

    // Track pages we've already mapped so overlapping PT_LOAD segments
    // (common in Rust binaries) share the same physical frame.
    let mut mapped_vaddrs: [usize; 64] = [usize::MAX; 64];
    let mut mapped_phys: [usize; 64] = [0; 64];
    let mut mapped_count = 0usize;

    // Second pass: map and copy each segment at base + (vaddr - link_base).
    for i in 0..phnum {
        let ph = phoff + i * phentsize;
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

        let rel_vaddr = base + (p_vaddr - link_base);
        let seg_start = rel_vaddr & !0xfff;
        let seg_end = (rel_vaddr + p_memsz + 0xfff) & !0xfff;
        serial_println!(
            "elf: PT_LOAD vaddr=0x{:x} -> 0x{:x} filesz={} memsz={}",
            p_vaddr, rel_vaddr, p_filesz, p_memsz
        );

        let mut page_vaddr = seg_start;
        while page_vaddr < seg_end {
            // Find this page in the mapped list, or allocate+map a new one.
            let frame = if let Some(k) = mapped_vaddrs[..mapped_count]
                .iter()
                .position(|&v| v == page_vaddr)
            {
                mapped_phys[k]
            } else {
                if mapped_count >= mapped_vaddrs.len() {
                    serial_println!("elf: too many pages");
                    return None;
                }
                let f = crate::memory::pfa::FRAME_ALLOCATOR
                    .lock()
                    .allocate()
                    .expect("elf: OOM loading segment");
                let dst = f as *mut u8;
                unsafe {
                    core::ptr::write_bytes(dst, 0, 4096);
                    map_page(page_vaddr, f, user_flags);
                }
                mapped_vaddrs[mapped_count] = page_vaddr;
                mapped_phys[mapped_count] = f;
                mapped_count += 1;
                f
            };
            let dst = frame as *mut u8;

            // Copy the portion of this page covered by p_filesz.
            // rel file offset = page_vaddr - rel_vaddr (clamped at 0).
            let page_file_start = if rel_vaddr > page_vaddr { rel_vaddr - page_vaddr } else { 0 };
            let file_remaining = p_filesz.saturating_sub(page_vaddr.saturating_sub(rel_vaddr));
            let copy_len = core::cmp::min(4096 - page_file_start, file_remaining);
            let src_off = p_offset + (page_vaddr.saturating_sub(rel_vaddr));
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

    Some(entry_addr)
}
