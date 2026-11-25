# Tile wall terminal

Tile 是同一个 `otter-native.exe` 的独立墙面模式，不启动普通捕获工作区；它不是普通投射贴图
或 LiveRelay 观看端。先在当前 Loom 配置墙面及授权，再选择 Otter 所在机器的输出。

```powershell
.\otter-native.exe --tile-outputs
.\otter-native.exe --tile --loom $AuthorizedLoomOrigin
.\otter-native.exe --tile-output $MonitorId --loom $AuthorizedLoomOrigin --profile $AbsoluteTileProfile
```

`--tile-outputs` 列出可用输出标识；`--tile-output` 使用其中一个 `monitor-…` 标识。
`--loom` 指定明确的 Loom origin；测试时使用 `--profile` 选择独立的绝对目录，不复用真实用户目录。
[`tile_cli.rs`](../../native/otter-runtime/src/tile_cli.rs) 定义参数，`tile_*` 模块核对墙面会话、输出身份和受管呈现。
墙面媒体 `loom.wall.media.v1` 与普通 Live 的 `NLLV` 独立，帧形状、顺序、尺寸和授权均由各自入口校验，
不从主屏幕或另一设备推断。服务端行为及部署以 Loom 的墙面文档与线协议为准。

`cargo test --locked --manifest-path native/Cargo.toml --test integration tile_product_runtime::` 验证 CLI 和隔离产品合同；
不代替真实多屏布局、外接显示链路或墙面控制验收。
