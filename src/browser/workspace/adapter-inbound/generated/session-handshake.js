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
//   - Browser handshake version declarations and response validation.
// - Must-Not:
//   - Persist credentials, perform network requests, or enable DOM controls.
// - Allows:
//   - Inputs: One in-memory credential and one decoded handshake payload.
//   - Outputs: Request headers and a typed browser compatibility outcome.
//   - Side effects: None.
// - Split-When:
//   - Optional capability negotiation becomes independent from required
//     versions.
// - Merge-When:
//   - Browser startup no longer performs a compatibility handshake.
// - Summary:
//   - Defines and validates the browser side of session compatibility.
// - Description:
//   - Keeps handshake wire validation independently executable under Node.
// - Usage:
//   - Build headers before the authenticated POST and validate its JSON
//     payload.
// - Defaults:
//   - Requires the same six exact first-release versions as the backend.
//
import { parseDiagnosticMetadata } from "./session-diagnostic.js";
const CAPABILITY_VERSION = "atrament.capability/1";
const PRODUCT_VERSION = "0.1.0";
const PROFILE_VERSION = "atrament.profile/1";
const PROMPT_VERSION = "atrament.prompt/1";
const PROTOCOL_VERSION = "atrament.runtime/1";
const RENDERER_VERSION = "atrament.renderer/1";
const VERSION_DIMENSIONS = [
    "capability",
    "product",
    "profile",
    "prompt",
    "protocol",
    "renderer",
];
const CURRENT_VERSIONS = {
    capability: CAPABILITY_VERSION,
    product: PRODUCT_VERSION,
    profile: PROFILE_VERSION,
    prompt: PROMPT_VERSION,
    protocol: PROTOCOL_VERSION,
    renderer: RENDERER_VERSION,
};
function isRecord(value) {
    return typeof value === "object" && value !== null;
}
function isVersionDimension(value) {
    return typeof value === "string"
        && VERSION_DIMENSIONS.some((dimension) => dimension === value);
}
function hasCurrentVersions(value) {
    if (!isRecord(value)) {
        return false;
    }
    return VERSION_DIMENSIONS.every((dimension) => {
        return value[dimension] === CURRENT_VERSIONS[dimension];
    });
}
export function handshakeHeaders(sessionSecret) {
    return {
        Authorization: `Bearer ${sessionSecret}`,
        "X-Atrament-Capability-Version": CAPABILITY_VERSION,
        "X-Atrament-Product-Version": PRODUCT_VERSION,
        "X-Atrament-Profile-Version": PROFILE_VERSION,
        "X-Atrament-Prompt-Version": PROMPT_VERSION,
        "X-Atrament-Protocol-Version": PROTOCOL_VERSION,
        "X-Atrament-Renderer-Version": RENDERER_VERSION,
    };
}
export function parseHandshakePayload(value) {
    if (!isRecord(value)) {
        return { kind: "invalid" };
    }
    if (value.result === "compatible"
        && hasCurrentVersions(value.versions)) {
        return { kind: "compatible" };
    }
    if (value.result !== "incompatible" || !isRecord(value.diagnostic)) {
        return { kind: "invalid" };
    }
    const diagnostic = value.diagnostic;
    const metadata = parseDiagnosticMetadata(diagnostic);
    if (metadata?.code !== "atrament.handshake.version-mismatch"
        || !isVersionDimension(diagnostic.dimension)
        || typeof diagnostic.expected !== "string") {
        return { kind: "invalid" };
    }
    return {
        kind: "incompatible",
        dimension: diagnostic.dimension,
        expected: diagnostic.expected,
    };
}
