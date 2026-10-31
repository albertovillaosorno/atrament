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

// Browser metadata is explanatory, not an unbounded decoded JSON work queue.
// Current handshake and draft transports emit exactly one diagnostic each.
const MAX_DIAGNOSTIC_ITEMS = 128;
const MAX_DIAGNOSTIC_ITEM_FIELDS = 128;

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

// Decoded JSON fields are own data properties. Do not invoke getters when
// these parsers are also passed objects constructed inside JavaScript.
export function ownDataField(
    value: object,
    field: string,
): unknown {
    const descriptor = Object.getOwnPropertyDescriptor(value, field);
    return descriptor !== undefined && Object.hasOwn(descriptor, "value")
        ? descriptor.value
        : undefined;
}

function isCompleteness(value: unknown): value is DiagnosticCompleteness {
    return value === "complete" || value === "incomplete";
}

function admittedDiagnosticItem(value: unknown): DiagnosticItem | null {
    if (!isRecord(value)) {
        return null;
    }
    // Copy own data descriptors, not the caller's object. Later consumers
    // must not reenter Proxy traps or observe field changes after admission.
    // A decoded JSON item has only string-keyed data fields; bound the work
    // per item as well as the number of items.
    try {
        const fields = Reflect.ownKeys(value);
        if (fields.length > MAX_DIAGNOSTIC_ITEM_FIELDS) {
            return null;
        }
        const snapshot: Record<string, unknown> = {};
        for (const field of fields) {
            if (typeof field !== "string") {
                return null;
            }
            const descriptor = Object.getOwnPropertyDescriptor(value, field);
            if (
                descriptor === undefined
                || !Object.hasOwn(descriptor, "value")
            ) {
                return null;
            }
            Object.defineProperty(snapshot, field, {
                value: descriptor.value,
                enumerable: descriptor.enumerable === true,
                configurable: true,
                writable: true,
            });
        }
        const code = ownDataField(snapshot, "code");
        return typeof code === "string" && code !== ""
            ? snapshot as DiagnosticItem
            : null;
    } catch {
        // In-process traps have no authority to break diagnostic admission.
        return null;
    }
}

function admittedDiagnosticItems(items: unknown[]): DiagnosticItem[] | null {
    // Avoid property getters even when an in-process caller wraps its Array.
    // Ordinary decoded JSON arrays always have an own data length.
    const length = ownDataField(items, "length");
    if (
        typeof length !== "number"
        || !Number.isSafeInteger(length)
        || length < 0
        || length > MAX_DIAGNOSTIC_ITEMS
    ) {
        return null;
    }
    // Both the collection and its item records must be owned snapshots.
    const admitted: DiagnosticItem[] = [];
    for (let index = 0; index < length; index += 1) {
        const item = admittedDiagnosticItem(
            ownDataField(items, String(index)),
        );
        if (item === null) {
            return null;
        }
        admitted.push(item);
    }
    return admitted;
}

export function parseDiagnosticSet(
    value: unknown,
): DiagnosticSetMetadata | null {
    if (!isRecord(value)) {
        return null;
    }
    const version = ownDataField(value, "version");
    const completeness = ownDataField(value, "completeness");
    const items = ownDataField(value, "items");
    if (
        version !== DIAGNOSTIC_VERSION
        || !isCompleteness(completeness)
        || !Array.isArray(items)
    ) {
        return null;
    }
    const admittedItems = admittedDiagnosticItems(items);
    return admittedItems === null
        ? null
        : { completeness, items: admittedItems };
}
