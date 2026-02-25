//! Utility modules for BoxLang compiler
//!
//! This module provides various utility functions and data structures
//! used throughout the compiler.

pub mod arena;

pub use arena::{Arena, ArenaError, ArenaResult, ArenaStats, BumpAlloc, GrowableArena, TypedArena};
