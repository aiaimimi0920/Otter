# Art Surface

Otter accepts typed scene operations, not HTML from an external process. Surface scene, layout, event delivery and
retirement live in `native/src/surface_*` and `native/src/desktop/*surface*`. Shared drawing primitives retain their
own bounds and resource budgets. Requests are tied to the current scene and authorized Loom identity; stale revisions,
unknown resource ownership and invalid layouts do not produce a committed scene.

The optional JavaScript mode executes in a restricted QuickJS worker reused by the Surface host. It does not load a browser,
fetch arbitrary URLs or grant desktop/native permissions to the script. SVG, image and text operations are validated and
rendered by the native host; images and text selection have distinct budgets and hit-test ownership.

Verify changes with the owning `surface_*` Rust tests and, when crossing process or peer boundaries, the exact worker or
Loom integration tests. Synthetic scenes do not establish visual parity or physical touch/IME acceptance.

Capability catalog visibility is not authorization to execute an offer or redirect input.
Remote offers and subsequent operations must bind to the authorized Loom session and current scene.

A closed Surface releases its slot only after JavaScript has finished, no rejected owner remains,
and its attachment is confirmed inactive or disposed. A failed admission that never created a window
can also release its slot after a confirmed terminal transition. Otter stages up to one journal batch
(32 hints, 64 KiB encoded) before releasing either kind of slot; normal shutdown writes that batch
separately from unresolved attachments. If the hint does not fit, the original owner and its slot remain.
An older or equal mount result for a retired attachment remains duplicate evidence; it cannot start
another deactivation. A newer authenticated remount is not blocked by the older hint.
The Loom menu can query a retired Surface in the current session without importing a file. A hint
is not permission to remount: only a fresh authenticated candidate and explicit confirmation can start
remounting. Importing a recovery file does not replace current-session choices. The staged batch is not
crash-durable.

## Shader

Product Shader uses the native `otter.hlsl.v1` ABI, not GLSL, WebGL or ANGLE.
The `shader_hlsl_*` modules validate the ABI, reflected bindings, input dimensions and allocation budgets;
compilation and off-screen rendering run in a bounded child process. Direct3D backend selection and
hardware fallback are runtime concerns, not peer-version compatibility.
`--shader-candidate` is rejected at startup. No ANGLE DLLs are needed for the native Shader path.

Changes to this path can be checked with:

```powershell
cargo test --locked --manifest-path native/Cargo.toml -p otter-runtime --lib shader_hlsl -- --test-threads=1
cargo test --locked --manifest-path native/Cargo.toml -p otter-native --test integration shader_startup:: -- --test-threads=1
```

Optional ignored cases have their own fixture requirements; a contract test is not proof of GPU parity
or an accepted Art workflow on a user's desktop.
