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
//   - Transport-neutral semantic command dependency graph validation.
// - Must-Not:
//   - Parse wire commands, normalize command order, mutate notebooks, or apply.
//   - Choose serialized command identity syntax or retry semantics.
// - Allows:
//   - Inputs: Ordered command nodes with caller-owned typed identities.
//   - Outputs: Valid graph or typed duplicate/reference/cycle failure.
//   - Side effects: Process-local validation allocation only.
// - Split-When:
//   - Command graph normalization or impact expansion becomes independent.
// - Merge-When:
//   - A future command-batch domain fully owns dependency validation.
// - Summary:
//   - Validates command dependency structure without freezing transport shape.
// - Description:
//   - Checks unique owners, dependency references, self-edges, and acyclicity.
// - Usage:
//   - Validate parsed command structure before semantic batch simulation.
// - Defaults:
//   - Command order is preserved and never normalized by graph validation.
//

//! Transport-neutral semantic command dependency graph validation.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// One ordered command and the command identities it explicitly depends on.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandNode<Identity> {
    /// Explicit predecessor command identities required by this command.
    pub dependencies: Vec<Identity>,
    /// Caller-owned command identity, whose representation remains external.
    pub id: Identity,
}

/// Read-only view of one ordered command dependency node.
pub trait CommandDependencyNode {
    /// Caller-owned command identity representation.
    type Identity: Ord;

    /// Explicit predecessor command identities required by this command.
    fn dependencies(&self) -> &[Self::Identity];

    /// Caller-owned identity of this command.
    fn id(&self) -> &Self::Identity;
}

impl<Identity> CommandDependencyNode for CommandNode<Identity>
where
    Identity: Ord,
{
    type Identity = Identity;

    fn dependencies(&self) -> &[Self::Identity] {
        &self.dependencies
    }

    fn id(&self) -> &Self::Identity {
        &self.id
    }
}

/// Caller-supplied coarse resource bounds for one command graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandGraphLimits {
    /// Maximum ordered commands admitted by the calling capability.
    pub commands: usize,
    /// Maximum explicit dependency edges admitted by the calling capability.
    pub dependency_edges: usize,
}

/// Exact coarse size of one in-memory command graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandGraphSize {
    /// Ordered command count.
    pub commands: usize,
    /// Explicit dependency-edge count, including repeated explicit edges.
    pub dependency_edges: usize,
}

/// Typed coarse command-graph resource failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandGraphLimitError {
    /// Ordered command count exceeds the supplied capability limit.
    CommandCountExceeded {
        /// Exact ordered command count.
        actual: usize,
        /// Maximum ordered commands admitted by the caller.
        limit: usize,
    },
    /// Explicit dependency-edge count exceeds the supplied capability limit.
    DependencyEdgeCountExceeded {
        /// Exact explicit dependency-edge count.
        actual: usize,
        /// Maximum explicit dependencies admitted by the caller.
        limit: usize,
    },
    /// Explicit dependency-edge counting exceeded addressable `usize` range.
    DependencyEdgeCountOverflow,
}

/// One explicit dependency edge omitted by an interactive selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissingDependencyRequirement<Identity> {
    /// Command whose complete dependency closure requires another command.
    pub command: Identity,
    /// Explicit required dependency absent from the caller's original
    /// selection.
    pub dependency: Identity,
}

/// Exact coarse size of one dependency selection and its required closure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DependencySelectionSummary {
    /// Explicit dependency edges omitted by the caller selection.
    pub missing_dependency_edges: usize,
    /// Commands in the complete transitive dependency closure.
    pub required_commands: usize,
    /// Commands explicitly selected by the caller.
    pub selected_commands: usize,
}

/// Typed failure while materializing a caller-bounded requirement report.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BoundedDependencyRequirementsError<Identity> {
    /// The complete source graph is invalid before requirements are derived.
    Graph {
        /// Typed structural failure in the complete command graph.
        reason: CommandGraphError<Identity>,
    },
    /// Exact omitted dependency-edge count exceeds the caller-supplied bound.
    RequirementCountExceeded {
        /// Exact omitted dependency-edge count.
        actual: usize,
        /// Maximum missing dependency edges the caller admits materializing.
        limit: usize,
    },
    /// Missing-edge counting exceeded addressable `usize` range.
    RequirementCountOverflow,
    /// Selection names no command in the complete source graph.
    UnknownSelection {
        /// Unknown command identity named by the selection.
        command: Identity,
    },
}

