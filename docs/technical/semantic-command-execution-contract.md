# Semantic command execution contract

## Status

Frozen for the semantic command-mode design.

## Purpose

This contract defines how an accepted Atrament notebook can be refined through
small typed application commands without replacing unrelated notebook content.
It connects clipboard-assisted model use, CLI automation, and MCP to one atomic
backend command boundary.

## Scope

The contract covers command-batch meaning, validation, application, retry
behavior, impact calculation, receipts, interactive review, and MCP parity. It
does not freeze HTTP routes, final wire field names, MCP tool names, or a JSON
Schema.

The Rust application core remains authoritative for command semantics,
validation, notebook revisions, dependency expansion, diagnostics, and derived
recomputation. The TypeScript browser may present and transport command text but
never parses or applies semantic commands.

## Contract

### Two model response modes

Initial structuring may return a complete candidate notebook. That mode remains
appropriate when no accepted notebook exists or when the requested
reorganization
is intentionally broad.

Command mode refines an accepted notebook. It returns an ordered semantic
command
batch against one explicit accepted base revision instead of returning a full
replacement notebook.

The backend chooses and describes the admitted response mode in the generated
prompt. An external model does not switch modes by emitting an unrecognized
envelope or by mixing replacement fields with commands.

### Command context export

A command-mode prompt is self-contained and names the accepted base revision. It
includes the admitted command vocabulary plus enough backend-selected semantic
context to perform the requested change without hidden prior chat context.

The context may be smaller than the complete notebook when stable identities and
dependencies make a bounded edit sufficient. The backend decides that scope and
must include any surrounding content or constraints required for a correct edit.

The request distinguishes readable context from writable command scope. An LLM
may use surrounding identities for reasoning without gaining permission to edit
them, and insertion commands are limited to explicitly admitted anchors.

A command-mode request has a reproducible prompt identity. That identity changes
when the prompt protocol version, accepted base revision, requested intent,
backend-selected readable context, writable scope, admitted command families, or
relevant constraints change.

Repeating Copy with all of those inputs unchanged produces the same prompt
identity. The returned batch is bound to that command context so a response from
an older revision or different scope cannot be accepted as the current request.

A pasted batch that targets outside the writable scope is rejected. A genuinely
global request may intentionally expose notebook-wide scope, but the model
cannot widen a bounded scope from its response.

Copying that request is a presentation action only. It does not create an
accepted command, mutate the notebook, or authorize a pasted response.

### Batch identity and base revision

Every command batch is associated with a protocol version, one accepted notebook
identity, and one explicit base revision. The base revision is a precondition,
not a hint for best-effort rebasing.

A browser-assisted batch is also bound to the backend-generated command context
that declared its readable identities, writable scope, and admitted command
families. That binding prevents a returned batch from silently expanding the
request it was given.

Each command has its own stable command identity within the batch. Commands
address stable semantic targets, insertion anchors, or accepted constraints
owned by the base revision rather than storage paths, DOM nodes, page pixels, or
serialized object offsets.

A batch also carries an application retry identity. Reusing that identity with a
different normalized batch is a conflict rather than permission to replace the
original request.

### Validation and proposal

Validation parses the complete envelope and resolves every required target
against one base snapshot. Version, target, provenance, capability, and command
preconditions are checked before accepted state can change.

Validation may simulate the ordered commands in an isolated candidate state. A
later command may observe valid effects produced earlier in the same batch when
the command contract explicitly admits that dependency.

A validation result contains a semantic diff, diagnostics, predicted changed
identities, and predicted derived invalidation. It never changes the accepted
notebook revision.

### Atomic apply

Apply performs the same validation and simulation before commit. The application
rechecks the accepted base revision at commit time so a concurrent accepted edit
cannot race between validation and mutation.

If one required command fails, the batch does not partially apply. The accepted
notebook, undo state, and authoritative derived inputs remain at the prior
revision.

A successful non-empty semantic change commits exactly one new accepted notebook
revision. The accepted batch is one undoable application transaction even when
it contains several commands.

A semantically empty batch does not create revision churn. Its receipt reports
an
unchanged accepted revision and an empty semantic change set.

### Retry and concurrency behavior

Repeating a completed apply with the same retry identity and the same normalized
batch does not apply the commands twice. The application returns the prior
normalized result or an equivalent typed idempotent receipt.

Repeating the retry identity with different normalized content is rejected. A
new batch against an obsolete base revision is also rejected rather than
silently
rebased onto newer accepted work.

When two valid batches race from one base revision, at most one can commit
first.
The other observes the changed current revision and returns a stale-base result
without partial mutation.

### Semantic change set

The core derives the semantic change set from accepted command effects. Targets
that receive no semantic change keep their stable identities and accepted
content.

Insert, delete, move, text, structure, provenance, style, and constraint
commands may affect different revision-owned semantic authorities. The command
type determines which authoritative data can change.

A command cannot claim that an object is unaffected merely to suppress work. The
backend computes the accepted change set from domain behavior rather than
trusting
an agent-provided list.

Viewport position, clipboard state, transient UI selection, device connection,
physical arming, and physical start are not part of the notebook revision. They
cannot be smuggled into a semantic command batch merely to gain batch atomicity
or MCP automation.

### Impact-scoped recomputation

