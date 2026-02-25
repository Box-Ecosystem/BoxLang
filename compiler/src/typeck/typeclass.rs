//! Type classes (traits) for BoxLang
//!
//! Type classes provide ad-hoc polymorphism, allowing different types
//! to implement the same interface.

use crate::ast::{Ident, Path, PathSegment, Span, Type as AstType};
use std::collections::HashMap;

/// Helper function to create a simple path type
fn path_type(name: &str) -> AstType {
    AstType::Path(Path {
        segments: vec![PathSegment {
            ident: Ident::new(name),
            generics: vec![],
        }],
    })
}

/// A type class definition
#[derive(Debug, Clone)]
pub struct TypeClass {
    /// Name of the type class
    pub name: Ident,
    /// Type parameters
    pub params: Vec<TypeParam>,
    /// Associated types
    pub associated_types: Vec<AssociatedType>,
    /// Methods defined by this type class
    pub methods: Vec<TypeClassMethod>,
    /// Super classes (type class inheritance)
    pub super_classes: Vec<Ident>,
    /// Span in source code
    pub span: Span,
}

/// Type parameter for type classes
#[derive(Debug, Clone)]
pub struct TypeParam {
    /// Name of the parameter
    pub name: Ident,
    /// Bounds (e.g., T: Display + Debug)
    pub bounds: Vec<Ident>,
    /// Default type if any
    pub default: Option<AstType>,
    /// Is this a const generic parameter?
    pub is_const: bool,
}

/// Associated type in a type class
#[derive(Debug, Clone)]
pub struct AssociatedType {
    /// Name of the associated type
    pub name: Ident,
    /// Bounds on the associated type
    pub bounds: Vec<Ident>,
    /// Default type if any
    pub default: Option<AstType>,
}

/// Method in a type class
#[derive(Debug, Clone)]
pub struct TypeClassMethod {
    /// Method name
    pub name: Ident,
    /// Method signature
    pub signature: MethodSignature,
    /// Default implementation if any
    pub default_impl: Option<()>, // Would be Expr in full implementation
    /// Documentation
    pub docs: String,
}

/// Method signature
#[derive(Debug, Clone)]
pub struct MethodSignature {
    /// Generic parameters
    pub generics: Vec<TypeParam>,
    /// Parameters
    pub params: Vec<Param>,
    /// Return type
    pub return_type: Option<AstType>,
    /// Is this a const fn?
    pub is_const: bool,
    /// Is this unsafe?
    pub is_unsafe: bool,
}

/// Parameter in a method signature
#[derive(Debug, Clone)]
pub struct Param {
    /// Parameter name
    pub name: Ident,
    /// Parameter type
    pub ty: AstType,
    /// Is this a self parameter?
    pub is_self: bool,
    /// Is self by reference?
    pub is_ref: bool,
    /// Is self mutable?
    pub is_mut: bool,
}

/// Implementation of a type class for a specific type
#[derive(Debug, Clone)]
pub struct Impl {
    /// Type class being implemented
    pub type_class: Ident,
    /// Generic parameters for this impl
    pub generics: Vec<TypeParam>,
    /// Type being implemented for
    pub for_type: AstType,
    /// Associated type assignments
    pub associated_types: HashMap<Ident, AstType>,
    /// Method implementations
    pub methods: Vec<MethodImpl>,
    /// Span in source code
    pub span: Span,
}

/// Method implementation
#[derive(Debug, Clone)]
pub struct MethodImpl {
    /// Method name
    pub name: Ident,
    /// Generic parameters
    pub generics: Vec<TypeParam>,
    /// Parameters
    pub params: Vec<Param>,
    /// Return type
    pub return_type: Option<AstType>,
    /// Body of the method
    pub body: (), // Would be Expr in full implementation
    /// Documentation
    pub docs: String,
}

/// Type class registry - stores all type class definitions and implementations
#[derive(Debug, Default)]
pub struct TypeClassRegistry {
    /// Type class definitions
    type_classes: HashMap<Ident, TypeClass>,
    /// Implementations: (type_class_name, for_type) -> Impl
    implementations: HashMap<(Ident, String), Impl>,
    /// Orphan rules check enabled
    orphan_check: bool,
}

