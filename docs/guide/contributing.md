# Contributing and translating

[简体中文](contributing.zh-CN.md) · [Documentation](../README.md) · [Engineering contributions](../../CONTRIBUTING.md)

Search [existing issues](https://github.com/aiaimimi0920/Otter/issues) before reporting a problem, proposing a workflow or sharing an example.
Include the package version or commit, Windows version, reproduction steps and expected versus actual behavior. Add display layout,
scaling, HDR or Loom version/authorization only if relevant. Remove private images, invitation content and credentials.
Report suspected vulnerabilities through the [private security process](../../SECURITY.md#reporting-a-vulnerability).

## Wording and translation

- Give the page or control location, original wording, proposed translation and reason. Fix a typo in its own language;
  when behavior changes, update its English and Simplified Chinese counterparts.
- Start with the [documentation index](../README.md). Active desktop text is in Rust components, for example the
  [edit-strip catalog](../../native/src/desktop/edit_strip_catalog.rs) and [pin settings](../../native/src/desktop/pin_settings_fields.rs).
  Translating these strings is a code change; neither translated documents nor the OCR translation setting add an interface language.
- Preserve file names, options, identifiers, placeholders and authorization/validation limits. Check local links, images,
  heading anchors and examples. Preserve license text and third-party attributions.

Run the checks relevant to the changed behavior. Do not report desktop or cross-device acceptance from a document preview or fixture.
