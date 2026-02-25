//! Internal type representation for type checking

use smol_str::SmolStr;
use std::fmt;

/// A type in the type system
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ty {
    /// Unit type: ()
    Unit,

    /// Never type: !
    Never,

    /// Boolean type
    Bool,

    /// Integer types
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,

    /// Unsigned integer types
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,

    /// Floating point types
    F32,
    F64,

    /// Character type
    Char,

    /// String slice type: str
    Str,

    /// String type
    String,

    /// Reference type: &T or &mut T
    Ref(Box<Ty>, Mutability),

    /// Raw pointer type: *const T or *mut T
    Ptr(Box<Ty>, Mutability),

    /// Array type: [T; n]
    Array(Box<Ty>, usize),

    /// Slice type: [T]
    Slice(Box<Ty>),

    /// Tuple type: (T1, T2, ...)
    Tuple(Vec<Ty>),

    /// Function type: fn(T1, T2) -> R
    Fn {
        params: Vec<Ty>,
        ret: Box<Ty>,
    },

    /// User-defined type (struct, enum, etc.)
    Adt(AdtDef),

    /// Named type reference (for forward references)
    Named(SmolStr),

    /// Type variable (for inference)
    Var(TypeVarId),

    /// Generic type parameter
    Param(TypeParamId),

    /// Future type: Future<T> (for async/await)
    Future(Box<Ty>),

    /// Module type (for module imports)
    Module,

    /// Extern type (for FFI)
    Extern(SmolStr),

    /// Function pointer type for FFI callbacks
    FnPtr {
        params: Vec<Ty>,
        ret: Box<Ty>,
        abi: String,
    },

    /// Error type (for error recovery)
    Error,
}

/// Mutability
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mutability {
    Mut,
    Not,
}

/// Type variable ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeVarId(pub u32);

/// Type parameter ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeParamId(pub u32);

/// Algebraic data type definition (struct or enum)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdtDef {
    pub name: SmolStr,
    pub kind: AdtKind,
    pub variants: Vec<VariantDef>,
}

/// ADT kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdtKind {
    Struct,
    Enum,
}

/// Variant definition (for structs and enums)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariantDef {
    pub name: SmolStr,
    pub fields: Vec<FieldDef>,
}

/// Field definition
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FieldDef {
    pub name: SmolStr,
    pub ty: Ty,
}