/// Typed failure while summarizing one dependency selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DependencySummaryError<Identity> {
    /// The complete source graph is invalid before summary derivation.
    Graph {
        /// Typed structural failure in the complete command graph.
        reason: CommandGraphError<Identity>,
    },
    /// Missing-edge counting exceeded addressable `usize` range.
    RequirementCountOverflow,
    /// Selection names no command in the complete source graph.
    UnknownSelection {
        /// Unknown command identity named by the selection.
        command: Identity,
    },
}

/// Typed failure while deriving dependency requirements for one selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DependencyRequirementsError<Identity> {
    /// The complete source graph is invalid before requirements are derived.
    Graph {
        /// Typed structural failure in the complete command graph.
        reason: CommandGraphError<Identity>,
    },
    /// Selection names no command in the complete source graph.
    UnknownSelection {
        /// Unknown command identity named by the selection.
        command: Identity,
    },
}

/// Typed failure while checking one selected command subset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DependencySelectionError<Identity> {
    /// The complete source graph is invalid before subset closure is checked.
    Graph {
        /// Typed structural failure in the complete command graph.
        reason: CommandGraphError<Identity>,
    },
    /// One selected command requires another command that was not selected.
    MissingRequiredDependency {
        /// Selected command whose dependency is absent from the selection.
        command: Identity,
        /// Required dependency omitted by the selection.
        dependency: Identity,
    },
    /// Selection names no command in the complete source graph.
    UnknownSelection {
        /// Unknown command identity named by the selection.
        command: Identity,
    },
}

/// Typed command dependency graph failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandGraphError<Identity> {
    /// One or more commands participate in a dependency cycle.
    Cycle,
    /// One command depends on a command that appears later in source order.
    DependencyAfterCommand {
        /// Command containing the invalid forward dependency.
        command: Identity,
        /// Dependency whose command appears later in the ordered batch.
        dependency: Identity,
    },
    /// One command identity is owned by more than one command node.
    DuplicateIdentity {
        /// Duplicated command identity.
        command: Identity,
    },
    /// One dependency names no command in the complete graph.
    MissingDependency {
        /// Command containing the invalid dependency.
        command: Identity,
        /// Missing dependency identity.
        dependency: Identity,
    },
    /// One command depends directly on itself.
    SelfDependency {
        /// Self-dependent command identity.
        command: Identity,
    },
}

struct DependencySelectionState<'graph, Identity> {
    positions: BTreeMap<&'graph Identity, usize>,
    required_positions: Vec<bool>,
    selected_positions: Vec<bool>,
}

/// Measure exact coarse command and dependency-edge counts.
///
/// # Errors
///
/// Returns [`CommandGraphLimitError::DependencyEdgeCountOverflow`] if summing
/// explicit dependency edges exceeds addressable `usize` range.
pub fn command_graph_size<Node>(
    nodes: &[Node],
) -> Result<CommandGraphSize, CommandGraphLimitError>
where
    Node: CommandDependencyNode,
{
    let mut dependency_edges = 0usize;
    for node in nodes {
        let Some(next) =
            dependency_edges.checked_add(node.dependencies().len())
        else {
            return Err(CommandGraphLimitError::DependencyEdgeCountOverflow);
        };
        dependency_edges = next;
    }
    Ok(CommandGraphSize {
        commands: nodes.len(),
        dependency_edges,
    })
}

/// Enforce caller-owned command-count and dependency-edge resource limits.
///
/// This function does not choose product limits and does not validate graph
/// semantics. It only rejects a complete in-memory graph when exact coarse size
/// exceeds a supplied capability bound; it never truncates nodes or edges.
///
/// # Errors
///
/// Returns a typed exact count overflow or at the first exceeded supplied
/// bound.
pub fn validate_command_graph_limits<Node>(
    nodes: &[Node],
    limits: CommandGraphLimits,
) -> Result<CommandGraphSize, CommandGraphLimitError>
where
    Node: CommandDependencyNode,
{
    if nodes.len() > limits.commands {
        return Err(CommandGraphLimitError::CommandCountExceeded {
            actual: nodes.len(),
            limit: limits.commands,
        });
    }
    let size = command_graph_size(nodes)?;
    if size.dependency_edges > limits.dependency_edges {
        return Err(CommandGraphLimitError::DependencyEdgeCountExceeded {
            actual: size.dependency_edges,
            limit: limits.dependency_edges,
        });
    }
    Ok(size)
}

