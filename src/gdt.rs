//! Global Descriptor Table and Task State Segment.
//!
//! The boot assembly sets up a minimal GDT (null / kernel code / kernel data).
//! This module replaces it with a full table that also contains ring-3
//! code/data segments and a TSS, so we can run user-mode tasks and handle
//! ring-3 → ring-0 transitions via `syscall` / interrupts.

/// Number of GDT entries (8 * 8 = 64 bytes).
const GDT_ENTRIES: usize = 8;
/// GDT limit (last valid byte offset).
const GDT_LIMIT: u16 = (GDT_ENTRIES * 8 - 1) as u16;

/// Selector values.
///
/// Layout: user data (0x18) must precede user code (0x20) so SYSRET can
/// derive CS = STAR[63:48]+16 and SS = STAR[63:48]+8 with STAR[63:48] = 0x10.
#[allow(dead_code)]
pub const KERNEL_CODE_SEL: u16 = 0x08;
#[allow(dead_code)]
pub const KERNEL_DATA_SEL: u16 = 0x10;
#[allow(dead_code)]
pub const USER_DATA_SEL: u16 = 0x18; // RPL 3 → 0x1b
#[allow(dead_code)]
pub const USER_CODE_SEL: u16 = 0x20; // RPL 3 → 0x23
pub const TSS_SEL: u16 = 0x28;

/// x86-64 Task State Segment (long mode layout).
#[repr(C, packed)]
pub struct TaskStateSegment {
    reserved0: u32,
    /// Ring-0 stack pointer, loaded on ring-3 → ring-0 transitions.
    pub rsp0: u64,
    rsp1: u64,
    rsp2: u64,
    ist: [u64; 7],
    reserved1: u64,
    iomap_base: u16,
}

impl TaskStateSegment {
    const fn new() -> Self {
        TaskStateSegment {
            reserved0: 0,
            rsp0: 0,
            rsp1: 0,
            rsp2: 0,
            ist: [0; 7],
            reserved1: 0,
            iomap_base: 0,
        }
    }
}

static mut TSS: TaskStateSegment = TaskStateSegment::new();
static mut GDT: [u64; GDT_ENTRIES] = [0; GDT_ENTRIES];

/// Build a 64-bit code/data segment descriptor.
fn segment(base: u32, limit: u32, access: u8, flags: u8) -> u64 {
    let limit_low = (limit & 0xFFFF) as u64;
    let base_low = ((base & 0xFFFF) as u64) << 16;
    let base_mid = (((base >> 16) & 0xFF) as u64) << 32;
    let access_u = (access as u64) << 40;
    let limit_high = (((limit >> 16) & 0xF) as u64) << 48;
    let flags_u = ((flags & 0xF) as u64) << 52;
    let base_high = (((base >> 24) & 0xFF) as u64) << 56;
    limit_low | base_low | base_mid | access_u | limit_high | flags_u | base_high
}

/// Build a TSS descriptor (system segment, present, available).
fn tss_descriptor(base: u64, limit: u32) -> [u64; 2] {
    // access = 0x89 (P=1, DPL=0, S=0, type=9 TSS available), flags = 0.
    let low = ((limit & 0xFFFF) as u64)
        | ((base & 0xFFFF) << 16)
        | (((base >> 16) & 0xFF) << 32)
        | (0x89u64 << 40)
        | ((((limit >> 16) & 0xF) as u64) << 48)
        | (((base >> 24) & 0xFF) << 56);
    let high = base >> 32;
    [low, high]
}

/// Initialize the full GDT and TSS, then load them.
///
/// Must be called once from Rust entry, with interrupts disabled.
pub fn init() {
    let tss_addr = core::ptr::addr_of_mut!(TSS) as u64;
    let limit = (core::mem::size_of::<TaskStateSegment>() - 1) as u32;

    unsafe {
        // 0x00: null
        GDT[0] = 0;
        // 0x08: kernel code (DPL=0, L=1, exec/read)
        GDT[1] = segment(0, 0, 0x9A, 0x2);
        // 0x10: kernel data (DPL=0, read/write, G=1 limit=4GiB)
        GDT[2] = segment(0, 0xFFFFF, 0x92, 0xC);
        // 0x18: user data (DPL=3, read/write, G=1 limit=4GiB)
        // SS limit is still checked in long mode, so it must cover the stack.
        GDT[3] = segment(0, 0xFFFFF, 0xF2, 0xC);
        // 0x20: user code (DPL=3, L=1, exec/read)
        GDT[4] = segment(0, 0, 0xFA, 0x2);
        // 0x28/0x30: TSS
        let tss = tss_descriptor(tss_addr, limit);
        GDT[5] = tss[0];
        GDT[6] = tss[1];

        let gdt_ptr = GdtPointer {
            limit: GDT_LIMIT,
            base: core::ptr::addr_of!(GDT) as u64,
        };
        core::arch::asm!("lgdt [{}]", in(reg) &gdt_ptr, options(nostack));

        // Reload segment registers with kernel selectors.
        core::arch::asm!(
            "mov ax, 0x10",
            "mov ds, ax",
            "mov es, ax",
            "mov fs, ax",
            "mov gs, ax",
            "mov ss, ax",
            out("ax") _,
            options(nostack),
        );

        // Load task register with TSS selector (0x28).
        core::arch::asm!("ltr ax", in("ax") TSS_SEL, options(nostack));
    }

    crate::serial_println!("GDT/TSS: initialized (TSS at 0x{:x})", tss_addr);
}

/// Update the ring-0 stack pointer used on ring-3 → ring-0 transitions.
pub fn set_rsp0(rsp: usize) {
    unsafe {
        // rsp0 is at byte offset 4 in the packed TSS.
        let ptr = (core::ptr::addr_of_mut!(TSS) as *mut u8).add(4) as *mut u64;
        ptr.write_unaligned(rsp as u64);
    }
}

#[repr(C, packed)]
struct GdtPointer {
    limit: u16,
    base: u64,
}
