//! MIR Builder - Converts AST to MIR
//!
//! This module provides functionality to convert BoxLang AST
//! into Mid-level Intermediate Representation (MIR).

use crate::ast::*;
use crate::middle::mir::*;
use smol_str::SmolStr;

/// Helper function to create a Path from a string
fn make_path(name: &str) -> Path {
    Path {
        segments: vec![PathSegment {
            ident: Ident::new(name),
            generics: vec![],
        }],
    }
}

/// Get field index by field name
/// In a full implementation, this would look up the struct definition
/// and find the field index by name
fn get_field_index_by_name(field_name: &Ident) -> usize {
    match field_name.as_str() {
        "x" | "re" => 0,
        "y" | "im" => 1,
        "z" => 2,
        "w" => 3,
        _ => 0, // Default to first field
    }
}

/// Builder for constructing MIR from AST
pub struct MirBuilder {
    /// The MIR body being constructed
    body: MirBody,
    /// Current basic block
    current_block: BasicBlock,
    /// Local counter for generating unique locals
    local_counter: u32,
    /// Block counter for generating unique blocks
    block_counter: u32,
}

impl MirBuilder {
    /// Create a new MIR builder
    pub fn new(arg_count: usize, span: Span) -> Self {
        let mut body = MirBody::new(arg_count, span.clone());

        // Create return local (_0)
        body.push_local(LocalDecl::new(Type::Unit, span.clone()));

        // Create argument locals (_1, _2, ...)
        for _ in 0..arg_count {
            body.push_local(LocalDecl::new(Type::Unit, span.clone()).arg());
        }

        // Create entry block
        let entry_block = body.push_block(BasicBlockData::new());

        Self {
            body,
            current_block: entry_block,
            local_counter: (arg_count + 1) as u32,
            block_counter: 1,
        }
    }

    /// Build a function into MIR
    pub fn build_function(&mut self, func: &Function) -> &MirBody {
        // Set parameter names and types
        for (i, param) in func.params.iter().enumerate() {
            let local_idx = i + 1;
            if local_idx < self.body.local_decls.len() {
                self.body.local_decls[local_idx].name = Some(param.name.as_str().into());
                self.body.local_decls[local_idx].ty = param.ty.clone();
            }
        }

        // Build the function body
        self.build_block(&func.body);

        // Ensure the function has a return terminator
        self.ensure_return();

        &self.body
    }

