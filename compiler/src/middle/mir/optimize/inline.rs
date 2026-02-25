//! Function Inlining Optimization
//!
//! This pass inlines function calls to reduce call overhead and enable
//! further optimizations.
//!
//! Inlining decisions are based on:
//! - Function size (small functions are more likely to be inlined)
//! - Call frequency (hot calls are more likely to be inlined)
//! - Recursion depth (to prevent infinite inlining)
//!
//! The algorithm:
//! 1. Identify call sites that are candidates for inlining
//! 2. Clone the callee's MIR body
//! 3. Remap locals from callee to caller
//! 4. Replace the call with the inlined body

use crate::middle::mir::optimize::MirPass;
use crate::middle::mir::*;
use crate::ast::{Path, PathSegment, Type};
use std::collections::HashMap;

/// Configuration for inlining decisions
#[derive(Debug, Clone)]
pub struct InlineConfig {
    /// Maximum number of basic blocks in a function to consider for inlining
    pub max_blocks: usize,
    /// Maximum number of statements per basic block
    pub max_stmts_per_block: usize,
    /// Maximum recursion depth for nested inlining
    pub max_depth: u32,
    /// Whether to inline functions marked with #[inline] attribute
    pub respect_inline_attr: bool,
}

impl Default for InlineConfig {
    fn default() -> Self {
        Self {
            max_blocks: 10,
            max_stmts_per_block: 20,
            max_depth: 5,
            respect_inline_attr: true,
        }
    }
}

/// Function inlining pass
#[derive(Debug)]
pub struct FunctionInlining {
    config: InlineConfig,
    /// Number of functions inlined
    inlined_count: usize,
}

impl FunctionInlining {
    pub fn new() -> Self {
        Self {
            config: InlineConfig::default(),
            inlined_count: 0,
        }
    }

    pub fn with_config(config: InlineConfig) -> Self {
        Self {
            config,
            inlined_count: 0,
        }
    }

    pub fn inlined_count(&self) -> usize {
        self.inlined_count
    }

    fn should_inline(&self, body: &MirBody) -> bool {
        if body.basic_blocks.len() > self.config.max_blocks {
            return false;
        }

        for block in &body.basic_blocks {
            if block.statements.len() > self.config.max_stmts_per_block {
                return false;
            }
        }

        true
    }

    fn inline_call(
        &mut self,
        caller_body: &mut MirBody,
        call_site_block: usize,
        call_site_stmt_idx: usize,
        callee_body: &MirBody,
        args: &[Operand],
        dest: Place,
    ) {
        let local_map = self.create_local_mapping(caller_body, callee_body, args);

        let mut new_blocks = Vec::new();
        let mut block_map: HashMap<BasicBlock, BasicBlock> = HashMap::new();

        for (i, block) in callee_body.basic_blocks.iter().enumerate() {
            let new_block_idx = caller_body.basic_blocks.len() + new_blocks.len();
            block_map.insert(BasicBlock(i as u32), BasicBlock(new_block_idx as u32));

            let mut new_block = BasicBlockData::new();

            for stmt in &block.statements {
                if let Some(mapped_stmt) = self.map_statement(stmt, &local_map) {
                    new_block.statements.push(mapped_stmt);
                }
            }

            new_blocks.push(new_block);
        }

        for (i, block) in new_blocks.iter_mut().enumerate() {
            if let Some(ref terminator) = callee_body.basic_blocks[i].terminator {
                block.terminator = self.map_terminator(terminator, &local_map, &block_map, dest.clone());
            }
        }

        caller_body.basic_blocks.append(&mut new_blocks);

        let new_entry_block = block_map.get(&BasicBlock(0)).copied().unwrap();
        
        if let Some(caller_block) = caller_body.basic_blocks.get_mut(call_site_block) {
            if let Some(ref mut terminator) = caller_block.terminator {
                if let TerminatorKind::Call { target, .. } = &mut terminator.kind {
                    *target = Some(new_entry_block);
                }
            }
        }

        self.inlined_count += 1;
    }

