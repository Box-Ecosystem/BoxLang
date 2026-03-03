//! Cranelift Backend Module
//!
//! This module provides code generation using Cranelift,
//! a fast, non-optimizing code generator suitable for debug builds.
//!
//! Based on the official Cranelift JIT demo and documentation:
//! - https://github.com/bytecodealliance/cranelift-jit-demo
//! - https://docs.rs/cranelift-frontend

use crate::middle::mir::{
    BasicBlock, BinOp, Constant, Local, MirBody, Operand,
    Rvalue, Scalar, Statement, Terminator, TerminatorKind, UnOp,
};
use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};
use std::collections::HashMap;

/// Cranelift backend configuration
#[derive(Debug, Clone)]
pub struct CraneliftConfig {
    pub opt_level: OptLevel,
    pub target: String,
    pub debug_info: bool,
}

impl Default for CraneliftConfig {
    fn default() -> Self {
        Self {
            opt_level: OptLevel::None,
            target: "x86_64-unknown-linux-gnu".to_string(),
            debug_info: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptLevel {
    None,
    Basic,
    Standard,
    Aggressive,
}

/// Cranelift backend
pub struct CraneliftBackend {
    config: CraneliftConfig,
}

impl CraneliftBackend {
    pub fn new(config: CraneliftConfig) -> Self {
        Self { config }
    }

    /// Compile a MIR body to machine code
    pub fn compile(&self, body: &MirBody) -> Result<CompiledFunction, CraneliftError> {
        let mut flag_builder = settings::builder();

        match self.config.opt_level {
            OptLevel::None => flag_builder
                .set("opt_level", "none")
                .map_err(|e| CraneliftError::CompilationFailed(e.to_string()))?,
            OptLevel::Basic => flag_builder
                .set("opt_level", "speed")
                .map_err(|e| CraneliftError::CompilationFailed(e.to_string()))?,
            OptLevel::Standard => flag_builder
                .set("opt_level", "speed_and_size")
                .map_err(|e| CraneliftError::CompilationFailed(e.to_string()))?,
            OptLevel::Aggressive => {
                flag_builder
                    .set("opt_level", "speed_and_size")
                    .map_err(|e| CraneliftError::CompilationFailed(e.to_string()))?;
                flag_builder
                    .set("enable_alias_analysis", "true")
                    .map_err(|e| CraneliftError::CompilationFailed(e.to_string()))?;
            }
        }

        flag_builder
            .set("enable_verifier", "true")
            .map_err(|e| CraneliftError::CompilationFailed(e.to_string()))?;

        let isa_builder = cranelift_native::builder()
            .map_err(|e| CraneliftError::CompilationFailed(e.to_string()))?;
        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .map_err(|e| CraneliftError::CompilationFailed(e.to_string()))?;

        let builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        let mut module = JITModule::new(builder);

        let func_id = self.translate_function(&mut module, "main", body)?;

        module
            .finalize_definitions()
            .map_err(|e| CraneliftError::CompilationFailed(e.to_string()))?;

        let code_ptr = module.get_finalized_function(func_id);

        Ok(CompiledFunction {
            ptr: code_ptr,
            module: Box::new(module),
            func_id,
        })
    }

    /// Compile multiple functions
    pub fn compile_module(
        &self,
        functions: &[(&str, &MirBody)],
    ) -> Result<CompiledModule, CraneliftError> {
        let mut flag_builder = settings::builder();
        flag_builder
            .set("opt_level", "none")
            .map_err(|e| CraneliftError::CompilationFailed(e.to_string()))?;
        flag_builder
            .set("enable_verifier", "true")
            .map_err(|e| CraneliftError::CompilationFailed(e.to_string()))?;

        let isa_builder = cranelift_native::builder()
            .map_err(|e| CraneliftError::CompilationFailed(e.to_string()))?;
        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .map_err(|e| CraneliftError::CompilationFailed(e.to_string()))?;

        let builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        let mut module = JITModule::new(builder);

        let mut compiled_functions = HashMap::new();

        for (name, body) in functions {
            let func_id = self.translate_function(&mut module, name, body)?;
            compiled_functions.insert(name.to_string(), func_id);
        }

        module
            .finalize_definitions()
            .map_err(|e| CraneliftError::CompilationFailed(e.to_string()))?;

        Ok(CompiledModule {
            module: Box::new(module),
            functions: compiled_functions,
        })
    }

    /// Translate a single function
    fn translate_function(
        &self,
        module: &mut JITModule,
        name: &str,
        body: &MirBody,
    ) -> Result<FuncId, CraneliftError> {
        let mut ctx = module.make_context();
        let mut func_ctx = FunctionBuilderContext::new();

        // Set up signature: () -> i64 for simplicity
        ctx.func.signature.returns.push(AbiParam::new(types::I64));

        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);

        // Create entry block and seal it immediately since it has no predecessors
        let entry_block = builder.create_block();
        builder.switch_to_block(entry_block);

        // Build the function body
        let mut translator = FunctionTranslator::new(&mut builder, body);
        translator.translate_body()?;

        // Finalize
        builder.finalize();

        // Verify
        let flags = module.isa().flags().clone();
        if let Err(errors) = cranelift_codegen::verify_function(&ctx.func, &flags) {
            return Err(CraneliftError::VerificationFailed(format!("{:?}", errors)));
        }

        // Declare and define
        let func_id = module
            .declare_function(name, Linkage::Export, &ctx.func.signature)
            .map_err(|e| CraneliftError::CompilationFailed(e.to_string()))?;

        module
            .define_function(func_id, &mut ctx)
            .map_err(|e| CraneliftError::CompilationFailed(e.to_string()))?;

        Ok(func_id)
    }

    /// Compile to IR for debugging
    pub fn compile_to_ir(&self, name: &str, body: &MirBody) -> Result<String, CraneliftError> {
        let mut flag_builder = settings::builder();
        flag_builder
            .set("opt_level", "none")
            .map_err(|e| CraneliftError::CompilationFailed(e.to_string()))?;

        let isa_builder = cranelift_native::builder()
            .map_err(|e| CraneliftError::CompilationFailed(e.to_string()))?;
        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .map_err(|e| CraneliftError::CompilationFailed(e.to_string()))?;

        let builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        let mut module = JITModule::new(builder);

        let mut ctx = module.make_context();
        let mut func_ctx = FunctionBuilderContext::new();

        ctx.func.signature.returns.push(AbiParam::new(types::I64));

        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let entry_block = builder.create_block();
        builder.switch_to_block(entry_block);

        let mut translator = FunctionTranslator::new(&mut builder, body);
        translator.translate_body()?;

        builder.finalize();

        Ok(format!(
            "; Cranelift IR for function: {}\n{}\n",
            name,
            ctx.func.display()
        ))
    }
}

impl Default for CraneliftBackend {
    fn default() -> Self {
        Self::new(CraneliftConfig::default())
    }
}

/// Compiled function
pub struct CompiledFunction {
    pub ptr: *const u8,
    #[allow(dead_code)]
    module: Box<JITModule>,
    func_id: FuncId,
}

impl CompiledFunction {
    pub fn as_ptr<T>(&self) -> *const T {
        self.ptr as *const T
    }

    pub unsafe fn call(&self, arg: i64) -> i64 {
        let func: unsafe extern "C" fn(i64) -> i64 = std::mem::transmute(self.ptr);
        func(arg)
    }

    pub unsafe fn call_void(&self) -> i64 {
        let func: unsafe extern "C" fn() -> i64 = std::mem::transmute(self.ptr);
        func()
    }
}

/// Compiled module
pub struct CompiledModule {
    #[allow(dead_code)]
    module: Box<JITModule>,
    functions: HashMap<String, FuncId>,
}

impl CompiledModule {
    pub fn get_function(&self, name: &str) -> Option<*const u8> {
        self.functions
            .get(name)
            .map(|&func_id| self.module.get_finalized_function(func_id))
    }

    pub unsafe fn call(&self, name: &str, arg: i64) -> Option<i64> {
        self.get_function(name).map(|ptr| {
            let func: unsafe extern "C" fn(i64) -> i64 = std::mem::transmute(ptr);
            func(arg)
        })
    }
}

/// Function translator - translates MIR to Cranelift IR
///
/// Following the official Cranelift documentation:
/// 1. All blocks must be sealed when all their predecessors are known
/// 2. Use declare_var/def_var/use_var for automatic SSA construction
struct FunctionTranslator<'a, 'b> {
    builder: &'a mut FunctionBuilder<'b>,
    body: &'b MirBody,
    /// Map from MIR locals to Cranelift variables
    variables: HashMap<Local, Variable>,
    /// Map from basic blocks to Cranelift blocks
    blocks: HashMap<BasicBlock, Block>,
    /// Counter for next variable index
    next_var_index: usize,
}

impl<'a, 'b> FunctionTranslator<'a, 'b> {
    fn new(builder: &'a mut FunctionBuilder<'b>, body: &'b MirBody) -> Self {
        Self {
            builder,
            body,
            variables: HashMap::new(),
            blocks: HashMap::new(),
            next_var_index: 0,
        }
    }

    /// Translate MIR body to Cranelift IR
    fn translate_body(&mut self) -> Result<(), CraneliftError> {
        // Handle empty body - just return 0
        if self.body.basic_blocks.is_empty() {
            let block = self.builder.create_block();
            self.builder.switch_to_block(block);
            let zero = self.builder.ins().iconst(types::I64, 0);
            self.builder.ins().return_(&[zero]);
            self.builder.seal_block(block);
            return Ok(());
        }

        // Create all basic blocks first
        for (idx, _) in self.body.basic_blocks.iter().enumerate() {
            let block = self.builder.create_block();
            self.blocks.insert(BasicBlock(idx as u32), block);
        }

        // Switch to entry block first
        let entry_block = self.blocks[&BasicBlock(0)];
        self.builder.switch_to_block(entry_block);

        // Declare all locals as variables (in entry block)
        self.declare_locals();

        // First pass: translate all blocks (but don't seal yet)
        let num_blocks = self.body.basic_blocks.len();
        for (idx, block_data) in self.body.basic_blocks.iter().enumerate() {
            let block = self.blocks[&BasicBlock(idx as u32)];

            // Switch to the block (entry block already switched)
            if idx > 0 {
                self.builder.switch_to_block(block);
            }

            // Translate statements
            for stmt in &block_data.statements {
                self.translate_statement(stmt)?;
            }

            // Translate terminator
            if let Some(ref terminator) = block_data.terminator {
                self.translate_terminator(terminator)?;
            } else if idx == num_blocks - 1 {
                // Last block without terminator - return 0
                let zero = self.builder.ins().iconst(types::I64, 0);
                self.builder.ins().return_(&[zero]);
            }
        }

        // Second pass: seal all blocks
        // All predecessors are now known since we've translated all blocks
        for (idx, _) in self.body.basic_blocks.iter().enumerate() {
            let block = self.blocks[&BasicBlock(idx as u32)];
            self.builder.seal_block(block);
        }

        Ok(())
    }

    /// Declare all locals as Cranelift variables
    fn declare_locals(&mut self) {
        for (i, _local_decl) in self.body.local_decls.iter().enumerate() {
            let local = Local(i as u32);
            let var = Variable::new(self.next_var_index);
            self.next_var_index += 1;

            // Declare variable with type i64
            self.builder.declare_var(var, types::I64);

            // Initialize with 0
            let zero = self.builder.ins().iconst(types::I64, 0);
            self.builder.def_var(var, zero);

            self.variables.insert(local, var);
        }
    }

    /// Get or create a variable for a local
    fn get_var(&self, local: Local) -> Result<Variable, CraneliftError> {
        self.variables
            .get(&local)
            .copied()
            .ok_or_else(|| CraneliftError::InvalidInput(format!("Local {:?} not declared", local)))
    }

    /// Translate a statement
    fn translate_statement(&mut self, stmt: &Statement) -> Result<(), CraneliftError> {
        match stmt {
            Statement::Assign(place, rvalue) => {
                let value = self.translate_rvalue(rvalue)?;
                let var = self.get_var(place.local)?;
                self.builder.def_var(var, value);
            }
            Statement::StorageLive(_) | Statement::StorageDead(_) => {
                // No-op for now - these are for borrow checking
            }
            Statement::Nop | Statement::InlineAsm(_) => {}
        }
        Ok(())
    }

    /// Translate an rvalue
    fn translate_rvalue(&mut self, rvalue: &Rvalue) -> Result<Value, CraneliftError> {
        match rvalue {
            Rvalue::Use(operand) => self.translate_operand(operand),
            Rvalue::BinaryOp(op, left, right) => {
                let left_val = self.translate_operand(left)?;
                let right_val = self.translate_operand(right)?;

                let result = match op {
                    BinOp::Add => self.builder.ins().iadd(left_val, right_val),
                    BinOp::Sub => self.builder.ins().isub(left_val, right_val),
                    BinOp::Mul => self.builder.ins().imul(left_val, right_val),
                    BinOp::Div => self.builder.ins().sdiv(left_val, right_val),
                    BinOp::Rem => self.builder.ins().srem(left_val, right_val),
                    BinOp::BitAnd => self.builder.ins().band(left_val, right_val),
                    BinOp::BitOr => self.builder.ins().bor(left_val, right_val),
                    BinOp::BitXor => self.builder.ins().bxor(left_val, right_val),
                    BinOp::Shl => self.builder.ins().ishl(left_val, right_val),
                    BinOp::Shr => self.builder.ins().sshr(left_val, right_val),
                    BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                        // Comparison operations return i8, need to extend to i64
                        let int_cc = match op {
                            BinOp::Eq => IntCC::Equal,
                            BinOp::Ne => IntCC::NotEqual,
                            BinOp::Lt => IntCC::SignedLessThan,
                            BinOp::Le => IntCC::SignedLessThanOrEqual,
                            BinOp::Gt => IntCC::SignedGreaterThan,
                            BinOp::Ge => IntCC::SignedGreaterThanOrEqual,
                            _ => unreachable!(),
                        };
                        let cmp = self.builder.ins().icmp(int_cc, left_val, right_val);
                        self.builder.ins().uextend(types::I64, cmp)
                    }
                    _ => {
                        return Err(CraneliftError::UnsupportedOperation(format!(
                            "Binary op {:?}",
                            op
                        )))
                    }
                };
                Ok(result)
            }
            Rvalue::UnaryOp(op, operand) => {
                let val = self.translate_operand(operand)?;
                let result = match op {
                    UnOp::Neg => self.builder.ins().ineg(val),
                    UnOp::Not => self.builder.ins().bnot(val),
                };
                Ok(result)
            }
            Rvalue::Cast(_, operand, _) => {
                // For now, just pass through
                self.translate_operand(operand)
            }
            Rvalue::Ref(place, _) => {
                // For now, return the local's value
                let var = self.get_var(place.local)?;
                Ok(self.builder.use_var(var))
            }
            _ => Err(CraneliftError::UnsupportedOperation(
                "Complex rvalue".to_string(),
            )),
        }
    }

    /// Translate an operand
    fn translate_operand(&mut self, operand: &Operand) -> Result<Value, CraneliftError> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                let var = self.get_var(place.local)?;
                Ok(self.builder.use_var(var))
            }
            Operand::Constant(constant) => match constant {
                Constant::Scalar(Scalar::Int(val, _)) => {
                    Ok(self.builder.ins().iconst(types::I64, *val as i64))
                }
                Constant::Scalar(Scalar::Float(val, _)) => Ok(self.builder.ins().f64const(*val)),
                _ => Err(CraneliftError::UnsupportedOperation(
                    "Constant type".to_string(),
                )),
            },
        }
    }

    /// Translate a terminator
    fn translate_terminator(&mut self, terminator: &Terminator) -> Result<(), CraneliftError> {
        match &terminator.kind {
            TerminatorKind::Return => {
                // Return _0 (return place)
                let ret_var = self.get_var(Local::RETURN_PLACE)?;
                let ret_val = self.builder.use_var(ret_var);
                self.builder.ins().return_(&[ret_val]);
            }
            TerminatorKind::Goto { target } => {
                let target_block = self.blocks.get(target).ok_or_else(|| {
                    CraneliftError::InvalidInput(format!("Block {:?} not found", target))
                })?;
                self.builder.ins().jump(*target_block, &[]);
            }
            TerminatorKind::SwitchInt {
                discr,
                targets,
                otherwise,
                ..
            } => {
                let discr_val = self.translate_operand(discr)?;
                let otherwise_block = self.blocks.get(otherwise).ok_or_else(|| {
                    CraneliftError::InvalidInput(format!("Block {:?} not found", otherwise))
                })?;

                // Build chain of comparisons for switch
                for (i, (val, target)) in targets.iter().enumerate() {
                    let target_block = self.blocks.get(target).ok_or_else(|| {
                        CraneliftError::InvalidInput(format!("Block {:?} not found", target))
                    })?;

                    let const_val = self.builder.ins().iconst(types::I64, *val as i64);
                    let cmp = self.builder.ins().icmp(IntCC::Equal, discr_val, const_val);

                    if i == targets.len() - 1 {
                        // Last target - use brif with otherwise
                        self.builder
                            .ins()
                            .brif(cmp, *target_block, &[], *otherwise_block, &[]);
                    } else {
                        // Create intermediate block for next comparison
                        let next_block = self.builder.create_block();
                        self.builder
                            .ins()
                            .brif(cmp, *target_block, &[], next_block, &[]);
                        self.builder.switch_to_block(next_block);
                        // Seal this intermediate block immediately
                        self.builder.seal_block(next_block);
                    }
                }

                // If no targets, just jump to otherwise
                if targets.is_empty() {
                    self.builder.ins().jump(*otherwise_block, &[]);
                }
            }
            TerminatorKind::Call {
                func: _,
                args: _,
                target,
                ..
            } => {
                // For now, just jump to target (no actual call)
                if let Some(target_block) = target {
                    let block = self.blocks.get(target_block).ok_or_else(|| {
                        CraneliftError::InvalidInput(format!("Block {:?} not found", target_block))
                    })?;
                    self.builder.ins().jump(*block, &[]);
                } else {
                    let zero = self.builder.ins().iconst(types::I64, 0);
                    self.builder.ins().return_(&[zero]);
                }
            }
            _ => {
                // Unsupported terminator - just return 0
                let zero = self.builder.ins().iconst(types::I64, 0);
                self.builder.ins().return_(&[zero]);
            }
        }
        Ok(())
    }
}

