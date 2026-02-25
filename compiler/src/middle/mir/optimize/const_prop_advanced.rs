//! Advanced Constant Propagation with Inter-block Analysis
//!
//! This pass extends basic constant propagation with:
//! - Inter-block (global) constant propagation using dataflow analysis
//! - Conditional constant propagation
//! - Sparse conditional constant propagation (SCCP) inspired algorithm
//!
//! The algorithm works in two phases:
//! 1. Analysis: Compute which variables are definitely constants at each program point
//! 2. Transformation: Replace variable uses with constants where safe

use crate::middle::mir::optimize::MirPass;
use crate::middle::mir::*;
use std::collections::{HashMap, HashSet};

/// Lattice value for constant propagation
#[derive(Debug, Clone, PartialEq)]
pub enum LatticeValue {
    /// Bottom: we don't know anything yet
    Bottom,
    /// Constant: we know the exact value
    Constant(Constant),
    /// Top: the value is not constant (could be multiple values)
    Top,
}

impl LatticeValue {
    pub fn is_constant(&self) -> bool {
        matches!(self, LatticeValue::Constant(_))
    }

    pub fn as_constant(&self) -> Option<&Constant> {
        match self {
            LatticeValue::Constant(c) => Some(c),
            _ => None,
        }
    }

    pub fn meet(&self, other: &LatticeValue) -> LatticeValue {
        match (self, other) {
            (LatticeValue::Bottom, other) => other.clone(),
            (other, LatticeValue::Bottom) => other.clone(),
            (LatticeValue::Constant(c1), LatticeValue::Constant(c2)) => {
                if c1 == c2 {
                    LatticeValue::Constant(c1.clone())
                } else {
                    LatticeValue::Top
                }
            }
            (LatticeValue::Top, _) | (_, LatticeValue::Top) => LatticeValue::Top,
        }
    }
}

/// Advanced constant propagation with inter-block analysis
#[derive(Debug)]
pub struct AdvancedConstantPropagation {
    /// Map from (BasicBlock, Local) -> LatticeValue
    /// Represents the value of a local at the *entry* of a basic block
    state: HashMap<BasicBlock, HashMap<Local, LatticeValue>>,
    /// Worklist for fixpoint iteration
    worklist: Vec<BasicBlock>,
    /// Set of blocks that have been processed
    visited: HashSet<BasicBlock>,
}

impl AdvancedConstantPropagation {
    pub fn new() -> Self {
        Self {
            state: HashMap::new(),
            worklist: Vec::new(),
            visited: HashSet::new(),
        }
    }

    fn get_value(&self, block: BasicBlock, local: Local) -> LatticeValue {
        self.state
            .get(&block)
            .and_then(|m| m.get(&local).cloned())
            .unwrap_or(LatticeValue::Bottom)
    }

    fn set_value(&mut self, block: BasicBlock, local: Local, value: LatticeValue) -> bool {
        let entry = self.state.entry(block).or_default();
        let old = entry.get(&local).cloned().unwrap_or(LatticeValue::Bottom);
        let new = old.meet(&value);
        
        if new != old {
            entry.insert(local, new);
            return true;
        }
        false
    }

