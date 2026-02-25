//! Code Generation Module
//!
//! This module provides multiple backends for code generation:
//! - C: Generates C code (for bootstrapping)
//! - Cranelift: Fast compilation for debug builds
//! - LLVM: Optimized compilation for release builds
//!
//! Dual Backend Architecture:
//! - Cranelift: Used for debug builds and JIT compilation (fast compilation)
//! - LLVM: Used for release builds (optimized code generation)

use crate::ast::*;
use crate::middle::mir::MirBody;

pub mod c;
pub mod cranelift;
pub mod llvm;

pub use c::CCodeGen;
pub use cranelift::{
    CraneliftBackend, CraneliftConfig, CraneliftError, OptLevel as CraneliftOptLevel,
};
pub use llvm::{LlvmBackend, LlvmConfig, LlvmError, OptLevel as LlvmOptLevel};

// Re-export JIT types
#[cfg(feature = "inkwell")]
pub use llvm::jit::{JitConfig, JitContext, JitType, JitValue};

/// Trait for code generators
pub trait CodeGen {
    type Output;
    type Error;

    /// Generate code from a module
    fn generate(&mut self, module: &Module) -> Result<Self::Output, Self::Error>;
}

/// Generate C code from a module
pub fn generate_c(module: &Module) -> Result<String, String> {
    let mut codegen = CCodeGen::new();
    codegen.generate(module)
}

/// Backend selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// C code generation
    C,
    /// Cranelift backend (fast debug builds)
    Cranelift,
    /// LLVM backend (optimized release builds)
    Llvm,
}

impl Backend {
    /// Get the default backend based on build profile
    pub fn default_for_profile(is_release: bool) -> Self {
        if is_release {
            Backend::Llvm
        } else {
            Backend::Cranelift
        }
    }

    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            Backend::C => "C",
            Backend::Cranelift => "Cranelift",
            Backend::Llvm => "LLVM",
        }
    }
}

/// Optimization levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptLevel {
    /// No optimization
    None,
    /// Basic optimizations
    Basic,
    /// Standard optimizations
    Standard,
    /// Aggressive optimizations
    Aggressive,
}

impl OptLevel {
    /// Convert to numeric level
    pub fn as_u8(&self) -> u8 {
        match self {
            OptLevel::None => 0,
            OptLevel::Basic => 1,
            OptLevel::Standard => 2,
            OptLevel::Aggressive => 3,
        }
    }

    /// Create from numeric level
    pub fn from_u8(level: u8) -> Self {
        match level {
            0 => OptLevel::None,
            1 => OptLevel::Basic,
            2 => OptLevel::Standard,
            _ => OptLevel::Aggressive,
        }
    }
}

/// Compiler configuration
#[derive(Debug, Clone)]
pub struct CompilerConfig {
    /// Selected backend
    pub backend: Backend,
    /// Optimization level
    pub opt_level: OptLevel,
    /// Enable debug info
    pub debug_info: bool,
    /// Target triple
    pub target: Option<String>,
    /// Output file path
    pub output_path: Option<std::path::PathBuf>,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            backend: Backend::Cranelift,
            opt_level: OptLevel::None,
            debug_info: true,
            target: None,
            output_path: None,
        }
    }
}

impl CompilerConfig {
    /// Create configuration for debug builds
    pub fn debug() -> Self {
        Self {
            backend: Backend::Cranelift,
            opt_level: OptLevel::None,
            debug_info: true,
            target: None,
            output_path: None,
        }
    }

    /// Create configuration for release builds
    pub fn release() -> Self {
        Self {
            backend: Backend::Llvm,
            opt_level: OptLevel::Aggressive,
            debug_info: false,
            target: None,
            output_path: None,
        }
    }

    /// Set the backend
    pub fn with_backend(mut self, backend: Backend) -> Self {
        self.backend = backend;
        self
    }

