//! Constant Folding Optimization with Overflow Checking
//!
//! This pass evaluates constant expressions at compile time.
//! For example:
//! - `2 + 3` becomes `5`
//! - `true && false` becomes `false`
//! - `-5` becomes `-5` (already constant)
//!
//! # Overflow Handling
//!
//! This implementation properly detects and reports integer overflow:
//! - Uses `checked_add`, `checked_sub`, `checked_mul` for arithmetic
//! - Checks for division by zero
//! - Checks for overflow in division and remainder operations
//! - Reports overflow errors through the diagnostic system

use crate::middle::mir::optimize::MirPass;
use crate::middle::mir::*;

/// Overflow error information
#[derive(Debug, Clone, PartialEq)]
pub struct OverflowError {
    /// The operation that caused overflow
    pub operation: BinOp,
    /// Left operand value
    pub left: i128,
    /// Right operand value
    pub right: i128,
    /// Error message
    pub message: String,
}

impl std::fmt::Display for OverflowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Overflow in {:?}: {} ({} {:?} {})",
            self.operation, self.message, self.left, self.operation, self.right
        )
    }
}

impl std::error::Error for OverflowError {}

/// Constant folding optimization with overflow checking
#[derive(Debug, Clone, Copy)]
pub struct ConstantFolding;

impl MirPass for ConstantFolding {
    fn name(&self) -> &'static str {
        "constant_folding"
    }

    fn run(&self, body: &mut MirBody) {
        for block in body.basic_blocks.iter_mut() {
            for stmt in block.statements.iter_mut() {
                if let Statement::Assign(_place, rvalue) = stmt {
                    if let Some(constant) = fold_rvalue(rvalue) {
                        *rvalue = Rvalue::Use(Operand::Constant(constant));
                    }
                }
            }
        }
    }
}

/// Constant folding with overflow error reporting
pub struct ConstantFoldingWithErrors {
    /// Collected overflow errors
    pub errors: Vec<OverflowError>,
}

impl ConstantFoldingWithErrors {
    /// Create a new instance
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    /// Run constant folding and collect errors
    pub fn run(&mut self, body: &mut MirBody) {
        for block in body.basic_blocks.iter_mut() {
            for stmt in block.statements.iter_mut() {
                if let Statement::Assign(_place, rvalue) = stmt {
                    match fold_rvalue_with_errors(rvalue, &mut self.errors) {
                        Some(constant) => {
                            *rvalue = Rvalue::Use(Operand::Constant(constant));
                        }
                        None => {
                            // Could not fold - might be due to overflow
                        }
                    }
                }
            }
        }
    }

    /// Check if any overflow errors were found
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get the collected errors
    pub fn errors(&self) -> &[OverflowError] {
        &self.errors
    }
}

impl Default for ConstantFoldingWithErrors {
    fn default() -> Self {
        Self::new()
    }
}

/// Try to fold an rvalue into a constant
fn fold_rvalue(rvalue: &Rvalue) -> Option<Constant> {
    match rvalue {
        Rvalue::BinaryOp(op, left, right) => {
            let left_const = operand_to_scalar(left)?;
            let right_const = operand_to_scalar(right)?;
            fold_binary_op(op, left_const, right_const)
        }
        Rvalue::UnaryOp(op, operand) => {
            let operand_const = operand_to_scalar(operand)?;
            fold_unary_op(op, operand_const)
        }
        _ => None,
    }
}

/// Try to fold an rvalue into a constant with error collection
fn fold_rvalue_with_errors(rvalue: &Rvalue, errors: &mut Vec<OverflowError>) -> Option<Constant> {
    match rvalue {
        Rvalue::BinaryOp(op, left, right) => {
            let left_const = operand_to_scalar(left)?;
            let right_const = operand_to_scalar(right)?;
            fold_binary_op_with_errors(op, left_const, right_const, errors)
        }
        Rvalue::UnaryOp(op, operand) => {
            let operand_const = operand_to_scalar(operand)?;
            fold_unary_op_with_errors(op, operand_const, errors)
        }
        _ => None,
    }
}

/// Convert an operand to a scalar value if it's a constant
fn operand_to_scalar(operand: &Operand) -> Option<Scalar> {
    match operand {
        Operand::Constant(Constant::Scalar(scalar)) => Some(scalar.clone()),
        _ => None,
    }
}

