# Application operation lifecycle contract

## Status

Frozen for first-release application operation lifecycle semantics.

## Purpose

This contract defines cancellation, progress, completion, and session shutdown
semantics shared by Atrament application capabilities. It lets direct, CLI, and
MCP adapters expose long-running work without inventing different meanings for
partial progress or cancellation.

## Scope

The contract covers Validate, Apply, history traversal, Render, Export, and
Plan operation lifecycle. It defines operation identity, progress authority,
cancellation boundaries, completion, transport loss, session shutdown, and
adapter parity.

It does not require an asynchronous API, polling protocol, background worker,
job database, final progress units, wire fields, thread model, or MCP task
mechanism. A capability may remain a blocking call when that is sufficient.

## Contract

### Capability-owned lifecycle

Every application operation keeps the semantic effect boundary defined by its
own capability contract. A common lifecycle layer does not turn read-only work
into mutation or make persistent side effects reversible.

Adapters may expose the operation as one blocking invocation or through an
admitted longer-running transport projection. Both forms preserve the same
application result and effect semantics.

### Operation correlation

An implementation may assign one ephemeral operation identity for correlating
progress, cancellation, and completion inside the active session.

That identity is operational metadata. It is not a notebook revision, command
context, retry identity, export identity, MCP admission credential, browser
secret, or physical-device authorization token.

Operation identities are not persisted merely to reconnect a later Atrament
process to work from a prior disposable session.

### Progress is observational

Progress reports may describe admitted phases, completed units, total units when
known, bounded estimates, or diagnostics useful to a caller.

Progress is observational. It cannot become notebook authority, a commit
receipt, proof that a file exists, proof that a plan completed, or proof that an
Apply crossed its commit boundary.

A progress percentage is not required to be a semantic identity. Implementations
must not make automation infer application success from `100%` without the
final typed result or receipt.

Progress wording and granularity may vary by adapter. Machine-readable phase or
state information, when exposed, retains the same lifecycle meaning across
adapters.

### Cancellation admission

A capability can expose cancellation only when the active implementation can
observe and honor it safely. Capability discovery indicates whether cancellation
is admitted for an operation class when callers need to rely on it.

A cancellation request is a request to stop admitted work. It is not proof that
the operation stopped before its effect boundary.

The final operation result remains authoritative for whether a semantic commit,
history traversal, or file commit occurred.

### Read-only operation cancellation

Validate, Render, and Plan have no accepted semantic or persistent file side
effect. If cancellation takes effect before a successful result is admitted,
partial candidate, render, or plan work is discarded as an incomplete operation.

An implementation may retain ordinary reusable caches whose full authoritative
input identity remains valid. It cannot expose an incomplete Render or Plan as a
successful complete result merely because some internal regions finished.

Cancellation does not mutate the accepted notebook to make read-only work stop.

### Apply cancellation

Semantic Apply follows its existing atomic commit boundary.

Cancellation that takes effect before commit produces the admitted
cancelled-before-commit semantics with no accepted revision change.

Once the atomic commit point is crossed, cancellation cannot relabel the
accepted transaction as cancelled or roll it back implicitly. The caller uses
the committed receipt, same-retry recovery, or normal history traversal if a
later semantic reversal is desired.

### History cancellation

Undo and Redo follow their own accepted-history commit boundary.

Cancellation before the traversal commit leaves accepted history unchanged.
Cancellation after traversal commit cannot restore the prior history position by
pretending the traversal never happened.

Lost completion follows the history same-retry recovery contract rather than
issuing another Undo or Redo with a new identity.

### Export cancellation

Export has a persistent file effect and therefore uses its file-commit boundary.

Cancellation before file commit cleans owned temporary intermediates and
preserves any pre-existing target according to the Export contract.

Once file commit succeeds, cancellation cannot delete, rewrite, or roll back the
committed output merely to report a cancelled state. A lost final response uses
the Export retry and target-drift recovery rules.

A caller that later wants the committed file removed or replaced performs a
separate explicit file operation admitted by the owning boundary; cancellation
is not implicit deletion authority.

### Completion and receipts

An operation is complete for application purposes only when its owning
capability returns a final typed result or receipt, or same-retry recovery
resolves a previously unknown mutating outcome.

Adapter disconnect, cancelled transport, process signals, progress termination,
or UI closure do not manufacture an application result class.

For read-only Render and Plan, repeating an equivalent request after transport
loss remains safe according to their retry contracts.

### Unknown outcomes

When transport disappears around a mutating effect boundary, the caller may not
know the final application outcome.

Apply, history traversal, and Export recover through their owning retry identity
or idempotence mechanism while the same active session and recovery state remain
available.

The lifecycle layer does not invent a global retry identity spanning different
capabilities.

### Session shutdown

Orderly shutdown stops admission of new application operations before destroying
the session.

In-flight work either reaches an owning final boundary or observes admitted
cancellation according to its capability. Shutdown does not turn an already
committed semantic revision or file into an uncommitted result.

Process termination can of course interrupt work without a delivered receipt.
The disposable-session contracts still prohibit creating a hidden persistent
job database merely to make interrupted work survive a fresh process.

Explicit files that completed their Export commit remain persistent because
Export deliberately crosses the session boundary. In-memory candidate, render,
plan, history-retry, and operation-correlation state do not.

### Adapter parity

Direct, browser, CLI, and MCP projections preserve the same effect boundary and
final result semantics for equivalent application operations.

One adapter may expose richer progress or cancellation ergonomics than another
only when it does not change what counts as committed, cancelled, failed, or
successful application work.

MCP tool framing cannot convert a transport cancellation into an application
rollback, and browser UI closure cannot make a committed Export disappear.

## Failure Modes

The contract fails if progress is treated as commit evidence, if cancellation
after a semantic or file commit is reported as though no effect happened, or if
a partial Render or Plan is exposed as a complete successful result.

It fails if one adapter defines a different commit boundary, if cancellation
silently deletes a committed file, or if a lost mutating response causes a new
retry identity and duplicate effect.

Lifecycle privacy fails if operation correlation becomes a durable credential,
hidden recovery database, or cross-session job authority.

## Verification

A read-only cancellation fixture interrupts Validate, Render, and Plan before
successful completion. Accepted revision and persistent files remain unchanged,
and no partial projection is reported as complete.

An Apply fixture cancels once before commit and once after the simulated commit
boundary. The first leaves the revision unchanged; the second resolves as the
committed Apply rather than a rollback.

An Export fixture begins replacement of an existing target and cancels before
file commit. The old target remains intact and owned intermediates disappear.
A second fixture loses cancellation or transport after file commit and recovers
the committed output without deleting or rewriting it implicitly.

A history fixture cancels before traversal commit, then tests a lost response
after traversal commit. Same-retry recovery proves exactly one history step was
accepted.

A progress fixture emits several progress observations and then a final result.
Automation branches on the final typed result, not on progress text or a numeric
percentage.

A shutdown fixture starts admitted in-flight read-only and mutating operations,
initiates orderly session shutdown, and verifies each operation obeys its owning
effect boundary while no hidden job state survives a fresh process.
