//! Mid-level Intermediate Representation (MIR) for BoxLang
//!
//! MIR is a simplified representation of the program that is used for:
//! - Borrow checking
//! - Optimization
//! - Code generation
//!
//! MIR is based on control flow graphs (CFG) with basic blocks.

use crate::ast::{BinaryOp, Ident, Span, Type, UnaryOp};
use std::fmt;

pub mod builder;
pub mod optimize;

/// Index for a local variable
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Local(pub u32);

impl Local {
    /// The return value local (always _0)
    pub const RETURN_PLACE: Local = Local(0);

    /// Create a new local from an index
    pub fn new(index: u32) -> Self {
        Local(index)
    }

    /// Get the index as usize
    pub fn index(&self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for Local {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "_{}", self.0)
    }
}

/// Index for a basic block
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BasicBlock(pub u32);

impl BasicBlock {
    pub fn new(index: u32) -> Self {
        BasicBlock(index)
    }

    pub fn index(&self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for BasicBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bb{}", self.0)
    }
}

/// A MIR function body
#[derive(Debug, Clone, PartialEq)]
pub struct MirBody {
    /// Basic blocks
    pub basic_blocks: Vec<BasicBlockData>,

    /// Local variable declarations
    pub local_decls: Vec<LocalDecl>,

    /// Number of function arguments
    pub arg_count: usize,

    /// Span of the function
    pub span: Span,
}

impl MirBody {
    /// Create a new empty MIR body
    pub fn new(arg_count: usize, span: Span) -> Self {
        Self {
            basic_blocks: Vec::new(),
            local_decls: Vec::new(),
            arg_count,
            span,
        }
    }

    /// Get the return local
    pub fn return_local(&self) -> Local {
        Local::RETURN_PLACE
    }

    /// Get the local declaration
    pub fn local_decl(&self, local: Local) -> &LocalDecl {
        &self.local_decls[local.index()]
    }

    /// Get a mutable reference to a basic block
    pub fn basic_block_mut(&mut self, block: BasicBlock) -> &mut BasicBlockData {
        &mut self.basic_blocks[block.index()]
    }

    /// Get a basic block
    pub fn basic_block(&self, block: BasicBlock) -> &BasicBlockData {
        &self.basic_blocks[block.index()]
    }

    /// Push a new basic block
    pub fn push_block(&mut self, block: BasicBlockData) -> BasicBlock {
        let index = self.basic_blocks.len() as u32;
        self.basic_blocks.push(block);
        BasicBlock(index)
    }

    /// Push a new local declaration
    pub fn push_local(&mut self, decl: LocalDecl) -> Local {
        let index = self.local_decls.len() as u32;
        self.local_decls.push(decl);
        Local(index)
    }
}

/// A basic block in the control flow graph
#[derive(Debug, Clone, PartialEq)]
pub struct BasicBlockData {
    /// Statements in the block
    pub statements: Vec<Statement>,

    /// The terminator (control flow instruction)
    pub terminator: Option<Terminator>,

    /// Is this block reachable?
    pub is_cleanup: bool,
}

impl BasicBlockData {
    /// Create a new empty basic block
    pub fn new() -> Self {
        Self {
            statements: Vec::new(),
            terminator: None,
            is_cleanup: false,
        }
    }

    /// Add a statement to the block
    pub fn push_stmt(&mut self, stmt: Statement) {
        self.statements.push(stmt);
    }

    /// Set the terminator
    pub fn set_terminator(&mut self, term: Terminator) {
        self.terminator = Some(term);
    }
}

impl Default for BasicBlockData {
    fn default() -> Self {
        Self::new()
    }
}

/// A statement in MIR
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// Assignment: place = rvalue
    Assign(Place, Rvalue),

    /// Mark a local as live (for borrow checking)
    StorageLive(Local),

    /// Mark a local as dead (for borrow checking)
    StorageDead(Local),

    /// Inline assembly
    InlineAsm(Box<InlineAsm>),

    /// No-op (used during optimization)
    Nop,
}

/// A terminator instruction (controls flow between basic blocks)
#[derive(Debug, Clone, PartialEq)]
pub struct Terminator {
    pub kind: TerminatorKind,
    pub span: Span,
}

/// Kinds of terminator instructions
#[derive(Debug, Clone, PartialEq)]
pub enum TerminatorKind {
    /// Go to another block
    Goto { target: BasicBlock },

    /// Conditional branch
    SwitchInt {
        /// The value to test
        discr: Operand,
        /// The type of the value
        switch_ty: Type,
        /// Targets for each value
        targets: Vec<(u128, BasicBlock)>,
        /// Default target
        otherwise: BasicBlock,
    },

    /// Return from function
    Return,

    /// Unconditional panic/abort
    Unwind,

