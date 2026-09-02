# Derived and output result taxonomy

## Status

Frozen for first-release Render, Plan, and Export application results.

## Purpose

This contract defines the minimum typed result classes needed for reliable
browser, CLI, and MCP automation of derived projections and explicit persistent
output. It prevents callers from branching on diagnostic prose or filesystem
error strings.

## Scope

The taxonomy covers Render, device-neutral Plan, and explicit Export results,
including stale input, capability rejection, cancellation, export path and
retry conflicts, internal failure, and transport-unknown recovery.

It does not freeze final enum names, numeric codes, HTTP statuses, MCP error
framing, output format errors, diagnostic codes, or filesystem-specific error
values.

## Contract

### Result and diagnostic separation

Every completed Render, Plan, or Export operation returns one typed
application-level result class plus the receipt and diagnostics admitted for
that class.

The frozen diagnostic envelope explains why and where a condition occurred. It
does not replace the result class that says whether a projection completed or a
persistent export committed.

Adapter-specific prose, terminal status, exception strings, and UI labels are
not automation branch conditions.

### Completed projection

Meaning: Render or Plan completed successfully for the explicitly requested
accepted revision and normalized projection inputs.

The result carries its backend-owned render or plan identity plus the owning
receipt. Accepted notebook source and application history remain unchanged.

A Completed projection does not mean a file was exported or physical hardware
was authorized.

### Exported

Meaning: one explicit Export request crossed its file-commit boundary and the
target represents the complete admitted artifact.

The receipt identifies the consumed revision, normalized export intent, output
identity, target, overwrite disposition, and admitted diagnostics or manifest
information.

Exported does not enable autosave and does not imply later revisions were
persisted.

### Idempotent export replay

Meaning: a completed Export is repeated with the same export retry identity and
same normalized export request, and recovery proves the prior committed output
still satisfies the owning retry contract.

The target is not intentionally rewritten and no duplicate artifact is created
merely to recover a lost receipt.

If external target drift prevents safe recovery, the result is not Idempotent
export replay.

### Stale or unavailable revision

Meaning: the requested accepted revision is no longer admitted for the requested
Render, Plan, or Export operation, or the operation cannot consume that exact
revision under its owning retention rules.

The backend does not silently substitute the current revision. No semantic
mutation occurs and Export does not write another revision's content instead.

The caller inspects current state and deliberately issues a new request when
appropriate.

### Capability or validation rejection

Meaning: the requested projection cannot be admitted because required semantic,
asset, output-profile, live-capability, geometry, provenance, or other owning
validation failed.

No successful Render or Plan result is manufactured by dropping unsupported
content. Export does not commit a file merely to produce some output.

Typed diagnostics identify the blocking evidence and affected semantic owners.

### Export path rejection

Meaning: the explicit Export target violates the owning path boundary or is not
an admitted output target for the requested operation.

No file commit occurs. The backend does not silently substitute another path,
directory, or hidden internal location.

The explicit rejected target can appear in its own Export diagnostic when
needed, without exposing unrelated internal paths.

### Export overwrite conflict

Meaning: the requested target already exists or otherwise conflicts with the
explicit overwrite disposition before a new file commit can proceed.

The prior target remains unchanged. The caller chooses an explicit different
path or overwrite intent rather than relying on remembered prior behavior.

### Export retry conflict

Meaning: one Export retry identity is reused with a normalized export request
different from the request bound to its first admitted attempt.

No new file effect is authorized by that retry. The caller corrects its retry
identity handling or intentionally issues a distinct export request.

An Export retry identity is not a credential or generic output identifier.

### External target drift conflict

Meaning: a prior Export committed successfully, but same-retry recovery observes
that the target changed externally or can no longer be proven to represent the
previously committed output.

Recovery does not overwrite the external change merely to reproduce the prior
receipt. The caller chooses a new explicit export action.

This class is distinct from Export overwrite conflict on an original request.

### Cancelled before result or effect

Meaning: admitted cancellation took effect before a read-only Render or Plan
produced a complete successful result, or before Export crossed file commit.

Render and Plan expose no incomplete projection as Completed. Export cleans its
owned temporary work and preserves any pre-existing target according to the
operation lifecycle and Export contracts.

Cancellation after file commit cannot produce this result for that Export.

### Internal failure with known no effect

Meaning: an internal application or adapter failure occurred and the core can
prove no complete Render/Plan result was admitted and no Export file commit took
place.

Accepted notebook state remains unchanged. For Export, an existing replacement
target remains preserved according to the file-commit contract.

The implementation does not claim known-no-effect when file commit status is
actually unknown.

### Unknown transport outcome

Unknown transport outcome is a caller state, not a result class fabricated by
the application.

For Render and Plan, the caller can safely repeat the same normalized read-only
request after transport loss.

For Export, the caller repeats the same normalized export request with the same
retry identity inside the active session. It does not infer Exported or failure
from the missing transport response alone.

### Operation lifecycle relationship

Progress, cancellation requests, connection state, and operation-correlation
metadata do not replace final result classes.

A progress report at `100%` is not Completed or Exported by itself. A
cancellation request is not Cancelled before result or effect until the owning
operation reports the corresponding final semantics.

### Adapter parity

Equivalent direct, browser, CLI, and MCP operations preserve result-class
meaning even when their wire framing or human-readable messages differ.

MCP projects these classes in machine-readable form when the corresponding
capability is implemented. It does not collapse path conflict, stale revision,
validation rejection, and committed Export into one undifferentiated tool error.

## Failure Modes

The taxonomy fails if automation must parse prose to distinguish a successful
projection, stale revision, validation rejection, path conflict, overwrite
conflict, committed Export, or safe replay.

It fails if Exported is reported before file commit, if cancellation after file
commit is mislabeled as no effect, or if an externally changed target is
overwritten during same-retry recovery.

It also fails if adapters assign different application meanings to equivalent
results or if progress is treated as a substitute for final completion.

## Verification

A Render fixture succeeds once and then requests an unavailable old revision.
The first result is Completed and the second is Stale or unavailable revision;
neither mutates notebook history or writes a file.

A Plan fixture includes unsupported live-only capability content. It returns
Capability or validation rejection with blocking typed diagnostics rather than
silently omitting the object.

Export fixtures cover a successful file commit, same-retry lost-receipt
recovery, invalid path, existing-target no-overwrite behavior, changed retry
content, and external target drift. Each maps to its distinct semantic result
class and preserves the file invariants of the Export contract.

A cancellation fixture interrupts Render, Plan, and Export before their owning
completion or effect boundary. No partial projection is reported as Completed
and no partial Export is reported as Exported.

A transport fixture loses a successful Export response after file commit. The
caller begins in Unknown transport outcome and same-retry recovery resolves to
Exported-equivalent or Idempotent export replay without rewriting the target.

Parity fixtures run equivalent Render, Plan, and Export outcomes through direct,
browser, CLI, and MCP projections and compare typed result classes separately
from diagnostic prose.
