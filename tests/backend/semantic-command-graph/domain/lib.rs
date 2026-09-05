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
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

use atrament_semantic_command_graph::{
    BoundedDependencyRequirementsError, CommandDependencyNode,
    CommandGraphError, CommandGraphLimitError, CommandGraphLimits,
    CommandGraphSize, CommandNode, DependencyRequirementsError,
    DependencySelectionError, DependencySelectionSummary,
    DependencySummaryError, MissingDependencyRequirement, command_graph_size,
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
    assert_eq!(validate_command_graph::<CommandNode<u32>>(&[]), Ok(()));
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
        command_graph_size::<CommandNode<u32>>(&[]),
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
fn full_selection_fast_path_still_rejects_same_size_unknown_set() {
    let nodes = [node(1, &[]), node(2, &[1]), node(3, &[2])];
    let selected = BTreeSet::from([1, 2, 9]);
    assert_eq!(
        dependency_selection_requirements(&nodes, &selected),
        Err(DependencyRequirementsError::UnknownSelection { command: 9 }),
    );
    assert_eq!(
        dependency_selection_summary(&nodes, &selected),
        Err(DependencySummaryError::UnknownSelection { command: 9 }),
    );
    assert_eq!(
        validate_dependency_closed_selection(&nodes, &selected),
        Err(DependencySelectionError::UnknownSelection { command: 9 }),
    );
}

#[test]
fn empty_selection_still_validates_the_complete_source_graph() {
    let valid = [node(1, &[]), node(2, &[1])];
    let selected = BTreeSet::new();
    assert_eq!(
        validate_dependency_closed_selection(&valid, &selected),
        Ok(()),
    );
    assert_eq!(
        dependency_selection_requirements(&valid, &selected),
        Ok(Vec::new()),
    );
    assert_eq!(
        dependency_selection_summary(&valid, &selected),
        Ok(DependencySelectionSummary {
            missing_dependency_edges: 0,
            required_commands: 0,
            selected_commands: 0,
        }),
    );

    let invalid = [node(1, &[2]), node(2, &[])];
    assert_eq!(
        dependency_selection_requirements(&invalid, &selected),
        Err(DependencyRequirementsError::Graph {
            reason: CommandGraphError::DependencyAfterCommand {
                command: 1,
                dependency: 2,
            },
        }),
    );
    assert_eq!(
        dependency_selection_summary(&invalid, &selected),
        Err(DependencySummaryError::Graph {
            reason: CommandGraphError::DependencyAfterCommand {
                command: 1,
                dependency: 2,
            },
        }),
    );
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

#[derive(Debug)]
struct CountingIdentity<'counter> {
    clones: &'counter AtomicUsize,
    value: u32,
}

impl Clone for CountingIdentity<'_> {
    fn clone(&self) -> Self {
        let _previous = self.clones.fetch_add(1, AtomicOrdering::Relaxed);
        Self {
            clones: self.clones,
            value: self.value,
        }
    }
}

impl Eq for CountingIdentity<'_> {}

impl Ord for CountingIdentity<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

impl PartialEq for CountingIdentity<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl PartialOrd for CountingIdentity<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn counting_identity(clones: &AtomicUsize, value: u32) -> CountingIdentity<'_> {
    CountingIdentity { clones, value }
}

#[test]
#[allow(clippy::mutable_key_type)]
fn bounded_requirement_rejection_does_not_clone_identity_pairs() {
    let clones = AtomicUsize::new(0);
    let nodes = [
        CommandNode {
            dependencies: Vec::new(),
            id: counting_identity(&clones, 1),
        },
        CommandNode {
            dependencies: vec![counting_identity(&clones, 1)],
            id: counting_identity(&clones, 2),
        },
        CommandNode {
            dependencies: vec![counting_identity(&clones, 2)],
            id: counting_identity(&clones, 3),
        },
    ];
    let selected = BTreeSet::from([counting_identity(&clones, 3)]);
    clones.store(0, AtomicOrdering::Relaxed);
    assert_eq!(
        dependency_selection_requirements_bounded(&nodes, &selected, 1),
        Err(
            BoundedDependencyRequirementsError::RequirementCountExceeded {
                actual: 2,
                limit: 1,
            }
        ),
    );
    assert_eq!(clones.load(AtomicOrdering::Relaxed), 0);
}

