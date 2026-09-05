# Localhost session runtime contract

## Status

Frozen for the first-release runtime boundary.

## Purpose

This contract defines the observable security and lifecycle behavior of the
localhost Rust service and browser session. It connects the local-first document
ADR to the P1 service, browser launch, local authentication, handshake, and
session-destruction work without choosing a Rust web framework.

## Scope

The contract covers process startup, loopback binding, browser origin, local
session authentication, request admission, startup publication, shutdown, and
in-memory session lifetime. It does not define notebook fields, application
commands, or file-format schemas.

CLI and MCP adapters may share the same application services, but they do not
implicitly inherit browser credentials. MCP follows the frozen local MCP
session-admission contract; every other inbound adapter declares its own
admission boundary.

## Contract

### Loopback binding

The service binds only an operating-system loopback address. A wildcard,
private-LAN, public, tunnel, container-bridge, or user-selected remote address
is not an admitted first-release configuration.

The preferred IPv4 endpoint is `127.0.0.1`. IPv6 loopback `::1` may be admitted
when the platform and browser launch path prove equivalent host and origin
checks. Binding one family does not justify silently opening the other family.

The process asks the operating system for an available port by binding port
zero, then reads the assigned port from the bound listener. It does not select a
port by probing and later reopening it, because another process could claim the
port between those operations.

The listener is created before any browser is launched or startup success is
published. Failure to bind is a startup failure and creates no active session.

### Canonical browser origin

One successful startup selects one exact browser origin from the bound address
and assigned port. For IPv4 it has this form:

```text
http://127.0.0.1:<port>
```

The browser frontend uses only that origin for application requests. Alternate
host spellings such as machine names, LAN addresses, wildcard aliases, or an
attacker-controlled domain resolving to loopback are not equivalent origins.

The service validates the HTTP `Host` value against the exact startup endpoint.
Requests with an absent, malformed, or different host are rejected before they
reach application services.

### Session secret

Each process startup creates a new session secret from a cryptographically
secure operating-system random source. The secret has at least 256 bits of
unpredictable material before encoding and is never reused after shutdown.

The secret exists only in process and browser memory. It is not written to a
notebook, log, cache, crash-recovery file, shell history, exported document,
render manifest, or URL query string.

The launcher delivers the secret to the initial browser document without making
it a normal HTTP request target. A URL fragment is an admitted first-release
handoff because the fragment is not transmitted in the HTTP request. Frontend
startup consumes the fragment, stores the secret only in memory, and removes it
from the visible browser location before ordinary navigation or copying.

### Request authentication

Every API request that reads or mutates session-private state presents the
session secret in an authorization header. Static frontend resources and a
minimal unauthenticated health endpoint may be public because they expose no
session state or mutation capability.

The backend compares credentials without data-dependent early exit. Missing,
malformed, stale, or incorrect credentials receive the same unauthenticated
response shape and never disclose whether a notebook exists.

The service never authenticates a request through cookies, ambient browser
credentials, source IP alone, a predictable process identifier, or knowledge of
the port number.

### Origin admission

Browser API requests must also carry the exact startup origin where the browser
platform supplies origin metadata. A different origin is rejected even when it
can reach loopback and possesses no valid session secret.

The service does not enable permissive cross-origin resource sharing. It does
not reflect arbitrary request origins, admit wildcard origins, or accept browser
mutation through form-compatible endpoints that another local or remote page
could trigger without the application client.

Every runtime response also carries
`Content-Security-Policy: frame-ancestors 'none'` and
`Referrer-Policy: no-referrer`. A real Firefox fixture must be able to load the
workspace directly while a distinct loopback origin fails to embed the same
resource as a frame.

Authentication and origin checks are separate. The secret protects the active
session from unrelated local pages and processes that discover the port; exact
origin admission reduces browser-based request-forgery and rebinding surfaces.

### Startup publication

Startup has three externally visible phases:

```text
starting
listening
ready
```

`starting` means no endpoint is promised. `listening` means the loopback socket
is owned but the application handshake is not yet ready. `ready` means the
frontend resource set and protocol handshake endpoint are available for the
current session.

Machine-readable startup output contains the process version, exact loopback
origin, protocol version, and readiness state. It never contains the session
secret. Human-facing launch errors identify the failed phase without dumping
request headers or private source material.

### Protocol handshake

The first authenticated application exchange is a version handshake. It
compares product, protocol format, prompt, profile, renderer, and
capability versions before editing commands are enabled.

A mismatch is a typed incompatibility diagnostic. The browser cannot downgrade
or ignore a required backend incompatibility, and the backend does not accept an
unknown required frontend capability merely because both processes are local.

The first-release browser sends `POST /api/handshake` with no body after it
scrubs the launch fragment. The request carries the session Bearer credential,
the exact startup `Origin`, and exactly one of each required version header:

- `X-Atrament-Capability-Version: atrament.capability/1`;
- `X-Atrament-Product-Version: 0.1.0`;
- `X-Atrament-Profile-Version: atrament.profile/1`;
- `X-Atrament-Prompt-Version: atrament.prompt/1`;
- `X-Atrament-Protocol-Version: atrament.runtime/1`; and
- `X-Atrament-Renderer-Version: atrament.renderer/1`.

