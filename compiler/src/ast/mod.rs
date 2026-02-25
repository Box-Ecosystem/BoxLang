use crate::frontend::lexer::token::StringLitKind;
use smol_str::SmolStr;
use std::fmt;

pub type Ident = SmolStr;
pub type Span = std::ops::Range<usize>;

#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Spanned<U> {
        Spanned {
            node: f(self.node),
            span: self.span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub name: Ident,
    pub items: Vec<Item>,
    
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]

pub enum Item {
    Function(Function),
    Struct(StructDef),
    Union(UnionDef),
    Enum(EnumDef),
    Impl(ImplBlock),
    Trait(TraitDef),
    Import(Import),
    Const(ConstDef),
    Static(StaticDef),
    TypeAlias(TypeAlias),
    Module(SubModule),
    ExternBlock(ExternBlock),
    MacroRules(MacroRulesDef),
    Callback(CallbackDef),
    SafeWrapper(SafeWrapper),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: Ident,
    pub params: Vec<Param>,
    
    pub return_type: Option<Type>,
    pub body: Block,
    
    pub visibility: Visibility,
    
    pub is_async: bool,
    
    pub is_unsafe: bool,
    
    pub is_extern: bool,
    
    pub abi: Option<String>,
    
    pub generics: Vec<GenericParam>,
    
    pub ffi_attrs: FfiAttributes,
    
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: Ident,
    pub ty: Type,
    
    pub is_mut: bool,
    
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]

pub enum Visibility {
    #[default]
    Private,
    Public,
    PublicSuper,
    PublicCrate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]

