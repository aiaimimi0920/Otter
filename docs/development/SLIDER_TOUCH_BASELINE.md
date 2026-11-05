# Slider 触摸兼容性基线

## v162 活动 pan 生命周期与正式数值事件

新生命周期探针在第 4 次触点更新时要求 pan 已活动、内容确实滚动，随后分别
撤销本地编辑/提交权限和替换场景。继续到第 8 次更新、抬起并泵消息，强制断言
观察到系统 GF_INERTIA PAN 和 GID_END，不以等待 800ms 冒充惯性覆盖。
成功实测两种停止位置均 [0,-32]，后续不再移动、pan 清空、Slider=50，
无数值编辑、祖先 click 或错误；不在整个父窗口销毁后继续注入。

水平探针通过正式 prepare_edit/settle_edits 有界清空队列（最多 32 轮），
验证数值 payload、slider 节点、input continuous 和唯一末尾 change commit。
实测共 2 个事件、最终值 72；此处没有网络请求，不能算服务端业务验收。

最新证据 linshi/otter-native-touch-20261021-111117-0c0d6ffe，4 passed，
10.99 秒，receipt 保存当前测试 EXE hash。保留失败证据
linshi/otter-native-touch-20261021-110914-ad3e28fb：撤销前未观察到活动 pan，
断言拒绝空泛通过。此后仅增加撤销前 Trace 和 pan/origin/foreground 诊断，
未弱化条件或修改生产算法；后续成功不证明该偶发前置失败已根治。
v161 水平父内容误移在本轮三个有界重复中未复现，亦保留待核验。
物理设备、多指/笔、斜向阈值和复杂边界仍不在本次验收范围。

## v161 单指垂直父滚动的系统手势路径

实际系统探针证明：Slider 启用单指垂直 pan、阻止单指水平 pan 后，Windows
把垂直序列识别为 WM_GESTURE，并由默认过程转发给父窗口；GESTUREINFO 的
hwndTarget 仍是子 Slider。水平方向不产生 pan，继续沿已隔离来源的 Slider 输入。
生产不消费部分 WM_POINTER、不同时维护第二份触点表；完整 pointer 序列和
方向识别由系统拥有，原生宿主只接纳已解码、目标核验通过的手势。

Slider 在创建时一次配置垂直 pan 和系统惯性。Surface 父窗口原先只接纳目标
为自身的手势，现在额外接纳当前实际挂载的 Slider；未知子窗口、其他 Surface
及其子窗口、已替换控件或 Host 借用忙碌时均不准入。BEGIN/END 同样先验目标，
避免未经准入的消息取消点击或滚动。Pan 同时绑定 target HWND 与 sequence，
不同目标即使序列号相同也不能移动/结束当前 pan；已接纳目标失效时撤销本序列，
不把暂忙期间遗漏的位移积压到后续消息。

