# Capture, annotation and display export

These controls are in the internal development tree; this is not a public release or
a hardware acceptance record.

* **Built-in shortcuts:** Choose the authorized Otter device in Loom, edit its
  policy, and wait for the device's reply. Queued is not applied. Otter reads the
  saved policy locally on its next offline start. The master switch is separate;
  full-screen game suppression is off by default. Delay, restore input and toggle
  shortcuts have no default global binding.
* **Region selection:** Ordinary capture still commits on mouse release. `Z` toggles
  the pixel loupe; `Tab` selects HEX/RGB/HSL, and `C` copies the current colour.
  Frozen selection retains `Ctrl+C` for the selected image. In the pin colour picker,
  clicking assigns a colour instead of copying. `R` recalls the last successfully
  delivered ordinary region in this run. The tray offers 3 s / 5 s delay and cancellation.
* **Archive:** Copy and ordinary-capture archiving are separate, initially off.
  The tray shows the destination before enabling either option. It defaults to
  `saved/` under the profile selected at startup; that subfolder is created only
  after confirmation. Without a profile, choose an existing fixed-local folder.
  Pending images keep their authorized directory even if you change the destination.
  If the selected path no longer identifies that directory, the write fails;
  an existing PNG is never replaced. An unsuccessful archive does not undo
  the copy or close the new pin. Ordinary exit waits for admitted archive writes;
  an already saved workspace does not allow system logoff while an output is
  pending. Archiving is not a copy of editable workspace history.
* **Static editing:** Rectangular Blur/Mosaic use the same effects as their brush
  counterparts. Text boxes have wrapping and padding; serial arrows keep the number's
  circle separate from its tip. Plain Unicode text pasted as a *new pin* becomes
  editable text; existing image and file formats take precedence. A malformed image
  on the clipboard is not silently interpreted as plain text.
  A fixed-width draft and its exported text use the same DirectWrite line
  breaks. Clicks and line navigation use those breaks, not the native text
  buffer's visual line numbers. Native clipboard and undo remain available.
* **Spotlight and magnifier:** Add rectangular, rounded or elliptical openings to
  one shared mask (20–90% darkness, initially 60%). The magnifier has separate
  source and destination handles (150–800%, initially 250%). It samples a separate
  surface drawn after erase, blur and mosaic, before the mask and other magnifiers.
  If that surface cannot be obtained, preview skips the magnified content and a
  flattened export fails rather than sampling the original.
  Rasterization retains these dynamic layers. While either is present, ordinary
  annotations above them also remain editable; only lower highlighter/redaction
  layers can be rasterized, preserving mask order and the magnifier's sample.
* **Mouse pass-through:** On a static pin, finish or cancel any active edit first.
  Use the tray's restore action even if the optional restore shortcut is unavailable.
  The shortcut restores the topmost matching pin under the pointer, or all
  pass-through pins if there is no match. Live and Art use different input paths.
* **Desktop layout:** Select pins or a group to align, compact, distribute or lay out
  a grid within each monitor's work area. Distribution needs at least three pins per
  applicable group. Layout undo does not move a pin that was independently moved
  after the operation. The tray can restore pins temporarily hidden by a shake.
* **Export:** Export the current display as PNG, white-backed JPEG, lossless WebP or lossy
  WebP with lossless alpha. A quality of 100 is still a lossy format, not a switch
  to lossless mode. The byte count describes the current settings; wait for it before
  confirming. HDR original export is separate: it exports source data, **not** a
  redacted image. Editable workspaces retain source data too. For sensitive content,
  remove the source pixels in the flattened output, or use coarse rectangular mosaic
  and inspect the result. Blur is not irreversible; neither the editable workspace
  nor an HDR original should be sent as a redacted copy.
* **Names:** Only explicitly enabling `{sourceApp}` or `{windowTitle}` in the local
  template requests that field at capture confirmation. A window title can contain
  a document name or page title; a file name you share can therefore disclose it.
  Missing data falls back to `Screen` / `Untitled`. Captured source names are not
  embedded as image metadata or sent as projection metadata.

## Administrator capture

Double-click `otter-native.exe` for normal use. For a local administrator window,
choose **Exit** from Otter's tray, then right-click the same EXE in Explorer and
choose **Run as administrator**. Confirm Windows' prompt and use the usual capture
shortcut. The tray shows the current process's actual privilege. Exit and
double-click again for normal use. There is no second capture program, installation
step, elevated sign-in option or automatic relaunch. Ordinary sign-in startup is
still an explicit tray setting; neither kind of launch changes it.

Use a trusted copy of the whole portable package. Workspace ownership is still
exclusive; starting another instance does not take over a running desktop. Windows
may block drag-and-drop from ordinary Explorer into the elevated window; use
**Open image** instead. Use a normal session for Loom, extensions or device projection.