impl Ty {
    /// Check if this type is an integer type
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            Ty::I8
                | Ty::I16
                | Ty::I32
                | Ty::I64
                | Ty::I128
                | Ty::Isize
                | Ty::U8
                | Ty::U16
                | Ty::U32
                | Ty::U64
                | Ty::U128
                | Ty::Usize
        )
    }

    /// Check if this type is an unsigned integer type
    pub fn is_unsigned(&self) -> bool {
        matches!(
            self,
            Ty::U8 | Ty::U16 | Ty::U32 | Ty::U64 | Ty::U128 | Ty::Usize
        )
    }

    /// Check if this type is a signed integer type
    pub fn is_signed(&self) -> bool {
        matches!(
            self,
            Ty::I8 | Ty::I16 | Ty::I32 | Ty::I64 | Ty::I128 | Ty::Isize
        )
    }

    /// Check if this type is a floating point type
    pub fn is_float(&self) -> bool {
        matches!(self, Ty::F32 | Ty::F64)
    }

    /// Check if this type is numeric (integer or float)
    pub fn is_numeric(&self) -> bool {
        self.is_integer() || self.is_float()
    }

    /// Check if this type is a reference
    pub fn is_ref(&self) -> bool {
        matches!(self, Ty::Ref(_, _))
    }

    /// Check if this type is a mutable reference
    pub fn is_mut_ref(&self) -> bool {
        matches!(self, Ty::Ref(_, Mutability::Mut))
    }

    /// Check if this type can be coerced to another type
    pub fn can_coerce_to(&self, target: &Ty) -> bool {
        match (self, target) {
            // Same type is always compatible
            (a, b) if a == b => true,
            // Integer to integer: smaller can coerce to larger
            (a, b) if a.is_integer() && b.is_integer() => {
                // For simplicity, allow all integer coercions
                true
            }
            // Integer to float
            (a, b) if a.is_integer() && b.is_float() => true,
            // Float to float: smaller to larger
            (Ty::F32, Ty::F64) => true,
            // Reference coercion: &mut T to &T
            (Ty::Ref(t1, Mutability::Mut), Ty::Ref(t2, Mutability::Not)) if t1 == t2 => true,
            _ => false,
        }
    }

    /// Get the size of this type in bytes (simplified)
    pub fn size(&self) -> usize {
        match self {
            Ty::Unit => 0,
            Ty::Never => 0,
            Ty::Bool => 1,
            Ty::I8 | Ty::U8 => 1,
            Ty::I16 | Ty::U16 => 2,
            Ty::I32 | Ty::U32 | Ty::F32 => 4,
            Ty::I64 | Ty::U64 | Ty::F64 => 8,
            Ty::I128 | Ty::U128 => 16,
            Ty::Isize | Ty::Usize => 8, // Assume 64-bit
            Ty::Char => 4,
            Ty::Str => 16,                      // ptr + length
            Ty::String => 24,                   // ptr + length + capacity
            Ty::Ref(_, _) | Ty::Ptr(_, _) => 8, // Assume 64-bit
            Ty::Array(elem, len) => elem.size() * len,
            Ty::Slice(_) => 16, // ptr + length
            Ty::Tuple(elems) => elems.iter().map(|e| e.size()).sum(),
            Ty::Fn { .. } => 8, // Function pointer
            Ty::Adt(adt) => {
                // Calculate size based on the largest variant (for enums)
                // or the only variant (for structs)
                adt.variants
                    .iter()
                    .map(|v| v.fields.iter().map(|f| f.ty.size()).sum::<usize>())
                    .max()
                    .unwrap_or(0)
            }
            Ty::Var(_) => 0,    // Unknown
            Ty::Param(_) => 0,  // Unknown
            Ty::Named(_) => 0,  // Named type, size unknown
            Ty::Future(_) => 8, // Future is a pointer-like object
            Ty::Module => 0,    // Module is a compile-time concept
            Ty::Extern(_) => 0, // Extern type size unknown
            Ty::FnPtr { .. } => 8, // Function pointer
            Ty::Error => 0,
        }
    }

    /// Check if this type can be implicitly convert to another type
    pub fn can_implicitly_convert_to(&self, target: &Ty) -> bool {
        // Same type
        if self == target {
            return true;
        }
        
        // Error type can convert to anything (for error recovery)
        if self == &Ty::Error || target == &Ty::Error {
            return true;
        }
        
        // Type parameters can convert to/from any type (for generic inference)
        if matches!(self, Ty::Param(_)) || matches!(target, Ty::Param(_)) {
            return true;
        }
        
        // Type variables can convert to/from any type (for inference)
        if matches!(self, Ty::Var(_)) || matches!(target, Ty::Var(_)) {
            return true;
        }
        
        // Named types can convert to/from any type (for forward references)
        if matches!(self, Ty::Named(_)) || matches!(target, Ty::Named(_)) {
            return true;
        }

        // Integer to integer (with size check)
        if self.is_integer() && target.is_integer() {
            // Allow if target is larger or same size
            return self.size() <= target.size();
        }

        // Integer to float
        if self.is_integer() && target.is_float() {
            return true;
        }

        // Reference coercion: &mut T to &T
        if let (Ty::Ref(self_inner, Mutability::Mut), Ty::Ref(target_inner, Mutability::Not)) =
            (self, target)
        {
            return self_inner == target_inner;
        }
        
        // Reference to same inner type
        if let (Ty::Ref(self_inner, _), Ty::Ref(target_inner, _)) = (self, target) {
            return self_inner.can_implicitly_convert_to(target_inner);
        }

        // String to str (for function calls)
        if self == &Ty::String && target == &Ty::Str {
            return true;
        }
        
        // ADT type compatibility (for Result, Option, etc.)
        // Two ADTs with the same name are considered compatible
        if let (Ty::Adt(self_adt), Ty::Adt(target_adt)) = (self, target) {
            return self_adt.name == target_adt.name;
        }

        false
    }
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ty::Unit => write!(f, "()"),
            Ty::Never => write!(f, "!"),
            Ty::Bool => write!(f, "bool"),
            Ty::I8 => write!(f, "i8"),
            Ty::I16 => write!(f, "i16"),
            Ty::I32 => write!(f, "i32"),
            Ty::I64 => write!(f, "i64"),
            Ty::I128 => write!(f, "i128"),
            Ty::Isize => write!(f, "isize"),
            Ty::U8 => write!(f, "u8"),
            Ty::U16 => write!(f, "u16"),
            Ty::U32 => write!(f, "u32"),
            Ty::U64 => write!(f, "u64"),
            Ty::U128 => write!(f, "u128"),
            Ty::Usize => write!(f, "usize"),
            Ty::F32 => write!(f, "f32"),
            Ty::F64 => write!(f, "f64"),
            Ty::Char => write!(f, "char"),
            Ty::Str => write!(f, "str"),
            Ty::String => write!(f, "String"),
            Ty::Ref(inner, Mutability::Mut) => write!(f, "&mut {}", inner),
            Ty::Ref(inner, Mutability::Not) => write!(f, "&{}", inner),
            Ty::Ptr(inner, Mutability::Mut) => write!(f, "*mut {}", inner),
            Ty::Ptr(inner, Mutability::Not) => write!(f, "*const {}", inner),
            Ty::Array(inner, len) => write!(f, "[{}; {}]", inner, len),
            Ty::Slice(inner) => write!(f, "[{}]", inner),
            Ty::Tuple(elems) => {
                write!(f, "(")?;
                for (i, elem) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                write!(f, ")")
            }
            Ty::Fn { params, ret } => {
                write!(f, "fn(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }
                write!(f, ") -> {}", ret)
            }
            Ty::Adt(adt) => write!(f, "{}", adt.name),
            Ty::Named(name) => write!(f, "{}", name),
            Ty::Var(id) => write!(f, "'t{}", id.0),
            Ty::Param(id) => write!(f, "'T{}", id.0),
            Ty::Future(inner) => write!(f, "Future<{}>", inner),
            Ty::Module => write!(f, "module"),
            Ty::Extern(name) => write!(f, "extern {}", name),
            Ty::FnPtr { params, ret, abi } => {
                write!(f, "extern \"{}\" fn(", abi)?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }
                write!(f, ") -> {}", ret)
            }
            Ty::Error => write!(f, "<error>"),
        }
    }
}

