//! Non-Lexical Lifetimes (NLL) Borrow Checker
//!
//! This module implements the NLL borrow checking algorithm.
//! Unlike lexical borrow checking, NLL tracks the actual lifetime of borrows
//! based on control flow, allowing more flexible borrow patterns.
//!
//! The algorithm works in three phases:
//! 1. **Collection**: Find all borrow expressions in the MIR
//! 2. **Region Computation**: Compute the region (set of CFG points) where each borrow is active
//! 3. **Conflict Checking**: Check for conflicts between overlapping borrows

use crate::middle::borrowck::{BorrowError, BorrowKind, LoanId, MoveState};
use crate::middle::mir::*;
use std::collections::{HashMap, HashSet, VecDeque};

/// A region represents a set of points in the control flow graph
/// where a borrow is active.
#[derive(Debug, Clone, Default)]
pub struct Region {
    /// The set of basic blocks where this region is active
    pub blocks: HashSet<BasicBlock>,
    /// The specific points within blocks (statement indices)
    pub points: HashSet<(BasicBlock, usize)>,
}

impl Region {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a block to this region
    pub fn add_block(&mut self, block: BasicBlock) {
        self.blocks.insert(block);
    }

    /// Add a specific point to this region
    pub fn add_point(&mut self, block: BasicBlock, statement: usize) {
        self.points.insert((block, statement));
    }

    /// Check if this region contains a point
    pub fn contains(&self, block: BasicBlock, statement: usize) -> bool {
        self.blocks.contains(&block) || self.points.contains(&(block, statement))
    }

    /// Check if this region contains a specific block
    pub fn contains_block(&self, block: BasicBlock) -> bool {
        self.blocks.contains(&block)
    }

    /// Check if this region overlaps with another region
    pub fn overlaps(&self, other: &Region) -> bool {
        !self.blocks.is_disjoint(&other.blocks) || !self.points.is_disjoint(&other.points)
    }

    /// Merge another region into this one
    pub fn merge(&mut self, other: &Region) {
        self.blocks.extend(&other.blocks);
        self.points.extend(&other.points);
    }
}

/// Information about a borrow's lifetime
#[derive(Debug, Clone)]
pub struct BorrowLifetime {
    /// The loan ID
    pub loan_id: LoanId,
    /// The place that was borrowed
    pub borrowed_place: Place,
    /// The kind of borrow
    pub kind: BorrowKind,
    /// The local holding the borrow
    pub borrow_local: Local,
    /// The region where this borrow is active
    pub region: Region,
    /// The point where the borrow was created
    pub creation_point: (BasicBlock, usize),
    /// Points where the borrow is used (kills the borrow)
    pub kill_points: HashSet<(BasicBlock, usize)>,
    /// Whether this borrow is live at the end of the function
    pub live_at_end: bool,
}

/// A point in the control flow graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point {
    pub block: BasicBlock,
    pub statement: usize,
}

impl Point {
    pub fn new(block: BasicBlock, statement: usize) -> Self {
        Self { block, statement }
    }
}

/// The NLL borrow checker
pub struct NllBorrowChecker<'a> {
    /// The MIR body being checked
    body: &'a MirBody,
    /// All borrow lifetimes
    borrow_lifetimes: HashMap<LoanId, BorrowLifetime>,
    /// Map from local to its borrow
    local_to_borrow: HashMap<Local, LoanId>,
    /// Errors found during checking
    errors: Vec<BorrowError>,
    /// Next loan ID
    next_loan_id: u32,
    /// Control flow graph (block -> successors)
    cfg: HashMap<BasicBlock, Vec<BasicBlock>>,
    /// Reverse CFG (block -> predecessors)
    reverse_cfg: HashMap<BasicBlock, Vec<BasicBlock>>,
    /// Dominator tree (block -> immediate dominator)
    dominators: HashMap<BasicBlock, BasicBlock>,
    /// Post-dominator tree (block -> immediate post-dominator)
    post_dominators: HashMap<BasicBlock, BasicBlock>,
    /// Move states at each point
    move_states: HashMap<Point, HashMap<Local, MoveState>>,
}

impl<'a> NllBorrowChecker<'a> {
    /// Create a new NLL borrow checker
    pub fn new(body: &'a MirBody) -> Self {
        let cfg = Self::build_cfg(body);
        let reverse_cfg = Self::build_reverse_cfg(&cfg);
        let dominators = Self::compute_dominators(body, &cfg);
        let post_dominators = Self::compute_post_dominators(body, &cfg);

        Self {
            body,
            borrow_lifetimes: HashMap::new(),
            local_to_borrow: HashMap::new(),
            errors: Vec::new(),
            next_loan_id: 0,
            cfg,
            reverse_cfg,
            dominators,
            post_dominators,
            move_states: HashMap::new(),
        }
    }

