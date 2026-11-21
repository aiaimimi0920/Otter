# Capability details

[简体中文](capabilities.zh-CN.md) · [Documentation](../README.md)

- **Capture:** Ctrl+1 captures a region or window, Ctrl+2 creates a
  [Live sticker](../features/LIVE_CAPTURE.md), and Ctrl+3 captures a scrolling region.
  Resource limits may refuse additional Live stickers without deleting existing ones.
- **Stickers and canvas:** Ctrl+E opens editing. Crop, annotate, draw, undo, save
  workspaces or restore from the recycle bin. Hold Shift to drag an image file into
  Explorer. See [features and shortcuts](../FEATURES.md).
- **Loom and Art:** [Connect to Loom](../features/LOOM_CONNECTION.md) to use installed,
  enabled and authorized capabilities. OCR, translation and code scanning come from
  capability packages. Local capture and editing work without Loom.
- **Projection:** Select a device or group, or use a
  [QR code or link](../features/QR_PROJECTION.md). Receive (the default) and Auto
  accept authorized deliveries without a second approval. Reject and pause restrict
  participation without overriding Loom's device authorization. Invalid stored
  policy fails closed for new deliveries.
- **Tile terminal:** `--tile` runs a [wall output](../features/TILE_TERMINAL.md)
  managed by Loom, separate from ordinary projection or LiveRelay viewing.
  This does not imply physical-wall acceptance.

Protected content, UAC secure desktop and cross-user or upward-integrity input
are outside the supported scope.
