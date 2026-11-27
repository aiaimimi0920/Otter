<p align="center"><img src="./native/assets/branding/otter-icon.svg" width="80" height="80" alt="Otter 截图图标"></p>
<h1 align="center">Otter</h1>
<p align="center"><strong>截下画面，留在桌面，改成你想要的样子。</strong><br>连接节点，生成效果。让桌面成为你的创作工作台。</p>
<p align="center">
  <a href="https://github.com/aiaimimi0920/Otter/releases">下载</a> ·
  <a href="./docs/README.md">文档中心</a> ·
  <a href="https://github.com/aiaimimi0920/Otter/issues">反馈</a>
</p>
<p align="center">
  <a href="./LICENSE"><img src="https://img.shields.io/badge/license-MIT-36564f?style=flat-square" alt="MIT 许可证"></a>
  <img src="https://img.shields.io/badge/platform-Windows_10_%2F_11-36564f?style=flat-square" alt="Windows 10 / 11">
  <a href="https://github.com/aiaimimi0920/Otter/actions/workflows/build-otter-exe.yml"><img src="https://github.com/aiaimimi0920/Otter/actions/workflows/build-otter-exe.yml/badge.svg" alt="Build Otter EXE"></a>
</p>
<p align="center"><a href="./README.md">English</a> · 简体中文</p>

Otter 是一款免费开源的截图工具，让截图、标注、整理与分享融入日常工作。
将桌面化作工作台，连接不同的处理节点，组合功能、制作效果，让截下的画面拥有更多可能。

[功能](#功能) · [开始使用](#开始使用) · [参与贡献](#参与贡献)

## 功能

- **截下你想要的画面。** 支持区域、窗口和长截图，也能随手拾取屏幕颜色。
- **把图片钉在桌面。** 让参考图随时可见，通过分组、图库和历史记录整理素材。
- **随手标注，继续编辑。** 裁剪、擦除、旋转，添加文字、箭头与图形，也能用模糊和马赛克遮挡信息。
- **让钉图实时更新。** 把窗口或局部画面变成实时钉图，持续查看内容，并与支持的源窗口交互。
- **连起节点，组合效果。** 接入 Loom 的 OCR、翻译和 Art 能力，通过连线组装自己的处理流程。
- **把画面投向其他设备。** 将图片或实时画面投射到已连接、配对的设备，方便跨屏查看与协作。

[**功能与快捷键 →**](./docs/FEATURES.md)

[内部开发版：截图、脱敏标注、鼠标穿透和图像导出](./docs/features/DESKTOP_CAPTURE_EDIT_EXPORT.md)

## 开始使用

| 从这里开始 | 需要做什么 |
| --- | --- |
| [公开下载](https://github.com/aiaimimi0920/Otter/releases) | 下载 Windows x64 便携 ZIP 和配套 SHA-256 校验文件。完整解压，保留同目录的 `otter_live_hover_hook.dll`，然后运行 `otter-native.exe`。便携版未做 Windows 代码签名，不是安装包。 |
| 配合 Loom 使用 | 启动 Loom，安装所需的 OCR、翻译或 Art 能力，再[连接并授权 Otter](docs/features/LOOM_CONNECTION.md)。 |

1. 按 **`Ctrl+1`** 区域截图、**`Ctrl+2`** 实时截图，或 **`Ctrl+3`** 长截图。
2. 选中钉图，按 **`Ctrl+E`** 进行编辑，或按 **`Tab`** 展开参数面板。

平时双击 `otter-native.exe`。需要截取管理员窗口时，先在托盘退出 Otter，再右键同一个 EXE，选**以管理员身份运行**。只影响这一次，不改变开机自启；用完退出，再双击即可恢复普通模式。Loom 和设备投射请在普通模式使用。

[**完整使用教程 →**](./docs/guide/getting-started.zh-CN.md)

## 开发与历史版本

主干是 Otter 的当前版本。开发、构建和内部打包见[开发指南](docs/development/WINDOWS_NATIVE_BUILD.md)。最后一个 WebView 主干保存在 Git 标签 `archive/webview-main`；已有公开下载不会随主干更改。

## 参与贡献

欢迎反馈问题、参与翻译、改进文档或贡献代码。
从[参与贡献与翻译](./docs/guide/contributing.zh-CN.md)开始，
也可以从[项目目录说明](./docs/architecture/overview.md#2-repository-layout)了解代码结构。
新增或移动文件前，请阅读[代码组织与开发规范](./docs/development/CODE_ORGANIZATION.md)。
代码修改请遵循[工程贡献规范](./CONTRIBUTING.md)，反馈前先搜索[已有 Issues](https://github.com/aiaimimi0920/Otter/issues)。

## 许可证

Otter 自有代码采用 [MIT 许可](./LICENSE)，依赖保留各自的[许可与声明](./THIRD_PARTY_NOTICES.md)。
