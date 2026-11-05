"""显式收集已审阅 ANGLE 构建的本地材料；不安装、分发 SDK 或默认激活 DLL。"""

import argparse
import json
import sys
from pathlib import Path

if __name__ == "__main__":
    sys.path.insert(0, str(Path(__file__).resolve().parent))

from angle_intake import material_output, reviewed_build
from angle_materials_evidence import FALSE_BOUNDARIES, STAGE_FILES
from angle_materials_verify import MAX_FILE, MAX_FILES, MAX_TOTAL, SCOPE, verify
from angle_notices import GAPS, selected_notices
from angle_sbom import documents
from build_inputs import inventory_file
from prepare_toolchain import write_json
from stage_inventory import Materializer
from toolchain_paths import file_hash, read_json


def own_sources():
    return {path.name: file_hash(path) for path in Path(__file__).resolve().parent.glob("angle_*.py")}


def create(stage, expected_stage, build, expected_build, output):
    context = reviewed_build(stage, expected_stage, build, expected_build)
    output = material_output(output, context)
    notices, sources = selected_notices(context["stage"]), own_sources()
    output.mkdir()
    try:
        writer = Materializer(output)
        writer.claim("materials.json")

        def budget(size):
            if type(size) is not int or not 0 <= size <= MAX_FILE or writer.total + size > MAX_TOTAL or len(writer.files) >= MAX_FILES:
                raise ValueError("Materials write budget exceeded")

        def copy(root, name, target, expected):
            inventory_file(root, name, expected)
            path = root / name
            budget(path.stat().st_size)
            with path.open("rb") as stream:
                row = writer.write(target, stream, path.stat().st_size, {})
            if row["sha256"] != expected:
                raise ValueError("Materials changed during copy")

        stage = context["stage"]
        for name in STAGE_FILES:
            copy(stage, name, "evidence/stage/" + name, file_hash(stage / name))
        for row in read_json(stage / "profile/profile.json")["files"]:
            name = "profile/" + row["path"]
            copy(stage, name, "evidence/stage/" + name, row["sha256"])
        for name, digest in context["evidence"].items():
            copy(context["build"], name, "evidence/build/" + name, digest)
        for row in context["receipt"]["files"]:
            copy(context["build"] / "payload", row["path"], "payload/" + row["path"], row["sha256"])
        for row in notices:
            copy(stage, row["sourcePath"], row["path"], row["sha256"])
        owner = Path(__file__).resolve().parent
        for category, root, records in (("builder", owner.parent / "native-angle", context["receipt"]["sourceSha256"]), ("collector", owner, sources)):
            for name, digest in records.items():
                copy(root, name, f"evidence/source/{category}/{name}", digest)
        copy(owner.parents[1] / "native/src", "shader_angle_api.rs", "evidence/source/shader_angle_api.rs", context["receipt"]["requiredApiSha256"])
        records = [{key: row[key] for key in ("path", "bytes", "sha256")} for row in writer.files.values()]
        records.sort(key=lambda row: row["path"])
        for name, value in documents(records, expected_build).items():
            data = (json.dumps(value, ensure_ascii=False, indent=2) + "\n").encode("utf-8")
            budget(len(data))
            row = writer.bytes(name, data, {})
            records.append({key: row[key] for key in ("path", "bytes", "sha256")})
        # 原输入和生产者源码再次核对；不是恶意并发写入者下的原子保护。
        reviewed_build(stage, expected_stage, build, expected_build)
        if sources != own_sources():
            raise ValueError("Materials producer sources changed")
        manifest = {"schemaVersion": 1, "kind": "otter-angle-local-materials", "scope": SCOPE,
            "stageSha256": expected_stage, "buildSha256": expected_build, "producerSourceSha256": sources,
            "files": sorted(records, key=lambda row: row["path"]), "notices": notices, "gaps": list(GAPS),
            "unmaterializedGitlinks": context["receipt"]["unmaterializedGitlinks"],
            **{key: False for key in FALSE_BOUNDARIES}}
        write_json(output / "materials.json", manifest)
        expected = file_hash(output / "materials.json")
        verify(output, expected)
        return {"output": str(output), "materialsSha256": expected, "files": len(records), "notices": len(notices)}
    except BaseException:
        (output / "FAILED.txt").write_text("ANGLE materials failed; preserve evidence and retry with a new directory.\n", encoding="utf-8")
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    make = commands.add_parser("create")
    for name in ("stage", "expected-stage-sha256", "build", "expected-build-sha256", "output"):
        make.add_argument("--" + name, required=True)
    check = commands.add_parser("verify")
    check.add_argument("--output", required=True)
    check.add_argument("--expected-materials-sha256", required=True)
    args = parser.parse_args()
    try:
        if args.command == "create":
            result = create(args.stage, args.expected_stage_sha256, args.build, args.expected_build_sha256, args.output)
        else:
            manifest = verify(args.output, args.expected_materials_sha256)
            result = {"materialsSha256": args.expected_materials_sha256, "files": len(manifest["files"]), "scope": manifest["scope"]}
        print(json.dumps(result, ensure_ascii=False))
    except (OSError, ValueError, KeyError, TypeError, AttributeError) as error:
        print(f"angle-materials: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
