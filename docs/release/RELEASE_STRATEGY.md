# Release strategy

`main` builds internal Windows x64 ZIPs. GitHub Actions artifacts are internal candidates, not Releases.
No existing public asset, signing state or installer changes when a candidate is built.
For local packaging see the [build guide](../development/WINDOWS_NATIVE_BUILD.md); for promotion requirements see
[release provenance](release-provenance.md) and [versioning](VERSIONING.md).
