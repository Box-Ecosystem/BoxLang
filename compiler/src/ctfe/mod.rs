//! Compile-time function evaluation (CTFE) for BoxLang
//!
//! CTFE allows evaluating functions at compile time, enabling
//! constant generics and compile-time computation.

use crate::ast::{BinaryOp, Expr, Literal, Type, UnaryOp};
use std::collections::HashMap;

/// Value representation for CTFE
#[derive(Debug, Clone, PartialEq)]
pub enum ConstValue {
    /// Unit value
    Unit,
    /// Boolean
    Bool(bool),
    /// Integer
    I64(i64),
    /// Unsigned integer
    U64(u64),
    /// Float
    F64(f64),
    /// String
    String(String),
    /// Character
    Char(char),
    /// Array
    Array(Vec<ConstValue>),
    /// Tuple
    Tuple(Vec<ConstValue>),
    /// Error during evaluation
    Error(String),
}

impl ConstValue {
    /// Convert to boolean if possible
    pub fn to_bool(&self) -> Option<bool> {
        match self {
            ConstValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Convert to i64 if possible
    pub fn to_i64(&self) -> Option<i64> {
        match self {
            ConstValue::I64(n) => Some(*n),
            _ => None,
        }
    }

    /// Convert to u64 if possible
    pub fn to_u64(&self) -> Option<u64> {
        match self {
            ConstValue::U64(n) => Some(*n),
            _ => None,
        }
    }

    /// Convert to f64 if possible
    pub fn to_f64(&self) -> Option<f64> {
        match self {
            ConstValue::F64(f) => Some(*f),
            ConstValue::I64(n) => Some(*n as f64),
            ConstValue::U64(n) => Some(*n as f64),
            _ => None,
        }
    }

    /// Get the type of this value
    pub fn get_type(&self) -> Type {
        fn make_path(name: &str) -> crate::ast::Path {
            crate::ast::Path {
                segments: vec![crate::ast::PathSegment {
                    ident: crate::ast::Ident::new(name),
                    generics: vec![],
                }],
            }
        }

        match self {
            ConstValue::Unit => Type::Unit,
            ConstValue::Bool(_) => Type::Path(make_path("bool")),
            ConstValue::I64(_) => Type::Path(make_path("i64")),
            ConstValue::U64(_) => Type::Path(make_path("u64")),
            ConstValue::F64(_) => Type::Path(make_path("f64")),
            ConstValue::String(_) => Type::Path(make_path("String")),
            ConstValue::Char(_) => Type::Path(make_path("char")),
            ConstValue::Array(vals) => {
                let elem_ty = vals.first().map(|v| v.get_type()).unwrap_or(Type::Unit);
                Type::Array(Box::new(elem_ty), Some(vals.len()))
            }
            ConstValue::Tuple(vals) => Type::Tuple(vals.iter().map(|v| v.get_type()).collect()),
            ConstValue::Error(_) => Type::Never,
        }
    }

    /// Check if this value is an error
    pub fn is_error(&self) -> bool {
        matches!(self, ConstValue::Error(_))
    }
}

impl std::fmt::Display for ConstValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConstValue::Unit => write!(f, "()"),
            ConstValue::Bool(b) => write!(f, "{}", b),
            ConstValue::I64(n) => write!(f, "{}", n),
            ConstValue::U64(n) => write!(f, "{}", n),
            ConstValue::F64(n) => write!(f, "{}", n),
            ConstValue::String(s) => write!(f, "\"{}\"", s),
            ConstValue::Char(c) => write!(f, "'{}'", c),
            ConstValue::Array(vals) => {
                write!(f, "[")?;
                for (i, val) in vals.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", val)?;
                }
                write!(f, "]")
            }
            ConstValue::Tuple(vals) => {
                write!(f, "(")?;
                for (i, val) in vals.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", val)?;
                }
                write!(f, ")")
            }
            ConstValue::Error(msg) => write!(f, "error: {}", msg),
        }
    }
}

/// Constant evaluator
pub struct ConstEvaluator {
    /// Maximum number of steps before timeout
    max_steps: usize,
    /// Current step count
    steps: usize,
    /// Variable bindings
    bindings: HashMap<String, ConstValue>,
}

impl ConstEvaluator {
    /// Create a new constant evaluator
    pub fn new() -> Self {
        Self {
            max_steps: 10000,
            steps: 0,
            bindings: HashMap::new(),
        }
    }

