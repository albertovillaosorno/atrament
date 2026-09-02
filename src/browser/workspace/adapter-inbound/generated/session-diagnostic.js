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
//   - Browser validation of shared diagnostic namespace and code metadata.
// - Must-Not:
//   - Reimplement diagnostic semantics, choose blocking policy, or perform I/O.
// - Allows:
//   - Inputs: One decoded adapter diagnostic value.
//   - Outputs: Version-admitted diagnostic code metadata or null.
//   - Side effects: None.
// - Split-When:
//   - Browser diagnostic transport gains independently versioned projections.
// - Merge-When:
//   - All browser transports consume diagnostic metadata through another
//     module.
// - Summary:
//   - Admits only current-version diagnostic metadata in the browser adapter.
// - Description:
//   - Keeps namespace validation shared without duplicating backend policy.
// - Usage:
//   - Parse adapter diagnostic metadata before route-specific field validation.
// - Defaults:
//   - Accepts only the first-release diagnostic namespace identity.
//
export const DIAGNOSTIC_VERSION = "atrament.diagnostic/1";
function isRecord(value) {
    return typeof value === "object" && value !== null;
}
function isCompleteness(value) {
    return value === "complete" || value === "incomplete";
}
function isDiagnosticItem(value) {
    return isRecord(value)
        && typeof value.code === "string"
        && value.code !== "";
}
export function parseDiagnosticSet(value) {
    if (!isRecord(value)) {
        return null;
    }
    if (value.version !== DIAGNOSTIC_VERSION
        || !isCompleteness(value.completeness)
        || !Array.isArray(value.items)
        || !value.items.every(isDiagnosticItem)) {
        return null;
    }
    return {
        completeness: value.completeness,
        items: value.items,
    };
}
