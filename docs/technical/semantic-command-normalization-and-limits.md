# Semantic command normalization and limits

## Status

Frozen for semantic command protocol implementation.

## Purpose

This contract defines the semantic equality and resource-bound rules required by
retry identity, adapter parity, and autonomous MCP use. It removes ambiguity
from
phrases such as "same normalized batch" without freezing final JSON field names
or a concrete serialization.

## Scope

The contract covers parsed command envelopes after protocol-version dispatch. It
defines normalized semantic identity, order, command dependency validity,
resource-limit behavior, and parity expectations.

It does not define a byte-level canonical JSON encoding, hash algorithm, numeric
limit values, HTTP limits, MCP transport framing, or implementation-specific
data
structures.

## Contract

### Version-owned normalization

Each admitted command protocol version owns one deterministic normalization
procedure. All direct, CLI, clipboard-assisted, and MCP adapters dispatch to
that
same procedure after parsing the versioned envelope.

Normalization produces the semantic representation used for retry comparison,
receipt parity, and deterministic command validation. It is not an additional
document authority and does not mutate the accepted notebook.

A protocol upgrade may change normalization only through explicit versioning and
compatibility rules. Two adapters cannot choose different normalizers for the
same admitted protocol version.

### Semantic equality

Two parsed batches are the same normalized batch only when every
application-significant input agrees after version-owned normalization.

Application-significant input includes at least:

- protocol version;
- accepted notebook identity and base revision;
- command-context or prompt identity when the mode requires it;
- ordered command sequence;
- each command identity and family;
- semantic targets, admitted insertion anchors, and batch-local references;
- command preconditions and admitted dependencies;
- revision-owned values requested by each command.

Transport whitespace, browser text-control newline representation,
object-member serialization order, adapter-local request IDs, UI labels, logging
metadata, and receipt timestamps are not semantic merely because they appear in
one transport representation.

For the first-release browser text path, backend-presented command prompts use
canonical `LF` newlines. The final parser accepts the browser-observed
normalized
text without making `CRLF` versus `LF` a semantic batch distinction.

A final wire contract may admit explicitly preserved extension data. That
version must define whether an extension participates in normalized equality;
adapters cannot silently discard unknown application-significant data.

### Command order

Command order is significant unless a future command family explicitly defines
commutative semantics. Normalization does not sort commands merely to make two
batches compare equal.

A later command may observe an earlier command's simulated effect only through
an
admitted dependency or other ordering rule defined by its command family.
Changing meaningful command order therefore changes normalized batch content.

This prevents a retry identity from treating two behaviorally different ordered
transactions as the same request.

### Command identity uniqueness

Every command identity is unique within one batch. Duplicate command identities
are invalid even when their command bodies are byte-for-byte identical.

A command identity is local to the batch unless a future protocol explicitly
assigns a wider meaning. It is not an accepted notebook identity and cannot be
used to forge document authority.

If batch-local handles are admitted for newly inserted objects, handle names are
also unique within the batch and cannot collide ambiguously with accepted
semantic identities. Their normalized meaning includes the producing command and
admitted dependency relationships, not merely the handle's display spelling.

### Dependency validity

An explicit intra-batch dependency references an existing command identity or an
admitted batch-local handle according to the command family contract.

The command dependency graph must be acyclic. A direct self-dependency, missing
reference, or dependency cycle invalidates the complete batch before accepted
mutation.

The backend does not break cycles heuristically, reorder commands to guess model
intent, or partially apply the acyclic portion of an invalid dependency graph.

### Retry equality

A retry identity records the normalized semantic batch associated with its first
admitted Apply attempt according to the application retry contract.

The same retry identity plus the same normalized batch is idempotent. The same
retry identity plus different normalized semantic content is a conflict even if
the requested final notebook might coincidentally be equivalent.

Differences that are purely transport representation do not manufacture a retry
conflict. Differences in command order, targets, preconditions, dependencies, or
revision-owned requested values do.

### Receipt normalization

Normalized receipts compare semantic outcomes rather than adapter formatting.
Equivalent direct, CLI, clipboard-assisted, and MCP calls agree on result class,
base and result revision semantics, per-command outcomes, changed identities,
impact, and diagnostics required by the application contract.

