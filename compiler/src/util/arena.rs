//! Arena Allocator for BoxLang
//!
//! This module provides efficient arena-based memory allocation for AST and MIR nodes.
//! Features:
//! - Fast bump allocation
//! - Bulk deallocation
//! - Type-safe allocation
//! - Support for growable arenas
//! - Memory usage tracking

use std::alloc::{alloc, dealloc, Layout};
use std::cell::Cell;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ptr::NonNull;

/// Arena allocation error
#[derive(Debug, Clone, PartialEq)]
pub enum ArenaError {
    /// Out of memory
    OutOfMemory,
    /// Invalid layout (e.g., size overflow)
    InvalidLayout,
    /// Alignment not supported
    UnsupportedAlignment,
    /// Arena is full
    ArenaFull,
}

impl std::fmt::Display for ArenaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArenaError::OutOfMemory => write!(f, "out of memory"),
            ArenaError::InvalidLayout => write!(f, "invalid memory layout"),
            ArenaError::UnsupportedAlignment => write!(f, "unsupported alignment"),
            ArenaError::ArenaFull => write!(f, "arena full"),
        }
    }
}

impl std::error::Error for ArenaError {}

/// Result type for arena operations
pub type ArenaResult<T> = Result<T, ArenaError>;

/// A fast bump allocator for arena-style allocation
///
/// Allocates objects from a pre-allocated chunk of memory.
/// All objects are freed when the arena is dropped.
/// This is ideal for AST nodes and MIR structures that have
/// similar lifetimes.
pub struct Arena {
    /// Base pointer to the arena memory
    base: NonNull<u8>,
    /// Current offset into the arena
    offset: Cell<usize>,
    /// Total capacity of the arena
    capacity: usize,
    /// Alignment requirement
    alignment: usize,
    /// Total bytes allocated (statistics)
    total_allocated: Cell<usize>,
    /// Number of allocations
    num_allocations: Cell<usize>,
}

impl Arena {
    /// Default arena capacity (1MB)
    pub const DEFAULT_CAPACITY: usize = 1024 * 1024;
    /// Default alignment (8 bytes)
    pub const DEFAULT_ALIGNMENT: usize = 8;

    /// Create a new arena with the default capacity
    pub fn new() -> ArenaResult<Self> {
        Self::with_capacity(Self::DEFAULT_CAPACITY)
    }

    /// Create a new arena with the given capacity
    pub fn with_capacity(capacity: usize) -> ArenaResult<Self> {
        Self::with_alignment(capacity, Self::DEFAULT_ALIGNMENT)
    }

    /// Create a new arena with custom alignment
    pub fn with_alignment(capacity: usize, alignment: usize) -> ArenaResult<Self> {
        if capacity == 0 {
            return Err(ArenaError::InvalidLayout);
        }

        let layout =
            Layout::from_size_align(capacity, alignment).map_err(|_| ArenaError::InvalidLayout)?;

        unsafe {
            let ptr = alloc(layout);
            if ptr.is_null() {
                return Err(ArenaError::OutOfMemory);
            }

            Ok(Self {
                base: NonNull::new_unchecked(ptr),
                offset: Cell::new(0),
                capacity,
                alignment,
                total_allocated: Cell::new(0),
                num_allocations: Cell::new(0),
            })
        }
    }

    /// Allocate space for a value in the arena
    ///
    /// Returns a mutable reference to the allocated space.
    /// The memory is uninitialized - you must write to it before reading.
    pub fn alloc<T>(&self) -> ArenaResult<&mut MaybeUninit<T>> {
        let size = std::mem::size_of::<T>();
        let align = std::mem::align_of::<T>();

        // Align the offset
        let current_offset = self.offset.get();
        let aligned_offset = (current_offset + align - 1) & !(align - 1);

        if aligned_offset + size > self.capacity {
            return Err(ArenaError::ArenaFull);
        }

        unsafe {
            let ptr = self.base.as_ptr().add(aligned_offset) as *mut MaybeUninit<T>;
            self.offset.set(aligned_offset + size);
            self.total_allocated.set(self.total_allocated.get() + size);
            self.num_allocations.set(self.num_allocations.get() + 1);
            Ok(&mut *ptr)
        }
    }

    /// Allocate and initialize a value in the arena
    pub fn alloc_init<T>(&self, value: T) -> ArenaResult<&mut T> {
        let slot = self.alloc::<T>()?;
        Ok(slot.write(value))
    }

