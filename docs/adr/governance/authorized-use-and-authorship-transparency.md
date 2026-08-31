# Authorized use and authorship transparency

## Status

Accepted.

## Decision ID

`atrament.governance.authorized-use-and-authorship-transparency`

## Context

Personal handwriting synthesis is useful for accessibility, private notebooks,
authorized document production, artistic work, and physical-machine control.
The same fidelity can be misused to misrepresent machine-produced work as a
person's unaided writing or to imitate another writer without consent.

## Decision

Atrament supports profiles created by, or with the explicit authorization of,
the represented writer. Product guidance must distinguish generated output
from claims about who physically wrote it and must not promote evasion of
authorship, academic, legal, or institutional rules.

The core file and rendering contracts retain creation provenance sufficient for
an authorized workflow to inspect how an artifact was produced. Exporters may
project clean pages, but they do not erase the authoritative local provenance
record or falsely attest human authorship.

## Consequences

- Calibration requires an explicit profile owner or authorization claim.
- Documentation emphasizes legitimate personal and assisted-writing uses.
- Generated artifacts can be traced within the user's local project history.
- Atrament does not decide external submission rules; the operator remains
  responsible for obtaining permission and representing authorship honestly.

## Rejected Alternatives

- An unrestricted impersonation feature was rejected because fidelity is not
  consent.
- Mandatory visible watermarks on every page were rejected because many
  authorized artistic, accessibility, and private uses require clean output.
- Removing all provenance was rejected because it would make local review and
  responsible automation materially weaker.

## Verification

Profile creation, import, export, CLI, and MCP tests must preserve the declared
owner and provenance policy. Public documentation must not instruct users to
defeat authenticity checks or conceal prohibited automation.
