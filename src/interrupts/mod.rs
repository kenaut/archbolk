pub mod handler;
pub mod idt;
pub mod pic;
pub mod pit;

/// Global tick counter, incremented by the PIT IRQ handler.
static TICK_COUNT: core::sync::atomic::AtomicU64 =
    core::sync::atomic::AtomicU64::new(0);

/// Initialize the interrupt subsystem: IDT, PIC, and PIT.
pub fn init() {
    idt::init();
    pic::init();
    pit::init(100); // ~100 Hz timer
}

/// Called from the IRQ 0 (PIT timer) handler.
fn pit_tick() {
    TICK_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
}

/// Return the current tick count since PIT initialization.
pub fn get_tick_count() -> u64 {
    TICK_COUNT.load(core::sync::atomic::Ordering::Relaxed)
}
