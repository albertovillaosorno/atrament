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
//   - Regression evidence for the shared semantic diagnostic envelope.
// - Must-Not:
//   - Choose wire field names, HTTP mappings, or application result classes.
// - Allows:
//   - Inputs: Deterministic synthetic diagnostic values.
//   - Outputs: Assertions over version, typing, relationships, and
//     completeness.
//   - Side effects: Process-local test allocations only.
// - Split-When:
//   - One diagnostic evidence family needs independent acceptance fixtures.
// - Merge-When:
//   - The shared diagnostic model no longer has a cross-capability contract.
// - Summary:
//   - Verifies the transport-independent diagnostic value vocabulary.
// - Description:
//   - Covers stable codes, operation context, relational locations, and
//     evidence.
// - Usage:
//   - Compile this root harness directly against the diagnostic domain module.
// - Defaults:
//   - Uses no adapter serialization or presentation strings.
//
use diagnostic::{
    BlockingDisposition, Completeness, Diagnostic, DiagnosticCode,
    DiagnosticSet, Evidence, EvidenceUnit, LocationKind, LocationRole,
    Operation, OperationBinding, OperationContext, OperationContextKind,
    PhysicalBoundaryEdge, PhysicalLengthQuantity, RelationshipKind,
    Remediation, SemanticLocation, Severity,
};

#[allow(dead_code)]
#[path = "../../../../src/backend/diagnostic/domain/lib.rs"]
mod diagnostic;

#[test]
fn namespace_and_condition_codes_are_stable() {
    assert_eq!(diagnostic::DIAGNOSTIC_VERSION, "atrament.diagnostic/1");
    assert_eq!(
        DiagnosticCode::LayoutFixedRegionOverflow.stable_name(),
        "atrament.layout.fixed-region-overflow",
    );
    assert_eq!(
        DiagnosticCode::HandshakeVersionMismatch.stable_name(),
        "atrament.handshake.version-mismatch",
    );
    assert_eq!(
        DiagnosticCode::SessionDraftResourceLimit.stable_name(),
        "atrament.session-draft.resource-limit",
    );
}

#[test]
fn severity_and_blocking_are_independent_typed_dimensions() {
    let diagnostic = Diagnostic {
        code: DiagnosticCode::HandshakeVersionMismatch,
        disposition: BlockingDisposition::Blocking,
        evidence: vec![Evidence::RequiredVersion {
            dimension: "protocol",
            expected: "atrament.runtime/1",
        }],
        locations: vec![],
        operation: OperationBinding {
            contexts: vec![],
            operation: Operation::SessionHandshake,
        },
        remediations: vec![Remediation::UseCompatibleClient],
        severity: Severity::Warning,
    };
    assert_eq!(diagnostic.severity, Severity::Warning);
    assert_eq!(diagnostic.disposition, BlockingDisposition::Blocking);
}

#[test]
fn operation_context_and_relational_locations_keep_semantic_owners() {
    let diagnostic = Diagnostic {
        code: DiagnosticCode::SessionDraftResourceLimit,
        disposition: BlockingDisposition::Advisory,
        evidence: vec![
            Evidence::LimitExceeded {
                maximum: 1_048_576,
                observed: 1_048_577,
                unit: EvidenceUnit::Bytes,
            },
            Evidence::PhysicalLength {
                micrometres: 6_000,
                quantity: PhysicalLengthQuantity::Overflow,
            },
        ],
        locations: vec![
            SemanticLocation {
                identity: String::from("object:title"),
                kind: LocationKind::Object,
                relationship: None,
                role: LocationRole::Primary,
            },
            SemanticLocation {
                identity: String::from("object:figure"),
                kind: LocationKind::Geometry,
                relationship: Some(RelationshipKind::Collision),
                role: LocationRole::Related,
            },
        ],
        operation: OperationBinding {
            contexts: vec![OperationContext {
                identity: String::from("revision:42"),
                kind: OperationContextKind::AcceptedRevision,
            }],
            operation: Operation::Render,
        },
        remediations: vec![
            Remediation::ChangeConstraint,
            Remediation::InspectRelatedIdentity,
        ],
        severity: Severity::Error,
    };
    assert_eq!(diagnostic.locations.len(), 2);
    assert_eq!(
        diagnostic.locations[1].relationship,
        Some(RelationshipKind::Collision),
    );
    assert_eq!(diagnostic.operation.contexts.len(), 1);
    let set = DiagnosticSet {
        completeness: Completeness::Incomplete,
        diagnostics: vec![diagnostic],
    };
    assert_eq!(set.completeness, Completeness::Incomplete);
    assert_eq!(set.diagnostics.len(), 1);
}

#[test]
fn layout_overflow_has_typed_edge_and_physical_amount_evidence() {
    let diagnostic = Diagnostic {
        code: DiagnosticCode::LayoutFixedRegionOverflow,
        disposition: BlockingDisposition::Blocking,
        evidence: vec![
            Evidence::PhysicalBoundary {
                edge: PhysicalBoundaryEdge::Bottom,
            },
            Evidence::PhysicalLength {
                micrometres: 6_000,
                quantity: PhysicalLengthQuantity::Overflow,
            },
        ],
        locations: vec![],
        operation: OperationBinding {
            contexts: vec![OperationContext {
                identity: String::from("revision:7"),
                kind: OperationContextKind::AcceptedRevision,
            }],
            operation: Operation::Layout,
        },
        remediations: vec![Remediation::ChangeConstraint],
        severity: Severity::Error,
    };
    assert_eq!(diagnostic.evidence[0], Evidence::PhysicalBoundary {
        edge: PhysicalBoundaryEdge::Bottom,
    },);
    assert_eq!(diagnostic.operation.operation, Operation::Layout);
}
