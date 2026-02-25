//! Memory management for BoxLang
//!
//! Provides multiple allocation strategies:
//! - Stack allocation (default for small, short-lived data)
//! - Heap allocation via Box (for owned data)
//! - Arena allocation (for bulk allocation with bulk free)
//! - Pool allocation (for reusable objects)
//! - Stack pools (for stack-like allocation patterns)

use std::alloc::{alloc, dealloc, Layout};
use std::cell::RefCell;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ptr::NonNull;

/// Memory allocation error
#[derive(Debug, Clone, PartialEq)]
pub enum AllocError {
    /// Out of memory
    OutOfMemory,
    /// Invalid layout (e.g., size overflow)
    InvalidLayout,
    /// Alignment not supported
    UnsupportedAlignment,
    /// Pool exhausted
    PoolExhausted,
    /// Arena full
    ArenaFull,
}

impl std::fmt::Display for AllocError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AllocError::OutOfMemory => write!(f, "out of memory"),
            AllocError::InvalidLayout => write!(f, "invalid memory layout"),
            AllocError::UnsupportedAlignment => write!(f, "unsupported alignment"),
            AllocError::PoolExhausted => write!(f, "pool exhausted"),
            AllocError::ArenaFull => write!(f, "arena full"),
        }
    }
}

impl std::error::Error for AllocError {}

/// Result type for allocation operations
pub type AllocResult<T> = Result<T, AllocError>;

/// Statistics for memory usage
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    /// Total bytes allocated
    pub total_allocated: usize,
    /// Total bytes deallocated
    pub total_deallocated: usize,
    /// Current bytes in use
    pub current_usage: usize,
    /// Peak memory usage
    pub peak_usage: usize,
    /// Number of allocations
    pub num_allocations: usize,
    /// Number of deallocations
    pub num_deallocations: usize,
}

impl MemoryStats {
    /// Record an allocation
    pub fn record_alloc(&mut self, size: usize) {
        self.total_allocated += size;
        self.current_usage += size;
        self.num_allocations += 1;
        if self.current_usage > self.peak_usage {
            self.peak_usage = self.current_usage;
        }
    }

    /// Record a deallocation
    pub fn record_dealloc(&mut self, size: usize) {
        self.total_deallocated += size;
        self.current_usage -= size;
        self.num_deallocations += 1;
    }
}

/// Thread-local memory statistics
thread_local! {
    static MEMORY_STATS: RefCell<MemoryStats> = RefCell::new(MemoryStats::default());
}

/// Get memory statistics
pub fn memory_stats() -> MemoryStats {
    MEMORY_STATS.with(|stats| stats.borrow().clone())
}

/// Reset memory statistics
pub fn reset_memory_stats() {
    MEMORY_STATS.with(|stats| {
        *stats.borrow_mut() = MemoryStats::default();
    });
}

/// Record an allocation in statistics
fn record_alloc(size: usize) {
    MEMORY_STATS.with(|stats| {
        stats.borrow_mut().record_alloc(size);
    });
}

/// Record a deallocation in statistics
fn record_dealloc(size: usize) {
    MEMORY_STATS.with(|stats| {
        stats.borrow_mut().record_dealloc(size);
    });
}

/// Box type for heap allocation
///
/// Similar to Rust's Box, but with additional tracking
pub struct Box<T> {
    ptr: NonNull<T>,
    _marker: PhantomData<T>,
}

impl<T> Box<T> {
    /// Allocate a new Box
    pub fn new(value: T) -> AllocResult<Self> {
        let layout = Layout::new::<T>();

        // Check for zero-sized types
        if layout.size() == 0 {
            // For ZSTs, we don't need to allocate
            return Ok(Self {
                ptr: NonNull::dangling(),
                _marker: PhantomData,
            });
        }

        unsafe {
            let ptr = alloc(layout) as *mut T;
            if ptr.is_null() {
                return Err(AllocError::OutOfMemory);
            }

            // Use write for safe initialization
            ptr.write(value);
            record_alloc(layout.size());

            // Use NonNull::new for safety, then unwrap_unchecked for performance
            // after null check
            match NonNull::new(ptr) {
                Some(non_null) => Ok(Self {
                    ptr: non_null,
                    _marker: PhantomData,
                }),
                None => {
                    // This should never happen due to null check above
                    std::hint::unreachable_unchecked()
                }
            }
        }
    }

