# 参与贡献与翻译

[English](../../CONTRIBUTING.md) · [文档中心](../README.md) · [工程贡献规范](../../CONTRIBUTING.md#development-and-verification)

反馈问题、提议工作流或分享示例前先搜索[已有 Issues](https://github.com/aiaimimi0920/Otter/issues)。
提供包版本或提交、Windows 版本、复现步骤、预期和实际结果；显示布局、缩放、HDR、Loom 版本及授权仅在相关时补充。
移除私人画面、投射邀请和凭据。疑似漏洞使用[私密报告流程](../../SECURITY.md#reporting-a-vulnerability)。

## 文案与翻译

- 写明页面或控件位置、原文、建议译文与理由。错别字只改对应语言；行为变化时同步中英文对应说明。
- 界面文案维护在 `native/assets/locales/<language>/*.ftl`，英文 `en` 为完整基准，
  简体中文 `zh-Hans` 随程序内置。使用稳定语义 ID、Fluent 参数与复数选择，不能用译文判断程序状态。
  增加语言需登记资源与支持列表，并验证消息契约、原生布局及两个设置入口；不通过能力包下载语言。
  详见[显示语言机制](../features/DISPLAY_LANGUAGE.md)。OCR 翻译目标语言与界面语言相互独立。
- 保留文件名、命令选项、标识符、占位符及授权和验证限制；核对本地链接、图片、标题锚点和示例。
  许可证正文与第三方归属声明保持完整。

按改动范围执行验证。文档预览和隔离夹具不能代替普通桌面或双机验收。
