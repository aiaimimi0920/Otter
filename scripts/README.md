# Otter scripts

Run from the repository root. See [build and package verification](../docs/development/WINDOWS_NATIVE_BUILD.md)
for the current PowerShell entry points. `native-candidate/` handles exact inputs, license materials and package checks.
`tests/` holds reproducible package, [Live](../docs/features/LIVE_JOINT_TESTING.md) and
[projection](../docs/features/QR_PROJECTION.md#从仓库运行联合测试) verification.
`loom-connection/` handles the explicitly imported public connection profile, not private credentials.
`native-angle/` is an opt-in shader toolchain; it is not part of the default package.