    /// Create a new Box without initializing the value
    ///
    /// # Safety
    /// The caller must ensure the value is properly initialized before use
    pub unsafe fn new_uninit() -> AllocResult<Box<MaybeUninit<T>>> {
        let layout = Layout::new::<T>();

        let ptr = alloc(layout) as *mut MaybeUninit<T>;
        if ptr.is_null() {
            return Err(AllocError::OutOfMemory);
        }

        record_alloc(layout.size());

        Ok(Box {
            ptr: NonNull::new_unchecked(ptr),
            _marker: PhantomData,
        })
    }

    /// Get a reference to the value
    pub fn as_ref(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }

    /// Get a mutable reference to the value
    pub fn as_mut(&mut self) -> &mut T {
        unsafe { self.ptr.as_mut() }
    }

    /// Convert to a raw pointer
    pub fn into_raw(self) -> *mut T {
        let ptr = self.ptr.as_ptr();
        std::mem::forget(self);
        ptr
    }

    /// Create from a raw pointer
    ///
    /// # Safety
    /// The pointer must have been obtained from Box::into_raw
    pub unsafe fn from_raw(ptr: *mut T) -> Self {
        Self {
            ptr: NonNull::new_unchecked(ptr),
            _marker: PhantomData,
        }
    }

    /// Get the size of the allocation
    fn size() -> usize {
        std::mem::size_of::<T>()
    }
}

impl<T> std::ops::Deref for Box<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.as_ref()
    }
}

impl<T> std::ops::DerefMut for Box<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.as_mut()
    }
}

impl<T> Drop for Box<T> {
    fn drop(&mut self) {
        let layout = Layout::new::<T>();

        // Skip deallocation for zero-sized types
        if layout.size() == 0 {
            return;
        }

        unsafe {
            // Drop the value first
            std::ptr::drop_in_place(self.ptr.as_ptr());
            // Then deallocate the memory
            dealloc(self.ptr.as_ptr() as *mut u8, layout);
            record_dealloc(layout.size());
        }
    }
}

impl<T: Clone> Clone for Box<T> {
    fn clone(&self) -> Self {
        match Self::new((**self).clone()) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Box clone failed: {}", e);
                std::process::abort()
            }
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Box<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&**self, f)
    }
}

impl<T: std::fmt::Display> std::fmt::Display for Box<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&**self, f)
    }
}

unsafe impl<T: Send> Send for Box<T> {}
unsafe impl<T: Sync> Sync for Box<T> {}

/// Arena allocator for bulk allocation
///
/// Allocates objects from a pre-allocated chunk of memory.
/// All objects are freed when the arena is dropped.
pub struct Arena {
    /// Base pointer to the arena memory
    base: NonNull<u8>,
    /// Current offset into the arena
    offset: usize,
    /// Total capacity of the arena
    capacity: usize,
    /// Alignment requirement
    alignment: usize,
}

impl Arena {
    /// Create a new arena with the given capacity
    pub fn new(capacity: usize) -> AllocResult<Self> {
        Self::with_alignment(capacity, 8)
    }

    /// Create a new arena with custom alignment
    pub fn with_alignment(capacity: usize, alignment: usize) -> AllocResult<Self> {
        let layout =
            Layout::from_size_align(capacity, alignment).map_err(|_| AllocError::InvalidLayout)?;

        unsafe {
            let ptr = alloc(layout);
            if ptr.is_null() {
                return Err(AllocError::OutOfMemory);
            }

            record_alloc(capacity);

            Ok(Self {
                base: NonNull::new_unchecked(ptr),
                offset: 0,
                capacity,
                alignment,
            })
        }
    }

    /// Allocate an object in the arena
    pub fn alloc<T>(&mut self) -> AllocResult<&mut T> {
        let size = std::mem::size_of::<T>();
        let align = std::mem::align_of::<T>();

        // Align the offset
        let aligned_offset = (self.offset + align - 1) & !(align - 1);

        if aligned_offset + size > self.capacity {
            return Err(AllocError::ArenaFull);
        }

        unsafe {
            let ptr = self.base.as_ptr().add(aligned_offset) as *mut T;
            self.offset = aligned_offset + size;
            Ok(&mut *ptr)
        }
    }

    /// Allocate and initialize an object
    pub fn alloc_init<T>(&mut self, value: T) -> AllocResult<&mut T> {
        let slot = self.alloc::<T>()?;
        *slot = value;
        Ok(slot)
    }

