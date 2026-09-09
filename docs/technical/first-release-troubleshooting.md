# First-release troubleshooting

## Status

Verified against the current source-tree localhost runtime, browser workspace,
profile validation, and live-safety evidence.

## Purpose

This guide provides operator and developer troubleshooting for failures that are
observable in the current executable boundaries. It identifies the owning layer,
records the fail-closed state that should remain intact, and distinguishes safe
retry from cases that require restart, input correction, or future
implementation work.

This is not a packaged-release installation manual. It must not be used to claim
that contract-only Export, CLI, MCP, PDF serialization, media, or hardware
adapters are already shipped.

## Scope

The guide covers:

- localhost runtime startup and automatic browser launch;
- browser handshake and draft hydration/synchronization;
- localhost HTTP response classes useful during development diagnostics;
- explicit prompt clipboard transport;
- portable-profile validation failures; and
- device-neutral live dry-run and interruption recovery outcomes.

The exact runtime contract is
`docs/technical/localhost-session-runtime-contract.md`. Adapter separation is
documented in `docs/technical/adapter-boundary-validation.md`, profile details
in `docs/technical/portable-profile-container-validation.md`, and physical
safety in `docs/technical/live-physical-safety-validation.md`.

## Contract

### Identify the failing boundary before retrying

A failure message does not authorize weakening the boundary that produced it.
Troubleshooting starts by classifying the failure as startup, authentication,
protocol compatibility, draft transport, clipboard transport, profile
validation, derived-output capability, or physical-safety evidence.

Do not respond to a local authentication failure by disabling Host/Origin
checks, reusing a credential from another adapter, or publishing the session
secret manually. Do not respond to a physical-safety refusal by skipping dry
run or recovery evidence.

### Runtime startup phases

The runtime publishes machine-readable startup states in this order:

```text
starting
listening
ready
```

`starting` means no endpoint is promised. `listening` means Atrament owns its
loopback socket but has not completed automatic browser launch. `ready` means
the
runtime can serve the current browser workspace and application handshake.

If loopback binding fails, startup reports an `Atrament loopback startup failed`
error and no usable session should be assumed.

If automatic browser launch fails, startup fails closed with text beginning:

```text
Atrament browser launch failed:
```

The recovery instruction is to fix automatic browser launch and restart
Atrament. The session credential is intentionally not published for manual
recovery. A bare loopback origin is not an authenticated continuation URL.

The launcher currently has a separate known P1 defect: the credential-bearing
fragment is still exposed through child process arguments. Troubleshooting must
not work around that defect by moving the same secret to environment variables,
temporary files, or shell history.

### Browser startup and handshake states

The browser keeps editing disabled until authenticated handshake and complete
three-field draft hydration succeed.

Current session-status text has these meanings:

- `Frontend ready · credential unavailable`: no admitted in-memory launch
  credential. Restart through the admitted automatic launch path.
- `Handshake unavailable · editing off`: the handshake request could not
  complete. Check the running local process; restart the session if needed.
- `Incompatible <dimension> · expected <value>`: backend/frontend versions do
  not match. Use matching artifacts and do not downgrade the check.
- `Authorization failed`: session credential or Origin admission failed. Treat
  the session as unusable and restart rather than inventing a credential.
- `Invalid backend handshake · editing disabled`: the response did not match
  the admitted handshake envelope. Diagnose backend/frontend drift.
- `Loading session draft…`: authenticated draft reads are in progress. Do not
  edit until the current load completes.
- `Draft load unavailable · editing off`: a draft read could not complete.
  Confirm the local process is reachable, then re-enter through an admitted
  session.
- `Draft load rejected · editing off`: a draft read returned a non-200
  response. Inspect runtime compatibility; empty browser fields are not
  authority.
- `Session ready`: handshake and draft state are synchronized and editing is
  admitted.
- `Draft offline · retry edit`: a replacement request could not complete. Keep
  the visible browser value and retry only after connectivity returns.
