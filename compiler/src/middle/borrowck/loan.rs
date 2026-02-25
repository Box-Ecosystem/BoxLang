//! Loan and Borrow State Management
//!
//! This module tracks the state of borrows and loans in the program.
//! A "loan" represents a borrow that has been created, and we track
//! its validity and conflicts with other operations.

use crate::middle::mir::{Local, Place};
use std::collections::HashMap;

/// A unique identifier for a loan
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LoanId(pub u32);

impl LoanId {
    pub fn new(id: u32) -> Self {
        LoanId(id)
    }
}

/// The kind of borrow
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BorrowKind {
    /// Shared borrow (&T) - allows multiple readers
    Shared,
    /// Mutable borrow (&mut T) - allows single writer
    Mut,
    /// Unique immutable borrow (for match guards)
    Unique,
}

impl BorrowKind {
    /// Check if this borrow conflicts with another borrow
    pub fn conflicts_with(self, other: BorrowKind) -> bool {
        match (self, other) {
            // Shared borrows don't conflict with each other
            (BorrowKind::Shared, BorrowKind::Shared) => false,
            // Everything else conflicts
            _ => true,
        }
    }

    /// Is this a mutable borrow?
    pub fn is_mut(&self) -> bool {
        matches!(self, BorrowKind::Mut)
    }
}

/// Information about a single borrow (loan)
#[derive(Debug, Clone)]
pub struct Loan {
    /// Unique identifier for this loan
    pub id: LoanId,
    /// The place that was borrowed
    pub borrowed_place: Place,
    /// The kind of borrow
    pub kind: BorrowKind,
    /// The local variable holding the borrow
    pub borrow_local: Local,
}

impl Loan {
    pub fn new(id: LoanId, borrowed_place: Place, kind: BorrowKind, borrow_local: Local) -> Self {
        Self {
            id,
            borrowed_place,
            kind,
            borrow_local,
        }
    }
}

/// The state of a local variable regarding moves
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveState {
    /// The value is present (not moved)
    Available,
    /// The value has been moved
    Moved,
    /// The value is partially moved (e.g., a field of a struct)
    PartiallyMoved,
}

/// Tracks the borrow and move state for a function body
#[derive(Debug, Clone)]
pub struct BorrowState {
    /// Active loans (borrows that are currently valid)
    active_loans: HashMap<LoanId, Loan>,
    /// Loans indexed by the local that holds the borrow
    loans_by_local: HashMap<Local, LoanId>,
    /// Loans indexed by the borrowed place's local
    loans_by_borrowed_local: HashMap<Local, Vec<LoanId>>,
    /// Move state for each local
    move_states: HashMap<Local, MoveState>,
    /// Next loan ID to assign
    next_loan_id: u32,
}

impl BorrowState {
    pub fn new() -> Self {
        Self {
            active_loans: HashMap::new(),
            loans_by_local: HashMap::new(),
            loans_by_borrowed_local: HashMap::new(),
            move_states: HashMap::new(),
            next_loan_id: 0,
        }
    }

    /// Create a new loan
    pub fn create_loan(
        &mut self,
        borrowed_place: Place,
        kind: BorrowKind,
        borrow_local: Local,
    ) -> LoanId {
        let id = LoanId::new(self.next_loan_id);
        self.next_loan_id += 1;

        let loan = Loan::new(id, borrowed_place.clone(), kind, borrow_local);

        self.active_loans.insert(id, loan);
        self.loans_by_local.insert(borrow_local, id);

        self.loans_by_borrowed_local
            .entry(borrowed_place.local)
            .or_default()
            .push(id);

        id
    }

    /// Invalidate loans that conflict with accessing a place
    pub fn invalidate_conflicting_loans(&mut self, place: &Place, is_write: bool) -> Vec<LoanId> {
        let mut invalidated = Vec::new();
        let mut to_remove = Vec::new();

        for (id, loan) in &self.active_loans {
            // Check if this loan conflicts with the access
            if self.places_conflict(&loan.borrowed_place, place) {
                // Shared borrows are only invalidated by writes
                if !is_write && loan.kind == BorrowKind::Shared {
                    continue;
                }
                invalidated.push(*id);
                to_remove.push(*id);
            }
        }

        // Remove invalidated loans
        for id in to_remove {
            self.remove_loan(id);
        }

        invalidated
    }

