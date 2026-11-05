# Otter Technical Architecture

## Live 静止源与尺寸换代（v544）

窗口恢复／resize 的尺寸变化帧不能直接交付，但丢弃后仅 Recreate 原帧池可能让静止源永久没有下一帧。`capture_wgc::live::pool` 在原 Session 内共同换代两帧池和捕获会话，保留 item／设备／通知／Consumer／帧序号和取消身份；旧＋新预算事务及整体错误关闭不变，不增轮询或重试。真实静止源三尺寸回归和原显隐失败门已通过；A/B 人工验收通过，普通 C 完整前台流程仍未关闭。详见[模块所有权](../development/CODE_ORGANIZATION.md#live-帧池与捕获会话尺寸换代所有权v544)及[证据和剩余边界](../development/WINDOWS_NATIVE_BUILD.md#v544-静止源尺寸变化后的-live-停帧修复)。

## 参数绘制与消息导航隔离（v541）

参数导航允许系统`IsDialogMessageW`处理消息，但滚动显露只有位置变化才触发布局/父失效，防止聚焦时父paint自续并饿死child初始绘制。选项仍是标准COMBOBOX；列表项由参数像素叶绘制，收起区域复用既有Surface标准控件像素层，字符串/索引/键盘/UIA和原参数事务不迁移。DIB及真实Application消息泵保护文字、初始绘制和资源寿命，不冒充普通可见验收。详见[所有权](../development/CODE_ORGANIZATION.md#参数初始绘制所有权v541)和[当前验证边界](../development/WINDOWS_NATIVE_BUILD.md#v541-参数初始绘制与下拉框修复)。

## Art 候选结果与正式输出隔离（v540）

候选预览在原执行worker准备为有界不可变Arc，只接受内联常见图片并缩至64边；正式端口保持原事务。Results在完成commit后绑定当前配置，配置/集合身份失效即撤旧选择资格。完整参数窗用标准BUTTON显示候选，沿同一Snapshot提交result_index后显式执行，不把预览像素当正式结果，也不让新草稿/IME或数值clamp穿过提交边界。单字段capsule不复制列表，无新增网络/线程/资源池。模块归属见[所有权](../development/CODE_ORGANIZATION.md#art-候选预览与选择所有权v540)，实际验证与普通可见/长稳边界见[构建记录](../development/WINDOWS_NATIVE_BUILD.md#v540-art-多候选预览与选择)。


## 当前Local Art签名包（v538）

当前主链只对接最新v3，不协商/降级；签名包持真实端口与按方向默认显隐，用户覆盖沿原Workspace。权限、资源、离线及清理所有权不变。详见[所有权](../development/CODE_ORGANIZATION.md#当前签名端口显隐所有权v538)和[验证边界](../development/WINDOWS_NATIVE_BUILD.md#v538-当前签名包与端口默认显隐)。

## 普通贴图输入与画布隔离（v537）

普通贴图手动输入归desktop/sticker_input，原图/编辑器仍归Canvas；图解析只借用共享手动资源，不用上传覆盖画布显示。配置Stamp统一覆盖执行、文件回包、UI目录和保存资格；保存及恢复跨worker传递共享不可变状态，不传HWND/可变UI状态。初始窗口到Application的交接归startup_entry，保持原安装及失败清理顺序。详见[所有权](../development/CODE_ORGANIZATION.md#普通贴图输入所有权v537)与[构建记录](../development/WINDOWS_NATIVE_BUILD.md#v537-普通贴图图片输入)。

## 端口单字段控件所有权（v536）

公共Art参数输入展开后可挂真正单字段原生child，复用EDIT、COMBOBOX、bool、步进、slider、颜色和文件控件及原参数事务；不是弹完整面板冒充内嵌。工作区仅一个活动editor，宽300逻辑像素，放图片左侧、空间不足翻右侧，两侧不足不挂，不覆盖图像。端口`Enter`/`Space`仍切换展开，`F2`从已聚焦参数端口进入控件，child内`F6`返回端口；文本Ctrl+Enter仍提交文本，不能误连线。linked有效值只读、secret不呈现；只允许已确认自身同步提交重建新Catalog后继显示，不继承旧草稿。

`chrome::capsule`拥有工作区单槽Rc候选、目录/宿主/parent身份及后继显示；`parameter_panel::window`拥有完整owned窗和单字段child的共同构造与资源寿命，`controls::capsule`仅筛选字段动作，复用原Snapshot/Spec和唯一参数提交。dirty/IME/pending纳入原chrome idle及保存/恢复/结束会话门。外部配置、隐藏、锁、捕获、端口显隐及失焦退休旧资格；同步focus/place/close/request返回只可取消同一Rc，旧栈不能撤销或覆盖后继。模态返回再次核panel owner及配置，旧panel不继续消费后缀；child继承端口条affinity，不对child调用顶层DisplayAffinity。没有新依赖、网络合同、权限、线程、timer、每端口HWND或像素/队列预算。

本批选定**98项Rust passed**，14项新增，选定终态0 failed/0 ignored；1项既有默认ignored未选、不计通过，非整库。覆盖真实child/单字段、多种公共widget、原子提交/linked只读、IME边沿/DPI草稿/父先销毁、region及图片外几何、单槽/输出/secret、dirty空闲门、自身提交后继、F2/F6/两段Esc、外部状态拒旧草稿、同步focus/place/close/request安装后继和退休owner拒发布；原chrome/参数/系统会话/Live输入取消依赖同批验证。官方fmt/版本/全仓strict行数、strict all-targets Clippy、固定SHA库、普通Release与workspace目标通过。23项源码/版本delta均UTF-8无BOM，6个新文件，Rust最大397有效行，无新增例外。input1首次Clippy因子模块可见性、测试未用导入和Stamp断言编译失败，原回执保留；input2只修这些编译问题，不重分版本。input2的chrome为47通过/1失败：后继回归夹具在同尺寸SetWindowPos后误等CHANGED；input3只在该cfg(test)分支先改变child高度以实际触发几何消息，原失败保留。guard回归验证返回后退休资格，不冒充真实颜色/文件模态人工验收。

前台限定恢复有效，仅新profile、自有测试窗。按用户本轮快速收口要求，只完成已启动批次的定向回归和产品构建，不重开与旧批次相同工具边界的普通前台尝试。本批没有启动新可见fixture或发送UI输入；普通可见capsule/颜色文件模态/完整端点键盘及人工IME未验，不能由隔离HWND回归推定通过。旧前台工具边界和清理失败回执继续保留。当前capsule仅公共Art参数；普通sticker的image上传入口及Local签名默认显隐元数据仍未完成，输出不提供参数editor。普通完整Art/Live/端点键盘及人工IME、普通双产品、物理多屏/跨机/DPI、剩余全量/合计资源长稳、最终完整隔离包及用户全面验收仍开放。Talk构想和旧Web/Tauri混用不列阻塞；三冻结副线/v464固定包/用户release97保持，不启动Loom原生迁移，不提交推送发布部署，总迁移active。

## 端点键盘所有权（v535）

`chrome::keyboard`独占线程内焦点、原宿主身份、Catalog和单槽选源；`keys`只识别精确键位、方向与松键位图。使用原每节点两条owned窗口，只SetFocus本线程已核窗口，不SetForegroundWindow、不注入全局输入。同步SetFocus前释放借用，返回核同一Rc候选；目录/显隐/锁/隐藏/捕获/宿主销毁和外部失焦退休，鼠标接管取消键盘选源。键盘选源计入原idle门；连线复用原Creation及唯一Application提交，提交前再核当前目录、两端owner与Workspace，取消保旧边。不新增依赖、协议、权限、线程、timer、每端口HWND或像素/队列预算。

本批选定**75项Rust passed**，12项新增，选定终态0 failed/1既有默认ignored，非整库。覆盖实际自有端点HWND焦点、遍历/首尾/反向、展开、长按和松键、唯一连线提交、重连取消、提交前锁定、隐藏端口、过期owner、捕获、外部失焦、鼠标接管及同步SetFocus重入；原chrome/参数面板/结束会话/Live输入取消直接依赖同批验证。官方fmt、版本、全仓strict行数、strict all-targets Clippy、固定SHA库、普通Release及workspace目标通过。9项源码/版本delta均UTF-8无BOM，3个新文件，Rust最大337有效行，无新增例外。首次runner因误选原默认ignored鼠标捕获项而拒绝，原回执保留；只读重解析原日志并核未改源码声明，未重跑成功测试、不把ignored计通过。

前台限定恢复有效，仅新profile、自有测试窗。本批只运行未切换独立desktop/自有Job回归；没有普通前台输入。v534工具未返回普通产品窗口的边界仍开放，不猜HWND、不换其他自动化API、不重试同条件失败，不将隐藏HWND通过称普通可见键盘通过。可编辑capsule内嵌控制、普通完整Art/Live/键盘及人工IME、普通双产品、物理多屏/跨机/DPI、剩余全量/合计资源长稳、最终完整隔离包及用户全面验收仍开放。Local签名默认显隐字段当前合同未提供；Talk构想和旧Web/Tauri混用不列阻塞。三冻结副线/v464固定包/用户release97保持，不做Loom原生迁移，不提交推送发布部署，总迁移active。

## 端口外观所有权（v534）

`workspace::ports`拥有有界不可变覆盖和独立`AppearanceStamp`；保存Receipt/dirty/恢复Guard及UI Catalog核外观身份，执行图仍只沿原拓扑/配置凭证。`chrome::ports`拥有单槽Open及Application模态前后准入，dialog/model/template只持草稿和标准控件；重新核owner marker、锁/隐藏/结束会话/捕获/退出/保存恢复及目录后才提交。保存用当前覆盖替换旧字段，reset删除旧字段，有外观编辑的初始空宿主不再省略。Live快照和同ID冻结沿同一Workspace保存，不另造State副本。没有新依赖、协议、权限、线程、timer、每端口HWND或像素/队列预算。

本批选定**84项Rust passed**，21项新增，选定终态0 failed/0 ignored，非整库。覆盖中文/非法/重复/null/容量、缺省优先级、外观独立、真实LISTBOX/COMBOBOX通知、取消/关闭/DPI/affinity/字体回收、单槽/过期owner与Catalog、实际参数按钮事务、隐藏端点和原边保留、保存重开/未知字段/reset及Live真实worker保存/Receipt失效/冻结后再保存重开。官方fmt、版本、全仓strict行数、strict all-targets Clippy、固定SHA库、普通Release与workspace目标通过。39项源码输入delta均UTF-8无BOM，10个新增文件，受影响Rust最大497有效行，无新增例外。

Local v2签名端口当前没有默认显隐元数据，仍缺省显示，不从未签名提示伪造包默认，本批不改Loom/wire。可编辑capsule、完整端点键盘、普通完整Art/Live/键盘与人工IME、普通双产品、物理多屏/跨机/DPI、剩余全量/合计资源长稳、最终完整隔离包及用户全面验收仍开放。Talk外部会话控制构想及旧Web/Tauri混用按用户决定排除。三冻结副线/v464固定包/用户release97保持，不启动Loom原生迁移，不提交推送发布部署，整体迁移active。

## Local Art 端口所有权（v533）

`art_execution_ports`和`art_local_ports_record`拥有独立证明及包绑定记录；`document_local_art::Config`只拥有Local身份、共享Arc记录和本次暂定边，非Backend selected_art。`desktop/art_local_ports`独占候选拓扑发布，并在无Win32/网络/脚本回调区更新依赖和配置凭证；`application_art_source`用真实图片输出名及配置Stamp围住连续Add Art。Loom `source_ports`只投影resolver验证的不可变manifest，`local_art_ports`与签发器复核预算/独立签名域。没有新依赖、权限、常驻线程、timer或扩大像素/队列预算。

本批最终选定 **83项Rust passed**，其中17项新回归；当前选定终态failed=0，不是整库。签名域/篡改/预算、旧能力协商、拒绝不重放、图拓扑、中文自定义输入、真实2×2红像素、Live配置换代拒迟到、零输入不交付像素、自定义rendered输出继续建子Art及Save/窗口重建/有限离线均已验。真实Loom LAN HTTPS跨进程联合在线和停服后离线独立核signedPorts、完整资源、正式像素和正式结果；不完整缓存拒绝，父子不混加。Otter strict all-targets Clippy、两仓fmt、官方版本/strict行数、Otter lib/普通Release/工作区与Tile驱动构建通过；Loom签名安全/daemon测试库通过。66项源码delta均UTF-8无BOM，Rust最大500有效行，无新增行数例外。两个原跨仓grant门及两个SDK示例默认ignored未运行；Loom整仓strict Clippy旧债未重跑，不称两仓全绿。

逐端口显隐/默认值与持久化、可编辑capsule/完整端点键盘仍未完成；普通完整Art/Live/键盘与人工IME、普通双产品、物理多屏/跨机DPI、剩余全量/合计资源长稳、最终完整隔离包和用户全面验收仍开放。Talk外部会话控制构想和旧Web/Tauri混用按用户决定排除。三冻结副线/v464固定包、用户release97保持；不启动Loom原生迁移，不提交推送发布部署，整体迁移active。

## Live Add Art 所有权（v532）

`application_add_art::source`统一文档marker与Live稳定Arc输入身份，拒绝同ID冻结后的文档继承旧Live资格；`application_live_add_art`拥有Picker/来源围栏，共用唯一Backend发布事务与原Local链。`art_picker_dialog::Context`只在同步DialogBox栈上借用allowed/session，不给Live伪造State或保存Application裸指针。`composition_live_art`只停止源输入并排单槽意图，Live停止/保存/恢复/退出/系统会话空闲门明确纳入它；`capability_catalog::request_for`复用唯一目录任务。没有新依赖、协议、权限、线程、timer或像素缓存。

最终input4同源定向 **62项Rust passed / 0 failed / 0 ignored**，含9项新测试，不是整库。真实owned目录窗/搜索/IME合成/affinity、独立子节点/默认值、取消/过期/容量、输入停止/单槽/空闲门、Backend真实回环TLS执行和Local签名child原始2×2红像素/源关闭后迟到拒绝已验。官方fmt/版本/全仓strict行数、strict all-targets Clippy及lib/普通Release/workspace三目标构建通过；21份Rust最大496有效行，没有新增行数例外。只分配一次v0.2.33.532，Loom源码未改。

Local 自定义主输入名仍缺完整目录/图/状态/保存恢复接线；逐端口显隐及默认值/持久化、可编辑capsule和完整端点键盘仍开放。普通Art/Live/完整键盘与人工IME、普通双产品、物理多屏跨机DPI、剩余全量/合计资源长稳、最终完整隔离包和用户全面验收仍须继续。Talk构想与旧客户端混用排除，三冻结副线/v464固定包保持，不启动Loom原生迁移，总迁移active。
普通前台及原失败边界见[构建记录](../development/WINDOWS_NATIVE_BUILD.md#v532-live-源-add-art-与独立子节点)。

## 工作区 chrome 所有权（v531）

`application::chrome`及model/scene/draw/input/window/strips/integration叶独占三态、Catalog/几何快照、单槽手势、GDI穿透线层与每节点最多两个owned稀疏端点条；回调只持Rc UI数据和弱Binding，不持Application/文档可解引用裸指针，不复制图片。相同Catalog复用图，宿主消息更新几何，隐藏只影响绘制；拖动只扩线层范围，不逐move重建端点region。
菜单和chrome共用`application_links::apply_link_creation`唯一目录/端口/两端锁与隐藏/Workspace提交围栏；有效drop才替换，取消保旧边。Live原renderer只接三态可见性，源纹理不变；捕获保护、线层/端点各33,554,432像素预算、弱引用/owned销毁及原恢复/保存/退出/系统会话空闲门保持明确。无新依赖、协议、权限、线程、timer或全局输入。
19项新chrome回归及原Live GPU去框/恢复像素通过；本批定向90 Rust passed/2默认ignored、严格静态/三目标构建通过，非整库。逐端口显隐/持久化、可编辑capsule和完整端点键盘仍缺；工具无法独立选自有文档/端口，本次普通前台未发送输入且仍未验，不改样式/捕获保护绕过。详见[构建记录](../development/WINDOWS_NATIVE_BUILD.md#v531-工作区三态与跨窗端口连线)，整体迁移继续active。

## Add Art 入口所有权（v530）

`node_menu::art_picker`及model/dialog/template拥有精确Shift+1、源绑定不可变目录、256 UTF-16搜索、真实标准控件和同步模态生命周期；`application_add_art`独占消费单槽意图及普通能力的隐藏子节点/连线事务。`application_local_art_node`保原OpenArt行为，Add Art使用强制创建并在启动前复核专属predicate；`application_propagation`仅接原自动通知链，不能绕过notify/listen/upstream。
16项新测试保护目录index/禁用、来源/权限、真实隐藏控件/IME合成/DPI/捕获排除、同步建窗撤销、同Art独立创建、失败回滚和已落盘系统会话准入；相关回归见[构建记录](../development/WINDOWS_NATIVE_BUILD.md#v530-shift1-原生-art-入口)。没有新依赖、协议、权限、常驻线程或像素预算；前台/人工IME及Live源范围不能从隐藏测试推定。

## 参数面板所有权（v529）

`document_window::parameter_panel`及model/controls/rows/events/keyboard/layout/theme/transaction叶拥有惰性文档专属owned窗、稳定Rc/marker、有界本地草稿与消息导航；`application_parameter_panel`拥有应用四窗预算、调度及捕获列表接线。`capability_parameter_menu::commit`是菜单/面板唯一候选事务；`parameter_value`保原Spec，`parameter_view::after_local_text`仅推进同步未连线文本的既有视图配置，不能用于任意外部/模态变化。
23项新测试覆盖真实隐藏EDIT/COMBOBOX/Slider、16字段批量、dirty→Run/step/Choice、JSON导致索引移位、连线/配置失效、DPI/IME草稿、分页/捕获排除/宿主先毁/预算/托盘与清理；相关回归见[构建记录](../development/WINDOWS_NATIVE_BUILD.md#v529-tab-原生参数编辑窗)。没有新依赖、线程、网络合同或权限；普通前台只是部分证据，端口手势/候选/Live外围不是本批完整实现。

## 工具面板键盘所有权（v528）

`desktop/tool_panel_keyboard.rs`独占Ctrl+E/F4精确识别、bit30重复抑制、同宿主/面板身份和同步重入后的准入；`tool_panel`复用原modeless工具窗及WM_CLOSE/线程焦点回还，`procedure`只接线。Tab、Art入口和三态Unit chrome不被合并到该模块；不改内容/图像/选择、全局前台、权限或持久化。
`tool_panel_keyboard_tests`拥有7项新纯键位/真实隐藏宿主与子按钮/门禁/身份/资源回收回归，本批合计32项定向Rust通过，严格静态和三目标构建通过。普通前台证据边界及清理结果见[构建记录](../development/WINDOWS_NATIVE_BUILD.md#v528-ctrle-工具面板键盘闭环)；不是整体迁移验收。

## 截图/取色历史所有权（v527）

`native/src/capture_history.rs`及image/store叶拥有原版同屏两列表、各24条、兼容JSON与独立原生profile事务；`image_decode_history`只在WIC源先缩到240，不完整分配旧大图。未知根字段与暂态失败截图原条目保留，不读取URL/路径、不自动迁移旧Tauri配置。
`desktop/capture_history_ui`、worker/window及application_capture_history叶分别拥有共享View/有界意图、惰性单后台槽、标准owned窗和主消息泵接线。回调不执行磁盘/建线程；同步Win32外释放借用；Origin用UnitId+Rc隔离旧宿主，队列最多4份共享Image，shutdown先join再关闭wake。`PreparedDocument.record_history`显式区分真实capture、普通打开和粘贴；只有整批成功提交后入栈，复用缩略图不再次入栈。
本批55项Rust定向通过（16新增），三目标编译和严格静态门通过；普通产品的历史owned窗未被工具返回，真实键盘/视觉不冒充通过。原前台Job五秒清理等待失败保留，只读核已无自有产品进程；不是严格/正常清理证明。代码/验证与失败证据见[构建记录](../development/WINDOWS_NATIVE_BUILD.md#v527-原版-ctrlh-截图与取色历史)，无新依赖/网络协议/授权。

## JS 测试前台所有权（v526）

原 `surface_javascript::fixture`（`surface_javascript_fixture.rs`，仅 cfg(test)）继续独占 READY→G 启动前提和 Foreground guard；不新增产品入口或共享业务层。focus 先核目标 PID，guard 只保留及恢复当前测试进程 HWND，移除跨线程 AttachThreadInput 兜底，真实前台断言不变。跨进程自有 UIA target 仍只登记恢复 owner，不代目标发起激活。
本轮新增 3 项所有权回归、既有 6 项 Application/2 项 guard 共 11 项 Rust 通过；临时 WinForms sponsor 在 linshi，逐次按钮后才建 child，限定 PID 授权，不是普通产品键盘或人工 IME 证明。没有产品结构/依赖/权限/预算变化；详见[构建记录](../development/WINDOWS_NATIVE_BUILD.md)。

## Tile 有界呈现证据（v525）

`desktop/composition_present_status` 只读观测原 D3D11/D3D12 Present HRESULT，每交换链最多记录 16 次状态变化，不改变仅 `S_OK` 确认规则或后端选择。
`tile_shared::diagnostics` 独占原可见性判断的固定分类和 owner 生命期内的有界阶段位图。窗口 paint、Shared 确认及心跳正文准备不记录身份、凭据或像素；stderr 写入发生在 Shared 锁外。
`HeartbeatPrepared*` 不是发送或 ACK 回执；没有新增线程、timer、协议、重试、资源或授权预算。本次普通 Release 在未切换独立 desktop 返回 `DXGI_STATUS_OCCLUDED`，业务帧停在 `FramePaintUnconfirmed`，没有取得 applied；这不反推旧 v524 的未观测根因，也不证明普通可见桌面通过。运行证据与验收边界见[原生构建记录](../development/WINDOWS_NATIVE_BUILD.md)。

## 原版紧急退出双通道（v524，隔离定向通过）

补齐原版已实现的两条紧急退出：完整 Esc+Delete 三次组合（相邻严格小于400ms，双键重叠且完整释放），以及独立 watchdog 的 Ctrl+Alt+Shift+F12。主消息循环与 child 共用纯状态机；child 在配置/UI 前分流，先见松开才 arm，以原版8ms周期只读键位，不注入输入、不记录文本。
复用原同EXE受限白名单 launcher 和 Hello/Ready；直接父PID、创建时间、同EXE路径及pipe创建者共同认证。普通恢复权限仍只有查询/等待，只有紧急认证额外取得 PROCESS_TERMINATE，并只使用保活原父句柄。正常Drop和握手失败只回收自己新建watchdog，不终止Job或承担Live恢复责任的child。紧急退出绕过保存/析构，不能保证未保存工作保留；原Live父死恢复机制继续负责已登记源。原生未修改全局光标约束，故不无条件重置其他程序的 ClipCursor/ShowCursor/SetSystemCursor。
当前选定Rust按crate/完整名去重 106 passed、6 默认ignored，非整库；11项新增回归覆盖严格400ms边界、完整释放、启动已按住、缺失keyup/迟到消息、参数、认证失败不arm、握手失败、正常Drop及阻塞牺牲父进程的三连/备用终止和父死child退出。另复测原恢复认证/握手/管道/launcher、三进程父死恢复、系统会话消息/Checkpoint和普通固定SHA EXE工作区。全部在未切换的独立desktop/自有Job执行，不申请SWITCHDESKTOP，不向用户应用发送输入。
strict all-targets Clippy、lib/普通product/workspace/恢复认证及session目标构建、官方fmt/版本/全仓strict行数/diff通过。input1因Application达到503有效行而在构建前失败，原回执保留；input2按职责提取application_loop，保持原消息泵/显示恢复/保存/退出顺序，不增行数例外。input2 Clippy通过后，因只读审查发现局部消息可漏收keyup，停止本轮自建在途构建树并保留中止记录；input3主通道按当前键位快照保守累计，不凭历史消息制造重叠，新增负向回归。源码只分配一次v0.2.33.524，Loom未改。
新增常驻进程单独实测180秒（97点）：产品及watchdog全程存活，含idle60秒和三轮hidden20秒/resumed20秒；正常隔离QUERY/END后整棵自有树归零、desktop关闭、工作区保留。该4进程树（产品、watchdog及两个conhost）Private commit为8.691–8.957MiB，Working set合计61.430–61.578MiB（共享页可能重复计），模式内单核CPU约0.156–0.260%，Handles658/GDI33/USER16稳定。Ctrl+1仍记录已有热键占用0x80070581，不冒充快捷键验收。资源明细见main-emergency-exit-1/runtime-audit-3.json；不是Otter+Loom总资源、GPU、物理输入或长稳验收。
本批仅关闭源码与限定隔离门，不恢复前台自动化，不证明物理三连/F12或普通可见混合链通过。v518前台身份变化来源及v517普通可见混合门未关闭，不拿当前隔离通过覆盖历史失败。人工IME/旋转EDIT、物理多屏/跨机/DPI热插拔、资源长稳及最终隔离整包/用户全面手测仍开放。Talk外部会话控制按用户决定排除本轮，旧Web/Tauri混用不重列必做；三冻结副线、v464固定包及真实profile/设备不动，总迁移active，不是正式发布。

## 系统结束会话持久化准入（v523，隔离消息链验证）

WM_QUERYENDSESSION只消费原Save/Checkpoint已提交凭证；未保存、在途/排队请求、Live待交付操作及本地Art输入/确认/结果未结算时立即拒绝，设置短ShutdownBlockReason，不在同步回调等待保存或弹框。批准后共享Queried阶段封住新业务和标准子控件编辑；END FALSE恢复，END TRUE不可逆进入Ended并沿原统一shutdown排空。系统结束不等于删除最后Unit，不写空工作区。
MAIN独占session_end及application_session_end、Application/文档/Live/Relay/Surface接缝与专属测试。查询回调仅在只读wait栈词法作用域安装并RAII撤回，按Control身份隔离且禁止重入复用；主循环/shutdown的owner只持Weak<Control>，不保存Application裸指针。surface_edit_pending只提取原pending观察并新增空闲观察，surface_window_session_end独占Surface准入接缝；不复制保存、输入或排空框架。


## Tile服务端布局确认准入（v522，限定后台联合通过）

MAIN独占tile_shared/admission叶及tile_presenter、tile_surface_action_dispatch接缝；本地paint不再等同服务端applied。原串行heartbeat经严格成功ACK且generation及完整Key（endpoint/lease/wall/revision/clock）仍匹配后，才开放未发送交互。null/错误revision或黑场/冻结/识别的减权报告在HTTP前即撤旧确认，防止回复在途时本地重paint沿用旧资格；旧Key捕获/迟到ACK不退休后继。同布局普通续期和纯Art redraw不强制重新确认，但实际新Art paint仍独立必需。换代/过期/clear/stop永久退休；取消已有执行保留原通道。
未发送意图只在原有界队列/原1500ms期限等待，不授新期限，不重放未知HTTP写。昂贵准备后及写前仍复核资格，短暂未确认可归还原意图；没有新线程、timer、协议、权限、依赖、队列或像素预算。新纯生命周期/实际TLS顺序/实际Runtime与已paint自有HWND三叶共11项确认门回归，核成功、在途、严格拒绝、撤权、原eventId恰一次、原期限到期、准备期间撤权及paint与服务确认互不替代。
本批选定Rust按crate/完整名去重301 passed、5默认ignored，非整库；11组定向、双GPU提交像素读回、默认GPU识别/DPI/销毁门、普通固定SHA EXE工作区6项，以及6条真实Loom LAN父门通过，父子不混加。input3已过Tile窗口组按独立cfg(test)叶精确delta/SHA复用，其余组在input5运行。LAN包含键盘Switch/Select、Slider、文本/IME合成边沿/表单、无handler值、原click/确认/拒绝/取消、原图片/冻结/黑场/恢复/停止。私有desktop不申请SWITCHDESKTOP；GPU门仅自有离屏NOACTIVATE，不SetFocus/SendInput，不冒充普通可见完整键盘或人工IME验收。
strict Otter all-targets Clippy、lib/普通产品/workspace构建、官方格式/版本/全仓strict行数/diff及精确UTF-8无BOM/空白通过，10份Rust最大396有效行。input1编译失败（诊断变量作用域/测试COM私有入口）、input3新TLS夹具240秒时限失败和input4诊断30秒时限失败保留；最小自有回环实验实证Windows listener克隆副本恢复阻塞，显式nonblocking立即WouldBlock，input5只修该cfg(test)叶并重验。没有放宽时限或重跑刷绿；input2仅冻结未构建，只分配一次内部版本。产品/workspace与所有Loom输入未变，按完整delta/固定SHA复用input3产品目标及Loom库/handler，既有101条Clippy/过期行数例外未重跑或宣称修复。证据main-tile-applied-admission-1/runtime-audit-5.json、listener-clone-probe.json及STATUS.md。
当前只关闭已证的服务端applied准入缺口和本批限定联合回归；不以当前LAN通过证明v521 input6或v511历史HTTP失败的唯一根因，也不覆盖旧失败。v518全局前台变化来源、v517普通可见混合门、普通产品完整键盘/人工IME候选窗与旋转EDIT、物理多屏/跨机/DPI热插拔、长稳、其余迁移退出条件、最终隔离整包及用户全面手测仍开放。前台自动化继续暂停，总迁移active；三旧副线/v464固定包/真实profile设备不动，旧Web/Tauri混用不重列必做。

## Tile导航原子交接（v521，后台定向通过，LAN部分失败）

Tab导航以Rc身份跨旧Select/Slider销毁、ReleaseCapture、滚动同步paint及新控件创建复核；重入取消/后继导航后旧栈结束，不恢复旧候选或退休新owner。MAIN独占tile_surface_keyboard_atomic_tests，三项真实HWND回归覆盖两种值控件销毁取消/后继实际EDIT及显露paint取消。隐藏WM_PAINT只验证更新区，不提交CPU画面或确认呈现；同步DPI/销毁后的renderer仍退休。没有新线程、队列、协议、权限、依赖或预算。
键盘验证在不申请SWITCHDESKTOP的独立desktop/自有Job进程树中运行；cfg(test)仅在runner专属且再核非input desktop时选择原CPU真实提交并禁用本测试线程系统IME，仍保painted准入。双GPU提交像素读回及默认GPU隐藏识别/同步DPI/销毁另用默认desktop自有离屏NOACTIVATE HWND，无SetFocus/SendInput，不冒充普通可见GPU完整键盘或人工IME验收。
本批选定Rust按crate/完整名去重275 passed、1 failed、5默认ignored，非整库；三项原子回归及既有键盘/取消/滚动/值/表单、三条真实Loom LAN键盘父门（Slider/文本/无handler值）、普通固定SHA EXE工作区原5项及新夹具1项通过，父子不混加。values LAN本次Switch意图已受理且通过写前校验，HTTP未获确认，服务端无ACK，原期限失败；保留队列/source-at-deadline/profile及stdout/stderr。input7仅加cfg(test)有限错误分类，诊断尝试exit=0、分类=[]；这不是因果修复，不用后续诊断成绩覆盖原失败，不发布完整INTEGRATION_VALIDATED。
input4实际通过七组定向及五GPU门；input6仅工作区integration夹具变化，input7仅测试错误诊断，按完整delta/归一化产品token及产物SHA复用未受影响成绩，不伪称重跑。受影响strict Clippy/lib/workspace、官方格式/版本/Otter全仓strict行数/diff及精确UTF-8无BOM/空白通过，8份Rust最大469有效行。空desktop EnumWindows的false/ERROR_SUCCESS误判经最小API实验证实；仅零错误且无结果沿原15秒等待，枚举前清LastError，非零错误/部分结果仍失败，保PID/类/可见性，不增权限或sentinel。input5新测试manual_dangling_ptr修成标准dangling_mut；input2隐藏CPU E_HANDLE、input3自有中断、input4枚举及input5静态失败均保留。Loom全部源码未变，复用固定SHA库/handler；101旧Clippy/到期行数例外仍失败，不称两仓全绿。证据见main-tile-focus-isolation-1/runtime-partial-audit-7.json及STATUS.md。
本批状态PARTIAL；本次values LAN失败、v511历史派发超时及v518前台变化来源均未定位，不将其混为同一已证根因。v517普通可见混合门、普通产品完整键盘/人工IME/旋转EDIT、物理多屏/跨机/DPI热插拔、长稳、其余退出条件、最终隔离整包及用户全面手测仍开放。前台自动化暂停，总迁移active；三旧副线/v464固定包/真实profile设备不变，旧Web/Tauri混用不重列必做。

## 本地Art自动触发原生链（v520，限定联合通过）

原生显式配置→原Live唯一控制worker Configure/Claim→签名child onLiveTrigger→原Application/Results实际事务→ACK及Stop/回收已接通。独立不可逆自动授权绑定完整实例/工作区/包、会话/epoch与条件revision；Preview不等授权，重连不复活旧gate。单槽/16KiB、原期限与共同容量门不变，不新增线程、timer、持久队列、权限或WebView。
用户在既有观看系统菜单配置严格Condition并明确“允许自动回调”；停止先撤本地gate。回调不伪造click/gesture、不请求焦点或恢复暂停timer。Applied只在正式端口原子提交后产生，formalRevision为当前执行实例成功提交序号，不是磁盘版本；NoResult只在实际present后报告。原Results最终predicate再核Runtime/Lease及固定期限，Stop与canonical事务共用无Win32/网络/脚本回调的短锁。
本批仅关闭本地Art自动触发的源码/定向/隐藏Application及真实LAN联合接线，不是普通可见双产品、真实跨机或最终包验收。v518全局前台身份失败、v517可见混合门/v511历史键盘根因仍开放，前台自动化继续暂停；人工IME候选窗/旋转活动EDIT、物理多屏/跨机/DPI热插拔、长稳、其余迁移退出条件、最终隔离整包与用户全面手测仍开放。三旧副线、v464固定手测包与真实profile/设备保持；旧Web/Tauri混用不重列必做，总迁移active。 见[构建记录](../development/WINDOWS_NATIVE_BUILD.md)。

## 本地Art自动触发服务边界（v519）

local_art_live_trigger协议及Reply叶独占版本化Device-only自动回调合同；local_art/live_trigger只接原签名宿主与当前审批，runtime/live_session_local_trigger及operations独占登记、单槽、Claim/ACK和退休。Surface与本地目标共用live_session_trigger_evaluation唯一条件状态转移，合计沿原64注册门；不伪造Surface附件、不改grant四权限。
宿主显式automatic_local_callback授权绑定完整Art实例、工作区/代际/包、Live会话/epoch、条件revision及独立authorizationId；Preview不等自动授权。Claim在HTTP响应前消费payload，重复/未知领取不重放；ACK仅报告宿主结果，不证明canonical保存或接收端呈现。本地delivery至多16KiB，超限明确停用；原观察新鲜度30秒、grant期限、墙钟高水位/每事件期限与单调期限共同围栏，查询不续期。停止、最后Viewer断连、审批修订变化、过期和Session close回收或拒绝执行，重连/重批不复活旧授权。没有新增线程、timer、持久队列、依赖或脚本权限。
本批只关闭Loom本地Art Live触发服务实现及HTTP定向验证；Otter实际配置菜单、独立自动授权owner、onLiveTrigger私有command/SDK、签名child执行、正式结果/取消接线和真实HTTPS/WSS联合仍由MAIN继续完成，不能以此宣布本地Art触发产品完成。v518全局前台身份失败、v517可见混合门、v511历史键盘根因仍开放，不恢复前台自动化。人工IME候选窗/旋转活动EDIT、物理多屏/跨机/DPI热插拔、长稳、其余迁移退出条件、最终隔离整包与用户全面手测仍开放。三旧副线、v464固定手测包及真实profile/设备保持；旧Web/Tauri混用不重列必做。 见[构建记录](../development/WINDOWS_NATIVE_BUILD.md)。

## Tile编辑取消与测试回执边界（v518）

Tile Popup明确区分Select，真实打开的下拉列表先消费Esc并抑制同次长按；keyboard_window以Rc身份围住取消/同步销毁，只在同一已paint快照保留Tab位置，迟到epoch不得取消后继owner；外部失焦仍完整退休，父Tile拒同次Esc长按连带关闭，新鲜Esc仍退出。取消退休未发Edit后缀，不重放未知写或丢已接受本地值。
keyboard_joint_access及keyboard_fixture_tests仅cfg(test)，Tab后才计按钮受理，避免字段change冒充Enter成功；测试线程键表完整RAII恢复，生产Ctrl+Enter拒绝不放宽，并记录实际焦点/paint/修饰键。专属Combo实验只在未切换desktop及新测试线程运行，不申请SWITCHDESKTOP；只禁用该线程真实IME，人工组合消息不等人工候选窗验收。
当前唯一终态失败为tab_edit_to_button_commits_local_draft_and_enter_repeat_does_not_duplicate的全局GetForegroundWindow前后身份断言，尚未确认是用户切窗、产品行为还是其他来源，不用历史通过覆盖、不反复重跑、不恢复前台自动化。故本批仅PARTIAL，不发布INTEGRATION_VALIDATED或完整键盘/整项迁移完成。v517混合可见门、v511历史根因、本地Art Live触发独立授权/执行/回执合同、人工IME/旋转活动EDIT、物理多屏/跨机/DPI热插拔、长稳、最终完整隔离包和用户全面手测仍开放；旧Web/Tauri混用不重列必做，三旧副线/v464固定包及真实profile设备保持。 见[构建记录](../development/WINDOWS_NATIVE_BUILD.md)。

## Tile普通产品与交换链证据分层（v516）

presentation_readback_test独占cfg(test)有界staging/Map/预算与读回像素；composition_d3d12_readback_test读成功Present后的上一物理buffer，composition_readback_test读同一D3D11双缓冲flip-sequential链的已提交逻辑buffer1。software_surface/render_software/render_surface仅有cfg(test)委托。tile_presented_pixel_tests复用产品Renderer，独立矩阵核图片/Art/旋转/crop/透明叠层/换帧/黑场恢复。唯一生产结构变化为Renderer::new复用私有from_surface初始化，测试建链前选定后端，不在同HWND叠建两条GPU链。
native/tests/tile_product_runtime与support/tile_product_window独占普通固定SHA EXE、真实CLI/物理输出、隔离profile及自启PID/精确类名的窗口验证；Loom tile_product_native_joint只负责真实生产库LAN HTTPS/签名配对与续签、layout/presentation及权威ACK。四象限独立RGBA常量对应六阶段running/旋转/黑场/恢复/crop/冻结，外部仅采四个前后均确认无遮挡的桌面点，随后正常关闭和endpoint offline。桌面合成像素不冒充产品交换链直接读回，库内各后端读回也不冒充普通产品可见运行；父子不混加。子产品由Kill-on-close Job持有，失败/超时只清理自己的进程。初始可见门因遮挡明确失败后，按用户授权由Computer Use仅激活驱动的普通自有启动器一次，驱动核实际前台后向已核PID/类的同测试子Tile转交一次。不以该显式激活声称产品自动置顶或全程无激活；不移动鼠标、不注入用户应用、不改物理显示或真实数据。没有新产品诊断开关、依赖、路由、权限、线程或资源预算，旧三副线及v464固定包冻结。
本批只关闭上述本机单屏普通EXE静态图片可见链和产品Renderer各后端提交目标读回，不关闭普通EXE可见Art/Live、完整Tile键盘、人工IME候选窗/旋转活动EDIT视觉、物理多屏/跨机/DPI热插拔、长稳、本地Art Live触发、其余迁移退出条件、最终完整隔离包或用户全面手测。v511历史超时根因仍开放；旧Web/Tauri混用按用户确认排除。 见[构建记录](../development/WINDOWS_NATIVE_BUILD.md)。

## Tile本地滚动条与同步重入（v515）

tile_surface_scrollbar独占已paint滚动条身份和捕获坐标；desktop/tile_surface_scrollbar_window独占单槽Thumb/Page手势、Rc token、捕获及timer回收。复用surface_scroll的Bar/Drag、唯一surface_scroll_repeat allocator/Repeat、原Presentation COW计划与tile_surface_scroll_window安装/实际paint，不复制Surface业务。Repeat登记提升至desktop，普通Surface保同名alias；tile_window_test_access只承接cfg(test)窗口入口。
滑块可拖出原track/窗口，轨道立即翻页并在400ms后每50ms至多一步，不追赶积压；thumb到指针、移出track或反向时暂停，回到原方向再继续。活动EDIT由滚动条提交一次，保中文emoji草稿及无handler表单；IME组合拒绝接管。prepare空隙只被动保留且不滚动，空隙抬起释放、不采用迟到位置。同步paint前移出owner，取消轮换token，旧回调不得恢复旧手势或覆盖successor；隐藏/冻结/黑场/过期/撤权/DPI/关闭/丢capture及wheel/Tab接管都清捕获和timer。初始资格仍核topmost/crop/旋转/clip/布局/view/代际，捕获后屏外坐标不授新命中资格。不改scene/canonical、网络动作、权限、原队列/像素/TTL预算；无新hook、线程、依赖、WebView或用户应用输入注入。
本批仅关闭Tile滚动条实现、定向与隔离LAN集成；inputCapabilities仍pointer。完整键盘、人工IME候选窗/旋转活动EDIT视觉、普通可见Tile与最终交换链读回、物理多屏跨机/DPI热插拔、长稳、本地Art Live触发合同、其余迁移退出条件、最终完整隔离包及用户全面手测仍开放。v511历史超时根因仍未确立，后续成功不反证旧故障已修复；旧Web/Tauri混用按用户确认排除。 见[构建记录](../development/WINDOWS_NATIVE_BUILD.md)。

## Tile滚动快照与实际paint（v514）

surface_scroll及surface_presentation_scroll_state独占COW滚动快照、未滚动布局SHA与受crop约束的显露；desktop/surface_scroll_input只共享原系统滚轮解码。tile_surface_scroll_render/window复用原Presentation与唯一HWND，先准备、提交/隐藏活动EDIT并退休旧手势，再安装和实际paint；同步Win32调用前释放借用。tile_surface_focus/text/local按未滚动布局保草稿，不借屏外保留身份激活输入。tile_surface_actions在原预算中携带实际激活时的scroll，原唯一worker新鲜布局恢复该几何后再校验。tile_surface_scroll_memory只保留至多四个同key/view/代际/尺寸的COW偏移跨越prepare空隙，不保留旧Art/scene/像素或呈现资格；减权/过期/身份变化清理。
滚轮复用系统行/字符/整页偏好、Shift水平与Ctrl让出，嵌套祖先只消费一次；Tab/反向Tab能显露屏外字段，仍受crop、四向旋转、最上层命中和有界搜索约束。重投影重装嵌入文本/Select/Slider绘制缓存，保中文emoji草稿和无handler本地值；同view/代际/尺寸新snapshot继承并钳位scroll，换代重置。IME组合不提交或换焦；隐藏/冻结/过期/准备空隙拒新滚动，旧按压不重新命中新位置。不改权威scene/canonical、表单字段范围、协议、权限、队列/像素预算、原1500ms/120秒门或未知写不重放规则；无WebView、新依赖、hook、线程或输入注入。
本批关闭Tile本地wheel与Tab屏外reveal的实现、定向和隔离LAN链；不等完整键盘、滚动条拖拽/重复翻页、人工IME候选窗/旋转活动EDIT视觉、普通Tile输出EXE可见链、最终交换链读回、物理多屏跨机/DPI热插拔、长稳、本地Art Live触发合同、其余迁移退出条件或最终完整隔离包/用户全面手测。inputCapabilities仍为pointer；旧Web/Tauri混用互通按用户确认排除，v511旧超时根因仍未确立。 实际验证见[构建记录](../development/WINDOWS_NATIVE_BUILD.md)。

## Tile真实paint准入与写前生命周期（v513）

普通意图只在对应Art的实际paint Frame就绪后取出，候选publish不代表已呈现。原唯一worker持Command，Queue仅保弱Ticket及固定计费；短暂prepare/paint空隙可恢复尚未发送的原意图，但事件ID、顺序和原期限不变。Scene/Bitmap/Presentation准备后再次核真实paint、job/授权、Edit、预留及期限，最后锁内围栏构成一次HTTP尝试的提交边界。
停止或退休不能承诺撤回已提交字节，但会拒绝迟到结果；HTTP后没有defer/重放路径。原队列、资源预算、期限和冻结Cancel语义保持，没有新增线程、hook、依赖或权限。144项相关Rust去重、11新回归、9条真实LAN父调用及普通工作区通过；不是普通可见Tile、最终交换链、物理跨端或最终包验收，v511历史超时根因仍开放。详见[构建记录](../development/WINDOWS_NATIVE_BUILD.md)。

## Tile本地值cache（v512）

tile_surface_local独占无handler的Switch/Select/Slider本地值与源值SHA stamp，desktop/tile_surface_local_window独占UI线程cache、paint身份校验和生命周期，tile_surface_local_render只投影到原Presentation绘制缓存；不改权威scene或持有Art像素。复用tile_surface_focus、既有鼠标/键盘控件及原唯一Art串行worker，不新增输入框架。
无handler值可鼠标/键盘编辑；同合同同源换快照保本地值，新权威源值替换，view/代际/合同/布局/几何变化退休。准备空隙仅被动保留且拒新输入，隐藏/冻结/黑场/停止/撤权/过期/DPI或窗口换代清理；Select取消不变值，Slider取消恢复此前接受值。与文本共享每view64/总256字段，不加倍预算；本地操作不offer空动作。surfaceValues仍只含input/textarea/number字符串，不加入Switch/Select/Slider，保原16字段/48KiB表单门、祖先click、队列/权限/资源预算。
v511的键盘/Tab接线及当前回归虽已通过，input6的values LAN派发超时仍缺根因证据；后续成功不等生产修复，不发布该批INTEGRATION_VALIDATED，也不把旧失败改写为通过。 验证及未验边界见WINDOWS_NATIVE_BUILD的v512记录。

## Tile文本/IME与当前草稿表单（v510）

tile_surface_text独占真实已paint字段身份/合同和源→crop→Wall/输出旋转几何；desktop/tile_surface_text_owner/window/popup独占有界EDIT会话、IME、草稿/ticket、表单装饰与HWND回收。surface_edit_session只复用unbound本地编辑/编码入口，真实Surface仍要求原Binding，不能互相绕过权限。tile_surface_edit独占跨字段Group与原worker栈上Dispatch清理，tile_surface_edits沿原32槽动作队列串行化前缀，不新增动作worker。
文本、textarea、number以真实EDIT通知生成input/change；IME组合抑制中间态，textarea归一CRLF，number拒无效编辑并支持step。无handler字段只保本地draft，不伪造change；实际按钮沿原16字段/48KiB表单门读取当前草稿，已移除或合同不匹配字段不复活。连续input只合并同Edit尾项，跨字段及后续按钮保顺序；在途/ACK等待保固定期限，终态只授一次原1500ms就绪派发窗口。原64个120秒跟踪/事件64KiB/32槽8MiB不扩大，重复GET不续期，未知写不重放。
新权威未paint时不按旧scene结算ticket；准备空隙只被动保留同租约/view/代际草稿。新paint后完整复核合同/几何，失败、取消、隐藏、冻结、停止、撤权、DPI/窗口换代清理编辑组和后缀。每view最多64/总256字段；owned popup marker/单调epoch拒旧HWND消息，Win32同步调用前移出owner避免RefCell重入，活动EDIT直立显示，非活动草稿复用原唯一D2D/crop/旋转/clip绘制。没有WebView、新线程、hook、依赖或权限。
本批关闭Tile文本/textarea/number、IME合成边沿及文本草稿表单按钮的隔离闭环，不等完整键盘/Tab导航、已安装输入法候选窗人工验收、旋转活动EDIT视觉、全部值控件本地draft/完整表单。inputCapabilities仍为pointer。普通Tile输出EXE可见链、最终交换链读回、物理多屏/跨机/热插拔/真实DPI、本地Art Live触发合同、长稳、其余迁移退出条件、最终完整隔离包和用户全面手测仍开放；旧Web/Tauri混用互通已按用户确认排除。 验证与失败边界见WINDOWS_NATIVE_BUILD的v510记录。

## Tile Slider 原生拖动与结果收敛（v509）

tile_surface_slider 独占 canonical Range/编辑合同及单手势身份；tile_surface_edits 独占原队列尾 input 合并、同 Edit 后缀退休及前缀就绪接缝。desktop/tile_surface_slider_window 独占原 HWND 的鼠标捕获与释放、抬起提交、单槽异步预览和取消；surface_paint_controls 只准备嵌入控件绘制缓存/paint-only 预览，不改 scene 或普通 Surface 子 HWND 权限。
原 Art 唯一串行 worker 先 GET 新鲜 state 再至多一次 POST；32槽/8MiB、64个120秒跟踪、事件64KiB不变。Slider 同手势依赖仅在固定在途/ACK跟踪期限内等待，前缀终态 GET 只授一次原1500ms派发准入，重复GET不续期；失败、拒绝、取消、合同/窗口换代及等待到期都退休后缀，不重放未知写。新快照撤旧呈现、Art准备完成及候选发布但未paint的过渡只被动保留同租约/同view/代际手势，不发送空白期输入；实际paint后完整复核合同/几何，准备期抬起不提交。没有扩大网络/业务超时、鉴权或资源预算。
只覆盖声明了 input/change 的 Slider 鼠标交互；无 handler 仍只读，文本/IME/完整键盘与完整表单仍开放，inputCapabilities 保持 pointer。普通 Tile 输出 EXE 可见链、最终交换链读回、物理多屏/跨机/热插拔/真实 DPI、本地 Art Live 触发合同、长稳和最终整包/用户全面手测仍开放；旧 Web/Tauri 混用互通已按用户确认排除，不再计为本轮待办。 验证与失败边界集中见 WINDOWS_NATIVE_BUILD 的 v509 记录。

## Tile 声明式离散值动作（v508）

tile_surface_value 独占已呈现字段命中、Switch 布尔翻转、Select 有序索引和值合同及新鲜派发校验；surface_input_event 只共享 wire 编码，不伪造普通 Surface 挂载资格。desktop/tile_surface_value_window 独占原生菜单 RAII、取消和模态重入复核，原 Tile pressed 生命周期负责退休手势。沿实际 paint 的 view/generation/snapshot/placement/node 身份、crop/旋转/clip/最上层命中和原 Art 唯一串行 worker，不新增线程、hook、依赖、权限、像素面或预算。
UI只生成有界意图，Switch保祖先click顺序，Select取消/当前项不发事件；模态及后台提交各自复核当前来源，权威值只由正式服务结果更新。只覆盖声明了 change 动作的离散值；无 handler 仍只读，没有新增本地 draft，不广告文本、IME、连续 Slider 或完整键盘导航，inputCapabilities 仍为 pointer。普通 Tile 输出 EXE 可见链、最终交换链像素读回、物理多屏/跨机/热插拔/真实 DPI、本地 Art 触发、长稳及最终整包/用户全面手测仍开放。 隔离联合及普通工作区回归已过，完整证据与静态旧阻塞见 WINDOWS_NATIVE_BUILD 的 v508 记录，不等最终产品验收。

## v507 图片、Live、Art同一Tile动态联合

tile_mixed_joint_tests/tile_mixed_native_joint专属cfg(test)叶复用原真实Source系统菜单、签名配对/续签、Loom LAN HTTPS/WSS、Tile Runtime和唯一原生呈现。静态图片通过新资源/layout、Live通过原Source受预算纹理、Art通过原snapshot/资源分别换像素；独立手写PBGRA核三路首帧和变化后同一已paint帧，不以帧号增长冒充内容变化。冻结保实际Frame Arc并停Tile媒体但保Source，黑场/恢复沿原链；停止/撤权核view/像素/Shared/线程/媒体permit清理和源附件/权威值7保留。
本批相关Rust完整名去重70 passed/0 ignored，无终态失败，不是整库；26份input1实际定向回执经input2仅两专属新叶delta/测试体与生产未变/SHA精确复用。八条真实LAN联合在input2实跑：三类动态共同更新后停止8.688秒、变化后撤权9.032秒、三类冻结/黑场/恢复/停止16.563秒，分别为三个独立完整Source会话。另两旧Live、两旧Art及原点击/确认/取消联合重验通过。7份cfg(test) Rust最大421有效行，生产协议/权限/预算/依赖和后端选择不变。普通Tile输出EXE可见链/最终交换链读回、物理多屏/跨机/热插拔/真实DPI、完整Art值控件/IME/键盘、本地Art触发、长稳、其余迁移退出条件、最终完整包和用户全面手测仍开放。自有离屏NOACTIVATE HWND actual present不是上述验收。证据与复用边界见WINDOWS_NATIVE_BUILD，未改生产或重开三旧副线。

## v506 Tile Art 点击与显式确认

tile_surface_hit/click只将实际已paint帧的最上层interactive placement映射到canonical Art，保crop/四向旋转/clip与按钮down/up身份；共用原纯wire编码，不伪造普通Surface挂载资格。tile_surface_actions独占32槽/8MiB动作和64个120秒确认锚点，UI只排意图，原Art唯一串行worker先GET新鲜state再核view/generation/节点/命中/期限，至多一次Device/nonce POST，不重放未知写。
原生左键点击声明按钮/点击动作；右键菜单显式审阅批准/拒绝或取消已运行可取消请求，不自动弹窗或自动批准。模态返回复核完整提示、来源、已呈现布局及当前授权；短暂识别/冻结/黑场只减权，未发Event/Decision丢弃、有界确认锚点保留，真实布局/view/generation换代/停止/撤权仍清。当前只广告pointer，不广告值输入/键盘。
新真实Loom LAN→原生管理器配对/独立自有离屏NOACTIVATE HWND真实WM_LBUTTONDOWN/UP→原executor/真实framework与Art进程联合23.390秒通过，沿原30秒父门：None不拒绝、待确认经真实识别/同view恢复后显式Reject不执行，Approve后权威/正式结果及实际paint文本101，慢执行后显式Cancel且晚结果不改变101，执行记录恰fs，view/控制/线程回收且保源附件。选择回调复用生产choose，不冒称实际MessageBox点击；不是普通可见双产品、最终交换链读回或物理扫描。
相关Rust完整名去重175 passed / 0终态failed / 1 ignored，不是整库；25组定向在input9实际通过后，input10仅两份Loom专属联合夹具/外置handler变化，按完整delta、测试体/生产未变及产物SHA复用原回执，不伪称新库整套重跑。11条Tile真实LAN联合和普通固定SHA EXE工作区5项均在input10实际运行。完整Art值控件/IME/键盘、本地Art触发、真实Loom图片/Live/Art三类动态共同联合、普通Tile输出EXE可见链/最终交换链读回、物理多屏/跨机/热插拔、资源长稳、其余迁移退出条件、最终整包和用户全面手测仍开放。集中证据见WINDOWS_NATIVE_BUILD，三旧副线/固定包不改。

v505 Tile物理输出识别：真实输出枚举提供display name/canIdentify，独立临时Marker不依赖layout或图片/Live/Art资源。原Presenter唯一控制线程串行消费单槽applied/dismissed，沿精确Device/nonce/HTTPS报告；本地期限从状态GET开始保守计，重复观察不延期，迟到旧请求/lease/paint不能复活。识别及恢复围栏中不回报旧layout applied，正常呈现等待新鲜控制/心跳；原Art工作保留而非借识别取消已接受业务。
原Tile HWND/RenderSurface内DirectWrite绘制名称/身份/尺寸/倒计时及关闭按钮，只有可见且实际提交才报告；关闭/Escape绑定实际显示Arc，不让旧画面取消未绘制successor，DPI/显示变化/撤权/停止沿旧owner清理。无新线程、timer、像素面、依赖、权限或预算。input3相关166 Rust通过/1 ignored，含独立WIC像素、隐藏paint/重入负向、两新真实Loom LAN联合及旧八Tile/普通EXE工作区回归。Loom只新增既有identify/report精确LAN白名单，管理identify仍不开放。完整输入/失败/旧静态阻塞见WINDOWS_NATIVE_BUILD；Art输入、普通Tile输出EXE/最终交换链、物理多屏跨机、长稳/最终包未关闭，以下旧条目保留历史边界。

v504 Tile 慢 Art 隔离：每个在途输出 job 只在存在 Art 时惰性建立一个串行 Art worker，持同 Grant 独立第三 Lease/COM/Views，仍受原16租约和像素总预算；无额外采样/请求队列。图片/Live 继续在原 Assets 线程，Art 单槽稳定 Prepared Arc 到同一完整画布。发布核准备身份，换版先撤旧准备，迟到 paint 不得确认旧 Art；退休先撤准备再关闭 view，冻结 wait 不跨 Shared 借用。保持原HTTP8秒/Live5秒/旧帧1秒期限，不靠放宽限制或重连掩盖阻塞。
input2相关147 Rust通过/1 ignored；真实TLS/WSS夹具挂住Art图片超过5秒期间原Live连接仍有至少两次Ping，图片/Live/Art资源齐备后原生实际提交；停止/撤权可取消在途，迟到不发布，原八Tile LAN/两Source-Viewer/普通EXE工作区通过。此处三类慢网用可控TLS peer，不冒称真实Loom三类动态共同联合；完整场景等待资源期间仍清旧图，不承诺连续显示。17 Rust最大449有效行，精确输入/复用/耗时和Loom旧阻塞见WINDOWS_NATIVE_BUILD。普通Tile输出EXE/最终交换链读回、物理多屏跨机/热插拔、Art输入/识别、长稳和最终包仍开放。

v503 Tile 只读 Art：精确 Wall view 身份与单调版本沿原 Device/nonce/Lease，声明式 snapshot 和哈希图片组成不可变 Arc；后台只传纯数据，UI 独占 Presentation/COM 并按稳定 Arc 缓存布局。复用原 Assets 唯一线程、混合16M像素预算及唯一画布，Art 与既有图片/Live 共用 crop、四向旋转、clip 和 z 序，不执行终端 JS、不创建 WebView/子 HWND。最多4来源/16 placements/512累计节点/16唯一图片，控件只读绘制声明值，输入能力仍空。未知 open 不重放，冻结保实际 paint，迟到 state 丢弃/可信迟到新 view 关闭，新 snapshot 先清旧帧，撤权/停止回收 view 但保源实例。
最终 input6 相关696项 Rust 去重通过/7 ignored，含两条 Art 真实 LAN 联合17.203/8.484秒及原六 Tile、Source/Viewer、实际 Art Application 和普通 EXE 工作区回归，不是整库或最终包。实际原生提交与独立源像素不冒充最终交换链读回。公共 Client 新 Surface 响应门及 Transport 暂态映射真缺口已窄修并重验，失败均保留；完整身份/耗时/Loom旧静态阻塞见 WINDOWS_NATIVE_BUILD。当前广告 image/raw_bgra/surface_v1/scheduledPresentation；单张 Art HTTP 最多8秒仍可使同线程 Live 超过原5秒期限，未放宽限制或宣称慢网平滑。Art 输入/确认/取消、识别、三类动态共同联合、普通 Tile 输出 EXE、物理多屏/跨机/热插拔、长稳和最终包仍开放；以下旧条目只保留各自输入边界。

v502 Tile Live：原Wall媒体以精确LAN WSS路由/loom.wall.media.v1/NLWM80接入原Assets线程，不使用普通Viewer成员、控制权限或NLLV64解码器。TCP/TLS握手和受预算BGRA转换下沉为原链共用接缝；raw与PNG沿原像素/WIC/512MiB池，有界四流/三pending加一current及完整画布发布。公共时钟含RTT、年龄漂移与舍入保护；不可变Frame的1000ms期限由窗口独立核验，新候选不能替旧paint续期。Shared保实际paint快照，冻结保该帧并停媒体，黑场/恢复/停止/撤权与旧代际清理共用原owner，无新线程/依赖/权限。
真实Source菜单→Loom生产库LAN→图片/Live混合Tile14.453秒和撤权5.094秒联合通过：独立固定PBGRA、raw实际新解码帧/窗口提交、PNG首帧、冻结停Tile媒体而保Source、黑场/恢复/停止及permit/Weak像素/线程归零；不是最终交换链读回或PNG连续更新验收。192项Rust去重通过/1前台门ignored，含普通Source/Viewer输入释放与H264两联合、原四项静态/输出恢复联合和普通EXE工作区5项。当前image/raw_bgra/scheduledPresentation，Art/输入/识别Tile、普通Tile EXE输出、物理跨机/多屏同步、长稳和最终包仍开放；输入、失败及Loom旧静态阻塞见WINDOWS_NATIVE_BUILD。

v501 Tile输出生命周期恢复：独立Output owner只保一个active和一个retiring，输出缺失/枚举失败/同ID几何或DPI变化先隐藏并销毁旧HWND、停止旧Runtime；实际线程排空并复核原Grant后才建立新代际。不同输出不接管，停止/用户关闭/撤权不复活，不隐式配对或续签。原生短状态窗在缺席/排空时保留停止入口；复用原500ms物理枚举、授权/网络/像素预算与产品呈现设施，无新增线程、hook、依赖或权限。同步paint期间display/DPI变化后不能重新显示旧窗或回挂renderer。
136项相关Rust去重通过/1前台门ignored；真实Loom生产库LAN联合6.078秒核尺寸重注册、缺席offline、同ID恢复新lease和资源回收，恢复后撤权联合9.016秒核终态不复活。旧静态呈现/冻结/黑场/恢复/停止与撤权联合均重验通过。枚举快照仅测试注入，不改系统显示配置，不等物理拔插、普通Tile EXE可见输出或最终交换链像素读回；完整Tile/多屏同步/长稳/最终包仍开放。完整输入、首轮Clippy修复和Loom静态旧债见WINDOWS_NATIVE_BUILD；以下旧条目保留当时边界。

v500 真实静态 Tile 联合：tile_manager 只抽出共用的菜单授权消费点；新增 cfg(test) 联合叶驱动首次配对/批准/签名续签，关闭管理进程后由独立输出测试进程沿原 authorize/Runtime 自行注册、租约、取图和心跳。Loom 生产库通过原 LAN HTTPS 提供真实服务，管理员不代发终端请求；实际 D3D12 提交、服务场景应用/冻结/黑场/恢复、停止与撤权后 offline/像素租约/线程回收均有隔离证据。
相关108项Rust去重通过/1前台退出门ignored；源图手工PBGRA oracle、native present和服务端回执分别记录，不冒充最终交换链像素或物理扫描。5份Rust最大379有效行，Loom本批仅测试/登记，不新增生产API/权限/预算；完整输入/失败/复用及既有静态阻塞见 WINDOWS_NATIVE_BUILD。普通 Tile EXE 输出启动、可见双产品/跨机/多屏同步、自动热插拔、完整Tile、长稳及最终包仍开放；以下旧批次只保留当时边界。

v499 原生 Tile 图片终端：独立 CLI/原生菜单管理器与输出进程，不初始化普通截图工作区、托盘或全局输入。沿原设备授权、固定 HTTPS 路由和单串行 presenter 完成 CAS 注册/租约/心跳；图片 owner 独立且有界，复用共享像素预算、WIC/隔离 WebP。单槽帧按 endpoint/lease/wall/revision/clock 身份接纳，原生 D3D12→D3D11→CPU 提交之后才签发 appliedRevision；场景保守时钟、冻结、黑场、六秒/原授权期限及迟到结果均受同一 owner 约束。
管理器在消息泵之后按稳定owner复核关闭，不消费销毁后的输出启动意图；窗口同步paint重入关闭后不再显隐旧HWND。相关174项Rust去重通过/2 ignored；真实Loom静态Tile联合、自动热插拔恢复与配套普通daemon包仍未验，完整输入/失败/复用边界见WINDOWS_NATIVE_BUILD。只广告静态图片与调度能力；可见 Live/Surface 失败关闭，不用空白帧假确认。输出丢失/物理几何改变安全清屏并退出，当前需显式重新启动绑定，热插拔自动恢复仍开放。Loom 仅为图片 Tile 提供精确 LAN 接缝，不开放管理员布局/上传/输入。定向及产品验证边界见 WINDOWS_NATIVE_BUILD，不代表物理多屏同步、普通双产品/跨机、完整 Tile、长稳或最终包。

v498：Source控制GET短暂失联在原串行线程保可靠cursor、有限四次400/800/1600/2000ms退避；先实际释放旧键鼠并suspend，再由原媒体owner受控重连，让服务可靠记录controller=None。旧输入验证/消费但不执行，恢复不自动再获取控制，不新增结果可能不明的撤权写。媒体/控制/观察恢复状态独立，任一未恢复均非Ready；单重连意图在socket尚未安装时也不丢失。正常终态、授权/来源/epoch与原资源/线程/Lease/关闭边界保持。
Source观察冻结Submission只允许一次结果不明的POST，其后仅认证GET核完整Source/几何/能力和唯一完整payload；旧actor水位不证明原请求将来不会提交。仅发送前Connect可重试未发请求；原20秒候选期限/3秒HTTP/有界退避、取消与Tracks确认顺序不变，不换源/epoch、不重新采样覆盖未知候选。真实控制断线联合2.562秒及原UIA观察5.469秒通过，191 Rust去重通过/9 ignored、42 Application；观察丢回复故障本身由确定性TLS矩阵验证，不冒称在真实Loom中制造过该故障。详见WINDOWS_NATIVE_BUILD；普通可见双产品、真桌面WGC/跨机、跨epoch/重新授权、完整LiveRelay/Tile、长稳和最终包仍开放。

v497：Source媒体异常断链在同Grant/来源/epoch、原Consumer/Publication/媒体线程内有限重建WSS；新nonce、原Lease验证，不再次create/续签或重放结果不明的AU。frame_id失败也消费，静态源沿唯一已有GPU Arc发新raw/独立IDR；失败codec跨重连仍sticky。四次退避400/800/1600/2000ms，恢复确认只读认证快照，核完整来源/几何、sourceConnected及本次lastFrameId；握手和本地send不算恢复，不从服务快照采用actor或新epoch。确认期间有界GET，不增加每帧常驻轮询、捕获线程或资源预算。
媒体故障generation交给原串行输入owner真实释放键鼠并suspend；旧事件仍验证/消费，可靠controller=None后才接受新的显式获取，不自动复活旧控制。原生菜单显示“Live 重连中…”并保停止，恢复不替换捕获身份。正常Close、授权/协议/TLS/容量错误仍终态；Source控制GET/观察写入恢复及跨epoch/重新授权仍未实现。
本批相关177项Rust去重通过、7 ignored，含41 Application及普通EXE工作区5项。真实Loom LAN H264 Source断流联合10.797秒核原捕获/窗口/epoch、服务独立帧3/IDR与像素；持键鼠断流2.109秒核实际释放和显式再获，Viewer媒体/控制及旧输入联合回归通过。共用Loom测试启动屏障修复后重验受影响三门，未变Otter输入/三产物按SHA复用；失败/静态旧债及未验边界详见WINDOWS_NATIVE_BUILD。普通可见双产品/真桌面WGC/物理跨机/长稳/最终包与其余迁移仍开放。

v496：Viewer控制事件GET或事件触发的注册快照GET遇明确暂态传输/408/429/500/502/503/504时，保原Grant/Owner/Permit/可靠cursor/媒体，在原串行线程内最多四次退避400/800/1600/2000ms。认证、解析和应用完整成功才结束恢复；事件已提交后仅重试未完成的注册刷新，不重复消费、不用新快照跳过事件。每请求新nonce，不重新join/续签，不跨epoch或重放结果不明的写请求。
媒体与控制恢复状态独立，任一未恢复都禁输入并保“重连中”；损失清待发/减权/保释放屏障，恢复不自动重获控制。待发trigger意图被拒绝，Unknown不伪确认。停止/撤权复用Lease唤醒，不增线程/timer/队列/权限/依赖或预算。证书/协议/历史reset/错误epoch/超限仍终态；16KiB HTTP满缓冲明确容量错误，不把零长read当暂态EOF。
同源153项Rust去重通过/7 ignored，含41项Application；真实Loom LAN控制断流联合6.265秒核原cursor4→5、同HWND/epoch/actor序号/媒体连接与独立颜色oracle；媒体恢复、键鼠释放和普通EXE工作区回归通过。input1容量分类失败及input2修复/重建证据保留，Loom库按未变输入/SHA复用；静态旧债与精确边界见WINDOWS_NATIVE_BUILD。Source发布/跨epoch恢复、本地Art触发、完整LiveRelay/Tile、旧Web实际互通、普通可见双产品/真桌面WGC/跨机/长稳/最终包仍开放，以下旧条目保留各自输入边界。

v495：Viewer媒体TCP异常断开或liveness失效时，在原控制owner/同Grant、同媒体身份、同epoch和原Permit内有限重建WSS；每次重新核授权绑定/到期/撤销，使用新nonce，不自动加入或续签。保原ViewerSession、窗口、codec参考链及预算，复用原网络线程park/unpark退避400/800/1600/2000ms；一个无新像素进度周期最多四次，握手或旧帧不重置。真正提交新像素才退出恢复状态，静态旧raw不冒充恢复进度。
媒体损失清空未发键鼠并尝试释放控制，恢复后不自动重获；迟到acquire ACK及旧会话不能复活输入。只容忍新连接初始已知水位的重复raw/JPEG预热，出现新帧后仍严格拒绝回退；H264沿原decoder参考链，sticky fallback在重连时只offer legacy。正常Close、撤权、TLS/upgrade/profile/帧/协议/worker错误仍终态。没有新增线程、timer、队列、权限、依赖或预算，控制HTTP错误仍失败关闭，Source发布与跨epoch恢复不在此批范围。
同授权TLS/nonce/停止/预算、真实WS重复raw及输入减权定向通过；真实Loom LAN联合4.437秒核同窗口、epoch1、精确H264帧2→3、独立像素与未自动重获控制，后续关键帧/raw回退/停止归零继续通过。相关124项Rust去重通过，含41项Application及普通EXE工作区5项；不是普通可见双产品、人工菜单导航、真桌面WGC或物理跨机验收。当前源码/失败/静态旧债详见WINDOWS_NATIVE_BUILD；控制HTTP/Source/跨epoch、本地Art触发、完整LiveRelay/Tile、旧Web实际互通、历史前台/长稳及最终包仍开放。

v494：Live Source复用唯一捕获Consumer和媒体线程，惰性资格选择系统同步baseline H264或raw；encoder实际建立成功才广告H264能力，真实WS协商后再接受Loom video_policy。只有至少一个Viewer且全部支持H264才启用视频；需求变化/新关键帧序号及每30帧要求IDR。旧Viewer或显式fallback触发raw；codec/alpha/预算错误在本连接sticky raw，不自动重建失败codec。epoch不匹配或序号回退关闭连接，不伪装恢复。
BGRA直接写BT709 limited NV12 sample，单输入/输出且精确核timestamp；临时MFT buffer与最多1MiB Annex-B AU独立预算，AU校验SPS/PPS/IDR与尺寸后才发送。系统MF内部堆和调用耗时不被伪称硬实时受限。原GPU缓存只保一个Arc，静止画面可响应IDR请求；无新捕获/网络/输入线程、hook、timer、权限或依赖，原Lease/取消/分片/close仍由Source owner掌管。
实际系统codec的64×64/680×430/1920×1080往返、关键帧/回退/取消/预算，以及真实Source菜单→Loom LAN→Viewer解码/上传的策略、请求、旧Viewer回退/恢复/显式fallback及资源回收联合通过。wire codec和呈现RawBgra按精确帧号分别核验，独立颜色oracle不调用新renderer。当前108项Rust去重通过，41项Application实际通过；普通EXE工作区5项通过。此门不等于人工菜单导航、普通可见双产品、真桌面WGC、物理跨机、长稳或最终包；详细复用与未通过门见WINDOWS_NATIVE_BUILD，以下旧条目保留当时边界。

v493：已授权真实Surface宿主的Viewer系统菜单可显式新增/编辑/禁用高级JSON触发条件。配置固定本次Surface/attachment，最多16KiB，operand沿原4KiB/16层/512节点门；单pending/inflight槽，停止撤销待发与迟到发布。LocalArt没有此触发合同，不伪装Surface。复用原串行控制线程/Lease/唯一Sender actor游标、菜单和模态资源owner，无新线程、hook、timer、依赖或输入框架。
配置先释放旧键鼠控制。前置GET核当前身份/成员/本人双游标和恰好下一revision；只POST一次，随后以同源GET核本人control恰好next、input不变及唯一完整注册的authorizer/condition/target/enabled，才确认并推进同一发送游标。明确拒绝可继续观看，超时/5xx或不匹配无法证明未提交时关闭旧观看，不盲重放。GET只更新注册，不覆盖可靠观察/审计或推进可靠cursor；Viewer不自行求值/执行Art。
隔离真实Source UIA→Viewer生产配置接缝→Loom LAN HTTPS/WSS→原Surface executor→真实runtime-host/Art进程联合13.390秒通过，独立核最终eventAck/requestId、权威price/附件快照文本/正式结果=101、恰一次执行、禁用后新心跳不再执行及连接归零。不是runner注入、人工菜单导航、普通双EXE、Surface最终绘制像素或物理跨机验收；Fired仍不等业务完成，禁用不撤回已reserved动作。
相关113项Rust通过、2项未运行；测试修复、复用边界、普通EXE工作区及静态旧债见WINDOWS_NATIVE_BUILD。本地Art触发合同、H264 Source、重连/epoch、完整LiveRelay/Tile、旧Web实际互通、历史前台、资源长稳、可见双产品/跨端和最终整包仍开放；三旧副线和v464固定包保持冻结。以下旧版本只保留各自输入边界。

v492：live_source_uia/sample只在受限MTA child读取显式窗口；process复用bounded_worker_process与overlapped Pipe，128MiB/child、四份512MiB池、单在途扫描2秒、256节点/16层/512KiB响应，密码与跨PID子树排除，不执行Pattern动作。live_source_observation拥有两样本稳定/1秒心跳/缺失Stale与失败Error、256项/512KiB累计快照，仅真实ack或精确只读对账后commit；runtime独占同Grant独立Lease/串行观察线程和child回收。
原生Live系统菜单新增显式“发布 Live 并共享窗口观察”。普通发布和允许控制不默认读UIA，观察不授输入能力；Region/裁切窗口拒绝整窗观察，避免读取选区以外内容。通过原Device/nonce/HTTPS观察路由发布，不新增依赖、权限、全局hook或第二截图/网络框架。
原Source媒体owner在停止后join观察/回收child，再以原媒体Lease对账actor并close；不再固定close=2，观察ID另有序号。迟到扫描不新发、迟到ack不commit；已经到达服务端的请求不能撤回，close前按同锁member快照核source/device/hook/epoch、cursor与完整唯一payload，不盲重放。2秒是扫描pipe期限，不是包含驱动/内核清理在内的硬实时保证。
当前同源相关Rust完整名去重71 passed / 0 failed / 3 ignored，Source及Viewer观察19项Application逐例独立进程。自有离屏NOACTIVATE标准EDIT到受限child的真实值变化、密码排除、取消/Job/容量回收通过；状态/HTTP覆盖稳定、心跳、locator变化、失败/缺失、丢ack精确对账。实际Source菜单→真实Loom生产库LAN HTTPS观察路由的stable seven→eight/密码排除/close联合4.67秒通过；原Source输入/释放0.982秒及Viewer→Source菜单输入/释放4.076秒回归通过。普通EXE工作区5 passed/1前台退出ignored，2.809秒，覆盖启动/Checkpoint/中断重开/末Unit删除不复活及SHA准入。
本批关闭Source显式UIA观察采集/发布/停止的定向与隔离LAN联合，不关闭实际Viewer可见观察窗口/触发配置与执行全链、H264 Source、重连/epoch恢复、完整LiveRelay/Tile、旧Web实际互通、历史前台、资源长稳、普通可见双产品/物理跨端、最终整包及用户全面手测。三旧副线/v464固定包/真实profile与设备不动，不提交/推送/发布/部署。输入、失败、复用及静态边界见WINDOWS_NATIVE_BUILD。


v491：原生远程观看系统菜单“查看观察与触发记录”现可显示观察状态/值/来源/置信度/时间、加入时触发注册快照及最近16项审计。reservation显示“派发中”，不伪报Surface动作完成；Viewer不重新计算条件或执行Art。只在用户显式打开时格式化，单值512字符/整窗128Ki UTF-16单位，超限明确标记，NUL/换行/bidi控制可见转义；敏感文本不写普通日志/磁盘。不新增线程、timer、hook、依赖、权限、网络端点或资源预算。
composition_relay_window_observation拥有显式系统菜单、Host已授权业务快照和面板移出借用后的接线；observation_window独占标准只读EDIT/自有HWND/捕获排除、选区/阅读位置及同步销毁；observation_text独占有界语义投影，不读取UIA或执行动作。Business以共享显示身份避免普通帧/输入/心跳重写文本，只有成功语义事务换代。Application专属observation_peer/tests复用原TLS/WSS/菜单夹具与独立常量oracle，Session/Viewers只加cfg(test)查询。
打开查看窗先释放原远端键鼠控制；关闭面板保留观看，撤权/关Viewer/退出销毁子控件和共享快照。更新不持Relay/Host的RefCell借用跨Win32同步调用；重入关闭不回填旧owner。相同语义身份不重写EDIT，语义更新保留选区/垂直阅读位置；失败只关闭查看窗并留下可重试入口，不将只读面板错误升级为文档或媒体失败。
当前同源相关Rust完整名去重85 passed / 0 failed / 1 ignored，20项Viewer Application逐例独立进程通过，其中新增两项Surface/本地Art观察快照→可靠增量→reservation/final替换→关闭重开→撤权，以及一项打开查看先释放控制。新真实EDIT/菜单/捕获排除/选区/最大容量/控制符/同步销毁回归通过；Application观察数据来自独立TLS/WSS常量peer，不是实际Loom UIA。另原真实Loom生产库LAN HTTPS/WSS→Viewer→Source执行/释放联合3.357秒通过，作为共同观看/控制链回归，不冒充新观察服务端或触发执行全链。普通EXE工作区5 passed/1前台退出ignored，核启动/Checkpoint/中断重开/末Unit删除不复活及SHA准入，不是普通可见双产品或物理跨机。
本批只关闭已授权Viewer的原生观察/注册快照/审计查看，不关闭Source UIA生产、触发配置/执行、H264 Source、重连/epoch恢复、完整LiveRelay/Tile、旧Web实际互通、历史前台根因、资源长稳、普通可见双产品/物理跨端、最终整合包和用户全面手测。三旧副线与v464固定包保持冻结，不触真实profile/设备，不提交/推送/发布/部署。详细输入/失败及静态边界见WINDOWS_NATIVE_BUILD。
## 当前目标架构：2026-10-28

v490：在线本地Art宿主加入Live后，可由既有原生菜单显式获取/释放控制并发送窗口输入；Preview只证明精确宿主，不自动授权、不为JS增加输入/网络API。沿独立loom.art.local.live-control.v1与精确Device-only POST端点，仍核签名、绑定、在线期限、当前审批/工具/Device、已有Viewer成员、Source能力以及原controller排他、epoch和双序号。
attach与control复用同一授权边界；acquire/续租重核immutable源码，input不逐键重读源码。只有release减权可在Art撤审批/过期/停止后继续，必须是当前Device签名身份和自己已有controller；不能借此重新获取或输入。Ready后的观看寿命独立，宿主退休后禁新输入/再获取并尝试release。原四槽、队列16、预算/Lease、串行线程和退出owner不变。
17项原生Application菜单/TLS/WSS/自有离屏窗口用例及真实Art LAN HTTPS控制/撤权/release联合通过；后者明确未测试媒体或Source执行，另跑原真实Surface/Source联合证明执行与按键/鼠标释放无回归。相关122项通过（当前组合80项实跑、42项未变原生夹具按input2/产物SHA复用）、10项未运行，protocol组17项在input2通过后字节未变；不是普通可见双EXE、真桌面WGC或物理跨机。Loom既有101条daemon/10条protocol Clippy无新增，不称全仓静态通过。观察/trigger、H264 Source、重连/epoch恢复、完整LiveRelay/Tile、历史前台根因、长稳和最终整包仍开放。以下旧条目保留当时输入边界。

v489：真实Surface公共attach/active→原生Viewer目录/加入/获取控制权→真实Loom LAN HTTPS control/input→可靠事件→Source原Session唯一输入worker的联合已闭合。Loom LAN补齐精确Surface attach、实例lifecycle及设备stream；保持Device/nonce、当前审批/附件身份与生命周期围栏，16KiB请求门，不暴露Surface管理、任意事件、资源或WebView。
独立协议/像素oracle核80字节NLLV、2x2 BGRA精确[17,83,211,255]；两轮A/B键与鼠标按下经原worker到自有离屏窗口，菜单释放和WM_KILLFOCUS均在Source仍发布时实收KeyUp/MouseUp并回收worker，继续观看。最终actor9/input4、Surface Mounted1→Active2→Inactive3、媒体连接0，Editor文档戳不变。原桥资格保留至发布停止，不与输入worker寿命混淆。
真实生产库服务与两个lib-test Application联合3.886秒通过，不是普通可见双EXE、真桌面WGC或物理跨机。相关365项Rust完整名去重通过：21项本批实跑、344项未变回执按完整输入/配置/产物SHA复用，24项显式门未运行；不是新库整库重跑。Otter严格Clippy与两仓所需构建/严格行数通过；Loom仍101条原Clippy诊断、精确签名无新增，不称全仓通过。
本地Art观看proof仍不授控制。旧Web/Tauri经代码确认也是vk:NN/modifiers0，不应再重复实现DOM转换；旧Web实际产品互通、本地Art控制/观察/trigger、H264 Source、恢复、完整LiveRelay/Tile、普通可见/跨端、资源长稳与最终整包仍开放。以下旧版本的缺口说明保留其当时边界，当前证据见WINDOWS_NATIVE_BUILD。

v488接通真实Surface发起的原生viewer系统菜单“获取远程控制权／释放远程控制权”→原串行Control线程→同Lease的HTTPS control/input与actor双序号→可靠事件/媒体继续观看。只在本实例显式获取成功后转发当前已呈现内容框中的Win32事件，不把同device已有Web控制权自动接管。最多16项输入，连续move合并，非法/不支持/溢出撤销控制；失焦、菜单、尺寸变化及关闭清队列，取消后立即重新启用仍须先过旧控制释放屏障。HTTP接受不代表Source已执行，不自动重发未知结果。
同一线程有控制意图时使用50ms有界事件等待和10ms空批退让，普通只读仍750ms；30秒控制租约每15秒续期，不增加输入线程、Lease、等待句柄、hook、依赖或持久配置。窗口回调无网络/join，不移动鼠标、不抢焦点、不发送全局输入。远端控制撤销保观看，输入HTTP失败明确关闭并上报；本地Art证明只有观看权限，仍只读，不伪造Surface或放宽授权。
15项新增测试及相关348项Rust完整名去重通过（Otter库341/Loom2/固定SHA普通工作区5）、0失败、24 ignored。11项viewer Application逐用例独立运行，含获取/释放、失焦、关闭、远端撤权、HTTP500不重放和原六观看；独立TLS/WSS oracle核双序号/按键/坐标，自有离屏NOACTIVATE窗口消息不冒充物理键鼠。14项Source Application及两真实Loom Source LAN联合直接回归通过，但后者不覆盖新viewer控制公共HTTP/UI与Source完整联合。Editor文档戳不变有断言，不扩称canonical逐字节不变。
严格Otter Clippy/所需构建、格式及行数通过；input4仅修测试夹具，已过未变结果与普通产品按输入delta/产物SHA复用，失败保留。Loom本批源码/配置不变，不重编或重跑旧静态门，既有101条Clippy诊断不能称全仓通过。本地Art控制合同、Web DOM键码互通、真实Loom viewer输入联合、观察/trigger、H264 Source、重连/epoch恢复、普通可见双产品/物理跨机、完整LiveRelay/Tile、长稳及最终包仍开放；集中证据见WINDOWS_NATIVE_BUILD，固定v464包未覆盖本批。

v487为窗口来源增加独立的“发布 Live 并允许远程控制”系统菜单，普通发布保持只读、Region不授输入。Application消费时复核工作区/Unit/稳定来源、Device授权和容量，Source仅广告原窗口消息能力；控制轮询与媒体使用同Grant的独立Lease/Cancel。可靠事件整批预检后经过单在途桥和原消息泵，交由Session唯一输入worker执行，真实回复或释放完成后才推进游标，不以排队成功冒充执行成功。
远程占用时禁用本地Live输入；controller切换/cancel必须先停止、排空并释放旧worker。Bridge持原StopSignal，退出不依赖UI继续取请求；配置早退/panic、撤权、工作区换代、源关闭和空闲Bridge撤销均停止原Source。仍用原四槽、编码/像素预算和退出等待设施，无新增等待句柄、全局输入hook、SendInput、鼠标移动或焦点切换。
相关341项Rust完整名去重通过（Otter库334/Loom2/固定SHA普通Otter工作区5）、0失败、24 ignored；14项Source Application与6项原Surface/Art观看逐用例独立进程通过。两真实Loom LAN联合分别1.055/1.404秒，原媒体worker核80字节NLLV/2x2 BGRA精确[17,83,211,255]，输入联合经HTTPS事件到自有离屏NOACTIVATE窗口，实收A键和鼠标按下/释放，停止后Source关闭/连接归0且保本地采集。控制侧用原Loom store构造membership/controller/input，不是viewer公共HTTP/UI验收；捕获仍为合成WARP纹理，不是真桌面WGC、普通可见双产品或物理跨机。
Otter严格Clippy/所需构建通过，未变结果及产物按完整输入delta/SHA复用；Loom严格Clippy仍101条旧诊断无新增，不称全仓静态通过。当前键码仅vk:NN，text/focus、X1/X2、非零modifiers、同次双轴wheel不支持；原生viewer获取/释放控制权及输入UI、Web键码互通、观察/trigger、H264 Source、重连/epoch恢复、完整LiveRelay/Tile、长稳和最终整包仍开放。详细证据见WINDOWS_NATIVE_BUILD，旧固定手测包不覆盖本批；以下旧版本条目保留历史边界。

v486接通普通Live系统菜单的显式发布/停止：Application复核当前工作区/Unit/稳定Arc来源、有效Device Lease和等待容量，后台沿原HTTPS创建与WSS Source角色复用同一捕获Consumer和GPU读回。只广告raw_bgra/srgb、空interaction/observation，不复制WGC或授远程输入；固定16KiB分片复用原TLS/64KiB写缓冲，完整帧共用250ms I/O期限，网络缓冲从原进程共享编码预算预留512KiB，像素仍沿原Pixels/staging预算。
Source共享原远程/Control四槽；停止未join仍占来源身份，旧stop Arc不能停止新owner。显式停止先停止发送并中断socket，保原Lease以尝试有界HTTP close；Confirmed/Unconfirmed/AuthorizationEnded/NotCreated分开，不自动重连或恢复发布授权。撤权、工作区换代、源关闭及退出沿原owner回收；停止后的Ready event从等待集合移除，退出只等真实线程，避免常亮信号忙转或扩张原等待数组。
本批相关270项Rust完整名去重通过（Otter库264/Loom1/固定SHA普通Otter工作区5）、0失败、9 ignored；7项新Source Application及6项既有Surface/Art观看逐用例独立进程通过。真正配对的Loom LAN HTTPS/WSS联合1.760秒，接收原媒体worker独立核80字节NLLV/2x2 BGRA精确[17,83,211,255]，原菜单停止后closed且media_connections=0，本地采集保留至退出。输入是合成WARP纹理，不是用户桌面WGC或普通双产品可见流程。严格Otter Clippy/所需构建通过，Loom仍101条原诊断无新增；详细输入/失败/边界见WINDOWS_NATIVE_BUILD。远程输入/观察、H264 Source、重连/epoch恢复、物理跨端、完整LiveRelay/Tile及最终包继续后续，旧固定手测包不覆盖。

v485将真实Loom LAN的既有Live HTTPS控制与WSS接入原媒体owner。精确路由/参数、Host、Device/nonce、成员与原预算边界不放宽；已验TLS stream直接移交原grant/permit/worker与视频协商，升级连接不占四短HTTP槽。LAN Manager经OnceLock/Weak绑定runtime，停用/重配由原stop与有界IO回收，避免registry/runtime强引用循环或锁内join。编码session路径只解码一次，合法斜杠身份不丢失。
联合发现并修复Otter Device媒体旧deviceId query冲突：认证头/nonce是Device身份唯一入口，本地非Device兼容query及内部Prepared身份保留。真实配对、目录→加入→控制/媒体→原生D2D/GPU精确帧、source+五viewer不饿死HTTP、nonce409/撤权1008及LAN关闭/重启回收通过；原Surface/Art六Application观看及普通固定SHA EXE工作区回归通过。相关Rust405项去重通过、16 ignored，不称整库；Loom strict Clippy仍101条旧诊断无新增。源码/产物输入、失败和复用边界见WINDOWS_NATIVE_BUILD。
当前关闭的是隔离真实服务/原生消费者联合，非普通Windows source捕获/发布/输入控制、普通可见双产品/物理跨机或H264 LAN完整矩阵。完整LiveRelay/Tile、历史前台/长稳、最终包及用户手测继续后续，旧固定手测包不覆盖；以下v484等“LAN未开放”属于历史输入，不改写原回执。

v484为在线本地Art接通独立原生观看宿主合同。只有已验签Prepared、显式Preview权限、当前ready窗口及原工作区/输入身份能提供私有稳定Arc证明；离线恢复不提供联网资格。原生菜单选择后，Application重新检查目录、宿主、当前Device和新Lease；ViewerHost显式区分Surface附件与LocalArt证明，不伪造Surface ID，也不给Art脚本新增网络、控制或输入API。
Loom以Device专属32KiB的local live-viewer attach复核签名/完整绑定/期限、当前审批、immutable包与源码摘要、工具启用和Device guard；最终审批与加入串行，旧Surface viewers端点仍保持真实附件要求。两路径共享原发现/actor游标/响应校验、Session四槽、单份编码预算、Pixels解码预算及控制/媒体/窗口和退出泵。Ready前宿主失效取消迟到加入，Ready后观看独立于发起Art/工作区，显式停止/原观看授权结束才回收。
UI宿主资格只读观察授权，不推进worker共享时钟，避免后台请求先取样后取锁时被误判回退。相关440项Rust去重通过（Otter340/Loom95/普通EXE工作区5），19项显式门未运行；另protocol14项通过。新增本地Art菜单到隔离TLS/WSS及原生D2D/GPU精确帧/回收通过，真实Loom HTTPS加入/成员/撤审批父子联合通过；后者明确不测媒体。Loom strict Clippy仍原101条诊断无新增，不称全仓静态通过。
当前LAN未开放完整Live目录/控制及媒体WSS升级；本批加入合同联合与合成peer观看链不能拼成真实Loom全产品端到端。Source发布/控制、完整LiveRelay/Tile、普通可见双产品/物理跨机、历史前台根因、长稳和最终整合包仍开放；本批集中证据与工具失败边界见WINDOWS_NATIVE_BUILD，旧固定手测包不覆盖。

v483为真实已挂载且确认active的Surface接入原目录→原授权请求队列→Application观看准入→原Session/控制/媒体→原生Viewer HWND；不修改Loom协议、不伪造附件，也不把Binding::local_art当作服务端Surface。选择快照只含身份，Application消费时重核目录、实际附件、设备、新Lease和原等待容量；最多四个包含未回收停止owner的槽位，重复origin/session拒绝，停止key绑定同一Arc owner。
Ready前发起Surface失效取消加入，迟到Ready不建窗；Ready后观看不随面板/工作区结束，显式停止、授权清除或原租约终止才关窗并沿原退出泵回收。编码媒体预算进程共享一份MAX_FRAME_BYTES+HEADER_LEN，解码沿原全局Pixels预算，无新轮询器/线程框架或依赖。只读consumer接受原Owner已事务应用的已知状态/观察/审计，不执行source输入或再次登记trigger，未知业务失败关闭。
专属Application验证复用合成认证Surface及真实隔离HTTPS/WSS、原窗口/D2D/GPU读回，不用屏幕截图代替帧像素；集中成绩与未验边界见WINDOWS_NATIVE_BUILD。这不关闭新本地Art观看附件合同、Source发布/控制、完整LiveRelay、普通可见双端、真实Loom联合/跨机器或最终整包门。

v482以两个真实Loom持久服务和两个原生Application测试消费者补齐协作联合证据：原菜单发送/接纳、真实NOACTIVATE帧、冻结未烘焙标注的basis、切two_way、两端原调度自动Apply、默认Checkpoint后ack及同profile重开；切one_way后receiver本地冲突保留，显式采用远端并重开，source停止后保最后图像。
父门独立核两端服务权威和revision/对象坐标，32x32纯蓝basis不含红标注，三份重开/终态PNG逐像素核固定红框/蓝底；被动本PID前台事件监测覆盖receiver接纳到终态，不要求外部用户前台恒定。联合17.469秒通过，本批相关29项Rust完整名去重通过、0失败，1前台退出门未运行；严格Otter Clippy和两仓所需构建通过，Loom仍101条原诊断无新增。
该批只新增cfg(test)验证和最小登记，不改变生产发送/保存/授权/PNG业务；Otter ingress为pinned HTTPS，peer仍signed loopback HTTP，不是普通双产品可见流程、全链TLS、物理跨机器或自动服务预算阈值实跑。普通可见双端、LiveRelay/Tile、长稳/历史前台及最终整合包/用户手测仍开放；证据与失败边界见WINDOWS_NATIVE_BUILD。

v481模式与自动维护复用原协作Job/四Slot、750ms读取调度、Editor和默认Checkpoint/ack。source原生菜单设置one_way/two_way，receiver只显示已确认模式；Mode/服务Checkpoint只观察权威对象，不把尚未发送的本地修改当已确认。成功source probe或receiver read释放原任务后才申请单批最多32项的Apply，source在receipt224或240对象且含墓碑时先请求原服务Checkpoint，不新增第三轮询器、线程或配额。
自动准入最多保64项工作区/Unit暂停记录，操作失败或取消保持暂停，只有真实ack才解除；重开仍由磁盘pending阻断自动重放。显式Submit可恢复原Apply/Mode/Checkpoint，新的Mode或Automatic不能替换未知请求。明确可恢复拒绝先持久标记，再只读对账并经精确默认Checkpoint后退休，未知错误不得取得此资格。receiver one_way、真实对象冲突或suspended不自动发写。
最终input2相关596项Rust去重通过（库591、普通EXE工作区5），含13项新边界；5项显式真实Loom/前台门未运行。原菜单/隔离TLS/默认Checkpoint/真实重开验证shared/offline自动提交、Mode丢回复原请求重试及期间编辑保留；两处测试同步失败保留并修正，不为测试改变产品退出或两阶段保存合同。input2仅两测试叶变化，584项未变库结果与普通产品按完整delta/SHA复用；严格静态及所需构建通过，22份Rust最大468有效行。收口见WINDOWS_NATIVE_BUILD；不冒充普通可见双端、物理跨机器、资源长稳或最终整包完成。

v480源端协作复用原sender/Editor/PNG/Job与默认Checkpoint。显式入口先冻结不含活动矢量且普通opacity为1的basis PNG，栅格标注/crop/flip/erase/外观仍沿原renderer；原始initial、anchor、session和PNG先写原有界journal，再发网。仅明确404 projection_edit_not_found允许把旧flattened PNG换为basis；未知/不支持/拒绝响应不替换。后续目标复用同一冻结basis/session，活动blur/mosaic明确拒绝，不静默遗漏。
源端结果发布只核原Candidate两端allocation、索引、原意图与当前授权，允许尚未落默认Checkpoint的正常过渡，但不授ack资格；真实commit后必须回到durable sender包含凭证，再后台退休原请求。Source Ready接管原Active/Lease，旧Job析构不误撤销；普通metadata取消与四Slot回收规则不变。期间已有对象编辑保留，one_way源端可提交、receiver仍只读，重开不自动重发未知pending。
最终input6相关583项Rust完整名去重通过（库578、普通产品工作区5），含8项实际Application源端测试；5项显式真实Loom/前台门未运行，不计通过。严格静态/所需构建通过，52份Rust最大467有效行；失败及源码/产物身份分别保留，收口见WINDOWS_NATIVE_BUILD。这是隔离Application/TLS/保存重开验收，不冒充模式切换、自动编辑提交、普通可见双端、物理跨机器或最终包完成；后文旧阶段状态保留原历史边界。

v479冲突恢复沿原协作owner/Job/四Slot与默认Checkpoint。明确拒绝的原Apply先写持久标记，再只读对账并保留本地view/冲突，实际Checkpoint后才退休；未知网络结果和未识别错误仍保原请求，不能借read或用户选择消除。原生菜单提供保留本地/采用远端，后者只丢选择前的修改，read期间的新编辑仍保留；最新one_way不允许本地重提交。
最终input4相关363项Rust去重通过（库358、普通产品工作区5），含14项新边界；其中3项journal与普通产品按完整输入差异及SHA复用，生产未因负向夹具修复而变化。严格静态/构建与行数门通过，22份Rust最大466有效行；两项显式联合/前台门未运行。原失败保留，集中证据见WINDOWS_NATIVE_BUILD；这不是源端初始化、多目标、模式切换、普通可见双端/跨机器或完整协作验收。

v478为已有receiver协作会话接通原生提交菜单→原Job/四Slot写前Apply→当前Editor重读→精确默认Checkpoint→后台ack及有界续进；每批最多32项、每次显式入口最多八批，未发项和网络期间修改不误确认。失败/取消/撤权/底图或会话换代保留原请求，显式重试仍用原opId/基线/payload，重开不自动重放；不造第二网络或编辑框架。
session_projection_scope的有界协作包含身份绑定同一allocation，只有真实默认Checkpoint commit提升durable，HTTP成功或手动Save均不退休pending。手动Save只在精确候选/Receipt和实际磁盘commit后共同接管只读接收Index，避免original合法换代被误判为失效；取消或同字节不同allocation不得替代。edit与receiver显式恢复/自动收件/持续read准入互斥，共用原等待名额，不提高63句柄或四Slot预算。
最终input3相关469项Rust去重通过（库464/普通产品工作区5），含11项新模型/实际Application提交测试；两个显式联合/前台门未运行。原TLS/菜单/Job/Editor/Save/Checkpoint/重开链通过，不冒充普通可见双端、物理跨机器、模式/冲突菜单或最终包验收。源端冻结basis初始化/多目标继续后续；下文v477及更早的只读/无edit写入描述保留其历史边界，集中证据见WINDOWS_NATIVE_BUILD。

v477协作文档接收：原accept/read完整校验独立Document与冻结PNG basis，同PNG revision的metadata也推进标注。初始矢量与协作记录在原单贴纸候选/工作区Scope内构造，Base64移动一次并持原租约；持续结果复用原Job/Lease/Slot/调度/Guard，在原Editor内保留本地变化和冲突。basis初次冻结需要换图时，标注与底图在同一预检事务发布，undo不回旧像素。
首次接入的三种无新底图响应均核本地自然像素/尺寸与破坏性历史，不以远端合法PNG冒充本地basis证明；已有owner依弱像素/几何围栏，不逐轮hash。原有本地对象保留为新增；异宽高比basis与像素共用内容框，恢复后的留白/尺度通过原session映射接回View。元数据未变不造undo或无意义换代，过期/缺失/换session或basis不发布。
最终input3相关450项Rust去重通过（库445/普通产品工作区5），两项显式联合/前台门未运行；实际Application原TLS/Editor/Save/Checkpoint/重开、独立像素/几何oracle及迟到/撤权/本地保留通过。此批只读消费协作文档，不新增edit写入/自动ack，也不冒充完整双向/模式与冲突菜单或最终包；后文v476及更早的editing拒绝为当时历史状态，集中收口见WINDOWS_NATIVE_BUILD。

协作工作区记录独立保留完整Document/basis、当前可见basis对象、角色/路由/Context与冲突；它不授网络权限，也不会因普通工作区保存而确认未知pending。原Save/Checkpoint同一候选写入记录与当前图源/标注/编辑状态的流式摘要，恢复核scope/Unit/内容后只绑定既有Editor，不重放旧像素或制造undo。
Editor到canonical的视口平移/比例与View几何分开，保存时组合逆映射，避免把20DIP留白或工作区尺度当成图像尺寸。来源/几何改变或不能表示的本地标注持久标为suspended，普通保存继续；重开保留新图/crop与原协作权威，后续恢复必须显式解决。像素只保弱身份，元数据身份纳入原Save/Restore Stamp。
本批实现与验证边界见WINDOWS_NATIVE_BUILD；网络初始化/模式/冲突菜单和完整双向链尚未接通，原editing拒绝保持，不将工作区恢复记录冒充Loom授权或联合验收。

v476相关276项完整名去重通过，含10项新记录/实际Application保存恢复测试及5项普通EXE工作区门；两个显式联合/前台门未运行。元数据移除使旧ready候选失效、坏canonical/取消恢复不换活动源、crop变化显式暂停而保图、普通保存不退休未知网络pending均实际核验。显式null保持字段存在性并拒绝，输入冻结后只更新文档；不是网络产品入口、物理跨端或最终包通过。

协作标注适配复用既有Editor与仅标注事务；不可变View保留原wire/f64/缺省字段/稳定ID与层级，expected只描述原生表示，回读仅覆盖实际变化的字段。本文所述原始wire保真属于View桥，不代表普通canonical会保留全部协议词法或可选字段；Application网络/模式/冲突与精确checkpoint确认仍待接，editing拒绝保持。
恢复/变换域独立于普通控件域，样式、文字、完整角度、小数序号半径和层级元数据沿原历史/保存；实际呈现和命中共用零宽/透明/零尺寸判定，多边形上限64。文字原文保留CRLF/NUL，无法无损表达NUL的EDIT拒绝打开，不截断提交；平台字体只在PCWSTR适配处选有效前缀或系统fallback，不反写模型。本批验证见WINDOWS_NATIVE_BUILD。
Bridge与canonical共用有界OrderPlan；未新增时保留最大合法层级，删除不改其他层级，真实重排或新增溢出才按Editor顺序整体重编号，不以GUID字典序决定新增层级。v475相关489项去重通过（含18项新增及普通EXE工作区5），两项显式门未运行；真实D2D/WIC/DirectWrite和Application保存/Checkpoint/重开通过，普通canonical不冒充View协作权威。仅测试oracle修复后的未变结果按manifest/cfg边界/SHA复用，失败保留；不是完整协作/跨端/最终包验收。

Editor仅标注替换使用带文档身份/提交版本的一次性候选，历史仅交换矢量与serial，保住原底图/crop/完整逻辑框和工具样式。来源像素/历史不被候选持有，过期、异文档、草稿或预算失败不部分发布。
多点line/arrow的恢复、编辑/history及canonical保存沿原Editor；末有效段箭头前缀/头由arrow_path_geometry只读借用，D2D/PNG、精确命中、组边界与窗口区域复用，不再各自猜首末直线。单点line/arrow无可见平头路径，polyline原有dab不变。
这不开放网络双向协作入口；完整文字合法域、模式/冲突菜单与真实checkpoint接线仍待后续，原editing拒绝保持。本批验证状态见WINDOWS_NATIVE_BUILD。
v474相关408项完整名去重通过（含18项新增与普通EXE工作区5项），前台菜单门1项未运行；真实Application保存/Checkpoint/重开核稳定ID/全部路径点及底图不烘焙。新测试修正普通视口留白oracle后完整补验，生产/未改模块按明确delta与SHA复用，失败保留；不是完整双向/跨端或最终包通过。

协作视图以不可变basis对象为真值，本地逻辑尺寸只派生wire锚点；非等比几何按轴、线宽/圆角/字体按正向平均倍率映射，逆向除以同一倍率。frame resize共享basis，不形成编辑或累积标量漂移。
projection_edit_view/mapping保留合法wire域、原ID/zIndex及空白文本，拒绝非协议字段；缺省字体只在local物化，不使原wire临界对象超限。view_merge分离权威响应与当前本地overlay，对象冲突不静默重基，异步保存后重读与真实checkpoint仍由Application负责。
该基础层不代替Editor最终呈现/模式菜单/工作区接线，不签发落盘回执；v473定向与隔离journal/TLS及普通产品工作区相关54项去重通过，23项新增包含大倍率舍入和临界16KiB原对象，两项显式门未运行，既有editing拒绝保留。

原生标注独立颜色alpha与整体opacity分层：Stroke/StrokeStyle持有有界stroke/fill alpha，原history/snapshot/adopt及canonical读写保留，不把颜色alpha折进整体opacity。
单色路径/箭头先合并为一层避免交叉重复加深，形状与序号先组合独立fill/stroke后应用整体opacity，荧光笔既有35%wash和效果工具合同不变；屏幕、软件呈现与PNG仍复用原D2D owner。
该基础能力不等于完整协作Editor适配；协议几何/文字范围及Application/模式/冲突/checkpoint接线仍需完成，editing拒绝guard保留。
v472相关444项去重测试及普通EXE工作区通过，含13项新增模型/像素/保存恢复边界；5项显式门未运行，长旧canonical纯色与极小alpha仍可重开，不扩大协议或Surface预算。

原生projection_edit以独立v1文档/六操作适配Loom权威，严格核session/basis/版本围栏与标注，不将PNG和已烘焙标注重复导入。
profile日志在网络前持久保存原请求、固定路由、已确认文档和本地view，完整磁盘CAS/单在途/取消围栏复用既有owner；冲突/丢回复保留，不自动rebase或换opId。
Control只交付已核响应，不自动确认canonical保存；Application/渲染/模式与冲突菜单尚待接，既有editing拒绝guard保持。
专属双真实Loom门只证明原生事务、HTTPS ingress及authority一致，测试显式ack不冒充Application保存或可见双向产品验收。
v471完整父门4.875秒通过，相关227项Rust去重通过；原请求重开重试、跨路由合并、checkpoint/墓碑及冲突本地保留均实际核验。原生产品已构建，但协作编辑产品入口/渲染/冲突决策仍未开放。

普通产品离线联合测试由local_art_runtime/offline_joint沿原菜单/UIA/自有launcher执行两个固定SHA的otter-native.exe；专属profile/一次性合成身份和独立条带oracle不进入生产实现。
Loom专属product_joint父门拥有双真实库服务和接收端PNG/digest审计；v470普通双产品实际菜单首帧/reset/flip、同profile重开、停止/终态再重开与正常退出已通过，三份PNG独立核全像素/尺寸/digest。
12项去重定向/集成含未变Art/Job的明确输入复用；canonical source定位跟随已保存矩形与坐标空间，不改产品恢复。普通产品流程、生产库服务、普通daemon业务EXE及物理跨机分开记证：本批peer仍signed loopback HTTP，普通Loom业务EXE/全链TLS/跨机和最终整包未验。

双真实Loom联合门复用原生发送/接收Application和真实服务；LAN HTTPS adapter显式允许现有offline device/signed peer端点，不赋管理员权限或放宽认证/预算。
接收菜单、保存/真实NOACTIVATE新帧后的displayed、持续更新、同profile重开及终态保图分别核验；Otter ingress HTTPS与signed loopback peer互传分开记证，不冒充全链TLS、普通产品可见流程或物理跨设备。
v469上述两条双Loom停止/撤销门及原单Loom回归已通过，相关44项去重、五份PNG独立像素/尺寸/digest复核；四种peer HTTPS成功响应另独立验签。
source离线read仍只返回metadata，PNG oracle从授权receiver读取；catalog digest按原合同追加在完整challenge之后，不改变生产合同。普通可见产品/跨机与最终包继续单独验收。

OfflinePeer接收保留远端source签名envelope，将当前配对Loom的transport origin和peer单独固定于原写前意图与工作区binding。
共享inbox广告离线能力后，原Job顺序请求本机静态offline inbox；完整合并才清未查询标记，离线失败保共享，取消/撤权仍整体拒绝。
Accept、恢复read、持续read、displayed和补偿unlink复用原owner/限额/保存呈现围栏，不能靠网络响应注入URL或回落shared路径。
旧在线持久形状不变，新增离线路由由旧读者失败关闭，不删除字段降级；定向/普通产品与真实双Loom/跨设备验收分开记录。

原生发送侧以显式OfflinePeer路由复用原Store/Job/Lease/PNG/CAS/Checkpoint/停止链，当前配对Loom独占互信revision、epoch和对等转发。
peerId与remoteDeviceId来自固定目录并在原意图持久化；复合目标、兼容Web元数据和native marker相互核验，不把离线字段降格为SharedLoom。
所有网络操作只请求本机配对origin的offline-projections静态端点，原source签名保持不变；响应必须匹配peer/route，不能注入offlineTransport地址。
组发送/停止允许混合路由并保留每个目标的持久事实；原生接收侧离线路由、真实双端及最终包仍单独验收。

设备组停止由projection_sender_stop_group/flow在原stop单槽上逐项持久受理，最多八份既有Request，不建立第二套解绑实现。
原stop轮转处理已受理项的canonical pending、unlink回执、canonical移除及ack；网络失败保留持久恢复状态，不阻住其他成员减权。
组结果区分已确认/待确认/失败/未受理；取消仅放弃未受理成员，已持久意图不撤销。来源围栏只随原stop重新核验的patch推进，旧菜单或外部原文不能重新抓取为基准。
Checkpoint编码canonical的独立allocation只由真实commit回执登记，仍核原Receipt来源、Workspace和Index；网络成功不替代这个持久事实。
菜单范围是当前目录明确列出的成员ID；不可用计数不含身份，不能猜测其关联或声称全部远端设备都已停止。真实接收端呈现、跨设备和最终包仍单独验收。

设备组发送由Application的projection_sender_group/flow在原单在途发送链上串行推进，不创建批量网络框架。
组选择固定目录Arc/Unit，首次签名前统一核全部待发送成员的八项容量；每台的原canonical确认后才推进下一台。
只允许本批已确认元数据换代来源围栏；原Retry阻住剩余目标，取消/失效保留已保存关联并清除未发送计划。
组菜单显示逐目标结果，已存在关联不重发，离线广告仍不授发送能力；逐目标停止/持续更新继续由既有owner负责。
专属测试从实际菜单经Job到隔离TLS独立解码PNG，不将邀请/保存成功宣称为接收端已呈现；组级全部停止与真实跨端不在本闭环内。

固定原生候选的运行验证由tests/support/product固定显式EXE与SHA256，并保留只读文件到spawn后；workspace_runtime仍是原离屏工作区测试。
它不改Application、保存owner或产品CLI，不以编译时target的旧EXE替代待交付release。v464包的Checkpoint/中断重启/末Unit删除与正常退出已通过；前台菜单与完整smoke失败仍各自保留。

普通产品Art持续投送场景复用现有Application、公开SDK和原发送菜单；测试peer只接合成设备的有界TLS请求，
从接收PNG独立核对源分辨率及固定色块，区分预览、正式结果、持久停止和重开。
UIA子菜单按原ExpandCollapse合同路由，显式产品EXE必须核SHA256；projection_oracle独立构造几何覆盖像素。
input16普通EXE三条可见场景已完整通过：红图首发、预览不更新、同envelope绿图更新、持久停止、停止后蓝图checkpoint及跨进程恢复/正常退出，
另覆盖完整资源包服务/有限离线与服务图片结果重开。两份投送PNG经独立解码核像素/尺寸/digest；启动器由Computer Use真实激活，
菜单仍核真实GetForegroundWindow。原失败回执保留，不将此隔离TLS/合成身份验收写成真实Loom可见联合、物理输入或跨设备通过。

恢复阻断已定位到无变更自动库维护：首次呈现等模型变化使完整Receipt失配，旧领取顺序在识别no-op前误报并弹模态警告。
application_reference现先核worker取消/策略/候选，无变更不发布也不置脏；真正写库仍核原Receipt和退出围栏。
新增maintenance测试保护来源owner/取消/策略变化，普通产品无输入探针确认两个恢复窗口enabled/响应且无模态框，不增加线程或缓存。

正式结果不属于Editor版本；保存脏状态现在复用原OutputStamp，独立跟踪正式端口身份，预览/进度仍不触发持久化。
原Receipt继续拒绝过时或取消候选，不增加像素owner、后台队列或线程；清空正式端口重开不能复活旧结果。
对应Checkpoint/maintenance测试及直接依赖/公开SDK/UIA/startup共175项去重通过，另三条可见场景通过；input16源码集中构建，文档收尾不重复构建，不以此声称全库通过。

可见场景的原图不变断言发现原始PBGRA经WIC自动反预乘保存会产生通道漂移。
export_raw_alpha只为保存的png_checkpoint在原共享预算内的WIC副本上显式转换straight BGRA，普通png_raw与Shader straight入口不变，
无额外整图缓存、线程或权限；完整alpha/channel矩阵、普通图标四次WIC保存恢复及对应冻结输入的302项直接回归已通过。
这证明所验表示/取整合同，不泛化为任意图像编码无损或当前整个迁移验收通过；原失败回执保留。

持续发送真实Loom联合门沿原菜单与HTTPS/设备认证，投送body必须包含PNG的独立digest；
此前模拟peer漏校该生产必需字段，v463仅补齐客户端合同而不放宽服务端。新显式双仓测试分别拥有Application消费者、
隔离服务及独立接收PNG oracle，已显式验证接纳/crop更新/read/持久停止恢复与最终410。
相关163项去重通过，另普通Otter产品构建及两个worker通过；54份模拟TLS及实际接收PNG身份/尺寸/digest一致，
持续20份与真实接收16x8绿色PNG经独立解码逐像素复核。input1路径表示夹具失败保留，仅修夹具后重验。
Otter严格Clippy/lib通过，Loom仍101条既有诊断、无新增签名；此门不代表可见UI/跨设备或最终包验收。

已保存SharedLoom关联的持续发送沿原Sessions/Job/Lease/Active/四槽，按当前来源凭证生成源分辨率PNG，
先read同一ID的权威版本，再以priorRevision/revision执行CAS；回复丢失只读对账，不重放update或创建新邀请。
单在途和有界最旧到期调度合并快速编辑；原签名envelope及首帧写前日志不被更新覆盖。
更新与首发使用不同结果/取消语义，旧来源/工作区/授权的迟到结果不能写canonical，更不能补偿解绑稳定关联。
内容缓存只持原文/像素弱身份，不能延长预算租约；canonical保守匹配只影响缓存，不使首发Scope因纯元数据保存失效。
更新开始前核profile root，Updating时原停止菜单先持久减权，再沿原checkpoint/unlink/ack收尾，不增加易失意图队列。
本批290项去重相关回归、普通产品构建和两个既有worker通过；34份既有/20份持续TLS PNG核尺寸/metadata/digest，
新20份经独立PNG解码与固定色块/crop oracle核像素。严格Clippy/lib构建/格式/行数通过，测试夹具变更不改变生产DNS或期限。
原失败和receiver单组总期限中断保留，仅补无终态用例；同生产字节回归按输入/产物哈希复用，不宣称单进程全库通过。
隔离TLS、普通EXE可见业务及真实跨端分别验收；本批不证明接收端已呈现、Art到投送完整可见联合、长稳或最终包完成。

敏感服务确认新增独立显式协议与serviceWithConfirmation；请求询问不代表批准，JS不能交gesture/approve。
原Task/Lease/InvocationControl固定原期限、command/package/scope与输入指纹；可信原生宿主显示参数和实际上传图片缩略图，默认拒绝。
非模态窗口过程只记录选择和Wake，原Window重新核当前宿主后才决定；远端换代/撤权经同任务poll撤销提示，取消后不接纳迟到结果。
缩略图和菜单复用同一image_thumbnail像素/绘制设施，编码/解码仍计入原预算。旧服务与canonical/Save归属不变。
参数区Enter和自有Escape经独立提示翻译记录拒绝，创建不自动设焦点；不把多行Edit的Enter交给默认按钮猜测。
本批定向/隐藏HWND与五条实际双仓HTTPS联合通过，两仓普通产品构建及三个既有worker通过。
最新组合输入去重Otter57/runtime53/protocol12/daemon38；Loom仍有已留证旧静态诊断，不等于全仓检查、
可见用户确认、完整SDK、跨端或最终包通过。证据main-art-service-confirmation-1。

v460可见Art测试的foreground owner仅属于integration夹具，先取得自建启动器前台，
再向精确自启子PID授予许可；不向产品添加测试授权后门、不使用全局输入或放宽原焦点门。
service_image场景复用原菜单、完整签名包与独立PNG/checkpoint oracle；目前前台准入未通过。
库回归的总期限中断和原生Enter前台失败单列保留；tray_actions仅修测试的录制菜单oracle，生产路由不变。
这些结果不关闭敏感服务、普通可见产品/真实Loom联合、跨端、长稳或最终手测包门。

v460显式服务图像结果：独立image-output网络合同保留旧JSON门，PNG经原pipe二进制传递，
在原Canvas图片表原子追加后才生成不可伪造image句柄；原图、Editor及canonical不会被服务直接修改。
服务元数据仍受64KiB限制，图片单项2MiB/4096边长/16MiB解码；输入和结果合用原累计图片与32MiB Canvas预算。
Loom复用原任务表、Lease、敏感effects拒绝和cancel清理，终态图片存储有独立8MiB编码总上限；不创建另一资源框架。
正式结果/保存仍由原Application接纳。本批Otter294、protocol11、daemon43项去重定向通过，
包含新公开示例的隐藏Application/Save恢复和四条真实HTTPS联合；两仓产品构建、Otter三项既有worker回归及严格Clippy通过。
Loom严格Clippy仍101条既有诊断、无新增签名；这些证据不等于普通EXE图像输出可见UI、跨端或最终包完成。

v459新增显式serviceCanvases：JS只选当前Canvas，调用时同步flush，Rust沿原背板与PNG codec生成私有快照。
art_service_canvas逐图完整复核PNG/尺寸/聚合预算，再移动原编码到既有图片服务请求；不安装成跨操作资源或交JS任意字节。
后续绘制/resize与在途快照独立，CPU与提交数仍累计原operation预算；普通末帧复用已flush无变更的背板。
参数准备使用原子准入，失败不消耗服务序号，Proxy trap不能重入覆盖在途调用。
专属模型/child/TLS/取消、实际产品worker和真实Loom HTTPS联合通过；本批Otter287、protocol10/daemon38项去重通过，另三项产品worker。
不等于图片输出、完整SDK或可见产品验收；Loom严格Clippy仍有既有101诊断，无新增签名。

v458新增已安装不可变图片的服务输入：私有句柄→原Prepared/dispatcher→独立images请求→Loom原任务及资源租约。
原JSON服务、授权/取消/正式结果和Save owner不改义，不接受脚本任意图像字节、URL/路径或伪造gesture。
单图4MiB/合计8MiB、最多4图沿已有图片预算，后台编码副本另计费；图片不进入JS堆或控制请求。
本批Otter263、Loom protocol10/daemon37项去重定向运行通过，含实际双仓HTTPS图片输入/撤审批与原JSON回归；
两仓产品构建及Otter实际产品Canvas/状态worker两项通过。Otter严格Clippy通过，Loom严格Clippy旧诊断仍失败且未观察到新增。
这些证据不关闭Canvas服务输入、图片输出、敏感服务、可见产品或最终整包验收。

v457补充独立otter.art.v1作者入口，由Loom/sdk/otter-art维护真实原生API的类型与可直接分发示例；
旧loom.surface.v1、服务v1和正式结果合同不改义。Otter显式测试读取同份示例并核摘要，经原签名包/child/TLS/Application运行。
服务成功/暂态隔离、完整缓存有限离线、独立像素/正式值和Save沿既有owner验证；不引浏览器、Node、任意文件或网络能力。
本批不改变资源/期限门，不以类型声明补齐资源型/敏感服务合同或冒称可见产品/跨端/最终包验收。
同份示例已显式运行通过，200与503场景各自停服后新Application无Device Session恢复并提交offline结果；
82项去重相关测试、严格all-target Clippy/lib构建及无DOM严格类型/JS语法通过，不是全4553项lib验收。

v456服务暂态失败与本地执行权分离：连接/HTTP传输及429/502/503/504只拒绝当前Promise，
原Call被消费并至多一次取消，清理不超原调用/授权/父operation期限且最多一秒；不重放或新建许可，不切换离线。
清理前后重新核原Permit/Lease，认证/身份/服务wire/TLS信任失败仍终止实例；旧服务输出不能作为清理成功结果交付。
正式结果与保存仍由原Application owner负责；119项去重相关测试通过，另1项实际产品Canvas worker显式通过。
真实child/TLS和Application覆盖错误捕获后原实例/源图保留、显式新call、正式像素/42值与Save/Restore；
原失败保留，1项授权producer夹具ignored未运行。严格all-target Clippy/lib和产品构建通过，不代表资源型服务、
完整SDK、普通EXE可见菜单、真实Loom联合或最终包完成。

v455以同份资源包/TLS夹具验证完整Application链：原OpenArt/签名child收到交互队列后先发布37%服务进度，
终态才提交独立2x2正式图像与42值；原Save/Restore可重开同Art节点，无Device Session离线再执行并shutdown。
原截图像素存储保持不变；新增窗口访问器仅cfg(test)，生产权限、SDK预算及提交owner不变。
30项定向回归、严格all-target Clippy和lib/产品测试目标链接通过；普通EXE可见菜单的前台准入仍失败，
不重复相同前台条件刷绿、不把隐藏/屏外输入队列等同物理交互或最终产品验收；失败和旧测试进程现场保留。

v454资源包闭环：独立包证明绑定原grant及完整资源清单；art_prepare_package复用原后台链与取消，
art_package_content把已验证文本/二进制/PNG接原Content/image安装，art_package_cache分块复用原32项/32MiB缓存。
仅完整缓存才发布离线凭证；旧签名域、设备授权及原期限不改义。Otter668项相关passed/5默认ignored、Loom40项相关passed/5默认ignored；
另三项显式HTTPS联合门覆盖在线服务/撤权、单入口及完整资源包停服后不同Otter进程恢复；新资源门还验证缺块拒绝和独立2x2像素oracle。
实际双仓产品构建及Otter Canvas worker通过；未变主体回归按输入/产物哈希复用，只有显式ignored联合夹具重编重验。
Loom严格Clippy仍失败，诊断位于字节未变的旧文件；不是可见产品UI、完整SDK、跨端、长稳或最终手测整包验收。

离线恢复新增独立resume签名域，不改变旧grant绑定；精确源码/设备公钥/issuer与原offline期限均须复核。
art_offline_store负责有界恢复凭证、启动时钟高水位和减权，device_authorization_local只提供无网络Session的停止租约。
原Tasks/Runtime/Application与正式结果/Save仍是唯一owner，授权清除或切换先停止本地视图。
显式清除另沿原有界授权worker持久化origin cutoff，即刻退出不取消已接受的减权；普通Select不永久撤销离线许可。
身份锁冲突/损坏失败关闭，不冒充Missing；离线启动先写时钟高水位，同毫秒旧票也拒绝，不存Device token。
v453的Otter643项及Loom33项相关回归通过，两项显式HTTPS联合门分别覆盖在线服务/撤权和停服后的不同Otter进程恢复；
后者没有Device Session，服务返回offline后仍提交正式42结果。实际产品Canvas worker另通过，Loom严格Clippy仍失败。
这不是完整包资源离线、可见产品UI、真实跨端、长稳或最终产品包验收；期限不续期，也不提供可信硬件时钟。

本地Art持久源码缓存只优化字节传输：profile/art-source-cache按连接/设备/包版本摘要/SDK/独立issuer索引，
每次打开仍实时describe并获取绑定新workspace/instance/request的完整许可。显式prepare.v2仅在源码摘要命中时返回null，
未命中返回完整源码；旧v1不改义。磁盘最多32项/32MiB，单源码1MiB，同目录暂存额外最多1MiB，不驱逐用户内容。
缓存忙/损坏/不可写不提升权限，不存grant或token；该源码缓存自身不提供跨重启离线许可，有限恢复见上节。
本批230项Otter去重定向、12项隐藏Application、真实Loom HTTPS联合及显式产品worker通过；
Loom相关19项通过，严格Clippy仍有未改文件46/55项旧诊断，不作为完整SDK或最终包验收。

Art新增源图像到Canvas的受控安装合同：输入PNG沿原pipe独立有界传输，描述与内容digest复核，
JS只持当前会话的图片句柄。drawImage复用原软件D2D及正式图像结果链，不引入WebView或任意URL/文件读取。
解码输入/target位图共享原Canvas总预算，安装及mount共用原Session期限；小图内联合同保留，
大图不扩大JSON门。Application来源变化/取消/工作区退休仍拒绝旧结果。
此闭环严格静态门和279项去重定向测试已通过，含11项真实隐藏Application及2项显式产品worker门；不是最终产品包验收。

公共legacy区域统一表达为有界source/target Projection，Canvas/D2D与CPU效果不再生成巨大整图坐标；
原始纹理与裁剪历史保留，reset/undo和保存换基准不烘焙或丢失原图。有界采样及相关478项去重回归通过。

本地Art图输入接线：art_local_images复用既有图解析和后台输入准备，art_local_mounting复核后进入签名Runtime。
不可变Content携带端口描述/资源，不新增文件或网络能力；来源变化、取消、换工作区拒绝旧帧与正式结果。
内联资源仍受原512KiB门限制；较大图片使用独立有界二进制通道，不静默缩图。
前一批206项定向验证建立了图输入接线，本批补齐Canvas正式图像及保存恢复；既有真实Loom联合夹具本批未运行。
完整图像处理SDK的其余能力、完整包内资源缓存与最终包仍未完成；有限单入口授权恢复见上节。

最新接线：独立 Art 节点事务已由 application_local_art_node 实现；art_local_dependencies
限定执行时的上游拓扑，在结果领取前拒绝输入图变化，不因无关节点编辑失效。
拓扑及像素/正式输出凭证已接入原执行生命周期；完整图像处理SDK仍有未完范围。
投送宿主 snapshot 提供已验证 owner 的 legacy_pixels，专属投送渲染器消费该区域并保持逐轴自然密度；
不修改 canonical，不重复应用已物化 flip。PNG最终35项已接原发送链，当前组合输入完整sender50及34份隔离TLS
接收PNG的独立像素/实际尺寸/元数据/digest一致；真实跨端显示、最终包及用户全面手测仍未验证。

本地 Art 正式结果通道正在实现：独立版本化请求与单槽，不复用 emit 事件，也不改变预览含义。
正式输出目标必须是独立 Art 节点，再由现有 Results/Save/Checkpoint 交付与持久化，不能覆盖发起截图。
当前已接显式workspace_result许可、已验证Canvas像素、单槽领取和既有Art节点正式端口；一般截图目标仍拒绝。
结果替换沿原OutputStamp和宿主生命周期复核，保存仍由原Save/Checkpoint负责。Otter侧71项定向回归及严格Clippy通过，Loom合同7项、daemon真实HTTP权限与直接回归13项通过；创建独立Art目标与完整输入SDK仍未完成。

用户自行滚动长截图由 CAPTURE 专线复用既有 Session/单在途任务实现；MAIN 保留公共菜单、快捷键和结果提交。
录制来源用可选 otterNativeRecording 扩展保留帧数/方向/空间位置，未知轴不伪造为垂直；sourceRect 仍为原选区。
MAIN 共用提交先校验元数据再启动后台任务；来源与提交定向15项通过。CAPTURE input-3专属实现及MAIN菜单/Ctrl+3/Delivery公共入口已合主仓，含timer失败安全关闭修复；主线组合构建/严格Clippy和124项去重定向测试通过，含两个显式真实自建窗口门。物理滚轮与最终整包仍未验。

本地Art服务进度经原授权dispatcher→Runtime固定大小单槽→宿主现有状态交付，复用Wake，不新线程/事件队列；
只显示状态/有界百分比，不透传任意服务文本。进度不产生正式帧或工作区结果，停止/撤权/换代拒绝旧状态。
input-2已通过后台TLS/真实child/Runtime、宿主生命周期及严格静态门；可见进度联合验收和正式结果尚未完成。

HLSL产品适配已接默认原生安装：desktop 的单请求 Invocation 区分帧语义，原生分支调用已有受限 Auto worker，
使用独立 straight PNG/PBGRA 结果桥接；缓存计入新 source/abi 的占用。宿主强制 shader_abi，Loom样例显式返回HLSL；
隐藏Application与实际EXE取码/缓存/正式PNG运行门、最终静态/输入审计已通过，详见main-product-hlsl-1。
ANGLE生产入口退役批次已断开旧候选/worker，实际DLL加载和旧child通道限定测试目标；
生产库旧render返回Unsupported，原生ZIP拒绝libEGL/libGLESv2。产品边界、实际EXE原生重渲染、
Application与严格静态门通过；合成ZIP合同通过，最终实际包仍未验收。

Shader input-6原生实现已合入当前主线工作区并注册私有受限worker入口，主线集成验证单独记账。
新合同otter.hlsl.v1返回straight RGBA8，不能送入旧shader_image的反预乘链；公共显示/正式PNG、产品派发、
产品默认原生已由MAIN完成并另行验收；ANGLE总退役与最终包仍须MAIN继续验证。

本地 Art Runtime 区分宿主执行停止与认证撤销：原 Lease 内 execution_cancel 只中断执行，
原认证取消信号继续保护一次有界的同 owner 服务清理；认证撤销同步触发两个信号，不复制凭据或建立新租约。
执行停止后的 mount/交互/帧领取全部拒绝，后台完成清理后撤销原 Lease；不是允许停止后的服务重试或工作区写入。

child Canvas 背板改为复用现有线程绑定软件 DIB 的 D2D DC target；软件窗口呈现和 Canvas 共用一个叶资源实现，
不复制窗口/提交业务、不增加 GPU owner。DIB/DC 生命周期、精确拷贝和 GDIFlush 由 software_dib 维护，
Canvas backing 独占 target 和像素预算，原绘制语义/透明度/PNG/执行门不变。该变更正在集中验证，尚不作为完整产品验收。

本地 Art 服务客户端沿用原 Prepared 的 Permit/Lease，固定 origin 和独立 issuer 准入不变；
art_prepare::service 只负责后台串行协议交换，严格绑定 owner、序号、终态及原绝对期限。
服务回复不是正式工作区提交；signed Local 拒绝门保持，尚未接入真实授权 dispatcher。
QuickJS 私有服务 Promise 已接同 operation 的父子往返：脚本不持宿主身份或网络能力，父侧只调用显式 dispatcher。
等待发生在后台/JS 栈之外，排除等待时长但保留原 250ms 执行、布局/Canvas 次数及两秒传输验证预算；
单 operation 至多 32 次串行服务，总服务期限 60 秒，最终 Output 前不产生 Frame 或结算 UI ticket。
服务失败、取消、超时或畸形回复不交付部分帧；当前进行定向验证，不代表已完成 SDK/产品联合验收。

本地 Art 在后台执行 mount 前先建立隐藏禁输入 HWND，并将真实客户区/DPI 沿原签名准入传给 Session；
首帧通过视口/宿主复核后才进入 ready 和显示状态，不猜测 96 DPI，不在 UI 线程执行 JS。
Shader 原生替代由独立 shader_hlsl 专线实现并交付；公共呈现、设备恢复和最终产品接线仍归 MAIN 总集成。

进行中的 Application 接入批次将 describe 与 prepare 合并在同一后台任务，联网前即登记原 Registry 代际与信任租约。
描述请求只包含宿主上下文，不伪造尚未取得的 package 元数据；窗口挂载还需独立 signed Local 后台 owner，不能套用旧附件授权。
该后台 owner 与本地 HWND owner 的源码已接入模块：首次准入的单调/墙钟截止沿原 Permit 传递，空闲等待原 Lease、
child 终态及命令事件，不新建轮询循环。窗口先验证宿主再提交帧，Win32 调用后重验。
Application OpenArt 已接入 local owner，拥有工作区/实例 wire 身份、配置凭证及退休记录；首帧和终态复用 execution_wake。
旧窗口关闭在恢复所有权交换前执行；隐藏首帧可在之后首次显示，不重复覆盖用户隐藏意图。完整 Application 组合与产品验收仍未完成。

精确本地 Art 候选通过独立 local describe 合同取得，设备审批和 immutable 包验证仍由 Loom local_art 拥有；
Otter 以原连接/Lease 与当前宿主身份构造请求。描述不授予执行权，prepare 签名准入保持独立，旧 capability metadata 不提升权限。

本地 Art 的后台下载由 desktop/art_local_tasks 有界拥有，复用工作区 Registry 的请求代际和原设备 Lease，
配置/DNS/TLS/验签不在 UI 线程运行；线程完成或失败经原通知返回，撤销先于回收。目录到精确本地包请求的
生产合同和 Application 首帧接线已写入，但完整窗口输入/SDK、服务与结果链尚未完成，不沿用远端 Surface attach 冒充本地执行。

v440 普通工作区与钉图的 Canvas 兼容渲染统一为显式 D3D11 合成 owner，替代下述历史 HWND target。
D3D12 优先、D3D11 兼容与 CPU 兜底共享关联准入及退休规则；默认仅一个主呈现后端，不新增设备常驻链。
历史无 DComp 普通窗口的立即退休分支已移除；本批不代表 Art/Shader/ANGLE 或全部原生退出条件完成。

v439 手动自动重选重新进入 D3D12 优先候选链，兼容选择使用原兼容路径（钉图 D3D11，普通窗口 HWND target），
不是强制创建第二套 GPU。活动合成 owner 先退休，明确完成后才发布目的地；未知关联不解锁。
CPU→GPU 在绘制周期内移除 LAYERED 并保留 NOREDIRECTIONBITMAP，样式事务必须在新候选可能关联前提交，
创建 attached failure 后不能回滚 layered；明确未关联的创建失败另持一次最近失败凭证，可手动回 CPU，不自动循环重试。
原分层窗口还需在 GPU 首帧前临时隐藏以撤销残留内容；可见性 owner 使用 NOACTIVATE 隐藏/恢复，保留原隐藏/最小化状态，
恢复前重新检查宿主及托盘/分组意图。它不与新 GPU 关联共享样式 rollback，也不为了重选重建文档或长期运行双后端。

v438 手动 CPU 选择消费现有退休事务，活动 D3D12/D3D11 必须确认退休后才切换；CPU/传统 HWND
可在没有合成图的前提下重建。绘制重入、创建期关联拒绝、GPU 退休失败/超时仍禁止手动绕过。
菜单重试不清原错误，只有成功帧清除；选择只属于当前窗口，不改默认 D3D12 优先策略或写全局设置。
本批验证 CPU 编辑/撤销/重做、生产保存 worker、落盘、真实文件恢复和再呈现，不宣称全部软件功能验收。

v436 将运行中 D3D11 故障接入相同退休事务，确认后才允许 CPU 呈现；无 DComp 的 HWND target
设备错误可以直接退役后进入 CPU。初始钉图的 GPU 候选仅在未关联失败时允许软件兜底，关联不明仍拒绝。
CPU 钉图通过分层窗口首帧事务保留 NOREDIRECTIONBITMAP 与整体透明度；重入换文档/换 Target 或
销毁宿主会取消提交/确认，不能向复用 HWND 回滚。凭证仍不覆盖同文档的全部 viewport/展示瞬态。

v435 运行中恢复按完整窗口事务实现：Canvas 分离活动 Target 和退休 owner，后者跨最小化/换源保活，
WM_TIMER 有界轮询既有 DComp 退休，明确 Complete 后才进入 sticky compatibility；失败/超时及不可恢复
绘制错误保窗口/文档，不发 WM_CLOSE。关闭清理不等待 worker；DComp 完成与 GPU drain 仍独立，
不把此事务当作 CPU 钉图样式迁移、手动恢复或全部原生重构完成证明。

v433 将 Canvas 的 surface 创建和首个画刷准备纳入同一准入事务，完整 Target 准备完才发布。
已关联 surface 的画刷准备失败先退休，再交 Canvas 受阻状态保文档；不再绕过 v432 的跨重绘保护。
此保护仍不等于运行中所有资源绑定/绘制失败均已安全退休，自动恢复和 CPU 业务链仍待完成。

v432 创建关联准入跨两层选择器传至 Canvas；未确认关联错误持久化到窗口私有准入状态，
设备缓存丢弃不解除阻止。受阻重绘不再创建后端、不自动关窗；诊断和文档保留，但尚无手动解除/退休续接，
也不保证此失败状态下所有可见编辑入口可用。CPU 恢复及其他最终绘制错误仍须继续接线。

v431 创建失败携带显式候选准入：未尝试关联的准备失败可回退，已尝试关联的失败拒绝立即回退。
该凭证贯穿 D3D12→兼容和 D3D11 hardware→WARP 的创建选择，不把异步退休开始当作确认完成。
拒绝是保守终止当前创建事务，不是完整保文档终态或退休后续接；运行中恢复仍须由窗口 owner 协调。

v430 将 D3D12 创建关联移入完整资源 owner 的事务：关联失败先发起退休，原错误向上传递，
未确认时保留完整图和预算。构造返回错误仍不是 HWND 可以安全切换候选的凭证，准入协调继续开放。

v429 RenderSurface 提供统一 DComp 退休派发，窗口事务仍须持有 owner 并等待确认后切换；
无 DComp 图返回 None，不代表所有 GPU 工作已排空。默认 Canvas 尚未发起自动退休。

v428 D3D11 接入完整退休 owner，与 D3D12 共用 DComp worker 与窗口图预算；未确认时连同共享设备
租约整体保留，健康正常关闭仍释放。D3D11 构造先 bind 后提交关联，业务绘制在 BeginDraw 前检查退休状态。
默认 Canvas 尚不发起退休；未确认创建关联的候选协调与最终 CPU 恢复仍未完成。

v426 为 D3D12 owner 增加显式单向合成图退休：DComp-only worker 执行解绑和完成等待，
UI 只轮询，不等待或转移 D2D/GPU owner；Pending/Failed/TimedOut 与 Complete 分开。
未确认图连同预算保留；Complete 不是 GPU drain 完成。默认 Canvas 尚不发起该操作。

v425 将普通工作区 CPU 候选接入真实 Canvas/WM_PAINT：D3D12 失败后仍先试原 HWND target，后者创建
失败才分配软件后端；已有软件选择在资源重建后保持。软件 draw 仅离屏，独立帧在释放模型借用后提交，
成功且宿主/Target/已提交文档身份仍有效才增加正常文档确认计数。外部 display/缩略帧不计入该计数。
此阶段不含钉图 CPU、手动选择、GPU 运行中完整解绑恢复或软件失败后的保文档终态。

以 [统一修订计划](../development/WINDOWS_NATIVE_REVISED_PLAN.md) 为准：共享业务/绘制语义，
D3D12 优先、D3D11 兼容与按需辅助、CPU 兜底；允许明确用途的 D3D11On12，不默认双份渲染。
Art JS 在 Otter 所属 QuickJS 执行设施运行，Loom 保留包/授权/服务；旧 v1 语义须版本化升级。
以下为已实现切片的历史记录，不表示当前已有双后端、本地源码 Art 或 ANGLE 退役。

v416 为 D3D12 窗口候选增加合成视觉透明度及同设备资源创建入口。透明度不改变文档和 backbuffer
像素，帧起止仍归候选 draw；DComp 提交链失败后拒绝继续绘制，不能把旧数值缓存冒充属性回滚。
当前仍未默认接线，多窗口共享设备和运行中恢复是独立未完成项。

v417 已实现并定向验证多窗口共享设备：同一 UI 线程的 D3D12 候选共享设备/队列/11On12/D2D device，
窗口仍独占 context/交换链/合成树。Weak 缓存不延长健康设备寿命；未确认队列排空会毒化整代，
旧窗口不能继续绘制或提交，新代仍受最多两代设备预算约束，不无限替换或强制 RemoveDevice。
这只提供代际拒绝边界，未实现应用级重新创建窗口资源和自动降级。

v418 开始让 RenderSurface 默认优先创建 D3D12 后端，工作区/钉图共用 Canvas 绘制语义；
完整创建失败后回到各自原兼容链，并记录失败与最终选择。主链成功不预先创建第二套后端。
这是每窗创建期选择，尚非统一手动选择、故障后的自动降级、CPU 独立软件底线或 Shader 迁移完成。

v419 增加运行中单向降级：只有 D3D12 后端明确失效才退休 GPU Target 并锁定原兼容链，业务文档和
编辑历史不参与替换。WM_PAINT 安排降级后的单次重绘，不能因普通重绘/最小化来回切换后端。
兼容链也不可用时的最终保文档处理、手动选择、CPU 底线仍未完成，原致命错误关窗边界尚待替换。

v420 软件候选使用显式软件 D2D DC target 和 CPU DIB，借 GDI/UpdateLayeredWindow 提交，
不经本项目的 DXGI/DComp 呈现链；尚未参与默认后端选择。它复用既有 D2D 绘制语义，
仍要求 Windows Direct2D/GDI 可用，不是跨平台软件引擎或完整 CPU 产品链已经验收。

v421 为软件接线增加独立窗口分层事务，尚未启用自动回退。已有 NOREDIRECTIONBITMAP 实测不能原地
移除且失败调用可能部分改位，事务写前拒绝，GPU 钉图需要由窗口 owner 创建替代 HWND 并保留文档关联。
样式调用可能同步重入窗口过程，不得在 Canvas 可变借用内切换；成功软件帧后提交事务。
事务只拥有 LAYERED 及回滚，不声称替代 HWND、DComp/GPU 解绑已实现或 CPU 产品底线已通过。

v422 在 WM_NCCREATE 安装 State 指针前绑定精确 HWND，重复候选不能进入原 State 的消息/销毁清理。
未绑定的隐藏候选只有默认窗口行为，不登记宿主；生产消息与面板刷新按身份过滤。
此保护不提供所有权转移。v422 额外探针证明本机保留 NOREDIRECTIONBITMAP 并增加 LAYERED 后可
成功 UpdateLayeredWindow，纠正此前“必须替代 HWND”的推断。优先验证旧 DComp 解绑后的同 HWND
软件呈现与屏幕结果；若确需替代，才实施发布/任务迁移/退休事务。当前组件仍保持原保守拒绝，未启用该路径。

v423 正在验证同 HWND 的 D3D12→软件纵向切换。软件分层候选允许保留 NOREDIRECTIONBITMAP，
但调用者仍负责解绑旧合成树及无跨重入借用；普通 BitBlt 不获得该许可。显式可见测试不等于
运行中自动恢复或生产有界等待已经实现，默认 Canvas 选择保持不变。

v413 默认窗口创建边界：`composition_selection` 只在完整呈现候选成功后返回，当前
依次尝试 D3D11 hardware/WARP，各一次；设备创建或呈现链失败均可进入下一候选。
`composition_device` 按 UI 线程分别弱缓存两类设备，`composition` 继续拥有每窗交换链、
D2D 与 DirectComposition 资源。显式 Live 设备不参加此回退；运行中设备代际恢复尚未接入。

v414 的 `composition_d3d12` 为独立窗口候选，尚未被默认选择：D3D12 device/queue/swapchain
拥有呈现，11On12 在同一队列上承载共享 D2D 绘制。device 子模块拥有 fence/event，buffers 子模块
拥有双缓冲包装视图和帧 RAII，lifetime 子模块限制候选数量及未确认关闭的完整资源保留预算。
这不是无适配的纯 D3D12 Shader 路径，也不代表原生双后端或完整故障恢复已经完成。

v415 把默认 Canvas 四条绘制分支收敛到 RenderSurface 的闭包式帧事务，局部 clip/transform 和
BeginDraw/EndDraw 由 render_frame 的栈上 guard 管理；绘制失败不执行钉图的显式 Present。
保持 Target 私有缓存和绘制计数语义，默认后端未切换；D3D12 候选接线仍须补齐共享 owner/透明度/选择与恢复。

## v411：稳定发送关联显式停止

稳定绑定停止独立于发送中 Cancel：固定 Binding/canonical allocation 后，原 sender Store 先持久减权，
原 canonical owner 保存 stopPending，再由原 Job/Lease/当前原设备授权执行 unlink。
已确认的原 selection/envelope/PNG 以终态回执留在原有界日志，canonical 移除关联后才后台精确 ack；
重启不凭“没有 Request”认定成功，不重新签名，不借新 origin/device，也不新增 sidecar 或网络 owner。
metadata patch/日志写入在单槽后台，安装前核 Workspace、canonical Source 和 Record allocation；手动 Save 不授 canonical proof。
HTTP 成功到本地回执提交前仍可能中断：保留 pending，404 不冒充成功；旧程序读取新增 stop 字段将失败关闭。
本轮同源定向 lib 门通过，startup 正式集成仍有一项原 15 秒超时，未关闭完整迁移与历史 startup/呈现失败；见 [v411 构建记录](../development/WINDOWS_NATIVE_BUILD.md#v411稳定发送关联显式停止)。

## v410：投送目标编辑与外观归属

发送快照现在将 resolved 权威底图与目标 canvas.editor 分开捕获；上游 editor 只可参与底图选择，
不能把上游标注或 crop 误当发送目标参数。目标当前 editor 继续优先于 canonical 中的旧 crop/reset。
export_projection_target 在原后台 Job 中按目标 UnitId 读取 canonical 的 borderWidth/borderColor/cornerRadius/beautify，
大图片和历史字段保持 RawValue 借用；外观复用原合成器，beautify 后的最终宽高再进入发送尺寸准入与签名。
目标没有 logical frame/raster 时，adapter 使用 canonical 目标展开框（含缩略态 savedRect）作逻辑尺寸回退。
原 Source/Weak/display/editor 新鲜度、Job/Lease/四槽及唯一 Save/Checkpoint owner 保持不变。
完整历史 erase/flip、传播内容框和 PNG parity 未闭合；证据见 [v410 构建记录](../development/WINDOWS_NATIVE_BUILD.md#v410投送目标编辑与外观归属)。下方 v409 为历史。

## v409：本地保留原图生产捕获

application_projection_sender_snapshot 在原 UI 捕获边界选择显式权威像素：匹配活动 editor 的 retained source，
或当前 layers.base/宿主 fallback；恢复原图校验先于生成新 identity 的 export_snapshot。
后台复用原合成与发送 Job，不重选底图、不重建活动工作区。Scope 的新增 Weak 像素凭证补足原图单独替换的新鲜度围栏，
不延长解码图像及预算 lease 的强引用寿命，原 display/document/workspace/opacity 围栏继续生效。
resolved frame 仍保留旧 owner 语义，不与目标 retained 混用；完整目标外观 metadata/上游传播接线仍开放。
输入与验证边界见 [v409 构建记录](../development/WINDOWS_NATIVE_BUILD.md#v409本地保留原图生产捕获)。下方 v408 为历史。

## v408：有界源分辨率编码出口

投送和工作流共用 source-resolution 合成 owner；新增 export_source_png 只分离最终准入/编码资源，
不增加另一份渲染器、来源恢复、持久化或网络 owner。发送模式先核源图/裁剪内框/美化外框尺寸，
后经共享预算编码并检查实际 4 MiB；普通 workflow 不继承投送专用限制。
出口消费 ComposedImage，保持编码完成后先归还合成输出预算、再复制 PNG 的既有顺序；发送编码预算不随位图过早释放。
生产入口仍从当前 editor.layers.base 选择权威底图，不从旧宿主图或未经身份验证的 retained source 回退。
retained natural source、目标与上游 metadata 分离及完整 beautify/历史编辑接线仍开放。
Live worker 的退休生命周期与保存成员集合分离：旧 owner 允许保留待后台退出，但已离图 Unit 不进入新快照；
automatic_ready/capture/receipt 共用当前图成员规则，仍在图内的异常关闭 owner 不被静默忽略。
实际身份与验证见 [v408 构建记录](../development/WINDOWS_NATIVE_BUILD.md#v408有界源分辨率编码出口)。下方 v407 为历史。

## v407：单目标发送生产接线

新增单目标发送 Application owner：固定 Unit/Directory→canonical scope 准入→原 Job 后台 PNG/原意图→
read/Create/reconcile→后台 Binding candidate→Application metadata→原 canonical checkpoint 精确包含。
网络 owner、四槽、Lease、Save/Checkpoint 和 outbox 均复用既有边界；不引入旁路持久化或第二套投送协议。
网络失败保原 Request，显式重试不重新签名/编码；结果被过滤也必须先交回取消责任。
Record allocation 负责 stale/rollback，canonical source 负责 admission/Binding；手动磁盘提交只防误解绑，不授 canonical 成功证明。
worker 返回前执行完整 outbox 基线核验；UI join 仅租约/内存交付，后续完整验证及网络操作仍受原基线/授权约束。
后台持久取消与网络清理有界退避，精确退休发布事实处理晚取消；普通退出/换工作区不解绑稳定已保存关联。

当前显示 PNG 不等 source-resolution/beautify parity；持续发送、设备组、稳定 binding 显式停止、真实双端、资源长稳及完整迁移仍开放。
验证身份与边界见 [v407 构建记录](../development/WINDOWS_NATIVE_BUILD.md#v407单目标发送生产接线)。下方 v406 为历史。

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

application_projection_sync 是唯一 application_projection_receive 的 child；
schedule 只拥有限64公平/退避时间，publish只在原围栏复核后交换Source/Index/Canvas。
projection_receive_sync、projection_sync_http/read/patch 归原投送Job，不另建网络/身份/
工作区/存储owner。projection_binding_source 拥有持久原始像素证明；editor_source 归原
editor_raster，主editor文件不增长；editor_source/
edit_history_source 只拥有兼容共享底图的原子历史重基，不复制或清除用户历史。
同步测试按wire、editor、schedule、Application生命周期与真实profile重开分责；
复用原CA/身份/窗口夹具。旧实现与N01–N12边界继续保留，不合并其他独立仓库业务。

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

application_projection_receive_recovery 是唯一 receive owner 的 child；
projection_receive_recovery 只做原 runtime Job 的只读核验，不是第二网络 owner。
application_projection_checkpoint_proof 归 canonical checkpoint，保存目标显示与原始
source 的共享身份及原 Document guard，不复制 PNG/像素或另建持久存储。菜单只排显式
请求，原 Sessions/授权/profile/checkpoint/呈现/cleanup 生命周期不另起 owner。
新测试按 TLS/menu/profile 重开、worker 负向和 checkpoint 边界分责复用原夹具。

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

application_projection_receive_lifecycle 是原 receive owner 的 child，
只拥有状态终止/稳定化接线，不是第二 owner。Ready/Hold 保不可复制活动许可与 fresh；
原 save worker 生成同源完整 Index，checkpoint/manual 发布者只在真实 commit 后通知。
新 lifecycle/persistence tests 分责复用原 TLS/合成身份/隐藏 HWND/磁盘事务/restore pipeline。
补齐原 Ready 移交后的 fresh 生命周期：原网络 Job 原子返回的新建资格随
Ready/Hold 移动；拒收、准备错误/取消、发布/保存等待失败移交精确无许可原票据。
普通 Preparing stop 仍只请求取消，回收后才交原 cleanup owner；终态先 join 再排 marker。
没有第二网络、身份、journal 或工作区 owner，不删除已经追加的本地图像。

默认 checkpoint 和手动保存均在真实 disk.commit 成功后、后续 UI poll 或错误返回前，
用实际 candidate 的完整 Binding Index 和 Saved 捕获的 Workspace IdentityStamp
同步清 fresh。预备索引、失败 commit、异工作区和完整索引不包含 expected 均不能冒充。
若其他旧/坏标记使完整 Index 未知，普通保存仍允许；同工作区真实 commit 后仅放弃
自动补偿，保留显式恢复，不新增 durable/可见呈现/displayed 权限。Reporting 已有真实
保存事实，取消或回执失败不自动解绑。复用旧意图仍保显式恢复，不借本次失败越权减权。

新回归覆盖 Ready 拒收/Scope/profile/复用、成功准备后取消、隐藏追加后取消/撤权/退出、
真实认证解绑前持久 marker、manual commit 与晚选目标、失败 commit/索引错配、坏索引
降级及真实工作区交换。无真实 Loom、任意位置 panic/突然断电或资源长稳验收；displayed
恢复重试、read/持续同步、发送端/策略/离线互信/双向 journal/Tile 和完整 N01–N12 仍缺。

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

原 Accept Job 的 Attempt 保精确新建票据到 join，Sessions 单项交接供
Application 原 cleanup owner 持久减权。成功 Ready 与 abandoned 互斥；复用旧 intent
不获自动解绑资格。正常关闭/整体切换不解绑已保存稳定关联；没有第二网络或工作区 owner。

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

唯一 Application 清理 owner 从原 checkpoint 取得已保存 Unit 的删除资格，先持久标记再复用原认证解绑。Opening 期间保完整 Binding，最后 Unit/终态先收回原 receive Hold 再保存清理标记；正常退出/整体工作区切换不生成全量解绑。

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

Application 唯一接收 owner 接续隐藏保存后的呈现/回执；原 Document/Canvas/RenderSurface 只提供新帧提交事实。HTTP 仍归原 Control/Job，原 Hold 跨阶段移动，未建立新任务池、凭据或持久化 owner。

最终严格静态/3799 文件 ratchet 零违规/警告/新例外；同源八件 45 选定门全过。
24 变更/7 新增、最大 381 有效行，UTF-8 无 BOM；visible 3/0、present 5/0、
placement 2/0、确认 1/0，过滤组不累计覆盖。344 旧件/七诊断、开发失败/输入说明、
最终冻结输入/七文档与源码三次匹配见迁移台账 v393/closeout。startup 7/0 不关旧根因，
正式前台菜单仍 ignored；无真实时钟到期/worker panic 全链注入，不代完整迁移。

v392 由 Application 的单一接收状态连接原网络 Ready→后台候选→隐藏追加→精确
checkpoint。目标 Source/Space/Guard 在发网前捕获，原 Grant/Store/Record 必须一致；
Hold 保原 Lease、共享槽和同 intent 许可。Job 在错误/取消/panic 后也持 Hold 至 UI
领取，成功后移动给 checkpoint 阶段；worker 不拥有活动许可。Preparing 纳入原
恢复/保存/退出围栏，poll 取走自身状态后复核，避免自阻塞。撤权与完成复用 Wake，
到期复用原 deadline，无第二任务池/等待句柄或轮询。终态/切换停止旧 owner，本地
Unit/journal 不隐式删除。最终严格静态/3,792 文件 ratchet 零违规/警告/新例外，同源八件 41 选定运行门全过；
25 变更/6 新增、最大 454 有效行，UTF-8 无 BOM，owner 10/0、watch 2/0，过滤组不累计覆盖。四份开发输入、
最终 2,640 冻结输入/七文档、首个静态候选/失败程序/析构诊断、336 旧件/五诊断及
后续 344 件/七诊断保护见迁移台账 v392/closeout。startup 7/0 不关旧根因，前台
菜单仍 ignored；尚无接受菜单、可见呈现/displayed/自动
补偿/持续同步/Tile，不把本链当完整迁移或用户包。

v391 复用接收/保存/恢复 worker，将原意图 UUID/设备与 Unit 的完整 qrProjection
关联落入同一会话。小型索引持共享不可变 Source，不复制 PNG；checkpoint 在
Receipt/staged/candidate exact 身份和真实 disk.commit 成功后才提升。规范启动
同源恢复可重建关联，恢复副本/普通导入不直接提升；新 Source/新 intent/删除不能
借旧包含事实。profile 菜单仅非破坏性分类，不替代 Lease/可见显示/displayed，
不自动发网或退休意图，无第二保存/状态 owner。最终严格静态/3,786 文件 ratchet
零违规/警告/新例外，同源八件 39 选定门全过，28 变更/4 新增、最大 349 有效行，
UTF-8 无 BOM。关联模型 5/0、checkpoint 新组 5/0/全组 18/0、恢复菜单 4/0；过滤组
不累计独立覆盖。四份开发输入、最终 2,634 冻结输入/七文档、首个静态候选和 null
兼容修正后的 final-run2、328 旧件/五诊断及后续 336 件/五诊断保护见迁移台账 v391/
closeout。startup 7/0 不关旧偶发根因，前台菜单仍 ignored，原 Grant 仍须由未来唯一
接收状态贯穿异步准备/提交；
完整接收状态/自动生命周期补偿/同步/Tile、完整迁移/真实 Loom/资源及用户包仍开放。

v390 在默认 workspace 恢复屏障之后由 Application 后台打开一次 journal，长期 owner
持 Store/未结 Arc 候选；完成复用原 Wake、终态 join，不增等待句柄或第二任务池。
显式终态解绑经原共享菜单确认→profile 同候选→当前 Lease→原 Sessions 单槽→认证
unlink→持久退休；仅在任务回收边界对账 Store 内存，取消/切换不复活已退休项，
本地图片不删除。检查/无 profile 启动隔离；最终严格静态/3,782 文件 ratchet
零违规/警告/新例外，同源八件 37 选定门全过，19 变更/5 新增、最大 453 有效行，
UTF-8 无 BOM。profile 4/0、恢复菜单 3/0、Application 解绑 8/0，过滤组不累计
覆盖；两份开发输入、2,630 冻结输入/七文档、初始空过滤/夹具失败程序、320 旧件/
四诊断及后续 328 件/五诊断保护见迁移台账 v390/closeout。startup 7/0 不关旧
偶发根因，前台菜单仍 ignored；不代接受入口、
checkpoint 包含凭证、可见 present/displayed、持续同步或完整 N09/迁移/用户包。

v389 接收写前意图复用原 session 文件事务：Store 持独占 profile 租约并由后台宿主
共享；Record 只存原邀请/设备/Unit/原期望值，128 项/512 KiB，无 token/PNG。
同 intent 许可贯穿 HTTP→Ready→Ticket，当前记录/来源/Lease 先验后发网，不把
共享 Store 的短 Mutex 当网络串行 owner。取消不删不确定副作用，认证原来源解绑
且 unlinked:true 才退休；失败/403 保留重试。profile_directory.pin 共用原祖先
算法，原文件 CAS/无覆盖恢复/保存槽和工作区状态 owner 不变。尚无接受菜单、
工作区意图关联/重启对账、真实 present/displayed 或自动退出补偿消费者。最终
严格静态/3,777 文件 ratchet 零违规/警告/新例外，同源八件 34 选定门全过，16
变更/6 新增、最大 440 有效行，UTF-8 无 BOM；journal 14/0、认证补偿 9/0、投送
客户端 49/0，过滤组不累计覆盖。六份开发输入、最终 2,625 冻结输入/七文档、
312 旧件/四诊断及后续 320 件保护见迁移台账 v389/closeout。startup 7/0 不关闭
旧偶发根因，菜单仍 ignored；不将此内核当完整 N09/迁移/用户包验收，不更改 Loom 实现。

v388 接收准备内核：认证 accept 返回压缩 PNG，原 Grant/四槽贯穿解码 Ready；
完整验证后生成确定 delivery-ID/qrProjection 会话，已解码像素移交现有 Candidate，
不再解码。新增共同 session_append/application_append 分别拥有原文与隐藏单窗口
追加，冻结库复用同一事务，Source 保 allocation/编码预算，原保存/checkpoint 仍
唯一落盘 owner。接受菜单、持久取消补偿、真实可见显示/displayed 仍缺，隐藏
HWND/落盘测试不能代替这些产品门。最终严格静态/3,771 文件 ratchet 和同源八件
32 个选定门全过，包含真实 TLS 接收、隐藏窗口追加/检查点/重启逐像素及冻结库/
Candidate/工作流归档回归，过滤组不累计覆盖。31 变更/12 新增、最大 367 有效行，
UTF-8 无 BOM；大枚举仅 Box 化，首轮失败 exact 输入另保。304 旧件/四诊断及后续
312 件保护见迁移台账 v388/closeout。startup 7/0 不关闭旧偶发根因，前台菜单仍
ignored；不代持久补偿/真实显示/displayed、完整 N09/迁移或最终用户包。

v387把传统投送单次收件箱查询/拒绝接到现有授权共享菜单和Application结算点。
UI只保无凭据候选/请求，原Sessions子模块持唯一Job，原Lease/四Slot/HTTPS/完成
句柄和退出drain保持唯一owner；Application先处理切换/撤权/成功工作区交换再
收结果，当前Arc身份保护候选。取消不提前take未完成线程，recorded:true才报告拒绝
成功；不建轮询或第二网络/身份/工作区实现。最终同源运行与生命周期实绩见迁移
台账v387；accept/图片下载/持久化显示/displayed/同步/离线互信/journal/Tile仍缺，
不把元数据消费者当完整N09或迁移/真实Loom/资源长稳/最终用户包验收。

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

v386在普通文档窗口的WM_WINDOWPOSCHANGED边界补有界queued结算通知；sent消息
自身不会保证外层GetMessage返回。每HWND一条合并通知、ack后重开、销毁禁投递；
保DefWindowProc的WM_MOVE/WM_SIZE，保存仍由原Application/单槽/Receipt/检查点处理。
没有第二模型/保存owner或定时轮询。确定性RED与最终原产品链实绩见迁移台账v386，
不把此修复当N09桌面接收、完整N06/迁移或最终用户包验收。

v386最终final-run严格fmt、全目标Clippy -D warnings/check、版本及官方ratchet
均过，零违规/警告/新例外；7份native变更/3新增，最大479有效行，UTF-8无BOM。
同源八件21个选定后台/隔离门全过：wake3/0、checkpoint8/0（含两项新隐藏HWND），
原workspace_runtime2/0/1 ignored完成移动提交→强制中断→重启恢复→删除最后Unit
落空；保原15秒和断言。正式菜单退出仍ignored未验，不关闭v384前台或旧完整组/
压力失败。三次源码/288旧件/四诊断/v385完整失败输入及原现场由closeout复核，
后续保护296件；状态window-geometry-wake-and-workspace-runtime-verified，不代完整迁移。

v385把传统投送inbox/新邀请rejected接到原认证Control的私有发送接缝；合同解析/
envelope形状、HTTP方法、单次后台Job分责，共用原4任务Slot/Lease/取消和后台CA/
DNS。完成领取仍验撤权/到期，当前没有workspace消费者，不重新配对或建第二状态/
网络owner。不是接收整链，accept/图片持久化呈现/displayed/同步/离线互信/Tile
仍缺。最终定向实绩见迁移台账v385，不关闭旧前台或完整迁移/用户包门。

v385最终final-run2严格fmt、全目标Clippy -D warnings/check、版本及3,749文件ratchet
均过，零违规/警告/新例外。10份native变更/7新增，最大362有效行；四生产模块
168/111/29/128，三tests186/151/196，UTF-8无BOM。同源八件的13个选定后台/隔离
门12通过/1失败，projection16/0、device HTTP144/0（包含新9项，不能累计独立覆盖）。
workspace_runtime1/1/1 ignored在checkpoint-moved-window原15秒超时，尚未进入中断/
重启/删除链；失败profile仍revision7原坐标，stderr仅热键冲突，不足以确定根因。
菜单前台门没有重跑，ignored不计通过。三次源码/
旧280件/四诊断、首轮静态失败完整输入及288件后续保护见final-run2/closeout；
状态projection-client-implemented-workspace-runtime-pending，不代桌面/N09或完整迁移。

v384仅修workspace_runtime正式菜单退出测试的Win32类名发现；Static/STATIC按ASCII
不区分大小写，原PID/精确owner/visible和确认标题仍核验。新matcher子模块只拥有
离屏NOACTIVATE窗口正负回归，不改变生产菜单、退出Intent/Receipt/CAS或前台准入。
开发真实菜单链1/0，最终同源封存验证及272旧件/四诊断身份见迁移台账v384；不代
完整N06/N01–N12、旧前台return/压力、N09、资源长稳或最终用户包验收。

v384最终两份测试变更/一新增最大197有效行，严格静态/3,742文件ratchet及18后台门
通过。独立正式菜单门0/1在原15秒前台前提失败，未入菜单；开发1/0不代最终候选
原生批准。完整失败输入/七文档/八件、三次源码/旧272件/四诊断与280件后续保护见
closeout，状态native-exit-menu-foreground-pending；生产owner及旧门边界不变。

v383将正常退出的最终时间点留在原保存单槽/worker/Receipt/文件事务。独立轻量退出
Intent不再借手动dirty判断Live批准；checkpoint owner只扩最终等待及预期revision。
提交前与销毁前两次复核来源/模型/待处理意图，Live只提供自身窗口状态，不恢复
采集/控制权。无首帧或stale/错误保窗口、无profile/只读owner不新增写权；模块边界、
真实产品菜单退出与最终验证状态见迁移台账v383，不扩大完整迁移或用户包结论。

v383首轮startup4/3证明完整restore idle不能作为无profile退出批准的通用条件；旧
smoke原有execution/preview退出清理断言保持。final-run2将批准稳定性与最终profile
捕获/提交/销毁的完整idle分离，后者仍沿原Receipt/CAS。首轮17/19与完整失败输入/
八件独立保留；真实菜单前台0/1后不循环争抢，修正候选菜单整链未验，不继承旧证据。

v383最终两新增生产owner Intent/final_exit均64有效行；20变更/6新增最大450行，
严格静态/3,741文件ratchet及18选定后台/隔离门通过。退出11/0保护一次性批准、最终
快照及no-profile清理，Live checkpoint19/0与相邻58/0逐层包含，旧startup7/0恢复；
菜单退出1 ignored仍未验。三次源码/八件/旧256件/四诊断、首轮失败完整输入/八件及
272件后续保护见closeout，状态exit-checkpoint-native-pending，不代完整迁移或长稳。

v382将Live时间点接入原默认检查点/显式文件自动保存链；无首帧或手势busy延后，
基线只记实际Receipt的Live身份/几何/透明度，不记流序号或持有GPU帧，不逐帧写盘。
手动dirty与独立profile基线保持，恢复为静态贴纸，不恢复捕获/控制权限。Live显式
操作仍由原window/单槽owner持有，短路消费与终态内部唤醒避免多个请求丢失；无新
队列/定时器/序列化owner。测试和最终实绩见迁移台账v382；退出flush/完整迁移仍未验。

v382最终18变更/4新增最大437有效行，严格静态/3,735文件ratchet及17相关门全过；
新Live8/0和相邻Live47/0为包含关系，不累计作独立覆盖。真实WARP/静态恢复/中断和
普通EXE启动链共同保护原owner；三次源码/八程序/旧248件/四诊断、256件后续保护见
迁移台账/closeout，不复活捕获权限、不改变完整JS/迁移或用户包门禁。

v381普通无来源启动先取得OtterNative隔离workspace租约和逐层非reparse目录pin，再经
原session_job/Bound/隐藏建窗恢复；恢复完成后才显示和注册全局捕获。显式CLI来源及
smoke不接默认workspace，--workspace-check必须隔离且只读workspace内容。持续静态
检查点复用保存单槽/Receipt/编码/原子事务，独立磁盘修订和轻量Stamp不改变手动文件
目标或dirty；最后Unit删除写空与应用退出保工作区由独立终态区分。最近成功快照是
恢复保证，不是最后编辑/退出flush/断电零丢失；Live自动持久化及完整N06仍开放。
模块owner、开发失败、定向验证和源码/程序身份见组织规范及迁移台账v381。

v381的Owner缓存已验证磁盘修订，与成功提交基线同进，不在事件调度重解析完整JSON；
全字节冲突校验仍由原文件事务拥有。最终25变更/8新增最大447行，严格静态/3,731
文件ratchet及14相关运行门全过，含普通EXE默认发现/移动提交/中断重启/最后Unit清空。
三次源码/八程序/旧240件与四诊断及248件后续保护见迁移台账v381和final-run2/closeout，
不以静态链成功替代Live/最终flush/完整迁移或用户验收，原前台未通过门仍开放。

v380新增application_workspace_autosave作为显式自动保存协调owner，多个State只借用
同一Rc开关/代际；后台编码/磁盘暂存/Receipt/原子替换仍由现有保存链拥有。正常busy
复用restore guard的idle判定，过期候选回收后才生成当前快照；无空闲定时器或新队列。
成功换工作区/另存路径失效授权；手动请求留在原UI槽，退出先取消自动候选再确认。
自动保存不调用外部工作流，不将Live的时间点快照标成永久干净，不扩Surface能力。
默认启动恢复/崩溃恢复及完整迁移仍开放；最终证据和身份见迁移台账v380。

Application::new完整移至application_initialize，唯一持有首窗口到共享状态/后台任务
通知与初始会话的构造接线；字段布局及析构顺序仍在Application，不新增状态owner。
该移动用于解除本轮494→501行的严格软限制失败，不增加豁免或改变初始化时序。

最终Application432、构造74、自动保存148有效行，19变更/4新增最大432；3,723文件
ratchet与严格静态通过。10相关运行门含真实产品startup7/0、session_startup9/0，
三次源码/六件程序/旧234件及四诊断、240件后续保护见迁移台账v380和closeout；
不以本次保存/恢复闭环替代默认启动恢复、崩溃恢复、Live或完整迁移/用户包验收。

v379仅测试层的前台启动/许可交接候选，不改变产品架构。启动握手仍归原fixture，
Application在原TLS计时前调用；Foreground继续有条件归还，Child/管道/超时/Drop仍归
原UIA进程owner，定向许可与return不成为产品激活API或第二进程框架。真实点击只
发生在自有诊断按钮，外部窗口不接收消息。最终原生尝试因实际输入导致父前台丢失，
新增归还路径未到达；停止争抢前台，四项最终前台门未执行。后台/静态/封存身份见
迁移台账v379，保持foreground-handoff-native-pending，不代完整迁移或最终用户包。

最终四份native测试变更最大467有效行，严格静态/3,719文件ratchet通过；八后台门全
过，三次源码/六件/旧228件和四诊断、234份后续保护见迁移台账v379。同源前台0/1
仍是失败，四项完整前台门未重跑，不因编译/后台成功宣称许可归还或完整迁移已通过。

v378扩展原Insets为四边border used width，几何/测量/absolute/client/scroll及控件投影
统一消费同一盒模型。圆角内边界由原Shape独占，CPU hit与D2D clip不再各算uniform
近似。Border四边共享一个factory缓存，半平面分色仅产生最多七顶点的凸多边形，ring
唯一决定内外边界；EDIT临时HRGN由独立RAII模块释放。私有CSS borders factory接原
declarations/cascade，不建立新DOM、队列、线程或权限owner。通用纵向回归和最终
身份见迁移台账v378：最终45变更/11新增/最大445有效行，3,719文件ratchet零违规警告，
22相关门全过，三次源码/六件/旧222件与后续228件保护。不能据此宣布完整JS/真实业务、
长稳或完整迁移/用户包验收，也不关闭旧前台/压力未知失败。

v377新增surface_text_overflow作为原文字Engine的最终呈现适配，不参与固有测量或
继承Style。text_frame在最终分配宽度后创建并配置省略布局，原metrics/UTF-16/几何
不变，绘制和命中继续共享同一COM布局；滚动只重投影，后帧不可修改旧共享对象。
text_word只借隐藏后缀范围，DOM/CSS/Runtime/Session/FIFO/权限仍各归原owner；CSS
仅开放明确的nowrap/left/hidden文字叶子集。验证与身份见迁移台账v377，不等完整
JS/业务/迁移或最终用户包，不以新切片关闭旧前台或性能未知失败。

最终v377固定19相关门全过，含DWrite/WIC/异常trimmed零高命中、真实child/产品与
隐藏HWND；原四压力门显式按原规模/预算通过。17变更/7新增/最大440有效行及3,708
文件ratchet通过；三次源码、六件、旧216件/后续222件保护见迁移台账v377。不存在
第二文字/选择owner或跨帧新缓存，旧前台失败、完整JS/真实业务/长稳/迁移仍未关闭。

v376仅收敛原CSS computed内的临时winner容器为空原型record，准入的canonical key、
优先级和源序不变；声明Map、原DOM/几何/JSON/Runtime/权限owner不变，无跨帧缓存。
新增测试模块保护旧Map差分/调用计数/污染，原projection继续拥有rank oracle，不新增
生产模块或第二owner。20门18过/2前台前提失败，四个原规模压力本次通过，但固定
墙钟样本未证整体加速，历史根因未关闭。行数/三次源码/六件/旧210份及后续216份
保护见迁移台账v376，仍partial-runtime-failure，不代表完整迁移、默认JS或用户包。

v375最终生产owner与首轮一致；原拒绝fixture仅更新支持集边界，保原子断言，不新建
业务或状态owner。21门17过/4压力失败：功能、产品、HWND和CSS模型/投影/Session
通过，Collect期限及首轮完整窗口前台未知失败仍开放；不提高预算、不重跑旧窗口
求绿。最终源码/两套六件与210份保护见迁移台账v375，当前仍partial、非完整迁移。

v375的author :hover复用原Hover状态/私有DOM连接与原cascade/Scene投影/几何mutation
identity；bootstrap只接线，不建立第二份树、状态缓存或FIFO。三个新测试owner分别
负责child/产品、Engine有界模型/压力及真实HWND像素/生命周期，通用fixture不翻译
Loom股票业务。不扩公开DOM selector或静止指针自动重命中；最终运行/身份与保留
失败见迁移台账v375，不放行默认完整JS、真实业务、完整迁移或最终用户包。

首轮22门18过/4失败，三类旧CSS拒绝样本失败与独立完整窗口前台失败分别保留；
后续仅由原拒绝fixture更新“不支持”的样本，不改变产品架构、授权或输入准入，
不重跑未改窗口组求绿。行数/相关成绩/六件与源码身份见迁移台账v375。

v374未改产品架构或focus准入。Application测试逐段拥有真实前台前提，自有独立
进程负向沿现成UIA Child owner证明线程焦点与前台不同、拒绝后不重放。新增模式/
窄重导出均仅测试构建，无第二进程框架、状态或权限owner。诊断未复现旧失败及同
线程前提失败分别留证；最终矩阵/身份见迁移台账v374，不代完整迁移或用户包验收。

首轮失败促使进一步收窄测试验证边界：网络updated只验数据，focus/blur在交付点
验真实前台/焦点；Foreground仅加track_caller。显式跨进程新门始终单独运行，默认
ignored不计通过；首轮与修正候选分开封存，不修改产品架构或覆盖旧完整组失败。

第三候选把自有子进程激活移到目标线程，以原管道一次命令/确认替代父线程立即
检查；被动Foreground guard仍沿原有条件恢复，无产品激活、额外状态owner或重试。
前两候选失败与第三候选实际10门/身份分别见迁移台账v374。

最终10相关门全过，Application focus3/0、完整默认窗口129/0/1 ignored及显式跨进程
负向1/0分报；默认ignored不计通过。六份测试/接线变更最大321行、严格静态/3,696
文件ratchet、三次源码/六件/旧192份与前两候选见迁移台账v374/closeout，下一轮保护
198份。产品focus源码与before一致，不关闭旧完整组失败或放行默认JS/完整迁移。

v373未改产品架构。声明式键盘测试在原owner内显式建立普通消息的线程modifier，
保存/恢复完整键盘表；raw消息保产品快捷键拒绝/Shift反向。无全局输入、前台激活、
新生产状态或权限owner；受控RED与最终相关门/身份见迁移台账v373，不以测试夹具
修复代替完整迁移、默认JS、历史未知根因或最终用户包验收。

最终一份测试变更152→296行，严格静态/3,696文件ratchet通过。8门7过、1失败，
fixture7/0、窗口129/0、产品keyboard通过；Application14/11的EDIT焦点断言/十项
锁中毒不被窗口成功覆盖。2,544输入/三次源码/六件/旧174份及三件诊断身份见迁移
台账v373/closeout，保持partial-runtime-failure，不改变产品架构或关闭历史根因。

v372在既有Win32→Mount FIFO→Runtime/child→EventTarget→Canvas/DOM→Application链路
补非捕获hover，允许真实consumer的包装节点listener。通用最上层DOM命中借原Presentation，
JS只保上一条有界派发路径，不复制树/几何；两类terminal共享原容量/sequence，硬取消
保prepared、未派不伪造leave。hover不夺EDIT焦点/捕获，物理Canvas Press仍归原pointer，
State/原窗口生命周期处理重入取消。新增模块/测试责任见CODE_ORGANIZATION，最终运行
分解与源码/程序身份见迁移台账v372，不把普通hover当完整Pointer/真实股票/默认JS/
OS隔离或总迁移验收。

最终严格静态/3,696文件ratchet通过，35份native变更最大500行。34运行门32过、
2失败；Hover独立Application1/0完成两轮TLS/Grant/FIFO/同child，完整JS189/0及
十四产品worker/三原规模压力门通过。完整Application13/12与窗口122/4的直接失败/
锁中毒分解见迁移台账v372，不以独立成功覆盖。2,544输入、三次源码/六件/旧168份
由最终closeout核验，仍partial-runtime-failure，未新增权限owner或放行完整迁移。

v371在原Win32→Mount FIFO→Runtime/child→EventTarget→Canvas→Application/TLS/Grant
链路接Canvas左键Pointer。物理Press归surface_window_javascript_pointer，原Queue保存
terminal reservation与已派手势元数据，后者不是第二队列；原State::run归还Host时结算
同步重入取消。共享容量/sequence/fence、viewport epoch、当前target和授权链不变。
只在背压/同节点JS回帧/同HWND下一Canvas手势软保留，真实失焦/隐藏/几何变化硬取消；
普通控件/选择/滚动仍由原owner处理。新模块/测试责任见CODE_ORGANIZATION，最终实绩/
身份见迁移台账v371，完整Pointer、默认JS/OS隔离和完整迁移验收不因此完成。

最终37份native变更最大497行、3,684文件ratchet/严格静态通过；27运行门26过/1失败，
窗口120/0、JS184/0和十三产品worker过，Application13/11前台失败/锁中毒保留。新增
Pointer固定一次独立Application1/0完成两轮原TLS/Grant/FIFO；身份/三次源码/六件/
旧162份及部分失败状态见迁移台账v371/closeout，不新增队列/权限owner或扩大验收结论。

v370只缩减原CSS cascade热路径分配：保留important/inline/三项specificity及遍历源序，
每次computed独占winners并原地更新当前属性，不修改sheet或selector、不跨节点/帧缓存。
原JSON、DOM、几何、worker/Job和授权owner均不变。独立oracle、旧算法差分、局部性/污染
与原规模Engine/child/实际产品证明见迁移台账v370，不扩大默认JS或完整迁移验收结论。

最终严格静态/3,671文件ratchet通过，两文件最大300有效行；24运行门23过、1失败，
压力三路径、完整JS177/0和窗口111/0通过，Application前台获取失败与九项锁中毒保留。
三诊断及新旧库均15/15的固定矩阵、三次源码/六件/旧156份见迁移台账v370/closeout，
不新增架构owner或以局部优化替代Pointer/默认JS隔离和完整迁移验收。

v369未改产品架构，仅修测试Foreground guard退出时覆盖后来前台选择的清理所有权。
previous/target留在原测试fixture；只有仍持有target或target已毁且前台NULL时恢复。
desktop的foreground tests保护新选择/普通归还/unwind，测试自身cleanup也不覆盖外部
新前台；不公开HWND或新增产品调度/输入/资源owner。候选诊断一次通过不关闭v368丢失，
最终实绩/身份见迁移台账v369，前台获取与其他历史根因及完整迁移验收仍开放。

最终严格静态/3,671文件ratchet通过，四测试变更最大192有效行；12门10过、2失败，
Application23/0、values5/0、child/队列/摘要绑定产品keyboard、startup7/0及资源/worker
门通过。foreground1/1的unwind初始前台获取失败，完整窗口93/18中两项新回归均过，
但旧focus获取失败+17锁中毒；不以另一上下文成功覆盖失败。首轮Clippy失败、身份/
三次源码/六件/原150份见迁移台账v369/最终closeout，保持partial-runtime-failure。

v368未改产品架构：原键盘入口继续要求真实前台、当前EDIT焦点和原输入策略。只修
Application测试在TLS批次后仍假定持有前台的驱动，用原测试Foreground guard按输入段
建立/恢复，每次发送前核验；新增真实后台down/up不得入原FIFO且恢复后不重放的证明。
精确观测、最终完整Application/相邻门与身份见迁移台账v368，不把未观测历史失败或
压力/form/startup未知根因一并关闭，不增加生产状态/调度/资源owner。
新驱动开发门仍因当前游戏占前台而0/1，后台不入队负向1/0，均保留原证据。未操作
游戏；用户随后允许短暂占用前台，最终恢复完整10门矩阵，实际终态另记，不把授权、
运行启动或更准确的前提错误诊断当成完整Application通过。

最终10门8过、2失败，无deferred门：Application13/10为Enter前台fixture失败+九项锁
中毒；SurfaceWindow90/19为点击候选未入队、另一前台fixture失败+17项锁中毒，新增
后台负向最终未进入测试体。严格静态/3,670文件ratchet通过，两个测试owner为134/329
有效行；child/队列/SHA绑定产品keyboard、startup7/0及资源/worker门通过。三次源码/
六件/原144份与精确实绩见迁移台账v368/最终closeout，保持partial-runtime-failure，
不把局部通过当未知根因已修、默认JS/完整迁移或资源长稳验收。

v367在原CSS owner缩减全定义表扫描、class拆词和sheet匹配时的迭代器分配。计算记录仍
是单次投影私有null-prototype对象，规则有序声明替代旧Map，不增加树、缓存或状态owner；
inline、原层叠和真实native consumer不改。原算法差分、固定诊断与真实child/产品压力
各自留证，最终成绩/身份见迁移台账v367，不外推完整迁移或资源长稳/旧keyup根因关闭。
私有JSON仍是唯一编码/复制owner，bulk描述符不改变payload标准序列化或协议形状；阶段
时钟只插入测试副本，独立诊断上限不进入Engine/child/产品的原操作期限。

v367最终24实际运行门18过、6失败，另两诊断独立分报；原压力Engine/child/实际产品
均0/1，Application13/10、JS137/38/26 ignored及startup4/3的失败不被窗口108/0或八
非压力产品worker各1/0覆盖。严格静态/3,670文件ratchet和12文件行数审计通过，最大
428有效行；三次源码/六件/原138份、计时自检和partial-runtime-failure见迁移台账v367。
默认JS、真实股票/完整迁移、总资源长稳/用户验收及旧未知根因仍开放。

v366的author CSS位置匹配留在原selector owner，原tree只提供已有mutation identity；
每代按需WeakMap索引元素兄弟位置，不另存树或布局。nth-child公式/specificity仍经原
sheet/cascade/projection进入同一native consumer，first-child语义不扩大。三份通用
夹具分别验证四列重排、同步几何失效与原型污染；Rust oracle、child/实际产品及HWND
像素/命中分开验。精确实绩和限制见迁移台账v366，不改变默认JS或完整迁移/长稳门禁。

最终14个运行门13过、原256节点/16规则压力门失败，未提高250ms；nth-child6/0/2 ignored、
完整窗口108/0与三摘要绑定实际产品worker各1/0等不代表复杂样式吞吐或完整业务达标。
首轮Application13/10、固定一次Enter复核1/0及最终不同程序字节分别留证，根因仍开放。
严格静态/3,664文件ratchet和11文件行数审计通过，身份/partial状态见迁移台账v366。

v365仅在原form owner降低有界值校验的临时字符串/逐字符数组分配，捕获的native
exec处理ASCII/连续保留段，Unicode仍先验原代理项/UTF-8边界。无新状态、权限、调度
或资源owner；原child/单槽/EDIT草稿/Grant/FIFO不变。测试专用阶段/线程CPU探针、
Rust oracle差分和child/产品证明分开留证，结果/边界见迁移台账v365，不外推完整
迁移、总资源/长稳或历史未知根因已清除。

最终七个native变更最大422行、四新增最大92，严格静态/3,658文件ratchet及43选定
封存门通过；完整窗口107/0、Application23/0、JS164/0/20、十四产品worker各1/0等见
迁移台账v365。原mount/ASCII固定中位大幅降低但无预算扩张；源码/六件/旧120份和
四份探针/失败分别留证，既有历史收据不覆盖，不能外推完整迁移/默认产品或长稳。

v364的文本input Enter沿原EDIT Session/Draft/ticket和32槽FIFO进入同一child，原
EventTarget返回私有取消结果；默认change在父侧重验原字段/dirty代次、scope/期限、
真实focus epoch/前台/策略/租约后替换原键盘槽，不另建输入队列或同步等待。键盘
不回写旧文本，默认消费已接纳DOM值，后续草稿仍归原Session。原窗口Host取出/归还/
stop仅等价迁入state_lifetime模块，资源/授权owner不变。具体受限API、实绩和失败
见原生JS合同/迁移台账v364；不是全键盘、同步form submit、默认JS或完整迁移验收。

38个native变更最大493有效行，严格静态/3,654文件ratchet通过；41封存门首轮40通过、
1失败。Enter11/0/1 ignored、Application23/0、JS161/0/18、十三产品worker各1/0等通过；
旧输入夹具mount期限失败与两项锁中毒保留，同字节固定六门诊断含完整窗口107/0
通过不等根因关闭。三次源码/六件/旧114份及partial-runtime-failure收据见迁移台账
v364，不为复核新增排程owner、放宽预算或扩大完整迁移/稳定性结论。

v363以原child回复交付最多32项focus/blur意图，不复制浏览器焦点状态。父侧复用当前
Mount/语义句柄和真实focus epoch，只在已有前台Surface/应用输入策略内切焦，Host
已归还才进入SetFocus，使EN_KILLFOCUS继续进入原change/FIFO。应用同一输入策略在
回复前收紧，过期/ABA/后台/撤权请求不重放；不建立新线程、队列、窗口索引或权限。
这不是同步activeElement/焦点事件/滚动/options，真实键盘默认动作取消仍缺。精确
边界/实绩见原生JS合同与迁移台账v363，默认JS/真实插件/完整迁移门禁不变。

最终24个native变更最大499有效行、八新增最大168，严格静态/3,645文件ratchet和
37选定封存门通过：焦点9/0/1 ignored、JS157/0/17、Application22/0、十二产品worker
各1/0及startup7/0等见迁移台账v363。三次源码/六程序/旧108份及开发失败分别留证，
仍不承诺同步浏览器焦点、键盘默认动作取消、真实插件或完整N01–N12/资源用户验收。

v362把文本input接到原Win32 EDIT/Session/Draft/Mount/Queue，不建立第二套本地输入。
私有TextInput只传有界target/phase/value，后台Runtime复用原单槽与child，事件沿
同一EventTarget/Engine job边界；本地ticket在DOM帧接纳后结算，emit的远端authority/
Grant/FIFO/收据独立保留。后台忙不清掉合法本地草稿，点击继续用原fence限定前缀；
form只管dirty值/属性反射，不持窗口、布局或远端权限。实际行数/回归/身份见迁移
台账v362；keydown/默认动作、focus/blur、pointer、默认JS及完整迁移/用户验收未放行。

最终36个native变更最大483有效行、八新增最大175，严格静态/3,637文件ratchet及
33选定封存门通过：输入7/0/1 ignored、JS153/0/16、Application19/0、十一产品门各1/0
及startup7/0等精确结果见迁移台账v362。失败、三次源码/六程序/旧102份保护分别
留证；不把同child/native EDIT基础链路等同完整浏览器、真实插件或最终用户验收。

v361以私有EventTarget owner统一同文档合成事件与已有native click，登记、阶段快照、
once/passive、取消/停止与finally清理不再由bootstrap手写冒泡。Node保原树owner，
EventTarget复用其捕获intrinsic/父链接与原单调时钟；公开dispatch不可信，私有native
click可信，事件没有远端authority。真实child/HWND/Application沿原Grant/FIFO/收据与
同child更新验证，不新增任务/协议/权限owner。精确子集/原预算及实绩见原生JS合同/
迁移台账v361；键盘/pointer/input接入、默认JS、完整N01–N12/股票Loom/资源长稳/
最终用户验收仍未放行，旧未知根因不清除。

native click沿原Engine job边界直接分派，bootstrap显式await，公共dispatch仍同步；
私有carrier/Promise保护不新增跨operation队列、线程或预算owner。最终19个native
变更最大386有效行、八新增最大148，3,629文件ratchet/严格静态与29封存门通过，
包含事件10/0/1 ignored、JS149/0/15、Application18/0、十产品worker各1/0及startup7/0。
UIA夹具/两条同步栈/模型接线失败、三次源码/六程序和旧96份保护见迁移台账v361/
final-run closeout；不外推旧未知故障或完整输入/真实插件/长稳/最终用户验收。

v360将原surface_box的单标量内缩改为四边Insets；起点和两轴总量统一交给原布局、
测量/反馈、文字/图片及textarea owner。CSS只展开/级联四个longhand，不新增JS
布局或渲染树。同步几何/ResizeObserver复用同一内容盒；absolute和overflow继续
使用原padding edge。surface_round_shape只求四个内容圆角及真实D2D几何，原Clips
继续负责工厂绑定缓存/资源回收；单值/默认控件和同HWND草稿合同保留。测试与实际
验收见迁移台账v360，不改变默认JS或关闭旧未知失败；完整N01–N12/股票Loom、
总资源/长稳及最终用户验收仍开放。

最终40个native变更最大455有效行、九新增最大168行，3,621文件ratchet/全目标
严格静态与26封存门通过，包含padding43/0/1 ignored、JS141/0/14、九摘要绑定产品
worker各1/0与startup7/0。首轮JS的过期双值拒绝/scalar预期仅修两个测试输入，
生产输入相对首轮不变，但两轮源码/程序身份独立，不以重测或局部通过解释旧未知
根因。失败收据、三次源码/六件与旧90份保护见迁移台账v360/最终closeout。

v359的CSS fonts factory统一字体声明值与font shorthand展开，原family选择仍归
文字Engine，未创建第二个字体/布局owner。unitless在原cascade中继承数字，逐节点
交原em resolver按最终字号解析；normal重置祖先行距，DirectWrite继续提供实际
基线。原declarations保持完整替换、priority与四longhand反射/移除，失败不部分
提交；模型/child/同步几何/隐藏HWND和产品成绩见迁移台账v359。默认JS、完整插件/
股票/Loom及N01–N12/总资源/长稳/最终用户验收仍开放，旧未知失败不清除。

属性用户转换前移至公开Proxy入口，投影的单调用native层合入prepare；真实Engine
已复现并修正该新重入栈机制，不提高原256KiB/250ms，也不解释旧未知Canvas/启动
故障。聚焦模型/child/HWND8/0/1 ignored与只读22条消费者字体值证据各自分报，
最终冻结/静态/产品封存见迁移台账v359，不用纯值探针代替实际产品运行。

最终16个native变更最大380有效行、无新例外，3,612文件ratchet/全目标严格静态/
六程序构建及21封存门通过；JS136/0/13 ignored、typography8/0/1、八产品worker各
1/0和startup7/0。源/程序摘要、三次源码、旧78份、失败收据与边界见迁移台账v359/
closeout；没有新增默认能力、Loom修改或完整迁移/总资源/长稳/用户验收批准。

v358将CSS font-family继承接到原文字Engine的唯一已安装字体选择owner，按需取得
DirectWrite系统集合且仅保64项选择缓存；Styles以独立Rc持已选单一名称，文字
测量/绘制与GDI字段共用，不自引用、不枚举/下载字体或另建布局owner。原Scene
身份、原子呈现、同步几何和mutation失效链不变；已安装family选择不等逐字形
作者字体链或完整插件支持，实际边界/验证见原生JS合同及迁移台账v358。
最终values在trim前先验字体输入字符域，修复NBSP被少计并移除的原始UTF-8门缺口；
它与过期拒绝夹具的修正分开留证，最终生产输入也变化，不复用中间构建的成绩。

最终全目标严格静态、3,605文件ratchet和六程序构建通过，18个native变更最大377
有效行、无新例外；final-admission封存19门全部通过，JS129/0/12 ignored、原生
EDIT2/0、Select1/0、七产品worker各1/0及startup7/0。源码/程序摘要、首轮失败与
旧72份保护见迁移台账v358/closeout；成功只覆盖本字体选择切片，既有Canvas/启动
未知根因、完整N01–N12、真实股票/Loom及总资源/长稳/用户验收仍开放。

v357的固定失败阶段只编译进测试：原Engine/Canvas owner以有界Trace记录阶段，
测试worker沿原私有Response交付可选Report，Session在原失败分支脱敏输出；不
创建新的执行/资源owner或JS调用层，不读取异常属性。产品协议和原预算不变，
实际回归/封存状态见迁移台账v357，诊断不等于Canvas未知故障或完整迁移已修复。

直接Engine Canvas模型也在原apply错误返回后消费同Report；不覆盖未完成new的
状态，不改变异常身份或生命周期。首轮20门19过，JS124/1/11 ignored的模型mount
失败保留；只改该测试helper后的独立16门、三个完整JS125/0/11观察通过，仍不作
根因修复。九个native变更最大374有效行，3,599文件ratchet无新例外；源码/六程序/
旧60封存件及真实消费者CSS/input/listener准入缺口见迁移台账v357，整轮partial。

v356把CSS输入的String转换留在原公开setter/replaceSync，进入私有声明/规则parser
之前完成；同一次转换/解析/通知/提交顺序不变，重入同步几何不叠加外层parser栈。
Engine/真实child/产品摘要门共用两份通用夹具，原Session/Job/viewport/cache/回收
仍是唯一owner，不提高原栈、期限或几何预算。最终证据见迁移台账v356；不承诺
任意深度重入或默认JS，不以此结清旧未知失败、完整迁移或最终用户手测。

嵌套sheet值校验继续归原values，validate并入唯一调用者expand、color直接用捕获
exec，去掉冗余调用层而不放宽准入；grid/spatial及原生布局owner不变。

v356最终全目标严格静态/构建、geometry29/0/1 ignored、JS118/0/11、六个产品worker
及startup7/0通过；真实产品几何门包含七个重入模式。Canvas Application初始回复
失败及三项锁中毒连带使19门仅18门通过、整轮partial；四次新旧隔离成功不是根因
修复，完整身份与失败保留见迁移台账v356，不升级完整迁移/默认能力。

v355在原tree owner增加私有mutation identity token；属性/inline CSS/sheets/Canvas
尺寸提交统一使geometry的单份encoded source失效。缓存仅同Engine operation，
begin/dispose清理，投影前后核token；连接box getter每次仍交原生reads/布局/期限/
viewport准入，不缓存最终矩形，不改变授权/单槽/事件FIFO。模型fake boxes和实际
原生证据分开，验证状态见迁移台账v355，默认JS/完整迁移与旧未知失败不放行。

v355严格静态/构建和真实HWND/child/Application、五个产品worker及startup7/0通过，
但新增输入转换重入模型仍在原256KiB下栈溢出，geometry/JS两集合共享此失败，18门
仅16门通过、保持partial。模型副本只增加miss计数/故障注入，不在生产增加额外
投影wrapper；最终身份、旧42封存件与未定位边界见迁移台账v355，不清除旧失败。

v354的timer工厂与rAF共用同child/Instant；其有界登记不拥有业务/布局或远端请求。
Engine逐callback完成microtask，私有帧只携带有界剩余等待。原AuthorizedWindow
按策略/confirmed owner/输入和收据背压复用应用deadline与Runtime单槽；TimerTick
不提前触发rAF或不断后推已有rAF期限。Session/Mounted及呈现保原Grant/binding/
revision/源码期限/viewport/epoch与emit FIFO。支持/拒绝和验证见JS合同/迁移台账
v354，不引入新线程、队列、依赖或默认JS，也不清除原启动未知失败。

第二轮仅修旧counter无timer断言，生产输入不变；timer13/0/1 ignored、JS104/0/11
和五个摘要绑定产品worker各1/0通过。geometry集合与startup仍失败，18门仅16门
通过，整轮partial；未知根因/默认能力和完整迁移门不因成功子集而放行，见迁移台账。

v353将原隐藏启动验收序列从desktop/mod.rs迁入startup_smoke.rs，窗口/COM资源
仍归原owner并在调用返回后释放。smoke_timing提供显式、可配对的单调阶段计时，
写入失败不参与业务结果；图标、宿主、呈现目标和销毁只增加观测边界。默认关闭，
没有新线程、队列、依赖、网络/文件入口或启动期限变化。实际证据和未定位启动
失败见迁移台账v353，诊断不是完整迁移或用户验收。

最终严格静态/构建、计时5项及相关资源门通过；startup6/1仍超原15秒，整轮partial。
2,435输入和六程序身份分别冻结/封存复核，零项设备筛选由实际模块5项正向门补证，
不计零项为通过；旧失败和默认能力边界不变，完整记录见迁移台账v353/closeout。

v352的Canvas绘制命令由原JS业务产生，child持有有界WIC/D2D背板并用DirectWrite
绘制文字；不嵌浏览器、生成业务专用图形或借SVG字符串伪装Canvas。PNG仅为私有
进程间像素编码，父侧核对当前Scene身份、尺寸和容量后，纯Pixels才移交UI既有
Bitmap/Presentation。Canvas自然尺寸由同一原生布局消费，绘制按CSS盒fill缩放；
普通图片保持contain。无新线程、网络或授权owner，原heap/时间/Job/单槽/viewport
和代次门不增；实际支持/拒绝与验证见原生JS合同和迁移台账v352。

v352第二轮封存Canvas14/0/1 ignored、JS94/0/10及四个实际产品worker单项通过，
证明子集已走真实像素链；旧canvas负例改video及新增markup回归未改生产输入。
startup6/1超原15秒导致整轮16门仅15门通过，保持partial；最后restored-batch信号
不证明根因，旧未知失败、默认JS、真实Loom及完整迁移和最终验收均不放行。

v351 的 ResizeObserver 工厂只持同文档登记和有界深度交付批次；实际 content/border/
device box 仍由原 Scene/NativeMeasure/PreparedScene 和内容内缩 owner 提供。Rust
engine 在原 operation 收尾驱动 callback/microtask；rAF 不被同步抢跑。真实 HWND
尺寸/DPI消息只留代次，原 AuthorizedWindow 在当前策略/唯一 owner/单槽空闲门排帧，
与 rAF pending 分开且无空闲轮询。viewport+代次共同保护呈现和 emit，含 A→B→A。
没有新线程、网络/动作 owner 或第二棵业务树。精确范围与终态循环策略见
[ResizeObserver](../development/SURFACE_JAVASCRIPT_NATIVE.md#v351-事件驱动的-resizeobserver-子集)，
实际验证见迁移台账 v351；Canvas/真实股票/Loom、默认能力和完整迁移仍开放。

v350 的 animation 工厂封装有界 callback/handle 快照，Rust engine 逐回调完成 microtask
checkpoint，同帧 timestamp/performance.now 共用私有 Instant 时钟。私有操作沿原
Session/Mounted/Runtime 单槽和授权/快照/viewport fence；原 AuthorizedWindow 只在
sync_input 核定当前策略及唯一 confirmed owner 后，复用消息等待 deadline 排帧。
不新增线程/周期 timer/动作或授权 owner；外借 owner/背压不贡献过去 deadline。
真实 child/HWND/Application TLS/FIFO 与产品门分别验证，最终证据见迁移台账 v350。
精确支持/在途策略边界见 [rAF](../development/SURFACE_JAVASCRIPT_NATIVE.md#v350-宿主驱动的-animation-frame)，
ResizeObserver/Canvas/timer、VSync、真实股票/Loom、默认 JS 和完整迁移仍未放行。

v350严格静态/编译与源码rAF9/1 ignored通过，显式摘要绑定实际产品animation worker
另1项通过；封存animation4/5/1及其他child/geometry/TLS集合失败，UIA回归也有未知
根因，整轮和封存运行保持partial。SERIAL连带与独立失败分开记录；startup7/0或
后续单项成功不清除失败，精确矩阵和身份见迁移台账v350/closeout，不作完整验收。

v349 的空间 CSS 仅把已验证的 position/anchors/inset 和容器同轴 overflow 转换成
既有 Scene layout；原 surface_position/overflow→NativeMeasure/Tree→Presentation
仍独占几何、裁剪、像素和命中。spatial 工厂不计算坐标，不新增业务/窗口/授权/
资源 owner。declarations 在 cascade 前展开独立 longhand，原 Grid 共享反射/删除。
直接定位父、paint-tail 和百分比/滚动等保守拒绝防止默认回退冒充完整 CSS；详见
[空间 CSS 合同](../development/SURFACE_JAVASCRIPT_NATIVE.md#v349-受限定位与容器裁剪)。
Application 原 TLS/FIFO 使用定位按钮与同步读，原 ID/字节/Grant owner 不变。
本轮结果见迁移台账 v349；原 v348 启动超时仍未定位/修复，三次固定矩阵成功不
清除旧失败。默认 JS、Canvas/完整股票/Loom、权限隔离、长稳及 N01–N12 仍开放。

v349严格源码/编译与相关回归、封存后spatial11/1 ignored、实际产品worker、Application
JS10、原预算/资源及startup7/0通过；2,407源码输入和六程序/旧封存产品身份一致，
相关进程残留零。精确证据与早期失败见迁移台账v349/closeout；本轮通过不是原
v348启动超时的根因修复，也不是全库、真实Loom或完整迁移验收。

v348 的同步几何沿当前 DOM→CSS projection→Scene→NativeMeasure→PreparedScene，
返回同一字体/客户区/DPI 下的 border/client box；surface_viewport 是 Host/child 共用
合同，geometry 只有有界单投影缓存，不持第二棵业务树。initial/click/update 经过
原 Runtime 传真实 viewport，UI 交付前后与 emit 最终 batch 移交前均复核依赖；
过期终态关闭、不重跑 listener，原 FIFO/事件/Grant/关闭 owner 保留。无新线程、
宿主 I/O 或依赖，原资源门不提高。精确语义和未实现范围见
[同步几何合同](../development/SURFACE_JAVASCRIPT_NATIVE.md)，最终验证见迁移台账
v348；ResizeObserver/rAF/Canvas、自动 resize 恢复、默认 JS、真实股票/Loom、权限
隔离及完整 N01–N12 仍未放行。

v348严格源码/编译与相关回归通过；六程序封存后geometry15/1 ignored、Application
JS10、实际产品geometry worker及原预算/资源门通过，但startup4/3超15秒，因此
整体运行证据为partial。后来单场景通过未定位原超时，不提升为已修复。2,400源码
输入与六程序/旧封存产品身份一致，相关进程残留零；完整边界见迁移台账v348。

v347 的 CSS Grid 复用原私有 CSS/Scene→NativeMeasure→Presentation→Geometry/Hits 链。
css_grid_tracks 非递归准入轨道，css_grid 展开 shorthand/端点/枚举；declarations 在
cascade 前保存 longhand，projection 才处理上下文与 Grid/self normal 语义差异。
原生 Grid 继续独占布局算法，负线不在 JS 重算，无浏览器/业务分支/宿主 I/O/线程或
依赖。真实 child/HWND/同几何命中及显式封存产品门分别验证；结果见迁移台账 v347，
支持与拒绝见 [JS Grid](../development/SURFACE_JAVASCRIPT_NATIVE.md)。默认 JS/
capability 及完整股票/Loom/CSS/Canvas/权限隔离/N01–N12 仍未放行。

最终严格静态/相关回归和 3,543 文件行数门通过；六程序封存后 JS Grid 17/1 ignored、
实际产品 Grid worker 与旧 JS/Application/预算/资源/startup 门通过。2,391 源码输入
和六程序身份一致、旧 .339–.346 产品不变，相关进程残留零。完整证据、早期失败与
非验收边界见迁移台账 v347 和 closeout，默认能力仍不扩大。

v346 将 measured-content Grid 接入既有 Scene→NativeMeasure→Tree→Presentation→
DirectWrite/D2D→Geometry/Hits 链。tracks、placement、axis、item 分别负责语法、稀疏
占位、轨道求值和尺寸/基线约束；grid_measure 只持双阶段测量与每轮有界缓存。
Tree/快照的 grid_cell 是图片百分比/比例的包含块，不另存第二份几何或动作权威。
没有浏览器、新线程/依赖或股票业务翻译；JS CSS Grid 与默认 capability 尚未开放，
完整迁移仍未完成，精确支持/拒绝与预算见 [Grid 合同](../development/SURFACE_GRID.md)。

最终 Grid 专属 22 项（宽筛选 25）、Surface 676/14 ignored、SurfaceWindow 217/11
ignored 及相关回归/严格静态通过；实际 debug 产品重建后封存六程序，Grid/Application/
JS/资源/startup 同字节门通过，2,383 源码输入与六程序身份一致。旧 .339–.345 封存
产品不变、相关进程残留零；完整结果和早期失败见迁移台账 v346，不代替完整产品验收。

v345 的 constructed CSS/inline cascade 在原 QuickJS child 内计算，再由独立 projection
转换为真实 type/layout/style/visible，不把 CSS 字符串直接塞进保留未知键的 Scene。
css_values/selectors/declarations/sheets/projection 分别拥有准入、匹配、声明、采用/层叠
与呈现转换；原 NativeMeasure/DirectWrite/D2D/Geometry/Hits owner 不变。跨帧重新计算
class/data/inline/sheet 更新，隐藏不以 opacity 冒充；显式继承覆盖按钮原生默认。
没有新浏览器、线程、宿主 I/O、依赖或默认 JS 能力，完整 Grid/DOM/CSS/Canvas、真实
股票与 N01–N12 仍开放。精确子集、资源预算及真实产品门见
[原生 JS/CSS](../development/SURFACE_JAVASCRIPT_NATIVE.md) 和迁移台账 v345。

最终 v2 的开发 JS 60/4 ignored、封存后 CSS 12/1 ignored、Application JS 七项和
实际产品 CSS worker 通过；2,373 源码输入、六程序与旧 .339–.344 产品摘要一致。
真实 HWND/生产帧的几何、DirectWrite/D2D 像素和隐藏命中验证不代替完整产品/Loom
或全量迁移验收；v1 未推广，完整回归/失败及预算边界见本轮 closeout 与迁移台账。

v344 将同一原生 JS 文档模型按状态责任拆为 tree/attributes/selectors/markup/json
五个私有工厂，engine 直接组合到原 bootstrap，不添加浏览器、全局 bridge 或插件
业务翻译。tree 唯一持有节点 identity、live 集合、Fragment/移动准入；bootstrap
仍拥有 listener/生命周期与有界原生投影。私有状态/数组无原型，collection 操作
提前捕获，协议只复制自有数据再编码；外部查询/snapshot 是副本而非私有容器。
Application 沿原 HWND→listener→FIFO→收敛→同 child update 消费通用 fragment/
反射/委托，授权、网络、线程/句柄 owner 不变。合同见
[原生 JS owner](../development/SURFACE_JAVASCRIPT_NATIVE.md)，验证见迁移台账 v344。
该文档子集不等于 CSS/Canvas/完整 UIA、真实股票、默认产品 JS 或 OS 权限沙箱；
完整迁移仍 active，不因新增标签/测试计数放行。
最终 v2 的 JS 48/3 ignored、同字节 Application 七项、实际产品文档/旧 worker 与
资源/启动门通过；2,362 源码输入和六程序/旧 .339–.343 产品身份一致。详见迁移台账。

v343 补入显式受限 JS 的 Application 输入/收敛/update 链：原 HWND 候选经身份/命中/
焦点门进入持久 Runtime listener，宿主编码的不可变 emit 仍走原批次 FIFO、唯一 owner、
ACK/确认/显式接续。不建立 JS HTTP 旁路，不重新挂载或替换 child；runtime 来源绑定
原事件，不从响应升级能力。完整服务端收敛只产生暂存权威快照；同 child update 的
DOM 成功交付并再核授权/绑定/显示后才推进版本和原 Pending 收据、释放输入/输出门。
原回复事件跨结算点保留最后唤醒，关闭只等线程终态，无新增等待槽、依赖或线程。
业务 state 与生命周期字符串独立解析；恢复证据沿旧格式。完整边界见
[原生 JS owner](../development/SURFACE_JAVASCRIPT_NATIVE.md)，验证见迁移台账 v343。
默认 JS、完整 DOM/CSS/Canvas/图片宿主、权限沙箱、产品真实 Loom 与完整迁移仍不放行。
最终 v2 的 JS 定向 37/2 ignored、封存后 Application JS 七项、产品私有 worker/
预算/资源/startup 门及严格静态/版本/材料/行数门通过；六程序与旧 .339–.342 摘要
不变，2,352 项源码输入冻结/构建后/运行后相同。固定 TLS peer 的 Application 生产
路径与实际产品私有 worker 分开验证，仍不是默认产品菜单/真实 Loom 或完整插件。

v342 的 Application JS 挂载由 authorized::Runtime 持原 Grant 的独立串行租约和同一
child，prepare 复用原获取/求值门；单槽邮箱与后台等待不把阻塞调用带到 UI。
AuthorizedWindow 仍持唯一附件 owner，隐藏构造/JS 接缝提取为明确子模块。初始
DOM 完成才允许 active，Active ACK 绑定同步完成才可 show；关闭不 join，完成
句柄保留至收割。空闲撤权/资源到期不再依赖 UI refresh 才终止 child。回收/排程
先于生命周期分类，失败保留附件证据且不自动重放。输入/异步快照更新桥尚未接通，
默认能力仍关闭 JS；不是完整 DOM/权限沙箱/真实 Loom 验收，详见
[原生 JS owner](../development/SURFACE_JAVASCRIPT_NATIVE.md)。
最终 v3 的 JS 定向 31/2 ignored、同字节 Application JS 四项、实际产品私有 worker
及资源/启动/裸入口门通过，六程序与旧 .339–.341 摘要不变；源码冻结/构建后/运行后
一致。v2 身份失败与既有测试原字节恢复证据保留，不把当前子链提升为默认能力、
完整产品真实 Loom/输入/FIFO/迁移验收。详见迁移台账 v342。

v341 在原设备 Grant 与资源 HTTPS owner 上接入后台 JavaScript 入口获取/真实 mount。
脚本入口只来自当前快照，精确租约/MIME/大小/摘要门通过后执行；Ready 仅同 Grant
移交，Mounted 每操作复核版本/生命周期/撤权并保留原请求租约。空闲撤权仍依赖未来
Application 等待集合/refresh，所有执行调用只供后台 owner；不新增轮询，不放宽图片
门或默认 JS 能力。等价 payload 计数修复不改变预算。完整 owner、已验范围和缺口见
[原生 JS](../development/SURFACE_JAVASCRIPT_NATIVE.md) 与迁移台账 v341。

v340 为显式原生文本/按钮 DOM 开发子集加入真实持久 QuickJS-NG child。Session 拥有
进程/管道/权威快照，worker/engine 只在 Job 准入后执行；bootstrap 拥有 DOM/listener/
生命周期，先处理可运行 Promise jobs 再收集输出。原 SurfaceWindow/Presentation
负责投影呈现/命中；父侧依据权威 scene 复核 emit 并填写 binding/revision。不改变
默认 attach/capability，不替代 WebView，不声称完整股票/CSS/Canvas 兼容；Job 不是
OS 权限沙箱。详见 [原生 JS owner 与预算](../development/SURFACE_JAVASCRIPT_NATIVE.md)。
严格五锁 OSV 首轮发现后，仅定向升级 native rustls-webpki 0.103.16 / webpki-roots
1.0.11；复扫 native 无返回匹配，但其他锁仍使整门失败，不授予产品安全或发行批准。
候选许可证补充归 javascript_licenses，仅消费精确 rquickjs-core/sys 0.14.0 对应的
两份摘要绑定原文；全图/预算/输出仍由 licenses 拥有，不给任意缺失包提供回退文本。
开发侧定向/静态门及实际产品私有 worker 门已通过；封存产品资源/启动/入口门
2/1/7/2 passed，前后摘要相同。它不经过默认 Application JS 接线，不能宣称真实
Loom/完整插件或整个迁移完成；最终证据与历史失败边界见迁移台账 v340 收尾。

v339 续进仅补实际产品构建和运行证据，不新增生产 owner。封存产品的 build_resources/
exe_resources/startup 正式门 2/1/7 项通过，原期限不变；菜单真实服务链仍未通过正常
前台准入，不能继承库级六请求证明。桌面/Job/UIA 驱动都在 linshi，生产菜单 guard、
授权 owner、默认能力、源码和版本不变。修正后夹具未整链重跑，旧失败/证据链问题
与完整迁移门仍开放，详情见 [迁移台账](../development/WINDOWS_NATIVE_MIGRATION.md)。

v339：Application remote_live/resources 下新增显式 ignored 的真实 Loom 测试模块，
沿生产授权 owner 取得新 Ed25519 设备会话，并经过 surface_start/poll/AuthorizedWindow
完成真实 attach、active、原生控件/绘制和关闭后的 inactive ACK。TLS 由独立 loopback
夹具终结，原始业务响应来自隔离内部 daemon，三次业务均为 Device/独立 nonce；没有
伪造凭据/Outcome/ACK、修改系统信任或生产 owner。自有窗口屏幕外显示且不激活，
不发全局输入或执行股票动作。新测试 199、接线文件 239 有效行；真实门及相关回归/
严格静态门已过，不代表实际产品 EXE/菜单入口、用户在线 Loom 或 JavaScript 验收。
生产实现和默认能力不变，旧 UIA/启动/证据链 partial 与完整迁移门仍开放；范围和复现
前置条件见 [原生构建](../development/WINDOWS_NATIVE_BUILD.md)。

v338：Loom 声明式快照的四项可选数据属性在 surface_scene 边界接受 missing/null/
object；仅前两者投影为空 Map，原始快照不变。严格 events/children、类型和有界
数据门保留，HTTP/设备/授权/资源 owner 不变，没有新线程、队列或依赖。真实股票
Art declarative fallback 的 15 节点可由公共库原生窗口完成两次绘制，中文描述句柄
随关闭撤销；未绑定呈现窗口仍无编辑控件、local edit/pointer 关闭。外部驱动的
两个错误假设已按实际 Win32/挂载合同纠正，失败证据保留，不改产品权限凑绿。
Scene 11、非 desktop Surface 606/10 ignored、Application attach/image 两项及严格
静态门通过；隔离管理员 HTTP 与旧内部 daemon 不替代设备 HTTPS/Application
实际 daemon 链或 JavaScript。旧完整库 UIA/启动/证据链问题仍开放，本轮未重编
产品 EXE。详情见 [迁移台账](../development/WINDOWS_NATIVE_MIGRATION.md)，证据在
linshi/otter-native-v338-regression；完整 N01–N12/资源长稳/供应链与最终手测不放行。

v337：tool_panel_content 只改变隐藏 pinned 工具面板 BUTTON 的首次使用准备；panel
HWND、字体、图标、截图保护与捕获 registry 仍立即存在，普通隐藏工作区仍 eager。
可见 Application 的 window_state 在原事务提交前完成可失败准备，包含 Live freeze，
恢复非 pinned 所有权交换不增加可失败工作。临时 Vec<Window> 负责失败回滚，全部
成功后转交原父面板；BUILDING/READY 防同步重入，首次刷新不采信未投影的缓存。
SWP_SHOWWINDOW 的定位消息统一准备/失败取消展示；无新线程/队列/轮询/依赖，
关闭仍沿原资源 owner。隐藏 pin 的 UIA 子树首次使用前不枚举 BUTTON，不冒称完整
UIA 兼容。七项新 HWND 回归（含真实 ShowWindow）、面板初轮 28 项和 composition
102 passed / 4 ignored 通过；READY 只标记集合创建，既有 best-effort 刷新/重入补刷
与完整 UIA/TAB 边界保留；
完整运行产物/矩阵另绑定，证据在 linshi/otter-native-v337-regression。旧失败、完整
N01–N12、实际用户 Loom/JavaScript/4K/内存/长稳/供应链与最终手测不因此放行。

v337 最终面板 29 项、严格门、Shader 固定三轮 3/3 和六套 EXE 26/26 均有完整日志，
运行九摘要不变。追加诊断后原 gates 索引及四份诊断材料出现全零/NUL 尾段；原因
未证，原件保留，不补造索引/缺失结果。本轮仍 partial，独立现态复核入口在
linshi/otter-native-v337-recovery；不将完整日志的一部分当作完整原始证据链。

v336：默认钉图的 D3D11 device 由 composition_device 按 UI 线程弱共享，健康检查后
复用，hardware→WARP 回退不变；最后一个 surface 租约结束不由缓存保留 COM 资源。
composition 仍独占每窗交换链/D2D context/DirectComposition 树和透明度，私有资源
先析构再释放设备租约；Live with_device 继续使用捕获设备，不混用默认池。
四项真实多窗 GPU/像素/释放和五项 owner 回归通过；设备失效是状态机注入，不是
真实显卡复位或 RAM 节省证明。composition 102 passed / 4 ignored、严格静态/行数
门通过；首轮普通完整 smoke 仍 15 秒超时，本轮 partial。没有新生产线程/队列/
后台轮询或辅助进程，完整迁移/长稳/UIA/实际用户 Loom/发行门不变。
证据在 linshi/otter-native-v336-regression。

v335：工具面板子按钮创建/实际 HWND 几何更新归 tool_panel_children，纯 DIP 布局、
滚动、字体、绘制、命令与父窗口生命周期仍归原 owner。直接按最终几何创建按钮并
保留最后校正，仅跳过实际位置/大小不变的 SetWindowPos，不缓存状态或延迟 UI。
tool_panel_layout_tests 以隐藏自有窗口验证去重、纠偏、resize/滚动和子句柄释放，
回调计数指针有真实 owner，Drop 先解除回调；无全局 hook/新线程/队列。
工具面板相关 22 项、Shader 65 passed / 15 ignored、严格 Clippy/check/fmt 通过；
固定三轮无计时诊断产品正向门 2 passed / 1 failed，原 15 秒期限仍有超时。
额外 build 导致 EXE 身份变化的断言保留，新字节先封存另验，不覆盖失败；本轮
partial，不结清旧 UIA/启动或完整迁移。证据在 linshi/otter-native-v335-regression，
候选信任、默认关闭、SDK/材料批准和发行边界不变。

v334：实际产品 Startup 接入显式固定 Shader 候选，默认 `Installation=None` 不变。
shader_candidate/其 files/acl 子模块分别拥有 pin/同句柄摘要、逐级本地盘租约与
owner/DACL 门；TrustedBundle 持有文件租约，固定候选 child 自行重验。startup_entry
在截图/建窗前解析，只把 Installation 送给原 Application；不扫描、不安装、不
持久保存，也不让父进程加载 ANGLE。application_shader_startup 的显式 smoke 复用
原菜单/输入/认证取码/缓存/child/commit owner，不增加生产线程或队列。
独立 TLS peer 的实际 EXE 两次 child/正式 PNG与缓存有通过运行，但最终默认
计时正向门在原 15 秒超时；一次计时诊断通过不替代正式失败，本轮 partial。
最终 Shader 65 passed / 15 ignored、严格 Clippy/check/fmt/版本/3,459 文件行数
与摘要绑定六套 EXE 集成 26 项通过；八新文件 53–178 行、修改 Rust 最大 498，
无新例外，九文档 UTF-8/链接通过，不清除旧失败。逐级 ACL 不替代历史句柄/OS
信任前提；完整用法与限制见 [Shader 候选](../development/SHADER_LOCAL_CANDIDATE.md)。
本地开发运行不等于可信安装、真实用户 Loom、完整桌面/迁移或发行批准。

v333：ANGLE 构建之后的本地材料消费归 scripts/native-candidate，和 native-angle
builder 分离。angle_intake/angle_notices/angle_sbom 负责只读准入、原字节声明与
材料清单，angle_materials/evidence/verify 负责有预算的复制、证据关联和外部预期
摘要独立核验。复用现有路径/名称空间/JSON/流式写入，不增加应用线程或安装 owner。
159 清单文件/109 声明的真实候选、独立核验、官方 schema 和直接消费 payload 的
硬件/WARP 外部驱动通过；不执行快照源码，不默认加载 DLL，也不把本地材料 SBOM
当成完整 DLL 依赖或许可批准。完整来源/CRT/声明/可信安装和产品验收仍开放；
原生 Rust、产品 EXE、builder 与默认能力不变，详见
[ANGLE 材料边界](../development/ANGLE_TOOLCHAIN.md)。
105 Python/9 Node 合同、fmt/版本/3,451 文件行数门通过，九新文件 30–120 有效行、
无新例外；34 Python AST/七文档 UTF-8/本地链接通过，原生 1,767 项输入/旧 EXE
及默认能力不变，相关进程零残留；不冒充完整桌面回归。

v332：构建侧 build_fixed 编排固定 GN/Ninja；build_inputs 拥有外部 stage 预期摘要和
构建前后精确集合，build_job 拥有 suspended/Job/resume、标准句柄和有界清理，
build_binary 拥有不加载 DLL 的 PE/必需函数可执行地址/普通及 delay-import CRT 门。
复用原路径/环境/staging owner，不新增应用线程、队列、安装器或 capability。
Job/mutex 不提供权限沙箱/跨 session ACL 信任，构建成功不代表完整源码/工具/许可
闭包、运行放行或产品安装。调用与输出见 [ANGLE 隔离构建](../development/ANGLE_TOOLCHAIN.md)。

最终源码绑定的新树实际完成 12,569 文件暂存、GN 513 targets/135 files、504 步
完整 DLL 编译（558.765 秒）及无工作/零诊断；PE/静态 CRT 和新显式夹具硬件/WARP
外部驱动门通过，不是实际产品 EXE/Loom/完整桌面。81 合同、fmt/版本/3,442 文件
行数门通过，旧弱 PE 门构建已主动中止并保留。原生输入/用户程序/默认能力不变，
66 个 gitlink、完整源码/工具/许可、可信安装和完整迁移/旧失败门继续开放。

v331：构建侧 stage_build 将固定本地 Git 对象和工具归档组合为外部构建树，复用
prepare_profile；stage_pins/inventory/git/archives 按身份、名称空间与预算、对象和
归档分责。Git 只读消费不跟随缺省 gitlink、下载或执行 hooks，timer 和管道由同一
owner 回收；没有新应用线程/队列/安装器。最终暂存树 12,569 个构建文件与首轮相同，
最终树真实 GN 的 504 命令计划也逐字节相同；首轮树完整 DLL 编译和新显式夹具的
硬件/WARP 外部驱动门通过，不冒称最终树重编译或实际产品 EXE/Loom 验收。
79 个 gitlink 仍有 66 个未提供，完整默认 source/tool/许可闭包、编译执行 owner
与可信安装未完成。54 合同、fmt/版本/3,434 文件行数门通过，默认安装/capability、
原生 Rust、用户程序及完整迁移边界不变；见 [ANGLE 入口](../development/ANGLE_TOOLCHAIN.md)。

v330：构建侧 gn_profile 拥有固定 Clang/lld+MSVC STL 的 x64 D3D11 Release 静态
CRT GN overlay，prepare_profile 复用原工具链 adapter 生成外部目录与绑定回执。
不下载/暂存完整源码或执行工具，不改应用 Win32、Shader、JavaScript 架构。真实
新实验树已消费仓库输出，513 targets/504 命令的完整两 DLL 编译与 Ninja 原生 GN
再生通过，toolchain 前后字节一致，无生成文件 prefix patch。C++/资源头依赖
5/24/24 与失效/恢复门通过；Clang 排除系统头，不据此声明全部工具内容闭包。
新 DLL 的 x64/52 必需导出/无动态 MSVC CRT 及公共库外部驱动硬件/WARP 像素/
child 门通过，完整编译日志无 warning/error 诊断；不是实际产品 EXE/Loom/UI
验收。完整 staging/内容/许可/可信安装和完整迁移仍待完成，默认 Installation=None、
原生 Rust 与用户程序不变。入口见 [ANGLE 工具链边界](../development/ANGLE_TOOLCHAIN.md)。

v329：构建侧 prepare_toolchain 增加精确无路径 marker 的空参数还原，修复真实
Windows GN exec_script 丢失空 argv；不扩大 runtime_dirs/PATH 权限。25 项 adapter
合同通过（含不继承的 VSLANG 请求，不保证英文资源存在）。授权 SDK 的真实双架构
工具链探针、隔离树 GN 111 targets、完整 DLL 计划 402 条 dry-run 命令及四个静态
目标编译通过，不称 archive 自包含或 warning-free。中文 MSVC 的初次头依赖为 0，
实验 post-GN prefix 适配后为 121，并通过真实头文件 mtime 失效/恢复门；该树仍是
linshi 实验，不是仓库完整 staging/build owner。D3D11 Release /MT 的无 WDK/
runtime staging 接缝有精确 profile 断言，不改变应用 runtime 或完整迁移目标。
后续固定资源工具已在 linshi 获取/展开，完整两 DLL 构建、52 个所需导出与
121/24/24 头依赖门通过。新鲜公共库外部驱动的硬件/WARP 像素/隔离 child 重放
通过，但不等于实际产品 EXE 或正式安装；44 条 D9002 和首轮直接诊断的立即卸载
断言失败保留，上游 lazy loader 模块引用由进程退出兜底。仓库完整 staging/GN
再生、完整内容/许可闭包、可信安装/长稳/最终手测继续未完成；默认
Installation=None、原生 Rust 与用户程序不变。详见
[ANGLE 工具链边界](../development/ANGLE_TOOLCHAIN.md)。

v328：构建侧新增 [ANGLE 显式工具链 adapter](../development/ANGLE_TOOLCHAIN.md)，
责任位于 scripts/native-angle，不改变应用或 Shader runtime。它在全新外部目录为
固定上游 setup_toolchain 接入显式清单；缺失/冲突即拒绝，无系统 SDK 发现回退，
不继承 ambient include/lib/path，并移除空 PATH 段的 CWD cl.exe 抢占。输出包含
固定 gn_helpers 与自有模块，22 项合成数据/实际 Python 消费/CLI 合同通过；不声称
已集成完整 GN 暂存树、真实 SDK、DLL 构建/重放、完整内容/许可/工具闭包或可信安装。
原生 Rust、用户程序与默认 Installation=None 不变，完整迁移/长稳/最终手测仍未完成。

v327：Shader 的两个配置捕获分支在写强制 mode/output_mode 前拒绝同名真实图片
端口，不依赖参数声明，不改变普通 Art 名称/图片优先规则；不丢图片、改名或取消
模式。拒绝仍归 loom_art_config，application_parameters 在 worker 启动前传播原
配置错误；没有新运行 owner/线程/队列/句柄。新增应用测试归原 Shader 隐藏夹具，
验证 ready 状态下错误先于准备，旧请求/preview/正式输出不变，不改显式取消语义。
新回归先红后绿，协议/配置 13、Shader 60 passed / 15 ignored，硬件/WARP 平台
16 次真实 child、最终摘要绑定六套集成 26 项及 Clippy/fmt/3417 文件行数/diff
通过，修改 Rust 均不超过 500 有效行，源码/EXE 不变。证据位于
linshi/otter-native-v327-regression；定向测试有交集，不代表完整桌面库或实际服务。
旧 UIA/间歇超时证据、默认 capability 和正式供应链/完整迁移验收边界保留，未组包或推送。

v326：下层 session_file_trace 只观察原文件事务，沿既有显式计时开关默认关闭；
guard/write/replace 的文件句柄、完整比较、取消、sync_all、无覆盖发布和恢复 owner
不变。输出无路径/内容/错误文本，写诊断失败不改变事务，无新线程/队列。一次观察
11.547 秒正常退出、231 个操作配对完整，文件操作合计 183.739 ms，不是性能修复或
旧超时根因证明。文件事务/图片保存/真实保存调用方 29/5/19 项通过，0-test filter
不算覆盖；正式六套集成共 26 项及最终 EXE 摘要绑定无编译复验通过，期限未改。
Cargo 测试重编译导致的初次 EXE 身份检查失败保留，诊断与最终 EXE 独立绑定。
Clippy/fmt/3416 文件行数/diff check 通过，源码不变；证据位于
linshi/otter-native-v326-regression。未重跑完整桌面库，不宣称旧间歇失败已修复，
默认 capability/发行/实际服务及完整迁移验收边界不变，未组包或提交推送。

v325：Application 拥有独立有界 shader_recipe_cache（16 项/64 MiB payload）；
真实远端取码、本地派生结果及正式事务全部成功后才收录原始配方。缓存按 Unit、
Selection、Art、catalog 实例和参考内容隔离，不按参数 revision 分裂。命中继续
走原输入准备/请求/配置/来源/拓扑提交门，shader_prepare 重新绑定当前图片和参数，
shader_execution_job 启动本地任务而非 Session/Delivery；不发送远端执行/控制。
输入内容摘要仅由 loom_input_batch 子模块在原输入 worker 内按需计算，不替代来源
凭证。Recipe Arc 关联字节租约，驱逐仍在使用的条目不会提前归还额度，不持有输入、
像素、GPU 或子进程。worker 两槽与统一 Tasks 一致，真实 child 仍单容量且有期限。
sync_capture_windows 原样移到现有 application_window_state owner，无行为变化。
最终源码 Shader 定向 57 passed / 15 ignored，应用 16 次真实 child（8 次认证取码/
8 次缓存无新连接）、双缓存四场景/六次成功 child 和容量释放前取消回收、输入
worker 15 项、严格 Clippy、fmt、3415 文件行数和 diff check 通过；修改 Rust
模块均不超过 500 有效行。原生源码前后一致，最终 EXE 另绑定六产品/后端、两本地
平台和两种私有入口拒绝。资源/EXE/会话启动 2/1/9 passed，startup 则为 5 passed /
2 failed，隐藏窗口及透明钉图各在原 15 秒期限超时；无 stderr，根因未确证，
两项各一次计时隔离诊断仍超时：隐藏窗口 application 12766 ms 成功结束并打印
基础绘制/重建标记，透明钉图最后计时为 groups begin；不推断单一阻塞点或与缓存/
v324 前台失败同因。不放宽测试或以 Shader 成绩放行。首轮 Clippy 大枚举失败已通过可选元数据 Box
修正，原始失败及 partial 回执位于 linshi/otter-native-v325-regression。
完整桌面回归本轮未重跑，保留 v324 的 UIA/外部前台失败且不操纵用户应用；默认
安装/capability、正式供应链/实际服务和完整迁移边界不变。本增量不代表常驻 GPU
renderer、编译程序缓存或完整连续实时预览，未组包或提交推送。

v324（定向验证通过，完整桌面回归未通过）：公共字符串可显式引用当前 workspace 的 Unit 图片，包含
历史未声明参数。application_shader_inputs 只筛选身份/正式 Graph 输出，唯一
图片来源链仍为 application_image_inputs→Source→PreparedBatch；Art preview
不能提供正式来源，Sticker/Live 的解析和所有捕获/预算/取消 owner 不变。同轮
resolved Params 决定图片、远端配置与 Runtime；connected 图片优先，只把实际
使用引用及其上游加入 freshness，避免旧 inline 值和无关 Unit 造成分叉/取消。
loom_art_config 三种编码接缝省略已进入 inputs 的同名 parameters，符合本地 Loom
adapter 的静态合并合同；Runtime 仍消费完整 resolved。framework 的 params/inputs
分区及图片转路径由工具声明/实际 Art 决定，不据透传证明真实服务业务兼容。
新增测试复用原应用/TLS/child 夹具，不新增线程/队列/缓存。当前源码 Shader 定向
50 passed / 14 ignored、应用平台 16 次真实 child、严格 Clippy、资源/EXE/会话
启动/startup 2/1/9/7、fmt、3411 文件行数和 diff check 通过；修改 Rust 模块均
不超过 500 有效行。源码前后相同，最终 EXE 已另绑定摘要复验六项产品/后端、
两项本地平台及两种私有入口拒绝，无本次原生进程/运行临时文件残留，Node 缓存保留。
完整库两次分别 3181 passed / 5 failed / 73 ignored 和 3185 passed / 1 failed /
73 ignored；首轮 wire 旧期望已修正，四项 Surface 隔离通过但根因未确证，UIA
select 隔离仍失败且记录外部应用成为前台。未操纵该用户应用，不将隔离成功或
外部前台证据当作所有失败已修复。完整库间生产源码未变，最终测试类型别名整理
后未重跑完整库；partial 回执位于 linshi/otter-native-v324-regression。默认安装/
capability、实际服务及完整迁移边界不变，未组包或提交推送。

v323：shader_parameters 选择公共 image_link/file 参数与同名准备输入，原 Runtime
冻结名称；shader_prepare 在既有执行 worker 内消费同一 PreparedBatch 的 PNG，
优先于手动/响应纹理，不增加图解析、来源权限或线程/队列。主图、reference、保密、
内部和禁用字段不自动升级为纹理，缺少已冻结输入直接失败；合并数量及像素预算不变。
内联 PNG 输入复用 loom_image_value::png_base64 和原 Formal 准备，避免低 alpha/
透明 RGB 经 PBGRA 再量化；显示图和原 PNG 同源且均计预算，非 PNG 转换维持原状。
新增应用测试子模块复用原 Class/COM/TLS/真实 child，验证图纹理与来源/拓扑拒绝；
完整库复测 3181 passed / 73 ignored，Clippy、资源/EXE/会话启动/startup 2/1/9/7、
fmt、3408 文件行数与 diff check 通过，原生源码前后一致，最终 EXE 另绑定摘要复验；
证据位于 linshi/otter-native-v323-regression。首轮 6 项桌面/UIA 失败保留，同源码
复测通过不宣称 UIA 间歇断言已修复。正式供应链已核验六个固定源码 checkout/官方
tree，并静态审查 bootstrap/递归 DEPS；固定 SDK common/x64 NuGet 包签名和 ZIP
布局已核验，但未执行包内工具、接受新增许可、安装 SDK、执行上游脚本或构建 DLL。
标准 vcvarsall 分支仍从注册表重选 SDK，不能把隔离 GN 路径冒充工具链已适配。
默认安装/capability、实际服务、任意 Unit 图片引用、实时缓存和完整迁移边界不变。

v322：Shader 主输入优先级归 loom_capability_runtime 的同一选择合同。应用图输入
owner 只在可信 Shader 且无可用主输入时加入自身 src，复用 PreparedBatch 和原
输入 worker；普通 Art 不变。来源只能是 editor.layers 的真实 raster base，
不能读 Results.display/resolved display 或 preview-only 的 Canvas.image。
Source::capture_base 共享底图、携带真实文档凭证，编码不二次叠加自身标注/裁剪，
未完成编辑/接管、缺失参考图及文档替换仍由原准入/新鲜度门拒绝。
连接优先与自身回退均已进入原真实 TLS/child 应用平台门，硬件/WARP 各覆盖菜单
及最终参数触发；完整库 3177 passed / 73 ignored，Clippy、资源/EXE/会话启动/
startup 2/1/9/7、fmt、3407 文件行数及 diff check 通过，原生源码前后一致，最终
EXE 另绑定摘要复验；证据位于 linshi/otter-native-v322-regression。自身底图仍走
既有 PBGRA→PNG，不据不透明样例宣称源资产原始 RGB/profile/方向的完全保真。
没有新增线程/队列/生产模块或发行依赖；默认安装/capability、实际服务与正式
供应链、通用图片/Unit 参数、缓存渲染和 N01–N12/JavaScript/长稳/手测边界不变。

v321：应用参数 owner 在同一次 resolved Params 中冻结取码 Configuration 和本地
Shader Runtime，Pending/Execution 单次携带 Runtime 到统一 Tasks::start/Job；
原输入准备、网络/租约、两个槽、完成通知与应用正式事务不增加 owner。独立
shader_parameters 持有有限数值/布尔和 PNG 覆盖快照，不采纳路径/HTTP 或秘密字段；
shader_prepare 从同份 PreparedBatch 按声明及旧 fallback 选主图，参数不能覆盖 input。
菜单/最终参数触发的实际 TLS fixture→真实 child→pending_windows 提交已组合，
硬件/WARP 各验证两次精确 PNG/显示；普通门保护未安装、自动开关与配置变更。
Application 默认 Installation=None，没有 Art/环境变量/用户设置可开启；描述对象
不是供应链/ACL/许可证验证。隐藏应用和测试 EXE 不等产品入口、阻塞主循环或实际
Loom 服务验收；通用图片/Unit 参数、自身图片 fallback、缓存式实时渲染、正式
供应链及完整 N01–N12/JavaScript/长稳/手测仍待完成。完整库 3174 passed /
73 ignored，Clippy、资源/EXE/会话启动/startup 2/1/9/7、fmt、3406 文件行数和
diff check 通过，原生源码前后一致，最终 EXE 另绑定摘要复验；证据位于
linshi/otter-native-v321-regression，未组包、替换程序或提交推送。

v320：Shader 的真实取码和本地派生结果共用原认证远端执行 worker、两个槽和清理
owner；Session 成功且 lease 释放后，shader_fetch_render 在同一线程消费原批次，
经 shader_payload 的主端口/首 content/有界字段合同和 shader_prepare 的同源 PNG
纹理准备，再启动既有受限 child、准备 PNG、安装到真实 Delivery 的派生批次。
不伪造远端身份/lease，不将本地失败回写成远端 cancel，不新增队列/线程 owner。
image_decode 共用像素转换但明确区分显示 PBGRA 与 GPU straight RGBA，PNG 尺寸及
剩余总预算在分配前核验；不访问 Art URI 路径/网络，可信 bundle 独立于配方。
真实浏览器完整 PNG 纹理矩阵零差异，TLS fixture→原样 Loom GLSL/LUT→真实 child→
正式图像已有硬件/WARP 组合门；实际 Loom adapter 的内联输入通路另以源码核查。
fixture/测试 EXE child 不冒充实际服务、产品用户入口或可信正式安装；通用格式/
profile/方向、动态参数/字符串纹理、正式供应链和 N01–N12/长稳/手测继续待完成。
v320 完整库 3169 passed / 72 ignored，认证取码组合/纹理准备各 4 passed；Clippy、
资源/EXE/会话启动/startup 2/1/9/7、fmt、3403 文件行数/diff 通过，原生源码前后
一致，最终 EXE 另绑定摘要重验后端和本地平台门。回执位于
linshi/otter-native-v320-regression，未组包、替换用户程序或提交推送。

v319：Shader 的 owned request 在父侧共享纹理像素、拥有有界源码/绑定，并复用
wire 准入。按需 shader_execution_job 通过原受限 child 渲染，child 退出后准备
完整 Image+PNG；输入/捕获纹理/许可先于完成栅栏和原 execution_wake 通知释放。
loom_execution_work 区分远端租约 owner 与本地结果，loom_execution_tasks 仍是
唯一两个执行槽，退休中/未领取结果也占槽。本地结果进入原 Application 主循环和
正式事务，重验请求/来源、原配置/拓扑绑定，失败保留旧端口，替换请求不被旧报告
结束；本地不创建远端 identity/delivery/lease。真实硬件/WARP child 的共享任务
与隐藏应用主循环路径均验证完整矩阵 PNG/显示精确；确定性七场景另证变更/取消。
这些不接通用户侧 Shader 获取/staging，不批准测试 bundle 或声明 Shader capability；
来源凭证不证明任意纹理同源，实际取码/纹理构造和可信正式供应链仍是下一边界。
v319 完整库 3160 passed / 71 ignored，显式本地平台 2、原后端平台 6 passed；Clippy、
资源/EXE/会话启动/startup 2/1/9/7、fmt/3395 文件行数/diff 通过。源码前后一致，
最终 EXE 另绑定摘要重验平台门；未组包、替换用户程序或提交推送。

v318：正式图像使用 desktop::loom_image_value 同时共享 PBGRA 显示像素与可选原始
PNG，避免 Shader straight RGB 经显示像素反预乘后再次量化。Result 的正式别名、
显示/中继/传播解析、下游输入、会话保存和恢复统一消费该 owner；预览不获得正式
输出权。输入和保存仅在原后台 worker 编码，有 PNG 就复用原字节，无 PNG 则用
export_raw；恢复仅解码有界内联字节并共享重复引用，不访问文件路径或网络。
原 OutputStamp/请求代次/失败原子性保持，逻辑端口预算保守计入像素+PNG 的别名。
真实 owner/输入 worker、会话保存恢复和隐藏 HWND 应用图定向已验证；新 PNG 图
用例不重复 TLS 执行。未新增线程/队列/编码器，本地异步 stamped Shader 事务与
应用调度仍待接通，Shader capability/pure-shader 拒绝、正式供应链边界不变。
v318 完整库 3155 passed / 69 ignored，Clippy、资源/EXE/会话启动/startup 2/1/9/7、
fmt/3389 文件行数/diff 通过，前后原生源码身份一致；未组包或替换用户程序。

v317：desktop::shader_image 将受限 child 的 raw Frame 准备成完整的 PBGRA Image
和 PNG。它独占旧 premultiplied-alpha Canvas 语义：显示 clamp 到 alpha，编码
PNG 逐步 f32 反预乘，与真实浏览器完整 256² 矩阵独立解码值精确相等；不能把
raw 当 straight RGBA 再预乘，也不能把 PNG→Canvas2D 的额外量化当 PNG 字节。
export_raw 提供显式 straight BGRA 入口，复用原有 WIC/预算/取消 owner；原普通
PBGRA 路径不变，不增加后台 owner、网络访问或发布路径。转换/配额/编码失败
及迟到取消均无部分结果，raw 输入不保留；WIC 内部工作非硬实时或总内存承诺。
真实产品硬件/WARP 的矩阵、三组 Loom 与 4K→PNG/显示门通过；正式 bundle、
应用取码/纹理和本地 stamped 结果端口仍待接通，能力报告/pure-shader 拒绝不变。
v317 完整库 3149 passed / 69 ignored、Clippy、资源/EXE/会话启动/startup 2/1/9/7、
fmt/3386 文件行数/diff 通过，前后原生源码身份一致；未组包或替换用户程序。

v316：shader_process 是父侧隔离 API，可信安装 bundle 与 Art Request 类型分离，
父进程不加载 ANGLE；shader_worker 在 settings/profile/UI 前分流，先验 Job 后读
单请求并创建 Renderer。Shader 专属 wire 保留 raw RGBA 与 typed failure，client
在完整响应、正常退出、EOF 和最终取消检查后才交付。共用 bounded_worker_process
独占 suspended→Job→resume、句柄白名单和回收；图片仍为 128/256 MiB 空环境，
Shader 使用独立 256 MiB 单 child 池及正常 15 秒绝对期限。硬件空环境失败已通过
无 Job 对照定位为缺 SystemRoot，仅从 OS API 生成这一环境变量，不继承父环境。
Job 不等于权限沙箱/GPU 总预算，驱动/内核回收无硬实时上界。真实测试/产品 child
的十二组浏览器合同与产品 4K raw RGBA 平台门已通过；这不开放 Shader 应用能力，
旧 Canvas PNG alpha 桥接、正式供应链和本地结果端口事务仍是后续边界。
v316 完整库 3144 passed / 66 ignored、Clippy、资源/EXE/会话启动/startup 2/1/9/7、
fmt/3383 文件行数/diff 通过，前后原生源码身份一致；产品 EXE 与实际构建/摘要
绑定，不把本轮开发证据当作分发或完整 N01–N12 验收，未组包或替换用户程序。

v315：shader_angle 是尚未接应用的原样 GLSL ES3 离屏执行层，通过 ANGLE/D3D11
创建无 HWND 的 EGL pbuffer，不引入浏览器/WebView。进程唯一、线程亲和 context
防止 current/display 生命周期相互干扰，API 错误永久退休，编译失败可恢复。
library 拥有可信 DLL 长度/摘要、文件锁、System32-only 依赖搜索和实际模块路径
复核，api 绑定固定 ABI，render 拥有单次 GPU 对象与逻辑像素租约，失败/取消 RAII。
真实旧 ShaderRenderer 与 Loom Color Transfer/LUT 的三组 raw RGBA 在硬件/WARP
均逐字节相同，但旧 Canvas PNG 的 alpha 语义不同，raw RGBA 不能直接交给 D2D。
DLL 只作隔离测试夹具；可信可发布 bundle、child/Job/期限、PNG/PBGRA 桥接及
Loom/图发布仍待完成。应用能力报告与 pure-shader 拒绝保持不变，不能把后端证明
当成 Shader 应用完成、分发许可通过或完整迁移验收。
v315 定向 6、完整库 3133 passed / 61 ignored、Clippy、资源/EXE/会话启动/startup
2/1/9/7、fmt/行数/diff 通过；前后原生源码身份相同、测试进程零残留，未组包或替换用户程序。

v314：输入 Backend 仍负责坐标映射，live_input_message 在最后的空间鼠标消息投递
期间复用坐标模块 DPI guard，防止 SendMessageTimeoutW 跨 awareness 再次虚拟化；
RAII 在成功和超时返回时恢复线程，键盘与原授权/身份/期限合同不变，无新 worker
或常驻状态。独立隐藏 HWND 九组合 oracle 先复现错误、再验证精确坐标及实际超时。
局部实际 WGC 复用原整窗夹具和正式 Session/输入/恢复 owner，覆盖两种窗口样式的
移动、放大与三种结束路径，验证中心像素反馈及精确输入，不冒充全图像素比较。
v314 定向 1/2/1/6、完整库 3129 passed / 59 ignored、Clippy、资源/EXE/会话启动/
startup 2/1/9/7、fmt/行数/diff 通过，前后源码身份相同，测试进程零残留；未组包
或替换用户程序。任意应用/跨显示器/资源长稳及完整 N01–N12 验收仍未完成。

v313：Ctrl+2 的选择表面与静态截图共用冻结帧事件/绘制，但仅交付 Live 矩形意图，
不复制无用的裁剪图。capture_live_selection 在冻结前绑定 Z 序候选 Target，确认
后重验原身份/边界；局部 Crop 与整窗复用 Application 的源恢复保护和会话 owner。
live_window_target::region 将同一物理裁剪交给 WGC GPU 路径与输入 Backend；WGC
严格核验 DWM/item/content 对应关系，复用原 resize 预算与关闭链，不猜边框偏移。
源失效不重新选择或回退桌面；item 几何/隐藏坐标/WARP 测试不冒充连续捕获验收。
v313 完整库 3129 passed / 57 ignored，Clippy、资源/EXE/会话启动/startup 2/1/9/7、
fmt/行数/diff 通过，前后源码身份相同；未组包、替换用户程序或公开发布。

v312：Ctrl+2 与 Ctrl+1/Ctrl+3 共用唯一注册宿主，Primary/Live/Long 三槽记录实际
注册，附属冲突互不影响；关闭失败恢复已释放槽，销毁独立清理全部槽。应用共享
Live 菜单状态，恢复事务同时核验来源、候选和共享状态，并沿原 owner 移交协议
注册新宿主。Ctrl+2 分派到现有屏幕区域 Live，不复制捕获/会话实现；旧窗口内
区域锚定与双击整窗语义仍待迁移，当前入口不是这些能力的替代完成。
v312 完整库 3125 passed / 55 ignored，Clippy、资源/EXE/会话启动/startup 2/1/9/7、
fmt/行数/diff 通过，前后源码身份相同；未组包或替换用户运行程序。

v311：活动长截图的 Ctrl+3 先通过 thread-local 弱路由转交结束请求；路由绑定
共享 capture_active 身份、HWND 属性和递增代次，通知合并为一个 PostMessage。
long_capture_dialog_finish 与完成按钮共用停止/排空/拼接状态，接收当前 worker
任务后才拼接，不开启新滚动，不复制采集或交付实现；失败保留会话显式重试，
取消/关闭/销毁退休路由。真实屏幕外 DialogBox 回归验证禁用 owner 的消息分派，
不以此冒充真实全局按键/滚动应用或自动方向兼容。

v310：surface_capabilities 的共用报告声明 image、loom_resource、remote_resources，
让要求已实现图片能力的 Art 能正常协商。报告不赋予读取权限；获取/解码/安装仍
分别经过原租约、预算、来源、同 Grant 与撤权门禁。没有增加 runtime、scroll、
touch 或资源来源类型，TLS peer 独立核验实际 attach 请求而非仅自比较报告。

v309：Application poll 将同一有状态配置回调顺序传入表单和资源调度；配置仍在
当前租约及 device 匹配后读取，失败不消费窗口意图或 owner。退出泵保留默认入口，
常规运行仍用环境配置。初次 attach 和 active/inactive 也复用这一私有接缝与原
lease/device 准入；本地 TLS 组合链验证隐藏建窗后真实 ACK 才显示、自动表单提交/快照回填、
资源/内联图片安装及关闭停用，撤权不伪造远端状态。未接入用户 Loom 验收，
也未扩大图片/滚动能力报告。smoke_timing 仅提供显式开启的分段诊断，默认不计时/输出，
不改变启动门禁、动作顺序或错误传播。

v308：desktop/surface_capabilities 统一新建/恢复的真实声明式宿主报告，加入
input/textarea/number/switch/select/slider，不扩大 runtime、touch 或资源 transport。
报告用于远端协商；本地仍按授权 binding、runtime、revision 与 Scene parser 准入。
surface_form_attach_tests/peer 使用本地 TLS 组合验证六类表单的 attach、原生消息、
batch 提交、快照回填与撤权回收，activation ACK 注入，与真实服务端验收分开。

v307：resolved display 的非 Art/mini 复制复用 Frame 快照并生成静态 editable
Content，不附带目标 Canvas capture 来源。duplicate_snapshot_with_fallback_base
只给无 raster 快照绑定显示底图，保留 logical_size/crop/elements 与 fresh identity；
已有 layers 保持原样，避免丢失栅格编辑。此路径不复制 Live owner/连线。

v306：clipboard_placement 保存 UnitId、复制时物理尺寸、鼠标偏移及源 DPI 换算的
级联步长。paste 请求提前采样鼠标，application_import 在建窗前解析当前源矩形，
仅用于 inside 判断；workspace 提交成功后才推进 session 锚点。新 copy 使用独立
identity，旧排队请求不能改写新 session。显式物理框走 create_at 与 crop-aware fit，
不重复乘目标 DPI；同 session 批内多图级联尚未支持。

v305：可编辑剪贴板携带CaptureSource值快照及共享只读原图租约，不携带源
DocumentStamp。新Editor安装后绑定来源；读取路径不替代租约。缓存与多个文档
共同保持租约寿命，最后一个owner释放才允许原文件退休；外部显示帧不混用来源。

v304：静态展开钉图的可编辑复制使用应用共享单槽 Content，像素只读共享，
Editor/历史/标注身份在每次粘贴重新建立。最终剪贴板发布区分 skipped 和
Published(sequence)，只在全部格式转移成功且仍持锁时取得凭证；读取持锁匹配，
锁外走既有后台准备和 staged window 交付。PNG/DIB/文件外部格式保持不变。
裁剪/栅格标注/底图透明度进入新窗口；Art/显示帧/缩略来源仍降级为扁平图片。
不注册外部指针格式，不支持跨进程编辑协议，也不把可编辑内容隐式用于图片替换。

v303：全局 Ctrl+3 复用垂直长截图入口，Ctrl+1 保持唯一热键宿主，附属键冲突
不撤销主键。共享菜单呈现附属键实际可用性；恢复凭证记录两项实际状态。
正常关闭先释放附属键，主键失败时补偿；销毁逐项独立清理。恢复托盘后重验
门禁，不在注册/恢复事务中弹模态警告。不包含自动方向或录制中再次按键完成。

v302：textarea 的测量、父 D2D 背景/边框与子 EDIT 内容区域共享默认/声明盒模型；
子窗口边框内缩、圆角 region 与既有遮挡裁剪组合。声明配色由绘制帧解析，
HWND palette 在 Host 暂时移出时仍可安全读取；与 Select 共用颜色资源管理，
保留高对比回退。classic EDIT 背景 alpha 预合成不等于父场景透明，不能据此
开放完整控件能力声明。

v301：textarea 自身 textAlign 由 Mount 应用到 EDIT，正文和 placeholder 共用
实际 HWND 对齐位；更新不回写文本、不重建控件或结束组合。字段自身合同变化
仍按既有规则退休 owner；未将此接线推广为 input/number 自身样式或完整 CSS。

v300：声明式 EDIT 与 Select 使用同一个 surface_control_font HFONT owner；
Presentation 保存布局已解析的字段字体，Mount 仅将 DIP 字号转换并应用到实际
控件 DPI。EDIT 子类在默认 NCDESTROY 之后释放字体，父窗口关闭也不会泄漏。
不改草稿/事件合同，不将字体接线扩称为完整 CSS 或 capability report 放行。

v299：Live HWND 改为 WS_POPUP | WS_SYSMENU，无标题栏/系统拖边；原本地手势
继续负责移动与缩放。Surface 在帧绘制后叠加主题绿边/黄角，边角几何与输入
命中共用物理客户区，L 热区不扩成整个角方块。Alt+Space 或黄角右键进入原
系统菜单，Alt+Delete 走原 WM_CLOSE/回收策略。菜单期间不建立新几何或源
控制事务，返回后重验 owner。失败通知与持久菜单状态分离，去重后由应用使用
已有 MessageBox 提示；只停止源控制而不停止采集，不添加常驻 header/buttons。

v298：Live 本地 Alt 移动/Alt+Shift 等比拖动拥有独立捕获状态，源输入先撤销，
Session 后续控制状态刷新不能释放本地捕获。几何始终基于初始物理客户区和
屏幕指针快照；等比拖动固定左上角并复用全局尺寸预算，不改变采集源。
开始标记保护输入撤销的重入窗口，update revision 约束定位重试和失败补偿；
独立 owner 身份防止旧 mouse-up 退休新手势。生命周期/锁隐藏均显式取消，
取消保留已经完成的实际位置，不把普通几何错误升级成采集设备失败。

v297：Live 本地 Ctrl+wheel 按物理客户区围绕指针缩放，Alt+wheel 调整整体
合成透明度；组合键精确匹配，锁定/隐藏不修改窗口。两者优先于源输入并走原
取消通道，后续控制仍需既有启用流程。State 在可重入合成调用前发布期望值，
补偿绘制重新应用；Host 在首帧、暂停恢复和换帧保持透明度，不修改帧像素。
保存/回收通过静态 opacityNormal metadata 保留值并纳入 receipt 重验；冻结
在提交时采样并在静态窗口准备后重验。后续静态导出遵循原 normal_opacity 语义。

v296：Live 输出不再固定 960×640/系统默认位置。区域源保留用户物理 RECT，
窗口源读取经过身份复核的可见物理外框，作为输出客户区初始矩形。建窗仍隐藏，
先捕获排除，再按实际 HWND DPI 校正非客户区并复核客户区，成功后才启动采集。
WGC item/ContentSize 仍是帧尺寸权威；无框外观、本地手势及跨屏交互另行收口。

v295：Live 自身可打开断线选择菜单。静态和 Live 共用连线快照/分页与应用层
删除准入；系统消息不改拓扑，菜单返回后核验来源 HWND、工作区身份/拓扑及
两端锁定/模态状态，随后移除选中边。显示缓存和执行仍按原拓扑凭证退休，
不另建通知、取消或持久化机制；菜单句柄由局部 RAII owner 释放。

v294：Live 可从自身系统菜单主动连到静态/Art 目标，出向与入向选择复用
同一原生对话框与 Catalog 候选构造。应用在模态确认后重新核对 owner、目录、
锁定和事务状态再写入 Workspace；UI 回调不直接修改拓扑或启动网络执行。

v293：Live 显式刷新是固定帧事务：菜单单槽请求进入应用，后台显示任务共用
原槽位，每个 owner 最多排队一个最新帧。成功读回且来源有效才替换画面并走
现有下游通知策略；失败保留旧显示，普通流帧不触发执行，过期请求/通知退休。
通知与像素 worker 不直接联网，仍由原执行准备和执行 owner 调度。

v292：显示图不再把所有来源都视为 DocumentWindow。Live 首次输出连线显示
通过原显示 worker 取得快照，原生显示帧共享 CPU 像素，不改目标原图；Art
保留目标覆盖层。缓存随会话身份及有效连线退休，UI 只取消过期任务，完成
后领取。此路径不逐帧读回；显式刷新及快照下游通知仍待接线。

v291：Live 可以作为真实执行图的图像来源，目录仅发布 output，GPU 帧仅在
运行边界移交现有输入 worker 读回/编码。执行凭证固定帧但跟踪会话 owner：
新帧不取消执行，关闭或替换会话则拒绝旧准备和结果。运行依赖不再假定所有
来源都是 DocumentWindow；画布显示图与 Live 快照传播尚待后续接线。

v290：Live 创建/重命名分组经快照请求进入应用模态域，名称输入复用现有
shape_dialog；确认后重新检查 HWND、工作区与目录，克隆候选完成名称和成员
修改后一次发布。同步对话期间不保持窗口状态借用，不新增线程或存储格式。
真实可见输入与自绘一致性尚未验收，Live 连线/图像执行仍是后续边界。

v289：Live 分组选择经系统菜单排队，在应用循环按工作区与分组快照验证后修改
共享模型；隐藏/锁定/删除/明确不回收的批量关闭由同一 group owner 协调两类
窗口。Live 的输入授权在隐藏/锁定时撤销；本轮没有新建自绘菜单或执行数据流。

v288：graph Unit 不再等同于静态 DocumentWindow。Live 和静态文档由应用
统一登记/保留，冻结是同身份 owner 交接；最后静态宿主替换仅移除该旧 Unit，
不影响存活 Live 的拓扑。运行中的 Live 交互和执行适配仍未完整接入。

v287：用户关闭 Live 的 UI 回调只排队，后台复用保存槽准备回收记录，UI 在
来源/几何复核和真实 HWND 销毁后提交内存库，再由原 Session 回收采集/输入。
捕获源故障和应用退出不进入用户回收事务；显式放弃命令保留无帧退出通道。

v286：Live 显式会话保存使用既有保存 worker，不新增线程池或实时 CPU 复制。
UI 捕获有预算的 GPU 帧 owner，后台转成普通 sticker 内容，最终磁盘提交前
复核来源及窗口几何。恢复只恢复静态帧，不序列化或重启采集授权；Live 交互
graph 登记及直接关闭回收仍是未完成的后续边界。

v285：Live 静态化几何由 UI 线程在提交时取得，后台只读回原始帧，不缓存旧窗口
位置。LiveWindow 使用共享 Viewport 算出客户区中的图像物理区域，应用复用
静态恢复建窗入口继承位置/显示尺寸，建窗后复核几何，失败仍保留 Live。

v284：显式停止 Live 的生产路径先后台取得像素，再以创建时稳定 UnitId 准备
静态文档，工作区登记成功后才停止采集。失败不得提前关窗。Session 绑定创建时
Workspace 实例凭证，整体恢复后旧 Live 不得向新工作区发布；普通编辑不改变
实例凭证。实时 Live 的完整 graph 生命周期/保存尚未接通，本路径不伪装为已完成。

v283：Live 显式快照和输入呈现身份分离。snapshot_frame 刷新并移交最新独立
GPU 帧 Arc，不改 presented_viewport；整个帧 owner 跨后台读回保存预算。BGRA
读回只在显式请求时使用 staging，按 RowPitch 复制并保证 Unmap；关闭后不从旧
surface 伪造新快照。此处只提供像素边界，Live 到同身份持久 sticker 的转换尚未
接通，不以另建静态 Unit 代替旧版停止语义。

v282：Application 独占当前回收策略，配置 Document 保留已加载/已保存基线。
菜单仅排队；模态对话框编辑独立草稿，确认后检查窗口身份、策略和来源，再更新
运行值并调度裁剪。配置 worker 同时捕获命名、临时缓存和回收策略，提交核验
所有对应快照；自动保存差异检测包含回收策略，不因未改命名而漏写。

v281：回收站维护由会话加载/配置提交事件触发，在现有单任务槽空闲时后台裁剪。
模型返回无变化时保留会话来源身份和保存状态；有变化也只更新内存原文，磁盘
写入仍属于显式保存事务。提交凭证包括来源/工作区及当前回收策略；错误只报告
一次，不在空闲期循环重试。主循环先确认旧唤醒再处理任务，避免新完成通知丢失。

v280：显式回收关闭进入 UI 协调器。后台结果不是提交许可，必须再次核验当前
来源和编辑凭证；最后宿主在旧窗口销毁前已真实创建并登记，避免宿主计数归零。
热键转交复用恢复事务的补偿逻辑；销毁失败且原窗口仍活着时不发布原文，已销毁
则必须保留冻结记录。空宿主由 transient_host 标记，保存时按内容排除，不阻止
以后编辑或恢复。此路径不拦截通用 WM_CLOSE，不覆盖独立 Live/远程 HWND。

v279：关闭前回收快照复用保存 worker 的单 owner 生命周期，并显式标注
Recycle(unit) 用途；它不是普通磁盘保存或只改库原文的 LibraryUpdate。
旧消费者必须拒绝该结果，直到关闭协调器在 UI 线程复核 Receipt 并完成窗口
生命周期事务。真实隐藏图片窗口 smoke 验证当前编码、取消和消费者隔离；
当前仍未接用户关闭入口，不会提前销毁 HWND 或发布原文。

v278：新增关闭前的回收候选准备。它借用当前已编码快照生成新冻结记录，
从原文活动窗口/链接中仅裁掉待关闭对象，保留其他库、未知字段及磁盘 revision。
回收策略先按日期过滤再保留数组尾部，默认 15 条/不按时间过期，0 禁用对应
软限制但不绕过 4096 条导入硬限制。收藏与回收共享冻结字段处理。
该候选尚未接到真实关闭消息；发布必须在资源准备完成后重新核验编辑凭证，
并处理最后窗口、空宿主、热键及窗口销毁失败，不能先关闭后补存。

v277：冻结库条目现在展开明确的动作子菜单：回收站提供恢复/删除，参考库提供
复制/删除。删除命令与普通动作、分页导航使用不相交范围，仍携带绘制时来源
和条目身份。后台仅借用原文生成指定库删除候选，不准备图片或新 HWND；发布
复用库更新任务、编辑凭证和独立磁盘基线。不存在/重复/过期请求不误删其他条目。

v276：当前普通贴纸的收藏/更新快照与取消收藏已进入原生菜单。收藏复用完整
保存快照编码器，在后台生成当前图片/标注/几何，再仅替换 referenceLibrary；
取消不执行图片编码。保存/恢复/收藏共用互斥槽，模式明确区分，取消或编辑/来源
过期不发布，成功不关闭活动贴纸也不伪装成文件已保存。
内存编辑原文与磁盘 expected bytes 现在分离：首次库编辑保留已提交来源，
后续库编辑不覆盖该基线；普通覆盖保存仍逐字节防止外部文件竞争，成功后更新
并释放旧基线。冻结恢复/复制同样走此边界。关闭入回收站、保留策略和 Live
专用实时快照动作仍待对齐，不能将普通贴纸收藏验证视作全部冻结生命周期完成。

v275：原生弹出/托盘菜单接入回收站和参考库。索引按需在单个受控线程读取，
每页最多 64 条，分页保留根菜单/捕获，普通选择才关闭菜单。窗口回调退出后
经单槽请求交给 Application，重新核验窗口目标、来源及已有保存/恢复互斥，
复用冻结 worker 与单窗口追加事务。索引不读取图片/文件，不授权新路径；
析构取消并 join，当前有界 JSON 解析不能中途取消，最大关闭等待尚未实测。
正常关闭入库、参考条目新增/删除及保留策略仍未接线，不代表 N06 完成。

v274：冻结恢复与参考复制接入已有单任务恢复协调器，在 worker 中实例化并准备
图片/矢量资源，完成后走单窗口追加分支。请求时保留同一来源、坐标空间与原
工作区凭证，物理鼠标先转换为会话逻辑坐标再按旧规则偏移。取消/失败/过期结果
不发布；普通会话输入入口拒绝裸 Frozen，防止误入整工作区替换。原生菜单请求
尚未接线，当前通过独立隐藏 Application 验证该异步链路。

v273：冻结库新增单窗口追加事务，将现有恢复资源链准备的隐藏 HWND、annotation
身份和保留会话原文同时提交。既有 HWND/分组/连接不替换；回收站仅随成功提交
消费，参考库保留。来源 storage、工作区/窗口 stamps、目录与坐标空间过期时
拒绝并释放候选。保留原文不提前推进磁盘 revision；正式保存仍走既有完整快照
与磁盘提交机制。菜单/请求调度、关闭时冻结和保留策略尚未接线。

v272：冻结条目可准备为新身份的单窗口会话候选，保留未拥有的快照原始值，
剥离分组/节点执行所有权；共用有界序列化与原有后台会话资源准入。
候选准备不创建窗口、不替换当前工作区、不删除回收站或参考库条目，
不授权 filePath/src 访问。追加发布事务、菜单和成功后的库持久化仍待接线。

v271：Session Document 新增按需冻结库读取边界，识别 recycleBin 与
referenceLibrary 的 FrozenStickerEntry 合同；每项保留条目/快照原文，
复用 Window 的几何、透明度、缩略和捕获元数据校验。库读取不授权路径、
解码图片或创建窗口，坏库不阻止原有活动窗口恢复。菜单/复制/恢复尚待接线。

v270：对照证明 v269 的跨进程事件失败与客户端瞬态子树订阅范围有关。生产
provider 与关闭时序不变，精确菜单项订阅和稳定 desktop root 子树订阅均
能接收真实 Invoked；测试覆盖两种范围乘三种输入，缓存先按 PID 过滤，再
读取名称；缓存错误明确上报。不能据此保证被销毁菜单根上的子树订阅交付。

v269：独立进程 Invoked 订阅回归暴露尚未修复的事件交付失败：首个 UIA
Invoke 路径收到发送成功但客户端超时。键鼠路径因首例失败尚未执行；不能将
同进程事件通过或跨进程命令通过视为跨进程事件已通过。保留失败回归，生产
provider 未采用诊断延时、跳过撤销或跳过销毁清理等规避方案。

v268：菜单 UIA 验证新增独立客户端进程边界：父进程保有真实 Cascade/STA
消息泵，子进程仅接收 HWND 和父 PID，通过系统 UIA 验证归属、逻辑子树、
展开、滚动、状态与 Invoke；旧滚动节点及关闭后叶子均拒绝继续访问。此证据
覆盖跨进程封送，不代替发布包、Narrator/NVDA 或物理输入验收。

v267：菜单 ItemStatus 只读呈现已有 checked/radio 标记，禁用项仍保留状态，
根与分隔符不报告选择。未勾选且无 radio 标记的 HMENU 项无法区分普通命令
与 checkbox，故不猜测角色，也不伪造 Toggle/SelectionItem 模式。状态改变
沿用快照代次撤销旧节点及结构通知，不在读取属性时改变业务或排队命令。

v266：叶子选择在统一 activate 路径发出 UIA Invoked 通知，覆盖鼠标、键盘和
UIA 请求；通知表示菜单选择接受，不代表外层文档业务已成功。通知后复核窗口、
前台、捕获、队列取消/DPI 与过载状态，重入失效则撤销命令交付，不重放操作。

v265：菜单在控制器提交完整树/焦点后，按有界快照差异发送 UIA 焦点、结构、
展开状态和垂直滚动属性通知。无监听者不保留通知快照；系统调用前释放内部
借用，每项通知前复核 provider 生命周期，失败不重放操作。MTA 客户端已验证
真实订阅交付及处理器释放；Invoke 通知与完整辅助技术验收仍未完成。

v264：菜单可访问性将祖先行的视觉高亮与键盘焦点分开，只有最深的实际聚焦页
报告焦点。FragmentRoot::GetFocus 沿有界弱关系汇总子页焦点；UIA 请求重新
聚焦同一父行也会关闭子页，避免同值快路留下键盘目标。父行身份保持不变。

v263：菜单 UIA 树将展开的弹出子窗口归到对应父行。父行可导航到子菜单根，
子菜单根可返回逻辑父行；各 HWND 仍提供自身 host provider。跨页面关系使用
代次校验和双向弱引用，不互相保活窗口/COM 对象；关闭或替换后旧关系失效。
系统 MTA 客户端已验证跨 HWND 导航、往返身份、兄弟顺序及叶子 Invoke。

v262：菜单面板根提供 IScrollProvider，支持垂直整行/整页及百分比滚动；水平
不可滚动，空或短列表报告 NoScroll/100% 可见。请求绑定当前快照代次并进入
同一 32 项队列，由控制器滚动、刷新及撤销旧行。系统 UIA 客户端已验证滚至
列表末尾、拒绝旧行 Invoke 并执行新可见行；跨子菜单树与事件通知仍待整合。

v261：子菜单节点提供 IExpandCollapseProvider，展开/收起沿同一有界队列执行。
控制器将实际子页面状态同步到快照，状态变化不撤销原父节点；COM 请求返回
仅代表入队，CurrentExpandCollapseState 不提前声称操作成功。叶子仍只支持
Invoke，禁用和过期节点拒绝请求；跨子菜单树整合与通知仍待补齐。

v260：菜单通过 WM_GETOBJECT 懒建 STA UIA provider，提供 Simple、Fragment、
FragmentRoot 和叶子 Invoke。根绑定 HWND host，行节点绑定可撤销快照；代次、
面板唯一序号和索引组成 runtime ID。COM 回调只读快照或排队，系统回调前释放
RefCell 借用，销毁先撤销节点再清理 UIA。独立 MTA 系统客户端发现与 Invoke
已有回归；ExpandCollapse、滚动模式、通知及完整辅助技术验收仍待完成。

v259：菜单可访问性增加独立快照 owner 和最多 32 项的类型化操作队列，接入
现有 Panel/Cascade 消费链。节点身份由面板生命周期、代次和模型索引共同约束；
滚动/内容替换撤销旧节点，单纯焦点刷新保留节点。Focus/Invoke/Expand/Collapse
不模拟鼠标坐标、不直接执行业务。此阶段尚未接入 WM_GETOBJECT/COM provider，
不能宣称 Windows UIA 客户端已经可以发现或操作菜单。

v258：菜单在消费面板队列、鼠标、键盘和助记符前重新读取各 HWND 的屏幕矩形。
异步平移更新命中区域及后续子菜单锚点，同时失效旧点击/滑块拖动和排队指针事件，
保留键盘与取消事件。外部改变面板尺寸仍安全取消，不冒充动态 DPI 重排。

v257：剪贴板文字导入以当前解析显示快照而非隐藏的底层图片确定准入中心。
文字编辑使用独立 Text intent，即使当前工具为 Pan 也执行原有显示接管事务；
文字与接管一起结算和撤销。全空白或无效输入在接管前退出，不生成空编辑事务。

v256：历史入口按 FEATURES 合同追加在菜单末尾，前置分隔行；动态更新使用实际
父行索引，不再假定第零行。历史文字由共享 delivery 按标注框/图片中心逐行水平
居中、整批垂直居中，托盘不再独立计算插入坐标。保留单步撤销与目标身份准入。

v255：托盘菜单加入同一历史子菜单，按展开读取、关闭释放。图片选择直接生成
新钉图，不恢复来源窗口；文字选择恢复原窗口后再核验 MenuTarget，定位到当前
图片/标注框中心后复用生产文字交付。恢复期间目标失效不执行，取消不触发恢复。

v254：文字单对象上限提高到 65536 UTF-16 单元，同时引入全应用共享的
32 MiB 正文保留预算。正文与租约同属 Arc owner，复制、撤销和快照共享，
最后一个 owner 释放才归还预算。历史长单行仍导入为一个文字对象，不拆段或截断。

v253：历史交付增加 Application 级隐藏窗口冒烟，验证文字进入真实文档并单步
撤销、过期目标/忙碌门禁拒绝，以及原编码经真实工作线程生成独立钉图且保留
来源文档。测试只构造自有条目，不调用 Windows 历史读取或改变剪贴板。

v252：历史图片在后台生成有界缩略图，菜单行携带共享引用并在标记栏显示。
缩略图像素纳入共享像素预算及历史总内容预算；正常/焦点背景分别合成 alpha。
图片导入仍移动原始编码，缩略图不参与文档交付，不复制整幅解码像素到菜单。

v251：画布菜单增加按展开读取的剪贴板历史子菜单，关闭子菜单释放 Session；
状态/条目经 track_live 刷新，选择按绘制 ID 再核验历史变化。菜单返回后继续
核验原 MenuTarget，图片作为新钉图导入，文字按行一次批量插入。当前图片仅
显示尺寸标签，尚无缩略图；超出通用文字模型上限的单行明确拒绝而非截断。

v250：自绘菜单允许异步结果替换已展开子菜单，保留根 HWND 与捕获，重建子页
尺寸和可见行。替换前检查完整树预算；成功后清除旧按下/拖动/半个字符状态，
重建失败关闭菜单。track_live 将本地更新回调接入现有消息泵，不增加后台轮询线程。

v249：托盘自绘菜单直接以临时根面板作为激活目标，隐藏的业务窗口保持隐藏。
前台申请失败或创建期间前台已改变则退出，不抢占已有捕获，不恢复主窗口。
激活成功后的消息泵与画布共用；销毁菜单后才返回命令供托盘原分发器执行。

v248：画布右键菜单切换到 native_menu 自绘级联；业务仍构造 HMENU，宿主只
消费快照，退出并归还模态门禁后再校验 MenuTarget、执行原命令。错误直接
返回，不自动重放到系统菜单。托盘隐藏 owner 仍使用单独的系统菜单入口。
模态泵支持左右键选择，按下与松开必须属于同一按钮，避免混合按钮误执行。

v247：自绘菜单的轨道、滑块绘制与命中共用几何；拖动按起点计算绝对行偏移，
越界坐标有界夹取。根捕获保持不变，越出面板的移动/松开仍路由至拖动页；
松开只结束拖动，不执行行命令。应用右键入口和历史交付尚未切换。

v246：助记符输入接入模态 WM_CHAR/WM_SYSCHAR，非导航键通过 TranslateMessage
产生字符；唯一匹配返回命令或展开子菜单，重复匹配只循环焦点等待确认。
UTF-16 半个代理字符不能跨导航/指针事件带入另一目标，owner 失效时拒绝执行。

v245：菜单面板使用由 owner DPI 创建的普通/粗体字体，保留 radio/default
展示状态，标签与快捷键独立测量和裁剪。菜单宽度依据整页条目而非当前可见
切片测量，滚动不引起宽度抖动；跨 DPI 动态重排仍未接通，不能算完整 DPI 验收。

v244：cascade::track 为自绘菜单提供模态泵，公共入口要求 owner 是当前前台，
拒绝抢占已有捕获；根面板捕获鼠标，屏幕坐标路由到最终级联矩形。退出时释放
本实例捕获/定时器并销毁面板，WM_QUIT 原退出码交还外层。应用仍须先建模态
门禁，并在返回后核验 MenuTarget；尚未替换现有 TrackPopupMenu 入口。

v243：Cascade 将菜单快照与真实 popup 面板连接，hover 创建子面板，键盘
导航跳过禁用/分隔符并滚动显露焦点；输入只生成一次命令 ID，先销毁级联窗口
再交还调用者。外层仍须提供模态消息路由和 MenuTarget 校验，目前未替换入口。

v242：新增非激活 owned popup 菜单面板基础窗口，窗口子类只附着自有 STATIC，
稳定堆状态由 Panel 持有，销毁 HWND 后才释放状态；身份核对防止误销毁复用
句柄。输入队列有界，连续移动合并，溢出锁定取消；窗口过程不调用编辑业务。
当前只有基础行绘制和消息收集，尚未替换系统菜单或接齐级联菜单交互。

v241：原生菜单展示层通过 native_menu::snapshot 保留既有 HMENU 命令树的
标签、命令 ID、禁用/选中/radio/default 状态和子菜单，不复制业务 append
实现。快照独立拥有字符串，HMENU 销毁后仍有效；目前尚未替换系统菜单宿主。

v240：历史列表布局将最终屏幕 bounds、可见行和滚动条命中绑定到同一模型，
右侧不足时左开并限位到工作区，滚动只改变本地偏移；异步替换模型不继承拖动。
这不是可见菜单窗口，尚需原生宿主负责绘制、捕获、键盘和完整菜单生命周期。

v239：历史 Session 将事件与后台任务组合为 UI 可轮询的状态，不轮询系统内容。
有变化即失效旧列表，读取期间的变化合并到后续刷新；启动失败保留刷新请求，
重试间隔至少 250 ms。UI 等待 6 秒后取消接收而不 join 提供方；选择还需匹配
已绘制条目 ID，防止刷新重排后的旧索引误选。窗口/文档授权仍归未来 UI owner。

v238：历史 Events 以单个原子标记合并 HistoryChanged、ContentChanged 和
HistoryEnabledChanged；回调不读取内容、不访问窗口或可重入 UI 状态。令牌
在 STA 退出前注销，回调仅持有 Weak，迟到事件不能延长 UI owner 生命周期。
UI 仍需接线，并在读取任务运行期间保留刷新请求，避免丢失后续事件。

v237：历史 Task 在启动线程前取得进程许可，后台仅通过容量为 1 的私有通道
返回结果；UI poll 不等待，cancel/Drop 关闭接收端并发出取消信号，不 join。
每个任务独占通道隔离新旧菜单结果；应用结果前的窗口/文档/菜单目标核验仍是
未来 UI owner 的责任。提供方阻塞时线程保留许可，不允许重复启动读取线程。

v236：新增只读 Windows 历史快照后端，结果仅保存受限编码字节/文本及尺寸，
不写系统剪贴板、不落盘或记录内容。STA 中的 WinRT 对象不跨线程，取消请求
须等操作离开 Started 后才释放读取许可；调用者必须异步接收，禁止 UI join。
WIC 限额是预先核对的输出像素预算，不是系统编解码器内部工作区的硬隔离。

v235：文字批量导入使用 Editor 的单事务接口，不逐行调用单条插入，也不
克隆整个文档快照。历史复用已有 batch 所有权转移，预算包括 undo/redo
保留点。系统剪贴板历史的读取、列表和异步目标授权仍需独立接线。

v234：独立原生内部分发包由已验证候选与当前源码共同生成，包含 EXE、
许可证材料、构建身份摘要和包清单。归档验证只读 ZIP 并交叉核对全部文件，
不解包执行、不授予签名、公开发布或整体运行验收状态。

v233：原生分发材料收集器从离线 Cargo metadata 的 native 根遍历全部平台
normal/build 边，排除 dev-only 分支，收集实际许可/NOTICE/COPYRIGHT 文本，
绑定元数据、四份补丁来源和文本字节。它只提供分发材料，不授予发布资格。

v232：原生启动探针的文件输入来自内嵌品牌 PNG 的短生命周期副本，
application_smoke_image 拥有独占创建与清理。四条探针保留真实解码、
异步导入和 HWND 链，但不再要求分发机器存在编译时源码目录。

v231：独立原生内部 Release 候选通过 scripts/build-native-candidate.ps1
构建，native-candidate 子目录分离源码快照、工具身份与最终字节收据。
构建前后身份变化拒绝候选；dirty 允许记录但不能发布。该通道不调用
Tauri，不替代签名、ZIP 分发、便携性和完整运行验收。

v230：独立原生产品的 PE 图标/版本资源由 native/build_support/windows_resources
在构建期生成并编译，仅链接产品 bin；不增加 GUI 框架或运行时依赖。
窗口图标 owner 与 Common Controls manifest 不变。PE 保留公共三段版本，
内部身份、最终候选组包及包级 provenance 仍是独立交付边界。

v229：固定 usvg 的滤镜结果区分作者显式名称与内部生成名称；只有前者可被
显式输入引用，默认前项与未知/前向回退仍由原解析器处理。原生专属回归
归 surface_svg_filter_reference_tests。生产滤镜白名单不变，不新增进程、
资源或解析 owner；这不是滤镜准入或整体 Windows 迁移完成。

v228：资源实测使用仅测试可用的 SVG peer，经生产 IPC host 和 Job 调用
既有 rasterize。测试中有意绕过 XML 功能白名单以测量未准入滤镜；产品
入口及 filter/fe* 拒绝策略不变。成功只回传小确认帧，测量不覆盖大像素
帧传输，也不作为像素等价或正式滤镜准入证据。

v227：SVG worker 使用 render_with_filter_budget，单次渲染共享累计滤镜策略
积分；额度归 renderer 的 work_budget，宿主只设 4,000,000,000 的策略上限并
映射 InvalidData。嵌套 image/pattern/mask/feImage 不重置额度，失败不输出
部分像素。它不是全 SVG 耗时、峰值内存或浏览器兼容保证，生产滤镜仍拒绝。

v225：SVG worker 调用 resvg render_checked，将滤镜执行错误映射为
InvalidData 而不输出部分像素。错误状态属于单次渲染，贯通 pattern/mask、
clip paint、feImage 与嵌套 SVG image；坐标检查属于 renderer，不在宿主
重建变换管线。生产滤镜准入继续关闭，累计工作预算尚未完成。

v222：SVG worker 在严格 XML 解析后、语义白名单和 usvg 之前执行滤镜预算
预检，限制定义/节点、数值列表、卷积核和 octave 的解析规模；不改写文档。
滤镜模板引用仍由原非递归结构图拥有。预算通过不授予渲染权限，filter/fe*
仍被功能 policy 拒绝；变换后坐标安全与累计像素工作预算尚待实现。

v221：原生宿主的 resvg 改为仓库固定来源，usvg 同步已重放的数值保护。
仍由现有受限 SVG worker 解析和渲染，父侧不加载 XML；原生 Cargo 锁只变更
resvg 来源，不启用 raster-images、系统字体扫描或新 features。filter/fe*
及 filter 属性继续由 policy 拒绝，依赖能力与宿主准入分别验收，不改变
Job、超时、授权安装或窗口生命周期。Tauri 依赖图不变。

v220：SVG paint-order 经原 policy 的严格 token 校验进入受限 worker，复用
既有 renderer 的 fill/stroke/markers 排序。拒绝非法值的默认回退，允许 normal
和无重复的部分/完整顺序；祖先继承沿原解析链路。不开放 CSS style 或全局关键字，
不修改第三方源码、资源预算、父侧解析或 Application 安装生命周期。

v219 准入继续接线：text/tspan/textPath 的 textLength、lengthAdjust 与 ltr/rtl
direction 通过原 SVG policy 进入受限 worker，使用已接入的元素范围布局补丁。
零/负长度沿 renderer 忽略语义；无效数值、错误枚举/元素仍拒绝，CSS style 未开放。
不新增 parent XML 解析、全系统字体扫描或渲染后端；完整 SVG/双向文字验收仍未完成。

v219：原生 Cargo 接入仓库固定的 usvg 扩展与 roxmltree 迭代 tokenizer；后者和
文字叶 span 提取组合消除已复现的 512 层 child 栈溢出，不扩大 Job/超时预算。
所有解析仍在原 worker 内；新增长度/方向能力尚未由生产 policy 开放。
来源 schema 2 区分上游与新增源码，准确字节身份继续绑定 Cargo 与双 SBOM。

v218：受限 SVG child 中的 usvg flatten 按 glyph 提交 outline Path，避免
轮廓合并改变重叠抗锯齿/透明度/描边合成。外层 opacity 与 paint-server 的
整段文字 bbox 保持原链路。节点数增加由既有 Job 预算兜底，4096 glyph 已测；
不宣称全字体性能完成，也未开放 textLength 实验能力。

v217：路径文字按 dx 后的 advance 跨度锚定，法线查询排序与 glyph 顺序分离；
局部 f64 运算避免公共起点参与跨度相减。仍在原受限 SVG child 内执行，
字体预算、Job、像素协议、Ready 授权安装和普通文字路径均不变。

v216：SVG textPath 沿既有受限 worker、字体预算和 Ready 安装接入。
startOffset 与路径引用在解析前置门校验；usvg 直线段采用精确弧长定位，避免
曲线容差导致普通文字与直线路径文字位置不同。不引入父侧 XML 或新进程 owner。

v215：native Cargo 使用仓库内 usvg/tiny-skia 固定补丁副本，修复
零宽 context-stroke 来源丢失及窄画布 AA hairline 提前退出。原 Job、像素协议、
授权和资源预算不变；源码哈希完整性门及带来源身份的 SBOM 见 third_party/README.md。

v214：静态 SVG marker 沿用原 worker/像素协议接入，引用图前置拒绝 marker/use
递归；Application Ready 与授权边界不变。基线与验证状态见 SURFACE_SVG.md。

v213：既有 SVG normalize 修复仅描述子内容的 pattern 模板选择，policy 开放
实测四种 pattern overflow。原 parent/child、授权与预算不变，证据见 SURFACE_SVG.md。

v212：SVG worker 开放静态 pattern；前置引用图覆盖 fill/stroke 与模板依赖，
复用原 Job、像素协议及 Application Ready 安装，不在父进程解析或加载外部资源。
浏览器对照、回归状态和剩余能力见 SURFACE_SVG.md。

v211（开发中）：引用图模块更名为 surface_svg_reference_policy，并覆盖 symbol/use。
normalize 将 symbol 适配为 defs/svg，复用实例 viewport 分支修复百分比与零尺寸。
浏览器配对基线与专项已通过；完整回归状态及剩余边界见 SURFACE_SVG.md。

v210：SVG mask 接入原 Job child。policy 后增加含 mask 文档的结构引用图准入，
normalize 独立处理根尺寸、零尺寸嵌套与 mask 定义属性的 renderer 差异。
XML/引用图不进入父进程；所有资源仍拒绝外部来源，量化差异见 SURFACE_SVG.md。

v209：SVG child 开放嵌套 viewport，根自然尺寸及 OSI2/OSP2 协议不变。
viewport 规范化负责零尺寸嵌套不绘制的浏览器语义；原 policy 递归检查全部
后代属性/资源，不因嵌套开放新的外部来源。验收记录见 SURFACE_SVG.md。

v207（开发中）将有限 text/tspan 接入 SVG child。surface_svg_text_fonts 适配已有
字体来源，保留预算与 TTC index，拒绝不可表达的 simulation/scale，并校验加载
metrics 与已布局 glyph；不在 Application 解析 XML 或读取字体。完整验收尚未完成。

v206 字体来源增加系统范围 fallback，保留 UTF-16 映射长度、scale 和缺字 None；
COM 分析源在同步调用内持有文本/locale。与显式选择共用预算/文件来源，不新增
进程 owner。usvg 单字符回调不足以无损表达该合同，renderer 接线仍待完成。

v205 新增 SVG child 的按需字体来源模块及可信文件句柄叶模块，保留 TTC index/
simulation，限制查询/文件/字节预算。复用原 Job launcher；尚未接排版器，text
allowlist 和生产依赖不变。安全保证及验证边界见 docs/development/SURFACE_SVG.md。

v204 SVG 的 OSI2/OSP2 协议携带独立 f32 逻辑尺寸，父侧在像素分配前验证。
Resource/Inline Decoded 经 Ready 同 Grant 转移到窗口 Bitmap，完整保留逻辑尺寸；
Image 保持整数栅格，WebP v1 不变。小数尺寸已接通，AA/子像素布局/DPI 等仍待核验。

v203 Surface Bitmap 分开逻辑尺寸与整数栅格，Presentation/NativeMeasure 传递浮点
尺寸，contain 和 sourceRect 使用逻辑尺寸；上传/释放仍按像素预算。此阶段保留
整数默认构造与测量适配，不改变通用 Image/Relay；SVG child 元数据接线仍待完成。

v202 新增受限 SVG renderer/policy/viewport 叶模块，SVG 固定 OSI1/OSP1 协议复用
原 child/Job/容量池。image_decode::surface 的共用 image worker client 在响应尺寸
验证后领取 Pixels，最终成功门保持；WebP wrapper 保留预检。首批静态矢量支持及
明确拒绝/后续能力归 docs/development/SURFACE_SVG.md，不是完整 SVG 迁移完成。

v201 图片解码共用 host/wire/process owner 已由现有 WebP 生产路径使用。renderer
回调之前验证 Job 和请求长度；返回后独立验证尺寸/像素数量再发成功响应。
process 在根模块只编译一次，共用原 256 MiB 容量池；没有增加第二套 launcher。
当前 v1 仍仅 WebP，SVG 接入边界见 docs/development/SURFACE_SVG.md。

v200 color 的私有 lut 叶模块为 A2B0 mft2 RGB→XYZ 保持浏览器分阶段求值顺序，
不把 gamma 预融合进规则采样网格；复用原 ICC parser，直接借用表、无全图复制。
仅写 RGB，随后仍由 worker 预乘；其他 profile 保留原通用路径，Job/IPC/来源门不变。

v199 WebP child 在 straight RGB(A) 解码与 PBGRA 预乘之间应用嵌入 ICC→sRGB。
私有 color 模块借用元数据，以 4096 像素通用 matrix/LUT 变换限制每批缓冲，保留
alpha。无效 metadata 的已验证 fallback 不包括资源不足/不支持/转换失败，后者
拒绝成功图片；所有第三方工作内存仍在 Job 内，不新增主进程解码回退。

v198 WebP 保持编码方向，与实测 Chromium 普通 img 基线一致；EXIF 方向不套用
JPEG/TIFF WIC 变换。新增的是固定浏览器夹具的真实 child 回归，不增加解析、缓冲、
IPC 或权限边界。32 案例和独立有效性核验见 SURFACE_WEBP_ISOLATION.md。

v197 的 WebP 父侧 run_with 是私有启动接缝：生产调用固定 spawn，cfg(test) 以同一
Job launcher 启动坏响应 peer，直接验生产 IPC 和生命周期；不增加用户可配置 worker
或主进程回退路径。八类失败及合法对照逐一检查 child 已退出，边界见
SURFACE_WEBP_ISOLATION.md。

v196 静态 WebP 从 Surface 实际签名分流至受限同 EXE child，不走系统可选 WIC codec。
纯 RIFF 前检及子侧复检后输出预乘 BGRA，父侧校验完整协议/退出后返回共享 Pixels，
继续沿已有 Job/Ready/同 Grant/HWND 链安装。child 拥有 128 MiB Job，最多两路并行，
其余请求可取消地等待；低层管道复用不带入 recovery 业务协议。空环境和句柄白名单
不等于权限沙箱，child 仍同用户 token。WebP 元数据/动画及全故障/整组峰值验收待补，
详见 SURFACE_WEBP_ISOLATION.md；此前增量的未支持项以该最新记录为准。

v195 普通 Surface 的静态位图路径以有限签名确定实际格式，WIC 继续复核容器及单帧，
APNG/EXIF/像素预算跟随实际格式，不把声明 MIME 当容器身份。data URL 在原始来源
门后规范化并分流 SVG；原字节摘要、Binding/Grant、生命周期不变。Tile/Wall 与普通
Image 不改，实际 SVG/WebP/动画/blob 仍不支持；不是任意 WIC codec 隔离保证。

v194 Surface JPEG/TIFF 在后台 WIC 源链处理 EXIF 1–8，返回定向后的像素与自然尺寸；
原尺寸在元数据/变换前准入，最终源再次限制，共享 Pixels 预算保持。元数据 RAII 清理，
COM 对象不离开线程，普通打开和 Relay 仍传原始帧。XMP/PNG eXIf/ICC 不在此增量内；
没有新增网络来源、依赖、授权绕过或 UI 控件。具体边界及基线见 SURFACE_IMAGES.md。

v193 无像素 data:image/ 来源不再等同 unresolved resource：保留 Image 节点，使用
本地缺图标记及 alt 的独立测量/文字绘制，成功安装替换；不伪造自然像素/图片宽高比。
它表示当前像素不可用，不把等待伪装成已收到远端错误。来源优先级、同 Grant 和
原始快照门不变；替代标志不从协议反序列化，不引入按钮、焦点或远端图片事件。
TextFrame 中 alt 不可选择，绘制复用 Image 的内容/祖先/圆角裁剪及透明层。

v192 图片生产链增加受限 data:image/ 静态源：类型化 Request/Key 区分资源和节点，
resourceId 优先且失败不回退；内联在配置/Client 准备前分流，复用后台 Job/Slot/Ready。
内联不伪造资源租约，不获取 URL/文件；来源绑定原始快照摘要、完整 Binding/revision，
仍要求原 Lease 与同 Grant 的目标窗口授权。安装刷新只计算一次快照摘要，避免每图
解析整棵 Scene。数据 URL/像素/场景预算不扩大，完整格式/blob/错误占位仍待迁移。

v191 Image 呈现语义与旧 img 分支对齐：使用 props.alt，不应用通用可访问标签或
disabled；Image 在呈现树中是叶节点，原始 children 仍受协议预算和合同指纹约束。
带 alt 图片作为 UIA Image 信息元素，纯装饰图片只留 raw provider，不进入 control/
content view。既有 HWND/UIA 身份撤销与输入白名单不变，图片不获得 Invoke/焦点。

v190 应用 poll_remote_live 完成场景结算后自动消费 Ready 并安排 resourceId GET。
AuthorizedWindow 持有完整 Binding/revision 下的最多 64 个去重候选；仅 confirmed
active 请求，开始成功/终态失败即消费，Busy 保留，不对同身份失败无限重试。
Windows 轮转起点，每窗口每轮最多一个启动；Ready 精确匹配后仍走同 Grant 安装。
Config/Lease 在 UI 结算点准备，TLS/文件信任/DNS/解码在已有任务内完成，等待/退出
复用既有句柄与回收链。合成挂载 + 真实授权/GET + 应用 poll 已有回归，非在线 Loom
完整业务验收，通用 image 能力报告尚未扩大。

v189 Ready 消费安装入口先核验原请求，再向同 Grant 的窗口 Lease 转交授权；
消费释放网络任务槽，不为每张安装图片永久保留请求 Lease。AuthorizedWindow 的
surface_window_image_install 持有有界 Bitmap 表，从当前资源快照生成呈现投影，
到期/换绑撤图、换场景/关闭/撤权清空。图片期限复用现有主循环 deadline，HWND
调用后再次复核窗口授权。同场景刷新保留未变文字选区，自动 GET/投递调度仍待接通。

v188 应用先建立不含未就绪图片呈现的授权窗口，再保留原始资源快照供当前 Binding
准备请求。Scene availability 投影不改协议 props/资源标识；布局、命中、UIA 复用
同一 visible 门。初始/事件/batch 都保留原成功提交与撤权边界；自动调度和像素安装
尚未接线，pending 投影不是删除图片功能或迁移完成。

v187 显式 Images 从 SurfaceWindow 创建/替换进入 Host 与既有 Presentation；普通
API 传空表，不从旧场景继承。surface_window_scene 独占窗口提交接缝，保持 UIA
身份撤销及重入检查。真实 HWND 复用 target 重建与关闭资源释放；此入口不授予
来源读权，应用自动 GET/Ready 授权安装/到期撤除仍需接线。

v185 原始资源快照沿 attach admission/完整事件收敛进入 AuthorizedWindow，与当前
scene revision 同步提交；不在 Sessions 猜测哪个候选已显示。资源 mount 使用当前
lifecycle Binding，身份/代际/场景版本严格校验；关闭撤权释放快照。纯模型不授予
读取权限，Ready 消费仍检查原租约；自动请求和 HWND 图片安装尚待接线。

v184 显式解码图片表沿 NativeMeasure → Presentation → PaintFrame 接通：图片模块
只计算自然比例/内容盒反馈，reflow 同时等待文字与图片稳定。Presentation 的图片表
与场景同生命周期；新场景不隐式复用，所有滚动/显露投影保留当前 owner。
这仍是低层已验证像素入口，来源租约/当前 revision 验证和应用 HWND 安装尚待接线。

v183 图片进入现有 PaintFrame 后端：UI-only Rc Bitmap 共享 Pixels，精确 target
缓存与全局上传字节预算；帧退场/target 重置释放。绘制不发请求或重新解码，不增加
窗口、覆盖层或线程。生产 Image 自动安装和自然尺寸测量尚未接通，能力声明不变。

v182 共享 WIC Image 提取至 image_decode，桌面旧路径重导出，旧调用语义不变。
Surface worker 内执行静态解码，编码缓冲返回前清零，Ready 仅保留共享预算像素和
身份；UI 消费继续复核原租约。资源到期接现有 wait deadline 与取消/释放，不增定时线程。

v181 图片资源 Job 接入 Sessions 的共享容量、wait/try_join、关闭与退出 drain；
Ready 保留原授权 Lease 和共享槽直到消费/丢弃。应用结算前后按当前活窗口 Binding/
snapshot revision 取消旧资源。此阶段仍无图片自动请求、解码、缓存或 UI 安装。

v180 图片资源获取沿现有 Live/Surface Control → 固定来源 HTTPS 通道；私有 lease
类型限制路径、头与 16 MiB 预算。VerifiedResource 只拥有校验过的传输字节和请求
身份，消费须核对当前 mount/期限/取消。后台 job、WIC、布局及 PaintFrame 接线尚未
完成，不新增常驻线程或缓存，不扩大 image 能力声明。

v179 高亮范围在每次 draw 内一次解析，ProjectedSelection 不持有文字或跨帧索引。
逐 Text 命令复用既有索引，在原 layer/clip 内绘制，无新增缓存、owner 或绘制通道。

v178 上下扩选先走 focus 的实际 DirectWrite 行，再进入相邻 selectable owner；
列意图通过旧原点与新原点换算，软换行是否移动不再只由 UTF-16 偏移判断。
单 Surface 所有权边界保持，页面跨 Surface 选择仍未实现。

v177 水平键盘扩选复用跨节点端点，原始导航 anchor 不变；focus 布局负责字素
步进与行端点，selection_keyboard 负责有界相邻 selectable owner 搜索。普通
跨节点左右不折叠选区；上下跨节点尚未实现，其 down/up 统一放行。

v176 依据真实旧组件词手势基线，分离初始双击范围与拖动词干边界；唯一 WordDrag
记录初始坐标和移动阶段，未移动抬起保留空白，拖动后返回原点也不恢复尾随空白。

v175 词级拖动复用跨节点端点模型，初始词固定属于 anchor，当前词边界只属于
focus owner；切换重新查询，结束释放，不累计节点缓存。复制门禁仍拒绝聚合，
键盘跨节点和系统双击实际输入验收仍待完成。

v174 普通拖选使用独立 anchor/focus 节点归属，范围按 TextFrame 场景先序投影；
PaintFrame 原有逐文本绘制通道负责所有选中项高亮。重排验证范围内成员/顺序/文字，
滚动显露跟随 focus owner。跨节点不聚合复制，词级和键盘跨节点迁移仍未完成。

v173 为词选择补系统双击探针，生产结构不变。首次运行被前台保护在输入注入前
拒绝，不能视为系统输入验收通过；保留失败证据和显式手动探针入口。

v172 原生双击复用 Presentation 选区和父窗口捕获：DirectWrite 命中、系统 ICU
有界词边界、仅手势期间的词范围缓存。DLL 限系统目录按需加载，缺库按操作失败
处理，不增加启动静态依赖。词级拖动沿原自动滚动/取消路径，跨节点仍待迁移。

v171 为 v170 增加实际 Win32 timer 队列验证，包括停止后已入队消息的迟到派发；
生产结构不变，仍由手势身份与 timer 身份共同保护生命周期，不依赖 KillTimer 清空队列。

v170 拖选边缘滚动复用 Scroll 祖先消费、缓存投影和单一 Selection，HWND 仅持有
活动手势的按需 timer 与身份；每次 tick 核验捕获/焦点/权限，不追赶积压。
同步重入取消后旧查询结果不得重建 timer；松键/关闭清理，无空闲轮询或新增线程。

v169 行首/尾扩选以 DirectWrite 行度量为唯一边界来源；独立 line_edge 子模块
按文本上限临时查询，Presentation 保留 anchor 并更新 focus/行归属，再走既有
显露和控件同步。普通滚动键与 EDIT 门禁不变，不将行级操作混同全局全选。

v168 上下行选区导航继续复用实际 DirectWrite 排版；唯一 Selection 增加可选
局部列/行探针，解决短行列保持及软换行同索引的行归属，鼠标与显露共用它。
重新排版才重置旧点，纯滚动投影不重置；HWND 只接 Shift+Up/Down，不改变
普通滚动或系统 EDIT 所有权。没有新增线程、队列、计时器或文字副本。

v167 文字选区端点显露复用 Scroll 的局部 region nearest 与缓存投影事务；
DirectWrite 提供 cluster 几何，选择模块确认候选至少部分可见后提交滚动。
Host 只在实际滚动后同步既有 EDIT region，不改变草稿或创建第二套输入状态。
不可达候选不推动外层，纯显露不重测排版；鼠标越界自动滚动仍是未完成范围。

v166 水平键盘选择复用 DirectWrite cluster 和 Presentation 的单一 anchor/focus；
Host 同步 viewport，独立文字键盘子模块以实际父焦点、local_edit、导航锚点
准入，在滚动前处理。动作候选仍由旧生命周期撤销，不创建编辑 FIFO 或复制
文字缓存；文本逻辑移动与尚未实现的 bidi 视觉导航明确分开记录。

v165 文字光标通过既有 Host/Presentation 的正式选择起点判断取得，不维护
第二份选择或命中状态；使用系统共享 IDC_IBEAM，无光标资源所有权转移。
系统鼠标测试在独立可见自有窗口运行，有界观察器不消费生产消息；生产架构
没有新增线程、队列或编辑窗口。真实输入证据与剪贴板/物理手测边界分别记录。

v164 `Presentation` 持有单节点文字选择，原绘制指令负责高亮层次；HWND 桥接
仅转换坐标及捕获，取消通过单标记跨 State::run 重入兑现。Ctrl+C 独立于远端
click/FIFO，复用 clipboard 的有界 HGLOBAL owner，清空前复核当前焦点/场景/选区。
不新增窗口、计时器、线程或隐藏文本编辑器；已完成选区随同内容重排保留。

v163 可选择文字基础仍归既有 DirectWrite Layout/TextItem 所有；规范化 UTF-16
快照随布局共享/释放，命中及范围矩形复用同一 native layout。TextItem 只做
坐标和裁剪转换，不拥有手势或剪贴板；后续选区交互不得另建隐藏 EDIT 排版。

v162 仅增强测试：真实活动 pan 的撤权/换场景生命周期由独立 ignored 探针
验证；正式 Surface 夹具负责状态断言和 prepare/settle 事件检验。观察器增加
有界 activation/cancel/focus/resize 诊断，不消费消息、不改变生产 owner。
系统惯性和 GID_END 必须实际出现；不引入生产线程、队列或新输入算法。

v161 Slider 通过创建期 SetGestureConfig 把单指垂直输入交给系统 pan、水平
保留值输入。系统默认转发的手势仍以子 HWND 为目标，Surface 仅接纳当前挂载
Slider；Pan 以 target+sequence 持有既有父滚动，不新增业务状态或惯性循环。
WM_POINTER 完整交默认过程，不混用半条自行消费、半条系统提升的序列。

v160 在 Slider 既有拖动 owner 增加来源位：系统触摸提升不进入鼠标 click
候选，跨来源移动/释放不改变当前手势。该标记不作为权限凭证，仍复用原数值
提交队列；没有新增渲染循环、线程或业务 owner。完整触摸/父滚动接线仍待实现。
显式系统输入测试由 Invoke-NativeTouchProbe.ps1 可见启动，真实命中门禁保留。

v159 新增测试专用系统触摸探针，真实输入与生产子类之间仅安装不消费消息的
观察器；验证实际目标 HWND 后才允许注入。显式运行目前被命中门禁拒绝，
不能作为 touch capability 证据。生产输入架构不变，无新增常驻资源。

v158 的浏览器触摸基线要求方向判定、父滚动交接和兼容鼠标去重；触摸不能
直接沿用 CapturedPointer 的鼠标 click 冒泡。尚未新增生产触摸 owner，不能
据系统默认可能提升为鼠标而宣称完成；参见开发目录 SLIDER_TOUCH_BASELINE.md。

v157 将 Slider checkpoint 与 finish/cancel 分开：正常切焦提交当前值并更新
拖动基线，但保留鼠标捕获和原始 click 目标；抬起才 finish，系统撤销仍 cancel。
父 Tab 路由仅对实际捕获的挂载 Slider 保留 native_press，其余候选失效不变。

v156 区分普通坐标 Pointer 与原生捕获完成 CapturedPointer。后者仅由真实
Slider 按下准入形成，完成时保持原节点，不按越界抬起位置改投其他控件；
合同和事件准备重新验证当前可聚焦/挂载/只读状态，复用同一场景身份与 fence。
真实 Chromium 对照证明本场景不是共同祖先重定向，更新此前待确认假设。

v155 为 Slider 内完成的鼠标手势接入祖先 click。子 HWND 仅发送同线程私有
边沿，父 Pointer 用挂载身份、场景 token 和实际命中准入；不抢父捕获、不另建
动作队列。Slider 的 input/change 先进入共享 FIFO，祖先 click 捕获之后的
fence，必须等待这些边沿结算；与 Switch 的 click-before-change 顺序不同。
无 click handler 不占单槽，取消/替换/撤权不能留下迟到候选。

v154 以真实 Chromium 事件序列修正 Slider 混合输入：键盘提交更新拖动基线，
不取消后续拖动；Tab 先提交未结束手势再导航，鼠标抬起复用同一收尾。
收尾释放捕获后验证 HWND owner/权限/数值，不能在同步销毁后继续通知或导航。
系统取消/禁用/隐藏仍只撤销手势，不被重解释为用户提交。

v153 的 Slider IsReadOnly 与 Value 共用单消息/重入护栏；Mount 是权限状态来源，
仅停止发布不改变本地编辑只读状态。范围参数属于现有编辑合同，更改时重建
节点并撤销旧 provider，以结构变化表达新范围，不对旧身份发属性修改通知。

v152 将 Slider 跨进程验收扩展到存活订阅：先证明旧订阅实际收到数值变化，
再保留至旧进程退出和新实例操作完成，验证同名节点通知不跨进程身份误投。
结束时显式解绑并确认 handler 释放；有界测试 Observer 不进入生产构建。

v151 的独立进程 Slider 验收复用现有 Fixture owner 和真实 MTA 订阅客户端。
父进程验证系统树和旧 provider 生命周期，子进程仅在正式 prepare/settle
校验六个 input/change 边沿后确认 accepted；强制退出只操作所持 Child 句柄。
这是跨进程协议证据，不替代真实读屏与完整用户输入验收，不改变生产架构。

v150 为 Slider 增加 UIA RangeValue.Value 属性变化通知，输入和自动化设置均
从 Mount 已接纳值提取差量，不单独维护可修改数值。沿用单个延迟通知消息，
同节点合并至最新值、净零变化取消；Host 忙时延后，回调重入后再次检查身份
和当前值，替换/关闭清旧待发送项。订阅发送在 Host 恢复后且无 RefCell 借用。
本增量只覆盖 Value 通知，不代表 read-only/range 元数据通知或真实读屏完成。
Slider 以 ProviderOwnsSetFocus 使用既有子 HWND 焦点桥，焦点属性纳入实际
interactive 判定；不依赖系统 STATIC provider 代选最近窗口。

v149 的 Slider RangeValue 通过现有 HWND override 合并系统 UIA 树，复用
canonical COM 身份、owner 查询和 Mount 数值状态。读数不缓存；写入先检查
身份、权限、捕获和共享队列容量，再提交归一数值和 input/change 快照。
不模拟键鼠或直接发送网络。属性变化通知与真实读屏验收仍待补齐。

v148 将 Slider 接入真实 Surface 的测量、Mount、权限和共享事件 FIFO：
私有父通知形成 input/continuous 与 change/commit 数值快照，准备、取消重试、
结算均走原队列；不加入文本表单值。键盘双边沿先检查容量，拒绝后恢复 HWND
已接纳值，防止半个动作。相同 source/几何同步保留本地值和拖动捕获；不同权威
source 静默同步，合同或类型变化先撤销旧意图。Tab 共用现有焦点路由，Slider
跳过 disabled 节点。UIA RangeValue 和完整用户交互/资源验收仍未完成。

v147 为 Slider 新增真实原生自绘子窗口。数值继续使用 Range 浮点模型，
方向键/端点/翻页及鼠标捕获只生成私有父通知，不直接发送动作；父回调重入
销毁后禁止迟到 change。局部坐标有符号解析，取消、禁用、隐藏、失焦和销毁
释放捕获，正常拖动抬起才提交。默认/焦点/禁用 GDI 绘制按 DPI 缩放，高对比度
走系统色。当前控件 owner 已独立测试，但 Mount 共享队列、业务接线和 UIA
尚未完成，不能据此声明声明式 Slider 已端到端可用。
desktop 公开启动入口参数适配迁到 startup_entry，原导出名与执行顺序保持。

v146 新增 Slider 数值与 wire 基础：现有前端显式 min/max/step/value 缺省为
0/100/1/0，严格接受有限 number；归一阶段保留浏览器 range 的钳制和中点向上
规则，非正 step 回退 1。Range 是无资源的数值 owner，事件仍走既有不可变
编码，不授权、不发送、不写入文本表单草稿。真实原生控件/输入/UIA 尚未挂载，
此增量不能视为 Slider 已可交互；极端浮点跨度/步距显式拒绝仍属兼容缺口。

v145 接通 Select 显式 color：PaintFrame 复用既有 Color 解析和父计算值，
通过 Presentation/Mount 将有界 RGBA 快照交给 HWND。收起区域及未选中列表行
按实际背景合成文字 alpha，选中/禁用/高对比度状态色优先。私有颜色属性随
HWND 销毁释放，不持有场景引用、不改变事件或选择状态。fontFamily 不在
当前声明式样式白名单内，本次未扩大协议；旧记录列为待支持不构成新增授权。

v144 为 Select 列表采用 CBS_OWNERDRAWFIXED，继续保留 CBS_HASSTRINGS 和原
系统 COMBOBOX/UIA。父 HWND 仅安装一次无状态 WM_DRAWITEM subclass，按实际
父子关系及私有 owner 标记路由；父窗口销毁时解除，无 Rust 会话裸指针。
行高由当前 GDI 字体度量，未变化不设置，避免打断展开。选中行使用 Neuro
focusSurface/focusInk；高对比度显式使用系统颜色。后文 v142 未使用 owner-draw
是历史增量描述，当前列表绘制以此段为准；没有自建选择或可访问性 provider。

v143 补 Select 的 hover 边沿：客户端内首次 WM_MOUSEMOVE 启动系统 leave
跟踪并切换 #18222c 背景；重复 move 不重复申请跟踪/重绘，leave、disabled、
hide、cancel 清除状态。状态只占现有 subclass ref-data，销毁时取消跟踪，
不更改焦点、选择或业务提交；高对比度仍由系统绘制。

v142 将 Select 收起区域改为原生 GDI 绘制，系统 COMBOBOX 继续负责输入、
选项索引、下拉窗口与 UIA。收起区域使用深底浅字、系统几何中的箭头位置、
disabled 次级文字和按 DPI 缩放的黄色焦点框。高对比度模式交回系统，未使用
CBS_OWNERDRAW 或自建 Selection provider。WM_PAINT 不先画白底，避免双重绘制。

v141 补齐原生 Select 弹出列表的未选中行配色。COMBOBOX 子类在确认实际
hwndList 后处理 WM_CTLCOLORLISTBOX，与 Surface EDIT 复用 Neuro 控件背景
#111720、文字 #f7f8ef。继续使用系统列表/选择/UIA 模式，不新增自绘列表或
拥有型画刷；系统高对比度模式优先，不改全局主题设置。

v140 将 Select 的原生 GDI 字体接到测量层已解析的继承样式，避免布局按声明
字号测量、控件仍用 stock 字体显示。场景级缓存仅保存 Select 所需的有界
owned 字体快照；滚动不重算相对单位、不创建新字体。WM_SETFONT 仍走 v139
的 HWND 资源生命周期，未引入另一套样式解析器或自绘下拉模式。

v139 补齐 Select 固定 stock 字体不随布局 DPI 缩放的接缝：每次从原始字体
描述计算目标像素高度，避免显示器往返累积误差；同字号复用 HWND 所有的
GDI 字体。窗口默认销毁结束前保持字体有效，Rust owner 撤销不提前释放。
此增量不宣称声明式 fontFamily/fontWeight、主题配色或人工多显示器验收完成。

v138 新对照修正了下面的诊断方向：Common Controls v6 manifest 使真实 UIA
树与 Selection 模式正常工作，旧 MSAA 子对象缺陷不再等价于 UIA 失败。
`native/build.rs` 为 MSVC 宿主和测试统一嵌入 `windows.manifest`；系统控件
继续提供模式，未引入自建 COM 代理。最终选择在真实 CLOSEUP 上核对并经过
原权限/FIFO/accepted 去重；全量库 2438 passed/33 ignored、startup 7 项、
跨进程 4 项及 Clippy/fmt 通过。以下旧 MSAA 排查不再代表当前 UIA 状态。

Select UIA 接线正在验证：COMBOBOX 通过现有 HWND override 进入逻辑父级，但
系统 Combo MSAA 子 List 的 NAVDIR_NEXT 会错误离开控件树。纯系统 HWND、跨
进程 UIA 与直接 MSAA 调用已有失败证据；不是 options 重复，也不能用自绘
popup provider 存在来宣告修复。当前不扩大能力声明，模式/提交全链验收未完成。
直接 MSAA 窄包装试验也未通过 UIA 验收，已撤回：系统模式依赖服务暴露，而
原样透传的 Ex/返回对象路径会绕过包装树。后续适配必须同时保证 pair 身份、
模式返回元素和导航边界，不得以丢失 ExpandCollapse/Selection 换取遍历通过。

系统 COMBOBOX 重定位不会自动收起弹出列表；Select Mount 现以常数大小几何
快照识别真实锚点/裁剪/DPI 变化并在重定位前关闭列表，无变化同步不关闭。
真实滚动回归验证了可见区退出/恢复时的禁用与选择保持，不合成业务 change。

Select 通过独立窄子类接入已有混合焦点导航，不复用 EDIT 组合输入处理。Tab
先收起系统列表，再按 owner marker 复核后转发 NAVIGATE；Mount 将逻辑节点
路由到真实 HWND，disabled/hidden 跳过，真实前后向导航已纳入回归。

Select 的真实下拉现由字段 Mount 创建，系统 CBN_SELCHANGE 驱动共享有界 FIFO，
选择快照在 prepare 时编码 change/commit，不进入文本 draft/表单集合。回压恢复
最后接纳索引，绑定/合同变化销毁旧 owner 并撤销旧意图；服务端和完整 UIA 尚未验收。

Select 已有独立 Win32 COMBOBOX 资源 owner：不排序、不把 label 当 wire value，
同线程父窗口与私有 marker 保证归属，禁用收起列表，销毁释放系统下拉窗口。
实际 HWND/键盘消息/展开销毁已有回归；Surface 挂载及 live 提交仍未接入。

Select 的原生前置模型与 EDIT 草稿分开：旧前端只发字符串 change，不把本地
选择写入按钮表单值。声明快照限制选项/文本预算并保留重复项，事件复用共享编码；
真实下拉 HWND 与宿主接线仍待实现，当前不暴露为已支持交互的控件。

跨进程自绘焦点通知通过系统 FocusChanged 接口验证：cache PID 先限定自启
进程，Switch/Button 切换核对节点身份与实时焦点互斥，注销后检查 handler 释放
及停止交付。不抢桌面前台，不能以此替代 Narrator/全局前台的人工验收。

跨进程结构通知已由真实客户端验证：先订阅稳定 HWND 根，再经自有管道请求
UI owner 替换场景；收到 ChildrenInvalidated 后从同一根读取新原生 EDIT 的
身份/标签/Value，确认旧节点消失和旧自绘模式失效，最终验证 handler 释放。

活跃 UIA 订阅已验证可穿过自启进程强制退出而不串接同名场景的新实例：新订阅
实际接收事件，旧订阅保持安静，旧模式失效，随后新旧 handler 均可注销释放。
该测试补充订阅中的进程终止边界，不代表业务恢复、焦点/结构或 Narrator 全验收。

跨进程 UIA 测试除操作返回值外，也订阅真实 ToggleState/Invoked 通知。客户端
核对 cache 身份和状态载荷，通过有界 channel 跨回调交接，不重入查询 UI；注销
后等待 COM handler 释放，再执行进程退出。这尚非订阅中崩溃的生命周期证明。

跨进程 UIA 生命周期验证覆盖自启进程被强制终止后的旧引用失效，并在保留旧
引用时启动相同场景的新实例。新实例独立初始化且可操作，旧模式不能恢复或写入
新实例；这一证据覆盖进程隔离，不等同应用业务持久化/崩溃恢复的完整验收。

原生 UIA 已增加真正跨进程验证：独立 STA 子进程持有 Surface，父 MTA 仅通过
系统 UIA 读取逻辑父级、操作原生 EDIT 与自绘 Switch/Button。子进程独立检查
实际值和 Automation 动作收据；正常退出后保留的模式引用不可继续读写/调用。
测试进程由 Child 句柄和有界管道管理，不通过全局窗口枚举或缓存 HWND 清理。
这不是跨进程事件订阅、异常退出或 Narrator 的完整验收。

原生字段 Mount 记录实际 HWND 集合变化，State.run 在恢复 Host 后提取 dirty 并
复用根结构通知。即使后续操作失败，已经发生的结构变化仍通知；普通几何/值同步
不生成结构事件。无需每次查询扫描句柄，不建立第二份持久窗口列表。晚绑定和
同场景 rebind 已纳入验证，当前映射不依赖 HWND 数值永久唯一。

原生 EDIT 不再作为声明式树之外的平级系统窗口：根 HwndOverride 将当前 Mount
的 HWND 映射到对应节点，Fragment 导航和系统 host 合并成唯一元素。原生字段
使用 HWND runtime identity，未挂载字段不暴露；仅覆写语义身份与显式标签，
Value/Text、只读/焦点等属性保留系统实现。EDIT SetFocus 复用既有 owner 并在
重入后验证，根 GetFocus/命中返回承载节点，但不重复发送系统 EDIT 焦点事件。

整场景替换撤销旧子树后，在稳定 HWND 根合并报告 ChildrenInvalidated。通知
不列举每个旧节点，不随场景更换根 provider，失败替换也不能保留旧身份缓存。
私有消息延后到 Host 恢复，重入只排下一轮，关闭后抑制。该机制是重新查询子树
的信号，不是结构操作日志；EDIT 精确重定位和其他动态结构来源仍独立推进。

焦点通知从既有 HWND/逻辑键盘 owner 读取当前状态，经单个私有消息延迟发送。
窗口只记录 dirty/posted/flushing 与已报 runtime identity，不复制业务焦点；
失焦重置去重身份，同 ID 场景替换使用新 identity。Host busy 等外层 run 恢复，
回调重入延至下一轮，Raise 不跨借用。系统 EDIT 仍由系统 provider 通知。
真实 MTA 订阅已验证 SetFocus、Tab、自有窗口鼠标消息及释放；不抢前台的测试
不等同桌面全局焦点、跨进程或 Narrator 人工验收。

ToggleState 通知统一来自共享 Switch Session 的实际值变化，包括 UIA、鼠标、键盘、
同步与结算。Mount dirty 标记避免每次 Host 操作扫描 Switch；窗口按当前 runtime
identity 合并变化，以单个私有 PostMessage 延迟到输入事务之后发送。通知不跨
Host/RefCell 借用，重入只收集变化；发送前复查身份及实时值，替换/关闭撤销旧记录。
这是可合并的当前状态通知，不是每次切换的动作日志。真实 MTA 订阅已验证 UIA 及
自有 HWND 鼠标/键盘消息路径，未替代跨进程或 Narrator 人工验收。

Switch UIA Toggle 不维护独立 checked 状态：读取共享编辑 Session，并沿用 change
FIFO、本地 editing 与 submitting 分离、pre-toggle 祖先 click fence。显露/EDIT
同步后复查身份与权限，提交点不再进入 Win32。独立 Toggle COM 头共享节点生命周期，
二态通知经共享窗口出口延迟发送，不在 Toggle 回调内同步 Raise，避免打断祖先
click 候选的提交事务。

Invoked 通知已建立 MTA 系统订阅证据：回调消费 cache 身份，不回读 UI State；
注册/注销同线程，handler 由 COM 引用计数释放。一次 provider raise 可能被系统
重复交付，通知次数不用于动作次数或完成判断。发送计数仅用于测试，生产不增加
重试、去重队列或通知状态；其他 UIA 通知与跨进程验收仍独立推进。

Button UIA Invoke 使用独立 COM 接口头并共享节点身份，进入既有有界候选链；
Automation 来源按节点准备事件但不依赖逻辑键盘焦点，仍受 pointer identity、
本地编辑/提交许可、绑定和 FIFO fence 约束。Invoke 不直接发送业务请求，也不
自行设置焦点；系统客户端的 AutoSetFocus 由 UIA 控制。鼠标 press/已有 pending
会阻止新自动化入槽，避免已接纳请求被 mouse-up 覆盖。Invoked 通知只表示接纳，
不冒充远端完成，通知交付专项和其他模式仍待验证/接入。

UIA 焦点沿用现有键盘 owner 和滚动显露。窗口根声明 ProviderOwnsSetFocus，
先检查本地编辑、可见性与 enabled，再执行 Win32 SetFocus；取消 capture 和
系统焦点调用之后重新验证场景与权限。声明式 Button/Switch 的 GetFocus 和
HasKeyboardFocus 共用真实 HWND + 逻辑节点判定，原生 EDIT 不重复暴露。
提交暂停不禁止本地聚焦，SetFocus 不触发业务事件；模式和焦点通知仍未接入。

UIA 几何查询不把屏幕矩形复制进长期快照，而通过 UI 线程 Weak<State> 注册表进入
当前 Host，先同步 viewport，再复用滚动后的布局和输入命中。完整 bounds 与裁剪
可见性分开；窗口移动即时转换屏幕原点。查询前后以 runtime-id/closed 检查旧
provider，busy/重入关闭失败时不返回默认几何。窗口 State 与消息回调采用 Rc
保活，COM 只持 HWND/线程/撤销证据，不持 Rc/State 指针。焦点/操作/通知仍待接入。

UIA 只读 Fragment 树使用窗口级稳定根与场景级可撤销节点两层身份。Simple、
Fragment、FragmentRoot 是三个独立原生 COM 接口头，共享 IUnknown 和引用计数。
节点强持根接口，根不缓存节点；关系来自 Weak 语义快照，旧场景不能通过同 ID
重新绑定。runtime-id 包含场景唯一身份和索引，以独立 SAFEARRAY 转移给客户端。
实际 MTA UIA 客户端已验证节点遍历及替换后重查。原生 EDIT 留给 HWND provider，
暂未精确重定位；子屏幕几何、根命中/焦点及操作/事件仍是明确缺口。

SurfaceWindow 按需通过 WM_GETOBJECT 暴露一个稳定窗口根 UIA provider。
根是 STA COM 对象，独立于声明式快照身份；只声明名称/framework/Pane 类型，
其余属性委托 HWND host，模式返回成功空指针。关闭先撤销 owner，再销毁窗口并
清理 UIA raised-event map；外部引用不含 State/Host，不能读取已销毁的 HWND。
系统 UIA 客户端的同进程跨 apartment 测试已覆盖根读取，尚不提供声明式子节点
Fragment、操作模式、通知或 Narrator 验收。前一轮快照层仍待接到 Fragment。

UIA 前置语义层 `surface_accessibility` 使用不可变有界快照与原子撤销标记。
窗口拥有唯一强引用，客户端描述句柄仅持 Weak；场景替换先撤销旧身份，成功后
安装新身份，关闭与销毁统一撤销。跨线程读取不进入 Host 或 UI RefCell，不持
HWND，不复制输入内容/动作。此层尚非 COM provider，不实现 WM_GETOBJECT、
导航、模式或事件；已返回静态副本也不具备运行时权限。

Switch 绘制把数据与交互视觉分开：本地 bool 只覆盖当前帧滑块/轨道，纯 palette
计算决定 Neuro hover/pressed/声明 disabled 状态色，不修改权限或 Session。
每帧从静态几何复制，因此双向切换不会累积位移。96 DPI WIC 像素测试复用统一
COM/画刷/裁剪恢复夹具，尚不作为真实硬件、全局门禁暗化或 UIA 证明。

Switch 祖先 click 现在复用现有候选/授权通道：激活前封存表单值和字段 fence，
本地 toggle 入 FIFO 后，祖先 click 只依赖此前前缀，不依赖自身 change。
`surface_edit_prepare` 支持截止到 fence 的准备循环，消费无 handler 前缀后也
交回授权层，避免后续字段事件越过点击。原生单次激活只生成一批祖先 click，
不复制旧 DOM label 转发导致的重复动作；真实浏览器与硬件验证边界仍保留。

Switch 本地状态现由 Mount 内无 HWND 的 Session 持有，复用字段 Draft ticket、
共享 FIFO、依赖 fence 和预算；鼠标/Space 激活只要求 editing，不绕过提交阶段
的 submitting/binding/revision 门禁。Host 绘制即时借用 bool，不重建布局。
合同更换先删除旧会话和旧意图，再创建同 ID 新会话，旧回执不能跨合同影响新输入。
结算后若采用权威值，父窗口获得重绘请求。正式能力报告仍关闭 Switch，UIA、
完整视觉及 DOM 祖先 click 冒泡对照尚未完成。

Switch 首先接入纯原生静态测量与绘制链：固定 DIP 轨道/滑块由 PaintFrame 按
场景顺序绘制，声明 true 为 on，复用父裁剪/opacity，不引入子窗口或动画资源。
事件编码已可表示严格 bool change/commit，但尚没有交互状态 owner，正式能力
报告与焦点集合不开放 Switch。后续本地 toggle 必须与 submitting 权限分离，
接入既有字段事件生命周期，而非把控件伪装成 Click 队列里的 Button。

Number 的删除预检复用插入候选 grammar：非组合状态的无修饰 Delete/Backspace
和 WM_CLEAR/WM_CUT 在默认 EDIT 之前验证；拒绝不回写、不清空既有原生 undo。
删除范围遵循实测 classic EDIT 的单次 Delete / 重复 Backspace（先删选区）
差异。预检没有剪贴板副作用，合法剪切仍交系统；真实剪切未测试，修饰键词删除
及组合状态仍有 Session fallback，不声称全路径撤销历史无损。

非组合 Number 的 WM_PASTE 使用一次有界 Unicode 剪贴板快照，锁外将同一份
数据送 EM_REPLACESEL，不采用“先检查、再默认 WM_PASTE 重读”的双读方式。
临时 HWND property 标记在读取期间的编辑/选区/焦点/组合/权限/窗口变化中作废；
标记 Box 持续存活，旧 HWND 重用也不能接收旧快照。父窗口 CAN_PASTE 要求当前
Host 拥有可编辑且非组合的 Number，Host 忙时拒绝。合法输入继续走原草稿和
事件 FIFO；禁提交可本地粘贴但不能发送。无剪贴板监听、写入或后台重试。

Number 的可预判插入新增编辑前检查：普通 WM_CHAR 和 EM_REPLACESEL 结合当前
UTF-16 选区形成完整候选，非法结果不进入默认 EDIT、不修改原生撤销记录。
合法插入继续交给原生过程，不另建撤销栈，不改变调用方 canUndo 参数。输入
消息指针仅同步有界读取；组合中的粘贴、IME 和其他未预检路径继续走 Session 边沿，
这些兜底路径的回写恢复仍不保证原生编辑历史无损。

Number 原生输入分为编辑态和提交态：符号、小数点、指数的不完整字符串可暂存，
最终有限值仍由严格 parse 转 number/null。普通非法键入先拦截；粘贴/替换结果
在同一编辑边沿验证，结构不合法时恢复原草稿及编辑前选区，不生成输入事件。
组合输入留到结束处理，Input/Textarea 不进入数字过滤。整段拒绝的回写恢复尚不
保证撤销历史、水平滚动或 caret 方向无损；局部化与真实 IME 仍待完成。

原生 Input/Number/Textarea 的 placeholder 是 EDIT 默认绘制后的提示层，使用
Neuro textMuted #929a9f，只在实际文本为空且非 IME 组合时出现，空字段持焦仍
显示。提示不是默认值，不能进入草稿、表单或事件；不创建辅助窗口/网络 owner。
不依赖仅支持单行且要求 v6 common-controls 的 EM_SETCUEBANNER，也不全局切换
manifest。当前 classic comctl32 缺少 GetWindowSubclass 导出，使用项目既有
HWND property 身份检查配合 Rc 生命周期；真实 IME/屏幕重绘和完整文字样式仍待验收。

Number 鼠标步进使用系统 up-down 子窗口，EDIT 保留字符串草稿。UDN_DELTAPOS
返回非零，禁止系统整数 position/buddy 回写成为第二份数值状态；仅把有效正负
单步送入现有 min/max/step 和 input/change 成对容量门禁。切焦不持 Host 借用，
切焦后重新核对字段身份与 local_edit。撤权、布局同步、取消、失活、尺寸/DPI
变化清捕获；完全空的裁剪区域禁用通知准入。未新增网络、线程或应用定时器。
系统按钮主题、真实长按、完整视觉/UIA 仍未验收，number 能力报告仍关闭。

Number 原生编辑复用已有 EDIT/草稿/FIFO owner，而非把状态改成 f64。权威值仍
只接收字符串，有限数值仅用于事件 payload，未完成草稿保留在控件中且表单值
为空。上下箭头步进走本地编辑门禁，不获得提交权；步进 input/change 成对准入。
min/max 缺省均为 0 是旧 Surface mapper 的实际合同，不改写为无限范围；手工
输入不因越界被钳制。鼠标步进已有上方增量实现；完整视觉仍未完成，能力报告未开放 number。

原生声明式 Icon 使用 glyph 文字（缺省为 •），并映射 Neuro signal-primary
到 #d9ff38。字形共用有界 DirectWrite 布局、裁剪与滚动缓存，不支持资源 URL，
不成为独立动作或焦点 owner；点击仍可沿既有合同路由到有声明的容器祖先。
新建/恢复的宿主报告共同新增 icon，不由此宣称 image 或其他表单节点支持。

Surface 触控滚动通过父窗口 WM_GESTURE 接入 Windows pan/系统惯性，单次序列
固定起始 anchor，并在同一缓存帧上原子计算双轴位移。local_edit 撤权、换场景、
窗口尺寸/DPI 变化、失焦、捕获丢失和关闭清除旧序列；Host 暂忙丢弃本次位移，
不积压补跳。sequence ID 仅作相等匹配，不是权限或硬件身份。真实触控硬件与
EDIT 子窗口互操作尚待验收，不宣称浏览器触控行为完全等价。

未发编辑 FIFO 带 30 秒单调期限，派发准备前检查并截断过期依赖链；没有新定时
线程，不删除在途证据或重试已尝试事件。过期不清用户文字，清理失焦提交标记，
需要用户新的输入才能产生新意图。取消恢复原期限，不能通过反复预留延长旧动作。

编辑会话现以完整节点合同摘要复用，仅根 props.value 更新可跨越本地草稿生命周期。
摘要涵盖其他属性、布局、样式、可访问性、事件和子树；任何变化重建字段 owner。
摘要是进程内比较辅助，不替代远端 lease/binding/revision 证据，也不授权旧事件重放。

编辑 FIFO 的失败依赖边界由批次证据交付驱动：前置失败/未知清除未发意图而不
清用户文字，不采用未验证权威值。明确未启动可归还；有效确认等待保持暂停语义。
事件 ID 必须属于当前 Mount 登记，旧会话回执不得影响新队列。此路径不新建网络
owner，也不授权失败后的自动重试。

文本输入从字段布尔标记改为 UI owner 内的有界 FIFO：waiting + prepared 最多
32，只合并队尾同字段 input。change 保存边沿值，Session 在准备时不重新采样
较晚文字；后续排队草稿由独立本地 ticket 保护。网络提交/完整证据结算仍走原链。
此编辑队列尚未包含按钮及前置执行依赖，不能等同于完整 Tile 队列合同。

文本编辑会话复用还要求 events 声明一致；动作更换/增加/移除会重建字段 owner，
其本地草稿、phase 和 ticket 不跨合同复用。此边界不取代 lease/binding/revision
验证。完整输入顺序迁移须区分普通 Surface 与 Tile 的发送合同，不能通过放宽
旧候选场景检查或无依据的同版本拼批来宣称等价。

Click 候选现携带激活边沿的有界字段快照，而不是发送准备时重新读取草稿。快照
与候选同寿命、同单槽容量；门禁关闭/场景更替照常清除，不跨新场景延续。快照
解决点击的数据身份，尚不解决编辑 phase 与离散点击之间的完整调度顺序。

原生 Surface 表单汇总由 Mount 内的 `surface_form_values::Values` 拥有；提交缓存
最多 16 字段、48KiB UTF-8，与可见编辑草稿分离。鼠标及键盘动作在当前 Host
准备时读取缓存，按 includeSurfaceValues 合并到原事件编码器，继续通过同一授权
批次路径；没有新增任务、网络 owner 或旁路。换绑清空缓存，场景移除字段保留已
登记值，与现前端一致。离散点击时刻快照/输入事件调度仍需继续对照，不能将准备
时快照当作整个异步交互已经等价。

EDIT 编辑命令现通过开始/结束通知在 UI owner 内合并中间 EN_CHANGE；它是同步
消息边界，不是新任务队列。textarea 用户 maxlength 在规范化 LF 后按 UTF-16
检查，权威值与用户编辑限制分开。超限拒绝恢复原草稿，IME 收尾仍清理组合预留。
整次拒绝的选择区/撤销行为尚未声明与浏览器一致。

原生 Surface 通过 `surface_scroll` 保存有界双轴偏移，并在已提交布局上统一生成
滚动后的文字、裁剪、命中和 EDIT 位置。窗口滚轮路由不发送动作，读取系统偏好；
纯滚动复用 `surface_scroll_frame` 的未平移布局与 DirectWrite 布局引用，不重新
测量文字；场景、尺寸或样式变化重建缓存。每次仍重建几何/绘制/命中快照，不能
据此宣称零分配或完整性能达标。滚动条已增加不挤动内容的覆盖层，auto 溢出轴
显示轨道/thumb，hidden 轴不显示；支持分页与单槽捕获拖动，EDIT 区域让出条区域。
按住轨道使用仅手势存活期间存在的 RAII timer：400ms 首次等待、50ms 后续间隔，
每 tick 至多一页，并按最新几何重新命中；到达 thumb/离开原轨道时暂停分页，
取消/松开/失焦/场景与权限失效统一释放。timer 身份不复用，旧排队消息无效。
父 HWND 持焦时，无修饰方向键按 40 DIP、PageUp/Down 按页、Home/End 按纵向边界
滚动；以当前焦点/背景点击 anchor 从内向外消费，hidden 轴不接受手动滚动。
方向键可自动重复，滚动不清逻辑焦点但废弃旧动作候选；EDIT 持焦时仍处理自己的
光标/编辑按键。背景 anchor 不授予动作权限，也不代表独立 scrollbar UIA 已完成。
这是原生 10 DIP 的呈现选择，不是浏览器 thin 的精确像素合同；触控和可见桌面
验收仍待完成。键盘导航通过
`surface_focus_navigation` 显露屏外候选后提交几何并同步 EDIT；导航集合不授予
动作权限，不可达目标不会留下滚动，EDIT 导航受本地资格和实际焦点来源约束。

原生 Surface 点击通过 `surface_window_deferred` 保存激活快照和 30 秒单调期限，
以编辑队列序号屏障等待前置结果，再由 AuthorizedWindow 复核合同与授权。
后续输入不得超越依赖点击；该模块不拥有网络或远端执行权限。

原生 Surface 字段现由 `surface_edit_mount` 在 Host 中拥有，激活确认后绑定并按
布局挂载真实 EDIT；Windows 通知只更新 UI 草稿/待发 phase，Application 沿原
授权批次路径取得唯一 owner。结果经过完整证据检查和场景替换后才结算 ticket。
`surface_edit_messages` 保留默认 IME 过程；硬门禁暂停显式收尾组合，软暂停只
阻止发送。窗口 region 处理祖先矩形裁剪和后绘制节点保守遮挡。完整样式、字段
汇总和输入边沿兼容仍未完成，不据此扩大能力报告。下文为此前检查点。

`surface_edit_session` 在 UI 线程组合单字段 EDIT、草稿及有界事件 ticket，不发送
网络请求，也不替代 Surface 授权。会话按绑定/节点身份存活，保留后台请求期间的
后续本地编辑；组合输入期间延迟权威回写，取消组合不冒充提交结果。当前只有
隐藏控件合同回归，尚未接入生产窗口消息、焦点、布局和 IME 路由。

输入门禁已拆分：Application 的编辑意图受可见性/modal/origin 等条件限制，
AuthorizedWindow 复核租约和已确认活动生命周期；提交另受 owner 独占和结果
门禁限制。后台提交软停止只使候选/按压失效，保留逻辑焦点；硬失效和租约撤销
仍完整清理/关窗。成功场景替换仅保留仍合法的同 ID 焦点，所有旧候选作废。
这尚不是 EDIT/IME/草稿的生产挂载，不能据此宣告文本输入控件已经完成。

文本输入的 wire 编码复用 click 的不可变事件与输出预算；单字段草稿独立于窗口
指针/按钮候选生命周期，按 scope ticket 防止旧结果污染新编辑。当前已有合同与
真实 EDIT 本地回归，但尚未接入生产挂载。现有后台批次会清逻辑焦点并停止键盘
处理；必须先分离“本地编辑仍有效”与“动作可提交”，继续保留唯一 owner、租约、
版本和完整结果证据门禁，不能简单放开原来的统一输入开关。

文本输入新增 Windows 内置 Unicode EDIT 资源 owner：线程限定、隐藏创建、
窗口属性身份检查、有界 UTF-16/UTF-8 读写，复用系统字体，不引入额外 UI 引擎。
当前未连接 Surface 输入节点；后续需连接焦点、通知、草稿和事件生命周期，
并验收 IME、样式/裁剪及 placeholder。此阶段不能声明 input/textarea 已支持。

按钮交互反馈通过 PaintFrame 中的动态颜色选择，不通过场景替换或文本重排。
原生 HWND 仅在 hover/press/focus 状态变化时失效绘制区域；leave 使用系统消息，
没有应用轮询/动画计时器。显式样式保留优先级，禁用节点不应用动态色。客户区
光标复用系统资源。宿主现声明 button 及 pointer/hover/keyboard，未实现的节点
和触屏/脚本仍不声明，不能据此推导完整迁移或无障碍验收已经完成。

按钮键盘路径使用 HWND 焦点加布局时生成的可聚焦节点快照：Tab/Shift+Tab 更新
逻辑焦点，Enter/Space 生成与鼠标共用的有界候选。准备事件直接校验节点而非假造
坐标；既有 owner/版本授权仍是发送前置。焦点变更使旧候选身份失效，按键回调不
执行网络动作。焦点线是节点内的有序 PaintFrame 命令，遵循组透明度、裁剪和
兄弟遮挡，不额外持有 GPU 资源或启动重绘循环。悬浮/按下反馈仍待补齐。

声明式按钮复用原生文本与绘制帧：场景节点提供缓存的静态默认样式，NativeMeasure
与 TextFrame 共用双轴内容盒，DirectWrite 排版 label，Direct2D 绘制背景/边框。
点击继续经过既有 HWND 候选、场景身份验证、绑定和事件编码，不因新增控件绕过
权限；disabled 抑制动作且按钮停止冒泡。当前仅基础呈现和指针事件链路，键盘
焦点及动态交互反馈未完成，生产能力报告仍不声明完整 Button 支持。

模糊分块的插值邻域现在按设备像素足迹计算，而非固定 1 个源像素。局部纹理
缩小时，过窄邻域会让块四边形的覆盖率误差进入接缝。固定 512 源网格结合二次幂
邻域桶保持缓存复用，上传矩形裁到源尺寸；已有缓存容量限制不变。分块与整图的
组合回归不等于所有倍率、浏览器插值或驱动资源峰值已经验收。

源读回的插值模式属于 Layout：底图、擦除和普通导出维持最近邻，局部 blur
显式使用线性平滑取样，再进行局部卷积。共用 WIC/D2D 读回不全局改变插值。
该路径没有新增 CPU 中间缓冲，但线性取样与浏览器各缩放算法的完整像素等价
仍需实际基准，不作为已经完成的全面视觉验收。

工作流 blur 按稳定标注 ID 读取持久效果框，独立投影和卷积，不再模糊整个可见框。
原图方向惰性恢复；现代 crop 使用直接源像素偏移，不套底图 contain/density 或
传播框。框外邻域透明，相同持久框及取整绘制半径共享资源。整个效果框裁切约束
白色覆盖，源投影裁切只约束模糊颜色；失败/取消不写回活动历史。完整笔迹几何
按局部画布的双重 ceil 比例缩回持久框，wash 排除像素取整填充。局部效果绕持久
框中心旋转，裁切使用几何 layer，避免旋转后的轴对齐包围框泄露颜色。D2D 取样
与擦除共享预算读回模块，不提前应用装饰。kernel、离屏 rasterization/插值及
复杂组合仍待完整对齐；几何层的内部资源开销也需实测，不能宣称全面视觉一致。

Workflow base-image layout now consumes propagated source/content frames without
copying image strings. Containment is resolved in the target frame; ordinary flips
mirror placement around that frame, while logical crops flip the image in place.
A base-only clip is popped before borders and annotations. Nonuniform crop viewports
contain natural images directly rather than stretching a prior containment result.
Erase history uses the resulting placement. Blur retains its independent effect-local
sampling path; full blur parity remains a separate work item. Legacy crops without
sourceSize map bounded natural-source intersections into destination rectangles,
with explicit empty sampling instead of giant full-image placements. Whole-frame
flips transform source and destination; out-of-source areas remain transparent.
Without erase strokes D2D samples the source directly. With strokes, the same D2D
path materializes the final-size base under explicit WIC/readback budgets before
erasing in visible coordinates, preserving local marks even when one source pixel
is magnified. Empty paths do not materialize; cancellation does not exchange Frame.

Origin-workflow messages retain full image fields under the current replacement
contract. The inspected Loom bridge converts each received snapshot to YAML and
atomically replaces the prior file; absent src/previewSrc fields are not merged
from previous state. Sender-side signatures or ACK caches alone therefore cannot
authorize omission. Incremental images require an explicit receiver contract.

Workflow baking now resolves the composite base through a saved-session adapter
to graph_image, including upstream formal outputs, manual inline images, relay
switches and capability ports. An independent raster annotation layer keeps the
node's own source priority. References remain borrowed inside the adapter callback;
external images still require the explicitly admitted Root. The bake reuses local
decoded pixels or loads one selected image with the target's flip flags.

The archive worker consumes its pre-archive Ready into a private ArchiveInput
before publishing resources. This drops the obsolete baked snapshot and its
encoding leases on the worker, while retaining original bytes and their source
lease through staging. The archive-only input exposes no network batch API;
sync still prepares a fresh snapshot from the committed revision.

Workflow directory selection now retains a pre-dialog Receipt and revalidates it
after the modal picker returns. A changed workspace or capability identity cannot
inherit the earlier selection. Archive batches also carry the validated adoption
plan: originWorkflowId/originNodeId are committed with resource references and the
new revision, then preserved through the Record's saved Source on later saves.

The application-level workflow integration test now exercises a hidden Win32
workspace, real session/archive writes and a pinned local TLS/WebSocket server.
Before acknowledging, the server verifies that the transmitted revision is
already committed on disk and that the archive index points to a published file.
This proves the isolated success path, not installed-Loom or visible-UI acceptance.

Native workflow UI now requires an explicit writable asset-directory selection
to start sync and enable save-triggered sync for the current Record. Successful
ordinary saves coalesce the latest Source intent; exit/open saves and archive's
own internal commit do not recursively schedule sync. Completion consumes reports
and recovery warnings. Disabling affects future save intents, not an in-flight
cycle, and directory authority is not persisted through session JSON.

A Record-owned workflow Cycle now coordinates archive preparation, UI commit,
re-preparation of the committed revision, sync and result consumption. The
existing workflow polling entry services these phases and retains both archive
publication warnings and partial sync reports. Explicit root admission remains
required; explicit workflow UI configuration enables ordinary-save triggering.

Archive jobs now stage an exact-baseline replacement after resource publication.
The UI archive owner revalidates its receipt, commits without pumping messages
and swaps the saved Source and Stamp only on success. Recovery warnings remain
part of the result. The Record-owned Cycle re-prepares the new revision for
network sync; the workflow UI owner schedules explicitly enabled save intents.

Native archive batches now publish inline source/preview resources sequentially,
retain their handles and install processed references plus the archive index in
a next-revision session candidate. Unknown document fields and topology survive
assembly. Prepared retains resource guards through candidate use; production UI
replacement and baseline publication are not yet connected to this path.

An explicitly admitted native asset Root can now publish encoded or inline
images through same-directory staging and non-overwriting commit. Legacy FNV
names are retained, but existing bytes are verified before reuse. Published
resources hold read-only shared handles through the caller's session commit;
this does not implement multi-resource transactions or orphan collection.

Workflow Completion retains the original UI receipt across result consumption;
stale or failed completion cannot expose Ready for current-workspace updates.
The archive-index candidate replaces only hinted workflows and keeps unrelated
entries, accepting published image references rather than inline payloads.
It performs no resource writes, revision advancement or durable session commit.

The native workflow sync worker now transfers each admission request through a
single-slot mailbox with a one-shot reply. UI-owned Sync retains and rechecks
the receipt; workers own only prepared data and pinned identity. Cancellation
and Drop terminate pending admission and join the worker without erasing partial
acknowledgement. The normal UI loop services the current Record's sync owner
after catalog updates and retains unconsumed delivery. Automatic save triggers,
delivery consumption and archive commit are not yet connected.

Native workflow batch dispatch now connects encoding and transport sequentially.
Admission is renewed after each pinned-identity connection; only a validated
success advances the batch. Failures retain the pending request and report
partial acknowledgement without automatic retry. Application integration still
needs a UI admission handshake; background callbacks must not read UI owners.

Native workflow requests now have a single-attempt TLS/WebSocket exchange using
an admitted BridgeSocket. Responses must match protocol, request ID and workflow
ID; the exchange has a five-second deadline and bounded cancellable reads. Its
send copy owns a separate budget lease until the socket is destroyed. This is
not yet wired to application scheduling, authorization revalidation or archive
commit, and it never advances a batch automatically.

The native workflow preparation result exposes a borrowed, lazy request batch.
It retains only one encoded envelope and its pre-admitted byte-budget lease;
retries preserve the request UUID and bytes. Explicit matching acknowledgement
advances the batch but does not verify a remote response or commit origin data.
Transport authorization, real acknowledgement validation and archive commit
remain separate, unfinished integration responsibilities.

This document describes the current implementation. Historical plans and
migration notes are intentionally not part of the active documentation set; Git
history and the `文本优化前版本` tag preserve them when historical investigation
is necessary.

## 1. Product and runtime model

工作流准备已有独立后台任务：输入指定版本会话、能力目录及可选的已准入资源根，
输出真实合成快照与来源计划；取消、单次通知/消费和 Drop join 由任务 owner 负责。
结果不证明保存已提交或授权已获得。Application 现有基于已提交且未变化会话的
准备/取消/领取入口，任务随 Record 存活，交付前后均可核对来源/工作区/能力目录。
大段字节复制放在 worker 并计入预算。用户触发/自动调度、归档和实际发送仍需连接。

快照及来源视图现可编码 loom.hook.workflow.sync 控制请求，保留 requestId 与
workflowId 的实际协议字段，并对整个消息限长/取消。前端 action 包装仍独立保留；
编码成功不等于远端接收或确认，授权、传输及 ACK 处理仍需接线。

源分辨率 PNG 已有独立合成入口，显式自然源与逻辑坐标分离，并已接入恢复候选的
工作流内容准备。返回快照持有新编码图片的预算租约，仅提供借用访问。普通导出
不变；继续编辑或含草稿的候选不能恢复旧源。边框已按逻辑线宽与自然输出密度
绘制，位于底图之上、独立标注之下，不继承底图透明度。圆角蒙版覆盖整个合成栈，
按可见逻辑框裁剪并独立计入像素预算，层在 EndDraw 前释放。遗留整图 rotation
按当前主干不额外变换，标注旋转独立处理；完整工作流同步仍未连接。

历史内容擦除现先物化到自然底图，再执行边框/标注合成；含裁剪偏移的可见框路径
映射回自然像素，原共享图不变。历史笔宽允许随贴图缩放，交互工具范围保持不变；
路径数量与分配受预算限制，取消或失败不返回部分结果。

批量工作流烘焙复用外层内容策略已经解析的节点，避免每张图片重解析整个会话；
公共单张入口仍按 UnitId 查找，不允许外部任意节点与候选像素混用。

模糊内核现有独立双轴像素密度入口，保留逻辑强度并使用有界高斯核/环形行缓冲。
现有整数与精确强度入口合同不变。BlurImage 已分离像素尺寸与逻辑采样框，整图
及固定网格分块渲染使用同一映射。自然源效果准备现已接入工作流烘焙，按强度共享
资源并仅替换局部导出快照；失败不写回编辑器。持久裁剪的输出尺寸和原始 contain
映射经 Frame 统一提供，非等比拉伸不会重新引入留白。仍需浏览器视觉对照和人工验收。

工作流美化在内部合成后包装：留白、预设背景、圆角白卡片及下移高斯阴影不改变
节点逻辑尺寸，关闭时保持原路径。直接读取 WIC bitmap 并在后台处理，所有额外
像素缓冲受预算约束，逐行可取消；不通过 PNG 往返完成中间处理。

来源采用和资源归档提示已有只读规划，精确匹配原会话；提示不等于实际资源索引
或持久化成功。工作流烘焙使用独立的自然底图、逻辑几何和透明度合同，而不直接
调用逻辑分辨率导出；当前不发送近似图片冒充对齐结果。

工作流内容策略已统一参数过滤与直接图片/贴图烘焙选择。Art 要求唯一匹配能力，
贴图有编辑时必须成功烘焙，不能回退原始底图或泄漏原参数。该策略接入 Snapshot
构造入口及桌面恢复候选的真实烘焙调用方，但应用调度、归档提交和授权发送仍需连接。

来源工作流视图现从同一精确会话规划连通分量，保留全局身份，仅在来源视图重映射
节点与连线。视图借用全局准备内容，同来源断开分量合并，冲突拒绝；源摘要防止
不同会话共用相同 revision 时混入来源信息。尚未执行应用来源更新或网络同步。

全局 hook-live 快照已有独立会话映射模块，携带指定 documentRevision 并有界编码
workflow_id/snapshot。参数/图片显式准备，不复制未知私有字段；该模块尚未接入
应用或发送器，不能将 payload 准备成功等同于持久化或远端同步成功。

会话内联图片输出现可恢复为正式 Image 端口，重复 Data URL 复用解码像素；已有
预览优先，无预览时从正式图片选择显示。后台有界解码全部成功后才安装，外部
URL、文件路径、资源描述符不自动访问。仍保持 Idle 和独立的新挂载授权边界。

正式混合输出保存已覆盖本地 Image：UI 只冻结共享像素引用，后台编码 PNG Data URL
并与 JSON 值整体写入 outputs，嵌套字符串租约保持至候选释放。恢复目前仍将这些
字符串视为纯数据，正式图片端口解码/显示及远端同步尚待接入。

会话正式 JSON 输出已进入后台 Candidate 的 Art 结果恢复，保持 Idle、无请求及
无 Surface 绑定。恢复只提供纯数据，不将资源描述符转换成联网/文件访问权限。
端口解析限制数量/名称、拒绝重复键并整体安装；图片型输出资源解析和远端同步
仍未完成。

正式 JSON 输出现在随原生会话保存快照冻结，后台整体替换 outputs；正式空结果
清除历史值，未产生正式结果则保留导入字段。已有 OutputStamp 防止保存期间的新
输出被旧候选覆盖。resource/stream 只保存描述符；混合图片端口、恢复接线及
远端工作流同步仍待完成，本地保存不代表远端同步成功。

新挂载窗口现在负责一次性注册节点的 Surface 写入资格，普通输出不能自行首次
绑定或换绑。授权刷新可能同步关闭 HWND，因此节点和界面状态的借用在刷新后
的回调内获取；成功消费资格，失败保留。普通 Loom 执行撤销资格后，旧窗口
不能再次注册。完整重挂载交互仍待端到端验收。

Surface 新结果现已在释放结果借用后进入既有图传播入口，同轮按节点合并通知，
旧/重复版本不再传播。复用普通正式结果的下游策略和执行准入；无新结果不
分配通知缓存。工作流同步尚待完成，不把传播通知当作同步成功。

正式 Surface 输出已进入应用节点交付：从已呈现收据借出 Commit，重查当前
服务、窗口授权/显示版本和 Art 节点后写入 Results。保留原 Commit 供恢复，
输入按节点已应用版本解除输出阻塞。结果不能擅自换绑，也不能在普通 Loom
执行撤销资格后自动重绑。工作流同步及完整重挂载交互验收仍待完成。

Surface 节点结果已有独立写入资格和单调版本管理，复用 Results 存储但原子
替换全部正式端口，不使用普通 Loom 结果的合并语义。空输出清旧端口，旧版本
不变更凭证，旧附件不能覆盖重挂载状态，普通 Loom 执行撤销 Surface 写入资格。
应用层授权交付、传播/同步尚待连接，不能视作正式输出已全链路应用。

Surface 正式结果已有节点值适配：与现有前端一致解包 value/resource/stream，
保留 null/false/0/空串和描述符扩展字段。资源/流仍是数据，不授予下载/订阅
权限；该层不执行脚本或访问路径。原始 Commit 保留，节点原子应用尚待连接。

生产 pointer 已按授权生命周期接线：创建时关闭，当前服务下可见、已确认激活
且有容量/无未结算证据的窗口才启用。模态、退出、服务切换或 owner 在途时
关闭；待应用输出和未呈现收据阻止新点击。状态不变不丢弃候选，状态切换使
旧候选失效。生产端到端交互和输出应用尚未完整验收，不能视作迁移完成。

HWND 点击候选已有应用消费到批次任务的链路，容量检查和结果预留先于 owner
移交，启动失败保留完整输入供结算。无候选时不准备配置/租约，退出结算不
发送新输入。pointer 生产开关已接入授权生命周期，完整运行验证尚待完成。

显式接续现已接入原生菜单“继续 Surface 批次的剩余操作”。菜单候选须通过
窗口当前授权与版本审核，点击仅排队原批次 ID；应用执行前重新检查服务/
设备、退出、新请求和窗口，再移交 owner 启动任务。不自动继续，不批准新
提示，不重发完成前缀。真实菜单操作仍待人工验收，生产点击默认禁用。

显式接续选择事务已接入应用集合，使用原批次 ID，过滤不满足接续边界或窗口
不可用的候选。取消保留原顺序和完整收据，启动失败将 owner 与前缀证据返还
既有批次交付流程；不会因为后缀未发送而丢失前缀。原生菜单已接线，实际交互尚待验收。

接续 worker 已加入应用集合的容量、正常/退出等待、取消与收割，按完整批次
计入未保存预算。结果沿用现有批次交付和恢复记录；在途不能生成不完整快照。
三类批次任务统一预留完成缓存。显式接续选择已连接，菜单入口尚待连接。

批次接续已有后台 worker，重新审核边界并仅提交原事件列表的未尝试后缀。
原前缀决定与 ACK 保留，新结果追加，观察失败不覆盖提交事实；取消/迟到撤权
返还完整收据并独立标记交付权限。接续应用任务集合已连接，显式入口尚待连接。

确认后的批次接续已有证据审核及唯一 owner 移交入口：仅允许成功前缀与全未
尝试后缀，最后确认项必须已批准并接收。窗口当前授权/版本重新审核，整批
身份保持独占，失败完整返还。移交本身不发送后缀，接续 worker 已实现，应用入口尚待连接。

批次确认已接入原生菜单请求和 Win32 弹窗：默认取消，提示前后复核窗口授权/
期限，模态返回后再审核退出与服务切换。仅明确选择后消费提示和移交 owner，
失败保留收据或未提交决定。现有菜单候选包含单项与批次，不自动弹窗；实际
用户交互尚待验收，生产点击仍默认禁用。

批次收据选择事务已按事件 ID 接入应用集合：先检查提示状态/期限，再由调用方
复核窗口可用性；明确选择后整批移入回调。取消完整返还，启动失败保存未提交
决定证据，未提交后缀不执行。原生批次弹窗与菜单已接线，真实交互尚待验收。

批次决定 worker 已接入 Sessions 的容量、等待/退出句柄、取消和收割。成功
回填使用现有批次结果交付与恢复日志；异常回填不丢弃输入，单独占用有界容量
并阻止输出不完整的退出 bundle。收据选择事务和应用层确认 UI 已接入。

批次决定结果已可并回原批次：只替换选中项观察，逐项保留决定接收证据及其他
事件状态。整批交付额外检查决定提交、确认、身份和版本，恢复记录带脱敏决定
提示；不自动批准或重放未提交后缀。应用任务调度已连接，批次确认 UI 尚待接入。

批次选中项的决定已有后台 worker：单次提交后观察原动作，完整收据和唯一
owner 返回，只更新目标 ACK，未提交后缀不动。提交/收敛/交付授权分离，
复用共享容量与取消等待。结果合并回批次、决定元数据和应用任务调度已接入。

明确选择后的批次决定已有唯一 owner 移交接口，携带完整收据和决定，当前
授权/版本/提示期限重新审核。允许引用已提交原事件，但拒绝重发已尝试决定；
失败完整返还，成功移交本身仍不发送网络。批次决定 worker 已实现，应用调度尚待接入。

批次待确认项已有窗口审核与显式选择准备接口，索引、当前授权/版本、ACK 和
提示期限都需匹配。准备结果同时持有整批收据和单项决定，其他事件证据不被
拆散；失败返还原收据，成功也不移交 owner、不发请求。批次接续任务尚待连接。

批次恢复元数据已接入正常退出 bundle：逐事件保留尝试、ACK、收敛/输出版本，
附加批次首事件/顺序/数量和整体呈现阶段，缺项或乱序记录拒绝编码/解码。
未交付 owner 另保存附件提示；不保存 payload、确认正文或场景，不自动重放。

应用集合已接入批次等待/收割/取消和主循环/退出泵窗口结算，批次按事件数
共享 32 条未保存动作预算。结果和收据有界保留，拒收不丢失。批次恢复元数据
已适配；在途任务或不一致记录仍拒绝生成快照，点击仍未启用。

整批窗口交付已有 accept_batch/BatchReceipt：先精确恢复 owner，再独立核查
整批成功、逐项 ACK/观察、最高场景版本和当前授权。部分完成/等待确认不能
呈现场景，拒收完整返还，接纳后的非场景证据进入收据；替换后复核撤权。
应用收据缓存和正常退出恢复记录已接入批次结果。

批次窗口 owner 已有 begin_batch/finish_batch 移交接口，Activation 记录完整
有序事件 ID 列表，拒绝单项、缺项、乱序或其他事务冒领。移交前复核当前
窗口授权/版本，允许部分事件未尝试的原批次返还；恢复 owner 不更新场景。

批次观察已在需要确认时读取一次对应提示，独立保留提示结果、最后 ACK 与
收敛状态；读取使用同一整批截止且至多 3 秒，超时不延长预算。不自动批准或
提交后缀；批次批准后的恢复及应用交付仍未接入。

点击批次 worker 已在提交后逐项 GET 观察 ACK，整批共用 25 秒收敛预算；
每项保存最后 ACK、结果版本/错误及输出 commit，只保留最高版本的一份场景。
提交与观察结果分离，观察失败不会重放动作；候选快照不代表整批可呈现。
批次确认提示读取已接入，批准后恢复与应用交付仍待接入。

点击批次提交已有独立后台 Job，整批事件和唯一 owner 随结果返回，复用共享
线程容量与授权取消。单项提交异常保留此前 ACK；迟到撤权禁止交付但不抹去
尝试事实。批次收敛观察已接入，确认恢复和应用任务集合仍待接入，点击尚未启用。

点击冒泡已有有界顺序 HTTP 批次提交接口，整批预检后共用 10 秒提交预算；
按原顺序保留 ACK，遇到确认/失败停止，剩余事件保持未尝试，不自动重放。
该接口已连接批次提交/收敛 worker，应用调度仍未接入，原生点击入口仍默认禁用。

确认菜单选择还会调用对应窗口的当前授权检查；关闭窗口/旧场景的提示不能
挡住后面的可处理提示。刷新可能同步重入，选择阶段不持有授权 View 借用，
发布候选前再次核对所选服务。此筛选不消费收据，不替代实际提交前复核。

确认菜单候选和调度共用 confirmation_ready；过期或不再符合确认条件的收据
不挡住后面的有效提示。实际调度再次读取当前时间，过期菜单选择不会弹窗；
过滤不删除历史证据，也不能替代窗口与提交前授权。

授权菜单已提供“处理待确认的 Surface 操作…”显式入口，排队 event ID 后由
应用循环调用原生确认框和决定任务事务。共享模态 Scope 阻止其他模态操作，
消息循环不持有授权 View 借用。返回后重新核查退出/服务/排队请求，状态改变
则保留 NotStarted 证据，不提交。真实服务上的确认交互与点击启用尚未验收。

应用任务集合已有按 event ID 的确认调度事务：显示 UI 前检查关闭/容量并
预留结果缓存，取消放回收据，启动失败保存 NotStarted 和完整输入供统一交付。
不会自动弹窗或重试决定；用户入口、模态互斥和退出检查已连接此事务。

begin_receipt_decision 已将明确选择后的原收据转为决定任务 Input；失败完整
返还收据和决定，成功移交唯一 owner、原事件和 ACK。残留提示/旧决定/输出
或已呈现场景不能进入此转换，避免丢失其他证据；当前版本从窗口读取。

显式选择后的 prepare_decision 已从原收据移交确认提示，失败保留提示和 ACK，
成功不移交窗口 owner、不发起网络请求。它与后续 begin_decision/提交分别校验
授权和有效性，不允许复制提示来绕过一次性消费。原生确认对话框入口已实现，
默认取消、是/否才生成决定，返回后再次复核授权与期限。远端字段转义控制/
双向字符，过长内容拒绝展示而非截断。应用模态互斥、退出控制和调度已接入。

Surface 后台 Job 已串联一次事件 POST 与设备 HTTPS 流收敛，收敛阶段最多
25 秒（不含提交阶段），可取消等待，成功后独立刷新且不重放动作 POST。
ACK 与场景/收敛错误分开保留，收割时另验授权；确认请求立即交还调用方。
授权窗口已有事件结果交付接口和精确 pending event 路由：恢复 owner 不代表
呈现成功，应用 Scene 前复核授权/ACK/版本，替换后再检查撤权，返回独立收据。
主循环和退出泵已在生命周期结算前交付事件，并将收据存入有界应用缓存。
未保存动作总数最多 32，缓存不占网络槽位，不影响附件停用；单独的旧附件日志
拒绝遗漏动作证据。正常退出已在后台保存独立动作恢复提示，不含 payload、
场景、凭据或远端错误文本，读取不授权、不重放。两类文件非原子事务，后续
写入失败会报错并保留已有文件；尚无运行中崩溃日志。result commit 已校验并
沿 Tracker/快照/收据链路保留，只在恢复提示中保存结果版本；输出与本地图的
实际连接及确认交互仍待接入，
原生点击入口仍默认禁用。

确认提示已有身份/风险/到期校验与一次设备过滤流 GET 接口，保留最新 ACK，
整批后续失败可否决提示。后台事件 Job 已在等待确认后读取一次提示，保留最新
ACK 和独立读取结果并传入窗口收据；展示前有当前授权/状态/到期复核入口。
原生确认对话框已接入显式菜单请求的应用交互调度。底层已有显式批准/拒绝的单次决定
POST 接口，网络尝试后不重放；HTTP 成功仅表示决定已接收。另有已确认决定
后的原动作 GET 收敛接口，不重发点击事件，当前应用不会自动调用批准路径。
决定后台 Job 已封装上述事务，复用共享容量并完整返还 owner/event/ACK/决定，
提交、收敛和交付授权分开保存；窗口提供当前授权下的决定 owner 移交入口。
应用任务集合已接入决定等待、取消、收割和退出泵，拒收保留完整结果；决定
元数据进入正常退出的恢复 bundle，分开记录选择、尝试和接收状态，不保存
确认 payload，不把记录解释为可自动重发的批准。

决定结果已转换为统一事件 Outcome，复用主循环/退出泵的窗口交付、有界收据
和恢复记录，不再保留独立结果队列。决定提交结果与最后 ACK 分开保留，
窗口呈现另验决定已接收、事件/ACK 身份和当前版本；恢复 owner 不放宽原有
授权与场景检查。拒收完整返还证据，已结算收据仍保留决定元数据。

result commit 的 outputs 已有共享类型解析：value 借用原文，资源描述符校验
哈希/类型/尺寸，流描述符校验 ID/序号，端口名解码去重；读取接口仍不下载、
订阅、执行或更新本地图，不能把描述符有效误当作资源访问授权。

原生 Surface 已有单事件场景收敛 Tracker：成功 ACK 后独立刷新最新快照，
整批校验附件/代际/终态后才产生一次性 Scene，不以旧快照冒充动作结果。
网络调度已接入后台 Job；窗口交付已有本地 HWND 回归并接入应用结算，
正常退出动作提示已持久化，result commit 已有界接收并传递；恢复交互、运行中
崩溃保障和输出应用仍待实现。本地 TLS 不代表真实 Loom 联调。

原生 Surface 已有设备消息流的单次 HTTPS 读取和有界批次解析，不访问管理员
完整实例、不启动常驻轮询。批次保留 reset 与游标，具体事件身份及场景收敛
仍由后续 owner 校验，读取成功不等于动作或场景更新成功。

Surface 点击候选已绑定窗口私有场景身份，领取后遇到场景替换/尺寸变化也会
失效；消费时同一 Host 借用重新命中并准备事件，授权窗口只接受该候选路径。
入口仍默认关闭，最终发送调度与结果收敛尚未接通。

Surface HWND 已有默认关闭的左键候选入口，真实消息处理按下/抬起同节点、
拖动阈值与捕获取消，单槽待消费且不发网络。应用尚未启用，消费时授权与
场景检查、完整交互规则和发送结果收敛仍需接入。

Surface 已连接客户区像素命中、当前场景 click 冒泡与有界事件批次准备，
整批输出不超过 64 KiB，失败不返回部分动作。授权窗口准备前后复核状态，
仍不发送事件；鼠标消息与后续结果收敛尚未接线。

Surface 窗口已有事件 owner 独占移交接口：显示/租约/场景版本检查通过后，
事件与生命周期事务不能并发持有同一 owner；归还严格匹配 eventId 和绑定。
归还 owner 不等于交付 ACK，也不重新显示关闭窗口，输入调度仍待接线。

Surface 事件 Job 已纳入应用 Sessions 的共享容量、等待集合、正常收割和退出
取消链；未消费结果仍占容量并保留执行证据。旧附件恢复格式不能表示动作，
存在待处理事件时拒绝导出该格式；动作恢复持久化与窗口输入接纳仍待完成。

Surface 点击提交已有有界后台 Job，独占 owner/事件并提供等待句柄；完成
收割返还执行证据和独立的当前授权结果，迟到撤权不丢弃 ACK。应用消息循环
的任务登记/收割和原生输入接线尚未接入，不以该 Job 宣称交互已完成。

原生 Surface 已有单次 HTTPS click 提交 API，复用设备认证和取消机制，发送
前检查 active/绑定/场景版本，传输层限制 ACK 大小。已尝试事件禁止直接重放；
原生输入到后台任务的接线和错误后的查询收敛仍待完成。

原生 Surface ACK 已有大小受限、身份匹配的类型化解析，完整保留接收、确认、
执行、取消和 unknown 状态。ACK 成功与场景收敛分开；HTTP/流消息 owner 仍需
校验来源与顺序，解析器不代替授权或当前附件生命周期验证。

原生 click 动作已有有界 loom.surface.v1 编码，事件身份使用安全随机 ID，
保留绑定身份与当前场景版本，输出总量限制为 64 KiB。编码不授予动作执行
权限，也不发送 HTTP；授权复核、ACK 及状态收敛仍需后续接入。

原生 Surface 挂载交付以 AdmittedSnapshot 保留 Scene 与同一响应 revision，
授权窗口单独保存场景版本，不受生命周期 ACK 推进影响。后续输入事件的
baseRevision 必须来自该已呈现快照，而不是 generation/lifecycleRevision。

Surface Presentation 的 click_route 已连接当前命中和场景声明，固定深度
借用路径按容器冒泡、按钮阻断解析动作，不复制 payload 或发送事件。
输入权限、选择手势与协议发送仍须由输入 owner 接入，不扩大输入能力报告。

Surface 原生窗口宿主命中 API 已接当前 DPI/客户区尺寸同步，重绘前查询
也能更新布局且不创建渲染目标。忙碌与关闭拒绝查询，鼠标/动作接线仍待完成。

Surface Presentation 已包含与绘制一致的 CPU 命中快照，检查叠放、可见性、
矩形/圆角祖先裁剪；尚未连接原生输入消息或远端动作，不扩大输入能力声明。

Surface 布尔 visible=false 已从准备和绘制链完整排除子树，隐藏根产生空
快照；原始场景仍接受完整输入安全校验。opacity=0 继续保留布局占位。

Surface Progress 已使用原生画布绘制确定进度，数值有界归一，复用画刷且
不创建 HWND/定时器。控件颜色映射 Neuro 规范默认色；动态主题与 UIA 待接入。

原生 Divider 通过固定有效属性视图接入满宽 1px 分隔线测量与绘制，保留
原始场景字段的安全校验；当前使用暗色回退色，动态主题变量尚未接入。

原生声明式 Surface 已增加 spacer 叶子节点：默认伸缩、零固有内容、复用
背景/边框绘制，并加入真实宿主报告；未扩大输入与滚动能力声明。

Surface 容器类默认 width:100% 已统一进入布局取值器，区分缺失值与显式
auto，供根盒、正常流、Stack、定位和测量共用；其他控件默认样式仍待接入。

Surface 根 absolute 双端 auto 尺寸已按宿主剩余空间求解，再应用尺寸约束；
区分默认容器满宽和显式 auto。单端收缩适配与循环百分比仍待完善。

Surface 根盒默认高度已接入内容测量及文字断行反馈，宿主视口保持独立。
默认根宽度仍为块级宿主宽度；absolute auto 与循环百分比仍待对齐。

Surface 根节点 relative/absolute 锚点已接入布局树，位置使用最终约束尺寸
与宿主包含块，子树随根平移但宿主裁剪不动；auto 尺寸定位仍待完善。

Surface 根盒约束与宿主 viewport 已分离。根显式尺寸/min/max/边框下限由
surface_root_box 求解，而祖先根裁剪保持宿主范围；默认内容自适应仍待对齐。

Surface 布局快照现在记录百分比 padding 的包含块宽度，区分正常流内容盒
与绝对定位 padding box；文字最终断行使用同一基准，避免自身 width 造成
错误内缩。固有尺寸中的循环百分比仍需进一步对齐。

Surface 子节点的边框/padding 已成为行列、Stack 与绝对定位的不可压缩
尺寸下限，空间不足时允许溢出。根宿主和百分比 padding 循环仍需专项对齐。

Surface 单轴 overflow 通过 surface_overflow 归一计算值，再共同驱动矩形
裁剪和圆角 mask；不再保留只裁声明轴的临时行为。滚动交互仍待单独接入。

Surface overflow 的矩形裁剪与圆角 mask 已对齐 padding box；Presentation
几何和文字裁剪使用同一计算字号下的边框宽度。节点自身的背景/边框仍按
原外轮廓绘制；矩形快照不替代圆角命中测试或完整 overflow 规范实现。

Surface 边框绘制使用 surface_border 的 alternate 内外轮廓几何，保留透明
中心及单次 alpha 覆盖。PaintFrame 在背景后、文字前发出边框指令；几何
随场景拥有并按 Direct2D 工厂缓存。完整 overflow 内边界仍待接入。

Surface 的 border-box 尺寸由 surface_box 统一解析，容器排版和文字测量
共用边框/padding 内缩；绝对定位锚定 padding box，文字反馈恢复外盒高度
和基线。目前只完成尺寸接线，边框绘制及 overflow 内边界仍待接通。

双轴裁剪的圆角 Surface 容器已增加子树几何 mask，和 opacity 层正确嵌套，
不影响普通可见子树或后续兄弟。几何按 Direct2D 工厂惰性缓存，换工厂时
整批重建。圆角 cqw/cqh 已统一回宿主查询容器，不再错误使用直接父尺寸。
单轴完整 CSS 归一、多值逐角和边框仍未完成，实际合成资源成本尚待验收。

Surface 背景支持单值圆角：提交时解析自身百分比及计算后字号上下文，
超大半径等比缩减，绘制完整 rounded rectangle 后应用祖先裁剪，不改变
被截断圆弧的几何。普通子内容不被背景圆角误裁；边框、多值逐角和圆角
overflow 子树裁剪仍待实现，跨 DPI/可见 UI 尚未验收。

Surface 组 opacity 已通过 Direct2D 子树层合成，避免重叠图元重复累积 alpha。
零透明度子树不绘制、完全不透明节点不建层；提交时计算子树可见范围，
保留 overflow:visible，异常路径恢复层栈。真实像素覆盖嵌套与后续绘制恢复，
但半透明组的实际 GPU/离屏资源成本和跨 DPI 尚未验收。

Surface HWND 呈现已接入文字继承颜色和节点纯色背景：提交布局时预计算
有序指令，绘制时复用文字布局/画刷并按树顺序交织背景与文字，保留裁剪。
真实离屏像素已验证颜色、alpha 和 Stack 遮挡；主题变量、边框/圆角及组
opacity 等仍待实现，不能视为完整 CSS 支持或跨 DPI 可见验收。

恢复候选已有显式确认菜单：默认否，确认前后校验候选身份；启动时消费
候选并重验授权/设备/节点和句柄容量。后台 remount 复用原有挂载准入与
active 显示链，不自动创建或重试。应用代码链已接通，真实服务与可见 UI
全链验收尚未执行；历史管理及崩溃前日志仍待实现。

恢复提示已提供按需读取入口：原生文件选择只排队路径，后台读取后列出
最多 32 个查询项。当前服务、设备和有效授权匹配才允许查询，完成后再核对
身份；只保留待确认候选，不自动 remount，也不将 None 当作未创建证明。
本路径不做启动全目录扫描，可见 UI 与真实服务查询尚未验收。

Surface 正常退出后的最小恢复提示已接入后台落盘，位置在原生 profile 的
`surface-recovery`，至多 128 份、每份 64 KiB/32 条，不覆盖旧记录。提示不含
token/场景/参数，不可直接转成授权或操作 owner。启动恢复交互及崩溃前日志
尚未实现；失败/半写文件保留并由严格解码拒绝，不把写入意图当持久化成功。

未建窗的已确认 Surface 附件也纳入退出停用：失败记录独立保留生命周期
owner，原始挂载证据不变。退出宽限等待迟到挂载，但不为清理创建窗口。
重复、unknown 或身份不一致记录不生成停用 owner；请求重新验证设备授权。

退出前已加入有界 Surface 停用阶段：关闭窗口后保留授权供一次 inactive
事务，等待最长 2 秒的绝对截止并继续处理消息；之后无条件撤权和回收。
unknown 不重放，超时仍保留 owner 并报告未确认。尚未在线验收或持久化。

应用退出回收后台线程后另做 Surface 附件结算：可匹配结果回交原 owner，
未确认状态导致退出错误，而非静默视为清理成功。此阶段不发请求、不显示、
不重放 unknown；证据仅在进程内保留；现有窗口的有界退出停用已接入，恢复持久化仍未实现。

本地 Surface 关闭按 inactive 上报，不等于删除工作区节点或远端附件；已知
状态和有效授权才调度，unknown 不重放。Live 与 Surface 在启动/接纳前共用
最坏 63 句柄预算检查；应用退出网络收尾及恢复持久化尚未完成。

Loom 菜单的“打开 Art（原生预览）”将用户输入与当前窗口 UnitId 排队，
应用验证授权和容量后启动临时挂载。宿主报告只声明静态容器/文字节点；
其他运行时及控件不被冒充支持。失败处置、关闭上报与在线验收仍待完成。

应用 Surface 结算路径已将 pending 窗口 Owner 调度到 Sessions 的 active
后台事务，完成后验证回交并复核授权；成功时显示一次，失败记录但不自动
重发。未匹配结果原位保留、有界扫描，不阻挡其他完成项。在线/可见验收未做。

Activation 在窗口与后台之间移交唯一生命周期 Owner，pending 时只保留
确认绑定用于身份检查；回交须满足同附件同 generation 和允许的版本变化。
窗口关闭后仍保留回交位置，迟到确认不复活窗口；应用作业调度尚未连接。

Surface Binding 同时保存确认生命周期与版本，显示必须经过当前租约检查
及 Owner::confirmed_active。unknown、mounted 或 disposed 都不构成显示许可；
后台 active 事务回交窗口的链路尚未接通。

应用结算点已把挂载完成结果交给 Surface 准入，复核选中服务并获取当前
租约后创建隐藏窗口。失败/未知/重复记录与窗口共享容量，接收方满载时
原样退回 Sessions。active 确认后显示及用户发起入口仍待连接。

Application 已持有有界 Surface Windows 集合，等待前结算授权并登记存活
窗口的撤权句柄和最近截止，退出时关闭本地窗口。终态保留附件证据但退出
等待集合；挂载完成结果已接纳到集合，正式生命周期上报仍未接通。

应用等待函数 next_until 可用绝对单调截止触发一次 WM_NULL 结算，保留原
无限等待语义且不新增轮询；截止优先于排队输入。Surface 集合将墙钟过期
时间转换为本次等待截止，每次应用结算仍重新检查授权。

`desktop/surface_authorized_window` 将挂载准入、隐藏窗口、租约撤权通知与附件
Owner 组合；refresh 在授权失效时关闭本地窗口并保留服务端生命周期证据。
显示前再次检查，关闭后不复活；应用仍须登记等待句柄及过期截止并调用结算。

租约通过独立的惰性手动重置事件通知本地撤销，不复用网络唤醒槽，不新增
线程；事件终态保持，注册与撤销共锁。截止时间单独提供，调用方仍须安排
过期等待与 validate。该通知已由 AuthorizedWindow 持有，应用等待集合尚未接通。

`Outcome::scene_for` 是挂载结果到本地场景的交付准入：重新检查租约有效性、
绑定与快照身份、已确认状态及运行时。失败保留 Outcome，不自动重发请求；
该时点检查尚未连接应用建窗，也不能替代持续撤权和附件生命周期管理。

`desktop/surface_window` 拥有 Surface HWND，连接绘制、重绘失效和关闭消息，
同步重入关闭不会复活已取出的 Host。隐藏创建后由调用方显式显示，应用
授权绑定结果到该窗口的入口仍待接通；不绕过 Surface 生命周期协议。

`desktop/surface_host` 将声明式文字 Presentation 绘制到同线程 HWND，
客户区/DPI 不变时复用布局；变化时重排，绘制目标出错后释放且不忙循环。
隐藏窗口运行测试已覆盖绘制和尺寸变化；消息处理由 surface_window 接通，
应用入口尚未接通。

`surface_presentation` 持有 Scene、独立 Geometry 与文字快照，准备成功后一次替换；失败
保留旧内容。TextFrame 保存节点 ID 而非借用节点，不需要自引用窗口结构，
重绘可复用 DirectWrite 布局。该状态尚未接入生产窗口和完整控件运行链。

Geometry 保存同轮 Tree 的全部节点位置、尺寸、父索引和可见矩形，不借用
原节点；未包含滚动变换，也不能作为交互授权或最终事件命中结果。

`surface_clip` 从 Tree 累计根宿主和祖先容器的逐轴裁剪，文字快照保存与
自身矩形的交集。不可见文字跳过绘制但保留布局和高度反馈，避免可见性
改变几何；滚动状态、变换和实际窗口集成仍待完成。

`surface_reflow` 在 NativeMeasure 内执行有界高度/基线反馈，稳定后交付
一致的 PreparedScene，最多 8 轮，失败不替换调用方已有有效快照。该流程
保留初始宽度基准，不宣称通用 CSS 收敛和完整渲染实现。

`surface_text_frame` 把 Tree 最终文字矩形转换为持有原生 Layout 的绘制
快照，按实际宽度重排并记录溢出；包含祖先交集的裁剪成对恢复。父尺寸
反馈已接通 prepare；滚动及窗口绘制周期仍待宿主集成。

Tree 通过 Measure::font_size 把原生样式表中的计算字号传给几何模块，
父内边距和子尺寸/偏移分别使用对应节点上下文。纯几何入口保留显式
单字体用法，不能在原生树布局中忽略继承字号。

`surface_native_measure` 拥有单场景字体样式、DirectWrite 工厂和容器固有
尺寸缓存，并实现 Tree 的测量接口。它仍是初始测量链，不代表分配后文字
重排和尺寸收敛已经完成；上下文变化应重建，未实现节点显式拒绝。

`surface_text_styles` 为单场景预计算继承文字样式并绑定具体节点引用，
DirectWrite 测量直接使用计算值，不重复解析相对单位。新 Scene 重建样式
表，不按同名 ID 复用旧场景缓存；完整测量和绘制 owner 仍待接线。

`surface_layout_tree` 通过 Measure 合同和容器布局器构建有界场景坐标树，
失败不返回部分几何；它不负责测量缓存回滚、字体继承、文字重排或窗口
绘制，原生测量/渲染 owner 仍待接线。

`surface_stack` 处理给定内容区域内的重叠子节点，复用交叉轴与定位逻辑，
保持场景顺序；不冒充完整 grid 轨道求解器，复杂分布仍显式拒绝。

容器定位由 `surface_position` 处理：relative 只偏移结果，absolute 从
普通流中排除并单独求解，但仍保持场景输出顺序。祖先包含块搜索和自动
静态位置尚未实现，不能当作完整浏览器定位语义。

`surface_container` 将真实场景行列容器及身份匹配的测量结果接到两轴
分配器，返回子节点矩形。父内容百分比和宿主查询容器单位分别处理，
不支持的定位显式报错；递归树布局和绘制 owner 仍待接线。

`surface_cross_axis` 接收测量尺寸及基线并计算交叉轴对齐，输出两侧溢出
供上层滚动/裁剪使用，不在几何层静默裁剪。它与主轴计算一样尚需完整
节点树布局 owner 接线。

Surface text 支持显式行高：从真实字体行度量调整基线后设置 DirectWrite
统一行距，再读取最终尺寸；baseline 可供容器对齐。跨字体视觉一致性和
完整树布局仍需验收，不以原生测量测试替代窗口视觉测试。

`surface_text_node` 将 Scene 中的 text 节点及显式父样式转换到 DirectWrite
布局，复用长度解析，避免 em 使用过期字号上下文。它只负责 text 适配，
不构成完整节点树布局或控件交互。

`surface_text` 提供 DirectWrite 文字测量和同布局 Direct2D 绘制入口，
工厂/布局由渲染 owner 管理，输入有界且无全局缓存。测量测试不等于
Surface 窗口的视觉验收，节点树布局和完整样式/交互仍未接通。

`surface_flow` 是行/列主轴空间分配器：接收已解析和测量的尺寸，输出
位置、尺寸及溢出量，不持有窗口或像素资源。节点树布局、交叉轴、文本
测量及绘制尚未接通，不能将此模块视为完整 flex/grid renderer。

声明式长度通过 `surface_length` 解析为原生数值，支持明确的单位和受限
min/calc 结构。布局 owner 提供视口/容器/字体/fraction 上下文，auto 不由
解析器猜测；该模块尚不构成完整布局或渲染引擎。

`surface_scene` 是声明式场景的有界纯数据模型，挂载结果经
`MountedSurface::declarative_scene` 进入该模型。18 种节点类型只表示 wire
合同覆盖，尚不代表原生布局/绘制/输入能力；资源和属性使用须由对应模块
验证，不执行远端脚本或任意资源路径。

恢复查询当前由 `surface_recovery_runtime` 后台执行，应用通过
`desktop/composition_relay_recovery` 管理任务及有界结果缓存，复用统一等待
和退出回收；查询结果不改变原创建 unknown，不自动 remount。候选确认 UI、
跨进程恢复和 Surface renderer 仍未完成。

候选 remount 当前已通过 `surface_attach_runtime::Request::recover` 接入
已有挂载后台 Job 与 Sessions 集合；创建/恢复显式分流，恢复失败不改为
创建。查询后台任务、候选确认 UI 与跨进程恢复仍未接通。

首次挂载结果不明时，`surface_recovery_http` 使用设备认证 stream 查询
当前绑定候选，不重发创建；空结果不能证明未创建。显式候选 remount 再从
认证响应取得实际生命周期版本，校验身份/代次/Art/runtime。查询及 remount
协议已实现，但恢复后台任务、UI 接线和跨进程状态保存仍未完成。

原生分支当前 Surface 首次挂载由 `surface_attach_runtime` 后台执行，
`desktop/composition_relay_attach` 持有任务及待领取结果，与目录、观看和
生命周期事务共享四项集合容量，复用应用完成句柄等待及退出回收。取消只
抑制场景交付，不抹掉已确认绑定或 unknown 创建上下文；不自动重发创建。
这些是进程内协议/任务机制，尚不代表原生 Surface renderer 和 UI 入口完成。
最新迁移状态与历史检查点见 `docs/development/WINDOWS_NATIVE_MIGRATION.md`。

Live 控制 API 已提供认证 catalog 列表，输出稳定排序的身份/revision/成员
标记并完整校验有界响应。Runtime 的 selected 身份约束与 expected 续签
约束分离：目录选择允许首次显式加入非成员，但若实际 discovery 来源或
epoch 已改变则在 POST viewers 前拒绝；attach revision 不能低于本次发现。
续签仍要求现有成员。目录不是加入
授权，也不是观察/触发业务快照。后台目录任务和选择 UI 尚未接入。

Application 已持有有界远程会话集合，将终态轮询及 Ready/帧/完成句柄纳入
既有 Live 消息等待流程。四个远程会话包含停止未回收项，每项最多两个句柄；
与本地 Live、可见性和授权任务合计上界为 63。退出先请求远程停止，再继续
泵消息直到后台回收，不依赖 UI 线程阻塞 Drop。正常关闭不弹错误，设备撤权
和窗口错误保留为终态提示。菜单启动和具体业务 consumer 仍未接入。

远程 Session 已配对控制 Runtime 与 RelayWindow：后台 Ready 后在 UI 线程
创建隐藏窗口；显式 show 意图可提前保存。统一等待集包含 Ready、线程完成
和窗口帧事件；窗口关闭触发请求停止，线程完成后才返回一次 Outcome。许可
失效导致的窗口关闭不抢先取消，保留媒体 Closed/DeviceRevoked；构造期间
撤权不误报窗口错误。当前 owner 已联调验证，尚未接入 Application/菜单。

远程 RelayWindow 已将呈现 Host 接入独立 HWND 的 paint/resize/show/close
生命周期，默认隐藏并设置系统捕获排除。它向应用暴露帧事件句柄，由应用
等待后 refresh，不启动计时器。隐藏/最小化不上传或绘制新帧，仍消费最新帧
和撤权；恢复后绘制待上传帧。已存在 GPU 资源暂保留，隐藏资源释放策略未完。
当前窗口组件已验证，尚未接入 Application 会话集合、等待集或菜单入口。

远程像素已有独立原生呈现 Host：复用 Composition 的窗口私有合成链，在
UI 线程按事件领取最新 PBGRA 帧。同尺寸通过 CopyFromMemory 复用位图，
改变尺寸重建位图；重复 paint 不重新上传。上传完成即释放 CPU 帧，按比例
缩放并保留透明留边。固定许可失效后丢弃待上传像素、位图和合成链，不自动
admit 新许可。当前只完成呈现组件及隐藏 HWND 验证，实际观看窗口尚未接入。

ViewerSession 已提供 Windows 帧/许可变化事件。接受新帧、暂停或关闭都会
唤醒等待者；单个 UI consumer 用 poll_frame 在同一状态锁内复位事件并取
最新帧，避免清除通知与并发发布之间丢失唤醒。突发帧仍由有界缓存合并，无
新增轮询线程或定时器。暂停/关闭通知要求窗口释放旧像素/GPU 显示资源；
当前已验证后台到等待句柄的链路，尚未实际接入远程观看 HWND。

Live 控制 Runtime 的 start_viewer/start_viewer_configured 已连接媒体 Runtime。
首批控制消费成功后，授权租约使用同一 Client 的固定 origin、TLS 和解析地址
生成 Device WSS 请求，后台启动网络与 codec worker；Ready 不携带认证头。
控制线程退出时回收媒体 Runtime，UI 不 join 网络/codec。媒体线程现先保存
终态，再撤销租约以唤醒在途控制 HTTP，不再等待轮询超时；控制线程保留媒体
关闭/设备撤权等具体原因。显式取消先发生时仍按 Stopped 处理。
目前初始媒体游标为已验证 epoch 的零帧，不代表 resume/自动重连已完成。

Live 控制 Runtime 现支持配置快照启动：CA 加载、DNS 解析和后续 HTTPS 都在
同一后台线程，使用租约自己的取消状态。客户端准备前后复核凭据 origin 和
过期；Live 不创建或读取设备身份 Store，也不隐式重新授权。桌面只需捕获
Config 并交出既有租约，不必在 UI 线程构造网络 Client。

原生 Live 控制后台 Runtime 已串联 discovery/attach/bootstrap、完整首批
消费与持续轮询，持有授权租约并在退出/异常时撤销。Ready 只发布一次，携带
已验证媒体许可和序列基线；独立 Win32 就绪事件与线程完成句柄供 UI 等待。
停止后迟到 Ready 会被丢弃，空批次最短退让可取消以防服务端导致忙循环。
当前仍未接入桌面 Application 等待集与媒体 Runtime，业务 consumer 也仍由
调用方提供；这不是已可操作的完整远程观看入口。

原生授权缓存现以 Grant 持有 Session，并可签出独立后台租约。每份租约拥有
自己的网络取消状态和单个可选媒体绑定；缓存替换/移除会关闭媒体并唤醒在途
请求，失败配对则保留原授权。绑定和撤销共锁，不允许撤销后重新激活媒体。
Session 通过 Arc 共享而不复制 token；后台退出释放最后引用后擦除凭据。
这补齐了撤销机制，但尚未将桌面远程观看任务接到租约；到期仍由请求前后
检查，不代表已实现独立到期定时器或完整重连策略。

Windows-native 控制 owner 已将事件批次与 ViewerSession 媒体许可连接：首次
完整重放和业务消费后才激活；后续 revision 更新不重建帧缓存。成员缺失或
closed 可见性使本 owner 不可逆关闭；异常、消费失败和 HTTP 错误同样停止
本地旧观看，但不冒充服务端已关闭。404 仅在认证成员快照确认后报告远端关闭。
这仍是后台可调用的组件，不是已接入桌面的完整 Viewer：授权缓存撤销联动、
后台任务/媒体线程编排、非状态业务 consumer 和会话选择 UI 仍待完成。

Windows-native Viewer 加入准备已串联目标 discovery、成员专属 requesterControl
恢复、checked 下一序列、attach 与加入事件游标。续签既在 preview 检查成员，
也要求服务端 requireExistingMembership；缺成员字段不能被当作首次加入。
resume 已实现真实 HTTP 请求，成功且身份/成员匹配后才返回新序列。Prepared
不激活媒体；控制 owner 在完整批次应用后激活，桌面任务联动仍需实现。

Windows-native Live 控制 HTTP 已实现 member snapshot、Viewer attach、加入
事件游标、持续 events 和成员关闭确认；复用设备 HTTPS 交换，凭据来自已
签发 Session，每请求使用独立随机 nonce。401 在读取错误体之前分类，404
不直接当作关闭；控制批次在业务应用前不会自动提交游标。持续流 reset 拒绝，
bootstrap 只容忍本次加入以前的历史淘汰。discovery、actor sequence 基线与
resume 已接入控制准备；控制 owner 已实现许可边界，桌面运行时联动仍未完成。

Windows `BridgeStream` 以非阻塞 socket 加网络/取消事件实现后台同步 I/O；
取消不再依赖复制 socket 的 shutdown。复用该流的 HTTPS 和媒体 TLS 握手均
继承同一绝对期限与事件取消机制，不新增轮询线程。已复现的三秒取消失败路径
通过加强测试和连续重复回归验证，跨环境性能验收仍另行进行。

Windows-native 分支的桌面菜单已接入明确 origin 选择、配对、批准等待、续签
和本地取消。共享 View 不存 token；Application 独占授权 Owner，先消费撤销
再回收结果，完成句柄与 UI 消息共同等待。关闭入口后取消并泵消息至任务退出。
连接选择目前不落盘，尚未联动远端 Live 控制/媒体许可或能力目录，不能视为
远端功能已全部迁移。Win32 输入控件尚待统一主题与手动交互验收。

`device_authorization_config` 固定显式 origin、原生隔离 profile 和 TLS 环境快照，
由 Configured 授权任务在后台读取信任配置、解析地址并构造客户端，再执行既有
配对/批准/续签流程。UI 不必同步执行 DNS 或 CA 文件读取。

`loom_address_resolver` 使用 Windows 原生异步解析和完成/取消事件；没有应用
自建的 DNS 等待线程。五秒等待后取消，仍等待系统完成才释放异步结构，因此
不是任何环境下都能硬限时返回。后台配置准备已由桌面授权命令调用。

`device_authorization_owner` 管理按 origin 隔离的授权缓存与有界后台任务，撤销
会永久丢弃旧 pending 的结果；配对成功清除同 origin 旧缓存，失败不清除。
这不自动撤销已经建立的媒体许可；此联动仍待实现。

`device_authorization_runtime` 为设备配对、首次批准和续签提供独立后台 Task，
最多四个未回收实例；线程拥有临时身份与网络资源。Windows 完成句柄与 try_join
允许桌面 owner 非阻塞回收，取消阻止旧结果发布。实际桌面消息循环已接线，
Drop 的阻塞 join 只是兜底，不应在 UI 活跃任务路径中使用。

`device_approval` 将明确待批准响应转为有总期限的串行重试，普通续签不走该
路径。`device_request_cancel` 通过 socket interrupt 和 Condvar 取消网络操作
与等待间隔，不新增定时线程。应用管理任务数量、UI 完成句柄和旧授权撤销。

`device_pairing` 通过 HTTPS 提交公钥注册请求，只接受公钥匹配且无歧义的设备
关联。显式默认绑定路径通过 Store 提交；网络成功但本地快照失效时报告部分
失败，不覆盖较新文件。注册不是批准，不会签发 token；应用授权撤销/批准
轮询与实际目录接线尚未完成。

`device_session_client` 将单次设备会话协议接入实际 HTTPS，使用显式 origin
信任根和已解析地址、十秒请求总期限、握手前交付 interrupt。固定 ureq 的 HTTP
解析运行在调用线程，通过 loom_http_transport 消费已验证的 TLS 流一次；不
启用代理、重定向或连接池。应用 owner/地址解析/批准轮询仍未接入。

`device_session_attempt` 将 challenge proof 与签发响应连接为单次消费式协议，
仅 challenge 的精确未批准响应可成为 PendingApproval。Session 限定原 HTTPS
origin、设备与有效期；不泄漏正文到错误日志。真实 HTTP 客户端已提供 TLS 验证、
读取限额与中断入口，生命周期和批准轮询仍由后续应用 owner 提供。

`device_identity_store` 在显式目录中区分只读加载、初次创建/legacy 迁移和配对
绑定保存；保护文档由 device_identity_file 同目录刷盘提交。新实现实例通过 OS
独占 lock 协调，快照 owner/hash 防止可检测的过期覆盖；非合作外部替换仍存在
核对后竞争窗口。应用数据目录与真实授权网络流程尚未接入。

原生设备身份由 device_identity 与 device_key_protection 解析并保护，兼容主干
legacy seed/DPAPI1 格式；device_session_proof 生成固定设备、随机 nonce 的 Ed25519
challenge 签名。当前没有自动访问用户身份文件或发送授权 HTTP 请求，不把签名
能力当作配对批准；外层仍须负责可信 origin、持久化、请求期限与响应校验。

`live_relay_viewer_runtime` 将连接与 codec worker 组合为固定授权代次的后台任务，
上限八个网络线程。停止先关闭发布 gate，再中断已建立 TCP（包括 TLS/upgrade
阶段），线程退出时回收 codec。应用 UI 使用完成句柄与 try_join；真实授权与
bootstrap 仍须由外层 owner 完成，Runtime 不将传输成功解释成业务授权成功。

`loom_tls_trust` 将操作员 CA 限定到单个 HTTPS origin，并与调用方的默认根合并；
PEM/文件读取有界且配置错误不降级。媒体目标通过 local_viewer_with_trust 获取
自身 origin 的快照，不从发现清单扩展信任，也不修改系统根库。Trust::load_default
使用固定 webpki-roots 1.0.9 的编译期 Mozilla 根，不是 Windows 平台 verifier；完整
应用 owner 接线仍待完成。

HTTP 媒体发现复用 `loom_discovery` 的清单来源与失败优先级，但不依赖 hook bridge
是否开启。`loom_media_target` 将本机 none/bearer 配置转换为固定本机设备的媒体
请求；远端目标仍要求独立 Device 授权。localhost 与数值 loopback 均受候选地址
检查保护，HTTP 清单不自动提供 TLS 信任或远端设备权限。

`live_relay_media_endpoint` 固定 origin 与显式 TLS 信任根，构造媒体请求和随机
Device nonce；`live_relay_media_transport` 负责 TCP/TLS/HTTP upgrade、实际子协议
验证及媒体服务的截止时间/取消封装。凭据获取、可信发现与 CA 加载仍由后续 owner
接线，不把可构造 Device 头当作已获得远端授权。当前真实 TLS 回环是协议机制验证。

`live_relay_media_liveness` 按连接持有一个在途 Ping 与最近 RTT；socket service
读取前驱动两秒探测，精确匹配 Pong，十秒过期清除测量。无独立定时线程，实际
节奏依赖有限读超时的服务循环；此机制不替代认证、撤权或重连策略。

`live_relay_media_socket` 对已准入连接执行有界单步服务，接管 WebSocket Binary
缓冲交给 worker，消费控制意图并区分普通 Close 与精确 Device 撤销。回环 WS
路径已验证；真实连接/TLS/凭据握手、重连和 HWND 仍待接入。消息接管租约不涵盖
WebSocket 库读取期间的临时分配，读取前的消息大小限制不可省略。

`live_relay_media_worker` 在线程内部创建 Pipeline/COM，持有两项输入和四项回复
队列；停止撤销原代次并唤醒等待，线程结束前回收 codec。UI 可借用完成句柄并
try_join，不能把活跃 worker 的阻塞 Drop 当作异步关闭。真实 socket 和 HWND
接线尚未完成。

`live_relay_media_pipeline` 在线程绑定的 MTA 中组合固定许可、视频/静态解码和
会话缓存提交。失效后 check 释放 codec 并永久退役，网络循环需主动接入该检查；
目前尚未创建真实 socket worker，也未将缓存连接到 HWND 呈现。

`live_relay_viewer_session` 以 state→frames 锁序绑定本地发布状态与像素缓存；
控制 owner 在认证/重放后激活并为 worker 发放代次许可。暂停和关闭取消旧许可，
提交时再核对身份/epoch/代次。此 gate 尚未接入真实认证和 worker，不代表远端
撤权已经执行到 codec 或 HWND；已移交的 UI 资源须由 UI owner 另行撤下。

`live_relay_viewer_identity` 与 `live_relay_viewer_bootstrap` 校验已认证 HTTP
快照和本次加入的连续事件后缀；不持有凭据，也不自行授予媒体权限。HTTP/Device
认证及事件应用接线仍待完成，不能从解析成功直接创建已授权的 viewer。

控制 envelope 由 `live_relay_control_events` 共用解析，ControlCursor 两阶段提交
整批序列；业务 owner 须在同一事务中应用结果并提交 cursor。权威关闭由
`live_relay_viewer_terminal` 校验 member snapshot，网络错误/404 不是关闭事实。

原生迁移分支的最新阶段以 [迁移台账](../development/WINDOWS_NATIVE_MIGRATION.md)
和 [功能对照](../development/WINDOWS_NATIVE_PARITY.md) 为准。N09 的 `live_mf`
owner 已实现系统 H264 decoder 生命周期、类型协商及 AU 提交/输出处理；格式与
可见区域分别由 `live_mf_format`、`live_mf_layout` 管理。`live_mf_decode` 持有参考
时间轴，`live_mf_samples` 限定 COM 对象生命周期，`live_mf_pixels` 输出预算内 Pixels。
尚未接入网络会话和 viewer；具体运行证据见迁移台账，不等同于产品视频链完成。

`live_relay_video_receiver` 已组合参考链与系统 codec，输出关键帧请求/降级意图
或带原帧身份的 Pixels；不自行授权或发送。连接 owner 必须提供真实授权 epoch，
并在媒体提交和控制发送前同锁复核终态及权限，不能以 wire epoch 代替授权状态。

H264 Receiver 和静态 JPEG/raw 入口统一交付 `PixelFrame`；`ViewerPixels` 与原
`ViewerFrames` 共用 FrameStore 的三帧缓存及关闭逻辑，保留 Pixels 存储和租约，
不重新封装网络帧。授权和 HWND 呈现尚未接线，不能将缓存提交当作已显示。

Live HWND 系统菜单显式启用控制，窗口输入模块只翻译/排队，由 Session 在回调外
送入 worker。已呈现 Viewport 提供坐标，失焦/捕获丢失/菜单/关闭通过 StopSignal
立即撤销；UI 不执行跨窗口发送或 join。完整 WGC/UI/源控件联合验收仍待进行。

Session 现拥有可选输入 worker：显式 Enable ACK 后才准入，错误/关闭撤销，等待
输入退出再恢复源并交付会话完成。Application 主等待最多 51 个句柄，关闭最多 34；
UI 模式回复由线程消息唤醒。窗口菜单和键鼠消息翻译尚未连接这些入口。

`live_input_worker` 在线程内创建 Controller/Backend，以 16 项有界通道和队列外
停止标志运行，回复事件与线程句柄分别通知结果/终止。UI 关闭应异步等待清理和 join；
目前 Application/Session/UI 尚未接入，Drop 仍只是可能阻塞的最终兜底。

`live_window_target::input_backend` 组合安全、路由、坐标和消息模块，实现 Controller
的实际 Win32 Backend；held 目标通过 Clone 共享进程句柄，取消使用最后有效坐标。
生产调用必须在独立输入 worker 中，目前 worker/UI 接线尚未实现。

`live_window_target::input_routing` 只在原 root 内有界查找子控件，结果必须经过
Destination 安全预检；读取焦点而不抢焦点，保留手势失效只回退原 root。当前只读
路由不修改透明样式，尚未与输入 Controller/worker/UI 联合。

`live_window_target::input_coordinates` 在源 DPI 下处理客户区或物理 frame 裁剪，
经物理屏幕坐标桥接到子目标 DPI 下的客户区和滚轮屏幕坐标。线程上下文作用域恢复，
不持有额外图像或授权；Controller/worker/UI 尚未连接该转换。

`live_window_target::input_target::Destination` 绑定原 root 下的子 HWND，保留原
进程句柄并逐次校验身份/后代关系及权限。send_to/release_to 使用此身份投递，
不靠裸 HWND 长期授权；自动命中、焦点和坐标转换尚未接入。

`live_window_target::input_message` 封装受限键鼠消息与 root 投递，检查安全预检、
坐标范围和同线程超时陷阱。调用方必须是未合并目标消息队列的独立输入 worker；
当前尚未与 Controller/UI 连接，也不包含子控件路由及帧到客户区转换。

`live_input::Controller` 管理输入授权状态、单调序号和固定容量 held 状态；发生错误
即撤销控制，通过与普通 send 分离的 release 接口尝试释放，不触发点击补偿。该层
尚未连接 Win32 transport 和 UI，不允许把模型 backend 的测试结果作为实际输入验收。

`live_window_target::input_security` 提供不发送输入的安全预检：原 Target 身份、
默认桌面、同会话/用户和向下完整性边界。token/desktop 资源分别归属小模块，读取
有界且 RAII 释放；预检不是显式授权，UI 和输入 transport 尚未接入该入口。

Live 显示几何集中在 `live_viewport.rs`，D2D 绘制与帧像素映射共用目标矩形。
Host 仅在成功提交呈现后保存序号和几何；pending 帧不改变快照，尺寸变化未重绘、
暂停或关闭时不提供旧映射。此层不授予输入权限，也不把帧像素直接当成 HWND 客户区
坐标；原生输入投射的安全校验与发送链仍待实现。

当前原生分支：live_visibility_owner 已连接 Application、Session、共享 Registry
与 Journal。窗口 Live 按需启动恢复进程，同源共享登记/首次快照；Session 在 ACK
之后且输出仍有效时隐藏源，最后 lease 退出恢复。主等待集合加入恢复任务/子进程
句柄，关闭消息泵同时结算采集和恢复，最后源退休后退出 helper；区域 Live 不启动它。
shell 激活召回、最小化用户源兼容等仍待完成。以下恢复链描述保留了分阶段证据，
其中未接线结论是历史检查点，以本段的当前状态为准。
召回协调现已接入共享源 owner 的状态机：有效显式意图恢复源可见性，但保留 Live
会话；退休 ACK 后关闭空登记 helper，后续 Session poll 不重新隐藏已召回源。
隐藏前/已取消的意图和无意图的前台变化不能召回。该协调入口尚未连接 OS shell
输入 watcher，因此真实任务栏/Alt+Tab 召回仍待接线，不是已完成的用户操作功能。
最新 watcher 接线已补充：有隐藏需求时才安装低级键鼠 hook 与前台/最小化 WinEvent，
在 UI owner 线程分类物理手势、排除注入输入，通过有界事件唤醒 Application；主等待
集合容量由 34 调整为 35。poll 读取最新可取消意图，交给共享源召回状态机。全部召回
或关闭即注销。真实任务栏/Alt+Tab 用户操作及长时间 hook 可用性尚未验收。
真实 WGC 补充门禁已验证自有普通/初始最小化源：alpha=1 后颜色更新继续到达，
召回恢复可见性后仍有新帧，最后采集线程及恢复子进程退出。此证据使用受控召回
意图、测试专用像素读回，不代表任意用户应用兼容或生产启用 CPU readback。

`live_recovery_parent.rs` 为恢复子进程提供父端身份与生存期 owner：直接父 PID、创建时间
顺序、canonical executable 路径、输入及 ACK 输出管道 server PID 必须一致；只持同步/有限查询权限。
后续死亡判定借用原进程句柄，不依赖 EOF。已有同测试 executable 子进程的真实通过及
拒绝测试。`live_recovery_handshake.rs` 补充 Hello/Ready 的方向及 session 对应关系；
`live_recovery_pipe.rs` 已提供 overlapped 双向管道、绝对截止时间及对端退出唤醒，
同进程真实 Windows 管道已验证 Hello/Ready 往返；生产跨进程握手及恢复循环仍未接入。
每个 I/O 使用事件等待，不创建读写线程或空闲轮询；取消后等待完成才释放操作资源。
部分传输后的超时不能视为回滚或安全重发，取消清理也不是硬实时期限保证。
会话 owner `live_recovery_session.rs` 已串联认证、握手、请求和 ACK，失败流不再读取，
但保留登记责任。恢复入口阻塞等待原父句柄退出，恢复失败继续保留源 owner；生产
Live 调度接线和跨进程窗口恢复验收仍待完成。`live_recovery_runtime.rs` 已提供独占的私有
子进程入口，在配置/UI 初始化前分流，并在父死后的恢复失败阶段退避重试；原进程退出、
窗口消失或 PID/TID 改变的记录无写入退休，其他失败不丢弃记录。
父端 `live_recovery_client.rs` 现已实现同 executable 启动、握手、请求序号和原进程
句柄健康检查；Ready 前失败回收子进程，Ready 后不因 owner Drop 杀掉恢复责任。
真实测试子进程覆盖正常握手/关闭/join 和子进程退出拒绝新请求；Live UI/后台调度
尚未接此 owner，用户应用窗口的父死恢复验收仍待完成。
三进程自有窗口门禁现已通过：源窗口在观察者存活，登记进程正常返回或直接 process::exit
后，恢复子进程还原快照并退出；管道先关闭、父进程尚存活时不提前恢复。这是隐藏夹具
窗口的真实跨进程证据，不代表用户应用、实际透明隐藏或所有恢复失败重试均已验收。
句柄继承检查发现原 Command::spawn 会把无关可继承事件传给恢复子进程；已替换为
恢复专用 CreateProcessW/STARTUPINFOEX 白名单，只允许协议端点与 NUL。相同事件
反例从失败转为通过；不能据此宣称应用中所有其他进程启动路径均已隔离。
Client 的 asynchronous::Owner 已提供单任务后台执行：无请求队列，空闲不保留工作
线程，等待句柄可加入 UI 消息等待集合；完成时交回 Client 与响应，避免在 UI 执行
协议 I/O。应用尚需接调度与关闭流程，不能在 busy 状态直接 Drop 并声称无阻塞。
共享可见性 lease 已增加受 arm 约束的 hide 后端，使用 LAYERED/TRANSPARENT、alpha=1
和非激活置底；失败尝试原快照恢复，待退休记录禁止再次隐藏。自有隐藏窗口三进程门禁
已改用此后端，父死后完整还原通过。应用尚未接自动登记/arm/hide，用户源隐藏未启用。
新增 `client::journal::Journal` 将异步通信转换为 UI 线程的有界登记状态机，最多
16 源、单请求在途。Ticket 在登记 ACK 后才 ready；退休回调只记录意图，收到退休
ACK 前返回 E_PENDING，本地 lease 不清快照、不允许再隐藏。取消在途登记也须等待
结果并退休；关闭或丢失 Journal 立即撤销新隐藏授权。通道故障或 worker 丢失 Client
时不能把缺失句柄当作子进程死亡，未知责任继续保留。Application 等待集合、源会话
生命周期和故障关闭尚未接入，不能将此协调层视为已完成的用户窗口功能。
请求 worker 的 panic 现由 Client 外的 unwind 边界接住，保留原进程句柄并封闭
通信流。故障 Journal 在所有源恢复/取消并请求退休后才终止自建恢复进程，等待真实
退出后清除远端重放风险；任何仍受保护的登记都会阻止终止。此为故障收尾路径，
正常路径仍要求 Retire/Shutdown ACK，应用层仍须接通恢复及消息泵。
Application 正常关闭的 Live 采集任务现已改为停止后泵消息结算，不再直接清空会话
并阻塞 join；保活线程句柄直到完成，期间暂存 WM_QUIT。该接线覆盖采集会话，尚未
把 Journal 纳入同一关闭等待集合。Snapshot 的消失源退休判断已由本地 lease 和
独立 Binding 共享，源消失后仍等待远端退休确认，不写其他窗口。
Application::run 的所有 Result 出口也已统一清理：主循环正常结束或返回错误后均
执行 shutdown，再交付原始错误，避免非零 WM_QUIT 和 `?` 提前绕过 Live 消息泵。
这不代表任意 panic、驱动永久阻塞或操作系统强制退出均可在进程内正常收尾。
继承管道接收增加内核模式检查，拒绝同步或消息管道，避免错误端点破坏截止时间假设。
同 executable 真实子进程已验证认证、Hello/Ready、不可绑定源拒绝及空登记 Shutdown；
没有修改用户窗口，不代表父死后的实际窗口恢复已经验收。

独立恢复通信的纯数据层为 `live_recovery_wire.rs`：固定 schema、session/sequence/id、
最多 4096 字节长度前缀帧，严格字段/几何/属性校验和 ACK 对应关系。源创建时间字段
用于后续登记时身份绑定，不由 codec 证明真实性。`live_recovery_service.rs` 已承接有界
幂等登记：单请求在途、最近请求 ACK 重放、单调 id、防覆盖与安全退休。生产恢复进程启动、
session 握手 I/O 与恢复循环仍待接入；Target 的 recovery Binding 已实现登记时创建时间/PID/TID 核对及原源句柄
保活，恢复不按 PID 重新绑定，但身份校验不能代替可信父通道和用户授权。
EOF 不能代替原父进程句柄的死亡判定，坏帧也不能撤销此前已确认
的恢复责任。终止恢复仅释放成功记录，失败 owner 保留供重试。

外部 Live 源召回的纯策略位于 `live_reclaim.rs`：UI owner 持有意图序列，每个共享可见性
owner 持有 Activation，前台变化只消费有效显式 shell 意图而不产生意图。序号与时间戳
分离，最新快照须在同一 owner 串行读取和消费；该状态机不是输入鉴权或窗口恢复实现，
宿主 hook、共享 visibility lease 与独立崩溃恢复尚待接入。
`live_window_target::snapshot` 提供只读恢复快照，完整保存扩展样式和可读取的 layered
属性、位置及 placement；读取前后验证原目标身份，但不承诺跨进程多次查询的原子性。
不可读取的属性拒绝而非猜测。快照读取不改变窗口；restore 子模块可显式写回完整样式、
layered、位置/placement 和 topmost 分类，分阶段校验 Target，失败保留快照。独立记录
visible 防止隐藏窗口被 placement 意外显示；该底层尚未接外部源隐藏入口，也不替代
变更前的崩溃恢复登记。
`live_window_target::visibility` 提供单 UI owner 的共享源 Registry/Lease：首次快照
唯一、普通 peer 退出不恢复、最后 peer 恢复、主动召回不释放 peer。窗口恢复与记录
退休分两阶段，退休失败不重写窗口，失败条目保留且阻止新隐藏。条目及 lease 有界，
回调外释放借用；独立恢复通道须在 arm 前就绪，当前尚未接 journal 或隐藏入口。

Live 输出最小化或零尺寸时，Host 释放呈现 surface（交换链/导入位图），保留最新共享帧
与订阅；恢复再惰性创建，静止源不必产生新帧。WM_SIZE 触发暂停，WM_PAINT 使用 IsIconic
防止最小化期间重建。这不暂停捕获，也不改变源应用窗口状态；实际驱动显存释放时机未测量。

Live 消费者已有可合并的 HWND 通知桥：源更新只投递唤醒和连接代次，实际帧从绑定
消费者读取。回调在源锁外运行，不等待 UI，不新增线程或轮询；基于
[PostMessageW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-postmessagew)。
投递失败可观测且允许后续帧重试，不假定系统队列无限或终止通知必达；应用通过已有消息
后的补读与真实线程退出等待回收终止会话。`composition::live::host::Host` 已将通知、最新待绘帧和 GPU
surface 串联：绘制时惰性创建、补读最新帧并提交；零尺寸暂停，关闭或绘制失败释放
订阅和 GPU owner。宿主需分发消息、执行失效/BeginPaint/EndPaint，并在 HWND 销毁前
释放 Host；`composition::live::window::LiveWindow` 已实现专用窗口消息分发与生命周期，
捕获排除成功后才订阅源，源关闭或绘制错误关闭窗口但不退出应用。当前仅隐藏窗口合成源
验证；worker/window 配对由 `composition::live::session::Session` 实现，Application
持有最多 16 个会话。托盘“区域 Live”复用冻结选区，确认后才启动；取消不改文档。
已有事件的 refresh 补读遗漏通知，线程完成句柄负责最终回收，不依赖关闭消息必达。
Session 的启动事务先同步绑定并排除窗口，成功后才创建采集线程；失败不启动捕获。
线程完成句柄来自实际 JoinHandle，可加入 UI 的消息等待集合，避免源关闭通知先于线程
退出而漏回收。显式 stop 不立即 join，最终 Drop 仍等待退出；poll 保留源失败的原始错误。
应用已用 [MsgWaitForMultipleObjectsEx](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-msgwaitformultipleobjectsex)
同时等待线程、源进程与消息，最多 32 个去重句柄，无 Live 时保留 GetMessage；
不增加等待线程或周期定时器。自建纯色窗口的真实 WGC 链路已有显式运行证据，真实用户
窗口交互、多屏/DPI、HDR 与资源占用仍待验收。
应用最终清理由 `application_shutdown.rs` 统一协调，先停止通知与 Live 会话再回收后台
执行任务；UI COM apartment 与 HWND owner 共同归属 `host_lifetime.rs`。

窗口 Live 后端新增显式 Target：保存顶层 HWND/PID/TID 并保活原进程同步句柄，不靠标题
或当前位置重新寻找目标。WGC 使用 [CreateForWindow](https://learn.microsoft.com/en-us/windows/win32/api/windows.graphics.capture.interop/nf-windows-graphics-capture-interop-igraphicscaptureiteminterop-createforwindow)
绑定单窗口，复用已有双帧池与交付链；worker 在 GPU 快照前后复核身份。进程退出、目标
关闭或变化时终止，不退化为桌面区域捕获。Target 可跨线程移动；同一存活 PID/TID 内的
HWND 快速复用仍有启动竞态，已绑定 item 则由 Closed 事件管理生命周期。托盘“窗口 Live”
已接原生候选菜单，最多 64 个外部可见窗口，跳过当前进程、最小化和 cloaked 窗口。
候选绑定对象而非标题，选择后重新验证；标题有界、转义菜单控制字符且不写日志。区域与
窗口请求共用一个请求槽及应用会话预算，回调外才启动。窗口源尺寸变化已按预算重建，
最小化/隐藏完整策略与真实用户窗口捕获仍待验证。
源窗口销毁还注册限定 PID/TID/HWND 的 WinEvent 兜底：本机实际测试发现 WGC Closed
未到达时，帧等待不会自然退出。销毁事件关闭帧源，唤醒 worker 后由目标复核报告结束；
不追加轮询或等待线程，owner 留在泵消息的 UI 线程并在完成/析构时注销 hook。
源进程句柄还独立参与主消息等待，不要求销毁通知必达。poll 发现目标失效后关闭帧源、
注销监听并退休进程等待句柄，防止已退出进程导致持续唤醒；保留 worker 的源失败结果。
自有独立纯色子进程的正常关闭与强制退出已验证首帧、唤醒、线程及帧回收。Live 启动
拒绝未初始化或仅隐式 MTA 的调用线程；owner 的显式 apartment 必须保持到任务全部退出。
重建前归还变化帧，并保守预留旧池加新池的峰值预算；使用
[Recreate](https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.direct3d11captureframepool.recreate)
复用同设备/同帧池事件绑定，成功后才提交新尺寸，随后等待新帧交付。零内容事件保留
最后一帧，不创建零尺寸池或周期轮询；固定区域/显示器源仍在尺寸变化时终止。预算不足
或重建失败关闭会话，逻辑预算不等同于驱动实际显存测量。

Live 呈现后端可复用捕获设备创建 DirectComposition 链，直接将 BGRA8 DXGI 纹理
包装为 D2D 位图，持有共享帧而不是只留下裸 COM 引用。新帧必须来自同设备，依据
[CreateBitmapFromDxgiSurface](https://learn.microsoft.com/en-us/windows/win32/api/d2d1_1/nf-d2d1_1-id2d1devicecontext-createbitmapfromdxgisurface(idxgisurface_constd2d1_bitmap_properties1_id2d1bitmap1))。
重复绘制复用位图，resize 复用已有合成生命周期；生产路径无 CPU 回读。当前只是
专用窗口与隐藏窗口验证，托盘入口已连接但未执行真实桌面捕获或可见 UI 验收。

区域 Live 的后台入口复用物理选区到显示器的绑定，GPU 复制前后复核身份/布局，
不将过期快照放入共享帧源。快照尺寸与区域一致，但捕获池仍覆盖显示器；区域复制
不消除整屏池成本。复制使用 [CopySubresourceRegion](https://learn.microsoft.com/en-us/windows/win32/api/d3d11/nf-d3d11-id3d11devicecontext-copysubresourceregion)，
源/目标边界均在调用前验证，避免依赖驱动处理越界。UI 与原生呈现仍待接线。

`live_job.rs` 将持续 WGC 会话放入后台线程，复用 `CaptureJob` 的线程回收与结果合同，
独立准入最多 16 个存活任务。取消通过关闭帧源触发事件唤醒，而不只设置取消标记；
异常或退出的作用域 guard 会关闭消费者并归还名额。宿主 UI 与原生呈现仍待连接。

原生持续显示器捕获现有 `capture_wgc::live::Session`：一个 MTA owner 持有设备/两帧
池，帧事件只触发有界通知，图像提取与 GPU 快照在 owner 线程执行。通知等待不依赖
DispatcherQueue 或计时轮询；free-threaded 池的事件来自其内部工作线程，依据
[CreateFreeThreaded](https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.direct3d11captureframepool.createfreethreaded)。
最后消费者离开可唤醒捕获等待；关闭撤销事件并关闭会话/帧池。UI、呈现与输入授权
尚未接入此后端，真实捕获/显示器变化仍需验收。

原生 Live 的新帧交付底层采用单最新帧槽，多个消费者读取同一个 Arc 资源，不使用
浏览器 IPC 或 JPEG 编码。GPU 适配器通过同设备 CopyResource 创建独立纹理，避免
捕获池复用修改已交付帧；该异步复制不等于跨设备同步，约束见
[CopyResource](https://learn.microsoft.com/en-us/windows/win32/api/d3d11/nf-d3d11-id3d11devicecontext-copyresource)。
源持有一帧，消费者可持有旧帧，所有 GPU 快照按逻辑像素字节持有共享预算；这不是
驱动实际显存提交量的精确测量。持续捕获后端已连接此层，原生呈现与输入授权仍未接入。

原生登录启动设置由 `autostart.rs` 的策略层和 `autostart_registry.rs` 的 HKCU 适配器
负责，托盘 `tray_startup.rs` 只呈现状态和接收显式操作。没有启动时自动注册或后台
轮询；其他安装、未知状态和更新失败保守处理。Run 命令按 Windows 的长度约束生成，
不通过 shell 执行，参见 [Run and RunOnce Registry Keys](https://learn.microsoft.com/en-us/windows/win32/setupapi/run-and-runonce-registry-keys)。

原生托盘的通知图标生命周期由 `desktop/tray.rs` 持有；常用操作由子模块
`desktop/tray_actions.rs` 路由到现有区域截图、双轴长截图和图片加载的新钉图入口。
菜单只呈现当前准入状态，实际执行前及宿主恢复后均复核，避免嵌套消息循环使状态
过期。没有新增常驻线程或轮询；托盘 Live 和投射入口仍未完成迁移，自启动实现见上。

原生长截图拼接底层接受已定位的共享 BGRA 帧，规划连续的轴向片段，再一次分配合成
结果，避免每次追加复制已有长图。单帧/重复覆盖共享原存储；几何与总量验证先于输出
分配，且输出继续受全局像素租约限制。该层不负责重叠推断、固定栏或自动滚动，尚未
接到桌面捕获入口；合成测试不能代替实际长截图验收。
独立精确匹配路径用逐线 BGRA 签名和 KMP 线性扫描寻找双向重叠；唯一候选必须通过
原像素复核，多个候选明确报告歧义。此路径不处理有损差异、固定栏或近似位移。
独立近似分析器对调用方限定的双向位移（含零位移）执行有界稀疏搜索与密集复核，
不分配图像副本或所有候选列表。低纹理、多候选接近、误差超限分别拒绝；成功仍是
采样推断，不能视为精确像素证明。固定栏、动态区域和捕获控制尚未接入该分析器。
固定边缘分析已可识别两帧轴向首尾完全不变的连续区域，将内部区间零拷贝交给同一
近似评分器。只有非零内容运动通过验证才报告边缘长度，全帧相同返回 Unchanged。
成对分析之后，可用固定边缘合成入口逐帧复核不变区域，按内容坐标拼接并各保留一次
首尾，不分配中间裁剪图；边缘变化即拒绝。跨帧固定栏确认及变化/浮动固定栏仍待实现。
纯数据 Session 已串联分析与合成：精确优先，未匹配才进行近似/固定栏分析；按最后
接受帧累计位移，追加事务化，重复/拒绝/取消不推进状态。固定栏至少两次非零运动
得到一致边界才可合成，且每帧复核首帧固定区域。实际采集控制与变化/浮动栏仍待接入。
SessionJob 已提供后台所有权：单 worker、单在途命令、有界命令与结果通道，无持续
轮询。UI 非阻塞取结果及状态快照；取消关闭通道，迟到输出不可发布，Drop 负责 join。
采集/滚动控制尚未接入，调用方须在非 pending 状态才启动下一次截图，避免生产端积压。
采集适配器现已将冻结物理矩形接到 worker 的显式 capture 命令，GDI 只分配选区
大小的 DIB 与像素缓冲，复用既有双份峰值预算及句柄 RAII。采集失败不推进 Session，
取消后迟到结果被丢弃。自动滚动、图像预览和真实场景验证仍待完成。
菜单现有垂直/水平手动长截图入口：选区后打开原生控制对话框，后台采集与合成，
完成交付新钉图。宿主保持隐藏，控制窗口要求捕获排除成功；取消、关闭或显示布局
变化停止会话。完成计时器只在任务在途时存在，仍未实现自动滚动和图像预览。
图像预览现已接入独立后台命令：直接按片段采样最长边 384 像素的缩略图，不先合成
完整输出；控制窗口 GDI 按需绘制，旧缩略图在新请求时释放。自动滚动仍未实现。
单步滚轮传输已有目标绑定与发送前复核，使用 150 ms SendMessageTimeout，不广播、
抢焦点或移动鼠标；同线程路径拒绝，避免该路径忽略超时。消息交付成功仍须以截图
确认是否滚动，不重试已交付但随后取消的消息。自动循环和 UI 尚未连接。
单步 UI 已连接后台动作源：滚轮交付后可取消等待 250 ms，复核目标后捕获并交由
Session 分析；容量 1 命令规则仍适用。每会话限制 511 次发送尝试，绑定失败保留
手动采集并禁用滚动按钮。尚未实现连续自动循环或基于多帧的稳定判定。
连续自动采集已接 UI：只在 Added 回复后发送下一步，重复、失败或调度上限停止。
滚动后最多采样五帧，连续两次全像素一致才进入分析；不稳定不追加、不重发输入。
停止自动仅阻止后续步骤，当前任务收尾后保留结果，预览/完成仍由用户显式触发。
这些路径仅以合成源/隐藏窗口验证，真实跨应用和可见交互仍未验收。
自动进度预览按 Added → Preview → Scroll 串行执行，不额外创建 worker 或堆积帧。
固定边缘未确认时直接继续下一次采集；确认后刷新缩略图，下一步滚动保留该预览。
用户停止会使预览回复失去继续滚动的权限，不影响显示已完成的有效结果。
长截图接受候选前检查共享像素预算中的最终输出余量，失败保留上一次有效会话。
发送滚轮前分别检查下一帧加保守输出、稳定采样峰值，避免已知容量不足仍改变目标应用。
这些是短租约探测，不是永久预留；其它窗口并发分配后，实际捕获/合成仍可能失败并需重试。
长截图方向由会话控制窗口持有，按轴显示向上/向下或向左/向右；单步和自动循环统一使用。
有在途任务或自动循环时拒绝方向切换，防止迟到命令改变后续输入；空闲切换保留已有拼接。
二维长截图已有独立已定位帧合成内核：保留早到覆盖、支持负坐标，透明空洞仅作内部
覆盖标记，完成时选最大完整矩形；裁剪原地压紧，不额外分配第二张输出。画布限制 64 MP，
全程受共享像素预算约束，逐行响应取消。该内核尚未接取景框移动、位置验证或会话 UI。
位置验证内核现已补上：已知物理位移形成借用重叠区域，逐像素检查 RGB；若仍有页面
运动，则复用有界近似匹配寻找剩余轴向位移。不会复制裁剪图，失败/歧义不返回可用坐标。
此结果尚未接入取景框移动和二维会话，不意味着二维采集已可从界面使用。
二维数据会话已接位置验证和合成，累计相对最后接受帧的位置；预算不足、取消或匹配
拒绝不推进基准。预览直接在共享输入帧上采样，保留完整范围中的透明空洞，不生成
全尺寸中间画布；最终导出仍使用最大完整矩形。worker 和取景框 UI 尚未接该会话。
一维转二维可生成独立候选，聚合图和最后采集帧分别担任覆盖种子与匹配参考，保留历史
计数、负坐标和已确认固定边缘；未确认边缘拒绝转换。候选成功前原会话保持可用，实际
替换仍待 worker 接线，不能把生成候选视为已完成桌面模式切换。
worker 已接活动会话状态机：显式 AppendAt 只有接受新空间帧才切换，后续普通采集/预览/
输出复用同一 worker 和容量 1 通道。二维普通滚动保留唯一精确位移快速路径，低纹理渐变
不被近似分析门槛误拒绝。现有对话框可展示二维结果；取景框移动与采集源事务仍待接入。
采集源事务协议已接 worker：先准备候选区域并采集，再依据追加结果提交或回退；接受的
重复帧也可确认源参考，但不强制切换二维模式。失败或取消回退，迟到结果不发布。现有
闭包采集源继续可用，不自动获得移动能力；实际桌面区域源和移动 UI 仍待接线。
真实桌面源已实现候选区域移动：检查有符号坐标、重叠、虚拟桌面边界，再进行有界稳定
采样；接受后更新区域和同一目标的滚轮坐标。滚动目标身份保持不变，窗口移动/变化则
拒绝。手动源也支持候选移动但不获得滚动能力；当前仍未开放取景框移动 UI。
四向移动按钮已接 MoveCapture，UI 参考坐标与源提交结果一致，失败不更新；成功后串行
刷新预览。在途及自动采集期间不允许移动，无能力源禁用入口。步长有界并保留重叠，
不移动系统鼠标或目标应用；隐藏窗口已验证链路，可见取景框提示与真实桌面交互仍待验收。
二维预览空洞现以棋盘格显示：仅在收到有透明像素的有界预览时生成显示副本，不改原始
覆盖数据或导出；不透明预览零复制，GDI 重绘不分配新像素。预算失败停止后续自动采集。
可见取景框已有四条窄层叠顶层边框，位于采集区域外，鼠标穿透且不激活；逐窗排除捕获，
不建立整屏覆盖画布或空闲刷新计时器。仅接受移动后定位，关闭立即销毁。隐藏 HWND 回归
已覆盖几何、样式和清理，但真实捕获排除、跨应用鼠标穿透和可见多 DPI 效果仍未验收。
跨目标移动现允许在虚拟桌面内继续取景，接受后撤销自动滚动能力并切为手动，不自动
绑定其它窗口；原目标身份仍在采样前后校验。源能力随每次 worker 回复更新，UI 禁用
滚动入口并显示手动状态；失败/拒绝不改变旧授权，后续手动采集和移动继续可用。

会话读取现验证主干 captureMeta：有界解析来源类型、来源矩形的合法数值、动态范围、
位深、色彩空间、backend 和降级标记。缺失不推断，已知字段矛盾拒绝，未知扩展保留
原始 JSON。保存快照与恢复文档已接入元数据映射，来源为空时清除旧 captureMeta。
恢复后再保存保留未知扩展；新捕获保存动态范围/位深/色彩空间/降级状态及来源类型、
初始取景矩形、长截图方向和实际 backend。原生矩形显式标为 physical，避免把来源
历史当成逻辑坐标或当前拼接输出尺寸；缺失 origin 时不补造矩形。解析成功不
代表原图文件存在或获准访问；只有独立内联原图验证成功才恢复租约，信息框区分仅有
元数据与实际持有原图的状态。

HDR 原图保存端已将活动租约快照到后台，流式 Base64 编码后写入原生会话扩展
otterNativeCaptureOriginalSrc；原图字节保真，不通过 SDR 预览反推。只增加有预算的
最终 Data URL 和 48 KiB 输入缓冲，不额外读取整份压缩文件。会话仍受 64 MiB 总量
限制，超限明确失败。恢复端已将该扩展流式解码到独占临时文件，逐行验证后把句柄
交给文档，可另存并再次保存；最后 owner 释放即删除临时文件。不读取外部路径，也
不把原图解码成 SDR 底图。该原生扩展不能视为旧版本读取器已支持原图。

文档现在持有 HDR 原图的只读文件租约，发布阶段在缓存排他锁内建立，随后只保留单个
文件句柄而非全局锁。禁止写入/删除共享，既有清理/容量回收遇到活动文件会保留；其它
截图仍可暂存。替换、销毁或同步时发现文档身份失效即释放，之后恢复正常缓存清理。

HDR 原图缓存复用现有 clipboard_cache 所有权机制：按编码上界预留容量，排他锁覆盖
准入、流式暂存和发布；编码时同步计算实际长度与 SHA-256，发布回执绑定文件身份。
取消或编码失败只清理本次未发布暂存，成功发布后的 PNG 不因迟到取消删除；回执写入
失败仍保留 PNG 并报错。没有新增完整压缩 PNG 内存副本，实际 UI 交付仍待接线。
后台任务已有 start_cached 入口，采集与原图编码在同一 worker 完成；PreparedCapture
持有预览、暂存和诊断，RGB16 编码后释放。提交接受回调在 UI 线程复核 owner，失败
不发布原图，成功返回预览与文件路径；既有 Outcome 路径保留。等待窗口新增 wait_cached，
复用同一模态生命周期处理 PreparedCapture，返回后仍不发布文件；区域截图菜单现已
消费该类型，主线程复核文档后发布并交付，真实桌面行为仍待验收。
文档准备与新钉图/替换提交现在接受 Captured：不解码原图路径，直接使用同帧 SDR 预览，
保留原图路径及采集诊断。来源绑定文档身份，正常替换清除，身份变化不再可见；它只是
来源追溯，不能作为当前编辑结果导出，也不授予文件访问权限。现有 GDI 截图已使用该
交付类型；区域截图菜单已接 HDR 路由，来源元数据和原图已接内联持久化，编辑后的
HDR 导出仍待接入，不能把原图字节另存当作 HDR 编辑导出。
原始 HDR 截图已有独立另存入口，直接从活动租约句柄流式复制完整 PNG，保留原始像素
和元数据，不合成后续编辑。保存路径选择后复核文档/租约身份，后台无覆盖提交；不把
复制原图宣称为画布 HDR 渲染或编辑导出。普通 PNG 导出保持现有行为。
右键“截图信息”按需展示原图格式、SDR 预览、HDR 降级及原因、显示器亮度元数据和
原图路径；降级时菜单标签提示，普通文档或来源身份过期时禁用。未知显示器配置和
白电平回退值显式区分。无常驻窗口/刷新计时器，不打开路径，不自动弹出诊断信息框。

HDR 纹理读回已有 D3D11 staging 接口：只接受同设备、单 mip/单层/非多采样 FP16 纹理，
使用 DO_NOT_WAIT Map，并对 busy 状态设置 250 ms 重试期限；设备错误直接返回。
回调借用映射中的行缓冲，guard 在成功、错误、取消和 panic 时 Unmap。调用方必须独占
immediate context，不得并发修改或映射源纹理。预算先记录逻辑 staging 像素，再根据
映射行跨度补足；不声称精确计量驱动私有显存，也不声称期限能中断驱动 API 本身。
WARP 合成纹理已实际验证 SDR/HDR 读回。
一次性 WGC 显示器入口已实现独立硬件设备、FP16 两帧池与一秒有界取帧，成功帧接到
读回层；内容尺寸或纹理尺寸变化立即拒绝，不转换未定义边缘。关闭顺序为帧、session、
pool、设备与线程 WinRT；不缓存空闲资源。保留系统边框且禁用光标采集，不请求无边框权限。
显示器局部物理像素的区域入口已接 GPU 裁剪读回，拒绝空、越界和溢出选区，绝不隐式截断。
仅 staging、映射和输出转换随选区缩小；WGC 两帧池仍按显示器大小计预算，不能宣称
小选区消除了整屏捕获缓冲。桌面全局物理坐标绑定层已实现，负坐标仅做原点平移，
不再次乘 DPI；通过 MonitorFromRect 选择候选后仍校验完整包含，跨屏返回明确错误。
采集前后复核句柄、设备名和矩形，变化时丢弃转换结果；这不是硬件身份或布局代次的
永久保证。动态范围编排已有 Auto/HDR/SDR 入口：标准区域、已确认 Windows 11 能力且
显示器启用 HDR 才尝试浮点捕获；显式 SDR 和非标准工作负载跳过 HDR 探测。
Auto 内容未超过 SDR 白电平时直接从同一浮点帧生成 SDR，不重新截图；可回退失败只
采集一次同区域 SDR，取消/布局变化不回退。成功后再次查询颜色元数据，改变时丢弃结果。
HDR 结果现在同时保留 RGB16/PQ 原图及同一浮点帧生成的 SDR 预览；按实际 SDR 白电平
转换预览，不把 PQ 数据误当普通 sRGB。两份输出分别计共享预算，任一失败不交付半成品；
迟到取消释放两份输出。into_parts 移交而不复制像素，后续文件/画布 owner 必须分开使用。
HDR 文件已有 PreparedHdr 两阶段发布：流式编码到同目录唯一暂存，sync_all 后等待 owner
复核，再无覆盖重命名。无需保留整张压缩 PNG；失败/取消/放弃仅清理本次暂存，成功后
不因迟到取消删除已发布文件。复用 Staged 新流式入口，不伪造会话候选身份。临时目录
真实文件测试已验证解码回读、冲突保护和清理；UI 交付、缓存归属及真实 HDR 查看仍待接线。
输出区分正常 SDR 与 HDR 降级，保留来源配置和错误诊断。Options::detect 已接只读
RtlGetVersion 查询，数值比较系统版本；探测失败返回错误，不将未知当作 HDR 可用。
Mode::parse 对齐主干的空白/大小写及未知值 Auto 语义。返回前再次核对选区尺寸、像素
长度和取消状态，错误回调结果不得交付。生产适配入口已复用原 GDI region 作为同选区
SDR 回调，移动共享 Pixels 而不再次复制。它要求后台执行，不能中断已进入的 BitBlt；
取消只能阻止开始或交付，系统版本探测失败则明确返回诊断。尚未接 UI 请求与文件发布，
自建纯色窗口的 WGC 采集、更新、尺寸重建和关闭已做显式运行验证；显示器采集、真实用户
窗口、多屏/HDR 与完整交互仍未验收，版本门槛不代替 WGC 实际能力检查。
CaptureJob 已将生产适配入口放到命名后台线程；每对象只有一个结果，无无界队列或
空闲轮询。take_ready 在任务未完成时返回 None，完成后只提取一次；取消既在 worker
返回时检查，也在 UI 提取时检查，已完成但未提取的结果同样可拒绝。Drop 取消并 join，
不遗留分离线程，但仍可能等待不可中断的 OS/驱动调用。UI 的任务数量、窗口/文档代次
和菜单路由仍须由上层接入。等待对话框已接 CaptureJob，先设置捕获排除再启动，只有
在途任务保留完成检查定时器；取消后保持消息泵、禁用取消按钮，后台退出后返回空结果。
显示布局变化返回错误，外部销毁清理定时器并取消/join；后者仍受驱动调用退出时间影响。
隐藏对话框测试覆盖结果交付、取消和销毁，没有开启真实捕获或验证可见窗口排除效果。
WARP 测试验证实际裁剪像素和选区 staging 的逻辑预算，坐标/布局复核采用注入测试。

HDR 颜色转换层已有借用 FP16 scRGB 缓冲的亮度分析及 SDR/BGRA 回退，处理行填充、
异常浮点值、白电平和增益；输出受共享像素预算限制并逐行取消，不复制原始浮点帧。
这是与主干一致的白电平归一化/限幅回退，不是保留 HDR 高光的色调压缩；真实 HDR
捕获与界面降级提示仍未接入。
HDR PNG 编码已有独立流式入口，写出 RGB16、cHRM/cICP/mDCV/cLLI 元数据并验证 IEND
及 flush；按行编码，不累计整张压缩数据，预留编码工作区预算。输出失败/取消后 sink
拒绝析构期间的后续写入。该入口尚未接截图文件发布流程，调用方仍须负责持久化与失败文件。
HDR 输出转换另走 BT.2020/PQ RGB16 大端路径，不经过 SDR 限幅；输出携带实际帧分析及
mastering 亮度范围，并复用共享字节预算与逐行取消。负值和非有限通道继续按主干策略
归零，不宣称保留负 scRGB 色域。PQ 与双精度参考、行填充/通道顺序和租约生命周期已有测试。
显示器元数据已有独立只读查询层：匹配 DXGI 输出与 DisplayConfig 活跃路径，读取 HDR
启用状态、SDR 白电平和亮度范围；枚举及配置变化重试有界，不缓存动态颜色状态。
查询失败与默认白电平通过独立标志暴露，尚未接入实际捕获路径，也未做真实 HDR 显示器验收。

原生屏幕取色以显式会话拥有临时鼠标 hook、模态状态与 HEX 预览窗口；hook 回调只
合并/投递事件，UI 消息按需用 GetPixel 采样一个像素并即时释放 DC，不保留整屏位图。
会话代号隔离迟到消息，确认复核来源，结束即卸载 hook；真实跨应用输入和 HDR 尚待验证。
使用临时 hook 是因为 [SetCapture 不能代替跨进程鼠标输入捕获](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setcapture)。
取色预览使用自身 STATIC 窗口的子类绘制色块和 HEX，保留原生可访问文本；DPI 改变时
同步尺寸、字体和内距，跨屏定位最多校正一次，不使用布局定时器。GDI 字体/画刷与
选入对象均按作用域回收/恢复，窗口销毁移除子类；实际多屏可见效果仍需人工验收。

原生 Loom 流的取消状态由流和中断句柄共享，Windows 先标记取消再触发独立事件；
后续读写、flush 及截止时间更新不能恢复已取消流，错误为不可重试的 ConnectionAborted。
WSAEventSelect 保证底层非阻塞，WouldBlock 后共同等待网络与取消事件；不用 socket
副本 shutdown 唤醒读取，完成网络通知通过 WSAEnumNetworkEvents 原子复位。

原生宿主的 HWND RAII 回收与 UI 线程宿主计数统一由
`native/src/desktop/host_lifetime.rs` 负责；窗口和绘制资源先于 COM apartment 释放。
图像打开、导出及编辑任务的完成通知由 `desktop/procedure_tasks.rs` 分派，各任务仍自行
拥有定时器停止、取消和资源回收；路由层不新增空闲轮询。

原生文件拖出传递层 `desktop/drag_transport.rs` 按需初始化 OLE，使用标准 Shell
IDataObject 和 CF_HDROP，不借用系统剪贴板，不允许移动或删除源文件。Shell 数据对象
接管有界路径包内存；DoDragDrop 是可重入调用，生产手势接线必须避免跨调用 UI 借用并
管理快照存活和关闭取消。`desktop/drag_prepare.rs` 单任务 worker 冻结快照、丢弃撤销
历史并仅编码 PNG；未发布暂存由任务 owner 取消/回收，提交和最终文件存活由调用方
控制。`drag_input` 已将平移工具下的 Shift 手势与系统阈值接到准备 worker，保留
选择/绘图工具的 Shift 操作；`drag_input_finish`
复核来源后发布并调用 OLE。活动期间保护来源文件、阻止命令重入、关闭触发取消；
文件在 OLE 返回后保留，不因取消直接删除。真实键鼠及外部目标的验收仍未完成。
准备阶段的普通取消只发出信号并保留任务 owner，由已有完成定时器回收；WIC 编码
不会因松键处理而被同步 join。已取消任务不再消费编辑鼠标移动或释放后续捕获，
同时占有唯一准备槽防止堆积。窗口销毁边界仍等待任务退出，不遗留后台线程。
拖动预览由同一 worker 从已合成 WIC 位图生成，最长边限制为 256 物理像素且不放大；
UI 只建立有界 DIB 并一次交给 Shell IDragSourceHelper，不重新解码 PNG 或逐帧合成。
像素使用直通 alpha，遵循 [InitializeFromBitmap 的 alpha 要求](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-idragsourcehelper-initializefrombitmap)。
预览失败不阻止文件复制；真实可见拖动、居中热点及跨 DPI 效果仍需人工验收。
实现依据 [Shell 数据对象](https://learn.microsoft.com/en-us/windows/win32/shell/dataobject)
和 [OLE 拖放初始化](https://learn.microsoft.com/en-us/windows/win32/shell/dragdrop)。

The `feat/windows-native` branch also contains an independent migration host in
`native/`, built from `native/Cargo.toml` with its own lockfile. It uses a blocking
Win32 message loop, WIC image decoding and a per-window Direct2D render target.
It currently supports image display, pan, zoom and basic opaque annotations with
gesture-level undo/redo. The window-independent editor owns drafts and bounded
incremental history; the D2D adapter owns drawing resources. The native
runtime also owns shared naming settings across document windows. PNG export
templates can be edited for the current run through a native dialog; new and
restored windows retain that shared owner. The naming context follows the main
frontend contract: capability labels supply label/title, while process means the
artId, not an operating-system process name. Offline restored Art retains its ID;
context construction borrows the catalog without copying parameters or images.
The remaining naming consumers are still pending.
`app_settings_document` adds an I/O-free, bounded JSON editing boundary for that
explicit import/save-as path. It preserves unrelated configuration values, validates
naming fields and rejects unsupported schema versions without resetting data.
An explicit naming-import menu reads a selected file through a single owned
worker with a pre-allocation size bound. Publication checks the originating unit
and prior settings; cancellation, invalid input and stale results preserve the
current settings. Import is read-only. Save-as retains the imported document's
unrelated fields and schema, stages bounded bytes on a worker, then checks owner,
settings and source identity before publishing a new file without replacement.
The saved file can be imported again or explicitly loaded at startup with
`--settings path`. Validation precedes capture/window creation, and restored
windows inherit the same naming owner. Invalid settings fail startup without
resetting data. Explicit updates to the active settings file retain the original
bytes, reject external edits, and reuse the recoverable replacement transaction.
Only successful publication advances the baseline; save-as switches the active
source to the new file. Replacement is not atomic, and retained recovery files
are reported. Explicit startup loading can read a unique recovery copy only when
the original file is missing, with bounded scanning and the same byte limit.
Existing originals take precedence, including invalid ones; ambiguity is rejected.
Recovered sources require save-as and are never overwritten by the update path.
Recovery does not modify any file. Optional naming autosave is off by default and
is not persisted across launches. It reuses the single save slot on changed values,
without an idle timer. Disabling it discards unpublished automatic results; errors
disable retries and retain runtime settings. Application-level exit waits for an
enabled pending save; individual window closure and forced termination do not.
Normal startup uses LOCALAPPDATA/OtterNative/app-settings.json, isolated from the
main application. OTTER_NATIVE_PROFILE_DIR overrides the directory; --settings
takes precedence. A missing initial profile does not create files until saving;
first publication never replaces a concurrent writer. Hidden checks skip profile
discovery unless explicitly overridden, and child tests remove inherited profile
overrides. Power-loss validation remains pending; no implicit access to the main
settings directory occurs.
The native naming dialog edits save, drag and clipboard filename patterns plus
the title length as one bounded draft. Validation and stale-state checks precede
shared publication; cancellation changes nothing. Existing persistence owns file
writes, not the dialog. Drag filename consumers and visible dialog/DPI
acceptance remain pending.
`file_naming::allocate` owns atomic exact and incrementing filename allocation.
Explicit PNG export uses exact creation and still refuses existing files. The
incrementing API preserves the main naming sequence and bounded suffix length,
retries only collisions, and supports cancellation before allocation. Default PNG
save now consumes it, writing a frozen display snapshot under the isolated native
profile's saved directory. The existing single export worker owns directory creation,
encoding and writing; explicit --settings does not redirect image output. Drag
production consumers remain pending. Allocation does not
authorize directories or own cleanup after a successful return.
Clipboard publication now accepts an optional bounded Unicode CF_HDROP packet,
preparing all image/file HGLOBAL allocations before opening and emptying the
clipboard once. Partial publication after emptying cannot be rolled back; callers
must preserve completed backing files even on such failures. Existing copy jobs
now freeze the clipboard filename pattern and isolated cache directory, stage the
encoded PNG on the worker, then revalidate window/document/cancellation before
publishing the backing file and all three formats. Hidden smoke injects image-only
mode; file-flow tests inject only the final publisher and use isolated disk paths.
Actual cross-application paste acceptance remains pending.
Application-wide copy authority cancels earlier window requests without transferring
their job ownership. The production path snapshots the clipboard sequence and checks
it before publishing cached files and again under the clipboard lock before emptying.
Cache-lock contention waits only on active workers, bounded to one cancellable second.
Tests inject sequence values and publication; they do not modify the system clipboard.
Sequence semantics and lock ordering follow Microsoft's
[GetClipboardSequenceNumber](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getclipboardsequencenumber)
and [OpenClipboard](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-openclipboard)
contracts, including delayed-rendering limitations; this is not global ordering of
unpublished intents across processes.
The cache transaction now stages encoded PNG bytes under the isolated profile,
holding an exclusive cooperative lock through bounded admission and publication.
Unpublished stages are reclaimed on cancellation/drop; successful publication
retains the backing file. Incremented publication renames the same stage without
rewriting image bytes and is forbidden for revision-bound session candidates.
Admission rejects caches exceeding the configured disk quota or 4096 entries rather than deleting
unknown files. If retention cannot free enough space, a full cache fails explicitly
rather than silently downgrading copy to image-only formats.
Newly published cache PNGs receive bounded ownership records containing filename,
size, SHA-256, creation time and volume/file identity. Admission reserves space for
both the PNG and its record. A record failure retains the published PNG and reports
its path; legacy files are not retroactively claimed. These records support
conservative retention checks, not authorization against malicious local tampering.
Before staging, the locked worker retires verified files older than seven days.
Both receipt and PNG handles deny write/delete sharing through verification and
handle-based retirement; identity, size, digest and single-link checks are required.
Unknown, changed, replaced, busy, hardlinked or future-dated artifacts are retained.
Each run bounds enumeration and hashing; tests only clean isolated temporary data.
Startup cache settings now control the disk quota and retention (256 MiB/seven days
by default; zero disables the respective soft policy), with main-compatible range
validation before capture or window creation. New/restored windows share the policy,
and copy tasks freeze it. Naming imports preserve but do not apply cache fields.
Entry, single-PNG and hash-work limits remain independent. Under quota pressure,
the locked worker orders bounded receipt candidates by creation time, rechecks the
entire receipt and file identity/content, and retires eligible entries toward half
quota while reserving the incoming output. Age and pressure cleanup share a hash
budget. Zero quota disables eviction; impossible incoming sizes fail before cleanup.
Unknown/changed/busy artifacts remain protected and insufficient space still fails.
The native cache dialog edits exact byte quota and retention days as a validated
draft; cancellation, stale edits and closed owners preserve current preferences.
Confirmation updates shared memory, not disk cleanup. Save/save-as/optional autosave
snapshot both naming and cache policy, reject either changing during selection or
background staging, and merge known cache fields while retaining extensions. A
cache-only change participates in autosave and exit waiting. Visible dialog/DPI
acceptance remains pending. Cached references can expire by age or capacity.
The native
`desktop/path_handles.rs` adapter shares handle previews and gesture lifetimes for
line/arrow endpoints and Bezier controls. Model transactions remain in the respective
editor modules; rotation-center compensation is shared by `rotation.rs`. PNG export reuses the
annotation renderer on a WIC-backed target at source resolution, with one background
job sharing immutable source pixels and snapshotting committed edits. Existing files
are never overwritten; interrupted writes may retain a newly created partial file.
The `--pin` entry uses a borderless topmost HWND and a premultiplied-alpha
DirectComposition swap chain. Image and annotation rendering remain shared with
the ordinary workspace. Presentation is change-driven, and resize drops backbuffer
references before reallocating. Pixel readback exists only in native tests.
Pin hit regions use cached, bounded alpha runs through `SetWindowRgn`; editing
restores the full client region. Annotation bounds and highly fragmented alpha
use conservative regions, not exact per-pixel hit testing. Cross-process click
acceptance, dialog precedence and multi-monitor parity remain pending.
Workspace file commands are owned by `desktop/application_commands.rs`; callbacks
queue bounded requests rather than opening dialogs or replacing window owners.
`application_open_session.rs` validates the source workspace across file selection
and confirmation before starting the restore worker. `application_save_ui.rs`
coordinates background encoding, guarded file publication and save-before-exit.
The `otterNativeCoordinateSpace` extension records the saved coordinate context;
explicit caller coordinates override it, while legacy files require explicit context.
Visible legacy restoration offers a bounded native coordinate dialog with empty
defaults; cancellation or source changes leave the original workspace intact.
Hidden checks never open that dialog and reject missing context.
Selecting a session file does not grant access to its referenced asset directories.
An explicit asset-directory variant uses a native folder picker and transfers the
pinned directory owner to the restore worker. Deferred save-before-open retains
that same owner; cancellation releases it. Admission remains limited to direct
child files, not recursive directory access.
These paths have hidden integration coverage; visible dialogs and complete parity
remain under development.
It does not replace the
complete application described below or automatically load its user data. See the
[active migration plan](../development/WINDOWS_NATIVE_MIGRATION.md).

Otter is a Windows-first Tauri 2 desktop application with a SolidJS/WebView2
frontend and a Rust native backend. The same main window is used in three
operational modes:

- **overlay** — transparent desktop surface used for capture and pinned stickers;
- **canvas** — focused editing and workflow workspace;
- **tray** — the main window is hidden while the process remains resident.

Startup configuration flows through:

```text
start-otter.bat
  -> launch-config.cmd
  -> start-otter.vbs
  -> Rust boot-profile parsing
```

The startup wrapper is not a passive launcher: `start-otter.vbs` force-terminates
existing `otter.exe` processes before starting its target. Do not use this wrapper
when preserving incident evidence or testing single-instance refusal. Exit normally
and launch the selected candidate EXE directly for those checks.

The tray exposes capture, live capture, long capture, open-image, receive-projection,
checkable current-user autostart, and quit actions; its settings entry is hidden.
Live capture uses the native backend directly for every application, including
browsers. It binds window-local pixels, not webpage elements. The browser
extension shortcut relay, document provider, and import lifecycle were removed;
the generic Loom extension bridge remains independent of Live capture.

The native `LiveCaptureFrameBuffer` owns a three-item local IPC queue and an
independent latest-frame snapshot for relay consumers. Immutable JPEG bytes are
shared across these references; draining a local read cannot starve the relay.
Clearing capture state releases both owners. Tauri's owned response buffer is
obtained or copied only after releasing the capture mutex; NLLV framing remains
connection-owned and is not a zero-copy transport.
The app-settings command and dialog remain implemented, but their tray entry is
temporarily hidden.

The normal LiveRelay UI shares one controller in `LiveFeatures`. `UnitLivePublication`
publishes a local Live Unit; `UnitLiveViewer` discovers and explicitly joins from a
Surface-capable Art parameter panel using its mounted `surfaceStore` identity.
Join admission and late-result cleanup live in `liveRelayController`, not in a
second transport. Closing the panel cancels only an unfinished join; established
viewer windows retain the global owner. Native/Loom attachment and viewer authorization
remain authoritative, and input ownership is acquired separately.
`liveRelayPresentation` holds one decoded-submitted sample owned by that controller;
`liveRelayDiagnostics` projects fixed bounded fields into each viewer's DOM attribute.
No new transport, timer, media copy or persistent history is introduced. Package/process
binding remains an external acceptance step, described in `../features/LIVE_RELAY_DIAGNOSTICS.md`.

`native/live/relay/live_relay_viewer_state.rs` owns viewer control bootstrap and terminal
confirmation. Bootstrap anchors to the exact attach revision's `viewer_joined`
event; only pre-join history is skipped, and missing post-join events still fail
closed. The same attach snapshot seeds the bounded observation map before either
worker starts, preserving strict per-observation ordering after late join. On
control poll failure, an authenticated, identity/epoch/member-checked
snapshot must explicitly report `closed=true` before stopping workers and clearing
pixels. Transport errors and generic 404 responses remain recoverable. Terminal
cleanup and frame acceptance share the state-to-frame lock order so late media or
connection callbacks cannot resurrect a stopped viewer.

## 2. Repository layout

```text
Otter/
├── native/                     independent Windows-native migration host and tests
├── src/                        SolidJS frontend
│   ├── main.tsx                desktop / Tile lazy entry
│   ├── app/                    startup and cross-feature orchestration
│   ├── features/               UI, state, hooks and services grouped by domain
│   ├── platform/               Tauri and desktop interaction boundaries
│   ├── integrations/           Otter-side Loom / Tea adapters
│   └── shared/                 domain-independent primitives and global styles
├── src-tauri/
│   ├── src/                    Rust modules; native/ groups include fragments by owner
│   ├── crates/                 locally maintained capture/drag dependencies
│   └── tauri.conf.json
├── scripts/                    build, packaging, smoke, and release helpers
├── tests/                      frontend unit/runtime/contract/performance tests
├── .github/workflows/          CI, release, performance, and signing workflows
└── docs/                       guide, architecture, features, development, release, security
```

目录归属和新增文件规则见 [代码组织与开发规范](../development/CODE_ORGANIZATION.md)。
目录分组表达已有状态所有权，保留 Rust include 作用域；功能目录不代表历史跨功能依赖已消除。

## Projection editing ownership

`useProjectionTargetControls.ts` owns secondary-toolbar checkbox intent, fresh
directory validation and per-target operation feedback. It reuses the durable
create journal and batch sender; cancellation persists `stopPending` on saved
bindings and waits for the existing sync owner's unlink result. Uncertain creates
are cancelled in the journal before unlink. `projectionTargetStatus.ts` distinguishes
invitation creation from receiver display and aggregates group results. Sync status
snapshots replace earlier fields, so recovery cannot retain an obsolete error.
The full projection management dialog retains its separate batch-selection flow.

`projectionEditEngine.ts` serializes each Unit's editing operations and fences
workspace changes, removal and delayed responses. `projectionEditSession.ts` binds
that engine to graph actions, native transport and the existing projection poller.
`projectionEditJournal.ts` owns bounded IndexedDB records with transaction-complete
durability and revision compare-and-set; ordinary workflow saving does not replace
this request journal. Geometry, validation and concurrent-view merging have separate
modules. Otter owns original annotations and base-image rasterization; Loom owns
authenticated bindings, shared revisions, mode authority and checkpoint fences.
The native `qr_projection/edit.rs` boundary validates typed requests and documents
before routing through the paired local or offline-peer projection endpoint.

## 3. Frontend architecture

### 3.1 Integration entry

`../../src/app/app.tsx` owns cross-subsystem orchestration rather than domain algorithms.
Its responsibilities include:

- registering Tauri event listeners;
- restoring the persisted session;
- entering and leaving capture modes;
- routing native overlay mouse events;
- coordinating global shortcut results;
- connecting canvas, sticker, history, settings, Loom, Talk, and Tea surfaces.

New pure domain logic should not be added directly to `app.tsx` when it can live
in a focused service or hook.

### 3.2 State ownership

- `../../src/features/workspace/state/graphStore.ts` owns persistent workspace data: units, links, groups,
  recycle bin, reference library, parameters, and persistence-facing mutations.
- `../../src/features/workspace/state/uiStore.ts` owns transient interaction state: selection, active edit
  target, panels, tool modes, capture UI, drag previews, and temporary notices.
- `../../src/features/workspace/services/enhancementNoticeQueue.ts` owns notice identity and bounded FIFO
  transformations; `UnitVisualOverlays.tsx` renders each unit's notices in a
  right-aligned, dismissible stack so messages never leak across sticker/Art
  boundaries.

Persistent edits should pass through `graphStore.actions`. Transient high-rate
interaction state should not be written into the persisted graph on every input
sample.

### 3.3 Interaction hooks

- `useSelection.ts` — region/window/long/live-capture frontend lifecycle;
- `liveCaptureController.ts` and `liveCaptureStore.ts` — transient local live
  presentation, binary-frame polling, object-URL ownership, and view geometry;
- `useDraggable.ts` — sticker/unit drag sessions, compositor transforms, snapping,
  link previews, GPU warming, and final position commits;
- `useClipboard.ts` — internal units, system images, file payloads, and cascading
  paste behavior;
- `clipboard_history.rs` — read-only Windows history and system-event subscriptions,
  bounded background bitmap decoding and compact thumbnails; no private history archive.
  `StickerClipboardHistoryPanel.tsx` owns transient menu reads/subscription cleanup;
  `stickerClipboardPaste.ts` commits explicit selections as new images or undoable text.
  `stickerContextMenuLayout.ts` owns viewport avoidance and the combined native hit
  rectangle; clipboard scrolling reuses `OverlayMenuScrollArea` for synthetic input.
- `useShortcuts.ts` — context-sensitive frontend shortcut dispatch;
- `useUnitActions.ts` — minify/restore and higher-level unit actions;
- `useFileDrop.ts`, `useLinking.ts`, `useNodeParameters.ts` — focused intake,
  graph-link, and parameter behavior.

### 3.4 Rendering components

- `CanvasUnits.tsx`, `CanvasLinks.tsx`, `CanvasSelection.tsx` render the workspace;
  `CanvasGraphLink.tsx` retains each SVG edge while reactive geometry changes;
- `UnitView.tsx` renders each sticker or Art node and owns its DOM interaction
  boundary;
- `StickerAnnotationLayer.tsx` owns editable annotation rendering and geometry-
  aware hit testing;
- `StickerTopStrip.tsx` and `StickerTopStripPropertyBar.tsx` own sticker editing
  tools and properties;
- `UnitParamsPanel.tsx` and `components/params/` render Art parameters;
- `ShaderPreview.tsx` and `ShaderRenderer.ts` provide the GPU shader preview path;
- context menu, history, group, snapshot, settings, and color-picker components
  remain separate UI surfaces.

### 3.5 Domain services

Services now live under their owning feature or boundary directory:

- **desktop boundary** — `api.ts`, `bootProfile.ts`, `client.ts`, `protocol.ts`;
- **session/workflow** — `syncService.ts`, `sessionStickerMapping.ts`,
  `sessionStickerPayload.ts`, `sessionAssetReferences.ts`, `workflowInstantiation.ts`, `workflowPayload.ts`;
- **Art** — `artCapabilities.ts`, `artCapabilityNormalization.ts`,
  `artCandidateCache.ts`, `artDelivery*.ts`, `artNodeFactory.ts`, `artPorts.ts`;
- **sticker editing** — `stickerEditing.ts`, `stickerGeometry.ts`,
  `stickerAnnotationMutations.ts`, `stickerEditTransforms.ts`,
  `stickerEditPropagation.ts`, `stickerEffects.ts`, `stickerHistory.ts`,
  `stickerSnapshot.ts`, `stickerRasterize*.ts`, and `stickerExport.ts`;
- **performance/lifetime** — `shaderCache.ts`, `stickerGpuWarmPool.ts`,
  `syncImageCache.ts`, `imageSearchCandidateCache.ts`,
  `imageSearchPrefetchGeneration.ts`, `dragFollowerRegistry.ts`,
  `dragTargetIndex.ts`, and `liveEraseQueue.ts`;
- **general support** — `fileNaming.ts`, `appSettings.ts`, `historyModel.ts`,
  `imageSource.ts`, `fontCatalog.ts`, `logger.ts`, and `errorDiagnostics.ts`.

Two image-resolution paths intentionally coexist. Canvas display resolution and
capability-aware workflow resolution have different ordering and fallback rules;
they must not be unified as a cosmetic refactor without behavior tests.

Session image references are acknowledged only after a successful native document
commit. The persistence boundary replaces unchanged source/preview bytes with
managed paths while retaining the original frontend pixels for Loom's full-image
snapshot contract. Workspace replacement, unit removal and stale save responses
cannot restore retired references; a missing managed asset retries once from the
retained pixels at the same expected document revision.

## 4. Native backend architecture

Session save/load commands admit at most two blocking workers; the existing
session mutex and file lease keep a single writer with revision conflict checks
and atomic replacement. Disk I/O, image validation and sync no longer execute on
Tauri's main thread. Image clipboard consumers reuse the validated decode;
file-naming consumers inspect dimensions without a second pixel decode.

The independent tile path uses `TileTerminal.tsx`, `tilePresenter.ts`, bounded
image/Live caches, and the shared `wallGeometry.ts` mapping. Its serial input
controller owns one bounded queue and one endpoint control lease. A separate
500 ms output monitor invalidates late control results after geometry or display
loss; six seconds without renewed authorization clears even static content.
Input feedback is a noninteractive badge, separate from full-screen error state.

Art has a separate `tileSurfaceController.ts` action owner, serial state cache
and bounded resource cache. `TileSurfaceLayers.tsx` composes existing declarative
controls with media using the shared crop/rotation mapping. Stable node IDs and
`surfaceInputDraft.ts` preserve local edits until an outcome read completes.
Loom owns the ephemeral view, event identity, confirmation and execution state;
the terminal never launches Art code. Layout and connection teardown release
only presentation grants, leaving the source and its formal output intact.

### 4.1 Entry and command surface

- `../../src-tauri/src/main.rs` handles process-only CLI modes such as `--version`,
  `--self-check`, smoke helpers, and the emergency watchdog child process.
- `tile_terminal.rs` handles `--tile` / `--tile-output`, owns one mutex per physical
  output and registers only terminal commands. It does not install capture hooks
  or start the ordinary Otter workspace. `tile_outputs.rs` owns stable Windows
  output identities and physical/DPI geometry; `wall_client.rs` and `wall_live.rs`
  keep device credentials and network media outside the frontend.
- `../../src-tauri/src/lib.rs` registers Tauri commands, initializes the runtime,
  installs global input handling, owns overlay/tray transitions, and connects the
  frontend to native services.

`lib.rs` is the main native integration point. New self-contained algorithms
should prefer a dedicated module, but input-hook and window-lifecycle changes
must be made conservatively because event ordering is load-bearing.

### 4.2 Capture modules

- `capture.rs` — region capture command surface and response metadata;
- `screenshot.rs` — Windows Graphics Capture, HDR/scRGB conversion, SDR/GDI
  fallback, and display selection;
- `screenshot/dwm_shared_surface.rs` — protected-window capture fallback. It
  checks `GetWindowDisplayAffinity`, resolves the undocumented
  `DwmGetDxSharedSurface` export dynamically so unsupported Windows builds fail
  closed, opens the returned D3D11 shared texture, copies it to a CPU-readable
  staging texture, validates dimensions/format/size, crops with display metrics
  and DWM extended-frame bounds, and returns RGB pixels. This path is bounded
  to a 512 MiB mapped surface and never falls back to a desktop crop when a
  protected window cannot provide a valid surface.
- `screenshot/protected_region.rs` — mixed region composition. It captures the
  desktop SDR background at the requested dimensions, then overlays the generic
  protected surface intersection at its physical offset. Windows above the
  protected target in `EnumWindows` z-order are converted to clipped occlusion
  rectangles, so their desktop pixels remain on top. A missing protected surface
  is an explicit error rather than a silent desktop-only result.
- `screenshot/wgc_session.rs` — shared D3D11 device, HWND-backed WGC capture,
  crop geometry, and bounded failure accounting;
- `screenshot/wgc_transient.rs` — short-lived SDR/HDR WGC sessions with bounded
  frame waits and unusable-frame rejection;
- `screenshot/wgc_persistent.rs` — opt-in thread-affine persistent WGC session,
  cached-frame validation, timeout fallback, and CPU crop ownership.
- `screenshot/live_capture.rs` — product live-session delivery owner, fresh-frame
  callback slot, JPEG presentation encoding, resize recovery, and source identity;
- `native/live/capture/live_capture_types.rs` and `native/live/capture/live_capture_commands.rs` — bounded
  session/frame ownership and Tauri lifecycle/raw-binary IPC commands;
- `native/live/source/live_source_window.rs`, `native/live/source/live_source_input.rs`, and
  `native/live/source/live_source_recovery.rs` — reversible source-window state, ordered
  same-user, no-upward-integrity window-message input, target identity checks, and a content-free
  atomic watchdog journal;
- `native/live/source/live_source_security.rs` checks the active desktop, session, owned token
  user SID, and UIPI direction. `src/features/live/services/liveCaptureFeedback.ts` exposes bounded,
  fixed-text Unit notices and code-only diagnostics when input is unavailable;
- `capture_coords.rs` — physical/global/logical coordinate normalization;
- `capture_windows.rs` — visible window enumeration, target filtering, and
  top-to-bottom z-order ranks used by protected-region composition;
- `long_capture.rs` — overlap analysis and incremental stitching.

Global Loom connection restart intent is owned by `loom_connection/resume.rs`.
It atomically stores a bounded, secret-free `loom-resume.json` independently of
history. Successful activation and manual disconnect persist under the owner
lock; shutdown retains intent. Startup reuses the authenticated connection worker
and can claim only generation zero, so delayed restoration cannot replace a user
action. Saved history without explicit intent never triggers an automatic request.

Ordinary region capture can return a file-backed PNG plus metadata. HDR output
uses 16-bit BT.2020/PQ when appropriate; SDR output uses 8-bit sRGB. Long capture
remains SDR by design. After region selection, long-capture activation releases
the native selection mouse hook and full-screen input shield before sampling,
so wheel and scrollbar drags reach the target application. Its independent
Escape scope stays active even when the target has focus or backend capture
falls back to frontend sampling; finish, cancellation, and failed activation
clear that scope through the existing input cleanup command.

`long_capture/preview.rs` samples a bounded thumbnail directly from aggregate
segments, without flattening the full image. `native/long_capture/long_capture_preview.rs`
encodes it in memory outside the session lock. `LongCapturePreview.tsx` polls
serially at 500 ms with a merged-frame revision and discards stale responses on
session changes. The click-through preview prefers the region's right side,
then its left; an inset fallback requires successful native capture exclusion.
Preview pixels are never persisted and finish still consumes the same aggregate.
Recording fingerprints retain at most a 320 x 320 RGB validation thumbnail.
If exact motion signatures fail on a changed frame, `native/long_capture/long_capture_repaint.rs`
requires a Good pixel-overlap match before admission; scaled coordinates never
drive the full-resolution stitch. Throttled reason-only diagnostics distinguish
identical frames, near duplicates, verified repaint motion, and unmatched changes.

Single-device live capture is a separate session path rather than a loop added to
`capture_region`. Each session owns one delivery worker, one replaceable callback slot,
and at most three encoded frames. `read_live_capture_frame` returns raw IPC bytes;
the Solid controller owns and revokes short-lived object URLs. JPEG is only the
local WebView presentation transport and is not the `loom.live.v1` network media
codec. Capture/control state never enters `loom.surface.v1` as frame payloads.
The local maximum request is 60 FPS, reduced by `live_gpu/work_budget.rs` using
full-source pixel demand and total requested cadence. Unit demand updates every
250 ms; the shared source owner folds the minimum within its 100 ms control loop.
`screenshot/live_capture_producer.rs` attaches window Units through
`live_shared_source.rs` to one thread-affine owner in `live_shared_source_worker.rs`,
keyed by HWND/process/dimensions. The WGC pool has two buffers. Independent display
capture stays private. A new subscriber refreshes the pool for static initial
pixels, with old callbacks excluded by the source generation and subscriber lock.
Owned ROI copies happen before the source callback returns, without CPU readback.
`live_resources` owns process-wide source reservations before worker/pool creation,
cached OS CPU/RAM/DXGI sampling, upward allocation-growth estimates and hysteresis.
Source geometry is rechecked against its reservation before every pool rebuild.
The hard ceiling is 16; actual admission depends on estimated marginal cost and
runtime headroom. Pressure slows source cadence without deleting graph Units.
See `../features/LIVE_RESOURCE_ADMISSION.md` for telemetry and estimation boundaries.
A capacity-one callback notification wakes the worker;
there is no full-frame sleep after encoding. `live_frame_wic.rs` uses the built-in
Windows JPEG codec with scoped COM ownership, full dimensions, quality 0.82 and
4:4:4 chroma. A one-time diagnostic identifies software-codec fallback. Capture
timestamps precede CPU conversion; encode timestamps indicate completed encoding.
`liveCapturePresentation.ts` pre-decodes JPEGs before URL publication and keeps
one in-flight poll/read/decode chain, subtracting its cost from the frame interval.
Late decoded frames are revoked on stop, and slow consumers still skip old frames.
The automatic route B renderer in `src-tauri/src/live_gpu/` copies a bounded owned
BGRA8 texture before CPU conversion, then submits it on one compositor thread to
per-Unit DXGI swapchains. No WGC frame-pool texture crosses the callback lifetime.
`liveGpuPreview.ts` supplies physical DOM bounds and a renewable visibility lease;
Unit state and input ownership remain unchanged. `live_gpu/snapshot.rs` owns
bounded one-shot staging readback and PNG encoding on blocking workers;
`liveGpuSnapshot.ts` validates binary delivery and `liveCaptureUnit.ts` commits
explicit copy/save/drag snapshots with timestamp ordering and lifecycle guards.
`live_frame_handoff.rs` owns a latest GPU mailbox, a reusable spare and the arrival
clock. Callbacks never Map or encode; contended native submissions retain one
owned texture and transfer it later without overwriting a newer native frame.
Startup/fault/unsupported-layout fallback acquires one process-wide CPU permit
before staging/Map and holds it through JPEG. The permit imposes pixel/rate and
measured wall-time cooldown budgets. Each source relay worker holds an
`EncodedFrameConsumer` lease in `work_budget.rs`, including during reconnect.
The capture registration owns its identity; worker exit or failed spawn releases
the lease without affecting other publishers or a replacement capture. Encoded
demand keeps capture/CPU admission active independently of local visibility and
bypasses only GPU-only JPEG suppression, not the shared resource limits.
Hidden/offscreen consumers without that demand retain source health at 1 FPS
without JPEG production. `liveGpuPreviewScheduler.ts` supplies one
layout lease clock and a lazy shared geometry sample; CPU compatibility and GPU
fault modes continue publishing visibility demand.
The C1 source boundary in `src-tauri/src/live_video/` adds a single-source GPU
subscription and a thread-affine hardware H.264 encoder. The WGC callback copies
immutable BGRA input before returning; only unencoded input uses latest-wins.
`GpuFrameConsumer` keeps acquisition active without granting hidden CPU/JPEG
admission. Reset generations and terminal slot closure protect capture ownership;
encoder failure/cancellation invalidates the entire reference chain. The source
worker requests this subscription only after H.264 negotiation and policy approval;
the viewer decodes access units in order before caching decoded images. Unsupported
hardware or a failed reference chain falls back to the existing JPEG/raw path.
This is not GPU decoding, zero-copy presentation, or a performance acceptance claim.
See [H.264 contracts and verification](../features/LIVE_RELAY_H264_POC.md).
Retained textures restore a static source even without another WGC update.
`liveCaptureHandoff.ts` coalesces transient PNG handoffs and waits for decode/paint
before normal native disable. Capture timestamps order handoffs and in-flight JPEGs;
PNG wins an equal-timestamp tie. These transitions do not commit streaming graph data.
Set `OTTER_LIVE_GPU_PREVIEW=0` to force compatibility. The Windows backend does not
use DWM thumbnails. See `../features/LIVE_GPU_PRESENTATION.md` for composition and acceptance boundaries.
When a live drag is fully contained by one enumerated program window, the frontend
stores the rectangle relative to that window. Rust converts it once to a physical
WGC surface crop; source movement does not alter that crop. The same local region
is translated through the window's current bounds for ordered input, so capture
and interaction remain aligned after the program moves. Session status keeps
logical WebView dimensions for its entire lifetime; physical WGC dimensions live
only in frame descriptors and never resize the local view implicitly.

Logical hide is capability-scoped to a live HWND session. The source remains a
near-transparent, non-activating compositor window so WGC continues receiving
fresh frames; original placement, extended style, topmost state, and layered
attributes are restored exactly. Cached child input targets are accepted only
while their HWND remains in the selected source subtree and the target process
passes the same-session/equal-integrity preflight; this supports legitimate
cross-thread and helper-process controls without relaxing UIPI. If a deepest
child becomes invalid, input falls back to the nearest valid ancestor instead of
disabling the whole session. `SendMessageTimeoutW` provides bounded delivery.
This does not emulate hardware input, secure desktop, or native minimization.

`native/live/source/live_source_button.rs` tracks a bounded push-button gesture for WinForms
controls whose ordinary mouse-up Click is suppressed by occlusion. Only an
explicit, un-dragged release at the same validated, enabled control may complete
the standard parent `BN_CLICKED` notification. Cleanup never runs this action.
Ordinary discrete input delivery allows 250 ms for cold control handlers; move
samples and cleanup releases retain their 50 ms bound. Unknown completion is not
retried. Existing user/session, UIPI and child-HWND checks remain in force.

Static rectangular region drags intentionally use the visible display composition
to preserve every window that overlaps the selection. Live drags are program-first:
a rectangle fully contained by one valid top-level window uses that HWND plus a
fixed window-local crop; a rectangle spanning windows or desktop remains a screen
region. Full-window double click and protected-window static composition keep their
existing paths.

The live protected-window probes are local-only ignored Rust tests. They accept
any visible protected HWND through `HOOK_PROTECTED_WINDOW_HWND`; production code
does not match a process name or title. A Telegram installation is only a useful
local fixture. For example:

```powershell
$handle = (Get-Process Telegram).MainWindowHandle
$env:HOOK_PROTECTED_WINDOW_HWND = ('{0:x}' -f $handle)
cargo test --manifest-path src-tauri/Cargo.toml `
  screenshot::dwm_shared_surface::tests::captures_live_protected_window_from_dwm_shared_surface `
  --lib -- --ignored --nocapture
```

GitHub's ordinary `cargo test` invocation never enables `--ignored`, so hosted
runners do not require Telegram. Geometry, z-order masking, and fractional-DPI
rounding remain covered by deterministic tests that run without external apps.

### 4.3 Desktop lifecycle and input

- `single_instance.rs` prevents two normal Otter instances from running;
- `mouse_monitor.rs` supports overlay hit testing and click-through decisions;
- `emergency_exit.rs` owns the shared Esc+Delete chord state machine: three
  overlapping press cycles, both keys released between cycles, and adjacent
  chords less than 400 ms apart. Native keyboard/rdev trackers remain independent;
- `emergency_watchdog.rs` polls the same state machine in an independent process
  and can terminate the main process after three Esc+Delete chords or the preserved
  `Ctrl+Alt+Shift+F12` backup chord, then restores cursor/input state;
- `app_settings.rs` and `file_naming.rs` own validated settings and atomic visible
  filename allocation.

The overlay input path uses a bounded native event queue. Replaceable move samples
may be coalesced, but button/key edges must remain ordered. During an active
sticker drag, the Windows overlay stream is authoritative; the parallel trusted
WebView stream is ignored to prevent two cursor clocks from alternating the DOM
transform.

### 4.4 Local capability bridges

- `loom_otter.rs` maps the local Art/workflow surface;
- `loom_bridge_client.rs` owns the pinned TLS/exporter authentication for the
  private `loom.local-bridge.v1` discovery descriptor. Its enablement gate precedes
  discovery and networking, including one-shot Art and shader operations. Native
  integration defaults on; `OTTER_ENABLE_LOOM_OTTER=0` disables this local bridge;
- `extension_bridge_native.rs` owns one leased, epoch-fenced extension socket
  for the main WebView. Its IPC permits only protocol negotiation, resource
  authorization and command invocation, with bounded input/output queues;
- `loom_config.rs` and `loom_connector.rs` discover and invoke Loom capabilities;
- `loom_tls.rs` owns optional origin-scoped, certificate-only private CA trust
  for Loom HTTP and media WSS. `network_proxy.rs` partitions shared clients by
  that trust identity and disables redirects for scoped clients; live and wall
  sockets obtain the same Rustls connector without modifying OS trust.
- `talk_connector.rs` invokes the local Talk voice capability;
- `tea_client.rs` submits tickets to the local Tea service and redacts sensitive
  error content;
- `voice/` contains audio capture, session, provider, hotkey, clipboard insertion,
  and client logic.

Package Arts are forwarded to Loom through `loom.hook.v1`. Otter does not maintain
per-Art command executors in the frontend or Rust host.

The extension lifecycle uses `extensionNativeTransport.ts`, not a browser
WebSocket. Native discovery supplies the actual bridge port on every reconnect;
neither the endpoint nor bridge credentials are obtained from JavaScript.
Disposing the App mount releases the native epoch and cancels its reconnect
timer. A stalled or destroyed WebView loses its native polling lease after
30 seconds. Native exit cancels and joins both extension and coordination
listeners, including remote Surface polling. Local read operations have an
absolute deadline so partial frames cannot indefinitely hold cancellation.
Pure browser preview cannot read trusted native discovery or produce TLS exporter
proofs, so its request API fails locally and push registration creates no socket
or retry timer. Art execution emits a local failure without preparing images.
The main WebView's static `connect-src` allows only self/native IPC; boot settings
cannot add a loopback WebSocket origin. Static script/frame policies remain intact.

`extensionOverlayVisibility.ts` owns cached OCR-context overlay presentation.
Each attachment can retain a host-owned `overlayVisible` preference outside its
capability payload. Toggling visibility preserves the result revision and digest;
only a real result update goes through attachment CAS. The preference is persisted
with the unit but is omitted from capability command inputs. A replacement renderer
suppresses its source without changing that source's preference, and source identity
guards continue rejecting translation after a real OCR or image refresh.

Loom OCR blocks may carry additive `rawText` correction evidence,
`lineGeometry`, and CTC-timestep-aligned `characterSpans`/`wordSpans`. Otter
accepts only the named geometry sources, bounds list sizes and coordinates,
requires span text to match the displayed non-translated row, and retains valid
span extents as recognition evidence without replacing the detector row used for
whole-line typography. CTC timesteps are not pixel-level glyph segmentation, so
using their union as the presentation box can invent gaps or shrink a row.
Estimated baselines are accepted only at a safe angle and only when the rotated
text remains inside the screenshot frame. Missing, mismatched, or malformed
geometry follows the axis-aligned line box path, preserving mixed-version
Otter/Loom operation.

## 5. Capture and window targeting

Static capture starts from `Ctrl+1` or the tray; live capture starts from `Ctrl+2`
or its dedicated tray entry. The native target enumerator excludes
desktop/taskbar/hidden/tool windows and returns visible client/window bounds. The
frontend may highlight a hovered target; final capture revalidates the selected
window instead of trusting an old hover rectangle.

Coordinate conversion always distinguishes:

- virtual-desktop physical coordinates;
- monitor-local logical coordinates;
- WebView client coordinates;
- display scale factor and negative monitor origins.

Do not mix these spaces implicitly. High-DPI and secondary-monitor behavior must
be covered whenever capture or overlay input coordinates change.

## 6. Sticker rendering and performance

### 6.1 Whole-sticker drag

At drag start, Otter collects registered follower elements and prepares their GPU
transform layer. During ordinary dragging:

- the freshest native pointer sample writes `translate3d` directly;
- RAF updates snapping, transient drag/link-preview positions and metrics;
- the persisted graph position is written once at release;
- blur, visibility loss, pointer cancel, restart, and watchdog paths clear the
  transient transform and GPU-warm state.

### 6.2 Minified stickers

Window geometry and `minified` metadata are published in one graph-store write.
Live image and annotation layers remain mounted and visible during minify and
restore; the minified view does not switch to a baked bitmap, including when a
late preview arrives. Minification clips the window rather than shrinking the
content coordinate system: saved full-size geometry and crop offsets keep image
layers, flips, contain margins, and annotations aligned.

### 6.3 Annotation interaction

Freehand, line, arrow, rotated shape, mosaic, and blur annotations use geometry-
aware hit tests after a bounds prefilter. Moving an annotation uses an imperative
SVG transform preview and commits the actual annotation coordinates/history once
on release.

## 7. Persistence and files

Otter persists:

- session units, links, groups, recycle bin, and reference library;
- screenshot/color history;
- sticker tool settings;
- global application settings;
- content-addressed session image assets;
- runtime logs and bounded clipboard/cache artifacts.

User-visible filenames are rendered from the shared file-naming model and are
sanitized for Windows while preserving normal Unicode. Collision allocation is
atomic (`name.png`, `name_2.png`, ...). Internal content-addressed assets, capture
transport files, logs, and caches do not use the visible naming templates.

The public bundle identity and canonical automatic data root are
`com.yamiyu.hook`. Tests and isolated launches can override the data root with
`OTTER_APPDATA_DIR` (`HOOK_APPDATA_DIR` remains a fallback); the runtime does not
scan or migrate obsolete identities. See [environment compatibility](../development/RENAMING.md).
Startup grants the WebView asset protocol access to image files directly inside
that effective data root's `images` directory so persisted stickers can render
after restart. The grant does not expose session/settings files or the whole data root.

The active Art boundary enforces the same canonical-only rule:
current schemas, package layouts, and app-data identities are accepted;
obsolete aliases and migration paths are not production inputs.

## 8. Build, test, and release

### 8.1 Frontend output

`npm run build` calls `scripts/build-static.cmd` from the repository root and
produces `.output/public`. Vite's `emptyOutDir` clears previous output before the
build; `scripts/clean-tauri-dist.mjs` then removes `.gz`/`.br` precompressed files
that Tauri does not consume, not arbitrary stale resources.

### 8.2 Portable executable

`scripts/build-local-otter-exe.ps1` performs version validation, runs
`tauri build --no-bundle`, and copies `src-tauri/target/release/otter.exe` to the
requested output directory. `scripts/package-release-zip.ps1` creates the public
portable archive with license and third-party notice files.

### 8.3 CI lanes

- `build-otter-exe.yml` — PR/main/manual verification; product changes build a portable
  candidate, while the classified documentation-only lane does not build an EXE;
- `runtime-performance.yml` — runtime/performance-oriented gate;
- `release-otter-tag.yml` — `vx.y.z` provenance validation, portable zip release,
  and retained unsigned UIAccess signing-candidate evidence; existing `Vx.y.z` tags
  remain compatible, while internal four-part tags are excluded;
- `signpath-signing.yml` — manual reviewed signing flow for the future installer
  phase.

The current user-facing release package is portable. The repository retains the
installer/UIAccess path, but it must fail closed until SignPath and the protected
approval environment are provisioned.

## 9. Maintenance invariants

1. Treat current code, configuration, scripts, workflows, and tests as the source
   of truth; update docs in the same change when behavior changes.
2. Preserve native input edge ordering. Do not trade reliable Down/Up/Escape
   delivery for move-event throughput.
3. Keep persistent graph data separate from transient UI/drag/edit previews.
4. Avoid per-move graph writes, full document queries, synchronous full-frame GPU
   readbacks, and unbounded caches/queues.
5. Session loading is tolerant: unknown fields may be reported, but readable
   sessions should not be rejected solely for schema drift.
6. Never publish an unsigned UIAccess candidate as a signed installer.
7. Keep project and bundled-source licenses in every release archive.
8. Do not add compatibility aliases or persisted-data/package-layout migration
   paths to the production Art boundary.

See [`../../CONTRIBUTING.md`](../../CONTRIBUTING.md) for the change workflow and
[`../README.md`](../README.md) for the maintained documentation index.
## v424 软件帧提交边界

`software_surface::prepare_present` 输出单次消费的不可变 DIB 租约，不调用窗口 API。
窗口 owner 可在释放 Canvas/State 借用后切换样式并提交，避免同步窗口消息的重入借用冲突；
同一 DIB 待提交期间禁止再绘制，resize 新缓冲与保留旧帧均计入现有共享像素预算。
本切片验证 owner 分界，未启用产品 CPU 自动降级，也未完成呈现计数的窗口 owner 接线。

### 原许可服务 dispatcher（当前实现，待集中验证）

- `native/src/art_service_dispatcher.rs` 只拥有从已下载 Prepared 移交的 Client、签名 envelope、绑定及原 Permit/Lease 引用；不新建信任或凭据。
- `art_service_http.rs` 的借用 Authority 统一原 Prepared 与 signed Local 服务路径，网络期间不持授权锁；原 Call 公开行为保持。
- `art_service_signed_tests.rs` 负责验签下载→受限 child→回环TLS的最终null/错误、scope与offline拒绝、HTTP后撤权丢帧；非实际 Loom daemon 联合证明。

### 跨仓实际服务联合门（input-2 定向通过，非完整产品验收）

- Otter art_service_joint_tests.rs 仅在显式环境指定的隔离Loom夹具中运行，使用真实设备挑战/签名续签、原Prepared下载/验签和真实受限child；不模拟服务回复、不写用户配置。
- Loom local_art_joint.rs 持有独立daemon、签名插件和精确包审批，协调Otter测试进程完成调用/null及服务端撤权拒绝；有界等待、仅回收自建进程，失败保留夹具证据。