#[derive(Clone, Copy)]
struct BorrowedNode<'graph> {
    dependencies: &'graph [u32],
    id: &'graph u32,
}

impl CommandDependencyNode for BorrowedNode<'_> {
    type Identity = u32;

    fn dependencies(&self) -> &[Self::Identity] {
        self.dependencies
    }

    fn id(&self) -> &Self::Identity {
        self.id
    }
}

#[test]
fn graph_validation_accepts_borrowed_node_views() {
    let one = 1_u32;
    let two = 2_u32;
    let three = 3_u32;
    let no_dependencies = [];
    let depends_on_one = [1_u32];
    let depends_on_two = [2_u32];
    let nodes = [
        BorrowedNode {
            dependencies: &no_dependencies,
            id: &one,
        },
        BorrowedNode {
            dependencies: &depends_on_one,
            id: &two,
        },
        BorrowedNode {
            dependencies: &depends_on_two,
            id: &three,
        },
    ];
    assert_eq!(validate_command_graph(&nodes), Ok(()));
    assert_eq!(
        dependency_selection_summary(&nodes, &BTreeSet::from([three])),
        Ok(DependencySelectionSummary {
            missing_dependency_edges: 2,
            required_commands: 3,
            selected_commands: 1,
        }),
    );
}

fn next_graph_oracle_value(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *seed >> 32
}

fn exhaustive_dependency_mask(mask: u8) -> Vec<u32> {
    [0_u32, 1, 2, 9]
        .into_iter()
        .enumerate()
        .filter_map(|(bit, dependency)| {
            (mask & (1_u8 << bit) != 0).then_some(dependency)
        })
        .collect()
}

fn reference_three_node_graph(
    nodes: &[CommandNode<u32>; 3],
) -> Result<(), CommandGraphError<u32>> {
    let mut first_forward = None;
    let mut reachability = [[false; 3]; 3];
    for (position, node) in nodes.iter().enumerate() {
        for dependency in &node.dependencies {
            if *dependency == node.id {
                return Err(CommandGraphError::SelfDependency {
                    command: node.id,
                });
            }
            let dependency_position = nodes
                .iter()
                .position(|candidate| candidate.id == *dependency);
            let Some(dependency_position) = dependency_position else {
                return Err(CommandGraphError::MissingDependency {
                    command: node.id,
                    dependency: *dependency,
                });
            };
            reachability[position][dependency_position] = true;
            if dependency_position > position && first_forward.is_none() {
                first_forward = Some((node.id, *dependency));
            }
        }
    }
    let Some((command, dependency)) = first_forward else {
        return Ok(());
    };
    for intermediate in 0..3 {
        for from in 0..3 {
            for to in 0..3 {
                reachability[from][to] |= reachability[from][intermediate]
                    && reachability[intermediate][to];
            }
        }
    }
    if (0..3).any(|position| reachability[position][position]) {
        Err(CommandGraphError::Cycle)
    } else {
        Err(CommandGraphError::DependencyAfterCommand {
            command,
            dependency,
        })
    }
}

#[test]
fn every_three_command_graph_matches_invalid_precedence_oracle() {
    let mut cases = 0_u32;
    for first_mask in 0_u8..16 {
        for second_mask in 0_u8..16 {
            for third_mask in 0_u8..16 {
                let nodes = [
                    node(0, &exhaustive_dependency_mask(first_mask)),
                    node(1, &exhaustive_dependency_mask(second_mask)),
                    node(2, &exhaustive_dependency_mask(third_mask)),
                ];
                assert_eq!(
                    validate_command_graph(&nodes),
                    reference_three_node_graph(&nodes),
                    "graph mismatch for masks {}/{}/{}",
                    first_mask,
                    second_mask,
                    third_mask,
                );
                cases = cases.saturating_add(1);
            }
        }
    }
    assert_eq!(cases, 4_096);
}

