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
//   - Local browser controls for Rust-admitted handwriting statistics.
// - Must-Not:
//   - Invent glyph strokes, calibrate a writer, or admit profile JSON.
//   - Persist files, profiles, or credentials in browser storage.
// - Allows:
//   - Inputs: User fields and authenticated Rust profile/dimension responses.
//   - Outputs: Unsaved numeric proposals and explicitly downloaded JSON.
//   - Side effects: DOM, same-origin requests, and explicit file downloads.
// - Split-When:
//   - Independent calibrated glyph preview becomes executable.
// - Merge-When:
//   - The workspace owns statistics controls directly.
// - Summary:
//   - Edits every native bounded dimension and exact grapheme override.
// - Description:
//   - Native validation is the final authority; no strokes are synthesized.
// - Usage:
//   - Started only after a compatible authenticated localhost handshake.
// - Defaults:
//   - Session-free controls stay disabled; nothing is saved implicitly.
//
const FIELDS = [
    "minimum", "maximum", "mean", "median", "standard_deviation",
];
const MAX_JSON_BYTES = 1_048_576;
const MAX_GLYPHS = 512;
const MAX_CATALOG_BYTES = 4096;
function required(id) {
    const element = document.getElementById(id);
    if (element === null) {
        throw new Error(`Missing handwriting editor control: ${id}`);
    }
    return element;
}
function boundedJson(text, maximum) {
    if (new TextEncoder().encode(text).length > maximum) {
        throw new Error("Handwriting JSON exceeds its size limit");
    }
    return JSON.parse(text);
}
function record(value) {
    if (typeof value !== "object" || value === null || Array.isArray(value)) {
        throw new Error("Unexpected handwriting JSON shape");
    }
    return value;
}
function keysMatch(value, keys) {
    return Object.keys(value).length === keys.length
        && keys.every((key) => Object.hasOwn(value, key));
}
function safeInteger(value) {
    return typeof value === "number" && Number.isSafeInteger(value);
}
function parseCatalog(text) {
    const root = record(boundedJson(text, MAX_CATALOG_BYTES));
    if (!keysMatch(root, ["dimensions"]) || !Array.isArray(root.dimensions)
        || root.dimensions.length !== 10) {
        throw new Error("Invalid native dimension catalog");
    }
    const dimensions = [];
    for (const item of root.dimensions) {
        const entry = record(item);
        if (!keysMatch(entry, ["key", "unit", "minimum", "maximum"])
            || typeof entry.key !== "string"
            || !/^[a-z_]{2,32}$/.test(entry.key)
            || !["micrometres", "millidegrees", "parts_per_thousand"]
                .includes(String(entry.unit))
            || !safeInteger(entry.minimum) || !safeInteger(entry.maximum)
            || entry.minimum > entry.maximum
            || dimensions.some((previous) => previous.key === entry.key)) {
            throw new Error("Invalid native dimension catalog entry");
        }
        dimensions.push(entry);
    }
    return dimensions;
}
function parseProfile(text, dimensions) {
    const root = record(boundedJson(text, MAX_JSON_BYTES));
    if (!keysMatch(root, ["version", "seed", "globals", "glyphs"])
        || root.version !== "atrament.handwriting.statistics.v1"
        // JS cannot round-trip larger u64 seeds without losing digits.
        || !safeInteger(root.seed) || root.seed < 0
        || !Array.isArray(root.globals)
        || !Array.isArray(root.glyphs) || root.glyphs.length > MAX_GLYPHS) {
        throw new Error("Unsupported handwriting profile or seed precision");
    }
    const checkParameter = (raw) => {
        const item = record(raw);
        if (!keysMatch(item, ["dimension", ...FIELDS])
            || typeof item.dimension !== "string") {
            throw new Error("Unexpected statistics parameter");
        }
        const dimension = dimensions.find((d) => d.key === item.dimension);
        if (dimension === undefined) {
            throw new Error("Unknown native dimension");
        }
        for (const field of FIELDS) {
            const value = item[field];
            const lower = field === "standard_deviation"
                ? 0 : dimension.minimum;
            const upper = field === "standard_deviation"
                ? dimension.maximum - dimension.minimum : dimension.maximum;
            if (!safeInteger(value) || value < lower || value > upper) {
                throw new Error("Statistics outside native dimension bounds");
            }
        }
        return item;
    };
    const globals = root.globals.map(checkParameter);
    const completeGlobals = globals.length === dimensions.length;
    const uniqueGlobals = dimensions.every((dimension) => {
        const key = dimension.key;
        return globals.filter((entry) => entry.dimension === key).length === 1;
    });
    if (!completeGlobals || !uniqueGlobals) {
        throw new Error("Incomplete native global statistics");
    }
    const glyphs = [];
    for (const raw of root.glyphs) {
        const item = record(raw);
        if (!keysMatch(item, ["grapheme", "parameters"])
            || typeof item.grapheme !== "string"
            || !item.grapheme
            || new TextEncoder().encode(item.grapheme).length > 64
            || !Array.isArray(item.parameters)
            || item.parameters.length < 1
            || item.parameters.length > dimensions.length
            || glyphs.some((g) => g.grapheme === item.grapheme)) {
            throw new Error("Invalid exact grapheme override");
        }
        const parameters = item.parameters.map(checkParameter);
        if (new Set(parameters.map((p) => p.dimension)).size
            !== parameters.length) {
            throw new Error("Repeated glyph dimension");
        }
        glyphs.push({ grapheme: item.grapheme, parameters });
    }
    return { version: root.version, seed: root.seed, globals, glyphs };
}
export function createHandwritingProfileEditor(context) {
    const scope = required("handwriting-scope");
    const dimensionSelect = required("handwriting-dimension");
    const glyphSelect = required("handwriting-glyph");
    const newGlyph = required("handwriting-new-glyph");
    const addGlyph = required("handwriting-add-glyph");
    const removeGlyph = required("handwriting-remove-glyph");
    const addOverride = required("handwriting-add-override");
    const removeOverride = required("handwriting-remove-override");
    const seed = required("handwriting-seed");
    const upload = required("handwriting-load");
    const resetButton = required("handwriting-reset");
    const save = required("handwriting-save");
    const status = required("handwriting-status");
    const preview = required("handwriting-preview");
    const fields = FIELDS.map((field) => ({
        field,
        number: required(`handwriting-${field}`),
        slider: required(`handwriting-${field}-range`),
    }));
    const controls = [scope, dimensionSelect, glyphSelect, newGlyph,
        addGlyph, removeGlyph, addOverride, removeOverride, seed,
        upload, resetButton, save];
    let dimensions = [];
    let profile = null;
    let generation = 0;
    let busy = false;
    let downloadUrl = null;
    let selectedGlyph = "";
    let drafts = new WeakMap();
    function currentGlyph() {
        const glyphs = profile?.glyphs;
        return glyphs?.find((glyph) => glyph.grapheme === selectedGlyph);
    }
    function selectedDimension() {
        const key = dimensionSelect.value;
        return dimensions.find((dimension) => dimension.key === key);
    }
    function globalParameter(key) {
        const globals = profile?.globals;
        return globals?.find((parameter) => parameter.dimension === key);
    }
    function selectedParameter() {
        const key = dimensionSelect.value;
        return scope.value === "global" ? globalParameter(key)
            : currentGlyph()?.parameters.find((p) => p.dimension === key);
    }
    function sync() {
        const disabled = busy || profile === null || dimensions.length === 0
            || context.currentSecret() === null;
        for (const control of controls) {
            control.disabled = disabled;
        }
        const isGlyph = scope.value === "glyph";
        const glyph = currentGlyph();
        const parameter = selectedParameter();
        glyphSelect.disabled = disabled || !isGlyph
            || profile?.glyphs.length === 0;
        newGlyph.disabled = disabled || !isGlyph;
        addGlyph.disabled = disabled || !isGlyph || !newGlyph.value;
        removeGlyph.disabled = disabled || !isGlyph || glyph === undefined;
        addOverride.disabled = disabled || !isGlyph || glyph === undefined
            || parameter !== undefined;
        removeOverride.disabled = disabled || !isGlyph
            || parameter === undefined;
        for (const { number, slider } of fields) {
            number.disabled = disabled || parameter === undefined;
            slider.disabled = disabled || parameter === undefined;
        }
    }
    function refreshGlyphOptions() {
        glyphSelect.replaceChildren();
        for (const glyph of profile?.glyphs ?? []) {
            const option = document.createElement("option");
            option.value = glyph.grapheme;
            option.textContent = glyph.grapheme;
            glyphSelect.append(option);
        }
        const target = selectedGlyph;
        const glyphs = profile?.glyphs;
        const exists = glyphs?.some((glyph) => glyph.grapheme === target);
        if (!exists) {
            selectedGlyph = profile?.glyphs[0]?.grapheme ?? "";
        }
        glyphSelect.value = selectedGlyph;
    }
    function display() {
        const dim = selectedDimension();
        const parameter = selectedParameter();
        const inherited = scope.value === "glyph" && parameter === undefined;
        const effective = parameter ?? globalParameter(dimensionSelect.value);
        for (const { field, number, slider } of fields) {
            const value = effective === undefined ? ""
                : drafts.get(effective)?.[field] ?? String(effective[field]);
            const lower = field === "standard_deviation"
                ? 0 : dim?.minimum ?? 0;
            const upper = field === "standard_deviation"
                ? (dim?.maximum ?? 0) - (dim?.minimum ?? 0) : dim?.maximum ?? 0;
            number.min = String(lower);
            number.max = String(upper);
            slider.min = String(lower);
            slider.max = String(upper);
            number.value = value;
            slider.value = value;
        }
        const owner = scope.value === "global" ? "Global"
            : selectedGlyph ? `Exact glyph ${selectedGlyph}`
                : "No glyph selected";
        const kind = inherited
            ? " · inherits global (add override to edit)" : "";
        const values = effective === undefined ? "no statistics" :
            (`range ${fields[0]?.number.value ?? ""}…`
                + `${fields[1]?.number.value ?? ""}, `)
                + `mean ${fields[2]?.number.value ?? ""}, `
                + `median ${fields[3]?.number.value ?? ""}, `
                + `deviation ${fields[4]?.number.value ?? ""}`;
        preview.textContent = `${owner}: ${dim?.key ?? "unknown"} `
            + `(${dim?.unit ?? "unknown"}) · ${values}${kind}. `
            + "Statistical controls only; no handwritten strokes rendered.";
        sync();
    }
    function showProfile(next) {
        profile = next;
        drafts = new WeakMap();
        seed.value = String(next.seed);
        refreshGlyphOptions();
        display();
    }
    function isCurrent(secret, observed) {
        return context.currentSecret() === secret && generation === observed
            && !context.sessionSignal.aborted;
    }
    async function request(secret, endpoint, json) {
        return fetch(`.${endpoint}`, {
            method: json === undefined ? "GET" : "POST",
            headers: {
                Authorization: `Bearer ${secret}`,
                ...(json === undefined ? {} : {
                    "Content-Type": "application/json",
                }),
            },
            ...(json === undefined ? {} : { body: json }),
            credentials: "omit",
            cache: "no-store",
            mode: "same-origin",
            redirect: "error",
            referrerPolicy: "origin",
            signal: AbortSignal.any([
                context.sessionSignal, AbortSignal.timeout(30_000),
            ]),
        });
    }
    async function fetchJson(secret, endpoint, json) {
        const response = await request(secret, endpoint, json);
        if (response.status === 401) {
            context.unauthorized();
            throw new Error("Session expired");
        }
        if (!response.ok || !response.headers.get("content-type")
            ?.startsWith("application/json")) {
            throw new Error("Rust refused the handwriting profile");
        }
        const text = await response.text();
        boundedJson(text, endpoint === "/api/handwriting/dimensions"
            ? MAX_CATALOG_BYTES : MAX_JSON_BYTES);
        return text;
    }
    function candidate() {
        if (profile === null) {
            throw new Error("Missing native handwriting profile");
        }
        const seedValue = Number(seed.value);
        if (!seed.value || !Number.isSafeInteger(seedValue) || seedValue < 0) {
            throw new Error("Seed must be a nonnegative safe integer");
        }
        const mapParameters = (parameters) => (parameters.map((parameter) => {
            const next = { ...parameter };
            const draft = drafts.get(parameter);
            for (const field of FIELDS) {
                const raw = draft?.[field];
                if (raw !== undefined) {
                    const value = Number(raw);
                    const key = parameter.dimension;
                    const dim = dimensions.find((d) => d.key === key);
                    const lower = field === "standard_deviation"
                        ? 0 : dim?.minimum ?? 0;
                    const upper = field === "standard_deviation"
                        ? (dim?.maximum ?? 0) - (dim?.minimum ?? 0)
                        : dim?.maximum ?? 0;
                    if (!raw || !Number.isSafeInteger(value)
                        || value < lower || value > upper) {
                        throw new Error("Fix out-of-range statistical fields");
                    }
                    next[field] = value;
                }
            }
            return next;
        }));
        return {
            ...profile,
            seed: seedValue,
            globals: mapParameters(profile.globals),
            glyphs: profile.glyphs.map((glyph) => ({
                grapheme: glyph.grapheme,
                parameters: mapParameters(glyph.parameters),
            })),
        };
    }
    function offerDownload(json) {
        if (downloadUrl !== null) {
            URL.revokeObjectURL(downloadUrl);
        }
        const url = URL.createObjectURL(new Blob([json], {
            type: "application/json",
        }));
        downloadUrl = url;
        const link = document.createElement("a");
        link.href = url;
        link.download = "atrament-handwriting.json";
        link.rel = "noopener";
        link.click();
        window.setTimeout(() => {
            if (downloadUrl === url) {
                URL.revokeObjectURL(url);
                downloadUrl = null;
            }
        }, 60_000);
    }
    async function action(work) {
        const secret = context.currentSecret();
        if (secret === null || busy || profile === null) {
            return;
        }
        const observed = generation;
        busy = true;
        sync();
        try {
            await work(secret, observed);
        }
        catch {
            if (isCurrent(secret, observed)) {
                status.textContent = "Invalid statistics or unavailable "
                    + "Rust backend; "
                    + "no file saved.";
            }
        }
        finally {
            if (isCurrent(secret, observed)) {
                busy = false;
                sync();
            }
        }
    }
    async function start(secret) {
        generation += 1;
        const observed = generation;
        profile = null;
        dimensions = [];
        busy = true;
        sync();
        status.textContent = "Loading native handwriting bounds and profile…";
        try {
            const catalogEndpoint = "/api/handwriting/dimensions";
            const catalogJson = await fetchJson(secret, catalogEndpoint);
            if (!isCurrent(secret, observed))
                return;
            const loadedDimensions = parseCatalog(catalogJson);
            const profileEndpoint = "/api/handwriting/profile";
            const profileJson = await fetchJson(secret, profileEndpoint);
            if (!isCurrent(secret, observed))
                return;
            const loadedProfile = parseProfile(profileJson, loadedDimensions);
            dimensions = loadedDimensions;
            dimensionSelect.replaceChildren();
            for (const dimension of dimensions) {
                const option = document.createElement("option");
                option.value = dimension.key;
                option.textContent = `${dimension.key.replaceAll("_", " ")} `
                    + `(${dimension.unit})`;
                dimensionSelect.append(option);
            }
            dimensionSelect.value = dimensions[0]?.key ?? "";
            scope.value = "global";
            showProfile(loadedProfile);
            status.textContent = "Native statistics loaded · edit, validate, "
                + "and download JSON. No glyph rendering yet.";
        }
        catch {
            if (isCurrent(secret, observed)) {
                status.textContent = "Handwriting profile unavailable "
                    + "or seed too large "
                    + "for the browser; use the Rust CLI.";
            }
        }
        finally {
            if (isCurrent(secret, observed)) {
                busy = false;
                sync();
            }
        }
    }
    function reset() {
        generation += 1;
        busy = false;
        profile = null;
        dimensions = [];
        drafts = new WeakMap();
        selectedGlyph = "";
        upload.value = "";
        seed.value = "";
        newGlyph.value = "";
        dimensionSelect.replaceChildren();
        glyphSelect.replaceChildren();
        for (const { number, slider } of fields) {
            number.value = "";
            slider.value = "";
        }
        preview.textContent = "No handwriting session loaded.";
        status.textContent = "Requires a compatible backend session.";
        if (downloadUrl !== null) {
            URL.revokeObjectURL(downloadUrl);
            downloadUrl = null;
        }
        sync();
    }
    scope.addEventListener("change", display);
    dimensionSelect.addEventListener("change", display);
    glyphSelect.addEventListener("change", () => {
        selectedGlyph = glyphSelect.value;
        display();
    });
    newGlyph.addEventListener("input", sync);
    seed.addEventListener("input", () => {
        status.textContent = "Unsaved statistical changes.";
    });
    for (const { field, number, slider } of fields) {
        const update = (raw) => {
            const entry = selectedParameter();
            if (entry === undefined)
                return;
            let draft = drafts.get(entry);
            if (draft === undefined) {
                draft = {};
                drafts.set(entry, draft);
            }
            draft[field] = raw;
            number.value = raw;
            slider.value = raw;
            display();
            status.textContent = "Unsaved statistical changes. "
                + "Rust will validate on Save JSON.";
        };
        number.addEventListener("input", () => update(number.value));
        slider.addEventListener("input", () => update(slider.value));
    }
    addGlyph.addEventListener("click", () => {
        if (profile === null || scope.value !== "glyph")
            return;
        const grapheme = newGlyph.value;
        if (!grapheme || new TextEncoder().encode(grapheme).length > 64
            || profile.glyphs.length >= MAX_GLYPHS
            || profile.glyphs.some((item) => item.grapheme === grapheme)) {
            status.textContent = "Choose a new exact grapheme "
                + "(max 64 UTF-8 bytes).";
            return;
        }
        const inherited = globalParameter(dimensionSelect.value);
        if (inherited === undefined)
            return;
        profile.glyphs.push({ grapheme, parameters: [{ ...inherited }] });
        selectedGlyph = grapheme;
        newGlyph.value = "";
        refreshGlyphOptions();
        display();
        status.textContent = "Unsaved exact grapheme override; Rust validates "
            + "the grapheme when saving.";
    });
    removeGlyph.addEventListener("click", () => {
        if (profile === null || scope.value !== "glyph")
            return;
        const target = selectedGlyph;
        const glyphs = profile.glyphs;
        const remaining = glyphs.filter((item) => item.grapheme !== target);
        profile.glyphs = remaining;
        refreshGlyphOptions();
        display();
        status.textContent = "Unsaved glyph removal.";
    });
    addOverride.addEventListener("click", () => {
        const glyph = currentGlyph();
        const inherited = globalParameter(dimensionSelect.value);
        if (glyph === undefined || inherited === undefined
            || selectedParameter()) {
            return;
        }
        glyph.parameters.push({ ...inherited });
        display();
        status.textContent = "Unsaved glyph dimension override.";
    });
    removeOverride.addEventListener("click", () => {
        const glyph = currentGlyph();
        if (glyph === undefined || profile === null
            || !selectedParameter())
            return;
        const key = dimensionSelect.value;
        const parameters = glyph.parameters;
        const remaining = parameters.filter((entry) => entry.dimension !== key);
        glyph.parameters = remaining;
        if (glyph.parameters.length === 0) {
            profile.glyphs = profile.glyphs.filter((entry) => entry !== glyph);
            refreshGlyphOptions();
        }
        display();
        status.textContent = "Unsaved override removal; missing dimensions "
            + "inherit global statistics.";
    });
    resetButton.addEventListener("click", () => {
        void action(async (secret, observed) => {
            const json = await fetchJson(secret, "/api/handwriting/profile");
            if (!isCurrent(secret, observed))
                return;
            showProfile(parseProfile(json, dimensions));
            status.textContent = "Restored native uncalibrated defaults.";
        });
    });
    save.addEventListener("click", () => {
        void action(async (secret, observed) => {
            const proposal = JSON.stringify(candidate());
            if (new TextEncoder().encode(proposal).length > MAX_JSON_BYTES) {
                throw new Error("Statistics proposal exceeds 1 MiB");
            }
            const endpoint = "/api/handwriting/profile";
            const json = await fetchJson(secret, endpoint, proposal);
            if (!isCurrent(secret, observed))
                return;
            showProfile(parseProfile(json, dimensions));
            offerDownload(json);
            status.textContent = "Rust-validated handwriting JSON downloaded. "
                + "No glyph strokes have been rendered.";
        });
    });
    upload.addEventListener("change", () => {
        const file = upload.files?.item(0);
        upload.value = "";
        if (file === null || file === undefined)
            return;
        if (file.size > MAX_JSON_BYTES) {
            status.textContent = "Handwriting JSON exceeds 1 MiB.";
            return;
        }
        void action(async (secret, observed) => {
            const text = await file.text();
            if (!isCurrent(secret, observed))
                return;
            const endpoint = "/api/handwriting/profile";
            const json = await fetchJson(secret, endpoint, text);
            if (!isCurrent(secret, observed))
                return;
            showProfile(parseProfile(json, dimensions));
            status.textContent = "Rust-validated handwriting JSON loaded.";
        });
    });
    sync();
    return { start, reset };
}
