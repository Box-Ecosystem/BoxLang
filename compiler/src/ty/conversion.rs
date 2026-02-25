//! Type conversion utilities
//!
//! Provides unified type conversion between AST types and internal type representations.

use crate::ast::{self, Type, Path};
use crate::typeck::ty::{Ty, AdtDef, AdtKind, VariantDef, FieldDef, Mutability};

/// Trait for converting AST types to internal type representations
pub trait TypeConverter {
    /// Convert an AST type to internal Ty
    fn ast_type_to_ty(&self, ty: &Type) -> Ty;

    /// Convert a path type to Ty
    fn path_to_ty(&self, path: &Path) -> Ty;

    /// Check if a type name is a primitive type
    fn is_primitive_type(&self, name: &str) -> bool;

    /// Convert a primitive type name to Ty
    fn primitive_to_ty(&self, name: &str) -> Option<Ty>;
}

/// Standard type converter implementation
pub struct StandardTypeConverter;

impl StandardTypeConverter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for StandardTypeConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeConverter for StandardTypeConverter {
    fn ast_type_to_ty(&self, ty: &Type) -> Ty {
        match ty {
            Type::Unit => Ty::Unit,
            Type::Never => Ty::Never,
            Type::Path(path) => self.path_to_ty(path),
            Type::Ref(inner, is_mut) => {
                let inner_ty = self.ast_type_to_ty(inner);
                let mutability = if *is_mut {
                    Mutability::Mut
                } else {
                    Mutability::Not
                };
                Ty::Ref(Box::new(inner_ty), mutability)
            }
            Type::Ptr(inner, is_mut) => {
                let inner_ty = self.ast_type_to_ty(inner);
                let mutability = if *is_mut {
                    Mutability::Mut
                } else {
                    Mutability::Not
                };
                Ty::Ptr(Box::new(inner_ty), mutability)
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
                let tys: Vec<Ty> = types.iter().map(|t| self.ast_type_to_ty(t)).collect();
                Ty::Tuple(tys)
            }
            Type::Function(func_type) => {
                let param_tys: Vec<Ty> = func_type.params.iter().map(|p| self.ast_type_to_ty(p)).collect();
                let ret_ty = self.ast_type_to_ty(&func_type.return_type);
                Ty::Fn {
                    params: param_tys,
                    ret: Box::new(ret_ty),
                }
            }
            Type::Generic(base, args) => {
                let base_ty = self.ast_type_to_ty(base);
                if args.is_empty() {
                    return base_ty;
                }
                
                let arg_tys: Vec<Ty> = args.iter().map(|t| self.ast_type_to_ty(t)).collect();
                
                if let Ty::Adt(adt) = &base_ty {
                    if adt.variants.is_empty() {
                        let name = adt.name.clone();
                        match name.as_str() {
                            "Result" => {
                                let ok_ty = arg_tys.first().cloned().unwrap_or(Ty::Error);
                                let err_ty = arg_tys.get(1).cloned().unwrap_or(Ty::Error);
                                return Ty::Adt(AdtDef {
                                    name: "Result".into(),
                                    kind: AdtKind::Enum,
                                    variants: vec![
                                        VariantDef {
                                            name: "Ok".into(),
                                            fields: vec![FieldDef {
                                                name: "0".into(),
                                                ty: ok_ty,
                                            }],
                                        },
                                        VariantDef {
                                            name: "Err".into(),
                                            fields: vec![FieldDef {
                                                name: "0".into(),
                                                ty: err_ty,
                                            }],
                                        },
                                    ],
                                });
                            }
                            "Option" => {
                                let some_ty = arg_tys.first().cloned().unwrap_or(Ty::Error);
                                return Ty::Adt(AdtDef {
                                    name: "Option".into(),
                                    kind: AdtKind::Enum,
                                    variants: vec![
                                        VariantDef {
                                            name: "Some".into(),
                                            fields: vec![FieldDef {
                                                name: "0".into(),
                                                ty: some_ty,
                                            }],
                                        },
                                        VariantDef {
                                            name: "None".into(),
                                            fields: vec![],
                                        },
                                    ],
                                });
                            }
                            "Vec" => {
                                let elem_ty = arg_tys.first().cloned().unwrap_or(Ty::Error);
                                return Ty::Adt(AdtDef {
                                    name: "Vec".into(),
                                    kind: AdtKind::Struct,
                                    variants: vec![VariantDef {
                                        name: "Vec".into(),
                                        fields: vec![FieldDef {
                                            name: "0".into(),
                                            ty: Ty::Slice(Box::new(elem_ty)),
                                        }],
                                    }],
                                });
                            }
                            _ => {
                                return Ty::Adt(AdtDef {
                                    name: adt.name.clone(),
                                    kind: adt.kind,
                                    variants: vec![VariantDef {
                                        name: adt.name.clone(),
                                        fields: arg_tys.iter().enumerate().map(|(i, ty)| FieldDef {
                                            name: format!("_{}", i).into(),
                                            ty: ty.clone(),
                                        }).collect(),
                                    }],
                                });
                            }
                        }
                    }
                }
                base_ty
            }
            Type::ImplTrait(traits) => {
                if let Some(first_trait) = traits.first() {
                    Ty::Named(first_trait.path.segments.first()
                        .map(|s| s.ident.clone())
                        .unwrap_or_else(|| "ImplTrait".into()))
                } else {
                    Ty::Named("ImplTrait".into())
                }
            }
            Type::DynTrait(traits) => {
                if let Some(first_trait) = traits.first() {
                    Ty::Named(first_trait.path.segments.first()
                        .map(|s| format!("dyn_{}", s.ident))
                        .unwrap_or_else(|| "DynTrait".into())
                        .into())
                } else {
                    Ty::Named("DynTrait".into())
                }
            }
        }
    }

