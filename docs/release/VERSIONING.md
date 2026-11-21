# Otter versioning

- Public identity: `vx.y.z`; historical uppercase tags retain their original identity.
- Internal identity: `vx.y.z.n`. Run `npm run version:internal` once per iteration and include `version-state.json`.
- Keep the three-part value aligned in `package.json`, `version-state.json`, `native/Cargo.toml` and
  the Otter entry in `native/Cargo.lock`. Check with `npm run verify:version`.
- Do not create a public tag or bump the public version during an internal iteration.
  See [release provenance](release-provenance.md) for the currently available internal package process.