    /// Allocate a slice of values in the arena
    ///
    /// Returns a mutable reference to uninitialized memory.
    pub fn alloc_slice<T>(&self, len: usize) -> ArenaResult<&mut [MaybeUninit<T>]> {
        let size = std::mem::size_of::<T>()
            .checked_mul(len)
            .ok_or(ArenaError::InvalidLayout)?;
        let align = std::mem::align_of::<T>();

        let current_offset = self.offset.get();
        let aligned_offset = (current_offset + align - 1) & !(align - 1);

        if aligned_offset + size > self.capacity {
            return Err(ArenaError::ArenaFull);
        }

        unsafe {
            let ptr = self.base.as_ptr().add(aligned_offset) as *mut MaybeUninit<T>;
            self.offset.set(aligned_offset + size);
            self.total_allocated.set(self.total_allocated.get() + size);
            self.num_allocations.set(self.num_allocations.get() + 1);
            Ok(std::slice::from_raw_parts_mut(ptr, len))
        }
    }

    /// Allocate and initialize a slice from an iterator
    pub fn alloc_slice_from_iter<T, I>(&self, iter: I) -> ArenaResult<&mut [T]>
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        let iter = iter.into_iter();
        let len = iter.len();

        let uninit_slice = self.alloc_slice::<T>(len)?;

        for (i, item) in iter.enumerate() {
            uninit_slice[i].write(item);
        }

        // SAFETY: All elements have been initialized
        unsafe { Ok(&mut *(uninit_slice as *mut [MaybeUninit<T>] as *mut [T])) }
    }

    /// Reset the arena, freeing all allocations
    ///
    /// # Safety
    /// After reset, all previously allocated references become invalid.
    /// This is safe as long as you don't use those references.
    pub fn reset(&self) {
        self.offset.set(0);
        self.total_allocated.set(0);
        self.num_allocations.set(0);
    }

    /// Get the remaining capacity in bytes
    pub fn remaining(&self) -> usize {
        self.capacity - self.offset.get()
    }

    /// Get the used bytes
    pub fn used(&self) -> usize {
        self.offset.get()
    }

    /// Get the total capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get total bytes allocated (including alignment padding)
    pub fn total_allocated(&self) -> usize {
        self.total_allocated.get()
    }

    /// Get number of allocations
    pub fn num_allocations(&self) -> usize {
        self.num_allocations.get()
    }

    /// Get memory usage statistics
    pub fn stats(&self) -> ArenaStats {
        ArenaStats {
            capacity: self.capacity,
            used: self.used(),
            remaining: self.remaining(),
            total_allocated: self.total_allocated(),
            num_allocations: self.num_allocations(),
            utilization: self.used() as f64 / self.capacity as f64,
        }
    }
}

impl Default for Arena {
    fn default() -> Self {
        // Create arena with default capacity - this should never fail with reasonable defaults
        match Self::new() {
            Ok(arena) => arena,
            Err(_) => {
                // If even default arena creation fails, we have bigger problems
                // Create a minimal arena as fallback
                Self::with_capacity(1024).unwrap_or_else(|_| {
                    panic!("Failed to create default arena - system may be out of memory")
                })
            }
        }
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::from_size_align_unchecked(self.capacity, self.alignment);
            dealloc(self.base.as_ptr(), layout);
        }
    }
}

// SAFETY: Arena doesn't share mutable state between threads
unsafe impl Send for Arena {}
unsafe impl Sync for Arena {}

/// Arena memory usage statistics
#[derive(Debug, Clone, Copy)]
pub struct ArenaStats {
    /// Total capacity
    pub capacity: usize,
    /// Bytes used
    pub used: usize,
    /// Bytes remaining
    pub remaining: usize,
    /// Total bytes allocated (including alignment padding)
    pub total_allocated: usize,
    /// Number of allocations
    pub num_allocations: usize,
    /// Utilization ratio (0.0 - 1.0)
    pub utilization: f64,
}

/// A typed arena for allocating values of a specific type
///
/// This is more efficient than the general Arena when you know
/// you'll only be allocating one type of value.
pub struct TypedArena<T> {
    /// Base pointer
    base: NonNull<T>,
    /// Current offset (in elements, not bytes)
    offset: Cell<usize>,
    /// Capacity in elements
    capacity: usize,
    /// Marker for type
    _marker: PhantomData<T>,
}

