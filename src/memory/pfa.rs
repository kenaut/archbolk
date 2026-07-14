use crate::serial_println;
use multiboot2::BootInformation;

/// A physical page frame allocator using a simple stack-based approach.
///
/// The allocator maintains a stack of free frame addresses. Frames are
/// 4 KiB in size (the standard x86_64 page size).
pub struct FrameAllocator {
    /// Stack of free frame base addresses (physical).
    stack: [usize; Self::STACK_SIZE],
    /// Number of free frames currently in the stack.
    top: usize,
}

impl FrameAllocator {
    /// Maximum number of free frames we can track.
    const STACK_SIZE: usize = 65536;

    /// Create a new empty FrameAllocator.
    const fn new() -> Self {
        FrameAllocator {
            stack: [0; Self::STACK_SIZE],
            top: 0,
        }
    }

    /// Allocate a single 4 KiB frame.
    ///
    /// Returns `None` if no frames are available.
    pub fn allocate(&mut self) -> Option<usize> {
        if self.top == 0 {
            return None;
        }
        self.top -= 1;
        Some(self.stack[self.top])
    }

    /// Free a previously allocated frame.
    ///
    /// # Safety
    /// The caller must ensure `addr` is a valid frame-aligned address
    /// that was previously returned by `allocate()`.
    pub unsafe fn free(&mut self, addr: usize) {
        if self.top < Self::STACK_SIZE {
            self.stack[self.top] = addr;
            self.top += 1;
        }
    }
}

/// Global physical frame allocator instance.
pub static FRAME_ALLOCATOR: spin::Mutex<FrameAllocator> = spin::Mutex::new(FrameAllocator::new());

/// Initialize the frame allocator from a raw memory map (PVH boot).
///
/// `entries` is a slice of `(base_addr, size)` pairs for Available RAM regions.
/// Skips the first 1 MiB (reserved for BIOS/boot) and the kernel's own memory region.
pub fn init_from_mmap(entries: &[(u64, u64)]) {
    let kernel_start = 0x100000; // 1 MiB — kernel load address
    let kernel_end = kernel_end_address();

    for &(base, size) in entries {
        let mut base = base as usize;
        let end = (base + size as usize) & !0xfff;

        // Align base up to 4 KiB boundary
        if base % 4096 != 0 {
            base = (base & !0xfff) + 0x1000;
        }

        let mut addr = base;
        while addr < end {
            // Skip the first 1 MiB (BIOS/boot area)
            if addr < 0x100000 {
                addr = 0x100000;
                continue;
            }

            // Skip the kernel's memory region
            if addr >= kernel_start && addr < kernel_end {
                let aligned_end = (kernel_end + 0xfff) & !0xfff;
                addr = aligned_end;
                continue;
            }

            // SAFETY: We're adding known-available frames from the
            // bootloader's memory map.
            unsafe {
                FRAME_ALLOCATOR.lock().free(addr);
            }
            addr += 4096;
        }
    }

    let total = FRAME_ALLOCATOR.lock().top;
    serial_println!("Frame allocator: {} free frames ({} KiB)", total, total * 4);
}

/// Initialize the frame allocator from the Multiboot2 memory map.
///
/// Scans all available memory regions and adds usable frames to the
/// allocator stack. Skips the first 1 MiB (reserved for BIOS/boot) and
/// the kernel's own memory region.
pub fn init(boot_info: &BootInformation) {
    let kernel_start = 0x100000; // 1 MiB — kernel load address
    let kernel_end = kernel_end_address();

    if let Some(mmap_tag) = boot_info.memory_map_tag() {
        for area in mmap_tag.memory_areas() {
            if area.typ() != multiboot2::MemoryAreaType::Available {
                continue;
            }

            let mut base = area.start_address() as usize;
            let end = (area.start_address() + area.size()) as usize;

            // Align base up to 4 KiB boundary
            if base % 4096 != 0 {
                base = (base & !0xfff) + 0x1000;
            }

            // Align end down to 4 KiB boundary
            let end = end & !0xfff;

            let mut addr = base;
            while addr < end {
                // Skip the first 1 MiB (BIOS/boot area)
                if addr < 0x100000 {
                    addr = 0x100000;
                    continue;
                }

                // Skip the kernel's memory region
                if addr >= kernel_start && addr < kernel_end {
                    // Align kernel_end up to 4 KiB boundary so subsequent
                    // frame addresses are page-aligned.
                    let aligned_end = (kernel_end + 0xfff) & !0xfff;
                    addr = aligned_end;
                    continue;
                }

                // SAFETY: We're adding known-available frames from the
                // bootloader's memory map.
                unsafe {
                    FRAME_ALLOCATOR.lock().free(addr);
                }
                addr += 4096;
            }
        }
    }

    let total = FRAME_ALLOCATOR.lock().top;
    serial_println!("Frame allocator: {} free frames ({} KiB)", total, total * 4);
}

/// Get the end address of the kernel image.
///
/// Uses symbols defined in the linker script.
fn kernel_end_address() -> usize {
    unsafe extern "C" {
        static _kernel_end: u8;
    }
    // SAFETY: _kernel_end is a linker symbol, safe to read its address.
    unsafe { &_kernel_end as *const u8 as usize }
}

/// Return the number of free frames currently in the allocator.
pub fn frame_count() -> usize {
    FRAME_ALLOCATOR.lock().top
}
