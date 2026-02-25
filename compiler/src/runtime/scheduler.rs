//! Task scheduler for BoxLang green threads
//!
//! Implements a work-stealing scheduler for lightweight threads.

use std::cell::RefCell;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker as StdWaker};

use super::future::{AsyncState, Future};

/// Thread-local scheduler instance
thread_local! {
    static SCHEDULER: RefCell<Option<Scheduler>> = RefCell::new(None);
}

/// Unique identifier for a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(u64);

impl TaskId {
    /// Generate a new unique task ID
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        TaskId(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

/// State of a task
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Task is ready to run
    Ready,
    /// Task is currently running
    Running,
    /// Task is blocked (e.g., waiting for I/O or a channel)
    Blocked,
    /// Task has completed
    Completed,
    /// Task has been cancelled
    Cancelled,
}

/// A task (green thread)
#[derive(Clone)]
pub struct Task {
    /// Unique identifier
    pub id: TaskId,
    /// Current state
    pub state: TaskState,
    /// Task priority (higher = more important)
    pub priority: u8,
    /// Stack size in bytes
    pub stack_size: usize,
    /// Whether the task is a daemon (doesn't prevent runtime shutdown)
    pub is_daemon: bool,
}

impl Task {
    /// Create a new task with the given ID
    pub fn new(id: TaskId) -> Self {
        Self {
            id,
            state: TaskState::Ready,
            priority: 5,             // Default priority
            stack_size: 1024 * 1024, // 1MB default stack
            is_daemon: false,
        }
    }

    /// Set the task priority
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Set the stack size
    pub fn with_stack_size(mut self, size: usize) -> Self {
        self.stack_size = size;
        self
    }

    /// Mark the task as a daemon
    pub fn daemon(mut self) -> Self {
        self.is_daemon = true;
        self
    }
}

/// Task queue for a worker thread
pub struct TaskQueue {
    /// Local task queue
    local: Vec<Task>,
    /// Work-stealing queue (shared with other workers)
    shared: Arc<Mutex<Vec<Task>>>,
}

impl TaskQueue {
    /// Create a new task queue
    pub fn new() -> Self {
        Self {
            local: Vec::new(),
            shared: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Push a task to the local queue
    pub fn push_local(&mut self, task: Task) {
        self.local.push(task);
    }

    /// Pop a task from the local queue
    pub fn pop_local(&mut self) -> Option<Task> {
        self.local.pop()
    }

    /// Push a task to the shared queue
    pub fn push_shared(&self, task: Task) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.push(task);
        }
        // If lock fails (poisoned), we silently drop the task
        // In production, this should be logged
    }

    /// Steal a task from the shared queue
    pub fn steal(&self) -> Option<Task> {
        self.shared.lock().ok()?.pop()
    }

    /// Check if the local queue is empty
    pub fn is_empty(&self) -> bool {
        self.local.is_empty()
    }

    /// Get the number of local tasks
    pub fn len(&self) -> usize {
        self.local.len()
    }

    /// Iterate over tasks in the local queue
    pub fn iter(&self) -> impl Iterator<Item = &Task> {
        self.local.iter()
    }

    /// Push a task to the back of the local queue
    pub fn push_back(&mut self, task: Task) {
        self.local.push(task);
    }
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Task scheduler
pub struct Scheduler {
    /// All tasks in the system
    tasks: Arc<Mutex<HashMap<TaskId, Task>>>,
    /// Task queues for each worker
    queues: Vec<TaskQueue>,
    /// Number of worker threads
    num_workers: usize,
    /// Whether the scheduler is running
    running: bool,
}

impl Scheduler {
    /// Create a new scheduler with the given number of workers
    pub fn new(num_workers: usize) -> Self {
        let mut queues = Vec::with_capacity(num_workers);
        for _ in 0..num_workers {
            queues.push(TaskQueue::new());
        }

        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            queues,
            num_workers,
            running: false,
        }
    }

    /// Spawn a new task
    pub fn spawn<F>(&mut self, f: F) -> TaskId
    where
        F: FnOnce() + Send + 'static,
    {
        let id = TaskId::new();
        let task = Task::new(id);

        // Add to tasks map
        if let Ok(mut tasks) = self.tasks.lock() {
            tasks.insert(id, task);
        }

        // Add to a worker queue (round-robin for now)
        let worker_id = id.0 as usize % self.num_workers;
        self.queues[worker_id].push_shared(Task::new(id));

        id
    }

    /// Get the state of a task
    pub fn task_state(&self, id: TaskId) -> Option<TaskState> {
        self.tasks.lock().ok()?.get(&id).map(|t| t.state)
    }

