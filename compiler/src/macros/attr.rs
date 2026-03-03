//! Attribute macros for BoxLang
//!
//! Attribute macros allow custom attributes to transform items at compile time.

use super::{Macro, MacroContext, MacroError, MacroResult};
use crate::ast::Item;

/// Trait for attribute macros
pub trait AttributeMacro: Macro {
    /// Apply the macro to an item
    fn apply(
        &self,
        item: &mut Item,
        args: &[crate::ast::Expr],
        ctx: &MacroContext,
    ) -> MacroResult<()>;
}

/// Registry for attribute macros
#[derive(Default)]
pub struct AttributeMacroRegistry {
    macros: std::collections::HashMap<String, Box<dyn AttributeMacro>>,
}

impl std::fmt::Debug for AttributeMacroRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AttributeMacroRegistry")
            .field("macros", &self.macros.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl AttributeMacroRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            macros: std::collections::HashMap::new(),
        }
    }

    /// Register an attribute macro
    pub fn register(&mut self, macro_def: Box<dyn AttributeMacro>) -> MacroResult<()> {
        let name = macro_def.name().to_string();

        if self.macros.contains_key(&name) {
            return Err(MacroError::InvalidPattern {
                message: format!("macro '{}' already registered", name),
            });
        }

        self.macros.insert(name, macro_def);
        Ok(())
    }

    /// Look up an attribute macro by name
    pub fn lookup(&self, name: &str) -> Option<&dyn AttributeMacro> {
        self.macros.get(name).map(|m| m.as_ref())
    }

    /// List all registered macros
    pub fn list_macros(&self) -> Vec<&str> {
        self.macros.keys().map(|k| k.as_str()).collect()
    }
}

/// Built-in attribute macros
pub mod builtin {
    use super::*;

    /// Test attribute macro - marks a function as a test
    pub struct TestMacro;

    impl Macro for TestMacro {
        fn name(&self) -> &str {
            "test"
        }

        fn docs(&self) -> &str {
            "Mark a function as a test case"
        }
    }

    impl AttributeMacro for TestMacro {
        fn apply(
            &self,
            item: &mut Item,
            _args: &[crate::ast::Expr],
            _ctx: &MacroContext,
        ) -> MacroResult<()> {
            match item {
                Item::Function(_func) => Ok(()),
                _ => Err(MacroError::InvalidPattern {
                    message: "test can only be applied to functions".to_string(),
                }),
            }
        }
    }

    /// Inline attribute macro - suggests function inlining
    pub struct InlineMacro;

    impl Macro for InlineMacro {
        fn name(&self) -> &str {
            "inline"
        }

        fn docs(&self) -> &str {
            "Suggest that the function should be inlined"
        }
    }

    impl AttributeMacro for InlineMacro {
        fn apply(
            &self,
            item: &mut Item,
            _args: &[crate::ast::Expr],
            _ctx: &MacroContext,
        ) -> MacroResult<()> {
            match item {
                Item::Function(_func) => Ok(()),
                _ => Err(MacroError::InvalidPattern {
                    message: "inline can only be applied to functions".to_string(),
                }),
            }
        }
    }

    /// NoMangle attribute macro - prevents name mangling
    pub struct NoMangleMacro;

    impl Macro for NoMangleMacro {
        fn name(&self) -> &str {
            "no_mangle"
        }

        fn docs(&self) -> &str {
            "Prevent name mangling for this item"
        }
    }

    impl AttributeMacro for NoMangleMacro {
        fn apply(
            &self,
            item: &mut Item,
            _args: &[crate::ast::Expr],
            _ctx: &MacroContext,
        ) -> MacroResult<()> {
            match item {
                Item::Function(_func) => Ok(()),
                _ => Err(MacroError::InvalidPattern {
                    message: "no_mangle can only be applied to functions".to_string(),
                }),
            }
        }
    }

    /// Doc attribute macro - adds documentation
    pub struct DocMacro;

    impl Macro for DocMacro {
        fn name(&self) -> &str {
            "doc"
        }

