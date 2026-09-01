# Semantic command acceptance fixtures

## Status

Frozen for semantic command-mode acceptance.

## Purpose

These fixtures define observable command-mode outcomes before the final wire
schema or backend routes exist. They exercise atomicity, scope, identity,
idempotence, impact expansion, adapter parity, and application boundaries.

## Scope

The fixtures reuse accepted notebook authorities and semantic identities defined
by the representative notebook contracts. They do not freeze serialized command
field names, protocol encoding, HTTP endpoints, or MCP tool names.

Each fixture assumes one version-compatible command vocabulary projected through
the same application core. A future implementation may add more fixtures without
weakening these acceptance conditions.

## Contract

### Fixture `idea-text-correction`

Base: accepted `sober-single-pen` before the human correction frozen in the
first
complete user journey.

Intent: remove `entonces` from the `Idea` paragraph while preserving the exact
remaining Spanish text and every formula.

Expected semantic result:

- one existing paragraph identity changes text;
- no other accepted semantic identity changes content or identity;
- no new block is inserted or deleted;
- provenance remains attached to the same semantic owner.

Expected derived behavior: text measurement, wrapping, downstream flow geometry,
diagnostics, preview, and requested outputs may recompute where dependencies
require it. Unrelated semantic content is not regenerated.

### Fixture `bounded-scope-escape`

Base: the same accepted `sober-single-pen` revision.

Command context: readable context includes the `Idea`, `Ejemplo 1`, and
`Ejemplo 2` sections. Writable scope contains only the `Idea` paragraph.

Returned batch attempts the requested `Idea` correction and also changes one
`Ejemplo 1` formula.

Expected result: validation rejects the complete batch for writable-scope
violation. Accepted revision, undo history, formulas, and paragraph text remain
unchanged.

### Fixture `unrepresentable-request`

Base: a bounded command context whose writable scope and admitted families
cannot
express the requested change.

One example asks the model to import a new remote image while command mode only
admits edits to an existing paragraph and no asset ingestion capability is part
of the context.

Expected model behavior: use the backend-admitted unresolved or refusal form
without inventing a URL-download command, widening writable scope, or replacing
the complete notebook.

Expected backend behavior: a malicious or mistaken returned batch that attempts
those effects is rejected atomically. Accepted revision and external state
remain
unchanged.

### Fixture `forged-accepted-identity`

Base: a valid insertion context with one admitted anchor.

Returned batch invents an accepted-looking semantic identity for the new block
instead of using the backend-owned allocation mechanism admitted by the final
wire contract.

Expected result: validation rejects the forged accepted identity. A valid
insertion request allows the backend to allocate the new accepted identity and
reports it in the apply receipt.

### Fixture `stale-base-revision`

Base command context names revision `R0`. Before its returned batch is applied,
a different valid edit commits revision `R1`.

Expected result: the `R0` batch is rejected as stale. The core does not infer an
agent-side rebase, search for a similar target, or apply the command against
`R1` merely because the stable identity still exists.

A new command context may be generated from `R1` if the caller still wants the
edit.

### Fixture `idempotent-retry`

A valid batch against revision `R0` applies successfully with retry identity
`K1`, producing revision `R1`.

Repeating the same normalized batch with `K1` returns the prior normalized apply
result or an equivalent idempotent receipt. It does not create revision `R2` and
does not duplicate inserted content.

Reusing `K1` with different normalized command content is a typed retry conflict
and leaves `R1` unchanged.

### Fixture `lost-apply-receipt`

A valid batch with retry identity `K2` reaches the atomic commit point, but the
client transport drops before the Apply receipt is delivered.

The caller cannot tell from the failed transport alone whether the batch
committed. It repeats the same normalized batch with `K2`.

Expected result: exactly one accepted revision exists for that semantic apply.
The retry returns the prior normalized result or equivalent receipt rather than
applying the batch a second time.

Repeating the edit with a new retry identity is not the recovery procedure and
is not used by this fixture.

### Fixture `equivalent-serialization-retry`

Two transport envelopes differ only in representation details that the admitted
protocol version declares semantically irrelevant, such as harmless whitespace
or object-member serialization order.

Both parse to the same normalized semantic batch and use the same retry
identity.

Expected result: retry comparison treats the second request as an idempotent
replay rather than a conflict. No second revision is created.

### Fixture `ordered-retry-conflict`

Two batches contain the same command identities and bodies but reverse two
commands whose order is behaviorally significant. Both use the same retry
identity.

Expected result: normalized semantic content differs because command order is
significant. The second request is a retry conflict rather than an idempotent
replay.

### Fixture `duplicate-command-identity`

One batch repeats the same command identity for two command entries, regardless
of whether their bodies match.

Expected result: validation rejects the complete batch before accepted mutation.
No command outcome is committed and no retry success is recorded for an invalid
transaction.

### Fixture `dependency-cycle`

A batch contains a direct self-dependency, a missing command dependency, or a
cycle across multiple command identities.

Expected result: validation rejects the complete dependency graph. The backend
does not guess an order, drop an edge, or apply the acyclic subset.

