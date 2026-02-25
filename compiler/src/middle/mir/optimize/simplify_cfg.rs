//! Control Flow Graph Simplification
//!
//! This pass simplifies the control flow graph by:
//! - Merging consecutive goto blocks
//! - Removing unreachable blocks
//! - Simplifying switch instructions

use crate::middle::mir::optimize::MirPass;
use crate::middle::mir::*;
use std::collections::{HashMap, HashSet};

/// CFG simplification optimization
#[derive(Debug, Clone, Copy)]
pub struct SimplifyCfg;

impl MirPass for SimplifyCfg {
    fn name(&self) -> &'static str {
        "simplify_cfg"
    }

    fn run(&self, body: &mut MirBody) {
        // Find unreachable blocks
        let reachable = find_reachable_blocks(body);

        // Remove unreachable blocks
        remove_unreachable_blocks(body, &reachable);

        // Merge consecutive goto blocks
        merge_goto_blocks(body);

        // Simplify switch instructions with single target
        simplify_switches(body);
    }
}

/// Find all reachable blocks from the entry
fn find_reachable_blocks(body: &MirBody) -> HashSet<BasicBlock> {
    let mut reachable = HashSet::new();
    let mut worklist = vec![BasicBlock(0)]; // Start from entry block

    while let Some(block) = worklist.pop() {
        if reachable.insert(block) {
            if let Some(block_data) = body.basic_blocks.get(block.index()) {
                if let Some(ref terminator) = block_data.terminator {
                    for target in terminator_successors(terminator) {
                        worklist.push(target);
                    }
                }
            }
        }
    }

    reachable
}

/// Get successor blocks from a terminator
fn terminator_successors(terminator: &Terminator) -> Vec<BasicBlock> {
    match &terminator.kind {
        TerminatorKind::Goto { target } => vec![*target],
        TerminatorKind::SwitchInt {
            targets, otherwise, ..
        } => {
            let mut succs: Vec<_> = targets.iter().map(|(_, b)| *b).collect();
            succs.push(*otherwise);
            succs
        }
        TerminatorKind::Call { target, .. } => target.map(|t| vec![t]).unwrap_or_default(),
        TerminatorKind::Return => vec![],
        _ => vec![],
    }
}

/// Remove unreachable blocks
fn remove_unreachable_blocks(body: &mut MirBody, reachable: &HashSet<BasicBlock>) {
    // Create a mapping from old block indices to new ones
    let mut new_indices: HashMap<BasicBlock, BasicBlock> = HashMap::new();
    let mut new_blocks = Vec::new();

    for (old_idx, block) in body.basic_blocks.drain(..).enumerate() {
        let old_block = BasicBlock(old_idx as u32);
        if reachable.contains(&old_block) {
            let new_idx = new_blocks.len();
            new_indices.insert(old_block, BasicBlock(new_idx as u32));
            new_blocks.push(block);
        }
    }

    body.basic_blocks = new_blocks;

    // Update all block references
    for block in body.basic_blocks.iter_mut() {
        if let Some(ref mut terminator) = block.terminator {
            update_terminator_blocks(terminator, &new_indices);
        }
    }
}

/// Update block references in a terminator
fn update_terminator_blocks(
    terminator: &mut Terminator,
    mapping: &HashMap<BasicBlock, BasicBlock>,
) {
    match &mut terminator.kind {
        TerminatorKind::Goto { target } => {
            if let Some(&new) = mapping.get(target) {
                *target = new;
            }
        }
        TerminatorKind::SwitchInt {
            targets, otherwise, ..
        } => {
            for (_, target) in targets.iter_mut() {
                if let Some(&new) = mapping.get(target) {
                    *target = new;
                }
            }
            if let Some(&new) = mapping.get(otherwise) {
                *otherwise = new;
            }
        }
        TerminatorKind::Call { target, .. } => {
            if let Some(ref mut t) = target {
                if let Some(&new) = mapping.get(t) {
                    *t = new;
                }
            }
        }
        _ => {}
    }
}

/// Merge consecutive goto blocks
fn merge_goto_blocks(body: &mut MirBody) {
    loop {
        let mut merged = false;

        for block_idx in 0..body.basic_blocks.len() {
            let block = BasicBlock(block_idx as u32);

            // Check if this block is just a goto
            if let Some(target) = is_simple_goto(&body.basic_blocks[block_idx]) {
                // Check if we can merge
                if can_merge_block(body, block, target) {
                    // Merge the target block into this one
                    merge_blocks(body, block, target);
                    merged = true;
                    break;
                }
            }
        }

        if !merged {
            break;
        }
    }
}

/// Check if a block is a simple goto
fn is_simple_goto(block: &BasicBlockData) -> Option<BasicBlock> {
    if !block.statements.is_empty() {
        return None;
    }

    if let Some(ref terminator) = block.terminator {
        if let TerminatorKind::Goto { target } = terminator.kind {
            return Some(target);
        }
    }

    None
}