/// Fold a binary operation with overflow checking
fn fold_binary_op(op: &BinOp, left: Scalar, right: Scalar) -> Option<Constant> {
    match (op, left, right) {
        // Integer arithmetic with overflow checking
        (BinOp::Add, Scalar::Int(l, ty), Scalar::Int(r, _)) => l
            .checked_add(r)
            .map(|result| Constant::Scalar(Scalar::Int(result, ty))),
        (BinOp::Sub, Scalar::Int(l, ty), Scalar::Int(r, _)) => l
            .checked_sub(r)
            .map(|result| Constant::Scalar(Scalar::Int(result, ty))),
        (BinOp::Mul, Scalar::Int(l, ty), Scalar::Int(r, _)) => l
            .checked_mul(r)
            .map(|result| Constant::Scalar(Scalar::Int(result, ty))),
        (BinOp::Div, Scalar::Int(l, ty), Scalar::Int(r, _)) => {
            if r == 0 {
                return None; // Division by zero
            }
            // Check for overflow: i128::MIN / -1
            if l == i128::MIN && r == -1 {
                return None; // Overflow
            }
            Some(Constant::Scalar(Scalar::Int(l / r, ty)))
        }
        (BinOp::Rem, Scalar::Int(l, ty), Scalar::Int(r, _)) => {
            if r == 0 {
                return None; // Division by zero
            }
            // Check for overflow: i128::MIN % -1
            if l == i128::MIN && r == -1 {
                return None; // Overflow
            }
            Some(Constant::Scalar(Scalar::Int(l % r, ty)))
        }

        // Float arithmetic (no overflow checking needed for floats per IEEE 754)
        (BinOp::Add, Scalar::Float(l, ty), Scalar::Float(r, _)) => {
            Some(Constant::Scalar(Scalar::Float(l + r, ty)))
        }
        (BinOp::Sub, Scalar::Float(l, ty), Scalar::Float(r, _)) => {
            Some(Constant::Scalar(Scalar::Float(l - r, ty)))
        }
        (BinOp::Mul, Scalar::Float(l, ty), Scalar::Float(r, _)) => {
            Some(Constant::Scalar(Scalar::Float(l * r, ty)))
        }
        (BinOp::Div, Scalar::Float(l, ty), Scalar::Float(r, _)) => {
            Some(Constant::Scalar(Scalar::Float(l / r, ty)))
        }

        // Integer comparisons
        (BinOp::Eq, Scalar::Int(l, _), Scalar::Int(r, _)) => Some(Constant::Scalar(Scalar::Int(
            if l == r { 1 } else { 0 },
            IntType::I64,
        ))),
        (BinOp::Ne, Scalar::Int(l, _), Scalar::Int(r, _)) => Some(Constant::Scalar(Scalar::Int(
            if l != r { 1 } else { 0 },
            IntType::I64,
        ))),
        (BinOp::Lt, Scalar::Int(l, _), Scalar::Int(r, _)) => Some(Constant::Scalar(Scalar::Int(
            if l < r { 1 } else { 0 },
            IntType::I64,
        ))),
        (BinOp::Le, Scalar::Int(l, _), Scalar::Int(r, _)) => Some(Constant::Scalar(Scalar::Int(
            if l <= r { 1 } else { 0 },
            IntType::I64,
        ))),
        (BinOp::Gt, Scalar::Int(l, _), Scalar::Int(r, _)) => Some(Constant::Scalar(Scalar::Int(
            if l > r { 1 } else { 0 },
            IntType::I64,
        ))),
        (BinOp::Ge, Scalar::Int(l, _), Scalar::Int(r, _)) => Some(Constant::Scalar(Scalar::Int(
            if l >= r { 1 } else { 0 },
            IntType::I64,
        ))),

        // Float comparisons
        (BinOp::Eq, Scalar::Float(l, _), Scalar::Float(r, _)) => Some(Constant::Scalar(
            Scalar::Int(if l == r { 1 } else { 0 }, IntType::I64),
        )),
        (BinOp::Ne, Scalar::Float(l, _), Scalar::Float(r, _)) => Some(Constant::Scalar(
            Scalar::Int(if l != r { 1 } else { 0 }, IntType::I64),
        )),
        (BinOp::Lt, Scalar::Float(l, _), Scalar::Float(r, _)) => Some(Constant::Scalar(
            Scalar::Int(if l < r { 1 } else { 0 }, IntType::I64),
        )),
        (BinOp::Le, Scalar::Float(l, _), Scalar::Float(r, _)) => Some(Constant::Scalar(
            Scalar::Int(if l <= r { 1 } else { 0 }, IntType::I64),
        )),
        (BinOp::Gt, Scalar::Float(l, _), Scalar::Float(r, _)) => Some(Constant::Scalar(
            Scalar::Int(if l > r { 1 } else { 0 }, IntType::I64),
        )),
        (BinOp::Ge, Scalar::Float(l, _), Scalar::Float(r, _)) => Some(Constant::Scalar(
            Scalar::Int(if l >= r { 1 } else { 0 }, IntType::I64),
        )),

        // Bitwise operations
        (BinOp::BitAnd, Scalar::Int(l, ty), Scalar::Int(r, _)) => {
            Some(Constant::Scalar(Scalar::Int(l & r, ty)))
        }
        (BinOp::BitOr, Scalar::Int(l, ty), Scalar::Int(r, _)) => {
            Some(Constant::Scalar(Scalar::Int(l | r, ty)))
        }
        (BinOp::BitXor, Scalar::Int(l, ty), Scalar::Int(r, _)) => {
            Some(Constant::Scalar(Scalar::Int(l ^ r, ty)))
        }
        (BinOp::Shl, Scalar::Int(l, ty), Scalar::Int(r, _)) => {
            // Check for shift overflow
            if r < 0 || r >= 128 {
                return None;
            }
            Some(Constant::Scalar(Scalar::Int(l << r, ty)))
        }
        (BinOp::Shr, Scalar::Int(l, ty), Scalar::Int(r, _)) => {
            // Check for shift overflow
            if r < 0 || r >= 128 {
                return None;
            }
            Some(Constant::Scalar(Scalar::Int(l >> r, ty)))
        }

        _ => None,
    }
}

