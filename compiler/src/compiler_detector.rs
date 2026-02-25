use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilerKind {
    Clang,
    Gcc,
    Msvc,
}

#[derive(Debug, Clone)]
pub struct CCompiler {
    pub path: PathBuf,
    pub kind: CompilerKind,
}

impl CCompiler {
    pub fn is_msvc(&self) -> bool {
        self.kind == CompilerKind::Msvc
    }

    pub fn is_clang(&self) -> bool {
        self.kind == CompilerKind::Clang
    }

    pub fn is_gcc(&self) -> bool {
        self.kind == CompilerKind::Gcc
    }
}

pub struct CCompilerDetector;

impl CCompilerDetector {
    pub fn detect() -> Result<CCompiler> {
        Self::detect_compiler()
    }

    pub fn detect_linker() -> Result<CCompiler> {
        Self::detect_linker_internal()
    }

    fn detect_compiler() -> Result<CCompiler> {
        let mut compiler_cmd: Option<(PathBuf, CompilerKind)> = None;

        #[cfg(windows)]
        {
            let clang_paths = [
                PathBuf::from(r"C:\Program Files\LLVM\bin\clang.exe"),
                PathBuf::from(r"C:\Program Files (x86)\LLVM\bin\clang.exe"),
            ];
            for path in &clang_paths {
                if path.exists() {
                    compiler_cmd = Some((path.clone(), CompilerKind::Clang));
                    break;
                }
            }
        }

        if compiler_cmd.is_none() {
            let candidates = [
                ("clang", CompilerKind::Clang),
                ("gcc", CompilerKind::Gcc),
                ("cl", CompilerKind::Msvc),
            ];
            for (name, kind) in &candidates {
                if Command::new(name).arg("--version").output().is_ok() {
                    compiler_cmd = Some((PathBuf::from(name), *kind));
                    break;
                }
            }
        }

        let (path, kind) = compiler_cmd.ok_or_else(|| {
            anyhow::anyhow!("No C compiler found. Please install clang, gcc, or MSVC.")
        })?;

        Ok(CCompiler { path, kind })
    }

    fn detect_linker_internal() -> Result<CCompiler> {
        let mut linker_cmd: Option<(PathBuf, CompilerKind)> = None;

        #[cfg(windows)]
        {
            let clang_paths = [
                PathBuf::from(r"C:\Program Files\LLVM\bin\clang.exe"),
                PathBuf::from(r"C:\Program Files (x86)\LLVM\bin\clang.exe"),
            ];
            for path in &clang_paths {
                if path.exists() {
                    linker_cmd = Some((path.clone(), CompilerKind::Clang));
                    break;
                }
            }
        }

        if linker_cmd.is_none() {
            let candidates = [
                ("clang", CompilerKind::Clang),
                ("gcc", CompilerKind::Gcc),
                ("ld", CompilerKind::Gcc),
                ("link", CompilerKind::Msvc),
            ];
            for (name, kind) in &candidates {
                if Command::new(name).arg("--version").output().is_ok() {
                    linker_cmd = Some((PathBuf::from(name), *kind));
                    break;
                }
            }
        }

        let (path, kind) = linker_cmd.ok_or_else(|| {
            anyhow::anyhow!("No linker found. Please install clang, gcc, or MSVC.")
        })?;

        Ok(CCompiler { path, kind })
    }
}

pub struct CompileOptions {
    pub opt_level: u8,
    pub debug: bool,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            opt_level: 0,
            debug: true,
        }
    }
}

pub struct CCompilerRunner;

impl CCompilerRunner {
    pub fn compile_to_exe(
        compiler: &CCompiler,
        c_file: &PathBuf,
        output: &PathBuf,
        opts: &CompileOptions,
    ) -> Result<()> {
        let mut cmd = Command::new(&compiler.path);

        let opt_flag = match opts.opt_level {
            0 => "-O0",
            1 => "-O1",
            2 => "-O2",
            3 => "-O3",
            _ => "-O0",
        };

        if compiler.is_msvc() {
            let msvc_opt = match opts.opt_level {
                0 => "/Od",
                1 | 2 => "/O2",
                3 => "/Ox",
                _ => "/Od",
            };
            cmd.arg(c_file)
                .arg(format!("/Fe:{}", output.display()))
                .arg("/W4")
                .arg(msvc_opt);
        } else {
            cmd.arg(c_file).arg("-o").arg(output).arg("-Wall").arg(opt_flag);

            if opts.debug && opts.opt_level == 0 {
                cmd.arg("-g");
            }
        }

        let output_result = cmd
            .output()
            .with_context(|| format!("Failed to run C compiler: {}", compiler.path.display()))?;

        if !output_result.status.success() {
            let stderr = String::from_utf8_lossy(&output_result.stderr);
            return Err(anyhow::anyhow!("C compilation failed:\n{}", stderr));
        }

        Ok(())
    }

    pub fn compile_to_obj(
        compiler: &CCompiler,
        c_file: &PathBuf,
        obj_file: &PathBuf,
        release: bool,
    ) -> Result<()> {
        let mut cmd = Command::new(&compiler.path);

        if compiler.is_msvc() {
            cmd.arg("/c").arg(c_file).arg(format!("/Fo:{}", obj_file.display()));
            if release {
                cmd.arg("/O2");
            } else {
                cmd.arg("/Od");
            }
        } else {
            cmd.arg("-c").arg(c_file).arg("-o").arg(obj_file);
            if release {
                cmd.arg("-O2");
            } else {
                cmd.arg("-O0").arg("-g");
            }
            cmd.arg("-Wall");
        }

        let output = cmd
            .output()
            .with_context(|| format!("Failed to run C compiler: {}", compiler.path.display()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("C compilation failed:\n{}", stderr));
        }

        Ok(())
    }

    pub fn link_objects(
        linker: &CCompiler,
        obj_files: &[PathBuf],
        output: &PathBuf,
        release: bool,
    ) -> Result<()> {
        let mut cmd = Command::new(&linker.path);

        for obj in obj_files {
            cmd.arg(obj);
        }

        cmd.arg("-o").arg(output);

        if release {
            cmd.arg("-O2");
        }

        let output_result = cmd
            .output()
            .with_context(|| format!("Failed to run linker: {}", linker.path.display()))?;

        if !output_result.status.success() {
            let stderr = String::from_utf8_lossy(&output_result.stderr);
            return Err(anyhow::anyhow!("Linking failed:\n{}", stderr));
        }

        Ok(())
    }
}
