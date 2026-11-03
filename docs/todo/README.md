# Atrament task records

`TODO.md` is the public index of unfinished work. Each open task has **one**
canonical `.mdc` file in `docs/todo/open/<lane>/`; closed task records move to
`docs/todo/completed/`. Do not copy a task's evidence back into the index.

Each entry in `TODO.md` contains a `### TODO - ...` heading, a single synthesis
paragraph, and one direct link whose visible label is the exact typed record
path, for example:
`[docs/todo/open/runtime/task.mdc](docs/todo/open/runtime/task.mdc)`.
Current priorities are P1–P4; historical task IDs remain unchanged.

## Typed frontmatter

Each open record defines the following fields:

- `record_type`: `task`.
- `id`: permanent `ATR-Pn-NNN` identity, independent of current priority.
- `title`: human-readable task title matching the index.
- `status`: `open` while unfinished; move closed work to `completed/`.
- `priority`: current P1–P4 rank, where P1 is highest.
- `lane`: workstream within that horizon.
- `dependencies`: typed references to prerequisite task identities.
- `dependencies_specified`: `false` means prerequisites remain unknown;
  an empty `dependencies` array does **not** mean no prerequisites exist.
  `true` means the listed prerequisite edges have been reviewed against the
  task's acceptance behavior.
- `dependency_rationale`: when `dependencies_specified` is `true`, records
  why the listed capabilities must be ready before accepting this task.
- `acceptance_criteria`: the original task objective is retained as the first
  scope statement. The second, `Finite v1 completion`, is the governing
  deliverable and stop condition. It limits the first statement when the
  historical wording is unbounded.
- `evidence_status`: `not_independently_verified` for migrated historical
  statements; update only after checking the cited code, fixtures, visual, or
  physical evidence.
- `migrated_from`: provenance of the original task body.

The frontend regression suite checks that reviewed prerequisites reference
existing tasks, do not duplicate edges, and form a cycle-free graph. It rejects
unreviewed edges rather than silently treating them as authoritative.

Task identity is permanent, not its priority or position in `TODO.md`.
Closing a task never renumbers remaining records.
Reviewed dependencies may reference a completed task by its same identity.
The integrity suite checks both open and completed records so those edges
remain resolvable without treating theoretical completion as a live feature.

The `Scope and existing evidence` section preserves the task's original
requirements, progress evidence, and limitations. It does **not** mean those
statements were reverified during the migration.

## Finite acceptance and honest evidence

The first release accepts a finite deliverable, not an endless claim of
completeness across all future formats, browsers, machines, or hardware.
Each open task has a bounded executable criterion; historical design tasks
retain their narrower theoretical completion. One representative accepted
and rejected example is enough unless the criterion explicitly demands more.

Future variants are separate work, not an
implicit reason to reopen a bounded task. P9 remains optional.

Use one of two explicit acceptance bases:

- **Practical:** the named artifact, code, or integration for the finite scope
  exists. Tests, benchmarks, or independent reproduction are useful evidence,
  but are not mandatory to accept the bounded deliverable. Do not label
  unperformed checks as passing.
- **Theoretical:** when practical execution, measurements, devices, external
  integrations, or proof cannot reasonably be supplied, a complete written
  contract is sufficient for acceptance of that finite design deliverable.
  It identifies inputs and outputs, relevant invariants, one worked example,
  one refusal/failure path, assumptions, and unresolved risks. No physical
  experiment, cross-browser claim, certification, or independent verification
  is required to accept the design as theoretical.

Practical and theoretical acceptance are **not interchangeable claims**.
**A complete theoretical deliverable is sufficient for academic task closure.**

No practical implementation or independent test is additionally required to
close that design artifact. A theoretical task can be complete as a design
while the corresponding feature remains explicitly unavailable or
unverified.

In particular, theoretical security, privacy, cleanup, or
physical-device acceptance never establishes
that secret bytes were erased, an installer works, or a machine is safe to
operate. Do not issue physical commands or claim a production release is safe
solely because those documents were accepted.

Dependencies record intended implementation sequencing. An unfinished
prerequisite does not prevent **theoretical acceptance** of a separate bounded
contract. Its dependency and resulting integration limitation must stay
visible; it cannot silently be treated as implemented. Unknown prerequisites
may remain explicitly unknown when no concrete integration is admitted.

## Closing a task

Check that the specific finite deliverable is present in the record. The
owner may accept a historical design deliverable theoretically without an
independent test run. New **executable** integration tasks require the named
working capability; documentation alone does not close them. Record the basis,
produced artifact, unresolved assumptions and risks under `Acceptance
decision`; retain honest `evidence_status` rather than inventing verification.

Preserve the task identity and historical evidence, set `status: completed`,
move the record to `completed/`, and remove it from `TODO.md`. Update internal
links.
Do not automatically close a task merely because this policy exists: its
particular bounded deliverable must actually be written or supplied.

## Current executable product decision — November 2, 2026

The owner requires the existing Rust codebase, a local Rust CLI, and a web
editor served by the existing loopback Rust runtime. Native notebook and
handwriting profile editing, statistical per-glyph controls, real previews,
and printable PDF/transparent PNG are required. The accidental Python
notebook application has been removed rather than retained as a prototype.

Hardware, external model accounts, MCP, and native desktop installers remain
optional advanced work. Their accepted theoretical designs stay archived,
while unfinished implementation appears in separate future TODO records.

The open work is one list, ordered from P1 to P4. All 77 accepted original
designs remain represented by linked implementation tasks. Completed theory
is not erased or misrepresented as working software.

A theoretical deliverable is sufficient to close its academic design task.
An executable follow-up closes only when its promised working functionality
exists; no second theoretical acceptance can silently finish the software.
Consult [the Rust-first roadmap](../technical/rust-cli-localhost-priority.md)
for the exact priority order and glyph coverage targets.

## Completed-design integrity review — 2026-11-03

The archive audit at
`docs/technical/completed-todo-acceptance-audit-2026-11-03.md` found
substantive finite theoretical work in all 77 original completed tasks.
They remain completed.

All 77 also have an explicitly indexed future
implementation path. This preserves the complex work while prioritizing
the local Rust CLI and notebook/PDF UI.

Reopen a historical design only if
its theoretical deliverable itself is materially incomplete.

## Future implementation tracking

Every implementation follow-up sourced from an accepted historical design
has `migrated_from: accepted-theoretical-designs`, a bounded practical
criterion, a P1–P4 `priority` and a stable `source_designs` list.
The TODO integrity suite requires every historical design to appear in
exactly one such follow-up, whether that follow-up is open or completed.
The most urgent local notebook work is P1. Optional hardware is P4.
