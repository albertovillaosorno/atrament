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

use std::collections::{BTreeMap, BTreeSet};

/// One ordered command and the command identities it explicitly depends on.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandNode<Identity> {
    /// Explicit predecessor command identities required by this command.
    pub dependencies: Vec<Identity>,
    /// Caller-owned command identity, whose representation remains external.
    pub id: Identity,
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

/// Measure exact coarse command and dependency-edge counts.
///
/// # Errors
///
/// Returns [`CommandGraphLimitError::DependencyEdgeCountOverflow`] if summing
/// explicit dependency edges exceeds addressable `usize` range.
pub fn command_graph_size<Identity>(
    nodes: &[CommandNode<Identity>],
) -> Result<CommandGraphSize, CommandGraphLimitError> {
    let mut dependency_edges = 0usize;
    for node in nodes {
        let Some(next) = dependency_edges.checked_add(node.dependencies.len())
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
pub fn validate_command_graph_limits<Identity>(
    nodes: &[CommandNode<Identity>],
    limits: CommandGraphLimits,
) -> Result<CommandGraphSize, CommandGraphLimitError> {
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
pub fn validate_command_graph<Identity>(
    nodes: &[CommandNode<Identity>],
) -> Result<(), CommandGraphError<Identity>>
where
    Identity: Clone + Ord,
{
    let mut indegrees = BTreeMap::new();
    let mut positions = BTreeMap::new();
    for (position, node) in nodes.iter().enumerate() {
        if indegrees.insert(node.id.clone(), 0usize).is_some() {
            return Err(CommandGraphError::DuplicateIdentity {
                command: node.id.clone(),
            });
        }
        let _previous = positions.insert(node.id.clone(), position);
    }

    let mut dependents: BTreeMap<Identity, Vec<Identity>> = BTreeMap::new();
    for node in nodes {
        for dependency in &node.dependencies {
            if dependency == &node.id {
                return Err(CommandGraphError::SelfDependency {
                    command: node.id.clone(),
                });
            }
            if !positions.contains_key(dependency) {
                return Err(CommandGraphError::MissingDependency {
                    command: node.id.clone(),
                    dependency: dependency.clone(),
                });
            }
            if let Some(degree) = indegrees.get_mut(&node.id) {
                *degree = degree.saturating_add(1);
            }
            dependents
                .entry(dependency.clone())
                .or_default()
                .push(node.id.clone());
        }
    }

    let mut ready = BTreeSet::new();
    for (command, degree) in &indegrees {
        if *degree == 0 {
            let _inserted = ready.insert(command.clone());
        }
    }
    let mut processed = 0usize;
    while let Some(command) = ready.pop_first() {
        processed = processed.saturating_add(1);
        if let Some(next_commands) = dependents.get(&command) {
            for next in next_commands {
                if let Some(degree) = indegrees.get_mut(next) {
                    *degree = degree.saturating_sub(1);
                    if *degree == 0 {
                        let _inserted = ready.insert(next.clone());
                    }
                }
            }
        }
    }
    if processed != nodes.len() {
        return Err(CommandGraphError::Cycle);
    }
    for (position, node) in nodes.iter().enumerate() {
        for dependency in &node.dependencies {
            if let Some(dependency_position) = positions.get(dependency)
                && *dependency_position > position
            {
                return Err(CommandGraphError::DependencyAfterCommand {
                    command: node.id.clone(),
                    dependency: dependency.clone(),
                });
            }
        }
    }
    Ok(())
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
pub fn validate_dependency_closed_selection<Identity>(
    nodes: &[CommandNode<Identity>],
    selected: &BTreeSet<Identity>,
) -> Result<(), DependencySelectionError<Identity>>
where
    Identity: Clone + Ord,
{
    if let Err(reason) = validate_command_graph(nodes) {
        return Err(DependencySelectionError::Graph { reason });
    }
    let known = nodes
        .iter()
        .map(|node| node.id.clone())
        .collect::<BTreeSet<_>>();
    if let Some(command) =
        selected.iter().find(|command| !known.contains(*command))
    {
        return Err(DependencySelectionError::UnknownSelection {
            command: command.clone(),
        });
    }
    for node in nodes {
        if !selected.contains(&node.id) {
            continue;
        }
        for dependency in &node.dependencies {
            if !selected.contains(dependency) {
                return Err(
                    DependencySelectionError::MissingRequiredDependency {
                        command: node.id.clone(),
                        dependency: dependency.clone(),
                    },
                );
            }
        }
    }
    Ok(())
}
