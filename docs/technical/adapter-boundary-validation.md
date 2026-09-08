# Adapter boundary validation

## Status

Verified against the current browser, localhost, clipboard, MCP, Render, Export,
and Plan contracts.

## Purpose

This guide maps the first-release adapter boundaries that already have frozen
technical contracts. It explains where transport ends, where application
authority begins, which identities remain adapter-local, and which side effects
must stay behind explicit application capabilities.

The guide does not create a new adapter interface. It records current authority
separation so future browser, CLI, MCP, file, model, and hardware adapters do
not
silently invent parallel mutation or output semantics.

## Scope

The verified adapter map spans these technical contracts:

- `docs/technical/localhost-session-runtime-contract.md` for the authenticated
  loopback browser runtime;
- `docs/technical/clipboard-command-transport-contract.md` for explicit human
  Copy/Paste model transport;
- `docs/technical/mcp-local-session-admission-contract.md` for local MCP
  admission separate from browser credentials;
- `docs/technical/mcp-application-capability-projection.md` for MCP projection
  of shared application capabilities;
- `docs/technical/render-application-contract.md` for read-only derived visual
  output;
- `docs/technical/explicit-export-application-contract.md` for explicit
  persistent file side effects; and
- `docs/technical/device-neutral-plan-application-contract.md` for read-only
  live-plan compilation before physical hardware authority.

Current browser implementation evidence also lives under
`src/browser/workspace/adapter-inbound/` and
`tests/browser/workspace/adapter-inbound/`.

This guide does not define final CLI syntax, MCP transport fields, filesystem
APIs, provider APIs, hardware commands, or release-packaging entry points.

## Contract

### Inbound transport is not application authority

An inbound adapter may authenticate or admit a caller, parse its own transport,
and project admitted application inputs. It does not become semantic notebook,
render, file, or hardware authority merely because it received the request.

The application boundary owns revision checks, writable scope, provenance,
capability admission, retries, diagnostics, and atomic mutation. Equivalent
normalized application requests retain equivalent semantics regardless of which
admitted inbound adapter carried them.

Transport-local state must not leak into accepted semantic identity unless a
separate owning contract explicitly declares it meaningful.

### Browser runtime admission is session private

The localhost browser runtime binds an exact loopback endpoint and uses a fresh
in-memory session secret. Protected reads and mutations require the Bearer
credential and exact browser Origin when Origin is present.

The browser secret is not a notebook identity, command-context identity, retry
identity, render identity, export identity, plan identity, or MCP credential.
The browser frontend therefore cannot reuse it as generic admission material for
another adapter.

Static frontend resources and the minimal health endpoint may be public because
they expose no session-private state or mutation authority.

### Clipboard is explicit human transport

Clipboard Copy moves backend-presented model-request text into the operating
system clipboard only after an explicit user action. Copy does not mutate the
notebook, create an accepted command, Export a file, or authorize a later
response.

Pasted model text remains untrusted transport data. The browser does not parse
semantic commands or commit notebook mutations from pasted text. Backend-owned
validation and application must run before accepted state changes.

Clipboard prompt or context identity is correlation metadata, not
authentication. Operating-system clipboard lifetime also remains outside
Atrament after a successful explicit Copy.

### MCP admission is distinct from browser admission

MCP follows its own local session-admission contract. It does not inherit the
browser secret, copied prompt text, command-context identity, command retry
identity, or export retry identity.

Admission only determines which application effect classes the MCP caller may
reach. It never bypasses capability-specific revision, scope, provenance, path,
overwrite, retry, output-profile, calibration, or physical-safety checks.

Schema and tool discovery are interface descriptions rather than credentials.
Read-only admission therefore remains distinguishable from Apply, Export, or
future physical-device authority.

### Shared application capabilities preserve effect classes

The current frozen adapter-facing application capabilities have distinct effect
classes:

| Capability | Effect class | Persistent or physical side effect |
| --- | --- | --- |
| Inspect / command context | Read-only | None |
| Validate | Read-only candidate simulation | None |
| Apply | Accepted-revision mutation | Notebook revision only |
| Undo / redo | Accepted-history mutation | Notebook revision only |
| Render | Derived computation | None |
| Plan | Derived device-neutral computation | None |
| Export | Explicit persistent output | Selected file target |
| Physical device actions | Separate future adapter authority | Real hardware |