/// Error types
#[derive(Debug, Clone)]
pub enum CraneliftError {
    CompilationFailed(String),
    VerificationFailed(String),
    InvalidInput(String),
    UnsupportedOperation(String),
}

impl std::fmt::Display for CraneliftError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CraneliftError::CompilationFailed(msg) => write!(f, "Compilation failed: {}", msg),
            CraneliftError::VerificationFailed(msg) => write!(f, "Verification failed: {}", msg),
            CraneliftError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            CraneliftError::UnsupportedOperation(msg) => {
                write!(f, "Unsupported operation: {}", msg)
            }
        }
    }
}

impl std::error::Error for CraneliftError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middle::mir::*;

    #[test]
    fn test_backend_creation() {
        let backend = CraneliftBackend::default();
        assert_eq!(backend.config.opt_level, OptLevel::None);
    }

    #[test]
    fn test_compile_empty_body() {
        let backend = CraneliftBackend::default();
        let body = MirBody::new(0, 0..100);

        let result = backend.compile(&body);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_simple_function() {
        let backend = CraneliftBackend::default();

        // Create a simple function: return 42
        let mut body = MirBody::new(0, 0..100);
        body.push_local(LocalDecl::new(crate::ast::Type::Unit, 0..10)); // Return local

        let mut block = BasicBlockData::new();
        // _0 = 42
        block.statements.push(Statement::Assign(
            Place::from_local(Local::RETURN_PLACE),
            Rvalue::Use(Operand::Constant(Constant::Scalar(Scalar::Int(
                42,
                IntType::I64,
            )))),
        ));
        block.set_terminator(Terminator {
            kind: TerminatorKind::Return,
            span: 0..100,
        });

        body.basic_blocks.push(block);

        let result = backend.compile(&body);
        assert!(result.is_ok());

        // Try to call the function
        let compiled = result.unwrap();
        let ret: i64 = unsafe { compiled.call_void() };
        assert_eq!(ret, 42);
    }

    #[test]
    fn test_compile_to_ir() {
        let backend = CraneliftBackend::default();
        let body = MirBody::new(0, 0..100);

        let ir = backend.compile_to_ir("test", &body);
        assert!(ir.is_ok());
        let ir_str = ir.unwrap();
        assert!(ir_str.contains("test"));
        assert!(ir_str.contains("function"));
    }

    #[test]
    fn test_arithmetic_operations() {
        let backend = CraneliftBackend::default();

        // Create a function that computes: (10 + 20) * 2
        let mut body = MirBody::new(0, 0..100);
        body.push_local(LocalDecl::new(crate::ast::Type::Unit, 0..10)); // _0: return
        body.push_local(LocalDecl::new(crate::ast::Type::Unit, 10..20)); // _1: temp1
        body.push_local(LocalDecl::new(crate::ast::Type::Unit, 20..30)); // _2: temp2

        let mut block = BasicBlockData::new();

        // _1 = 10 + 20
        block.statements.push(Statement::Assign(
            Place::from_local(Local(1)),
            Rvalue::BinaryOp(
                BinOp::Add,
                Box::new(Operand::Constant(Constant::Scalar(Scalar::Int(
                    10,
                    IntType::I64,
                )))),
                Box::new(Operand::Constant(Constant::Scalar(Scalar::Int(
                    20,
                    IntType::I64,
                )))),
            ),
        ));

        // _2 = _1 * 2
        block.statements.push(Statement::Assign(
            Place::from_local(Local(2)),
            Rvalue::BinaryOp(
                BinOp::Mul,
                Box::new(Operand::Copy(Place::from_local(Local(1)))),
                Box::new(Operand::Constant(Constant::Scalar(Scalar::Int(
                    2,
                    IntType::I64,
                )))),
            ),
        ));

        // _0 = _2
        block.statements.push(Statement::Assign(
            Place::from_local(Local::RETURN_PLACE),
            Rvalue::Use(Operand::Copy(Place::from_local(Local(2)))),
        ));

        block.set_terminator(Terminator {
            kind: TerminatorKind::Return,
            span: 0..100,
        });

        body.basic_blocks.push(block);

        let result = backend.compile(&body);
        assert!(result.is_ok());

        let compiled = result.unwrap();
        let ret: i64 = unsafe { compiled.call_void() };
        assert_eq!(ret, 60); // (10 + 20) * 2 = 60
    }
}
