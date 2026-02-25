use anyhow::{Context, Result};
use clap::{Parser as ClapParser, Subcommand};
use std::fs;
use std::path::PathBuf;

use boxlang_compiler::{
    compilation_pipeline::{CompilationPipeline, CompileRequest},
    compiler_detector::{CCompilerDetector, CCompilerRunner, CompileOptions},
    ui::{error, final_error, final_success, header, info, init_ui, section, success, warning},
    tokenize, parse, type_check, generate_c,
    ConstEvaluator, ConstValue,
    typeck::error::TypeError,
};

/// BoxLang Compiler - A systems programming language for Box Ecosystem
#[derive(ClapParser)]
#[command(name = "boxlang")]
#[command(about = "BoxLang Compiler - A systems programming language for Box Ecosystem")]
#[command(version = "0.1.0")]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Disable colored output
    #[arg(long, global = true)]
    no_color: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile a BoxLang source file to executable
    #[command(visible_alias = "c")]
    Compile {
        /// Source file to compile
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Output file path
        #[arg(short, long, value_name = "OUTPUT")]
        output: Option<PathBuf>,

        /// Target architecture (e.g., x86_64, aarch64)
        #[arg(short, long, value_name = "TARGET")]
        target: Option<String>,

        /// Optimization level (0-3)
        #[arg(short = 'O', long, value_name = "LEVEL", default_value = "0")]
        opt_level: u8,

        /// Emit LLVM IR instead of binary
        #[arg(long, group = "emit")]
        emit_llvm: bool,

        /// Emit assembly instead of binary
        #[arg(long, group = "emit")]
        emit_asm: bool,

        /// Emit C code instead of binary
        #[arg(long, group = "emit")]
        emit_c: bool,

        /// Emit MIR instead of binary
        #[arg(long, group = "emit")]
        emit_mir: bool,

        /// Only run the lexer (for debugging)
        #[arg(long, group = "stage")]
        lex: bool,

        /// Only run the parser (for debugging)
        #[arg(long, group = "stage")]
        parse: bool,

        /// Only run type checking (for debugging)
        #[arg(long, group = "stage")]
        check: bool,

        /// Skip borrow checking
        #[arg(long)]
        skip_borrow_check: bool,

        /// Path to standard library
        #[arg(long, value_name = "PATH")]
        std: Option<PathBuf>,

        /// Disable standard library
        #[arg(long)]
        no_std: bool,
    },

    /// Tokenize a source file and print tokens (for debugging)
    #[command(visible_alias = "t")]
    Tokenize {
        /// Source file to tokenize
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Parse a source file and print AST (for debugging)
    #[command(visible_alias = "a")]
    Ast {
        /// Source file to parse
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Output format (debug, json)
        #[arg(short, long, default_value = "debug")]
        format: String,
    },

    /// Initialize a new BoxLang project
    #[command(visible_alias = "n")]
    New {
        /// Project name
        #[arg(value_name = "NAME")]
        name: String,

        /// Project directory (defaults to project name)
        #[arg(short, long, value_name = "PATH")]
        path: Option<PathBuf>,

        /// Create a library project instead of executable
        #[arg(long)]
        lib: bool,

        /// Initialize git repository
        #[arg(long)]
        git: bool,
    },

    /// Build the current project
    #[command(visible_alias = "b")]
    Build {
        /// Build in release mode with optimizations
        #[arg(short, long)]
        release: bool,

        /// Target architecture
        #[arg(short, long, value_name = "TARGET")]
        target: Option<String>,

        /// Number of parallel jobs
        #[arg(short, long, value_name = "N")]
        jobs: Option<usize>,

        /// Build specific package
        #[arg(short, long, value_name = "SPEC")]
        package: Option<String>,
    },

    /// Run the compiled program
    #[command(visible_alias = "r")]
    Run {
        /// Build in release mode before running
        #[arg(short, long)]
        release: bool,

        /// Arguments to pass to the program
        #[arg(value_name = "ARGS")]
        args: Vec<String>,
    },

    /// Check the source code for errors without compiling
    #[command(visible_alias = "chk")]
    Check {
        /// Source file to check (defaults to src/main.box)
        #[arg(value_name = "FILE")]
        file: Option<PathBuf>,

        /// Check all files in project
        #[arg(long)]
        all: bool,
    },

    /// Format source files
    #[command(visible_alias = "f")]
    Fmt {
        /// Source files to format (defaults to all .box files)
        #[arg(value_name = "FILES")]
        files: Vec<PathBuf>,

        /// Check formatting without modifying files
        #[arg(long)]
        check: bool,

        /// Emit formatted code to stdout instead of writing to file
        #[arg(long)]
        stdout: bool,
    },

    /// Clean build artifacts
    Clean,

    /// Show project information
    #[command(visible_alias = "info")]
    Project,

    /// Show the BoxLang logo and version
    #[command(name = "logo")]
    Logo,

    /// Interactive REPL mode (experimental)
    #[command(visible_alias = "i")]
    Repl,

    /// Package the application as AppBox format
    #[command(visible_alias = "p")]
    Package {
        /// Output directory for the package
        #[arg(short, long, value_name = "DIR")]
        output: Option<PathBuf>,

        /// Application name
        #[arg(short, long, value_name = "NAME")]
        name: Option<String>,

        /// Application version
        #[arg(short, long, value_name = "VERSION", default_value = "1.0.0")]
        version: String,

        /// Application author
        #[arg(short, long, value_name = "AUTHOR")]
        author: Option<String>,

        /// Optimization level (0-3)
        #[arg(short = 'O', long, default_value = "2")]
        opt_level: u8,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize UI with verbose mode based on command
    let verbose = cli.verbose
        || matches!(
            &cli.command,
            Commands::Compile {
                lex: true,
                parse: true,
                ..
            } | Commands::Tokenize { .. }
                | Commands::Ast { .. }
        );
    init_ui(verbose);

    // Handle global no-color flag
    if cli.no_color {
        std::env::set_var("NO_COLOR", "1");
    }

    match cli.command {
        Commands::Compile {
            file,
            output,
            target,
            opt_level,
            emit_llvm,
            emit_asm,
            emit_c,
            lex,
            parse: parse_only,
            check: check_only,
            std,
            no_std,
            ..
        } => {
            CompilationPipeline::compile(CompileRequest {
                file,
                output,
                target,
                opt_level,
                emit_llvm,
                emit_asm,
                emit_c,
                lex_only: lex,
                parse_only,
                check_only,
                std_path: std,
                no_std,
            })?;
        }
        Commands::Tokenize { file, format } => {
            tokenize_file(file, &format)?;
        }
        Commands::Ast { file, format } => {
            parse_file(file, &format)?;
        }
        Commands::New {
            name,
            path,
            lib,
            git,
        } => {
            create_new_project(&name, path, lib, git)?;
        }
        Commands::Build {
            release,
            target,
            jobs,
            package,
        } => {
            build_project(release, target, jobs, package)?;
        }
        Commands::Run { release, args } => {
            run_project(release, args)?;
        }
        Commands::Check { file, all } => {
            check_file(file, all)?;
        }
        Commands::Fmt {
            files,
            check,
            stdout,
        } => {
            format_files(&files, check, stdout)?;
        }
        Commands::Clean => {
            clean_project()?;
        }
        Commands::Project => {
            show_project_info()?;
        }
        Commands::Logo => {
            show_box_logo();
        }
        Commands::Repl => {
            start_repl()?;
        }

        Commands::Package {
            output,
            name,
            version,
            author,
            opt_level,
        } => {
            package_appbox(output, name, version, author, opt_level)?;
        }
    }

    Ok(())
}



fn compile_c_to_exe(c_file: &PathBuf, output: &PathBuf, opt_level: u8) -> Result<()> {
    let compiler = CCompilerDetector::detect()?;
    let opts = CompileOptions {
        opt_level,
        debug: opt_level == 0,
    };
    CCompilerRunner::compile_to_exe(&compiler, c_file, output, &opts)
}

fn tokenize_file(file: PathBuf, format: &str) -> Result<()> {
    if !file.exists() {
        return Err(anyhow::anyhow!("File not found: {}", file.display()));
    }

    let source = fs::read_to_string(&file)
        .with_context(|| format!("Failed to read file: {}", file.display()))?;

    header("Tokenize");
    info("File", &file.display().to_string());
    println!();

    let tokens = tokenize(&source);

    match format {
        "json" => {
            let json: Vec<_> = tokens
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "token": format!("{:?}", t.token),
                        "line": t.line,
                        "column": t.column,
                        "span": {
                            "start": t.span.start,
                            "end": t.span.end
                        }
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        _ => {
            section(&format!("Found {} tokens:", tokens.len()));
            println!();
            for token in tokens {
                println!(
                    "  {:4}:{:<4}  {:?}",
                    token.line, token.column, token.token
                );
            }
        }
    }

    final_success("Tokenization complete");
    Ok(())
}

fn parse_file(file: PathBuf, format: &str) -> Result<()> {
    if !file.exists() {
        return Err(anyhow::anyhow!("File not found: {}", file.display()));
    }

    let source = fs::read_to_string(&file)
        .with_context(|| format!("Failed to read file: {}", file.display()))?;

    header("Parse");
    info("File", &file.display().to_string());
    println!();

    let ast = parse(&source).with_context(|| "Failed to parse source code")?;

    match format {
        "json" => {
            let json = format!("{:#?}", ast);
            println!("\n{}", json);
        }
        _ => {
            section("Abstract Syntax Tree");
            println!("\n{:#?}", ast);
        }
    }

    final_success("Parsing complete");
    Ok(())
}

fn create_new_project(name: &str, path: Option<PathBuf>, is_lib: bool, init_git: bool) -> Result<()> {
    let project_dir = path.unwrap_or_else(|| PathBuf::from(name));

    header("New Project");
    info("Name", name);
    info("Type", if is_lib { "library" } else { "executable" });
    info("Path", &project_dir.display().to_string());
    println!();

    // Check if directory already exists
    if project_dir.exists() {
        return Err(anyhow::anyhow!(
            "Directory '{}' already exists",
            project_dir.display()
        ));
    }

    section("Creating project structure...");

    // Create directory structure
    fs::create_dir_all(&project_dir)?;
    fs::create_dir_all(project_dir.join("src"))?;

    // Create main.box or lib.box
    let main_content = if is_lib {
        format!(
            r#"module {};

/// {} library
pub struct {} {{
    // Add your fields here
}}

impl {} {{
    /// Create a new instance
    pub fn new() -> Self {{
        Self {{ }}
    }}
}}
"#,
            name, name, name, name
        )
    } else {
        format!(
            r#"module {};

/// Main entry point
pub fn main() {{
    println("Hello, BoxLang!");
}}
"#,
            name
        )
    };

    let main_file = if is_lib { "lib.box" } else { "main.box" };
    fs::write(project_dir.join("src").join(main_file), main_content)?;
    success(&format!("Created src/{}", main_file));

    // Create box.toml
    let toml_content = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2024"
authors = []
description = "A BoxLang project"
license = "MIT"

[dependencies]
"#,
        name
    );

    fs::write(project_dir.join("box.toml"), toml_content)?;
    success("Created box.toml");

    // Create .gitignore
    let gitignore_content = r#"# Build artifacts
/target
*.exe
*.o
*.obj
*.ll
*.s

# IDE
.idea/
.vscode/
*.swp
*.swo
*~

# OS
.DS_Store
Thumbs.db
"#;

    fs::write(project_dir.join(".gitignore"), gitignore_content)?;
    success("Created .gitignore");

    // Initialize git repository if requested
    if init_git {
        use std::process::Command;
        let output = Command::new("git")
            .arg("init")
            .current_dir(&project_dir)
            .output();

        if output.is_ok() {
            success("Initialized git repository");
        } else {
            warning("Failed to initialize git repository (git not found)");
        }
    }

    // Create README.md
    let readme_content = format!(
        r#"# {}

A BoxLang project.

## Building

```bash
boxlang build
```

## Running

```bash
boxlang run
```

## Project Structure

- `src/` - Source code
- `box.toml` - Project configuration
"#,
        name
    );

    fs::write(project_dir.join("README.md"), readme_content)?;
    success("Created README.md");

    println!();
    final_success(&format!("Created {} project '{}'", if is_lib { "library" } else { "executable" }, name));

    println!("\nTo get started:");
    println!("  cd {}", project_dir.display());
    println!("  boxlang build");
    if !is_lib {
        println!("  boxlang run");
    }

    Ok(())
}

fn build_project(
    release: bool,
    target: Option<String>,
    jobs: Option<usize>,
    package: Option<String>,
) -> Result<()> {
    use std::process::Command;

    header("Building BoxLang Project");

    // Print build configuration
    section("Build Configuration");
    info("Mode", if release { "release" } else { "debug" });
    if let Some(t) = &target {
        info("Target", t);
    }
    if let Some(j) = jobs {
        info("Jobs", &j.to_string());
    }
    if let Some(p) = &package {
        info("Package", p);
    }
    println!();

    // Check for box.toml
    if !PathBuf::from("box.toml").exists() {
        return Err(anyhow::anyhow!(
            "No box.toml found. Are you in a BoxLang project directory?"
        ));
    }

    // Find all .box files in the project
    section("Scanning source files...");
    let box_files = find_box_files(".")?;

    if box_files.is_empty() {
        warning("No .box files found in project");
        return Ok(());
    }

    success(&format!("Found {} source file(s)", box_files.len()));
    for file in &box_files {
        println!("    * {}", file.display());
    }
    println!();

    // Create output directory
    let output_dir = PathBuf::from(if release {
        "target/release"
    } else {
        "target/debug"
    });
    fs::create_dir_all(&output_dir)?;

    // Compile each file to object file
    section("Compiling source files...");
    let mut obj_files = Vec::new();
    let mut compiled_count = 0;
    let mut failed_count = 0;

    for file in &box_files {
        print!("  Compiling {}...", file.display());
        match compile_file_for_build(file, release, &output_dir) {
            Ok(obj_file) => {
                println!(" OK");
                obj_files.push(obj_file);
                compiled_count += 1;
            }
            Err(e) => {
                println!(" FAILED");
                eprintln!("    Error: {}", e);
                failed_count += 1;
            }
        }
    }

    if failed_count > 0 {
        final_error(&format!(
            "Build failed: {}/{} files failed to compile",
            failed_count,
            box_files.len()
        ));
        return Err(anyhow::anyhow!("Compilation failed"));
    }

    success(&format!(
        "Compiled {}/{} files",
        compiled_count,
        box_files.len()
    ));
    println!();

    // Link all object files into executable
    section("Linking executable...");
    let exe_name = if cfg!(windows) { "main.exe" } else { "main" };
    let exe_path = output_dir.join(exe_name);

    match link_objects(&obj_files, &exe_path, release) {
        Ok(_) => {
            success("Linking complete");
            info("Output", &exe_path.display().to_string());
        }
        Err(e) => {
            error(&format!("Linking failed: {}", e));
            final_error("Build failed");
            return Err(e);
        }
    }

    println!();
    final_success(&format!("Successfully built {} file(s)", compiled_count));
    Ok(())
}

/// Find all .box files in the given directory recursively
fn find_box_files(dir: &str) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            // Skip target directory
            if path.file_name() != Some(std::ffi::OsStr::new("target")) {
                files.extend(find_box_files(&path.to_string_lossy())?);
            }
        } else if path.extension() == Some(std::ffi::OsStr::new("box")) {
            files.push(path);
        }
    }
    Ok(files)
}

