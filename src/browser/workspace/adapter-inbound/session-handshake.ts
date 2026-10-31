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
import { ownDataField, parseDiagnosticSet } from "./session-diagnostic.js";

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
] as const;

type VersionDimension = (typeof VERSION_DIMENSIONS)[number];

type VersionRecord = {
    capability: string;
    product: string;
    profile: string;
    prompt: string;
    protocol: string;
    renderer: string;
};

export type HandshakeOutcome =
    | { kind: "compatible" }
    | {
        kind: "incompatible";
        dimension: VersionDimension;
        expected: string;
    }
    | { kind: "invalid" };

const CURRENT_VERSIONS: VersionRecord = {
    capability: CAPABILITY_VERSION,
    product: PRODUCT_VERSION,
    profile: PROFILE_VERSION,
    prompt: PROMPT_VERSION,
    protocol: PROTOCOL_VERSION,
    renderer: RENDERER_VERSION,
};

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === "object"
        && value !== null
        && !Array.isArray(value);
}

function isVersionDimension(value: unknown): value is VersionDimension {
    return typeof value === "string"
        && VERSION_DIMENSIONS.some((dimension) => dimension === value);
}

function hasCurrentVersions(value: unknown): boolean {
    if (!isRecord(value)) {
        return false;
    }
    return VERSION_DIMENSIONS.every((dimension) => {
        return ownDataField(value, dimension) === CURRENT_VERSIONS[dimension];
    });
}

export function handshakeHeaders(
    sessionSecret: string,
): Record<string, string> {
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

export function parseHandshakePayload(value: unknown): HandshakeOutcome {
    // Descriptor traps from in-process objects are invalid payloads.
    try {
        if (!isRecord(value)) {
            return { kind: "invalid" };
        }
        const result = ownDataField(value, "result");
        const versions = ownDataField(value, "versions");
        if (result === "compatible" && hasCurrentVersions(versions)) {
            return { kind: "compatible" };
        }
        const diagnostics = ownDataField(value, "diagnostics");
        if (result !== "incompatible" || !isRecord(diagnostics)) {
            return { kind: "invalid" };
        }
        const metadata = parseDiagnosticSet(diagnostics);
        if (metadata === null || metadata.items.length !== 1) {
            return { kind: "invalid" };
        }
        const diagnostic = metadata.items[0];
        if (diagnostic === undefined) {
            return { kind: "invalid" };
        }
        const dimension = ownDataField(diagnostic, "dimension");
        const expected = ownDataField(diagnostic, "expected");
        if (
            ownDataField(diagnostic, "code")
                !== "atrament.handshake.version-mismatch"
            || !isVersionDimension(dimension)
            || expected !== CURRENT_VERSIONS[dimension]
        ) {
            return { kind: "invalid" };
        }
        return {
            kind: "incompatible",
            dimension,
            expected: CURRENT_VERSIONS[dimension],
        };
    } catch {
        return { kind: "invalid" };
    }
}
