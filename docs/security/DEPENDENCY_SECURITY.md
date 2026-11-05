# Otter Dependency Security

## 原版紧急退出双通道（v524，隔离定向通过）

补齐原版已实现的两条紧急退出：完整 Esc+Delete 三次组合（相邻严格小于400ms，双键重叠且完整释放），以及独立 watchdog 的 Ctrl+Alt+Shift+F12。主消息循环与 child 共用纯状态机；child 在配置/UI 前分流，先见松开才 arm，以原版8ms周期只读键位，不注入输入、不记录文本。
复用原同EXE受限白名单 launcher 和 Hello/Ready；直接父PID、创建时间、同EXE路径及pipe创建者共同认证。普通恢复权限仍只有查询/等待，只有紧急认证额外取得 PROCESS_TERMINATE，并只使用保活原父句柄。正常Drop和握手失败只回收自己新建watchdog，不终止Job或承担Live恢复责任的child。紧急退出绕过保存/析构，不能保证未保存工作保留；原Live父死恢复机制继续负责已登记源。原生未修改全局光标约束，故不无条件重置其他程序的 ClipCursor/ShowCursor/SetSystemCursor。
当前选定Rust按crate/完整名去重 106 passed、6 默认ignored，非整库；11项新增回归覆盖严格400ms边界、完整释放、启动已按住、缺失keyup/迟到消息、参数、认证失败不arm、握手失败、正常Drop及阻塞牺牲父进程的三连/备用终止和父死child退出。另复测原恢复认证/握手/管道/launcher、三进程父死恢复、系统会话消息/Checkpoint和普通固定SHA EXE工作区。全部在未切换的独立desktop/自有Job执行，不申请SWITCHDESKTOP，不向用户应用发送输入。
strict all-targets Clippy、lib/普通product/workspace/恢复认证及session目标构建、官方fmt/版本/全仓strict行数/diff通过。input1因Application达到503有效行而在构建前失败，原回执保留；input2按职责提取application_loop，保持原消息泵/显示恢复/保存/退出顺序，不增行数例外。input2 Clippy通过后，因只读审查发现局部消息可漏收keyup，停止本轮自建在途构建树并保留中止记录；input3主通道按当前键位快照保守累计，不凭历史消息制造重叠，新增负向回归。源码只分配一次v0.2.33.524，Loom未改。
新增常驻进程单独实测180秒（97点）：产品及watchdog全程存活，含idle60秒和三轮hidden20秒/resumed20秒；正常隔离QUERY/END后整棵自有树归零、desktop关闭、工作区保留。该4进程树（产品、watchdog及两个conhost）Private commit为8.691–8.957MiB，Working set合计61.430–61.578MiB（共享页可能重复计），模式内单核CPU约0.156–0.260%，Handles658/GDI33/USER16稳定。Ctrl+1仍记录已有热键占用0x80070581，不冒充快捷键验收。资源明细见main-emergency-exit-1/runtime-audit-3.json；不是Otter+Loom总资源、GPU、物理输入或长稳验收。
本批仅关闭源码与限定隔离门，不恢复前台自动化，不证明物理三连/F12或普通可见混合链通过。v518前台身份变化来源及v517普通可见混合门未关闭，不拿当前隔离通过覆盖历史失败。人工IME/旋转EDIT、物理多屏/跨机/DPI热插拔、资源长稳及最终隔离整包/用户全面手测仍开放。Talk外部会话控制按用户决定排除本轮，旧Web/Tauri混用不重列必做；三冻结副线、v464固定包及真实profile/设备不动，总迁移active，不是正式发布。

## 系统结束会话持久化准入（v523，隔离消息链验证）

只消费已落盘且来源/精确projection scope/当前stamp均匹配的原凭证，不新增保存路径、丢弃批准、外部输入或系统退出权限。MAIN独占session_end及application_session_end、Application/文档/Live/Relay/Surface接缝与专属测试。查询回调仅在只读wait栈词法作用域安装并RAII撤回，按Control身份隔离且禁止重入复用；主循环/shutdown的owner只持Weak<Control>，不保存Application裸指针。surface_edit_pending只提取原pending观察并新增空闲观察，surface_window_session_end独占Surface准入接缝；不复制保存、输入或排空框架。
冻结不拦原销毁/取消/读取清理；WM_GESTURE/WM_TOUCH沿默认过程关闭句柄，WM_DROPFILES显式DragFinish。只向自启PID的精确HWND发送系统消息，真实注销/关机、强制终止和最终包未验。

## 2026-10-28 迁移目标边界

2026-10-29 ANGLE生产入口退役批次：默认HLSL产品路径验收后，旧候选一律拒绝且不打开目录，
旧worker参数在私有派发前拒绝；旧child通道及实际DLL加载仅编译到测试目标，生产旧render返回Unsupported。
原生候选stage原本仅复制EXE，没有ANGLE DLL复制步骤；Archive核验新增双DLL禁入，大小写/嵌套路径均检查。
历史ANGLE工具、许可证和材料SBOM保留为历史证据，不作为新native产品运行依赖；不改写历史通过回执。
当前退役集中验证位于main-angle-retirement-1；不等于正式包、供应链全门或整项迁移验收。

v440 普通窗口兼容呈现也拥有 D3D11/DComp 图，创建失败和运行失败均消费原有关联/退休准入；
不再凭普通窗口类型或 HRESULT 签发立即完成。复用有界退休 worker、设备弱缓存和完整 owner 保留，
没有新依赖、权限、网络、持久化设置或常驻线程。软件候选仍只在明确未关联失败后惰性创建。

v439 GPU 重选复用同一创建/退休准入；无关联失败凭证只来源于类型化 Failure，不能凭 HRESULT 或缺 owner 推断。
GPU 回升可见性重建仅对本次移除 layered 且原本可见的自有窗口执行；使用 NOACTIVATE/NOZORDER，不请求全局前台，
不主动恢复键盘焦点。所有异常路径尝试一次恢复，宿主撤销/托盘或分组隐藏意图禁止恢复，不写已复用 HWND。
样式操作只改自有宿主的 LAYERED 位，不持 Canvas 借用、不跨新候选关联保留 rollback 事务；宿主撤销即停止写入。
没有新增权限、依赖、后台线程或全局偏好。实验只显示自有色块并采样其中心，不发送用户输入或操作其他窗口。

v438 手动渲染命令重新检查稳定宿主、paint 重入和关联准入；CPU 选择不清除未知 GPU 退休终态。
CPU 创建前失败允许显式重试，但仅在已明确选择软件且创建期准入未拒绝时，不伪造 GPU Complete。
命令不跨 Win32 调度持有 Canvas 借用，不新增依赖、线程、磁盘配置或权限；保存测试只用 linshi 独占 GUID 文件。

v436 CPU 钉图不绕过关联准入：初始 fallback 消费原 Failure 凭证，运行中 D3D11 仍需 DComp Complete。
窗口软件样式事务在每次可能重入的写操作后复核稳定宿主身份，撤销后不写入或回滚可能复用的 HWND。
提交像素仍由不可变 Rc DIB 租约拥有，模型借用外提交；不新增线程、依赖、GPU 配额或系统设置。

v435 窗口恢复将旧 Target 从活动槽移入单独 owner，最小化、隐藏或换源不能解除退休门禁。
已有 DComp worker 的配额和两秒终态不变；窗口只用一个私有 timer 轮询，不新增线程/结果队列。
失败/超时及调度失败不放行兼容候选，晚到成功不解锁；WM_DESTROY/WM_NCDESTROY 停 timer，
关闭不 join worker，未确认 GPU 图继续遵守后端保留预算。确认后 GPU drain 仍可能同步等待最多 5 秒，
不能声称整个恢复无 UI 停顿。无 DComp 的既有一次可恢复重建保留，其他终态保文档而不自动关窗。
本批不新增系统权限、依赖或外部输入，失败/超时测试使用合成状态，不冒充真实驱动卡死验证。

v432 Canvas 的创建准入只缓存一个原始错误，不增加资源队列、后台线程或全局 HWND 表。
阻止状态不随 discard_device 复位，避免从后续 WM_PAINT 绕过关联保护；此状态的消息入口不发 WM_CLOSE。
这是保文档的受阻状态而非安全重试授权；手动恢复必须另有确认完成/安全宿主策略，不得直接清空布尔值。

v431 创建选择器消费类型化关联准入凭证；关联失败不能降格为普通可回退错误后继续建立另一合成链。
原 HRESULT/消息保持，准入不依赖易混淆的错误文本。该保护不新增等待、线程、缓存或系统配置，
也不承诺运行中自动恢复已经完成；现有窗口最终错误处理仍需改为保文档终态。

v430 D3D12 创建期关联操作在完整 ManuallyDrop owner 内执行；SetContent/SetRoot/Commit 任一失败
均发起已有退休流程并保留原错误，未确认时保住全部 GPU 图与预算，不增加 worker 或设备配额。
新增测试仅在独立进程使用真实窗口/设备及合成失败凭证，故意保留至进程退出；不修改真实 profile，
不注入系统驱动错误。未确认关联后的下一候选准入仍未完成，不能据此声称自动恢复安全性已整体通过。

v428 两后端共享最多两个 DComp worker 和 MAX_UNITS 个窗口图预算，不能各自扩展一份等待配额。
D3D11 ManuallyDrop<State> 覆盖 context/backing/chain/effect/visual/target/device/shared Rc/配额，
未确认时不能只在普通 Drop 提前 return（普通字段仍会自动析构）。新增保留测试显式隔离进程，
使用真实 GPU owner 加合成失败凭证，不谎称驱动故障。正常设备最后租约释放与 Live/relay 由原回归保护。

v426 的 unsafe Send 仅限官方明确线程安全的 DComp device/visual/target 私有封装，禁止扩展至
单线程 D2D factory、GPU owner、Rc 或 HWND 操作。退休前封住此窗口后续绘制与合成属性提交，
同一设备不并发发起另一批 Commit。工作线程初始化 MTA，释放 DComp 引用后解除初始化。
最多两个在途 worker、每个一个结果；两秒 UI 终态不等于终止无 timeout 的系统等待，
超时线程继续占名额直到真实退出。未确认时保留全部 GPU State 与图租约，不以超时为释放许可。
没有新增依赖/系统配置/网络权限；真实驱动卡死及未确认图长期回收仍需后续验证。

v425 软件帧只在 Canvas/resolved_display 借用结束后由窗口 owner 提交；Target 使用独立 Rc 身份，
回调后复核宿主归属、Target 与 DocumentStamp，不确认新文档的旧帧。DocumentStamp 仅覆盖文档身份/
已提交 revision，不代表所有 viewport/手势瞬态。嵌套 paint 只记 deferred，外层退出最多补一次失效。
软件资源接口限 desktop 内部同步适配；禁止保存裸 COM target 或绕过帧 owner。无新依赖、后台队列或权限。
创建期 CPU 兜底仅普通工作区，钉图分层事务尚未接入；历史 WM_CLOSE 最终错误策略仍待替换，不宣称数据恢复完成。

v424 软件提交凭证只持有 Rc DIB/DC/预算租约，无像素复制、后台线程或未计费缓存；凭证不可 Clone，
提交成功或失败均消费，丢弃即取消。同一 DIB 有在途凭证时拒绝再次绘制，resize 保留旧 DIB 至最后租约结束。
提交限制为同 UI 线程的有效 HWND，但线程检查不是授权；目标窗口归属和无模型借用仍由内部窗口 owner 保证。
真实样式重入测试将原渲染器在 WM_STYLECHANGING 中释放，核验提交凭证独立生命期；没有修改真实 profile 或外部窗口。
原 WM_SIZE 测试实测没有触发预期通知而失败，不能据此声称 ULW 必然发送 WM_SIZE；原证据保留。
裸 COM target 资源出口收窄为内部测试，绘制闭包仍是可信适配边界，禁止保存 target 或自行启停帧；
租约约束不对绕过接口的任意 unsafe COM 操作提供安全隔离。

[统一修订计划](../development/WINDOWS_NATIVE_REVISED_PLAN.md) 要求替代验证后退役 ANGLE，
届时同步运行包、loader、测试、SBOM/许可证与供应链材料；本轮未删除依赖或扩大 SDK 使用授权。
Loom 管理源码包不等于任意本机执行授权；包版本/digest、权限、离线有效期与实例生命周期必须版本化。
允许 D3D11On12 不等于允许任意 DLL 加载；CPU/恢复路径仍须有界并保留文档真值。

v416 仅补候选透明度/同设备资源入口，无新依赖、线程或像素副本。透明度须为有限的 [0,1] 数值；
DComp 提交链错误保留旧已提交值但将候选标为失效，不伪造回滚成功。没有新增系统配置或 SDK 操作。

v417 共享 owner 不跨线程、不增加后台队列或 DLL。窗口图进程预算复用 MAX_UNITS=128，
设备代际最多 2；两者均包含健康在途和未确认关闭后永久保留者。单窗口仍限 16M 像素/固定双缓冲，
该数量上限不等于已验证的总显存预算或性能承诺，默认接线前仍需资源验证。
任意 drain 错误毒化共享设备，后续不重复 5 秒等待；嵌套帧/帧内 resize 拒绝不单独毒化设备，
但帧内关闭若无法安全排空则保留窗口图并毒化整代。D2D/11On12 包装资源释放完成后才释放帧准入。

v418 将已有 D3D12 后端接入 Canvas 默认创建事务；不增加依赖、系统安装、配置写入或权限。
主链失败才创建原兼容链，各链有界尝试，错误保留两次原因及最终 HRESULT；不通过环境变量注入假成功。
设备代际/窗口图租约、像素限制、帧异常清理沿用原 owner。当前未新增运行中自动恢复保证，
现有 WM_PAINT 失败处理仍须纳入后续保文档恢复改造，不能以创建期回退冒充运行中降级。

v419 降级只释放 GPU 缓存并改变窗口内运行期选择，不写用户配置、不改文档真值或扩大权限。
后端失效由 failed/buffer 完整性/设备状态判断，不按任意业务错误的消息字符串或 E_FAIL 猜测。
兼容选择在 GPU 析构前锁定，正常重绘不自动回跳；单向降级一次，旧可恢复错误重建一次，不产生后台重试。
实际 GPU hang/移除尚未注入，未确认排空时仍保留原 GPU 图及配额；这不证明驱动调用延迟有硬上限。

v421 样式事务无新依赖/后台线程，不向脚本暴露 HWND。入口检查当前 UI 线程，unsafe 合同要求调用者
保证 HWND 生命期、旧合成树解绑与无跨重入借用；不能把线程检查当作任意 HWND 的授权判断。
写前拒绝 NOREDIRECTIONBITMAP 窗口，避免其原地切换失败的部分修改；替代 HWND 仍待窗口 owner 接线。
只拥有分层位，回滚不覆盖其他位、不改位置尺寸或前台焦点；回滚失败明确记录，不无限重试。

v422 在 WM_NCCREATE 前置宿主身份绑定，候选拒绝时不安装共享 State 指针；路由和面板刷新只接受
匹配的 HWND。原始 State 指针仍由内部 DocumentWindow owner 保证生命期，不向外部或脚本开放。
无新依赖/线程/配置；无 State 候选不影响宿主计数，不执行原文档清理；尚未提供宿主转移接口。
额外同 HWND 分层探针只操作自建隐藏窗口/DC/DIB，结束时释放；未改产品的 NOREDIRECTIONBITMAP 拒绝规则。
探针成功只说明局部 API 组合可行，不能跳过 DComp 解绑/资源保留/实际显示验证或冒充故障恢复。

v423 的可见门必须显式运行，窗口不激活、不发全局输入；只读取自有测试色块中心像素，不保存桌面截图。
GPU 图保留至软件提交之后，测试显式确认 DComp 解绑，不把释放 HWND 当作驱动排空。
分层软件候选允许保留 NOREDIRECTIONBITMAP，普通 BitBlt 仍拒绝；不移除该标记、不增加权限或 DLL。
测试使用 WaitForCommitCompletion 不代表生产等待有硬上限；驱动故障与自动降级仍待实现验证。

v420 软件候选无新依赖/线程/系统配置，所有帧限制 8192 边长/16M 像素；DIB 与两份内部位图高水位
共同使用原 512 MiB 共享像素预算。缩小窗口不假定 D2D 已释放内部缓存；COM/GDI 对象等非像素开销仍待实测。
新 DIB 初始化为零，创建/预算失败不暴露未初始化像素。BindDC 失败不猜绑定回滚，保留两个 DC 至 target 析构；
所有 GDI 批次在 DIB/DC 释放前排空，先恢复旧选入对象再 DeleteObject/DeleteDC。
调用方负责只传入自有 HWND，接口检查句柄有效性、所需样式和有限 [0,1] 透明度；不向 Art 暴露原始句柄。
候选不修改样式、不注册输入或读取桌面/真实 profile。
该实现仍未接入产品 CPU 降级链，不能用软件 target 创建成功代替最终低资源或全功能验收。

v415 仅迁移默认 Canvas 的帧 owner：无新依赖、线程、队列、权限或像素副本。
错误/panic 由栈上 guard 配对结束 D2D 绘制并恢复 clip/transform；失败不执行钉图显式 Present，
但旧 HWND target 的 EndDraw 内部呈现不提供像素回滚。Target/文档/缓存及设备丢失后的整体丢弃责任不变。

v414 仅为已锁定 windows 0.60.0 启用 Direct3D12/Direct3D11on12 API feature；不升级 crate、
不下载或分发新 DLL/SDK，不修改系统配置。候选使用系统 API，同线程持有 COM，不暴露 GPU 指针给 Art。
窗口尺寸继续受 8192 边长/16M 像素限制，固定双缓冲；失败绘制不能 present，包装资源由 RAII 释放。
GPU 完成无法确认时，最多两个候选的完整资源及 event 保留至进程退出，不无限创建替代品，
不以 5 秒 fence 等待期限承诺整个驱动析构也有界；这是未默认接入候选的保守保护，不是最终恢复验收。