/// Compile a single .box file for project build
fn compile_file_for_build(file: &PathBuf, release: bool, output_dir: &PathBuf) -> Result<PathBuf> {
    let source = fs::read_to_string(file)
        .with_context(|| format!("Failed to read file: {}", file.display()))?;

    // Parse the file
    let module =
        parse(&source).with_context(|| format!("Failed to parse file: {}", file.display()))?;

    // Type check the entire module
    type_check(&module).with_context(|| format!("Type checking failed for: {}", file.display()))?;

    // Generate C code
    let c_code = generate_c(&module).map_err(|e| anyhow::anyhow!("Code generation failed: {}", e))?;

    // Determine output C file path
    let file_stem = file
        .file_stem()
        .ok_or_else(|| anyhow::anyhow!("Invalid file name: {}", file.display()))?;
    let c_file = output_dir.join(format!("{}.c", file_stem.to_string_lossy()));

    // Write C code to output directory
    fs::write(&c_file, &c_code)
        .with_context(|| format!("Failed to write C code to {}", c_file.display()))?;

    // Compile C code to object file
    let obj_file = compile_c_to_obj(&c_file, release)?;

    Ok(obj_file)
}

/// Compile C code to object file
fn compile_c_to_obj(c_file: &PathBuf, release: bool) -> Result<PathBuf> {
    let compiler = CCompilerDetector::detect()?;
    let obj_file = c_file.with_extension("o");
    CCompilerRunner::compile_to_obj(&compiler, c_file, &obj_file, release)?;
    Ok(obj_file)
}

