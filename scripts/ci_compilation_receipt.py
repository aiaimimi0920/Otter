"""Record compiled bytes without claiming tests, packaging or release approval."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess


BUILD_COMMAND = "cargo build --locked --release --workspace --manifest-path native/Cargo.toml"
OUTPUTS = ("native/target/release/otter-native.exe",
           "native/target/release/otter_live_hover_hook.dll")


def git(root, *args):
    return subprocess.check_output(
        ["git", "--no-optional-locks", *args], cwd=root,
        stderr=subprocess.PIPE, text=True, encoding="utf-8").strip()


def file_evidence(root, relative):
    path = root / relative
    if not path.is_file() or path.is_symlink():
        raise ValueError(f"Missing regular compilation output: {relative}")
    if not path.resolve().is_relative_to(root.resolve()):
        raise ValueError(f"Compilation output escapes the repository: {relative}")
    digest = hashlib.sha256()
    size = 0
    with path.open("rb") as stream:
        while chunk := stream.read(1024 * 1024):
            digest.update(chunk)
            size += len(chunk)
    if size == 0:
        raise ValueError(f"Empty compilation output: {relative}")
    return {"path": relative, "sizeBytes": size, "sha256": digest.hexdigest()}


def receipt(root, expected_sha):
    if not re.fullmatch(r"[0-9a-f]{40}", expected_sha):
        raise ValueError("Expected an exact GitHub commit SHA")
    head = git(root, "rev-parse", "HEAD")
    if head != expected_sha:
        raise ValueError("HEAD does not match the GitHub commit SHA")
    # Compilation evidence is intentionally weaker than candidate provenance.
    # Still reject tracked inputs changed after the clean CI checkout.
    git(root, "diff", "--exit-code", "HEAD", "--")
    return {
        "schemaVersion": 1,
        "compilationOnly": True,
        "testsVerified": False,
        "packageVerified": False,
        "runtimeVerified": False,
        "formalReleaseApproved": False,
        "sourceCommit": head,
        "githubSha": expected_sha,
        "buildCommand": BUILD_COMMAND,
        "cargoLockSha256": file_evidence(root, "native/Cargo.lock")["sha256"],
        "files": [file_evidence(root, relative) for relative in OUTPUTS],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--expected-sha", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    evidence = receipt(root, args.expected_sha)
    # Do not overwrite evidence from another attempt or write partial success.
    with args.output.open("x", encoding="utf-8", newline="\n") as stream:
        stream.write(json.dumps(evidence, indent=2) + "\n")


if __name__ == "__main__":
    main()
