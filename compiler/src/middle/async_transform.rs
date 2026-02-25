//! Async/Await State Machine Transformation
//!
//! This module transforms async functions and blocks into state machines
//! that implement the Future trait. This is similar to how Rust compiles
//! async/await code.
//!
//! The transformation works as follows:
//! 1. Identify all await points in the async function
//! 2. Create an enum representing the state machine states
//! 3. Transform the function body into a state machine with resume points
//! 4. Generate the Future trait implementation

use crate::ast::*;
use crate::middle::mir::*;
use std::collections::HashMap;

/// Represents a state in the async state machine
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AsyncState {
    /// Initial state (start of the function)
    Start,
    /// State at a specific await point (identified by index)
    Awaiting(u32),
    /// Completed state
    Completed,
    /// Panicked state
    Panicked,
}

/// Information about an await point
#[derive(Debug, Clone)]
pub struct AwaitPoint {
    /// Unique identifier for this await point
    pub id: u32,
    /// The expression being awaited
    pub expr: Expr,
    /// The type of the awaited value
    pub await_ty: Type,
    /// The state to transition to after this await
    pub next_state: AsyncState,
}

/// Async state machine generator
pub struct AsyncStateMachineGenerator {
    /// Counter for generating unique state IDs
    state_counter: u32,
    /// Counter for generating unique await point IDs
    await_counter: u32,
    /// Map from await point ID to await point info
    await_points: HashMap<u32, AwaitPoint>,
    /// The states in the state machine
    states: Vec<AsyncState>,
}

impl AsyncStateMachineGenerator {
    /// Create a new async state machine generator
    pub fn new() -> Self {
        Self {
            state_counter: 0,
            await_counter: 0,
            await_points: HashMap::new(),
            states: vec![AsyncState::Start],
        }
    }

    /// Generate a new state ID
    fn new_state_id(&mut self) -> u32 {
        let id = self.state_counter;
        self.state_counter += 1;
        id
    }

    /// Generate a new await point ID
    fn new_await_id(&mut self) -> u32 {
        let id = self.await_counter;
        self.await_counter += 1;
        id
    }

    /// Transform an async function into a state machine
    pub fn transform_async_function(&mut self, func: &Function) -> AsyncFunctionTransform {
        // Collect all await points in the function body
        let await_points = self.collect_await_points_in_block(&func.body);

        // Generate states for each await point
        let mut states = vec![AsyncState::Start];
        for (i, _) in await_points.iter().enumerate() {
            states.push(AsyncState::Awaiting(i as u32));
        }
        states.push(AsyncState::Completed);
        states.push(AsyncState::Panicked);

        self.states = states.clone();

        // Generate the state machine structure
        let state_enum = self.generate_state_enum(&states);

        // Generate the Future implementation
        let future_impl = self.generate_future_impl(func, &await_points, &states);

        AsyncFunctionTransform {
            original_func: func.clone(),
            state_enum,
            future_impl,
            await_points,
            states,
        }
    }

    /// Collect all await points in a block
    fn collect_await_points_in_block(&mut self, block: &Block) -> Vec<AwaitPoint> {
        let mut points = Vec::new();
        for stmt in &block.stmts {
            self.collect_await_points_in_stmt(stmt, &mut points);
        }
        points
    }

    /// Collect all await points in an expression
    fn collect_await_points(&mut self, expr: &Expr) -> Vec<AwaitPoint> {
        let mut points = Vec::new();
        self.collect_await_points_recursive(expr, &mut points);
        points
    }

