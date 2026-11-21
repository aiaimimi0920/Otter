# Otter release provenance

`main` produces **internal** Windows packages via the [build and verification guide](../development/WINDOWS_NATIVE_BUILD.md).
There is no public release or signing entry point for this product in `main`. Existing GitHub releases are not changed.

- `build-native-candidate.ps1` binds the Git revision/dirty state, input hashes, exact Cargo resolution,
  pinned NeuroDesign input, tool versions and the EXE/DLL digests. It rejects source changes during the build.
- Packaging rechecks the input snapshot, collects applicable third-party license materials and records every file signature
  as a SHA-256 digest, not a cryptographic publisher signature. The independent archive verifier validates file layout and digests.
- The packaged startup probe uses the actual extracted EXE and an isolated profile; it does not replace physical or joint acceptance.
- Dirty source may be used only as internal evidence. Future public publication requires an exact clean source, passed dependency
  review, the required runtime gates, new release tooling and separate approval. See [versioning](VERSIONING.md).
