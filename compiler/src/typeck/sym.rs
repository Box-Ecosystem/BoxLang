//! Symbol table for type checking

use crate::ast::{Ident, Span};
use crate::typeck::ty::Ty;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub name: Ident,
    pub kind: SymbolKind,
    pub ty: Ty,
    pub is_mut: bool,
    pub span: Span,
    pub visibility: Visibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
    Crate,
    Super,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Variable,
    Function {
        params: Vec<Ty>,
        ret: Ty,
    },
    ExternFunction {
        params: Vec<Ty>,
        ret: Ty,
        abi: String,
    },
    ExternStatic {
        ty: Ty,
        is_mutable: bool,
    },
    ExternType,
    Callback {
        params: Vec<Ty>,
        ret: Ty,
        abi: String,
    },
    SafeWrapper {
        extern_name: Ident,
        params: Vec<Ty>,
        ret: Ty,
        error_ty: Option<Ty>,
    },
    Type,
    Module {
        path: Vec<String>,
    },
    Const,
    Static,
    Struct {
        fields: Vec<(Ident, Ty)>,
    },
    Method {
        params: Vec<Ty>,
        ret: Ty,
        receiver: MethodReceiver,
        impl_type: Ty,
    },
    Variant {
        parent_enum: Ident,
        variant_index: usize,
        fields: Vec<crate::typeck::ty::FieldDef>,
    },
    Imported {
        original_name: Ident,
        module_path: Vec<String>,
    },
}

/// Method receiver type (self, &self, &mut self)
#[derive(Debug, Clone, PartialEq)]
pub enum MethodReceiver {
    /// self (by value)
    Value,
    /// &self (shared reference)
    Ref,
    /// &mut self (mutable reference)
    RefMut,
    /// No receiver (associated function)
    None,
}

#[derive(Debug, Clone)]
pub struct Scope {
    symbols: HashMap<Ident, Symbol>,
    parent: Option<usize>,
    module_path: Option<Vec<String>>,
}

impl Scope {
    pub fn new(parent: Option<usize>) -> Self {
        Self {
            symbols: HashMap::new(),
            parent,
            module_path: None,
        }
    }

    pub fn with_module(parent: Option<usize>, module_path: Vec<String>) -> Self {
        Self {
            symbols: HashMap::new(),
            parent,
            module_path: Some(module_path),
        }
    }

    pub fn insert(&mut self, symbol: Symbol) {
        self.symbols.insert(symbol.name.clone(), symbol);
    }

    pub fn get(&self, name: &Ident) -> Option<&Symbol> {
        self.symbols.get(name)
    }

    pub fn contains(&self, name: &Ident) -> bool {
        self.symbols.contains_key(name)
    }

    pub fn module_path(&self) -> Option<&[String]> {
        self.module_path.as_deref()
    }

    pub fn symbols(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.values()
    }
}

#[derive(Debug)]
pub struct SymbolTable {
    scopes: Vec<Scope>,
    current_scope: usize,
    modules: HashMap<String, usize>,
    prelude_loaded: bool,
}

impl SymbolTable {
    pub fn new() -> Self {
        let mut table = Self {
            scopes: vec![Scope::new(None)],
            current_scope: 0,
            modules: HashMap::new(),
            prelude_loaded: false,
        };

        table.insert_builtin_types();

        table
    }

    fn insert_builtin_types(&mut self) {
        let builtins = vec![
            ("bool", Ty::Bool),
            ("i8", Ty::I8),
            ("i16", Ty::I16),
            ("i32", Ty::I32),
            ("i64", Ty::I64),
            ("i128", Ty::I128),
            ("isize", Ty::Isize),
            ("u8", Ty::U8),
            ("u16", Ty::U16),
            ("u32", Ty::U32),
            ("u64", Ty::U64),
            ("u128", Ty::U128),
            ("usize", Ty::Usize),
            ("f32", Ty::F32),
            ("f64", Ty::F64),
            ("char", Ty::Char),
            ("str", Ty::Str),
            ("String", Ty::String),
        ];

        for (name, ty) in builtins {
            self.insert(Symbol {
                name: name.into(),
                kind: SymbolKind::Type,
                ty,
                is_mut: false,
                span: 0..0,
                visibility: Visibility::Public,
            });
        }

        self.insert(Symbol {
            name: "println".into(),
            kind: SymbolKind::Function {
                params: vec![Ty::String],
                ret: Ty::Unit,
            },
            ty: Ty::Fn {
                params: vec![Ty::String],
                ret: Box::new(Ty::Unit),
            },
            is_mut: false,
            span: 0..0,
            visibility: Visibility::Public,
        });

        self.insert(Symbol {
            name: "print".into(),
            kind: SymbolKind::Function {
                params: vec![Ty::String],
                ret: Ty::Unit,
            },
            ty: Ty::Fn {
                params: vec![Ty::String],
                ret: Box::new(Ty::Unit),
            },
            is_mut: false,
            span: 0..0,
            visibility: Visibility::Public,
        });
    }

