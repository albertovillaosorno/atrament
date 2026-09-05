# Semantic notebook model

## Status

Accepted.

## Decision ID

`atrament.document.semantic-notebook-model`

## Context

Atrament must accept plain writing, formulas, photos, tables, transcript
fragments, headings, dates, highlights, and margin annotations. Treating these
as undifferentiated drawing commands would make correction, reflow, AI use, and
alternative output unreliable.

## Decision

The authoritative document is a typed notebook model with stable identities for
notebooks, pages, flows, blocks, inline spans, media, styles, and provenance.
Content meaning is separate from its chosen handwriting, page placement, and
device motion.

Blocks include paragraphs, definitions, lists, headings, dates, mathematics,
tables, figures, callouts, rules, and explicit freeform regions. Unsupported or
ambiguous input remains a typed unresolved block rather than being discarded or
guessed into a supported form.

## Consequences

- Content can be edited without regenerating unrelated notebook regions.
- Layout and handwriting are projections of stable semantic identities.
- AI tools operate on a bounded semantic model instead of arbitrary canvas
  mutations.
- Model evolution requires explicit migrations and compatibility tests.

## Rejected Alternatives

- A flat stream of pen strokes was rejected because it cannot preserve formulas
  or editable structure.
- HTML or office-document state as authority was rejected because their layout
  and mutation semantics do not match the physical notebook contract.
- Images of complete pages as authority were rejected because they are output,
  not editable source.

## Verification

Round-trip fixtures must serialize, reopen, edit, and render every block family
without identity loss. Model tests must reject unknown required semantics while
preserving explicitly admitted extension data.

### Implementation evidence

Definition blocks are now executable semantic values with their own block kind.
Their inline spans participate in the existing candidate identity graph,
accepted-identity promotion, exact Text-content editing, Style-role and
Provenance material, identity inspection, and semantic Undo/Redo without being
reclassified as paragraphs. Layout-specific definition presentation remains a
later measurement/style concern rather than semantic storage behavior.