impl<T> TypedArena<T> {
    /// Create a new typed arena with the given capacity
    pub fn with_capacity(capacity: usize) -> ArenaResult<Self> {
        if capacity == 0 {
            return Err(ArenaError::InvalidLayout);
        }

        let size = std::mem::size_of::<T>()
            .checked_mul(capacity)
            .ok_or(ArenaError::InvalidLayout)?;
        let align = std::mem::align_of::<T>();

        let layout = Layout::from_size_align(size, align).map_err(|_| ArenaError::InvalidLayout)?;

        unsafe {
            let ptr = alloc(layout) as *mut T;
            if ptr.is_null() {
                return Err(ArenaError::OutOfMemory);
            }

            Ok(Self {
                base: NonNull::new_unchecked(ptr),
                offset: Cell::new(0),
                capacity,
                _marker: PhantomData,
            })
        }
    }

    /// Allocate a value in the arena
    pub fn alloc(&self) -> ArenaResult<&mut MaybeUninit<T>> {
        let current = self.offset.get();
        if current >= self.capacity {
            return Err(ArenaError::ArenaFull);
        }

        unsafe {
            let ptr = self.base.as_ptr().add(current);
            self.offset.set(current + 1);
            Ok(&mut *(ptr as *mut MaybeUninit<T>))
        }
    }

    /// Allocate and initialize a value
    pub fn alloc_init(&self, value: T) -> ArenaResult<&mut T> {
        let slot = self.alloc()?;
        Ok(slot.write(value))
    }

    /// Allocate multiple values
    pub fn alloc_many(&self, count: usize) -> ArenaResult<&mut [MaybeUninit<T>]> {
        let current = self.offset.get();
        if current + count > self.capacity {
            return Err(ArenaError::ArenaFull);
        }

        unsafe {
            let ptr = self.base.as_ptr().add(current);
            self.offset.set(current + count);
            Ok(std::slice::from_raw_parts_mut(
                ptr as *mut MaybeUninit<T>,
                count,
            ))
        }
    }

    /// Reset the arena
    pub fn reset(&self) {
        self.offset.set(0);
    }

    /// Get remaining capacity
    pub fn remaining(&self) -> usize {
        self.capacity - self.offset.get()
    }

    /// Get used count
    pub fn used(&self) -> usize {
        self.offset.get()
    }

    /// Get total capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl<T> Drop for TypedArena<T> {
    fn drop(&mut self) {
        unsafe {
            // Drop all initialized values
            for i in 0..self.offset.get() {
                let ptr = self.base.as_ptr().add(i);
                std::ptr::drop_in_place(ptr);
            }

            let size = std::mem::size_of::<T>() * self.capacity;
            let align = std::mem::align_of::<T>();
            let layout = Layout::from_size_align_unchecked(size, align);
            dealloc(self.base.as_ptr() as *mut u8, layout);
        }
    }
}

// SAFETY: TypedArena doesn't share mutable state between threads
unsafe impl<T: Send> Send for TypedArena<T> {}
unsafe impl<T: Sync> Sync for TypedArena<T> {}

/// A growable arena that can expand as needed
///
/// Unlike Arena, this can grow to accommodate more allocations.
pub struct GrowableArena {
    /// List of chunks
    chunks: Vec<ArenaChunk>,
    /// Current chunk index
    current_chunk: usize,
    /// Chunk size
    chunk_size: usize,
    /// Alignment
    alignment: usize,
}

struct ArenaChunk {
    base: NonNull<u8>,
    offset: Cell<usize>,
    capacity: usize,
    alignment: usize,
}

impl GrowableArena {
    /// Create a new growable arena
    pub fn new() -> ArenaResult<Self> {
        Self::with_chunk_size(Arena::DEFAULT_CAPACITY)
    }

    /// Create with custom chunk size
    pub fn with_chunk_size(chunk_size: usize) -> ArenaResult<Self> {
        let mut chunks = Vec::new();
        chunks.push(Self::create_chunk(chunk_size, Arena::DEFAULT_ALIGNMENT)?);

        Ok(Self {
            chunks,
            current_chunk: 0,
            chunk_size,
            alignment: Arena::DEFAULT_ALIGNMENT,
        })
    }

