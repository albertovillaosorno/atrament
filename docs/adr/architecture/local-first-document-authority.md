# Local-first document authority

## Status

Accepted.

## Decision ID

`atrament.architecture.local-first-document-authority`

## Context

Handwriting samples, assignments, recordings, photos, and derived profiles are
personal and may be sensitive. Composition and physical writing also need to
work when external services are unavailable or unsuitable.

## Decision

Atrament starts a loopback-only, ephemeral local session. It has no account,
cloud sync, database, background upload, or automatic document persistence.
Closing the session discards its notebook, assets, undo history, previews, and
derived render state.

Users may explicitly import or export a notebook bundle, `.atrament` profile,
PDF, image, or machine plan. Those deliberate files are the only persistent
product state and remain under the path selected by the user.

Temporary conversions use owned temporary storage with bounded lifetime and
explicit cleanup. A failed external operation leaves the current in-memory
session readable and does not silently persist or replace it.

## Consequences

- Core editing, layout, profile use, and export remain available offline.
- Refresh, close, or crash loses unsaved session work by design.
- The interface must make that ephemeral behavior impossible to miss.
- Cloud transcription or model adapters require explicit configuration.
- Profiles and notebook bundles persist only through explicit import or export.

## Rejected Alternatives

- Cloud-only document storage was rejected because availability and privacy
  would depend on an external account.
- Automatic local autosave was rejected because the requested notebook is a
  disposable localhost workspace rather than a personal document database.
- Treating generated caches as authority was rejected because derived state can
  be recreated and may become stale.
- Silent uploads for convenience were rejected because personal samples and
  recordings require deliberate disclosure.

## Verification

End-to-end fixtures must compose and render with network access disabled.
Lifecycle tests must prove that closing a session removes ephemeral state while
explicit exports remain readable and no undeclared local files are created.
