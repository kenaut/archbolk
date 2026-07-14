use crate::serial_println;

/// The register state saved by the interrupt stub.
///
/// Layout matches the push order in `interrupt.asm`:
/// rax, rbx, rcx, rdx, rsi, rdi, rbp, r8-r15.
/// The vector number and error code are above these on the stack,
/// followed by the CPU-pushed `iretq` frame (rip, cs, rflags, rsp, ss).
#[derive(Debug)]
#[repr(C)]
pub struct InterruptFrame {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rbp: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
    /// Vector number (pushed by stub).
    pub vector: u64,
    /// Error code (pushed by CPU or dummy 0).
    pub error_code: u64,
    /// CPU-pushed `iretq` frame follows:
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

/// Names for CPU exception vectors 0-31.
const EXCEPTION_NAMES: &[&str] = &[
    "Divide-by-zero",
    "Debug",
    "Non-maskable Interrupt",
    "Breakpoint",
    "Overflow",
    "Bound Range Exceeded",
    "Invalid Opcode",
    "Device Not Available",
    "Double Fault",
    "Coprocessor Segment Overrun",
    "Invalid TSS",
    "Segment Not Present",
    "Stack-Segment Fault",
    "General Protection Fault",
    "Page Fault",
    "Reserved",
    "x87 Floating-Point Exception",
    "Alignment Check",
    "Machine Check",
    "SIMD Floating-Point Exception",
    "Virtualization Exception",
    "Control Protection Exception",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Hypervisor Injection Exception",
    "VMM Communication Exception",
    "Security Exception",
    "Reserved",
];

/// The common interrupt handler called from assembly.
///
/// `frame` points to the `InterruptFrame` on the stack (rsp at entry).
#[unsafe(no_mangle)]
pub extern "C" fn isr_common_handler(frame: &InterruptFrame) {
    let vector = frame.vector;

    if vector < 32 {
        // CPU exception — print diagnostic and halt
        let name = EXCEPTION_NAMES
            .get(vector as usize)
            .copied()
            .unwrap_or("Unknown Exception");
        serial_println!(
            "!! EXCEPTION: {} (vector {}) at RIP=0x{:x}",
            name,
            vector,
            frame.rip,
        );
        serial_println!("!! Error code: 0x{:x}", frame.error_code);
        serial_println!("!! HALT");
        loop {
            unsafe {
                core::arch::asm!("cli; hlt");
            }
        }
    } else if vector < 48 {
        // Hardware IRQ (remapped to 32-47)
        let irq = (vector - 32) as u8;
        handle_irq(irq);
    }
    // Vectors 48-255 are unused — silently return
}

/// Handle a hardware IRQ (0-15).
fn handle_irq(irq: u8) {
    match irq {
        0 => {
            // PIT timer tick
            super::pit_tick();
            crate::scheduler::timer_tick();
        }
        1 => {
            // PS/2 keyboard IRQ
            crate::keyboard::irq_handler();
        }
        _ => {
            // Unhandled IRQ — just acknowledge
        }
    }

    // Send EOI to PIC
    super::pic::eoi(irq);
}
