# Pen, ink, and paper contact model

## Status

Accepted.

## Decision ID

`atrament.physics.pen-ink-paper-contact-model`

## Context

Ballpoint and marker traces change with contact angle, normal force, motion,
ink transfer, paper texture, and absorption. A decorative blur or opacity
jitter may look plausible at a glance but cannot support measured presets or
correspondence with a physical pen. Hidden variables such as microscopic ball
rotation or ink rheology are also not observable from ordinary calibration
samples well enough to justify pretending they are known state.

## Decision

Atrament uses an empirical contact-and-transfer model fitted to measurable
trace behavior. Stroke centerlines remain geometric authority. Material state
may change width and appearance around that centerline, but it cannot move the
semantic geometry or silently alter a live motion plan.

The admitted model inputs are physical position, direction, curvature, speed,
dwell time, contact state, and a pressure value. Pressure is expressed in force
units only when the capture or device is calibrated for force; otherwise it is
a named dimensionless proxy. Pen angle or other variables are inputs only for a
preset whose calibration evidence measured them.

A preset maps those inputs and seeded paper coordinates to bounded observable
outputs: trace width, coverage, starvation, pooling, edge displacement, and
absorption or drying response. Calibration may use lookup tables or smooth
fitted functions, but every parameter records units, evidence, confidence,
valid input range, and error measures. Extrapolation beyond that range is an
explicit diagnostic rather than a hidden physical claim.

Microscopic ball rotation, fluid chemistry, fiber mechanics, and other latent
phenomena are not independent simulation state in the first release. They may
motivate a fitted parameter or a later model only when controlled evidence can
identify it. Fast and final rendering use the same calibrated functions and
seed; quality profiles may change sampling density or texture resolution, not
the physical interpretation of a preset.

## Consequences

- Digital material behavior is tied to observations rather than decorative
  randomness or inaccessible microscopic assumptions.
- Geometry, live motion, and simulated material appearance retain separate
  authorities.
- A normalized pressure proxy cannot be mislabeled as Newtons.
- Calibration requires controlled traces and imaging across the claimed range.
- Better physical models can replace fitted functions behind the same measured
  contract when evidence justifies them.

## Rejected Alternatives

- Pure visual noise was rejected because it has no stable relationship to pen
  motion or media.
- A molecular fluid or ball-bearing simulation was rejected because ordinary
  calibration cannot identify enough of its hidden state for honest presets.
- Inferring unmeasured force from a generic pressure proxy was rejected because
  the units would imply evidence that does not exist.
- Marketing presets named after media without measurements were rejected because
  names are not calibration evidence.

## Verification

Benchmarks must compare controlled lines, curves, corners, pauses, speed
changes, and pressure changes against photographed physical traces. Fixtures
must prove output bounds, seeded replay, fast/final agreement of model inputs,
and explicit refusal or warning outside calibrated ranges. Each preset states
its validated pen, ink, paper, conditions, input ranges, error measures, and
known failure modes.
