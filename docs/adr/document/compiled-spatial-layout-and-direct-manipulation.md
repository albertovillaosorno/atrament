# Compiled spatial layout and direct manipulation

## Status

Accepted.

## Decision ID

`atrament.document.compiled-spatial-layout-and-direct-manipulation`

## Context

Users need to type and preview content, then move and resize photographs with
the immediacy of a visual editor. Fully unconstrained office-style layout would
make output unstable, while source-only markup would make ordinary spatial
adjustments needlessly indirect.

## Decision

Atrament uses compiled layout with constrained direct manipulation. Semantic
flows produce a reproducible page plan from physical geometry and style rules;
user placement creates explicit constraints, anchors, crops, and overrides that
become part of the source document.

The primary desktop composition is a 16:9 split workspace with both authoring
surfaces visible. The LLM editor owns structured source, task instructions, and
the one-shot chat exchange; the human editor owns selection, text correction,
dragging, resizing, layering, cropping, and visual constraint adjustments.

Preview is never the sole authority. Dragging a photo, resizing a callout, or
pinning a title updates typed constraints and triggers a new compiled plan with
diagnostics for overflow, collision, or impossible placement.

Content never disappears beyond a page edge. Flowing blocks paginate, fixed
objects report an out-of-bounds error, and the user chooses whether to move,
resize, crop, or create another page.

## Consequences

- Visual operations remain reproducible and inspectable.
- Reflow can preserve deliberate composition rather than guessing from pixels.
- The editor needs clear feedback when a constraint cannot be satisfied.
- Freeform pages remain possible through explicit regions, not hidden offsets.
- Both editors remain synchronized through stable semantic object identities.

## Rejected Alternatives

- A Word-like mutable page tree was rejected because implicit layout state is
  difficult to reproduce across outputs.
- Pure TeX source editing was rejected as the only interface because photo
  placement and visual correction need direct manipulation.
- Saving only final coordinates was rejected because page or profile changes
  require intent, anchors, and constraints to recompile safely.

## Verification

Golden fixtures must produce the same plan from identical document, profile,
paper, and asset inputs. Editor tests must prove that every direct manipulation
has an equivalent serialized constraint and survives reflow. Viewport tests
must prove the 16:9 split, focus behavior, and page-overflow diagnostics.
