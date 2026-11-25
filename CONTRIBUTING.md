# Contributing to Otter

Thank you for helping improve Otter. Bug reports, translations, documentation,
workflow examples and code contributions are all welcome — you do not need to
write code to take part.

[Simplified Chinese](docs/guide/contributing.zh-CN.md) · [Documentation](docs/README.md) · [Browse issues](https://github.com/aiaimimi0920/Otter/issues)

## Find your starting point

| I want to… | Start here |
| --- | --- |
| Report a bug or suggest an improvement | Search [existing issues](https://github.com/aiaimimi0920/Otter/issues), then describe the problem and expected result. |
| Improve wording or translate documentation | Read [wording and translation](#wording-and-translation) / [Simplified Chinese](docs/guide/contributing.zh-CN.md#%E6%96%87%E6%A1%88%E4%B8%8E%E7%BF%BB%E8%AF%91). |
| Share a workflow or help other users | Describe the steps and expected result in an issue or example. Remove private images, tokens and personal data. |
| Fix a bug or develop a feature | Follow the [build guide](docs/development/WINDOWS_NATIVE_BUILD.md), then find the owning module in the [code organization standard](docs/development/CODE_ORGANIZATION.md). |
| Report a security vulnerability | Use the [security policy](SECURITY.md), not a public bug report containing exploit details. |

## Make your first contribution

1. **Check the existing discussion.** For a larger feature or structural change,
   discuss the scope in an issue before investing in a broad implementation.
2. **Work on a focused branch from `main`.** Keep unrelated cleanup separate and
   preserve existing behavior outside your change.
3. **Read the owning code and guidance.** Otter is a Rust/Win32 desktop
   application; WebView history is archived at `archive/webview-main`. Use current implementation and tests, not historical plans, as
   the source of truth. Read [AGENTS.md](AGENTS.md) and the
   [development requirements below](#development-and-verification) for code work.
4. **Verify what you changed.** Run the relevant tests and checks. Documentation-only
   changes need link/content checks and their consuming contract tests, not an EXE rebuild.
5. **Open a pull request.** Explain the problem, the change, how you verified it,
   and anything still untested. Include before/after screenshots for visible UI changes,
   with private information removed.

For bug reports, include the Otter version, Windows version, reproduction steps,
expected versus actual behavior, and relevant sanitized logs or screenshots.
Mention display scaling, monitor layout, HDR or Loom authorization only when relevant.

## Wording and translation

- Use English for source comments, docstrings and default documentation. Keep translations
  in language-suffixed documents such as `.zh-CN.md`, `-zh.md` or `-cn.md`. Comment translation
  must not alter UI strings, protocol values, diagnostic matching or Unicode test data.
- Give the page or control location, original wording, proposed translation and reason.
  Fix a typo in its own language; update both language variants when behavior changes.
- Start with the [documentation index](docs/README.md). Desktop text lives in Rust components,
  for example the [edit-strip catalog](native/src/desktop/edit_strip_catalog.rs) and
  [pin settings](native/src/desktop/pin_settings_fields.rs). Changing these strings is a code
  change; translated documents or the OCR translation setting do not add an interface language.
- Preserve file names, options, identifiers, placeholders, authorization limits, license text
  and third-party attributions. Check local links, images, heading anchors and examples.
- Keep one maintained page per contract or procedure. Remove completed plans and superseded
  progress notes from the current documentation; Git retains their history. When retiring
  process records, remove their dedicated tooling and references together, without removing
  ongoing test or quality gates. Do not hide unresolved limitations.

## Before opening a pull request

- [ ] The change has one clear purpose and does not remove unrelated work or data.
- [ ] Relevant checks pass; untested native or cross-device behavior is stated explicitly.
- [ ] Documentation and both language variants are updated when affected.
- [ ] No credentials, private screenshots, local logs or generated build output are included.
- [ ] `git diff --check` is clean.

## Development and verification

Otter uses Win32, Direct2D and DirectComposition for the Windows product. Use the
[build guide](docs/development/WINDOWS_NATIVE_BUILD.md) for prerequisites and setup; legacy
browser tests do not validate the desktop app.

Before changing code, locate its owner, callers, tests and failure paths using the
[code organization standard](docs/development/CODE_ORGANIZATION.md).
Keep the message pump non-blocking and reuse existing worker budgets, generations and cleanup.
Move samples may coalesce; button, key-release, cancellation and shutdown edges may not be lost.
Keep physical coordinates, DIPs and source coordinates explicit, and validate authorization and
exact session/target identity at trust boundaries. Do not add obsolete peer protocols merely to
stay connected; hardware fallback, same-protocol reconnect and local data recovery are separate contracts.

| Your change | Verification to include in the PR |
| --- | --- |
| Documentation or translation | Check affected links, anchors, examples and language consistency; run the documentation contracts that consume those files. |
| Desktop UI | Native component and input regressions; describe visible behavior and verify the affected workflow. |
| Native capture or desktop input | Relevant Rust tests, plus the applicable Windows runtime verification. |
| Packaging or dependencies | Follow [release provenance](docs/release/release-provenance.md) and [dependency-security gates](docs/security/DEPENDENCY_SECURITY.md). |

Run the smallest applicable test first. `npm run verify:local` runs version consistency,
strict effective-line limits, Rust formatting, workspace tests and the locked release build.
The [build guide](docs/development/WINDOWS_NATIVE_BUILD.md) lists targeted tests, packaging and startup probes.
Documentation-only changes need content/link checks and the relevant documentation contracts, not an EXE rebuild.
The CI workflow skips the Windows build only for its known documentation-only paths; all other changes
must pass same-run native candidate gates before an artifact is promoted. The stable check remains
`build-windows-exe`, and dependency/security analysis is independent.

Report the checks actually run. Compilation, synthetic fixtures and package startup do not
substitute for ordinary-desktop, physical-input, mixed-DPI or cross-device acceptance.
Use [versioning and provenance](docs/release/release-provenance.md#versioning) for build identity,
[dependency security](docs/security/DEPENDENCY_SECURITY.md) for dependency changes, and
[signing policy](docs/release/CODE_SIGNING_POLICY.md) for any separately approved public release.

## Review and follow-up

Keep the pull request description focused on the problem and the evidence for the
solution. Link the relevant issue when one exists. Maintainers may ask for a smaller
scope, additional tests or documentation changes before merging. Do not combine a
public release, version-tag change or external deployment with an ordinary fix
unless that work has been agreed separately.