    /// Call a function
    Call {
        /// Function to call
        func: Operand,
        /// Arguments
        args: Vec<Operand>,
        /// Where to store the return value
        destination: Place,
        /// Block to go to after the call
        target: Option<BasicBlock>,
    },

    /// Assert condition, panic if false
    Assert {
        /// Condition to check
        cond: Operand,
        /// Expected value
        expected: bool,
        /// Message
        msg: AssertMessage,
        /// Target if assertion passes
        target: BasicBlock,
        /// Target if assertion fails (panic)
        cleanup: Option<BasicBlock>,
    },
}

/// Assert message kinds
#[derive(Debug, Clone, PartialEq)]
pub enum AssertMessage {
    BoundsCheck { len: Operand, index: Operand },
    DivisionByZero,
    RemainderByZero,
    Overflow(BinaryOp),
}

/// A place (memory location)
#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    /// The local variable
    pub local: Local,
    /// Projections (field access, indexing, dereferencing)
    pub projection: Vec<PlaceElem>,
}

impl Place {
    /// Create a simple place from a local
    pub fn from_local(local: Local) -> Self {
        Self {
            local,
            projection: Vec::new(),
        }
    }

    /// Add a projection element
    pub fn project(mut self, elem: PlaceElem) -> Self {
        self.projection.push(elem);
        self
    }

    /// Is this a simple local (no projections)?
    pub fn is_local(&self) -> bool {
        self.projection.is_empty()
    }
}

/// A projection element (accessing part of a value)
#[derive(Debug, Clone, PartialEq)]
pub enum PlaceElem {
    /// Dereference: *place
    Deref,
    /// Field access: place.field
    Field(usize),
    /// Index: place[index]
    Index(Local),
    /// Constant index: place[constant]
    ConstantIndex {
        offset: u64,
        min_length: u64,
        from_end: bool,
    },
    /// Subslice: place[from..to]
    Subslice { from: u64, to: u64, from_end: bool },
}

/// A right-hand side value (can be computed)
#[derive(Debug, Clone, PartialEq)]
pub enum Rvalue {
    /// Use an operand
    Use(Operand),

    /// Copy a value
    Copy(Place),

    /// Move a value
    Move(Place),

    /// Reference: &place or &mut place
    Ref(Place, Mutability),

    /// Address of: &raw const place or &raw mut place
    AddressOf(Place, Mutability),

    /// Binary operation
    BinaryOp(BinOp, Box<Operand>, Box<Operand>),

    /// Unary operation
    UnaryOp(UnOp, Box<Operand>),

    /// Cast between types
    Cast(CastKind, Box<Operand>, Type),

    /// Array length
    Len(Place),

    /// Discriminant (for enums)
    Discriminant(Place),

    /// Aggregate value (array, tuple, struct)
    Aggregate(AggregateKind, Vec<Operand>),
}

/// Mutability
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mutability {
    Mut,
    Not,
}

/// Binary operations in MIR
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    BitXor,
    BitAnd,
    BitOr,
    Shl,
    Shr,
    Eq,
    Lt,
    Le,
    Ne,
    Ge,
    Gt,
    Offset,
    /// Logical AND (short-circuiting)
    And,
    /// Logical OR (short-circuiting)
    Or,
}

impl TryFrom<BinaryOp> for BinOp {
    type Error = String;

    fn try_from(op: BinaryOp) -> Result<Self, Self::Error> {
        match op {
            BinaryOp::Add => Ok(BinOp::Add),
            BinaryOp::Sub => Ok(BinOp::Sub),
            BinaryOp::Mul => Ok(BinOp::Mul),
            BinaryOp::Div => Ok(BinOp::Div),
            BinaryOp::Rem => Ok(BinOp::Rem),
            BinaryOp::And => Ok(BinOp::BitAnd),
            BinaryOp::Or => Ok(BinOp::BitOr),
            BinaryOp::Xor => Ok(BinOp::BitXor),
            BinaryOp::Shl => Ok(BinOp::Shl),
            BinaryOp::Shr => Ok(BinOp::Shr),
            BinaryOp::Eq => Ok(BinOp::Eq),
            BinaryOp::Ne => Ok(BinOp::Ne),
            BinaryOp::Lt => Ok(BinOp::Lt),
            BinaryOp::Le => Ok(BinOp::Le),
            BinaryOp::Gt => Ok(BinOp::Gt),
            BinaryOp::Ge => Ok(BinOp::Ge),
            BinaryOp::LogicalAnd => Ok(BinOp::And),
            BinaryOp::LogicalOr => Ok(BinOp::Or),
            _ => Err(format!("Unsupported binary op in MIR: {:?}", op)),
        }
    }
}

/// Unary operations in MIR
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnOp {
    Not,
    Neg,
}