/// Link object files into executable
fn link_objects(obj_files: &[PathBuf], output: &PathBuf, release: bool) -> Result<()> {
    let linker = CCompilerDetector::detect_linker()?;
    CCompilerRunner::link_objects(&linker, obj_files, output, release)
}

fn run_project(release: bool, args: Vec<String>) -> Result<()> {
    use std::process::Command;

    header("Running BoxLang Project");

    if !args.is_empty() {
        info("Arguments", &args.join(" "));
        println!();
    }

    // First build the project if needed
    let exe_name = if cfg!(windows) { "main.exe" } else { "main" };
    let exe_path = if release {
        PathBuf::from("target/release").join(exe_name)
    } else {
        PathBuf::from("target/debug").join(exe_name)
    };

    // Check if we need to build
    let needs_build = if exe_path.exists() {
        // Check if source files are newer than executable
        let exe_modified = fs::metadata(&exe_path)?.modified()?;
        let box_files = find_box_files(".")?;
        box_files.iter().any(|f| {
            if let Ok(meta) = fs::metadata(f) {
                if let Ok(modified) = meta.modified() {
                    return modified > exe_modified;
                }
            }
            false
        })
    } else {
        true
    };

    if needs_build {
        section("Building before run...");
        build_project(release, None, None, None)?;
    }

    if !exe_path.exists() {
        anyhow::bail!("No executable found. Build may have failed.");
    }

    // Run the executable
    run_executable(&exe_path, &args)
}