    /// Block a task (e.g., waiting for I/O)
    pub fn block_task(&mut self, id: TaskId) {
        if let Ok(mut tasks) = self.tasks.lock() {
            if let Some(task) = tasks.get_mut(&id) {
                task.state = TaskState::Blocked;
            }
        }
    }

    /// Unblock a task (e.g., I/O completed)
    pub fn unblock_task(&mut self, id: TaskId) {
        if let Ok(mut tasks) = self.tasks.lock() {
            if let Some(task) = tasks.get_mut(&id) {
                task.state = TaskState::Ready;
            }
        }
    }

    /// Complete a task
    pub fn complete_task(&mut self, id: TaskId) {
        if let Ok(mut tasks) = self.tasks.lock() {
            if let Some(task) = tasks.get_mut(&id) {
                task.state = TaskState::Completed;
            }
        }
    }

    /// Cancel a task
    pub fn cancel_task(&mut self, id: TaskId) {
        if let Ok(mut tasks) = self.tasks.lock() {
            if let Some(task) = tasks.get_mut(&id) {
                task.state = TaskState::Cancelled;
            }
        }
    }

    /// Check if the scheduler is running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Start the scheduler
    pub fn start(&mut self) {
        self.running = true;
        // In a full implementation, this would start the worker threads
    }

    /// Stop the scheduler
    pub fn stop(&mut self) {
        self.running = false;
        // In a full implementation, this would stop the worker threads
    }

    /// Get the number of tasks
    pub fn num_tasks(&self) -> usize {
        self.tasks.lock().map(|t| t.len()).unwrap_or(0)
    }

    /// Get the number of ready tasks
    pub fn num_ready_tasks(&self) -> usize {
        self.tasks
            .lock()
            .map(|t| t.values().filter(|t| t.state == TaskState::Ready).count())
            .unwrap_or(0)
    }

    /// Get the number of running tasks
    pub fn num_running_tasks(&self) -> usize {
        self.tasks
            .lock()
            .map(|t| t.values().filter(|t| t.state == TaskState::Running).count())
            .unwrap_or(0)
    }

    /// Get the number of blocked tasks
    pub fn num_blocked_tasks(&self) -> usize {
        self.tasks
            .lock()
            .map(|t| t.values().filter(|t| t.state == TaskState::Blocked).count())
            .unwrap_or(0)
    }
}

/// A task that wraps a Future
pub struct FutureTask<F> {
    /// The underlying future
    future: F,
    /// Task ID
    id: TaskId,
    /// Current state
    state: AsyncState,
}

impl<F: Future> FutureTask<F> {
    /// Create a new future task
    pub fn new(future: F, id: TaskId) -> Self {
        Self {
            future,
            id,
            state: AsyncState::Start,
        }
    }

    /// Poll the future once
    ///
    /// # Safety
    /// This method uses unsafe to pin the future. The caller must ensure that:
    /// - The FutureTask is not moved while the future is pinned
    /// - The future is not polled again after it returns Ready
    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<F::Output> {
        self.state = AsyncState::Running;
        // SAFETY: We're pinning the future in place. This is safe because:
        // 1. FutureTask is stored in a stable location (heap-allocated in the scheduler)
        // 2. We don't move the future after pinning
        // 3. We only poll the future while it's pinned
        let pinned = unsafe { Pin::new_unchecked(&mut self.future) };
        let result = pinned.poll(cx);
        match &result {
            Poll::Ready(_) => self.state = AsyncState::Completed,
            Poll::Pending => self.state = AsyncState::Waiting(0),
        }
        result
    }

    /// Get the current state
    pub fn state(&self) -> AsyncState {
        self.state
    }
}

impl Scheduler {
    /// Spawn a future as a task
    pub fn spawn_future<F>(&mut self, future: F) -> TaskId
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let id = TaskId::new();
        let task = Task::new(id);

        // Add to tasks map
        if let Ok(mut tasks) = self.tasks.lock() {
            tasks.insert(id, task);
        }

        // Add to a worker queue (round-robin for now)
        let worker_id = id.0 as usize % self.num_workers;
        self.queues[worker_id].push_shared(Task::new(id));

        id
    }

    /// Poll a future task once
    ///
    /// # Safety
    /// The caller must ensure that:
    /// - The future is not moved while pinned
    /// - The future is stored in a stable memory location
    pub fn poll_future<F>(&self, future: &mut F, task_id: TaskId) -> Poll<F::Output>
    where
        F: Future,
    {
        // Create a waker for this task
        let waker = self.create_waker(task_id);
        let mut cx = Context::from_waker(&waker);

        // SAFETY: We're pinning the future in place. This is safe because:
        // 1. The future is passed by mutable reference and not moved
        // 2. The future remains in the same memory location during the poll
        let pinned = unsafe { Pin::new_unchecked(future) };
        pinned.poll(&mut cx)
    }

