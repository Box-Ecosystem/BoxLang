use crate::ast::*;
use crate::frontend::lexer::token::Token;
use crate::frontend::parser::{ParseError, Parser};

impl Parser {
    /// Parse a type
    pub fn parse_type(&mut self) -> Result<Type, ParseError> {
        self.parse_type_with_precedence(0)
    }

    fn parse_type_with_precedence(&mut self, min_prec: u8) -> Result<Type, ParseError> {
        let mut left = self.parse_type_primary()?;

        loop {
            // Parse postfix type operators
            match self.peek() {
                Some(Token::Star) if min_prec < 10 => {
                    self.advance();
                    let is_mut = if self.check(&Token::Mut) {
                        self.advance();
                        true
                    } else {
                        false
                    };
                    left = Type::Ptr(Box::new(left), is_mut);
                }
                Some(Token::And) if min_prec < 10 => {
                    self.advance();
                    let is_mut = if self.check(&Token::Mut) {
                        self.advance();
                        true
                    } else {
                        false
                    };
                    left = Type::Ref(Box::new(left), is_mut);
                }
                Some(Token::LBracket) if min_prec < 10 => {
                    self.advance();
                    if self.check(&Token::RBracket) {
                        // Slice: [T]
                        self.advance();
                        left = Type::Slice(Box::new(left));
                    } else {
                        // Array: [T; n]
                        let size = if let Some(Token::Integer(n)) = self.peek() {
                            let n = *n as usize;
                            self.advance();
                            Some(n)
                        } else {
                            None
                        };
                        self.expect(&Token::RBracket)?;
                        left = Type::Array(Box::new(left), size);
                    }
                }
                Some(Token::LParen) if min_prec < 10 => {
                    // Function type: fn(T, U) -> R
                    if let Type::Path(path) = &left {
                        if path.segments.len() == 1 && path.segments[0].ident == "fn" {
                            self.advance();
                            let mut params = Vec::new();
                            while !self.check(&Token::RParen) {
                                params.push(self.parse_type()?);
                                if self.check(&Token::Comma) {
                                    self.advance();
                                } else {
                                    break;
                                }
                            }
                            self.expect(&Token::RParen)?;

                            let return_type = if self.check(&Token::Arrow) {
                                self.advance();
                                self.parse_type()?
                            } else {
                                Type::Unit
                            };

                            left = Type::Function(FunctionType {
                                params,
                                return_type: Box::new(return_type),
                            });
                            continue;
                        }
                    }
                    break;
                }
                Some(Token::Lt) if min_prec < 5 => {
                    // Generic arguments
                    let generics = self.parse_generic_args()?;
                    left = Type::Generic(Box::new(left), generics);
                }
                _ => break,
            }
        }

        Ok(left)
    }

