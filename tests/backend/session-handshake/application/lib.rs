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
//   - Regression evidence for browser/backend version compatibility decisions.
// - Must-Not:
//   - Parse HTTP, inspect credentials, or mutate application state.
// - Allows:
//   - Inputs: Deterministic required-version sets.
//   - Outputs: Assertions over compatible and typed incompatible outcomes.
//   - Side effects: None.
// - Split-When:
//   - Optional feature negotiation requires independent compatibility fixtures.
// - Merge-When:
//   - Session startup no longer has an independent compatibility decision.
// - Summary:
//   - Verifies every first-release handshake version dimension.
// - Description:
//   - Mutates each required version independently against the current backend.
// - Usage:
//   - Compile against the handshake port and application components.
// - Defaults:
//   - Requires exact equality across all six required version identities.
//
use atrament_diagnostic::{
    BlockingDisposition, Completeness, DiagnosticCode, Evidence, LocationKind,
    LocationRole, Operation, Remediation, Severity,
};
use atrament_session_handshake_port::{
    HandshakeResult, SessionHandshake, VersionDimension, Versions,
};

#[allow(dead_code)]
#[path = "../../../../src/backend/session-handshake/application/lib.rs"]
mod handshake;

use handshake::HandshakeService;

#[test]
fn current_version_set_is_compatible() {
    let service = HandshakeService;
    let current = HandshakeService::current_versions();
    assert_eq!(service.evaluate(current), HandshakeResult::Compatible {
        versions: current
    },);
}

#[test]
fn every_required_version_mismatch_blocks_with_its_dimension() {
    let service = HandshakeService;
    let current = HandshakeService::current_versions();
    let cases = [
        (VersionDimension::Capability, Versions {
            capability: "atrament.capability/0",
            ..current
        }),
        (VersionDimension::Product, Versions {
            product: "0.0.0",
            ..current
        }),
        (VersionDimension::Profile, Versions {
            profile: "atrament.profile/0",
            ..current
        }),
        (VersionDimension::Prompt, Versions {
            prompt: "atrament.prompt/0",
            ..current
        }),
        (VersionDimension::Protocol, Versions {
            protocol: "atrament.runtime/0",
            ..current
        }),
        (VersionDimension::Renderer, Versions {
            renderer: "atrament.renderer/0",
            ..current
        }),
    ];

    for (dimension, versions) in cases {
        let result = service.evaluate(versions);
        let HandshakeResult::Incompatible {
            diagnostics,
            dimension: actual,
            expected,
            observed,
        } = result
        else {
            panic!("mismatched version must remain an incompatible result");
        };
        assert_eq!(actual, dimension);
        assert_ne!(expected, observed);
        assert_eq!(diagnostics.completeness, Completeness::Complete);
        let [diagnostic] = diagnostics.diagnostics.as_slice() else {
            panic!("version mismatch must return one diagnostic");
        };
        assert_eq!(diagnostic.code, DiagnosticCode::HandshakeVersionMismatch);
        assert_eq!(diagnostic.disposition, BlockingDisposition::Blocking);
        assert_eq!(diagnostic.operation.operation, Operation::SessionHandshake);
        assert_eq!(diagnostic.remediations, [Remediation::UseCompatibleClient]);
        assert_eq!(diagnostic.severity, Severity::Error);
        assert!(matches!(
            diagnostic.evidence.as_slice(),
            [Evidence::RequiredVersion {
                dimension: evidence_dimension,
                expected: evidence_expected,
            }] if *evidence_dimension == version_dimension_name(dimension)
                && *evidence_expected == expected
        ));
        assert!(matches!(
            diagnostic.locations.as_slice(),
            [location]
                if location.kind == LocationKind::Capability
                    && location.role == LocationRole::Primary
                    && location.relationship.is_none()
        ));
    }
}

const fn version_dimension_name(dimension: VersionDimension) -> &'static str {
    match dimension {
        VersionDimension::Capability => "capability",
        VersionDimension::Product => "product",
        VersionDimension::Profile => "profile",
        VersionDimension::Prompt => "prompt",
        VersionDimension::Protocol => "protocol",
        VersionDimension::Renderer => "renderer",
    }
}
