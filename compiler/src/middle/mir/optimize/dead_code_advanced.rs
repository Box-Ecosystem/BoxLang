//! Advanced Dead Code Elimination
//!
//! This pass extends basic dead code elimination with:
//! - Aggressive dead store elimination
//! - Unused parameter detection
//! - Dead local elimination
//! - Removal of dead basic blocks (unreachable code)
//!
//! The algorithm uses a worklist-based approach to iteratively
//! identify and remove dead code until a fixpoint is reached.

use crate::middle::mir::optimize::MirPass;
use crate::middle::mir::*;
use std::collections::{HashMap, HashSet};

/// Advanced dead code elimination pass
#[derive(Debug, Default)]
pub struct AdvancedDeadCodeElimination {
    /// Locals that are definitely used
    used_locals: HashSet<Local>,
    /// Locals that have side effects when computed
    has_side_effects: HashSet<Local>,
    /// Number of statements removed
    removed_count: usize,
    /// Number of blocks removed
    blocks_removed: usize,
}

impl AdvancedDeadCodeElimination {
    pub fn new() -> Self {
        Self {
            used_locals: HashSet::new(),
            has_side_effects: HashSet::new(),
            removed_count: 0,
            blocks_removed: 0,
        }
    }

    pub fn removed_count(&self) -> usize {
        self.removed_count
    }

    pub fn blocks_removed(&self) -> usize {
        self.blocks_removed
    }

    fn find_all_used_locals(&mut self, body: &MirBody) {
        self.used_locals.clear();
        self.has_side_effects.clear();

        self.used_locals.insert(Local::RETURN_PLACE);

        for i in 0..body.arg_count {
            self.used_locals.insert(Local((i + 1) as u32));
        }

        let mut changed = true;
        while changed {
            changed = false;

            for block in &body.basic_blocks {
                for stmt in &block.statements {
                    if let Statement::Assign(place, rvalue) = stmt {
                        if self.used_locals.contains(&place.local) {
                            let mut new_used = HashSet::new();
                            Self::find_used_in_rvalue(rvalue, &mut new_used);
                            
                            for local in new_used {
                                if !self.used_locals.contains(&local) {
                                    self.used_locals.insert(local);
                                    changed = true;
                                }
                            }
                        }
                        
                        if Self::rvalue_has_side_effects(rvalue) {
                            self.has_side_effects.insert(place.local);
                        }
                    }
                }

                if let Some(ref terminator) = block.terminator {
                    let mut new_used = HashSet::new();
                    Self::find_used_in_terminator(terminator, &mut new_used);
                    
                    for local in new_used {
                        if !self.used_locals.contains(&local) {
                            self.used_locals.insert(local);
                            changed = true;
                        }
                    }
                }
            }
        }
    }

    fn find_used_in_rvalue(rvalue: &Rvalue, used: &mut HashSet<Local>) {
        match rvalue {
            Rvalue::Use(operand) => {
                Self::find_used_in_operand(operand, used);
            }
            Rvalue::Copy(place) | Rvalue::Move(place) | Rvalue::Ref(place, _) | Rvalue::AddressOf(place, _) => {
                Self::find_used_in_place(place, used);
            }
            Rvalue::BinaryOp(_, left, right) => {
                Self::find_used_in_operand(left, used);
                Self::find_used_in_operand(right, used);
            }
            Rvalue::UnaryOp(_, operand) => {
                Self::find_used_in_operand(operand, used);
            }
            Rvalue::Cast(_, operand, _) => {
                Self::find_used_in_operand(operand, used);
            }
            Rvalue::Len(place) | Rvalue::Discriminant(place) => {
                Self::find_used_in_place(place, used);
            }
            Rvalue::Aggregate(_, operands) => {
                for operand in operands {
                    Self::find_used_in_operand(operand, used);
                }
            }
        }
    }

