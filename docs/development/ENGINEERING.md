# Engineering contribution requirements

Start with the [ownership map](CODE_ORGANIZATION.md) and [development build](WINDOWS_NATIVE_BUILD.md).
The source, tests and current Loom contracts are the source of truth.

## 1. Scope and implementation

Identify the owning entry point, outside consumers, tests and failure behavior before changing it.
Keep the message pump non-blocking. Reuse the existing worker, budget, generation and resource lifetime
mechanisms. Move samples can coalesce; button, key-release, cancellation and shutdown edges may not be lost.
Keep physical coordinates, DIPs and source coordinates explicit. Maintain authorization and exact
session/target identity at the boundary. Do not accept old peer protocols or add downgrade paths merely to stay connected.
Hardware fallback, reconnect of the same current protocol and reading existing local user data are independent.

## 2. Tests and verification

Run the smallest applicable test first. Run `npm run verify:local` for the serial source gate:
release version consistency, strict effective-line limits, Rust formatting, workspace tests and locked build.
The [build guide](WINDOWS_NATIVE_BUILD.md) also describes ZIP and actual EXE startup verification.
Do not use synthetic window tests to claim ordinary-desktop, mixed DPI, physical input or cross-device acceptance.

The EXE workflow keeps the stable `build-windows-exe` check. Only known docs-only changes may skip the
Windows build. Other changes require the same-run native candidate gates before the verified ZIP is promoted.
Dependency and static security analysis remain independent. Review `git diff --check` and the affected diff.

## 3. Documentation and versions

Keep one maintained page per contract or operational procedure; old plans, per-version receipts and superseded
instructions live in Git. Update both README languages when user-facing information changes.
Use the internal revision command once per iteration; do not change the public version for internal work.
See [versioning](../release/VERSIONING.md) and [dependency security](../security/DEPENDENCY_SECURITY.md).
