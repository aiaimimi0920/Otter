// 原子批量登记或更新 review-coverage.json：复用快速 LF 归一化哈希扫描（跳过全仓词法行数统计）并通过临时文件原子替换落盘。
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';
import { scanRepositoryHashes } from './review-coverage.mjs';

const ROOT = fileURLToPath(new URL('../', import.meta.url));
const USAGE =
  'Usage: node scripts/review-ledger-upsert.mjs --entries <entries.json> | --file <entries.json> | --json <array> | --entry <outcome|path|finding> [...] | --refresh-paths <p1> [p2...] | --stdin';

export function normalizeEntries(rawEntries, inventoryMap) {
  if (!Array.isArray(rawEntries) || rawEntries.length === 0) {
    throw new Error('Expected a non-empty JSON array of review entries');
  }
  const updates = new Map();
  for (const item of rawEntries) {
    const filePath = String(item?.path ?? '').trim().replace(/\\/g, '/');
    const outcome = String(item?.outcome ?? '').trim();
    const finding = String(item?.finding ?? '').trim();
    if (!filePath || !inventoryMap.has(filePath)) {
      throw new Error(`Review entry path absent from repository inventory: ${filePath || '<empty>'}`);
    }
    if (updates.has(filePath)) throw new Error(`Duplicate review update: ${filePath}`);
    if (!['retained', 'changed'].includes(outcome)) {
      throw new Error(`Invalid review outcome "${outcome}" for ${filePath} (expected retained or changed)`);
    }
    if (!finding) {
      throw new Error(`Missing non-empty finding for ${filePath}`);
    }
    updates.set(filePath, {
      path: filePath,
      sourceSha256: inventoryMap.get(filePath).sourceSha256,
      outcome,
      finding,
    });
  }
  return updates;
}

function loadInventoryAndLedger(repoRoot) {
  const policy = JSON.parse(fs.readFileSync(path.join(repoRoot, 'scripts/effective-code-lines-policy.json'), 'utf8'));
  const inventory = scanRepositoryHashes(repoRoot, policy);
  if (inventory.diagnostics.length) {
    throw new Error(inventory.diagnostics.join('\n'));
  }
  const inventoryMap = new Map(inventory.rows.map(row => [row.path, row]));
  const ledgerPath = path.join(repoRoot, 'docs/development/review-coverage.json');
  const ledger = JSON.parse(fs.readFileSync(ledgerPath, 'utf8'));
  if (ledger.schemaVersion !== 1 || !Array.isArray(ledger.files)) {
    throw new Error('Unsupported review-coverage.json schema');
  }
  const byPath = new Map();
  for (const record of ledger.files) {
    if (!inventoryMap.has(record.path)) throw new Error(`Review entry absent from repository inventory: ${record.path}`);
    if (byPath.has(record.path)) throw new Error(`Duplicate review entry: ${record.path}`);
    byPath.set(record.path, record);
  }
  return { inventoryMap, ledgerPath, ledger, byPath };
}

function writeAndSummarize(inventoryMap, ledgerPath, ledger, inserted, updated) {
  ledger.files.sort((a, b) => a.path.localeCompare(b.path));
  const payload = JSON.stringify(ledger, null, 2) + '\n';
  const tempPath = `${ledgerPath}.${process.pid}.${Date.now()}.tmp`;
  try {
    fs.writeFileSync(tempPath, payload, 'utf8');
    fs.renameSync(tempPath, ledgerPath);
  } finally {
    if (fs.existsSync(tempPath)) fs.unlinkSync(tempPath);
  }
  let reviewed = 0;
  const stale = [];
  for (const entry of ledger.files) {
    if (inventoryMap.get(entry.path)?.sourceSha256 === entry.sourceSha256) reviewed++;
    else stale.push(entry.path);
  }
  return {
    inserted,
    updated,
    totalInventory: inventoryMap.size,
    reviewed,
    pendingConfirmation: inventoryMap.size - reviewed,
    stale,
  };
}

