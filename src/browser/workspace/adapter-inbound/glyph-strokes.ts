// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - LicenseRef-Proprietary
// Confidential:
//   - true
// License-File:
//   - LICENSE
//
// Boundary-Contract:
// - Owns:
//   - Explicit local editing of caller-authored Unicode stroke geometry.
// - Must-Not:
//   - Claim full font/ideal-vector export, calibration, or Rust admission.
//   - Write files or credentials without a deliberate download gesture.
// - Allows:
//   - Inputs: Exact Unicode glyphs, finite stroke fields, chosen TSV file.
//   - Outputs: Scaled visual preview and native CLI-compatible TSV download.
//   - Side effects: Local DOM, explicit file read, explicit file download.
// - Split-When:
//   - Actual native glyph-file admission becomes a backend application port.
// - Merge-When:
//   - A full authored-stroke editor takes over this bounded first slice.
// - Summary:
//   - Edits authored segments and opt-in geometric starter approximations.
// - Description:
//   - Rust still validates downloaded files on CLI load or PDF/PNG export.
// - Usage:
//   - Initialized after the same-origin localhost session is admitted.
// - Defaults:
//   - Empty in memory; no browser storage, implicit export, or silent fallback.
//

//! Bounded local authoring surface for explicit glyph line segments.

const HEADER = "atrament.glyph-strokes.v1";
const MAX_BYTES = 524_288;
const MAX_STROKES = 4096;
const SVG_NS = "http://www.w3.org/2000/svg";
const CIRCLE_SEGMENTS = 48;
const OPEN_ARC_SEGMENTS = 36;
const ARCH_SEGMENTS = 16;
const AXES = ["x1", "y1", "x2", "y2", "width"] as const;
type Axis = typeof AXES[number];
type Point = { x: number; y: number };
type Stroke = {
    grapheme: string;
    x1: number;
    y1: number;
    x2: number;
    y2: number;
    width: number;
    color: string;
};

function exactGrapheme(value: string): boolean {
    // In Unicode mode an astral scalar is one character; lone UTF-16
    // surrogates still match. TextEncoder would silently replace them with
    // U+FFFD on TSV download, changing the caller's authored glyph identity.
    if (!value || /[\uD800-\uDFFF]/u.test(value)
        || new TextEncoder().encode(value).length > 64
        // Native flow and coverage audits reserve these two Unicode row
        // separators as structure; they can never name explicit pen ink.
        || /[\p{Cc}\u2028\u2029]/u.test(value) || value === " "
        // Only standalone default-ignorable scalars are refused; a ZWJ
        // emoji or a visible glyph with variation selector is still exact.
        || /^[\u200B-\u200D\u2060\uFE0E\uFE0F\uFEFF]$/u
            .test(value)) {
        return false;
    }
    if (typeof Intl.Segmenter !== "function") return false;
    return [...new Intl.Segmenter(undefined, {
        granularity: "grapheme",
    }).segment(value)].length === 1;
}

function boundedInteger(raw: string, maximum: number, label: string): number {
    if (!/^[0-9]+$/.test(raw)) {
        throw new Error(`${label} requires a nonnegative whole micrometre`);
    }
    const value = Number(raw);
    if (!Number.isSafeInteger(value) || value > maximum
        || (label === "width" && value < 80)) {
        throw new Error(`${label} outside native bounds`);
    }
    return value;
}

function validateStroke(stroke: Stroke): Stroke {
    if (!exactGrapheme(stroke.grapheme)) {
        throw new Error("Exactly one visible Unicode grapheme is required");
    }
    for (const axis of AXES) {
        const maximum = axis === "width" ? 1500
            : axis.startsWith("x") ? 3500 : 4500;
        boundedInteger(String(stroke[axis]), maximum, axis);
    }
    if (stroke.x1 === stroke.x2 && stroke.y1 === stroke.y2) {
        throw new Error("Stroke endpoints must differ");
    }
    if (!/^#[0-9a-fA-F]{6}$/.test(stroke.color)) {
        throw new Error("Ink color requires #RRGGBB");
    }
    return { ...stroke, color: stroke.color.toUpperCase() };
}

export function parseGlyphLibrary(text: string): Stroke[] {
    if (new TextEncoder().encode(text).length > MAX_BYTES
        || !text.endsWith("\n") || text.includes("\r")) {
        throw new Error(
            "Glyph TSV exceeds 512 KiB or has bad line endings",
        );
    }
    const lines = text.slice(0, -1).split("\n");
    if (lines.shift() !== HEADER || lines.length < 1
        || lines.length > MAX_STROKES) {
        throw new Error("Invalid native glyph TSV header or stroke count");
    }
    return lines.map((line, index) => {
        const fields = line.split("\t");
        if (fields.length !== 7) {
            throw new Error(`Glyph TSV line ${index + 2} needs seven fields`);
        }
        const [grapheme, x1, y1, x2, y2, width, color] = fields;
        if ([grapheme, x1, y1, x2, y2, width, color]
            .some((field) => field === undefined)) {
            throw new Error("Glyph TSV field missing");
        }
        return validateStroke({
            grapheme: grapheme ?? "", color: color ?? "",
            x1: boundedInteger(x1 ?? "", 3500, "x1"),
            y1: boundedInteger(y1 ?? "", 4500, "y1"),
            x2: boundedInteger(x2 ?? "", 3500, "x2"),
            y2: boundedInteger(y2 ?? "", 4500, "y2"),
            width: boundedInteger(width ?? "", 1500, "width"),
        });
    });
}

export function encodeGlyphLibrary(strokes: Stroke[]): string {
    if (strokes.length < 1 || strokes.length > MAX_STROKES) {
        throw new Error("Save requires 1..4096 authored glyph strokes");
    }
    const lines = strokes.map((item) => {
        const stroke = validateStroke(item);
        return [stroke.grapheme, stroke.x1, stroke.y1, stroke.x2,
            stroke.y2, stroke.width, stroke.color].join("\t");
    });
    const result = `${HEADER}\n${lines.join("\n")}\n`;
    if (new TextEncoder().encode(result).length > MAX_BYTES) {
        throw new Error("Glyph TSV exceeds 524288 bytes");
    }
    return result;
}

// Match the Rust --inspect-paths definition: exact directed endpoints and
// identical pen width/RGB across consecutive strokes of one exact grapheme.
// These are candidate paths, not persisted pen-down/pen-lift metadata.
export type CandidatePenPath = {
    first: number;
    last: number;
    start: Point;
    end: Point;
    width: number;
    color: string;
};

export function candidatePenPaths(
    strokes: readonly Stroke[], grapheme: string,
): CandidatePenPath[] {
    if (!exactGrapheme(grapheme)) {
        throw new Error("Path inspection requires one exact visible glyph");
    }
    const chosen = strokes.filter((stroke) => stroke.grapheme === grapheme);
    if (chosen.length === 0) return [];
    const paths: CandidatePenPath[] = [];
    const record = (first: number, endExclusive: number): void => {
        const start = chosen[first]!;
        const end = chosen[endExclusive - 1]!;
        paths.push({
            first: first + 1, last: endExclusive,
            start: { x: start.x1, y: start.y1 },
            end: { x: end.x2, y: end.y2 },
            width: start.width, color: start.color.toUpperCase(),
        });
    };
    let first = 0;
    for (let index = 1; index < chosen.length; index += 1) {
        const before = chosen[index - 1]!;
        const after = chosen[index]!;
        if (before.x2 !== after.x1 || before.y2 !== after.y1
            || before.width !== after.width
            || before.color.toUpperCase() !== after.color.toUpperCase()) {
            record(first, index);
            first = index;
        }
    }
    record(first, chosen.length);
    return paths;
}

// Select a later segment as the entry to an already closed pen loop.
// A cyclic reorder leaves its actual geometry/direction untouched and
// retains interleaved exact-Unicode TSV slots and the original input.
export function rotateClosedCandidatePath(
    strokes: readonly Stroke[], grapheme: string, selectedIndex: number,
): { strokes: Stroke[]; selectedIndex: number } {
    if (!Number.isSafeInteger(selectedIndex) || selectedIndex < 0
        || strokes[selectedIndex]?.grapheme !== grapheme) {
        throw new Error("Path start needs a selected exact glyph stroke");
    }
    const indices = strokes.flatMap((stroke, index) =>
        stroke.grapheme === grapheme ? [index] : []);
    const position = indices.indexOf(selectedIndex) + 1;
    const path = candidatePenPaths(strokes, grapheme).find((candidate) =>
        position >= candidate.first && position <= candidate.last);
    if (!path || path.first === path.last || position === path.first) {
        throw new Error("Closed path needs a later selected start stroke");
    }
    if (path.start.x !== path.end.x || path.start.y !== path.end.y) {
        throw new Error("Path start change requires a closed pen loop");
    }
    const slots = indices.slice(path.first - 1, path.last);
    const shift = position - path.first;
    const reordered = [...slots.slice(shift), ...slots.slice(0, shift)]
        .map((index) => strokes[index]!);
    const next = strokes.slice();
    slots.forEach((index, offset) => { next[index] = reordered[offset]!; });
    encodeGlyphLibrary(next);
    return { strokes: next, selectedIndex: slots[0]! };
}

// Close one selected open path by an explicit short straight pen line.
// Reuse native 1200 µm join bounds, preserve interleaved exact Unicode rows,
// and validate the entire proposed v1 TSV before touching the local draft.
export function closeCandidatePath(
    strokes: readonly Stroke[], grapheme: string, selectedIndex: number,
): { strokes: Stroke[]; selectedIndex: number } {
    if (!Number.isSafeInteger(selectedIndex) || selectedIndex < 0
        || strokes[selectedIndex]?.grapheme !== grapheme) {
        throw new Error("Path closure needs a selected exact glyph stroke");
    }
    const indices = strokes.flatMap((stroke, index) => {
        return stroke.grapheme === grapheme ? [index] : [];
    });
    const position = indices.indexOf(selectedIndex) + 1;
    const path = candidatePenPaths(strokes, grapheme).find((candidate) => {
        return position >= candidate.first && position <= candidate.last;
    });
    if (!path || path.first === path.last) {
        throw new Error("Path closure requires two continuous strokes");
    }
    const dx = path.end.x - path.start.x;
    const dy = path.end.y - path.start.y;
    const squared = dx * dx + dy * dy;
    if (squared === 0) throw new Error("Path is already closed");
    if (squared > 1200 * 1200) {
        throw new Error("Path closure exceeds 1200 µm gap limit");
    }
    if (strokes.length === MAX_STROKES) {
        throw new Error("Path closure exceeds 4096 authored strokes");
    }
    const endSlot = indices[path.last - 1]!;
    const closure: Stroke = {
        grapheme,
        x1: path.end.x, y1: path.end.y,
        x2: path.start.x, y2: path.start.y,
        width: path.width, color: path.color,
    };
    const next = strokes.slice();
    next.splice(endSlot + 1, 0, closure);
    encodeGlyphLibrary(next);
    return { strokes: next, selectedIndex: endSlot + 1 };
}

// Reverse every directed segment in one touching path AND its sequence.
// Interleaved glyph rows stay in their original v1 TSV positions.
export function reverseCandidatePath(
    strokes: readonly Stroke[], grapheme: string, selectedIndex: number,
): { strokes: Stroke[]; selectedIndex: number } {
    if (!Number.isSafeInteger(selectedIndex) || selectedIndex < 0
        || strokes[selectedIndex]?.grapheme !== grapheme) {
        throw new Error("Path reversal needs a selected stroke of this glyph");
    }
    const indices = strokes.flatMap((stroke, index) => {
        return stroke.grapheme === grapheme ? [index] : [];
    });
    const position = indices.indexOf(selectedIndex);
    const path = candidatePenPaths(strokes, grapheme).find((candidate) => {
        return position + 1 >= candidate.first
            && position + 1 <= candidate.last;
    });
    if (!path || path.first === path.last) {
        throw new Error("Path reversal requires two continuous strokes");
    }
    const slots = indices.slice(path.first - 1, path.last);
    const reversed = slots.map((index) => strokes[index]!).reverse()
        .map((stroke) => ({
            ...stroke, x1: stroke.x2, y1: stroke.y2,
            x2: stroke.x1, y2: stroke.y1,
        }));
    const next = strokes.slice();
    slots.forEach((index, i) => { next[index] = reversed[i]!; });
    return {
        strokes: next,
        selectedIndex: slots[slots.length - 1
            - slots.indexOf(selectedIndex)]!,
    };
}

// Translate one complete candidate pen path while retaining every joint,
// interleaved Unicode TSV slot, and the original pen/segment direction.
// Validate the ENTIRE moved contour before returning a changed draft.
export function translateCandidatePath(
    strokes: readonly Stroke[], grapheme: string, selectedIndex: number,
    dx: number, dy: number,
): { strokes: Stroke[]; selectedIndex: number } {
    if (!Number.isSafeInteger(selectedIndex) || selectedIndex < 0
        || strokes[selectedIndex]?.grapheme !== grapheme) {
        throw new Error("Path translation needs a selected exact glyph stroke");
    }
    if (!Number.isSafeInteger(dx) || !Number.isSafeInteger(dy)
        || Math.abs(dx) > 3500 || Math.abs(dy) > 4500
        || (dx === 0 && dy === 0)) {
        throw new Error("Path offsets need nonzero bounded whole µm values");
    }
    const indices = strokes.flatMap((stroke, index) =>
        stroke.grapheme === grapheme ? [index] : []);
    const position = indices.indexOf(selectedIndex) + 1;
    const path = candidatePenPaths(strokes, grapheme).find((candidate) =>
        position >= candidate.first && position <= candidate.last);
    if (!path) throw new Error("Selected glyph has no candidate pen path");
    const slots = indices.slice(path.first - 1, path.last);
    const translated = slots.map((index) => {
        const stroke = strokes[index]!;
        const moved = {
            ...stroke, x1: stroke.x1 + dx, y1: stroke.y1 + dy,
            x2: stroke.x2 + dx, y2: stroke.y2 + dy,
        };
        validateStroke(moved);
        const radius = stroke.width / 2;
        if ([moved.x1, moved.x2].some((x) =>
            x < radius || x > 3500 - radius)
            || [moved.y1, moved.y2].some((y) =>
                y < radius || y > 4500 - radius)) {
            throw new Error("Translated pen contour clips the glyph cell");
        }
        return moved;
    });
    const next = strokes.slice();
    slots.forEach((index, offset) => { next[index] = translated[offset]!; });
    encodeGlyphLibrary(next);
    return { strokes: next, selectedIndex };
}

// Reorder neighboring candidate paths, keeping every stroke within each
// path in the same direction/order and every other glyph in the same TSV slot.
export function moveCandidatePath(
    strokes: readonly Stroke[], grapheme: string, selectedIndex: number,
    direction: -1 | 1,
): { strokes: Stroke[]; selectedIndex: number } {
    if (!Number.isSafeInteger(selectedIndex) || selectedIndex < 0
        || strokes[selectedIndex]?.grapheme !== grapheme
        || (direction !== -1 && direction !== 1)) {
        throw new Error("Path move requires a selected exact glyph stroke");
    }
    const indices = strokes.flatMap((stroke, index) => {
        return stroke.grapheme === grapheme ? [index] : [];
    });
    const position = indices.indexOf(selectedIndex) + 1;
    const paths = candidatePenPaths(strokes, grapheme);
    const chosen = paths.findIndex((path) => {
        return position >= path.first && position <= path.last;
    });
    const adjacent = chosen + direction;
    if (chosen < 0 || adjacent < 0 || adjacent >= paths.length) {
        throw new Error("No adjacent continuous pen path to move past");
    }
    const earlier = paths[Math.min(chosen, adjacent)]!;
    const later = paths[Math.max(chosen, adjacent)]!;
    const slots = indices.slice(earlier.first - 1, later.last);
    const former = slots.slice(0, earlier.last - earlier.first + 1);
    const latter = slots.slice(former.length);
    const reordered = [...latter, ...former].map((index) => strokes[index]!);
    const next = strokes.slice();
    slots.forEach((index, offset) => { next[index] = reordered[offset]!; });
    const withinChosen = position - paths[chosen]!.first;
    const newPosition = direction === -1 ? withinChosen
        : latter.length + withinChosen;
    return {
        strokes: next, selectedIndex: slots[newPosition]!,
    };
}

// The construction is defined by an analytic circle; v1 TSV can only
// preserve its rounded line-segment approximation, not that ideal primitive.
function acuteAccent(
    cx: number, cy: number, radius: number, pen: number,
    dx: number, lift: number, topOffset = radius,
) {
    const baseline = cy - topOffset - pen - 120 - lift;
    return {
        x1: cx - Math.round(radius * 0.35) + dx, y1: baseline,
        x2: cx + Math.round(radius * 0.25) + dx,
        y2: baseline - Math.round(radius * 0.45),
    };
}

// Two quadratic arcs give the upper story a smooth horizontal top tangent.
// The bowl remains an analytic circle. Only the sampled ink is serialized.
function upperStory(cx: number, cy: number, radius: number) {
    const start = {
        x: Math.round(cx - radius / Math.SQRT2),
        y: Math.round(cy - radius / Math.SQRT2),
    };
    const peak = { x: cx, y: cy - 2 * radius };
    const end = { x: cx + radius, y: cy - radius };
    const left = { x: cx - radius, y: peak.y };
    const right = { x: cx + radius, y: peak.y };
    return { start, peak, end, left, right };
}

// The n upper hump and u lower bowl use two smooth quadratic arcs.
// Their exact guides are retained only in SVG; v1 TSV samples the arcs.
// The C-shaped circular arc is exact in SVG; its open ends leave a
// quarter-circle gap to the right. The native TSV rounds 36 arc chords.
function openCirclePoint(
    cx: number, cy: number, radius: number, step: number,
): Point {
    const radians = Math.PI / 4
        + 3 * Math.PI / 2 * step / OPEN_ARC_SEGMENTS;
    return {
        x: cx + radius * Math.cos(radians),
        y: cy + radius * Math.sin(radians),
    };
}

function dottedI(cx: number, cy: number, radius: number, pen: number) {
    const dotY = cy - radius;
    const stemY = cy - Math.round(radius * 0.4);
    if (stemY - (dotY + 80) < pen + 100) {
        throw new Error("Dotted i requires visible pen-clearance above stem");
    }
    return {
        dot: { from: { x: cx, y: dotY },
            to: { x: cx, y: dotY + 80 } },
        stem: { from: { x: cx, y: stemY },
            to: { x: cx, y: cy + radius } },
    };
}

// The j is a straight downstroke, a short separate dot, and one ideal
// quadratic turn. Native TSV samples only the turn into ordered chords.
function hookedJ(cx: number, cy: number, radius: number, pen: number) {
    const dotted = dottedI(cx, cy, radius, pen);
    const turnY = cy + Math.round(radius * 1.4);
    const start = { x: cx, y: cy + Math.round(radius * 0.7) };
    const control = { x: cx, y: turnY };
    const end = { x: cx - Math.round(radius / 2), y: turnY };
    return {
        stem: { from: dotted.stem.from, to: start },
        dot: dotted.dot,
        turn: [start, control, end] as [Point, Point, Point],
    };
}

// Lowercase g reuses the unmodified ideal o circle and adds an ordered
// right stem plus a separate quadratic descender with bounded pen width.
function hookedG(cx: number, cy: number, radius: number) {
    const x = cx + radius;
    const from = { x, y: cy + Math.round(radius * 0.65) };
    const bottom = cy + Math.round(radius * 1.45);
    const control = { x, y: bottom };
    const end = { x: cx + Math.round(radius / 2), y: bottom };
    return {
        stem: { from: { x, y: cy }, to: from },
        turn: [from, control, end] as [Point, Point, Point],
    };
}

// Closed-bowl letters share exactly the same circle as o/a. Ascenders
// extend above the bowl; descenders extend below it. In v1 only their
// integer straight-stem geometry survives the explicit TSV export.
function loopStem(
    cx: number, cy: number, radius: number,
    g: "a" | "á" | "b" | "d" | "p" | "q",
): { from: Point; to: Point } {
    const left = g === "b" || g === "p";
    const ascender = g === "b" || g === "d";
    const descender = g === "p" || g === "q";
    const x = cx + (left ? -radius : radius);
    return {
        from: { x, y: ascender ? cy - Math.round(radius * 1.5)
            : cy - radius },
        to: { x, y: descender ? cy + Math.round(radius * 1.5)
            : cy + radius },
    };
}

function roundedStory(
    cx: number, cy: number, radius: number, g: "n" | "u" | "h",
) {
    const high = cy - radius;
    const low = cy + radius;
    const bowY = g === "u" ? low : high;
    const leftX = cx - radius;
    const rightX = cx + radius;
    return {
        start: { x: leftX, y: cy },
        peak: { x: cx, y: bowY },
        end: { x: rightX, y: cy },
        left: { x: leftX, y: bowY },
        right: { x: rightX, y: bowY },
        firstStem: {
            from: { x: leftX, y: g === "u" ? high : low },
            to: { x: leftX, y: g === "h"
                ? cy - Math.round(radius * 1.5)
                : g === "n" ? high : cy },
        },
        lastStem: {
            from: { x: rightX, y: cy },
            to: { x: rightX, y: g === "u" ? high : low },
        },
    };
}

// An m is two exact quadratic upper humps inside the same horizontal
// cell, plus three explicit stems. The m SVG keeps those quadratics; TSV
// exports 64 rounded chords and three independently ordered pen lines.
// Straight forms retain exact SVG/TSV line segments, not rounded chords.
// The t crossbar reuses the e guide's existing bounded bar coordinates.
// Both V-shaped forms use exact integer line endpoints in SVG and TSV.
// Stroke ordering follows the left-to-right connected polyline.
function zigzag(cx: number, cy: number, radius: number, g: "v" | "w") {
    const left = cx - radius;
    const right = cx + radius;
    const high = cy - radius;
    const low = cy + radius;
    return g === "v" ? [
        { x: left, y: high }, { x: cx, y: low },
        { x: right, y: high },
    ] : [
        { x: left, y: high },
        { x: cx - Math.round(radius / 2), y: low },
        { x: cx, y: cy },
        { x: cx + Math.round(radius / 2), y: low },
        { x: right, y: high },
    ];
}

// These straight strokes share one geometric definition for SVG guides
// and native TSV. Disconnected lines retain separate pen-down commands.
type StraightGlyph = "v" | "w" | "k" | "x" | "y" | "z";
type StraightLine = { from: Point; to: Point };

function straightLines(
    cx: number, cy: number, radius: number, glyph: StraightGlyph,
): StraightLine[] {
    const left = cx - radius;
    const right = cx + radius;
    const upperY = cy - radius;
    const bottom = cy + radius;
    const chain = (vertices: Point[]): StraightLine[] =>
        vertices.slice(1).map((to, index) => {
            const from = vertices[index];
            if (from === undefined) throw new Error("Missing straight vertex");
            return { from, to };
        });
    if (glyph === "v" || glyph === "w") {
        return chain(zigzag(cx, cy, radius, glyph));
    }
    if (glyph === "x") return [
        { from: { x: left, y: upperY }, to: { x: right, y: bottom } },
        { from: { x: right, y: upperY }, to: { x: left, y: bottom } },
    ];
    if (glyph === "z") return chain([
        { x: left, y: upperY }, { x: right, y: upperY },
        { x: left, y: bottom }, { x: right, y: bottom },
    ]);
    if (glyph === "y") return [
        { from: { x: left, y: upperY }, to: { x: cx, y: cy } },
        { from: { x: right, y: upperY }, to: { x: cx, y: cy } },
        { from: { x: cx, y: cy }, to: {
            x: cx - Math.round(radius / 2),
            y: cy + Math.round(radius * 1.5),
        } },
    ];
    return [
        { from: { x: left, y: cy - Math.round(radius * 1.5) },
            to: { x: left, y: bottom } },
        { from: { x: right, y: upperY }, to: { x: left, y: cy } },
        { from: { x: left, y: cy }, to: { x: right, y: bottom } },
    ];
}

// Manually selected Spanish/English punctuation uses explicit pen-down
// order. Questions retain three ideal quadratic arcs in SVG; native v1 TSV
// contains only sampled chords. Inverted marks rotate all control points.
type PunctuationGlyph = "." | "," | "!" | "¡" | "?" | "¿";
type MarkPiece = { from: Point; to: Point; control?: Point };

function punctuationParts(
    cx: number, cy: number, r: number, glyph: PunctuationGlyph,
): MarkPiece[] {
    const point = (x: number, y: number): Point => ({
        x: cx + Math.round(x * r), y: cy + Math.round(y * r),
    });
    const dot = (): MarkPiece => ({
        from: point(0, 0.78), to: point(0, 0.9),
    });
    if (glyph === ".") return [dot()];
    if (glyph === ",") return [{
        from: point(0.12, 0.72), to: point(-0.17, 0.98),
    }];
    const upsideDown = glyph === "¡" || glyph === "¿";
    const rotate = (p: Point): Point => upsideDown
        ? { x: 2 * cx - p.x, y: 2 * cy - p.y } : p;
    const rotatePart = (piece: MarkPiece): MarkPiece => ({
        from: rotate(piece.from), to: rotate(piece.to),
        ...(piece.control ? { control: rotate(piece.control) } : {}),
    });
    const stem = glyph === "!" || glyph === "¡"
        ? [{ from: point(0, -0.9), to: point(0, 0.35) }, dot()]
        : [
            { from: point(-0.55, -0.45),
                control: point(-0.65, -0.9), to: point(0, -0.9) },
            { from: point(0, -0.9),
                control: point(0.9, -0.9), to: point(0.65, -0.25) },
            { from: point(0.65, -0.25),
                control: point(0.6, 0), to: point(0, 0.25) },
            { from: point(0, 0.25), to: point(0, 0.35) },
            dot(),
        ];
    return stem.map(rotatePart);
}

// Guillemets are exact twin chevrons. Curly quotes have independent
// sampled quadratic pen strokes, each kept as a separate pen gesture.
// Mirror native rounded points for exact left/right quote symmetry.
type QuoteGlyph = "«" | "»" | "‘" | "’" | "“" | "”";
type QuoteSource = "«" | "‘" | "“";

function quoteParts(
    cx: number, cy: number, r: number, source: QuoteSource,
): MarkPiece[] {
    const point = (x: number, y: number): Point => ({
        x: cx + Math.round(x * r), y: cy + Math.round(y * r),
    });
    if (source === "«") {
        const chevron = (x: number): MarkPiece[] => [
            { from: point(x + 0.4, -0.5), to: point(x, 0) },
            { from: point(x, 0), to: point(x + 0.4, 0.5) },
        ];
        return [...chevron(-0.7), ...chevron(-0.05)];
    }
    const curl = (shift: number): MarkPiece => ({
        from: point(shift - 0.05, -0.85),
        control: point(shift - 0.32, -0.5),
        to: point(shift + 0.04, -0.18),
    });
    return source === "‘" ? [curl(0)]
        : [curl(-0.32), curl(0.32)];
}

function quotedSource(glyph: QuoteGlyph): QuoteSource {
    return glyph === "»" ? "«"
        : glyph === "’" ? "‘" : glyph === "”" ? "“"
            : glyph;
}

function quoteMirror(
    cx: number, glyph: QuoteGlyph, p: Point,
): Point {
    return ["»", "’", "”"].includes(glyph)
        ? { x: 2 * cx - p.x, y: p.y } : p;
}

// Math starters retain ideal SVG lines/quadratics; native v1 samples curves
// into integer pen chords. ASCII hyphen and Unicode minus stay distinct.
const MATH_GLYPHS = ["+", "-", "−", "±", "∓", "=", "×", "÷",
    "√", "π", "∞", "≠", "<", ">", "≤", "≥", "∧", "∨",
    "∩", "∪", "∈", "∉", "∋", "∌", "⊂", "⊆", "⊃", "⊇",
    "⊄", "⊈", "⊅", "⊉", "∑", "∫",
    "∂", "Δ", "≈", "→", "←", "↔", "↑", "↓", "↕", "∝",
    "∅", "∇", "≡", "≢", "∥", "⊥", "∼", "≅",
    "∀", "∃", "∄", "∴", "∵", "⊤", "⊢", "⊨",
    "∠", "∟", "⊬", "⊭", "¬", "⇒", "⇔",
    "≪", "≫", "≮", "≯", "≰", "≱", "↗", "↘",
    "↖", "↙", "⇐", "↦", "↤", "∣", "∤",
    "′", "″", "‴", "⋅", "∗", "∖", "∦",
    "⌊", "⌋", "⌈", "⌉"] as const;
// Additional exact notation is manually selected, not automatically filled.
// A full text/math fill currently sits just 12 strokes below the native cap.
const MANUAL_MATH_GLYPHS = [
    "⟨", "⟩", "α", "β", "θ", "λ", "μ", "γ", "δ", "σ", "ρ", "τ",
    "ε", "ω", "κ", "χ", "φ", "ψ", "ζ", "η", "ξ", "ι", "ο", "υ",
    "ν", "ς", "Γ", "Ω", "Π", "Σ", "Λ", "Θ", "Ξ", "Φ", "Ψ", "Χ",
    "Β", "Υ", "Α", "Ε", "Ι", "Ο", "Ζ", "Η",
    "Κ", "Μ", "Ν", "Ρ", "Τ", "∏", "∐", "∮", "∬", "∭",
    "ℕ", "ℤ", "ℚ", "ℝ", "ℂ",
    "∛", "∜", "⊕", "⊗", "⊙", "⊖", "⊘", "⋂", "⋃", "⋀", "⋁", "⊓", "⊔",
    "⊏", "⊐", "⊑", "⊒", "⊞", "⊟", "⊠", "⊡",
    "≺", "≻", "≼", "≽", "⊲", "⊳", "⊴", "⊵",
    "⋖", "⋗", "⋘", "⋙",
    "≶", "≷", "⋚", "⋛",
] as const;
const SELECTABLE_MATH_GLYPHS = [
    ...MATH_GLYPHS, ...MANUAL_MATH_GLYPHS,
] as const;
// Extra manual marks do not change the 93-glyph math fill.
const CIRCLE_MARKS = ["°", "%"] as const;
// Currency is manual, not an implicit text/math baseline substitution.
const CURRENCY_MARKS = ["$", "€"] as const;
type CurrencyMark = typeof CURRENCY_MARKS[number];
type CircleMark = typeof CIRCLE_MARKS[number];
type MathGlyph = typeof SELECTABLE_MATH_GLYPHS[number];

