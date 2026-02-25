//! Constant Propagation Optimization
//!
//! This pass propagates constant values to their uses.
//! For example:
//! ```
//! _1 = 5
//! _2 = _1 + 3  // becomes: _2 = 5 + 3
//! ```
//!
//! This works within basic blocks and doesn't handle control flow.

use crate::middle::mir::optimize::MirPass;
use crate::middle::mir::*;
use std::collections::HashMap;

/// Constant propagation optimization
#[derive(Debug, Clone, Copy)]
pub struct ConstantPropagation;

impl MirPass for ConstantPropagation {
    fn name(&self) -> &'static str {
        "constant_propagation"
    }

    fn run(&self, body: &mut MirBody) {
        for block in body.basic_blocks.iter_mut() {
            // Track constant values for each local in this block
            let mut constants: HashMap<Local, Constant> = HashMap::new();

            for stmt in block.statements.iter_mut() {
                match stmt {
                    Statement::Assign(place, rvalue) => {
                        // Propagate constants into the rvalue
                        propagate_in_rvalue(rvalue, &constants);

                        // If this assigns a constant, track it
                        if let Rvalue::Use(Operand::Constant(constant)) = rvalue {
                            if let Some(local) = place.as_local() {
                                constants.insert(local, constant.clone());
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Propagate into terminator
            if let Some(ref mut terminator) = block.terminator {
                propagate_in_terminator(terminator, &constants);
            }
        }
    }
}

/// Propagate constants into an rvalue
fn propagate_in_rvalue(rvalue: &mut Rvalue, constants: &HashMap<Local, Constant>) {
    match rvalue {
        Rvalue::Use(operand) => {
            propagate_in_operand(operand, constants);
        }
        Rvalue::BinaryOp(_, left, right) => {
            propagate_in_operand(left, constants);
            propagate_in_operand(right, constants);
        }
        Rvalue::UnaryOp(_, operand) => {
            propagate_in_operand(operand, constants);
        }
        Rvalue::Aggregate(_, operands) => {
            for operand in operands.iter_mut() {
                propagate_in_operand(operand, constants);
            }
        }
        _ => {}
    }
}

/// Propagate constants into an operand
fn propagate_in_operand(operand: &mut Operand, constants: &HashMap<Local, Constant>) {
    if let Operand::Copy(place) | Operand::Move(place) = operand {
        if let Some(local) = place.as_local() {
            if let Some(constant) = constants.get(&local) {
                *operand = Operand::Constant(constant.clone());
            }
        }
    }
}

/// Propagate constants into a terminator
fn propagate_in_terminator(terminator: &mut Terminator, constants: &HashMap<Local, Constant>) {
    match &mut terminator.kind {
        TerminatorKind::SwitchInt { discr, .. } => {
            propagate_in_operand(discr, constants);
        }
        TerminatorKind::Call { func, args, .. } => {
            propagate_in_operand(func, constants);
            for arg in args.iter_mut() {
                propagate_in_operand(arg, constants);
            }
        }
        _ => {}
    }
}

/// Extension trait for Place
trait PlaceExt {
    fn as_local(&self) -> Option<Local>;
}

impl PlaceExt for Place {
    fn as_local(&self) -> Option<Local> {
        if self.projection.is_empty() {
            Some(self.local)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_body() -> MirBody {
        let mut body = MirBody::new(0, 0..100);

        // Create a basic block with:
        // _1 = 5
        // _2 = _1 + 3
        let mut block = BasicBlockData::new();

        // _1 = 5
        block.statements.push(Statement::Assign(
            Place::from_local(Local(1)),
            Rvalue::Use(Operand::Constant(Constant::Scalar(Scalar::Int(
                5,
                IntType::I64,
            )))),
        ));

        // _2 = _1 + 3
        block.statements.push(Statement::Assign(
            Place::from_local(Local(2)),
            Rvalue::BinaryOp(
                BinOp::Add,
                Box::new(Operand::Copy(Place::from_local(Local(1)))),
                Box::new(Operand::Constant(Constant::Scalar(Scalar::Int(
                    3,
                    IntType::I64,
                )))),
            ),
        ));

        body.basic_blocks.push(block);
        body
    }

    #[test]
    fn test_constant_propagation() {
        let mut body = create_test_body();

        // Run constant propagation
        let pass = ConstantPropagation;
        pass.run(&mut body);

        // Check that _1 was propagated into the second statement
        let second_stmt = &body.basic_blocks[0].statements[1];
        if let Statement::Assign(_, rvalue) = second_stmt {
            if let Rvalue::BinaryOp(_, left, _) = rvalue {
                // The left operand should now be a constant
                assert!(matches!(left.as_ref(), Operand::Constant(_)));
            } else {
                panic!("Expected binary operation");
            }
        } else {
            panic!("Expected assignment");
        }
    }

    #[test]
    fn test_no_propagation_for_non_constants() {
        let mut body = MirBody::new(0, 0..100);
        let mut block = BasicBlockData::new();

        // _1 = _0 (not a constant)
        block.statements.push(Statement::Assign(
            Place::from_local(Local(1)),
            Rvalue::Use(Operand::Copy(Place::from_local(Local(0)))),
        ));

        // _2 = _1 + 3
        block.statements.push(Statement::Assign(
            Place::from_local(Local(2)),
            Rvalue::BinaryOp(
                BinOp::Add,
                Box::new(Operand::Copy(Place::from_local(Local(1)))),
                Box::new(Operand::Constant(Constant::Scalar(Scalar::Int(
                    3,
                    IntType::I64,
                )))),
            ),
        ));

        body.basic_blocks.push(block);

        // Run constant propagation
        let pass = ConstantPropagation;
        pass.run(&mut body);

        // Check that _1 was NOT propagated (it's not a constant)
        let second_stmt = &body.basic_blocks[0].statements[1];
        if let Statement::Assign(_, rvalue) = second_stmt {
            if let Rvalue::BinaryOp(_, left, _) = rvalue {
                // The left operand should still be a copy
                assert!(matches!(left.as_ref(), Operand::Copy(_)));
            } else {
                panic!("Expected binary operation");
            }
        } else {
            panic!("Expected assignment");
        }
    }
}