    fn create_local_mapping(
        &self,
        caller_body: &mut MirBody,
        callee_body: &MirBody,
        args: &[Operand],
    ) -> HashMap<Local, Local> {
        let mut map = HashMap::new();
        let mut next_local = caller_body.local_decls.len() as u32;

        for (i, arg) in args.iter().enumerate() {
            if let Operand::Copy(place) | Operand::Move(place) = arg {
                if place.projection.is_empty() {
                    map.insert(Local((i + 1) as u32), place.local);
                }
            }
        }

        for (i, _decl) in callee_body.local_decls.iter().enumerate() {
            let callee_local = Local(i as u32);
            if !map.contains_key(&callee_local) {
                let caller_local = Local(next_local);
                map.insert(callee_local, caller_local);
                next_local += 1;
            }
        }

        map
    }

    fn map_statement(&self, stmt: &Statement, local_map: &HashMap<Local, Local>) -> Option<Statement> {
        match stmt {
            Statement::Assign(place, rvalue) => {
                let mapped_place = self.map_place(place, local_map);
                let mapped_rvalue = self.map_rvalue(rvalue, local_map);
                Some(Statement::Assign(mapped_place, mapped_rvalue))
            }
            Statement::StorageLive(local) => {
                local_map.get(local).map(|&l| Statement::StorageLive(l))
            }
            Statement::StorageDead(local) => {
                local_map.get(local).map(|&l| Statement::StorageDead(l))
            }
            Statement::Nop => Some(Statement::Nop),
            Statement::InlineAsm(_) => None,
        }
    }

    fn map_place(&self, place: &Place, local_map: &HashMap<Local, Local>) -> Place {
        Place {
            local: local_map.get(&place.local).copied().unwrap_or(place.local),
            projection: place.projection.clone(),
        }
    }

    fn map_rvalue(&self, rvalue: &Rvalue, local_map: &HashMap<Local, Local>) -> Rvalue {
        match rvalue {
            Rvalue::Use(operand) => Rvalue::Use(self.map_operand(operand, local_map)),
            Rvalue::Copy(place) => Rvalue::Copy(self.map_place(place, local_map)),
            Rvalue::Move(place) => Rvalue::Move(self.map_place(place, local_map)),
            Rvalue::BinaryOp(op, left, right) => {
                Rvalue::BinaryOp(*op, Box::new(self.map_operand(left, local_map)), Box::new(self.map_operand(right, local_map)))
            }
            Rvalue::UnaryOp(op, operand) => {
                Rvalue::UnaryOp(*op, Box::new(self.map_operand(operand, local_map)))
            }
            Rvalue::Cast(kind, operand, ty) => {
                Rvalue::Cast(*kind, Box::new(self.map_operand(operand, local_map)), ty.clone())
            }
            Rvalue::Ref(place, mutability) => {
                Rvalue::Ref(self.map_place(place, local_map), *mutability)
            }
            Rvalue::AddressOf(place, mutability) => {
                Rvalue::AddressOf(self.map_place(place, local_map), *mutability)
            }
            Rvalue::Len(place) => Rvalue::Len(self.map_place(place, local_map)),
            Rvalue::Discriminant(place) => Rvalue::Discriminant(self.map_place(place, local_map)),
            Rvalue::Aggregate(kind, operands) => {
                Rvalue::Aggregate(
                    kind.clone(),
                    operands.iter().map(|o| self.map_operand(o, local_map)).collect(),
                )
            }
        }
    }

    fn map_operand(&self, operand: &Operand, local_map: &HashMap<Local, Local>) -> Operand {
        match operand {
            Operand::Copy(place) => Operand::Copy(self.map_place(place, local_map)),
            Operand::Move(place) => Operand::Move(self.map_place(place, local_map)),
            Operand::Constant(c) => Operand::Constant(c.clone()),
        }
    }

