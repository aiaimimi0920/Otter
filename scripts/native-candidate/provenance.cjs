// 候选收据只声明构建身份；静态验证不授予发布、签名、运行或功能验收状态。
const fs = require('node:fs');
const path = require('node:path');
const { assertUnchanged, hash } = require('./source.cjs');
const { checkedPath } = require('../release/third-party-sources.cjs');

function read(root, name, limit = 16 * 1024 * 1024) {
  const file = checkedPath(root, name);
  if (fs.statSync(file).size > limit) throw new Error(`Oversized candidate metadata: ${name}`);
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}
function digest(root, name) {
  const file = checkedPath(root, name);
  const stat = fs.statSync(file);
  if (!stat.isFile() || stat.size > 512 * 1024 * 1024) throw new Error(`Invalid candidate file: ${name}`);
  const bytes = fs.readFileSync(file);
  return { path: name, bytes: bytes.length, sha256: hash(bytes) };
}
function validateSource(source) {
  if (source.schemaVersion !== 1 || source.kind !== 'otter-native-build-inputs' ||
      !Array.isArray(source.files) || source.files.length === 0 || source.files.length > 10000 ||
      hash(JSON.stringify(source.files)) !== source.sourceSha256 ||
      !/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(source.productVersion) ||
      source.buildVersion !== `v${source.productVersion}.${source.internalRevision}` ||
      !Number.isInteger(source.internalRevision) || source.internalRevision < 1 || source.internalRevision > 65535) {
    throw new Error('Invalid native source binding');
  }
  const seen = new Set();
  for (const file of source.files) {
    if (typeof file.path !== 'string' || !file.path || /[\\:\x00-\x1f]/.test(file.path) || file.path.split('/').some(x => !x || x === '.' || x === '..') ||
        seen.has(file.path.toLowerCase()) || !Number.isSafeInteger(file.bytes) || file.bytes < 0 || !/^[a-f0-9]{64}$/.test(file.sha256)) {
      throw new Error('Invalid native source entry');
    }
    seen.add(file.path.toLowerCase());
  }
}
function validateTools(tools) {
  for (const name of ['cargo', 'rustc', 'resource', 'node', 'git']) {
    if (typeof tools[name]?.path !== 'string' || !tools[name].path || !/^[a-f0-9]{64}$/.test(tools[name].sha256)) {
      throw new Error('Invalid native tool binding');
    }
  }
  if (typeof tools.rustcVersion !== 'string' || !/^host: x86_64-pc-windows-msvc\s*$/m.test(tools.rustcVersion) ||
      typeof tools.cargoVersion !== 'string' || !tools.cargoVersion.startsWith('cargo ')) throw new Error('Invalid native tool version');
  for (const name of ['environmentHashes', 'externalCargoConfigHashes']) {
    if (!tools[name] || typeof tools[name] !== 'object' || Array.isArray(tools[name]) ||
        Object.values(tools[name]).some(value => !/^[a-f0-9]{64}$/.test(value))) throw new Error('Invalid native tool configuration');
  }
}
function create(root) {
  if (fs.existsSync(path.join(root, 'FAILED.txt'))) throw new Error('Failed native candidate cannot be verified');
  const before = read(root, 'source-before.json');
  const after = read(root, 'source-after.json');
  validateSource(before);
  validateSource(after);
  assertUnchanged(before, after);
  const toolsBefore = read(root, 'tools-before.json');
  const toolsAfter = read(root, 'tools-after.json');
  validateTools(toolsBefore);
  validateTools(toolsAfter);
  assertUnchanged(toolsBefore, toolsAfter);
  const build = read(root, 'build.json');
  if (build.exitCode !== 0 || build.profile !== 'release' || build.target !== 'x86_64-pc-windows-msvc' ||
      build.channel !== 'internal' || build.gitDirty !== true && build.gitDirty !== false ||
      !/^[a-f0-9]{40}$/.test(build.gitCommit) ||
      JSON.stringify(build.arguments) !== JSON.stringify(['build', '--manifest-path', 'native/Cargo.toml', '--locked', '--offline', '--release', '--bin', 'otter-native', '--target', 'x86_64-pc-windows-msvc'])) {
    throw new Error('Invalid native build invocation');
  }
  return { schemaVersion: 1, kind: 'otter-native-internal-candidate', channel: 'internal', publishable: false,
    buildVersion: before.buildVersion, productVersion: before.productVersion, internalRevision: before.internalRevision,
    sourceSha256: before.sourceSha256, gitCommit: build.gitCommit, gitDirty: build.gitDirty,
    profile: 'release', runtimeVerified: false, signed: false,
    files: ['payload/otter-native.exe', 'source-before.json', 'source-after.json', 'tools-before.json',
      'tools-after.json', 'build.json', 'build.log', 'metadata.json'].map(name => digest(root, name)) };
}
function verify(root) {
  const actual = read(root, 'provenance.json');
  const expected = create(root);
  if (JSON.stringify(actual) !== JSON.stringify(expected)) throw new Error('Native candidate provenance/content mismatch');
  return expected;
}
if (require.main === module) {
  const [command, root] = process.argv.slice(2);
  if (command === 'create') {
    fs.writeFileSync(path.join(root, 'provenance.json'), JSON.stringify(create(root), null, 2) + '\n', { encoding: 'utf8', flag: 'wx' });
  } else if (command !== 'verify') throw new Error('Expected create or verify');
  const result = verify(root);
  process.stdout.write(JSON.stringify({ status: 'build-identity-verified-not-runtime-or-release', buildVersion: result.buildVersion,
    sourceSha256: result.sourceSha256, executable: result.files[0] }) + '\n');
}
module.exports = { create, verify, validateSource };
