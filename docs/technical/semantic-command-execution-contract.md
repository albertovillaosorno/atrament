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

### Capability snapshot

Before generating or accepting command-mode work, the backend owns one
versioned capability snapshot for the active application context. It describes
the command behavior a caller may rely on without exposing implementation
internals.

The snapshot communicates at least:

- admitted command protocol versions;
- admitted semantic command families and family behavior versions;
- backend-owned command/context resource limits relevant to callers;
- normalization and typed-result behavior versions that affect compatibility;
- whether command-context, Validate, Apply, selective rebatching, and related
  application capabilities are admitted for the active adapter.

A capability snapshot is read-only application metadata. It is not a notebook
revision, retry identity, command-context identity, browser secret, or MCP
credential.

Command contexts bind the capability snapshot or equivalent behavior identity
used to construct them. If command meaning changes while the accepted notebook
revision remains otherwise unchanged, an older context is not silently upgraded
or reinterpreted.

A caller refreshes capability discovery and command context after a typed
Unsupported protocol or capability or Command-context mismatch result requires
it.

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

### Model instruction and data separation

Backend-generated command prompts distinguish application instructions from
notebook, task, source, citation, and asset-derived content supplied as data.
Content inside readable context never becomes an application instruction merely
because its prose addresses an agent or resembles a prompt.

A self-contained copied prompt labels or structurally delimits untrusted source
data sufficiently for the admitted model workflow. It does not interpolate the
browser session secret, MCP admission material, internal file paths, hidden
application credentials, or unrelated private runtime state into model context.

Source text may contain adversarial phrases such as requests to ignore writable
scope, invent commands, export files, reveal credentials, or control hardware.
Those phrases do not widen admitted command families or application authority.

The backend remains secure even when an external model follows hostile source
instructions. Returned content is still untrusted and must pass protocol,
command-context, writable-scope, precondition, capability, provenance, and
atomic validation before any accepted mutation.

MCP structured context preserves the same distinction. Agent-readable notebook
content is domain data; tool descriptions and application capability contracts
remain the authority for what operations are available.

### Agent response requirements

A command-mode request instructs the model to return only the admitted response
mode for that command context. It must not mix a complete replacement notebook
with semantic commands unless a future version explicitly defines such a mode.

The model may reference only writable semantic identities or insertion anchors
admitted by the command context. Readable neighboring context is evidence for
reasoning, not permission to mutate those identities.

The model does not allocate accepted identities, choose its own base revision,
or claim an impact set as authoritative. New identity allocation and dependency
expansion remain backend responsibilities.

Semantic batches do not contain clipboard actions, file export requests, browser
state, device lifecycle commands, or raw motion. Those operations remain
separate application capabilities.

When the requested edit cannot be expressed within the admitted families,
writable scope, or supplied evidence, the model uses the backend-admitted
unresolved or refusal form rather than broadening scope or fabricating data.

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

### Command preconditions

The explicit base revision is the global stale-state precondition for the whole
batch. Per-command preconditions add local intent checks; they do not weaken or
replace the base-revision requirement.

An admitted command family may require preconditions such as:

- the target identity exists in the named base revision and has the expected
  semantic kind or owner;
- an insertion or move anchor is still the admitted anchor or relationship;
- a normalized semantic value, relationship, or revision-owned constraint has
  the expected base value when the family needs compare-and-set intent;
- an admitted asset or semantic dependency has the expected identity and kind.

Preconditions are expressed in semantic authority terms. Storage paths, DOM
selectors, serialized array indexes, page pixels, browser state, and mutable
adapter internals are not command preconditions.

A failed local precondition rejects the atomic batch through semantic
validation.
The backend does not search for a similar object, choose a nearby anchor, or
reinterpret the command merely because the requested intent appears obvious.

Per-command preconditions also do not provide an automatic rebase mechanism. A
batch whose base revision is stale returns Stale base before a caller can use
local preconditions as permission to apply against the newer revision.

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

Undo and redo operate on accepted application history after a batch commits.
They are not semantic command families that can be embedded inside another batch
alongside new edits.

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
first. The other observes the changed current revision and returns a stale-base
result without partial mutation.

### Cancellation and unknown outcomes

Validation and pre-commit simulation may admit cancellation without changing the
accepted revision. Cancellation is not implemented by committing part of a batch
or by truncating its remaining commands.

Once Apply crosses its atomic commit point, a caller disconnect, timeout, or
cancel request does not pretend the accepted revision rolled back. The commit
result remains authoritative even when the transport failed before delivering
the receipt.

A caller with an unknown Apply outcome retries with the same retry identity and
the same normalized batch. Within the active session, the application returns
the prior normalized result or completes the one admitted attempt according to
its retry contract.

