use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};
use crate::serial_println;

/// Heap start virtual address (right after the kernel image).
const HEAP_START: usize = 0x4444_0000_0000;
/// Initial heap size: 64 KiB.
const HEAP_SIZE: usize = 64 * 1024;

/// A simple bump allocator for the kernel heap.
///
/// Allocates memory by incrementing a pointer. Never frees individual
/// allocations — memory is only reclaimed when the entire heap is reset.
/// This is sufficient for early kernel initialization before a proper
/// slab allocator is implemented.
struct BumpAllocator {
    /// Current bump pointer (offset from HEAP_START).
    next: AtomicUsize,
    /// Number of allocations made (for debugging).
    allocations: AtomicUsize,
}

impl BumpAllocator {
    const fn new() -> Self {
        BumpAllocator {
            next: AtomicUsize::new(0),
            allocations: AtomicUsize::new(0),
        }
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let align = layout.align();
        let size = layout.size();

        // Current bump position
        let current = self.next.load(Ordering::SeqCst);

        // Align up
        let aligned = (current + align - 1) & !(align - 1);

        // Check if we have space
        if aligned + size > HEAP_SIZE {
            return core::ptr::null_mut();
        }

        // Update bump pointer
        self.next.store(aligned + size, Ordering::SeqCst);
        self.allocations.fetch_add(1, Ordering::SeqCst);

        (HEAP_START + aligned) as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator does not free individual allocations.
        // This is fine for early kernel init.
    }
}

/// Global heap allocator instance.
#[global_allocator]
static HEAP_ALLOCATOR: BumpAllocator = BumpAllocator::new();

/// Initialize the kernel heap.
///
/// Maps `HEAP_SIZE` bytes at `HEAP_START` using the page table manager.
pub fn init() {
    use crate::memory::paging::{map_page, EntryFlags};
    use crate::memory::pfa::FRAME_ALLOCATOR;

    let mut flags = EntryFlags::new();
    flags.set(EntryFlags::PRESENT);
    flags.set(EntryFlags::WRITABLE);

    // Map each 4 KiB page of the heap
    for offset in (0..HEAP_SIZE).step_by(4096) {
        let virt = HEAP_START + offset;
        let phys = FRAME_ALLOCATOR.lock().allocate()
            .expect("OOM: cannot allocate heap page");
        // SAFETY: We're mapping fresh pages into the kernel's address space.
        unsafe {
            map_page(virt, phys, flags);
        }
    }

    serial_println!(
        "Heap: {} KiB at 0x{:016x}",
        HEAP_SIZE / 1024,
        HEAP_START,
    );
}

/// Test heap allocation with Box and Vec.
pub fn test_alloc() {
    use alloc::boxed::Box;
    use alloc::vec::Vec;

    // Test Box
    let val = Box::new(42u64);
    assert_eq!(*val, 42);
    serial_println!("Heap test: Box<u64> = {}", *val);
    drop(val);

    // Test Vec
    let mut vec = Vec::new();
    for i in 0..100 {
        vec.push(i);
    }
    assert_eq!(vec.len(), 100);
    assert_eq!(vec[42], 42);
    serial_println!("Heap test: Vec<u64> len = {}", vec.len());
    drop(vec);

    serial_println!("Heap allocation tests passed.");
}
