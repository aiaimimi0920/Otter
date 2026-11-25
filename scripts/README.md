# Otter scripts

Run from the repository root. See [build and package verification](../docs/development/WINDOWS_NATIVE_BUILD.md)
for the current PowerShell entry points. `native-candidate/` handles exact inputs, license materials and package checks.
`tests/` holds reproducible package, [Live](../docs/features/LIVE_JOINT_TESTING.md) and
[projection](../docs/development/JOINT_TESTING.md#账号-png-联测) verification.
`loom-connection/` handles the explicitly imported public connection profile, not private credentials.
The package runtime and joint test runners reuse [`native-angle/build_job.py`](native-angle/build_job.py)
for Windows child-process cleanup. They do not use ANGLE to build the product.

`node scripts/review-coverage.mjs` reports the current [source review coverage](../docs/development/review-coverage.json),
checking recorded source hashes instead of treating changed files or passing tests as completed review.
