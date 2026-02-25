//! Dead Code Elimination Optimization
//!
//! This pass removes statements that have no side effects and
//! whose results are never used.
//!
//! For example:
//! ```
//! _1 = 5      // dead code - result never used
//! _2 = 10
//! return _2
//! ```
//!
//! Becomes:
//! ```
//! _2 = 10
//! return _2
//! ```

use crate::middle::mir::optimize::MirPass;
use crate::middle::mir::*;
use std::collections::HashSet;

/// Dead code elimination optimization
#[derive(Debug, Clone, Copy)]
pub struct DeadCodeElimination;

impl MirPass for DeadCodeElimination {
    fn name(&self) -> &'static str {
        "dead_code_elimination"
    }

    fn run(&self, body: &mut MirBody) {
        // Find all used locals
        let used_locals = find_used_locals(body);

        // Remove dead assignments
        for block in body.basic_blocks.iter_mut() {
            block.statements.retain(|stmt| {
                match stmt {
                    Statement::Assign(place, rvalue) => {
                        // Keep if:
                        // 1. The place is used later, or
                        // 2. The rvalue has side effects
                        let local = place.local;
                        used_locals.contains(&local) || has_side_effects(rvalue)
                    }
                    // Keep other statements (StorageLive, StorageDead, Nop)
                    _ => true,
                }
            });
        }
    }
}

/// Find all locals that are used (read from)
fn find_used_locals(body: &MirBody) -> HashSet<Local> {
    let mut used = HashSet::new();

    // Always mark return local and arguments as used
    used.insert(Local::RETURN_PLACE);
    for i in 0..body.arg_count {
        used.insert(Local((i + 1) as u32));
    }

    // Find all locals that are read
    for block in &body.basic_blocks {
        for stmt in &block.statements {
            if let Statement::Assign(_, rvalue) = stmt {
                find_used_in_rvalue(rvalue, &mut used);
            }
        }

        if let Some(ref terminator) = block.terminator {
            find_used_in_terminator(terminator, &mut used);
        }
    }

    used
}

/// Find locals used in an rvalue
fn find_used_in_rvalue(rvalue: &Rvalue, used: &mut HashSet<Local>) {
    match rvalue {
        Rvalue::Use(operand) => {
            find_used_in_operand(operand, used);
        }
        Rvalue::BinaryOp(_, left, right) => {
            find_used_in_operand(left, used);
            find_used_in_operand(right, used);
        }
        Rvalue::UnaryOp(_, operand) => {
            find_used_in_operand(operand, used);
        }
        Rvalue::Aggregate(_, operands) => {
            for operand in operands {
                find_used_in_operand(operand, used);
            }
        }
        _ => {}
    }
}

/// Find locals used in an operand
fn find_used_in_operand(operand: &Operand, used: &mut HashSet<Local>) {
    match operand {
        Operand::Copy(place) | Operand::Move(place) => {
            used.insert(place.local);
        }
        Operand::Constant(_) => {}
    }
}

/// Find locals used in a terminator
fn find_used_in_terminator(terminator: &Terminator, used: &mut HashSet<Local>) {
    match &terminator.kind {
        TerminatorKind::SwitchInt { discr, .. } => {
            find_used_in_operand(discr, used);
        }
        TerminatorKind::Call {
            func,
            args,
            destination,
            ..
        } => {
            find_used_in_operand(func, used);
            for arg in args {
                find_used_in_operand(arg, used);
            }
            // The destination local is written to, not read
            let _ = destination;
        }
        TerminatorKind::Return => {
            // Return uses the return place
            used.insert(Local::RETURN_PLACE);
        }
        _ => {}
    }
}

/// Check if an rvalue has side effects
fn has_side_effects(rvalue: &Rvalue) -> bool {
    match rvalue {
        // These have no side effects
        Rvalue::Use(_) => false,
        Rvalue::BinaryOp(_, _, _) => false,
        Rvalue::UnaryOp(_, _) => false,
        Rvalue::Aggregate(_, _) => false,
        Rvalue::Cast(..) => false,
        // These might have side effects (function calls, etc.)
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dead_code_elimination() {
        let mut body = MirBody::new(0, 0..100);

        let mut block = BasicBlockData::new();

        // _1 = 5 (dead code - never used)
        block.statements.push(Statement::Assign(
            Place::from_local(Local(1)),
            Rvalue::Use(Operand::Constant(Constant::Scalar(Scalar::Int(
                5,
                IntType::I64,
            )))),
        ));

        // _2 = 10 (used - assigned to return place)
        block.statements.push(Statement::Assign(
            Place::from_local(Local::RETURN_PLACE),
            Rvalue::Use(Operand::Constant(Constant::Scalar(Scalar::Int(
                10,
                IntType::I64,
            )))),
        ));

        // Return
        block.terminator = Some(Terminator {
            kind: TerminatorKind::Return,
            span: 0..10,
        });

        body.basic_blocks.push(block);

        // Run dead code elimination
        let pass = DeadCodeElimination;
        pass.run(&mut body);

        // The first statement should be removed (dead code)
        // The second statement should be kept (used for return)
        assert_eq!(body.basic_blocks[0].statements.len(), 1);

        // The remaining statement should assign to return place
        if let Statement::Assign(place, _) = &body.basic_blocks[0].statements[0] {
            assert_eq!(place.local, Local::RETURN_PLACE);
        } else {
            panic!("Expected assignment");
        }
    }

    #[test]
    fn test_keep_used_assignments() {
        let mut body = MirBody::new(0, 0..100);

        let mut block = BasicBlockData::new();

        // _1 = 5 (used in next statement)
        block.statements.push(Statement::Assign(
            Place::from_local(Local(1)),
            Rvalue::Use(Operand::Constant(Constant::Scalar(Scalar::Int(
                5,
                IntType::I64,
            )))),
        ));

        // _2 = _1 + 3 (uses _1, result used for return)
        block.statements.push(Statement::Assign(
            Place::from_local(Local::RETURN_PLACE),
            Rvalue::BinaryOp(
                BinOp::Add,
                Box::new(Operand::Copy(Place::from_local(Local(1)))),
                Box::new(Operand::Constant(Constant::Scalar(Scalar::Int(
                    3,
                    IntType::I64,
                )))),
            ),
        ));

        // Return
        block.terminator = Some(Terminator {
            kind: TerminatorKind::Return,
            span: 0..10,
        });

        body.basic_blocks.push(block);

        // Run dead code elimination
        let pass = DeadCodeElimination;
        pass.run(&mut body);

        // Both statements should be kept
        assert_eq!(body.basic_blocks[0].statements.len(), 2);
    }

    #[test]
    fn test_keep_side_effects() {
        let mut body = MirBody::new(0, 0..100);

        let mut block = BasicBlockData::new();

        // StorageLive has no side effects but is not an assignment
        block.statements.push(Statement::StorageLive(Local(1)));

        // Nop has no side effects
        block.statements.push(Statement::Nop);

        body.basic_blocks.push(block);

        // Run dead code elimination
        let pass = DeadCodeElimination;
        pass.run(&mut body);

        // Non-assignment statements should be kept
        assert_eq!(body.basic_blocks[0].statements.len(), 2);
    }
}