    /// Set optimization level
    pub fn with_opt_level(mut self, level: OptLevel) -> Self {
        self.opt_level = level;
        self
    }

    /// Set target triple
    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    /// Set output path
    pub fn with_output(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.output_path = Some(path.into());
        self
    }
}

/// Compilation result
#[derive(Debug)]
pub struct CompilationResult {
    /// Generated code or path to output
    pub output: CompilationOutput,
    /// Backend used
    pub backend: Backend,
    /// Compilation time
    pub duration: std::time::Duration,
}

/// Compilation output type
#[derive(Debug)]
pub enum CompilationOutput {
    /// C source code
    CCode(String),
    /// LLVM IR
    LlvmIr(String),
    /// Object file path
    Object(std::path::PathBuf),
    /// Executable path
    Executable(std::path::PathBuf),
    /// JIT compiled function pointer
    JitPtr(*const u8),
}

/// Unified backend interface
pub enum CodegenBackend {
    C(CCodeGen),
    Cranelift(CraneliftBackend),
    Llvm(LlvmBackend),
}

impl CodegenBackend {
    /// Create a backend from config
    pub fn from_config(config: &CompilerConfig) -> Self {
        match config.backend {
            Backend::C => CodegenBackend::C(CCodeGen::new()),
            Backend::Cranelift => {
                let cf_config = CraneliftConfig {
                    opt_level: match config.opt_level {
                        OptLevel::None => CraneliftOptLevel::None,
                        OptLevel::Basic => CraneliftOptLevel::Basic,
                        OptLevel::Standard => CraneliftOptLevel::Standard,
                        OptLevel::Aggressive => CraneliftOptLevel::Aggressive,
                    },
                    target: config
                        .target
                        .clone()
                        .unwrap_or_else(|| "x86_64-unknown-linux-gnu".to_string()),
                    debug_info: config.debug_info,
                };
                CodegenBackend::Cranelift(CraneliftBackend::new(cf_config))
            }
            Backend::Llvm => {
                let llvm_config = LlvmConfig {
                    opt_level: match config.opt_level {
                        OptLevel::None => llvm::OptLevel::None,
                        OptLevel::Basic => llvm::OptLevel::Less,
                        OptLevel::Standard => llvm::OptLevel::Default,
                        OptLevel::Aggressive => llvm::OptLevel::Aggressive,
                    },
                    target_triple: config
                        .target
                        .clone()
                        .unwrap_or_else(|| "x86_64-unknown-linux-gnu".to_string()),
                    target_cpu: "generic".to_string(),
                    target_features: String::new(),
                    debug_info: config.debug_info,
                    output_type: llvm::OutputType::Object,
                    pic: true,
                    lto: llvm::LtoLevel::None,
                };
                CodegenBackend::Llvm(LlvmBackend::new(llvm_config))
            }
        }
    }

    /// Compile a single function
    pub fn compile_function(&mut self, name: &str, body: &MirBody) -> Result<String, CodegenError> {
        match self {
            CodegenBackend::C(_) => Err(CodegenError::Unsupported(
                "C backend doesn't support MIR compilation".to_string(),
            )),
            CodegenBackend::Cranelift(backend) => {
                let result = backend
                    .compile(body)
                    .map_err(|e| CodegenError::Cranelift(e))?;
                // Return the function pointer as a string representation
                Ok(format!("{:p}", result.ptr))
            }
            CodegenBackend::Llvm(backend) => backend
                .compile(name, body)
                .map_err(|e| CodegenError::Llvm(e)),
        }
    }

