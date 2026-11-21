use core::arch::asm;

/// MSR addresses for syscall configuration.
const MSR_STAR: u32 = 0xC000_0081;
const MSR_LSTAR: u32 = 0xC000_0082;
const MSR_CSTAR: u32 = 0xC000_0083;
const MSR_SF_MASK: u32 = 0xC000_0084;

/// Syscall numbers (Linux x86-64 compatible subset).
pub const SYS_WRITE: u64 = 1;
pub const SYS_BRK: u64 = 12;
pub const SYS_SCHED_YIELD: u64 = 24;
pub const SYS_NANOSLEEP: u64 = 35;
pub const SYS_GETPID: u64 = 39;
pub const SYS_EXIT: u64 = 60;

/// Saved register block layout — matches the push order in `syscall_entry`.
#[repr(C)]
pub struct SyscallFrame {
    pub rax: u64,   // syscall number (overwritten with return value)
    pub rcx: u64,   // return RIP (set by SYSCALL)
    pub r11: u64,   // original RFLAGS (set by SYSCALL)
    pub rdi: u64,   // arg1
    pub rsi: u64,   // arg2
    pub rdx: u64,   // arg3
    pub r10: u64,   // arg4
    pub r8: u64,    // arg5
    pub r9: u64,    // arg6
    pub rbx: u64,
    pub rbp: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
}

/// MSR address for EFER (Extended Feature Enable Register).
const MSR_EFER: u32 = 0xC000_0080;

/// EFER bit for System Call Extensions (enables SYSCALL/SYSRET).
const EFER_SCE: u64 = 1;

/// Configure MSRs for the SYSCALL/SYSRET mechanism.
///
/// Must be called once during kernel init, after the GDT is set up.
///
/// STAR layout:
///   [63:48] = SYSRET CS base (kernel data segment + 8 → user code segment)
///   [47:32] = SYSCALL CS base (kernel code segment)
///   [31:0]  = system call target base (unused in x86_64)
///
/// For our GDT:
///   selector 0x08 = kernel code (ring 0)
///   selector 0x10 = kernel data (ring 0)
///   SYSRET CS = STAR[63:48] = 0x10 + 8 = 0x18 (user code, ring 3)
///   SYSCALL CS = STAR[47:32] = 0x08 (kernel code, ring 0)
///   SYSRET SS = STAR[63:48] + 8 = 0x20 (user data, ring 3)
pub fn init() {
    let syscall_entry_ptr = syscall_entry as *const () as u64;

    // Enable EFER.SCE (System Call Extensions) — required for SYSCALL/SYSRET
    let efer = rdmsr(MSR_EFER);
    wrmsr(MSR_EFER, efer | EFER_SCE);

    // STAR: SYSCALL CS = 0x08 (kernel code).
    // SYSRET (64-bit): CS = STAR[63:48]+16 = 0x20 (user code),
    //                  SS = STAR[63:48]+8  = 0x18 (user data).
    let star = (0x10u64 << 48) | (0x08u64 << 32);
    wrmsr(MSR_STAR, star);

    // LSTAR: syscall entry point RIP
    wrmsr(MSR_LSTAR, syscall_entry_ptr);

    // CSTAR: compatibility mode syscall entry (unused in long mode, but set it)
    wrmsr(MSR_CSTAR, syscall_entry_ptr);

    // SF_MASK: clear IF (interrupts), DF (direction), and TF (trap) on syscall entry.
    // RFLAGS & ~SF_MASK is applied. We clear IF so interrupts are disabled
    // during syscall handling.
    wrmsr(MSR_SF_MASK, 0x200); // clear IF (bit 9)

    crate::serial_println!("Syscall: entry at 0x{:x}", syscall_entry_ptr);
}

/// Read from an x86_64 MSR.
fn rdmsr(msr: u32) -> u64 {
    let (low, high): (u32, u32);
    unsafe {
        asm!("rdmsr", in("ecx") msr, out("eax") low, out("edx") high, options(nostack, preserves_flags));
    }
    (low as u64) | ((high as u64) << 32)
}

unsafe extern "C" {
    fn syscall_entry();
}

