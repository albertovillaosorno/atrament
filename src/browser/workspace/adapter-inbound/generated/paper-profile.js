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
//   - Browser input widgets and file downloads for paper-only profiles.
// - Must-Not:
//   - Admit invalid geometry, compile PDFs, or render notebook pages.
//   - Persist profiles or credentials in browser storage.
// - Allows:
//   - Inputs: User fields and authenticated Rust paper-profile HTTP results.
//   - Outputs: Editable controls and explicit user-selected downloads.
//   - Side effects: DOM changes, file selection, Blob download and requests.
// - Split-When:
//   - An independent paper-profile editor application is implemented.
// - Merge-When:
//   - Shared browser workspace integration owns these controls.
// - Summary:
//   - Provides paper editing with Rust-owned PDF and transparent PNG export.
// - Description:
//   - Client-side values are proposals, never authoritative geometry.
// - Usage:
//   - Imported by the authenticated workspace after session handshake.
// - Defaults:
//   - Controls stay disabled until the backend supplies a profile.
//
const NUMERIC_CONTROLS = [
    ["width_um", "paper-width"],
    ["height_um", "paper-height"],
    ["spacing_um", "paper-spacing"],
    ["margin_top_um", "paper-upper"],
    ["margin_bottom_um", "paper-bottom"],
    ["margin_left_um", "paper-left"],
    ["margin_right_um", "paper-right"],
];
// Presets are ergonomic field suggestions. Rust validates each proposed size.
const SHEET_PRESETS = {
    a3: [297_000, 420_000],
    a4: [210_000, 297_000],
    a5: [148_000, 210_000],
    b5: [176_000, 250_000],
    letter: [215_900, 279_400],
    legal: [215_900, 355_600],
};
function required(id) {
    const element = document.getElementById(id);
    if (element === null) {
        throw new Error(`Missing paper editor control: ${id}`);
    }
    return element;
}
function isPaperStyle(value) {
    return value === "blank" || value === "grid"
        || value === "ruled" || value === "dot";
}
function readCanonicalProfile(json) {
    const parsed = JSON.parse(json);
    if (typeof parsed !== "object" || parsed === null) {
        throw new Error("Missing canonical paper profile");
    }
    const profile = parsed;
    if (Object.keys(profile).length !== 10
        || profile.schema !== "atrament.paper.v1"
        || typeof profile.style !== "string"
        || !isPaperStyle(profile.style)
        || typeof profile.line_color !== "string"
        || !/^#[0-9a-fA-F]{6}$/.test(profile.line_color)) {
        throw new Error("Invalid paper profile response");
    }
    for (const [key] of NUMERIC_CONTROLS) {
        if (!Number.isSafeInteger(profile[key]) || Number(profile[key]) < 0) {
            throw new Error("Invalid paper dimensions from backend");
        }
    }
    return profile;
}
export function createPaperProfileEditor(context) {
    const preset = required("paper-preset");
    const pattern = required("paper-pattern");
    const color = required("paper-color");
    const upload = required("paper-load");
    const resetButton = required("paper-reset");
    const saveButton = required("paper-save");
    const exportButton = required("paper-export");
    const exportPng = required("paper-export-png");
    const pngDpi = required("paper-png-dpi");
    const pngDpiRange = required("paper-png-dpi-range");
    const status = required("paper-status");
    const numbers = NUMERIC_CONTROLS.map(([key, id]) => [key, required(id)]);
    const sliders = NUMERIC_CONTROLS.map(([, id]) => (required(`${id}-range`)));
    const controls = [preset, pattern, color, upload, resetButton,
        saveButton, exportButton, exportPng, pngDpi, pngDpiRange,
        ...numbers.map(([, input]) => input), ...sliders];
    let admitted = null;
    let generation = 0;
    let busy = false;
    let downloadUrl = null;
    function setStatus(message) {
        status.textContent = message;
    }
    function isCurrent(secret, observed) {
        return generation === observed
            && context.currentSecret() === secret
            && !context.sessionSignal.aborted;
    }
    function syncControls() {
        const disabled = busy || admitted === null
            || context.currentSecret() === null;
        for (const control of controls) {
            control.disabled = disabled;
        }
    }
    function showProfile(profile) {
        admitted = profile;
        pattern.value = profile.style;
        color.value = profile.line_color;
        for (const [index, [key, input]] of numbers.entries()) {
            input.value = String(profile[key] / 1000);
            const slider = sliders[index];
            if (slider !== undefined) {
                slider.value = input.value;
            }
        }
        preset.value = Object.entries(SHEET_PRESETS).find(([, dimensions]) => dimensions[0] === profile.width_um
            && dimensions[1] === profile.height_um)?.[0] ?? "custom";
        syncControls();
    }
    function proposedJson() {
        if (admitted === null) {
            throw new Error("No backend paper profile loaded");
        }
        const candidate = { ...admitted };
        for (const [key, input] of numbers) {
            if (input.value === "" || !input.validity.valid) {
                throw new Error("Check the highlighted paper dimensions");
            }
            const mm = Number(input.value);
            if (!Number.isFinite(mm) || mm < 0) {
                throw new Error("Paper dimensions must be finite");
            }
            candidate[key] = Math.round(mm * 1000);
        }
        if (!isPaperStyle(pattern.value)) {
            throw new Error("Choose a supported paper pattern");
        }
        candidate.style = pattern.value;
        candidate.line_color = color.value;
        return JSON.stringify(candidate);
    }
    async function requestPaper(secret, endpoint, json, dpi) {
        return fetch(`.${endpoint}`, {
            method: json === undefined ? "GET" : "POST",
            headers: {
                Authorization: `Bearer ${secret}`,
                ...(json === undefined ? {} : {
                    "Content-Type": "application/json",
                }),
                ...(dpi === undefined ? {} : {
                    "X-Atrament-Png-Dpi": String(dpi),
                }),
            },
            ...(json === undefined ? {} : { body: json }),
            credentials: "omit",
            cache: "no-store",
            mode: "same-origin",
            redirect: "error",
            referrerPolicy: "origin",
            signal: AbortSignal.any([
                context.sessionSignal,
                AbortSignal.timeout(30_000),
            ]),
        });
    }
    async function canonicalize(secret, json) {
        const response = await requestPaper(secret, "/api/paper/profile", json);
        if (response.status === 401) {
            context.unauthorized();
            throw new Error("Session expired");
        }
        if (!response.ok) {
            throw new Error("Rust rejected the paper profile");
        }
        if (!response.headers.get("content-type")?.startsWith("application/json")) {
            throw new Error("Unexpected paper profile response");
        }
        const acceptedJson = await response.text();
        if (new TextEncoder().encode(acceptedJson).length > 4096) {
            throw new Error("Paper profile response exceeds limit");
        }
        return {
            json: acceptedJson,
            profile: readCanonicalProfile(acceptedJson),
        };
    }
    function offerDownload(blob, filename) {
        if (downloadUrl !== null) {
            URL.revokeObjectURL(downloadUrl);
        }
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
    }
    async function action(work) {
        const secret = context.currentSecret();
        if (secret === null || busy || admitted === null) {
            return;
        }
        const observed = generation;
        busy = true;
        syncControls();
        try {
            await work(secret, observed);
        }
        catch {
            if (isCurrent(secret, observed)) {
                setStatus("Paper action rejected or unavailable; no file saved.");
            }
        }
        finally {
            if (isCurrent(secret, observed)) {
                busy = false;
                syncControls();
            }
        }
    }
    async function start(secret) {
        generation += 1;
        const observed = generation;
        admitted = null;
        busy = true;
        syncControls();
        setStatus("Loading default paper profile from Rust…");
        try {
            const response = await requestPaper(secret, "/api/paper/profile");
            if (!isCurrent(secret, observed)) {
                return;
            }
            if (response.status === 401) {
                context.unauthorized();
                return;
            }
            if (!response.ok) {
                throw new Error("Paper profile request failed");
            }
            const json = await response.text();
            if (!isCurrent(secret, observed)) {
                return;
            }
            if (new TextEncoder().encode(json).length > 4096) {
                throw new Error("Oversized default paper profile");
            }
            showProfile(readCanonicalProfile(json));
            setStatus("Paper profile loaded · edit fields, then save or export.");
        }
        catch {
            if (isCurrent(secret, observed)) {
                setStatus("Paper profile unavailable; check the local backend.");
            }
        }
        finally {
            if (isCurrent(secret, observed)) {
                busy = false;
                syncControls();
            }
        }
    }
    function reset() {
        generation += 1;
        admitted = null;
        busy = false;
        upload.value = "";
        pngDpi.value = "150";
        pngDpiRange.value = "150";
        for (const [, input] of numbers) {
            input.value = "";
        }
        if (downloadUrl !== null) {
            URL.revokeObjectURL(downloadUrl);
            downloadUrl = null;
        }
        syncControls();
        setStatus("Requires a compatible backend session.");
    }
    preset.addEventListener("change", () => {
        const dimensions = SHEET_PRESETS[preset.value];
        if (dimensions !== undefined) {
            const width = required("paper-width");
            const height = required("paper-height");
            width.value = String(dimensions[0] / 1000);
            height.value = String(dimensions[1] / 1000);
            if (sliders[0] !== undefined && sliders[1] !== undefined) {
                sliders[0].value = width.value;
                sliders[1].value = height.value;
            }
        }
    });
    for (const [index, [key, input]] of numbers.entries()) {
        const slider = sliders[index];
        input.addEventListener("input", () => {
            if (slider !== undefined) {
                slider.value = input.value;
            }
            if (key === "width_um" || key === "height_um") {
                preset.value = "custom";
            }
        });
        slider?.addEventListener("input", () => {
            input.value = slider.value;
            if (key === "width_um" || key === "height_um") {
                preset.value = "custom";
            }
        });
    }
    pngDpi.addEventListener("input", () => {
        pngDpiRange.value = pngDpi.value;
    });
    pngDpiRange.addEventListener("input", () => {
        pngDpi.value = pngDpiRange.value;
    });
    resetButton.addEventListener("click", () => {
        void action(async (secret, observed) => {
            const response = await requestPaper(secret, "/api/paper/profile");
            if (!isCurrent(secret, observed)) {
                return;
            }
            if (response.status === 401) {
                context.unauthorized();
                return;
            }
            if (!response.ok) {
                throw new Error("Cannot reset paper profile");
            }
            const profile = readCanonicalProfile(await response.text());
            if (isCurrent(secret, observed)) {
                showProfile(profile);
                setStatus("Paper profile reset to the Rust default.");
            }
        });
    });
    saveButton.addEventListener("click", () => {
        void action(async (secret, observed) => {
            const result = await canonicalize(secret, proposedJson());
            if (isCurrent(secret, observed)) {
                showProfile(result.profile);
                offerDownload(new Blob([result.json], { type: "application/json" }), "atrament-paper.json");
                setStatus("Validated paper JSON downloaded.");
            }
        });
    });
    exportButton.addEventListener("click", () => {
        void action(async (secret, observed) => {
            const response = await requestPaper(secret, "/api/paper/pdf", proposedJson());
            if (!isCurrent(secret, observed)) {
                return;
            }
            if (response.status === 401) {
                context.unauthorized();
                return;
            }
            if (!response.ok || response.headers.get("content-type")
                !== "application/pdf") {
                throw new Error("Rust PDF export refused");
            }
            const pdf = await response.blob();
            if (isCurrent(secret, observed)) {
                offerDownload(pdf, "atrament-paper.pdf");
                setStatus("Rust PDF downloaded · paper only.");
            }
        });
    });
    exportPng.addEventListener("click", () => {
        void action(async (secret, observed) => {
            if (pngDpi.value === "" || !pngDpi.validity.valid) {
                throw new Error("Choose a valid PNG resolution");
            }
            const dpi = Number(pngDpi.value);
            if (!Number.isSafeInteger(dpi) || dpi < 72 || dpi > 300) {
                throw new Error("PNG DPI must be 72 through 300");
            }
            const response = await requestPaper(secret, "/api/paper/png", proposedJson(), dpi);
            if (!isCurrent(secret, observed)) {
                return;
            }
            if (response.status === 401) {
                context.unauthorized();
                return;
            }
            if (!response.ok || response.headers.get("content-type")
                !== "image/png") {
                throw new Error("Rust PNG export refused");
            }
            const png = await response.blob();
            if (isCurrent(secret, observed)) {
                offerDownload(png, "atrament-paper.png");
                setStatus("Rust transparent PNG downloaded · paper only.");
            }
        });
    });
    upload.addEventListener("change", () => {
        const file = upload.files?.item(0);
        upload.value = "";
        if (file === null || file === undefined) {
            return;
        }
        if (file.size > 4096) {
            setStatus("Paper JSON exceeds the 4096-byte limit.");
            return;
        }
        void action(async (secret, observed) => {
            const json = await file.text();
            if (!isCurrent(secret, observed)) {
                return;
            }
            const result = await canonicalize(secret, json);
            if (isCurrent(secret, observed)) {
                showProfile(result.profile);
                setStatus("Validated paper profile loaded from file.");
            }
        });
    });
    async function adoptJson(json) {
        let applied = false;
        await action(async (secret, observed) => {
            const result = await canonicalize(secret, json);
            if (isCurrent(secret, observed)) {
                showProfile(result.profile);
                setStatus("Notebook paper loaded for visual editing.");
                applied = true;
            }
        });
        return applied;
    }
    syncControls();
    return { start, reset, draftJson: proposedJson, adoptJson };
}
function readNotebook(json) {
    if (new TextEncoder().encode(json).length > 8192) {
        throw new Error("Notebook JSON exceeds the native limit");
    }
    const parsed = JSON.parse(json);
    if (typeof parsed !== "object" || parsed === null) {
        throw new Error("Missing canonical notebook profile");
    }
    const value = parsed;
    if (Object.keys(value).length !== 7
        || value.schema !== "atrament.notebook.v1"
        || !["none", "line", "box"].includes(String(value.date_style))
        || !["none", "top-left", "top-right"].includes(String(value.date_placement))
        || !Number.isSafeInteger(value.border_radius_um)
        || Number(value.border_radius_um) < 0
        || typeof value.page_numbers !== "boolean"
        || typeof value.printed_sheet !== "boolean") {
        throw new Error("Invalid canonical notebook metadata");
    }
    // The shared reader is only a display-shape check, not paper admission.
    readCanonicalProfile(JSON.stringify(value.paper));
    return value;
}
export function createNotebookProfileEditor(context) {
    const style = required("notebook-date-style");
    const placement = required("notebook-date-placement");
    const radius = required("notebook-radius");
    const slider = required("notebook-radius-range");
    const numbers = required("notebook-page-numbers");
    const printed = required("notebook-printed-sheet");
    const load = required("notebook-load");
    const usePaper = required("notebook-use-paper");
    const editPaper = required("notebook-edit-paper");
    const defaults = required("notebook-default");
    const save = required("notebook-save");
    const exportPdf = required("notebook-export-pdf");
    const exportPng = required("notebook-export-png");
    const pdfPages = required("notebook-pdf-pages");
    const pngDpi = required("notebook-png-dpi");
    const status = required("notebook-status");
    const controls = [style, placement, radius, slider, numbers, printed,
        load, usePaper, editPaper, defaults, save,
        exportPdf, exportPng, pdfPages, pngDpi];
    let admitted = null;
    let generation = 0;
    let busy = false;
    let downloadUrl = null;
    function active(secret, generationAtStart) {
        return generation === generationAtStart
            && context.currentSecret() === secret
            && !context.sessionSignal.aborted;
    }
    function sync() {
        for (const control of controls) {
            control.disabled = busy || admitted === null
                || context.currentSecret() === null;
        }
    }
    function display(profile) {
        admitted = profile;
        style.value = profile.date_style;
        placement.value = profile.date_placement;
        radius.value = String(profile.border_radius_um / 1000);
        slider.value = radius.value;
        numbers.checked = profile.page_numbers;
        printed.checked = profile.printed_sheet;
        sync();
    }
    async function request(secret, proposed) {
        const response = await fetch("./api/notebook/profile", {
            method: proposed === undefined ? "GET" : "POST",
            headers: {
                Authorization: `Bearer ${secret}`,
                ...(proposed === undefined ? {} : {
                    "Content-Type": "application/json",
                }),
            },
            ...(proposed === undefined ? {} : { body: proposed }),
            credentials: "omit",
            cache: "no-store",
            mode: "same-origin",
            redirect: "error",
            referrerPolicy: "origin",
            signal: AbortSignal.any([
                context.sessionSignal,
                AbortSignal.timeout(30_000),
            ]),
        });
        if (response.status === 401) {
            context.unauthorized();
            throw new Error("Session expired");
        }
        if (!response.ok || !response.headers.get("content-type")
            ?.startsWith("application/json")) {
            throw new Error("Rust notebook validation refused");
        }
        return response.text();
    }
    async function canonical(secret, proposed) {
        if (new TextEncoder().encode(proposed).length > 8192) {
            throw new Error("Notebook proposal exceeds limit");
        }
        const json = await request(secret, proposed);
        return { json, profile: readNotebook(json) };
    }
    function proposal() {
        if (admitted === null || radius.value === ""
            || !radius.validity.valid) {
            throw new Error("Notebook profile is not ready");
        }
        const millimetres = Number(radius.value);
        if (!Number.isFinite(millimetres) || millimetres < 0
            || !Number.isSafeInteger(Math.round(millimetres * 1000))) {
            throw new Error("Invalid border radius");
        }
        return {
            ...admitted,
            date_style: style.value,
            date_placement: placement.value,
            border_radius_um: Math.round(millimetres * 1000),
            page_numbers: numbers.checked,
            printed_sheet: printed.checked,
        };
    }
    function download(blob, filename) {
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
    }
    async function exportLayer(secret, format, amount, notebook) {
        return fetch(`./api/notebook/paper/${format}`, {
            method: "POST",
            headers: {
                Authorization: `Bearer ${secret}`,
                "Content-Type": "application/json",
                ...(format === "pdf" ? {
                    "X-Atrament-Pdf-Pages": String(amount),
                } : {
                    "X-Atrament-Png-Dpi": String(amount),
                }),
            },
            body: notebook,
            credentials: "omit",
            cache: "no-store",
            mode: "same-origin",
            redirect: "error",
            referrerPolicy: "origin",
            signal: AbortSignal.any([
                context.sessionSignal,
                AbortSignal.timeout(30_000),
            ]),
        });
    }
    function validExportNumber(input, minimum, maximum) {
        const number = Number(input.value);
        if (input.value === "" || !input.validity.valid
            || !Number.isSafeInteger(number)
            || number < minimum || number > maximum) {
            throw new Error("Invalid notebook paper export settings");
        }
        return number;
    }
    async function action(work) {
        const secret = context.currentSecret();
        if (busy || admitted === null || secret === null)
            return;
        const observed = generation;
        busy = true;
        sync();
        try {
            await work(secret, observed);
        }
        catch {
            if (active(secret, observed)) {
                status.textContent = "Notebook refused; nothing saved.";
            }
        }
        finally {
            if (active(secret, observed)) {
                busy = false;
                sync();
            }
        }
    }
    async function start(secret) {
        generation += 1;
        const observed = generation;
        admitted = null;
        busy = true;
        sync();
        status.textContent = "Loading native notebook defaults…";
        try {
            const profile = readNotebook(await request(secret));
            if (active(secret, observed)) {
                display(profile);
                status.textContent = "Edit metadata, then save JSON.";
            }
        }
        catch {
            if (active(secret, observed)) {
                status.textContent = "Notebook profile unavailable.";
            }
        }
        finally {
            if (active(secret, observed)) {
                busy = false;
                sync();
            }
        }
    }
    function reset() {
        generation += 1;
        admitted = null;
        busy = false;
        load.value = "";
        if (downloadUrl !== null) {
            URL.revokeObjectURL(downloadUrl);
            downloadUrl = null;
        }
        sync();
        status.textContent = "Requires a compatible backend session.";
    }
    radius.addEventListener("input", () => {
        slider.value = radius.value;
    });
    slider.addEventListener("input", () => {
        radius.value = slider.value;
    });
    editPaper.addEventListener("click", () => {
        void action(async (secret, observed) => {
            const paper = JSON.stringify(proposal().paper);
            const applied = await context.adoptPaper(paper);
            if (!active(secret, observed))
                return;
            if (!applied) {
                throw new Error("Paper editor unavailable");
            }
            status.textContent = "Paper loaded into sheet editor. "
                + "Use current paper to save its edits into this notebook.";
        });
    });
    usePaper.addEventListener("click", () => {
        void action(async (secret, observed) => {
            const paper = JSON.parse(context.paperDraft());
            const result = await canonical(secret, JSON.stringify({
                ...proposal(), paper,
            }));
            if (active(secret, observed)) {
                display(result.profile);
                status.textContent = "Current sheet settings validated.";
            }
        });
    });
    defaults.addEventListener("click", () => {
        void action(async (secret, observed) => {
            const profile = readNotebook(await request(secret));
            if (active(secret, observed)) {
                display(profile);
                status.textContent = "Notebook reset to Rust defaults.";
            }
        });
    });
    load.addEventListener("change", () => {
        const file = load.files?.item(0);
        load.value = "";
        if (file === null || file === undefined)
            return;
        if (file.size > 8192) {
            status.textContent = "Notebook JSON exceeds 8192 bytes.";
            return;
        }
        void action(async (secret, observed) => {
            const proposed = await file.text();
            if (!active(secret, observed))
                return;
            const result = await canonical(secret, proposed);
            if (active(secret, observed)) {
                display(result.profile);
                status.textContent = "Validated notebook JSON loaded.";
            }
        });
    });
    save.addEventListener("click", () => {
        void action(async (secret, observed) => {
            const result = await canonical(secret, JSON.stringify(proposal()));
            if (!active(secret, observed))
                return;
            display(result.profile);
            download(new Blob([result.json], {
                type: "application/json",
            }), "atrament-notebook.json");
            status.textContent = "Validated notebook JSON downloaded.";
        });
    });
    for (const [button, format, input, minimum, maximum] of [
        [exportPdf, "pdf", pdfPages, 1, 32],
        [exportPng, "png", pngDpi, 72, 300],
    ]) {
        button.addEventListener("click", () => {
            void action(async (secret, observed) => {
                const amount = validExportNumber(input, minimum, maximum);
                const proposed = JSON.stringify(proposal());
                const result = await canonical(secret, proposed);
                if (!active(secret, observed))
                    return;
                const body = result.json;
                const reply = await exportLayer(secret, format, amount, body);
                if (!active(secret, observed))
                    return;
                if (reply.status === 401) {
                    context.unauthorized();
                    return;
                }
                const mime = format === "pdf"
                    ? "application/pdf" : "image/png";
                if (!reply.ok
                    || reply.headers.get("content-type") !== mime) {
                    throw new Error("Rust notebook paper export refused");
                }
                const blob = await reply.blob();
                if (active(secret, observed)) {
                    display(result.profile);
                    download(blob, `atrament-notebook-paper.${format}`);
                    status.textContent = "Notebook paper layer downloaded "
                        + "· metadata and ink not rendered.";
                }
            });
        });
    }
    sync();
    return { start, reset };
}
