//! Channel implementation for BoxLang
//!
//! Provides multi-producer, multi-consumer channels for message passing
//! between lightweight threads.

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

/// Error types for channel operations
#[derive(Debug, Clone, PartialEq)]
pub enum ChannelError {
    /// Channel is full (for bounded channels)
    Full,
    /// Channel is empty
    Empty,
    /// Channel is closed
    Closed,
    /// Timeout
    Timeout,
    /// Send failed because all receivers are dropped
    SendError,
    /// Receive failed because all senders are dropped
    RecvError,
}

impl std::fmt::Display for ChannelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelError::Full => write!(f, "channel is full"),
            ChannelError::Empty => write!(f, "channel is empty"),
            ChannelError::Closed => write!(f, "channel is closed"),
            ChannelError::Timeout => write!(f, "operation timed out"),
            ChannelError::SendError => write!(f, "send failed: all receivers dropped"),
            ChannelError::RecvError => write!(f, "receive failed: all senders dropped"),
        }
    }
}

impl std::error::Error for ChannelError {}

/// Result type for channel operations
pub type ChannelResult<T> = Result<T, ChannelError>;

/// Internal shared state of the channel
struct ChannelInner<T> {
    /// The message queue
    queue: VecDeque<T>,
    /// Capacity of the channel (0 for unbounded)
    capacity: usize,
    /// Number of active senders
    sender_count: usize,
    /// Number of active receivers
    receiver_count: usize,
    /// Whether the channel is closed
    closed: bool,
}

impl<T> ChannelInner<T> {
    fn new(capacity: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            capacity,
            sender_count: 1,
            receiver_count: 1,
            closed: false,
        }
    }

    fn is_full(&self) -> bool {
        self.capacity > 0 && self.queue.len() >= self.capacity
    }

    fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    fn len(&self) -> usize {
        self.queue.len()
    }
}

/// A multi-producer, multi-consumer channel
pub struct Channel<T> {
    inner: Arc<Mutex<ChannelInner<T>>>,
    send_condvar: Arc<Condvar>,
    recv_condvar: Arc<Condvar>,
}

impl<T> Channel<T> {
    /// Create a new bounded channel with the given capacity
    pub fn new(capacity: usize) -> (Sender<T>, Receiver<T>) {
        let inner = Arc::new(Mutex::new(ChannelInner::new(capacity)));
        let send_condvar = Arc::new(Condvar::new());
        let recv_condvar = Arc::new(Condvar::new());

        let channel = Channel {
            inner: inner.clone(),
            send_condvar: send_condvar.clone(),
            recv_condvar: recv_condvar.clone(),
        };

        let sender = Sender {
            channel: channel.clone(),
        };

        let receiver = Receiver { channel };

        (sender, receiver)
    }

    /// Create an unbounded channel
    pub fn unbounded() -> (Sender<T>, Receiver<T>) {
        Self::new(0)
    }

    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            send_condvar: self.send_condvar.clone(),
            recv_condvar: self.recv_condvar.clone(),
        }
    }
}

/// Sending end of a channel
pub struct Sender<T> {
    channel: Channel<T>,
}

impl<T> Sender<T> {
    /// Send a message into the channel
    ///
    /// Returns `Err(ChannelError::Full)` if the channel is bounded and full.
    /// Returns `Err(ChannelError::Closed)` if the channel is closed.
    pub fn send(&self, msg: T) -> ChannelResult<()> {
        let mut inner = self
            .channel
            .inner
            .lock()
            .map_err(|_| ChannelError::Closed)?;

        if inner.closed {
            return Err(ChannelError::Closed);
        }

        if inner.capacity > 0 && inner.is_full() {
            return Err(ChannelError::Full);
        }

        inner.queue.push_back(msg);
        self.channel.recv_condvar.notify_one();

        Ok(())
    }

