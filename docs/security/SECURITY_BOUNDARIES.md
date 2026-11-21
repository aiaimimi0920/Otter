# Otter security boundaries

## Device identity and network

Otter uses the device authorization owner to bind Loom ID, verified endpoint, trust configuration and the current lease.
Pairing or import never substitutes an arbitrary remote origin. Remote requests must use the authenticated transport;
Loom credentials and local private identity material are not exposed to UI scripts, sharing links or diagnostics.
The current device identity store protects the private identity with Windows data protection. Test connections and
loops must not use production credentials. See [Loom connection](../features/LOOM_CONNECTION.md).

## Projection and Live control

Import parses and verifies a bounded invitation locally; explicit acceptance and the current scope are checked by Loom.
The receiving pin is committed to the workspace before display. Unknown, stale or revoked operations cannot publish.
Remote Live input belongs to a specific source/epoch and control grant; disconnect, close, revocation and source change
retire that right and release held input. Capabilities and invitations do not grant operating-system input authority.
See [projection](../features/QR_PROJECTION.md) and [Live protocol](../features/LIVE_SCREENSHOT_PROTOCOL.md).

## Files and runtime work

File readers and image decoders admit bounded input before allocating. Workspace transactions bind revision, identity and
source. Local schema support and recovery records exist to preserve user work; a current-peer-only policy does not retire them.
Art Surface scripts run as restricted native workers; allowlisted requests, graphics budgets and resource provenance belong to
the host. In tests use disposable profiles and owned processes rather than the current workspace.

## Evidence

Do not publish bearer tokens, private keys, QR invitation content, user images or real profile data in reports.
Dependency, source and archive checks do not confer a security approval. For vulnerability disclosure, use
[the reporting policy](../../SECURITY.md); for dependency policy see [dependency security](DEPENDENCY_SECURITY.md).