    fn find_used_in_operand(operand: &Operand, used: &mut HashSet<Local>) {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                Self::find_used_in_place(place, used);
            }
            Operand::Constant(_) => {}
        }
    }

    fn find_used_in_place(place: &Place, used: &mut HashSet<Local>) {
        used.insert(place.local);
        for elem in &place.projection {
            if let PlaceElem::Index(index_local) = elem {
                used.insert(*index_local);
            }
        }
    }

    fn find_used_in_terminator(terminator: &Terminator, used: &mut HashSet<Local>) {
        match &terminator.kind {
            TerminatorKind::SwitchInt { discr, .. } => {
                Self::find_used_in_operand(discr, used);
            }
            TerminatorKind::Call { func, args, .. } => {
                Self::find_used_in_operand(func, used);
                for arg in args {
                    Self::find_used_in_operand(arg, used);
                }
            }
            TerminatorKind::Assert { cond, .. } => {
                Self::find_used_in_operand(cond, used);
            }
            TerminatorKind::Return => {
                used.insert(Local::RETURN_PLACE);
            }
            _ => {}
        }
    }

    fn rvalue_has_side_effects(rvalue: &Rvalue) -> bool {
        match rvalue {
            Rvalue::Use(operand) => Self::operand_has_side_effects(operand),
            Rvalue::BinaryOp(_, left, right) => {
                Self::operand_has_side_effects(left) || Self::operand_has_side_effects(right)
            }
            Rvalue::UnaryOp(_, operand) => Self::operand_has_side_effects(operand),
            _ => false,
        }
    }

    fn operand_has_side_effects(operand: &Operand) -> bool {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                !place.projection.is_empty()
            }
            Operand::Constant(_) => false,
        }
    }

    fn remove_dead_statements(&mut self, body: &mut MirBody) {
        for block in body.basic_blocks.iter_mut() {
            let original_len = block.statements.len();
            
            block.statements.retain(|stmt| {
                match stmt {
                    Statement::Assign(place, rvalue) => {
                        let is_used = self.used_locals.contains(&place.local);
                        let has_effects = self.has_side_effects.contains(&place.local) 
                            || Self::rvalue_has_side_effects(rvalue);
                        is_used || has_effects
                    }
                    Statement::StorageLive(local) | Statement::StorageDead(local) => {
                        self.used_locals.contains(local)
                    }
                    Statement::InlineAsm(_) => true,
                    Statement::Nop => false,
                }
            });

            self.removed_count += original_len - block.statements.len();
        }
    }

    fn remove_unreachable_blocks(&mut self, body: &mut MirBody) {
        let reachable = self.find_reachable_blocks(body);
        
        let original_len = body.basic_blocks.len();
        
        let mut new_indices: HashMap<BasicBlock, BasicBlock> = HashMap::new();
        let mut new_blocks = Vec::new();

        for (old_idx, block) in body.basic_blocks.drain(..).enumerate() {
            let old_block = BasicBlock(old_idx as u32);
            if reachable.contains(&old_block) {
                let new_idx = new_blocks.len();
                new_indices.insert(old_block, BasicBlock(new_idx as u32));
                new_blocks.push(block);
            }
        }

        body.basic_blocks = new_blocks;

        for block in body.basic_blocks.iter_mut() {
            if let Some(ref mut terminator) = block.terminator {
                self.update_terminator_blocks(terminator, &new_indices);
            }
        }

        self.blocks_removed = original_len - body.basic_blocks.len();
    }

    fn find_reachable_blocks(&self, body: &MirBody) -> HashSet<BasicBlock> {
        let mut reachable = HashSet::new();
        let mut worklist = vec![BasicBlock(0)];

        while let Some(block) = worklist.pop() {
            if reachable.insert(block) {
                if let Some(block_data) = body.basic_blocks.get(block.index()) {
                    if let Some(ref terminator) = block_data.terminator {
                        for target in Self::get_successors(terminator) {
                            if !reachable.contains(&target) {
                                worklist.push(target);
                            }
                        }
                    }
                }
            }
        }

        reachable
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

    fn update_terminator_blocks(
        &self,
        terminator: &mut Terminator,
        mapping: &HashMap<BasicBlock, BasicBlock>,
    ) {
        match &mut terminator.kind {
            TerminatorKind::Goto { target } => {
                if let Some(&new) = mapping.get(target) {
                    *target = new;
                }
            }
            TerminatorKind::SwitchInt {
                targets, otherwise, ..
            } => {
                for (_, target) in targets.iter_mut() {
                    if let Some(&new) = mapping.get(target) {
                        *target = new;
                    }
                }
                if let Some(&new) = mapping.get(otherwise) {
                    *otherwise = new;
                }
            }
            TerminatorKind::Call { target, .. } => {
                if let Some(ref mut t) = target {
                    if let Some(&new) = mapping.get(t) {
                        *t = new;
                    }
                }
            }
            TerminatorKind::Assert { target, .. } => {
                if let Some(&new) = mapping.get(target) {
                    *target = new;
                }
            }
            _ => {}
        }
    }
}

