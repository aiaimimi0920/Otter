<p align="center"><img src="./src-tauri/icons/otter-icon.svg" width="80" height="80" alt="Otter capture icon"></p>
<h1 align="center">Otter</h1>
<p align="center"><strong>Capture it. Pin it. Make it yours.</strong><br>Connect nodes. Create effects. Turn your desktop into a creative workspace.</p>
<p align="center">
  Website ·
  <a href="https://github.com/aiaimimi0920/Otter/releases">Download</a> ·
  <a href="./docs/README.md">Documentation</a> ·
  <a href="https://github.com/aiaimimi0920/Otter/issues">Feedback</a>
</p>
<p align="center">
  <a href="./LICENSE"><img src="https://img.shields.io/badge/license-MIT-36564f?style=flat-square" alt="License: MIT"></a>
  <img src="https://img.shields.io/badge/platform-Windows_10_%2F_11-36564f?style=flat-square" alt="Windows 10 / 11">
  <a href="https://github.com/aiaimimi0920/Otter/actions/workflows/build-otter-exe.yml"><img src="https://github.com/aiaimimi0920/Otter/actions/workflows/build-otter-exe.yml/badge.svg" alt="Build Otter EXE"></a>
</p>
<p align="center">English · <a href="./README.zh-CN.md">简体中文</a></p>

Otter is a free, open-source screenshot tool that brings capturing, annotating,
organizing and sharing into your everyday work. Turn your desktop into a workbench:
connect different processing nodes to combine functions, create effects and
explore more possibilities for the images you capture.

<details open>
<summary>Desktop workspace · Show / hide animation</summary>

<p align="center">
  <img src="./docs/assets/otter-workflow-demo-v2.gif" width="860" alt="Otter workflow demo: capture, translation, node processing, two-way projection, annotation, editing and drag-and-drop saving.">
</p>

</details>

