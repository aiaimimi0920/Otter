// 按子库与模块前缀将待确认文件自动聚成 40-80 文件批次，并仅对实际未审/过期文件执行有效行数词法统计与热点扫描。
// 仅作阅读排期与注意力引导，不替代逐文件人工完整阅读契约。
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { countEffectiveLines, languageForExtension } from './effective-code-lines-lexer.mjs';
import { classifyGroup, scanRepositoryHashes } from './review-coverage.mjs';

export { classifyGroup };

const ROOT = fileURLToPath(new URL('../', import.meta.url));

const HOTSPOT_PATTERNS = [
  ['json_decode', /\bserde_json::(?:from_str|from_slice|from_value|to_vec|to_string)\b/g, 2],
  ['alloc_clone', /\.(?:clone|to_vec|to_owned)\s*\(/g, 4],
  ['crypto_hash', /\b(?:Sha256|Hmac|base64|digest)\b/gi, 2],
  ['win32_query_gdi', /\b(?:GetWindowRect|GetClientRect|GetDpiForWindow|SetWindowPos|CreateCompatibleDC|CreateCompatibleBitmap|SelectObject|DeleteObject|SendMessageW|PostMessageW)\b/g, 2],
  ['sync_cancel', /\b(?:Mutex|Condvar|RwLock|WaitForMultipleObjects|WaitForSingleObject|PoisonError|AtomicBool)\b/g, 2],
  ['unsafe_ffi', /\bunsafe\b/g, 3],
];

export function moduleClusterKey(filePath) {
  const group = classifyGroup(filePath);
  const base = path.posix.basename(filePath).replace(/\.[a-z0-9]+$/i, '');
  const trimmed = base.replace(/_(?:tests|smoke|fixture|peer|oracle)$/i, '');
  const parts = trimmed.split(/[_-]/).filter(Boolean);
  const prefix = parts.slice(0, Math.min(2, parts.length)).join('_') || trimmed;
  return `${group}:${prefix}`;
}

export function detectHotspots(sourceText) {
  const tags = [];
  for (const [name, regex, minHits] of HOTSPOT_PATTERNS) {
    const matches = sourceText.match(regex);
    if (matches && matches.length >= minHits) {
      tags.push(`${name}(${matches.length})`);
    }
  }
  return tags;
}

export function buildReviewBatches(repoRoot = ROOT, options = {}) {
  const batchSize = Math.max(10, Math.min(120, Number(options.batchSize) || 50));
  const filterGroup = options.group || null;
  const limitBatches = Math.max(1, Number(options.limit) || 4);

  const policy = JSON.parse(fs.readFileSync(path.join(repoRoot, 'scripts/effective-code-lines-policy.json'), 'utf8'));
  const ledger = JSON.parse(fs.readFileSync(path.join(repoRoot, 'docs/development/review-coverage.json'), 'utf8'));
  const ledgerByPath = new Map(ledger.files.map(entry => [entry.path, entry]));

  const inventory = scanRepositoryHashes(repoRoot, policy, {
    filterGroup,
    needSourceFor: (filePath, sha) => ledgerByPath.get(filePath)?.sourceSha256 !== sha,
  });
  if (inventory.diagnostics.length) {
    throw new Error(inventory.diagnostics.join('\n'));
  }

  const pendingRows = [];
  const stalePaths = [];

  for (const row of inventory.rows) {
    const existing = ledgerByPath.get(row.path);
    if (existing && existing.sourceSha256 === row.sourceSha256) continue;
    const status = existing ? 'stale' : 'pending';
    if (existing) stalePaths.push(row.path);
    const text = row.source ?? fs.readFileSync(path.join(repoRoot, row.path), 'utf8');
    const language = languageForExtension(path.extname(row.path).toLowerCase());
    pendingRows.push({
      path: row.path,
      group: classifyGroup(row.path),
      cluster: moduleClusterKey(row.path),
      effectiveLines: countEffectiveLines(text, language),
      status,
      hotspots: detectHotspots(text),
    });
  }

  pendingRows.sort((a, b) => a.cluster.localeCompare(b.cluster) || a.path.localeCompare(b.path));

  const batches = [];
  let current = null;
  for (const item of pendingRows) {
    if (!current || current.group !== item.group || current.files.length >= batchSize) {
      current = {
        batchIndex: batches.length + 1,
        group: item.group,
        totalEffectiveLines: 0,
        hotspotFileCount: 0,
        cleanCandidateCount: 0,
        clusters: new Set(),
        files: [],
      };
      batches.push(current);
    }
    current.files.push(item);
    current.totalEffectiveLines += item.effectiveLines;
    current.clusters.add(item.cluster.split(':')[1]);
    if (item.hotspots.length > 0) current.hotspotFileCount++;
    else current.cleanCandidateCount++;
  }

  const formatted = batches.slice(0, limitBatches).map(b => ({
    batchIndex: b.batchIndex,
    group: b.group,
    fileCount: b.files.length,
    totalEffectiveLines: b.totalEffectiveLines,
    hotspotFileCount: b.hotspotFileCount,
    cleanCandidateCount: b.cleanCandidateCount,
    modeHint: b.hotspotFileCount === 0 ? 'fast-retained-sweep' : 'mixed-audit-and-batch-fix',
    clusters: [...b.clusters],
    hotspotFiles: b.files
      .filter(f => f.hotspots.length > 0)
      .map(f => ({ path: f.path, lines: f.effectiveLines, hotspots: f.hotspots })),
    cleanFiles: b.files.filter(f => f.hotspots.length === 0).map(f => f.path),
  }));

  return {
    totalPendingInScope: pendingRows.length,
    staleCount: stalePaths.length,
    batchSize,
    totalBatches: batches.length,
    nextBatches: formatted,
  };
}

function parseCliArgs(argv) {
  const opts = {};
  for (let i = 0; i < argv.length; i++) {
    if (argv[i] === '--batch-size' && argv[i + 1]) opts.batchSize = Number(argv[++i]);
    else if (argv[i] === '--group' && argv[i + 1]) opts.group = argv[++i];
    else if (argv[i] === '--limit' && argv[i + 1]) opts.limit = Number(argv[++i]);
  }
  return opts;
}

if (process.argv[1] && path.resolve(process.argv[1]) === path.resolve(fileURLToPath(import.meta.url))) {
  const result = buildReviewBatches(ROOT, parseCliArgs(process.argv.slice(2)));
  console.log(JSON.stringify(result, null, 2));
}