Apply cannot silently Export, Plan cannot arm hardware, Render cannot write a
persistent file, and Export cannot become autosave.

### Render stays derived and adapter neutral

Render consumes one accepted revision and authoritative render inputs. Browser,
CLI, MCP, and later Export paths may project that result differently, but they
must not introduce a second layout engine or reinterpret semantic content.

Browser viewport, clipboard state, adapter identity, wall-clock time, and
presentation coordinates do not become authoritative render inputs. Persistent
files require the separate Export capability.

### Export is the explicit persistent file boundary

Export is the ordinary application path from session memory to persistent
output. It requires one accepted revision, admitted output inputs, explicit
target-path authority, and explicit overwrite intent.

Browser, CLI, and MCP callers must use the same application/file boundary.
Notebook text, model text, diagnostics, clipboard content, or a prior successful
export cannot manufacture path or overwrite authority.

Same-retry recovery belongs to Export when a persistent side effect may have
completed before the caller lost its receipt. Export retry identity is not a
credential and is not reusable for unrelated operations.

### Plan stays device neutral

Plan compiles an accepted revision against an admitted Live capability profile
and returns deterministic device-neutral motion intent. It does not connect,
identify, home, calibrate, arm, start, pause, resume, cancel, or safe-stop a
physical machine.

A successful Plan receipt is not a hardware token. Physical calibration,
limits, dry run, operator arming, and adapter execution remain separate safety
boundaries.

### Adapter-specific identities remain scoped

The current contracts intentionally keep these concepts distinct:

- browser session secret;
- MCP admission identity or credential;
- prompt or command-context identity;
- semantic command retry identity;
- export retry identity;
- accepted revision identity;
- render/output identity;
- device-neutral plan identity; and
- physical compatibility or calibration identity.

Sharing an identifier between layers merely for convenience would collapse
security, retry, provenance, or reproducibility boundaries that are currently
independent.

### Current implementation status must stay explicit

The browser/localhost draft and handshake adapters have executable
implementation and regression evidence today. Clipboard presentation behavior
also has executable browser tests.

MCP application projection, general CLI projection, real Export file adapters,
PDF/image serialization, model-response parsing, media engines, and physical
hardware adapters remain partially or wholly contract-only. Documentation and
tool discovery must not describe those designed capabilities as shipped merely
because their technical contracts exist.

## Failure Modes

The adapter boundary fails when transport code mutates accepted notebook state
without the shared application authority, or when one adapter creates a second
semantic command, Render, Export, or Plan model.

Security fails when browser credentials are reused for MCP, when prompt or retry
identities are treated as authentication, or when schema discovery becomes
permission.

Persistence fails when Render or Plan writes files implicitly, when Export turns
into autosave, or when model/notebook text becomes file-path authority.

Physical safety fails when successful Plan compilation is treated as arming or
when generic semantic/MCP admission implies hardware control.

Parity fails when equivalent normalized requests produce different accepted
semantics merely because they arrived through browser, CLI, clipboard-assisted,
or MCP transport.

## Verification

Current evidence includes:

- browser/runtime handshake and draft tests under
  `tests/browser/workspace/adapter-inbound/` and the session-runtime backend
  fixtures;
- clipboard browser regressions plus the frozen clipboard transport contract;
- semantic command and history fixtures that define adapter-independent Apply,
  Validate, Undo, and Redo behavior;
- Render, Export, and Plan application contracts that explicitly separate
  derived computation, persistent files, and physical-device authority; and
- MCP contracts that require separate admission and application-capability
  parity without reusing browser or clipboard credentials.

Before adding or extending an adapter:

1. identify its admission/authentication boundary separately from document
   identity;
2. normalize only transport details explicitly admitted by its contract;
3. dispatch through the existing application capability instead of duplicating
   business rules;
4. keep persistent file effects behind Export and physical effects behind the
   hardware boundary;
5. preserve capability-specific retry, provenance, and diagnostics semantics;
6. expose unavailable capabilities honestly rather than approximating them in
   adapter code; and
7. add parity tests against another admitted adapter whenever two adapters can
   invoke the same application capability.

Still-open adapter work includes final CLI projection, executable MCP transport
and admission, persistent file adapters, PDF/image encoders, model-response and
media adapters, hardware adapters, release packaging entry points, and complete
adapter troubleshooting on packaged builds.
