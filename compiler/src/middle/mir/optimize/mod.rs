//! MIR Optimization Module
//!
//! This module provides various optimization passes for MIR:
//! - Constant folding: Evaluate constant expressions at compile time
//! - Constant propagation: Propagate constant values to their uses
//! - Dead code elimination: Remove unused statements
//! - Simplify CFG: Merge basic blocks and remove unreachable code
//! - Common subexpression elimination: Remove redundant computations
//! - Advanced optimizations: Inter-block analysis, aggressive DCE

use crate::middle::mir::*;

pub mod const_fold;
pub mod const_prop;
pub mod const_prop_advanced;
pub mod cse;
pub mod dead_code;
pub mod dead_code_advanced;
pub mod inline;
pub mod licm;
pub mod simplify_cfg;

pub use const_fold::ConstantFolding;
pub use const_prop::ConstantPropagation;
pub use const_prop_advanced::AdvancedConstantPropagation;
pub use cse::CommonSubexpressionElimination;
pub use dead_code::DeadCodeElimination;
pub use dead_code_advanced::AdvancedDeadCodeElimination;
pub use inline::FunctionInlining;
pub use licm::LoopInvariantCodeMotion;
pub use simplify_cfg::SimplifyCfg;

/// A MIR optimization pass
pub trait MirPass {
    /// Name of the pass
    fn name(&self) -> &'static str;

    /// Run the pass on a MIR body
    fn run(&self, body: &mut MirBody);
}

/// Optimization level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptLevel {
    NoOptimization,
    Minimal,
    Default,
    Aggressive,
}

/// Optimization pipeline
pub struct OptimizationPipeline {
    passes: Vec<Box<dyn MirPass>>,
    level: OptLevel,
}

impl OptimizationPipeline {
    /// Create an empty pipeline
    pub fn new() -> Self {
        Self {
            passes: Vec::new(),
            level: OptLevel::Default,
        }
    }

    /// Add a pass to the pipeline
    pub fn add_pass<P: MirPass + 'static>(&mut self, pass: P) {
        self.passes.push(Box::new(pass));
    }

    /// Run all passes on a MIR body
    pub fn run(&self, body: &mut MirBody) {
        for pass in &self.passes {
            pass.run(body);
        }
    }

    /// Create a default optimization pipeline
    pub fn default_pipeline() -> Self {
        let mut pipeline = Self::new();
        pipeline.level = OptLevel::Default;
        pipeline.add_pass(SimplifyCfg);
        pipeline.add_pass(ConstantFolding);
        pipeline.add_pass(ConstantPropagation);
        pipeline.add_pass(CommonSubexpressionElimination::new());
        pipeline.add_pass(DeadCodeElimination);
        pipeline.add_pass(SimplifyCfg);
        pipeline
    }

    /// Create a pipeline for debug builds (minimal optimizations)
    pub fn debug_pipeline() -> Self {
        let mut pipeline = Self::new();
        pipeline.level = OptLevel::Minimal;
        pipeline.add_pass(SimplifyCfg);
        pipeline
    }

    /// Create a pipeline for release builds (full optimizations)
    pub fn release_pipeline() -> Self {
        let mut pipeline = Self::new();
        pipeline.level = OptLevel::Aggressive;
        
        pipeline.add_pass(SimplifyCfg);
        pipeline.add_pass(FunctionInlining::new());
        pipeline.add_pass(LoopInvariantCodeMotion::new());
        
        for _ in 0..3 {
            pipeline.add_pass(ConstantFolding);
            pipeline.add_pass(AdvancedConstantPropagation::new());
            pipeline.add_pass(CommonSubexpressionElimination::new());
        }
        
        pipeline.add_pass(AdvancedDeadCodeElimination::new());
        pipeline.add_pass(SimplifyCfg);
        pipeline
    }

    /// Create a pipeline based on optimization level (0-3)
    pub fn from_level(level: u8) -> Self {
        match level {
            0 => Self::no_optimization(),
            1 => Self::debug_pipeline(),
            2 => Self::default_pipeline(),
            3 | _ => Self::release_pipeline(),
        }
    }

    /// Create a pipeline with no optimizations
    pub fn no_optimization() -> Self {
        Self {
            passes: Vec::new(),
            level: OptLevel::NoOptimization,
        }
    }

    /// Get the optimization level
    pub fn level(&self) -> OptLevel {
        self.level
    }

    /// Get the number of passes
    pub fn pass_count(&self) -> usize {
        self.passes.len()
    }
}

impl Default for OptimizationPipeline {
    fn default() -> Self {
        Self::default_pipeline()
    }
}

/// Helper trait for visiting MIR
pub trait MirVisitor {
    fn visit_body(&mut self, body: &MirBody) {
        for (block_idx, block) in body.basic_blocks.iter().enumerate() {
            self.visit_basic_block(BasicBlock(block_idx as u32), block);
        }
    }

