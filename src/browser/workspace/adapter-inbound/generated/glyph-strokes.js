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
function tallStem(cx, cy, radius, g) {
    return {
        from: { x: cx, y: g === "l"
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
        "n", "u", "h", "m", "l", "t", "v", "w",
        "c", "e", "i", "í"].includes(g)) {
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
    if (["n", "u", "h", "m"].includes(g)) {
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
        const bow = roundedStory(cx, cy, radius, g);
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
        output.push(line(bow.lastStem.from, bow.lastStem.to));
        return output;
    }
    if (g === "v" || g === "w") {
        const vertices = zigzag(cx, cy, radius, g);
        return vertices.slice(1).map((end, index) => {
            const first = vertices[index];
            if (first === undefined)
                throw new Error("Missing zigzag point");
            return validateStroke({
                grapheme: g, x1: first.x, y1: first.y,
                x2: end.x, y2: end.y, width: pen, color,
            });
        });
    }
    if (g === "l" || g === "t") {
        const stem = tallStem(cx, cy, radius, g);
        if (stem.from.y < halfPen) {
            throw new Error("Tall stem and pen must fit inside glyph cell");
        }
        const lines = [validateStroke({
                grapheme: g, x1: stem.from.x, y1: stem.from.y,
                x2: stem.to.x, y2: stem.to.y, width: pen, color,
            })];
        if (g === "t")
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
    const base = strokes.filter((stroke) => stroke.grapheme === source)
        .map(validateStroke);
    if (base.length === 0) {
        throw new Error("Source grapheme has no authored strokes");
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
    const earlier = required("glyph-earlier");
    const later = required("glyph-later");
    const save = required("glyph-save");
    const preview = required("glyph-preview");
    const status = required("glyph-status");
    const numeric = {};
    const ranges = {};
    for (const axis of AXES) {
        numeric[axis] = required(`glyph-${axis}`);
        ranges[axis] = required(`glyph-${axis}-range`);
    }
    const controls = [character, color, source, add, copyFrom, copy, compose,
        form, construct, centerX, centerY, radius, accentShift, accentLift,
        selected, update,
        remove, reverse, earlier, later, save,
        ...AXES.flatMap((axis) => [numeric[axis], ranges[axis]])];
    let active = false;
    let generation = 0;
    let strokes = [];
    let selectedIndex = null;
    let drawing = null;
    let downloadUrl = null;
    const message = (text) => { status.textContent = text; };
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
                "p", "q"].includes(chosen.grapheme);
            const circleState = circleVisible ? shown : "hidden";
            idealCircle.setAttribute("visibility", circleState);
            const stemGlyph = ["a", "á", "b", "d", "p", "q"]
                .includes(chosen.grapheme)
                ? chosen.grapheme
                : "a";
            const stem = loopStem(cx, cy, r, stemGlyph);
            idealStem.setAttribute("x1", String(stem.from.x));
            idealStem.setAttribute("y1", String(stem.from.y));
            idealStem.setAttribute("x2", String(stem.to.x));
            idealStem.setAttribute("y2", String(stem.to.y));
            idealStem.setAttribute("stroke-width", numeric.width.value);
            idealStem.setAttribute("stroke", color.value);
            const stemVisible = active && ["a", "á", "b", "d", "p", "q"]
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
            const isZigzag = ["v", "w"].includes(chosen.grapheme);
            const vertices = zigzag(cx, cy, r, chosen.grapheme === "w" ? "w" : "v");
            const zigzagD = [
                `M ${vertices[0]?.x} ${vertices[0]?.y}`,
                ...vertices.slice(1).map((p) => `L ${p.x} ${p.y}`),
            ].join(" ");
            const roundedD = isZigzag ? zigzagD
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
                && ["n", "u", "h", "m", "v", "w"].includes(chosen.grapheme);
            const roundedState = roundedVisible ? "visible" : "hidden";
            const roundedStemState = roundedVisible && !isZigzag
                ? "visible" : "hidden";
            roundedPath.setAttribute("visibility", roundedState);
            roundedLeft.setAttribute("visibility", roundedStemState);
            roundedRight.setAttribute("visibility", roundedStemState);
            const openStart = openCirclePoint(cx, cy, r, 0);
            const openEnd = openCirclePoint(cx, cy, r, OPEN_ARC_SEGMENTS);
            const arcHeader = ["M", openStart.x, openStart.y];
            const arcTail = ["A", r, r, 0, 1, 1,
                openEnd.x, openEnd.y];
            const openD = [...arcHeader, ...arcTail].join(" ");
            openPath.setAttribute("d", openD);
            openPath.setAttribute("stroke-width", numeric.width.value);
            openPath.setAttribute("stroke", color.value);
            const openVisible = active && ["c", "e"]
                .includes(chosen.grapheme);
            const openState = openVisible ? "visible" : "hidden";
            openPath.setAttribute("visibility", openState);
            eBar.setAttribute("x1", String(cx - r));
            eBar.setAttribute("y1", String(cy));
            eBar.setAttribute("x2", String(cx + Math.round(r * 0.55)));
            eBar.setAttribute("y2", String(cy));
            eBar.setAttribute("stroke-width", numeric.width.value);
            eBar.setAttribute("stroke", color.value);
            const barState = active && ["e", "t"]
                .includes(chosen.grapheme) ? "visible" : "hidden";
            eBar.setAttribute("visibility", barState);
            const pen = Number(numeric.width.value);
            const iParts = ["i", "í"].includes(chosen.grapheme)
                ? dottedI(cx, cy, r, pen)
                : dottedI(cx, cy, 1100, 250);
            if (["l", "t"].includes(chosen.grapheme)) {
                const tallKind = chosen.grapheme === "l" ? "l" : "t";
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
            const iState = active && ["i", "í", "l", "t"]
                .includes(chosen.grapheme) ? "visible" : "hidden";
            const dotState = active && chosen.grapheme === "i"
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
            ideal.setAttribute("aria-label", "Analytic geometric "
                + form.value + " guide, not retained in line-segment TSV");
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
    const render = () => {
        renderIdeal();
        const shown = strokes.flatMap((stroke, index) => {
            return stroke.grapheme === character.value
                ? [{ stroke, index }] : [];
        });
        if (!shown.some(({ index }) => index === selectedIndex)) {
            selectedIndex = null;
        }
        const options = shown.map(({ index }, position) => {
            const option = document.createElement("option");
            option.value = String(index);
            option.textContent = "Stroke " + (position + 1)
                + " of " + shown.length;
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
        earlier.disabled = !active || neighbor(-1) === null;
        later.disabled = !active || neighbor(1) === null;
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
            const bytes = encodeGlyphLibrary(strokes);
            const mediaType = "text/tab-separated-values";
            const blob = new Blob([bytes], { type: mediaType });
            if (downloadUrl !== null)
                URL.revokeObjectURL(downloadUrl);
            const url = URL.createObjectURL(blob);
            downloadUrl = url;
            const link = document.createElement("a");
            link.href = url;
            link.download = "my-glyph-strokes.tsv";
            link.rel = "noopener";
            link.click();
            window.setTimeout(() => {
                if (downloadUrl === url) {
                    URL.revokeObjectURL(url);
                    downloadUrl = null;
                }
            }, 60_000);
            message("Glyph TSV downloaded; Rust validates it before export");
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