impl TypeClassRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            type_classes: HashMap::new(),
            implementations: HashMap::new(),
            orphan_check: true,
        }
    }

    /// Register a type class definition
    pub fn register_type_class(&mut self, type_class: TypeClass) -> Result<(), TypeClassError> {
        let name = type_class.name.clone();

        if self.type_classes.contains_key(&name) {
            return Err(TypeClassError::DuplicateTypeClass {
                name: name.to_string(),
            });
        }

        self.type_classes.insert(name, type_class);
        Ok(())
    }

    /// Register an implementation
    pub fn register_impl(&mut self, impl_def: Impl) -> Result<(), TypeClassError> {
        let type_class = impl_def.type_class.clone();
        let for_type = Self::type_to_string(&impl_def.for_type);
        let key = (type_class.clone(), for_type.clone());

        if self.implementations.contains_key(&key) {
            return Err(TypeClassError::DuplicateImpl {
                type_class: type_class.to_string(),
                for_type,
            });
        }

        // Check orphan rules if enabled
        if self.orphan_check {
            self.check_orphan_rules(&impl_def)?;
        }

        // Check that all required methods are implemented
        self.check_required_methods(&impl_def)?;

        self.implementations.insert(key, impl_def);
        Ok(())
    }

    /// Convert a type to a simple string representation
    fn type_to_string(ty: &AstType) -> String {
        match ty {
            AstType::Path(path) => {
                path.segments.iter()
                    .map(|s| s.ident.as_str())
                    .collect::<Vec<_>>()
                    .join("::")
            }
            AstType::Ref(inner, is_mut) => {
                if *is_mut {
                    format!("&mut {}", Self::type_to_string(inner))
                } else {
                    format!("&{}", Self::type_to_string(inner))
                }
            }
            AstType::Ptr(inner, is_mut) => {
                if *is_mut {
                    format!("*mut {}", Self::type_to_string(inner))
                } else {
                    format!("*const {}", Self::type_to_string(inner))
                }
            }
            AstType::Array(inner, size) => {
                match size {
                    Some(n) => format!("[{}; {}]", Self::type_to_string(inner), n),
                    None => format!("[{}]", Self::type_to_string(inner)),
                }
            }
            AstType::Slice(inner) => {
                format!("[{}]", Self::type_to_string(inner))
            }
            AstType::Tuple(elems) => {
                let inner: Vec<String> = elems.iter().map(Self::type_to_string).collect();
                format!("({})", inner.join(", "))
            }
            AstType::Function(func) => {
                let params: Vec<String> = func.params.iter().map(Self::type_to_string).collect();
                let ret = Self::type_to_string(&func.return_type);
                format!("fn({}) -> {}", params.join(", "), ret)
            }
            AstType::Unit => "()".to_string(),
            AstType::Never => "!".to_string(),
            AstType::Generic(base, args) => {
                let base_str = Self::type_to_string(base);
                let args_str: Vec<String> = args.iter().map(Self::type_to_string).collect();
                format!("{}<{}>", base_str, args_str.join(", "))
            }
            AstType::ImplTrait(bounds) => {
                let traits: Vec<String> = bounds.iter()
                    .map(|b| b.path.segments.iter()
                        .map(|s| s.ident.as_str())
                        .collect::<Vec<_>>()
                        .join("::"))
                    .collect();
                format!("impl {}", traits.join(" + "))
            }
            AstType::DynTrait(bounds) => {
                let traits: Vec<String> = bounds.iter()
                    .map(|b| b.path.segments.iter()
                        .map(|s| s.ident.as_str())
                        .collect::<Vec<_>>()
                        .join("::"))
                    .collect();
                format!("dyn {}", traits.join(" + "))
            }
        }
    }

    /// Look up a type class by name
    pub fn lookup_type_class(&self, name: &Ident) -> Option<&TypeClass> {
        self.type_classes.get(name)
    }

    /// Look up an implementation
    pub fn lookup_impl(&self, type_class: &Ident, for_type: &str) -> Option<&Impl> {
        self.implementations
            .get(&(type_class.clone(), for_type.to_string()))
    }

    /// Check if a type implements a type class
    pub fn implements(&self, type_class: &Ident, for_type: &str) -> bool {
        self.lookup_impl(type_class, for_type).is_some()
    }

    /// Get all implementations for a type class
    pub fn implementations_for(&self, type_class: &Ident) -> Vec<&Impl> {
        self.implementations
            .values()
            .filter(|impl_def| impl_def.type_class == *type_class)
            .collect()
    }

    /// Check orphan rules (simplified)
    ///
    /// Orphan rules prevent overlapping implementations and ensure coherence.
    /// In a full implementation, this would check:
    /// 1. Either the type class or the type must be local
    /// 2. No overlapping instances
    fn check_orphan_rules(&self, _impl_def: &Impl) -> Result<(), TypeClassError> {
        // Simplified: always allow for now
        // In a full implementation, check that either:
        // - The type class is defined in the current crate, OR
        // - The type is defined in the current crate
        Ok(())
    }

    /// Check that all required methods are implemented
    fn check_required_methods(&self, impl_def: &Impl) -> Result<(), TypeClassError> {
        let type_class = self
            .lookup_type_class(&impl_def.type_class)
            .ok_or_else(|| TypeClassError::UnknownTypeClass {
                name: impl_def.type_class.to_string(),
            })?;

        let implemented_methods: std::collections::HashSet<_> =
            impl_def.methods.iter().map(|m| m.name.clone()).collect();

        for method in &type_class.methods {
            if method.default_impl.is_none() && !implemented_methods.contains(&method.name) {
                return Err(TypeClassError::MissingMethod {
                    type_class: type_class.name.to_string(),
                    method: method.name.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Enable/disable orphan checking
    pub fn set_orphan_check(&mut self, enabled: bool) {
        self.orphan_check = enabled;
    }
}

/// Errors related to type classes
#[derive(Debug, Clone)]
pub enum TypeClassError {
    /// Type class already defined
    DuplicateTypeClass { name: String },
    /// Implementation already exists
    DuplicateImpl {
        type_class: String,
        for_type: String,
    },
    /// Unknown type class
    UnknownTypeClass { name: String },
    /// Missing method implementation
    MissingMethod { type_class: String, method: String },
    /// Orphan rule violation
    OrphanRuleViolation {
        type_class: String,
        for_type: String,
    },
    /// Method signature mismatch
    MethodSignatureMismatch {
        method: String,
        expected: String,
        found: String,
    },
    /// Associated type not found
    UnknownAssociatedType { name: String },
    /// Super class not satisfied
    SuperClassNotSatisfied {
        type_class: String,
        super_class: String,
    },
}

impl std::fmt::Display for TypeClassError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeClassError::DuplicateTypeClass { name } => {
                write!(f, "type class '{}' is already defined", name)
            }
            TypeClassError::DuplicateImpl {
                type_class,
                for_type,
            } => {
                write!(
                    f,
                    "implementation of '{}' for '{}' already exists",
                    type_class, for_type
                )
            }
            TypeClassError::UnknownTypeClass { name } => {
                write!(f, "unknown type class '{}'", name)
            }
            TypeClassError::MissingMethod { type_class, method } => {
                write!(
                    f,
                    "missing method '{}' in implementation of '{}'",
                    method, type_class
                )
            }
            TypeClassError::OrphanRuleViolation {
                type_class,
                for_type,
            } => {
                write!(
                    f,
                    "orphan rule violation: cannot implement '{}' for '{}'",
                    type_class, for_type
                )
            }
            TypeClassError::MethodSignatureMismatch {
                method,
                expected,
                found,
            } => {
                write!(
                    f,
                    "method '{}' has signature '{}' but expected '{}'",
                    method, found, expected
                )
            }
            TypeClassError::UnknownAssociatedType { name } => {
                write!(f, "unknown associated type '{}'", name)
            }
            TypeClassError::SuperClassNotSatisfied {
                type_class,
                super_class,
            } => {
                write!(
                    f,
                    "super class '{}' not satisfied for '{}'",
                    super_class, type_class
                )
            }
        }
    }
}

impl std::error::Error for TypeClassError {}

/// Built-in type classes that BoxLang provides
pub mod builtin {
    use super::*;

    /// Create the Display type class
    pub fn display_type_class() -> TypeClass {
        TypeClass {
            name: Ident::new("Display"),
            params: vec![TypeParam {
                name: Ident::new("Self"),
                bounds: vec![],
                default: None,
                is_const: false,
            }],
            associated_types: vec![],
            methods: vec![TypeClassMethod {
                name: Ident::new("fmt"),
                signature: MethodSignature {
                    generics: vec![],
                    params: vec![
                        Param {
                            name: Ident::new("self"),
                            ty: path_type("Self"),
                            is_self: true,
                            is_ref: true,
                            is_mut: false,
                        },
                        Param {
                            name: Ident::new("f"),
                            ty: path_type("Formatter"),
                            is_self: false,
                            is_ref: false,
                            is_mut: false,
                        },
                    ],
                    return_type: Some(path_type("Result")),
                    is_const: false,
                    is_unsafe: false,
                },
                default_impl: None,
                docs: "Format the value using the given formatter".to_string(),
            }],
            super_classes: vec![],
            span: 0..0,
        }
    }

    /// Create the Eq type class
    pub fn eq_type_class() -> TypeClass {
        TypeClass {
            name: Ident::new("Eq"),
            params: vec![TypeParam {
                name: Ident::new("Self"),
                bounds: vec![],
                default: None,
                is_const: false,
            }],
            associated_types: vec![],
            methods: vec![TypeClassMethod {
                name: Ident::new("eq"),
                signature: MethodSignature {
                    generics: vec![],
                    params: vec![
                        Param {
                            name: Ident::new("self"),
                            ty: path_type("Self"),
                            is_self: true,
                            is_ref: true,
                            is_mut: false,
                        },
                        Param {
                            name: Ident::new("other"),
                            ty: path_type("Self"),
                            is_self: false,
                            is_ref: true,
                            is_mut: false,
                        },
                    ],
                    return_type: Some(path_type("bool")),
                    is_const: false,
                    is_unsafe: false,
                },
                default_impl: None,
                docs: "Compare two values for equality".to_string(),
            }],
            super_classes: vec![],
            span: 0..0,
        }
    }

    /// Create the Ord type class
    pub fn ord_type_class() -> TypeClass {
        TypeClass {
            name: Ident::new("Ord"),
            params: vec![TypeParam {
                name: Ident::new("Self"),
                bounds: vec![Ident::new("Eq")],
                default: None,
                is_const: false,
            }],
            associated_types: vec![],
            methods: vec![TypeClassMethod {
                name: Ident::new("cmp"),
                signature: MethodSignature {
                    generics: vec![],
                    params: vec![
                        Param {
                            name: Ident::new("self"),
                            ty: path_type("Self"),
                            is_self: true,
                            is_ref: true,
                            is_mut: false,
                        },
                        Param {
                            name: Ident::new("other"),
                            ty: path_type("Self"),
                            is_self: false,
                            is_ref: true,
                            is_mut: false,
                        },
                    ],
                    return_type: Some(path_type("Ordering")),
                    is_const: false,
                    is_unsafe: false,
                },
                default_impl: None,
                docs: "Compare two values".to_string(),
            }],
            super_classes: vec![Ident::new("Eq")],
            span: 0..0,
        }
    }

    /// Create the Add type class
    pub fn add_type_class() -> TypeClass {
        TypeClass {
            name: Ident::new("Add"),
            params: vec![TypeParam {
                name: Ident::new("Self"),
                bounds: vec![],
                default: None,
                is_const: false,
            }],
            associated_types: vec![AssociatedType {
                name: Ident::new("Output"),
                bounds: vec![],
                default: Some(path_type("Self")),
            }],
            methods: vec![TypeClassMethod {
                name: Ident::new("add"),
                signature: MethodSignature {
                    generics: vec![],
                    params: vec![
                        Param {
                            name: Ident::new("self"),
                            ty: path_type("Self"),
                            is_self: true,
                            is_ref: false,
                            is_mut: false,
                        },
                        Param {
                            name: Ident::new("rhs"),
                            ty: path_type("Self"),
                            is_self: false,
                            is_ref: false,
                            is_mut: false,
                        },
                    ],
                    return_type: Some(path_type("Output")),
                    is_const: false,
                    is_unsafe: false,
                },
                default_impl: None,
                docs: "Add two values".to_string(),
            }],
            super_classes: vec![],
            span: 0..0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::builtin::*;
    use super::*;

    #[test]
    fn test_register_type_class() {
        let mut registry = TypeClassRegistry::new();
        let display = display_type_class();

        assert!(registry.register_type_class(display).is_ok());
        assert!(registry.lookup_type_class(&Ident::new("Display")).is_some());
    }

    #[test]
    fn test_duplicate_type_class() {
        let mut registry = TypeClassRegistry::new();
        let display = display_type_class();

        registry.register_type_class(display.clone()).unwrap();
        assert!(registry.register_type_class(display).is_err());
    }

    #[test]
    fn test_implements() {
        let mut registry = TypeClassRegistry::new();
        let display = display_type_class();

        registry.register_type_class(display).unwrap();

        // Initially, i32 doesn't implement Display
        assert!(!registry.implements(&Ident::new("Display"), "i32"));

        // Add an implementation
        let impl_def = Impl {
            type_class: Ident::new("Display"),
            generics: vec![],
            for_type: path_type("i32"),
            associated_types: HashMap::new(),
            methods: vec![MethodImpl {
                name: Ident::new("fmt"),
                generics: vec![],
                params: vec![],
                return_type: None,
                body: (),
                docs: String::new(),
            }],
            span: 0..0,
        };

        registry.register_impl(impl_def).unwrap();
        assert!(registry.implements(&Ident::new("Display"), "i32"));
    }

    #[test]
    fn test_missing_method() {
        let mut registry = TypeClassRegistry::new();
        let display = display_type_class();

        registry.register_type_class(display).unwrap();

        // Try to register an impl without the required method
        let impl_def = Impl {
            type_class: Ident::new("Display"),
            generics: vec![],
            for_type: path_type("i32"),
            associated_types: HashMap::new(),
            methods: vec![], // Missing fmt method
            span: 0..0,
        };

        assert!(registry.register_impl(impl_def).is_err());
    }
}