v412 仅增加固定大小的几何快照和原 Document/Space 围栏，UI 不解析会话或新增像素复制。
后台转换失败不回退到旧尺寸掩盖错误；原图像/PNG 预算、Lease、授权及唯一 Save/Checkpoint 保持不变。
专门回归与相关 lib 门通过，实际边界见 [v412 构建记录](../development/WINDOWS_NATIVE_BUILD.md#v412无-raster-目标实时外框)。

## v411：稳定发送关联显式停止

无新依赖、授权来源或网络 owner。显式停止保完整 selection/envelope，先在原有界 Store 持久减权；
canonical pending 真实提交后才使用原 Job/Lease 的当前同 origin/device 授权执行 unlink。
成功事实保留在原日志直到精确 canonical ack，不将任意缺记录、404、手动保存或 candidate 当成功。
终态回执不能重新 activate；同目标重发须等 ack，旧 intent/proof 不能删除新 intent。日志仍受 64 项/48 MiB、完整基线及恢复副本围栏约束。
新增 stop 字段默认缺失保持旧记录语义，但旧程序 deny_unknown_fields 会拒绝新记录，不能声称双向降级兼容。
HTTP 成功而本地回执未提交时仍只能保 pending；不降低失败关闭规则。隐藏合成测试不代替真实双端/长稳或 release 依赖审计。
同源定向 lib 门通过，startup 正式集成仍有一项原 15 秒超时；Ack 恢复副本分支只有静态复核，没有完整文件故障注入成绩。
证据见 [v411 构建记录](../development/WINDOWS_NATIVE_BUILD.md#v411稳定发送关联显式停止)。无真实 profile、系统安装、注册表/证书或默认 DLL 变更。

## v410：投送目标编辑与外观归属

无新依赖、权限、协议或持久化 owner。目标纯外观只在原发送 worker 中从 canonical 原文读取，
Unit 数沿既有会话上限，图片/大历史借用跳过，纯外观值转换前限长；重复目标、坏状态、超限或无目标失败关闭。
解析不授网络许可；原 Binding admission、当前授权/Lease、持久意图、PNG source/inner/outer 与共享预算保持不变。
resolved 像素与目标 editor 独立，不借上游标注或旧 canonical crop 覆盖目标当前编辑；历史操作不盲重放。
隐藏合成 profile/loopback TLS 不是完整依赖审计、真实双端或资源长稳验收；无系统安装、注册表/证书、默认 DLL 或发布。
结果见 [v410 构建记录](../development/WINDOWS_NATIVE_BUILD.md#v410投送目标编辑与外观归属)。下方 v409 为历史。

## v409：本地保留原图生产捕获

无新依赖、权限、协议或持久化 owner。retained 原图只有在当前文档、底图 owner/size 与 logical frame 均匹配时才准入；
同文档的内容擦除后不借旧原图或滞后宿主像素覆盖新底图。所选像素 Weak 身份与原显示/文档围栏共同过滤迟到结果，
不将像素内容相等当 allocation 相同，也不让 Scope 的凭证额外保留解码图像预算。
UI 只共享像素及已提交 editor，不增加大 JSON/解码/编码阻塞；原后台 source/inner/outer、4 MiB、共享预算及取消围栏不变。
resolved 图源不借目标 retained，完整 metadata/历史编辑对齐仍未验证；隐藏合成 profile/loopback TLS 不代替真实双端或资源长稳。
结果见 [v409 构建记录](../development/WINDOWS_NATIVE_BUILD.md#v409本地保留原图生产捕获)。无系统安装、注册表/证书变更、默认 DLL 或发布。下方 v408 为历史。

## v408：有界源分辨率编码出口

无新依赖、权限或协议变更。投送编码在合成前分别核 source/inner/beautify outer 的 8192/16M 限额，
在 WIC 编码前预占保守 HGlobal 上界，实际压缩 PNG 大于 4 MiB 拒绝，不自动缩图。
美化 WIC 副本与返回 PNG 继续共享预算；取消前后检查不等 WIC 中途可强制打断。
消费式位图出口在 PNG 拷贝前释放合成/美化 WIC 副本的预算，发送 HGlobal 租约仍覆盖拷贝阶段；
普通 workflow 不因输出 lease 延寿而收紧既有预算准入，独立预算回归核释放顺序与发送租约存活。
当前已提交 base 优先于旧宿主 image；本轮不启用未经活动文档身份校验的 retained source。
独立预算和真实像素/大 PNG 回归不替代完整 release 依赖审计、真实双端或资源长稳验收。
无系统安装/注册表/证书改动、真实 profile 或默认 DLL 启用；完整 PNG metadata 接线仍开放。
Live 保存只排除已离图的同 Workspace 退休成员，不以 is_alive 跳过当前异常关闭 Unit；
保持 fail-closed 和原后台退出/reap 责任，不在 UI 同步等待 worker，也不放宽保存基线校验。
结果见 [v408 构建记录](../development/WINDOWS_NATIVE_BUILD.md#v408有界源分辨率编码出口)。下方 v407 为历史。

## v407：单目标发送生产接线

无新增依赖或协议扩权；只准入当前 Directory 中的 SharedLoom 目标、当前授权、已证明的 canonical scope 和原设备身份。
Preparation 在原 Job/Lease 内完成，发送前后台校验会话角色/容量，持久登记 Request 后才允许 read/Create。
已登记原意图在 preparation error/panic、晚撤权及 closed/discarded 过滤后仍交回 owner；失败不自动重新签名或重新 Create。
完整 outbox 基线只在后台执行，UI 领取 metadata 不授新的网络许可；后续 Ready 验证仍检查基线。
worker 结束后的外部磁盘变化不在 UI 重新读盘检测；保存 Binding metadata 不授权新请求，后续后台验证仍须拒绝变化。
同 Store 的真实退休发布事实只用于结算减权责任，外部删文件不能冒充确认 unlink。
手动 Save As/未知 index 不能冒充 canonical 成功；失败清理保记录并有界退避，当前设备授权不符时不借权解绑。
PNG 边长/像素/传输上限及编码前共享预算限制仍生效；原始分辨率/完整 beautify parity 尚未完成。

仅隔离合成 profile、隐藏 HWND 与 loopback TLS；无真实凭据/profile、系统安装/注册表/证书变更、默认 DLL 启用或发布。
这不是新的联网依赖审计或完整 release 门成绩；结果见 [v407 构建记录](../development/WINDOWS_NATIVE_BUILD.md#v407单目标发送生产接线)。下方 v406 为历史。

## v406：发送绑定候选与精确检查点

新增 sender::binding，将已校验 SenderReady 后台转换为兼容旧格式的 sticker.projectionSenders 候选，
不复用 receiver qrProjection，不增加 documentRevision，不写磁盘，也不退休原 sender outbox。
linked 按 receiver 是否存在取值，awaiting 合法为 false；原生只接受 SharedLoom，offlineTransport/peerId 不隐式降级。
完整 envelope、unit、target 和 durable scope 必须一致；每项先限 16 KiB、每源最多 8 项，旧 primary source 同计容量。
重复身份/目标、同 ID 不同 envelope、版本回退或同版本不同 digest 均拒绝；stopPending/stopped 阻止更新。
合法 JSON 转义与未知扩展保留，大型未知图片字段借用 RawValue，整输出沿原 session_write 共享预算。

projection_sender_binding/inventory/patch 分别拥有模型校验、兼容容量规则与后台原文 patch；
projection_sender_checkpoint 拥有精确 Source allocation 与完整 Binding 索引；原 Save/Checkpoint 仍是唯一提交 owner。
worker 对最终 candidate.bytes 捕获索引，只有 canonical disk.commit 成功后才提升 Owner.senders；
默认恢复只领取 canonical、非 recovery/stopped 且对应原文的索引。普通文件保存保留坏/未知字段，但不授原生发送资格。
Application::projection_sender_inclusion 只证明当前 Workspace/Record/scope/canonical Source 精确包含绑定，
不是当前授权、未停止、网络或接收端显示证明；候选、暂存、手动另存、旧 Source 或同 UnitId 新工作区均不能借用。
Ready::prepare_binding 前后核原 Lease/Grant/Active；完整 outbox 基线含磁盘 I/O，只能后台调用，不能在消息线程执行。

同源选定 20 组 lib 门全部通过，过滤组不累计独立覆盖：sender 46/0/0、隐藏 sender checkpoint 3/0/0、
scope 模型 4/0/0、隐藏 scope 8/0/0、保存快照 23/0/0、session candidate 55/0/0、Live checkpoint 19/0/0、
接收 checkpoint 9/0/0、binding 6/0/0、runtime 9/0/0；其余原文/保存/档案/崩溃/几何逐项回执封存。
真实 loopback TLS Ready→candidate 与隐藏 Application 保存验证为分段验证；后者通过 Record::edit_original 注入候选，
不是生产 sender result consumer 或 UI 端到端投送证明。四项正式集成门 build_resources 2/0/0、exe_resources 1/0/0、
startup 7/0/0、session_startup 9/0/0；child 使用编译时 target 产品，前后核与封存 EXE 同字节。
隐藏离线 --workspace-check 通过，新增 sender bindings durable 计数；显式隔离合成 profile，不是真实 profile 或最终整包。
首次产品封存早于集成构建结束，cargo test --no-run 重建 target 产品后触发 Product drift，集成测试未启动。
旧封存 EXE/PDB 保持原字节，失败观察留存；最后一次构建完成后以同一 input-3 重新封存 product-final-2，
重新运行隐藏启动及四项集成门，不放宽哈希围栏，不把前次产物结果冒充最终产物。

中间 Clippy 重复 getters/借用及 type_complexity 失败保留；移除重复定义、修正借用并提取 Replacement，未加 allow。
只读审计发现恢复入口缺 primary 容量围栏及旧字符串转义问题，input-3 修正生产规则并补测试；不是仅改夹具。
正式输入重建并跑完全部选定门，无断言/期限放宽或新增 ignore。严格全目标 Clippy -D warnings、fmt、版本/组织合同通过，
3894 文件 ratchet 零违规/警告/新例外；相对 v405 native 20 变化/6 新增，最大 356 有效行。
源码 SHA-256：`068bfabc1a63a0727f1b382f3d1ca5590503732a98fc95788e93fd8af9c273bb`；产品 EXE 44691456 字节，SHA-256：`40843d29e769e1d8e059db1ffbc024568303d204374a4d3cb3254ad61df08d38`。

当前显示 PNG 捕获、实际发送菜单、生产 Application sender result consumer 与源失效/撤权/切工作区/退出后台 CancelSend 交接仍未接通。
持续发送、真实 Loom 双端/离线互信、双向编辑/LiveRelay/Tile/资源长稳、完整 N01–N12 与最终用户手测仍开放。
只有确认解绑才退休 sender outbox；保存绑定不退休。未重跑完整 34 门，v398 间歇呈现失败与历史严格前台/启动根因仍开放。
目标 active、objectiveComplete=false、publishable=false；无新依赖、真实 profile、前台/全局输入、系统安装、默认 DLL 或 commit/push/publish。
下方 v405 为历史。

## v405：持久工作区发送 scope

新增会话根字段 `otterNativeProjectionScope`：32 字节系统随机数、64 个小写十六进制字符，
不从默认 workspace.json 路径、可复用 UnitId 或运行期 Arc 身份推断。scope 不是授权或发送完成证据。
缺字段的可恢复旧会话由当前 Record 在首次快照时仅分配一次；原文暂未包含字段时，
原 Data 每次携带同一 scope，由原后台 writer 安装并沿共享预算编码，不新增工作区/持久化 owner。
已存在的坏值、null、重复键、零值或不同 scope 失败关闭；原始字段先限 386 字节再解码转义。

session_projection_scope 拥有值与后台字节 Proof；原 session_document/candidate 负责加载，
Record 拥有待保存元数据，Receipt 同时核 scope，原 Save/Checkpoint 仍是唯一提交 owner。
application_workspace_scope 只读 canonical 已提交 Proof、当前 Workspace 身份、原 Source 与 scope；
生成/编码/暂存、普通导入、手动另存、旧 Source、新 Workspace、只读恢复副本或 stopped/exit 均不能借旧证明。
缺 scope 的默认原生检查点即使没有用户编辑也执行一次迁移，随后不反复写入。
未改 Record.original/disk_original 来分配 scope；导入手动文件保持完整磁盘基线，Ctrl+S 仍按原比较替换。
带 scope 的重新打开和另存视为同一逻辑工作区、保留身份；不是自动分叉副本，未来分叉需另定生命周期。
无 scope 的两代导入即便复用相同路径/UnitId 也不同，且新代必须等自己的 canonical 提交。

同源选定 18 组门全部通过（过滤组不累计覆盖）：scope 模型 4/0/0、隐藏 Application scope 8/0/0、
保存快照 23/0/0、session candidate 55/0/0、Live checkpoint 19/0/0、接收 checkpoint 9/0/0、sender 39/0/0；
原文/保存/档案/崩溃/几何等其余门的逐项回执已封存。正式集成门 build_resources 2/0/0、
exe_resources 1/0/0、startup 7/0/0、session_startup 9/0/0；child 使用编译时 target 产品，
已核与封存 EXE 前后同字节，不声称从封存目录启动 child。隐藏离线 --workspace-check 通过，非最终整包。
run-1 新 scope 用例 7 过/1 失败（夹具缺坐标元数据），保存组 21 过/2 失败（手工快照未带原 scope）；
仅修两份测试文件的三处夹具，input-2→3 无生产变化，重建并重跑全部选定门。失败输入/程序/PDB/日志保留，
未放宽断言、期限、生产校验或新增 allow/ignore。严格全目标 Clippy -D warnings、fmt、版本/组织合同通过，
3888 文件 ratchet 零违规/警告/新例外；相对 v404 native 21 变化/4 新增，最大 432 有效行。
源码 SHA-256：`f56ce7a95a5ee25d59315d542446ce054e6618f35821f92c5096acb711416a87`；产品 EXE 44518912 字节，SHA-256：`192787d1c5b8039b02dec86e5a00d0b211d75914aa6a8010816a4f756c558c46`。

实际发送菜单、源 PNG/工作区 projectionSenders binding/checkpoint/源失效后台交接、持续发送、
真实 Loom/离线互信、双向编辑/Tile/资源长稳、完整 N01–N12 与最终用户手测仍开放。
scope 不退休 sender outbox；仅确认解绑才沿原 sender 合同退休。未重跑完整 34 门，
v398 间歇呈现及历史严格前台/启动根因仍开放，不以本轮启动通过关闭旧失败。
目标 active、publishable=false；无新依赖、真实 profile、全局/前台输入、系统安装、默认 DLL 启用或 commit/push/publish。
下方 v404 为历史。

## v404：同 ID 创建恢复与显式解绑

发送意图已接原后台 Job/Lease/四槽，固定 read→明确 404 projection_not_found→一次 Create；
Create 失败仅再 read 同 ID 一次 reconcile，不重签、不换 ID、不重复 Create。每次 HTTP 独立 nonce，
原 Device/12 秒/6 MiB/共享预算、固定 origin、当前 actor、既有签名者和磁盘完整基线均须通过。
回复严格核原 envelope/target/linked、revision 不低于原签名源版本且同版本 digest 不变、receiver 成对与状态一致；
非空 editing 明确拒绝；跨重试的当前版本检查仍须后续工作区 binding owner 接线。
未绑定工作区前保留独立 sender 日志；Sent/未领取结果保原活动许可及四槽，不把网络接入当接收端显示成功。

projection_sender_activity 拥有同 key 串行许可和租约；response 拥有有界响应；原 HTTP child 拥有固定协议；
projection_sender_ready 保结果/原 Grant，原 work 分派 Send/CancelSend，不新增网络、身份或持久化 owner。
Ready 校验仍包含完整磁盘基线，属于后台 API；未来 UI owner 不得把大日志 I/O 放在消息线程。
取消先撤销活动租约，并在 state 锁后再次撤销以封住 activate 登记竞争；独立减权事务保原 envelope/PNG。
CancelSend 先持久取消，仅 unlinked:true 才 retire；失败/外改/旧 Request 保日志、不覆盖或退休新记录。
完成后的晚撤权压住 join，Ready Drop 不隐式 I/O；原意图保留，仍需 Application 后台显式 CancelSend，
不能据此宣称源生命周期取消或工作区恢复已完成。目标菜单仍只读，尚未启用发送 UI。

同源封存相关门：projection_delivery::sender:: 39/0/0；projection_delivery::runtime 9/0/0；projection_delivery::receive 5/0/0；projection_delivery::compensation 9/0/0；projection_delivery::tests 7/0/0；projection_delivery::accept::tests 5/0/0；projection_delivery::journal 20/0/0；device_authorization_lease::tests 69/0/0；device_identity 9/0/0；projection_targets 20/0/0。
过滤组有重叠，不累计独立覆盖；含真实 loopback TLS、read/Create 中 socket 中断、外改取消失败关闭、
未领取/Ready 保槽、false unlink 保日志、显式重试、旧 Request 不退休新意图、晚撤权清理和后台 CA/DNS。
身份缺失/更换、错误 actor、篡改持久签名、跨来源/到期授权及矛盾响应均不得获发送许可。
中间 Clippy 的新测试 slots 未使用及收紧断言的多余 unit 表达式错误均保留，分别显式释放/移除冗余表达式；
无 allow/ignore/断言或期限放宽。
首轮 sender 39/0/0 后，仅收紧 socket 断言，拒绝把超时当关闭；新输入重建并重跑全部选定门，旧程序/PDB/日志留存。
严格全目标 Clippy -D warnings、fmt、版本/组织合同和 3884 文件 ratchet 通过，
零违规/警告/新例外；相对 v403 native 19 份变化/10 新增，最大 352 有效行。
源码 SHA-256：`29cae005e79dac99435f515bdc7f1714e48ed34bbac73d9172cf313b3a5ec0f9`；产品 EXE 44494336 字节，SHA-256：`3c11b23238490cb9d94156cc5343a2de6525056b2c322c846000c8c4d714de4f`。
产品/PDB 独立封存，显式合成 profile 的隐藏离线 --workspace-check 通过，不是最终用户整包。

sender 工作区 scope/绑定/checkpoint/源失效后台交接、持续同步、真实 Loom/离线互信、双向编辑/Tile/资源长稳、
完整 N01–N12 和最终用户手测仍开放。未重跑完整 34 门；v398 间歇呈现失败及严格前台/启动根因未关闭。
目标 active、publishable=false；无新依赖、真实 profile、前台/全局输入、系统安装、默认 DLL 启用或 commit/push/publish。
下方 v403 为历史。

## v403：发送签名与独立写前意图

新增 projection_delivery::sender 的后台 API：只读已有设备身份，核 actor，完整校验 PNG 并签名，
成功落盘后才返回只读 Intent。没有联网 Create、Job/菜单发送接线、source binding/checkpoint 或持续发送；
目标菜单仍只读。本切片是实际发送前的持久依赖，不能当作发送端或 N09 完成。

projection_sender 拥有准备/取消；file 复用原 session 的完整基线、暂存、替换和恢复事务，
独立 projection-sender.json/lock，不使用 receiver Store/Ticket，不写接收策略或工作区。
record/wire 拥有 64 条/48 MiB 严格格式，数组第 65 项即拒绝，借用 Base64/键字段，
小 envelope 先过 8 KiB 门才分配；总字节计数先于序列化目的分配，PNG/Base64/原文沿共享预算。
snapshot 复用 WIC 完整解码、4 MiB PNG、8192 边长/16M 像素门，拒绝伪尺寸和坏图片。
envelope 的 signing child 复用原 Ed25519，精确保留旧十一项换行消息、270 秒期限和独立随机身份。
重开重试验证原签名，不因帧变化或期限变化重签；没有生成/迁移/修复设备身份，不把私钥/token 落入日志。

首写不可替换；相同稳定工作区 scope 摘要/来源/Unit/目标串行，不同 scope 不复用。
scope 摘要尚待 Application 保存目标 owner 接线，不从环境猜测路径；不接受 peer-target 离线地址。
取消独立持久保留原 envelope/PNG，重开不能复活；Intent 只读快照不是当前联网/授权/生命周期许可。
所有 Intent 保 Store 独占租约；外改/删除/未知格式/恢复歧义失败关闭。新内容已提交但恢复副本退休失败时
更新真实内存基线并报 RecoveryRetained，本 Store 后续操作受阻，不伪报回滚或继续副作用。

同源封存相关门：projection_delivery::sender::tests 19/0/0；projection_delivery::tests 7/0/0；projection_delivery::accept::tests 5/0/0；projection_delivery::journal 20/0/0；device_identity 9/0/0；projection_targets 20/0/0；projection_delivery::runtime 9/0/0。
过滤组有重叠，不累计独立覆盖；新链路含真实文件锁、四线程同 key、独立子进程和故意异常退出重开，
也验证 48 MiB 追加失败保旧文件、原签名独立验签、坏/缺/更换身份、持久取消和外改围栏。
严格全目标 Clippy -D warnings、fmt、版本/组织合同和 3874 文件 ratchet 通过，
零违规/警告/新例外；相对 v402 native 11 份变化/9 新增，最大 348 有效行。
中间 Clippy-2 的新测试冗余循环计数器报错保留，仅改用既有 index；失败输入不覆盖，无 allow/ignore/断言放宽。
源码 SHA-256：`262e3fd2db08d3cef82681402cd7eeabe4adb39e113f55b5bc1ef84d5f4fa552`；产品 EXE 44224512 字节，SHA-256：`5634e09a5f28fab4e215903fc8d85dfabc37f91f117f8ba86106c5bbfda36e46`。
产品/PDB 独立封存，显式合成 profile 的隐藏离线 --workspace-check 通过，不是最终用户整包。

实际同 ID read/reconcile/Create、严格 projection_not_found 分类、持久取消后的 unlink、sender 工作区绑定/恢复/
持续同步、真实 Loom/离线互信、双向编辑/Tile/资源长稳、完整 N01–N12 和最终用户手测仍开放。
未重跑完整 34 门；v398 间歇呈现失败及严格前台/启动根因未关闭。目标 active、publishable=false；
无新依赖、真实 profile、前台/全局输入、系统安装、默认 DLL 启用或 commit/push/publish。下方 v402 为历史。

## v402：发送目标目录原生链路

原生共享菜单新增“投送目标/刷新投送目标”，固定 POST /v1/projections/targets，沿原授权 Sessions、Lease、
四任务容量、后台 CA/DNS、独立 nonce 和 12 秒/1 MiB HTTPS 门；未新增网络或持久化 owner。
目录请求和失败都有原 Pending 的类型标记，由独立 application_projection_targets 消费，不进入 receive::Scope，
不更新接收轮询期限、不刷新接收 journal；本机接收暂停及缺少可保存工作区不阻止查询。
切服务、清会话、撤权、取消或退出后迟到结果不能复活；新查询先丢旧快照，失败不保留伪新鲜目录。

projection_delivery::targets 的 projection_targets 拥有有界目录/组，projection_target_routes 独立校验 route。
最多 64 目标/32 组；拒绝重复 ID、失效/重复组成员、非法计数、控制字符及超限名称；缺 groups/peerDirectory
兼容旧 Loom，显式 null 或未知状态拒绝。离线 ID 严格等于 peerId 与 remoteDeviceId 的 SHA-256 绑定，
保留传输可用性和完整/部分/忙/不可用目录状态。离线广告不等于原生已实现发送，菜单各目标/组仍只读。
device_projection_targets_ui 归原共享授权 View；菜单名截长/转义，子菜单异常释放、成功才由父菜单接管。
协议、真实 loopback TLS/任务、隐藏 HWND 菜单、Application 生命周期四份新测试各归原有测试 owner；
composition_relay_sessions_tests 保原收件容量测试，并复用同一逻辑验证目录与收件互斥、取消未领取保槽和关闭门。

同源封存回归：projection_targets 20/0/0；原任务 runtime 9/0/0、
授权 UI 18/0/0、Sessions 49/0/0；
另六项原收件菜单/失败/切换/恢复/撤权/退出用例分别通过；策略 29/0/0、
接收 owner 10/0/0、lifecycle 18/0/0、
reattach 9/0/0、sync 25/0/0、checkpoint 22/0/0。
过滤组重叠，不累计独立覆盖；未重跑完整 34 门，v398 间歇新帧呈现失败及严格前台/启动根因保持开放。
严格全目标 Clippy -D warnings、fmt、版本、组织合同、3865 文件 ratchet 通过，零违规/警告/新例外。
相对 v401 native 变更 21 份/8 新增，最大 363 有效行；UTF-8 无 BOM/增量空白审计通过。
首次 Clippy 的新测试初始化写法错误保留。run-1 的 14 过/6 失败均留证；六个应用场景在联网前的夹具断言失败，
实查 Application::new 会合成空白会话。仅新测试显式移除会话以构造原计划的无保存目标场景，原断言/期限不变，
input-2→3 仅该测试文件变化，生产源码不变；失败输入/程序/PDB/独立身份及原始日志未覆盖。
末次安全复核又将新测试的无写入断言对齐实际 projection-receive.json/lock/policy 文件名，
input-3→4 仍仅该测试文件变化；正式输入重新构建并重跑全部选定门，旧 run-2/product-final 留存。
未禁用诊断、放宽断言/期限、增加盲重试或 ignore。
源码 SHA-256：`999cf3a600115650426d17fbac4d04f3f2d0edbc4dbabf272e0a213062cb8150`；产品 EXE 44221952 字节，SHA-256：`df4a3baa54d9b0a00daf66b9ef280477e550054dac73d7e449d33860d7515ca3`。
产品及匹配 PDB 独立封存，显式合成 profile 的隐藏离线 --workspace-check 通过，不是最终用户整包。

本轮仅闭合发送目录依赖；签名创建前 durable intent、sender binding/恢复/取消、持续发送、真实 Loom/离线互信、
双向编辑/Tile/资源长稳、完整 N01–N12 及最终用户手测仍缺。目标 active、publishable=false；
无真实 profile、全局/CUA 输入、系统安装、默认 DLL 启用或 commit/push/publish。下方 v401 为历史记录。

## v401：本机接收暂停与手动模式轮询

共享菜单新增“暂停本机接收/恢复本机接收”，独立于 NEW 的自动/手动/拒绝同意策略。
复用原 profile 的 projection-receive-policy.json、版本 1 和 1 KiB 上限；旧文档缺 paused 兼容为未暂停，
无效 paused 或不可读文件按暂停处理。原 Staged/Replacement 完整基线事务保未知字段，commit 成功才安装新值。
暂停开始立即更换 Participation 代次并清旧菜单，写失败或暂停再恢复不复活旧 inbox/Ready/Preparing。
NEW Stamp 仍独立；更改 NEW 同意不撤销既有 accepted 的参与凭证，追加后 checkpoint/displayed/sync 不追溯撤销。

手动模式现在沿原 Sessions 单槽自动查询，NEW 只展示、不自动 Accept；accepted 仅在本地缺 journal/确定 Unit
且当前保存目标可安全追加时尝试一次只读 Reattach。既有记录不后台迁移到其他工作区，失败不自动解绑旧 receiver。
本地 Reject 与 InboxPolicy 分离，查询仍固定 confirm。旧 managed receiver 的连接配置归 Loom、固定 confirm；
本轮没有虚构 delivery settings 服务端读写 API，也未完成 managed origin discovery 或完整 N09 parity。
metadata 成功后从真实完成时刻等待 3 秒，给原 750ms sync 留窗口；显式成功/失败也先对齐 origin，
同来源不清 attempted，新来源才 reset，避免显式结果的 deadline 被首次 due 清掉。

生产变更仍归 policy file/job/schedule、共享 Consent、原 inbox/receive::Scope owner；不另开网络、journal 或工作区 owner。
新增测试按暂停事务、Confirm 查询、accepted 迟到生命周期及慢 inbox/sync 分责，复用原合成 CA/身份/隐藏 HWND。
application_projection_pause_tests 归 policy tests；application_projection_poll_tests 归 receive-owner tests；
application_projection_poll_reattach_tests 归 reattach tests；application_projection_sync_poll_tests 归 sync tests，不另建测试基础设施 owner。
手动协议夹具只将真实 deadline 延后以隔离无关轮询，未改变生产调度分支；专门自动用例显式恢复调度。
两项 run-1 超时经 run-2 固定阶段日志复现：IPv6 fallback 约 2 秒，加 3 秒调度等待或故意 3.1 秒响应，
碰到原共享 5 秒测试期限。拒绝用例仅推进传入 Instant，并核到期前不发请求/到期后自动拒绝；
新慢服务用例将连接/响应分段计时，响应独立 5 秒、整体等待有界 10 秒。原通用 5 秒、生产 12 秒和成功断言不变。
run-3 又发现旧 Reject 自动重建用例继承手动夹具的延后 deadline 却未恢复调度；仅补显式 resume 接线，原读请求/断言不变。
额外 authorization 整组运行碰到原 240 秒驱动上限，回执和精确归属回收记录保留，不算通过；最终按原命名空间互斥分区，
先枚举并核完整用例集合恰好覆盖一次，各分区仍限 240 秒，未增加 ignore、未遗漏用例或扩大单门期限。
首次 Clippy 私有测试入口错误及全部失败输入/程序/PDB/profile/原始日志均保留，没有盲重试或放宽呈现围栏。

同源封存回归：策略 29/0/0、reattach 9/0/0、owner 10/0/0、
lifecycle 18/0/0、sync 25/0/0、cleanup 13/0/0、checkpoint 22/0/0；
display-recovery 26/0/0、accept 合同 5/0/0、authorization Application 分区 121/0/0，
三个原围栏组分别 2/0/0、1/0/0、1/0/0；另三项原失败用例定点各 1/0/0。
过滤组重叠，不累计独立覆盖；本轮通过不能关闭 v398 的间歇新帧呈现失败，原根因保持开放，未重跑完整 34 门。
严格全目标 Clippy -D warnings、fmt、版本、组织合同和 3857 文件 ratchet 通过，零违规/警告/新例外。
相对 v400 native 变更 29 份/4 新增，最大 348 有效行；UTF-8 无 BOM/增量空白审计通过。
源码 SHA-256：`74453ad6dbfc359b7c9158b51786ff8872d18ef412eaabfcce738643a2d11739`；产品 EXE 43979776 字节，SHA-256：`dee7feb287a65d58a706c8c27a0490c18ff8cdd8cf9b3dce94b45269ff7d8402`。
产品及匹配 PDB 已封存，显式合成 profile 的隐藏离线 --workspace-check 通过，不是最终用户整包。
真实 Loom/离线互信/发送端/双向编辑/Tile/资源长稳、完整 N01–N12、严格前台/启动根因及最终用户手测仍缺。
目标 active、publishable=false；无真实 profile、全局/CUA 输入、系统安装、默认 DLL 启用或 commit/push/publish。下方 v400 为历史记录。

## v400：已接受投送的只读重建

共享菜单新增“恢复已接受投送”，沿原确认框核同一邀请 Arc 和活窗口后明确保存到当前工作区。
Reattach 只允许 accepted，底层再次核验；固定 POST /v1/projections/read、knownRevision=0，绝不重发 Accept。
响应仍核原 envelope、当前设备、确定 receiverUnitId、单调 revision、PNG 摘要/完整解码及共享预算；
非空 editing 文档明确拒绝，不能静默把双向编辑降级为单向 PNG。
复用原 profile Store/Attempt/Sessions 单槽/Lease/Ready/Hold/receive；Grant 和目标来源/几何/Document 围栏保留。
只有此已关联只读路径省略 NEW consent stamp；新接受仍受原不可复活 Consent 代次保护。
即使本地新建 journal，Reattach 的 fresh 也固定 false，读失败/撤权/切目标/迟到/取消不自动解绑旧 receiver。
仍须原 checkpoint 精确包含，再经真实新帧确认才发 displayed；不另开工作区或网络 owner，journal 格式未变。

自动/拒绝模式对 accepted 仅在 journal 缺失且当前无确定 Unit 时尝试一次；已有 journal 不后台迁移到其他工作区。
失败留下未结记录，可经显式菜单重试；显式恢复也禁止当前已有同 ID Unit。手动模式仍沿显式刷新，完整接收 parity 未关闭。
deadline 未到先返回，不准备目标 Scope/候选列表；每项确定 UnitId 只构造一次，不随当前 Unit 数重复分配。
变更归原 application_projection_inbox/receive::Scope/policy::Schedule 及 projection_delivery_runtime::Work/Attempt；
三份新测试按只读 HTTP、原生端到端和迟到生命周期分责，不新增持久 owner、依赖或系统配置。

同源封存回归：reattach 9/0/0、策略 16/0/0、owner 10/0/0、
lifecycle 18/0/0、sync 23/0/0、cleanup 13/0/0、checkpoint 22/0/0；
display-recovery 26/0/0、accept 合同 5/0/0，三个原围栏组分别 2/0/0、1/0/0、1/0/0。
过滤组重叠，不累计独立覆盖；这次 display-recovery 通过不证明修复 v398 的间歇新帧失败，原根因保持开放。
严格全目标 Clippy -D warnings、fmt、版本、组织合同和 3853 文件 ratchet 通过，零违规/警告/新例外。
相对 v399 native 变更 20 份/3 新增，最大 346 有效行；UTF-8 无 BOM/增量空白审计通过。
源码 SHA-256：`0af2dbe4eaf419ae5fc15bd74efb5b03079dcc2613fb8dc131ee62c3d2da340e`；产品 EXE 43973120 字节，SHA-256：`639ec6bfd48bbf51ee7a5d50c0c9871389a0bb3cc51211b0648b36e5ca002e38`。
产品及匹配 PDB 已封存，显式合成 profile 的隐藏离线 --workspace-check 通过，不是最终用户整包。
首次 Clippy 的测试借用参数错误和本地证据脚本失败均保留；未放宽断言/期限/呈现围栏或添加盲重试。
真实 Loom/离线互信/发送端/双向编辑/Tile/资源长稳、完整 N01–N12、严格前台/启动根因及最终用户手测仍缺。
目标 active、publishable=false；无真实 profile、全局输入、系统安装、默认 DLL 启用或 commit/push/publish。下方 v399 为历史记录。

## v399：设备级 NEW 投送接收策略

原生共享菜单已接入自动/手动/拒绝，所有窗口复用原 authorization View 的同一 Consent；
独立于服务端 InboxPolicy，查询仍用 confirm，不把本地拒绝写成 disabled。
设备同意文件 projection-receive-policy.json 归原 native profile/进程租约，版本 1、上限 1 KiB；
缺失默认手动，字段无效拒绝，坏格式/未知版本/不可读/恢复副本拒绝且不自动覆盖。
普通命名配置导入不能提升接受权；已知版本显式变更保未知字段。复用原暂存/完整基线冲突检测/
恢复事务；成功 commit 后才更新内存，失败不授 auto。策略切换使用不可复活代次，
治理原 Accept/Ready/Preparing 到 publish_append 之前；追加后原 checkpoint/report/recovery/sync 不追溯撤销。

projection_receive_policy 只拥有有界文档/枚举；application_projection_policy 归原 profile owner，
file/job/schedule children 分管事务、单槽 worker 和有界调度；device_projection_policy_ui 只排共享菜单请求。
自动路径复用原 Sessions/Lease/Scope/receive/cleanup，不另开 journal、网络或工作区 owner。
自动/拒绝常规轮询 3 秒，失败退避封顶 30 秒；公平轮转最多 64 项，不对连续保留的同一邀请版本后台重试 Accept。
手动模式沿现有显式刷新；服务端已 accepted 但本地没有 journal 的关联重建仍待补齐，不能声称完整 WebView parity。

本轮策略 16/0/0、receive-owner 10/0/0、lifecycle 18/0/0、
sync 23/0/0、cleanup 13/0/0、workspace-checkpoint 22/0/0；
另三个原围栏过滤组分别 2/0/0、1/0/0、1/0/0。过滤组有重叠，不累计独立覆盖。
严格全目标 Clippy -D warnings、fmt、版本、组织约束、3850 文件 ratchet 通过，零违规/警告/新例外；
相对 v398 停点 native 变更 25 份/11 新增，最大 365 有效行，UTF-8 无 BOM。
源码 SHA-256：`5a6d52e415f86f31a464c148f1e132db15dafb30d2ef27853c2d711bc247fd2d`；同源产品 EXE 43956224 字节，
SHA-256：`3a12098925eb625c5097b4c8286d1ae5037af0d7aabb4498d86f3327e62f2422`，仅通过显式隔离 profile 的隐藏只读启动检查，非最终用户整包。

保留全部失败输入：首次 check 的字节预算构造/测试字段、首次 Clippy 的测试私有入口调用；
run-1 策略组在共享菜单用例超时，诊断证明原授权队列错误要求设备策略先选服务。
仅为 ReceivePolicy 豁免该前置，其他校验/错误 MessageBox/断言/期限不变；自有失败进程核路径后停止。
run-2 用新源码完整重建/封存程序及匹配 PDB 后通过；失败程序/PDB/profile/log 不覆盖。
原 400 封存程序/29 诊断程序保持不动，另保本轮两份 harness 及产品与匹配符号。

v398 final-run2/final-run3 的间歇呈现失败和严格前台/启动根因仍开放，本轮未重跑完整 34 门，
不以本轮通过抵销旧失败。真实 Loom/离线互信/发送端/双向编辑/Tile/资源长稳、完整 N09/N01–N12、
最终用户整包和手测仍未完成；目标 active、publishable=false。无新依赖、系统 SDK 安装、默认 DLL 启用、
全局/CUA 输入、真实 profile 访问或 commit/push/publish。下方 v398 记录仅为历史快照。

## v398：单向接收端 PNG 持续同步

### v398 当前测试诊断：WM_PAINT 只读观察器

development-run-8仅改两个测试文件：display_recovery新增paint_trace child，归本夹具HWND所有，
完整转发原过程，记录WM_PAINT进入/返回、客户区、update region及原subclass/过程/线程身份；
RAII仅移除自己的subclass，不消费队列、不触碰BeginPaint或Canvas，不增加全局hook/输入。
新增真实隐藏HWND回归证明转发和移除；首次Clippy因GetWindowSubclass的可选指针参数编译失败，
修为当前windows crate的Some参数后严格全目标Clippy -D warnings、fmt和测试构建通过；失败输入/日志保留。

本次只跑一次带观察器的恢复组：观察器1/0/0、恢复26/0/0、呈现保护5/0/0。
成功路径观察为WM_PAINT 1/1→2/2、客户区160×80、update_pending=false，原subclass/过程/线程均匹配。
没有再次捕获原间歇失败，不以本次通过抵销final-run2/final-run3的33过/1失败，根因仍开放。
新诊断程序及匹配PDB、完整输入和三次源码身份已封存；source SHA-256：`827a1118c7c65e68785363612f8248ab8a9aeaf56d00ebf737d8d0c0ee97c07c`。
相对final-run3仅两个测试文件不同，产品实现未变；本停点没有重建产品EXE或重跑34门。
下方final-run3仅指其封存源码/程序，不冒充当前测试源码的新整包。

3839文件ratchet零违规/警告/新例外；v398 native变更39份/18新增，最大498有效行。
保400封存程序及29诊断程序；未放宽期限/围栏、未增加自动重试/后台displayed。
历史严格前台/启动、真实Loom、发送端/策略/双向编辑/Tile、资源长稳、完整N09/N01–N12和最终手测仍开放。
无新依赖、版本递增、系统安装、默认DLL启用、commit/push/publish或全局/CUA输入。

### v398 历史同源候选：final-run3，原间歇根因仍开放

该快照源码已重建并封存产品EXE、lib及六个集成测试程序共八件，另保匹配PDB；
source SHA-256：`b3b1ada83764b02523b65b9350f8c708f2bf9006b871d8ac64f98dc616bae3d2`，与development-run-7九组保护回归源码相同。
本快照34组选定运行门：33过/1失败；过滤组有重叠，不累计独立覆盖。
receiver-sync 23/0/0、display-recovery 24/1/0、receive-lifecycle 18/0/0、projection-receive-owner 10/0/0、authorization-application 99/0/0、workspace-checkpoint 22/0/0、startup 7/0/0、workspace_runtime 2/0/1。
本次再次捕获lost-reply/reopen第二次显式恢复失败：read已结束，但同步redraw前后paints/confirmed为[('(1, 1)', '(1, 1)')]，没有确认新帧，因而未发送displayed。此证据定位到呈现阶段，尚未证明redraw未分发的具体原因；不将其归因为网络或声称已修复。

严格fmt、全目标Clippy -D warnings/check、版本、3838文件ratchet通过，
零行数违规/警告/新例外；v398 native变更38份/17新增，最大498有效行。

集成harness使用编译期CARGO_BIN_EXE指向target产品；每门前及全部运行后核该产品与封存副本同SHA，
不是宣称child从封存路径启动。冻结输入、三次源码身份、原程序/诊断/失败输入均保留并核验。
产品EXE 43880448字节，SHA-256：`a7ce334997ef8e5f3204a861f90b0af4ae3b1b5433fb1894f38f7a32d12d208e`；publishable=false，未制作最终用户整包。

原final-run2的34门33过/1失败及旧同字节7过/2失败仍保留；本次结果不构成其根因修复证明。
诊断没有放宽围栏、原期限或授权，不增加自动重试/后台displayed；前台三个projection_visible_
用例仍明确不在这34门内，历史严格前台/启动根因未关闭。真实Loom、发送端/策略/双向编辑/Tile、
资源长稳、完整N09/N01–N12及最终用户验收仍未完成；不以本候选代完整迁移或正式发布。
仍为v0.2.33.398，无新增依赖、默认DLL启用、系统安装、全局/CUA输入或commit/push/publish。
后续保护400封存程序和28诊断程序。

### v398 历史诊断停点：诊断已覆盖 read 到 receipt，间歇失败尚未关闭

原 final-run2 的34组选定门仍为33过/1失败；旧同字节定点探测7过/2失败，
第二次显式恢复read后未收到displayed receipt的原日志、输入、八件和匹配PDB保持不动。
后续通过不抵销该失败，也没有声称已修复根因。

development-run-5 补入paint/confirmed前后计数、原始非S_OK HRESULT及首个文档失配类别；
第二次状态断言提前到peer join之前。限定12次定点探测均通过，恢复25/0/0、sync23/0/0、
呈现保护5/0/0；未捕获新的失败。其restore_guard过滤为0用例，明确不计作验证。

此前development-run-7补齐recovery worker/transfer、呈现后proof及displayed worker的封闭错误类别；
不打印Ready、token、图片、路径或远端正文。Visual/Document共用原条件和短路顺序，
新增自有隐藏HWND围栏回归，证明首失配优先及不消费待处理请求。
本快照：restore 1/0/0、idle 1/0/0、composition 2/0/0、sharing 4/0/0、presentation 5/0/0、recovery 25/0/0、sync 23/0/0、lifecycle 18/0/0、owner 10/0/0。
严格全目标Clippy -D warnings、测试构建、fmt、3838文件ratchet通过；
38份v398 native变更/17新增，最大498有效行，UTF-8无BOM。
中间一次Clippy的浮点否定比较和测试缺少import已作等价修正，失败输入/日志保留，不加豁免。
新HWND夹具首次运行因缺少窗口类注册失败；补齐自有注册/按序注销后再构建验证，不改断言。

这些修改只改善失败可观察性，不放宽成功条件、原期限或授权，不增加自动重试/后台displayed。
该诊断停点当时尚未重建产品EXE/完整八件或重跑完整34门；最新构建状态见上方final-run3。
保392封存程序和28诊断程序；原间歇失败、历史严格前台/启动、真实Loom、资源长稳、
完整N09/N01–N12及最终用户整包仍开放。无commit/push/publish或全局输入。

无新依赖、SDK用途、OS权限、线程池或网络/像素预算；定时复用原消息deadline。
已保存的 receiver 单向 PNG 关联可自动持续 read，使用当前 Binding revision 作为
knownRevision；unchanged 的 snapshot:null 不解码/换源，changed 严格校验完整 envelope、
原 intent/device/unit、版本单调、PNG digest/尺寸/像素预算。真实 nested error.code 的
八码终态仅停止关联，保最后内容和 journal；不重新 Accept、不发 displayed、不 unlink/retire。

唯一 receive owner 复用原 Job/Attempt/Lease/四槽，单在途、最多64关联；最旧due优先，
成功750ms、首失败1s并指数退避至30s。先剔除stopped再限64；无关联/授权不设常驻轮询，
忙时清wake避免过期deadline忙循环。首次自动准入需真实canonical包含证明，普通导入/
recovery-copy不借恢复索引自动联网；连续推进仍核原workspace/source/完整Binding/Document
及原Grant，晚保存/真实工作区交换/本地编辑/撤权/切来源/模态均不能发布旧候选。
只读sync不锁住用户保存/恢复准备，回收前仍保原活动许可和共享任务槽，fresh=false不获补偿。

远端底图更新不新增本地undo条目；兼容时同时重基当前及全部undo/redo的共享底图，保矢量、
独立raster标注、crop、history cursor、Unit/frame/opacity。后台contain投影保持原逻辑框，
原始PNG另随Source保存；运行时旧输出/显示缓存一并失效，避免后续save写回旧outputs。
已有Blur/破坏性历史/缩略模式等不兼容状态只拒绝changed像素，仍读取metadata/终态。
原始像素尺寸/SHA证明随Binding保存；旧Binding无证明或烘焙擦除后真实重开均保本地图，
不把丢失历史当未编辑底图。大像素SHA分块取消，编码/原文序列化共用原Lease撤销标志；
JSON解析仍受原会话上界限制，不宣称任意CPU步骤可抢占或资源长稳已验。

本轮是单向receiver PNG持续同步，不是双向编辑/journal parity；自动下游重执行、
真实Loom跨端、发送端/策略/离线互信/Tile、完整N01–N12、历史严格前台/启动根因、
资源长稳和最终用户整包仍未完成。没有自动show/抢前台或后台displayed重放。

仅分配一次 v0.2.33.398；保全部dirty edits，不提交/推送/发布。
首轮check因Schedule可见性及RawValue移动借用失败，第二构建因Pixels无AsRef失败，
均精确修正，不扩大公网API。首个封存候选sync 7/4/0：新擦除夹具低于原4像素最小宽度，
以及新迟到夹具轮询抢先领取；仅修夹具合法输入和真实worker完成/未领取边界，保原断言/期限。
第二封存候选sync 21/0/0，但原display_recovery 10/15/0：显式恢复结束后旧due自动read
在同一轮抢跑。修生产交接：显式操作结算后重新等待750ms，且不缩短原失败退避；
不cfg(test)禁用自动同步、不加retry或放宽期限。只读复核另发现独立false取消标志，
改复用原Lease并给大像素SHA加分块检查；输出原文与运行时旧端口一并清除。
第三封存候选sync 22/0/0、display_recovery 25/0/0、lifecycle 18/0/0、owner 10/0/0。
预审发现editor原500有效行仅因新增两行模块接线到502；将新source child接到原raster
owner，主editor恢复原字节，不添加行数例外或无关重构。最终同源程序还需独立全门验证。
首次final-run在严格Clippy因两类大枚举、复杂tuple、布尔表达式及测试模块顺序失败；
改用Box负载/具名replacement结构/等价判断/模块顺序，无lint豁免，原冻结输入和失败日志保留。
交叉复核另发现Syncing时拒绝显式Display会污染UI busy，改按原前台网络/receive phase
计算UI busy，不借被拒请求撤销有效sync，新增真实菜单拒绝后继续第二次read回归。
所有失败/中间程序、输入、日志及合成profile均保留；构建/运行中不编辑生产源码或测试。

最终 final-run2 严格 fmt、全目标 Clippy -D warnings/check、版本与
3838 文件 ratchet 均通过，零违规/警告/新例外；
33 份 native 变更/17 新增，最大 498 有效行，UTF-8 无 BOM。
同源八件的 34 选定运行门 33 过/1 失败；
本候选运行门未闭合：display-recovery 24/1/0，lost-reply/reopen 用例的 TLS peer 等待请求超时；稍后的重叠 authorization Application 组虽通过，不能抹去该失败，根因仍待定位。
原始运行结果如下：
receiver_sync 23/0/0、display_recovery 24/1/0、receive_lifecycle 18/0/0、
receive owner 10/0/0、cleanup 13/0/0、
authorization Application 99/0/0、workspace checkpoint 22/0/0。
过滤组重叠不累计独立覆盖；startup 7/0/0、workspace_runtime 2/0/1。
原期限/ignored 不改，不关闭历史启动/严格前台根因。本轮明确不运行 projection_visible_
三个用例，其余通过不能覆盖 v394 失败；隐藏窗口同步不是严格前台验收。
无真实 Loom/真实 profile/全局输入，不代表全库、资源长稳、完整 N09/N01–N12 或最终整包验收。

冻结输入 2686 件、七文档、源码三次一致、scoped diff 和
384 旧封存程序/21 诊断均由本轮 closeout 重新核验；本轮另保留 3 个开发运行程序，
后续保护 392 封存程序/24 诊断。debug EXE 43872768 字节，SHA-256：
`a76baa07a9185f55f24206cd6f1a0c74994cbf77628cfd0881e95ec598c630c4`；source SHA-256：
`4764b341908759217daadfd4677a5f48e60c5e827fd7dbce0255cea55b015a26`。证据位于
otter-native-v398-receiver-sync/final-run2，publishable=false，总目标未完成。

## v397：已保存投送的显式只读回执恢复

无新依赖、SDK 用途、OS 权限、任务池、计时器或网络/像素预算；read 复用原 accept 预算和解析器。
新增已保存候选的显式“重试呈现回执”：原 canonical checkpoint 完整 Binding、
Workspace IdentityStamp、原 Source、保存 Stamp 及目标 Document 的显示/config/output
凭证必须同时有效；热路径只查内存身份，不重新解析磁盘或图片。真实 commit/规范启动
同步提升受预算共享显示/原始来源像素身份；manual save 改 Source 后须先新 canonical
commit，recovery-copy、cleanup:true、在途编辑/模态/撤权/工作区交换均不获得恢复资格。

原串行 Job/四槽在后台 activate 原 Ticket，固定 origin 认证 read knownRevision=0，
带许可 Ticket 复用原 Attempt 保到回收，HTTP/解码/像素失败及 synthetic panic 也不提前
释放同 intent 许可；恢复一律 fresh=false，不授自动补偿。不重新 Accept。
复用 Received/Verified 的完整 PNG SHA/设备/快照校验，严格比较完整
Binding 和解码原始来源像素；恢复后的逻辑投影不冒充原 PNG。保原 Visual 围栏，真实
present 后及 Reporting 期间再次核严格 canonical 条件，typed recorded:true 才确认。
失败保本地内容/journal，不追加 Unit、不替图、不 unlink/retire，不换 Grant 复活旧任务。

只提供显式恢复/重试，不在启动、授权或定时器上自动 show/read/report；同设备/epoch 的
服务端 displayed 幂等可覆盖丢失回复，但客户端不宣称独立 durable displayed outbox。
持续 read/同步、自动退避重试、发送端/策略/离线互信/双向 journal/Tile、完整 N01–N12、
真实 Loom 跨端、历史严格前台/启动根因、资源长稳与最终用户整包仍未完成。

仅分配一次 v0.2.33.397；保全部 dirty edits，不提交/推送/发布。
开发首轮 check 因 E0624 调用私有 validate 失败，改在 HTTP 层核原 Ticket origin，
不扩大校验器可见性。测试构建 E0599 来自新夹具误用 Workspace::new，改用原 Default；
后一构建成功但有 unused_mut，已去除，不增 lint 豁免。
首个开发运行停在新 cleanup 负向夹具：误用真实 UI 错误包装，触发不可见 MessageBox。
核准本轮 sealed harness PID/路径后控制停止；保完整失败 EXE/profile/输入/日志，
不算产品网络死锁或用例通过。负向校验改走原 Application 入口及独立菜单 request 测试，
不改产品警告框、断言或期限；只读 CUA inventory，未发送任何 CUA/全局输入。
第二个开发候选恢复 20/0/0、lifecycle 18/0/0、owner 10/0/0，原程序独立保留。
随后只读复核确认失败恢复 worker 提前释放同 intent 许可，已最小复用原 Attempt：
带许可 Ticket 保到 join/Drop，成功移交原 Ready；fresh=false 永不产生自动补偿。
新增 HTTP/解码/像素失败未领取与 synthetic panic join/Drop 回归；原始 2×1 图和
160×80 逻辑投影的真实 profile 重开用例保留。不以中间候选代最终冻结结果，
构建/运行中不编辑生产源码或测试，所有失败输入和程序均不覆盖。
第三个开发候选严格 Clippy -D warnings、测试构建及恢复 25/0/0、lifecycle 18/0/0、
owner 10/0/0 均通过；两个预期 synthetic panic 由原 Job 正确回收，不是未处理测试失败。

最终 final-run 严格 fmt、全目标 Clippy -D warnings/check、版本与
3821 文件 ratchet 均通过，零违规/警告/新例外；
24 份 native 变更/8 新增，最大 320 有效行，UTF-8 无 BOM。
同源八件的 33 选定运行门 33 过/0 失败；
display_recovery 25/0/0、receive_lifecycle 18/0/0、
receive owner 10/0/0、cleanup 13/0/0、
authorization Application 87/0/0、workspace checkpoint 22/0/0。
过滤组重叠不累计独立覆盖；startup 7/0/0、workspace_runtime 2/0/1。
原期限/ignored 不改，不关闭历史启动/严格前台根因。本轮明确不运行 projection_visible_
三个用例，其余通过不能覆盖 v394 失败；新恢复呈现证明不是严格前台验收。
无真实 Loom/真实 profile/全局输入，不代表全库、资源长稳、完整 N09/N01–N12 或最终整包验收。

冻结输入 2669 件、七文档、源码三次一致、scoped diff 和
376 旧封存程序/18 诊断均由本轮 closeout 重新核验；本轮另保留 3 个开发运行程序，
后续保护 384 封存程序/21 诊断。debug EXE 43672576 字节，SHA-256：
`72a4345895147417128c3ce8f69601cc87bd6393304b1a1a93e1c73119128d6d`；source SHA-256：
`7dbd536a82e5b3593de612e4710ba64a37366e74f9b6d542f61c816068c4d26f`。证据位于
otter-native-v397-displayed-recovery/final-run，publishable=false，总目标仍 active。

## v396：Ready 后生命周期补偿与真实保存稳定化

无新依赖、SDK 用途、OS 权限、线程池或网络/像素预算。只补本次原 Store
新建意图的减权移交，原 marker/CAS/活动许可/认证 unlink/typed unlinked:true 退休链不变。
持久保存时同步放弃自动补偿；Index 未知只降级显式恢复，不伪造 durable 或 displayed。

仅分配一次 v0.2.33.396；保全部 dirty edits，不提交/推送/发布。
开发首轮 check 成功但有 unused_mut；首个测试构建 E0624 来自新测试调用私有 wrapper，
改用原已有可见的 poll_restore_with，不扩大生产可见性。首个运行 6/12/0，失败为
新夹具用旧 cleanup:false 快照激活（正确被 Changed 拒绝），以及坏索引夹具改到了本次
Binding；改为当前 marker 原 intent 许可核验、在另一真实 Unit 上注入坏标记。
保留完整失败程序/profile/输入/日志，不以修正后通过抹去失败；构建/运行中不编辑源码。

最终 final-run 严格 fmt、全目标 Clippy -D warnings/check、版本与
3813 文件 ratchet 均通过，零违规/警告/新例外；
18 份 native 变更/3 新增，最大 313 有效行，UTF-8 无 BOM。
同源八件的 32 选定运行门 32 过/0 失败；
新 receive_lifecycle 18/0/0、receive owner 10/0/0、
accept_abandonment 11/0/0、cleanup 13/0/0、
authorization Application 65/0/0、workspace checkpoint 22/0/0。
过滤组重叠不累计独立覆盖；startup 7/0/0、workspace_runtime 2/0/1，
原期限/ignored 不改，不关闭历史启动/严格前台根因。本轮明确不运行 projection_visible_
三个用例，其余回归不能覆盖 v394 的失败；无真实 Loom/真实 profile/全局输入，不代表
全库全部测试、资源长稳、完整 N09/N01–N12 或最终用户整包验收。

冻结输入 2661 件、七文档、源码三次一致、scoped diff 和
368 旧封存程序/16 诊断均由本轮 closeout 重新核验；本轮另保留 2 个开发运行程序，
后续保护 376 封存程序/18 诊断。debug EXE 43620352 字节，SHA-256：
`1774a184d74099020e3a616142fe65e444f6a5d01f83c1f3466b55d1cb1c9956`；source SHA-256：
`91b85ff7524844c6d95fec4be6e137fc2a6183592baa273a55c65bd494862534`。证据位于
otter-native-v396-receive-lifecycle-cleanup/final-run，publishable=false，总目标仍 active。

## v395：Accept 网络阶段的精确未结交接

无新依赖、SDK 用途、OS 权限、线程池或网络/像素预算。只有原 Store
本次新建且成功激活的未移交意图获得自动减权资格；复用旧意图保留显式恢复。
原活动 Ticket 留到 join，marker 用无许可同 intent 快照；不跨身份/来源换票据，
不在 UI 正常结算触盘，不以 HTTP 错误或撤权伪造 unlinked:true。marker 前崩溃仍保
写前意图但未必已有 cleanup 标记，旧 reader 对 cleanup:true 的 fail-closed 边界不变。

仅分配一次 v0.2.33.395；保全部 dirty edits，不提交/推送/发布。
开发首轮 all-targets check 的 fixture stream 类型错误及 unused warning、修正输入独立保留。
首轮针对性运行 10/1/0 的失败仅在新 WM_QUIT 夹具读取；完整失败程序/profile/日志保留，
后续沿仓库原 1024 消息 smoke 泵，不调用会提前 refresh profile 的 Application poll。
不以修正后通过抹去原记录；构建/运行门期间不编辑源码。

最终 final-run 严格 fmt、全目标 Clippy -D warnings/check、版本与
3810 文件 ratchet 均通过，零违规/警告/新例外；
17 份 native 变更/4 新增，最大 292 有效行，UTF-8 无 BOM。
同源八件的 31 选定运行门 31 过/0 失败；
新 accept_abandonment 11/0/0、cleanup 13/0/0、
receive owner 10/0/0、authorization Application 47/0/0、
workspace checkpoint 22/0/0，过滤组重叠不累计独立覆盖。
startup 7/0/0、workspace_runtime 2/0/1；原期限/ignored 不改，
不关闭历史启动或严格前台根因。本轮明确不运行 projection_visible_ 三个用例；
其余依赖回归通过不能覆盖 v394 的失败。无真实 Loom/真实 profile/全局输入，
不代表全库全部测试、资源长稳、N09/N01–N12 或最终用户整包验收。

冻结输入 2658 件、七文档、源码三次一致、scoped diff 和
360 旧封存程序/15 诊断均由本轮 closeout 重新核验；另保留本轮失败开发程序，
后续保护 368 封存程序/16 诊断。debug EXE 43613696 字节，SHA-256：
`f550b43900750f0744a6544cf8ce3b021453b11c850ab8624f81b02bf58ec035`；source SHA-256：
`1f031d8b30e4d0c8d80957ac460658d261954ee837c271060ea417ed73e5c694`。证据位于
otter-native-v395-accept-abandonment/final-run，publishable=false，总目标仍 active。

## v394：已保存接收贴图删除清理

无依赖/SDK 用途/OS 权限或预算扩张。cleanup:true 仅单向减权；原 Store/完整 intent/磁盘 CAS/同 intent 活动许可不变。原 origin/device 的当前 Lease 只授解绑，typed unlinked:true 才退休；403、负回执、撤权不等成功。旧记录缺字段默认 false；旧 native reader 遇新 true 字段会 fail closed，不是完全双向兼容。

最终 final-run2 严格 fmt、全目标 Clippy -D warnings/check、版本和
3806 文件 ratchet 均通过，零违规/警告/新例外；27 份 native 变更/
7 新增，最大 456 有效行，UTF-8 无 BOM。同源八件 47 选定运行门
46 过/1 失败；cleanup 13/0/0、新 TLS 4/0/0、
投送桌面 48/1/0，严格前台断言仍失败，不豁免或以其他重叠组覆盖。
过滤组重叠不累计独立覆盖。
workspace_runtime 2/0/1，正式前台菜单退出仍 ignored；startup
7/0/0 不关闭旧偶发根因，原期限/断言不变。无真实 Loom/真实 profile 或
全局/CUA 输入，无新依赖/SDK 用途/权限/预算，完整迁移和最终用户整包未验收。

首个候选 46 过/1 前台断言后的次生 panic abort，原八件/输入/日志完整保留；只修
夹具 unwind 并补主 panic 回归/前台 PID-class 诊断，不放宽断言或关闭首次根因。
完整输入/七文档、开发失败与旧 352 件/七诊断保护见迁移台账 v394/closeout。

## v393：显式可见接收链

无依赖/SDK 用途/OS 权限或预算扩张。显式确认前后核同 Arc/发起 HWND；原 Source/Grant/Store/Record 与精确 checkpoint 复核。呈现排除隐藏/零尺寸/零 opacity/cloaked/offscreen/旧帧，回执只用原 Lease/Slot/intent；取消不提前释放，撤销后停用已过期 deadline。displayed 不退休 journal，自动补偿和在线服务安全验收仍缺。

最终严格静态/3799 文件 ratchet 零违规/警告/新例外；同源八件 45 选定门全过。
24 变更/7 新增、最大 381 有效行，UTF-8 无 BOM；visible 3/0、present 5/0、
placement 2/0、确认 1/0，过滤组不累计覆盖。344 旧件/七诊断、开发失败/输入说明、
最终冻结输入/七文档与源码三次匹配见迁移台账 v393/closeout。startup 7/0 不关旧根因，
正式前台菜单仍 ignored；无真实时钟到期/worker panic 全链注入，不代完整迁移。

Otter applies inventory, advisory scanning, triage, remediation, and regenerated
release evidence as one supply-chain control loop. A green scan is a release
gate; it is not proof that unknown vulnerabilities do not exist.

## Deployed controls

v392 无依赖/SDK 用途/OS 权限/网络或像素预算扩张。唯一接收 owner 在发网前冻结
目标，同 Source allocation/Space/Guard 与同 Store/Record/原 Grant 逐阶段复核；
相同字节/同 origin/device 不构成接管权。Job 独占原 Hold 至回收，错误/panic/取消
不提前释放四槽/活动许可，不复用新 Grant 越过撤权。单回调在锁外通知并隔离 panic，
复用原 Wake/绝对到期等待，无新增等待句柄、轮询器或第二任务池。只隐藏追加并核
精确 checkpoint，不发 displayed/自动解绑/退休 journal/删除图片。首轮失败及修正
证据保留。最终严格静态/3,792 文件 ratchet 零违规/警告/新例外，同源八件 41 选定运行门全过；
25 变更/6 新增、最大 454 有效行，UTF-8 无 BOM，owner 10/0、watch 2/0，过滤组不累计覆盖。四份开发输入、
最终 2,640 冻结输入/七文档、首个静态候选/失败程序/析构诊断、336 旧件/五诊断及
后续 344 件/七诊断保护见迁移台账 v392/closeout。startup 7/0 不关旧根因，前台
菜单仍 ignored；不代在线扫描、可见业务、自动补偿或完整安全/迁移验收。

v391 无依赖/SDK/OS 权限/网络预算扩张。接收结果单次绑定原意图，拒绝旧结果重贴
新 UUID；会话关联严格核设备/envelope/Unit/intent，完整快照还要精确比 revision/
digest。有界索引由原后台 worker 解析并绑定 Source allocation，真实 checkpoint
提交/规范启动才提升；坏/旧标记不推断、不覆盖或阻止普通保存。菜单分类不授
网络/显示权，不退休 journal 或删除本地数据；历史提交事实不等持续文件存在性。
最终严格静态/3,786 文件 ratchet 零违规/警告/新例外，同源八件 39 门全过；28 变更/
4 新增、最大 349 有效行，UTF-8 无 BOM。关联模型 5/0、checkpoint 新组 5/0/全组
18/0、恢复菜单 4/0、journal 15/0/补偿 9/0；过滤组重叠不累计覆盖。四份开发输入、
最终 2,634 冻结输入/七文档、首个静态候选与 null 兼容修正后 final-run2、328 旧件/
五诊断及后续 336 件/五诊断保护见迁移台账 v391/closeout。startup 7/0 不关旧根因，
前台菜单仍 ignored。未来接收状态须保原 Grant 跨异步生命周期，不把本轮局部证明
当在线扫描/真实 Loom/自动补偿/displayed/全面安全或迁移验收。

v390 无依赖/SDK/OS 权限/网络预算扩张。默认 profile 恢复后后台单次打开 Store，
坏/锁冲突不降级覆盖、不停用正常 workspace checkpoint，也不自动重试或发 HTTP。
未结投送只供显式减权：终态确认默认否，前后核同一候选和 busy/来源/退出/模态，
Application 再核 profile Arc；当前授权只用于原设备/原 origin 解绑，重试快照
不复制活动许可。只在原任务回收时对账内存提交，丢弃成功 UI 不复活已退休项，
失败不删意图/本地图片。最终严格静态/3,782 文件 ratchet 零违规/警告/新例外，
同源八件 37 选定门全过；19 变更/5 新增、最大 453 有效行，UTF-8 无 BOM。
profile 4/0、恢复菜单 3/0、Application 解绑 8/0、journal 15/0，过滤组不累计
覆盖。两份开发输入、2,630 冻结输入/七文档、初始空过滤/夹具失败程序、320 旧件/
四诊断及后续 328 件/五诊断保护见迁移台账 v390/closeout。startup 7/0 不关旧
偶发根因，前台菜单仍 ignored；不代在线扫描、真实 Loom、
checkpoint 包含/可见 displayed、自动补偿或完整迁移/全面安全/用户包验收。

v389 无新依赖/SDK/OS 权限或预算扩张。接收写前队列 128 项/512 KiB，不存 token/
PNG；来源和 Lease preflight 先于落盘，完整磁盘基线先于重试。原设备/原 origin
的明确授权只用于减权解绑，不授权新 Grant 领取旧 Ready；typed unlinked:true 才
退休，403/取消/坏回复/失败不推断已解绑。独占 Store 跨任务共享，同 intent 移动
许可防并发/过期 Ticket，Drop 无磁盘删除；坏/满/竞争/恢复歧义 fail closed。
原 CAS/句柄/祖先 pin/预算 owner 复用；已提交但旧副本未退休明确报告，不冒充
回滚。最终严格静态/3,777 文件 ratchet 零违规/警告/新例外，同源八件 34 选定门
全过，16 变更/6 新增、最大 440 有效行，UTF-8 无 BOM。journal 14/0、认证补偿
9/0、投送客户端 49/0，含丢回复/重启、旧 Ticket/同 intent 独占、403/坏回复/
撤权/竞争写者负向门；过滤组不累计覆盖。六份开发输入、最终 2,625 冻结输入/
七文档、312 旧件/四诊断及后续 320 件保护见迁移台账 v389/closeout。startup
7/0 不关闭旧偶发根因，菜单仍 ignored；不等于在线扫描/真实 Loom/全面安全；
菜单/工作区意图对账/可见 displayed/自动退出补偿及完整迁移验收仍未接入。

v388 接收准备内核无新依赖/SDK/OS 权限或资源预算。accept 的 6 MiB wire、
4 MiB PNG 与解码像素计入既有共享预算，PNG 原字节验摘要，WIC 核实际尺寸后才
成为 Verified；Ready 保原 Lease/四槽，同 origin/device 的新 Grant 不可接管。
网络成功不等于本地落盘或显示；接受菜单与 displayed 均未开放，持久取消补偿仍
必须补齐后才能开放消费者。测试只用回环 TLS、合成身份、linshi profile/自有隐藏
HWND。最终严格静态/3,771 文件 ratchet 和同源八件 32 个选定门通过，包含原 Grant/
撤权/四槽、HTTP 独立限额/坏 PNG、checkpoint 重启像素及库/归档源预算回归。
31 变更/12 新增、最大 367 有效行，UTF-8 无 BOM；大枚举 Box 化不加 warning 豁免，
首轮 Clippy exact 输入及所有开发失败证据另保。304 旧件/四诊断与后续 312 件保护
见迁移台账 v388/closeout。startup 7/0 不关闭旧偶发根因，菜单仍 ignored；不作为
在线扫描、真实 Loom、持久补偿/displayed、全面安全或完整迁移/用户包验收。

v387无依赖/SDK许可/OS权限/资源预算扩张。单次投送查询/拒绝沿原Device认证/Lease/
有界解析/四槽/HTTPS取消，菜单不持token；当前Arc身份/accepted/origin三重准入，
配置/CA准备不在菜单联网。取消未回收仍占槽，closed/discarded结果不发布，成功
workspace交换清候选，失败恢复不误清；模态/恢复/退出阻止新任务。仅recorded:true
算拒绝成功，无accept或伪造displayed。测试复用回环CA/合成身份/隔离profile及自有
隐藏HWND，无系统证书/注册表/真实Loom变更。最终门及296旧件/四诊断见迁移台账v387，
不作为全面安全/在线扫描/完整接收/长稳/用户包批准，不关闭旧前台/压力未知失败。

v387最终final-run严格fmt、全目标Clippy -D warnings/check、版本及3,759文件ratchet
均过，零违规/警告/新例外；19份native变更/7新增，最大452有效行，UTF-8无BOM。
同源八件24个选定后台/隔离门23过/1失败：投送桌面13/0、原投送客户端16/0；授权菜单/
Application、远程共享容量/关闭、HTTP/Lease/退出drain、保存/恢复/消息等待及资源/
session/workspace启动门同过，过滤组有包含关系不累计独立覆盖。原workspace_runtime2/0/1 ignored
完成移动提交→强制中断→重启恢复→删除最后Unit落空，保原15秒和断言；正式菜单
退出仍ignored未验。startup6/1在透明贴图原15秒超时，最后输出crop-effect完成，stderr
为空，不能断言卡死在下一步骤或归因本轮接线。单次开启原计时开关的同一测试1/0，
未复现且成功路径不回显child阶段日志，不关闭首次失败、不循环重跑。三次源码/
296旧件/四诊断/本轮2,606开发失败输入、2,607最终失败输入及v385完整现场由closeout
复核，后续保护304件。状态projection-desktop-inbox-verified-startup-timeout-pending，
不代accept/下载/呈现/displayed/同步/Tile、旧前台失败、完整N09/迁移或最终用户包。

v386无依赖/SDK许可/OS权限/资源预算扩张。每文档窗口最多一个queued几何通知，
无payload所有权/全局表/线程/定时器；State只在拥有HWND的UI线程访问，销毁禁投递，
失败不保pending。WndProc不进行磁盘或持有跨重入模型借用，保原Receipt/CAS/路径/
文件锁。测试只用linshi隔离profile及自有隐藏/离屏HWND，无全局/CUA输入、无系统
证书/注册表变更。最终严格门及288旧件/四诊断见迁移台账v386，不当全面安全批准。

v386最终final-run严格fmt、全目标Clippy -D warnings/check、版本及官方ratchet
均过，零违规/警告/新例外；7份native变更/3新增，最大479有效行，UTF-8无BOM。
同源八件21个选定后台/隔离门全过：wake3/0、checkpoint8/0（含两项新隐藏HWND），
原workspace_runtime2/0/1 ignored完成移动提交→强制中断→重启恢复→删除最后Unit
落空；保原15秒和断言。正式菜单退出仍ignored未验，不关闭v384前台或旧完整组/
压力失败。三次源码/288旧件/四诊断/v385完整失败输入及原现场由closeout复核，
后续保护296件；状态window-geometry-wake-and-workspace-runtime-verified，不代完整迁移。

v385无依赖/SDK许可/OS权限/资源预算扩张。传统邀请仅形状/固定HTTPS来源验证，
不声称本地crypto验签；Debug隐去nonce/签名。收件元数据限1MiB/64项，整批校验，
固定route沿原Control的Device/独立nonce/前后期限/取消，只有recorded:true成功；
无accept或伪造displayed。过期pending可提交拒绝，认证token/Lease期限并未放松。
原4槽由Job保到领取，后台CA/DNS，撤权后不发布，Drop取消/join、不detach；未来
桌面owner仍须核workspace/selection并避免UI阻塞回收。只跑回环CA/隔离profile，
不改系统证书或真实身份。最终门及280旧件/四诊断见迁移台账v385，不当全面安全/
在线扫描/真实Loom/完整迁移或最终用户包批准，旧前台/压力失败不关闭。

v385最终final-run2严格fmt、全目标Clippy -D warnings/check、版本及3,749文件ratchet
均过，零违规/警告/新例外。10份native变更/7新增，最大362有效行；四生产模块
168/111/29/128，三tests186/151/196，UTF-8无BOM。同源八件的13个选定后台/隔离
门12通过/1失败，projection16/0、device HTTP144/0（包含新9项，不能累计独立覆盖）。
workspace_runtime1/1/1 ignored在checkpoint-moved-window原15秒超时，尚未进入中断/
重启/删除链；失败profile仍revision7原坐标，stderr仅热键冲突，不足以确定根因。
菜单前台门没有重跑，ignored不计通过。三次源码/
旧280件/四诊断、首轮静态失败完整输入及288件后续保护见final-run2/closeout；
状态projection-client-implemented-workspace-runtime-pending，不代桌面/N09或完整迁移。

v384无生产代码/依赖/SDK许可/权限/预算变更。测试matcher仅规范ASCII类名大小写，
保PID/精确owner/visible及确认标题；真实离屏NOACTIVATE正负回归RED0/1→GREEN1/0。
开发正式菜单退出1/0只操作自启隔离profile，不触真实数据、不注入全局输入或绕过
前台保护。最终同源候选、272旧程序/四诊断、静态及运行实绩见迁移台账v384；不
作为在线扫描、完整安全/迁移或最终用户包批准，旧前台/压力未知失败仍开放。

v384最终严格静态/3,742文件ratchet及18后台门通过；两份测试变更/一新增最大197
有效行。独立正式菜单门0/1在原15秒前台前提失败，不继承开发1/0或以ignored代
通过，不扩大前台许可/期限。三次源码/失败完整输入/七文档/八件/旧272件/四诊断
及280件后续保护见closeout，状态native-exit-menu-foreground-pending；SDK/权限与
预算不扩，仍非完整安全/迁移/长稳批准。

v383无新依赖/SDK/OS权限/资源预算。最终退出只复用原单槽/Receipt/全字节CAS及
已授权的可写profile；只读或stopped不重新授权。一次性批准保轻量模型/来源/完整
Live几何，不保GPU/视频历史/控制权，提交与销毁均复核，晚到意图不被越过。无首帧
保窗口并报告，正常stale取消退出，磁盘冲突停用一次且保外部字节；测试只用linshi/
合成流/自有隐藏或离屏PID。最终门与身份见迁移台账v383，不当在线扫描或全面批准。

v383首轮17/19门通过，startup4/3为通用退出批准误用restore idle的兼容性回归，产品
前台0/1未进入完整菜单链。完整失败输入/八件封存保留；final-run2不删原退出清理
断言、不放宽保存Receipt/restore guard/CAS，只分离无profile批准与严格profile快照
空闲。修正候选不重复争抢前台或继承首轮原生通过结论，菜单仍未验，ignored不计通过。

v383最终严格静态/3,741文件ratchet及18选定后台/隔离门全过。退出11/0覆盖无profile
明确放弃的execution/preview清理、可写profile初始busy及提交前/后晚到execution保
窗口，原Receipt/restore guard/CAS未放宽；相邻Live58/0含该11项，startup恢复7/0。
菜单退出1 ignored未验，状态exit-checkpoint-native-pending；20变更/6新增最大450行、
三次源码/八件/旧256件/四诊断、首轮失败完整输入/八件及272件后续保护由closeout
复核。无SDK/依赖/权限/预算扩张，不作为在线扫描、完整安全/迁移或最终用户包批准。

v382无新依赖/SDK/OS权限/既有资源预算。Live自动保存只落静态时间点，目标/HWND/
控制权限不进入文档；轻量基线不持GPU帧、不按视频帧写盘。无首帧/忙手势延后，
真正错误沿原自动路径停用一次；Receipt/全字节CAS/文件租约及原编码上限不放宽。
freeze/recycle只在可领取时消费一个，终态单次内部唤醒，不建无界重试或跨工作区
授权。测试只用linshi合成内容/隐藏HWND/现成子进程owner，最终实绩见迁移台账v382；
不作为在线扫描、完整安全/迁移或用户包批准，不关闭旧前台/完整组/压力未知失败。

v382最终严格静态/3,735文件ratchet和17相关门全过，无ignored；新Live8/0覆盖首帧/
流不重写/原目标与dirty保持/忙手势stale/静态恢复/多请求续进及跳过Drop的中断。
完整相邻Live组47/0包含新8项，原冲突/profile/普通产品恢复门同过，不放宽原文件锁/
路径准入。18变更/4新增最大437行、三次源码/八件/旧248件/四诊断、开发失败日志及
256件后续保护见迁移台账v382/closeout；不当在线扫描或完整安全批准，SDK用途不扩。

v381无依赖/SDK/OS权限/既有资源预算扩张。默认workspace与旧目录隔离；租约文件
share_mode(0)及不共享DELETE的祖先目录句柄保护重定向，拒reparse/parent/超过128组件。
真实有界读取和未知schema/坐标准入仍归原session owner，错误不降级覆盖；唯一恢复
副本只读，含混副本拒绝。检查点复用全字节CAS/共享锁/暂存sync/不覆盖rename，失败
停用一次、保内存/外部文件；独立revision仍单次有界编码，不绕过Receipt或改手动dirty。
检查入口不发现真实profile，仅隔离workspace内容只读；lease文件不删除，进程异常退出
由OS回收句柄。Live停用自动检查点并提示显式快照，不复活捕获/控制能力。开发失败、
最后Unit并发占槽修复及最终静态/运行/身份见迁移台账v381，不当完整安全或在线扫描。

v381最终严格静态/3,731文件ratchet与14相关运行门全过，含跨进程租约、外部冲突不
覆盖、唯一副本只读/多副本拒绝、坏/未知/缺坐标文件保留、真实中断后最近成功快照
恢复及删除最后Unit写空。缓存修订不替代全字节比较；恢复GUID夹具修正不放宽matcher，
共享测试进程夹具单点加载，不allow Clippy警告。25变更/8新增最大447行、三次源码/
八件/旧240件/四诊断/失败完整源码及248件后续保护见迁移台账v381/closeout，不关闭
旧前台/压力失败，不作为在线扫描、完整安全/迁移或最终用户包批准，SDK用途不扩。

v380无依赖/SDK/OS权限/资源预算变更。静态工作区自动保存须当前进程显式开启且已有
保存目标；不发现旧目录、不默认开启、不持久化授权。代际保护off→on，成功换工作区
或另存目标失效；正常手势busy/过期结果不发布，磁盘冲突沿原全字节比较/共享锁保护，
失败停用一次，不反复覆盖或重试旧候选。手动/打开请求不消费丢失，退出先回收自动
候选，不绕过Receipt或自动执行Loom工作流；Live显式拒绝，避免恒Changed触发无限写盘。
smoke只创建并删除自身linshi GUID文件，无系统剪贴板/前台/全局输入。开发失败与最终
静态/原生/程序身份见迁移台账v380，不当在线扫描、完整安全或迁移/用户包验收。

v380最终严格静态/3,723文件ratchet及10相关运行门全过；自动保存8/0覆盖off→on、
关闭在途候选、外部冲突不覆盖、打开/手动请求保留、退出原确认及代际耗尽两种状态
均停用；真实startup7/0、session_startup9/0通过。19变更/4新增最大432行，构造移动
保原token和字段顺序，不放宽Receipt/文件锁或新增豁免。源码/六件/旧234件及四诊断
与240件后续保护见迁移台账v380/closeout；未重跑旧前台失败或扩大SDK许可/预算。

v379无新依赖/SDK/产品权限或预算；仅测试层可选启动握手及精确父/Child前台许可。
G只解除外层就绪屏障，不创建业务权或跳过真正foreground/focus/epoch/FIFO准入；
默认/产品不等待。Child只处理自启父PID和所持Child，禁止ASFW_ANY；保原命令/确认/
stdout/生命周期和Drop上限。真实点击仅经当前自有窗口核验，不向游戏发消息、修改
系统锁或重放拒绝请求。诊断授权失败及peer期限失败保留；最后实际输入导致父前台
前提失败，新增归还路径未验证，停止后四项前台最终门不执行。后台/静态/身份见迁移
台账v379，不能称后台拒绝全部通过、旧失败修复、在线扫描、OS隔离或完整安全验收。

最终八后台门全过，含跨进程UIA退出撤销和摘要绑定实际产品Enter worker各显式1/0，
Engine121/0/5 ignored、Session77/0/26 ignored与资源/启动/worker通过，ignored不计
通过。四份测试变更最大467有效行、严格静态/3,719文件ratchet、三次源码/六件/旧228
件、四诊断和234份后续保护见迁移台账v379。同源前台0/1和四项未重跑门仍开放，
不作为完整安全/迁移批准，也不扩大原SDK许可或产品资源/权限预算。

v378无新依赖/SDK/权限/资源预算。边框CSS仅ASCII有界值、非负px/关键词和三种线型，
简写完整校验后才写私有cascade键；未知线型/image/逻辑属性原子拒绝，原型污染不调用
公开String/Object入口，边框非继承。Native保原长度/节点/场景上限，分区最多七顶点，
四边共用一份有界factory缓存；D2D push/pop与antialias恢复配对，失败COM转换先于push。
临时GDI region错误路径同样释放，EDIT字段合同安全门未放宽。开发真实RED、像素漏洞
修复与child/隐藏HWND/EDIT回归见迁移台账v378；最终22相关门全过，包含摘要绑定产品、
原子拒绝/污染/回收及四原规模/原预算压力，ignored不计通过。严格静态/3,719文件
ratchet、三次源码/六件/旧222件与后续228件保护见同台账，不代表在线扫描、
OS权限sandbox、真实Lease/业务或长稳验收，不关闭旧前台/压力失败或扩大SDK许可用途。

v377无新依赖/SDK/权限/预算。CSS只准入clip/ellipsis及受限nowrap/left/hidden文字叶，
原私有记录/捕获intrinsic/非继承/未知值和压平run拒绝不绕过。DWrite sign/布局随
原帧COM资源释放，不增加跨帧缓存或将完整文本截断；仅新建尚未共享对象可配置，
原节点/文本/尺寸上限不扩大。Unicode强制多行与非法native值在最终准备拒绝；旧
安装帧应保持原子性。真实负向/污染/child回收、像素与最终运行身份见迁移台账v377，
不代表在线扫描、UIA TextPattern、OS权限sandbox、真实Lease撤销/业务或长稳验收。

最终v377的19相关门全过，包含实际产品worker摘要绑定、真实child回收/原型污染/
不支持上下文拒绝、坏候选保旧帧；四个原预算压力门显式通过。异常trimmed零高
命中仅恢复DWrite行高，inside不扩大到文字整盒，隐藏后缀范围有上界检查。严格
静态/3,708文件ratchet、三次源码/六件/旧216件及后续222件保护见迁移台账v377。
行为RED与开发失败保留；v376两项前台失败及压力未知根因不因本次成功关闭，不作
性能不退化、完整安全/迁移或最终用户包批准，原Otter隔离SDK用途/预算不变。

v376无新依赖/SDK/权限/资源预算。computed的winner仅由原准入canonical CSS键写入
捕获intrinsic创建的null-prototype record；不读取公开Object/Map方法或继承属性，
不保跨帧缓存。新真实QuickJS污染/差分2/0、原rank oracle与Engine114/0/Session74/0
通过；20门18过/2前台前提失败，显式跨进程负向未进入产品拒绝主体，不能称全部
安全负向通过。四压力本次过但墙钟短样本未证加速，不据此关闭历史失败或扩大
32MiB/256KiB/250ms/Job预算。四变更最大386行、严格静态/三次源码/六件/旧210份与
两失败见迁移台账v376，后续保护216份；无全局输入/重试，不作在线扫描、真实Lease
撤销、OS权限隔离、默认完整JS/真实业务/长稳/完整迁移批准，原Otter隔离SDK许可不变。

v375最终仅修旧拒绝样本，生产owner与首轮一致；21门17过/4压力失败，前三可观测
路径Collect后期限失效，产品缺详细诊断，不以首轮成功/功能通过关闭压力根因或
宣称安全验收。预算与原子拒绝断言不改，首轮完整窗口前台失败未重跑求绿。十一
native变更最大437行、严格静态/三次源码/六件/旧204份及首轮身份见迁移台账v375；
下一轮保护210份，不发布、默认启用完整JS或扩大原Otter隔离SDK许可。

v375无新依赖/SDK/权限/资源预算。author :hover只读原Hover owner可信目标的当前
私有父链，32层有界/root连接判定，不读公开属性或递归computed；synthetic事件不
授权样式状态，未知pseudo仍原子拒绝。suspend/dispose/leave沿原owner清理，无新
进程/线程/队列/句柄或持久缓存。受控RED与synthetic嵌套布局触栈失败分别保留，
未提高32MiB heap/256KiB stack/250ms/Job/startup预算；最终产品/压力/相邻门和身份
见迁移台账v375，不宣称在线依赖扫描、OS隔离、Lease撤销或完整安全/迁移验收。

首轮22门18过/4失败，新的author Hover与产品/四压力门通过不代表全部回归；旧
拒绝样本在语法支持后过时，须保原子断言改用未知pseudo。完整窗口真实前台前提
失败保持开放，不归因未经核实的外部进程，不重跑请求/放宽准入或预算求绿。首轮
源码/六件/旧198份与后续候选独立记录，见迁移台账v375，不代最终安全验收。

v374无新依赖/SDK/权限/资源预算。只建立测试真实前台前提，产品focus源码移除
临时观测后与before一致；当前epoch的后台请求仍拒绝，不抢前台/入队/恢复重放。
独立进程窗口仅来自所持Child且核PID，沿原有界stdout/命令/30秒生命周期/Drop，
foreground模式不伪造UIA action；cfg(test)可见性仅限surface_window子树。无全局
输入、失败请求重试或产品准入宽限。最终相关门与身份见迁移台账v374，不关闭旧
未知根因或声称在线扫描、真实Lease撤销、OS隔离、默认完整JS与最终安全验收。

首轮11门6过/5失败保留；updated不再断言跨TLS批次拥有前台，真正focus/blur交付
点检查不削弱。新跨进程测试仅改显式执行分类，独立--ignored门仍强制运行，不能
用默认ignored覆盖负向失败；Foreground仅加track_caller，不扩大权限或重放请求。
首轮完整Application13/12等失败与修正候选实际成绩分别见迁移台账v374。

第三候选仅向持有的Child管道发一次foreground命令，由目标线程核真实前台后确认；
不对其他进程发送激活消息，不引入全局输入/重试或产品宽限。原5秒确认、64KiB
stdout、64字节四条命令、30秒生命周期及Child Drop保持；被动guard复用原有条件
归还owner，前两候选和最终10门分别留证，不将失败归因未知外部进程或隐去显式负向。

最终10相关门全过，显式后台focus1/0、Application focus3/0、默认完整窗口129/0/
1 ignored、UIA及摘要绑定实际产品keyboard通过；该ignored另有显式门实际通过。
六变更最大321行、严格静态/3,696文件ratchet与三次源码/六件/旧192份、前两候选
和诊断身份见迁移台账v374/closeout。产品focus源码与before一致，无新依赖/权限/
预算/SDK用途；旧完整组失败、真实Lease/OS隔离/长稳和完整迁移安全验收仍不关闭。

v373无新依赖/SDK/权限/资源预算，只修原测试owner的modifier夹具。每条普通消息
保存完整线程键盘表，中和modifier按下位并在正常/unwind恢复，不发送全局输入，
不改变产品焦点/前台/策略/快捷键准入。raw消息负向与恢复回归的最终实绩、受控
Ctrl RED及身份见迁移台账v373；不声称v372未知失败根因已关闭、OS沙箱/真实Lease
撤销或默认完整JS获准，原Otter隔离SDK许可不扩大。

最终fixture7/0含受控modifier/raw拒绝/Shift反向与unwind恢复，完整窗口129/0、
实际产品keyboard1/0及资源/启动/worker通过；8门7过、1失败，Application14/11的
EDIT焦点断言/十项锁中毒保留，不作所有安全负向或历史根因已关闭声明。唯一测试
文件152→296行、严格静态/3,696文件ratchet与三次源码/六件/旧174份/三诊断身份见
迁移台账v373/closeout，仍partial-runtime-failure，无在线扫描或最终安全验收声明。

v372无新依赖/SDK/线程/权限或资源预算。hover只采当前可见启用前台Surface的无按键
鼠标，允许原EDIT持焦但不转移焦点/捕获；私有wire拒未知字段、非法target/modifier与
非有限越界DIP，不含HWND/远端权限。原Queue统一容量/sequence预留两个terminal，硬取消
撤销waiting而不重放prepared，MAX复用已有槽。relatedTarget由原EventTarget brand验证，
native carrier与捕获intrinsics不读公开原型状态，不绕过JSON/emit/TLS/Grant/viewport。
同child/rAF/HWND/产品/Application及失败的真实成绩见迁移台账v372，不声称在线扫描、
OS权限沙箱、真实Lease撤销或默认完整JS放行；SDK许可仍仅限既有Otter隔离构建。

最终Hover HWND6/0、child4/0、独立Application1/0、产品1/0；完整JS189/0、十四产品
worker及三原规模压力门通过。34门32过、2失败：完整Application13/12为Enter发送前
丢前台后十一项锁中毒；完整窗口122/4为四项旧键盘焦点/候选直接失败，根因未定，
不能称为全部安全负向已过或用独立成功覆盖。35文件/12新增/最大500行、严格静态/
3,696文件ratchet、2,544输入与三次源码/六件/旧168份见迁移台账v372/closeout，仍
partial-runtime-failure，不关闭历史未知根因或扩大默认JS/权限/SDK许可。

v371无新依赖/SDK/权限或资源预算。Canvas Pointer需当前前台/父HWND焦点、可见启用、
local editing及当前Canvas目标；私有wire拒未知字段/非有限越界坐标/非法phase或modifier。
terminal共享原容量/sequence预留，硬取消撤销waiting不重放prepared，MAX序号复用槽。
SetCapture同步重入复验owner，ReleaseCapture只豁免预期capture-change，不屏蔽隐藏/
失焦。事件family自有属性/brand、null-prototype carrier与捕获intrinsics保护私有边界；
不动JSON清洗/payload/emit，Pointer取消不暗自改变普通click合同。原TLS/Grant/batch及
viewport校验保留，policy关闭测试不替代真实Lease撤销。实绩/失败/身份见迁移台账v371；
不声称在线扫描、OS权限沙箱、默认完整JS或最终安全验收，隔离SDK许可仍仅限Otter。

最终Pointer HWND9/0/child5/0/产品1/0，完整窗口120/0、JS184/0及三压力门通过；27门
26过/1失败，Application13/11为Enter前台获取失败及十项锁中毒。新增Pointer固定一次
独立Application1/0不覆盖完整组失败，不把policy测试称为真实Lease撤销。37文件行数、
严格静态/3,684文件ratchet及三次源码/六件/旧162份见迁移台账v371/closeout，仍partial。

v370无新依赖/SDK/权限/预算。cascade保留分层优先级与源序，仅缩减私有rank/对象分配；
winner自有数据字段只在单次computed内更新，不将插件对象、可变sheet或跨帧状态作为
缓存。新字段原型accessor/节点属性隔离、旧算法差分及原压力三路径实绩见迁移台账v370。
JSON getter/toJSON清洗和payload标准语义完全不动，不以私有容器为由绕过copy；不作在线
扫描、OS沙箱、默认完整JS或最终迁移安全批准，原Otter隔离SDK许可和历史未知根因不变。

最终严格静态/ratchet通过，24运行门23过、1失败：压力三路径各1/0、完整JS177/0、
窗口111/0及资源/启动门通过；Application13/10前台失败后锁中毒不能由其他组覆盖。
新旧库相邻固定矩阵均15/15，不能清除历史压力根因；三诊断、两文件行数和三次源码/
六件/旧156份身份见迁移台账v370/最终closeout，保持partial-runtime-failure与既有边界。

v369无新依赖/SDK/权限/预算。只修测试Foreground guard的清理所有权：确定性自有HWND
反例证明无条件恢复覆盖后来选择，现只归还仍持有的target或已毁target留下的空前台；
其他新前台不覆盖，测试cleanup亦不覆盖外部窗口。不改变原获取流程、AttachThreadInput
边界或生产前台准入，不增加重试、等待、全局输入/键盘重放。候选诊断一次通过不关闭
旧失败；RED/编译失败与最终实绩/身份见迁移台账v369，不作前台独占、OS沙箱、默认JS、
在线扫描或完整迁移安全批准，原Otter隔离构建SDK许可不扩大。

最终严格静态/3,671文件ratchet通过；12门10过、2失败，无deferred。Application23/0、
values5/0、child/队列/摘要绑定产品keyboard、startup7/0及资源/worker门通过；专门
foreground1/1及完整窗口93/18的前台获取/锁中毒分报，后台Enter负向最终未进入体。
后来选择的RED→通过已证，unwind完整窗口组通过不能覆盖专门门获取失败，不作前台
稳定性或安全负向全过声明。首轮Clippy失败、四变更最大192行、三次源码/六件/原150份
见迁移台账v369/最终closeout，保持partial-runtime-failure，不清除历史未知根因。

v368无新依赖/SDK/权限或预算。Enter诊断证明本次keyup被原前台门正确拒绝，不放松
生产检查；测试每段用原Foreground guard建立并恢复真实前台，每次发送前断言，失败
不重发。真实HWND负向证明后台down/up不入队、不在恢复前台后重放；无全局输入注入、
系统设置修改或产品自动激活。观测/最终实绩/身份见迁移台账v368，不据此清除所有
历史未知根因、旧压力/form/startup失败或声称默认JS/完整迁移安全批准。
新驱动开发验证仍0/1，负向1/0；确认前台被当前游戏使用后不操作其进程，先暂停交互
验证。用户随后允许短暂占用前台，最终恢复完整10门矩阵，不用background-only。
真实终态另记，不以授权、运行启动或更准确的前提报错代替安全/功能通过。

最终严格静态/3,670文件ratchet通过，无重编完整10门8过、2失败，未跳过交互门。
Application13/10为前台fixture失败+九项锁中毒；SurfaceWindow90/19含点击候选未入队、
另一前台fixture失败+17项锁中毒，新增负向最终取锁失败，不能算安全负向证明已过。
child/队列/摘要绑定实际产品keyboard、startup7/0及资源/worker门通过，原预算不变。
身份/三次源码/六件/原144份与失败分解见迁移台账v368/最终closeout，保持partial-runtime-failure；
不据新HWND推断进程归属，不作在线扫描、默认JS、OS沙箱或完整迁移安全批准。

v367无新依赖/SDK/权限/预算。稀疏枚举仅针对私有null-prototype计算样式，class正则只
插入已准入ASCII标识符且固定空白边界，无任意正则/插件回调；构造器与exec提前捕获。
sheet有序声明是唯一规则存储，原replace/adopt预算和原子提交、inline/层叠/资源期限
不改。差分/污染/原压力/child/实际产品实绩见迁移台账v367，不扩大默认JS或把Windows
Job说成OS权限沙箱；旧keyup未知根因与完整迁移/真实业务/资源长稳安全验收仍开放。
私有JSON的bulk描述符表先去原型，再用捕获hasOwn拒绝accessor；非枚举和Symbol仍忽略，
不执行getter/inherited toJSON。阶段模型的2秒诊断不进入生产代码或代替原250ms完成门。
匹配context只在无插件回调的当前computed内共用私有state/ordinal，不跨帧复用；原有
mutation identity及祖先位置失效不变，不建立对外可覆盖的缓存或绕过当前权限链。

v367最终严格静态/3,670文件ratchet通过，24实际运行门18过、6失败，另两诊断exit0
不算吞吐验收。原256节点/16规则/250ms的Engine/child/产品均0/1，Application13/10、
JS137/38/26 ignored及startup4/3的直接失败/锁中毒/原15秒期限分别保留，未知根因
未关闭。三次源码/六件/原138份、12文件行数和精确身份见迁移台账v367/最终closeout，
保持partial-runtime-failure；不是新在线扫描、OS沙箱、默认JS或完整迁移安全批准，
不提高预算或扩大SDK许可，不以受控中断或诊断退出0代替必须完成的压力断言。

v366无新依赖/SDK/权限/默认能力或资源预算。An+B仅接受有界ASCII整数子集，拒绝
未知of-selector/函数/小数和超界系数，计算在精确整数范围；原选择器长度/段数/
测试数、sheet/规则/节点和heap/stack/250ms/Job/startup不变。位置WeakMap复用原
mutation identity换代，弱键不保留断开的节点，不增加历史、宿主读取或远端authority；
全部操作沿原捕获intrinsic。原子拒绝/原型污染、child/产品/HWND的实际验证和失败
见迁移台账v366，不是在线扫描、OS沙箱、默认JS/完整股票或完整迁移安全验收。

最终严格静态/3,664文件ratchet通过，14运行门13过、压力门0/1失败；原256节点/16规则
仍触发原250ms，不能以受控中断当吞吐通过。首轮Application13/10与单次Enter复核1/0
分别保留，keyup根因未清除，最终不同字节未重跑完整Application组。三次源码/六件/
原132份身份及partial状态见迁移台账v366，不放宽预算、提升默认权限或扩大SDK许可。

v365无新依赖/SDK/权限/默认能力或预算。ASCII/换行扫描只调用捕获的native exec，
私有固定字符类/非空分段无插件回调，循环与分配受原65536 units/48KiB限制。Unicode
在清理前完整拒绝NUL/畸形代理项，失败不写dirty值；不以去换行拼接非法代理项。
测试专用CPU/阶段探针不进入产品协议；原型数值槽污染仅围绕被测校验调用finally
恢复，保留QuickJS native stringify失败证据，不声明全局污染后任意操作均可继续。
验证/身份见迁移台账v365，不是在线扫描/OS沙箱、默认产品/发行或完整安全批准，
原heap/stack/250ms/Job/startup/SDK许可与历史未知根因不变。

最终严格静态/3,658文件ratchet及43选定封存门通过，值校验3/0/2 ignored、完整窗口
107/0、JS164/0/20、Application23/0、十四外部SHA绑定产品worker各1/0等见迁移台账
v365。七个native变更最大422行、四新增最大92，无新例外；三次源码/六件/旧120份、
固定前后探针及原型夹具失败分别留证。探针改善不等完整资源/长稳、安全扫描或历史
所有未知根因已清除；原输入/权限/默认能力及正式发行边界不变。

v364无新依赖/SDK/默认权限或预算扩张。Keyboard carrier仅含有界target/phase/七bit
flags，拒绝未知字段/HWND/远端action，不夹带旧value；私有取消状态不读公开反射。
与文本共用原32槽/30秒/scope/fence和Session ticket，默认change只替换原在途槽；
无同步child等待/系统按键重放。接纳后核当前字段、dirty代次、焦点epoch/前台/策略/
租约，失效默认不重放。消息编号已核唯一并补回归；Host生命周期等价抽出不增加权限
owner。实际验证/失败/身份见迁移台账v364；不是在线扫描、OS沙箱、默认JS/正式发行
或完整迁移安全批准，原SDK许可、heap/stack/250ms/Job/startup与旧未知根因不变。

最终严格静态/3,654文件ratchet通过，38个native变更最大493有效行，无新例外。41
封存门首轮40通过、1窗口门失败；Enter11/0/1 ignored、Application23/0、JS161/0/18、
十三外部SHA绑定产品worker各1/0等通过。旧输入夹具mount超时与两项锁中毒留证；
固定六门同字节复核含窗口107/0通过，不改预算/源码、不覆盖失败或关闭deadline
未知根因，closeout保留partial-runtime-failure。身份/旧114份/精确边界见迁移台账
v364，不是在线扫描、OS权限沙箱、默认产品/正式发行或完整安全验收。

v363无依赖/SDK/默认权限或预算扩张。焦点carrier仅含有界target/focus、每operation
最多32项，拒绝未知字段/非法ID；私有factory不读公开DOM反射或被覆盖的collection
方法。宿主只使用当前原Mount/语义句柄，不接收child的HWND、激活、路径或远端动作。
同一前台Surface/输入策略/已确认owner和focus epoch共同准入，ABA/后台/隐藏/撤权
请求丢弃不重放。SetFocus不跨Host借用，失焦仍走原change/FIFO；应用唯一策略在回复
前先收紧。原32MiB/256KiB/250ms/read/layout/Job/startup和SDK许可不变；验证/失败见
迁移台账v363，不是新在线扫描、OS权限沙箱、默认JS/发行或完整迁移安全批准。

最终严格静态/3,645文件ratchet及37选定封存门通过；焦点9/0/1 ignored、JS157/0/17、
Application22/0、十二外部SHA绑定产品worker各1/0、startup7/0。24个native变更最大
499有效行、八新增最大168，无新例外；前台测试支架仅同步关联并立即解除线程队列，
不放宽产品foreground检查、不注入全局输入或改系统设置。失败/源/程序/旧108份见
迁移台账v363/closeout，不把选定green提升为旧未知根因已清除或默认/发行安全批准。

v362无新依赖/SDK、默认能力/权限或原heap/stack/250ms/Job预算扩张。文本carrier
只含target/phase/value，拒绝未知阶段/字段、不安全ID、NUL/CR/LF和超过48KiB的值，
没有远端action、HWND或凭据。DOM程序值原子校验/去CRLF，私有投影与native接纳不
调用插件可覆盖的value/event/collection方法；仍受原文档/heap总量约束。原IME/
32槽FIFO/30秒意图期限/click fence/草稿ticket复用，DOM帧接纳结算本地ticket不等
远端emit成功。真实污染/失败回收/HWND/Application/产品成绩见迁移台账v362；不是
新在线漏洞扫描、OS权限隔离、发行批准或完整迁移验收，旧未知根因和SDK许可不变。

DOM原始值另限65536 UTF-16 units；反射描述符可被覆盖，但native接纳/投影不调用
公共value getter/setter。最终严格静态/3,637文件ratchet及33选定封存门通过，输入
7/0/1 ignored、JS153/0/16、Application19/0、十一产品worker各1/0、startup7/0。36个
native变更最大483有效行、八新增最大175；descriptor污染失败和锁中毒保留，身份/
旧102份/精确子集见迁移台账v362。不将选定成功提升为未知根因已清除或默认批准。

v361无新依赖/SDK/默认能力/权限或预算扩张。EventTarget每文档lifetime最多2048个
（含Node）、每target活跃listener16、type256 UTF-16 units、path/嵌套dispatch各32；
原heap/stack/250ms/父侧/Job预算不变，非空signal明确拒绝。私有WeakMap/列表只走
捕获intrinsic，returnValue不调用插件可覆盖的公开preventDefault，passive不能取消。
public dispatch不可信；原宿主click通过私有dispatcher标trusted不授予远端动作，
emit仍经原snapshot/父侧authority/Grant/FIFO。callback异常清理后保持原fail-closed，
不是浏览器异常report或OS权限sandbox。真实回归/产品成绩见迁移台账v361，不等
在线扫描、安全批准、原生键盘/pointer/input、完整迁移或旧未知根因已关闭。

native click只使用原Engine本operation内的JS job边界，bootstrap await失败；不增
跨operation队列或重置总期限。私有非thenable carrier不读Node.then，固定私有
constructor/species与捕获then不经过插件Promise劫持。原栈下Grid写入/空间投影
失败分别由直接Engine确认，不以首次压平后单项成功解释完整回归或旧未知故障。

最终严格静态/3,629文件ratchet与29封存门通过，事件10/0/1 ignored、JS149/0/15、
Application18/0、十个摘要绑定实际产品worker各1/0、startup7/0。19个native变更最大
386有效行、八新增最大148，无新例外。UIA夹具、两次同步栈和六模型接线失败均
保留；源码/六程序及旧96份由final-run closeout绑定，不因新切片成功清除旧未知
根因或宣称在线漏洞扫描、OS隔离、发行批准、默认JS及完整原生输入/迁移验收。

v360无新依赖/SDK/默认能力/权限或原预算扩张。padding值最多256 ASCII bytes、1–4
项0或0–8192px，全量成功才展开提交；未知值不忽略，非法声明不部分覆盖或使
几何缓存失效。四边Insets复用原Length/重排预算，四角几何最多随原节点数增长，
缓存仍绑定原D2D工厂并由帧释放，不引入全局缓存、网络、线程或新原生输入owner。
重入/污染/child/产品及真实HWND的实际成绩见迁移台账v360；不是在线漏洞扫描、
OS隔离或发行批准，完整迁移、真实股票/Loom及旧Canvas/启动未知根因仍开放。

最终全目标严格静态、3,621文件ratchet/六程序构建及26封存门通过，padding43/0/1
ignored、JS141/0/14、九摘要绑定产品worker各1/0与startup7/0。首轮旧双值拒绝及
scalar投影测试预期产生的JS99/42/14失败保留，最终仅修两个测试输入，不放宽
生产准入/原预算。40个native变更最大455有效行、无新例外；三次源码/程序摘要、
旧90份与相关进程见迁移台账v360/最终closeout，不提升在线扫描、OS隔离或默认能力。

v359无新依赖/SDK、权限、默认能力或预算扩张。新私有CSS fonts统一原family边界
及shorthand四项准入，font原始输入512 UTF-8 bytes，其他项/规范化family256 bytes，
列表仍最多8项/每项31 UTF-16 units，拒绝未知样式/URL/下载；先验完整四项再提交，
不放宽原250ms/heap/stack/Job/read/layout/startup门。unitless/native em与normal重置
不创建新资源owner。污染/原子拒绝/真实child和产品成绩见迁移台账v359；不是在线
安全扫描、OS权限隔离、发行或完整迁移批准，旧Canvas/启动未知故障与SDK许可不变。

公开Proxy在私有声明链前完成用户转换，原投影单调用层合入prepare以保重入栈；
不提高256KiB/250ms或新增产品异常字段。固定受控夹具在真实Engine复现的栈溢出
及修复后聚焦8/0/1 ignored分别留证；消费者22条值探针不执行业务、不授予能力，
不能据此提升安全/完整CSS或股票Loom验收。最终静态/封存运行见迁移台账v359。

最终全目标严格静态、3,612文件ratchet/六程序构建和21封存门全部通过；JS136/0/13
ignored、typography8/0/1、八实际产品worker各1/0、资源与startup通过。16个native
变更最大380有效行，无新例外；最终摘要、三次源码和旧78份由closeout核验，开发
失败证据留存。该新重入机制修正不外推旧未知故障，不提升默认能力或发行批准。

v358无新依赖/SDK/默认能力或原预算扩张。font-family名称和规范化输出有界，最多
8项/256 UTF-8 bytes，每项31 UTF-16 units；不接收URL/转义/@font-face。原文字
Engine按需查询已安装family，不开启更新/字体下载或向脚本暴露集合；缓存最多
64项，Rc随场景/Engine释放。先完整验证再提交原CSS状态，拒绝不使几何缓存失效。
实际原子性/污染/资源/原生回归见迁移台账v358；不是逐字形作者字体链、新在线
安全扫描、OS权限隔离、完整迁移或发行批准，旧未知根因与SDK许可范围不变。
最终values在trim前先验字符域；255个NBSP加a的511字节反例原被少计后移除，现
通过共用fonts夹具验证原子拒绝。纯Node生产值探针只证明准入分支，native/产品
另验；最终生产JS也变化，不把中间fixture-only构建当作最终同源证据。

final-admission严格静态/3,605文件ratchet和封存19门全部通过，JS129/0/12 ignored、
字体11/0/1及绑定最终EXE的font-family产品1/0，包含NBSP反例的严格错误身份/原子
拒绝；原生EDIT/Select和资源/startup亦通过。18个native变更最大377有效行、无新
例外，最终源码/六件摘要及旧72份由closeout核验，首轮失败留存。该结果不提升
OS安全隔离、在线依赖扫描、默认能力或发布批准，不清除旧Canvas/启动未知故障。

v357无依赖/SDK/默认能力或预算扩张。仅测试wire传固定enum/整数/bool，轨迹每个
owner最多64项，父侧校验有界/单调后输出；不读JS异常属性或记录脚本、快照、节点、
路径与凭据。诊断管道错误不改变原失败/child回收。实际产品协议不增加字段，
成功产品旧回复仍可由测试父侧读取；回归与实际Application状态见迁移台账v357。
该诊断不构成新在线扫描、OS权限隔离、安全/法律或完整迁移批准，旧未知失败不清零。

首轮20门仅19门通过，JS124/1/11 ignored的直接Engine Canvas模型mount失败未定位。
只在测试helper的apply错误后接同Report，独立新源码/六程序的16门通过，三个完整
JS观察各125/0/11、diagnostic7/0和原预算门通过；实际产品无诊断前缀/敏感测试标记。
生产输入相对首轮不变，旧60封存件与最终身份见迁移台账v357/closeout；成功不结清
模型/v356 Application及其他未知根因，整轮仍partial，不提升默认能力或安全批准。

v356无依赖/SDK/默认能力或原stack/heap/时间/Job/几何/startup预算变化。原CSS公开
入口在私有parser之外完成预捕获String转换；sheet先验receiver，转换/完整解析/
预算全部成功才通知及提交，主动重入mutation仍保留。真实Engine复现并修复原
栈边界，拒绝回归不吞任意异常；实际冻结/封存成绩见迁移台账v356，不等任意深度
重入、新在线扫描、OS权限隔离、完整迁移/安全/法律或发行批准，旧未知失败不清零。

values合并私有validate单调用层、color直接用预捕获exec；不换用可污染的公开
RegExp方法，保原grid/spatial限额、普通值原长校验/规范化与原子替换门。

v356严格静态/构建与原几何预算拒绝门、真实Engine/child/产品重入回归通过；
但Canvas Application初始回复仍有未定位Execution(InvalidData)，三项锁中毒连带，
19封存门仅18门通过、整轮partial。新旧ABBA隔离四次成功不结清失败，不能由成功
子集提升完整运行/安全批准；身份、旧未知根因和SDK限制见迁移台账v356/closeout。

v355无依赖/SDK或默认能力变更。几何缓存只持私有mutation token和单份encoded
projection，token用启动时捕获的intrinsic创建且不对插件公开；输入转换/校验先于
通知及提交，编码前后核token。连接box getter仍逐次进入原生reads/layout/250ms/
viewport/粘性失败门，不缓存授权或最终矩形，原heap/stack/Job/父侧门不变。
fake-box模型、真实运行和旧未知失败分开记录于迁移台账v355；不是新在线扫描、
OS权限隔离、安全/法律/完整迁移或发行批准，SDK仅限已批准隔离构建。

最终严格静态/构建、原预算粘性拒绝、实际HWND/child/Application及五产品worker
通过；模型新增输入转换重入仍超原256KiB，geometry/JS集合共享此失败，18门仅16门
通过，不能以其他子集通过升级安全或完整运行验收。未删重入场景/放宽预算，
模型与实际生产边界的归因仍开放；旧失败和完整身份见迁移台账v355/closeout。

v354无SDK/依赖/默认能力或原heap/stack/时间/Job/帧/startup门扩张。timers使用
私有预捕获intrinsic、64登记/32参数/不复用ID及到期快照；只用原child单调时钟，
拒绝timer字符串handler，不新增文件/网络/线程旁路，也不宣称禁用JS语言内的eval。
同步回调异常/死循环沿原终态回收，背压与授权、
源码期限和viewport/epoch/FIFO门不旁路；精确边界见JS合同timers节，实际验证见
迁移台账v354，不是新在线扫描、OS权限隔离、完整迁移/安全/法律或发行批准。

第二轮仅按新能力修正counter旧无timer断言，网络/进程等禁用要求保留；与首轮
生产输入相同。严格静态/构建、timer/JS和五产品worker门通过，但geometry与startup
失败、18门仅16门通过，保持partial；不能把过期夹具已修升级为未知根因、安全或
完整运行验收。原失败、两轮封存和SDK边界不改，具体身份见迁移台账v354/closeout。

v353无SDK/依赖变更；显式诊断只记录固定阶段、ID、单调时间和状态，不包含路径、
脚本、像素、凭据或原错误详情，管道断开也不改变原操作结果。原资源生命周期和
child15秒不变；startup_smoke迁出不新增生产权限或线程。初步启动诊断仍失败，
不冒充产品修复、新在线扫描、OS隔离、安全/法律/完整迁移或发行批准。

最终断管道/状态保留/并发ID等计时5项及相关资源门通过；严格静态/构建和3,587
文件行数门通过，原startup测试与依赖字节未改。但startup6/1仍超15秒，整轮partial，
不能把诊断缺陷已修升级为超时根因、安全或完整运行验收；零项筛选不计通过。

v352无新依赖/SDK或默认能力。Canvas绘图只交child私有WIC/D2D/DirectWrite，无
文件/网络/图片URL/loader入口；数据批次、身份、坐标、路径、背板及PNG逐层有界。
父侧先核Scene中的Canvas身份与期望尺寸，再检查PNG元数据/总像素并申请原Pixels
预算，不能以伪造图片跳过资源身份或动作授权。原4MiB帧、250ms、heap/stack/父两秒/
Job门不增，native失败粘性终态；UI沿原原子帧替换、设备资源和生命周期回收。
这不是新在线扫描、C/FFI/OS权限隔离、完整浏览器/迁移或发行批准；精确范围和
实际结果见原生JS合同/迁移台账v352，旧未知根因及SDK许可边界不变。

v352第二轮严格静态/构建与Canvas/JS/摘要绑定实际产品worker门通过；16个封存
门中startup6/1超原15秒，整轮partial，不以子集通过升级安全或运行验收。首轮旧
canvas拒绝夹具改video并补正向像素回归，生产输入未改、原子性要求未删；原失败
日志和封存件保留，无新在线扫描或SDK/默认能力变更，精确身份见本轮closeout。

v351 无依赖/SDK 变更。ResizeObserver 使用私有 Element/receiver 身份，最多32个
lifetime observer 和64个登记对；冻结 entry/预捕获 intrinsic，不增加宿主文件/网络
能力。观察与 rAF 共用原 heap/stack/时间/Job/256reads/8layouts 门，异常及循环终态
回收，不以持续 polling 兜底。宿主 resize 只记录不可回绕代次，原授权/FIFO/策略/
单槽和视口门保持，另拒绝在途 A→B→A；无 owner/背压不贡献过去 deadline。
精确支持及结果见原生 JS 合同/迁移台账 v351；不代表新在线扫描、OS权限隔离、
完整迁移或发行批准，SDK许可仍仅限 linshi 内 Otter 隔离构建。

v350 无依赖/SDK 变更或新在线 OSV。rAF 队列最多64 callback，handle 不回绕/复用，
每帧快照且逐 callback microtask checkpoint，共享原250ms总期限；错误/死循环仍
终态回收 child，不提高32MiB heap/256KiB stack/父两秒/Job128MiB。私有时钟只读
Instant elapsed，无文件/网络/loader/线程旁路，原单槽和 Grant/binding/revision/
snapshot/viewport 校验不变。UI 排帧须本轮策略和唯一 confirmed owner，外借 owner/
emit/presentation 背压不抢槽或忙循环。已在途帧仍可收割，不承诺模态冻结在途 DOM。
精确边界见 [rAF](../development/SURFACE_JAVASCRIPT_NATIVE.md#v350-宿主驱动的-animation-frame)，
失败/最终验证见迁移台账 v350；默认 JS、C/FFI/OS 权限隔离/法律/发行门仍不放行。
原 v348 startup 根因未定位/未修；SDK 许可仍仅限 linshi 内 Otter 隔离构建。

v350严格静态/版本/依赖/28材料合同和源码rAF集合通过，但整轮/封存运行为partial：
UIA Toggle/Select、TLS/batch、child deadline/原生geometry失败均保留，根因未定位/
未修，SERIAL连带不计独立根因。实际摘要绑定产品worker和startup7/0成功不清除这些
失败或原v348问题；原预算不增，不以资源采样推断因果，不作为新在线扫描、C/FFI/
OS隔离/法律/发行批准。完整程序身份、矩阵及最终只读查询见迁移台账v350/closeout。

v349 无依赖/SDK 变更或新在线 OSV。空间 CSS 先完整验证再展开/提交，只接已有
原生 position/overflow consumer；直接父、paint-tail、百分比、双轴锚点/尺寸和
同轴裁剪均有显式拒绝，不保存后静默忽略。所有新私有操作复用捕获 intrinsic，
quad 仅读自有属性；尺寸 decimal 规范化避免 native 默认回退。原 heap/stack/时间/
Job 门不提高，没有宿主网络/文件/loader/线程或授权旁路。精确边界与本轮失败/
验证见迁移台账 v349 和 [空间 CSS](../development/SURFACE_JAVASCRIPT_NATIVE.md#v349-受限定位与容器裁剪)。
旧 v348 startup 超时未定位/未修，固定三轮成功不消除原失败。默认 JS、OS 权限/
C/FFI/法律/供应链/完整发行门不放行；SDK 授权仍仅限 linshi 内 Otter 隔离构建。

v349严格静态/依赖/28材料合同、相关回归和封存后实际产品spatial worker/Application/
预算/资源/startup7/0通过，源码及六程序/旧封存件身份一致、相关进程残留零。
原资源预算无豁免，早期夹具失败和原v348启动未知根因保留；本轮成功不替代新在线
扫描、C/FFI/OS隔离/法律或发行批准，完整证据与边界见迁移台账v349/closeout。

v348 无依赖/SDK 变更或新在线 OSV。同步 geometry callback 只在 child 私有闭包，
使用同一 Scene/NativeMeasure/DirectWrite，不提供文件/网络/loader/线程宿主能力。
viewport 校验 dpi 非零、每轴≤32768；每 operation 最多256原生读取/8不同布局，
缓存仅一份完整投影。任何原生失败/超限/返回时过期均粘性终止，即使 JS catch；
准入层缺 viewport/压平 inline 则明确抛错，不返回猜测尺寸。原 heap/stack/时间/
Job 门不增；native 同步调用不能由 JS interrupt 中途打断，父两秒门负责最终兜底。
Map entries/iterator next 和 rounding 提前捕获，rect 无原型且冻结，私有数据仍沿
原协议复制/编码。生命周期压平冗余 async 层而非提高栈预算。

读布局的 frame 绑定实际客户区/DPI；交付前后、emit 暂存/take/cancel 及最终 batch
移交前保留复核，过期关闭不重放 listener 或提交旧尺寸事件。原 Grant/FIFO/事件
身份 owner 不变。结果与失败保留见迁移台账 v348；这不是新在线扫描、C/FFI 审计、
法律/OS 权限隔离/供应链/正式发行批准，旧缺口不结清。SDK 授权仍仅限 linshi 内
Otter 隔离构建，不系统安装、改注册表、分发或默认启用新 DLL。

v348严格静态/依赖/28材料合同及相关回归通过，源码与六程序/旧封存件身份一致，
相关进程残留零；封存后的geometry/Application/实际产品worker门通过。但startup
4/3超原15秒，整体运行门为partial；后来单跑诊断通过不证明原失败已修复。没有
放宽期限或用新成功覆盖旧失败、OSV/权限/法律结论，精确证据见迁移台账v348。

v347 无依赖/SDK 变更或新在线 OSV。CSS Grid 只准入已有原生 consumer 的值；轨道
输入/展开输出/单 bound/轨道数、正 fr 的 f32 表示和端点均有独立上限。shorthand
先完整验证再写 longhand，不以 projection 优先级掩盖 cascade；恢复显示重新检查
上下文，未知/压平 inline 的几何声明仍拒绝。非递归扫描与压平调用保护原栈预算，
intrinsic 含 fround 预捕获，私有状态不走插件可变集合/原型 setter。无宿主网络/
文件、线程、动作或授权旁路；原生呈现失败保旧帧。开发/封存结果及失败保留见
迁移台账 v347；默认 JS/能力、旧 OSV、vendored C/FFI、法律、OS 权限隔离和发行
缺口不结清。SDK 授权仍仅限 linshi 内 Otter 隔离构建，不安装/分发或默认启用 DLL。

最终严格静态/依赖合同/28 材料合同、Grid/JS/授权/Application 相关回归及封存后
实际产品 Grid worker/原 JS/预算/资源/startup 门通过。2,391 源码输入、六程序目标/
副本和旧 .339–.346 产品摘要一致，相关进程残留零；早期栈/夹具失败保留，无预算
豁免。这不是新在线扫描、C/FFI 或 OS 权限沙箱/法律/发行批准，详见迁移台账 v347。

v346 不改变依赖/SDK 或旧扫描结论。Grid 专属属性严格检查类型/语法、轨道与放置
范围，非法输入不提交部分几何；通用长度仍遵守既有 Scene 默认回退。最终 cell、
搜索 row probe、跨轨道贡献、缓存 miss 和原八轮收敛均有独立上限；fr 冻结使用
active mask。没有新网络/文件宿主、线程、任意表达式或动作权威，失败替换沿原
Presentation 保持旧帧。完整预算和拒绝见 [Grid 合同](../development/SURFACE_GRID.md)。
默认 JS/能力不扩大，JS Grid 未接；本切片不是 OS 权限沙箱、新在线 OSV、vendored
C/FFI 审计、法律/供应链或发行批准。SDK 授权仍仅限 linshi 内 Otter 隔离构建。

最终严格静态/依赖合同、28 材料合同、Grid/相关回归和封存后 Grid/JS/Application/
预算/资源/startup 门通过；源码、本轮六程序及旧 .339–.345 封存产品摘要一致，
相关进程残留零。首次编译、夹具未知背景键与 Clippy 失败保留；没有放宽期限或
以新成功覆盖旧扫描/权限/法律结论。精确证据见迁移台账 v346 与本轮 closeout。

v345 没有依赖/SDK 变更或新在线 OSV。CSS 只开放有真实 consumer 的声明/值和有界
author matcher，不接受 @import/URL/@media/未知规则或保存后忽略。replace/采用/inline
先解析并检查 brand/预算再提交；正边宽/盒尺寸/行高组合显式约束 native 语义。
私有 CSS 操作捕获集合/字符串/RegExp exec/数值 intrinsic，避免插件覆盖 Symbol.split/
exec/Number 后改变准入或经原型 setter 暴露状态；snapshot/协议仍走原私有 JSON。
单 sheet 128、采用后 256 rules，最多 16 sheets；原节点/heap/stack/时间/Job 预算不增。
这不是浏览器 CSS、安全沙箱或供应链放行；默认 JS、旧全仓 OSV、vendored C/FFI、法律/
权限隔离与发行缺口仍开放。真实验证与失败保留见迁移台账 v345，SDK 授权范围不扩大。

最终 v2 的严格静态/依赖合同、28 材料合同及 JS 60/4 ignored 通过；封存后 CSS
12/1 ignored、实际产品 CSS/文档/旧 worker、Application/预算/资源/启动门通过。
源码与六程序/旧 .339–.344 产品摘要一致，相关进程残留零；RGB Unicode 空白
反例导致 v1 不推广，最终只采用 ASCII 色值准入后的 v2，原失败/诊断证据不覆盖。
这些结果不是新在线 OSV、vendored C/FFI 审计、法律或 OS 权限沙箱/发行批准。

v344 没有依赖/SDK 变更或新在线扫描。新增 fragment 只接受显式标签/属性/简单选择器，
不提供 script/style/URL/on*、网络/文件/loader；解析失败保原树，节点/深度/属性/
源码和原引擎预算有界。插件可变 WeakMap/Map/Array intrinsic 与 inherited setter/
toJSON 不得接触真实状态或私有协议；捕获操作、无原型私有记录和数据复制提供这条
JS 层保护，父侧仍复核动作权威。emit payload 自有 toJSON 语义不因此关闭。
QuickJS 栈耗尽失败保留，改显式遍历/分步转义而非提高预算。开发门与封存产品门须
分开取证，详见迁移台账 v344 和 [原生 JS](../development/SURFACE_JAVASCRIPT_NATIVE.md)。
默认 JS 仍关闭；Job 不是 OS 权限沙箱，不结清旧全仓 OSV、vendored C/FFI、法律/
供应链和完整发行缺口，SDK 授权仍仅限 linshi 内 Otter 隔离构建。
最终 v2 的严格静态/安全/28 材料合同、JS 48/3 ignored、同字节 Application 七项及
实际产品文档/旧 worker/预算/资源/启动门通过；源码、六程序与旧封存产品摘要一致。
v1 测试固定 identity 和早期栈耗尽/协议完整性失败原件保留；这些通过不等于新在线
OSV、C 引擎或完整权限隔离/发行批准，不能追溯结清旧告警。

v343 没有依赖/SDK 变更或新在线扫描。JS listener 的原始 emit 保留 ID/字节/顺序，
最多 32 项且整批 64 KiB，沿原租约、唯一 owner、FIFO/ACK/确认/接续，不绕过 HTTP
门；容量不足不重新执行 listener。事件 runtime 来源只由父侧权威校验标记，远端
响应不能升级普通事件或将 JS 静默降为 Declarative。服务端权威 scene 不直接渲染，
Pending 仅表示同 child update 在途；真实 DOM/授权/绑定再核通过才放行收据/输入/输出。
原回复事件保留最后唤醒，关闭只等线程完成；恢复记录沿既有 NotStarted，不改格式。
修正业务 snapshot.state 与 lifecycle state 混用的解析缺陷，不放宽后者的类型检查。
默认 JS、初始 emit/图片宿主和权限沙箱仍关闭/开放缺口；Job 的权限保证不扩大。
验证见迁移台账 v343；固定 TLS peer 不是实际 Loom，旧全仓 OSV 失败、C/FFI/法律/
供应链及完整发行门不结清，SDK 仍仅限 linshi 内 Otter 隔离构建。
最终 v2 的 JS 37/2 ignored、同字节 Application JS 七项与产品私有 worker/预算/
资源/startup 门通过，目标/副本及旧 .339–.342 产品摘要一致，源码身份前后一致。
v1 非最简布尔表达式的严格 Clippy 失败与协议/夹具原件保留，旧集合未封存/推广；
静态安全合同不冒充新的在线 OSV、C 引擎审计、法律/权限沙箱或完整发行批准。

v342 没有依赖/SDK 变更或新在线扫描。Application 的持久 JS 后台 owner 保留原 Grant
串行租约，初始/绑定同步回复逐次复核完整 Binding、revision、原始快照与最早期限；
单槽不得覆盖或跨凭据代际交付。后台等待撤权/过期/child 退出，无轮询；关闭只停止
自有执行，回收后原附件 owner 才可继续 inactive。未消费回复在关闭后不参与等待，
避免退出忙循环；呈现失败不得借空 rebind 伪造成功。JS 的默认准入、输入/更新桥和图片宿主
仍关闭，不扩大任意 URL、宿主 API 或 Job 的权限保证。既有五锁 OSV 失败和沙箱缺口
不因本轮接线而结清，完整边界见 [原生 JS](../development/SURFACE_JAVASCRIPT_NATIVE.md)。
最终 JS 31/2 ignored、Application JS 四项和相关授权/资源/等待/关闭回归通过，
六封存程序目标/副本及旧 .339–.341 摘要不变。v2 构建的源码身份不一致已被拒绝且
未推广，仅 v3 冻结/构建后/运行后相同集合进入收尾。固定测试会话的 TLS peer 不证明
真实 Loom 配对/服务端批准；真实产品私有 worker 门也不是完整 Application JS/FIFO
或权限沙箱验收。无新在线 OSV、法律/供应链批准或 SDK 使用范围变更。

v341 不变更依赖或历史扫描结论。原生 JS 入口仅来自已授权挂载快照，复用固定 HTTPS/
Device/nonce/lease/SHA-256 传输门后才由后台执行；不是任意 URL/脚本字符串的授权
回退。Ready 不能凭相同 origin/device 跨 Grant 移交，Mounted 保留原租约并在操作
前后复核；不能通过新资源租约复活过期状态或静默换程序。空闲撤权须由未来应用
等待句柄/refresh 驱动，Job 仍不是权限沙箱，默认 JS 关闭。payload 等价计数优化保留
所有原预算。完整安全/资源边界见 [原生 JS](../development/SURFACE_JAVASCRIPT_NATIVE.md)。
初始 emit 的不可变身份也不随生命周期 ACK 重绑；初始帧交付前不执行后续操作。

v340 的 Windows-only 原生 JavaScript 开发子集精确锁定 rquickjs/core/sys 0.14.0，
只开 std；真实 ES module/Promise/DOM 运行在持久受限 child，默认 attach/capability
仍关闭。父侧按权威 scene 复核事件，源码/帧/节点/CPU/heap/stack/Job/管道和销毁
均有明确 owner 与预算；Job 不是文件/网络/用户权限沙箱，C/FFI 漏洞不被这些预算
证明安全。详见 [原生 JavaScript 安全边界](../development/SURFACE_JAVASCRIPT_NATIVE.md)。

候选初查在线 OSV 对独立 probe 的 17 个精确 Rust 包没有返回记录；正式合入后首轮
真实五锁扫描为 1,830 包、24 个漏洞 ID，严格门失败。新增八个原生身份无返回匹配，
原 native rustls-webpki 0.103.15 / webpki-roots 1.0.9 分别返回 RUSTSEC-2026-0382 /
RUSTSEC-2026-0381。联网核对 RustSec 与上游修复后，仅将 native 两包定向升级至
0.103.16 / 1.0.11，保留 rustls 0.23.46 和其他依赖；公共根/附加 CA 的作用域不变。
修复后真实五锁复扫的 native lock 无返回匹配，但其他既有锁仍使整门失败（1,830 包、
24 个漏洞 ID）。两次原始 JSON/失败回执均保留，无新 suppressions，不放行发行。

另行联网核对 QuickJS-NG 官方十份公告（包含 OOM/UAF/RCE）：vendored 声明 0.16.2，
绑定官方 commit 0fdea21ff1090084e91dad812b343d92e79ba9d9，六份核心 C/头/LICENSE
逐字节相同。GHSA-rxpx-wj25-jhqq 的 affected 范围缺上界、patched=0.16.0；已核对
源码包含 patch 前错误检查，但未复现 PoC/做完整历史修复审计，不以版本或 Rust
查询代替 vendored C 风险审查。Windows MSVC 仍属上游 experimental。
sys 构建的 WASI 分支存在未见摘要校验的 SDK 下载；本 Windows target 不走该分支，
不能将其当跨目标安全保证。正式许可证审计通过、原始两份 MIT notice 已保留，
候选源码身份新增绑定 THIRD_PARTY_NOTICES.md。候选材料合同另发现 core/sys crate
缺少仓库级 MIT；javascript_licenses 仅为精确 0.14.0/官方 registry/MIT 身份补充
同图 rquickjs 的 LICENSE 和 sys/quickjs/LICENSE，两份均绑定固定上游联网原件摘要。
不通用回退、不联网获取代用文本，缺失/篡改/重复 peer/越界仍拒绝；原失败回执保留。
修复后 28 项候选材料/身份合同通过，实际 metadata 的 165 个全平台 normal/build
包可生成逐文件摘要材料；这不是精确链接集合、完整逐文件归属/分发/发行批准。

v334 增加实际产品的显式本地候选门，而不是默认安装器。代码固定 v332/v333 两 DLL
身份，不接受 CLI hash；逐级本地固定盘路径/owner/DACL、两文件精确集合、同句柄
摘要和租约独立于材料一致性门。固定 pin 的 child 重新准入，父进程不加载 ANGLE。
共享目录读锁不能阻止 FILE_WRITE_ATTRIBUTES reparse 控制，因此每级祖先 ACL 与
持续可信权限不可省；刚收紧的公共目录无法撤销历史属性/WRITE_DAC 句柄。固定卷根、
当前用户、SYSTEM、Administrators 是外部信任前提，不声称 Job 是权限沙箱。
安全假设/命令见 [本地 Shader 候选](../development/SHADER_LOCAL_CANDIDATE.md)。
私有 HOME 新夹具的实际 EXE 首次认证取码/正式提交/缓存第二 child 有通过运行，
但最终默认计时正向门在原 15 秒超时；一次计时诊断通过不替代失败，本轮 partial。
最终 Shader 65 passed / 15 ignored、严格 Clippy/check/fmt/版本/3,459 文件行数
和摘要绑定六套 EXE 集成 26 项通过，九文档 UTF-8/链接通过；未重跑完整桌面库或
结清旧失败。材料所有 false 批准、完整 source/
tool/notice/CRT 权利及可信发行安装门不变；无 SDK 系统安装、分发或默认 DLL 激活。

v333 adds explicit local ANGLE materials intake under scripts/native-candidate.
The caller supplies independently reviewed stage/build hashes; intake reuses the
existing stage/toolchain/log/PE gates without running build tools. Copies are
allowlisted: two DLLs, bound evidence/source snapshots and original notice bytes;
no SDK/MSVC binary or cached d3dcompiler is redistributed. Per-write and verify
budgets, safe Windows paths and exact file/directory sets reject extra files,
empty directories, failure markers and fabricated approvals. Failed outputs are
preserved, not overwritten. See [materials boundaries](../development/ANGLE_TOOLCHAIN.md).

Independent verification requires an externally reviewed materials manifest hash,
not the original stage/VS/SDK. It checks sealed consistency, never executes copied
source and does not re-prove build execution, ACL trust, provenance signatures or
legal clearance. It is not atomic protection against malicious concurrent writers.
The CycloneDX/SPDX documents describe material files only, not a full DLL dependency
closure; license/copyright conclusions remain NOASSERTION. Actual materials have
159 inventoried files/109 notice carriers and pass official online-fetched schemas
with format checks. A fresh public-library driver directly consumes this payload:
hardware/WARP 18 pixel comparisons, 12 successful children and two invalid-GLSL
rejections pass with zero pixel differences. This is not actual product acceptance.
Chromium root attribution, implicit CRT/final archive members, complete source/tool/
notice closure, trusted installation and release remain unapproved; all material
approval/runtime/install flags stay false. Native inputs, product EXE and default
capabilities are unchanged; no DLL rebuild, SDK distribution, publication or push.
Final focused gates pass: 105 Python contracts, nine native-candidate Node contracts,
Rust formatting, version alignment, the 3,451-file line ratchet and diff checks.
Nine new files are 30–120 effective lines without new exceptions. All 34 Python
sources parse; seven documents pass UTF-8/local-link checks. All 1,767 native inputs
and the old product EXE remain unchanged; no related process remains. The complete
desktop library was not rerun, and previous intermittent failures are not cleared.

v332 增加固定 DLL 显式构建 owner，不自动信任未知 stage：调用者提供已独立审阅的
stage SHA-256，暂存清单/profile/Python/自有源码都需匹配，已有缓存或未登记输入拒绝。
复用原隔离环境，不继承 ambient PATH/INCLUDE/LIB；父进程明确设置
DEPOT_TOOLS_WIN_TOOLCHAIN=0。构建前后重验输入、VS/SDK anchors、自有源码和 Rust API。
Win32 进程 suspended 创建并挂入 kill-on-close Job 后才恢复，精确继承三个标准句柄，
最多 32 活动进程，超时/异常/root 退出有界终止剩余子树并清理句柄。Job 不是权限、
网络或文件系统沙箱；Local mutex 仅协调同 session 遵循入口的调用，不认证目录 ACL。
stdout/stderr 各 16 MiB 轮询上限可能 overshoot，不是磁盘硬配额；全新输出、失败和
主动中止现场均保留，不能覆盖重试或凭残留 DLL 放行。

GN trace 限定五种审阅脚本，计划要求 `/MT`、`/WX`、lld-link，无动态 CRT/发现回退；
编译诊断和无工作门独立检查。纯字节 PE 关联名称、函数数、ordinal、非零 EAT、唯一
文件 backing RVA 和必需 API 的 executable 节，拒绝 forwarder；普通与 RVA-based
delay imports 都拒绝动态 MSVC CRT 和 api-ms-win-crt，VA-based delay imports 不接受。
这不证明机器码语义或抵抗并发恶意替换；其他合法数据导出不被误当作函数。
payload 只含两 DLL，不包含上游已复制到隔离构建缓存的 SDK d3dcompiler，也不分发
SDK、不安装系统 SDK、修改注册表或默认启用
capability。stage/build 回执的 source/toolchain closure、runtimeVerified/Activated
和 releaseApproved 继续 false；完整源码/工具/许可、可信安装、实际产品及迁移验收
仍是独立门，详见 [构建信任边界](../development/ANGLE_TOOLCHAIN.md)。

最终源码绑定的新树已真实完成 12,569 文件暂存、GN 513 targets/135 files、504 步
两 DLL 编译（558.765 秒），无工作/零编译诊断及加固 PE/静态 CRT 门通过。新显式
夹具的公共库外部驱动硬件/WARP 六 context/18 像素、12 成功 child/两无效 GLSL
拒绝通过，像素零差异、child 父进程不加载 ANGLE。81 合同、fmt/版本/3,442 文件
行数门通过；旧弱 PE 门构建主动中止并保留，不计成功、不改写旧 stage 迁就新源码。
原生 1,767 项输入/旧产品 EXE/默认能力不变，未运行新产品或完整桌面/Loom；79 个
gitlink 中 66 个仍未提供，完整默认源码/工具/许可、可信安装、长稳/最终手测和
旧 UIA/间歇启动失败不追溯放行。证据位于 linshi/otter-native-v332-regression。

v331 为固定 profile 新增构建侧本地暂存 owner，不是下载器、SDK 安装器、编译执行
或运行时激活。Git commit/tree/blob 身份和工具归档 SHA-256 均显式固定；不消费
worktree filters/hooks，不允许 lazy fetch、对象替换、外部 alternates、全局/系统
配置或网络 protocol。只读 Git 每个 reader 有 300 秒 timer，成功、失败及 timer
启动失败均回收进程。Windows 设备名/大小写/file-dir 别名、穿越、链接、加密 ZIP
和容量扩张均拒绝。逐文件、配置、Git、归档和自有源码摘要成功前再次复核，失败
保留 FAILED.txt。这些不是 ACL 信任、原子恶意竞争保护或完整 Git/Python/MSVC/SDK
实例闭包；hash pin 不是签名或许可批准。SDK 仍仅用于已获准的 linshi 内 Otter 隔离构建。

加固后的真实暂存树有 12,569 个构建文件，与首轮树实际字节一致；最终树真实 GN
产生相同的 504 条原始命令计划，完整 DLL 编译发生在首轮树而非最终树重编译。
两 x64 DLL 的必需导出/静态 CRT 和完整构建日志诊断门通过；显式新夹具的公共库
外部驱动硬件/WARP 像素/隔离 child 通过，不是实际产品 EXE/Loom/完整桌面验收。
54 合同、fmt/版本/3,434 文件行数门通过。79 个 gitlink 中 66 个仍未提供，其全部
不可达性审计、完整默认 source/tool/许可闭包、编译执行 owner、可信安装、长稳和
最终验收继续开放。不改 Rust/用户程序/默认 capability/安装状态，不分发 SDK 或
发布；详见 [暂存与信任边界](../development/ANGLE_TOOLCHAIN.md)。

v330 新增固定 Clang/lld + MSVC STL 的 x64 D3D11 Release 静态 CRT profile 准备器。
它复用已有 adapter，只生成全新外部 overlay；上游 anchors、配置、Python EXE、
自有源码与输出摘要前后绑定，不执行工具、不获取/暂存完整源码、不接受新许可。
精确 VS/SDK、空 WDK 与非 debug/component/无符号等 guards 限定 runtime staging
豁免，GN 字符串拒绝插值，不放宽 `/WX`。有效回执须完整可解析、库存匹配且无
FAILED.txt，失败现场不清理。摘要与 guards 不是完整工具内容/ACL/原子替换保护。
真实新实验树补齐固定 tools/win，复用获授权 SDK 和 v329 固定资源工具；实际 DLL
构建与仓库输出消费通过，Ninja 原生 GN 自动再生前后 toolchain 字节相同，无
prefix patch。三项项目头失效/恢复通过，但 Clang `/showIncludes:user` 排除系统头，
不能据此声明 SDK 全内容自动失效；完整源码/工具 staging、许可闭包仍未完成。
新 DLL x64/52 必需导出/无动态 MSVC CRT 门通过，编译日志无 warning/error 诊断；
显式夹具复用已绑定公共库外部驱动，硬件/WARP 像素与 child 门通过，父进程在 child
模式不加载 ANGLE。不是实际产品 EXE、Loom、可信安装或完整 UI 验收。没有系统 SDK
安装/注册表修改/SDK 分发/默认 DLL 激活；原生 Rust、用户程序与关闭 capability
不变。旧失败不追溯放行，完整迁移/资源长稳/最终手测与 release approval 继续开放。
见 [工具链边界](../development/ANGLE_TOOLCHAIN.md)。

v329 narrows the actual Windows GN argument seam: only the exact
otter-no-runtime-dirs marker becomes an empty runtime_dirs value; near matches,
appended paths and other runtime directories remain rejected. No path permission
or ambient environment fallback is added. Two new regressions fail before the fix;
all 25 adapter contracts pass, including an explicit non-inherited VSLANG request.
VSLANG is not proof that English compiler resources exist. User-approved SDK archives are extracted
inside linshi only, with per-file hashes; real x86/x64 compile/link/rc/mt probes pass.
There is no system SDK installation, registry modification, SDK distribution or
default runtime activation. Pinned Git blob copies, explicit checkout overrides,
build-commit metadata and guarded static Release D3D11/MSVC profile form a separate
experimental GN tree. Actual GN generates 111 targets, EGL/GLESv2 dry-run plans 402
commands, and four static-target builds pass, not a self-contained archive or a
warning-free final profile: zlib emits D9002 for ignored Clang flags. GN traces contain reviewed
setup/timestamp/CPU/memory scripts, not VS discovery or runtime DLL staging. The
Chinese-only MSVC initially records zero header dependencies despite VSLANG=1033.
A generated-Ninja prefix adaptation records 121 dependencies and passes real header
mtime invalidation/restoration; this post-GN seam is experimental, not a reusable
repository owner or protection against later GN regeneration.
The later v329 experiment acquires fixed official Clang/Chromium rc objects with
generation, response, size and hash records, safely expands regular files and runs
the complete resource wrapper including SDK rc comparison. These tool EXEs are
not Authenticode signed; upstream pins/HTTPS/hash checks are not signature proof.
Both actual x64 DLL builds pass, with all 52 required exports and no dynamic MSVC
CRT imports. Per-rule prefixes preserve C++/RC dependencies (121/24/24); real header
mtime invalidation/restoration passes. All 44 D9002 warnings remain. An external
driver linked to the freshly compiled unchanged public Otter library passes hardware/
WARP pixels and 12 success/two compile-failure children; its child-mode parent never
loads ANGLE. This is not actual product EXE or Loom validation. The initial direct
diagnostic's immediate-unload assertion fails and is retained: upstream lazy EGL
dispatch retains GLES module references until process exit, reinforcing the child
lifetime boundary. No product isolation policy is weakened. Complete tool/content/
historical rc-source/license closure, repository staging with GN regeneration,
trusted installation and release approval remain incomplete.
Runtime Rust, user programs, absent Installation and closed capability remain unchanged.
See [the toolchain boundary](../development/ANGLE_TOOLCHAIN.md).

v328 adds a build-only [ANGLE toolchain adapter](../development/ANGLE_TOOLCHAIN.md),
not an SDK installer, DLL or runtime capability. Explicit bounded JSON and pinned
upstream identities replace setup_toolchain discovery, reject root/architecture/
anchor mismatches and isolate include/lib/path. Removing the upstream empty runtime
PATH prefix prevents CWD cl.exe shadowing; generated imports are actually exercised.
22 synthetic-data/upstream-main/CLI contracts pass, not real SDK/GN/DLL validation.
Key anchors are not full tool/content closure, signed provenance, ACL validation or
atomic protection against concurrent malicious replacement. No SDK/compiler tool,
CIPD, vpython, hook, GN or ninja is executed; tests execute reviewed Python data
logic and the existing Python interpreter. No license/install/release grant follows.
The external output still needs a separately reviewed complete GN staging consumer;
runtime Rust and default absent Installation/closed capability remain unchanged.

v327 adds no dependency, installation surface, thread, queue or handle. Both Shader
configuration constructors reject real image ports named mode/output_mode before
encoding the forced mode parameters, including absent parameter declarations.
Ordinary Art still accepts these names. No image is dropped/renamed, no forced mode
or Loom collision check is weakened. Existing bounded port validation remains the
only generic admission owner; configuration failure precedes input-worker startup.
新回归先红后绿，隐藏应用门验证旧注册令牌/preview/正式输出保留，不改显式取消语义。
协议/配置 13、Shader 60 passed / 15 ignored，硬件/WARP 本地应用夹具 16 次真实
child（8 次认证取码/8 次缓存无新连接）、最终摘要绑定六套集成 26 项及严格
Clippy/fmt/3417 文件行数/diff 通过，修改 Rust 均不超过 500 有效行，源码/EXE 不变。
证据位于 linshi/otter-native-v327-regression。没有完整库重跑或实际 Loom 验收，
旧间歇超时/UIA 证据不追溯放行；默认安装/capability、正式供应链、完整迁移/长稳/
最终手测边界不变，未组包或发布。

v326 introduces no dependency, installation surface, thread or queue. A private,
default-off file-transaction timing owner uses the existing explicit smoke timing
switch and emits only static stages, anonymous IDs, byte counts, durations and
status, never paths, contents or error text. Diagnostic write failure cannot change
the transaction result. Full baseline comparison, cancellation, share restrictions,
sync_all, non-overwriting publication, recovery and cleanup remain intact.
一次诊断 231 项配对完整、文件操作合计 183.739 ms，不确证旧超时根因。文件事务/
图片保存/真实保存调用方 29/5/19 passed，0-test filter 不算覆盖；正式六套集成
26 项及最终摘要绑定无编译复验通过，原启动期限不变。Cargo 重编译造成原 EXE 身份
检查失败，证据保留且诊断/最终 EXE 分开绑定；Clippy/fmt/3416 文件行数/diff 通过，
源码前后相同。证据位于 linshi/otter-native-v326-regression。没有性能修复或完整
库复验，不追溯放行 v325/v324 失败；默认安装/capability、正式供应链、实际服务与
完整迁移/资源长稳/最终手测边界不变，未组包或发布。

v325 adds a UI-owned bounded Shader recipe cache, not a dependency or installation
surface. Only successful authenticated acquisition, local rendering/PNG preparation
and formal application commit admit the original payload. Keys isolate Unit,
Selection, Art, catalog instance and optional input/reference content; parameter
overrides never mutate response defaults or LUTs. Cache hits rebind current inputs
under the existing request/configuration/source/topology gates and execute locally,
without a remote Session, Delivery, execution, cancellation or lease-control call.
The 16-entry/64 MiB payload allowance follows Recipe Arc lifetimes even after
eviction; it is not a total heap/catalog/GPU/compiler bound. Optional chunked hashes
run in the existing input worker and confer no source authority. Missing reference
context cannot fall back to an independent recipe. Two local workers use the same
two task slots; the real child capacity remains one and the original deadline
includes capacity waiting. The default trusted installation and capability remain
closed; fixture authorization is not a production installation grant.
最终 Shader 定向 57 passed / 15 ignored，应用 16 次真实 child（8 次取码/8 次
缓存无新连接），双缓存四场景/六次成功 child、容量释放前 Cancelled 回收、输入
worker 15 项、严格 Clippy、fmt、3415 文件行数与 diff check 通过；修改 Rust
模块均不超过 500 有效行。源码前后相同，最终 EXE 另绑定六产品/后端、两本地平台
及两种私有入口拒绝。资源/EXE/会话启动 2/1/9 passed，但 startup 5 passed /
2 failed：隐藏窗口与透明钉图各触发原 15 秒超时，无 stderr，根因未确证；不放宽
测试。两项各一次计时隔离诊断仍超时，隐藏窗口 application 在 12766 ms 成功
结束并打印基础绘制/重建标记，透明钉图最后计时为 groups begin；未确认单一
阻塞点或与缓存/v324 前台失败的因果关系。首轮 Clippy 大枚举问题已通过可选
元数据 Box 修正，失败日志保留；证据和
partial 回执位于 linshi/otter-native-v325-regression，不作为完整启动/发行放行。
本轮完整桌面库未重跑，v324 的 UIA/外部前台失败仍保留，不操纵用户应用或借定向
成绩放行完整回归。正式供应链、实际 Loom、完整迁移/资源长稳/最终手测仍未完成；
配方缓存不是常驻 renderer 或编译 GPU 程序复用，未组包或发布。

v324 adds no dependency, binary, worker or installation
surface. Public resolved string parameters may exactly identify a Unit already in
the current workspace, including retained legacy fields. The application reuses
its sole Graph/Source/PreparedBatch path: Art requires a formal output image;
Sticker/Live retain existing source semantics. Unknown strings, paths and URLs do
not trigger I/O. Primary/reserved/secret/internal/disabled fields are excluded.
One resolved snapshot supplies image candidates, acquisition configuration and
local bindings. Real same-name connected images take priority; only actually used
Unit references and their upstream nodes become source freshness dependencies.
All three configuration encoding seams omit scalar parameters replaced by image
inputs, matching the current local Loom adapter's name-collision rejection; local
bindings retain the original resolved snapshot. Unit promotion excludes inputs,
params and forced mode/output_mode names. Framework params/inputs dispatch and
data-URL-to-path handling remain tool/Art-specific, not guaranteed by generic
adapter acceptance. Explicit declared mode/output_mode image-port collisions in
the broader configuration API remain outside this Unit-input increment.
Node/Output/Live identity, topology/parameter checks and the 32-input/31-auxiliary
limits remain authoritative.

v324 的最终源码 Shader 定向 50 passed / 14 ignored、应用平台 16 次真实 child、
严格 all-target Clippy、资源/EXE/会话启动/startup 2/1/9/7、fmt、3411 文件行数
与 diff check 通过；修改 Rust 模块均不超过 500 有效行。源码前后摘要一致，最终
EXE 另绑定摘要复验六项产品/后端、两项本地平台和两种私有入口拒绝；本次原生
进程/运行临时文件无残留，Node 编译缓存保留。完整桌面回归未通过：两次完整库
分别 3181 passed / 5 failed / 73 ignored 和 3185 passed / 1 failed / 73 ignored。
首轮 wire 旧期望已修正；四项 Surface 隔离通过但根因未确证，UIA select 隔离仍
失败并记录外部应用成为前台，未操纵该用户应用。两轮完整库间生产源码未变，
最终测试类型别名整理后未重跑完整库。证据位于 linshi/otter-native-v324-regression，
仅生成 partial 回执，不将定向/隔离成功视为完整桌面验收或放行依据。默认安装和
capability 仍关闭；实际服务、可信发行供应链和完整迁移仍待独立完成，未组包/发布。

v323 adds no dependency, binary, thread or queue. Local Shader texture bindings may
consume a prepared image only for the same named public image_link/file parameter.
Primary input, reference, secret/internal/disabled fields and undeclared parameters
do not acquire texture authority. Captured names never resolve paths, URLs or Unit
strings; missing frozen inputs fail instead of falling back to response locations.
Prepared, manual and response textures share the existing 31-item auxiliary ceiling,
decode/pixel limits, task slots, cancellation and source/topology freshness checks.
Inline PNG preparation now reuses ImageValue's same-byte PNG owner and the existing
Formal path, retaining original bytes rather than quantizing through PBGRA again.
Display pixels plus PNG count toward the existing input budget; non-PNG conversion
is unchanged. Original PNG ancillary metadata is retained too; this is explicit
inline input, not filesystem/network access or a general profile/orientation claim.
Final v323 source gates pass with 3181 library tests / 73 ignored, strict all-target
Clippy, resource/EXE/session-startup/startup 2/1/9/7, formatting and the 3408-file line
gate. Source identity is unchanged and the final EXE has separate backend/local-
platform/private-entry checks. The first full run's six desktop/UIA failures and raw
logs are retained; same-source isolated and full reruns pass, not a claim that UIA
intermittency was fixed. Fixed ANGLE/depot_tools/build/buildtools/jsoncpp source trees have
official HTTPS identity receipts, with the initial CRLF failure preserved and clean
LF acquisition verified. Static bootstrap review does not execute or approve CIPD,
vpython, hooks, SDK installation or DLLs. DEPOT_TOOLS_UPDATE=0 alone is not a download
or execution boundary; any later sync must review both nohooks and noprehooks and
bind actual tool instances. The parent's exact jsoncpp implementation revision is
also acquired and officially identity-checked, not the wrapper's dummy master.
Fixed SDK 10.0.26100.1 common/x64 NuGet archives have successful full Microsoft author
and NuGet.org repository signature verification and include 10.0.26100.0 headers/libs.
Archive/metadata audit is not license acceptance, installation, execution of package
tools or proof of an isolated build environment. The standard vcvarsall branch still
rediscovers the system SDK via registry; setting only a GN SDK path is insufficient.
Dependency/tool-instance closure, SDK environment adaptation, build/replay and release
approval remain incomplete. Default Installation stays absent, Shader capability remains closed,
and neither fixture/source acquisition nor these changes grant release clearance.

v322 adds no dependency, binary, queue or execution owner. A trusted Shader path
may fall back to its own editor-owned raster base only when no prepared primary
input is available. Ordinary Art is unchanged; missing auxiliary images still
block automatic dispatch. The base-capture entry derives pixels from editor.layers
and preserves the real document stamp, never inferring source authority from a
display-only restored Canvas.image or result preview. Overlay/crop content is not
baked into the Shader input a second time. Existing preparation/encoding, input
budgets, cancellation and source-freshness checks remain authoritative. Expanded
real hardware/WARP fixture tests do not authorize the external ANGLE DLLs, actual
Loom-service acceptance or public capability. Opaque own-base cases do not prove
lossless recovery of original source PNG straight RGB/hidden transparent colors,
profiles or orientation from already-premultiplied raster data. Source gates pass
with 3177 library tests / 73 ignored, strict all-target Clippy, existing startup/
resource gates, formatting and the 3407-file line gate. Source identity is unchanged;
the final EXE digest is separately rebound to backend/local-platform and private
entry checks under linshi/otter-native-v322-regression. Default installation remains
absent. Pinned upstream build documents have been fetched over HTTPS with hashes;
the required SDK 10.0.26100.0 is not the locally found 10.0.22621.0. No new DLL,
upstream build execution, machine toolchain modification or release approval follows.

v321 adds no dependency, distributed binary or installation switch. One resolved
parameter snapshot supplies both authenticated acquisition configuration and local
Shader bindings before asynchronous input preparation. Numeric/bool bindings and
inline static PNG overrides exclude secrets, internal fields and reference; the
prepared primary input cannot be replaced by a parameter texture. Existing typed
binding limits, configuration/source credentials, task slots, remote leases and
cancellation owners remain authoritative. Captured String/JSON allocations are not
covered by Pixels leases or the child Job. Application defaults to no Installation;
this trusted-caller descriptor is not a provenance, ACL, license or installer check.
Real hardware/WARP menu/parameter tests use authenticated fixtures and the test EXE
child, not the actual Loom service or product user entry. Public capability remains
closed pending trusted ANGLE supply chain/installation and remaining parity gates.
Full v321 source gates pass with 3174 library tests / 73 ignored, strict all-target
Clippy, existing startup/resource gates, formatting and the 3406-file line gate.
Source identity is unchanged across verification; the final product EXE digest is
separately rebound to backend/local-platform and private-entry rejection checks.
Evidence under linshi/otter-native-v321-regression does not grant release clearance,
general image/Unit-parameter support, cached real-time rendering, long-run resource
acceptance or final user acceptance.

v320 adds no dependency or distributed binary. Shader acquisition reuses the
authenticated remote Session, cancellation, leases, cleanup retries and bounded
task slots. Only a released, consumed real Delivery may receive a complete local
derived image; local failure after a successful remote terminal does not issue a
new remote cancel. Inputs are decoded from the same prepared PNG bytes sent to
Loom. Supporting textures accept inline static PNG only, never paths or HTTP;
trusted bundle selection remains outside Art data. RGBA decoding checks natural
dimensions and remaining texture budget before pixel allocation and never wraps
straight RGBA as a display Image. Wrapper JSON uses count-limited typed fields
instead of a second full Value tree; direct Values are checked before conversion.
The upstream JSON/string allocations and WIC internal memory are not protected by
Pixels leases or the Shader child Job. Real TLS fixtures verify exporter proofs,
but are not actual Loom-service acceptance. Browser matrix evidence does not
approve general color-profile/orientation semantics. Public capability, user
parameter/texture dispatch and trusted ANGLE installation/supply chain remain
closed. Final v320 source gates pass with 3169 library tests / 72 ignored,
strict all-target Clippy and the existing startup/resource gates. The final
product EXE digest is separately rebound to the six backend, two local-platform
and private-entry rejection gates. This is development evidence, not release
clearance or actual Loom-service/user acceptance.

v319 adds no dependency or distributed binary. An owned local Shader request shares
budgeted texture pixels and reuses wire admission before starting an on-demand
parent worker. The existing bounded child remains the only ANGLE loader; its exit
precedes Image/PNG preparation. Local and remote outcomes share the existing two
task slots, notification and application commit loop, but only remote outcomes own
remote leases/cleanup. Local completion independently binds unit/request and is
single-use; existing source/output, configuration and topology retirement guards
remain in force. Captured textures/inputs/permits are released before completion
notification, including early cancellation. Input freshness does not prove an
arbitrary texture's provenance: the future staging owner must decode the same
prepared inputs. Real local task/Application tests use the actual test child;
product-child platform gates remain separate. No public Shader capability, trusted
installation owner, source/texture fetch path or release clearance is granted.
WIC calls and driver cleanup remain outside hard-real-time guarantees; Job remains
resource/fault isolation, not a privilege sandbox or total GPU/process-tree budget.

v318 adds no dependency, binary, worker or public capability. Formal images retain
an optional original PNG alongside immutable display pixels, with both counted in
the conservative per-port budget. Only trusted same-encoding or same-byte decoding
call sites may construct a pair; shape/signature checks do not prove arbitrary
pixel/PNG equivalence. Existing input/save workers reuse these bytes, and restore
accepts bounded inline data only, without paths, network or profile access. Preview
does not acquire formal-output authority. Request/OutputStamp rejection and atomic
failure remain in the existing result owner. This data path neither enables local
Shader scheduling nor approves the external ANGLE test bundle for distribution.

v317 adds no dependency or distribution binary. The Canvas-alpha bridge validates
Shader geometry and complete pixels before reserving separate display/straight
buffers, reuses the WIC export owner, and rejects allocation/encoding failures and
late cancellation without partial results. Encoded PNG straight bytes are checked
against an independently decoded browser matrix, not a Canvas2D re-quantization.
The bridge neither publishes graph results nor enables the Shader capability.
Existing WIC internal-cache/stream and non-interruptible-call boundaries remain;
logical pixel leases are not a total-memory or hard-real-time guarantee. External
ANGLE fixtures are still not an approved release supply chain or installed owner.

v316 adds a bounded single-request Shader child, not an application capability or
release dependency. The shared process owner preserves image child limits and
empty environments; Shader uses a separate 256 MiB single-child pool, an absolute
normal deadline, explicit inherited handles and suspended-to-Job-to-resume startup.
Only an OS-API-derived SystemRoot is added to the Shader environment: independent
no-Job diagnostics reproduced hardware initialization failure without it. Parent
environment values, PATH and user directories are not inherited. Job termination
and synchronous cleanup are resource/fault isolation, not an AppContainer or
restricted-token sandbox, GPU-total limit or hard-real-time driver cleanup promise.
The Shader wire rejects short-body large-texture allocations before reserving
pixels; raw RGBA success and bounded typed failures remain distinct. Real product
EXE/4K tests do not approve the external DLL fixture for distribution. Trusted
bundle provenance, licenses, SBOM/advisory evidence and application result/alpha
integration are still required; no new Cargo/npm dependency or binary is shipped.

v315 adds a native offscreen ANGLE ABI adapter for development, but no ANGLE
binary or new Cargo/npm dependency is shipped or inventoried as release-approved.
The explicit test-only two-DLL fixture is outside the repository and bound to
length/SHA-256 identities. The loader holds read-only sharing handles, restricts
dependency search to System32 and checks actual module paths; the caller must
still provide a trusted-ACL directory and a trusted manifest, never Art-supplied
DLLs or fingerprints. Logical pixel leases do not bound compiler/driver memory.
A reviewed, fixed source/build identity, complete licenses/SBOM/advisory evidence
and an isolated child with Job/deadline termination are required before application
integration or packaging. This backend test is not supply-chain or release clearance.

v225 refreshes the replay-verified resvg identity after checked lattice/stitch
arithmetic and explicit per-render filter failure propagation. The worker rejects
failed results; filter admission remains closed. The complete 43-file source
snapshot preserves upstream package metadata and licenses. This is not a new
Advisory scan, total workload proof, or release-security clearance.

v224 refreshes the resvg source identity and both SBOMs after a replay-verified
seed-initialization overflow fix. Only src/filter/turbulence.rs changes relative
to v221's source snapshot; the original package metadata, licenses, lock versions
and features remain unchanged. The source gate and actual Cargo resolution are
rechecked. This is not a new Advisory scan or release-security clearance; filter
admission remains closed and other coordinate/workload risks remain unresolved.

v221 imports the replay-verified resvg source-only snapshot and the usvg numeric
guards into repository-owned paths. The native lock changes only resvg's source,
not package versions or features. Four source identities are checked against actual
Cargo resolution; all local-added modules retain independent line-size checks.
The regenerated CycloneDX/SPDX inventory contains 1257 identities; the license
audit covers 749 Rust packages and 40 expressions. A fresh online Advisory scan
covers five locks / 1822 package records and still reports 25 vulnerability IDs.
No vulnerability exceptions were added. This is development integration, not an
Enforce-mode clearance or formal release. SVG filter admission remains closed.

v219 subsequently imports the replay-verified usvg and roxmltree snapshots into
repository-owned paths. Native Cargo resolution and schema-2 inventories bind
their exact source identities; the native lock only removes the registry source
and checksum for roxmltree 0.21.1. A dedicated contract checks every local-added
source module with the repository effective-line lexer, independent of upstream
directory exclusions. Both SBOMs verify 1257 identities; license audit covers
749 Rust packages and 40 expressions. The live Advisory scan covers five locks,
1822 package records and still reports 25 vulnerability IDs. No exceptions were
added and this is not a release-security clearance. Evidence is under linshi with
the `otter-v219-source-import-` prefix; SVG attribute admission remains unchanged.

v219 extends the repository source-manifest verifier to explicit schema 2.
Every file declares `origin: upstream` with an upstream hash, or
`origin: local-added` with `upstreamSha256: null`. Added files must exactly match
the separate `added` inventory; `modified` continues to contain only changed
upstream files. Legacy manifests remain supported without inferring additions.
Unknown schemas/origins, forged hashes, omitted/duplicate inventory entries and
actual file tampering fail closed. The package index remains schema 1.
This verifies declared bytes and inventories, not the authenticity of an arbitrary
manifest or approval of a dependency: archive/patch replay, review, Cargo resolution
and release gates remain required. Production dependency identities and SVG
admission are unchanged; candidate usvg/roxmltree snapshots were checked separately.

v207 开发中启用 resvg text，新增 12 个锁定包；仍关闭系统字体自动扫描和图片/压缩
SVG features。skrifa 0.44.0 同时作为显式 metrics 验证依赖，仍采用 std-only。
首次五锁 Advisory 扫描为 1822 包、23 个漏洞 ID；749 个 Rust 包/40 个许可证表达式，
双 SBOM 为 1257 个唯一身份。相对 v202 新出现的 GHSA-9jrq-w75r-8gcw、
GHSA-rq7h-c2jc-7f22、GHSA-vfpm-58rq-9qcg 均属于已有 npm vite 8.3.0，
不是新字体包；没有新增豁免，尚待适用性分析。显式 skrifa 接线后已刷新最终
锁文件扫描/许可证/双 SBOM，计数相同，1257 身份校验通过；证据位于 linshi 的
otter-v207-final-*。这不是正式发布门通过，也不是零漏洞声明。

v202 为受限 SVG child 固定 resvg 0.48.1（关闭默认 features）与 svgtypes 0.16.1，
不启用系统字体、text、raster-images 或 svgz。严格 XML 准入后调用 from_xmltree，
两个 image resolver 均拒绝资源；未实现的 SVG 内容明确失败而非静默消失。
native lock 新增 20 个包。五锁 Advisory 扫描覆盖 1810 包，保留 20 个漏洞 ID；
许可证检查覆盖 737 个 Rust 包、40 个表达式，双 SBOM 为 1245 个唯一身份。
这些是 dirty 开发验证，不是零漏洞或正式发布声明。

v199 adds exactly pinned `moxcms 0.8.1` (BSD-3-Clause OR Apache-2.0) only to the
bounded WebP child color path, using scalar general transforms with `lut` enabled.
Native lock additions also include `num-traits 0.2.19`, `pxfm 0.1.30`, and build
dependency `autocfg 1.5.1`. Profile/CLUT/TRC quotas are not a total-memory limit;
the existing Job remains authoritative. Recoverable allocation and distinguishable
quota/unsupported errors must not become successful unconverted pixels; the library
conflates some CLUT quota failures with InvalidProfile, a documented remaining boundary.
The full Advisory scan
covered 1790 packages in five locks and retained 20 vulnerability IDs; license
checks covered 718 Rust packages, and both SBOMs verified 1229 unique identities.
These are dirty development receipts, not formal release authorization.

v196 native static WebP pins `image-webp = 0.2.4` (MIT OR Apache-2.0), adding
`byteorder-lite 0.1.0` and `quick-error 2.0.1` to the native lockfile. The decoder's
memory-limit API does not bound static working allocations: a dedicated child
Job enforces 128 MiB committed memory, with two concurrent children and bounded,
cancellable capacity waiting. This is resource isolation, not an AppContainer
or restricted-token sandbox. See `../development/SURFACE_WEBP_ISOLATION.md`.
The 2026-10-21 full Advisory scan covered five locks / 1786 packages and retained
20 vulnerability IDs; none was returned for these three added package versions.
The license audit passed (717 Rust packages / 40 expressions). These dirty-tree
results are development evidence, not a clean exact-ref formal release gate.

Native device HTTP response budgets remain explicit per request. The transport
ceiling is 8 MiB for the device-filtered Surface stream; existing session and
Live callers retain their smaller limits, and Surface action acknowledgements
use 64 KiB. Request bodies remain capped at 1 MiB. Stream decoding additionally
limits batches to 512 messages and each message to 1 MiB; this does not authorize
applying unverified event, attachment or generation data.

Native HTTPS device sessions use exactly pinned `ureq 3.4.2` with default
features disabled. Its unversioned transport adapter consumes the existing,
verified rustls stream once; it does not add an async runtime, environment proxy,
redirect handling or pooled connections. Updating this dependency requires the
actual TLS/HTTP fixtures as well as lockfile, license and vulnerability checks.

- Dependabot checks Cargo, npm, and GitHub Actions weekly with bounded routine
  update groups. Security updates are not delayed by those grouping limits.
- OSV-Scanner 2.5.1 scans the exact committed lockfiles on pull requests,
  `main`, a weekly schedule, manual dispatch, and the exact release tag/ref.
- CodeQL runs extended Rust, JavaScript/TypeScript, and Actions queries.
- Formal release builds generate and verify CycloneDX 1.6 and SPDX 2.3 SBOMs,
  SHA-256 checksums, a manifest, and build provenance. The public Release keeps
  only the portable ZIP and its checksum sidecar; the remaining evidence stays in
  the maintainer/workflow verification boundary.

## Refreshing the release scan configuration safely

The code-scanning tool-status page tracks the default branch. A failed manual
release run on `main` can leave that workflow's configuration reporting failure
even when its OSV job succeeded; later successful tag runs do not refresh the
default-branch record. Check the failing job before changing scanner settings.

`Release Otter Tag` now runs pure advisory scans on `main` pushes and relevant
pull requests through the `dependency-security` caller job in `release-otter-tag.yml`.
The reusable `dependency-security.yml` retains the `osv-scan` leaf job ID; caller
and leaf identities must not be conflated when investigating code-scanning status.
Manual **scan-only=true** still scans the exact workflow commit. PR/main and
maintenance events skip the entire publication job, including its cleanup and
signing steps. Explicit typed false plus a valid three-part tag is required for
manual publication; valid tag pushes retain strict release enforcement.

Do not rerun an old release, move a tag, or delete scan history to clear a banner.
Verify scan completion and SARIF processing against the exact new main commit.

## Development advisory policy

During private development, valid vulnerability and static-quality findings are
reported without blocking ordinary product compilation or functional tests.
OSV runs the same pinned 2.5.1 binary from an immutable container digest, directly
instead of the upstream shell wrapper. Its full inventory JSON must cover all
five lockfiles, and its SARIF must cover each vulnerability alias group at every affected package,
version and lockfile location before a findings exit of 1 can become
advisory success. Unknown exits, missing/corrupt/incomplete evidence, startup,
network, reporter, and upload failures remain failed jobs. No blanket
`continue-on-error` is used.

Development uses `security/osv-advisory.toml` without release suppressions.
Full JSON/SARIF artifacts, step summaries, and existing deduplicated Security
alerts retain findings. No duplicate issues or additional credentials are needed.
The pinned reporter aggregates aliases, but emits a result per package/source.
Validation checks alias-group membership, package/version messages, physical
locations and the reporter's SHA-256 fingerprints; a retained ID alone is not
coverage. JSON evidence is never rewritten or reduced.

Development enables `--all-vulns`. Strict scanning preserves the scanner's normal
exit semantics without that flag: JSON can legitimately retain only uncalled or
unimportant findings with exit 0. Those records still require full SARIF coverage;
exit 0 with actionable findings (or any findings under `--all-vulns`) is rejected
as inconsistent evidence. Release scans keep the reviewed, time-bounded exceptions in `osv-scanner.toml`
and fail on unsuppressed findings after retaining reports.

`Code Quality Advisory` independently runs ESLint, stable Rust format checking,
license inventory, and the effective-line ratchet. It retains complete tool and
JSON evidence. ESLint fatal parsing/configuration errors, missing dependency
manifests, incomplete Cargo metadata, unknown line-checker diagnostics, and
unrecognized Rust format output remain errors. Format findings require a valid
in-repository diff with no stderr. Actual typechecking, compilation, checker
unit tests, frontend/native tests, and browser/performance tests stay blocking
in `Build Otter EXE`. Release and `npm run verify:local` keep their strict checks;
`verify:local` also creates release packages and is not a scan-only command.

Native repository protection settings are not changed by these workflows.
Any separate required-check policy must be reviewed explicitly.

## Machine-authoritative inventory

`security/dependency-security-policy.json` owns the inventory:

1. `package-lock.json` for the Solid/Tauri frontend and build toolchain.
2. `src-tauri/Cargo.lock` for the desktop application.
3. `src-tauri/crates/drag/Cargo.lock` for the independent drag crate.
4. `src-tauri/crates/scap-direct3d/Cargo.lock` for the capture backend crate.
5. `native/Cargo.lock` for the independent Windows-native migration host.

The license audit resolves both application manifests. The native migration
format, tests and release compilation run in the existing build candidate job;
the promoted public EXE remains the Tauri application until migration acceptance.

The native SVG renderer uses repository-local, pinned usvg 0.48.1 and tiny-skia
0.12.0 patches. `third_party/sources.json` and per-file source manifests preserve
upstream archive and patch hashes. `scripts/release/third-party-sources.cjs`
verifies exact file inventories and bytes before license auditing and SBOM
inventory generation. Patched Cargo PURLs carry a SHA-256 source-tree checksum
qualifier, so they cannot collapse into an official same-version component.
Original licenses remain in each crate copy. See `third_party/README.md` for
replay and update requirements; integrity is not a vulnerability clearance.

The native HDR PNG writer pins `png` 0.18.1, also used by the main application.
It uses streaming encoding for validated internal RGB16 frames; it does not expose
the decoder to runtime input. The decoder is used by round-trip tests only.
Its transitive packages remain covered by `native/Cargo.lock` and the same inventory;
this development addition does not authorize a public release or reuse old SBOMs.

Update the policy, workflow, Dependabot configuration, contract, and this
document together whenever a dependency-resolution boundary changes.

The native media TLS configuration pins `webpki-roots` 1.0.9, matching the main
application lockfile, for compiled-in Mozilla public roots. It does not load the
Windows system certificate store or enable a certificate-verification bypass.
Operator CA roots remain scoped to their configured HTTPS origin. This addition
is covered by the existing native lockfile inventory; it adds no new resolution
boundary. Root updates require a rebuilt candidate and refreshed security evidence.

The native Device identity implementation declares existing locked `ring` 0.17.14
and `zeroize` 1.9.1 as direct dependencies for Ed25519 and owned-secret cleanup.
It uses Windows user-scoped DPAPI for the main-compatible protected envelope;
no new package versions or independent resolution boundary are introduced.
Owned-buffer cleanup does not claim to erase caller-owned JSON or cryptographic
library internal copies. Device HTTP issuance remains a separate integration gate.

## Local gate

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\tests\Test-DependencySecurityContract.ps1
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\Invoke-DependencySecurityScan.ps1
```

The installer downloads only the pinned Windows scanner, verifies its SHA-256,
and caches it under ignored `.tmp`. The scan also verifies the reported scanner
version and writes validated JSON evidence under `.tmp/dependency-security`.

For a local development report, pass `-Mode Advisory`. The default is `Enforce`,
so existing release/local callers retain their strict behavior.

## Triage and remediation

Record the locked package/version, dependency path, shipped target,
reachability, attacker-controlled inputs, exploit maturity, fixed version, and
affected release IDs. Treat active compromise or a suspected malicious package
as P0, reachable critical/high impact as P1, applicable high as P2, medium/low
as P3, and unmaintained/non-applicable transitive packages as P4.

Prefer a compatible upgrade, then removal, replacement, or feature containment.
Update lockfiles through package managers rather than editing resolved metadata.
Run focused tests, compile/typecheck gates, this contract, and the real OSV scan.
Any shipped dependency change requires a new release ID and regenerated SBOM,
provenance, manifest, and checksums; never rewrite prior release evidence.

## Temporary exceptions

`security/osv-scanner.toml` may contain only one `[[IgnoredVulns]]` entry per
canonical advisory. Each entry needs a concrete reason and an `ignoreUntil`
date no more than `maximumExceptionDays` in the future. The reviewing change is
the human approval record. Broad `[[PackageOverrides]]`, automatic renewal, and
invented approval identities are forbidden.

## Suspected malicious dependency incident

Stop installation, builds, releases, and promotion. Preserve lockfiles,
archives, hashes, SBOMs, provenance, logs, and runner details. Isolate affected
hosts, rotate reachable credentials, remove or pin away from the package using
a trusted source, and rebuild on a reviewed clean host. Mark affected releases;
do not silently replace them.

## Current baseline and limitations

The baseline was refreshed on 2026-08-24. Directly fixable npm and Cargo
findings were removed from the lockfiles. Remaining exception entries are
time-bounded unmaintained transitive packages or GTK/glib code that is not
compiled into Otter's formal Windows-only release. Those exceptions explicitly
block any future Linux publication until the Linux dependency stack is upgraded
or independently reassessed. The gate does not replace source review, runtime
reachability analysis, secret scanning, tests, or clean-source verification.