/// Check if we can merge two blocks
fn can_merge_block(body: &MirBody, from: BasicBlock, to: BasicBlock) -> bool {
    // Don't merge if the target has multiple predecessors
    let predecessors = count_predecessors(body, to);
    predecessors == 1 && from.index() + 1 == to.index()
}

/// Count predecessors of a block
fn count_predecessors(body: &MirBody, target: BasicBlock) -> usize {
    let mut count = 0;
    for block in &body.basic_blocks {
        if let Some(ref terminator) = block.terminator {
            for succ in terminator_successors(terminator) {
                if succ == target {
                    count += 1;
                }
            }
        }
    }
    count
}

/// Merge two blocks
fn merge_blocks(body: &mut MirBody, from: BasicBlock, to: BasicBlock) {
    // Remove the goto terminator from the from block
    if let Some(block) = body.basic_blocks.get_mut(from.index()) {
        block.terminator = None;
    }

    // Append statements from the target block
    // Check index bounds before removing
    let to_idx = to.index();
    if to_idx >= body.basic_blocks.len() {
        eprintln!(
            "Warning: Target block index {} out of bounds (len: {})",
            to_idx,
            body.basic_blocks.len()
        );
        return;
    }
    let target_block = body.basic_blocks.remove(to_idx);
    if let Some(block) = body.basic_blocks.get_mut(from.index()) {
        block.statements.extend(target_block.statements);
        block.terminator = target_block.terminator;
    }
}

/// Simplify switch instructions
fn simplify_switches(body: &mut MirBody) {
    for block in body.basic_blocks.iter_mut() {
        if let Some(ref mut terminator) = block.terminator {
            if let TerminatorKind::SwitchInt {
                ref targets,
                ref otherwise,
                ..
            } = terminator.kind
            {
                // If all targets are the same, replace with goto
                let all_targets: HashSet<_> = targets
                    .iter()
                    .map(|(_, b)| *b)
                    .chain(std::iter::once(*otherwise))
                    .collect();

                if all_targets.len() == 1 {
                    // Get the single target safely
                    let target = match all_targets.iter().next() {
                        Some(&t) => t,
                        None => continue, // Should not happen, but handle gracefully
                    };
                    terminator.kind = TerminatorKind::Goto { target };
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_unreachable_blocks() {
        let mut body = MirBody::new(0, 0..100);

        // Block 0: entry, has some statements so it won't be merged
        let mut block0 = BasicBlockData::new();
        block0.statements.push(Statement::Nop);
        block0.terminator = Some(Terminator {
            kind: TerminatorKind::Goto {
                target: BasicBlock(1),
            },
            span: 0..10,
        });
        body.basic_blocks.push(block0);

        // Block 1: reachable, has statements so it won't be merged
        let mut block1 = BasicBlockData::new();
        block1.statements.push(Statement::Nop);
        block1.terminator = Some(Terminator {
            kind: TerminatorKind::Return,
            span: 0..10,
        });
        body.basic_blocks.push(block1);

        // Block 2: unreachable (not referenced by any block)
        let mut block2 = BasicBlockData::new();
        block2.statements.push(Statement::Nop);
        block2.terminator = Some(Terminator {
            kind: TerminatorKind::Return,
            span: 0..10,
        });
        body.basic_blocks.push(block2);

        // Run simplification
        let pass = SimplifyCfg;
        pass.run(&mut body);

        // Block 2 should be removed, leaving 2 blocks
        assert_eq!(body.basic_blocks.len(), 2);
    }

    #[test]
    fn test_simplify_switch() {
        let mut body = MirBody::new(0, 0..100);

        let mut block = BasicBlockData::new();
        block.terminator = Some(Terminator {
            kind: TerminatorKind::SwitchInt {
                discr: Operand::Constant(Constant::Scalar(Scalar::Int(0, IntType::I64))),
                switch_ty: crate::ast::Type::Unit,
                targets: vec![(0, BasicBlock(1)), (1, BasicBlock(1))],
                otherwise: BasicBlock(1),
            },
            span: 0..10,
        });
        body.basic_blocks.push(block);

        // Target block
        let mut target = BasicBlockData::new();
        target.terminator = Some(Terminator {
            kind: TerminatorKind::Return,
            span: 0..10,
        });
        body.basic_blocks.push(target);

        // Run simplification
        let pass = SimplifyCfg;
        pass.run(&mut body);

        // Switch should be simplified to goto
        if let Some(ref terminator) = body.basic_blocks[0].terminator {
            assert!(matches!(terminator.kind, TerminatorKind::Goto { .. }));
        } else {
            panic!("Expected terminator");
        }
    }
}