function mathParts(
    cx: number, cy: number, radius: number, glyph: MathGlyph,
): MarkPiece[] {
    const point = (x: number, y: number): Point => ({
        x: cx + Math.round(radius * x),
        y: cy + Math.round(radius * y),
    });
    const horizontal = (y: number): MarkPiece => ({
        from: point(-0.7, y), to: point(0.7, y),
    });
    const vertical = (): MarkPiece => ({
        from: point(0, -0.7), to: point(0, 0.7),
    });
    if (glyph === "-" || glyph === "−") return [horizontal(0)];
    if (glyph === "+") return [horizontal(0), vertical()];
    if (glyph === "=") return [horizontal(-0.3), horizontal(0.3)];
    // Number-theory divisibility is one exact upright pen stroke.
    // Negated divisibility retains it and lifts for a crossing slash;
    // neither form borrows the ASCII pipe or another glyph's ink.
    if (glyph === "∣" || glyph === "∤") return [
        { from: point(0, -0.8), to: point(0, 0.8) },
        ...(glyph === "∤" ? [
            { from: point(-0.65, 0.65), to: point(0.65, -0.65) },
        ] : []),
    ];
    // Set difference is its own slanted backslash-like Unicode ink.
    // Nonparallel keeps both directed parallel uprights with a lifted slash.
    if (glyph === "∖") return [
        { from: point(-0.7, -0.75), to: point(0.7, 0.75) },
    ];
    if (glyph === "∦") return [
        ...mathParts(cx, cy, radius, "∥"),
        { from: point(-0.7, 0.7), to: point(0.7, -0.7) },
    ];
    // Floor and ceiling are independently authored Unicode delimiters.
    // Each vertical and its lower/upper foot share one directed pen path;
    // neither is borrowed from a square bracket in the caller's TSV.
    if (glyph === "⌊" || glyph === "⌋"
        || glyph === "⌈" || glyph === "⌉") {
        const left = glyph === "⌊" || glyph === "⌈";
        const floor = glyph === "⌊" || glyph === "⌋";
        const edge = left ? -0.68 : 0.68;
        const end = left ? 0.4 : -0.4;
        const stemStart = floor ? -0.75 : 0.75;
        const corner = -stemStart;
        return [
            { from: point(edge, stemStart), to: point(edge, corner) },
            { from: point(edge, corner), to: point(end, corner) },
        ];
    }
    // Separate exact directed ink for parallel and perpendicular geometry.
    // Neither substitutes for a Latin letter or ordinary equality glyph.
    if (glyph === "∥") return [
        { from: point(-0.36, -0.85), to: point(-0.36, 0.85) },
        { from: point(0.36, -0.85), to: point(0.36, 0.85) },
    ];
    if (glyph === "⊥") return [
        { from: point(0, -0.8), to: point(0, 0.7) },
        horizontal(0.7),
    ];
    // Manual logic quantifiers and a three-dot inference mark are exact
    // Unicode shapes, not substitution for Latin A/E or arithmetic dots.
    if (glyph === "∀") return [
        { from: point(-0.7, -0.8), to: point(0, 0.85) },
        { from: point(0, 0.85), to: point(0.7, -0.8) },
        { from: point(-0.42, 0.15), to: point(0.42, 0.15) },
    ];
    if (glyph === "∃" || glyph === "∄") return [
        { from: point(0.7, -0.75), to: point(0.7, 0.75) },
        { from: point(-0.6, -0.75), to: point(0.7, -0.75) },
        { from: point(-0.6, 0), to: point(0.7, 0) },
        { from: point(-0.6, 0.75), to: point(0.7, 0.75) },
        ...(glyph === "∄" ? [{ from: point(-0.75, 0.85),
            to: point(0.75, -0.85) }] : []),
    ];
    if (glyph === "∴") return [
        { from: point(0, -0.72), to: point(0, -0.58) },
        { from: point(-0.55, 0.48), to: point(-0.55, 0.62) },
        { from: point(0.55, 0.48), to: point(0.55, 0.62) },
    ];
    // Because reverses the therefore dot arrangement without aliasing it.
    // Short real strokes remain nonzero in native v1 TSV pen geometry.
    if (glyph === "∵") return [
        { from: point(-0.55, -0.62), to: point(-0.55, -0.48) },
        { from: point(0.55, -0.62), to: point(0.55, -0.48) },
        { from: point(0, 0.58), to: point(0, 0.72) },
    ];
    // Mathematical truth / derivation / semantic entailment have distinct
    // exact Unicode keys and independent lifted pen paths, not a reusable
    // Latin T or an inferred symbol from an authored perpendicular sign.
    if (glyph === "⊤") return [
        horizontal(-0.7),
        { from: point(0, -0.7), to: point(0, 0.7) },
    ];
    if (glyph === "⊢") return [
        { from: point(-0.65, -0.75), to: point(-0.65, 0.75) },
        { from: point(-0.65, 0), to: point(0.75, 0) },
    ];
    if (glyph === "⊨") return [
        { from: point(-0.82, -0.75), to: point(-0.82, 0.75) },
        { from: point(-0.55, -0.75), to: point(-0.55, 0.75) },
        { from: point(-0.82, 0), to: point(0.75, 0) },
    ];
    // School-angle contours keep one continuous directed ray path. A
    // right angle has a vertical corner, never an alternate angle alias.
    if (glyph === "∠") return [
        { from: point(0.78, 0.7), to: point(-0.68, 0.7) },
        { from: point(-0.68, 0.7), to: point(0.55, -0.7) },
    ];
    if (glyph === "∟") return [
        { from: point(-0.68, -0.78), to: point(-0.68, 0.7) },
        { from: point(-0.68, 0.7), to: point(0.75, 0.7) },
    ];
    // Non-derivability and non-entailment retain their own exact Unicode
    // keys, each preserving its positive turnstile with a lifted slash.
    if (glyph === "⊬" || glyph === "⊭") return [
        ...mathParts(cx, cy, radius, glyph === "⊬" ? "⊢" : "⊨"),
        { from: point(-0.84, 0.82), to: point(0.86, -0.82) },
    ];
    // Explicit propositional negation is a connected overbar with a
    // descending right terminal, never a disguised Latin dash.
    if (glyph === "¬") return [
        { from: point(-0.75, -0.35), to: point(0.7, -0.35) },
        { from: point(0.7, -0.35), to: point(0.7, 0.55) },
    ];
    // Implication and equivalence use TWO lifted shafts and independent
    // arrowheads. Their Unicode identities cannot borrow the single-
    // shaft → or ↔ geometry from the same authored glyph library.
    if (glyph === "⇒") return [
        { from: point(-0.8, -0.16), to: point(0.45, -0.16) },
        { from: point(-0.8, 0.16), to: point(0.45, 0.16) },
        { from: point(0.2, -0.55), to: point(0.8, 0) },
        { from: point(0.8, 0), to: point(0.2, 0.55) },
    ];
    // Converse implication keeps two left-bound shafts plus one
    // connected left head, not a re-keyed single arrow.
    if (glyph === "⇐") return [
        { from: point(0.8, -0.16), to: point(-0.45, -0.16) },
        { from: point(0.8, 0.16), to: point(-0.45, 0.16) },
        { from: point(-0.2, -0.55), to: point(-0.8, 0) },
        { from: point(-0.8, 0), to: point(-0.2, 0.55) },
    ];
    if (glyph === "⇔") return [
        { from: point(-0.4, -0.16), to: point(0.4, -0.16) },
        { from: point(-0.4, 0.16), to: point(0.4, 0.16) },
        { from: point(-0.2, -0.55), to: point(-0.8, 0) },
        { from: point(-0.8, 0), to: point(-0.2, 0.55) },
        { from: point(0.2, -0.55), to: point(0.8, 0) },
        { from: point(0.8, 0), to: point(0.2, 0.55) },
    ];
    // Identity and negated identity have three independent horizontal
    // strokes. Negation adds its own separately lifted diagonal ink;
    // neither key is inferred from an authored '=' or '≠'.
    if (glyph === "≡" || glyph === "≢") return [
        horizontal(-0.65), horizontal(0), horizontal(0.65),
        ...(glyph === "≢" ? [{ from: point(-0.75, 0.8),
            to: point(0.75, -0.8) }] : []),
    ];
    if (glyph === "×") return [
        { from: point(-0.6, -0.6), to: point(0.6, 0.6) },
        { from: point(0.6, -0.6), to: point(-0.6, 0.6) },
    ];
    const line = (x1: number, y1: number,
        x2: number, y2: number): MarkPiece => ({
        from: point(x1, y1), to: point(x2, y2),
    });
    // Inner-product angle brackets are taller, sharper connected strokes
    // than ordinary comparisons. They are manual to respect the TSV cap.
    if (glyph === "⟨") return [
        line(0.62, -0.8, -0.58, 0),
        line(-0.58, 0, 0.62, 0.8),
    ];
    if (glyph === "⟩") return [
        line(-0.62, -0.8, 0.58, 0),
        line(0.58, 0, -0.62, 0.8),
    ];
    // Mathematical primes represent derivatives/arc minutes independently
    // of quotes. Each upper stroke lifts the pen; the centered multiply
    // dot and star do not inherit punctuation or ASCII asterisk geometry.
    if (glyph === "′") return [line(0, -0.8, -0.25, -0.15)];
    if (glyph === "″") return [
        line(-0.33, -0.8, -0.58, -0.15),
        line(0.33, -0.8, 0.08, -0.15),
    ];
    if (glyph === "‴") return [
        line(-0.55, -0.8, -0.8, -0.15),
        line(0, -0.8, -0.25, -0.15),
        line(0.55, -0.8, 0.3, -0.15),
    ];
    if (glyph === "⋅") return [line(-0.08, 0, 0.08, 0)];
    if (glyph === "∗") return [
        line(-0.7, 0, 0.7, 0),
        line(-0.35, -0.6, 0.35, 0.6),
        line(0.35, -0.6, -0.35, 0.6),
    ];
    // ± retains two independently lifted horizontal marks. The upper
    // plus has an intersecting stem; the lower minus never contacts it.
    if (glyph === "±") return [
        horizontal(-0.32), line(0, -0.85, 0, 0.2), horizontal(0.75),
    ];
    // ∓ is independent ink, not an implicit rotation or fallback of ±.
    // Its upper minus and lower plus use separate pen-lifted strokes.
    if (glyph === "∓") return [
        horizontal(-0.75), horizontal(0.32), line(0, -0.2, 0, 0.85),
    ];
    if (glyph === "≠") return [
        horizontal(-0.3), horizontal(0.3), line(-0.55, 0.75, 0.55, -0.75),
    ];
    // Strict chevrons are their own exact Unicode keys. Their two
    // connected strokes also form the respective <= and >= contours;
    // only the inclusive signs add a separately lifted baseline.
    if (glyph === "<" || glyph === "≤") return [
        line(0.55, -0.6, -0.6, 0.05),
        line(-0.6, 0.05, 0.55, 0.6),
        ...(glyph === "≤" ? [horizontal(0.88)] : []),
    ];
    if (glyph === ">" || glyph === "≥") return [
        line(-0.55, -0.6, 0.6, 0.05),
        line(0.6, 0.05, -0.55, 0.6),
        ...(glyph === "≥" ? [horizontal(0.88)] : []),
    ];
    // Normal-subgroup relations are closed three-sided triangular paths.
    // Each inclusive form lifts the pen for a separate equality underbar;
    // these exact keys cannot borrow curved precedence or subset ink.
    if (["⊲", "⊳", "⊴", "⊵"].includes(glyph)) {
        const mirror = glyph === "⊳" || glyph === "⊵" ? -1 : 1;
        const triangle: MarkPiece[] = [
            line(mirror * 0.68, -0.65, -mirror * 0.72, 0),
            line(-mirror * 0.72, 0, mirror * 0.68, 0.65),
            line(mirror * 0.68, 0.65, mirror * 0.68, -0.65),
        ];
        if (glyph === "⊴" || glyph === "⊵") {
            triangle.push(line(-0.68, 0.96, 0.68, 0.96));
        }
        return triangle;
    }
    // Precedes/follows are two joined quadratic curves rather than the
    // sharp < and > comparisons. Inclusive relations add a lifted underbar.
    // All four forms remain opt-in, outside the full automatic math batch.
    if (["≺", "≻", "≼", "≽"].includes(glyph)) {
        const mirror = glyph === "≻" || glyph === "≽" ? -1 : 1;
        const curve: MarkPiece[] = [
            { from: point(mirror * 0.65, -0.65),
                control: point(-mirror * 0.72, -0.65),
                to: point(-mirror * 0.72, 0) },
            { from: point(-mirror * 0.72, 0),
                control: point(-mirror * 0.72, 0.65),
                to: point(mirror * 0.65, 0.65) },
        ];
        if (glyph === "≼" || glyph === "≽") {
            curve.push(line(-0.65, 0.96, 0.65, 0.96));
        }
        return curve;
    }
    // Manual dotted strict comparisons draw a continuous angled pair
    // then lift for a short, separately editable interior-side dot.
    // Mirroring coordinates preserves the distinct exact Unicode keys.
    if (glyph === "⋖" || glyph === "⋗") {
        const mirror = glyph === "⋗" ? -1 : 1;
        return [
            line(mirror * 0.45, -0.7, -mirror * 0.52, 0),
            line(-mirror * 0.52, 0, mirror * 0.45, 0.7),
            line(mirror * 0.8, -0.09, mirror * 0.8, 0.09),
        ];
    }
    // Triple chevrons are three separately lifted joined angular pen
    // gestures, never repeat-filled or silently mapped to ≪ and ≫.
    if (glyph === "⋘" || glyph === "⋙") {
        const mirror = glyph === "⋙" ? -1 : 1;
        return [-0.6, 0, 0.6].flatMap((center) => [
            line(center + mirror * 0.15, -0.72,
                center - mirror * 0.15, 0),
            line(center - mirror * 0.15, 0,
                center + mirror * 0.15, 0.72),
        ]);
    }
    // Stacked trichotomy signs keep two independently joined angular
    // comparisons, with a separately lifted middle equality bar for ⋚/⋛.
    // Exact glyphs do not alias the single <, > or ≤/≥ constructions.
    if (["≶", "≷", "⋚", "⋛"].includes(glyph)) {
        const upper = glyph === "≷" || glyph === "⋛" ? -1 : 1;
        return [
            line(upper * 0.65, -0.88, -upper * 0.45, -0.6),
            line(-upper * 0.45, -0.6, upper * 0.65, -0.32),
            line(-upper * 0.65, 0.32, upper * 0.45, 0.6),
            line(upper * 0.45, 0.6, -upper * 0.65, 0.88),
            ...(["⋚", "⋛"].includes(glyph)
                ? [line(-0.55, 0, 0.55, 0)] : []),
        ];
    }
    // Doubled chevrons are two separately lifted, connected comparisons.
    // Negated strict signs keep their exact positive contour and then lift
    // the pen for a diagonal slash; no fallback to other Unicode signs.
    if (glyph === "≪") return [
        line(-0.12, -0.65, -0.76, 0),
        line(-0.76, 0, -0.12, 0.65),
        line(0.72, -0.65, 0.08, 0),
        line(0.08, 0, 0.72, 0.65),
    ];
    if (glyph === "≫") return [
        line(-0.72, -0.65, -0.08, 0),
        line(-0.08, 0, -0.72, 0.65),
        line(0.12, -0.65, 0.76, 0),
        line(0.76, 0, 0.12, 0.65),
    ];
    if (glyph === "≮" || glyph === "≯") return [
        ...mathParts(cx, cy, radius, glyph === "≮" ? "<" : ">"),
        line(-0.82, 0.82, 0.82, -0.82),
    ];
    // Negated inclusive comparisons retain their own equality bars;
    // the diagonal crosses both the chevron and the lower bar.
    if (glyph === "≰" || glyph === "≱") return [
        ...mathParts(cx, cy, radius, glyph === "≰" ? "≤" : "≥"),
        line(-0.7, 1.0, 0.7, -1.0),
    ];
    // A diagonal arrow's shaft is one gesture; its joined arrowhead is
    // lifted and remains distinct from the horizontal/vertical arrows.
    if (glyph === "↗") return [
        line(-0.7, 0.7, 0.7, -0.7),
        line(0.7, 0.1, 0.7, -0.7),
        line(0.7, -0.7, -0.1, -0.7),
    ];
    if (glyph === "↘") return [
        line(-0.7, -0.7, 0.7, 0.7),
        line(-0.1, 0.7, 0.7, 0.7),
        line(0.7, 0.7, 0.7, -0.1),
    ];
    // Both remaining diagonals retain a separate, two-stroke arrowhead
    // and their own Unicode identity, never a reflected saved key.
    if (glyph === "↖") return [
        line(0.7, 0.7, -0.7, -0.7),
        line(-0.7, 0.1, -0.7, -0.7),
        line(-0.7, -0.7, 0.1, -0.7),
    ];
    if (glyph === "↙") return [
        line(0.7, -0.7, -0.7, 0.7),
        line(0.1, 0.7, -0.7, 0.7),
        line(-0.7, 0.7, -0.7, -0.1),
    ];
    // Logical wedges are exact Unicode ink, never substitutions for the
    // visually similar Latin v or strict-comparison chevrons.
    if (glyph === "∧") return [
        line(-0.65, 0.6, 0, -0.6),
        line(0, -0.6, 0.65, 0.6),
    ];
    if (glyph === "∨") return [
        line(-0.65, -0.6, 0, 0.6),
        line(0, 0.6, 0.65, -0.6),
    ];
    // Square lattice meet/join retain one connected three-sided contour,
    // distinct from rounded ∩/∪ and their larger ⋂/⋃ counterparts.
    // Both square corners and the bar are drawn without a pen lift.
    if (glyph === "⊓" || glyph === "⊔") {
        const sign = glyph === "⊓" ? 1 : -1;
        return [
            line(-0.75, sign * 0.72, -0.75, -sign * 0.72),
            line(-0.75, -sign * 0.72, 0.75, -sign * 0.72),
            line(0.75, -sign * 0.72, 0.75, sign * 0.72),
        ];
    }
    // Square subset/superset are connected three-sided open contours.
    // Equal-or-square variants lift for a physically separate underbar;
    // neither direction borrows rounded subset nor square lattice ink.
    if (["⊏", "⊐", "⊑", "⊒"].includes(glyph)) {
        const mirror = glyph === "⊐" || glyph === "⊒" ? -1 : 1;
        const subset: MarkPiece[] = [
            line(mirror * 0.72, -0.62, -mirror * 0.72, -0.62),
            line(-mirror * 0.72, -0.62, -mirror * 0.72, 0.62),
            line(-mirror * 0.72, 0.62, mirror * 0.72, 0.62),
        ];
        if (glyph === "⊑" || glyph === "⊒") {
            subset.push(line(-mirror * 0.72, 0.95,
                mirror * 0.72, 0.95));
        }
        return subset;
    }
    // Exact square operators keep a closed four-edge perimeter. Their
    // plus, minus, times or center-dot ink is lifted inside the border;
    // circular operator contours are never silently substituted.
    if (["⊞", "⊟", "⊠", "⊡"].includes(glyph)) {
        const perimeter = [
            line(-0.78, -0.78, 0.78, -0.78),
            line(0.78, -0.78, 0.78, 0.78),
            line(0.78, 0.78, -0.78, 0.78),
            line(-0.78, 0.78, -0.78, -0.78),
        ];
        if (glyph === "⊞") return [...perimeter,
            line(-0.4, 0, 0.4, 0), line(0, -0.4, 0, 0.4)];
        if (glyph === "⊟") return [...perimeter,
            line(-0.4, 0, 0.4, 0)];
        if (glyph === "⊠") return [...perimeter,
            line(-0.4, -0.4, 0.4, 0.4),
            line(0.4, -0.4, -0.4, 0.4)];
        return [...perimeter, line(-0.06, 0, 0.06, 0)];
    }
    // N-ary logical AND/OR uses exact, independently selected wide wedges,
    // not implicit enlargement or substitution of ordinary ∧/∨ ink.
    // Both arms join at their peak in one uninterrupted pen path.
    if (glyph === "⋀" || glyph === "⋁") {
        const sign = glyph === "⋀" ? 1 : -1;
        return [
            line(-0.84, sign * 0.95, 0, -sign * 0.95),
            line(0, -sign * 0.95, 0.84, sign * 0.95),
        ];
    }
    // N-ary intersection and union have taller stems and a wider rounded
    // middle than the regular ∩/∪. All four directed pieces join as one
    // pen-down path; these exact Unicode keys remain manual-only ink.
    if (glyph === "⋂" || glyph === "⋃") {
        const sign = glyph === "⋂" ? 1 : -1;
        return [
            line(-0.78, sign * 0.87, -0.78, -sign * 0.12),
            { from: point(-0.78, -sign * 0.12),
                control: point(-0.78, -sign * 0.85),
                to: point(0, -sign * 0.85) },
            { from: point(0, -sign * 0.85),
                control: point(0.78, -sign * 0.85),
                to: point(0.78, -sign * 0.12) },
            line(0.78, -sign * 0.12, 0.78, sign * 0.87),
        ];
    }
    // Intersection and union are true paired quadratic guides. Native
    // TSV uses one continuous 32-chord path per exact Unicode glyph,
    // never substituting the two-stroke logical AND/OR wedges.
    if (glyph === "∩") return [
        { from: point(-0.65, 0.65),
            control: point(-0.65, -0.6), to: point(0, -0.6) },
        { from: point(0, -0.6),
            control: point(0.65, -0.6), to: point(0.65, 0.65) },
    ];
    if (glyph === "∪") return [
        { from: point(-0.65, -0.65),
            control: point(-0.65, 0.6), to: point(0, 0.6) },
        { from: point(0, 0.6),
            control: point(0.65, 0.6), to: point(0.65, -0.65) },
    ];
    // Exact membership and reverse-membership keep independent Unicode
    // identities. Their ideal two-quadratic bows mirror before sampling;
    // a separately lifted crossbar stays in reading order. Negated forms
    // lift again for an independent diagonal strike, never an implicit
    // fallback for an unsaved key.
    if (["∈", "∉", "∋", "∌"].includes(glyph)) {
        const reverse = glyph === "∋" || glyph === "∌";
        const bowPoint = (x: number, y: number): Point =>
            point(reverse ? -x : x, y);
        const parts: MarkPiece[] = [
            { from: bowPoint(0.65, -0.65),
                control: bowPoint(-0.65, -0.65),
                to: bowPoint(-0.65, 0) },
            { from: bowPoint(-0.65, 0),
                control: bowPoint(-0.65, 0.65),
                to: bowPoint(0.65, 0.65) },
            reverse ? line(-0.55, 0, 0.65, 0)
                : line(-0.65, 0, 0.55, 0),
        ];
        if (glyph === "∉" || glyph === "∌") {
            parts.push(line(-0.85, 0.85, 0.85, -0.85));
        }
        return parts;
    }
    // Set inclusion uses four-quadratic contours facing left or right.
    // Reflect ideal control points, not sampled chords, preserving the
    // true SVG guide and exact Rust TSV for each independent Unicode key.
    // Inclusive signs lift for a lower equality bar; the negated forms
    // additionally lift for a distinct crossing slash without inferring
    // the plain counterpart from a saved library.
    if (["⊂", "⊆", "⊃", "⊇", "⊄", "⊈", "⊅", "⊉"]
        .includes(glyph)) {
        const facing = ["⊃", "⊇", "⊅", "⊉"].includes(glyph) ? -1 : 1;
        const facingPoint = (x: number, y: number): Point =>
            point(facing * x, y);
        const contour: MarkPiece[] = [
            { from: facingPoint(0.65, -0.6),
                control: facingPoint(-0.1, -0.72),
                to: facingPoint(-0.52, -0.4) },
            { from: facingPoint(-0.52, -0.4),
                control: facingPoint(-0.8, -0.3),
                to: facingPoint(-0.8, 0) },
            { from: facingPoint(-0.8, 0),
                control: facingPoint(-0.8, 0.3),
                to: facingPoint(-0.52, 0.4) },
            { from: facingPoint(-0.52, 0.4),
                control: facingPoint(-0.1, 0.72),
                to: facingPoint(0.65, 0.6) },
        ];
        if (["⊆", "⊇", "⊈", "⊉"].includes(glyph)) {
            contour.push(horizontal(0.9));
        }
        if (["⊄", "⊈", "⊅", "⊉"].includes(glyph)) {
            contour.push(line(-0.85, 0.85, 0.85, -0.85));
        }
        return contour;
    }
    if (glyph === "√") return [
        line(-0.85, 0.2, -0.5, 0.1), line(-0.5, 0.1, -0.1, 0.8),
        line(-0.1, 0.8, 0.45, -0.75),
        line(0.45, -0.75, 0.85, -0.75),
    ];
    if (glyph === "π") return [
        line(-0.8, -0.6, 0.8, -0.6),
        line(-0.5, -0.6, -0.5, 0.75),
        line(0.5, -0.6, 0.5, 0.55),
        line(0.5, 0.55, 0.7, 0.75),
    ];
    if (glyph === "∑") return [
        line(-0.75, -0.85, 0.75, -0.85),
        line(0.75, -0.85, -0.35, 0),
        line(-0.35, 0, 0.75, 0.85),
        line(0.75, 0.85, -0.75, 0.85),
    ];
    // Nabla is a continuously drawn inverted triangle and stays a
    // separate exact Unicode identity from the upward delta.
    if (glyph === "∇") return [
        line(-0.8, -0.9, 0.8, -0.9),
        line(0.8, -0.9, 0, 0.9),
        line(0, 0.9, -0.8, -0.9),
    ];
    if (glyph === "Δ") return [
        line(0, -0.9, -0.8, 0.9),
        line(-0.8, 0.9, 0.8, 0.9),
        line(0.8, 0.9, 0, -0.9),
    ];
    // The shaft lifts before an independently directed arrowhead;
    // its two arrowhead segments meet at the same physical tip.
    if (glyph === "→") return [
        line(-0.8, 0, 0.8, 0),
        line(0.2, -0.55, 0.8, 0),
        line(0.8, 0, 0.2, 0.55),
    ];
    // Reverse arrowhead points are separate directed pen paths. The
    // bidirectional arrow has one shaft and one head on either end,
    // without treating either exact Unicode glyph as a right arrow.
    if (glyph === "←") return [
        line(0.8, 0, -0.8, 0),
        line(-0.2, -0.55, -0.8, 0),
        line(-0.8, 0, -0.2, 0.55),
    ];
    if (glyph === "↔") return [
        line(-0.8, 0, 0.8, 0),
        line(-0.2, -0.55, -0.8, 0),
        line(-0.8, 0, -0.2, 0.55),
        line(0.2, -0.55, 0.8, 0),
        line(0.8, 0, 0.2, 0.55),
    ];
    // Function mapsto arrows keep an upright endpoint bar, one directed
    // shaft and a separate connected arrowhead. These exact symbols never
    // inherit the ordinary arrows or borrow an authored glyph's ink.
    if (glyph === "↦") return [
        line(-0.75, -0.65, -0.75, 0.65),
        line(-0.75, 0, 0.8, 0),
        line(0.2, -0.55, 0.8, 0),
        line(0.8, 0, 0.2, 0.55),
    ];
    if (glyph === "↤") return [
        line(0.75, -0.65, 0.75, 0.65),
        line(0.75, 0, -0.8, 0),
        line(-0.2, -0.55, -0.8, 0),
        line(-0.8, 0, -0.2, 0.55),
    ];
    // Vertical arrows keep their own exact Unicode and ordered shaft.
    // Each separate head joins at its tip without making an implied
    // continuous stroke across the opposite head or another glyph.
    if (glyph === "↑") return [
        line(0, 0.8, 0, -0.8),
        line(-0.55, -0.2, 0, -0.8),
        line(0, -0.8, 0.55, -0.2),
    ];
    if (glyph === "↓") return [
        line(0, -0.8, 0, 0.8),
        line(-0.55, 0.2, 0, 0.8),
        line(0, 0.8, 0.55, 0.2),
    ];
    if (glyph === "↕") return [
        line(0, -0.8, 0, 0.8),
        line(-0.55, -0.2, 0, -0.8),
        line(0, -0.8, 0.55, -0.2),
        line(-0.55, 0.2, 0, 0.8),
        line(0, 0.8, 0.55, 0.2),
    ];
    const curve = (start: [number, number],
        handle: [number, number], end: [number, number]): MarkPiece => ({
        from: point(...start), control: point(...handle), to: point(...end),
    });
    // Common Greek mathematics symbols have exact, manually selected
    // Unicode keys. Sample their analytic bowls into native pen chords;
    // do not add them to the nearly full automatic 93-glyph math batch.
    if (glyph === "α") return [
        curve([0, -0.8], [0.65, -0.8], [0.65, 0]),
        curve([0.65, 0], [0.65, 0.8], [0, 0.8]),
        curve([0, 0.8], [-0.65, 0.8], [-0.65, 0]),
        curve([-0.65, 0], [-0.65, -0.8], [0, -0.8]),
        line(0.65, 0, 0.75, 0.8),
    ];
    if (glyph === "β") return [
        line(-0.55, -0.85, -0.55, 0.9),
        curve([-0.55, -0.78], [0.8, -0.95], [0.65, -0.25]),
        curve([0.65, -0.25], [0.7, 0], [-0.55, 0]),
        curve([-0.55, 0], [0.9, 0.1], [0.65, 0.5]),
        curve([0.65, 0.5], [0.35, 1], [-0.55, 0.8]),
    ];
    if (glyph === "θ") return [
        curve([0, -0.85], [0.65, -0.85], [0.65, 0]),
        curve([0.65, 0], [0.65, 0.85], [0, 0.85]),
        curve([0, 0.85], [-0.65, 0.85], [-0.65, 0]),
        curve([-0.65, 0], [-0.65, -0.85], [0, -0.85]),
        line(-0.65, 0, 0.65, 0),
    ];
    if (glyph === "λ") return [
        line(-0.38, -0.85, -0.75, 0.8),
        line(-0.38, -0.85, 0.75, 0.8),
    ];
    // Lowercase mu has a continuous descended left leg, two curved
    // bowls and a right exit. This is exact Greek ink, not Latin u.
    if (glyph === "μ") return [
        line(-0.65, 1.05, -0.65, -0.6),
        curve([-0.65, -0.6], [-0.65, 0.65], [0, 0.65]),
        curve([0, 0.65], [0.65, 0.65], [0.65, -0.6]),
        line(0.65, -0.6, 0.65, 0.65),
    ];
    // Lowercase gamma deliberately lifts before its descender instead
    // of reusing a Latin y or merging two unrelated pen directions.
    if (glyph === "γ") return [
        curve([-0.75, -0.65], [-0.2, 0.35], [0.1, 0.1]),
        curve([0.1, 0.1], [0.45, -0.4], [0.75, -0.65]),
        line(0.1, 0.1, 0.1, 1.05),
    ];
    // Lowercase delta is its own lower bowl with an ascending diagonal;
    // sigma has a closed rounded sweep and a separately lifted top exit.
    // These exact Greek shapes never reuse Latin o or Greek theta ink.
    if (glyph === "δ") return [
        curve([0, -0.25], [0.7, -0.25], [0.7, 0.35]),
        curve([0.7, 0.35], [0.7, 0.9], [0, 0.9]),
        curve([0, 0.9], [-0.7, 0.9], [-0.7, 0.35]),
        curve([-0.7, 0.35], [-0.7, -0.25], [0, -0.25]),
        line(0.4, -0.25, -0.25, -0.95),
    ];
    if (glyph === "σ") return [
        curve([0, -0.7], [0.7, -0.7], [0.7, 0]),
        curve([0.7, 0], [0.7, 0.7], [0, 0.7]),
        curve([0, 0.7], [-0.7, 0.7], [-0.7, 0]),
        curve([-0.7, 0], [-0.7, -0.7], [0, -0.7]),
        line(0.15, -0.7, 0.85, -0.7),
    ];
    // Lowercase rho descends below a dedicated right bowl. Tau keeps a
    // separate top cap and a connected falling stem with its own exit.
    // Neither shape is substituted from an authored Latin p or t.
    if (glyph === "ρ") return [
        curve([-0.55, -0.65], [0.75, -0.85], [0.75, 0]),
        curve([0.75, 0], [0.75, 0.85], [-0.55, 0.65]),
        curve([-0.55, 0.65], [-0.8, 0.35], [-0.55, 0]),
        curve([-0.55, 0], [-0.8, -0.35], [-0.55, -0.65]),
        line(-0.55, 1.05, -0.55, -0.65),
    ];
    if (glyph === "τ") return [
        line(-0.75, -0.55, 0.75, -0.55),
        line(0.15, -0.55, 0.15, 0.65),
        line(0.15, 0.65, 0.65, 0.75),
    ];
    // Epsilon's left-facing open loop lifts for its separate middle bar.
    // Omega draws two joined bowls and a short lifted baseline foot;
    // neither silently borrows Latin e or w handwriting.
    if (glyph === "ε") return [
        curve([0.75, -0.7], [-0.75, -1], [-0.65, 0]),
        curve([-0.65, 0], [-0.65, 0.85], [0.75, 0.7]),
        line(-0.55, 0, 0.45, 0),
    ];
    if (glyph === "ω") return [
        curve([-0.85, -0.65], [-0.85, 0.75], [-0.45, 0.75]),
        curve([-0.45, 0.75], [-0.05, 0.75], [0, -0.15]),
        curve([0, -0.15], [0.05, 0.75], [0.45, 0.75]),
        curve([0.45, 0.75], [0.85, 0.75], [0.85, -0.65]),
        line(-0.75, 0.85, -0.35, 0.85),
    ];
    // Kappa lifts from an upright to a separate, joined curved arm.
    // Chi draws two independent crossing curved paths; neither borrows
    // the Latin k/x outlines or a Unicode lookalike.
    if (glyph === "κ") return [
        line(-0.65, -0.85, -0.65, 0.85),
        curve([0.7, -0.75], [0.1, -0.4], [-0.45, 0.05]),
        curve([-0.45, 0.05], [0.35, 0.4], [0.75, 0.85]),
    ];
    if (glyph === "χ") return [
        curve([-0.65, -0.8], [-0.2, 0.02], [0.75, 0.8]),
        curve([0.65, -0.85], [0.15, -0.1], [-0.75, 0.85]),
    ];
    // Phi uses a sampled, closed bowl and a separately lifted long stem.
    // Psi has a joined pair of shoulders before its own upright stem;
    // neither reuses another Greek or Latin glyph's pen geometry.
    if (glyph === "φ") return [
        curve([0, -0.75], [0.65, -0.75], [0.65, 0]),
        curve([0.65, 0], [0.65, 0.75], [0, 0.75]),
        curve([0, 0.75], [-0.65, 0.75], [-0.65, 0]),
        curve([-0.65, 0], [-0.65, -0.75], [0, -0.75]),
        line(0, -1.05, 0, 1.05),
    ];
    if (glyph === "ψ") return [
        curve([-0.75, -0.7], [-0.75, 0.65], [0, 0.65]),
        curve([0, 0.65], [0.75, 0.65], [0.75, -0.7]),
        line(0, -0.85, 0, 1.05),
    ];
    // Zeta's three connected bowls turn into a descending exit; eta
    // lifts between an independent left upright and a right-descending
    // double-curve shoulder. Neither borrows Latin z/n pen geometry.
    if (glyph === "ζ") return [
        curve([0.7, -0.85], [-0.55, -1.05], [-0.55, -0.4]),
        curve([-0.55, -0.4], [-0.5, 0.2], [0.45, 0.3]),
        curve([0.45, 0.3], [0.95, 0.65], [-0.15, 1.05]),
    ];
    if (glyph === "η") return [
        line(-0.65, -0.7, -0.65, 0.65),
        curve([-0.65, -0.55], [0, -0.9], [0.6, -0.4]),
        curve([0.6, -0.4], [0.7, 0.2], [0.6, 1.05]),
    ];
    // Xi joins four successive curved turns into a descending exit.
    // Iota has one uninterrupted short rising hook without Latin i's
    // dot; neither glyph takes another authored Unicode key as ink.
    if (glyph === "ξ") return [
        curve([0.6, -0.85], [-0.1, -1], [-0.6, -0.55]),
        curve([-0.6, -0.55], [-0.85, -0.05], [0.45, 0.1]),
        curve([0.45, 0.1], [0.85, 0.4], [-0.4, 0.65]),
        curve([-0.4, 0.65], [-0.55, 0.9], [0.35, 1.05]),
    ];
    if (glyph === "ι") return [
        curve([-0.2, -0.7], [-0.4, 0.3], [-0.1, 0.55]),
        curve([-0.1, 0.55], [0.1, 0.85], [0.45, 0.55]),
    ];
    // Omicron has an asymmetric, exact Unicode oval, not a copy of the
    // circular Latin o. Upsilon joins two open shoulders at its bowl;
    // both keep their own uninterrupted directed native pen paths.
    if (glyph === "ο") return [
        curve([0, -0.75], [0.68, -0.85], [0.7, 0.02]),
        curve([0.7, 0.02], [0.65, 0.75], [0.05, 0.75]),
        curve([0.05, 0.75], [-0.75, 0.8], [-0.7, 0.05]),
        curve([-0.7, 0.05], [-0.7, -0.75], [0, -0.75]),
    ];
    if (glyph === "υ") return [
        curve([-0.7, -0.7], [-0.65, 0.7], [0, 0.72]),
        curve([0, 0.72], [0.65, 0.7], [0.65, -0.7]),
    ];
    // Lowercase nu descends and rises in one rounded V-shaped motion.
    // Final sigma ς keeps a descending terminal hook distinct from
    // the closed medial σ, from Latin s and from Unicode theta variants.
    if (glyph === "ν") return [
        curve([-0.7, -0.65], [-0.35, 0.1], [-0.12, 0.65]),
        curve([-0.12, 0.65], [0.3, 0.25], [0.65, -0.65]),
    ];
    if (glyph === "ς") return [
        curve([0.65, -0.65], [-0.45, -1.05], [-0.65, -0.18]),
        curve([-0.65, -0.18], [-0.85, 0.52], [0.1, 0.5]),
        curve([0.1, 0.5], [0.9, 0.55], [0.25, 1.05]),
    ];
    // Uppercase Gamma is one continuous top-corner motion. Omega draws
    // an open upper bowl between separately lifted baseline feet;
    // neither aliases Latin L or lowercase omega's double bowl.
    if (glyph === "Γ") return [
        line(-0.7, 0.85, -0.7, -0.8),
        line(-0.7, -0.8, 0.7, -0.8),
    ];
    if (glyph === "Ω") return [
        line(-0.75, 0.8, -0.35, 0.8),
        curve([-0.45, 0.55], [-0.9, -0.1], [-0.55, -0.65]),
        curve([-0.55, -0.65], [-0.2, -1.05], [0, -0.85]),
        curve([0, -0.85], [0.2, -1.05], [0.55, -0.65]),
        curve([0.55, -0.65], [0.9, -0.1], [0.45, 0.55]),
        line(0.35, 0.8, 0.75, 0.8),
    ];
    // Capital Pi traces both uprights and the connecting cap without a
    // lift. Capital Sigma has one angular zigzag between its top/bottom
    // bars. Each is distinct from Gamma and the summation operator ∑.
    if (glyph === "Π") return [
        line(-0.7, 0.85, -0.7, -0.85),
        line(-0.7, -0.85, 0.7, -0.85),
        line(0.7, -0.85, 0.7, 0.85),
    ];
    if (glyph === "Σ") return [
        line(0.75, -0.85, -0.65, -0.85),
        line(-0.65, -0.85, 0.25, 0),
        line(0.25, 0, -0.65, 0.85),
        line(-0.65, 0.85, 0.75, 0.85),
    ];
    // Capital Lambda joins the ascending and descending peak without a
    // pen lift. Capital Theta retains a rounded oval and lifted bar,
    // separate from the smaller lowercase theta geometry.
    if (glyph === "Λ") return [
        line(-0.7, 0.85, 0, -0.85),
        line(0, -0.85, 0.7, 0.85),
    ];
    if (glyph === "Θ") return [
        curve([0, -0.85], [0.85, -0.9], [0.78, 0]),
        curve([0.78, 0], [0.85, 0.9], [0, 0.85]),
        curve([0, 0.85], [-0.85, 0.9], [-0.78, 0]),
        curve([-0.78, 0], [-0.85, -0.9], [0, -0.85]),
        line(-0.6, 0, 0.6, 0),
    ];
    // Capital Xi lifts between three separate horizontal pen bars. The
    // center bar is shorter than its cap and foot, so this exact shape
    // is not borrowed from the identity operator. Capital Phi closes an
    // oval before a lifted vertical through its center.
    if (glyph === "Ξ") return [
        line(-0.7, -0.75, 0.7, -0.75),
        line(-0.45, 0, 0.45, 0),
        line(-0.7, 0.75, 0.7, 0.75),
    ];
    if (glyph === "Φ") return [
        curve([0, -0.72], [0.8, -0.8], [0.75, 0]),
        curve([0.75, 0], [0.8, 0.8], [0, 0.72]),
        curve([0, 0.72], [-0.8, 0.8], [-0.75, 0]),
        curve([-0.75, 0], [-0.8, -0.8], [0, -0.72]),
        line(0, -0.95, 0, 0.95),
    ];
    // Capital Psi has one connected upper shoulder bowl with a lifted
    // central descending stem. Capital Chi crosses two separate straight
    // diagonals; neither reuses its lowercase curved letter's ink.
    if (glyph === "Ψ") return [
        curve([-0.7, -0.8], [-0.7, 0.55], [0, 0.55]),
        curve([0, 0.55], [0.7, 0.55], [0.7, -0.8]),
        line(0, 0.45, 0, 0.95),
    ];
    if (glyph === "Χ") return [
        line(-0.7, -0.85, 0.7, 0.85),
        line(0.7, -0.85, -0.7, 0.85),
    ];
    // Capital Beta keeps a separately lifted upright and four joined
    // quadratic bowl sweeps. Capital Upsilon lifts between its two arms,
    // then continues into one central stem; neither borrows Latin B/Y.
    if (glyph === "Β") return [
        line(-0.65, -0.85, -0.65, 0.85),
        curve([-0.65, -0.85], [0.9, -0.9], [0.65, -0.06]),
        curve([0.65, -0.06], [0.5, 0.12], [-0.65, 0.05]),
        curve([-0.65, 0.05], [0.95, 0.12], [0.7, 0.47]),
        curve([0.7, 0.47], [0.7, 0.9], [-0.65, 0.85]),
    ];
    if (glyph === "Υ") return [
        line(-0.7, -0.85, 0, 0),
        line(0.7, -0.85, 0, 0),
        line(0, 0, 0, 0.9),
    ];
    // Capital Alpha joins its two peaked strokes before lifting for the
    // crossbar; Epsilon joins three outer sides then lifts for its middle
    // bar. These are exact Greek keys with their own geometry, not Latin A/E.
    if (glyph === "Α") return [
        line(-0.7, 0.85, 0, -0.85),
        line(0, -0.85, 0.7, 0.85),
        line(-0.4, 0.23, 0.4, 0.23),
    ];
    if (glyph === "Ε") return [
        line(0.65, -0.85, -0.7, -0.85),
        line(-0.7, -0.85, -0.7, 0.85),
        line(-0.7, 0.85, 0.65, 0.85),
        line(-0.7, 0, 0.48, 0),
    ];
    // Capital Iota is one undotted upright with no borrowed Latin caps.
    // Capital Omicron samples four joined, asymmetric oval quadratics;
    // its native ink is distinct from Latin O and lowercase Greek omicron.
    if (glyph === "Ι") return [
        line(0, -0.85, 0, 0.85),
    ];
    if (glyph === "Ο") return [
        curve([0, -0.8], [0.75, -0.9], [0.76, 0]),
        curve([0.76, 0], [0.75, 0.8], [0.05, 0.83]),
        curve([0.05, 0.83], [-0.75, 0.85], [-0.72, 0]),
        curve([-0.72, 0], [-0.7, -0.88], [0, -0.8]),
    ];
    // Capital Zeta's top, diagonal and foot are one continuous angular
    // pen route. Capital Eta lifts between two uprights and its centered
    // crossbar, retaining a distinct exact Unicode key from Latin H.
    if (glyph === "Ζ") return [
        line(-0.7, -0.85, 0.7, -0.85),
        line(0.7, -0.85, -0.7, 0.85),
        line(-0.7, 0.85, 0.7, 0.85),
    ];
    if (glyph === "Η") return [
        line(-0.7, -0.85, -0.7, 0.85),
        line(0.7, -0.85, 0.7, 0.85),
        line(-0.7, 0, 0.7, 0),
    ];
    // Greek capital Kappa lifts from its upright to two connected arms.
    // Mu and Nu form uninterrupted angular pen routes; Greek identities
    // remain distinct from their Latin lookalikes and lowercase letters.
    if (glyph === "Κ") return [
        line(-0.7, -0.85, -0.7, 0.85),
        line(0.7, -0.85, -0.58, 0),
        line(-0.58, 0, 0.7, 0.85),
    ];
    if (glyph === "Μ") return [
        line(-0.7, 0.85, -0.7, -0.85),
        line(-0.7, -0.85, 0, 0.28),
        line(0, 0.28, 0.7, -0.85),
        line(0.7, -0.85, 0.7, 0.85),
    ];
    if (glyph === "Ν") return [
        line(-0.7, 0.85, -0.7, -0.85),
        line(-0.7, -0.85, 0.7, 0.85),
        line(0.7, 0.85, 0.7, -0.85),
    ];
    // Capital Rho has a straight lifted upright and two connected bowl
    // quadratics. Capital Tau separates its top cap from its central stem.
    if (glyph === "Ρ") return [
        line(-0.65, -0.85, -0.65, 0.85),
        curve([-0.65, -0.85], [0.85, -0.95], [0.7, -0.15]),
        curve([0.7, -0.15], [0.75, 0.5], [-0.65, 0.15]),
    ];
    if (glyph === "Τ") return [
        line(-0.75, -0.85, 0.75, -0.85),
        line(0, -0.85, 0, 0.85),
    ];
    // Product and coproduct are distinct n-ary operators, not uppercase
    // Greek Pi. Both lift at the cap, uprights and their two serif feet;
    // their cap positions are vertically inverted from one another.
    if (glyph === "∏") return [
        line(-0.85, -0.85, 0.85, -0.85),
        line(-0.6, -0.85, -0.6, 0.85),
        line(0.6, -0.85, 0.6, 0.85),
        line(-0.85, 0.85, -0.35, 0.85),
        line(0.35, 0.85, 0.85, 0.85),
    ];
    if (glyph === "∐") return [
        line(-0.85, 0.85, 0.85, 0.85),
        line(-0.6, 0.85, -0.6, -0.85),
        line(0.6, 0.85, 0.6, -0.85),
        line(-0.85, -0.85, -0.35, -0.85),
        line(0.35, -0.85, 0.85, -0.85),
    ];
    // Similarity and congruence each have one continuously authored
    // two-quadratic tilde; congruence lifts for two independent bars.
    // A saved '=' or approximate sign never supplies either shape.
    if (glyph === "∼") return [
        curve([-0.8, 0.2], [-0.4, -0.5], [0, 0]),
        curve([0, 0], [0.4, 0.5], [0.8, -0.2]),
    ];
    if (glyph === "≅") return [
        curve([-0.8, -0.45], [-0.4, -1.05], [0, -0.65]),
        curve([0, -0.65], [0.4, -0.15], [0.8, -0.85]),
        horizontal(0.1), horizontal(0.8),
    ];
    // The empty set uses four connected quadratic ellipse guides, each
    // sampled into sixteen native chords, and a separately lifted slash.
    // It is not an automatic replacement for Latin Ø or digit zero.
    if (glyph === "∅") return [
        curve([0, -0.8], [0.65, -0.8], [0.65, 0]),
        curve([0.65, 0], [0.65, 0.8], [0, 0.8]),
        curve([0, 0.8], [-0.65, 0.8], [-0.65, 0]),
        curve([-0.65, 0], [-0.65, -0.8], [0, -0.8]),
        line(-0.8, 0.9, 0.8, -0.9),
    ];
    if (glyph === "∫") return [
        curve([0.75, -0.82], [0.4, -1], [0.12, -0.52]),
        curve([0.12, -0.52], [-0.12, -0.1], [-0.2, 0.5]),
        curve([-0.2, 0.5], [-0.4, 1], [-0.78, 0.78]),
    ];
    // Calculus variants have exact keys rather than substitutions for ∫.
    // The contour integral keeps its original uninterrupted S-shaped path
    // and then lifts to a closed, four-quadratic loop across the midpoint.
    if (glyph === "∮") return [
        ...mathParts(cx, cy, radius, "∫"),
        curve([0, -0.36], [0.6, -0.36], [0.6, 0]),
        curve([0.6, 0], [0.6, 0.36], [0, 0.36]),
        curve([0, 0.36], [-0.6, 0.36], [-0.6, 0]),
        curve([-0.6, 0], [-0.6, -0.36], [0, -0.36]),
    ];
    if (glyph === "∬" || glyph === "∭") {
        // Shrink only horizontal geometry; independently translated
        // integral paths remain continuous and separately editable.
        const triple = glyph === "∭";
        const scale = triple ? 0.46 : 0.62;
        const offsets = triple ? [-0.47, 0, 0.47] : [-0.35, 0.35];
        const transform = (p: Point, offset: number): Point => ({
            x: cx + Math.round((p.x - cx) * scale + offset * radius),
            y: p.y,
        });
        const integral = mathParts(cx, cy, radius, "∫");
        return offsets.flatMap((offset) => integral.map((part) => ({
            from: transform(part.from, offset),
            to: transform(part.to, offset),
            ...(part.control ? {
                control: transform(part.control, offset),
            } : {}),
        })));
    }
    // Blackboard number sets are opt-in exact Unicode outlines, not
    // Latin letter substitutions. Second rails have deliberate pen lifts.
    if (glyph === "ℕ") return [
        line(-0.8, 0.8, -0.8, -0.8),
        line(-0.8, -0.8, 0.8, 0.8),
        line(0.8, 0.8, 0.8, -0.8),
        line(-0.56, 0.6, -0.56, -0.48),
    ];
    if (glyph === "ℤ") return [
        line(-0.75, -0.8, 0.75, -0.8),
        line(0.75, -0.8, -0.75, 0.8),
        line(-0.75, 0.8, 0.75, 0.8),
        line(0.56, -0.8, -0.56, 0.8),
    ];
    if (glyph === "ℚ") return [
        curve([0, -0.85], [0.7, -0.85], [0.7, 0]),
        curve([0.7, 0], [0.7, 0.85], [0, 0.85]),
        curve([0, 0.85], [-0.7, 0.85], [-0.7, 0]),
        curve([-0.7, 0], [-0.7, -0.85], [0, -0.85]),
        line(0.15, 0.4, 0.8, 0.95),
        line(-0.42, -0.4, -0.42, 0.4),
    ];
    if (glyph === "ℝ") return [
        line(-0.7, 0.85, -0.7, -0.85),
        curve([-0.7, -0.85], [0.7, -1], [0.65, -0.35]),
        curve([0.65, -0.35], [0.65, 0.15], [-0.7, 0.15]),
        line(-0.7, 0.15, 0.75, 0.85),
        line(-0.48, -0.6, -0.48, 0.05),
    ];
    if (glyph === "ℂ") return [
        curve([0.68, -0.65], [0.1, -1], [-0.56, -0.6]),
        curve([-0.56, -0.6], [-0.9, -0.3], [-0.8, 0]),
        curve([-0.8, 0], [-0.9, 0.3], [-0.56, 0.6]),
        curve([-0.56, 0.6], [0.1, 1], [0.68, 0.65]),
        line(-0.51, -0.48, -0.51, 0.48),
    ];
    // Indexed radicals leave the 93-key math batch unchanged. The root
    // gesture is joined; independently lifted index strokes distinguish
    // cube and fourth roots from each other and from the simple root.
    if (glyph === "∛" || glyph === "∜") {
        const root: MarkPiece[] = [
            line(-0.42, 0.1, -0.15, 0),
            line(-0.15, 0, 0.17, 0.75),
            line(0.17, 0.75, 0.5, -0.77),
            line(0.5, -0.77, 0.82, -0.77),
        ];
        if (glyph === "∛") return [
            ...root,
            curve([-0.82, -0.85], [-0.18, -1], [-0.49, -0.65]),
            curve([-0.49, -0.65], [-0.1, -0.2], [-0.82, -0.3]),
        ];
        return [
            ...root,
            line(-0.72, -0.88, -0.82, -0.48),
            line(-0.82, -0.48, -0.33, -0.48),
            line(-0.4, -0.93, -0.4, -0.27),
        ];
    }
    // Manual circled arithmetic keeps a single joined four-quadratic
    // outer ring and separately lifted arithmetic or short dot ink.
    // These are their own Unicode keys, not +, × or Latin O aliases.
    if (["⊕", "⊗", "⊙", "⊖", "⊘"].includes(glyph)) {
        const ring: MarkPiece[] = [
            curve([0, -0.82], [0.82, -0.82], [0.82, 0]),
            curve([0.82, 0], [0.82, 0.82], [0, 0.82]),
            curve([0, 0.82], [-0.82, 0.82], [-0.82, 0]),
            curve([-0.82, 0], [-0.82, -0.82], [0, -0.82]),
        ];
        if (glyph === "⊕") return [
            ...ring, line(-0.43, 0, 0.43, 0),
            line(0, -0.43, 0, 0.43),
        ];
        if (glyph === "⊗") return [
            ...ring, line(-0.32, -0.32, 0.32, 0.32),
            line(-0.32, 0.32, 0.32, -0.32),
        ];
        if (glyph === "⊖") return [
            ...ring, line(-0.43, 0, 0.43, 0),
        ];
        if (glyph === "⊘") return [
            ...ring, line(-0.4, 0.4, 0.4, -0.4),
        ];
        return [...ring, line(0, -0.08, 0, 0.08)];
    }
    if (glyph === "∂") return [
        // A single directed pen path: high tail, round bowl and return
        // to the shoulder. The native v1 library samples four curves.
        curve([0.6, -0.85], [-0.45, -0.98], [-0.35, -0.2]),
        curve([-0.35, -0.2], [-0.95, 0], [-0.7, 0.6]),
        curve([-0.7, 0.6], [-0.15, 1.05], [0.45, 0.55]),
        curve([0.45, 0.55], [1, -0.15], [-0.35, -0.2]),
    ];
    // Proportionality is a single open, directed curve: incoming upper
    // arm, rounded left loop and outgoing lower arm. It is not infinity.
    if (glyph === "∝") return [
        curve([0.75, -0.65], [0.05, -0.6], [-0.12, 0]),
        curve([-0.12, 0], [-0.62, 0.9], [-0.85, 0]),
        curve([-0.85, 0], [-0.62, -0.9], [-0.12, 0]),
        curve([-0.12, 0], [0.05, 0.6], [0.75, 0.65]),
    ];
    if (glyph === "≈") return [-0.45, 0.45].flatMap((row) => [
        curve([-0.8, row + 0.2], [-0.35, row - 0.35], [0, row]),
        curve([0, row], [0.35, row + 0.35], [0.8, row - 0.2]),
    ]);
    if (glyph === "∞") {
        return [
            curve([0, 0], [-0.55, -0.85], [-0.8, 0]),
            curve([-0.8, 0], [-0.55, 0.85], [0, 0]),
            curve([0, 0], [0.55, -0.85], [0.8, 0]),
            curve([0.8, 0], [0.55, 0.85], [0, 0]),
        ];
    }
    return [horizontal(0),
        { from: point(0, -0.7), to: point(0, -0.58) },
        { from: point(0, 0.58), to: point(0, 0.7) },
    ];
}

