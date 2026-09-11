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

/// One batch-local handle declared by a producing command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchLocalHandleDeclaration<CommandIdentity, Handle> {
    /// Command that produces the candidate object named by this handle.
    pub command: CommandIdentity,
    /// Batch-local candidate handle whose representation remains external.
    pub handle: Handle,
}

/// One later command reference to a batch-local candidate handle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchLocalHandleReference<CommandIdentity, Handle> {
    /// Command that consumes the candidate object named by this handle.
    pub command: CommandIdentity,
    /// Batch-local candidate handle whose representation remains external.
    pub handle: Handle,
}

/// Typed structural failure for batch-local candidate handles.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BatchLocalHandleError<CommandIdentity, Handle> {
    /// A handle declaration names no command in the complete batch graph.
    DeclarationCommandMissing {
        /// Missing producing command identity.
        command: CommandIdentity,
        /// Handle whose producer is absent.
        handle: Handle,
    },
    /// More than one producing command declares the same batch-local handle.
    DuplicateHandle {
        /// First command that declared this handle.
        first_command: CommandIdentity,
        /// Duplicated batch-local handle.
        handle: Handle,
        /// Later command that attempted to redeclare this handle.
        second_command: CommandIdentity,
    },
    /// The complete command graph is invalid before handles are considered.
    Graph {
        /// Typed command dependency graph failure.
        reason: CommandGraphError<CommandIdentity>,
    },
    /// A handle reference names no consuming command in the complete graph.
    ReferenceCommandMissing {
        /// Missing consuming command identity.
        command: CommandIdentity,
        /// Handle referenced by the absent command.
        handle: Handle,
    },
    /// Consumer omits the required explicit dependency on the producer.
    RequiredDependencyMissing {
        /// Consuming command identity.
        command: CommandIdentity,
        /// Referenced batch-local handle.
        handle: Handle,
        /// Producing command that must be an explicit dependency.
        producer: CommandIdentity,
    },
    /// A command references a batch-local handle with no declaration.
    UndeclaredHandle {
        /// Consuming command identity.
        command: CommandIdentity,
        /// Undeclared batch-local handle.
        handle: Handle,
    },
}

/// Result of validating batch-local handle ownership and references.
pub type BatchLocalHandleValidationResult<CommandIdentity, Handle> =
    Result<(), BatchLocalHandleError<CommandIdentity, Handle>>;

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

/// Result of validating one complete command graph.
pub type CommandGraphValidationResult<Identity> =
    Result<(), CommandGraphError<Identity>>;

/// Complete omitted-dependency requirement report result.
pub type DependencyRequirementsResult<Identity> = Result<
    Vec<MissingDependencyRequirement<Identity>>,
    DependencyRequirementsError<Identity>,
>;

/// Caller-bounded omitted-dependency requirement report result.
pub type BoundedDependencyRequirementsResult<Identity> = Result<
    Vec<MissingDependencyRequirement<Identity>>,
    BoundedDependencyRequirementsError<Identity>,
>;

/// Dependency-selection summary result.
pub type DependencySummaryResult<Identity> =
    Result<DependencySelectionSummary, DependencySummaryError<Identity>>;

/// Result of validating one dependency-closed selection.
pub type DependencySelectionValidationResult<Identity> =
    Result<(), DependencySelectionError<Identity>>;

type CommandPositions<'graph, Identity> = BTreeMap<&'graph Identity, usize>;
type CommandPositionsResult<'graph, Identity> =
    Result<CommandPositions<'graph, Identity>, CommandGraphError<Identity>>;
type ForwardDependencyResult<Identity> =
    Result<Option<(Identity, Identity)>, CommandGraphError<Identity>>;
type SelectionStateResult<'graph, Identity> = Result<
    Option<DependencySelectionState<'graph, Identity>>,
    DependencyRequirementsError<Identity>,
>;


struct DependencySelectionState<'graph, Identity> {
    positions: BTreeMap<&'graph Identity, usize>,
    required_positions: Vec<bool>,
    selected_positions: Vec<bool>,
}

