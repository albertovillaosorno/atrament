# Live physical-writing safety validation

## Status

Verified against the current device-neutral live-output safety domains.

## Purpose

This guide maps the accepted physical-writing-device ADR to the executable
first-release safety evidence that exists today. It explains the ordering from
capability classification through motion-plan inspection, calibration-bound dry
run, offline simulation, interruption recovery, and exact compatibility records
without implying that Atrament can currently arm or control physical hardware.

The authoritative product decision remains
`docs/adr/output/physical-writing-device-boundary.md`. This guide records
current
implementation evidence rather than adding vendor commands, device support, or
operator recovery procedures.

## Scope

The current executable live-safety boundary spans these backend domains:

- `src/backend/output-capability-matrix/domain/lib.rs` for exhaustive Digital
  and
  Live `Accept` / `Convert` / `Reject` / `Future` dispositions;
- `src/backend/device-neutral-motion-plan/domain/lib.rs` for ordered pen-up and
  pen-down motion intent, pauses, checkpoints, bounds evidence, capability
  assumptions, provenance, and optional estimated duration;
- `src/backend/device-coordinate-calibration/domain/lib.rs` for measured device
  setup evidence without coordinate-transform execution;
- `src/backend/motion-plan-dry-run/domain/lib.rs` for exact plan/calibration
  binding and fail-closed per-operation limit evidence;
- `src/backend/motion-plan-simulation-trace/domain/lib.rs` for read-only offline
  inspection of a validated dry run;
- `src/backend/physical-recovery-state/domain/lib.rs` for fail-closed
  interruption
  resume admission; and
- `src/backend/physical-compatibility-ledger/domain/lib.rs` for exact tested
  model/firmware/transport/setup evidence and known limitations.

These domains do not connect to hardware, discover devices, issue commands,
perform coordinate transforms, compute physical limits, home a carriage, arm a
machine, start motion, pause hardware, resume hardware, cancel hardware, or
execute a safe stop.

## Contract

### Capability classification precedes live compilation

The first-release output capability matrix contains every frozen Digital and
Live row across semantic, handwriting/decoration, color, image-treatment,
page/paper, and hardware-action families.

Each lookup produces exactly one `CapabilityDisposition`:

- `Accept` means the requested capability can be preserved in that mode;
- `Convert` means an explicit recorded conversion is required before acceptance;
- `Reject` means the requested capability blocks that mode; and
- `Future` reserves the capability for a later profile and is never a fallback.

The matrix performs no conversion and never mutates semantic source. In
particular, a `Convert` result is not equivalent to acceptance and a `Future`
result cannot disappear into best-effort output.

### Device-neutral plans are motion intent, not authorization

`DeviceNeutralMotionPlan` retains the exact accepted revision identity and a
backend-owned plan identity. It also retains ordered operations, physical bounds
and writable-region evidence, pen/capability assumptions, and optional estimated
duration.

`MotionPlanOperation` distinguishes checkpoints, pauses, and physical segments.
Every `MotionSegment` retains caller-owned physical geometry, speed,
acceleration, semantic origin, optional pressure, and explicit `PenUp` or
`PenDown` contact state.

The plan is inspectable intent only. Its existence does not authorize hardware
motion and does not prove that its geometry fits a real device.

### Coordinate calibration is evidence, not transform math

`DeviceCoordinateCalibration` binds one exact device identity and calibration
identity to grouped measurement evidence:

- coordinate origin, axis orientation, scale, and skew;
- pen-up height, contact height, speed, and acceleration; and
- usable area, page clamping, and boundary clearance.

The domain preserves those caller-owned measurements and provenance without
choosing units, applying a transform, validating tolerances, or deciding that a
machine is safe to arm.

### Dry run requires complete limit evidence

`MotionPlanDryRun` binds one exact device-neutral plan to one coordinate
calibration record and one ordered limit evaluation for every plan operation.

`validate_motion_plan_dry_run` rejects when the number of evaluations does not
match the plan operation count. Pen-up and pen-down motion cannot claim that
limit checks are not applicable. `Unknown` and `Violated` limit states reject at
the exact operation index.

Non-motion pauses and checkpoints may carry `NotApplicable` limit evidence. A
passing dry run means only that the supplied complete limit evidence is
structurally admissible; this domain does not compute transforms or boundaries
itself.

### Offline simulation cannot bypass the dry run

`build_motion_plan_simulation_trace` first invokes the dry-run validator. An
invalid dry run therefore produces no simulation trace.