// The dollar reuses the same four smooth S quadratics with a separate
// vertical pen stroke. The euro reuses the analytic open circle used for c,
// plus two independent crossbars. Neither is a sampled font or fallback.
function currencyParts(
    cx: number, cy: number, radius: number, glyph: CurrencyMark,
): MarkPiece[] {
    if (glyph === "$") {
        return [
            ...smoothS(cx, cy, radius).map(([from, control, to]) =>
                ({ from, control, to })),
            { from: { x: cx, y: cy - radius },
                to: { x: cx, y: cy + radius } },
        ];
    }
    return [-0.28, 0.28].map((height) => ({
        from: { x: cx - Math.round(radius * 0.83),
            y: cy + Math.round(radius * height) },
        to: { x: cx + Math.round(radius * 0.23),
            y: cy + Math.round(radius * height) },
    }));
}

// Digits are deliberately distinct from lowercase letters. The zero and
// paired bowls of eight use exact SVG ellipses; only their sampled chords
// enter the Rust v1 line-only library. Other forms use analytic quadratics
// and straight pen strokes with independent pen lifts.
type DigitGlyph = "0" | "1" | "2" | "3" | "4" | "5"
    | "6" | "7" | "8" | "9";
type Ellipse = { cx: number; cy: number; rx: number; ry: number };

function digitGeometry(
    cx: number, cy: number, radius: number, glyph: DigitGlyph,
): { ellipses: Ellipse[]; parts: MarkPiece[] } {
    const p = (x: number, y: number): Point => ({
        x: cx + Math.round(radius * x),
        y: cy + Math.round(radius * y),
    });
    const line = (x1: number, y1: number, x2: number, y2: number)
        : MarkPiece => ({ from: p(x1, y1), to: p(x2, y2) });
    const arc = (x1: number, y1: number, hx: number, hy: number,
        x2: number, y2: number): MarkPiece => ({
        from: p(x1, y1), control: p(hx, hy), to: p(x2, y2),
    });
    const ellipse = (y: number, rx: number, ry: number): Ellipse => ({
        cx, cy: cy + Math.round(y * radius),
        rx: Math.round(rx * radius), ry: Math.round(ry * radius),
    });
    switch (glyph) {
        case "0": return { ellipses: [ellipse(0, 0.7, 1)], parts: [] };
        case "1": return { ellipses: [], parts: [
            line(-0.4, -0.6, 0, -1), line(0, -1, 0, 1),
            line(-0.48, 1, 0.48, 1),
        ] };
        case "2": return { ellipses: [], parts: [
            arc(-0.7, -0.55, -0.7, -1, 0, -1),
            arc(0, -1, 0.9, -1, 0.7, -0.13),
            line(0.7, -0.13, -0.7, 1), line(-0.7, 1, 0.7, 1),
        ] };
        case "3": return { ellipses: [], parts: [
            arc(-0.6, -0.85, 0.9, -1, 0.7, -0.38),
            arc(0.7, -0.38, 0.7, 0, 0, 0),
            arc(0, 0, 0.85, 0.02, 0.7, 0.42),
            arc(0.7, 0.42, 0.85, 1, -0.6, 0.85),
        ] };
        case "4": return { ellipses: [], parts: [
            line(0.4, -1, -0.75, 0.22),
            line(-0.75, 0.22, 0.75, 0.22), line(0.4, -1, 0.4, 1),
        ] };
        case "5": return { ellipses: [], parts: [
            line(0.7, -1, -0.7, -1), line(-0.7, -1, -0.7, -0.12),
            arc(-0.7, -0.12, 0.8, -0.3, 0.7, 0.4),
            arc(0.7, 0.4, 0.7, 1, -0.7, 0.86),
        ] };
        case "6": return {
            ellipses: [ellipse(0.37, 0.7, 0.62)],
            parts: [arc(0.5, -1, -0.7, -1, -0.7, 0.37)],
        };
        case "7": return { ellipses: [], parts: [
            line(-0.7, -1, 0.7, -1), line(0.7, -1, -0.25, 1),
        ] };
        case "8": return {
            ellipses: [ellipse(-0.5, 0.57, 0.5),
                ellipse(0.5, 0.7, 0.5)], parts: [],
        };
        case "9": return {
            ellipses: [ellipse(-0.37, 0.7, 0.62)],
            parts: [arc(0.7, -0.37, 0.7, 0.8, -0.4, 1)],
        };
    }
}

// Degree and percent have exact ideal SVG circles, sampled only when
// exporting to the existing integer-micrometre line-only native library.
function circleMarkGeometry(
    cx: number, cy: number, radius: number, glyph: CircleMark,
): ReturnType<typeof digitGeometry> {
    const ring = (x: number, y: number): Ellipse => ({
        cx: cx + Math.round(radius * x),
        cy: cy + Math.round(radius * y),
        rx: Math.round(radius * 0.28),
        ry: Math.round(radius * 0.28),
    });
    if (glyph === "°") {
        return { ellipses: [ring(0.4, -0.63)], parts: [] };
    }
    return {
        ellipses: [ring(-0.48, -0.56), ring(0.48, 0.56)],
        parts: [{
            from: { x: cx + Math.round(radius * 0.75),
                y: cy - Math.round(radius * 0.90) },
            to: { x: cx - Math.round(radius * 0.75),
                y: cy + Math.round(radius * 0.90) },
        }],
    };
}

function ellipsePoint(ellipse: Ellipse, step: number): Point {
    const radians = 2 * Math.PI * step / CIRCLE_SEGMENTS;
    return {
        x: ellipse.cx + Math.round(ellipse.rx * Math.cos(radians)),
        y: ellipse.cy + Math.round(ellipse.ry * Math.sin(radians)),
    };
}

function digitGuide(geometry: ReturnType<typeof digitGeometry>): string {
    const loops = geometry.ellipses.map((ellipse) => {
        const { cx, cy, rx, ry } = ellipse;
        return `M ${cx + rx} ${cy} A ${rx} ${ry} 0 1 1 ${cx - rx} ${cy}`
            + ` A ${rx} ${ry} 0 1 1 ${cx + rx} ${cy}`;
    });
    return [...loops, punctuationGuide(geometry.parts)].filter(Boolean)
        .join(" ");
}

// Print capitals use normalized analytic geometry independent of lowercase
// authoring. O/Q share the exact o circle, C/G use its open circular arc,
// S shares the smooth quadratics, and the remaining curves are quadratic
// SVG guides. Only sampled integer chords survive in Rust v1 TSV.
type CapitalGlyph = "A" | "B" | "C" | "D" | "E" | "F" | "G"
    | "H" | "I" | "J" | "K" | "L" | "M" | "N" | "O" | "P"
    | "Q" | "R" | "S" | "T" | "U" | "V" | "W" | "X"
    | "Y" | "Z";

function capitalParts(
    cx: number, cy: number, radius: number, glyph: CapitalGlyph,
): MarkPiece[] {
    const point = (x: number, y: number): Point => ({
        x: cx + Math.round(radius * x),
        y: cy + Math.round(radius * y),
    });
    const line = (x1: number, y1: number, x2: number, y2: number)
        : MarkPiece => ({ from: point(x1, y1), to: point(x2, y2) });
    const curve = (x1: number, y1: number, hx: number, hy: number,
        x2: number, y2: number): MarkPiece => ({
        from: point(x1, y1), control: point(hx, hy),
        to: point(x2, y2),
    });
    const stem = () => line(-0.75, -1, -0.75, 1);
    const bowl = (): MarkPiece[] => [
        curve(-0.75, -1, 0.75, -1, 0.75, -0.5),
        curve(0.75, -0.5, 0.75, 0, -0.75, 0),
    ];
    switch (glyph) {
        case "A": return [line(-0.75, 1, 0, -1),
            line(0, -1, 0.75, 1), line(-0.39, 0.05, 0.39, 0.05)];
        case "B": return [stem(), ...bowl(),
            curve(-0.75, 0, 0.75, 0, 0.75, 0.5),
            curve(0.75, 0.5, 0.75, 1, -0.75, 1)];
        case "C": return [];
        case "D": return [stem(),
            curve(-0.75, -1, 0.8, -1, 0.8, 0),
            curve(0.8, 0, 0.8, 1, -0.75, 1)];
        case "E": return [stem(), line(-0.75, -1, 0.75, -1),
            line(-0.75, 0, 0.45, 0), line(-0.75, 1, 0.75, 1)];
        case "F": return [stem(), line(-0.75, -1, 0.75, -1),
            line(-0.75, 0, 0.45, 0)];
        case "G": return [line(0.1, 0.35, 0.75, 0.35),
            line(0.75, 0.35, 0.75, 0.65)];
        case "H": return [stem(), line(0.75, -1, 0.75, 1),
            line(-0.75, 0, 0.75, 0)];
        case "I": return [line(-0.55, -1, 0.55, -1),
            line(0, -1, 0, 1), line(-0.55, 1, 0.55, 1)];
        case "J": return [line(-0.2, -1, 0.7, -1),
            line(0.7, -1, 0.7, 0.5),
            curve(0.7, 0.5, 0.4, 1, -0.25, 1),
            curve(-0.25, 1, -0.75, 1, -0.75, 0.55)];
        case "K": return [stem(), line(0.75, -1, -0.75, 0),
            line(-0.35, -0.27, 0.75, 1)];
        case "L": return [stem(), line(-0.75, 1, 0.75, 1)];
        case "M": return [stem(), line(-0.75, -1, 0, 0.2),
            line(0, 0.2, 0.75, -1), line(0.75, -1, 0.75, 1)];
        case "N": return [stem(), line(-0.75, -1, 0.75, 1),
            line(0.75, 1, 0.75, -1)];
        case "O": return [];
        case "P": return [stem(), ...bowl()];
        case "Q": return [line(0.3, 0.4, 1, 1)];
        case "R": return [stem(), ...bowl(),
            line(-0.2, 0, 0.75, 1)];
        case "S": return [];
        case "T": return [line(-0.8, -1, 0.8, -1), line(0, -1, 0, 1)];
        case "U": return [line(-0.75, -1, -0.75, 0.35),
            curve(-0.75, 0.35, -0.75, 1, 0, 1),
            curve(0, 1, 0.75, 1, 0.75, 0.35),
            line(0.75, 0.35, 0.75, -1)];
        case "V": return [line(-0.75, -1, 0, 1),
            line(0, 1, 0.75, -1)];
        case "W": return [line(-0.85, -1, -0.45, 1),
            line(-0.45, 1, 0, 0), line(0, 0, 0.45, 1),
            line(0.45, 1, 0.85, -1)];
        case "X": return [line(-0.75, -1, 0.75, 1),
            line(0.75, -1, -0.75, 1)];
        case "Y": return [line(-0.75, -1, 0, 0),
            line(0.75, -1, 0, 0), line(0, 0, 0, 1)];
        case "Z": return [line(-0.75, -1, 0.75, -1),
            line(0.75, -1, -0.75, 1), line(-0.75, 1, 0.75, 1)];
    }
}

