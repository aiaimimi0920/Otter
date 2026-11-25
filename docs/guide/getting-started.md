# Getting started with Otter

[Simplified Chinese](getting-started.zh-CN.md) · [Documentation](../README.md)

## Setup

- This guide uses English control names. Right-click the tray icon and choose **Language / 语言 → English / 简体中文** to change Otter's display language; no capability package is required.
- Windows 10 or 11. Extract the entire internal ZIP, run `otter-native.exe` and keep `otter_live_hover_hook.dll` in the same directory. Internal ZIPs are unsigned development packages, not public releases or installers; older public downloads have their own instructions.
- Capture, pins and annotations work without Loom. OCR, translation and Art require current Loom, installed capabilities and authorization.
- Workspace and settings default to `%LOCALAPPDATA%/OtterNative`, separate from historical releases. Use an absolute `OTTER_NATIVE_PROFILE_DIR` for an isolated test profile; do not overwrite a running profile or copy device credentials.

The selection is saved and applied without restarting after active modal operations finish.
Loom's Otter Connection settings can send the same change to one connected device.
This is separate from OCR translation and Loom's own language. See [display language](../features/DISPLAY_LANGUAGE.md).

## 1. Capture and pin

1. Launch Otter, press **`Ctrl+1`**, and select the area to capture.
2. Move the resulting pinned image to the desired desktop position for reference or further editing.
3. Use **`Ctrl+2`** for live capture or **`Ctrl+3`** for long capture. Source compatibility and resource limits are explained in [live capture](../features/LIVE_CAPTURE.md).

UAC secure-desktop capture and overlays are unsupported. Live capture does not guarantee background input in every application or a fixed frame rate. See [security boundaries](../security/SECURITY_BOUNDARIES.md).

## 2. Edit and annotate

1. Select a pinned image and press **`Ctrl+E`** to show or hide the editing toolbar.
2. Use text, arrows, shapes, brushes, blur and other tools. Press **`Ctrl+Z`** to undo or **`Ctrl+Y`** to redo.
3. With focus on the pin, press **`Tab`** to open its parameter panel. Inside the panel,
   Tab navigates controls; **`Esc`** cancels an edited value or leaves the panel.
4. Press **`Esc`** to cancel the current gesture, selection or dialog first. When the pin is idle, Esc closes it; this is not deletion of the source file. Use **`Delete`** to delete a selected annotation.
5. Press **`Ctrl+S`** to save the selected image. In browse/pan mode, hold **`Shift`** to drag a PNG copy into File Explorer; editing tools may use Shift for their own actions.

Read more: [brush and Bézier editing](../features/BEZIER_EDITING.md) · [features and shortcuts](../FEATURES.md).

## 3. Use OCR, translation and Art nodes

Start Loom, install and enable the required capability packages, and complete Otter's connection and authorization. See [global Loom connection](../features/LOOM_CONNECTION.md) for connection, approval and reconnection details.

With an image selected and a capability catalog providing these bindings (the installed catalog
and granted permissions determine availability):

| Shortcut | Action |
| --- | --- |
| **`Ctrl+4`** | Run the available OCR capability again; copying the complete result also requires clipboard permission. |
| **`Alt+4`** | Toggle OCR results; recognize and display them if none exist. |
| **`Ctrl+5`** | Translate cached OCR content, recognizing first if needed. |
| **`Alt+5`** | Toggle translation results, generating the required results on first use. |

Press **`Shift+1`** to open the Art menu, then choose an available capability to add a node.
The menu shortcut is built in; its candidates still depend on the capability catalog and authorization.

Connect nodes and assemble processing flows in Otter's node workspace; Loom supplies installed and authorized capabilities. Available nodes and parameters depend on those capabilities. OCR and translation overlays are mutually exclusive; toggling cached results does not repeat model requests.

To recover a Surface closed in this session, use the Loom menu to query closed Surfaces,
then explicitly confirm a recovery candidate. See [Surface recovery](../development/SURFACE.md);
this does not promise automatic recovery across processes or versions.

## 4. Project a pinned image to another device

A device here means an Otter receiver that supports the existing projection protocol and has completed the required connection and authorization. It does not imply compatibility with every client or platform. Start with two Windows computers running Otter and paired to the same Loom:

1. Connect both devices to the same Loom and complete pairing and approval. Enable receiving on the receiver; new deliveries require Receive or Auto for automatic acceptance.
2. On the sender, select a pinned image, press **`Ctrl+E`**, and open **Pin settings → Projection**.
3. Open the target device or device-group list and select an available target, then click Confirm. The sender shows success after a receipt; check that the image is visible on the receiver.
4. Once associated, later edits on the sender continue to synchronize. Deselect the target and confirm to stop projection; closing the panel does not stop an existing association.

QR-code and link invitations are also supported. Receive them through the tray's receive-projection entry or the image's import panel. See [image projection](../features/QR_PROJECTION.md) for invitation expiry, cross-Loom trust, receiving policies and two-way editing limits.

## Troubleshooting

- OCR, translation or Art unavailable: check Loom connection and authorization, and whether the corresponding package is installed and enabled.
- Receiver missing: check both connections, pairing approval and the receiving policy. A connected status does not grant every remote capability.
- Unexpected shortcut behavior: check whether an image, annotation or text input is selected and whether editing is still active. Follow the current shortcut configuration.
- Before reporting, search [existing issues](https://github.com/aiaimimi0920/Otter/issues) and follow [ways to contribute](../../CONTRIBUTING.md). Include the version, reproduction steps and sanitized screenshots. Do not upload keys, tokens or private screen content.

[Development and verification](../development/WINDOWS_NATIVE_BUILD.md) · [Tile wall mode](../features/TILE_TERMINAL.md)
