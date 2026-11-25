# Otter scripts

Run from the repository root. See [build and package verification](../docs/development/WINDOWS_NATIVE_BUILD.md)
for the current PowerShell entry points. `native-candidate/` handles exact inputs, license materials and package checks.
`tests/` holds reproducible package, [Live](../docs/features/LIVE_JOINT_TESTING.md) and
[projection](../docs/development/JOINT_TESTING.md#account-png-joint-testing) verification.
`loom-connection/` handles [explicit import of public Loom connection details](../docs/features/LOOM_CONNECTION.md#importing-public-connection-details), not private credentials.
The package runtime and joint test runners reuse [`native-angle/build_job.py`](native-angle/build_job.py)
for Windows child-process cleanup. They do not use ANGLE to build the product.
