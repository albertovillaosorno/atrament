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
    try {
        if (
            typeof Intl === "undefined"
            || typeof Intl.Segmenter !== "function"
        ) {
            return null;
        }
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

function setAttributeIfChanged(
    element: Element,
    name: string,
    value: string,
): void {
    if (element.getAttribute(name) !== value) {
        element.setAttribute(name, value);
    }
}

function updateCharacterCount(
    input: HTMLTextAreaElement,
    output: HTMLElement,
): void {
    const { count, unit } = countText(input.value);
    const suffix = count === 1 ? unit : `${unit}s`;
    setTextIfChanged(output, `${count} ${suffix}`);
}

function bindCharacterCount(
    input: HTMLTextAreaElement,
    output: HTMLElement,
): void {
    input.addEventListener("input", (): void => {
        updateCharacterCount(input, output);
    });
    updateCharacterCount(input, output);
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

type ClipboardWrite = (text: string) => unknown;

type ClipboardRequest = {
    generation: number;
    prompt: string;
    write: ClipboardWrite;
};

let copyGeneration = 0;
let clipboardWriteInFlight = false;
let activeClipboardWrite: ClipboardRequest | null = null;
let pendingClipboardWrite: ClipboardRequest | null = null;
let presentedPromptValue = promptOutput.value;

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

function invalidateClipboardRequests(): number {
    copyGeneration += 1;
    pendingClipboardWrite = null;
    if (activeClipboardWrite !== null) {
        activeClipboardWrite.prompt = "";
    }
    return copyGeneration;
}

function syncPromptCopyState(): void {
    const prompt = promptOutput.value;
    const changed = prompt !== presentedPromptValue;
    if (changed) {
        presentedPromptValue = prompt;
        invalidateClipboardRequests();
    }
    const available = prompt.length > 0;
    promptOutput.disabled = !available;
    copyPrompt.disabled = !available;
    if (!available) {
        setTextIfChanged(copyStatus, "Waiting for a prompt from the backend.");
    } else if (changed) {
        setTextIfChanged(copyStatus, "");
    }
}

promptOutput.addEventListener("input", syncPromptCopyState);
syncPromptCopyState();

function finishClipboardWrite(
    request: ClipboardRequest,
    succeeded: boolean,
): void {
    if (activeClipboardWrite !== request) {
        return;
    }
    clipboardWriteInFlight = false;
    activeClipboardWrite = null;
    if (
        request.generation === copyGeneration
        && promptOutput.value === request.prompt
    ) {
        setTextIfChanged(
            copyStatus,
            succeeded ? "Prompt copied." : "Clipboard write failed.",
        );
    }
    drainClipboardWrite();
}

function drainClipboardWrite(): void {
    if (clipboardWriteInFlight || pendingClipboardWrite === null) {
        return;
    }
    const request = pendingClipboardWrite;
    pendingClipboardWrite = null;
    if (
        request.generation !== copyGeneration
        || promptOutput.value !== request.prompt
    ) {
        drainClipboardWrite();
        return;
    }

    clipboardWriteInFlight = true;
    activeClipboardWrite = request;
    try {
        const write = request.write(request.prompt);
        if (
            write === null
            || (typeof write !== "object" && typeof write !== "function")
        ) {
            finishClipboardWrite(request, false);
            return;
        }
        const then = (write as { then?: unknown }).then;
        if (typeof then !== "function") {
            finishClipboardWrite(request, false);
            return;
        }
        then.call(
            write,
            (): void => finishClipboardWrite(request, true),
            (): void => finishClipboardWrite(request, false),
        );
    } catch {
        finishClipboardWrite(request, false);
    }
}

copyPrompt.addEventListener("click", (): void => {
    const prompt = promptOutput.value;
    if (prompt.length === 0) {
        pendingClipboardWrite = null;
        setTextIfChanged(copyStatus, "Waiting for a prompt from the backend.");
        return;
    }
    if (
        (
            activeClipboardWrite?.generation === copyGeneration
            && activeClipboardWrite.prompt === prompt
        )
        || (
            pendingClipboardWrite?.generation === copyGeneration
            && pendingClipboardWrite.prompt === prompt
        )
    ) {
        setTextIfChanged(copyStatus, "Copying prompt…");
        return;
    }

    const generation = invalidateClipboardRequests();
    const writeClipboard = getClipboardWrite();
    if (writeClipboard === null) {
        pendingClipboardWrite = null;
        setTextIfChanged(copyStatus, "Clipboard access is unavailable.");
        return;
    }

    setTextIfChanged(copyStatus, "Copying prompt…");
    pendingClipboardWrite = {
        generation,
        prompt,
        write: writeClipboard,
    };
    drainClipboardWrite();
});


const divider = requireElement<HTMLElement>("#workspace-divider");

function browserSupportsDividerPointerCapture(): boolean {
    try {
        return typeof divider.setPointerCapture === "function"
            && typeof divider.hasPointerCapture === "function"
            && typeof divider.releasePointerCapture === "function";
    } catch {
        return false;
    }
}

let dividerPointerCaptureAvailable = browserSupportsDividerPointerCapture();

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
    try {
        if (typeof window.matchMedia !== "function") {
            return null;
        }
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
let compactWorkspaceResizeBound = false;
const supportsPreviewZoom = browserSupportsPreviewZoom();
let wideEditorShare = 46;
let activeDividerPointerId: number | null = null;
let activeDividerPointerOffsetX = 0;

function clearSessionText(): void {
    taskInput.value = "";
    sourceInput.value = "";
    promptOutput.value = "";
    presentedPromptValue = "";
    candidateInput.value = "";
    taskInput.disabled = true;
    sourceInput.disabled = true;
    promptOutput.disabled = true;
    candidateInput.disabled = true;
    copyPrompt.disabled = true;
    setTextIfChanged(copyStatus, "Waiting for a prompt from the backend.");
    updateCharacterCount(taskInput, taskCount);
    updateCharacterCount(sourceInput, sourceCount);
    updateCharacterCount(candidateInput, candidateCount);
}

window.addEventListener("pagehide", (event): void => {
    invalidateClipboardRequests();
    if (activeDividerPointerId !== null) {
        releaseDividerPointer(activeDividerPointerId);
    }
    clearSessionText();
    if (event.persisted) {
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

function discardLocalNavigationFragment(): boolean {
    const hash = window.location.hash;
    if (hash === "") {
        return false;
    }
    let targetId: string;
    try {
        targetId = decodeURIComponent(hash.slice(1));
    } catch {
        return false;
    }
    if (document.getElementById(targetId) === null) {
        return false;
    }
    try {
        window.history.replaceState(
            window.history.state,
            "",
            `${window.location.pathname}${window.location.search}`,
        );
    } catch {
        // The delayed reset below still contains late fragment scrolling.
    }
    return true;
}

window.addEventListener("pageshow", (event): void => {
    if (event.persisted) {
        window.location.reload();
        return;
    }
    const hadLocalNavigationFragment = discardLocalNavigationFragment();
    resetLocalViewportState();
    if (hadLocalNavigationFragment) {
        window.setTimeout(resetLocalViewportState, 0);
    }
});

function isCompactWorkspace(): boolean {
    try {
        return compactWorkspace?.matches ?? window.innerWidth <= 480;
    } catch {
        compactWorkspace = null;
        bindWidthResize();
        return window.innerWidth <= 480;
    }
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
    setAttributeIfChanged(divider, "aria-valuenow", String(share));
    setAttributeIfChanged(
        divider,
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
        setAttributeIfChanged(divider, "aria-valuemin", "50");
        setAttributeIfChanged(divider, "aria-valuemax", "50");
        setAttributeIfChanged(divider, "aria-disabled", "true");
        setAttributeIfChanged(divider, "tabindex", "-1");
        setEditorShare(50);
        if (document.activeElement === divider) {
            sourceEditorTitle.focus();
        }
        return;
    }

    setAttributeIfChanged(divider, "aria-valuemin", "35");
    setAttributeIfChanged(divider, "aria-valuemax", "65");
    divider.removeAttribute("aria-disabled");
    setAttributeIfChanged(divider, "tabindex", "0");
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
    if (!dividerHasPointerCapture(event.pointerId)) {
        disableDividerPointerCapture();
        activeDividerPointerOffsetX = 0;
        return;
    }
    divider.focus();
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
    divider.focus();
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
window.addEventListener("blur", (): void => {
    if (activeDividerPointerId !== null) {
        releaseDividerPointer(activeDividerPointerId);
    }
});
document.addEventListener("visibilitychange", (): void => {
    if (
        document.visibilityState === "hidden"
        && activeDividerPointerId !== null
    ) {
        releaseDividerPointer(activeDividerPointerId);
    }
});
divider.addEventListener("lostpointercapture", (event): void => {
    if (activeDividerPointerId === event.pointerId) {
        activeDividerPointerId = null;
        activeDividerPointerOffsetX = 0;
    }
});

divider.addEventListener("keydown", (event): void => {
    if (
        divider.getAttribute("aria-disabled") === "true"
        || event.altKey
        || event.ctrlKey
        || event.metaKey
        || event.shiftKey
    ) {
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

function handleViewportResize(): void {
    if (activeDividerPointerId !== null) {
        releaseDividerPointer(activeDividerPointerId);
    }
    syncDividerAvailability();
}

function bindWidthResize(): void {
    if (!compactWorkspaceResizeBound) {
        window.addEventListener("resize", handleViewportResize);
        compactWorkspaceResizeBound = true;
    }
}

function bindCompactWorkspaceChanges(): void {
    bindWidthResize();
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
    bindWidthResize();
}

bindCompactWorkspaceChanges();
syncDividerAvailability();

let previewZoom = 100;

function setPreviewZoom(percent: number): void {
    const previousZoom = previewZoom;
    const focusedControl = document.activeElement;
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
    if (
        (focusedControl === zoomOut && zoomOut.disabled)
        || (focusedControl === zoomIn && zoomIn.disabled)
    ) {
        zoomReset.focus();
    } else if (focusedControl === zoomReset && zoomReset.disabled) {
        if (previousZoom > 100) {
            zoomIn.focus();
        } else if (previousZoom < 100) {
            zoomOut.focus();
        }
    }
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
