use crate::memory::pfa::FRAME_ALLOCATOR;
use crate::serial_println;

/// Page table entry flags.
#[derive(Clone, Copy)]
pub struct EntryFlags(u64);

#[allow(dead_code)]
impl EntryFlags {
    pub const PRESENT: u64 = 1 << 0;
    pub const WRITABLE: u64 = 1 << 1;
    pub const USER: u64 = 1 << 2;
    pub const WRITE_THROUGH: u64 = 1 << 3;
    pub const NO_CACHE: u64 = 1 << 4;
    pub const ACCESSED: u64 = 1 << 5;
    pub const DIRTY: u64 = 1 << 6;
    pub const HUGE_PAGE: u64 = 1 << 7;
    pub const GLOBAL: u64 = 1 << 8;
    pub const NO_EXECUTE: u64 = 1 << 63;

    pub fn new() -> Self {
        EntryFlags(0)
    }

    pub fn set(&mut self, flag: u64) {
        self.0 |= flag;
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

/// A 64-bit page table entry.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    pub fn is_unused(&self) -> bool {
        self.0 == 0
    }

    pub fn set_addr(&mut self, addr: usize) {
        self.0 = (self.0 & 0xfff) | (addr as u64 & 0x000f_ffff_ffff_f000);
    }

    pub fn addr(&self) -> usize {
        (self.0 & 0x000f_ffff_ffff_f000) as usize
    }

    pub fn set_flags(&mut self, flags: EntryFlags) {
        self.0 |= flags.value();
    }

    pub fn flags(&self) -> u64 {
        self.0 & 0xfff
    }
}

/// A page table (512 entries, 4 KiB total).
#[repr(align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; 512],
}

impl PageTable {
    pub fn zeroed() -> Self {
        PageTable {
            entries: [PageTableEntry(0); 512],
        }
    }
}

/// Initialize page table management.
///
/// Extends the boot identity mapping (first 2 MiB) to cover all
/// available physical memory so that physical addresses returned by
/// the frame allocator can be dereferenced directly.
///
/// `memory_map_entries` is a slice of (base, length) pairs from the
/// Multiboot2 memory map, used to determine the highest physical
/// address that needs identity mapping.
pub fn init(memory_map_entries: &[(u64, u64)]) {
    let cr3: u64;
    // SAFETY: Reading CR3 is always safe.
    unsafe {
        core::arch::asm!("mov {}, cr3", out(reg) cr3);
    }

    serial_println!("Page tables: CR3 = 0x{:016x}", cr3);

    // Find the highest usable physical address to determine how much
    // to identity-map.
    let mut max_phys: u64 = 0;
    for &(base, len) in memory_map_entries {
        let end = base + len;
        if end > max_phys {
            max_phys = end;
        }
    }
    // Round up to 2 MiB boundary
    let identity_end = ((max_phys + (2 * 1024 * 1024) - 1) / (2 * 1024 * 1024)) * (2 * 1024 * 1024);
    serial_println!("  Extending identity map to 0x{:016x}", identity_end);

    // Walk existing PML4[0] -> PDP[0] -> PD
    let pml4 = unsafe { &mut *(cr3 as *mut PageTable) };
    let pdp = unsafe { &mut *((pml4.entries[0].addr()) as *mut PageTable) };
    let pd = unsafe { &mut *((pdp.entries[0].addr()) as *mut PageTable) };

    serial_println!("  PML4[0] -> 0x{:016x}", pml4.entries[0].addr());
    serial_println!("  PDP[0]  -> 0x{:016x}", pdp.entries[0].addr());
    serial_println!("  PD[0]   -> 0x{:016x} (flags=0x{:03x})",
        pd.entries[0].addr(), pd.entries[0].flags());

    // PD[0] and PD[1] are already set up by boot.S (0-4 MiB).
    // Add PD entries for 4 MiB up to identity_end.
    // Each PD entry covers 2 MiB.
    // Clamp to PD's 512 entries (max 1024 MiB identity map).
    let start_pd_index = 2; // PD[2] = 4 MiB
    let end_pd_index = core::cmp::min(
        (identity_end / (2 * 1024 * 1024)) as usize,
        512,
    );
    for i in start_pd_index..end_pd_index {
        let phys = (i as u64) * (2 * 1024 * 1024);
        // Flags: present | writable | huge page
        pd.entries[i].0 = phys | 0x83;
    }

    serial_println!("Page tables initialized.");
}

