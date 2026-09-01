# Explicit export application contract

## Status

Frozen for first-release persistent output boundaries.

## Purpose

This contract defines how an accepted Atrament revision becomes an explicit
persistent output through browser, CLI, or MCP application capabilities. It
keeps Export separate from semantic Apply, prevents accidental autosave, and
makes file side effects recoverable when an automation transport loses a
receipt.

## Scope

The contract covers export inputs, accepted-revision binding, target paths,
overwrite intent, diagnostics, temporary intermediates, file commit behavior,
retry recovery, receipts, concurrency, and adapter parity.

It does not freeze filesystem APIs, path string syntax, PDF internals, image
encoders, machine-plan formats, or final wire field names.

## Contract

### Explicit invocation

Export occurs only after an explicit application request. Semantic Apply,
Validate, Render, Plan, notebook editing, and ordinary session lifecycle do not
write a persistent output as a hidden side effect.

A successful export does not enable autosave. Later notebook edits remain
memory-only until another explicit export request occurs.

### Accepted revision binding

Every export request identifies the accepted revision whose output is being
requested. The backend compiles from authoritative semantic and derived state
for that revision plus admitted export inputs.

Export does not silently switch to a newer revision merely because editing
continues while an output is being prepared. A stale or unavailable requested
revision returns a typed application failure according to the owning output
contract.

The receipt identifies the revision actually consumed.

### Export inputs

The application owns typed export inputs appropriate to the selected output.
They include at least the admitted output format or profile, the explicit target
path, overwrite disposition, and any renderer or output choices that can change
the artifact.

Output choices that affect reproducibility participate in the output identity or
manifest according to their owning renderer and format contracts.

Browser UI labels, current working directory accidents, window state, and
clipboard content are not export inputs.

### Explicit path boundary

The caller supplies or deliberately selects the target path through the owning
file adapter. The backend validates the path and output kind before persistent
write effects begin.

MCP does not gain a second path authority. It uses the same admitted export file
boundary as direct or CLI application and cannot mutate repository internals,
cache state, or hidden session files merely by naming them as output targets.

The implementation does not silently substitute a different directory, append a
new filename suffix, or fall back to an internal path when the requested target
is invalid.

### Overwrite intent

Existing-target behavior is explicit. A request that does not admit replacement
fails rather than overwriting an existing file silently.

When replacement is explicitly admitted, the file adapter preserves the prior
target unless and until the new output reaches its final successful file-commit
boundary. A failed compilation or temporary write does not destroy the previous
valid target.

The application does not guess overwrite intent from a previous export to the
same directory.

### Pre-write validation

Export checks blocking semantic, capability, source, layout, asset, and output
format diagnostics through the frozen typed diagnostic envelope contract before
reporting success.

An unsupported or unresolved object is not silently dropped merely to create a
file. The output-specific contract determines whether an admitted conversion,
refusal, or user resolution is required.

Validation failures leave the accepted notebook and existing target unchanged.

### Temporary intermediates

An exporter may use bounded process-owned temporary storage when the format or
platform requires it. Temporary intermediates are not notebook authority and do
not become hidden recovery files.

Owned temporary files are removed after success or failure as soon as the
operation no longer requires them. Their paths are not exposed as normal output
paths or semantic document identity.

### File commit behavior

Export reports success only after the selected target represents the complete
admitted artifact for the request. A partially written target is never reported
as a successful export.

The concrete adapter may use memory, a temporary sibling, atomic replacement, or
another platform-appropriate strategy. The technical contract requires the
observable all-or-failed boundary without freezing one filesystem primitive.

On failure before that boundary, the adapter cleans owned intermediates and
preserves any pre-existing target when replacement was requested.

### Output identity

A successful export returns a backend-owned output identity and enough manifest
information to relate the artifact to its accepted revision and output-affecting
inputs.

The output identity is not the filesystem path itself. Moving or renaming an
already produced file does not change which accepted inputs produced its
contents.

