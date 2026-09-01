# Semantic command capability matrix

## Status

Frozen for the semantic command-mode design.

## Purpose

This matrix defines the minimum semantic mutation families that command mode
must
support without freezing final wire operation names. It makes bounded LLM edits
implementable while preserving the revision, adapter, and hardware boundaries
already accepted by Atrament.

## Scope

The matrix covers commands that can mutate one accepted notebook revision. It
does not define JSON field names, HTTP routes, MCP tool names, asset ingestion,
file export, browser viewport state, or physical device execution.

One future wire protocol may project a family into several concrete typed
commands. Family names in this document describe application semantics rather
than serialized operation strings.

## Contract

### Common command rules

Every semantic command belongs to one admitted family and one accepted base
revision. It targets stable semantic identities, backend-admitted insertion
anchors, or revision-owned constraints.

Existing accepted identities are referenced, not reconstructed from document
paths. New accepted identities are allocated or admitted by the backend during
validation and apply; a model cannot forge authority by inventing an ID that
collides with accepted state.

Each family declares the authority it can mutate. A command that includes fields
outside that authority is invalid rather than permission to mutate additional
state.

The backend derives the semantic change set and dependency-expanded impact set.
An agent may explain expected impact, but that explanation never controls
invalidation.

### Capability matrix

#### Text content

Authority: text owned by an existing semantic text identity.

Effect: correct or replace a paragraph, heading, list item, callout, or admitted
inline span without replacing its semantic owner.

Impact: shaping, wrapping, flow geometry, diagnostics, rendering, handwriting,
and motion projections that depend on the changed text.

#### Structured content

Authority: typed children inside mathematics, tables, lists, and other admitted
structured blocks.

Effect: change one formula component, table cell, list item, or typed child
without flattening the parent structure to prose.

Impact: structure validation plus layout and output dependencies of the owning
block.

#### Block insertion and deletion

Authority: semantic block membership in an admitted flow or region.

Effect: insert a new typed block at an admitted anchor or delete an existing
block.

Impact: owning flow, pagination, following geometry, diagnostics, and affected
outputs.

#### Ordering and grouping

Authority: semantic order and admitted grouping relationships.

Effect: move a block relative to stable anchors or change an admitted group
without mutating page pixels.

Impact: affected flow or group geometry and its downstream outputs.

#### Provenance

Authority: revision-owned source, claim, citation, and unresolved-state
metadata.

Effect: attach, correct, or remove provenance metadata without silently changing
claim text.

Impact: source and citation diagnostics plus outputs that expose provenance.

#### Style role

Authority: admitted semantic style assignment or revision-owned style override.

Effect: change a block role, emphasis, handwriting role, or admitted material
treatment.

Impact: measurement when style changes metrics, rendering, handwriting,
capability diagnostics, and affected outputs.

#### Spatial constraint

Authority: revision-owned placement, size, crop, anchor, alignment, and layer
constraints.

Effect: express a human move, resize, crop, align, or layer operation as typed
intent.

Impact: owning page or region layout, collisions, bounds, render, and capability
projections.

#### Asset reference

Authority: semantic references to already admitted asset identities.

Effect: attach, replace, or remove an admitted image or media reference without
embedding new asset bytes.

Impact: media diagnostics, layout, rendering, and output capabilities that
depend on the asset.

#### Document constraint

Authority: accepted paper, page, flow, style, or other output-relevant
constraint owned by the notebook revision.

Effect: change an intentionally global document constraint.

Impact: dependency expansion may invalidate the complete derived notebook.

### Text and structure boundaries

A text command edits semantic text; it does not replace a table, formula, image,
or page with an arbitrary string. Structured families preserve the typed
container and reject unsupported semantics instead of flattening them.

Changing a complete block type is a structural operation. It must pass the same
model and provenance validation as an equivalent human edit or initial candidate
acceptance.

### Identity and insertion behavior

Delete targets an existing accepted identity. Move and insert use stable
backend-admitted anchors rather than array indexes or page coordinates.

