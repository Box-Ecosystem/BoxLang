use crate::ast::*;
use crate::frontend::lexer::token::Token;
use crate::frontend::parser::{ParseError, Parser};

impl Parser {
    /// Parse a block of statements
    pub fn parse_block(&mut self) -> Result<Block, ParseError> {
        let start = self.current_span().start;

        self.expect(&Token::LBrace)?;

        let mut stmts = Vec::new();

        while !self.check(&Token::RBrace) {
            if self.check(&Token::RBrace) {
                break;
            }

            // Check if this is the last statement in the block
            // by looking ahead for the closing brace
            let is_last = self.is_last_stmt_in_block();

            stmts.push(self.parse_stmt_with_optional_semi(is_last)?);
        }

        self.expect(&Token::RBrace)?;

        let end = self.current_span().end;

        Ok(Block {
            stmts,
            span: start..end,
        })
    }

    /// Check if this is the last statement in the block
    fn is_last_stmt_in_block(&self) -> bool {
        // Look ahead to see if the next token is RBrace
        if self.current >= self.tokens.len() {
            return true;
        }
        matches!(self.tokens[self.current].token, Token::RBrace)
    }

    /// Parse a statement with optional semicolon for the last statement
    fn parse_stmt_with_optional_semi(&mut self, is_last: bool) -> Result<Stmt, ParseError> {
        match self.peek() {
            Some(Token::Let) => self.parse_let_stmt_optional_semi(is_last).map(Stmt::Let),
            Some(Token::Fn) | Some(Token::Struct) | Some(Token::Enum) | Some(Token::Impl)
            | Some(Token::Trait) | Some(Token::Mod) | Some(Token::Pub) => {
                self.parse_item().map(Stmt::Item)
            }
            Some(_) => {
                // Parse expression first
                let expr = self.parse_expr()?;
                
                // After parsing the expression, check if we're at RBrace
                // If so, semicolon is optional (this is the last expression in block)
                let at_rbrace = self.check(&Token::RBrace);
                
                if at_rbrace {
                    // Last expression in block - semicolon is optional
                    // Don't consume semicolon even if present
                } else if self.check(&Token::Semi) {
                    // Has semicolon - consume it
                    self.advance();
                } else if !matches!(
                    expr,
                    Expr::Block(_)
                        | Expr::If(_)
                        | Expr::Match(_)
                        | Expr::Loop(_)
                        | Expr::While(_)
                        | Expr::For(_)
                ) {
                    // Not a block expression and no semicolon - error
                    return Err(self.error_unexpected("semicolon", &format!("{:?}", self.peek())));
                }
                
                Ok(Stmt::Expr(expr))
            }
            None => Err(ParseError::UnexpectedEof),
        }
    }

    /// Parse a let statement with optional semicolon
    fn parse_let_stmt_optional_semi(&mut self, is_last: bool) -> Result<LetStmt, ParseError> {
        let start = self.current_span().start;

        self.expect(&Token::Let)?;

        // Check for 'mut'
        let is_mut = if self.check(&Token::Mut) {
            self.advance();
            true
        } else {
            false
        };

        let name = self.expect_identifier()?;

        // Optional type annotation
        let ty = if self.check(&Token::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        // Optional initializer
        let init = if self.check(&Token::Eq) {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };

        // Semicolon is optional for the last statement in a block
        if !is_last || self.check(&Token::Semi) {
            self.expect(&Token::Semi)?;
        }

        let end = self.current_span().end;

        Ok(LetStmt {
            name,
            ty,
            init,
            is_mut,
            span: start..end,
        })
    }

    /// Parse an expression statement with optional semicolon
    fn parse_expr_stmt_optional_semi(&mut self) -> Result<Stmt, ParseError> {
        let expr = self.parse_expr()?;

        // Semicolon is optional for the last expression in a block
        if self.check(&Token::Semi) {
            self.advance();
        }

        Ok(Stmt::Expr(expr))
    }

    /// Parse a statement
    pub fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        match self.peek() {
            Some(Token::Let) => self.parse_let_stmt().map(Stmt::Let),
            Some(Token::Fn) | Some(Token::Struct) | Some(Token::Enum) | Some(Token::Impl)
            | Some(Token::Trait) | Some(Token::Mod) | Some(Token::Pub) => {
                self.parse_item().map(Stmt::Item)
            }
            Some(_) => self.parse_expr_stmt(),
            None => Err(ParseError::UnexpectedEof),
        }
    }

    /// Parse a let statement
    fn parse_let_stmt(&mut self) -> Result<LetStmt, ParseError> {
        let start = self.current_span().start;

        self.expect(&Token::Let)?;

        // Check for 'mut'
        let is_mut = if self.check(&Token::Mut) {
            self.advance();
            true
        } else {
            false
        };

        let name = self.expect_identifier()?;

        // Optional type annotation
        let ty = if self.check(&Token::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        // Optional initializer
        let init = if self.check(&Token::Eq) {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };

        self.expect(&Token::Semi)?;

        let end = self.current_span().end;

        Ok(LetStmt {
            name,
            ty,
            init,
            is_mut,
            span: start..end,
        })
    }

    /// Parse an expression statement
    fn parse_expr_stmt(&mut self) -> Result<Stmt, ParseError> {
        let expr = self.parse_expr()?;

        // Expression statements must end with semicolon, unless it's a block
        if !matches!(
            expr,
            Expr::Block(_)
                | Expr::If(_)
                | Expr::Match(_)
                | Expr::Loop(_)
                | Expr::While(_)
                | Expr::For(_)
        ) {
            self.expect(&Token::Semi)?;
        } else if self.check(&Token::Semi) {
            // Optional semicolon after block expressions
            self.advance();
        }

        Ok(Stmt::Expr(expr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::lexer::tokenize;

    #[test]
    fn test_parse_let_stmt() {
        let source = "let x: i32 = 42;";
        let tokens = tokenize(source);
        let mut parser = Parser::new(tokens);

        let result = parser.parse_stmt();
        assert!(result.is_ok(), "parse failed: {:?}", result.err());

        match result.unwrap() {
            Stmt::Let(let_stmt) => {
                assert_eq!(let_stmt.name, "x");
                assert!(!let_stmt.is_mut);
                assert!(let_stmt.ty.is_some());
                assert!(let_stmt.init.is_some());
            }
            _ => panic!("Expected let statement"),
        }
    }

    #[test]
    fn test_parse_let_mut_stmt() {
        let source = "let mut x = 42;";
        let tokens = tokenize(source);
        let mut parser = Parser::new(tokens);

        let result = parser.parse_stmt();
        assert!(result.is_ok(), "parse failed: {:?}", result.err());

        match result.unwrap() {
            Stmt::Let(let_stmt) => {
                assert_eq!(let_stmt.name, "x");
                assert!(let_stmt.is_mut);
            }
            _ => panic!("Expected let statement"),
        }
    }

    #[test]
    fn test_parse_block() {
        let source = "{ let x = 1; let y = 2; }";
        let tokens = tokenize(source);
        let mut parser = Parser::new(tokens);

        let result = parser.parse_block();
        assert!(result.is_ok(), "parse failed: {:?}", result.err());

        let block = result.unwrap();
        assert_eq!(block.stmts.len(), 2);
    }

    #[test]
    fn test_parse_empty_block() {
        let source = "{}";
        let tokens = tokenize(source);
        let mut parser = Parser::new(tokens);

        let result = parser.parse_block();
        assert!(result.is_ok(), "parse failed: {:?}", result.err());

        let block = result.unwrap();
        assert!(block.stmts.is_empty());
    }
}
