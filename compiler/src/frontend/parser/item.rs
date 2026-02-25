use crate::ast::*;
use crate::frontend::lexer::token::Token;
use crate::frontend::parser::{ParseError, Parser};

impl Parser {
    /// Parse a function definition
    /// Supports both regular functions with body and extern functions without body
    pub fn parse_function(&mut self, visibility: Visibility) -> Result<Function, ParseError> {
        let start = self.current_span().start;

        self.expect(&Token::Fn)?;
        let name = self.expect_identifier()?;

        // Parse generics
        let generics = if self.check(&Token::Lt) {
            self.parse_generic_params()?
        } else {
            Vec::new()
        };

        // Parse parameters
        self.expect(&Token::LParen)?;
        let params = self.parse_function_params()?;
        self.expect(&Token::RParen)?;

        // Parse return type
        let return_type = if self.check(&Token::Arrow) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        // Parse body - for extern functions, body may be absent (just a semicolon)
        let body = if self.check(&Token::LBrace) {
            self.parse_block()?
        } else if self.check(&Token::Semi) {
            self.advance();
            // Empty block for extern functions
            Block {
                stmts: Vec::new(),
                span: start..start,
            }
        } else {
            return Err(self.error_unexpected("{ or ;", &format!("{:?}", self.peek())));
        };

        let end = self.current_span().end;

        Ok(Function {
            name,
            params,
            return_type,
            body,
            visibility,
            is_async: false,
            is_unsafe: false,
            is_extern: false,
            abi: None,
            generics,
            ffi_attrs: FfiAttributes::default(),
            span: start..end,
        })
    }

