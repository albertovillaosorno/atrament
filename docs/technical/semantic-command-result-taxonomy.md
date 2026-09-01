# Semantic command result taxonomy

## Status

Frozen for semantic command application results.

## Purpose

This contract defines the minimum semantic result classes required for reliable
interactive, CLI, and MCP command automation. Callers must be able to react to a
typed outcome instead of matching human-readable diagnostic prose.

## Scope

The taxonomy covers Validate and Apply at the application boundary, including
atomic rejection, retry behavior, cancellation, and stale context. It does not
freeze final enum names, numeric codes, HTTP status mappings, MCP error framing,
or the complete diagnostic envelope.

Transport failure and application result are deliberately separate. A missing
transport response does not invent an application rejection result.

## Contract

### Result envelope principles

Every completed Validate or Apply call returns one typed batch-level result
class plus the receipt and diagnostics admitted for that class. Human-readable
messages may explain the result but never replace the typed classification.

Per-command outcomes provide detail inside the batch result. They cannot make an
atomically rejected batch appear partially committed.

Result classes are normalized across direct, CLI, clipboard-assisted, and MCP
application paths. Adapter transports may map them differently at the wire level
while preserving the same application meaning.

### Successful validation

Meaning: the complete batch is valid against the inspected base snapshot and
command context at validation time.

Accepted state is unchanged. The result may include predicted semantic changes,
derived impact, per-command simulation outcomes, and diagnostics.

Successful validation is not a commit reservation. A later Apply can still
return stale-base or another commit-time failure when accepted state changes.

### Applied

Meaning: one valid, non-no-op batch committed atomically and produced one new
accepted notebook revision.

The receipt identifies the base and result revisions, normalized batch and retry
identity, committed command outcomes, semantic change set, impact, transaction
provenance, and admitted diagnostics.

An Applied result never means Export, Plan, Arm, or Start also happened.

### No-op

Meaning: the complete valid batch resolves to no semantic change against the
accepted base revision.

No new revision or undo-history entry is created merely to represent execution.
The receipt reports an unchanged revision and empty semantic change set.

### Idempotent replay

Meaning: the caller repeated a completed Apply with the same retry identity and
the same normalized semantic batch.

The application returns the prior normalized result or an equivalent receipt and
does not apply commands again. Automation treats this as recovered success when
the original Apply outcome was lost in transport.

### Unsupported protocol or capability

Meaning: the command envelope requests a protocol version, command family, or
application capability not admitted by the active backend context.

No accepted mutation occurs. The caller must negotiate or obtain a compatible
context rather than guessing a downgraded command representation.

### Command-context mismatch

Meaning: the batch is not bound to the active backend-generated command context
required for that workflow, or its prompt/context identity does not match.

No accepted mutation occurs. Clipboard-assisted automation obtains a fresh
command request; MCP obtains a fresh bounded command context before retrying the
semantic intent.

### Stale base

Meaning: the batch names an accepted base revision that is no longer current at
commit time or at another revision-sensitive validation boundary.

No partial mutation occurs. The caller inspects the current revision and decides
whether to generate a new batch. The application does not silently rebase the
old one.

### Writable-scope violation

Meaning: one or more commands attempt to mutate identities, anchors,
constraints,
or authority outside the admitted writable scope.

The complete batch is rejected. Readable context does not become writable merely
because the returned response references it.

The caller needs a newly admitted broader command context or a different edit,
not a retry of the same invalid batch.

### Retry conflict

Meaning: one retry identity is reused with normalized semantic content different
from the content associated with its prior admitted Apply attempt.

No new mutation occurs. This is a caller identity error, not permission to
replace the earlier request.

### Resource-limit rejection

Meaning: the command envelope, command graph, context, writable scope, or family
payload exceeds one backend-owned admitted limit.

The backend rejects the complete request without silent truncation. A caller may
request a smaller context, split the intended work into separately valid
transactions, or use another admitted workflow.

Splitting does not weaken atomicity within each resulting batch and cannot be
performed automatically when cross-batch semantics require one atomic change.

### Dependency-graph rejection

Meaning: command dependencies contain a duplicate identity, missing reference,
self-dependency, cycle, or another invalid graph condition defined by the active
protocol.

