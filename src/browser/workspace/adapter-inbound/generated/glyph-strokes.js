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
//   - Generate character outlines, claim calibration, or admit Rust profiles.
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
//   - Edits real hand-authored line segments, never imaginary glyph shapes.
// - Description:
//   - Rust still validates downloaded files on CLI load or PDF/PNG export.
// - Usage:
//   - Initialized after the same-origin localhost session is admitted.
// - Defaults:
//   - Empty in memory; no browser storage, implicit export, or silent fallback.
//
//! Temporary local authoring surface for real explicit glyph line segments.
const HEADER = "atrament.glyph-strokes.v1";
const MAX_BYTES = 65_536;
const MAX_STROKES = 256;
const SVG_NS = "http://www.w3.org/2000/svg";
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
        throw new Error("Glyph TSV must be at most 65536 bytes, with LF lines");
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
        throw new Error("Save requires 1..256 authored glyph strokes");
    }
    const lines = strokes.map((item) => {
        const stroke = validateStroke(item);
        return [stroke.grapheme, stroke.x1, stroke.y1, stroke.x2,
            stroke.y2, stroke.width, stroke.color].join("\t");
    });
    const result = `${HEADER}\n${lines.join("\n")}\n`;
    if (new TextEncoder().encode(result).length > MAX_BYTES) {
        throw new Error("Glyph TSV exceeds 65536 bytes");
    }
    return result;
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
    const remove = required("glyph-remove");
    const save = required("glyph-save");
    const preview = required("glyph-preview");
    const status = required("glyph-status");
    const numeric = {};
    const ranges = {};
    for (const axis of AXES) {
        numeric[axis] = required(`glyph-${axis}`);
        ranges[axis] = required(`glyph-${axis}-range`);
    }
    const controls = [character, color, source, add, remove, save,
        ...AXES.flatMap((axis) => [numeric[axis], ranges[axis]])];
    let active = false;
    let generation = 0;
    let strokes = [];
    let downloadUrl = null;
    const message = (text) => { status.textContent = text; };
    const render = () => {
        const shown = strokes.filter((stroke) => {
            return stroke.grapheme === character.value;
        });
        const lines = shown.map((stroke) => {
            const line = document.createElementNS(SVG_NS, "line");
            for (const axis of ["x1", "y1", "x2", "y2"]) {
                line.setAttribute(axis, String(stroke[axis]));
            }
            line.setAttribute("stroke", stroke.color);
            line.setAttribute("stroke-width", String(stroke.width));
            line.setAttribute("stroke-linecap", "round");
            return line;
        });
        preview.replaceChildren(...lines);
        const description = `${shown.length} authored strokes for `
            + "the exact selected grapheme";
        preview.setAttribute("aria-label", description);
    };
    const editable = () => validateStroke({
        grapheme: character.value, color: color.value,
        x1: boundedInteger(numeric.x1.value, 3500, "x1"),
        y1: boundedInteger(numeric.y1.value, 4500, "y1"),
        x2: boundedInteger(numeric.x2.value, 3500, "x2"),
        y2: boundedInteger(numeric.y2.value, 4500, "y2"),
        width: boundedInteger(numeric.width.value, 1500, "width"),
    });
    character.addEventListener("input", render);
    for (const axis of AXES) {
        numeric[axis].addEventListener("input", () => {
            ranges[axis].value = numeric[axis].value;
        });
        ranges[axis].addEventListener("input", () => {
            numeric[axis].value = ranges[axis].value;
        });
    }
    add.addEventListener("click", () => {
        if (!active)
            return;
        try {
            if (strokes.length === MAX_STROKES) {
                throw new Error("Native glyph limit is 256 line segments");
            }
            const next = [...strokes, editable()];
            encodeGlyphLibrary(next);
            strokes = next;
            render();
            message(`${strokes.length} unsaved authored strokes`);
        }
        catch (error) {
            message(String(error));
        }
    });
    remove.addEventListener("click", () => {
        if (!active)
            return;
        const index = strokes.findLastIndex((stroke) => {
            return stroke.grapheme === character.value;
        });
        if (index < 0) {
            message("No segment for this exact grapheme to remove");
            return;
        }
        strokes.splice(index, 1);
        render();
        message(`${strokes.length} unsaved authored strokes`);
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
                throw new Error("Glyph file exceeds 64 KiB");
            }
            const bytes = await file.arrayBuffer();
            if (!active || generation !== expected)
                return;
            const decoder = new TextDecoder("utf-8", { fatal: true });
            const text = decoder.decode(bytes);
            const checked = parseGlyphLibrary(text);
            strokes = checked;
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
        for (const control of controls)
            control.disabled = true;
        source.value = "";
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
            message("Select one exact glyph and add hand-drawn segments");
        },
    };
}
