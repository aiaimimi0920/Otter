# 代码组织与开发规范

目录和 owner 以[当前架构](../architecture/overview.md)为准；行为与测试要求见[工程规范](ENGINEERING.md)。

| 目录 | 责任 |
| --- | --- |
| `native/otter-core` | 纯几何、像素、画笔动力学、裁剪与基础线协议值模型；零 Win32 窗口耦合。 |
| `native/otter-media` | 屏幕与 HDR 色彩管线、Surface 排版与绘制、Live 输入/恢复连接基础原语。 |
| `native/otter-runtime` | 矢量标注与历史编辑模型、工作区与 Loom 图参数契约、WGC 窗口目标及受限 HLSL/UIA/滚动子进程运行时。 |
| `native/src/desktop` | Win32 HWND、消息路由、可见状态和 Application 生命周期；外部操作交所属 worker 或客户端。 |
| `native/src` | 会话持久化、Loom 桥接与工作流、设备投射、LiveRelay、Tile 呈现、ANGLE Shader 及宿主组装；根级重导出子库保持模块契约一致，并支持 `scoped-tests` 分域单测裁剪。 |
| `native/assets`、`native/build_support` | 程序图标、原生工具栏 SVG、版本与 Windows 资源。 |
| `native/hover-hook` | 本地 Live 悬浮桥接 DLL，与 EXE 同版本构建、验证及打包。 |
| `native/tests/integration.rs`、`native/tests/suites`、邻接 `*_tests.rs` | 单入口聚合的 CLI/整进程契约与模块回归；共享线协议夹具放 `native/tests/fixtures` 或 `protocol/fixtures`。 |
| `scripts/native-candidate`、`scripts/tests`、`scripts/review-*.mjs` | 构建来源、归档、长期运行、联合测试及审查覆盖统计、热点分包（`review-batches.mjs`）与原子台账写回（`review-ledger-upsert.mjs`）工具；只把可再生输出放临时目录。 |
| `third_party` | 固定上游、补丁和可验证源码来源；不放 Otter 业务实现。 |

钉图设置的投射目标草稿归 `pin_settings_targets`；接收策略、设备授权、投射 journal 和 checkpoint 分开管理。
`projection_checkpoint_session` 负责发送／接收索引的共同原文遍历，各索引仍拥有自己的关联校验。
真实 Loom 联合门共用测试专用的 `projection_joint_input`，从同一文件句柄核验隔离路径及读取预算；发送与协作夹具各自解析其 JSON 契约。
`session_candidate_placements` 装配恢复位置和窗口设置；`session_append` 暂存已验证的贴纸原文供快照比较和追加写出，仍校验最终原文。
冻结实例与保留会话归 `session_frozen_input` 在既有恢复任务一起准备；`application_append` 复核并提交。
移动钉图时，附属编辑、端点和设置窗沿同一窗口组更新位置；几何变化与内容更新分开，避免在鼠标移动时重复构建控件。
长截图录制状态持有浮层布局缓存，浮层模块负责绘制及 GDI 区域交接；尺寸或 DPI 变化才重建箭头布局，退出时清除缓存。
保存与工作流启动探针的独占临时会话及资源目录归 `application_smoke_session_fixture`，不借用用户文件或开发机固定路径。
截图交付由 `open_image` 准备来源与可取消命中区域；`document_window` 约束显示尺寸，视口复用 `pin_window` 的双轴适配，完整图像仍由文档持有。
工作区恢复发布由 `application_workflow_retirement` 取消并接管旧工作流 worker，完成后在应用泵回收；保存及发送凭证仍归各自工作区。
工作区发送前断线联合门由 `application_workflow_real_loom_tests` 验证原生事务，
`scripts/tests/loom_workflow_restart_joint.py` 隔离并控制真实 Loom bridge，独立读回服务端状态。
本机 Loom 连接与主动断开的线程回收归 `loom_connection_task`；连接选择、重试和断开屏障仍归 `loom_local_connection_owner`，HTTP 契约归 `loom_disconnect`。
远端与 Shader 执行共用 `loom_execution_tasks` 的退休和回收调度；Job 的通知后期限逐层接入应用等待，不增加等待句柄。相邻 `loom_execution_reap_fixture` 与 `application_execution_test_pump` 仅供有界生命周期回归。
扩展目录与命令各保留原 Job/Owner；目录撤权与线程回收分离，命令邮箱由局部析构守卫清理。通知后期限接入同一应用等待，回收回归邻接扩展模块。
协作 UI、工具槽碰撞及擦除、翻转、模糊、缩放、栅格化、裁剪 timer 预约的回归各邻接所属模块；各工具保留自身调度契约，缩放两入口共用预约路径，模糊合并请求沿用原 timer。
`application_edit_retirement` 接管已关闭文档的栅格 Job 和解码线程，`application_output_retirement` 负责复制、拖拽准备与已准入导出；都不保留 HWND 或 State，由应用等待器回收。解码交接时也释放替换校验用的文档快照。`edit_worker_budget` 分池约束栅格、输出和解码各自的活动与退休容量。
移动观察夹具独占 subclass 数据和卸载生命周期，不向生产状态添加测试 owner。
Surface 前台复验由 `scripts/tests/native-surface-foreground.py` 按摘要及精确清单启动自有测试进程，
前台许可由用户激活的启动窗授予，测试本身仍检查实际前台；进程树归 `build_job.py` 回收。
Live Source 的媒体连接轮换与资源回收归 `live_relay_source_runtime`，退出优先级归 `live_relay_source_outcome`。
Live 本地 Chromium 的滚轮分流与协议流退休归 `live_input_browser_scroll`；观察和滚动的受限 worker 默认走生产入口，只有实际的 libtest 宿主显式选测试入口。
原生菜单的可见行文字快照及快捷键度量归 `native_menu_panel_text`，由各面板维护，不把 GDI 资源或列表状态跨窗口共享。
冻结图库的索引与视口缩略图分别由 `popup_frozen_index`、`popup_frozen_preview` 准备；`popup_frozen_worker` 为每类任务提供一个应用级活动或退休槽，菜单退出不等待，应用等待器回收已退出线程。
NeuroDesign 管控件绘制及布局；Loom 管能力目录、执行和设备授权；Otter 不复制它们的业务逻辑。

新代码按单一职责拆分，优先 100–250 有效行；超过 500 行须按仓库行数门审查，超过 700 行须拆分。
明确测试责任和资源清理边界，不因纯移动或模块拆分改变行为。