/// Run an executable with the given arguments
fn run_executable(exe_path: &PathBuf, args: &[String]) -> Result<()> {
    use std::process::Command;

    section("Running executable...");
    info("Executable", &exe_path.display().to_string());
    println!();

    let mut cmd = Command::new(exe_path);
    cmd.args(args);

    // Run the program and capture output
    let output = cmd
        .output()
        .with_context(|| format!("Failed to run executable: {}", exe_path.display()))?;

    // Print stdout
    if !output.stdout.is_empty() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("{}", stdout);
    }

    // Print stderr if any
    if !output.stderr.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("{}", stderr);
    }

    // Check exit status
    if output.status.success() {
        final_success(&format!("Program exited with code 0"));
        Ok(())
    } else {
        let code = output.status.code().unwrap_or(-1);
        final_error(&format!("Program exited with code {}", code));
        Err(anyhow::anyhow!("Program exited with non-zero status"))
    }
}

fn check_file(file: Option<PathBuf>, check_all: bool) -> Result<()> {
    header("Checking BoxLang Source");

    if check_all {
        section("Checking all files in project...");
        let box_files = find_box_files(".")?;

        if box_files.is_empty() {
            warning("No .box files found in project");
            return Ok(());
        }

        let mut passed = 0;
        let mut failed = 0;

        for file in box_files {
            print!("  Checking {}...", file.display());
            let source = match fs::read_to_string(&file) {
                Ok(s) => s,
                Err(e) => {
                    println!(" ERROR (read: {})", e);
                    failed += 1;
                    continue;
                }
            };

            match parse(&source) {
                Ok(ast) => match type_check(&ast) {
                    Ok(_) => {
                        println!(" OK");
                        passed += 1;
                    }
                    Err(e) => {
                        println!(" TYPE ERROR: {}", e);
                        failed += 1;
                    }
                },
                Err(e) => {
                    println!(" PARSE ERROR: {}", e);
                    failed += 1;
                }
            }
        }

        println!();
        if failed > 0 {
            final_error(&format!("{} file(s) failed, {} passed", failed, passed));
        } else {
            final_success(&format!("All {} file(s) passed checks", passed));
        }
    } else {
        let file = file.unwrap_or_else(|| PathBuf::from("src/main.box"));

        info("File", &file.display().to_string());
        println!();

        if !file.exists() {
            return Err(anyhow::anyhow!("File not found: {}", file.display()));
        }

        let source = fs::read_to_string(&file)
            .with_context(|| format!("Failed to read file: {}", file.display()))?;

        section("Checking syntax...");
        match parse(&source) {
            Ok(ast) => {
                success("Syntax OK");
                section("Checking types...");
                match type_check(&ast) {
                    Ok(_) => {
                        success("Type check OK");
                        println!();
                        final_success("No errors found!");
                    }
                    Err(e) => {
                        match &e {
                            TypeError::Multiple(errors) => {
                                for err in errors {
                                    error(&format!("Type error: {}", err));
                                }
                            }
                            _ => {
                                error(&format!("Type error: {}", e));
                            }
                        }
                        final_error("Type checking failed");
                    }
                }
            }
            Err(e) => {
                error(&format!("Parse error: {}", e));
                final_error("Syntax check failed");
            }
        }
    }

    Ok(())
}

