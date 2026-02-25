use crate::ast::*;
use crate::frontend::lexer::token::{SpannedToken, Token};
use thiserror::Error;

pub mod expr;
pub mod item;
pub mod stmt;
pub mod ty;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum ParseError {
    #[error(
        "unexpected token: expected {expected}, found {found} at line {line}, column {column}"
    )]
    UnexpectedToken {
        expected: String,
        found: String,
        line: usize,
        column: usize,
    },
    #[error("unexpected end of file")]
    UnexpectedEof,
    #[error("invalid syntax: {message} at line {line}, column {column}")]
    InvalidSyntax {
        message: String,
        line: usize,
        column: usize,
    },
    #[error("unclosed delimiter: {delimiter} at line {line}, column {column}")]
    UnclosedDelimiter {
        delimiter: String,
        line: usize,
        column: usize,
    },
}

/// Parser for BoxLang source code
pub struct Parser {
    tokens: Vec<SpannedToken>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<SpannedToken>) -> Self {
        Self { tokens, current: 0 }
    }

    /// Parse a complete module
    pub fn parse_module(&mut self) -> Result<Module, ParseError> {
        let start = self.current_span().start;
        let mut items = Vec::new();

        // Parse optional module declaration
        // Support both simple module name (module test;) and path form (module boxos::gpio;)
        let name = if self.check(&Token::Module) {
            self.advance(); // consume 'module'
            let path = self.parse_path()?;
            self.expect(&Token::Semi)?;
            // Convert path to string representation
            let name_str = path.segments.iter()
                .map(|s| s.ident.as_str())
                .collect::<Vec<_>>()
                .join("::");
            name_str.into()
        } else {
            "main".into()
        };

        // Parse items with error recovery
        while !self.is_at_end() {
            match self.parse_item() {
                Ok(item) => items.push(item),
                Err(e) => {
                    // Report error and attempt to recover
                    eprintln!("Parse error at line {}: {}", self.current_line(), e);
                    self.synchronize();
                    // Continue parsing if possible
                    if self.is_at_end() {
                        return Err(e);
                    }
                }
            }
        }

        let end = self.current_span().end;
        Ok(Module {
            name,
            items,
            span: start..end,
        })
    }

    /// Parse a top-level item
    fn parse_item(&mut self) -> Result<Item, ParseError> {
        // Check for visibility
        let visibility = if self.check(&Token::Pub) {
            self.advance();
            Visibility::Public
        } else {
            Visibility::Private
        };

        match self.peek() {
            Some(Token::Fn) => self.parse_function(visibility).map(Item::Function),
            Some(Token::Struct) => self.parse_struct(visibility).map(Item::Struct),
            Some(Token::Enum) => self.parse_enum(visibility).map(Item::Enum),
            Some(Token::Impl) => self.parse_impl().map(Item::Impl),
            Some(Token::Trait) => self.parse_trait(visibility).map(Item::Trait),
            Some(Token::Import) | Some(Token::Use) => self.parse_import().map(Item::Import),
            Some(Token::Const) => self.parse_const(visibility).map(Item::Const),
            Some(Token::Static) => self.parse_static(visibility).map(Item::Static),
            Some(Token::Type) => self.parse_type_alias(visibility).map(Item::TypeAlias),
            Some(Token::Mod) => self.parse_submodule(visibility).map(Item::Module),
            Some(Token::Extern) => self.parse_extern_block().map(Item::ExternBlock),
            Some(Token::Callback) => self.parse_callback(visibility).map(Item::Callback),
            Some(Token::Safe) => self.parse_safe_wrapper(visibility).map(Item::SafeWrapper),
            Some(token) => {
                Err(self.error_unexpected("item (fn, struct, enum, etc.)", &format!("{:?}", token)))
            }
            None => Err(ParseError::UnexpectedEof),
        }
    }

    // Helper methods

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.current).map(|t| &t.token)
    }

    fn peek_ahead(&self, n: usize) -> Option<&Token> {
        self.tokens.get(self.current.saturating_add(n)).map(|t| &t.token)
    }

    fn advance(&mut self) -> Option<&SpannedToken> {
        if !self.is_at_end() {
            self.current += 1;
            self.tokens.get(self.current - 1)
        } else {
            None
        }
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len()
    }

    fn check(&self, token: &Token) -> bool {
        matches!(self.peek(), Some(t) if t == token)
    }

    /// Check if current token matches without consuming
    fn is_at(&self, token: &Token) -> bool {
        self.check(token)
    }

    fn expect(&mut self, token: &Token) -> Result<(), ParseError> {
        if self.check(token) {
            self.advance();
            Ok(())
        } else {
            Err(self.error_unexpected(&format!("{:?}", token), &format!("{:?}", self.peek())))
        }
    }

    fn expect_identifier(&mut self) -> Result<Ident, ParseError> {
        match self.peek() {
            Some(Token::Ident(name)) => {
                let name = name.clone();
                self.advance();
                Ok(name.into())
            }
            Some(token) => Err(self.error_unexpected("identifier", &format!("{:?}", token))),
            None => Err(ParseError::UnexpectedEof),
        }
    }

    /// Expect a specific token and return error if not found
    fn expect_token(&mut self, expected: Token) -> Result<(), ParseError> {
        match self.peek() {
            Some(token) if *token == expected => {
                self.advance();
                Ok(())
            }
            Some(token) => Err(ParseError::UnexpectedToken {
                expected: format!("{:?}", expected),
                found: format!("{:?}", token),
                line: self.current_line(),
                column: self.current_column(),
            }),
            None => Err(ParseError::UnexpectedEof),
        }
    }

    /// Get current line number for error reporting
    fn current_line(&self) -> usize {
        self.tokens.get(self.current)
            .map(|t| t.line)
            .unwrap_or(1)
    }

    /// Get current column number for error reporting
    fn current_column(&self) -> usize {
        self.tokens.get(self.current)
            .map(|t| t.column)
            .unwrap_or(1)
    }

    fn current_span(&self) -> Span {
        if self.current < self.tokens.len() {
            self.tokens[self.current].span.clone()
        } else if !self.tokens.is_empty() {
            let last = &self.tokens[self.tokens.len() - 1];
            last.span.end..last.span.end
        } else {
            0..0
        }
    }

    fn error_unexpected(&self, expected: &str, found: &str) -> ParseError {
        let (line, column) = if let Some(token) = self.tokens.get(self.current) {
            (token.line, token.column)
        } else if let Some(token) = self.tokens.last() {
            (token.line, token.column + 1)
        } else {
            (1, 1)
        };
        ParseError::UnexpectedToken {
            expected: expected.to_string(),
            found: found.to_string(),
            line,
            column,
        }
    }

    fn error_syntax(&self, message: &str) -> ParseError {
        let (line, column) = if let Some(token) = self.tokens.get(self.current) {
            (token.line, token.column)
        } else if let Some(token) = self.tokens.last() {
            (token.line, token.column + 1)
        } else {
            (1, 1)
        };
        ParseError::InvalidSyntax {
            message: message.to_string(),
            line,
            column,
        }
    }

    /// Synchronize after an error to try to continue parsing
    fn synchronize(&mut self) {
        self.advance();

        while !self.is_at_end() {
            if matches!(self.peek(), Some(Token::Semi)) {
                self.advance();
                return;
            }

            match self.peek() {
                Some(Token::Fn) | Some(Token::Struct) | Some(Token::Enum) | Some(Token::Impl)
                | Some(Token::Trait) | Some(Token::Import) | Some(Token::Const)
                | Some(Token::Static) | Some(Token::Type) | Some(Token::Mod) | Some(Token::Pub) => {
                    return
                }
                _ => {}
            }

            self.advance();
        }
    }
}

