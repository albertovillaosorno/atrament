# Mathematics, tables, and structured blocks

## Status

Accepted.

## Decision ID

`atrament.document.mathematics-tables-and-structured-blocks`

## Context

Technical notes contain formulas, aligned derivations, ruled tables, diagrams,
and boxed facts that cannot be represented faithfully as ordinary prose. The
source must remain editable and machine-operable while the result should look
deliberately written and ruled by hand.

## Decision

Mathematics remains semantic TeX-compatible content through parsing, editing,
and layout. Formula rendering supplies measured glyph geometry to page layout;
it does not flatten the source into an opaque image before placement.

Tables, boxes, separators, and simple diagrams use typed geometry with style
roles for hand-drawn, ruler-straight, highlighted, or printed treatment. AI and
clipboard adapters must preserve supported structure and emit unresolved input
when they cannot prove a faithful mapping.

The initial school-note vocabulary includes headings, dates, paragraphs, lists,
definitions, quotations, citations, footnotes, tables, aligned calculations,
equations, diagrams, arrows, labels, callouts, dividers, and page references.
Every block participates in measurement, collision, and pagination.

## Consequences

- Formulas can be copied, searched, corrected, and reformatted.
- Tables align with page geometry while retaining a chosen handmade character.
- Unsupported TeX or complex diagrams require explicit fallback behavior.
- Formula metrics become a versioned rendering input.

## Rejected Alternatives

- Rasterizing every formula at ingestion was rejected because it destroys
  editability and semantic reuse.
- Replacing formulas with handwritten Unicode approximations was rejected
  because notation coverage and alignment would be unreliable.
- Allowing AI to silently rewrite malformed mathematics was rejected because
  plausibility is not equivalence.

## Verification

Fixtures must cover inline mathematics, displays, aligned equations, fractions,
matrices, tables, boxes, and failure cases. Round trips must preserve source
semantics and report unsupported constructs without silent substitution. A
representative bilingual assignment must exercise every initial block family.

### Implementation evidence

The backend now has a dependency-free mathematical source analyzer that retains
exact UTF-8 source and exposes structural spans for the admitted first slice:
groups, scripts, fractions, binomial coefficients, square roots, grouped
mathematical alphabets, common one-group accents, upright unit or label groups,
grouped text and custom operator names, vector, overline, and underline
decorations, escaped TeX special characters, standard Greek control-sequence
notation, common named symbols, relation and logic symbols, operators, calculus,
set notation, aligned separators, and matrix environments. Unknown control words
and
unadmitted control symbols remain
explicit unsupported constructs, and malformed source returns typed syntax
failures.

Semantic formulas carry an explicit inline, display, or aligned mode. Candidate
acceptance validates supported mathematics before identity promotion, while the
existing unresolved-block family can preserve unsupported source exactly rather
than rewriting it. Accepted formulas can also be replaced against an exact base
revision while retaining their stable semantic identity.

Expression-tree completeness, component-level formula editing, canonical
serialization/migration round trips, formula measurement, glyph geometry,
math-specific diagnostics, and output consumption remain future verification.

Table rows now retain an explicit semantic `Header` or `Body` role through
candidate acceptance. The accepted application boundary can replace that role
against an exact base revision without changing the row identity or cell
contents, including rows inside callouts, list items, and nested table cells.

Table cells now own explicit nonzero logical row and column spans. The first row
establishes the table's logical width from its ordered cell spans. Later rows
fill every unoccupied logical column from left to right while row spans from
prior rows reserve their covered columns.

Candidate acceptance rejects cells that cross reserved columns or table width,
row spans that extend below the table, and rows that leave logical columns
uncovered. Accepted identity promotion preserves each span exactly. Grid
validation tracks occupied span intervals rather than expanding one object per
logical column, including at the maximum admitted column-span value.

The same grid invariant now validates an exact-base direct cell-span edit. A
valid change preserves the cell identity and child blocks, commits one accepted
revision, and participates in semantic history. A change that would invalidate
the owning merged table is a typed no-effect. Cell spans are also admitted as
generic Structured-content editable values.

Ordered command simulation validates a cloned owning-table candidate, while
batch Apply overlays coalesced span changes and validates every affected final
table before one atomic commit.

Cell alignment and wrapping, ruler-like border geometry, table measurement, and
output consumption remain future verification. The span model does not choose
wire field names or physical cell geometry.
