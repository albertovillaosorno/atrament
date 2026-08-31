# AI formatting through CLI and MCP

## Status

Accepted.

## Decision ID

`atrament.input.ai-formatting-cli-and-mcp`

## Context

An agent should be able to turn raw material into organized notes, preserve
formulas, choose headings, construct tables, place photos, and request output.
If agent operations bypass the same contracts used by people, documents will
become difficult to review and impossible to reproduce.

## Decision

One application capability model is projected through both CLI commands and
MCP tools. Operations accept and return versioned schemas for notebooks,
profiles, assets, layout constraints, diagnostics, previews, and exports.

The first browser workflow requires no model account or embedded model API. A
permanent Copy prompt button produces one self-contained request containing the
task, paper and style constraints, supported schema, source rules, and required
return envelope. The user pastes it once into a chat and pastes the response
back into the LLM editor for validation and preview.

A repository-owned system instruction teaches agents to use semantic blocks,
retain source facts, preserve mathematics, mark ambiguity, and validate before
export. The instruction guides tool use but never becomes a hidden source of
document truth or permission to mutate unsupported fields.

The prompt requires citations for externally sourced claims and distinguishes
provided, derived, cited, and unverified content. Atrament validates citation
structure and provenance but does not claim that a formatted citation proves
the source or statement correct.

## Consequences

- Human scripts and AI agents exercise the same product boundaries.
- Commands remain composable and inspectable without a graphical session.
- The ordinary chat workflow needs one outbound copy and one inbound paste.
- A malformed model response cannot mutate the accepted notebook.
- Schema and prompt versions require compatibility evidence.
- Agent convenience cannot bypass consent, provenance, or physical safety.

## Rejected Alternatives

- A private UI-only automation channel was rejected because behavior would be
  difficult to inspect and reuse.
- Letting an agent write internal files directly was rejected because it would
  bypass validation and migration.
- Freeform prompt output as the document format was rejected because prose is
  not a stable notebook contract.

## Verification

Contract tests must execute equivalent operations through the direct
application boundary, CLI, and MCP and compare accepted results. Adversarial
fixtures must prove that unsupported fields, fabricated formulas, and unsafe
machine commands are refused or surfaced for review. Clipboard tests must prove
the prompt is complete without prior chat context.
