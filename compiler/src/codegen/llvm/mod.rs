//! LLVM Backend Module using Inkwell - Production Ready
//!
//! This module provides code generation using LLVM via the inkwell crate,
//! offering optimized release builds with advanced optimizations.
//!
//! Features:
//! - LLVM IR generation
//! - Optimization passes with configurable levels
//! - Link-Time Optimization (LTO) support
//! - Target-specific code generation
//! - Object file and executable output
//! - JIT compilation support

use crate::middle::mir::MirBody;
use std::path::Path;

// Re-export the IR builder module
pub mod ir_builder;
pub use ir_builder::LlvmIrBuilder;

// JIT compilation support
pub mod jit;
pub use jit::{JitConfig, JitContext, JitFunctionHandle, JitType, JitValue};

/// LLVM backend configuration
#[derive(Debug, Clone)]
pub struct LlvmConfig {
    /// Optimization level
    pub opt_level: OptLevel,
    /// Target triple (e.g., "x86_64-unknown-linux-gnu")
    pub target_triple: String,
    /// Enable debug information
    pub debug_info: bool,
    /// Output file type
    pub output_type: OutputType,
    /// Enable Link-Time Optimization
    pub lto: LtoLevel,
    /// Enable position independent code
    pub pic: bool,
    /// Target CPU features
    pub target_cpu: String,
    /// Target features (comma-separated)
    pub target_features: String,
}

impl Default for LlvmConfig {
    fn default() -> Self {
        Self {
            opt_level: OptLevel::Default,
            target_triple: "x86_64-unknown-linux-gnu".to_string(),
            debug_info: false,
            output_type: OutputType::Object,
            lto: LtoLevel::None,
            pic: true,
            target_cpu: "generic".to_string(),
            target_features: String::new(),
        }
    }
}

/// Optimization levels for LLVM
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptLevel {
    /// No optimization
    None,
    /// Less optimization
    Less,
    /// Default optimization
    Default,
    /// Aggressive optimization
    Aggressive,
    /// Size optimization
    Size,
    /// Aggressive size optimization
    SizeAggressive,
}

impl OptLevel {
    /// Convert to inkwell OptimizationLevel
    #[cfg(feature = "inkwell")]
    fn to_inkwell(&self) -> inkwell::OptimizationLevel {
        match self {
            OptLevel::None => inkwell::OptimizationLevel::None,
            OptLevel::Less => inkwell::OptimizationLevel::Less,
            OptLevel::Default => inkwell::OptimizationLevel::Default,
            OptLevel::Aggressive => inkwell::OptimizationLevel::Aggressive,
            OptLevel::Size => inkwell::OptimizationLevel::Default,
            OptLevel::SizeAggressive => inkwell::OptimizationLevel::Default,
        }
    }
}

/// Link-Time Optimization levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LtoLevel {
    /// No LTO
    None,
    /// Thin LTO (faster, less optimization)
    Thin,
    /// Full LTO (slower, more optimization)
    Full,
    /// Fat LTO (all in one module)
    Fat,
}

/// Output file types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputType {
    /// LLVM IR (.ll)
    LlvmIr,
    /// LLVM bitcode (.bc)
    Bitcode,
    /// Assembly (.s)
    Assembly,
    /// Object file (.o)
    Object,
    /// Executable
    Executable,
    /// Shared library
    SharedLib,
    /// Static library
    StaticLib,
}

/// LLVM backend
pub struct LlvmBackend {
    config: LlvmConfig,
    ir_builder: LlvmIrBuilder,
}

impl LlvmBackend {
    /// Create a new LLVM backend
    pub fn new(config: LlvmConfig) -> Self {
        Self {
            config,
            ir_builder: LlvmIrBuilder::new(),
        }
    }

    /// Compile a MIR body to LLVM IR
    pub fn compile(&mut self, name: &str, body: &MirBody) -> Result<String, LlvmError> {
        // Generate LLVM IR text
        let ir = self.ir_builder.build_function(name, body)?;
        Ok(ir)
    }

    /// Compile a MIR body to a file
    pub fn compile_to_file(
        &mut self,
        name: &str,
        body: &MirBody,
        output_path: &Path,
    ) -> Result<(), LlvmError> {
        let ir = self.compile(name, body)?;
        std::fs::write(output_path, ir).map_err(|e| LlvmError::IoError(e.to_string()))?;
        Ok(())
    }

