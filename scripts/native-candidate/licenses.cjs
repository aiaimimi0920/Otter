// Collect actual license text for non-dev dependencies in the resolved graph; complete materials do not imply legal or security approval.
const fs = require('node:fs');
const path = require('node:path');
const { hash } = require('./source.cjs');
const { checkedPath } = require('../release/third-party-sources.cjs');
const { readBytes } = require('./input-bytes.cjs');
const { javascriptLicenses } = require('./javascript_licenses.cjs');
const { fluentLicenses } = require('./fluent_licenses.cjs');
const design = require('./design-source.cjs');
const rxing = require('./rxing-source.cjs');
const licenseName = /^(licen[sc]es?|copying|copyright|notice)([._-]|$)/i;
const safeName = value => typeof value === 'string' && /^[A-Za-z0-9][A-Za-z0-9_.+-]*$/.test(value);

function selectedPackages(metadata) {
  const packages = new Map(metadata.packages.map(p => [p.id, p]));
  const nodes = new Map(metadata.resolve.nodes.map(n => [n.id, n]));
  if (packages.size !== metadata.packages.length || nodes.size !== metadata.resolve.nodes.length ||
      packages.get(metadata.resolve.root)?.name !== 'otter-native') throw new Error('Invalid native license graph');
  const selected = new Set();
  const pending = [metadata.resolve.root];
  while (pending.length) {
    const id = pending.pop();
    if (selected.has(id)) continue;
    if (!packages.has(id) || !nodes.has(id)) throw new Error('Incomplete native license graph');
    selected.add(id);
    for (const dependency of nodes.get(id).deps) {
      if (!Array.isArray(dependency.dep_kinds) || !dependency.dep_kinds.length ||
          dependency.dep_kinds.some(k => ![null, 'normal', 'build', 'dev'].includes(k.kind))) {
        throw new Error('Invalid native dependency kind');
      }
      // Conservatively include normal/build edges for all platforms, without claiming an exact linked-component set.
      if (dependency.dep_kinds.some(k => k.kind !== 'dev')) pending.push(dependency.pkg);
    }
  }
  return [...selected].map(id => packages.get(id)).sort((a, b) => a.id.localeCompare(b.id));
}

function licenseFiles(directory) {
  const names = [];
  function visit(relative, depth) {
    if (depth > 8 || names.length > 2048) throw new Error('Oversized license tree');
    const file = checkedPath(directory, relative);
    const stat = fs.statSync(file);
    if (stat.isDirectory()) {
      for (const name of fs.readdirSync(file).sort()) visit(`${relative}/${name}`, depth + 1);
    } else if (stat.isFile()) names.push(relative);
    else throw new Error('Unsupported license source');
  }
  for (const name of fs.readdirSync(directory).sort().filter(name => licenseName.test(name))) visit(name, 0);
  return names;
}

