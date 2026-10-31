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
//   - Run this Cargo harness against the public handshake crate.
// - Defaults:
//   - Requires exact equality across all six required version identities.
//

//! Public session-handshake regression tests.

use atrament_diagnostic::{
    BlockingDisposition, Completeness, Diagnostic, DiagnosticCode,
    DiagnosticSet, Evidence, LocationKind, LocationRole, Operation,
    OperationBinding, Remediation, SemanticLocation, Severity,
};
use atrament_handwriting_profile_container as _;
use atrament_one_shot_formatting_prompt::FORMATTING_PROMPT_VERSION;
use atrament_session_handshake_port::{
    HandshakeResult, SessionHandshake as _, VersionDimension, Versions,
};

use atrament_session_handshake as handshake;

use handshake::HandshakeService;

#[test]
fn handshake_prompt_version_comes_from_the_prompt_domain() {
    assert_eq!(
        HandshakeService::current_versions().prompt,
        FORMATTING_PROMPT_VERSION,
    );
}

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
    let product_mismatch = if current.product == "0.0.0" {
        "atrament-test-product-mismatch"
    } else {
        "0.0.0"
    };
    let cases = [
        (VersionDimension::Capability, Versions {
            capability: "atrament.capability/0",
            ..current
        }),
        (VersionDimension::Product, Versions {
            product: product_mismatch,
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
        let (expected, observed) = match dimension {
            VersionDimension::Capability => {
                (current.capability, versions.capability)
            },
            VersionDimension::Product => (current.product, versions.product),
            VersionDimension::Profile => (current.profile, versions.profile),
            VersionDimension::Prompt => (current.prompt, versions.prompt),
            VersionDimension::Protocol => (current.protocol, versions.protocol),
            VersionDimension::Renderer => (current.renderer, versions.renderer),
        };
        assert_ne!(expected, observed);
        assert_eq!(
            service.evaluate(versions),
            HandshakeResult::Incompatible {
                diagnostics: DiagnosticSet {
                    completeness: Completeness::Complete,
                    diagnostics: vec![Diagnostic {
                        code: DiagnosticCode::HandshakeVersionMismatch,
                        disposition: BlockingDisposition::Blocking,
                        evidence: vec![Evidence::RequiredVersion {
                            dimension: version_dimension_name(dimension),
                            expected,
                        }],
                        locations: vec![SemanticLocation {
                            identity: format!(
                                "handshake:{}",
                                version_dimension_name(dimension),
                            ),
                            kind: LocationKind::Capability,
                            relationship: None,
                            role: LocationRole::Primary,
                        }],
                        operation: OperationBinding {
                            contexts: vec![],
                            operation: Operation::SessionHandshake,
                        },
                        remediations: vec![Remediation::UseCompatibleClient],
                        severity: Severity::Error,
                    }],
                },
                dimension,
                expected,
                observed,
            },
        );
    }
}

#[test]
fn all_version_mismatch_combinations_choose_the_first_required_dimension() {
    let service = HandshakeService;
    let current = HandshakeService::current_versions();
    let product_mismatch = if current.product == "0.0.0" {
        "atrament-test-product-mismatch"
    } else {
        "0.0.0"
    };
    // The six bits are in backend rejection priority, not wire field order.
    for mask in 0u8..=63 {
        let offered = Versions {
            capability: if mask & 0b10_0000 == 0 {
                current.capability
            } else {
                "atrament.capability/0"
            },
            product: if mask & 0b00_0001 == 0 {
                current.product
            } else {
                product_mismatch
            },
            profile: if mask & 0b00_1000 == 0 {
                current.profile
            } else {
                "atrament.profile/0"
            },
            prompt: if mask & 0b00_0100 == 0 {
                current.prompt
            } else {
                "atrament.prompt/0"
            },
            protocol: if mask & 0b00_0010 == 0 {
                current.protocol
            } else {
                "atrament.runtime/0"
            },
            renderer: if mask & 0b01_0000 == 0 {
                current.renderer
            } else {
                "atrament.renderer/0"
            },
        };
        let priority = [
            (
                0b00_0001,
                VersionDimension::Product,
                current.product,
                offered.product,
            ),
            (
                0b00_0010,
                VersionDimension::Protocol,
                current.protocol,
                offered.protocol,
            ),
            (
                0b00_0100,
                VersionDimension::Prompt,
                current.prompt,
                offered.prompt,
            ),
            (
                0b00_1000,
                VersionDimension::Profile,
                current.profile,
                offered.profile,
            ),
            (
                0b01_0000,
                VersionDimension::Renderer,
                current.renderer,
                offered.renderer,
            ),
            (
                0b10_0000,
                VersionDimension::Capability,
                current.capability,
                offered.capability,
            ),
        ];
        let expected = priority
            .into_iter()
            .find(|(bit, _, _, _)| mask & bit != 0)
            .map(|(_, dimension, required, supplied)| {
                (dimension, required, supplied)
            });
        let actual = match service.evaluate(offered) {
            HandshakeResult::Compatible { .. } => None,
            HandshakeResult::Incompatible {
                dimension,
                expected: required,
                observed,
                ..
            } => Some((dimension, required, observed)),
        };
        assert_eq!(actual, expected, "priority mismatch for mask {mask:06b}");
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
