//! Loop Invariant Code Motion (LICM)
//!
//! This pass identifies computations inside loops that produce the same
//! result on every iteration and moves them outside the loop.
//!
//! For example:
//! ```
//! for i in 0..n {
//!     let x = a + b;  // a and b are not modified in the loop
//!     let y = x * i;
//! }
//! ```
//!
//! Becomes:
//! ```
//! let x = a + b;  // Moved outside the loop
//! for i in 0..n {
//!     let y = x * i;
//! }
//! ```
//!
//! The algorithm:
//! 1. Identify loop headers and loop bodies
//! 2. For each instruction in the loop, check if it's loop-invariant
//! 3. Check if the instruction can be safely hoisted
//! 4. Move loop-invariant instructions to the loop preheader

use crate::middle::mir::optimize::MirPass;
use crate::middle::mir::*;
use std::collections::{HashMap, HashSet};

/// Loop information
#[derive(Debug, Clone)]
pub struct LoopInfo {
    /// The loop header block
    pub header: BasicBlock,
    /// Blocks that belong to this loop
    pub body: HashSet<BasicBlock>,
    /// Predecessor blocks outside the loop (preheader candidates)
    pub preheaders: Vec<BasicBlock>,
}

impl LoopInfo {
    pub fn new(header: BasicBlock) -> Self {
        Self {
            header,
            body: HashSet::new(),
            preheaders: Vec::new(),
        }
    }

    pub fn contains(&self, block: BasicBlock) -> bool {
        self.body.contains(&block)
    }
}

/// Loop invariant code motion pass
#[derive(Debug, Default)]
pub struct LoopInvariantCodeMotion {
    /// Identified loops
    loops: Vec<LoopInfo>,
    /// Number of instructions hoisted
    hoisted_count: usize,
}

impl LoopInvariantCodeMotion {
    pub fn new() -> Self {
        Self {
            loops: Vec::new(),
            hoisted_count: 0,
        }
    }

    pub fn hoisted_count(&self) -> usize {
        self.hoisted_count
    }

    fn find_loops(&mut self, body: &MirBody) {
        self.loops.clear();

        let mut back_edges: HashMap<BasicBlock, Vec<BasicBlock>> = HashMap::new();

        for (block_idx, block) in body.basic_blocks.iter().enumerate() {
            if let Some(ref terminator) = block.terminator {
                for successor in Self::get_successors(terminator) {
                    if successor.0 as usize <= block_idx {
                        back_edges.entry(successor).or_default().push(BasicBlock(block_idx as u32));
                    }
                }
            }
        }

        for (header, back_edge_sources) in back_edges {
            let mut loop_info = LoopInfo::new(header);
            loop_info.body.insert(header);

            for back_edge_source in back_edge_sources {
                self.find_loop_body(body, back_edge_source, header, &mut loop_info);
            }

            for (block_idx, block) in body.basic_blocks.iter().enumerate() {
                if !loop_info.contains(BasicBlock(block_idx as u32)) {
                    if let Some(ref terminator) = block.terminator {
                        for successor in Self::get_successors(terminator) {
                            if successor == header {
                                loop_info.preheaders.push(BasicBlock(block_idx as u32));
                            }
                        }
                    }
                }
            }

            self.loops.push(loop_info);
        }
    }

    fn find_loop_body(
        &self,
        body: &MirBody,
        current: BasicBlock,
        header: BasicBlock,
        loop_info: &mut LoopInfo,
    ) {
        if loop_info.contains(current) {
            return;
        }

        loop_info.body.insert(current);

        if let Some(block) = body.basic_blocks.get(current.index()) {
            if let Some(ref terminator) = block.terminator {
                for predecessor in Self::get_predecessors_hint(body, current) {
                    if predecessor != header {
                        self.find_loop_body(body, predecessor, header, loop_info);
                    }
                }
            }
        }
    }

    fn get_predecessors_hint(body: &MirBody, target: BasicBlock) -> Vec<BasicBlock> {
        let mut predecessors = Vec::new();
        for (block_idx, block) in body.basic_blocks.iter().enumerate() {
            if let Some(ref terminator) = block.terminator {
                for successor in Self::get_successors(terminator) {
                    if successor == target {
                        predecessors.push(BasicBlock(block_idx as u32));
                    }
                }
            }
        }
        predecessors
    }

