#![no_std]
#![no_main]

extern crate alloc;

mod vga_buffer;
mod serial;
mod memory;
mod interrupts;
mod keyboard;
mod shell;
mod scheduler;
mod syscall;
mod gdt;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use core::fmt::Write;
    if let Some(location) = info.location() {
        let _ = write!(
            *crate::serial::SERIAL.lock(),
            "PANIC at {}:{}: {}",
            location.file(),
            location.line(),
            info.message()
        );
    } else {
        let _ = write!(
            *crate::serial::SERIAL.lock(),
            "PANIC: {}",
            info.message()
        );
    }
    loop {}
}

/// Multiboot2 header — must be in the first 8192 bytes of the kernel image.
#[unsafe(link_section = ".multiboot2_header")]
#[used]
static MULTIBOOT2_HEADER: [u8; 24] = {
    let magic: u32 = 0xe85250d6;
    let arch: u32 = 0;
    let len: u32 = 24;
    let checksum: u32 = 0u32.wrapping_sub(magic).wrapping_sub(arch).wrapping_sub(len);

    [
        (magic >> 0) as u8, (magic >> 8) as u8, (magic >> 16) as u8, (magic >> 24) as u8,
        (arch >> 0) as u8, (arch >> 8) as u8, (arch >> 16) as u8, (arch >> 24) as u8,
        (len >> 0) as u8, (len >> 8) as u8, (len >> 16) as u8, (len >> 24) as u8,
        (checksum >> 0) as u8, (checksum >> 8) as u8, (checksum >> 16) as u8, (checksum >> 24) as u8,
        0, 0, 0, 0,
        8, 0, 0, 0,
    ]
};

/// Xen HVM start info magic value: "xEn3" with 0x80 bit of 'E' set.
const XEN_HVM_START_MAGIC: u32 = 0x336ec578;

/// Rust entry point — called from the 64-bit boot assembly.
///
/// `boot_info_addr` is the physical address of boot info structure:
///   - Multiboot2 boot: address of Multiboot2 info structure
///   - PVH boot: address of hvm_start_info structure
///
/// We detect the boot protocol by checking the magic value at the address.
#[unsafe(no_mangle)]
pub extern "C" fn _start_rust(boot_info_addr: usize) -> ! {
    println!("archbolk kernel booting...");
    println!("Running in 64-bit long mode.");

    // Initialize serial port
    serial::SERIAL.lock().init();
    serial::SERIAL.unlock();
    serial_println!("archbolk serial output OK");

    // Detect boot protocol by reading magic at boot_info_addr
    let magic: u32 = unsafe { core::ptr::read_volatile(boot_info_addr as *const u32) };
    let is_pvh = magic == XEN_HVM_START_MAGIC;
    serial_println!(
        "Boot protocol: {} (magic=0x{:08x})",
        if is_pvh { "PVH" } else { "Multiboot2" },
        magic
    );

    // Initialize memory subsystem
    memory::init(boot_info_addr, is_pvh);

    // Replace boot GDT with full table (user segments + TSS)
    gdt::init();

    // Initialize interrupt subsystem (IDT, PIC, PIT)
    interrupts::init();
    serial_println!("Interrupt subsystem initialized");

    // Initialize syscall MSRs
    syscall::init();

    // Unmask IRQ0 (PIT timer) and IRQ1 (keyboard)
    interrupts::pic::unmask(0);
    interrupts::pic::unmask(1);

    // Initialize PS/2 keyboard
    keyboard::init();

    // Initialize scheduler with idle task
    {
        static mut IDLE_STACK: core::mem::MaybeUninit<[u8; 16384]> =
            core::mem::MaybeUninit::uninit();
        let ptr = core::ptr::addr_of_mut!(IDLE_STACK) as *mut u8;
        let stack = unsafe { core::slice::from_raw_parts_mut(ptr, 16384) };
        scheduler::SCHEDULER.lock().init(idle_task, stack);
    }

    // Spawn a test task that prints periodically
    {
        static mut TASK1_STACK: core::mem::MaybeUninit<[u8; 16384]> =
            core::mem::MaybeUninit::uninit();
        let ptr = core::ptr::addr_of_mut!(TASK1_STACK) as *mut u8;
        let stack = unsafe { core::slice::from_raw_parts_mut(ptr, 16384) };
        scheduler::SCHEDULER.lock().spawn("test1", test_task_1, stack);
    }

    // Create a ring-3 user demo task.
    spawn_user_demo();

    serial_println!("Scheduler ready. Type on the serial console:");

    // Test: write directly to serial port
    {
        use core::fmt::Write;
        let serial = crate::serial::SERIAL.lock();
        let _ = serial.write_str("[init] Testing serial...\n");
        crate::serial::SERIAL.unlock();
    }

    // Test serial input before enabling interrupts
    {
        use core::fmt::Write;
        let serial = crate::serial::SERIAL.lock();
        let _ = serial.write_str("[init] Testing serial input...\n");
        // Drain any pending input
        while serial.read_byte().is_some() {}
        let _ = serial.write_str("[init] Serial input drained.\n");
        crate::serial::SERIAL.unlock();
    }

    // Enable interrupts — scheduler will take over
    unsafe {
        core::arch::asm!("sti");
    }

    // This becomes the idle task's main loop
    idle_task()
}

