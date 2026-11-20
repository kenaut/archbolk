use core::sync::atomic::{AtomicBool, Ordering};

// ── PS/2 8042 controller ports ──────────────────────────────────────────
const DATA_PORT: u16 = 0x60;
const STATUS_PORT: u16 = 0x64;
const COMMAND_PORT: u16 = 0x64;

// ── 8042 status register bits ───────────────────────────────────────────
const STATUS_OUTPUT_FULL: u8 = 1 << 0;
const STATUS_INPUT_FULL: u8 = 1 << 1;

// ── 8042 commands ───────────────────────────────────────────────────────
const CMD_READ_CONFIG: u8 = 0x20;
const CMD_WRITE_CONFIG: u8 = 0x60;
const CMD_DISABLE_PORT2: u8 = 0xA7;
const CMD_ENABLE_PORT1: u8 = 0xAE;
const CMD_SELF_TEST: u8 = 0xAA;
const CMD_PORT1_TEST: u8 = 0xAB;

// ── Keyboard commands ───────────────────────────────────────────────────
const KBD_CMD_SET_LEDS: u8 = 0xED;
#[allow(dead_code)]
const KBD_CMD_ECHO: u8 = 0xEE;
const KBD_CMD_SET_SCANSET: u8 = 0xF0;
const KBD_CMD_ENABLE: u8 = 0xF4;
#[allow(dead_code)]
const KBD_CMD_DISABLE: u8 = 0xF5;
const KBD_CMD_RESET: u8 = 0xFF;

const KBD_ACK: u8 = 0xFA;
const KBD_SELF_TEST_OK: u8 = 0xAA;

// ── Lock-free ring buffer ───────────────────────────────────────────────
/// Single-producer (ISR), single-consumer (main loop) ring buffer.
/// Uses atomic head/tail so no mutex is needed.
const RING_CAPACITY: usize = 256;

struct RingBuffer {
    buf: [core::cell::UnsafeCell<u8>; RING_CAPACITY],
    /// Next write position (written only by ISR).
    head: core::sync::atomic::AtomicUsize,
    /// Next read position (written only by main loop).
    tail: core::sync::atomic::AtomicUsize,
}

unsafe impl Sync for RingBuffer {}

