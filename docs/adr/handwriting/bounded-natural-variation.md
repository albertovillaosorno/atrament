# Bounded natural variation

## Status

Accepted.

## Decision ID

`atrament.handwriting.bounded-natural-variation`

## Context

Perfect repetition looks synthetic, while independent random noise destroys a
writer's identity and legibility. Users also want direct control over extremes
such as slant, roundness, flattening, height, width, spacing, and vertical
pressure without editing individual glyphs.

## Decision

Every variable parameter has typed units, an observed or authorized minimum and
maximum, a central tendency, a distribution family, correlation groups, and
context rules. Variation is sampled from a document seed and stable semantic
identities so repeated renders can be reproduced or intentionally regenerated.

Parameters operate at distinct scales: profile, document, page, line, word,
character, and stroke. Slow correlated drift models rhythm across a line while
local alternatives prevent exact repetition without becoming white noise.

## Consequences

- Users can widen or narrow natural behavior deliberately.
- Rendering can be repeatable for debugging and export.
- Profile calibration must distinguish measured ranges from creative presets.
- Correlated models require statistical and perceptual validation.

## Rejected Alternatives

- Unseeded randomness was rejected because output could not be reproduced.
- Independent jitter on every point was rejected because it creates vibration,
  not handwriting.
- Fixed glyph alternates alone were rejected because line-level rhythm and
  continuous deformation would remain absent.

## Verification

Property tests must prove every sampled value respects configured bounds and
replays from the same seed. Statistical fixtures must detect frozen repetition,
excessive noise, lost correlations, and illegible extreme configurations.
