## Native action feedback. Diagnostic details and user content are passed through unchanged.
action-new-window-failed = Cannot create a workspace window.
    { $error }
action-new-pin-failed = Cannot create a pin. The original document is unchanged.
    { $error }
action-hotkey-failed = Cannot change the Ctrl+1 capture shortcut. Another application may be using it.
    { $error }
action-hotkey-degraded = Global capture ({ $shortcuts } unavailable)
action-hotkey-menu = Global capture / Live / scrolling capture
action-hotkey-suspended = Capture shortcuts suspended in full screen
capture-loupe-keys = Tab: format · Z: hide
action-shortcut-settings = Configure shortcuts in Loom…
action-shortcut-settings-guide = Connect this Otter device to Loom. In Loom, open Settings → Otter → Shortcuts, choose this device, and apply the bindings. Otter keeps its last saved bindings when disconnected.
action-shortcut-import-conflict = Shortcut settings differ from this profile. Change shortcuts through Loom, not by importing a naming file.
action-open-failed = Cannot open the image. The original document is unchanged.
    { $error }
action-replace-one-image = Replacing an image requires exactly one image. The original document is unchanged.
action-replace-editable = Editable content cannot replace the current image.
action-replace-stale = The document changed while loading. Replacement was cancelled; open the image again.
action-import-failed = Cannot import images. The original document is unchanged.
    { $error }
action-drop-count = Drop between 1 and 32 images at a time.
action-drop-path = The dropped file path is empty or too long.
action-drop-incomplete = Cannot read the complete dropped file path.
action-image-loading = Images are still loading. Wait or press Esc to cancel.
action-open-paths-invalid = File selection is incomplete, contains an invalid path, or exceeds the batch limit.
action-flip-failed = Flip was not applied. The original image and annotations are unchanged.
    { $error }
action-erase-failed = Erase was not applied. The original document is unchanged.
    { $error }
action-crop-failed = Crop was not applied. The original frame and effects are unchanged.
    { $error }
action-resize-failed = Resize was not applied. The original annotations are unchanged.
    { $error }
action-rasterize-failed = Rasterization was not applied. The original annotations are unchanged.
    { $error }
action-blur-failed = Blur preparation failed. The original image and annotations are unchanged.
    { $error }
action-text-failed = Cannot start text input.
action-text-draft-rejected = Text was not saved. The draft exceeds the text limits or the available memory budget.
action-text-draft-unavailable = Cannot display this draft. Text is still in the editor.
    { $error }
action-font-failed = Cannot apply the font.
    { $error }
action-cache-failed = Cache settings were not changed.
    { $error }
action-copy-unavailable = Cannot schedule copying. The clipboard is unchanged.
action-copy-failed = Copy did not complete.
    { $error }
action-paste-failed = Cannot paste the image. The original document is unchanged.
    { $error }
action-paste-no-image = The clipboard contains no usable image.
action-paste-one-file = Replacing an image requires exactly one clipboard file.
action-clipboard-files-invalid = The clipboard file list is invalid or exceeds the 32-file batch limit.
action-crop-selection = Crop selection
action-reset-crop = Reset crop
action-rasterize-selection = Rasterize selected annotations
action-rasterize-all = Rasterize all annotations
action-rasterizing = Rasterizing…
action-receive-upstream = Receive upstream images
action-inline-text = Enter annotation text
action-zoom-preview = Otter · Zoom preview
action-serial-radius = The sequence radius must be 8–999 image pixels. Settings were not changed.
action-session-unsaved = The workspace has not been saved safely. Complete the operation or exit Otter first.
action-configuration-changed = Node configuration changed; the previous preparation was cancelled.
action-hdr-unrestored = HDR (original file not restored)
action-capture-unrecorded = Not recorded
action-untitled-window = Untitled window
action-window-limit = Only the first 64 windows are shown
action-settings-recovery-title = Otter settings recovery
action-settings-recovery = The original settings file is missing; the only recovery copy was loaded.
    Original path: { $target }
    Recovery copy: { $recovery }
    No file was changed. Use Save As in naming settings; direct overwrite is disabled.
action-naming-import-failed = Naming settings were not imported.
    { $error }
action-naming-save-failed = File and cache settings were not saved.
    { $error }
action-naming-save-warning = File and cache settings were saved, but the old recovery file could not be removed: { $path }
    { $error }
action-naming-autosave-failed = Automatic saving stopped. Current settings are retained; resolve the error before enabling it again.
    { $error }
action-naming-load-first = Load a profile or save settings as a new file before enabling automatic saving.
action-naming-recovery-save = Recovered settings require Save As before automatic overwrite can be enabled.
action-naming-no-source = No settings file is loaded. Import one or start with --settings.
action-naming-recovery-update = Settings were loaded from a recovery copy. Use Save As; the recovery file will not be overwritten.
action-naming-new-file = Choose a new file. Save As does not overwrite existing files.
action-naming-import-title = Import settings (this session only; source file unchanged)
action-naming-save-title = Save file and cache settings as a new file
action-live-control-failed = Live source control is unavailable. Captured frames are retained.
    { $error }
action-live-stopped = Live stopped.
    { $error }
action-live-retain-failed = The Live frame was not retained.
    { $error }
action-live-start-failed = Cannot start Live.
    { $error }
capture-loupe-hex = HEX · 8-bit sRGB
capture-loupe-rgb = RGB · 8-bit sRGB
capture-loupe-hsl = HSL · sRGB
tray-delay = Delayed capture
tray-delay-three = Capture in 3 seconds
tray-delay-five = Capture in 5 seconds
tray-delay-cancel = Cancel delayed capture
capture-countdown-badge = Capture in { $seconds } s
archive-menu = Automatic image archive
archive-copy = Archive copied images
archive-pin = Archive new screenshot pins
archive-directory = Choose archive folder…
archive-select-directory = Choose an existing local archive folder first.
archive-config-failed = Archive settings were not changed: { $error }
archive-consent = During this run, Otter will save PNG images in { $directory } without overwriting files. These are permanent files, not clipboard cache. Enable?
archive-queue-full = The image archive queue is full.
archive-failed = Image archive failed: { $error }. The original operation is unaffected.
paste-card-invalid = Clipboard text must contain valid text, at most 65,536 UTF-16 units and 400 lines.
paste-card-budget = This text card exceeds the available layout or image budget.
edit-rect-mask = Rectangular mask
edit-text-wrap = Wrap text in a box
edit-box-width = Box width
edit-box-padding = Box padding
export-webp-lossless = WebP (lossless)
tray-token-elevated = Current session: running as administrator
tray-token-standard = Current session: not elevated
tray-token-unknown = Current session: unable to read elevation status
loom-needs-standard-session = Exit Otter and open it normally to use Loom or device projection.
export-webp-lossy = WebP (lossy)
export-quality-webp = WebP quality: { $quality } · Alpha is preserved losslessly.
export-quality-lossless = Lossless · Original transparency is preserved.
export-quality-white = JPEG quality: { $quality } · Transparency is composited onto white.
export-size-pending = Encoding preview…
export-size-exact = Encoded size: { $bytes } bytes
export-size-failed = Preview unavailable: { $error }
export-extension-mismatch = The filename extension does not match the selected image format.
tool-spot-rect = Rectangular spotlight
tool-spot-round = Rounded spotlight
tool-spot-ellipse = Elliptical spotlight
tool-spot-density = Spotlight dimming
tool-magnifier = Detail magnifier
tool-lens-zoom = Magnification