    /// Compile to object file (LLVM only)
    #[cfg(feature = "inkwell")]
    pub fn compile_to_object(
        &self,
        name: &str,
        body: &MirBody,
        path: &std::path::Path,
    ) -> Result<(), CodegenError> {
        match self {
            CodegenBackend::C(_) => Err(CodegenError::Unsupported(
                "C backend doesn't support object file generation".to_string(),
            )),
            CodegenBackend::Cranelift(_) => Err(CodegenError::Unsupported(
                "Cranelift backend doesn't support object file generation".to_string(),
            )),
            CodegenBackend::Llvm(_) => {
                #[cfg(feature = "inkwell")]
                {
                    use llvm::inkwell_backend::InkwellBackend;
                    let config = LlvmConfig::default();
                    let backend = InkwellBackend::new(config).map_err(|e| CodegenError::Llvm(e))?;
                    backend
                        .compile_to_object(name, body, path)
                        .map_err(|e| CodegenError::Llvm(e))
                }
                #[cfg(not(feature = "inkwell"))]
                {
                    Err(CodegenError::Unsupported(
                        "inkwell feature not enabled".to_string(),
                    ))
                }
            }
        }
    }

    /// Get backend name
    pub fn name(&self) -> &'static str {
        match self {
            CodegenBackend::C(_) => "C",
            CodegenBackend::Cranelift(_) => "Cranelift",
            CodegenBackend::Llvm(_) => "LLVM",
        }
    }
}

/// Unified error type for code generation
#[derive(Debug)]
pub enum CodegenError {
    /// Cranelift error
    Cranelift(CraneliftError),
    /// LLVM error
    Llvm(LlvmError),
    /// Unsupported operation
    Unsupported(String),
    /// Generic error
    Other(String),
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodegenError::Cranelift(e) => write!(f, "Cranelift error: {}", e),
            CodegenError::Llvm(e) => write!(f, "LLVM error: {}", e),
            CodegenError::Unsupported(msg) => write!(f, "Unsupported: {}", msg),
            CodegenError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for CodegenError {}

impl From<CraneliftError> for CodegenError {
    fn from(e: CraneliftError) -> Self {
        CodegenError::Cranelift(e)
    }
}

impl From<LlvmError> for CodegenError {
    fn from(e: LlvmError) -> Self {
        CodegenError::Llvm(e)
    }
}

/// Dual backend compiler - automatically selects appropriate backend
pub struct DualBackendCompiler {
    config: CompilerConfig,
}

impl DualBackendCompiler {
    /// Create a new dual backend compiler
    pub fn new(config: CompilerConfig) -> Self {
        Self { config }
    }

    /// Create for debug builds (uses Cranelift)
    pub fn debug() -> Self {
        Self::new(CompilerConfig::debug())
    }

    /// Create for release builds (uses LLVM)
    pub fn release() -> Self {
        Self::new(CompilerConfig::release())
    }

    /// Compile a function
    pub fn compile(&self, name: &str, body: &MirBody) -> Result<CompilationResult, CodegenError> {
        let start = std::time::Instant::now();

        let mut backend = CodegenBackend::from_config(&self.config);
        let output = backend.compile_function(name, body)?;

        let duration = start.elapsed();

        Ok(CompilationResult {
            output: CompilationOutput::LlvmIr(output),
            backend: self.config.backend,
            duration,
        })
    }

    /// Get the current backend type
    pub fn backend(&self) -> Backend {
        self.config.backend
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_selection() {
        assert_eq!(Backend::default_for_profile(false), Backend::Cranelift);
        assert_eq!(Backend::default_for_profile(true), Backend::Llvm);
    }

    #[test]
    fn test_opt_level() {
        assert_eq!(OptLevel::None.as_u8(), 0);
        assert_eq!(OptLevel::from_u8(0), OptLevel::None);
        assert_eq!(OptLevel::from_u8(3), OptLevel::Aggressive);
    }

    #[test]
    fn test_compiler_config() {
        let debug = CompilerConfig::debug();
        assert_eq!(debug.backend, Backend::Cranelift);
        assert_eq!(debug.opt_level, OptLevel::None);

        let release = CompilerConfig::release();
        assert_eq!(release.backend, Backend::Llvm);
        assert_eq!(release.opt_level, OptLevel::Aggressive);
    }
}