/// Validate one complete command dependency graph without changing node order.
///
/// # Errors
///
/// Returns a typed failure for duplicate command identities, direct
/// self-dependencies, missing or forward dependency identities, or dependency
/// cycles.
pub fn validate_command_graph<Node>(
    nodes: &[Node],
) -> Result<(), CommandGraphError<Node::Identity>>
where
    Node: CommandDependencyNode,
    Node::Identity: Clone,
{
    validated_command_positions(nodes).map(|_positions| ())
}

fn validated_command_positions<Node>(
    nodes: &[Node],
) -> Result<BTreeMap<&Node::Identity, usize>, CommandGraphError<Node::Identity>>
where
    Node: CommandDependencyNode,
    Node::Identity: Clone,
{
    let mut positions = BTreeMap::new();
    for (position, node) in nodes.iter().enumerate() {
        if positions.insert(node.id(), position).is_some() {
            return Err(CommandGraphError::DuplicateIdentity {
                command: node.id().clone(),
            });
        }
    }

    let mut first_forward = None;
    for (position, node) in nodes.iter().enumerate() {
        for dependency in node.dependencies() {
            if dependency == node.id() {
                return Err(CommandGraphError::SelfDependency {
                    command: node.id().clone(),
                });
            }
            let Some(dependency_position) = positions.get(dependency).copied()
            else {
                return Err(CommandGraphError::MissingDependency {
                    command: node.id().clone(),
                    dependency: dependency.clone(),
                });
            };
            if dependency_position > position && first_forward.is_none() {
                first_forward = Some((node.id().clone(), dependency.clone()));
            }
        }
    }
    let Some((forward_command, forward_dependency)) = first_forward else {
        return Ok(positions);
    };

    let mut indegrees = vec![0usize; nodes.len()];
    let mut dependents = vec![Vec::new(); nodes.len()];
    for (position, node) in nodes.iter().enumerate() {
        for dependency in node.dependencies() {
            let Some(dependency_position) = positions.get(dependency).copied()
            else {
                continue;
            };
            if let Some(degree) = indegrees.get_mut(position) {
                *degree = degree.saturating_add(1);
            }
            if let Some(next_commands) = dependents.get_mut(dependency_position)
            {
                next_commands.push(position);
            }
        }
    }

    let mut ready = VecDeque::new();
    for (position, degree) in indegrees.iter().enumerate() {
        if *degree == 0 {
            ready.push_back(position);
        }
    }
    let mut processed = 0usize;
    while let Some(position) = ready.pop_front() {
        processed = processed.saturating_add(1);
        let Some(next_commands) = dependents.get(position) else {
            continue;
        };
        for next_position in next_commands {
            let Some(degree) = indegrees.get_mut(*next_position) else {
                continue;
            };
            *degree = degree.saturating_sub(1);
            if *degree == 0 {
                ready.push_back(*next_position);
            }
        }
    }
    if processed != nodes.len() {
        return Err(CommandGraphError::Cycle);
    }
    Err(CommandGraphError::DependencyAfterCommand {
        command: forward_command,
        dependency: forward_dependency,
    })
}

fn dependency_selection_state<'graph, Node>(
    nodes: &'graph [Node],
    selected: &BTreeSet<Node::Identity>,
) -> Result<
    Option<DependencySelectionState<'graph, Node::Identity>>,
    DependencyRequirementsError<Node::Identity>,
