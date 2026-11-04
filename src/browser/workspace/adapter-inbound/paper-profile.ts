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

//! Thin browser form adapter for Rust paper-only profiles.

type PaperStyle = "blank" | "ruled" | "grid" | "dot";
type PaperProfile = {
    schema: "atrament.paper.v1";
    style: PaperStyle;
    width_um: number;
    height_um: number;
    spacing_um: number;
    margin_top_um: number;
    margin_bottom_um: number;
    margin_left_um: number;
    margin_right_um: number;
    line_color: string;
};

type NumericKey =
    "width_um" | "height_um" | "spacing_um"
    | "margin_top_um" | "margin_bottom_um"
    | "margin_left_um" | "margin_right_um";

const NUMERIC_CONTROLS = [
    ["width_um", "paper-width"],
    ["height_um", "paper-height"],
    ["spacing_um", "paper-spacing"],
    ["margin_top_um", "paper-upper"],
    ["margin_bottom_um", "paper-bottom"],
    ["margin_left_um", "paper-left"],
    ["margin_right_um", "paper-right"],
] as const satisfies ReadonlyArray<readonly [NumericKey, string]>;

// Presets are ergonomic field suggestions. Rust validates each proposed size.
const SHEET_PRESETS: Record<string, readonly [number, number]> = {
    a3: [297_000, 420_000],
    a4: [210_000, 297_000],
    a5: [148_000, 210_000],
    b5: [176_000, 250_000],
    letter: [215_900, 279_400],
    legal: [215_900, 355_600],
};

function required<T extends Element>(id: string): T {
    const element = document.getElementById(id);
    if (element === null) {
        throw new Error(`Missing paper editor control: ${id}`);
    }
    return element as unknown as T;
}

function isPaperStyle(value: string): value is PaperStyle {
    return value === "blank" || value === "grid"
        || value === "ruled" || value === "dot";
}

function readCanonicalProfile(json: string): PaperProfile {
    const parsed: unknown = JSON.parse(json);
    if (typeof parsed !== "object" || parsed === null) {
        throw new Error("Missing canonical paper profile");
    }
    const profile = parsed as Record<string, unknown>;
    if (
        Object.keys(profile).length !== 10
        || profile.schema !== "atrament.paper.v1"
        || typeof profile.style !== "string"
        || !isPaperStyle(profile.style)
        || typeof profile.line_color !== "string"
        || !/^#[0-9a-fA-F]{6}$/.test(profile.line_color)
    ) {
        throw new Error("Invalid paper profile response");
    }
    for (const [key] of NUMERIC_CONTROLS) {
        if (!Number.isSafeInteger(profile[key]) || Number(profile[key]) < 0) {
            throw new Error("Invalid paper dimensions from backend");
        }
    }
    return profile as PaperProfile;
}

type PaperEditorContext = {
    currentSecret: () => string | null;
    sessionSignal: AbortSignal;
    unauthorized: () => void;
};

type PaperEditor = {
    reset: () => void;
    start: (secret: string) => Promise<void>;
    draftJson?: () => string;
    adoptJson?: (json: string) => Promise<boolean>;
};

