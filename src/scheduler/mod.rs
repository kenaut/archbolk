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

    /// Initialize the scheduler with an idle task.
    pub fn init(&mut self, idle_entry: fn() -> !, idle_stack: &'static mut [u8]) {
        self.spawn(idle_entry, idle_stack);
        self.current = 0;
        if let Some(ref mut task) = self.tasks[0] {
            task.state = TaskState::Running;
        }
        serial_println!("Scheduler: initialized with {} task(s)", self.count);
    }

    /// Pick the next runnable task and return its saved RSP.
    ///
    /// Saves `old_rsp` (the current task's interrupt-frame pointer on its
    /// stack) into the current task, marks it Ready, then selects the next
    /// Ready task round-robin and marks it Running. Returns the new task's
    /// RSP so the assembly stub can `iretq` into it.
    fn pick_next(&mut self, old_rsp: usize) -> usize {
        if self.count == 0 {
            return old_rsp;
        }

        // Find the currently running task.
        let mut running_idx = self.current;
        for i in 0..MAX_TASKS {
            if let Some(ref task) = self.tasks[i] {
                if task.state == TaskState::Running {
                    running_idx = i;
                    break;
                }
            }
        }

        // Save current context and mark Ready.
        if let Some(ref mut task) = self.tasks[running_idx] {
            task.rsp = old_rsp;
            if task.state == TaskState::Running {
                task.state = TaskState::Ready;
            }
        }

        // Find the next Ready task (round-robin, starting after current).
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
        if let Some(ref mut task) = self.tasks[next] {
            task.state = TaskState::Running;
            return task.rsp;
        }

        old_rsp
    }

    /// Return the RSP of the currently running task (its initial frame).
    pub fn current_rsp(&self) -> usize {
        self.tasks
            .get(self.current)
            .and_then(|t| t.as_ref())
            .map(|t| t.rsp)
            .unwrap_or(0)
    }
}

/// Global scheduler instance.
pub static SCHEDULER: spin::Mutex<Scheduler> = spin::Mutex::new(Scheduler::new());

/// Set to true by the timer IRQ to request a reschedule.
static NEED_RESCHEDULE: AtomicBool = AtomicBool::new(false);

/// Called from the timer IRQ handler (interrupt context).
pub fn timer_tick() {
    NEED_RESCHEDULE.store(true, Ordering::Release);
}

/// Called from the interrupt stub on every interrupt return.
///
/// If a reschedule was requested, switches to the next task's interrupt
/// frame. Returns the new RSP, or 0 if no switch is needed (the stub then
/// keeps the current RSP).
#[unsafe(no_mangle)]
pub extern "C" fn switch_from_interrupt(old_rsp: usize) -> usize {
    if !NEED_RESCHEDULE.swap(false, Ordering::AcqRel) {
        return 0;
    }
    SCHEDULER.lock().pick_next(old_rsp)
}
