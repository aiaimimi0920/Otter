# Otter release provenance

`main` produces **unsigned internal Windows x64 ZIPs** via the [build and verification guide](../development/WINDOWS_NATIVE_BUILD.md).
GitHub Actions artifacts are development candidates, not GitHub Releases or installers. There is no public release
or signing entry point in `main`; existing GitHub releases are unchanged. See the [signing policy](CODE_SIGNING_POLICY.md)
for the separate requirements on a future signed release.

- `build-native-candidate.ps1` binds the Git revision/dirty state, input hashes, exact Cargo resolution,
  pinned NeuroDesign input, tool versions and the EXE/DLL digests. It rejects source changes during the build.
- Packaging rechecks the input snapshot, collects applicable third-party license materials and records every file signature
  as a SHA-256 digest, not a cryptographic publisher signature. The independent archive verifier validates file layout and digests.
- The packaged startup probe uses the actual extracted EXE and an isolated profile; it does not replace physical or joint acceptance.
- Dirty source may be used only as internal evidence. Future public publication requires an exact clean source, passed dependency
  review, the required runtime gates, new release tooling and separate approval. See [versioning](VERSIONING.md).
