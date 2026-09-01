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
//   - Browser-only workspace interaction and clipboard presentation.
// - Must-Not:
//   - Validate or reinterpret backend-owned notebook data.
//   - Implement layout, rendering, diagnostics, or output compilation.
// - Allows:
//   - Inputs: User text entry and backend-presented prompt text.
//   - Outputs: Character counts, local viewport controls, and clipboard writes.
//   - Side effects: DOM updates, pointer capture, clipboard writes, and
//     bfcache reloads.
// - Split-When:
//   - Backend transport wiring needs an independently testable adapter.
// - Merge-When:
//   - Browser interaction no longer has independent ownership.
// - Summary:
//   - Keeps the TypeScript client intentionally thin.
// - Description:
//   - Presents backend authority without duplicating its domain contracts.
// - Usage:
//   - Loaded by the localhost workspace document.
// - Defaults:
//   - Performs no network request and persists no browser state.
//
function requireElement<T extends Element>(selector: string): T {
    const element = document.querySelector<T>(selector);
    if (element === null) {
        throw new Error(`Missing required workspace element: ${selector}`);
    }
    return element;
}

function createGraphemeSegmenter(): Intl.Segmenter | null {
    if (
        typeof Intl === "undefined"
        || typeof Intl.Segmenter !== "function"
    ) {
        return null;
    }
    try {
        return new Intl.Segmenter(undefined, { granularity: "grapheme" });
    } catch {
        return null;
    }
}

let graphemeSegmenter = createGraphemeSegmenter();

type TextCount = {
    count: number;
    unit: "character" | "code point";
};

function countText(value: string): TextCount {
    if (graphemeSegmenter !== null) {
        try {
            let count = 0;
            for (const _segment of graphemeSegmenter.segment(value)) {
                count += 1;
            }
            return { count, unit: "character" };
        } catch {
            graphemeSegmenter = null;
            // Fall through to the code-point count below.
        }
    }
    let count = 0;
    for (const _codePoint of value) {
        count += 1;
    }
    return { count, unit: "code point" };
}

function setTextIfChanged(element: HTMLElement, value: string): void {
    if (element.textContent?.trim() !== value) {
        element.textContent = value;
    }
}

function bindCharacterCount(
    input: HTMLTextAreaElement,
    output: HTMLElement,
): void {
    const update = (): void => {
        const { count, unit } = countText(input.value);
        const suffix = count === 1 ? unit : `${unit}s`;
        setTextIfChanged(output, `${count} ${suffix}`);
    };

    input.addEventListener("input", update);
    update();
}

const taskInput = requireElement<HTMLTextAreaElement>("#task-input");
const sourceInput = requireElement<HTMLTextAreaElement>("#source-input");
const candidateInput = requireElement<HTMLTextAreaElement>("#candidate-input");
const taskCount = requireElement<HTMLElement>("#task-count");
const sourceCount = requireElement<HTMLElement>("#source-count");
const candidateCount = requireElement<HTMLElement>("#candidate-count");
const promptOutput = requireElement<HTMLTextAreaElement>("#prompt-output");
const copyPrompt = requireElement<HTMLButtonElement>("#copy-prompt");
const copyStatus = requireElement<HTMLElement>("#copy-status");
const sessionStatus = requireElement<HTMLElement>("#session-status");

let copyGeneration = 0;
let clipboardWriteQueue: Promise<void> = Promise.resolve();

type ClipboardWrite = (text: string) => Promise<void>;

function getClipboardWrite(): ClipboardWrite | null {
    try {
        const clipboard = navigator.clipboard;
        const writeText = clipboard?.writeText;
        if (typeof writeText !== "function") {
            return null;
        }
        return writeText.bind(clipboard);
    } catch {
        return null;
    }
}

bindCharacterCount(taskInput, taskCount);
bindCharacterCount(sourceInput, sourceCount);
bindCharacterCount(candidateInput, candidateCount);

