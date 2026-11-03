# Atrament task records

`TODO.md` is the public index of unfinished work. Each open task has **one**
canonical `.mdc` file in `docs/todo/open/<lane>/`; closed task records move to
`docs/todo/completed/`. Do not copy a task's evidence back into the index.

Each entry in `TODO.md` contains a `### TODO - ...` heading, a single synthesis
paragraph, and one direct link whose visible label is the exact typed record
path, for example:
`[docs/todo/open/runtime/task.mdc](docs/todo/open/runtime/task.mdc)`.
The first-release horizons are P1–P8; P9 is optional hardware research.

## Typed frontmatter

Each open record defines the following fields:

- `record_type`: `task`.
- `id`: permanent `ATR-Pn-NNN` identity; do not reuse after closing a task.
- `title`: human-readable task title matching the index.
- `status`: `open` while unfinished; move closed work to `completed/`.
- `priority`: the governing P1–P9 horizon.
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

Task identity is permanent, not its position in `TODO.md`. Closing a task
leaves gaps in the numbered open index; it never renumbers later records.
Reviewed dependencies may reference a completed task by its same identity.
The integrity suite checks both open and completed records so those edges
remain resolvable without treating theoretical completion as a live feature.

The `Scope and existing evidence` section preserves the task's original
requirements, progress evidence, and limitations. It does **not** mean those
statements were reverified during the migration.

## Finite acceptance and honest evidence

The first release accepts a finite deliverable, not an endless claim of
completeness across all future formats, browsers, machines, or hardware. Each
open record has a narrower `Finite v1 completion` criterion. One documented
representative example per named capability is sufficient when the criterion
does not explicitly demand more.

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

Practical and theoretical acceptance are **not interchangeable claims**. A
theoretical task can be complete as a design while the corresponding feature
remains explicitly unavailable or unverified. In particular, theoretical
security, privacy, cleanup, or physical-device acceptance never establishes
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
owner may accept it as practical or theoretical without a separate proof or
test run. Record the basis, produced artifact, unresolved assumptions and
risks, and any unavailable integrations under `Acceptance decision`; retain
the honest `evidence_status` rather than inventing verification.

Preserve the task identity and historical evidence, set `status: completed`,
move the record to `completed/`, and remove it from `TODO.md`. Update internal
links.
Do not automatically close a task merely because this policy exists: its
particular bounded deliverable must actually be written or supplied.

## Owner's narrower product decision — November 2, 2026

The current executable product goal is **Linux CLI, personal use, notebook
profiles, statistical per-letter handwriting, and PDF/PNG printing**.
Multiplatform native installers, a browser editor, MCP automation, external
LLM accounts, a physical plotter, and a custom writing robot are **not
completion prerequisites**. Previous task acceptance records remain archived
as academic/design evidence and should not be rewritten to simulate practical
software validation. Hardware tasks are satisfied by finite theoretical
plans; they do not require a device, prototype, firmware or test bench.

Use an actual CLI-exported PDF or PNG when claiming the new digital pathway
works. When a feature is theoretical, label it theoretical; neither a closed
TODO nor a design specification is proof of a running feature. Follow the
short practical scope and limits in the root README.
