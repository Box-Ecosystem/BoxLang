//! Type checking logic

use crate::ast::*;
use crate::typeck::error::{TypeError, TypeErrors, TypeResult};
use crate::typeck::sym::{Symbol, SymbolKind, SymbolTable, Visibility};
use crate::typeck::ty::{AdtDef, AdtKind, Mutability, Ty, TypeParamId, TypeVarId};
use crate::typeck::typeclass::{TypeClassRegistry, TypeClassError};
use std::collections::HashMap;

fn make_symbol(name: Ident, kind: SymbolKind, ty: Ty, is_mut: bool, span: Span) -> Symbol {
    Symbol {
        name,
        kind,
        ty,
        is_mut,
        span,
        visibility: Visibility::Private,
    }
}

fn make_public_symbol(name: Ident, kind: SymbolKind, ty: Ty, is_mut: bool, span: Span) -> Symbol {
    Symbol {
        name,
        kind,
        ty,
        is_mut,
        span,
        visibility: Visibility::Public,
    }
}

pub struct TypeChecker {
    symbol_table: SymbolTable,
    errors: TypeErrors,
    current_function_return: Option<Ty>,
    in_async_context: bool,
    source: String,
    type_class_registry: TypeClassRegistry,
    generic_inference: GenericInference,
}

#[derive(Debug, Default)]
pub struct GenericInference {
    type_var_counter: u32,
    substitutions: HashMap<u32, Ty>,
    trait_bounds: HashMap<u32, Vec<Ident>>,
}

impl GenericInference {
    pub fn new() -> Self {
        Self {
            type_var_counter: 0,
            substitutions: HashMap::new(),
            trait_bounds: HashMap::new(),
        }
    }

    pub fn fresh_type_var(&mut self) -> Ty {
        let id = self.type_var_counter;
        self.type_var_counter += 1;
        Ty::Var(crate::typeck::ty::TypeVarId(id))
    }

    fn occurs_check(&self, var_id: &TypeVarId, ty: &Ty) -> bool {
        match ty {
            Ty::Var(id) => id == var_id,
            Ty::Ref(inner, _) => self.occurs_check(var_id, inner),
            Ty::Ptr(inner, _) => self.occurs_check(var_id, inner),
            Ty::Array(inner, _) => self.occurs_check(var_id, inner),
            Ty::Slice(inner) => self.occurs_check(var_id, inner),
            Ty::Tuple(elems) => elems.iter().any(|e| self.occurs_check(var_id, e)),
            Ty::Fn { params, ret } => {
                params.iter().any(|p| self.occurs_check(var_id, p))
                    || self.occurs_check(var_id, ret)
            }
            Ty::Future(inner) => self.occurs_check(var_id, inner),
            Ty::Adt(adt) => adt.variants.iter().any(|v| {
                v.fields.iter().any(|f| self.occurs_check(var_id, &f.ty))
            }),
            _ => false,
        }
    }

    pub fn unify(&mut self, t1: &Ty, t2: &Ty) -> Result<Ty, TypeError> {
        match (t1, t2) {
            (Ty::Var(id1), Ty::Var(id2)) if id1 == id2 => Ok(t1.clone()),
            (Ty::Var(id), other) | (other, Ty::Var(id)) => {
                if self.occurs_check(id, other) {
                    return Err(TypeError::RecursiveType {
                        span: 0..0,
                        line: 0,
                        column: 0,
                    });
                }
                self.substitutions.insert(id.0, other.clone());
                Ok(other.clone())
            }
            (Ty::Ref(a1, m1), Ty::Ref(a2, m2)) if m1 == m2 => {
                let inner = self.unify(a1, a2)?;
                Ok(Ty::Ref(Box::new(inner), *m1))
            }
            (Ty::Array(e1, n1), Ty::Array(e2, n2)) if n1 == n2 => {
                let elem = self.unify(e1, e2)?;
                Ok(Ty::Array(Box::new(elem), *n1))
            }
            (Ty::Tuple(elems1), Ty::Tuple(elems2)) if elems1.len() == elems2.len() => {
                let unified: Result<Vec<_>, _> = elems1
                    .iter()
                    .zip(elems2.iter())
                    .map(|(a, b)| self.unify(a, b))
                    .collect();
                Ok(Ty::Tuple(unified?))
            }
            (a, b) if a == b => Ok(a.clone()),
            _ => Err(TypeError::MismatchedTypes {
                expected: t1.to_string(),
                found: t2.to_string(),
                span: 0..0,
                line: 0,
                column: 0,
            }),
        }
    }

    pub fn resolve(&self, ty: &Ty) -> Ty {
        match ty {
            Ty::Var(id) => {
                if let Some(resolved) = self.substitutions.get(&id.0) {
                    self.resolve(resolved)
                } else {
                    ty.clone()
                }
            }
            Ty::Ref(inner, m) => Ty::Ref(Box::new(self.resolve(inner)), *m),
            Ty::Ptr(inner, m) => Ty::Ptr(Box::new(self.resolve(inner)), *m),
            Ty::Array(inner, n) => Ty::Array(Box::new(self.resolve(inner)), *n),
            Ty::Slice(inner) => Ty::Slice(Box::new(self.resolve(inner))),
            Ty::Tuple(elems) => Ty::Tuple(elems.iter().map(|e| self.resolve(e)).collect()),
            Ty::Fn { params, ret } => Ty::Fn {
                params: params.iter().map(|p| self.resolve(p)).collect(),
                ret: Box::new(self.resolve(ret)),
            },
            Ty::Future(inner) => Ty::Future(Box::new(self.resolve(inner))),
            _ => ty.clone(),
        }
    }

    pub fn add_trait_bound(&mut self, type_var_id: u32, trait_name: Ident) {
        self.trait_bounds
            .entry(type_var_id)
            .or_default()
            .push(trait_name);
    }

    pub fn check_trait_bounds(
        &self,
        ty: &Ty,
        registry: &TypeClassRegistry,
    ) -> Result<(), TypeClassError> {
        if let Ty::Var(id) = ty {
            if let Some(traits) = self.trait_bounds.get(&id.0) {
                for trait_name in traits {
                    let resolved = self.resolve(ty);
                    let type_str = format!("{}", resolved);
                    if !registry.implements(trait_name, &type_str) {
                        return Err(TypeClassError::SuperClassNotSatisfied {
                            type_class: type_str,
                            super_class: trait_name.to_string(),
                        });
                    }
                }
            }
        }
        Ok(())
    }
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut registry = TypeClassRegistry::new();
        
        registry.register_type_class(crate::typeck::typeclass::builtin::display_type_class()).ok();
        registry.register_type_class(crate::typeck::typeclass::builtin::eq_type_class()).ok();
        registry.register_type_class(crate::typeck::typeclass::builtin::ord_type_class()).ok();
        registry.register_type_class(crate::typeck::typeclass::builtin::add_type_class()).ok();

        let mut checker = Self {
            symbol_table: SymbolTable::new(),
            errors: TypeErrors::new(),
            current_function_return: None,
            in_async_context: false,
            source: String::new(),
            type_class_registry: registry,
            generic_inference: GenericInference::new(),
        };
        