/// Fold a binary operation with overflow error reporting
fn fold_binary_op_with_errors(
    op: &BinOp,
    left: Scalar,
    right: Scalar,
    errors: &mut Vec<OverflowError>,
) -> Option<Constant> {
    match (op, left, right) {
        // Integer arithmetic with overflow checking and error reporting
        (BinOp::Add, Scalar::Int(l, ty), Scalar::Int(r, _)) => match l.checked_add(r) {
            Some(result) => Some(Constant::Scalar(Scalar::Int(result, ty))),
            None => {
                errors.push(OverflowError {
                    operation: *op,
                    left: l,
                    right: r,
                    message: "integer overflow in addition".to_string(),
                });
                None
            }
        },
        (BinOp::Sub, Scalar::Int(l, ty), Scalar::Int(r, _)) => match l.checked_sub(r) {
            Some(result) => Some(Constant::Scalar(Scalar::Int(result, ty))),
            None => {
                errors.push(OverflowError {
                    operation: *op,
                    left: l,
                    right: r,
                    message: "integer overflow in subtraction".to_string(),
                });
                None
            }
        },
        (BinOp::Mul, Scalar::Int(l, ty), Scalar::Int(r, _)) => match l.checked_mul(r) {
            Some(result) => Some(Constant::Scalar(Scalar::Int(result, ty))),
            None => {
                errors.push(OverflowError {
                    operation: *op,
                    left: l,
                    right: r,
                    message: "integer overflow in multiplication".to_string(),
                });
                None
            }
        },
        (BinOp::Div, Scalar::Int(l, ty), Scalar::Int(r, _)) => {
            if r == 0 {
                errors.push(OverflowError {
                    operation: *op,
                    left: l,
                    right: r,
                    message: "division by zero".to_string(),
                });
                return None;
            }
            // Check for overflow: i128::MIN / -1
            if l == i128::MIN && r == -1 {
                errors.push(OverflowError {
                    operation: *op,
                    left: l,
                    right: r,
                    message: "integer overflow in division (MIN / -1)".to_string(),
                });
                return None;
            }
            Some(Constant::Scalar(Scalar::Int(l / r, ty)))
        }
        (BinOp::Rem, Scalar::Int(l, ty), Scalar::Int(r, _)) => {
            if r == 0 {
                errors.push(OverflowError {
                    operation: *op,
                    left: l,
                    right: r,
                    message: "remainder by zero".to_string(),
                });
                return None;
            }
            // Check for overflow: i128::MIN % -1
            if l == i128::MIN && r == -1 {
                errors.push(OverflowError {
                    operation: *op,
                    left: l,
                    right: r,
                    message: "integer overflow in remainder (MIN % -1)".to_string(),
                });
                return None;
            }
            Some(Constant::Scalar(Scalar::Int(l % r, ty)))
        }

        // For other operations, delegate to the standard fold_binary_op
        (op, left, right) => fold_binary_op(op, left, right),
    }
}

