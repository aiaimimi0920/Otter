# HDR Capture

Otter uses an on-demand HDR path for ordinary region capture on Windows 11.

## Capture policy

- `auto` (default): when Windows reports that HDR is enabled for the capture display, Otter requests one transient `R16G16B16A16Float` Windows Graphics Capture frame.
- If the captured scRGB content exceeds the current Windows SDR white level, Otter writes a 16-bit RGB PNG encoded as BT.2020/PQ.
- If the content is SDR-only, Otter converts it to 8-bit sRGB instead of keeping a misleading HDR container.
- If an attempted HDR capture fails with a retryable error, Otter tries one transient BGRA8 WGC capture of the same region, then falls back to GDI if that attempt also fails. Cancellation or a changed monitor binding ends the request instead of capturing again.
- Long capture remains SDR so repeated sampling never creates HDR GPU sessions or adds HDR conversion work to the hot loop.

The optional `OTTER_CAPTURE_DYNAMIC_RANGE` environment variable accepts `auto`, `hdr`, or `sdr`. `hdr` preserves a successful float capture as HDR even when the content itself does not exceed the SDR white level. It still falls back safely when HDR is unavailable. `sdr` disables the HDR attempt.
See the [native capture entry point](../../native/src/desktop/capture_input_dynamic.rs) for how this override is read.

## Output metadata

HDR PNG files contain:

- 16-bit RGB samples;
- BT.2020 color primaries;
- SMPTE ST 2084 (PQ) transfer metadata via PNG `cICP`;
- mastering display metadata (`mDCV`);
- content light level metadata (`cLLI`).

The capture response also reports `dynamicRange`, `bitDepth`, `colorSpace`, `captureBackend`, and `downgradedFromHdr`.

## Resource lifetime

Each WGC attempt has its own bounded frame pool and capture session. They close after the selected pixels are read back or when the attempt fails. The default path never stores a persistent HDR or full-screen WGC session. This preserves Otter's existing memory and idle-CPU performance boundary.
