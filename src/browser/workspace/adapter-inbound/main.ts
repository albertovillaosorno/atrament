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
//   - Side effects: DOM updates, pointer capture, and clipboard writes only.
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

const graphemeSegmenter = typeof Intl.Segmenter === "function"
    ? new Intl.Segmenter(undefined, { granularity: "grapheme" })
    : null;

function countCharacters(value: string): number {
    if (graphemeSegmenter !== null) {
        return Array.from(graphemeSegmenter.segment(value)).length;
    }
    return Array.from(value).length;
}

function bindCharacterCount(
    input: HTMLTextAreaElement,
    output: HTMLElement,
): void {
    const update = (): void => {
        const count = countCharacters(input.value);
        const suffix = count === 1 ? "character" : "characters";
        output.textContent = `${count} ${suffix}`;
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

bindCharacterCount(taskInput, taskCount);
bindCharacterCount(sourceInput, sourceCount);
bindCharacterCount(candidateInput, candidateCount);

function syncPromptCopyState(): void {
    copyGeneration += 1;
    const available = promptOutput.value.length > 0;
    promptOutput.disabled = !available;
    copyPrompt.disabled = !available;
    if (!available) {
        copyStatus.textContent = "Waiting for a prompt from the backend.";
    } else {
        copyStatus.textContent = "";
    }
}

promptOutput.addEventListener("input", syncPromptCopyState);
syncPromptCopyState();

copyPrompt.addEventListener("click", (): void => {
    const prompt = promptOutput.value;
    const generation = ++copyGeneration;
    if (prompt.length === 0) {
        copyStatus.textContent = "Waiting for a prompt from the backend.";
        return;
    }

    if (navigator.clipboard === undefined) {
        copyStatus.textContent = "Clipboard access is unavailable.";
        return;
    }

    const write = clipboardWriteQueue.then(
        (): Promise<void> => navigator.clipboard.writeText(prompt),
    );
    clipboardWriteQueue = write.catch((): void => {});
    void write.then(
        (): void => {
            if (
                generation === copyGeneration
                && promptOutput.value === prompt
            ) {
                copyStatus.textContent = "Prompt copied.";
            }
        },
        (): void => {
            if (
                generation === copyGeneration
                && promptOutput.value === prompt
            ) {
                copyStatus.textContent = "Clipboard write failed.";
            }
        },
    );
});


const divider = requireElement<HTMLElement>("#workspace-divider");
const zoomOut = requireElement<HTMLButtonElement>("#zoom-out");
const zoomReset = requireElement<HTMLButtonElement>("#zoom-reset");
const zoomIn = requireElement<HTMLButtonElement>("#zoom-in");
const zoomStatus = requireElement<HTMLOutputElement>("#zoom-status");
const previewScale = requireElement<HTMLElement>("#preview-scale");
const workspace = requireElement<HTMLElement>(".workspace-grid");

divider.setAttribute("tabindex", "0");
divider.removeAttribute("aria-disabled");

function setEditorShare(percent: number): void {
    const share = Math.min(65, Math.max(35, Math.round(percent)));
    document.documentElement.style.setProperty("--editor-share", `${share}%`);
    divider.setAttribute("aria-valuenow", String(share));
    divider.setAttribute(
        "aria-valuetext",
        `${share}% source, ${100 - share}% preview`,
    );
}

function shareFromPointer(clientX: number): number {
    const bounds = workspace.getBoundingClientRect();
    return ((clientX - bounds.left) / bounds.width) * 100;
}

divider.addEventListener("pointerdown", (event): void => {
    if (!event.isPrimary || event.button !== 0) {
        return;
    }
    event.preventDefault();
    divider.focus({ preventScroll: true });
    divider.setPointerCapture(event.pointerId);
    setEditorShare(shareFromPointer(event.clientX));
});

divider.addEventListener("pointermove", (event): void => {
    if (divider.hasPointerCapture(event.pointerId)) {
        setEditorShare(shareFromPointer(event.clientX));
    }
});

divider.addEventListener("keydown", (event): void => {
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

let previewZoom = 100;

function setPreviewZoom(percent: number): void {
    previewZoom = Math.min(160, Math.max(60, percent));
    document.documentElement.style.setProperty(
        "--preview-zoom",
        String(previewZoom / 100),
    );
    zoomReset.textContent = `${previewZoom}%`;
    zoomStatus.textContent = `Preview zoom ${previewZoom}%`;
    previewScale.textContent = `Preview · ${previewZoom}%`;
    document.documentElement.toggleAttribute(
        "data-preview-expanded",
        previewZoom > 100,
    );
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
sessionStatus.textContent = "Frontend ready · waiting for backend session";