function capitalGuide(
    cx: number, cy: number, radius: number, glyph: CapitalGlyph,
): string {
    if (glyph === "S") {
        return smoothS(cx, cy, radius).map(([from, control, to], index) =>
            `${index === 0 ? `M ${from.x} ${from.y} ` : ""}`
            + `Q ${control.x} ${control.y} ${to.x} ${to.y}`).join(" ");
    }
    return punctuationGuide(capitalParts(cx, cy, radius, glyph));
}

// Complete the finite native Spanish/English text inventory without
// substituting typographic quotes for independent ASCII quote keys.
// These additional caller-selected notation shapes do not change the
// frozen 114-glyph language inventory or its eleven-mark fill action.
const NOTATION_MARKS = ["/", "\\", "{", "}"] as const;
type TextMark = ";" | ":" | "\"" | "'" | "…" | "–" | "—"
    | "(" | ")" | "[" | "]" | typeof NOTATION_MARKS[number];
const TEXT_MARKS = [";", ":", "\"", "'", "…", "–", "—",
    "(", ")", "[", "]"] as const;

function textMarkParts(
    cx: number, cy: number, r: number, glyph: TextMark,
): MarkPiece[] {
    const p = (x: number, y: number): Point => ({
        x: cx + Math.round(x * r), y: cy + Math.round(y * r),
    });
    const line = (x: number, y: number, xx: number, yy: number)
        : MarkPiece => ({ from: p(x, y), to: p(xx, yy) });
    const dot = (x: number, y: number) => line(x, y, x, y + 0.12);
    if (glyph === ":") return [dot(0, -0.6), dot(0, 0.6)];
    if (glyph === ";") return [dot(0, -0.6),
        line(0.13, 0.6, -0.18, 0.98)];
    if (glyph === "'") return [line(0.04, -0.95, -0.07, -0.55)];
    if (glyph === "\"") return [line(-0.35, -0.95, -0.46, -0.55),
        line(0.35, -0.95, 0.24, -0.55)];
    if (glyph === "…") return [-0.7, 0, 0.7].map((x) => dot(x, 0.78));
    if (glyph === "/") return [line(0.65, -0.95, -0.65, 0.95)];
    if (glyph === "\\") return [line(-0.65, -0.95, 0.65, 0.95)];
    if (glyph === "{" || glyph === "}") {
        const side = glyph === "{" ? 1 : -1;
        const point = (x: number, y: number) => p(side * x, y);
        const upper = point(-0.15, -0.62);
        const middle = point(-0.5, 0);
        const lower = point(-0.15, 0.62);
        return [
            { from: point(0.35, -0.95),
                control: point(-0.15, -0.95), to: upper },
            { from: upper, control: point(-0.1, -0.1), to: middle },
            { from: middle, control: point(-0.1, 0.1), to: lower },
            { from: lower, control: point(-0.15, 0.95),
                to: point(0.35, 0.95) },
        ];
    }
    if (glyph === "–" || glyph === "—") {
        const half = glyph === "—" ? 0.9 : 0.65;
        return [line(-half, 0, half, 0)];
    }
    if (glyph === "[") return [line(0.3, -0.9, -0.5, -0.9),
        line(-0.5, -0.9, -0.5, 0.9), line(-0.5, 0.9, 0.3, 0.9)];
    if (glyph === "]") return [line(-0.3, -0.9, 0.5, -0.9),
        line(0.5, -0.9, 0.5, 0.9), line(0.5, 0.9, -0.3, 0.9)];
    const flip = glyph === ")" ? -1 : 1;
    return [
        { from: p(flip * -0.2, -0.95),
            control: p(flip * -0.75, -0.5),
            to: p(flip * -0.75, 0) },
        { from: p(flip * -0.75, 0),
            control: p(flip * -0.75, 0.5),
            to: p(flip * -0.2, 0.95) },
    ];
}

function punctuationGuide(parts: MarkPiece[]): string {
    const commands: string[] = [];
    let pen: Point | null = null;
    for (const part of parts) {
        if (pen === null || pen.x !== part.from.x
            || pen.y !== part.from.y) {
            commands.push(`M ${part.from.x} ${part.from.y}`);
        }
        commands.push(part.control
            ? `Q ${part.control.x} ${part.control.y} ${part.to.x} ${part.to.y}`
            : `L ${part.to.x} ${part.to.y}`);
        pen = part.to;
    }
    return commands.join(" ");
}

function straightGuide(lines: StraightLine[]): string {
    const commands: string[] = [];
    let pen: Point | null = null;
    for (const { from, to } of lines) {
        if (pen === null || pen.x !== from.x || pen.y !== from.y) {
            commands.push(`M ${from.x} ${from.y}`);
        }
        commands.push(`L ${to.x} ${to.y}`);
        pen = to;
    }
    return commands.join(" ");
}

function tallStem(
    cx: number, cy: number, radius: number, g: "l" | "t" | "f",
) {
    return {
        from: { x: cx, y: g === "l" || g === "f"
            ? cy - Math.round(radius * 1.5) : cy - radius },
        to: { x: cx, y: cy + radius },
    };
}

function doubleHump(cx: number, cy: number, radius: number) {
    const left = cx - radius;
    const right = cx + radius;
    const high = cy - radius;
    const bottom = cy + radius;
    const firstPeak = cx - Math.round(radius / 2);
    const secondPeak = cx + Math.round(radius / 2);
    const arcs: [Point, Point, Point][] = [
        [{ x: left, y: cy }, { x: left, y: high },
            { x: firstPeak, y: high }],
        [{ x: firstPeak, y: high }, { x: cx, y: high },
            { x: cx, y: cy }],
        [{ x: cx, y: cy }, { x: cx, y: high },
            { x: secondPeak, y: high }],
        [{ x: secondPeak, y: high }, { x: right, y: high },
            { x: right, y: cy }],
    ];
    return {
        arcs,
        firstStem: {
            from: { x: left, y: bottom }, to: { x: left, y: high },
        },
        middleStem: {
            from: { x: cx, y: cy }, to: { x: cx, y: bottom },
        },
        lastStem: {
            from: { x: right, y: cy }, to: { x: right, y: bottom },
        },
    };
}

// Four tangent-continuous quadratic arcs give lowercase s a smooth
// upper turn and mirrored lower turn. The handles are reflected at each
// join, so changing radius cannot introduce an angular kink in SVG.
function smoothS(
    cx: number, cy: number, radius: number,
): [Point, Point, Point][] {
    const half = Math.round(radius * 0.7);
    const mid = Math.round(radius * 0.5);
    const upper = { x: cx - half, y: cy - mid };
    const topHandle = { x: cx, y: cy - radius };
    const leftHandle = {
        x: 2 * upper.x - topHandle.x,
        y: 2 * upper.y - topHandle.y,
    };
    const center = { x: cx, y: cy };
    const rightHandle = {
        x: 2 * center.x - leftHandle.x,
        y: 2 * center.y - leftHandle.y,
    };
    const lower = { x: cx + half, y: cy + mid };
    const bottomHandle = {
        x: 2 * lower.x - rightHandle.x,
        y: 2 * lower.y - rightHandle.y,
    };
    return [
        [{ x: cx + half, y: cy - Math.round(radius * 0.8) },
            topHandle, upper],
        [upper, leftHandle, center],
        [center, rightHandle, lower],
        [lower, bottomHandle,
            { x: cx - half, y: cy + Math.round(radius * 0.8) }],
    ];
}

function quadratic(
    from: Point, control: Point, to: Point, step: number,
): Point {
    const t = step / ARCH_SEGMENTS;
    const u = 1 - t;
    return {
        x: Math.round(u * u * from.x + 2 * u * t * control.x
            + t * t * to.x),
        y: Math.round(u * u * from.y + 2 * u * t * control.y
            + t * t * to.y),
    };
}

// Apply a user-selected shear to every existing pen stroke of one exact
// glyph. Interleaved NFC/NFD and unrelated glyphs are never modified.
// One rejected endpoint prevents the entire candidate from being returned.
export function italicizeGlyphStrokes(
    source: Stroke[], grapheme: string, angle: number, baseline: number,
): Stroke[] {
    if (!exactGrapheme(grapheme)) {
        throw new Error("Italic edit requires one exact visible glyph");
    }
    if (!Number.isSafeInteger(angle) || angle < 0 || angle > 20) {
        throw new Error("Italic angle must be 0..20 whole degrees");
    }
    const pivot = boundedInteger(String(baseline), 4500,
        "italic baseline");
    const slope = Math.tan(angle * Math.PI / 180);
    let found = false;
    const candidate = source.map((line) => {
        if (line.grapheme !== grapheme) return line;
        found = true;
        const stroke = validateStroke(line);
        const x1 = Math.round(stroke.x1 + slope * (pivot - stroke.y1));
        const x2 = Math.round(stroke.x2 + slope * (pivot - stroke.y2));
        if ([x1, x2].some((x) => x - stroke.width / 2 < 0
            || x + stroke.width / 2 > 3500)) {
            throw new Error("Italic ink and pen clip glyph cell");
        }
        return validateStroke({ ...stroke, x1, x2 });
    });
    if (!found) throw new Error("Selected glyph has no authored ink");
    encodeGlyphLibrary(candidate);
    return candidate;
}

// Resize one caller-authored glyph vertically without changing its pen,
// ordered paths, neighboring Unicode glyphs, or source library.
export function scaleGlyphHeight(
    source: Stroke[], grapheme: string, percent: number, baseline: number,
): Stroke[] {
    if (!exactGrapheme(grapheme)) {
        throw new Error("Height edit requires one exact visible glyph");
    }
    if (!Number.isSafeInteger(percent) || percent < 50 || percent > 150) {
        throw new Error("Height percent must be 50..150 whole percent");
    }
    const pivot = boundedInteger(String(baseline), 4500,
        "glyph height baseline");
    let found = false;
    const candidate = source.map((line) => {
        if (line.grapheme !== grapheme) return line;
        found = true;
        const stroke = validateStroke(line);
        if ([stroke.x1, stroke.x2].some((x) =>
            x - stroke.width / 2 < 0
            || x + stroke.width / 2 > 3500)) {
            throw new Error("Resized ink and pen clip glyph cell");
        }
        const y1 = Math.round(pivot + (stroke.y1 - pivot) * percent / 100);
        const y2 = Math.round(pivot + (stroke.y2 - pivot) * percent / 100);
        if ([y1, y2].some((y) => y - stroke.width / 2 < 0
            || y + stroke.width / 2 > 4500)) {
            throw new Error("Resized ink and pen clip glyph cell");
        }
        if (stroke.x1 === stroke.x2 && y1 === y2) {
            throw new Error("Height scaling collapses a pen segment");
        }
        return validateStroke({ ...stroke, y1, y2 });
    });
    if (!found) throw new Error("Selected glyph has no authored ink");
    encodeGlyphLibrary(candidate);
    return candidate;
}

// Change horizontal proportions of one exact authored glyph around an X
// anchor without changing sampled Y positions or pen characteristics.
export function scaleGlyphWidth(
    source: Stroke[], grapheme: string, percent: number, anchor: number,
): Stroke[] {
    if (!exactGrapheme(grapheme)) {
        throw new Error("Width edit requires one exact visible glyph");
    }
    if (!Number.isSafeInteger(percent) || percent < 50 || percent > 150) {
        throw new Error("Width percent must be 50..150 whole percent");
    }
    const pivot = boundedInteger(String(anchor), 3500,
        "glyph width anchor");
    let found = false;
    const candidate = source.map((line) => {
        if (line.grapheme !== grapheme) return line;
        found = true;
        const stroke = validateStroke(line);
        if ([stroke.y1, stroke.y2].some((y) =>
            y - stroke.width / 2 < 0
            || y + stroke.width / 2 > 4500)) {
            throw new Error("Resized ink and pen clip glyph cell");
        }
        const x1 = Math.round(pivot + (stroke.x1 - pivot) * percent / 100);
        const x2 = Math.round(pivot + (stroke.x2 - pivot) * percent / 100);
        if ([x1, x2].some((x) => x - stroke.width / 2 < 0
            || x + stroke.width / 2 > 3500)) {
            throw new Error("Resized ink and pen clip glyph cell");
        }
        if (stroke.y1 === stroke.y2 && x1 === x2) {
            throw new Error("Width scaling collapses a pen segment");
        }
        return validateStroke({ ...stroke, x1, x2 });
    });
    if (!found) throw new Error("Selected glyph has no authored ink");
    encodeGlyphLibrary(candidate);
    return candidate;
}

// Clockwise rotation in page coordinates (positive Y points down).
// Rounded directed endpoints and full pen-radius clearance match native CLI.
export function rotateGlyphStrokes(
    source: Stroke[], grapheme: string, degrees: number,
    pivotX: number, pivotY: number,
): Stroke[] {
    if (!exactGrapheme(grapheme)) {
        throw new Error("Glyph rotation requires one exact visible glyph");
    }
    if (!Number.isSafeInteger(degrees) || Math.abs(degrees) > 20
        || !Number.isSafeInteger(pivotX) || pivotX < 0 || pivotX > 3500
        || !Number.isSafeInteger(pivotY) || pivotY < 0 || pivotY > 4500) {
        throw new Error("Rotation requires -20..20° and bounded X/Y pivot");
    }
    const radians = degrees * Math.PI / 180;
    const sin = Math.sin(radians);
    const cos = Math.cos(radians);
    let found = false;
    const candidate = source.map((line) => {
        if (line.grapheme !== grapheme) return line;
        found = true;
        const stroke = validateStroke(line);
        if (degrees === 0) return stroke;
        const rotate = (x: number, y: number): Point => {
            const relativeX = x - pivotX;
            const relativeY = y - pivotY;
            const nextX = Math.round(pivotX + cos * relativeX
                - sin * relativeY);
            const nextY = Math.round(pivotY + sin * relativeX
                + cos * relativeY);
            if (nextX * 2 < stroke.width
                || nextX * 2 > 7000 - stroke.width
                || nextY * 2 < stroke.width
                || nextY * 2 > 9000 - stroke.width) {
                throw new Error("Rotated pen contour clips glyph cell");
            }
            return { x: nextX, y: nextY };
        };
        const start = rotate(stroke.x1, stroke.y1);
        const end = rotate(stroke.x2, stroke.y2);
        if (start.x === end.x && start.y === end.y) {
            throw new Error("Glyph rotation collapses a pen segment");
        }
        return validateStroke({ ...stroke, x1: start.x, y1: start.y,
            x2: end.x, y2: end.y });
    });
    if (!found) throw new Error("Selected glyph has no authored ink");
    encodeGlyphLibrary(candidate);
    return candidate;
}

// Move the entire caller-authored glyph without changing its geometry,
// pen order or neighboring exact Unicode records. Zero offset is identity.
export function translateGlyphStrokes(
    source: Stroke[], grapheme: string, dx: number, dy: number,
): Stroke[] {
    if (!exactGrapheme(grapheme)) {
        throw new Error("Glyph translation requires one exact visible glyph");
    }
    if (!Number.isSafeInteger(dx) || !Number.isSafeInteger(dy)
        || Math.abs(dx) > 3500 || Math.abs(dy) > 4500) {
        throw new Error("Glyph offsets require signed whole µm within cell");
    }
    let found = false;
    const candidate = source.map((line) => {
        if (line.grapheme !== grapheme) return line;
        found = true;
        const stroke = validateStroke(line);
        const moved = { ...stroke, x1: stroke.x1 + dx,
            y1: stroke.y1 + dy, x2: stroke.x2 + dx,
            y2: stroke.y2 + dy };
        if ([moved.x1, moved.x2].some((x) =>
            x * 2 < stroke.width || x * 2 > 7000 - stroke.width)
            || [moved.y1, moved.y2].some((y) =>
                y * 2 < stroke.width || y * 2 > 9000 - stroke.width)) {
            throw new Error("Translated pen contour clips glyph cell");
        }
        return validateStroke(moved);
    });
    if (!found) throw new Error("Selected glyph has no authored ink");
    encodeGlyphLibrary(candidate);
    return candidate;
}

// Scale pen thickness, not the glyph contour. Unlike seeded variation,
// this is an explicit reversible-by-reload edit of the unsaved TSV draft.
export function scaleGlyphInk(
    source: Stroke[], grapheme: string, percent: number,
): Stroke[] {
    if (!exactGrapheme(grapheme)) {
        throw new Error("Ink edit requires one exact visible glyph");
    }
    if (!Number.isSafeInteger(percent) || percent < 50 || percent > 150) {
        throw new Error("Ink percent must be 50..150 whole percent");
    }
    let found = false;
    const candidate = source.map((line) => {
        if (line.grapheme !== grapheme) return line;
        found = true;
        const stroke = validateStroke(line);
        const width = Math.round(stroke.width * percent / 100);
        if (width < 80 || width > 1500) {
            throw new Error("Scaled pen width outside 80..1500 um");
        }
        if ([stroke.x1, stroke.x2].some((x) => x * 2 < width
            || x * 2 > 7000 - width)
            || [stroke.y1, stroke.y2].some((y) => y * 2 < width
                || y * 2 > 9000 - width)) {
            throw new Error("Scaled ink and pen clip glyph cell");
        }
        return validateStroke({ ...stroke, width });
    });
    if (!found) throw new Error("Selected glyph has no authored ink");
    encodeGlyphLibrary(candidate);
    return candidate;
}

// Explicit connected-pen o/a draft. The first 48 circle chords are the
// unmodified ideal-o sample, entered from its leftmost point. A repeated
// lower half-circle carries the still-down pen to the right exit; an a
// continues through the authored stem before exiting. Only the optional
// acute is a separate pen lift. Adjacent letters are not auto-joined.
function cursiveLoop(base: Stroke[], radius: number): Stroke[] {
    const loop = base.slice(0, CIRCLE_SEGMENTS);
    const left = loop.at(CIRCLE_SEGMENTS / 2);
    const right = loop.at(0);
    if (left === undefined || right === undefined) {
        throw new Error("Cursive loop requires a complete authored circle");
    }
    const glyph = left.grapheme;
    const stem = glyph === "a" || glyph === "á"
        ? base.at(CIRCLE_SEGMENTS) : undefined;
    const accent = glyph === "ó" || glyph === "á"
        ? base.at(-1) : undefined;
    const lead = Math.min(200, Math.round(radius / 5));
    const pen = left.width;
    const draw = (from: Point, to: Point): Stroke => {
        for (const point of [from, to]) {
            if (point.x - pen / 2 < 0 || point.x + pen / 2 > 3500
                || point.y - pen / 2 < 0
                || point.y + pen / 2 > 4500) {
                throw new Error("Cursive connector and pen clip glyph cell");
            }
        }
        return validateStroke({ grapheme: glyph,
            x1: from.x, y1: from.y, x2: to.x, y2: to.y,
            width: pen, color: left.color });
    };
    const start = { x: left.x1, y: left.y1 };
    const rightPoint = { x: right.x1, y: right.y1 };
    const entry = draw({ x: start.x - lead, y: start.y + lead }, start);
    const fullCircle = [
        ...loop.slice(CIRCLE_SEGMENTS / 2),
        ...loop.slice(0, CIRCLE_SEGMENTS / 2),
    ];
    const carry = loop.slice(CIRCLE_SEGMENTS / 2);
    const ink = [entry, ...fullCircle, ...carry];
    let exitStart = rightPoint;
    if (stem !== undefined) {
        ink.push(draw(rightPoint, { x: stem.x1, y: stem.y1 }));
        ink.push(stem);
        exitStart = { x: stem.x2, y: stem.y2 };
    }
    ink.push(draw(exitStart, {
        x: exitStart.x + lead, y: exitStart.y - lead,
    }));
    if (accent !== undefined) ink.push(accent);
    return ink;
}

// An explicitly selected cursive c/e keeps one continuous pen gesture:
// a short entry, the open circular contour in reverse pen order, then an
// exit. Cursive e also crosses upward from the left interior. Printed
// c/e remain independent; no adjacent character is joined implicitly.
function cursiveOpenArc(base: Stroke[], radius: number): Stroke[] {
    const arc = base.slice(0, OPEN_ARC_SEGMENTS);
    const bar = base[OPEN_ARC_SEGMENTS];
    const upper = arc.at(-1);
    const lower = arc[0];
    if (upper === undefined || lower === undefined
        || (upper.grapheme === "e" && bar === undefined)) {
        throw new Error("Cursive c/e requires an authored arc source");
    }
    const glyph = upper.grapheme;
    const label = glyph === "e" ? "Cursive e" : "Cursive c";
    const pen = upper.width;
    const lead = Math.min(200, Math.round(radius / 5));
    const upperPoint = { x: upper.x2, y: upper.y2 };
    const bottom = { x: lower.x1, y: lower.y1 };
    const entry = bar === undefined ? upperPoint : { x: bar.x1, y: bar.y1 };
    const draw = (from: Point, to: Point): Stroke => {
        for (const point of [from, to]) {
            if (point.x - pen / 2 < 0 || point.x + pen / 2 > 3500
                || point.y - pen / 2 < 0
                || point.y + pen / 2 > 4500) {
                throw new Error(`${label} and pen clip glyph cell`);
            }
        }
        return validateStroke({ grapheme: glyph,
            x1: from.x, y1: from.y, x2: to.x, y2: to.y,
            width: pen, color: upper.color });
    };
    return [
        draw({ x: entry.x - lead, y: entry.y + lead }, entry),
        ...(bar === undefined ? [] : [draw(entry, upperPoint)]),
        ...arc.toReversed().map((stroke) => {
            const from = { x: stroke.x2, y: stroke.y2 };
            const to = { x: stroke.x1, y: stroke.y1 };
            return draw(from, to);
        }),
        draw(bottom, { x: bottom.x + lead, y: bottom.y - lead }),
    ];
}

// Explicit connected i/í/l/t/f keeps the printed downward stem and adds
// short entry/exit strokes. Dots, accents and t/f crossbars retain a pen
// lift; only the printed l has no second mark.
function cursiveStem(base: Stroke[], radius: number): Stroke[] {
    const [stem, mark] = base;
    if (stem === undefined || (stem.grapheme !== "l"
        && mark === undefined)) {
        throw new Error("Cursive stem requires the authored base strokes");
    }
    const lead = Math.min(200, Math.round(radius / 5));
    const pen = stem.width;
    const draw = (from: Point, to: Point): Stroke => {
        for (const point of [from, to]) {
            if (point.x - pen / 2 < 0 || point.x + pen / 2 > 3500
                || point.y - pen / 2 < 0
                || point.y + pen / 2 > 4500) {
                throw new Error("Cursive stem and pen clip glyph cell");
            }
        }
        return validateStroke({ grapheme: stem.grapheme,
            x1: from.x, y1: from.y, x2: to.x, y2: to.y,
            width: pen, color: stem.color });
    };
    const start = { x: stem.x1, y: stem.y1 };
    const end = { x: stem.x2, y: stem.y2 };
    return [
        draw({ x: start.x - lead, y: start.y + lead }, start),
        stem,
        draw(end, { x: end.x + lead, y: end.y - lead }),
        ...(mark === undefined ? [] : [mark]),
    ];
}

// Explicit v/w/z and smooth s connectors wrap existing directed contours.
// Their 2/4/3 straight segments or 64 sampled quadratic chords are
// unchanged. No intermediate pen lifts or adjacent-letter joins are added.
function cursiveContour(base: Stroke[], radius: number): Stroke[] {
    const first = base[0];
    const last = base.at(-1);
    const glyph = first?.grapheme ?? "";
    const counts: Record<string, number> = { v: 2, w: 4, z: 3, s: 64 };
    if (!first || !last || base.length !== counts[glyph]
        || base.some((stroke, index) => stroke.grapheme !== glyph
            || (index > 0 && (base[index - 1]!.x2 !== stroke.x1
                || base[index - 1]!.y2 !== stroke.y1)))) {
        throw new Error("Cursive contour needs an uninterrupted source");
    }
    const lead = Math.min(200, Math.round(radius / 5));
    const pen = first.width;
    const draw = (from: Point, to: Point): Stroke => {
        for (const point of [from, to]) {
            if (point.x - pen / 2 < 0 || point.x + pen / 2 > 3500
                || point.y - pen / 2 < 0
                || point.y + pen / 2 > 4500) {
                throw new Error(glyph === "s"
                    ? "Cursive s and pen clip glyph cell"
                    : "Cursive zigzag and pen clip glyph cell");
            }
        }
        return validateStroke({ grapheme: glyph,
            x1: from.x, y1: from.y, x2: to.x, y2: to.y,
            width: pen, color: first.color });
    };
    const start = { x: first.x1, y: first.y1 };
    const end = { x: last.x2, y: last.y2 };
    return [
        draw({ x: start.x - lead, y: start.y + lead }, start),
        ...base,
        draw(end, { x: end.x + lead, y: end.y - lead }),
    ];
}

// The printed y contains two arms that meet at one fork and a descending
// tail. A deliberate reverse of the right arm reaches its start, then the
// original right arm and tail complete one pen-down path. Printed ink stays
// untouched and the extra retrace is explicit, not an inferred word join.
function cursiveFork(base: Stroke[], radius: number): Stroke[] {
    if (base.length !== 3 || base.some((stroke) =>
        stroke.grapheme !== "y")) {
        throw new Error("Cursive y needs the three printed fork strokes");
    }
    const [left, right, tail] = base as [Stroke, Stroke, Stroke];
    if (left.x2 !== right.x2 || left.y2 !== right.y2
        || tail.x1 !== left.x2 || tail.y1 !== left.y2) {
        throw new Error("Cursive y requires a shared fork junction");
    }
    const lead = Math.min(200, Math.round(radius / 5));
    const pen = left.width;
    const draw = (from: Point, to: Point): Stroke => {
        for (const point of [from, to]) {
            if (point.x - pen / 2 < 0 || point.x + pen / 2 > 3500
                || point.y - pen / 2 < 0
                || point.y + pen / 2 > 4500) {
                throw new Error("Cursive y connector and pen clip glyph cell");
            }
        }
        return validateStroke({ grapheme: "y",
            x1: from.x, y1: from.y, x2: to.x, y2: to.y,
            width: pen, color: left.color });
    };
    const start = { x: left.x1, y: left.y1 };
    const fork = { x: left.x2, y: left.y2 };
    const rightTop = { x: right.x1, y: right.y1 };
    const finish = { x: tail.x2, y: tail.y2 };
    return [
        draw({ x: start.x - lead, y: start.y + lead }, start),
        left, draw(fork, rightTop), right, tail,
        draw(finish, { x: finish.x + lead, y: finish.y - lead }),
    ];
}

// The printed k uses three detached strokes. A down-and-back stem,
// upper diagonal out-and-back, and lower outgoing diagonal form one
// deliberate pen gesture. Every original stroke is retained in its
// authored direction; both retraces and lifted word joins stay explicit.
function cursiveK(base: Stroke[], radius: number): Stroke[] {
    if (base.length !== 3 || base.some((stroke) =>
        stroke.grapheme !== "k")) {
        throw new Error("Cursive k needs the printed stem and diagonals");
    }
    const [stem, upper, lower] = base as [Stroke, Stroke, Stroke];
    if (stem.x1 !== stem.x2 || stem.x1 !== upper.x2
        || upper.x2 !== lower.x1 || upper.y2 !== lower.y1
        || !(stem.y1 < upper.y2 && upper.y2 < stem.y2)) {
        throw new Error("Cursive k needs a shared stem and diagonal fork");
    }
    const lead = Math.min(200, Math.round(radius / 5));
    const pen = stem.width;
    const draw = (from: Point, to: Point): Stroke => {
        for (const point of [from, to]) {
            if (point.x - pen / 2 < 0 || point.x + pen / 2 > 3500
                || point.y - pen / 2 < 0
                || point.y + pen / 2 > 4500) {
                throw new Error("Cursive k connector and pen clip glyph cell");
            }
        }
        return validateStroke({ grapheme: "k",
            x1: from.x, y1: from.y, x2: to.x, y2: to.y,
            width: pen, color: stem.color });
    };
    const start = { x: stem.x1, y: stem.y1 };
    const bottom = { x: stem.x2, y: stem.y2 };
    const fork = { x: upper.x2, y: upper.y2 };
    const upperEnd = { x: upper.x1, y: upper.y1 };
    const finish = { x: lower.x2, y: lower.y2 };
    return [
        draw({ x: start.x - lead, y: start.y + lead }, start),
        stem, draw(bottom, fork), draw(fork, upperEnd), upper, lower,
        draw(finish, { x: finish.x + lead, y: finish.y - lead }),
    ];
}

// Optional j keeps its straight stem and sampled hooked descender in
// one directed pen gesture. Its dot must remain a separate pen lift.
// This does not connect neighboring glyphs or change the printed j.
function cursiveHook(base: Stroke[], radius: number): Stroke[] {
    if (base.length !== ARCH_SEGMENTS + 2
        || base.some((stroke) => stroke.grapheme !== "j")) {
        throw new Error("Cursive j needs the authored hook and dot");
    }
    const stem = base[0]!;
    const turn = base.slice(1, -1);
    const dot = base.at(-1)!;
    const end = turn.at(-1)!;
    if (stem.x2 !== turn[0]!.x1 || stem.y2 !== turn[0]!.y1) {
        throw new Error("Cursive j needs a continuous printed hook");
    }
    const lead = Math.min(200, Math.round(radius / 5));
    const pen = stem.width;
    const draw = (from: Point, to: Point): Stroke => {
        for (const point of [from, to]) {
            if (point.x - pen / 2 < 0 || point.x + pen / 2 > 3500
                || point.y - pen / 2 < 0
                || point.y + pen / 2 > 4500) {
                throw new Error("Cursive j connector and pen clip glyph cell");
            }
        }
        return validateStroke({ grapheme: "j",
            x1: from.x, y1: from.y, x2: to.x, y2: to.y,
            width: pen, color: stem.color });
    };
    const start = { x: stem.x1, y: stem.y1 };
    const finish = { x: end.x2, y: end.y2 };
    return [
        draw({ x: start.x - lead, y: start.y + lead }, start),
        stem, ...turn,
        draw(finish, { x: finish.x + lead, y: finish.y - lead }),
        dot,
    ];
}

// Optional n/h/u/r keep the printed rounded shoulder in one pen gesture.
// n/h/r retrace the rising left stem to meet the arch; u already connects.
// r ends at its shoulder without inventing the n's descending right stem.
// Exact accent and diaeresis marks remain separate lifted pen paths.
function cursiveRounded(base: Stroke[], radius: number): Stroke[] {
    const glyph = base[0]?.grapheme;
    if (!["n", "h", "u", "r"].includes(glyph ?? "")
        || base.length !== 2 * ARCH_SEGMENTS + (glyph === "r" ? 1 : 2)) {
        throw new Error("Cursive rounded glyph needs the printed contour");
    }
    const first = base[0]!;
    const arc = base[1]!;
    const last = base.at(-1)!;
    const lead = Math.min(200, Math.round(radius / 5));
    const pen = first.width;
    const draw = (from: Point, to: Point): Stroke => {
        for (const point of [from, to]) {
            if (point.x - pen / 2 < 0 || point.x + pen / 2 > 3500
                || point.y - pen / 2 < 0
                || point.y + pen / 2 > 4500) {
                throw new Error("Cursive hump and pen clip glyph cell");
            }
        }
        return validateStroke({ grapheme: first.grapheme,
            x1: from.x, y1: from.y, x2: to.x, y2: to.y,
            width: pen, color: first.color });
    };
    const start = { x: first.x1, y: first.y1 };
    const stemTop = { x: first.x2, y: first.y2 };
    const hump = { x: arc.x1, y: arc.y1 };
    const end = { x: last.x2, y: last.y2 };
    return [
        draw({ x: start.x - lead, y: start.y + lead }, start),
        first,
        ...(["n", "h", "r"].includes(first.grapheme)
            ? [draw(stemTop, hump)] : []),
        ...base.slice(1),
        draw(end, { x: end.x + lead, y: end.y - lead }),
    ];
}

