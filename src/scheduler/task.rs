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
    /// Human-readable name.
    pub name: &'static str,
    /// Saved RSP pointing at this task's interrupt frame on its kernel stack.
    pub rsp: usize,
    /// Task state.
    pub state: TaskState,
    /// Whether this task has been terminated and should be reaped.
    pub terminated: bool,
    /// Whether this task runs in ring 3.
    pub user_mode: bool,
    /// Kernel stack top (loaded into TSS.rsp0 while this task runs).
    pub kernel_stack_top: usize,
    /// Kernel stack for this task (kept alive; frame lives inside it).
    #[allow(dead_code)]
    pub stack: &'static mut [u8],
    /// Tick count at which a sleeping task should be woken (None = not sleeping).
    pub wake_tick: Option<u64>,
    /// Program break for user-space heap allocations.
    pub brk: usize,
}

impl Task {
    /// Build the 22-slot initial interrupt frame on a kernel stack.
    ///
    /// Layout (low to high address, matches `InterruptFrame`):
    ///   [r15..rax]  [vector=0]  [error=0]  [rip]  [cs]  [rflags]  [rsp]  [ss]
    fn build_frame(
        kernel_stack_top: usize,
        entry: usize,
        cs: u64,
        user_rsp: usize,
        ss: u64,
    ) -> usize {
        // Align the frame start down to 8 bytes (stack base may be unaligned).
        let frame_start = (kernel_stack_top - FRAME_SLOTS * 8) & !7usize;
        let base = frame_start as *mut u64;
        unsafe {
            for i in 0..15 {
                base.add(i).write(0); // r15..rax
            }
            base.add(15).write(0); // vector (dummy)
            base.add(16).write(0); // error code (dummy)
            base.add(17).write(entry as u64); // rip
            base.add(18).write(cs); // cs
            base.add(19).write(0x202); // rflags (IF set)
            base.add(20).write(user_rsp as u64); // rsp
            base.add(21).write(ss); // ss
        }
        frame_start
    }

    /// Create a kernel (ring 0) task.
    pub fn new(id: usize, name: &'static str, entry: fn() -> !, stack: &'static mut [u8]) -> Self {
        let stack_top = stack.as_ptr() as usize + stack.len();
        let rsp = Self::build_frame(stack_top, entry as usize, 0x08, stack_top, 0x10);
        Task {
            id,
            name,
            rsp,
            state: TaskState::Ready,
            terminated: false,
            user_mode: false,
            kernel_stack_top: stack_top,
            stack,
            wake_tick: None,
            brk: 0,
        }
    }

    /// Create a user-mode (ring 3) task with separate user and kernel stacks.
    pub fn new_user(
        id: usize,
        name: &'static str,
        entry: usize,
        user_stack_top: usize,
        kernel_stack: &'static mut [u8],
        user_brk: usize,
    ) -> Self {
        let kernel_stack_top = kernel_stack.as_ptr() as usize + kernel_stack.len();
        // cs = 0x23 (USER_CODE_SEL 0x20 | RPL3), ss = 0x1b (USER_DATA_SEL 0x18 | RPL3)
        let rsp = Self::build_frame(kernel_stack_top, entry, 0x23, user_stack_top, 0x1b);
        Task {
            id,
            name,
            rsp,
            state: TaskState::Ready,
            terminated: false,
            user_mode: true,
            kernel_stack_top,
            stack: kernel_stack,
            wake_tick: None,
            brk: user_brk,
        }
    }
}
