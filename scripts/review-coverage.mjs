// 复用官方源码清单与排除契约，仅计算 LF 归一化 sourceSha256（跳过全仓词法行数统计以将台账核验压至亚秒级）。
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { decodeUtf8, SUPPORTED_EXTENSIONS } from './effective-code-lines-lexer.mjs';
import { normalizeRepoPath } from './effective-code-lines.mjs';

const ROOT = fileURLToPath(new URL('../', import.meta.url));

export function classifyGroup(filePath) {
  if (filePath.startsWith('native/otter-core/')) return 'otter-core';
  if (filePath.startsWith('native/otter-media/')) return 'otter-media';
  if (filePath.startsWith('native/otter-runtime/')) return 'otter-runtime';
  if (filePath.startsWith('native/hover-hook/')) return 'hover-hook';
  if (filePath.startsWith('native/src/desktop/')) return 'desktop';
  if (filePath.startsWith('native/src/')) return 'native-model';
  if (filePath.startsWith('native/tests/')) return 'native-integration';
  if (filePath.startsWith('scripts/')) return 'scripts';
  return 'build-and-ci';
}

function directoryMayContainGroup(relativeDir, filterGroup) {
  if (!filterGroup || !relativeDir) return true;
  const prefix = `${relativeDir}/`;
  const groupRootMap = {
    'otter-core': 'native/otter-core/',
    'otter-media': 'native/otter-media/',
    'otter-runtime': 'native/otter-runtime/',
    'hover-hook': 'native/hover-hook/',
    desktop: 'native/src/desktop/',
    'native-integration': 'native/tests/',
    scripts: 'scripts/',
  };
  const targetPrefix = groupRootMap[filterGroup];
  if (targetPrefix) {
    return targetPrefix.startsWith(prefix) || prefix.startsWith(targetPrefix);
  }
  if (filterGroup === 'native-model') {
    if (prefix.startsWith('native/src/desktop/')) return false;
    return 'native/src/'.startsWith(prefix) || prefix.startsWith('native/src/');
  }
  if (filterGroup === 'build-and-ci') {
    return classifyGroup(prefix) === 'build-and-ci';
  }
  return true;
}

function buildExclusions(policy) {
  assert.equal(policy.schemaVersion, 1, 'policy schemaVersion must be 1');
  for (const ext of policy.sourceExtensions) {
    assert.ok(SUPPORTED_EXTENSIONS.includes(ext), `unsupported source extension: ${ext}`);
  }
  return {
    names: new Set(policy.excludedDirectories.map(e => e.name.toLowerCase())),
    prefixes: policy.excludedDirectoryPrefixes.map(e => e.prefix.toLowerCase()),
    paths: policy.excludedPaths.map(e => normalizeRepoPath(e.path)),
    extensions: new Set(policy.sourceExtensions),
  };
}

function isExcluded(relativePath, entryName, exclusions) {
  const lower = entryName.toLowerCase();
  if (exclusions.names.has(lower)) return true;
  if (exclusions.prefixes.some(p => lower.startsWith(p))) return true;
  return exclusions.paths.some(p => relativePath === p || relativePath.startsWith(`${p}/`));
}

export function hashSourceBytes(relativePath, bytes, needSource = false) {
  const source = decodeUtf8(bytes, relativePath);
  const hasBom = bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf;
  if (!hasBom && !bytes.includes(0x0d)) {
    const sourceSha256 = crypto.createHash('sha256').update(bytes).digest('hex');
    return needSource ? { path: relativePath, sourceSha256, source } : { path: relativePath, sourceSha256 };
  }
  const canonical = source.replace(/\r\n|\r/g, '\n');
  const sourceSha256 = crypto.createHash('sha256').update(canonical, 'utf8').digest('hex');
  return needSource ? { path: relativePath, sourceSha256, source: canonical } : { path: relativePath, sourceSha256 };
}