impl fmt::Display for AdtDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_types() {
        assert!(Ty::I32.is_integer());
        assert!(Ty::U32.is_integer());
        assert!(!Ty::F32.is_integer());
        assert!(!Ty::Bool.is_integer());
    }

    #[test]
    fn test_numeric_types() {
        assert!(Ty::I32.is_numeric());
        assert!(Ty::F64.is_numeric());
        assert!(!Ty::Bool.is_numeric());
    }

    #[test]
    fn test_type_sizes() {
        assert_eq!(Ty::Unit.size(), 0);
        assert_eq!(Ty::Bool.size(), 1);
        assert_eq!(Ty::I8.size(), 1);
        assert_eq!(Ty::I32.size(), 4);
        assert_eq!(Ty::I64.size(), 8);
        assert_eq!(Ty::F32.size(), 4);
        assert_eq!(Ty::F64.size(), 8);
    }

    #[test]
    fn test_array_size() {
        let arr = Ty::Array(Box::new(Ty::I32), 10);
        assert_eq!(arr.size(), 40);
    }

    #[test]
    fn test_implicit_conversion() {
        // i32 to i64 is allowed
        assert!(Ty::I32.can_implicitly_convert_to(&Ty::I64));
        // i64 to i32 is not allowed
        assert!(!Ty::I64.can_implicitly_convert_to(&Ty::I32));
        // i32 to f64 is allowed
        assert!(Ty::I32.can_implicitly_convert_to(&Ty::F64));
    }
}
