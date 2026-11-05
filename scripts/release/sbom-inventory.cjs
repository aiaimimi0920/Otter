const fs = require("node:fs");
const path = require("node:path");
const { resolvedSources } = require("./third-party-sources.cjs");

function npmInventory(lock) {
  if (![2, 3].includes(lock.lockfileVersion) || !lock.packages) throw new Error("Unsupported npm lockfile");
  return Object.entries(lock.packages).flatMap(([location, entry]) => {
    if (!location || entry.link) return [];
    const inferred = location.match(/(?:^|\/)node_modules\/((?:@[^/]+\/)?[^/]+)$/)?.[1];
    const name = entry.name || inferred;
    if (typeof name !== "string" || !name || typeof entry.version !== "string" || !entry.version) {
      throw new Error(`Unresolved npm package identity: ${location}`);
    }
    return [{ ecosystem: "npm", name, version: entry.version }];
  });
}

function purl({ ecosystem, name, version, sourceSha256 }) {
  const encodedName = name.split("/").map(encodeURIComponent).join("/");
  const qualifier = sourceSha256 ? `?checksum=sha256%3A${sourceSha256}` : "";
  return `pkg:${ecosystem}/${encodedName}@${encodeURIComponent(version)}${qualifier}`;
}

function lockInventory(root) {
  const sources = resolvedSources(root);
  const resolved = new Set();
  const packages = npmInventory(JSON.parse(fs.readFileSync(path.join(root, "package-lock.json"), "utf8")));
  for (const file of ["native/Cargo.lock", "src-tauri/Cargo.lock", "src-tauri/crates/drag/Cargo.lock", "src-tauri/crates/scap-direct3d/Cargo.lock"]) {
    const text = fs.readFileSync(path.join(root, file), "utf8");
    const records = text.split(/^\[\[package\]\]\s*$/m).slice(1).map(block => {
      const record = block.match(/^\s*name\s*=\s*"([^"]+)"\s+version\s*=\s*"([^"]+)"([\s\S]*)$/);
      if (!record) throw new Error(`Malformed Cargo package: ${file}`);
      return record;
    });
    if (!records.length) throw new Error(`Empty Cargo inventory: ${file}`);
    packages.push(...records.map(([, name, version, body]) => {
      const source = file === "native/Cargo.lock" && sources.find(s => s.name === name && s.version === version);
      if (source) {
        if (/^source\s*=/m.test(body)) throw new Error(`Patched dependency resolved upstream: ${name}`);
        resolved.add(name);
      }
      return { ecosystem: "cargo", name, version, ...(source ? { sourceSha256: source.sourceSha256 } : {}) };
    }));
  }
  if (resolved.size !== sources.length) throw new Error("Missing patched dependency in native lockfile");
  return [...new Map(packages.map(p => [purl(p), { ...p, purl: purl(p) }])).values()];
}

function verifyInventory(expected, cyclone, spdx) {
  const expectedMap = new Map(expected.map(p => [p.purl, p]));
  const documents = [
    ["CycloneDX", cyclone.components?.map(p => ({ name: p.name, version: p.version, purl: p.purl }))],
    ["SPDX", spdx.packages?.map(p => {
      const refs = p.externalRefs?.filter(r => r.referenceType === "purl") ?? [];
      if (refs.length !== 1) throw new Error("SPDX package requires one purl");
      return { name: p.name, version: p.versionInfo, purl: refs[0].referenceLocator };
    })],
  ];
  for (const [format, packages] of documents) {
    const seen = new Set();
    if (!packages || packages.length !== expectedMap.size) throw new Error(`${format} inventory count mismatch`);
    for (const item of packages) {
      const known = expectedMap.get(item.purl);
      if (!known || seen.has(item.purl) || known.name !== item.name || known.version !== item.version) {
        throw new Error(`${format} inventory mismatch: ${item.purl}`);
      }
      seen.add(item.purl);
    }
  }
}

if (require.main === module) {
  const [mode, root, cyclonePath, spdxPath] = process.argv.slice(2);
  const expected = lockInventory(root);
  if (mode === "inventory") process.stdout.write(JSON.stringify(expected));
  else if (mode === "verify") {
    verifyInventory(expected, JSON.parse(fs.readFileSync(cyclonePath, "utf8")), JSON.parse(fs.readFileSync(spdxPath, "utf8")));
    console.log(`Verified ${expected.length} lockfile package identities in both SBOMs`);
  } else throw new Error("Unknown SBOM inventory mode");
}
module.exports = { npmInventory, lockInventory, verifyInventory, purl };