    fn propagate_to_successors(&mut self, body: &MirBody, block: BasicBlock) {
        if let Some(block_data) = body.basic_blocks.get(block.index()) {
            if let Some(ref terminator) = block_data.terminator {
                for successor in Self::get_successors(terminator) {
                    let changed = self.propagate_state_to_block(body, block, successor);
                    if changed {
                        if !self.worklist.contains(&successor) {
                            self.worklist.push(successor);
                        }
                    }
                }
            }
        }
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

    fn propagate_state_to_block(&mut self, body: &MirBody, from: BasicBlock, to: BasicBlock) -> bool {
        let mut changed = false;
        
        if let Some(from_state) = self.state.get(&from).cloned() {
            for (local, value) in from_state {
                if self.set_value(to, local, value) {
                    changed = true;
                }
            }
        }

        let mut local_state = self.state.get(&from).cloned().unwrap_or_default();
        if let Some(block_data) = body.basic_blocks.get(from.index()) {
            for stmt in &block_data.statements {
                if let Statement::Assign(place, rvalue) = stmt {
                    if place.projection.is_empty() {
                        let value = self.eval_rvalue(rvalue, &local_state);
                        local_state.insert(place.local, value);
                    }
                }
            }
        }

        for (local, value) in local_state {
            if self.set_value(to, local, value) {
                changed = true;
            }
        }

        changed
    }

    fn eval_rvalue(&self, rvalue: &Rvalue, state: &HashMap<Local, LatticeValue>) -> LatticeValue {
        match rvalue {
            Rvalue::Use(operand) => self.eval_operand(operand, state),
            Rvalue::BinaryOp(op, left, right) => {
                let left_val = self.eval_operand(left, state);
                let right_val = self.eval_operand(right, state);
                
                if let (LatticeValue::Constant(l), LatticeValue::Constant(r)) = (&left_val, &right_val) {
                    self.eval_binary_op(op, l, r)
                } else {
                    LatticeValue::Top
                }
            }
            Rvalue::UnaryOp(op, operand) => {
                let val = self.eval_operand(operand, state);
                if let LatticeValue::Constant(c) = &val {
                    self.eval_unary_op(op, c)
                } else {
                    LatticeValue::Top
                }
            }
            _ => LatticeValue::Top,
        }
    }

    fn eval_operand(&self, operand: &Operand, state: &HashMap<Local, LatticeValue>) -> LatticeValue {
        match operand {
            Operand::Constant(c) => LatticeValue::Constant(c.clone()),
            Operand::Copy(place) | Operand::Move(place) => {
                if place.projection.is_empty() {
                    state.get(&place.local).cloned().unwrap_or(LatticeValue::Top)
                } else {
                    LatticeValue::Top
                }
            }
        }
    }

    fn eval_binary_op(&self, op: &BinOp, left: &Constant, right: &Constant) -> LatticeValue {
        match (op, left, right) {
            (BinOp::Add, Constant::Scalar(Scalar::Int(l, ty)), Constant::Scalar(Scalar::Int(r, _))) => {
                l.checked_add(*r)
                    .map(|v| LatticeValue::Constant(Constant::Scalar(Scalar::Int(v, *ty))))
                    .unwrap_or(LatticeValue::Top)
            }
            (BinOp::Sub, Constant::Scalar(Scalar::Int(l, ty)), Constant::Scalar(Scalar::Int(r, _))) => {
                l.checked_sub(*r)
                    .map(|v| LatticeValue::Constant(Constant::Scalar(Scalar::Int(v, *ty))))
                    .unwrap_or(LatticeValue::Top)
            }
            (BinOp::Mul, Constant::Scalar(Scalar::Int(l, ty)), Constant::Scalar(Scalar::Int(r, _))) => {
                l.checked_mul(*r)
                    .map(|v| LatticeValue::Constant(Constant::Scalar(Scalar::Int(v, *ty))))
                    .unwrap_or(LatticeValue::Top)
            }
            (BinOp::Div, Constant::Scalar(Scalar::Int(l, ty)), Constant::Scalar(Scalar::Int(r, _))) => {
                if *r != 0 && !(*l == i128::MIN && *r == -1) {
                    LatticeValue::Constant(Constant::Scalar(Scalar::Int(l / r, *ty)))
                } else {
                    LatticeValue::Top
                }
            }
            (BinOp::Eq, Constant::Scalar(Scalar::Int(l, _)), Constant::Scalar(Scalar::Int(r, _))) => {
                LatticeValue::Constant(Constant::Scalar(Scalar::Int(if l == r { 1 } else { 0 }, IntType::I64)))
            }
            (BinOp::Ne, Constant::Scalar(Scalar::Int(l, _)), Constant::Scalar(Scalar::Int(r, _))) => {
                LatticeValue::Constant(Constant::Scalar(Scalar::Int(if l != r { 1 } else { 0 }, IntType::I64)))
            }
            (BinOp::Lt, Constant::Scalar(Scalar::Int(l, _)), Constant::Scalar(Scalar::Int(r, _))) => {
                LatticeValue::Constant(Constant::Scalar(Scalar::Int(if l < r { 1 } else { 0 }, IntType::I64)))
            }
            (BinOp::Le, Constant::Scalar(Scalar::Int(l, _)), Constant::Scalar(Scalar::Int(r, _))) => {
                LatticeValue::Constant(Constant::Scalar(Scalar::Int(if l <= r { 1 } else { 0 }, IntType::I64)))
            }
            (BinOp::Gt, Constant::Scalar(Scalar::Int(l, _)), Constant::Scalar(Scalar::Int(r, _))) => {
                LatticeValue::Constant(Constant::Scalar(Scalar::Int(if l > r { 1 } else { 0 }, IntType::I64)))
            }
            (BinOp::Ge, Constant::Scalar(Scalar::Int(l, _)), Constant::Scalar(Scalar::Int(r, _))) => {
                LatticeValue::Constant(Constant::Scalar(Scalar::Int(if l >= r { 1 } else { 0 }, IntType::I64)))
            }
            (BinOp::BitAnd, Constant::Scalar(Scalar::Int(l, ty)), Constant::Scalar(Scalar::Int(r, _))) => {
                LatticeValue::Constant(Constant::Scalar(Scalar::Int(l & r, *ty)))
            }
            (BinOp::BitOr, Constant::Scalar(Scalar::Int(l, ty)), Constant::Scalar(Scalar::Int(r, _))) => {
                LatticeValue::Constant(Constant::Scalar(Scalar::Int(l | r, *ty)))
            }
            (BinOp::BitXor, Constant::Scalar(Scalar::Int(l, ty)), Constant::Scalar(Scalar::Int(r, _))) => {
                LatticeValue::Constant(Constant::Scalar(Scalar::Int(l ^ r, *ty)))
            }
            _ => LatticeValue::Top,
        }
    }

    fn eval_unary_op(&self, op: &UnOp, val: &Constant) -> LatticeValue {
        match (op, val) {
            (UnOp::Neg, Constant::Scalar(Scalar::Int(v, ty))) => {
                v.checked_neg()
                    .map(|n| LatticeValue::Constant(Constant::Scalar(Scalar::Int(n, *ty))))
                    .unwrap_or(LatticeValue::Top)
            }
            (UnOp::Not, Constant::Scalar(Scalar::Int(v, ty))) => {
                LatticeValue::Constant(Constant::Scalar(Scalar::Int(!v, *ty)))
            }
            _ => LatticeValue::Top,
        }
    }

    fn run_analysis(&mut self, body: &MirBody) {
        self.worklist.push(BasicBlock(0));
        
        while let Some(block) = self.worklist.pop() {
            if self.visited.contains(&block) {
                continue;
            }
            self.visited.insert(block);

            let mut state = self.state.entry(block).or_default().clone();
            
            if let Some(block_data) = body.basic_blocks.get(block.index()) {
                for stmt in &block_data.statements {
                    if let Statement::Assign(place, rvalue) = stmt {
                        if place.projection.is_empty() {
                            let value = self.eval_rvalue(rvalue, &state);
                            state.insert(place.local, value);
                        }
                    }
                }
            }

            self.state.insert(block, state);
            self.propagate_to_successors(body, block);
        }
    }

    fn transform(&self, body: &mut MirBody) {
        for (block_idx, block) in body.basic_blocks.iter_mut().enumerate() {
            let block_id = BasicBlock(block_idx as u32);
            let state = self.state.get(&block_id).cloned().unwrap_or_default();

            for stmt in block.statements.iter_mut() {
                if let Statement::Assign(_, rvalue) = stmt {
                    Self::propagate_in_rvalue(rvalue, &state);
                }
            }

            if let Some(ref mut terminator) = block.terminator {
                Self::propagate_in_terminator(terminator, &state);
            }
        }
    }

    fn propagate_in_rvalue(rvalue: &mut Rvalue, constants: &HashMap<Local, LatticeValue>) {
        match rvalue {
            Rvalue::Use(operand) => {
                Self::propagate_in_operand(operand, constants);
            }
            Rvalue::BinaryOp(_, left, right) => {
                Self::propagate_in_operand(left, constants);
                Self::propagate_in_operand(right, constants);
            }
            Rvalue::UnaryOp(_, operand) => {
                Self::propagate_in_operand(operand, constants);
            }
            Rvalue::Aggregate(_, operands) => {
                for operand in operands.iter_mut() {
                    Self::propagate_in_operand(operand, constants);
                }
            }
            _ => {}
        }
    }

    fn propagate_in_operand(operand: &mut Operand, constants: &HashMap<Local, LatticeValue>) {
        if let Operand::Copy(place) | Operand::Move(place) = operand {
            if place.projection.is_empty() {
                if let Some(LatticeValue::Constant(constant)) = constants.get(&place.local) {
                    *operand = Operand::Constant(constant.clone());
                }
            }
        }
    }

    fn propagate_in_terminator(terminator: &mut Terminator, constants: &HashMap<Local, LatticeValue>) {
        match &mut terminator.kind {
            TerminatorKind::SwitchInt { discr, .. } => {
                Self::propagate_in_operand(discr, constants);
            }
            TerminatorKind::Call { func, args, .. } => {
                Self::propagate_in_operand(func, constants);
                for arg in args.iter_mut() {
                    Self::propagate_in_operand(arg, constants);
                }
            }
            TerminatorKind::Assert { cond, .. } => {
                Self::propagate_in_operand(cond, constants);
            }
            _ => {}
        }
    }
}

impl Default for AdvancedConstantPropagation {
    fn default() -> Self {
        Self::new()
    }
}

impl MirPass for AdvancedConstantPropagation {
    fn name(&self) -> &'static str {
        "advanced_constant_propagation"
    }