pub enum Stmt {
    Let(LetStmt),
    Expr(Expr),
    Item(Item),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LetStmt {
    pub name: Ident,
    
    pub ty: Option<Type>,
    
    pub init: Option<Expr>,
    
    pub is_mut: bool,
    
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]

pub enum Expr {
    Literal(Literal),
    Ident(Ident),
    Path(Path),
    PathCall(Path, Vec<Expr>),
    Binary(BinaryExpr),
    Unary(UnaryExpr),
    Call(CallExpr),
    MethodCall(MethodCallExpr),
    FieldAccess(FieldAccessExpr),
    Index(IndexExpr),
    Block(Block),
    If(IfExpr),
    Match(MatchExpr),
    Loop(LoopExpr),
    While(WhileExpr),
    For(ForExpr),
    Return(Option<Box<Expr>>),
    Break(Option<Box<Expr>>),
    Continue,
    StructInit(StructInitExpr),
    ArrayInit(ArrayInitExpr),
    TupleInit(Vec<Expr>),
    Closure(ClosureExpr),
    Async(Block),
    Await(Box<Expr>),
    Try(Box<Expr>),
    Assign(AssignExpr),
    CompoundAssign(CompoundAssignExpr),
    Range(RangeExpr),
    Cast(CastExpr),
    SizeOf(Box<Type>),
    TypeOf(Box<Expr>),
    Unsafe(Block),
    Asm(InlineAsm),
}

#[derive(Debug, Clone, PartialEq)]

pub enum Literal {
    Integer(i64),
    Float(f64),
    String(StringLitKind),
    Char(char),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BinaryExpr {
    pub left: Box<Expr>,
    pub op: BinaryOp,
    pub right: Box<Expr>,
}

#[derive(Debug, Clone, Copy, PartialEq)]

pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    And,
    Or,
    Xor,
    Shl,
    Shr,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    LogicalAnd,
    LogicalOr,
    Assign,
    Pipe,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnaryExpr {
    pub op: UnaryOp,
    pub expr: Box<Expr>,
}

#[derive(Debug, Clone, Copy, PartialEq)]

pub enum UnaryOp {
    Neg,
    Not,
    Deref,
    Ref,
    RefMut,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallExpr {
    pub func: Box<Expr>,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodCallExpr {
    pub receiver: Box<Expr>,
    pub method: Ident,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldAccessExpr {
    pub expr: Box<Expr>,
    pub field: Ident,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndexExpr {
    pub expr: Box<Expr>,
    pub index: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfExpr {
    pub cond: Box<Expr>,
    pub then_branch: Block,
    
    pub else_branch: Option<Box<Expr>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchExpr {
    pub expr: Box<Expr>,
    pub arms: Vec<MatchArm>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    
    pub guard: Option<Expr>,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq)]

pub enum Pattern {
    Wildcard,
    Literal(Literal),
    Ident(Ident),
    Path(Path),
    Struct(Path, Vec<(Ident, Pattern)>),
    Tuple(Vec<Pattern>),
    Array(Vec<Pattern>),
    Range(Box<Pattern>, Box<Pattern>),
    Ref(Box<Pattern>),
    Mut(Box<Pattern>),
    Binding(Ident, Box<Pattern>),
    Or(Vec<Pattern>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoopExpr {
    pub body: Block,
    
    pub label: Option<Ident>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhileExpr {
    pub cond: Box<Expr>,
    pub body: Block,
    
    pub label: Option<Ident>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForExpr {
    pub pattern: Pattern,
    pub expr: Box<Expr>,
    pub body: Block,
    
    pub label: Option<Ident>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructInitExpr {
    pub path: Path,
    pub fields: Vec<(Ident, Expr)>,
    
    pub rest: Option<Box<Expr>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArrayInitExpr {
    pub elements: Vec<Expr>,
    
    pub repeat: Option<(Box<Expr>, Box<Expr>)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClosureExpr {
    pub params: Vec<Param>,
    
    pub return_type: Option<Type>,
    pub body: Box<Expr>,
    
    pub is_move: bool,
    
    pub is_async: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssignExpr {
    pub left: Box<Expr>,
    pub right: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompoundAssignExpr {
    pub left: Box<Expr>,
    pub op: BinaryOp,
    pub right: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RangeExpr {
    
    pub start: Option<Box<Expr>>,
    
    pub end: Option<Box<Expr>>,
    
    pub inclusive: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CastExpr {
    pub expr: Box<Expr>,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InlineAsm {
    pub template: String,
    pub inputs: Vec<AsmOperand>,
    pub outputs: Vec<AsmOperand>,
    pub clobbers: Vec<String>,
    
    pub options: AsmOptions,
    
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AsmOperand {
    pub constraint: String,
    pub expr: Expr,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AsmOptions {
    
    pub pure: bool,
    
    pub nomem: bool,
    
    pub readonly: bool,
    
    pub preserves_flags: bool,
    
    pub nostack: bool,
}

#[derive(Debug, Clone, PartialEq)]

pub enum Type {
    Unit,
    Never,
    Path(Path),
    Ref(Box<Type>, bool),
    Ptr(Box<Type>, bool),
    Array(Box<Type>, Option<usize>),
    Slice(Box<Type>),
    Tuple(Vec<Type>),
    Function(FunctionType),
    Generic(Box<Type>, Vec<Type>),
    ImplTrait(Vec<TraitBound>),
    DynTrait(Vec<TraitBound>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionType {
    pub params: Vec<Type>,
    pub return_type: Box<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub segments: Vec<PathSegment>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PathSegment {
    pub ident: Ident,
    
    pub generics: Vec<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDef {
    pub name: Ident,
    pub fields: Vec<FieldDef>,
    
    pub generics: Vec<GenericParam>,
    
    pub visibility: Visibility,
    
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnionDef {
    pub name: Ident,
    pub fields: Vec<FieldDef>,
    
    pub generics: Vec<GenericParam>,
    
    pub visibility: Visibility,
    
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MacroRulesDef {
    pub name: Ident,
    pub rules: Vec<MacroRule>,
    
    pub visibility: Visibility,
    
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MacroRule {
    pub pattern: MacroPattern,
    pub template: MacroTemplate,
    
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]

pub enum MacroPattern {
    Empty,
    Token(String),
    Group(Vec<MacroPattern>),
    Repeat(Box<MacroPattern>, RepetitionKind, Option<String>),
    Capture(Ident, CaptureKind),
}

#[derive(Debug, Clone, Copy, PartialEq)]

pub enum CaptureKind {
    Item,
    Block,
    Stmt,
    Expr,
    Ty,
    Ident,
    Path,
    Tt,
    Literal,
    Lifetime,
}

#[derive(Debug, Clone, Copy, PartialEq)]

pub enum RepetitionKind {
    ZeroOrMore,
    OneOrMore,
    ZeroOrOne,
}

#[derive(Debug, Clone, PartialEq)]

pub enum MacroTemplate {
    Empty,
    Token(String),
    Variable(Ident),
    Group(Vec<MacroTemplate>),
    Repeat(Box<MacroTemplate>, RepetitionKind, Option<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDef {
    pub name: Ident,
    pub ty: Type,
    
    pub visibility: Visibility,
    
    pub default: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDef {
    pub name: Ident,
    pub variants: Vec<EnumVariant>,
    
    pub generics: Vec<GenericParam>,
    
    pub visibility: Visibility,
    
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: Ident,
    pub fields: EnumVariantFields,
    
    pub discriminant: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]

pub enum EnumVariantFields {
    Unit,
    Tuple(Vec<Type>),
    Struct(Vec<FieldDef>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImplBlock {
    
    pub trait_: Option<Path>,
    pub ty: Type,
    pub items: Vec<ImplItem>,
    
    pub generics: Vec<GenericParam>,
    
    pub is_unsafe: bool,
    
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]

pub enum ImplItem {
    Function(Function),
    Const(ConstDef),
    Type(TypeAlias),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitDef {
    pub name: Ident,
    pub items: Vec<TraitItem>,
    
    pub generics: Vec<GenericParam>,
    
    pub super_traits: Vec<TraitBound>,
    
    pub visibility: Visibility,
    
    pub is_unsafe: bool,
    
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]

pub enum TraitItem {
    Function(TraitFunction),
    Const(ConstDef),
    Type(TypeAlias),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitFunction {
    pub name: Ident,
    pub params: Vec<Param>,
    
    pub return_type: Option<Type>,
    
    pub default: Option<Block>,
    
    pub generics: Vec<GenericParam>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitBound {
    pub path: Path,
    
    pub generics: Vec<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    pub path: Path,
    
    pub alias: Option<Ident>,
    
    pub is_glob: bool,
    
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstDef {
    pub name: Ident,
    pub ty: Type,
    pub value: Expr,
    
    pub visibility: Visibility,
    
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StaticDef {
    pub name: Ident,
    pub ty: Type,
    pub value: Expr,
    
    pub is_mut: bool,
    
    pub visibility: Visibility,
    
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeAlias {
    pub name: Ident,
    pub ty: Type,
    
    pub generics: Vec<GenericParam>,
    
    pub visibility: Visibility,
    
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericParam {
    pub name: Ident,
    
    pub bounds: Vec<TraitBound>,
    
    pub default: Option<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubModule {
    pub name: Ident,
    pub items: Vec<Item>,
    
    pub visibility: Visibility,
    
    pub is_inline: bool,
    
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExternBlock {
    pub abi: String,
    pub items: Vec<ExternItem>,
    
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]

pub enum ExternItem {
    Function(Function),
    Static(StaticDef),
    Type(ExternType),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExternType {
    pub name: Ident,
    pub visibility: Visibility,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FfiAttributes {
    pub link_name: Option<String>,
    pub link_section: Option<String>,
    pub is_callback: bool,
    pub safe_wrapper: bool,
    pub deprecated: bool,
    pub deprecated_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallbackDef {
    pub name: Ident,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub abi: String,
    pub visibility: Visibility,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SafeWrapper {
    pub extern_name: Ident,
    pub wrapper_name: Ident,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub error_type: Option<Type>,
    pub abi: String,
    pub visibility: Visibility,
    pub span: Span,
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryOp::Add => write!(f, "+"),
            BinaryOp::Sub => write!(f, "-"),
            BinaryOp::Mul => write!(f, "*"),
            BinaryOp::Div => write!(f, "/"),
            BinaryOp::Rem => write!(f, "%"),
            BinaryOp::And => write!(f, "&"),
            BinaryOp::Or => write!(f, "|"),
            BinaryOp::Xor => write!(f, "^"),
            BinaryOp::Shl => write!(f, "<<"),
            BinaryOp::Shr => write!(f, ">>"),
            BinaryOp::Eq => write!(f, "=="),
            BinaryOp::Ne => write!(f, "!="),
            BinaryOp::Lt => write!(f, "<"),
            BinaryOp::Le => write!(f, "<="),
            BinaryOp::Gt => write!(f, ">"),
            BinaryOp::Ge => write!(f, ">="),
            BinaryOp::LogicalAnd => write!(f, "&&"),
            BinaryOp::LogicalOr => write!(f, "||"),
            BinaryOp::Assign => write!(f, "="),
            BinaryOp::Pipe => write!(f, "|>"),
        }
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnaryOp::Neg => write!(f, "-"),
            UnaryOp::Not => write!(f, "!"),
            UnaryOp::Deref => write!(f, "*"),
            UnaryOp::Ref => write!(f, "&"),
            UnaryOp::RefMut => write!(f, "&mut "),
        }
    }
}
