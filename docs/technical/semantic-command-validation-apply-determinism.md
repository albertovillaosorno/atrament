# Semantic command Validate and Apply determinism

## Status

Frozen for semantic command validation and application.

## Purpose

This contract defines when a successful command validation is a stable semantic
prediction of a later Apply. It prevents interactive review and autonomous MCP
workflows from approving one semantic change while the same unchanged request
commits a different one.

## Scope

The contract covers deterministic command simulation, validation snapshots,
Apply revalidation, semantic receipt comparison, revision allocation, and
nondeterministic inputs.

It does not reserve accepted state after Validate, freeze a byte-identical
receipt encoding, define revision-ID syntax, or require wall-clock and adapter
metadata to match.

## Contract

### Deterministic simulation inputs

For one admitted protocol and engine behavior version, command validation is a
pure semantic simulation over its declared authoritative inputs.

Those inputs include at least:

- accepted notebook identity and immutable base revision;
- command-context identity when required by the workflow;
- normalized semantic batch and command dependency graph;
- admitted revision-owned assets and constraints referenced by the batch;
- protocol, domain, normalization, and capability behavior versions that can
  change command meaning;
- explicit accepted seeds when a command family legitimately consumes seeded
  deterministic behavior.

Wall-clock time, adapter request IDs, terminal formatting, UI focus, clipboard
state, map iteration order, process addresses, and unseeded randomness do not
change semantic command simulation.

### Validate prediction

Successful validation reports the semantic candidate that Apply would attempt to
commit from the same authoritative inputs. Its semantic prediction includes:

- per-command simulation outcomes;
- semantic change set;
- dependency-expanded impact set;
- deterministic diagnostics relevant to acceptance;
- normalized no-op or mutation classification.

Validation remains read-only. A successful result is evidence about one snapshot
and request, not a reservation or lock on the accepted revision.

### Apply equivalence

When the accepted base revision, required command context, normalized batch, and
all command-meaning versions remain unchanged between Validate and Apply, Apply
repeats the same semantic validation and simulation before commit.

If the transaction commits, its semantic command outcomes, semantic change set,
impact set, and deterministic acceptance diagnostics match the successful
validation prediction.

Apply may additionally report commit-owned metadata unavailable to Validate,
such as the newly allocated result revision identity, accepted transaction
provenance, insertion-handle mappings finalized at commit, and operational
receipt metadata.

Those additions do not authorize a different semantic edit from the one that was
validated.

### Revision and context drift

Validate does not authorize Apply against a changed accepted revision. If the
current revision no longer satisfies the named base revision, Apply returns the
Stale base result rather than recomputing a different valid edit against the new
state.

If the command context or another versioned command-meaning input no longer
matches, Apply returns the corresponding typed compatibility or context result.
It does not reinterpret the old request under new semantics silently.

A caller obtains a fresh context and validation when it still wants the edit.

### Stable no-op behavior

A batch that validates as a semantic no-op against one immutable base snapshot
remains a no-op when applied against that same unchanged snapshot and semantic
behavior version.

Apply does not manufacture a revision merely because time passed after Validate,
an adapter changed, or receipt metadata differs.

### Inserted identity allocation

Validation may use batch-local insertion handles or candidate identities without
pretending they are accepted notebook IDs.

Apply allocates accepted identities at its commit boundary according to the
identity contract. The semantic owner, insertion position, dependencies, and
normalized requested values match validation even when the final accepted ID was
not available during simulation.

Same-retry recovery returns the committed mapping rather than rerunning identity
allocation as a second semantic insertion.

### Deterministic diagnostics

Diagnostics that are functions of the semantic candidate and declared command
behavior inputs are deterministic application evidence. Their normalized codes,
semantic targets, and ordered data participate in Validate and Apply parity when
the diagnostic contract says they are acceptance-relevant.

Adapter prose, timestamps, durations, logging context, and other operational
metadata may differ without creating a semantic mismatch.

### External and derived capability state

