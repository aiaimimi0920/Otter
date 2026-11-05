"""材料合同的合成成功回执；只模拟 PE 门，任何进程启动均由测试 tripwire 拒绝。"""

import sys
from contextlib import contextmanager
from pathlib import Path
from unittest.mock import patch

from native_angle_build_fixture import build_fixture, save

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "native-candidate"))

import angle_materials
from build_fixed import SCRIPTS
from build_inputs import own_sources
from toolchain_paths import file_hash, read_json


def materials_fixture():
    root, config, stage, _ = build_fixture()
    rows = read_json(stage / "files.json")
    for name, data in (("src/LICENSE", b"ANGLE license fixture\n"),
        ("src/third_party/zlib/LICENSE", b"zlib license fixture\n"),
        ("src/src/common/base/anglebase/sha1.cc", b"// Chromium inline attribution fixture\nint fixture;\n")):
        path = stage / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        rows.append({"path": name, "bytes": len(data), "sha256": file_hash(path), "origin": {"fixture": True}})
    save(stage / "files.json", rows)
    staged = read_json(stage / "stage.json")
    staged.update({"filesSha256": file_hash(stage / "files.json"), "fileCount": len(rows)})
    save(stage / "stage.json", staged)
    expected_stage = file_hash(stage / "stage.json")
    build = root / "build"
    (build / "payload").mkdir(parents=True)
    dlls = []
    for name in ("libEGL.dll", "libGLESv2.dll"):
        path = build / "payload" / name
        path.write_bytes(b"synthetic PE bytes; never load\n")
        (stage / "src/out/otter" / name).write_bytes(path.read_bytes())
        dlls.append(fake_dll(path, None, None))
    commands = []
    for name, data in (("gn", "generated"), ("plan", "clang-cl.exe /MT /WX\nlld-link.exe\n"),
                       ("compile", "built"), ("no-work", "ninja: no work to do.")):
        record = {"name": name, "status": "exited", "exitCode": 0}
        for channel, text in (("stdout", data), ("stderr", "")):
            path = build / f"{name}.{channel}.log"
            path.write_text(text, encoding="utf-8")
            record[channel + "Sha256"] = file_hash(path)
        commands.append(record)
    save(build / "gn-trace.json", {"traceEvents": [{"name": name, "cat": "script_exec"} for name in SCRIPTS]})
    api = Path(__file__).resolve().parents[2] / "native/src/shader_angle_api.rs"
    profile = read_json(stage / "profile/profile.json")
    receipt = {"schemaVersion": 1, "kind": "otter-angle-fixed-dll-build", "profile": staged["profile"],
        "stageSha256": expected_stage, "sourceSha256": own_sources(), "requiredApiSha256": file_hash(api),
        "python": profile["python"], "environmentSha256": file_hash(stage / "profile/environment.json"),
        "commands": commands, "gnTraceSha256": file_hash(build / "gn-trace.json"), "gnScripts": sorted(SCRIPTS),
        "compileCommands": 1, "diagnosticLines": 0, "files": dlls, "unmaterializedGitlinks": 0,
        "buildToolsExecuted": True, "generatedNinjaPatched": False,
        **{key: False for key in ("completeSourceClosure", "completeToolchainClosure", "runtimeVerified", "sdkInstalled", "runtimeActivated", "releaseApproved")}}
    save(build / "build.json", receipt)
    return root, stage, expected_stage, build, file_hash(build / "build.json")


def fake_dll(path, required, system):
    return {"path": path.name, "bytes": path.stat().st_size, "sha256": file_hash(path), "syntheticNotPE": True}


@contextmanager
def no_tools():
    with patch("subprocess.Popen", side_effect=AssertionError("Materials must never execute tools")), \
         patch("angle_intake.verify_dll", side_effect=fake_dll):
        yield