    /// Recursively collect await points
    fn collect_await_points_recursive(&mut self, expr: &Expr, points: &mut Vec<AwaitPoint>) {
        match expr {
            Expr::Await(await_expr) => {
                let id = self.new_await_id();
                let point = AwaitPoint {
                    id,
                    expr: *await_expr.clone(),
                    await_ty: Type::Unit, // Would be inferred from type checking
                    next_state: AsyncState::Awaiting(id),
                };
                self.await_points.insert(id, point.clone());
                points.push(point);
            }
            Expr::Block(block) => {
                for stmt in &block.stmts {
                    self.collect_await_points_in_stmt(stmt, points);
                }
            }
            Expr::If(if_expr) => {
                self.collect_await_points_recursive(&if_expr.cond, points);
                self.collect_await_points_recursive(
                    &Expr::Block(if_expr.then_branch.clone()),
                    points,
                );
                if let Some(else_branch) = &if_expr.else_branch {
                    self.collect_await_points_recursive(else_branch, points);
                }
            }
            Expr::While(while_expr) => {
                self.collect_await_points_recursive(&while_expr.cond, points);
                self.collect_await_points_recursive(&Expr::Block(while_expr.body.clone()), points);
            }
            Expr::Loop(loop_expr) => {
                self.collect_await_points_recursive(&Expr::Block(loop_expr.body.clone()), points);
            }
            Expr::For(for_expr) => {
                self.collect_await_points_recursive(&for_expr.expr, points);
                self.collect_await_points_recursive(&Expr::Block(for_expr.body.clone()), points);
            }
            Expr::Match(match_expr) => {
                self.collect_await_points_recursive(&match_expr.expr, points);
                for arm in &match_expr.arms {
                    self.collect_await_points_recursive(&arm.body, points);
                }
            }
            Expr::Binary(binary) => {
                self.collect_await_points_recursive(&binary.left, points);
                self.collect_await_points_recursive(&binary.right, points);
            }
            Expr::Unary(unary) => {
                self.collect_await_points_recursive(&unary.expr, points);
            }
            Expr::Call(call) => {
                self.collect_await_points_recursive(&call.func, points);
                for arg in &call.args {
                    self.collect_await_points_recursive(arg, points);
                }
            }
            Expr::MethodCall(method) => {
                self.collect_await_points_recursive(&method.receiver, points);
                for arg in &method.args {
                    self.collect_await_points_recursive(arg, points);
                }
            }
            Expr::Return(ret) => {
                if let Some(ret_expr) = ret {
                    self.collect_await_points_recursive(ret_expr, points);
                }
            }
            Expr::Assign(assign) => {
                self.collect_await_points_recursive(&assign.left, points);
                self.collect_await_points_recursive(&assign.right, points);
            }
            Expr::FieldAccess(field) => {
                self.collect_await_points_recursive(&field.expr, points);
            }
            Expr::Index(index) => {
                self.collect_await_points_recursive(&index.expr, points);
                self.collect_await_points_recursive(&index.index, points);
            }
            Expr::ArrayInit(array) => {
                for elem in &array.elements {
                    self.collect_await_points_recursive(elem, points);
                }
            }
            Expr::StructInit(struct_init) => {
                for (_, field_expr) in &struct_init.fields {
                    self.collect_await_points_recursive(field_expr, points);
                }
            }
            Expr::Closure(closure) => {
                self.collect_await_points_recursive(&closure.body, points);
            }
            Expr::Async(block) => {
                // Nested async blocks are handled separately
                self.collect_await_points_recursive(&Expr::Block(block.clone()), points);
            }
            _ => {}
        }
    }

    /// Collect await points in a statement
    fn collect_await_points_in_stmt(&mut self, stmt: &Stmt, points: &mut Vec<AwaitPoint>) {
        match stmt {
            Stmt::Let(let_stmt) => {
                if let Some(init) = &let_stmt.init {
                    self.collect_await_points_recursive(init, points);
                }
            }
            Stmt::Expr(expr) => {
                self.collect_await_points_recursive(expr, points);
            }
            Stmt::Item(item) => {
                if let Item::Function(func) = item {
                    if func.is_async {
                        let nested_points = self.collect_await_points_in_block(&func.body);
                        points.extend(nested_points);
                    }
                }
            }
        }
    }

    /// Generate the state enum for the state machine
    fn generate_state_enum(&self, states: &[AsyncState]) -> StateEnum {
        let variants: Vec<StateVariant> = states
            .iter()
            .map(|state| match state {
                AsyncState::Start => StateVariant {
                    name: Ident::new("Start"),
                    fields: vec![],
                },
                AsyncState::Awaiting(id) => StateVariant {
                    name: Ident::new(&format!("Awaiting{}", id)),
                    fields: vec![
                        // Field for the future being awaited
                        StateField {
                            name: Ident::new("future"),
                            ty: Type::Path(make_path("BoxFuture")),
                        },
                    ],
                },
                AsyncState::Completed => StateVariant {
                    name: Ident::new("Completed"),
                    fields: vec![],
                },
                AsyncState::Panicked => StateVariant {
                    name: Ident::new("Panicked"),
                    fields: vec![],
                },
            })
            .collect();

        StateEnum {
            name: Ident::new("__AsyncState"),
            variants,
        }
    }