    /// Build a block into MIR
    fn build_block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.build_stmt(stmt);
        }
    }

    /// Build a statement into MIR
    fn build_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let(let_stmt) => {
                // Create a local for the variable
                let ty = let_stmt.ty.clone().unwrap_or(Type::Unit);
                let local = self.new_local(ty);
                let place = Place::from_local(local);

                // Store the variable name
                self.body.local_decls[local.0 as usize].name = Some(let_stmt.name.as_str().into());

                // Build the initializer expression
                if let Some(init) = &let_stmt.init {
                    let operand = self.build_expr(init);
                    self.push_stmt(Statement::Assign(place, Rvalue::Use(operand)));
                }
            }
            Stmt::Expr(expr) => {
                // Build the expression
                self.build_expr(expr);
            }
            Stmt::Item(item) => {
                // Handle nested items (functions, etc.)
                match item {
                    Item::Function(func) => {
                        // For now, just build the function inline
                        // In a real implementation, this would be a separate function
                        self.build_block(&func.body);
                    }
                    _ => {
                        // Other items are handled at module level
                    }
                }
            }
        }
    }

    /// Build an expression into MIR
    fn build_expr(&mut self, expr: &Expr) -> Operand {
        match expr {
            Expr::Literal(lit) => self.build_literal(lit),
            Expr::Ident(ident) => self.build_ident(ident),
            Expr::Path(path) => self.build_path(path),
            Expr::PathCall(path, args) => self.build_path_call(path, args),
            Expr::Binary(binary) => self.build_binary(binary),
            Expr::Unary(unary) => self.build_unary(unary),
            Expr::Call(call) => self.build_call(call),
            Expr::Block(block) => {
                self.build_block(block);
                Operand::Constant(Constant::Scalar(Scalar::Int(0, IntType::I64)))
            }
            Expr::If(if_expr) => self.build_if(if_expr),
            Expr::While(while_expr) => self.build_while(while_expr),
            Expr::Loop(loop_expr) => self.build_loop(loop_expr),
            Expr::For(for_expr) => self.build_for(for_expr),
            Expr::Return(ret) => self.build_return(ret),
            Expr::Assign(assign) => self.build_assign(assign),
            Expr::FieldAccess(field) => self.build_field_access(field),
            Expr::StructInit(struct_init) => self.build_struct_init(struct_init),
            Expr::ArrayInit(array_init) => self.build_array_init(array_init),
            Expr::Index(index) => self.build_index(index),
            Expr::MethodCall(method) => self.build_method_call(method),
            Expr::Closure(closure) => self.build_closure(closure),
            Expr::Match(match_expr) => self.build_match(match_expr),
            Expr::Break(_) => {
                // For now, just return a dummy operand
                // In a full implementation, this would jump to the loop end block
                Operand::Constant(Constant::Scalar(Scalar::Int(0, IntType::I64)))
            }
            Expr::Continue => {
                // For now, just return a dummy operand
                // In a full implementation, this would jump to the loop start block
                Operand::Constant(Constant::Scalar(Scalar::Int(0, IntType::I64)))
            }
            Expr::Async(block) => self.build_async(block),
            Expr::Await(expr) => self.build_await(expr),
            Expr::Unsafe(block) => {
                self.build_block(block);
                Operand::Constant(Constant::Scalar(Scalar::Int(0, IntType::I64)))
            }
            _ => {
                // Unsupported expression - return unit
                Operand::Constant(Constant::Scalar(Scalar::Int(0, IntType::I64)))
            }
        }
    }

    /// Build a path expression (e.g., Result::Ok, Option::Some)
    fn build_path(&mut self, path: &Path) -> Operand {
        // For simple paths, treat as identifier
        if path.segments.len() == 1 {
            return self.build_ident(&path.segments[0].ident);
        }
        // For qualified paths like Result::Ok, return a constant
        Operand::Constant(Constant::Scalar(Scalar::Int(0, IntType::I64)))
    }

    /// Build a path call expression (e.g., Result::Ok(value))
    fn build_path_call(&mut self, path: &Path, args: &[Expr]) -> Operand {
        // Build arguments
        let _args: Vec<Operand> = args.iter().map(|a| self.build_expr(a)).collect();
        // Return a placeholder for now
        Operand::Constant(Constant::Scalar(Scalar::Int(0, IntType::I64)))
    }

    /// Build a literal expression
    fn build_literal(&mut self, lit: &Literal) -> Operand {
        let scalar = match lit {
            Literal::Integer(n) => Scalar::Int((*n).into(), IntType::I64),
            Literal::Float(f) => Scalar::Float(*f, FloatType::F64),
            Literal::Bool(b) => Scalar::Int(if *b { 1 } else { 0 }, IntType::I8),
            Literal::Char(c) => Scalar::Int((*c as i128).into(), IntType::I32),
            Literal::String(_) => {
                // Strings are more complex - for now return a placeholder
                return Operand::Constant(Constant::ZST);
            }
            Literal::Null => Scalar::Int(0, IntType::I64),
        };
        Operand::Constant(Constant::Scalar(scalar))
    }

    /// Build an identifier expression
    fn build_ident(&mut self, ident: &Ident) -> Operand {
        // Look up the local variable
        for (i, decl) in self.body.local_decls.iter().enumerate() {
            if let Some(ref name) = decl.name {
                if name == ident {
                    return Operand::Copy(Place::from_local(Local(i as u32)));
                }
            }
        }
        // If not found, return a constant (for global constants)
        Operand::Constant(Constant::Scalar(Scalar::Int(0, IntType::I64)))
    }

    /// Build a binary expression
    fn build_binary(&mut self, binary: &BinaryExpr) -> Operand {
        let left = self.build_expr(&binary.left);
        let right = self.build_expr(&binary.right);

        // Create a temporary local for the result
        let result_local = self.new_local(Type::Unit);
        let result_place = Place::from_local(result_local);

        // Map the binary operator
        let op = match binary.op {
            BinaryOp::Add => BinOp::Add,
            BinaryOp::Sub => BinOp::Sub,
            BinaryOp::Mul => BinOp::Mul,
            BinaryOp::Div => BinOp::Div,
            BinaryOp::Rem => BinOp::Rem,
            BinaryOp::And => BinOp::BitAnd,
            BinaryOp::Or => BinOp::BitOr,
            BinaryOp::Xor => BinOp::BitXor,
            BinaryOp::Shl => BinOp::Shl,
            BinaryOp::Shr => BinOp::Shr,
            BinaryOp::Eq => BinOp::Eq,
            BinaryOp::Ne => BinOp::Ne,
            BinaryOp::Lt => BinOp::Lt,
            BinaryOp::Le => BinOp::Le,
            BinaryOp::Gt => BinOp::Gt,
            BinaryOp::Ge => BinOp::Ge,
            BinaryOp::LogicalAnd => BinOp::And,
            BinaryOp::LogicalOr => BinOp::Or,
            BinaryOp::Assign => {
                // Assignment is handled specially
                return left;
            }
            BinaryOp::Pipe => {
                // Pipeline operator - for now just return left
                return left;
            }
        };

        // Create the binary operation
        self.push_stmt(Statement::Assign(
            result_place.clone(),
            Rvalue::BinaryOp(op, Box::new(left), Box::new(right)),
        ));

        Operand::Copy(result_place)
    }

    /// Build a unary expression
    fn build_unary(&mut self, unary: &UnaryExpr) -> Operand {
        let expr = self.build_expr(&unary.expr);

        // Create a temporary local for the result
        let result_local = self.new_local(Type::Unit);
        let result_place = Place::from_local(result_local);

        // Map the unary operator
        let op = match unary.op {
            UnaryOp::Neg => UnOp::Neg,
            UnaryOp::Not => UnOp::Not,
            UnaryOp::Deref => {
                // Dereference - for now just return the expression
                return expr;
            }
            UnaryOp::Ref => {
                // Reference - for now just return the expression
                return expr;
            }
            UnaryOp::RefMut => {
                // Mutable reference - for now just return the expression
                return expr;
            }
        };

        // Create the unary operation
        self.push_stmt(Statement::Assign(
            result_place.clone(),
            Rvalue::UnaryOp(op, Box::new(expr)),
        ));

        Operand::Copy(result_place)
    }

    /// Build a function call expression
    fn build_call(&mut self, call: &CallExpr) -> Operand {
        // Build the function operand
        let func = self.build_expr(&call.func);

        // Build the argument operands
        let args: Vec<Operand> = call.args.iter().map(|arg| self.build_expr(arg)).collect();

        // Create a temporary local for the return value
        let result_local = self.new_local(Type::Unit);
        let result_place = Place::from_local(result_local);

        // Create the call terminator
        let call_terminator = Terminator {
            kind: TerminatorKind::Call {
                func,
                args,
                destination: result_place.clone(),
                target: None,
            },
            span: 0..0,
        };

        self.set_terminator(call_terminator);

        // Create a new block for after the call
        let after_call = self.new_block();
        self.current_block = after_call;

        Operand::Copy(result_place)
    }

    /// Build an if expression
    fn build_if(&mut self, if_expr: &IfExpr) -> Operand {
        // Build the condition
        let cond = self.build_expr(&if_expr.cond);

        // Create blocks for then, else, and end
        let then_block = self.new_block();
        let else_block = self.new_block();
        let end_block = self.new_block();

        // Create the switch terminator
        // The switch type is the condition's type (typically bool or integer)
        let switch = Terminator {
            kind: TerminatorKind::SwitchInt {
                discr: cond,
                switch_ty: Type::Path(Path {
                    segments: vec![PathSegment {
                        ident: "bool".into(),
                        generics: vec![],
                    }],
                }), // Condition type is boolean for if expressions
                targets: vec![(1, then_block)],
                otherwise: else_block,
            },
            span: 0..0,
        };
        self.set_terminator(switch);

        // Build the then block
        self.current_block = then_block;
        self.build_block(&if_expr.then_branch);
        let goto_end = Terminator {
            kind: TerminatorKind::Goto { target: end_block },
            span: 0..0,
        };
        self.set_terminator(goto_end);

        // Build the else block
        self.current_block = else_block;
        if let Some(else_branch) = &if_expr.else_branch {
            self.build_expr(else_branch);
        }
        let goto_end = Terminator {
            kind: TerminatorKind::Goto { target: end_block },
            span: 0..0,
        };
        self.set_terminator(goto_end);

        // Switch to end block
        self.current_block = end_block;

        Operand::Constant(Constant::Scalar(Scalar::Int(0, IntType::I64)))
    }

    /// Build a while expression
    fn build_while(&mut self, while_expr: &WhileExpr) -> Operand {
        // Create blocks for condition, body, and end
        let cond_block = self.new_block();
        let body_block = self.new_block();
        let end_block = self.new_block();

        // Jump to condition block
        let goto_cond = Terminator {
            kind: TerminatorKind::Goto { target: cond_block },
            span: 0..0,
        };
        self.set_terminator(goto_cond);

        // Build condition block
        self.current_block = cond_block;
        let cond = self.build_expr(&while_expr.cond);
        let switch = Terminator {
            kind: TerminatorKind::SwitchInt {
                discr: cond,
                switch_ty: Type::Path(Path {
                    segments: vec![PathSegment {
                        ident: "bool".into(),
                        generics: vec![],
                    }],
                }), // Condition type is boolean for while expressions
                targets: vec![(1, body_block)],
                otherwise: end_block,
            },
            span: 0..0,
        };
        self.set_terminator(switch);

        // Build body block
        self.current_block = body_block;
        self.build_block(&while_expr.body);
        let goto_cond_again = Terminator {
            kind: TerminatorKind::Goto { target: cond_block },
            span: 0..0,
        };
        self.set_terminator(goto_cond_again);

        // Switch to end block
        self.current_block = end_block;

        Operand::Constant(Constant::Scalar(Scalar::Int(0, IntType::I64)))
    }

    /// Build a loop expression
    fn build_loop(&mut self, loop_expr: &LoopExpr) -> Operand {
        // Create blocks for body and end
        let body_block = self.new_block();
        let end_block = self.new_block();

        // Jump to body block
        let goto_body = Terminator {
            kind: TerminatorKind::Goto { target: body_block },
            span: 0..0,
        };
        self.set_terminator(goto_body);

        // Build body block
        self.current_block = body_block;
        self.build_block(&loop_expr.body);
        let goto_body_again = Terminator {
            kind: TerminatorKind::Goto { target: body_block },
            span: 0..0,
        };
        self.set_terminator(goto_body_again);

        // Switch to end block (unreachable in infinite loop)
        self.current_block = end_block;

        Operand::Constant(Constant::Scalar(Scalar::Int(0, IntType::I64)))
    }

    /// Build a for expression
    fn build_for(&mut self, for_expr: &ForExpr) -> Operand {
        // Get the loop variable name from pattern
        let var_name: SmolStr = match &for_expr.pattern {
            Pattern::Ident(ident) => ident.as_str().into(),
            Pattern::Binding(ident, _) => ident.as_str().into(),
            _ => "i".into(), // Default to "i" for other patterns
        };

        // Handle range expression
        if let Expr::Range(range_expr) = &*for_expr.expr {
            // Create blocks for init, condition, body, and end
            let _init_block = self.current_block;
            let cond_block = self.new_block();
            let body_block = self.new_block();
            let end_block = self.new_block();

            // Create i64 type for loop variable
            let i64_ty = Type::Path(make_path("i64"));
            let bool_ty = Type::Path(make_path("bool"));

            // Build init block: initialize loop variable
            let loop_var = self.new_local(i64_ty.clone());
            let loop_var_place = Place::from_local(loop_var);
            self.body.local_decls[loop_var.0 as usize].name = Some(var_name);

            // Initialize with start value (default to 0 if not specified)
            let start_val = if let Some(start) = &range_expr.start {
                self.build_expr(start)
            } else {
                Operand::Constant(Constant::Scalar(Scalar::Int(0, IntType::I64)))
            };
            self.push_stmt(Statement::Assign(
                loop_var_place.clone(),
                Rvalue::Use(start_val),
            ));

            // Jump to condition block
            let goto_cond = Terminator {
                kind: TerminatorKind::Goto { target: cond_block },
                span: 0..0,
            };
            self.set_terminator(goto_cond);

            // Build condition block: check if loop variable < end
            self.current_block = cond_block;
            let loop_var_op = Operand::Copy(loop_var_place.clone());
            let end_val = if let Some(end) = &range_expr.end {
                self.build_expr(end)
            } else {
                Operand::Constant(Constant::Scalar(Scalar::Int(0, IntType::I64)))
            };

            // Create comparison: loop_var < end
            let cmp_local = self.new_local(bool_ty.clone());
            let cmp_place = Place::from_local(cmp_local);
            let cmp_op = if range_expr.inclusive {
                BinOp::Le
            } else {
                BinOp::Lt
            };
            self.push_stmt(Statement::Assign(
                cmp_place.clone(),
                Rvalue::BinaryOp(cmp_op, Box::new(loop_var_op.clone()), Box::new(end_val)),
            ));

            let cond_op = Operand::Copy(cmp_place);
            let switch = Terminator {
                kind: TerminatorKind::SwitchInt {
                    discr: cond_op,
                    switch_ty: bool_ty,
                    targets: vec![(1, body_block)],
                    otherwise: end_block,
                },
                span: 0..0,
            };
            self.set_terminator(switch);

            // Build body block
            self.current_block = body_block;
            self.build_block(&for_expr.body);

            // Increment loop variable
            let one = Operand::Constant(Constant::Scalar(Scalar::Int(1, IntType::I64)));
            let inc_local = self.new_local(i64_ty);
            let inc_place = Place::from_local(inc_local);
            self.push_stmt(Statement::Assign(
                inc_place.clone(),
                Rvalue::BinaryOp(BinOp::Add, Box::new(loop_var_op), Box::new(one)),
            ));
            self.push_stmt(Statement::Assign(
                loop_var_place,
                Rvalue::Use(Operand::Copy(inc_place)),
            ));

            // Jump back to condition
            let goto_cond_again = Terminator {
                kind: TerminatorKind::Goto { target: cond_block },
                span: 0..0,
            };
            self.set_terminator(goto_cond_again);

            // Switch to end block
            self.current_block = end_block;
        } else {
            // For non-range expressions, just execute body once as placeholder
            self.build_block(&for_expr.body);
        }

        Operand::Constant(Constant::Scalar(Scalar::Int(0, IntType::I64)))
    }

    /// Build an async block expression
    fn build_async(&mut self, block: &Block) -> Operand {
        // For now, async blocks are compiled as regular blocks
        // In a full implementation, this would create a future/generator
        // that can be polled

        // Build the block normally
        self.build_block(block);

        // Return a placeholder - in full implementation would return Future
        Operand::Constant(Constant::Scalar(Scalar::Int(0, IntType::I64)))
    }

    /// Build an await expression
    fn build_await(&mut self, expr: &Expr) -> Operand {
        // For now, await just evaluates the expression
        // In a full implementation, this would:
        // 1. Check if the expression is a Future
        // 2. If not ready, yield control back to the executor
        // 3. When ready, return the value

        self.build_expr(expr)
    }

    /// Build a return expression
    fn build_return(&mut self, ret: &Option<Box<Expr>>) -> Operand {
        let value = if let Some(expr) = ret {
            self.build_expr(expr)
        } else {
            Operand::Constant(Constant::Scalar(Scalar::Int(0, IntType::I64)))
        };

        // Assign to return place (_0)
        let return_place = Place::from_local(Local::RETURN_PLACE);
        self.push_stmt(Statement::Assign(return_place.clone(), Rvalue::Use(value)));

        // Create return terminator
        let return_term = Terminator {
            kind: TerminatorKind::Return,
            span: 0..0,
        };
        self.set_terminator(return_term);

        // Create a new block for unreachable code
        let unreachable_block = self.new_block();
        self.current_block = unreachable_block;

        Operand::Constant(Constant::Scalar(Scalar::Int(0, IntType::I64)))
    }

    /// Build an assignment expression
    fn build_assign(&mut self, assign: &AssignExpr) -> Operand {
        let place = self.build_place(&assign.left);
        let value = self.build_expr(&assign.right);

        self.push_stmt(Statement::Assign(place, Rvalue::Use(value)));

        Operand::Constant(Constant::Scalar(Scalar::Int(0, IntType::I64)))
    }

    /// Build a field access expression
    fn build_field_access(&mut self, field: &FieldAccessExpr) -> Operand {
        let base = self.build_expr(&field.expr);

        // Convert base operand to a place and add field projection
        match base {
            Operand::Copy(place) | Operand::Move(place) => {
                // Look up field index from the field name
                let field_idx = get_field_index_by_name(&field.field);
                let field_place = place.project(PlaceElem::Field(field_idx));
                Operand::Copy(field_place)
            }
            _ => {
                // For constants or other operands, we can't project
                // Create a temporary and project from there
                let temp_place = self.create_temp_place();
                self.push_stmt(Statement::Assign(temp_place.clone(), Rvalue::Use(base)));
                let field_idx = get_field_index_by_name(&field.field);
                let field_place = temp_place.project(PlaceElem::Field(field_idx));
                Operand::Copy(field_place)
            }
        }
    }

    /// Build a struct initialization expression
    fn build_struct_init(&mut self, struct_init: &StructInitExpr) -> Operand {
        // Build the field values
        let fields: Vec<Operand> = struct_init
            .fields
            .iter()
            .map(|(_, expr)| self.build_expr(expr))
            .collect();

        // Create a temporary local for the struct
        let result_local = self.new_local(Type::Unit);
        let result_place = Place::from_local(result_local);

        // Create the aggregate - use a placeholder name for now
        self.push_stmt(Statement::Assign(
            result_place.clone(),
            Rvalue::Aggregate(AggregateKind::Struct(SmolStr::new("anon")), fields),
        ));

        Operand::Copy(result_place)
    }

    /// Build an array initialization expression
    fn build_array_init(&mut self, array_init: &ArrayInitExpr) -> Operand {
        // Build the element values
        let elements: Vec<Operand> = array_init
            .elements
            .iter()
            .map(|expr| self.build_expr(expr))
            .collect();

        // Create a temporary local for the array
        let result_local = self.new_local(Type::Unit);
        let result_place = Place::from_local(result_local);

        // Create the aggregate - use Unit type for now
        self.push_stmt(Statement::Assign(
            result_place.clone(),
            Rvalue::Aggregate(AggregateKind::Array(Type::Unit), elements),
        ));

        Operand::Copy(result_place)
    }

    /// Build an index expression
    fn build_index(&mut self, index: &IndexExpr) -> Operand {
        let base = self.build_expr(&index.expr);
        let index_operand = self.build_expr(&index.index);

        // Convert index operand to a place and store it in a local
        let index_local = match index_operand {
            Operand::Copy(place) | Operand::Move(place) => place.local,
            _ => {
                // For constants or other cases, create a temporary local
                // Use Path type for i64
                let i64_ty = Type::Path(crate::ast::Path {
                    segments: vec![crate::ast::PathSegment {
                        ident: crate::ast::Ident::new("i64"),
                        generics: vec![],
                    }],
                });
                let local = self.new_local(i64_ty);
                let place = Place::from_local(local);
                self.push_stmt(Statement::Assign(place.clone(), Rvalue::Use(index_operand)));
                local
            }
        };

        // Convert base operand to a place and add index projection
        match base {
            Operand::Copy(place) | Operand::Move(place) => {
                let indexed_place = place.project(PlaceElem::Index(index_local));
                Operand::Copy(indexed_place)
            }
            _ => {
                // For constants or other operands, we can't project directly
                // Create a temporary to hold the base value, then index into it
                let temp_place = self.create_temp_place();
                self.push_stmt(Statement::Assign(temp_place.clone(), Rvalue::Use(base)));
                let indexed_place = temp_place.project(PlaceElem::Index(index_local));
                Operand::Copy(indexed_place)
            }
        }
    }

    /// Create a temporary place for intermediate values
    fn create_temp_place(&mut self) -> Place {
        let local = self.new_local(Type::Unit);
        Place::from_local(local)
    }

    /// Build a method call expression
    fn build_method_call(&mut self, method: &MethodCallExpr) -> Operand {
        // Build the receiver
        let receiver = self.build_expr(&method.receiver);

        // Build the arguments
        let args: Vec<Operand> = method.args.iter().map(|arg| self.build_expr(arg)).collect();

        // Create a temporary local for the return value
        let result_local = self.new_local(Type::Unit);
        let result_place = Place::from_local(result_local);

        // Create the call terminator
        let call_terminator = Terminator {
            kind: TerminatorKind::Call {
                func: receiver,
                args,
                destination: result_place.clone(),
                target: None,
            },
            span: 0..0,
        };

        self.set_terminator(call_terminator);

        // Create a new block for after the call
        let after_call = self.new_block();
        self.current_block = after_call;

        Operand::Copy(result_place)
    }

    /// Build a closure expression
    fn build_closure(&mut self, _closure: &ClosureExpr) -> Operand {
        // For now, just return a placeholder
        // Closures are complex and require capturing environment
        Operand::Constant(Constant::Scalar(Scalar::Int(0, IntType::I64)))
    }

    /// Build a match expression
    fn build_match(&mut self, match_expr: &MatchExpr) -> Operand {
        // Build the scrutinee expression
        let scrutinee = self.build_expr(&match_expr.expr);

        // Create result local
        let result_local = self.new_local(Type::Unit);
        let result_place = Place::from_local(result_local);

        // Create blocks for each arm and the end
        let end_block = self.new_block();
        let arm_blocks: Vec<BasicBlock> =
            match_expr.arms.iter().map(|_| self.new_block()).collect();

        // Get the scrutinee type to determine switch type
        let switch_ty = self.infer_expr_type(&match_expr.expr);

        // Build switch targets from literal patterns
        let mut targets: Vec<(u128, BasicBlock)> = Vec::new();
        let mut otherwise_block = end_block;

        for (i, arm) in match_expr.arms.iter().enumerate() {
            match &arm.pattern {
                Pattern::Literal(lit) => {
                    if let Some(val) = self.literal_to_u128(lit) {
                        targets.push((val, arm_blocks[i]));
                    }
                }
                Pattern::Wildcard => {
                    otherwise_block = arm_blocks[i];
                }
                _ => {
                    // For non-literal patterns, use as otherwise for now
                    otherwise_block = arm_blocks[i];
                }
            }
        }

        // Create switch terminator
        let switch_term = Terminator {
            kind: TerminatorKind::SwitchInt {
                discr: scrutinee,
                switch_ty,
                targets,
                otherwise: otherwise_block,
            },
            span: 0..0,
        };
        self.set_terminator(switch_term);

        // Build each arm body
        for (i, arm) in match_expr.arms.iter().enumerate() {
            self.current_block = arm_blocks[i];

            // Handle guard if present
            if let Some(_guard) = &arm.guard {
                // For now, guards are not fully implemented
                // In full implementation, would evaluate guard and conditionally branch
            }

            // Build arm body
            let arm_result = self.build_expr(&arm.body);

            // Store result
            self.push_stmt(Statement::Assign(
                result_place.clone(),
                Rvalue::Use(arm_result),
            ));

            // Jump to end
            let goto_end = Terminator {
                kind: TerminatorKind::Goto { target: end_block },
                span: 0..0,
            };
            self.set_terminator(goto_end);
        }

        // Set current block to end
        self.current_block = end_block;

        Operand::Copy(result_place)
    }

    /// Convert a literal to u128 for switch targets
    fn literal_to_u128(&self, lit: &Literal) -> Option<u128> {
        match lit {
            Literal::Integer(n) => Some(*n as u128),
            Literal::Bool(b) => Some(if *b { 1 } else { 0 }),
            Literal::Char(c) => Some(*c as u128),
            _ => None,
        }
    }

    /// Infer the type of an expression for switch
    fn infer_expr_type(&self, expr: &Expr) -> Type {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Integer(_) => Type::Path(make_path("i64")),
                Literal::Bool(_) => Type::Path(make_path("bool")),
                Literal::Char(_) => Type::Path(make_path("char")),
                _ => Type::Unit,
            },
            _ => Type::Unit,
        }
    }

    /// Build a place from an expression
    fn build_place(&mut self, expr: &Expr) -> Place {
        match expr {
            Expr::Ident(ident) => {
                for (i, decl) in self.body.local_decls.iter().enumerate() {
                    if let Some(ref name) = decl.name {
                        if name == ident {
                            return Place::from_local(Local(i as u32));
                        }
                    }
                }
                Place::from_local(Local(0))
            }
            Expr::Path(path) => {
                // For simple paths, treat as identifier
                if path.segments.len() == 1 {
                    let ident = &path.segments[0].ident;
                    for (i, decl) in self.body.local_decls.iter().enumerate() {
                        if let Some(ref name) = decl.name {
                            if name == ident {
                                return Place::from_local(Local(i as u32));
                            }
                        }
                    }
                }
                Place::from_local(Local(0))
            }
            _ => Place::from_local(Local(0)),
        }
    }

    /// Create a new local variable
    fn new_local(&mut self, ty: Type) -> Local {
        let local = Local(self.local_counter);
        self.local_counter += 1;
        self.body.push_local(LocalDecl::new(ty, 0..0));
        local
    }

    /// Create a new basic block
    fn new_block(&mut self) -> BasicBlock {
        let block = BasicBlock(self.block_counter);
        self.block_counter += 1;
        self.body.push_block(BasicBlockData::new());
        block
    }

    /// Push a statement to the current block
    fn push_stmt(&mut self, stmt: Statement) {
        self.body
            .basic_block_mut(self.current_block)
            .push_stmt(stmt);
    }

    /// Set the terminator for the current block
    fn set_terminator(&mut self, term: Terminator) {
        self.body
            .basic_block_mut(self.current_block)
            .set_terminator(term);
    }

    /// Ensure the function has a return terminator
    fn ensure_return(&mut self) {
        let current_block = &self.body.basic_blocks[self.current_block.0 as usize];
        if current_block.terminator.is_none() {
            // Add a return terminator
            let return_term = Terminator {
                kind: TerminatorKind::Return,
                span: 0..0,
            };
            self.set_terminator(return_term);
        }
    }
}

impl Default for MirBuilder {
    fn default() -> Self {
        Self::new(0, 0..0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_simple_function() {
        let func = Function {
            name: Ident::new("test"),
            params: vec![],
            return_type: None,
            body: Block {
                stmts: vec![],
                span: 0..0,
            },
            visibility: crate::ast::Visibility::Private,
            is_extern: false,
            is_async: false,
            is_unsafe: false,
            abi: None,
            generics: vec![],
            ffi_attrs: crate::ast::FfiAttributes::default(),
            span: 0..0,
        };

        let mut builder = MirBuilder::new(0, 0..0);
        let body = builder.build_function(&func);

        assert_eq!(body.local_decls.len(), 1); // Just return local
        assert_eq!(body.basic_blocks.len(), 1);
    }
}
