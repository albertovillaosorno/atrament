# Atrament task records

`TODO.md` is the public index of unfinished work. Each open task has **one**
canonical `.mdc` file in `docs/todo/open/<lane>/`; closed task records move to
`docs/todo/completed/`. Do not copy a task's evidence back into the index.

Each entry in `TODO.md` contains a `### TODO - ...` heading, a single synthesis
paragraph, and one direct link to its typed record. The first-release horizons
are P1–P8; P9 is optional hardware research.

## Typed frontmatter

Each open record defines the following fields:

- `record_type`: `task`.
- `id`: permanent `ATR-Pn-NNN` identity; do not reuse after closing a task.
- `title`: human-readable task title matching the index.
- `status`: `open` while unfinished; move closed work to `completed/`.
- `priority`: the governing P1–P9 horizon.
- `lane`: workstream within that horizon.
- `dependencies`: typed references to prerequisite task identities.
- `dependencies_specified`: `false` means prerequisites were **not yet**
  established by the previous monolithic TODO. An empty `dependencies` array
  does **not** assert that no prerequisites exist. Set to `true` only after
  reviewing and recording the dependencies.
- `acceptance_criteria`: the original task objective, kept as a typed first
  requirement; expand it with verifiable conditions before closing the task.
- `evidence_status`: `not_independently_verified` for migrated historical
  statements; update only after checking the cited code, fixtures, visual, or
  physical evidence.
- `migrated_from`: provenance of the original task body.

The `Scope and existing evidence` section preserves the task's original
requirements, progress evidence, and limitations. It does **not** mean those
statements were reverified during the migration.

## Closing a task

Verify the task's complete acceptance criteria and dependencies. Preserve its
identity and evidence, mark it `completed`, move the file to `completed/`, and
remove its entry from `TODO.md`. Existing links in technical documentation
should point to the moved record. A task does not close merely because a
contract or a mockup exists.