    /// Allocate a slice of objects
    pub fn alloc_slice<T>(&mut self, len: usize) -> AllocResult<&mut [T]> {
        let size = std::mem::size_of::<T>() * len;
        let align = std::mem::align_of::<T>();

        let aligned_offset = (self.offset + align - 1) & !(align - 1);

        if aligned_offset + size > self.capacity {
            return Err(AllocError::ArenaFull);
        }

        unsafe {
            let ptr = self.base.as_ptr().add(aligned_offset) as *mut T;
            self.offset = aligned_offset + size;
            Ok(std::slice::from_raw_parts_mut(ptr, len))
        }
    }

    /// Reset the arena, freeing all allocations
    pub fn reset(&mut self) {
        self.offset = 0;
    }

    /// Get the remaining capacity
    pub fn remaining(&self) -> usize {
        self.capacity - self.offset
    }

    /// Get the used bytes
    pub fn used(&self) -> usize {
        self.offset
    }

    /// Get the total capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::from_size_align_unchecked(self.capacity, self.alignment);
            dealloc(self.base.as_ptr(), layout);
            record_dealloc(self.capacity);
        }
    }
}

unsafe impl Send for Arena {}
unsafe impl Sync for Arena {}

/// Object pool for reusable objects
///
/// Pre-allocates a fixed number of objects and reuses them.
/// Useful for objects with expensive construction/destruction.
pub struct Pool<T> {
    /// Storage for objects
    storage: Vec<MaybeUninit<T>>,
    /// Stack of available indices
    available: Vec<usize>,
    /// Capacity of the pool
    capacity: usize,
}

impl<T> Pool<T> {
    /// Create a new pool with the given capacity
    pub fn new(capacity: usize) -> Self {
        let mut storage = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            storage.push(MaybeUninit::uninit());
        }

        let available: Vec<usize> = (0..capacity).collect();

        Self {
            storage,
            available,
            capacity,
        }
    }

    /// Acquire an object from the pool
    pub fn acquire<F>(&mut self, init: F) -> AllocResult<PoolGuard<'_, T>>
    where
        F: FnOnce() -> T,
    {
        match self.available.pop() {
            Some(index) => {
                unsafe {
                    self.storage[index].as_mut_ptr().write(init());
                }
                Ok(PoolGuard {
                    pool: self,
                    index,
                    _marker: PhantomData,
                })
            }
            None => Err(AllocError::PoolExhausted),
        }
    }

    /// Try to acquire an object without initializing
    pub fn try_acquire(&mut self) -> AllocResult<PoolGuard<'_, T>>
    where
        T: Default,
    {
        self.acquire(T::default)
    }

    /// Get the number of available objects
    pub fn available(&self) -> usize {
        self.available.len()
    }

    /// Get the number of objects in use
    pub fn in_use(&self) -> usize {
        self.capacity - self.available.len()
    }

    /// Get the total capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Check if the pool is exhausted
    pub fn is_exhausted(&self) -> bool {
        self.available.is_empty()
    }

    /// Release an object back to the pool
    unsafe fn release(&mut self, index: usize) {
        std::ptr::drop_in_place(self.storage[index].as_mut_ptr());
        self.available.push(index);
    }
}

impl<T> Drop for Pool<T> {
    fn drop(&mut self) {
        // Drop all initialized objects
        for i in 0..self.capacity {
            if !self.available.contains(&i) {
                unsafe {
                    std::ptr::drop_in_place(self.storage[i].as_mut_ptr());
                }
            }
        }
    }
}

/// Guard for a pooled object
pub struct PoolGuard<'a, T> {
    pool: *mut Pool<T>,
    index: usize,
    _marker: PhantomData<&'a mut T>,
}

impl<'a, T> PoolGuard<'a, T> {
    /// Get a reference to the object
    pub fn get(&self) -> &T {
        unsafe { &*((&(*self.pool).storage)[self.index].as_ptr()) }
    }

    /// Get a mutable reference to the object
    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *((&mut (*self.pool).storage)[self.index].as_mut_ptr()) }
    }
}

impl<'a, T> std::ops::Deref for PoolGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.get()
    }
}

impl<'a, T> std::ops::DerefMut for PoolGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.get_mut()
    }
}

impl<'a, T> Drop for PoolGuard<'a, T> {
    fn drop(&mut self) {
        unsafe {
            (*self.pool).release(self.index);
        }
    }
}