Semantic Apply does not become nondeterministic merely because unrelated runtime
or device state changes. Clipboard availability, viewport state, device
connection, and physical arming are outside the notebook revision transaction.

A command family that legitimately depends on an admitted versioned capability
or asset must bind that input through the command context or revision authority.
Unbound mutable external state cannot silently alter command semantics between
Validate and Apply.

### Seeded behavior only

Command validation and revision mutation do not use ambient randomness. When a
revision-owned command value includes a seed or another deterministic generation
input, that input participates in normalized semantics according to its command
family.

Intentional rerolling therefore creates a different admitted semantic request or
revision-owned value. It is not an invisible second interpretation of the same
validated batch.

### Parity across adapters

Direct, clipboard-assisted, CLI, and MCP paths use the same deterministic
simulation and Apply services. Equivalent validated inputs therefore predict the
same semantic Apply result independent of adapter transport.

An adapter may omit a separate Validate call and invoke Apply directly. Apply
still performs the same authoritative validation and simulation before commit.

### Implementation evidence

The active-process `SessionApplication` delegates exact and caller-bounded
transport-neutral batch simulation through the same owned semantic authority as
Apply. This is an internal Validate foundation only; it does not publish
protocol normalization, command context, diagnostics, or a discoverable
Validate capability.

The current application foundation can deterministically simulate an ordered
batch of the eleven established generic replacement value families against one
immutable accepted revision. It validates generic command dependencies first,
then consumes valid commands through a private value-and-impact overlay for only
the editable identities targeted by the batch. Command IDs and requested values
move into owned prediction/candidate evidence, local preconditions are borrowed,
and only extra typed failure evidence is cloned.

Successful commands update the overlay rather than a cloned notebook. A middle
failure leaves accepted state unchanged and marks later commands as not
evaluated.

Dependent same-target commands can observe earlier simulated candidate values
only through an explicit command dependency. Per-command changes retain their
local before/after values while aggregate coalescing stores first/last mutating
prediction indexes per target. Final aggregate changes still compare accepted
base with final candidate. An empty ordered batch returns NoOp before semantic
target indexing.

The same simulation derives conservative backend-owned impact seeds for text,
structured-content, figure asset-reference, block style-reference, list
ordering-significance, provenance-record, semantic constraint-kind,
page-profile geometry, and per-page profile-assignment changes. Asset,
block-style, and list-ordering changes use local block/flow scope with
`AllDerived` authority.

Provenance changes use notebook scope with only Diagnostics and Output
authorities, while constraint-kind changes use notebook scope with `AllDerived`.
Page-profile geometry changes seed every referencing page with `AllDerived`,
while a page-profile assignment change seeds only that exact page. These are
conservative seeds for later dependency expansion; this service does not execute
reflow.

Single-target review and ordered batches share those seeds, and net no-ops emit
none. Indexed target material and impact scopes move through final simulation
evidence instead of being cloned again.

The targeted semantic scan uses document-order borrowed slice frames for blocks,
list items, table rows, and table cells. It stops once every unique target is
resolved, bypasses block traversal for constraint-, page-, profile-, and
provenance-only batches,
and retains ordinary semantic material fallback for non-editable targets.
Pending traversal state
follows container depth rather than sibling count.

A pinned release probe measured a first target ahead of 100,000 top-level blocks
at about 20 microseconds after traversal hardening versus 1,099 microseconds
before it. The equivalent first child ahead of 100,000 callout siblings measured
about 19 microseconds with slice frames versus 186 microseconds before them.
These are implementation measurements, not product guarantees.

Graph validation now avoids cycle-state allocation for the ordinary ordered
case where every explicit dependency points backward, while preserving cycle
precedence when a forward dependency exists. Interactive selection analysis can
report complete transitive requirements or summarize closure size against an
exact base revision without creating a replacement batch. Generic report bounds
reject oversized identity-pair materialization before allocation, and session
analysis borrows proposal identities until concrete detail or failure is
returned. Complete selections skip transitive closure work after source-graph
validation; empty selections still preserve complete-graph failure precedence.