/// Validate batch-local handle ownership and explicit producer dependencies.
///
/// The complete command graph is validated first. Handle declarations are then
/// checked in caller order for known producers and uniqueness, followed by
/// references in caller order for known consumers, declared handles, and an
/// explicit dependency on the producing command. Inputs are never reordered.
///
/// This function does not decide which command families may declare handles,
/// allocate accepted semantic identities, or choose serialized handle syntax.
/// Those remain responsibilities of the admitted command protocol and Apply.
///
/// # Errors
///
/// Returns the complete graph failure or the first declaration/reference
/// structural failure in the caller-provided order described above.
pub fn validate_batch_local_handles<Node, Handle>(
    nodes: &[Node],
    declarations: &[
        BatchLocalHandleDeclaration<Node::Identity, Handle>
    ],
    references: &[BatchLocalHandleReference<Node::Identity, Handle>],
) -> BatchLocalHandleValidationResult<Node::Identity, Handle>
where
    Node: CommandDependencyNode,
    Node::Identity: Clone,
    Handle: Clone + Ord,
{
    let positions = validated_command_positions(nodes)
        .map_err(|reason| BatchLocalHandleError::Graph { reason })?;
    let mut producers = BTreeMap::<&Handle, &Node::Identity>::new();
    for declaration in declarations {
        if !positions.contains_key(&declaration.command) {
            return Err(BatchLocalHandleError::DeclarationCommandMissing {
                command: declaration.command.clone(),
                handle: declaration.handle.clone(),
            });
        }
        if let Some(first_command) =
            producers.insert(&declaration.handle, &declaration.command)
        {
            return Err(BatchLocalHandleError::DuplicateHandle {
                first_command: first_command.clone(),
                handle: declaration.handle.clone(),
                second_command: declaration.command.clone(),
            });
        }
    }
    for reference in references {
        let Some(command_position) =
            positions.get(&reference.command).copied()
        else {
            return Err(BatchLocalHandleError::ReferenceCommandMissing {
                command: reference.command.clone(),
                handle: reference.handle.clone(),
            });
        };
        let Some(producer) = producers.get(&reference.handle).copied() else {
            return Err(BatchLocalHandleError::UndeclaredHandle {
                command: reference.command.clone(),
                handle: reference.handle.clone(),
            });
        };
        let Some(command) = nodes.get(command_position) else {
            return Err(BatchLocalHandleError::ReferenceCommandMissing {
                command: reference.command.clone(),
                handle: reference.handle.clone(),
            });
        };
        if !command.dependencies().contains(producer) {
            return Err(BatchLocalHandleError::RequiredDependencyMissing {
                command: reference.command.clone(),
                handle: reference.handle.clone(),
                producer: producer.clone(),
            });
        }
    }
    Ok(())
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
) -> CommandGraphValidationResult<Node::Identity>
where
    Node: CommandDependencyNode,
    Node::Identity: Clone,
{
    validated_command_positions(nodes).map(|_positions| ())
}

fn collect_command_positions<Node>(
    nodes: &[Node],
) -> CommandPositionsResult<'_, Node::Identity>
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
    Ok(positions)
}

fn first_forward_dependency<Node>(
    nodes: &[Node],
    positions: &CommandPositions<'_, Node::Identity>,
) -> ForwardDependencyResult<Node::Identity>
where
    Node: CommandDependencyNode,
    Node::Identity: Clone,
{
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
    Ok(first_forward)
}

fn command_graph_has_cycle<Node>(
    nodes: &[Node],
    positions: &CommandPositions<'_, Node::Identity>,
) -> bool
where
    Node: CommandDependencyNode,
{
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
    let mut ready = indegrees
        .iter()
        .enumerate()
        .filter_map(|(position, degree)| (*degree == 0).then_some(position))
        .collect::<VecDeque<_>>();
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
    processed != nodes.len()
}

fn validated_command_positions<Node>(
    nodes: &[Node],
) -> CommandPositionsResult<'_, Node::Identity>
where
    Node: CommandDependencyNode,
    Node::Identity: Clone,
{
    let positions = collect_command_positions(nodes)?;
    let Some((command, dependency)) =
        first_forward_dependency(nodes, &positions)?
    else {
        return Ok(positions);
    };
    if command_graph_has_cycle(nodes, &positions) {
        return Err(CommandGraphError::Cycle);
    }
    Err(CommandGraphError::DependencyAfterCommand {
        command,
        dependency,
    })
}

fn dependency_selection_state<'graph, Node>(
    nodes: &'graph [Node],
    selected: &BTreeSet<Node::Identity>,
) -> SelectionStateResult<'graph, Node::Identity>
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
) -> BoundedDependencyRequirementsResult<Node::Identity>
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
) -> DependencySummaryResult<Node::Identity>
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
) -> DependencyRequirementsResult<Node::Identity>
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
) -> DependencySelectionValidationResult<Node::Identity>
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