    /// Evaluate an expression to a constant value
    pub fn eval(&mut self, expr: &Expr) -> ConstValue {
        if self.steps >= self.max_steps {
            return ConstValue::Error("evaluation step limit exceeded".to_string());
        }
        self.steps += 1;

        match expr {
            Expr::Literal(lit) => self.eval_literal(lit),
            Expr::Binary(binary) => self.eval_binary(binary),
            Expr::Unary(unary) => self.eval_unary(unary),
            Expr::Ident(ident) => self.eval_ident(ident),
            Expr::ArrayInit(array_init) => self.eval_array_init(&array_init.elements),
            Expr::TupleInit(elems) => self.eval_tuple_init(elems),
            Expr::Block(block) => self.eval_block(block),
            Expr::If(if_expr) => self.eval_if(if_expr),
            Expr::Match(match_expr) => self.eval_match(match_expr),
            Expr::Call(call) => self.eval_call(call),
            _ => ConstValue::Error(format!("unsupported expression in const context")),
        }
    }

    /// Evaluate a literal
    fn eval_literal(&self, lit: &Literal) -> ConstValue {
        match lit {
            Literal::Integer(n) => ConstValue::I64(*n),
            Literal::Float(f) => ConstValue::F64(*f),
            Literal::String(s) => ConstValue::String(s.to_string()),
            Literal::Bool(b) => ConstValue::Bool(*b),
            Literal::Char(c) => ConstValue::Char(*c),
            Literal::Null => ConstValue::Unit,
        }
    }

    /// Evaluate a binary expression
    fn eval_binary(&mut self, binary: &crate::ast::BinaryExpr) -> ConstValue {
        let left = self.eval(&binary.left);
        let right = self.eval(&binary.right);

        if left.is_error() {
            return left;
        }
        if right.is_error() {
            return right;
        }

        match binary.op {
            BinaryOp::Add => self.eval_add(&left, &right),
            BinaryOp::Sub => self.eval_sub(&left, &right),
            BinaryOp::Mul => self.eval_mul(&left, &right),
            BinaryOp::Div => self.eval_div(&left, &right),
            BinaryOp::Rem => self.eval_rem(&left, &right),
            BinaryOp::LogicalAnd => self.eval_logical_and(&left, &right),
            BinaryOp::LogicalOr => self.eval_logical_or(&left, &right),
            BinaryOp::Eq => self.eval_eq(&left, &right),
            BinaryOp::Ne => self.eval_ne(&left, &right),
            BinaryOp::Lt => self.eval_lt(&left, &right),
            BinaryOp::Le => self.eval_le(&left, &right),
            BinaryOp::Gt => self.eval_gt(&left, &right),
            BinaryOp::Ge => self.eval_ge(&left, &right),
            BinaryOp::And => self.eval_bitand(&left, &right),
            BinaryOp::Or => self.eval_bitor(&left, &right),
            BinaryOp::Xor => self.eval_bitxor(&left, &right),
            BinaryOp::Shl => self.eval_shl(&left, &right),
            BinaryOp::Shr => self.eval_shr(&left, &right),
            _ => ConstValue::Error(format!("unsupported binary operator")),
        }
    }

    /// Evaluate addition
    fn eval_add(&self, left: &ConstValue, right: &ConstValue) -> ConstValue {
        match (left, right) {
            (ConstValue::I64(a), ConstValue::I64(b)) => ConstValue::I64(a + b),
            (ConstValue::U64(a), ConstValue::U64(b)) => ConstValue::U64(a + b),
            (ConstValue::F64(a), ConstValue::F64(b)) => ConstValue::F64(a + b),
            _ => ConstValue::Error("cannot add these types".to_string()),
        }
    }

    /// Evaluate subtraction
    fn eval_sub(&self, left: &ConstValue, right: &ConstValue) -> ConstValue {
        match (left, right) {
            (ConstValue::I64(a), ConstValue::I64(b)) => ConstValue::I64(a - b),
            (ConstValue::U64(a), ConstValue::U64(b)) => ConstValue::U64(a - b),
            (ConstValue::F64(a), ConstValue::F64(b)) => ConstValue::F64(a - b),
            _ => ConstValue::Error("cannot subtract these types".to_string()),
        }
    }