/// Convenience function to parse source code
pub fn parse(source: &str) -> Result<Module, ParseError> {
    let tokens = crate::frontend::lexer::tokenize(source);
    let mut parser = Parser::new(tokens);
    parser.parse_module()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_module() {
        let source = "";
        let result = parse(source);
        assert!(result.is_ok(), "parse failed: {:?}", result.err());
        let module = result.unwrap();
        assert_eq!(module.name, "main");
        assert!(module.items.is_empty());
    }

    #[test]
    fn test_parse_module_with_name() {
        let source = "module test;";
        let result = parse(source);
        assert!(result.is_ok(), "parse failed: {:?}", result.err());
        let module = result.unwrap();
        assert_eq!(module.name, "test");
    }

    #[test]
    fn test_parse_simple_function() {
        let source = r#"
            fn main() {
                return 0;
            }
        "#;
        let result = parse(source);
        assert!(result.is_ok(), "parse failed: {:?}", result.err());
        let module = result.unwrap();
        assert_eq!(module.items.len(), 1);

        match &module.items[0] {
            Item::Function(func) => {
                assert_eq!(func.name, "main");
                assert!(func.params.is_empty());
            }
            _ => panic!("Expected function item"),
        }
    }

    #[test]
    fn test_parse_function_with_params() {
        let source = r#"
            fn add(a: i32, b: i32) -> i32 {
                return a + b;
            }
        "#;
        let result = parse(source);
        assert!(result.is_ok(), "parse failed: {:?}", result.err());
        let module = result.unwrap();

        match &module.items[0] {
            Item::Function(func) => {
                assert_eq!(func.name, "add");
                assert_eq!(func.params.len(), 2);
                assert!(func.return_type.is_some());
            }
            _ => panic!("Expected function item"),
        }
    }

    #[test]
    fn test_parse_public_function() {
        let source = r#"
            pub fn public_func() {}
        "#;
        let result = parse(source);
        assert!(result.is_ok(), "parse failed: {:?}", result.err());
        let module = result.unwrap();

        match &module.items[0] {
            Item::Function(func) => {
                assert_eq!(func.visibility, Visibility::Public);
            }
            _ => panic!("Expected function item"),
        }
    }
}