>
where
    Node: CommandDependencyNode,
    Node::Identity: Clone,
{
    let positions = match validated_command_positions(nodes) {
        Ok(positions) => positions,
        Err(reason) => {
            return Err(DependencyRequirementsError::Graph { reason });
        },
    };
    if selected.len() == nodes.len()
        && selected.iter().zip(positions.keys()).all(
            |(selected_command, known_command)| {
                selected_command == *known_command
            },
        )
    {
        return Ok(None);
    }
    let mut selected_positions = vec![false; nodes.len()];
    for command in selected {
        let Some(position) = positions.get(command).copied() else {
            return Err(DependencyRequirementsError::UnknownSelection {
                command: command.clone(),
            });
        };
        if let Some(is_selected) = selected_positions.get_mut(position) {
            *is_selected = true;
        }
    }

    let mut required_positions = selected_positions.clone();
    for (position, node) in nodes.iter().enumerate().rev() {
        if !required_positions.get(position).copied().unwrap_or(false) {
            continue;
        }
        for dependency in node.dependencies() {
            let Some(dependency_position) = positions.get(dependency).copied()
            else {
                continue;
            };
            if let Some(is_required) =
                required_positions.get_mut(dependency_position)
            {
                *is_required = true;
            }
        }
    }
    Ok(Some(DependencySelectionState {
        positions,
        required_positions,
        selected_positions,
    }))
}

fn missing_dependency_edge_count<Node>(
    nodes: &[Node],
    state: &DependencySelectionState<'_, Node::Identity>,
) -> Option<usize>
where
    Node: CommandDependencyNode,
{
    let mut count = 0usize;
    for (position, node) in nodes.iter().enumerate() {
        if !state
            .required_positions
            .get(position)
            .copied()
            .unwrap_or(false)
        {
            continue;
        }
        for dependency in node.dependencies() {
            let Some(dependency_position) =
                state.positions.get(dependency).copied()
            else {
                continue;
            };
            if state
                .selected_positions
                .get(dependency_position)
                .copied()
                .unwrap_or(false)
            {
                continue;
            }
            count = count.checked_add(1)?;
        }
    }
    Some(count)
}

fn missing_dependency_requirements<Node>(
    nodes: &[Node],
    state: &DependencySelectionState<'_, Node::Identity>,
) -> Vec<MissingDependencyRequirement<Node::Identity>>
where
    Node: CommandDependencyNode,
    Node::Identity: Clone,
{
    let mut missing = Vec::new();
    for (position, node) in nodes.iter().enumerate() {
        if !state
            .required_positions
            .get(position)
            .copied()
            .unwrap_or(false)
        {
            continue;
        }
        for dependency in node.dependencies() {
            let Some(dependency_position) =
                state.positions.get(dependency).copied()
            else {
                continue;
            };
            if !state
                .selected_positions
                .get(dependency_position)
                .copied()
                .unwrap_or(false)
            {
                missing.push(MissingDependencyRequirement {
                    command: node.id().clone(),
                    dependency: dependency.clone(),
                });
            }
        }
    }
    missing
}

/// Derive omitted dependency requirements subject to a caller-supplied report
/// bound.
///
/// The complete exact omitted-edge count is derived before any identity-pair
/// result is materialized. The source graph and caller selection remain
/// unchanged.
///
/// # Errors
///
/// Returns a typed graph or unknown-selection failure, exact count overflow, or
/// an exact omitted-edge count greater than `maximum_missing_edges`.
pub fn dependency_selection_requirements_bounded<Node>(
    nodes: &[Node],
    selected: &BTreeSet<Node::Identity>,
    maximum_missing_edges: usize,
) -> Result<
    Vec<MissingDependencyRequirement<Node::Identity>>,
    BoundedDependencyRequirementsError<Node::Identity>,
>
where
    Node: CommandDependencyNode,
    Node::Identity: Clone,
{
    let state = match dependency_selection_state(nodes, selected) {
        Ok(Some(state)) => state,
        Ok(None) => return Ok(Vec::new()),
        Err(DependencyRequirementsError::Graph { reason }) => {
            return Err(BoundedDependencyRequirementsError::Graph { reason });
        },
        Err(DependencyRequirementsError::UnknownSelection { command }) => {
            return Err(BoundedDependencyRequirementsError::UnknownSelection {
                command,
            });
        },
    };
    let Some(actual) = missing_dependency_edge_count(nodes, &state) else {
        return Err(
            BoundedDependencyRequirementsError::RequirementCountOverflow,
        );
    };
    if actual > maximum_missing_edges {
        return Err(
            BoundedDependencyRequirementsError::RequirementCountExceeded {
                actual,
                limit: maximum_missing_edges,
            },
        );
    }
    Ok(missing_dependency_requirements(nodes, &state))
}