**Manual check:** Using a disposable profile, confirm normal tray status, exit,
cancel a right-click launch at UAC, then repeat and accept. Verify administrator
status and capture a non-sensitive administrator window with `Ctrl+1`. Exit and
double-click again: the workspace and ordinary tray status should return.
Compare `otter-native.exe --check-login-startup` before and after; the ordinary
Run setting must not change. The hidden package probes do not exercise UAC.

## Read-only workspace checkpoint check

`otter-native.exe --check-restart-checkpoint REVISION SHA256` checks the
published workspace checkpoint without starting a resident desktop. Supply an
explicit, existing test profile via `OTTER_NATIVE_PROFILE_DIR`, a positive
decimal revision and the lowercase digest of the *previously published* bytes.
The process must acquire the existing exclusive `workspace.lock`; it never
creates a missing directory or lock. It pins the original checkpoint, checks
the digest, restores those same bytes into hidden windows, checks the restored
revision and paints without registering capture keys. It will not substitute a
recovery copy. A digest from the caller is not proof of a trusted caller or
authorization to launch, register a task or release another process's lock.

## Verifying text paste without overwriting a user's clipboard

The default tests use an owned Win32 memory block or an injected text snapshot.
The checkpoint test runs the worker, native inline editor and on-disk reopen on
a private desktop. A private **desktop** does not isolate the system clipboard:
real cross-process `WM_COPY` is a separate acceptance test on a fresh private
**window station**. In an isolated test VM whose account can create stations, run:

```powershell
cargo nextest run --locked --manifest-path native/Cargo.toml -p otter-native --lib --features test-loom --run-ignored only -E 'test(external_editor_clipboard_creates_a_new_editable_text_pin)'
```

`CreateWindowStationW` returning `E_ACCESSDENIED` means the real clipboard test
could not start. Never fall back to `WinSta0` or change its permissions.

## Checking mouse delivery in an isolated Windows VM

1. Open [the local receiver](../../scripts/tests/pin-input-receiver.html) in a
   separate browser process. Confirm its process is not the Otter candidate's
   process. Move, pause, click, drag and scroll on the uncovered blue pad first:
   the page should count all five. Click **Before** to reset that row.
2. Cover the *whole pad*, but not the phase buttons or report, with an opaque,
   contrasting static pin. Repeat those gestures by hand. Choose **Pass-through**,
   enable the mode on that pin and repeat. Choose **Restored**, restore via
   the tray and repeat. Keep the pointer inside the covered pad during the drag.
3. Save the three rows and a desktop screenshot in each phase, with the cursor
   away from the sampled point: the same contrasting pin pixel must remain
   visible in all three. A stationary pause after a delivered move increments
   `hover`; `drag` requires a down, held move and up. These are browser-delivered
   events, not a count of Win32 messages. If wheel events also reach the covered
   pad in the baseline, that run is inconclusive for wheel delivery.
4. Repeat on the actual DirectComposition and software backends and record
   which was used. A backend fallback, `WindowFromPoint`, a synthetic event,
   or a passing unit test is not a physical-input receipt. Repeat tray restore
   with the optional restore shortcut unbound.

## Reproducing the lossy WebP budget probe

From the repository root, after `cargo fetch --locked --manifest-path native/Cargo.toml`,
use the source path reported by Cargo for the locked encoder. The probe compiles only
that crate (no optional features) and the allocation meter, not a second Cargo workspace.

```powershell
$m = cargo metadata --locked --offline --manifest-path native/Cargo.toml --format-version 1 | ConvertFrom-Json
$p = @($m.packages | Where-Object { $_.name -eq 'tiny-webp' -and $_.version -eq '0.1.1' })
if ($p.Count -ne 1) { throw 'Expected exactly one locked encoder' }
$src = ($p[0].targets | Where-Object { $_.kind -contains 'lib' }).src_path
$out = New-Item -ItemType Directory -Force .tmp/webp-admission
$lib = Join-Path $out.FullName libtiny_webp.rlib
$exe = Join-Path $out.FullName webp-encoder-budget.exe
rustc --edition 2021 --crate-name tiny_webp --crate-type rlib -C opt-level=3 $src -o $lib
if ($LASTEXITCODE) { throw 'Encoder build failed' }
rustc --edition 2021 -C opt-level=3 scripts/tests/webp-encoder-budget.rs --extern "tiny_webp=$lib" -o $exe
if ($LASTEXITCODE) { throw 'Probe build failed' }
& $exe
if ($LASTEXITCODE) { throw 'Encoder admission probe failed' }
```

The output reports elapsed encoding time, RIFF size and peak live allocator bytes
including input, for deterministic opaque/transparent/noisy frames. This is not a
measurement of total process working set, UI latency or the full export pipeline.
