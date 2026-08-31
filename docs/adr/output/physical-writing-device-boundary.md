# Physical writing device boundary

## Status

Accepted.

## Decision ID

`atrament.output.physical-writing-device-boundary`

## Context

A plotter or custom machine can put real ink onto a calibrated notebook, but
devices differ in axes, homing, limits, acceleration, pressure, tool changes,
feedback, and interruption behavior. Sending renderer coordinates directly to a
machine would make physical safety depend on assumptions hidden in page code.

## Decision

The core emits a device-neutral writing plan in physical units with ordered
strokes, contact state, speed, optional pressure, pen identity, safe regions,
and provenance. A device adapter declares capabilities and converts an accepted
plan into motion only after coordinate calibration, limit validation, and an
inspectable dry run.

The initial live capability profile uses one pen on one flat, blank sheet. It
does not promise automatic colors, highlighters, pasted photos, sticky notes, or
tool changes. Titles use the same pen and line-art images are reduced to
transparent single-color paths with configurable levels.

Every adapter supports explicit connect, identify, home, arm, start, pause,
resume, cancel, and safe-stop behavior appropriate to its capabilities. Missing
feedback, uncertain position, or a violated boundary fails closed and requires
operator recovery.

Compatibility is admitted per model and transport, never by the generic claim
that a device accepts pens. Initial adapter research targets SVG-driven
NextDraw or AxiDraw CLI operation and documented HP-GL or GP-GL plotters; a
vendor or protocol family is not marked supported until physical acceptance.

## Consequences

- New machines can be added without changing handwriting semantics.
- Simulation and preview can inspect the exact intended physical plan.
- Pressure or tool-change features remain optional declared capabilities.
- Real hardware acceptance cannot be replaced by software-only tests.
- File export, managed CLI, protocol, and direct-device adapters are separate
  compatibility tiers.

## Rejected Alternatives

- Emitting one device's command language from the renderer was rejected because
  it couples page meaning to hardware.
- Assuming every machine can resume from an arbitrary stroke was rejected
  because position and ink state may be uncertain.
- Automatic motion immediately after connection was rejected because calibration
  and operator arming are mandatory safety boundaries.
- Claiming every consumer cutter or pen machine was rejected because many
  vendor workflows expose no stable, documented control interface.

## Verification

Protocol simulators must cover capabilities, limits, disconnects, cancellation,
and uncertain state. Each supported machine needs physical evidence for homing,
page registration, dry run, boundary refusal, interruption, and safe recovery.
The compatibility record names the exact model, firmware, transport, paper,
pen, usable area, and accepted adapter tier.