    /// Parse function parameters
    fn parse_function_params(&mut self) -> Result<Vec<Param>, ParseError> {
        let mut params = Vec::new();

        while !self.check(&Token::RParen) {
            let param_start = self.current_span().start;

            // Check for reference: &self or &mut self
            let is_ref = if self.check(&Token::And) {
                self.advance();
                true
            } else {
                false
            };

            // Check for 'mut'
            let is_mut = if self.check(&Token::Mut) {
                self.advance();
                true
            } else {
                false
            };

            // Check for 'self' keyword
            let name = if self.check(&Token::SelfLower) {
                self.advance();
                "self".into()
            } else {
                self.expect_identifier()?
            };

            // 'self' doesn't need a type annotation, others do
            let ty = if name.as_str() == "self" {
                // For 'self', create appropriate type based on reference and mutability
                if is_ref {
                    Type::Ref(Box::new(Type::Path(Path {
                        segments: vec![PathSegment {
                            ident: "Self".into(),
                            generics: Vec::new(),
                        }],
                    })), is_mut)
                } else {
                    Type::Path(Path {
                        segments: vec![PathSegment {
                            ident: "Self".into(),
                            generics: Vec::new(),
                        }],
                    })
                }
            } else {
                self.expect(&Token::Colon)?;
                self.parse_type()?
            };

            let param_end = self.current_span().end;

            params.push(Param {
                name,
                ty,
                is_mut,
                span: param_start..param_end,
            });

            if self.check(&Token::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        Ok(params)
    }

    /// Parse a struct definition
    pub fn parse_struct(&mut self, visibility: Visibility) -> Result<StructDef, ParseError> {
        let start = self.current_span().start;

        self.expect(&Token::Struct)?;
        let name = self.expect_identifier()?;

        // Parse generics
        let generics = if self.check(&Token::Lt) {
            self.parse_generic_params()?
        } else {
            Vec::new()
        };

        // Parse fields
        self.expect(&Token::LBrace)?;
        let fields = self.parse_struct_fields()?;
        self.expect(&Token::RBrace)?;

        let end = self.current_span().end;

        Ok(StructDef {
            name,
            fields,
            generics,
            visibility,
            span: start..end,
        })
    }

    /// Parse struct fields
    fn parse_struct_fields(&mut self) -> Result<Vec<FieldDef>, ParseError> {
        let mut fields = Vec::new();

        while !self.check(&Token::RBrace) {
            // Check if we've reached the end
            if self.check(&Token::RBrace) {
                break;
            }

            // Check for visibility
            let field_visibility = if self.check(&Token::Pub) {
                self.advance();
                Visibility::Public
            } else {
                Visibility::Private
            };

            let name = self.expect_identifier()?;
            self.expect(&Token::Colon)?;
            let ty = self.parse_type()?;

            // Optional default value
            let default = if self.check(&Token::Eq) {
                self.advance();
                Some(self.parse_expr()?)
            } else {
                None
            };

            fields.push(FieldDef {
                name,
                ty,
                visibility: field_visibility,
                default,
            });

            if self.check(&Token::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        Ok(fields)
    }

    /// Parse an enum definition
    pub fn parse_enum(&mut self, visibility: Visibility) -> Result<EnumDef, ParseError> {
        let start = self.current_span().start;

        self.expect(&Token::Enum)?;
        let name = self.expect_identifier()?;

        // Parse generics
        let generics = if self.check(&Token::Lt) {
            self.parse_generic_params()?
        } else {
            Vec::new()
        };

        // Parse variants
        self.expect(&Token::LBrace)?;
        let variants = self.parse_enum_variants()?;
        self.expect(&Token::RBrace)?;

        let end = self.current_span().end;

        Ok(EnumDef {
            name,
            variants,
            generics,
            visibility,
            span: start..end,
        })
    }

    /// Parse enum variants
    fn parse_enum_variants(&mut self) -> Result<Vec<EnumVariant>, ParseError> {
        let mut variants = Vec::new();

        while !self.check(&Token::RBrace) {
            let name = self.expect_identifier()?;

            // Parse variant fields
            let fields = if self.check(&Token::LBrace) {
                // Struct variant
                self.advance();
                let fields = self.parse_struct_fields()?;
                self.expect(&Token::RBrace)?;
                EnumVariantFields::Struct(fields)
            } else if self.check(&Token::LParen) {
                // Tuple variant
                self.advance();
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
                EnumVariantFields::Tuple(types)
            } else {
                EnumVariantFields::Unit
            };

            // Optional discriminant
            let discriminant = if self.check(&Token::Eq) {
                self.advance();
                Some(self.parse_expr()?)
            } else {
                None
            };

            variants.push(EnumVariant {
                name,
                fields,
                discriminant,
            });

            if self.check(&Token::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        Ok(variants)
    }

    /// Parse an impl block
    pub fn parse_impl(&mut self) -> Result<ImplBlock, ParseError> {
        let start = self.current_span().start;

        self.expect(&Token::Impl)?;

        // Parse generics
        let generics = if self.check(&Token::Lt) {
            self.parse_generic_params()?
        } else {
            Vec::new()
        };

        // Check for trait impl: impl Trait for Type
        let trait_ = if self
            .peek_ahead(1)
            .map(|t| *t == Token::For)
            .unwrap_or(false)
        {
            let path = self.parse_path()?;
            self.expect(&Token::For)?;
            Some(path)
        } else {
            None
        };

        let ty = self.parse_type()?;

        self.expect(&Token::LBrace)?;
        let items = self.parse_impl_items()?;
        self.expect(&Token::RBrace)?;

        let end = self.current_span().end;

        Ok(ImplBlock {
            trait_,
            ty,
            items,
            generics,
            is_unsafe: false,
            span: start..end,
        })
    }

    /// Parse impl items
    fn parse_impl_items(&mut self) -> Result<Vec<ImplItem>, ParseError> {
        let mut items = Vec::new();

        while !self.check(&Token::RBrace) {
            if self.check(&Token::RBrace) {
                break;
            }

            // Check for visibility
            let visibility = if self.check(&Token::Pub) {
                self.advance();
                Visibility::Public
            } else {
                Visibility::Private
            };

            let item = match self.peek() {
                Some(Token::Fn) => ImplItem::Function(self.parse_function(visibility)?),
                Some(Token::Const) => ImplItem::Const(self.parse_const(visibility)?),
                Some(Token::Type) => ImplItem::Type(self.parse_type_alias(visibility)?),
                Some(token) => {
                    return Err(self
                        .error_unexpected("impl item (fn, const, type)", &format!("{:?}", token)))
                }
                None => return Err(ParseError::UnexpectedEof),
            };

            items.push(item);
        }

        Ok(items)
    }

    /// Parse a trait definition
    pub fn parse_trait(&mut self, visibility: Visibility) -> Result<TraitDef, ParseError> {
        let start = self.current_span().start;

        self.expect(&Token::Trait)?;
        let name = self.expect_identifier()?;

        // Parse generics
        let generics = if self.check(&Token::Lt) {
            self.parse_generic_params()?
        } else {
            Vec::new()
        };

        // Parse super traits: trait Foo: Bar + Baz
        let super_traits = if self.check(&Token::Colon) {
            self.advance();
            self.parse_trait_bounds()?
        } else {
            Vec::new()
        };

        self.expect(&Token::LBrace)?;
        let items = self.parse_trait_items()?;
        self.expect(&Token::RBrace)?;

        let end = self.current_span().end;

        Ok(TraitDef {
            name,
            items,
            generics,
            super_traits,
            visibility,
            is_unsafe: false,
            span: start..end,
        })
    }

    /// Parse trait items
    fn parse_trait_items(&mut self) -> Result<Vec<TraitItem>, ParseError> {
        let mut items = Vec::new();

        while !self.check(&Token::RBrace) {
            let item = match self.peek() {
                Some(Token::Fn) => TraitItem::Function(self.parse_trait_function()?),
                Some(Token::Const) => TraitItem::Const(self.parse_const(Visibility::Private)?),
                Some(Token::Type) => TraitItem::Type(self.parse_type_alias(Visibility::Private)?),
                Some(token) => {
                    return Err(self
                        .error_unexpected("trait item (fn, const, type)", &format!("{:?}", token)))
                }
                None => return Err(ParseError::UnexpectedEof),
            };

            items.push(item);
        }

        Ok(items)
    }

    /// Parse a trait function (without body, or with default body)
    fn parse_trait_function(&mut self) -> Result<TraitFunction, ParseError> {
        self.expect(&Token::Fn)?;
        let name = self.expect_identifier()?;

        // Parse generics
        let generics = if self.check(&Token::Lt) {
            self.parse_generic_params()?
        } else {
            Vec::new()
        };

        // Parse parameters
        self.expect(&Token::LParen)?;
        let params = self.parse_function_params()?;
        self.expect(&Token::RParen)?;

        // Parse return type
        let return_type = if self.check(&Token::Arrow) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        // Optional default body
        let default = if self.check(&Token::LBrace) {
            Some(self.parse_block()?)
        } else {
            self.expect(&Token::Semi)?;
            None
        };

        Ok(TraitFunction {
            name,
            params,
            return_type,
            default,
            generics,
        })
    }

    /// Parse an import statement (supports both 'import' and 'use' keywords)
    pub fn parse_import(&mut self) -> Result<Import, ParseError> {
        let start = self.current_span().start;

        // Accept both 'import' and 'use' keywords
        if self.check(&Token::Import) {
            self.advance();
        } else if self.check(&Token::Use) {
            self.advance();
        } else {
            return Err(self.error_unexpected("import or use", &format!("{:?}", self.peek())));
        }
        let path = self.parse_path()?;

        // Check for glob: import std::io::*; or use std::io::*;
        let is_glob = if self.check(&Token::Star) {
            self.advance();
            true
        } else {
            false
        };

        // Check for alias: import std::io as io; or use std::io as io;
        let alias = if self.check(&Token::As) {
            self.advance();
            Some(self.expect_identifier()?)
        } else {
            None
        };

        self.expect(&Token::Semi)?;

        let end = self.current_span().end;

        Ok(Import {
            path,
            alias,
            is_glob,
            span: start..end,
        })
    }

    /// Parse a const definition
    pub fn parse_const(&mut self, visibility: Visibility) -> Result<ConstDef, ParseError> {
        let start = self.current_span().start;

        self.expect(&Token::Const)?;
        let name = self.expect_identifier()?;
        self.expect(&Token::Colon)?;
        let ty = self.parse_type()?;
        self.expect(&Token::Eq)?;
        let value = self.parse_expr()?;
        self.expect(&Token::Semi)?;

        let end = self.current_span().end;

        Ok(ConstDef {
            name,
            ty,
            value,
            visibility,
            span: start..end,
        })
    }

    /// Parse a static definition
    pub fn parse_static(&mut self, visibility: Visibility) -> Result<StaticDef, ParseError> {
        let start = self.current_span().start;

        self.expect(&Token::Static)?;

        // Check for 'mut'
        let is_mut = if self.check(&Token::Mut) {
            self.advance();
            true
        } else {
            false
        };

        let name = self.expect_identifier()?;
        self.expect(&Token::Colon)?;
        let ty = self.parse_type()?;
        self.expect(&Token::Eq)?;
        let value = self.parse_expr()?;
        self.expect(&Token::Semi)?;

        let end = self.current_span().end;

        Ok(StaticDef {
            name,
            ty,
            value,
            is_mut,
            visibility,
            span: start..end,
        })
    }

    /// Parse a type alias
    pub fn parse_type_alias(&mut self, visibility: Visibility) -> Result<TypeAlias, ParseError> {
        let start = self.current_span().start;

        self.expect(&Token::Type)?;
        let name = self.expect_identifier()?;

        // Parse generics
        let generics = if self.check(&Token::Lt) {
            self.parse_generic_params()?
        } else {
            Vec::new()
        };

        self.expect(&Token::Eq)?;
        let ty = self.parse_type()?;
        self.expect(&Token::Semi)?;

        let end = self.current_span().end;

        Ok(TypeAlias {
            name,
            ty,
            generics,
            visibility,
            span: start..end,
        })
    }

    /// Parse a submodule
    pub fn parse_submodule(&mut self, visibility: Visibility) -> Result<SubModule, ParseError> {
        let start = self.current_span().start;

        self.expect(&Token::Mod)?;
        let name = self.expect_identifier()?;

        // Inline module: mod foo { ... }
        // Or file module: mod foo;
        let (items, is_inline) = if self.check(&Token::LBrace) {
            self.advance();
            let mut items = Vec::new();
            while !self.check(&Token::RBrace) {
                items.push(self.parse_item()?);
            }
            self.expect(&Token::RBrace)?;
            (items, true)
        } else {
            self.expect(&Token::Semi)?;
            (Vec::new(), false)
        };

        let end = self.current_span().end;

        Ok(SubModule {
            name,
            items,
            visibility,
            is_inline,
            span: start..end,
        })
    }

    /// Parse an extern block or extern function declaration
    /// Supports both:
    /// - Block form: extern "C" { fn foo(); }
    /// - Single function form: extern "builtin" fn foo();
    pub fn parse_extern_block(&mut self) -> Result<ExternBlock, ParseError> {
        let start = self.current_span().start;

        self.expect(&Token::Extern)?;

        // Parse ABI string: extern "C" { ... } or extern "builtin" fn ...
        let abi = if let Some(Token::StringLit(s)) = self.peek() {
            let s = s.clone();
            self.advance();
            match s {
                crate::frontend::lexer::token::StringLitKind::Simple(s) => s,
                crate::frontend::lexer::token::StringLitKind::Interpolated(s) => s,
            }
        } else {
            "C".to_string()
        };

        // Check if this is a single function declaration: extern "abi" fn foo();
        // or a block: extern "abi" { fn foo(); }
        if self.check(&Token::Fn) {
            // Single function declaration
            let func = self.parse_function(Visibility::Private)?;
            let end = self.current_span().end;
            Ok(ExternBlock {
                abi,
                items: vec![ExternItem::Function(func)],
                span: start..end,
            })
        } else {
            // Block form
            self.expect(&Token::LBrace)?;
            let items = self.parse_extern_items()?;
            self.expect(&Token::RBrace)?;

            let end = self.current_span().end;

            Ok(ExternBlock {
                abi,
                items,
                span: start..end,
            })
        }
    }

    /// Parse extern items
    fn parse_extern_items(&mut self) -> Result<Vec<ExternItem>, ParseError> {
        let mut items = Vec::new();

        while !self.check(&Token::RBrace) {
            let item = match self.peek() {
                Some(Token::Fn) => ExternItem::Function(self.parse_function(Visibility::Private)?),
                Some(Token::Static) => ExternItem::Static(self.parse_static(Visibility::Private)?),
                Some(Token::Type) => ExternItem::Type(self.parse_extern_type(Visibility::Private)?),
                Some(token) => {
                    return Err(
                        self.error_unexpected("extern item (fn, static, type)", &format!("{:?}", token))
                    )
                }
                None => return Err(ParseError::UnexpectedEof),
            };

            items.push(item);
        }

        Ok(items)
    }

    /// Parse an extern type declaration
    fn parse_extern_type(&mut self, visibility: Visibility) -> Result<ExternType, ParseError> {
        let start = self.current_span().start;
        
        self.expect(&Token::Type)?;
        let name = self.expect_identifier()?;
        self.expect(&Token::Semi)?;
        
        let end = self.current_span().end;
        
        Ok(ExternType {
            name,
            visibility,
            span: start..end,
        })
    }

    /// Parse a callback definition
    /// Syntax: callback "abi" fn name(params) -> return_type;
    pub fn parse_callback(&mut self, visibility: Visibility) -> Result<CallbackDef, ParseError> {
        let start = self.current_span().start;
        
        self.expect(&Token::Callback)?;
        
        // Parse optional ABI string
        let abi = if let Some(Token::StringLit(s)) = self.peek() {
            let s = s.clone();
            self.advance();
            match s {
                crate::frontend::lexer::token::StringLitKind::Simple(s) => s,
                crate::frontend::lexer::token::StringLitKind::Interpolated(s) => s,
            }
        } else {
            "C".to_string()
        };
        
        self.expect(&Token::Fn)?;
        let name = self.expect_identifier()?;
        
        // Parse parameters
        self.expect(&Token::LParen)?;
        let params = self.parse_function_params()?;
        self.expect(&Token::RParen)?;
        
        // Parse return type
        let return_type = if self.check(&Token::Arrow) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };
        
        self.expect(&Token::Semi)?;
        
        let end = self.current_span().end;
        
        Ok(CallbackDef {
            name,
            params,
            return_type,
            abi,
            visibility,
            span: start..end,
        })
    }

    /// Parse a safe wrapper definition
    /// Syntax: safe fn wrapper_name(extern_name: extern "abi" fn(params) -> ret) -> Result<T, E>;
    pub fn parse_safe_wrapper(&mut self, visibility: Visibility) -> Result<SafeWrapper, ParseError> {
        let start = self.current_span().start;
        
        self.expect(&Token::Safe)?;
        self.expect(&Token::Fn)?;
        
        let wrapper_name = self.expect_identifier()?;
        
        // Parse parameters
        self.expect(&Token::LParen)?;
        let params = self.parse_function_params()?;
        self.expect(&Token::RParen)?;
        
        // Parse return type
        let return_type = if self.check(&Token::Arrow) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };
        
        // Parse error type (optional)
        let error_type = if self.check(&Token::Not) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };
        
        self.expect(&Token::Semi)?;
        
        let end = self.current_span().end;
        
        // Extract extern name from first param if it's an extern fn
        let extern_name = params.first()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "extern_fn".into());
        
        Ok(SafeWrapper {
            extern_name,
            wrapper_name,
            params,
            return_type,
            error_type,
            abi: "C".to_string(),
            visibility,
            span: start..end,
        })
    }

    /// Parse FFI attributes
    fn parse_ffi_attrs(&mut self) -> Result<FfiAttributes, ParseError> {
        let mut attrs = FfiAttributes::default();
        
        // Parse attributes like #[link_name = "name"], #[link_section = "section"]
        while self.check(&Token::Hash) {
            self.advance();
            self.expect(&Token::LBrace)?;
            
            let attr_name = self.expect_identifier()?;
            
            match attr_name.as_str() {
                "link_name" => {
                    self.expect(&Token::Eq)?;
                    if let Some(Token::StringLit(s)) = self.peek() {
                        let s = s.clone();
                        self.advance();
                        attrs.link_name = Some(match s {
                            crate::frontend::lexer::token::StringLitKind::Simple(s) => s,
                            crate::frontend::lexer::token::StringLitKind::Interpolated(s) => s,
                        });
                    }
                }
                "link_section" => {
                    self.expect(&Token::Eq)?;
                    if let Some(Token::StringLit(s)) = self.peek() {
                        let s = s.clone();
                        self.advance();
                        attrs.link_section = Some(match s {
                            crate::frontend::lexer::token::StringLitKind::Simple(s) => s,
                            crate::frontend::lexer::token::StringLitKind::Interpolated(s) => s,
                        });
                    }
                }
                "callback" => {
                    attrs.is_callback = true;
                }
                "safe" => {
                    attrs.safe_wrapper = true;
                }
                "deprecated" => {
                    attrs.deprecated = true;
                    if self.check(&Token::Eq) {
                        self.advance();
                        if let Some(Token::StringLit(s)) = self.peek() {
                            let s = s.clone();
                            self.advance();
                            attrs.deprecated_message = Some(match s {
                                crate::frontend::lexer::token::StringLitKind::Simple(s) => s,
                                crate::frontend::lexer::token::StringLitKind::Interpolated(s) => s,
                            });
                        }
                    }
                }
                _ => {}
            }
            
            self.expect(&Token::RBrace)?;
        }
        
        Ok(attrs)
    }

    /// Parse generic parameters
    fn parse_generic_params(&mut self) -> Result<Vec<GenericParam>, ParseError> {
        self.expect(&Token::Lt)?;

        let mut params = Vec::new();

        while !self.check(&Token::Gt) {
            let name = self.expect_identifier()?;

            // Parse bounds: T: Clone + Debug
            let bounds = if self.check(&Token::Colon) {
                self.advance();
                self.parse_trait_bounds()?
            } else {
                Vec::new()
            };

            // Parse default: T = i32
            let default = if self.check(&Token::Eq) {
                self.advance();
                Some(self.parse_type()?)
            } else {
                None
            };

            params.push(GenericParam {
                name,
                bounds,
                default,
            });

            if self.check(&Token::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        self.expect(&Token::Gt)?;

        Ok(params)
    }

    /// Parse trait bounds
    pub fn parse_trait_bounds(&mut self) -> Result<Vec<TraitBound>, ParseError> {
        let mut bounds = Vec::new();

        loop {
            let path = self.parse_path()?;

            // Parse generic arguments
            let generics = if self.check(&Token::Lt) {
                self.parse_generic_args()?
            } else {
                Vec::new()
            };

            bounds.push(TraitBound { path, generics });

            if self.check(&Token::Plus) {
                self.advance();
            } else {
                break;
            }
        }

        Ok(bounds)
    }

    /// Parse generic arguments
    pub fn parse_generic_args(&mut self) -> Result<Vec<Type>, ParseError> {
        self.expect(&Token::Lt)?;

        let mut args = Vec::new();

        while !self.check(&Token::Gt) {
            args.push(self.parse_type()?);

            if self.check(&Token::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        self.expect(&Token::Gt)?;

        Ok(args)
    }
}