    /// Check if two places conflict (one is a prefix of the other or they overlap)
    fn places_conflict(&self, place1: &Place, place2: &Place) -> bool {
        // Simple case: same local
        if place1.local == place2.local {
            // Check if projections overlap
            return self.projections_conflict(&place1.projection, &place2.projection);
        }
        false
    }

    /// Check if two projection sequences conflict
    fn projections_conflict(
        &self,
        proj1: &[crate::middle::mir::PlaceElem],
        proj2: &[crate::middle::mir::PlaceElem],
    ) -> bool {
        // For simplicity, assume they conflict if one is a prefix of the other
        // or if they are equal
        let min_len = proj1.len().min(proj2.len());
        proj1[..min_len] == proj2[..min_len]
    }

    /// Remove a loan
    fn remove_loan(&mut self, id: LoanId) {
        if let Some(loan) = self.active_loans.remove(&id) {
            self.loans_by_local.remove(&loan.borrow_local);

            if let Some(loans) = self
                .loans_by_borrowed_local
                .get_mut(&loan.borrowed_place.local)
            {
                loans.retain(|&lid| lid != id);
            }
        }
    }

    /// Get active loans for a local
    pub fn get_loans_for_local(&self, local: Local) -> Vec<&Loan> {
        self.loans_by_borrowed_local
            .get(&local)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.active_loans.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get active loans held by a borrow local
    pub fn get_loan_by_borrow_local(&self, local: Local) -> Option<&Loan> {
        self.loans_by_local
            .get(&local)
            .and_then(|id| self.active_loans.get(id))
    }

    /// Check if there are any active mutable borrows
    pub fn has_active_mut_borrow(&self, local: Local) -> bool {
        self.get_loans_for_local(local)
            .iter()
            .any(|loan| loan.kind.is_mut())
    }

    /// Check if there are any active shared borrows
    pub fn has_active_shared_borrow(&self, local: Local) -> bool {
        self.get_loans_for_local(local)
            .iter()
            .any(|loan| matches!(loan.kind, BorrowKind::Shared))
    }

    /// Mark a local as moved
    pub fn mark_moved(&mut self, local: Local) {
        self.move_states.insert(local, MoveState::Moved);
    }

    /// Get the move state of a local
    pub fn get_move_state(&self, local: Local) -> MoveState {
        self.move_states
            .get(&local)
            .copied()
            .unwrap_or(MoveState::Available)
    }

    /// Check if a local has been moved
    pub fn is_moved(&self, local: Local) -> bool {
        matches!(
            self.get_move_state(local),
            MoveState::Moved | MoveState::PartiallyMoved
        )
    }

    /// Get all active loans
    pub fn active_loans(&self) -> &HashMap<LoanId, Loan> {
        &self.active_loans
    }
}

impl Default for BorrowState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_loan() {
        let mut state = BorrowState::new();
        let place = Place::from_local(Local(1));

        let loan_id = state.create_loan(place, BorrowKind::Shared, Local(2));

        assert_eq!(state.active_loans.len(), 1);
        assert!(state.active_loans.contains_key(&loan_id));
    }

    #[test]
    fn test_borrow_kind_conflicts() {
        assert!(!BorrowKind::Shared.conflicts_with(BorrowKind::Shared));
        assert!(BorrowKind::Shared.conflicts_with(BorrowKind::Mut));
        assert!(BorrowKind::Mut.conflicts_with(BorrowKind::Shared));
        assert!(BorrowKind::Mut.conflicts_with(BorrowKind::Mut));
    }

    #[test]
    fn test_move_state() {
        let mut state = BorrowState::new();

        assert!(!state.is_moved(Local(1)));

        state.mark_moved(Local(1));

        assert!(state.is_moved(Local(1)));
        assert_eq!(state.get_move_state(Local(1)), MoveState::Moved);
    }
}