    fn path_to_ty(&self, path: &Path) -> Ty {
        if path.segments.len() == 1 {
            let name = path.segments[0].ident.as_str();
            if let Some(primitive) = self.primitive_to_ty(name) {
                return primitive;
            }
        }

        // For user-defined types, create an ADT
        let path_str = path
            .segments
            .iter()
            .map(|s| s.ident.as_str())
            .collect::<Vec<_>>()
            .join("::");
        
        Ty::Adt(AdtDef {
            name: path_str.into(),
            kind: AdtKind::Struct,
            variants: Vec::new(),
        })
    }

    fn is_primitive_type(&self, name: &str) -> bool {
        matches!(
            name,
            "bool"
                | "i8"
                | "i16"
                | "i32"
                | "i64"
                | "i128"
                | "isize"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "u128"
                | "usize"
                | "f32"
                | "f64"
                | "char"
                | "str"
                | "String"
        )
    }

    fn primitive_to_ty(&self, name: &str) -> Option<Ty> {
        match name {
            "bool" => Some(Ty::Bool),
            "i8" => Some(Ty::I8),
            "i16" => Some(Ty::I16),
            "i32" => Some(Ty::I32),
            "i64" => Some(Ty::I64),
            "i128" => Some(Ty::I128),
            "isize" => Some(Ty::Isize),
            "u8" => Some(Ty::U8),
            "u16" => Some(Ty::U16),
            "u32" => Some(Ty::U32),
            "u64" => Some(Ty::U64),
            "u128" => Some(Ty::U128),
            "usize" => Some(Ty::Usize),
            "f32" => Some(Ty::F32),
            "f64" => Some(Ty::F64),
            "char" => Some(Ty::Char),
            "str" => Some(Ty::Str),
            "String" => Some(Ty::String),
            _ => None,
        }
    }
}

/// Trait for converting types to C type strings
pub trait ToCType {
    /// Convert to C type string
    fn to_c_type(&self) -> String;

    /// Convert to C type for function parameters (arrays become pointers)
    fn to_c_param_type(&self) -> String;
}

impl ToCType for Type {
    fn to_c_type(&self) -> String {
        match self {
            Type::Unit => "void".to_string(),
            Type::Never => "void".to_string(),
            Type::Path(path) => {
                if path.segments.len() == 1 {
                    match path.segments[0].ident.as_str() {
                        "bool" => "int",
                        "i8" => "int8_t",
                        "i16" => "int16_t",
                        "i32" => "int32_t",
                        "i64" => "int64_t",
                        "u8" => "uint8_t",
                        "u16" => "uint16_t",
                        "u32" => "uint32_t",
                        "u64" => "uint64_t",
                        "f32" => "float",
                        "f64" => "double",
                        "char" => "char",
                        "str" => "const char*",
                        "String" => "char*",
                        _ => "void",
                    }
                    .to_string()
                } else {
                    "void".to_string()
                }
            }
            Type::Ref(inner, _) => inner.to_c_type(),
            Type::Ptr(inner, _) => format!("{}*", inner.to_c_type()),
            Type::Array(inner, size) => {
                let inner_c = inner.to_c_type();
                match size {
                    Some(n) => format!("{}[{}]", inner_c, n),
                    None => format!("{}*", inner_c),
                }
            }
            Type::Slice(inner) => {
                let inner_c = inner.to_c_type();
                format!("{}*", inner_c)
            }
            Type::Tuple(_) => "void".to_string(), // Tuples not directly supported in C
            Type::Function(_) => "void".to_string(), // Function types need special handling
            Type::Generic(base, _) => base.to_c_type(),
            Type::ImplTrait(_) => "void".to_string(),
            Type::DynTrait(_) => "void".to_string(),
        }
    }

    fn to_c_param_type(&self) -> String {
        match self {
            Type::Array(inner, _) | Type::Slice(inner) => {
                let inner_c = inner.to_c_type();
                format!("{}*", inner_c)
            }
            _ => self.to_c_type(),
        }
    }
}

