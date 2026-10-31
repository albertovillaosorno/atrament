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
//   - Browser request metadata for pre-acceptance session draft reads and
//     replacements.
// - Must-Not:
//   - Persist draft text, perform network requests, or define notebook
//     authority.
// - Allows:
//   - Inputs: One draft field identity and one in-memory session credential.
//   - Outputs: Same-origin endpoint and explicit request headers.
//   - Side effects: None.
// - Split-When:
//   - Draft fields acquire independently versioned browser transport contracts.
// - Merge-When:
//   - Browser draft replacement is subsumed by another session transport
//     module.
// - Summary:
//   - Defines exact browser metadata for protected session draft transport.
// - Description:
//   - Keeps route names and credential headers independently testable under
//     Node.
// - Usage:
//   - Build authenticated GET or POST requests after a compatible session
//     handshake.
// - Defaults:
//   - Uses text/plain UTF-8 bodies and no ambient browser credentials.
//
import { ownDataField, parseDiagnosticSet } from "./session-diagnostic.js";
export function draftReadHeaders(sessionSecret) {
    return {
        Authorization: `Bearer ${sessionSecret}`,
    };
}
export function draftReadTarget(field) {
    return `./api/session/${field}`;
}
export function draftMutationHeaders(sessionSecret) {
    return {
        ...draftReadHeaders(sessionSecret),
        "Content-Type": "text/plain; charset=utf-8",
    };
}
export function draftMutationTarget(field) {
    return draftReadTarget(field);
}
// A successful backend draft read must be a declared UTF-8 text resource.
// Do not treat a same-origin HTML/error page with HTTP 200 as draft state.
export function isDraftReadContentType(value) {
    return value?.trim().toLowerCase() === "text/plain; charset=utf-8";
}
function isRecord(value) {
    return typeof value === "object"
        && value !== null
        && !Array.isArray(value);
}
export function isResourceLimit(value) {
    // Descriptor traps from in-process objects are invalid payloads.
    try {
        if (!isRecord(value)
            || ownDataField(value, "error") !== "resource_limit") {
            return false;
        }
        const metadata = parseDiagnosticSet(ownDataField(value, "diagnostics"));
        const item = metadata?.items[0];
        return metadata?.items.length === 1
            && item !== undefined
            && ownDataField(item, "code")
                === "atrament.session-draft.resource-limit";
    }
    catch {
        return false;
    }
}