    fn get_successors(terminator: &Terminator) -> Vec<BasicBlock> {
        match &terminator.kind {
            TerminatorKind::Goto { target } => vec![*target],
            TerminatorKind::SwitchInt { targets, otherwise, .. } => {
                let mut succs: Vec<_> = targets.iter().map(|(_, b)| *b).collect();
                succs.push(*otherwise);
                succs
            }
            TerminatorKind::Call { target, .. } => {
                target.map(|t| vec![t]).unwrap_or_default()
            }
            TerminatorKind::Assert { target, .. } => vec![*target],
            _ => vec![],
        }
    }

    fn is_loop_invariant(
        &self,
        rvalue: &Rvalue,
        loop_info: &LoopInfo,
        defined_outside: &HashSet<Local>,
    ) -> bool {
        match rvalue {
            Rvalue::Use(operand) => {
                self.is_operand_invariant(operand, loop_info, defined_outside)
            }
            Rvalue::Copy(place) | Rvalue::Move(place) => {
                defined_outside.contains(&place.local)
            }
            Rvalue::BinaryOp(_, left, right) => {
                self.is_operand_invariant(left, loop_info, defined_outside)
                    && self.is_operand_invariant(right, loop_info, defined_outside)
            }
            Rvalue::UnaryOp(_, operand) => {
                self.is_operand_invariant(operand, loop_info, defined_outside)
            }
            Rvalue::Cast(_, operand, _) => {
                self.is_operand_invariant(operand, loop_info, defined_outside)
            }
            Rvalue::Len(place) => {
                defined_outside.contains(&place.local)
            }
            Rvalue::Discriminant(place) => {
                defined_outside.contains(&place.local)
            }
            Rvalue::Aggregate(_, operands) => {
                operands
                    .iter()
                    .all(|o| self.is_operand_invariant(o, loop_info, defined_outside))
            }
            Rvalue::Ref(place, _) | Rvalue::AddressOf(place, _) => {
                defined_outside.contains(&place.local)
            }
        }
    }

    fn is_operand_invariant(
        &self,
        operand: &Operand,
        loop_info: &LoopInfo,
        defined_outside: &HashSet<Local>,
    ) -> bool {
        match operand {
            Operand::Constant(_) => true,
            Operand::Copy(place) | Operand::Move(place) => {
                defined_outside.contains(&place.local)
            }
        }
    }

    fn can_hoist_safely(
        &self,
        place: &Place,
        rvalue: &Rvalue,
        _loop_info: &LoopInfo,
        _body: &MirBody,
    ) -> bool {
        if let Rvalue::BinaryOp(op, _, _) = rvalue {
            match op {
                BinOp::Div | BinOp::Rem => {
                    return false;
                }
                _ => {}
            }
        }

        let mut has_side_effects = false;
        if let Rvalue::BinaryOp(_, left, right) = rvalue {
            if let Operand::Copy(p) | Operand::Move(p) = &**left {
                if !p.projection.is_empty() {
                    has_side_effects = true;
                }
            }
            if let Operand::Copy(p) | Operand::Move(p) = &**right {
                if !p.projection.is_empty() {
                    has_side_effects = true;
                }
            }
        }

        !has_side_effects
    }

    fn hoist_invariants(&mut self, body: &mut MirBody) {
        for loop_info in &self.loops.clone() {
            let mut defined_outside: HashSet<Local> = HashSet::new();

            for (block_idx, block) in body.basic_blocks.iter().enumerate() {
                if !loop_info.contains(BasicBlock(block_idx as u32)) {
                    for stmt in &block.statements {
                        if let Statement::Assign(place, _) = stmt {
                            defined_outside.insert(place.local);
                        }
                    }
                }
            }

            let mut hoistable: Vec<(BasicBlock, usize, Statement)> = Vec::new();

            for &block_id in &loop_info.body {
                if let Some(block) = body.basic_blocks.get(block_id.index()) {
                    for (stmt_idx, stmt) in block.statements.iter().enumerate() {
                        if let Statement::Assign(place, rvalue) = stmt {
                            if place.projection.is_empty()
                                && self.is_loop_invariant(rvalue, loop_info, &defined_outside)
                                && self.can_hoist_safely(place, rvalue, loop_info, body)
                            {
                                hoistable.push((
                                    block_id,
                                    stmt_idx,
                                    Statement::Assign(place.clone(), rvalue.clone()),
                                ));
                            }
                        }
                    }
                }
            }

            if let Some(&preheader) = loop_info.preheaders.first() {
                for (block_id, _stmt_idx, stmt) in hoistable {
                    let stmt_clone = stmt.clone();
                    if let Some(block) = body.basic_blocks.get_mut(preheader.index()) {
                        let insert_pos = block.statements.len().saturating_sub(1);
                        block.statements.insert(insert_pos, stmt);
                        self.hoisted_count += 1;
                    }

                    if let Some(block) = body.basic_blocks.get_mut(block_id.index()) {
                        block.statements.retain(|s| s != &stmt_clone);
                    }
                }
            }
        }
    }
}