function collect(repo, metadata, output, registryRoot) {
  if (fs.existsSync(output)) throw new Error('License output already exists');
  const sources = require('../release/third-party-sources.cjs').verifiedSources(repo);
  require('../release/third-party-sources.cjs').verifyResolution(repo, sources, metadata);
  rxing.selectedPackage(metadata);
  const records = [];
  let total = 0;
  const keys = new Set();
  // Validate/read every input first; failure creates no partial output directory.
  for (const pkg of selectedPackages(metadata)) {
    if (!safeName(pkg.name) || !safeName(pkg.version) || !pkg.license || /UNLICENSED|PROPRIETARY|NOASSERTION/i.test(pkg.license)) {
      throw new Error('Unresolved native license identity');
    }
    const key = `${pkg.name}-${pkg.version}`;
    if (keys.has(key.toLowerCase())) throw new Error('Duplicate native license package');
    keys.add(key.toLowerCase());
    let directory = path.dirname(pkg.manifest_path);
    if (pkg.id === metadata.resolve.root) {
      if (fs.realpathSync(pkg.manifest_path) !== fs.realpathSync(path.join(repo, 'native/Cargo.toml'))) throw new Error('Invalid project license root');
      directory = repo;
    } else if (pkg.name === design.NAME) {
      directory = design.directory(repo, pkg);
    } else if (pkg.name === rxing.NAME) {
      directory = rxing.directory(repo, pkg);
    } else if (pkg.source == null) {
      const workspaceDirs = {
        'otter-live-hover-hook': 'native/hover-hook/Cargo.toml',
        'otter-core': 'native/otter-core/Cargo.toml',
        'otter-media': 'native/otter-media/Cargo.toml',
        'otter-runtime': 'native/otter-runtime/Cargo.toml',
      };
      if (workspaceDirs[pkg.name]) {
        if (fs.realpathSync(pkg.manifest_path) !== fs.realpathSync(path.join(repo, workspaceDirs[pkg.name]))) {
          throw new Error(`Invalid workspace license root: ${pkg.name}`);
        }
        directory = repo;
      } else {
        const local = sources.find(s => s.name === pkg.name && s.version === pkg.version);
        if (!local || fs.realpathSync(directory) !== fs.realpathSync(path.join(repo, local.directory))) throw new Error('Unknown local license source');
      }
    } else {
      if (!pkg.source.startsWith('registry+')) throw new Error('Unsupported registry license source');
      const relative = path.relative(registryRoot, directory).split(path.sep).join('/');
      directory = checkedPath(registryRoot, relative);
    }
    const names = licenseFiles(directory);
    if (pkg.license_file && pkg.id !== metadata.resolve.root && !names.includes(pkg.license_file)) names.push(pkg.license_file);
    const files = names.sort().map(name => {
      const file = checkedPath(directory, name);
      const bytes = readBytes(file, Math.min(4 * 1024 * 1024, 64 * 1024 * 1024 - total), 1);
      total += bytes.length;
      new TextDecoder('utf-8', { fatal: true }).decode(bytes);
      return { path: `${key}/${name}`, bytes, sha256: hash(bytes) };
    });
    // Supplements admit only reviewed exact identities and original-text digests; missing licenses still fail.
    for (const file of [...javascriptLicenses(pkg, metadata, registryRoot), ...fluentLicenses(pkg, registryRoot)]) {
      const destination = `${key}/${file.name}`;
      if (files.some(f => f.path === destination) || (total += file.bytes.length) > 64 * 1024 * 1024) {
        throw new Error('Duplicate or oversized supplemental license material');
      }
      files.push({ path: destination, bytes: file.bytes, sha256: file.sha256, upstream: file.upstream });
    }
    if (!files.length) throw new Error(`Missing license texts: ${key}`);
    records.push({ id: pkg.id, name: pkg.name, version: pkg.version, expression: pkg.license, files });
  }
  fs.mkdirSync(output);
  for (const record of records) for (const file of record.files) {
    const destination = path.join(output, file.path);
    fs.mkdirSync(path.dirname(destination), { recursive: true });
    fs.writeFileSync(destination, file.bytes, { flag: 'wx' });
  }
  const inventory = { schemaVersion: 1, kind: 'otter-native-license-materials', scope: 'all-platform-normal-and-build-closure',
    metadataSha256: hash(JSON.stringify(metadata)), patchedSources: sources,
    legalClearance: false, packages: records.map(p => ({ ...p, files: p.files.map(f => ({
      path: f.path, bytes: f.bytes.length, sha256: f.sha256, ...(f.upstream ? { upstream: f.upstream } : {}),
    })) })) };
  fs.writeFileSync(path.join(output, 'inventory.json'), JSON.stringify(inventory, null, 2) + '\n', { flag: 'wx' });
  return inventory;
}
if (require.main === module) {
  const [repo, metadataFile, output] = process.argv.slice(2);
  const cargoHome = process.env.CARGO_HOME || path.join(process.env.USERPROFILE, '.cargo');
  const result = collect(repo, JSON.parse(readBytes(metadataFile, 16 * 1024 * 1024).toString('utf8')), output, path.join(cargoHome, 'registry/src'));
  console.log(JSON.stringify({ packages: result.packages.length, output, legalClearance: false }));
}
module.exports = { selectedPackages, licenseFiles, collect };
