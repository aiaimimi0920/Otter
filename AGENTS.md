# Otter Repository Instructions

Scope: this independent Otter repository and all of its subdirectories.

Otter owns the visual desktop, capture, overlay/canvas, editing, and local
integration surfaces. Preserve the repository boundary: integrate with Loom,
Talk, Tea, Gateway, and Platform through their contracts rather than copying
their internal implementations here.

Before development, read `CONTRIBUTING.md`, the README, and the owning design,
test, security, and release documents. When mounted inside Neuro,
`../docs/DEVELOPMENT_STANDARD.md` is the expanded common standard. This file is
self-contained so a standalone Otter checkout inherits the same mandatory rules.

Repository placement and contribution rules are maintained in
`docs/development/CODE_ORGANIZATION.md`. Read that document before adding or moving
files. Keep its ownership map and the architecture overview synchronized with
structural changes; do not duplicate business implementations in shared modules.

## Mandatory incremental code-size standard

Effective code lines exclude blank lines, comment-only lines, and comment-only
multiline regions. A line containing code plus an inline comment still counts.
A repository language-aware checker, when present, is authoritative.

- Target about 150 effective lines; 100-250 is the preferred design range.
- 251-500 is acceptable only for one clear responsibility.
- 501-700 is a soft-limit exception. Record why another split would harm
  cohesion/readability/verification and identify protecting tests.
- A new file or new extraction result at 701-1500 is not complete; split it.
- More than 1500 is a hard violation with no waiver; split again.

The rule covers handwritten production code, tests, scripts, and styles.
Generated or immutable third-party files are excluded only by explicit policy.
Do not game the metric with comments, strings, minification, generated code,
renames, extensions, or dumping grounds named `common`, `utils`, or `helpers`.

## Legacy-debt ratchet

- Existing oversized files are grandfathered debt. Unrelated work does not have
  to split every historical file before it can finish.
- The grandfathering protects existing content, not future growth. New files,
  modules, responsibilities, types/functions, and material extensions are new
  code and must follow the limits.
- Put a new responsibility in a cohesive compliant module and leave only
  minimal wiring in an existing large file.
- A narrow correctness, security, compatibility, or wiring fix may be made in
  place without an unrelated rewrite. Avoid net growth where practical; report
  unavoidable growth and why immediate extraction would increase risk.
- A file that was at most 700 effective lines before the task must not cross
  700 because of the task.
- If the task explicitly refactors a large file, every new or fully migrated
  result must be at most 700 lines. Report unrelated debt honestly.

## Required development workflow

1. Measure affected files before design and check for stricter local policy.
2. Preserve behavior with focused tests; split by responsibility, dependency
   direction, state ownership, and resource lifetime rather than arbitrary size.
3. Add concise comments for module purpose and non-obvious invariants, trust
   boundaries, concurrency, cancellation, cleanup, and performance tradeoffs.
4. Review each new or materially changed file for input/auth/secret safety,
   injection and traversal, bounded memory/queues, task/handle/process cleanup,
   UI listener/window cleanup, blocking work, allocation/copying, and hot paths.
5. Run focused tests, directly dependent compile/typecheck/static checks, the
   official formatter, and any repository line checker.
6. Run `git diff --check` and inspect this independent repository's scoped diff.
7. For runtime or release work, run Otter's documented package/runtime gates;
   documentation-only governance changes do not require a fake release build.

Release and dependency changes must also follow `docs/release/release-provenance.md` and
`docs/security/DEPENDENCY_SECURITY.md`; a dirty candidate is evidence, not a publishable
formal release.

## Fast iteration and build-cache discipline

- Read related Rust and desktop TypeScript files in parallel, plan 150-500
  effective-line module splits upfront, and implement each complete vertical
  slice in one coordinated batch so you do not pay a second split-and-relink pass.
- Preserve the layered Cargo workspace (`native/otter-core` -> `native/otter-media`
  -> `native/otter-runtime` -> `native/src` host + single-binary
  `native/tests/integration.rs`); test changes in `otter-core`, `otter-media`, or
  `otter-runtime` via `-p otter-core` / `-p otter-media` / `-p otter-runtime` first
  rather than relinking the full `otter-native` host when host code is untouched.
  When iterating on host (`otter-native`) code, use opt-in scoped test features
  (`--features test-live`, `test-surface`, `test-save`, `test-capture`, `test-loom`,
  or `test-canvas`) for fast targeted inner-loop runs while keeping default
  unflagged `cargo nextest` / CI runs covering 100% of unit tests.
- Respect unified `CARGO_BUILD_JOBS=6` and `CARGO_INCREMENTAL=1` concurrency
  (`~/.cargo/config.toml` enables `lld-link`, `debug = 0`, `build-override`
  `opt-level = 3`, and heavy image/crypto crate `opt-level = 2`); use
  `npm run check:fast` (`+nightly -Z threads=6` isolated in `native/target-nightly-check`)
  for fast multi-core typechecks and `cargo nextest` (`native/.config/nextest.toml`)
  for process-isolated test execution with timeouts.
- For repository-wide review (`docs/development/review-coverage.json`), run
  `node scripts/review-batches.mjs` to group pending modules and pre-screen hotspots,
  and update the ledger atomically via `npm run review:upsert`
  (`node scripts/review-ledger-upsert.mjs --entries <batch.json>` or `--refresh-paths`,
  never hand-crafting ad-hoc SHA256/JSON rewrite scripts). Separate read-only review
  batches (dispatching 3-5 read-only subagents across independent directories and
  registering 40-80 `retained` files per batch without Cargo builds or
  `version:internal` bumps) from batched code-optimization slices, overlap background
  Cargo builds with read-only pre-review of the next batch, and relay to a fresh
  session every ~400-500 reviewed files to keep context lean.
- Never run `cargo check --tests` immediately before an unchanged `cargo test`;
  format modified Rust files incrementally via
  `rustfmt --edition 2021 --config skip_children=true <files>`, reuse built test
  binaries under `native/target/debug/deps/` when Rust source is unchanged, and
  never spawn concurrent Cargo writers on `target/` or
  disposable cold `CARGO_TARGET_DIR` folders under `linshi/`.

Any stricter repository-local checker or CI gate overrides this common floor.

## Public and internal versions

- Public releases use `vx.y.z`; internal iterations use `vx.y.z.n`.
- For an internal development iteration, run `npm run version:internal` once
  and commit `version-state.json` with that iteration. Do not increment the
  public package or Cargo version for internal work.
- Advance the public three-part version only for an explicitly requested public
  release, align all package metadata, and reset `internalRevision` to zero.
- Cargo and npm retain three-part SemVer; the fourth part is a Otter build
  identity in provenance, not a dependency version. Never publish an internal
  candidate or a four-part tag through the public release pipeline.
- Follow `docs/release/VERSIONING.md` for commands, compatibility, and release gates.