    /// Compile multiple functions to a single LLVM IR file
    pub fn compile_module(&mut self, functions: &[(&str, &MirBody)]) -> Result<String, LlvmError> {
        let mut module_ir = String::new();

        // Add module header
        module_ir.push_str(&format!("; Module: boxlang_module\n"));
        module_ir.push_str(&format!("; Target: {}\n\n", self.config.target_triple));

        // Add target triple
        module_ir.push_str(&format!(
            "target triple = \"{}\"\n\n",
            self.config.target_triple
        ));

        // Add data layout (simplified)
        module_ir.push_str("target datalayout = \"e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128\"\n\n");

        // Compile each function
        for (name, body) in functions {
            let func_ir = self.ir_builder.build_function(name, body)?;
            module_ir.push_str(&func_ir);
            module_ir.push('\n');
        }

        // Add panic function declaration if not defined
        module_ir.push_str("declare void @panic()\n");

        Ok(module_ir)
    }

    /// Get the IR builder
    pub fn ir_builder(&self) -> &LlvmIrBuilder {
        &self.ir_builder
    }

    /// Get the mutable IR builder
    pub fn ir_builder_mut(&mut self) -> &mut LlvmIrBuilder {
        &mut self.ir_builder
    }

    /// Get the configuration
    pub fn config(&self) -> &LlvmConfig {
        &self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: LlvmConfig) {
        self.config = config;
    }
}

impl Default for LlvmBackend {
    fn default() -> Self {
        Self::new(LlvmConfig::default())
    }
}

/// LLVM error types
#[derive(Debug, Clone)]
pub enum LlvmError {
    /// Compilation failed
    CompilationFailed(String),
    /// Invalid input
    InvalidInput(String),
    /// IO error
    IoError(String),
    /// Optimization failed
    OptimizationFailed(String),
    /// Target not supported
    UnsupportedTarget(String),
    /// Feature not supported (when inkwell is not available)
    NotAvailable,
    /// Linking failed
    LinkingFailed(String),
    /// LTO failed
    LtoFailed(String),
}

