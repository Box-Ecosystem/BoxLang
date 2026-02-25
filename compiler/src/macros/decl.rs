//! Declarative macros (macro_rules!) for BoxLang
//!
//! Declarative macros allow pattern-based code generation using
//! pattern matching and repetition.

use crate::ast::{Expr, Ident, Item, Path, Span, Stmt};
use crate::frontend::lexer::token::Token;
use std::collections::HashMap;

/// A macro rule definition
#[derive(Debug, Clone)]
pub struct MacroRule {
    /// Pattern to match
    pub pattern: MacroPattern,
    /// Template to expand to
    pub template: MacroTemplate,
    pub span: Span,
}

/// Pattern for macro matching
#[derive(Debug, Clone)]
pub enum MacroPattern {
    /// Empty pattern
    Empty,
    /// Single token
    Token(Token),
    /// Capture group: $name:kind
    Capture { name: Ident, kind: CaptureKind },
    /// Sequence of patterns
    Sequence(Vec<MacroPattern>),
    /// Repetition: $(...)* or $(...)+ or $(...)?
    Repetition {
        pattern: Box<MacroPattern>,
        separator: Option<Token>,
        kind: RepetitionKind,
    },
}

/// Capture kind for macro patterns
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CaptureKind {
    /// Expression
    Expr,
    /// Statement
    Stmt,
    /// Type
    Ty,
    /// Identifier
    Ident,
    /// Path
    Path,
    /// Block
    Block,
    /// Literal
    Literal,
    /// Token tree (any sequence)
    Tt,
    /// Item
    Item,
    /// Meta (attribute content)
    Meta,
}

impl std::fmt::Display for CaptureKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaptureKind::Expr => write!(f, "expr"),
            CaptureKind::Stmt => write!(f, "stmt"),
            CaptureKind::Ty => write!(f, "ty"),
            CaptureKind::Ident => write!(f, "ident"),
            CaptureKind::Path => write!(f, "path"),
            CaptureKind::Block => write!(f, "block"),
            CaptureKind::Literal => write!(f, "literal"),
            CaptureKind::Tt => write!(f, "tt"),
            CaptureKind::Item => write!(f, "item"),
            CaptureKind::Meta => write!(f, "meta"),
        }
    }
}

/// Repetition kind
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RepetitionKind {
    /// Zero or more: *
    ZeroOrMore,
    /// One or more: +
    OneOrMore,
    /// Zero or one: ?
    ZeroOrOne,
}

/// Template for macro expansion
#[derive(Debug, Clone)]
pub enum MacroTemplate {
    /// Empty template
    Empty,
    /// Literal token
    Token(Token),
    /// Variable substitution: $name
    Variable(Ident),
    /// Sequence of templates
    Sequence(Vec<MacroTemplate>),
    /// Repetition in template: $(...)*
    Repetition {
        template: Box<MacroTemplate>,
        separator: Option<Token>,
        kind: RepetitionKind,
    },
}

/// A declarative macro definition
#[derive(Debug, Clone)]
pub struct DeclarativeMacro {
    /// Macro name
    pub name: Ident,
    /// Macro rules
    pub rules: Vec<MacroRule>,
    /// Whether this is exported
    pub exported: bool,
    pub span: Span,
}

/// Captured values during macro matching
#[derive(Debug, Clone)]
pub struct Captures {
    /// Single captures
    pub single: HashMap<Ident, CapturedValue>,
    /// Repeated captures (for repetition patterns)
    pub repeated: HashMap<Ident, Vec<CapturedValue>>,
}

impl Captures {
    pub fn new() -> Self {
        Self {
            single: HashMap::new(),
            repeated: HashMap::new(),
        }
    }

    pub fn insert(&mut self, name: Ident, value: CapturedValue) {
        self.single.insert(name, value);
    }

    pub fn insert_repeated(&mut self, name: Ident, values: Vec<CapturedValue>) {
        self.repeated.insert(name, values);
    }