    fn create_chunk(capacity: usize, alignment: usize) -> ArenaResult<ArenaChunk> {
        let layout =
            Layout::from_size_align(capacity, alignment).map_err(|_| ArenaError::InvalidLayout)?;

        unsafe {
            let ptr = alloc(layout);
            if ptr.is_null() {
                return Err(ArenaError::OutOfMemory);
            }

            Ok(ArenaChunk {
                base: NonNull::new_unchecked(ptr),
                offset: Cell::new(0),
                capacity,
                alignment,
            })
        }
    }

    /// Allocate space for a value
    pub fn alloc<T>(&self) -> ArenaResult<&mut MaybeUninit<T>> {
        let size = std::mem::size_of::<T>();
        let align = std::mem::align_of::<T>();

        // Try to allocate in current chunk
        if let Some(result) = self.try_alloc_in_current_chunk(size, align) {
            return unsafe { Ok(&mut *(result as *mut MaybeUninit<T>)) };
        }

        // Need to grow
        Err(ArenaError::ArenaFull)
    }

    fn try_alloc_in_current_chunk(&self, size: usize, align: usize) -> Option<*mut u8> {
        let chunk = self.chunks.get(self.current_chunk)?;

        let current_offset = chunk.offset.get();
        let aligned_offset = (current_offset + align - 1) & !(align - 1);

        if aligned_offset + size > chunk.capacity {
            return None;
        }

        unsafe {
            let ptr = chunk.base.as_ptr().add(aligned_offset);
            chunk.offset.set(aligned_offset + size);
            Some(ptr)
        }
    }

    /// Grow the arena by adding a new chunk
    pub fn grow(&mut self) -> ArenaResult<()> {
        let new_chunk = Self::create_chunk(self.chunk_size, self.alignment)?;
        self.chunks.push(new_chunk);
        self.current_chunk += 1;
        Ok(())
    }

    /// Reset all chunks
    pub fn reset(&self) {
        for chunk in &self.chunks {
            chunk.offset.set(0);
        }
    }

    /// Get total used bytes across all chunks
    pub fn total_used(&self) -> usize {
        self.chunks.iter().map(|c| c.offset.get()).sum()
    }

    /// Get total capacity across all chunks
    pub fn total_capacity(&self) -> usize {
        self.chunks.iter().map(|c| c.capacity).sum()
    }
}

impl Drop for GrowableArena {
    fn drop(&mut self) {
        for chunk in &self.chunks {
            unsafe {
                let layout = Layout::from_size_align_unchecked(chunk.capacity, chunk.alignment);
                dealloc(chunk.base.as_ptr(), layout);
            }
        }
    }
}

// SAFETY: GrowableArena doesn't share mutable state between threads
unsafe impl Send for GrowableArena {}
unsafe impl Sync for GrowableArena {}

/// A bump allocator that can be reset frequently
///
/// This is optimized for short-lived allocations that are all freed at once.
pub struct BumpAlloc {
    /// Base pointer
    base: NonNull<u8>,
    /// Current offset
    offset: Cell<usize>,
    /// Capacity
    capacity: usize,
    /// Alignment
    alignment: usize,
}

impl BumpAlloc {
    /// Create a new bump allocator
    pub fn new(capacity: usize) -> ArenaResult<Self> {
        Self::with_alignment(capacity, Arena::DEFAULT_ALIGNMENT)
    }

    /// Create with custom alignment
    pub fn with_alignment(capacity: usize, alignment: usize) -> ArenaResult<Self> {
        let layout =
            Layout::from_size_align(capacity, alignment).map_err(|_| ArenaError::InvalidLayout)?;

        unsafe {
            let ptr = alloc(layout);
            if ptr.is_null() {
                return Err(ArenaError::OutOfMemory);
            }

            Ok(Self {
                base: NonNull::new_unchecked(ptr),
                offset: Cell::new(0),
                capacity,
                alignment,
            })
        }
    }

    /// Allocate memory
    pub fn alloc<T>(&self) -> ArenaResult<&mut MaybeUninit<T>> {
        let size = std::mem::size_of::<T>();
        let align = std::mem::align_of::<T>();

        let current_offset = self.offset.get();
        let aligned_offset = (current_offset + align - 1) & !(align - 1);

        if aligned_offset + size > self.capacity {
            return Err(ArenaError::ArenaFull);
        }

        unsafe {
            let ptr = self.base.as_ptr().add(aligned_offset) as *mut MaybeUninit<T>;
            self.offset.set(aligned_offset + size);
            Ok(&mut *ptr)
        }
    }