[Features](#features) · [Get started](#get-started) · [Contribute](#contributing)

## Features

- **Capture the view you want.** Take region, window and scrolling screenshots, and pick colors from the screen.
- **Pin images to your desktop.** Keep references in view and organize your material with groups, the library and history.
- **Annotate freely and keep editing.** Crop, erase and rotate; add text, arrows and shapes; hide information with blur and mosaic.
- **Keep pinned images live.** Turn a window or part of the screen into a live pinned view to follow changing content and interact with supported source windows.
- **Connect nodes and combine effects.** Use Loom's OCR, translation and Art capabilities, connecting nodes to build your own processing flows.
- **Project your view to other devices.** Send images or live views to connected, paired devices for viewing and collaboration across screens.

[**Full features →**](./docs/FEATURES.md)

## Get started

- **Capture and pin:** Capture any area of your desktop and pin it anywhere on the desktop.
- **Edit quickly:** Use the editing toolbar to quickly annotate pinned images.
- **Connect a workflow:** Use Loom's node tools to assemble advanced workflows.
- **Share the view:** Use the push tools to send a pinned image's content to connected, paired and authorized Otter receivers.

| Start here | What you need to do |
| --- | --- |
| [Download Otter](https://github.com/aiaimimi0920/Otter/releases) | Extract and launch the Otter App to get started. |
| Download Otter + Loom | Extract and launch the Otter App and Loom App. Install capability packages such as OCR and translation in Loom, and add commonly used MCP tools and Art nodes as needed to expand your creative workflow. |

1. Press **`Ctrl+1`** for a region screenshot, **`Ctrl+2`** for live capture, or **`Ctrl+3`** for a scrolling screenshot.
2. Select a pinned image and press **`Ctrl+E`** to show or hide the editing toolbar, or **`Tab`** to expand or collapse the parameter panel. After leaving editing or selection mode, press **`Esc`** to delete the pinned image.
3. With the corresponding Loom capabilities available, press **`Ctrl+4`** for OCR and **`Alt+4`** to toggle recognition results; press **`Ctrl+5`** to translate and **`Alt+5`** to toggle translation results; press **`Shift+1`** to open the Art menu and add a new node.
4. Select a pinned image, open the toolbar, check projection targets, and click Confirm. Receivers using Receive or Auto accept automatically; the sender shows waiting and received counts. Both ends must use the latest Otter.

[**Complete usage guide →**](./docs/guide/getting-started.md)

## Windows-native main branch

`main` now builds a Rust/Win32 application without WebView2. See the [native build guide](docs/guide/development.md) for local builds and internal packages. The last WebView-based `main` is preserved as `archive/webview-main`; existing public downloads are unchanged.

The native right-click recycle bin, favorites and clipboard submenus use compact three-column cards with local wheel scrolling and a top-right × to delete an individual entry. Library actions remain available by clicking a card. Hovering a clipboard card opens a larger text or aspect-preserving image preview beside the grid; it opens to the left when the screen's right edge has insufficient space. Thumbnail updates preserve scrolling. Ordinary desktop appearance still requires hand testing.

Native pin right-click menus now follow the main-branch Save/Close, recycle-bin and favorites layout, with Clipboard as the only additional entry. Tray right-click follows Capture, Live capture, Long capture, Edit existing image, Receive projection, Autostart and Exit; it no longer appends pin libraries or migration commands. Endpoint strips use the normal arrow cursor, including after linking or cancellation. Ordinary desktop gestures and cross-device reception still require hand testing.

Native `Ctrl+1` now selects over the changing desktop instead of showing a frozen full-screen preview. Drag and release to capture the selected region at release time; Escape/right-click cancels. Existing pins remain visible, and Live/long-capture selection keeps its separate behavior.

Ordinary region delivery now reacts to capture and document-preparation completion rather than waiting for the hidden capture/import polling ticks. Release-time sampling and HDR preservation remain unchanged; actual release-to-visible latency still depends on acquisition and first-frame rendering.

Pasted pins now appear with the editing toolbar closed and remain freely draggable. The source pin keeps its current editing state; Ctrl+E or F4 still opens the pasted pin's toolbar explicitly. Source-relative positioning and cumulative paste offsets are unchanged.

Native copy now binds its internal source to the final clipboard sequence after Windows closes and synthesizes image formats, preventing ordinary pastes from losing the source anchor. An isolated-window-station regression exercises actual system publication and reading; physical shortcuts still require hand testing.

Normal native startup keeps an empty workspace in the tray without a console or button panel; saved pins still restore. Clicking the tray icon while the workspace is empty keeps its command host hidden instead of displaying a black window. After copying pin A, pastes anywhere inside A share one 20/40/60-DIP cascade from A's current top-left corner, regardless of cursor movement or overlapping copies. Moving outside the pins retains the copied mouse offset. Flattened Art and thumbnails use the same placement rules. Shift-drag filenames use `{label}_{shortId}_{date}_{time}.png` by default, with centisecond time precision and no overwrite. Physical clipboard/drag gestures and ordinary desktop appearance still require hand testing.

Tab's left/right rows, inline controls, continuous backing and bottom node drawer follow their own pin's Z-order and topmost state rather than the lowest pin in the workspace. The bottom caret keeps a stable horizontal position when toggled. Port content (including Tab and F2) and the editing toolbar's left-hand Pin Settings panel are mutually exclusive for the same pin, preserving parameter drafts. The bottom triangle's drawer is independent and can coexist with either panel. Row gaps remain click-through, and focusing a field does not reorder the pins. Ordinary desktop appearance still requires hand testing.

The native `Shift+1` Art picker uses a centered 250×300 DIP search menu with clear/close controls and single-click selection. It keeps the existing modal, IME and authorization flow rather than claiming full non-modal parity. Collapsed bottom drawers clip away the rectangle outside the triangle, and Tab updates retain complete control frames. Shift-drag from an idle Pan or quiet-mode pin can save a new PNG copy to a folder; only the export copy may be moved, never the original image. Physical dragging and ordinary desktop appearance still require hand testing.

The unpublished Windows-native branch adds `F6` port navigation, `Ctrl+Enter` keyboard linking, and `F2` focus entry into the corresponding attached inline public Art parameter row. These are development-branch controls, not a claim about the downloaded public release or completed desktop acceptance. See the [current native parity and verification boundaries](docs/development/WINDOWS_NATIVE_PARITY.md).

The native toolbar's second button is now a selection toggle, enabled whenever `Ctrl+E` opens editing. Clicking it again (or pressing `Q`) enters quiet mode: existing annotations cannot be picked, while a newly created annotation remains selected for immediate adjustment. Move/Rotate/Scale dropdown modes are removed; selected annotations retain their handles and modifier gestures. Rotation and scale handles follow the live drag preview. Ordinary desktop hand testing remains separate.

In the native editing toolbar's selection mode, `Ctrl+Alt+Wheel` scales selected annotations around their group center; adding `Shift` scales around each annotation's own center. These gestures do not resize the pinned window or change its opacity. See the [manual regression matrix](docs/FEATURES.md).

The native line group contains only straight lines and Bézier curves. Arrowheads remain a line property; the duplicate standalone Arrow/Polyline entries and their A/U tool shortcuts are no longer offered. Toolbar and property-row drawing changes still require ordinary desktop hand testing.

The native branch also consumes trusted Loom extension shortcut declarations, including OCR/translation execution and overlay toggles when the installed capabilities declare them. Native text inputs keep their editing keys; physical-key and real-model acceptance remain separate from source regression tests.

Official OCR/translation overlay text in the native branch supports single-block click-to-copy, same-block drag selection and Ctrl+C, without moving the pin. Blank space and painting tools retain their existing input. See the [manual regression matrix](docs/FEATURES.md) for verification boundaries.

An active official OCR data type can keep its saved overlay visible during a missing-renderer transition. Unavailable extension data uses bounded, noninteractive badges; revoked or offline capabilities do not retain their old attachment scenes.

Native extension feedback keeps the newest eight notices above the pin, with local stack/long-card scrolling and five-second dismissal. It follows the pin without activating it; notification wheels do not resize or change the pin's opacity. Ordinary desktop acceptance remains separate.

As on the baseline, active developer-mode extensions may display saved views and retain their notices. This display eligibility does not authorize extension commands, shortcuts or attachment writes; those keep their existing trusted execution boundary.

The native pin settings include the baseline's original/translated full-text copy buttons. Hidden cached results remain copyable without executing a model; stale translations are unavailable. Full text is not silently truncated, and the clicked button briefly reports success or failure.

Expanded native Tab port rows reveal hidden ports and use the baseline eye icons to change collapsed visibility without deleting graph links. Tab now places ordinary-image and public Art parameter controls inside their attached rows, with a local scrolling parameter section beneath fixed explicit inputs. Scrolling keeps text drafts and HWNDs; typed options and batch commits use the existing parameter transaction. Local disclosure, total Tab and F2 share the same inline owner; independent rows may stay open together. Standalone local Art forms and the right-side auxiliary area remain in migration.

Native Tab rows keep hidden normal-size port slots in place, synchronize local disclosure on Tab state changes, and allow each anchor to fold independently after expanding all rows. Consecutive expanded rows share the baseline dark backing without making row gaps interactive; output eyes and names use the baseline content slots. Merely focusing a port or clicking its label does not expand it; connection gestures start and end on anchors. Fields use the same attached inline controls for local disclosure and total Tab; the right-side auxiliary region and local Art forms remain transitional. Ordinary desktop appearance requires separate hand testing.

The native Ctrl+E toolbar also presents trusted extension toolbar contributions, including direct text buttons and grouped command lists. Catalog/source changes invalidate old clicks; the existing command authorization path remains in force. Art VIEW selection and the remaining Tab/connection UI are still in migration.

The native pencil's GLOW switch retains the pencil tool, opacity and smoothing fields. Highlighted pencil strokes use the existing 35% wash multiplied by pencil opacity, keep pencil smoothing, and remain editable through undo/redo and saved sessions. The duplicate standalone highlighter entry and H tool command are removed; saved highlighter strokes retain their rendering.

Native document pins expose the attached bottom node drawer after an explicit node click or Tab expansion. Its original icons control collapsed endpoint visibility, downstream notifications, sticker edit inheritance, and Art automation/manual execution. The drawer follows its own pin in the existing movement and Z-order group; settings and port content share an exclusive disclosure space. Existing parameter drafts commit through the same transaction before a setting or execution action. Settings survive saved sessions; drawer disclosure is transient. Complete node multiselection, standalone Live settings, Art VIEW and connection/projection UI remain in migration. Ordinary desktop acceptance is separate.

Public Art result candidates in the native branch now use a compact attached grid below Tab's right output rows, with separate local scrolling and the existing candidate/parameter execution transaction. Real multi-result Loom and ordinary desktop acceptance remain separate; Live auxiliary controls and local Art forms are still in migration.

Native pin settings provide Connect/Disconnect, the connected Loom account, device/group/user checklists and four compact receive policies. QR and link panels stay inside the pin and switch between LAN and public sharing through the current authenticated Loom. Importing a PNG or pasting a link only parses it locally until Import is clicked; acceptance adds a linked pin and saves it before display. Received pins expose their source and unlink action. Only current v2 clients are supported. Two real Loom daemons with different synthetic accounts have passed the Application lifecycle for PNG updates, reopening and unlinking; ordinary desktop, physical cross-device and public-network acceptance remain separate. See the [current sharing scope](docs/features/QR_PROJECTION.md).

## Contributing

Bug reports, translations, documentation improvements and code contributions are welcome.
Start with [ways to contribute](./docs/guide/contributing.md) or the
[translation guide](./docs/guide/translating.md), and explore the
[repository layout](./docs/architecture/overview.md#2-repository-layout) to understand the code structure.
Code changes follow the [engineering contribution guide](./CONTRIBUTING.md).
File placement and module ownership follow the [code organization standard](./docs/development/CODE_ORGANIZATION.md).
Search [existing issues](https://github.com/aiaimimi0920/Otter/issues) before reporting a problem.

## License

Otter-owned code is [MIT licensed](./LICENSE). Dependencies retain their own
[licenses and notices](./THIRD_PARTY_NOTICES.md).
