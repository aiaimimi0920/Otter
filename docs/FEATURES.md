# Otter 功能与快捷键 / Features and shortcuts

当前主干；历史下载以自己的发行说明为准。涉及 Loom 的功能取决于当前设备的授权、能力目录和安装状态。

| 功能 / Feature | 操作 / Controls |
| --- | --- |
| 区域、实时和长截图 / Region, Live and scrolling capture | `Ctrl+1` / `Ctrl+2` / `Ctrl+3` |
| 钉图编辑 / Pin editing | `Ctrl+E`；裁剪、画笔、文字、形状、模糊、马赛克；`Ctrl+Z`/`Ctrl+Y` 撤销／重做 |
| 参数与节点 / Parameters and nodes | `Tab` 展开参数，`Shift+1` 打开 Art 菜单；`F6` 导航端点、`Ctrl+Enter` 键盘连线、`F2` 聚焦对应公开参数 |
| OCR 与翻译 / OCR and translation | 有相应授权时 `Ctrl+4`/`Ctrl+5` 执行，`Alt+4`/`Alt+5` 切换结果 |
| 保存及拖出 / Save and export | `Ctrl+S` 保存；空闲钉图支持 `Shift` 拖出 PNG 副本 |
| 投射 / Projection | 钉图设置选择经授权的设备，确认发送；二维码或链接使用独立的当前 v2 分享流程 |

[中文使用教程](guide/getting-started.zh-CN.md) · [English guide](guide/getting-started.md) ·
[实时截图](features/LIVE_CAPTURE.md) · [投射](features/QR_PROJECTION.md)

## 手测回归 / Manual checks

- 普通截图松手出图；已有钉图、拖动、保存及重开不受影响。
- 分别关闭和打开编辑栏、设置栏、参数行，拖动时各附属控件同步，焦点和编辑草稿不丢失。
- 使用授权的当前 Loom 验证 OCR/翻译和 Art；目录撤销后不可继续提交旧动作。
- 真机验证投射目标发现、策略、接收回执、重开与解除；发送成功不等于接收可见。
- 分别记录普通桌面、混合 DPI/多显示器、输入法、跨机及长稳结果。构建、合成窗口或局域网测试不替代未运行的项目。