    pub fn get(&self, name: &Ident) -> Option<&CapturedValue> {
        self.single.get(name)
    }

    pub fn get_repeated(&self, name: &Ident) -> Option<&Vec<CapturedValue>> {
        self.repeated.get(name)
    }
}

/// A captured value
#[derive(Debug, Clone)]
pub enum CapturedValue {
    /// Single token
    Token(Token),
    /// Expression
    Expr(Expr),
    /// Statement
    Stmt(Stmt),
    /// Type
    Type(crate::ast::Type),
    /// Identifier
    Ident(Ident),
    /// Path
    Path(Path),
    /// Block
    Block(crate::ast::Block),
    /// Literal
    Literal(crate::ast::Literal),
    /// Token tree (sequence)
    TokenTree(Vec<Token>),
    /// Item
    Item(Item),
    /// Meta content
    Meta(Vec<Token>),
}

/// Macro expander
pub struct MacroExpander {
    /// Defined macros
    macros: HashMap<Ident, DeclarativeMacro>,
    /// Recursion depth limit
    max_depth: usize,
}

impl MacroExpander {
    pub fn new() -> Self {
        Self {
            macros: HashMap::new(),
            max_depth: 128,
        }
    }

    /// Register a macro
    pub fn register(&mut self, macro_def: DeclarativeMacro) {
        self.macros.insert(macro_def.name.clone(), macro_def);
    }

    /// Expand a macro invocation
    pub fn expand(
        &self,
        name: &Ident,
        tokens: &[Token],
        depth: usize,
    ) -> Result<Vec<Token>, MacroError> {
        if depth > self.max_depth {
            return Err(MacroError::RecursiveExpansion {
                macro_name: name.to_string(),
            });
        }

        let macro_def = self
            .macros
            .get(name)
            .ok_or_else(|| MacroError::UnknownMacro {
                name: name.to_string(),
            })?;

        // Try each rule in order
        for rule in &macro_def.rules {
            if let Some(captures) = self.match_pattern(&rule.pattern, tokens)? {
                let expanded = self.expand_template(&rule.template, &captures, depth + 1)?;
                return Ok(expanded);
            }
        }

        Err(MacroError::NoMatchingRule {
            macro_name: name.to_string(),
        })
    }

    /// Match a pattern against tokens
    fn match_pattern(
        &self,
        pattern: &MacroPattern,
        tokens: &[Token],
    ) -> Result<Option<Captures>, MacroError> {
        let mut captures = Captures::new();
        let mut token_iter = tokens.iter().peekable();

        if self.match_pattern_recursive(pattern, &mut token_iter, &mut captures)? {
            // Check if all tokens were consumed
            if token_iter.peek().is_none() {
                return Ok(Some(captures));
            }
        }

        Ok(None)
    }