impl TryFrom<UnaryOp> for UnOp {
    type Error = String;

    fn try_from(op: UnaryOp) -> Result<Self, Self::Error> {
        match op {
            UnaryOp::Not => Ok(UnOp::Not),
            UnaryOp::Neg => Ok(UnOp::Neg),
            _ => Err(format!("Unsupported unary op in MIR: {:?}", op)),
        }
    }
}

/// Cast kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CastKind {
    /// Numeric cast (int -> int, float -> float, etc.)
    Numeric,
    /// Pointer cast (*T -> *U)
    Pointer,
    /// Reference cast (&T -> &U)
    Reference,
    /// Unsize cast (T -> dyn Trait, etc.)
    Unsize,
}

/// Aggregate kinds
#[derive(Debug, Clone, PartialEq)]
pub enum AggregateKind {
    /// Array: [T; N]
    Array(Type),
    /// Tuple: (T1, T2, ...)
    Tuple,
    /// Struct: StructName { field1, field2, ... }
    Struct(Ident),
    /// Enum variant: EnumName::Variant
    Enum(Ident, Ident),
    /// Closure
    Closure(Ident),
}

/// An operand (argument to operations)
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    /// Copy a value from a place
    Copy(Place),
    /// Move a value from a place
    Move(Place),
    /// Constant value
    Constant(Constant),
}

/// A constant value
#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    /// Scalar value
    Scalar(Scalar),
    /// Zero-sized type
    ZST,
}

/// Scalar values
#[derive(Debug, Clone, PartialEq)]
pub enum Scalar {
    /// Integer
    Int(i128, IntType),
    /// Float
    Float(f64, FloatType),
    /// Pointer (address)
    Pointer(u64),
}

/// Integer types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntType {
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
}

/// Float types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloatType {
    F32,
    F64,
}

/// Local variable declaration
#[derive(Debug, Clone, PartialEq)]
pub struct LocalDecl {
    /// Type of the local
    pub ty: Type,
    /// Source span
    pub span: Span,
    /// Is this a function argument?
    pub is_arg: bool,
    /// Is this mutable?
    pub mutability: Mutability,
    /// Name (for debugging)
    pub name: Option<Ident>,
}

impl LocalDecl {
    /// Create a new local declaration
    pub fn new(ty: Type, span: Span) -> Self {
        Self {
            ty,
            span,
            is_arg: false,
            mutability: Mutability::Not,
            name: None,
        }
    }

    /// Mark as function argument
    pub fn arg(mut self) -> Self {
        self.is_arg = true;
        self
    }

    /// Mark as mutable
    pub fn mutable(mut self) -> Self {
        self.mutability = Mutability::Mut;
        self
    }

    /// Set name
    pub fn with_name(mut self, name: Ident) -> Self {
        self.name = Some(name);
        self
    }
}

/// Inline assembly
#[derive(Debug, Clone, PartialEq)]
pub struct InlineAsm {
    /// Assembly template
    pub template: String,
    /// Input operands
    pub inputs: Vec<InlineAsmOperand>,
    /// Output operands
    pub outputs: Vec<InlineAsmOperand>,
    /// Clobbers
    pub clobbers: Vec<String>,
    /// Options
    pub options: InlineAsmOptions,
}

/// Inline assembly operand
#[derive(Debug, Clone, PartialEq)]
pub struct InlineAsmOperand {
    pub constraint: String,
    pub value: Operand,
}

/// Inline assembly options
#[derive(Debug, Clone, PartialEq, Default)]
pub struct InlineAsmOptions {
    pub pure: bool,
    pub nomem: bool,
    pub readonly: bool,
    pub preserves_flags: bool,
    pub nostack: bool,
}

/// Pretty printing for MIR
pub mod pretty_print {
    use super::*;

    /// Print a MIR body
    pub fn print_mir(body: &MirBody, name: &str) -> String {
        let mut output = format!("fn {}(\n", name);

        // Print arguments
        for i in 0..body.arg_count {
            let local = Local(i as u32);
            let decl = body.local_decl(local);
            output.push_str(&format!("    {}: {},\n", local, format_type(&decl.ty)));
        }
        output.push_str(") -> ");
        output.push_str(&format_type(&body.local_decl(Local::RETURN_PLACE).ty));
        output.push_str(" {\n");

        // Print locals
        for (i, decl) in body.local_decls.iter().enumerate().skip(body.arg_count + 1) {
            output.push_str(&format!(
                "    let {}: {};\n",
                Local(i as u32),
                format_type(&decl.ty)
            ));
        }

        // Print basic blocks
        for (i, block) in body.basic_blocks.iter().enumerate() {
            output.push_str(&format!("\nbb{}: {{\n", i));

            for stmt in &block.statements {
                output.push_str(&format!("        {};\n", format_statement(stmt)));
            }

            if let Some(ref term) = block.terminator {
                output.push_str(&format!("        {}\n", format_terminator(term)));
            }

            output.push_str("    }\n");
        }

        output.push_str("}\n");
        output
    }