impl MirPass for AdvancedDeadCodeElimination {
    fn name(&self) -> &'static str {
        "advanced_dead_code_elimination"
    }

    fn run(&self, body: &mut MirBody) {
        let mut this = Self::new();
        
        loop {
            let prev_removed = this.removed_count;
            
            this.find_all_used_locals(body);
            this.remove_dead_statements(body);
            this.remove_unreachable_blocks(body);
            
            if this.removed_count == prev_removed {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dead_assignment_removal() {
        let mut body = MirBody::new(0, 0..100);

        let mut block = BasicBlockData::new();

        block.statements.push(Statement::Assign(
            Place::from_local(Local(1)),
            Rvalue::Use(Operand::Constant(Constant::Scalar(Scalar::Int(5, IntType::I64)))),
        ));

        block.statements.push(Statement::Assign(
            Place::from_local(Local::RETURN_PLACE),
            Rvalue::Use(Operand::Constant(Constant::Scalar(Scalar::Int(10, IntType::I64)))),
        ));

        block.terminator = Some(Terminator {
            kind: TerminatorKind::Return,
            span: 0..10,
        });

        body.basic_blocks.push(block);

        let pass = AdvancedDeadCodeElimination::new();
        pass.run(&mut body);

        assert_eq!(body.basic_blocks[0].statements.len(), 1);
    }

    #[test]
    fn test_unreachable_block_removal() {
        let mut body = MirBody::new(0, 0..100);

        let mut block0 = BasicBlockData::new();
        block0.terminator = Some(Terminator {
            kind: TerminatorKind::Return,
            span: 0..10,
        });
        body.basic_blocks.push(block0);

        let mut block1 = BasicBlockData::new();
        block1.terminator = Some(Terminator {
            kind: TerminatorKind::Return,
            span: 10..20,
        });
        body.basic_blocks.push(block1);

        let pass = AdvancedDeadCodeElimination::new();
        pass.run(&mut body);

        assert_eq!(body.basic_blocks.len(), 1);
    }

    #[test]
    fn test_used_value_preserved() {
        let mut body = MirBody::new(0, 0..100);

        let mut block = BasicBlockData::new();

        block.statements.push(Statement::Assign(
            Place::from_local(Local(1)),
            Rvalue::Use(Operand::Constant(Constant::Scalar(Scalar::Int(5, IntType::I64)))),
        ));

        block.statements.push(Statement::Assign(
            Place::from_local(Local::RETURN_PLACE),
            Rvalue::Use(Operand::Copy(Place::from_local(Local(1)))),
        ));

        block.terminator = Some(Terminator {
            kind: TerminatorKind::Return,
            span: 0..10,
        });

        body.basic_blocks.push(block);

        let pass = AdvancedDeadCodeElimination::new();
        pass.run(&mut body);

        assert_eq!(body.basic_blocks[0].statements.len(), 2);
    }
}