    /// Reset the allocator
    pub fn reset(&self) {
        self.offset.set(0);
    }

    /// Get remaining capacity
    pub fn remaining(&self) -> usize {
        self.capacity - self.offset.get()
    }

    /// Get used bytes
    pub fn used(&self) -> usize {
        self.offset.get()
    }
}

impl Drop for BumpAlloc {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::from_size_align_unchecked(self.capacity, self.alignment);
            dealloc(self.base.as_ptr(), layout);
        }
    }
}

// SAFETY: BumpAlloc doesn't share mutable state between threads
unsafe impl Send for BumpAlloc {}
unsafe impl Sync for BumpAlloc {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_creation() {
        let arena = Arena::new();
        assert!(arena.is_ok());

        let arena = arena.unwrap();
        assert_eq!(arena.capacity(), Arena::DEFAULT_CAPACITY);
    }

    #[test]
    fn test_arena_allocation() {
        let arena = Arena::new().unwrap();

        // Allocate an integer
        let ptr: &mut MaybeUninit<i32> = arena.alloc().unwrap();
        ptr.write(42);

        // SAFETY: We just wrote to it
        unsafe {
            assert_eq!((*ptr).assume_init(), 42);
        }
    }

    #[test]
    fn test_arena_alloc_init() {
        let arena = Arena::new().unwrap();

        let value: &mut i32 = arena.alloc_init(42).unwrap();
        assert_eq!(*value, 42);
    }

    #[test]
    fn test_arena_slice_allocation() {
        let arena = Arena::new().unwrap();

        let slice: &mut [MaybeUninit<i32>] = arena.alloc_slice::<i32>(10).unwrap();
        assert_eq!(slice.len(), 10);

        // Initialize all elements
        for (i, slot) in slice.iter_mut().enumerate() {
            slot.write(i as i32);
        }
    }

    #[test]
    fn test_arena_reset() {
        let arena = Arena::new().unwrap();

        // Allocate some values
        for _ in 0..100 {
            let _: &mut MaybeUninit<i64> = arena.alloc().unwrap();
        }

        assert!(arena.used() > 0);

        arena.reset();
        assert_eq!(arena.used(), 0);
    }

    #[test]
    fn test_arena_stats() {
        let arena = Arena::new().unwrap();

        let _ = arena.alloc_init(42i32);
        let stats = arena.stats();

        assert_eq!(stats.capacity, Arena::DEFAULT_CAPACITY);
        assert!(stats.used > 0);
        assert_eq!(stats.num_allocations, 1);
    }

    #[test]
    fn test_typed_arena() {
        let arena = TypedArena::<i32>::with_capacity(100).unwrap();

        let v1 = arena.alloc_init(1).unwrap();
        let v2 = arena.alloc_init(2).unwrap();

        assert_eq!(*v1, 1);
        assert_eq!(*v2, 2);
        assert_eq!(arena.used(), 2);
    }

    #[test]
    fn test_typed_arena_many() {
        let arena = TypedArena::<i32>::with_capacity(100).unwrap();

        let slice = arena.alloc_many(10).unwrap();
        assert_eq!(slice.len(), 10);

        // Initialize
        for (i, slot) in slice.iter_mut().enumerate() {
            slot.write(i as i32);
        }
    }

    #[test]
    fn test_bump_alloc() {
        let bump = BumpAlloc::new(1024).unwrap();

        let v1 = bump.alloc::<i32>().unwrap();
        v1.write(10);

        let v2 = bump.alloc::<i64>().unwrap();
        v2.write(20);

        assert!(bump.used() > 0);

        bump.reset();
        assert_eq!(bump.used(), 0);
    }

    #[test]
    fn test_arena_full() {
        // Create a small arena
        let arena = Arena::with_capacity(16).unwrap();

        // This should fill it up
        let result = arena.alloc::<[u8; 32]>();
        assert!(result.is_err());
    }

    #[test]
    fn test_alloc_slice_from_iter() {
        let arena = Arena::new().unwrap();

        let values = vec![1, 2, 3, 4, 5];
        let slice = arena.alloc_slice_from_iter(values).unwrap();

        assert_eq!(slice, &[1, 2, 3, 4, 5]);
    }
}
