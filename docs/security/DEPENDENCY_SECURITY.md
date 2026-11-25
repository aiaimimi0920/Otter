# Otter dependency security

`native/Cargo.lock` is the active dependency inventory. The pinned OSV scanner checks that lock.
Dependabot and CodeQL are independent signals and do not substitute for reviewed exact-lock evidence.

```powershell
npm run test:dependency-security
npm run audit:dependencies
```

The local audit defaults to `Enforce` and writes validated JSON evidence; it does not generate SARIF.
The standalone dependency-security CI workflow defaults to `Advisory`, while its reusable `workflow_call`
defaults to `Enforce`. CI validates JSON and SARIF coverage, retains both reports and attempts SARIF upload before
evaluating the findings verdict. Scanner, coverage, report or upload failures remain failures in either mode.

`Advisory` reports findings without suppressing IDs. `Enforce` rejects blocking findings not covered by a
compliant exception; it is not a blanket rejection of every advisory record. Neither this policy nor a passing
policy-contract test establishes that the current dependency graph has no vulnerabilities.

Before changing a dependency, review source, license, feature set, advisories and lockfile changes.
`neuro-mini-ui` comes only from the approved NeuroDesign repository at a full Git SHA; the candidate checks
its resolved checkout, commit, files and MIT license. Renderer patches have separately verified manifests in
[`third_party`](../../third_party/README.md). The package gate inventories all-platform normal/build dependency materials,
including exact reviewed license supplements for QuickJS. There are no legacy frontend/runtime npm dependencies.

For any future exception, use a specific advisory, reason and bounded expiry in `security/osv-scanner.toml`;
the contract rejects broad package overrides, expired exceptions and expiry beyond the policy's maximum lifetime.
See [runtime security](SECURITY_BOUNDARIES.md)
and [provenance](../release/release-provenance.md). A package build or this policy test is not a vulnerability scan result.