    /// Send a message, blocking if the channel is full
    pub fn send_blocking(&self, msg: T) -> ChannelResult<()> {
        let mut inner = self
            .channel
            .inner
            .lock()
            .map_err(|_| ChannelError::Closed)?;

        loop {
            if inner.closed {
                return Err(ChannelError::Closed);
            }

            if inner.capacity == 0 || !inner.is_full() {
                inner.queue.push_back(msg);
                self.channel.recv_condvar.notify_one();
                return Ok(());
            }

            // Wait for space to become available
            match self.channel.send_condvar.wait(inner) {
                Ok(new_inner) => inner = new_inner,
                Err(_) => return Err(ChannelError::Closed),
            }
        }
    }

    /// Try to send a message with a timeout
    pub fn send_timeout(&self, msg: T, timeout: Duration) -> ChannelResult<()> {
        let deadline = Instant::now() + timeout;
        let mut inner = self
            .channel
            .inner
            .lock()
            .map_err(|_| ChannelError::Closed)?;

        loop {
            if inner.closed {
                return Err(ChannelError::Closed);
            }

            if inner.capacity == 0 || !inner.is_full() {
                inner.queue.push_back(msg);
                self.channel.recv_condvar.notify_one();
                return Ok(());
            }

            let now = Instant::now();
            if now >= deadline {
                return Err(ChannelError::Timeout);
            }

            let remaining = deadline - now;
            let (new_inner, timeout_result) = self
                .channel
                .send_condvar
                .wait_timeout(inner, remaining)
                .map_err(|_| ChannelError::Closed)?;

            if timeout_result.timed_out() {
                return Err(ChannelError::Timeout);
            }

            inner = new_inner;
        }
    }

    /// Check if the channel is closed
    pub fn is_closed(&self) -> bool {
        self.channel.inner.lock().map(|i| i.closed).unwrap_or(true)
    }

    /// Get the number of messages in the channel
    pub fn len(&self) -> usize {
        self.channel.inner.lock().map(|i| i.len()).unwrap_or(0)
    }

    /// Check if the channel is empty
    pub fn is_empty(&self) -> bool {
        self.channel
            .inner
            .lock()
            .map(|i| i.is_empty())
            .unwrap_or(true)
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        if let Ok(mut inner) = self.channel.inner.lock() {
            inner.sender_count += 1;
        }

        Self {
            channel: self.channel.clone(),
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        if let Ok(mut inner) = self.channel.inner.lock() {
            inner.sender_count -= 1;

            if inner.sender_count == 0 {
                // Last sender dropped, close the channel
                inner.closed = true;
                self.channel.recv_condvar.notify_all();
            }
        }
    }
}

/// Receiving end of a channel
pub struct Receiver<T> {
    channel: Channel<T>,
}

impl<T> Receiver<T> {
    /// Receive a message from the channel
    ///
    /// Returns `Err(ChannelError::Empty)` if the channel is empty.
    /// Returns `Err(ChannelError::Closed)` if the channel is closed and empty.
    pub fn recv(&self) -> ChannelResult<T> {
        let mut inner = self
            .channel
            .inner
            .lock()
            .map_err(|_| ChannelError::Closed)?;

        if let Some(msg) = inner.queue.pop_front() {
            self.channel.send_condvar.notify_one();
            return Ok(msg);
        }

        if inner.closed {
            return Err(ChannelError::Closed);
        }

        Err(ChannelError::Empty)
    }

    /// Receive a message, blocking until one is available
    pub fn recv_blocking(&self) -> ChannelResult<T> {
        let mut inner = self
            .channel
            .inner
            .lock()
            .map_err(|_| ChannelError::Closed)?;

        loop {
            if let Some(msg) = inner.queue.pop_front() {
                self.channel.send_condvar.notify_one();
                return Ok(msg);
            }

            if inner.closed {
                return Err(ChannelError::Closed);
            }

            // Wait for a message to become available
            match self.channel.recv_condvar.wait(inner) {
                Ok(new_inner) => inner = new_inner,
                Err(_) => return Err(ChannelError::Closed),
            }
        }
    }

