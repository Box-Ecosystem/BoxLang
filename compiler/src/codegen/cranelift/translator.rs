//! MIR to Cranelift IR Translator
//!
//! This module translates BoxLang MIR into Cranelift IR,
//! which can then be compiled to machine code.

use crate::middle::mir::*;
use crate::codegen::cranelift::CraneliftError;

use cranelift::prelude::*;
use cranelift_module::{Module, Linkage, FuncId};
use cranelift_codegen::{Context, ir::{Function, UserFuncName}};

/// Translates MIR to Cranelift IR
pub struct MirToCraneliftTranslator<'a, M: Module> {
    /// The Cranelift module being built
    module: &'a mut M,
    /// Function builder context
    builder_ctx: FunctionBuilderContext,
}

use std::collections::HashMap;

impl<'a, M: Module> MirToCraneliftTranslator<'a, M> {
    /// Create a new translator
    pub fn new(module: &'a mut M) -> Self {
        Self {
            module,
            builder_ctx: FunctionBuilderContext::new(),
        }
    }

    /// Translate a MIR body to a Cranelift function
    pub fn translate_function(
        &mut self,
        name: &str,
        body: &MirBody,
    ) -> Result<FuncId, CraneliftError> {
        // Create function signature
        let mut sig = self.module.make_signature();
        
        // Add return type (if not unit)
        // For simplicity, assume i64 return type
        sig.returns.push(AbiParam::new(types::I64));
        
        // Add parameters
        for _ in 0..body.arg_count {
            sig.params.push(AbiParam::new(types::I64));
        }

        // Declare function
        let func_id = self.module
            .declare_function(name, Linkage::Local, &sig)
            .map_err(|e| CraneliftError::CompilationFailed(e.to_string()))?;

        // Build function body
        let mut func = Function::with_name_signature(
            UserFuncName::user(0, func_id.as_u32()),
            sig,
        );

        {
            let mut builder = FunctionBuilder::new(&mut func, &mut self.builder_ctx);
            
            // Create entry block
            let entry_block = builder.create_block();
            builder.switch_to_block(entry_block);
            builder.seal_block(entry_block);

            // Initialize parameters as variables
            let mut variables: HashMap<Local, Variable> = HashMap::new();
            let mut next_var = 0;
            
            for (i, _local) in (0..body.arg_count).enumerate() {
                let var = Variable::new(next_var);
                next_var += 1;
                variables.insert(Local((i + 1) as u32), var);
                
                let param_val = builder.block_params(entry_block)[i];
                builder.def_var(var, param_val);
            }

            // Translate all basic blocks
            translate_body(&mut builder, body, &mut variables, &mut next_var)?;
        }

        // Create context and define function in module
        let mut ctx = Context::for_function(func);
        self.module
            .define_function(func_id, &mut ctx)
            .map_err(|e| CraneliftError::CompilationFailed(e.to_string()))?;

        Ok(func_id)
    }
}

/// Translate a MIR body
fn translate_body(
    builder: &mut FunctionBuilder,
    body: &MirBody,
    variables: &mut HashMap<Local, Variable>,
    next_var: &mut usize,
) -> Result<(), CraneliftError> {
    let mut blocks: HashMap<BasicBlock, Block> = HashMap::new();

    // First pass: create all blocks
    for (idx, _) in body.basic_blocks.iter().enumerate() {
        let block = if idx == 0 {
            // Entry block already created
            builder.current_block().unwrap()
        } else {
            builder.create_block()
        };
        blocks.insert(BasicBlock(idx as u32), block);
    }

    // Second pass: translate blocks
    for (idx, block_data) in body.basic_blocks.iter().enumerate() {
        let block = blocks[&BasicBlock(idx as u32)];
        builder.switch_to_block(block);

        // Translate statements
        for stmt in &block_data.statements {
            translate_statement(builder, stmt, variables, next_var)?;
        }

        // Translate terminator
        if let Some(ref terminator) = block_data.terminator {
            translate_terminator(builder, terminator, &blocks, variables)?;
        }
    }

    Ok(())
}

/// Translate a statement
fn translate_statement(
    builder: &mut FunctionBuilder,
    stmt: &Statement,
    variables: &mut HashMap<Local, Variable>,
    next_var: &mut usize,
) -> Result<(), CraneliftError> {
    match stmt {
        Statement::Assign(place, rvalue) => {
            let value = translate_rvalue(builder, rvalue, variables, next_var)?;
            let var = get_or_create_variable(variables, next_var, place.local);
            builder.def_var(var, value);
        }
        Statement::StorageLive(_) |
        Statement::StorageDead(_) |
        Statement::Nop => {
            // These are for borrow checking, ignore in codegen
        }
        Statement::InlineAsm(_) => {
            return Err(CraneliftError::UnsupportedOperation(
                "inline assembly".to_string()
            ));
        }
    }
    Ok(())
}

