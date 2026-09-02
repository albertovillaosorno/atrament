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
//   - Inbound application contract for browser/backend version compatibility.
// - Must-Not:
//   - Parse HTTP, choose version policy, or perform session authentication.
// - Allows:
//   - Inputs: Six browser-presented required version identities.
//   - Outputs: Compatible current versions or one typed incompatibility.
//   - Side effects: None.
// - Split-When:
//   - Optional-feature negotiation becomes independent from required versions.
// - Merge-When:
//   - Browser startup no longer needs an application compatibility port.
// - Summary:
//   - Defines the transport-independent session handshake application port.
// - Description:
//   - Lets inbound adapters invoke compatibility without owning its policy.
// - Usage:
//   - Implement in the handshake application and inject at process composition.
// - Defaults:
//   - Represents six required first-release version dimensions.
//

//! Inbound application port for Atrament session version compatibility.

/// Result of comparing one browser version set with the current backend.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandshakeResult<'version> {
    /// Every required version identity is compatible.
    Compatible {
        /// Exact backend versions admitted for this session.
        versions: Versions<'static>,
    },
    /// One required version identity is incompatible.
    Incompatible {
        /// Version dimension that blocked the handshake.
        dimension: VersionDimension,
        /// Version identity required by the backend.
        expected: &'static str,
        /// Version identity presented by the browser.
        observed: &'version str,
    },
}

/// Application service capable of deciding browser/backend compatibility.
pub trait SessionHandshake {
    /// Compare one browser version set with the current backend policy.
    fn evaluate<'version>(
        &self,
        versions: Versions<'version>,
    ) -> HandshakeResult<'version>;
}

/// One required version dimension in the first-release compatibility handshake.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VersionDimension {
    /// Output capability behavior contract.
    Capability,
    /// Atrament product build version.
    Product,
    /// Portable profile format contract.
    Profile,
    /// Model prompt contract.
    Prompt,
    /// Browser/backend transport protocol.
    Protocol,
    /// Deterministic renderer behavior contract.
    Renderer,
}

/// Browser/backend version identities carried across the application port.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Versions<'version> {
    /// Output capability behavior version.
    pub capability: &'version str,
    /// Atrament product version.
    pub product: &'version str,
    /// Portable profile format version.
    pub profile: &'version str,
    /// Model prompt contract version.
    pub prompt: &'version str,
    /// Browser/backend protocol version.
    pub protocol: &'version str,
    /// Deterministic renderer behavior version.
    pub renderer: &'version str,
}
