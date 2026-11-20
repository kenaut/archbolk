use core::arch::asm;

/// PIT I/O ports.
const PIT_CHANNEL0: u16 = 0x40;
const PIT_COMMAND: u16 = 0x43;

/// PIT base frequency: 1.193182 MHz.
const PIT_FREQUENCY: u64 = 1_193_182;

/// Configure PIT channel 0 in mode 2 (rate generator) at the given frequency.
///
/// Returns the actual frequency achieved (which may differ slightly due to
/// integer divisor rounding).
pub fn init(target_hz: u64) -> u64 {
    let divisor = (PIT_FREQUENCY / target_hz) as u16;
    let actual_hz = PIT_FREQUENCY / divisor as u64;

    unsafe {
        // Command byte: channel 0, lobyte/hibyte access, mode 2 (rate generator), binary mode
        asm!("out dx, al", in("dx") PIT_COMMAND, in("al") 0x36u8);

        // Send divisor (low byte then high byte)
        asm!("out dx, al", in("dx") PIT_CHANNEL0, in("al") (divisor & 0xFF) as u8);
        asm!("out dx, al", in("dx") PIT_CHANNEL0, in("al") (divisor >> 8) as u8);
    }

    actual_hz
}

/// Read the current PIT counter value (0-65535).
///
/// Returns the number of ticks remaining before the next IRQ.
#[allow(dead_code)]
pub fn read_count() -> u16 {
    unsafe {
        // Latch command: channel 0
        asm!("out dx, al", in("dx") PIT_COMMAND, in("al") 0x00u8);

        let low: u8;
        let high: u8;
        asm!("in al, dx", out("al") low, in("dx") PIT_CHANNEL0);
        asm!("in al, dx", out("al") high, in("dx") PIT_CHANNEL0);

        u16::from_le_bytes([low, high])
    }
}
