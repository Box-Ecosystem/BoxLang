//! Frontend Module
//!
//! The frontend is responsible for lexing and parsing source code into AST.
//! It includes:
//! - Lexer: Tokenizes source code
//! - Parser: Builds AST from tokens

pub mod lexer;
pub mod parser;

// Re-exports
pub use lexer::Lexer;
pub use parser::Parser;
