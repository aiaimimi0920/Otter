# Tile wall client

Tile is a distinct current application mode for a configured Loom wall, not a browser preview.
The `native/src/tile_*` modules validate the wall session, output identity, managed presentation and media.
The wall media envelope (`loom.wall.media.v1`) is independent of the normal Live `NLLV` path:
shape, frame ordering, named output and permission are not inferred from the primary monitor or another device.

See [Tile terminal](TILE_TERMINAL.md) for local startup and the Loom wall protocol in the Loom repository
for service expectations. Ordinary multi-monitor and hardware acceptance must be recorded separately.