    /// Evaluate multiplication
    fn eval_mul(&self, left: &ConstValue, right: &ConstValue) -> ConstValue {
        match (left, right) {
            (ConstValue::I64(a), ConstValue::I64(b)) => ConstValue::I64(a * b),
            (ConstValue::U64(a), ConstValue::U64(b)) => ConstValue::U64(a * b),
            (ConstValue::F64(a), ConstValue::F64(b)) => ConstValue::F64(a * b),
            _ => ConstValue::Error("cannot multiply these types".to_string()),
        }
    }

    /// Evaluate division
    fn eval_div(&self, left: &ConstValue, right: &ConstValue) -> ConstValue {
        match (left, right) {
            (ConstValue::I64(a), ConstValue::I64(b)) => {
                if *b == 0 {
                    ConstValue::Error("division by zero".to_string())
                } else {
                    ConstValue::I64(a / b)
                }
            }
            (ConstValue::U64(a), ConstValue::U64(b)) => {
                if *b == 0 {
                    ConstValue::Error("division by zero".to_string())
                } else {
                    ConstValue::U64(a / b)
                }
            }
            (ConstValue::F64(a), ConstValue::F64(b)) => ConstValue::F64(a / b),
            _ => ConstValue::Error("cannot divide these types".to_string()),
        }
    }

    /// Evaluate remainder
    fn eval_rem(&self, left: &ConstValue, right: &ConstValue) -> ConstValue {
        match (left, right) {
            (ConstValue::I64(a), ConstValue::I64(b)) => ConstValue::I64(a % b),
            (ConstValue::U64(a), ConstValue::U64(b)) => ConstValue::U64(a % b),
            _ => ConstValue::Error("cannot compute remainder of these types".to_string()),
        }
    }

    /// Evaluate logical and
    fn eval_logical_and(&self, left: &ConstValue, right: &ConstValue) -> ConstValue {
        match (left, right) {
            (ConstValue::Bool(a), ConstValue::Bool(b)) => ConstValue::Bool(*a && *b),
            _ => ConstValue::Error("cannot apply 'and' to non-boolean types".to_string()),
        }
    }

    /// Evaluate logical or
    fn eval_logical_or(&self, left: &ConstValue, right: &ConstValue) -> ConstValue {
        match (left, right) {
            (ConstValue::Bool(a), ConstValue::Bool(b)) => ConstValue::Bool(*a || *b),
            _ => ConstValue::Error("cannot apply 'or' to non-boolean types".to_string()),
        }
    }

    /// Evaluate equality
    fn eval_eq(&self, left: &ConstValue, right: &ConstValue) -> ConstValue {
        ConstValue::Bool(left == right)
    }

    /// Evaluate inequality
    fn eval_ne(&self, left: &ConstValue, right: &ConstValue) -> ConstValue {
        ConstValue::Bool(left != right)
    }

    /// Evaluate less than
    fn eval_lt(&self, left: &ConstValue, right: &ConstValue) -> ConstValue {
        match (left.to_i64(), right.to_i64()) {
            (Some(a), Some(b)) => ConstValue::Bool(a < b),
            _ => ConstValue::Error("cannot compare these types".to_string()),
        }
    }

    /// Evaluate less than or equal
    fn eval_le(&self, left: &ConstValue, right: &ConstValue) -> ConstValue {
        match (left.to_i64(), right.to_i64()) {
            (Some(a), Some(b)) => ConstValue::Bool(a <= b),
            _ => ConstValue::Error("cannot compare these types".to_string()),
        }
    }

    /// Evaluate greater than
    fn eval_gt(&self, left: &ConstValue, right: &ConstValue) -> ConstValue {
        match (left.to_i64(), right.to_i64()) {
            (Some(a), Some(b)) => ConstValue::Bool(a > b),
            _ => ConstValue::Error("cannot compare these types".to_string()),
        }
    }

    /// Evaluate greater than or equal
    fn eval_ge(&self, left: &ConstValue, right: &ConstValue) -> ConstValue {
        match (left.to_i64(), right.to_i64()) {
            (Some(a), Some(b)) => ConstValue::Bool(a >= b),
            _ => ConstValue::Error("cannot compare these types".to_string()),
        }
    }

    /// Evaluate bitwise and
    fn eval_bitand(&self, left: &ConstValue, right: &ConstValue) -> ConstValue {
        match (left.to_i64(), right.to_i64()) {
            (Some(a), Some(b)) => ConstValue::I64(a & b),
            _ => ConstValue::Error("cannot apply bitwise and to these types".to_string()),
        }
    }

