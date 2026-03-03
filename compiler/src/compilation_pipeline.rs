use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::ast;
use crate::codegen::generate_c;
use crate::compiler_detector::{CCompilerDetector, CCompilerRunner, CompileOptions};
use crate::frontend::lexer::tokenize;
use crate::frontend::parser::parse;
use crate::typeck::type_check;
use crate::typeck::sym::SymbolTable;
use crate::ui::{error, final_error, final_success, header, info, section, success, warning};
use crate::module::loader::StdLoader;
use crate::middle::mir::optimize::OptimizationPipeline;

pub struct CompileRequest {
    pub file: PathBuf,
    pub output: Option<PathBuf>,
    pub target: Option<String>,
    pub opt_level: u8,
    pub emit_llvm: bool,
    pub emit_asm: bool,
    pub emit_c: bool,
    pub lex_only: bool,
    pub parse_only: bool,
    pub check_only: bool,
    pub std_path: Option<PathBuf>,
    pub no_std: bool,
}

pub struct CompilationPipeline;

impl CompilationPipeline {
    pub fn compile(req: CompileRequest) -> Result<()> {
        Self::validate(&req)?;

        let source = Self::read_source(&req.file)?;

        Self::print_header(&req);

        let tokens = Self::lexical_analysis(&source)?;

        if req.lex_only {
            return Self::output_tokens(&tokens);
        }

        let ast = Self::parsing(&source)?;

        if req.parse_only {
            return Self::output_ast(&ast);
        }

        let mut symbol_table = Self::load_std(&req)?;
        
        Self::type_checking_with_symbols(&ast, &mut symbol_table)?;

        if req.check_only {
            return Self::output_type_check_success();
        }

        let c_code = Self::code_generation(&ast)?;

        if req.emit_c {
            return Self::output_c_code(&c_code, &req);
        }

        if req.emit_llvm {
            warning("LLVM IR output not yet implemented");
            return Ok(());
        }

        if req.emit_asm {
            warning("Assembly output not yet implemented");
            return Ok(());
        }

        Self::compile_to_executable(&c_code, &req)
    }

    fn validate(req: &CompileRequest) -> Result<()> {
        if req.opt_level > 3 {
            return Err(anyhow::anyhow!("Optimization level must be between 0 and 3"));
        }

        if !req.file.exists() {
            return Err(anyhow::anyhow!("File not found: {}", req.file.display()));
        }

        if req.file.extension() != Some(std::ffi::OsStr::new("box")) {
            warning("File does not have .box extension");
        }

        Ok(())
    }

    fn read_source(file: &PathBuf) -> Result<String> {
        fs::read_to_string(file)
            .with_context(|| format!("Failed to read file: {}", file.display()))
    }

    fn print_header(req: &CompileRequest) {
        header("BoxLang Compiler");
        section("Build Configuration");
        info("Source", &req.file.display().to_string());
        info("Target", req.target.as_deref().unwrap_or("native"));
        info("Opt Level", &format!("-O{}", req.opt_level));
        if req.no_std {
            info("Std", "disabled");
        }
        if req.emit_llvm {
            info("Emit", "LLVM IR");
        } else if req.emit_asm {
            info("Emit", "Assembly");
        } else if req.emit_c {
            info("Emit", "C code");
        }
        println!();
    }