- `Draft too large · reduce`: the backend resource limit rejected the whole
  replacement. Reduce the field; the previous backend value remains intact.
- `Invalid draft diagnostic`: a `413` response did not carry the expected typed
  diagnostic. Treat this as frontend/backend drift.
- `Draft sync rejected · retry edit`: draft mutation returned another rejected
  status. Inspect the runtime result and do not assume backend persistence.

Page exit aborts current handshake/draft requests and invalidates stale
completions. A late response or diagnostic-body completion from an abandoned
page must not repopulate session text or status. Concurrent field
synchronization keeps any field failure visible until that field succeeds;
another field's success cannot advertise `Session ready` while failed or active
work remains.

### Localhost HTTP status classes

The current runtime uses these response classes at the inbound HTTP boundary:

- `200 OK`: public resource or health success, a compatible handshake, or an
  authenticated draft read.
- `204 No Content`: authenticated whole-field draft replacement applied.
- `400 Bad Request`: invalid method, target, framing, body, UTF-8, or another
  malformed request.
- `401 Unauthorized`: credential and/or required Origin admission failed.
- `404 Not Found`: unknown public GET target.
- `408 Request Timeout`: the complete request did not arrive within the total
  configured read deadline.
- `409 Conflict`: authenticated handshake version incompatibility.
- `413 Content Too Large`: draft field exceeds the backend-owned resource
  limit.
- `421 Misdirected Request`: request Host is not the exact startup host.
- `500 Internal Server Error`: backend could not construct an admitted typed
  diagnostic response.

A `401` does not disclose whether private draft state exists. `400`, `401`,
`408`, `409`, `413`, and `421` are not invitations to weaken request grammar,
framing, Host, Origin, credential, version, or size checks.

The health endpoint is public only for the exact canonical Host. A successful
health response does not grant access to session-private routes.

### Draft replacement is whole-field and non-truncating

Task, Source, and Candidate are independent pre-acceptance fields. The browser
sends complete UTF-8 replacement values rather than patches.

Malformed framing, invalid UTF-8, admission failure, and resource-limit failure
must leave the backend field unchanged. A `413` therefore means “reduce and
retry,” not “the backend saved a prefix.”

The browser may coalesce obsolete intermediate edits, but a successful `204`
always represents one complete field value.

### Clipboard prompt transport

Prompt Copy is an explicit external data-egress action. Current copy-status text
has these meanings:

- `Waiting for a prompt from the backend.` means there is no prompt to copy;
- `Copying prompt…` means one write is active or an equivalent duplicate Copy is
  already queued;
- `Prompt copied.` means the system clipboard accepted the presented text;
- `Clipboard access is unavailable.` means the browser exposes no usable write
  capability; and
- `Clipboard write failed.` means the attempted write rejected, threw, or did
  not provide the required thenable behavior.

A clipboard failure does not mutate accepted notebook state or create a fake
model receipt. Retrying Copy does not create a new prompt identity when the
backend-presented prompt has not changed.

After a successful Copy, the operating system owns clipboard lifetime. Atrament
cannot truthfully promise that session shutdown erases an already exported
clipboard value.

### Portable-profile validation failures

Current `.atrament` profile validation is pre-serialization and fail closed.
Useful typed failure classes include:

- unsupported container version;
- unsupported required feature;
- duplicate manifest entry path;
- invalid or traversal entry path;
- empty media type;
- missing root manifest;
- missing declared entry;
- undeclared observed entry;
- duplicate observed archive entry;
- byte-length mismatch;
- digest mismatch;
- missing required ZIP64 evidence; and
- unexpected ZIP64 evidence for an ordinary archive.

These failures do not authorize a best-effort import. Unknown optional metadata
may remain opaque, but unknown required features and integrity failures reject.

There is no admitted prior-version migration yet. A version failure should not
be “fixed” by editing a manifest version string or rewriting the current Rust
struct representation as JSON.

