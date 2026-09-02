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
use atrament_session_handshake_port::{
    HandshakeResult, SessionHandshake, VersionDimension, Versions,
};

#[allow(dead_code)]
#[path = "../src/backend/session-handshake/application/lib.rs"]
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
        assert!(matches!(
            result,
            HandshakeResult::Incompatible {
                dimension: actual,
                ..
            } if actual == dimension
        ));
    }
}
