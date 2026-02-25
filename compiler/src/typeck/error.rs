//! Type checking errors

use crate::ast::Span;
use thiserror::Error;

/// Type checking error
#[derive(Error, Debug, Clone, PartialEq)]
pub enum TypeError {
    #[error("mismatched types: expected `{expected}`, found `{found}`{}", 
        match (line, column) {
            (0, 0) => "".to_string(),
            (l, c) => format!(" at line {}, column {}", l, c),
        })]
    MismatchedTypes {
        expected: String,
        found: String,
        span: Span,
        line: usize,
        column: usize,
    },

    #[error("cannot infer type for `{name}`{}", 
        match (line, column) {
            (0, 0) => "".to_string(),
            (l, c) => format!(" at line {}, column {}", l, c),
        })]
    CannotInfer {
        name: String,
        span: Span,
        line: usize,
        column: usize,
    },

    #[error("undefined variable `{name}`{}", 
        match (line, column) {
            (0, 0) => "".to_string(),
            (l, c) => format!(" at line {}, column {}", l, c),
        })]
    UndefinedVar {
        name: String,
        span: Span,
        line: usize,
        column: usize,
    },

    #[error("undefined function `{name}`{}", 
        match (line, column) {
            (0, 0) => "".to_string(),
            (l, c) => format!(" at line {}, column {}", l, c),
        })]
    UndefinedFunction {
        name: String,
        span: Span,
        line: usize,
        column: usize,
    },

    #[error("undefined type `{name}`{}", 
        match (line, column) {
            (0, 0) => "".to_string(),
            (l, c) => format!(" at line {}, column {}", l, c),
        })]
    UndefinedType {
        name: String,
        span: Span,
        line: usize,
        column: usize,
    },

    #[error("invalid number of arguments: expected {expected}, found {found}{}", 
        match (line, column) {
            (0, 0) => "".to_string(),
            (l, c) => format!(" at line {}, column {}", l, c),
        })]
    ArgCountMismatch {
        expected: usize,
        found: usize,
        span: Span,
        line: usize,
        column: usize,
    },

    #[error("invalid binary operation: `{op}` between `{left}` and `{right}`{}", 
        match (line, column) {
            (0, 0) => "".to_string(),
            (l, c) => format!(" at line {}, column {}", l, c),
        })]
    InvalidBinaryOp {
        op: String,
        left: String,
        right: String,
        span: Span,
        line: usize,
        column: usize,
    },

    #[error("invalid unary operation: `{op}` on `{ty}`{}", 
        match (line, column) {
            (0, 0) => "".to_string(),
            (l, c) => format!(" at line {}, column {}", l, c),
        })]
    InvalidUnaryOp {
        op: String,
        ty: String,
        span: Span,
        line: usize,
        column: usize,
    },

    #[error("cannot assign to immutable variable `{name}`{}", 
        match (line, column) {
            (0, 0) => "".to_string(),
            (l, c) => format!(" at line {}, column {}", l, c),
        })]
    AssignToImmutable {
        name: String,
        span: Span,
        line: usize,
        column: usize,
    },

    #[error("missing return type annotation{}", 
        match (line, column) {
            (0, 0) => "".to_string(),
            (l, c) => format!(" at line {}, column {}", l, c),
        })]
    MissingReturnType {
        span: Span,
        line: usize,
        column: usize,
    },

    #[error("return type mismatch: expected `{expected}`, found `{found}`{}", 
        match (line, column) {
            (0, 0) => "".to_string(),
            (l, c) => format!(" at line {}, column {}", l, c),
        })]
    ReturnTypeMismatch {
        expected: String,
        found: String,
        span: Span,
        line: usize,
        column: usize,
    },

    #[error("dead code detected{}", 
        match (line, column) {
            (0, 0) => "".to_string(),
            (l, c) => format!(" at line {}, column {}", l, c),
        })]
    DeadCode {
        span: Span,
        line: usize,
        column: usize,
    },

    #[error("recursive type has infinite size{}", 
        match (line, column) {
            (0, 0) => "".to_string(),
            (l, c) => format!(" at line {}, column {}", l, c),
        })]
    RecursiveType {
        span: Span,
        line: usize,
        column: usize,
    },

    #[error("type annotation needed{}", 
        match (line, column) {
            (0, 0) => "".to_string(),
            (l, c) => format!(" at line {}, column {}", l, c),
        })]
    TypeAnnotationNeeded {
        span: Span,
        line: usize,
        column: usize,
    },

    #[error("undefined method `{method_name}` for type `{type_name}`{}", 
        match (line, column) {
            (0, 0) => "".to_string(),
            (l, c) => format!(" at line {}, column {}", l, c),
        })]
    UndefinedMethod {
        type_name: String,
        method_name: String,
        span: Span,
        line: usize,
        column: usize,
    },

    #[error("undefined field `{field_name}` for type `{type_name}`{}", 
        match (line, column) {
            (0, 0) => "".to_string(),
            (l, c) => format!(" at line {}, column {}", l, c),
        })]
    UndefinedField {
        type_name: String,
        field_name: String,
        span: Span,
        line: usize,
        column: usize,
    },

    #[error("invalid type `{name}`{}", 
        match (line, column) {
            (0, 0) => "".to_string(),
            (l, c) => format!(" at line {}, column {}", l, c),
        })]
    InvalidType {
        name: String,
        span: Span,
        line: usize,
        column: usize,
    },

    #[error("await used outside of async function{}", 
        match (line, column) {
            (0, 0) => "".to_string(),
            (l, c) => format!(" at line {}, column {}", l, c),
        })]
    AwaitOutsideAsync {
        span: Span,
        line: usize,
        column: usize,
    },

    #[error("value is not awaitable{}", 
        match (line, column) {
            (0, 0) => "".to_string(),
            (l, c) => format!(" at line {}, column {}", l, c),
        })]
    NotAwaitable {
        ty: String,
        span: Span,
        line: usize,
        column: usize,
    },

    #[error("multiple errors occurred")]
    Multiple(Vec<TypeError>),
}

