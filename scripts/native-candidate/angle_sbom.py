"""为本地材料集合生成标准文件清单；不把 source 存在推断成 DLL 的完整依赖闭包。"""

import datetime
import re
import uuid


def documents(records, identity, *, stamp=None, serial=None):
    stamp = stamp or datetime.datetime.now(datetime.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")
    serial = serial or "urn:uuid:" + str(uuid.uuid4())
    if not re.fullmatch(r"\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ", stamp) or not re.fullmatch(r"urn:uuid:[0-9a-f-]{36}", serial):
        raise ValueError("Invalid SBOM timestamp or serial")
    datetime.datetime.strptime(stamp, "%Y-%m-%dT%H:%M:%SZ")
    uuid.UUID(serial[9:])
    description = "Local ANGLE materials inventory excluding this manifest and both SBOMs; NOT a complete DLL dependency SBOM or legal/release approval."
    cdx = {"bomFormat": "CycloneDX", "specVersion": "1.6", "version": 1, "serialNumber": serial,
        "metadata": {"timestamp": stamp, "component": {"type": "data", "name": "Otter ANGLE materials",
            "version": identity, "description": description},
            "properties": [{"name": "otter:scope", "value": "materials-file-inventory-not-runtime-dependency-closure"},
                           {"name": "otter:legalClearance", "value": "false"}]},
        "components": [{"type": "file", "bom-ref": "file:" + row["path"], "name": row["path"],
                        "hashes": [{"alg": "SHA-256", "content": row["sha256"]}]} for row in records]}
    spdx = {"spdxVersion": "SPDX-2.3", "dataLicense": "CC0-1.0", "SPDXID": "SPDXRef-DOCUMENT",
        "name": "Otter ANGLE materials file inventory",
        "documentNamespace": "https://otter.invalid/spdx/angle-materials/" + serial[9:],
        "creationInfo": {"created": stamp, "creators": ["Tool: Otter-angle-materials"]},
        "comment": description,
        "files": [{"fileName": "./" + row["path"], "SPDXID": f"SPDXRef-File-{index}",
                   "checksums": [{"algorithm": "SHA256", "checksumValue": row["sha256"]}],
                   "licenseConcluded": "NOASSERTION", "licenseInfoInFiles": ["NOASSERTION"], "copyrightText": "NOASSERTION"}
                  for index, row in enumerate(records)],
        "relationships": [{"spdxElementId": "SPDXRef-DOCUMENT", "relationshipType": "DESCRIBES",
                           "relatedSpdxElement": f"SPDXRef-File-{index}"} for index in range(len(records))]}
    return {"sbom.cdx.json": cdx, "sbom.spdx.json": spdx}