    fn parse_type_primary(&mut self) -> Result<Type, ParseError> {
        match self.peek() {
            Some(Token::And) => {
                self.advance();
                let is_mut = if self.check(&Token::Mut) {
                    self.advance();
                    true
                } else {
                    false
                };
                let inner = self.parse_type_primary()?;
                Ok(Type::Ref(Box::new(inner), is_mut))
            }
            Some(Token::Star) => {
                self.advance();
                let is_mut = if self.check(&Token::Mut) {
                    self.advance();
                    true
                } else {
                    false
                };
                let inner = self.parse_type_primary()?;
                Ok(Type::Ptr(Box::new(inner), is_mut))
            }
            Some(Token::LParen) => {
                self.advance();
                if self.check(&Token::RParen) {
                    // Unit type: ()
                    self.advance();
                    Ok(Type::Unit)
                } else {
                    // Tuple type: (T, U, V)
                    let mut types = Vec::new();
                    while !self.check(&Token::RParen) {
                        types.push(self.parse_type()?);
                        if self.check(&Token::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    self.expect(&Token::RParen)?;

                    if types.len() == 1 {
                        // Single-element tuple is just the type itself
                        Ok(types
                            .into_iter()
                            .next()
                            .expect("types has exactly 1 element"))
                    } else {
                        Ok(Type::Tuple(types))
                    }
                }
            }
            Some(Token::Not) => {
                // Never type: !
                self.advance();
                Ok(Type::Never)
            }
            Some(Token::Underscore) => {
                // Inferred type: _
                self.advance();
                // For now, treat as unit
                Ok(Type::Unit)
            }
            Some(Token::Fn) => {
                // Function type: fn(T) -> R
                self.advance();
                self.expect(&Token::LParen)?;
                let mut params = Vec::new();
                while !self.check(&Token::RParen) {
                    params.push(self.parse_type()?);
                    if self.check(&Token::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.expect(&Token::RParen)?;

                let return_type = if self.check(&Token::Arrow) {
                    self.advance();
                    self.parse_type()?
                } else {
                    Type::Unit
                };

                Ok(Type::Function(FunctionType {
                    params,
                    return_type: Box::new(return_type),
                }))
            }
            Some(Token::Impl) => {
                // impl Trait
                self.advance();
                let bounds = self.parse_trait_bounds()?;
                Ok(Type::ImplTrait(bounds))
            }
            Some(Token::Dyn) => {
                // dyn Trait
                self.advance();
                let bounds = self.parse_trait_bounds()?;
                Ok(Type::DynTrait(bounds))
            }
            Some(Token::Ident(_)) => {
                // Path type
                let path = self.parse_path()?;
                Ok(Type::Path(path))
            }
            Some(Token::I8) | Some(Token::I16) | Some(Token::I32) | Some(Token::I64)
            | Some(Token::U8) | Some(Token::U16) | Some(Token::U32) | Some(Token::U64)
            | Some(Token::F32) | Some(Token::F64) | Some(Token::Bool) | Some(Token::Char)
            | Some(Token::Str) | Some(Token::String)
            | Some(Token::Option) | Some(Token::Result) | Some(Token::Vec) => {
                // Builtin type
                let path = self.parse_path_from_token()?;
                Ok(Type::Path(path))
            }
            Some(Token::LBracket) => {
                // Array type: [T; n] or Slice type: [T]
                self.advance();
                let elem_type = self.parse_type()?;

                if self.check(&Token::Semi) {
                    // Array type: [T; n]
                    self.advance();
                    let size = if let Some(Token::Integer(n)) = self.peek() {
                        let n = *n as usize;
                        self.advance();
                        Some(n)
                    } else {
                        None
                    };
                    self.expect(&Token::RBracket)?;
                    Ok(Type::Array(Box::new(elem_type), size))
                } else if self.check(&Token::RBracket) {
                    // Slice type: [T]
                    self.advance();
                    Ok(Type::Slice(Box::new(elem_type)))
                } else {
                    Err(self.error_unexpected("; or ]", "other"))
                }
            }
            Some(token) => Err(self.error_unexpected("type", &format!("{:?}", token))),
            None => Err(ParseError::UnexpectedEof),
        }
    }

    /// Parse a path (e.g., std::io::Result)
    /// Also handles keywords like Option, Result, Vec as path segments
    pub fn parse_path(&mut self) -> Result<Path, ParseError> {
        let mut segments = Vec::new();

        loop {
            // Handle identifier or type keywords as path segments
            let ident = self.expect_path_identifier()?;

            // Check for generic arguments
            let generics = if self.check(&Token::Lt) {
                self.parse_generic_args()?
            } else {
                Vec::new()
            };

            segments.push(PathSegment { ident, generics });

            if self.check(&Token::ColonColon) {
                self.advance();
            } else {
                break;
            }
        }

        Ok(Path { segments })
    }

    /// Expect an identifier or type keyword as a path segment
    fn expect_path_identifier(&mut self) -> Result<Ident, ParseError> {
        match self.peek() {
            Some(Token::Ident(name)) => {
                let name = name.clone();
                self.advance();
                Ok(name.into())
            }
            Some(Token::I8) => { self.advance(); Ok("i8".into()) }
            Some(Token::I16) => { self.advance(); Ok("i16".into()) }
            Some(Token::I32) => { self.advance(); Ok("i32".into()) }
            Some(Token::I64) => { self.advance(); Ok("i64".into()) }
            Some(Token::U8) => { self.advance(); Ok("u8".into()) }
            Some(Token::U16) => { self.advance(); Ok("u16".into()) }
            Some(Token::U32) => { self.advance(); Ok("u32".into()) }
            Some(Token::U64) => { self.advance(); Ok("u64".into()) }
            Some(Token::F32) => { self.advance(); Ok("f32".into()) }
            Some(Token::F64) => { self.advance(); Ok("f64".into()) }
            Some(Token::Bool) => { self.advance(); Ok("bool".into()) }
            Some(Token::Char) => { self.advance(); Ok("char".into()) }
            Some(Token::Str) => { self.advance(); Ok("str".into()) }
            Some(Token::String) => { self.advance(); Ok("String".into()) }
            Some(Token::Option) => { self.advance(); Ok("Option".into()) }
            Some(Token::Result) => { self.advance(); Ok("Result".into()) }
            Some(Token::Vec) => { self.advance(); Ok("Vec".into()) }
            Some(token) => Err(self.error_unexpected("identifier or type", &format!("{:?}", token))),
            None => Err(ParseError::UnexpectedEof),
        }
    }

    /// Parse a path from a builtin type token
    fn parse_path_from_token(&mut self) -> Result<Path, ParseError> {
        let name = match self.peek() {
            Some(Token::I8) => "i8",
            Some(Token::I16) => "i16",
            Some(Token::I32) => "i32",
            Some(Token::I64) => "i64",
            Some(Token::U8) => "u8",
            Some(Token::U16) => "u16",
            Some(Token::U32) => "u32",
            Some(Token::U64) => "u64",
            Some(Token::F32) => "f32",
            Some(Token::F64) => "f64",
            Some(Token::Bool) => "bool",
            Some(Token::Char) => "char",
            Some(Token::Str) => "str",
            Some(Token::String) => "String",
            Some(Token::Option) => "Option",
            Some(Token::Result) => "Result",
            Some(Token::Vec) => "Vec",
            _ => return Err(self.error_unexpected("builtin type", "other")),
        };

        self.advance();

        Ok(Path {
            segments: vec![PathSegment {
                ident: name.into(),
                generics: Vec::new(),
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::lexer::tokenize;

    #[test]
    fn test_parse_simple_type() {
        let source = "i32";
        let tokens = tokenize(source);
        let mut parser = Parser::new(tokens);

        let result = parser.parse_type();
        assert!(result.is_ok());

        match result.expect("parse should succeed") {
            Type::Path(path) => {
                assert_eq!(path.segments.len(), 1);
                assert_eq!(path.segments[0].ident, "i32");
            }
            _ => panic!("Expected path type"),
        }
    }

    #[test]
    fn test_parse_path_type() {
        let source = "std::io::Result";
        let tokens = tokenize(source);
        let mut parser = Parser::new(tokens);

        let result = parser.parse_type();
        assert!(result.is_ok());

        match result.expect("parse should succeed") {
            Type::Path(path) => {
                assert_eq!(path.segments.len(), 3);
                assert_eq!(path.segments[0].ident, "std");
                assert_eq!(path.segments[1].ident, "io");
                assert_eq!(path.segments[2].ident, "Result");
            }
            _ => panic!("Expected path type"),
        }
    }

    #[test]
    fn test_parse_generic_type() {
        let source = "Vec<i32>";
        let tokens = tokenize(source);
        let mut parser = Parser::new(tokens);

        let result = parser.parse_type();
        assert!(result.is_ok());

        match result.expect("parse should succeed") {
            Type::Generic(base, args) => {
                match *base {
                    Type::Path(path) => assert_eq!(path.segments[0].ident, "Vec"),
                    _ => panic!("Expected path"),
                }
                assert_eq!(args.len(), 1);
            }
            _ => panic!("Expected generic type"),
        }
    }

    #[test]
    fn test_parse_reference_type() {
        let source = "&mut i32";
        let tokens = tokenize(source);
        let mut parser = Parser::new(tokens);

        let result = parser.parse_type();
        assert!(result.is_ok());

        match result.expect("parse should succeed") {
            Type::Ref(inner, is_mut) => {
                assert!(is_mut);
                match *inner {
                    Type::Path(path) => assert_eq!(path.segments[0].ident, "i32"),
                    _ => panic!("Expected path"),
                }
            }
            _ => panic!("Expected reference type"),
        }
    }

    #[test]
    fn test_parse_pointer_type() {
        let source = "*mut u8";
        let tokens = tokenize(source);
        let mut parser = Parser::new(tokens);

        let result = parser.parse_type();
        assert!(result.is_ok());

        match result.expect("parse should succeed") {
            Type::Ptr(inner, is_mut) => {
                assert!(is_mut);
                match *inner {
                    Type::Path(path) => assert_eq!(path.segments[0].ident, "u8"),
                    _ => panic!("Expected path"),
                }
            }
            _ => panic!("Expected pointer type"),
        }
    }

    #[test]
    fn test_parse_array_type() {
        let source = "[i32; 10]";
        let tokens = tokenize(source);
        let mut parser = Parser::new(tokens);

        let result = parser.parse_type();
        assert!(result.is_ok());

        match result.expect("parse should succeed") {
            Type::Array(inner, size) => {
                assert_eq!(size, Some(10));
                match *inner {
                    Type::Path(path) => assert_eq!(path.segments[0].ident, "i32"),
                    _ => panic!("Expected path"),
                }
            }
            _ => panic!("Expected array type"),
        }
    }

    #[test]
    fn test_parse_slice_type() {
        let source = "[u8]";
        let tokens = tokenize(source);
        let mut parser = Parser::new(tokens);

        let result = parser.parse_type();
        assert!(result.is_ok());

        match result.expect("parse should succeed") {
            Type::Slice(inner) => match *inner {
                Type::Path(path) => assert_eq!(path.segments[0].ident, "u8"),
                _ => panic!("Expected path"),
            },
            _ => panic!("Expected slice type"),
        }
    }

    #[test]
    fn test_parse_tuple_type() {
        let source = "(i32, bool)";
        let tokens = tokenize(source);
        let mut parser = Parser::new(tokens);

        let result = parser.parse_type();
        assert!(result.is_ok());

        match result.expect("parse should succeed") {
            Type::Tuple(types) => {
                assert_eq!(types.len(), 2);
            }
            _ => panic!("Expected tuple type"),
        }
    }

    #[test]
    fn test_parse_unit_type() {
        let source = "()";
        let tokens = tokenize(source);
        let mut parser = Parser::new(tokens);

        let result = parser.parse_type();
        assert!(result.is_ok());

        match result.expect("parse should succeed") {
            Type::Unit => {}
            _ => panic!("Expected unit type"),
        }
    }

    #[test]
    fn test_parse_function_type() {
        let source = "fn(i32, i32) -> i32";
        let tokens = tokenize(source);
        let mut parser = Parser::new(tokens);

        let result = parser.parse_type();
        assert!(result.is_ok());

        match result.expect("parse should succeed") {
            Type::Function(func_type) => {
                assert_eq!(func_type.params.len(), 2);
            }
            _ => panic!("Expected function type"),
        }
    }
}
