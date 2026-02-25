//! Borrow Checking Algorithm
//!
//! This module implements the core borrow checking logic.
//! It walks through the MIR and validates all borrows and moves.

use crate::middle::borrowck::{BorrowError, BorrowKind, BorrowState};
use crate::middle::mir::*;
use std::collections::HashSet;

/// The borrow checker context
pub struct BorrowChecker<'a> {
    /// The MIR body being checked
    body: &'a MirBody,
    /// Current borrow state
    state: BorrowState,
    /// Errors found during checking
    errors: Vec<BorrowError>,
    /// Visited blocks to avoid infinite loops
    visited_blocks: HashSet<BasicBlock>,
}

impl<'a> BorrowChecker<'a> {
    /// Create a new borrow checker
    pub fn new(body: &'a MirBody) -> Self {
        Self {
            body,
            state: BorrowState::new(),
            errors: Vec::new(),
            visited_blocks: HashSet::new(),
        }
    }

    /// Run the borrow check
    pub fn check(mut self) -> Result<(), Vec<BorrowError>> {
        // Start from the entry block
        self.check_block(BasicBlock(0));

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors)
        }
    }

    /// Check a basic block
    fn check_block(&mut self, block: BasicBlock) {
        // Avoid infinite loops
        if !self.visited_blocks.insert(block) {
            return;
        }

        let block_data = self.body.basic_block(block);

        // Check each statement
        for stmt in &block_data.statements {
            self.check_statement(stmt);
        }

        // Check the terminator
        if let Some(ref terminator) = block_data.terminator {
            self.check_terminator(terminator);
        }
    }

    /// Check a statement
    fn check_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Assign(place, rvalue) => {
                self.check_assign(place, rvalue);
            }
            Statement::StorageLive(_local) => {
                // Mark the local as available
                // (In a full implementation, we'd track this more carefully)
            }
            Statement::StorageDead(local) => {
                // Invalidate any borrows of this local
                self.state.invalidate_conflicting_loans(
                    &Place::from_local(*local),
                    true, // Treat as a write
                );
            }
            Statement::InlineAsm(_) => {
                // Inline asm is unsafe - assume it can do anything
                // In a full implementation, we'd check the constraints
            }
            Statement::Nop => {}
        }
    }

    /// Check an assignment
    fn check_assign(&mut self, place: &Place, rvalue: &Rvalue) {
        // First, check the rvalue for any violations
        self.check_rvalue(rvalue);

        // Then, check if we can write to the place
        self.check_write_access(place);

        // Handle special rvalues that create borrows
        match rvalue {
            Rvalue::Ref(borrowed_place, mutability) => {
                self.check_borrow(borrowed_place, *mutability, place.local);
            }
            Rvalue::Move(moved_place) => {
                self.check_move(moved_place);
            }
            _ => {}
        }
    }

    /// Check an rvalue
    fn check_rvalue(&mut self, rvalue: &Rvalue) {
        match rvalue {
            Rvalue::Use(operand) => {
                self.check_operand(operand);
            }
            Rvalue::Cast(_, operand, _) => {
                self.check_operand(operand);
            }
            Rvalue::BinaryOp(_, left, right) => {
                self.check_operand(left);
                self.check_operand(right);
            }
            Rvalue::UnaryOp(_, operand) => {
                self.check_operand(operand);
            }
            Rvalue::Ref(place, _) => {
                // Just check that the place is valid (not moved)
                self.check_read_access(place);
            }
            Rvalue::AddressOf(place, _) => {
                // Raw pointers are more permissive
                // But we still check the base is valid
                self.check_read_access(place);
            }
            Rvalue::Copy(place) => {
                self.check_read_access(place);
            }
            Rvalue::Move(place) => {
                // Move is handled in check_assign
                self.check_read_access(place);
            }
            Rvalue::Len(place) => {
                self.check_read_access(place);
            }
            Rvalue::Discriminant(place) => {
                self.check_read_access(place);
            }
            Rvalue::Aggregate(_, operands) => {
                for operand in operands {
                    self.check_operand(operand);
                }
            }
        }
    }

    /// Check an operand
    fn check_operand(&mut self, operand: &Operand) {
        match operand {
            Operand::Copy(place) => {
                self.check_read_access(place);
            }
            Operand::Move(place) => {
                // This shouldn't happen in normal MIR (moves are in Rvalue)
                // But check anyway
                self.check_read_access(place);
            }
            Operand::Constant(_) => {
                // Constants are always fine
            }
        }
    }

    /// Check a terminator
    fn check_terminator(&mut self, terminator: &Terminator) {
        match &terminator.kind {
            TerminatorKind::Goto { target } => {
                self.check_block(*target);
            }
            TerminatorKind::SwitchInt {
                discr,
                targets,
                otherwise,
                ..
            } => {
                self.check_operand(discr);
                for (_, target) in targets {
                    self.check_block(*target);
                }
                self.check_block(*otherwise);
            }
            TerminatorKind::Return => {
                // Check that the return value is valid
                let return_place = Place::from_local(Local::RETURN_PLACE);
                self.check_read_access(&return_place);
            }
            TerminatorKind::Unwind => {
                // Cleanup - nothing to check
            }
            TerminatorKind::Call {
                func, args, target, ..
            } => {
                self.check_operand(func);
                for arg in args {
                    self.check_operand(arg);
                }

                // Function calls can invalidate borrows
                // (In a full implementation, we'd check the function signature)

                if let Some(target_block) = target {
                    self.check_block(*target_block);
                }
            }
            TerminatorKind::Assert { cond, target, .. } => {
                self.check_operand(cond);
                self.check_block(*target);
            }
        }
    }

    /// Check a borrow operation
    fn check_borrow(
        &mut self,
        borrowed_place: &Place,
        mutability: Mutability,
        borrow_local: Local,
    ) {
        let borrow_kind = match mutability {
            Mutability::Mut => BorrowKind::Mut,
            Mutability::Not => BorrowKind::Shared,
        };

        let loans = self.state.get_loans_for_local(borrowed_place.local);

        for loan in loans {
            match (borrow_kind, loan.kind) {
                (BorrowKind::Shared, BorrowKind::Shared) => continue,
                (BorrowKind::Mut, _) => {
                    self.errors.push(BorrowError::CannotBorrowMutably);
                    return;
                }
                (_, BorrowKind::Mut) => {
                    if borrow_kind == BorrowKind::Shared {
                        self.errors.push(BorrowError::CannotBorrowImmutably);
                    } else {
                        self.errors.push(BorrowError::CannotBorrowMutably);
                    }
                    return;
                }
                _ => {}
            }
        }

        if self.state.is_moved(borrowed_place.local) {
            self.errors
                .push(BorrowError::UseOfMovedValue(borrowed_place.local));
            return;
        }

        self.state
            .create_loan(borrowed_place.clone(), borrow_kind, borrow_local);
    }

    /// Check a move operation
    fn check_move(&mut self, place: &Place) {
        // Check if there are active borrows
        if self.state.has_active_shared_borrow(place.local)
            || self.state.has_active_mut_borrow(place.local)
        {
            self.errors.push(BorrowError::CannotMoveOutOfBorrow);
            return;
        }

        // Check if already moved
        if self.state.is_moved(place.local) {
            self.errors.push(BorrowError::UseOfMovedValue(place.local));
            return;
        }

        // Mark as moved
        self.state.mark_moved(place.local);
    }

    /// Check read access to a place
    fn check_read_access(&mut self, place: &Place) {
        // Check if the place has been moved
        if self.state.is_moved(place.local) {
            self.errors.push(BorrowError::UseOfMovedValue(place.local));
        }

        // Check for active mutable borrows (can't read while mutably borrowed)
        if self.state.has_active_mut_borrow(place.local) {
            // This is actually allowed if we're reading through the borrow itself
            // For simplicity, we allow it here
        }
    }

    /// Check write access to a place
    fn check_write_access(&mut self, place: &Place) {
        // Invalidate any conflicting loans
        let invalidated = self.state.invalidate_conflicting_loans(place, true);

        // If there were active loans, that's an error
        if !invalidated.is_empty() {
            // We already invalidated them, but we should report an error
            // if this was an active borrow
            self.errors.push(BorrowError::CannotBorrowMutably);
        }

        // Check if the place has been moved
        if self.state.is_moved(place.local) {
            self.errors.push(BorrowError::UseOfMovedValue(place.local));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn create_simple_body() -> MirBody {
        MirBody::new(0, 0..100)
    }

    #[test]
    fn test_empty_body() {
        let mut body = create_simple_body();
        let block = BasicBlockData::new();
        body.basic_blocks.push(block);
        let checker = BorrowChecker::new(&body);
        assert!(checker.check().is_ok());
    }

    #[test]
    fn test_shared_borrow_allowed() {
        let mut body = create_simple_body();

        let mut block = BasicBlockData::new();

        block.statements.push(Statement::Assign(
            Place::from_local(Local(1)),
            Rvalue::Ref(Place::from_local(Local::RETURN_PLACE), Mutability::Not),
        ));

        block.statements.push(Statement::Assign(
            Place::from_local(Local(2)),
            Rvalue::Ref(Place::from_local(Local::RETURN_PLACE), Mutability::Not),
        ));

        body.basic_blocks.push(block);

        let checker = BorrowChecker::new(&body);
        assert!(checker.check().is_ok());
    }

    #[test]
    fn test_mut_borrow_conflict() {
        let mut body = create_simple_body();

        let mut block = BasicBlockData::new();

        block.statements.push(Statement::Assign(
            Place::from_local(Local(1)),
            Rvalue::Ref(Place::from_local(Local::RETURN_PLACE), Mutability::Mut),
        ));

        block.statements.push(Statement::Assign(
            Place::from_local(Local(2)),
            Rvalue::Ref(Place::from_local(Local::RETURN_PLACE), Mutability::Not),
        ));

        body.basic_blocks.push(block);

        let checker = BorrowChecker::new(&body);
        let result = checker.check();
        assert!(result.is_err(), "Expected error when creating shared borrow while mutable borrow is active");
    }

    #[test]
    fn test_use_after_move() {
        let mut body = create_simple_body();

        let mut block = BasicBlockData::new();

        block.statements.push(Statement::Assign(
            Place::from_local(Local(1)),
            Rvalue::Use(Operand::Constant(Constant::Scalar(Scalar::Int(
                42,
                IntType::I64,
            )))),
        ));

        block.statements.push(Statement::Assign(
            Place::from_local(Local(2)),
            Rvalue::Move(Place::from_local(Local(1))),
        ));

        block.statements.push(Statement::Assign(
            Place::from_local(Local(3)),
            Rvalue::Use(Operand::Copy(Place::from_local(Local(1)))),
        ));

        body.basic_blocks.push(block);

        let checker = BorrowChecker::new(&body);
        let result = checker.check();
        assert!(result.is_err(), "Expected error when using value after move");
    }
}
