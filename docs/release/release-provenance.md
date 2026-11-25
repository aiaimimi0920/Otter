# Otter release provenance

`main` produces **unsigned internal Windows x64 ZIPs** via the [build and verification guide](../development/WINDOWS_NATIVE_BUILD.md).
GitHub Actions artifacts are development candidates, not GitHub Releases or installers. There is no public release
or signing entry point in `main`; existing GitHub releases are unchanged. See the [signing policy](CODE_SIGNING_POLICY.md)
for the separate requirements on a future signed release.
Do not label an internal ZIP as a signed installer or UIAccess package.

CI builds and uploads the internal package only when change classification selects the full product gates and those
gates pass. A documentation-only classification runs documentation checks and skips the EXE build; a green check
on that path is not a newly built package.

- `build-native-candidate.ps1` binds the Git revision/dirty state, input hashes, exact Cargo resolution,
  pinned NeuroDesign input, tool versions and the EXE/DLL digests. It rejects changes to the inventoried build inputs,
  tool snapshot or `HEAD` during the build; this is not a claim to track every file in the worktree.
- Packaging rechecks the input snapshot and collects applicable third-party license materials.
  `package-manifest.json` records SHA-256 digests for its listed payload files, excluding the manifest itself.
  These hashes are byte-consistency evidence, not a cryptographic publisher signature. The independent archive
  verifier validates file layout and digests.
- The packaged startup probe uses the actual extracted EXE and an isolated profile; it does not replace physical or joint acceptance.
- Dirty source may be used only as internal evidence. Future public publication requires an exact clean source, passed dependency
  review, the required runtime gates, new release tooling and separate approval.

## Versioning

- Public identity is `vx.y.z`; historical uppercase tags retain their original identity.
- Internal identity is `vx.y.z.n`. Run `npm run version:internal` once per internal development iteration and include
  `version-state.json`; do not create a public tag or bump the public version as part of that iteration.
  Documentation-only changes do not require an internal version increment.
- Keep the three-part public version aligned in `package.json`, `version-state.json`, `native/Cargo.toml`,
  and the manifests for `native/otter-core`, `native/otter-media`, `native/otter-runtime` and `native/hover-hook`.
  `npm run verify:version` checks those fields and the `otter-native` package entry in `native/Cargo.lock`;
  it does not check every workspace package entry in the lockfile.
- A separately authorized public release advances the three-part version and resets `internalRevision` to zero.
  The fourth component identifies an internal build in provenance, not a Cargo or npm dependency version.
  Internal candidates and four-part tags are not inputs to a public release.
