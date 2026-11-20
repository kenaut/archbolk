use core::arch::asm;

/// I/O ports for the master and slave PICs.
const PIC1_COMMAND: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_COMMAND: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;

/// End-of-interrupt command.
const PIC_EOI: u8 = 0x20;

/// ICW1: initialization command word 1.
const ICW1_INIT: u8 = 0x11; // Initialization + ICW4 required
const ICW4_8086: u8 = 0x01; // 8086/88 mode

/// Remap the PIC IRQs 0-15 to interrupt vectors 32-47.
///
/// This avoids overlap with CPU exceptions (vectors 0-31).
pub fn init() {
    unsafe {
        // Start initialization sequence (cascade mode)
        asm!("out dx, al", in("dx") PIC1_COMMAND, in("al") ICW1_INIT);
        asm!("out dx, al", in("dx") PIC2_COMMAND, in("al") ICW1_INIT);

        // ICW2: remap IRQ base vectors
        asm!("out dx, al", in("dx") PIC1_DATA, in("al") 0x20u8); // IRQ 0-7 -> vectors 32-39
        asm!("out dx, al", in("dx") PIC2_DATA, in("al") 0x28u8); // IRQ 8-15 -> vectors 40-47

        // ICW3: tell master PIC that slave is at IRQ2, tell slave its cascade identity
        asm!("out dx, al", in("dx") PIC1_DATA, in("al") 0x04u8); // Slave at IRQ2 (bit 2)
        asm!("out dx, al", in("dx") PIC2_DATA, in("al") 0x02u8); // Slave cascade identity = 2

        // ICW4: 8086 mode
        asm!("out dx, al", in("dx") PIC1_DATA, in("al") ICW4_8086);
        asm!("out dx, al", in("dx") PIC2_DATA, in("al") ICW4_8086);

        // Mask all IRQs by default — individual drivers will unmask as needed.
        asm!("out dx, al", in("dx") PIC1_DATA, in("al") 0xFFu8);
        asm!("out dx, al", in("dx") PIC2_DATA, in("al") 0xFFu8);
    }
}

/// Send End-of-Interrupt to the PIC(s).
///
/// For IRQs 0-7, only the master PIC is notified.
/// For IRQs 8-15, both master and slave are notified.
pub fn eoi(irq: u8) {
    if irq >= 8 {
        unsafe {
            asm!("out dx, al", in("dx") PIC2_COMMAND, in("al") PIC_EOI);
        }
    }
    unsafe {
        asm!("out dx, al", in("dx") PIC1_COMMAND, in("al") PIC_EOI);
    }
}

/// Mask (disable) a specific IRQ.
#[allow(dead_code)]
pub fn mask(irq: u8) {
    let port = if irq < 8 { PIC1_DATA } else { PIC2_DATA };
    let irq_bit = 1 << (irq & 0x07);
    unsafe {
        let mut mask: u8;
        asm!("in al, dx", out("al") mask, in("dx") port);
        mask |= irq_bit;
        asm!("out dx, al", in("dx") port, in("al") mask);
    }
}

/// Unmask (enable) a specific IRQ.
pub fn unmask(irq: u8) {
    let port = if irq < 8 { PIC1_DATA } else { PIC2_DATA };
    let irq_bit = 1 << (irq & 0x07);
    unsafe {
        let mut mask: u8;
        asm!("in al, dx", out("al") mask, in("dx") port);
        mask &= !irq_bit;
        asm!("out dx, al", in("dx") port, in("al") mask);
    }
}
