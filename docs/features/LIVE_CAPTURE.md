# Live capture

Use **Ctrl+2** or the tray and select a region. A selection fully inside a valid
program window follows that window's local pixels when it moves; other selections
capture a screen region. Each Live sticker has its own placement, refresh and
input. Drag a yellow corner to move it. Clicking the image can operate the source
when the source and input method are supported. A browser window is captured as an
external native window: scrolling and tab changes change the visible pixels.
No browser extension or pinned-page mode is required or supplied.

## Watch from another device

On the source Live sticker, use **Live Projection → Publish to Loom**. On the receiver,
open a Surface-capable Art's parameters, refresh the LiveRelay list, select the
session and choose **Join Viewer**. Joining requires a current authorized Surface
attachment and does not grant input control. Closing the panel during join cancels
a late result; an existing viewer has independent lifetime. **Pair Again** submits
the existing device identity after Loom deletes the registration; approval and
joining again are separate actions. An old revoked viewer is not revived.

LiveRelay is distinct from [QR/link projection](QR_PROJECTION.md), which syncs
images and does not convey controllable Live media. See the
[LiveRelay protocol](LIVE_SCREENSHOT_PROTOCOL.md) for media and authorization.

## Compatibility and validation

- WGC may stop producing frames for a static source. Existing pixels remain;
  source closure or identity loss is terminal, and recoverable capture failures
  use cancellable backoff. A failed or ambiguous input is not replayed.
- Live input is restricted to the active desktop/session and the same user's
  equal-or-lower-integrity windows. Controlled Win32/WinForms checks do not
  certify every Chromium, WPF, Qt, game or elevated control, nor truly minimized
  or protected windows. Multi-monitor, mixed DPI, HDR and device loss need their
  own checks with the relevant build.
- Capture sharing is scoped to the source identity. Each consumer retains its crop,
  placement, presentation and input permission. Closing one consumer does not close a
  source still in use; cleanup retires leases and in-flight work at the owning boundary.
- Pixel, byte and queue budgets limit capture admission and cadence. A newer frame can
  replace pending presentation work, but source loss, input release and teardown cannot
  be dropped. GPU recovery is independent of peer versions. Copy/save/edit use snapshots;
  per-frame changes are not continuous Art outputs.
- Use [joint testing and diagnostics](LIVE_JOINT_TESTING.md#diagnostics-and-evidence-boundaries) for test
  procedures, metric interpretation and the limits of controlled evidence.