function syncPromptCopyState(): void {
    copyGeneration += 1;
    const available = promptOutput.value.length > 0;
    promptOutput.disabled = !available;
    copyPrompt.disabled = !available;
    if (!available) {
        setTextIfChanged(copyStatus, "Waiting for a prompt from the backend.");
    } else {
        setTextIfChanged(copyStatus, "");
    }
}

promptOutput.addEventListener("input", syncPromptCopyState);
syncPromptCopyState();

copyPrompt.addEventListener("click", (): void => {
    const prompt = promptOutput.value;
    const generation = ++copyGeneration;
    if (prompt.length === 0) {
        setTextIfChanged(copyStatus, "Waiting for a prompt from the backend.");
        return;
    }

    const writeClipboard = getClipboardWrite();
    if (writeClipboard === null) {
        setTextIfChanged(copyStatus, "Clipboard access is unavailable.");
        return;
    }

    setTextIfChanged(copyStatus, "Copying prompt…");
    const write = clipboardWriteQueue.then((): Promise<void> | void => {
        if (
            generation !== copyGeneration
            || promptOutput.value !== prompt
        ) {
            return;
        }
        return writeClipboard(prompt);
    });
    clipboardWriteQueue = write.catch((): void => {});
    void write.then(
        (): void => {
            if (
                generation === copyGeneration
                && promptOutput.value === prompt
            ) {
                setTextIfChanged(copyStatus, "Prompt copied.");
            }
        },
        (): void => {
            if (
                generation === copyGeneration
                && promptOutput.value === prompt
            ) {
                setTextIfChanged(copyStatus, "Clipboard write failed.");
            }
        },
    );
});


const divider = requireElement<HTMLElement>("#workspace-divider");
let dividerPointerCaptureAvailable =
    typeof divider.setPointerCapture === "function"
    && typeof divider.hasPointerCapture === "function"
    && typeof divider.releasePointerCapture === "function";

function syncDividerPointerCapability(): void {
    if (dividerPointerCaptureAvailable) {
        divider.setAttribute("data-pointer-drag", "");
    } else {
        divider.removeAttribute("data-pointer-drag");
    }
}

syncDividerPointerCapability();
const zoomOut = requireElement<HTMLButtonElement>("#zoom-out");
const zoomReset = requireElement<HTMLButtonElement>("#zoom-reset");
const zoomIn = requireElement<HTMLButtonElement>("#zoom-in");
const zoomStatus = requireElement<HTMLOutputElement>("#zoom-status");
const previewScale = requireElement<HTMLElement>("#preview-scale");
const sourceEditorTitle = requireElement<HTMLElement>("#llm-editor-title");
const sourcePanel = requireElement<HTMLElement>("#source-panel");
const previewPanel = requireElement<HTMLElement>("#preview-panel");
const pageStage = requireElement<HTMLElement>("#page-stage");
const workspace = requireElement<HTMLElement>(".workspace-grid");
function getCompactWorkspaceQuery(): MediaQueryList | null {
    if (typeof window.matchMedia !== "function") {
        return null;
    }
    try {
        return window.matchMedia("(max-width: 480px)");
    } catch {
        return null;
    }
}

function browserSupportsPreviewZoom(): boolean {
    if (typeof CSS === "undefined") {
        return false;
    }
    try {
        return typeof CSS.supports === "function"
            && CSS.supports("zoom", "1.1");
    } catch {
        return false;
    }
}

let compactWorkspace = getCompactWorkspaceQuery();
const supportsPreviewZoom = browserSupportsPreviewZoom();
let wideEditorShare = 46;
let activeDividerPointerId: number | null = null;
let activeDividerPointerOffsetX = 0;

function clearSessionText(): void {
    taskInput.value = "";
    sourceInput.value = "";
    promptOutput.value = "";
    candidateInput.value = "";
}

window.addEventListener("pagehide", (event): void => {
    copyGeneration += 1;
    if (activeDividerPointerId !== null) {
        releaseDividerPointer(activeDividerPointerId);
    }
    if (event.persisted) {
        clearSessionText();
        workspace.textContent = "";
    }
});

