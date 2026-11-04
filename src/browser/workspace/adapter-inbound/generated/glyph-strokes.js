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
const AXES = ["x1", "y1", "x2", "y2", "width"];
function exactGrapheme(value) {
    if (!value || new TextEncoder().encode(value).length > 64
        || /[\p{Cc}]/u.test(value) || value === " ")
        return false;
    if (typeof Intl.Segmenter !== "function")
        return false;
    return [...new Intl.Segmenter(undefined, {
            granularity: "grapheme",
        }).segment(value)].length === 1;
}
function boundedInteger(raw, maximum, label) {
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
function validateStroke(stroke) {
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
export function parseGlyphLibrary(text) {
    if (new TextEncoder().encode(text).length > MAX_BYTES
        || !text.endsWith("\n") || text.includes("\r")) {
        throw new Error("Glyph TSV exceeds 512 KiB or has bad line endings");
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
export function encodeGlyphLibrary(strokes) {
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
export function candidatePenPaths(strokes, grapheme) {
    if (!exactGrapheme(grapheme)) {
        throw new Error("Path inspection requires one exact visible glyph");
    }
    const chosen = strokes.filter((stroke) => stroke.grapheme === grapheme);
    if (chosen.length === 0)
        return [];
    const paths = [];
    const record = (first, endExclusive) => {
        const start = chosen[first];
        const end = chosen[endExclusive - 1];
        paths.push({
            first: first + 1, last: endExclusive,
            start: { x: start.x1, y: start.y1 },
            end: { x: end.x2, y: end.y2 },
            width: start.width, color: start.color.toUpperCase(),
        });
    };
    let first = 0;
    for (let index = 1; index < chosen.length; index += 1) {
        const before = chosen[index - 1];
        const after = chosen[index];
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
export function rotateClosedCandidatePath(strokes, grapheme, selectedIndex) {
    if (!Number.isSafeInteger(selectedIndex) || selectedIndex < 0
        || strokes[selectedIndex]?.grapheme !== grapheme) {
        throw new Error("Path start needs a selected exact glyph stroke");
    }
    const indices = strokes.flatMap((stroke, index) => stroke.grapheme === grapheme ? [index] : []);
    const position = indices.indexOf(selectedIndex) + 1;
    const path = candidatePenPaths(strokes, grapheme).find((candidate) => position >= candidate.first && position <= candidate.last);
    if (!path || path.first === path.last || position === path.first) {
        throw new Error("Closed path needs a later selected start stroke");
    }
    if (path.start.x !== path.end.x || path.start.y !== path.end.y) {
        throw new Error("Path start change requires a closed pen loop");
    }
    const slots = indices.slice(path.first - 1, path.last);
    const shift = position - path.first;
    const reordered = [...slots.slice(shift), ...slots.slice(0, shift)]
        .map((index) => strokes[index]);
    const next = strokes.slice();
    slots.forEach((index, offset) => { next[index] = reordered[offset]; });
    encodeGlyphLibrary(next);
    return { strokes: next, selectedIndex: slots[0] };
}
// Close one selected open path by an explicit short straight pen line.
// Reuse native 1200 µm join bounds, preserve interleaved exact Unicode rows,
// and validate the entire proposed v1 TSV before touching the local draft.
export function closeCandidatePath(strokes, grapheme, selectedIndex) {
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
    if (squared === 0)
        throw new Error("Path is already closed");
    if (squared > 1200 * 1200) {
        throw new Error("Path closure exceeds 1200 µm gap limit");
    }
    if (strokes.length === MAX_STROKES) {
        throw new Error("Path closure exceeds 4096 authored strokes");
    }
    const endSlot = indices[path.last - 1];
    const closure = {
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
export function reverseCandidatePath(strokes, grapheme, selectedIndex) {
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
    const reversed = slots.map((index) => strokes[index]).reverse()
        .map((stroke) => ({
        ...stroke, x1: stroke.x2, y1: stroke.y2,
        x2: stroke.x1, y2: stroke.y1,
    }));
    const next = strokes.slice();
    slots.forEach((index, i) => { next[index] = reversed[i]; });
    return {
        strokes: next,
        selectedIndex: slots[slots.length - 1
            - slots.indexOf(selectedIndex)],
    };
}
// Translate one complete candidate pen path while retaining every joint,
// interleaved Unicode TSV slot, and the original pen/segment direction.
// Validate the ENTIRE moved contour before returning a changed draft.
export function translateCandidatePath(strokes, grapheme, selectedIndex, dx, dy) {
    if (!Number.isSafeInteger(selectedIndex) || selectedIndex < 0
        || strokes[selectedIndex]?.grapheme !== grapheme) {
        throw new Error("Path translation needs a selected exact glyph stroke");
    }
    if (!Number.isSafeInteger(dx) || !Number.isSafeInteger(dy)
        || Math.abs(dx) > 3500 || Math.abs(dy) > 4500
        || (dx === 0 && dy === 0)) {
        throw new Error("Path offsets need nonzero bounded whole µm values");
    }
    const indices = strokes.flatMap((stroke, index) => stroke.grapheme === grapheme ? [index] : []);
    const position = indices.indexOf(selectedIndex) + 1;
    const path = candidatePenPaths(strokes, grapheme).find((candidate) => position >= candidate.first && position <= candidate.last);
    if (!path)
        throw new Error("Selected glyph has no candidate pen path");
    const slots = indices.slice(path.first - 1, path.last);
    const translated = slots.map((index) => {
        const stroke = strokes[index];
        const moved = {
            ...stroke, x1: stroke.x1 + dx, y1: stroke.y1 + dy,
            x2: stroke.x2 + dx, y2: stroke.y2 + dy,
        };
        validateStroke(moved);
        const radius = stroke.width / 2;
        if ([moved.x1, moved.x2].some((x) => x < radius || x > 3500 - radius)
            || [moved.y1, moved.y2].some((y) => y < radius || y > 4500 - radius)) {
            throw new Error("Translated pen contour clips the glyph cell");
        }
        return moved;
    });
    const next = strokes.slice();
    slots.forEach((index, offset) => { next[index] = translated[offset]; });
    encodeGlyphLibrary(next);
    return { strokes: next, selectedIndex };
}
// Reorder neighboring candidate paths, keeping every stroke within each
// path in the same direction/order and every other glyph in the same TSV slot.
export function moveCandidatePath(strokes, grapheme, selectedIndex, direction) {
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
    const earlier = paths[Math.min(chosen, adjacent)];
    const later = paths[Math.max(chosen, adjacent)];
    const slots = indices.slice(earlier.first - 1, later.last);
    const former = slots.slice(0, earlier.last - earlier.first + 1);
    const latter = slots.slice(former.length);
    const reordered = [...latter, ...former].map((index) => strokes[index]);
    const next = strokes.slice();
    slots.forEach((index, offset) => { next[index] = reordered[offset]; });
    const withinChosen = position - paths[chosen].first;
    const newPosition = direction === -1 ? withinChosen
        : latter.length + withinChosen;
    return {
        strokes: next, selectedIndex: slots[newPosition],
    };
}
// The construction is defined by an analytic circle; v1 TSV can only
// preserve its rounded line-segment approximation, not that ideal primitive.
function acuteAccent(cx, cy, radius, pen, dx, lift, topOffset = radius) {
    const baseline = cy - topOffset - pen - 120 - lift;
    return {
        x1: cx - Math.round(radius * 0.35) + dx, y1: baseline,
        x2: cx + Math.round(radius * 0.25) + dx,
        y2: baseline - Math.round(radius * 0.45),
    };
}
// Two quadratic arcs give the upper story a smooth horizontal top tangent.
// The bowl remains an analytic circle. Only the sampled ink is serialized.
function upperStory(cx, cy, radius) {
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
function openCirclePoint(cx, cy, radius, step) {
    const radians = Math.PI / 4
        + 3 * Math.PI / 2 * step / OPEN_ARC_SEGMENTS;
    return {
        x: cx + radius * Math.cos(radians),
        y: cy + radius * Math.sin(radians),
    };
}
function dottedI(cx, cy, radius, pen) {
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
function hookedJ(cx, cy, radius, pen) {
    const dotted = dottedI(cx, cy, radius, pen);
    const turnY = cy + Math.round(radius * 1.4);
    const start = { x: cx, y: cy + Math.round(radius * 0.7) };
    const control = { x: cx, y: turnY };
    const end = { x: cx - Math.round(radius / 2), y: turnY };
    return {
        stem: { from: dotted.stem.from, to: start },
        dot: dotted.dot,
        turn: [start, control, end],
    };
}
// Lowercase g reuses the unmodified ideal o circle and adds an ordered
// right stem plus a separate quadratic descender with bounded pen width.
function hookedG(cx, cy, radius) {
    const x = cx + radius;
    const from = { x, y: cy + Math.round(radius * 0.65) };
    const bottom = cy + Math.round(radius * 1.45);
    const control = { x, y: bottom };
    const end = { x: cx + Math.round(radius / 2), y: bottom };
    return {
        stem: { from: { x, y: cy }, to: from },
        turn: [from, control, end],
    };
}
// Closed-bowl letters share exactly the same circle as o/a. Ascenders
// extend above the bowl; descenders extend below it. In v1 only their
// integer straight-stem geometry survives the explicit TSV export.
function loopStem(cx, cy, radius, g) {
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
function roundedStory(cx, cy, radius, g) {
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
function zigzag(cx, cy, radius, g) {
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
function straightLines(cx, cy, radius, glyph) {
    const left = cx - radius;
    const right = cx + radius;
    const upperY = cy - radius;
    const bottom = cy + radius;
    const chain = (vertices) => vertices.slice(1).map((to, index) => {
        const from = vertices[index];
        if (from === undefined)
            throw new Error("Missing straight vertex");
        return { from, to };
    });
    if (glyph === "v" || glyph === "w") {
        return chain(zigzag(cx, cy, radius, glyph));
    }
    if (glyph === "x")
        return [
            { from: { x: left, y: upperY }, to: { x: right, y: bottom } },
            { from: { x: right, y: upperY }, to: { x: left, y: bottom } },
        ];
    if (glyph === "z")
        return chain([
            { x: left, y: upperY }, { x: right, y: upperY },
            { x: left, y: bottom }, { x: right, y: bottom },
        ]);
    if (glyph === "y")
        return [
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
function punctuationParts(cx, cy, r, glyph) {
    const point = (x, y) => ({
        x: cx + Math.round(x * r), y: cy + Math.round(y * r),
    });
    const dot = () => ({
        from: point(0, 0.78), to: point(0, 0.9),
    });
    if (glyph === ".")
        return [dot()];
    if (glyph === ",")
        return [{
                from: point(0.12, 0.72), to: point(-0.17, 0.98),
            }];
    const upsideDown = glyph === "¡" || glyph === "¿";
    const rotate = (p) => upsideDown
        ? { x: 2 * cx - p.x, y: 2 * cy - p.y } : p;
    const rotatePart = (piece) => ({
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
function quoteParts(cx, cy, r, source) {
    const point = (x, y) => ({
        x: cx + Math.round(x * r), y: cy + Math.round(y * r),
    });
    if (source === "«") {
        const chevron = (x) => [
            { from: point(x + 0.4, -0.5), to: point(x, 0) },
            { from: point(x, 0), to: point(x + 0.4, 0.5) },
        ];
        return [...chevron(-0.7), ...chevron(-0.05)];
    }
    const curl = (shift) => ({
        from: point(shift - 0.05, -0.85),
        control: point(shift - 0.32, -0.5),
        to: point(shift + 0.04, -0.18),
    });
    return source === "‘" ? [curl(0)]
        : [curl(-0.32), curl(0.32)];
}
function quotedSource(glyph) {
    return glyph === "»" ? "«"
        : glyph === "’" ? "‘" : glyph === "”" ? "“"
            : glyph;
}
function quoteMirror(cx, glyph, p) {
    return ["»", "’", "”"].includes(glyph)
        ? { x: 2 * cx - p.x, y: p.y } : p;
}
function mathParts(cx, cy, radius, glyph) {
    const point = (x, y) => ({
        x: cx + Math.round(radius * x),
        y: cy + Math.round(radius * y),
    });
    const horizontal = (y) => ({
        from: point(-0.7, y), to: point(0.7, y),
    });
    const vertical = () => ({
        from: point(0, -0.7), to: point(0, 0.7),
    });
    if (glyph === "-" || glyph === "−")
        return [horizontal(0)];
    if (glyph === "+")
        return [horizontal(0), vertical()];
    if (glyph === "=")
        return [horizontal(-0.3), horizontal(0.3)];
    if (glyph === "×")
        return [
            { from: point(-0.6, -0.6), to: point(0.6, 0.6) },
            { from: point(0.6, -0.6), to: point(-0.6, 0.6) },
        ];
    return [horizontal(0),
        { from: point(0, -0.7), to: point(0, -0.58) },
        { from: point(0, 0.58), to: point(0, 0.7) },
    ];
}
function digitGeometry(cx, cy, radius, glyph) {
    const p = (x, y) => ({
        x: cx + Math.round(radius * x),
        y: cy + Math.round(radius * y),
    });
    const line = (x1, y1, x2, y2) => ({ from: p(x1, y1), to: p(x2, y2) });
    const arc = (x1, y1, hx, hy, x2, y2) => ({
        from: p(x1, y1), control: p(hx, hy), to: p(x2, y2),
    });
    const ellipse = (y, rx, ry) => ({
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
function ellipsePoint(ellipse, step) {
    const radians = 2 * Math.PI * step / CIRCLE_SEGMENTS;
    return {
        x: ellipse.cx + Math.round(ellipse.rx * Math.cos(radians)),
        y: ellipse.cy + Math.round(ellipse.ry * Math.sin(radians)),
    };
}
function digitGuide(geometry) {
    const loops = geometry.ellipses.map((ellipse) => {
        const { cx, cy, rx, ry } = ellipse;
        return `M ${cx + rx} ${cy} A ${rx} ${ry} 0 1 1 ${cx - rx} ${cy}`
            + ` A ${rx} ${ry} 0 1 1 ${cx + rx} ${cy}`;
    });
    return [...loops, punctuationGuide(geometry.parts)].filter(Boolean)
        .join(" ");
}
function capitalParts(cx, cy, radius, glyph) {
    const point = (x, y) => ({
        x: cx + Math.round(radius * x),
        y: cy + Math.round(radius * y),
    });
    const line = (x1, y1, x2, y2) => ({ from: point(x1, y1), to: point(x2, y2) });
    const curve = (x1, y1, hx, hy, x2, y2) => ({
        from: point(x1, y1), control: point(hx, hy),
        to: point(x2, y2),
    });
    const stem = () => line(-0.75, -1, -0.75, 1);
    const bowl = () => [
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
function capitalGuide(cx, cy, radius, glyph) {
    if (glyph === "S") {
        return smoothS(cx, cy, radius).map(([from, control, to], index) => `${index === 0 ? `M ${from.x} ${from.y} ` : ""}`
            + `Q ${control.x} ${control.y} ${to.x} ${to.y}`).join(" ");
    }
    return punctuationGuide(capitalParts(cx, cy, radius, glyph));
}
const TEXT_MARKS = [";", ":", "\"", "'", "…", "–", "—",
    "(", ")", "[", "]"];
function textMarkParts(cx, cy, r, glyph) {
    const p = (x, y) => ({
        x: cx + Math.round(x * r), y: cy + Math.round(y * r),
    });
    const line = (x, y, xx, yy) => ({ from: p(x, y), to: p(xx, yy) });
    const dot = (x, y) => line(x, y, x, y + 0.12);
    if (glyph === ":")
        return [dot(0, -0.6), dot(0, 0.6)];
    if (glyph === ";")
        return [dot(0, -0.6),
            line(0.13, 0.6, -0.18, 0.98)];
    if (glyph === "'")
        return [line(0.04, -0.95, -0.07, -0.55)];
    if (glyph === "\"")
        return [line(-0.35, -0.95, -0.46, -0.55),
            line(0.35, -0.95, 0.24, -0.55)];
    if (glyph === "…")
        return [-0.7, 0, 0.7].map((x) => dot(x, 0.78));
    if (glyph === "–" || glyph === "—") {
        const half = glyph === "—" ? 0.9 : 0.65;
        return [line(-half, 0, half, 0)];
    }
    if (glyph === "[")
        return [line(0.3, -0.9, -0.5, -0.9),
            line(-0.5, -0.9, -0.5, 0.9), line(-0.5, 0.9, 0.3, 0.9)];
    if (glyph === "]")
        return [line(-0.3, -0.9, 0.5, -0.9),
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
function punctuationGuide(parts) {
    const commands = [];
    let pen = null;
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
function straightGuide(lines) {
    const commands = [];
    let pen = null;
    for (const { from, to } of lines) {
        if (pen === null || pen.x !== from.x || pen.y !== from.y) {
            commands.push(`M ${from.x} ${from.y}`);
        }
        commands.push(`L ${to.x} ${to.y}`);
        pen = to;
    }
    return commands.join(" ");
}
function tallStem(cx, cy, radius, g) {
    return {
        from: { x: cx, y: g === "l" || g === "f"
                ? cy - Math.round(radius * 1.5) : cy - radius },
        to: { x: cx, y: cy + radius },
    };
}
function doubleHump(cx, cy, radius) {
    const left = cx - radius;
    const right = cx + radius;
    const high = cy - radius;
    const bottom = cy + radius;
    const firstPeak = cx - Math.round(radius / 2);
    const secondPeak = cx + Math.round(radius / 2);
    const arcs = [
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
function smoothS(cx, cy, radius) {
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
function quadratic(from, control, to, step) {
    const t = step / ARCH_SEGMENTS;
    const u = 1 - t;
    return {
        x: Math.round(u * u * from.x + 2 * u * t * control.x
            + t * t * to.x),
        y: Math.round(u * u * from.y + 2 * u * t * control.y
            + t * t * to.y),
    };
}
export function geometricStarter(g, x, y, r, w, color, pos, kind = "single") {
    if (!["o", "a", "ó", "á", "b", "d", "p", "q",
        "n", "u", "h", "m", "r", "l", "t", "f", "v", "w",
        "k", "x", "y", "z", "c", "e", "i", "í", "j", "g", "s",
        ".", ",", "!", "¡", "?", "¿",
        "«", "»", "‘", "’", "“", "”",
        "+", "-", "−", "=", "×", "÷",
        ..."0123456789", ..."ABCDEFGHIJKLMNOPQRSTUVWXYZ",
        ...TEXT_MARKS].includes(g)) {
        throw new Error("Choose a supported exact geometric glyph");
    }
    if (kind !== "single" && kind !== "two-storey"
        || (kind === "two-storey" && !["a", "á"].includes(g))) {
        throw new Error("Two-storey construction requires a or á");
    }
    const cx = boundedInteger(String(x), 3500, "center X");
    const cy = boundedInteger(String(y), 4500, "center Y");
    const radius = boundedInteger(String(r), 1600, "radius");
    const pen = boundedInteger(String(w), 1500, "width");
    const dx = pos?.dx ?? 0;
    if (!Number.isSafeInteger(dx) || dx < -500 || dx > 500) {
        throw new Error("Acute shift must be within -500..500 µm");
    }
    const lift = boundedInteger(String(pos?.lift ?? 0), 500, "accent lift");
    const halfPen = pen / 2;
    const topOffset = kind === "two-storey" ? 2 * radius : radius;
    if (radius < 200 || cx - radius < halfPen
        || cy - radius < halfPen
        || cx + radius + halfPen > 3500
        || cy + radius + halfPen > 4500) {
        throw new Error("Shape and pen thickness must fit inside glyph cell");
    }
    if (TEXT_MARKS.includes(g)) {
        if ([";", ":", "…"].includes(g) && radius * 0.5 < pen + 80) {
            throw new Error("Text punctuation needs dot pen-clearance");
        }
        const line = (from, to) => {
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
        return textMarkParts(cx, cy, radius, g).flatMap(({ from, control, to }) => control === undefined
            ? [line(from, to)]
            : Array.from({ length: ARCH_SEGMENTS }, (_, step) => line(quadratic(from, control, to, step), quadratic(from, control, to, step + 1))));
    }
    if (/^[A-Z]$/.test(g)) {
        const line = (from, to) => {
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
        const parts = capitalParts(cx, cy, radius, g);
        const output = [];
        if (g === "O" || g === "Q") {
            const circle = (step) => {
                const a = 2 * Math.PI * step / CIRCLE_SEGMENTS;
                return { x: Math.round(cx + radius * Math.cos(a)),
                    y: Math.round(cy + radius * Math.sin(a)) };
            };
            for (let step = 0; step < CIRCLE_SEGMENTS; step += 1) {
                output.push(line(circle(step), circle(step + 1)));
            }
        }
        else if (g === "C" || g === "G") {
            for (let step = 0; step < OPEN_ARC_SEGMENTS; step += 1) {
                const first = openCirclePoint(cx, cy, radius, step);
                const last = openCirclePoint(cx, cy, radius, step + 1);
                output.push(line({ x: Math.round(first.x),
                    y: Math.round(first.y) }, { x: Math.round(last.x),
                    y: Math.round(last.y) }));
            }
        }
        else if (g === "S") {
            for (const [from, control, to] of smoothS(cx, cy, radius)) {
                for (let step = 0; step < ARCH_SEGMENTS; step += 1) {
                    output.push(line(quadratic(from, control, to, step), quadratic(from, control, to, step + 1)));
                }
            }
        }
        for (const { from, control, to } of parts) {
            if (control === undefined)
                output.push(line(from, to));
            else
                for (let step = 0; step < ARCH_SEGMENTS; step += 1) {
                    output.push(line(quadratic(from, control, to, step), quadratic(from, control, to, step + 1)));
                }
        }
        return output;
    }
    if (/^[0-9]$/.test(g)) {
        const geometry = digitGeometry(cx, cy, radius, g);
        const line = (from, to) => {
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
        const ellipses = geometry.ellipses.flatMap((ellipse) => Array.from({ length: CIRCLE_SEGMENTS }, (_, step) => line(ellipsePoint(ellipse, step), ellipsePoint(ellipse, step + 1))));
        const parts = geometry.parts.flatMap(({ from, control, to }) => control === undefined ? [line(from, to)]
            : Array.from({ length: ARCH_SEGMENTS }, (_, step) => line(quadratic(from, control, to, step), quadratic(from, control, to, step + 1))));
        return [...ellipses, ...parts];
    }
    if (["+", "-", "−", "=", "×", "÷"].includes(g)) {
        if (g === "÷" && radius * 0.58 < pen + 100) {
            throw new Error("Division dots need visible pen-clearance");
        }
        return mathParts(cx, cy, radius, g).map(({ from, to }) => {
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
        });
    }
    if (["«", "»", "‘", "’", "“", "”"].includes(g)) {
        const glyph = g;
        const pieces = quoteParts(cx, cy, radius, quotedSource(glyph));
        const orient = (point) => quoteMirror(cx, glyph, point);
        const line = (from, to) => {
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
        return pieces.flatMap(({ from, control, to }) => control === undefined ? [line(orient(from), orient(to))]
            : Array.from({ length: ARCH_SEGMENTS }, (_, step) => line(orient(quadratic(from, control, to, step)), orient(quadratic(from, control, to, step + 1)))));
    }
    if ([".", ",", "!", "¡", "?", "¿"].includes(g)) {
        // Rotate already-rounded native samples so the inverted template
        // remains an exact integer transform, not a separately rounded curve.
        const inverted = g === "¡" || g === "¿";
        const source = g === "¡" ? "!" : g === "¿" ? "?"
            : g;
        const parts = punctuationParts(cx, cy, radius, source);
        const orient = (point) => inverted
            ? { x: 2 * cx - point.x, y: 2 * cy - point.y } : point;
        if (g === "!" || g === "¡" || g === "?" || g === "¿") {
            const before = parts.at(-2);
            const dot = parts.at(-1);
            if (before === undefined || dot === undefined
                || Math.abs(dot.from.y - before.to.y) < pen + 100) {
                throw new Error("Punctuation dot needs visible pen-clearance");
            }
        }
        const line = (from, to) => {
            for (const point of [from, to]) {
                if (point.x < halfPen || point.x + halfPen > 3500
                    || point.y < halfPen || point.y + halfPen > 4500) {
                    throw new Error("Punctuation ink must fit inside glyph cell");
                }
            }
            return validateStroke({ grapheme: g,
                x1: from.x, y1: from.y, x2: to.x, y2: to.y,
                width: pen, color });
        };
        return parts.flatMap(({ from, to, control }) => {
            if (control === undefined)
                return [line(orient(from), orient(to))];
            return Array.from({ length: ARCH_SEGMENTS }, (_, step) => line(orient(quadratic(from, control, to, step)), orient(quadratic(from, control, to, step + 1))));
        });
    }
    if (g === "s") {
        const lines = [];
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
        const line = (first, last) => {
            return validateStroke({
                grapheme: g, x1: first.x, y1: first.y,
                x2: last.x, y2: last.y, width: pen, color,
            });
        };
        if (g === "m") {
            const parts = doubleHump(cx, cy, radius);
            const lines = [line(parts.firstStem.from, parts.firstStem.to)];
            for (const [index, [from, control, to]] of parts.arcs.entries()) {
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
        const bow = roundedStory(cx, cy, radius, g === "r" ? "n" : g);
        if (g === "h" && bow.firstStem.to.y < halfPen) {
            throw new Error("Ascender stem and pen must fit inside glyph cell");
        }
        const output = [line(bow.firstStem.from, bow.firstStem.to)];
        for (const [from, control, to] of [
            [bow.start, bow.left, bow.peak],
            [bow.peak, bow.right, bow.end],
        ]) {
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
        const lines = straightLines(cx, cy, radius, g);
        if (lines.some(({ from, to }) => [from, to].some((point) => point.x < halfPen || point.y < halfPen
            || point.x + halfPen > 3500
            || point.y + halfPen > 4500))) {
            throw new Error("Straight strokes and pen must fit inside glyph cell");
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
        if (g === "t" || g === "f")
            lines.push(validateStroke({
                grapheme: g, x1: cx - radius, y1: cy,
                x2: cx + Math.round(radius * 0.55), y2: cy,
                width: pen, color,
            }));
        return lines;
    }
    if (g === "c" || g === "e") {
        const result = Array.from({ length: OPEN_ARC_SEGMENTS }, (_, index) => {
            const first = openCirclePoint(cx, cy, radius, index);
            const last = openCirclePoint(cx, cy, radius, index + 1);
            return validateStroke({
                grapheme: g, x1: Math.round(first.x),
                y1: Math.round(first.y), x2: Math.round(last.x),
                y2: Math.round(last.y), width: pen, color,
            });
        });
        if (g === "e")
            result.push(validateStroke({
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
        const line = (from, to) => validateStroke({ grapheme: g, x1: from.x, y1: from.y,
            x2: to.x, y2: to.y, width: pen, color });
        const segments = [line(parts.stem.from, parts.stem.to)];
        const [from, control, to] = parts.turn;
        for (let step = 0; step < ARCH_SEGMENTS; step += 1) {
            segments.push(line(quadratic(from, control, to, step), quadratic(from, control, to, step + 1)));
        }
        segments.push(line(parts.dot.from, parts.dot.to));
        return segments;
    }
    if (g === "i" || g === "í") {
        const parts = dottedI(cx, cy, radius, pen);
        const segments = [parts.stem];
        if (g === "i") {
            segments.push(parts.dot);
        }
        else {
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
    const point = (step) => {
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
        const line = (from, to) => validateStroke({ grapheme: g, x1: from.x, y1: from.y,
            x2: to.x, y2: to.y, width: pen, color });
        lines.push(line(parts.stem.from, parts.stem.to));
        const [from, control, to] = parts.turn;
        for (let step = 0; step < ARCH_SEGMENTS; step += 1) {
            lines.push(line(quadratic(from, control, to, step), quadratic(from, control, to, step + 1)));
        }
        return lines;
    }
    if (kind === "two-storey") {
        if (cy - topOffset < halfPen) {
            throw new Error("Two-storey arch exceeds glyph cell");
        }
        const { start, peak, end, left, right } = upperStory(cx, cy, radius);
        const spans = [
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
        const { from, to } = loopStem(cx, cy, radius, g);
        if (from.y - halfPen < 0 || to.y + halfPen > 4500) {
            throw new Error("Loop stem and pen must fit inside glyph cell");
        }
        lines.push(validateStroke({
            grapheme: g, x1: from.x, y1: from.y,
            x2: to.x, y2: to.y, width: pen, color,
        }));
    }
    if (g === "ó" || g === "á") {
        const accent = acuteAccent(cx, cy, radius, pen, dx, lift, topOffset);
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
export function composeAccentedGlyph(strokes, source, target, color, pos = { dx: 0, lift: 0 }) {
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
            const separated = base.every((other, otherIndex) => otherIndex === index || low + stroke.width / 2 + 100
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
    const point = (x, yCoord) => ({
        x: Math.round(x), y: Math.round(yCoord),
    });
    const segments = combining === "\u0301"
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
function seededUnit(seed) {
    let state = seed >>> 0 || 0x6D2B79F5;
    return () => {
        state ^= state << 13;
        state ^= state >>> 17;
        state ^= state << 5;
        return (state >>> 0) / 0xFFFFFFFF * 2 - 1;
    };
}
function occurrenceSeed(seed, occurrence) {
    return (seed ^ Math.imul(occurrence + 1, 0x9E3779B9)) >>> 0;
}
function checkVariation(seed, occurrence, variation) {
    if (!Number.isSafeInteger(seed) || seed < 0 || seed > 0xFFFFFFFF
        || !Number.isSafeInteger(occurrence) || occurrence < 0
        || occurrence > 10000) {
        throw new Error("Variation seed or occurrence outside bounds");
    }
    for (const [key, max] of [
        ["width", 20], ["height", 20], ["thickness", 20],
        ["slant", 12], ["spacing", 500], ["regularity", 150],
    ]) {
        const value = variation[key];
        if (!Number.isSafeInteger(value) || value < 0 || value > max) {
            throw new Error(`Variation ${key} outside 0..${max}`);
        }
    }
}
export function varyGlyphStrokes(source, grapheme, seed, occurrence, variation) {
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
    const point = (x, y) => {
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
                throw new Error("Varied ink and pen must fit inside glyph cell");
            }
        }
        return validateStroke({
            ...stroke, x1: a.x, y1: a.y, x2: b.x, y2: b.y, width,
        });
    });
}
function required(id) {
    const node = document.getElementById(id);
    if (node === null)
        throw new Error(`Missing glyph editor control: ${id}`);
    return node;
}
export function createGlyphStrokeEditor() {
    const character = required("glyph-character");
    const color = required("glyph-color");
    const source = required("glyph-load");
    const add = required("glyph-add");
    const copyFrom = required("glyph-copy-from");
    const copy = required("glyph-copy");
    const compose = required("glyph-compose");
    const form = required("glyph-form");
    const construct = required("glyph-construct");
    const seedLowercase = required("glyph-seed-lowercase");
    const seedSpanish = required("glyph-seed-spanish");
    const seedSpanishNfd = required("glyph-seed-spanish-nfd");
    const seedUpperSpanish = required("glyph-seed-upper-spanish");
    const seedUpperSpanishNfd = required("glyph-seed-upper-spanish-nfd");
    const seedPunctuation = required("glyph-seed-punctuation");
    const seedQuotes = required("glyph-seed-quotes");
    const seedMath = required("glyph-seed-math");
    const seedDigits = required("glyph-seed-digits");
    const seedCapitals = required("glyph-seed-capitals");
    const seedTextMarks = required("glyph-seed-text-marks");
    const centerX = required("glyph-circle-x");
    const centerY = required("glyph-circle-y");
    const radius = required("glyph-circle-radius");
    const accentShift = required("glyph-accent-shift");
    const accentLift = required("glyph-accent-lift");
    const ideal = required("glyph-ideal-preview");
    const idealCircle = required("glyph-ideal-circle");
    const idealStem = required("glyph-ideal-stem");
    const idealAccent = required("glyph-ideal-accent");
    const idealArch = required("glyph-ideal-arch");
    const roundedPath = required("glyph-ideal-rounded");
    const roundedLeft = required("glyph-ideal-rounded-left");
    const roundedRight = required("glyph-ideal-rounded-right");
    const openPath = required("glyph-ideal-c-loop");
    const eBar = required("glyph-ideal-e-bar");
    const iStem = required("glyph-ideal-i-stem");
    const iDot = required("glyph-ideal-i-dot");
    const selected = required("glyph-selected");
    const update = required("glyph-update");
    const remove = required("glyph-remove");
    const reverse = required("glyph-reverse");
    const reversePath = required("glyph-reverse-path");
    const startPath = required("glyph-path-start");
    const closePath = required("glyph-path-close");
    const translatePath = required("glyph-path-translate");
    const pathDx = required("glyph-path-dx");
    const pathDy = required("glyph-path-dy");
    const earlierPath = required("glyph-path-earlier");
    const laterPath = required("glyph-path-later");
    const earlier = required("glyph-earlier");
    const later = required("glyph-later");
    const join = required("glyph-join");
    const save = required("glyph-save");
    const preview = required("glyph-preview");
    const status = required("glyph-status");
    const pathsStatus = required("glyph-paths");
    const variationPreview = required("glyph-variation-preview");
    const variationStatus = required("glyph-variation-status");
    const variationSeed = required("glyph-variation-seed");
    const variationSample = required("glyph-variation-sample");
    const variationDownload = required("glyph-variation-download");
    const variationControls = {};
    for (const key of ["width", "height", "thickness", "slant", "spacing",
        "regularity"]) {
        variationControls[key] = required(`glyph-variation-${key}`);
    }
    const numeric = {};
    const ranges = {};
    for (const axis of AXES) {
        numeric[axis] = required(`glyph-${axis}`);
        ranges[axis] = required(`glyph-${axis}-range`);
    }
    const controls = [character, color, source, add, copyFrom, copy, compose,
        form, construct, seedLowercase, seedSpanish, seedSpanishNfd,
        seedUpperSpanish, seedUpperSpanishNfd,
        seedPunctuation, seedQuotes, seedMath, seedDigits, seedCapitals,
        seedTextMarks,
        centerX, centerY, radius,
        accentShift, accentLift, variationSeed,
        variationSample, variationDownload,
        ...Object.values(variationControls),
        selected, update,
        remove, reverse, reversePath, startPath, closePath, translatePath,
        pathDx, pathDy,
        earlierPath, laterPath, earlier, later, join, save,
        ...AXES.flatMap((axis) => [numeric[axis], ranges[axis]])];
    let active = false;
    let generation = 0;
    let strokes = [];
    let selectedIndex = null;
    let drawing = null;
    let downloadUrl = null;
    const message = (text) => { status.textContent = text; };
    const download = (bytes, filename) => {
        const blob = new Blob([bytes], { type: "text/tab-separated-values" });
        if (downloadUrl !== null)
            URL.revokeObjectURL(downloadUrl);
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
    const neighbor = (offset) => {
        if (selectedIndex === null)
            return null;
        const grapheme = strokes[selectedIndex]?.grapheme;
        let index = selectedIndex + offset;
        while (index >= 0 && index < strokes.length) {
            if (strokes[index]?.grapheme === grapheme)
                return index;
            index += offset;
        }
        return null;
    };
    const selectedForm = () => ({
        grapheme: form.value.endsWith("-two")
            ? form.value.slice(0, -4) : form.value,
        variant: form.value.endsWith("-two")
            ? "two-storey" : "single",
    });
    const starterFromInputs = () => {
        const chosen = selectedForm();
        const values = [
            chosen.grapheme, Number(centerX.value), Number(centerY.value),
            Number(radius.value), Number(numeric.width.value), color.value,
        ];
        const position = {
            dx: Number(accentShift.value), lift: Number(accentLift.value),
        };
        return geometricStarter(...values, position, chosen.variant);
    };
    const renderIdeal = () => {
        try {
            // The SVG circle remains mathematically circular. Exporting the
            // v1 library explicitly samples it into straight ink segments.
            starterFromInputs();
            const cx = Number(centerX.value);
            const cy = Number(centerY.value);
            const r = Number(radius.value);
            idealCircle.setAttribute("cx", String(cx));
            idealCircle.setAttribute("cy", String(cy));
            idealCircle.setAttribute("r", String(r));
            idealCircle.setAttribute("stroke-width", numeric.width.value);
            idealCircle.setAttribute("stroke", color.value);
            const shown = active ? "visible" : "hidden";
            const chosen = selectedForm();
            const circleVisible = ["o", "a", "ó", "á", "b", "d",
                "p", "q", "g", "O", "Q"].includes(chosen.grapheme);
            const circleState = circleVisible ? shown : "hidden";
            idealCircle.setAttribute("visibility", circleState);
            const stemGlyph = ["a", "á", "b", "d", "p", "q"]
                .includes(chosen.grapheme)
                ? chosen.grapheme
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
            const { start, peak, end, left, right } = upperStory(cx, cy, r);
            const beginning = ["M", start.x, start.y].join(" ");
            const firstArc = ["Q", left.x, left.y, peak.x, peak.y].join(" ");
            const nextArc = ["Q", right.x, right.y, end.x, end.y].join(" ");
            const archPath = [beginning, firstArc, nextArc].join(" ");
            idealArch.setAttribute("d", archPath);
            idealArch.setAttribute("stroke-width", numeric.width.value);
            idealArch.setAttribute("stroke", color.value);
            const archVisible = active && chosen.variant === "two-storey";
            const archState = archVisible ? "visible" : "hidden";
            idealArch.setAttribute("visibility", archState);
            const roundedKind = chosen.grapheme === "u" ? "u"
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
                `M ${cx} ${cy} L ${cx} ${cy + r}`,
            ].join(" ");
            const isStraight = ["v", "w", "k", "x", "y", "z"]
                .includes(chosen.grapheme);
            const straightD = isStraight ? straightGuide(straightLines(cx, cy, r, chosen.grapheme)) : "";
            const jParts = chosen.grapheme === "j"
                ? hookedJ(cx, cy, r, Number(numeric.width.value)) : null;
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
                ? smoothS(cx, cy, r).map(([from, control, to], index) => `${index === 0 ? `M ${from.x} ${from.y} ` : ""}`
                    + `Q ${control.x} ${control.y} ${to.x} ${to.y}`).join(" ") : "";
            const isPunctuation = [".", ",", "!", "¡", "?", "¿"]
                .includes(chosen.grapheme);
            const punctuationD = isPunctuation
                ? punctuationGuide(punctuationParts(cx, cy, r, chosen.grapheme)) : "";
            const isQuote = ["«", "»", "‘", "’", "“", "”"]
                .includes(chosen.grapheme);
            const quoteD = isQuote ? punctuationGuide(quoteParts(cx, cy, r, quotedSource(chosen.grapheme))
                .map(({ from, to, control }) => ({
                from: quoteMirror(cx, chosen.grapheme, from),
                to: quoteMirror(cx, chosen.grapheme, to),
                ...(control ? { control: quoteMirror(cx, chosen.grapheme, control) } : {}),
            }))) : "";
            const isMath = ["+", "-", "−", "=", "×", "÷"]
                .includes(chosen.grapheme);
            const mathD = isMath ? punctuationGuide(mathParts(cx, cy, r, chosen.grapheme)) : "";
            const isDigit = /^[0-9]$/.test(chosen.grapheme);
            const digitD = isDigit ? digitGuide(digitGeometry(cx, cy, r, chosen.grapheme)) : "";
            const isCapital = /^[A-Z]$/.test(chosen.grapheme);
            const capitalD = isCapital ? capitalGuide(cx, cy, r, chosen.grapheme) : "";
            const isTextMark = TEXT_MARKS
                .includes(chosen.grapheme);
            const markD = isTextMark ? punctuationGuide(textMarkParts(cx, cy, r, chosen.grapheme)) : "";
            const roundedD = isTextMark ? markD
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
            ]) {
                element.setAttribute("x1", String(part.from.x));
                element.setAttribute("y1", String(part.from.y));
                element.setAttribute("x2", String(part.to.x));
                element.setAttribute("y2", String(part.to.y));
                element.setAttribute("stroke-width", numeric.width.value);
                element.setAttribute("stroke", color.value);
            }
            const roundedVisible = active
                && ["n", "u", "h", "m", "r", "j", "g", "s",
                    "v", "w", "k", "x", "y", "z",
                    ".", ",", "!", "¡", "?", "¿",
                    "«", "»", "‘", "’", "“", "”",
                    "+", "-", "−", "=", "×", "÷",
                    ..."0123456789", ..."ABCDEFGHIJKLMNOPQRSTUVWXYZ",
                    ...TEXT_MARKS]
                    .includes(chosen.grapheme);
            const roundedState = roundedVisible ? "visible" : "hidden";
            const roundedStemState = roundedVisible && !isStraight
                && !isPunctuation && !isQuote && !isMath && !isDigit
                && !isCapital && !isTextMark
                && !["j", "g", "s"].includes(chosen.grapheme)
                ? "visible" : "hidden";
            roundedPath.setAttribute("visibility", roundedState);
            roundedLeft.setAttribute("visibility", roundedStemState);
            const roundedRightState = chosen.grapheme === "r"
                ? "hidden" : roundedStemState;
            roundedRight.setAttribute("visibility", roundedRightState);
            const openStart = openCirclePoint(cx, cy, r, 0);
            const openEnd = openCirclePoint(cx, cy, r, OPEN_ARC_SEGMENTS);
            const arcHeader = ["M", openStart.x, openStart.y];
            const arcTail = ["A", r, r, 0, 1, 1,
                openEnd.x, openEnd.y];
            const openD = [...arcHeader, ...arcTail].join(" ");
            openPath.setAttribute("d", openD);
            openPath.setAttribute("stroke-width", numeric.width.value);
            openPath.setAttribute("stroke", color.value);
            const openVisible = active && ["c", "e", "C", "G"]
                .includes(chosen.grapheme);
            const openState = openVisible ? "visible" : "hidden";
            openPath.setAttribute("visibility", openState);
            eBar.setAttribute("x1", String(cx - r));
            eBar.setAttribute("y1", String(cy));
            eBar.setAttribute("x2", String(cx + Math.round(r * 0.55)));
            eBar.setAttribute("y2", String(cy));
            eBar.setAttribute("stroke-width", numeric.width.value);
            eBar.setAttribute("stroke", color.value);
            const barState = active && ["e", "t", "f"]
                .includes(chosen.grapheme) ? "visible" : "hidden";
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
            ]) {
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
            const top = chosen.variant === "two-storey" ? 2 * r : r;
            const accent = acuteAccent(cx, cy, r, pen, dx, lift, top);
            for (const axis of ["x1", "y1", "x2", "y2"]) {
                idealAccent.setAttribute(axis, String(accent[axis]));
            }
            idealAccent.setAttribute("stroke-width", numeric.width.value);
            idealAccent.setAttribute("stroke", color.value);
            const hasAcute = ["ó", "á", "í"].includes(chosen.grapheme);
            const accentState = active && hasAcute ? "visible" : "hidden";
            idealAccent.setAttribute("visibility", accentState);
            const guideLabel = isMath
                ? `Exact mathematical ${form.value} ink guide`
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
        }
        catch {
            idealCircle.setAttribute("visibility", "hidden");
            idealStem.setAttribute("visibility", "hidden");
            idealArch.setAttribute("visibility", "hidden");
            roundedPath.setAttribute("visibility", "hidden");
            roundedLeft.setAttribute("visibility", "hidden");
            roundedRight.setAttribute("visibility", "hidden");
            openPath.setAttribute("visibility", "hidden");
            eBar.setAttribute("visibility", "hidden");
            iStem.setAttribute("visibility", "hidden");
            iDot.setAttribute("visibility", "hidden");
            idealAccent.setAttribute("visibility", "hidden");
            ideal.setAttribute("aria-label", "Invalid geometric settings");
        }
    };
    const readVariation = () => {
        const seed = boundedInteger(variationSeed.value, 0xFFFFFFFF, "variation seed");
        const settings = {};
        for (const key of ["width", "height", "thickness", "slant",
            "spacing", "regularity"]) {
            const raw = variationControls[key].value;
            if (!/^[0-9]+$/.test(raw)) {
                throw new Error(`Variation ${key} requires whole units`);
            }
            settings[key] = Number(raw);
        }
        checkVariation(seed, 0, settings);
        return { seed, settings };
    };
    const renderVariation = () => {
        const authored = strokes.filter((item) => item.grapheme === character.value);
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
            const lines = [];
            for (let row = 0; row < 2; row += 1) {
                for (let copy = 0; copy < 3; copy += 1) {
                    const sequence = row === 0 ? authored
                        : varyGlyphStrokes(authored, character.value, seed, copy, settings);
                    const tracking = row === 0 ? 0 : Math.round(seededUnit(occurrenceSeed(seed, copy + 3))()
                        * settings.spacing);
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
                        line.setAttribute("data-variation-row", row === 0 ? "ideal" : "seeded");
                        lines.push(line);
                    }
                }
            }
            variationPreview.replaceChildren(...lines);
            variationDownload.disabled = false;
            variationPreview.setAttribute("aria-label", `Top: three ideal ${character.value} glyphs; bottom: `
                + `three seed ${seed} varied glyphs`);
            variationStatus.textContent =
                `Three ideal and three seeded varied ${character.value} `
                    + "copies. Authored library unchanged; preview only.";
        }
        catch (error) {
            variationDownload.disabled = true;
            variationPreview.replaceChildren();
            variationPreview.setAttribute("aria-label", "Variation refused");
            variationStatus.textContent = String(error);
        }
    };
    const render = () => {
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
        const pathNumbers = new Array(shown.length);
        for (const [pathIndex, path] of paths.entries()) {
            for (let i = path.first - 1; i < path.last; i += 1) {
                pathNumbers[i] = pathIndex + 1;
            }
        }
        if (!active) {
            pathsStatus.textContent = "Requires a compatible backend session";
        }
        else if (paths.length === 0) {
            pathsStatus.textContent = "No authored pen paths for this glyph";
        }
        else {
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
        const pathIndex = paths.indexOf(chosenPath);
        earlierPath.disabled = !active || pathIndex <= 0;
        laterPath.disabled = !active || pathIndex < 0
            || pathIndex >= paths.length - 1;
        earlier.disabled = !active || neighbor(-1) === null;
        later.disabled = !active || neighbor(1) === null;
        join.disabled = !active || neighbor(1) === null;
        const lines = shown.map(({ stroke, index }) => {
            const line = document.createElementNS(SVG_NS, "line");
            for (const axis of ["x1", "y1", "x2", "y2"]) {
                line.setAttribute(axis, String(stroke[axis]));
            }
            line.setAttribute("stroke", stroke.color);
            line.setAttribute("stroke-width", String(stroke.width));
            line.setAttribute("stroke-linecap", "round");
            if (index === selectedIndex) {
                line.setAttribute("data-selected", "true");
            }
            line.addEventListener("click", () => {
                if (active)
                    choose(index);
            });
            line.addEventListener("pointerdown", (event) => {
                event.stopPropagation();
                if (active && event.button === 0)
                    choose(index);
            });
            return line;
        });
        if (drawing !== null && drawing.grapheme === character.value) {
            const points = drawing.points.slice();
            const last = points.at(-1);
            if (last?.x !== drawing.last.x || last?.y !== drawing.last.y) {
                points.push(drawing.last);
            }
            if (points.length === 1)
                points.push(drawing.last);
            for (let i = 1; i < points.length; i += 1) {
                const first = points[i - 1];
                const second = points[i];
                if (first === undefined || second === undefined)
                    continue;
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
        const label = `${shown.length} strokes for selected glyph`;
        preview.setAttribute("aria-label", label);
        renderVariation();
    };
    const choose = (index) => {
        const stroke = strokes[index];
        if (!active || stroke?.grapheme !== character.value)
            return;
        selectedIndex = index;
        for (const axis of AXES) {
            numeric[axis].value = String(stroke[axis]);
            ranges[axis].value = String(stroke[axis]);
        }
        color.value = stroke.color;
        render();
    };
    const editable = () => validateStroke({
        grapheme: character.value, color: color.value,
        x1: boundedInteger(numeric.x1.value, 3500, "x1"),
        y1: boundedInteger(numeric.y1.value, 4500, "y1"),
        x2: boundedInteger(numeric.x2.value, 3500, "x2"),
        y2: boundedInteger(numeric.y2.value, 4500, "y2"),
        width: boundedInteger(numeric.width.value, 1500, "width"),
    });
    for (const control of [
        form, centerX, centerY, radius, color, accentShift, accentLift,
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
        if (!active)
            return;
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
            if (axis === "width")
                renderIdeal();
        });
        ranges[axis].addEventListener("input", () => {
            numeric[axis].value = ranges[axis].value;
            if (axis === "width")
                renderIdeal();
        });
    }
    const pointerPoint = (event) => {
        const box = preview.getBoundingClientRect();
        if (box.width <= 0 || box.height <= 0
            || !Number.isFinite(event.clientX)
            || !Number.isFinite(event.clientY))
            return null;
        const x = Math.round(3500 * (event.clientX - box.left) / box.width);
        const y = Math.round(4500 * (event.clientY - box.top) / box.height);
        return {
            x: Math.max(0, Math.min(3500, x)),
            y: Math.max(0, Math.min(4500, y)),
        };
    };
    preview.addEventListener("pointerdown", (event) => {
        if (!active || drawing !== null || event.button !== 0
            || !event.isPrimary)
            return;
        const first = pointerPoint(event);
        if (first === null)
            return;
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
        }
        catch (error) {
            message(String(error));
        }
    });
    preview.addEventListener("pointermove", (event) => {
        if (!active || drawing?.pointerId !== event.pointerId)
            return;
        const point = pointerPoint(event);
        if (point === null)
            return;
        drawing.last = point;
        const sampled = drawing.points.at(-1);
        if (sampled !== undefined) {
            const dx = point.x - sampled.x;
            const dy = point.y - sampled.y;
            if (dx * dx + dy * dy >= 60 * 60) {
                if (drawing.points.length >= 129) {
                    drawing.overflow = true;
                }
                else {
                    drawing.points.push(point);
                }
            }
        }
        render();
    });
    preview.addEventListener("pointerup", (event) => {
        if (!active || drawing?.pointerId !== event.pointerId)
            return;
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
        }
        catch (error) {
            message(String(error));
        }
        render();
    });
    preview.addEventListener("pointercancel", (event) => {
        if (drawing?.pointerId !== event.pointerId)
            return;
        drawing = null;
        if (preview.hasPointerCapture(event.pointerId)) {
            preview.releasePointerCapture(event.pointerId);
        }
        render();
    });
    construct.addEventListener("click", () => {
        if (!active)
            return;
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
        }
        catch (error) {
            message(String(error));
        }
    });
    seedLowercase.addEventListener("click", () => {
        if (!active)
            return;
        try {
            if (drawing !== null) {
                throw new Error("Finish the current pen trace before seeding");
            }
            const authored = new Set(strokes.map((stroke) => stroke.grapheme));
            const missing = [..."abcdefghijklmnopqrstuvwxyz"].filter((glyph) => !authored.has(glyph));
            if (missing.length === 0) {
                message("All 26 lowercase glyphs already have authored ink");
                return;
            }
            const x = Number(centerX.value);
            const y = Number(centerY.value);
            const r = Number(radius.value);
            const pen = Number(numeric.width.value);
            const added = missing.flatMap((glyph) => geometricStarter(glyph, x, y, r, pen, color.value));
            const next = [...strokes, ...added];
            encodeGlyphLibrary(next);
            strokes = next;
            selectedIndex = null;
            render();
            message(`Filled ${missing.length} missing lowercase templates (`
                + `${added.length} sampled strokes); unsaved draft`);
        }
        catch (error) {
            message(String(error));
        }
    });
    seedPunctuation.addEventListener("click", () => {
        if (!active)
            return;
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
            const added = missing.flatMap((g) => geometricStarter(g, Number(centerX.value), Number(centerY.value), Number(radius.value), Number(numeric.width.value), color.value));
            const next = [...strokes, ...added];
            encodeGlyphLibrary(next);
            strokes = next;
            selectedIndex = null;
            render();
            message(`Filled ${missing.length} missing punctuation glyphs (`
                + `${added.length} sampled strokes); unsaved draft`);
        }
        catch (error) {
            message(String(error));
        }
    });
    seedQuotes.addEventListener("click", () => {
        if (!active)
            return;
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
            const added = missing.flatMap((g) => geometricStarter(g, Number(centerX.value), Number(centerY.value), Number(radius.value), Number(numeric.width.value), color.value));
            const next = [...strokes, ...added];
            encodeGlyphLibrary(next);
            strokes = next;
            selectedIndex = null;
            render();
            message(`Filled ${missing.length} missing quotation glyphs (`
                + `${added.length} sampled strokes); unsaved draft`);
        }
        catch (error) {
            message(String(error));
        }
    });
    seedMath.addEventListener("click", () => {
        if (!active)
            return;
        try {
            if (drawing !== null) {
                throw new Error("Finish the current pen trace before seeding");
            }
            const glyphs = ["+", "-", "−", "=", "×", "÷"];
            const authored = new Set(strokes.map((stroke) => stroke.grapheme));
            const missing = glyphs.filter((g) => !authored.has(g));
            if (missing.length === 0) {
                message("All six basic math glyphs have authored ink");
                return;
            }
            const added = missing.flatMap((g) => geometricStarter(g, Number(centerX.value), Number(centerY.value), Number(radius.value), Number(numeric.width.value), color.value));
            const next = [...strokes, ...added];
            encodeGlyphLibrary(next);
            strokes = next;
            selectedIndex = null;
            render();
            message(`Filled ${missing.length} missing math glyphs (`
                + `${added.length} exact strokes); unsaved draft`);
        }
        catch (error) {
            message(String(error));
        }
    });
    seedDigits.addEventListener("click", () => {
        if (!active)
            return;
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
            const added = missing.flatMap((g) => geometricStarter(g, Number(centerX.value), Number(centerY.value), Number(radius.value), Number(numeric.width.value), color.value));
            const next = [...strokes, ...added];
            encodeGlyphLibrary(next);
            strokes = next;
            selectedIndex = null;
            render();
            message(`Filled ${missing.length} missing numeral glyphs (`
                + `${added.length} sampled strokes); unsaved draft`);
        }
        catch (error) {
            message(String(error));
        }
    });
    seedCapitals.addEventListener("click", () => {
        if (!active)
            return;
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
            const added = missing.flatMap((g) => geometricStarter(g, Number(centerX.value), Number(centerY.value), Number(radius.value), Number(numeric.width.value), color.value));
            const next = [...strokes, ...added];
            encodeGlyphLibrary(next);
            strokes = next;
            selectedIndex = null;
            render();
            message(`Filled ${missing.length} missing uppercase glyphs (`
                + `${added.length} sampled strokes); unsaved draft`);
        }
        catch (error) {
            message(String(error));
        }
    });
    seedTextMarks.addEventListener("click", () => {
        if (!active)
            return;
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
            const added = missing.flatMap((g) => geometricStarter(g, Number(centerX.value), Number(centerY.value), Number(radius.value), Number(numeric.width.value), color.value));
            const next = [...strokes, ...added];
            encodeGlyphLibrary(next);
            strokes = next;
            selectedIndex = null;
            render();
            message(`Filled ${missing.length} missing text punctuation `
                + `glyphs (${added.length} sampled strokes); unsaved draft`);
        }
        catch (error) {
            message(String(error));
        }
    });
    const fillSpanishAccents = (normalization, letterCase = "lowercase") => {
        if (!active)
            return;
        try {
            if (drawing !== null) {
                throw new Error("Finish the current pen trace before accents");
            }
            const targets = [
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
                return [casedBase, normalized];
            }).filter(([, target]) => !authored.has(target));
            if (missing.length === 0) {
                message(`All seven Spanish ${letterCase} ${normalization}`
                    + " accents are authored");
                return;
            }
            const withoutBases = missing.flatMap(([base]) => authored.has(base) ? [] : [base]);
            if (withoutBases.length > 0) {
                throw new Error(`Author ${letterCase} base glyphs first: `
                    + [...new Set(withoutBases)].join(", "));
            }
            const shift = Number(accentShift.value);
            const lift = Number(accentLift.value);
            const added = missing.flatMap(([base, target]) => composeAccentedGlyph(strokes, base, target, color.value, { dx: shift, lift }));
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
        }
        catch (error) {
            message(String(error));
        }
    };
    seedSpanish.addEventListener("click", () => fillSpanishAccents("NFC"));
    seedSpanishNfd.addEventListener("click", () => fillSpanishAccents("NFD"));
    seedUpperSpanish.addEventListener("click", () => fillSpanishAccents("NFC", "uppercase"));
    seedUpperSpanishNfd.addEventListener("click", () => fillSpanishAccents("NFD", "uppercase"));
    copy.addEventListener("click", () => {
        if (!active)
            return;
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
        }
        catch (error) {
            message(String(error));
        }
    });
    compose.addEventListener("click", () => {
        if (!active)
            return;
        try {
            const target = character.value;
            const sourceGlyph = copyFrom.value;
            const lines = composeAccentedGlyph(strokes, sourceGlyph, target, color.value, {
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
        }
        catch (error) {
            message(String(error));
        }
    });
    add.addEventListener("click", () => {
        if (!active)
            return;
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
        }
        catch (error) {
            message(String(error));
        }
    });
    update.addEventListener("click", () => {
        if (!active || selectedIndex === null)
            return;
        try {
            const changed = editable();
            const index = selectedIndex;
            if (strokes[index]?.grapheme !== character.value)
                return;
            const next = strokes.slice();
            next[index] = changed;
            encodeGlyphLibrary(next);
            strokes = next;
            render();
            message("Updated selected stroke; " + strokes.length
                + " unsaved strokes");
        }
        catch (error) {
            message(String(error));
        }
    });
    reverse.addEventListener("click", () => {
        if (!active || selectedIndex === null)
            return;
        const index = selectedIndex;
        const existing = strokes[index];
        if (existing?.grapheme !== character.value)
            return;
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
        if (!active || selectedIndex === null)
            return;
        try {
            const glyph = character.value;
            const changed = reverseCandidatePath(strokes, glyph, selectedIndex);
            encodeGlyphLibrary(changed.strokes);
            strokes = changed.strokes;
            choose(changed.selectedIndex);
            message("Reversed selected continuous pen path; unsaved");
        }
        catch (error) {
            message(String(error));
        }
    });
    startPath.addEventListener("click", () => {
        if (!active || selectedIndex === null)
            return;
        try {
            const changed = rotateClosedCandidatePath(strokes, character.value, selectedIndex);
            strokes = changed.strokes;
            choose(changed.selectedIndex);
            message("Changed closed pen loop starting stroke; unsaved");
        }
        catch (error) {
            message(String(error));
        }
    });
    closePath.addEventListener("click", () => {
        if (!active || selectedIndex === null)
            return;
        try {
            const glyph = character.value;
            const changed = closeCandidatePath(strokes, glyph, selectedIndex);
            strokes = changed.strokes;
            choose(changed.selectedIndex);
            message("Closed selected pen path with a short bridge; unsaved");
        }
        catch (error) {
            message(String(error));
        }
    });
    translatePath.addEventListener("click", () => {
        if (!active || selectedIndex === null)
            return;
        try {
            const signed = (raw, maximum) => {
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
            const changed = translateCandidatePath(strokes, character.value, selectedIndex, signed(pathDx.value, 3500), signed(pathDy.value, 4500));
            strokes = changed.strokes;
            choose(changed.selectedIndex);
            message("Translated selected continuous pen path; unsaved");
        }
        catch (error) {
            message(String(error));
        }
    });
    const movePath = (direction) => {
        if (!active || selectedIndex === null)
            return;
        try {
            const glyph = character.value;
            const index = selectedIndex;
            const changed = moveCandidatePath(strokes, glyph, index, direction);
            encodeGlyphLibrary(changed.strokes);
            strokes = changed.strokes;
            choose(changed.selectedIndex);
            message("Reordered selected continuous pen path; unsaved");
        }
        catch (error) {
            message(String(error));
        }
    };
    earlierPath.addEventListener("click", () => movePath(-1));
    laterPath.addEventListener("click", () => movePath(1));
    const move = (offset) => {
        if (!active || selectedIndex === null)
            return;
        const index = selectedIndex;
        const target = neighbor(offset);
        if (target === null)
            return;
        const next = strokes.slice();
        [next[index], next[target]] = [next[target], next[index]];
        encodeGlyphLibrary(next);
        strokes = next;
        choose(target);
        message("Changed pen stroke order; unsaved");
    };
    earlier.addEventListener("click", () => move(-1));
    later.addEventListener("click", () => move(1));
    join.addEventListener("click", () => {
        if (!active || selectedIndex === null)
            return;
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
            const bridge = {
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
        }
        catch (error) {
            message(String(error));
        }
    });
    remove.addEventListener("click", () => {
        if (!active || selectedIndex === null)
            return;
        if (strokes[selectedIndex]?.grapheme !== character.value)
            return;
        strokes.splice(selectedIndex, 1);
        selectedIndex = null;
        render();
        message(strokes.length + " unsaved authored strokes");
    });
    source.addEventListener("change", async () => {
        if (!active)
            return;
        const file = source.files?.[0];
        if (file === undefined)
            return;
        generation += 1;
        const expected = generation;
        source.value = "";
        try {
            if (file.size > MAX_BYTES) {
                throw new Error("Glyph file exceeds 512 KiB");
            }
            const bytes = await file.arrayBuffer();
            if (!active || generation !== expected)
                return;
            const decoder = new TextDecoder("utf-8", { fatal: true });
            const text = decoder.decode(bytes);
            const checked = parseGlyphLibrary(text);
            strokes = checked;
            drawing = null;
            selectedIndex = null;
            render();
            message(`Loaded ${checked.length} authored strokes in memory`);
        }
        catch (error) {
            if (active && generation === expected)
                message(String(error));
        }
    });
    save.addEventListener("click", () => {
        if (!active)
            return;
        try {
            download(encodeGlyphLibrary(strokes), "my-glyph-strokes.tsv");
            message("Glyph TSV downloaded; Rust validates it before export");
        }
        catch (error) {
            message(String(error));
        }
    });
    variationDownload.addEventListener("click", () => {
        if (!active)
            return;
        try {
            const { seed, settings } = readVariation();
            const sample = boundedInteger(variationSample.value, 3, "variation sample");
            if (sample < 1) {
                throw new Error("Choose varied sample 1, 2 or 3");
            }
            const changed = varyGlyphStrokes(strokes, character.value, seed, sample - 1, settings);
            let position = 0;
            const candidate = strokes.map((item) => item.grapheme === character.value
                ? changed[position++] : item);
            const bytes = encodeGlyphLibrary(candidate);
            download(bytes, "my-varied-glyph-strokes.tsv");
            message(`Varied sample ${sample} downloaded as a separate TSV; `
                + "authored draft unchanged, Rust validates before export");
        }
        catch (error) {
            message(String(error));
        }
    });
    const reset = () => {
        generation += 1;
        active = false;
        strokes = [];
        drawing = null;
        selectedIndex = null;
        for (const control of controls)
            control.disabled = true;
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
        start: () => {
            active = true;
            for (const control of controls)
                control.disabled = false;
            render();
            message("Select one exact glyph and add hand-drawn segments");
        },
    };
}
