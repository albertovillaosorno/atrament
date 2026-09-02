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
//   - Browser/backend compatibility identities and handshake decisions.
// - Must-Not:
//   - Parse HTTP, inspect credentials, mutate notebook state, or launch
//     browsers.
// - Allows:
//   - Inputs: Product, protocol, prompt, profile, renderer, and capability
//     version identities through the inbound handshake port.
//   - Outputs: Compatible current versions or one required incompatibility.
//   - Side effects: None.
// - Split-When:
//   - Compatibility negotiation needs independently versioned optional
//     features.
// - Merge-When:
//   - Session startup no longer has an application compatibility boundary.
// - Summary:
//   - Decides whether one browser build can operate the current backend.
// - Description:
//   - Keeps version compatibility semantics independent from HTTP transport.
// - Usage:
//   - Inject HandshakeService into the localhost runtime at process
//     composition.
// - Defaults:
//   - Requires exact equality for all six first-release version identities.
//

//! First-release Atrament browser/backend compatibility handshake.

use atrament_diagnostic::{
    BlockingDisposition, Completeness, Diagnostic, DiagnosticCode,
    DiagnosticSet, Evidence, LocationKind, LocationRole, Operation,
    OperationBinding, Remediation, SemanticLocation, Severity,
};
use atrament_session_handshake_port::{
    HandshakeResult, SessionHandshake, VersionDimension, Versions,
};

/// First-release output-capability behavior identity.
pub const CAPABILITY_VERSION: &str = "atrament.capability/1";
/// First-release portable profile format identity.
pub const PROFILE_VERSION: &str = "atrament.profile/1";
/// Product version participating in browser/backend compatibility.
pub const PRODUCT_VERSION: &str = match option_env!("CARGO_PKG_VERSION") {
    Some(version) => version,
    None => "0.1.0",
};
/// First-release model prompt contract identity.
pub const PROMPT_VERSION: &str = "atrament.prompt/1";
/// Local browser/backend protocol identity.
pub const PROTOCOL_VERSION: &str = "atrament.runtime/1";
/// First-release deterministic renderer behavior identity.
pub const RENDERER_VERSION: &str = "atrament.renderer/1";

/// Stateless first-release compatibility service.
#[derive(Clone, Copy, Debug, Default)]
pub struct HandshakeService;

impl HandshakeService {
    /// Return the exact version set required by this backend build.
    #[must_use]
    pub const fn current_versions() -> Versions<'static> {
        Versions {
            capability: CAPABILITY_VERSION,
            product: PRODUCT_VERSION,
            profile: PROFILE_VERSION,
            prompt: PROMPT_VERSION,
            protocol: PROTOCOL_VERSION,
            renderer: RENDERER_VERSION,
        }
    }
}

impl SessionHandshake for HandshakeService {
    fn evaluate<'version>(
        &self,
        versions: Versions<'version>,
    ) -> HandshakeResult<'version> {
        let current = Self::current_versions();
        let checks = [
            (VersionDimension::Product, current.product, versions.product),
            (
                VersionDimension::Protocol,
                current.protocol,
                versions.protocol,
            ),
            (VersionDimension::Prompt, current.prompt, versions.prompt),
            (VersionDimension::Profile, current.profile, versions.profile),
            (
                VersionDimension::Renderer,
                current.renderer,
                versions.renderer,
            ),
            (
                VersionDimension::Capability,
                current.capability,
                versions.capability,
            ),
        ];
        for (dimension, expected, observed) in checks {
            if expected != observed {
                return HandshakeResult::Incompatible {
                    diagnostics: mismatch_diagnostics(dimension, expected),
                    dimension,
                    expected,
                    observed,
                };
            }
        }
        HandshakeResult::Compatible { versions: current }
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

fn mismatch_diagnostics(
    dimension: VersionDimension,
    expected: &'static str,
) -> DiagnosticSet {
    DiagnosticSet {
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
                    version_dimension_name(dimension)
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
    }
}