export function scanRepositoryHashes(root = ROOT, policy = null, options = {}) {
  const activePolicy =
    policy || JSON.parse(fs.readFileSync(path.join(root, 'scripts/effective-code-lines-policy.json'), 'utf8'));
  const resolvedRoot = fs.realpathSync(root);
  const exclusions = buildExclusions(activePolicy);
  const filterGroup = options.filterGroup || null;
  const needSourceFor = options.needSourceFor || null;
  const rows = [];
  const diagnostics = [];

  function visit(directory, relativeDirectory = '') {
    const entries = fs
      .readdirSync(directory, { withFileTypes: true })
      .sort((a, b) => a.name.localeCompare(b.name, 'en'));
    for (const entry of entries) {
      const relativePath = relativeDirectory ? `${relativeDirectory}/${entry.name}` : entry.name;
      if (isExcluded(relativePath, entry.name, exclusions)) continue;
      const absolutePath = path.join(directory, entry.name);
      if (entry.isSymbolicLink()) {
        diagnostics.push(`${relativePath}: symbolic links and junctions are not scanned`);
        continue;
      }
      if (entry.isDirectory()) {
        if (!directoryMayContainGroup(relativePath, filterGroup)) continue;
        const realDirectory = fs.realpathSync(absolutePath);
        if (realDirectory !== resolvedRoot && !realDirectory.startsWith(`${resolvedRoot}${path.sep}`)) {
          diagnostics.push(`${relativePath}: directory escapes repository root`);
          continue;
        }
        visit(absolutePath, relativePath);
      } else if (entry.isFile() && exclusions.extensions.has(path.extname(entry.name).toLowerCase())) {
        const normalized = normalizeRepoPath(relativePath);
        if (filterGroup && classifyGroup(normalized) !== filterGroup) continue;
        const bytes = fs.readFileSync(absolutePath);
        const hashed = hashSourceBytes(normalized, bytes, Boolean(needSourceFor));
        if (needSourceFor && !needSourceFor(normalized, hashed.sourceSha256)) {
          delete hashed.source;
        }
        rows.push(hashed);
      }
    }
  }

  visit(resolvedRoot);
  rows.sort((a, b) => a.path.localeCompare(b.path, 'en'));
  return { rows, diagnostics };
}

export function computeReviewCoverage(repoRoot = ROOT) {
  const read = rel => JSON.parse(fs.readFileSync(path.join(repoRoot, rel), 'utf8'));
  const ledger = read('docs/development/review-coverage.json');
  if (ledger.schemaVersion !== 1) throw new Error('Unsupported review ledger version');
  const inventory = scanRepositoryHashes(repoRoot, read('scripts/effective-code-lines-policy.json'));
  if (inventory.diagnostics.length) throw new Error(inventory.diagnostics.join('\n'));
  const files = new Map(inventory.rows.map(row => [row.path, row]));
  const reviewed = new Set();
  const stale = [];
  for (const entry of ledger.files) {
    if (reviewed.has(entry.path) || stale.includes(entry.path)) {
      throw new Error(`Duplicate review entry: ${entry.path}`);
    }
    if (!files.has(entry.path)) throw new Error(`Review entry absent from inventory: ${entry.path}`);
    if (!['retained', 'changed'].includes(entry.outcome) || !entry.finding?.trim()) {
      throw new Error(`Incomplete review entry: ${entry.path}`);
    }
    if (files.get(entry.path).sourceSha256 === entry.sourceSha256) reviewed.add(entry.path);
    else stale.push(entry.path);
  }
  const groups = {};
  for (const row of inventory.rows) {
    const count = (groups[classifyGroup(row.path)] ??= { total: 0, reviewed: 0, pendingConfirmation: 0 });
    count.total++;
    if (reviewed.has(row.path)) count.reviewed++;
    else count.pendingConfirmation++;
  }
  return {
    total: files.size,
    reviewed: reviewed.size,
    pendingConfirmation: files.size - reviewed.size,
    stale,
    groups,
  };
}

const invoked = process.argv[1] ? pathToFileURL(path.resolve(process.argv[1])).href : undefined;
if (invoked === import.meta.url) {
  console.log(JSON.stringify(computeReviewCoverage(ROOT), null, 2));
}
