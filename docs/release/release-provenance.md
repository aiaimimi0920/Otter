# Otter release provenance

`main` produces **unsigned internal Windows x64 ZIPs** via the [build and verification guide](../development/WINDOWS_NATIVE_BUILD.md).
These GitHub Actions artifacts are development candidates, not GitHub Releases or installers.
The separate **Build public Otter portable ZIP** workflow builds an unsigned portable release from a tag.
See the [signing policy](CODE_SIGNING_POLICY.md) for the separate requirements on a future signed installer.
Do not label an internal ZIP as a signed installer or UIAccess package.

CI builds and uploads the internal package only when change classification selects the full product gates and those
gates pass. A documentation-only classification runs documentation checks and skips the EXE build; a green check
on that path is not a newly built package.

The separate `Compile Windows (no package)` workflow uploads only a compilation receipt,
not binaries or a package. Its `compilationOnly=true`, `packageVerified=false` and
`formalReleaseApproved=false` fields distinguish compiled EXE/DLL hashes from candidate
provenance. Success does not satisfy or bypass the unchanged `build-windows-exe` test,
package or runtime gates, and does not authorize a formal release.

- `build-native-candidate.ps1` binds the Git revision/dirty state, input hashes, exact Cargo resolution,
  pinned NeuroDesign input, tool versions and the EXE/DLL digests. It rejects changes to the inventoried build inputs,
  tool snapshot or `HEAD` during the build; this is not a claim to track every file in the worktree.
- Packaging rechecks the input snapshot and collects applicable third-party license materials.
  `package-manifest.json` records SHA-256 digests for its listed payload files, excluding the manifest itself.
  These hashes are byte-consistency evidence, not a cryptographic publisher signature. The independent archive
  verifier validates file layout and digests.
- The packaged startup probe uses the actual extracted EXE and an isolated profile; it does not replace physical or joint acceptance.
- Dirty source may be used only as internal evidence. Public publication requires an exact clean source, a passed
  dependency review, the required runtime gates and separate maintainer approval.

## Public Windows x64 portable release

1. With maintainer approval, align the product versions and set `internalRevision` to zero.
   Push the reviewed commit to `main`. Require a full successful `Build Otter EXE` on that SHA,
   not a documentation-only or compilation-only check. Its `vx.y.z.0` artifact is still internal.
2. Create a new lowercase three-part tag at that commit. Dispatch `public-portable.yml` **on the tag**.
   The workflow checks the full product run, enforces the OSV verdict on the exact lock, then builds the
   EXE and DLL afresh from a clean tag reachable from `origin/main`. There is no internal ZIP input.
3. The workflow independently validates the archive, starts its extracted EXE, and attests its SHA-256
   using GitHub build provenance. The static manifest grants no publication approval; the runtime receipt and
   successful workflow are separate evidence. Windows binaries are unsigned and do not request UIAccess.
4. From the successful run, verify the ZIP and `.sha256` against both receipts and the artifact attestation.
   Check `build-provenance.json` for the repository, tag, SHA, run and attempt. Upload only the verified ZIP
   and checksum to a **new draft** at that existing tag; check both uploaded digests before publishing.
   Read back the public release, tag and downloads. Do not overwrite existing tags or assets.

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
