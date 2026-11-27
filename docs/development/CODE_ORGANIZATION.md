# Code organization and development rules

This page maintains directory and code ownership. See the [current architecture](../architecture/overview.md)
for runtime boundaries, and the [contribution guide](../../CONTRIBUTING.md#development-and-verification)
and repository `AGENTS.md` for development and verification requirements.

| Directory | Responsibility |
| --- | --- |
| `native/otter-core` | Pure geometry, pixels, brush dynamics, cropping and basic wire-protocol value models; no Win32 window coupling. |
| `native/otter-media` | Screen and HDR color pipelines, Surface layout/rendering, and primitives for Live input and recovery connections. |
| `native/otter-runtime` | Vector annotations and editing history, workspace and Loom graph-parameter contracts, WGC window targets, and constrained HLSL/UIA/scrolling child-process runtimes. |
| `native/src/desktop` | Win32 HWNDs, message routing, visible state and Application lifecycle; external operations go to their owning worker or client. |
| `native/src` | Session persistence, Loom bridging/workflows, device projection, LiveRelay, Tile presentation, and Surface/Shader host composition; root re-exports keep subcrate call paths consistent. |
| `native/assets`, `native/build_support` | Application icons, native toolbar SVGs, embedded Fluent language catalogs, versioning and Windows resources. |
| `native/hover-hook` | Local Live hover-bridge DLL, built, verified and packaged with the same version as the EXE. |
| `native/tests/integration.rs`, `native/tests/suites`, adjacent `*_tests.rs` | Single-entry aggregated CLI/process contracts and module regressions; shared wire fixtures live in `native/tests/fixtures` or `protocol/fixtures`. |
| `scripts/native-candidate`, `scripts/tests`, `scripts/effective-code-lines*.mjs` | Provenance, packaging, runtime acceptance and effective-code-line checks; see [scripts](../../scripts/README.md). Only reproducible output belongs in temporary directories. |
| `third_party` | Pinned upstream code, patches and verifiable source provenance; no Otter business implementations. |

## Cross-module entry points

| Responsibility | Owning entry points |
| --- | --- |
| Editing budgets and cleanup after closing | Runtime `edit_worker_budget`; desktop `application_edit_retirement` and `application_output_retirement`. |
| Saving, recovery and workspace switching | Host `session_*`; desktop `application_save_*`, `application_restore_*` and `application_workflow_retirement`. |
| Restart checkpoint verification | `workspace_profile` owns the exclusive lease; `workspace_restart_checkpoint` pins the exact existing checkpoint. Hidden startup restores the verified buffer, checks its revision and never enters the resident desktop loop. |
| Loom connection and execution | Host `loom_local_connection_owner`, `loom_connection_task` and `loom_execution_tasks`; extension catalogs and commands retain separate owners. |
| Projection and reception | Desktop `pin_settings_targets`; host `device_authorization_*` and `projection_*`; policy, journal and checkpoint are separate. |
| One-way resolved source | `application_projection_sender_snapshot` separates ordinary natural pixels from `application_projection_redacted_grid`, which can only use the redacted working surface. Upstream overlays do not become target-owned editable objects. |
| Startup and session privileges | Media `autostart` owns the ordinary Run entry. Windows handles manual elevation of the desktop; `elevation_status` reads its token for the tray and gates ordinary-session integrations. |
| Live capture, input and cleanup | Runtime `capture_wgc`, `live_texture` and `live_*`; host `live_relay_*`; desktop `composition_live_*`. |
| Constrained child processes | Media `bounded_worker_process` and `bounded_worker_token` own process/job lifetime and the ordinary-user token boundary; runtime `shader_hlsl_*`, host `surface_image_worker_session` and `surface_javascript_*` own payloads. |
| Window and menu caches | Desktop `annotation_cache`, `native_menu_panel_text` and `popup_frozen_*`; resources are reclaimed with their owning window, task or application slot. |
| Capture history | Host `capture_history*` owns persisted entries; desktop `capture_history_worker` serializes background work and `capture_history_previews` owns the open panel's decoded pixels. |
| Display language | Core `display_language`; host `ui_text*` and `assets/locales`; desktop `application_locale*` owns persistence and refresh. `loom_locale*` receives authenticated one-shot requests, never a second settings owner. |
| Built-in shortcuts | Core `shortcut_policy`; host `loom_shortcut_*` and `app_settings_shortcuts`; desktop `application_shortcut_*`, `shortcut_registry` and `fullscreen_watch`. Loom selects a device and edits a draft; it does not own local registration or the settings file. |
| Selection and image archiving | Core `capture_loupe_layout`, `color_format` and `capture_last_region`; desktop `capture_loupe_window`, `capture_countdown` and `export_image_autosave`. `export_archive_directory` binds consent to fixed-local directory handles; `export_archive_file` stages and publishes without reopening the temporary file. Archive writes use output retirement, not settings autosave. |
| Static annotations and text cards | Runtime `editor_effect_rect`, `editor_serial_arrow` and `text_box_style`; desktop `clipboard_text_paste` validates the clipboard representation and `text_pin_builder` prepares content on the open worker. |
| Boxed draft editing | `annotation_text` shapes document-unit glyphs. `text_box_draft` displays the same layout and maps cluster hits to the original Win32 EDIT buffer. Unboxed legacy annotations keep their original editing path. |
| Spotlight and magnifier | Runtime `editor_spotlight` and `editor_magnifier` own editable values and history. Desktop `annotation_spotlight` unions the mask; `annotation_magnifier` owns the redacted sample with a per-frame pixel lease. `display_snapshot` controls publication of settled edits. |
| Input transparency | Desktop `pin_click_through` owns window style/region changes; `pin_click_panels` checks panel lifetime and type. `application_pin_click` coordinates workspace recovery. |
| Local path leases | Media `install_file_leases` pins files and ancestor directories for archive output, checkpoint inspection and shader payload checks. |
| Desktop arrangement | Core `pin_arrange_math`, `pin_shake_gesture`; desktop `application_pin_arrange` validates stable window owners and records position undo, `application_pin_shake` owns temporary visibility. |
| Display export | Desktop `export_style` composes the frame; `export_save_dialog`, `export_preview` and `export_codec` own selection, bounded encoding and exact-size bytes. `capture_window_meta` samples only opted-in fields; `naming_context` consumes volatile capture identity, not current foreground state. |

Keep detailed cancellation, capacity, input-validation and resource-lifetime constraints in the
owning modules and adjacent tests, not a running fix log here. NeuroDesign owns shared control
rendering and layout; Loom owns capability catalogs, execution and device authorization.
Otter integrates only through their contracts.

Split new code by responsibility, preferably into 100–250 effective lines. Files above 500 lines
require review under the repository's line-count gate; files above 700 require splitting.
Make test ownership and cleanup boundaries explicit. File moves or module splits must not change behavior.
