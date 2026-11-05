// 从已验证候选生成内部分发目录；不复制机器环境/凭证，不授予发布状态。
const fs = require('node:fs');
const path = require('node:path');
const { verify } = require('./provenance.cjs');
const { snapshot, assertUnchanged, hash } = require('./source.cjs');
const { collect } = require('./licenses.cjs');
const { checkedPath } = require('../release/third-party-sources.cjs');

function stage(repo, candidate, output) {
  const receipt = verify(candidate);
  const metadata = JSON.parse(fs.readFileSync(checkedPath(candidate, 'metadata.json'), 'utf8'));
  const source = JSON.parse(fs.readFileSync(checkedPath(candidate, 'source-before.json'), 'utf8'));
  assertUnchanged(source, snapshot(repo, metadata));
  if (fs.existsSync(output)) throw new Error('Native package staging directory already exists');
  fs.mkdirSync(output);
  const cargoHome = process.env.CARGO_HOME || path.join(process.env.USERPROFILE, '.cargo');
  collect(repo, metadata, path.join(output, 'licenses'), path.join(cargoHome, 'registry/src'));
  fs.copyFileSync(checkedPath(candidate, 'payload/otter-native.exe'), path.join(output, 'otter-native.exe'), fs.constants.COPYFILE_EXCL);
  fs.writeFileSync(path.join(output, 'build-provenance.json'), JSON.stringify(receipt, null, 2) + '\n', { flag: 'wx' });
  fs.writeFileSync(path.join(output, 'README.txt'),
    `Otter Windows Native ${receipt.buildVersion}\n\n内部开发候选，未签名，不是正式发布。\n运行 otter-native.exe 启动原生程序。\n` +
    'licenses 目录包含 normal/build 依赖材料；不表示法律或安全检查已批准。\n' +
    'build-provenance.json 是构建身份摘要，原始构建日志与源码快照不随包分发。\n' +
    'package-manifest.json 提供字节一致性记录，不提供签名认证或完整功能验收。\n', { flag: 'wx' });
  const files = [];
  function walk(relative = '') {
    const directory = relative ? checkedPath(output, relative) : output;
    for (const name of fs.readdirSync(directory).sort()) {
      const relativeFile = relative ? `${relative}/${name}` : name;
      const file = checkedPath(output, relativeFile);
      if (fs.statSync(file).isDirectory()) walk(relativeFile);
      else {
        const bytes = fs.readFileSync(file);
        files.push({ path: relativeFile, bytes: bytes.length, sha256: hash(bytes) });
      }
    }
  }
  walk();
  const exe = files.find(f => f.path === 'otter-native.exe');
  if (exe.sha256 !== receipt.files[0].sha256 || exe.bytes !== receipt.files[0].bytes) throw new Error('Candidate changed during staging');
  assertUnchanged(source, snapshot(repo, metadata));
  verify(candidate);
  const manifest = { schemaVersion: 1, kind: 'otter-native-internal-package', buildVersion: receipt.buildVersion,
    sourceSha256: receipt.sourceSha256, publishable: false, signed: false, runtimeVerified: false, files };
  fs.writeFileSync(path.join(output, 'package-manifest.json'), JSON.stringify(manifest, null, 2) + '\n', { flag: 'wx' });
  return manifest;
}
if (require.main === module) {
  const result = stage(...process.argv.slice(2));
  console.log(JSON.stringify({ buildVersion: result.buildVersion, files: result.files.length, publishable: false }));
}
module.exports = { stage };
