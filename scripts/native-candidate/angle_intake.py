"""只读消费独立审阅的 ANGLE stage/build；复用构建 owner，不执行任何构建工具。"""

import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "native-angle"))

from build_binary import verify_dll
from build_fixed import MAX_LOG, SCRIPTS, check_plan
from build_inputs import JSON_LIMIT, actual_files, inventory_file, new_output, own_sources, verify_stage
from otter_toolchain_env import create_manifest
from toolchain_paths import checked_path, file_hash, read_json


def reviewed_build(stage, stage_sha256, build, build_sha256):
    stage, build = checked_path(str(stage)), checked_path(str(build))
    inventory_file(build, "build.json", build_sha256)
    if any((build / name).exists() for name in ("FAILED.txt", "ABORTED.json")):
        raise ValueError("Failed or aborted ANGLE build cannot become candidate materials")
    staged, profile = verify_stage(stage, stage_sha256, built=True)
    receipt = read_json(build / "build.json", limit=JSON_LIMIT)
    if type(receipt.get("schemaVersion")) is not int or receipt["schemaVersion"] != 1 or receipt.get("kind") != "otter-angle-fixed-dll-build":
        raise ValueError("Expected fixed ANGLE DLL build receipt")
    if receipt.get("stageSha256") != stage_sha256 or receipt.get("profile") != staged["profile"] or receipt.get("python") != profile["python"]:
        raise ValueError("ANGLE stage/build identity mismatch")
    if receipt.get("sourceSha256") != own_sources() or receipt.get("generatedNinjaPatched") is not False or receipt.get("buildToolsExecuted") is not True:
        raise ValueError("ANGLE build owner or execution boundary mismatch")
    for name in ("completeSourceClosure", "completeToolchainClosure", "runtimeVerified", "sdkInstalled", "runtimeActivated", "releaseApproved"):
        if receipt.get(name) is not False:
            raise ValueError("ANGLE build falsely approved boundary")
    manifest = read_json(stage / "profile/environment.json")
    config = manifest["config"]
    if create_manifest(config) != manifest or file_hash(stage / "profile/environment.json") != receipt["environmentSha256"]:
        raise ValueError("ANGLE toolchain anchors or environment changed")
    api = Path(__file__).resolve().parents[2] / "native/src/shader_angle_api.rs"
    if file_hash(api) != receipt["requiredApiSha256"]:
        raise ValueError("ANGLE required API changed")
    commands = receipt["commands"]
    if not isinstance(commands, list) or [row["name"] for row in commands] != ["gn", "plan", "compile", "no-work"]:
        raise ValueError("Incomplete ANGLE command evidence")
    evidence = {"build.json": build_sha256, "gn-trace.json": receipt["gnTraceSha256"]}
    for row in commands:
        if row.get("status") != "exited" or type(row.get("exitCode")) is not int or row["exitCode"] != 0:
            raise ValueError("Unsuccessful ANGLE command evidence")
        for channel in ("stdout", "stderr"):
            name = row["name"] + "." + channel + ".log"
            if (build / name).stat().st_size > MAX_LOG:
                raise ValueError("Oversized ANGLE command evidence")
            evidence[name] = row[channel + "Sha256"]
    for name, expected in evidence.items():
        inventory_file(build, name, expected)
    scripts = {row["name"] for row in read_json(build / "gn-trace.json", limit=JSON_LIMIT)["traceEvents"] if row.get("cat") == "script_exec"}
    if scripts != SCRIPTS or receipt.get("gnScripts") != sorted(scripts):
        raise ValueError("Unexpected ANGLE GN scripts")
    plan = (build / "plan.stdout.log").read_text("utf-8")
    if type(receipt.get("compileCommands")) is not int or check_plan(plan) != receipt["compileCommands"]:
        raise ValueError("ANGLE compile plan mismatch")
    if "no work to do" not in (build / "no-work.stdout.log").read_text("utf-8") or receipt.get("diagnosticLines") != 0:
        raise ValueError("ANGLE build remains incomplete")
    for channel in ("stdout", "stderr"):
        if re.search(r"(?im)\b(?:warning|error)(?:\s+[A-Z]+\d+)?\s*:|D9002|警告|错误|FAILED:", (build / f"compile.{channel}.log").read_text("utf-8")):
            raise ValueError("ANGLE compiler diagnostics require review")
    if actual_files(build / "payload", built=False) != {"libEGL.dll", "libGLESv2.dll"}:
        raise ValueError("ANGLE payload must contain exactly two DLLs")
    files = receipt["files"]
    if not isinstance(files, list) or [row["path"] for row in files] != ["libEGL.dll", "libGLESv2.dll"]:
        raise ValueError("Invalid ANGLE DLL inventory")
    for row, prefix in zip(files, ("egl", "gl"), strict=True):
        required = re.findall(r'"(' + prefix + r'[A-Z][A-Za-z0-9_]*)"', api.read_text("utf-8"))
        inventory_file(build / "payload", row["path"], row["sha256"], row["bytes"])
        inventory_file(stage / "src/out/otter", row["path"], row["sha256"], row["bytes"])
        if verify_dll(build / "payload" / row["path"], required, config["systemRoot"]) != row:
            raise ValueError("ANGLE PE evidence mismatch")
    if receipt["unmaterializedGitlinks"] != staged["unmaterializedGitlinks"]:
        raise ValueError("ANGLE omission inventory mismatch")
    return {"stage": stage, "build": build, "stageReceipt": staged, "receipt": receipt,
            "stageSha256": stage_sha256, "buildSha256": build_sha256, "config": config, "evidence": evidence}


def material_output(output, context):
    output = new_output(Path(output), context["stage"], context["config"])
    if output.is_relative_to(context["build"]) or context["build"].is_relative_to(output):
        raise ValueError("Materials output overlaps ANGLE build evidence")
    return output
