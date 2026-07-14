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
        let ptr = unsafe { core::ptr::addr_of_mut!(IDLE_STACK) as *mut u8 };
        let stack = unsafe { core::slice::from_raw_parts_mut(ptr, 16384) };
        scheduler::SCHEDULER.lock().init(idle_task, stack);
    }

    // Spawn a test task that prints periodically
    {
        static mut TASK1_STACK: core::mem::MaybeUninit<[u8; 16384]> =
            core::mem::MaybeUninit::uninit();
        let ptr = unsafe { core::ptr::addr_of_mut!(TASK1_STACK) as *mut u8 };
        let stack = unsafe { core::slice::from_raw_parts_mut(ptr, 16384) };
        scheduler::SCHEDULER.lock().spawn(test_task_1, stack);
    }

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
    let mut debug_count: u64 = 0;

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
                Some(b) => {
                    debug_count += 1;
                    shell.handle_key(b);
                }
                None => break,
            }
        }
        // Debug: periodically show we're alive and polling
        debug_count += 1;
        if debug_count % 50000 == 0 {
            // Check LSR register directly
            let lsr: u8;
            unsafe { core::arch::asm!("in al, dx", out("al") lsr, in("dx") 0x3FDu16); }
            if lsr & 0x01 != 0 {
                // Write directly without serial_println! to avoid lock issues
                {
                    use core::fmt::Write;
                    let s = crate::serial::SERIAL.lock();
                    let _ = s.write_str("[dbg] LSR says data ready! lsr=0x");
                    // Can't easily print hex without format_args, skip for now
                    let _ = s.write_str("\n");
                    crate::serial::SERIAL.unlock();
                }
                let ch: u8;
                unsafe { core::arch::asm!("in al, dx", out("al") ch, in("dx") 0x3F8u16); }
                {
                    use core::fmt::Write;
                    let s = crate::serial::SERIAL.lock();
                    let _ = s.write_str("[dbg] read byte: 0x");
                    let _ = s.write_str("\n");
                    crate::serial::SERIAL.unlock();
                }
            }
        }
        // Check if scheduler wants to switch tasks
        scheduler::reschedule();
        // Yield to other tasks
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
        // Yield to let other tasks run
        scheduler::reschedule();
    }
}