/// Summarize selection and transitive dependency-closure size without
/// materializing missing identity pairs.
///
/// # Errors
///
/// Returns a typed graph failure, unknown selected identity, or exact count
/// overflow. The caller selection is never changed.
pub fn dependency_selection_summary<Node>(
    nodes: &[Node],
    selected: &BTreeSet<Node::Identity>,
) -> Result<DependencySelectionSummary, DependencySummaryError<Node::Identity>>
where
    Node: CommandDependencyNode,
    Node::Identity: Clone,
{
    let state = match dependency_selection_state(nodes, selected) {
        Ok(Some(state)) => state,
        Ok(None) => {
            return Ok(DependencySelectionSummary {
                missing_dependency_edges: 0,
                required_commands: nodes.len(),
                selected_commands: selected.len(),
            });
        },
        Err(DependencyRequirementsError::Graph { reason }) => {
            return Err(DependencySummaryError::Graph { reason });
        },
        Err(DependencyRequirementsError::UnknownSelection { command }) => {
            return Err(DependencySummaryError::UnknownSelection { command });
        },
    };
    let required_commands = state
        .required_positions
        .iter()
        .filter(|is_required| **is_required)
        .count();
    let Some(missing_dependency_edges) =
        missing_dependency_edge_count(nodes, &state)
    else {
        return Err(DependencySummaryError::RequirementCountOverflow);
    };
    Ok(DependencySelectionSummary {
        missing_dependency_edges,
        required_commands,
        selected_commands: selected.len(),
    })
}

/// Derive the complete explicit dependency requirements omitted by a selection.
///
/// The source graph is validated first. The function computes the transitive
/// dependency closure required by the selected commands but does not mutate or
/// return a replacement selection. Requirements are reported in original
/// command order and each command's explicit dependency order.
///
/// # Errors
///
/// Returns the complete graph failure or the first selected identity absent
/// from the source graph.
pub fn dependency_selection_requirements<Node>(
    nodes: &[Node],
    selected: &BTreeSet<Node::Identity>,
) -> Result<
    Vec<MissingDependencyRequirement<Node::Identity>>,
    DependencyRequirementsError<Node::Identity>,
>
where
    Node: CommandDependencyNode,
    Node::Identity: Clone,
{
    let Some(state) = dependency_selection_state(nodes, selected)? else {
        return Ok(Vec::new());
    };
    Ok(missing_dependency_requirements(nodes, &state))
}

/// Check that one selected subset contains every explicit command dependency.
///
/// The complete graph is validated first. Selection is set-valued and therefore
/// does not define or rewrite command order. Required dependencies are checked
/// in the original command and dependency order supplied by `nodes`.
///
/// # Errors
///
/// Returns the complete graph failure, an unknown selected command identity, or
/// the first selected command whose explicit dependency was omitted.
pub fn validate_dependency_closed_selection<Node>(
    nodes: &[Node],
    selected: &BTreeSet<Node::Identity>,
) -> Result<(), DependencySelectionError<Node::Identity>>
where
    Node: CommandDependencyNode,
    Node::Identity: Clone,
{
    let positions = match validated_command_positions(nodes) {
        Ok(positions) => positions,
        Err(reason) => return Err(DependencySelectionError::Graph { reason }),
    };
    if selected.len() == nodes.len()
        && selected.iter().zip(positions.keys()).all(
            |(selected_command, known_command)| {
                selected_command == *known_command
            },
        )
    {
        return Ok(());
    }
    if let Some(command) = selected
        .iter()
        .find(|command| !positions.contains_key(*command))
    {
        return Err(DependencySelectionError::UnknownSelection {
            command: command.clone(),
        });
    }
    for node in nodes {
        if !selected.contains(node.id()) {
            continue;
        }
        for dependency in node.dependencies() {
            if !selected.contains(dependency) {
                return Err(
                    DependencySelectionError::MissingRequiredDependency {
                        command: node.id().clone(),
                        dependency: dependency.clone(),
                    },
                );
            }
        }
    }
    Ok(())
}