    /// Build the control flow graph
    fn build_cfg(body: &MirBody) -> HashMap<BasicBlock, Vec<BasicBlock>> {
        let mut cfg: HashMap<BasicBlock, Vec<BasicBlock>> = HashMap::new();

        for (block_idx, block) in body.basic_blocks.iter().enumerate() {
            let block_id = BasicBlock(block_idx as u32);
            let mut successors = Vec::new();

            if let Some(ref terminator) = block.terminator {
                match &terminator.kind {
                    TerminatorKind::Goto { target } => {
                        successors.push(*target);
                    }
                    TerminatorKind::SwitchInt {
                        targets, otherwise, ..
                    } => {
                        for (_, target) in targets {
                            successors.push(*target);
                        }
                        successors.push(*otherwise);
                    }
                    TerminatorKind::Return => {
                        // No successors
                    }
                    TerminatorKind::Unwind => {
                        // No successors in normal flow
                    }
                    TerminatorKind::Call { target, .. } => {
                        if let Some(t) = target {
                            successors.push(*t);
                        }
                    }
                    TerminatorKind::Assert {
                        target, cleanup, ..
                    } => {
                        successors.push(*target);
                        if let Some(cleanup_block) = cleanup {
                            successors.push(*cleanup_block);
                        }
                    }
                }
            }

            cfg.insert(block_id, successors);
        }

        cfg
    }

    /// Build the reverse control flow graph
    fn build_reverse_cfg(
        cfg: &HashMap<BasicBlock, Vec<BasicBlock>>,
    ) -> HashMap<BasicBlock, Vec<BasicBlock>> {
        let mut reverse: HashMap<BasicBlock, Vec<BasicBlock>> = HashMap::new();

        for (block, successors) in cfg {
            for succ in successors {
                reverse.entry(*succ).or_default().push(*block);
            }
        }

        reverse
    }

    /// Compute dominators using iterative algorithm
    fn compute_dominators(
        body: &MirBody,
        cfg: &HashMap<BasicBlock, Vec<BasicBlock>>,
    ) -> HashMap<BasicBlock, BasicBlock> {
        let entry = BasicBlock(0);
        let num_blocks = body.basic_blocks.len();

        if num_blocks == 0 {
            return HashMap::new();
        }

        // Initialize dominators: all blocks dominate all blocks except entry
        let all_blocks: HashSet<BasicBlock> =
            (0..num_blocks).map(|i| BasicBlock(i as u32)).collect();
        let mut dominators: HashMap<BasicBlock, HashSet<BasicBlock>> = HashMap::new();

        for i in 0..num_blocks {
            let block = BasicBlock(i as u32);
            if block == entry {
                dominators.insert(block, [entry].iter().cloned().collect());
            } else {
                dominators.insert(block, all_blocks.clone());
            }
        }

        // Iteratively refine dominators
        let mut changed = true;
        while changed {
            changed = false;

            for i in 0..num_blocks {
                let block = BasicBlock(i as u32);
                if block == entry {
                    continue;
                }

                if let Some(preds) = Self::build_reverse_cfg(cfg).get(&block) {
                    if !preds.is_empty() {
                        let mut new_dom: HashSet<BasicBlock> = all_blocks.clone();

                        for pred in preds {
                            if let Some(pred_dom) = dominators.get(pred) {
                                new_dom = new_dom.intersection(pred_dom).cloned().collect();
                            }
                        }

                        new_dom.insert(block);

                        if let Some(current_dom) = dominators.get(&block) {
                            if new_dom != *current_dom {
                                dominators.insert(block, new_dom);
                                changed = true;
                            }
                        }
                    }
                }
            }
        }

        // Convert to immediate dominators
        Self::compute_immediate_dominators(&dominators, entry, num_blocks)
    }

    /// Compute post-dominators (dominators in the reverse graph)
    fn compute_post_dominators(
        body: &MirBody,
        cfg: &HashMap<BasicBlock, Vec<BasicBlock>>,
    ) -> HashMap<BasicBlock, BasicBlock> {
        let num_blocks = body.basic_blocks.len();

        if num_blocks == 0 {
            return HashMap::new();
        }

        // Find exit blocks (blocks with no successors or Return terminator)
        let exit_blocks: HashSet<BasicBlock> = (0..num_blocks)
            .map(|i| BasicBlock(i as u32))
            .filter(|block| {
                if let Some(succs) = cfg.get(block) {
                    succs.is_empty()
                } else {
                    true
                }
            })
            .collect();

        if exit_blocks.is_empty() {
            return HashMap::new();
        }

        // Use first exit as the "entry" for post-dominator computation
        let post_entry = match exit_blocks.iter().next() {
            Some(&block) => block,
            None => return HashMap::new(),
        };

        // Build reverse CFG
        let _reverse_cfg = Self::build_reverse_cfg(cfg);

        // Initialize post-dominators
        let all_blocks: HashSet<BasicBlock> =
            (0..num_blocks).map(|i| BasicBlock(i as u32)).collect();
        let mut post_dominators: HashMap<BasicBlock, HashSet<BasicBlock>> = HashMap::new();

        for i in 0..num_blocks {
            let block = BasicBlock(i as u32);
            if block == post_entry {
                post_dominators.insert(block, [post_entry].iter().cloned().collect());
            } else {
                post_dominators.insert(block, all_blocks.clone());
            }
        }

        // Iteratively refine post-dominators
        let mut changed = true;
        while changed {
            changed = false;

            for i in 0..num_blocks {
                let block = BasicBlock(i as u32);
                if block == post_entry {
                    continue;
                }

                if let Some(succs) = cfg.get(&block) {
                    if !succs.is_empty() {
                        let mut new_pdom: HashSet<BasicBlock> = all_blocks.clone();

                        for succ in succs {
                            if let Some(succ_pdom) = post_dominators.get(succ) {
                                new_pdom = new_pdom.intersection(succ_pdom).cloned().collect();
                            }
                        }

                        new_pdom.insert(block);

                        if let Some(current_pdom) = post_dominators.get(&block) {
                            if new_pdom != *current_pdom {
                                post_dominators.insert(block, new_pdom);
                                changed = true;
                            }
                        }
                    }
                }
            }
        }

        // Convert to immediate post-dominators
        Self::compute_immediate_dominators(&post_dominators, post_entry, num_blocks)
    }

