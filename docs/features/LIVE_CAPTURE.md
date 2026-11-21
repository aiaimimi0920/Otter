# Live capture

Use **Ctrl+2** or the tray and select a region. A selection fully inside a valid
program window follows that window's local pixels when it moves; other selections
capture a screen region. Each Live sticker has its own placement, refresh and
input. Drag a yellow corner to move it. Clicking the image can operate the source
when the source and input method are supported. A browser window is captured as an
external native window: scrolling and tab changes change the visible pixels.
No browser extension or pinned-page mode is required or supplied.

## Watch from another device

On the source Live sticker, use **实时投射 → 发布到 Loom**. On the receiver,
open a Surface-capable Art's parameters, refresh the LiveRelay list, select the
session and choose **加入观看**. Joining requires a current authorized Surface
attachment and does not grant input control. Closing the panel during join cancels
a late result; an existing viewer has independent lifetime. **重新配对** submits
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
- Runtime budgets govern capture admission and cadence. See
  [resource admission](LIVE_RESOURCE_ADMISSION.md),
  [shared WGC capture](LIVE_SHARED_SOURCE_CAPTURE.md) and
  [GPU presentation](LIVE_GPU_PRESENTATION.md). Copy/save/edit use snapshot
  boundaries; per-frame changes are not continuous formal Art outputs.
- Use the [current joint runner](LIVE_JOINT_TESTING.md) and
  [receiver diagnostics](LIVE_RELAY_DIAGNOSTICS.md) for controlled evidence.
  Neither a successful build nor diagnostic counters prove physical display,
  manual input, real-network performance or acceptance on another device.
