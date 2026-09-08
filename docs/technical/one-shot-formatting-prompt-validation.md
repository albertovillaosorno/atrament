# One-shot formatting prompt validation

## Status

Verified against the current initial-formatting prompt and clipboard contracts.

## Purpose

This guide documents Atrament's first-release one-shot external-model boundary:
one self-contained initial formatting request is prepared from backend-owned
inputs, presented for explicit Copy, sent by the user to an external model, and
the complete response is pasted back as untrusted session text.

It connects the executable prompt structure to the frozen first-complete journey
and clipboard transport contract without creating a provider integration,
background chat session, response parser, or hidden model state.

## Scope

The current executable initial-prompt structure lives in
`src/backend/one-shot-formatting-prompt/domain/lib.rs`.

The surrounding frozen workflow is documented by:

- `docs/technical/first-complete-user-journey.md`, which requires one complete
  initial request and one complete response round trip; and
- `docs/technical/clipboard-command-transport-contract.md`, which defines
  explicit user Copy/Paste as text transport and intentional data egress.

The browser currently exposes Task, Source, a backend-presented Prompt surface,
Copy prompt, and a raw Candidate response surface. Runtime draft storage keeps
Task, Source, and Candidate separate process-memory fields. Neither browser nor
runtime draft storage makes pasted response text an accepted semantic notebook.

This guide covers the initial formatting request only. Later command-mode
refinement has separate command-context, semantic-command, and clipboard/MCP
contracts and must not be conflated with this one-shot candidate-formatting
boundary.

## Contract

### The initial request is self-contained

`OneShotFormattingPrompt` groups four explicit authorities:

- task and complete source material;
- current paper, style, and output constraints;
- complete response-protocol expectations; and
- backend-owned prompt identity plus prompt version.

The prompt value does not depend on an earlier external chat message. A receiver
must be able to interpret the request from the supplied source, constraints, and
protocol information alone.

### Source inputs are complete, not incremental chat patches

`FormattingPromptSourceInputs` contains the complete source material and current
formatting task. The structure does not describe a patch against a previous
model conversation and does not authorize the external model to fetch hidden
Atrament state.

The browser/runtime may coalesce draft synchronization while a user types, but
the one-shot request authority is still a complete backend-owned value, not a
sequence of browser-authored model instructions.

### Constraints travel with the same request

`FormattingPromptConstraintInputs` retains the currently admitted output
targets,
paper constraints, and style constraints. Those values are explicit prompt
inputs rather than assumptions left to prior conversation history.

The domain does not choose a paper, style, or output mode. It preserves the
caller/backend-owned values that the formatting workflow has already selected.

### Response protocol requirements are explicit

`FormattingPromptProtocolInputs` retains:

- the complete backend-owned semantic candidate format;
- source rules distinguishing supplied, derived, cited, and unverified content;
- provenance rules;
- diagnostic and ambiguity expectations; and
- the required outer return envelope.

These are protocol inputs, not a parser implementation. The current prompt
domain does not serialize their wording and does not validate pasted responses.

### Identity and version are data, not authentication

`OneShotFormattingPrompt` retains one backend-owned prompt identity and one
prompt-protocol version. The current domain preserves them but does not compute
or hash either value.

The frozen journey requires an unchanged task/source/format/paper/style/output
request to retain the same prompt identity. That identity is correlation data,
not a bearer credential and not authorization to read or mutate session state.

Browser session secrets and other local admission material are not part of the
one-shot formatting-prompt structure.

### Copy is an intentional egress boundary

Clipboard presentation happens only after an explicit user Copy action. The
browser transports backend-presented text; it does not independently assemble
semantic authority or execute prompt-like content.

Once copied text is placed on the operating-system clipboard and pasted into an
external model, it has intentionally left Atrament process memory. Atrament
cannot truthfully claim that the copied source remains local after the user
sends it to a third-party model.

The clipboard contract also makes lifetime explicit: shutdown or page cleanup
can scrub Atrament-owned copies but cannot revoke text already owned by the
operating system or an external application.

### Pasted model output remains untrusted session text

The external response enters the raw Candidate response surface and backend
draft
storage as text. It is not accepted semantic authority merely because it came
from the requested model or resembles the expected envelope.

The browser does not parse semantic commands, allocate accepted identities,
apply notebook mutations, or execute markup from the pasted text. The
first-complete journey requires future backend parsing/validation to produce
either a candidate notebook or diagnostics before explicit candidate acceptance.

That text-to-candidate parser is not implemented by the current one-shot prompt
domain and must not be inferred from typed candidate acceptance APIs elsewhere
in the repository.

### Initial formatting and refinement remain separate

The one-shot initial request asks for a complete candidate organization of the
prepared source. Later refinement works against an already accepted revision and
uses bounded command context, explicit writable scope, command preconditions,
and transactional semantic-command application.

The two paths may both use manual clipboard transport, but their authority is
different. An initial candidate response must not be treated as a semantic
command batch, and command-mode context must not depend on a hidden initial chat
history.

## Failure Modes

The current one-shot boundary is invalid if:

- prompt construction depends on hidden previous chat context;
- task or source material is silently omitted from the intended complete
  request;
- paper, style, or output constraints are treated as unstated external-model
  memory;
- required semantic format, source/provenance rules, diagnostics expectations,
  or return envelope are omitted from the request authority;
- browser session credentials are included in copied model context;
- a failed clipboard write is reported as successful external delivery;
- pasted response text mutates accepted notebook state before backend
  validation; or
- model-looking markup or command prose executes as browser/application action.

Other important failures are not executable yet because prompt serialization
and response parsing are not implemented. These include malformed return
envelopes, unknown required candidate-format features, invalid semantic identity
relationships, fabricated source facts, provenance violations, and provider-side
context truncation.

The current code must not hide those future failures by treating raw text as a
valid candidate. It also must not claim privacy properties for an external model
provider that Atrament cannot observe or control.

## Verification

Current executable evidence includes:

- `tests/backend/one-shot-formatting-prompt/domain/lib.rs`, which proves every
  frozen self-contained input remains explicit and prompt identity/version stay
  ordinary data without browser side effects;
- browser Copy-prompt tests, which exercise explicit copy availability,
  clipboard failure, duplicate/stale completion handling, prompt-change
  invalidation, and page-exit scrubbing; and
- localhost session-draft/runtime tests, which keep Task, Source, and Candidate
  fields separate, bounded, authenticated, and non-authoritative before semantic
  acceptance.

The frozen journey additionally requires the eventual end-to-end fixture to
prove:

1. one complete request can be interpreted without hidden chat history;
2. one complete model response enters as raw untrusted text;
3. malformed or unsupported response content cannot partially mutate the
   accepted notebook;
4. candidate acceptance remains an explicit transaction; and
5. the accepted notebook preserves source/provenance invariants through later
   editing and output.

Still-open implementation work includes canonical prompt-text serialization,
prompt identity computation, initial response parsing, candidate-envelope
validation, source/provenance checking of model output, candidate review UI,
actual external-provider guidance, and complete end-to-end one-shot journey
execution. No provider API integration is required by the accepted manual
clipboard workflow.
