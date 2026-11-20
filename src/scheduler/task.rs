/// Task state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Ready to run.
    Ready,
    /// Currently running.
    Running,
    /// Blocked (waiting for something).
    Blocked,
}

/// Number of u64 slots in a saved interrupt frame:
/// 15 registers (r15..rax) + vector + error_code + iretq (rip/cs/rflags/rsp/ss).
pub const FRAME_SLOTS: usize = 22;

/// Task Control Block.
pub struct Task {
    /// Task ID.
    pub id: usize,
    /// Saved RSP pointing at this task's interrupt frame on its stack.
    pub rsp: usize,
    /// Task state.
    pub state: TaskState,
    /// Kernel stack for this task.
    pub stack: &'static mut [u8],
}

impl Task {
    /// Create a new task with the given entry point and stack.
    ///
    /// We build an initial interrupt frame on the stack so that the generic
    /// `iretq`-based context-switch path can start the task: popping the
    /// registers and executing `iretq` jumps to `entry` with a fresh stack.
    ///
    /// Frame layout (low to high address, matches `InterruptFrame`):
    ///   [r15..rax]      <- RSP (15 slots, all zero)
    ///   [vector=0]      (dummy, skipped by `add rsp, 16`)
    ///   [error_code=0]  (dummy, skipped by `add rsp, 16`)
    ///   [rip=entry]
    ///   [cs=0x08]        kernel code segment
    ///   [rflags=0x202]   interrupts enabled
    ///   [rsp=stack_top]  fresh stack for the task
    ///   [ss=0x10]        kernel data segment
    pub fn new(id: usize, entry: fn() -> !, stack: &'static mut [u8]) -> Self {
        let stack_top = stack.as_ptr() as usize + stack.len();
        let frame_start = stack_top - FRAME_SLOTS * 8;
        let base = frame_start as *mut u64;

        unsafe {
            for i in 0..15 {
                base.add(i).write(0); // r15..rax
            }
            base.add(15).write(0); // vector (dummy)
            base.add(16).write(0); // error_code (dummy)
            base.add(17).write(entry as u64); // rip
            base.add(18).write(0x08); // cs
            base.add(19).write(0x202); // rflags (IF set)
            base.add(20).write(stack_top as u64); // rsp
            base.add(21).write(0x10); // ss
        }

        Task {
            id,
            rsp: frame_start,
            state: TaskState::Ready,
            stack,
        }
    }
}