A compatible exchange returns `200` with all six backend identities. An
authenticated mismatch returns `409` with the mismatched dimension and required
identity without reflecting the browser-provided value. Authentication or
origin failure returns the same `401` response shape before compatibility is
reported, and the service emits no permissive CORS response header.

Refreshing the browser does not create a new backend session. A refresh may
rejoin the current process only when it still possesses the in-memory session
secret and completes the handshake again.

### Pre-acceptance draft mutation

The first mutable browser application state is source-preparation draft text,
not an accepted notebook revision. Task text, source material, and the raw
external-model response remain separate complete fields in backend process
memory. Replacing one field cannot implicitly accept, parse, or apply another.

The first-release browser replaces those fields with authenticated same-origin
`POST` requests to `/api/session/task`, `/api/session/source`, and
`/api/session/candidate`. Bodies are UTF-8 text with one explicit
`Content-Length`; transfer encoding, malformed framing, and invalid UTF-8 are
rejected before mutation. Each field has a backend-owned one-mebibyte limit, and
over-limit input returns `413` without truncating or changing the current field.
A successful whole-field replacement returns `204`.

Framing failures that can be classified after connection admission return `400`
without invoking draft mutation. Request reads and response writes each use one
total transport deadline rather than a per-I/O budget. Incomplete request bodies
return `408`, while a slow response consumer loses its connection when the write
budget expires; intermittent progress cannot hold the single-thread listener
indefinitely.

Authenticated `GET` requests to the same three field paths return the exact
current UTF-8 field as uncached plain text. A read always requires the Bearer
credential. When the browser or another admitted client supplies `Origin`, it
must equal the startup origin. An absent `Origin` does not replace credential
authentication with ambient trust.

The browser coalesces rapid edits per field: it may skip obsolete intermediate
values but sends only complete field values, never browser-authored patches.
Pending draft synchronization is invalidated when the page leaves the active
session. Draft requests carry the in-memory Bearer credential and rely on the
browser-provided exact `Origin`; rejected authentication or origin admission
never invokes the draft application service.

### In-memory authority

Notebook documents, imported asset bytes, undo history, previews, diagnostics,
and derived render or motion plans live only in the active process memory unless
an explicit import or export operation requires bounded temporary storage.

The runtime does not create a database, autosave journal, browser local-storage
notebook, service worker cache of session data, hidden recovery file, or cloud
copy. Frontend state that can reconstruct private notebook content is session
memory, not browser persistence.

Temporary media or conversion files use repository-independent operating-system
runtime storage owned by the process adapter. They have bounded names and
lifetimes, are not document authority, and are removed on success or failure as
soon as the owning operation no longer needs them.

### Explicit file boundaries

Import and export operations are the only ordinary path from session memory to
persistent product files. The user selects or supplies the target path through
an explicit action, and the backend validates the operation against the owning
adapter contract.

A successful export does not turn its directory into an autosave location.
Subsequent edits remain memory-only until another explicit export.

### Shutdown and failure

Orderly shutdown stops accepting new application requests, cancels or safely
finishes owned operations according to their adapter contracts, releases the
listener, clears session memory, and removes owned temporary intermediates.

Process crash recovery does not reconstruct the notebook. A restarted process
creates a different port or listener ownership, a new session secret, an empty
session, and a fresh handshake.

If automatic browser launch fails before the service reaches `ready`, startup
fails closed. The listener and fresh session credential are released with the
process, and the secret-free recovery instruction directs the user to fix
browser
launch and restart Atrament. Startup must not publish a bare origin as a usable
continuation path or weaken host or authentication checks for manual opening.

## Failure Modes

The runtime contract is violated if the service binds `0.0.0.0`, `::`, a LAN
interface, or another remotely reachable address. It also fails if startup
chooses a port through a probe-then-bind race or accepts an arbitrary `Host`
value.

It is a failure to place the session secret in logs, a query string, a file,
browser persistence, or exported output. A local page must not gain notebook
read or mutation access by discovering the port, sending a cross-origin form,
or causing a browser to attach ambient credentials.

The contract is also violated if refresh or restart silently restores notebook
state, if protocol mismatch still enables editing, or if a failed import,
conversion, export, or hardware operation becomes accidental persistence.

## Verification

Network tests must inspect the bound socket and prove no non-loopback interface
accepts connections. Host tests must reject machine names, LAN addresses,
malformed hosts, and attacker-controlled hostnames that resolve to loopback.

Authentication tests must cover missing, malformed, incorrect, prior-session,
and current secrets. Browser security tests must show that an unrelated origin
cannot read or mutate session state even when it knows the loopback port.

Lifecycle tests must exercise browser refresh, browser close, orderly process
shutdown, forced process termination, and restart. Only explicit exports may
survive the process; notebook state, credentials, undo history, previews, and
owned temporary media must not.

Handshake tests must cover every required version identity and prove that one
incompatible required version blocks application commands with a typed
incompatibility diagnostic.