### Live capability, dry run, and recovery refusals

The Live capability matrix uses four explicit dispositions:

- `Accept` preserves the capability;
- `Convert` requires an explicit accepted conversion before proceeding;
- `Reject` blocks that requested Live result; and
- `Future` is reserved and never acts as a fallback.

A dry run rejects when operation/evidence counts differ, when a motion operation
claims limit evidence is not applicable, when limit state is `Unknown`, or when
a boundary is `Violated`.

After an interruption, physical resume is admitted only when feedback is
available, position is known, boundaries remain valid, and no partial stroke is
unresolved. Otherwise the result is `OperatorRecoveryRequired`.

These are data-level safety outcomes only. The current source tree does not ship
a physical adapter that can arm, start, pause, resume, cancel, or safe-stop real
hardware. Do not reinterpret `ResumeKnownState` as a command to move a device.

### Unavailable designed capabilities are not troubleshooting failures

Several frozen contracts describe capabilities whose adapters are not yet
implemented. Their absence is expected current product work, not an operator
configuration failure.

Do not invent recovery procedures for:

- persistent Export file adapters and final output serialization;
- executable MCP transport/admission;
- general CLI application projection;
- model-response parsing and candidate construction;
- media decoding/transcription engines;
- PDF/image encoders; or
- physical device adapters and arming.

When one of these capabilities becomes executable, its adapter must add concrete
troubleshooting evidence for its actual error/result surface.

### Persistence expectations during troubleshooting

The active notebook/session is intentionally memory-only unless an explicit
persistent capability succeeds. Browser refresh, process restart, crash, or
shutdown must not be treated as a hidden recovery store.

Do not search browser local storage, service-worker caches, autosave journals,
or an export directory expecting the application to have persisted ordinary
session edits there. Those are not admitted notebook-recovery mechanisms.

Explicitly produced output files, once Export exists and succeeds, will follow
their own persistent-output contract rather than session-memory lifetime.

## Failure Modes

Troubleshooting fails if it recommends bypassing Host, Origin, credential,
version, framing, resource, profile-integrity, capability, limit, or recovery
checks merely to make a workflow continue.

It also fails if it claims an absent adapter can be repaired through hidden
flags, if it treats a health response as authentication, or if it describes
clipboard/session cleanup as deletion of external clipboard contents.

Data-loss guidance fails if it promises crash/restart recovery from a hidden
store that Atrament intentionally does not maintain.

Physical guidance fails if a dry-run pass, plan identity, compatibility record,
or known-state resume disposition is described as permission to move hardware.

## Verification

Current troubleshooting evidence is grounded in:

- `src/backend/runtime-bootstrap/composition/main.rs` for startup phases and
  secret-free browser-launch recovery text;
- `src/backend/session-runtime/adapter-inbound/lib.rs` for exact HTTP routing,
  authentication, Host/Origin admission, deadlines, and status classes;
- `src/browser/workspace/adapter-inbound/main.ts` for visible handshake, draft,
  and clipboard status text;
- `tests/backend/session-runtime/adapter-inbound/lib.rs` for malformed,
  unauthorized, incompatible, oversized, timed-out, and successful runtime
  cases;
- `tests/browser/workspace/adapter-inbound/` for browser session, persistence,
  clipboard, accessibility, hostile-frame, and no-referrer behavior;
- `tests/backend/handwriting-profile-container/domain/lib.rs` for current
  profile
  validation failures;
- `tests/backend/motion-plan-dry-run/domain/lib.rs` for fail-closed motion-limit
  evidence; and
- `tests/backend/physical-recovery-state/domain/lib.rs` for interruption
  recovery
  refusal.

When troubleshooting behavior changes, update this guide only after the owning
runtime, browser, profile, or safety contract and executable regression evidence
change together. Do not document speculative error strings or recovery steps for
contract-only adapters.
