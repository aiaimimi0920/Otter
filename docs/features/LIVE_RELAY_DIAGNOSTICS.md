# Live relay diagnostics

1. Record the exact Otter and Loom revisions, package digest, topology, source and viewer roles,
   dimensions, selected media representation and whether local WGC is involved. Do not record tokens or user pixels.
2. Separate discovery, authorization, control-plane state, network frame receipt, decode and display.
   A successful WebSocket upgrade is not receipt of a valid frame; the source recovery watermark is service-confirmed.
3. On a reconnect, record session/epoch, progression of frame IDs, decoder recovery and actual presentation.
   A changed stream size must establish a fresh valid decode chain. Distinguish codec fallback from peer-version fallback.
4. Use [Live joint testing](LIVE_JOINT_TESTING.md) and its retained process receipts. Wall-clock timing
   does not infer GPU time, and isolated LAN or fixture results do not establish physical cross-device behavior.
