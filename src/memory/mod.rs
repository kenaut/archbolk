pub mod pfa;
pub mod paging;
pub mod heap;

use crate::{serial_print, serial_println};
use multiboot2::{BootInformation, BootInformationHeader};

/// Memory map entry type: Available RAM.
const XEN_HVM_MEMMAP_TYPE_RAM: u32 = 1;

/// Layout of the hvm_start_info structure (version 1).
///
/// See Linux include/xen/interface/hvm/start_info.h for canonical definition.
#[repr(C)]
struct HvmStartInfo {
    magic: u32,           // 0x336ec578
    version: u32,         // 1
    flags: u32,
    nr_modules: u32,
    modlist_paddr: u64,
    cmdline_paddr: u64,
    rsdp_paddr: u64,
    // Version 1+ fields:
    memmap_paddr: u64,
    memmap_entries: u32,
    reserved: u32,
}

/// Layout of a memory map table entry in hvm_start_info.
#[repr(C)]
struct HvmMemmapEntry {
    addr: u64,
    size: u64,
    typ: u32,
    reserved: u32,
}

/// Initialize the memory subsystem.
///
/// Called once during kernel boot.
///
/// - `boot_info_addr`: physical address of boot info structure
/// - `is_pvh`: if true, `boot_info_addr` points to `hvm_start_info`;
///             if false, it points to a Multiboot2 info structure.
pub fn init(boot_info_addr: usize, is_pvh: bool) {
    serial_println!("boot_info_addr = 0x{:x}, is_pvh = {}", boot_info_addr, is_pvh);

    if is_pvh {
        init_pvh(boot_info_addr);
    } else {
        init_multiboot2(boot_info_addr);
    }
}

/// Initialize memory subsystem from PVH hvm_start_info.
fn init_pvh(start_info_addr: usize) {
    // SAFETY: boot_info_addr points to a valid hvm_start_info provided by QEMU.
    let si = unsafe { &*(start_info_addr as *const HvmStartInfo) };

    serial_println!("PVH start_info: version={}, flags=0x{:x}, nr_modules={}",
        si.version, si.flags, si.nr_modules);
    serial_println!("PVH memmap: paddr=0x{:x}, entries={}",
        si.memmap_paddr, si.memmap_entries);

    // Dump raw hvm_start_info bytes for debugging
    let ptr = start_info_addr as *const u8;
    serial_print!("hvm_start_info raw bytes: ");
    for i in 0..56 {
        let byte: u8 = unsafe { *ptr.add(i) };
        serial_print!("{:02x} ", byte);
    }
    serial_println!("");

    // Parse memory map entries from hvm_start_info
    let mut mmap_entries: [(u64, u64); 32] = [(0, 0); 32];
    let mut mmap_count = 0;

    if si.version >= 1 && si.memmap_entries > 0 && si.memmap_paddr != 0 {
        let entries_ptr = si.memmap_paddr as *const HvmMemmapEntry;
        for i in 0..si.memmap_entries.min(32) {
            // SAFETY: entries are provided by QEMU.
            let entry = unsafe { &*entries_ptr.add(i as usize) };
            serial_println!(
                "  memmap[{}] addr=0x{:016x} size=0x{:016x} type={}",
                i, entry.addr, entry.size, entry.typ
            );
            if entry.typ == XEN_HVM_MEMMAP_TYPE_RAM && mmap_count < 32 {
                mmap_entries[mmap_count] = (entry.addr, entry.size);
                mmap_count += 1;
            }
        }
    } else {
        serial_println!("WARNING: No memory map from PVH! Using fallback.");
        // Fallback: assume 128 MiB of RAM starting at 1 MiB.
        // This is enough to boot and test, but not correct for production.
        mmap_entries[0] = (0x100000, 128 * 1024 * 1024); // 1 MiB → 129 MiB
        mmap_count = 1;
    }

    // Initialize physical frame allocator from raw memory map entries
    pfa::init_from_mmap(&mmap_entries[..mmap_count]);

    // Initialize page tables
    paging::init(&mmap_entries[..mmap_count]);

    // Initialize kernel heap
    heap::init();

    // Test heap allocation
    heap::test_alloc();

    serial_println!("Memory subsystem initialized (PVH).");
}

/// Initialize memory subsystem from Multiboot2 info.
fn init_multiboot2(mbi_addr: usize) {
    serial_println!("mbi_addr = 0x{:x}", mbi_addr);

    // Dump the first 64 bytes at mbi_addr to see what's there
    let ptr = mbi_addr as *const u8;
    serial_print!("MBI raw bytes: ");
    for i in 0..64 {
        let byte: u8 = unsafe { *ptr.add(i) };
        serial_print!("{:02x} ", byte);
    }
    serial_println!("");

    // SAFETY: mbi_addr is a valid physical address provided by the bootloader.
    let boot_info = unsafe {
        BootInformation::load(mbi_addr as *const BootInformationHeader)
            .expect("Invalid Multiboot2 information structure")
    };

    // Parse and print memory map
    if let Some(mmap_tag) = boot_info.memory_map_tag() {
        let entries = mmap_tag.memory_areas();
        serial_println!("Memory map entries: {}", entries.len());
        for (i, area) in entries.iter().enumerate() {
            serial_println!(
                "  [{:>3}] base=0x{:016x} len=0x{:016x} type={:?}",
                i,
                area.start_address(),
                area.size(),
                area.typ(),
            );
        }
    } else {
        serial_println!("WARNING: No memory map tag found!");
    }

    // Print total memory
    if let Some(basic_info) = boot_info.basic_memory_info_tag() {
        let lower = basic_info.memory_lower();
        let upper = basic_info.memory_upper();
        serial_println!("Lower memory: {} KiB", lower);
        serial_println!("Upper memory: {} KiB", upper);
    }

    // Initialize physical frame allocator from the memory map
    pfa::init(&boot_info);

    // Collect memory map entries for page table init.
    // Only include Available type regions — Reserved regions may extend
    // far beyond usable RAM (e.g. up to 4 TiB for MMIO), and we only
    // need to identity-map the physical memory that the frame allocator
    // will hand out.
    let mut mmap_entries: [(u64, u64); 32] = [(0, 0); 32];
    let mut mmap_count = 0;
    if let Some(mmap_tag) = boot_info.memory_map_tag() {
        for area in mmap_tag.memory_areas() {
            if area.typ() == multiboot2::MemoryAreaType::Available && mmap_count < 32 {
                mmap_entries[mmap_count] = (area.start_address(), area.size());
                mmap_count += 1;
            }
        }
    }

    // Initialize page tables (extends identity map to cover all physical memory)
    paging::init(&mmap_entries[..mmap_count]);

    // Initialize kernel heap
    heap::init();

    // Test heap allocation
    heap::test_alloc();

    // Locate the GRUB-loaded initrd module (tar archive).
    for module in boot_info.module_tags() {
        serial_println!(
            "module at 0x{:x}-0x{:x} ({} bytes)",
            module.start_address(),
            module.end_address(),
            module.end_address() - module.start_address()
        );
        crate::fs::tarfs::init(module.start_address() as u64, module.end_address() as u64);
    }

    serial_println!("Memory subsystem initialized.");
}