/// Stack allocator for stack-like allocation patterns
///
/// Allocates objects in a stack-like fashion (LIFO).
/// Objects must be freed in reverse order of allocation.
pub struct StackAlloc {
    /// Base pointer
    base: NonNull<u8>,
    /// Current offset
    offset: usize,
    /// Capacity
    capacity: usize,
    /// Stack of allocation sizes for proper deallocation
    sizes: Vec<usize>,
}

impl StackAlloc {
    /// Create a new stack allocator
    pub fn new(capacity: usize) -> AllocResult<Self> {
        let layout = Layout::from_size_align(capacity, 8).map_err(|_| AllocError::InvalidLayout)?;

        unsafe {
            let ptr = alloc(layout);
            if ptr.is_null() {
                return Err(AllocError::OutOfMemory);
            }

            record_alloc(capacity);

            Ok(Self {
                base: NonNull::new_unchecked(ptr),
                offset: 0,
                capacity,
                sizes: Vec::new(),
            })
        }
    }

    /// Push an allocation onto the stack
    pub fn push<T>(&mut self, value: T) -> AllocResult<&mut T> {
        let size = std::mem::size_of::<T>();
        let align = std::mem::align_of::<T>();

        let aligned_offset = (self.offset + align - 1) & !(align - 1);

        if aligned_offset + size > self.capacity {
            return Err(AllocError::OutOfMemory);
        }

        unsafe {
            let ptr = self.base.as_ptr().add(aligned_offset) as *mut T;
            ptr.write(value);
            self.offset = aligned_offset + size;
            self.sizes.push(size);
            Ok(&mut *ptr)
        }
    }

    /// Pop the last allocation from the stack
    pub fn pop<T>(&mut self) -> Option<T> {
        let size = self.sizes.pop()?;
        self.offset -= size;

        unsafe {
            let ptr = self.base.as_ptr().add(self.offset) as *mut T;
            Some(ptr.read())
        }
    }

    /// Reset the stack allocator
    pub fn reset(&mut self) {
        // Drop all objects in reverse order
        while let Some(size) = self.sizes.pop() {
            self.offset -= size;
        }
    }

    /// Get the remaining capacity
    pub fn remaining(&self) -> usize {
        self.capacity - self.offset
    }

    /// Get the used bytes
    pub fn used(&self) -> usize {
        self.offset
    }

    /// Get the number of allocations
    pub fn num_allocs(&self) -> usize {
        self.sizes.len()
    }
}

impl Drop for StackAlloc {
    fn drop(&mut self) {
        self.reset();
        unsafe {
            let layout = Layout::from_size_align_unchecked(self.capacity, 8);
            dealloc(self.base.as_ptr(), layout);
            record_dealloc(self.capacity);
        }
    }
}

unsafe impl Send for StackAlloc {}
unsafe impl Sync for StackAlloc {}

/// Bump allocator (fast, but can only free all at once)
///
/// Similar to Arena, but with a simpler implementation.
/// Very fast allocation, but no individual deallocation.
pub struct BumpAlloc {
    /// Base pointer
    base: NonNull<u8>,
    /// Current offset
    offset: usize,
    /// Capacity
    capacity: usize,
}

impl BumpAlloc {
    /// Create a new bump allocator
    pub fn new(capacity: usize) -> AllocResult<Self> {
        let layout = Layout::from_size_align(capacity, 8).map_err(|_| AllocError::InvalidLayout)?;

        unsafe {
            let ptr = alloc(layout);
            if ptr.is_null() {
                return Err(AllocError::OutOfMemory);
            }

            record_alloc(capacity);

            Ok(Self {
                base: NonNull::new_unchecked(ptr),
                offset: 0,
                capacity,
            })
        }
    }

    /// Allocate memory
    pub fn alloc<T>(&mut self) -> AllocResult<&mut T> {
        let size = std::mem::size_of::<T>();
        let align = std::mem::align_of::<T>();

        let aligned_offset = (self.offset + align - 1) & !(align - 1);

        if aligned_offset + size > self.capacity {
            return Err(AllocError::OutOfMemory);
        }

        unsafe {
            let ptr = self.base.as_ptr().add(aligned_offset) as *mut T;
            self.offset = aligned_offset + size;
            Ok(&mut *ptr)
        }
    }

    /// Allocate a slice
    pub fn alloc_slice<T>(&mut self, len: usize) -> AllocResult<&mut [T]> {
        let size = std::mem::size_of::<T>() * len;
        let align = std::mem::align_of::<T>();

        let aligned_offset = (self.offset + align - 1) & !(align - 1);

        if aligned_offset + size > self.capacity {
            return Err(AllocError::OutOfMemory);
        }

        unsafe {
            let ptr = self.base.as_ptr().add(aligned_offset) as *mut T;
            self.offset = aligned_offset + size;
            Ok(std::slice::from_raw_parts_mut(ptr, len))
        }
    }

