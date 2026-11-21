# Otter code signing policy

The current `main` builds **unsigned internal ZIPs**. It does not publish releases
or run the historical SignPath/UIAccess workflow. Existing signed or unsigned
public downloads are unchanged. Do not label a portable ZIP as an installer,
UIAccess package or signed build.

## Requirements for a future signed release

- The maintainer selects an explicit public `vx.y.z` tag reachable from protected
  `main`, reviews its source and workflow, and builds from that tag in the
  public repository. A four-part internal build or a local rebuild cannot be used.
- Bind the signing input to the exact repository, source SHA, tag, Actions run
  and reviewed SHA-256. Never replace the reviewed artifact. Verify Authenticode
  and the resulting package before publishing; see [release provenance](release-provenance.md).
- Only authorized maintainers may review and approve signing; use MFA and hosted
  key storage. The signing-provider decision is manual and separate from the
  protected GitHub Environment approval. No private key or live token may be
  committed or logged. Fail closed if configuration, approval, provenance or
  signature verification is missing. Do not publish an unsigned installer instead.
- If the SignPath lane is reinstated, document the exact inputs and include the
  applicable attribution: Free code signing provided by [SignPath.io](https://signpath.io/),
  certificate by [SignPath Foundation](https://signpath.org/).

## Incident response

Stop new approvals, revoke or rotate affected access, quarantine affected assets
and publish guidance about the safe version once facts are known. Use the
[private reporting process](../../SECURITY.md#reporting-a-vulnerability) for
suspected key, account or release-provenance compromise. Project roles are in
[governance](../../GOVERNANCE.md).
