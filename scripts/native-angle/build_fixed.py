"""显式执行已审核暂存树的固定 GN/Ninja DLL 构建；不安装、不分发或启用能力。"""

import argparse
import json
import os
import re
import shutil
import sys
from pathlib import Path

if __name__ == "__main__":
    sys.path.insert(0, str(Path(__file__).resolve().parent))

from build_binary import verify_dll
from build_inputs import JSON_LIMIT, new_output, own_sources, verify_stage
from build_job import run_job, stage_lock
from otter_toolchain_env import load_environment
from prepare_toolchain import write_json
from stage_pins import SOURCES
from toolchain_paths import checked_path, file_hash, read_json

SCRIPTS = {"//build/" + name for name in ("toolchain/get_concurrent_links.py", "compute_build_timestamp.py",
    "toolchain/win/setup_toolchain.py", "win/use_ansi_codes.py", "toolchain/get_cpu_count.py")}
MAX_LOG = 16 * 1024 * 1024


class Commands:
    def __init__(self, output, environment):
        self.output, self.environment, self.records = output, environment, []

    def run(self, name, arguments, cwd, timeout):
        record = {"name": name, "arguments": arguments, "cwd": str(cwd)}
        try:
            with (self.output / f"{name}.stdout.log").open("xb") as stdout, (self.output / f"{name}.stderr.log").open("xb") as stderr:
                record.update(run_job(arguments, cwd, self.environment, stdout, stderr, timeout=timeout, max_log=MAX_LOG))
        except BaseException as error:
            record.update({"status": "exception", "errorType": type(error).__name__})
            raise
        finally:
            for channel in ("stdout", "stderr"):
                path = self.output / f"{name}.{channel}.log"
                if path.exists():
                    record[channel + "Sha256"] = file_hash(path)
            write_json(self.output / (name + ".json"), record)
            self.records.append(record)
        if record["status"] != "exited" or record["exitCode"] != 0:
            raise ValueError(f"Build command failed: {name} ({record['status']}, {record['exitCode']})")
        path = self.output / f"{name}.stdout.log"
        if any((self.output / f"{name}.{channel}.log").stat().st_size > MAX_LOG for channel in ("stdout", "stderr")):
            raise ValueError("Build command output limit exceeded")
        return path.read_text("utf-8")


def check_plan(plan):
    compiles = [line for line in plan.splitlines() if "clang-cl.exe" in line]
    if not compiles or not all(re.search(r"(?:^|\s)/MT(?:\s|$)", line) and "/WX" in line for line in compiles):
        raise ValueError("Static CRT and warnings-as-errors compile plan required")
    if re.search(r"(?:^|\s)/MDd?(?:\s|$)", plan) or "vcvarsall" in plan or "vs_toolchain.py" in plan or "lld-link.exe" not in plan:
        raise ValueError("Unexpected toolchain in build plan")
    return len(compiles)


def execute(stage, expected, output):
    stage = checked_path(str(stage))
    with stage_lock(stage):
        receipt, profile = verify_stage(stage, expected)
        manifest = stage / "profile/environment.json"
        config = read_json(manifest)["config"]
        env = load_environment(str(manifest), "x64", config["visualStudioRoot"], config["sdkRoot"], False)
        env.update({"DEPOT_TOOLS_WIN_TOOLCHAIN": "0", "OTTER_ANGLE_TOOLCHAIN_MANIFEST": str(manifest),
            "PYTHONDONTWRITEBYTECODE": "1", "PYTHONUTF8": "1", "ANGLE_UPSTREAM_HASH": SOURCES[0][2][:12]})
        env["PATH"] = str(stage / "tools") + os.pathsep + env["PATH"]
        output = new_output(output, stage, config)
        sources = own_sources()
        api = Path(__file__).resolve().parents[2] / "native/src/shader_angle_api.rs"
        api_hash = file_hash(api)
        output.mkdir()
        try:
            commands = Commands(output, env)
            src, build = stage / "src", stage / "src/out/otter"
            gn, ninja = str(stage / "tools/gn.exe"), str(stage / "tools/ninja.exe")
            commands.run("gn", [gn, "gen", "out/otter", "--fail-on-unused-args", "--tracelog=" + str(output / "gn-trace.json")], src, 180)
            trace = read_json(output / "gn-trace.json", limit=JSON_LIMIT)
            scripts = {row["name"] for row in trace["traceEvents"] if row.get("cat") == "script_exec"}
            if scripts != SCRIPTS:
                raise ValueError("Unexpected executed GN scripts")
            plan = commands.run("plan", [ninja, "-t", "commands", "libEGL", "libGLESv2"], build, 60)
            compile_count = check_plan(plan)
            commands.run("compile", [ninja, "-j2", "libEGL", "libGLESv2"], build, 1800)
            if "no work to do" not in commands.run("no-work", [ninja, "-n", "libEGL", "libGLESv2"], build, 60):
                raise ValueError("DLL build remains out of date")
            diagnostics = []
            for channel in ("stdout", "stderr"):
                for line in (output / f"compile.{channel}.log").read_text("utf-8").splitlines():
                    if re.search(r"(?i)\b(?:warning|error)(?:\s+[A-Z]+\d+)?\s*:|D9002|警告|错误|FAILED:", line):
                        diagnostics.append(line)
            if diagnostics:
                raise ValueError("Compiler warning/error diagnostics require review")
            payload = output / "payload"
            payload.mkdir()
            dlls = []
            for name, prefix in (("libEGL.dll", "egl"), ("libGLESv2.dll", "gl")):
                required = re.findall(r'"(' + prefix + r'[A-Z][A-Za-z0-9_]*)"', api.read_text("utf-8"))
                record = verify_dll(build / name, required, config["systemRoot"])
                with (build / name).open("rb") as source, (payload / name).open("xb") as target:
                    shutil.copyfileobj(source, target, length=1024 * 1024)
                if file_hash(payload / name) != record["sha256"]:
                    raise ValueError("DLL copy identity changed")
                dlls.append(record)
            verify_stage(stage, expected, built=True)
            load_environment(str(manifest), "x64", config["visualStudioRoot"], config["sdkRoot"], False)
            if sources != own_sources() or api_hash != file_hash(api):
                raise ValueError("Build owner/API sources changed during execution")
            result = {"schemaVersion": 1, "kind": "otter-angle-fixed-dll-build", "profile": receipt["profile"],
                "stageSha256": expected, "sourceSha256": sources, "requiredApiSha256": api_hash,
                "python": profile["python"], "environmentSha256": file_hash(manifest),
                "environment": env, "gnScripts": sorted(scripts), "gnTraceSha256": file_hash(output / "gn-trace.json"),
                "commands": commands.records, "compileCommands": compile_count, "files": dlls,
                "buildToolsExecuted": True, "generatedNinjaPatched": False, "diagnosticLines": 0,
                "unmaterializedGitlinks": receipt["unmaterializedGitlinks"], "completeSourceClosure": False,
                "completeToolchainClosure": False, "runtimeVerified": False, "sdkInstalled": False,
                "runtimeActivated": False, "releaseApproved": False}
            write_json(output / "build.json", result)
            return result
        except BaseException:
            (output / "FAILED.txt").write_text("ANGLE build failed; preserve stage and evidence. Retry with new directories.\n", encoding="utf-8")
            raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--stage", required=True)
    parser.add_argument("--expected-stage-sha256", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    try:
        print(json.dumps(execute(args.stage, args.expected_stage_sha256, args.output), ensure_ascii=False))
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"angle-build: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