export function upsertReviewLedger(repoRoot = ROOT, rawEntries = []) {
  const { inventoryMap, ledgerPath, ledger, byPath } = loadInventoryAndLedger(repoRoot);
  const updates = normalizeEntries(rawEntries, inventoryMap);
  let inserted = 0;
  let updated = 0;
  for (const [filePath, record] of updates) {
    if (byPath.has(filePath)) updated++;
    else inserted++;
    byPath.set(filePath, record);
  }
  ledger.files = [...byPath.values()];
  return writeAndSummarize(inventoryMap, ledgerPath, ledger, inserted, updated);
}

export function refreshLedgerPaths(repoRoot = ROOT, rawPaths = []) {
  const { inventoryMap, ledgerPath, ledger, byPath } = loadInventoryAndLedger(repoRoot);
  const targets = new Set(rawPaths.map(p => String(p).trim().replace(/\\/g, '/')).filter(Boolean));
  if (targets.size === 0) {
    throw new Error('Expected at least one path for --refresh-paths');
  }
  let updated = 0;
  for (const target of targets) {
    const entry = byPath.get(target);
    if (!entry) throw new Error(`Refresh path absent from review ledger: ${target}`);
    const row = inventoryMap.get(target);
    if (entry.sourceSha256 !== row.sourceSha256) {
      entry.sourceSha256 = row.sourceSha256;
      updated++;
    }
  }
  return writeAndSummarize(inventoryMap, ledgerPath, ledger, 0, updated);
}

function parseCompactEntry(spec) {
  const first = spec.indexOf('|');
  const second = first === -1 ? -1 : spec.indexOf('|', first + 1);
  if (first <= 0 || second <= first + 1) {
    throw new Error(`Invalid --entry format (expected outcome|path|finding): ${spec}`);
  }
  return {
    outcome: spec.slice(0, first).trim(),
    path: spec.slice(first + 1, second).trim(),
    finding: spec.slice(second + 1).trim(),
  };
}

export function parseCliArgs(argv) {
  if (argv.length === 0) {
    throw new Error(USAGE);
  }
  if ((argv[0] === '--file' || argv[0] === '--entries') && argv[1] && argv.length === 2) {
    const parsed = JSON.parse(fs.readFileSync(argv[1], 'utf8').replace(/^\uFEFF/, ''));
    return { mode: 'upsert', entries: Array.isArray(parsed) ? parsed : parsed.entries };
  }
  if (argv[0] === '--json' && argv[1] && argv.length === 2) {
    const parsed = JSON.parse(argv[1]);
    return { mode: 'upsert', entries: Array.isArray(parsed) ? parsed : parsed.entries };
  }
  if (argv[0] === '--entry' && argv.length >= 2) {
    const entries = [];
    for (let i = 0; i < argv.length; i += 2) {
      if (argv[i] !== '--entry' || !argv[i + 1]) throw new Error(USAGE);
      entries.push(parseCompactEntry(argv[i + 1]));
    }
    return { mode: 'upsert', entries };
  }
  if (argv[0] === '--refresh-paths' && argv.length >= 2) {
    return { mode: 'refresh', paths: argv.slice(1) };
  }
  if (argv[0] === '--stdin' && argv.length === 1) {
    if (process.stdin.isTTY) {
      throw new Error('Refusing --stdin on an interactive TTY without piped input; pass --entries <file> or --entry');
    }
    const stdin = fs.readFileSync(0, 'utf8').replace(/^\uFEFF/, '').trim();
    if (!stdin) throw new Error(USAGE);
    const parsed = JSON.parse(stdin);
    return { mode: 'upsert', entries: Array.isArray(parsed) ? parsed : parsed.entries };
  }
  throw new Error(USAGE);
}

if (process.argv[1] && path.resolve(process.argv[1]) === path.resolve(fileURLToPath(import.meta.url))) {
  const cmd = parseCliArgs(process.argv.slice(2));
  const summary =
    cmd.mode === 'refresh' ? refreshLedgerPaths(ROOT, cmd.paths) : upsertReviewLedger(ROOT, cmd.entries);
  console.log(JSON.stringify(summary, null, 2));
}