// Printed m lifts between its first stem and first hump, and between
// the descending middle stem and the next hump. The manually selected
// connected variant retraces those two stems while the pen stays down;
// this does not change the original double-hump quadratic samples.
function cursiveDoubleHump(base: Stroke[], radius: number): Stroke[] {
    if (base.length !== 67 || base.some((s) => s.grapheme !== "m")) {
        throw new Error("Cursive m requires the full printed double hump");
    }
    const first = base[0]!;
    const middle = base[33]!;
    const last = base[66]!;
    const lead = Math.min(200, Math.round(radius / 5));
    const pen = first.width;
    const draw = (from: Point, to: Point): Stroke => {
        for (const point of [from, to]) {
            if (point.x - pen / 2 < 0 || point.x + pen / 2 > 3500
                || point.y - pen / 2 < 0
                || point.y + pen / 2 > 4500) {
                throw new Error("Cursive m connector and pen clip glyph cell");
            }
        }
        return validateStroke({ grapheme: "m",
            x1: from.x, y1: from.y, x2: to.x, y2: to.y,
            width: pen, color: first.color });
    };
    const start = { x: first.x1, y: first.y1 };
    const stemTop = { x: first.x2, y: first.y2 };
    const hump = { x: base[1]!.x1, y: base[1]!.y1 };
    const middleEnd = { x: middle.x2, y: middle.y2 };
    const nextHump = { x: base[34]!.x1, y: base[34]!.y1 };
    const end = { x: last.x2, y: last.y2 };
    return [
        draw({ x: start.x - lead, y: start.y + lead }, start),
        first, draw(stemTop, hump), ...base.slice(1, 33),
        middle, draw(middleEnd, nextHump), ...base.slice(34),
        draw(end, { x: end.x + lead, y: end.y - lead }),
    ];
}

// Optional one-click starters reuse the same accent composer as the manual
// Spanish fill. They never normalize or overwrite an existing Unicode key.
const ACCENTED_STARTERS: Readonly<Record<string, string>> = {
    "é": "e", "ú": "u", "ü": "u", "ñ": "n",
    "Á": "A", "É": "E", "Í": "I", "Ó": "O",
    "Ú": "U", "Ñ": "N", "Ü": "U",
};

// One-click NFD ink is opt-in and retains its exact Unicode TSV key.
// Its sampled geometry is identical to the caller-selected NFC starter.
const NFD_STARTERS = ["á", "é", "í", "ó", "ú", "ñ", "ü",
    "Á", "É", "Í", "Ó", "Ú", "Ñ", "Ü"]
    .map((glyph) => glyph.normalize("NFD"));

export function geometricStarter(
    g: string, x: number, y: number, r: number, w: number,
    color: string, pos?: { dx: number; lift: number;
        italicDegrees?: number },
    kind: "single" | "two-storey" | "italic" | "cursive"
        | "two-storey-italic" = "single",
): Stroke[] {
    if (NFD_STARTERS.includes(g)) {
        const composed = g.normalize("NFC");
        return geometricStarter(composed, x, y, r, w, color, pos, kind)
            .map((stroke) => ({ ...stroke, grapheme: g }));
    }
    if (!["o", "a", "ó", "á", "é", "ú", "ü", "ñ",
        "Á", "É", "Í", "Ó", "Ú", "Ñ", "Ü",
        "b", "d", "p", "q",
        "n", "u", "h", "m", "r", "l", "t", "f", "v", "w",
        "k", "x", "y", "z", "c", "e", "i", "í", "j", "g", "s",
        ".", ",", "!", "¡", "?", "¿",
        "«", "»", "‘", "’", "“", "”",
        ...SELECTABLE_MATH_GLYPHS, ...CIRCLE_MARKS,
        ...CURRENCY_MARKS,
        ..."0123456789", ..."ABCDEFGHIJKLMNOPQRSTUVWXYZ",
        ...TEXT_MARKS, ...NOTATION_MARKS].includes(g)) {
        throw new Error("Choose a supported exact geometric glyph");
    }
    const twoStorey = kind === "two-storey"
        || kind === "two-storey-italic";
    const italic = kind === "italic" || kind === "two-storey-italic";
    if (twoStorey && !["a", "á"].includes(g)) {
        throw new Error("Two-storey construction requires a or á");
    }
    if (!["single", "two-storey", "italic", "two-storey-italic",
        "cursive"].includes(kind) || (italic
            && !["o", "a", "ó", "á"].includes(g))) {
        throw new Error("Italic variant requires matching o/a glyph");
    }
    if (kind === "cursive" && !["o", "a", "ó", "á", "c", "e", "é",
        "i", "í", "j", "l", "t", "f", "n", "ñ", "u", "ú", "ü",
        "m", "h", "r", "v", "w", "z", "s", "y", "k"]
        .includes(g)) {
        throw new Error("Cursive variant requires o/a, c/e, i/í, "
            + "j, l/t/f, n/ñ, h/r, u/ú/ü, m, v/w/z, s/y/k glyph");
    }
    const accentBase = ACCENTED_STARTERS[g];
    if (accentBase !== undefined) {
        const baseKind = kind === "cursive"
            && ["é", "ñ", "ú", "ü"].includes(g)
            ? "cursive" : "single";
        const input = [accentBase, x, y, r, w, color, pos] as const;
        const base = geometricStarter(...input, baseKind);
        return composeAccentedGlyph(base, accentBase, g, color, {
            dx: pos?.dx ?? 0, lift: pos?.lift ?? 0,
        });
    }
    if (kind === "cursive") {
        const base = geometricStarter(g, x, y, r, w, color, pos, "single");
        if (g === "c" || g === "e") return cursiveOpenArc(base, r);
        if (g === "j") return cursiveHook(base, r);
        if (g === "y") return cursiveFork(base, r);
        if (g === "k") return cursiveK(base, r);
        if (["v", "w", "z", "s"].includes(g)) {
            return cursiveContour(base, r);
        }
        if (["i", "í", "l", "t", "f"].includes(g)) {
            return cursiveStem(base, r);
        }
        if (["n", "u", "h", "r"].includes(g)) {
            return cursiveRounded(base, r);
        }
        if (g === "m") return cursiveDoubleHump(base, r);
        return cursiveLoop(base, r);
    }
    if (italic) {
        const angle = pos?.italicDegrees ?? 12;
        if (!Number.isSafeInteger(angle) || angle < 0 || angle > 20) {
            throw new Error("Italic angle must be 0..20 whole degrees");
        }
        const baseKind = twoStorey ? "two-storey" : "single";
        const base = geometricStarter(g, x, y, r, w, color, pos,
            baseKind);
        return italicizeGlyphStrokes(base, g, angle, y + r);
    }
    const cx = boundedInteger(String(x), 3500, "center X");
    const cy = boundedInteger(String(y), 4500, "center Y");
    const radius = boundedInteger(String(r), 1600, "radius");
    const pen = boundedInteger(String(w), 1500, "width");
    const dx = pos?.dx ?? 0;
    if (!Number.isSafeInteger(dx) || dx < -500 || dx > 500) {
        throw new Error("Acute shift must be within -500..500 µm");
    }
    const lift = boundedInteger(
        String(pos?.lift ?? 0), 500, "accent lift");
    const halfPen = pen / 2;
    const topOffset = twoStorey ? 2 * radius : radius;
    if (radius < 200 || cx - radius < halfPen
        || cy - radius < halfPen
        || cx + radius + halfPen > 3500
        || cy + radius + halfPen > 4500) {
        throw new Error("Shape and pen thickness must fit inside glyph cell");
    }
    if ([...TEXT_MARKS, ...NOTATION_MARKS].includes(
        g as TextMark)) {
        if ([";", ":", "…"].includes(g) && radius * 0.5 < pen + 80) {
            throw new Error("Text punctuation needs dot pen-clearance");
        }
        const line = (from: Point, to: Point): Stroke => {
            for (const point of [from, to]) {
                if (point.x - halfPen < 0 || point.x + halfPen > 3500
                    || point.y - halfPen < 0
                    || point.y + halfPen > 4500) {
                    throw new Error("Text punctuation ink clips glyph cell");
                }
            }
            return validateStroke({ grapheme: g,
                x1: from.x, y1: from.y, x2: to.x, y2: to.y,
                width: pen, color });
        };
        return textMarkParts(cx, cy, radius, g as TextMark).flatMap(
            ({ from, control, to }) => control === undefined
                ? [line(from, to)]
                : Array.from({ length: ARCH_SEGMENTS }, (_, step) =>
                    line(quadratic(from, control, to, step),
                        quadratic(from, control, to, step + 1))));
    }
    if (/^[A-Z]$/.test(g)) {
        const line = (from: Point, to: Point): Stroke => {
            for (const point of [from, to]) {
                if (point.x - halfPen < 0 || point.x + halfPen > 3500
                    || point.y - halfPen < 0
                    || point.y + halfPen > 4500) {
                    throw new Error("Capital ink must fit inside glyph cell");
                }
            }
            return validateStroke({ grapheme: g,
                x1: from.x, y1: from.y, x2: to.x, y2: to.y,
                width: pen, color });
        };
        const parts = capitalParts(cx, cy, radius, g as CapitalGlyph);
        const output: Stroke[] = [];
        if (g === "O" || g === "Q") {
            const circle = (step: number): Point => {
                const a = 2 * Math.PI * step / CIRCLE_SEGMENTS;
                return { x: Math.round(cx + radius * Math.cos(a)),
                    y: Math.round(cy + radius * Math.sin(a)) };
            };
            for (let step = 0; step < CIRCLE_SEGMENTS; step += 1) {
                output.push(line(circle(step), circle(step + 1)));
            }
        } else if (g === "C" || g === "G") {
            for (let step = 0; step < OPEN_ARC_SEGMENTS; step += 1) {
                const first = openCirclePoint(cx, cy, radius, step);
                const last = openCirclePoint(cx, cy, radius, step + 1);
                output.push(line({ x: Math.round(first.x),
                    y: Math.round(first.y) }, { x: Math.round(last.x),
                    y: Math.round(last.y) }));
            }
        } else if (g === "S") {
            for (const [from, control, to] of smoothS(cx, cy, radius)) {
                for (let step = 0; step < ARCH_SEGMENTS; step += 1) {
                    output.push(line(quadratic(from, control, to, step),
                        quadratic(from, control, to, step + 1)));
                }
            }
        }
        for (const { from, control, to } of parts) {
            if (control === undefined) output.push(line(from, to));
            else for (let step = 0; step < ARCH_SEGMENTS; step += 1) {
                output.push(line(quadratic(from, control, to, step),
                    quadratic(from, control, to, step + 1)));
            }
        }
        return output;
    }
    if (/^[0-9]$/.test(g)) {
        const geometry = digitGeometry(cx, cy, radius, g as DigitGlyph);
        const line = (from: Point, to: Point): Stroke => {
            for (const point of [from, to]) {
                if (point.x - halfPen < 0 || point.x + halfPen > 3500
                    || point.y - halfPen < 0
                    || point.y + halfPen > 4500) {
                    throw new Error("Numeral ink must fit inside glyph cell");
                }
            }
            return validateStroke({ grapheme: g,
                x1: from.x, y1: from.y, x2: to.x, y2: to.y,
                width: pen, color });
        };
        const ellipses = geometry.ellipses.flatMap((ellipse) =>
            Array.from({ length: CIRCLE_SEGMENTS }, (_, step) =>
                line(ellipsePoint(ellipse, step),
                    ellipsePoint(ellipse, step + 1))));
        const parts = geometry.parts.flatMap(({ from, control, to }) =>
            control === undefined ? [line(from, to)]
                : Array.from({ length: ARCH_SEGMENTS }, (_, step) =>
                    line(quadratic(from, control, to, step),
                        quadratic(from, control, to, step + 1))));
        return [...ellipses, ...parts];
    }
    if ((CIRCLE_MARKS as readonly string[]).includes(g)) {
        const geometry = circleMarkGeometry(cx, cy, radius,
            g as CircleMark);
        if (geometry.ellipses.some((ring) => ring.rx < pen + 40)) {
            throw new Error("Degree/percent loops need pen-clearance");
        }
        const line = (from: Point, to: Point): Stroke => {
            for (const point of [from, to]) {
                if (point.x - halfPen < 0 || point.x + halfPen > 3500
                    || point.y - halfPen < 0
                    || point.y + halfPen > 4500) {
                    throw new Error("Degree/percent ink clips glyph cell");
                }
            }
            return validateStroke({ grapheme: g,
                x1: from.x, y1: from.y, x2: to.x, y2: to.y,
                width: pen, color });
        };
        const ink: Stroke[] = [];
        for (const ring of geometry.ellipses) {
            for (let step = 0; step < CIRCLE_SEGMENTS; step += 1) {
                const start = ellipsePoint(ring, step);
                const end = ellipsePoint(ring, step + 1);
                ink.push(line(start, end));
            }
        }
        for (const { from, to } of geometry.parts) {
            ink.push(line(from, to));
        }
        return ink;
    }
    if ((SELECTABLE_MATH_GLYPHS as readonly string[]).includes(g)) {
        if (["⊕", "⊗", "⊙", "⊖", "⊘"].includes(g)
            && radius < 2 * pen) {
            throw new Error("Circled math requires inner pen-clearance");
        }
        if (["⊑", "⊒"].includes(g) && radius * 0.33 < pen + 60) {
            throw new Error("Square subset underbar needs pen-clearance");
        }
        if (["≼", "≽"].includes(g) && radius * 0.31 < pen + 60) {
            throw new Error("Precedence underbar needs pen-clearance");
        }
        if (["⊴", "⊵"].includes(g) && radius * 0.31 < pen + 60) {
            throw new Error("Subgroup underbar needs pen-clearance");
        }
        if (["⋖", "⋗"].includes(g) && radius * 0.35 < pen + 60) {
            throw new Error("Dotted comparison needs pen-clearance");
        }
        if (["⋘", "⋙"].includes(g) && radius * 0.3 < pen + 60) {
            throw new Error("Triple comparison needs pen-clearance");
        }
        if (["≶", "≷", "⋚", "⋛"].includes(g) && radius
            * (["⋚", "⋛"].includes(g) ? 0.32 : 0.64) < pen + 60) {
            throw new Error("Stacked comparison needs pen-clearance");
        }
        if (["⊞", "⊟", "⊠", "⊡"].includes(g)
            && radius * 0.38 < pen + 80) {
            throw new Error("Square math operator needs pen-clearance");
        }
        if (g === "÷" && radius * 0.58 < pen + 100) {
            throw new Error("Division dots need visible pen-clearance");
        }
        const line = (from: Point, to: Point): Stroke => {
            for (const point of [from, to]) {
                if (point.x - halfPen < 0
                    || point.x + halfPen > 3500
                    || point.y - halfPen < 0
                    || point.y + halfPen > 4500) {
                    throw new Error("Math ink must fit inside glyph cell");
                }
            }
            return validateStroke({
                grapheme: g, x1: from.x, y1: from.y,
                x2: to.x, y2: to.y, width: pen, color,
            });
        };
        return mathParts(cx, cy, radius, g as MathGlyph).flatMap(
            ({ from, control, to }) => control === undefined
                ? [line(from, to)]
                : Array.from({ length: ARCH_SEGMENTS }, (_, step) =>
                    line(quadratic(from, control, to, step),
                        quadratic(from, control, to, step + 1))));
    }
    if ((CURRENCY_MARKS as readonly string[]).includes(g)) {
        const line = (from: Point, to: Point): Stroke => {
            for (const point of [from, to]) {
                if (point.x - halfPen < 0 || point.x + halfPen > 3500
                    || point.y - halfPen < 0
                    || point.y + halfPen > 4500) {
                    throw new Error("Currency ink clips glyph cell");
                }
            }
            return validateStroke({ grapheme: g,
                x1: from.x, y1: from.y, x2: to.x, y2: to.y,
                width: pen, color });
        };
        const parts = currencyParts(cx, cy, radius, g as CurrencyMark);
        const arc = g === "€" ? Array.from({ length: OPEN_ARC_SEGMENTS },
            (_, step) => {
                const first = openCirclePoint(cx, cy, radius, step);
                const last = openCirclePoint(cx, cy, radius, step + 1);
                return line({ x: Math.round(first.x),
                    y: Math.round(first.y) },
                { x: Math.round(last.x), y: Math.round(last.y) });
            }) : [];
        return [...arc, ...parts.flatMap(({ from, control, to }) =>
            control === undefined ? [line(from, to)]
                : Array.from({ length: ARCH_SEGMENTS }, (_, step) =>
                    line(quadratic(from, control, to, step),
                        quadratic(from, control, to, step + 1))))];
    }
    if (["«", "»", "‘", "’", "“", "”"].includes(g)) {
        const glyph = g as QuoteGlyph;
        const pieces = quoteParts(cx, cy, radius, quotedSource(glyph));
        const orient = (point: Point): Point => quoteMirror(cx, glyph, point);
        const line = (from: Point, to: Point): Stroke => {
            for (const point of [from, to]) {
                if (point.x - halfPen < 0 || point.x + halfPen > 3500
                    || point.y - halfPen < 0
                    || point.y + halfPen > 4500) {
                    throw new Error("Quotation ink must fit inside glyph cell");
                }
            }
            return validateStroke({ grapheme: g,
                x1: from.x, y1: from.y, x2: to.x, y2: to.y,
                width: pen, color });
        };
        return pieces.flatMap(({ from, control, to }) =>
            control === undefined ? [line(orient(from), orient(to))]
                : Array.from({ length: ARCH_SEGMENTS }, (_, step) =>
                    line(orient(quadratic(from, control, to, step)),
                        orient(quadratic(from, control, to, step + 1)))));
    }
    if ([".", ",", "!", "¡", "?", "¿"].includes(g)) {
        // Rotate already-rounded native samples so the inverted template
        // remains an exact integer transform, not a separately rounded curve.
        const inverted = g === "¡" || g === "¿";
        const source = g === "¡" ? "!" : g === "¿" ? "?"
            : g as PunctuationGlyph;
        const parts = punctuationParts(cx, cy, radius, source);
        const orient = (point: Point): Point => inverted
            ? { x: 2 * cx - point.x, y: 2 * cy - point.y } : point;
        if (g === "!" || g === "¡" || g === "?" || g === "¿") {
            const before = parts.at(-2);
            const dot = parts.at(-1);
            if (before === undefined || dot === undefined
                || Math.abs(dot.from.y - before.to.y) < pen + 100) {
                throw new Error("Punctuation dot needs visible pen-clearance");
            }
        }
        const line = (from: Point, to: Point): Stroke => {
            for (const point of [from, to]) {
                if (point.x < halfPen || point.x + halfPen > 3500
                    || point.y < halfPen || point.y + halfPen > 4500) {
                    throw new Error(
                        "Punctuation ink must fit inside glyph cell");
                }
            }
            return validateStroke({ grapheme: g,
                x1: from.x, y1: from.y, x2: to.x, y2: to.y,
                width: pen, color });
        };
        return parts.flatMap(({ from, to, control }) => {
            if (control === undefined) return [line(orient(from), orient(to))];
            return Array.from({ length: ARCH_SEGMENTS }, (_, step) =>
                line(orient(quadratic(from, control, to, step)),
                    orient(quadratic(from, control, to, step + 1))));
        });
    }
    if (g === "s") {
        const lines: Stroke[] = [];
        for (const [from, control, to] of smoothS(cx, cy, radius)) {
            for (let step = 0; step < ARCH_SEGMENTS; step += 1) {
                const first = quadratic(from, control, to, step);
                const last = quadratic(from, control, to, step + 1);
                lines.push(validateStroke({
                    grapheme: g, x1: first.x, y1: first.y,
                    x2: last.x, y2: last.y, width: pen, color,
                }));
            }
        }
        return lines;
    }
    if (["n", "u", "h", "m", "r"].includes(g)) {
        const line = (first: Point, last: Point): Stroke => {
            return validateStroke({
                grapheme: g, x1: first.x, y1: first.y,
                x2: last.x, y2: last.y, width: pen, color,
            });
        };
        if (g === "m") {
            const parts = doubleHump(cx, cy, radius);
            const lines = [line(parts.firstStem.from, parts.firstStem.to)];
            for (const [index, [from, control, to]]
                of parts.arcs.entries()) {
                for (let step = 0; step < ARCH_SEGMENTS; step += 1) {
                    const first = quadratic(from, control, to, step);
                    const last = quadratic(from, control, to, step + 1);
                    lines.push(line(first, last));
                }
                if (index === 1) {
                    const middle = parts.middleStem;
                    lines.push(line(middle.from, middle.to));
                }
            }
            lines.push(line(parts.lastStem.from, parts.lastStem.to));
            return lines;
        }
        const bow = roundedStory(cx, cy, radius,
            g === "r" ? "n" : g as "n" | "u" | "h");
        if (g === "h" && bow.firstStem.to.y < halfPen) {
            throw new Error("Ascender stem and pen must fit inside glyph cell");
        }
        const output = [line(bow.firstStem.from, bow.firstStem.to)];
        for (const [from, control, to] of [
            [bow.start, bow.left, bow.peak],
            [bow.peak, bow.right, bow.end],
        ] as [Point, Point, Point][]) {
            for (let step = 0; step < ARCH_SEGMENTS; step += 1) {
                const first = quadratic(from, control, to, step);
                const last = quadratic(from, control, to, step + 1);
                output.push(line(first, last));
            }
        }
        if (g !== "r") {
            output.push(line(bow.lastStem.from, bow.lastStem.to));
        }
        return output;
    }
    if (["v", "w", "k", "x", "y", "z"].includes(g)) {
        const lines = straightLines(cx, cy, radius, g as StraightGlyph);
        if (lines.some(({ from, to }) => [from, to].some((point) =>
            point.x < halfPen || point.y < halfPen
            || point.x + halfPen > 3500
            || point.y + halfPen > 4500))) {
            throw new Error(
                "Straight strokes and pen must fit inside glyph cell",
            );
        }
        return lines.map(({ from, to }) => validateStroke({
            grapheme: g, x1: from.x, y1: from.y,
            x2: to.x, y2: to.y, width: pen, color,
        }));
    }
    if (g === "l" || g === "t" || g === "f") {
        const stem = tallStem(cx, cy, radius, g);
        if (stem.from.y < halfPen) {
            throw new Error("Tall stem and pen must fit inside glyph cell");
        }
        const lines = [validateStroke({
            grapheme: g, x1: stem.from.x, y1: stem.from.y,
            x2: stem.to.x, y2: stem.to.y, width: pen, color,
        })];
        if (g === "t" || g === "f") lines.push(validateStroke({
            grapheme: g, x1: cx - radius, y1: cy,
            x2: cx + Math.round(radius * 0.55), y2: cy,
            width: pen, color,
        }));
        return lines;
    }
    if (g === "c" || g === "e") {
        const result = Array.from({ length: OPEN_ARC_SEGMENTS },
            (_, index) => {
                const first = openCirclePoint(cx, cy, radius, index);
                const last = openCirclePoint(cx, cy, radius, index + 1);
                return validateStroke({
                    grapheme: g, x1: Math.round(first.x),
                    y1: Math.round(first.y), x2: Math.round(last.x),
                    y2: Math.round(last.y), width: pen, color,
                });
            });
        if (g === "e") result.push(validateStroke({
            grapheme: g, x1: cx - radius, y1: cy,
            x2: cx + Math.round(radius * 0.55), y2: cy,
            width: pen, color,
        }));
        return result;
    }
    if (g === "j") {
        const parts = hookedJ(cx, cy, radius, pen);
        if (parts.turn[1].y + halfPen > 4500
            || parts.turn[2].x < halfPen) {
            throw new Error("Hooked descender and pen must fit glyph cell");
        }
        const line = (from: Point, to: Point): Stroke =>
            validateStroke({ grapheme: g, x1: from.x, y1: from.y,
                x2: to.x, y2: to.y, width: pen, color });
        const segments = [line(parts.stem.from, parts.stem.to)];
        const [from, control, to] = parts.turn;
        for (let step = 0; step < ARCH_SEGMENTS; step += 1) {
            segments.push(line(quadratic(from, control, to, step),
                quadratic(from, control, to, step + 1)));
        }
        segments.push(line(parts.dot.from, parts.dot.to));
        return segments;
    }
    if (g === "i" || g === "í") {
        const parts = dottedI(cx, cy, radius, pen);
        const segments = [parts.stem];
        if (g === "i") {
            segments.push(parts.dot);
        } else {
            const acute = acuteAccent(cx, cy, radius, pen, dx, lift);
            if (acute.y2 < halfPen || acute.x1 < halfPen
                || acute.x2 + halfPen > 3500) {
                throw new Error("Acute accent clips glyph cell");
            }
            segments.push({
                from: { x: acute.x1, y: acute.y1 },
                to: { x: acute.x2, y: acute.y2 },
            });
        }
        return segments.map((part) => validateStroke({
            grapheme: g, x1: part.from.x, y1: part.from.y,
            x2: part.to.x, y2: part.to.y, width: pen, color,
        }));
    }
    const point = (step: number): Point => {
        const radians = 2 * Math.PI * step / CIRCLE_SEGMENTS;
        return {
            x: Math.round(cx + radius * Math.cos(radians)),
            y: Math.round(cy + radius * Math.sin(radians)),
        };
    };
    const lines = Array.from({ length: CIRCLE_SEGMENTS }, (_, index) => {
        const first = point(index);
        const last = point(index + 1);
        return validateStroke({
            grapheme: g, x1: first.x, y1: first.y,
            x2: last.x, y2: last.y, width: pen, color,
        });
    });
    if (g === "g") {
        const parts = hookedG(cx, cy, radius);
        if (parts.turn[1].y + halfPen > 4500) {
            throw new Error("Loop hook and pen must fit inside glyph cell");
        }
        const line = (from: Point, to: Point): Stroke =>
            validateStroke({ grapheme: g, x1: from.x, y1: from.y,
                x2: to.x, y2: to.y, width: pen, color });
        lines.push(line(parts.stem.from, parts.stem.to));
        const [from, control, to] = parts.turn;
        for (let step = 0; step < ARCH_SEGMENTS; step += 1) {
            lines.push(line(quadratic(from, control, to, step),
                quadratic(from, control, to, step + 1)));
        }
        return lines;
    }
    if (twoStorey) {
        if (cy - topOffset < halfPen) {
            throw new Error("Two-storey arch exceeds glyph cell");
        }
        const { start, peak, end, left, right } =
            upperStory(cx, cy, radius);
        const spans: [Point, Point, Point][] = [
            [start, left, peak], [peak, right, end],
        ];
        for (const [from, control, to] of spans) {
            for (let step = 0; step < ARCH_SEGMENTS; step += 1) {
                const first = quadratic(from, control, to, step);
                const last = quadratic(from, control, to, step + 1);
                lines.push(validateStroke({
                    grapheme: g, x1: first.x, y1: first.y,
                    x2: last.x, y2: last.y, width: pen, color,
                }));
            }
        }
    }
    if (["a", "á", "b", "d", "p", "q"].includes(g)) {
        const { from, to } = loopStem(cx, cy, radius,
            g as "a" | "á" | "b" | "d" | "p" | "q");
        if (from.y - halfPen < 0 || to.y + halfPen > 4500) {
            throw new Error("Loop stem and pen must fit inside glyph cell");
        }
        lines.push(validateStroke({
            grapheme: g, x1: from.x, y1: from.y,
            x2: to.x, y2: to.y, width: pen, color,
        }));
    }
    if (g === "ó" || g === "á") {
        const accent = acuteAccent(
            cx, cy, radius, pen, dx, lift, topOffset);
        if (accent.y2 < halfPen || accent.x1 < halfPen
            || accent.x2 + halfPen > 3500) {
            throw new Error("Acute accent and pen must fit inside glyph cell");
        }
        lines.push(validateStroke({
            grapheme: g, ...accent, width: pen, color,
        }));
    }
    return lines;
}

// Compose one explicitly selected Latin accent from an already-authored
// base, never by normalizing or silently substituting another glyph. The
// authored source strokes and their colors remain unchanged in the draft.
export function composeAccentedGlyph(
    strokes: Stroke[], source: string, target: string, color: string,
    pos: { dx: number; lift: number } = { dx: 0, lift: 0 },
): Stroke[] {
    if (!exactGrapheme(source) || !exactGrapheme(target)
        || !/^[a-zA-Z]$/.test(source)) {
        throw new Error("Accent requires an exact Latin base and target");
    }
    const decomposed = [...target.normalize("NFD")];
    if (decomposed.length !== 2 || decomposed[0] !== source) {
        throw new Error("Target must be the selected base plus one accent");
    }
    const combining = decomposed[1];
    const valid = (combining === "\u0301" && /[AEIOUaeiou]/.test(source))
        || (combining === "\u0303" && /[Nn]/.test(source))
        || (combining === "\u0308" && /[Uu]/.test(source));
    if (!valid) {
        throw new Error("Only vowel acute, n tilde and u diaeresis supported");
    }
    if (!Number.isSafeInteger(pos.dx) || pos.dx < -500
        || pos.dx > 500) {
        throw new Error("Accent shift must be within -500..500 µm");
    }
    const lift = boundedInteger(String(pos.lift), 500, "accent lift");
    if (!/^#[0-9a-fA-F]{6}$/.test(color)) {
        throw new Error("Ink color requires #RRGGBB");
    }
    let base = strokes.filter((stroke) => stroke.grapheme === source)
        .map(validateStroke);
    if (base.length === 0) {
        throw new Error("Source grapheme has no authored strokes");
    }
    // A precomposed or decomposed acute i has no dot. Only remove a clearly
    // separate short upper dot; never guess which pen strokes to discard
    // from a nonstandard user-authored i.
    if (source === "i" && combining === "\u0301") {
        const dots = base.flatMap((stroke, index) => {
            const high = Math.min(stroke.y1, stroke.y2);
            const low = Math.max(stroke.y1, stroke.y2);
            const separated = base.every((other, otherIndex) =>
                otherIndex === index || low + stroke.width / 2 + 100
                < Math.min(other.y1, other.y2) - other.width / 2);
            return stroke.x1 === stroke.x2 && low - high >= 40
                && low - high <= 200 && separated ? [index] : [];
        });
        if (base.length !== 2 || dots.length !== 1) {
            throw new Error("Acute i requires one stem and a clear upper dot");
        }
        base = base.filter((_, index) => index !== dots[0]);
    }
    const xs = base.flatMap((stroke) => [stroke.x1, stroke.x2]);
    const topY = Math.min(...base.flatMap((stroke) => [stroke.y1, stroke.y2]));
    const center = Math.round((Math.min(...xs) + Math.max(...xs)) / 2)
        + pos.dx;
    const span = Math.max(...xs) - Math.min(...xs);
    const half = Math.round(Math.max(240, Math.min(800, span * 0.32)) / 2);
    const pen = Math.max(...base.map((stroke) => stroke.width));
    const y = topY - pen - 120 - lift;
    const rise = Math.round(Math.max(120, Math.min(440, span * 0.35)));
    const point = (x: number, yCoord: number) => ({
        x: Math.round(x), y: Math.round(yCoord),
    });
    const segments: [Point, Point][] = combining === "\u0301"
        ? [[point(center - half, y), point(center + half, y - rise)]]
        : combining === "\u0303" ? [
            [point(center - half, y), point(center - half / 3, y - 120)],
            [point(center - half / 3, y - 120),
                point(center + half / 3, y)],
            [point(center + half / 3, y), point(center + half, y - 120)],
        ] : [
            [point(center - half * 0.55, y),
                point(center - half * 0.55, y - 80)],
            [point(center + half * 0.55, y),
                point(center + half * 0.55, y - 80)],
        ];
    const accent = segments.map(([first, last]) => {
        for (const p of [first, last]) {
            if (p.x - pen / 2 < 0 || p.x + pen / 2 > 3500
                || p.y - pen / 2 < 0 || p.y + pen / 2 > 4500) {
                throw new Error("Accent and pen must fit inside glyph cell");
            }
        }
        return validateStroke({
            grapheme: target, x1: first.x, y1: first.y,
            x2: last.x, y2: last.y, width: pen, color,
        });
    });
    return [...base.map((stroke) => ({ ...stroke, grapheme: target })),
        ...accent];
}

