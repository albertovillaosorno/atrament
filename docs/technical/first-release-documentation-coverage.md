# First-release documentation coverage

## Status

Verified for the currently executable first-release surface.

## Purpose

This guide records how the P8 operator/developer documentation requirement is
covered by checked-in technical contracts and verified implementation guides. It
prevents an unavailable future adapter, schema, or packaged installer from
keeping present documentation artificially open or, conversely, from being
described as if it already shipped.

## Scope

The coverage record includes calibration, composition, one-shot model use, PDF,
live safety, hardware support status, profile/container internals, schema
status, adapters, validation, and troubleshooting.

It does not turn design-only capabilities into implemented ones. Documentation
for a future installer, CLI/MCP transport, file encoder, model/media adapter, or
physical device command path becomes required with the task that implements
that surface.

## Contract

The present first-release documentation categories are covered as follows:

- **Calibration:** `handwriting-calibration-evidence-validation.md` documents
  evidence provenance, resumable sessions, underdetermination, and downstream
  variation boundaries.
- **Handwriting variation quality:**
  `handwriting-variation-quality-evidence.md` documents required artifact axes,
  structural evidence completeness, and the deliberately absent detector and
  release policy.
- **Composition:** `pdf-print-composition-validation.md` documents vector,
  render-manifest, preview/final, print/scan, and regression-evidence
  boundaries.
- **One-shot LLM use:** `one-shot-formatting-prompt-validation.md` documents the
  self-contained request, clipboard egress, untrusted response boundary, and
  intentionally absent provider/parser behavior.
- **PDF:** `pdf-print-composition-validation.md` documents the pre-serialization
  PDF composition authority and explicitly marks byte emission unavailable.
- **Live safety:** `live-physical-safety-validation.md` documents capability,
  plan, calibration, dry-run, simulation, recovery, and compatibility evidence.
- **Supported hardware:** the same live-safety guide states that no generic
  device family is supported by type existence alone and that physical
  certification/adapters remain absent.
- **Profile internals:** `portable-profile-container-validation.md` documents
  manifest, path, inventory, entry-evidence, canonicality, rewrite, and
  compatibility behavior.
- **Schemas:** the profile guide and adapter guide explicitly distinguish typed
  Rust authority from final JSON/MCP/CLI wire schemas that are not yet frozen.
- **Adapters:** `adapter-boundary-validation.md` maps implemented browser and
  runtime
  boundaries and contract-only CLI, MCP, file, model, media, and hardware
  adapters without inventing transport entry points.
- **Validation:** each verified guide names its checked-in tests and failure
  conditions; `initial-repository-validation.md` records repository/Jig
  validation posture.
- **Troubleshooting:** `first-release-troubleshooting.md` maps current emitted
  startup/browser/runtime/profile/live-safety outcomes to fail-closed actions
  and
  marks unimplemented capabilities unavailable.

The technical index is the machine-readable inventory for these documents.
Every verified coverage path must remain registered in
`docs/technical/index.yml`
and must resolve to an existing document.

Future implementation work updates this coverage record only when it introduces
an operator/developer surface not already covered. Design-only TODO text alone
does not create a documentation obligation to fabricate commands, schemas, or
recovery steps.

## Failure Modes

Documentation coverage is incomplete if an implemented operator/developer
surface has no truthful contract or troubleshooting path, if a guide claims a
contract-only adapter is available, or if final wire/schema details are inferred
from internal Rust types before their owning authority is frozen.

Coverage also fails if hardware support is generalized from an evidence record,
if model/clipboard data egress is described as remaining local, if Render is
presented as Export, or if a successful Plan/dry run is described as hardware
arming.

A future packaged release must add its own installation, verification,
diagnostics, and uninstall documentation when the packaging task defines and
implements those behaviors. This current coverage record does not substitute for
that future package manual.

## Verification

The technical index must contain and resolve the following verified guides:

1. `portable-profile-container-validation.md`;
2. `pdf-print-composition-validation.md`;
3. `handwriting-calibration-evidence-validation.md`;
4. `handwriting-variation-quality-evidence.md`;
5. `live-physical-safety-validation.md`;
6. `one-shot-formatting-prompt-validation.md`;
7. `adapter-boundary-validation.md`;
8. `first-release-troubleshooting.md`; and
9. this documentation-coverage record.

A repository check resolves every `path:` entry in `docs/technical/index.yml`
and verifies the implementation/test references inside each verified guide.
Jig enforces the technical-document section profile and width/paragraph rules.

When an implementation task adds a new adapter, schema, packaged command,
physical control path, or other operator-facing surface, its change must update
or supersede the relevant guide before that implementation task can close.
