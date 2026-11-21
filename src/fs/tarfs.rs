//! Minimal USTAR reader for a GRUB-loaded initrd module.
//!
//! The bootloader places the initrd at a physical address we identity-map,
//! so we can read it directly. Only regular files and directories are
//! supported; GNU tar's long-name extension is not handled.

use core::sync::atomic::{AtomicU64, Ordering};

/// (start, end) physical addresses of the initrd module.
static INITRD_START: AtomicU64 = AtomicU64::new(0);
static INITRD_END: AtomicU64 = AtomicU64::new(0);

/// USTAR header layout (512 bytes).
#[repr(C)]
struct TarHeader {
    name: [u8; 100],
    mode: [u8; 8],
    uid: [u8; 8],
    gid: [u8; 8],
    size: [u8; 12],
    mtime: [u8; 12],
    chksum: [u8; 8],
    typeflag: u8,
    linkname: [u8; 100],
    magic: [u8; 6],
    version: [u8; 2],
    uname: [u8; 32],
    gname: [u8; 32],
    devmajor: [u8; 8],
    devminor: [u8; 8],
    prefix: [u8; 155],
}

/// Initialize the tarfs with the initrd module range.
pub fn init(start: u64, end: u64) {
    INITRD_START.store(start, Ordering::Release);
    INITRD_END.store(end, Ordering::Release);
    if start != 0 && end > start {
        crate::serial_println!(
            "tarfs: initrd at 0x{:x}-0x{:x} ({} bytes)",
            start,
            end,
            end - start
        );
    }
}

/// Return true if an initrd module has been registered.
pub fn is_present() -> bool {
    INITRD_END.load(Ordering::Acquire) > INITRD_START.load(Ordering::Acquire)
}

/// Parse a tar octal number field.
fn parse_octal(field: &[u8]) -> Option<u64> {
    let mut value: u64 = 0;
    let mut any = false;
    for &b in field {
        if b == 0 || b == b' ' {
            if any {
                break;
            }
            continue;
        }
        if !(b'0'..=b'7').contains(&b) {
            return None;
        }
        any = true;
        value = value.checked_mul(8)?.checked_add((b - b'0') as u64)?;
    }
    Some(value)
}

/// A single file entry in the archive.
pub struct TarEntry {
    pub name: [u8; 100],
    pub name_len: usize,
    pub data: &'static [u8],
    pub is_dir: bool,
}

impl TarEntry {
    pub fn name_str(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len]).unwrap_or("")
    }
}

/// Internal iterator over tar headers.
struct TarIter {
    pos: usize,
    end: usize,
    base: usize,
}

impl TarIter {
    fn new() -> Option<Self> {
        let start = INITRD_START.load(Ordering::Acquire) as usize;
        let end = INITRD_END.load(Ordering::Acquire) as usize;
        if start == 0 || end <= start {
            return None;
        }
        Some(TarIter { pos: 0, end: end - start, base: start })
    }

    fn next_entry(&mut self) -> Option<TarEntry> {
        while self.pos + 512 <= self.end {
            let hdr_ptr = (self.base + self.pos) as *const TarHeader;
            let hdr = unsafe { &*hdr_ptr };

            // Two consecutive zero blocks mark end of archive.
            if hdr.name.iter().all(|&b| b == 0) {
                return None;
            }

            let name_len = hdr.name.iter().position(|&b| b == 0).unwrap_or(100);
            let size = parse_octal(&hdr.size).unwrap_or(0) as usize;
            let typeflag = hdr.typeflag;
            let data_start = self.pos + 512;
            let data_end = data_start + size;
            if data_end > self.end {
                return None;
            }

            let data = unsafe {
                core::slice::from_raw_parts((self.base + data_start) as *const u8, size)
            };

            // Advance past header and data, rounded up to 512-byte boundary.
            let padded = (size + 511) & !511;
            self.pos = data_start + padded;

            if typeflag == b'5' || (name_len > 0 && hdr.name[name_len - 1] == b'/') {
                return Some(TarEntry {
                    name: hdr.name,
                    name_len,
                    data: &[],
                    is_dir: true,
                });
            }
            if typeflag == b'0' || typeflag == 0 || typeflag == b'7' {
                return Some(TarEntry {
                    name: hdr.name,
                    name_len,
                    data,
                    is_dir: false,
                });
            }
        }
        None
    }
}

/// List all archive entries to the serial console.
pub fn list() {
    if !is_present() {
        crate::serial_println!("tarfs: no initrd loaded");
        return;
    }
    let mut iter = match TarIter::new() {
        Some(i) => i,
        None => return,
    };
    crate::serial_println!("tarfs contents:");
    while let Some(entry) = iter.next_entry() {
        if entry.is_dir {
            crate::serial_println!("  d  {}", entry.name_str());
        } else {
            crate::serial_println!("  -  {} ({} bytes)", entry.name_str(), entry.data.len());
        }
    }
}

/// Find a file by name and return its contents.
pub fn read(name: &str) -> Option<&'static [u8]> {
    if !is_present() {
        return None;
    }
    let mut iter = TarIter::new()?;
    while let Some(entry) = iter.next_entry() {
        if !entry.is_dir && entry.name_str() == name {
            return Some(entry.data);
        }
    }
    None
}
