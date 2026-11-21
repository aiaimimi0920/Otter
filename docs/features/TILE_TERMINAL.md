# Tile terminal

Tile runs from the same Otter executable but uses dedicated wall mode. It does not start the ordinary capture workspace.
`native/src/tile_outputs.rs` lists physical output identities; each selected output is independently verified.
`native/src/tile_live_packet.rs` consumes the wall media protocol and applies frame and dimension bounds.

Use the CLI help shipped with the exact executable and the matching Loom wall configuration. A tile test fixture
or isolated output query does not establish multi-monitor placement, wired display behavior or remote control.
See the [wall client](TILE_WALL_CLIENT.md) and [development guide](../development/WINDOWS_NATIVE_BUILD.md).