Adapter-local request IDs, transport timing, terminal formatting, UI prose, and
other presentation metadata may differ without breaking parity.

When deterministic diagnostics contain an ordered semantic sequence, that order
is part of receipt semantics. Unordered sets are compared as typed sets rather
than by incidental storage iteration order.

### Backend-owned resource limits

The backend publishes or otherwise binds admitted resource limits to the active
protocol and application capability context. Final numeric values remain an
implementation and product-limit decision rather than a browser constant.

Command-mode limits must cover at least:

- maximum parsed envelope size;
- maximum command count per batch;
- maximum nesting or structured-value depth where applicable;
- maximum dependency-edge count;
- maximum readable command-context size;
- maximum writable-target or insertion-anchor count;
- bounded text or structured payload size per admitted command family.

Additional domain-specific limits may apply to tables, formulas, provenance,
constraints, or other structured authorities.

### Limit enforcement

Limits are validated before expensive candidate simulation whenever the required
information is available. Later domain validation may still reject a batch that
passes coarse resource bounds.

A batch that exceeds one required limit is rejected as a complete batch. The
backend does not silently truncate commands, text, dependencies, readable
context, writable scope, diagnostics, or semantic values to make the request
fit.

The model-facing command context communicates limits relevant to the requested
response so a caller can produce an admissible batch without hidden trial and
error.

### Context budgeting

Readable context is selected by the backend and remains distinct from writable
scope. Context budgeting may omit unrelated notebook regions, but it cannot omit
information the backend knows is required to perform the requested bounded edit
correctly.

When required context cannot fit the admitted command-mode budget, the backend
chooses a broader or different workflow, such as another bounded request or the
complete-candidate mode. It does not ask the model to infer missing authority.

MCP Inspect may expose a separately bounded inspection capability. A large
inspection result does not automatically widen one Apply operation's writable
scope or command-count limit.

### Failure isolation

Resource-limit, normalization, duplicate-identity, and dependency-graph failures
occur before accepted revision mutation. They do not create undo history or
partially update derived authorities.

Malformed or excessive command input does not authorize adapters to spill
semantic state to hidden files, browser storage, or a second mutation path.

### Implementation evidence

A dependency-free semantic-command-graph domain now validates the structural
graph rules that do not depend on final wire syntax: unique command identities,
existing dependency references, no direct self-dependency, and acyclicity. The
validator is generic over caller-owned identity representation and does not
reorder the supplied command sequence.

The graph validator is iterative; a 100,000-command dependency chain is covered
by direct executable evidence without recursive traversal. This does not define
a protocol-owned normalizer, command-count or edge-count limit, retry equality,
or batch Apply behavior. Those remain version-owned work for the future admitted
command protocol.

## Failure Modes

The contract fails if retry equality depends on raw JSON bytes, map iteration
order, adapter-specific serialization, or presentation metadata instead of the
version-owned semantic representation.

It fails if normalization reorders meaningful commands, accepts duplicate
command identities, guesses through missing dependencies, or breaks cycles by
partially executing a batch.

Resource safety fails if one adapter has materially weaker command limits than
another path to the same application capability, or if over-limit input is
silently truncated and then applied.

It also fails if frontend TypeScript defines protocol limits or normalization
rules independently from the backend command authority.

## Verification

An equivalent-serialization fixture sends two wire representations that parse to
the same version-owned semantic batch with harmless whitespace or member-order
differences. Retry comparison treats them as the same normalized batch.

An ordered-command fixture swaps two behaviorally significant commands while
keeping the retry identity. The second request is a retry conflict rather than
an
idempotent replay.

Duplicate-command-ID, missing-dependency, self-dependency, and dependency-cycle
fixtures all reject before accepted mutation. The accepted revision and undo
history remain unchanged.

Resource fixtures exercise each published bound at the admitted limit and one
step beyond it. Over-limit requests return typed diagnostics and never truncate
into an accepted transaction.

Parity fixtures feed equivalent parsed batches through direct, CLI,
clipboard-assisted, and MCP paths. Normalized receipts agree on semantic
outcomes even when adapter-local formatting, timing, or request metadata differ.
