"""The sole Clang/D3D11 static-CRT profile for pinned upstream GN; do not trim application JS/Shader contracts."""

import hashlib
import json

ANGLE_REVISION = "ded782bca9d5f165d1c4a70124cdc5384043a8b3"
PROFILE = "windows-x64-d3d11-clang-release-staticcrt"
ANCHORS = {
    ".gn": "b0da847d35931c15caacee0707a37269bfcfa66cc6ea822635e62dff884fdb61",
    "build/config/win/visual_studio_version.gni": "7d096a03d6d860046523b52cb7cc33fa788c2feb33fef9f6abdcd169fd25fcb3",
    "build/toolchain/win/BUILD.gn": "a805f46696b7a18f306dbfe08e646c438b62a5f596a1662d2ff309db97f27e5f",
}


def gn_string(value):
    # Configuration boundaries already validate paths, but GN's $ must still be rejected; JSON quoting is not GN evaluation isolation.
    if "$" in value:
        raise ValueError("GN interpolation is not admitted")
    return json.dumps(value.replace("\\", "/"), ensure_ascii=False)


def profile_files(originals, config, python):
    if set(originals) != set(ANCHORS):
        raise ValueError("Unexpected GN profile inputs")
    for name, expected in ANCHORS.items():
        if hashlib.sha256(originals[name]).hexdigest() != expected:
            raise ValueError(f"Pinned GN profile identity mismatch: {name}")
    result = {}
    text = originals[".gn"].decode("utf-8")
    result[".gn"] = text.replace('script_executable = "python3"', "script_executable = " + gn_string(python), 1)
    path = "build/config/win/visual_studio_version.gni"
    text = originals[path].decode("utf-8")
    start = text.index('if (visual_studio_path == "") {')
    end = text.index("visual_studio_version_logs =", start)
    # Explicit roots and markers reject discovery/external runtime-library directories; never fabricate WDK.
    replacement = f'''# Otter explicit desktop profile; never discover VS/SDK or invent a WDK.
assert(visual_studio_path == {gn_string(config['visualStudioRoot'])})
assert(visual_studio_version == "2022")
assert(windows_sdk_path == {gn_string(config['sdkRoot'])})
assert(windows_sdk_version == "10.0.26100.0")
assert(wdk_path == "", "This desktop profile does not provide a WDK")
visual_studio_runtime_dirs = "otter-no-runtime-dirs"

'''
    result[path] = text[:start] + replacement + text[end:]
    path = "build/toolchain/win/BUILD.gn"
    text = originals[path].decode("utf-8")
    start = text.index("# Copy the VS runtime DLL")
    end = text.index('if (target_cpu == "x86"', start)
    # Only this /MT, symbol-free, non-component profile needs no CRT/debugger/DIA staging.
    replacement = '''# Otter static-CRT profile; no runtime/debugger/DIA DLL staging.
assert(!is_debug && !is_component_build && symbol_level == 0)
assert(target_cpu == "x64" && target_os == "win")
assert(is_clang && !use_custom_libcxx && use_lld)

'''
    result[path] = text[:start] + replacement + text[end:]
    result["build/config/gclient_args.gni"] = '''# Explicit standalone checkout overrides, not upstream defaults.
android_ndk_version = "2@30.0.14608247"
checkout_angle_cl_deps = false
checkout_angle_dawn_deps = false
checkout_angle_internal = false
checkout_angle_mesa = false
checkout_angle_partition_alloc = false
checkout_angle_restricted_traces = false
generate_location_tags = false
'''
    arguments = {
        "target_os": "win", "target_cpu": "x64", "is_debug": False, "is_component_build": False,
        "is_clang": True, "use_lld": True, "use_custom_libcxx": False,
        "clang_use_chrome_plugins": False, "enable_rust": False, "enable_rust_cxx": False,
        "use_siso": False, "symbol_level": 0, "angle_build_tests": False,
        "build_angle_deqp_tests": False, "angle_enable_d3d11": True, "angle_enable_d3d9": False,
        "angle_enable_gl": False, "angle_enable_vulkan": False, "angle_enable_null": False,
        "angle_enable_wgpu": False, "angle_enable_cl": False,
        "visual_studio_path": config["visualStudioRoot"], "visual_studio_version": "2022",
        "windows_sdk_path": config["sdkRoot"], "windows_sdk_version": "10.0.26100.0",
    }
    result["out/otter/args.gn"] = "\n".join(
        f"{key} = {gn_string(value) if isinstance(value, str) else json.dumps(value)}"
        for key, value in arguments.items()
    ) + "\n"
    return {name: value.encode("utf-8") for name, value in result.items()}
