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
first identity that appears in both roles. An exhaustive 5,461-case compact
oracle covers every declaration sequence through six samples over two identities
and both roles, including repeated same-role declarations and both possible
conflict owners.

Repeated declarations with the same role are not rejected here because the
accepted ADR freezes only cross-role separation. Deduplication, sample
ownership,
and workflow persistence belong to the future calibration application layer.

Held-out samples are therefore structurally prevented from also serving as
training evidence in one validated role set.

### Held-out quality evidence stays attributable

`HeldOutQualityReport` retains caller-ordered measurements plus caller-owned
known failure modes. Each `HeldOutQualityMeasurement` names exactly one of the
six required first-release dimensions: geometry, rhythm, joins, spacing,
punctuation, or perceptual fidelity. Measured values and supporting evidence
remain generic so this boundary does not invent units, thresholds, or a scoring
model.

`validate_held_out_quality_report` first applies the existing training/held-out
role-separation rule. It then requires every report measurement to reference a
declared held-out sample, preserving report order for the first invalid sample,
and finally requires all six dimensions. Repeated measurements remain valid and
known failures may be empty because aggregation, failure discovery, sample
sufficiency, report generation, and pass/fail policy remain outside this pure
evidence boundary.

A compact oracle exhausts all 64 required-dimension subsets and all 729 possible
six-measurement reference sequences over held-out, training, and unknown sample
classes. The latter independently derives the first invalid sample in report
order, so a later failure cannot hide an earlier training or unknown reference.

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

`CalibrationSession::validate` rejects duplicate prompt identities.
`CalibrationSession::completed_prompts` returns borrowed completed prompts in
original guidance order for inspection without choosing replacement policy.
`CalibrationSession::next_pending_prompt_index` returns the first pending
prompt in that same order, while `CalibrationSession::is_complete` reports
whether all supplied prompts are complete.

`CalibrationSession::replace_completed_sample` is an exact-preconditioned
in-memory edit for one completed prompt. The caller supplies the prompt
identity,
the exact sample identity it previously inspected, and a replacement sample
identity. Unknown or pending prompts, duplicate prompt identities, and stale
sample expectations reject without changing any prompt.

An equal replacement is a no-op, while an applied replacement changes only that
prompt's sample link. An exhaustive 3,076-case compact oracle crosses every
progress mask through six prompts with present/missing targets, current/stale
expectations, and equal/different replacements. This does not classify a sample
as weak or define a replacement workflow.

The session domain does not decide how many prompts exist, which category mix is
minimum, what wording is shown to a user, which speeds or sizes are selected, or
how reference marks are represented geometrically.

### Bounded variation remains downstream evidence

`VariationParameter` can later retain a minimum, central tendency, maximum,
caller-owned units, distribution metadata, correlation groups, context rules,
and one explicit variation scale. Each minimum or maximum bound is marked
`Observed` or `Authorized`.

`VariationParameter::validate` only enforces numeric ordering of the
caller-owned bounds and central tendency. It does not infer a distribution, fit
a statistical
model, choose correlation semantics, or turn an authorized creative limit into
an observation.

`VariationReplayKey` retains the document seed plus stable semantic identity
required by later deterministic sampling. Sampling itself remains outside the
calibration and variation domains.

## Failure Modes

The present calibration domains fail structurally when:

- one sample identity is declared as both Training and HeldOut;
- a held-out report measurement names a training or unknown sample;
- a held-out report omits any required quality dimension;
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
  conflict rejection, held-out report attribution/completeness, and explicit
  additional-sample requirements;
- `tests/backend/handwriting-calibration-session/domain/lib.rs`, which pins all
  nine prompt categories, caller-owned plan order/reference geometry, completed
  sample inspection and exact replacement, resume position, completion state,
  duplicate-prompt rejection, and every compact progress mask through eight
  prompts; and
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
confidence policy, weak-sample classification/workflow and persistence,
held-out metric/scoring algorithms, and quality-report generation policy.
