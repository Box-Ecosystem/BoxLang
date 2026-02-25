//! Derive macros for BoxLang
//!
//! Derive macros automatically implement traits for structs and enums.

use super::{Macro, MacroContext, MacroError, MacroResult};
use crate::ast::{EnumDef, StructDef};

/// Trait for derive macros
pub trait DeriveMacro: Macro {
    /// Expand the macro for a struct
    fn expand_struct(&self, struct_def: &mut StructDef, ctx: &MacroContext) -> MacroResult<()>;

    /// Expand the macro for an enum
    fn expand_enum(&self, enum_def: &mut EnumDef, ctx: &MacroContext) -> MacroResult<()>;
}

/// Registry for derive macros
#[derive(Default)]
pub struct DeriveMacroRegistry {
    macros: std::collections::HashMap<String, Box<dyn DeriveMacro>>,
}

impl std::fmt::Debug for DeriveMacroRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeriveMacroRegistry")
            .field("macros", &self.macros.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl DeriveMacroRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            macros: std::collections::HashMap::new(),
        }
    }

    /// Register a derive macro
    pub fn register(&mut self, macro_def: Box<dyn DeriveMacro>) -> MacroResult<()> {
        let name = macro_def.name().to_string();

        if self.macros.contains_key(&name) {
            return Err(MacroError::InvalidPattern {
                message: format!("macro '{}' already registered", name),
            });
        }

        self.macros.insert(name, macro_def);
        Ok(())
    }

    /// Look up a derive macro by name
    pub fn lookup(&self, name: &str) -> Option<&dyn DeriveMacro> {
        self.macros.get(name).map(|m| m.as_ref())
    }

    /// List all registered macros
    pub fn list_macros(&self) -> Vec<&str> {
        self.macros.keys().map(|k| k.as_str()).collect()
    }
}

/// Built-in derive macros
pub mod builtin {
    use super::*;

    /// Debug derive macro - generates fmt::Debug implementation
    pub struct DebugMacro;

    impl Macro for DebugMacro {
        fn name(&self) -> &str {
            "Debug"
        }

        fn docs(&self) -> &str {
            "Generate fmt::Debug implementation for the type"
        }
    }

    impl DeriveMacro for DebugMacro {
        fn expand_struct(
            &self,
            struct_def: &mut StructDef,
            _ctx: &MacroContext,
        ) -> MacroResult<()> {
            // Generate Debug implementation for struct
            let _type_name = struct_def.name.to_string();

            // In a full implementation, this would add the impl block to the AST
            // For now, we just acknowledge the derive
            Ok(())
        }

        fn expand_enum(&self, enum_def: &mut EnumDef, _ctx: &MacroContext) -> MacroResult<()> {
            // Generate Debug implementation for enum
            let _type_name = enum_def.name.to_string();

            Ok(())
        }
    }

    /// Clone derive macro - generates Clone implementation
    pub struct CloneMacro;

    impl Macro for CloneMacro {
        fn name(&self) -> &str {
            "Clone"
        }

        fn docs(&self) -> &str {
            "Generate Clone implementation for the type"
        }
    }

    impl DeriveMacro for CloneMacro {
        fn expand_struct(
            &self,
            struct_def: &mut StructDef,
            _ctx: &MacroContext,
        ) -> MacroResult<()> {
            let _type_name = struct_def.name.to_string();
            Ok(())
        }

        fn expand_enum(&self, enum_def: &mut EnumDef, _ctx: &MacroContext) -> MacroResult<()> {
            let _type_name = enum_def.name.to_string();
            Ok(())
        }
    }

    /// Copy derive macro - marks type as Copy
    pub struct CopyMacro;

    impl Macro for CopyMacro {
        fn name(&self) -> &str {
            "Copy"
        }

        fn docs(&self) -> &str {
            "Mark the type as Copy (requires Clone)"
        }
    }

    impl DeriveMacro for CopyMacro {
        fn expand_struct(
            &self,
            struct_def: &mut StructDef,
            _ctx: &MacroContext,
        ) -> MacroResult<()> {
            // Check that all fields are Copy
            // In a full implementation, we'd verify this
            let _type_name = struct_def.name.to_string();
            Ok(())
        }

        fn expand_enum(&self, enum_def: &mut EnumDef, _ctx: &MacroContext) -> MacroResult<()> {
            let _type_name = enum_def.name.to_string();
            Ok(())
        }
    }

    /// Default derive macro - generates Default implementation
    pub struct DefaultMacro;

    impl Macro for DefaultMacro {
        fn name(&self) -> &str {
            "Default"
        }

        fn docs(&self) -> &str {
            "Generate Default implementation for the type"
        }
    }

    impl DeriveMacro for DefaultMacro {
        fn expand_struct(
            &self,
            struct_def: &mut StructDef,
            _ctx: &MacroContext,
        ) -> MacroResult<()> {
            let _type_name = struct_def.name.to_string();
            Ok(())
        }

        fn expand_enum(&self, enum_def: &mut EnumDef, _ctx: &MacroContext) -> MacroResult<()> {
            let _type_name = enum_def.name.to_string();
            Ok(())
        }
    }

    /// Eq derive macro - generates Eq implementation
    pub struct EqMacro;

    impl Macro for EqMacro {
        fn name(&self) -> &str {
            "Eq"
        }

        fn docs(&self) -> &str {
            "Generate Eq implementation for the type (marker trait)"
        }
    }

    impl DeriveMacro for EqMacro {
        fn expand_struct(
            &self,
            struct_def: &mut StructDef,
            _ctx: &MacroContext,
        ) -> MacroResult<()> {
            let _type_name = struct_def.name.to_string();
            Ok(())
        }

        fn expand_enum(&self, enum_def: &mut EnumDef, _ctx: &MacroContext) -> MacroResult<()> {
            let _type_name = enum_def.name.to_string();
            Ok(())
        }
    }

    /// PartialEq derive macro - generates PartialEq implementation
    pub struct PartialEqMacro;

    impl Macro for PartialEqMacro {
        fn name(&self) -> &str {
            "PartialEq"
        }

        fn docs(&self) -> &str {
            "Generate PartialEq implementation for the type"
        }
    }

    impl DeriveMacro for PartialEqMacro {
        fn expand_struct(
            &self,
            struct_def: &mut StructDef,
            _ctx: &MacroContext,
        ) -> MacroResult<()> {
            let _type_name = struct_def.name.to_string();
            Ok(())
        }

        fn expand_enum(&self, enum_def: &mut EnumDef, _ctx: &MacroContext) -> MacroResult<()> {
            let _type_name = enum_def.name.to_string();
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::builtin::*;
    use super::*;

    #[test]
    fn test_debug_macro_name() {
        let debug = DebugMacro;
        assert_eq!(debug.name(), "Debug");
    }

    #[test]
    fn test_clone_macro_name() {
        let clone = CloneMacro;
        assert_eq!(clone.name(), "Clone");
    }

    #[test]
    fn test_registry() {
        let mut registry = DeriveMacroRegistry::new();

        registry.register(Box::new(DebugMacro)).unwrap();
        registry.register(Box::new(CloneMacro)).unwrap();

        assert!(registry.lookup("Debug").is_some());
        assert!(registry.lookup("Clone").is_some());
        assert!(registry.lookup("NonExistent").is_none());

        let macros = registry.list_macros();
        assert!(macros.contains(&"Debug"));
        assert!(macros.contains(&"Clone"));
    }
}
