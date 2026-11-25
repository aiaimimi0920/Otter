# Display language

## User controls and defaults

Right-click the Otter tray icon and choose the localized **Language / 语言 → English / 简体中文** entry.
The native language names remain recognizable in either interface language.
Otter saves the choice in its existing `app-settings.json` profile and then applies it without
restarting. Active modal operations finish before a queued language change is applied.

A new profile starts in English. Existing settings without `displayLanguage` retain the previous
Simplified Chinese interface; an explicit choice writes canonical `en` or `zh-Hans`.
The persisted `displayLanguageRevision` orders changes; legacy `zh-CN` is read as `zh-Hans`.
Unknown settings fields and OCR `translationTargetLanguage` are preserved.
Save conflicts or permission errors leave the effective language unchanged. A recovered settings
file still requires Save As; changing language does not bypass recovery protection.

Interface language does not translate user text, filenames, fonts, OCR results, diagnostic details,
protocol identifiers or labels supplied by external capabilities. Windows-owned file-dialog
controls follow Windows language settings. Otter-owned captions use the selected language.
Existing event and error snapshots retain the language used when they occurred; subsequent events
use the new language. Switching language never replays business operations to regenerate old messages.
Independent Tile manager/output processes read their selected profile at startup; an already
running separate Tile process is not remotely controlled or live-synchronized by this mechanism.

## Setting one device from Loom

In Loom's Otter Connection settings, select an approved device with an active Otter language
receiver, choose a supported language and submit. This is a one-shot request, not an enduring
Loom preference or override. Otter remains the only owner of the persisted setting.
Changing it later from the tray is allowed. Loom's own UI language remains independent.

- Administration sends `POST /v1/devices/{id}/locale-requests` with a unique `requestId`,
  the displayed `receiverGeneration`, `expectedRevision` and canonical `language`.
- Otter reports through Device-authenticated `POST /v1/otter/locale-report`, using the existing
  fixed-origin TLS and nonce checks. The response can carry one pending request.
- Receivers and requests have a 15-second lifetime; reports normally run every two seconds.
  There is one outstanding request per device, no offline queue and no automatic reissue after conflict.
  A daemon restart discards outstanding requests rather than restoring stale commands.
- Otter bounds execution by both the request lifetime and the original authorization lease.
  Switching services, importing a settings profile or revoking authorization invalidates the old scope.
- `applied` means the settings transaction was saved and the Application owner applied it.
  A missing acknowledgement is **unconfirmed**, not proof of success or rollback.
  A late receipt can confirm a previously unconfirmed request. A later local choice does not
  invalidate evidence that an earlier request was applied at its own revision.

The receiver accepts only supported language values, never arbitrary settings, paths or code.
The LAN route is Device-only; the administrative request route is not exposed on the LAN listener.

## Translation architecture and contribution

Official catalogs under `native/assets/locales/{en,zh-Hans}` are UTF-8 Fluent resources embedded
at compile time. This keeps language selection offline and avoids a downloader, package signatures,
update coordination and executable-package permissions for a small set of text resources.
Language contributions remain normal reviewed source changes.

`ui_text` caches static labels in immutable catalog storage. Dynamic messages use named arguments;
Fluent handles language-specific selection/plurals and preserves directional isolation.
English is the complete message/argument contract and the missing-translation fallback.
The bundled translations are checked for completeness. A malformed embedded catalog or invalid
call is a development error, not silently replaced by a raw message ID.

Translations must not become state-machine keys. Presentation refresh must preserve window identity,
draft values, selection and authorization snapshots; language is not a business-configuration change.
Do not add a new locale to the advertised list before its catalog, controls, font/layout behavior
and both entry paths have been verified.

## Verification

Focused Rust coverage includes `ui_text_tests`, `app_settings_document_tests`,
`application_locale_tests`, `application_locale_remote_tests`, `loom_locale_protocol_tests` and
`loom_locale_http_tests`. Final publication guards also have settings-file rollback regression tests.
Loom maintains receiver-state, route-authentication and frontend request-correlation tests.

Automated fixtures do not replace ordinary desktop acceptance: check the tray, open panels with
unsaved text, tooltips/accessibility names, long translations at multiple DPIs, restart persistence,
local/remote revision conflicts and disconnected devices. Do not infer physical cross-machine
acceptance from loopback HTTP or hidden-window tests.
