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

/// Task Control Block.
pub struct Task {
    /// Task ID.
    pub id: usize,
    /// Saved stack pointer (RSP) when not running.
    pub rsp: usize,
    /// Task state.
    pub state: TaskState,
    /// Kernel stack for this task.
    pub stack: &'static mut [u8],
}

impl Task {
    /// Create a new task with the given entry point and stack.
    ///
    /// Sets up the stack so that the first `swtch` into this task will
    /// jump to `entry`. The stack layout matches what `swtch` expects:
    ///
    /// swtch push order: r15, r14, r13, r12, rbp, rbx
    /// swtch pop  order: rbx, rbp, r12, r13, r14, r15
    ///
    /// Stack layout after swtch saves (low to high address):
    ///   [rbx]            <- RSP after all pushes (popped 1st)
    ///   [rbp]            <- popped 2nd
    ///   [r12]            <- popped 3rd
    ///   [r13]            <- popped 4th
    ///   [r14]            <- popped 5th
    ///   [r15]            <- popped 6th
    ///   [return_address] <- ret reads this (highest address)
    ///
    /// So the initial stack for a new task should look like:
    ///   [rbx = 0]        <- RSP
    ///   [rbp = 0]
    ///   [r12 = 0]
    ///   [r13 = 0]
    ///   [r14 = 0]
    ///   [r15 = 0]
    ///   [entry]          <- return address (swtch will ret to this)
    pub fn new(id: usize, entry: fn() -> !, stack: &'static mut [u8]) -> Self {
        let stack_top = stack.as_ptr() as usize + stack.len();

        // Allocate 7 entries (56 bytes) at the top of the stack
        let frame_start = stack_top - 56;
        let base = frame_start as *mut u64;

        unsafe {
            base.add(0).write(0);              // rbx
            base.add(1).write(0);              // rbp
            base.add(2).write(0);              // r12
            base.add(3).write(0);              // r13
            base.add(4).write(0);              // r14
            base.add(5).write(0);              // r15
            base.add(6).write(entry as u64);   // return address
        }

        // RSP points to rbx (lowest address) — swtch pops rbx first
        let rsp = frame_start;

        Task {
            id,
            rsp,
            state: TaskState::Ready,
            stack,
        }
    }
}
