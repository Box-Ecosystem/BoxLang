//! Module system for BoxLang
//!
//! This module handles module resolution, loading, and dependency management.

pub mod resolver;
pub mod loader;
pub mod graph;

use std::path::PathBuf;
use std::collections::HashMap;
use crate::ast::Module;

pub use resolver::ModuleResolver;
pub use loader::ModuleLoader;
pub use graph::ModuleGraph;

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleId {
    pub path: Vec<String>,
}

impl ModuleId {
    pub fn new(path: Vec<String>) -> Self {
        Self { path }
    }

    pub fn from_str(s: &str) -> Self {
        Self {
            path: s.split("::").map(|s| s.to_string()).collect(),
        }
    }

    pub fn root(name: &str) -> Self {
        Self {
            path: vec![name.to_string()],
        }
    }

    pub fn is_std(&self) -> bool {
        self.path.first().map(|s| s == "std").unwrap_or(false)
    }

    pub fn is_core(&self) -> bool {
        self.path.first().map(|s| s == "core").unwrap_or(false)
    }

    pub fn is_boxos(&self) -> bool {
        self.path.first().map(|s| s == "boxos").unwrap_or(false)
    }

    pub fn parent(&self) -> Option<Self> {
        if self.path.len() > 1 {
            Some(Self {
                path: self.path[..self.path.len() - 1].to_vec(),
            })
        } else {
            None
        }
    }

    pub fn name(&self) -> &str {
        self.path.last().map(|s| s.as_str()).unwrap_or("")
    }

    pub fn to_string_path(&self) -> String {
        self.path.join("::")
    }
}

impl std::fmt::Display for ModuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string_path())
    }
}

#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub id: ModuleId,
    pub file_path: PathBuf,
    pub ast: Option<Module>,
    pub is_loaded: bool,
    pub is_std: bool,
}

impl ModuleInfo {
    pub fn new(id: ModuleId, file_path: PathBuf) -> Self {
        let is_std = id.is_std() || id.is_core() || id.is_boxos();
        Self {
            id,
            file_path,
            ast: None,
            is_loaded: false,
            is_std,
        }
    }
}

pub struct ModuleSystem {
    resolver: ModuleResolver,
    loader: ModuleLoader,
    graph: ModuleGraph,
    modules: HashMap<String, ModuleInfo>,
    std_path: Option<PathBuf>,
}

impl ModuleSystem {
    pub fn new() -> Self {
        Self {
            resolver: ModuleResolver::new(),
            loader: ModuleLoader::new(),
            graph: ModuleGraph::new(),
            modules: HashMap::new(),
            std_path: None,
        }
    }

    pub fn with_std_path(mut self, path: PathBuf) -> Self {
        self.std_path = Some(path.clone());
        self.resolver.set_std_path(path);
        self
    }

    pub fn resolve(&self, module_id: &ModuleId, from: Option<&PathBuf>) -> Result<PathBuf, ModuleError> {
        self.resolver.resolve(module_id, from)
    }

    pub fn load(&mut self, module_id: &ModuleId) -> Result<&ModuleInfo, ModuleError> {
        let key = module_id.to_string_path();
        
        let needs_load = match self.modules.get(&key) {
            Some(info) => !info.is_loaded,
            None => true,
        };
        
        if !needs_load {
            return self.modules.get(&key)
                .ok_or_else(|| ModuleError::NotFound { module: key });
        }

        let file_path = self.resolver.resolve(module_id, None)?;
        let mut info = ModuleInfo::new(module_id.clone(), file_path.clone());
        
        let source = self.loader.read_source(&file_path)?;
        let ast = self.loader.parse(&source)?;
        
        info.ast = Some(ast);
        info.is_loaded = true;
        
        self.modules.insert(key.clone(), info);
        self.graph.add_module(module_id.clone());

        self.modules.get(&key)
            .ok_or_else(|| ModuleError::NotFound { module: key })
    }

    pub fn get_module(&self, module_id: &ModuleId) -> Option<&ModuleInfo> {
        self.modules.get(&module_id.to_string_path())
    }

    pub fn get_module_mut(&mut self, module_id: &ModuleId) -> Option<&mut ModuleInfo> {
        self.modules.get_mut(&module_id.to_string_path())
    }

    pub fn all_modules(&self) -> impl Iterator<Item = &ModuleInfo> {
        self.modules.values()
    }

    pub fn is_loaded(&self, module_id: &ModuleId) -> bool {
        self.modules.get(&module_id.to_string_path())
            .map(|m| m.is_loaded)
            .unwrap_or(false)
    }

    pub fn add_dependency(&mut self, from: &ModuleId, to: &ModuleId) {
        self.graph.add_dependency(from.clone(), to.clone());
    }

    pub fn get_load_order(&self) -> Vec<ModuleId> {
        self.graph.topological_sort()
    }
}

impl Default for ModuleSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum ModuleError {
    NotFound { module: String },
    ParseError { module: String, error: String },
    IoError { path: String, error: String },
    CyclicDependency { modules: Vec<String> },
    InvalidPath { path: String },
}

impl std::fmt::Display for ModuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModuleError::NotFound { module } => {
                write!(f, "module '{}' not found", module)
            }
            ModuleError::ParseError { module, error } => {
                write!(f, "failed to parse module '{}': {}", module, error)
            }
            ModuleError::IoError { path, error } => {
                write!(f, "I/O error for '{}': {}", path, error)
            }
            ModuleError::CyclicDependency { modules } => {
                write!(f, "cyclic dependency detected: {}", modules.join(" -> "))
            }
            ModuleError::InvalidPath { path } => {
                write!(f, "invalid module path: '{}'", path)
            }
        }
    }
}

impl std::error::Error for ModuleError {}