A successful `MotionPlanSimulationTrace` borrows the exact validated dry-run
package. Its ordered steps expose the original operation, operation kind, exact
aligned limit evidence, plan and revision identities, calibration identity,
physical bounds, and admitted estimated duration without copying or rewriting
the motion plan.

The trace is not a graphical simulator and does not invent timing, positions,
or derived geometry.

### Interrupted physical state fails closed

`PhysicalRecoverySnapshot` records interruption provenance, feedback
availability, position knowledge, boundary state, and whether a partial stroke
remains unresolved.

The recognized interruption reasons are disconnect, emergency stop, power loss,
process crash, process restart, and user pause. Restart is retained as recovery
provenance only; this domain does not persist or restore hardware state across a
process lifetime. `physical_resume_disposition` returns `ResumeKnownState` only
when:

- required feedback is available;
- the physical position is known;
- boundaries remain within the admitted state; and
- no partial stroke remains.

Any missing feedback, unknown position, violated boundary, or partial stroke
returns `OperatorRecoveryRequired`. The domain does not issue a resume, home, or
safe-stop command and does not choose the operator's recovery steps.

### Compatibility is exact and evidence scoped

`PhysicalCompatibilityRecord` cannot establish generic family support. One
record is scoped to an exact:

- model;
- firmware;
- transport;
- adapter tier;
- paper/media identity;
- pen identity;
- usable area;
- tested settings;
- acceptance-evidence set;
- known-limitations set; and
- last acceptance date.

A NextDraw, AxiDraw, HP-GL, GP-GL, cutter, or plotter family therefore does not
become supported because it resembles a tested device or because it can hold a
pen. Physical acceptance and recertification policy remain outside the current
ledger domain.

## Failure Modes

The present safety domains fail closed before hardware adaptation when:

- an output capability is `Reject` or still requires explicit `Convert` work;
- a dry-run evaluation count differs from the plan operation count;
- a motion operation lacks applicable limit evidence;
- any limit state is `Unknown`;
- any physical boundary is reported `Violated`;
- required recovery feedback is missing;
- the current physical position is unknown; or
- an interruption leaves a partial stroke unresolved.

Important failures that are not executable yet include device discovery and
identity mismatch, failed connection, homing error, actual coordinate-transform
failure, measured boundary violation, lost transport during commands, vendor CLI
or protocol errors, physical safe-stop failure, pen/media mismatch, sheet
movement, and failed operator arming.

The current safety types must not turn those future failures into assumed
success. A clean dry run does not authorize motion, a compatibility record does
not authorize a different firmware/setup, and `ResumeKnownState` does not issue
or guarantee a successful hardware resume.

## Verification

Current checked-in evidence includes:

- `tests/backend/output-capability-matrix/domain/lib.rs`, which exhaustively
  pins
  all 118 frozen Digital/Live matrix rows;
- `tests/backend/device-neutral-motion-plan/domain/lib.rs`, which pins motion
  operation content, provenance, bounds, capability assumptions, and duration;
- `tests/backend/device-coordinate-calibration/domain/lib.rs`, which pins the
  grouped transform, pen-motion, sheet-registration, identity, and evidence
  fields;
- `tests/backend/motion-plan-dry-run/domain/lib.rs`, which pins exact
  plan/calibration binding and fail-closed limit evidence;
- `tests/backend/motion-plan-simulation-trace/domain/lib.rs`, which proves an
  invalid dry run produces no trace and a valid trace preserves exact order and
  evidence;
- `tests/backend/physical-recovery-state/domain/lib.rs`, which pins known-state
  resume and refusal for each unresolved uncertainty; and
- `tests/backend/physical-compatibility-ledger/domain/lib.rs`, which pins exact
  device/setup/evidence/limitation/date scoping.

Before extending physical output, preserve these boundaries:

1. capability `Convert`, `Reject`, and `Future` states must never disappear into
   implicit acceptance;
2. keep the device-neutral plan independent from vendor command languages;
3. require measured calibration and complete limit evidence before any future
   arming boundary;
4. do not create simulation evidence from an invalid dry run;
5. fail closed whenever feedback, position, boundaries, or partial-stroke state
   is uncertain; and
6. scope every compatibility claim to the exact physically tested combination.

Still-open work includes coordinate-transform and limit geometry, visual dry-run
projection, path optimization with drying/travel authority, device discovery,
managed CLI and protocol adapters, connect/identify/home/arm/start/pause/resume/
cancel/safe-stop execution, operator recovery procedures, compatibility
admission/recertification policy, and physical certification on actual hardware.
