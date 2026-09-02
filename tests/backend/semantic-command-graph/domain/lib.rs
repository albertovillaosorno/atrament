// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - MIT
// Confidential:
//   - false
// License-File:
//   - LICENSE-MIT
//
// Boundary-Contract:
// - Owns:
//   - Regression evidence for transport-neutral command dependency validation.
// - Must-Not:
//   - Parse command wire formats, mutate notebooks, or infer missing edges.
// - Allows:
//   - Inputs: Typed command nodes and dependency identities.
//   - Outputs: Assertions over valid and typed invalid command graphs.
//   - Side effects: None beyond test-process allocation.
// - Split-When:
//   - Batch normalization or impact-graph fixtures require separate authority.
// - Merge-When:
//   - Command dependency validation no longer has an independent contract.
// - Summary:
//   - Proves command dependency validity without identity-format assumptions.
// - Description:
//   - Covers unique IDs, references, cycles, order preservation, and depth.
// - Usage:
//   - Compile directly against the semantic-command-graph domain component.
// - Defaults:
//   - Large acyclic graphs validate iteratively without recursive traversal.
//
use atrament_semantic_command_graph::{
    CommandGraphError, CommandNode, validate_command_graph,
};

fn node(id: u32, dependencies: &[u32]) -> CommandNode<u32> {
    CommandNode {
        dependencies: dependencies.to_vec(),
        id,
    }
}

#[test]
fn empty_chain_and_diamond_graphs_are_valid() {
    assert_eq!(validate_command_graph::<u32>(&[]), Ok(()));
    assert_eq!(
        validate_command_graph(&[
            node(1, &[]),
            node(2, &[1]),
            node(3, &[1]),
            node(4, &[2, 3]),
        ]),
        Ok(()),
    );
}

#[test]
fn duplicate_command_identity_is_typed() {
    assert_eq!(
        validate_command_graph(&[node(4, &[]), node(4, &[])]),
        Err(CommandGraphError::DuplicateIdentity { command: 4 }),
    );
}

#[test]
fn missing_dependency_is_typed() {
    assert_eq!(
        validate_command_graph(&[node(1, &[]), node(2, &[9])]),
        Err(CommandGraphError::MissingDependency {
            command: 2,
            dependency: 9,
        }),
    );
}

#[test]
fn direct_self_dependency_is_typed() {
    assert_eq!(
        validate_command_graph(&[node(7, &[7])]),
        Err(CommandGraphError::SelfDependency { command: 7 }),
    );
}

#[test]
fn cycles_reject_without_reordering_input() {
    let nodes =
        vec![node(9, &[7]), node(7, &[8]), node(8, &[9]), node(10, &[9])];
    let before = nodes.clone();
    assert_eq!(
        validate_command_graph(&nodes),
        Err(CommandGraphError::Cycle)
    );
    assert_eq!(nodes, before);
}

#[test]
fn command_identity_representation_is_generic() {
    let nodes = [
        CommandNode {
            dependencies: Vec::new(),
            id: "draft",
        },
        CommandNode {
            dependencies: vec!["draft"],
            id: "review",
        },
    ];
    assert_eq!(validate_command_graph(&nodes), Ok(()));
}

#[test]
fn duplicate_dependency_edges_do_not_invent_a_new_error_class() {
    assert_eq!(
        validate_command_graph(&[node(1, &[]), node(2, &[1, 1])]),
        Ok(()),
    );
}

#[test]
fn one_hundred_thousand_command_chain_is_iterative() {
    let mut nodes = Vec::with_capacity(100_000);
    nodes.push(node(0, &[]));
    for id in 1_u32..100_000_u32 {
        nodes.push(node(id, &[id.saturating_sub(1)]));
    }
    assert_eq!(validate_command_graph(&nodes), Ok(()));
}