function resetLocalViewportState(): void {
    sourcePanel.scrollTop = 0;
    sourcePanel.scrollLeft = 0;
    previewPanel.scrollTop = 0;
    previewPanel.scrollLeft = 0;
    pageStage.scrollTop = 0;
    pageStage.scrollLeft = 0;
}

window.addEventListener("pageshow", (event): void => {
    if (event.persisted) {
        window.location.reload();
        return;
    }
    resetLocalViewportState();
});

function isCompactWorkspace(): boolean {
    return compactWorkspace?.matches ?? window.innerWidth <= 480;
}

function setEditorShare(percent: number): void {
    const compact = isCompactWorkspace();
    const minimum = compact ? 50 : 35;
    const maximum = compact ? 50 : 65;
    const clamped = Math.min(maximum, Math.max(minimum, percent));
    const share = Math.round(clamped * 10) / 10;
    const previewShare = Math.round((100 - share) * 10) / 10;
    if (!compact) {
        wideEditorShare = share;
    }
    document.documentElement.style.setProperty("--editor-track", `${share}fr`);
    document.documentElement.style.setProperty(
        "--preview-track",
        `${previewShare}fr`,
    );
    divider.setAttribute("aria-valuenow", String(share));
    divider.setAttribute(
        "aria-valuetext",
        `${share}% source, ${previewShare}% preview`,
    );
}

function disableDividerPointerCapture(): void {
    dividerPointerCaptureAvailable = false;
    syncDividerPointerCapability();
}

function dividerHasPointerCapture(pointerId: number): boolean {
    if (!dividerPointerCaptureAvailable) {
        return false;
    }
    try {
        return divider.hasPointerCapture(pointerId);
    } catch {
        disableDividerPointerCapture();
        return false;
    }
}

function releaseDividerPointer(pointerId: number): void {
    try {
        if (dividerHasPointerCapture(pointerId)) {
            divider.releasePointerCapture(pointerId);
        }
    } catch {
        disableDividerPointerCapture();
    }
    if (activeDividerPointerId === pointerId) {
        activeDividerPointerId = null;
        activeDividerPointerOffsetX = 0;
    }
}

function syncDividerAvailability(): void {
    if (isCompactWorkspace()) {
        if (activeDividerPointerId !== null) {
            releaseDividerPointer(activeDividerPointerId);
        }
        divider.setAttribute("aria-valuemin", "50");
        divider.setAttribute("aria-valuemax", "50");
        divider.setAttribute("aria-disabled", "true");
        divider.setAttribute("tabindex", "-1");
        setEditorShare(50);
        if (document.activeElement === divider) {
            sourceEditorTitle.focus();
        }
        return;
    }

    divider.setAttribute("aria-valuemin", "35");
    divider.setAttribute("aria-valuemax", "65");
    divider.removeAttribute("aria-disabled");
    divider.setAttribute("tabindex", "0");
    setEditorShare(wideEditorShare);
}

function shareFromPointer(clientX: number): number {
    const workspaceBounds = workspace.getBoundingClientRect();
    const dividerWidth = divider.getBoundingClientRect().width;
    const panelWidth = workspaceBounds.width - dividerWidth;
    const sourceWidth = clientX - workspaceBounds.left - dividerWidth / 2;
    return (sourceWidth / panelWidth) * 100;
}

divider.addEventListener("pointerdown", (event): void => {
    if (
        divider.getAttribute("aria-disabled") === "true"
        || activeDividerPointerId !== null
        || !event.isPrimary
        || event.button !== 0
    ) {
        return;
    }
    divider.focus();
    if (!dividerPointerCaptureAvailable) {
        return;
    }
    const dividerBounds = divider.getBoundingClientRect();
    const dividerCenter = dividerBounds.left + dividerBounds.width / 2;
    activeDividerPointerOffsetX = event.clientX - dividerCenter;
    try {
        divider.setPointerCapture(event.pointerId);
    } catch {
        disableDividerPointerCapture();
        return;
    }
    event.preventDefault();
    activeDividerPointerId = event.pointerId;
});