// The frozen 114-grapheme English/Spanish text baseline is distinct from
// optional math and notation symbols. Compose on a disposable candidate so
// one clipped mark, ambiguous custom i, or native TSV cap refuses *all* fills.
const BILINGUAL_DIRECT = [
    ..."abcdefghijklmnopqrstuvwxyz", ".", ",", "!", "¡", "?", "¿",
    "«", "»", "‘", "’", "“", "”", "-", ..."0123456789",
    ..."ABCDEFGHIJKLMNOPQRSTUVWXYZ", ...TEXT_MARKS,
];
const BILINGUAL_ACCENTS = [
    ["a", "á"], ["e", "é"], ["i", "í"], ["o", "ó"],
    ["u", "ú"], ["n", "ñ"], ["u", "ü"],
] as const;
function bilingualAccentVariants(
    uppercase: boolean, normalization: "NFC" | "NFD",
): { source: string; exact: string }[] {
    return BILINGUAL_ACCENTS.map(([base, target]) => ({
        source: uppercase ? base.toUpperCase() : base,
        exact: (uppercase ? target.toUpperCase() : target)
            .normalize(normalization),
    }));
}
const BILINGUAL_VARIANTS = [
    ...bilingualAccentVariants(false, "NFC"),
    ...bilingualAccentVariants(false, "NFD"),
    ...bilingualAccentVariants(true, "NFC"),
    ...bilingualAccentVariants(true, "NFD"),
];
const BILINGUAL_KEYS = [
    ...BILINGUAL_DIRECT, ...BILINGUAL_VARIANTS.map(({ exact }) => exact),
];

// Reporting is read-only, with independent NFC and NFD keys. No implicit
// normalization, inferred glyph shape, or persistent state is involved.
export function missingBilingualBaseline(strokes: readonly Stroke[]): string[] {
    const authored = new Set(strokes.map((stroke) => stroke.grapheme));
    return BILINGUAL_KEYS.filter((glyph) => !authored.has(glyph));
}

export function fillMissingBilingualBaseline(
    existing: Stroke[], x: number, y: number, r: number, pen: number,
    color: string, pos: { dx: number; lift: number },
): Stroke[] {
    const authored = new Set(existing.map((stroke) => stroke.grapheme));
    const next = existing.slice();
    for (const glyph of BILINGUAL_DIRECT) {
        if (!authored.has(glyph)) {
            next.push(...geometricStarter(glyph, x, y, r, pen, color));
            authored.add(glyph);
        }
    }
    for (const { source, exact } of BILINGUAL_VARIANTS) {
        if (!authored.has(exact)) {
            next.push(...composeAccentedGlyph(
                next, source, exact, color, pos));
            authored.add(exact);
        }
    }
    if (next.length === existing.length) return existing;
    encodeGlyphLibrary(next);
    return next;
}

// Deterministic, bounded variation of one occurrence. These shared glyph-level
// transforms make height, width, slant and ink thickness correlated across its
// strokes; matching endpoints receive identical local irregularity. Nothing
// is sampled into the caller's template or saved as a hidden alternate glyph.
export type InkVariation = {
    width: number;
    height: number;
    thickness: number;
    slant: number;
    spacing: number;
    regularity: number;
};

function seededUnit(seed: number): () => number {
    let state = seed >>> 0 || 0x6D2B79F5;
    return () => {
        state ^= state << 13;
        state ^= state >>> 17;
        state ^= state << 5;
        return (state >>> 0) / 0xFFFFFFFF * 2 - 1;
    };
}

function occurrenceSeed(seed: number, occurrence: number): number {
    return (seed ^ Math.imul(occurrence + 1, 0x9E3779B9)) >>> 0;
}

function checkVariation(seed: number, occurrence: number,
    variation: InkVariation): void {
    if (!Number.isSafeInteger(seed) || seed < 0 || seed > 0xFFFFFFFF
        || !Number.isSafeInteger(occurrence) || occurrence < 0
        || occurrence > 10000) {
        throw new Error("Variation seed or occurrence outside bounds");
    }
    for (const [key, max] of [
        ["width", 20], ["height", 20], ["thickness", 20],
        ["slant", 12], ["spacing", 500], ["regularity", 150],
    ] as const) {
        const value = variation[key];
        if (!Number.isSafeInteger(value) || value < 0 || value > max) {
            throw new Error(`Variation ${key} outside 0..${max}`);
        }
    }
}

export function varyGlyphStrokes(
    source: Stroke[], grapheme: string, seed: number, occurrence: number,
    variation: InkVariation,
): Stroke[] {
    checkVariation(seed, occurrence, variation);
    if (!exactGrapheme(grapheme)) {
        throw new Error("Preview requires one exact Unicode glyph");
    }
    const authored = source.filter((item) => item.grapheme === grapheme)
        .map(validateStroke);
    if (authored.length > 128) {
        throw new Error("Preview is limited to 128 selected strokes");
    }
    if (authored.length === 0) {
        throw new Error("Selected glyph has no authored ink");
    }
    const random = seededUnit(occurrenceSeed(seed, occurrence));
    const sx = 1 + random() * variation.width / 100;
    const sy = 1 + random() * variation.height / 100;
    const ink = 1 + random() * variation.thickness / 100;
    const shear = Math.tan(random() * variation.slant * Math.PI / 180);
    // Vertex noise depends on physical coordinates, not the input stroke
    // order: moving a pen stroke in the edit list must not change its shape.
    const point = (x: number, y: number): Point => {
        const vertexRandom = seededUnit(occurrenceSeed(seed, occurrence)
            ^ Math.imul(x, 0x85EBCA6B) ^ Math.imul(y, 0xC2B2AE35));
        return {
            x: Math.round(1750 + (x - 1750) * sx
                + (4500 - y) * shear
                + vertexRandom() * variation.regularity),
            y: Math.round(2250 + (y - 2250) * sy
                + vertexRandom() * variation.regularity),
        };
    };
    return authored.map((stroke) => {
        const a = point(stroke.x1, stroke.y1);
        const b = point(stroke.x2, stroke.y2);
        const width = Math.round(stroke.width * ink);
        for (const { x, y } of [a, b]) {
            if (x - width / 2 < 0 || x + width / 2 > 3500
                || y - width / 2 < 0 || y + width / 2 > 4500) {
                throw new Error(
                    "Varied ink and pen must fit inside glyph cell",
                );
            }
        }
        return validateStroke({
            ...stroke, x1: a.x, y1: a.y, x2: b.x, y2: b.y, width,
        });
    });
}

function required<T extends Element>(id: string): T {
    const node = document.getElementById(id);
    if (node === null) throw new Error(`Missing glyph editor control: ${id}`);
    return node as unknown as T;
}

