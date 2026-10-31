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
import { decodeBoundedUtf8Response } from "./session-diagnostic.js";
import { ownDataField, parseDiagnosticSet } from "./session-diagnostic.js";

export type DraftField = "candidate" | "source" | "task";

export function draftReadHeaders(
    sessionSecret: string,
): Record<string, string> {
    return {
        Authorization: `Bearer ${sessionSecret}`,
    };
}

export function draftReadTarget(field: DraftField): string {
    return `./api/session/${field}`;
}

export function draftMutationHeaders(
    sessionSecret: string,
): Record<string, string> {
    return {
        ...draftReadHeaders(sessionSecret),
        "Content-Type": "text/plain; charset=utf-8",
    };
}

export function draftMutationTarget(field: DraftField): string {
    return draftReadTarget(field);
}

// A successful backend draft read must be a declared UTF-8 text resource.
// Do not treat a same-origin HTML/error page with HTTP 200 as draft state.
export function isDraftReadContentType(value: string | null): boolean {
    return value?.trim().toLowerCase() === "text/plain; charset=utf-8";
}

// Preserve every admitted UTF-8 code point, including an initial U+FEFF.
// Response.text() removes a leading BOM and silently replaces invalid bytes.
export function decodeDraftReadBytes(bytes: ArrayBuffer): string | null {
    try {
        return new TextDecoder("utf-8", {
            fatal: true,
            ignoreBOM: true,
        }).decode(bytes);
    } catch {
        return null;
    }
}

// Mirror the backend-owned draft field transport limit. A misrouted or
// corrupted response must not allocate unbounded browser text before the
// complete three-field snapshot can be admitted.
const MAX_DRAFT_READ_BYTES = 1_048_576;

export async function decodeDraftReadResponse(
    response: Response,
): Promise<string | null> {
    return decodeBoundedUtf8Response(response, MAX_DRAFT_READ_BYTES);
}

// A resource-limit diagnostic is not authority without a declared JSON body.
export function isDraftDiagnosticContentType(value: string | null): boolean {
    return value?.trim().toLowerCase() === "application/json; charset=utf-8";
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === "object"
        && value !== null
        && !Array.isArray(value);
}

export function isResourceLimit(value: unknown): boolean {
    // Descriptor traps from in-process objects are invalid payloads.
    try {
        if (
            !isRecord(value)
            || ownDataField(value, "error") !== "resource_limit"
        ) {
            return false;
        }
        const metadata = parseDiagnosticSet(ownDataField(value, "diagnostics"));
        const item = metadata?.items[0];
        return metadata?.items.length === 1
            && item !== undefined
            && ownDataField(item, "code")
                === "atrament.session-draft.resource-limit";
    } catch {
        return false;
    }
}