    pub fn register_module(&mut self, module_path: &[String]) -> usize {
        let key = module_path.join("::");
        if let Some(&scope_idx) = self.modules.get(&key) {
            return scope_idx;
        }

        let scope_idx = self.scopes.len();
        self.scopes.push(Scope::with_module(Some(0), module_path.to_vec()));
        self.modules.insert(key, scope_idx);
        
        scope_idx
    }

    pub fn enter_module(&mut self, module_path: &[String]) {
        let key = module_path.join("::");
        if let Some(&scope_idx) = self.modules.get(&key) {
            self.current_scope = scope_idx;
        } else {
            let scope_idx = self.scopes.len();
            self.scopes.push(Scope::with_module(Some(0), module_path.to_vec()));
            self.modules.insert(key, scope_idx);
            self.current_scope = scope_idx;
        }
    }

    pub fn enter_scope(&mut self) {
        let new_scope = Scope::new(Some(self.current_scope));
        self.scopes.push(new_scope);
        self.current_scope = self.scopes.len() - 1;
    }

    pub fn exit_scope(&mut self) {
        if let Some(parent) = self.scopes[self.current_scope].parent {
            self.current_scope = parent;
        }
    }

    pub fn exit_to_root(&mut self) {
        self.current_scope = 0;
    }

    pub fn insert(&mut self, symbol: Symbol) {
        self.scopes[self.current_scope].insert(symbol);
    }

    pub fn insert_in_module(&mut self, module_path: &[String], symbol: Symbol) {
        let key = module_path.join("::");
        if let Some(&scope_idx) = self.modules.get(&key) {
            self.scopes[scope_idx].insert(symbol);
        } else {
            let scope_idx = self.register_module(module_path);
            self.scopes[scope_idx].insert(symbol);
        }
    }

    pub fn lookup(&self, name: &Ident) -> Option<&Symbol> {
        let mut scope_idx = self.current_scope;

        loop {
            if let Some(symbol) = self.scopes[scope_idx].get(name) {
                return Some(symbol);
            }

            if let Some(parent) = self.scopes[scope_idx].parent {
                scope_idx = parent;
            } else {
                return None;
            }
        }
    }

    pub fn lookup_in_module(&self, module_path: &[String], name: &Ident) -> Option<&Symbol> {
        let key = module_path.join("::");
        if let Some(&scope_idx) = self.modules.get(&key) {
            self.scopes[scope_idx].get(name)
        } else {
            None
        }
    }

    pub fn lookup_current(&self, name: &Ident) -> Option<&Symbol> {
        self.scopes[self.current_scope].get(name)
    }

    pub fn contains(&self, name: &Ident) -> bool {
        self.lookup(name).is_some()
    }

    pub fn contains_current(&self, name: &Ident) -> bool {
        self.scopes[self.current_scope].contains(name)
    }

    pub fn get_var_type(&self, name: &Ident) -> Option<&Ty> {
        self.lookup(name).map(|s| &s.ty)
    }

    pub fn get_function(&self, name: &Ident) -> Option<(Vec<Ty>, Ty)> {
        self.lookup(name).and_then(|s| match &s.kind {
            SymbolKind::Function { params, ret } => Some((params.clone(), ret.clone())),
            _ => None,
        })
    }

    pub fn get_function_ref(&self, name: &Ident) -> Option<(&[Ty], &Ty)> {
        self.lookup(name).and_then(|s| match &s.kind {
            SymbolKind::Function { params, ret } => Some((params.as_slice(), ret)),
            _ => None,
        })
    }

    pub fn is_type(&self, name: &Ident) -> bool {
        self.lookup(name)
            .map(|s| matches!(s.kind, SymbolKind::Type))
            .unwrap_or(false)
    }

    pub fn is_mutable(&self, name: &Ident) -> bool {
        self.lookup(name).map(|s| s.is_mut).unwrap_or(false)
    }

    pub fn get_struct_fields(&self, name: &Ident) -> Option<Vec<(Ident, Ty)>> {
        self.lookup(name).and_then(|s| match &s.kind {
            SymbolKind::Struct { fields } => Some(fields.clone()),
            _ => None,
        })
    }

    pub fn get_method(
        &self,
        type_name: &Ident,
        method_name: &Ident,
    ) -> Option<(Vec<Ty>, Ty, MethodReceiver)> {
        let full_name: Ident = format!("{}::{}", type_name, method_name).into();
        self.lookup(&full_name).and_then(|s| match &s.kind {
            SymbolKind::Method {
                params,
                ret,
                receiver,
                ..
            } => Some((params.clone(), ret.clone(), receiver.clone())),
            _ => None,
        })
    }