impl ToCType for Ty {
    fn to_c_type(&self) -> String {
        match self {
            Ty::Unit => "void".to_string(),
            Ty::Never => "void".to_string(),
            Ty::Bool => "int".to_string(),
            Ty::I8 => "int8_t".to_string(),
            Ty::I16 => "int16_t".to_string(),
            Ty::I32 => "int32_t".to_string(),
            Ty::I64 => "int64_t".to_string(),
            Ty::I128 => "__int128".to_string(),
            Ty::Isize => "intptr_t".to_string(),
            Ty::U8 => "uint8_t".to_string(),
            Ty::U16 => "uint16_t".to_string(),
            Ty::U32 => "uint32_t".to_string(),
            Ty::U64 => "uint64_t".to_string(),
            Ty::U128 => "unsigned __int128".to_string(),
            Ty::Usize => "uintptr_t".to_string(),
            Ty::F32 => "float".to_string(),
            Ty::F64 => "double".to_string(),
            Ty::Char => "char".to_string(),
            Ty::Str => "const char*".to_string(),
            Ty::String => "char*".to_string(),
            Ty::Ref(inner, _) => inner.to_c_type(),
            Ty::Ptr(inner, _) => format!("{}*", inner.to_c_type()),
            Ty::Array(inner, size) => format!("{}[{}]", inner.to_c_type(), size),
            Ty::Slice(inner) => format!("{}*", inner.to_c_type()),
            Ty::Tuple(_) => "void".to_string(),
            Ty::Fn { .. } => "void".to_string(),
            Ty::Adt(adt) => format!("struct {}", adt.name),
            Ty::Named(name) => format!("struct {}", name),
            Ty::Var(_) => "void".to_string(),
            Ty::Param(_) => "void".to_string(),
            Ty::Future(inner) => format!("Future<{}>", inner.to_c_type()),
            Ty::Module => "void".to_string(),
            Ty::Extern(name) => format!("struct {}", name),
            Ty::FnPtr { .. } => "void*".to_string(),
            Ty::Error => "void".to_string(),
        }
    }

    fn to_c_param_type(&self) -> String {
        match self {
            Ty::Array(inner, _) | Ty::Slice(inner) => {
                format!("{}*", inner.to_c_type())
            }
            _ => self.to_c_type(),
        }
    }
}

/// Helper functions for type conversion
pub mod utils {
    use super::*;

    /// Convert an enum variant to type system representation
    pub fn convert_enum_variant(variant: &ast::EnumVariant) -> VariantDef {
        let fields: Vec<FieldDef> = match &variant.fields {
            ast::EnumVariantFields::Unit => Vec::new(),
            ast::EnumVariantFields::Tuple(types) => types
                .iter()
                .enumerate()
                .map(|(idx, ty)| FieldDef {
                    name: format!("_{}", idx).into(),
                    ty: convert_field_type(ty),
                })
                .collect(),
            ast::EnumVariantFields::Struct(fields) => fields
                .iter()
                .map(|f| FieldDef {
                    name: f.name.clone(),
                    ty: convert_field_type(&f.ty),
                })
                .collect(),
        };

        VariantDef {
            name: variant.name.clone(),
            fields,
        }
    }

    /// Convert field type to Ty
    fn convert_field_type(ty: &Type) -> Ty {
        let converter = StandardTypeConverter::new();
        converter.ast_type_to_ty(ty)
    }

    /// Get the constructor type for an enum variant
    pub fn get_variant_constructor_type(
        _enum_def: &ast::EnumDef,
        variant: &ast::EnumVariant,
        enum_ty: &Ty,
    ) -> Ty {
        let converter = StandardTypeConverter::new();

        match &variant.fields {
            ast::EnumVariantFields::Unit => {
                // Unit variant - just returns the enum type
                enum_ty.clone()
            }
            ast::EnumVariantFields::Tuple(types) => {
                // Tuple variant - returns a function type
                let field_tys: Vec<Ty> = types
                    .iter()
                    .map(|t| converter.ast_type_to_ty(t))
                    .collect();

                Ty::Fn {
                    params: field_tys,
                    ret: Box::new(enum_ty.clone()),
                }
            }
            ast::EnumVariantFields::Struct(fields) => {
                // Struct variant - returns a function type
                let field_tys: Vec<Ty> = fields
                    .iter()
                    .map(|f| converter.ast_type_to_ty(&f.ty))
                    .collect();

                Ty::Fn {
                    params: field_tys,
                    ret: Box::new(enum_ty.clone()),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_conversion() {
        let converter = StandardTypeConverter::new();

        // Test bool
        let ast_ty = Type::Path(Path {
            segments: vec![ast::PathSegment {
                ident: "bool".into(),
                generics: Vec::new(),
            }],
        });
        assert_eq!(converter.ast_type_to_ty(&ast_ty), Ty::Bool);

        // Test i32
        let ast_ty = Type::Path(Path {
            segments: vec![ast::PathSegment {
                ident: "i32".into(),
                generics: Vec::new(),
            }],
        });
        assert_eq!(converter.ast_type_to_ty(&ast_ty), Ty::I32);
    }

    #[test]
    fn test_c_type_conversion() {
        // Test AST Type to C type
        let ast_ty = Type::Path(Path {
            segments: vec![ast::PathSegment {
                ident: "i32".into(),
                generics: Vec::new(),
            }],
        });
        assert_eq!(ast_ty.to_c_type(), "int32_t");

        // Test internal Ty to C type
        let ty = Ty::I32;
        assert_eq!(ty.to_c_type(), "int32_t");
    }
}