impl MirPass for LoopInvariantCodeMotion {
    fn name(&self) -> &'static str {
        "loop_invariant_code_motion"
    }

    fn run(&self, body: &mut MirBody) {
        let mut this = Self::new();
        this.find_loops(body);
        this.hoist_invariants(body);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loop_info_creation() {
        let loop_info = LoopInfo::new(BasicBlock(1));
        assert_eq!(loop_info.header, BasicBlock(1));
        assert!(loop_info.body.is_empty());
    }

    #[test]
    fn test_loop_info_contains() {
        let mut loop_info = LoopInfo::new(BasicBlock(1));
        loop_info.body.insert(BasicBlock(1));
        loop_info.body.insert(BasicBlock(2));

        assert!(loop_info.contains(BasicBlock(1)));
        assert!(loop_info.contains(BasicBlock(2)));
        assert!(!loop_info.contains(BasicBlock(3)));
    }

    #[test]
    fn test_licm_creation() {
        let licm = LoopInvariantCodeMotion::new();
        assert_eq!(licm.hoisted_count(), 0);
    }

    #[test]
    fn test_is_loop_invariant_constant() {
        let licm = LoopInvariantCodeMotion::new();
        let loop_info = LoopInfo::new(BasicBlock(1));
        let defined_outside = HashSet::new();

        let rvalue = Rvalue::Use(Operand::Constant(Constant::Scalar(Scalar::Int(
            42,
            IntType::I64,
        ))));
        assert!(licm.is_loop_invariant(&rvalue, &loop_info, &defined_outside));
    }

    #[test]
    fn test_is_loop_invariant_binary_op() {
        let licm = LoopInvariantCodeMotion::new();
        let loop_info = LoopInfo::new(BasicBlock(1));
        let mut defined_outside = HashSet::new();
        defined_outside.insert(Local(1));
        defined_outside.insert(Local(2));

        let rvalue = Rvalue::BinaryOp(
            BinOp::Add,
            Box::new(Operand::Copy(Place::from_local(Local(1)))),
            Box::new(Operand::Copy(Place::from_local(Local(2)))),
        );
        assert!(licm.is_loop_invariant(&rvalue, &loop_info, &defined_outside));
    }

    #[test]
    fn test_is_loop_invariant_loop_defined() {
        let licm = LoopInvariantCodeMotion::new();
        let loop_info = LoopInfo::new(BasicBlock(1));
        let defined_outside = HashSet::new();

        let rvalue = Rvalue::Use(Operand::Copy(Place::from_local(Local(1))));
        assert!(!licm.is_loop_invariant(&rvalue, &loop_info, &defined_outside));
    }

    #[test]
    fn test_can_hoist_safely_add() {
        let licm = LoopInvariantCodeMotion::new();
        let loop_info = LoopInfo::new(BasicBlock(1));
        let body = MirBody::new(0, 0..10);

        let rvalue = Rvalue::BinaryOp(
            BinOp::Add,
            Box::new(Operand::Copy(Place::from_local(Local(1)))),
            Box::new(Operand::Copy(Place::from_local(Local(2)))),
        );
        let place = Place::from_local(Local(3));

        assert!(licm.can_hoist_safely(&place, &rvalue, &loop_info, &body));
    }

    #[test]
    fn test_can_hoist_safely_div() {
        let licm = LoopInvariantCodeMotion::new();
        let loop_info = LoopInfo::new(BasicBlock(1));
        let body = MirBody::new(0, 0..10);

        let rvalue = Rvalue::BinaryOp(
            BinOp::Div,
            Box::new(Operand::Copy(Place::from_local(Local(1)))),
            Box::new(Operand::Copy(Place::from_local(Local(2)))),
        );
        let place = Place::from_local(Local(3));

        assert!(!licm.can_hoist_safely(&place, &rvalue, &loop_info, &body));
    }
}