    pub fn get_field_type(&self, struct_name: &Ident, field_name: &Ident) -> Option<Ty> {
        self.get_struct_fields(struct_name).and_then(|fields| {
            fields
                .iter()
                .find(|(name, _)| name == field_name)
                .map(|(_, ty)| ty.clone())
        })
    }

    pub fn get_module_symbols(&self, module_path: &[String]) -> Vec<&Symbol> {
        let key = module_path.join("::");
        if let Some(&scope_idx) = self.modules.get(&key) {
            self.scopes[scope_idx].symbols().collect()
        } else {
            Vec::new()
        }
    }

    pub fn import_symbol(&mut self, name: Ident, original_name: Ident, module_path: Vec<String>) {
        let symbol = Symbol {
            name,
            kind: SymbolKind::Imported {
                original_name,
                module_path,
            },
            ty: Ty::Error,
            is_mut: false,
            span: 0..0,
            visibility: Visibility::Public,
        };
        self.insert(symbol);
    }

    pub fn import_all_from_module(&mut self, module_path: &[String]) -> Vec<Ident> {
        let key = module_path.join("::");
        if let Some(&scope_idx) = self.modules.get(&key) {
            let symbols: Vec<Symbol> = self.scopes[scope_idx]
                .symbols()
                .filter(|s| s.visibility == Visibility::Public)
                .cloned()
                .collect();
            
            let imported_names: Vec<Ident> = symbols.iter().map(|s| s.name.clone()).collect();
            
            for symbol in symbols {
                if !matches!(symbol.kind, SymbolKind::Module { .. }) {
                    self.insert(symbol);
                }
            }
            
            imported_names
        } else {
            Vec::new()
        }
    }

    pub fn prelude_loaded(&self) -> bool {
        self.prelude_loaded
    }

    pub fn set_prelude_loaded(&mut self, loaded: bool) {
        self.prelude_loaded = loaded;
    }

    pub fn current_module(&self) -> Option<&[String]> {
        self.scopes[self.current_scope].module_path()
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Local variable information
#[derive(Debug, Clone, PartialEq)]
pub struct LocalVar {
    pub name: Ident,
    pub ty: Ty,
    pub is_mut: bool,
    pub span: Span,
}

/// Function information
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionInfo {
    pub name: Ident,
    pub params: Vec<(Ident, Ty)>,
    pub ret: Ty,
    pub span: Span,
}

/// Type information
#[derive(Debug, Clone, PartialEq)]
pub struct TypeInfo {
    pub name: Ident,
    pub ty: Ty,
    pub span: Span,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_symbol(name: &str, kind: SymbolKind, ty: Ty, is_mut: bool) -> Symbol {
        Symbol {
            name: name.into(),
            kind,
            ty,
            is_mut,
            span: 0..10,
            visibility: Visibility::Private,
        }
    }

    #[test]
    fn test_symbol_table_basic() {
        let mut table = SymbolTable::new();

        table.insert(make_symbol("x", SymbolKind::Variable, Ty::I32, false));

        let symbol = table.lookup(&"x".into());
        assert!(symbol.is_some());
        assert_eq!(symbol.unwrap().ty, Ty::I32);
    }

    #[test]
    fn test_symbol_table_scopes() {
        let mut table = SymbolTable::new();

        table.insert(make_symbol("x", SymbolKind::Variable, Ty::I32, false));

        table.enter_scope();

        table.insert(make_symbol("y", SymbolKind::Variable, Ty::F64, true));

        assert!(table.lookup(&"x".into()).is_some());
        assert!(table.lookup(&"y".into()).is_some());

        table.exit_scope();

        assert!(table.lookup(&"x".into()).is_some());
        assert!(table.lookup(&"y".into()).is_none());
    }

    #[test]
    fn test_builtin_types() {
        let table = SymbolTable::new();

        assert!(table.is_type(&"i32".into()));
        assert!(table.is_type(&"f64".into()));
        assert!(table.is_type(&"bool".into()));
        assert!(!table.is_type(&"unknown".into()));
    }

    #[test]
    fn test_module_registration() {
        let mut table = SymbolTable::new();
        
        table.register_module(&["std".to_string(), "io".to_string()]);
        
        let symbol = Symbol {
            name: "println".into(),
            kind: SymbolKind::Function {
                params: vec![Ty::String],
                ret: Ty::Unit,
            },
            ty: Ty::Fn {
                params: vec![Ty::String],
                ret: Box::new(Ty::Unit),
            },
            is_mut: false,
            span: 0..0,
            visibility: Visibility::Public,
        };
        
        table.insert_in_module(&["std".to_string(), "io".to_string()], symbol);
        
        let found = table.lookup_in_module(&["std".to_string(), "io".to_string()], &"println".into());
        assert!(found.is_some());
    }
}