/// Map a 4 KiB page at `virtual_addr` to `physical_addr`.
///
/// # Safety
/// Caller must ensure the virtual address is not already mapped and
/// that the physical address is a valid allocated frame.
pub unsafe fn map_page(virtual_addr: usize, physical_addr: usize, flags: EntryFlags) {
    let p4_index = (virtual_addr >> 39) & 0x1ff;
    let p3_index = (virtual_addr >> 30) & 0x1ff;
    let p2_index = (virtual_addr >> 21) & 0x1ff;
    let p1_index = (virtual_addr >> 12) & 0x1ff;

    // All paging-structure levels must allow user access for the leaf to be
    // usable by ring 3. Propagate USER to intermediate entries we create.
    let user_bit = flags.value() & EntryFlags::USER;

    let cr3: u64;
    // SAFETY: Reading CR3 is always safe.
    unsafe {
        core::arch::asm!("mov {}, cr3", out(reg) cr3);
    }
    // SAFETY: CR3 points to a valid PML4 table set up by the bootloader.
    let pml4 = unsafe { &mut *(cr3 as *mut PageTable) };

    // Walk or create PDP
    let pdp = if pml4.entries[p4_index].is_unused() {
        let frame = FRAME_ALLOCATOR.lock().allocate().expect("OOM: no frame for PDP");
        // SAFETY: frame is a freshly allocated physical page, safe to write.
        let table = unsafe { &mut *(frame as *mut PageTable) };
        *table = PageTable::zeroed();
        let mut flags = EntryFlags::new();
        flags.set(EntryFlags::PRESENT);
        flags.set(EntryFlags::WRITABLE);
        flags.set(user_bit);
        pml4.entries[p4_index].set_addr(frame);
        pml4.entries[p4_index].set_flags(flags);
        table
    } else {
        // SAFETY: entry is present, addr points to a valid page table.
        unsafe { &mut *(pml4.entries[p4_index].addr() as *mut PageTable) }
    };

    // Walk or create PD
    let pd = if pdp.entries[p3_index].is_unused() {
        let frame = FRAME_ALLOCATOR.lock().allocate().expect("OOM: no frame for PD");
        // SAFETY: frame is a freshly allocated physical page, safe to write.
        let table = unsafe { &mut *(frame as *mut PageTable) };
        *table = PageTable::zeroed();
        let mut f = EntryFlags::new();
        f.set(EntryFlags::PRESENT);
        f.set(EntryFlags::WRITABLE);
        f.set(user_bit);
        pdp.entries[p3_index].set_addr(frame);
        pdp.entries[p3_index].set_flags(f);
        table
    } else {
        // SAFETY: entry is present, addr points to a valid page table.
        unsafe { &mut *(pdp.entries[p3_index].addr() as *mut PageTable) }
    };

    // Walk or create PT
    let pt = if pd.entries[p2_index].is_unused() {
        let frame = FRAME_ALLOCATOR.lock().allocate().expect("OOM: no frame for PT");
        // SAFETY: frame is a freshly allocated physical page, safe to write.
        let table = unsafe { &mut *(frame as *mut PageTable) };
        *table = PageTable::zeroed();
        let mut f = EntryFlags::new();
        f.set(EntryFlags::PRESENT);
        f.set(EntryFlags::WRITABLE);
        f.set(user_bit);
        pd.entries[p2_index].set_addr(frame);
        pd.entries[p2_index].set_flags(f);
        table
    } else {
        // SAFETY: entry is present, addr points to a valid page table.
        unsafe { &mut *(pd.entries[p2_index].addr() as *mut PageTable) }
    };

    // Set the final page table entry
    pt.entries[p1_index].set_addr(physical_addr);
    pt.entries[p1_index].set_flags(flags);

    // Flush TLB for this page
    // SAFETY: virtual_addr is a valid mapped page, invlpg is always safe.
    unsafe {
        core::arch::asm!("invlpg [{}]", in(reg) virtual_addr, options(nostack, preserves_flags));
    }
}