Session graph-resource preflight derives exact command/dependency counts
directly
from proposal commands and checks caller-supplied coarse bounds against
capability behavior and exact base revision before semantic simulation. Resource
inspection does not require command-ID ordering or graph-node allocation.
Passing those bounds does not imply that the dependency graph itself is valid.

A bounded ordered simulation applies those coarse limits before structural graph
validation and semantic candidate work. Exact-limit inputs retain the same
prediction as unbounded simulation, while over-limit input returns a typed
resource rejection without truncation.

A transport-neutral application foundation now consumes that same simulation.
It rechecks the current accepted base before candidate replay, applies only the
coalesced final semantic changes, and commits one revision for a net mutation.
Executable fixtures compare successful simulation and application command
outcomes, semantic changes, and impact seeds exactly; net no-op application does
not allocate a revision, and a changed base rejects as stale. The accepted batch
enters semantic history as one Undo transaction.

A repeated-history fixture commits 64 consecutive one-command Apply
transactions, traverses every snapshot back to the base, then redoes every
snapshot forward. Authored text and stable target identity survive every move,
and each history traversal receives a fresh revision identity.

Semantic history fixtures also traverse whole applied batches in both
directions.
Redo restores the complete multi-command transaction with a fresh revision; a
new batch applied after Undo clears the abandoned Redo branch.

Replay coverage exercises all eleven established generic editable values in
one transaction: text, formula, page-profile geometry, page-profile assignment,
table-row role, table-cell span, asset reference, provenance, constraint kind,
block style reference, and list ordering significance. One accepted revision
contains all eleven changes, and one Undo restores every prior value with stable
semantic identities.

Application receipts preserve the simulator's move-oriented command identity
behavior. Resource-limit rejection borrows caller command identities without
cloning them, while a successful dependent same-target chain moves all command
identities into the Apply receipt without additional clones.

Bounded Apply preserves the same global gate order as simulation: capability
compatibility, accepted-session presence, and exact base authority decide before
caller resource limits. Once admitted by those gates and limits, an invalid
dependency graph still rejects before semantic replay or history mutation.

A synchronized in-process concurrency fixture releases two Apply attempts bound
to the same accepted base together. Exactly one commits; after that commit the
other observes the winning revision as its typed stale-base result. One Undo
restores the complete pre-race semantic snapshot.

These foundations change no advertised capability or published resource limit.
Full Validate/Apply admission still requires protocol normalization, command
context and writable scope, retry identity, complete dependency impact
expansion, diagnostics, cancellation behavior, and adapter parity.

## Failure Modes

The contract fails if unchanged validated inputs can commit a different semantic
change because of clock time, unseeded randomness, storage iteration order,
adapter identity, or unrelated runtime state.

It fails if Apply treats successful Validate as permission to rebase onto a
newer revision, or if an engine/protocol/context change silently reinterprets an
old batch instead of returning a typed mismatch.

It also fails if a successful validation shows one semantic diff but Apply
commits another while claiming the same authoritative inputs, or if a validated
no-op creates revision churn without an admitted semantic input change.

## Verification

A validate-then-apply fixture runs one non-no-op batch repeatedly against the
same immutable base and command context. Every validation predicts the same
semantic command outcomes, change set, impact, and deterministic diagnostics.

Apply then commits once. Its semantic receipt matches the validation prediction,
while its new revision identity and transaction provenance are compared as
commit-owned metadata rather than pretending they existed during Validate.

A no-op fixture validates and applies after an artificial wall-clock delay. The
accepted revision remains unchanged and the normalized result remains No-op.

A drift fixture validates against `R0`, commits an unrelated `R1`, and then
attempts Apply of the `R0` batch. Apply returns Stale base rather than
producing a
new semantic simulation against `R1`.

A context-version fixture changes one command-meaning version or required
command-context identity while leaving notebook bytes otherwise unchanged. The
old batch returns the typed compatibility or context result and requires fresh
validation.

A deterministic-input fixture varies adapter request IDs, prose formatting,
clock time, and map insertion order without changing semantic inputs. Normalized
Validate predictions remain equal.
