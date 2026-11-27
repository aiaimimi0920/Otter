// Create internal distribution directories from verified candidates; copy no machine environment/credentials and grant no release status.
const fs = require('node:fs');
const path = require('node:path');
const { digest, verify } = require('./provenance.cjs');
const { snapshot, assertUnchanged } = require('./source.cjs');
const { collect } = require('./licenses.cjs');
const { checkedPath } = require('../release/third-party-sources.cjs');

function stage(repo, candidate, output, channel = 'internal') {
  const receipt = verify(candidate, undefined, channel);
  const metadata = JSON.parse(fs.readFileSync(checkedPath(candidate, 'metadata.json'), 'utf8'));
  const source = JSON.parse(fs.readFileSync(checkedPath(candidate, 'source-before.json'), 'utf8'));
  assertUnchanged(source, snapshot(repo, metadata, channel));
  if (fs.existsSync(output)) throw new Error('Native package staging directory already exists');
  fs.mkdirSync(output);
  const cargoHome = process.env.CARGO_HOME || path.join(process.env.USERPROFILE, '.cargo');
  collect(repo, metadata, path.join(output, 'licenses'), path.join(cargoHome, 'registry/src'));
  fs.copyFileSync(checkedPath(candidate, 'payload/otter-native.exe'), path.join(output, 'otter-native.exe'), fs.constants.COPYFILE_EXCL);
  fs.copyFileSync(checkedPath(candidate, 'payload/otter_live_hover_hook.dll'), path.join(output, 'otter_live_hover_hook.dll'), fs.constants.COPYFILE_EXCL);
  fs.writeFileSync(path.join(output, 'build-provenance.json'), JSON.stringify(receipt, null, 2) + '\n', { flag: 'wx' });
  fs.writeFileSync(path.join(output, 'README.txt'),
    `Otter Windows Native ${receipt.buildVersion}\n\n` +
    (channel === 'public' ? 'Public Windows x64 portable ZIP. Unsigned; not an installer or a UIAccess package.\n' :
      'Internal development candidate: unsigned and not an official release.\n') +
    'Run otter-native.exe to start the native application. Keep otter_live_hover_hook.dll in the same directory for local Live browser hover support.\n' +
    'The licenses directory contains normal/build dependency materials; this does not indicate legal or security approval.\n' +
    'build-provenance.json summarizes build identity; original build logs and source snapshots are not distributed with this package.\n' +
    'package-manifest.json records byte consistency, not signature authentication or complete functional acceptance.\n', { flag: 'wx' });
  const files = [];
  function walk(relative = '') {
    const directory = relative ? checkedPath(output, relative) : output;
    for (const name of fs.readdirSync(directory).sort()) {
      const relativeFile = relative ? `${relative}/${name}` : name;
      const file = checkedPath(output, relativeFile);
      if (fs.statSync(file).isDirectory()) walk(relativeFile);
      else files.push(digest(output, relativeFile, 64 * 1024 * 1024));
    }
  }
  walk();
  const exe = files.find(f => f.path === 'otter-native.exe');
  if (exe.sha256 !== receipt.files[0].sha256 || exe.bytes !== receipt.files[0].bytes) throw new Error('Candidate changed during staging');
  const hook = files.find(f => f.path === 'otter_live_hover_hook.dll');
  if (hook.sha256 !== receipt.files[1].sha256 || hook.bytes !== receipt.files[1].bytes) throw new Error('Candidate hook changed during staging');
  assertUnchanged(source, snapshot(repo, metadata, channel));
  verify(candidate, receipt, channel);
  const manifest = { schemaVersion: 1, kind: `otter-native-${channel}-package`, buildVersion: receipt.buildVersion,
    sourceSha256: receipt.sourceSha256, publishable: false, signed: false, runtimeVerified: false, files };
  fs.writeFileSync(path.join(output, 'package-manifest.json'), JSON.stringify(manifest, null, 2) + '\n', { flag: 'wx' });
  return manifest;
}
if (require.main === module) {
  const result = stage(...process.argv.slice(2));
  console.log(JSON.stringify({ buildVersion: result.buildVersion, files: result.files.length, publishable: false }));
}
module.exports = { stage };
