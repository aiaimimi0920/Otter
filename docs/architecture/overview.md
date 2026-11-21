# Otter architecture

Otter is a Windows desktop application built from `native/Cargo.toml`. The process
uses Win32 windows, WIC image codecs, Direct2D and DirectComposition. Rendering
uses the available GPU path with bounded fallback and recovery. `neuro-mini-ui`
provides shared Win32 controls from an exact NeuroDesign Git commit.

## 1. Runtime boundaries

- **Capture and editing:** The desktop message pump owns windows and input. Capture,
  decode, export and saves use cancellable workers with bounded data. Pin state and
  drawing are separate from capture and share transport.
- **Live:** Windows Graphics Capture owns acquisition; source input has a separate
  permission and lifetime. The co-built `otter_live_hover_hook.dll` is delivered
  next to the EXE. [Live capture](../features/LIVE_CAPTURE.md) documents the limits.
- **Loom:** The authenticated Device connection owns the lease, directory and
  protected network requests. Local capabilities use the local bridge. Sharing,
  projected edits and Live control have separate message contracts; see
  [connection](../features/LOOM_CONNECTION.md) and [projection](../features/QR_PROJECTION.md).
- **Art surfaces:** Otter renders validated native scenes. Scripted surfaces run
  inside a restricted QuickJS process with CPU/memory limits and no DOM. Approved
  `Surface` operations and resource admission remain distinct from Loom execution.
- **Persistence:** Native profiles keep workspace snapshots and recovery records.
  Publishing received images requires workspace save; state changes retire stale
  worker results. Peer-version policy does not change workspace format handling.

## 2. Repository layout

See the [ownership map](../development/CODE_ORGANIZATION.md) for source, resources,
protocol fixtures and test placement. Start with the [build guide](../development/WINDOWS_NATIVE_BUILD.md)
for the internal package and [release provenance](../release/release-provenance.md)
for the difference between a candidate and a public release.