    /// Recursively match pattern
    fn match_pattern_recursive(
        &self,
        pattern: &MacroPattern,
        tokens: &mut std::iter::Peekable<std::slice::Iter<Token>>,
        captures: &mut Captures,
    ) -> Result<bool, MacroError> {
        match pattern {
            MacroPattern::Empty => Ok(true),

            MacroPattern::Token(expected) => {
                if let Some(actual) = tokens.next() {
                    Ok(actual == expected)
                } else {
                    Ok(false)
                }
            }

            MacroPattern::Capture { name, kind } => {
                self.capture_token(*kind, name.clone(), tokens, captures)
            }

            MacroPattern::Sequence(patterns) => {
                for pattern in patterns {
                    if !self.match_pattern_recursive(pattern, tokens, captures)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }

            MacroPattern::Repetition {
                pattern,
                separator,
                kind,
            } => self.match_repetition(pattern, separator.as_ref(), *kind, tokens, captures),
        }
    }

    /// Capture a token based on kind
    fn capture_token(
        &self,
        kind: CaptureKind,
        name: Ident,
        tokens: &mut std::iter::Peekable<std::slice::Iter<Token>>,
        captures: &mut Captures,
    ) -> Result<bool, MacroError> {
        match kind {
            CaptureKind::Ident => {
                if let Some(token) = tokens.peek() {
                    if let Token::Ident(ident) = token {
                        captures.insert(name, CapturedValue::Ident(Ident::new(ident)));
                        tokens.next();
                        return Ok(true);
                    }
                }
                Ok(false)
            }

            CaptureKind::Literal => {
                if let Some(token) = tokens.peek() {
                    if let Some(lit) = token_to_literal(token) {
                        captures.insert(name, CapturedValue::Literal(lit));
                        tokens.next();
                        return Ok(true);
                    }
                }
                Ok(false)
            }

            CaptureKind::Tt => {
                // Capture until matching delimiter or end
                let mut depth = 0;
                let mut captured = Vec::new();

                while let Some(token) = tokens.peek() {
                    match token {
                        Token::LParen | Token::LBracket | Token::LBrace => depth += 1,
                        Token::RParen | Token::RBracket | Token::RBrace => {
                            if depth == 0 {
                                break;
                            }
                            depth -= 1;
                        }
                        _ => {}
                    }
                    captured.push((*token).clone());
                    tokens.next();
                }

                if !captured.is_empty() {
                    captures.insert(name, CapturedValue::TokenTree(captured));
                    Ok(true)
                } else {
                    Ok(false)
                }
            }

            // For other kinds, we'd need more context (parser state)
            // For now, capture a single token
            _ => {
                if let Some(token) = tokens.next() {
                    captures.insert(name, CapturedValue::Token(token.clone()));
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        }
    }

    /// Match repetition pattern
    fn match_repetition(
        &self,
        pattern: &MacroPattern,
        separator: Option<&Token>,
        kind: RepetitionKind,
        tokens: &mut std::iter::Peekable<std::slice::Iter<Token>>,
        captures: &mut Captures,
    ) -> Result<bool, MacroError> {
        let mut matches = Vec::new();

        loop {
            let mut sub_captures = Captures::new();

            if !self.match_pattern_recursive(pattern, tokens, &mut sub_captures)? {
                break;
            }

            matches.push(sub_captures);

            // Check for separator
            if let Some(sep) = separator {
                if let Some(token) = tokens.peek() {
                    if *token == sep {
                        tokens.next();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        }

        let count = matches.len();

        // Validate repetition count
        let valid = match kind {
            RepetitionKind::ZeroOrMore => true,
            RepetitionKind::OneOrMore => count >= 1,
            RepetitionKind::ZeroOrOne => count <= 1,
        };

        if valid && count > 0 {
            // Merge captures from all matches
            for captures_from_match in matches {
                for (name, value) in captures_from_match.single {
                    captures.insert(name.clone(), value);
                }
            }
            Ok(true)
        } else if kind == RepetitionKind::ZeroOrMore || kind == RepetitionKind::ZeroOrOne {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Expand a template with captures
    fn expand_template(
        &self,
        template: &MacroTemplate,
        captures: &Captures,
        depth: usize,
    ) -> Result<Vec<Token>, MacroError> {
        let mut result = Vec::new();
        self.expand_template_recursive(template, captures, depth, &mut result)?;
        Ok(result)
    }

    /// Recursively expand template
    fn expand_template_recursive(
        &self,
        template: &MacroTemplate,
        captures: &Captures,
        depth: usize,
        result: &mut Vec<Token>,
    ) -> Result<(), MacroError> {
        match template {
            MacroTemplate::Empty => {}

            MacroTemplate::Token(token) => {
                result.push(token.clone());
            }

            MacroTemplate::Variable(name) => {
                if let Some(value) = captures.get(name) {
                    self.push_captured_value(value, result)?;
                } else if let Some(values) = captures.get_repeated(name) {
                    for value in values {
                        self.push_captured_value(value, result)?;
                    }
                } else {
                    return Err(MacroError::UnknownVariable {
                        name: name.to_string(),
                    });
                }
            }

            MacroTemplate::Sequence(templates) => {
                for template in templates {
                    self.expand_template_recursive(template, captures, depth, result)?;
                }
            }

            MacroTemplate::Repetition {
                template,
                separator: _,
                kind: _,
            } => {
                // For simplicity, expand once
                // In full implementation, handle repetition properly
                self.expand_template_recursive(template, captures, depth, result)?;
            }
        }

        Ok(())
    }

    /// Push a captured value as tokens
    fn push_captured_value(
        &self,
        value: &CapturedValue,
        result: &mut Vec<Token>,
    ) -> Result<(), MacroError> {
        match value {
            CapturedValue::Token(token) => result.push(token.clone()),
            CapturedValue::Ident(ident) => result.push(Token::Ident(ident.to_string())),
            CapturedValue::Literal(lit) => result.push(literal_to_token(lit)),
            CapturedValue::TokenTree(tokens) => result.extend(tokens.iter().cloned()),
            _ => {
                // For other types, we'd need to serialize them
                // This is simplified
            }
        }
        Ok(())
    }
}

/// Convert token to literal
fn token_to_literal(token: &Token) -> Option<crate::ast::Literal> {
    match token {
        Token::Integer(n) => Some(crate::ast::Literal::Integer(*n)),
        Token::Float(f) => Some(crate::ast::Literal::Float(*f)),
        Token::Str => Some(crate::ast::Literal::String(
            crate::frontend::lexer::token::StringLitKind::Simple("".to_string()),
        )),
        Token::Bool => Some(crate::ast::Literal::Bool(true)),
        Token::Char => Some(crate::ast::Literal::Char('\0')),
        _ => None,
    }
}

/// Convert literal to token
fn literal_to_token(lit: &crate::ast::Literal) -> Token {
    match lit {
        crate::ast::Literal::Integer(n) => Token::Integer(*n),
        crate::ast::Literal::Float(f) => Token::Float(*f),
        crate::ast::Literal::String(_s) => Token::Str,
        crate::ast::Literal::Bool(_b) => Token::Bool,
        crate::ast::Literal::Char(_c) => Token::Char,
        crate::ast::Literal::Null => Token::Null,
    }
}

/// Macro errors
#[derive(Debug, Clone)]
pub enum MacroError {
    UnknownMacro { name: String },
    NoMatchingRule { macro_name: String },
    RecursiveExpansion { macro_name: String },
    UnknownVariable { name: String },
    InvalidPattern { message: String },
}

impl std::fmt::Display for MacroError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MacroError::UnknownMacro { name } => write!(f, "unknown macro: {}", name),
            MacroError::NoMatchingRule { macro_name } => {
                write!(f, "no matching rule for macro '{}'", macro_name)
            }
            MacroError::RecursiveExpansion { macro_name } => {
                write!(f, "recursive expansion of macro '{}'", macro_name)
            }
            MacroError::UnknownVariable { name } => write!(f, "unknown variable: ${}", name),
            MacroError::InvalidPattern { message } => write!(f, "invalid pattern: {}", message),
        }
    }
}

impl std::error::Error for MacroError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_kind_display() {
        assert_eq!(CaptureKind::Expr.to_string(), "expr");
        assert_eq!(CaptureKind::Ident.to_string(), "ident");
    }

    #[test]
    fn test_captures() {
        let mut captures = Captures::new();
        let ident = Ident::new("x");
        captures.insert(ident.clone(), CapturedValue::Ident(ident.clone()));

        assert!(captures.get(&ident).is_some());
    }

    #[test]
    fn test_macro_expander_creation() {
        let expander = MacroExpander::new();
        assert!(expander.macros.is_empty());
    }
}