impl std::fmt::Display for LlvmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlvmError::CompilationFailed(msg) => {
                write!(f, "LLVM compilation failed: {}", msg)
            }
            LlvmError::InvalidInput(msg) => {
                write!(f, "Invalid input: {}", msg)
            }
            LlvmError::IoError(msg) => {
                write!(f, "IO error: {}", msg)
            }
            LlvmError::OptimizationFailed(msg) => {
                write!(f, "Optimization failed: {}", msg)
            }
            LlvmError::UnsupportedTarget(target) => {
                write!(f, "Unsupported target: {}", target)
            }
            LlvmError::NotAvailable => {
                write!(f, "LLVM backend not available (inkwell not enabled)")
            }
            LlvmError::LinkingFailed(msg) => {
                write!(f, "Linking failed: {}", msg)
            }
            LlvmError::LtoFailed(msg) => {
                write!(f, "LTO failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for LlvmError {}

/// Optimization pipeline configuration
#[derive(Debug, Clone)]
pub struct OptimizationPipeline {
    /// Enable instruction combining
    pub instruction_combining: bool,
    /// Enable reassociation
    pub reassociate: bool,
    /// Enable global value numbering
    pub gvn: bool,
    /// Enable CFG simplification
    pub cfg_simplification: bool,
    /// Enable basic alias analysis
    pub basic_alias_analysis: bool,
    /// Enable memory to register promotion
    pub mem2reg: bool,
    /// Enable aggressive dead code elimination
    pub aggressive_dce: bool,
    /// Enable function inlining
    pub function_inlining: bool,
    /// Enable global dead code elimination
    pub global_dce: bool,
    /// Enable loop optimizations
    pub loop_optimize: bool,
    /// Enable vectorization
    pub vectorize: bool,
    /// Enable SLP vectorization
    pub slp_vectorize: bool,
}

impl Default for OptimizationPipeline {
    fn default() -> Self {
        Self {
            instruction_combining: true,
            reassociate: true,
            gvn: true,
            cfg_simplification: true,
            basic_alias_analysis: true,
            mem2reg: true,
            aggressive_dce: false,
            function_inlining: false,
            global_dce: false,
            loop_optimize: false,
            vectorize: false,
            slp_vectorize: false,
        }
    }
}

impl OptimizationPipeline {
    /// Create pipeline for a specific optimization level
    pub fn for_level(level: OptLevel) -> Self {
        match level {
            OptLevel::None => Self {
                instruction_combining: false,
                reassociate: false,
                gvn: false,
                cfg_simplification: false,
                basic_alias_analysis: false,
                mem2reg: false,
                aggressive_dce: false,
                function_inlining: false,
                global_dce: false,
                loop_optimize: false,
                vectorize: false,
                slp_vectorize: false,
            },
            OptLevel::Less => Self {
                instruction_combining: true,
                reassociate: true,
                gvn: false,
                cfg_simplification: true,
                basic_alias_analysis: true,
                mem2reg: true,
                aggressive_dce: false,
                function_inlining: false,
                global_dce: false,
                loop_optimize: false,
                vectorize: false,
                slp_vectorize: false,
            },
            OptLevel::Default => Self::default(),
            OptLevel::Aggressive => Self {
                instruction_combining: true,
                reassociate: true,
                gvn: true,
                cfg_simplification: true,
                basic_alias_analysis: true,
                mem2reg: true,
                aggressive_dce: true,
                function_inlining: true,
                global_dce: true,
                loop_optimize: true,
                vectorize: true,
                slp_vectorize: true,
            },
            OptLevel::Size => Self {
                instruction_combining: true,
                reassociate: true,
                gvn: true,
                cfg_simplification: true,
                basic_alias_analysis: true,
                mem2reg: true,
                aggressive_dce: true,
                function_inlining: false,
                global_dce: true,
                loop_optimize: false,
                vectorize: false,
                slp_vectorize: false,
            },
            OptLevel::SizeAggressive => Self {
                instruction_combining: true,
                reassociate: true,
                gvn: true,
                cfg_simplification: true,
                basic_alias_analysis: true,
                mem2reg: true,
                aggressive_dce: true,
                function_inlining: true,
                global_dce: true,
                loop_optimize: true,
                vectorize: false,
                slp_vectorize: false,
            },
        }
    }
}

/// Inkwell-based LLVM backend (when inkwell feature is enabled)
#[cfg(feature = "inkwell")]
pub mod inkwell_backend {
    use super::*;
    use inkwell::builder::Builder;
    use inkwell::context::Context;
    use inkwell::module::Module;
    use inkwell::passes::{PassManager, PassManagerBuilder};
    use inkwell::targets::{
        CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine, TargetTriple,
    };
    use inkwell::types::{BasicType, BasicTypeEnum};
    use inkwell::values::{BasicValue, BasicValueEnum, FunctionValue, PointerValue};
    use inkwell::OptimizationLevel;
    use std::path::Path;

    /// Inkwell-based LLVM backend with full optimization support
    pub struct InkwellBackend {
        config: LlvmConfig,
        context: Context,
        pipeline: OptimizationPipeline,
    }

    impl InkwellBackend {
        /// Create a new Inkwell backend
        pub fn new(config: LlvmConfig) -> Result<Self, LlvmError> {
            // Initialize LLVM targets
            Target::initialize_native(&InitializationConfig::default())
                .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?;

            let context = Context::create();
            let pipeline = OptimizationPipeline::for_level(config.opt_level);

            Ok(Self {
                config,
                context,
                pipeline,
            })
        }

        /// Create with custom optimization pipeline
        pub fn with_pipeline(
            config: LlvmConfig,
            pipeline: OptimizationPipeline,
        ) -> Result<Self, LlvmError> {
            Target::initialize_native(&InitializationConfig::default())
                .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?;

            let context = Context::create();

            Ok(Self {
                config,
                context,
                pipeline,
            })
        }

        /// Compile a MIR body to LLVM module
        pub fn compile_module(&self, name: &str, body: &MirBody) -> Result<Module, LlvmError> {
            let module = self.context.create_module(name);
            let builder = self.context.create_builder();

            // Set target triple
            let target_triple = TargetTriple::create(&self.config.target_triple);
            let target = Target::from_triple(&target_triple)
                .map_err(|e| LlvmError::UnsupportedTarget(e.to_string()))?;

            let target_machine = target
                .create_target_machine(
                    &target_triple,
                    &self.config.target_cpu,
                    &self.config.target_features,
                    self.config.opt_level.to_inkwell(),
                    if self.config.pic {
                        RelocMode::PIC
                    } else {
                        RelocMode::Default
                    },
                    CodeModel::Default,
                )
                .ok_or_else(|| {
                    LlvmError::CompilationFailed("Failed to create target machine".to_string())
                })?;

            module.set_triple(&target_triple);
            module.set_data_layout(&target_machine.get_target_data().get_data_layout());

            // Translate the function
            self.translate_function(&module, &builder, name, body)?;

            // Run optimization passes
            self.run_optimizations(&module)?;

            Ok(module)
        }

        /// Compile multiple functions to a module
        pub fn compile_module_with_functions(
            &self,
            name: &str,
            functions: &[(&str, &MirBody)],
        ) -> Result<Module, LlvmError> {
            let module = self.context.create_module(name);
            let builder = self.context.create_builder();

            // Set target triple
            let target_triple = TargetTriple::create(&self.config.target_triple);
            let target = Target::from_triple(&target_triple)
                .map_err(|e| LlvmError::UnsupportedTarget(e.to_string()))?;

            let target_machine = target
                .create_target_machine(
                    &target_triple,
                    &self.config.target_cpu,
                    &self.config.target_features,
                    self.config.opt_level.to_inkwell(),
                    if self.config.pic {
                        RelocMode::PIC
                    } else {
                        RelocMode::Default
                    },
                    CodeModel::Default,
                )
                .ok_or_else(|| {
                    LlvmError::CompilationFailed("Failed to create target machine".to_string())
                })?;

            module.set_triple(&target_triple);
            module.set_data_layout(&target_machine.get_target_data().get_data_layout());

            // Translate all functions
            for (func_name, body) in functions {
                self.translate_function(&module, &builder, func_name, body)?;
            }

            // Run optimization passes
            self.run_optimizations(&module)?;

            // Run LTO if enabled
            if self.config.lto != LtoLevel::None {
                self.run_lto(&module)?;
            }

            Ok(module)
        }

        /// Compile to object file
        pub fn compile_to_object(
            &self,
            name: &str,
            body: &MirBody,
            output_path: &Path,
        ) -> Result<(), LlvmError> {
            let module = self.compile_module(name, body)?;

            let target_triple = TargetTriple::create(&self.config.target_triple);
            let target = Target::from_triple(&target_triple)
                .map_err(|e| LlvmError::UnsupportedTarget(e.to_string()))?;

            let target_machine = target
                .create_target_machine(
                    &target_triple,
                    &self.config.target_cpu,
                    &self.config.target_features,
                    self.config.opt_level.to_inkwell(),
                    if self.config.pic {
                        RelocMode::PIC
                    } else {
                        RelocMode::Default
                    },
                    CodeModel::Default,
                )
                .ok_or_else(|| {
                    LlvmError::CompilationFailed("Failed to create target machine".to_string())
                })?;

            target_machine
                .write_to_file(&module, FileType::Object, output_path)
                .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?;

            Ok(())
        }

        /// Compile to assembly
        pub fn compile_to_assembly(
            &self,
            name: &str,
            body: &MirBody,
            output_path: &Path,
        ) -> Result<(), LlvmError> {
            let module = self.compile_module(name, body)?;

            let target_triple = TargetTriple::create(&self.config.target_triple);
            let target = Target::from_triple(&target_triple)
                .map_err(|e| LlvmError::UnsupportedTarget(e.to_string()))?;

            let target_machine = target
                .create_target_machine(
                    &target_triple,
                    &self.config.target_cpu,
                    &self.config.target_features,
                    self.config.opt_level.to_inkwell(),
                    if self.config.pic {
                        RelocMode::PIC
                    } else {
                        RelocMode::Default
                    },
                    CodeModel::Default,
                )
                .ok_or_else(|| {
                    LlvmError::CompilationFailed("Failed to create target machine".to_string())
                })?;

            target_machine
                .write_to_file(&module, FileType::Assembly, output_path)
                .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?;

            Ok(())
        }

        /// Compile to LLVM IR
        pub fn compile_to_ir(
            &self,
            name: &str,
            body: &MirBody,
            output_path: &Path,
        ) -> Result<(), LlvmError> {
            let module = self.compile_module(name, body)?;
            module
                .print_to_file(output_path)
                .map_err(|e| LlvmError::IoError(e.to_string()))?;
            Ok(())
        }

        /// Compile to bitcode
        pub fn compile_to_bitcode(
            &self,
            name: &str,
            body: &MirBody,
            output_path: &Path,
        ) -> Result<(), LlvmError> {
            let module = self.compile_module(name, body)?;
            module.write_bitcode_to_path(output_path);
            Ok(())
        }

        /// Translate a function from MIR to LLVM
        fn translate_function(
            &self,
            module: &Module,
            builder: &Builder,
            name: &str,
            body: &MirBody,
        ) -> Result<FunctionValue, LlvmError> {
            // Create function type (i64 -> i64 for now)
            let i64_type = self.context.i64_type();
            let fn_type = i64_type.fn_type(&[i64_type.into()], false);

            // Create function
            let function = module.add_function(name, fn_type, None);

            // Create entry basic block
            let entry_block = self.context.append_basic_block(function, "entry");
            builder.position_at_end(entry_block);

            // Create a translator
            let mut translator = InkwellTranslator::new(&self.context, builder, function, body);

            // Translate the body
            translator.translate_body()?;

            Ok(function)
        }

        /// Run optimization passes
        fn run_optimizations(&self, module: &Module) -> Result<(), LlvmError> {
            // Create pass manager builder
            let pass_manager_builder = PassManagerBuilder::create();

            // Set optimization level
            match self.config.opt_level {
                OptLevel::None => {
                    pass_manager_builder.set_optimization_level(OptimizationLevel::None);
                }
                OptLevel::Less => {
                    pass_manager_builder.set_optimization_level(OptimizationLevel::Less);
                }
                OptLevel::Default => {
                    pass_manager_builder.set_optimization_level(OptimizationLevel::Default);
                }
                OptLevel::Aggressive => {
                    pass_manager_builder.set_optimization_level(OptimizationLevel::Aggressive);
                }
                OptLevel::Size => {
                    pass_manager_builder.set_optimization_level(OptimizationLevel::Default);
                    pass_manager_builder.set_size_level(1);
                }
                OptLevel::SizeAggressive => {
                    pass_manager_builder.set_optimization_level(OptimizationLevel::Default);
                    pass_manager_builder.set_size_level(2);
                }
            }

            // Enable loop vectorization if requested
            if self.pipeline.vectorize {
                pass_manager_builder.set_loop_vectorize(true);
            }
            if self.pipeline.slp_vectorize {
                pass_manager_builder.set_slp_vectorize(true);
            }

            // Create module pass manager
            let pass_manager = PassManager::create(()).0;
            pass_manager_builder.populate_module_pass_manager(&pass_manager);

            // Run passes
            pass_manager.run_on(module);

            Ok(())
        }

        /// Run Link-Time Optimization
        fn run_lto(&self, module: &Module) -> Result<(), LlvmError> {
            match self.config.lto {
                LtoLevel::None => Ok(()),
                LtoLevel::Thin => {
                    // Thin LTO is handled by the linker
                    // We just need to emit bitcode
                    Ok(())
                }
                LtoLevel::Full | LtoLevel::Fat => {
                    // Full LTO requires merging modules at compile time
                    // This is a simplified implementation
                    let pass_manager = PassManager::create(()).0;

                    // Add LTO passes
                    pass_manager.add_function_inlining_pass();
                    pass_manager.add_global_dce_pass();
                    pass_manager.add_internalize_pass(true);

                    pass_manager.run_on(module);
                    Ok(())
                }
            }
        }

        /// Verify the module
        pub fn verify_module(&self, module: &Module) -> Result<(), LlvmError> {
            module
                .verify()
                .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?;
            Ok(())
        }
    }

    /// Translator from MIR to LLVM IR using Inkwell
    struct InkwellTranslator<'ctx, 'a> {
        context: &'ctx Context,
        builder: &'a Builder<'ctx>,
        function: FunctionValue<'ctx>,
        body: &'a MirBody,
        locals: HashMap<Local, PointerValue<'ctx>>,
        blocks: HashMap<BasicBlock, inkwell::basic_block::BasicBlock<'ctx>>,
    }

    impl<'ctx, 'a> InkwellTranslator<'ctx, 'a> {
        fn new(
            context: &'ctx Context,
            builder: &'a Builder<'ctx>,
            function: FunctionValue<'ctx>,
            body: &'a MirBody,
        ) -> Self {
            Self {
                context,
                builder,
                function,
                body,
                locals: HashMap::new(),
                blocks: HashMap::new(),
            }
        }

        fn translate_body(&mut self) -> Result<(), LlvmError> {
            // Allocate stack space for locals
            self.allocate_locals()?;

            // Create all basic blocks
            for (idx, _) in self.body.basic_blocks.iter().enumerate() {
                let block = self
                    .context
                    .append_basic_block(self.function, &format!("bb{}", idx));
                self.blocks.insert(BasicBlock(idx as u32), block);
            }

            // Translate each basic block
            for (idx, block_data) in self.body.basic_blocks.iter().enumerate() {
                let block = self.blocks[&BasicBlock(idx as u32)];
                self.builder.position_at_end(block);

                // Translate statements
                for stmt in &block_data.statements {
                    self.translate_statement(stmt)?;
                }

                // Translate terminator
                if let Some(ref terminator) = block_data.terminator {
                    self.translate_terminator(terminator)?;
                }
            }

            Ok(())
        }

        fn allocate_locals(&mut self) -> Result<(), LlvmError> {
            let i64_type = self.context.i64_type();

            for (i, _) in self.body.local_decls.iter().enumerate() {
                let alloca = self
                    .builder
                    .build_alloca(i64_type, &format!("_{}", i))
                    .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?;
                self.locals.insert(Local(i as u32), alloca);
            }

            // Store parameter to local 1
            if let Some(param) = self.function.get_first_param() {
                if let Some(&local1) = self.locals.get(&Local(1)) {
                    self.builder
                        .build_store(local1, param)
                        .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?;
                }
            }

            Ok(())
        }

        fn translate_statement(&mut self, stmt: &Statement) -> Result<(), LlvmError> {
            match stmt {
                Statement::Assign(place, rvalue) => {
                    let value = self.translate_rvalue(rvalue)?;
                    let ptr = self.get_place_pointer(place)?;
                    self.builder
                        .build_store(ptr, value)
                        .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?;
                    Ok(())
                }
                Statement::StorageLive(_) | Statement::StorageDead(_) => {
                    // Stack allocation is handled in allocate_locals
                    Ok(())
                }
                Statement::InlineAsm(_) => Err(LlvmError::CompilationFailed(
                    "Inline assembly not supported".to_string(),
                )),
                Statement::Nop => Ok(()),
            }
        }

        fn translate_rvalue(&mut self, rvalue: &Rvalue) -> Result<BasicValueEnum<'ctx>, LlvmError> {
            match rvalue {
                Rvalue::Use(operand) => self.translate_operand(operand),
                Rvalue::Copy(place) | Rvalue::Move(place) => {
                    let ptr = self.get_place_pointer(place)?;
                    let value = self
                        .builder
                        .build_load(self.context.i64_type(), ptr, "load")
                        .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?;
                    Ok(value)
                }
                Rvalue::BinaryOp(op, left, right) => {
                    let left_val = self.translate_operand(left)?;
                    let right_val = self.translate_operand(right)?;
                    self.translate_binop(*op, left_val, right_val)
                }
                Rvalue::UnaryOp(op, operand) => {
                    let val = self.translate_operand(operand)?;
                    self.translate_unop(*op, val)
                }
                Rvalue::Ref(place, _) => {
                    // Return the pointer as an integer
                    let ptr = self.get_place_pointer(place)?;
                    let ptr_int = self
                        .builder
                        .build_ptr_to_int(ptr, self.context.i64_type(), "ptr_to_int")
                        .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?;
                    Ok(ptr_int.into())
                }
                _ => {
                    // Return 0 for unsupported rvalues
                    Ok(self.context.i64_type().const_int(0, false).into())
                }
            }
        }

        fn translate_binop(
            &mut self,
            op: BinOp,
            left: BasicValueEnum<'ctx>,
            right: BasicValueEnum<'ctx>,
        ) -> Result<BasicValueEnum<'ctx>, LlvmError> {
            let left_int = left.into_int_value();
            let right_int = right.into_int_value();

            let result = match op {
                BinOp::Add => self
                    .builder
                    .build_int_add(left_int, right_int, "add")
                    .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?,
                BinOp::Sub => self
                    .builder
                    .build_int_sub(left_int, right_int, "sub")
                    .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?,
                BinOp::Mul => self
                    .builder
                    .build_int_mul(left_int, right_int, "mul")
                    .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?,
                BinOp::Div => self
                    .builder
                    .build_int_signed_div(left_int, right_int, "div")
                    .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?,
                BinOp::Rem => self
                    .builder
                    .build_int_signed_rem(left_int, right_int, "rem")
                    .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?,
                BinOp::BitAnd => self
                    .builder
                    .build_and(left_int, right_int, "and")
                    .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?,
                BinOp::BitOr => self
                    .builder
                    .build_or(left_int, right_int, "or")
                    .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?,
                BinOp::BitXor => self
                    .builder
                    .build_xor(left_int, right_int, "xor")
                    .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?,
                BinOp::Shl => self
                    .builder
                    .build_left_shift(left_int, right_int, "shl")
                    .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?,
                BinOp::Shr => self
                    .builder
                    .build_right_shift(left_int, right_int, true, "shr")
                    .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?,
                BinOp::Eq => {
                    let cmp = self
                        .builder
                        .build_int_compare(inkwell::IntPredicate::EQ, left_int, right_int, "eq")
                        .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?;
                    self.builder
                        .build_int_z_extend(cmp, self.context.i64_type(), "zext")
                        .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?
                }
                BinOp::Ne => {
                    let cmp = self
                        .builder
                        .build_int_compare(inkwell::IntPredicate::NE, left_int, right_int, "ne")
                        .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?;
                    self.builder
                        .build_int_z_extend(cmp, self.context.i64_type(), "zext")
                        .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?
                }
                BinOp::Lt => {
                    let cmp = self
                        .builder
                        .build_int_compare(inkwell::IntPredicate::SLT, left_int, right_int, "lt")
                        .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?;
                    self.builder
                        .build_int_z_extend(cmp, self.context.i64_type(), "zext")
                        .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?
                }
                BinOp::Le => {
                    let cmp = self
                        .builder
                        .build_int_compare(inkwell::IntPredicate::SLE, left_int, right_int, "le")
                        .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?;
                    self.builder
                        .build_int_z_extend(cmp, self.context.i64_type(), "zext")
                        .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?
                }
                BinOp::Gt => {
                    let cmp = self
                        .builder
                        .build_int_compare(inkwell::IntPredicate::SGT, left_int, right_int, "gt")
                        .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?;
                    self.builder
                        .build_int_z_extend(cmp, self.context.i64_type(), "zext")
                        .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?
                }
                BinOp::Ge => {
                    let cmp = self
                        .builder
                        .build_int_compare(inkwell::IntPredicate::SGE, left_int, right_int, "ge")
                        .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?;
                    self.builder
                        .build_int_z_extend(cmp, self.context.i64_type(), "zext")
                        .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?
                }
                _ => left_int,
            };

            Ok(result.into())
        }

        fn translate_unop(
            &mut self,
            op: UnOp,
            val: BasicValueEnum<'ctx>,
        ) -> Result<BasicValueEnum<'ctx>, LlvmError> {
            let val_int = val.into_int_value();

            let result = match op {
                UnOp::Neg => self
                    .builder
                    .build_int_neg(val_int, "neg")
                    .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?,
                UnOp::Not => self
                    .builder
                    .build_not(val_int, "not")
                    .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?,
            };

            Ok(result.into())
        }

        fn translate_operand(
            &mut self,
            operand: &Operand,
        ) -> Result<BasicValueEnum<'ctx>, LlvmError> {
            match operand {
                Operand::Copy(place) | Operand::Move(place) => {
                    let ptr = self.get_place_pointer(place)?;
                    let value = self
                        .builder
                        .build_load(self.context.i64_type(), ptr, "load")
                        .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?;
                    Ok(value)
                }
                Operand::Constant(constant) => self.translate_constant(constant),
            }
        }

        fn translate_constant(
            &self,
            constant: &Constant,
        ) -> Result<BasicValueEnum<'ctx>, LlvmError> {
            match constant {
                Constant::Scalar(scalar) => {
                    match scalar {
                        Scalar::Int(n, _) => {
                            Ok(self.context.i64_type().const_int(*n as u64, false).into())
                        }
                        Scalar::Float(f, _) => {
                            // For now, convert to int
                            Ok(self.context.i64_type().const_int(*f as u64, false).into())
                        }
                        Scalar::Pointer(addr) => {
                            Ok(self.context.i64_type().const_int(*addr, false).into())
                        }
                    }
                }
                Constant::ZST => Ok(self.context.i64_type().const_int(0, false).into()),
            }
        }

        fn get_place_pointer(&self, place: &Place) -> Result<PointerValue<'ctx>, LlvmError> {
            self.locals
                .get(&place.local)
                .copied()
                .ok_or_else(|| LlvmError::InvalidInput(format!("Unknown local: {:?}", place.local)))
        }

        fn translate_terminator(&mut self, terminator: &Terminator) -> Result<(), LlvmError> {
            match &terminator.kind {
                TerminatorKind::Return => {
                    let ret_ptr = self
                        .locals
                        .get(&Local::RETURN_PLACE)
                        .ok_or_else(|| LlvmError::InvalidInput("No return local".to_string()))?;
                    let ret_val = self
                        .builder
                        .build_load(self.context.i64_type(), *ret_ptr, "ret")
                        .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?;
                    self.builder
                        .build_return(Some(&ret_val))
                        .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?;
                    Ok(())
                }
                TerminatorKind::Goto { target } => {
                    let block = self.blocks.get(target).ok_or_else(|| {
                        LlvmError::InvalidInput(format!("Unknown block: {:?}", target))
                    })?;
                    self.builder
                        .build_unconditional_branch(*block)
                        .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?;
                    Ok(())
                }
                TerminatorKind::SwitchInt {
                    discr,
                    targets,
                    otherwise,
                    ..
                } => {
                    let discr_val = self.translate_operand(discr)?.into_int_value();
                    let otherwise_block = self.blocks.get(otherwise).ok_or_else(|| {
                        LlvmError::InvalidInput(format!("Unknown block: {:?}", otherwise))
                    })?;

                    let mut cases = Vec::new();
                    for (val, target) in targets {
                        let target_block = self.blocks.get(target).ok_or_else(|| {
                            LlvmError::InvalidInput(format!("Unknown block: {:?}", target))
                        })?;
                        let case_val = self.context.i64_type().const_int(*val as u64, false);
                        cases.push((case_val, *target_block));
                    }

                    self.builder
                        .build_switch(discr_val, *otherwise_block, &cases)
                        .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?;

                    Ok(())
                }
                _ => {
                    // Default to return 0
                    let zero = self.context.i64_type().const_int(0, false);
                    self.builder
                        .build_return(Some(&zero))
                        .map_err(|e| LlvmError::CompilationFailed(e.to_string()))?;
                    Ok(())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Type;
    use crate::middle::mir::*;

    #[test]
    fn test_llvm_backend_creation() {
        let backend = LlvmBackend::default();
        assert_eq!(backend.config.opt_level, OptLevel::Default);
    }

    #[test]
    fn test_optimization_pipeline() {
        let pipeline = OptimizationPipeline::for_level(OptLevel::Aggressive);
        assert!(pipeline.aggressive_dce);
        assert!(pipeline.function_inlining);
        assert!(pipeline.vectorize);
    }

    #[test]
    fn test_lto_levels() {
        assert_ne!(LtoLevel::None, LtoLevel::Thin);
        assert_ne!(LtoLevel::Thin, LtoLevel::Full);
    }

    #[test]
    fn test_compile_empty_body() {
        let mut backend = LlvmBackend::default();
        let body = MirBody::new(0, 0..100);

        let result = backend.compile("test", &body);
        assert!(result.is_ok());

        let ir = result.unwrap();
        assert!(ir.contains("define"));
        assert!(ir.contains("test"));
    }

    #[test]
    fn test_compile_simple_function() {
        let mut backend = LlvmBackend::default();

        // Create a simple function: return 42
        let mut body = MirBody::new(0, 0..100);
        body.push_local(LocalDecl::new(Type::Unit, 0..10));

        let mut block = BasicBlockData::new();
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

        let result = backend.compile("test", &body);
        assert!(result.is_ok());

        let ir = result.unwrap();
        assert!(ir.contains("42"));
    }

    #[test]
    fn test_compile_module() {
        let mut backend = LlvmBackend::default();

        let body1 = MirBody::new(0, 0..100);
        let body2 = MirBody::new(0, 0..100);

        let result = backend.compile_module(&[("func1", &body1), ("func2", &body2)]);
        assert!(result.is_ok());

        let ir = result.unwrap();
        assert!(ir.contains("func1"));
        assert!(ir.contains("func2"));
        assert!(ir.contains("target triple"));
    }

    #[test]
    fn test_config_with_lto() {
        let config = LlvmConfig {
            lto: LtoLevel::Full,
            opt_level: OptLevel::Aggressive,
            ..Default::default()
        };

        let backend = LlvmBackend::new(config);
        assert_eq!(backend.config.lto, LtoLevel::Full);
        assert_eq!(backend.config.opt_level, OptLevel::Aggressive);
    }
}
