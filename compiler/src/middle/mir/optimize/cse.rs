//! Common Subexpression Elimination (CSE)
//!
//! This pass identifies and eliminates redundant computations.
//! For example:
//! ```
//! _1 = a + b
//! _2 = a + b  // This is redundant, can be replaced with _2 = _1
//! _3 = a + b  // This is also redundant
//! ```
//!
//! Becomes:
//! ```
//! _1 = a + b
//! _2 = _1
//! _3 = _1
//! ```

use crate::middle::mir::optimize::MirPass;
use crate::middle::mir::*;
use std::collections::HashMap;

/// Expression key for CSE - a simplified representation that can be compared
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExprKey {
    BinaryOp {
        op: BinOp,
        left: OperandKey,
        right: OperandKey,
    },
    UnaryOp {
        op: UnOp,
        operand: OperandKey,
    },
    Cast {
        kind: CastKind,
        operand: OperandKey,
        ty_hash: u64,
    },
    Aggregate {
        kind: AggregateKey,
        operands: Vec<OperandKey>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OperandKey {
    Local(u32),
    Constant(i128, u8),
    ConstantFloat(u64),
    ConstantPtr(u64),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AggregateKey {
    Array,
    Tuple,
    Struct(String),
    Enum(String, String),
}

impl OperandKey {
    fn from_operand(operand: &Operand) -> Self {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                if place.projection.is_empty() {
                    OperandKey::Local(place.local.0)
                } else {
                    OperandKey::Unknown
                }
            }
            Operand::Constant(Constant::Scalar(Scalar::Int(v, _ty))) => {
                OperandKey::Constant(*v, 0)
            }
            Operand::Constant(Constant::Scalar(Scalar::Float(v, _ty))) => {
                OperandKey::ConstantFloat(v.to_bits())
            }
            Operand::Constant(Constant::Scalar(Scalar::Pointer(v))) => {
                OperandKey::ConstantPtr(*v)
            }
            Operand::Constant(Constant::ZST) => OperandKey::Constant(0, 0),
        }
    }
}

impl ExprKey {
    fn from_rvalue(rvalue: &Rvalue) -> Option<ExprKey> {
        match rvalue {
            Rvalue::BinaryOp(op, left, right) => {
                let left_key = OperandKey::from_operand(left);
                let right_key = OperandKey::from_operand(right);
                
                if matches!(left_key, OperandKey::Unknown) || matches!(right_key, OperandKey::Unknown) {
                    return None;
                }
                
                Some(ExprKey::BinaryOp {
                    op: *op,
                    left: left_key,
                    right: right_key,
                })
            }
            Rvalue::UnaryOp(op, operand) => {
                let operand_key = OperandKey::from_operand(operand);
                
                if matches!(operand_key, OperandKey::Unknown) {
                    return None;
                }
                
                Some(ExprKey::UnaryOp {
                    op: *op,
                    operand: operand_key,
                })
            }
            Rvalue::Cast(kind, operand, _ty) => {
                let operand_key = OperandKey::from_operand(operand);
                
                if matches!(operand_key, OperandKey::Unknown) {
                    return None;
                }
                
                Some(ExprKey::Cast {
                    kind: *kind,
                    operand: operand_key,
                    ty_hash: 0,
                })
            }
            Rvalue::Aggregate(kind, operands) => {
                let agg_key = match kind {
                    AggregateKind::Array(_) => AggregateKey::Array,
                    AggregateKind::Tuple => AggregateKey::Tuple,
                    AggregateKind::Struct(name) => AggregateKey::Struct(name.as_str().to_string()),
                    AggregateKind::Enum(enum_name, variant_name) => {
                        AggregateKey::Enum(enum_name.as_str().to_string(), variant_name.as_str().to_string())
                    }
                    AggregateKind::Closure(_) => return None,
                };
                
                let operand_keys: Vec<OperandKey> = operands
                    .iter()
                    .map(OperandKey::from_operand)
                    .collect();
                
                if operand_keys.iter().any(|k| matches!(k, OperandKey::Unknown)) {
                    return None;
                }
                
                Some(ExprKey::Aggregate {
                    kind: agg_key,
                    operands: operand_keys,
                })
            }
            _ => None,
        }
    }
}

/// Common Subexpression Elimination pass
#[derive(Debug, Default)]
pub struct CommonSubexpressionElimination {
    /// Map from expression key to the local that holds the result
    available: HashMap<ExprKey, Local>,
    /// Number of expressions eliminated
    eliminated_count: usize,
}

impl CommonSubexpressionElimination {
    pub fn new() -> Self {
        Self {
            available: HashMap::new(),
            eliminated_count: 0,
        }
    }

    pub fn eliminated_count(&self) -> usize {
        self.eliminated_count
    }

    fn invalidate_local(&mut self, local: Local) {
        self.available.retain(|_key, &mut result_local| {
            result_local != local
        });
    }
}

impl MirPass for CommonSubexpressionElimination {
    fn name(&self) -> &'static str {
        "common_subexpression_elimination"
    }

    fn run(&self, body: &mut MirBody) {
        let mut this = Self::new();
        
        for block in body.basic_blocks.iter_mut() {
            this.available.clear();
            
            for stmt in block.statements.iter_mut() {
                if let Statement::Assign(place, rvalue) = stmt {
                    if place.projection.is_empty() {
                        if let Some(expr_key) = ExprKey::from_rvalue(rvalue) {
                            if let Some(&existing_local) = this.available.get(&expr_key) {
                                *rvalue = Rvalue::Use(Operand::Copy(Place::from_local(existing_local)));
                                this.eliminated_count += 1;
                            } else {
                                this.available.insert(expr_key, place.local);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cse_basic() {
        let mut body = MirBody::new(0, 0..100);

        let mut block = BasicBlockData::new();

        block.statements.push(Statement::Assign(
            Place::from_local(Local(1)),
            Rvalue::BinaryOp(
                BinOp::Add,
                Box::new(Operand::Copy(Place::from_local(Local(0)))),
                Box::new(Operand::Constant(Constant::Scalar(Scalar::Int(5, IntType::I64)))),
            ),
        ));

        block.statements.push(Statement::Assign(
            Place::from_local(Local(2)),
            Rvalue::BinaryOp(
                BinOp::Add,
                Box::new(Operand::Copy(Place::from_local(Local(0)))),
                Box::new(Operand::Constant(Constant::Scalar(Scalar::Int(5, IntType::I64)))),
            ),
        ));

        block.terminator = Some(Terminator {
            kind: TerminatorKind::Return,
            span: 0..10,
        });

        body.basic_blocks.push(block);

        let pass = CommonSubexpressionElimination::new();
        pass.run(&mut body);

        if let Statement::Assign(_, rvalue) = &body.basic_blocks[0].statements[1] {
            assert!(matches!(rvalue, Rvalue::Use(Operand::Copy(_))));
        } else {
            panic!("Expected assignment");
        }
    }

    #[test]
    fn test_operand_key_creation() {
        let local_op = Operand::Copy(Place::from_local(Local(5)));
        assert_eq!(OperandKey::from_operand(&local_op), OperandKey::Local(5));

        let const_op = Operand::Constant(Constant::Scalar(Scalar::Int(42, IntType::I64)));
        assert_eq!(OperandKey::from_operand(&const_op), OperandKey::Constant(42, 0));
    }

    #[test]
    fn test_expr_key_creation() {
        let rvalue = Rvalue::BinaryOp(
            BinOp::Add,
            Box::new(Operand::Copy(Place::from_local(Local(1)))),
            Box::new(Operand::Constant(Constant::Scalar(Scalar::Int(5, IntType::I64)))),
        );

        let key = ExprKey::from_rvalue(&rvalue);
        assert!(key.is_some());

        let key = key.unwrap();
        if let ExprKey::BinaryOp { op, left, right } = key {
            assert_eq!(op, BinOp::Add);
            assert_eq!(left, OperandKey::Local(1));
            assert_eq!(right, OperandKey::Constant(5, 0));
        } else {
            panic!("Expected BinaryOp");
        }
    }
}