    fn load_std(req: &CompileRequest) -> Result<SymbolTable> {
        if req.no_std {
            section("Standard Library");
            warning("Standard library disabled (--no-std)");
            println!();
            return Ok(SymbolTable::new());
        }

        section("Standard Library");
        
        let std_path = req.std_path.clone().or_else(|| {
            Self::find_std_path()
        });

        if let Some(path) = &std_path {
            info("Std Path", &path.display().to_string());
            
            let mut loader = StdLoader::new();
            let mut symbol_table = SymbolTable::new();
            
            match loader.load_core(path) {
                Ok(modules) => {
                    for (name, ast) in modules {
                        symbol_table.register_module(&name.split("::").map(String::from).collect::<Vec<_>>());
                        if let Err(e) = type_check(&ast) {
                            warning(&format!("Type check warning in {}: {}", name, e));
                        }
                    }
                    success("Loaded core library");
                }
                Err(e) => {
                    warning(&format!("Could not load core: {}", e));
                }
            }

            match loader.load_std(path) {
                Ok(modules) => {
                    for (name, ast) in modules {
                        symbol_table.register_module(&name.split("::").map(String::from).collect::<Vec<_>>());
                        if let Err(e) = type_check(&ast) {
                            warning(&format!("Type check warning in {}: {}", name, e));
                        }
                    }
                    success("Loaded std library");
                }
                Err(e) => {
                    warning(&format!("Could not load std: {}", e));
                }
            }

            match loader.load_boxos(path) {
                Ok(modules) => {
                    for (name, ast) in modules {
                        symbol_table.register_module(&name.split("::").map(String::from).collect::<Vec<_>>());
                        if let Err(e) = type_check(&ast) {
                            warning(&format!("Type check warning in {}: {}", name, e));
                        }
                    }
                    success("Loaded boxos library");
                }
                Err(e) => {
                    warning(&format!("Could not load boxos: {}", e));
                }
            }

            symbol_table.set_prelude_loaded(true);
            println!();
            Ok(symbol_table)
        } else {
            warning("Standard library not found, using builtins only");
            println!();
            Ok(SymbolTable::new())
        }
    }

