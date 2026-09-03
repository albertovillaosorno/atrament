# Semantic inspection and command-context contract

## Status

Frozen for first-release semantic inspection and command-context projection.

## Purpose

This contract defines the read-only application boundary used to inspect an
accepted Atrament revision and derive bounded command context for human, CLI,
and MCP editing workflows. It prevents agents and browser presentation from
turning storage shape, hidden runtime state, or readable context into mutation
authority.

## Scope

The contract covers accepted-revision binding, semantic selectors, bounded
inspection, completeness, continuation, capability binding, context identity,
readable context, writable scope, secret exclusion, diagnostics, and adapter
parity.

It does not freeze final wire fields, query syntax, pagination token encoding,
MCP tool names, database indexes, storage paths, or browser request routes.

## Contract

### Read-only inspection

Inspect is a read-only application capability. It can return accepted semantic
state, stable identities, revision-owned constraints, provenance, diagnostics,
capability metadata, and bounded derived summaries required for a caller's next
explicit operation.

Inspect never mutates the accepted notebook, application history, files,
clipboard, adapters, or hardware. Reading an identity does not make it writable.

### Accepted revision binding

Every inspection names the accepted revision it intends to observe or explicitly
requests the current accepted revision through the owning application boundary.
The result identifies the revision actually inspected.

A caller does not receive content from a newer revision under an older revision
identity merely because editing advanced while inspection was running. The
backend returns the admitted stale/current outcome for the final inspect
protocol rather than silently substituting authority.

### Semantic selectors

Inspection selects semantic authority through backend-owned selectors such as
accepted identities, semantic owners, admitted sections, source relationships,
diagnostic owners, or another versioned semantic query class.

Selectors do not expose serialized object offsets, DOM nodes, CSS selectors,
canvas coordinates, repository paths, memory addresses, or database keys as
notebook authority.

An inspection selector can request neighboring context for understanding. That
read expansion does not grant mutation permission to the returned neighbors.

### Capability snapshot binding

Inspect can expose the backend-owned capability snapshot required by command
mode. Protocol versions, command families, behavior versions, and relevant
resource bounds are application metadata, not notebook content or credentials.

When behavior-significant capability metadata changes, an older command context
is not silently reinterpreted under the new behavior. A caller obtains fresh
capability discovery and a fresh context when the typed application outcome
requires it.

### Bounded inspection

The backend owns inspection resource limits. A request can be bounded by
semantic result count, context size, diagnostic volume, or another admitted
application limit without making the browser responsible for truncation.

A bounded Inspect result states whether the requested semantic projection is
complete for that request. The backend never silently drops identities,
diagnostics, provenance, or values and presents the remainder as a complete
answer.

When a larger read is admissible, the result may expose continuation semantics
owned by the application protocol. Continuation remains tied to the inspected
revision, selector meaning, and behavior identity that produced it.

A continuation identifier is not a notebook identity, retry identity, mutation
permission, browser secret, or MCP credential. If its owning revision or query
meaning is no longer admitted, the caller restarts inspection instead of
combining pages from incompatible snapshots.

### Deterministic inspection meaning

For one accepted revision, selector, capability behavior, and admitted inspect
options, repeated inspection returns equivalent semantic meaning independent of
adapter formatting, wall-clock time, or map iteration order.

Operational timing, request IDs, transport pagination framing, and prose
formatting do not become semantic inspection data merely because a receipt
contains them.

### Command-context derivation

Command context is a backend-owned read-only projection derived for one accepted
base revision and one admitted edit intent. It is not assembled by the browser
from arbitrary Inspect fragments.

The context identifies at least the behavior identity used to interpret it,
base revision, readable semantic context, writable identities or insertion
anchors, admitted command families, relevant constraints, local precondition
material, resource bounds, and its own command-context identity.

The backend may use prior inspection as input to choosing that projection, but
it independently verifies every authority placed in the context.

### Read context versus write scope

Readable context can be wider than writable scope so a model or human can reason
about neighboring content, dependencies, provenance, and constraints. Only the
backend-declared writable identities, anchors, and admitted command families can
be mutated by a returned semantic batch.

Echoing a readable identity in the response does not promote it into writable
scope. A genuinely broader edit requires a new intentionally broader command
context or another admitted workflow.

### Context completeness

A command context contains the evidence the backend knows is required for the
requested bounded edit. Required surrounding content, constraints, provenance,
owner relationships, and precondition values cannot be silently omitted merely
to fit a context budget.

If a safe command context cannot fit the admitted limits, the backend returns an
unrepresentable, resource-limit, or other typed non-mutation outcome and may
suggest a smaller intent, another bounded context, or complete-candidate mode.
It does not ask the model to infer missing document authority.

Inspect pagination does not weaken this rule. A caller cannot concatenate an
arbitrary subset of inspect pages and declare the result a valid command
context.

### Instruction and data separation

Notebook, task, source, citation, and asset-derived prose returned by Inspect or
placed in command context remains domain data. Text that resembles agent
instructions does not widen capabilities, write scope, file access, or hardware
authority.

The application never inserts browser session secrets, MCP admission material,
internal repository paths, hidden temporary paths, retry identities from
unrelated capabilities, or other runtime credentials into model-readable
semantic context.

### Context identity

