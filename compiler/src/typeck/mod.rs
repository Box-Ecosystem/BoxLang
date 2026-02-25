//! Type checking module

pub mod check;
pub mod error;
pub mod sym;
pub mod ty;
pub mod typeclass;

pub use check::{type_check, TypeChecker};
pub use error::{TypeError, TypeErrors, TypeResult};
pub use sym::{Symbol, SymbolKind, SymbolTable};
pub use ty::{Mutability, Ty};
pub use typeclass::{TypeClass, TypeClassError, TypeClassRegistry};
