# Handwriting calibration evidence validation

## Status

Verified against the current transport-neutral calibration domains.

## Purpose

This guide maps the accepted handwriting-calibration ADR to the executable
first-release evidence and session boundaries that exist today. It explains how
Atrament preserves observed and inferred parameter evidence, keeps held-out
writing separate from training samples, resumes caller-supplied calibration
plans, and represents underdetermined behavior without silently inventing data.

The authoritative product decision remains
`docs/adr/handwriting/calibration-from-written-samples.md`. This guide records
current implementation evidence rather than defining capture geometry, a sample
quota, or a fitting algorithm.

## Scope

The current executable calibration boundary spans two pure backend domains:

- `src/backend/handwriting-calibration-evidence/domain/lib.rs` owns parameter
  evidence provenance, observed/inferred status, accepted correction history,
  training/held-out sample roles, and explicit underdetermination; and
- `src/backend/handwriting-calibration-session/domain/lib.rs` owns the accepted
  prompt-category vocabulary, caller-supplied prompt order, resumable progress,
  prompt-identity validation, and caller-owned reference geometry.

Downstream bounded-variation structure lives in
`src/backend/handwriting-variation/domain/lib.rs`. It can retain observed or
explicitly authorized parameter bounds after calibration, but it does not decide
which calibration samples are sufficient or how parameters are fitted.

The present calibration domains do not read images, decode scans, detect marks,
fit camera transforms, choose confidence scales, define measurement units,
extract handwriting, select minimum sample counts, or compute a quality score.

## Contract

### Every extracted parameter retains evidence

`CalibrationParameterEvidence` preserves:

- caller-owned parameter identity and value;
- caller-owned source region;
- caller-owned measurement unit;
- caller-owned confidence value;
- ordered accepted correction history; and
- an explicit evidentiary basis.

The evidentiary basis is either `Observed` or `InferredExtreme`. Those states
are
not interchangeable. An inferred extreme cannot become observed evidence merely
because a later consumer prefers a complete parameter envelope.

The current domain deliberately leaves the parameter vocabulary, source-region
shape, units, confidence scale, correction payload, and extraction method to
their owning future authorities.

### Underdetermination requests more evidence

`CalibrationDetermination::Underdetermined` returns true from
`requires_additional_sample()`. This is the executable form of the accepted ADR
rule that required behavior with insufficient evidence must ask for another
sample rather than being fabricated invisibly.

The domain does not decide which behavior is required, how much evidence is
enough, or which prompt should be scheduled next. Those remain workflow and
model-policy decisions.

### Training and held-out writing stay separate

Every `CalibrationSample` carries a stable caller-owned identity and either the
`Training` or `HeldOut` role. `validate_calibration_sample_roles` rejects the
first identity that appears in both roles.

Repeated declarations with the same role are not rejected here because the
accepted ADR freezes only cross-role separation. Deduplication, sample
ownership,
and workflow persistence belong to the future calibration application layer.

Held-out samples are therefore structurally prevented from also serving as
training evidence in one validated role set. This domain does not calculate the
final held-out quality report.

### Guided calibration plans are caller supplied and resumable

`CalibrationPromptKind` centralizes the nine accepted prompt categories:

- free writing;
- heading;
- isolated character;
- join;
- mathematical symbol;
- numeral;
- punctuation;
- sentence; and
- word.

Each `CalibrationPrompt` retains its stable identity, category, caller-owned
speed and size, and either `Pending` progress or one completed sample identity.
`CalibrationSession` preserves the caller-supplied prompt order and caller-owned
known reference geometry.

`validate_calibration_session` rejects duplicate prompt identities.
`next_pending_calibration_prompt_index` returns the first pending prompt in the
original guidance order, while `calibration_session_complete` reports whether
all supplied prompts are complete.

The session domain does not decide how many prompts exist, which category mix is
minimum, what wording is shown to a user, which speeds or sizes are selected, or
how reference marks are represented geometrically.

### Bounded variation remains downstream evidence

`VariationParameter` can later retain a minimum, central tendency, maximum,
caller-owned units, distribution metadata, correlation groups, context rules,
and one explicit variation scale. Each minimum or maximum bound is marked
`Observed` or `Authorized`.

`validate_variation_parameter` only enforces numeric ordering of the
caller-owned
bounds and central tendency. It does not infer a distribution, fit a statistical
model, choose correlation semantics, or turn an authorized creative limit into
an observation.

`VariationReplayKey` retains the document seed plus stable semantic identity
required by later deterministic sampling. Sampling itself remains outside the
calibration and variation domains.

## Failure Modes

The present calibration domains fail structurally when:

- one sample identity is declared as both Training and HeldOut;
- one guided plan repeats a stable prompt identity;
- a variation minimum exceeds its maximum;
- a variation central tendency falls below the minimum; or
- a variation central tendency exceeds the maximum.

Other important calibration failures are not executable yet because the owning
capture, geometry, extraction, or quality authorities do not exist. These
include unreadable images, incorrect reference marks, camera distortion that
cannot be corrected, insufficient resolution, unsupported capture metadata,
failed character/stroke extraction, low-confidence measurements, and a held-out
quality score below an accepted threshold.

The current domains must not convert those future failures into guessed values.
In particular, they must not silently label an inference as observed, reuse a
held-out sample for fitting, choose a default confidence scale, fabricate a
minimum sample count, or declare physical calibration successful without the
future measurement authority.

## Verification

Current checked-in regression evidence includes:

- `tests/backend/handwriting-calibration-evidence/domain/lib.rs`, which pins
  complete evidence retention, observed/inferred separation, held-out/training
  conflict rejection, and explicit additional-sample requirements;
- `tests/backend/handwriting-calibration-session/domain/lib.rs`, which pins all
  nine prompt categories, caller-owned plan order/reference geometry, resume
  position, completion state, and duplicate-prompt rejection; and
- `tests/backend/handwriting-variation/domain/lib.rs`, which pins bound
  evidence,
  envelope ordering, metadata preservation, variation scale, and deterministic
  replay inputs without sampling.

Before extending calibration, preserve these boundaries:

1. keep measurements attributable to source region, units, confidence, and
   correction history;
2. never collapse `InferredExtreme` into `Observed`;
3. keep held-out sample identities out of training evidence;
4. leave sample count, prompt wording, speeds, sizes, confidence policy, and
   reference-mark geometry with the workflow or measurement authority that can
   justify them; and
5. add image/scan decoding, geometry correction, extraction, quality scoring,
   and persistence only in adapters or application services that can observe
   their actual failures.

Still-open implementation work includes the minimum viable sample set, capture
UI and persistence, photographic/scanner/digital input adapters, physical
reference-mark geometry and transform fitting, handwriting extraction,
confidence
policy, weak-sample replacement workflow, held-out quality metrics, and the
final calibration quality report.