    /// Evaluate bitwise or
    fn eval_bitor(&self, left: &ConstValue, right: &ConstValue) -> ConstValue {
        match (left.to_i64(), right.to_i64()) {
            (Some(a), Some(b)) => ConstValue::I64(a | b),
            _ => ConstValue::Error("cannot apply bitwise or to these types".to_string()),
        }
    }

    /// Evaluate bitwise xor
    fn eval_bitxor(&self, left: &ConstValue, right: &ConstValue) -> ConstValue {
        match (left.to_i64(), right.to_i64()) {
            (Some(a), Some(b)) => ConstValue::I64(a ^ b),
            _ => ConstValue::Error("cannot apply bitwise xor to these types".to_string()),
        }
    }

    /// Evaluate left shift
    fn eval_shl(&self, left: &ConstValue, right: &ConstValue) -> ConstValue {
        match (left.to_i64(), right.to_i64()) {
            (Some(a), Some(b)) => ConstValue::I64(a << b),
            _ => ConstValue::Error("cannot shift these types".to_string()),
        }
    }

    /// Evaluate right shift
    fn eval_shr(&self, left: &ConstValue, right: &ConstValue) -> ConstValue {
        match (left.to_i64(), right.to_i64()) {
            (Some(a), Some(b)) => ConstValue::I64(a >> b),
            _ => ConstValue::Error("cannot shift these types".to_string()),
        }
    }

    /// Evaluate unary expression
    fn eval_unary(&mut self, unary: &crate::ast::UnaryExpr) -> ConstValue {
        let operand = self.eval(&unary.expr);

        if operand.is_error() {
            return operand;
        }

        match unary.op {
            UnaryOp::Neg => self.eval_neg(&operand),
            UnaryOp::Not => self.eval_not(&operand),
            _ => ConstValue::Error("unsupported unary operation in const context".to_string()),
        }
    }

    /// Evaluate negation
    fn eval_neg(&self, operand: &ConstValue) -> ConstValue {
        match operand {
            ConstValue::I64(n) => ConstValue::I64(-n),
            ConstValue::F64(f) => ConstValue::F64(-f),
            _ => ConstValue::Error("cannot negate this type".to_string()),
        }
    }

    /// Evaluate logical not
    fn eval_not(&self, operand: &ConstValue) -> ConstValue {
        match operand {
            ConstValue::Bool(b) => ConstValue::Bool(!b),
            _ => ConstValue::Error("cannot apply 'not' to non-boolean type".to_string()),
        }
    }

    /// Evaluate identifier
    fn eval_ident(&self, ident: &crate::ast::Ident) -> ConstValue {
        if let Some(value) = self.bindings.get(&ident.to_string()) {
            return value.clone();
        }
        ConstValue::Error(format!("unknown constant: {}", ident))
    }

    /// Evaluate array initialization
    fn eval_array_init(&mut self, elems: &[Expr]) -> ConstValue {
        let mut values = Vec::new();
        for elem in elems {
            let val = self.eval(elem);
            if val.is_error() {
                return val;
            }
            values.push(val);
        }
        ConstValue::Array(values)
    }

    /// Evaluate tuple initialization
    fn eval_tuple_init(&mut self, elems: &[Expr]) -> ConstValue {
        let mut values = Vec::new();
        for elem in elems {
            let val = self.eval(elem);
            if val.is_error() {
                return val;
            }
            values.push(val);
        }
        ConstValue::Tuple(values)
    }

    /// Evaluate a block
    fn eval_block(&mut self, block: &crate::ast::Block) -> ConstValue {
        let mut result = ConstValue::Unit;
        for stmt in &block.stmts {
            match stmt {
                crate::ast::Stmt::Let(let_stmt) => {
                    if let Some(ref init) = let_stmt.init {
                        let value = self.eval(init);
                        if value.is_error() {
                            return value;
                        }
                        self.bindings.insert(let_stmt.name.to_string(), value);
                    }
                }
                crate::ast::Stmt::Expr(expr) => {
                    result = self.eval(expr);
                    if result.is_error() {
                        return result;
                    }
                }
                _ => {}
            }
        }
        result
    }

    /// Evaluate an if expression
    fn eval_if(&mut self, if_expr: &crate::ast::IfExpr) -> ConstValue {
        let cond = self.eval(&if_expr.cond);
        match cond {
            ConstValue::Bool(true) => self.eval_block(&if_expr.then_branch),
            ConstValue::Bool(false) => {
                if let Some(ref else_branch) = if_expr.else_branch {
                    self.eval(else_branch)
                } else {
                    ConstValue::Unit
                }
            }
            _ => ConstValue::Error("if condition must be boolean".to_string()),
        }
    }

