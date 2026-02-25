//! Middle-end Module
//!
//! The middle-end performs analysis and transformations on MIR:
//! - MIR: Mid-level Intermediate Representation
//! - Borrow Checker: Ownership and lifetime analysis
//! - Async Transform: Async/await state machine transformation
//! - Query System: Incremental compilation support

pub mod async_transform;
pub mod borrowck;
pub mod mir;
pub mod query;

// Re-exports
pub use async_transform::{AsyncStateMachineGenerator, MirAsyncTransformer};
pub use mir::{BasicBlock, Local, MirBody};
