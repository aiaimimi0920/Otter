"""封存材料的证据关联；只核对数据，不执行快照源码或重新宣称原始构建已发生。"""

from pathlib import PurePosixPath

from angle_notices import notice_records
from build_inputs import JSON_LIMIT, inventory_file
from toolchain_paths import read_json

FALSE_BOUNDARIES = ("legalClearance", "runtimeVerified", "runtimeActivated", "releaseApproved", "publishable",
    "completeSourceClosure", "completeToolchainClosure", "completeNoticeCoverage", "sdkInstalled", "sdkRedistributed")
STAGE_FILES = ("stage.json", "files.json", "gitlinks.json", "profile/profile.json", "profile/adapter.json")
SBOMS = {"sbom.cdx.json", "sbom.spdx.json"}


def expected_evidence(root, manifest):
    stage, build = root / "evidence/stage", root / "evidence/build"
    inventory_file(stage, "stage.json", manifest["stageSha256"])
    inventory_file(build, "build.json", manifest["buildSha256"])
    staged = read_json(stage / "stage.json")
    receipt = read_json(build / "build.json", limit=JSON_LIMIT)
    if staged.get("kind") != "otter-angle-fixed-build-stage" or receipt.get("kind") != "otter-angle-fixed-dll-build":
        raise ValueError("Invalid preserved ANGLE receipt kinds")
    if receipt.get("stageSha256") != manifest["stageSha256"] or receipt.get("profile") != staged["profile"]:
        raise ValueError("Preserved stage/build binding mismatch")
    if receipt.get("sourceSha256") != staged["stagingSourceSha256"]:
        raise ValueError("Preserved build source mismatch")
    for value in (staged, receipt):
        for key in ("completeSourceClosure", "completeToolchainClosure", "runtimeActivated", "releaseApproved"):
            if value.get(key) is not False:
                raise ValueError("Preserved evidence falsely approves a boundary")
    if receipt.get("runtimeVerified") is not False or receipt.get("sdkInstalled") is not False or receipt.get("buildToolsExecuted") is not True or receipt.get("generatedNinjaPatched") is not False:
        raise ValueError("Invalid preserved build execution boundaries")
    hashes = {"evidence/stage/stage.json": manifest["stageSha256"], "evidence/build/build.json": manifest["buildSha256"]}
    for name, key in (("files.json", "filesSha256"), ("gitlinks.json", "gitlinksSha256"), ("profile/profile.json", "profileSha256")):
        hashes["evidence/stage/" + name] = staged[key]
    profile = read_json(stage / "profile/profile.json")
    if profile.get("profile") != staged["profile"] or profile.get("python") != receipt["python"]:
        raise ValueError("Preserved profile/Python mismatch")
    hashes["evidence/stage/profile/adapter.json"] = profile["adapterSha256"]
    for row in profile["files"]:
        name = "evidence/stage/profile/" + row["path"]
        if name in hashes:
            raise ValueError("Duplicate preserved profile path")
        hashes[name] = row["sha256"]
    if hashes.get("evidence/stage/profile/environment.json") != receipt["environmentSha256"]:
        raise ValueError("Preserved environment binding mismatch")
    hashes["evidence/build/gn-trace.json"] = receipt["gnTraceSha256"]
    commands = receipt["commands"]
    if [row["name"] for row in commands] != ["gn", "plan", "compile", "no-work"]:
        raise ValueError("Incomplete preserved command evidence")
    for row in commands:
        if row.get("status") != "exited" or type(row.get("exitCode")) is not int or row["exitCode"] != 0:
            raise ValueError("Unsuccessful preserved command evidence")
        for channel in ("stdout", "stderr"):
            hashes[f"evidence/build/{row['name']}.{channel}.log"] = row[channel + "Sha256"]
    if [row["path"] for row in receipt["files"]] != ["libEGL.dll", "libGLESv2.dll"]:
        raise ValueError("Only two reviewed ANGLE DLLs are allowed")
    for row in receipt["files"]:
        hashes["payload/" + row["path"]] = row["sha256"]
        if (root / "payload" / row["path"]).stat().st_size != row["bytes"]:
            raise ValueError("Preserved payload size mismatch")
    for category, sources in (("builder", receipt["sourceSha256"]), ("collector", manifest["producerSourceSha256"])):
        if not isinstance(sources, dict) or not 1 <= len(sources) <= 64:
            raise ValueError("Invalid preserved source inventory")
        for name, digest in sources.items():
            if PurePosixPath(name).name != name or not name.endswith(".py"):
                raise ValueError("Invalid preserved source filename")
            hashes[f"evidence/source/{category}/{name}"] = digest
    hashes["evidence/source/shader_angle_api.rs"] = receipt["requiredApiSha256"]
    notices = notice_records(read_json(stage / "files.json", limit=JSON_LIMIT))
    if manifest["notices"] != notices:
        raise ValueError("Notice selection differs from preserved stage inventory")
    for row in notices:
        hashes[row["path"]] = row["sha256"]
    if manifest["unmaterializedGitlinks"] != staged["unmaterializedGitlinks"] or receipt["unmaterializedGitlinks"] != staged["unmaterializedGitlinks"]:
        raise ValueError("Preserved source omissions mismatch")
    return hashes
