// rquickjs 0.14.0 的两个子 crate 未携带仓库级 MIT；仅消费精确审阅的原字节，不通用回退。
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { checkedPath } = require('../release/third-party-sources.cjs');
const REGISTRY = 'registry+https://github.com/rust-lang/crates.io-index';
const RQUICKJS = {
  sha256: '976ad3d07927343ab99b31510625acba89eac8e0e517c712925620ddeda91b70',
  upstream: 'https://raw.githubusercontent.com/DelSkayn/rquickjs/d7ef5eeae702fea24c03643064de454f1c1dd4b0/LICENSE',
};
const QUICKJS = {
  sha256: '96f73f9d2a16c21a36b418f06073be26e7d6d5e7c1bc99756b21a4f2c74ef171',
  upstream: 'https://raw.githubusercontent.com/quickjs-ng/quickjs/0fdea21ff1090084e91dad812b343d92e79ba9d9/LICENSE',
};
function original(pkg, registryRoot, name, pin) {
  const relative = path.relative(registryRoot, path.dirname(pkg.manifest_path)).split(path.sep).join('/');
  const directory = checkedPath(registryRoot, relative);
  const file = checkedPath(directory, name);
  const stat = fs.statSync(file);
  if (!stat.isFile() || stat.size < 1 || stat.size > 4096) throw new Error('Invalid JavaScript license size');
  const bytes = fs.readFileSync(file);
  new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  const sha256 = crypto.createHash('sha256').update(bytes).digest('hex');
  if (bytes.length > 4096 || sha256 !== pin.sha256) throw new Error('JavaScript license original mismatch');
  return { name, bytes, sha256, upstream: pin.upstream };
}
function javascriptLicenses(pkg, metadata, registryRoot) {
  if (!['rquickjs-core', 'rquickjs-sys'].includes(pkg.name)) return [];
  if (pkg.version !== '0.14.0' || pkg.license !== 'MIT' || pkg.source !== REGISTRY) {
    throw new Error('Unreviewed JavaScript license identity');
  }
  const peers = metadata.packages.filter(p => p.name === 'rquickjs' && p.version === '0.14.0' &&
    p.source === REGISTRY && p.license === 'MIT');
  if (peers.length !== 1) throw new Error('Missing or duplicate reviewed JavaScript license peer');
  const files = [original(peers[0], registryRoot, 'LICENSE', RQUICKJS)];
  if (pkg.name === 'rquickjs-sys') files.push(original(pkg, registryRoot, 'quickjs/LICENSE', QUICKJS));
  return files;
}
module.exports = { javascriptLicenses };
