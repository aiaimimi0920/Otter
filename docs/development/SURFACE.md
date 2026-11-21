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
