# Image projection: devices, QR codes and links

## Current scope

Upgrade Otter and Loom together. There are three independent entry points. The selected workflow
determines the protocol; failure does not trigger a protocol downgrade:

| Entry point | Current protocol and authorization scope |
| --- | --- |
| LAN QR/link (the panel's default scope) | `otter://projection/lan#` wraps `sourceLoomId` and a signed `neuro.qr-projection.v1` envelope. Targets the current or an interconnected Loom; no central account is required. |
| Public-network account QR/link | `neuro.qr-projection.v2` JSON or a `hook://projection/v2#` link. Loom must have a configured central service and a signed-in account; the receiver may use a different account. |
| Directed delivery to devices, device groups or users | Uses `neuro.qr-projection.v1` envelopes and the Loom inbox; not a fallback for failed QR/link sharing. |

All entry points use the currently connected, Device-authorized Loom, either local or on the LAN.
Platform owns public-network account authorization, invitations and association coordination.
Cross-Loom images reuse the existing Iroh channel. Otter retains ownership of the canvas, export,
receiver windows and workspace checkpoints; it does not provide a standalone browser viewer.

## Sharing by QR code or link

1. On the sender, select an ordinary pin or an Art with a committed image. Open Ctrl+E → Pin Settings
   → Projection and choose QR or link export. The panel floats above and moves with the pin.
   Switch between LAN and public-network scopes; unconfigured public-network sharing is explicitly unavailable.
2. Switch between QR and single-line link views within the same panel. Both reuse the current
   scope's invitation and window. The panel may extend beyond the pin, but is constrained by
   the monitor work area. Closing it does not stop saved associations.
3. On the receiver, open the import panel from the incoming-projection entry in pin settings
   or Receive Projection in the tray. Select the invitation's LAN/public-network scope, then
   choose a QR PNG or paste a link. Both undergo bounded local parsing. Public v2 signatures
   are verified locally with the envelope's public key, which does not itself establish the
   account-to-key binding. Authenticated Loom verifies LAN v1 signature authenticity.
4. Only clicking Import checks and accepts the invitation through the receiver's currently
   authorized Loom. Success creates a new associated pin without overwriting the entry pin.
   Received content is displayed only after entering an actual workspace checkpoint.
5. The receiver continues updating when the sender changes its image or Art produces a new
   committed image. Closing the sharing panel does not stop synchronization. Each sharing path
   persists its own associations for reopening. Receiver position, window size and local layout
   do not follow sender movement.
6. On the sender, choose Unlink in the sharing panel for the corresponding scope. Receivers
   can leave through incoming-projection settings. After remote confirmation and persistence
   of the stopped state, synchronization stops and the receiver retains the final image.
   Network failures preserve current content and report failure rather than falsely claiming unlinking succeeded.

Otter issues LAN v1 invitations valid for 4 minutes 30 seconds. Public v2 invitations last at most
5 minutes and cannot outlive the current account session. Accepted associations do not stop when
the invitation expires. Each invitation binds one receiving device and one receiving Unit.
Expired invitations are not re-signed to masquerade as the original association.

## Cancellation, pausing and reopening

Closing the import panel cancels uncommitted tasks and prevents late results from being displayed.
Saved content remains; cancellation cannot roll back acceptance already completed remotely.
The write-ahead record retains the original invitation and receiving Unit for later reconciliation.

Pausing local reception invalidates current participation credentials; resuming does not revive
previously in-flight results. Rejecting new deliveries is separate from restoring accepted
associations: rejection does not prevent reopening accepted associations, while pausing still
constrains both. Cancellation, source changes or receiving-permission changes during a checkpoint
must also prevent late presentation.

Recovery first reads Loom's authoritative state and fetches content only after confirming the original
acceptance exists; unknown acceptance is not automatically replayed. Multiple recovery intents within
one workspace are handled separately. After unlinking, a write-ahead record is retired only when the
stopped state has actually been saved. Save conflicts, disk-write failures and workspace changes
do not produce a Saved receipt.

## Content and boundaries

Ordinary pins produce composited PNGs through the existing export path; Art uses its committed image.
Receivers need not execute the same Art. This sharing path currently transfers continuously updated
PNGs, not cross-Loom Live media; PNG polling cannot substitute for that media path. Directed
device/group delivery and bidirectional annotation are separate workflows. This flow does not
transfer independently editable annotation objects.

The central-service address in a public-network QR code must match Loom's signed-in service,
but sender and receiver accounts need not match. Otter's network target remains the currently
authenticated Loom. LAN links carry `sourceLoomId`; Loom routes only to itself or an interconnected
Loom with pinned trust, never to arbitrary origins from a link. Address aliases for the same Loom
can be used after validation against the authenticated import response. The signed invitation
remains unchanged, and persistence records the actual authenticated transport address.
Local parsing or signature verification is not authorization. QR codes cannot switch connections,
grant device permissions or elevate receiver permissions to sender permissions.
Account credentials and Loom private keys never enter Otter, QR codes or workspace association records.
The final image is retained while Loom is offline. If public-network account authorization becomes invalid,
resuming synchronization still requires renewed current account authorization.

PNGs are limited to 4 MiB, 8192 per dimension and 16,777,216 total pixels.
QR images are limited to 4 MiB and 4096 per dimension, with bounded decode scaling and a single worker.
If ordinary QR detection fails, at most one additional pure-code extraction is attempted through
the same parsing and authorization path for the selected scope.
Projection JSON requests and responses are limited to 6 MiB. Loom's LAN projection store and
public-network local outbox each have separate limits of 64 entries and 48 MiB of PNGs;
they do not share one global quota.

## Directed projection and receiving policies

1. Complete connection and device approval as described in [Loom connection](LOOM_CONNECTION.md).
2. Select a source pin, press `Ctrl+E`, and select devices, groups or users under Projection in
   pin settings, then confirm. At confirmation, groups and users resolve to a fixed set of
   authorized devices; refreshes cannot expand the task. Deselecting targets also requires confirmation.
3. The receiver must stay connected, have device permission and participate in the projection
   inbox. The sender displays Received only after actual workspace persistence and receipt
   confirmation. Closing settings does not revoke existing projections.

| Receiving policy | New deliveries and existing projections |
| --- | --- |
| Accept (default), Automatic | Automatically accepts authorized new deliveries without a second approval prompt. |
| Reject | Rejects new deliveries without revoking accepted associations. |
| Pause | Stops receiving participation and updates. Switching to another policy clears the pause but does not revive old in-flight results. |

Accept is not a guarantee of success and does not bypass Loom authorization, catalog or content
validation. Invalid receiving configuration blocks new deliveries. The previous catalog may remain
visible during refresh, but dispatch never uses a stale catalog. Inbox refresh is independent of
old-projection cleanup. Bounded retries handle only explicitly retryable same-Loom errors,
not automatic replay of writes with unknown outcomes or cross-Loom writes.

- **Target not visible:** First connect both ends to the same reachable Loom. Confirm that the
  receiver is approved, not paused and participating in the server's projection inbox, then
  refresh the sender. For interconnected Loom instances, also check connection and catalog
  authorization. A cached catalog is not proof that a device is online.
- **Stuck waiting:** Confirm that you clicked Confirm, then check the receiver's new-delivery
  policy, inbox and workspace-write result. Pending entries do not prove presentation, and
  closing the panel is not cancellation.

See [two-machine acceptance and diagnosis](../development/JOINT_TESTING.md) and
[account PNG joint testing](../development/JOINT_TESTING.md#account-png-joint-testing).
Account joint tests cover only the public v2 synthetic-account flow between two Loom instances
on one machine, not LAN v1, physical two-machine or real public-network acceptance.
