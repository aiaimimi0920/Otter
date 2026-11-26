# Otter architecture

Otter is a Windows desktop application built from the `native/Cargo.toml` Cargo workspace (`otter-core`,
`otter-media`, `otter-runtime`, `otter-live-hover-hook`, and host crate `otter-native`, with unified process contract tests in
`native/tests/integration.rs`). The process uses Win32 windows, WIC image codecs, Direct2D and DirectComposition.
Rendering uses the available GPU path with bounded fallback and recovery. `neuro-mini-ui` provides shared Win32
controls from an exact NeuroDesign Git commit. This page describes runtime responsibilities; the
[ownership map](../development/CODE_ORGANIZATION.md) locates their code.

## 1. Runtime boundaries

- **Capture and editing:** The desktop message pump owns windows, input and publication of
  worker results. Capture, decode, raster edits and output run under bounded worker budgets.
  Closing a document revokes late publication and transfers outstanding work to application
  retirement owners; it does not block the UI waiting for threads. Already admitted file exports
  may finish under their separate output budget. Drawing caches belong to the window or operation,
  not a global cross-window resource pool.
  Ordinary canvases use shared D3D11 devices with window-private surfaces; Tile
  retains D3D12-first selection. Hardware fallback still requires confirmed retirement.
  Capture history keeps compact encodings in save snapshots. Its single worker decodes
  panel previews, which are discarded when the panel closes.
- **Live:** Windows Graphics Capture owns acquisition; source input has a separate
  permission and lifetime. The co-built `otter_live_hover_hook.dll` is delivered
  next to the EXE. [Live capture](../features/LIVE_CAPTURE.md) documents the limits.
  Source recovery and emergency shutdown use separately owned processes. Remote media,
  control authorization and input release follow the [LiveRelay contract](../features/LIVE_SCREENSHOT_PROTOCOL.md).
- **Loom:** The authenticated Device connection owns the lease, directory and
  protected network requests. Local capabilities use the local bridge. Sharing,
  projected edits and Live control have separate message contracts; see
  [connection](../features/LOOM_CONNECTION.md) and [projection](../features/QR_PROJECTION.md).
  Cancellation revokes execution or catalog authority before thread reaping. Connection
  replacement waits for the previous disconnect; it cannot reuse a stale lease or task result.
- **Art surfaces:** Otter renders validated native scenes. Scripted surfaces run
  inside a restricted QuickJS process with CPU/memory limits and no browser DOM. Approved
  `Surface` operations and resource admission remain distinct from Loom execution.
  Short-lived image workers are scoped to the resource origin; HLSL workers are
  scoped to the target and current selection identity. Idle workers retain their
  Job limits, yield capacity to queued work and are reaped by the parent.
  SVG font admission remains per request even when its worker is reused. See the
  [Surface and Shader contract](../development/SURFACE.md) for supported host operations.
- **Persistence:** Native profiles keep workspace snapshots and recovery records.
  Publishing received images requires workspace save; state changes retire stale
  worker results. Restore and append validate workspace contents and projection associations
  before publishing windows.
  A workspace swap cancels and retires old workflow workers in one application slot;
  their completion cannot publish an archive or receipt to the next workspace.
  Peer-version policy does not change workspace format handling.
- **Display language:** Embedded Fluent catalogs are presentation resources, not executable
  capabilities. The Application owner serializes local tray and authenticated Loom requests
  through the existing settings transaction. Language publication follows successful persistence;
  changing presentation must not replace document state, editor drafts or business stamps.
  See [display language](../features/DISPLAY_LANGUAGE.md) for defaults and remote request semantics.

## 2. Repository layout

See the [ownership map](../development/CODE_ORGANIZATION.md) for source, resources,
protocol fixtures and test placement. Start with the [build guide](../development/WINDOWS_NATIVE_BUILD.md)
for the internal package and [release provenance](../release/release-provenance.md)
for the difference between a candidate and a public release.