Equivalent command-context inputs under the same behavior version produce the
same context identity according to the final identity contract. Inputs include
at least the base revision, requested intent, selected readable context,
writable scope, admitted command families, relevant constraints, and capability
behavior identity.

A change that can alter command interpretation or admitted mutation changes the
context identity. Context identity is correlation metadata, not authentication
or permission by itself.

### Diagnostics and provenance

Inspect and command-context generation use the frozen typed diagnostic envelope
contract. Diagnostics can identify omitted unavailable data, unsupported
selectors, stale inputs, unresolved provenance, or capability limits without
changing accepted state.

Semantic source provenance stays attached to the semantic authority being read.
Transaction provenance can explain how the current accepted revision was
produced without storing complete external conversations or clipboard archives.

### Adapter parity

Direct, browser-assisted, CLI, and MCP inspection dispatch through the same
application semantics. Equivalent requests observe equivalent accepted semantic
content, completeness, diagnostic meaning, and capability behavior.

The browser may display or copy a backend-generated command context. It does not
parse semantic authority, widen scope, paginate notebook state independently, or
construct a second command-context implementation in TypeScript.

### Implementation evidence

The semantic notebook domain now exposes a read-only identity descriptor
covering
notebook, page, flow, block subtype, inline span, formula, list and item,
figure,
table, row, cell, page profile, asset, constraint, output profile, provenance,
and style identities. Each descriptor carries the direct structural owner, with
the notebook root represented explicitly as having no owner. Nested semantic
content is resolved by stable identity rather than serialized offsets or page
coordinates.

The current session application also admits one deliberately narrow Inspect
foundation: inspect one accepted identity against one exact revision. A
successful read returns the named revision, target identity, semantic kind, and
direct owner without changing accepted state. Stale revisions, missing targets,
and an empty session return typed read-only outcomes.

A second internal Inspect foundation walks only the target's backend-owned
structural-owner ancestry. The caller supplies a result bound; the response is
explicitly complete when it reaches the notebook root or incomplete when the
bound stops earlier, naming the first omitted target-or-owner chain identity.
That identity remains ordinary semantic data and is not a continuation token,
retry identity, credential, or writable-scope grant.

The active-process `SessionApplication` delegates both exact identity inspection
and bounded ancestry to its owned semantic authority. This keeps runtime
composition from reaching into the concrete semantic service while still adding
no transport route or advertised Inspect protocol.

A read-only local-precondition checker can require an exact semantic kind, an
exact direct owner, notebook-root ownership, or no owner constraint. Wrong kind
and wrong owner are distinct typed failures, and stale revision rejection occurs
before local comparison.

The session can also derive one target's local command material directly from an
exact accepted revision. That material combines semantic kind, direct owner,
exact editable base value when established, and the currently executable direct
edit family. A combined checker validates requested family, kind, owner, and an
optional exact base value against that one projection.

Command capability discovery is now deterministic and versioned independently
from notebook revisions. It reports only the three family behaviors with current
direct-edit targets, while command protocol, normalization, context generation,
Validate, Apply, selective rebatching, and their numeric resource limits remain
unadvertised. A caller can detect a bound capability-version mismatch read-only.
This still does not define final command-context identity, readable-neighbor
selection, or wire fields.

This is not the complete first-release Inspect protocol. Bounded multi-object
selectors, completeness and continuation semantics, capability snapshots,
diagnostics, provenance projection, command-context generation, context
identity, resource limits, and direct/CLI/browser/MCP parity remain open.

## Failure Modes

The contract fails if Inspect mutates accepted state, silently mixes revisions,
exposes storage or DOM shape as semantic authority, or returns a truncated
result
without explicit incompleteness.

It fails if continuation can combine incompatible revisions, if continuation or
context identity becomes a credential, or if adapter-local pagination changes
semantic meaning.

Command-context safety fails if readable context implicitly becomes writable,
if required evidence is silently omitted to meet a budget, if the browser builds
its own authoritative context, or if source prose can grant application
capabilities.

Privacy fails if browser session secrets, MCP credentials, unrelated retry
identities, internal paths, or hidden runtime state are exposed as ordinary
model-readable context.

## Verification

A deterministic-inspect fixture repeats one semantic inspection against the same
accepted revision through direct, CLI, browser-assisted, and MCP paths. Stable
semantic identities, values, provenance, diagnostics, and completeness agree.

A bounded-inspect fixture requests more semantic objects than one admitted
response can carry. The first result explicitly reports incompleteness and an
admitted continuation. Following it against the same snapshot returns the
remaining semantic content without duplication or silent omission.

A continuation-stale fixture advances the accepted revision before following a
continuation. The backend refuses to combine incompatible snapshot pages and the
caller restarts inspection.

A bounded-command-context fixture exposes three neighboring paragraphs for
reading while admitting only the middle paragraph for text mutation. A returned
batch targeting either neighbor is rejected for writable-scope violation.

A context-completeness fixture requires a constraint or provenance fact that
cannot fit the admitted command-context budget. Generation returns a typed
non-mutation outcome rather than omitting the fact and asking the model to infer
it.

A hostile-source fixture includes prose requesting file access, wider scope,
credential disclosure, and hardware motion. Inspection preserves the prose as
data while command context exposes none of those capabilities unless they were
independently admitted by the application.

A privacy fixture scans generated command context for the browser session
secret, MCP admission material, unrelated retry identities, and internal runtime
paths. None appears in model-readable semantic data.