divider.addEventListener("click", (event): void => {
    if (
        dividerPointerCaptureAvailable
        || divider.getAttribute("aria-disabled") === "true"
        || event.detail === 0
    ) {
        return;
    }
    setEditorShare(shareFromPointer(event.clientX));
});

divider.addEventListener("pointermove", (event): void => {
    if (dividerHasPointerCapture(event.pointerId)) {
        setEditorShare(
            shareFromPointer(event.clientX - activeDividerPointerOffsetX),
        );
    }
});

function finishDividerPointer(event: PointerEvent): void {
    releaseDividerPointer(event.pointerId);
}

divider.addEventListener("pointerup", finishDividerPointer);
divider.addEventListener("pointercancel", finishDividerPointer);
divider.addEventListener("lostpointercapture", (event): void => {
    if (activeDividerPointerId === event.pointerId) {
        activeDividerPointerId = null;
        activeDividerPointerOffsetX = 0;
    }
});

divider.addEventListener("keydown", (event): void => {
    if (divider.getAttribute("aria-disabled") === "true") {
        return;
    }
    const current = Number(divider.getAttribute("aria-valuenow") ?? "46");
    if (event.key === "ArrowLeft") {
        event.preventDefault();
        setEditorShare(current - 2);
    } else if (event.key === "ArrowRight") {
        event.preventDefault();
        setEditorShare(current + 2);
    } else if (event.key === "Home") {
        event.preventDefault();
        setEditorShare(35);
    } else if (event.key === "End") {
        event.preventDefault();
        setEditorShare(65);
    }
});

function bindCompactWorkspaceChanges(): void {
    if (compactWorkspace !== null) {
        try {
            if (typeof compactWorkspace.addEventListener === "function") {
                compactWorkspace.addEventListener(
                    "change",
                    syncDividerAvailability,
                );
                return;
            }
        } catch {
            // Try the legacy listener below before falling back to resize.
        }
        try {
            if (typeof compactWorkspace.addListener === "function") {
                compactWorkspace.addListener(syncDividerAvailability);
                return;
            }
        } catch {
            // Fall through to the width-based resize listener below.
        }
    }
    compactWorkspace = null;
    window.addEventListener("resize", syncDividerAvailability);
}

bindCompactWorkspaceChanges();
syncDividerAvailability();

let previewZoom = 100;

function setPreviewZoom(percent: number): void {
    if (!supportsPreviewZoom) {
        previewZoom = 100;
        setTextIfChanged(zoomReset, "100%");
        setTextIfChanged(
            zoomStatus,
            "Preview zoom unavailable in this browser.",
        );
        setTextIfChanged(previewScale, "Preview · 100%");
        zoomOut.disabled = true;
        zoomReset.disabled = true;
        zoomIn.disabled = true;
        return;
    }

    previewZoom = Math.min(160, Math.max(60, percent));
    document.documentElement.style.setProperty(
        "--preview-zoom",
        String(previewZoom / 100),
    );
    setTextIfChanged(zoomReset, `${previewZoom}%`);
    setTextIfChanged(zoomStatus, `Preview zoom ${previewZoom}%`);
    setTextIfChanged(previewScale, `Preview · ${previewZoom}%`);
    zoomOut.disabled = previewZoom <= 60;
    zoomReset.disabled = previewZoom === 100;
    zoomIn.disabled = previewZoom >= 160;
}

zoomOut.addEventListener("click", (): void => {
    setPreviewZoom(previewZoom - 10);
});

zoomReset.addEventListener("click", (): void => {
    setPreviewZoom(100);
});

zoomIn.addEventListener("click", (): void => {
    setPreviewZoom(previewZoom + 10);
});

setPreviewZoom(100);
setTextIfChanged(sessionStatus, "Frontend ready · waiting for backend session");