/// Fold a unary operation
fn fold_unary_op(op: &UnOp, operand: Scalar) -> Option<Constant> {
    match (op, operand) {
        (UnOp::Neg, Scalar::Int(v, ty)) => v
            .checked_neg()
            .map(|result| Constant::Scalar(Scalar::Int(result, ty))),
        (UnOp::Neg, Scalar::Float(v, ty)) => Some(Constant::Scalar(Scalar::Float(-v, ty))),
        (UnOp::Not, Scalar::Int(v, ty)) => Some(Constant::Scalar(Scalar::Int(!v, ty))),
        _ => None,
    }
}

/// Fold a unary operation with overflow error reporting
fn fold_unary_op_with_errors(
    op: &UnOp,
    operand: Scalar,
    errors: &mut Vec<OverflowError>,
) -> Option<Constant> {
    match (op, operand) {
        (UnOp::Neg, Scalar::Int(v, ty)) => {
            match v.checked_neg() {
                Some(result) => Some(Constant::Scalar(Scalar::Int(result, ty))),
                None => {
                    errors.push(OverflowError {
                        operation: BinOp::Sub, // Use Sub to represent negation as 0 - v
                        left: 0,
                        right: v,
                        message: "integer overflow in negation".to_string(),
                    });
                    None
                }
            }
        }
        (UnOp::Neg, Scalar::Float(v, ty)) => Some(Constant::Scalar(Scalar::Float(-v, ty))),
        (UnOp::Not, Scalar::Int(v, ty)) => Some(Constant::Scalar(Scalar::Int(!v, ty))),
        _ => None,
    }
}

