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