When an insertion needs a new semantic identity, the application owns the
accepted identity allocation. A returned command may carry a batch-local handle
for later commands in the same batch only if the final wire contract explicitly
admits that mechanism.

A later command in one batch may depend on an earlier insertion only through an
admitted command dependency. Hidden reliance on mutation order is not accepted
as a substitute for typed intent.

### Asset boundary

Semantic commands may reference an asset already admitted to the active session
and accepted revision context. They do not download remote URLs, embed arbitrary
base64 payloads as document fields, or create undeclared persistent files.

Asset import and media ingestion are separate application capabilities. After an
asset is admitted and receives an identity, a semantic batch may attach or place
that identity through the normal revision transaction.

### Spatial command parity

Human direct manipulation and command mode converge on the same revision-owned
constraints. A drag is not privileged merely because it came from pointer input,
and an LLM cannot bypass placement diagnostics merely because it emitted the
constraint textually.

Moving, resizing, cropping, aligning, or layering an object therefore produces
the same accepted constraint semantics whether requested by the human editor,
CLI, or MCP.

### Document-wide constraints

Some commands are intentionally broad. Changing paper geometry, a global style,
or another accepted document-wide constraint can make the complete derived plan
stale even though only one semantic command was applied.

Impact-scoped recomputation does not promise small impact for every small batch.
It promises that invalidation follows real dependencies instead of blindly
regenerating everything or trusting an agent-provided impact list.

### Commands outside semantic batches

The following capabilities are not semantic notebook batch families:

- clipboard read or write;
- browser focus, selection, zoom, split ratio, or scroll position;
- raw asset-byte ingestion or remote download;
- import from or export to a persistent file;
- runtime health, process, or adapter lifecycle control;
- undo or redo of already accepted application history;
- physical device connect, identify, home, arm, start, pause, resume, cancel, or
  safe stop;
- raw device motion or vendor command streams.

CLI and MCP may expose admitted application capabilities for some of these
operations. They remain separate from the notebook revision transaction and keep
their own validation, lifecycle, provenance, and safety rules.

Undo and redo replay accepted history through the application core. They do not
permit a model to hide history traversal inside a new semantic edit batch.

### Receipt expectations

A normalized receipt identifies each command family and per-command outcome. It
reports accepted semantic identities that changed and the dependency-expanded
derived regions invalidated by the complete batch.

For insertions, the receipt exposes the accepted identity allocated by the
backend. For deletions, it identifies the removed accepted identity. For a
no-op,
it reports no semantic change and does not create revision churn.

The receipt does not claim that every invalidated region changed visibly. It
reports which derived results had to be considered stale and recomputed for
correctness.

## Failure Modes

The matrix fails if a command family can mutate authority outside the accepted
notebook revision, or if storage paths, DOM nodes, page pixels, remote URLs, raw
asset bytes, or device commands become semantic edit targets.

It also fails if text operations flatten supported structured content, if
insertion trusts agent-forged accepted identities, or if an asset reference can
bypass the admitted ingestion boundary.

Direct manipulation and command mode must not create different source semantics
for the same placement intent. Impact calculation must not assume a
document-wide
rebuild when dependencies prove a bounded result, nor omit dependencies merely
to keep the impact set small.

## Verification

The `sober-single-pen` `Idea` correction exercises the text family. Only that
paragraph's semantic content changes, while flow-dependent geometry may expand
the derived impact set.

A table fixture must change one cell while preserving the table identity and all
unrelated cells. A structural fixture must insert and then move a block through
stable anchors without using serialized array indexes.

A provenance fixture must correct citation metadata without changing claim text.
An asset fixture must reject raw bytes or a remote URL in a semantic batch, then
accept a reference to an asset identity admitted by the ingestion boundary.

A spatial fixture must compare one drag-produced constraint with the equivalent
semantic command through direct, CLI, and MCP entry points. Accepted source and
normalized impact receipts must match.

A document-wide paper or style fixture must prove that one accepted command can
legitimately invalidate the full derived notebook when every page depends on the
changed constraint.
