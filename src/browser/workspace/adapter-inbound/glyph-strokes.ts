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
    if (!value || new TextEncoder().encode(value).length > 64
        || /[\p{Cc}]/u.test(value) || value === " ") return false;
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
            "Glyph TSV must be at most 524288 bytes, with LF lines",
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

// The construction is defined by an analytic circle; v1 TSV can only
// preserve its rounded line-segment approximation, not that ideal primitive.
function acuteAccent(
    cx: number, cy: number, radius: number, pen: number,
    dx: number, lift: number,
) {
    const baseline = cy - radius - pen - 120 - lift;
    return {
        x1: cx - Math.round(radius * 0.35) + dx, y1: baseline,
        x2: cx + Math.round(radius * 0.25) + dx,
        y2: baseline - Math.round(radius * 0.45),
    };
}

export function geometricStarter(
    g: string, x: number, y: number, r: number, w: number,
    color: string, pos?: { dx: number; lift: number },
): Stroke[] {
    if (!["o", "a", "ó", "á"].includes(g)) {
        throw new Error("Circle starter requires exact o, a, ó or á");
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
    if (radius < 200 || cx - radius < halfPen
        || cy - radius < halfPen
        || cx + radius + halfPen > 3500
        || cy + radius + halfPen > 4500) {
        throw new Error("Circle and pen thickness must fit inside glyph cell");
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
    if (g === "a" || g === "á") {
        lines.push(validateStroke({
            grapheme: g, x1: cx + radius, y1: cy - radius,
            x2: cx + radius, y2: cy + radius, width: pen, color,
        }));
    }
    if (g === "ó" || g === "á") {
        const accent = acuteAccent(cx, cy, radius, pen, dx, lift);
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
    const form = required<HTMLSelectElement>("glyph-form");
    const construct = required<HTMLButtonElement>("glyph-construct");
    const centerX = required<HTMLInputElement>("glyph-circle-x");
    const centerY = required<HTMLInputElement>("glyph-circle-y");
    const radius = required<HTMLInputElement>("glyph-circle-radius");
    const accentShift = required<HTMLInputElement>("glyph-accent-shift");
    const accentLift = required<HTMLInputElement>("glyph-accent-lift");
    const ideal = required<SVGSVGElement>("glyph-ideal-preview");
    const idealCircle = required<SVGCircleElement>("glyph-ideal-circle");
    const idealStem = required<SVGLineElement>("glyph-ideal-stem");
    const idealAccent = required<SVGLineElement>("glyph-ideal-accent");
    const selected = required<HTMLSelectElement>("glyph-selected");
    const update = required<HTMLButtonElement>("glyph-update");
    const remove = required<HTMLButtonElement>("glyph-remove");
    const reverse = required<HTMLButtonElement>("glyph-reverse");
    const earlier = required<HTMLButtonElement>("glyph-earlier");
    const later = required<HTMLButtonElement>("glyph-later");
    const save = required<HTMLButtonElement>("glyph-save");
    const preview = required<SVGSVGElement>("glyph-preview");
    const status = required<HTMLElement>("glyph-status");
    const numeric = {} as Record<Axis, HTMLInputElement>;
    const ranges = {} as Record<Axis, HTMLInputElement>;
    for (const axis of AXES) {
        numeric[axis] = required<HTMLInputElement>(`glyph-${axis}`);
        ranges[axis] = required<HTMLInputElement>(`glyph-${axis}-range`);
    }
    const controls = [character, color, source, add, copyFrom, copy,
        form, construct, centerX, centerY, radius, accentShift, accentLift,
        selected, update,
        remove, reverse, earlier, later, save,
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

    const message = (text: string): void => { status.textContent = text; };
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
    const starterFromInputs = (): Stroke[] => {
        const values: [string, number, number, number, number, string] = [
            form.value, Number(centerX.value), Number(centerY.value),
            Number(radius.value), Number(numeric.width.value), color.value,
        ];
        const position = {
            dx: Number(accentShift.value), lift: Number(accentLift.value),
        };
        return geometricStarter(...values, position);
    };
    const renderIdeal = (): void => {
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
            idealCircle.setAttribute("visibility", shown);
            idealStem.setAttribute("x1", String(cx + r));
            idealStem.setAttribute("y1", String(cy - r));
            idealStem.setAttribute("x2", String(cx + r));
            idealStem.setAttribute("y2", String(cy + r));
            idealStem.setAttribute("stroke-width", numeric.width.value);
            idealStem.setAttribute("stroke", color.value);
            const stemVisible = active && ["a", "á"].includes(form.value);
            const stemState = stemVisible ? "visible" : "hidden";
            idealStem.setAttribute("visibility", stemState);
            const pen = Number(numeric.width.value);
            const dx = Number(accentShift.value);
            const lift = Number(accentLift.value);
            const accent = acuteAccent(cx, cy, r, pen, dx, lift);
            for (const axis of ["x1", "y1", "x2", "y2"] as const) {
                idealAccent.setAttribute(axis, String(accent[axis]));
            }
            idealAccent.setAttribute("stroke-width", numeric.width.value);
            idealAccent.setAttribute("stroke", color.value);
            const hasAcute = ["ó", "á"].includes(form.value);
            const accentState = active && hasAcute ? "visible" : "hidden";
            idealAccent.setAttribute("visibility", accentState);
            ideal.setAttribute("aria-label", "Analytic geometric "
                + form.value + " guide, not retained in line-segment TSV");
        } catch {
            idealCircle.setAttribute("visibility", "hidden");
            idealStem.setAttribute("visibility", "hidden");
            idealAccent.setAttribute("visibility", "hidden");
            ideal.setAttribute("aria-label", "Invalid circle settings");
        }
    };
    const render = (): void => {
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
        preview.setAttribute("aria-label",
            shown.length + " authored strokes for the exact selected grapheme");
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
            if (character.value !== form.value) {
                throw new Error("Choose matching o, a, ó or á");
            }
            const lines = starterFromInputs();
            const next = [
                ...strokes.filter((stroke) => stroke.grapheme !== form.value),
                ...lines,
            ];
            encodeGlyphLibrary(next);
            strokes = next;
            selectedIndex = null;
            render();
            message("Constructed " + lines.length + " sampled segments for "
                + form.value + "; analytic circle is not retained by TSV");
        } catch (error) {
            message(String(error));
        }
    });
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
            const decoder = new TextDecoder("utf-8", { fatal: true });
            const text = decoder.decode(bytes);
            const checked = parseGlyphLibrary(text);
            strokes = checked;
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
            const bytes = encodeGlyphLibrary(strokes);
            const mediaType = "text/tab-separated-values";
            const blob = new Blob([bytes], { type: mediaType });
            if (downloadUrl !== null) URL.revokeObjectURL(downloadUrl);
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
        } catch (error) {
            message(String(error));
        }
    });
    const reset = (): void => {
        generation += 1;
        active = false;
        strokes = [];
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