fn reference_dependency_requirements(
    nodes: &[CommandNode<u32>],
    selected: &BTreeSet<u32>,
) -> (
    BTreeSet<u32>,
    Vec<MissingDependencyRequirement<u32>>,
) {
    let mut required = selected.clone();
    for node in nodes.iter().rev() {
        if !required.contains(&node.id) {
            continue;
        }
        required.extend(node.dependencies.iter().copied());
    }
    let mut missing = Vec::new();
    for node in nodes {
        if !required.contains(&node.id) {
            continue;
        }
        for dependency in &node.dependencies {
            if !selected.contains(dependency) {
                missing.push(MissingDependencyRequirement {
                    command: node.id,
                    dependency: *dependency,
                });
            }
        }
    }
    (required, missing)
}

fn reference_closed_selection(
    nodes: &[CommandNode<u32>],
    selected: &BTreeSet<u32>,
) -> Result<(), DependencySelectionError<u32>> {
    for node in nodes {
        if !selected.contains(&node.id) {
            continue;
        }
        for dependency in &node.dependencies {
            if !selected.contains(dependency) {
                return Err(
                    DependencySelectionError::MissingRequiredDependency {
                        command: node.id,
                        dependency: *dependency,
                    },
                );
            }
        }
    }
    Ok(())
}

#[test]
fn valid_dag_selection_apis_match_reference_oracle() {
    const CASES: usize = 20_000;
    let mut seed = 0x05ee_dda6_2026_u64;
    for case in 0..CASES {
        let node_count = (next_graph_oracle_value(&mut seed) % 9) as u32;
        let mut nodes = Vec::with_capacity(node_count as usize);
        for id in 0..node_count {
            let mut dependencies = Vec::new();
            for dependency in 0..id {
                if next_graph_oracle_value(&mut seed).is_multiple_of(3) {
                    dependencies.push(dependency);
                    if next_graph_oracle_value(&mut seed).is_multiple_of(5) {
                        dependencies.push(dependency);
                    }
                }
            }
            nodes.push(CommandNode { dependencies, id });
        }
        let selected = nodes
            .iter()
            .filter_map(|node| {
                (!next_graph_oracle_value(&mut seed).is_multiple_of(3))
                    .then_some(node.id)
            })
            .collect::<BTreeSet<_>>();
        let (required, expected_missing) =
            reference_dependency_requirements(&nodes, &selected);
        let expected_summary = DependencySelectionSummary {
            missing_dependency_edges: expected_missing.len(),
            required_commands: required.len(),
            selected_commands: selected.len(),
        };
        let expected_size = CommandGraphSize {
            commands: nodes.len(),
            dependency_edges: nodes
                .iter()
                .map(|node| node.dependencies.len())
                .sum(),
        };

        assert_eq!(
            validate_command_graph(&nodes),
            Ok(()),
            "graph validity mismatch in generated case {case}",
        );
        assert_eq!(
            command_graph_size(&nodes),
            Ok(expected_size),
            "graph size mismatch in generated case {case}",
        );
        assert_eq!(
            dependency_selection_requirements(&nodes, &selected),
            Ok(expected_missing.clone()),
            "requirements mismatch in generated case {case}",
        );
        assert_eq!(
            dependency_selection_summary(&nodes, &selected),
            Ok(expected_summary),
            "summary mismatch in generated case {case}",
        );
        assert_eq!(
            dependency_selection_requirements_bounded(
                &nodes,
                &selected,
                expected_missing.len(),
            ),
            Ok(expected_missing.clone()),
            "bounded requirements mismatch in generated case {case}",
        );
        assert_eq!(
            validate_dependency_closed_selection(&nodes, &selected),
            reference_closed_selection(&nodes, &selected),
            "closure mismatch in generated case {case}",
        );
        if !expected_missing.is_empty() {
            let limit = expected_missing.len() - 1;
            assert_eq!(
                dependency_selection_requirements_bounded(
                    &nodes,
                    &selected,
                    limit,
                ),
                Err(
                    BoundedDependencyRequirementsError::
                        RequirementCountExceeded {
                            actual: expected_missing.len(),
                            limit,
                        },
                ),
                "bounded rejection mismatch in generated case {case}",
            );
        }
    }
}
