# Semantic application history contract

## Status

Frozen for first-release undo and redo semantics.

## Purpose

This contract defines accepted application history for direct human edits,
semantic command batches, and equivalent CLI or MCP mutations. It makes undo and
redo deterministic application commands rather than browser snapshot tricks or
semantic commands hidden inside another batch.

## Scope

The contract covers history entries, accepted revisions, undo, redo, branching,
stable semantic identities, recomputation, transaction provenance, concurrency,
and retry behavior.

It does not freeze final history wire fields, UI shortcuts, storage structures,
history depth limits, or a persistent history format. First-release history
remains in memory for the active disposable session.

## Contract

### Accepted transaction history

Every accepted non-no-op semantic mutation creates one application transaction
history entry and one new accepted revision. The entry records enough
backend-owned information to replay or reverse the accepted semantic transition
without depending on DOM, canvas, preview pixels, or adapter-local state.

A multi-command semantic batch contributes one history transaction because Apply
commits it atomically. A direct human semantic command enters the same history
model after backend acceptance.

Rejected validation, stale work, failed Apply, and semantic no-op operations do
not add history entries merely because they were attempted.

### History position

The active session has one current accepted revision and one history position.
Undo and redo operate relative to that current accepted history state rather
than
searching arbitrary past content for something that appears similar.

A history traversal names or otherwise preconditions the current accepted
revision. If another accepted mutation wins a race first, the traversal rejects
as stale instead of reversing a different current state.

### Undo semantics

Undo reverses exactly one accepted transaction admitted by the history position.
It restores the prior revision-owned semantic state represented by that
transaction boundary.

Undo itself is an accepted application-history mutation and therefore produces a
new accepted revision identity. The new revision's semantic content can equal an
earlier historical snapshot while its revision identity remains new and current.

This rule preserves stale-revision detection. A caller cannot mistake a restored
old semantic snapshot for the old revision instance that originally contained
it.

### Redo semantics

Redo reapplies exactly one transaction from the admitted redo branch. It also
produces a new accepted revision identity while restoring the transaction's
semantic result.

Redo is not implemented by submitting the original semantic batch as a new Apply
with a new retry identity. It traverses accepted history through the dedicated
history capability and retains the semantics of the original accepted
transaction.

### Stable semantic identities

Undo and redo preserve stable semantic identities from the historical semantic
states they restore. Unchanged surviving objects keep their IDs rather than
being regenerated simply because history moved.

When undo removes an object that an accepted insertion created, redo restores
that historical semantic object with the same accepted semantic identity used by
the original committed transaction.

Accepted semantic identities are not recycled for unrelated new objects during
the active session. If an edit after undo creates a new history branch, identity
allocation cannot collide with identities present in the abandoned redo branch.

### Branching after undo

If the user or an automated caller accepts a new semantic mutation while not at
the tip of the current redo branch, that mutation creates a new branch and the
prior redo path is no longer the active redo sequence.

The application does not silently apply an old redo transaction onto the new
branch merely because its original targets still exist.

First release need not expose arbitrary branch navigation. It must at least
avoid
ambiguous redo behavior after new accepted work diverges from the undone state.

### Derived recomputation

History traversal restores semantic authority first, then invalidates derived
work according to the dependencies of the reversed or replayed semantic change.
Undo and redo do not require whole-notebook regeneration when dependency
evidence
proves a bounded impact.

Correctness remains stronger than minimal invalidation. Reversing a global paper
or style constraint can legitimately invalidate all dependent pages and output
projections.

The resulting history receipt reports semantic identities restored, removed, or
changed plus dependency-expanded derived impact. Preview pixels are not the
source of truth for deciding what history changed.

### Transaction provenance

History records retain the transaction provenance of the original accepted
mutation. Undo and redo also record that a history traversal occurred and which
accepted transaction it reversed or replayed.

Undo does not rewrite an MCP-origin edit into a human-origin edit, and redo does
not erase the original clipboard-assisted or CLI provenance merely because the
semantic state was restored.

Complete external conversations, clipboard archives, browser credentials, or
adapter secrets are not required to retain this local provenance.

### History result semantics

History traversal returns a typed application outcome rather than requiring an
adapter to parse prose.

The minimum semantic cases are:

- Traversed: one admitted Undo or Redo committed and produced a new accepted
  revision;
- History boundary: no admitted Undo or Redo step exists at the current history
  position, so accepted state is unchanged;
- Idempotent replay: the same completed traversal and retry identity is repeated
  and history does not advance again;
- Stale current revision: another accepted mutation changed the revision or
  history position before the traversal could commit;