fn format_files(files: &[PathBuf], check: bool, stdout: bool) -> Result<()> {
    header("Formatting BoxLang Source Files");

    let files_to_format = if files.is_empty() {
        section("Scanning for .box files...");
        find_box_files(".")?
    } else {
        section(&format!("Formatting {} specified file(s)...", files.len()));
        files.to_vec()
    };

    if files_to_format.is_empty() {
        warning("No .box files found");
        return Ok(());
    }

    success(&format!(
        "Found {} file(s) to format",
        files_to_format.len()
    ));
    println!();

    if check {
        info("Mode", "check (no files will be modified)");
        println!();
    } else if stdout {
        info("Mode", "stdout (output to stdout)");
        println!();
    }

    let mut formatted_count = 0;
    let mut unchanged_count = 0;
    let mut error_count = 0;

    section("Formatting files...");

    for file in &files_to_format {
        if !stdout {
            print!("  {}...", file.display());
        }

        let source = match fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                if !stdout {
                    println!(" ERROR (read: {})", e);
                }
                error_count += 1;
                continue;
            }
        };

        // Apply formatting rules
        let formatted = format_source(&source);

        if formatted == source {
            if !stdout {
                println!(" (unchanged)");
            }
            unchanged_count += 1;
        } else if check {
            if !stdout {
                println!(" (would change)");
            }
            formatted_count += 1;
        } else if stdout {
            println!("=== {} ===", file.display());
            println!("{}", formatted);
        } else {
            match fs::write(file, &formatted) {
                Ok(_) => {
                    println!(" OK");
                    formatted_count += 1;
                }
                Err(e) => {
                    println!(" ERROR (write: {})", e);
                    error_count += 1;
                }
            }
        }
    }

    println!();

    if error_count > 0 {
        final_error(&format!("{} file(s) failed to format", error_count));
    } else if check {
        if formatted_count > 0 {
            warning(&format!("{} file(s) would be reformatted", formatted_count));
        } else {
            final_success("All files are properly formatted");
        }
    } else if stdout {
        // No summary for stdout mode
    } else {
        final_success(&format!(
            "Formatted {} file(s), {} unchanged",
            formatted_count, unchanged_count
        ));
    }

    Ok(())
}

fn clean_project() -> Result<()> {
    header("Cleaning Project");

    let target_dir = PathBuf::from("target");

    if target_dir.exists() {
        section("Removing target directory...");
        fs::remove_dir_all(&target_dir)?;
        success("Removed target/ directory");
    } else {
        info("Status", "Nothing to clean (target/ does not exist)");
    }

    // Clean up intermediate C files
    let box_files = find_box_files(".")?;
    let mut cleaned = 0;
    for file in box_files {
        let c_file = file.with_extension("c");
        if c_file.exists() {
            fs::remove_file(&c_file)?;
            cleaned += 1;
        }
    }

    if cleaned > 0 {
        success(&format!("Removed {} intermediate C file(s)", cleaned));
    }

    println!();
    final_success("Project cleaned");
    Ok(())
}

fn show_project_info() -> Result<()> {
    header("Project Information");

    // Check for box.toml
    let toml_path = PathBuf::from("box.toml");
    if toml_path.exists() {
        section("Package Configuration");
        let content = fs::read_to_string(&toml_path)?;
        println!("{}", content);
    } else {
        warning("No box.toml found in current directory");
    }

    // List source files
    section("Source Files");
    let box_files = find_box_files(".")?;
    if box_files.is_empty() {
        println!("  No .box files found");
    } else {
        for file in &box_files {
            println!("  • {}", file.display());
        }
    }

    // Check for build artifacts
    section("Build Artifacts");
    let debug_exe = PathBuf::from("target/debug/main.exe");
    let release_exe = PathBuf::from("target/release/main.exe");

    if debug_exe.exists() {
        let meta = fs::metadata(&debug_exe)?;
        let size = meta.len();
        println!(
            "  • target/debug/main.exe ({} bytes)",
            size
        );
    }

    if release_exe.exists() {
        let meta = fs::metadata(&release_exe)?;
        let size = meta.len();
        println!(
            "  • target/release/main.exe ({} bytes)",
            size
        );
    }

    if !debug_exe.exists() && !release_exe.exists() {
        println!("  No build artifacts found");
    }

    println!();
    final_success("Project information displayed");
    Ok(())
}

