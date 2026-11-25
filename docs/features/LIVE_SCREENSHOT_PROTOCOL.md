# LiveRelay protocol

Use matching current Otter and Loom versions. Loom's `protocol/LIVE_SCREENSHOT_PROTOCOL.md`
and `protocol/schemas/live-control.v1.schema.json` define the server contract; Otter owns
capture, presentation and local windows. The `v1` in a protocol name is not a legacy-client mode.

## Control, media and performance

- `loom.live.v1` control messages use `protocolVersion`, `sessionId`, `epoch`, `sequence`,
  `messageType` and `payload`. Session, permission, key/button edges, observation and trigger-audit
  events must remain ordered. Only replaceable mouse-movement samples may be coalesced.
- `/v1/live/media` is a separate WebSocket media channel. The shared 64-byte `NLLV` header contains
  epoch, frame ID, capture/encode timestamps, dimensions, keyframe status, dropped-frame count,
  color space, codec and payload length. Frames are neither saved to the workspace nor included
  in Surface control polling.
- The subprotocol selected by the WebSocket handshake determines the media profile. Viewers offer
  `loom.live.h264.v1`, `loom.live.jpeg.v1` and `loom.live.v1` (raw BGRA), in that order. Sources offer
  only raw by default, or H.264 and raw after successfully creating the system encoder; they do not
  offer JPEG. After a viewer decoding-path failure, reconnection offers only raw and does not advertise
  video capability again. H.264 still depends on the source encoder, server video policy and receiver
  decoder. These fallbacks do not require a legacy client.
- The raw profile accepts neither JPEG nor H.264. JPEG is accepted under the JPEG or H.264 profile;
  H.264 is accepted only under the H.264 profile. JPEG frames are independent keyframes; H.264 requires
  ordered decoding and cannot be treated as independent images. See
  [`live_relay_media_endpoint.rs`](../../native/src/live_relay_media_endpoint.rs).
- Frame headers, dimensions, sequence, codec and allocation budgets are checked at the receiving
  boundary. Corrupt frames do not replace usable content, and epoch changes do not mix in old frames.
  `NLLV` framing and receive admission live in
  [`live_relay_frame.rs`](../../native/otter-media/src/live_relay_frame.rs) and
  [`live_relay_receive.rs`](../../native/otter-media/src/live_relay_receive.rs).
  Host modules in `native/src/live_relay_*` own connections and lifecycles. For metric interpretation
  and evidence limits, see [joint testing and diagnostics](LIVE_JOINT_TESTING.md#diagnostics-and-evidence-boundaries).

Frame dimensions may change within a published session. An H.264 source rebuilds the relevant
encoder on the media thread and starts with a new keyframe; a failed encoder is not automatically
revived by resizing. Loom and viewers update cache, decoding and bitmap state from frame-header
dimensions. Initial dimensions in the session catalog are not the current frame dimensions.

## Authorization and recovery

Selecting a session or joining as a viewer does not grant input control. Device identity,
Surface attachment, controller lease, epoch, authorization expiry and revocation state must be
checked separately. Windows input remains constrained by the current user, session, integrity
level and secure desktop. Public discovery returns no observation values; ordinary logs do not
retain titles, pixels, keyboard input or OCR content.

Renewal reuses the original signed device identity and membership without re-pairing, replaying
input or reacquiring a controller lease. Receivers derive their cursor from `requesterControl`
in the authenticated snapshot; new connections start at the confirmed `viewer_joined` event.
Identity, epoch, snapshot or membership mismatches stop the flow. Loom checks
`requireExistingMembership` atomically. Source recovery retires the old worker before resuming
from the confirmed observation sequence; old cached values cannot become new trusted observations.

When a Device media connection receives `1008` with reason `live_media_device_revoked`, it closes
the local relay, releases authorization and clears received frames. An ordinary disconnect is not
device revocation, and HTTP 401 authentication handling does not match response text. After a source
receives an authenticated HTTP 401 or media-revocation message, a concurrent stop does not attempt
to close the original session. A 401 during closing or pre-close reconciliation records authorization
termination without claiming the session was closed. Failures do not trigger unconfirmed new joins
or replay. Remote media requires currently authorized HTTPS/WSS; local loopback has separate restrictions.
Do not claim public NAT/TURN relay capability where none is available.

## Observation and triggers

UI Automation reads currently permitted controls on a dedicated thread. Stable-locator conflicts,
unknown values and stale values are not inferred to be `exact`. Snapshots and events are bounded,
and observation sequences are independent of media frame IDs. Otter validates and forwards trigger
requests bound to a Surface; Loom owns trusted values, stability durations, permissions, idempotency
and actual Art execution. Optional vision/OCR capabilities come from validated Loom Capability
Plugins, not implicit activation by the Otter core.

See also [usage and validation boundaries](LIVE_CAPTURE.md#compatibility-and-validation).
