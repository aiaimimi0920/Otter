# Otter architecture

Otter is a Windows desktop application built from the `native/Cargo.toml` Cargo workspace (`otter-core`,
`otter-media`, `otter-runtime`, `otter-live-hover-hook`, and host crate `otter-native`, with unified process contract tests in
`native/tests/integration.rs`). The process uses Win32 windows, WIC image codecs, Direct2D and DirectComposition.
Rendering uses the available GPU path with bounded fallback and recovery. `neuro-mini-ui` provides shared Win32
controls from an exact NeuroDesign Git commit.

## 1. Runtime boundaries

- **Capture and editing:** The desktop message pump owns windows and input. Capture,
  decode, export and saves use cancellable workers with bounded data. Pin state and
  drawing are separate from capture and share transport.
  Long-capture recording owns its overlay layout cache; preview pixels and status updates
  reuse arrow geometry until dimensions or DPI change. Window teardown retires the cache.
  Capture delivery prepares cancellable hit masks and retains the full source image.
  Initial pin windows respect the presentation budget and fit both viewport axes.
  Native menu panels own their visible-row text and shortcut measurements; repaint and
  focus-only updates reuse them without sharing GDI resources between windows.
  Frozen-gallery index and thumbnail workers each share one slot across active and
  retired menus. Closing the menu cancels publication; the application pump reaps
  retired threads without relying on the destroyed menu timer.
  Editing tools reserve their result timer before launching a worker. Blur replacements
  retain the existing timer and single pending request; scheduling failure does not join active work.
  Closing a document cancels and transfers outstanding raster edits to the application,
  without retaining the window or publishing late results. Live and retired jobs share
  one slot budget; the message pump reaps exited workers even after window timers are gone.
  Clipboard and drag preparation lose publication authority when the document closes,
  while already admitted file exports finish under a separate bounded output budget.
- **Live:** Windows Graphics Capture owns acquisition; source input has a separate
  permission and lifetime. The co-built `otter_live_hover_hook.dll` is delivered
  next to the EXE. [Live capture](../features/LIVE_CAPTURE.md) documents the limits.
  Live relay serializes source media rotation with its socket owner; source outcome
  selection keeps authorization termination distinct from an ordinary stop.
  Snapshot-pool and staging-reuse facilities retain byte leases until GPU release,
  but production capture and relay release them per frame until shared-budget
  pressure reclamation exists. Window-private annotation caches retain committed
  drawing resources; transient previews neither replace nor evict committed geometry.
  The background wheel hook follows visible, authorized Live windows; emergency
  monitoring uses its own authenticated process and event-driven input/pipe waits.
- **Loom:** The authenticated Device connection owns the lease, directory and
  protected network requests. Local capabilities use the local bridge. Sharing,
  projected edits and Live control have separate message contracts; see
  [connection](../features/LOOM_CONNECTION.md) and [projection](../features/QR_PROJECTION.md).
  Remote and Shader execution keep retired workers in the same bounded task owner;
  post-notification deadlines wake the application until exited workers can be reaped without blocking the UI.
  Extension catalog and command owners use the same deadline mechanism. Catalog authority is revoked
  before thread reaping; unconsumed command requests are cleared before completion notification, including unwinding.
  Local Loom connection and disconnect also reap only exited workers; a replacement
  connection stays gated until the previous disconnect has finished.
- **Art surfaces:** Otter renders validated native scenes. Scripted surfaces run
  inside a restricted QuickJS process with CPU/memory limits and no DOM. Approved
  `Surface` operations and resource admission remain distinct from Loom execution.
  Short-lived image workers are scoped to the resource origin; HLSL workers are
  scoped to the target and current selection identity. Idle workers retain their
  Job limits, yield capacity to queued work and are reaped by the parent.
  SVG font admission remains per request even when its worker is reused.
- **Persistence:** Native profiles keep workspace snapshots and recovery records.
  Publishing received images requires workspace save; state changes retire stale
  worker results. Save and restore workers share a borrowed raw-window walk for device
  sender and receiver indices, while each index validates its own associations.
  Projection receive workers reuse the parsed sticker for window resources and session append;
  the resulting workspace is still validated before publication.
  A workspace swap cancels and retires old workflow workers in one application slot;
  their completion cannot publish an archive or receipt to the next workspace.
  Peer-version policy does not change workspace format handling.

## 2. Repository layout

See the [ownership map](../development/CODE_ORGANIZATION.md) for source, resources,
protocol fixtures and test placement. Start with the [build guide](../development/WINDOWS_NATIVE_BUILD.md)
for the internal package and [release provenance](../release/release-provenance.md)
for the difference between a candidate and a public release.