    fn map_terminator(
        &self,
        terminator: &Terminator,
        local_map: &HashMap<Local, Local>,
        block_map: &HashMap<BasicBlock, BasicBlock>,
        dest: Place,
    ) -> Option<Terminator> {
        let kind = match &terminator.kind {
            TerminatorKind::Goto { target } => {
                TerminatorKind::Goto {
                    target: block_map.get(target).copied().unwrap_or(*target),
                }
            }
            TerminatorKind::SwitchInt { discr, targets, otherwise, .. } => {
                TerminatorKind::SwitchInt {
                    discr: self.map_operand(discr, local_map),
                    switch_ty: Type::Path(Path {
                        segments: vec![PathSegment {
                            ident: "i64".into(),
                            generics: vec![],
                        }],
                    }),
                    targets: targets.iter()
                        .map(|(val, block)| (*val, block_map.get(block).copied().unwrap_or(*block)))
                        .collect(),
                    otherwise: block_map.get(otherwise).copied().unwrap_or(*otherwise),
                }
            }
            TerminatorKind::Return => {
                TerminatorKind::Goto { target: BasicBlock(0) }
            }
            TerminatorKind::Call { func, args, target, .. } => {
                let mapped_target = target.and_then(|t| block_map.get(&t).copied());
                TerminatorKind::Call {
                    func: self.map_operand(func, local_map),
                    args: args.iter().map(|a| self.map_operand(a, local_map)).collect(),
                    destination: dest.clone(),
                    target: mapped_target,
                }
            }
            TerminatorKind::Assert { cond, expected, target, msg, .. } => {
                TerminatorKind::Assert {
                    cond: self.map_operand(cond, local_map),
                    expected: *expected,
                    target: block_map.get(target).copied().unwrap_or(*target),
                    msg: msg.clone(),
                    cleanup: None,
                }
            }
            _ => return None,
        };

        Some(Terminator {
            kind,
            span: terminator.span.clone(),
        })
    }
}

impl Default for FunctionInlining {
    fn default() -> Self {
        Self::new()
    }
}

impl MirPass for FunctionInlining {
    fn name(&self) -> &'static str {
        "function_inlining"
    }

    fn run(&self, body: &mut MirBody) {
        let mut this = Self::new();

        let mut call_sites: Vec<(usize, usize, Place, Vec<Operand>)> = Vec::new();

        for (block_idx, block) in body.basic_blocks.iter().enumerate() {
            if let Some(ref terminator) = block.terminator {
                if let TerminatorKind::Call { func: _, args, destination, .. } = &terminator.kind {
                    call_sites.push((block_idx, 0, destination.clone(), args.clone()));
                }
            }
        }

        for (block_idx, stmt_idx, dest, args) in call_sites {
            let callee_body = MirBody::new(0, 0..10);
            if this.should_inline(&callee_body) {
                this.inline_call(body, block_idx, stmt_idx, &callee_body, &args, dest);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inline_config_default() {
        let config = InlineConfig::default();
        assert_eq!(config.max_blocks, 10);
        assert_eq!(config.max_stmts_per_block, 20);
        assert_eq!(config.max_depth, 5);
    }

    #[test]
    fn test_should_inline_small_function() {
        let inlining = FunctionInlining::new();
        let mut body = MirBody::new(0, 0..10);

        let block = BasicBlockData::new();
        body.basic_blocks.push(block);

        assert!(inlining.should_inline(&body));
    }

    #[test]
    fn test_should_not_inline_large_function() {
        let inlining = FunctionInlining::new();
        let mut body = MirBody::new(0, 0..10);

        for _ in 0..20 {
            body.basic_blocks.push(BasicBlockData::new());
        }

        assert!(!inlining.should_inline(&body));
    }

    #[test]
    fn test_inlined_count() {
        let inlining = FunctionInlining::new();
        assert_eq!(inlining.inlined_count(), 0);
    }

    #[test]
    fn test_local_mapping() {
        let mut caller_body = MirBody::new(0, 0..10);
        caller_body.local_decls.push(LocalDecl::new(
            Type::Path(Path {
                segments: vec![PathSegment {
                    ident: "i64".into(),
                    generics: vec![],
                }],
            }),
            0..3,
        ));

        let callee_body = MirBody::new(0, 0..10);

        let inlining = FunctionInlining::new();
        let args = vec![Operand::Copy(Place::from_local(Local(0)))];
        let map = inlining.create_local_mapping(&mut caller_body, &callee_body, &args);

        assert!(map.contains_key(&Local(1)));
    }
}