Operational timestamps or adapter request IDs do not become semantic notebook
state merely because an export receipt records them.

### Export retry identity

Because Export has a persistent side effect, an automated request uses an export
retry identity or equivalent idempotence mechanism owned by the application
capability.

The retry identity binds the normalized export request: consumed revision,
output kind and output-affecting options, target path, and overwrite
disposition.
Reusing it with different normalized export content is a retry conflict.

If the file commit succeeds but the response is lost, repeating the same export
request and retry identity recovers the prior normalized result without creating
a second artifact or intentionally rewriting the target.

Retry state is in-memory session state. It is not a persistent export database
or
credential and disappears with the disposable session.

### External target drift during retry

Same-retry recovery does not blindly overwrite a target that changed outside the
completed export after the original file commit.

When the adapter can prove the current target still represents the previously
committed output, it may return the prior receipt. When target state conflicts
or
cannot satisfy the admitted recovery check, the application returns a typed
conflict or unknown-output condition rather than overwriting external work.

A caller can then choose an explicit new export request, path, or overwrite
operation.

### Concurrent exports

Concurrent requests targeting the same path cannot interleave bytes or both
claim incompatible successful replacement outcomes.

The file adapter serializes, rejects, or otherwise resolves the conflicting file
commit according to one explicit application rule. Requests to independent
paths may proceed independently when their output dependencies allow it.

Semantic notebook mutation remains separate from this file-side concurrency.

### Receipt

A successful export receipt identifies at least the consumed accepted revision,
normalized export intent, target path, output identity, overwrite disposition,
and diagnostics or manifest identity required by the output contract.

A failed export receipt or typed result distinguishes validation, path,
overwrite, compilation, file-commit, retry, and external-target conflicts well
enough for automation to choose its next explicit action without parsing prose.

The receipt does not convert its target directory into an autosave location.

### Adapter parity

Browser, CLI, and MCP Export capabilities dispatch through the same application
and file-adapter boundary. Equivalent normalized requests produce equivalent
output semantics and receipts independent of the inbound adapter.

The browser may provide a file chooser or path presentation. CLI and MCP may
supply paths through their admitted interfaces. None gains permission to bypass
output validation or write internal state directly.

## Failure Modes

The contract fails if semantic Apply writes a file implicitly, if a successful
export begins autosaving later revisions, or if an exporter silently changes the
requested target path or overwrite policy.

It fails if a reported successful target is partial, if a failed replacement
destroys the prior valid file, or if owned temporary artifacts survive without
an
admitted reason.

Automation safety fails if a lost receipt causes duplicate side effects, if the
same retry identity accepts a different export request, or if retry recovery
blindly overwrites externally changed target content.

The boundary also fails if MCP can write repository internals or hidden session
state outside the same explicit export path policy used by other adapters.

## Verification

The first user journey exports PDF from its accepted `layout-resolved` revision.
The receipt names that revision, the explicit path, and one output identity
while
accepted notebook and live-plan state remain unchanged.

A no-overwrite fixture pre-creates the target and requests export without
replacement permission. Export rejects and the original bytes remain unchanged.

A replacement-failure fixture starts with a valid existing target, admits
replacement, then fails compilation or the temporary write before final file
commit. The original target remains readable and unchanged.

A lost-receipt fixture commits one output and drops the transport response.
Repeating the same normalized export request and retry identity returns the
prior
result without a duplicate output or second intentional rewrite.

An external-drift fixture modifies the target after successful export and before
same-retry recovery. Recovery detects the conflict and does not overwrite the
external change.

A temporary-cleanup fixture forces success and failure paths and proves owned
intermediates disappear. No database, autosave journal, or hidden recovery file
is created.

Parity fixtures export equivalent accepted revisions through direct, browser,
CLI, and MCP entry paths and compare output identity inputs, diagnostics, and
persistent artifact semantics.