    /// Evaluate a match expression
    fn eval_match(&mut self, match_expr: &crate::ast::MatchExpr) -> ConstValue {
        let scrutinee = self.eval(&match_expr.expr);
        if scrutinee.is_error() {
            return scrutinee;
        }

        for arm in &match_expr.arms {
            // Simplified pattern matching - just check guards
            if let Some(ref guard) = arm.guard {
                let guard_val = self.eval(guard);
                if let ConstValue::Bool(true) = guard_val {
                    return self.eval(&arm.body);
                }
            } else {
                // For now, match first arm without guard
                return self.eval(&arm.body);
            }
        }

        ConstValue::Error("no matching arm in match expression".to_string())
    }

    /// Evaluate a function call
    fn eval_call(&mut self, call: &crate::ast::CallExpr) -> ConstValue {
        // For now, only support built-in const functions
        if let Expr::Ident(ref func_name) = call.func.as_ref() {
            return self.eval_builtin(&func_name.to_string(), &call.args);
        }
        ConstValue::Error("only built-in functions supported in const context".to_string())
    }

    /// Evaluate built-in functions
    fn eval_builtin(&mut self, name: &str, args: &[Expr]) -> ConstValue {
        match name {
            "abs" => {
                if args.len() != 1 {
                    return ConstValue::Error("abs takes exactly one argument".to_string());
                }
                let val = self.eval(&args[0]);
                match val {
                    ConstValue::I64(n) => ConstValue::I64(n.abs()),
                    _ => ConstValue::Error("abs requires numeric argument".to_string()),
                }
            }
            "max" => {
                if args.len() != 2 {
                    return ConstValue::Error("max takes exactly two arguments".to_string());
                }
                let a = self.eval(&args[0]);
                let b = self.eval(&args[1]);
                match (a.to_i64(), b.to_i64()) {
                    (Some(x), Some(y)) => ConstValue::I64(if x > y { x } else { y }),
                    _ => ConstValue::Error("max requires numeric arguments".to_string()),
                }
            }
            "min" => {
                if args.len() != 2 {
                    return ConstValue::Error("min takes exactly two arguments".to_string());
                }
                let a = self.eval(&args[0]);
                let b = self.eval(&args[1]);
                match (a.to_i64(), b.to_i64()) {
                    (Some(x), Some(y)) => ConstValue::I64(if x < y { x } else { y }),
                    _ => ConstValue::Error("min requires numeric arguments".to_string()),
                }
            }
            _ => ConstValue::Error(format!("unknown const function: {}", name)),
        }
    }
}

impl Default for ConstEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_const_value_display() {
        assert_eq!(ConstValue::I64(42).to_string(), "42");
        assert_eq!(ConstValue::Bool(true).to_string(), "true");
        assert_eq!(
            ConstValue::String("hello".to_string()).to_string(),
            "\"hello\""
        );
    }

    #[test]
    fn test_eval_literal() {
        let mut eval = ConstEvaluator::new();
        let expr = Expr::Literal(Literal::Integer(42));
        assert_eq!(eval.eval(&expr), ConstValue::I64(42));
    }

    #[test]
    fn test_eval_binary() {
        let mut eval = ConstEvaluator::new();

        // Test addition
        let expr = Expr::Binary(crate::ast::BinaryExpr {
            op: BinaryOp::Add,
            left: Box::new(Expr::Literal(Literal::Integer(10))),
            right: Box::new(Expr::Literal(Literal::Integer(32))),
        });
        assert_eq!(eval.eval(&expr), ConstValue::I64(42));

        // Test multiplication
        let expr = Expr::Binary(crate::ast::BinaryExpr {
            op: BinaryOp::Mul,
            left: Box::new(Expr::Literal(Literal::Integer(6))),
            right: Box::new(Expr::Literal(Literal::Integer(7))),
        });
        assert_eq!(eval.eval(&expr), ConstValue::I64(42));
    }

    #[test]
    fn test_eval_comparison() {
        let mut eval = ConstEvaluator::new();

        let expr = Expr::Binary(crate::ast::BinaryExpr {
            op: BinaryOp::Lt,
            left: Box::new(Expr::Literal(Literal::Integer(10))),
            right: Box::new(Expr::Literal(Literal::Integer(20))),
        });
        assert_eq!(eval.eval(&expr), ConstValue::Bool(true));
    }
}
