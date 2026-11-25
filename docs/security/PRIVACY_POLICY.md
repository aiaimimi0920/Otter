# Otter Privacy Policy

This policy describes the current Otter desktop application. Historical downloads have their own behavior.

## Local data

Ordinary capture, pinning and editing do not require a cloud account or upload the captured image by themselves.
Otter does not bundle third-party advertising or analytics SDKs. Depending on use, it can store settings, workspace
snapshots and recovery data, image and clipboard caches, capture history, and runtime diagnostics locally.

Settings and the ordinary workspace default to `%LOCALAPPDATA%/OtterNative`. An explicit absolute
`OTTER_NATIVE_PROFILE_DIR` selects a separate profile. Exports and temporary files may be written outside the profile.
The current device identity uses Windows data protection for private material; do not export a whole profile to share a connection.

## Connections and optional workflows

- **Local discovery:** Otter can use an authenticated local Loom bridge by default when a trusted local connection is available.
  Set `OTTER_ENABLE_LOOM_OTTER=0` before launch to disable that bridge. The old `HOOK_ENABLE_LOOM_HOOK` fallback is read only
  when the canonical variable is absent, as implemented in [`loom_discovery.rs`](../../native/src/loom_discovery.rs).
  A saved device connection is separate; explicitly disconnect it to cancel restart reconnect.
  See [Loom connection](../features/LOOM_CONNECTION.md).
- **OCR, translation and Art:** An invoked capability receives the inputs and permissions needed for that request.
  What the capability processes locally or sends to another provider depends on the capability and Loom configuration.
  A Loom connection by itself is not consent for every capability.
- **Projection:** An authorized delivery may transmit image content and routing or association metadata through Loom.
  Account-based sharing uses the configured account service for invitation and association coordination; it does not
  require the receiver to use the same account. See the [projection boundaries](../features/QR_PROJECTION.md#内容与边界).
- **Live:** Authorized sharing can transmit frames and, when separately granted, control events. Frames are not saved in
  the Otter workspace as a recording. Local/private Loom does not require a cloud account and does not imply public relay.
  See the [Live protocol](../features/LIVE_SCREENSHOT_PROTOCOL.md).

GitHub and any separate service used to deliver downloads may process network metadata under their own policies.

## User control and reporting

Use Loom to manage capability permissions and device authorizations. Disconnect saved connections or pause receiving
when appropriate. To remove local data, exit Otter and remove the relevant exports, caches, or profile only after
backing up any work you want to keep. Removing files locally does not delete copies already sent to other devices.

Before sharing diagnostics, remove user images, text, invitations, tokens and personal paths. Use
[GitHub Issues](https://github.com/aiaimimi0920/Otter/issues) for public privacy questions and the
[private reporting process](../../SECURITY.md#reporting-a-vulnerability) for suspected exposure or vulnerabilities.
