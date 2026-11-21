# 共用 Loom 的两台电脑

Loom 所在电脑运行 Loom 和 Otter，另一台电脑只运行 Otter。两端使用当前原生版本。

在 Loom 电脑导出公开连接资料：

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/loom-connection/Export-LoomConnection.ps1 -OutputDirectory C:/path/to/new-connection
```

输出包含地址、Loom ID、公有 TLS 证书和导入脚本，不含管理令牌、私钥或设备身份。
将输出目录复制到 Otter 产品包 `payload` 的同级目录；在另一台电脑退出旧 Otter，
双击该目录内 `Connect-Otter.cmd`。脚本将配置导入当前用户 `%LOCALAPPDATA%/OtterNative`，
保留已有配置备份，然后启动同级 `payload/otter-native.exe`。错误返回非零并保留终端。

首次原生身份需要在 Loom 的设备管理中批准；等待审批时最多每 30 秒重试一次。批准后自动获取会话、接收轮询、发现同一 Loom
上的可接收设备；之后直接启动 Otter 即可恢复连接和提前续签。暂停接收或禁止投射的设备
不会被当作可投射目标。显式清除会话同时暂停自动连接，重启保持暂停，菜单续签或重新导入解除。

独立导入命令也支持显式 `-ConnectionFile`、`-ProfileDirectory` 和可选 `-Executable`。
只允许导入可信来源的连接文件；目标变化时重新从正确的 Loom 导出。
不要复制 `loom.json`、`device-identity.json` 或整个用户 profile。