    /// Generate the Future trait implementation
    fn generate_future_impl(
        &self,
        func: &Function,
        await_points: &[AwaitPoint],
        states: &[AsyncState],
    ) -> FutureImpl {
        // Generate the poll method body
        let poll_body = self.generate_poll_body(func, await_points, states);

        FutureImpl {
            struct_name: Ident::new(&format!("__AsyncFuture_{}", func.name)),
            output_ty: func.return_type.clone().unwrap_or(Type::Unit),
            poll_body,
        }
    }

    /// Generate the body of the poll method
    fn generate_poll_body(
        &self,
        _func: &Function,
        await_points: &[AwaitPoint],
        _states: &[AsyncState],
    ) -> Vec<Stmt> {
        let mut stmts = Vec::new();

        // Generate match on current state
        let mut arms = Vec::new();

        // Start state arm
        arms.push(MatchArm {
            pattern: Pattern::Ident(Ident::new("Start")),
            guard: None,
            body: Expr::Block(Block {
                stmts: vec![
                    // Transition to first await or complete
                    Stmt::Expr(Expr::Assign(AssignExpr {
                        left: Box::new(Expr::Ident(Ident::new("self.state"))),
                        right: Box::new(Expr::Ident(Ident::new("State0"))),
                    })),
                ],
                span: 0..0,
            }),
        });

        // Await state arms
        for point in await_points {
            let await_id = point.id;
            arms.push(MatchArm {
                pattern: Pattern::Ident(Ident::new(&format!("Awaiting{}", await_id))),
                guard: None,
                body: Expr::Block(Block {
                    stmts: vec![
                        // Poll the inner future
                        Stmt::Expr(Expr::Call(CallExpr {
                            func: Box::new(Expr::Ident(Ident::new("future.poll"))),
                            args: vec![Expr::Ident(Ident::new("cx"))],
                        })),
                    ],
                    span: 0..0,
                }),
            });
        }

        // Completed state arm
        arms.push(MatchArm {
            pattern: Pattern::Ident(Ident::new("Completed")),
            guard: None,
            body: Expr::Block(Block {
                stmts: vec![Stmt::Expr(Expr::Return(Some(Box::new(Expr::Ident(
                    Ident::new("Poll::Ready(result)"),
                )))))],
                span: 0..0,
            }),
        });

        // Generate the match expression
        let match_expr = Expr::Match(MatchExpr {
            expr: Box::new(Expr::Ident(Ident::new("self.state"))),
            arms,
        });

        stmts.push(Stmt::Expr(match_expr));

        stmts
    }
}

impl Default for AsyncStateMachineGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of transforming an async function
#[derive(Debug, Clone)]
pub struct AsyncFunctionTransform {
    /// The original async function
    pub original_func: Function,
    /// The generated state enum
    pub state_enum: StateEnum,
    /// The generated Future implementation
    pub future_impl: FutureImpl,
    /// All await points in the function
    pub await_points: Vec<AwaitPoint>,
    /// All states in the state machine
    pub states: Vec<AsyncState>,
}

/// A state enum definition
#[derive(Debug, Clone)]
pub struct StateEnum {
    pub name: Ident,
    pub variants: Vec<StateVariant>,
}

/// A variant in the state enum
#[derive(Debug, Clone)]
pub struct StateVariant {
    pub name: Ident,
    pub fields: Vec<StateField>,
}

/// A field in a state variant
#[derive(Debug, Clone)]
pub struct StateField {
    pub name: Ident,
    pub ty: Type,
}

/// A Future trait implementation
#[derive(Debug, Clone)]
pub struct FutureImpl {
    pub struct_name: Ident,
    pub output_ty: Type,
    pub poll_body: Vec<Stmt>,
}

/// Helper function to create a Path from a string
fn make_path(name: &str) -> Path {
    Path {
        segments: vec![PathSegment {
            ident: Ident::new(name),
            generics: vec![],
        }],
    }
}

/// Transform MIR for async functions
pub struct MirAsyncTransformer;

impl MirAsyncTransformer {
    /// Transform a MIR body for an async function
    pub fn transform(body: &MirBody) -> AsyncMirBody {
        // Identify await points in the MIR
        let await_points = Self::find_await_points(body);

        // Generate state machine MIR
        let state_machine = Self::generate_state_machine_mir(body, &await_points);

        AsyncMirBody {
            original_body: body.clone(),
            state_machine,
            await_points,
        }
    }