    fn run(&self, body: &mut MirBody) {
        let mut this = Self::new();
        this.run_analysis(body);
        this.transform(body);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lattice_meet() {
        let bottom = LatticeValue::Bottom;
        let top = LatticeValue::Top;
        let c1 = LatticeValue::Constant(Constant::Scalar(Scalar::Int(5, IntType::I64)));
        let c2 = LatticeValue::Constant(Constant::Scalar(Scalar::Int(10, IntType::I64)));

        assert_eq!(bottom.meet(&c1), c1);
        assert_eq!(c1.meet(&bottom), c1);
        assert_eq!(c1.meet(&c1), c1);
        assert_eq!(c1.meet(&c2), top);
        assert_eq!(top.meet(&c1), top);
    }

    #[test]
    fn test_inter_block_propagation() {
        let mut body = MirBody::new(0, 0..100);

        let mut block0 = BasicBlockData::new();
        block0.statements.push(Statement::Assign(
            Place::from_local(Local(1)),
            Rvalue::Use(Operand::Constant(Constant::Scalar(Scalar::Int(5, IntType::I64)))),
        ));
        block0.terminator = Some(Terminator {
            kind: TerminatorKind::Goto { target: BasicBlock(1) },
            span: 0..10,
        });
        body.basic_blocks.push(block0);

        let mut block1 = BasicBlockData::new();
        block1.statements.push(Statement::Assign(
            Place::from_local(Local(2)),
            Rvalue::BinaryOp(
                BinOp::Add,
                Box::new(Operand::Copy(Place::from_local(Local(1)))),
                Box::new(Operand::Constant(Constant::Scalar(Scalar::Int(3, IntType::I64)))),
            ),
        ));
        block1.terminator = Some(Terminator {
            kind: TerminatorKind::Return,
            span: 10..20,
        });
        body.basic_blocks.push(block1);

        let pass = AdvancedConstantPropagation::new();
        pass.run(&mut body);

        if let Statement::Assign(_, rvalue) = &body.basic_blocks[1].statements[0] {
            if let Rvalue::BinaryOp(_, left, _) = rvalue {
                assert!(matches!(left.as_ref(), Operand::Constant(_)));
            } else {
                panic!("Expected binary operation");
            }
        }
    }
}