export function createGlyphStrokeEditor(): {
    start: () => void;
    reset: () => void;
} {
    const character = required<HTMLInputElement>("glyph-character");
    const color = required<HTMLInputElement>("glyph-color");
    const source = required<HTMLInputElement>("glyph-load");
    const add = required<HTMLButtonElement>("glyph-add");
    const copyFrom = required<HTMLInputElement>("glyph-copy-from");
    const copy = required<HTMLButtonElement>("glyph-copy");
    const compose = required<HTMLButtonElement>("glyph-compose");
    const form = required<HTMLSelectElement>("glyph-form");
    const construct = required<HTMLButtonElement>("glyph-construct");
    const seedLowercase = required<HTMLButtonElement>("glyph-seed-lowercase");
    const seedSpanish = required<HTMLButtonElement>("glyph-seed-spanish");
    const seedSpanishNfd =
        required<HTMLButtonElement>("glyph-seed-spanish-nfd");
    const seedUpperSpanish =
        required<HTMLButtonElement>("glyph-seed-upper-spanish");
    const seedUpperSpanishNfd =
        required<HTMLButtonElement>("glyph-seed-upper-spanish-nfd");
    const seedPunctuation =
        required<HTMLButtonElement>("glyph-seed-punctuation");
    const seedQuotes = required<HTMLButtonElement>("glyph-seed-quotes");
    const seedMath = required<HTMLButtonElement>("glyph-seed-math");
    const seedDigits = required<HTMLButtonElement>("glyph-seed-digits");
    const seedCapitals =
        required<HTMLButtonElement>("glyph-seed-capitals");
    const seedTextMarks =
        required<HTMLButtonElement>("glyph-seed-text-marks");
    const seedBaseline =
        required<HTMLButtonElement>("glyph-seed-baseline");
    const focusMissing =
        required<HTMLButtonElement>("glyph-focus-missing");
    const baselineCoverage =
        required<HTMLElement>("glyph-baseline-coverage");
    const centerX = required<HTMLInputElement>("glyph-circle-x");
    const centerY = required<HTMLInputElement>("glyph-circle-y");
    const radius = required<HTMLInputElement>("glyph-circle-radius");
    const accentShift = required<HTMLInputElement>("glyph-accent-shift");
    const accentLift = required<HTMLInputElement>("glyph-accent-lift");
    const italicAngle = required<HTMLInputElement>("glyph-italic-angle");
    const authoredAngle = required<HTMLInputElement>("glyph-authored-angle");
    const authoredBaseline =
        required<HTMLInputElement>("glyph-authored-baseline");
    const slantAuthored =
        required<HTMLButtonElement>("glyph-slant-authored");
    const undoTransform = required<HTMLButtonElement>("glyph-slant-undo");
    const redoTransform = required<HTMLButtonElement>("glyph-slant-redo");
    const slantPreview = required<SVGSVGElement>("glyph-slant-preview");
    const slantPreviewStatus =
        required<HTMLElement>("glyph-slant-preview-status");
    const heightPercent = required<HTMLInputElement>("glyph-height-percent");
    const heightBaseline =
        required<HTMLInputElement>("glyph-height-baseline");
    const scaleHeight = required<HTMLButtonElement>("glyph-scale-height");
    const heightPreview = required<SVGSVGElement>("glyph-height-preview");
    const heightPreviewStatus =
        required<HTMLElement>("glyph-height-preview-status");
    const widthPercent = required<HTMLInputElement>("glyph-width-percent");
    const widthAnchor = required<HTMLInputElement>("glyph-width-anchor");
    const scaleWidth = required<HTMLButtonElement>("glyph-scale-width");
    const widthPreview = required<SVGSVGElement>("glyph-width-preview");
    const widthPreviewStatus =
        required<HTMLElement>("glyph-width-preview-status");
    const rotateAngle = required<HTMLInputElement>("glyph-rotate-angle");
    const rotatePivotX =
        required<HTMLInputElement>("glyph-rotate-pivot-x");
    const rotatePivotY =
        required<HTMLInputElement>("glyph-rotate-pivot-y");
    const rotateGlyph = required<HTMLButtonElement>("glyph-rotate-all");
    const rotatePreview = required<SVGSVGElement>("glyph-rotate-preview");
    const rotatePreviewStatus =
        required<HTMLElement>("glyph-rotate-preview-status");
    const moveX = required<HTMLInputElement>("glyph-move-x");
    const moveY = required<HTMLInputElement>("glyph-move-y");
    const moveGlyph = required<HTMLButtonElement>("glyph-move-all");
    const movePreview = required<SVGSVGElement>("glyph-move-preview");
    const movePreviewStatus =
        required<HTMLElement>("glyph-move-preview-status");
    const inkPercent = required<HTMLInputElement>("glyph-ink-percent");
    const scaleInk = required<HTMLButtonElement>("glyph-scale-ink");
    const inkPreview = required<SVGSVGElement>("glyph-ink-preview");
    const inkPreviewStatus =
        required<HTMLElement>("glyph-ink-preview-status");
    const idealGeometry = required<SVGGElement>("glyph-ideal-geometry");
    const ideal = required<SVGSVGElement>("glyph-ideal-preview");
    const idealCircle = required<SVGCircleElement>("glyph-ideal-circle");
    const idealStem = required<SVGLineElement>("glyph-ideal-stem");
    const idealEntry = required<SVGLineElement>("glyph-ideal-entry");
    const idealExit = required<SVGLineElement>("glyph-ideal-exit");
    const idealApproach = required<SVGLineElement>("glyph-ideal-approach");
    const idealAccent = required<SVGLineElement>("glyph-ideal-accent");
    const idealDiacritic =
        required<SVGPathElement>("glyph-ideal-diacritic");
    const idealArch = required<SVGPathElement>("glyph-ideal-arch");
    const roundedPath = required<SVGPathElement>("glyph-ideal-rounded");
    const roundedLeft = required<SVGLineElement>("glyph-ideal-rounded-left");
    const roundedRight = required<SVGLineElement>("glyph-ideal-rounded-right");
    const openPath = required<SVGPathElement>("glyph-ideal-c-loop");
    const eBar = required<SVGLineElement>("glyph-ideal-e-bar");
    const iStem = required<SVGLineElement>("glyph-ideal-i-stem");
    const iDot = required<SVGLineElement>("glyph-ideal-i-dot");
    const selected = required<HTMLSelectElement>("glyph-selected");
    const update = required<HTMLButtonElement>("glyph-update");
    const remove = required<HTMLButtonElement>("glyph-remove");
    const reverse = required<HTMLButtonElement>("glyph-reverse");
    const reversePath = required<HTMLButtonElement>("glyph-reverse-path");
    const startPath = required<HTMLButtonElement>("glyph-path-start");
    const closePath = required<HTMLButtonElement>("glyph-path-close");
    const translatePath = required<HTMLButtonElement>("glyph-path-translate");
    const pathDx = required<HTMLInputElement>("glyph-path-dx");
    const pathDy = required<HTMLInputElement>("glyph-path-dy");
    const earlierPath = required<HTMLButtonElement>("glyph-path-earlier");
    const laterPath = required<HTMLButtonElement>("glyph-path-later");
    const earlier = required<HTMLButtonElement>("glyph-earlier");
    const later = required<HTMLButtonElement>("glyph-later");
    const join = required<HTMLButtonElement>("glyph-join");
    const save = required<HTMLButtonElement>("glyph-save");
    const preview = required<SVGSVGElement>("glyph-preview");
    const status = required<HTMLElement>("glyph-status");
    const pathsStatus = required<HTMLElement>("glyph-paths");
    const variationPreview = required<SVGSVGElement>("glyph-variation-preview");
    const variationStatus = required<HTMLElement>("glyph-variation-status");
    const variationSeed = required<HTMLInputElement>("glyph-variation-seed");
    const variationSample =
        required<HTMLSelectElement>("glyph-variation-sample");
    const variationDownload =
        required<HTMLButtonElement>("glyph-variation-download");
    const variationControls = {} as Record<
        keyof InkVariation, HTMLInputElement
    >;
    for (const key of ["width", "height", "thickness", "slant", "spacing",
        "regularity"] as const) {
        variationControls[key] = required<HTMLInputElement>(
            `glyph-variation-${key}`,
        );
    }

    const numeric = {} as Record<Axis, HTMLInputElement>;
    const ranges = {} as Record<Axis, HTMLInputElement>;
    for (const axis of AXES) {
        numeric[axis] = required<HTMLInputElement>(`glyph-${axis}`);
        ranges[axis] = required<HTMLInputElement>(`glyph-${axis}-range`);
    }
    const controls = [character, color, source, add, copyFrom, copy, compose,
        form, construct, seedLowercase, seedSpanish, seedSpanishNfd,
        seedUpperSpanish, seedUpperSpanishNfd,
        seedPunctuation, seedQuotes, seedMath, seedDigits, seedCapitals,
        seedTextMarks, seedBaseline, focusMissing,
        centerX, centerY, radius,
        accentShift, accentLift, italicAngle,
        authoredAngle, authoredBaseline, slantAuthored,
        heightPercent, heightBaseline, scaleHeight,
        widthPercent, widthAnchor, scaleWidth,
        inkPercent, scaleInk, moveX, moveY, moveGlyph,
        rotateAngle, rotatePivotX, rotatePivotY, rotateGlyph,
        undoTransform, redoTransform, variationSeed,
        variationSample, variationDownload,
        ...Object.values(variationControls),
        selected, update,
        remove, reverse, reversePath, startPath, closePath, translatePath,
        pathDx, pathDy,
        earlierPath, laterPath, earlier, later, join, save,
        ...AXES.flatMap((axis) => [numeric[axis], ranges[axis]])];
    let active = false;
    let generation = 0;
    let strokes: Stroke[] = [];
    let selectedIndex: number | null = null;
    let drawing: {
        pointerId: number;
        grapheme: string;
        color: string;
        width: number;
        points: Point[];
        last: Point;
        overflow: boolean;
    } | null = null;
    let downloadUrl: string | null = null;
    // One session-only recovery step for a selected whole-glyph transform.
    // Exact TSV snapshots prevent any later ink edit from being overwritten.
    type TransformKind = "slant" | "height" | "width" | "ink"
        | "move" | "rotation";
    let transformHistory: {
        before: string; after: string;
        side: "before" | "after"; grapheme: string;
        kind: TransformKind;
    } | null = null;

    const message = (text: string): void => { status.textContent = text; };
    const download = (bytes: string, filename: string): void => {
        const blob = new Blob([bytes], { type: "text/tab-separated-values" });
        if (downloadUrl !== null) URL.revokeObjectURL(downloadUrl);
        const url = URL.createObjectURL(blob);
        downloadUrl = url;
        const link = document.createElement("a");
        link.href = url;
        link.download = filename;
        link.rel = "noopener";
        link.click();
        window.setTimeout(() => {
            if (downloadUrl === url) {
                URL.revokeObjectURL(url);
                downloadUrl = null;
            }
        }, 60_000);
    };
    const neighbor = (offset: -1 | 1): number | null => {
        if (selectedIndex === null) return null;
        const grapheme = strokes[selectedIndex]?.grapheme;
        let index = selectedIndex + offset;
        while (index >= 0 && index < strokes.length) {
            if (strokes[index]?.grapheme === grapheme) return index;
            index += offset;
        }
        return null;
    };
    const selectedForm = () => {
        const italic = form.value.endsWith("-italic");
        const cursive = form.value.endsWith("-cursive");
        const raw = italic ? form.value.slice(0, -7)
            : cursive ? form.value.slice(0, -8) : form.value;
        const twoStorey = raw.endsWith("-two");
        return {
            grapheme: twoStorey ? raw.slice(0, -4) : raw,
            variant: twoStorey
                ? italic ? "two-storey-italic" as const
                    : "two-storey" as const
                : italic ? "italic" as const
                    : cursive ? "cursive" as const : "single" as const,
        };
    };
    const starterFromInputs = (): Stroke[] => {
        const chosen = selectedForm();
        const values: [string, number, number, number, number, string] = [
            chosen.grapheme, Number(centerX.value), Number(centerY.value),
            Number(radius.value), Number(numeric.width.value), color.value,
        ];
        const position = {
            dx: Number(accentShift.value), lift: Number(accentLift.value),
            italicDegrees: italicAngle.value === ""
                ? NaN : Number(italicAngle.value),
        };
        return geometricStarter(...values, position, chosen.variant);
    };
    const renderIdeal = (): void => {
        try {
            // The SVG circle remains mathematically circular. Exporting the
            // v1 library explicitly samples it into straight ink segments.
            const drafted = starterFromInputs();
            const cx = Number(centerX.value);
            const cy = Number(centerY.value);
            const r = Number(radius.value);
            idealCircle.setAttribute("cx", String(cx));
            idealCircle.setAttribute("cy", String(cy));
            idealCircle.setAttribute("r", String(r));
            idealCircle.setAttribute("stroke-width", numeric.width.value);
            idealCircle.setAttribute("stroke", color.value);
            const shown = active ? "visible" : "hidden";
            const selectedGlyph = selectedForm();
            // Use canonical geometry only for the read-only SVG guide;
            // starterFromInputs still saves the caller's exact NFD key.
            const chosen = { ...selectedGlyph,
                grapheme: selectedGlyph.grapheme.normalize("NFC") };
            const isItalic = chosen.variant.endsWith("italic");
            italicAngle.disabled = !active || !isItalic;
            const slope = isItalic
                ? Math.tan(Number(italicAngle.value) * Math.PI / 180) : 0;
            const baseline = cy + r;
            idealGeometry.setAttribute("transform", isItalic
                ? `matrix(1 0 ${-slope} 1 ${slope * baseline} 0)` : "");
            const circleVisible = ["o", "a", "ó", "á", "b", "d",
                "p", "q", "g", "O", "Ó", "Q"].includes(chosen.grapheme);
            const circleState = circleVisible ? shown : "hidden";
            idealCircle.setAttribute("visibility", circleState);
            const stemGlyph = ["a", "á", "b", "d", "p", "q"]
                .includes(chosen.grapheme)
                ? chosen.grapheme as "a" | "á" | "b" | "d" | "p" | "q"
                : "a";
            const stem = chosen.grapheme === "g"
                ? hookedG(cx, cy, r).stem
                : loopStem(cx, cy, r, stemGlyph);
            idealStem.setAttribute("x1", String(stem.from.x));
            idealStem.setAttribute("y1", String(stem.from.y));
            idealStem.setAttribute("x2", String(stem.to.x));
            idealStem.setAttribute("y2", String(stem.to.y));
            idealStem.setAttribute("stroke-width", numeric.width.value);
            idealStem.setAttribute("stroke", color.value);
            const stemVisible = active && ["a", "á", "b", "d", "p", "q", "g"]
                .includes(chosen.grapheme);
            const stemState = stemVisible ? "visible" : "hidden";
            idealStem.setAttribute("visibility", stemState);
            const connected = chosen.variant === "cursive";
            const lead = Math.min(200, Math.round(r / 5));
            const endY = ["a", "á"].includes(chosen.grapheme)
                ? cy + r : cy;
            const cursiveE = connected && ["e", "é"]
                .includes(chosen.grapheme);
            const cursiveC = connected && chosen.grapheme === "c";
            const cursiveArc = cursiveE || cursiveC;
            const cursiveStem = connected && ["i", "í", "l", "t", "f"]
                .includes(chosen.grapheme);
            const cursiveJ = connected && chosen.grapheme === "j";
            const jParts = chosen.grapheme === "j"
                ? hookedJ(cx, cy, r, Number(numeric.width.value)) : null;
            const jHookEnd = cursiveJ && jParts !== null
                ? jParts.turn[2] : null;
            const cursiveZig = connected && ["v", "w", "z"]
                .includes(chosen.grapheme);
            let zigLines: StraightLine[] = [];
            if (cursiveZig) {
                zigLines = straightLines(cx, cy, r,
                    chosen.grapheme as "v" | "w" | "z");
            }
            const smoothParts = connected && chosen.grapheme === "s"
                ? smoothS(cx, cy, r) : null;
            const cursiveY = connected && chosen.grapheme === "y";
            const yLines = cursiveY ? straightLines(cx, cy, r, "y") : [];
            const cursiveKForm = connected && chosen.grapheme === "k";
            const kLines = cursiveKForm ? straightLines(cx, cy, r, "k") : [];
            const contourStart = zigLines[0]?.from ?? smoothParts?.[0]?.[0];
            const contourEnd = zigLines.at(-1)?.to
                ?? smoothParts?.at(-1)?.[2];
            const cursiveUp = connected && ["n", "ñ", "h", "r"]
                .includes(chosen.grapheme);
            const cursiveU = connected && ["u", "ú", "ü"]
                .includes(chosen.grapheme);
            const cursiveM = connected && chosen.grapheme === "m";
            const upKind = chosen.grapheme === "h" ? "h" : "n";
            const upHump = roundedStory(cx, cy, r, upKind);
            const uBowl = roundedStory(cx, cy, r, "u");
            const roundedExit = cursiveUp
                ? (chosen.grapheme === "r"
                    ? upHump.end : upHump.lastStem.to)
                : uBowl.lastStem.to;
            const stemStartY = ["l", "f"].includes(chosen.grapheme)
                ? cy - Math.round(r * 1.5)
                : chosen.grapheme === "t" ? cy - r
                    : cy - Math.round(r * 0.4);
            const eUpper = openCirclePoint(cx, cy, r, OPEN_ARC_SEGMENTS);
            const eLower = openCirclePoint(cx, cy, r, 0);
            const connectors: [SVGLineElement, Point, Point][] = [
                [idealEntry,
                    cursiveStem || cursiveJ
                        ? { x: cx - lead, y: stemStartY + lead }
                        : cursiveM ? {
                            x: cx - r - lead, y: cy + r + lead }
                        : cursiveUp || cursiveU ? {
                            x: cx - r - lead,
                            y: (cursiveUp ? upHump : uBowl).firstStem.from.y
                                + lead }
                        : cursiveC
                            ? { x: eUpper.x - lead, y: eUpper.y + lead }
                            : { x: cx - r - lead, y: cy + lead },
                    cursiveStem || cursiveJ ? { x: cx, y: stemStartY }
                        : cursiveM ? { x: cx - r, y: cy + r }
                        : cursiveUp || cursiveU
                            ? (cursiveUp ? upHump : uBowl).firstStem.from
                        : cursiveC ? eUpper : { x: cx - r, y: cy }],
                [idealApproach,
                    cursiveM ? { x: cx - r, y: cy - r }
                        : cursiveUp ? upHump.firstStem.to
                        : cursiveE ? { x: cx - r, y: cy }
                            : { x: cx + r, y: cy },
                    cursiveM ? { x: cx - r, y: cy }
                        : cursiveUp ? upHump.start
                        : cursiveE ? eUpper : { x: cx + r, y: cy - r }],
                [idealExit,
                    cursiveStem ? { x: cx, y: cy + r }
                        : cursiveM ? { x: cx + r, y: cy + r }
                        : cursiveUp || cursiveU
                            ? roundedExit
                            : cursiveArc ? eLower : { x: cx + r, y: endY },
                    cursiveStem ? { x: cx + lead, y: cy + r - lead }
                        : cursiveM ? {
                            x: cx + r + lead, y: cy + r - lead }
                        : cursiveUp || cursiveU ? {
                            x: cx + r + lead,
                            y: roundedExit.y - lead }
                        : cursiveArc
                            ? { x: eLower.x + lead, y: eLower.y - lead }
                            : { x: cx + r + lead, y: endY - lead }],
            ];
            if (jHookEnd !== null) {
                const exit = connectors[2]!;
                exit[1] = jHookEnd;
                exit[2] = { x: jHookEnd.x + lead, y: jHookEnd.y - lead };
            }
            if (contourStart !== undefined && contourEnd !== undefined) {
                const entry = connectors[0]!;
                entry[1] = { x: contourStart.x - lead,
                    y: contourStart.y + lead };
                entry[2] = contourStart;
                const exit = connectors[2]!;
                exit[1] = contourEnd;
                exit[2] = { x: contourEnd.x + lead, y: contourEnd.y - lead };
            }
            const yStart = yLines[0]?.from;
            const yJoint = yLines[0]?.to;
            const yTop = yLines[1]?.from;
            const yFinish = yLines[2]?.to;
            if (yStart !== undefined && yJoint !== undefined
                && yTop !== undefined && yFinish !== undefined) {
                const entry = connectors[0]!;
                entry[1] = { x: yStart.x - lead, y: yStart.y + lead };
                entry[2] = yStart;
                const retrace = connectors[1]!;
                retrace[1] = yJoint;
                retrace[2] = yTop;
                const exit = connectors[2]!;
                exit[1] = yFinish;
                exit[2] = { x: yFinish.x + lead, y: yFinish.y - lead };
            }
            const kStem = kLines[0];
            const kUpper = kLines[1];
            const kLower = kLines[2];
            if (kStem && kUpper && kLower) {
                const entry = connectors[0]!;
                entry[1] = { x: kStem.from.x - lead,
                    y: kStem.from.y + lead };
                entry[2] = kStem.from;
                const upperReturn = connectors[1]!;
                upperReturn[1] = kUpper.to;
                upperReturn[2] = kUpper.from;
                const exit = connectors[2]!;
                exit[1] = kLower.to;
                exit[2] = { x: kLower.to.x + lead,
                    y: kLower.to.y - lead };
            }
            for (const [guide, from, to] of connectors) {
                for (const [axis, value] of [
                    ["x1", from.x], ["y1", from.y],
                    ["x2", to.x], ["y2", to.y],
                ] as const) guide.setAttribute(axis, String(value));
                guide.setAttribute("stroke-width", numeric.width.value);
                guide.setAttribute("stroke", color.value);
                guide.setAttribute("visibility", connected &&
                    (guide !== idealApproach || cursiveE || cursiveUp
                        || cursiveM || cursiveY || cursiveKForm
                        || ["a", "á"].includes(chosen.grapheme))
                    ? shown : "hidden");
            }
            const { start, peak, end, left, right } = upperStory(cx, cy, r);
            const beginning = ["M", start.x, start.y].join(" ");
            const firstArc = ["Q", left.x, left.y, peak.x, peak.y].join(" ");
            const nextArc = ["Q", right.x, right.y, end.x, end.y].join(" ");
            const archPath = [beginning, firstArc, nextArc].join(" ");
            idealArch.setAttribute("d", archPath);
            idealArch.setAttribute("stroke-width", numeric.width.value);
            idealArch.setAttribute("stroke", color.value);
            const archVisible = active
                && chosen.variant.startsWith("two-storey");
            const archState = archVisible ? "visible" : "hidden";
            idealArch.setAttribute("visibility", archState);
            const roundedKind: "n" | "u" | "h" =
                ["u", "ú", "ü"].includes(chosen.grapheme) ? "u"
                    : chosen.grapheme === "h" ? "h" : "n";
            const rounded = roundedStory(cx, cy, r, roundedKind);
            const doubled = doubleHump(cx, cy, r);
            const roundedStart = ["M", rounded.start.x,
                rounded.start.y].join(" ");
            const roundedFirst = ["Q", rounded.left.x, rounded.left.y,
                rounded.peak.x, rounded.peak.y].join(" ");
            const roundedSecond = ["Q", rounded.right.x, rounded.right.y,
                rounded.end.x, rounded.end.y].join(" ");
            const doubledCommands = doubled.arcs.map(([, control, end]) => {
                const coordinates = [control.x, control.y, end.x, end.y];
                return ["Q", ...coordinates].join(" ");
            });
            const doubledD = [
                `M ${cx - r} ${cy}`,
                ...doubledCommands,
                `M ${cx} ${cy} L ${cx} ${cy + r}`
                    + (cursiveM ? ` L ${cx} ${cy}` : ""),
            ].join(" ");
            const isStraight = ["v", "w", "k", "x", "y", "z"]
                .includes(chosen.grapheme);
            const straightD = isStraight ? straightGuide(straightLines(
                cx, cy, r, chosen.grapheme as StraightGlyph,
            )) : "";
            const gParts = chosen.grapheme === "g"
                ? hookedG(cx, cy, r) : null;
            const jTurn = jParts?.turn;
            const jD = jTurn === undefined ? ""
                : `M ${jTurn[0].x} ${jTurn[0].y}`
                    + ` Q ${jTurn[1].x} ${jTurn[1].y}`
                    + ` ${jTurn[2].x} ${jTurn[2].y}`;
            const gTurn = gParts?.turn;
            const gD = gTurn === undefined ? ""
                : `M ${gTurn[0].x} ${gTurn[0].y}`
                    + ` Q ${gTurn[1].x} ${gTurn[1].y}`
                    + ` ${gTurn[2].x} ${gTurn[2].y}`;
            const sD = chosen.grapheme === "s"
                ? smoothS(cx, cy, r).map(([from, control, to], index) =>
                    `${index === 0 ? `M ${from.x} ${from.y} ` : ""}`
                    + `Q ${control.x} ${control.y} ${to.x} ${to.y}`,
                ).join(" ") : "";
            const isPunctuation = [".", ",", "!", "¡", "?", "¿"]
                .includes(chosen.grapheme);
            const punctuationD = isPunctuation
                ? punctuationGuide(punctuationParts(cx, cy, r,
                    chosen.grapheme as PunctuationGlyph)) : "";
            const isQuote = ["«", "»", "‘", "’", "“", "”"]
                .includes(chosen.grapheme);
            const quoteD = isQuote ? punctuationGuide(quoteParts(
                cx, cy, r, quotedSource(chosen.grapheme as QuoteGlyph))
                .map(({ from, to, control }) => ({
                    from: quoteMirror(cx, chosen.grapheme as QuoteGlyph, from),
                    to: quoteMirror(cx, chosen.grapheme as QuoteGlyph, to),
                    ...(control ? { control: quoteMirror(
                        cx, chosen.grapheme as QuoteGlyph, control) } : {}),
                }))) : "";
            const isCircleMark = (CIRCLE_MARKS as readonly string[])
                .includes(chosen.grapheme);
            const isMath = isCircleMark
                || (SELECTABLE_MATH_GLYPHS as readonly string[])
                    .includes(chosen.grapheme);
            let mathD = "";
            if (isCircleMark) {
                const geometry = circleMarkGeometry(cx, cy, r,
                    chosen.grapheme as CircleMark);
                mathD = digitGuide(geometry);
            } else if (isMath) {
                mathD = punctuationGuide(mathParts(cx, cy, r,
                    chosen.grapheme as MathGlyph));
            }
            const isCurrency = (CURRENCY_MARKS as readonly string[])
                .includes(chosen.grapheme);
            const currencyD = isCurrency ? punctuationGuide(currencyParts(
                cx, cy, r, chosen.grapheme as CurrencyMark)) : "";
            const isDigit = /^[0-9]$/.test(chosen.grapheme);
            const digitD = isDigit ? digitGuide(digitGeometry(
                cx, cy, r, chosen.grapheme as DigitGlyph)) : "";
            const capitalSource = ACCENTED_STARTERS[chosen.grapheme]
                ?? chosen.grapheme;
            const isCapital = /^[A-Z]$/.test(capitalSource);
            const capitalD = isCapital ? capitalGuide(cx, cy, r,
                capitalSource as CapitalGlyph) : "";
            const isTextMark = [...TEXT_MARKS, ...NOTATION_MARKS]
                .includes(chosen.grapheme as TextMark);
            const markD = isTextMark ? punctuationGuide(textMarkParts(
                cx, cy, r, chosen.grapheme as TextMark)) : "";
            const roundedD = isCurrency ? currencyD
                : isTextMark ? markD
                : isCapital ? capitalD
                : isDigit ? digitD
                : isMath ? mathD
                : isQuote ? quoteD
                : isPunctuation ? punctuationD
                : isStraight ? straightD
                : chosen.grapheme === "s" ? sD
                : chosen.grapheme === "j" ? jD
                    : chosen.grapheme === "g" ? gD
                    : chosen.grapheme === "m" ? doubledD
                    : [roundedStart, roundedFirst, roundedSecond].join(" ");
            roundedPath.setAttribute("d", roundedD);
            roundedPath.setAttribute("stroke-width", numeric.width.value);
            roundedPath.setAttribute("stroke", color.value);
            for (const [element, part] of [
                [roundedLeft, chosen.grapheme === "m"
                    ? doubled.firstStem : rounded.firstStem],
                [roundedRight, chosen.grapheme === "m"
                    ? doubled.lastStem : rounded.lastStem],
            ] as [SVGLineElement, { from: Point; to: Point }][]) {
                element.setAttribute("x1", String(part.from.x));
                element.setAttribute("y1", String(part.from.y));
                element.setAttribute("x2", String(part.to.x));
                element.setAttribute("y2", String(part.to.y));
                element.setAttribute("stroke-width", numeric.width.value);
                element.setAttribute("stroke", color.value);
            }
            if (kStem && kUpper) {
                roundedLeft.setAttribute("x1", String(kStem.to.x));
                roundedLeft.setAttribute("y1", String(kStem.to.y));
                roundedLeft.setAttribute("x2", String(kUpper.to.x));
                roundedLeft.setAttribute("y2", String(kUpper.to.y));
            }
            const roundedVisible = active
                && (isCapital || ["n", "u", "ñ", "ú", "ü", "h", "m", "r",
                    "j", "g", "s",
                    "v", "w", "k", "x", "y", "z",
                    ".", ",", "!", "¡", "?", "¿",
                    "«", "»", "‘", "’", "“", "”",
                    ...SELECTABLE_MATH_GLYPHS, ...CIRCLE_MARKS,
                    ...CURRENCY_MARKS,
                    ..."0123456789", ..."ABCDEFGHIJKLMNOPQRSTUVWXYZ",
                    ...TEXT_MARKS, ...NOTATION_MARKS]
                    .includes(chosen.grapheme));
            const roundedState = roundedVisible ? "visible" : "hidden";
            const roundedStemState = roundedVisible && !isStraight
                && !isPunctuation && !isQuote && !isMath && !isDigit
                && !isCapital && !isTextMark && !isCurrency
                && !["j", "g", "s"].includes(chosen.grapheme)
                ? "visible" : "hidden";
            roundedPath.setAttribute("visibility", roundedState);
            roundedLeft.setAttribute("visibility", cursiveKForm
                ? shown : roundedStemState);
            const roundedRightState = chosen.grapheme === "r"
                ? "hidden" : roundedStemState;
            roundedRight.setAttribute("visibility", roundedRightState);
            const openStart = openCirclePoint(cx, cy, r, 0);
            const openEnd = openCirclePoint(cx, cy, r,
                OPEN_ARC_SEGMENTS);
            const arcHeader = ["M", openStart.x, openStart.y];
            const arcTail = ["A", r, r, 0, 1, 1,
                openEnd.x, openEnd.y];
            const openD = cursiveArc
                ? ["M", openEnd.x, openEnd.y, "A", r, r, 0, 1, 0,
                    openStart.x, openStart.y].join(" ")
                : [...arcHeader, ...arcTail].join(" ");
            openPath.setAttribute("d", openD);
            openPath.setAttribute("stroke-width", numeric.width.value);
            openPath.setAttribute("stroke", color.value);
            const openVisible = active && ["c", "e", "é", "C", "G", "€"]
                .includes(chosen.grapheme);
            const openState = openVisible ? "visible" : "hidden";
            openPath.setAttribute("visibility", openState);
            eBar.setAttribute("x1", String(cx - r));
            eBar.setAttribute("y1", String(cy));
            eBar.setAttribute("x2", String(cx + Math.round(r * 0.55)));
            eBar.setAttribute("y2", String(cy));
            eBar.setAttribute("stroke-width", numeric.width.value);
            eBar.setAttribute("stroke", color.value);
            const barState = active && !cursiveE
                && ["e", "é", "t", "f"].includes(chosen.grapheme)
                ? "visible" : "hidden";
            eBar.setAttribute("visibility", barState);
            const pen = Number(numeric.width.value);
            const iParts = ["i", "í", "j"].includes(chosen.grapheme)
                ? dottedI(cx, cy, r, pen)
                : dottedI(cx, cy, 1100, 250);
            if (jParts !== null) {
                iParts.stem = jParts.stem;
            }
            if (["l", "t", "f"].includes(chosen.grapheme)) {
                const tallKind = chosen.grapheme === "l" ? "l"
                    : chosen.grapheme === "f" ? "f" : "t";
                iParts.stem = tallStem(cx, cy, r, tallKind);
            }
            for (const [iElement, part] of [
                [iStem, iParts.stem], [iDot, iParts.dot],
            ] as [SVGLineElement, { from: Point; to: Point }][]) {
                iElement.setAttribute("x1", String(part.from.x));
                iElement.setAttribute("y1", String(part.from.y));
                iElement.setAttribute("x2", String(part.to.x));
                iElement.setAttribute("y2", String(part.to.y));
                iElement.setAttribute("stroke-width", numeric.width.value);
                iElement.setAttribute("stroke", color.value);
            }
            const iState = active && ["i", "í", "j", "l", "t", "f"]
                .includes(chosen.grapheme) ? "visible" : "hidden";
            const dotState = active && ["i", "j"].includes(chosen.grapheme)
                ? "visible" : "hidden";
            iStem.setAttribute("visibility", iState);
            iDot.setAttribute("visibility", dotState);
            const dx = Number(accentShift.value);
            const lift = Number(accentLift.value);
            const top = chosen.variant.startsWith("two-storey")
                ? 2 * r : r;
            const composedAcute = ["é", "ú", "Á", "É", "Í", "Ó", "Ú"]
                .includes(chosen.grapheme);
            const accent = composedAcute ? drafted.at(-1)!
                : acuteAccent(cx, cy, r, pen, dx, lift, top);
            for (const axis of ["x1", "y1", "x2", "y2"] as const) {
                idealAccent.setAttribute(axis, String(accent[axis]));
            }
            idealAccent.setAttribute("stroke-width", numeric.width.value);
            idealAccent.setAttribute("stroke", color.value);
            const hasAcute = ["ó", "á", "í", "é", "ú",
                "Á", "É", "Í", "Ó", "Ú"]
                .includes(chosen.grapheme);
            idealAccent.setAttribute("visibility", active && hasAcute
                ? "visible" : "hidden");
            const composedMarks = ["ñ", "Ñ"].includes(chosen.grapheme)
                ? 3 : ["ü", "Ü"].includes(chosen.grapheme) ? 2 : 0;
            const markLines = composedMarks > 0
                ? drafted.slice(-composedMarks) : [];
            const diacriticD = markLines.map((line) =>
                `M ${line.x1} ${line.y1} L ${line.x2} ${line.y2}`)
                .join(" ");
            idealDiacritic.setAttribute("d", diacriticD);
            idealDiacritic.setAttribute("stroke-width", numeric.width.value);
            idealDiacritic.setAttribute("stroke", color.value);
            idealDiacritic.setAttribute("visibility", active && composedMarks
                ? "visible" : "hidden");
            const guideLabel = connected
                ? `Manual connected-pen ${form.value} draft, no adjacent joining`
                : isMath
                ? `Exact mathematical ${form.value} ink guide`
                : isCurrency
                ? `Analytic currency ${form.value} guide, sampled to TSV`
                : isQuote
                ? (["«", "»"].includes(chosen.grapheme)
                    ? `Exact geometric ${form.value} chevrons`
                    : `Analytic ${form.value} curls sampled to TSV`)
                : isPunctuation
                ? (["?", "¿"].includes(chosen.grapheme)
                    ? `Analytic ${form.value} guide, sampled to TSV`
                    : `Exact straight ${form.value} punctuation ink`)
                : isStraight
                ? `Exact straight ${form.value} guide, retained in TSV`
                : `Analytic geometric ${form.value} guide, not retained in TSV`;
            ideal.setAttribute("aria-label", guideLabel);
        } catch {
            italicAngle.disabled = !active
                || !selectedForm().variant.endsWith("italic");
            idealGeometry.setAttribute("transform", "");
            idealCircle.setAttribute("visibility", "hidden");
            idealStem.setAttribute("visibility", "hidden");
            for (const guide of [idealEntry, idealApproach, idealExit]) {
                guide.setAttribute("visibility", "hidden");
            }
            idealArch.setAttribute("visibility", "hidden");
            roundedPath.setAttribute("visibility", "hidden");
            roundedLeft.setAttribute("visibility", "hidden");
            roundedRight.setAttribute("visibility", "hidden");
            openPath.setAttribute("visibility", "hidden");
            eBar.setAttribute("visibility", "hidden");
            iStem.setAttribute("visibility", "hidden");
            iDot.setAttribute("visibility", "hidden");
            idealAccent.setAttribute("visibility", "hidden");
            idealDiacritic.setAttribute("visibility", "hidden");
            ideal.setAttribute("aria-label", "Invalid geometric settings");
        }
    };
    const readVariation = (): { seed: number; settings: InkVariation } => {
        const seed = boundedInteger(variationSeed.value, 0xFFFFFFFF,
            "variation seed");
        const settings = {} as InkVariation;
        for (const key of ["width", "height", "thickness", "slant",
            "spacing", "regularity"] as const) {
            const raw = variationControls[key].value;
            if (!/^[0-9]+$/.test(raw)) {
                throw new Error(`Variation ${key} requires whole units`);
            }
            settings[key] = Number(raw);
        }
        checkVariation(seed, 0, settings);
        return { seed, settings };
    };
    const renderVariation = (): void => {
        const authored = strokes.filter(
            (item) => item.grapheme === character.value,
        );
        if (!active || authored.length === 0) {
            variationDownload.disabled = true;
            variationPreview.replaceChildren();
            variationStatus.textContent = active
                ? "Author this exact glyph to compare repeated ink"
                : "Requires a compatible backend session";
            return;
        }
        try {
            const { seed, settings } = readVariation();
            if (authored.length > 128) {
                throw new Error("Preview is limited to 128 selected strokes");
            }
            const lines: SVGLineElement[] = [];
            for (let row = 0; row < 2; row += 1) {
                for (let copy = 0; copy < 3; copy += 1) {
                    const sequence = row === 0 ? authored
                        : varyGlyphStrokes(authored, character.value,
                            seed, copy, settings);
                    const tracking = row === 0 ? 0 : Math.round(
                        seededUnit(occurrenceSeed(seed, copy + 3))()
                            * settings.spacing,
                    );
                    for (const ink of sequence) {
                        const line = document.createElementNS(SVG_NS, "line");
                        // Keep the +/-500 µm tracking inside SVG bounds.
                        const xOffset = 600 + copy * 4450 + tracking;
                        const yOffset = 200 + row * 4800;
                        line.setAttribute("x1", String(ink.x1 + xOffset));
                        line.setAttribute("y1", String(ink.y1 + yOffset));
                        line.setAttribute("x2", String(ink.x2 + xOffset));
                        line.setAttribute("y2", String(ink.y2 + yOffset));
                        line.setAttribute("stroke-width", String(ink.width));
                        line.setAttribute("stroke", ink.color);
                        line.setAttribute("stroke-linecap", "round");
                        line.setAttribute("data-variation-row",
                            row === 0 ? "ideal" : "seeded");
                        lines.push(line);
                    }
                }
            }
            variationPreview.replaceChildren(...lines);
            variationDownload.disabled = false;
            variationPreview.setAttribute("aria-label",
                `Top: three ideal ${character.value} glyphs; bottom: `
                    + `three seed ${seed} varied glyphs`);
            variationStatus.textContent =
                `Three ideal and three seeded varied ${character.value} `
                + "copies. Authored library unchanged; preview only.";
        } catch (error) {
            variationDownload.disabled = true;
            variationPreview.replaceChildren();
            variationPreview.setAttribute("aria-label", "Variation refused");
            variationStatus.textContent = String(error);
        }
    };
    // The candidate never replaces authored strokes until an explicit click.
    // SVG is a diagnostic over sampled native pen segments, not a curve asset.
    const renderSlantPreview = (): void => {
        slantPreview.replaceChildren();
        if (!active) {
            slantPreviewStatus.textContent =
                "Requires a compatible backend session";
            return;
        }
        const target = character.value;
        const authored = strokes.filter((item) =>
            item.grapheme === target);
        if (authored.length === 0) {
            slantPreviewStatus.textContent =
                "Author this exact glyph before previewing its slant";
            return;
        }
        try {
            if (authoredAngle.value === ""
                || authoredBaseline.value === "") {
                throw new Error("Choose an italic angle and baseline");
            }
            if (authored.length > 128) {
                throw new Error("Preview limited to 128 authored segments");
            }
            const angle = Number(authoredAngle.value);
            const baseline = Number(authoredBaseline.value);
            const shear = italicizeGlyphStrokes;
            const candidate = shear(strokes, target, angle, baseline);
            const chosen = candidate.filter((item) =>
                item.grapheme === target);
            const lines = chosen.map((item) => {
                const line = document.createElementNS(SVG_NS, "line");
                for (const axis of ["x1", "y1", "x2", "y2"] as const) {
                    line.setAttribute(axis, String(item[axis]));
                }
                line.setAttribute("stroke", item.color);
                line.setAttribute("stroke-width", String(item.width));
                line.setAttribute("stroke-linecap", "round");
                return line;
            });
            slantPreview.replaceChildren(...lines);
            slantPreviewStatus.textContent = `${lines.length} proposed `
                + "sampled pen segments; preview only, draft unchanged";
        } catch (error) {
            slantPreviewStatus.textContent = String(error);
        }
    };
    const renderHeightPreview = (): void => {
        heightPreview.replaceChildren();
        if (!active) {
            heightPreviewStatus.textContent =
                "Requires a compatible backend session";
            return;
        }
        const target = character.value;
        const authored = strokes.filter((item) =>
            item.grapheme === target);
        if (authored.length === 0) {
            heightPreviewStatus.textContent =
                "Author this exact glyph before previewing height";
            return;
        }
        try {
            if (heightPercent.value === ""
                || heightBaseline.value === "") {
                throw new Error("Choose a height percent and baseline");
            }
            if (authored.length > 128) {
                throw new Error("Preview limited to 128 authored segments");
            }
            const candidate = scaleGlyphHeight(strokes, target,
                Number(heightPercent.value), Number(heightBaseline.value));
            const lines = candidate.filter((item) =>
                item.grapheme === target).map((item) => {
                const line = document.createElementNS(SVG_NS, "line");
                for (const axis of ["x1", "y1", "x2", "y2"] as const) {
                    line.setAttribute(axis, String(item[axis]));
                }
                line.setAttribute("stroke", item.color);
                line.setAttribute("stroke-width", String(item.width));
                line.setAttribute("stroke-linecap", "round");
                return line;
            });
            heightPreview.replaceChildren(...lines);
            heightPreviewStatus.textContent = `${lines.length} proposed `
                + "height-scaled segments; preview only, draft unchanged";
        } catch (error) {
            heightPreviewStatus.textContent = String(error);
        }
    };
    const renderWidthPreview = (): void => {
        widthPreview.replaceChildren();
        if (!active) {
            widthPreviewStatus.textContent =
                "Requires a compatible backend session";
            return;
        }
        const target = character.value;
        const authored = strokes.filter((item) =>
            item.grapheme === target);
        if (authored.length === 0) {
            widthPreviewStatus.textContent =
                "Author this exact glyph before previewing width";
            return;
        }
        try {
            if (widthPercent.value === ""
                || widthAnchor.value === "") {
                throw new Error("Choose a width percent and anchor");
            }
            if (authored.length > 128) {
                throw new Error("Preview limited to 128 authored segments");
            }
            const candidate = scaleGlyphWidth(strokes, target,
                Number(widthPercent.value), Number(widthAnchor.value));
            const lines = candidate.filter((item) =>
                item.grapheme === target).map((item) => {
                const line = document.createElementNS(SVG_NS, "line");
                for (const axis of ["x1", "y1", "x2", "y2"] as const) {
                    line.setAttribute(axis, String(item[axis]));
                }
                line.setAttribute("stroke", item.color);
                line.setAttribute("stroke-width", String(item.width));
                line.setAttribute("stroke-linecap", "round");
                return line;
            });
            widthPreview.replaceChildren(...lines);
            widthPreviewStatus.textContent = `${lines.length} proposed `
                + "width-scaled segments; preview only, draft unchanged";
        } catch (error) {
            widthPreviewStatus.textContent = String(error);
        }
    };
    const renderInkPreview = (): void => {
        inkPreview.replaceChildren();
        if (!active) {
            inkPreviewStatus.textContent =
                "Requires a compatible backend session";
            return;
        }
        const target = character.value;
        const authored = strokes.filter((item) =>
            item.grapheme === target);
        if (authored.length === 0) {
            inkPreviewStatus.textContent =
                "Author this exact glyph before previewing ink thickness";
            return;
        }
        try {
            if (inkPercent.value === "") {
                throw new Error("Choose an ink percent");
            }
            if (authored.length > 128) {
                throw new Error("Preview limited to 128 authored segments");
            }
            const candidate = scaleGlyphInk(strokes, target,
                Number(inkPercent.value));
            const lines = candidate.filter((item) =>
                item.grapheme === target).map((item) => {
                const line = document.createElementNS(SVG_NS, "line");
                for (const axis of ["x1", "y1", "x2", "y2"] as const) {
                    line.setAttribute(axis, String(item[axis]));
                }
                line.setAttribute("stroke", item.color);
                line.setAttribute("stroke-width", String(item.width));
                line.setAttribute("stroke-linecap", "round");
                return line;
            });
            inkPreview.replaceChildren(...lines);
            inkPreviewStatus.textContent = `${lines.length} proposed `
                + "thickness-scaled segments; preview only, draft unchanged";
        } catch (error) {
            inkPreviewStatus.textContent = String(error);
        }
    };
    const renderRotatePreview = (): void => {
        rotatePreview.replaceChildren();
        if (!active) {
            rotatePreviewStatus.textContent =
                "Requires a compatible backend session";
            return;
        }
        const target = character.value;
        const authored = strokes.filter((item) =>
            item.grapheme === target);
        if (authored.length === 0) {
            rotatePreviewStatus.textContent =
                "Author this exact glyph before previewing rotation";
            return;
        }
        try {
            if (rotateAngle.value === "" || rotatePivotX.value === ""
                || rotatePivotY.value === "") {
                throw new Error("Choose angle and both rotation pivots");
            }
            if (authored.length > 128) {
                throw new Error("Preview limited to 128 authored segments");
            }
            const candidate = rotateGlyphStrokes(strokes, target,
                Number(rotateAngle.value), Number(rotatePivotX.value),
                Number(rotatePivotY.value));
            const lines = candidate.filter((item) =>
                item.grapheme === target).map((item) => {
                const line = document.createElementNS(SVG_NS, "line");
                for (const axis of ["x1", "y1", "x2", "y2"] as const) {
                    line.setAttribute(axis, String(item[axis]));
                }
                line.setAttribute("stroke", item.color);
                line.setAttribute("stroke-width", String(item.width));
                line.setAttribute("stroke-linecap", "round");
                return line;
            });
            rotatePreview.replaceChildren(...lines);
            rotatePreviewStatus.textContent = `${lines.length} proposed `
                + "rotated segments; preview only, draft unchanged";
        } catch (error) {
            rotatePreviewStatus.textContent = String(error);
        }
    };
    const renderMovePreview = (): void => {
        movePreview.replaceChildren();
        if (!active) {
            movePreviewStatus.textContent =
                "Requires a compatible backend session";
            return;
        }
        const target = character.value;
        const authored = strokes.filter((item) =>
            item.grapheme === target);
        if (authored.length === 0) {
            movePreviewStatus.textContent =
                "Author this exact glyph before previewing its position";
            return;
        }
        try {
            if (moveX.value === "" || moveY.value === "") {
                throw new Error("Choose horizontal and vertical offsets");
            }
            if (authored.length > 128) {
                throw new Error("Preview limited to 128 authored segments");
            }
            const candidate = translateGlyphStrokes(strokes, target,
                Number(moveX.value), Number(moveY.value));
            const lines = candidate.filter((item) =>
                item.grapheme === target).map((item) => {
                const line = document.createElementNS(SVG_NS, "line");
                for (const axis of ["x1", "y1", "x2", "y2"] as const) {
                    line.setAttribute(axis, String(item[axis]));
                }
                line.setAttribute("stroke", item.color);
                line.setAttribute("stroke-width", String(item.width));
                line.setAttribute("stroke-linecap", "round");
                return line;
            });
            movePreview.replaceChildren(...lines);
            movePreviewStatus.textContent = `${lines.length} proposed `
                + "positioned segments; preview only, draft unchanged";
        } catch (error) {
            movePreviewStatus.textContent = String(error);
        }
    };
    const render = (): void => {
        const missing = active ? missingBilingualBaseline(strokes) : [];
        focusMissing.disabled = !active || drawing !== null
            || missing.length === 0;
        if (!active) {
            baselineCoverage.textContent =
                "Bilingual coverage requires a compatible backend session";
        } else {
            const count = BILINGUAL_KEYS.length - missing.length;
            const display = (glyph: string): string =>
                glyph === glyph.normalize("NFC")
                    ? glyph : `${glyph.normalize("NFC")} (NFD)`;
            baselineCoverage.textContent = `Bilingual text baseline: `
                + `${count}/${BILINGUAL_KEYS.length} exact glyphs; `
                + (missing.length === 0 ? "complete" :
                    `${missing.length} missing; first: `
                    + missing.slice(0, 8).map(display).join(", ")
                    + (missing.length > 8 ? ", …" : ""));
        }
        if (transformHistory !== null) {
            const expected = transformHistory[transformHistory.side];
            if (strokes.length === 0
                || encodeGlyphLibrary(strokes) !== expected) {
                transformHistory = null;
            }
        }
        const lastKind = transformHistory?.grapheme === character.value
            ? transformHistory.kind : null;
        const transformLabel = lastKind === "ink"
            ? "glyph pen thickness"
            : lastKind === null ? "glyph transform" : `glyph ${lastKind}`;
        undoTransform.textContent = `Undo last ${transformLabel}`;
        redoTransform.textContent = `Redo last ${transformLabel}`;
        undoTransform.disabled = !active || drawing !== null
            || transformHistory?.side !== "after"
            || transformHistory.grapheme !== character.value;
        redoTransform.disabled = !active || drawing !== null
            || transformHistory?.side !== "before"
            || transformHistory.grapheme !== character.value;
        renderIdeal();
        const shown = strokes.flatMap((stroke, index) => {
            return stroke.grapheme === character.value
                ? [{ stroke, index }] : [];
        });
        if (!shown.some(({ index }) => index === selectedIndex)) {
            selectedIndex = null;
        }
        const paths = exactGrapheme(character.value)
            ? candidatePenPaths(strokes, character.value) : [];
        const pathNumbers = new Array<number>(shown.length);
        for (const [pathIndex, path] of paths.entries()) {
            for (let i = path.first - 1; i < path.last; i += 1) {
                pathNumbers[i] = pathIndex + 1;
            }
        }
        if (!active) {
            pathsStatus.textContent = "Requires a compatible backend session";
        } else if (paths.length === 0) {
            pathsStatus.textContent = "No authored pen paths for this glyph";
        } else {
            const details = paths.slice(0, 8).map((path, index) => {
                const closed = path.first < path.last
                    && path.start.x === path.end.x
                    && path.start.y === path.end.y;
                return `Path ${index + 1}: strokes `
                    + `${path.first}–${path.last}, `
                    + `(${path.start.x}, ${path.start.y}) → `
                    + `(${path.end.x}, ${path.end.y}), `
                    + `${path.width} µm ${path.color}`
                    + (closed ? " [closed loop]" : "");
            });
            const more = paths.length > 8
                ? `; ${paths.length - 8} more (inspect with Rust CLI)` : "";
            const pathWord = paths.length === 1 ? "path" : "paths";
            const lifts = paths.length - 1;
            const liftWord = lifts === 1 ? "lift" : "lifts";
            pathsStatus.textContent = `${shown.length} directed strokes, `
                + `${paths.length} candidate pen ${pathWord}, `
                + `${lifts} apparent pen ${liftWord}. `
                + details.join("; ") + more;
        }
        const options = shown.map(({ index }, position) => {
            const option = document.createElement("option");
            option.value = String(index);
            option.textContent = "Stroke " + (position + 1)
                + " of " + shown.length + ` (path ${pathNumbers[position]})`;
            return option;
        });
        const placeholder = document.createElement("option");
        placeholder.value = "";
        placeholder.textContent = "Choose a stroke to edit";
        selected.replaceChildren(placeholder, ...options);
        selected.value = selectedIndex === null ? "" : String(selectedIndex);
        selected.disabled = !active || shown.length === 0;
        slantAuthored.disabled = !active || shown.length === 0
            || drawing !== null;
        scaleHeight.disabled = !active || shown.length === 0
            || drawing !== null;
        scaleWidth.disabled = !active || shown.length === 0
            || drawing !== null;
        scaleInk.disabled = !active || shown.length === 0
            || drawing !== null;
        moveGlyph.disabled = !active || shown.length === 0
            || drawing !== null;
        rotateGlyph.disabled = !active || shown.length === 0
            || drawing !== null;
        update.disabled = !active || selectedIndex === null;
        remove.disabled = !active || selectedIndex === null;
        reverse.disabled = !active || selectedIndex === null;
        const chosenPosition = shown.findIndex(({ index }) => {
            return index === selectedIndex;
        }) + 1;
        const chosenPath = paths.find((path) => {
            return chosenPosition >= path.first
                && chosenPosition <= path.last;
        });
        reversePath.disabled = !active || !chosenPath
            || chosenPath.first === chosenPath.last;
        startPath.disabled = !active || !chosenPath
            || chosenPath.first === chosenPath.last
            || chosenPosition === chosenPath.first
            || chosenPath.start.x !== chosenPath.end.x
            || chosenPath.start.y !== chosenPath.end.y;
        const closureGap = chosenPath ?
            (chosenPath.end.x - chosenPath.start.x) ** 2
            + (chosenPath.end.y - chosenPath.start.y) ** 2 : 0;
        closePath.disabled = !active || !chosenPath
            || chosenPath.first === chosenPath.last || closureGap === 0
            || closureGap > 1200 * 1200 || strokes.length >= MAX_STROKES;
        translatePath.disabled = !active || !chosenPath;
        const pathIndex = paths.indexOf(chosenPath!);
        earlierPath.disabled = !active || pathIndex <= 0;
        laterPath.disabled = !active || pathIndex < 0
            || pathIndex >= paths.length - 1;
        earlier.disabled = !active || neighbor(-1) === null;
        later.disabled = !active || neighbor(1) === null;
        join.disabled = !active || neighbor(1) === null;
        const lines = shown.map(({ stroke, index }) => {
            const line = document.createElementNS(SVG_NS, "line");
            for (const axis of ["x1", "y1", "x2", "y2"] as const) {
                line.setAttribute(axis, String(stroke[axis]));
            }
            line.setAttribute("stroke", stroke.color);
            line.setAttribute("stroke-width", String(stroke.width));
            line.setAttribute("stroke-linecap", "round");
            if (index === selectedIndex) {
                line.setAttribute("data-selected", "true");
            }
            line.addEventListener("click", () => {
                if (active) choose(index);
            });
            line.addEventListener("pointerdown", (event) => {
                event.stopPropagation();
                if (active && event.button === 0) choose(index);
            });
            return line;
        });
        if (drawing !== null && drawing.grapheme === character.value) {
            const points = drawing.points.slice();
            const last = points.at(-1);
            if (last?.x !== drawing.last.x || last?.y !== drawing.last.y) {
                points.push(drawing.last);
            }
            if (points.length === 1) points.push(drawing.last);
            for (let i = 1; i < points.length; i += 1) {
                const first = points[i - 1];
                const second = points[i];
                if (first === undefined || second === undefined) continue;
                const draft = document.createElementNS(SVG_NS, "line");
                draft.setAttribute("x1", String(first.x));
                draft.setAttribute("y1", String(first.y));
                draft.setAttribute("x2", String(second.x));
                draft.setAttribute("y2", String(second.y));
                draft.setAttribute("stroke", drawing.color);
                draft.setAttribute("stroke-width", String(drawing.width));
                draft.setAttribute("stroke-linecap", "round");
                draft.setAttribute("stroke-dasharray", "150 100");
                draft.setAttribute("data-draft", "true");
                lines.push(draft);
            }
        }
        preview.replaceChildren(...lines);
        renderSlantPreview();
        renderHeightPreview();
        renderWidthPreview();
        renderInkPreview();
        renderMovePreview();
        renderRotatePreview();
        const label = `${shown.length} strokes for selected glyph`;
        preview.setAttribute("aria-label", label);
        renderVariation();
    };
    const choose = (index: number): void => {
        const stroke = strokes[index];
        if (!active || stroke?.grapheme !== character.value) return;
        selectedIndex = index;
        for (const axis of AXES) {
            numeric[axis].value = String(stroke[axis]);
            ranges[axis].value = String(stroke[axis]);
        }
        color.value = stroke.color;
        render();
    };
    const editable = (): Stroke => validateStroke({
        grapheme: character.value, color: color.value,
        x1: boundedInteger(numeric.x1.value, 3500, "x1"),
        y1: boundedInteger(numeric.y1.value, 4500, "y1"),
        x2: boundedInteger(numeric.x2.value, 3500, "x2"),
        y2: boundedInteger(numeric.y2.value, 4500, "y2"),
        width: boundedInteger(numeric.width.value, 1500, "width"),
    });

    for (const control of [authoredAngle, authoredBaseline]) {
        control.addEventListener("input", renderSlantPreview);
        control.addEventListener("change", renderSlantPreview);
    }
    for (const control of [heightPercent, heightBaseline]) {
        control.addEventListener("input", renderHeightPreview);
        control.addEventListener("change", renderHeightPreview);
    }
    for (const control of [widthPercent, widthAnchor]) {
        control.addEventListener("input", renderWidthPreview);
        control.addEventListener("change", renderWidthPreview);
    }
    for (const control of [rotateAngle, rotatePivotX, rotatePivotY]) {
        control.addEventListener("input", renderRotatePreview);
        control.addEventListener("change", renderRotatePreview);
    }
    for (const control of [moveX, moveY]) {
        control.addEventListener("input", renderMovePreview);
        control.addEventListener("change", renderMovePreview);
    }
    inkPercent.addEventListener("input", renderInkPreview);
    inkPercent.addEventListener("change", renderInkPreview);
    for (const control of [
        form, centerX, centerY, radius, color, accentShift, accentLift,
        italicAngle,
    ]) {
        control.addEventListener("input", renderIdeal);
        control.addEventListener("change", renderIdeal);
    }
    const variationInputs = [
        variationSeed, ...Object.values(variationControls),
    ];
    for (const control of variationInputs) {
        control.addEventListener("input", renderVariation);
        control.addEventListener("change", renderVariation);
    }
    character.addEventListener("input", () => {
        drawing = null;
        selectedIndex = null;
        render();
    });
    selected.addEventListener("change", () => {
        if (!active) return;
        if (!/^(0|[1-9][0-9]*)$/.test(selected.value)) {
            selectedIndex = null;
            render();
            return;
        }
        choose(Number(selected.value));
    });
    for (const axis of AXES) {
        numeric[axis].addEventListener("input", () => {
            ranges[axis].value = numeric[axis].value;
            if (axis === "width") renderIdeal();
        });
        ranges[axis].addEventListener("input", () => {
            numeric[axis].value = ranges[axis].value;
            if (axis === "width") renderIdeal();
        });
    }
    const pointerPoint = (event: PointerEvent): Point | null => {
        const box = preview.getBoundingClientRect();
        if (box.width <= 0 || box.height <= 0
            || !Number.isFinite(event.clientX)
            || !Number.isFinite(event.clientY)) return null;
        const x = Math.round(3500 * (event.clientX - box.left) / box.width);
        const y = Math.round(4500 * (event.clientY - box.top) / box.height);
        return {
            x: Math.max(0, Math.min(3500, x)),
            y: Math.max(0, Math.min(4500, y)),
        };
    };
    preview.addEventListener("pointerdown", (event) => {
        if (!active || drawing !== null || event.button !== 0
            || !event.isPrimary) return;
        const first = pointerPoint(event);
        if (first === null) return;
        try {
            if (!exactGrapheme(character.value)) {
                throw new Error("Choose exactly one Unicode grapheme to draw");
            }
            const width = boundedInteger(numeric.width.value, 1500, "width");
            if (!/^#[0-9a-fA-F]{6}$/.test(color.value)) {
                throw new Error("Ink color requires #RRGGBB");
            }
            preview.setPointerCapture(event.pointerId);
            drawing = {
                pointerId: event.pointerId,
                grapheme: character.value,
                color: color.value.toUpperCase(),
                width,
                points: [first], last: first, overflow: false,
            };
            render();
        } catch (error) {
            message(String(error));
        }
    });
    preview.addEventListener("pointermove", (event) => {
        if (!active || drawing?.pointerId !== event.pointerId) return;
        const point = pointerPoint(event);
        if (point === null) return;
        drawing.last = point;
        const sampled = drawing.points.at(-1);
        if (sampled !== undefined) {
            const dx = point.x - sampled.x;
            const dy = point.y - sampled.y;
            if (dx * dx + dy * dy >= 60 * 60) {
                if (drawing.points.length >= 129) {
                    drawing.overflow = true;
                } else {
                    drawing.points.push(point);
                }
            }
        }
        render();
    });
    preview.addEventListener("pointerup", (event) => {
        if (!active || drawing?.pointerId !== event.pointerId) return;
        const current = drawing;
        drawing = null;
        if (preview.hasPointerCapture(event.pointerId)) {
            preview.releasePointerCapture(event.pointerId);
        }
        try {
            if (current.overflow) {
                throw new Error("Trace exceeds 128 sampled line segments");
            }
            const last = pointerPoint(event) ?? current.last;
            const points = current.points.slice();
            const finalSample = points.at(-1);
            if (finalSample?.x !== last.x || finalSample?.y !== last.y) {
                points.push(last);
            }
            if (points.length < 2 || points.length > 129) {
                throw new Error("Drag across the preview to draw a segment");
            }
            const segments = points.slice(1).map((end, index) => {
                const first = points[index];
                if (first === undefined) {
                    throw new Error("Missing trace start point");
                }
                return validateStroke({
                    grapheme: current.grapheme,
                    x1: first.x, y1: first.y,
                    x2: end.x, y2: end.y,
                    width: current.width, color: current.color,
                });
            });
            const next = [...strokes, ...segments];
            encodeGlyphLibrary(next);
            strokes = next;
            selectedIndex = next.length - 1;
            message(strokes.length + " unsaved authored strokes");
        } catch (error) {
            message(String(error));
        }
        render();
    });
    preview.addEventListener("pointercancel", (event) => {
        if (drawing?.pointerId !== event.pointerId) return;
        drawing = null;
        if (preview.hasPointerCapture(event.pointerId)) {
            preview.releasePointerCapture(event.pointerId);
        }
        render();
    });
    construct.addEventListener("click", () => {
        if (!active) return;
        try {
            const chosen = selectedForm();
            const target = chosen.grapheme;
            if (character.value !== target) {
                throw new Error("Choose the matching geometric starter glyph");
            }
            const lines = starterFromInputs();
            const next = [
                ...strokes.filter((stroke) => stroke.grapheme !== target),
                ...lines,
            ];
            encodeGlyphLibrary(next);
            strokes = next;
            selectedIndex = null;
            render();
            message("Constructed " + lines.length + " sampled segments for "
                + target + "; curves are sampled into TSV");
        } catch (error) {
            message(String(error));
        }
    });
    const applyGlyphTransform = (
        next: Stroke[], target: string, kind: TransformKind,
    ): void => {
        const before = encodeGlyphLibrary(strokes);
        const after = encodeGlyphLibrary(next);
        strokes = next;
        // A 100% resize or zero-degree slant is an identity, not a new
        // edit. Retain any valid earlier recovery step on either side.
        if (before !== after) {
            transformHistory = {
                before, after, side: "after", grapheme: target, kind,
            };
        }
        selectedIndex = null;
        render();
    };
    slantAuthored.addEventListener("click", () => {
        if (!active) return;
        try {
            if (drawing !== null) {
                throw new Error("Finish the pen trace before italic edit");
            }
            if (authoredAngle.value === ""
                || authoredBaseline.value === "") {
                throw new Error("Choose an italic angle and baseline");
            }
            const angle = Number(authoredAngle.value);
            const pivot = Number(authoredBaseline.value);
            const target = character.value;
            const next = italicizeGlyphStrokes(
                strokes, target, angle, pivot,
            );
            applyGlyphTransform(next, target, "slant");
            message(`Slanted all authored ${character.value} ink; unsaved`);
        } catch (error) {
            message(String(error));
        }
    });
    scaleHeight.addEventListener("click", () => {
        if (!active) return;
        try {
            if (drawing !== null) {
                throw new Error("Finish the pen trace before height edit");
            }
            if (heightPercent.value === ""
                || heightBaseline.value === "") {
                throw new Error("Choose a height percent and baseline");
            }
            const target = character.value;
            const next = scaleGlyphHeight(strokes, target,
                Number(heightPercent.value), Number(heightBaseline.value));
            applyGlyphTransform(next, target, "height");
            message(`Resized all authored ${target} height; unsaved`);
        } catch (error) {
            message(String(error));
        }
    });
    scaleWidth.addEventListener("click", () => {
        if (!active) return;
        try {
            if (drawing !== null) {
                throw new Error("Finish the pen trace before width edit");
            }
            if (widthPercent.value === "" || widthAnchor.value === "") {
                throw new Error("Choose a width percent and anchor");
            }
            const target = character.value;
            const next = scaleGlyphWidth(strokes, target,
                Number(widthPercent.value), Number(widthAnchor.value));
            applyGlyphTransform(next, target, "width");
            message(`Resized all authored ${target} width; unsaved`);
        } catch (error) {
            message(String(error));
        }
    });
    rotateGlyph.addEventListener("click", () => {
        if (!active) return;
        try {
            if (drawing !== null) {
                throw new Error("Finish the pen trace before rotation");
            }
            if (rotateAngle.value === "" || rotatePivotX.value === ""
                || rotatePivotY.value === "") {
                throw new Error("Choose angle and both rotation pivots");
            }
            const target = character.value;
            const next = rotateGlyphStrokes(strokes, target,
                Number(rotateAngle.value), Number(rotatePivotX.value),
                Number(rotatePivotY.value));
            applyGlyphTransform(next, target, "rotation");
            message(`Rotated all authored ${target} strokes; unsaved`);
        } catch (error) {
            message(String(error));
        }
    });
    moveGlyph.addEventListener("click", () => {
        if (!active) return;
        try {
            if (drawing !== null) {
                throw new Error("Finish the pen trace before position edit");
            }
            if (moveX.value === "" || moveY.value === "") {
                throw new Error("Choose horizontal and vertical offsets");
            }
            const target = character.value;
            const next = translateGlyphStrokes(strokes, target,
                Number(moveX.value), Number(moveY.value));
            applyGlyphTransform(next, target, "move");
            message(`Moved all authored ${target} strokes; unsaved`);
        } catch (error) {
            message(String(error));
        }
    });
    scaleInk.addEventListener("click", () => {
        if (!active) return;
        try {
            if (drawing !== null) {
                throw new Error("Finish the pen trace before ink edit");
            }
            if (inkPercent.value === "") {
                throw new Error("Choose an ink percent");
            }
            const target = character.value;
            const next = scaleGlyphInk(strokes, target,
                Number(inkPercent.value));
            applyGlyphTransform(next, target, "ink");
            message(`Resized all authored ${target} pen thickness; unsaved`);
        } catch (error) {
            message(String(error));
        }
    });
    const restoreTransform = (side: "before" | "after"): void => {
        if (!active || drawing !== null || transformHistory === null) return;
        const history = transformHistory;
        const requiredSide = side === "before" ? "after" : "before";
        if (history.side !== requiredSide
            || history.grapheme !== character.value) return;
        try {
            if (encodeGlyphLibrary(strokes) !== history[requiredSide]) {
                transformHistory = null;
                throw new Error(
                    "Cannot restore transform after another ink edit");
            }
            strokes = parseGlyphLibrary(history[side]);
            history.side = side;
            selectedIndex = null;
            render();
            const label = history.kind === "ink"
                ? "pen thickness" : history.kind;
            message(side === "before"
                ? `Undid last glyph ${label}; unsaved`
                : `Redid last glyph ${label}; unsaved`);
        } catch (error) {
            message(String(error));
            render();
        }
    };
    undoTransform.addEventListener("click", () => restoreTransform("before"));
    redoTransform.addEventListener("click", () => restoreTransform("after"));
    seedLowercase.addEventListener("click", () => {
        if (!active) return;
        try {
            if (drawing !== null) {
                throw new Error("Finish the current pen trace before seeding");
            }
            const authored = new Set(strokes.map((stroke) => stroke.grapheme));
            const missing = [..."abcdefghijklmnopqrstuvwxyz"].filter(
                (glyph) => !authored.has(glyph));
            if (missing.length === 0) {
                message("All 26 lowercase glyphs already have authored ink");
                return;
            }
            const x = Number(centerX.value);
            const y = Number(centerY.value);
            const r = Number(radius.value);
            const pen = Number(numeric.width.value);
            const added = missing.flatMap((glyph) =>
                geometricStarter(glyph, x, y, r, pen, color.value));
            const next = [...strokes, ...added];
            encodeGlyphLibrary(next);
            strokes = next;
            selectedIndex = null;
            render();
            message(`Filled ${missing.length} missing lowercase templates (`
                + `${added.length} sampled strokes); unsaved draft`);
        } catch (error) {
            message(String(error));
        }
    });
    focusMissing.addEventListener("click", () => {
        if (!active || drawing !== null) return;
        const missing = missingBilingualBaseline(strokes);
        const previous = BILINGUAL_KEYS.indexOf(character.value);
        let next = missing[0];
        for (const glyph of missing) {
            if (BILINGUAL_KEYS.indexOf(glyph) > previous) {
                next = glyph;
                break;
            }
        }
        if (next === undefined) return;
        character.value = next;
        form.value = next;
        selectedIndex = null;
        render();
        const display = next.normalize("NFC") === next ? next
            : `${next.normalize("NFC")} (NFD)`;
        message(`Inspecting missing glyph ${display}; draft unchanged`);
    });
    seedBaseline.addEventListener("click", () => {
        if (!active) return;
        try {
            if (drawing !== null) {
                throw new Error("Finish the current pen trace before seeding");
            }
            const next = fillMissingBilingualBaseline(strokes,
                Number(centerX.value), Number(centerY.value),
                Number(radius.value), Number(numeric.width.value),
                color.value, { dx: Number(accentShift.value),
                    lift: Number(accentLift.value) });
            if (next === strokes) {
                message("All 114 bilingual text glyphs already have ink");
                return;
            }
            const count = new Set(next.map((line) => line.grapheme)).size
                - new Set(strokes.map((line) => line.grapheme)).size;
            const added = next.length - strokes.length;
            strokes = next;
            selectedIndex = null;
            render();
            message(`Filled ${count} missing bilingual baseline glyphs (`
                + `${added} sampled strokes); unsaved draft`);
        } catch (error) {
            message(String(error));
        }
    });
    seedPunctuation.addEventListener("click", () => {
        if (!active) return;
        try {
            if (drawing !== null) {
                throw new Error("Finish the current pen trace before seeding");
            }
            const glyphs = [".", ",", "!", "¡", "?", "¿"];
            const authored = new Set(strokes.map((stroke) => stroke.grapheme));
            const missing = glyphs.filter((g) => !authored.has(g));
            if (missing.length === 0) {
                message("All six basic punctuation glyphs have authored ink");
                return;
            }
            const added = missing.flatMap((g) => geometricStarter(g,
                Number(centerX.value), Number(centerY.value),
                Number(radius.value), Number(numeric.width.value),
                color.value));
            const next = [...strokes, ...added];
            encodeGlyphLibrary(next);
            strokes = next;
            selectedIndex = null;
            render();
            message(`Filled ${missing.length} missing punctuation glyphs (`
                + `${added.length} sampled strokes); unsaved draft`);
        } catch (error) {
            message(String(error));
        }
    });
    seedQuotes.addEventListener("click", () => {
        if (!active) return;
        try {
            if (drawing !== null) {
                throw new Error("Finish the current pen trace before seeding");
            }
            const glyphs = ["«", "»", "‘", "’", "“", "”"];
            const authored = new Set(strokes.map((stroke) => stroke.grapheme));
            const missing = glyphs.filter((g) => !authored.has(g));
            if (missing.length === 0) {
                message("All six quotation glyphs already have authored ink");
                return;
            }
            const added = missing.flatMap((g) => geometricStarter(g,
                Number(centerX.value), Number(centerY.value),
                Number(radius.value), Number(numeric.width.value),
                color.value));
            const next = [...strokes, ...added];
            encodeGlyphLibrary(next);
            strokes = next;
            selectedIndex = null;
            render();
            message(`Filled ${missing.length} missing quotation glyphs (`
                + `${added.length} sampled strokes); unsaved draft`);
        } catch (error) {
            message(String(error));
        }
    });
    seedMath.addEventListener("click", () => {
        if (!active) return;
        try {
            if (drawing !== null) {
                throw new Error("Finish the current pen trace before seeding");
            }
            const glyphs: readonly string[] = MATH_GLYPHS;
            const authored = new Set(strokes.map((stroke) => stroke.grapheme));
            const missing = glyphs.filter((g) => !authored.has(g));
            if (missing.length === 0) {
                message(`All ${MATH_GLYPHS.length} math glyphs`
                    + " have authored ink");
                return;
            }
            const added = missing.flatMap((g) => geometricStarter(g,
                Number(centerX.value), Number(centerY.value),
                Number(radius.value), Number(numeric.width.value),
                color.value));
            const next = [...strokes, ...added];
            encodeGlyphLibrary(next);
            strokes = next;
            selectedIndex = null;
            render();
            message(`Filled ${missing.length} missing math glyphs (`
                + `${added.length} exact strokes); unsaved draft`);
        } catch (error) {
            message(String(error));
        }
    });
    seedDigits.addEventListener("click", () => {
        if (!active) return;
        try {
            if (drawing !== null) {
                throw new Error("Finish the current pen trace before seeding");
            }
            const authored = new Set(strokes.map((stroke) => stroke.grapheme));
            const missing = [..."0123456789"].filter((g) => !authored.has(g));
            if (missing.length === 0) {
                message("All ten numeral glyphs already have authored ink");
                return;
            }
            const added = missing.flatMap((g) => geometricStarter(g,
                Number(centerX.value), Number(centerY.value),
                Number(radius.value), Number(numeric.width.value),
                color.value));
            const next = [...strokes, ...added];
            encodeGlyphLibrary(next);
            strokes = next;
            selectedIndex = null;
            render();
            message(`Filled ${missing.length} missing numeral glyphs (`
                + `${added.length} sampled strokes); unsaved draft`);
        } catch (error) {
            message(String(error));
        }
    });
    seedCapitals.addEventListener("click", () => {
        if (!active) return;
        try {
            if (drawing !== null) {
                throw new Error("Finish the current pen trace before seeding");
            }
            const authored = new Set(strokes.map((stroke) => stroke.grapheme));
            const missing = [..."ABCDEFGHIJKLMNOPQRSTUVWXYZ"]
                .filter((g) => !authored.has(g));
            if (missing.length === 0) {
                message("All 26 uppercase glyphs already have authored ink");
                return;
            }
            const added = missing.flatMap((g) => geometricStarter(g,
                Number(centerX.value), Number(centerY.value),
                Number(radius.value), Number(numeric.width.value),
                color.value));
            const next = [...strokes, ...added];
            encodeGlyphLibrary(next);
            strokes = next;
            selectedIndex = null;
            render();
            message(`Filled ${missing.length} missing uppercase glyphs (`
                + `${added.length} sampled strokes); unsaved draft`);
        } catch (error) {
            message(String(error));
        }
    });
    seedTextMarks.addEventListener("click", () => {
        if (!active) return;
        try {
            if (drawing !== null) {
                throw new Error("Finish the current pen trace before seeding");
            }
            const authored = new Set(strokes.map((stroke) => stroke.grapheme));
            const missing = TEXT_MARKS.filter((g) => !authored.has(g));
            if (missing.length === 0) {
                message("All eleven text punctuation glyphs are authored");
                return;
            }
            const added = missing.flatMap((g) => geometricStarter(g,
                Number(centerX.value), Number(centerY.value),
                Number(radius.value), Number(numeric.width.value),
                color.value));
            const next = [...strokes, ...added];
            encodeGlyphLibrary(next);
            strokes = next;
            selectedIndex = null;
            render();
            message(`Filled ${missing.length} missing text punctuation `
                + `glyphs (${added.length} sampled strokes); unsaved draft`);
        } catch (error) {
            message(String(error));
        }
    });
    const fillSpanishAccents = (
        normalization: "NFC" | "NFD",
        letterCase: "lowercase" | "uppercase" = "lowercase",
    ): void => {
        if (!active) return;
        try {
            if (drawing !== null) {
                throw new Error("Finish the current pen trace before accents");
            }
            const targets: [string, string][] = [
                ["a", "á"], ["e", "é"], ["i", "í"], ["o", "ó"],
                ["u", "ú"], ["n", "ñ"], ["u", "ü"],
            ];
            const authored = new Set(strokes.map((stroke) => stroke.grapheme));
            const missing = targets.map(([base, target]) => {
                const casedBase = letterCase === "uppercase"
                    ? base.toUpperCase() : base;
                const casedTarget = letterCase === "uppercase"
                    ? target.toUpperCase() : target;
                const normalized = casedTarget.normalize(normalization);
                return [casedBase, normalized] as const;
            }).filter(([, target]) => !authored.has(target));
            if (missing.length === 0) {
                message(`All seven Spanish ${letterCase} ${normalization}`
                    + " accents are authored");
                return;
            }
            const withoutBases = missing.flatMap(([base]) =>
                authored.has(base) ? [] : [base]);
            if (withoutBases.length > 0) {
                throw new Error(`Author ${letterCase} base glyphs first: `
                    + [...new Set(withoutBases)].join(", "));
            }
            const shift = Number(accentShift.value);
            const lift = Number(accentLift.value);
            const added = missing.flatMap(([base, target]) =>
                composeAccentedGlyph(strokes, base, target, color.value,
                    { dx: shift, lift }));
            const next = [...strokes, ...added];
            encodeGlyphLibrary(next);
            strokes = next;
            selectedIndex = null;
            render();
            const casing = letterCase === "uppercase" ? "uppercase " : "";
            message(`Filled ${missing.length} missing Spanish `
                + `${casing}${normalization} accents `
                + `(${added.length} sampled `
                + "strokes); unsaved draft");
        } catch (error) {
            message(String(error));
        }
    };
    seedSpanish.addEventListener("click", () => fillSpanishAccents("NFC"));
    seedSpanishNfd.addEventListener("click", () => fillSpanishAccents("NFD"));
    seedUpperSpanish.addEventListener("click", () =>
        fillSpanishAccents("NFC", "uppercase"));
    seedUpperSpanishNfd.addEventListener("click", () =>
        fillSpanishAccents("NFD", "uppercase"));
    copy.addEventListener("click", () => {
        if (!active) return;
        try {
            const target = character.value;
            const origin = copyFrom.value;
            if (!exactGrapheme(target) || !exactGrapheme(origin)
                || target === origin) {
                throw new Error("Copy needs two different exact graphemes");
            }
            const templates = strokes.filter((stroke) => {
                return stroke.grapheme === origin;
            });
            if (templates.length === 0) {
                throw new Error("Source grapheme has no authored strokes");
            }
            const next = [
                ...strokes.filter((stroke) => stroke.grapheme !== target),
                ...templates.map((stroke) => ({ ...stroke, grapheme: target })),
            ];
            encodeGlyphLibrary(next);
            strokes = next;
            selectedIndex = null;
            render();
            message("Copied " + templates.length
                + " strokes to the exact selected grapheme (unsaved)");
        } catch (error) {
            message(String(error));
        }
    });
    compose.addEventListener("click", () => {
        if (!active) return;
        try {
            const target = character.value;
            const sourceGlyph = copyFrom.value;
            const lines = composeAccentedGlyph(
                strokes, sourceGlyph, target, color.value, {
                    dx: Number(accentShift.value),
                    lift: Number(accentLift.value),
                });
            const next = [
                ...strokes.filter((stroke) => stroke.grapheme !== target),
                ...lines,
            ];
            encodeGlyphLibrary(next);
            strokes = next;
            selectedIndex = null;
            render();
            message("Composed " + lines.length + " ordered strokes for "
                + target + "; base preserved (unsaved)");
        } catch (error) {
            message(String(error));
        }
    });
    add.addEventListener("click", () => {
        if (!active) return;
        try {
            if (strokes.length === MAX_STROKES) {
                throw new Error("Native glyph limit is 4096 line segments");
            }
            const next = [...strokes, editable()];
            encodeGlyphLibrary(next);
            strokes = next;
            selectedIndex = next.length - 1;
            render();
            message(`${strokes.length} unsaved authored strokes`);
        } catch (error) {
            message(String(error));
        }
    });
    update.addEventListener("click", () => {
        if (!active || selectedIndex === null) return;
        try {
            const changed = editable();
            const index = selectedIndex;
            if (strokes[index]?.grapheme !== character.value) return;
            const next = strokes.slice();
            next[index] = changed;
            encodeGlyphLibrary(next);
            strokes = next;
            render();
            message("Updated selected stroke; " + strokes.length
                + " unsaved strokes");
        } catch (error) {
            message(String(error));
        }
    });
    reverse.addEventListener("click", () => {
        if (!active || selectedIndex === null) return;
        const index = selectedIndex;
        const existing = strokes[index];
        if (existing?.grapheme !== character.value) return;
        const next = strokes.slice();
        next[index] = {
            ...existing, x1: existing.x2, y1: existing.y2,
            x2: existing.x1, y2: existing.y1,
        };
        encodeGlyphLibrary(next);
        strokes = next;
        choose(index);
        message("Reversed selected pen stroke; unsaved");
    });
    reversePath.addEventListener("click", () => {
        if (!active || selectedIndex === null) return;
        try {
            const glyph = character.value;
            const changed = reverseCandidatePath(strokes, glyph, selectedIndex);
            encodeGlyphLibrary(changed.strokes);
            strokes = changed.strokes;
            choose(changed.selectedIndex);
            message("Reversed selected continuous pen path; unsaved");
        } catch (error) {
            message(String(error));
        }
    });
    startPath.addEventListener("click", () => {
        if (!active || selectedIndex === null) return;
        try {
            const changed = rotateClosedCandidatePath(
                strokes, character.value, selectedIndex,
            );
            strokes = changed.strokes;
            choose(changed.selectedIndex);
            message("Changed closed pen loop starting stroke; unsaved");
        } catch (error) {
            message(String(error));
        }
    });
    closePath.addEventListener("click", () => {
        if (!active || selectedIndex === null) return;
        try {
            const glyph = character.value;
            const changed = closeCandidatePath(strokes, glyph, selectedIndex);
            strokes = changed.strokes;
            choose(changed.selectedIndex);
            message("Closed selected pen path with a short bridge; unsaved");
        } catch (error) {
            message(String(error));
        }
    });
    translatePath.addEventListener("click", () => {
        if (!active || selectedIndex === null) return;
        try {
            const signed = (raw: string, maximum: number): number => {
                if (!/^-?[0-9]+$/.test(raw)) {
                    throw new Error("Path offset needs a whole micrometre");
                }
                const value = Number(raw);
                if (!Number.isSafeInteger(value)
                    || Math.abs(value) > maximum) {
                    throw new Error("Path offset outside native bounds");
                }
                return value;
            };
            const changed = translateCandidatePath(
                strokes, character.value, selectedIndex,
                signed(pathDx.value, 3500), signed(pathDy.value, 4500),
            );
            strokes = changed.strokes;
            choose(changed.selectedIndex);
            message("Translated selected continuous pen path; unsaved");
        } catch (error) {
            message(String(error));
        }
    });
    const movePath = (direction: -1 | 1): void => {
        if (!active || selectedIndex === null) return;
        try {
            const glyph = character.value;
            const index = selectedIndex;
            const changed = moveCandidatePath(
                strokes, glyph, index, direction,
            );
            encodeGlyphLibrary(changed.strokes);
            strokes = changed.strokes;
            choose(changed.selectedIndex);
            message("Reordered selected continuous pen path; unsaved");
        } catch (error) {
            message(String(error));
        }
    };
    earlierPath.addEventListener("click", () => movePath(-1));
    laterPath.addEventListener("click", () => movePath(1));
    const move = (offset: -1 | 1): void => {
        if (!active || selectedIndex === null) return;
        const index = selectedIndex;
        const target = neighbor(offset);
        if (target === null) return;
        const next = strokes.slice();
        [next[index], next[target]] = [next[target]!, next[index]!];
        encodeGlyphLibrary(next);
        strokes = next;
        choose(target);
        message("Changed pen stroke order; unsaved");
    };
    earlier.addEventListener("click", () => move(-1));
    later.addEventListener("click", () => move(1));
    join.addEventListener("click", () => {
        if (!active || selectedIndex === null) return;
        try {
            const first = strokes[selectedIndex];
            const target = neighbor(1);
            const second = target === null ? undefined : strokes[target];
            if (!first || !second || first.grapheme !== second.grapheme) {
                throw new Error("Joining needs the next exact glyph stroke");
            }
            if (first.width !== second.width
                || first.color.toUpperCase() !== second.color.toUpperCase()) {
                throw new Error("Joining needs matching pen width and RGB");
            }
            const dx = first.x2 - second.x1;
            const dy = first.y2 - second.y1;
            const distanceSquared = dx * dx + dy * dy;
            if (distanceSquared === 0) {
                throw new Error("Strokes are already continuous");
            }
            if (distanceSquared > 1200 * 1200) {
                throw new Error("Stroke gap exceeds 1200 µm join limit");
            }
            if (strokes.length === MAX_STROKES) {
                throw new Error("Joining exceeds 4096 authored strokes");
            }
            const bridge: Stroke = {
                grapheme: first.grapheme,
                x1: first.x2, y1: first.y2,
                x2: second.x1, y2: second.y1,
                width: first.width, color: first.color,
            };
            const next = strokes.slice();
            next.splice(selectedIndex + 1, 0, bridge);
            encodeGlyphLibrary(next);
            const joinedIndex = selectedIndex + 1;
            strokes = next;
            choose(joinedIndex);
            message("Connected selected pen strokes; unsaved");
        } catch (error) {
            message(String(error));
        }
    });
    remove.addEventListener("click", () => {
        if (!active || selectedIndex === null) return;
        if (strokes[selectedIndex]?.grapheme !== character.value) return;
        strokes.splice(selectedIndex, 1);
        selectedIndex = null;
        render();
        message(strokes.length + " unsaved authored strokes");
    });
    source.addEventListener("change", async () => {
        if (!active) return;
        const file = source.files?.[0];
        if (file === undefined) return;
        generation += 1;
        const expected = generation;
        source.value = "";
        try {
            if (file.size > MAX_BYTES) {
                throw new Error("Glyph file exceeds 512 KiB");
            }
            const bytes = await file.arrayBuffer();
            if (!active || generation !== expected) return;
            // Keep a leading BOM visible to the strict native TSV header
            // check. TextDecoder normally drops it and would silently admit
            // a file that Rust rejects as a different source format.
            const decoder = new TextDecoder("utf-8", {
                fatal: true, ignoreBOM: true,
            });
            const text = decoder.decode(bytes);
            const checked = parseGlyphLibrary(text);
            strokes = checked;
            transformHistory = null;
            drawing = null;
            selectedIndex = null;
            render();
            message(`Loaded ${checked.length} authored strokes in memory`);
        } catch (error) {
            if (active && generation === expected) message(String(error));
        }
    });
    save.addEventListener("click", () => {
        if (!active) return;
        try {
            download(encodeGlyphLibrary(strokes), "my-glyph-strokes.tsv");
            message("Glyph TSV downloaded; Rust validates it before export");
        } catch (error) {
            message(String(error));
        }
    });
    variationDownload.addEventListener("click", () => {
        if (!active) return;
        try {
            const { seed, settings } = readVariation();
            const sample = boundedInteger(variationSample.value, 3,
                "variation sample");
            if (sample < 1) {
                throw new Error("Choose varied sample 1, 2 or 3");
            }
            const changed = varyGlyphStrokes(
                strokes, character.value, seed, sample - 1, settings,
            );
            let position = 0;
            const candidate = strokes.map((item) =>
                item.grapheme === character.value
                    ? changed[position++]! : item,
            );
            const bytes = encodeGlyphLibrary(candidate);
            download(bytes, "my-varied-glyph-strokes.tsv");
            message(`Varied sample ${sample} downloaded as a separate TSV; `
                + "authored draft unchanged, Rust validates before export");
        } catch (error) {
            message(String(error));
        }
    });
    const reset = (): void => {
        generation += 1;
        active = false;
        strokes = [];
        transformHistory = null;
        drawing = null;
        selectedIndex = null;
        for (const control of controls) control.disabled = true;
        source.value = "";
        copyFrom.value = "";
        if (downloadUrl !== null) {
            URL.revokeObjectURL(downloadUrl);
            downloadUrl = null;
        }
        render();
        message("Requires a compatible backend session");
    };
    reset();
    return {
        reset,
        start: (): void => {
            active = true;
            for (const control of controls) control.disabled = false;
            render();
            message("Select one exact glyph and add hand-drawn segments");
        },
    };
}
