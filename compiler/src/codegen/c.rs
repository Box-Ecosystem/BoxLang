//! C code generator

use crate::ast::*;
use crate::codegen::CodeGen;
use crate::frontend::lexer::token::StringLitKind;
use crate::typeck::ty::Ty;
use std::collections::HashMap;
use std::fmt::Write;

/// C code generator
pub struct CCodeGen {
    output: String,
    indent_level: usize,
    /// Variable types inferred during code generation
    var_types: HashMap<String, Ty>,
}

impl CCodeGen {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent_level: 0,
            var_types: HashMap::new(),
        }
    }

    fn indent(&self) -> String {
        "    ".repeat(self.indent_level)
    }

    fn write_line(&mut self, line: &str) {
        self.output.push_str(&self.indent());
        self.output.push_str(line);
        self.output.push('\n');
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn increase_indent(&mut self) {
        self.indent_level += 1;
    }

    fn decrease_indent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }

    fn type_to_c(&self, ty: &Type) -> String {
        match ty {
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
                        "i128" => "__int128",
                        "isize" => "intptr_t",
                        "u8" => "uint8_t",
                        "u16" => "uint16_t",
                        "u32" => "uint32_t",
                        "u64" => "uint64_t",
                        "u128" => "unsigned __int128",
                        "usize" => "uintptr_t",
                        "f32" => "float",
                        "f64" => "double",
                        "char" => "char",
                        "str" => "const char*",
                        "String" => "char*",
                        name => name,
                    }
                    .to_string()
                } else {
                    let path_str = path.segments.iter()
                        .map(|s| s.ident.as_str())
                        .collect::<Vec<_>>()
                        .join("_");
                    path_str
                }
            }
            Type::Ref(inner, _) => self.type_to_c(inner),
            Type::Ptr(inner, _) => format!("{}*", self.type_to_c(inner)),
            Type::Array(inner, size) => {
                let inner_c = self.type_to_c(inner);
                match size {
                    Some(n) => format!("{}[{}]", inner_c, n),
                    None => format!("{}*", inner_c),
                }
            }
            Type::Slice(inner) => {
                let inner_c = self.type_to_c(inner);
                format!("{}*", inner_c)
            }
            Type::Tuple(types) => {
                if types.is_empty() {
                    "void".to_string()
                } else {
                    let type_name = format!("Tuple_{}", types.len());
                    type_name
                }
            }
            Type::Function(func_ty) => {
                let ret_c = self.type_to_c(&func_ty.return_type);
                format!("{}(*)()", ret_c)
            }
            Type::Generic(base, args) => {
                if args.is_empty() {
                    return self.type_to_c(base);
                }
                let base_c = self.type_to_c(base);
                let args_str: Vec<String> = args.iter()
                    .map(|a| self.type_to_c(a))
                    .collect();
                format!("{}_{}", base_c, args_str.join("_"))
            }
            Type::ImplTrait(traits) => {
                if let Some(first_trait) = traits.first() {
                    first_trait.path.segments.first()
                        .map(|s| format!("Impl_{}", s.ident))
                        .unwrap_or_else(|| "ImplTrait".to_string())
                } else {
                    "ImplTrait".to_string()
                }
            }
            Type::DynTrait(traits) => {
                if let Some(first_trait) = traits.first() {
                    first_trait.path.segments.first()
                        .map(|s| format!("Dyn_{}", s.ident))
                        .unwrap_or_else(|| "DynTrait".to_string())
                } else {
                    "DynTrait".to_string()
                }
            }
        }
    }

    /// Convert a type to C type for function parameters (arrays become pointers)
    fn param_type_to_c(&self, ty: &Type) -> String {
        match ty {
            Type::Array(inner, _) | Type::Slice(inner) => {
                let inner_c = self.type_to_c(inner);
                format!("{}*", inner_c)
            }
            _ => self.type_to_c(ty),
        }
    }

    fn generate_item(&mut self, item: &Item) -> Result<(), String> {
        match item {
            Item::Function(func) => self.generate_function(func),
            Item::Struct(struct_def) => self.generate_struct(struct_def),
            Item::Enum(enum_def) => self.generate_enum(enum_def),
            Item::Const(const_def) => self.generate_const(const_def),
            Item::Impl(impl_block) => self.generate_impl(impl_block),
            Item::Static(static_def) => self.generate_static(static_def),
            Item::ExternBlock(extern_block) => self.generate_extern_block(extern_block),
            Item::Callback(callback) => self.generate_callback(callback),
            Item::SafeWrapper(wrapper) => self.generate_safe_wrapper(wrapper),
            _ => {
                Ok(())
            }
        }
    }

    fn generate_function(&mut self, func: &Function) -> Result<(), String> {
        // Function signature
        let mut ret_type = func
            .return_type
            .as_ref()
            .map(|t| self.type_to_c(t))
            .unwrap_or_else(|| "void".to_string());

        let name = func.name.as_str();

        // main function should return int, not void
        if name == "main" && ret_type == "void" {
            ret_type = "int".to_string();
        }

        // Generate parameters
        let params: Vec<String> = func
            .params
            .iter()
            .map(|p| {
                let ty = self.param_type_to_c(&p.ty);
                format!("{} {}", ty, p.name.as_str())
            })
            .collect();

        write!(
            &mut self.output,
            "{} {}({})",
            ret_type,
            name,
            params.join(", ")
        )
        .map_err(|e| e.to_string())?;

        // Function body
        self.generate_block(&func.body)?;
        self.write("\n\n");

        Ok(())
    }

    fn generate_struct(&mut self, struct_def: &StructDef) -> Result<(), String> {
        self.write_line(&format!("typedef struct {0} {{", struct_def.name));
        self.increase_indent();

        for field in &struct_def.fields {
            let ty = self.type_to_c(&field.ty);
            self.write_line(&format!("{} {};", ty, field.name.as_str()));
        }

        self.decrease_indent();
        self.write_line(&format!("}} {};", struct_def.name));
        self.write("\n");

        Ok(())
    }

    fn generate_enum(&mut self, enum_def: &EnumDef) -> Result<(), String> {
        self.write_line(&format!("typedef enum {0} {{", enum_def.name));
        self.increase_indent();

        for (i, variant) in enum_def.variants.iter().enumerate() {
            let comma = if i < enum_def.variants.len() - 1 {
                ","
            } else {
                ""
            };
            self.write_line(&format!("{}{}", variant.name, comma));
        }

        self.decrease_indent();
        self.write_line(&format!("}} {};", enum_def.name));
        self.write("\n");

        Ok(())
    }

    fn generate_const(&mut self, const_def: &ConstDef) -> Result<(), String> {
        let ty = self.type_to_c(&const_def.ty);
        let value = self.expr_to_c(&const_def.value)?;
        self.write_line(&format!(
            "const {} {} = {};",
            ty,
            const_def.name.as_str(),
            value
        ));
        Ok(())
    }

    fn generate_static(&mut self, static_def: &StaticDef) -> Result<(), String> {
        let ty = self.type_to_c(&static_def.ty);
        let value = self.expr_to_c(&static_def.value)?;
        self.write_line(&format!(
            "static {} {} = {};",
            ty,
            static_def.name.as_str(),
            value
        ));
        Ok(())
    }

    fn generate_impl(&mut self, impl_block: &ImplBlock) -> Result<(), String> {
        // Get the type name for method naming
        let type_name = self.type_to_c(&impl_block.ty);

        // Generate each method
        for item in &impl_block.items {
            if let ImplItem::Function(func) = item {
                self.generate_method(func, &type_name)?;
            }
        }

        Ok(())
    }

    fn generate_method(&mut self, func: &Function, type_name: &str) -> Result<(), String> {
        // Method signature: ret_type Type_method(params)
        let ret_type = func
            .return_type
            .as_ref()
            .map(|t| self.type_to_c(t))
            .unwrap_or_else(|| "void".to_string());

        // Generate method name: Type_method
        let method_name = format!("{}_{}", type_name, func.name.as_str());

        // Generate parameters
        let mut params: Vec<String> = Vec::new();

        // Check if first param is self
        let mut param_start = 0;
        if let Some(first_param) = func.params.first() {
            if first_param.name.as_str() == "self" {
                // Add self as first parameter (pointer to struct)
                params.push(format!("{}* self", type_name));
                param_start = 1;
            }
        }

        // Add remaining parameters
        for param in func.params.iter().skip(param_start) {
            let ty = self.type_to_c(&param.ty);
            params.push(format!("{} {}", ty, param.name.as_str()));
        }

        write!(
            &mut self.output,
            "{} {}({})",
            ret_type,
            method_name,
            params.join(", ")
        )
        .map_err(|e| e.to_string())?;

        // Generate method body
        self.generate_block(&func.body)?;
        self.write("\n\n");

        Ok(())
    }

    fn generate_block(&mut self, block: &Block) -> Result<(), String> {
        self.write(" {\n");
        self.increase_indent();

        for (i, stmt) in block.stmts.iter().enumerate() {
            let is_last = i == block.stmts.len() - 1;
            self.generate_stmt(stmt, is_last)?;
        }

        self.decrease_indent();
        self.write_line("}");
        Ok(())
    }

    fn generate_stmt(&mut self, stmt: &Stmt, is_last: bool) -> Result<(), String> {
        match stmt {
            Stmt::Let(let_stmt) => {
                // Determine the type from annotation or infer from init expression
                let (ty_str, ty) = if let Some(annotated_ty) = &let_stmt.ty {
                    let ty_str = self.type_to_c(annotated_ty);
                    let ty = self.ast_type_to_ty(annotated_ty);
                    (ty_str, ty)
                } else if let Some(init) = &let_stmt.init {
                    // Infer type from initialization expression
                    let ty = self.infer_expr_type(init);
                    let ty_str = self.ty_to_c(&ty);
                    (ty_str, ty)
                } else {
                    ("int".to_string(), Ty::I32)
                };

                // Check if it's an array type before moving
                let is_array = matches!(ty, Ty::Array(_, _));

                // Store the variable type for later use
                self.var_types
                    .insert(let_stmt.name.as_str().to_string(), ty);

                if let Some(init) = &let_stmt.init {
                    let value = self.expr_to_c(init)?;
                    // Special handling for array types
                    let decl = if is_array {
                        format!("{} {}[] = {};", ty_str, let_stmt.name.as_str(), value)
                    } else {
                        format!("{} {} = {};", ty_str, let_stmt.name.as_str(), value)
                    };
                    self.write_line(&decl);
                } else {
                    self.write_line(&format!("{} {};", ty_str, let_stmt.name.as_str()));
                }
            }
            Stmt::Expr(expr) => {
                // Handle loop expressions specially
                match expr {
                    Expr::While(while_expr) => {
                        self.generate_while(while_expr)?;
                    }
                    Expr::Loop(loop_expr) => {
                        self.generate_loop(loop_expr)?;
                    }
                    Expr::For(for_expr) => {
                        self.generate_for(for_expr)?;
                    }
                    _ => {
                        let expr_str = self.expr_to_c(expr)?;
                        // Only generate return for the last statement if it's not an assignment
                        // and not already a return statement
                        let is_assignment = matches!(expr, Expr::Assign(_));
                        let is_return = matches!(expr, Expr::Return(_));
                        if is_last && !is_return && !is_assignment {
                            self.write_line(&format!("return {};", expr_str));
                        } else {
                            self.write_line(&format!("{};", expr_str));
                        }
                    }
                }
            }
            Stmt::Item(item) => {
                self.generate_item(item)?;
            }
        }
        Ok(())
    }

    fn expr_to_c(&mut self, expr: &Expr) -> Result<String, String> {
        match expr {
            Expr::Literal(lit) => Ok(self.literal_to_c(lit)),
            Expr::Ident(name) => Ok(name.as_str().to_string()),
            Expr::Path(path) => {
                // Convert path to C identifier (e.g., Result::Ok -> Result_Ok)
                let path_str = path.segments.iter()
                    .map(|s| s.ident.as_str())
                    .collect::<Vec<_>>()
                    .join("_");
                Ok(path_str)
            }
            Expr::PathCall(path, args) => {
                // Convert path call to C function call (e.g., Result::Ok(v) -> Result_Ok(v))
                let func_name = path.segments.iter()
                    .map(|s| s.ident.as_str())
                    .collect::<Vec<_>>()
                    .join("_");
                let args_str: Vec<String> = args
                    .iter()
                    .map(|a| self.expr_to_c(a))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(format!("{}({})", func_name, args_str.join(", ")))
            }
            Expr::Binary(binary) => {
                // Handle pipeline operator specially: a |> b -> b(a)
                if binary.op == BinaryOp::Pipe {
                    let left = self.expr_to_c(&binary.left)?;
                    let right = self.expr_to_c(&binary.right)?;
                    // right should be a function call or function name
                    Ok(format!("{}({})", right, left))
                } else {
                    let left = self.expr_to_c(&binary.left)?;
                    let right = self.expr_to_c(&binary.right)?;
                    let op = self.binary_op_to_c(binary.op);
                    Ok(format!("({} {} {})", left, op, right))
                }
            }
            Expr::Unary(unary) => {
                let expr = self.expr_to_c(&unary.expr)?;
                let op = self.unary_op_to_c(unary.op);
                Ok(format!("({}{})", op, expr))
            }
            Expr::Call(call) => {
                // Check for method call: Type::method()
                if let Expr::FieldAccess(field_access) = call.func.as_ref() {
                    if let Expr::Ident(type_name) = field_access.expr.as_ref() {
                        let method_name = &field_access.field;
                        let args: Vec<String> = call
                            .args
                            .iter()
                            .map(|a| self.expr_to_c(a))
                            .collect::<Result<Vec<_>, _>>()?;
                        return Ok(format!(
                            "{}_{}({})",
                            type_name.as_str(),
                            method_name.as_str(),
                            args.join(", ")
                        ));
                    }
                }

                let func = self.expr_to_c(&call.func)?;
                let args: Vec<String> = call
                    .args
                    .iter()
                    .map(|a| self.expr_to_c(a))
                    .collect::<Result<Vec<_>, _>>()?;

                // Handle builtin functions
                match func.as_str() {
                    "println" => {
                        // println("...") -> printf("...\n")
                        if args.len() == 1 {
                            Ok(format!("(printf({} \"\\n\"), 0)", args[0]))
                        } else {
                            Ok(format!("(printf({}), 0)", args.join(", ")))
                        }
                    }
                    "print" => {
                        // print("...") -> printf("...")
                        Ok(format!("(printf({}), 0)", args.join(", ")))
                    }
                    _ => Ok(format!("{}({})", func, args.join(", "))),
                }
            }
            Expr::MethodCall(method_call) => {
                // obj.method(args) -> Type_method(&obj, args)
                let receiver = self.expr_to_c(&method_call.receiver)?;
                let method_name = method_call.method.as_str();

                // Sanitize method name to prevent injection
                let safe_method_name: String = method_name
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_')
                    .collect();

                if safe_method_name.is_empty() {
                    return Err("Invalid method name: empty or contains only invalid characters".to_string());
                }

                // Validate method name doesn't start with digit
                if safe_method_name.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    return Err("Invalid method name: cannot start with a digit".to_string());
                }

                // Get the type name from receiver - we need to extract it from the expression
                // The receiver should be a valid C identifier
                let receiver_type = receiver
                    .split('.')
                    .next()
                    .unwrap_or(&receiver)
                    .to_string();

                // Validate receiver type name
                let safe_receiver: String = receiver_type
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_')
                    .collect();

                if safe_receiver.is_empty() {
                    return Err("Invalid receiver type name".to_string());
                }

                // Generate arguments
                let mut args: Vec<String> = vec![format!("&{}", receiver)];
                for arg in &method_call.args {
                    match self.expr_to_c(arg) {
                        Ok(arg_str) => args.push(arg_str),
                        Err(e) => return Err(format!("Failed to generate argument: {}", e)),
                    }
                }

                // Use sanitized names
                Ok(format!(
                    "{}_{}({})",
                    safe_receiver,
                    safe_method_name,
                    args.join(", ")
                ))
            }
            Expr::FieldAccess(field_access) => {
                let expr = self.expr_to_c(&field_access.expr)?;
                let field = field_access.field.as_str();
                Ok(format!("{}.{}", expr, field))
            }
            Expr::StructInit(struct_init) => {
                // Generate struct initialization
                let type_name = if struct_init.path.segments.len() == 1 {
                    struct_init.path.segments[0].ident.as_str()
                } else {
                    return Err("Complex path not supported".to_string());
                };

                let mut fields: Vec<String> = Vec::new();
                for (name, value) in &struct_init.fields {
                    let val = self.expr_to_c(value)?;
                    fields.push(format!(".{} = {}", name.as_str(), val));
                }

                Ok(format!("({}){{{}}}", type_name, fields.join(", ")))
            }
            Expr::Block(block) => {
                // For block expressions, we need to handle them specially
                // For now, just return the last statement
                if let Some(last_stmt) = block.stmts.last() {
                    match last_stmt {
                        Stmt::Expr(expr) => self.expr_to_c(expr),
                        _ => Ok("0".to_string()),
                    }
                } else {
                    Ok("0".to_string())
                }
            }
            Expr::Return(ret) => {
                if let Some(expr) = ret {
                    let value = self.expr_to_c(expr)?;
                    Ok(format!("return {}", value))
                } else {
                    Ok("return".to_string())
                }
            }
            Expr::If(if_expr) => {
                // Generate if statement as a statement, not an expression
                // Since C doesn't have if expressions, we need to handle this specially
                let cond = self.expr_to_c(&if_expr.cond)?;

                // Generate the if statement
                self.write_line(&format!("if ({}) {{", cond));
                self.increase_indent();

                // Generate then branch
                for (i, stmt) in if_expr.then_branch.stmts.iter().enumerate() {
                    let is_last = i == if_expr.then_branch.stmts.len() - 1;
                    self.generate_stmt(stmt, is_last)?;
                }

                self.decrease_indent();

                // Generate else branch if present
                if let Some(else_branch) = &if_expr.else_branch {
                    self.write_line("} else {");
                    self.increase_indent();

                    // Handle different types of else branches
                    match else_branch.as_ref() {
                        Expr::Block(block) => {
                            for (i, stmt) in block.stmts.iter().enumerate() {
                                let is_last = i == block.stmts.len() - 1;
                                self.generate_stmt(stmt, is_last)?;
                            }
                        }
                        Expr::If(_) => {
                            // Else-if: recursively generate
                            let else_expr = self.expr_to_c(else_branch)?;
                            self.write_line(&else_expr);
                        }
                        _ => {
                            // Single expression else branch
                            let else_val = self.expr_to_c(else_branch)?;
                            self.write_line(&format!("{};", else_val));
                        }
                    }

                    self.decrease_indent();
                    self.write_line("}");
                } else {
                    self.write_line("}");
                }

                // Return 0 since if statement doesn't produce a value in C
                Ok("0".to_string())
            }
            Expr::While(_while_expr) => {
                // Return 0 for now
                Ok("0".to_string())
            }
            Expr::Loop(_) => {
                // Return 0 for now
                Ok("0".to_string())
            }
            Expr::For(_) => {
                // Return 0 for now
                Ok("0".to_string())
            }
            Expr::Break(val) => {
                if let Some(v) = val {
                    self.expr_to_c(v)
                } else {
                    Ok("0".to_string())
                }
            }
            Expr::Continue => Ok("0".to_string()),
            Expr::Assign(assign) => {
                let left = self.expr_to_c(&assign.left)?;
                let right = self.expr_to_c(&assign.right)?;
                Ok(format!("{} = {}", left, right))
            }
            Expr::Async(block) => {
                // For now, async blocks are compiled as regular blocks
                // In a full implementation, this would create a future/generator
                if let Some(last_stmt) = block.stmts.last() {
                    match last_stmt {
                        Stmt::Expr(expr) => self.expr_to_c(expr),
                        _ => Ok("0".to_string()),
                    }
                } else {
                    Ok("0".to_string())
                }
            }
            Expr::Await(expr) => {
                // For now, await just evaluates the expression
                // In a full implementation, this would wait for the future
                self.expr_to_c(expr)
            }
            Expr::Match(match_expr) => self.generate_match(match_expr),
            Expr::Closure(_) => {
                // Closures are not directly supported in C
                // Return a placeholder function pointer
                Ok("NULL".to_string())
            }
            Expr::ArrayInit(array) => {
                // Generate array literal: {elem1, elem2, ...}
                let elements: Result<Vec<String>, String> = array
                    .elements
                    .iter()
                    .map(|e| self.expr_to_c(e))
                    .collect();
                Ok(format!("{{{}}}", elements?.join(", ")))
            }
            Expr::Index(index) => {
                let base = self.expr_to_c(&index.expr)?;
                let idx = self.expr_to_c(&index.index)?;
                Ok(format!("{}[{}]", base, idx))
            }
            Expr::Range(range) => {
                // Ranges are not directly supported in C
                // Return a placeholder
                let start = range
                    .start
                    .as_ref()
                    .map(|e| self.expr_to_c(e))
                    .unwrap_or_else(|| Ok("0".to_string()))?;
                let end = range
                    .end
                    .as_ref()
                    .map(|e| self.expr_to_c(e))
                    .unwrap_or_else(|| Ok("0".to_string()))?;
                Ok(format!("{{{}, {}}}", start, end))
            }
            Expr::Cast(cast) => {
                let expr = self.expr_to_c(&cast.expr)?;
                let target_ty = self.type_to_c(&cast.ty);
                Ok(format!("({}){}", target_ty, expr))
            }
            Expr::Try(expr) => {
                // Try operator is not directly supported in C
                // Just evaluate the expression
                self.expr_to_c(expr)
            }
            Expr::Unsafe(block) => {
                // Unsafe blocks are compiled as regular blocks in C
                if let Some(last_stmt) = block.stmts.last() {
                    match last_stmt {
                        Stmt::Expr(expr) => self.expr_to_c(expr),
                        _ => Ok("0".to_string()),
                    }
                } else {
                    Ok("0".to_string())
                }
            }
            Expr::TupleInit(_exprs) => {
                // Tuples are not directly supported in C
                // Return a placeholder
                Ok("0".to_string())
            }
            Expr::SizeOf(ty) => {
                // sizeof(type) in C
                let c_ty = self.type_to_c(ty);
                Ok(format!("sizeof({})", c_ty))
            }
            Expr::TypeOf(expr) => {
                // typeof(expr) in C (GNU extension)
                let c_expr = self.expr_to_c(expr)?;
                Ok(format!("typeof({})", c_expr))
            }
            Expr::Asm(_) => {
                // Inline assembly - not supported in C output
                Ok("0".to_string())
            }
            Expr::CompoundAssign(assign) => {
                let left = self.expr_to_c(&assign.left)?;
                let right = self.expr_to_c(&assign.right)?;
                let op = self.binary_op_to_c(assign.op);
                Ok(format!("{} {}= {}", left, op, right))
            }
        }
    }

    fn literal_to_c(&self, lit: &Literal) -> String {
        match lit {
            Literal::Integer(n) => n.to_string(),
            Literal::Float(f) => f.to_string(),
            Literal::String(StringLitKind::Simple(s)) => format!("\"{}\"", s),
            Literal::String(StringLitKind::Interpolated(s)) => {
                // For interpolated strings, we need to parse and generate format string
                // For now, treat as simple string
                format!("\"{}\"", s)
            }
            Literal::Char(c) => format!("'{}'", c),
            Literal::Bool(b) => (if *b { "1" } else { "0" }).to_string(),
            Literal::Null => "NULL".to_string(),
        }
    }

    fn binary_op_to_c(&self, op: BinaryOp) -> &'static str {
        match op {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Rem => "%",
            BinaryOp::And => "&",
            BinaryOp::Or => "|",
            BinaryOp::Xor => "^",
            BinaryOp::Shl => "<<",
            BinaryOp::Shr => ">>",
            BinaryOp::Eq => "==",
            BinaryOp::Ne => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Le => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Ge => ">=",
            BinaryOp::LogicalAnd => "&&",
            BinaryOp::LogicalOr => "||",
            BinaryOp::Assign => "=",
            BinaryOp::Pipe => "|>", // Pipeline operator (handled specially in expr_to_c)
        }
    }

    fn unary_op_to_c(&self, op: UnaryOp) -> &'static str {
        match op {
            UnaryOp::Neg => "-",
            UnaryOp::Not => "!",
            UnaryOp::Deref => "*",
            UnaryOp::Ref => "&",
            UnaryOp::RefMut => "&",
        }
    }

    /// Infer the type of an expression
    fn infer_expr_type(&self, expr: &Expr) -> Ty {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Integer(_) => Ty::I32,
                Literal::Float(_) => Ty::F64,
                Literal::String(_) => Ty::String,
                Literal::Char(_) => Ty::Char,
                Literal::Bool(_) => Ty::Bool,
                Literal::Null => Ty::Unit,
            },
            Expr::Ident(name) => {
                // Look up variable type
                self.var_types
                    .get(name.as_str())
                    .cloned()
                    .unwrap_or(Ty::I32)
            }
            Expr::FieldAccess(_field_access) => {
                // For now, assume field access on struct returns i32
                // In a full implementation, we'd look up the field type
                Ty::I32
            }
            Expr::StructInit(struct_init) => {
                // Return the struct type
                let name = if struct_init.path.segments.len() == 1 {
                    struct_init.path.segments[0].ident.clone()
                } else {
                    return Ty::Error;
                };
                Ty::Adt(crate::typeck::ty::AdtDef {
                    name,
                    kind: crate::typeck::ty::AdtKind::Struct,
                    variants: Vec::new(),
                })
            }
            Expr::Binary(binary) => {
                // For arithmetic operations, return the type of the left operand
                self.infer_expr_type(&binary.left)
            }
            Expr::Call(_call) => {
                // For now, assume function calls return i32
                // In a full implementation, we'd look up the function return type
                Ty::I32
            }
            Expr::ArrayInit(array) => {
                // Infer element type from first element
                let elem_ty = if let Some(first) = array.elements.first() {
                    self.infer_expr_type(first)
                } else {
                    Ty::I32
                };
                let size = array.elements.len();
                Ty::Array(Box::new(elem_ty), size)
            }
            _ => Ty::I32,
        }
    }

    /// Convert a Ty to C type string
    fn ty_to_c(&self, ty: &Ty) -> String {
        match ty {
            Ty::I32 => "int".to_string(),
            Ty::I64 => "long long".to_string(),
            Ty::U32 => "unsigned int".to_string(),
            Ty::U64 => "unsigned long long".to_string(),
            Ty::F32 => "float".to_string(),
            Ty::F64 => "double".to_string(),
            Ty::Bool => "int".to_string(),
            Ty::Char => "char".to_string(),
            Ty::String => "char*".to_string(),
            Ty::Unit => "void".to_string(),
            Ty::Adt(adt) => adt.name.as_str().to_string(),
            _ => "int".to_string(),
        }
    }

    /// Convert AST Type to Ty
    fn ast_type_to_ty(&self, ty: &Type) -> Ty {
        match ty {
            Type::Path(path) => {
                if path.segments.len() == 1 {
                    let name = path.segments[0].ident.as_str();
                    match name {
                        "i32" => Ty::I32,
                        "i64" => Ty::I64,
                        "u32" => Ty::U32,
                        "u64" => Ty::U64,
                        "f32" => Ty::F32,
                        "f64" => Ty::F64,
                        "bool" => Ty::Bool,
                        "char" => Ty::Char,
                        "String" => Ty::String,
                        "str" => Ty::Str,
                        "()" | "Unit" => Ty::Unit,
                        _ => Ty::Adt(crate::typeck::ty::AdtDef {
                            name: name.into(),
                            kind: crate::typeck::ty::AdtKind::Struct,
                            variants: Vec::new(),
                        }),
                    }
                } else {
                    Ty::Error
                }
            }
            Type::Ref(inner, is_mut) => {
                let mutability = if *is_mut {
                    crate::typeck::ty::Mutability::Mut
                } else {
                    crate::typeck::ty::Mutability::Not
                };
                Ty::Ref(Box::new(self.ast_type_to_ty(inner)), mutability)
            }
            Type::Array(inner, size) => {
                let size = size.unwrap_or(0);
                Ty::Array(Box::new(self.ast_type_to_ty(inner)), size)
            }
            Type::Tuple(types) => Ty::Tuple(types.iter().map(|t| self.ast_type_to_ty(t)).collect()),
            Type::Function(func_ty) => Ty::Fn {
                params: func_ty
                    .params
                    .iter()
                    .map(|t| self.ast_type_to_ty(t))
                    .collect(),
                ret: Box::new(self.ast_type_to_ty(&func_ty.return_type)),
            },
            Type::Never => Ty::Never,
            _ => Ty::Error,
        }
    }

    /// Generate a while loop
    fn generate_while(&mut self, while_expr: &WhileExpr) -> Result<(), String> {
        let cond = self.expr_to_c(&while_expr.cond)?;
        self.write_line(&format!("while ({}) {{", cond));
        self.increase_indent();

        // Generate loop body statements
        // Loop body should never generate return statements
        for stmt in &while_expr.body.stmts {
            self.generate_loop_body_stmt(stmt)?;
        }

        self.decrease_indent();
        self.write_line("}");
        Ok(())
    }

    /// Generate a statement in loop body (never generates return)
    fn generate_loop_body_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Let(let_stmt) => {
                // Determine the type from annotation or infer from init expression
                let (ty_str, ty) = if let Some(annotated_ty) = &let_stmt.ty {
                    let ty_str = self.type_to_c(annotated_ty);
                    let ty = self.ast_type_to_ty(annotated_ty);
                    (ty_str, ty)
                } else if let Some(init) = &let_stmt.init {
                    // Infer type from initialization expression
                    let ty = self.infer_expr_type(init);
                    let ty_str = self.ty_to_c(&ty);
                    (ty_str, ty)
                } else {
                    ("int".to_string(), Ty::I32)
                };

                // Store the variable type for later use
                self.var_types
                    .insert(let_stmt.name.as_str().to_string(), ty);

                if let Some(init) = &let_stmt.init {
                    let value = self.expr_to_c(init)?;
                    self.write_line(&format!(
                        "{} {} = {};",
                        ty_str,
                        let_stmt.name.as_str(),
                        value
                    ));
                } else {
                    self.write_line(&format!("{} {};", ty_str, let_stmt.name.as_str()));
                }
            }
            Stmt::Expr(expr) => {
                // Handle nested loop expressions
                match expr {
                    Expr::While(while_expr) => {
                        self.generate_while(while_expr)?;
                    }
                    Expr::Loop(loop_expr) => {
                        self.generate_loop(loop_expr)?;
                    }
                    Expr::For(for_expr) => {
                        self.generate_for(for_expr)?;
                    }
                    Expr::If(_if_expr) => {
                        // Handle if expression in loop body
                        let _ = self.expr_to_c(expr)?;
                    }
                    _ => {
                        let expr_str = self.expr_to_c(expr)?;
                        // Never generate return in loop body
                        self.write_line(&format!("{};", expr_str));
                    }
                }
            }
            Stmt::Item(item) => {
                self.generate_item(item)?;
            }
        }
        Ok(())
    }

    /// Generate an infinite loop
    fn generate_loop(&mut self, loop_expr: &LoopExpr) -> Result<(), String> {
        self.write_line("while (1) {");
        self.increase_indent();

        // Generate loop body statements
        // Loop body should never generate return statements
        for stmt in &loop_expr.body.stmts {
            self.generate_loop_body_stmt(stmt)?;
        }

        self.decrease_indent();
        self.write_line("}");
        Ok(())
    }

    /// Generate a for loop
    fn generate_for(&mut self, for_expr: &ForExpr) -> Result<(), String> {
        // Get the iteration variable name from pattern
        let var_name = self.pattern_to_var_name(&for_expr.pattern)?;

        // Handle different iterator types
        match for_expr.expr.as_ref() {
            // Handle range expressions like 0..5 or 0..=5
            Expr::Range(range) => {
                let start = range
                    .start
                    .as_ref()
                    .map(|e| self.expr_to_c(e))
                    .unwrap_or_else(|| Ok("0".to_string()))?;
                let end = range
                    .end
                    .as_ref()
                    .map(|e| self.expr_to_c(e))
                    .unwrap_or_else(|| Ok("0".to_string()))?;
                
                // Generate C for loop
                if range.inclusive {
                    // Inclusive range: 0..=5
                    self.write_line(&format!(
                        "for (int {} = {}; {} <= {}; {}++) {{",
                        var_name, start, var_name, end, var_name
                    ));
                } else {
                    // Exclusive range: 0..5
                    self.write_line(&format!(
                        "for (int {} = {}; {} < {}; {}++) {{",
                        var_name, start, var_name, end, var_name
                    ));
                }
            }
            // Handle other iterator expressions
            _ => {
                // For now, generate a simple loop with a counter
                let iter_expr = self.expr_to_c(&for_expr.expr)?;
                self.write_line(&format!(
                    "for (int {} = 0; {} < {}; {}++) {{",
                    var_name, var_name, iter_expr, var_name
                ));
            }
        }
        
        self.increase_indent();

        // Generate loop body statements
        // Loop body should never generate return statements
        for stmt in &for_expr.body.stmts {
            self.generate_loop_body_stmt(stmt)?;
        }

        self.decrease_indent();
        self.write_line("}");
        Ok(())
    }

    /// Extract variable name from a pattern
    fn pattern_to_var_name(&self, pattern: &Pattern) -> Result<String, String> {
        match pattern {
            Pattern::Ident(name) => Ok(name.to_string()),
            Pattern::Mut(inner) => {
                // For mutable patterns like `mut x`, extract the inner name
                if let Pattern::Ident(name) = inner.as_ref() {
                    Ok(name.to_string())
                } else {
                    self.pattern_to_var_name(inner)
                }
            }
            Pattern::Ref(inner) => {
                // For reference patterns like `&x`, extract the inner name
                self.pattern_to_var_name(inner)
            }
            Pattern::Wildcard => {
                // Use a placeholder for wildcard patterns
                Ok("__wildcard".to_string())
            }
            _ => Err(format!("Unsupported pattern in for loop: {:?}", pattern)),
        }
    }

    /// Generate a match expression
    fn generate_match(&mut self, match_expr: &MatchExpr) -> Result<String, String> {
        // Generate scrutinee
        let scrutinee = self.expr_to_c(&match_expr.expr)?;

        // Create a temporary variable for the result
        let result_var = format!("__match_result_{}", self.indent_level);
        let result_ty = self.infer_expr_type(&match_expr.arms[0].body);
        let c_ty = self.ty_to_c(&result_ty);

        // Generate switch statement
        self.write_line(&format!("{} {};", c_ty, result_var));
        self.write_line(&format!("switch ({}) {{", scrutinee));
        self.increase_indent();

        // Generate cases for each arm
        for arm in &match_expr.arms {
            match &arm.pattern {
                Pattern::Literal(lit) => {
                    let val = self.literal_to_c(lit);
                    self.write_line(&format!("case {}:", val));
                    self.increase_indent();

                    // Handle guard if present
                    if let Some(guard) = &arm.guard {
                        let guard_expr = self.expr_to_c(guard)?;
                        self.write_line(&format!("if (!({})) goto __match_default;", guard_expr));
                    }

                    // Generate arm body
                    let body_expr = self.expr_to_c(&arm.body)?;
                    self.write_line(&format!("{} = {};", result_var, body_expr));
                    self.write_line("break;");
                    self.decrease_indent();
                }
                Pattern::Wildcard => {
                    self.write_line("default:");
                    self.increase_indent();

                    // Generate arm body
                    let body_expr = self.expr_to_c(&arm.body)?;
                    self.write_line(&format!("{} = {};", result_var, body_expr));
                    self.write_line("break;");
                    self.decrease_indent();
                }
                _ => {
                    // For other patterns, use default case
                    self.write_line("default:");
                    self.increase_indent();
                    self.write_line(&format!("{} = 0;", result_var));
                    self.write_line("break;");
                    self.decrease_indent();
                }
            }
        }

        self.decrease_indent();
        self.write_line("}");

        Ok(result_var)
    }

    /// Generate extern block declarations
    fn generate_extern_block(&mut self, extern_block: &ExternBlock) -> Result<(), String> {
        for item in &extern_block.items {
            match item {
                ExternItem::Function(func) => {
                    self.generate_extern_function(func, &extern_block.abi)?;
                }
                ExternItem::Static(static_def) => {
                    self.generate_extern_static(static_def)?;
                }
                ExternItem::Type(extern_type) => {
                    self.generate_extern_type(extern_type)?;
                }
            }
        }
        Ok(())
    }

    /// Generate extern function declaration
    fn generate_extern_function(&mut self, func: &Function, abi: &str) -> Result<(), String> {
        let ret_type = func
            .return_type
            .as_ref()
            .map(|t| self.type_to_c(t))
            .unwrap_or_else(|| "void".to_string());

        let name = if let Some(link_name) = &func.ffi_attrs.link_name {
            link_name.clone()
        } else {
            func.name.as_str().to_string()
        };

        let params: Vec<String> = func
            .params
            .iter()
            .map(|p| {
                let ty = self.param_type_to_c(&p.ty);
                format!("{} {}", ty, p.name.as_str())
            })
            .collect();

        // Generate extern declaration
        if abi == "C" || abi == "cdecl" {
            self.write_line(&format!(
                "extern {} {}({});",
                ret_type,
                name,
                if params.is_empty() { "void" } else { &params.join(", ") }
            ));
        } else {
            // For other ABIs, add a comment
            self.write_line(&format!("/* extern \"{}\" */", abi));
            self.write_line(&format!(
                "extern {} {}({});",
                ret_type,
                name,
                if params.is_empty() { "void" } else { &params.join(", ") }
            ));
        }
        self.write("\n");
        Ok(())
    }

    /// Generate extern static declaration
    fn generate_extern_static(&mut self, static_def: &StaticDef) -> Result<(), String> {
        let ty = self.type_to_c(&static_def.ty);
        let name = static_def.name.as_str();
        
        if static_def.is_mut {
            self.write_line(&format!("extern {} {};", ty, name));
        } else {
            self.write_line(&format!("extern const {} {};", ty, name));
        }
        Ok(())
    }

    /// Generate extern type declaration (typedef)
    fn generate_extern_type(&mut self, extern_type: &ExternType) -> Result<(), String> {
        let name = extern_type.name.as_str();
        // Generate a forward declaration as an opaque type
        self.write_line(&format!("typedef struct {} {};", name, name));
        Ok(())
    }

    /// Generate callback type and trampoline
    fn generate_callback(&mut self, callback: &CallbackDef) -> Result<(), String> {
        let ret_type = callback
            .return_type
            .as_ref()
            .map(|t| self.type_to_c(t))
            .unwrap_or_else(|| "void".to_string());

        let name = callback.name.as_str();
        let params: Vec<String> = callback
            .params
            .iter()
            .map(|p| {
                let ty = self.param_type_to_c(&p.ty);
                format!("{} {}", ty, p.name.as_str())
            })
            .collect();

        // Generate callback typedef
        self.write_line(&format!(
            "typedef {} (*{})({});",
            ret_type,
            name,
            if params.is_empty() { "void" } else { &params.join(", ") }
        ));
        self.write("\n");
        Ok(())
    }

    /// Generate safe wrapper function
    fn generate_safe_wrapper(&mut self, wrapper: &SafeWrapper) -> Result<(), String> {
        let ret_type = wrapper
            .return_type
            .as_ref()
            .map(|t| self.type_to_c(t))
            .unwrap_or_else(|| "void".to_string());

        let wrapper_name = wrapper.wrapper_name.as_str();
        let extern_name = wrapper.extern_name.as_str();

        // Generate wrapper function signature
        let params: Vec<String> = wrapper
            .params
            .iter()
            .map(|p| {
                let ty = self.param_type_to_c(&p.ty);
                format!("{} {}", ty, p.name.as_str())
            })
            .collect();

        write!(
            &mut self.output,
            "{} {}({})",
            ret_type,
            wrapper_name,
            params.join(", ")
        )
        .map_err(|e| e.to_string())?;

        // Generate wrapper body with error handling
        self.write(" {\n");
        self.increase_indent();

        // Call the extern function with null checks
        let args: Vec<String> = wrapper
            .params
            .iter()
            .map(|p| p.name.as_str().to_string())
            .collect();

        if let Some(error_type) = &wrapper.error_type {
            // Generate error handling wrapper
            self.write_line("if (setjmp(__error_jmp) != 0) {");
            self.increase_indent();
            self.write_line(&format!("return {}_error();", wrapper_name));
            self.decrease_indent();
            self.write_line("}");
            self.write_line(&format!("{} result = {}({});", ret_type, extern_name, args.join(", ")));
            self.write_line("return result;");
        } else {
            // Simple wrapper without error handling
            self.write_line(&format!("return {}({});", extern_name, args.join(", ")));
        }

        self.decrease_indent();
        self.write_line("}\n");
        Ok(())
    }
}

