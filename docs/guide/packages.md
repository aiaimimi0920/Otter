# Packages, signing and local data

[简体中文](packages.zh-CN.md) · [Documentation](../README.md)

The current development package is an unsigned internal Windows x64 ZIP. Extract it in full and run `otter-native.exe` with the
Live DLL next to it. Existing public downloads have their own release instructions. `main` does not provide a public signing or installer workflow.

Use `OTTER_NATIVE_PROFILE_DIR` for an isolated test profile. The current workspace is separate from the old profile;
source cleanup is not a migration or deletion of user data. See the [build guide](../development/WINDOWS_NATIVE_BUILD.md)
and [release provenance](../release/release-provenance.md).
