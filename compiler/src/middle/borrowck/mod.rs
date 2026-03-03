//! Borrow Checker Module
//!
//! The borrow checker enforces Rust's ownership and borrowing rules:
//! - Ownership tracking
//! - Borrow validation
//! - Lifetime analysis
//! - Region inference

use crate::middle::mir::MirBody;

pub mod check;
pub mod loan;
pub mod nll;

pub use check::BorrowChecker;
pub use loan::{BorrowKind, BorrowState, Loan, LoanId, MoveState};
pub use nll::NllBorrowChecker;

/// Borrow checker error
#[derive(Debug, Clone)]
pub enum BorrowError {
    /// Cannot move out of borrowed content
    CannotMoveOutOfBorrow,
    /// Cannot borrow mutably while borrowed immutably
    CannotBorrowMutably,
    /// Cannot borrow immutably while borrowed mutably
    CannotBorrowImmutably,
    /// Use of moved value
    UseOfMovedValue(crate::middle::mir::Local),
    /// Lifetime mismatch
    LifetimeMismatch,
    /// NLL: Two borrows conflict
    ConflictingBorrows {
        loan1: LoanId,
        loan2: LoanId,
        place1: crate::middle::mir::Place,
        place2: crate::middle::mir::Place,
    },
    /// NLL: Write to a mutably borrowed place
    WriteToMutBorrowed {
        loan: LoanId,
        place: crate::middle::mir::Place,
    },
    /// NLL: Write to a shared borrowed place
    WriteToSharedBorrowed {
        loan: LoanId,
        place: crate::middle::mir::Place,
    },
    /// NLL: Move out of a borrowed place
    MoveOutOfBorrowed {
        loan: LoanId,
        place: crate::middle::mir::Place,
    },
}

impl std::fmt::Display for BorrowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BorrowError::CannotMoveOutOfBorrow => {
                write!(f, "cannot move out of borrowed content")
            }
            BorrowError::CannotBorrowMutably => {
                write!(f, "cannot borrow mutably while borrowed immutably")
            }
            BorrowError::CannotBorrowImmutably => {
                write!(f, "cannot borrow immutably while borrowed mutably")
            }
            BorrowError::UseOfMovedValue(local) => {
                write!(f, "use of moved value: {:?}", local)
            }
            BorrowError::LifetimeMismatch => {
                write!(f, "lifetime mismatch")
            }
            BorrowError::ConflictingBorrows { loan1, loan2, .. } => {
                write!(f, "conflicting borrows: {:?} and {:?}", loan1, loan2)
            }
            BorrowError::WriteToMutBorrowed { loan, .. } => {
                write!(
                    f,
                    "cannot write to mutably borrowed place (loan {:?})",
                    loan
                )
            }
            BorrowError::WriteToSharedBorrowed { loan, .. } => {
                write!(f, "cannot write to shared borrowed place (loan {:?})", loan)
            }
            BorrowError::MoveOutOfBorrowed { loan, .. } => {
                write!(f, "cannot move out of borrowed place (loan {:?})", loan)
            }
        }
    }
}

impl std::error::Error for BorrowError {}

/// Check a MIR body for borrow violations
pub fn check_borrows(body: &MirBody) -> Result<(), Vec<BorrowError>> {
    let checker = BorrowChecker::new(body);
    checker.check()
}

