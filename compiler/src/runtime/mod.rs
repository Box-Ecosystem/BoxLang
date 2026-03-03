//! BoxLang Runtime - Lightweight Thread and Concurrency Support
//!
//! This module provides:
//! - Green threads (lightweight user-level threads)
//! - Channels for message passing
//! - Select for multiplexing
//! - Async/await support

use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};
use std::thread;

pub mod channel;
pub mod future;
pub mod memory;
pub mod scheduler;

pub use channel::{Channel, Receiver, Sender};
pub use future::{
    AsyncState, AsyncStateMachine, BoxFuture, Future, FuturePoll, TaskContext, Waker,
};
pub use memory::{
    memory_stats, reset_memory_stats, AllocError, AllocResult, Arena, Box, BumpAlloc, MemoryStats,
    Pool, PoolGuard, StackAlloc,
};
pub use scheduler::{Scheduler, Task, TaskId};

// Thread-local storage for the current task
thread_local! {
    static CURRENT_TASK: std::cell::RefCell<Option<TaskId>> = std::cell::RefCell::new(None);
}

/// Set the current task ID
pub fn set_current_task(id: TaskId) {
    CURRENT_TASK.with(|task| {
        *task.borrow_mut() = Some(id);
    });
}

/// Get the current task ID
pub fn current_task() -> Option<TaskId> {
    CURRENT_TASK.with(|task| *task.borrow())
}

/// Clear the current task ID
pub fn clear_current_task() {
    CURRENT_TASK.with(|task| {
        *task.borrow_mut() = None;
    });
}

/// Spawn a new lightweight thread
pub fn spawn<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    // For now, use native threads
    // In a full implementation, this would use a green thread scheduler
    let handle = thread::spawn(f);
    JoinHandle {
        inner: Some(handle),
    }
}

/// Handle for a spawned thread
pub struct JoinHandle<T> {
    inner: Option<thread::JoinHandle<T>>,
}

impl<T> JoinHandle<T> {
    /// Wait for the thread to finish and return its result
    pub fn join(mut self) -> Result<T, std::boxed::Box<dyn std::any::Any + Send>> {
        match self.inner.take() {
            Some(handle) => handle.join(),
            None => {
                // This should not happen as we consume self
                // Return an error instead of panicking for production safety
                Err(std::boxed::Box::new("JoinHandle::join called on already-joined handle"))
            }
        }
    }

    /// Check if the thread has finished
    pub fn is_finished(&self) -> bool {
        // This is a simplified implementation
        // In a real implementation, we'd check the task status
        false
    }
}

impl<T> Drop for JoinHandle<T> {
    fn drop(&mut self) {
        // If the handle is dropped without joining, detach the thread
        if let Some(handle) = self.inner.take() {
            // In a real implementation, we might want to cancel the task
            let _ = handle;
        }
    }
}

/// Yield the current task, allowing other tasks to run
pub fn yield_now() {
    // In a full implementation, this would yield to the scheduler
    thread::yield_now();
}

/// Sleep for a specified duration
pub fn sleep(duration: std::time::Duration) {
    thread::sleep(duration);
}

/// A simple work-stealing queue for task scheduling
pub struct WorkQueue<T> {
    queue: Mutex<VecDeque<T>>,
    condvar: Condvar,
}

impl<T> WorkQueue<T> {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            condvar: Condvar::new(),
        }
    }

    /// Push a task to the back of the queue
    pub fn push(&self, task: T) {
        match self.queue.lock() {
            Ok(mut queue) => {
                queue.push_back(task);
                self.condvar.notify_one();
            }
            Err(poisoned) => {
                // Mutex is poisoned, but we can still use the data
                let mut queue = poisoned.into_inner();
                queue.push_back(task);
                self.condvar.notify_one();
            }
        }
    }

    /// Pop a task from the front of the queue
    pub fn pop(&self) -> Option<T> {
        match self.queue.lock() {
            Ok(mut queue) => queue.pop_front(),
            Err(poisoned) => poisoned.into_inner().pop_front(),
        }
    }

    /// Pop a task, blocking until one is available
    pub fn pop_blocking(&self) -> T {
        let mut queue = match self.queue.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        loop {
            if let Some(task) = queue.pop_front() {
                return task;
            }
            queue = match self.condvar.wait(queue) {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
        }
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        match self.queue.lock() {
            Ok(queue) => queue.is_empty(),
            Err(poisoned) => poisoned.into_inner().is_empty(),
        }
    }

    /// Get the number of tasks in the queue
    pub fn len(&self) -> usize {
        match self.queue.lock() {
            Ok(queue) => queue.len(),
            Err(poisoned) => poisoned.into_inner().len(),
        }
    }
}

impl<T> Default for WorkQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Runtime configuration
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Number of worker threads
    pub num_workers: usize,
    /// Stack size for each green thread
    pub stack_size: usize,
    /// Enable work stealing
    pub work_stealing: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            num_workers: num_cpus::get(),
            stack_size: 1024 * 1024, // 1MB
            work_stealing: true,
        }
    }
}

/// Initialize the runtime with the given configuration
pub fn init_runtime(config: RuntimeConfig) {
    // In a full implementation, this would initialize the scheduler
    // and worker threads
    println!("BoxLang Runtime initialized with config: {:?}", config);
}

/// Get the number of CPU cores
pub fn num_cpus() -> usize {
    num_cpus::get()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn() {
        let handle = spawn(|| 42);
        assert_eq!(handle.join().unwrap(), 42);
    }

    #[test]
    fn test_channel() {
        let (tx, rx) = Channel::new(10);
        tx.send(42).unwrap();
        assert_eq!(rx.recv().unwrap(), 42);
    }

    #[test]
    fn test_work_queue() {
        let queue = WorkQueue::new();
        queue.push(1);
        queue.push(2);
        assert_eq!(queue.pop(), Some(1));
        assert_eq!(queue.pop(), Some(2));
        assert_eq!(queue.pop(), None);
    }
}