    fn find_std_path() -> Option<PathBuf> {
        if let Ok(path) = std::env::var("BOXLANG_STD_PATH") {
            let path = PathBuf::from(path);
            if path.exists() {
                return Some(path);
            }
        }

        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                let std_path = parent.join("../std");
                if std_path.exists() {
                    return Some(std_path);
                }
                
                let std_path = parent.join("std");
                if std_path.exists() {
                    return Some(std_path);
                }
            }
        }

        if let Ok(cwd) = std::env::current_dir() {
            let candidates = vec![
                cwd.join("std"),
                cwd.join("../std"),
            ];
            
            for candidate in candidates {
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }

        None
    }

    fn lexical_analysis(source: &str) -> Result<Vec<crate::frontend::lexer::token::SpannedToken>> {
        section("Lexical Analysis");
        let tokens = tokenize(source);
        success(&format!("Tokenized {} tokens", tokens.len()));
        println!();
        Ok(tokens)
    }

    fn output_tokens(
        tokens: &[crate::frontend::lexer::token::SpannedToken],
    ) -> Result<()> {
        header("Token List");
        for (i, token) in tokens.iter().enumerate() {
            println!(
                "    [{:>3}] {:<20} at {}:{}",
                i,
                format!("{:?}", token.token),
                token.line,
                token.column
            );
        }
        final_success("Lexical analysis complete");
        Ok(())
    }

    fn parsing(source: &str) -> Result<ast::Module> {
        section("Parsing");
        let ast = parse(source).with_context(|| "Parsing failed")?;
        success(&format!("Parsed {} item(s)", ast.items.len()));
        println!();
        Ok(ast)
    }

    fn output_ast(ast: &ast::Module) -> Result<()> {
        header("Abstract Syntax Tree");
        info("Module", &ast.name);
        info("Items", &ast.items.len().to_string());
        for item in &ast.items {
            let item_desc = Self::describe_item(item);
            println!(
                "      {} {}",
                crate::ui::ui().color("*", crate::ui::colors::BRIGHT_BLACK),
                item_desc
            );
        }
        final_success("Parsing complete");
        Ok(())
    }

    fn describe_item(item: &ast::Item) -> String {
        match item {
            ast::Item::Function(f) => format!("Function: {}", f.name),
            ast::Item::Struct(s) => format!("Struct: {}", s.name),
            ast::Item::Union(u) => format!("Union: {}", u.name),
            ast::Item::Enum(e) => format!("Enum: {}", e.name),
            ast::Item::Impl(_) => "Impl block".to_string(),
            ast::Item::Trait(t) => format!("Trait: {}", t.name),
            ast::Item::Import(i) => {
                let path: Vec<String> = i.path.segments.iter().map(|s| s.ident.to_string()).collect();
                if i.is_glob {
                    format!("Import: {}::*", path.join("::"))
                } else if let Some(alias) = &i.alias {
                    format!("Import: {} as {}", path.join("::"), alias)
                } else {
                    format!("Import: {}", path.join("::"))
                }
            }
            ast::Item::Const(c) => format!("Const: {}", c.name),
            ast::Item::Static(s) => format!("Static: {}", s.name),
            ast::Item::TypeAlias(t) => format!("Type: {}", t.name),
            ast::Item::Module(m) => format!("Module: {}", m.name),
            ast::Item::ExternBlock(_) => "Extern block".to_string(),
            ast::Item::MacroRules(_) => "Macro rules".to_string(),
            ast::Item::Callback(c) => format!("Callback: {}", c.name),
            ast::Item::SafeWrapper(w) => format!("Safe wrapper: {}", w.wrapper_name),
        }
    }

    fn type_checking_with_symbols(ast: &ast::Module, _symbol_table: &mut SymbolTable) -> Result<()> {
        section("Type Checking");
        match type_check(ast) {
            Ok(_) => {
                success("Type check passed");
            }
            Err(e) => {
                error(&format!("Type check failed: {}", e));
                final_error("Compilation failed");
                return Err(anyhow::anyhow!("Type check failed"));
            }
        }
        println!();
        Ok(())
    }

    fn type_checking(ast: &ast::Module) -> Result<()> {
        section("Type Checking");
        match type_check(ast) {
            Ok(_) => {
                success("Type check passed");
            }
            Err(e) => {
                error(&format!("Type check failed: {}", e));
                final_error("Compilation failed");
                return Err(anyhow::anyhow!("Type check failed"));
            }
        }
        println!();
        Ok(())
    }

    fn output_type_check_success() -> Result<()> {
        final_success("Type checking complete");
        Ok(())
    }

    fn code_generation(ast: &ast::Module) -> Result<String> {
        section("Code Generation");
        
        let optimization_pipeline = OptimizationPipeline::from_level(0);
        let pass_count = optimization_pipeline.pass_count();
        if pass_count > 0 {
            info("MIR Optimization", &format!("{} passes", pass_count));
        }
        
        let c_code = generate_c(ast).map_err(|e| anyhow::anyhow!("Code generation failed: {}", e))?;
        success("C code generation complete");
        println!();
        Ok(c_code)
    }

    fn output_c_code(c_code: &str, req: &CompileRequest) -> Result<()> {
        let output_path = Self::determine_output_path(req);
        fs::write(&output_path, c_code)
            .with_context(|| format!("Failed to write C code to {}", output_path.display()))?;
        info("Output", &output_path.display().to_string());
        final_success("C code generation complete");
        Ok(())
    }

    fn compile_to_executable(c_code: &str, req: &CompileRequest) -> Result<()> {
        let output_path = Self::determine_output_path(req);

        let c_file = output_path.with_extension("c");
        fs::write(&c_file, c_code)
            .with_context(|| format!("Failed to write C code to {}", c_file.display()))?;
        info("Intermediate", &c_file.display().to_string());

        section("Compiling Executable");
        let compiler = CCompilerDetector::detect()?;
        let opts = CompileOptions {
            opt_level: req.opt_level,
            debug: req.opt_level == 0,
        };
        CCompilerRunner::compile_to_exe(&compiler, &c_file, &output_path, &opts)?;
        success("Executable generated");
        println!();

        info("Output", &output_path.display().to_string());

        final_success(&format!(
            "{} compiled successfully",
            req.file.file_name().unwrap_or_default().to_string_lossy()
        ));

        Ok(())
    }

    fn determine_output_path(req: &CompileRequest) -> PathBuf {
        req.output.clone().unwrap_or_else(|| {
            let mut path = req.file.clone();
            if req.emit_llvm {
                path.set_extension("ll");
            } else if req.emit_asm {
                path.set_extension("s");
            } else if req.emit_c {
                path.set_extension("c");
            } else {
                path.set_extension("exe");
            }
            path
        })
    }
}

use std::fs;