    /// Create a waker for a task
    ///
    /// # Safety
    /// The waker vtable functions must correctly handle the task_id data pointer.
    /// The task_id is converted to a pointer and back, which is safe for u64 values.
    fn create_waker(&self, task_id: TaskId) -> StdWaker {
        let tasks = self.tasks.clone();
        // Convert task_id to a raw pointer. This is safe because:
        // 1. TaskId is a u64, which fits in a pointer-sized integer
        // 2. We never dereference this pointer as a real memory address
        // 3. The vtable functions convert it back to TaskId using the u64 value
        let raw_waker = RawWaker::new(task_id.0 as *const (), &VTABLE);
        // SAFETY: The RawWaker is properly constructed with a valid vtable.
        // The data pointer is the task_id as a usize, not a real memory address.
        unsafe { StdWaker::from_raw(raw_waker) }
    }
}

/// VTable for our custom waker
///
/// SAFETY: All vtable functions must correctly handle the data pointer.
/// The data pointer stores the task_id as a usize (not a real memory address),
/// so clone and drop are no-ops, and wake functions convert it back to TaskId.
static VTABLE: RawWakerVTable =
    RawWakerVTable::new(clone_waker, wake_waker, wake_by_ref_waker, drop_waker);

/// Clone the waker
///
/// SAFETY: The data pointer contains the task_id as a usize value, not a real pointer.
/// Creating a new RawWaker with the same data is safe because:
/// 1. We don't actually dereference the pointer
/// 2. The data is just a u64 value stored as a pointer
unsafe fn clone_waker(data: *const ()) -> RawWaker {
    RawWaker::new(data, &VTABLE)
}

/// Wake the task by its ID
///
/// SAFETY: The data pointer contains the task_id as a usize value.
/// We convert it back to TaskId without dereferencing, which is safe.
/// The scheduler access is thread-safe due to RefCell and Mutex usage.
unsafe fn wake_waker(data: *const ()) {
    // SAFETY: data is the task_id stored as a pointer-sized integer.
    // We cast it back to u64 without dereferencing, which is safe.
    let task_id = TaskId(data as u64);
    // Actually reschedule the task by adding it back to the ready queue
    SCHEDULER.with(|scheduler| {
        if let Ok(mut sched) = scheduler.try_borrow_mut() {
            if let Some(ref mut s) = *sched {
                // Unblock the task if it was blocked
                s.unblock_task(task_id);
                // Add to first queue if not already there
                if !s.queues.is_empty() {
                    let queue = &mut s.queues[0];
                    if !queue.iter().any(|t| t.id == task_id) {
                        if let Ok(tasks) = s.tasks.lock() {
                            if let Some(task) = tasks.get(&task_id).cloned() {
                                queue.push_back(task);
                            }
                        }
                    }
                }
            }
        }
    });
}

/// Wake the task by reference
///
/// SAFETY: This just delegates to wake_waker with the same safety invariants.
unsafe fn wake_by_ref_waker(data: *const ()) {
    // SAFETY: Same invariants as wake_waker
    wake_waker(data);
}

/// Drop the waker
///
/// SAFETY: The data pointer contains the task_id as a usize, not allocated memory.
/// No cleanup is needed since we never allocated anything based on this "pointer".
unsafe fn drop_waker(_data: *const ()) {
    // Nothing to drop - data is just a task_id value, not a pointer to allocated memory
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new(num_cpus::get())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let id1 = TaskId::new();
        let id2 = TaskId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_scheduler_spawn() {
        let mut scheduler = Scheduler::new(4);
        let id = scheduler.spawn(|| {
            println!("Hello from task!");
        });

        assert!(scheduler.task_state(id).is_some());
    }

    #[test]
    fn test_task_state_transitions() {
        let mut scheduler = Scheduler::new(4);
        let id = scheduler.spawn(|| {});

        assert_eq!(scheduler.task_state(id), Some(TaskState::Ready));

        scheduler.block_task(id);
        assert_eq!(scheduler.task_state(id), Some(TaskState::Blocked));

        scheduler.unblock_task(id);
        assert_eq!(scheduler.task_state(id), Some(TaskState::Ready));

        scheduler.complete_task(id);
        assert_eq!(scheduler.task_state(id), Some(TaskState::Completed));
    }

    #[test]
    fn test_task_queue() {
        let mut queue = TaskQueue::new();

        let task1 = Task::new(TaskId::new());
        let task2 = Task::new(TaskId::new());

        queue.push_local(task1);
        queue.push_local(task2);

        assert_eq!(queue.len(), 2);

        let _ = queue.pop_local();
        assert_eq!(queue.len(), 1);

        let _ = queue.pop_local();
        assert!(queue.is_empty());
    }
}
