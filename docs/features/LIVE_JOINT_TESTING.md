# 原生 Live 联合测试

长期入口是 `scripts/tests/live-joint/run.py`。它从当前 Otter 和 Loom 源码构建测试程序，
串行复用 Cargo 缓存，按本轮 Cargo artifact 冻结测试 EXE，并把运行证据写到新目录。
不读取临时目录中的历史测试脚本或固定测试 EXE；清空缓存后可重新构建。

需要 Windows、Python 3.11+、Rust MSVC 工具链、Windows resource compiler，以及当前
Otter、Loom 和 Otter 路径依赖的 NeuroDesign checkout。以下命令从 Otter 根目录执行，
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

同一发布会话的帧尺寸可以变化。发送端在原媒体线程释放旧编码器后，按新尺寸创建编码器，
并从新关键帧开始；已失败的编码器不会因 resize 自动恢复。Loom 和观看端沿现有帧头尺寸
清理旧缓存、重建解码器和位图。会话创建描述保留初始尺寸，不能把目录中的尺寸当作实时值。

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
公网、真实外网 NAT 和 relay 验收按当前用户确认范围暂缓。

## 当前验证记录

2026-11-20，v0.2.33.667 产品源码及随后补齐地址接线的测试在本机局域网地址上完成
全部 7 个联合场景及独立 WGC 生命周期场景，8 项均通过。真实 WGC 联合确认
64×64 → 96×80 的 H.264 关键帧、
观看端新尺寸和改色后的 GPU 像素，以及停止后的源恢复、捕获回收和媒体连接数归零。
独立生命周期场景覆盖 12 个会话、36 次尺寸变化和 24 次输入重启；这些循环数不是测试数。
Direct 联合还确认两帧更新、撤权后原生窗口／媒体资源回收，以及 LAN listener 重启后
源端可重连而已撤权接收端仍被拒绝。发送／观看、系统 codec、Loom 连续性、指定接收者
隔离及 Application 停止／退出另有 25 项定向回归通过，其中本机模拟 peer 不是 LAN 服务证据。
该结果沿用上述单 Loom、同机原生测试进程的范围，未关闭双 Loom、物理双机或普通 GUI 验收。