The caller must not manufacture a new retry identity merely because it lost the
first response. A new identity could represent a genuinely new application
attempt and therefore cannot be used as outcome recovery.

Retry-result state is ephemeral session state. It is not an autosave or durable
cross-session transaction log, and closing the session removes it with the rest
of the notebook session.

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

### Transaction provenance

Semantic source provenance and application transaction provenance are distinct.
A provenance command may edit admitted revision-owned source, claim, citation,
or unresolved-state metadata, subject to its writable scope and validation.

The accepted application history separately records how a mutation entered the
core. At minimum, command-mode history can distinguish direct human application,
clipboard-assisted model response, CLI, and MCP entry paths together with the
base/result revision, batch identity, and command-context identity when one
exists.

Transaction provenance is assigned by the application boundary, not supplied as
self-attested truth by the returned model batch. A batch cannot label an MCP or
clipboard-assisted mutation as an unaided human edit.

The provenance record does not require storing the complete external chat,
clipboard contents, model prompt text, or browser session secret. Stable
identities and admitted metadata provide inspectable local history without
turning sensitive transport payloads into hidden persistence.

Within the disposable session, accepted history and its transaction provenance
remain ephemeral. An explicit notebook export preserves whatever provenance the
owning file contract requires; closing the unexported session still destroys the
in-memory history.

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

Semantic apply does not implicitly render, export, compile a motion plan, or
perform another adapter effect. Those are separate application capabilities
that consume the resulting accepted revision when explicitly requested.

Their own receipts report output identities without turning those projections
into notebook state.

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

### Interactive command selection

A validated batch is immutable as an application request. Interactive review may
show commands individually, but the browser does not delete or rewrite commands
inside the validated envelope and then apply the remainder as if it were the
same
batch.

If the user wants only a subset of returned commands, that choice becomes a new
backend-owned batch proposal against the same still-current base revision. The
backend checks that the selected commands form a valid dependency-closed
request,
normalizes the new batch, assigns or admits its own retry identity, and
validates
it again before acceptance.

A selection that omits a required dependency is not silently repaired by
applying
extra commands the user did not choose. The backend reports the dependency and
can present a new explicit proposal if the user wants the required closure.

If accepted state changed while the user was reviewing, the selected batch is
stale and must be regenerated or revalidated against a new command context. The
review UI cannot use a prior validation receipt as a commit reservation.

This keeps selective human review compatible with atomic application: each
accepted transaction remains one complete validated batch even when it was
created from a subset of an earlier model response.

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

The equivalent automated sequence omits clipboard transport and keeps mutation
separate from output effects:

```text
inspect
→ validate or apply
→ inspect receipt and diagnostics
→ render, export, or plan the accepted revision
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

### Implementation evidence

The current semantic session foundation can check one accepted identity against
one exact revision for expected semantic kind and direct structural owner. A
separate compare-and-set check covers exact accepted inline text, formula mode
and source, table-row role, and physical page-profile geometry without mutating
accepted state. Stale revision rejection precedes every local comparison.

The frozen application-level command-family taxonomy is now represented as a
typed value without choosing serialized operation names. Current direct-edit
targets expose only executable family admission: Text content for inline text,
Structured content for formulas and table-row roles, and Document constraint for
page profiles. One aggregate read-only check validates requested family, kind,
owner, and optional exact base value against backend-derived target material.

A deterministic capability snapshot now reports those three discoverable family
behaviors and one top-level behavior version. Because no serialized command
protocol is implemented, the snapshot advertises no protocol or normalization
version, no command-context, Validate, Apply, or selective-rebatch capability,
and no guessed command/context numeric limits. A read-only compatibility check
rejects an older capability behavior version independently from notebook
revision changes.

A separate single-target direct-edit simulator classifies the four established
replacement value families as applicable, no-op, domain-invalid, unavailable,
or value-family mismatched without mutation. All four existing direct mutation
paths now consume that same simulator before cloning accepted state or
allocating
a revision, so target, domain, and no-op validation are shared with proposal
evidence.

A version-bound direct-edit proposal now composes capability compatibility,
complete local target preconditions, and replacement simulation in one read-only
operation. A direct change preview uses the same target-material snapshot to
report one exact before/after semantic change, or an empty change set for a
no-op, while preserving typed simulation rejection.

A transport-neutral ordered direct-edit batch now combines those primitives with
the generic dependency graph. It derives a private overlay for only targeted
editable values and their impact scopes. Dependency validation borrows command
payloads first; valid commands are then consumed in order so caller command IDs
and requested values can move into prediction/candidate evidence. Local
preconditions are compared by reference, and only additional typed failure
evidence is cloned.

Simulation stops after the first semantic rejection and reports later command
identities as not evaluated. The accepted revision and identity allocator remain
untouched.

When a later command edits a target changed earlier in the same candidate, it
must explicitly depend on the previous target writer before observing that
candidate value. A preceding semantic no-op does not manufacture such a
dependency. Per-command changes remain visible while aggregate coalescing stores
only first/last mutating prediction indexes per target. Final changes still
compare accepted base to final candidate, including an empty net set for a
change-then-revert sequence.

Successful direct-edit predictions now classify their net effect explicitly as
Mutation or NoOp. They also derive conservative impact seeds from backend-owned
semantic relationships rather than trusting caller impact claims. Text seeds
identify the owning flow and page plus shaping, wrapping, geometry, diagnostic,
rendering, handwriting, and motion authorities. Structured edits seed their
nearest block, flow, and page for structure, layout, and output dependencies.

Page-profile edits seed every accepted page that references the changed profile.
Those seeds are shared by single-target review and ordered batch simulation and
are omitted for a net semantic no-op. They are inputs to future dependency
expansion, not the final authoritative Validate impact set.

The ordered overlay copies accepted values only for identities named by the
batch and stops semantic traversal once every unique target is indexed. Indexed
material and impact scopes are consumed through simulation rather than cloned
again. Profile-only batches resolve page references without walking unrelated
blocks.

Block, list-item, table-row, and table-cell traversal uses borrowed slice
continuation frames in document order. Pending traversal state therefore follows
container depth rather than sibling count, while mixed batches still continue
until every requested target resolves. Non-editable targets retain ordinary
semantic material fallback.

In one pinned release probe, a first-block target with 100,000 unrelated
trailing
blocks measured about 20 microseconds after traversal hardening versus 1,099
microseconds before it. A first-child target with 100,000 callout siblings
measured about 19 microseconds with slice frames versus 186 microseconds before
them. These measurements are implementation evidence, not a capability, latency
promise, or numeric product-limit decision.

A separate transport-neutral command-graph domain validates duplicate command
identities, direct self-dependencies, missing dependencies, cycles, and acyclic
dependencies that point to later commands in the ordered sequence. It is generic
over command identity representation, preserves caller order, and uses iterative
graph traversal.

The graph layer also checks whether an interactive command-ID selection contains
all required dependencies; omissions are reported instead of silently adding
commands. A read-only requirement report derives the complete transitive
explicit
dependency closure omitted by a selection while preserving caller selection and
source/dependency order.

Session-level selection analysis binds that requirement report to the current
capability behavior and exact accepted base revision before graph inspection. It
also exposes selected-command, complete required-command, and omitted-edge
counts without materializing command-identity pairs. Session analysis borrows
proposal command identities; bounded detail clones only identities carried by a
successful report or typed failure. Stale and unknown inputs remain read-only.

Graph validation fast-paths the normal ordered case where every dependency
points to an earlier command and reuses borrowed source positions for subset
analysis. Its read-only node-view contract lets application preflight borrow
command IDs and dependency slices instead of allocating per-command edge views.
A complete selection skips closure bitmaps after complete-graph validation,
while
an empty selection still validates the source graph before reporting no required
commands. Partial requirement closure continues to use positional bitmaps.

A caller-bounded report counts omitted edges before materializing pairs and
rejects one over the supplied bound without truncation. Ordered batch graph
preflight uses the same node view, so invalid graphs clone only identities
carried by their typed rejection.

The same transport-neutral batch now has an atomic application foundation. Apply
re-runs the existing validation and simulation, rechecks the accepted base, then
replays only the coalesced final semantic changes into a cloned notebook before
one commit. Multi-command mutation enters semantic Undo history as one
transaction.

A net semantic no-op keeps the current revision and history position.
Middle-command failure and stale base produce no accepted mutation, and
successful semantic change/impact-seed evidence matches prior simulation. A
caller-bounded application path reuses the graph-limit preflight before any
candidate replay or history mutation.

No protocol normalizer, command-context/writable-scope admission, retry
identity, selective-rebatch capability, published product limit, full Validate
service, or discoverable Apply capability is admitted yet.

Exact authored text and formula source are compared as currently accepted bytes;
no Unicode normalization form is implied by this implementation evidence. The
complete command protocol may define a versioned normalization contract later.

Candidate acceptance currently admits at most 256 nested block-containment
levels as an implementation resource bound. Deeper candidates return a typed
nesting-limit failure before identity promotion or accepted mutation, and their
recursive structures are dismantled iteratively so rejection itself cannot
consume unbounded process stack depth.

These primitives do not implement command-context generation, protocol-owned
normalization, writable-scope admission, complete impact expansion,
deterministic command diagnostics, retry identity, Validate, Apply, undo/redo,
or adapter parity. The
ordered direct-edit simulator is an internal application foundation, not an
advertised command-mode capability. Those parts of this contract remain open.

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
