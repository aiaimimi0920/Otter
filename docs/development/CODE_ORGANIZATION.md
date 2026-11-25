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
| `native/assets`, `native/build_support` | Application icons, native toolbar SVGs, versioning and Windows resources. |
| `native/hover-hook` | Local Live hover-bridge DLL, built, verified and packaged with the same version as the EXE. |
| `native/tests/integration.rs`, `native/tests/suites`, adjacent `*_tests.rs` | Single-entry aggregated CLI/process contracts and module regressions; shared wire fixtures live in `native/tests/fixtures` or `protocol/fixtures`. |
| `scripts/native-candidate`, `scripts/tests`, `scripts/effective-code-lines*.mjs` | Provenance, packaging, runtime acceptance and effective-code-line checks; see [scripts](../../scripts/README.md). Only reproducible output belongs in temporary directories. |
| `third_party` | Pinned upstream code, patches and verifiable source provenance; no Otter business implementations. |

## Cross-module entry points

| Responsibility | Owning entry points |
| --- | --- |
| Editing budgets and cleanup after closing | Runtime `edit_worker_budget`; desktop `application_edit_retirement` and `application_output_retirement`. |
| Saving, recovery and workspace switching | Host `session_*`; desktop `application_save_*`, `application_restore_*` and `application_workflow_retirement`. |
| Loom connection and execution | Host `loom_local_connection_owner`, `loom_connection_task` and `loom_execution_tasks`; extension catalogs and commands retain separate owners. |
| Projection and reception | Desktop `pin_settings_targets`; host `device_authorization_*` and `projection_*`; policy, journal and checkpoint are separate. |
| Live capture, input and cleanup | Runtime `capture_wgc`, `live_texture` and `live_*`; host `live_relay_*`; desktop `composition_live_*`. |
| Constrained child processes | Media `bounded_worker_process`; runtime `shader_hlsl_*`; host `surface_image_worker_session` and `surface_javascript_*`. |
| Window and menu caches | Desktop `annotation_cache`, `native_menu_panel_text` and `popup_frozen_*`; resources are reclaimed with their owning window, task or application slot. |

Keep detailed cancellation, capacity, input-validation and resource-lifetime constraints in the
owning modules and adjacent tests, not a running fix log here. NeuroDesign owns shared control
rendering and layout; Loom owns capability catalogs, execution and device authorization.
Otter integrates only through their contracts.

Split new code by responsibility, preferably into 100–250 effective lines. Files above 500 lines
require review under the repository's line-count gate; files above 700 require splitting.
Make test ownership and cleanup boundaries explicit. File moves or module splits must not change behavior.
