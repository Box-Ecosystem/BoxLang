use crate::ast::*;
use crate::frontend::lexer::token::Token;
use crate::frontend::parser::{ParseError, Parser};

impl Parser {
    /// Parse an expression
    pub fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_expr_with_precedence(0)
    }

    /// Parse expression with given precedence
    fn parse_expr_with_precedence(&mut self, min_prec: u8) -> Result<Expr, ParseError> {
        let mut left = self.parse_expr_prefix()?;

        loop {
            let (op, prec) = match self.peek() {
                Some(Token::Eq) => (Some(BinaryOp::Assign), 1),
                Some(Token::PlusEq) => (Some(BinaryOp::Add), 1),
                Some(Token::MinusEq) => (Some(BinaryOp::Sub), 1),
                Some(Token::StarEq) => (Some(BinaryOp::Mul), 1),
                Some(Token::SlashEq) => (Some(BinaryOp::Div), 1),
                Some(Token::PercentEq) => (Some(BinaryOp::Rem), 1),
                Some(Token::AndEq) => (Some(BinaryOp::And), 1),
                Some(Token::OrEq) => (Some(BinaryOp::Or), 1),
                Some(Token::XorEq) => (Some(BinaryOp::Xor), 1),
                Some(Token::ShlEq) => (Some(BinaryOp::Shl), 1),
                Some(Token::ShrEq) => (Some(BinaryOp::Shr), 1),
                Some(Token::OrOr) => (Some(BinaryOp::LogicalOr), 2),
                Some(Token::AndAnd) => (Some(BinaryOp::LogicalAnd), 3),
                Some(Token::EqEq) => (Some(BinaryOp::Eq), 4),
                Some(Token::NotEq) => (Some(BinaryOp::Ne), 4),
                Some(Token::Lt) => (Some(BinaryOp::Lt), 4),
                Some(Token::Le) => (Some(BinaryOp::Le), 4),
                Some(Token::Gt) => (Some(BinaryOp::Gt), 4),
                Some(Token::Ge) => (Some(BinaryOp::Ge), 4),
                Some(Token::Or) => (Some(BinaryOp::Or), 5),
                Some(Token::Xor) => (Some(BinaryOp::Xor), 6),
                Some(Token::And) => (Some(BinaryOp::And), 7),
                Some(Token::Shl) => (Some(BinaryOp::Shl), 8),
                Some(Token::Shr) => (Some(BinaryOp::Shr), 8),
                Some(Token::Plus) => (Some(BinaryOp::Add), 9),
                Some(Token::Minus) => (Some(BinaryOp::Sub), 9),
                Some(Token::Star) => (Some(BinaryOp::Mul), 10),
                Some(Token::Slash) => (Some(BinaryOp::Div), 10),
                Some(Token::Percent) => (Some(BinaryOp::Rem), 10),
                // Phase 2: Pipeline operator (low precedence, right associative)
                // Using precedence 11 (higher than multiplicative) for proper chaining
                Some(Token::Pipe) => (Some(BinaryOp::Pipe), 11),
                _ => (None, 0),
            };

            // Check for range expression (.. or ..=)
            // Range has very low precedence (0.5) to allow almost anything inside
            if let Some(Token::DotDot) = self.peek() {
                if min_prec <= 1 {
                    self.advance();
                    // Parse optional end expression
                    let end = if self.peek().map(|t| self.is_expr_start(t)).unwrap_or(false) {
                        Some(Box::new(self.parse_expr_with_precedence(1)?))
                    } else {
                        None
                    };
                    return Ok(Expr::Range(RangeExpr {
                        start: Some(Box::new(left)),
                        end,
                        inclusive: false,
                    }));
                }
            }

            if let Some(Token::DotDotDot) = self.peek() {
                if min_prec <= 1 {
                    self.advance();
                    // Parse optional end expression
                    let end = if self.peek().map(|t| self.is_expr_start(t)).unwrap_or(false) {
                        Some(Box::new(self.parse_expr_with_precedence(1)?))
                    } else {
                        None
                    };
                    return Ok(Expr::Range(RangeExpr {
                        start: Some(Box::new(left)),
                        end,
                        inclusive: true,
                    }));
                }
            }

            if let Some(op) = op {
                if prec < min_prec {
                    break;
                }
                self.advance();
                let right = self.parse_expr_with_precedence(prec + 1)?;
                left = Expr::Binary(BinaryExpr {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                });
            } else {
                break;
            }
        }

        Ok(left)
    }

    /// Parse prefix expression
    fn parse_expr_prefix(&mut self) -> Result<Expr, ParseError> {
        match self.peek() {
            Some(Token::Minus) => {
                self.advance();
                let expr = self.parse_expr_with_precedence(11)?;
                Ok(Expr::Unary(UnaryExpr {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                }))
            }
            Some(Token::Not) => {
                self.advance();
                let expr = self.parse_expr_with_precedence(11)?;
                Ok(Expr::Unary(UnaryExpr {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                }))
            }
            Some(Token::Star) => {
                self.advance();
                let expr = self.parse_expr_with_precedence(11)?;
                Ok(Expr::Unary(UnaryExpr {
                    op: UnaryOp::Deref,
                    expr: Box::new(expr),
                }))
            }
            Some(Token::And) => {
                self.advance();
                let is_mut = if self.check(&Token::Mut) {
                    self.advance();
                    true
                } else {
                    false
                };
                let expr = self.parse_expr_with_precedence(11)?;
                Ok(Expr::Unary(UnaryExpr {
                    op: if is_mut {
                        UnaryOp::RefMut
                    } else {
                        UnaryOp::Ref
                    },
                    expr: Box::new(expr),
                }))
            }
            Some(Token::Await) => {
                self.advance();
                let expr = self.parse_expr_with_precedence(11)?;
                Ok(Expr::Await(Box::new(expr)))
            }
            _ => self.parse_expr_postfix(),
        }
    }

    /// Check if the next tokens match struct initialization pattern: { field: value, ... } or { field }
    fn is_struct_init_pattern(&self) -> bool {
        // Lookahead: after LBrace, we expect: identifier (: | , | })
        // This distinguishes struct init from regular blocks
        if let Some(Token::Ident(_)) = self.peek_ahead(1) {
            match self.peek_ahead(2) {
                Some(Token::Colon) => return true,  // { field: value }
                Some(Token::Comma) => return true,  // { field, }
                Some(Token::RBrace) => return true, // { field }
                _ => {}
            }
        }
        false
    }

    /// Parse postfix expression
    fn parse_expr_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_expr_primary()?;

        loop {
            match self.peek() {
                Some(Token::LParen) => {
                    // Function call
                    self.advance();
                    let args = self.parse_call_args()?;
                    self.expect(&Token::RParen)?;
                    expr = Expr::Call(CallExpr {
                        func: Box::new(expr),
                        args,
                    });
                }
                Some(Token::LBrace)
                    if matches!(&expr, Expr::Ident(_)) && self.is_struct_init_pattern() =>
                {
                    // Struct initialization: Type { field: value, ... }
                    self.advance();
                    let fields = self.parse_struct_init_fields()?;
                    self.expect(&Token::RBrace)?;

                    // Convert the identifier to a path
                    let path = if let Expr::Ident(name) = expr {
                        Path {
                            segments: vec![PathSegment {
                                ident: name,
                                generics: Vec::new(),
                            }],
                        }
                    } else {
                        return Err(self.error_unexpected("struct name", "other"));
                    };

                    expr = Expr::StructInit(StructInitExpr {
                        path,
                        fields,
                        rest: None,
                    });
                }
                Some(Token::ColonColon) => {
                    // Path expression: Type::method or Module::Type
                    self.advance();
                    match self.peek() {
                        Some(Token::Ident(name)) => {
                            let name = name.clone();
                            self.advance();
                            
                            // Check if this is a path call: Result::Ok(value)
                            if self.check(&Token::LParen) {
                                self.advance();
                                let args = self.parse_call_args()?;
                                self.expect(&Token::RParen)?;
                                
                                // Build path from current expression
                                let path = Self::expr_to_path(&expr, name);
                                expr = Expr::PathCall(path, args);
                            } else {
                                // Convert to field access for now
                                // In a full implementation, this would be a proper path
                                expr = Expr::FieldAccess(FieldAccessExpr {
                                    expr: Box::new(expr),
                                    field: name.into(),
                                });
                            }
                        }
                        _ => return Err(self.error_unexpected("identifier after ::", "other")),
                    }
                }
                Some(Token::Dot) => {
                    self.advance();
                    match self.peek() {
                        Some(Token::Ident(name)) => {
                            let name = name.clone();
                            self.advance();

                            // Check for method call
                            if self.check(&Token::LParen) {
                                self.advance();
                                let args = self.parse_call_args()?;
                                self.expect(&Token::RParen)?;
                                expr = Expr::MethodCall(MethodCallExpr {
                                    receiver: Box::new(expr),
                                    method: name.into(),
                                    args,
                                });
                            } else {
                                expr = Expr::FieldAccess(FieldAccessExpr {
                                    expr: Box::new(expr),
                                    field: name.into(),
                                });
                            }
                        }
                        Some(Token::Integer(n)) => {
                            // Tuple index: tuple.0
                            let n = *n;
                            self.advance();
                            expr = Expr::FieldAccess(FieldAccessExpr {
                                expr: Box::new(expr),
                                field: n.to_string().into(),
                            });
                        }
                        _ => return Err(self.error_unexpected("identifier or number", "other")),
                    }
                }
                Some(Token::LBracket) => {
                    // Index expression
                    self.advance();
                    let index = self.parse_expr()?;
                    self.expect(&Token::RBracket)?;
                    expr = Expr::Index(IndexExpr {
                        expr: Box::new(expr),
                        index: Box::new(index),
                    });
                }
                Some(Token::Question) => {
                    // Try operator: expr?
                    self.advance();
                    expr = Expr::Try(Box::new(expr));
                }
                Some(Token::As) => {
                    // Cast expression
                    self.advance();
                    let ty = self.parse_type()?;
                    expr = Expr::Cast(CastExpr {
                        expr: Box::new(expr),
                        ty,
                    });
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    /// Parse struct initialization fields
    fn parse_struct_init_fields(&mut self) -> Result<Vec<(Ident, Expr)>, ParseError> {
        let mut fields = Vec::new();

        while !self.check(&Token::RBrace) {
            if self.check(&Token::RBrace) {
                break;
            }

            let name = self.expect_identifier()?;
            
            // Check for shorthand syntax: { field } means { field: field }
            let value = if self.check(&Token::Colon) {
                self.advance();
                self.parse_expr()?
            } else {
                // Shorthand: field name is also the value expression
                Expr::Ident(name.clone())
            };

            fields.push((name, value));

            if self.check(&Token::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        Ok(fields)
    }

    /// Parse primary expression
    fn parse_expr_primary(&mut self) -> Result<Expr, ParseError> {
        match self.peek() {
            Some(Token::Integer(n)) => {
                let n = *n;
                self.advance();
                Ok(Expr::Literal(Literal::Integer(n)))
            }
            Some(Token::Float(f)) => {
                let f = *f;
                self.advance();
                Ok(Expr::Literal(Literal::Float(f)))
            }
            Some(Token::StringLit(s)) => {
                let s = s.clone();
                self.advance();
                Ok(Expr::Literal(Literal::String(s)))
            }
            Some(Token::CharLit(c)) => {
                let c = *c;
                self.advance();
                Ok(Expr::Literal(Literal::Char(c)))
            }
            Some(Token::True) => {
                self.advance();
                Ok(Expr::Literal(Literal::Bool(true)))
            }
            Some(Token::False) => {
                self.advance();
                Ok(Expr::Literal(Literal::Bool(false)))
            }
            Some(Token::Null) => {
                self.advance();
                Ok(Expr::Literal(Literal::Null))
            }
            Some(Token::Ident(name)) => {
                let name = name.clone();
                self.advance();
                Ok(Expr::Ident(name.into()))
            }
            Some(Token::SelfLower) => {
                self.advance();
                Ok(Expr::Ident("self".into()))
            }
            Some(Token::Unsafe) => {
                // Unsafe block expression
                self.advance();
                let block = self.parse_block()?;
                Ok(Expr::Unsafe(block))
            }
            Some(Token::Option) | Some(Token::Result) | Some(Token::Vec) => {
                // Type keywords as path expressions (e.g., Result::Ok, Option::Some)
                let path = self.parse_path()?;
                Ok(Expr::Path(path))
            }
            Some(Token::Ok) => {
                // Ok() constructor
                self.advance();
                self.expect(&Token::LParen)?;
                let value = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(Expr::PathCall(Path {
                    segments: vec![PathSegment {
                        ident: "Ok".into(),
                        generics: Vec::new(),
                    }],
                }, vec![value]))
            }
            Some(Token::Err) => {
                // Err() constructor
                self.advance();
                self.expect(&Token::LParen)?;
                let value = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(Expr::PathCall(Path {
                    segments: vec![PathSegment {
                        ident: "Err".into(),
                        generics: Vec::new(),
                    }],
                }, vec![value]))
            }
            Some(Token::Some) => {
                // Some() constructor
                self.advance();
                self.expect(&Token::LParen)?;
                let value = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(Expr::PathCall(Path {
                    segments: vec![PathSegment {
                        ident: "Some".into(),
                        generics: Vec::new(),
                    }],
                }, vec![value]))
            }
            Some(Token::None) => {
                // None constant
                self.advance();
                Ok(Expr::Path(Path {
                    segments: vec![PathSegment {
                        ident: "None".into(),
                        generics: Vec::new(),
                    }],
                }))
            }
            Some(Token::LParen) => {
                self.advance();
                if self.check(&Token::RParen) {
                    // Unit literal: ()
                    self.advance();
                    Ok(Expr::Literal(Literal::Integer(0)))
                } else {
                    // Grouped expression or tuple
                    let expr = self.parse_expr()?;
                    if self.check(&Token::Comma) {
                        // Tuple
                        let mut elements = vec![expr];
                        while self.check(&Token::Comma) {
                            self.advance();
                            if self.check(&Token::RParen) {
                                break;
                            }
                            elements.push(self.parse_expr()?);
                        }
                        self.expect(&Token::RParen)?;
                        Ok(Expr::TupleInit(elements))
                    } else {
                        self.expect(&Token::RParen)?;
                        Ok(expr)
                    }
                }
            }
            Some(Token::LBracket) => {
                // Array literal
                self.advance();
                let mut elements = Vec::new();
                while !self.check(&Token::RBracket) {
                    elements.push(self.parse_expr()?);
                    if self.check(&Token::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.expect(&Token::RBracket)?;
                Ok(Expr::ArrayInit(ArrayInitExpr {
                    elements,
                    repeat: None,
                }))
            }
            Some(Token::LBrace) => {
                // Block expression
                let block = self.parse_block()?;
                Ok(Expr::Block(block))
            }
            Some(Token::If) => self.parse_if_expr(),
            Some(Token::Match) => self.parse_match_expr(),
            Some(Token::Loop) => self.parse_loop_expr(),
            Some(Token::While) => self.parse_while_expr(),
            Some(Token::For) => self.parse_for_expr(),
            Some(Token::Return) => {
                self.advance();
                let value = if self.check(&Token::Semi) || self.check(&Token::RBrace) {
                    None
                } else {
                    Some(Box::new(self.parse_expr()?))
                };
                Ok(Expr::Return(value))
            }
            Some(Token::Break) => {
                self.advance();
                let value = if self.check(&Token::Semi) || self.check(&Token::RBrace) {
                    None
                } else {
                    Some(Box::new(self.parse_expr()?))
                };
                Ok(Expr::Break(value))
            }
            Some(Token::Continue) => {
                self.advance();
                Ok(Expr::Continue)
            }
            Some(Token::Async) => {
                self.advance();
                if self.check(&Token::LBrace) {
                    // Async block
                    let block = self.parse_block()?;
                    Ok(Expr::Async(block))
                } else if self.check(&Token::Fn) || self.check(&Token::LParen) {
                    // Async closure
                    self.parse_closure_expr(true)
                } else {
                    Err(self.error_unexpected("async block or closure", "other"))
                }
            }
            Some(Token::Move) => {
                self.advance();
                self.parse_closure_expr(true)
            }
            Some(Token::Fn) | Some(Token::OrOr) | Some(Token::Or) => {
                // Closure expression
                self.parse_closure_expr(false)
            }
            Some(token) => Err(self.error_unexpected("expression", &format!("{:?}", token))),
            None => Err(ParseError::UnexpectedEof),
        }
    }

    /// Parse function call arguments
    fn parse_call_args(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut args = Vec::new();
        while !self.check(&Token::RParen) {
            args.push(self.parse_expr()?);
            if self.check(&Token::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        Ok(args)
    }

    /// Parse if expression
    fn parse_if_expr(&mut self) -> Result<Expr, ParseError> {
        self.expect(&Token::If)?;
        let cond = Box::new(self.parse_expr()?);
        let then_branch = self.parse_block()?;

        let else_branch = if self.check(&Token::Else) {
            self.advance();
            if self.check(&Token::If) {
                Some(Box::new(self.parse_if_expr()?))
            } else {
                Some(Box::new(Expr::Block(self.parse_block()?)))
            }
        } else {
            None
        };

        Ok(Expr::If(IfExpr {
            cond,
            then_branch,
            else_branch,
        }))
    }

    /// Parse match expression
    fn parse_match_expr(&mut self) -> Result<Expr, ParseError> {
        self.expect(&Token::Match)?;
        let expr = Box::new(self.parse_expr()?);
        self.expect(&Token::LBrace)?;

        let mut arms = Vec::new();
        while !self.check(&Token::RBrace) {
            arms.push(self.parse_match_arm()?);
            if self.check(&Token::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        self.expect(&Token::RBrace)?;

        Ok(Expr::Match(MatchExpr { expr, arms }))
    }

    /// Parse match arm
    fn parse_match_arm(&mut self) -> Result<MatchArm, ParseError> {
        let pattern = self.parse_pattern()?;

        let guard = if self.check(&Token::If) {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };

        self.expect(&Token::FatArrow)?;
        let body = self.parse_expr()?;

        Ok(MatchArm {
            pattern,
            guard,
            body,
        })
    }

    /// Parse pattern
    fn parse_pattern(&mut self) -> Result<Pattern, ParseError> {
        match self.peek() {
            Some(Token::Underscore) => {
                self.advance();
                Ok(Pattern::Wildcard)
            }
            Some(Token::Ident(name)) => {
                let name = name.clone();
                self.advance();

                // Check for path pattern (e.g., Some(x), None)
                if self.check(&Token::ColonColon)
                    || self.check(&Token::LParen)
                    || self.check(&Token::LBrace)
                {
                    // It's a path pattern
                    let mut path = Path {
                        segments: vec![PathSegment {
                            ident: name.into(),
                            generics: Vec::new(),
                        }],
                    };

                    while self.check(&Token::ColonColon) {
                        self.advance();
                        let ident = self.expect_identifier()?;
                        path.segments.push(PathSegment {
                            ident,
                            generics: Vec::new(),
                        });
                    }

                    // Parse struct or tuple pattern
                    if self.check(&Token::LBrace) {
                        self.advance();
                        let mut fields = Vec::new();
                        while !self.check(&Token::RBrace) {
                            let field_name = self.expect_identifier()?;
                            let pattern = if self.check(&Token::Colon) {
                                self.advance();
                                self.parse_pattern()?
                            } else {
                                Pattern::Ident(field_name.clone())
                            };
                            fields.push((field_name, pattern));
                            if self.check(&Token::Comma) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        self.expect(&Token::RBrace)?;
                        Ok(Pattern::Struct(path, fields))
                    } else if self.check(&Token::LParen) {
                        self.advance();
                        let mut patterns = Vec::new();
                        while !self.check(&Token::RParen) {
                            patterns.push(self.parse_pattern()?);
                            if self.check(&Token::Comma) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        self.expect(&Token::RParen)?;
                        Ok(Pattern::Tuple(patterns))
                    } else {
                        Ok(Pattern::Path(path))
                    }
                } else {
                    // Check for binding pattern: x @ pattern
                    if self.check(&Token::At) {
                        self.advance();
                        let inner = self.parse_pattern()?;
                        Ok(Pattern::Binding(name.into(), Box::new(inner)))
                    } else {
                        Ok(Pattern::Ident(name.into()))
                    }
                }
            }
            Some(Token::Integer(n)) => {
                let n = *n;
                self.advance();
                Ok(Pattern::Literal(Literal::Integer(n)))
            }
            Some(Token::StringLit(s)) => {
                let s = s.clone();
                self.advance();
                Ok(Pattern::Literal(Literal::String(s)))
            }
            Some(Token::CharLit(c)) => {
                let c = *c;
                self.advance();
                Ok(Pattern::Literal(Literal::Char(c)))
            }
            Some(Token::True) => {
                self.advance();
                Ok(Pattern::Literal(Literal::Bool(true)))
            }
            Some(Token::False) => {
                self.advance();
                Ok(Pattern::Literal(Literal::Bool(false)))
            }
            Some(Token::Ref) => {
                self.advance();
                let inner = self.parse_pattern()?;
                Ok(Pattern::Ref(Box::new(inner)))
            }
            Some(Token::Mut) => {
                self.advance();
                let inner = self.parse_pattern()?;
                Ok(Pattern::Mut(Box::new(inner)))
            }
            Some(Token::LParen) => {
                self.advance();
                let mut patterns = Vec::new();
                while !self.check(&Token::RParen) {
                    patterns.push(self.parse_pattern()?);
                    if self.check(&Token::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.expect(&Token::RParen)?;
                Ok(Pattern::Tuple(patterns))
            }
            Some(Token::LBracket) => {
                self.advance();
                let mut patterns = Vec::new();
                while !self.check(&Token::RBracket) {
                    patterns.push(self.parse_pattern()?);
                    if self.check(&Token::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.expect(&Token::RBracket)?;
                Ok(Pattern::Array(patterns))
            }
            Some(token) => Err(self.error_unexpected("pattern", &format!("{:?}", token))),
            None => Err(ParseError::UnexpectedEof),
        }
    }

    /// Parse loop expression
    fn parse_loop_expr(&mut self) -> Result<Expr, ParseError> {
        self.expect(&Token::Loop)?;
        let body = self.parse_block()?;
        Ok(Expr::Loop(LoopExpr { body, label: None }))
    }

    /// Parse while expression
    fn parse_while_expr(&mut self) -> Result<Expr, ParseError> {
        self.expect(&Token::While)?;
        let cond = Box::new(self.parse_expr()?);
        let body = self.parse_block()?;
        Ok(Expr::While(WhileExpr {
            cond,
            body,
            label: None,
        }))
    }

    /// Parse for expression
    fn parse_for_expr(&mut self) -> Result<Expr, ParseError> {
        self.expect(&Token::For)?;
        let pattern = self.parse_pattern()?;
        self.expect(&Token::In)?;
        let expr = Box::new(self.parse_expr()?);
        let body = self.parse_block()?;
        Ok(Expr::For(ForExpr {
            pattern,
            expr,
            body,
            label: None,
        }))
    }

    /// Parse closure expression
    fn parse_closure_expr(&mut self, is_move: bool) -> Result<Expr, ParseError> {
        // Parse parameters
        let params = if self.check(&Token::OrOr) {
            // || ...
            self.advance();
            Vec::new()
        } else if self.check(&Token::Or) {
            // |params| ...
            self.advance();
            let mut params = Vec::new();
            while !self.check(&Token::Or) {
                let is_mut = if self.check(&Token::Mut) {
                    self.advance();
                    true
                } else {
                    false
                };
                let name = self.expect_identifier()?;
                let ty = if self.check(&Token::Colon) {
                    self.advance();
                    self.parse_type()?
                } else {
                    Type::Unit // Inferred
                };
                params.push(Param {
                    name,
                    ty,
                    is_mut,
                    span: 0..0,
                });
                if self.check(&Token::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect(&Token::Or)?;
            params
        } else if self.check(&Token::Fn) {
            // fn(params) -> ret { body }
            return self.parse_function(Visibility::Private).map(|f| {
                Expr::Closure(ClosureExpr {
                    params: f.params,
                    return_type: f.return_type,
                    body: Box::new(Expr::Block(f.body)),
                    is_move,
                    is_async: false,
                })
            });
        } else {
            return Err(self.error_unexpected("closure parameters", "other"));
        };

        // Parse return type
        let return_type = if self.check(&Token::Arrow) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        // Parse body
        let body = if self.check(&Token::LBrace) {
            Box::new(Expr::Block(self.parse_block()?))
        } else {
            self.expect(&Token::FatArrow)?;
            Box::new(self.parse_expr()?)
        };

        Ok(Expr::Closure(ClosureExpr {
            params,
            return_type,
            body,
            is_move,
            is_async: false,
        }))
    }

    /// Convert an expression to a path, adding a new segment
    fn expr_to_path(expr: &Expr, new_segment: String) -> Path {
        match expr {
            Expr::Ident(name) => Path {
                segments: vec![
                    PathSegment {
                        ident: name.clone(),
                        generics: Vec::new(),
                    },
                    PathSegment {
                        ident: new_segment.into(),
                        generics: Vec::new(),
                    },
                ],
            },
            Expr::Path(path) => {
                let mut segments = path.segments.clone();
                segments.push(PathSegment {
                    ident: new_segment.into(),
                    generics: Vec::new(),
                });
                Path { segments }
            }
            Expr::FieldAccess(field_access) => {
                // Convert field access to path
                let base_path = Self::expr_to_path(&field_access.expr, field_access.field.as_str().to_string());
                let mut segments = base_path.segments;
                segments.push(PathSegment {
                    ident: new_segment.into(),
                    generics: Vec::new(),
                });
                Path { segments }
            }
            _ => Path {
                segments: vec![PathSegment {
                    ident: new_segment.into(),
                    generics: Vec::new(),
                }],
            },
        }
    }

    /// Check if a token can start an expression
    fn is_expr_start(&self, token: &Token) -> bool {
        matches!(
            token,
            Token::Integer(_)
                | Token::Float(_)
                | Token::StringLit(_)
                | Token::CharLit(_)
                | Token::True
                | Token::False
                | Token::Null
                | Token::Ident(_)
                | Token::SelfLower
                | Token::LParen
                | Token::LBrace
                | Token::LBracket
                | Token::If
                | Token::Match
                | Token::Loop
                | Token::While
                | Token::For
                | Token::Return
                | Token::Break
                | Token::Continue
                | Token::Async
                | Token::Move
                | Token::Fn
                | Token::Or
                | Token::OrOr
                | Token::Minus
                | Token::Not
                | Token::Star
                | Token::And
                | Token::Await
                | Token::Unsafe
                | Token::Option
                | Token::Result
                | Token::Vec
                | Token::Ok
                | Token::Err
                | Token::Some
                | Token::None
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::lexer::tokenize;

    #[test]
    fn test_parse_integer_literal() {
        let source = "42";
        let tokens = tokenize(source);
        let mut parser = Parser::new(tokens);

        let result = parser.parse_expr();
        assert!(result.is_ok(), "parse failed: {:?}", result.err());

        match result.unwrap() {
            Expr::Literal(Literal::Integer(42)) => {}
            _ => panic!("Expected integer literal"),
        }
    }

    #[test]
    fn test_parse_binary_expr() {
        let source = "1 + 2 * 3";
        let tokens = tokenize(source);
        let mut parser = Parser::new(tokens);

        let result = parser.parse_expr();
        assert!(result.is_ok(), "parse failed: {:?}", result.err());

        match result.unwrap() {
            Expr::Binary(BinaryExpr { left: _, op, right }) => {
                assert_eq!(op, BinaryOp::Add);
                // left should be 1
                // right should be 2 * 3
                match *right {
                    Expr::Binary(BinaryExpr { op, .. }) => {
                        assert_eq!(op, BinaryOp::Mul);
                    }
                    _ => panic!("Expected multiplication"),
                }
            }
            _ => panic!("Expected binary expression"),
        }
    }

    #[test]
    fn test_parse_function_call() {
        let source = "foo(1, 2)";
        let tokens = tokenize(source);
        let mut parser = Parser::new(tokens);

        let result = parser.parse_expr();
        assert!(result.is_ok(), "parse failed: {:?}", result.err());

        match result.unwrap() {
            Expr::Call(CallExpr { func, args }) => {
                match *func {
                    Expr::Ident(name) => assert_eq!(name, "foo"),
                    _ => panic!("Expected identifier"),
                }
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected call expression"),
        }
    }

    #[test]
    fn test_parse_if_expr() {
        let source = "if x { 1 } else { 2 }";
        let tokens = tokenize(source);
        let mut parser = Parser::new(tokens);

        let result = parser.parse_expr();
        assert!(result.is_ok(), "parse failed: {:?}", result.err());

        match result.unwrap() {
            Expr::If(IfExpr {
                cond: _,
                then_branch: _,
                else_branch,
            }) => {
                assert!(else_branch.is_some());
            }
            _ => panic!("Expected if expression"),
        }
    }

    #[test]
    fn test_parse_closure() {
        let source = "|x: i32| -> i32 { x + 1 }";
        let tokens = tokenize(source);
        let mut parser = Parser::new(tokens);

        let result = parser.parse_expr();
        assert!(result.is_ok(), "parse failed: {:?}", result.err());

        match result.unwrap() {
            Expr::Closure(ClosureExpr {
                params,
                return_type,
                ..
            }) => {
                assert_eq!(params.len(), 1);
                assert!(return_type.is_some());
            }
            _ => panic!("Expected closure expression"),
        }
    }

    #[test]
    fn test_parse_range_expr() {
        // Test inclusive range: 0..5
        let source = "0..5";
        let tokens = tokenize(source);
        let mut parser = Parser::new(tokens);

        let result = parser.parse_expr();
        assert!(result.is_ok(), "parse failed: {:?}", result.err());

        match result.unwrap() {
            Expr::Range(RangeExpr {
                start,
                end,
                inclusive,
            }) => {
                assert!(start.is_some());
                assert!(end.is_some());
                assert!(!inclusive);
            }
            _ => panic!("Expected range expression"),
        }
    }

    #[test]
    fn test_parse_range_inclusive_expr() {
        // Test inclusive range: 0...5
        let source = "0...5";
        let tokens = tokenize(source);
        let mut parser = Parser::new(tokens);

        let result = parser.parse_expr();
        assert!(result.is_ok(), "parse failed: {:?}", result.err());

        match result.unwrap() {
            Expr::Range(RangeExpr {
                start,
                end,
                inclusive,
            }) => {
                assert!(start.is_some());
                assert!(end.is_some());
                assert!(inclusive);
            }
            _ => panic!("Expected range expression"),
        }
    }

    #[test]
    fn test_parse_range_from_expr() {
        // Test range from: 0..
        let source = "0..";
        let tokens = tokenize(source);
        let mut parser = Parser::new(tokens);

        let result = parser.parse_expr();
        assert!(result.is_ok(), "parse failed: {:?}", result.err());

        match result.unwrap() {
            Expr::Range(RangeExpr {
                start,
                end,
                inclusive,
            }) => {
                assert!(start.is_some());
                assert!(end.is_none());
                assert!(!inclusive);
            }
            _ => panic!("Expected range expression"),
        }
    }
}
