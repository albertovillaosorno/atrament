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

export type DiagnosticCompleteness = "complete" | "incomplete";

export type DiagnosticItem = Record<string, unknown> & {
    code: string;
};

export type DiagnosticSetMetadata = {
    completeness: DiagnosticCompleteness;
    items: DiagnosticItem[];
};

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === "object"
        && value !== null
        && !Array.isArray(value);
}

function isCompleteness(value: unknown): value is DiagnosticCompleteness {
    return value === "complete" || value === "incomplete";
}

function isDiagnosticItem(value: unknown): value is DiagnosticItem {
    return isRecord(value)
        && Object.hasOwn(value, "code")
        && typeof value.code === "string"
        && value.code !== "";
}

function hasValidDiagnosticItems(items: unknown[]): items is DiagnosticItem[] {
    for (let index = 0; index < items.length; index += 1) {
        if (!Object.hasOwn(items, index) || !isDiagnosticItem(items[index])) {
            return false;
        }
    }
    return true;
}

export function parseDiagnosticSet(
    value: unknown,
): DiagnosticSetMetadata | null {
    if (!isRecord(value)) {
        return null;
    }
    if (
        !Object.hasOwn(value, "version")
        || !Object.hasOwn(value, "completeness")
        || !Object.hasOwn(value, "items")
        || value.version !== DIAGNOSTIC_VERSION
        || !isCompleteness(value.completeness)
        || !Array.isArray(value.items)
        || !hasValidDiagnosticItems(value.items)
    ) {
        return null;
    }
    return {
        completeness: value.completeness,
        items: value.items,
    };
}
