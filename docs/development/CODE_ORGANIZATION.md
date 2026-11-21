# 代码组织与开发规范

目录和 owner 以[当前架构](../architecture/overview.md)为准；行为与测试要求见[工程规范](ENGINEERING.md)。

| 目录 | 责任 |
| --- | --- |
| `native/src/desktop` | Win32 HWND、消息路由、可见状态和 Application 生命周期；外部操作交所属 worker 或客户端。 |
| `native/src` | 编辑模型、捕获、渲染、资源预算、鉴权、协议、保存及纯逻辑；仅在确实需要时持有系统资源。 |
| `native/assets`、`native/build_support` | 程序图标、原生工具栏 SVG、版本与 Windows 资源。 |
| `native/hover-hook` | 本地 Live 悬浮桥接 DLL，与 EXE 同版本构建、验证及打包。 |
| `native/tests`、模块邻接 `*_tests.rs` | CLI/整进程契约与模块回归；共享线协议夹具放 `native/tests/fixtures` 或 `protocol/fixtures`。 |
| `scripts/native-candidate`、`scripts/tests` | 构建来源、归档、长期运行及联合测试；只把可再生输出放临时目录。 |
| `third_party` | 固定上游、补丁和可验证源码来源；不放 Otter 业务实现。 |

钉图设置的投射目标草稿归 `pin_settings_targets`；接收策略、设备授权、投射 journal 和 checkpoint 分开管理。
移动钉图时，附属编辑、端点和设置窗沿同一窗口组更新位置；几何变化与内容更新分开，避免在鼠标移动时重复构建控件。
NeuroDesign 管控件绘制及布局；Loom 管能力目录、执行和设备授权；Otter 不复制它们的业务逻辑。

新代码按单一职责拆分，优先 100–250 有效行；超过 500 行须按仓库行数门审查，超过 700 行须拆分。
明确测试责任和资源清理边界，不因纯移动或模块拆分改变行为。
