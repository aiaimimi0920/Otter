# Otter dependency security

`native/Cargo.lock` is the active dependency inventory. CI runs the pinned OSV scanner against that lock,
validates JSON and SARIF coverage, and retains evidence. Advisory mode reports findings without suppressing
IDs; scanner, coverage and report failures are not advisory success. Enforce mode does not allow current findings.
Dependabot and CodeQL are independent signals and do not substitute for reviewed exact-lock evidence.

```powershell
npm run test:dependency-security
npm run audit:dependencies
```

Before changing a dependency, review source, license, feature set, advisories and lockfile changes.
`neuro-mini-ui` comes only from the approved NeuroDesign repository at a full Git SHA; the candidate checks
its resolved checkout, commit, files and MIT license. Renderer patches have separately verified manifests in
[`third_party`](../../third_party/README.md). The package gate inventories actual non-dev dependency materials,
including exact reviewed license supplements for QuickJS. There are no legacy frontend/runtime npm dependencies.

For any future exception, use a specific advisory, reason and bounded expiry in `security/osv-scanner.toml`;
no broad package overrides or expired exceptions. See [runtime security](SECURITY_BOUNDARIES.md)
and [provenance](../release/release-provenance.md). A package build or this policy test is not a vulnerability scan result.
