# Continuous stroke synthesis

## Status

Accepted.

## Decision ID

`atrament.handwriting.continuous-stroke-synthesis`

## Context

Selecting static glyph outlines produces repeated letters, mechanical spacing,
broken joins, and line rhythm that looks typeset rather than written. Atrament
needs a representation that can connect character intent to continuous pen
motion and later physical-device execution.

## Decision

Handwriting is synthesized as time-ordered strokes with position, tangent,
curvature, width or pressure proxy, velocity, contact state, and semantic
origin. Characters contribute contextual stroke candidates with entry and exit
conditions rather than immutable complete outlines.

A planner selects, joins, deforms, and spaces strokes using the profile and
neighboring context, then emits an inspectable stroke plan. Raster, vector, PDF,
preview, and machine motion are projections of that plan.

## Consequences

- Ligatures and pen lifts become explicit behavior.
- One stroke authority can drive both simulated ink and a real pen.
- Planning is more complex than font substitution and needs robust diagnostics.
- Editing a semantic span can invalidate only its dependent stroke region.

## Rejected Alternatives

- Static font glyph selection was rejected because repeated contours and joins
  remain visibly mechanical.
- Raster patches as the canonical handwriting unit were rejected because they
  cannot produce resolution-independent geometry or device motion.
- Direct machine paths without semantic origin were rejected because correction
  and provenance would be lost.

## Verification

Fixtures must cover isolated letters, cursive joins, print handwriting, pen
lifts, punctuation, numerals, formulas, line wrapping, and edit-local
replanning. Every stroke must trace back to a semantic span and profile choice.
