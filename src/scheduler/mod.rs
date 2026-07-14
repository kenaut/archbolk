pub mod task;

use task::{Task, TaskState};
use crate::serial_println;
use core::sync::atomic::{AtomicBool, Ordering};

/// Maximum number of tasks.
const MAX_TASKS: usize = 64;

/// Round-robin scheduler.
pub struct Scheduler {
    /// Task table.
    tasks: [Option<Task>; MAX_TASKS],
    /// Index of the currently running task.
    current: usize,
    /// Number of active tasks.
    count: usize,
}

impl Scheduler {
    pub const fn new() -> Self {
        const NONE: Option<Task> = None;
        Scheduler {
            tasks: [NONE; MAX_TASKS],
            current: 0,
            count: 0,
        }
    }

    /// Add a new task. Returns its task ID.
    pub fn spawn(&mut self, entry: fn() -> !, stack: &'static mut [u8]) -> usize {
        let id = self.count;
        self.tasks[id] = Some(Task::new(id, entry, stack));
        self.count += 1;
        id
    }

    /// Return the ID of the current task.
    pub fn current_id(&self) -> usize {
        self.current
    }

    /// Switch to the next ready task.
    ///
    /// Finds the currently running task, marks it `Ready`,
    /// then searches for the next `Ready` task and marks it `Running`.
    /// Returns the next task's RSP.
    ///
    /// NOTE: The current task's RSP is NOT saved here — `swtch` does that.
    pub fn schedule(&mut self) -> usize {
        if self.count == 0 {
            return 0;
        }

        // Find the currently running task and mark it Ready.
        let mut running_idx = self.current;
        for i in 0..MAX_TASKS {
            if let Some(ref task) = self.tasks[i] {
                if task.state == TaskState::Running {
                    running_idx = i;
                    break;
                }
            }
        }

        if let Some(ref mut task) = self.tasks[running_idx] {
            task.state = TaskState::Ready;
        }

        // Search for the next ready task, starting after the current one.
        let start = running_idx;
        let mut next = start;
        loop {
            next = (next + 1) % MAX_TASKS;
            if next == start {
                break;
            }
            if let Some(ref task) = self.tasks[next] {
                if task.state == TaskState::Ready {
                    break;
                }
            }
        }

        self.current = next;

        if let Some(ref mut task) = self.tasks[self.current] {
            task.state = TaskState::Running;
            return task.rsp;
        }

        // Fallback: no ready task found.
        0
    }

    /// Initialize the scheduler with an idle task.
    pub fn init(&mut self, idle_entry: fn() -> !, idle_stack: &'static mut [u8]) {
        self.spawn(idle_entry, idle_stack);
        self.current = 0;
        if let Some(ref mut task) = self.tasks[0] {
            task.state = TaskState::Running;
        }
        serial_println!("Scheduler: initialized with {} task(s)", self.count);
    }
}

/// Global scheduler instance.
pub static SCHEDULER: spin::Mutex<Scheduler> = spin::Mutex::new(Scheduler::new());

/// Set to true by the timer IRQ to request a reschedule.
static NEED_RESCHEDULE: AtomicBool = AtomicBool::new(false);

/// Called from the timer IRQ handler (interrupt context).
/// Just sets a flag — the actual switch happens in the idle loop.
pub fn timer_tick() {
    NEED_RESCHEDULE.store(true, Ordering::Release);
}

/// Called from the idle loop. If a reschedule was requested, perform it.
pub fn reschedule() {
    if !NEED_RESCHEDULE.load(Ordering::Acquire) {
        return;
    }
    NEED_RESCHEDULE.store(false, Ordering::Release);

    unsafe extern "C" {
        fn swtch(old_rsp: &mut usize, new_rsp: usize);
    }

    // Step 1: Get a pointer to the current task's rsp field.
    // We need this BEFORE calling schedule() because schedule() will
    // update self.current to point to the next task.
    let old_rsp_ptr = {
        let sched = SCHEDULER.lock();
        // Find the currently running task
        let mut running_idx = sched.current;
        for i in 0..MAX_TASKS {
            if let Some(ref task) = sched.tasks[i] {
                if task.state == TaskState::Running {
                    running_idx = i;
                    break;
                }
            }
        }
        if let Some(ref task) = sched.tasks[running_idx] {
            &task.rsp as *const usize as *mut usize
        } else {
            return;
        }
    };

    // Step 2: Find the next task to run.
    // schedule() marks current Running→Ready, next Ready→Running,
    // updates self.current, and returns the next task's saved RSP.
    let new_rsp = SCHEDULER.lock().schedule();

    if new_rsp == 0 {
        return;
    }

    // Step 3: swtch saves current registers + RSP into *old_rsp_ptr,
    // then loads new_rsp and pops the new task's registers.
    // When this task is later resumed, swtch returns here.
    unsafe {
        swtch(&mut *old_rsp_ptr, new_rsp);
    }
}