fn start_repl() -> Result<()> {
    header("BoxLang REPL (Experimental)");
    println!();
    println!("Type ':help' for available commands, ':quit' to exit.");
    println!();

    use std::io::{self, Write};
    use boxlang_compiler::{ConstEvaluator, ConstValue};

    let mut evaluator = ConstEvaluator::new();
    let mut session_vars: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    loop {
        print!("boxlang> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        match input {
            ":quit" | ":q" => {
                println!("Goodbye!");
                break;
            }
            ":help" | ":h" => {
                println!("Available commands:");
                println!("  :help, :h     Show this help message");
                println!("  :quit, :q     Exit the REPL");
                println!("  :clear, :c    Clear the screen");
                println!("  :vars         Show defined variables");
                println!("  :reset        Reset the session");
                println!();
                println!("Type BoxLang code to evaluate it.");
                println!("Supported: literals, arithmetic, comparisons, let bindings, if/match expressions");
            }
            ":clear" | ":c" => {
                print!("\x1B[2J\x1B[1;1H");
            }
            ":vars" => {
                if session_vars.is_empty() {
                    println!("No variables defined.");
                } else {
                    println!("Defined variables:");
                    for (name, value) in &session_vars {
                        println!("  {} = {}", name, value);
                    }
                }
            }
            ":reset" => {
                evaluator = ConstEvaluator::new();
                session_vars.clear();
                println!("Session reset.");
            }
            _ => {
                match parse(input) {
                    Ok(ast) => {
                        match type_check(&ast) {
                            Ok(_) => {
                                let mut results = Vec::new();
                                for item in &ast.items {
                                    match item {
                                        boxlang_compiler::ast::Item::Const(const_def) => {
                                            let value = evaluator.eval(&const_def.value);
                                            session_vars.insert(const_def.name.to_string(), format!("{}", value));
                                            println!("{} = {}", const_def.name, value);
                                            results.push(format!("{}", value));
                                        }
                                        boxlang_compiler::ast::Item::Function(func) => {
                                            if func.name.as_str() == "main" || func.body.stmts.len() == 1 {
                                                if let Some(stmt) = func.body.stmts.first() {
                                                    if let boxlang_compiler::ast::Stmt::Expr(expr) = stmt {
                                                        let value = evaluator.eval(expr);
                                                        if !matches!(value, ConstValue::Unit) {
                                                            println!("{}", value);
                                                        }
                                                        results.push(format!("{}", value));
                                                    }
                                                }
                                            } else {
                                                println!("Function '{}' defined (not evaluated)", func.name);
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                
                                if results.is_empty() {
                                    println!("OK");
                                }
                            }
                            Err(e) => {
                                eprintln!("Type error: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Parse error: {}", e);
                    }
                }
            }
        }
    }

    Ok(())
}

/// BoxLang source code formatter (rustfmt-style)
fn format_source(source: &str) -> String {
    let mut result = String::new();
    let mut indent_level = 0;
    let mut prev_line_empty = false;
    let mut in_string = false;
    let mut string_char = '"';

    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // Skip empty lines at the start or multiple consecutive empty lines
        if trimmed.is_empty() {
            if !prev_line_empty && !result.is_empty() {
                result.push('\n');
                prev_line_empty = true;
            }
            i += 1;
            continue;
        }
        prev_line_empty = false;

        // Calculate brace balance for this line
        let (open_braces, close_braces) = count_braces(trimmed, &mut in_string, &mut string_char);

        // Decrease indent for lines starting with closing braces
        let starts_with_close =
            trimmed.starts_with('}') || trimmed.starts_with("]") || trimmed.starts_with(")");

        let current_indent = if starts_with_close && indent_level > 0 {
            indent_level - 1
        } else {
            indent_level
        };

        // Add indentation (4 spaces per level, rustfmt style)
        let indent = "    ".repeat(current_indent);
        result.push_str(&indent);

        // Format the line content
        let formatted_line = format_line_content(trimmed);
        result.push_str(&formatted_line);
        result.push('\n');

        // Update indent level for next lines
        // Increase for opening braces at end of line
        indent_level += open_braces;

        // Decrease for closing braces at start of line (already handled above)
        if starts_with_close {
            indent_level = indent_level.saturating_sub(1);
        }

        // Decrease for closing braces at end of line
        let ends_with_close =
            trimmed.ends_with('}') || trimmed.ends_with("]") || trimmed.ends_with(")");
        if ends_with_close && indent_level > 0 {
            indent_level -= 1;
        }

        i += 1;
    }

    // Ensure single trailing newline
    while result.ends_with('\n') {
        result.pop();
    }
    result.push('\n');

    result
}

/// Count opening and closing braces in a line, respecting strings
fn count_braces(line: &str, in_string: &mut bool, string_char: &mut char) -> (usize, usize) {
    let mut open = 0;
    let mut close = 0;

    for (i, c) in line.chars().enumerate() {
        if *in_string {
            if c == *string_char {
                // Check for escape
                let backslash_count = line[..i].chars().rev().take_while(|&c| c == '\\').count();
                if backslash_count % 2 == 0 {
                    *in_string = false;
                }
            }
        } else {
            match c {
                '"' | '\'' => {
                    *in_string = true;
                    *string_char = c;
                }
                '{' | '[' | '(' => open += 1,
                '}' | ']' | ')' => close += 1,
                _ => {}
            }
        }
    }

    (open, close)
}

/// Format the content of a single line
fn format_line_content(line: &str) -> String {
    let mut result = String::new();
    let mut prev_char = ' ';
    let mut in_string = false;
    let mut string_char = '"';

    for (i, c) in line.chars().enumerate() {
        if in_string {
            result.push(c);
            if c == string_char {
                let backslash_count = line[..i].chars().rev().take_while(|&c| c == '\\').count();
                if backslash_count % 2 == 0 {
                    in_string = false;
                }
            }
        } else {
            match c {
                '"' | '\'' => {
                    in_string = true;
                    string_char = c;
                    result.push(c);
                }
                // Add space after commas (but not in strings)
                ',' => {
                    result.push(c);
                    // Add space after comma if not at end of line
                    if i + 1 < line.len() && line.chars().nth(i + 1) != Some(' ') {
                        result.push(' ');
                    }
                }
                // Add space around operators (but not in strings)
                '+' | '-' | '*' | '/' | '%' | '=' | '<' | '>' | '!' | '&' | '|' => {
                    // Check if it's a compound operator
                    let next_char = line.chars().nth(i + 1).unwrap_or(' ');
                    let is_compound = matches!(
                        (c, next_char),
                        ('+', '=')
                            | ('-', '=')
                            | ('*', '=')
                            | ('/', '=')
                            | ('%', '=')
                            | ('=', '=')
                            | ('!', '=')
                            | ('<', '=')
                            | ('>', '=')
                            | ('&', '&')
                            | ('|', '|')
                            | ('<', '<')
                            | ('>', '>')
                    );

                    if is_compound {
                        // Add space before compound operator
                        if prev_char != ' ' && !is_operator_char(prev_char) {
                            result.push(' ');
                        }
                        result.push(c);
                        result.push(next_char);
                        // Skip next char
                        continue;
                    } else if is_operator_char(prev_char) {
                        // Part of compound operator, no extra space
                        result.push(c);
                    } else {
                        // Single operator
                        if prev_char != ' ' {
                            result.push(' ');
                        }
                        result.push(c);
                        // Add space after if not end of line
                        if i + 1 < line.len() {
                            if let Some(next) = line.chars().nth(i + 1) {
                                if next != ' ' && !is_operator_char(next) {
                                    result.push(' ');
                                }
                            }
                        }
                    }
                }
                ':' => {
                    // Type annotation or struct field
                    result.push(c);
                    if i + 1 < line.len() && line.chars().nth(i + 1) != Some(' ') {
                        // Check if it's a path separator (::)
                        if line.chars().nth(i + 1) != Some(':') {
                            result.push(' ');
                        }
                    }
                }
                ';' => {
                    result.push(c);
                    // Add space after semicolon if not at end
                    if i + 1 < line.len() && line.chars().nth(i + 1) != Some(' ') {
                        result.push(' ');
                    }
                }
                _ => {
                    result.push(c);
                }
            }
        }
        prev_char = c;
    }

    // Trim trailing whitespace
    result.trim_end().to_string()
}

/// Check if a character is an operator character
fn is_operator_char(c: char) -> bool {
    matches!(
        c,
        '+' | '-' | '*' | '/' | '%' | '=' | '<' | '>' | '!' | '&' | '|'
    )
}

/// Show BoxLang logo with diagonal gradient
fn show_box_logo() {
    // Gradient colors: #f9f871 #b5fa8a #6ef5b0 #00ecd6 #00def2 #00cdff
    // Converted to ANSI 256 approximations
    let gradient_colors: [(u8, u8, u8); 6] = [
        (249, 248, 113), // #f9f871 - yellow
        (181, 250, 138), // #b5fa8a - light green
        (110, 245, 176), // #6ef5b0 - mint
        (0, 236, 214),   // #00ecd6 - cyan
        (0, 222, 242),   // #00def2 - sky blue
        (0, 205, 255),   // #00cdff - bright blue
    ];

    // Full logo lines
    let lines = [
        "██████╗  ██████╗ ██╗  ██╗    ██╗      █████╗ ███╗   ██╗ ██████╗ ",
        "██╔══██╗██╔═══██╗╚██╗██╔╝    ██║     ██╔══██╗████╗  ██║██╔════╝ ",
        "██████╔╝██║   ██║ ╚███╔╝     ██║     ███████║██╔██╗ ██║██║  ███╗",
        "██╔══██╗██║   ██║ ██╔██╗     ██║     ██╔══██║██║╚██╗██║██║   ██║",
        "██████╔╝╚██████╔╝██╔╝ ██╗    ███████╗██║  ██║██║ ╚████║╚██████╔╝",
        "╚═════╝  ╚═════╝ ╚═╝  ╚═╝    ╚══════╝╚═╝  ╚═╝╚═╝  ╚═══╝ ╚═════╝ ",
    ];

    let max_row = lines.len();
    let max_col = lines[0].chars().count();

    println!();
    for (row, line) in lines.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            if ch == ' ' {
                print!(" ");
                continue;
            }
            // Calculate diagonal gradient position (0.0 to 1.0)
            let t = (row + col) as f32 / (max_row + max_col) as f32;
            let color = interpolate_color(&gradient_colors, t);
            let ansi_color = rgb_to_ansi256(color.0, color.1, color.2);
            print!("\x1b[38;5;{}m{}\x1b[0m", ansi_color, ch);
        }
        println!();
    }

    // Slogan with colored "Box" and "Lang"
    println!();
    let box_color = rgb_to_ansi256(249, 248, 113); // #f9f871 - yellow
    let lang_color = rgb_to_ansi256(0, 205, 255); // #00cdff - bright blue
    print!(
        "              Think Outside the \x1b[38;5;{}mBox\x1b[0m, Code Inside the \x1b[38;5;{}mLang\x1b[0m",
        box_color, lang_color
    );
    println!();

    // Version info
    println!();
    println!("                    Version 0.1.0");
    println!();
}

/// Interpolate between gradient colors based on position t (0.0 to 1.0)
fn interpolate_color(colors: &[(u8, u8, u8)], t: f32) -> (u8, u8, u8) {
    let n = colors.len() - 1;
    let scaled_t = t * n as f32;
    let idx = scaled_t.floor() as usize;
    let frac = scaled_t - idx as f32;

    let idx = idx.min(n - 1);
    let c1 = colors[idx];
    let c2 = colors[idx + 1];

    let r = (c1.0 as f32 * (1.0 - frac) + c2.0 as f32 * frac) as u8;
    let g = (c1.1 as f32 * (1.0 - frac) + c2.1 as f32 * frac) as u8;
    let b = (c1.2 as f32 * (1.0 - frac) + c2.2 as f32 * frac) as u8;

    (r, g, b)
}

/// Convert RGB to ANSI 256 color code
fn rgb_to_ansi256(r: u8, g: u8, b: u8) -> u8 {
    // Grayscale check
    if r == g && g == b {
        if r < 8 {
            return 16;
        }
        if r > 248 {
            return 231;
        }
        return 232 + ((r - 8) / 10) as u8;
    }

    // Color cube (6x6x6)
    let r_idx = (r as f32 / 51.0).floor() as u8;
    let g_idx = (g as f32 / 51.0).floor() as u8;
    let b_idx = (b as f32 / 51.0).floor() as u8;

    16 + 36 * r_idx + 6 * g_idx + b_idx
}

/// Package the application as AppBox format
fn package_appbox(
    output: Option<PathBuf>,
    name: Option<String>,
    version: String,
    author: Option<String>,
    opt_level: u8,
) -> Result<()> {
    use boxlang_compiler::integration::appbox::{AppBoxBuilder, generate_default_main};

    header("Packaging AppBox Application");

    // Determine project name
    let project_name = name.unwrap_or_else(|| {
        // Try to read from box.toml
        if let Ok(content) = fs::read_to_string("box.toml") {
            if let Ok(config) = content.parse::<toml::Value>() {
                if let Some(name) = config.get("package").and_then(|p| p.get("name")).and_then(|n| n.as_str()) {
                    return name.to_string();
                }
            }
        }
        "myapp".to_string()
    });

    // Determine output directory
    let output_dir = output.unwrap_or_else(|| PathBuf::from("dist"));

    // Determine author
    let app_author = author.unwrap_or_else(|| {
        std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "Anonymous".to_string())
    });

    section("Build Configuration");
    info("Project", &project_name);
    info("Version", &version);
    info("Author", &app_author);
    info("Opt Level", &format!("-O{}", opt_level));
    info("Output", &output_dir.display().to_string());
    println!();

    // Find source files
    section("Collecting source files...");
    let mut source_files = Vec::new();

    // Check for src/main.box or src/lib.box
    if PathBuf::from("src/main.box").exists() {
        source_files.push(PathBuf::from("src/main.box"));
        success("Found src/main.box");
    } else if PathBuf::from("src/lib.box").exists() {
        source_files.push(PathBuf::from("src/lib.box"));
        success("Found src/lib.box");
    }

    // Find all .box files in src directory
    if let Ok(entries) = fs::read_dir("src") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension() == Some(std::ffi::OsStr::new("box")) {
                if !source_files.contains(&path) {
                    source_files.push(path);
                }
            }
        }
    }

    if source_files.is_empty() {
        warning("No .box files found, creating default main.box");
        fs::create_dir_all("src")?;
        let default_main = generate_default_main(&project_name);
        fs::write("src/main.box", default_main)?;
        source_files.push(PathBuf::from("src/main.box"));
    }

    success(&format!("Found {} source file(s)", source_files.len()));
    println!();

    // Build the package
    section("Building AppBox package...");

    let mut builder = AppBoxBuilder::new(&project_name, &output_dir)
        .version(&version)
        .author(&app_author)
        .opt_level(opt_level);

    for source in &source_files {
        builder = builder.add_source(source);
    }

    match builder.build() {
        Ok(package_path) => {
            success("Package created successfully");
            info("Package", &package_path.display().to_string());
            println!();
            final_success(&format!("Successfully packaged {}", project_name));
        }
        Err(e) => {
            error(&format!("Failed to create package: {}", e));
            final_error("Packaging failed");
            return Err(e);
        }
    }

    Ok(())
}
