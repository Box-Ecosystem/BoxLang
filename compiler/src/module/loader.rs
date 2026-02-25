//! Module loader for BoxLang
//!
//! Loads and parses BoxLang source files.

use std::fs;
use std::path::PathBuf;
use crate::ast::Module;
use crate::frontend::parser::parse;
use super::ModuleError;

pub struct ModuleLoader {
    cache: Vec<(PathBuf, String)>,
}

impl ModuleLoader {
    pub fn new() -> Self {
        Self {
            cache: Vec::new(),
        }
    }

    pub fn read_source(&self, path: &PathBuf) -> Result<String, ModuleError> {
        fs::read_to_string(path)
            .map_err(|e| ModuleError::IoError {
                path: path.display().to_string(),
                error: e.to_string(),
            })
    }

    pub fn parse(&self, source: &str) -> Result<Module, ModuleError> {
        parse(source).map_err(|e| ModuleError::ParseError {
            module: "unknown".to_string(),
            error: e.to_string(),
        })
    }

    pub fn load(&mut self, path: &PathBuf) -> Result<Module, ModuleError> {
        let source = self.read_source(path)?;
        self.parse(&source)
    }

    pub fn load_with_source(&mut self, path: &PathBuf) -> Result<(Module, String), ModuleError> {
        let source = self.read_source(path)?;
        let ast = self.parse(&source)?;
        Ok((ast, source))
    }
}

impl Default for ModuleLoader {
    fn default() -> Self {
        Self::new()
    }
}

pub struct StdLoader {
    loader: ModuleLoader,
    loaded_modules: Vec<String>,
}

impl StdLoader {
    pub fn new() -> Self {
        Self {
            loader: ModuleLoader::new(),
            loaded_modules: Vec::new(),
        }
    }

    pub fn load_core(&mut self, std_path: &PathBuf) -> Result<Vec<(String, Module)>, ModuleError> {
        let core_path = std_path.join("core");
        self.load_std_module(&core_path, "core")
    }

    pub fn load_std(&mut self, std_path: &PathBuf) -> Result<Vec<(String, Module)>, ModuleError> {
        let std_dir = std_path.join("std");
        self.load_std_module(&std_dir, "std")
    }

    pub fn load_boxos(&mut self, std_path: &PathBuf) -> Result<Vec<(String, Module)>, ModuleError> {
        let boxos_path = std_path.join("boxos");
        self.load_std_module(&boxos_path, "boxos")
    }

    fn load_std_module(&mut self, base_path: &PathBuf, root_name: &str) -> Result<Vec<(String, Module)>, ModuleError> {
        let mut modules = Vec::new();
        
        if !base_path.exists() {
            return Ok(modules);
        }

        self.load_module_recursive(base_path, root_name, &mut modules)?;

        Ok(modules)
    }

    fn load_module_recursive(
        &mut self,
        dir: &PathBuf,
        module_prefix: &str,
        modules: &mut Vec<(String, Module)>,
    ) -> Result<(), ModuleError> {
        let mod_file = dir.join("mod.box");
        
        if mod_file.exists() {
            let module_name = module_prefix.to_string();
            
            if !self.loaded_modules.contains(&module_name) {
                let ast = self.loader.load(&mod_file)?;
                modules.push((module_name.clone(), ast));
                self.loaded_modules.push(module_name);
            }
        }

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                
                if path.is_dir() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        let sub_module_prefix = format!("{}::{}", module_prefix, name);
                        self.load_module_recursive(&path, &sub_module_prefix, modules)?;
                    }
                } else if path.extension().map(|e| e == "box").unwrap_or(false) {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if stem != "mod" {
                            let module_name = format!("{}::{}", module_prefix, stem);
                            
                            if !self.loaded_modules.contains(&module_name) {
                                let ast = self.loader.load(&path)?;
                                modules.push((module_name.clone(), ast));
                                self.loaded_modules.push(module_name);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn loaded_modules(&self) -> &[String] {
        &self.loaded_modules
    }

    pub fn is_loaded(&self, module_name: &str) -> bool {
        self.loaded_modules.contains(&module_name.to_string())
    }
}

impl Default for StdLoader {
    fn default() -> Self {
        Self::new()
    }
}
