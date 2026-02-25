//! BoxLang Compiler Library
//!
//! A systems programming language for Box Ecosystem

pub mod frontend;

pub mod ast;

pub mod middle;

pub use middle::mir;

pub mod typeck;

pub mod codegen;

pub mod ctfe;

pub mod runtime;

pub mod ui;

pub mod macros;

pub mod util;

pub mod integration;

pub mod compiler_detector;

pub mod compilation_pipeline;

pub mod ty;

pub mod diagnostics;

pub mod module;

pub use codegen::generate_c;
pub use ctfe::{ConstEvaluator, ConstValue};
pub use frontend::lexer::tokenize;
pub use frontend::parser::parse;
pub use macros::{MacroContext, MacroError, MacroRegistry};
pub use middle::mir::{BasicBlock, Local, MirBody, Operand, Place, Rvalue, Statement, Terminator};
pub use typeck::type_check;
pub use ui::{init_ui, UI};

pub use module::{ModuleSystem, ModuleId, ModuleInfo, ModuleError};
pub use module::loader::StdLoader;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub const NAME: &str = "BoxLang";

pub use ast::Module;
pub use frontend::lexer::token::SpannedToken;
pub use frontend::parser::ParseError;
