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
//   - Browser request metadata for pre-acceptance session draft replacement.
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
//   - Defines exact browser metadata for protected session draft replacement.
// - Description:
//   - Keeps route names and credential headers independently testable under
//     Node.
// - Usage:
//   - Build a POST request after a compatible authenticated session handshake.
// - Defaults:
//   - Uses text/plain UTF-8 bodies and no ambient browser credentials.
//
export type DraftField = "candidate" | "source" | "task";

export function draftMutationHeaders(
    sessionSecret: string,
): Record<string, string> {
    return {
        Authorization: `Bearer ${sessionSecret}`,
        "Content-Type": "text/plain; charset=utf-8",
    };
}

export function draftMutationTarget(field: DraftField): string {
    return `./api/session/${field}`;
}