滚动仍由既有 Host.pan_anchor/Host.pan 及 Surface 滚动状态持有；没有新滚动
模型、惯性线程、定时器或输入业务队列。消费的 PAN 句柄由现有 RAII 关闭，
未消费及 BEGIN/END 交默认过程，避免双重关闭。参考
[WM_GESTURE](https://learn.microsoft.com/en-us/windows/win32/wintouch/wm-gesture)
和 [SetGestureConfig](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setgestureconfig)。

新增实际 Surface/编辑挂载探针：垂直滑动使内容 origin 从 [0,0] 到约 [0,-48]，
Slider 保持 50、无数值队列和祖先 click；水平滑动把 Slider 改为 72，父内容
origin 不变、有数值队列且无祖先 click。数值和位移是夹具结果，不是协议常量。
生产配置由 GetGestureConfig 读回验证；常规回归还覆盖外部/未挂载/替换/忙碌
目标拒绝，以及相同序列号不同目标不能接管已有滚动。

本轮显式输入入口共 3 个测试（默认兼容路径、系统轴向转发、实际滚动 Surface），
保持 -RunInteractive 准入。单指基本轴向路径已获得真实 Windows 消息证据；
多指/笔、斜向阈值、复杂嵌套/边界惯性和物理设备仍未完成，不提升整体 touch
capability。v158–v160 中的“垂直父滚动未接入”是历史状态，不再代表本增量。

## v160 真实系统输入接通与兼容鼠标点击修复

v159 的命中失败已通过显式可见启动方式解决：独立 WinForms 小实验在普通终端
启动上下文中也无法命中，但 Start-Process -WindowStyle Normal 后自有 HWND
命中一致、置顶样式正确。该证据定位到交互启动方式差异，不归因于 Slider；
查询到当前 job UI 限制为 0、线程和输入桌面均为 Default，未修改任何系统策略。
单接触注入采用 pointerId=0；保留默认 touchMask=0 即可实际注入成功，无须
额外压力/方向字段。不能从此推导所有系统的 pointerId 取值规则。

可复现入口改为：

```powershell
./scripts/tests/Invoke-NativeTouchProbe.ps1 -RunInteractive
```

脚本从 Cargo 当前构建的 compiler-artifact 定位测试 EXE，以显式可见方式启动；
默认拒绝运行，子进程有 30 秒期限，只清理持有句柄的自启进程。保留真实命中
核验，不移动或关闭其他应用。证据目录在 linshi，保存 stdout/stderr、EXE SHA256
和退出码。该入口确实需要短暂交互窗口，不能替换成普通后台隐藏启动。

首次成功观测发现：tap/horizontal/vertical 均向父窗口发送 CLICK_BEGIN/END，
cancel 发 BEGIN/CANCEL；这证明默认提升为鼠标会错误进入祖先点击候选。
修复前证据：linshi/otter-native-touch-20261021-100909-6229402f/stdout.log。

生产修复仅限已证实的兼容鼠标路径：在按下入口、任何焦点/捕获重入之前，按
GetMessageExtraInfo 的系统标记保存触摸来源；触摸不启动鼠标 click 候选，笔和
普通鼠标保持原行为。拖动 owner 保存来源，另一来源的 move/up/down 不能改变
其值、提前释放或重置基线。取消清理来源，数值输入和完成提交仍沿用既有队列。
这是兼容路径纠错，不是把鼠标提升作为最终触摸方案，也不提升 touch capability。

系统探针四组验证真实 GetPointerInfo 目标、无鼠标 click 候选、水平 input/change、
取消保留 input 但不 change、结束不残留捕获。新增常规测试覆盖所有低 7 位 ID 的
签名区分、笔/鼠标保持，以及两方向混合来源的移动/释放/重复按下隔离。
观察到物理鼠标移动曾混入捕获期间数值更新，来源隔离后再次实际运行通过。

此夹具父窗口不是滚动 Surface：垂直移动只验证没有数值改变/点击，不能证明
父滚动已接入。单接触方向判定、完整 pointer owner、父滚动交接、多指/笔和
物理设备验收仍待完成。签名识别只是事件语义，不是安全授权凭证。
标记规则见 [System Events and Mouse Messages](https://learn.microsoft.com/en-us/windows/win32/tablet/system-events-and-mouse-messages)。

## 证据范围

2026-10-21，v0.2.33.158。通过真实 Chromium 的 CDP 触摸模拟向实际
`input[type=range]` 发送接触、移动、结束及取消；不是通过 JavaScript 手工
触发 input/change。每组导航到新文档，结束后观察 400 ms，包含兼容 click
和滚动惯性的观察窗口。此证据不等于物理 Windows 触摸屏验收。

本地复现脚本：`C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-slider-touch-probe.js`。
原始结构化结果：同目录 `otter-slider-touch-evidence.json`。探测浏览器已关闭。

## 实测合同

初始值 50，范围 0–100，240 CSS px 水平 range 位于可垂直滚动容器内。

| 操作 | Slider 事件顺序 | 数值 / 父滚动 |
| --- | --- | --- |
| 同值轻触 | pointerdown → pointerup | 50，未滚动，无 click |
| 水平移动后结束 | pointerdown → input → pointerup → change | 81，未滚动，无 click |
| 垂直移动后结束 | pointerdown → pointercancel | 50，父容器发生滚动 |
| 水平移动后取消 | pointerdown → input → pointercancel | 81，无 change/click |

同场景的普通 button 轻触产生 button-click，作为兼容 click 观察的阳性对照。
81 和最终滚动 500 是本夹具的几何/惯性结果，不是应硬编码的协议常量。
本轮没有测试多指、笔、所有方向阈值、混合 DPI、设备丢失或真实触摸硬件。

## 原生现状与不采用的捷径

`surface_slider_messages` 目前处理鼠标和键盘，没有显式触摸序列 owner；
`surface_window_gesture` 为父窗口配置系统 pan。不能仅凭父窗口启用手势就
宣称子 Slider 支持触摸，也不能将当前鼠标测试当作触摸证据。

不要直接把 Slider 所有手势禁用并无条件提升成鼠标：上述实测要求垂直接触
交给父滚动，而且触摸结束不等价于现有鼠标 click 冒泡。未经区分的提升可能
误发祖先业务 click，丢失垂直滚动，或与另一触摸处理路径重复提交。

Microsoft 明确说明，不处理 pointer 时，默认过程可能产生手势或鼠标消息；
选择性消费同一 pointer 序列、把其余消息交给默认过程的行为未定义。
接触取消或捕获丢失也不能假定一定收到成对的 UP。
见 [WM_POINTERDOWN 官方文档](https://learn.microsoft.com/en-us/windows/win32/inputmsg/wm-pointerdown)。

窗口手势配置可在创建时设置，且持续窗口整个生命周期；配置与父链的合并
结果可查询。不要在每次输入时无理由改写系统配置。
见 [SetGestureConfig](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setgestureconfig)
和 [GetGestureConfig](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getgestureconfig)。

## 后续实现与退出条件

1. 先确定单接触序列的 owner，校验 pointer 类型、ID、实际目标 HWND、取消和
   捕获状态；额外接触不能覆盖正在处理的接触。不维护无界 pointer 表。
2. 在确认方向前不能抢占父滚动或发出不可撤回的业务 change；水平接触更新
   Slider，垂直接触移交既有父 pan/scroll owner，不复制滚动模型。
3. 结束/取消分别处理：正常结束提交 change；取消保留已接纳 input 值但不
   伪造 change。触摸不得自动使用鼠标 CapturedPointer 的 click 冒泡。
4. 对已接管序列完整处理，明确兼容鼠标消息的去重边界，不能把同一输入
   同时送入两套业务提交路径；未接管序列必须完整遵循系统默认处理约定。
5. 焦点、队列满、编辑权限撤销、场景/合同替换、关闭和迟到 UP 都需回归。
6. 验证真实 WM_POINTER/WM_GESTURE 接线及物理设备行为，再考虑提升 touch
   capability。模型测试、伪造窗口消息和浏览器模拟不能替代这两层证明。

本轮仅建立和验证兼容性基线，没有新增生产触摸处理，也没有改变能力声明。

## v159 Windows 系统输入探针及当前失败证据

新增显式 ignored 测试 `surface_slider_system_touch_probe`，位于
`native/src/desktop/surface_slider_touch_probe_tests.rs`。它使用系统
InitializeTouchInjection/InjectTouchInput，而不是 SendMessage 构造 pointer。
观察子类不消费消息，GetPointerInfo 成功且目标是实际 Slider 才计为有效触摸；
测试不把注入 API 成功、鼠标回退或手工消息视为 pointer 接线证据。

探针运行前必须设置 `OTTER_NATIVE_TOUCH_PROBE=1`，并只选择该测试：

```powershell
$env:CARGO_TARGET_DIR='C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-target'
$env:OTTER_NATIVE_TOUCH_PROBE='1'
rtk proxy cargo test --manifest-path native/Cargo.toml --locked --offline --lib surface_slider_system_touch_probe -- --ignored --nocapture --test-threads=1
```

该测试会短暂创建可见的自有测试窗口并尝试激活/置顶，不能作为后台常规测试。
它使用 PMv2 物理像素坐标；DOWN 前核验实际顶层命中和子 HWND。命中其他应用
时直接失败，禁止绕过检查继续全局注入。日志最多 128 项，消息泵有截止时间；
窗口/子类/触点分别由 owner 清理，未结束的触点在 Drop 发送 CANCELED|UP。
Pointer 绑定只通过 Windows dev-dependency feature 启用，没有更换依赖版本。

本机此次显式运行未通过：自有父窗口报告 visible=true、enabled=true，边界为
(400,300)-(700,550)，但触点 (511,360) 的实际顶层命中属于外部窗口而不是
测试父窗口；请求置顶后读回扩展样式仍为 0x100。探针在第一次 DOWN 前失败，
因此没有向其他应用注入输入，也没有取得四组触摸行为或真实 pointer 到达证据。
前景/置顶请求成功与否不能替代实际命中检查。原因尚未确定，不能归因为 Slider
触摸算法、物理设备或系统权限问题。下一步先解决交互桌面上的自有窗口命中，
再使用该探针验证生产接线；不删除保护断言来制造通过。

系统注入只能证明 Windows 消息路径，不替代物理触摸屏、多指、笔或惯性验收。
API 约束见 [InjectTouchInput](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-injecttouchinput)。