    /// Reset the allocator
    pub fn reset(&mut self) {
        self.offset = 0;
    }

    /// Get the remaining capacity
    pub fn remaining(&self) -> usize {
        self.capacity - self.offset
    }

    /// Get the used bytes
    pub fn used(&self) -> usize {
        self.offset
    }
}

impl Drop for BumpAlloc {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::from_size_align_unchecked(self.capacity, 8);
            dealloc(self.base.as_ptr(), layout);
            record_dealloc(self.capacity);
        }
    }
}

unsafe impl Send for BumpAlloc {}
unsafe impl Sync for BumpAlloc {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box() {
        let b = Box::new(42).expect("Box allocation should succeed in test");
        assert_eq!(*b, 42);
    }

    #[test]
    fn test_arena() {
        let mut arena = Arena::new(1024).expect("Arena creation should succeed in test");

        // Test allocation
        let a: &mut i32 = arena
            .alloc()
            .expect("Arena allocation should succeed in test");
        *a = 42;
        assert_eq!(*a, 42);

        // Test slice allocation
        let slice = arena
            .alloc_slice::<i32>(10)
            .expect("Arena slice allocation should succeed in test");
        slice[0] = 1;
        slice[9] = 10;
        assert_eq!(slice[0], 1);
        assert_eq!(slice[9], 10);

        // Test reset
        arena.reset();
        assert_eq!(arena.used(), 0);

        // After reset, we can allocate again
        let d: &mut i32 = arena
            .alloc()
            .expect("Arena allocation after reset should succeed in test");
        *d = 100;
        assert_eq!(*d, 100);
    }

    #[test]
    fn test_pool() {
        let mut pool = Pool::<i32>::new(3);

        {
            let guard1 = pool
                .acquire(|| 1)
                .expect("Pool acquire should succeed in test");
            assert_eq!(*guard1, 1);
        }

        {
            let guard2 = pool
                .acquire(|| 2)
                .expect("Pool acquire should succeed in test");
            assert_eq!(*guard2, 2);
        }

        // After guards are dropped, objects are returned to pool
        assert_eq!(pool.available(), 3);

        // Can acquire again
        let guard = pool
            .acquire(|| 3)
            .expect("Pool acquire should succeed in test");
        assert_eq!(*guard, 3);
    }

    #[test]
    fn test_stack_alloc() {
        let mut stack = StackAlloc::new(1024).expect("StackAlloc creation should succeed in test");

        stack.push(1).expect("Stack push should succeed in test");
        stack.push(2).expect("Stack push should succeed in test");
        stack.push(3).expect("Stack push should succeed in test");

        assert_eq!(stack.pop::<i32>(), Some(3));
        assert_eq!(stack.pop::<i32>(), Some(2));
        assert_eq!(stack.pop::<i32>(), Some(1));
        assert_eq!(stack.pop::<i32>(), None);
    }

    #[test]
    fn test_bump_alloc() {
        let mut bump = BumpAlloc::new(1024).expect("BumpAlloc creation should succeed in test");

        // Test allocation
        let a: &mut i32 = bump
            .alloc()
            .expect("BumpAlloc allocation should succeed in test");
        *a = 42;
        assert_eq!(*a, 42);

        // Test slice allocation
        let slice = bump
            .alloc_slice::<i32>(10)
            .expect("BumpAlloc slice allocation should succeed in test");
        slice[0] = 1;
        slice[9] = 10;
        assert_eq!(slice[0], 1);
        assert_eq!(slice[9], 10);

        // Test reset
        bump.reset();
        assert_eq!(bump.used(), 0);

        // After reset, we can allocate again
        let d: &mut i32 = bump
            .alloc()
            .expect("BumpAlloc allocation after reset should succeed in test");
        *d = 100;
        assert_eq!(*d, 100);
    }

    #[test]
    fn test_memory_stats() {
        reset_memory_stats();

        {
            let _b1 = Box::new(1).expect("Box allocation should succeed in test");
            let _b2 = Box::new(2).expect("Box allocation should succeed in test");
        }

        let stats = memory_stats();
        assert!(stats.num_allocations >= 2);
        assert!(stats.num_deallocations >= 2);
    }
}