        checker.register_builtin_types();
        checker
    }
    
    fn register_builtin_types(&mut self) {
        let result_ty = Ty::Adt(AdtDef {
            name: "Result".into(),
            kind: AdtKind::Enum,
            variants: vec![
                crate::typeck::ty::VariantDef {
                    name: "Ok".into(),
                    fields: vec![crate::typeck::ty::FieldDef {
                        name: "0".into(),
                        ty: Ty::Param(TypeParamId(0)),
                    }],
                },
                crate::typeck::ty::VariantDef {
                    name: "Err".into(),
                    fields: vec![crate::typeck::ty::FieldDef {
                        name: "0".into(),
                        ty: Ty::Param(TypeParamId(1)),
                    }],
                },
            ],
        });
        
        self.symbol_table.insert(make_symbol(
            "Result".into(),
            SymbolKind::Type,
            result_ty,
            false,
            0..0,
        ));
        
        self.symbol_table.insert(make_symbol(
            "Result::Ok".into(),
            SymbolKind::Variant {
                parent_enum: "Result".into(),
                variant_index: 0,
                fields: vec![crate::typeck::ty::FieldDef {
                    name: "0".into(),
                    ty: Ty::Param(TypeParamId(0)),
                }],
            },
            Ty::Fn {
                params: vec![Ty::Param(TypeParamId(0))],
                ret: Box::new(Ty::Adt(AdtDef {
                    name: "Result".into(),
                    kind: AdtKind::Enum,
                    variants: vec![],
                })),
            },
            false,
            0..0,
        ));
        
        self.symbol_table.insert(make_symbol(
            "Result::Err".into(),
            SymbolKind::Variant {
                parent_enum: "Result".into(),
                variant_index: 1,
                fields: vec![crate::typeck::ty::FieldDef {
                    name: "0".into(),
                    ty: Ty::Param(TypeParamId(1)),
                }],
            },
            Ty::Fn {
                params: vec![Ty::Param(TypeParamId(1))],
                ret: Box::new(Ty::Adt(AdtDef {
                    name: "Result".into(),
                    kind: AdtKind::Enum,
                    variants: vec![],
                })),
            },
            false,
            0..0,
        ));
        
        let option_ty = Ty::Adt(AdtDef {
            name: "Option".into(),
            kind: AdtKind::Enum,
            variants: vec![
                crate::typeck::ty::VariantDef {
                    name: "Some".into(),
                    fields: vec![crate::typeck::ty::FieldDef {
                        name: "0".into(),
                        ty: Ty::Param(TypeParamId(0)),
                    }],
                },
                crate::typeck::ty::VariantDef {
                    name: "None".into(),
                    fields: vec![],
                },
            ],
        });
        
        self.symbol_table.insert(make_symbol(
            "Option".into(),
            SymbolKind::Type,
            option_ty,
            false,
            0..0,
        ));
        
        self.symbol_table.insert(make_symbol(
            "Option::Some".into(),
            SymbolKind::Variant {
                parent_enum: "Option".into(),
                variant_index: 0,
                fields: vec![crate::typeck::ty::FieldDef {
                    name: "0".into(),
                    ty: Ty::Param(TypeParamId(0)),
                }],
            },
            Ty::Fn {
                params: vec![Ty::Param(TypeParamId(0))],
                ret: Box::new(Ty::Adt(AdtDef {
                    name: "Option".into(),
                    kind: AdtKind::Enum,
                    variants: vec![],
                })),
            },
            false,
            0..0,
        ));
        
        self.symbol_table.insert(make_symbol(
            "Option::None".into(),
            SymbolKind::Variant {
                parent_enum: "Option".into(),
                variant_index: 1,
                fields: vec![],
            },
            Ty::Adt(AdtDef {
                name: "Option".into(),
                kind: AdtKind::Enum,
                variants: vec![],
            }),
            false,
            0..0,
        ));
    }

    /// Set the source code for error location calculation
    pub fn set_source(&mut self, source: String) {
        self.source = source;
    }

    fn span_to_line_column(&self, span: &Span) -> (usize, usize) {
        if self.source.is_empty() || span.start >= self.source.len() {
            return (0, 0);
        }

        let source_before = &self.source[..span.start];
        let line = source_before.matches('\n').count() + 1;
        let line_start = source_before.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let column = span.start - line_start + 1;

        (line, column)
    }

    fn instantiate_generic_fn(
        &mut self,
        generics: &[GenericParam],
        _args: &[Expr],
    ) -> HashMap<Ident, Ty> {
        let mut subst = HashMap::new();
        
        for gp in generics {
            let ty_var = self.generic_inference.fresh_type_var();
            subst.insert(gp.name.clone(), ty_var);
            
            for bound in &gp.bounds {
                if let Some(ty) = subst.get(&gp.name) {
                    if let Ty::Var(id) = ty {
                        self.generic_inference.add_trait_bound(id.0, bound.path.segments.first().map(|s| s.ident.clone()).unwrap_or_default());
                    }
                }
            }
        }
        
        subst
    }

    fn substitute_generics(&self, ty: &Ty, subst: &HashMap<Ident, Ty>) -> Ty {
        match ty {
            Ty::Param(id) => {
                if let Some(name) = subst.keys().find(|k| format!("'T{}", id.0) == k.as_str()) {
                    subst.get(name).cloned().unwrap_or(ty.clone())
                } else {
                    ty.clone()
                }
            }
            Ty::Ref(inner, m) => Ty::Ref(Box::new(self.substitute_generics(inner, subst)), *m),
            Ty::Ptr(inner, m) => Ty::Ptr(Box::new(self.substitute_generics(inner, subst)), *m),
            Ty::Array(inner, n) => Ty::Array(Box::new(self.substitute_generics(inner, subst)), *n),
            Ty::Slice(inner) => Ty::Slice(Box::new(self.substitute_generics(inner, subst))),
            Ty::Tuple(elems) => Ty::Tuple(elems.iter().map(|e| self.substitute_generics(e, subst)).collect()),
            Ty::Fn { params, ret } => Ty::Fn {
                params: params.iter().map(|p| self.substitute_generics(p, subst)).collect(),
                ret: Box::new(self.substitute_generics(ret, subst)),
            },
            Ty::Future(inner) => Ty::Future(Box::new(self.substitute_generics(inner, subst))),
            Ty::Adt(adt) => Ty::Adt(crate::typeck::ty::AdtDef {
                name: adt.name.clone(),
                kind: adt.kind,
                variants: adt.variants.iter().map(|v| crate::typeck::ty::VariantDef {
                    name: v.name.clone(),
                    fields: v.fields.iter().map(|f| crate::typeck::ty::FieldDef {
                        name: f.name.clone(),
                        ty: self.substitute_generics(&f.ty, subst),
                    }).collect(),
                }).collect(),
            }),
            _ => ty.clone(),
        }
    }

    fn infer_generic_args(
        &mut self,
        params: &[Ty],
        args: &[Expr],
        subst: &mut HashMap<Ident, Ty>,
    ) -> TypeResult<()> {
        for (param_ty, arg) in params.iter().zip(args.iter()) {
            let arg_ty = self.check_expr(arg)?;
            self.unify_with_generic(param_ty, &arg_ty, subst)?;
        }
        Ok(())
    }

    fn unify_with_generic(
        &mut self,
        expected: &Ty,
        found: &Ty,
        subst: &mut HashMap<Ident, Ty>,
    ) -> TypeResult<()> {
        match (expected, found) {
            (Ty::Param(id), other) => {
                let key = format!("'T{}", id.0);
                if let Some(existing) = subst.get(key.as_str()) {
                    if existing != other {
                        return Err(TypeError::MismatchedTypes {
                            expected: existing.to_string(),
                            found: other.to_string(),
                            span: 0..0,
                            line: 0,
                            column: 0,
                        });
                    }
                } else {
                    subst.insert(key.into(), other.clone());
                }
                Ok(())
            }
            (Ty::Ref(e1, m1), Ty::Ref(e2, m2)) if m1 == m2 => {
                self.unify_with_generic(e1, e2, subst)
            }
            (Ty::Array(e1, n1), Ty::Array(e2, n2)) if n1 == n2 => {
                self.unify_with_generic(e1, e2, subst)
            }
            (Ty::Tuple(elems1), Ty::Tuple(elems2)) if elems1.len() == elems2.len() => {
                for (e1, e2) in elems1.iter().zip(elems2.iter()) {
                    self.unify_with_generic(e1, e2, subst)?;
                }
                Ok(())
            }
            (a, b) if a == b => Ok(()),
            _ => Err(TypeError::MismatchedTypes {
                expected: expected.to_string(),
                found: found.to_string(),
                span: 0..0,
                line: 0,
                column: 0,
            }),
        }
    }

    /// Check a module
    pub fn check_module(&mut self, module: &Module) -> TypeResult<()> {
        // First pass: collect all type definitions and function signatures
        for item in &module.items {
            self.collect_item_signature(item);
        }

        // Second pass: check function bodies
        for item in &module.items {
            if let Err(e) = self.check_item(item) {
                self.errors.push(e);
            }
        }

        self.errors.take_result()
    }

    /// Collect item signatures (first pass)
    fn collect_item_signature(&mut self, item: &Item) {
        match item {
            Item::Function(func) => {
                let params: Vec<Ty> = func
                    .params
                    .iter()
                    .map(|p| self.ast_type_to_ty(&p.ty))
                    .collect();
                let ret = func
                    .return_type
                    .as_ref()
                    .map(|t| self.ast_type_to_ty(t))
                    .unwrap_or(Ty::Unit);

                self.symbol_table.insert(make_symbol(
                    func.name.clone(),
                    SymbolKind::Function {
                        params: params.clone(),
                        ret: ret.clone(),
                    },
                    Ty::Fn {
                        params,
                        ret: Box::new(ret),
                    },
                    false,
                    func.span.clone(),
                ));
            }
            Item::Struct(struct_def) => {
                let fields: Vec<(Ident, Ty)> = struct_def
                    .fields
                    .iter()
                    .map(|f| (f.name.clone(), self.ast_type_to_ty(&f.ty)))
                    .collect();

                self.symbol_table.insert(make_symbol(
                    struct_def.name.clone(),
                    SymbolKind::Struct {
                        fields: fields.clone(),
                    },
                    Ty::Adt(crate::typeck::ty::AdtDef {
                        name: struct_def.name.clone(),
                        kind: crate::typeck::ty::AdtKind::Struct,
                        variants: vec![crate::typeck::ty::VariantDef {
                            name: struct_def.name.clone(),
                            fields: fields
                                .iter()
                                .map(|(name, ty)| crate::typeck::ty::FieldDef {
                                    name: name.clone(),
                                    ty: ty.clone(),
                                })
                                .collect(),
                        }],
                    }),
                    false,
                    struct_def.span.clone(),
                ));
            }
            Item::Enum(enum_def) => {
                let variants: Vec<crate::typeck::ty::VariantDef> = enum_def
                    .variants
                    .iter()
                    .map(|v| self.convert_enum_variant(v))
                    .collect();

                let enum_ty = Ty::Adt(AdtDef {
                    name: enum_def.name.as_str().into(),
                    kind: AdtKind::Enum,
                    variants: variants.clone(),
                });

                for (idx, variant) in enum_def.variants.iter().enumerate() {
                    let variant_ty = self.get_variant_constructor_type(enum_def, variant, &enum_ty);
                    let variant_name = format!("{}::{}", enum_def.name, variant.name);

                    self.symbol_table.insert(make_symbol(
                        variant_name.into(),
                        SymbolKind::Variant {
                            parent_enum: enum_def.name.clone(),
                            variant_index: idx,
                            fields: variants[idx].fields.clone(),
                        },
                        variant_ty,
                        false,
                        enum_def.span.clone(),
                    ));
                }

                self.symbol_table.insert(make_symbol(
                    enum_def.name.clone(),
                    SymbolKind::Type,
                    enum_ty,
                    false,
                    enum_def.span.clone(),
                ));
            }
            Item::Impl(impl_block) => {
                self.collect_impl_signatures(impl_block);
            }
            Item::ExternBlock(extern_block) => {
                self.collect_extern_signatures(extern_block);
            }
            _ => {}
        }
    }
    
    fn collect_extern_signatures(&mut self, extern_block: &ExternBlock) {
        for item in &extern_block.items {
            if let ExternItem::Function(func) = item {
                let params: Vec<Ty> = func
                    .params
                    .iter()
                    .map(|p| self.ast_type_to_ty(&p.ty))
                    .collect();
                let ret = func
                    .return_type
                    .as_ref()
                    .map(|t| self.ast_type_to_ty(t))
                    .unwrap_or(Ty::Unit);

                self.symbol_table.insert(make_symbol(
                    func.name.clone(),
                    SymbolKind::Function {
                        params: params.clone(),
                        ret: ret.clone(),
                    },
                    Ty::Fn {
                        params,
                        ret: Box::new(ret),
                    },
                    false,
                    func.span.clone(),
                ));
            }
        }
    }

    /// Convert an AST enum variant to a type system variant definition
    fn convert_enum_variant(&self, variant: &EnumVariant) -> crate::typeck::ty::VariantDef {
        let fields = match &variant.fields {
            EnumVariantFields::Unit => vec![],
            EnumVariantFields::Tuple(types) => types
                .iter()
                .enumerate()
                .map(|(idx, ty)| crate::typeck::ty::FieldDef {
                    name: format!("{}", idx).into(),
                    ty: self.ast_type_to_ty(ty),
                })
                .collect(),
            EnumVariantFields::Struct(field_defs) => field_defs
                .iter()
                .map(|f| crate::typeck::ty::FieldDef {
                    name: f.name.clone(),
                    ty: self.ast_type_to_ty(&f.ty),
                })
                .collect(),
        };

        crate::typeck::ty::VariantDef {
            name: variant.name.clone(),
            fields,
        }
    }

    /// Get the constructor type for an enum variant
    fn get_variant_constructor_type(
        &self,
        _enum_def: &EnumDef,
        variant: &EnumVariant,
        enum_ty: &Ty,
    ) -> Ty {
        match &variant.fields {
            EnumVariantFields::Unit => {
                // Unit variant: just returns the enum type
                enum_ty.clone()
            }
            EnumVariantFields::Tuple(types) => {
                // Tuple variant: fn(T1, T2, ...) -> EnumType
                let param_tys: Vec<Ty> = types.iter().map(|t| self.ast_type_to_ty(t)).collect();
                Ty::Fn {
                    params: param_tys,
                    ret: Box::new(enum_ty.clone()),
                }
            }
            EnumVariantFields::Struct(fields) => {
                // Struct variant: fn(field1: T1, field2: T2, ...) -> EnumType
                // For struct variants, we create a function that takes fields as named parameters
                // But for simplicity in type checking, we treat it like a tuple variant
                let param_tys: Vec<Ty> =
                    fields.iter().map(|f| self.ast_type_to_ty(&f.ty)).collect();
                Ty::Fn {
                    params: param_tys,
                    ret: Box::new(enum_ty.clone()),
                }
            }
        }
    }

    fn collect_impl_signatures(&mut self, impl_block: &ImplBlock) {
        let impl_ty = self.ast_type_to_ty(&impl_block.ty);
        let type_name = self.ty_to_ident(&impl_ty);

        for item in &impl_block.items {
            if let ImplItem::Function(func) = item {
                let receiver = if let Some(first_param) = func.params.first() {
                    let param_name = first_param.name.as_str();
                    match param_name {
                        "self" => {
                            match &first_param.ty {
                                Type::Ref(_, false) => crate::typeck::sym::MethodReceiver::Ref,
                                Type::Ref(_, true) => crate::typeck::sym::MethodReceiver::RefMut,
                                _ => crate::typeck::sym::MethodReceiver::Value,
                            }
                        }
                        "&self" => crate::typeck::sym::MethodReceiver::Ref,
                        "&mut self" => crate::typeck::sym::MethodReceiver::RefMut,
                        _ => {
                            if matches!(first_param.ty, Type::Ref(_, false)) {
                                crate::typeck::sym::MethodReceiver::Ref
                            } else if matches!(first_param.ty, Type::Ref(_, true)) {
                                crate::typeck::sym::MethodReceiver::RefMut
                            } else {
                                crate::typeck::sym::MethodReceiver::None
                            }
                        }
                    }
                } else {
                    crate::typeck::sym::MethodReceiver::None
                };

                let params: Vec<Ty> = func
                    .params
                    .iter()
                    .skip(
                        if matches!(
                            receiver,
                            crate::typeck::sym::MethodReceiver::Value
                                | crate::typeck::sym::MethodReceiver::Ref
                                | crate::typeck::sym::MethodReceiver::RefMut
                        ) {
                            1
                        } else {
                            0
                        },
                    )
                    .map(|p| self.ast_type_to_ty(&p.ty))
                    .collect();

                let ret = func
                    .return_type
                    .as_ref()
                    .map(|t| self.ast_type_to_ty(t))
                    .unwrap_or(Ty::Unit);

                let method_name: Ident = format!("{}::{}", type_name, func.name).into();
                self.symbol_table.insert(make_symbol(
                    method_name,
                    SymbolKind::Method {
                        params: params.clone(),
                        ret: ret.clone(),
                        receiver: receiver.clone(),
                        impl_type: impl_ty.clone(),
                    },
                    Ty::Fn {
                        params,
                        ret: Box::new(ret.clone()),
                    },
                    false,
                    func.span.clone(),
                ));
            }
        }
    }

    /// Convert type to identifier for naming
    fn ty_to_ident(&self, ty: &Ty) -> Ident {
        match ty {
            Ty::Adt(adt) => adt.name.clone(),
            Ty::Named(name) => name.clone(),
            Ty::Ref(inner, _) => self.ty_to_ident(inner),
            Ty::Ptr(inner, _) => self.ty_to_ident(inner),
            Ty::Error => "Error".into(),
            _ => "Unknown".into(),
        }
    }

    fn check_item(&mut self, item: &Item) -> TypeResult<()> {
        match item {
            Item::Function(func) => self.check_function(func),
            Item::Struct(struct_def) => self.check_struct(struct_def),
            Item::Enum(enum_def) => self.check_enum(enum_def),
            Item::Const(const_def) => self.check_const(const_def),
            Item::Static(static_def) => self.check_static(static_def),
            Item::Impl(impl_block) => self.check_impl(impl_block),
            Item::Import(import) => self.check_import(import),
            _ => Ok(()),
        }
    }

    fn check_import(&mut self, import: &Import) -> TypeResult<()> {
        let module_path: Vec<String> = import
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();

        if import.is_glob {
            let imported_names = self.symbol_table.import_all_from_module(&module_path);
            
            for name in imported_names {
                if let Some(symbol) = self.symbol_table.lookup_in_module(&module_path, &name) {
                    let mut imported_symbol = symbol.clone();
                    imported_symbol.visibility = Visibility::Public;
                    self.symbol_table.insert(imported_symbol);
                }
            }
        } else if let Some(last_segment) = import.path.segments.last() {
            let name = if let Some(ref alias) = import.alias {
                alias.clone()
            } else {
                last_segment.ident.clone()
            };

            if module_path.len() > 1 {
                let parent_module: Vec<String> = module_path[..module_path.len() - 1].to_vec();
                let item_name = last_segment.ident.clone();

                if let Some(symbol) = self.symbol_table.lookup_in_module(&parent_module, &item_name) {
                    let mut imported_symbol = symbol.clone();
                    imported_symbol.name = name.clone();
                    imported_symbol.visibility = Visibility::Public;
                    self.symbol_table.insert(imported_symbol);
                } else {
                    self.symbol_table.import_symbol(name, item_name, parent_module);
                }
            } else {
                self.symbol_table.insert(make_symbol(
                    name.clone(),
                    SymbolKind::Module {
                        path: module_path.clone(),
                    },
                    Ty::Module,
                    false,
                    import.span.clone(),
                ));
            }
        }

        Ok(())
    }

    /// Check a function
    fn check_function(&mut self, func: &Function) -> TypeResult<()> {
        self.symbol_table.enter_scope();

        let ret_ty = func
            .return_type
            .as_ref()
            .map(|t| self.ast_type_to_ty(t))
            .unwrap_or(Ty::Unit);
        self.current_function_return = Some(ret_ty.clone());

        for param in &func.params {
            let ty = self.ast_type_to_ty(&param.ty);
            self.symbol_table.insert(make_symbol(
                param.name.clone(),
                SymbolKind::Variable,
                ty,
                param.is_mut,
                param.span.clone(),
            ));
        }

        let body_ty = self.check_block(&func.body)?;

        if !body_ty.can_implicitly_convert_to(&ret_ty) {
            return Err(TypeError::ReturnTypeMismatch {
                expected: ret_ty.to_string(),
                found: body_ty.to_string(),
                span: func.span.clone(),
                line: 0,
                column: 0,
            });
        }

        self.symbol_table.exit_scope();
        self.current_function_return = None;

        Ok(())
    }

    fn check_struct(&mut self, struct_def: &StructDef) -> TypeResult<()> {
        for field in &struct_def.fields {
            let field_ty = self.ast_type_to_ty(&field.ty);
            // Allow Ty::Error for forward references
            if matches!(field_ty, Ty::Error) {
                // Try to resolve the type name from the AST
                if let Type::Path(path) = &field.ty {
                    if let Some(segment) = path.segments.last() {
                        // Register the type as a forward reference
                        self.symbol_table.insert(make_symbol(
                            segment.ident.clone(),
                            SymbolKind::Type,
                            Ty::Named(segment.ident.clone()),
                            false,
                            struct_def.span.clone(),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn check_impl(&mut self, impl_block: &ImplBlock) -> TypeResult<()> {
        let impl_ty = self.ast_type_to_ty(&impl_block.ty);

        let type_name = self.ty_to_ident(&impl_ty);
        
        // Allow Ty::Error for error recovery
        if !matches!(impl_ty, Ty::Error) && self.symbol_table.lookup(&type_name).is_none() {
            return Err(TypeError::UndefinedType {
                name: type_name.to_string(),
                span: impl_block.span.clone(),
                line: 0,
                column: 0,
            });
        }

        for item in &impl_block.items {
            if let ImplItem::Function(func) = item {
                self.check_method(func, &impl_ty)?;
            }
        }

        Ok(())
    }

    fn check_method(&mut self, func: &Function, impl_ty: &Ty) -> TypeResult<()> {
        self.symbol_table.enter_scope();

        let ret_ty = func
            .return_type
            .as_ref()
            .map(|t| self.ast_type_to_ty(t))
            .unwrap_or(Ty::Unit);
        self.current_function_return = Some(ret_ty.clone());

        let mut param_start = 0;
        if let Some(first_param) = func.params.first() {
            let param_name = first_param.name.as_str();
            let is_self_param = param_name == "self" || param_name == "&self" || param_name == "&mut self";
            
            if is_self_param {
                let self_ty = match &first_param.ty {
                    Type::Ref(_inner, is_mut) => {
                        Ty::Ref(
                            Box::new(impl_ty.clone()),
                            if *is_mut { Mutability::Mut } else { Mutability::Not }
                        )
                    }
                    _ => impl_ty.clone(),
                };
                self.symbol_table.insert(make_symbol(
                    "self".into(),
                    SymbolKind::Variable,
                    self_ty,
                    first_param.is_mut,
                    first_param.span.clone(),
                ));
                param_start = 1;
            }
        }

        for param in func.params.iter().skip(param_start) {
            let ty = self.ast_type_to_ty(&param.ty);
            self.symbol_table.insert(make_symbol(
                param.name.clone(),
                SymbolKind::Variable,
                ty,
                param.is_mut,
                param.span.clone(),
            ));
        }

        let body_ty = self.check_block(&func.body)?;

        if !body_ty.can_implicitly_convert_to(&ret_ty) {
            return Err(TypeError::ReturnTypeMismatch {
                expected: ret_ty.to_string(),
                found: body_ty.to_string(),
                span: func.span.clone(),
                line: 0,
                column: 0,
            });
        }

        self.symbol_table.exit_scope();
        self.current_function_return = None;

        Ok(())
    }

    /// Check an enum definition
    fn check_enum(&mut self, enum_def: &EnumDef) -> TypeResult<()> {
        // Check for duplicate variant names
        let mut seen_names = std::collections::HashSet::new();
        for variant in &enum_def.variants {
            if !seen_names.insert(variant.name.clone()) {
                return Err(TypeError::InvalidType {
                    name: format!("duplicate variant name: {}", variant.name),
                    span: enum_def.span.clone(),
                    line: 0,
                    column: 0,
                });
            }

            // Check variant fields
            match &variant.fields {
                EnumVariantFields::Unit => {
                    // Unit variant - no fields to check
                }
                EnumVariantFields::Tuple(types) => {
                    // Tuple variant - check each field type
                    for ty in types {
                        let _ = self.ast_type_to_ty(ty);
                    }
                }
                EnumVariantFields::Struct(fields) => {
                    // Struct variant - check each field
                    let mut field_names = std::collections::HashSet::new();
                    for field in fields {
                        if !field_names.insert(field.name.clone()) {
                            return Err(TypeError::InvalidType {
                                name: format!(
                                    "duplicate field name: {} in variant {}",
                                    field.name, variant.name
                                ),
                                span: enum_def.span.clone(),
                                line: 0,
                                column: 0,
                            });
                        }
                        let _ = self.ast_type_to_ty(&field.ty);
                    }
                }
            }

            // Check discriminant expression if present
            if let Some(discriminant) = &variant.discriminant {
                let disc_ty = self.check_expr(discriminant)?;
                // Discriminant should be an integer type
                if !matches!(
                    disc_ty,
                    Ty::I8
                        | Ty::I16
                        | Ty::I32
                        | Ty::I64
                        | Ty::I128
                        | Ty::U8
                        | Ty::U16
                        | Ty::U32
                        | Ty::U64
                        | Ty::U128
                        | Ty::Isize
                        | Ty::Usize
                ) {
                    return Err(TypeError::MismatchedTypes {
                        expected: "integer type".to_string(),
                        found: disc_ty.to_string(),
                        span: enum_def.span.clone(),
                        line: 0,
                        column: 0,
                    });
                }
            }
        }

        Ok(())
    }

    fn check_const(&mut self, const_def: &ConstDef) -> TypeResult<()> {
        let ty = self.ast_type_to_ty(&const_def.ty);
        let value_ty = self.check_expr(&const_def.value)?;

        if ty != value_ty {
            return Err(TypeError::mismatched_types(
                ty.to_string(),
                value_ty.to_string(),
                const_def.span.clone(),
                0,
                0,
            ));
        }

        self.symbol_table.insert(make_symbol(
            const_def.name.clone(),
            SymbolKind::Const,
            ty,
            false,
            const_def.span.clone(),
        ));

        Ok(())
    }

    fn check_static(&mut self, static_def: &StaticDef) -> TypeResult<()> {
        let ty = self.ast_type_to_ty(&static_def.ty);
        let value_ty = self.check_expr(&static_def.value)?;

        if ty != value_ty {
            return Err(TypeError::mismatched_types(
                ty.to_string(),
                value_ty.to_string(),
                static_def.span.clone(),
                0,
                0,
            ));
        }

        self.symbol_table.insert(make_symbol(
            static_def.name.clone(),
            SymbolKind::Static,
            ty,
            static_def.is_mut,
            static_def.span.clone(),
        ));

        Ok(())
    }

    /// Check a block
    fn check_block(&mut self, block: &Block) -> TypeResult<Ty> {
        self.symbol_table.enter_scope();

        let mut last_ty = Ty::Unit;
        for (i, stmt) in block.stmts.iter().enumerate() {
            let is_last = i == block.stmts.len() - 1;
            last_ty = self.check_stmt(stmt, is_last)?;
        }

        self.symbol_table.exit_scope();

        Ok(last_ty)
    }

    fn check_stmt(&mut self, stmt: &Stmt, is_last: bool) -> TypeResult<Ty> {
        match stmt {
            Stmt::Let(let_stmt) => {
                let init_ty = let_stmt
                    .init
                    .as_ref()
                    .map(|e| self.check_expr(e))
                    .transpose()?
                    .unwrap_or(Ty::Unit);

                let ty = if let Some(annotated_ty) = &let_stmt.ty {
                    let annotated = self.ast_type_to_ty(annotated_ty);
                    if !init_ty.can_implicitly_convert_to(&annotated) {
                        return Err(TypeError::mismatched_types(
                            annotated.to_string(),
                            init_ty.to_string(),
                            let_stmt.span.clone(),
                            0,
                            0,
                        ));
                    }
                    annotated
                } else {
                    init_ty
                };

                self.symbol_table.insert(make_symbol(
                    let_stmt.name.clone(),
                    SymbolKind::Variable,
                    ty,
                    let_stmt.is_mut,
                    let_stmt.span.clone(),
                ));

                Ok(Ty::Unit)
            }
            Stmt::Expr(expr) => {
                let ty = self.check_expr(expr)?;
                if is_last {
                    Ok(ty)
                } else {
                    Ok(Ty::Unit)
                }
            }
            Stmt::Item(item) => {
                self.check_item(item)?;
                Ok(Ty::Unit)
            }
        }
    }

    /// Check an expression
    fn check_expr(&mut self, expr: &Expr) -> TypeResult<Ty> {
        match expr {
            Expr::Literal(lit) => Ok(self.check_literal(lit)),
            Expr::Ident(name) => self.check_ident(name),
            Expr::Path(path) => self.check_path(path),
            Expr::PathCall(path, args) => self.check_path_call(path, args),
            Expr::Binary(binary) => self.check_binary(binary),
            Expr::Unary(unary) => self.check_unary(unary),
            Expr::Call(call) => self.check_call(call),
            Expr::MethodCall(method_call) => self.check_method_call(method_call),
            Expr::FieldAccess(field_access) => self.check_field_access(field_access),
            Expr::StructInit(struct_init) => self.check_struct_init(struct_init),
            Expr::Block(block) => self.check_block(block),
            Expr::Return(ret) => self.check_return(ret.as_deref()),
            Expr::If(if_expr) => self.check_if(if_expr),
            Expr::While(while_expr) => self.check_while(while_expr),
            Expr::Loop(loop_expr) => self.check_loop(loop_expr),
            Expr::For(for_expr) => self.check_for(for_expr),
            Expr::Break(_) => Ok(Ty::Never),
            Expr::Continue => Ok(Ty::Never),
            Expr::Assign(assign) => self.check_assign(assign),
            Expr::Async(block) => self.check_async(block),
            Expr::Await(expr) => self.check_await(expr),
            Expr::Match(match_expr) => self.check_match(match_expr),
            Expr::ArrayInit(array) => self.check_array_init(array),
            Expr::Index(index) => self.check_index(index),
            Expr::Range(range) => self.check_range(range),
            Expr::Cast(cast) => self.check_cast(cast),
            Expr::Try(expr) => self.check_try(expr),
            Expr::Unsafe(block) => self.check_block(block),
            _ => {
                // Expression type not yet fully implemented
                // Return Error type to allow compilation to continue
                Ok(Ty::Error)
            }
        }
    }

    /// Check a path expression (e.g., Result::Ok, Option::Some)
    fn check_path(&mut self, path: &Path) -> TypeResult<Ty> {
        // For simple paths like Result::Ok, Option::Some, etc.
        // Return a generic type for now
        if path.segments.len() == 1 {
            let name = &path.segments[0].ident;
            return self.check_ident(name);
        }
        
        if path.segments.len() == 2 {
            let type_name = &path.segments[0].ident;
            let variant_name = &path.segments[1].ident;
            
            match (type_name.as_str(), variant_name.as_str()) {
                ("Result", "Ok") | ("Result", "Err") | ("Option", "Some") | ("Option", "None") => {
                    let full_name = format!("{}::{}", type_name, variant_name);
                    if let Some(symbol) = self.symbol_table.lookup(&full_name.into()) {
                        return Ok(symbol.ty.clone());
                    }
                }
                _ => {}
            }
        }
        
        let path_str = path.segments.iter()
            .map(|s| s.ident.as_str())
            .collect::<Vec<_>>()
            .join("::");
        
        if let Some(symbol) = self.symbol_table.lookup(&path_str.into()) {
            return Ok(symbol.ty.clone());
        }
        
        Ok(Ty::Error)
    }

    /// Check a path call expression (e.g., Result::Ok(value), Option::Some(value))
    fn check_path_call(&mut self, path: &Path, args: &[Expr]) -> TypeResult<Ty> {
        if path.segments.len() < 2 {
            for arg in args {
                self.check_expr(arg)?;
            }
            return Ok(Ty::Error);
        }
        
        let type_name = &path.segments[0].ident;
        let variant_name = &path.segments[1].ident;
        
        let arg_tys: Vec<Ty> = args.iter()
            .map(|arg| self.check_expr(arg))
            .collect::<Result<Vec<_>, _>>()?;
        
        // First check for user-defined enum variants
        let full_name = format!("{}::{}", type_name, variant_name);
        if let Some(symbol) = self.symbol_table.lookup(&full_name.into()) {
            if let SymbolKind::Variant { parent_enum, .. } = &symbol.kind {
                if let Some(parent_symbol) = self.symbol_table.lookup(parent_enum) {
                    return Ok(parent_symbol.ty.clone());
                }
            }
        }
        
        // Check if we have a current function return type to infer from
        let expected_ret = self.current_function_return.clone();
        
        // Handle built-in Result and Option types
        match (type_name.as_str(), variant_name.as_str()) {
            ("Result", "Ok") => {
                let ok_ty = arg_tys.first().cloned().unwrap_or(Ty::Error);
                
                // Try to infer Err type from expected return type
                let err_ty = if let Some(ref ret) = expected_ret {
                    if let Ty::Adt(adt) = ret {
                        if adt.name == "Result" && adt.variants.len() == 2 {
                            if let Some(err_variant) = adt.variants.get(1) {
                                if let Some(err_field) = err_variant.fields.first() {
                                    err_field.ty.clone()
                                } else {
                                    Ty::Error
                                }
                            } else {
                                Ty::Error
                            }
                        } else {
                            Ty::Error
                        }
                    } else {
                        Ty::Error
                    }
                } else {
                    Ty::Error
                };
                
                Ok(Ty::Adt(AdtDef {
                    name: "Result".into(),
                    kind: AdtKind::Enum,
                    variants: vec![
                        crate::typeck::ty::VariantDef {
                            name: "Ok".into(),
                            fields: vec![crate::typeck::ty::FieldDef {
                                name: "0".into(),
                                ty: ok_ty,
                            }],
                        },
                        crate::typeck::ty::VariantDef {
                            name: "Err".into(),
                            fields: vec![crate::typeck::ty::FieldDef {
                                name: "0".into(),
                                ty: err_ty,
                            }],
                        },
                    ],
                }))
            }
            ("Result", "Err") => {
                let err_ty = arg_tys.first().cloned().unwrap_or(Ty::Error);
                
                // Try to infer Ok type from expected return type
                let ok_ty = if let Some(ref ret) = expected_ret {
                    if let Ty::Adt(adt) = ret {
                        if adt.name == "Result" && adt.variants.len() == 2 {
                            if let Some(ok_variant) = adt.variants.first() {
                                if let Some(ok_field) = ok_variant.fields.first() {
                                    ok_field.ty.clone()
                                } else {
                                    Ty::Error
                                }
                            } else {
                                Ty::Error
                            }
                        } else {
                            Ty::Error
                        }
                    } else {
                        Ty::Error
                    }
                } else {
                    Ty::Error
                };
                
                Ok(Ty::Adt(AdtDef {
                    name: "Result".into(),
                    kind: AdtKind::Enum,
                    variants: vec![
                        crate::typeck::ty::VariantDef {
                            name: "Ok".into(),
                            fields: vec![crate::typeck::ty::FieldDef {
                                name: "0".into(),
                                ty: ok_ty,
                            }],
                        },
                        crate::typeck::ty::VariantDef {
                            name: "Err".into(),
                            fields: vec![crate::typeck::ty::FieldDef {
                                name: "0".into(),
                                ty: err_ty,
                            }],
                        },
                    ],
                }))
            }
            ("Option", "Some") => {
                let some_ty = arg_tys.first().cloned().unwrap_or(Ty::Error);
                Ok(Ty::Adt(AdtDef {
                    name: "Option".into(),
                    kind: AdtKind::Enum,
                    variants: vec![
                        crate::typeck::ty::VariantDef {
                            name: "Some".into(),
                            fields: vec![crate::typeck::ty::FieldDef {
                                name: "0".into(),
                                ty: some_ty,
                            }],
                        },
                        crate::typeck::ty::VariantDef {
                            name: "None".into(),
                            fields: vec![],
                        },
                    ],
                }))
            }
            ("Option", "None") => {
                Ok(Ty::Adt(AdtDef {
                    name: "Option".into(),
                    kind: AdtKind::Enum,
                    variants: vec![
                        crate::typeck::ty::VariantDef {
                            name: "Some".into(),
                            fields: vec![],
                        },
                        crate::typeck::ty::VariantDef {
                            name: "None".into(),
                            fields: vec![],
                        },
                    ],
                }))
            }
            _ => {
                if let Some(type_symbol) = self.symbol_table.lookup(type_name) {
                    if let Ty::Adt(_adt) = &type_symbol.ty {
                        return Ok(type_symbol.ty.clone());
                    }
                }
                Ok(Ty::Error)
            }
        }
    }

    /// Check a literal
    fn check_literal(&self, lit: &Literal) -> Ty {
        match lit {
            Literal::Integer(_) => Ty::I32, // Default to i32
            Literal::Float(_) => Ty::F64,   // Default to f64
            Literal::String(_) => Ty::String,
            Literal::Char(_) => Ty::Char,
            Literal::Bool(_) => Ty::Bool,
            Literal::Null => Ty::Unit,
        }
    }

    /// Check an identifier
    fn check_ident(&self, name: &Ident) -> TypeResult<Ty> {
        self.symbol_table
            .get_var_type(name)
            .cloned()
            .ok_or_else(|| TypeError::undefined_var(name.as_str(), 0..0, 0, 0))
    }

    /// Check a binary expression
    fn check_binary(&mut self, binary: &BinaryExpr) -> TypeResult<Ty> {
        let left_ty = self.check_expr(&binary.left)?;
        let right_ty = self.check_expr(&binary.right)?;

        match binary.op {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => {
                // Arithmetic operations require numeric types
                if !left_ty.is_numeric() || !right_ty.is_numeric() {
                    return Err(TypeError::InvalidBinaryOp {
                        op: binary.op.to_string(),
                        left: left_ty.to_string(),
                        right: right_ty.to_string(),
                        span: 0..0,
                        line: 0,
                        column: 0,
                    });
                }

                // Result type is the larger of the two
                if left_ty.size() >= right_ty.size() {
                    Ok(left_ty)
                } else {
                    Ok(right_ty)
                }
            }
            BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge => {
                // Comparison operations return bool
                if !left_ty.is_numeric() || !right_ty.is_numeric() {
                    return Err(TypeError::InvalidBinaryOp {
                        op: binary.op.to_string(),
                        left: left_ty.to_string(),
                        right: right_ty.to_string(),
                        span: 0..0,
                        line: 0,
                        column: 0,
                    });
                }
                Ok(Ty::Bool)
            }
            BinaryOp::And | BinaryOp::Or | BinaryOp::LogicalAnd | BinaryOp::LogicalOr => {
                // Logical operations require bool
                if left_ty != Ty::Bool || right_ty != Ty::Bool {
                    return Err(TypeError::InvalidBinaryOp {
                        op: binary.op.to_string(),
                        left: left_ty.to_string(),
                        right: right_ty.to_string(),
                        span: 0..0,
                        line: 0,
                        column: 0,
                    });
                }
                Ok(Ty::Bool)
            }
            BinaryOp::Pipe => {
                // Pipeline operator: left |> right
                // left is the argument, right should be a function call or function identifier
                // The result is the return type of the function

                // For pipeline chains like: a |> b |> c
                // This is parsed as: (a |> b) |> c
                // So left is the result of previous pipe, right is the next function

                match &binary.right.as_ref() {
                    Expr::Ident(func_name) => {
                        // Simple function identifier: x |> func
                        // Look up the function
                        if let Some((params, ret)) = self.symbol_table.get_function(func_name) {
                            if params.len() == 1 {
                                // Check if left type matches parameter type
                                let param_ty = &params[0];
                                if left_ty == *param_ty || left_ty.can_coerce_to(param_ty) {
                                    Ok(ret.clone())
                                } else {
                                    // Allow implicit conversion for numeric types
                                    if left_ty.is_numeric() && param_ty.is_numeric() {
                                        Ok(ret.clone())
                                    } else {
                                        Err(TypeError::MismatchedTypes {
                                            expected: param_ty.to_string(),
                                            found: left_ty.to_string(),
                                            span: 0..0,
                                            line: 0,
                                            column: 0,
                                        })
                                    }
                                }
                            } else {
                                // Function takes more than 1 argument
                                Ok(Ty::Error)
                            }
                        } else {
                            // Not a function in symbol table, just return the type of right
                            let right_ty = self.check_expr(&binary.right)?;
                            Ok(right_ty)
                        }
                    }
                    Expr::Call(call) => {
                        // Function call on right side: x |> func(args)
                        // Check the call with left as first argument
                        let mut call_to_check = call.clone();
                        // Prepend left expression as first argument
                        let left_expr = binary.left.as_ref().clone();
                        call_to_check.args.insert(0, left_expr);
                        self.check_call(&call_to_check)
                    }
                    _ => {
                        // Other expressions - just check both sides
                        let right_ty = self.check_expr(&binary.right)?;
                        Ok(right_ty)
                    }
                }
            }
            BinaryOp::Xor | BinaryOp::Shl | BinaryOp::Shr => {
                // Bitwise operations (excluding And/Or which are handled above as logical)
                // These require integer types
                if !left_ty.is_integer() || !right_ty.is_integer() {
                    return Err(TypeError::InvalidBinaryOp {
                        op: binary.op.to_string(),
                        left: left_ty.to_string(),
                        right: right_ty.to_string(),
                        span: 0..0,
                        line: 0,
                        column: 0,
                    });
                }
                // Result type is the larger of the two
                if left_ty.size() >= right_ty.size() {
                    Ok(left_ty)
                } else {
                    Ok(right_ty)
                }
            }
            BinaryOp::Assign => {
                // Assignment operations return unit type
                // The actual assignment check is done elsewhere
                Ok(Ty::Unit)
            }
        }
    }

    /// Check a unary expression
    fn check_unary(&mut self, unary: &UnaryExpr) -> TypeResult<Ty> {
        let expr_ty = self.check_expr(&unary.expr)?;

        match unary.op {
            UnaryOp::Neg => {
                if !expr_ty.is_numeric() {
                    return Err(TypeError::InvalidUnaryOp {
                        op: unary.op.to_string(),
                        ty: expr_ty.to_string(),
                        span: 0..0,
                        line: 0,
                        column: 0,
                    });
                }
                Ok(expr_ty)
            }
            UnaryOp::Not => {
                // Allow ! on bool or for unwrap on Result/Option types
                if expr_ty == Ty::Bool {
                    Ok(Ty::Bool)
                } else if matches!(expr_ty, Ty::Param(_)) {
                    // For generic type parameters, allow the operation
                    // (they will be resolved during monomorphization)
                    Ok(expr_ty)
                } else if let Ty::Adt(adt) = &expr_ty {
                    // For Result<T, E> or Option<T>, return the inner type
                    if adt.name == "Result" || adt.name == "Option" {
                        if let Some(first_variant) = adt.variants.first() {
                            if let Some(first_field) = first_variant.fields.first() {
                                return Ok(first_field.ty.clone());
                            }
                        }
                    }
                    Err(TypeError::InvalidUnaryOp {
                        op: unary.op.to_string(),
                        ty: expr_ty.to_string(),
                        span: 0..0,
                        line: 0,
                        column: 0,
                    })
                } else {
                    Err(TypeError::InvalidUnaryOp {
                        op: unary.op.to_string(),
                        ty: expr_ty.to_string(),
                        span: 0..0,
                        line: 0,
                        column: 0,
                    })
                }
            }
            UnaryOp::Deref => {
                // Dereference: *expr
                // The expression must be a reference or pointer type
                match expr_ty {
                    Ty::Ref(inner, _) => Ok(*inner),
                    Ty::Ptr(inner, _) => Ok(*inner),
                    _ => Err(TypeError::InvalidUnaryOp {
                        op: "*".to_string(),
                        ty: expr_ty.to_string(),
                        span: 0..0,
                        line: 0,
                        column: 0,
                    }),
                }
            }
            UnaryOp::Ref => {
                // Immutable reference: &expr
                Ok(Ty::Ref(
                    Box::new(expr_ty),
                    crate::typeck::ty::Mutability::Not,
                ))
            }
            UnaryOp::RefMut => {
                // Mutable reference: &mut expr
                Ok(Ty::Ref(
                    Box::new(expr_ty),
                    crate::typeck::ty::Mutability::Mut,
                ))
            }
        }
    }

    /// Check a function call
    fn check_call(&mut self, call: &CallExpr) -> TypeResult<Ty> {
        // Handle method calls: Type::method()
        if let Expr::FieldAccess(field_access) = call.func.as_ref() {
            if let Expr::Ident(type_name) = field_access.expr.as_ref() {
                let method_name = &field_access.field;
                if let Some((params, ret, _)) = self.symbol_table.get_method(type_name, method_name)
                {
                    // Check argument count (excluding receiver for associated functions)
                    if params.len() != call.args.len() {
                        return Err(TypeError::ArgCountMismatch {
                            expected: params.len(),
                            found: call.args.len(),
                            span: 0..0,
                            line: 0,
                            column: 0,
                        });
                    }

                    // Check argument types
                    for (param_ty, arg) in params.iter().zip(call.args.iter()) {
                        let arg_ty = self.check_expr(arg)?;
                        if !arg_ty.can_implicitly_convert_to(param_ty) {
                            return Err(TypeError::mismatched_types(
                                param_ty.to_string(),
                                arg_ty.to_string(),
                                0..0,
                                0,
                                0,
                            ));
                        }
                    }

                    return Ok(ret);
                }
            }
        }

        // Handle regular function calls
        if let Expr::Ident(name) = call.func.as_ref() {
            if let Some((params, ret)) = self.symbol_table.get_function(name) {
                if params.len() != call.args.len() {
                    return Err(TypeError::ArgCountMismatch {
                        expected: params.len(),
                        found: call.args.len(),
                        span: 0..0,
                        line: 0,
                        column: 0,
                    });
                }

                // Check argument types
                for (_i, (param_ty, arg)) in params.iter().zip(call.args.iter()).enumerate() {
                    let arg_ty = self.check_expr(arg)?;
                    if !arg_ty.can_implicitly_convert_to(param_ty) {
                        return Err(TypeError::mismatched_types(
                            param_ty.to_string(),
                            arg_ty.to_string(),
                            0..0,
                            0,
                            0,
                        ));
                    }
                }

                Ok(ret)
            } else {
                Err(TypeError::undefined_function(name.as_str(), 0..0, 0, 0))
            }
        } else {
            // Function pointers and closures are handled by checking the expression type
            let func_ty = self.check_expr(&call.func)?;
            match func_ty {
                Ty::Fn { params, ret } => {
                    if params.len() != call.args.len() {
                        return Err(TypeError::ArgCountMismatch {
                            expected: params.len(),
                            found: call.args.len(),
                            span: 0..0,
                            line: 0,
                            column: 0,
                        });
                    }
                    for (param_ty, arg) in params.iter().zip(call.args.iter()) {
                        let arg_ty = self.check_expr(arg)?;
                        if !arg_ty.can_implicitly_convert_to(param_ty) {
                            return Err(TypeError::mismatched_types(
                                param_ty.to_string(),
                                arg_ty.to_string(),
                                0..0,
                                0,
                                0,
                            ));
                        }
                    }
                    Ok(*ret)
                }
                _ => {
                    // Not a callable type
                    Err(TypeError::InvalidType {
                        name: "non-callable expression".to_string(),
                        span: 0..0,
                        line: 0,
                        column: 0,
                    })
                }
            }
        }
    }

    /// Check a method call (obj.method(args))
    fn check_method_call(&mut self, method_call: &MethodCallExpr) -> TypeResult<Ty> {
        let receiver_ty = self.check_expr(&method_call.receiver)?;
        let type_name = self.ty_to_ident(&receiver_ty);
        let method_name = &method_call.method;

        // Look up the method
        if let Some((params, ret, _receiver_kind)) =
            self.symbol_table.get_method(&type_name, method_name)
        {
            // Check argument count (excluding receiver)
            if params.len() != method_call.args.len() {
                return Err(TypeError::ArgCountMismatch {
                    expected: params.len(),
                    found: method_call.args.len(),
                    span: 0..0,
                    line: 0,
                    column: 0,
                });
            }

            // Check argument types
            for (param_ty, arg) in params.iter().zip(method_call.args.iter()) {
                let arg_ty = self.check_expr(arg)?;
                if !arg_ty.can_implicitly_convert_to(param_ty) {
                    return Err(TypeError::mismatched_types(
                        param_ty.to_string(),
                        arg_ty.to_string(),
                        0..0,
                        0,
                        0,
                    ));
                }
            }

            Ok(ret)
        } else {
            Err(TypeError::UndefinedMethod {
                type_name: type_name.to_string(),
                method_name: method_name.to_string(),
                span: 0..0,
                line: 0,
                column: 0,
            })
        }
    }

    /// Check field access (obj.field)
    fn check_field_access(&mut self, field_access: &FieldAccessExpr) -> TypeResult<Ty> {
        // First check if this is an enum variant access (e.g., GpioError::InitFailed)
        if let Expr::Ident(type_name) = field_access.expr.as_ref() {
            // Check if this is an enum variant
            let variant_name = format!("{}::{}", type_name, field_access.field);
            if let Some(symbol) = self.symbol_table.lookup(&variant_name.into()) {
                if let SymbolKind::Variant { parent_enum, .. } = &symbol.kind {
                    // Return the parent enum type
                    if let Some(parent_symbol) = self.symbol_table.lookup(parent_enum) {
                        return Ok(parent_symbol.ty.clone());
                    }
                }
            }
            
            // Check if this is a type (for associated constants or other type members)
            if let Some(symbol) = self.symbol_table.lookup(type_name) {
                if matches!(symbol.kind, SymbolKind::Type) {
                    // This is a type access, check for variant
                    if let Ty::Adt(adt) = &symbol.ty {
                        if adt.kind == AdtKind::Enum {
                            // Look for the variant
                            for variant in &adt.variants {
                                if variant.name == field_access.field.as_str() {
                                    return Ok(symbol.ty.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Regular field access
        let expr_ty = self.check_expr(&field_access.expr)?;
        let type_name = self.ty_to_ident(&expr_ty);
        let field_name = &field_access.field;

        // Look up the field
        if let Some(field_ty) = self.symbol_table.get_field_type(&type_name, field_name) {
            Ok(field_ty)
        } else {
            Err(TypeError::UndefinedField {
                type_name: type_name.to_string(),
                field_name: field_name.to_string(),
                span: 0..0,
                line: 0,
                column: 0,
            })
        }
    }

    /// Check struct initialization (Struct { field: value, ... })
    fn check_struct_init(&mut self, struct_init: &StructInitExpr) -> TypeResult<Ty> {
        let struct_name = struct_init.path.segments.last()
            .map(|s| &s.ident)
            .ok_or_else(|| TypeError::InvalidType {
                name: "empty path".to_string(),
                span: 0..0,
                line: 0,
                column: 0,
            })?;

        // Get struct fields
        let fields = self
            .symbol_table
            .get_struct_fields(struct_name)
            .ok_or_else(|| TypeError::UndefinedType {
                name: struct_name.to_string(),
                span: 0..0,
                line: 0,
                column: 0,
            })?;

        // Create a map of field names to types
        let field_map: std::collections::HashMap<_, _> = fields.iter().cloned().collect();

        // Check each field initialization
        for (field_name, value) in &struct_init.fields {
            let expected_ty =
                field_map
                    .get(field_name)
                    .ok_or_else(|| TypeError::UndefinedField {
                        type_name: struct_name.to_string(),
                        field_name: field_name.to_string(),
                        span: 0..0,
                        line: 0,
                        column: 0,
                    })?;

            let value_ty = self.check_expr(value)?;
            if !value_ty.can_implicitly_convert_to(expected_ty) {
                return Err(TypeError::mismatched_types(
                    expected_ty.to_string(),
                    value_ty.to_string(),
                    0..0,
                    0,
                    0,
                ));
            }
        }

        // Return the struct type
        Ok(Ty::Adt(crate::typeck::ty::AdtDef {
            name: struct_name.clone(),
            kind: crate::typeck::ty::AdtKind::Struct,
            variants: vec![crate::typeck::ty::VariantDef {
                name: struct_name.clone(),
                fields: fields
                    .iter()
                    .map(|(name, ty)| crate::typeck::ty::FieldDef {
                        name: name.clone(),
                        ty: ty.clone(),
                    })
                    .collect(),
            }],
        }))
    }

    fn check_return(&mut self, expr: Option<&Expr>) -> TypeResult<Ty> {
        let ret_ty = self.current_function_return.clone().unwrap_or(Ty::Unit);

        if let Some(e) = expr {
            let expr_ty = self.check_expr(e)?;
            if !expr_ty.can_implicitly_convert_to(&ret_ty) {
                return Err(TypeError::ReturnTypeMismatch {
                    expected: ret_ty.to_string(),
                    found: expr_ty.to_string(),
                    span: 0..0,
                    line: 0,
                    column: 0,
                });
            }
        }

        Ok(Ty::Never)
    }

    /// Check an if expression
    fn check_if(&mut self, if_expr: &IfExpr) -> TypeResult<Ty> {
        let cond_ty = self.check_expr(&if_expr.cond)?;
        if cond_ty != Ty::Bool {
            return Err(TypeError::mismatched_types(
                "bool".to_string(),
                cond_ty.to_string(),
                0..0,
                0,
                0,
            ));
        }

        let then_ty = self.check_block(&if_expr.then_branch)?;

        if let Some(else_branch) = &if_expr.else_branch {
            let else_ty = match else_branch.as_ref() {
                Expr::Block(block) => self.check_block(block)?,
                Expr::If(if_expr) => self.check_if(if_expr)?,
                _ => self.check_expr(else_branch)?,
            };

            // Both branches should have compatible types
            if then_ty != else_ty {
                // Check for implicit conversion from then_ty to else_ty or vice versa
                if then_ty.can_implicitly_convert_to(&else_ty) {
                    Ok(else_ty)
                } else if else_ty.can_implicitly_convert_to(&then_ty) {
                    Ok(then_ty)
                } else {
                    // Types are incompatible
                    Err(TypeError::mismatched_types(
                        then_ty.to_string(),
                        else_ty.to_string(),
                        0..0,
                        0,
                        0,
                    ))
                }
            } else {
                Ok(then_ty)
            }
        } else {
            Ok(Ty::Unit)
        }
    }

    /// Check a while expression
    fn check_while(&mut self, while_expr: &WhileExpr) -> TypeResult<Ty> {
        let cond_ty = self.check_expr(&while_expr.cond)?;
        if cond_ty != Ty::Bool {
            return Err(TypeError::mismatched_types(
                "bool".to_string(),
                cond_ty.to_string(),
                0..0,
                0,
                0,
            ));
        }

        self.check_block(&while_expr.body)?;
        Ok(Ty::Unit)
    }

    /// Check a loop expression
    fn check_loop(&mut self, loop_expr: &LoopExpr) -> TypeResult<Ty> {
        self.check_block(&loop_expr.body)?;
        Ok(Ty::Never) // Loop never returns normally
    }

    /// Check a for expression
    fn check_for(&mut self, for_expr: &ForExpr) -> TypeResult<Ty> {
        self.symbol_table.enter_scope();

        // Check the iterator expression
        let iter_ty = self.check_expr(&for_expr.expr)?;

        // Determine the element type from the iterator
        let elem_ty = self.get_iterator_element_type(&iter_ty);

        // Extract pattern names and bind them with appropriate types
        self.bind_pattern(&for_expr.pattern, &elem_ty)?;

        self.check_block(&for_expr.body)?;
        self.symbol_table.exit_scope();

        Ok(Ty::Unit)
    }

    /// Get the element type from an iterator type
    fn get_iterator_element_type(&self, iter_ty: &Ty) -> Ty {
        match iter_ty {
            // Array iterator: [T; n] or &[T; n] -> T
            Ty::Array(elem, _) => *elem.clone(),
            // Slice iterator: &[T] or [T] -> T
            Ty::Slice(elem) => *elem.clone(),
            // Range iterator: Range<T> -> T
            Ty::Adt(adt) if adt.name == "Range" => {
                // Extract T from Range<T>
                adt.variants
                    .first()
                    .and_then(|v| v.fields.first())
                    .map(|f| f.ty.clone())
                    .unwrap_or(Ty::I32)
            }
            // Vec iterator
            Ty::Adt(adt) if adt.name == "Vec" => adt
                .variants
                .first()
                .and_then(|v| v.fields.first())
                .map(|f| f.ty.clone())
                .unwrap_or(Ty::Error),
            // Default to i32 for integer ranges
            _ if iter_ty.is_integer() => iter_ty.clone(),
            // Fallback
            _ => Ty::Error,
        }
    }

    fn bind_pattern(&mut self, pattern: &Pattern, ty: &Ty) -> TypeResult<()> {
        match pattern {
            Pattern::Ident(name) => {
                self.symbol_table.insert(make_symbol(
                    name.clone(),
                    SymbolKind::Variable,
                    ty.clone(),
                    false,
                    0..0,
                ));
                Ok(())
            }
            Pattern::Wildcard => {
                Ok(())
            }
            Pattern::Mut(inner) => {
                if let Pattern::Ident(name) = inner.as_ref() {
                    self.symbol_table.insert(make_symbol(
                        name.clone(),
                        SymbolKind::Variable,
                        ty.clone(),
                        true,
                        0..0,
                    ));
                    Ok(())
                } else {
                    self.bind_pattern(inner, ty)
                }
            }
            Pattern::Ref(inner) => {
                let ref_ty = Ty::Ref(Box::new(ty.clone()), Mutability::Not);
                self.bind_pattern(inner, &ref_ty)
            }
            Pattern::Tuple(patterns) => {
                if let Ty::Tuple(elem_tys) = ty {
                    if patterns.len() != elem_tys.len() {
                        return Err(TypeError::MismatchedTypes {
                            expected: format!("tuple with {} elements", elem_tys.len()),
                            found: format!("tuple with {} elements", patterns.len()),
                            span: 0..0,
                            line: 0,
                            column: 0,
                        });
                    }
                    for (pat, elem_ty) in patterns.iter().zip(elem_tys.iter()) {
                        self.bind_pattern(pat, elem_ty)?;
                    }
                    Ok(())
                } else {
                    Err(TypeError::MismatchedTypes {
                        expected: "tuple type".to_string(),
                        found: ty.to_string(),
                        span: 0..0,
                        line: 0,
                        column: 0,
                    })
                }
            }
            Pattern::Binding(name, inner) => {
                self.symbol_table.insert(make_symbol(
                    name.clone(),
                    SymbolKind::Variable,
                    ty.clone(),
                    false,
                    0..0,
                ));
                self.bind_pattern(inner, ty)
            }
            _ => {
                Ok(())
            }
        }
    }

    /// Check an assignment
    fn check_assign(&mut self, assign: &AssignExpr) -> TypeResult<Ty> {
        // Check that left side is mutable
        if !self.is_mutable_expr(&assign.left) {
            let name = self.expr_to_ident(&assign.left);
            return Err(TypeError::AssignToImmutable {
                name: name.to_string(),
                span: 0..0,
                line: 0,
                column: 0,
            });
        }

        let left_ty = self.check_expr(&assign.left)?;
        let right_ty = self.check_expr(&assign.right)?;

        if !right_ty.can_implicitly_convert_to(&left_ty) {
            return Err(TypeError::mismatched_types(
                left_ty.to_string(),
                right_ty.to_string(),
                0..0,
                0,
                0,
            ));
        }

        Ok(Ty::Unit)
    }

    /// Check an async block
    fn check_async(&mut self, block: &Block) -> TypeResult<Ty> {
        // Enter async context
        let was_in_async = self.in_async_context;
        self.in_async_context = true;

        // Check the block
        let block_ty = self.check_block(block)?;

        // Restore previous async context
        self.in_async_context = was_in_async;

        // Return Future<T> type
        Ok(Ty::Future(Box::new(block_ty)))
    }

    /// Check an await expression
    fn check_await(&mut self, expr: &Expr) -> TypeResult<Ty> {
        // Check that we're in an async context
        if !self.in_async_context {
            return Err(TypeError::AwaitOutsideAsync {
                span: 0..0,
                line: 0,
                column: 0,
            });
        }

        // Check the expression being awaited
        let expr_ty = self.check_expr(expr)?;

        // Extract the inner type from Future<T>
        match expr_ty {
            Ty::Future(inner) => Ok(*inner),
            _ => Err(TypeError::NotAwaitable {
                ty: expr_ty.to_string(),
                span: 0..0,
                line: 0,
                column: 0,
            }),
        }
    }

    /// Check if an expression is mutable (can be assigned to)
    fn is_mutable_expr(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Ident(ident) => {
                // Look up the symbol and check if it's mutable
                self.symbol_table
                    .lookup(ident)
                    .map(|s| s.is_mut)
                    .unwrap_or(false)
            }
            Expr::FieldAccess(field_access) => {
                // Field access is mutable if the base is mutable
                self.is_mutable_expr(&field_access.expr)
            }
            Expr::Index(index) => {
                // Index access is mutable if the base is mutable
                self.is_mutable_expr(&index.expr)
            }
            Expr::Unary(unary) => {
                // Dereference is mutable if the pointer is mutable
                matches!(unary.op, UnaryOp::Deref)
            }
            _ => false,
        }
    }

    /// Convert an expression to an identifier (for error messages)
    fn expr_to_ident(&self, expr: &Expr) -> Ident {
        match expr {
            Expr::Ident(ident) => ident.clone(),
            Expr::FieldAccess(field_access) => {
                // Get the base identifier and append field name
                let base = self.expr_to_ident(&field_access.expr);
                format!("{}.{}", base, field_access.field).into()
            }
            _ => "<expression>".into(),
        }
    }

    /// Convert AST type to internal type
    fn ast_type_to_ty(&self, ty: &Type) -> Ty {
        match ty {
            Type::Unit => Ty::Unit,
            Type::Never => Ty::Never,
            Type::Path(path) => {
                self.resolve_type_path(path)
            }
            Type::Ref(inner, is_mut) => {
                let inner_ty = self.ast_type_to_ty(inner);
                Ty::Ref(
                    Box::new(inner_ty),
                    if *is_mut {
                        Mutability::Mut
                    } else {
                        Mutability::Not
                    },
                )
            }
            Type::Ptr(inner, is_mut) => {
                let inner_ty = self.ast_type_to_ty(inner);
                Ty::Ptr(
                    Box::new(inner_ty),
                    if *is_mut {
                        Mutability::Mut
                    } else {
                        Mutability::Not
                    },
                )
            }
            Type::Array(inner, size) => {
                let inner_ty = self.ast_type_to_ty(inner);
                Ty::Array(Box::new(inner_ty), size.unwrap_or(0))
            }
            Type::Slice(inner) => {
                let inner_ty = self.ast_type_to_ty(inner);
                Ty::Slice(Box::new(inner_ty))
            }
            Type::Tuple(types) => {
                let elem_tys: Vec<Ty> = types.iter().map(|t| self.ast_type_to_ty(t)).collect();
                Ty::Tuple(elem_tys)
            }
            Type::Function(func_ty) => {
                let params: Vec<Ty> = func_ty
                    .params
                    .iter()
                    .map(|t| self.ast_type_to_ty(t))
                    .collect();
                let ret = self.ast_type_to_ty(&func_ty.return_type);
                Ty::Fn {
                    params,
                    ret: Box::new(ret),
                }
            }
            Type::Generic(base, args) => {
                // Extract the base type name and resolve with generics
                if let Type::Path(path) = base.as_ref() {
                    if let Some(last_segment) = path.segments.last() {
                        return self.resolve_generic_type(&last_segment.ident, args);
                    }
                }
                self.ast_type_to_ty(base)
            }
            _ => Ty::Error,
        }
    }
    
    fn resolve_type_path(&self, path: &Path) -> Ty {
        if path.segments.is_empty() {
            return Ty::Error;
        }
        
        let last_segment = &path.segments[path.segments.len() - 1];
        let name = &last_segment.ident;
        
        if !last_segment.generics.is_empty() {
            return self.resolve_generic_type(name, &last_segment.generics);
        }
        
        if path.segments.len() == 1 {
            if let Some(symbol) = self.symbol_table.lookup(name) {
                match &symbol.kind {
                    SymbolKind::Type | SymbolKind::Struct { .. } | SymbolKind::Variant { .. } => {
                        return symbol.ty.clone();
                    }
                    _ => {}
                }
            }
            
            match name.as_str() {
                "bool" => Ty::Bool,
                "i8" => Ty::I8,
                "i16" => Ty::I16,
                "i32" => Ty::I32,
                "i64" => Ty::I64,
                "i128" => Ty::I128,
                "isize" => Ty::Isize,
                "u8" => Ty::U8,
                "u16" => Ty::U16,
                "u32" => Ty::U32,
                "u64" => Ty::U64,
                "u128" => Ty::U128,
                "usize" => Ty::Usize,
                "f32" => Ty::F32,
                "f64" => Ty::F64,
                "char" => Ty::Char,
                "str" => Ty::Str,
                "String" => Ty::String,
                _ => Ty::Error,
            }
        } else if path.segments.len() == 2 {
            let module_name = &path.segments[0].ident;
            match module_name.as_str() {
                "core" | "std" => {
                    if let Some(symbol) = self.symbol_table.lookup(name) {
                        match &symbol.kind {
                            SymbolKind::Type | SymbolKind::Struct { .. } | SymbolKind::Variant { .. } => {
                                return symbol.ty.clone();
                            }
                            _ => {}
                        }
                    }
                    Ty::Error
                }
                _ => {
                    if let Some(symbol) = self.symbol_table.lookup(name) {
                        match &symbol.kind {
                            SymbolKind::Type | SymbolKind::Struct { .. } | SymbolKind::Variant { .. } => {
                                return symbol.ty.clone();
                            }
                            _ => {}
                        }
                    }
                    Ty::Error
                }
            }
        } else {
            if let Some(symbol) = self.symbol_table.lookup(name) {
                match &symbol.kind {
                    SymbolKind::Type | SymbolKind::Struct { .. } | SymbolKind::Variant { .. } => {
                        return symbol.ty.clone();
                    }
                    _ => {}
                }
            }
            Ty::Error
        }
    }
    
    fn resolve_generic_type(&self, name: &Ident, generics: &[Type]) -> Ty {
        let arg_tys: Vec<Ty> = generics.iter().map(|t| self.ast_type_to_ty(t)).collect();
        
        match name.as_str() {
            "Result" => {
                let ok_ty = arg_tys.first().cloned().unwrap_or(Ty::Error);
                let err_ty = arg_tys.get(1).cloned().unwrap_or(Ty::Error);
                Ty::Adt(AdtDef {
                    name: "Result".into(),
                    kind: AdtKind::Enum,
                    variants: vec![
                        crate::typeck::ty::VariantDef {
                            name: "Ok".into(),
                            fields: vec![crate::typeck::ty::FieldDef {
                                name: "0".into(),
                                ty: ok_ty,
                            }],
                        },
                        crate::typeck::ty::VariantDef {
                            name: "Err".into(),
                            fields: vec![crate::typeck::ty::FieldDef {
                                name: "0".into(),
                                ty: err_ty,
                            }],
                        },
                    ],
                })
            }
            "Option" => {
                let some_ty = arg_tys.first().cloned().unwrap_or(Ty::Error);
                Ty::Adt(AdtDef {
                    name: "Option".into(),
                    kind: AdtKind::Enum,
                    variants: vec![
                        crate::typeck::ty::VariantDef {
                            name: "Some".into(),
                            fields: vec![crate::typeck::ty::FieldDef {
                                name: "0".into(),
                                ty: some_ty.clone(),
                            }],
                        },
                        crate::typeck::ty::VariantDef {
                            name: "None".into(),
                            fields: vec![],
                        },
                    ],
                })
            }
            "Vec" => {
                let elem_ty = arg_tys.first().cloned().unwrap_or(Ty::Error);
                Ty::Adt(AdtDef {
                    name: "Vec".into(),
                    kind: AdtKind::Struct,
                    variants: vec![crate::typeck::ty::VariantDef {
                        name: "Vec".into(),
                        fields: vec![crate::typeck::ty::FieldDef {
                            name: "0".into(),
                            ty: Ty::Slice(Box::new(elem_ty)),
                        }],
                    }],
                })
            }
            _ => {
                // Check if it's a user-defined generic type
                if let Some(symbol) = self.symbol_table.lookup(name) {
                    match &symbol.kind {
                        SymbolKind::Type | SymbolKind::Struct { .. } | SymbolKind::Variant { .. } => {
                            return symbol.ty.clone();
                        }
                        _ => {}
                    }
                }
                Ty::Error
            }
        }
    }

    fn check_match(&mut self, match_expr: &MatchExpr) -> TypeResult<Ty> {
        let scrutinee_ty = self.check_expr(&match_expr.expr)?;

        let mut arm_tys = Vec::new();
        for arm in &match_expr.arms {
            self.symbol_table.enter_scope();
            self.check_pattern(&arm.pattern, &scrutinee_ty)?;
            let arm_ty = self.check_expr(&arm.body)?;
            arm_tys.push(arm_ty);
            self.symbol_table.exit_scope();
        }

        if let Some(first_ty) = arm_tys.first() {
            for (_i, arm_ty) in arm_tys.iter().enumerate().skip(1) {
                if !arm_ty.can_implicitly_convert_to(first_ty) {
                    return Err(TypeError::MismatchedTypes {
                        expected: first_ty.to_string(),
                        found: arm_ty.to_string(),
                        span: 0..0,
                        line: 0,
                        column: 0,
                    });
                }
            }
            Ok(first_ty.clone())
        } else {
            Ok(Ty::Unit)
        }
    }

    fn check_pattern(&mut self, pattern: &Pattern, ty: &Ty) -> TypeResult<()> {
        match pattern {
            Pattern::Ident(name) => {
                self.symbol_table.insert(make_symbol(
                    name.clone(),
                    SymbolKind::Variable,
                    ty.clone(),
                    false,
                    0..0,
                ));
                Ok(())
            }
            Pattern::Wildcard => Ok(()),
            Pattern::Tuple(patterns) => {
                if let Ty::Tuple(elem_tys) = ty {
                    for (pat, elem_ty) in patterns.iter().zip(elem_tys.iter()) {
                        self.check_pattern(pat, elem_ty)?;
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Check an array initialization expression
    fn check_array_init(&mut self, array: &ArrayInitExpr) -> TypeResult<Ty> {
        let mut elem_ty = Ty::Error;
        for elem in &array.elements {
            let ty = self.check_expr(elem)?;
            if elem_ty == Ty::Error {
                elem_ty = ty;
            }
        }
        Ok(Ty::Array(Box::new(elem_ty), array.elements.len()))
    }

    /// Check an index expression
    fn check_index(&mut self, index: &IndexExpr) -> TypeResult<Ty> {
        let base_ty = self.check_expr(&index.expr)?;
        let index_ty = self.check_expr(&index.index)?;

        // Index should be integer
        if !index_ty.is_integer() {
            return Err(TypeError::mismatched_types(
                "integer".to_string(),
                index_ty.to_string(),
                0..0,
                0,
                0,
            ));
        }

        // Return element type
        match base_ty {
            Ty::Array(elem, _) => Ok(*elem),
            Ty::Slice(elem) => Ok(*elem),
            _ => Ok(Ty::Error),
        }
    }

    /// Check a range expression
    fn check_range(&mut self, range: &RangeExpr) -> TypeResult<Ty> {
        let start_ty = range
            .start
            .as_ref()
            .map(|e| self.check_expr(e))
            .transpose()?;
        let end_ty = range
            .end
            .as_ref()
            .map(|e| self.check_expr(e))
            .transpose()?;

        // Range type is typically the same as start/end type
        Ok(start_ty.or(end_ty).unwrap_or(Ty::I32))
    }

    /// Check a cast expression
    fn check_cast(&mut self, cast: &CastExpr) -> TypeResult<Ty> {
        let expr_ty = self.check_expr(&cast.expr)?;
        let target_ty = self.ast_type_to_ty(&cast.ty);

        // Check if cast is valid
        if expr_ty.is_integer() && target_ty.is_integer() {
            Ok(target_ty)
        } else {
            Ok(target_ty)
        }
    }

    /// Check a try expression (? operator)
    fn check_try(&mut self, expr: &Expr) -> TypeResult<Ty> {
        let expr_ty = self.check_expr(expr)?;
        
        // Handle Result<T, E> and Option<T> types
        if let Ty::Adt(adt) = &expr_ty {
            if adt.name == "Result" || adt.name == "Option" {
                // Return the Ok/Some type
                if let Some(first_variant) = adt.variants.first() {
                    if let Some(first_field) = first_variant.fields.first() {
                        return Ok(first_field.ty.clone());
                    }
                }
            }
        }
        
        // For generic type parameters, return them as-is
        // (they will be resolved during monomorphization)
        if matches!(expr_ty, Ty::Param(_)) {
            return Ok(expr_ty);
        }
        
        // For error recovery, return the type as-is
        Ok(expr_ty)
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Type check a module
pub fn type_check(module: &Module) -> TypeResult<()> {
    let mut checker = TypeChecker::new();
    checker.check_module(module)
}
