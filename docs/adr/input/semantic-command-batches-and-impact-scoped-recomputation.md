# Semantic command batches and impact-scoped recomputation

## Status

Accepted.

## Decision ID

`atrament.input.semantic-command-batches-and-impact-scoped-recomputation`

## Context

A complete notebook response is useful when an agent structures raw material for
the first time. It is wasteful when an accepted notebook already exists and the
requested change affects only one paragraph, table cell, style constraint, page
object, or export setting.

Clipboard chat, CLI automation, and MCP also need one inspectable mutation
model.
If each adapter invents its own patch format, Atrament would duplicate
validation,
produce inconsistent recomputation, and make automated edits hard to audit.

## Decision

Atrament exposes versioned semantic command batches as application input. A
batch
addresses one accepted notebook revision and contains ordered typed commands
against stable semantic identities, insertion anchors, or explicit session-level
settings.

Each command carries enough precondition information to reject stale or
inapplicable intent. The application core validates the complete batch against
one base snapshot before any accepted state changes.

Batch application is atomic. If one required command is malformed, stale,
unsupported, unauthorized, or invalid for the selected capabilities, no command
in that batch mutates the accepted notebook.

The core computes an impact set from accepted command effects and dependency
relationships. Semantic identities outside that set keep their accepted content,
while only invalidated derived layout, handwriting, diagnostics, preview,
export,
or motion projections are recomputed.

Impact-scoped recomputation is an optimization of derived work, not a weaker
correctness rule. Any dependency whose result can change must be invalidated
even
when it was not named directly by the command.

A command result exposes the accepted revision identity, per-command outcome,
changed semantic identities, invalidated derived regions, diagnostics, and any
requested output identities. Equivalent direct, CLI, browser-assisted, and MCP
requests return equivalent application results.

The browser does not parse or apply semantic commands. The backend may present a
self-contained command-mode prompt and a complete command envelope for Copy. The
prompt may expose broader read context than write scope, and the browser may
carry the returned text back through the raw-response surface without
interpreting
that scope.

Clipboard command batches remain untrusted input until backend validation. An
interactive workflow may preview their semantic diff and require acceptance;
an explicitly invoked CLI or MCP apply operation may commit the validated batch
without a browser click because the caller already requested that application
command.

MCP projects bounded tools from the same application command model rather than
creating agent-only mutation paths. Full automation therefore composes inspect,
validate, apply, render, export, and plan operations without bypassing notebook,
provenance, capability, or physical-safety rules.

## Consequences

- Agents can request small edits without regenerating unrelated notebook
  content.
- Clipboard, CLI, and MCP share one mutation and validation boundary.
- Stable identities and base revisions make stale agent edits rejectable.
- Accepted semantic changes remain atomic even when recomputation is
  incremental.
- Undo can record accepted command batches instead of DOM or canvas snapshots.
- Diagnostics can identify the exact command and semantic identity that failed.
- MCP automation can be end to end without making the agent a document
  authority.

## Rejected Alternatives

- Full-notebook replacement for every model edit was rejected because small
  changes would cause unnecessary churn and identity loss risk.
- JSON Patch or path-based object mutation as the application contract was
  rejected because storage paths are not semantic commands or stable identities.
- Browser-side command parsing was rejected because TypeScript presentation must
  not become a second domain-validation implementation.
- MCP-only mutation tools were rejected because they would create behavior that
  clipboard, CLI, and direct application tests could not reproduce.
- Best-effort partial batch application was rejected because it leaves accepted
  state ambiguous when a later command fails.

## Verification

Contract tests must apply equivalent command batches through the direct
application boundary, CLI, browser-assisted paste path, and MCP. Accepted
revision, changed identities, diagnostics, and authoritative derived results
must
match after normalization.

Fixtures must prove that one-block edits preserve unrelated semantic identities
and avoid unrelated derived recomputation. Dependency fixtures must also prove
that indirectly affected layout or output regions are invalidated when required.

Negative tests must cover stale base revisions, deleted targets, duplicate
command identities, incompatible versions, unsupported capabilities, malformed
batches, and failure after an earlier valid command. Every such required failure
must leave the accepted notebook at the original revision.

Automation tests must prove that MCP can inspect, validate, apply, render, and
export without browser interaction while exercising the same core commands and
safety checks as interactive workflows.