impl CodeGen for CCodeGen {
    type Output = String;
    type Error = String;

    fn generate(&mut self, module: &Module) -> Result<Self::Output, Self::Error> {
        // Add header
        self.write_line("/* Generated by BoxLang Compiler */");
        self.write_line("#include <stdint.h>");
        self.write_line("#include <stddef.h>");
        self.write_line("#include <stdbool.h>");
        self.write_line("#include <stdio.h>");
        self.write("\n");

        // Generate items
        for item in &module.items {
            self.generate_item(item)?;
        }

        Ok(self.output.clone())
    }
}

impl Default for CCodeGen {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_simple_function() {
        let module = Module {
            name: "test".into(),
            items: vec![Item::Function(Function {
                name: "main".into(),
                params: vec![],
                return_type: Some(Type::Path(Path {
                    segments: vec![PathSegment {
                        ident: "i32".into(),
                        generics: vec![],
                    }],
                })),
                body: Block {
                    stmts: vec![Stmt::Expr(Expr::Literal(Literal::Integer(0)))],
                    span: 0..10,
                },
                visibility: Visibility::Public,
                is_async: false,
                is_unsafe: false,
                is_extern: false,
                abi: None,
                generics: vec![],
                ffi_attrs: FfiAttributes::default(),
                span: 0..50,
            })],
            span: 0..100,
        };