/// Check orphan rules for trait implementations
///
/// This is a convenience wrapper around the orphan_rules module.
/// Returns Ok(()) if the implementation is valid,
/// Returns Err with a description if it violates orphan rules.
///
/// # Examples
///
/// ```
/// use boxlang_compiler::middle::borrowck::check_orphan_rules;
/// use boxlang_compiler::ast::{Path, Type};
///
/// // Check if implementing a foreign trait for a foreign type is allowed
/// // (it shouldn't be)
/// ```
pub fn check_orphan_rules(
    trait_path: &crate::ast::Path,
    impl_type: &crate::ast::Type,
    local_crate: &str,
) -> Result<(), String> {
    orphan_rules::check_impl(trait_path, impl_type, local_crate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middle::mir::*;

    #[test]
    fn test_borrow_checker_creation() {
        let mut body = MirBody::new(0, 0..100);
        // Add at least one basic block
        let block = BasicBlockData::new();
        body.basic_blocks.push(block);

        let checker = BorrowChecker::new(&body);
        assert!(checker.check().is_ok());
    }

    #[test]
    fn test_simple_valid_borrows() {
        let mut body = MirBody::new(0, 0..100);

        let mut block = BasicBlockData::new();

        // _1 = &_0 (shared borrow)
        block.statements.push(Statement::Assign(
            Place::from_local(Local(1)),
            Rvalue::Ref(Place::from_local(Local::RETURN_PLACE), Mutability::Not),
        ));

        // _2 = *_1 (dereference the borrow)
        block.statements.push(Statement::Assign(
            Place::from_local(Local(2)),
            Rvalue::Use(Operand::Copy(Place {
                local: Local(1),
                projection: vec![PlaceElem::Deref],
            })),
        ));

        body.basic_blocks.push(block);

        assert!(check_borrows(&body).is_ok());
    }

    #[test]
    fn test_double_mutable_borrow_fails() {
        let mut body = MirBody::new(0, 0..100);

        let mut block = BasicBlockData::new();

        // _1 = &mut _0 (first mutable borrow)
        block.statements.push(Statement::Assign(
            Place::from_local(Local(1)),
            Rvalue::Ref(Place::from_local(Local::RETURN_PLACE), Mutability::Mut),
        ));

        // _2 = &mut _0 (second mutable borrow - should fail)
        block.statements.push(Statement::Assign(
            Place::from_local(Local(2)),
            Rvalue::Ref(Place::from_local(Local::RETURN_PLACE), Mutability::Mut),
        ));

        body.basic_blocks.push(block);

        assert!(check_borrows(&body).is_err());
    }

    #[test]
    fn test_mixed_borrows_fails() {
        let mut body = MirBody::new(0, 0..100);

        let mut block = BasicBlockData::new();

        // _1 = &mut _0 (mutable borrow)
        block.statements.push(Statement::Assign(
            Place::from_local(Local(1)),
            Rvalue::Ref(Place::from_local(Local::RETURN_PLACE), Mutability::Mut),
        ));

        // _2 = &_0 (shared borrow while mutable active - should fail)
        block.statements.push(Statement::Assign(
            Place::from_local(Local(2)),
            Rvalue::Ref(Place::from_local(Local::RETURN_PLACE), Mutability::Not),
        ));

        body.basic_blocks.push(block);

        assert!(check_borrows(&body).is_err());
    }

    #[test]
    fn test_move_out_of_borrow_fails() {
        let mut body = MirBody::new(0, 0..100);

        let mut block = BasicBlockData::new();

        // _1 = &_0 (create a borrow)
        block.statements.push(Statement::Assign(
            Place::from_local(Local(1)),
            Rvalue::Ref(Place::from_local(Local::RETURN_PLACE), Mutability::Not),
        ));

        // _2 = move _0 (move while borrowed - should fail)
        block.statements.push(Statement::Assign(
            Place::from_local(Local(2)),
            Rvalue::Move(Place::from_local(Local::RETURN_PLACE)),
        ));

        body.basic_blocks.push(block);

        assert!(check_borrows(&body).is_err());
    }
}

/// Orphan rules check for trait implementations
///
/// In Rust, orphan rules prevent implementing foreign traits for foreign types.
/// At least one of the trait or the type must be local to the crate.
pub mod orphan_rules {
    use crate::ast::{Path, Type};

    /// Check if a trait implementation violates orphan rules
    ///
    /// Returns Ok(()) if the implementation is valid,
    /// Returns Err with a description if it violates orphan rules
    pub fn check_impl(
        trait_path: &Path,
        impl_type: &Type,
        local_crate: &str,
    ) -> Result<(), String> {
        let trait_is_local = is_local_path(trait_path, local_crate);
        let type_is_local = is_local_type(impl_type, local_crate);

        if !trait_is_local && !type_is_local {
            Err(format!(
                "Orphan rule violation: cannot implement foreign trait `{}` for foreign type `{}`",
                path_to_string(trait_path),
                type_to_string(impl_type)
            ))
        } else {
            Ok(())
        }
    }

    /// Check if a path refers to a local crate item
    fn is_local_path(path: &Path, local_crate: &str) -> bool {
        // Empty path is considered local
        if path.segments.is_empty() {
            return true;
        }

        // Check if the first segment matches the local crate name
        // or if it's a relative path (no crate prefix)
        let first_segment = &path.segments[0].ident;
        first_segment.as_str() == local_crate || first_segment.as_str() == "crate"
    }

    /// Check if a type is local to the crate
    fn is_local_type(ty: &Type, local_crate: &str) -> bool {
        match ty {
            Type::Path(path) => is_local_path(path, local_crate),
            Type::Ref(inner, _) => is_local_type(inner, local_crate),
            Type::Ptr(inner, _) => is_local_type(inner, local_crate),
            Type::Array(inner, _) => is_local_type(inner, local_crate),
            Type::Slice(inner) => is_local_type(inner, local_crate),
            Type::Tuple(types) => types.iter().any(|t| is_local_type(t, local_crate)),
            Type::Function(func_ty) => {
                func_ty.params.iter().any(|t| is_local_type(t, local_crate))
                    || is_local_type(&func_ty.return_type, local_crate)
            }
            Type::Generic(inner, _) => is_local_type(inner, local_crate),
            // Primitive types and other built-ins are not local
            _ => false,
        }
    }

    /// Convert a path to a string representation
    fn path_to_string(path: &Path) -> String {
        path.segments
            .iter()
            .map(|s| s.ident.as_str())
            .collect::<Vec<_>>()
            .join("::")
    }

    /// Convert a type to a string representation
    fn type_to_string(ty: &Type) -> String {
        format!("{:?}", ty)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::ast::{Ident, PathSegment};

        fn make_path(segments: &[&str]) -> Path {
            Path {
                segments: segments
                    .iter()
                    .map(|s| PathSegment {
                        ident: Ident::new(s),
                        generics: vec![],
                    })
                    .collect(),
            }
        }

        #[test]
        fn test_local_trait_local_type() {
            let trait_path = make_path(&["crate", "MyTrait"]);
            let impl_type = Type::Path(make_path(&["crate", "MyType"]));

            assert!(check_impl(&trait_path, &impl_type, "my_crate").is_ok());
        }

        #[test]
        fn test_local_trait_foreign_type() {
            let trait_path = make_path(&["crate", "MyTrait"]);
            let impl_type = Type::Path(make_path(&["std", "String"]));

            assert!(check_impl(&trait_path, &impl_type, "my_crate").is_ok());
        }

        #[test]
        fn test_foreign_trait_local_type() {
            let trait_path = make_path(&["std", "Display"]);
            let impl_type = Type::Path(make_path(&["crate", "MyType"]));

            assert!(check_impl(&trait_path, &impl_type, "my_crate").is_ok());
        }

        #[test]
        fn test_foreign_trait_foreign_type_violation() {
            let trait_path = make_path(&["std", "Display"]);
            let impl_type = Type::Path(make_path(&["std", "String"]));

            assert!(check_impl(&trait_path, &impl_type, "my_crate").is_err());
        }
    }
}
