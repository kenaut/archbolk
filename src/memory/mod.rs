pub mod pfa;
pub mod paging;
pub mod heap;

use crate::{serial_print, serial_println};
use multiboot2::{BootInformation, BootInformationHeader};

/// Initialize the memory subsystem.
///
/// Called once during kernel boot with the physical address of the
/// Multiboot2 information structure.
pub fn init(mbi_addr: usize) {
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

    serial_println!("Memory subsystem initialized.");
}