/// Idle task: runs when nothing else is ready.
fn idle_task() -> ! {
    let mut shell = shell::Shell::new();
    shell::Shell::print_prompt(&shell);

    loop {
        // Check for keyboard input and feed the shell
        while let Some(ch) = keyboard::read_key() {
            shell.handle_key(ch);
        }
        // Also check serial port for input (e.g. from QEMU -nographic)
        // Read one byte at a time, releasing the lock between each
        loop {
            let byte = {
                let serial = crate::serial::SERIAL.lock();
                let b = serial.read_byte();
                crate::serial::SERIAL.unlock();
                b
            };
            match byte {
                Some(b) => shell.handle_key(b),
                None => break,
            }
        }
        // Preemptive scheduling is driven by the timer IRQ; no explicit
        // yield is needed. Pause briefly to reduce busy-loop overhead.
        unsafe {
            core::arch::asm!("pause");
        }
    }
}

/// Test task: prints a counter periodically to demonstrate multitasking.
fn test_task_1() -> ! {
    let mut count: u64 = 0;
    loop {
        count += 1;
        if count % 50000 == 0 {
            serial_println!("[task1] count={}", count);
        }
        // The timer IRQ preempts us; no explicit yield needed.
        unsafe {
            core::arch::asm!("pause");
        }
    }
}

/// Map the ring-3 demo's code page and a user stack, then spawn it.
fn spawn_user_demo() {
    use memory::paging::{map_page, EntryFlags};

    // Use PML4[1] (0x200_0000_0000) — a canonical low-half address with a
    // fresh page-table hierarchy, avoiding the kernel's identity huge pages.
    const USER_CODE_VADDR: usize = 0x200_0000_0000;
    const USER_STACK_VADDR: usize = 0x200_0010_0000; // stack grows down from here

    unsafe extern "C" {
        fn user_demo();
    }

    let mut user_flags = EntryFlags::new();
    user_flags.set(EntryFlags::PRESENT);
    user_flags.set(EntryFlags::WRITABLE);
    user_flags.set(EntryFlags::USER);

    unsafe {
        // Map the page containing user_demo into user space at USER_CODE_VADDR.
        let user_fn: unsafe extern "C" fn() = user_demo;
        let user_addr = user_fn as usize;
        let code_phys = user_addr & !0xfff;
        let code_off = user_addr & 0xfff;
        map_page(USER_CODE_VADDR, code_phys, user_flags);

        // Allocate and map one user-stack page.
        let stack_frame = memory::pfa::FRAME_ALLOCATOR
            .lock()
            .allocate()
            .expect("OOM: user stack");
        map_page(USER_STACK_VADDR, stack_frame, user_flags);
        let user_stack_top = USER_STACK_VADDR + 0x1000;

        // Kernel stack for ring-0 transitions (interrupt/syscall frames).
        static mut USER_KERNEL_STACK: core::mem::MaybeUninit<[u8; 16384]> =
            core::mem::MaybeUninit::uninit();
        let kptr = core::ptr::addr_of_mut!(USER_KERNEL_STACK) as *mut u8;
        let kernel_stack = core::slice::from_raw_parts_mut(kptr, 16384);

        let entry = USER_CODE_VADDR + code_off;
        let id = scheduler::SCHEDULER
            .lock()
            .spawn_user("user_demo", entry, user_stack_top, kernel_stack);
        serial_println!(
            "User demo spawned: id={} entry=0x{:x} ustack=0x{:x}",
            id,
            entry,
            user_stack_top
        );
    }
}

/// Dynamically spawned worker: prints a periodic heartbeat until terminated.
pub fn spawned_task() -> ! {
    static NEXT_ID: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
    let id = NEXT_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    let mut count: u64 = 0;
    loop {
        count += 1;
        if count % 2_000_000 == 0 {
            serial_println!("[worker#{}] tick {}", id, count);
        }
        unsafe {
            core::arch::asm!("pause");
        }
    }
}