/// Translate an rvalue to a Cranelift value
fn translate_rvalue(
    builder: &mut FunctionBuilder,
    rvalue: &Rvalue,
    variables: &mut HashMap<Local, Variable>,
    next_var: &mut usize,
) -> Result<Value, CraneliftError> {
    match rvalue {
        Rvalue::Use(operand) => {
            translate_operand(builder, operand, variables)
        }
        Rvalue::BinaryOp(op, left, right) => {
            let left_val = translate_operand(builder, left, variables)?;
            let right_val = translate_operand(builder, right, variables)?;
            translate_binop(builder, *op, left_val, right_val)
        }
        Rvalue::UnaryOp(op, operand) => {
            let val = translate_operand(builder, operand, variables)?;
            translate_unop(builder, *op, val)
        }
        Rvalue::Copy(place) |
        Rvalue::Move(place) => {
            // Load from place
            let var = get_or_create_variable(variables, next_var, place.local);
            Ok(builder.use_var(var))
        }
        Rvalue::Ref(_, _) => {
            // References not yet implemented
            Err(CraneliftError::UnsupportedOperation(
                "references".to_string()
            ))
        }
        Rvalue::AddressOf(_, _) => {
            Err(CraneliftError::UnsupportedOperation(
                "raw pointers".to_string()
            ))
        }
        Rvalue::Cast(_, operand, _) => {
            // For now, just pass through
            translate_operand(builder, operand, variables)
        }
        Rvalue::Len(_) |
        Rvalue::Discriminant(_) |
        Rvalue::Aggregate(_, _) => {
            Err(CraneliftError::UnsupportedOperation(
                "complex rvalue".to_string()
            ))
        }
    }
}

/// Translate a binary operation
fn translate_binop(
    builder: &mut FunctionBuilder,
    op: BinOp,
    left: Value,
    right: Value,
) -> Result<Value, CraneliftError> {
    let result = match op {
        BinOp::Add => builder.ins().iadd(left, right),
        BinOp::Sub => builder.ins().isub(left, right),
        BinOp::Mul => builder.ins().imul(left, right),
        BinOp::Div => builder.ins().sdiv(left, right),
        BinOp::Rem => builder.ins().srem(left, right),
        BinOp::BitAnd => builder.ins().band(left, right),
        BinOp::BitOr => builder.ins().bor(left, right),
        BinOp::BitXor => builder.ins().bxor(left, right),
        BinOp::Shl => builder.ins().ishl(left, right),
        BinOp::Shr => builder.ins().sshr(left, right),
        BinOp::Eq => {
            let cmp = builder.ins().icmp(IntCC::Equal, left, right);
            builder.ins().sextend(types::I64, cmp)
        }
        BinOp::Ne => {
            let cmp = builder.ins().icmp(IntCC::NotEqual, left, right);
            builder.ins().sextend(types::I64, cmp)
        }
        BinOp::Lt => {
            let cmp = builder.ins().icmp(IntCC::SignedLessThan, left, right);
            builder.ins().sextend(types::I64, cmp)
        }
        BinOp::Le => {
            let cmp = builder.ins().icmp(IntCC::SignedLessThanOrEqual, left, right);
            builder.ins().sextend(types::I64, cmp)
        }
        BinOp::Gt => {
            let cmp = builder.ins().icmp(IntCC::SignedGreaterThan, left, right);
            builder.ins().sextend(types::I64, cmp)
        }
        BinOp::Ge => {
            let cmp = builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, left, right);
            builder.ins().sextend(types::I64, cmp)
        }
        BinOp::Offset => {
            return Err(CraneliftError::UnsupportedOperation(
                "pointer offset".to_string()
            ));
        }
    };
    Ok(result)
}

/// Translate a unary operation
fn translate_unop(
    builder: &mut FunctionBuilder,
    op: UnOp,
    val: Value,
) -> Result<Value, CraneliftError> {
    match op {
        UnOp::Neg => {
            let zero = builder.ins().iconst(types::I64, 0);
            Ok(builder.ins().isub(zero, val))
        }
        UnOp::Not => {
            Ok(builder.ins().bnot(val))
        }
    }
}

