# 原生 Live 联合测试

长期入口是 `scripts/tests/live-joint/run.py`。它从当前 Otter 和 Loom 源码构建测试程序，
串行复用 Cargo 缓存，按本轮 Cargo artifact 冻结测试 EXE，并把运行证据写到新目录。
不读取临时目录中的历史测试脚本或固定测试 EXE；清空缓存后可重新构建。

需要 Windows、Python 3.11+、Rust MSVC 工具链、Windows resource compiler，以及当前
Otter、Loom 源码。NeuroDesign 使用 Cargo 锁定的 Git 提交，不需要相邻 checkout。
以下命令从 Otter 根目录执行，
路径与局域网 IP 按当前机器修改：

```powershell
python -B scripts/tests/live-joint/run.py `
  --loom-repo C:/work/Loom `
  --lan-host 192.168.1.20 `
  --evidence-root C:/work/test-output `
  --cargo-target-dir C:/work/cargo-cache `
  --rc 'C:/Program Files/LLVM/bin/llvm-rc.exe' `
  --with-wgc
```

输出目录建议位于专门的一次性测试目录，例如 `linshi`；其中的身份、profile、日志和构建
产物均可再生。测试不使用真实账号或工作区。Win32 Job 负责有界运行、失败和超时后的本次
进程树回收。测试库的临时目录清理只针对本次 `TEMP/TMP` 内的自建夹具。

默认依次执行：

| 场景 | 检查内容 |
| --- | --- |
| `wgc-h264` | 自有窗口真实 WGC → LAN H.264 → 原生观看端；改变颜色和尺寸后核验像素，停止后恢复源窗口 |
| `h264` | 当前 H.264 编码、HTTPS/WSS、系统解码、原生 GPU 像素及关键帧请求 |
| `viewer-reconnect` | 观看端媒体连接断开后保持窗口、收到更新帧并回收旧连接 |
| `control-reconnect` | 控制 GET 连接断开后的顺序恢复，不自动取得输入控制 |
| `source-reconnect` | 发送端媒体连接断开后的重接和观看端更新帧 |
| `input-release` | 当前 RawBgra Live 帧、两次键鼠控制及菜单／失焦释放 |
| `direct-revocation` | 指定接收端、两帧更新、撤权回收、LAN listener 停止／重启后拒绝已撤权设备 |

可重复传 `--case` 只运行指定场景。`--with-wgc` 另外验证真实 WGC 的 12 个会话、
36 次尺寸变化、24 次输入重启与源窗口恢复；它会短暂显示自有测试窗口，不发送系统输入。
也可以用 `--case wgc-cycles` 单独复验此项。WGC 场景会一并构建当前源码的
`otter_live_hover_hook.dll` 并放在冻结测试 EXE 旁，不能只复制测试 EXE。
H.264 联合场景不接入旧客户端，不以旧格式互操作或版本降级作为验收要求。

每个联合场景运行一个真实 Loom 服务夹具，媒体连接实际经过 `--lan-host` 指定的本机
RFC1918 局域网 IPv4。除 `direct-revocation` 外，均使用两个 Otter Application 测试进程；
`wgc-h264` 使用真实 WGC，其余使用合成 GPU 纹理。`direct-revocation` 使用一个 Otter
测试进程，通过生产 Relay Session 观看测试客户端发送的 RawBgra 帧；它不验证 source
Application 菜单。必须区分完整捕获传输、媒体故障恢复、授权和本地生命周期。

`evidence.json` 包含各项结果和明确边界；`artifacts.json` 记录两个测试程序 SHA-256。
H.264 的 wire 帧号与 viewer 的解码／GPU 上传帧号对应，输入测试由源窗口消息作独立核验，
最终要求角色、媒体连接和后台 owner 回收。测试 EXE 不作为普通产品交付。

这些测试不证明两个独立 Loom 之间的 Live、物理双机、普通产品 GUI 或真实 OAuth。
账号 v2 双 Loom 目前只有 PNG 分享链，详见 [二维码与链接投射](QR_PROJECTION.md)。
公网 NAT 或 relay 的能力与实际验收须另行验证。



## 单机 WinForms 输入门禁

在已解锁的交互式 Windows 桌面，从 Otter 仓库根目录编译仓库内的专用夹具并运行显式忽略用例：

```powershell
$evidence = Join-Path ([IO.Path]::GetTempPath()) ("otter-winforms-" + [guid]::NewGuid())
[void][IO.Directory]::CreateDirectory($evidence)
$csc = Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
$env:OTTER_NATIVE_WINFORMS_FIXTURE = Join-Path $evidence 'fixture.exe'
$env:OTTER_NATIVE_WINFORMS_EVIDENCE_ROOT = $evidence
$source = (Resolve-Path -LiteralPath 'scripts/tests/fixtures/LiveInputWinFormsFixture.cs').Path
& $csc /nologo /target:winexe "/out:$env:OTTER_NATIVE_WINFORMS_FIXTURE" /reference:System.Windows.Forms.dll /reference:System.Drawing.dll $source
if ($LASTEXITCODE -ne 0) { throw "WinForms fixture compilation failed: $LASTEXITCODE" }
cargo test --locked --manifest-path native/Cargo.toml --lib actual_managed_winforms_click_drag_cancel_and_restore -- --ignored --test-threads=1 --nocapture
```

用例只控制自己启动的 CLR 进程。`ready.json` 和 `state.json` 保存在每次运行的独占子目录，失败也保留；
不指定产物根时用系统临时目录。这个门禁验证自有控件的真实点击、拖动、取消和窗口样式恢复，
不发系统鼠标输入，不覆盖第三方应用、产品 UI 或跨机流程。

## 诊断与证据边界

- 记录双方版本或提交、包摘要、网络拓扑、源/观看端角色、尺寸、实例数、运行时长和媒体格式；
  不记录令牌或用户像素。性能报告需覆盖 Otter、hook、Loom 与辅助进程，而不是只看主进程内存。
- 分开核对发现、授权、会话状态、收包、解码、GPU 上传和呈现。WebSocket 成功升级不代表已收到
  有效帧；重连看会话、epoch、帧号进展和实显，恢复水位以服务端确认值为准。
- GPU 像素回读、墙钟耗时、收包和解码计数不能推导显示器 FPS、GPU 时间或物理输入延迟。
  普通桌面、混合 DPI、物理双机和真实外网各自留证。逐版本结果从 Git 历史和对应产物查阅。
