# Product story and repository metadata

## Status

Accepted.

## Decision ID

`atrament.governance.product-story-and-repository-metadata`

## Context

Atrament combines uncommon mechanisms, but repository visitors first need to
understand the transformation it performs. Listing languages, frameworks,
architecture patterns, or component brands makes a distinctive product sound
like a generic implementation exercise.

## Decision

Public summaries lead with what Atrament does: it turns text, lectures,
formulas, and photos into uncanny handwritten notebooks on PDF or real paper.
Repository topics describe observable capabilities and uses. Implementation
details belong in technical documentation and never displace the product story.

The preferred voice is elegant, strange, and concrete. It may call Atrament an
artificial penmanship studio, but it must not claim completed behavior without
evidence.

## Consequences

- The GitHub description remains product-first.
- Topics favor handwriting, notebooks, transcription, and physical output.
- Future README introductions explain the experience before the stack.
- Technical depth remains available without becoming the elevator pitch.

## Rejected Alternatives

- A dense stack inventory was rejected because it explains construction, not
  purpose.
- A generic productivity description was rejected because it erases the
  project's unusual physical and personal character.
- Marketing claims without implementation evidence were rejected because they
  would confuse intent with delivered behavior.

## Verification

Review the repository description, topics, README introduction, and release
copy together. A reader unfamiliar with the code must be able to state the
inputs, transformation, and outputs without first decoding implementation
terminology.
