# Otter 与 Loom 联合验证

| 要验证的链路 | 使用哪一项 |
| --- | --- |
| 单 Loom、设备或设备组是否出现在目录、自动接收是否实际显示 | [双机验收](#双机验收) |
| 二维码/链接 v2：账号邀请、同机双 Loom PNG 同步、重开和解绑 | [账号 PNG 联测](#账号-png-联测) |
| 单 Loom LAN Live：WGC、H.264、重连、授权撤销和输入释放 | [Live 联合测试](../features/LIVE_JOINT_TESTING.md) |
| 内部包归档及解包后程序启动 | [内部 ZIP 与实际启动](WINDOWS_NATIVE_BUILD.md#内部-zip-与实际启动) |

## 双机验收

在两台 Windows 电脑使用同一当前版本的 Otter；至少一台运行当前 Loom。保持实际业务工作区不变，
使用可丢弃的测试钉图，不公开用户截图、邀请、令牌或私钥。

1. [打开 Loom 局域网连接](../features/LOOM_CONNECTION.md#局域网发现连接码与审批)，确认另一台电脑能访问所选地址、
   两端已配对并获批。测试设备在发送端应可选；收件策略选“接收”。
2. 发送端打开钉图设置 → 投射 → 设备，勾选接收端并点“确认”。若显示“等待”，确认它最终转为“已接收”；核对接收端实际钉图，
   并编辑源图验证更新。用新钉图重试“自动”，再分别核对“拒绝”与“暂停”对新投送和已有投射的区别。
3. 用另一张源图通过二维码或链接导入，再核对更新、重启恢复、解除关联和保留末图。不要拿第一次已用过的邀请绑定第二个设备。
4. 核对断开后设备变离线，重启仍保持手动断开；再次连接复用原设备，避免重复记录。

普通菜单、多屏、混合 DPI、物理输入、跨机与真实外网的结果分别记录。附双方版本和产物摘要、
发送与接收角色、首次缺失的状态及脱敏日志；不能只报“已连接”或发送端的计数。

## 账号 PNG 联测

使用 `scripts/tests/account-projection/run.py`，从当前源码生成测试程序和独立运行目录。

前提：Windows、Python 3.11+、Node、Rust MSVC 工具链、Windows resource compiler、
运行中的 Docker（Linux 容器），以及当前 Otter、Loom、Platform 源码。
先在 Platform 执行 `npm ci`，在 Loom 执行 `cargo build --locked -p loom-daemon`。
两者的业务实现直接从各自仓库复用；本入口不复制服务端实现、不新增依赖。

从 Otter 根目录执行，示例路径与 IP 按当前机器修改：

```powershell
python -B scripts/tests/account-projection/run.py `
  --loom-repo C:/work/Loom `
  --platform-repo C:/work/Platform `
  --daemon C:/work/Loom/target/debug/loom-daemon.exe `
  --lan-host 192.168.1.20 `
  --evidence-root C:/work/test-output `
  --rc 'C:/Program Files/LLVM/bin/llvm-rc.exe'
```

`--lan-host` 必须是本机可绑定的局域网 IPv4 地址。`--cargo-target-dir` 可指定可复用的
Cargo 缓存；未指定时使用 Cargo 默认配置。`--rc` 在资源编译器已配置时可以省略。
每次均由 Cargo 检查当前源码并解析实际测试 artifact，冻结本次测试 EXE，不接受历史测试 EXE。
首次构建可能较慢；输出目录打印在开头。退出码 0 表示联合流程和清理均成功，
`evidence.json` 仅在业务检查、本机 HTTP 服务、Redis 客户端及两个 Loom 的清理均成功后写入；
外层 Redis 容器仍由 Python runner 清理，需另核 `redis-cleanup.json` 和整体退出码。
`artifacts.json` 记录两个 EXE 摘要，日志保留真实失败。

入口拥有一个随机命名、无持久化且仅绑定 loopback 的 Redis 容器和两个隔离 Loom。
账号批准使用两个不同的合成用户，八步 Otter Application 运行在从未切换到前台的私有桌面。
每个 actor 的设备身份、TLS 信任及工作区使用同一 profile，跨步骤和重启保持不变。
外层复用仓库 Win32 Job owner，在失败／超时时收回本次子进程树；结束仅删除本次 Redis
和 Loom 分配的临时 profile，保留 Otter profile、原始结果及日志供诊断。
测试不读取真实账号或工作区。输出含合成设备资料，不应提交到 Git。

此入口验证单物理机上两个 Loom 的 v2 PNG 生命周期；不代替物理双机、Live、普通桌面菜单或真实 OAuth/公网验收。
逐版本的验收回执从 Git 历史和当时的产物核对，不作为当前代码通过的证据。

## 桌面回归

- 普通截图松手出图；已有钉图、拖动、保存及重开不受影响。
- 依次打开编辑栏、设置栏和参数面板，拖动时各附属控件跟随窗口，焦点和未提交设置不丢失。
- 用当前授权的 Loom 验证 OCR、翻译及 Art；目录撤销后不能使用旧动作。记录实际桌面、
  混合 DPI/多屏、输入法及长稳结果，未测的场景不要归为通过。