        let mut codegen = CCodeGen::new();
        let result = codegen
            .generate(&module)
            .expect("codegen should succeed in test");

        assert!(result.contains("int32_t main()"));
        assert!(result.contains("return 0;"));
    }

    #[test]
    fn test_generate_extern_block() {
        let module = Module {
            name: "test".into(),
            items: vec![Item::ExternBlock(ExternBlock {
                abi: "C".to_string(),
                items: vec![ExternItem::Function(Function {
                    name: "external_func".into(),
                    params: vec![Param {
                        name: "x".into(),
                        ty: Type::Path(Path {
                            segments: vec![PathSegment {
                                ident: "i32".into(),
                                generics: vec![],
                            }],
                        }),
                        is_mut: false,
                        span: 0..10,
                    }],
                    return_type: Some(Type::Path(Path {
                        segments: vec![PathSegment {
                            ident: "i32".into(),
                            generics: vec![],
                        }],
                    })),
                    body: Block {
                        stmts: vec![],
                        span: 0..0,
                    },
                    visibility: Visibility::Private,
                    is_async: false,
                    is_unsafe: false,
                    is_extern: true,
                    abi: Some("C".to_string()),
                    generics: vec![],
                    ffi_attrs: FfiAttributes::default(),
                    span: 0..50,
                })],
                span: 0..100,
            })],
            span: 0..100,
        };

        let mut codegen = CCodeGen::new();
        let result = codegen
            .generate(&module)
            .expect("codegen should succeed in test");

        assert!(result.contains("extern int32_t external_func(int32_t x)"));
    }

    #[test]
    fn test_generate_callback() {
        let module = Module {
            name: "test".into(),
            items: vec![Item::Callback(CallbackDef {
                name: "EventHandler".into(),
                params: vec![Param {
                    name: "event".into(),
                    ty: Type::Path(Path {
                        segments: vec![PathSegment {
                            ident: "i32".into(),
                            generics: vec![],
                        }],
                    }),
                    is_mut: false,
                    span: 0..10,
                }],
                return_type: None,
                abi: "C".to_string(),
                visibility: Visibility::Public,
                span: 0..50,
            })],
            span: 0..100,
        };

        let mut codegen = CCodeGen::new();
        let result = codegen
            .generate(&module)
            .expect("codegen should succeed in test");

        assert!(result.contains("typedef void (*EventHandler)(int32_t event)"));
    }
}
