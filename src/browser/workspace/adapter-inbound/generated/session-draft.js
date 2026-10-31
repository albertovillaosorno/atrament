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
// Preserve every admitted UTF-8 code point, including an initial U+FEFF.
// Response.text() removes a leading BOM and silently replaces invalid bytes.
export function decodeDraftReadBytes(bytes) {
    try {
        return new TextDecoder("utf-8", {
            fatal: true,
            ignoreBOM: true,
        }).decode(bytes);
    }
    catch {
        return null;
    }
}
// Mirror the backend-owned draft field transport limit. A misrouted or
// corrupted response must not allocate unbounded browser text before the
// complete three-field snapshot can be admitted.
const MAX_DRAFT_READ_BYTES = 1_048_576;
export async function decodeDraftReadResponse(response) {
    const declaredLength = response.headers.get("content-length");
    if (declaredLength !== null
        && (!/^(0|[1-9][0-9]*)$/.test(declaredLength)
            || Number(declaredLength) > MAX_DRAFT_READ_BYTES)) {
        void response.body?.cancel().catch(() => { });
        return null;
    }
    // Browser fetch has a streamed body, including for unknown lengths.
    // The fallback handles null/absent bodies and isolated transport stubs.
    if (!response.body) {
        const bytes = await response.arrayBuffer();
        return bytes.byteLength > MAX_DRAFT_READ_BYTES
            ? null
            : decodeDraftReadBytes(bytes);
    }
    const reader = response.body.getReader();
    const decoder = new TextDecoder("utf-8", {
        fatal: true,
        ignoreBOM: true,
    });
    let receivedBytes = 0;
    let result = "";
    try {
        for (;;) {
            const { done, value } = await reader.read();
            if (done) {
                try {
                    return result + decoder.decode();
                }
                catch {
                    return null;
                }
            }
            if (value.byteLength > MAX_DRAFT_READ_BYTES - receivedBytes) {
                void reader.cancel().catch(() => { });
                return null;
            }
            receivedBytes += value.byteLength;
            try {
                result += decoder.decode(value, { stream: true });
            }
            catch {
                void reader.cancel().catch(() => { });
                return null;
            }
        }
    }
    finally {
        reader.releaseLock();
    }
}
// A resource-limit diagnostic is not authority without a declared JSON body.
export function isDraftDiagnosticContentType(value) {
    return value?.trim().toLowerCase() === "application/json; charset=utf-8";
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