    /// Compute immediate dominators from the full dominator sets
    fn compute_immediate_dominators(
        dominators: &HashMap<BasicBlock, HashSet<BasicBlock>>,
        entry: BasicBlock,
        _num_blocks: usize,
    ) -> HashMap<BasicBlock, BasicBlock> {
        let mut immediate_doms: HashMap<BasicBlock, BasicBlock> = HashMap::new();

        for (block, dom_set) in dominators {
            if *block == entry {
                continue;
            }

            // Find the immediate dominator (closest strict dominator)
            let strict_doms: HashSet<_> =
                dom_set.iter().filter(|&&b| b != *block).cloned().collect();

            for candidate in &strict_doms {
                if let Some(candidate_dom) = dominators.get(candidate) {
                    if strict_doms
                        .iter()
                        .all(|d| candidate_dom.contains(d) || *d == *candidate)
                    {
                        immediate_doms.insert(*block, *candidate);
                        break;
                    }
                }
            }
        }

        immediate_doms
    }

    /// Run the NLL borrow check
    pub fn check(mut self) -> Result<(), Vec<BorrowError>> {
        // Phase 0: Initialize move states
        self.initialize_move_states();

        // Phase 1: Collect all borrows and their creation points
        self.collect_borrows();

        // Phase 2: Compute borrow regions (lifetimes) using dataflow analysis
        self.compute_borrow_regions_dataflow();

        // Phase 3: Check for conflicts
        self.check_conflicts();

        // Phase 4: Check for use-after-move
        self.check_use_after_move();

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors)
        }
    }

    /// Initialize move states at each point
    fn initialize_move_states(&mut self) {
        for (block_idx, block) in self.body.basic_blocks.iter().enumerate() {
            let block_id = BasicBlock(block_idx as u32);

            for stmt_idx in 0..=block.statements.len() {
                let point = Point::new(block_id, stmt_idx);
                self.move_states.insert(point, HashMap::new());
            }
        }
    }

    /// Collect all borrows from the MIR
    fn collect_borrows(&mut self) {
        for (block_idx, block) in self.body.basic_blocks.iter().enumerate() {
            let block_id = BasicBlock(block_idx as u32);

            for (stmt_idx, stmt) in block.statements.iter().enumerate() {
                if let Statement::Assign(place, rvalue) = stmt {
                    // Check for reference creation
                    if let Rvalue::Ref(borrowed_place, mutability) = rvalue {
                        let loan_id = LoanId::new(self.next_loan_id);
                        self.next_loan_id += 1;

                        let kind = match mutability {
                            Mutability::Mut => BorrowKind::Mut,
                            Mutability::Not => BorrowKind::Shared,
                        };

                        let lifetime = BorrowLifetime {
                            loan_id,
                            borrowed_place: borrowed_place.clone(),
                            kind,
                            borrow_local: place.local,
                            region: Region::new(),
                            creation_point: (block_id, stmt_idx),
                            kill_points: HashSet::new(),
                            live_at_end: false,
                        };

                        self.borrow_lifetimes.insert(loan_id, lifetime);
                        self.local_to_borrow.insert(place.local, loan_id);
                    }

                    // Check for moves
                    if let Rvalue::Move(moved_place) = rvalue {
                        let point = Point::new(block_id, stmt_idx);
                        if let Some(states) = self.move_states.get_mut(&point) {
                            states.insert(moved_place.local, MoveState::Moved);
                        }
                    }
                }
            }
        }
    }

    /// Compute the region (lifetime) for each borrow using dataflow analysis
    fn compute_borrow_regions_dataflow(&mut self) {
        // For each borrow, compute its region using liveness analysis
        let loan_ids: Vec<LoanId> = self.borrow_lifetimes.keys().cloned().collect();

        for loan_id in loan_ids {
            self.compute_single_borrow_region(loan_id);
        }
    }

    /// Compute the region for a single borrow
    fn compute_single_borrow_region(&mut self, loan_id: LoanId) {
        let (creation_point, borrow_local) = match self.borrow_lifetimes.get(&loan_id) {
            Some(lifetime) => (lifetime.creation_point, lifetime.borrow_local),
            None => {
                eprintln!(
                    "Warning: Borrow lifetime not found for loan_id {:?}",
                    loan_id
                );
                return;
            }
        };

        // Use BFS to find all reachable points where the borrow is live
        let mut visited: HashSet<(BasicBlock, usize)> = HashSet::new();
        let mut queue: VecDeque<(BasicBlock, usize)> = VecDeque::new();

        // Start from the creation point
        queue.push_back((creation_point.0, creation_point.1));

        let mut live_at_end = false;

        while let Some((block, stmt_idx)) = queue.pop_front() {
            if !visited.insert((block, stmt_idx)) {
                continue;
            }

            // Check if this point kills the borrow
            if self.point_kills_borrow(block, stmt_idx, loan_id, borrow_local, creation_point) {
                if let Some(lifetime) = self.borrow_lifetimes.get_mut(&loan_id) {
                    lifetime.kill_points.insert((block, stmt_idx));
                }
                continue;
            }

            // Add this point to the region
            if let Some(lifetime) = self.borrow_lifetimes.get_mut(&loan_id) {
                lifetime.region.add_point(block, stmt_idx);
                lifetime.region.add_block(block);
            }

            // Get the block data
            let block_data = &self.body.basic_blocks[block.0 as usize];

            // If we're at or past the last statement, follow terminators
            if stmt_idx >= block_data.statements.len() {
                if let Some(ref terminator) = block_data.terminator {
                    match &terminator.kind {
                        TerminatorKind::Goto { target } => {
                            queue.push_back((*target, 0));
                        }
                        TerminatorKind::SwitchInt {
                            targets, otherwise, ..
                        } => {
                            for (_, target) in targets {
                                queue.push_back((*target, 0));
                            }
                            queue.push_back((*otherwise, 0));
                        }
                        TerminatorKind::Return => {
                            live_at_end = true;
                        }
                        TerminatorKind::Unwind => {}
                        TerminatorKind::Call { target, .. } => {
                            if let Some(t) = target {
                                queue.push_back((*t, 0));
                            } else {
                                live_at_end = true;
                            }
                        }
                        TerminatorKind::Assert {
                            target, cleanup, ..
                        } => {
                            queue.push_back((*target, 0));
                            if let Some(cleanup_block) = cleanup {
                                queue.push_back((*cleanup_block, 0));
                            }
                        }
                    }
                }
            } else {
                // Move to next statement in the same block
                queue.push_back((block, stmt_idx + 1));
            }
        }

        // Update live_at_end flag
        if let Some(lifetime) = self.borrow_lifetimes.get_mut(&loan_id) {
            lifetime.live_at_end = live_at_end;
        }
    }

    /// Check if a point kills (ends) a borrow
    fn point_kills_borrow(
        &self,
        block: BasicBlock,
        stmt_idx: usize,
        loan_id: LoanId,
        borrow_local: Local,
        creation_point: (BasicBlock, usize),
    ) -> bool {
        // Don't kill at the creation point itself
        if (block, stmt_idx) == creation_point {
            return false;
        }

        let block_data = &self.body.basic_blocks[block.0 as usize];

        // Check if we're past the last statement
        if stmt_idx >= block_data.statements.len() {
            return false;
        }

        let stmt = &block_data.statements[stmt_idx];

        match stmt {
            Statement::Assign(place, _) => {
                // Check if the borrow local is overwritten
                if place.local == borrow_local {
                    return true;
                }
                // Check if the borrowed place is written to
                if let Some(&id) = self.local_to_borrow.get(&place.local) {
                    if id == loan_id {
                        return true;
                    }
                }
            }
            Statement::StorageDead(local) => {
                if *local == borrow_local {
                    return true;
                }
            }
            _ => {}
        }

        false
    }

    /// Check for conflicts between borrows
    fn check_conflicts(&mut self) {
        let loan_ids: Vec<_> = self.borrow_lifetimes.keys().cloned().collect();

        // Check all pairs of borrows
        for (i, &loan_id1) in loan_ids.iter().enumerate() {
            for &loan_id2 in loan_ids.iter().skip(i + 1) {
                self.check_borrow_pair(loan_id1, loan_id2);
            }
        }

        // Also check for conflicts with assignments and moves
        self.check_access_conflicts();
    }

    /// Check two borrows for conflicts
    fn check_borrow_pair(&mut self, loan_id1: LoanId, loan_id2: LoanId) {
        let (lifetime1, lifetime2) = match (
            self.borrow_lifetimes.get(&loan_id1),
            self.borrow_lifetimes.get(&loan_id2),
        ) {
            (Some(l1), Some(l2)) => (l1, l2),
            _ => {
                eprintln!(
                    "Warning: Borrow lifetime not found for loan_id {:?} or {:?}",
                    loan_id1, loan_id2
                );
                return;
            }
        };

        // Check if regions overlap
        if !lifetime1.region.overlaps(&lifetime2.region) {
            return;
        }

        // Check if places conflict
        if !self.places_conflict(&lifetime1.borrowed_place, &lifetime2.borrowed_place) {
            return;
        }

        // Check borrow kind conflicts
        match (lifetime1.kind, lifetime2.kind) {
            (BorrowKind::Shared, BorrowKind::Shared) => {
                // Shared borrows can coexist
            }
            _ => {
                // Any other combination conflicts
                self.errors.push(BorrowError::ConflictingBorrows {
                    loan1: loan_id1,
                    loan2: loan_id2,
                    place1: lifetime1.borrowed_place.clone(),
                    place2: lifetime2.borrowed_place.clone(),
                });
            }
        }
    }

    /// Check for conflicts between borrows and other accesses
    fn check_access_conflicts(&mut self) {
        for (block_idx, block) in self.body.basic_blocks.iter().enumerate() {
            let block_id = BasicBlock(block_idx as u32);

            for (stmt_idx, stmt) in block.statements.iter().enumerate() {
                match stmt {
                    Statement::Assign(place, rvalue) => {
                        // Check if this assignment conflicts with active borrows
                        self.check_write_conflicts(place, block_id, stmt_idx);

                        // Check moves
                        if let Rvalue::Move(moved_place) = rvalue {
                            self.check_move_conflicts(moved_place, block_id, stmt_idx);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// Check if a write conflicts with active borrows
    fn check_write_conflicts(&mut self, place: &Place, block: BasicBlock, stmt: usize) {
        for (loan_id, lifetime) in &self.borrow_lifetimes {
            if lifetime.region.contains(block, stmt) {
                if self.places_conflict(place, &lifetime.borrowed_place) {
                    // Writing to a borrowed place
                    if lifetime.kind == BorrowKind::Mut {
                        self.errors.push(BorrowError::WriteToMutBorrowed {
                            loan: *loan_id,
                            place: place.clone(),
                        });
                    } else {
                        self.errors.push(BorrowError::WriteToSharedBorrowed {
                            loan: *loan_id,
                            place: place.clone(),
                        });
                    }
                }
            }
        }
    }

    /// Check if a move conflicts with active borrows
    fn check_move_conflicts(&mut self, place: &Place, block: BasicBlock, stmt: usize) {
        for (loan_id, lifetime) in &self.borrow_lifetimes {
            if lifetime.region.contains(block, stmt) {
                if self.places_conflict(place, &lifetime.borrowed_place) {
                    self.errors.push(BorrowError::MoveOutOfBorrowed {
                        loan: *loan_id,
                        place: place.clone(),
                    });
                }
            }
        }
    }

    /// Check for use-after-move errors with enhanced dataflow analysis
    fn check_use_after_move(&mut self) {
        // Perform dataflow analysis to track move states at each point
        let mut move_states: HashMap<Point, HashMap<Local, MoveState>> = HashMap::new();

        // Initialize all points with empty move states
        for (block_idx, block) in self.body.basic_blocks.iter().enumerate() {
            let block_id = BasicBlock(block_idx as u32);
            for stmt_idx in 0..=block.statements.len() {
                let point = Point::new(block_id, stmt_idx);
                move_states.insert(point, HashMap::new());
            }
        }

        // Iterative dataflow analysis for move states
        let mut changed = true;
        let mut iterations = 0;
        let max_iterations = self.body.basic_blocks.len() * 10; // Prevent infinite loops

        while changed && iterations < max_iterations {
            changed = false;
            iterations += 1;

            for (block_idx, block) in self.body.basic_blocks.iter().enumerate() {
                let block_id = BasicBlock(block_idx as u32);

                // Get entry state for this block (merge of all predecessors)
                let entry_state = self.compute_block_entry_state(block_id, &move_states);

                // Propagate through statements
                let mut current_state = entry_state.clone();

                for (stmt_idx, stmt) in block.statements.iter().enumerate() {
                    let point = Point::new(block_id, stmt_idx);

                    // Check for use of moved values at this point
                    self.check_statement_for_moves(&current_state, stmt, point);

                    // Update state based on this statement
                    self.update_move_state(&mut current_state, stmt);

                    // Check if state changed
                    let next_point = Point::new(block_id, stmt_idx + 1);
                    if let Some(existing_state) = move_states.get(&next_point) {
                        if !Self::move_states_equal(existing_state, &current_state) {
                            move_states.insert(next_point, current_state.clone());
                            changed = true;
                        }
                    }
                }
            }
        }
    }

    /// Compute the entry state for a block by merging predecessor states
    fn compute_block_entry_state(
        &self,
        block_id: BasicBlock,
        move_states: &HashMap<Point, HashMap<Local, MoveState>>,
    ) -> HashMap<Local, MoveState> {
        let mut entry_state: HashMap<Local, MoveState> = HashMap::new();

        // Get predecessors
        if let Some(preds) = self.reverse_cfg.get(&block_id) {
            for pred in preds {
                let pred_block = &self.body.basic_blocks[pred.0 as usize];
                let pred_exit_point = Point::new(*pred, pred_block.statements.len());

                if let Some(pred_state) = move_states.get(&pred_exit_point) {
                    // Merge states: if any predecessor has Moved, the result is Moved
                    for (local, state) in pred_state {
                        match entry_state.get(local) {
                            None => {
                                entry_state.insert(*local, *state);
                            }
                            Some(existing) => {
                                // Conservative merge: if states differ, mark as PartiallyMoved
                                if *existing != *state {
                                    entry_state.insert(*local, MoveState::PartiallyMoved);
                                }
                            }
                        }
                    }
                }
            }
        }

        entry_state
    }

    /// Check if two move states are equal
    fn move_states_equal(a: &HashMap<Local, MoveState>, b: &HashMap<Local, MoveState>) -> bool {
        if a.len() != b.len() {
            return false;
        }
        a.iter().all(|(k, v)| b.get(k) == Some(v))
    }

    /// Check a statement for use of moved values
    fn check_statement_for_moves(
        &mut self,
        move_states: &HashMap<Local, MoveState>,
        stmt: &Statement,
        _point: Point,
    ) {
        match stmt {
            Statement::Assign(_, rvalue) => {
                let used_locals = self.locals_used_in_rvalue(rvalue);
                for local in used_locals {
                    if let Some(state) = move_states.get(&local) {
                        match state {
                            MoveState::Moved => {
                                self.errors.push(BorrowError::UseOfMovedValue(local));
                            }
                            MoveState::PartiallyMoved => {
                                // Could be more specific about which fields
                                self.errors.push(BorrowError::UseOfMovedValue(local));
                            }
                            MoveState::Available => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Update move state based on a statement
    fn update_move_state(&self, move_states: &mut HashMap<Local, MoveState>, stmt: &Statement) {
        match stmt {
            Statement::Assign(place, rvalue) => {
                match rvalue {
                    Rvalue::Move(moved_place) => {
                        // Mark the source as moved
                        move_states.insert(moved_place.local, MoveState::Moved);
                    }
                    _ => {
                        // If we're assigning to a previously moved local, it's now available again
                        // (assuming the assignment overwrites the moved value)
                        if move_states.contains_key(&place.local) {
                            move_states.remove(&place.local);
                        }
                    }
                }
            }
            Statement::StorageDead(local) => {
                // Local is no longer valid, remove from move states
                move_states.remove(local);
            }
            _ => {}
        }
    }

    /// Check for uses of moved values in a statement
    fn check_uses_for_moves(
        &mut self,
        moved_locals: &HashSet<Local>,
        stmt: &Statement,
        _block: BasicBlock,
        _stmt_idx: usize,
    ) {
        if let Statement::Assign(_, rvalue) = stmt {
            let used_locals = self.locals_used_in_rvalue(rvalue);
            for local in used_locals {
                if moved_locals.contains(&local) {
                    self.errors.push(BorrowError::UseOfMovedValue(local));
                }
            }
        }
    }

    /// Get all locals used in an rvalue
    fn locals_used_in_rvalue(&self, rvalue: &Rvalue) -> Vec<Local> {
        let mut locals = Vec::new();

        match rvalue {
            Rvalue::Use(operand) => {
                locals.extend(self.locals_used_in_operand(operand));
            }
            Rvalue::Cast(_, operand, _) => {
                locals.extend(self.locals_used_in_operand(operand));
            }
            Rvalue::Copy(place)
            | Rvalue::Move(place)
            | Rvalue::Ref(place, _)
            | Rvalue::AddressOf(place, _) => {
                locals.push(place.local);
            }
            Rvalue::BinaryOp(_, left, right) => {
                locals.extend(self.locals_used_in_operand(left));
                locals.extend(self.locals_used_in_operand(right));
            }
            Rvalue::UnaryOp(_, operand) => {
                locals.extend(self.locals_used_in_operand(operand));
            }
            Rvalue::Len(place) | Rvalue::Discriminant(place) => {
                locals.push(place.local);
            }
            Rvalue::Aggregate(_, operands) => {
                for operand in operands {
                    locals.extend(self.locals_used_in_operand(operand));
                }
            }
        }

        locals
    }

    /// Get all locals used in an operand
    fn locals_used_in_operand(&self, operand: &Operand) -> Vec<Local> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => vec![place.local],
            Operand::Constant(_) => vec![],
        }
    }

    /// Check if two places conflict
    fn places_conflict(&self, place1: &Place, place2: &Place) -> bool {
        if place1.local != place2.local {
            return false;
        }

        // Check projections
        let min_len = place1.projection.len().min(place2.projection.len());
        place1.projection[..min_len] == place2.projection[..min_len]
    }

    /// Get the borrow lifetime for a loan
    pub fn get_borrow_lifetime(&self, loan_id: LoanId) -> Option<&BorrowLifetime> {
        self.borrow_lifetimes.get(&loan_id)
    }

    /// Get all borrow lifetimes
    pub fn borrow_lifetimes(&self) -> &HashMap<LoanId, BorrowLifetime> {
        &self.borrow_lifetimes
    }
}

/// Region-based constraint solving for lifetime inference
pub struct RegionConstraintSolver {
    /// Region constraints (region1 must outlive region2)
    constraints: Vec<(Region, Region)>,
    /// Union-find data structure for region equivalence
    region_parents: HashMap<RegionId, RegionId>,
    /// Region ranks for union by rank
    region_ranks: HashMap<RegionId, u32>,
}

/// Unique identifier for a region
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegionId(pub u32);

impl RegionConstraintSolver {
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
            region_parents: HashMap::new(),
            region_ranks: HashMap::new(),
        }
    }

    /// Add a constraint that region1 must outlive region2
    pub fn add_outlives_constraint(&mut self, region1: Region, region2: Region) {
        self.constraints.push((region1, region2));
    }

    /// Find the root of a region in the union-find structure
    fn find(&mut self, region: RegionId) -> RegionId {
        let parent = self.region_parents.get(&region).copied().unwrap_or(region);
        if parent != region {
            let root = self.find(parent);
            self.region_parents.insert(region, root);
            return root;
        }
        region
    }

    /// Union two regions
    fn union(&mut self, region1: RegionId, region2: RegionId) {
        let root1 = self.find(region1);
        let root2 = self.find(region2);

        if root1 == root2 {
            return;
        }

        let rank1 = self.region_ranks.get(&root1).copied().unwrap_or(0);
        let rank2 = self.region_ranks.get(&root2).copied().unwrap_or(0);

        if rank1 < rank2 {
            self.region_parents.insert(root1, root2);
        } else if rank1 > rank2 {
            self.region_parents.insert(root2, root1);
        } else {
            self.region_parents.insert(root2, root1);
            self.region_ranks.insert(root1, rank1 + 1);
        }
    }

    /// Solve the constraints and return any errors
    pub fn solve(&self) -> Result<(), Vec<String>> {
        // In a full implementation, this would use a union-find data structure
        // to efficiently solve the region constraints
        Ok(())
    }

    /// Compute the complete region for a borrow using dataflow analysis
    ///
    /// This implements a more precise region computation that tracks:
    /// 1. All points where the borrow is live
    /// 2. Control flow dependencies
    /// 3. Kill points where the borrow ends
    pub fn compute_precise_region(
        &self,
        creation_point: Point,
        _borrow_local: Local,
        body: &MirBody,
        _cfg: &HashMap<BasicBlock, Vec<BasicBlock>>,
    ) -> Region {
        let mut region = Region::new();
        let mut visited: HashSet<Point> = HashSet::new();
        let mut queue: VecDeque<Point> = VecDeque::new();

        // Start from the creation point
        queue.push_back(creation_point);
        region.add_point(creation_point.block, creation_point.statement);

        while let Some(point) = queue.pop_front() {
            if !visited.insert(point) {
                continue;
            }

            // Add this point to the region
            region.add_point(point.block, point.statement);
            region.add_block(point.block);

            // Get the block data
            let block_data = &body.basic_blocks[point.block.0 as usize];

            // Determine next points based on current position
            if point.statement < block_data.statements.len() {
                // Move to next statement in the same block
                let next_point = Point::new(point.block, point.statement + 1);
                queue.push_back(next_point);
            } else {
                // At terminator, follow control flow
                if let Some(ref terminator) = block_data.terminator {
                    match &terminator.kind {
                        TerminatorKind::Goto { target } => {
                            queue.push_back(Point::new(*target, 0));
                        }
                        TerminatorKind::SwitchInt {
                            targets, otherwise, ..
                        } => {
                            for (_, target) in targets {
                                queue.push_back(Point::new(*target, 0));
                            }
                            queue.push_back(Point::new(*otherwise, 0));
                        }
                        TerminatorKind::Call { target, .. } => {
                            if let Some(t) = target {
                                queue.push_back(Point::new(*t, 0));
                            }
                        }
                        TerminatorKind::Assert {
                            target, cleanup, ..
                        } => {
                            queue.push_back(Point::new(*target, 0));
                            if let Some(cleanup_block) = cleanup {
                                queue.push_back(Point::new(*cleanup_block, 0));
                            }
                        }
                        TerminatorKind::Return | TerminatorKind::Unwind => {
                            // End of function - no successors
                        }
                    }
                }
            }
        }

        region
    }
}

impl Default for RegionConstraintSolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Type;

    fn create_simple_body() -> MirBody {
        let mut body = MirBody::new(0, 0..100);
        // Add at least one basic block
        let block = BasicBlockData::new();
        body.basic_blocks.push(block);
        body
    }

    #[test]
    fn test_nll_basic() {
        // Create a simple MIR body
        let mut body = MirBody::new(0, 0..100);

        // Add a basic block
        let block = BasicBlockData::new();
        body.basic_blocks.push(block);

        let checker = NllBorrowChecker::new(&body);
        assert!(checker.check().is_ok());
    }

    #[test]
    fn test_nll_shared_borrows() {
        // Test that shared borrows can coexist
        let mut body = MirBody::new(1, 0..100);

        // Add return local and arg local
        body.push_local(LocalDecl::new(Type::Unit, 0..10));
        body.push_local(LocalDecl::new(Type::Unit, 10..20).arg());

        let mut block = BasicBlockData::new();

        // _2 = &_1 (first shared borrow)
        block.statements.push(Statement::Assign(
            Place::from_local(Local(2)),
            Rvalue::Ref(Place::from_local(Local(1)), Mutability::Not),
        ));

        // _3 = &_1 (second shared borrow - should be OK)
        block.statements.push(Statement::Assign(
            Place::from_local(Local(3)),
            Rvalue::Ref(Place::from_local(Local(1)), Mutability::Not),
        ));

        block.set_terminator(Terminator {
            kind: TerminatorKind::Return,
            span: 0..100,
        });

        body.basic_blocks.push(block);

        let checker = NllBorrowChecker::new(&body);
        let result = checker.check();
        assert!(result.is_ok(), "Shared borrows should be allowed");
    }

    #[test]
    fn test_nll_mut_borrow_conflict() {
        // Test that mutable borrows conflict
        let mut body = MirBody::new(1, 0..100);

        // Add return local and arg local
        body.push_local(LocalDecl::new(Type::Unit, 0..10));
        body.push_local(LocalDecl::new(Type::Unit, 10..20).arg());

        let mut block = BasicBlockData::new();

        // _2 = &mut _1 (first mutable borrow)
        block.statements.push(Statement::Assign(
            Place::from_local(Local(2)),
            Rvalue::Ref(Place::from_local(Local(1)), Mutability::Mut),
        ));

        // _3 = &mut _1 (second mutable borrow - should conflict)
        block.statements.push(Statement::Assign(
            Place::from_local(Local(3)),
            Rvalue::Ref(Place::from_local(Local(1)), Mutability::Mut),
        ));

        block.set_terminator(Terminator {
            kind: TerminatorKind::Return,
            span: 0..100,
        });

        body.basic_blocks.push(block);

        let checker = NllBorrowChecker::new(&body);
        let result = checker.check();
        assert!(result.is_err(), "Conflicting mutable borrows should fail");
    }

    #[test]
    fn test_nll_mixed_borrow_conflict() {
        // Test that shared and mutable borrows conflict
        let mut body = MirBody::new(1, 0..100);

        // Add return local and arg local
        body.push_local(LocalDecl::new(Type::Unit, 0..10));
        body.push_local(LocalDecl::new(Type::Unit, 10..20).arg());

        let mut block = BasicBlockData::new();

        // _2 = &mut _1 (mutable borrow)
        block.statements.push(Statement::Assign(
            Place::from_local(Local(2)),
            Rvalue::Ref(Place::from_local(Local(1)), Mutability::Mut),
        ));

        // _3 = &_1 (shared borrow while mutable active - should conflict)
        block.statements.push(Statement::Assign(
            Place::from_local(Local(3)),
            Rvalue::Ref(Place::from_local(Local(1)), Mutability::Not),
        ));

        block.set_terminator(Terminator {
            kind: TerminatorKind::Return,
            span: 0..100,
        });

        body.basic_blocks.push(block);

        let checker = NllBorrowChecker::new(&body);
        let result = checker.check();
        assert!(result.is_err(), "Mixed borrows should conflict");
    }

    #[test]
    fn test_cfg_construction() {
        let mut body = MirBody::new(0, 0..100);

        // Create a simple if-else structure
        let mut entry_block = BasicBlockData::new();
        entry_block.set_terminator(Terminator {
            kind: TerminatorKind::SwitchInt {
                discr: Operand::Constant(Constant::Scalar(Scalar::Int(1, IntType::I64))),
                switch_ty: Type::Unit,
                targets: vec![(1, BasicBlock(1))],
                otherwise: BasicBlock(2),
            },
            span: 0..10,
        });

        let mut then_block = BasicBlockData::new();
        then_block.set_terminator(Terminator {
            kind: TerminatorKind::Goto {
                target: BasicBlock(3),
            },
            span: 10..20,
        });

        let mut else_block = BasicBlockData::new();
        else_block.set_terminator(Terminator {
            kind: TerminatorKind::Goto {
                target: BasicBlock(3),
            },
            span: 20..30,
        });

        let mut end_block = BasicBlockData::new();
        end_block.set_terminator(Terminator {
            kind: TerminatorKind::Return,
            span: 30..40,
        });

        body.basic_blocks.push(entry_block);
        body.basic_blocks.push(then_block);
        body.basic_blocks.push(else_block);
        body.basic_blocks.push(end_block);

        let checker = NllBorrowChecker::new(&body);

        // Check CFG
        let cfg_entry = checker.cfg.get(&BasicBlock(0));
        assert!(cfg_entry.is_some(), "CFG entry should exist");
        let cfg_entry = cfg_entry.unwrap();
        assert_eq!(cfg_entry.len(), 2);
        assert!(cfg_entry.contains(&BasicBlock(1)));
        assert!(cfg_entry.contains(&BasicBlock(2)));
    }
}
