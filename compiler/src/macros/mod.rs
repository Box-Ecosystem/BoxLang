//! Macro system for BoxLang
//!
//! BoxLang provides a powerful macro system similar to Rust's,
//! including derive macros and attribute macros for metaprogramming.

use crate::ast::{EnumDef, Ident, Item, StructDef};
use std::collections::HashMap;

pub mod attr;
pub mod decl;
pub mod derive;
pub mod hygiene;

pub use attr::{AttributeMacro, AttributeMacroRegistry};
pub use decl::{
    CaptureKind, DeclarativeMacro, MacroError, MacroExpander, MacroPattern, MacroRule,
    MacroTemplate, RepetitionKind,
};
pub use derive::{DeriveMacro, DeriveMacroRegistry};
pub use hygiene::{HygieneResolver, HygieneTable, HygienicIdent, SyntaxContext};

/// A macro expansion context
#[derive(Debug)]
pub struct MacroContext {
    /// Current module path
    pub module_path: Vec<String>,
    /// Current item name
    pub item_name: String,
    /// Macro-specific data
    pub data: HashMap<String, String>,
}

impl MacroContext {
    /// Create a new macro context
    pub fn new(module_path: Vec<String>, item_name: String) -> Self {
        Self {
            module_path,
            item_name,
            data: HashMap::new(),
        }
    }

    /// Get the full path of the current item
    pub fn full_path(&self) -> String {
        if self.module_path.is_empty() {
            self.item_name.clone()
        } else {
            format!("{}::{}", self.module_path.join("::"), self.item_name)
        }
    }
}

/// Trait for all macro types
pub trait Macro {
    /// Get the macro name
    fn name(&self) -> &str;

    /// Get the macro documentation
    fn docs(&self) -> &str;
}

/// Macro expansion result
pub type MacroResult<T> = Result<T, MacroError>;

/// Macro registry that manages all macros
#[derive(Debug, Default)]
pub struct MacroRegistry {
    derive_registry: DeriveMacroRegistry,
    attr_registry: AttributeMacroRegistry,
}

impl MacroRegistry {
    /// Create a new macro registry with built-in macros
    pub fn new() -> Self {
        let mut registry = Self {
            derive_registry: DeriveMacroRegistry::new(),
            attr_registry: AttributeMacroRegistry::new(),
        };

        // Register built-in derive macros
        registry.register_builtin_derives();

        registry
    }

    /// Register built-in derive macros
    fn register_builtin_derives(&mut self) {
        use derive::builtin::*;

        // Register Debug derive macro
        let _ = self.derive_registry.register(Box::new(DebugMacro));

        // Register Clone derive macro
        let _ = self.derive_registry.register(Box::new(CloneMacro));

        // Register Copy derive macro
        let _ = self.derive_registry.register(Box::new(CopyMacro));

        // Register Default derive macro
        let _ = self.derive_registry.register(Box::new(DefaultMacro));

        // Register Eq derive macro
        let _ = self.derive_registry.register(Box::new(EqMacro));

        // Register PartialEq derive macro
        let _ = self.derive_registry.register(Box::new(PartialEqMacro));
    }

    /// Register a derive macro
    pub fn register_derive(&mut self, macro_def: Box<dyn DeriveMacro>) -> MacroResult<()> {
        self.derive_registry.register(macro_def)
    }

    /// Register an attribute macro
    pub fn register_attr(&mut self, macro_def: Box<dyn AttributeMacro>) -> MacroResult<()> {
        self.attr_registry.register(macro_def)
    }

    /// Get all registered derive macros
    pub fn list_derive_macros(&self) -> Vec<&str> {
        self.derive_registry.list_macros()
    }

    /// Get all registered attribute macros
    pub fn list_attr_macros(&self) -> Vec<&str> {
        self.attr_registry.list_macros()
    }
}

/// Helper functions for macro expansion
pub mod utils {
    /// Generate a field access expression
    pub fn gen_field_access(field_name: &str) -> String {
        format!("self.{}", field_name)
    }

    /// Generate a method call expression
    pub fn gen_method_call(receiver: &str, method: &str, args: &[String]) -> String {
        if args.is_empty() {
            format!("{}.{}()", receiver, method)
        } else {
            format!("{}.{}({})", receiver, method, args.join(", "))
        }
    }

    /// Generate a match arm
    pub fn gen_match_arm(pattern: &str, expr: &str) -> String {
        format!("{} => {}", pattern, expr)
    }

    /// Generate an impl block
    pub fn gen_impl_block(type_name: &str, trait_name: Option<&str>, body: &str) -> String {
        if let Some(trait_name) = trait_name {
            format!("impl {} for {} {{\n{}\n}}", trait_name, type_name, body)
        } else {
            format!("impl {} {{\n{}\n}}", type_name, body)
        }
    }

    /// Generate a function definition
    pub fn gen_fn_def(
        name: &str,
        params: &[(String, String)],
        return_type: Option<&str>,
        body: &str,
    ) -> String {
        let params_str = params
            .iter()
            .map(|(name, ty)| format!("{}: {}", name, ty))
            .collect::<Vec<_>>()
            .join(", ");

        let ret_str = return_type.map_or(String::new(), |ty| format!(" -> {}", ty));

        format!("fn {}({}){} {{\n{}\n}}", name, params_str, ret_str, body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macro_context() {
        let ctx = MacroContext::new(
            vec!["std".to_string(), "io".to_string()],
            "Result".to_string(),
        );

        assert_eq!(ctx.full_path(), "std::io::Result");
    }

    #[test]
    fn test_macro_registry_creation() {
        let registry = MacroRegistry::new();

        // Check that built-in macros are registered
        let derives = registry.list_derive_macros();
        assert!(derives.contains(&"Debug"));
        assert!(derives.contains(&"Clone"));
        assert!(derives.contains(&"Copy"));
    }

    #[test]
    fn test_utils() {
        assert_eq!(utils::gen_field_access("x"), "self.x");
        assert_eq!(utils::gen_method_call("obj", "method", &[]), "obj.method()");
        assert_eq!(
            utils::gen_method_call("obj", "method", &["arg1".to_string(), "arg2".to_string()]),
            "obj.method(arg1, arg2)"
        );
    }
}