impl TypeError {
    /// Get the span of this error
    pub fn span(&self) -> Option<&Span> {
        match self {
            TypeError::MismatchedTypes { span, .. } => Some(span),
            TypeError::CannotInfer { span, .. } => Some(span),
            TypeError::UndefinedVar { span, .. } => Some(span),
            TypeError::UndefinedFunction { span, .. } => Some(span),
            TypeError::UndefinedType { span, .. } => Some(span),
            TypeError::ArgCountMismatch { span, .. } => Some(span),
            TypeError::InvalidBinaryOp { span, .. } => Some(span),
            TypeError::InvalidUnaryOp { span, .. } => Some(span),
            TypeError::AssignToImmutable { span, .. } => Some(span),
            TypeError::MissingReturnType { span, .. } => Some(span),
            TypeError::ReturnTypeMismatch { span, .. } => Some(span),
            TypeError::DeadCode { span, .. } => Some(span),
            TypeError::RecursiveType { span, .. } => Some(span),
            TypeError::TypeAnnotationNeeded { span, .. } => Some(span),
            TypeError::UndefinedMethod { span, .. } => Some(span),
            TypeError::UndefinedField { span, .. } => Some(span),
            TypeError::InvalidType { span, .. } => Some(span),
            TypeError::AwaitOutsideAsync { span, .. } => Some(span),
            TypeError::NotAwaitable { span, .. } => Some(span),
            TypeError::Multiple(_) => None,
        }
    }

    /// Helper constructor for mismatched types
    pub fn mismatched_types(
        expected: String,
        found: String,
        span: Span,
        line: usize,
        column: usize,
    ) -> Self {
        TypeError::MismatchedTypes {
            expected,
            found,
            span,
            line,
            column,
        }
    }

    /// Helper constructor for undefined variable
    pub fn undefined_var(name: &str, span: Span, line: usize, column: usize) -> Self {
        TypeError::UndefinedVar {
            name: name.to_string(),
            span,
            line,
            column,
        }
    }

    /// Helper constructor for undefined function
    pub fn undefined_function(name: &str, span: Span, line: usize, column: usize) -> Self {
        TypeError::UndefinedFunction {
            name: name.to_string(),
            span,
            line,
            column,
        }
    }

    /// Helper constructor for invalid binary operation
    pub fn invalid_binary_op(
        op: String,
        left: String,
        right: String,
        span: Span,
        line: usize,
        column: usize,
    ) -> Self {
        TypeError::InvalidBinaryOp {
            op,
            left,
            right,
            span,
            line,
            column,
        }
    }

    /// Helper constructor for invalid unary operation
    pub fn invalid_unary_op(
        op: String,
        ty: String,
        span: Span,
        line: usize,
        column: usize,
    ) -> Self {
        TypeError::InvalidUnaryOp {
            op,
            ty,
            span,
            line,
            column,
        }
    }

    /// Helper constructor for argument count mismatch
    pub fn arg_count_mismatch(
        expected: usize,
        found: usize,
        span: Span,
        line: usize,
        column: usize,
    ) -> Self {
        TypeError::ArgCountMismatch {
            expected,
            found,
            span,
            line,
            column,
        }
    }

    /// Helper constructor for undefined method
    pub fn undefined_method(
        type_name: String,
        method_name: String,
        span: Span,
        line: usize,
        column: usize,
    ) -> Self {
        TypeError::UndefinedMethod {
            type_name,
            method_name,
            span,
            line,
            column,
        }
    }
}

/// A collection of type errors
#[derive(Debug, Clone, PartialEq)]
pub struct TypeErrors {
    errors: Vec<TypeError>,
}

impl TypeErrors {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn push(&mut self, error: TypeError) {
        self.errors.push(error);
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn len(&self) -> usize {
        self.errors.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &TypeError> {
        self.errors.iter()
    }

    pub fn into_result(self) -> TypeResult<()> {
        if self.errors.is_empty() {
            Ok(())
        } else if self.errors.len() == 1 {
            // SAFETY: We just checked that there's exactly one error
            // This is more efficient than creating a Multiple error with one element
            let mut errors = self.errors;
            let first = errors.pop().expect("len == 1 guarantees an element exists");
            Err(first)
        } else {
            Err(TypeError::Multiple(self.errors))
        }
    }

    /// Take the result without consuming self, clearing errors after
    pub fn take_result(&mut self) -> TypeResult<()> {
        if self.errors.is_empty() {
            Ok(())
        } else if self.errors.len() == 1 {
            let first = self.errors.pop().expect("len == 1 guarantees an element exists");
            Err(first)
        } else {
            let errors = std::mem::take(&mut self.errors);
            Err(TypeError::Multiple(errors))
        }
    }
}

impl Default for TypeErrors {
    fn default() -> Self {
        Self::new()
    }
}

/// Result type for type checking operations
pub type TypeResult<T> = Result<T, TypeError>;
