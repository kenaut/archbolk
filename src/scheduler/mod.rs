pub mod task;

use task::{Task, TaskState};
use crate::serial_println;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Maximum number of tasks.
const MAX_TASKS: usize = 64;
/// Number of preallocated stacks for dynamically spawned tasks.
const MAX_SPAWN: usize = 16;
/// Stack size for each dynamically spawned task.
const SPAWN_STACK_SIZE: usize = 16384;

/// Preallocated stacks for tasks created via `spawn_dynamic`.
static mut SPAWN_STACKS: [core::mem::MaybeUninit<[u8; SPAWN_STACK_SIZE]>; MAX_SPAWN] =
    [const { core::mem::MaybeUninit::uninit() }; MAX_SPAWN];
static NEXT_STACK: AtomicUsize = AtomicUsize::new(0);

/// Round-robin scheduler.
pub struct Scheduler {
    /// Task table.
    tasks: [Option<Task>; MAX_TASKS],
    /// Index of the currently running task.
    current: usize,
    /// Number of task slots ever used.
    count: usize,
}

/// Snapshot of a task for the `ps` command.
#[derive(Clone, Copy)]
pub struct TaskInfo {
    pub id: usize,
    pub name: &'static str,
    pub state: TaskState,
    pub terminated: bool,
    pub rsp: usize,
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
    pub fn spawn(&mut self, name: &'static str, entry: fn() -> !, stack: &'static mut [u8]) -> usize {
        let id = self.count;
        self.tasks[id] = Some(Task::new(id, name, entry, stack));
        self.count += 1;
        id
    }

    /// Return the ID of the current task.
    pub fn current_id(&self) -> usize {
        self.current
    }

    /// Initialize the scheduler with an idle task.
    pub fn init(&mut self, idle_entry: fn() -> !, idle_stack: &'static mut [u8]) {
        self.spawn("idle", idle_entry, idle_stack);
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
                if task.state == TaskState::Ready && !task.terminated {
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

    /// Mark a task as terminated. Returns true on success.
    ///
    /// Terminating task 0 (idle) is forbidden — the system needs it.
    pub fn kill(&mut self, id: usize) -> bool {
        if id == 0 || id >= self.count {
            return false;
        }
        match self.tasks[id] {
            Some(ref mut task) => {
                task.terminated = true;
                task.state = TaskState::Blocked;
                true
            }
            None => false,
        }
    }

    /// Fill `out` with a snapshot of all live tasks. Returns the number filled.
    pub fn list(&self, out: &mut [TaskInfo]) -> usize {
        let mut n = 0;
        for i in 0..self.count {
            if n >= out.len() {
                break;
            }
            if let Some(ref task) = self.tasks[i] {
                out[n] = TaskInfo {
                    id: task.id,
                    name: task.name,
                    state: task.state,
                    terminated: task.terminated,
                    rsp: task.rsp,
                };
                n += 1;
            }
        }
        n
    }

    /// Number of preallocated stacks still available.
    pub fn free_stacks(&self) -> usize {
        MAX_SPAWN.saturating_sub(NEXT_STACK.load(Ordering::Relaxed))
    }
}

/// Global scheduler instance.
pub static SCHEDULER: spin::Mutex<Scheduler> = spin::Mutex::new(Scheduler::new());

/// Allocate a preallocated stack and spawn a dynamic task.
/// Returns the task ID, or `None` if no stacks/task slots remain.
pub fn spawn_dynamic(name: &'static str, entry: fn() -> !) -> Option<usize> {
    let idx = NEXT_STACK.fetch_add(1, Ordering::Relaxed);
    if idx >= MAX_SPAWN {
        return None;
    }
    let stack: &'static mut [u8] = unsafe {
        let slot = SPAWN_STACKS[idx].as_mut_ptr() as *mut u8;
        core::slice::from_raw_parts_mut(slot, SPAWN_STACK_SIZE)
    };
    let mut sched = SCHEDULER.lock();
    if sched.count >= MAX_TASKS {
        return None;
    }
    Some(sched.spawn(name, entry, stack))
}

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
