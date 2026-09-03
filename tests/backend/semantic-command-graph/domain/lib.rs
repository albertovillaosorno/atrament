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
use std::collections::BTreeSet;

use atrament_semantic_command_graph::{
    BoundedDependencyRequirementsError, CommandGraphError,
    CommandGraphLimitError, CommandGraphLimits, CommandGraphSize, CommandNode,
    DependencyRequirementsError, DependencySelectionError,
    DependencySelectionSummary, DependencySummaryError,
    MissingDependencyRequirement, command_graph_size,
    dependency_selection_requirements,
    dependency_selection_requirements_bounded, dependency_selection_summary,
    validate_command_graph, validate_command_graph_limits,
    validate_dependency_closed_selection,
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
fn dependency_on_later_command_is_typed_without_reordering() {
    let nodes = [node(1, &[2]), node(2, &[])];
    let before = nodes.clone();
    assert_eq!(
        validate_command_graph(&nodes),
        Err(CommandGraphError::DependencyAfterCommand {
            command: 1,
            dependency: 2,
        }),
    );
    assert_eq!(nodes, before);
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
fn command_graph_size_counts_commands_and_explicit_edges_exactly() {
    let nodes = [node(1, &[]), node(2, &[1, 1]), node(3, &[1, 2])];
    assert_eq!(
        command_graph_size(&nodes),
        Ok(CommandGraphSize {
            commands: 3,
            dependency_edges: 4,
        }),
    );
    assert_eq!(
        command_graph_size::<u32>(&[]),
        Ok(CommandGraphSize {
            commands: 0,
            dependency_edges: 0,
        }),
    );
}

#[test]
fn graph_resource_limits_accept_exact_bounds_without_truncation() {
    let nodes = [node(1, &[]), node(2, &[1]), node(3, &[1, 2])];
    let before = nodes.clone();
    assert_eq!(
        validate_command_graph_limits(&nodes, CommandGraphLimits {
            commands: 3,
            dependency_edges: 3,
        },),
        Ok(CommandGraphSize {
            commands: 3,
            dependency_edges: 3,
        }),
    );
    assert_eq!(nodes, before);
}

#[test]
fn graph_resource_limits_reject_one_command_or_edge_over() {
    let nodes = [node(1, &[]), node(2, &[1]), node(3, &[1, 2])];
    assert_eq!(
        validate_command_graph_limits(&nodes, CommandGraphLimits {
            commands: 2,
            dependency_edges: 3,
        },),
        Err(CommandGraphLimitError::CommandCountExceeded {
            actual: 3,
            limit: 2,
        }),
    );
    assert_eq!(
        validate_command_graph_limits(&nodes, CommandGraphLimits {
            commands: 3,
            dependency_edges: 2,
        },),
        Err(CommandGraphLimitError::DependencyEdgeCountExceeded {
            actual: 3,
            limit: 2,
        }),
    );
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

#[test]
fn independent_interactive_subset_is_dependency_closed() {
    let nodes = [node(1, &[]), node(2, &[1]), node(3, &[])];
    let selected = BTreeSet::from([3]);
    assert_eq!(
        validate_dependency_closed_selection(&nodes, &selected),
        Ok(()),
    );
}

#[test]
fn dependent_interactive_subset_reports_omitted_dependency() {
    let nodes = [node(1, &[]), node(2, &[1]), node(3, &[])];
    let selected = BTreeSet::from([2]);
    assert_eq!(
        validate_dependency_closed_selection(&nodes, &selected),
        Err(DependencySelectionError::MissingRequiredDependency {
            command: 2,
            dependency: 1,
        }),
    );
    let closed = BTreeSet::from([1, 2]);
    assert_eq!(
        validate_dependency_closed_selection(&nodes, &closed),
        Ok(())
    );
}

#[test]
fn transitive_selection_never_silently_adds_missing_commands() {
    let nodes = [node(1, &[]), node(2, &[1]), node(3, &[2])];
    let selected = BTreeSet::from([2, 3]);
    let before = nodes.clone();
    assert_eq!(
        validate_dependency_closed_selection(&nodes, &selected),
        Err(DependencySelectionError::MissingRequiredDependency {
            command: 2,
            dependency: 1,
        }),
    );
    assert_eq!(nodes, before);
}

#[test]
fn unknown_selection_and_invalid_source_graph_are_typed() {
    let nodes = [node(1, &[]), node(2, &[1])];
    assert_eq!(
        validate_dependency_closed_selection(&nodes, &BTreeSet::from([9])),
        Err(DependencySelectionError::UnknownSelection { command: 9 }),
    );
    let invalid = [node(1, &[2]), node(2, &[])];
    assert_eq!(
        validate_dependency_closed_selection(&invalid, &BTreeSet::from([1])),
        Err(DependencySelectionError::Graph {
            reason: CommandGraphError::DependencyAfterCommand {
                command: 1,
                dependency: 2,
            },
        }),
    );
}

#[test]
fn dependency_requirements_report_complete_transitive_closure_without_mutation()
{
    let nodes = [node(1, &[]), node(2, &[1]), node(3, &[2]), node(4, &[1])];
    let selected = BTreeSet::from([3]);
    let before = selected.clone();
    assert_eq!(
        dependency_selection_requirements(&nodes, &selected),
        Ok(vec![
            MissingDependencyRequirement {
                command: 2,
                dependency: 1,
            },
            MissingDependencyRequirement {
                command: 3,
                dependency: 2,
            },
        ]),
    );
    assert_eq!(selected, before);
}

#[test]
fn dependency_requirements_are_empty_for_closed_or_independent_selection() {
    let nodes = [node(1, &[]), node(2, &[1]), node(3, &[])];
    assert_eq!(
        dependency_selection_requirements(&nodes, &BTreeSet::from([1, 2])),
        Ok(Vec::new()),
    );
    assert_eq!(
        dependency_selection_requirements(&nodes, &BTreeSet::from([3])),
        Ok(Vec::new()),
    );
}

#[test]
fn dependency_requirements_preserve_explicit_duplicate_edges() {
    let nodes = [node(1, &[]), node(2, &[1, 1])];
    assert_eq!(
        dependency_selection_requirements(&nodes, &BTreeSet::from([2])),
        Ok(vec![
            MissingDependencyRequirement {
                command: 2,
                dependency: 1,
            },
            MissingDependencyRequirement {
                command: 2,
                dependency: 1,
            },
        ]),
    );
}

#[test]
fn dependency_requirements_reject_unknown_selection_and_invalid_graph() {
    let nodes = [node(1, &[]), node(2, &[1])];
    assert_eq!(
        dependency_selection_requirements(&nodes, &BTreeSet::from([9])),
        Err(DependencyRequirementsError::UnknownSelection { command: 9 }),
    );
    let invalid = [node(1, &[2]), node(2, &[])];
    assert_eq!(
        dependency_selection_requirements(&invalid, &BTreeSet::from([1])),
        Err(DependencyRequirementsError::Graph {
            reason: CommandGraphError::DependencyAfterCommand {
                command: 1,
                dependency: 2,
            },
        }),
    );
}

#[test]
fn one_hundred_thousand_selection_requirements_are_iterative() {
    let mut nodes = Vec::with_capacity(100_000);
    nodes.push(node(0, &[]));
    for id in 1_u32..100_000_u32 {
        nodes.push(node(id, &[id.saturating_sub(1)]));
    }
    let selected = BTreeSet::from([99_999]);
    let before = selected.clone();
    let missing = dependency_selection_requirements(&nodes, &selected)
        .expect("valid chain requirements");
    assert_eq!(missing.len(), 99_999);
    assert_eq!(
        missing.first(),
        Some(&MissingDependencyRequirement {
            command: 1,
            dependency: 0,
        }),
    );
    assert_eq!(
        missing.last(),
        Some(&MissingDependencyRequirement {
            command: 99_999,
            dependency: 99_998,
        }),
    );
    assert_eq!(selected, before);
}

#[test]
fn dependency_selection_summary_counts_closure_without_materializing_pairs() {
    let nodes = [node(1, &[]), node(2, &[1]), node(3, &[2]), node(4, &[1])];
    let selected = BTreeSet::from([3]);
    let before = selected.clone();
    assert_eq!(
        dependency_selection_summary(&nodes, &selected),
        Ok(DependencySelectionSummary {
            missing_dependency_edges: 2,
            required_commands: 3,
            selected_commands: 1,
        }),
    );
    assert_eq!(selected, before);
}

#[test]
fn dependency_selection_summary_preserves_duplicate_edge_count() {
    let nodes = [node(1, &[]), node(2, &[1, 1])];
    assert_eq!(
        dependency_selection_summary(&nodes, &BTreeSet::from([2])),
        Ok(DependencySelectionSummary {
            missing_dependency_edges: 2,
            required_commands: 2,
            selected_commands: 1,
        }),
    );
}

#[test]
fn dependency_selection_summary_rejects_unknown_and_invalid_graph() {
    let nodes = [node(1, &[]), node(2, &[1])];
    assert_eq!(
        dependency_selection_summary(&nodes, &BTreeSet::from([9])),
        Err(DependencySummaryError::UnknownSelection { command: 9 }),
    );
    let invalid = [node(1, &[2]), node(2, &[])];
    assert_eq!(
        dependency_selection_summary(&invalid, &BTreeSet::from([1])),
        Err(DependencySummaryError::Graph {
            reason: CommandGraphError::DependencyAfterCommand {
                command: 1,
                dependency: 2,
            },
        }),
    );
}

#[test]
fn bounded_dependency_requirements_accept_exact_limit_and_closed_selection() {
    let nodes = [node(1, &[]), node(2, &[1]), node(3, &[2])];
    assert_eq!(
        dependency_selection_requirements_bounded(
            &nodes,
            &BTreeSet::from([3]),
            2,
        ),
        Ok(vec![
            MissingDependencyRequirement {
                command: 2,
                dependency: 1,
            },
            MissingDependencyRequirement {
                command: 3,
                dependency: 2,
            },
        ]),
    );
    assert_eq!(
        dependency_selection_requirements_bounded(
            &nodes,
            &BTreeSet::from([1, 2, 3]),
            0,
        ),
        Ok(Vec::new()),
    );
}

#[test]
fn bounded_dependency_requirements_reject_before_pair_materialization() {
    let nodes = [node(1, &[]), node(2, &[1]), node(3, &[2])];
    assert_eq!(
        dependency_selection_requirements_bounded(
            &nodes,
            &BTreeSet::from([3]),
            1,
        ),
        Err(
            BoundedDependencyRequirementsError::RequirementCountExceeded {
                actual: 2,
                limit: 1,
            }
        ),
    );
}