/// Write to an x86_64 MSR.
fn wrmsr(msr: u32, value: u64) {
    let low = (value & 0xFFFF_FFFF) as u32;
    let high = (value >> 32) as u32;
    unsafe {
        asm!("wrmsr", in("ecx") msr, in("eax") low, in("edx") high, options(nostack, preserves_flags));
    }
}

/// The Rust syscall handler — called from `syscall_entry` assembly.
///
/// `frame` points to the saved register block on the kernel stack.
/// The return value is written into `frame.rax` so the assembly epilogue
/// pops it as the return value.
#[unsafe(no_mangle)]
pub extern "C" fn syscall_handler(frame: &mut SyscallFrame) {
    let nr = frame.rax;

    match nr {
        SYS_WRITE => {
            // sys_write(fd: i32, buf: *const u8, count: usize) -> isize
            let _fd = frame.rdi as i32;
            let buf = frame.rsi as *const u8;
            let count = frame.rdx as usize;

            // Only fd 1 (stdout) is supported — write to serial
            let slice = if _fd == 1 || _fd == 2 {
                unsafe { core::slice::from_raw_parts(buf, count) }
            } else {
                frame.rax = core::ffi::c_long::MAX as u64; // EBADF
                return;
            };

            // Write to serial port
            {
                use core::fmt::Write;
                let s = core::str::from_utf8(slice).unwrap_or("");
                let _ = write!(*crate::serial::SERIAL.lock(), "{}", s);
                crate::serial::SERIAL.unlock();
            }

            frame.rax = count as u64;
        }

        SYS_GETPID => {
            // sys_getpid() -> pid_t (the current task ID)
            frame.rax = crate::scheduler::SCHEDULER.lock().current_id() as u64;
        }

        SYS_SCHED_YIELD => {
            // sys_sched_yield(): no-op. The timer IRQ already provides
            // round-robin preemption; forcing a switch from syscall context
            // (mid-frame on the user stack) races with the normal IRQ path.
            frame.rax = 0;
        }

        SYS_NANOSLEEP => {
            // Simplified nanosleep: rdi = milliseconds to sleep, returns 0.
            // (Linux passes struct timespec*, but ms is easier for our demo.)
            let ms = frame.rdi;
            let ticks = core::cmp::max(1, ms / 10); // PIT runs at ~100 Hz
            frame.rax = 0;
            // Busy-wait on the PIT tick while continuously requesting a
            // reschedule so other tasks run. Avoids blocking-state races in
            // syscall context (where the CPU is already in ring 0).
            let target = crate::interrupts::get_tick_count() + ticks;
            // We're running on the user stack in syscall context; defer context
            // switches so the timer can advance ticks without switching us out
            // mid-frame. Enable interrupts so the PIT keeps ticking.
            crate::scheduler::set_defer_schedule(true);
            unsafe {
                asm!("sti");
            }
            while crate::interrupts::get_tick_count() < target {
                unsafe {
                    asm!("pause");
                }
            }
            unsafe {
                asm!("cli");
            }
            crate::scheduler::set_defer_schedule(false);
        }

        SYS_BRK => {
            // sys_brk(addr): extend/shrink the user heap. addr=0 queries.
            let id = crate::scheduler::SCHEDULER.lock().current_id();
            let mut sched = crate::scheduler::SCHEDULER.lock();
            frame.rax = if frame.rdi == 0 {
                sched.brk(id) as u64
            } else {
                sched.set_brk(id, frame.rdi as usize) as u64
            };
        }

        SYS_EXIT => {
            // sys_exit(status: i32): mark the current task terminated.
            // The timer IRQ will then schedule it away (pick_next skips
            // terminated tasks), so we just wait with interrupts enabled.
            let status = frame.rdi as i32;
            let id = crate::scheduler::SCHEDULER.lock().current_id();
            crate::scheduler::SCHEDULER.lock().kill(id);
            crate::serial_println!("syscall: task {} exit({})", id, status);
            // Re-enable interrupts so the timer IRQ can schedule us away.
            loop {
                unsafe {
                    asm!("sti; hlt");
                }
            }
        }

        _ => {
            // Unknown syscall — return -ENOSYS
            crate::serial_println!("syscall: unknown nr={}", nr);
            frame.rax = (-38isize) as u64; // -ENOSYS
        }
    }
}