    fn format_type(ty: &Type) -> String {
        format!("{:?}", ty)
    }

    fn format_statement(stmt: &Statement) -> String {
        match stmt {
            Statement::Assign(place, rvalue) => {
                format!("{} = {}", format_place(place), format_rvalue(rvalue))
            }
            Statement::StorageLive(local) => format!("StorageLive({})", local),
            Statement::StorageDead(local) => format!("StorageDead({})", local),
            Statement::InlineAsm(_) => "InlineAsm(...)".to_string(),
            Statement::Nop => "nop".to_string(),
        }
    }

    fn format_terminator(term: &Terminator) -> String {
        match &term.kind {
            TerminatorKind::Goto { target } => format!("goto -> bb{}", target.0),
            TerminatorKind::Return => "return".to_string(),
            TerminatorKind::Unwind => "unwind".to_string(),
            TerminatorKind::Call {
                func,
                args,
                destination,
                target,
            } => {
                let args_str = args
                    .iter()
                    .map(format_operand)
                    .collect::<Vec<_>>()
                    .join(", ");
                let target_str = target.map(|t| format!(" -> bb{}", t.0)).unwrap_or_default();
                format!(
                    "{} = {}({}){}",
                    format_place(destination),
                    format_operand(func),
                    args_str,
                    target_str
                )
            }
            _ => format!("{:?}", term.kind),
        }
    }

    fn format_place(place: &Place) -> String {
        let mut result = format!("{}", place.local);
        for elem in &place.projection {
            match elem {
                PlaceElem::Deref => result = format!("(*{})", result),
                PlaceElem::Field(i) => result = format!("{}.{}", result, i),
                PlaceElem::Index(l) => result = format!("{}[{}]", result, l),
                _ => result = format!("{}[?]", result),
            }
        }
        result
    }

    fn format_rvalue(rvalue: &Rvalue) -> String {
        match rvalue {
            Rvalue::Use(op) => format_operand(op),
            Rvalue::Copy(place) => format!("Copy({})", format_place(place)),
            Rvalue::Move(place) => format!("Move({})", format_place(place)),
            Rvalue::BinaryOp(op, left, right) => {
                format!(
                    "{} {} {}",
                    format_operand(left),
                    format_binop(op),
                    format_operand(right)
                )
            }
            Rvalue::UnaryOp(op, val) => {
                format!("{}{}", format_unop(op), format_operand(val))
            }
            _ => format!("{:?}", rvalue),
        }
    }

    fn format_operand(op: &Operand) -> String {
        match op {
            Operand::Copy(place) => format_place(place),
            Operand::Move(place) => format!("Move({})", format_place(place)),
            Operand::Constant(c) => format!("{:?}", c),
        }
    }

    fn format_binop(op: &BinOp) -> &'static str {
        match op {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Rem => "%",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            _ => "?",
        }
    }

    fn format_unop(op: &UnOp) -> &'static str {
        match op {
            UnOp::Not => "!",
            UnOp::Neg => "-",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_display() {
        assert_eq!(Local(0).to_string(), "_0");
        assert_eq!(Local(42).to_string(), "_42");
    }

    #[test]
    fn test_basic_block_display() {
        assert_eq!(BasicBlock(0).to_string(), "bb0");
        assert_eq!(BasicBlock(5).to_string(), "bb5");
    }

    #[test]
    fn test_place_construction() {
        let place = Place::from_local(Local(1));
        assert!(place.is_local());
        assert_eq!(place.local, Local(1));

        let place2 = place.project(PlaceElem::Field(0));
        assert!(!place2.is_local());
    }

    #[test]
    fn test_mir_body_construction() {
        let mut body = MirBody::new(2, 0..100);

        // Add return local
        body.push_local(LocalDecl::new(Type::Unit, 0..10));

        // Add arg locals
        let i32_ty = Type::Path(crate::ast::Path {
            segments: vec![crate::ast::PathSegment {
                ident: crate::ast::Ident::new("i32"),
                generics: vec![],
            }],
        });
        body.push_local(LocalDecl::new(i32_ty.clone(), 10..20).arg());
        body.push_local(LocalDecl::new(i32_ty, 20..30).arg());

        // Add a basic block
        let mut block = BasicBlockData::new();
        block.push_stmt(Statement::Nop);
        block.set_terminator(Terminator {
            kind: TerminatorKind::Return,
            span: 30..40,
        });

        let bb = body.push_block(block);
        assert_eq!(bb, BasicBlock(0));
    }
}