    fn visit_basic_block(&mut self, block: BasicBlock, data: &BasicBlockData) {
        for stmt in &data.statements {
            self.visit_statement(block, stmt);
        }
        if let Some(ref terminator) = data.terminator {
            self.visit_terminator(block, terminator);
        }
    }

    fn visit_statement(&mut self, _block: BasicBlock, stmt: &Statement) {
        match stmt {
            Statement::Assign(place, rvalue) => {
                self.visit_assign(place, rvalue);
            }
            Statement::StorageLive(local) => {
                self.visit_storage_live(*local);
            }
            Statement::StorageDead(local) => {
                self.visit_storage_dead(*local);
            }
            Statement::Nop => {}
            Statement::InlineAsm(_) => {}
        }
    }

    fn visit_assign(&mut self, place: &Place, rvalue: &Rvalue) {
        self.visit_place(place);
        self.visit_rvalue(rvalue);
    }

    fn visit_rvalue(&mut self, rvalue: &Rvalue) {
        match rvalue {
            Rvalue::Use(operand) => self.visit_operand(operand),
            Rvalue::BinaryOp(_, left, right) => {
                self.visit_operand(left);
                self.visit_operand(right);
            }
            Rvalue::UnaryOp(_, operand) => {
                self.visit_operand(operand);
            }
            Rvalue::Aggregate(_, operands) => {
                for operand in operands {
                    self.visit_operand(operand);
                }
            }
            _ => {}
        }
    }

    fn visit_operand(&mut self, operand: &Operand) {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                self.visit_place(place);
            }
            Operand::Constant(constant) => {
                self.visit_constant(constant);
            }
        }
    }

    fn visit_place(&mut self, _place: &Place) {}

    fn visit_constant(&mut self, _constant: &Constant) {}

    fn visit_storage_live(&mut self, _local: Local) {}

    fn visit_storage_dead(&mut self, _local: Local) {}

    fn visit_terminator(&mut self, _block: BasicBlock, terminator: &Terminator) {
        match &terminator.kind {
            TerminatorKind::Goto { .. } => {}
            TerminatorKind::SwitchInt { discr, .. } => {
                self.visit_operand(discr);
            }
            TerminatorKind::Return => {}
            TerminatorKind::Call { func, args, .. } => {
                self.visit_operand(func);
                for arg in args {
                    self.visit_operand(arg);
                }
            }
            _ => {}
        }
    }
}

/// Helper trait for mutating MIR
pub trait MirMutVisitor {
    fn visit_body(&mut self, body: &mut MirBody) {
        for block_idx in 0..body.basic_blocks.len() {
            let block = BasicBlock(block_idx as u32);
            self.visit_basic_block(body, block);
        }
    }

    fn visit_basic_block(&mut self, body: &mut MirBody, block: BasicBlock) {
        self.visit_statements(body, block);
        self.visit_terminator(body, block);
    }

    fn visit_statements(&mut self, body: &mut MirBody, block: BasicBlock) {
        let block_data = body.basic_block_mut(block);
        for stmt_idx in 0..block_data.statements.len() {
            self.visit_statement(body, block, stmt_idx);
        }
    }

    fn visit_statement(&mut self, body: &mut MirBody, block: BasicBlock, stmt_idx: usize);
    fn visit_terminator(&mut self, body: &mut MirBody, block: BasicBlock);
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPass;

    impl MirPass for TestPass {
        fn name(&self) -> &'static str {
            "test"
        }

        fn run(&self, _body: &mut MirBody) {
            // Do nothing
        }
    }

    #[test]
    fn test_pipeline_creation() {
        let pipeline = OptimizationPipeline::default_pipeline();
        assert!(!pipeline.passes.is_empty());
        assert_eq!(pipeline.level(), OptLevel::Default);
    }

    #[test]
    fn test_add_pass() {
        let mut pipeline = OptimizationPipeline::new();
        pipeline.add_pass(TestPass);
        assert_eq!(pipeline.pass_count(), 1);
    }

    #[test]
    fn test_optimization_levels() {
        let pipeline0 = OptimizationPipeline::from_level(0);
        assert_eq!(pipeline0.level(), OptLevel::NoOptimization);
        assert_eq!(pipeline0.pass_count(), 0);

        let pipeline1 = OptimizationPipeline::from_level(1);
        assert_eq!(pipeline1.level(), OptLevel::Minimal);

        let pipeline2 = OptimizationPipeline::from_level(2);
        assert_eq!(pipeline2.level(), OptLevel::Default);

        let pipeline3 = OptimizationPipeline::from_level(3);
        assert_eq!(pipeline3.level(), OptLevel::Aggressive);
    }

    #[test]
    fn test_release_pipeline() {
        let pipeline = OptimizationPipeline::release_pipeline();
        assert!(pipeline.pass_count() > 5);
    }
}
