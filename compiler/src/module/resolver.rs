//! Module resolver for BoxLang
//!
//! Resolves module paths to file system paths.

use std::path::{Path, PathBuf};
use std::env;
use super::{ModuleId, ModuleError};

pub struct ModuleResolver {
    std_path: Option<PathBuf>,
    search_paths: Vec<PathBuf>,
}

impl ModuleResolver {
    pub fn new() -> Self {
        let mut search_paths = vec![PathBuf::from(".")];
        
        if let Ok(cwd) = env::current_dir() {
            search_paths.push(cwd);
        }
        
        Self {
            std_path: None,
            search_paths,
        }
    }

    pub fn set_std_path(&mut self, path: PathBuf) {
        self.std_path = Some(path);
    }

    pub fn add_search_path(&mut self, path: PathBuf) {
        if !self.search_paths.contains(&path) {
            self.search_paths.push(path);
        }
    }

    pub fn resolve(&self, module_id: &ModuleId, from: Option<&PathBuf>) -> Result<PathBuf, ModuleError> {
        if module_id.is_std() || module_id.is_core() || module_id.is_boxos() {
            self.resolve_std_module(module_id)
        } else {
            self.resolve_user_module(module_id, from)
        }
    }

    fn resolve_std_module(&self, module_id: &ModuleId) -> Result<PathBuf, ModuleError> {
        let std_path = self.std_path.clone()
            .or_else(|| self.find_std_path())
            .ok_or_else(|| ModuleError::NotFound {
                module: module_id.to_string_path(),
            })?;

        let root = module_id.path.first().unwrap();
        let relative_path: PathBuf = module_id.path[1..].iter().collect();
        
        let mut base_path = std_path.join(root);
        if !relative_path.as_os_str().is_empty() {
            base_path = base_path.join(&relative_path);
        }

        let candidates = self.get_candidates(&base_path, module_id.name());
        
        for candidate in candidates {
            if candidate.exists() {
                return Ok(candidate);
            }
        }

        Err(ModuleError::NotFound {
            module: module_id.to_string_path(),
        })
    }

    fn resolve_user_module(&self, module_id: &ModuleId, from: Option<&PathBuf>) -> Result<PathBuf, ModuleError> {
        let base_dir = from
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .or_else(|| env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));

        let relative_path: PathBuf = module_id.path.iter().collect();
        let module_dir = base_dir.join(&relative_path);
        
        let candidates = self.get_candidates(&module_dir, module_id.name());
        
        for candidate in &candidates {
            if candidate.exists() {
                return Ok(candidate.clone());
            }
        }

        for search_path in &self.search_paths {
            let module_dir = search_path.join(&relative_path);
            let candidates = self.get_candidates(&module_dir, module_id.name());
            
            for candidate in &candidates {
                if candidate.exists() {
                    return Ok(candidate.clone());
                }
            }
        }

        Err(ModuleError::NotFound {
            module: module_id.to_string_path(),
        })
    }

    fn get_candidates(&self, dir: &Path, name: &str) -> Vec<PathBuf> {
        vec![
            dir.join(format!("{}.box", name)),
            dir.join("mod.box"),
            dir.join(format!("{}/mod.box", name)),
        ]
    }

    fn find_std_path(&self) -> Option<PathBuf> {
        if let Ok(path) = env::var("BOXLANG_STD_PATH") {
            return Some(PathBuf::from(path));
        }

        if let Ok(exe_path) = env::current_exe() {
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

        if let Ok(cwd) = env::current_dir() {
            let candidates = vec![
                cwd.join("std"),
                cwd.join("../std"),
                cwd.join("../../std"),
            ];
            
            for candidate in candidates {
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }

        None
    }

    pub fn std_path(&self) -> Option<&Path> {
        self.std_path.as_deref()
    }
}

impl Default for ModuleResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_id_parsing() {
        let id = ModuleId::from_str("std::io");
        assert_eq!(id.path, vec!["std", "io"]);
        assert!(id.is_std());
        assert_eq!(id.name(), "io");
    }

    #[test]
    fn test_module_id_parent() {
        let id = ModuleId::from_str("std::io::file");
        let parent = id.parent().unwrap();
        assert_eq!(parent.to_string_path(), "std::io");
    }

    #[test]
    fn test_module_id_root() {
        let id = ModuleId::root("main");
        assert_eq!(id.path, vec!["main"]);
        assert!(!id.is_std());
    }
}