    /// Find all await points in a MIR body
    fn find_await_points(body: &MirBody) -> Vec<MirAwaitPoint> {
        let mut points = Vec::new();

        for (block_idx, block) in body.basic_blocks.iter().enumerate() {
            for (stmt_idx, stmt) in block.statements.iter().enumerate() {
                if let Statement::Assign(_, rvalue) = stmt {
                    // Check for await operation (represented as a special Rvalue or function call)
                    if Self::is_await_rvalue(rvalue) {
                        points.push(MirAwaitPoint {
                            block: BasicBlock(block_idx as u32),
                            statement: stmt_idx,
                            id: points.len() as u32,
                        });
                    }
                }
            }
        }

        points
    }

    /// Check if an rvalue represents an await operation
    fn is_await_rvalue(_rvalue: &Rvalue) -> bool {
        // In a real implementation, this would check for a specific await intrinsic
        // For now, we don't have a Call variant in Rvalue, so return false
        false
    }

    /// Generate the state machine MIR
    fn generate_state_machine_mir(body: &MirBody, await_points: &[MirAwaitPoint]) -> MirBody {
        let mut state_machine_body = MirBody::new(body.arg_count, body.span.clone());

        // Copy local declarations
        state_machine_body.local_decls = body.local_decls.clone();

        // Add state variable
        let state_local =
            state_machine_body.push_local(LocalDecl::new(Type::Path(make_path("u32")), 0..0));

        // Generate basic blocks for each state
        let num_states = await_points.len() + 2; // Start, Await states, Completed

        for state_id in 0..num_states {
            let mut block = BasicBlockData::new();

            if state_id == 0 {
                // Start state - initialize and jump to first await or complete
                block.statements.push(Statement::Assign(
                    Place::from_local(state_local),
                    Rvalue::Use(Operand::Constant(Constant::Scalar(Scalar::Int(
                        0,
                        IntType::I32,
                    )))),
                ));
            }

            // Add terminator based on state
            if state_id < await_points.len() {
                // Await state - poll the future
                let next_state = BasicBlock((state_id + 1) as u32);
                block.set_terminator(Terminator {
                    kind: TerminatorKind::Goto { target: next_state },
                    span: 0..0,
                });
            } else {
                // Completed state
                block.set_terminator(Terminator {
                    kind: TerminatorKind::Return,
                    span: 0..0,
                });
            }

            state_machine_body.basic_blocks.push(block);
        }

        state_machine_body
    }
}

/// An await point in MIR
#[derive(Debug, Clone)]
pub struct MirAwaitPoint {
    pub block: BasicBlock,
    pub statement: usize,
    pub id: u32,
}

/// Transformed async MIR body
#[derive(Debug, Clone)]
pub struct AsyncMirBody {
    pub original_body: MirBody,
    pub state_machine: MirBody,
    pub await_points: Vec<MirAwaitPoint>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_state_machine_generator_creation() {
        let generator = AsyncStateMachineGenerator::new();
        assert_eq!(generator.state_counter, 0);
        assert_eq!(generator.await_counter, 0);
    }

    #[test]
    fn test_collect_await_points_simple() {
        let mut generator = AsyncStateMachineGenerator::new();

        // Create a simple async block with one await
        let await_expr = Expr::Await(Box::new(Expr::Ident(Ident::new("future"))));
        let block = Block {
            stmts: vec![Stmt::Expr(await_expr)],
            span: 0..10,
        };

        let points = generator.collect_await_points(&Expr::Block(block));
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].id, 0);
    }

    #[test]
    fn test_state_enum_generation() {
        let generator = AsyncStateMachineGenerator::new();
        let states = vec![
            AsyncState::Start,
            AsyncState::Awaiting(0),
            AsyncState::Completed,
        ];

        let state_enum = generator.generate_state_enum(&states);
        assert_eq!(state_enum.variants.len(), 3);
        assert_eq!(state_enum.variants[0].name.as_str(), "Start");
        assert_eq!(state_enum.variants[1].name.as_str(), "Awaiting0");
        assert_eq!(state_enum.variants[2].name.as_str(), "Completed");
    }

    #[test]
    fn test_mir_async_transformer() {
        let body = MirBody::new(0, 0..100);
        let async_body = MirAsyncTransformer::transform(&body);

        assert_eq!(async_body.await_points.len(), 0);
        assert!(!async_body.state_machine.basic_blocks.is_empty());
    }
}