    /// Try to receive a message with a timeout
    pub fn recv_timeout(&self, timeout: Duration) -> ChannelResult<T> {
        let deadline = Instant::now() + timeout;
        let mut inner = self
            .channel
            .inner
            .lock()
            .map_err(|_| ChannelError::Closed)?;

        loop {
            if let Some(msg) = inner.queue.pop_front() {
                self.channel.send_condvar.notify_one();
                return Ok(msg);
            }

            if inner.closed {
                return Err(ChannelError::Closed);
            }

            let now = Instant::now();
            if now >= deadline {
                return Err(ChannelError::Timeout);
            }

            let remaining = deadline - now;
            let (new_inner, timeout_result) = self
                .channel
                .recv_condvar
                .wait_timeout(inner, remaining)
                .map_err(|_| ChannelError::Closed)?;

            if timeout_result.timed_out() {
                return Err(ChannelError::Timeout);
            }

            inner = new_inner;
        }
    }

    /// Try to receive a message without blocking
    pub fn try_recv(&self) -> ChannelResult<T> {
        self.recv()
    }

    /// Check if the channel is closed
    pub fn is_closed(&self) -> bool {
        self.channel.inner.lock().map(|i| i.closed).unwrap_or(true)
    }

    /// Get the number of messages in the channel
    pub fn len(&self) -> usize {
        self.channel.inner.lock().map(|i| i.len()).unwrap_or(0)
    }

    /// Check if the channel is empty
    pub fn is_empty(&self) -> bool {
        self.channel
            .inner
            .lock()
            .map(|i| i.is_empty())
            .unwrap_or(true)
    }
}

impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        if let Ok(mut inner) = self.channel.inner.lock() {
            inner.receiver_count += 1;
        }

        Self {
            channel: self.channel.clone(),
        }
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        if let Ok(mut inner) = self.channel.inner.lock() {
            inner.receiver_count -= 1;

            if inner.receiver_count == 0 {
                // Last receiver dropped, close the channel
                inner.closed = true;
                self.channel.send_condvar.notify_all();
            }
        }
    }
}

/// Create a new bounded channel
pub fn channel<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    Channel::new(capacity)
}

/// Create a new unbounded channel
pub fn unbounded<T>() -> (Sender<T>, Receiver<T>) {
    Channel::unbounded()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_bounded_channel() {
        let (tx, rx) = channel::<i32>(2);

        assert!(tx.send(1).is_ok());
        assert!(tx.send(2).is_ok());
        assert_eq!(tx.send(3), Err(ChannelError::Full));

        assert_eq!(rx.recv().expect("recv should succeed in test"), 1);
        assert!(tx.send(3).is_ok());
    }

    #[test]
    fn test_unbounded_channel() {
        let (tx, rx) = unbounded::<i32>();

        for i in 0..100 {
            assert!(tx.send(i).is_ok());
        }

        for i in 0..100 {
            assert_eq!(rx.recv().expect("recv should succeed in test"), i);
        }
    }

    #[test]
    fn test_channel_close() {
        let (tx, rx) = channel::<i32>(10);

        tx.send(1).expect("send should succeed in test");
        drop(tx);

        assert_eq!(rx.recv().expect("recv should succeed in test"), 1);
        assert_eq!(rx.recv(), Err(ChannelError::Closed));
    }

    #[test]
    fn test_channel_blocking() {
        let (tx, rx) = channel::<i32>(1);

        let tx_clone = tx.clone();
        
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(10));
            tx_clone.send_blocking(2)
                .expect("send_blocking should succeed in test");
        });

        tx.send_blocking(1)
            .expect("send_blocking should succeed in test");
        
        assert_eq!(rx.recv_blocking(), Ok(1));
        assert_eq!(rx.recv_blocking(), Ok(2));
    }

    #[test]
    fn test_multiple_senders() {
        let (tx, rx) = channel::<i32>(100);

        let tx2 = tx.clone();

        thread::spawn(move || {
            tx.send(1).expect("send should succeed in test");
        });

        thread::spawn(move || {
            tx2.send(2).expect("send should succeed in test");
        });

        let mut received = vec![];
        received.push(
            rx.recv_blocking()
                .expect("recv_blocking should succeed in test"),
        );
        received.push(
            rx.recv_blocking()
                .expect("recv_blocking should succeed in test"),
        );
        received.sort();

        assert_eq!(received, vec![1, 2]);
    }
}