The complete batch is rejected before accepted mutation. The application does
not reorder or drop commands to guess a valid graph.

### Semantic validation rejection

Meaning: the envelope is structurally admitted but one or more required commands
fail target, precondition, provenance, domain, capability, or other semantic
validation.

The accepted revision remains unchanged. Per-command diagnostics may explain the
candidate simulation, but successful earlier simulations are marked as not
committed when the batch fails atomically.

### Unrepresentable or unresolved response

Meaning: the admitted command workflow cannot safely express the requested edit
with its current evidence, command families, or writable scope.

A model-facing unresolved or refusal response is not accepted as a mutation. It
returns actionable information for obtaining evidence, another context, or a
broader workflow without fabricating notebook content.

### Cancelled before commit

Meaning: an admitted cancellation took effect before Apply crossed its atomic
commit point.

No new accepted revision is created. A caller may choose whether to issue a new
Apply attempt according to the ordinary revision and retry rules.

Cancellation after the commit point cannot convert an Applied transaction into
this result.

### Internal failure with known no-commit outcome

Meaning: an application failure occurred and the core can prove the accepted
commit point was not crossed.

No accepted revision changes. The typed diagnostic distinguishes this from a
semantic validation rejection and from a transport-unknown outcome.

The implementation must not claim this result when commit status is genuinely
unknown.

### Unknown transport outcome

Unknown transport outcome is not an application result class returned by the
core. It is the caller's state when the connection fails before a valid Apply
receipt arrives.

The caller recovers by repeating the same normalized batch with the same retry
identity in the active session. It does not infer Applied or Rejected from the
transport failure alone.

### Per-command outcomes

A successful Applied receipt identifies which commands produced semantic change
and which valid commands were semantic no-ops.

A rejected atomic batch may report commands that validated in isolated
simulation, commands that failed, and commands not evaluated after a decisive
failure. None is represented as committed.

Per-command outcomes retain command identities so interactive and automated
clients can correlate diagnostics without depending on list position alone.

### Diagnostic relationship

Result class answers what happened to the application operation. Diagnostics
explain why and where, using the shared typed diagnostic contract when
implemented.

One result may carry several diagnostics. Diagnostic severity does not override
the batch result class, and prose wording is not a substitute for typed outcome
semantics.

### Automation guidance

An autonomous caller handles classes by semantics:

- Applied, No-op, and Idempotent replay continue from the reported revision;
- Stale base and Command-context mismatch require fresh inspection or context;
- Retry conflict requires correcting caller retry identity handling;
- Writable-scope, Dependency-graph, and Semantic validation rejections require a
  corrected request or intentionally different context;
- Resource-limit rejection requires an admitted smaller or alternate workflow;
- Unsupported protocol or capability requires compatibility negotiation;
- Cancelled before commit is known not to have committed;
- Unknown transport outcome uses same-retry recovery before any new edit.

These are application semantics, not a mandate for one UI or MCP retry policy.

## Failure Modes

The taxonomy fails if callers must parse English prose to distinguish stale
revision, retry conflict, scope violation, validation failure, and successful
idempotent replay.

It fails if an atomically rejected batch reports some commands as committed, if
No-op creates revision churn, or if transport timeout is misreported as a known
application rejection.

Automation is unsafe when Stale base silently rebases, Retry conflict is treated
as success, or a lost Apply receipt causes a new retry identity and duplicate
mutation.

It also fails if adapters invent result meanings unavailable from the shared
application core.

## Verification

Each semantic command acceptance fixture asserts one batch-level result class in
addition to accepted-state invariants and diagnostics.

The stale-base, scope-escape, retry-conflict, dependency-cycle, resource-limit,
no-op, atomic-failure, and lost-receipt fixtures exercise distinct classes.

A parity fixture runs equivalent outcomes through direct, CLI, and MCP adapters
and proves their normalized result classes agree despite transport-specific
presentation.

A transport-loss fixture intentionally drops the Apply response after commit.
The first caller state is Unknown transport outcome; same-retry recovery then
returns Applied-equivalent or Idempotent replay semantics with exactly one
accepted revision change.