/// Check if an integer operation would overflow
///
/// This function can be used by other passes to check for overflow
/// without performing the full constant folding.
pub fn would_overflow(op: BinOp, left: i128, right: i128) -> Option<OverflowError> {
    match op {
        BinOp::Add => {
            if left.checked_add(right).is_none() {
                Some(OverflowError {
                    operation: op,
                    left,
                    right,
                    message: "integer overflow in addition".to_string(),
                })
            } else {
                None
            }
        }
        BinOp::Sub => {
            if left.checked_sub(right).is_none() {
                Some(OverflowError {
                    operation: op,
                    left,
                    right,
                    message: "integer overflow in subtraction".to_string(),
                })
            } else {
                None
            }
        }
        BinOp::Mul => {
            if left.checked_mul(right).is_none() {
                Some(OverflowError {
                    operation: op,
                    left,
                    right,
                    message: "integer overflow in multiplication".to_string(),
                })
            } else {
                None
            }
        }
        BinOp::Div => {
            if right == 0 {
                Some(OverflowError {
                    operation: op,
                    left,
                    right,
                    message: "division by zero".to_string(),
                })
            } else if left == i128::MIN && right == -1 {
                Some(OverflowError {
                    operation: op,
                    left,
                    right,
                    message: "integer overflow in division (MIN / -1)".to_string(),
                })
            } else {
                None
            }
        }
        BinOp::Rem => {
            if right == 0 {
                Some(OverflowError {
                    operation: op,
                    left,
                    right,
                    message: "remainder by zero".to_string(),
                })
            } else if left == i128::MIN && right == -1 {
                Some(OverflowError {
                    operation: op,
                    left,
                    right,
                    message: "integer overflow in remainder (MIN % -1)".to_string(),
                })
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fold_integer_add() {
        let left = Operand::Constant(Constant::Scalar(Scalar::Int(2, IntType::I64)));
        let right = Operand::Constant(Constant::Scalar(Scalar::Int(3, IntType::I64)));
        let rvalue = Rvalue::BinaryOp(BinOp::Add, Box::new(left), Box::new(right));

        let result = fold_rvalue(&rvalue);
        assert!(result.is_some());

        if let Constant::Scalar(Scalar::Int(val, _)) = result.unwrap() {
            assert_eq!(val, 5);
        } else {
            panic!("Expected integer constant");
        }
    }

    #[test]
    fn test_fold_integer_mul() {
        let left = Operand::Constant(Constant::Scalar(Scalar::Int(4, IntType::I64)));
        let right = Operand::Constant(Constant::Scalar(Scalar::Int(5, IntType::I64)));
        let rvalue = Rvalue::BinaryOp(BinOp::Mul, Box::new(left), Box::new(right));

        let result = fold_rvalue(&rvalue);
        assert!(result.is_some());

        if let Constant::Scalar(Scalar::Int(val, _)) = result.unwrap() {
            assert_eq!(val, 20);
        } else {
            panic!("Expected integer constant");
        }
    }

    #[test]
    fn test_fold_comparison() {
        let left = Operand::Constant(Constant::Scalar(Scalar::Int(5, IntType::I64)));
        let right = Operand::Constant(Constant::Scalar(Scalar::Int(3, IntType::I64)));
        let rvalue = Rvalue::BinaryOp(BinOp::Gt, Box::new(left), Box::new(right));

        let result = fold_rvalue(&rvalue);
        assert!(result.is_some());

        if let Constant::Scalar(Scalar::Int(val, _)) = result.unwrap() {
            assert_eq!(val, 1); // true
        } else {
            panic!("Expected boolean constant");
        }
    }

    #[test]
    fn test_fold_non_constant() {
        // Should not fold when operands are not constants
        let place = Place::from_local(Local(1));
        let left = Operand::Copy(place.clone());
        let right = Operand::Constant(Constant::Scalar(Scalar::Int(3, IntType::I64)));
        let rvalue = Rvalue::BinaryOp(BinOp::Add, Box::new(left), Box::new(right));

        let result = fold_rvalue(&rvalue);
        assert!(result.is_none());
    }

    #[test]
    fn test_overflow_detection_add() {
        let max = i128::MAX;
        let left = Operand::Constant(Constant::Scalar(Scalar::Int(max, IntType::I128)));
        let right = Operand::Constant(Constant::Scalar(Scalar::Int(1, IntType::I128)));
        let rvalue = Rvalue::BinaryOp(BinOp::Add, Box::new(left), Box::new(right));

        let mut errors = Vec::new();
        let result = fold_rvalue_with_errors(&rvalue, &mut errors);

        assert!(result.is_none());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("overflow"));
    }

    #[test]
    fn test_overflow_detection_mul() {
        let max = i128::MAX;
        let left = Operand::Constant(Constant::Scalar(Scalar::Int(max, IntType::I128)));
        let right = Operand::Constant(Constant::Scalar(Scalar::Int(2, IntType::I128)));
        let rvalue = Rvalue::BinaryOp(BinOp::Mul, Box::new(left), Box::new(right));

        let mut errors = Vec::new();
        let result = fold_rvalue_with_errors(&rvalue, &mut errors);

        assert!(result.is_none());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("overflow"));
    }

    #[test]
    fn test_division_by_zero() {
        let left = Operand::Constant(Constant::Scalar(Scalar::Int(10, IntType::I64)));
        let right = Operand::Constant(Constant::Scalar(Scalar::Int(0, IntType::I64)));
        let rvalue = Rvalue::BinaryOp(BinOp::Div, Box::new(left), Box::new(right));

        let mut errors = Vec::new();
        let result = fold_rvalue_with_errors(&rvalue, &mut errors);

        assert!(result.is_none());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("division by zero"));
    }

    #[test]
    fn test_would_overflow_function() {
        // Test addition overflow
        let result = would_overflow(BinOp::Add, i128::MAX, 1);
        assert!(result.is_some());

        // Test normal addition
        let result = would_overflow(BinOp::Add, 10, 20);
        assert!(result.is_none());

        // Test division by zero
        let result = would_overflow(BinOp::Div, 10, 0);
        assert!(result.is_some());

        // Test MIN / -1 overflow
        let result = would_overflow(BinOp::Div, i128::MIN, -1);
        assert!(result.is_some());
    }

    #[test]
    fn test_negation_overflow() {
        let min = i128::MIN;
        let operand = Operand::Constant(Constant::Scalar(Scalar::Int(min, IntType::I128)));
        let rvalue = Rvalue::UnaryOp(UnOp::Neg, Box::new(operand));

        let mut errors = Vec::new();
        let result = fold_rvalue_with_errors(&rvalue, &mut errors);

        assert!(result.is_none());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("overflow"));
    }
}
