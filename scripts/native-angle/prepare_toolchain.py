"""Generate pinned-upstream toolchain-adapter fragments in fresh out-of-repository directories; run no upstream or packaged tools."""

import argparse
import hashlib
import json
import sys
from pathlib import Path

from otter_toolchain_env import create_manifest
from toolchain_paths import checked_path, file_hash, read_json

BUILD_REVISION = "8a46ede8ea86151737d9c284f82d238b3c701fa7"
SETUP_SHA256 = "f6ae24c99ae0bfc72b093da415615119652f25415155dffb4db22f49b78ca8cf"
GN_HELPERS_SHA256 = "4c99d6d159f0faef3b83ac594ea0cf4d30973c029179721f3525cf87d765f082"
SETUP_PATH = "toolchain/win/setup_toolchain.py"
MODULES = ("toolchain_paths.py", "otter_toolchain_env.py")


def adapted_setup(original):
    if hashlib.sha256(original).hexdigest() != SETUP_SHA256:
        raise ValueError("Pinned upstream setup_toolchain.py identity mismatch")
    source = original.decode("utf-8")
    start = source.index("def _LoadToolchainEnv(")
    end = source.index("def _FormatAsEnvironmentBlock(", start)
    replacement = '''def _LoadToolchainEnv(cpu, toolchain_root, sdk_dir, target_store):
  # Otter adaptation: explicit data only; never call vcvarsall/SetEnv/discovery.
  from otter_toolchain_env import load_environment
  return load_environment(os.environ.get('OTTER_ANGLE_TOOLCHAIN_MANIFEST'),
                          cpu, toolchain_root, sdk_dir, target_store)


'''
    source = source[:start] + replacement + source[end:]
    # GN/Windows may lose empty positional arguments; exact markers represent empty values only, granting no runtime paths.
    source = source.replace("  runtime_dirs = sys.argv[3]\n", """  runtime_dirs = sys.argv[3]
  if runtime_dirs == 'otter-no-runtime-dirs':
    runtime_dirs = ''
  if runtime_dirs:
    raise ValueError('External runtime_dirs are not admitted by the Otter adapter')
""", 1)
    # Even with empty runtime_dirs, original upstream concatenation leaves an empty PATH segment that searches CWD.
    source = source.replace("      env['PATH'] = runtime_dirs + os.pathsep + env['PATH']\n", "", 1)
    source = source.replace("      include_I = ['/I' + i for i in include]\n", """      # SDK packages need not use the system's Windows Kits directory spelling.
      windows_kits_dir = relflag(os.path.join(win_sdk_path, 'Include', SDK_VERSION))
      include_I = ['/I' + i for i in include]
""", 1)
    return source.encode("utf-8")


def write_json(path, value):
    with path.open("x", encoding="utf-8", newline="\n") as stream:
        json.dump(value, stream, ensure_ascii=False, indent=2)
        stream.write("\n")


def prepare(build_source, config_file, output):
    source = checked_path(str(build_source))
    setup = checked_path(str(source / SETUP_PATH), directory=False)
    helpers = checked_path(str(source / "gn_helpers.py"), directory=False)
    if file_hash(setup) != SETUP_SHA256:
        raise ValueError("Pinned upstream setup_toolchain.py identity mismatch")
    if file_hash(helpers) != GN_HELPERS_SHA256:
        raise ValueError("Pinned upstream gn_helpers.py identity mismatch")
    patched = adapted_setup(setup.read_bytes())
    own_directory = Path(__file__).parent
    own_sources = {name: file_hash(own_directory / name) for name in (*MODULES, "prepare_toolchain.py")}
    config_sha256 = file_hash(config_file)
    config = read_json(config_file)
    manifest = create_manifest(config)
    output = Path(output)
    # Permit only new names under existing link-free parents; neither create parents nor clean failed directories.
    parent = checked_path(str(output.parent))
    if not output.is_absolute() or output.name in ("", ".", ".."):
        raise ValueError("Absolute new output directory required")
    repo = Path(__file__).resolve().parents[2]
    for protected in (repo, source, *(Path(config[key]) for key in (
        "visualStudioRoot", "sdkRoot", "systemRoot", "tempRoot"
    ))):
        if output.is_relative_to(protected) or protected.is_relative_to(output):
            raise ValueError("Output overlaps a protected input tree")
    # Check nonexistent final components by the same rules, preventing Windows path aliases from bypassing exclusive mkdir.
    checked_path(str(parent / output.name)) if output.exists() else checked_path(str(parent))
    if any(char in output.name for char in ';"%!?&|<>$`:\\/') or output.name.endswith((".", " ")):
        raise ValueError("Unsafe output directory name")
    output.mkdir()
    try:
        destination = output / "build/toolchain/win"
        destination.mkdir(parents=True)
        (destination / "setup_toolchain.py").write_bytes(patched)
        for name in MODULES:
            (destination / name).write_bytes((own_directory / name).read_bytes())
        (output / "build/gn_helpers.py").write_bytes(helpers.read_bytes())
        write_json(output / "environment.json", manifest)
        payloads = [output / "environment.json", output / "build/gn_helpers.py", *sorted(destination.iterdir())]
        if file_hash(config_file) != config_sha256 or create_manifest(config) != manifest:
            raise ValueError("Toolchain inputs changed during preparation")
        if file_hash(setup) != SETUP_SHA256 or file_hash(output / "build/gn_helpers.py") != GN_HELPERS_SHA256:
            raise ValueError("Upstream inputs changed during preparation")
        if own_sources != {name: file_hash(own_directory / name) for name in own_sources}:
            raise ValueError("Adapter sources changed during preparation")
        receipt = {
            "schemaVersion": 1, "kind": "otter-angle-toolchain-adapter",
            "upstreamBuildRevision": BUILD_REVISION,
            "upstreamSetupSha256": SETUP_SHA256, "upstreamGnHelpersSha256": GN_HELPERS_SHA256,
            "configSha256": config_sha256,
            "adapterSourceSha256": own_sources,
            "files": [{"path": path.relative_to(output).as_posix(), "sha256": file_hash(path)} for path in payloads],
            "toolsExecuted": False, "completeToolchainClosure": False,
            "sdkInstalled": False, "releaseApproved": False,
        }
        write_json(output / "adapter.json", receipt)
        return receipt
    except BaseException:
        (output / "FAILED.txt").write_text("Adapter preparation failed; preserve this directory.\n", encoding="utf-8")
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build-source", required=True)
    parser.add_argument("--config", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    try:
        receipt = prepare(args.build_source, args.config, args.output)
        print(json.dumps(receipt, ensure_ascii=False))
    except (OSError, ValueError) as error:
        print(f"angle-toolchain: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