        fn docs(&self) -> &str {
            "Add documentation to an item"
        }
    }

    impl AttributeMacro for DocMacro {
        fn apply(
            &self,
            item: &mut Item,
            args: &[crate::ast::Expr],
            _ctx: &MacroContext,
        ) -> MacroResult<()> {
            if args.is_empty() {
                return Err(MacroError::InvalidPattern {
                    message: "doc expects a documentation string".to_string(),
                });
            }

            match item {
                Item::Function(_func) => Ok(()),
                Item::Struct(_struct_def) => Ok(()),
                Item::Enum(_enum_def) => Ok(()),
                _ => Ok(()),
            }
        }
    }

    /// Allow attribute macro - allows specific lints
    pub struct AllowMacro;

    impl Macro for AllowMacro {
        fn name(&self) -> &str {
            "allow"
        }

        fn docs(&self) -> &str {
            "Allow specific lints for this item"
        }
    }

    impl AttributeMacro for AllowMacro {
        fn apply(
            &self,
            item: &mut Item,
            args: &[crate::ast::Expr],
            _ctx: &MacroContext,
        ) -> MacroResult<()> {
            if args.is_empty() {
                return Err(MacroError::InvalidPattern {
                    message: "allow expects a lint name".to_string(),
                });
            }

            match item {
                Item::Function(_func) => Ok(()),
                Item::Struct(_struct_def) => Ok(()),
                Item::Enum(_enum_def) => Ok(()),
                _ => Ok(()),
            }
        }
    }

    /// MustUse attribute macro - warns if result is unused
    pub struct MustUseMacro;

    impl Macro for MustUseMacro {
        fn name(&self) -> &str {
            "must_use"
        }

        fn docs(&self) -> &str {
            "Warn if the return value is not used"
        }
    }

    impl AttributeMacro for MustUseMacro {
        fn apply(
            &self,
            item: &mut Item,
            _args: &[crate::ast::Expr],
            _ctx: &MacroContext,
        ) -> MacroResult<()> {
            match item {
                Item::Function(_func) => Ok(()),
                Item::Struct(_struct_def) => Ok(()),
                Item::Enum(_enum_def) => Ok(()),
                _ => Err(MacroError::InvalidPattern {
                    message: "must_use can only be applied to functions, structs, or enums"
                        .to_string(),
                }),
            }
        }
    }

    /// Repr attribute macro - controls memory representation
    pub struct ReprMacro;

    impl Macro for ReprMacro {
        fn name(&self) -> &str {
            "repr"
        }

        fn docs(&self) -> &str {
            "Control the memory representation of a type"
        }
    }

    impl AttributeMacro for ReprMacro {
        fn apply(
            &self,
            item: &mut Item,
            args: &[crate::ast::Expr],
            _ctx: &MacroContext,
        ) -> MacroResult<()> {
            if args.is_empty() {
                return Err(MacroError::InvalidPattern {
                    message: "repr expects a representation type (e.g., C, transparent, packed)"
                        .to_string(),
                });
            }

            match item {
                Item::Struct(_struct_def) => Ok(()),
                Item::Enum(_enum_def) => Ok(()),
                _ => Err(MacroError::InvalidPattern {
                    message: "repr can only be applied to structs or enums".to_string(),
                }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::builtin::*;
    use super::*;

    #[test]
    fn test_test_macro_name() {
        let test = TestMacro;
        assert_eq!(test.name(), "test");
    }

    #[test]
    fn test_inline_macro_name() {
        let inline = InlineMacro;
        assert_eq!(inline.name(), "inline");
    }

    #[test]
    fn test_registry() {
        let mut registry = AttributeMacroRegistry::new();

        registry.register(Box::new(TestMacro)).unwrap();
        registry.register(Box::new(InlineMacro)).unwrap();

        assert!(registry.lookup("test").is_some());
        assert!(registry.lookup("inline").is_some());
        assert!(registry.lookup("nonexistent").is_none());

        let macros = registry.list_macros();
        assert!(macros.contains(&"test"));
        assert!(macros.contains(&"inline"));
    }
}
