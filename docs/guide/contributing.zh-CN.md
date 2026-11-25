# 参与贡献与翻译

[English](../../CONTRIBUTING.md) · [文档中心](../README.md) · [工程贡献规范](../../CONTRIBUTING.md#development-and-verification)

反馈问题、提议工作流或分享示例前先搜索[已有 Issues](https://github.com/aiaimimi0920/Otter/issues)。
提供包版本或提交、Windows 版本、复现步骤、预期和实际结果；显示布局、缩放、HDR、Loom 版本及授权仅在相关时补充。
移除私人画面、投射邀请和凭据。疑似漏洞使用[私密报告流程](../../SECURITY.md#reporting-a-vulnerability)。

## 文案与翻译

- 写明页面或控件位置、原文、建议译文与理由。错别字只改对应语言；行为变化时同步中英文对应说明。
- 从[文档中心](../README.md)找源文件。当前界面文字在 Rust 组件内，例如
  [编辑栏](../../native/src/desktop/edit_strip_catalog.rs)与[钉图设置](../../native/src/desktop/pin_settings_fields.rs)。
  改界面文字属于代码修改；文档有译文、OCR 能选择翻译语言，都不等于新增了界面语言。
- 保留文件名、命令选项、标识符、占位符及授权和验证限制；核对本地链接、图片、标题锚点和示例。
  许可证正文与第三方归属声明保持完整。

按改动范围执行验证。文档预览和隔离夹具不能代替普通桌面或双机验收。
