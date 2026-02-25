//! Module dependency graph for BoxLang
//!
//! Tracks dependencies between modules and provides topological ordering.

use std::collections::{HashMap, HashSet, VecDeque};
use super::ModuleId;

pub struct ModuleGraph {
    modules: HashSet<String>,
    dependencies: HashMap<String, HashSet<String>>,
    dependents: HashMap<String, HashSet<String>>,
}

impl ModuleGraph {
    pub fn new() -> Self {
        Self {
            modules: HashSet::new(),
            dependencies: HashMap::new(),
            dependents: HashMap::new(),
        }
    }

    pub fn add_module(&mut self, module_id: ModuleId) {
        let key = module_id.to_string_path();
        self.modules.insert(key.clone());
        self.dependencies.entry(key.clone()).or_insert_with(HashSet::new);
        self.dependents.entry(key).or_insert_with(HashSet::new);
    }

    pub fn add_dependency(&mut self, from: ModuleId, to: ModuleId) {
        let from_key = from.to_string_path();
        let to_key = to.to_string_path();
        
        self.modules.insert(from_key.clone());
        self.modules.insert(to_key.clone());
        
        self.dependencies
            .entry(from_key.clone())
            .or_insert_with(HashSet::new)
            .insert(to_key.clone());
        
        self.dependents
            .entry(to_key)
            .or_insert_with(HashSet::new)
            .insert(from_key);
    }

    pub fn has_module(&self, module_id: &ModuleId) -> bool {
        self.modules.contains(&module_id.to_string_path())
    }

    pub fn get_dependencies(&self, module_id: &ModuleId) -> Option<&HashSet<String>> {
        self.dependencies.get(&module_id.to_string_path())
    }

    pub fn get_dependents(&self, module_id: &ModuleId) -> Option<&HashSet<String>> {
        self.dependents.get(&module_id.to_string_path())
    }

    pub fn has_cycle(&self) -> bool {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for module in &self.modules {
            if !visited.contains(module) {
                if self.dfs_cycle(module, &mut visited, &mut rec_stack) {
                    return true;
                }
            }
        }
        false
    }

    fn dfs_cycle(
        &self,
        module: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> bool {
        visited.insert(module.to_string());
        rec_stack.insert(module.to_string());

        if let Some(deps) = self.dependencies.get(module) {
            for dep in deps {
                if !visited.contains(dep) {
                    if self.dfs_cycle(dep, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(dep) {
                    return true;
                }
            }
        }

        rec_stack.remove(module);
        false
    }

    pub fn find_cycle(&self) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for module in &self.modules {
            if !visited.contains(module) {
                if let Some(cycle) = self.dfs_find_cycle(module, &mut visited, &mut rec_stack, &mut path) {
                    return Some(cycle);
                }
            }
        }
        None
    }

    fn dfs_find_cycle(
        &self,
        module: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(module.to_string());
        rec_stack.insert(module.to_string());
        path.push(module.to_string());

        if let Some(deps) = self.dependencies.get(module) {
            for dep in deps {
                if !visited.contains(dep) {
                    if let Some(cycle) = self.dfs_find_cycle(dep, visited, rec_stack, path) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(dep) {
                    let cycle_start = path.iter().position(|m| m == dep).unwrap_or(0);
                    let mut cycle = path[cycle_start..].to_vec();
                    cycle.push(dep.to_string());
                    return Some(cycle);
                }
            }
        }

        path.pop();
        rec_stack.remove(module);
        None
    }

    pub fn topological_sort(&self) -> Vec<ModuleId> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut result = Vec::new();
        let mut queue = VecDeque::new();

        for module in &self.modules {
            let degree = self.dependencies.get(module)
                .map(|deps| deps.len())
                .unwrap_or(0);
            in_degree.insert(module.clone(), degree);
            
            if degree == 0 {
                queue.push_back(module.clone());
            }
        }

        while let Some(module) = queue.pop_front() {
            result.push(ModuleId::from_str(&module));

            if let Some(dependents) = self.dependents.get(&module) {
                for dependent in dependents {
                    if let Some(degree) = in_degree.get_mut(dependent) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(dependent.clone());
                        }
                    }
                }
            }
        }

        result
    }

    pub fn modules(&self) -> impl Iterator<Item = &String> {
        self.modules.iter()
    }

    pub fn module_count(&self) -> usize {
        self.modules.len()
    }
}

impl Default for ModuleGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_module() {
        let mut graph = ModuleGraph::new();
        graph.add_module(ModuleId::from_str("main"));
        assert!(graph.has_module(&ModuleId::from_str("main")));
    }

    #[test]
    fn test_add_dependency() {
        let mut graph = ModuleGraph::new();
        graph.add_dependency(
            ModuleId::from_str("main"),
            ModuleId::from_str("std::io"),
        );
        
        assert!(graph.has_module(&ModuleId::from_str("main")));
        assert!(graph.has_module(&ModuleId::from_str("std::io")));
        
        let deps = graph.get_dependencies(&ModuleId::from_str("main")).unwrap();
        assert!(deps.contains("std::io"));
    }

    #[test]
    fn test_no_cycle() {
        let mut graph = ModuleGraph::new();
        graph.add_dependency(ModuleId::from_str("a"), ModuleId::from_str("b"));
        graph.add_dependency(ModuleId::from_str("b"), ModuleId::from_str("c"));
        
        assert!(!graph.has_cycle());
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = ModuleGraph::new();
        graph.add_dependency(ModuleId::from_str("a"), ModuleId::from_str("b"));
        graph.add_dependency(ModuleId::from_str("b"), ModuleId::from_str("c"));
        graph.add_dependency(ModuleId::from_str("c"), ModuleId::from_str("a"));
        
        assert!(graph.has_cycle());
    }

    #[test]
    fn test_topological_sort() {
        let mut graph = ModuleGraph::new();
        graph.add_dependency(ModuleId::from_str("main"), ModuleId::from_str("std::io"));
        graph.add_dependency(ModuleId::from_str("main"), ModuleId::from_str("std::math"));
        
        let order = graph.topological_sort();
        assert_eq!(order.len(), 3);
        
        let main_pos = order.iter().position(|m| m.to_string_path() == "main").unwrap();
        let io_pos = order.iter().position(|m| m.to_string_path() == "std::io").unwrap();
        let math_pos = order.iter().position(|m| m.to_string_path() == "std::math").unwrap();
        
        assert!(io_pos < main_pos);
        assert!(math_pos < main_pos);
    }
}