### Fixture `resource-limit-rejection`

For each backend-admitted command-mode bound, one request exercises the limit
and
one exceeds it. Bounds include envelope size, command count, dependency edges,
context, writable scope, and family-specific structured payload size where
applicable.

Expected result: requests at the admitted bound continue to normal semantic
validation. Over-limit input returns typed diagnostics before accepted mutation
and is never silently truncated into a smaller transaction.

### Fixture `atomic-middle-failure`

A batch contains three ordered commands. The first and third are individually
valid; the second targets a deleted identity or violates an admitted
precondition.

Expected result: the complete batch fails before accepted mutation. Effects from
the first command are not committed, the third command is not partially applied,
and the accepted revision remains the original base.

Validation may report useful per-command diagnostics from its isolated candidate
simulation without leaking candidate state into accepted authority.

### Fixture `concurrent-base-race`

Two valid non-no-op batches start from the same accepted base revision and race
to commit.

Expected result: one may commit first. The other rechecks the current accepted
revision at commit time, observes a stale base, and returns a typed conflict
without partial mutation.

The losing caller must inspect the new revision before deciding whether to issue
a new command.

### Fixture `table-cell-edit`

Base: an accepted representative notebook containing a typed table.

Intent: change exactly one admitted cell through the structured-content family.

Expected semantic result: table identity and all unrelated cell identities stay
stable. The edited cell changes through typed table semantics rather than a text
replacement of the whole table.

Expected derived impact is limited to the owning table, its measured flow, and
any downstream page geometry or outputs that depend on the new measurement.

### Fixture `spatial-parity`

Base: an accepted notebook with one movable callout or image and a stable object
identity.

Human path: a direct drag serializes one revision-owned placement constraint.
Command path: CLI or MCP requests the equivalent semantic spatial constraint.

Expected result: accepted constraint semantics and normalized derived impact
match after input normalization. Neither path may mutate preview pixels as the
source of truth.

### Fixture `admitted-asset-reference`

Base: an active session where image bytes have already passed the separate asset
ingestion boundary and received an admitted asset identity.

Valid path: a semantic command attaches that existing asset identity to an
admitted figure or block.

Invalid paths attempt to embed raw bytes, a base64 payload, a local file path,
or
a remote URL inside the semantic batch.

Expected result: only the admitted identity reference can enter the notebook
revision. Other forms are rejected without creating files, downloads, or partial
semantic state.

### Fixture `global-constraint-impact`

Base: a multi-page accepted notebook whose complete layout depends on one
revision-owned paper or global style constraint.

Intent: change that admitted global constraint with one semantic command.

Expected semantic result: authored block identities and content remain stable
unless the command family explicitly changes them.

Expected derived behavior: the impact set may legitimately include every page,
layout region, handwriting or render projection that depends on the changed
constraint. A one-command batch is not proof of a one-object impact set.

### Fixture `no-op-apply`

A valid command resolves to the same normalized semantic value already accepted
at its target.

Expected result: validation may report a no-op command outcome. Apply does not
create a new accepted revision, does not add undo history, and reports an empty
semantic change set.

Derived work is not invalidated merely to manufacture evidence of application.

### Fixture `clipboard-and-mcp-parity`

Base: the accepted `sober-single-pen` revision used by `idea-text-correction`.

Interactive path: backend command context is copied, the complete returned batch
is pasted into the raw response surface, backend validation produces the diff,
and the user accepts the batch.

Automated path: MCP inspects the same revision and applies the equivalent batch
through the same application command model without clipboard transport.

Expected result: normalized accepted content, result revision semantics, changed
identities, impact set, diagnostics, and provenance match. Adapter-specific
transport metadata may differ and is not document authority.

Physical `arm` and `start` are outside this fixture. Producing an admitted
machine-neutral plan does not authorize motion.

## Failure Modes

These fixtures fail if a rejected batch changes accepted state, if retries
create
revision churn, if stale work is silently rebased, or if writable scope can be
expanded by the returned model response.

They also fail when an unrelated semantic identity is regenerated without a
semantic command effect, when an indirect dependency is omitted from impact, or
when a global dependency is artificially treated as local.

Asset, clipboard, browser, file, and hardware boundaries fail if their state is
smuggled into semantic revision commands. Interactive and MCP parity fails if
the same normalized application intent produces different accepted notebook
semantics.

## Verification

Each fixture becomes an executable contract test when the Rust application core
and final command schemas are present. Tests should construct the accepted base
through domain fixtures rather than patching serialized storage internals.

Direct application tests run first. Equivalent CLI and MCP projections then
normalize and compare receipts against the direct result. Clipboard-assisted
fixtures additionally prove that the browser transports command text unchanged
and does not parse semantic commands.

Tests record the base and result revision identities, semantic change set,
derived impact set, per-command outcomes, diagnostics, and output identities
when
an output was explicitly requested.

Negative fixtures assert the complete accepted snapshot is unchanged, not merely
that an error code was returned. Concurrency and retry fixtures must inspect the
accepted state after all competing operations have completed.