/// Translate an operand
fn translate_operand(
    builder: &mut FunctionBuilder,
    operand: &Operand,
    variables: &HashMap<Local, Variable>,
) -> Result<Value, CraneliftError> {
    match operand {
        Operand::Copy(place) |
        Operand::Move(place) => {
            let var = variables.get(&place.local)
                .copied()
                .unwrap_or_else(|| Variable::new(0));
            Ok(builder.use_var(var))
        }
        Operand::Constant(constant) => {
            translate_constant(builder, constant)
        }
    }
}

/// Translate a constant
fn translate_constant(
    builder: &mut FunctionBuilder,
    constant: &Constant,
) -> Result<Value, CraneliftError> {
    match constant {
        Constant::Scalar(scalar) => {
            match scalar {
                Scalar::Int(val, _) => {
                    Ok(builder.ins().iconst(types::I64, *val as i64))
                }
                Scalar::Float(val, _) => {
                    Ok(builder.ins().f64const(*val))
                }
                Scalar::Pointer(_) => {
                    Err(CraneliftError::UnsupportedOperation(
                        "pointer constants".to_string()
                    ))
                }
            }
        }
        Constant::ZST => {
            // Zero-sized type - use dummy value
            Ok(builder.ins().iconst(types::I64, 0))
        }
    }
}

/// Translate a terminator
fn translate_terminator(
    builder: &mut FunctionBuilder,
    terminator: &Terminator,
    blocks: &HashMap<BasicBlock, Block>,
    variables: &HashMap<Local, Variable>,
) -> Result<(), CraneliftError> {
    match &terminator.kind {
        TerminatorKind::Goto { target } => {
            let block = blocks[target];
            builder.ins().jump(block, &[]);
        }
        TerminatorKind::SwitchInt { discr, targets, otherwise, .. } => {
            let discr_val = translate_operand(builder, discr, variables)?;
            let otherwise_block = blocks[otherwise];
            
            let mut switch = cranelift::frontend::Switch::new();
            for (val, target) in targets {
                let block = blocks[target];
                switch.set_entry(*val as u128, block);
            }
            switch.emit(builder, discr_val, otherwise_block);
        }
        TerminatorKind::Return => {
            // Return the return place value
            let return_var = variables.get(&Local::RETURN_PLACE)
                .copied()
                .unwrap_or_else(|| Variable::new(0));
            let return_val = builder.use_var(return_var);
            builder.ins().return_(&[return_val]);
        }
        TerminatorKind::Unwind => {
            // For now, just return
            builder.ins().return_(&[]);
        }
        TerminatorKind::Call { func, args, destination, target, .. } => {
            // Translate function call
            let func_val = translate_operand(builder, func, variables)?;
            let _arg_vals: Vec<_> = args.iter()
                .map(|arg| translate_operand(builder, arg, variables))
                .collect::<Result<_, _>>()?;
            
            // For now, indirect call not fully implemented
            // In a full implementation, we'd look up the function and call it
            
            // Store result in destination
            let dest_var = variables.get(&destination.local)
                .copied()
                .unwrap_or_else(|| Variable::new(0));
            builder.def_var(dest_var, func_val); // Placeholder
            
            // Continue to target
            if let Some(target_block) = target {
                let block = blocks[target_block];
                builder.ins().jump(block, &[]);
            }
        }
        TerminatorKind::Assert { cond, target, .. } => {
            let cond_val = translate_operand(builder, cond, variables)?;
            let target_block = blocks[target];
            
            // Convert condition to bool (non-zero = true)
            let zero = builder.ins().iconst(types::I64, 0);
            let is_true = builder.ins().icmp(IntCC::NotEqual, cond_val, zero);
            
            // Create fail block that returns
            let fail_block = builder.create_block();
            builder.switch_to_block(fail_block);
            builder.ins().return_(&[]);
            
            // Branch
            builder.switch_to_block(builder.current_block().unwrap());
            builder.ins().brif(is_true, target_block, &[], fail_block, &[]);
        }
    }
    Ok(())
}

/// Get or create a variable for a local
fn get_or_create_variable(
    variables: &mut HashMap<Local, Variable>,
    next_var: &mut usize,
    local: Local,
) -> Variable {
    if let Some(&var) = variables.get(&local) {
        var
    } else {
        let var = Variable::new(*next_var);
        *next_var += 1;
        variables.insert(local, var);
        var
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a Cranelift module, which is complex to set up
    // In a real implementation, we'd use a JIT module for testing
}
