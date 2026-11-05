// 独立原生候选的源码身份；不读取凭证内容，不冒充可复现或已签名构建。
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { verifiedSources, verifyResolution, checkedPath } = require('../release/third-party-sources.cjs');
const hash = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const encoded = value => JSON.stringify(value);

function filesUnder(root, relative, output) {
  const directory = checkedPath(root, relative);
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const name = `${relative}/${entry.name}`;
    if (entry.isSymbolicLink()) throw new Error(`Linked native build input: ${name}`);
    if (entry.isDirectory()) filesUnder(root, name, output);
    else if (entry.isFile()) output.add(name);
    else throw new Error(`Unsupported native build input: ${name}`);
  }
}

function snapshot(root, metadata) {
  root = fs.realpathSync(root);
  const identities = verifiedSources(root);
  verifyResolution(root, identities, metadata);
  const allowedLocal = new Set(['otter-native', ...identities.map(p => p.name)]);
  if (metadata.packages.some(p => p.source == null && !allowedLocal.has(p.name))) {
    throw new Error('Uninventoried local Cargo dependency');
  }
  const native = metadata.packages.filter(p => p.name === 'otter-native');
  if (native.length !== 1 || fs.realpathSync(native[0].manifest_path) !== path.join(root, 'native', 'Cargo.toml')) {
    throw new Error('Native package resolution mismatch');
  }
  const version = JSON.parse(fs.readFileSync(checkedPath(root, 'version-state.json'), 'utf8'));
  const pkg = JSON.parse(fs.readFileSync(checkedPath(root, 'package.json'), 'utf8'));
  if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version.publicVersion) || version.publicVersion !== pkg.version ||
      version.publicVersion !== native[0].version || !Number.isInteger(version.internalRevision) ||
      version.internalRevision < 1 || version.internalRevision > 65535) throw new Error('Invalid internal build version');
  const names = new Set(['native/Cargo.toml', 'native/Cargo.lock', 'native/build.rs', 'native/windows.manifest',
    'src-tauri/icons/icon.ico', 'src-tauri/icons/32x32.png', 'package.json', 'version-state.json', 'third_party/sources.json',
    'scripts/build-native-candidate.ps1', 'scripts/package-native-candidate.ps1', 'scripts/verify-native-package.ps1',
    'scripts/version-identity.ps1', 'scripts/file-hash.ps1', 'THIRD_PARTY_NOTICES.md',
    'scripts/release/PathSafety.ps1', 'scripts/release/third-party-sources.cjs']);
  for (const directory of ['native/src', 'native/tests', 'native/build_support', 'scripts/native-candidate']) {
    filesUnder(root, directory, names);
  }
  for (const directory of ['.cargo', 'native/.cargo']) {
    if (fs.existsSync(path.join(root, directory))) filesUnder(root, directory, names);
  }
  for (const name of ['rust-toolchain', 'rust-toolchain.toml', 'native/rust-toolchain', 'native/rust-toolchain.toml']) {
    if (fs.existsSync(path.join(root, name))) names.add(name);
  }
  for (const entry of identities) {
    names.add(entry.manifest);
    names.add(entry.patch);
    filesUnder(root, entry.directory, names);
  }
  const files = [...names].sort().map(name => {
    const bytes = fs.readFileSync(checkedPath(root, name));
    return { path: name, bytes: bytes.length, sha256: hash(bytes) };
  });
  return { schemaVersion: 1, kind: 'otter-native-build-inputs',
    buildVersion: `v${version.publicVersion}.${version.internalRevision}`, productVersion: version.publicVersion,
    internalRevision: version.internalRevision, identities, files, sourceSha256: hash(encoded(files)) };
}

function assertUnchanged(before, after) {
  if (encoded(before) !== encoded(after)) throw new Error('Native build inputs changed during build; candidate rejected');
}

if (require.main === module) {
  const [root, metadataFile, output] = process.argv.slice(2);
  const result = snapshot(root, JSON.parse(fs.readFileSync(metadataFile, 'utf8')));
  fs.writeFileSync(output, JSON.stringify(result, null, 2) + '\n', { encoding: 'utf8', flag: 'wx' });
}
module.exports = { snapshot, assertUnchanged, hash };
