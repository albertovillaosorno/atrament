# Semantic command review projection

## Status

Frozen for interactive and automated command review.

## Purpose

This contract defines the backend-owned review view for semantic command
validation and Apply receipts. It lets a person or agent distinguish requested
commands, accepted semantic changes, and dependency-expanded impact without
reconstructing domain meaning in the browser or MCP adapter.

## Scope

The contract covers validation review, semantic diff structure, impact
presentation, diagnostics correlation, selective review, parity, and sensitive
metadata boundaries.

It does not freeze final UI layout, wire field names, JSON encoding, diff text
formatting, or pagination mechanics for unusually large review payloads.

## Contract

### Three separate review layers

A command review keeps three concepts separate:

1. ordered commands requested by the batch;
2. semantic changes produced by candidate simulation;
3. derived identities or regions invalidated by dependency expansion.

Command count is not change count, and change count is not impact count. One
valid command may be a semantic no-op, while one global constraint change may
invalidate the complete derived notebook.

### Backend-owned semantic diff

The application core derives the semantic diff by comparing the immutable base
revision with the isolated validated candidate. Adapters do not compute a diff
from serialized storage or DOM state.

Each semantic change identifies enough typed information to review its meaning,
including:

- owning command identity or identities where attribution is meaningful;
- accepted semantic identity, batch-local insertion handle, or deleted identity;
- semantic authority and command family involved;
- typed change kind such as insert, delete, move, modify, or constraint change;
- normalized before and after semantic values or an equivalent inspectable
  backend-owned representation when the value is structured;
- provenance changes when provenance authority itself is modified.

The diff does not expose storage paths, DOM selectors, memory addresses, browser
session secrets, MCP credentials, or renderer-internal object layouts as
semantic change meaning.

### No-op and rejected commands

A valid command that produces no semantic change remains visible as a
per-command
No-op outcome but does not manufacture a semantic change entry.

An atomically rejected batch may show simulated outcomes and diagnostics, but
its review projection clearly marks that no semantic change is committed. A
successful earlier simulation cannot be presented as an accepted edit when a
later required command rejects the batch.

### Insertions and deletions

Validation can review an insertion through its batch-local handle and proposed
semantic owner before an accepted identity exists. Apply receipts replace or
supplement that candidate handle with the backend-allocated accepted identity.

Deletion review identifies the accepted semantic owner removed by the candidate.
The browser does not infer deletion from an object disappearing from preview.

### Moves and spatial changes

Move, crop, resize, alignment, grouping, and layering review describe the
revision-owned semantic constraint change. Pixel deltas in one preview may be
shown as presentation evidence but are not the authoritative diff.

Equivalent direct manipulation and command-mode intent therefore converge on the
same semantic review representation after backend normalization.

### Impact view

The impact view is derived separately from the semantic diff. It identifies
layout, handwriting, diagnostic, preview, render, export, or motion authorities
that became stale because they depend on changed semantic state.

An impacted derived region is not claimed to have changed visibly. It means its
prior result could no longer be trusted without recomputation.

The review explains dependency expansion sufficiently to distinguish a bounded
edit from an intentionally global invalidation. It never trusts a model-provided
impact list as authority.

### Diagnostic correlation

Diagnostics correlate to command identities, semantic identities, constraints,
or derived impact regions through backend-owned typed references. Human-readable
prose may explain a problem but is not the correlation key.

The same normalized diagnostic relationships are available to interactive, CLI,
and MCP review paths.

### Complete versus summarized values

A review surface may present a bounded summary for large structured values, but
it must indicate when the visible representation is incomplete. A summary cannot
silently hide a semantic field that affects acceptance.

The backend remains able to provide the complete admitted semantic value or a
structured inspection of that change before interactive acceptance or automated
decision when the workflow requires it.

Presentation truncation never changes normalized batch equality, semantic diff,
or Apply behavior.

### Selective review

The review projection is read-only application evidence. A browser or agent does
not edit diff entries and submit the modified projection as document authority.

When an interactive user chooses only some command outcomes, the selection is
expressed through command identities and enters the selective rebatching
contract.
The backend checks dependency closure, constructs or admits a new normalized
batch, and validates it again.

Selecting a semantic change entry does not imply every command that contributed
to it can be removed independently. Command dependencies remain authoritative.

### Result and receipt relationship

Successful Validate review describes a predicted semantic result. Apply review
uses the same semantic projection when the authoritative inputs remain
unchanged,
plus commit-owned metadata such as result revision, transaction provenance, and
final insertion identity mappings.

No-op, stale, scope, dependency, validation, and other typed result classes
control how review evidence is interpreted. Diagnostic severity does not replace
the result class.

### Adapter parity

The browser renders backend-owned review data and may choose compact or expanded
presentation. It does not parse command envelopes to reconstruct the semantic
diff.

CLI may format the same projection for terminals, and MCP may expose it as
structured result data. Equivalent application results preserve the same
semantic changes, impact meaning, diagnostic correlations, and completeness
markers across adapters.

## Failure Modes

The contract fails if the browser, CLI, or MCP adapter computes semantic changes
from storage paths or presentation state instead of consuming core-owned review
data.

It fails if semantic changes and derived impact are conflated, if an invalidated
region is falsely reported as a semantic edit, or if a no-op command creates a
fake changed identity.

Review safety fails if summarized values silently omit acceptance-relevant
content, if a rejected batch presents simulated changes as committed, or if a
user can splice an already validated diff into a new mutation without backend
rebatching and validation.

It also fails if internal paths, session credentials, adapter secrets, or memory
identities become part of normal semantic review output.

## Verification

The `idea-text-correction` fixture shows exactly one semantic paragraph change
and any dependency-expanded downstream layout impact as separate lists.

The table-cell fixture shows one structured cell change while preserving the
table and unrelated cell identities. The global-constraint fixture shows one
semantic constraint change with notebook-wide derived impact.

The insertion-handle fixture validates using a candidate handle, then Apply
returns the same semantic insertion with the newly allocated accepted identity.
Same-retry receipt recovery preserves that mapping.

The spatial-parity fixture compares a direct drag and equivalent semantic
command.
Their normalized semantic review matches even if adapter-local pointer or UI
metadata differs.

The interactive-subset fixture proves selecting commands produces a new
backend-validated batch rather than an adapter-edited diff. The original review
projection remains immutable evidence of the original validation.

A large structured-change fixture forces summarized presentation. The review
marks incompleteness explicitly and allows complete backend inspection before
acceptance without changing semantic equality or the candidate result.