- Cancelled before commit or known no-commit failure when the implementation can
  prove no traversal committed.

A lost transport response remains an Unknown transport outcome at the caller,
not a fabricated history result. Same-retry recovery resolves the committed or
non-committed history operation inside the active session.

History boundary is not represented as an error that should be retried forever.
Read-only history inspection can expose whether Undo or Redo is currently
admitted so an agent need not probe mutation merely to discover availability.

Final wire enum names may differ, but browser, CLI, and MCP adapters preserve
these application meanings.

### Retry and lost receipts

Undo and redo are mutating application capabilities and therefore use their own
retry identities or equivalent idempotence mechanism admitted by the final
history protocol.

If a history traversal commits but its receipt is lost in transport, same-retry
recovery returns the committed traversal result instead of traversing one more
history step.

A caller does not create a new retry identity merely because it lost the first
Undo or Redo response. Retry identities remain ephemeral session data and are
not history credentials.

### Concurrency

Two concurrent mutating operations cannot both assume the same current revision
and commit independent history transitions. Commit-time revision checks
serialize the accepted result.

For example, an Undo racing with a semantic Apply may win or lose according to
the application commit boundary. The loser observes stale current state and must
inspect again before choosing its next operation.

### Selection and viewport state

Browser focus, selection, split ratio, zoom, and scroll are presentation state,
not accepted history authority. Undo and redo do not store DOM or canvas
snapshots merely to restore those values.

Receipts may identify semantically relevant changed objects so an adapter can
choose a useful focus target. That presentation handoff never changes which
semantic history state was accepted.

### Session lifetime

History exists only for the active in-memory session unless an explicit export
contract deliberately serializes admitted provenance or document history.
Closing an unexported session destroys the notebook history, history retry
state,
and redo state with the rest of the session.

Crash recovery, browser form restoration, or hidden files do not become an
alternate persistent undo implementation.

### Adapter parity

Direct, browser-assisted, CLI, and MCP entry paths call the same application
history services. Equivalent Undo or Redo at the same history position produces
the same restored semantic state and dependency impact.

MCP cannot bypass history by editing serialized internal snapshots, and the
browser cannot implement authoritative undo by manipulating preview DOM.

### Implementation evidence

The current in-memory semantic session stores Undo and Redo snapshots behind one
exact-current-revision history authority. A synchronized fixture races Undo
against semantic Apply from the same current revision and proves exactly one can
commit while the loser reports the winner's fresh revision as stale.

A separate fixture establishes a Redo branch, then submits semantic no-op,
semantic-rejected, and resource-rejected Apply attempts. None changes the
current revision or destroys the branch, and the original Redo remains
traversable.

Candidate replacement also branches through the same history authority. After
Undo abandons one candidate snapshot, accepting another candidate clears Redo
without reusing any accepted semantic identity still present in that abandoned
snapshot.

Without a history retry identity, replaying a completed traversal against its
old exact base returns stale and cannot traverse again. This prevents duplicate
effects but does not satisfy lost-receipt recovery, because base and direction
alone cannot distinguish an idempotent retry from a separate caller operation.

## Failure Modes

The contract fails if Undo or Redo mutates DOM or preview snapshots as document
authority, regenerates unrelated stable semantic identities, or creates an
ambiguous redo after a new branch is accepted.

It fails if history traversal reuses an old revision identity for a new current
revision, if a lost receipt can advance history twice, or if concurrent mutation
can reverse a different state than the caller preconditioned.

It also fails if undoing an inserted object and redoing it allocates a different
accepted semantic identity without an explicit migration reason, or if history
persistence silently survives the disposable session.

## Verification

The first user-journey text correction commits one transaction. Undo creates a
new accepted revision whose paragraph text matches the pre-correction semantic
state while unrelated identities remain stable; redo creates another new
revision restoring the corrected text.

An insertion fixture commits a new block, records its accepted semantic
identity,
undoes the insertion, and redoes it. The restored block uses the same accepted
semantic identity and derived recomputation follows the owning flow
dependencies.

A branching fixture commits edits A and B, undoes B, then commits C. Redo of the
old B path is no longer admitted as the active redo sequence.

A lost-receipt fixture drops the response after Undo commits. Repeating the same
history operation with the same retry identity returns the prior result and does
not undo transaction A as an accidental second step.

A race fixture starts Undo and a new semantic Apply from the same current
revision. At most one commits from that base; the other observes stale state and
must inspect the resulting history position.

Parity fixtures perform equivalent history traversals through direct, browser,
CLI, and MCP paths and compare restored semantic state, stable identities,
transaction provenance, and dependency-expanded impact.