impl RingBuffer {
    const fn new() -> Self {
        const INIT: core::cell::UnsafeCell<u8> = core::cell::UnsafeCell::new(0);
        RingBuffer {
            buf: [INIT; RING_CAPACITY],
            head: core::sync::atomic::AtomicUsize::new(0),
            tail: core::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Push from ISR context. Returns false if full.
    fn push(&self, val: u8) -> bool {
        let head = self.head.load(core::sync::atomic::Ordering::Relaxed);
        let tail = self.tail.load(core::sync::atomic::Ordering::Acquire);
        let next = (head + 1) % RING_CAPACITY;
        if next == tail {
            return false;
        }
        unsafe { *self.buf[head].get() = val; }
        self.head.store(next, core::sync::atomic::Ordering::Release);
        true
    }

    /// Pop from main loop. Returns None if empty.
    fn pop(&self) -> Option<u8> {
        let tail = self.tail.load(core::sync::atomic::Ordering::Relaxed);
        let head = self.head.load(core::sync::atomic::Ordering::Acquire);
        if tail == head {
            return None;
        }
        let val = unsafe { *self.buf[tail].get() };
        self.tail.store((tail + 1) % RING_CAPACITY, core::sync::atomic::Ordering::Release);
        Some(val)
    }
}

// ── Global state ────────────────────────────────────────────────────────
static RING: RingBuffer = RingBuffer::new();
static KEYBOARD_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Whether the left/right Shift, Ctrl, Alt are held.
static SHIFT: AtomicBool = AtomicBool::new(false);
static CTRL: AtomicBool = AtomicBool::new(false);
static ALT: AtomicBool = AtomicBool::new(false);
/// CapsLock, NumLock, ScrollLock toggles.
static CAPS_LOCK: AtomicBool = AtomicBool::new(false);
static NUM_LOCK: AtomicBool = AtomicBool::new(false);
static SCROLL_LOCK: AtomicBool = AtomicBool::new(false);

// ── I/O helpers ─────────────────────────────────────────────────────────
fn inb(port: u16) -> u8 {
    let val: u8;
    unsafe { core::arch::asm!("in al, dx", out("al") val, in("dx") port); }
    val
}

fn outb(port: u16, val: u8) {
    unsafe { core::arch::asm!("out dx, al", in("dx") port, in("al") val); }
}

/// Wait until the 8042 input buffer is empty (ready to send a command).
fn wait_input_empty() {
    loop {
        let status = inb(STATUS_PORT);
        if status & STATUS_INPUT_FULL == 0 {
            break;
        }
    }
}

/// Wait until the 8042 output buffer has data.
fn wait_output_full() -> u8 {
    loop {
        let status = inb(STATUS_PORT);
        if status & STATUS_OUTPUT_FULL != 0 {
            return inb(DATA_PORT);
        }
    }
}

/// Send a command byte to the 8042 controller.
fn send_command(cmd: u8) {
    wait_input_empty();
    outb(COMMAND_PORT, cmd);
}

/// Send a data byte to the 8042 controller's data port.
fn send_data(val: u8) {
    wait_input_empty();
    outb(DATA_PORT, val);
}

/// Read a byte from the keyboard (blocking, with timeout).
fn read_keyboard_response() -> Option<u8> {
    // Poll with a simple timeout (~50k iterations)
    for _ in 0..50000 {
        let status = inb(STATUS_PORT);
        if status & STATUS_OUTPUT_FULL != 0 {
            return Some(inb(DATA_PORT));
        }
    }
    None
}

/// Send a command to the keyboard device (via data port) and wait for ACK.
fn keyboard_command(cmd: u8) -> bool {
    send_data(cmd);
    match read_keyboard_response() {
        Some(KBD_ACK) => true,
        Some(0xFE) => {
            // Resend requested — try once more
            send_data(cmd);
            read_keyboard_response() == Some(KBD_ACK)
        }
        _ => false,
    }
}

/// Send a command with data byte to the keyboard and wait for ACK.
fn keyboard_command_data(cmd: u8, data: u8) -> bool {
    send_data(cmd);
    if read_keyboard_response() != Some(KBD_ACK) {
        return false;
    }
    send_data(data);
    read_keyboard_response() == Some(KBD_ACK)
}

// ── Initialization ──────────────────────────────────────────────────────
/// Initialize the PS/2 keyboard controller and device.
pub fn init() {
    // 1. Disable both PS/2 ports
    send_command(CMD_DISABLE_PORT2); // disable port 2 (mouse)
    send_command(0xAD);              // disable port 1 (keyboard)

    // 2. Flush output buffer
    inb(DATA_PORT);

    // 3. Read and modify configuration byte
    send_command(CMD_READ_CONFIG);
    let config = wait_output_full();
    // Enable interrupts for port 1, disable port 2
    let new_config = (config & !0x30) | 0x01; // bit 0 = enable IRQ1
    send_command(CMD_WRITE_CONFIG);
    send_data(new_config);

    // 4. Controller self-test
    send_command(CMD_SELF_TEST);
    if wait_output_full() != 0x55 {
        crate::serial_println!("Keyboard: 8042 self-test FAILED");
        return;
    }

    // 5. Test port 1
    send_command(CMD_PORT1_TEST);
    if wait_output_full() != 0x00 {
        crate::serial_println!("Keyboard: port 1 test FAILED");
        return;
    }

    // 6. Enable port 1
    send_command(CMD_ENABLE_PORT1);

    // 7. Reset keyboard
    if !keyboard_command(KBD_CMD_RESET) {
        crate::serial_println!("Keyboard: reset FAILED (no ACK)");
        return;
    }
    // After reset, keyboard sends BAT result (0xAA = OK) then its ID (0xAB, 0x83)
    let bat = read_keyboard_response();
    if bat != Some(KBD_SELF_TEST_OK) {
        crate::serial_println!("Keyboard: BAT failed (got {:?})", bat);
        return;
    }
    // Read keyboard ID (2 bytes)
    let _id1 = read_keyboard_response();
    let _id2 = read_keyboard_response();

    // 8. Set scan code set 1 (default, but be explicit)
    keyboard_command_data(KBD_CMD_SET_SCANSET, 1);

    // 9. Enable scanning
    if !keyboard_command(KBD_CMD_ENABLE) {
        crate::serial_println!("Keyboard: enable FAILED");
        return;
    }

    KEYBOARD_INITIALIZED.store(true, Ordering::Release);
    crate::serial_println!("Keyboard: initialized");
}

// ── Scan code set 1 → ASCII mapping ─────────────────────────────────────
/// Map a Set 1 scancode (make code, bit 7 clear) to its ASCII character.
fn scancode_to_ascii(scancode: u8, shift: bool, caps: bool) -> Option<u8> {
    // Set 1 make-code to ASCII mapping (US keyboard layout).
    // Index by scancode byte. 0 = unmapped.
    const SET1_BASE: &[u8] = b"\x00\x1b1234567890-=\x08\tqwertyuiop[]\n\x00asdfghjkl;'`\x00\\zxcvbnm,./\x00*\x00 \x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x007-456+1230.\x00\x00\x00\x00";
    // Shifted mapping
    const SET1_SHIFTED: &[u8] = b"\x00\x1b!@#$%^&*()_+\x08\tQWERTYUIOP{}\n\x00ASDFGHJKL:\"~\x00|ZXCVBNM<>?\x00*\x00 \x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x007-456+1230.\x00\x00\x00\x00";

    let idx = scancode as usize;
    if idx >= SET1_BASE.len() {
        return None;
    }

    let shifted = shift ^ caps;
    let ch = if shifted { SET1_SHIFTED[idx] } else { SET1_BASE[idx] };

    if ch == 0 {
        return None;
    }
    Some(ch)
}

/// Called from the IRQ1 handler (interrupt context).
/// Reads the scancode from the data port and enqueues it.
pub fn irq_handler() {
    let scancode = inb(DATA_PORT);

    // QEMU's send-key sends two bytes per keypress:
    //   byte 1: make code (unpredictable value)
    //   byte 2: break code = set1_scancode | 0x80
    // We use the break code's key (bit 7 set) for reliable identification.
    if scancode & 0x80 != 0 {
        // Break code — key = set1 scancode
        let key = scancode & 0x7F;
        match key {
            // Modifier keys (Set 1) — release
            0x2A | 0x36 => SHIFT.store(false, Ordering::Relaxed),
            0x1D => CTRL.store(false, Ordering::Relaxed),
            0x38 => ALT.store(false, Ordering::Relaxed),
            // Toggle keys — ignore break (handled on make)
            0x3A | 0x45 | 0x46 => {}
            // Printable keys — enqueue
            _ => {
                let shift = SHIFT.load(Ordering::Relaxed);
                let caps = CAPS_LOCK.load(Ordering::Relaxed);
                if let Some(ch) = scancode_to_ascii(key, shift, caps) {
                    RING.push(ch);
                }
            }
        }
        return;
    }

    // Make code — update modifier state on press
    match scancode {
        0x2A | 0x36 => SHIFT.store(true, Ordering::Relaxed),
        0x1D => CTRL.store(true, Ordering::Relaxed),
        0x38 => ALT.store(true, Ordering::Relaxed),
        // Toggle keys — toggle on make (press), ignore break
        0x3A => {
            let new = !CAPS_LOCK.load(Ordering::Relaxed);
            CAPS_LOCK.store(new, Ordering::Relaxed);
            update_leds();
        }
        0x45 => {
            let new = !NUM_LOCK.load(Ordering::Relaxed);
            NUM_LOCK.store(new, Ordering::Relaxed);
            update_leds();
        }
        0x46 => {
            let new = !SCROLL_LOCK.load(Ordering::Relaxed);
            SCROLL_LOCK.store(new, Ordering::Relaxed);
            update_leds();
        }
        _ => {}
    }
}

/// Update keyboard LEDs to match toggle state.
fn update_leds() {
    let caps = CAPS_LOCK.load(Ordering::Relaxed) as u8;
    let num = NUM_LOCK.load(Ordering::Relaxed) as u8;
    let scroll = SCROLL_LOCK.load(Ordering::Relaxed) as u8;
    let leds = caps | (num << 1) | (scroll << 2);
    keyboard_command_data(KBD_CMD_SET_LEDS, leds);
}

// ── Public API ──────────────────────────────────────────────────────────
/// Read the next keypress from the ring buffer, or `None` if empty.
pub fn read_key() -> Option<u8> {
    RING.pop()
}

/// Check whether the keyboard is initialized.
#[allow(dead_code)]
pub fn is_initialized() -> bool {
    KEYBOARD_INITIALIZED.load(Ordering::Acquire)
}
