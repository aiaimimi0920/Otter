# Third-party notices

Otter-owned code is MIT licensed. The dependency graph is pinned by `native/Cargo.lock`.
The [patched renderer sources](third_party/README.md) retain their original licenses and exact source records.
The Windows control library `neuro-mini-ui` comes from NeuroDesign under its MIT license.
The lossless WebP encoder is `image-webp` 0.2.4. The separately evaluated VP8 encoder is
`tiny-webp` 0.1.1 (MIT OR Apache-2.0), used without its CLI, default features or C bindings.
The QR reader `rxing` uses a pinned upstream Git commit under Apache-2.0; its exact source
and license material are verified alongside the other candidate inputs.
Packaged internal builds carry license texts and an `all-platform-normal-and-build-closure` inventory:
the resolved normal/build dependency closure across platforms, not an exact list of components linked into the EXE.
This notice is a required hashed candidate-build input. Neither the inventory nor its hashes imply legal,
security or public release approval.