The core expands the semantic change set through its dependency relationships to
produce an impact set. That set identifies derived regions whose results may no
longer be valid.

Derived layout, handwriting, diagnostics, preview, export, and motion
projections
are recomputed only where their dependencies are invalidated. A local edit may
therefore preserve unrelated pages or blocks when no dependency connects them.

Correctness is stronger than minimal invalidation. A reflowing paragraph may
invalidate following geometry, and a global paper or style constraint may expand
the impact set to the complete notebook even when only one command named it.

Export files and motion plans are projections, not patchable document authority.
When an affected output is requested, it is compiled from the current accepted
semantic and derived authorities.

### Application receipt

Every validation or apply operation returns a normalized receipt suitable for
interactive, CLI, and MCP comparison. The receipt identifies the base revision,
result revision or unchanged state, batch identity, and retry identity.

It also reports per-command outcomes, semantic identities changed by the batch,
derived identities or regions invalidated, and diagnostics. A scoped interactive
receipt can distinguish admitted writable targets from the identities that
actually changed.

Requested exports or plans report their own output identities without becoming
notebook state.

The receipt is inspectable enough to explain why a command did not apply and
which semantic or derived regions changed. It does not expose private internal
storage paths as application semantics.

### Clipboard-assisted command mode

The backend may present a self-contained command-mode request through the same
browser prompt surface used for one-shot model exchange. The user copies it to
an external chat and pastes the complete returned command envelope into the raw
untrusted response surface.

The observable interactive sequence is:

```text
accepted-revision
→ command-context-copied
→ command-batch-pasted
→ command-batch-validated
→ command-batch-accepted
→ accepted-revision
```

The browser transports that text unchanged. Backend validation returns the
command diff, impact preview, and diagnostics before an interactive acceptance
can commit the batch.

A pasted command batch is never accepted merely because it parses. Interactive
application remains an explicit accepted transaction after backend validation.

### CLI and MCP automation

CLI and MCP project the same inspect, validate, apply, render, export, and plan
application capabilities. They do not gain a separate command language or a
privileged document mutation path.

An explicitly invoked CLI or MCP apply capability may validate and commit a
batch without a browser acceptance click. That is full automation of the same
atomic application command, not a bypass around validation or revision
preconditions.

An MCP agent may iterate by inspecting the current revision, proposing or
validating commands, applying them, inspecting diagnostics, and requesting
outputs. A stale receipt requires another inspection rather than an implicit
agent-side rebase.

The equivalent automated sequence omits clipboard transport:

```text
inspect
→ validate or apply
→ inspect receipt and diagnostics
→ render, export, or plan
```

Generic semantic editing does not implicitly arm physical hardware. MCP may
automate notebook editing and device-neutral plan compilation, but `arm` and
`start` remain explicit device-boundary operations with their own safety and
operator requirements.

Other non-revision application commands, such as inspecting runtime status or
managing an admitted adapter lifecycle, use their own application contracts.
They are not embedded inside a semantic notebook batch.

### Browser boundary

No command vocabulary, command parser, revision validator, impact graph, or MCP
schema belongs in the TypeScript frontend. The browser may expose backend-owned
mode labels, prompt text, raw response text, diffs, receipts, and diagnostics.

Clipboard support remains a human transport convenience. Removing the browser or
clipboard from an automated workflow does not change command semantics because
CLI and MCP enter through the same application services.

## Failure Modes

The command-mode contract fails if any required command failure changes the
accepted revision, or if a stale base revision is silently rebased. It also
fails when a retry can apply the same normalized batch twice or when a no-op
batch creates revision churn.

It fails if an unrelated semantic identity is regenerated merely because an LLM
edited another identity. Omitting a dependency that can change from derived
invalidation is also a correctness failure.

Clipboard paste must not become accepted authority before backend validation and
acceptance. A returned batch must not expand its backend-declared writable
scope.

MCP must not use a privileged mutation model, and TypeScript must never become
command or impact authority.

## Verification

The `sober-single-pen` fixture provides the first one-paragraph command case.
Starting from its accepted notebook, command mode changes the `Idea` paragraph
from the authored sentence containing `entonces` to the shorter correction
already frozen in the first complete user journey.

The semantic change set for that batch contains only the `Idea` paragraph.
Every other authored block keeps its stable identity and content, while derived
layout impact may expand to later geometry when the shorter paragraph changes
flow measurements.

A global-style fixture must preserve authored content identities while
invalidating every derived result that actually depends on the style. This
proves that impact scope follows dependencies rather than command count.

A batch containing valid, invalid, then valid commands must leave the accepted
revision unchanged. A concurrent pair from the same base must allow one commit
and reject the other as stale.

Retry tests must send the same normalized batch and retry identity more than
once, then send different content with that identity. The first case is
idempotent and the second is a conflict.

Parity tests must normalize receipts from the direct application boundary, CLI,
clipboard-assisted validation, and MCP. Equivalent operations must agree on the
accepted revision, changed identities, impact set, diagnostics, and outputs.

An end-to-end MCP fixture must inspect the accepted `sober-single-pen`
revision, apply the same `Idea` edit, inspect any remaining diagnostics, and
request the next admitted output operation without opening a browser. The
resulting accepted notebook must match the equivalent interactive command
transaction.
