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
//   - Transport-neutral self-contained initial formatting-prompt structure.
// - Must-Not:
//   - Serialize prompt text, compute prompt identity, parse model responses,
//     define semantic command mode, access clipboard, or include session
//     secrets.
// - Allows:
//   - Inputs: Backend-owned task/source/constraints/protocol data plus identity
//     and version.
//   - Outputs: One inspectable complete initial-formatting prompt value.
//   - Side effects: None.
// - Split-When:
//   - Prompt text serialization or candidate-response parsing gains executable
//     authority.
// - Merge-When:
//   - Initial prompt structure becomes inseparable from one application
//     service.
// - Summary:
//   - Makes the one-shot request self-contained without hidden chat context.
// - Description:
//   - Separates source, constraints, protocol, identity, and version inputs.
// - Usage:
//   - Build backend-owned initial formatting intent before clipboard
//     presentation.
// - Defaults:
//   - No prompt wording, hashing, transport, or response semantics are
//     inferred.
//

//! Self-contained one-shot formatting prompt authority before text
//! serialization.

/// User-provided task and complete source material included in one request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormattingPromptSourceInputs<SourceMaterial, Task> {
    /// Complete source material provided to the formatting request.
    pub source_material: SourceMaterial,
    /// Formatting task or assignment intent.
    pub task: Task,
}

/// Current paper/style/output constraints included in one request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormattingPromptConstraintInputs<
    OutputTargets,
    PaperConstraints,
    StyleConstraints,
> {
    /// Requested output targets.
    pub output_targets: OutputTargets,
    /// Current paper constraints.
    pub paper_constraints: PaperConstraints,
    /// Current style constraints.
    pub style_constraints: StyleConstraints,
}

/// Complete backend-owned protocol information required to interpret a
/// response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormattingPromptProtocolInputs<
    DiagnosticExpectations,
    ProvenanceRules,
    ReturnEnvelope,
    SemanticFormat,
    SourceRules,
> {
    /// Diagnostics or ambiguity behavior expected from the model response.
    pub diagnostic_expectations: DiagnosticExpectations,
    /// Rules for source/claim provenance.
    pub provenance_rules: ProvenanceRules,
    /// Required outer return envelope.
    pub return_envelope: ReturnEnvelope,
    /// Complete backend-owned semantic candidate format.
    pub semantic_format: SemanticFormat,
    /// Rules governing provided, derived, cited, and unverified source content.
    pub source_rules: SourceRules,
}

/// Complete one-shot initial formatting request before browser text
/// presentation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OneShotFormattingPrompt<
    ConstraintInputs,
    PromptIdentity,
    ProtocolInputs,
    SourceInputs,
    PromptVersion,
> {
    /// Current paper/style/output constraints.
    pub constraints: ConstraintInputs,
    /// Stable backend-owned prompt identity for these exact prompt inputs.
    pub identity: PromptIdentity,
    /// Complete response protocol and semantic-format authority.
    pub protocol: ProtocolInputs,
    /// Task and complete source material.
    pub source: SourceInputs,
    /// Version of the backend-owned prompt protocol.
    pub version: PromptVersion,
}
