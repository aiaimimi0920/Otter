# Tile wall terminal

Tile is a separate wall mode of the same `otter-native.exe`; it does not start the normal capture
workspace and is neither an ordinary projection pin nor a LiveRelay viewer. Configure the wall
and its authorization in the current Loom first, then select an output on the Otter machine.

```powershell
.\otter-native.exe --tile-outputs
.\otter-native.exe --tile --loom $AuthorizedLoomOrigin
.\otter-native.exe --tile-output $MonitorId --loom $AuthorizedLoomOrigin --profile $AbsoluteTileProfile
```

`--tile-outputs` lists available output IDs; `--tile-output` takes one of those `monitor-…` IDs.
`--loom` specifies an explicit Loom origin. For testing, use `--profile` with a separate absolute
directory rather than a real user profile. [`tile_cli.rs`](../../native/otter-runtime/src/tile_cli.rs)
defines the arguments; `tile_*` modules validate wall sessions, output identity and managed presentation.
Wall media (`loom.wall.media.v1`) is separate from ordinary Live (`NLLV`). Each entry point validates
its own frame structure, ordering, dimensions and authorization, rather than inferring them from
the primary monitor or another device. Loom's wall documentation and wire protocol define server behavior and deployment.

## Verification boundaries

Run focused CLI argument validation without opening a visible wall:

```powershell
cargo nextest run --locked --manifest-path native/Cargo.toml -p otter-native --lib -E 'test(tile_tests::cli_rejects_ambiguous_modes_relative_profile_and_untrusted_origin)'
```

This case in [`tile_tests.rs`](../../native/src/tile_tests.rs) checks mutually exclusive modes,
relative profiles, untrusted origins and other arguments. Both product cases in
[`tile_product_runtime.rs`](../../native/tests/suites/tile_product_runtime.rs) are `#[ignore]`
visible gates; ordinary `cargo test ... tile_product_runtime::` does not execute them.
To run a visible gate, explicitly set `OTTER_TILE_VISIBLE_GATE=1`, prepare an isolated Loom fixture
and `OTTER_TILE_JOINT_INPUT`, and bind the ordinary product EXE under test. The gate briefly displays
its own full-screen color blocks and verifies pixels and stopping.
Passing it does not establish acceptance of real multi-monitor layouts, external display paths
or cross-device wall control.
