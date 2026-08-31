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
//   - Outputs: Character counts, focus behavior, and clipboard writes.
//   - Side effects: DOM updates and explicit clipboard writes only.
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

function bindCharacterCount(
    input: HTMLTextAreaElement,
    output: HTMLElement,
): void {
    const update = (): void => {
        const count = input.value.length;
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

bindCharacterCount(taskInput, taskCount);
bindCharacterCount(sourceInput, sourceCount);
bindCharacterCount(candidateInput, candidateCount);

copyPrompt.addEventListener("click", (): void => {
    const prompt = promptOutput.value;
    if (prompt.length === 0) {
        copyStatus.textContent = "Waiting for a prompt from the backend.";
        return;
    }

    void navigator.clipboard.writeText(prompt).then(
        (): void => {
            copyStatus.textContent = "Prompt copied.";
        },
        (): void => {
            copyStatus.textContent = "Clipboard write failed.";
        },
    );
});