export function createPaperProfileEditor(context: PaperEditorContext): PaperEditor {
    const preset = required<HTMLSelectElement>("paper-preset");
    const pattern = required<HTMLSelectElement>("paper-pattern");
    const color = required<HTMLInputElement>("paper-color");
    const upload = required<HTMLInputElement>("paper-load");
    const resetButton = required<HTMLButtonElement>("paper-reset");
    const saveButton = required<HTMLButtonElement>("paper-save");
    const exportButton = required<HTMLButtonElement>("paper-export");
    const exportPng = required<HTMLButtonElement>("paper-export-png");
    const pngDpi = required<HTMLInputElement>("paper-png-dpi");
    const pngDpiRange = required<HTMLInputElement>("paper-png-dpi-range");
    const status = required<HTMLElement>("paper-status");
    const numbers = NUMERIC_CONTROLS.map(([key, id]) => (
        [key, required<HTMLInputElement>(id)] as const
    ));
    const sliders = NUMERIC_CONTROLS.map(([, id]) => (
        required<HTMLInputElement>(`${id}-range`)
    ));
    const controls = [preset, pattern, color, upload, resetButton,
        saveButton, exportButton, exportPng, pngDpi, pngDpiRange,
        ...numbers.map(([, input]) => input), ...sliders];
    let admitted: PaperProfile | null = null;
    let generation = 0;
    let busy = false;
    let downloadUrl: string | null = null;

    function setStatus(message: string): void {
        status.textContent = message;
    }

    function isCurrent(secret: string, observed: number): boolean {
        return generation === observed
            && context.currentSecret() === secret
            && !context.sessionSignal.aborted;
    }

    function syncControls(): void {
        const disabled = busy || admitted === null
            || context.currentSecret() === null;
        for (const control of controls) {
            control.disabled = disabled;
        }
    }

    function showProfile(profile: PaperProfile): void {
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
        preset.value = Object.entries(SHEET_PRESETS).find(
            ([, dimensions]) => dimensions[0] === profile.width_um
                && dimensions[1] === profile.height_um,
        )?.[0] ?? "custom";
        syncControls();
    }

    function proposedJson(): string {
        if (admitted === null) {
            throw new Error("No backend paper profile loaded");
        }
        const candidate: PaperProfile = { ...admitted };
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

    async function requestPaper(
        secret: string,
        endpoint: "/api/paper/profile" | "/api/paper/pdf"
            | "/api/paper/png",
        json?: string,
        dpi?: number,
    ): Promise<Response> {
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

    async function canonicalize(
        secret: string,
        json: string,
    ): Promise<{ profile: PaperProfile; json: string }> {
        const response = await requestPaper(
            secret, "/api/paper/profile", json,
        );
        if (response.status === 401) {
            context.unauthorized();
            throw new Error("Session expired");
        }
        if (!response.ok) {
            throw new Error("Rust rejected the paper profile");
        }
        if (!response.headers.get("content-type")?.startsWith(
            "application/json",
        )) {
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

    function offerDownload(blob: Blob, filename: string): void {
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

    async function action(
        work: (secret: string, observed: number) => Promise<void>,
    ): Promise<void> {
        const secret = context.currentSecret();
        if (secret === null || busy || admitted === null) {
            return;
        }
        const observed = generation;
        busy = true;
        syncControls();
        try {
            await work(secret, observed);
        } catch {
            if (isCurrent(secret, observed)) {
                setStatus("Paper action rejected or unavailable; no file saved.");
            }
        } finally {
            if (isCurrent(secret, observed)) {
                busy = false;
                syncControls();
            }
        }
    }

    async function start(secret: string): Promise<void> {
        generation += 1;
        const observed = generation;
        admitted = null;
        busy = true;
        syncControls();
        setStatus("Loading default paper profile from Rust…");
        try {
            const response = await requestPaper(
                secret, "/api/paper/profile",
            );
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
        } catch {
            if (isCurrent(secret, observed)) {
                setStatus("Paper profile unavailable; check the local backend.");
            }
        } finally {
            if (isCurrent(secret, observed)) {
                busy = false;
                syncControls();
            }
        }
    }

    function reset(): void {
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

    preset.addEventListener("change", (): void => {
        const dimensions = SHEET_PRESETS[preset.value];
        if (dimensions !== undefined) {
            const width = required<HTMLInputElement>("paper-width");
            const height = required<HTMLInputElement>("paper-height");
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
        input.addEventListener("input", (): void => {
            if (slider !== undefined) {
                slider.value = input.value;
            }
            if (key === "width_um" || key === "height_um") {
                preset.value = "custom";
            }
        });
        slider?.addEventListener("input", (): void => {
            input.value = slider.value;
            if (key === "width_um" || key === "height_um") {
                preset.value = "custom";
            }
        });
    }
    pngDpi.addEventListener("input", (): void => {
        pngDpiRange.value = pngDpi.value;
    });
    pngDpiRange.addEventListener("input", (): void => {
        pngDpi.value = pngDpiRange.value;
    });
    resetButton.addEventListener("click", (): void => {
        void action(async (secret, observed) => {
            const response = await requestPaper(
                secret, "/api/paper/profile",
            );
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
    saveButton.addEventListener("click", (): void => {
        void action(async (secret, observed) => {
            const result = await canonicalize(secret, proposedJson());
            if (isCurrent(secret, observed)) {
                showProfile(result.profile);
                offerDownload(
                    new Blob([result.json], { type: "application/json" }),
                    "atrament-paper.json",
                );
                setStatus("Validated paper JSON downloaded.");
            }
        });
    });
    exportButton.addEventListener("click", (): void => {
        void action(async (secret, observed) => {
            const response = await requestPaper(
                secret, "/api/paper/pdf", proposedJson(),
            );
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
    exportPng.addEventListener("click", (): void => {
        void action(async (secret, observed) => {
            if (pngDpi.value === "" || !pngDpi.validity.valid) {
                throw new Error("Choose a valid PNG resolution");
            }
            const dpi = Number(pngDpi.value);
            if (!Number.isSafeInteger(dpi) || dpi < 72 || dpi > 300) {
                throw new Error("PNG DPI must be 72 through 300");
            }
            const response = await requestPaper(
                secret, "/api/paper/png", proposedJson(), dpi,
            );
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
    upload.addEventListener("change", (): void => {
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
            const bytes = await file.arrayBuffer();
            if (bytes.byteLength > 4096) {
                throw new Error("Paper JSON exceeds the 4096-byte limit");
            }
            // File.text() replaces bad UTF-8 with U+FFFD before Rust sees it.
            const json = new TextDecoder("utf-8", {
                fatal: true, ignoreBOM: true,
            }).decode(bytes);
            if (json.startsWith("\uFEFF")) {
                throw new Error("Paper JSON forbids an initial UTF-8 BOM");
            }
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
    async function adoptJson(json: string): Promise<boolean> {
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

// Notebook metadata is a thin browser proposal; Rust owns its nested paper
// schema, date semantics, printable-area constraints and canonical JSON.
type NotebookProfile = {
    schema: "atrament.notebook.v1";
    paper: PaperProfile;
    date_style: "none" | "line" | "box";
    date_placement: "none" | "top-left" | "top-right";
    date_text: string;
    border_radius_um: number;
    page_numbers: boolean;
    printed_sheet: boolean;
};

type NotebookEditorContext = PaperEditorContext & {
    paperDraft: () => string;
    adoptPaper: (json: string) => Promise<boolean>;
};

function readNotebook(json: string): NotebookProfile {
    if (new TextEncoder().encode(json).length > 8192) {
        throw new Error("Notebook JSON exceeds the native limit");
    }
    const parsed: unknown = JSON.parse(json);
    if (typeof parsed !== "object" || parsed === null) {
        throw new Error("Missing canonical notebook profile");
    }
    const value = parsed as Record<string, unknown>;
    if (Object.keys(value).length !== 8
        || value.schema !== "atrament.notebook.v1"
        || !["none", "line", "box"].includes(String(value.date_style))
        || !["none", "top-left", "top-right"].includes(
            String(value.date_placement),
        )
        || typeof value.date_text !== "string"
        || !(
            value.date_text === ""
            || /^[0-9]{4}-[0-9]{2}-[0-9]{2}$/.test(value.date_text)
            || /^[0-9]{2}([/.])[0-9]{2}\1[0-9]{4}$/.test(value.date_text)
        )
        || value.date_text.length > 10
        || !Number.isSafeInteger(value.border_radius_um)
        || Number(value.border_radius_um) < 0
        || typeof value.page_numbers !== "boolean"
        || typeof value.printed_sheet !== "boolean"
    ) {
        throw new Error("Invalid canonical notebook metadata");
    }
    // The shared reader is only a display-shape check, not paper admission.
    readCanonicalProfile(JSON.stringify(value.paper));
    return value as NotebookProfile;
}

export function createNotebookProfileEditor(
    context: NotebookEditorContext,
): PaperEditor {
    const style = required<HTMLSelectElement>("notebook-date-style");
    const placement = required<HTMLSelectElement>("notebook-date-placement");
    const dateText = required<HTMLInputElement>("notebook-date-text");
    const radius = required<HTMLInputElement>("notebook-radius");
    const slider = required<HTMLInputElement>("notebook-radius-range");
    const numbers = required<HTMLInputElement>("notebook-page-numbers");
    const printed = required<HTMLInputElement>("notebook-printed-sheet");
    const load = required<HTMLInputElement>("notebook-load");
    const usePaper = required<HTMLButtonElement>("notebook-use-paper");
    const editPaper = required<HTMLButtonElement>("notebook-edit-paper");
    const defaults = required<HTMLButtonElement>("notebook-default");
    const save = required<HTMLButtonElement>("notebook-save");
    const localName = required<HTMLInputElement>("notebook-local-name");
    const localLoad = required<HTMLButtonElement>("notebook-local-load");
    const localSave = required<HTMLButtonElement>("notebook-local-save");
    const exportPdf = required<HTMLButtonElement>("notebook-export-pdf");
    const exportPng = required<HTMLButtonElement>("notebook-export-png");
    const previewButton = required<HTMLButtonElement>("notebook-preview");
    const previewFrame = required<HTMLElement>("notebook-preview-frame");
    const previewImage = required<HTMLImageElement>("notebook-preview-image");
    const pdfPages = required<HTMLInputElement>("notebook-pdf-pages");
    const pngDpi = required<HTMLInputElement>("notebook-png-dpi");
    const pngPage = required<HTMLInputElement>("notebook-png-page");
    const status = required<HTMLElement>("notebook-status");
    const controls = [style, placement, dateText, radius, slider, numbers,
        printed, load, usePaper, editPaper, defaults, save,
        localName, localLoad, localSave,
        exportPdf, exportPng, previewButton, pdfPages, pngDpi, pngPage];
    let admitted: NotebookProfile | null = null;
    let generation = 0;
    let busy = false;
    let downloadUrl: string | null = null;
    let previewUrl: string | null = null;
    let trackedName: string | null = null;
    let trackedRevision = "new";

    function clearTracking(): void {
        trackedName = null;
        trackedRevision = "new";
    }

    function selectedName(): string {
        const value = localName.value;
        if (value.length > 64 || !/^[A-Za-z0-9_-][A-Za-z0-9_.-]*\.json$/.test(value)
            || value.includes("..")) {
            throw new Error("Choose a simple .json filename (max 64 characters)");
        }
        return value;
    }

    function validRevision(value: unknown): value is string {
        return typeof value === "string"
            && /^sha256:[0-9a-f]{64}$/.test(value);
    }

    function clearPreview(): void {
        previewFrame.hidden = true;
        previewImage.removeAttribute("src");
        if (previewUrl !== null) {
            URL.revokeObjectURL(previewUrl);
            previewUrl = null;
        }
    }

    function active(secret: string, generationAtStart: number): boolean {
        return generation === generationAtStart
            && context.currentSecret() === secret
            && !context.sessionSignal.aborted;
    }

    function sync(): void {
        for (const control of controls) {
            control.disabled = busy || admitted === null
                || context.currentSecret() === null;
        }
    }

    function display(profile: NotebookProfile): void {
        clearPreview();
        admitted = profile;
        style.value = profile.date_style;
        placement.value = profile.date_placement;
        dateText.value = profile.date_text;
        radius.value = String(profile.border_radius_um / 1000);
        slider.value = radius.value;
        numbers.checked = profile.page_numbers;
        printed.checked = profile.printed_sheet;
        sync();
    }

    async function request(secret: string, proposed?: string): Promise<string> {
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

    async function fileRequest(
        secret: string, name: string, json?: string, revision = "new",
    ): Promise<{ revision: string; profile?: unknown }> {
        const response = await fetch("./api/notebook/file", {
            method: json === undefined ? "GET" : "POST",
            headers: {
                Authorization: `Bearer ${secret}`,
                "X-Atrament-Profile-Name": name,
                ...(json === undefined ? {} : {
                    "Content-Type": "application/json",
                    "X-Atrament-Profile-Revision": revision,
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
        if (response.status === 401) {
            context.unauthorized();
            throw new Error("Session expired");
        }
        if (response.status === 409) throw new Error("stale-file");
        if (response.status === 503) throw new Error("disabled-file");
        if (!response.ok || !response.headers.get("content-type")
            ?.startsWith("application/json")) {
            throw new Error("Rust file operation refused");
        }
        const value: unknown = await response.json();
        if (typeof value !== "object" || value === null
            || !validRevision((value as {revision?: unknown}).revision)) {
            throw new Error("Invalid native file response");
        }
        return value as { revision: string; profile?: unknown };
    }

    async function canonical(secret: string, proposed: string):
        Promise<{ json: string; profile: NotebookProfile }> {
        if (new TextEncoder().encode(proposed).length > 8192) {
            throw new Error("Notebook proposal exceeds limit");
        }
        const json = await request(secret, proposed);
        return { json, profile: readNotebook(json) };
    }

    function proposal(): NotebookProfile {
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
            date_style: style.value as NotebookProfile["date_style"],
            date_placement: placement.value as NotebookProfile["date_placement"],
            date_text: dateText.value,
            border_radius_um: Math.round(millimetres * 1000),
            page_numbers: numbers.checked,
            printed_sheet: printed.checked,
        };
    }

    function download(blob: Blob, filename: string): void {
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
    }

    async function post(
        secret: string,
        format: "pdf" | "png",
        amount: number,
        notebook: string,
        page: number,
    ): Promise<Response> {
        return fetch(`./api/notebook/paper/${format}`, {
            method: "POST",
            headers: {
                Authorization: `Bearer ${secret}`,
                "Content-Type": "application/json",
                ...(format === "pdf" ? {
                    "X-Atrament-Pdf-Pages": String(amount),
                } : {
                    "X-Atrament-Png-Dpi": String(amount),
                    "X-Atrament-Png-Page": String(page),
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

    function validExportNumber(
        input: HTMLInputElement, minimum: number, maximum: number,
    ): number {
        const number = Number(input.value);
        if (input.value === "" || !input.validity.valid
            || !Number.isSafeInteger(number)
            || number < minimum || number > maximum) {
            throw new Error("Invalid notebook paper export settings");
        }
        return number;
    }

    async function action(
        work: (secret: string, observed: number) => Promise<void>,
    ): Promise<void> {
        const secret = context.currentSecret();
        if (busy || admitted === null || secret === null) return;
        const observed = generation;
        busy = true;
        sync();
        try {
            await work(secret, observed);
        } catch (error) {
            if (active(secret, observed)) {
                status.textContent = error instanceof Error
                    && error.message === "stale-file"
                    ? "Local notebook changed; reopen before saving. No overwrite."
                    : error instanceof Error && error.message === "disabled-file"
                        ? "Local files disabled; launch Rust with --profiles-dir."
                        : "Notebook refused; nothing saved.";
            }
        } finally {
            if (active(secret, observed)) {
                busy = false;
                sync();
            }
        }
    }

    async function start(secret: string): Promise<void> {
        generation += 1;
        const observed = generation;
        clearPreview();
        clearTracking();
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
        } catch {
            if (active(secret, observed)) {
                status.textContent = "Notebook profile unavailable.";
            }
        } finally {
            if (active(secret, observed)) {
                busy = false;
                sync();
            }
        }
    }

    function reset(): void {
        generation += 1;
        clearPreview();
        clearTracking();
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

    radius.addEventListener("input", (): void => {
        slider.value = radius.value;
        clearPreview();
    });
    slider.addEventListener("input", (): void => {
        radius.value = slider.value;
        clearPreview();
    });
    for (const control of [style, placement, dateText, numbers, printed,
        pngDpi, pngPage]) {
        control.addEventListener("input", clearPreview);
    }
    editPaper.addEventListener("click", (): void => {
        void action(async (secret, observed) => {
            const paper = JSON.stringify(proposal().paper);
            const applied = await context.adoptPaper(paper);
            if (!active(secret, observed)) return;
            if (!applied) {
                throw new Error("Paper editor unavailable");
            }
            status.textContent = "Paper loaded into sheet editor. "
                + "Use current paper to save its edits into this notebook.";
        });
    });
    usePaper.addEventListener("click", (): void => {
        void action(async (secret, observed) => {
            const paper = JSON.parse(context.paperDraft()) as PaperProfile;
            const result = await canonical(secret, JSON.stringify({
                ...proposal(), paper,
            }));
            if (active(secret, observed)) {
                display(result.profile);
                status.textContent = "Current sheet settings validated.";
            }
        });
    });
    localName.addEventListener("input", clearTracking);
    localLoad.addEventListener("click", (): void => {
        void action(async (secret, observed) => {
            const name = selectedName();
            const result = await fileRequest(secret, name);
            if (!active(secret, observed)) return;
            const canonicalProfile = await canonical(
                secret, JSON.stringify(result.profile),
            );
            if (!active(secret, observed)) return;
            display(canonicalProfile.profile);
            trackedName = name;
            trackedRevision = result.revision;
            status.textContent = "Notebook reopened from local Rust profile file.";
        });
    });
    localSave.addEventListener("click", (): void => {
        void action(async (secret, observed) => {
            const name = selectedName();
            const canonicalProfile = await canonical(
                secret, JSON.stringify(proposal()),
            );
            if (!active(secret, observed)) return;
            const revision = trackedName === name ? trackedRevision : "new";
            const result = await fileRequest(
                secret, name, canonicalProfile.json, revision,
            );
            if (!active(secret, observed)) return;
            display(canonicalProfile.profile);
            trackedName = name;
            trackedRevision = result.revision;
            status.textContent = "Native notebook saved in selected local directory.";
        });
    });
    defaults.addEventListener("click", (): void => {
        void action(async (secret, observed) => {
            const profile = readNotebook(await request(secret));
            if (active(secret, observed)) {
                display(profile);
                clearTracking();
                status.textContent = "Notebook reset to Rust defaults.";
            }
        });
    });
    load.addEventListener("change", (): void => {
        const file = load.files?.item(0);
        load.value = "";
        if (file === null || file === undefined) return;
        if (file.size > 8192) {
            status.textContent = "Notebook JSON exceeds 8192 bytes.";
            return;
        }
        void action(async (secret, observed) => {
            const bytes = await file.arrayBuffer();
            if (bytes.byteLength > 8192) {
                throw new Error("Notebook JSON exceeds 8192 bytes");
            }
            const proposed = new TextDecoder("utf-8", {
                fatal: true, ignoreBOM: true,
            }).decode(bytes);
            if (proposed.startsWith("\uFEFF")) {
                throw new Error("Notebook JSON forbids an initial UTF-8 BOM");
            }
            if (!active(secret, observed)) return;
            const result = await canonical(secret, proposed);
            if (active(secret, observed)) {
                display(result.profile);
                clearTracking();
                status.textContent = "Validated notebook JSON loaded.";
            }
        });
    });
    save.addEventListener("click", (): void => {
        void action(async (secret, observed) => {
            const result = await canonical(secret, JSON.stringify(proposal()));
            if (!active(secret, observed)) return;
            display(result.profile);
            download(new Blob([result.json], {
                type: "application/json",
            }), "atrament-notebook.json");
            status.textContent = "Validated notebook JSON downloaded.";
        });
    });
    previewButton.addEventListener("click", (): void => {
        void action(async (secret, observed) => {
            const dpi = validExportNumber(pngDpi, 72, 300);
            const page = validExportNumber(pngPage, 1, 32);
            const result = await canonical(
                secret, JSON.stringify(proposal()),
            );
            if (!active(secret, observed)) return;
            const reply = await post(
                secret, "png", dpi, result.json, page,
            );
            if (!active(secret, observed)) return;
            if (reply.status === 401) {
                context.unauthorized();
                return;
            }
            if (!reply.ok
                || reply.headers.get("content-type") !== "image/png") {
                throw new Error("Rust notebook PNG preview refused");
            }
            const blob = await reply.blob();
            if (active(secret, observed)) {
                display(result.profile);
                previewUrl = URL.createObjectURL(blob);
                previewImage.alt = `Rendered notebook paper, page ${page}`;
                previewImage.src = previewUrl;
                previewFrame.hidden = false;
                status.textContent = `Rendered PNG preview for page ${page}.`;
            }
        });
    });
    for (const [button, format, input, minimum, maximum] of [
        [exportPdf, "pdf", pdfPages, 1, 32],
        [exportPng, "png", pngDpi, 72, 300],
    ] as const) {
        button.addEventListener("click", (): void => {
            void action(async (secret, observed) => {
                const amount = validExportNumber(input, minimum, maximum);
                const page = format === "png"
                    ? validExportNumber(pngPage, 1, 32) : 1;
                const proposed = JSON.stringify(proposal());
                const result = await canonical(secret, proposed);
                if (!active(secret, observed)) return;
                const body = result.json;
                const reply = await post(
                    secret, format, amount, body, page,
                );
                if (!active(secret, observed)) return;
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
                    const filename = format === "png" && page !== 1
                        ? `atrament-notebook-page-${page}.png`
                        : `atrament-notebook-paper.${format}`;
                    download(blob, filename);
                    status.textContent = "Notebook paper, optional printed"
                        + " numeric date, borders and page numbers downloaded ·"
                        + " no handwriting or images."
                        + (format === "png" ? ` PNG sheet ${page}.` : "");
                }
            });
        });
    }
    sync();
    return { start, reset };
}
