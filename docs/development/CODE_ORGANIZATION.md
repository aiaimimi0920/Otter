# Otter 代码组织与开发规范

## 贴图右键输入回归所有权（v545）

procedure只拥有pinned＋HTCAPTION右键松开到原WM_CONTEXTMENU的消息接线；context_menu_input_tests是既有context_menu测试owner下的叶，拥有自有隐藏宿主、subclass观察及其释放，不弹菜单、不代普通前台验收。原菜单快照、模态、授权、会话、拖出与命令执行owner不变；没有新生产模块、依赖、线程、队列或兼容层。

## WGC 与普通 Loom 持续负载测试所有权（v544）

`application_live_art_load_runtime_tests`是既有Live Add Art测试的cfg(test)叶，拥有自建静态白窗、真实WGC会话、普通Loom目录/回环Art和600秒有界持续正式输出断言；恢复child使用原fixture，先登记再进入原Application结算。不增加生产入口、全局输入、线程框架或资源预算。普通Loom/HTTP启动采样脚本仅在本地临时证据目录，不冒充普通Otter/C前台和完整资源长稳；详见[验证边界](WINDOWS_NATIVE_BUILD.md#v544-真实-wgc-与普通-loom-持续-art-负载)。

## Live 连续运行回归所有权（v544）

`composition_live_cycles_runtime_tests` 是既有 `composition_live_session::input_runtime_tests` 的 cfg(test) 叶，仅拥有有界的重复会话／尺寸换代／输入owner重启场景，复用原自建窗口、像素读回、12秒阶段等待和恢复child；不增加生产入口、资源框架或全局输入。两份Rust最大336有效行，新叶184有效行，无新增例外。真实运行的12会话／36次resize／24次控制启停及严格静态通过；只有一个新增通过用例，不代普通前台C、长稳或整库通过，详见[验证边界](WINDOWS_NATIVE_BUILD.md#v544-live-连续生命周期运行门)。

## Live 帧池与捕获会话尺寸换代所有权（v544）

`capture_wgc::live::pool`（`live_wgc_pool.rs`）拥有两帧池、capture session 和 FrameArrived 的共同安装／换代；item Closed、Source/Consumer、单调序号、设备、取消和整体关闭仍归 Session。尺寸变化先归还旧帧、预留原旧＋新峰值预算，再解绑并关闭旧代、建立新代；失败由原 Session 关闭部分资源。不能只 Recreate 原池后等待静止源下一次重绘，也不能只关闭 session 后复用旧池创建新 session。没有新轮询、应用线程、队列、权限、依赖或兼容层。

`live_wgc_pool_runtime_tests` 归 pool 叶，显式运行自有静止窗口三种陈旧池尺寸的真实 WGC／GPU 读回回归；原显隐门只保留有界诊断，不放宽像素、序号、恢复和清理断言。受影响 Rust 为 Session 423、pool 80、新回归 164、原显隐回归 250 有效行，无新增例外。43 项相关 Rust 与新普通 EXE 的 6 项工作区门通过；A/B 人工通过、普通 C 全流程仍未关闭，详见[原因与验证边界](WINDOWS_NATIVE_BUILD.md#v544-静止源尺寸变化后的-live-停帧修复)。

## 宿主 IME 快捷键所有权（v543）

`desktop/node_menu::art_picker`拥有宿主Shift+1的窄IME识别，`key`只读取当前线程GetKeyState，纯`key_state`归一化满足精确键态的WM_IME_CHAR，再复用原chord及dispatch围栏；Live沿原入口复用，不另设按键、协议或输入法框架。标准EDIT继续拥有字符输入，不处理WM_CHAR/VK_PROCESSKEY来猜原键，不加全局输入、依赖、线程或timer。

新增`art_picker_ime_tests`叶归既有Art picker测试owner，纯正向组合、实际隐藏宿主负向及标准EDIT最终字符各有一项，Live原回归补负向/repeat。独立线程探针证明本环境不能用SetKeyboardState伪造数字键正向，旧两轮失败保留，普通新EXE正向快捷键和独立子Art另行取得实证；不把隐藏测试当人工IME通过。4份Rust delta最大223有效行、无新增例外，无生产模块或资源所有权迁移；详见[验证及剩余边界](WINDOWS_NATIVE_BUILD.md#v543-宿主-ime-快捷键与普通独立-art)。

## 文件选择取消结果所有权（v542）

`desktop/parameter_file_input`拥有系统选择与Accepted/Cancelled/Rejected结果，取消不产生路径请求或配置/提交变化；非零系统错误和选择后owner/Stamp/单槽围栏仍归原入口。`sticker_input::actions`传递typed结果，`parameter_panel`只投影错误及实际配置变化，不复制文件读取/提交框架。新增`parameter_file_input_tests`叶归文件选择owner，面板取消/拒绝回归归原sticker测试叶；无新依赖、线程、协议或资源预算。

5份Rust最大406有效行、4项新增回归，63项相关Rust及strict静态通过。普通产品文件框取消已复验无误报；窗口关闭后的工具边界和Job兜底不冒称自然退出，详见[完整验证边界](WINDOWS_NATIVE_BUILD.md#v542-图片文件取消不再误报配置错误)。

## 参数初始绘制所有权（v541）

`parameter_panel_layout::reveal`只在滚动位置实际改变时调用`place`；`IsDialogMessageW`可处理paint，不能在每次已处理消息后无条件重排并失效父窗口。`parameter_panel_options_paint`拥有标准COMBOBOX列表项绘制、DPI高度和subclass寿命，收起区域复用`surface_select_control::paint_standard`的既有像素层；系统仍拥有字符串、索引、键盘和可访问性，不复制Surface或参数业务状态。标签沿原128字符清洗，UTF-16固定257单元缓冲，无新线程、timer、依赖、协议或资源池。

新增真实DIB/聚焦Application消息泵7项回归；本批83项库测试及新普通EXE工作区6项通过。10份受影响Rust最大261有效行，3个新文件，无新增例外。普通前台在首次选窗前被物理Escape停止，视觉/CPU未验，缺严格退出回执仍保留；详见[验证边界](WINDOWS_NATIVE_BUILD.md#v541-参数初始绘制与下拉框修复)。

## Art 候选预览与选择所有权（v540）

desktop/loom_output_batch::candidates独占执行worker内有界准备，image_decode_candidate只负责WIC常见容器/源尺寸/64边缩图，不扩展通用解码或冻结WebP路线。loom_result_state拥有不可变集合和正式完成后的配置Stamp；application_execution只在原正式commit成功后绑定，Local替换和新执行退休旧集合。parameter_panel_model仅投影共享Arc，controls::candidates独占原生按钮绘制/资格，Choice复用Snapshot原参数事务及execution_start显式执行，不复制执行或正式输出框架。候选资格从不可变Snapshot预计算，提交仍核原数值index精确性、owner/配置/集合及dirty/IME；无新增线程/依赖/权限/网络或队列。15份Rust最大390有效行，5新文件，13新回归；完整证据及未验范围见[构建记录](WINDOWS_NATIVE_BUILD.md#v540-art-多候选预览与选择)。


## 当前签名端口显隐所有权（v538）

`art_execution_visibility`验证当前v3签名端口/显隐及共同预算；`art_local_ports_record`只保存包绑定提示，离线授权仍归原完整证明。`art_prepare_package_request`只发送当前路径，`art_prepare_resume`拥有同次grant/完整资源/设备绑定及原缓存事务，不协商或降级。desktop owner把已签默认值接到原Catalog，用户覆盖仍归workspace::ports，隐藏不改变图权限。Loom的source_ports只投影不可变包中当前单端口字段，package/cached/local_art及LAN adapter负责精确Device路由、请求预算和签发前后撤销围栏。没有新依赖、权限、常驻线程或扩大资源池。

已取消全部新旧兼容维护；旧记录仅历史。详见[验证与未验边界](WINDOWS_NATIVE_BUILD.md#v538-当前签名包与端口默认显隐)。

## 普通贴图输入所有权（v537）

`desktop/sticker_input`持不可变共享Config/Asset和不可复活身份；asset叶负责显式文件读取与预算，actions叶负责宿主/配置/owner围栏。`sticker_image_params`独占普通图片参数例外，`image_decode_sticker`独占尺寸准入，不放宽Art或其他解码入口。既有parameter_file_input/job使用typed目标沿原单worker/取消/join，参数窗和capsule只调用统一事务。保存worker组装参数、后台候选解码，UI不复制大URI。

为保持500有效行门，原初始窗口→Application交接函数原顺序迁到既有startup_entry；desktop仍拥有COM和主消息循环，未增业务框架、依赖或常驻线程。本批46项delta、11新文件，最大448有效行，无新增例外。详见[构建记录](WINDOWS_NATIVE_BUILD.md#v537-普通贴图图片输入)。

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
普通前台及原失败边界见[构建记录](WINDOWS_NATIVE_BUILD.md#v532-live-源-add-art-与独立子节点)。

## 工作区 chrome 所有权（v531）

`application::chrome`独占Rc UI状态、弱Binding和单槽手势/提交；model/scene/draw/input/window/strips/integration分别拥有三态标量、Catalog/几何快照、GDI绘制、拖动、穿透线层、每节点最多两owned稀疏端点条及Application接线。不得持Application或文档可解引用裸指针，不复制图像/执行/保存框架，不按每端口建HWND。相同Catalog复用图，宿主消息更新几何；隐藏不删图，拖动只扩线层，不逐move重建region。
`application_links::apply_link_creation`是菜单和chrome唯一提交围栏，重验两端owner/锁/隐藏、目录及Workspace；有效drop才替换旧边，取消保旧边。Live原renderer只新增三态可见性输入，不更改源纹理；端点继承宿主affinity，混合线层采用最严格保护。手势/单槽纳入原恢复/保存/退出/系统会话门，先关chrome再拆宿主；线层/端点各33,554,432像素，失败保文档且错误去重。无新依赖、协议、权限、线程、timer或全局输入。
19项新chrome回归及原Live GPU像素/直接依赖通过，当前选定Rust90 passed/2默认ignored，29份Rust最大500有效行；普通前台/完整端点业务仍未验或未实现，见[构建记录](WINDOWS_NATIVE_BUILD.md#v531-工作区三态与跨窗端口连线)。桌面mod仅增加Binding字段，叶模块均挂在所属owner下，没有新增行数例外。

## Add Art 入口所有权（v530）

`node_menu::art_picker`及model/dialog/template拥有精确Shift+1、源绑定不可变目录、256 UTF-16搜索、真实标准控件和同步模态生命周期；`application_add_art`独占消费单槽意图及普通能力的隐藏子节点/连线事务。`application_local_art_node`保原OpenArt行为，Add Art使用强制创建并在启动前复核专属predicate；`application_propagation`仅接原自动通知链，不能绕过notify/listen/upstream。
16项新测试保护目录index/禁用、来源/权限、真实隐藏控件/IME合成/DPI/捕获排除、同步建窗撤销、同Art独立创建、失败回滚和已落盘系统会话准入；相关回归见[构建记录](WINDOWS_NATIVE_BUILD.md#v530-shift1-原生-art-入口)。没有新依赖、协议、权限、常驻线程或像素预算；前台/人工IME及Live源范围不能从隐藏测试推定。

## 参数面板所有权（v529）

`document_window::parameter_panel`及model/controls/rows/events/keyboard/layout/theme/transaction叶拥有惰性文档专属owned窗、稳定Rc/marker、有界本地草稿与消息导航；`application_parameter_panel`拥有应用四窗预算、调度及捕获列表接线。`capability_parameter_menu::commit`是菜单/面板唯一候选事务；`parameter_value`保原Spec，`parameter_view::after_local_text`仅推进同步未连线文本的既有视图配置，不能用于任意外部/模态变化。
23项新测试覆盖真实隐藏EDIT/COMBOBOX/Slider、16字段批量、dirty→Run/step/Choice、JSON导致索引移位、连线/配置失效、DPI/IME草稿、分页/捕获排除/宿主先毁/预算/托盘与清理；相关回归见[构建记录](WINDOWS_NATIVE_BUILD.md#v529-tab-原生参数编辑窗)。没有新依赖、线程、网络合同或权限；普通前台只是部分证据，端口手势/候选/Live外围不是本批完整实现。

## 工具面板键盘所有权（v528）

`desktop/tool_panel_keyboard.rs`独占Ctrl+E/F4精确识别、bit30重复抑制、同宿主/面板身份和同步重入后的准入；`tool_panel`复用原modeless工具窗及WM_CLOSE/线程焦点回还，`procedure`只接线。Tab、Art入口和三态Unit chrome不被合并到该模块；不改内容/图像/选择、全局前台、权限或持久化。
`tool_panel_keyboard_tests`拥有7项新纯键位/真实隐藏宿主与子按钮/门禁/身份/资源回收回归，本批合计32项定向Rust通过，严格静态和三目标构建通过。普通前台证据边界及清理结果见[构建记录](WINDOWS_NATIVE_BUILD.md#v528-ctrle-工具面板键盘闭环)；不是整体迁移验收。

## 截图/取色历史所有权（v527）

`native/src/capture_history.rs`及image/store叶拥有原版同屏两列表、各24条、兼容JSON与独立原生profile事务；`image_decode_history`只在WIC源先缩到240，不完整分配旧大图。未知根字段与暂态失败截图原条目保留，不读取URL/路径、不自动迁移旧Tauri配置。
`desktop/capture_history_ui`、worker/window及application_capture_history叶分别拥有共享View/有界意图、惰性单后台槽、标准owned窗和主消息泵接线。回调不执行磁盘/建线程；同步Win32外释放借用；Origin用UnitId+Rc隔离旧宿主，队列最多4份共享Image，shutdown先join再关闭wake。`PreparedDocument.record_history`显式区分真实capture、普通打开和粘贴；只有整批成功提交后入栈，复用缩略图不再次入栈。
本批55项Rust定向通过（16新增），三目标编译和严格静态门通过；普通产品的历史owned窗未被工具返回，真实键盘/视觉不冒充通过。原前台Job五秒清理等待失败保留，只读核已无自有产品进程；不是严格/正常清理证明。代码/验证与失败证据见[构建记录](WINDOWS_NATIVE_BUILD.md#v527-原版-ctrlh-截图与取色历史)，无新依赖/网络协议/授权。

## JS 测试前台所有权（v526）

原 `surface_javascript::fixture`（`surface_javascript_fixture.rs`，仅 cfg(test)）继续独占 READY→G 启动前提和 Foreground guard；不新增产品入口或共享业务层。focus 先核目标 PID，guard 只保留及恢复当前测试进程 HWND，移除跨线程 AttachThreadInput 兜底，真实前台断言不变。跨进程自有 UIA target 仍只登记恢复 owner，不代目标发起激活。
本轮新增 3 项所有权回归、既有 6 项 Application/2 项 guard 共 11 项 Rust 通过；临时 WinForms sponsor 在 linshi，逐次按钮后才建 child，限定 PID 授权，不是普通产品键盘或人工 IME 证明。没有产品结构/依赖/权限/预算变化；详见[构建记录](WINDOWS_NATIVE_BUILD.md)。

## Tile 呈现证据所有权（v525）

`desktop/composition_present_status` 独占每交换链最多16次原始 Present 状态变化的只读日志，D3D11/D3D12 保留原 HRESULT 和仅 `S_OK` 确认规则。
`tile_shared::diagnostics`（`tile_presentation_trace`）独占原可见性判断的固定分类及每 Shared owner 每阶段一次的原子去重；窗口、confirm 和 presenter 仅接线，释放 Shared 锁后才写日志。
不记录设备/Key/lease/凭据/图片，不增加线程、timer、网络协议、重试或授权；`HeartbeatPrepared*` 仅表示本地正文候选，不证明发送或严格 ACK。
新专属测试保护限额、保守场景时钟、原授权/媒体边界与迟到 paint，不把 prepared 改判为 presented。本次普通 Release 在未切换独立 desktop 实际返回 `DXGI_STATUS_OCCLUDED`，帧停在 `FramePaintUnconfirmed`，原 15 秒业务门仍失败；不解释为具体用户窗口遮挡，不反推旧 v524 根因，不证明普通可见桌面通过。实际运行结果集中记录在[构建记录](WINDOWS_NATIVE_BUILD.md)。

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

## 本地Art自动触发原生所有权（v520，限定联合通过）

`art_live_trigger`及ticket/queue独占严格事件、独立不可逆自动授权与单槽；HTTP叶复用原Device通道，driver只在既有Live控制worker串行Configure/Claim/ACK并共享actor游标。窗口local_trigger叶复用原系统菜单/shape_dialog，模态返回核同HWND与完整宿主；脚本不能自行注册或生成授权。媒体/控制恢复共同暂停准入，重连不复活旧gate。
`surface_javascript_signed`沿原串行child派发私有onLiveTrigger，不造click/gesture、焦点或恢复暂停timer。desktop/art_local_trigger只调度已有FIFO/Runtime，原Results所有可失败准备后再核取消围栏；Applied只能在实际原子端口事务后产生。formalRevision是本地执行实例的成功正式提交序号，不是磁盘documentRevision。Stop与无Win32/网络/脚本回调的canonical事务共用短授权锁，不跨同步UI重入；没有新线程、timer、持久队列、权限或预算。
公开SDK由Loom的sdk/otter-art/native-live-trigger.d.ts及同份live-trigger.js声明和演示；专属wire/queue/ticket、真实child、隐藏Application/Save及真实Loom LAN/WSS联合夹具由MAIN拥有。shape_dialog_tests独占原通用输入对话框测试体，保持原测试身份；自动授权控件由独立隐藏窗口测试保护，不运行旧可见模态门。input4选定87项通过，真实HTTPS/WSS父门及原Results/Save/Stop接线通过，7默认ignored边界见WINDOWS_NATIVE_BUILD和本批回执；不以编译代验收，不关闭旧键盘/前台问题、普通可见产品、跨机、长稳及最终包。三旧副线/v464固定包保持冻结。

## 本地Art Live触发服务所有权（v519）

local_art_live_trigger协议及Reply叶独占版本化Device-only自动回调合同；local_art/live_trigger只接原签名宿主与当前审批，runtime/live_session_local_trigger及operations独占登记、单槽、Claim/ACK和退休。Surface与本地目标共用live_session_trigger_evaluation唯一条件状态转移，合计沿原64注册门；不伪造Surface附件、不改grant四权限。
宿主显式automatic_local_callback授权绑定完整Art实例、工作区/代际/包、Live会话/epoch、条件revision及独立authorizationId；Preview不等自动授权。Claim在HTTP响应前消费payload，重复/未知领取不重放；ACK仅报告宿主结果，不证明canonical保存或接收端呈现。本地delivery至多16KiB，超限明确停用；原观察新鲜度30秒、grant期限、墙钟高水位/每事件期限与单调期限共同围栏，查询不续期。停止、最后Viewer断连、审批修订变化、过期和Session close回收或拒绝执行，重连/重批不复活旧授权。没有新增线程、timer、持久队列、依赖或脚本权限。
input2选定相关Rust完整名按crate去重75 passed、1默认ignored，非整库；protocol39/daemon36，其中新增协议3项、服务12项。原Art观看/控制、Surface触发/观察、媒体断连清理与LAN边界回归通过，普通daemon构建及--version/--help通过。HTTP测试用owner直接登记Viewer在线，不是真实WSS或Otter联合。20份Rust最大457有效行；官方格式、版本、Otter全仓strict行数、两仓diff及精确UTF-8无BOM/空白门通过。
本批只关闭Loom本地Art Live触发服务实现及HTTP定向验证；Otter实际配置菜单、独立自动授权owner、onLiveTrigger私有command/SDK、签名child执行、正式结果/取消接线和真实HTTPS/WSS联合仍由MAIN继续完成，不能以此宣布本地Art触发产品完成。v518全局前台身份失败、v517可见混合门、v511历史键盘根因仍开放，不恢复前台自动化。人工IME候选窗/旋转活动EDIT、物理多屏/跨机/DPI热插拔、长稳、其余迁移退出条件、最终隔离整包与用户全面手测仍开放。三旧副线、v464固定手测包及真实profile/设备保持；旧Web/Tauri混用不重列必做。 见[构建记录](WINDOWS_NATIVE_BUILD.md)。

## Tile键盘取消与联合夹具所有权（v518，部分验证）

Tile Popup明确区分Select，真实打开的下拉列表先消费Esc并抑制同次长按；keyboard_window以Rc身份围住取消/同步销毁，只在同一已paint快照保留Tab位置，迟到epoch不得取消后继owner；外部失焦仍完整退休，父Tile拒同次Esc长按连带关闭，新鲜Esc仍退出。取消退休未发Edit后缀，不重放未知写或丢已接受本地值。
keyboard_joint_access及keyboard_fixture_tests仅cfg(test)，Tab后才计按钮受理，避免字段change冒充Enter成功；测试线程键表完整RAII恢复，生产Ctrl+Enter拒绝不放宽，并记录实际焦点/paint/修饰键。专属Combo实验只在未切换desktop及新测试线程运行，不申请SWITCHDESKTOP；只禁用该线程真实IME，人工组合消息不等人工候选窗验收。
input4当前264项相关Rust完整名去重passed、1 failed、5 ignored，非整库；Tile窗口93项实跑92 passed/1 failed，8项取消及3项夹具新增回归全过。8条真实Loom LAN父门通过，父子不混加：两种文本链在input4重跑62.406/69.218秒，六个非文本门按精确三份cfg(test)差异/SHA复用。普通固定SHA产品工作区5项通过证据复用，前台退出门未运行；product/workspace未因夹具变化重编。Otter strict Clippy/lib、官方格式/版本/全仓strict行数、diff及精确UTF-8无BOM/空白通过，9份Rust最大461有效行。
当前唯一终态失败为tab_edit_to_button_commits_local_draft_and_enter_repeat_does_not_duplicate的全局GetForegroundWindow前后身份断言，尚未确认是用户切窗、产品行为还是其他来源，不用历史通过覆盖、不反复重跑、不恢复前台自动化。故本批仅PARTIAL，不发布INTEGRATION_VALIDATED或完整键盘/整项迁移完成。v517混合可见门、v511历史根因、本地Art Live触发独立授权/执行/回执合同、人工IME/旋转活动EDIT、物理多屏/跨机/DPI热插拔、长稳、最终完整隔离包和用户全面手测仍开放；旧Web/Tauri混用不重列必做，三旧副线/v464固定包及真实profile设备保持。 见[构建记录](WINDOWS_NATIVE_BUILD.md)。

## Tile实际提交与普通产品可见门所有权（v516）

presentation_readback_test独占cfg(test)有界staging/Map/预算与读回像素；composition_d3d12_readback_test读成功Present后的上一物理buffer，composition_readback_test读同一D3D11双缓冲flip-sequential链的已提交逻辑buffer1。software_surface/render_software/render_surface仅有cfg(test)委托。tile_presented_pixel_tests复用产品Renderer，独立矩阵核图片/Art/旋转/crop/透明叠层/换帧/黑场恢复。唯一生产结构变化为Renderer::new复用私有from_surface初始化，测试建链前选定后端，不在同HWND叠建两条GPU链。
native/tests/tile_product_runtime与support/tile_product_window独占普通固定SHA EXE、真实CLI/物理输出、隔离profile及自启PID/精确类名的窗口验证；Loom tile_product_native_joint只负责真实生产库LAN HTTPS/签名配对与续签、layout/presentation及权威ACK。四象限独立RGBA常量对应六阶段running/旋转/黑场/恢复/crop/冻结，外部仅采四个前后均确认无遮挡的桌面点，随后正常关闭和endpoint offline。桌面合成像素不冒充产品交换链直接读回，库内各后端读回也不冒充普通产品可见运行；父子不混加。子产品由Kill-on-close Job持有，失败/超时只清理自己的进程。初始可见门因遮挡明确失败后，按用户授权由Computer Use仅激活驱动的普通自有启动器一次，驱动核实际前台后向已核PID/类的同测试子Tile转交一次。不以该显式激活声称产品自动置顶或全程无激活；不移动鼠标、不注入用户应用、不改物理显示或真实数据。没有新产品诊断开关、依赖、路由、权限、线程或资源预算，旧三副线及v464固定包冻结。
本批最终选定回执136项相关Rust完整名去重passed、2默认ignored，非整库；早期失败另行保留。三后端实际提交目标和自有窗口负向门四项新回归通过。4条真实Loom LAN父调用通过：新普通EXE单屏可见链，以及原图片/Live/Art三链回归；普通固定SHA EXE工作区5项通过，前台退出门未运行。17份Rust最大489有效行；Otter strict Clippy/格式/版本/全仓strict行数、两仓diff和精确UTF-8无BOM/空白门通过。 见[构建记录](WINDOWS_NATIVE_BUILD.md)。

## Tile滚动条所有权（v515）

tile_surface_scrollbar独占已paint滚动条身份和捕获坐标；desktop/tile_surface_scrollbar_window独占单槽Thumb/Page手势、Rc token、捕获及timer回收。复用surface_scroll的Bar/Drag、唯一surface_scroll_repeat allocator/Repeat、原Presentation COW计划与tile_surface_scroll_window安装/实际paint，不复制Surface业务。Repeat登记提升至desktop，普通Surface保同名alias；tile_window_test_access只承接cfg(test)窗口入口。
滑块可拖出原track/窗口，轨道立即翻页并在400ms后每50ms至多一步，不追赶积压；thumb到指针、移出track或反向时暂停，回到原方向再继续。活动EDIT由滚动条提交一次，保中文emoji草稿及无handler表单；IME组合拒绝接管。prepare空隙只被动保留且不滚动，空隙抬起释放、不采用迟到位置。同步paint前移出owner，取消轮换token，旧回调不得恢复旧手势或覆盖successor；隐藏/冻结/黑场/过期/撤权/DPI/关闭/丢capture及wheel/Tab接管都清捕获和timer。初始资格仍核topmost/crop/旋转/clip/布局/view/代际，捕获后屏外坐标不授新命中资格。不改scene/canonical、网络动作、权限、原队列/像素/TTL预算；无新hook、线程、依赖、WebView或用户应用输入注入。
当前245项相关Rust完整名去重passed、3 ignored、0当前运行终态failed，非整库；12项新回归覆盖四向旋转/crop/非等比映射、COW计划、真实EDIT提交、timer逻辑时钟、prepare/生命周期与同步paint取消/接管/销毁。普通Surface原轨道重复回归同批通过。11条真实Loom LAN父调用通过，父子不混加；新drag/page各有handler与无handler四独立会话，独立常量核唯一input/change/click、实际framework/Art正式结果、已paint及view/线程清理；原十条输入联合重验。固定SHA普通EXE工作区5项通过，前台退出门未运行。 边界及证据见[构建记录](WINDOWS_NATIVE_BUILD.md)。

## Tile滚动、屏外焦点与派发几何（v514）

surface_scroll及surface_presentation_scroll_state独占COW滚动快照、未滚动布局SHA与受crop约束的显露；desktop/surface_scroll_input只共享原系统滚轮解码。tile_surface_scroll_render/window复用原Presentation与唯一HWND，先准备、提交/隐藏活动EDIT并退休旧手势，再安装和实际paint；同步Win32调用前释放借用。tile_surface_focus/text/local按未滚动布局保草稿，不借屏外保留身份激活输入。tile_surface_actions在原预算中携带实际激活时的scroll，原唯一worker新鲜布局恢复该几何后再校验。tile_surface_scroll_memory只保留至多四个同key/view/代际/尺寸的COW偏移跨越prepare空隙，不保留旧Art/scene/像素或呈现资格；减权/过期/身份变化清理。
滚轮复用系统行/字符/整页偏好、Shift水平与Ctrl让出，嵌套祖先只消费一次；Tab/反向Tab能显露屏外字段，仍受crop、四向旋转、最上层命中和有界搜索约束。重投影重装嵌入文本/Select/Slider绘制缓存，保中文emoji草稿和无handler本地值；同view/代际/尺寸新snapshot继承并钳位scroll，换代重置。IME组合不提交或换焦；隐藏/冻结/过期/准备空隙拒新滚动，旧按压不重新命中新位置。不改权威scene/canonical、表单字段范围、协议、权限、队列/像素预算、原1500ms/120秒门或未知写不重放规则；无WebView、新依赖、hook、线程或输入注入。
当前232项相关Rust完整名去重passed、3 ignored、0当前运行终态failed，非整库。15项新回归覆盖COW、crop/hidden轴、滚动后嵌入缓存、新snapshot钳位、实际EDIT复用/表单、IME、旧press、无handler值、减权、四向旋转与隔离TLS恰一次POST，以及真实prepare空隙后的显露焦点/草稿/待发表单、纯偏移清理与身份隔离。10条真实Loom LAN父调用通过，父/子不混加；新滚动鼠标/键盘各覆盖有handler和无handler两独立会话，独立常量核中文emoji输入记录、唯一change/表单、实际framework/Art正式结果、已paint和view/线程清理。旧文本/IME、离散值、Slider、本地值两种入口重验；普通固定SHA EXE工作区5项通过，前台退出门未运行。 证据与未验边界见[构建记录](WINDOWS_NATIVE_BUILD.md)。

## Tile待派发意图与写前围栏（v513）

tile_surface_action_pending独占原唯一worker栈上Command的弱Ticket预留、固定字节计费与退休；tile_surface_actions/edits只沿原队列使用，不复制命令或新增worker。tile_surface_owner_actions负责真实已paint的Art身份就绪及短暂prepare空隙中的原意图恢复，tile_surface_action_dispatch负责昂贵准备后的写前生命周期围栏。HTTP提交后没有恢复或重放路径。
clear/suspend/discard、新failure、代际或Edit取消使旧预留永久失效；撤销身份不提前归还栈上owner仍占用的预算。原32槽/8MiB、1500ms意图、120秒前缀及HTTP期限不变。两份专属测试叶覆盖11项预算/顺序/真实TLS回归；144项相关Rust去重及9条真实LAN父调用通过，非整库。v511历史根因仍开放；完整证据和产品未验边界见[构建记录](WINDOWS_NATIVE_BUILD.md)。

## Tile本地值与权威同步（v512）

tile_surface_local独占无handler的Switch/Select/Slider本地值与源值SHA stamp，desktop/tile_surface_local_window独占UI线程cache、paint身份校验和生命周期，tile_surface_local_render只投影到原Presentation绘制缓存；不改权威scene或持有Art像素。复用tile_surface_focus、既有鼠标/键盘控件及原唯一Art串行worker，不新增输入框架。
无handler值可鼠标/键盘编辑；同合同同源换快照保本地值，新权威源值替换，view/代际/合同/布局/几何变化退休。准备空隙仅被动保留且拒新输入，隐藏/冻结/黑场/停止/撤权/过期/DPI或窗口换代清理；Select取消不变值，Slider取消恢复此前接受值。与文本共享每view64/总256字段，不加倍预算；本地操作不offer空动作。surfaceValues仍只含input/textarea/number字符串，不加入Switch/Select/Slider，保原16字段/48KiB表单门、祖先click、队列/权限/资源预算。
v511的键盘/Tab接线及当前回归虽已通过，input6的values LAN派发超时仍缺根因证据；后续成功不等生产修复，不发布该批INTEGRATION_VALIDATED，也不把旧失败改写为通过。 当前证据与边界见WINDOWS_NATIVE_BUILD的v512记录。

## Tile文本编辑、Group与生命周期（v510）

tile_surface_text独占真实已paint字段身份/合同和源→crop→Wall/输出旋转几何；desktop/tile_surface_text_owner/window/popup独占有界EDIT会话、IME、草稿/ticket、表单装饰与HWND回收。surface_edit_session只复用unbound本地编辑/编码入口，真实Surface仍要求原Binding，不能互相绕过权限。tile_surface_edit独占跨字段Group与原worker栈上Dispatch清理，tile_surface_edits沿原32槽动作队列串行化前缀，不新增动作worker。
文本、textarea、number以真实EDIT通知生成input/change；IME组合抑制中间态，textarea归一CRLF，number拒无效编辑并支持step。无handler字段只保本地draft，不伪造change；实际按钮沿原16字段/48KiB表单门读取当前草稿，已移除或合同不匹配字段不复活。连续input只合并同Edit尾项，跨字段及后续按钮保顺序；在途/ACK等待保固定期限，终态只授一次原1500ms就绪派发窗口。原64个120秒跟踪/事件64KiB/32槽8MiB不扩大，重复GET不续期，未知写不重放。
新权威未paint时不按旧scene结算ticket；准备空隙只被动保留同租约/view/代际草稿。新paint后完整复核合同/几何，失败、取消、隐藏、冻结、停止、撤权、DPI/窗口换代清理编辑组和后缀。每view最多64/总256字段；owned popup marker/单调epoch拒旧HWND消息，Win32同步调用前移出owner避免RefCell重入，活动EDIT直立显示，非活动草稿复用原唯一D2D/crop/旋转/clip绘制。没有WebView、新线程、hook、依赖或权限。
本批关闭Tile文本/textarea/number、IME合成边沿及文本草稿表单按钮的隔离闭环，不等完整键盘/Tab导航、已安装输入法候选窗人工验收、旋转活动EDIT视觉、全部值控件本地draft/完整表单。inputCapabilities仍为pointer。普通Tile输出EXE可见链、最终交换链读回、物理多屏/跨机/热插拔/真实DPI、本地Art Live触发合同、长稳、其余迁移退出条件、最终完整隔离包和用户全面手测仍开放；旧Web/Tauri混用互通已按用户确认排除。 验证、失败及复用边界见WINDOWS_NATIVE_BUILD的v510记录。

## Tile Slider 单手势与等待所有权（v509）

tile_surface_slider 独占 canonical Range/编辑合同及单手势身份；tile_surface_edits 独占原队列尾 input 合并、同 Edit 后缀退休及前缀就绪接缝。desktop/tile_surface_slider_window 独占原 HWND 的鼠标捕获与释放、抬起提交、单槽异步预览和取消；surface_paint_controls 只准备嵌入控件绘制缓存/paint-only 预览，不改 scene 或普通 Surface 子 HWND 权限。
原 Art 唯一串行 worker 先 GET 新鲜 state 再至多一次 POST；32槽/8MiB、64个120秒跟踪、事件64KiB不变。Slider 同手势依赖仅在固定在途/ACK跟踪期限内等待，前缀终态 GET 只授一次原1500ms派发准入，重复GET不续期；失败、拒绝、取消、合同/窗口换代及等待到期都退休后缀，不重放未知写。新快照撤旧呈现、Art准备完成及候选发布但未paint的过渡只被动保留同租约/同view/代际手势，不发送空白期输入；实际paint后完整复核合同/几何，准备期抬起不提交。没有扩大网络/业务超时、鉴权或资源预算。
专属模型/逻辑时钟/真实HWND及Loom LAN联合保护此链，详细输入、失败、实际成绩和静态旧阻塞见 WINDOWS_NATIVE_BUILD 的 v509 记录。只覆盖声明了 input/change 的 Slider 鼠标交互；无 handler 仍只读，文本/IME/完整键盘与完整表单仍开放，inputCapabilities 保持 pointer。普通 Tile 输出 EXE 可见链、最终交换链读回、物理多屏/跨机/热插拔/真实 DPI、本地 Art Live 触发合同、长稳和最终整包/用户全面手测仍开放；旧 Web/Tauri 混用互通已按用户确认排除，不再计为本轮待办。

## Tile 离散值职责（v508）

tile_surface_value 独占已呈现字段命中、Switch 布尔翻转、Select 有序索引和值合同及新鲜派发校验；surface_input_event 只共享 wire 编码，不伪造普通 Surface 挂载资格。desktop/tile_surface_value_window 独占原生菜单 RAII、取消和模态重入复核，原 Tile pressed 生命周期负责退休手势。沿实际 paint 的 view/generation/snapshot/placement/node 身份、crop/旋转/clip/最上层命中和原 Art 唯一串行 worker，不新增线程、hook、依赖、权限、像素面或预算。
专属模型、实际离屏 HWND/菜单和真实 LAN/framework/Art 联合测试保护此链；Loom只有专属cfg(test)叶与外置测试handler变化。只覆盖声明了 change 动作的离散值；无 handler 仍只读，没有新增本地 draft，不广告文本、IME、连续 Slider 或完整键盘导航，inputCapabilities 仍为 pointer。普通 Tile 输出 EXE 可见链、最终交换链像素读回、物理多屏/跨机/热插拔/真实 DPI、本地 Art 触发、长稳及最终整包/用户全面手测仍开放。 验证/失败/复用及静态旧阻塞集中见 WINDOWS_NATIVE_BUILD 的 v508 记录。

## v507 三类动态Tile联合职责

tile_mixed_joint_tests/tile_mixed_native_joint专属cfg(test)叶复用原真实Source系统菜单、签名配对/续签、Loom LAN HTTPS/WSS、Tile Runtime和唯一原生呈现。静态图片通过新资源/layout、Live通过原Source受预算纹理、Art通过原snapshot/资源分别换像素；独立手写PBGRA核三路首帧和变化后同一已paint帧，不以帧号增长冒充内容变化。冻结保实际Frame Arc并停Tile媒体但保Source，黑场/恢复沿原链；停止/撤权核view/像素/Shared/线程/媒体permit清理和源附件/权威值7保留。
7份cfg(test) Rust最大421有效行，生产协议/权限/预算/依赖和后端选择不变。本批相关Rust完整名去重70 passed/0 ignored，无终态失败，不是整库；26份input1实际定向回执经input2仅两专属新叶delta/测试体与生产未变/SHA精确复用。八条真实LAN联合在input2实跑：三类动态共同更新后停止8.688秒、变化后撤权9.032秒、三类冻结/黑场/恢复/停止16.563秒，分别为三个独立完整Source会话。另两旧Live、两旧Art及原点击/确认/取消联合重验通过。普通Tile输出EXE可见链/最终交换链读回、物理多屏/跨机/热插拔/真实DPI、完整Art值控件/IME/键盘、本地Art触发、长稳、其余迁移退出条件、最终完整包和用户全面手测仍开放。自有离屏NOACTIVATE HWND actual present不是上述验收。静态/失败/复用边界见WINDOWS_NATIVE_BUILD，三旧副线与v464固定包冻结。

## v506 Tile Art 点击与显式确认职责

tile_surface_hit/click只将实际已paint帧的最上层interactive placement映射到canonical Art，保crop/四向旋转/clip与按钮down/up身份；共用原纯wire编码，不伪造普通Surface挂载资格。tile_surface_actions独占32槽/8MiB动作和64个120秒确认锚点，UI只排意图，原Art唯一串行worker先GET新鲜state再核view/generation/节点/命中/期限，至多一次Device/nonce POST，不重放未知写。
原生左键点击声明按钮/点击动作；右键菜单显式审阅批准/拒绝或取消已运行可取消请求，不自动弹窗或自动批准。模态返回复核完整提示、来源、已呈现布局及当前授权；短暂识别/冻结/黑场只减权，未发Event/Decision丢弃、有界确认锚点保留，真实布局/view/generation换代/停止/撤权仍清。当前只广告pointer，不广告值输入/键盘。
相关Rust完整名去重175 passed / 0终态failed / 1 ignored，不是整库；25组定向在input9实际通过后，input10仅两份Loom专属联合夹具/外置handler变化，按完整delta、测试体/生产未变及产物SHA复用原回执，不伪称新库整套重跑。11条Tile真实LAN联合和普通固定SHA EXE工作区5项均在input10实际运行。43份Rust最大458有效行，证据/失败与静态旧债见WINDOWS_NATIVE_BUILD。完整Art值控件/IME/键盘、本地Art触发、真实Loom图片/Live/Art三类动态共同联合、普通Tile输出EXE可见链/最终交换链读回、物理多屏/跨机/热插拔、资源长稳、其余迁移退出条件、最终整包和用户全面手测仍开放。三旧副线和v464固定手测包保持冻结。

## v505 Tile 物理输出识别职责

tile_identification 独占 endpoint/lease/request 身份、保守单调期限、稳定 Marker Arc 和单槽 applied/dismissed 报告；重复观察不延期，过期/关闭/清理保最后身份防迟到复活。tile_presenter 沿原唯一控制线程取一次报告、Device/nonce HTTPS 发送并维持识别期间 null layout application；普通呈现须新鲜控制/心跳周期才恢复。tile_window 只在 HWND 可见且原 RenderSurface 成功提交后确认，输入绑定实际显示 Arc，旧画面关闭/Escape 不得取消未绘制 successor；隐藏 paint、DPI/显示重入、撤权/停止均不回挂旧资源。
tile_identification_render 在原唯一 target 内用 DirectWrite 绘名称/设备与输出身份/尺寸/倒计时/关闭按钮，不加线程、timer、像素面、依赖或权限；原图片/Live/Art worker 与预算不变。专属模型/TLS/WIC独立像素/自有离屏HWND及真实Loom LAN联合保护生命周期，Loom只精确开放既有 identify/report Device POST，不开放管理员 identify。
input3相关166项Rust完整名去重通过/1前台退出ignored，含两新增识别联合、原八Tile联合和普通EXE工作区5项；23份Rust最大451有效行。Art输入能力仍空，普通Tile输出EXE、最终交换链读回、物理多屏跨机/热插拔、长稳和最终包仍开放；失败、耗时与Loom旧静态阻塞见WINDOWS_NATIVE_BUILD。以下旧批次保留当时范围，三旧副线/固定手测包不改。

## v504 Tile 慢 Art 与 Live 隔离职责

tile_surface_worker 独占每个在途 Tile job 至多一个惰性 Art 串行线程、COM 及 Views；没有 Art 不建线程。Runtime/输出 owner 沿原16租约总门申请同 Grant 独立第三 Lease，控制、图片/媒体、Art 不并行复用取消槽或复制凭据。原 Assets 继续消费最多四路 Live，Art 只有一个稳定 Prepared Arc 槽，无请求/采样队列。Shared 发布同时核 job 和准备身份，Art 换版先清旧准备，paint 迟到也不能确认另一 Art 场景；退出先撤准备再关闭可信 view，冻结等待不持 Shared 锁，旧 job 实际排空才释放容量。
当前 input2 147项相关 Rust 去重通过/1前台门ignored，含新TLS/WSS挂住Art图片超过5秒但原Live连接持续Ping、三类内容随后完整呈现、在途停止/撤权、六项准备/生命周期模型、原八Tile真实LAN与两Source/Viewer联合及普通EXE工作区5项。17份Rust最大449有效行，构建/失败停点/精确复用见WINDOWS_NATIVE_BUILD。完整场景仍须等Art新资源，不把“Live持续接收”说成缺资源时仍能连续显示。无新依赖/权限/预算或WebView；真实Loom三类动态共同联合、普通Tile输出EXE/最终交换链、物理多屏跨机/长稳/最终包仍开放。以下v503慢HTTP同线程限制是旧输入边界。

## v503 Tile 只读 Art 与 view 生命周期职责

tile_surface_wire/snapshot/http/owner 独占精确 Wall binding、实例/附件/generation/sequence 与单调 snapshot/preview/formal revision，同版内容不可变，snapshot:null 保稳定 Arc。只接受声明的哈希图片资源，不执行终端 JS 或借普通 Surface 附件权限；最多4个 Art 来源、16个 Art placements、512累计节点、16张唯一图片，沿原混合16M像素及共享像素预算。tile_assets_live 复用原唯一 Assets 线程，每步至多下载一张资源；UI 独占 Presentation/COM，稳定 Arc 缓存布局，Art 直接混入原 Tile 唯一帧并复用 crop/四向旋转/clip/z 序，不加 WebView、子 HWND 或第二截图框架。
surface_embedded_control 只读绘制 Input/Textarea/Number/Select/Slider 的声明值，复用原字体、归一、canonical Insets、圆角与 opacity，不授 draft/输入权限。未知 open 停止且不自动重放；冻结保实际 paint 场景，迟到 state 不替换，迟到新 open 仅关闭已验证 view；新 snapshot 先清旧帧，关闭/撤权回收 view 但保来源实例。公共 Client 仅精确 Device POST 接原10/24MiB响应门，其余仍8MiB；art_service_failure 补原 Transport 暂态 Promise 分类，不重放 submit、不切离线或改授权。
最终 input6 相关696项 Rust 完整名去重通过、7 ignored，不是整库；含两条真实 Art LAN 联合、旧六 Tile 联合、公共 Client/Surface 绘制、实际 Art Application、Source/Viewer 与普通 EXE 工作区回归。36份 Rust 最大495有效行；失败、修复、SHA 与耗时见 WINDOWS_NATIVE_BUILD 和 main-tile-art-1。当前广告 image/raw_bgra/surface_v1/scheduledPresentation，输入能力仍空。单张 Art 同步 HTTP 最多8秒，可能影响同线程 Live 原5秒期限；慢网隔离、三类来源动态共同联合、Art 输入/确认/取消、识别、普通 Tile 输出 EXE、最终交换链读回、物理多屏/跨机/热插拔、长稳与最终包仍开放。以下旧条目保留当时范围，三旧副线和 v464 固定手测包不改。

## v502 Tile Live 媒体职责

media_websocket只共享原TCP/TLS/upgrade/interrupt，不包含普通Live或Wall业务Profile；普通NLLV64与Wall NLWM80严格分开。tile_live_endpoint/authorization拥有精确Wall请求、原Device/nonce和同Grant Lease围栏；tile_live_packet直接编译复用既有Wall包校验器，raw与PNG分别复用唯一受预算BGRA→PBGRA及Tile WIC路径，不复制业务或伪造Viewer成员。
tile_assets_live在原Assets唯一线程准备图片与最多四个Live连接；tile_live_socket独占socket/队列/编码租约，原512MiB池计费接收缓冲与返回payload并存。三pending加一current、80ms软件延迟、250ms迟到丢弃/1000ms旧帧失效；完整来源就绪才发布。Frame持不可延长的Live期限，Shared分别持最新候选与实际paint快照，冻结只能保留后者；停止/黑场/时钟或布局换代/撤权先退休旧连接，迟到结果不确认。无新网络线程、全局hook、依赖或权限。
专属packet/queue与真实Source→Loom LAN→混合Tile联合保护独立像素、raw动态新帧、PNG首帧、实际present及冻结/黑场/恢复/停止/撤权清理；原Source/Viewer共同运输/输入释放/H264回归重验。192项Rust完整名去重通过/1前台门ignored，37份Rust最大478有效行；输入、编译失败、test-only修复和Loom静态旧债见WINDOWS_NATIVE_BUILD。当前广告image/raw_bgra/scheduledPresentation，不广告Art/输入/识别。普通输出EXE/最终交换链读回/物理多屏或跨机/长稳/最终包仍开放，三旧副线与固定手测包不改。

## v501 Tile 输出恢复职责

desktop/tile_output_owner 独占一个稳定输出的 observed/active/retiring 代际；只接受原ID唯一且有界的枚举结果，先隐藏/销毁旧窗口、停止并实际排空旧Runtime，再核原Grant并申请新Lease。缺失/枚举失败不接管另一屏，用户关闭/停止及授权终态不复活。tile_output_status 只持原生短状态窗/停止按钮，无hook/timer/线程；tile_terminal沿原消息泵每500ms枚举并驱动两owner。tile_window遇display/DPI消息先隐藏并使旧代际失效，同步paint之后再核changed/closed，不回挂旧renderer或再次显示。
owner/status/window专属测试保护身份、预算/溢出、终态及同步关闭/DPI重入；tile_output_joint_tests复用原管理器配对/独立输出/真实Loom夹具，只注入测试枚举快照，不改物理显示设置。真实LAN核2×2→4×2注册/layout3、缺席offline、同ID返回/新lease、旧Weak像素和Shared释放，以及停止/撤权不复活。136项相关Rust去重通过/1前台门ignored，11份Rust最大365有效行；失败和静态旧债见WINDOWS_NATIVE_BUILD。物理拔插/普通Tile EXE输出启动/完整Tile与最终包仍开放，旧三副线及固定手测包保持冻结。

## v500 真实 Tile 联合测试职责

desktop/tile_manager 的 authorization_action 是原生菜单到原 Config/Owner 的唯一授权消费点，关闭后不创建任务；不复制配对/凭据业务。tile_native_joint_tests/fixture 专属拥有管理器菜单、隔离 profile、独立输出 authorize/Runtime、自有离屏 NOACTIVATE HWND 及原子阶段文件；只在 cfg(test) 编译，不新增产品入口。Loom tests/tile_native_joint 复用 wall_http 的真实服务/admin helper，管理员只放图/布局，由终端自行注册/租约/取图/心跳；所有 child 有终止/reap owner，失败现场保留在本批 TEMP。
独立固定 RGBA/PBGRA oracle 与权威 scene/presentation 回执分开核验，源像素不冒充最终交换链读回；实际 D3D12 提交、冻结/黑场/恢复、显式停止与撤权清理联合通过。相关108项Rust去重通过/1前台门ignored，5份Rust最大379有效行；input2只改Loom测试叶，完整差异/SHA复用未变目标，失败与静态旧债见 WINDOWS_NATIVE_BUILD。普通 Tile EXE 输出启动/物理多屏/自动热插拔/完整 Tile 与最终包仍开放，三旧副线不重开。

## v499 原生 Tile 图片终端职责

tile_wire/validation 拥有严格有界 Wall v1 DTO、引用及几何校验；tile_geometry 拥有与 Web 共义的裁剪/旋转，tile_clock 拥有有界单调时钟估计。tile_runtime/presenter 是唯一串行注册/租约/心跳 owner；tile_assets 是一个在途图片下载/解码 owner，tile_shared 只有单槽候选和实际呈现回执。tile_image/image_decode_tile 复用原 HTTPS 与共享 Pixels/WIC/隔离 WebP，不复制导出或网络框架。稳定物理输出枚举直接复用 src-tauri/src/tile_outputs.rs，同一实现由两条入口编译。
desktop/tile_terminal/manager/window/window_render 拥有独立原生管理器、输出进程、HWND 与产品 D3D12→D3D11→CPU 准入；main/tile_cli 只作工作区初始化前的精确分流。只广告 image/scheduledPresentation，不广告未实现的 Live/Surface/输入/识别。Loom lan_http_wall 仅适配 GET state 与 POST register/connect/heartbeat/disconnect/images/read，仍由原 Wall runtime 核 Device/nonce、CAS、租约和资源归属；管理员写布局及其它 Tile 路由不开放。
tile_manager_tests保护真实管理窗命令/关闭消费边界，tile_window_tests保护自有窗口同步paint重入销毁与资源清理；关闭后的title/show/hide不再写旧HWND。相关174项Rust去重通过、2 ignored，最终33份Rust最大485有效行；input4仅测试夹具差异，未变普通产品/workspace与Loom库按完整输入/SHA复用。实际失败及验证边界见 WINDOWS_NATIVE_BUILD；不能把源码接线等同于真实跨机、多显示器同步、Live/Art Tile 或最终包通过。显示输出失效/几何变化目前安全清屏并退出，显式重启后重新注册，不冒称已完成热插拔自动恢复。Shader/CAPTURE/PNG 与既有固定手测包不改。

v498：source_control独占同Grant/来源/epoch的控制GET四次有限恢复，保原可靠cursor；首次失联先由原输入owner实际Release/suspend，再请求原媒体owner受控重连，使服务产生可靠controller=None屏障，不新增撤权POST或自动再获取。source_runtime分别持媒体/控制/观察恢复标志及单个重连意图；socket安装前的意图保留，不能因另一链成功清除状态。原stream/Consumer/Publication/frame_id、Lease/取消/join/close及资源预算继续唯一所有权，无新线程/timer/队列/权限/依赖。
source_observation_http的ObservationSubmission持冻结候选/原actor和已尝试标志；未知POST之后只GET精确对账，旧cursor不授重发。仅Client在任何HTTP/TLS发送前产生的Connect允许重试未发请求；完整Source来源/几何/epoch/能力及唯一完整payload均校验。runtime保原20秒候选期限、3秒HTTP和四次暂态退避，真正确认且未取消才commit Tracks；没有“请求已创建=观察已发布”。两份专属测试叶复用既有TLS夹具，Application/Loom既有联合叶只增加控制断线模式，不复制网络/截图框架或重开旧副线。
本批191项Rust完整名去重通过/9 ignored，含42 Application；实际Source控制GET断流→原worker释放→同捕获继续发布→新显式获取/第二轮释放联合2.562秒，UIA真实观察到Loom稳定值变化/密码排除联合5.469秒，其余媒体/输入/普通工作区回归通过。12份Rust最大446有效行，失败、静态旧债、组织合同超时及最终收口边界见WINDOWS_NATIVE_BUILD与main-source-service-reconnect-1回执；不是普通可见双产品、物理跨机、完整迁移或最终包。

v497：live_relay_source_reconnect只拥有同Grant/来源/epoch的Source WSS有限恢复和认证lastFrameId确认；原stream继续独占Consumer/Publication/单调frame_id/唯一GPU帧Arc和媒体线程，不重开捕获、不再次create/续签或重放未知AU。400/800/1600/2000ms最多四次，握手/本地send不重置次数，只有本次新帧已被服务接收且Source仍连接才退出恢复。原Lease/停止唤醒、四槽/预算和join/close保持，无新线程/timer/队列/依赖/权限。
source_video重连等新策略并强制raw/独立IDR，保sticky codec失败；source_http复用完整来源/几何/epoch校验，只读进度不采用服务actor游标。source_control仍是唯一输入dispatch owner，媒体generation变化先实际Release再suspend；旧输入验证/消费但不执行，可靠controller=None屏障之后才允许新的显式获取。公共Application/菜单仅接Recovering短状态和停止，不授重复发布或自动重获控制。
两份既有Loom cfg(test)联合叶与三份原生联合叶覆盖真实Source/Viewer菜单、LAN HTTPS/WSS、系统codec/像素与持键鼠断流后的释放/显式重获；未交接或重开三旧副线。本批177项Rust去重通过、7 ignored，含41 Application，最终输入只在运行后追加三Markdown；23份Rust最大435有效行。input2启动屏障失败与input3仅Loom测试叶修复/精确SHA复用、实际结果及静态旧债详见WINDOWS_NATIVE_BUILD，不冒充普通可见双产品、跨epoch恢复或最终包。

v496：live_relay_control_reconnect只拥有原串行线程内、同Grant/来源/epoch的控制事件GET与注册快照GET有限恢复；Owner保原可靠cursor/Permit/媒体，400/800/1600/2000ms最多四次，不再次join/续签或重放写请求。事件批次已提交但注册刷新暂失时仅重试注册GET，不重复消费事件或从新快照跳过可靠水位；完整成功读周期才结束恢复。原Lease等待/停止唤醒复用，无新增线程、timer或队列。
device_session_failure只分类可辨认的IO传输损失，不自行重试；live_relay_http的retryable_read只由上述GET链使用。TLS/协议/历史reset/错误epoch/容量限制仍终态，POST结果不明仍失败关闭。loom_http_transport的16KiB接收缓冲耗尽明确InvalidData/Http，禁止零长read伪造EOF；原容量、证书和期限不变。
input_queue独立持有media_recovering/control_recovering，任何一方未恢复都禁输入；共用减权/清待发/释放屏障，迟到ACK或另一链恢复不能复活控制。trigger_queue暂停新配置并拒绝尚未发送的意图，不把Unknown变确认。测试断流/只读cursor均cfg(test)，Weak Lease不制造所有权环。
本批153项Rust完整名去重通过、7 ignored，含41项Application；真实Loom控制GET恢复联合6.265秒核cursor4→5、同窗口/epoch/actor/媒体连接及独立像素，媒体恢复回归5.859秒、原键鼠释放5.000秒、普通EXE工作区5项通过。18份Rust最大427有效行；Otter严格门通过，Loom101旧Clippy诊断无新增/过期行数例外仍阻塞。详细失败、复用和未验边界见WINDOWS_NATIVE_BUILD。三旧副线/v464固定包保持冻结，以下旧批次保留当时范围。

v495：live_relay_viewer_reconnect只拥有同Grant/同媒体来源/同epoch的有限WSS恢复；原viewer_runtime仍独占单网络/codec线程和Permit，authorized_media重新核绑定/到期并签发新nonce，不再次join或续签。复用原park/unpark退避400/800/1600/2000ms，无新像素进度最多四次；只读committed水位，握手/重复预热不刷新次数。media_socket只让异常传输/liveness继续使用原worker，协议/帧/预算/撤权及正常Close仍终态；worker记录sticky codec fallback，恢复不复活已失败的H264。
原input_queue在媒体损失时清待发、减权并保释放屏障，迟到acquire不恢复输入；真正新像素后仍须用户显式获取控制，旧图可保留并标短状态“重连中”。Source、控制HTTP轮询、跨epoch和重新授权不由此模块恢复。三项reconnect模型/实际WS测试、同授权TLS新nonce回归和真实Loom H264联合保护身份、进度及停止清理；专属断socket入口全部cfg(test)，不新增生产命令。
本批124项Rust去重通过、7项显式门未运行，含41项Application；真实LAN联合4.437秒核帧2→3、同窗口/epoch1、独立像素和未自动重获控制，原输入释放联合4.125秒、普通EXE工作区5项通过。22份Rust最大417有效行，Otter严格静态门通过；Loom仅cfg(test)改动，同口径101旧Clippy诊断无新增，过期行数例外仍阻塞全仓门。详细证据与边界见WINDOWS_NATIVE_BUILD；三旧副线/v464固定包保持冻结，普通可见双产品/真桌面WGC/跨机/长稳/最终包及其余迁移仍开放。

v494：live_mf_encode及format/pixels叶独占线程绑定的系统同步baseline H264编码、BT709 limited NV12输入、四槽和共享预算；复用原MF/COM及sample资源边界，不新增codec/网络框架。MFT声明的输出buffer与1MiB wire AU分别计费，返回Vec的租约不随codec提前归还；失败/取消立即退休。Source publication只在原媒体线程成功建立encoder后广告H264；小图/奇数/预算不足或透明输入保raw，不裁切、不丢alpha、不扩大原资源池。
source_policy只解析有界strict video_policy；source_video独占epoch/单调请求序号、IDR及sticky失败回退。Source socket仅在实际H264协商后接策略，transport还核所选profile属于真实offer；Viewer兼容raw。原stream仅保留一份已有GPU帧Arc，静止画面收到策略/关键帧请求也可重发，不重开捕获或增加采样线程。100ms控制调度、30fps上限、每30帧/显式请求IDR；原Device/nonce、Lease、250ms分片、停止/join/close保持。
application_live_viewer_video_joint与Loom live_h264_native_joint只拥有隔离联合测试：真实Source/Viewer菜单、LAN HTTPS/WSS及系统codec，服务/WS确认wire codec和精确frame_id，原生绘制按同一ID核像素，不把已解码RawBgra误判为wire H264。相关108项Rust通过、6项显式门未运行，含41项Application及两真实联合；定向/复用/失败与静态旧债见WINDOWS_NATIVE_BUILD。关闭所述Source H264隔离闭环，不关闭普通可见双产品/物理跨机、重连/epoch、完整LiveRelay/Tile、旧Web互通、长稳及最终包。三旧副线和v464固定包不动。

v493：live_relay_trigger_command拥有完整高级JSON配置及revision候选；viewer_trigger_queue拥有固定真实Surface/attachment的单pending/inflight槽和stop终态，viewer_trigger_http只做单次POST及已认证成员/双游标/完整注册的精确只读对账。键鼠与配置共用原Sender唯一actor游标、串行控制线程和Lease；先释放旧输入再配置。未知结果不重放，关闭旧观看；明确拒绝保留观看。LocalArt不伪造Surface配置资格。
composition_relay_window_trigger独占显式新增/编辑/禁用菜单与模态后宿主重入复核；shape_dialog仅增加16KiB Trigger模式及原生EDIT验证。Owner/Business只刷新已确认注册，不从GET导入超前observation/audit或推进可靠cursor；可靠trigger_condition事件触发只读注册刷新。配置不执行条件或Art，禁用不撤回已reserved动作，Fired不等Surface业务成功。
本批相关Rust完整名去重113 passed/0 failed/2 ignored，包含41项Application实际通过、真实Source UIA→Viewer配置→Loom→实际framework/Art进程→Surface正式结果联合13.390秒及旧控制释放联合3.218秒；普通EXE工作区5项通过。input4/5只修cfg(test)夹具，生产/未变测试/产物SHA证明后复用已过结果，失败留存，不称新库整库重跑。Loom仅增加cfg(test)夹具及实际Art fixture，无生产/依赖/权限变更；精确范围与静态旧债见WINDOWS_NATIVE_BUILD。
本批关闭真实Surface宿主的高级JSON触发配置/求值/执行终态隔离闭环，不关闭本地Art触发合同、普通可见菜单导航/最终像素、H264 Source、重连/epoch、完整LiveRelay/Tile、旧Web实际互通、历史前台、资源长稳、普通可见双产品/物理跨机、最终整包及用户全面手测。三旧副线/v464固定包/真实profile与设备不动，不提交/推送/发布/部署。

v492：live_source_uia/sample只在受限MTA child读取显式窗口；process复用bounded_worker_process与overlapped Pipe，128MiB/child、四份512MiB池、单在途扫描2秒、256节点/16层/512KiB响应，密码与跨PID子树排除，不执行Pattern动作。live_source_observation拥有两样本稳定/1秒心跳/缺失Stale与失败Error、256项/512KiB累计快照，仅真实ack或精确只读对账后commit；runtime独占同Grant独立Lease/串行观察线程和child回收。
原生Live系统菜单新增显式“发布 Live 并共享窗口观察”。普通发布和允许控制不默认读UIA，观察不授输入能力；Region/裁切窗口拒绝整窗观察，避免读取选区以外内容。通过原Device/nonce/HTTPS观察路由发布，不新增依赖、权限、全局hook或第二截图/网络框架。
原Source媒体owner在停止后join观察/回收child，再以原媒体Lease对账actor并close；不再固定close=2，观察ID另有序号。迟到扫描不新发、迟到ack不commit；已经到达服务端的请求不能撤回，close前按同锁member快照核source/device/hook/epoch、cursor与完整唯一payload，不盲重放。2秒是扫描pipe期限，不是包含驱动/内核清理在内的硬实时保证。
当前同源相关Rust完整名去重71 passed / 0 failed / 3 ignored，Source及Viewer观察19项Application逐例独立进程。自有离屏NOACTIVATE标准EDIT到受限child的真实值变化、密码排除、取消/Job/容量回收通过；状态/HTTP覆盖稳定、心跳、locator变化、失败/缺失、丢ack精确对账。实际Source菜单→真实Loom生产库LAN HTTPS观察路由的stable seven→eight/密码排除/close联合4.67秒通过；原Source输入/释放0.982秒及Viewer→Source菜单输入/释放4.076秒回归通过。普通EXE工作区5 passed/1前台退出ignored，2.809秒，覆盖启动/Checkpoint/中断重开/末Unit删除不复活及SHA准入。
本批关闭Source显式UIA观察采集/发布/停止的定向与隔离LAN联合，不关闭实际Viewer可见观察窗口/触发配置与执行全链、H264 Source、重连/epoch恢复、完整LiveRelay/Tile、旧Web实际互通、历史前台、资源长稳、普通可见双产品/物理跨端、最终整包及用户全面手测。三旧副线/v464固定包/真实profile与设备不动，不提交/推送/发布/部署。输入、失败、复用及静态边界见WINDOWS_NATIVE_BUILD。


v491：composition_relay_window_observation拥有显式系统菜单、Host已授权业务快照和面板移出借用后的接线；observation_window独占标准只读EDIT/自有HWND/捕获排除、选区/阅读位置及同步销毁；observation_text独占有界语义投影，不读取UIA或执行动作。Business以共享显示身份避免普通帧/输入/心跳重写文本，只有成功语义事务换代。Application专属observation_peer/tests复用原TLS/WSS/菜单夹具与独立常量oracle，Session/Viewers只加cfg(test)查询。
原生远程观看系统菜单“查看观察与触发记录”现可显示观察状态/值/来源/置信度/时间、加入时触发注册快照及最近16项审计。reservation显示“派发中”，不伪报Surface动作完成；Viewer不重新计算条件或执行Art。只在用户显式打开时格式化，单值512字符/整窗128Ki UTF-16单位，超限明确标记，NUL/换行/bidi控制可见转义；敏感文本不写普通日志/磁盘。不新增线程、timer、hook、依赖、权限、网络端点或资源预算。
打开查看窗先释放原远端键鼠控制；关闭面板保留观看，撤权/关Viewer/退出销毁子控件和共享快照。更新不持Relay/Host的RefCell借用跨Win32同步调用；重入关闭不回填旧owner。相同语义身份不重写EDIT，语义更新保留选区/垂直阅读位置；失败只关闭查看窗并留下可重试入口，不将只读面板错误升级为文档或媒体失败。
当前同源相关Rust完整名去重85 passed / 0 failed / 1 ignored，20项Viewer Application逐例独立进程通过，其中新增两项Surface/本地Art观察快照→可靠增量→reservation/final替换→关闭重开→撤权，以及一项打开查看先释放控制。新真实EDIT/菜单/捕获排除/选区/最大容量/控制符/同步销毁回归通过；Application观察数据来自独立TLS/WSS常量peer，不是实际Loom UIA。另原真实Loom生产库LAN HTTPS/WSS→Viewer→Source执行/释放联合3.357秒通过，作为共同观看/控制链回归，不冒充新观察服务端或触发执行全链。普通EXE工作区5 passed/1前台退出ignored，核启动/Checkpoint/中断重开/末Unit删除不复活及SHA准入，不是普通可见双产品或物理跨机。
本批只关闭已授权Viewer的原生观察/注册快照/审计查看，不关闭Source UIA生产、触发配置/执行、H264 Source、重连/epoch恢复、完整LiveRelay/Tile、旧Web实际互通、历史前台根因、资源长稳、普通可见双产品/物理跨端、最终整合包和用户全面手测。三旧副线与v464固定包保持冻结，不触真实profile/设备，不提交/推送/发布/部署。集中证据见WINDOWS_NATIVE_BUILD；以下旧条目保留当时输入边界。
## 当前计划与新增责任入口

v490：art_live_control_http只拥有本地Art原生菜单的acquire/release/input HTTPS适配，复用原controller响应/input envelope校验；不授JS输入API、不伪造Surface附件。live_relay_viewer_input_runtime沿同一串行控制线程、16项队列和Lease分发Surface/LocalArt；宿主退休禁新输入/再获取、尝试释放已有控制，独立观看保留。Application专属测试复用原TLS/WSS/离屏窗口夹具，没有新输入框架、线程或全局hook。
Loom protocol/local_art_live_control拥有独立strict wire合同；daemon local_art/live_control与live_host分别拥有原生意图适配和attach/control共享当前授权围栏，live_session_local_control只转原controller/input store，不复制成员、epoch或序号业务。精确Device-only LAN端点32KiB；Preview不自动join/acquire，需已有成员及Source输入许可。Art撤审批/过期后仅允许自己的release减权，Device/签名/精确绑定/原排他及双序号仍必需。
真实Source联合进一步发现会话close先于socket guard Drop时，media退休被closed围栏拒绝。live_session_media现允许已核Source/Viewer身份的false退休清理closed tombstone，true仍active-only；保活动controller过期规则，不伪清真实连接数。live_media_connection_cleanup以真实RoleGuard/Permit确定性测试双viewer/Source计数及伪身份/禁止重开；联合沿原30秒期限等待服务端role与permit归零。
本批相关122项Rust通过、10项显式门未运行，其中当前组合80项实跑、42项未变原生夹具按input2/SHA复用，protocol local_art17项另计；17项Viewer Application逐例隔离、真实Art HTTPS撤权减权联合及原Surface→Source实际执行释放回归通过。普通可见双产品/物理跨机/最终包仍开放，当前证据见WINDOWS_NATIVE_BUILD；以下v489等为历史边界。

v489：application_live_viewer_joint_tests/fixture只拥有真实Surface公共attach/active与原Viewer目录/菜单的cfg(test)消费者；source_joint_observe只观察原WndProc实际Key/Mouse及原worker完成/cleanup，不把仍在发布的Bridge资格当成线程未释放。Source仍沿原系统菜单、捕获与唯一输入worker，不注入用户程序输入。
Loom lan_http_surface只拥有三类原生viewer宿主必需的精确LAN路由、Device/nonce前置边界及16KiB请求门；原runtime继续独占设备/附件/生命周期/重放鉴权，不开放Surface管理、任意事件、mount或资源接口。专属原始请求测试与live_viewer_source_native_joint的独立状态/字节oracle保护边界，public lib/lan_http仅登记最小接缝。
真实Loom生产库/LAN HTTPS/WSS→两个原生Application完整联合3.886秒通过，两轮输入分别由菜单/失焦释放，Source继续发布且worker回收，Surface按Mounted1/Active2/Inactive3、actor9/input4终态和精确2x2像素核验。相关365项Rust去重通过中21项本批实跑、344项按完整delta/SHA复用，24 ignored不计通过；11份Rust最大436有效行。普通可见双EXE、真桌面WGC/物理跨机与完整迁移仍开放，集中证据见WINDOWS_NATIVE_BUILD。
旧Web/Tauri实际wire同为vk:1..254/modifiers0，不再凭旧DOM键码假设新增转换；旧Web产品运行互通仍未验。以下v488/v487等只保留历史边界。

v488：live_relay_viewer_input_http只拥有真实Surface附件的control/input合同、能力与响应校验；input_queue拥有16项有界UI队列、连续move合并及取消/重新启用间的释放屏障，input_runtime复用原串行Control线程/Lease和actor双游标，不另开输入线程或等待句柄。只有本实例显式acquire成功才启用，不接管同device已有controller；HTTP acceptedSequence只证明可靠事件被接受，不是Source执行回执。
composition_relay_window_input拥有独立获取/释放系统菜单、失焦/关闭/几何变化撤销及当前已呈现Viewport；window_input_event仅共享Win32事件解码，本地Live仍保原owner、队列和清理。Ready/Session只移交同一Queue，窗口回调无网络或join；控制失败关闭，远端撤销控制可继续观看。本地Art proof只授观看，保持只读，不伪造Surface附件或扩大授权。
三份input专属测试叶、共享mapper测试及application_live_viewer_input_tests/peer保护队列、双序号、释放屏障、隔离TLS/WSS与原Application/菜单/自有离屏窗口；15项新增均通过。相关348项Rust完整名去重通过、0失败、24 ignored；11项viewer含原六观看及14项Source Application通过，25份Rust最大472有效行。input4只修负向测试错误领取，未变生产产物与不依赖该夹具的结果按完整delta/SHA复用；真实Loom两Source联合不冒充viewer公共控制联合。集中边界见WINDOWS_NATIVE_BUILD；本地Art控制、Web键码互通、观察/trigger、H264 Source、恢复及最终产品验收仍开放，以下v487保留历史输入。

v487：live_relay_source_input只拥有有界单在途UI桥、真实执行/释放回执及原worker的StopSignal；source_input_wire整批预检可靠事件、当前controller和单调输入序号，source_control使用同Grant的独立Lease/Cancel轮询，不与媒体共用取消槽。原Source owner仍拥有四槽、媒体、预算和退出；配置早退/panic、空闲Bridge撤销及源停止均结束控制链，不另造输入框架或等待句柄。
composition_live_remote_input将请求接入Session唯一live_input_worker，controller切换/cancel先排空并释放旧worker；远程占用时阻止本地输入交错。publication/source与application_live_source仅接显式“发布 Live 并允许远程控制”意图，普通发布仍只读，Region不授窗口控制。原Application/工作区/来源/授权围栏及Win32 Target裁剪和安全检查保持；没有全局hook、SendInput、焦点切换或鼠标移动。
source_input/wire测试、application_live_source_interaction_tests/oracle及Loom cfg(test) live_source_native_joint分别保护模型、原Session真实键鼠释放和LAN联合。相关341项Rust完整名去重通过、0失败、24 ignored，26份Rust最大465有效行；输入5只修测试owner后，未变结果按完整delta/产物SHA复用，不称全库。当前仅窗口消息能力与vk:NN键码，原生viewer控制UI/协议键码互通、观察、H264 Source及恢复仍开放；真实联合控制侧直接使用Loom权威store夹具，不冒充viewer公共HTTP/UI、真桌面WGC或普通可见双端。集中证据及失败/复用边界见WINDOWS_NATIVE_BUILD；以下v486只读能力为历史输入。

v486：live_relay_source_http仅拥有普通Live的显式Source创建/关闭合同，复用原Control/Device/nonce；source_socket复用原TLS/WSS与固定写缓冲，16KiB continuation分片共用整帧250ms期限，不复制完整raw帧。source_runtime/stream只持同一捕获Consumer、一个后台owner和原GPU读回；沿共享编码预算预留512KiB网络缓冲，Pixels/staging仍由原owner计费，不新建WGC、网络框架或自动重连。
composition_live_publication只拥有系统菜单的单槽发布/停止意图，composition_live_source只移交当前来源/尺寸/Consumer；application_live_source在消费点核工作区、稳定Arc/Unit、当前Device Lease及等待容量。composition_relay_sources共享原远程四槽/Control Slot/编码预算；停止未join仍占身份，停止后不再把Ready event加入退出等待集合，原Application退出泵回收真实线程。显式停止不提前撤销原Lease，允许同身份有界close，并区分Confirmed/Unconfirmed/AuthorizationEnded/NotCreated，不能用丢回复伪报服务器已关闭。
专属HTTP/socket/runtime/Application与Loom cfg(test) live_source_native_joint保护实际菜单→真实LAN HTTPS/WSS→原媒体worker的精确像素及停止，测试expected来自固定wire/像素oracle；Loom生产输入不变。相关270项Rust完整名去重通过、9项显式门未运行；34份Rust最大460有效行，集中证据见WINDOWS_NATIVE_BUILD。本批只广告raw_bgra/srgb和空输入/观察能力；远程输入、H264 Source、重连/epoch恢复、真实桌面WGC及普通可见双端/物理跨机不由该联合关闭，以下旧Source缺口保留历史边界。

v485：Loom lan_http_live独占精确Live LAN路由/参数及升级请求白名单，lan_http继续HTTP适配但将已验TLS媒体移交原owner；runtime/live_media_transport仅拥有TCP/LAN TLS的有界IO/停止接缝，原媒体grant/permit/worker/视频协商和会员授权不迁移。lan_listener保存OnceLock/Weak避免registry/runtime循环，退休listener不在registry锁内join；live_session_route_support复用原decoder单次解码session segment。lib与daemon_lifecycle只登记最小接缝，不增加媒体框架或依赖。
Otter live_relay_media_endpoint的Device请求只用认证头/nonce，本地非Device兼容仍保query身份；原Prepared/Session/授权owner不变。Loom lan_http_live_tests/live_lan_native_joint和Otter live_lan_joint_tests/peer拥有本批原始边界及真实LAN→原生GPU/撤权/停用重启验证；旧Application peer只更新测试请求合同。相关405项Rust完整名去重通过、16项显式门未运行，两仓strict行数/Otter strict Clippy通过，21份Rust最大500有效行；Loom仍101条旧Clippy诊断无新增。详细输入/复用/失败见WINDOWS_NATIVE_BUILD；这关闭隔离运输与原生观看联合，不关闭普通source发布/控制、可见双产品/物理跨端或最终包。以下v484及更早LAN缺口保留历史输入边界。

art_live_viewer_http只持有已验签在线Prepared的本地宿主证明与独立Loom加入合同，不给JS增加网络API；art_local_window/owner只暴露当前ready宿主的稳定Arc证明，离线或退休宿主不产生资格。composition_relay_viewer_host显式区分真实Surface附件和本地Art证明；Application及原菜单统一复核当前目录/宿主/新Device Lease。两条加入路径共享原发现/actor游标/ack响应校验、Session四槽/编码预算/媒体窗口与停止回收，不向旧Surface端点填伪附件。专属proof/菜单/Application及双仓HTTP测试保护此边界；当前开发与验证状态见WINDOWS_NATIVE_BUILD。

v484当前批次关闭上述在线本地Art宿主的隔离观看与真实HTTPS加入合同：相关440项Rust去重通过，protocol14项另过，19项显式门未运行；三项新增Art观看与三项原Surface回归逐用例隔离验证原生精确帧及回收。34份Rust最大439有效行，两仓严格行数及Otter严格Clippy通过；Loom仍101条原诊断无新增。新Device-only端点独占于Loom local_art/live_viewer及protocol叶，专属HTTP/joint测试不放宽旧Surface附件。以下v483条目保留历史边界；当前真实LAN完整Live控制/媒体、Source发布/控制、普通可见双端和最终包仍开放。

device_live_viewer_ui只拥有无凭据的目录/真实Surface选择和原授权请求队列；先选附件再列最多512会话，沿公共1024项菜单快照预算，不构造4×512子菜单。application_live_viewer在Application消费点复核目录、实际shown/confirmed-active附件、当前设备/新Lease、模态/退出/恢复和等待容量；不将local_art伪绑定作为服务端Surface附件。
composition_relay_viewers沿原Session/四槽/媒体/窗口/退出泵管理观看，稳定Arc身份使旧停止请求不能停止新owner；停止未回收仍占身份与名额。Ready前失去附件则取消迟到加入，Ready后窗口独立于发起Surface/工作区，显式停止或原授权结束才回收。进程共享单份编码帧预算，解码仍用原Pixels预算；只读consumer显式接受已知状态/观察事件，不执行source输入或二次登记trigger，未知业务失败关闭。
专属UI/Session测试与application_live_viewer_tests/peer分别保护身份、预算及真实隔离TLS/WSS到原生D2D帧和回收；后者复用原合成认证Surface夹具，不冒充真实Loom或普通双产品可见验收。本批集中验证与实际边界见WINDOWS_NATIVE_BUILD；新本地Art附件合同、Source发布/控制和完整LiveRelay仍开放，三旧副线不重开。

application_projection_edit_joint/source/receiver只拥有cfg(test)的双真实Loom/双原生Application协作消费者，共用原隔离身份、菜单、750ms调度、默认Checkpoint与journal ack，不新增生产发送/保存入口。focus叶以本测试PID的被动WinEvent观察receiver从未激活，允许用户切换其他程序，退出注销hook；不吞或注入输入，不改变旧receiver联合门。
Loom projection_edit_application_joint/oracle复用原双服务/有界子进程owner，独立核权威revision、模式、对象坐标与basis/重开/终态PNG，expected不调用Otter新merge或renderer。v482本批29项Rust完整名去重通过（Otter库10、Loom14、固定SHA普通EXE工作区5），0失败、1前台退出门未运行；真实联合17.469秒通过，不冒充普通可见双端/物理跨机器或服务预算阈值实跑。
4047项冻结输入、9份Rust最大211有效行；strict Otter Clippy/两仓所需库构建与格式/严格行数通过，Loom strict Clippy仍101条原诊断且无新增签名。两次夹具失败保留，未改生产业务；普通产品按完整test-only delta/SHA复用。本批只关闭真实服务Application协作联合，普通可见/其余迁移及最终包继续后续，详见WINDOWS_NATIVE_BUILD。

projection_edit_prepare只准备原v1 Mode/Apply/Checkpoint，Mode和服务Checkpoint的acknowledged仅取权威基线，不误确认未发的本地对象；projection_edit_submit保原写前Store与四Slot，Automatic/新Mode遇未知pending只拒绝，显式Submit才恢复原请求。journal_rejection/record允许Mode/Checkpoint的明确可恢复拒绝，仍须read对账与精确默认Checkpoint才退休。
application_projection_edit_automatic只拥有最多64项的工作区/Unit暂停准入，无新timer/线程/队列；成功source probe或receiver read释放原owner后触发单批32项，失败/取消保暂停，真实ack成功才解除。source按receipt224或240对象且含墓碑的原阈值优先服务Checkpoint；receiver只有two_way且无冲突/暂停才自动写。模式菜单由source设置，receiver只显示权威勾选；原Application/Editor/保存/授权owner不变。
projection_edit_prepare_tests/journal_rejection_tests与两份Application automation测试叶保护阈值、未发编辑、模式丢回复重开、原请求重试、shared/offline read→Apply→默认Checkpoint与恢复。旧纯read合并测试显式暂停自动写，完整自动链另测；夹具不在两段rename间盲读日志，也不在同测试线程串联两个真实重开场景。
v481最终input2相关596项Rust完整名去重通过（库591、固定SHA普通EXE工作区5），0失败，5项显式真实Loom/前台门未运行；含13项新增边界。input1两项夹具失败保留，input2仅两cfg(test)叶变化，584项未变库结果及普通产品按完整delta/SHA复用，7项受影响Application重验全过。22份Rust最大468有效行，严格静态/所需构建与格式/行数/版本通过；集中证据见WINDOWS_NATIVE_BUILD，普通可见双端/当前真实Loom联合及最终交付仍开放。

projection_edit_source/wire拥有冻结PNG、basis布局和原始initial；projection_edit_journal_source复用原有界写前Store，任何PNG CAS前持久stage，未知pending不自动重放。projection_edit_source_flow沿原Job/四Slot进行单目标probe/CAS/initialize/attach，只有明确404 projection_edit_not_found允许替换旧flattened PNG；普通sender遇到协作仍拒绝烘焙覆盖。
desktop/projection_edit_source把原Editor快照接到同一basis布局；application_projection_sender_source共享原意图校验与冻结帧，原调度按具体projection公平推进并只缓存弱内容身份。application_projection_edit_source/start/poll拥有原生菜单、逐目标初始化、发布/取消、默认Checkpoint及后台ack；其过渡围栏只接受持有Candidate的原/新allocation与精确索引，不把候选授为已落盘事实。
projection_delivery_runtime只为已校验的Source Ready移交原Active/Lease，避免已join Job析构误撤权；普通metadata仍按原规则取消回收。source专属模型/PNG与实际Application测试及projection_receive_attempt_tests保护租约交接、期间编辑、多目标、失败/取消/撤权、同字节异allocation及恢复，复用原renderer/保存/网络框架，不新增线程、依赖或预算。
v480最终input6相关583项Rust完整名去重通过（库578、普通产品工作区5），0失败；5项显式真实Loom/前台门未运行。52份Rust最大467有效行，严格静态/所需构建及格式/行数/版本通过；input1至5失败保留，生产修复后产品和相关回归均以input6重建重验。源端初始化/冻结basis/多目标的隔离闭环关闭，模式切换、自动提交和普通可见双端/跨机器/最终包仍开放，详见WINDOWS_NATIVE_BUILD。

projection_edit_journal_rejection拥有明确拒绝的原Apply标记和精确Checkpoint后退休，不从未知HTTP/超时或普通read授予资格；原请求/路由/view和磁盘CAS不变。projection_edit_resolution复用原Job/四Slot，只读最新权威后提供显式选择候选；未知pending（包括同key不同device）阻断选择，重开不自动重发已拒绝Apply。
workspace_resolution区分用户选择前的本地修改与read期间的新编辑，采用远端也保留后来编辑并标真冲突；keep-local仍服从最新单向/双向模式。device_projection_edit_ui与application_projection_edit沿原菜单、Editor、Lease和默认Checkpoint发布/恢复，不新建保存或编辑框架；公共authorization只登记该请求已由edit先消费，不重复处理。
v479最终input4相关363项Rust去重通过（库358、普通产品工作区5），含14项新边界；其中3项未变journal按精确delta复用，其余360项运行同输入产物。22份Rust最大466有效行，严格静态/所需构建和格式/行数/版本通过；两项显式联合/前台门未运行。input2后的三份测试叶修复不改变生产，普通产品按完整manifest/SHA复用；编译和模态夹具失败均保留。源端冻结/初始化/多目标、模式切换和完整双向产品继续后续，收口见WINDOWS_NATIVE_BUILD。

projection_edit_submit只在原Job/四Slot内打开既有写前Store、恢复原Apply或固定最多32项请求；ack只走本地后台写盘，不重复HTTP。application_projection_edit/start/poll拥有原生菜单到已有receiver的单在途提交、当前Editor重读、精确默认Checkpoint后确认及最多八批续进；取消/迟到/撤权保原请求，不复制网络/PNG/保存框架。
projection_edit_checkpoint在原保存worker校验canonical协作记录，只留下最多64份轻量内容身份；session_projection_scope::Proof绑定同一allocation，只有真实默认Checkpoint commit可把提交owner标为durable，手动Save不误确认。application_save_publish在真实Save commit后共同推进同候选只读接收Index，不由轮询猜新来源，也不授默认Checkpoint的ack资格。
v478最终input3相关469项Rust完整名去重通过（库464、普通产品工作区5），含11项新模型/实际Application提交测试；两项显式联合/前台门未运行。HTTP后原Editor重读、精确Checkpoint后确认、shared/offline、失败原请求重试/期间修改续进、取消/撤权/底图换代、手动Save成功/取消/同字节不同allocation与重开均通过。4015项输入、33项差异含31份Rust最大481有效行；严格静态/所需构建和格式/行数/版本通过，收口见WINDOWS_NATIVE_BUILD。源端初始化/多目标、模式与冲突选择及普通可见双端/完整协作产品仍后续。

projection_receive_edit只拥有PNG响应内有界Document与同一digest/尺寸的绑定，以及初始receiver View/Context；accept/read沿原路由、授权、PNG预算与WIC，metadata-only同PNG版本也不可丢协作文档。receive_session以同一WindowCandidate移交Base64租约、矢量及有摘要的协作记录，不复制图源或烘焙标注；接收Scope在原Application一次分配并随保存沿用。
desktop/projection_edit_receiver只准备同一Context/单调session/basis的原Editor候选；复用workspace/view合并与原session仿射接缝，保留本地修改/ReadOnly或对象冲突。editor_annotations的可选源rebase与矢量预检后一次交换，undo不回到旧像素。application_projection_sync沿原单任务/Lease/Slot/750ms与失败退避；同版metadata、迟到/撤权/源换代继续原Guard，不新增网络框架或轮询器。
专属projection_receive_edit_tests、原read/editor/annotations扩展及application_projection_edit_receive_tests/fixture覆盖接收、原生D2D/WIC、同版更新/本地冲突/保存重开/迟到；application_projection_edit_attach_tests只拥有旧PNG首次basis接入的真实源/破坏性历史围栏及像素/矢量共同内容框oracle。首次无新底图的三种响应形态统一核源，已有owner不逐轮hash；Bridge保原本地新增，Snapshot沿像素fit内容框记录仿射映射。
v477最终input3相关450项Rust去重通过（库445、普通产品工作区5），两项显式联合/前台门未运行；4006项输入、34项差异含30份Rust最大453有效行。严格静态/所需构建与格式/行数/版本/scoped差异通过，集中收口见WINDOWS_NATIVE_BUILD。本批只读接收，不自动edit apply/ack，源端初始化/多目标、模式/冲突菜单及完整双向产品继续后续；后文v476及更早的editing拒绝是历史状态，不改写旧回执。

projection_edit/workspace拥有工作区内Document/basis、可见basis对象、角色/路由/Context、冲突与显式暂停记录；记录只为恢复，不授凭据、自动重放或journal确认。record流式摘要绑定同一canonical的图源/标注/编辑状态与展开尺寸，坏值/错scope或Unit/内容换代整体拒绝，不从普通annotationState重建wire真值。
desktop/projection_edit_workspace仅绑定原Editor和弱像素/几何身份；不支持的本地底图/几何或标注变化转为持久suspended，普通保存继续，重开不得用旧basis盖掉新图。session_projection_edit在原后台候选中恢复最多64份；Save/Restore Stamp同时跟踪轻量会话身份，元数据变化也淘汰旧候选。工作区仿射映射复用session_vector_write，保留20DIP偏移与会话比例；恢复只绑定，不造undo或复制栅格。
本批验证状态见WINDOWS_NATIVE_BUILD；仅关闭协作权威/当前view的原Save/Checkpoint/恢复接缝，不关闭网络模式/冲突菜单或完整双向产品，原editing拒绝保持。export_projection_content仅为Window新可选字段提供None，不重开PNG业务实现。

v476最终input2相关276项完整名去重通过（库271、普通EXE工作区5），含10项新记录/真实Application事务门，两项显式联合/前台门未运行。4000项输入、28项差异含24份Rust，最大471有效行；严格Clippy/所需构建、格式/行数/版本与scoped差异通过，原借用编译失败保留，显式null存在性修复后重验。普通EXE只验原工作区链，不冒充可见网络协作或完整迁移；收口详见WINDOWS_NATIVE_BUILD与本批final-audit。

projection_edit/editor只拥有View到既有Editor的一次性候选和当前标注回读；fields按原生表示差异回填未改wire原值，不取代basis真值、网络journal或Application生命周期。候选复用AnnotationReplacement，不接管raster/crop/history，文档换代/草稿/资源或合同失败不发布部分结果。
StrokeStyle/TextAnnotation将恢复与变换域同普通UI创建域分开；session读写保留扩域样式、原文、完整rotation、小数序号半径与稳定zIndex。annotation_id的恢复层级元数据随快照/历史保留，新建/复制身份不沿用旧层级；write_order由Bridge与canonical共用，只有真实重排或新增溢出才重编号，删除不改其他层级，最大合法层级在没有新增时也不重写。
editor_visibility供绘制/命中/窗口区域共享零宽/透明/零尺寸判定；polygon_geometry有界到64边。专属模型、真实D2D/WIC/DirectWrite及隔离Application保存重开测试按原owner登记；本批集中验证状态见WINDOWS_NATIVE_BUILD。View原始可选字段与secondaryFill由适配器basis保留，不伪装成独立canonical中的新协作权威；Application模式/网络/冲突/checkpoint最终接线仍开放，原editing拒绝保持。

v475相关489项完整名去重通过（库484、普通EXE工作区5），含18项新增，两项显式门未运行。input2后只有测试oracle修复，生产/cfg边界、完整输入和SHA证明后复用未变模块与普通产品，最后两模块16项重验全过；原失败保留，不称全库/双向产品或最终包通过。38份Rust最大487有效行。

editor_annotations只拥有带DocumentStamp的一次性标注替换候选；edit_history的Annotations只交换矢量/serial，不接管来源raster/crop/logical_size/history，预算与旧redo/淘汰统一由原History维护。
arrow_path_geometry只借用最后有效段之前的点前缀，保留实时Web的2单位锚段回溯；annotation_linear与原selection_hit/group_bounds/hit_region_annotations共用它，不另建编辑器或导出管线。session_vectors/editor_restore保留多点line/arrow与单点路径，普通创建手势仍保持原规则。
专属事务、canonical往返、D2D/WIC像素/命中/区域及实际Application保存重开测试按原模块登记；本批集中验证状态见WINDOWS_NATIVE_BUILD。完整文字域及Application协作模式/冲突/checkpoint仍待接，原editing拒绝保持。
v474相关408项完整名去重通过、18项新增，普通产品工作区5项含在内，前台菜单门1项未运行；20份Rust最大484有效行。仅Application测试oracle因20DIP留白修正后重编，生产及402项未改模块按delta/SHA复用，原失败保留，不称整库/完整协作或最终包验收。

projection_edit_view只拥有basis真值、本地逻辑尺寸与有界派生wire锚点；mapping按轴缩放几何、按正向平均倍率缩放样式/字体，保留合法wire域和稳定ID/zIndex，不经过普通session的UI限制或重编号。
view_merge分离权威基线与可见本地变更，保存期间必须重新读取current；同对象竞态保留sticky冲突，undo只更新当前payload，不从旧冲突复活已撤销内容。该纯层不发网/写盘/签发checkpoint回执。
专属数值、合法边界、对象冲突及真实隔离journal/TLS测试拥有独立oracle；v473最终input2相关54项Rust去重通过，含23项新增及普通EXE工作区5项，两项显式门未运行。缺省字号只物化local，不扩大原wire；数值容差不误合并大倍率相邻坐标。Application/Editor最终呈现、模式菜单及实际checkpoint接线仍待验收，原PNG editing拒绝保留，详细边界见WINDOWS_NATIVE_BUILD。

stroke_paint_alpha只拥有有界独立描边/填充alpha；StrokeStyle继续拥有整体opacity与既有RGB UI颜色，显式替换某个RGB只重置该画笔alpha。
editor_stroke_style拥有Stroke与手势样式的完整桥，history/snapshot/adopt沿原Editor，不新建协作编辑器。
session_vector_color复用纯色解析并保留alpha，原session_vectors/write负责canonical读写；annotation_opacity及原形状/路径/序号/笔刷叶复用同一Canvas/PNG管线。
专属alpha模型、WIC像素与真实Application保存/Checkpoint/CPU恢复测试分别拥有独立oracle；本批开发与验证状态见WINDOWS_NATIVE_BUILD。协作几何/文字合同及Application模式/冲突接线未由此关闭。
v472最终input2相关444项Rust去重通过、13项新增边界通过；5项显式门未运行，普通EXE工作区与严格静态/行数通过，不冒充完整协作产品或最终包验收。

projection_edit拥有独立v1文档/六操作、严格标注边界及固定shared/offline路由，不复制Loom合并权威。
journal/record/file分别拥有profile独占/单在途/确认围栏、持久形状与完整磁盘CAS/恢复；HTTP成功不退休pending，必须由调用者在精确工作区checkpoint后显式确认。
edit_http复用原Control/Device/nonce/Cancel，失败/冲突/丢回复保留原request与view，不自动重发、换opId或静默rebase。
模型、真实文件/TLS及显式双真实Loom测试分别拥有独立wire/恢复/权威oracle；联合fixture的显式ack不是Application保存证明。
Application/标注渲染/模式与冲突菜单仍待接，当前PNG的editing拒绝保留，不宣称双向产品完成。v471集中验证状态见WINDOWS_NATIVE_BUILD。
v471相关227项Rust去重通过，含26项新边界与双真实Loom父子联合；input2后仅联合叶oracle修复，Otter/生产未变明确复用，失败不抹除。严格Otter Clippy及双仓行数通过，Loom101条旧诊断无新增。

tests/local_art_runtime/offline_joint拥有普通双EXE的实际离线发送/接受、reset/flip、保存重开与停止场景；专属fixture仅安装合成身份/profile，oracle独立构造源分辨率条带。
Loom projection_offline_product_joint复用双真实服务和既有有界子进程夹具，从授权receiver读取PNG独立核像素/metadata/digest；不增加生产入口或更改预算/协议。v470完整普通双产品闭环及三份PNG独立复核通过。
product_process只在Windows测试内持有kill-on-close Job；child在父登记前不得spawn产品，失败不遗留孤儿；launcher标准UIA焦点请求仍必须满足真实前台和原15秒期限，不绕过系统锁。
本批source定位从当前canonical的明确Unit身份/坐标空间/完整矩形取得唯一自有HWND，不以初始位置或枚举顺序猜重开窗口；专属回归覆盖保存位置与非单位坐标空间。12项去重定向/集成通过，未变Art与Job按输入/SHA复用，不是整库或跨机器/最终包验收。

application_projection_receiver_joint_tests仅拥有显式双真实Loom的原生接收消费者，复用原菜单/Job/Checkpoint/实际NOACTIVATE帧/持续同步及同profile重开；sender_joint复用同份夹具，不新增产品入口。
Loom lan_http只放行既有offline device与signed peer精确路由，仍拒绝管理API、浏览器凭据和路径别名，沿原预算/独立认证；专属joint夹具以独立PNG oracle核两端，不把测试Application当普通EXE/跨机验收。
Loom projection_offline_joint_proof仅在合成隔离root读取身份，独立构造四种signed peer请求并经真实HTTPS核回复签名、重放/篡改拒绝与offer幂等；不复制生产传输或公开私钥。
v469选定44项去重联合/直接回归及五份PNG独立复核已通过；原source metadata-only read和catalog digest追加位置的夹具错误已修，不为测试改变生产协议或预算。普通EXE可见/全链TLS/跨机及最终包仍开放。

projection_receive_route仅拥有接收侧静态端点与显式transport origin/peer，原签名source origin不改；inbox_decode全量核元数据后原子合并，合计64项不截断。
accept/read在原PNG预算内核route/peer/真实本机device，journal与binding固定同一路由；缺字段兼容旧在线形状，显式null/残缺路由拒绝。
原Application/Job/Lease/Checkpoint/真实呈现/补偿及持续同步owner不变，只将本机transport用于授权和网络；Loom继续独占互信与跨peer转发。
offline模型/HTTP测试及原Application夹具的routed场景保护保存重开、accepted恢复、显示回执、取消/迟到/撤权、持续读取与解绑；本批运行状态见WINDOWS_NATIVE_BUILD。

projection_sender_route拥有固定目录到原Selection的显式离线路由、四个静态端点及原意图恢复，不授互信管理权。
binding_route保留Web的offlineTransport.origin与remote设备/peerId，native marker固定复合目标；不改签名envelope或从HTTP响应取路由。
原sender的Job/Lease/Store/PNG/CAS/Checkpoint/停止owner不变；菜单及组共享同一准入与八项容量，跨Loom同名设备不合并。
route_tests/offline_http_tests与desktop/offline_tests保护日志、身份、取消/迟到、真实隔离TLS及混合组停止；原生离线接收与真实双端另验。

application_projection_sender_stop_group/flow拥有组停止的最多八项原Request、逐项受理状态及精确来源围栏；不复制停止协议或网络框架。
device_projection_groups_ui固定目录Arc、Unit和canonical allocation，Application消费StopProjectionGroup；只操作当前组已知成员的原生关联。
原stop单槽先完成各项持久减权，再轮转处理pending/移除/ack，单项网络失败不能饿死其他成员；网络/持久回执仍由原cleanup与Job拥有。
取消/来源或工作区失效只丢弃尚未受理项；已发布的停止意图继续原恢复链。只有canonical移除和原日志ack均确认才报告停止已确认。
原Checkpoint真实commit通过stop_committed传递已核Index、编码canonical及原Receipt来源；仅此回执登记编码allocation，不从轮询当前原文猜测换代。
stop_group_tests/guard复用原组TLS和Checkpoint夹具，保护完整停止、失败隔离、取消/迟到、重开、缺原意图和菜单身份；不新增线程/依赖/权限或像素副本。

device_projection_groups_ui仅固定当前目录Arc、组索引和Unit；Application的sender_group/flow拥有最多64成员的串行队列与逐目标状态。
每个成员复用原单Job/Lease/PNG/Request/Checkpoint；只在本事务canonical包含后换代来源围栏并推进，不从任意当前原文重抓真值。
projection_sender_admission共享首发与整组容量核验，包括旧关联/primary；保持每Unit八项上限，首份签名和网络副作用前拒绝超限。
组失败暂停原Retry，取消/来源换代清除剩余计划但不解绑已确认关联；已有逐目标停止和持续更新不变。无新线程、协议、权限或整图副本。
group_tests/peer/guard拥有隔离组菜单、TLS独立PNG像素/digest、canonical/重开、取消/重试/迟到及容量测试；可见UI/真实跨端和完整组开关另验。

tests/support/product仅拥有集成验证的显式EXE/SHA256准入及只读文件租约；缺配对摘要、相对/重解析路径或不匹配字节均在spawn前拒绝。
workspace_runtime复用它验证固定release候选，原离屏位置/Checkpoint/中断重启/最后Unit删除与退出断言和15秒期限不变。
这是测试专属路径，不是产品启动开关；不新增生产权限/依赖，也不让测试加载真实profile。v464实际包及3项准入保护共5项通过，1项前台菜单门未运行。

tests/local_art_runtime/projection仅拥有普通产品Art正式结果到持续投送的可见集成场景，
projection_source.js使用公开SDK区分预览与正式提交，projection_peer复用原有界TLS夹具并独立解码PNG。
ui沿真实UIA ExpandCollapse展开同owner子菜单，状态仍在关闭不可变菜单后等待后台接纳；
fixture显式产品路径必须绑定SHA256，projection_oracle独立按矩形覆盖面积构造投送像素，再用既有WIC基准核编码舍入。
它们不新增生产发送入口、不改授权/预算或三副线冻结实现。input16同源普通EXE的三条可见场景已完整通过，
含持久停止、蓝图checkpoint、跨进程恢复及正常退出；两份接收PNG另经独立解码核像素/尺寸/digest。
启动器须经Computer Use真实激活，产品菜单仍核GetForegroundWindow；旧前台失败不改写，不能泛化为全部物理输入或跨端通过。

application_save_state以原OutputStamp追踪正式端口，Editor未变化也能使手动dirty与自动Checkpoint失配；预览/进度不推进此身份。
application_workspace_checkpoint_output_tests保护正式图像/值/清空、预览隔离、重开不复活旧端口及在途替换/取消不发布；
只保轻量身份，不延长像素租约。input16直接依赖、公开SDK、UIA及startup共175项去重通过，另三条普通可见场景通过，不是整库通过。

application_reference先验证worker取消/策略/候选，再将成功且library_changed=false的维护结果无发布结束；
首次呈现造成的模型变化不应让无变更维护误弹阻塞模态框。真正写库仍核完整Receipt和退出围栏。
application_workspace_checkpoint_maintenance_tests保护no-op不替换原文/置脏，以及真正过期、取消和策略变化仍拒绝；
不新增任务槽、线程或像素租约，同工作区无输入探针确认两个窗口enabled/响应且无模态框。

export_raw_alpha拥有原始PBGRA的png_checkpoint专属显式straight BGRA转换，application_save_images调用此入口，直接填充原预算内的WIC副本，
不新增整图中间缓存；独占bitmap锁不跨线程，逐行取消，所有失败路径由COM与预算owner回收。
完整alpha/channel矩阵、普通产品图标及连续四次真实WIC保存恢复已通过，直接raw消费者302项去重回归覆盖对应冻结输入；
普通png_raw和Shader straight入口的既有语义不变，不将该历史302项整体冒充当前所有源码的验收成绩。

application_projection_sender_joint_tests/fixture只拥有显式真实Loom联合消费者和有界一次性合成身份输入，
沿原Application/菜单/Job/Checkpoint，不创建第二发送实现。Loom projection_native_joint拥有真实HTTPS服务、
独立接收身份及PNG像素oracle，projection_http只登记；生产routes不改，缺digest仍拒绝。
本批修正sender update的必需digest字段并加强domain/peer断言，真实HTTPS联合与相关163项去重通过，
另普通Otter产品构建及两个既有worker通过。joint_fixture仅在范围核验时canonicalize，保留原profile路径准入；原失败不删除。
54份模拟TLS及真实Loom接收PNG已核尺寸/metadata/digest，持续与真实接收PNG另经独立逐像素复核；不修改三副线冻结实现。
严格Otter Clippy/lib通过，Loom101条既有诊断、无新增签名；生产未变产物按哈希复用，不等于可见UI/跨设备或最终包通过。

持续投送由projection_sender_update拥有单次CAS快照与精确版本/digest，projection_sender_update_http拥有
read/update/一次只读对账；projection_sender_update_flow沿原Job/Active/四槽，不登记新邀请补偿或修改首帧日志。
application_projection_sender_continuous拥有已提交关联的调度/当前来源捕获，update_schedule拥有有界公平/退避/内容凭证，
update_publish只在来源/工作区/Lease/canonical重新核验后交原Save/Checkpoint。公共来源/PNG渲染与正式保存owner不变。
application_projection_sender_content_stamp拥有弱原文/像素身份，缓存不保留Scope或延长像素预算租约；
snapshot的自然源凭证同样持弱身份，缓存的canonical保守检查不改变首发Scope语义。
Updating停止仍由原stop/poll owner先持久减权，再按原checkpoint/unlink/ack收尾；不新增易失停止队列或UI磁盘锁。
来源角色准入复用projection_sender_admission，不允许接收角色借旧source关联执行更新。
新增模型、时钟调度及隐藏菜单/隔离TLS/迟到/重开测试归各专属模块；相关290项去重通过，另普通产品构建及两个既有worker通过。
34份既有和20份持续TLS PNG核尺寸/metadata/digest，新20份用独立解码/固定像素oracle复核；严格静态/格式/行数门通过。
生产输入不变的回归按哈希复用；原失败/超时保留，不宣称单进程全组、普通可见产品、跨端或最终包通过。

敏感服务确认批次已通过选定定向/联合验证：art_service_confirmation拥有宿主专属单票据槽，confirmation_wire拥有显式网络合同与输入指纹，
confirmation_preview在原后台线程复用WIC/Thumbnail生成实际上传图像预览，不提高输入/像素/任务预算。
art_local_confirmation仅拥有非模态HWND和选择记录，art_local_window_confirmation在宿主/Lease重新核验后才移交批准。
Application只为自有提示翻译键盘，不持模型借用跨同步派发；参数区Enter和自有Escape记录拒绝，创建不自动设焦点。
默认拒绝、停止/换代/到期销毁窗口与计时器；长参数不得静默截断。
image_thumbnail统一拥有菜单和确认预览共用的原有GDI绘制叶模块，历史native_menu_panel_thumbnail源码路径保留且只编译一次。
Loom的service_confirmation为原Task的Gate，runtime/host/confirmation负责精确命令登记和scope绑定；旧SDK入口不自动扩权。
对应模型、signed TLS、真实隐藏HWND和五条显式双仓HTTPS均通过，含取消/撤权/命令换代及实际图片预览。
最新组合输入Otter57/runtime53/protocol12/daemon38项去重通过；两仓产品构建/三个既有worker与Otter严格Clippy通过。
Loom保留daemon101/protocol10旧诊断，runtime另4条诊断文件前态字节未改；不是全仓静态或可见用户确认验收。
三副线冻结文件不变；证据main-art-service-confirmation-1，不关闭完整SDK/跨端/长稳及最终包。

v460库回归续跑仅修改linshi中的运行/解析脚本及本节状态，不新增生产模块或更改文件归属。
4575个库测试名均有终态证据：4430通过、139默认ignored、6项原前台/串行锁失败仍开放；不是单进程全量通过。
嵌套UIA子进程的libtest输出按套件层级解析，不能覆盖父测试、把默认ignored子入口算通过或重复累计。
六项非前台锁中毒场景已补验通过，2414项原无终态已补齐；三副线35/28/18项与生产输入保持不变。

v460可见产品补验：tests/local_art_runtime/foreground只拥有自建启动器和精确子PID前台许可；
service_image读取同份SDK示例，复用原完整包/TLS/独立像素与checkpoint夹具，不能代替实际前台通过。
两次前台准入失败仍保留；完整lib总期限900秒中断，最后在途事件用例单独25.078秒通过，不据此声称全库通过。
补跑发现tray_actions旧菜单数量oracle漏掉录制入口；测试现在独立核对七命令顺序和两处分隔线，保留原模态/待交付断言。
生产菜单、权限、三副线与Loom不变；当前仍需可见产品、完整回归和最终交付验收。

v460服务图像输出批次：art_service_image_output拥有独立网络envelope与PNG身份校验，不扩大旧JSON门；
原dispatcher持有当前会话图片，成功后沿原cancel端点释放Loom的终态图片，正式结果仍由原Application提交。
surface_javascript_service_wire/worker仅在请求显式imageOutput时沿同一pipe传二进制；engine_services在续接Promise前原子安装。
canvas_images累计核输入和服务结果的原8图/编码/像素/Canvas预算，重复ID或失败不替换旧表；无新资源框架。
Loom local_art/service_image_output只提取显式单PNG content，不读文件/URL、不应用effects；service_tasks拥有有界存储与原终态/取消。
专属模型、signed TLS、真实双仓HTTPS与Application/Save测试和公开SDK示例保护整个结果链；
本批Otter294、protocol11、daemon43项去重通过，包含新输出及原JSON/图片/Canvas四条联合门。
两仓产品构建及Otter三项既有worker回归通过；Otter严格Clippy通过，Loom仍101条旧诊断且无新增签名，不代替可见UI/跨端/最终包。

v459 art_service_canvas只适配当前child的Canvas快照到原图片上传，不拥有授权、服务状态或资源框架。
surface_javascript_canvas的私有serviceFrames负责调用时flush与身份选择；engine_services在JS栈外编码，不读取后续命令。
canvas_wire::decode由正式帧与服务共用，保留原尺寸/格式/像素预算；services/wire继续拥有原单在途/调用限额与私有交付。
canvas_service_model_tests/canvas_service_tests分别拥有模型和实际child/产品worker，art_service_canvas_tests拥有原签名/TLS/取消链。
services在参数准备期间拒绝重入，成功选择后才消耗序号；选择失败不破坏下一次合法调用。
本批Otter287、Loom protocol10/daemon38项去重通过，含三种真实HTTPS联合；另三项实际产品worker及Otter严格Clippy/lib/产品构建通过。
Loom严格Clippy仍有既有101诊断，无新增签名；没有修改三副线冻结文件、公开依赖或Loom生产任务/网络协议。

v458 art_service_images拥有当前已安装PNG的选择、编码预算和独立service.images请求；
Prepared先合并已验包资源再将共享图片交原dispatcher，不接受脚本路径/字节或新授权。
surface_javascript_services私有闭包验证image句柄，仍共用原单在途和调用限额；HTTP控制/结果/清理沿原owner。
专属images测试覆盖实际signed child/TLS、拒绝和取消；joint夹具新增真实Loom HTTPS模式。
图像输出、Canvas采样服务输入及敏感gesture另列未完，三副线冻结文件未改。
本批Otter263、Loom protocol10/daemon37项去重定向通过，含真实双仓HTTPS图片输入与撤审批；
两仓产品构建及Otter实际产品Canvas/状态worker通过，Otter严格Clippy/lib通过，不代替可见菜单或最终包。
Loom严格Clippy仍有既有101/10诊断且未观察到新增；本批两个旧诊断文件被触及，但对应表达式/函数签名未变。

v457作者入口批次：Loom/sdk/otter-art拥有独立原生类型与直接分发JS示例，不与旧Surface全局合并；
application_local_art_sdk_tests只读取并核对同份公开示例，复用完整包/隔离TLS/Application/Save夹具。
peer线程拥有调用者传入的有界源码副本，不泄漏static字符串；正式输出/授权/缓存/离线及生产owner不变。
无DOM类型正反例与原生运行共同验收；类型/隐藏Application不替代可见产品、资源型完整SDK或最终包门。
本批TypeScript5.9.3正反例及JS语法、82项去重定向运行通过；公开示例显式ignored门已另实际执行，
包含200/503后各自停服、同profile新Application有限离线重开与独立2x2/42正式输出和Save恢复。严格Clippy/lib构建通过。

v456服务失败隔离由art_service_failure拥有固定暂态错误分类、原授权复核和失败链的一次有界清理；
art_service_http::Call::abandon消费原Call，只允许同owner取消，不恢复submit/poll或增加原期限。
dispatcher仍串行处理服务；认证/身份/服务wire/TLS信任错误停止实例，暂态错误只拒绝该Promise，不切换离线。
专属signed child/TLS测试保护取消、显式新调用及迟到输出；Application测试复用完整包夹具验证原结果/Save链。
本批119项去重定向及1项实际产品Canvas worker通过，严格all-target Clippy/lib与产品构建通过；
新增夹具已按原结果单槽等待输入恢复，原失败保留，不改变产品背压。1项授权producer夹具ignored未运行，不计通过。
这不是普通EXE可见菜单、真实Loom、资源型完整SDK、跨端或最终整包验收；三副线冻结实现未变。

v455新增完整包Application生命周期验证：application_local_art_package_tests复用tests/local_art_runtime的同份包/TLS夹具，
覆盖交互队列、服务37%进度、独立2x2正式图像/42值、保存恢复、无Device Session离线重开及shutdown；不复制生产实现。
art_local_owner/window仅新增cfg(test)窗口访问器；产品菜单、授权、正式结果及保存仍由原owner负责，没有产品测试后门。
tests/local_art_runtime分别拥有独立profile/自启进程、包签名、TLS、UIA和独立PNG/checkpoint oracle；UIA接口先于COM apartment释放。
本批30项定向运行、严格all-target Clippy及两种测试目标链接通过；真实普通EXE可见菜单门仍受前台准入失败阻断，
保留原AV/拒绝结束的自有旧测试PID现场，不提高权限，不以隐藏Application替代前台/真实Loom/跨端/最终包验收。

v454资源包闭环已接通并完成选定运行验收：art_execution_package拥有独立包证明和资源DTO；art_prepare_package在原worker逐项验签/下载，
art_package_cache把资源分块接原32项/32MiB缓存，art_package_content只合并已验证资源到原Content/图片安装。
原prepare/resume/issuer owner继续独占授权、设备、期限、持久减权及离线接纳；Loom仍独占包路径/resolver/签发。
本批Otter668项相关passed/5默认ignored、Loom40项相关passed/5默认ignored；三项HTTPS联合入口另显式通过，含完整资源包停服跨进程恢复及缺块拒绝。
后续只修显式ignored联合夹具，未变主体按哈希复用；实际双仓产品构建及Otter Canvas worker通过。Loom严格Clippy仍有未改文件旧诊断，不称全仓全绿。
本批不等于可见产品UI、完整SDK、跨端、长稳或最终手测包；详细输入/失败/复用边界见main-art-package-resources-1。

Art离线恢复扩展由art_execution_resume拥有独立签名域与稳定身份/新宿主绑定准入，旧grant不得换绑；
art_prepare_resume拥有显式可恢复在线准备，art_issuer_resume只从当前受信pin和已有profile设备身份取完整源码。
art_offline_store拥有32槽/1MiB的恢复凭证、启动时钟高水位与减权记录，复用原原子文件提交；不缓存Device token。
device_authorization_local是原Lease的纯本地停止凭证，没有Session且全部网络出口拒绝；原Device路径不变。
Tasks仍共用两槽后台链，OpenArt/Runtime/正式结果/Save保留原owner；授权清除/切换在无View借用时退休本地窗口。
显式清除沿原四槽授权worker持久减权，已接受意图不被退出取消；普通Select只停止会话，不永久撤销全部离线许可。
身份Missing与锁冲突/损坏分开，后者失败关闭；同毫秒旧票也受origin cutoff拒绝。原设备批准状态不变。
v453已通过Otter643项相关回归、Loom33项相关回归及两项显式HTTPS联合门，另产品Canvas worker通过；
跨进程离线门确认无Device Session、服务固定offline失败后仍产生正式42结果。Loom严格Clippy仍有未改文件旧诊断。
本功能仅覆盖单入口源码和原有效期限，不等于完整资源包离线、可见产品UI、跨端、长稳或最终产品包验收。

持久Art源码缓存由art_source_cache拥有有界磁盘项/独占锁，art_source_cache_file拥有同目录原子替换；
art_prepare_cache仅在新请求完整验签后写入，并通过显式prepare.v2引用源码，不保存grant或设备凭据。
art_local_tasks在既有后台下载worker串行describe→prepare，不增加UI磁盘IO/线程/采样队列。
缺失/忙/损坏缓存走原在线链，只有v2端点明确404回退v1；权限/签名/网络错误不能离线降级执行。
本批230项Otter去重定向、12项隐藏Application、真实Loom HTTPS联合门及显式产品Canvas worker已通过；
Loom相关19项通过，严格Clippy仍有未改文件的46/55项旧诊断。源码缓存不等于授权跨重启、离线或最终包验收。

Art图像处理闭环新增受控图片安装：surface_javascript_images拥有私有描述/二进制PNG门，
surface_javascript_mount拥有原Session准入及共享绝对期限，child在首条install后才接mount。
surface_javascript_canvas_images持有不可变解码输入和各target位图，和Canvas背板共享原32MiB逻辑预算；
JS私有images工厂只生成当前会话句柄，Canvas drawImage复用原D2D，正式结果仍由原Results/Save/Checkpoint提交。
loom_inline保留同份预算PNG，PreparedBatch仅克隆Arc交给Art，不重解码/复制巨大Base64；小图resource()合同保留，
大图使用image()而不提高512KiB内联门。此闭环严格静态门和279项去重定向测试已通过，含11项真实隐藏Application及2项显式产品worker门；不等于完整SDK/最终包验收。

本地 Art 图输入由 art_local_images 复用 Application 的图解析及 loom_input_job 后台PNG准备，
art_local_mounting 在原下载token/宿主/来源凭证全部通过后才启动原签名child；退休取消并排空原worker。
snapshot.state.inputs 的端口描述关联独立安全resourceId，Content沿原mount交付；小图可用NeuroSurface.resource，全部已安装输入可用NeuroSurface.image。
来源像素/正式OutputStamp/Live身份在呈现及结果领取时重验；大图不缩小、不绕过二进制和解码预算。
前一批206项定向验证补齐了图输入接线，本批在其基础上覆盖Canvas正式图像及保存恢复；既有真实Loom联合夹具本批未运行。
完整图像处理SDK的其余能力、完整包内资源缓存和最终包仍未完成；有限单入口授权恢复见上节。

公共legacy采样现由 editor_crop_sampling 返回有界 Projection（工作纹理source区→完整逻辑target区），
不再反解极窄选区的巨大整图placement；nested crop只改变clip，保存直接读取source区换基准。
raster_projection::project_region、erase_frame 与 canvas_raster_projection 分别复用既有CPU投影、
逆映射擦除及D2D源矩形绘制；blur/resize/cache仍复核同一映射。有界采样批次478项去重相关测试通过。

art_local_dependencies 是本地 Art owner 的输入拓扑凭证：捕获有界上游身份和输入连线，
在启动完成、轮询与正式结果领取时复核；无关节点或输出边不使实例退休。
application_local_art_node 已负责独立节点事务与正式结果接入；完整图像输入 SDK 仍未完成。
投送 export_projection_legacy_content 消费宿主已验证的 legacy_pixels，保持自然输出逐轴密度，
与现代目标 crop 分空间处理；snapshot 选择自然源或工作纹理并提供对应坐标，不由渲染器猜来源。
export_projection_active_blur负责活动效果采样/clip，history_mapping与mapped_erase负责有界历史空间适配；
它们是原投送出口的专属叶模块，不是第二套通用导出框架，公共来源/取消/checkpoint仍归Application。
PNG最终35项源码已集成；当前组合输入完整sender50及34份隔离TLS PNG/metadata验证通过，不代替跨端和整包验收。

editor_crop_sampling 拥有旧自然像素裁剪的活动采样矩形及复合坐标；状态内嵌 CropView，
沿既有裁剪历史 reset/undo/redo，不裁掉完整底图、不另建 Canvas 有效标志。
session_candidate 安装真实恢复视图，session_crop_write::merge_view 按活动模式写回或移除旧字段。
editor_crop_sampling 同时保留自然源尺寸并反解已 contain 工作底图的独立目标框；
raster_source_frame/erase_layers 复用既有内核消费底图与标注各自的框，Canvas 与 blur 缓存复核该映射。
工作底图失去 retained 自然源后，保存元数据必须同步换到工作纹理坐标。
公共采样批次已通过严格静态门与196项去重相关测试，含D2D/WIC像素门；投送消费、真实窗口与完整产品验收尚需完成。

本地 Art 正式结果批次进行中：surface_javascript_result.rs/.js 拥有独立 otter.art.result.v1 请求、每 operation 单槽及当前Canvas引用解析；
Session默认拒绝结果，signed Local仅在显式workspace_result许可下启用，旧emit(commit)不改义。Runtime原绑定/Lease/序号继续复核；
art_local_window在呈现/宿主复核后保留单份结果，未领取时停止安排新操作；art_local_results只向实际Art节点提交，不覆盖源截图。
art_local_result_state按原OutputStamp拒绝旧结果、原子替换端口，复用Save/Checkpoint输出采集与传播。Loom enum/schema/source同步显式权限。
Otter侧真实child、撤权迟到、Application到保存恢复等71项定向测试及严格Clippy已通过；Loom合同7项、daemon真实HTTP权限与直接回归13项通过。从截图创建独立Art目标及完整输入SDK仍未完成。

长截图录制专线 CAPTURE 持有检测/录制调度/专属控制窗口与测试，公共入口及交付由 MAIN 集成。
session_capture_long 是 session_capture 的持久来源 DTO 子模块，仅校验帧数、方向、坐标与轴约束，不实现图像检测；
capture_context 保留未知轴和原选区，desktop/open_job::start_long_context 复用原后台提交且先验证来源。
main-capture-seams-1 已通过 15 项来源/提交定向测试、lib 构建、严格 Clippy/格式/行数门。
CAPTURE input-3的28文件及MAIN剩余公共入口现已合入主仓：long_capture_detect拥有图像证据轴/方向，long_capture_recording及专属hook/schedule拥有有界录制，timer失败沿dialog关闭/撤销原任务。Ctrl+3优先既有完成路由再进入录制，手动/主动采集入口保留。主线组合构建/严格Clippy与124项去重定向测试通过，含四方向真实自建窗口像素门、目标移动拒绝和Art保存恢复；物理滚轮与最终整包仍未验。证据main-capture-integration-1。

art_service_progress是宿主专属固定大小最新状态槽，仅状态/0..100百分比进入已有UI状态，不透传服务任意JSON文本；
dispatcher沿原Call授权/序号后发布，Runtime领取前复核原绑定/Lease/期限，正式帧领取与下一操作清槽，停止不可逆关闭。
art_local_window只把进度作为状态，不提交Frame；art_local_owner把文案绑定请求token，换代/退休清理旧执行中文案。
main-service-progress-1 input-2服务22、进度槽3、Runtime8、art_local27、隐藏Application1及严格Clippy通过；
真实服务进度可见UI联合验收与正式结果/SDK仍未完成，不将后台回环TLS通过表述为完整产品验收。

HLSL产品接线批次已通过定向验收：desktop/shader_native_invocation仅适配已有后台输入到专线Request；
shader_invocation为单请求选择Native/Legacy，shader_native_image拥有straight PNG/PBGRA结果分流；
shader_native_route_tests覆盖显式ABI、旧配方拒绝及真实child到正式PNG接缝。默认原生选择与Loom生产者已接线，
application_native_shader_tests和tests/shader_startup覆盖隐藏Application/实际EXE取码、缓存重渲染和PNG提交；
运行门与最终静态/输入审计通过，见main-product-hlsl-1/STATUS.md。
随后ANGLE生产入口退役批次由MAIN持有：shader_startup拒绝旧候选，main拒绝旧worker，
shader_angle_library的实际DLL加载与shader_process旧child通道只编译进测试目标；历史源码/材料不删除。
Archive.ps1拒绝原生ZIP中的两种ANGLE DLL。本批产品边界/原生实际EXE/Application及静态门通过，
合成ZIP合同9通过；回执见main-angle-retirement-1，不提前宣称最终包验收。

Shader专线input-6已按冻结交付合入主线：shader_hlsl及17个子模块拥有原生HLSL合同、D3D12/D3D11、
受限worker和定向测试；Cargo Fxc、lib模块和main私有入口已注册。交付源码保持原始字节，主线不重复实现底层。
desktop/shader_prepare、shader_image和默认启动由MAIN集成，专线自身验证不替代产品和ANGLE退役验收。

device_authorization_execution 是原 Lease 的本地 Art 执行停止子模块：宿主停止中断执行网络与后台等待，
认证撤销同时中断执行和清理。art_service_dispatcher 在同一后台线程最多一次取消原 callId，原认证/期限失效即拒绝；
Runtime 结束后撤销整个 Lease。art_service_runtime_tests 保护 submit 回复尚未到达时的关闭、撤权和迟到结果拒绝。

software_dib 是软件呈现和 child Canvas 共用的线程绑定 DIB 叶资源，保留 desktop/software_bitmap.rs 历史源码路径，
只在 lib.rs 编译一次，desktop/software_surface 通过模块别名复用。DIB 指针不外泄，copy_into 严格匹配尺寸并排空 GDI 批次。
surface_javascript_canvas_backing 只拥有软件 D2D DC target、逻辑/中间位图预算和 DIB；target 先于绑定内存释放，
保留原 Canvas 绘制/PNG 协议、透明初始化、粘性失败及 250ms 门，不建立第二套 GPU 呈现后端。

surface_javascript_services 拥有后台 Session 的私有服务往返与 dispatcher 接缝，默认 Session 没有派发权；
surface_javascript_service_wire 只传受限服务名/JSON/局部序号，不接受 grant、设备或工作区身份。
surface_javascript_services.js 私有闭包拥有每 operation 单个待决 Promise 和最多 32 次调用；
surface_javascript_engine_services 只在 JS 栈退出后等待宿主，保留原执行/布局/Canvas 累计预算，不重置 begin。
父子通道的中间服务回复沿同一 operation 序号，Session 仅在最终 Output 后生成 Frame，失败/取消由原 Job 回收。
当前正在验证真实 child 与引擎测试；signed Local 的原 Permit/Lease 到 dispatcher 接入尚未完成，拒绝门保持。

art_prepare::service（art_service_http）拥有原 Prepared/Permit/Lease 借用下的串行 submit/poll/cancel，
art_service_wire 只消费独立服务 DTO 与 owner/序号/终态/大小约束，不复制 Loom 包管理实现。
调用 ID 由宿主生成，不从脚本或响应获取；失败不自动重发，终态缓存交付也重验授权。
art_service_http_tests 与 art_service_wire_tests 保护真实 TLS 和 wire 边界；当前尚未接 QuickJS Promise 续接或解除 signed Local 服务拒绝门。

本地首帧视口由 art_local_window::start 的隐藏禁输入 HWND 提供，经 Prepared/signed Runtime/Input 传给 Session::mount_in；
has_frame 只在帧呈现事务成功后置位，空壳不计 ready、不显示。Application 签名夹具在首条 mount 读取布局，保护整条接线。
Shader 替代独立对话拥有新增 shader_hlsl*.rs 与专属 fixture；公共呈现/设备恢复及产品集成仍由 MAIN 单一负责，详见共享协调目录。

v450（进行中）art_local_description 只拥有宿主描述请求与串行 describe→prepare，复用 art_local_requests 的登记/撤销事务。
surface_javascript_signed_runtime/mailbox/thread 拥有新许可的后台 Local、单槽命令/帧、撤销/child/期限等待；
desktop/art_local_window 只拥有首帧后的 HWND 和呈现前后宿主复核，不借用旧 AuthorizedWindow 的远端激活状态。
desktop/art_local_owner 拥有工作区 wire 身份、宿主配置凭证、下载和待回收窗口；application_local_art 只接用户明确的 HTTPS OpenArt 意图。
Runtime 通知复用 execution_wake，不突破主循环句柄预算；恢复关闭在所有权交换前执行。
对应 runtime/window/owner tests 保护真实 child、首帧显示和旧工作区完成拒绝；当前尚未完成集中验证，不作为已验收功能。
surface_javascript_signed_runtime_lifecycle_tests 保护首帧/终态通知的发布顺序和 UI/worker 共享时钟高水位；
art_local_owner_tests 另覆盖退休旧 child 终态不能取消同节点的新下载请求，不以此替代完整 Application 成功路径。
application_local_art_peer 是测试专用回环 TLS 设备授权/Art 描述/签名源码服务；application_local_art_tests
从真实 Application OpenArt 进入本地首帧，并验证配置变化、宿主锁定与关闭回收，不使用生产凭据或在线 Loom。
本地交互接线进行中：art_local_window_input 拥有单槽 deferred click、原字段 FIFO ticket、视口/焦点代次及 rAF/timer deadline；
art_local_window_present 拥有帧提交前后复核与 ticket settle/focus/complete 顺序。授权和 child 仍归原 signed Runtime，
服务和正式结果未开放；本地窗口与 signed 回归共 41 项通过，扩展前台焦点验证失败，产品交互验收仍未完成。
Tasks 两种入口共用槽位、线程和完成接纳；描述联网前已经挂到 Authority 的原 Lease，禁止 UI 同步描述及伪造包占位。
art_local_description_tests 与 desktop/art_local_description_tasks_tests 分别保护登记、换代、容量和失败归属；不单独代表 Application/UI 验收。

v449 art_describe_http 消费设备认证的精确已审批包描述，保留原 Lease；响应不拥有 origin/device/workspace/instance。
art_execution_grant::validate_expected 复用请求绑定与范围约束，不签发 Permit；art_describe_tests 覆盖严格字段和宿主注入。
真实 TLS 测试先 describe，再经 Tasks prepare/验签/child；Loom local_art/describe 复用审批、resolver 和源码检查，不另建目录授权。

v448 desktop/art_local_tasks 拥有至多两个本地 Art 后台下载线程，复用 CompletionSignal 和原 Registry。
配置准备/下载/线程异常统一回到原 token，已结束未领取及清理中的任务继续占槽；shutdown 先撤销所有链再 join。
Registry retain_units 释放已删除节点代际预算并撤销原 Lease；真实 TLS 测试从异步任务进入签名 child。
Application 目录的精确本地包元数据与 wire workspace 身份尚未接通，不能把此 owner 视为图节点或窗口挂载完成。

v447 art_local_requests 复用 graph_requests 的 current token/代际，拥有本地 Art 原租约与一次性交付。
Pending 在后台经 Authority 下载，成功和失败都带原 token 回来；旧失败不能取消新请求。clear/Drop 撤销原租约。
art_local_requests_tests 保护替换、重复交付、失败归属和工作区清理；TLS 测试经 Registry 进入真实 child。
Application 图节点调度、节点删除时 registry 保留集同步和工作区恢复接线尚未完成，不替代既有 graph_requests owner。

v446 Application 持有进程内 art_issuers，启动配置安装与已确认的设置导入更新其信任，退出析构撤销自有请求链。
application_art_issuer_confirmation 只拥有导入信任差异的指纹提示/默认拒绝模态，不从网络自动建立信任。
导入复用原后台文件任务与 owner 检查，模态前后核验来源/窗口/设置；普通命名导入不能静默提升执行权。
application_art_issuer_tests 用真实 Application/隐藏 HWND 验证启动、拒绝、接受、移除与退出；确认结果注入，不冒充模态视觉验收。

v445 art_issuer_trust 拥有显式 origin→issuer 配置与进程内信任 owner，签出绑定原设备 Lease 的后台 Authorization。
轮换/移除/owner Drop 撤销相关租约；签出后不持 owner 借用跨网络。配置复用 app_settings_document 和 settings_update
的保字段与基线替换事务，不新建第二套身份文件。art_issuer_trust_tests 保护配置验证、持久化、冲突与撤销；
art_prepare_http_tests 从 Authority 进入真实 TLS/child 链路。尚无确认 UI 或跨进程文件监视。

v444 art_prepare_http 复用 Control 的固定 origin、Device nonce 和 Lease 网络取消，拥有有界 prepare 响应解析、
独立 pin 验签及消费式 Prepared。原 Permit 随准备结果移交到 signed Local，保持首次准入的单调期限。
art_prepare_http_tests 使用真实回环 TLS 与真实 child；不建立 issuer 信任、不持久缓存、不冒充 UI/工作区 registry。

v443 art_execution_grant 与 Loom 合同统一真实 publisher/art 包身份及路径别名拒绝规则。
认证设备、显式审批、精确 immutable 包解析和源码签发归 Loom daemon local_art；Otter 不复制包管理实现。
本批不新增客户端自动下载、issuer 信任或 UI owner；生产 HTTP 测试不替代完整 Art 挂载验收。

v442 surface_javascript_signed 复用既有 Session/Job 资源预算与进程回收，拥有新签名许可保护的本地预览接缝。
它不复用旧资源 lease 作为执行许可，不建立 issuer 信任，不派发服务或正式结果；宿主窗口生命周期与闲置
到期唤醒仍须由后续 UI owner 接线。signed_tests 验证真实 child 的状态、操作前中后撤销、禁止输出和回收。

v441 art_execution_grant_wire 只拥有独立 loom.art.local.grant.v1 客户端 DTO；art_execution_grant 拥有
可信 issuer pin 验签、精确请求/源码准入和会话内不可逆 Permit。它不建立密钥信任、不下载包、不启动 JS。
art_execution_grant_tests 覆盖篡改、旧 token 拒绝、绑定/范围/时钟/离线/撤权及实际 Loom 签发字节互操作。
协议规范归独立 Loom 仓库 protocol/LOCAL_ART_EXECUTION.md；不得复制其包管理实现或改变旧 Surface v1。

v440 RenderSurface 将普通工作区和钉图的兼容路径统一为 D3d11 owner，移除隐式 HWND target 分支。
render_surface_selection 的 compatibility 统一类型化关联准入和惰性 CPU 兜底；Canvas 恢复不再把普通窗口当作
无 DComp 图而立即完成退休。原有恢复/菜单/像素测试同步验证两类窗口；无新增线程、设备缓存或后台绘制循环。
composition_workspace_transition_tests 复用显式可见夹具，独立验证普通重定向 HWND 的 D3D11→BitBlt CPU→D3D11
桌面像素与 144 DPI 边界；不以钉图 layered 提交冒充普通窗口可见验收。

v439 canvas_render_recovery 统一自动/兼容/CPU 的选择目的地与退休完成发布；render_commands 拥有三个选项和 CPU 重试。
render_window_visibility 拥有 CPU 分层→GPU 首帧间的临时可见性租约；procedure_paint 在绘制重入保护下调度，
无 Canvas 借用跨 Win32，失败也恢复原可见性，托盘/分组隐藏意图及宿主撤销优先。render_window_visibility_tests 验证焦点、
隐藏/最小化与失败收尾，canvas_gpu_reselect_tests 补生产窗口隐藏期间的销毁/分组重入。
canvas_creation_admission 仅以类型化未关联失败签发缺 target 的重试凭证，新尝试开始即撤销；未知 owner 仍拒绝。
software_window_style 增加 GPU 样式入口，procedure_paint 在无 Canvas 借用且 render_active 保护下先提交样式再创建候选。
canvas_gpu_reselect_tests 覆盖真实命令往返、选择状态、终态重试、候选凭证与样式重入；既有可见测试扩展为双向像素验证。

v438 render_commands 拥有当前文档的 CPU 选择/重试菜单与执行时再校验；Canvas 继续独占退休状态，
不允许 UI 清除未知关联。canvas_manual_render_tests 覆盖真实菜单、计时器、首帧与拒绝路径；
application_cpu_save_tests 复用既有隐藏 HWND/串行测试夹具，覆盖真实编辑、保存 worker、落盘与文件恢复。
没有新增全局偏好、线程或后端常驻实例；GPU 重新选择/关联失败后的完整宿主恢复仍是后续边界。
context_menu_shapes 接收既有自定义形状输入事务，避免菜单接线突破 500 行；保持模态 guard、
无借用跨对话框和取消/销毁后不应用的既有行为，由形状对话框/选项及菜单集成回归保护。

v436 procedure_software_present 拥有软件样式/首帧提交事务，借用外调用 Win32，按宿主身份与
Target/DocumentStamp 拒绝重入旧帧。software_window_style 增加稳定宿主 guard，撤销后不回滚旧 HWND。
Composition 拥有 D3D11 呈现失效标志，render_frame 仅观察 EndDraw 失败；Canvas 退休 owner 决定
兼容或 CPU 目的地。render_software 统一普通窗口/透明钉图的软件帧，canvas_cpu_recovery_tests 验证生产接线。

v435 canvas_render_recovery 独占恢复状态与退休 Target，普通 discard 不触及退休 owner；只有明确
Complete 才释放旧 owner 并允许一次兼容续接。procedure_render_recovery 拥有 HWND 定时调度与清理，
Canvas 不持有 timer/HWND；procedure_paint 拥有原错误，终态不再关窗。新增状态模型与真实窗口测试
分别由 canvas_render_recovery_tests、canvas_recovery_window_tests 拥有，复用现有隐藏窗口 fixture。

v434 smoke_timing 的诊断行增加 process/thread 身份，配对 id 仍仅在所属进程内唯一。
tool_panel/content 补定位、字体、首次投影计时，application_smoke 补共享捕获/快捷键/钉图阶段；
全部沿已有 opt-in 开关，不改变动作顺序、原错误、控件提前准备或验收期限。

v433 canvas_target 拥有 surface 已创建后的画刷准备与 Target 发布。准备失败时仍持有 surface，
先发起其合成退休并返回类型化准入失败；无 DComp 图则不伪造关联阻止。Canvas 仅在完整 Target
成功后提交后端选择标志，避免局部成功提前改变后续策略。故障测试继续由 canvas_software_tests 拥有。

v432 canvas_creation_admission 保存首个未确认关联错误并跨 discard_device/重绘拒绝创建；不持有 HWND、
GPU 图或文档，不提供隐式复位。composition/render_surface 的候选选择结果保持类型化至 Canvas，
procedure_paint 对该受阻状态消费失效区但不重试、不清错误、不排队关窗。正常绘制错误的旧策略仍待迁移。

v431 composition_creation 只拥有创建失败的原错误和候选准入凭证，不持有 HWND、文档或 GPU 图。
D3D12 new_candidate 与 D3D11 with_device_owner 在关联失败时签发拒绝回退凭证；两层选择器消费它，
不按 HRESULT/消息文本猜测关联状态。直接设备调用入口仍返回原 windows::Error，不增加自动候选。

v430 的 composition_d3d12 在完整 ManuallyDrop owner 建立后才执行 SetContent/SetRoot/Commit；
关联失败沿用既有退休 owner，不把资源保留责任留给局部变量自动析构。
composition_d3d12_creation_tests 专用于独立进程中的部分关联失败及原错误/设备租约/图预算保留门。
该构造事务不拥有候选切换许可，创建失败后的 HWND 准入协调仍待接线。

v429 RenderSurface 统一派发退休请求，后端仍拥有状态和资源。composition_software_transition_tests
按同一物理文件注册到 desktop，不再嵌套于 D3D12；共用测试流程覆盖两个后端的真实 RenderSurface
绘制/退休及同 HWND CPU 像素验证，不复制 D3D11 专用绘制实现。
canvas_render_recovery 持有显式退休转交入口但不丢弃 Target；startup_smoke 在隐藏自检末段消费它，
验证文档和确认计数未改变后再销毁。默认消息循环尚不调用这段自检轮询。

v428 将 composition_retirement 注册到 desktop，以便两后端共用同一 worker 准入计数；
composition_d3d12_lifetime.rs 按既有物理路径注册为 desktop::composition_lifetime，窗口图预算由两后端共用。
Composition 私有 State 统一持有 D3D11 全部资源与租约，ManuallyDrop 允许未确认时整体保留；
composition_retirement_owner_tests 专用于独立进程中的失败凭证/资源保留验证，不混入普通共享设备回归进程。
RenderSurface/Live 拥有实际 BeginDraw 前准入，relay 保留 resize 前置入口；资源 getter 不是绘制授权。

v427 的 composition_software_transition_tests 继续拥有显式可见同 HWND 迁移门，现直接消费
D3D12 owner 的异步退休接口，不再复制手写同步解绑序列；Pending 只处理自有窗口消息。
本轮无生产模块移动；D3D11 的直接 renderer/Live/relay 绘制入口与完整资源保留尚待接线。

v426 新增 composition_retirement 与 composition_retirement_tests，嵌套在 composition_d3d12，
仅负责有界准入、DComp 解绑/完成等待与单槽结果；不持有或迁移 GPU/D2D/Rc owner。
CompositionD3d12 保持完整 State/租约及单向退休状态，未确认时保留图，确认后仍分别执行 GPU drain。
尚未接入自动恢复或 D3D11，不把这个 owner API 当完整 CPU 产品链。

v425 新增 render_software，拥有 Canvas 软件离屏绘制及待提交帧；canvas_submission 按 Target 身份与文档
凭证确认提交。procedure_paint 从 procedure 拆出 WM_PAINT owner，释放模型借用后提交、复核宿主并处理
嵌套 deferred，原错误策略保持。canvas_software_tests 覆盖真实 Canvas/生产消息入口与过期确认拒绝。
普通工作区的旧 HWND target 创建失败后才进入软件链；钉图样式迁移和最终保文档错误状态仍未接线。

v424 将 software_submission 从 software_surface 拆出，独占一次性 HWND 提交及不可变 DIB 租约；
software_surface 继续拥有 D2D 绘制/resize，待提交 DIB 禁止覆盖，新 DIB 按共享预算分配。
software_submission_tests 覆盖租约取消/失败、resize/target 释放后的像素寿命及真实 WM_STYLECHANGING 重入。
尚未改变 Canvas/WM_PAINT owner；产品接线必须在释放模型借用后提交，成功后再确认呈现计数。

[统一修订计划](WINDOWS_NATIVE_REVISED_PLAN.md) 拥有新目标/顺序/验收，不复制业务实现。
v412 的 application_projection_sender_geometry 只捕获 fallback 几何并复用原 Document 围栏；
export_projection_geometry 在原 worker 换算逻辑框，不接管 renderer、网络或 Save owner。
两原生后端应共享上层语义，Art SDK 通过版本化合同调用 Loom，不把独立仓库内部实现搬入 shared。
具体新后端/SDK 模块随真实实现确定，不以本计划虚构已存在的 owner。

v416 的 composition_d3d12 继续拥有窗口私有合成视觉透明度和资源创建入口；不新增业务 owner。
透明度仅更新 DComp effect，提交成功后才更新已提交值，提交链错误令候选失效以拒绝后续绘制。
composition_d3d12_tests 覆盖透明度输入、像素不变、resize 保留和失效拒绝；不把纹理读回当屏幕观感验收。

v417 的 composition_d3d12_device 拥有 UI 线程共享设备代际、弱缓存、队列/fence/event 和帧准入；
composition_d3d12 仍拥有每窗 context/双缓冲/合成树及失败保留，不共享文档或交换链。
composition_d3d12_lifetime 分别计设备代际与窗口图预算，保留图不归还配额；buffers 的 Frame
先 EndDraw/Release/Flush 后释放帧准入。tests 覆盖四窗共享、窗口独立及关闭中间窗口后的继续绘制。

v418 新增 render_surface_selection，拥有 Canvas 创建事务中 D3D12 优先/兼容回退及错误诊断；
原 composition_selection 继续只拥有 D3D11 hardware/WARP 链，不混入另一套业务 owner。
render_surface 按后端派发资源创建、透明度、resize 与帧提交，D3D12 不嵌套 render_frame。
canvas_d3d12_tests 验证真实 Canvas 的工作区/钉图、DPI/resize 及丢弃 GPU 资源后文档保留。

v419 新增 canvas_render_recovery，仅拥有单向降级选择和 GPU Target 退休，不新增文档/保存 owner。
composition_d3d12 报告其资源失效，render_surface 转译为是否需要兼容路径；procedure 保持消息调度及
原 HRESULT 日志，降级重绘至多触发一次。Canvas 普通 discard_device 不重置兼容选择。
canvas_d3d12_tests 扩展真实 EndDraw 错误到兼容重绘、选择保持和文档凭证测试，业务闭包错误不降级。

v420 新增 software_bitmap，独占 GDI DC/DIB/旧选入对象及对应像素预算，释放前恢复选择并排空 GDI 批次。
software_surface 拥有软件 D2D target、内部位图高水位预算、帧/resize 和 HWND/分层窗口提交；
不创建 D3D/DXGI/DComp 呈现资源，也不擅自修改 HWND 样式。software_surface_tests 覆盖像素与隐藏窗口边界。
候选接口尚未接管 Canvas；窗口样式切换、失败后保文档及软件模式能力报告仍需接线验证。

v421 新增 software_window_style 及专门测试，嵌套于 software_surface::window_style；只拥有可重定向 HWND 的
WS_EX_LAYERED 事务，写入前拒绝 NOREDIRECTIONBITMAP 窗口，不接管 GPU 解绑、替代 HWND、Canvas 或文档。
Win32 样式调用可重入窗口过程，调用方必须释放 State/Canvas 借用并保证窗口生命期；未满足前不得接入 paint。
提交前核对样式，失败/异常回滚仅恢复分层位，其他 owner 的位保留；回滚失败报告原错误，不无限重试。

v422 host_lifetime 继续拥有宿主登记计数，新增 WM_NCCREATE 的精确 HWND 绑定与匹配 NCDESTROY 释放；
procedure 在业务路由和工具面板刷新前校验身份。host_binding_tests 覆盖真实重复创建失败隔离、
无 State 候选的销毁和原宿主继续绘制，不新增文档 owner，也不把初步保护当宿主替换完成。

v423 新增 composition_software_transition_tests，嵌套于 D3D12 owner，只在显式 ignored 门中显示自有色块
并检测同 HWND 的合成→软件切换；不新增生产恢复/窗口替换 owner。software_surface 与 window_style
允许 pinned 保留 NOREDIRECTIONBITMAP，但普通 BitBlt 仍拒绝，现有软件模式未接入默认 Canvas。

## v411：稳定发送关联显式停止

新增 sender/projection_sender_stop，在原 Store 内拥有显式减权、持久 unlink 回执与精确 ack；
projection_sender_binding_stop 只拥有保留原文的 pending/移除 patch 与 canonical ack 凭证，不另写文件。
desktop/application_projection_sender_stop 和 stop_job 分别负责单槽状态协调与后台日志/metadata 工作；
仍经 Record::edit_original 与原 Save/Checkpoint owner 发布，不把候选、手动另存或 HTTP 成功当最终完成。
菜单直接持有精确 Binding/canonical Source，不借刷新目录选择另一个目标；普通取消/换服务/退出不扩大为稳定解绑。
新增模型与隐藏菜单/TLS/重启测试分别归属 projection_sender_stop_tests、application_projection_sender_stop_tests。
本轮同源定向 lib 门通过，startup 正式集成仍有一项原 15 秒超时；实际证据与边界见 [v411 构建记录](WINDOWS_NATIVE_BUILD.md#v411稳定发送关联显式停止)。下方 v410 为历史。

## v410：投送目标编辑与外观归属

application_projection_sender_snapshot 分离权威底图与目标 editor：resolved 继续提供原底图，
canvas.editor 独立提供发送目标的已提交标注、栅格覆盖层和当前 crop/reset，不借上游 editor 接管目标编辑。
新增 desktop/export_projection_target，只在原发送 worker 中从 canonical Source 按目标 UnitId 借用读取纯外观；
不解码图片、不重建工作区、不把 crop/flip/erase 当作可无条件重放的外观。export_projection 仍复用原
source_resolution/边框/全栈圆角/beautify 与最终尺寸预算，不增加 renderer、Job、网络或持久化 owner。
目标无 logical frame/raster 时由该借用读取器提供 canonical 展开框尺寸回退，缩略态使用 savedRect，不取上游自然像素。
新增 application_projection_sender_target_tests 与 export_projection_target_tests 分别拥有生产链归属和后台读取/合成回归。
完整历史编辑、传播布局与 PNG parity 仍未完成；验证范围见 [v410 构建记录](WINDOWS_NATIVE_BUILD.md#v410投送目标编辑与外观归属)。下方 v409 为历史。

## v409：本地保留原图生产捕获

新增 desktop/application_projection_sender_snapshot，独立拥有发送快照的权威像素选择与 Weak 像素凭证，
不接管图源恢复、持久化、网络或另建 renderer。Source::matches 必须对活动 editor 调用后才导出 editor 快照；
选定图源显式交给原 export_projection/source_resolution，不允许后台再以 editor.layers.base 覆写 natural source。
Scope 继续拥有工作区/原文档/显示凭证，额外组合像素凭证；pixels::downgrade 只暴露 crate 内 Weak，不暴露可变字节或预算 owner。
application_projection_sender_source_tests 拥有对应生产菜单/TLS 与快照身份测试，复用原 TLS/保存/重启夹具。
resolved/target metadata 的完整分离不在本切片中，不把目标 retained 源注入 resolved owner。
规模及验证见 [v409 构建记录](WINDOWS_NATIVE_BUILD.md#v409本地保留原图生产捕获)。下方 v408 为历史。

## v408：有界源分辨率编码出口

`desktop/export_source_resolution` 继续拥有源像素/逻辑几何、D2D 合成与美化调度；
`desktop/export_source_png` 拥有最终 source/inner/outer 准入、HGlobal 编码预算及 PNG 大小/取消围栏，
不增加文件、图片恢复、网络或工作区 owner。独立预算测试与源分辨率发送测试分别归属同名测试模块。
PNG 出口消费原 ComposedImage，编码后立即释放位图/输出 lease；只在 stream 拷贝期间继续持有编码 HGlobal lease。
`desktop/export_projection` 显式选择当前已提交底图后调用原合成链，不绕过当前编辑去借旧 retained source。
生产 retained-source 与目标 metadata 捕获尚未完成；不复制第二套合成器或全量重建活动工作区来代替。
`desktop/application_save_live::owns_saved_unit` 统一自动保存准入、快照及 receipt 的 Live 成员选择，
不修改 Session::belongs_to 的预入图语义，也不承担退休 worker 的 reap；后者继续归原 Live lifecycle owner。
本轮规模/验证见 [v408 构建记录](WINDOWS_NATIVE_BUILD.md#v408有界源分辨率编码出口)。下方 v407 为历史。

## v407：单目标发送生产接线

`projection_sender_flow` 拥有原 Job 内的 preparation/Request tracker 交接；`projection_sender_admission` 拥有发送前会话准入，
`desktop/export_projection` 只复用原 export 合成并约束 PNG 资源，不增加文件、DIB 或持久化 owner。
`desktop/application_projection_sender` 按 start/retry/scope/profile/poll/lifecycle/cleanup 分拆责任，
`desktop/device_projection_sender_ui` 只生成固定来源菜单与命令；网络继续由原 Sessions/Job/Lease/四槽执行。
Binding 持久化仍只有原 Save/Checkpoint owner，Record.source stale/rollback 与 canonical binding_source 不混用。
Request 在结果过滤前移交；本地持久取消后台有界串行，原 CancelSend 只在当前原设备授权下清理，稳定关联不随退出全量解绑。
完整 outbox 基线核验位于原 worker 返回前，UI join 不做大日志 I/O；后续 Ready 验证不因领取 metadata 而获得网络许可。

本轮 source/测试文件规模与验证结果见 [v407 构建记录](WINDOWS_NATIVE_BUILD.md#v407单目标发送生产接线)；
PNG 完整 parity、设备组/持续发送、稳定 binding 显式停止及完整迁移仍未完成。下方 v406 为历史。

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

projection_receive_attempt 归原网络 Job，独占本次原票据回收交接；
projection_delivery_work 仍只分派协议，journal 原子区分新建/复用。Sessions 仅有一项
无许可交接，Application 在正常/终态结算中移交原 cleanup owner；不增第二网络、
身份、工作区或 journal owner。abandonment HTTP、panic 和 Application 测试分别保
协议/资源许可、精确回收和生产菜单/取消/切来源/终态边界，复用原 CA/身份/窗口夹具。

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

projection_receive_cleanup 只拥有原 journal 的单向减权标记。Application cleanup owner 保原 Binding 删除证据/原 Ticket/退避；mark 只后台写盘，dispatch 复用原 Sessions/Job/Control 网络单槽。原 checkpoint 保完整索引和工作区身份，既有 profile/restore/save/shutdown 仅接线；无第二网络、凭据或工作区 owner。

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

accept UI 只拥有确认/Arc/发起窗口围栏；placement 只拥有新图初始几何；present 复用原 Document 凭证和共享 Pixels。receive_display 仍是唯一 receive 状态的 child，projection_delivery_work 仅从原串行 Job 提取协议分派，没有第二 network/workspace/journal owner。

最终严格静态/3799 文件 ratchet 零违规/警告/新例外；同源八件 45 选定门全过。
24 变更/7 新增、最大 381 有效行，UTF-8 无 BOM；visible 3/0、present 5/0、
placement 2/0、确认 1/0，过滤组不累计覆盖。344 旧件/七诊断、开发失败/输入说明、
最终冻结输入/七文档与源码三次匹配见迁移台账 v393/closeout。startup 7/0 不关旧根因，
正式前台菜单仍 ignored；无真实时钟到期/worker panic 全链注入，不代完整迁移。

v392 新增 projection_receive_hold 拥有原 Lease/Slot/活动 Ticket 的不可复制移交；
application_projection_receive 是唯一消费状态，scope 只拥有发网前目标 Guard，job
只拥有单个后台候选及一次回收。Job 独占 Hold，worker 用原记录的无许可快照，错误/
panic 也不提前释放槽/活动许可。复用原候选/追加/checkpoint/像素/工作区 owner，
共用 same_space 仅扩到 Application 可见，不复制算法；watch 单回调复用 Wake，无
第二任务池/等待句柄/轮询器。owner tests 与 TLS/profile fixture 分责，复用原 CA/
身份/窗口夹具。最终 25 变更/6 新增、最大 454 有效行，UTF-8 无 BOM；新 owner 227、job 142、scope
64、Hold 37、fixture 214/owner tests 263 有效行，各为单一职责。最终严格静态/3,792 文件 ratchet 零违规/警告/新例外，同源八件 41 选定运行门全过；
owner 10/0、watch 2/0，过滤组不累计覆盖。四份开发输入、最终 2,640 冻结输入/
七文档、首个静态候选/失败程序/析构诊断、336 旧件/五诊断与后续 344 件/七诊断保护
见迁移台账 v392/closeout。不加豁免，startup 7/0 不关旧根因，前台菜单仍 ignored。

v391 新增 projection_receive_binding 拥有原意图/设备/完整接收快照关联格式，
projection_receive_checkpoint 只拥有真实不可变原文的有界关联索引，不授网络或
显示权。接收 worker 单次绑定 UUID；Candidate/恢复 Record、save Output/Prepared
仅移动同源索引。checkpoint 是唯一提升 owner，只有原事务真实提交/规范启动才
提升，Application 查询只加当前工作区围栏；profile/菜单只作非破坏性短状态分类。
没有第二保存/身份/网络 owner，无新依赖/线程/预算或豁免。新 binding tests 与
Application checkpoint tests 分责复用原 PNG/目录/隐藏窗口/TLS 接缝。最终 28 变更/
4 新增、最大 349 有效行，新关联/索引各 65、模型测试 158/Application 测试 210，
UTF-8 无 BOM。严格静态/3,786 文件 ratchet 零违规/警告/新例外，同源八件 39 门全过；
关联模型 5/0、checkpoint 新组 5/0/全组 18/0、恢复菜单 4/0，过滤组不累计覆盖。
四份开发输入、最终 2,634 冻结输入/七文档、首个静态候选与 null 兼容修正后的
final-run2、328 旧件/五诊断及后续 336 件/五诊断保护见迁移台账 v391/closeout。
startup 7/0 不关旧根因，前台菜单仍 ignored；不把本地包含事实当完整接收消费者。

v390 新增 application_projection_profile，归 Application 投送子模块，独占后台单次
Store 初始化/长期租约/未结候选及回收边界的内存对账；不复制工作区/checkpoint owner。
device_projection_recovery_ui 归原投送菜单，拥有终态风险确认/候选准入，无 token/
磁盘/HTTP；Sessions 复用原投送单槽和 wait/drain，Ticket 重试快照不复制活动许可。
新 profile、菜单与 Application TLS tests 分责复用原夹具；字段/初始化/退出只薄
接线。最终 19 变更/5 新增、最大 453 有效行，新 profile owner 139/恢复 UI 113/
测试最大 316，UTF-8 无 BOM。严格静态/3,782 文件 ratchet 零违规/警告/新例外，
同源八件 37 选定门全过，profile 4/0/菜单 3/0/Application 解绑 8/0；过滤组不累计
覆盖。两份开发输入、最终 2,630 冻结输入/七文档、初始空过滤/夹具失败程序、
320 旧件/四诊断及后续 328 件/五诊断保护见迁移台账 v390/closeout。startup
7/0 不关旧偶发根因，前台菜单仍 ignored；无接受/显示或完整迁移验收声明。

v389 新增 projection_receive_journal/record，分别拥有接收写前存储/严格有界格式；
projection_receive_activity 只拥有同 intent 的短锁内存许可，Ticket/Ready 保其移动
生命周期。Store 由 profile 宿主在后台打开一次后共享，Job 不重复争同一独占文件锁。
profile_directory.pin 从 workspace_profile 原算法提取，不复制第二祖先/路径 owner。
HTTP 复用原认证 Control，协议模块提供 unlink body，只有 unlinked:true 后退休。
原 session 文件事务/恢复/预算、Lease/四槽、工作区/checkpoint owner 不变；没有第二
序列化/凭据/图片存储。journal/activity/TLS 测试分责并复用原目录、CA、身份和子进程
夹具。最终 16 变更/6 新增、最大 440 有效行，journal/record/activity 分别为
262/100/52 行，workspace_profile 121→72，UTF-8 无 BOM。严格静态/3,777 文件
ratchet 零违规/警告/新例外，同源八件 34 选定门全过；journal 14/0、认证补偿 9/0、
投送客户端 49/0，过滤组不累计覆盖。六份开发输入、最终 2,625 冻结输入/七文档、
312 旧件/四诊断及后续 320 件保护见迁移台账 v389/closeout。startup 7/0 不关闭
旧偶发根因，菜单仍 ignored；接受消费者仍未接入，不代完整迁移或最终用户包。

v388：projection_accept 只拥有相关响应/原 PNG 摘要，image_decode_projection
复用 WIC 像素 owner；projection_receive_ready 保原 Lease/Slot 到同 Grant 领取，
receive_session 只构造保 qrProjection 的单窗口会话，session_projection_candidate
复用原 session/矢量/State pipeline，不复制解码或窗口状态。session_json_fields 从
原库 candidate 提取有界借用解析；session_append 从原 retention 提取共同原文追加，
库特有快照匹配/条目消费不外移。application_append 从 frozen_append 提取唯一窗口
发布事务，冻结库只保选择/retention 接线。Source 继续拥有原文 allocation/预算，
没有第二保存、工作流、网络或凭据 owner；尚无接受菜单消费者。

最终 31 变更/12 新增，最大 367 有效行；新增生产模块最大 179、测试最大 257 行，
共同原文追加 109/窗口追加 120、旧 retention 141→83/frozen_append 130→44，UTF-8
无 BOM。严格静态/3,771 文件 ratchet 零违规/警告/新例外，同源八件 32 选定门全过，
含真实 TLS→隐藏 HWND→checkpoint→重启的原图与显示投影逐字节核验。大枚举仅
Box 化，不加豁免；首轮失败 exact 输入、304 旧件/四诊断及后续 312 件保护见迁移
台账 v388/closeout。startup 7/0 不关旧偶发根因，菜单仍 ignored，不代完整迁移。

v387新增device_projection_inbox_ui作为原授权UI child，独占有界菜单数据/请求；
composition_projection_inbox作为原Sessions child，独占唯一Pending Job与丢弃标记，
复用四容量/原wait/drain。application_projection_inbox独占准入/配置Lease获取与
结果投影，Application/授权/恢复/退出只薄接线；不复制网络、凭据、身份或工作区owner。
三个UI/Application/guard测试及严格TLS peer各保护自身边界，复用原窗口/CA/身份夹具；
原Sessions测试补共享容量/取消待回收/关闭禁启。最终有效行/ratchet/精确差异见迁移
台账v387，不加豁免/资源预算；仅元数据查询/拒绝，不代完整接收消费者或迁移验收。

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

三个新生产模块UI/远程Pending/Application分别105/77/119有效行；新UI、Application、
guard测试及TLS peer分别110/181/116/69行。Application450→452仅模块接线，授权结算
223→253仍为单一owner，原Sessions246→256及容量测试286→332，无新豁免或第二owner。

v386新增desktop/window_geometry_wake仅拥有每HWND合并queued结算通知的active/pending；
State/构造/窗口过程只薄接线，销毁禁投递，DefWindowProc默认WM_MOVE/WM_SIZE保留。
Application/保存槽/Receipt/checkpoint/文件事务owner不变，不增加线程/计时器/轮询。
新geometry tests复用原隐藏Application/profile夹具，独占sent-only→真实队列→落盘
和默认尺寸派发；wake tests保护合并/ack/失败/销毁/非几何负向。最终有效行/ratchet
与同源原产品链见迁移台账v386；不新增豁免、预算或第二保存/状态owner。

v386最终final-run严格fmt、全目标Clippy -D warnings/check、版本及官方ratchet
均过，零违规/警告/新例外；7份native变更/3新增，最大479有效行，UTF-8无BOM。
同源八件21个选定后台/隔离门全过：wake3/0、checkpoint8/0（含两项新隐藏HWND），
原workspace_runtime2/0/1 ignored完成移动提交→强制中断→重启恢复→删除最后Unit
落空；保原15秒和断言。正式菜单退出仍ignored未验，不关闭v384前台或旧完整组/
压力失败。三次源码/288旧件/四诊断/v385完整失败输入及原现场由closeout复核，
后续保护296件；状态window-geometry-wake-and-workspace-runtime-verified，不代完整迁移。

v385新增projection_delivery独占传统收件箱/拒绝回执合同，projection_envelope子模块
独占邀请形状与来源（非本地crypto验签）；projection_delivery_http作为原Control
child复用唯一认证send_limited。projection_delivery_runtime只拥有单次Job/完成
句柄，复用原4槽/Lease/取消/后台配置准备，不复制凭据/网络/身份存储或桌面owner。
三个tests分别保护元数据原子性、真实TLS/HTTP及任务/配置生命周期，复用原device
fixture，不建第二CA/进程夹具。未接桌面、accept/图片/呈现/同步或Tile；最终有效
行数、ratchet和同源运行见迁移台账v385，无新豁免或资源预算。

v385最终final-run2严格fmt、全目标Clippy -D warnings/check、版本及3,749文件ratchet
均过，零违规/警告/新例外。10份native变更/7新增，最大362有效行；四生产模块
168/111/29/128，三tests186/151/196，UTF-8无BOM。同源八件的13个选定后台/隔离
门12通过/1失败，projection16/0、device HTTP144/0（包含新9项，不能累计独立覆盖）。
workspace_runtime1/1/1 ignored在checkpoint-moved-window原15秒超时，尚未进入中断/
重启/删除链；失败profile仍revision7原坐标，stderr仅热键冲突，不足以确定根因。
菜单前台门没有重跑，ignored不计通过。三次源码/
旧280件/四诊断、首轮静态失败完整输入及288件后续保护见final-run2/closeout；
状态projection-client-implemented-workspace-runtime-pending，不代桌面/N09或完整迁移。

v384不新增/改动生产owner。workspace_runtime/exit继续拥有真实产品菜单退出与PID/
owner/visible发现；matcher子模块独占真实标准STATIC窗口的大小写正向及身份负向
回归，只用自身离屏NOACTIVATE HWND和RAII。没有第二进程/菜单/状态owner或行数
豁免；正式15秒等待、产品前台/保存门禁不变。最终有效行数、严格ratchet及同源
运行/源码/272旧件与四诊断身份见迁移台账v384，不将默认ignored算业务验收。

v384最终两份测试变更/一新增：exit194→197、matcher66有效行，最大197，UTF-8无
BOM。3,742文件ratchet零违规/警告/新例外；严格静态和18后台门通过，独立菜单门
在原15秒前台前提0/1，不继承开发1/0。三次源码/失败完整输入/八件/旧272件/四
诊断及280件后续保护见closeout，不以组织门或ignored替代最终原生/完整迁移验收。

v383新增application_exit_intent仅拥有一次性轻量退出批准；checkpoint的final_exit
子模块拥有最终等待/预期revision，仍共用原保存槽/Receipt/发布owner。LiveWindow/
Session只薄接自身完整RECT/透明度和待处理意图，不读取GPU或复制状态。Application
只保存一次性save批准，exit拥有确认/继续/销毁；autosave/open只保护poll后间隙。
退出主测试、意图/失败子测试及workspace_runtime/exit真实产品菜单测试分别保护
对应边界，复用原GPU/进程夹具。最终行数与门状态见迁移台账v383，不新增豁免。

v383首轮startup暴露完整restore idle越界到无profile退出批准。Intent只检查批准的
来源/持久模型/明确新意图；完整idle仍由profile最终捕获/提交/销毁边界拥有，不改
restore guard或保存Receipt，旧Application smoke继续保护退出清理execution。失败
完整输入/八件与final-run2同版本验证分开保留，不借清理夹具消除产品兼容性回归。

v383最终20变更/6新增，最大450有效行为Application447→450；Live window437→449、
exit143→200、checkpoint220→234。Intent/final_exit均64，退出主/意图/失败测试分别
131/116/176，产品菜单测试194行，均UTF-8无BOM。3,741文件ratchet零违规/警告/新例外；
严格静态及18选定后台/隔离门通过，首轮失败输入/八件另保。三次源码/当前八件/旧
256件/四诊断和272件后续保护见closeout；菜单仍未验，不把ignored当组织/迁移验收。

v382不新增生产owner。save_live独占Live自动就绪/轻量Guard，save_snapshot从实际
Receipt导出LiveStamp，save_state分离手动dirty与自动比较；checkpoint/autosave仍
唯一拥有各自磁盘基线/授权，原保存单槽/worker/编码/文件发布不变。Live window仅
提供自身手势idle，freeze/recycle只在可领取时消费一个请求；各终态归还槽后内部
唤醒。新增Live检查点tests拥有WARP/文件/静态恢复，autosave、意图生命周期、中断
子测试各负责自己的边界；复用现成GPU流/线程键盘/唯一test_process，不复制实现。
最终有效行、ratchet、源码/程序身份与定向实绩见迁移台账v382；不新增行数豁免。

最终18份native变更/4新增，最大437为Live window429→437；快照148、Stamp244、
checkpoint220、autosave161行。新增Live主测试246、文件autosave108、意图生命周期91、
中断92行，复用GPU/键盘/子进程夹具而不复制owner。3,735文件ratchet零违规/警告/
新例外，严格静态/17相关门通过，三次源码/八件/旧248件/四诊断及256件后续保护见
迁移台账v382/closeout；无超过500行结果或新增豁免，不扩大完整迁移结论。

v381新增desktop/workspace_profile独占默认工作区路径、祖先目录pin及进程文件租约；
application_workspace_checkpoint独占独立磁盘基线/轻量Stamp/调度/最终Unit收尾，startup
子模块拥有恢复屏障和默认来源的手动文件去授权。Application只保owner和薄接线。
保存job新增目的枚举而非第二槽；save_control继续唯一回收worker；Data只带可选磁盘
修订，session_write继续唯一编码/预算/回读。checkpoint不调用手动publish的dirty/
目标/workflow副作用。Mode/Runtime只组织启动依赖，原Session/State/文件事务owner不变。
profile/检查点/中断与真实EXE启动测试各负责对应边界，不复制业务持久化。最终行数、
严格ratchet及身份实绩见迁移台账v381；不新增豁免或扩完整迁移结论。

严格Clippy发现共享子进程夹具重复加载后，lib根只在cfg(test,windows)加载一次原
tests/support/process，文件中断/profile/checkpoint测试只引用。夹具仍是唯一子进程/
管道/超时/Drop owner，集成测试的独立crate边界不变；首轮失败完整源码另存后重验。

v381最终25份native变更/8新增，最大447有效行为Application432→447；desktop入口
416→443保持薄接线，检查点217/启动55/profile121行。生命周期tests318行只拥有
Application检查点/文件隔离/最后Unit，中断83行独立；真实EXE运行202、只读启动151，
UTF-8无BOM。3,731文件ratchet零违规/警告/新例外；严格静态、14相关门与三次源码/
八件/旧240件/四诊断及248件后续保护见迁移台账v381，不建第二保存/序列化owner。

v380的application_workspace_autosave唯一拥有共享开关/代际和自动保存调度；State只
共享Rc，Application只持有唯一在途标识。save_control继续唯一回收保存worker，新增
Pending/Stale/Ready区分供自动路径丢弃正常过期候选，手动路径仍将stale报告为错误；
save_publish/restore_publish只在成功换目标/工作区时失效开关。原guard的idle判定供
自动调度借用，不复制busy列表；save_ui/open_session/exit仅保意图和收尾薄接线。
新smoke只拥有linshi合成场景，tests保护取消/代际/冲突/打开/手动/退出边界；不新增
worker/定时器/序列化/发布owner。最终行数、静态与身份实绩见迁移台账v380。

首轮严格行数门发现application因薄接线494→501，未新增豁免；将完整new初始化职责
移至application_initialize，原Application字段定义/析构顺序、共享状态/任务通知建立
顺序与构造接口不变，不按任意行号拆方法。最终原生smoke和启动恢复共同保护该移动。

最终19份native变更/4新增，Application494→432，构造74、自动保存148、smoke158、
tests244有效行，最大432，UTF-8无BOM；3,723文件ratchet零违规/警告/新例外。
构造逐token/字段顺序核验、严格静态、10相关运行门与源码/程序身份见迁移台账v380
及closeout；没有第二状态/序列化/文件发布owner，也不以定向回归宣称完整迁移。

v379不新增/移动生产模块。原surface_javascript_fixture拥有可选的测试启动握手，原
Foreground获取/条件归还算法不变；Application harness只在服务端计时前薄接线。
surface_uia_process_fixture仍唯一拥有Child/管道/确认/Drop，显式定向父/Child许可和
return均留在该owner，原四命令/输出/生命周期预算不扩。focus HWND测试只接许可
归还，不修改产品前台/键盘/FIFO/权限owner；诊断launcher留在linshi，无新永久wrapper。
最终行数/静态/后台与封存身份见迁移台账v379；原生归还路径仍未验证，不关闭旧失败。

最终四份测试变更无新增：Application466→467、Child321→363、focus HWND240→241、
共用fixture79→97有效行；最大467，3,719文件ratchet零违规/警告/新例外。八后台门
全过、同源前台0/1与四项未重跑门分报；三次源码/六件/旧228件、四诊断及234份后续
保护见迁移台账v379，不建立第二进程/状态/权限owner或扩大完整迁移结论。

v378由surface_box唯一拥有四边Insets，measure/tree/position/geometry/scroll只接各起边与
轴总量，不复制盒模型。surface_round_shape拥有独立内角及命中谓词，Background/Hit
复用同形。surface_border仍唯一持有factory缓存，新增border_partition拥有有界半平面
分色，border_style拥有side颜色与原按钮动态色接线；没有第二paint/state/cache owner。
desktop/surface_edit_border_clip只拥有临时GDI区域RAII，原edit_box负责HWND/格式投影。
surface_javascript_css_borders只负责边框准入/展开/反射/投影，原cascade/DOM/Session/
Runtime权限与资源owner不变。通用fixture、native几何/WIC、JS模型/child/产品、HWND和
EDIT测试各保护原边界，不复制股票业务。最终45份native变更/11新增，最大445有效行
为paint_frame455→445；text_frame249→253保持最终文字帧单一责任，Session440→443
仅测试接线。新增生产模块分别CSS94、border_style50、partition58、EDIT clip111；其余
新测试/fixture均≤100。3,719文件ratchet零违规/警告/新例外；22门全过及三次源码/
六件/旧222件与后续228件身份见迁移台账v378，不代默认JS/业务/完整迁移或关闭旧失败。

v377新增surface_text_overflow独占最终DWrite省略配置及隐藏后缀命中范围；text_frame
仅在原最终分配宽度入口接线，固有测量/字体Style继承/盒几何owner不变。text_word仅借
隐藏范围保护词级复制，不新增选择owner。CSS values负责枚举，spatial负责文字叶裁剪
和上下文准入，原projection/cascade不复制。新native测试分别拥有DWrite/缓存/选择、
WIC像素，JS模型/child/产品与HWND测试各自保护原边界，通用fixture不复制股票业务。
不新增缓存/进程/线程/权限owner；最终行数、ratchet和同源运行/身份见迁移台账v377。

最终17份native变更/7新增，最大440有效行为原Session薄测试接线437→440；新省略
模块108、DWrite测试232、WIC测试118、JS模型56、child/产品127、HWND65、fixture22。
3,708文件ratchet零违规/警告/新例外，UTF-8无BOM；异常trimmed命中修复仍归文字
Engine，不复制选择owner。19相关门全过、三次源码/六件/旧216件和后续222件保护见
迁移台账v377；不以局部闭环关闭旧前台失败或完整JS/业务/迁移/用户包验收。

v376不新增/移动生产模块：css_sheets仍唯一拥有cascade，winner空原型record只活在
当前computed，key只来自原准入/canonical化，无新缓存/状态owner。新增cascade_record_tests
116有效行独占临时容器差分/调用计数/原型边界，原projection测试先借其旧Map副本后
保留rank oracle；Engine仅三行测试接线。四变更分别116、301、74、386行，最大386，
3,701文件ratchet零违规/警告/新例外。20门18过/2前台前提失败，四压力过但短样本
不证整体加速；三次源码/六件/旧210份及216份后续保护见迁移台账v376，不代完整迁移。

v375最终十一native变更/四新增，最大437；新增HWND132、Engine124、child/产品152、
通用JS44有效行，旧拒绝fixture27→27只将过时:hover负向换成:hovering，保全部原子
断言。三个生产owner与首轮一致，无新增生产模块/队列/权限owner。3,700文件ratchet
零违规/警告/新例外，严格静态通过；21门17过/4压力失败、未重跑的首轮完整窗口与
三次源码/两套程序/旧204份/后续210份保护见迁移台账v375，不把功能green当完整验收。

v375仍由surface_javascript_hover.js唯一拥有上一派发路径；author matcher只借其私有
matches谓词，当前父链/root连接归原tree，cascade/projection/geometry仍各归原owner。
bootstrap只接工厂依赖，不新增队列/缓存/权限/历史。新增css_hover_tests拥有child/产品
及污染/原子性/生命周期，css_hover_model_tests拥有原Engine栈/32层/压力，desktop的
css_hover_tests拥有真实HWND/FIFO/像素/命中/leave/隐藏；通用JS夹具不复制股票业务。
父模块仅测试接线；最终有效行、ratchet和相关封存门见迁移台账v375，不代完整迁移。

首轮十份native变更/四新增分别132/124/152/44有效行，最大437为原Session模块的
三行测试接线；三个生产owner仅增长11/1/1行。3,700文件ratchet零违规/警告/新例外。
22门18过/4失败的过时样本与前台/锁中毒分解、封存身份及下一候选见迁移台账v375；
原拒绝夹具仍拥有原子性责任，后续仅更新支持集边界，不移除负向断言或新增owner。

v374不新增/移动生产模块。Application focus测试只拥有每段输入前台前提；现成
surface_uia_process_fixture继续唯一拥有自启Child/管道/超时/Drop，增加foreground
测试模式而不伪造UIA action。uia root仅cfg(test)窄重导出，范围限surface_window
子树；focus窗口测试拥有跨进程负向/恢复不重放证明，不复制进程owner或新增产品
状态。最终行数、相关矩阵与身份见迁移台账v374，不以测试修复关闭历史未知根因。

surface_javascript_fixture仅新增track_caller归因，不改变前台获取/归还算法；updated
数据验证不拥有跨网络批次的前台，实际焦点断言仍归prepare交付边界。跨进程新测试
按原惯例显式运行，最终独立门不得省略，不把默认ignored或一次通过当完整迁移。

第三候选仍由原UIA Child owner处理foreground单次命令/目标线程确认，不复制进程
框架；原Foreground增加被动guard构造并被本地获取函数复用，恢复状态仍只有一个
owner。10门显式负向和全部新旧候选身份必须分报，见迁移台账v374。

最终六份native测试/仅测试接线变更分别为Application focus93→128、Child fixture
277→321、process tests193→193、uia root99→101、focus HWND173→240、共用fixture75→79。
最大321行仍为唯一Child/管道/事件/清理责任，其余≤250；无新增模块/例外，UTF-8
无BOM，3,696文件ratchet零违规/警告。最终10相关门全过、三次源码/六件/旧192份及
前两候选身份见迁移台账v374/closeout，不覆盖旧完整组失败或声明完整迁移完成。

v373不新增/移动生产模块，原surface_window_keyboard_tests独占ThreadKeys测试状态：
普通消息逐条保存/中和modifier/恢复，raw消息保护真实产品准入，独立回归核嵌套与
unwind完整恢复。不抽取跨产品键盘owner、不保临时观测日志；最终行数/受控RED和
封存相关门/身份见迁移台账v373，不把夹具确定性修复等同历史根因或迁移已完成。

最终仅该测试文件152→296有效行，增长144行仍为同一键盘状态/候选/准入/恢复责任；
无新增生产owner。3,696文件ratchet零违规/警告/新例外，严格静态通过；8门7过、
1失败、窗口129/0与Application14/11分解、三次源码/六件/旧174份及三诊断程序见
迁移台账v373/closeout。下一轮保护180份最终封存件，不将相邻窗口green当完整迁移。

v372新增surface_javascript_hover_input拥有私有有界wire，hover.js只拥有上一条派发路径；
EventTarget仍唯一拥有事件identity与relatedTarget brand。desktop/surface_edit_hover_queue
只扩原Queue的容量/序号/取消元数据，不建立第二FIFO；surface_edit_hover接原Mount与
当前Presentation通用可见节点资格。surface_window_javascript_hover只采样HWND无按键
消息，复用原pointer的DIP/modifier/leave tracking及State归还接缝，不拥有焦点/capture。
Mount/Host/AuthorizedWindow保持薄接线，原Runtime/batch授权owner不变。

hover_queue_tests保护共享terminal/MAX/合并/fence/在途，hover_tests及通用夹具保护真实
child/产品/relatedTarget/污染/rAF，window hover tests保护EDIT焦点/原FIFO/像素/取消，
window hover lifetime子模块独占后台/带键/真实隐藏/Host归还回归，Application hover tests
只扩原TLS harness mode20，不复制服务协议。Mount pending仍为唯一调度可见性，不把
手动prepare当产品已领取。官方lexer行数、
ratchet与最终实绩/身份见迁移台账v372，不以局部子集关闭完整迁移或默认JS门禁。

最终35份native变更、12新增、最大500有效行，3,696文件ratchet零违规/警告/新例外，
严格静态通过。Mount497→500只保薄接线/唯一pending调度分型，新增Hover实现各归原
责任模块，不新建队列/调度owner。34门32过、2失败、独立Hover Application1/0与
完整组失败分解、三次源码/六件/旧168份见迁移台账v372/closeout，不把局部green当
完整回归或迁移完成；下一轮保护174份封存件。

v371新增surface_javascript_pointer_input拥有有界私有wire；EventTarget仍唯一拥有事件
identity/分派。desktop/surface_edit_pointer及pointer_queue只接原Mount/Queue与Canvas
资格/terminal预留，不建新FIFO；surface_host_pointer只借原Presentation/编辑owner。
surface_window_javascript_pointer独占物理Canvas Press，State/原window生命周期负责
重入取消与归还，原keyboard只增加Canvas焦点的窄软分流；Runtime/batch授权owner不变。
pointer_model_tests、pointer_tests和两JS夹具保护模型/child/产品；window pointer tests
及其reentry子模块保护真实HWND连续手势、像素、捕获同步重入和清理；queue tests保护
MAX序号/非Pointer顺序/fence，application pointer tests复用原TLS harness mode 19。
两个cfg(test)只读front快照用于在prepare兜底前核取消，不增加生产观察API。最终行数、
UTF-8/ratchet、实绩与身份见迁移台账v371，不以子集通过代表完整迁移或默认JS放行。

最终37个native变更、13新增、最大497有效行；新增HWND主测试273行只负责同一消息/
FIFO/像素/取消生命周期，重入另置105行模块，其他新增不超过214行。3,684文件ratchet
零违规/警告/新例外，严格静态通过；27门26过/1失败及独立Pointer Application1/0、
三次源码/六件/旧162份见迁移台账v371，不把一次独立成功替代完整Application失败。

v370不新增/移动模块：CSS层叠继续由surface_javascript_css_sheets.js唯一拥有，winner
记录只活在单次computed，跨声明更新不产生第二份样式/缓存/生命周期owner。原projection
测试owner扩展旧算法差分、32组Rust rank oracle及节点/属性隔离和样式更新/原型边界；
最终行数、原压力三路径与完整相邻门/身份见迁移台账v370，不新增权限或资源预算。

最终仅css_sheets73→75和projection tests186→300有效行，后者单一责任为投影/层叠
差分与oracle；3,671文件ratchet无违规/警告/新例外，严格静态通过。24运行门23过、
1失败，三诊断/同源身份/旧156份及Application前台失败分解见迁移台账v370/closeout；
旧库相邻矩阵亦全过，不将性能短样本或窗口组通过当完整迁移或未知根因关闭。

v369的Foreground仍唯一归surface_javascript_fixture测试owner，新增previous/target只用于
有条件恢复，不新增生产焦点/权限owner。desktop/surface_window_foreground_tests拥有三个
自有HWND的后来选择与unwind回归，由surface_window_tests接线；不为了测试公开私有HWND。
surface_edit_values_tests只增加原候选前提与失败分支诊断，不改变消息、草稿、激活快照
或后续输入断言。首次支架编译失败/确定性RED及最终行数、同源实绩见迁移台账v369，
不将测试cleanup修复说成产品获取前台、历史候选丢失或完整迁移已完成。

最终四变更values173→192、fixture65→75、接线158→160、新增foreground tests67有效行，
UTF-8无BOM，3,671文件ratchet零违规/警告/新例外，严格静态通过。12门10过、2失败，
Application23/0及values5/0等通过；foreground1/1和完整窗口93/18的前台获取/锁中毒及
两个上下文的unwind成绩见迁移台账v369，不互相覆盖。首轮Clippy失败保留；三次源码/
六件/旧150份与精确diff由最终closeout分别核验，保持partial-runtime-failure与原owner边界。

v368只改application_surface_javascript_keyboard_tests的驱动前台生命周期与发送前断言，
复用原Foreground guard；surface_window_javascript_keyboard_tests补后台键盘不入队/
不恢复重放的真实HWND证明。未改生产keyboard/FIFO/Host/权限owner，也不新增激活API、
测试全局输入或资源宽限。行数、完整Application/相邻门和身份见迁移台账v368；开发
状态观测留证但不把逐步打印留在最终代码，不覆盖历史失败或扩大完整迁移结论。
当前交互桌面被游戏使用，未操作其进程；开发负向1/0与Application发送前失败0/1
分别保留。用户随后允许短暂占用前台，最终恢复完整10门矩阵，实际终态另记，不能以
授权、运行启动或新断言更明确代替回归成功。

最终两测试文件分别112→134、282→329有效行，UTF-8无BOM；3,670文件ratchet零违规/
警告/新例外，严格静态通过。原定10门8过、2失败：Application13/10及SurfaceWindow90/19
的直接失败/共享锁中毒分解见迁移台账v368，新增负向最终未进入测试体。未改生产owner
或新增状态/权限/资源预算；三次源码/六件/旧144份与精确diff由最终closeout分别核验，
保持partial-runtime-failure，不用开发单项通过、startup7/0或更准确报错覆盖历史失败。

v367热点仍归原css_selectors/sheets/projection三个owner，不移动业务或建立第二份DOM/
几何/层叠状态。sheet的有序声明替代其原Map存储，不重复持有；inline仍归declarations。
css_budget_tests拥有固定生产Engine诊断，css_projection_tests拥有原算法策略差分/拒绝/
原型边界，css_pressure_tests与通用pressure夹具拥有真实child/产品完整帧与回收证明。
原nth压力用例的规模/断言/预算保留，行数及最终身份/实绩见迁移台账v367。
json owner只优化私有描述符复制；json_model_tests保护其数据/深度/污染边界，阶段探针
留在css_phase_tests的测试副本，不给Engine或产品增加诊断API、时钟或宽限预算。
selector的目标匹配context只活在单次sheets.computed调用，复用state/ordinal，不增加跨帧
缓存或并发owner；ancestor匹配仍沿原树，原mutation位置索引的失效边界不变。

v367最终12个native变更最大428有效行，六新增budget47、phase91、pressure88、projection
186、json52及压力夹具5行；其余为原四个JS owner窄改和两处测试接线。全量3,670文件
ratchet零违规/警告/新例外，UTF-8无BOM。24实际运行门18过、6失败，另两诊断分报；
Application/完整JS/startup及三压力路径的失败、三次源码/六件/原138份与partial状态
见迁移台账v367，不相加重叠集合，不新增状态/权限owner或把诊断成功当成迁移完成。

v366继续由surface_javascript_css_selectors.js唯一拥有author matcher、公式准入和
specificity；位置WeakMap只借原tree的mutation identity，每代每父线性扫描，不复制
DOM/几何/cascade或维护历史。first-child共用位置索引但保留原有父条件。新nth_model_tests
拥有Rust整数oracle/原子性/边界，nth_tests及三份通用夹具贯穿真实child/实际产品与
同步几何/污染，desktop的javascript_nth_tests只检验真实HWND像素/重排/命中及回收。
行数、失败、选定运行与同源身份见迁移台账v366，不新增业务/授权/资源owner。

v366最终11个native变更最大425有效行，六新增分别为130/121/75/21/20/18行，原selector
88→128；全量3,664文件ratchet零违规/警告/新例外，UTF-8无BOM。14运行门13过、压力
失败，首轮Application13/10及一次Enter复核另留证，不视为根因已修。三次源码/六件/
原132份与七文档/组织合同分别绑定，partial状态及完整身份见迁移台账v366。

v365继续由原surface_javascript_form.js唯一拥有值准入/去换行/dirty反射，不引入缓存、
第二份表单或native校验owner。form_budget_tests只在测试构建内记录原Engine固定样本
的墙钟/当前线程CPU；form_value_model_tests用Rust UTF-16/UTF-8 oracle检查784组输入，
form_value_tests及通用boundary夹具贯穿child/实际产品的22次有界操作。探针、模型和
产品证明分别报告，不用诊断测试退出码冒充预算通过。实际行数/失败/身份见迁移台账
v365，原Session/队列/焦点/权限/资源owner均不变。

最终七个native变更最大422有效行，四新增92/62/57/48行，原form61→76、其余仅测试
模块接线+3/+6；全部UTF-8无BOM。严格静态/3,658文件ratchet和43选定封存门通过，无新
例外。三次源码/六件/旧120份、固定探针和QuickJS native stringify夹具失败见迁移
台账v365，不新增业务owner，不把诊断短样本或模型差分等同完整迁移/资源验收。

v364的surface_javascript_local_input.rs只拥有私有Text/Keyboard分型和有界Enter carrier，
不把keydown伪装成文本Phase。原EventTarget唯一拥有KeyboardEvent identity/取消/分派；
form复用同一连接/disabled检查，不用键盘重写值。desktop/surface_edit_keyboard.rs
只扩展原Mount/Session/Queue的Enter捕获与默认change结算，原intents维护唯一容量、
序号、scope和fence，不新增键盘队列。surface_window_javascript_keyboard.rs仅接线
真实消息/当前焦点策略，原Application harness复用TLS/Grant/FIFO，不复制服务协议。
surface_window_state_lifetime.rs等价迁出原State::run/stop，独占Host临时借出/归还和
关闭接缝，不复制窗口/权限状态，使消息入口保持500行以内。通用Enter夹具、child/
产品/HWND/Application与queue/wire测试保护取消/顺序/污染/生命周期；行数/实绩见v364。

最终38个native变更最大493有效行，九新增112/92/56/55/282/41/143/93/33行，均UTF-8
无BOM；282行文件仅负责真实HWND Enter合同，原窗口入口499→465、Mount483→493。
严格静态/3,654文件ratchet无违规/警告或新例外。41封存门首轮40通过、1失败，固定
六门同字节诊断含完整窗口107/0通过；原mount超时/锁中毒不被复核覆盖，closeout仍
partial-runtime-failure。源码/程序/旧114份与诊断见迁移台账v364，不新增输入/权限owner。

v363的surface_javascript_focus.js只拥有当前operation的32项焦点意图，原tree/form仍
拥有节点/dirty值；surface_javascript_focus.rs只验证私有target/focus carrier，不含
HWND或远端权限。desktop/surface_window_javascript_focus.rs复用原Mount/语义句柄/
消息桥，记录focus epoch并在同一前台Surface内执行，不持额外窗口索引或输入队列。
应用surface_input_origin窄提取既有策略供回复前收紧和原dispatch共用，不能有第二
套模态/授权规则。两个通用夹具、child/产品/HWND/Application测试保护请求/污染/
原失焦change/TLS/Grant/FIFO/ABA/策略；原fixture增加可unwind恢复前台的屏外测试支架。
实际行数和运行成绩见迁移台账v363，不新增线程、资源预算或业务owner。

最终24个native变更最大499有效行，八新增26/18/102/135/168/93/28/10行，均UTF-8
无BOM；原消息桥494→499仅最小字段/初始化/分派接线，没有迁入新业务责任。3,645
文件ratchet零违规/警告或新例外，全目标严格静态和37选定封存门通过；前台支架/
默认失焦夹具失败、源码/六程序/旧108份保护见迁移台账v363，不扩大API或总目标结论。

v362新增surface_javascript_form.js仅拥有文本input的dirty值/默认属性和私有投影反射，
不复制tree/事件/布局/原生草稿。surface_javascript_text_input.rs归私有wire，只校验
target/phase/value，不含action/HWND/凭据。desktop/surface_edit_javascript.rs是原
Mount/Queue的本地出口，共用Session的ticket登记和原preparation_nodes准入，DOM帧
呈现后结算，与emit远端回执分离。Runtime/worker仍用原单槽，AuthorizedWindow沿
原click fence保持prefix→click→later顺序，不另建队列、线程或权限owner。
input_tests、两个通用夹具、desktop的text_tests以及原Application harness的text模式
分别保护真实child/产品、污染/值语义、HWND/IME/fence/后续草稿和TLS/Grant/FIFO。
新增/修改文件实际行数与验证见迁移台账v362，默认能力和完整迁移边界不扩大。

最终36个native变更最大483有效行；八新增55/57/75/175/145/69/35/14行，全部
UTF-8无BOM，3,637文件ratchet无违规/警告或新例外。严格静态及33选定封存门通过；
本地ticket不在提交失败时归队重放。源码/程序/旧102份和失败身份见迁移台账v362，
仍沿原字段合同失效，不为保HWND而放宽fingerprint，也不增加第二个输入owner。

v361新增surface_javascript_event_target.js，唯一拥有文档内事件 identity/listener/阶段/
取消/停止与回调清理，复用tree捕获的intrinsic和父链接；不持第二份树或远端authority。
tree仅使Node继承EventTarget，bootstrap保原click准入后调用同一私有dispatcher，
Engine沿原clock绑定时间戳。document模型的私有状态泄漏断言按实际tree字段更新，
不保留已迁走listeners的失效判断。事件模型/child/产品/HWND及四份通用夹具分别
保护语义、污染/预算、真实点击/像素和回收；Application在原input harness/peer追加
事件模式，复用原TLS/Grant/FIFO，不复制服务协议。实际行数/成绩见迁移台账v361，
不以合成事件支持代替尚缺的native键盘/pointer/input或完整迁移验收。

native click只借原Engine已拥有的job边界直接调用同一dispatcher，bootstrap显式await；
不新建排程owner/跨operation队列，public dispatch仍同步。私有carrier和Promise
constructor/species/then保护留在事件owner；直接Engine双夹具保护原栈下同步Grid
写入和空间几何投影，不把单夹具成功代替全回归。开发失败/最终行数见迁移台账。

最终19个native变更最大386有效行，八新增为事件owner129、模型62、child/产品148、
HWND58及四夹具49/40/44/35，全部UTF-8无BOM；3,629文件ratchet无违规/警告或新
例外，严格静态与29封存门通过。原timer模型helper只补tree所需同一生产事件工厂
参数，不复制工厂/算法或削弱断言。身份、失败及旧96份保护见迁移台账v361/closeout，
保持原Engine/Grant/FIFO/Job与各资源owner，不把本切片当成完整native输入或迁移。

v360的四边 Insets 归原 surface_box，唯一拥有内缩起点/横纵总量及单值/side/default
解析；各布局/文字/图片/几何 consumer 不复制解析或压成统一标量。原 CSS values
拥有 padding 展开/准入，declarations 保持 cascade/反射/原子提交 owner；新增
surface_round_shape 仅生成 content-edge 四角几何，缓存/工厂/COM仍归原 round_clip。
textarea 的格式矩形继续归原 surface_edit_box，不创建第二套原生输入或草稿 owner。
surface_padding_tests、JS padding model/child/三个通用夹具、HWND 和 image padding
pixels tests 分别保护内容盒、原子语义/原预算、实际像素/命中和四角；新文件行数和
最终实际成绩见迁移台账 v360，不以接线代替真实运行或完整迁移。

最终40个native变更最大455有效行，九新增为round shape100、padding tests168、
图片像素46、JS模型68、child/产品118、HWND61及三夹具43/31/11行，全部UTF-8无BOM。
3,621文件ratchet无违规/警告或新例外，全目标严格静态及26封存门通过；首轮
JS过期双值拒绝/scalar投影预期只修两个测试输入，生产owner/预算不变。首轮partial、
lint/证据脚本失败、源/程序摘要与旧90份保护见迁移台账v360/最终closeout。

v359新增surface_javascript_css_fonts.js，唯一负责family/字号/字重/行高的值准入与
font原子展开，原values只分派；不是另一个字体集合、布局或生命周期owner。原
declarations处理四项的priority/反射/移除，原projection逐节点保留unitless语义，
surface_text_node在normal时清除继承行距。文字Engine/Styles和原em resolver仍负责
实际native字体/度量，默认字号不复制进JS。typography_model_tests、typography_tests、
三份通用typography夹具及desktop的typography_tests分别保护模型、child/产品/同步
几何与真实HWND，行数和最终成绩见迁移台账v359，不以接线代替运行验收。

原CSS属性Proxy在公开入口完成用户String转换；原projection的native只被prepare
调用，已合入同一函数以减少重入栈，不移动cascade/空间验证/最终呈现的状态owner。
直接Engine的受控夹具异常仅用于测试诊断，不向实际产品增加异常文本或wire字段；
原几何mutation/source缓存及原native读取准入不变，实测边界见迁移台账v359。

最终16个native变更最大380有效行；七新增的CSS fonts64行、模型88行、child/产品
151行、HWND73行及三夹具39/38/11行全部合规。3,612文件ratchet无违规/警告或新
例外，严格静态/构建及21封存门通过；实际身份与失败/旧78份保护见迁移台账v359/
closeout，不以纯22条消费者值探针或选定green代替完整插件/迁移验收。

v358的surface_text_families.rs归原文字Engine，唯一拥有有界名称解析、lazy安装字体
集合和64项选择缓存，不创建全局字体/网络owner。Styles另持Rc并在get时借用，
NativeMeasure复用同一Engine完成全树预计算；DirectWrite和原生字段消费同一已选
family。CSS values只负责有界声明准入/规范化，原cascade/projection/mutation保持
唯一状态owner。font_tests和三份通用夹具保护真实child/同步几何/产品，原CSS HWND
及EDIT/Select测试接真实像素/字体句柄；精确子集、行数及成绩见JS合同/迁移台账v358。
values原字符域与字节准入仍归同一family分支；NBSP少计反例进入同通用fonts夹具，
不新增parser或预算owner。旧CSS拒绝夹具只更新过期的两项font-family拒绝值。

最终18个native变更最大377有效行；六新增分别为字体选择136行、字体选择测试158行、
JS字体测试156行及三份通用夹具31/13/18行。3,605文件ratchet无违规/警告或新例外，
严格静态和封存19门通过。原surface_javascript_css_tests.rs已还原至本轮前内容，不
计入18个变更；原拒绝夹具修正及生产NBSP准入修复分开留证，最终身份/成绩与旧
未知根因边界见迁移台账v358及final-admission/closeout，不新增全局或业务owner。

v357的surface_javascript_diagnostics.rs只归测试wire：固定enum、最多64样本的Trace、
Report校验及忽略写入错误的日志，不拥有执行/预算/异常或资源。Engine/Canvas
仍在自身阶段留痕，worker沿原Response交付，Session只在原失败分支记录；均受
cfg(test)隔离，不增加生产协议字段、JS调用wrapper或第二个生命周期owner。
diagnostic_tests归原Engine测试模块，真实child回归归既有Session tests，复用原
Canvas通用夹具；行数与验证见迁移台账v357，不预称Application故障已修复。

直接Engine Canvas模型的apply错误也复用原Report，不新建诊断owner。最终九个
native变更最大374有效行，两新增188/107，3,599文件ratchet无新例外。首轮20门
19过、JS124/1/11 ignored的模型mount失败保留；只改该测试helper后的独立16门通过、
三次完整JS各125/0/11，但不清除未知根因、整轮仍partial。身份/边界见迁移台账v357。

v356的输入String转换归原CSS公开setter/replaceSync，私有声明/规则parser只收
primitive string；不增加几何、授权或mutation owner。两份通用reentrancy夹具
由真实Engine/child/实际产品门消费，geometry_tests内测试专用mount_product复用
原Session/worker启动接缝，不复制产品实现。拒绝测试核转换阶段及错误身份/消息，
实际行数、冻结与成绩见迁移台账v356；默认能力与完整迁移边界不变。

values保留唯一值准入/规范化owner，只将expand的私有validate单调用层合并，并让
color直接使用同一捕获exec，减少真实嵌套解析栈；未复制规则或新增旁路。

v356七个native变更最大214有效行，新夹具14/34行，3,597文件ratchet无新例外。
严格静态/构建及真实重入Engine/child/产品门通过，但Canvas Application仍有未定位
失败，19封存门中18门通过、整轮partial；成功对照不抹去失败，详见迁移台账。

v355的mutation identity token归原tree owner，attributes/declarations/sheets/canvas
只在自身校验后通知，不复制树或建立变更日志。geometry只缓存同operation的encoded
projection，仍逐次走原生read，不拥有最终布局或授权；begin/dispose清理缓存。
geometry_projection_tests使用同生产BOOTSTRAP/真实QuickJS与fake boxes保护失效、
拒绝、重入、intrinsic隔离和生命周期，原Engine/child/HWND/Application继续验证
真实布局及预算。实际行数、冻结与运行成绩见迁移台账v355，不预称迁移完成。

本轮八个native变更最大289有效行，新测试模块194行，3,595文件ratchet无新增例外。
测试副本在cache miss处插入计数/故障注入，避免再套一层project调用；operation由
Rust交付，但输入转换重入模型仍超原256KiB，未删场景或放宽预算。实际HWND/
child/Application及产品worker通过不代替此失败，18门仅16门通过，见迁移台账v355。

v354：surface_javascript_timers.js独占64容量登记、signed32转换、嵌套clamp、到期
快照及暂停剩余时间；时钟仍归animation/child Instant，Engine负责逐callback
microtask及原总期限。wire/Session只传下一等待，Runtime沿原单槽，AuthorizedWindow
合并原等待deadline并区分timer-only/rAF；不创建新线程、轮询、业务队列或授权owner。
timer_model/child/HWND测试共用通用timers夹具，Application timer测试复用原input
TLS peer/两事件FIFO/同child收敛与teardown，不复制服务协议。实际行数与最终验证
见迁移台账v354，不继承旧绿灯或扩大默认能力。

最终24个native新增/修改文件最大382有效行、七新增最大139，3,594文件ratchet
无新例外。旧counter只修正无timer断言并保留安全断言，第二轮生产输入与首轮相同。
timer/JS/五产品worker门通过，但geometry和startup仍失败、18门仅16门通过，
partial及具体身份/根因边界见迁移台账v354/最终收据，不冒充完整迁移验收。

v353：startup_smoke.rs独占原隐藏启动验收序列，从desktop/mod.rs迁出但不拥有
窗口、COM、期限或系统输入。smoke_timing只拥有显式固定阶段、配对ID、单调计时
及忽略诊断管道错误；DocumentWindow/Canvas/RenderSurface/Window仍为原资源owner，
只增加诊断边界，不复制实现或放宽预算。并发ID/断管道/状态保留测试和原startup
场景保护该边界，实际行数、冻结及运行状态见迁移台账v353，不继承旧绿灯。

最终七个Rust文件最大415有效行，startup_smoke104、smoke_timing168，入口由498
降至415；3,587文件ratchet无新例外。计时5项与相关资源回归通过，但startup6/1
仍超原15秒，整轮partial；零项筛选不计通过，完整同源性与运行证据见本轮closeout。

v352：surface_javascript_canvas/context/path.js分别拥有Canvas identity/尺寸重置、
状态/保存栈/绘制提交和当前路径；不拥有像素或业务图表。canvas_raster/commands/
draw/path.rs分别拥有child WIC/D2D生命周期与预算、类型准入、真实绘制和路径几何；
canvas_wire.rs归私有JS协议，负责尺寸/PNG编码与父侧当前Scene身份验证。原
image_measure/image_paint只增加已验证Canvas元数据的自然尺寸/fill分支，普通图片
合同不变；SurfaceWindow接原图片owner，不创建第二个绘制或动作队列。模型/child/
HWND/Application测试共用单份javascript-canvas夹具与既有TLS peer，保护实际像素、
重置/alpha/路径/文字、observer→rAF和回收。精确子集与结果见JS合同/迁移台账v352。

v352第二轮共29个新增/修改native文件，13新增的生产模块最大257有效行，既有
修改最大374，3,586文件官方ratchet无新增例外。首轮旧canvas拒绝夹具改为video并
补markup正向像素测试，生产输入未变。严格静态/构建通过；封存Canvas/JS/实际产品
worker通过，但startup6/1超原15秒，整轮partial，旧失败和完整迁移验收保持开放。

v351：surface_javascript_resize.js 拥有有界 observer 登记、深度批次及冻结 entry，
不拥有布局或 timer；geometry 复用原 PreparedScene 和 surface_button::insets 交付
内容盒。engine 继续拥有逐 callback microtask/总期限。SurfaceWindow 只记录真实
尺寸/DPI消息代次，AuthorizedWindow 复用原单槽/策略/deadline 调度和 stale fence。
resize_model/child/HWND/Application tests 与单份通用 javascript-resize fixture 保护
真实盒/像素、无点击 resize、空闲/模态、ABA 和回收；不复制 TLS 或产品业务 owner。
精确子集见 [ResizeObserver](SURFACE_JAVASCRIPT_NATIVE.md#v351-事件驱动的-resizeobserver-子集)，
最终行数和实际运行状态见迁移台账 v351；没有默认能力、依赖或资源预算扩张。

v351共22个新增/修改native文件，六新增35–160有效行，既有修改最大494，官方
3,573文件ratchet无新增例外。严格静态/构建及封存后11个选定运行门通过，含真实
HWND/Application和摘要绑定产品worker；不是全库/完整迁移验收，旧未知根因保留。

v350 续进的 wait_reply_for_test 留在原 Runtime 的 cfg(test) 接缝，Application/owner
夹具共用线程终态优先的等待，不新增生产等待 owner。Geometry 的固定阶段轨迹仅
编译进测试、最多32项、不含脚本/节点/快照；原生产预算/错误策略和模块归属不改。
stale-frame 测试借原 Sessions 的 cfg(test) 接缝读取既有 event_backlog，区分事件
任务/收据与合法 inactive lifecycle；无新增生产任务/队列或协议 owner。

v350：surface_javascript_animation.js 只拥有每文档64容量的 callback/handle 快照和
私有时钟引用，engine 逐 callback 排尽 microtask；不复制文档/布局或业务 owner。
surface_window_javascript_animation 归原 AuthorizedWindow，只管理 pending deadline 与
当前策略/唯一 owner fence；Session/Mounted/Runtime/mailbox 沿原单槽操作，不增线程。
animation_model/child/HWND tests 和通用 animation/batch-animation fixture 保护语义、
预算回收、真实像素/同步几何；Application animation tests 复用原 input TLS peer/
setup/FIFO，不复制服务协议。精确边界见 [rAF](SURFACE_JAVASCRIPT_NATIVE.md#v350-宿主驱动的-animation-frame)，
行数/冻结/封存结果见迁移台账 v350；旧 runtime_tests 原字节保留，无默认能力放行。

v350八新增文件21–142有效行，既有修改最大450，3,567文件ratchet无新增例外。
严格静态/编译及源码rAF集合通过，但封存集合、UIA/TLS回归未全绿，partial状态和
失败/程序身份见迁移台账v350/closeout；实际产品worker单项成功不替代整轮验收。

v349：surface_javascript_css_spatial.js 负责空间声明的有界展开与直接父/绘制顺序/
同轴裁剪准入；projection 只接原 Scene layout 键，布局/裁剪仍归 surface_position/
overflow/NativeMeasure/Presentation，不复制原生算法。原 Grid 与新 shorthand 共用
declarations 的 names 列表和完整验证后提交，尺寸 decimal 统一由 values 规范化。
spatial_model/child/HWND tests 与三份通用 fixture 分别保护声明/污染、进程预算与
真实像素/命中/同步 bounds；Application 原 geometry 夹具消费同接线，不增协议 owner。
精确支持/拒绝见 [空间 CSS 合同](SURFACE_JAVASCRIPT_NATIVE.md#v349-受限定位与容器裁剪)，
行数、冻结/封存验证与失败保留见迁移台账 v349；默认 JS 与完整迁移仍未放行。

v349七新增文件17–204有效行，既有修改最大328，官方3,559文件ratchet无新增例外。
严格静态/相关回归及封存后spatial11/1 ignored、实际产品/Application/预算/资源/
startup7/0通过；2,407源码输入和六程序/旧封存产品身份一致，相关进程残留零。
本轮成功不清除原v348启动未知根因，完整身份与早期失败见迁移台账v349/closeout。

v348：surface_viewport 独占生产 Host/child 共用字体/context 与真实客户区/DPI。
surface_javascript_geometry.js 拥有同步 getters/DPR 和准入，Rust geometry 仅持本次
operation 的有界单投影缓存并复用 NativeMeasure/PreparedScene，不复制布局或树 owner。
wire/Session/Runtime 传捕获的 viewport；原 AuthorizedWindow input/presentation 在
交付和最终 batch 移交复核依赖，events_viewport 随 emit 暂存/take/cancel 保留，不
增加 FIFO、线程、动作或授权 owner。bootstrap 保留唯一 async 生命周期边界，CSS
私有 Map 迭代压平调用且提前捕获 intrinsic，不提高原栈预算。

geometry_model/engine/child tests 分别保护预算/缓存、真实 QuickJS 诊断和持久 child/
显式产品门；真实 HWND geometry tests 复用原 CSS 像素/几何夹具。Application 原
input tests/peer 增加三个 geometry 模式，不复制服务协议。九新增文件及既有修改
由官方 lexer/ratchet 测量，完整结果见迁移台账 v348；旧 runtime_tests 原字节保留。
同步支持/拒绝与未实现 observer/rAF/Canvas/默认 JS/完整迁移边界见
[JS 几何合同](SURFACE_JAVASCRIPT_NATIVE.md)。

v348九新增文件25–155有效行，既有修改最大450，官方3,552文件ratchet无新例外。
严格源码/编译与geometry/Application/实际产品worker等定向门通过；整体封存运行
因startup4/3超原15秒保持partial。后续单场景计时通过不当作根因修复，失败与
完整结果见迁移台账v348/closeout，不扩大默认能力或完整迁移验收结论。

v347：surface_javascript_css_grid_tracks.js 仅拥有非递归轨道准入/规范化；css_grid.js
拥有 Grid 声明展开、端点和对齐枚举。declarations 先全量验证再保存独立 longhand，
projection 独占上下文与原生键组合；不复制 Rust Grid 求值、文档树或资源/动作 owner。
grid_model_tests/grid_tests 分别保护真实 QuickJS 声明和持久 child/显式产品门，桌面
javascript_grid_tests 复用既有 CSS 测试像素/几何采样，不复制第二份渲染夹具。三个
通用 fixture 负责正向更新、拒绝原子性和跨帧 intrinsic 污染；旧 runtime_tests 保原
字节。没有新依赖/线程/宿主能力，默认 JS/完整迁移边界见
[JS Grid 合同](SURFACE_JAVASCRIPT_NATIVE.md)，行数与冻结/封存结果见迁移台账 v347。

八新增文件为 30–204 有效行，既有修改最大 278；官方 3,543 文件 ratchet 无新例外。
严格静态、JS Grid 17/1 ignored 及相关回归通过；六程序封存后 JS Grid/实际产品/
Application/资源/startup 门通过，源码及程序身份不变，相关进程残留零。完整结果
和失败保留绑定本轮 closeout，不提升为默认能力或完整迁移验收。

v346：surface_grid 编排原 Scene 的二维 measured-content 布局；tracks/placement/axis/
item 分别拥有严格轨道语法、稀疏占位、轨道分配和共享 item 约束/合成基线。原
NativeMeasure 下的 grid_measure 仅拥有双阶段测量与本轮缓存，Tree/快照保留 cell；
图片反馈复用 cell，不新增资源、线程、动作或渲染树 owner。模型/准入、真实度量和
隐藏 HWND 测试分别保护放置/预算、DirectWrite/比例、像素/命中/失败回滚/关闭。
完整合同及未实现范围见 [SURFACE_GRID.md](SURFACE_GRID.md)；JS CSS Grid 尚未接入，
默认 capability 不扩大，完整迁移未完成。

本切片十个新增文件为 84–306 有效行；仅 axis 为 306 行，保持轨道内容贡献、有限
上限/fr 冻结和分布的一致求值职责，未增加行数例外。其余新增文件均不超过 249 行；
既有修改源码最大 353 行。模型/预算、真实度量和隐藏 HWND 回归共同保护该边界，
官方 ratchet 和最终冻结/封存结果另由迁移台账记录。

最终 3,535 文件行数门、全目标严格静态、Grid/相关回归及封存后 Grid/Application/
JS/资源/startup 门通过。2,383 源码输入及六程序身份一致、旧 .339–.345 封存产品
不变，相关进程残留零；失败原件与完整边界见迁移台账 v346，不提升为默认能力。

v345：CSS 的值准入、author matcher、inline 声明、constructed sheet/层叠和真实 Scene
转换分别归 surface_javascript_css_values/selectors/declarations/sheets/projection.js。
bootstrap 不再内联文档投影；tree.ops 只提供预先捕获的私有 intrinsic，不复制文档树。
原 Presentation/NativeMeasure/Geometry/Paint/Hits 继续拥有真实资源与几何，无新线程/
网络/队列/业务 owner。CSS 模型/child/HWND 测试和三个通用 fixture 分别保护语义、
预算/销毁及像素/同几何命中；默认 capability 与旧 runtime_tests 原字节不变。
十一新增文件为 27–142 有效行，既有修改最大 275，无新例外；支持/拒绝与预算以
[原生 CSS 合同](SURFACE_JAVASCRIPT_NATIVE.md) 为准，最终验证见迁移台账 v345。

最终 v2 的 19 syntax、严格静态/相关回归和 3,525 文件行数门通过；六程序封存后
CSS 12/1 ignored、Application JS 七项与实际产品 CSS worker 门通过。源码及产物
身份、旧封存件不变和失败保留绑定本轮 closeout，不提升为默认 JS 或完整迁移支持。

v344：surface_javascript_tree.js 拥有私有文档节点/identity/live 集合、移动规划与最终
深度准入；attributes/selectors/markup 分别拥有同源属性反射、共用简单 matcher、
严格 fragment 与序列化。surface_javascript_json.js 独占私有协议数据复制/编码，
不调用插件 inherited toJSON/getter；bootstrap 仅拥有生命周期/listener/投影。
engine 以私有参数组合工厂，不添加全局 bridge/state 或第二棵业务树。document_tests/
document_model_tests 分别拥有真实 child/显式产品门与真实 QuickJS 模型诊断，三个
document fixture 覆盖通用正向、拒绝原子性与可变 intrinsic 反例。原 Application
夹具复用新模型，保原 FIFO/服务 peer/事件断言；旧 runtime_tests 原字节不改。
十个新文件 27–180 有效行，既有修改源码最大 272，无新行数例外；最终验证见迁移
台账 v344。完整 DOM/CSS/Canvas/股票、默认 JS、权限隔离与迁移门仍开放，详见
[原生 JS 合同](SURFACE_JAVASCRIPT_NATIVE.md)。
最终 v2 的 JS 48/3 ignored、Application 七项和实际产品文档/旧 worker/资源/启动门
通过；3,514 文件行数门无新例外。源码及六程序/旧封存件身份一致，未推广 v1 失败集。

v343：surface_window_javascript_input 拥有原 HWND 候选到后台 listener、原始 emit 批次
准备与容量门；surface_window_javascript_presentation 拥有收敛快照暂存/同 child update/
真实 DOM 后提交。原 Application input、batch FIFO/确认/接续和唯一 owner 不复制。
composition_relay_surface_presentations 只结算原 Pending 收据；runtime_mailbox 复用回复
事件持有最后唤醒，不增等待槽或产品线程。surface_convergence 按原宿主事件来源检查
runtime，并分离业务 snapshot.state 与严格 lifecycle state；record 复用 NotStarted，
不增加存储格式。测试 owner 为 interaction_tests 与 Application javascript_input 的
tests/setup/TLS peer 和固定 JS fixture；setup 不开启输入，生产 Application 自行准入。
默认 JS、初始 emit/图片宿主、完整 DOM/权限隔离和迁移验收仍开放；完整 owner/预算见
[原生 JavaScript](SURFACE_JAVASCRIPT_NATIVE.md)。本切片最终验证结果见迁移台账 v343。
八个新文件 27–234 有效行，修改源码最大 447；3,504 文件行数门和严格静态/版本/
材料合同通过，无新例外。JS 定向 37/2 ignored、Application resources 24/1 ignored
及直接回归通过；最终封存运行证据与完整产品边界仍分别记录，不以数量替代验收。
最终 v2 封存后 Application JS 七项、实际产品私有 worker/预算/资源/startup 门通过，
六程序和旧 .339–.342 产品摘要不变，源码冻结/构建后/运行后相同。v1 Clippy/协议/
夹具失败保留，不混用或推广旧集合；默认能力与完整迁移门不因此放行。

v342：surface_javascript_prepare 是 Job/Runtime 共用同步取码/求值门；runtime、
runtime_mailbox、runtime_thread 分别拥有 UI 非阻塞句柄、单槽邮箱、后台执行等待。
原 AuthorizedWindow 的隐藏构造提取至 surface_window_admission，JS 接缝归
surface_window_javascript，不复制生命周期/FIFO。Application 的 JS 排程归
application_surface_javascript；资源先收割、JS 再排程、生命周期最后分类，关闭
后的线程句柄仍留原 Windows/退出泵；统一 wait_budget 预留覆盖尚未启动任务，
所有 Live/新建/恢复准入复用同一计算。owner_tests 和 Application JS tests/peer
保护闲置撤权、原始快照/Grant、真实 TLS/隐藏 DOM/激活及关闭，旧产品 worker 测试
保留原字节。默认 JS、输入/更新桥、完整 DOM/沙箱与迁移仍开放，见
[原生 JavaScript](SURFACE_JAVASCRIPT_NATIVE.md)；本轮不新增依赖或行数例外。
最终十个新增文件为 48–236 有效行，修改源码最大 433 行；3,496 文件行数门、严格
静态/版本/材料合同通过。JS 定向 31/2 ignored、封存后 Application JS 四项及产品
私有 worker/资源/启动门通过；v2 身份失败保留，最终仅采用 v3 源码集，不混用产物。

v341：surface_javascript_resource 拥有当前快照入口/脚本 MIME/同源码续租准入，复用
surface_resource_http 的私有传输门，image 独立校验不变。surface_javascript_authorized
持原 Grant/租约/通知和会话，mount_job 独占共享槽/后台 thread/Ready 同 Grant 移交；
全部阻塞 API 只供后台 owner，空闲撤权等待/refresh 仍须由未来 Application 接线。
源码获取与授权负向回归归 surface_javascript_resource_tests/authorization_tests，
原 surface_window_javascript_tests 的 test-only 门复用于真 TLS→child→HWND 点击链，
不扩大产品窗口 API。bootstrap 仅优化等价 UTF-8 计数，原预算不变；Session/FIFO/
动作/窗口/默认 capability owner 不改。详见 [原生 JS](SURFACE_JAVASCRIPT_NATIVE.md)。
五个新文件 102–211 有效行，没有新增例外；JS 22/2 ignored、相关资源/设备/窗口回归、
严格静态/版本/材料合同及 3,486 文件行数门通过，完整产品/迁移边界仍另验。

v340：surface_javascript::Session 独占持久 JS child/管道/序号/权威快照；worker 只在
私有入口核对 Job 后读帧，engine 仅在 child 内真实求值。bootstrap 拥有受限 DOM/
listener/生命周期，先完成可运行 Promise jobs 再 collect；wire/event 分别拥有
有界协议及父侧权威动作复核。原 SurfaceWindow/Presentation/ClickEvent 的呈现、
候选、随机 ID/FIFO 授权 owner 不变。测试 fixture 只供库/真实隐藏 HWND 门，产品
worker 另有显式摘要绑定门；源码身份额外绑定第三方 MIT 声明。没有生产 UI 接线、
新网络/文件宿主或默认 capability；不是完整 JavaScript/沙箱/迁移验收，完整 owner
与预算见 [SURFACE_JAVASCRIPT_NATIVE.md](SURFACE_JAVASCRIPT_NATIVE.md)。

javascript_licenses 仅拥有 rquickjs-core/sys 0.14.0 的显式许可证补充：复用同解析图
rquickjs 的仓库级 MIT 与 sys/quickjs/LICENSE 原字节，精确来源/版本/表达式/上游摘要
均需匹配；licenses 仍拥有全图、总量、写入和 inventory，source 绑定其输入。
原始文本缺失/篡改、peer 重复或路径越界仍拒绝，不扩展为通用 SPDX 文本替代。
javascript-license-supplements 测试只修改显式临时根的原文副本；原材料合同继续验证
真实 metadata 的全图收集，28 项通过，不删除失败原件或跳过缺失包。

v339 续进没有新增生产模块或改变 owner。实际产品构建与封存后的资源/启动三套门
通过（2/1/7）；菜单真实服务夹具仍因正常前台准入失败保持 partial。隔离 driver 的
显式 Win32 desktop/kill-on-close Job/精确 HWND 和 PID 校验及 UIA 菜单客户端仅留
linshi，不加入产品或放宽生产 foreground/capture 门；修正后夹具尚未整链重跑。
产品源码/版本/依赖不变，旧证据不改；详情见迁移台账及原生构建入口。

v339：application_surface_real_loom_tests 归既有 Application remote_live/resources 测试
owner，只增加显式 ignored 真实服务门；production authorization_owner/surface_start/
poll/AuthorizedWindow 和网络期限不变。外部隔离夹具负责 daemon/原始依赖包/loopback
TLS 与精确新设备批准，不负责伪造 token、Outcome 或 ACK；本轮脚本/私有夹具都留在
linshi。新测试持有自有 COM/Application/窗口，显示前移出虚拟屏幕且不激活，无全局输入，
关闭及完成句柄沿原 owner 回收。新/接线文件 199/239 有效行，无新生产 owner、线程、
队列或依赖；真实门 1、resources 17/1 ignored、非 desktop Surface 606/10 ignored、
授权 84、启动 2 及严格静态/版本/3467 文件行数门通过。夹具失败原件保留；不是实际
产品 EXE、用户在线 Loom/JavaScript 或完整迁移验收，详见迁移台账及原生构建入口。

v338：surface_scene 仍独占声明式渲染前合同；仅四个 Loom 可选属性 Map 的反序列化
接受 missing/null，不放宽 events/children 或非对象，不在 HTTP 层改写原始快照。
surface_scene_tests 拥有五项空值/对象保留/非法类型/集合严格性/边界回归；原
surface_binding_http_tests 将真实 null 形态接入 TLS remount→declarative_scene，
沿原场景 owner 处理。没有新模块/依赖、网络/线程/队列或授权 owner；修改 Rust 为
236/226/91 有效行。Scene 11、非 desktop Surface 606/10 ignored、Application
attach/image 两项及严格静态/版本/3466 文件行数门通过。外部原生驱动的未绑定
窗口只验证呈现/输入关闭，不伪造 grant 或调用私有编辑挂载；真实设备/Application/
JavaScript/完整桌面与迁移门仍开放。证据在 linshi/otter-native-v338-regression。

v337：tool_panel_content 独占隐藏 pinned 面板 BUTTON 首次准备、BUILDING/READY、
首次缓存投影与展示前门；tool_panel_children 继续拥有真实 BUTTON 创建/几何，临时
Vec<Window> 成功后转交父 HWND，失败全部回滚。普通隐藏工作区仍 eager；可见
Application 在 window_state 中原事务提交前显式 prepare，恢复非 pinned 交换边界不变。
tool_panel_content_tests 拥有七项真实 HWND 生命周期/首次状态/滚动/失败/重入回归，
原 layout/capture/工具断言未删。没有新线程/队列/轮询/依赖；panel/字体/图标/截图保护
owner 不变，隐藏 pin UIA 子树仅在首次使用后拥有 BUTTON，完整 UIA 验收未结清。
面板初轮 28 项、额外真实 ShowWindow 一项及 composition 102 passed / 4 ignored 通过；
READY 只标记集合创建，既有 best-effort 刷新/重入补刷与完整 UIA/TAB 边界保留。
证据在 linshi/otter-native-v337-regression。

v337 最终面板 29 项及严格门、Shader 固定三轮和六套 EXE 有完整通过日志及九摘要
绑定；追加诊断后原索引及四份诊断文件出现全零/NUL 尾段，原件保留且原因未证。
本轮 partial，独立现态复核在 linshi/otter-native-v337-recovery；不补造原索引或以
部分完整日志声称整个原始证据链完好，不改变完整迁移门禁。

v336：composition_device 独占同 UI 线程默认 D3D11 设备的健康检查、hardware/WARP
创建和 Weak 租约池；窗口合成树/交换链/D2D context 仍归 composition，显式 Live
设备不进入默认池。composition_device_tests 拥有状态机注入/回退与真实线程隔离，
composition_sharing_tests 拥有真实多窗身份/像素/十钉图释放，复用既有 samples
读回 owner；无新线程、队列或常驻设备强引用。直接合成回归 102 passed / 4 ignored，
新三文件 65/104/194 有效行，原 composition 204→198 行，原测试 161 行，无例外。
Clippy/check/fmt/版本/3,464 文件行数通过，但普通完整 smoke 初次仍超时，本轮
partial，不结清旧失败或完整 N11；证据在 linshi/otter-native-v336-regression。

v335：tool_panel_children 独占面板子 BUTTON 创建和实际 HWND 几何更新，复用原
tool_panel_layout/scroll，不缓存句柄或几何、不改变父面板/字体/绘制/命令 owner。
按最终几何创建仍保留最后校正；不匹配或读取失败才沿原 SetWindowPos 路径定位。
tool_panel_layout_tests 只对子夹具按钮安装临时 WNDPROC，计数指针有真实 Box owner，
先还原回调再释放；panel→owner→State 的释放顺序保护窗口回调生命周期。三项真实
隐藏 HWND 回归及原工具面板相关共 22 项通过；Shader 65 passed / 15 ignored、严格
Clippy/check/fmt 通过。固定三轮正式正向 2 passed / 1 failed，完整 smoke 仍 partial。
额外 build 的 EXE 身份变化已拦截并另绑定新产物验证，不覆盖失败或结清旧问题。
没有新运行时线程、队列或行数例外；证据在 linshi/otter-native-v335-regression。

v334：shader_candidate 独占固定 pin/精确两文件/同句柄摘要准入，子模块 files 和 acl
分别拥有逐级固定盘租约/名称空间与句柄 owner/DACL。TrustedBundle 持租约，固定
候选 child 重新取得租约；worker 兼容清单合同不冒称经过新 pin 门。startup_settings
只提取显式路径，startup_entry/shader_startup 在截图/建窗前准入，原窗口/COM owner
不变；application_shader_startup 只安装候选及运行显式真实 smoke，复用生产菜单、
输入、认证取码、缓存、child 与正式提交，不增加生产线程/队列。测试 owner、命令
与持续私有权限/历史句柄边界见 [SHADER_LOCAL_CANDIDATE.md](SHADER_LOCAL_CANDIDATE.md)。
最终 Shader 65 passed / 15 ignored、参数/默认拒绝各三项、严格 Clippy/check/fmt/
版本/3,459 文件行数和摘要绑定六套 EXE 集成 26 项通过；八新文件 53–178 行，
修改 Rust 最大 498，无新例外。九文档 UTF-8/本地链接通过。实际正向虽有复验/
计时诊断通过，最终默认计时门仍在原 15 秒超时，本轮 partial，不追溯结清旧失败；
默认能力/安装及完整迁移边界不变，证据在 linshi/otter-native-v334-regression。

v333：本地 ANGLE 材料消费归 scripts/native-candidate，不修改 native-angle builder。
angle_intake 复用原 stage/PE/工具链门，angle_notices 负责原字节声明选择，angle_sbom
只产材料清单；angle_materials 编排白名单复制，angle_materials_evidence 关联封存
证据，angle_materials_verify 独占外部预期摘要/精确集合/独立核验。复用路径、JSON、
Windows 名称空间与流式写入 owner，预算在写入前执行；不执行快照源码，不新增应用
线程、队列或安装 owner。159 清单文件/109 声明的真实候选、独立 verify、官方 schema
及材料 payload 的硬件/WARP 外部驱动门通过，不等于法律/可信安装/产品或发行批准。
完整入口与边界见 [ANGLE_TOOLCHAIN.md](ANGLE_TOOLCHAIN.md)；Rust/默认 capability 不变。
105 Python/9 Node 合同、fmt/版本/3,451 文件行数门通过；九新文件 30–120 有效行，
没有新增超限例外。34 Python AST/七文档 UTF-8 与本地链接通过，原生 1,767 项输入
及旧 EXE 不变；不是完整桌面库或迁移放行。

v332：固定 DLL 构建归 build_fixed 编排；build_inputs 独占 fresh stage/外部预期摘要/
前后集合，build_job 独占 suspended→Job→resume、精确句柄继承和有界回收，
build_binary 独占不加载 DLL 的 PE/必需可执行导出/普通及 delay-import 静态 CRT 门。
复用已有路径/环境/profile，不新增应用 owner、线程、队列、安装器或默认 capability。
调用、输出与信任边界见 [ANGLE_TOOLCHAIN.md](ANGLE_TOOLCHAIN.md)。
81 合同、fmt/版本/3,442 文件行数门通过；八个新文件 74–167 有效行，原路径/JSON
owner 的必要接线 94→96 行，无新增超限例外。原生输入/产品程序/默认能力不变。
最终源码绑定的新树真实完成 504 步 DLL 编译及无工作/零诊断门（558.765 秒），
加固 PE/静态 CRT 和新夹具硬件/WARP 外部驱动门通过；不是实际产品 EXE/完整 UI。
旧弱门构建主动中止并保留，完整源码/工具/许可、可信安装与迁移验收继续开放。

v331：固定本地 source/tool 暂存归 stage_build 编排，复用 prepare_profile，不复制
adapter。stage_pins 独占身份，stage_git 独占只读对象/进程回收，stage_archives
独占数据归档，stage_inventory 独占 Windows 名称空间/预算/最终文件清单。新增测试
夹具及 materialize/staging 合同归 scripts/tests；不新增运行时 owner。实际暂存
12,569 个构建文件，最终树真实 GN/计划与首轮内容等价、完整 DLL 和显式外部驱动
硬件/WARP 门通过，不等于实际产品 EXE/完整 source closure；66 个 gitlink 未提供。
54 合同与全库 3,434 文件行数门通过，新八文件 23–132 有效行。完整工具/许可/编译
执行/可信安装和迁移边界仍开放，详情见 [ANGLE_TOOLCHAIN.md](ANGLE_TOOLCHAIN.md)。

v330：scripts/native-angle/gn_profile 独占固定上游 GN profile 变换/断言/args；
prepare_profile 复用 prepare_toolchain，拥有外部 overlay 编排、输入前后复核与
profile 回执，不成为完整源码获取/staging/编译 owner。测试归 scripts/tests 的
test_native_angle_profile，复用原合成夹具，涵盖固定 guards/pin/双架构/输出清单/
拒绝门及真实 -I CLI；不新增运行时 owner、线程、句柄或队列。Clang/lld+MSVC STL
静态 CRT 的真实 DLL 构建、仓库输出消费、Ninja 原生 GN 再生与三项头依赖失效门
通过，无生成后 prefix patch；Clang 排除系统头的 5/24/24 不等于完整内容闭包。
新 DLL 的 PE 与公共库外部驱动硬件/WARP 像素/child 门通过，不是实际产品 EXE。
完整 staging/许可/可信安装与完整迁移仍缺，原生 Rust/用户程序/默认 capability
不变。详见 [ANGLE_TOOLCHAIN.md](ANGLE_TOOLCHAIN.md)。
构建合同合计 35 passed，fmt/版本/全库 3,426 文件行数门通过，没有新增超限例外。

v329：prepare_toolchain 仍是固定 setup_toolchain adapter 的唯一转换 owner；只添加
精确无路径 marker→空 runtime_dirs 的窄接线，不新增环境/运行库/资源 owner。
test_native_angle_adapter 保护空值等价、真实 CLI 和近似 marker 拒绝；环境 owner
显式请求 VSLANG 而不继承调用者，缺英文资源仍须消费方验证实际前缀。25 项合同
通过。用户授权的真实 SDK、GN 实验树、固定源获取及静态编译产物全部留在 linshi，
不是新增仓库 staging/SDK 安装器。GN 111 targets/完整 DLL 计划 dry-run 402 命令及
四个静态目标编译不等于完整工具/DLL/安装/重放闭包。中文 MSVC 的 0→121 头依赖
及真实 mtime 失效门仅验证实验 post-GN prefix 接缝，尚未新增可复用仓库 owner；详见
[ANGLE_TOOLCHAIN.md](ANGLE_TOOLCHAIN.md)。后续固定资源工具、两 DLL 构建及新鲜
公共库外部驱动的硬件/WARP 像素与 child 重放已通过；实验脚本、驱动与逐文件证据
全部仍在 linshi，不新增仓库 owner，GN 再生与正式安装边界未完成。原生及 adapter
源码、用户程序和默认 capability 不变，不把外部驱动当实际产品 EXE 验收。

v328：scripts/native-angle 是固定 ANGLE 构建工具链接缝 owner，不归应用运行时或
候选组包器。prepare_toolchain 是标准库 CLI；toolchain_paths 拥有路径/schema/
anchor，otter_toolchain_env 拥有隔离环境与无回退消费。测试归 scripts/tests 的
native_angle_fixture、test_native_angle_environment、test_native_angle_adapter，
只在显式仓库外临时根写合成数据。入口/输入输出/副作用/退出码及未验证边界统一见
[ANGLE_TOOLCHAIN.md](ANGLE_TOOLCHAIN.md)。22 项合同通过不等于 SDK/GN/DLL 构建、
完整供应链或安装授权；原始 checkout、Rust、用户程序与默认 capability 不变。

v327：loom_art_config 仍是唯一公共配置编码 owner；仅两条 Shader 快照分支增加
强制模式与真实图片端口重名的 InvalidValue 拒绝，普通 Art 的公共 ports 校验不变。
application_shader_mode_tests 归原应用 Shader 测试模块，复用 Class/COM/TLS，
验证无同名参数声明时的三类冲突、worker 未启动和旧请求/preview/正式输出保留；
没有新生产 owner、线程、队列或句柄。新回归先红后绿；配置/协议 13 项、Shader
60 passed / 15 ignored、硬件/WARP 应用 16 次真实 child、最终摘要绑定六套集成
26 项及 Clippy/fmt/3417 文件行数/diff check 通过，修改 Rust 均不超过 500 有效行，
源码/EXE 不变。证据位于 linshi/otter-native-v327-regression；定向门有交集，完整
桌面库未重跑，不追溯宣称旧间歇问题已修复；默认 capability 和完整迁移/发行边界不变。

v326：session_file_trace 是下层匿名计时 owner，沿既有显式 smoke timing 开关启用，
默认关闭；guard/write/replace 只包装原操作，不转移文件、取消、恢复或清理所有权。
仅输出静态阶段、匿名 ID、字节数/耗时/状态，输出失败不改变业务结果；无新线程/队列。
一次诊断 231 项完整配对，不能据此认定旧超时原因或弱化同步/完整基线比较。文件事务、
capture-file、真实保存调用方 29/5/19 passed；0-test filter 不计覆盖。正式集成六套
26 项及最终 EXE 摘要绑定的无编译复验均通过，原 15 秒期限不变；Cargo 重编译造成
的初次 EXE 身份失败保留，诊断/最终 EXE 分开绑定。Clippy/fmt/3416 文件行数/diff
通过，修改 Rust 均不超过 500 有效行；源码前后相同。证据位于
linshi/otter-native-v326-regression；没有性能修复或完整库重跑，旧失败及迁移边界保留。

v325：desktop/shader_recipe_cache 独占 UI 线程配方 LRU、身份/上下文键与 payload
字节租约；只在应用 commit_checked 成功后记录原响应，不保存输入/像素/GPU/child。
loom_input_fingerprint 是原 loom_input_batch 的子模块，仅在输入 worker 内为显式
标记的主图/参考图计算内容摘要，不新增来源权或新鲜度 owner。shader_prepare 仍
独占当前参数/同份 PNG 与原配方合并，缓存不写回参数覆盖；shader_execution_job
的命中分支复用 Tasks 两槽、取消/完成栅栏和原 child 单容量，不创建远端 Session。
loom_capability_runtime 统一主图/参考端口的旧图片筛选，避免通用 image-like 名称
推断扩大上下文；application_window_state 接回原样 sync_capture_windows，维持
Application 的既有职责边界与行数上限。shader_recipe_cache_tests 拥有身份/LRU/
租约/默认值与上下文合同，其 platform 子模块拥有真实双缓存任务/容量/取消门。
Shader 定向 57 passed / 15 ignored、应用 16 次真实 child（8 次取码/8 次缓存）、
双缓存四场景/六次成功 child、输入 worker 15 项及严格 Clippy/fmt/3415 文件行数/
diff check 通过；修改 Rust 模块均不超过 500 有效行。可选 Acquired 元数据以 Box
持有，避免膨胀共享 Completion 枚举，没有 lint 例外。资源/EXE/会话启动 2/1/9
passed，但 startup 5 passed / 2 failed：两项完整冒烟在原 15 秒期限超时，根因
未确证；两项各一次计时隔离诊断仍超时，隐藏窗口 application 12766 ms 成功结束，
透明钉图最后计时为 groups begin，不推断单一阻塞点或与缓存/v324 前台失败同因。
原始失败及初次 Clippy 失败均保留，不放宽测试。源码前后一致、最终 EXE
另绑定产品/后端与本地平台/私有入口拒绝复验，partial 回执位于
linshi/otter-native-v325-regression，不宣称完整启动通过。
完整桌面库本轮未重跑，不以定向证明替代 v324 未通过的桌面门；默认安装/capability、
正式供应链和完整迁移边界不变。配方缓存不是常驻 renderer 或 GPU 程序缓存。

v324（定向验证通过，完整桌面回归未通过）：新增 application_shader_inputs，作为 application_image_inputs
的子模块，只筛选同轮公共 Params 的本地 Unit 身份并解析正式 Graph 图片。候选
图视图与最终来源依赖分离，只有实际使用的引用及其上游参与 Source freshness；
图片捕获/编码/准备/预算仍归原 owner。application_parameters 的单次 resolved
回调同时冻结来源、Configuration 和 Runtime，不重读 Shader 原始手动值。
application_shader_unit_tests 是原 Shader 应用夹具的子模块，复用 Class/COM/TLS，
覆盖未连接目标的中继、历史参数/声明参数/标量覆盖和来源失效；不增加线程/队列/
缓存或安装 owner。loom_art_config 统一拥有同名图片输入替代标量参数的 wire
规则，避免 Loom adapter 合并参数时碰撞；Runtime 仍保留原 resolved 快照。
最终源码 Shader 定向 50 passed / 14 ignored，应用平台 16 次真实 child、严格
Clippy、资源/EXE/会话启动/startup 2/1/9/7、fmt、3411 文件行数及 diff check 通过；
修改 Rust 模块均不超过 500 有效行，源码前后相同，最终 EXE 已另绑定摘要复验。
完整库两次分别 3181 passed / 5 failed / 73 ignored 和 3185 passed / 1 failed /
73 ignored，不能结为全绿。首轮 wire 旧期望已修正；四项 Surface 隔离通过但根因
未确证，UIA select 隔离仍失败且记录外部应用成为前台，未操纵该用户应用。
完整库间生产源码未变，最终测试类型别名整理后未重跑完整库。证据及 partial 回执
位于 linshi/otter-native-v324-regression，无本次原生进程/运行临时文件残留，Node
编译缓存保留；默认 capability/发行及完整迁移边界不变，未组包或提交推送。

v323：shader_parameters 独占公共图片参数与同名准备端口的相交/保密/禁用筛选，
Runtime 只冻结名称；shader_prepare 从原 PreparedBatch 取同源 PNG，复用解码、
总纹理预算及原执行/提交 owner，不另解析图或访问路径/URL。loom_image_value
新增 png_base64 接缝供恢复和输入共用；loom_input_data 的内联 PNG 转入原 Formal
准备链，保留原字节并按像素加 PNG 计预算，其他格式转换不变。新增
application_shader_texture_tests 是原应用 Shader 测试的子模块，复用 Class/COM/
TLS/主循环夹具；参数与来源 owner 的原测试另保护低 alpha/透明隐藏 RGB、原字节、
数量并集/预算/取消及准备后输出/拓扑变更。没有新生产模块、线程、队列或安装 owner；
完整库复测 3181 passed / 73 ignored，Clippy、资源/EXE/会话启动/startup 2/1/9/7、
fmt、3408 文件行数与 diff check 通过，修改 Rust 模块均不超过 500 有效行；原生
源码前后一致，最终 EXE 已另绑定摘要复验。首轮 6 项桌面/UIA 失败与原日志保留，
同源码重验成功不等于 UIA 间歇断言已修复。回执位于 linshi/otter-native-v323-regression；
固定源码/SDK 包获取只是 linshi 内供应链实验，不新增仓库安装器或构建 adapter，
默认 capability/发行边界不变。

v322：loom_capability_runtime 独占声明/标准端口的 Shader 主输入优先级，取代
shader_prepare 的重复选择。application_image_inputs 仍是唯一图到准备来源的
owner，仅为可信 Shader 目标添加真实自身 raster base 候选；普通 Art 不增加回退。
loom_input_batch::Source::capture_base 从 editor.layers 取底图并保留真实文档凭证，
复用原捕获准入/预算与编码 owner，不采纳可能只有预览的 Canvas.image，也不
二次烘焙标注/裁剪。没有新增生产模块、线程、队列或资源生命周期 owner。
application_shader_fallback_tests 是原应用平台测试的子模块，复用 Class/COM/TLS
夹具验证缺参考图、preview-only 恢复和新文档拒绝；原平台门扩展连接优先/自身
底图的八次真实 child 提交。loom_input_output_source_tests 另验原始尺寸/像素及
文档凭证；完整库 3177 passed / 73 ignored，Clippy、资源/EXE/会话启动/startup
2/1/9/7、fmt、3407 文件行数与 diff check 通过，修改 Rust 模块均不超过 500 有效行。
原生源码前后一致，最终 EXE 另绑定摘要复验，回执位于 linshi/otter-native-v322-regression。
默认安装/capability、正式供应链和完整迁移边界不变，未组包或提交推送。

v321：desktop/shader_parameters 独占公共 resolved Params 的不可变 Shader 绑定，
有限数值/布尔和内联 PNG 覆盖同名 response 初值，主 input 不可被参数纹理替换。
application_parameters 在同一次参数解析中冻结取码 Configuration 与 Runtime；
loom_capability_runtime 拥有主 Shader 输入端口推导，shader_prepare 只从同份
PreparedBatch 选择主图。Pending.shader→Execution.shader→统一 Job 是单次移交，
原 Tasks 仍独占两个槽、退休/取消/清理；不再保留独立开发期 Shader fetch 启动入口。
application_execution_start 仅在可信 Installation 存在且为允许的纯 Shader 时放行，
参数/上游自动开关及原注册/来源提交凭证不变；Application 构造仍不安装 bundle。
shader_parameter_tests 拥有冻结/筛选/预算/覆盖合同，application_shader_start_tests
复用原 Class/COM/TLS fixture，验证普通拒绝门和硬件/WARP 的菜单/参数真实 child
应用提交。测试子进程不是产品用户入口或实际 Loom 服务；Installation 不是来源、
ACL、许可证验证或安装器。通用图片/Unit 参数、自身 fallback、实时缓存和正式
供应链仍待完成；String/JSON 快照不属于 Pixels/child Job 硬预算。完整库 3174
passed / 73 ignored，Clippy、资源/EXE/会话启动/startup 2/1/9/7、fmt、3406 文件
行数和 diff check 通过，修改 Rust 模块均不超过 500 有效行，原生源码前后相同。
回执位于 linshi/otter-native-v321-regression；最终 EXE 另绑定摘要复验，未组包
或提交推送。

v320：desktop/shader_payload 拥有正式主端口/首个 content 到原样 Shader 配方的合同；
shader_payload_decode 独占源码和流式绑定数量准入，不在 wrapper 中建立第二棵完整
Value 树。image_decode_shader 拥有静态 PNG→straight RGBA 的形状/容器准入，复用
image_decode 的唯一受预算像素转换 owner，不把 raw RGBA 包装成 Image。
desktop/shader_prepare 从同份 PreparedBatch 建立同源 input 和必需的内联 PNG 纹理，
shader_fetch_render 只在真实远端交付已释放/消费后准备完整本地派生结果；原 Delivery
保持真实身份和 lease owner。loom_execution_job/session/tasks 仍拥有唯一网络、取消、
线程、两槽和清理生命周期，不新增队列或常驻 worker，不把本地失败误作远端未完成。
shader_prepare_tests 拥有字段/PNG/预算/覆盖与完整浏览器纹理摘要；loom_shader_fetch_tests
和 cleanup_tests 复用原 TLS fixture，验证 exporter 认证、实际 child、正式提交和取消/
租约拒绝。Loom 业务算法不复制到 Otter，正式安装和应用入口继续未接通。
v320 完整库 3169 passed / 72 ignored，Clippy、资源/EXE/会话启动/startup 2/1/9/7、
fmt、3403 文件行数和 diff check 通过；最终 EXE 摘要另绑定平台门重验。新增及
修改 Rust 模块均不超过 500 有效行；回执位于 linshi/otter-native-v320-regression，
没有组包、替换用户程序或提交推送。

v319：shader_process_request 独占父侧跨线程 Shader 源码/纹理绑定的 owned 描述，
纹理共享 Pixels，准入复用 wire；不持有 HWND 或从 Art 推断可信 bundle。
desktop/shader_execution_job 独占本地按需线程、许可、取消和完成栅栏，复用
shader_process 与 shader_image，不复制进程/Job/编码器。loom_execution_work
显式区分 Remote/Shader 的领域清理，原 loom_execution_tasks 保留唯一两槽集合、
退休/轮询/回收；loom_result_commit 复用正式事务，本地 Completion 另验节点/请求
且只消费一次。Application 继续拥有配置/拓扑绑定、主循环和传播，不增加队列。
shader_execution_task_tests 拥有确定性事务/容量/异常测试；runtime_tests 拥有
显式真实 child 的硬件/WARP 完整 alpha 矩阵门；application_local_shader_tests
复用原隐藏图链和真实主循环，分别验证七个确定性生命周期场景及两个真实后端。
用户入口/取码纹理 staging 与供应链尚未接通，不能把公开本地任务 API 当能力声明。
v319 完整库 3160 passed / 71 ignored，Clippy、资源/EXE/会话启动/startup 2/1/9/7、
fmt/3395 文件行数/diff 通过；新增及修改 Rust 模块仍不超过 500 有效行，证据位于
linshi/otter-native-v319-regression。没有组包、替换用户程序或提交推送。

v318：desktop/loom_image_value 独占正式 Image+可选原始 PNG 的不可变 owner；
同源构造仅供可信编码/同份字节解码的内部调用，别名/快照只克隆 Arc。output_batch
统一普通图片和保留 PNG 图片的读取，shader_image 只生成本地 Batch，不伪造远端
lease。result_state 继续独占正式端口原子提交/OutputStamp/预算；输入/保存/恢复
复用 ImageValue，不新增编码器、线程、队列或图调度 owner。普通无 PNG 图像复用
export_raw；编码不在 UI 发生。application_png_chain_tests 复用原隐藏应用链 fixture，
验证显示/中继/传播准入和真实下游准备后恢复现场，不重复远端执行。
owner 五项、真实会话保存恢复及应用图定向通过；异步 Shader 调度、供应链和能力
声明边界不变，不把数据 owner 接通当完整应用 Shader 功能。
v318 完整库 3155 passed / 69 ignored，Clippy、资源/EXE/会话启动/startup 2/1/9/7、
fmt/3389 文件行数/diff 通过；新增和修改 Rust 模块均不超过 500 有效行，回执
位于 linshi/otter-native-v318-regression。未组包、替换用户程序或提交推送。

v317：desktop/shader_image 独占旧 Canvas raw RGBA 的 alpha 解释与完整 Image+PNG
准备；不持有应用状态、线程、队列或图端口。export_raw 共用几何/预算/COM/
WIC/流复制/取消 owner，仅增加显式 straight BGRA 入口，不复制 PNG 实现。
shader_image_tests 拥有独立浏览器矩阵摘要、真实 WIC 和失败原子性回归；
shader_image_runtime_tests 作为原 Shader child 平台测试的子模块，复用同一
launcher/client/可信 fixture，组合完整矩阵 oracle 验证真实产品硬件/WARP 的
矩阵、Loom 原样 Shader 和 4K。旧合同的 PNG→Canvas2D 后值不当 straight oracle。
模块只做后端准备，能力报告/应用 pure-shader 拒绝和正式供应链边界不变。
v317 完整库 3149 passed / 69 ignored，Clippy、资源/EXE/会话启动/startup 2/1/9/7、
fmt/3386 文件行数/diff 通过，改动 Rust 模块均不超过 500 有效行；回执位于
linshi/otter-native-v317-regression，未组包、替换用户程序或提交推送。

v316：bounded_worker_process 独占计算 child 的进程/Job/容量 lease/句柄继承与回收，
领域 Policy 固定入口、上限、容量池和环境；bounded_worker_environment 仅提供
空环境或 OS API 真实 SystemRoot 白名单，不采纳父环境。surface_image_worker_process
保留图片 128/256 MiB 策略，shader_process 拥有独立 256 MiB 单 child 策略和可信
bundle 类型；client 独占完整帧/正常退出/EOF/最终取消发布门，worker 独占私有入口
和单次 ANGLE 生命周期。shader_worker_wire/request/response 分别拥有 Shader 领域
准入、逐项流式输入和 raw RGBA/typed failure，不复用图片 PBGRA wire。
shader_process_tests/worker_fixture 拥有实际故障 child，runtime_tests 复用既有
浏览器合同夹具和同一 launcher/client 验证真实产品 EXE、4K、Job 峰值、退出及
容量归还；测试只观测进程/Job，不拥有生产终止策略。没有新增常驻线程、队列
或帧循环，应用 Shader 派发/能力声明继续关闭，PNG/PBGRA 桥接与正式 bundle 待完成。
v316 完整库 3144 passed / 66 ignored，显式 ANGLE/child 平台门 6/3 passed，Clippy、
资源/EXE/会话启动/startup 2/1/9/7、fmt/3383 文件行数/diff 通过；改动 Rust 模块
不超过 500 有效行，前后原生源码身份一致，证据位于 linshi/otter-native-v316-regression。

v315：shader_angle 拥有唯一进程 EGL context 与线程亲和、描述和释放；api 只拥有
固定 ABI，library 独占可信 DLL 文件核验/锁/受限加载/路径检查/卸载，render 拥有
单次原样 GLSL 编译、纹理、uniform、读回及失败/取消清理，不拥有网络或应用状态。
API 错误永久 poison，编译失败与输入拒绝不混作 context loss；逻辑像素预算不是
驱动/编译硬上限。shader_angle_tests 覆盖模型和显式硬件/WARP 平台门，contract_tests
只消费真实旧生产 ShaderRenderer 的浏览器夹具及固定 Loom shader 身份，不复制
Loom 业务算法。两后端三组 raw 像素合同精确通过，旧 PNG/显示桥接未实现。
后端尚未接入应用或能力声明，必须先有 child/Job/期限和可信发行 bundle；固定
摘要测试 DLL 不入仓库/包，不新建长期线程、队列或 GPU 帧循环。
v315 定向 6、完整库 3133 passed / 61 ignored、Clippy、资源/EXE/会话启动/startup
2/1/9/7、fmt/3371 文件行数/diff 通过；原生源码前后相同，测试进程零残留，回执
位于 linshi/otter-native-v315-regression。改动模块均不超过 500 有效行，未组包或提交推送。

v314：live_input_message 独占最终投递的接收端 DPI 作用域，复用
input_coordinates::Dpi 的 RAII，不把映射 worker 的默认上下文当作目标上下文；
键盘不切换，原预检/身份/发送期限不变，不增加状态 owner、线程、队列或轮询。
live_input_delivery_dpi_tests 拥有自建隐藏接收窗口与发送端的九组合精确坐标、
横/纵滚轮和实际超时恢复 oracle。composition_live_crop_runtime_tests 是原整窗
input_runtime_tests 的子模块，复用同一 source/Live/WGC/恢复 child 夹具，覆盖两种
样式乘三种结束路径；只扩展夹具的图案与接收坐标记录，不复制生产采集/输入实现。
真实 WGC 颜色检查仅为中心反馈，源销毁清理明确区分 skipped 与 released。
v314 定向 1/2/1/6、完整库 3129 passed / 59 ignored、Clippy、资源/EXE/会话启动/
startup 2/1/9/7、fmt/3365 文件行数/diff 通过；改动 Rust 模块均不超过 500 有效行。
回执位于 linshi/otter-native-v314-regression；未组包、替换用户程序或提交推送。

v313：capture_live_selection 拥有冻结前有界 Target 快照、程序窗口过滤和选择后
身份/几何重验；capture_windows 共用 EnumWindows 并保留 HWND，不持有进程 owner。
capture_region 共用原事件/绘制，只为 Live 交付矩形和整窗意图；原选择回归迁入
capture_region_tests，与新增隐藏 HWND Live 手势测试同属该 owner。
live_window_target::region（live_window_region）独占局部 Crop、屏幕投影、纹理尺寸
合同与输入投影；live_job/WGC/Session 只移交此合同，不复制输入或采集算法。
Application 共用整窗/裁剪的源保护，WGC 在原事件路径核验 frame/content 并裁剪，
不增加轮询、线程或来源回退。live_window_region_tests 负责模型/自有窗口坐标/
WGC item 几何，live_texture_tests 负责实际 WARP 像素；item 探针不启动捕获。
v313 定向 4/2/3/1、完整库 3129 passed / 57 ignored、Clippy、资源/EXE/会话启动/
startup 2/1/9/7、fmt/3363 文件行数/diff 通过；所有新增/修改 Rust 模块仍不超过
500 有效行，证据位于 linshi/otter-native-v313-regression。

v312：global_capture_registration 独占三键实际状态、附属冲突降级和关闭补偿，
global_capture 拥有键映射、共享菜单和恢复前后准入；Ctrl+2 最终调用原 Live
入口，不拥有第二套采集/worker。State 保存本窗口实际 Live 注册和共享菜单
引用，Application 只绑定/移交；恢复 guard/document/manifest/job 核验该状态。
global_capture_dispatch_tests 负责隐藏 HWND 准入及注入选择回调；应用热键和
恢复冒烟使用相同注册状态机及模拟系统适配器，真实 EXE 启动门强制核验新标记。
不新增线程、空闲定时器、系统键盘注入或真实全局键注册测试。
v312 完整库 3125 passed / 55 ignored，Clippy、资源/EXE/会话启动/startup 2/1/9/7、
fmt、3358 文件行数和 diff check 通过；回执位于 linshi/otter-native-v312-regression。

v311：long_capture_finish_route 独占 UI 线程内的结束通知、共享采集身份与窗口代次
核验；弱注册不延长会话寿命，最多一个通知在途。long_capture_dialog_finish 拥有
停止/排空/拼接状态和路由注册寿命，不拥有采集 worker 或像素。global_capture 只
在新截图准入前转交活跃会话，capture_input 只传现有采集身份。原 dialog 保留
reply/UI/结果交付，movement 在收尾期间拒绝命令。finish_tests 验证真实 worker
和隐藏控件组合，modal_tests 验证真实屏幕外 DialogBox 消息循环，route_tests
验证身份/代次/通知合并；不注册系统热键或读取用户桌面。

v310：surface_capabilities 继续独占新建/恢复共用的宿主声明，增加已接通的静态
image 与租约资源能力。application_surface_start 的合同测试验证报告边界，真实
TLS 图片 peer 独立校验 wire 内容；不新增资源获取、解码、授权或线程 owner。

v309：application_remote_live 将同一配置回调顺序传给表单、资源与生命周期调度；
application_surface_start 保持正式环境配置 wrapper，并提供相同准入的私有接缝，
不持有额外凭据/网络状态。application_surface_forms_tests 负责真实 TLS 挂载至
自动输入、回填及停用；application_surface_attach_images_tests/peer 负责真实
挂载至资源/内联图片安装、撤权丢弃和停用。复用原后台 worker、fixture 与预算，
不复制生产实现或把测试接缝变成公开运行参数。smoke_timing 只拥有显式诊断。

v302：surface_textarea 独占 textarea 默认样式和二维盒模型投影，现有布局
minimum 入口分派至它；Cache 保存静态 BoxStyle，滚动只变几何/裁剪。父 D2D
复用 Background/Border，surface_edit_box 拥有子窗口边框内缩、内圆角 region
及格式矩形，不复制遮挡算法。surface_control_color 独占 HWND palette 和
alpha 合成，Select 保留列表刷新，surface_edit_colors 负责 EDIT 销毁及带
parent/marker 校验的 CTLCOLOR。surface_textarea_box_tests 验证真实窗口与像素。

v301：surface_edit_alignment 只拥有 EDIT 对齐位更新、owner 重验与失效重绘；
Mount 只对 textarea 映射自身 textAlign，placeholder 读取实际 EDIT 样式方向。
surface_edit_alignment_tests 复用真实 WM_PRINTCLIENT 像素夹具；独立 mount
测试核验生产接线、同合同草稿保持及 input/number 未扩大自身样式。

v300：surface_control_font 是 EDIT/Select 唯一的 HFONT 创建、复用与替换 owner，
复用原 surface_select_font.rs 文件位置；Select 仅导入该模块，不再重复编译。
surface_edit_font 只拥有 EDIT 子类销毁顺序；surface_scroll_frame 缓存已解析
字段字体，surface_edit_mount 在布局同步时应用。surface_edit_font_tests 验证
真实字体描述、继承、DPI 往返、文本/选区保留与释放，不引入全局字体缓存。

v299：composition_live_chrome 独占 Live 物理客户区边角绘制与 L 形命中几何，
画刷随 Surface 释放，只写呈现 backbuffer，不改采集帧。composition_live_menu
拥有无框菜单入口、模态重入阻断及退出后 HWND owner 重验，复用系统菜单命令。
drag 复用黄角命中开启原移动 owner；input 负责状态菜单文字及单槽失败通知，
Session 移交通知，application_live 在无 Session 借用时展示已有原生错误提示。
composition_live_menu_tests 验证隐藏 HWND/注入菜单选择；Live GPU tests 读回
backbuffer 验证精确颜色与源像素隔离，不把合成夹具当真实 WGC 截图。

v298：composition_live_drag 拥有 Live 本地移动/等比拖动、开始标记、owner、
更新 revision 及捕获生命周期；State 只保存该 owner 所需状态。local 共用
客户区读取与定位补偿，placement::place_client_while 在同步定位前后校验
调用方凭证。pin_geometry::resize_drag 与滚轮共用 bounded_size，不复制预算。
composition_live_drag_tests 复用线程键盘快照/隐藏 HWND/WARP，重入 seam 只
注入确定性边界；分组同步显式退休本地 owner，不依赖源输入清理猜测。

v297：composition_live_local 拥有 Live 本地精确 modifier 滚轮路由，优先于
源输入，复用 pin_geometry::zoom 与 placement::place_client；调整失败尝试
恢复原外框，不升级成采集失败。State 保存期望 opacity，Host 保存并在 surface
重建时重应用；本地手势走原取消通道释放源输入，不自动重新授权。保存/回收
复用 application_save_live 的 opacity guard，冻结采用提交时值与准备后重验。
composition_live_local_tests 只更改测试线程键盘快照，使用隐藏 HWND/WARP。

v296：composition_live_placement 拥有隐藏 Live HWND 的初始物理客户区定位、
非客户区/DPI 有界校正及实际坐标复核；Session 在捕获 worker 启动前完成该步骤。
application_live 保留区域 RECT，窗口源在保护登记前读取 Target::frame_bounds；
后者复用 input_coordinates 的物理外框读取，不把输出位置当作 WGC 尺寸依据。
composition_live_placement_tests 复用已有 WARP 帧夹具，不读取用户桌面。

v295：application_live_unlink 拥有 Live 断线菜单的应用模态准入、确认后重验
与提交；application_links::can_remove_link 是静态/Live 共用的删除校验。
link_menu::Snapshot 共用分页、命令索引及快照请求，不复制连线目录或图状态；
LiveWindow/Session 只移交单槽请求。application_live_unlink_tests 验证隐藏
HWND 事务及显示/保存/执行退休，link_menu_tests 验证真实 HMENU 分页和禁用状态。

v294：application_live_links 只拥有 Live 出向连线的应用模态准入与确认后
提交；application_links 共用两端 owner 校验。link_create 的同一选择框支持
固定来源或固定目标，link_catalog::make_link 共用端口到 Link 的候选构造。
LiveWindow 只排队系统命令；application_live_link_tests 配合 link_create_tests
验证隐藏控件、事务、真实显示和保存，不新增图状态或执行实现。

v293：LiveWindow 的快照命令只排队；application_display_live 拥有刷新准入
和固定帧移交。application_display_live_cache 拥有按 owner 合并的有界待刷新
帧、成功替换/失败保留规则及提交通知；display_composition 继续独占 worker。
application_propagation 复用统一目标策略，支持 Live 快照来源，不复制执行队列。
application_live_refresh_tests 使用可控 WARP 帧流验证固定边界、通知和错误清理。

v292：application_display_live 拥有 Live 显示图准入和目标显示帧发布；
application_display_live_cache 是 display_composition 子模块，拥有 Live
缓存凭证与 GPU 读回准备，复用父模块的单槽 worker/完成通知/取消和析构。
父缓存按 Document/Live 区分来源，两个 retain 边界不互相退休资源。
application_live_display_tests 复用现有隐藏 HWND/WARP 和连线 fixture，
验证实际像素、中继、共享缓存、目标覆盖层及过期来源清理。

v291：link_catalog 纳入 Live 来源身份与 output，application_links 对两类
owner 统一执行准入/锁定校验。application_image_inputs 解析 Live 来源并移交
固定 GPU 帧，loom_input_data 在原输入 worker 内读回，loom_input_stamp 统一
静态/Live 新鲜度校验；Session 独占不可复用的输入 owner 身份。独立
application_live_execution_tests 复用隐藏 Live/WARP 和回环 TLS fixture，
没有复制采集、读回、编码或后端执行实现。

v290：application_live_group_name 独占 Live 组名编辑的模态准入、确认后来源
复核及 Workspace 候选原子发布，复用 shape_dialog，不另建名称输入实现。
composition_live_groups 拥有 Create/Rename/Assign 快照动作；应用循环分派。
application_live_group_name_tests 复用既有隐藏 HWND/WARP fixture，注入名称
选择验证取消、错误、stale 与保存身份；不拥有第二套采集或保存实现。

v289：composition_live_groups 拥有 Live 系统分组子菜单、单槽快照请求及窗口
隐藏/锁定展示状态；HMENU 由父系统菜单释放。application_live_groups 核验
来源/分组快照并调用既有 Workspace::assign_group；application_groups 统一
同步静态和 Live 成员并处理明确不回收的批量关闭。Session 输入边界拒绝隐藏/
锁定交互；无变化的 Live 菜单不重复复制目录或重建 HMENU。

v288：application_live_workspace 独占 Live 身份准入与跨静态/Live owner 的
工作区保留规则；collect_closed、Live 轮询和回收共用。application_live_freeze
只转交已登记身份，不通过通用新建导入重复注册。Workspace::replacement_unit
提供保留其他 owner 的宿主替换候选，替代会丢失 Live 的整拓扑重建方法。
application_save_live 捕获分组归属；独立 workspace tests 验证生命周期合同。

v287：application_live_recycle 只拥有 Live 回收请求准入及关闭后发布，复用
application_save_recycle_job 的单槽与策略。application_recycle_close 按目标
owner 分派静态/Live 提交；冻结条目身份/时间生成由 application_reference 共用。
LiveWindow 排队管理型 WM_CLOSE，close_for_recycle 核验真实 HWND 销毁，Session
保留输入/采集回收职责。独立 application_live_recycle_tests 复用 WARP fixture。

v286：application_save_live 拥有保存专用的 GPU 帧移交、Live 集合/几何凭证及
后台静态 Document 物化；不拥有线程或磁盘。application_save_snapshot 的
Receipt 同时复核静态文档及 Live，assemble 的完整 encode 先物化 Live，裸
assemble 拒绝未物化输入。save_state 保守区分运行流与已保存静态时间点。
application_save_live_tests 复用隐藏 Live/WARP fixture 验证实际保存和恢复。

v285：composition_live_freeze_geometry 独占 Live 客户区到物理图像区域转换，
复用 live_viewport，不复制呈现缩放算法。application_live_freeze 负责提交前后
几何复核；application_window_state::prepare_window_at 复用共享状态绑定和
DocumentWindow::create_at，不新增第二套静态建窗或图片缩放实现。

v284：application_live_freeze 拥有停止转静态的请求准入、来源复核和提交后停止；
live_freeze_job 只拥有后台读回、命中区域准备、取消与 join。LiveWindow 排队并
拥有创建时 UnitId，Session 保留此身份和 Workspace::IdentityStamp。工作区实例
凭证与变化中的 TopologyStamp 分离，不拿 Source 缓冲身份冒充长期工作区身份。

v283：live_texture_readback 独占显式 BGRA GPU→CPU 读回、staging/输出预算和 Map
生命周期；不启动捕获、不开线程、不属于连续呈现热路径。composition_live_host
负责最新帧 owner 移交，LiveWindow 保持无跨重入 RefCell 借用，Session 提供上层
入口。持久 Unit 绑定和冻结提交事务不放进 readback；端到端停止转换尚待接入。

v282：application_recycle_settings 拥有回收偏好编辑事务；recycle_dialog 负责
Win32 控件和严格输入，recycle_dialog_template 仅拥有对话框布局。运行策略归
Application，配置字段合并归 app_settings_document，持久化复用 naming_save_job；
application_recycle_settings_smoke 覆盖菜单到保存，表单选择注入不代表视觉验收。

v281：session_library_prune 负责已有回收数组的无 I/O 裁剪，复用 recycle_policy；
application_recycle_maintenance 仅拥有加载/设置触发调度，仍由保存 worker 单槽
承担线程和取消。application_save_recycle_job 的 prune 适配保留无变化标志及
策略凭证，application_reference 在 UI 复核后决定是否发布，不把 no-op 置脏。
独立 prune_tests 和 maintenance_smoke 验证模型及真实应用/保存链路。

v280：reference_ui 的 Action 区分收藏、取消收藏和关闭回收，沿用共享有界请求槽；
application_reference 只负责请求准入/worker 调度，application_recycle_close
拥有最后宿主准备、Receipt 复核、热键补偿、目标销毁与回收原文提交。
热键转交复用 application_restore_publish 的既有实现，不复制注册逻辑。
application_recycle_close_smoke 验证真实菜单到关闭、临时文件保存和恢复；
注入失败后端只验证销毁失败和热键补偿，不替代系统级热键验收。

v279：application_save_recycle_job 拥有回收请求参数与后台快照编码适配，复用
application_save_job 的线程/取消/完成生命周期。Purpose 显式隔离三种结果用途；
application_save_control 只领取 Save，库更新轮询不消费 Recycle。
application_recycle_prepare_smoke 从真实 HWND 捕获 Receipt/图片验证隔离，
不拥有关闭发布，也不替代未来关闭协调器或最后宿主/热键事务。

v278：session_library_freeze 拥有当前编码窗口的共同冻结边界，收藏与回收候选
复用相同的身份/字段准入和运行所有权剥离。session_library_recycle 只准备
关闭前库原文候选、移除目标旧窗口及相关链接，不拥有 HWND 或发布。
session_library_recycle_policy 独占软数量/天数策略及无区域猜测的日期解析；
app_settings_document::recycle_policy 读取已有 cache 字段，不混入临时文件策略。
实际关闭提交、最后宿主与热键交接仍待接线，不能把候选测试当作关闭功能通过。

v277：session_library_remove 只拥有按 entryId 删除单条的有界原文候选，不按
sourceStickerId 扩大删除范围。popup_frozen 为每条记录生成恢复/复制及删除
动作子菜单，沿用分页与稳定来源；Choice.remove 在应用交付边界分派。
保存 worker 的模式标记统一为 is_library_update，删除复用该单槽、Receipt、
取消与 edit_original 发布，不另建协调器。library_remove_smoke 验证两库
第二页删除、取消/stale、活动 HWND 保留及临时文件覆盖保存。

v276：reference_ui 拥有收藏/取消收藏的菜单命令与有界请求；
application_reference 复用现有保存 Receipt、单 worker 槽和取消回收，不另建
线程池。session_library_reference 仅从已编码当前模型构造库原文更新，
保留其他条目和根字段，不推进磁盘 revision；独立 tests 覆盖身份/容量/字段预算。
Record::edit_original 统一记录库编辑前的已提交磁盘基线，冻结追加也使用它；
Destination::Replace 显式携带该基线，正式保存/归档成功才清除，不能降低磁盘
精确比较。reference_smoke 验证真实菜单命令、后台快照、复制与多次文件覆盖。

v275：native_menu_navigation 仅拥有不关闭菜单的局部导航协议；popup_frozen
拥有双库分页与命令身份映射，popup_frozen_index 独占只读索引线程及回收。
frozen_library_ui 只共享来源快照和单槽请求；application_frozen_menu 在回调
返回后重新验证目标并调用既有后台追加协调器，不在菜单消息泵内准备图片。
新窗口 request 归回 application_commands，保持原排队/唤醒语义与公开入口。
独立分页测试覆盖每库 4096 项，隐藏 smoke 验证第二页选择到 HWND 追加。

v274：session_frozen_input 只拥有后台实例化输入和选中窗口字节准备，复用
session_job 的线程、完成通知、取消和析构回收。application_frozen_job 拥有
请求时 source/space/鼠标坐标捕获，复用 application_restore_job 的单任务
owner 与完成协调；后者将追加模式与 worker 原子建立，普通替换入口拒绝裸
Frozen 输入。frozen_job_smoke 验证真 worker 到隐藏追加的完整链，不另建任务池。

v273：session_library_retention 拥有冻结追加的保留原文候选和条目对应校验，
复用有界字段解析/序列化，保留未知值并只裁剪已关闭原文窗口及其失效链接。
application_frozen_append 拥有隐藏窗口追加与原文同时提交，不走整工作区替换；
既有 restore_batch 的窗口准备/guard 保持唯一实现。独立 smoke 验证真实
图片 HWND、stale 拒绝、拓扑保留、库消费、保存组装和清理，startup 断言标记。

v272：session_library_candidate 拥有冻结条目的单窗口 JSON 候选准备，
不拥有 HWND、文件权限或库消费。借用原始字段值、限制 256 个唯一快照字段，
复用 session_write::encode_bounded 的共享预算输出；其 tests 覆盖身份、
字段准入、位置、缩略、只读库及现有 session_job worker 的接入边界。

v271：session_library 拥有冻结库只读解析、身份/容量准入和原文借用；
session_document::Document::library 是按需入口，复用同文件有界数组
visitor，不新增磁盘或图片 owner。session_library_tests 验证未知字段保留、
按库隔离、超限整库拒绝与无 I/O 数据读取，未将模型测试当作交互功能完成。

v270：uia_invoked_client_tests 统一精确源/稳定祖先两种订阅方式与 PID
过滤，回调错误使用有界通道显式报告；uia_process_tests 的六个独立客户端
组合复用同一断言。生产 provider、窗口销毁和失效节点准入没有为测试改写。

v269：uia_process_tests 抽取本文件共用 run_process，统一操作与事件子进程
的期限、STA 泵和 Child 回收；uia_invoked_client_tests 的有界接收函数仅
向父测试模块开放，保持缓存、去重及注销释放断言共用，生产 API 无变化。

v268：native_menu_uia_process_tests 独占跨进程测试生命周期与客户端断言。
复用测试二进制的精确 ignored 入口，无生产 CLI 开关/额外发布二进制。父侧
最多泵 64 条消息后检查 20 秒期限，Child RAII 仅 kill/wait 自己启动的进程；
子侧 COM 事务及状态轮询各有期限，不经文件或管道交换用户数据。

v267：ItemStatus 标记映射留在 native_menu_uia 属性边界；status_tests 验证
COM ABI、禁用/分隔符、旧代次撤销及不伪造模式；status_client_tests 经真实
Windows MTA 客户端读取四种状态，复用现有有界消息泵，不复制业务执行逻辑。

v266：native_menu_panel_uia 统一 provider 懒建、状态通知和选择通知的生命周期
桥；WM_GETOBJECT 与控制器共享该 owner，不重复持借用跨系统回调。Invoked
客户端回归覆盖三种选择路径及关闭后的缓存/订阅释放，Panel 回归覆盖重入取消。

v265：native_menu_uia_events 只拥有快照差异和系统事件发送；Panel 保存有界
通知基线，Cascade 在全部页面状态同步后触发。uia_event_client_tests 用独立
MTA 订阅焦点/结构/属性，缓存读取和非阻塞有界通道处理回调，RAII 注销订阅。

v264：accessibility::Owner 的 active 标记只影响对外焦点属性，不改变绘制
高亮或节点代次；links::View::focused 最多遍历 16 层。uia_focus_tests 验证
FragmentRoot 汇总和身份保留，树客户端回归验证子项/父项 SetFocus 与收起。

v263：native_menu_accessibility_links 拥有跨面板弱关联和撤销校验；Cascade
同步当前页面拓扑，Panel 仅桥接 owner。uia_fragment 复用这些关系完成弹出
窗口逻辑重设父级；uia_tree_client_tests 不读取子 HWND，沿系统 UIA 树验收。

v262：native_menu_uia_scroll 拥有根菜单 Scroll 模式的轴/数值合同；
accessibility::View 只排队绑定代次的绝对行偏移，Cascade 复用 set_first，
不复制鼠标滚动或菜单业务。滚动元数据由 refresh 同步，节点不接受根滚动动作。

v261：native_menu_uia_expand 拥有 ExpandCollapse 窄 ABI；实际 expanded 状态
由 Cascade::sync_accessibility 同步，不能由 COM 入口乐观设置。快照刷新保留
同一模型条目的展开状态，内容代次失效规则不变。客户端测试复用有界 STA 泵，
通过真实 MTA UIA 请求驱动子页建立/销毁并核验状态。

v260：native_menu_uia 拥有菜单 Simple ABI、规范 IUnknown 与引用计数；
uia_fragment 拥有导航、运行时身份和几何；uia_invoke 仅排队叶子命令。
Panel 的 WM_GETOBJECT 注册与销毁回收不借用业务模型。uia_client_tests
挂于 Cascade 测试 owner，以独立 MTA 系统客户端验证发现至菜单队列消费。

v259：native_menu_accessibility 拥有有界只读快照、节点撤销和操作准入；不依赖
Surface 业务树，不持裸 HWND/Panel 引用，不保存命令 ID 或图片。Panel 生命周期
与刷新同步 owner；Cascade 将可见节点映射回模型并再校验，业务交付仍在菜单
退出之后。Row.key 是模型索引，不是滚动后可复用的可见槽位。

v258：native_menu_cascade_geometry 统一拥有 HWND 几何同步和旧手势失效，
panel 只提供有界队列的指针事件丢弃。geometry_tests 使用真实 SetWindowPos
验证子菜单新位置路由、旧点击拒绝、缓冲事件处理和外部改尺寸取消。

v257：history delivery 在显示快照上计算文字准入中心，接管后才在目标文档中
居中插入；popup_history_adoption_tests 拥有空底图/异尺寸/平移工具的传播回归。
display_edit 的 Text intent 不沿用平移手势的免接管路径，其他准入门禁保持不变。

v256：popup_history 拥有末尾入口及其稳定父行索引；popup_history_deliver 统一
文字测量后的居中几何。tray_history 只拥有恢复策略及恢复后目标核验，移除重复
坐标换算。application_history_smoke 在真实文档中断言逐行与整批中心位置。

v255：popup_menu::choose 统一画布/托盘的历史树、会话和选择映射，激活策略仍
分别由 track_live/track_tray_live 负责。MenuTarget 移到 popup_menu/target，
作为 desktop 内共享凭证；tray/history 只负责恢复策略和文字插入锚点，不复制
历史读取或内容交付实现。

v254：text_annotation/content 拥有正文及共享字节租约，文字模型、撤销历史和
导出快照共用同一 Arc owner。32 MiB 预算只覆盖保留的 UTF-8 正文，不冒充
字体/DirectWrite 布局或 JSON 输入缓冲的完整进程内存上限。

v253：application_smoke/history 子模块拥有无系统历史访问的交付冒烟；它与
菜单共用 popup_menu::deliver_history 准入入口，不复制图片/文字导入业务。
业务目标凭证由调用者提供，入口再次拒绝失效窗口和仍占用的模态门禁。

v252：image_thumbnail 拥有最大 64×64 的共享 PBGRA 小图，历史后台复用已解码
原图生成它；popup_history 仅传递共享引用。panel_thumbnail 使用固定栈缓冲
按当前行背景合成并绘制，不持有额外 GDI 句柄、不解码、不访问剪贴板。

v251：desktop/popup_history 持有展开期间的 Session、绘制 ID 与动态命令映射，
popup_history_deliver 在菜单关闭及 MenuTarget 通过后交付；图片复用 open_job
新钉图目标，文字先测量再调用 Editor 批量事务。业务入口不直接调用 WinRT。

v250：native_menu_model 验证手写/动态菜单树的整树预算；cascade_update 只拥有
根条目子树替换和旧子窗口/手势失效，不读取剪贴板。modal::track_live 在现有
事件/定时器泵中调用非阻塞更新回调，历史会话仍由调用者持有，不能在回调中执行业务。

v249：托盘入口也读取同一 HMENU 快照并进入自绘宿主。modal::track_tray 仅在
显式托盘手势中请求激活菜单根面板；run_prepared 共用捕获、输入泵及退出清理。
不创建额外常驻窗口，不显示隐藏业务 owner，不再保留 TrackPopupMenu 回退。

v248：desktop/popup_menu 在应用模态门禁内读取 HMENU 快照并调用自绘宿主，
context_menu 保留 MenuTarget 和原命令分发。托盘通过 track_tray 独立保留系统
入口，隐藏 owner 的前台语义尚未迁移，不能静默套用画布准入条件。

v247：native_menu_scrollbar 拥有整行滚动的共享像素几何，cascade_scroll 拥有
轨道翻页、拖动偏移和点击隔离；panel_draw 只借用几何绘制。modal 在拖动期间
把越出窗口的移动/松开继续交给原页面，不转移根捕获。它不替代历史列表的像素滚动模型。

v246：native_menu_mnemonic 拥有标签 & 解析、UTF-16 字符组装和助记符选择，
模态泵只把字符消息交给它；快捷键列不参与匹配。焦点显露统一归 Cascade.focus，
键盘和助记符复用同一窗口切片，不复制滚动/命令执行逻辑。

v245：native_menu_panel_font 拥有普通/粗体 DPI 字体和 GDI 标签宽度测量，
panel_columns 拥有标记/标签/快捷键/箭头的互不覆盖分栏。panel_draw 借用字体，
DC 状态用 RAII 恢复；Cascade 按完整页标签测宽，仍受工作区宽度上限约束。

v244：native_menu_modal 拥有级联菜单的嵌套 GetMessage、根捕获、生命周期定时器
和输入路由；不拥有业务命令执行。modal_tests 通过真实队列验证键鼠消息、
WM_QUIT 转交、捕获保护及 owner 清理。面板销毁发无载荷 WM_NULL 唤醒消息泵。

v243：native_menu_cascade 拥有菜单树、打开面板路径、可见行窗口和命令返回；
cascade_input 拥有导航/点击/滚轮映射，不执行命令。面板只绘制可见切片，完整
菜单树仍保留；布局使用 Windows 工作区 popup 定位，销毁按 Panel 实例身份。

v242：native_menu_panel 拥有自有 owned popup HWND、实例身份与线程绑定；
panel_draw 只负责 GDI 绘制，panel_input 只记录有界输入，不调用业务命令。
面板最多接收 64 个可见行，不是菜单树总量上限；后续控制器负责切片、滚动、
焦点/捕获与级联。panel_tests 使用真实 HWND 验证绘制、消息和销毁。

v241：native_menu 拥有既有 HMENU 树到只读菜单快照的转换，不拥有或销毁调用者
句柄。原菜单 owner 继续构造业务命令及动态状态，自绘宿主只消费快照；解析
有深度/条目/文字预算，不支持的 owner-draw/bitmap 显式失败，不能静默降级。

v240：clipboard_history_list 拥有历史面板屏幕坐标、行裁剪、命中及滚动条几何；
不拥有 HWND 或输入捕获。Win32 宿主必须传入同一 DPI 空间的像素，处理系统
滚轮偏好和 capture 生命周期。list_tests 覆盖负坐标、左右翻转和异步重布局。

v239：clipboard_history_session 组合 Events 和 Task，拥有菜单会话的加载、刷新、
失败及可选择快照状态；合并读取期间变化、节流启动失败重试、丢弃旧快照，
选择时校验绘制条目 ID。session_tests 使用合成订阅和真实后台线程验证协议。

v238：clipboard_history_events 拥有历史、内容和启用状态三类 WinRT 事件订阅，
回调只设置合并标记；令牌 guard 负责部分失败回滚及销毁注销，STA 生命周期
不跨线程。独立 events_tests 验证合并、弱引用和注销失败继续清理的协议。

v237：clipboard_history_task 拥有后台线程启动、独占许可预留、单结果有界通道
和非阻塞取消/销毁。独立 task_tests 验证提供方阻塞、panic 与迟到结果；读取
实现仍归 clipboard_history，不复制 WinRT 或将菜单目标授权混入后台 owner。

v236：clipboard_history 拥有只读 WinRT 历史快照、STA 与进程级读取许可；
clipboard_history_content 拥有 Bitmap/Text 筛选和编码限额，wait 拥有消息泵
及取消确认。image_decode 仅增加来源专属的尺寸/像素预算参数，复用既有 WIC
解码，不复制图像实现。两组独立测试只用内存 DataPackage/延迟操作，不读取
用户剪贴板；本轮没有历史菜单、事件订阅或窗口 State 接线。

v235：editor_text_batch 拥有有界整批文字插入事务；先准备全部几何和标注 ID，
再一次修改元素、revision 与历史。editor_text 复用同一个 Stroke 构造函数，
单条插入行为不变；edit_history 仅增加复用既有 batch toggle 的 added_many
构造。独立 editor_text_batch_tests 覆盖一次撤销/重做、失败原子性与预算。
这是 N06 系统剪贴板历史的编辑侧前置，不是历史读取或菜单已接通。

v234：package-native-candidate.ps1 拥有内部 ZIP 编排，native-candidate/stage.cjs
负责候选/当前源码匹配及分发目录，Archive.ps1 负责安全路径、逐文件摘要、
构建身份和许可证清单交叉验证；verify-native-package.ps1 提供只读验证入口。
复用现有 release 路径/流式摘要门，不更改公共发布链。归档反例合同归
scripts/tests/Test-NativeCandidateArchive.ps1，不新增运行时或依赖 owner。

v233：scripts/native-candidate/licenses.cjs 拥有 normal/build 依赖图遍历和
实际许可证文本收集；不借用 npm/Tauri 依赖清单，不把声明字段当成文本。
来源限制、目录与容量边界及内容摘要归同一 owner；图和真实材料回归归
scripts/tests/native-license-materials.test.cjs。法律、安全与公开发布批准不归收集器。

v232：application_smoke_image 独立拥有启动探针的内嵌 PNG、独占临时文件
与作用域清理；open/paste/drop/group_lock 四个 smoke 复用该 owner，继续
走真实文件/工作线程链，不再通过 CARGO_MANIFEST_DIR 读取构建仓库。
模块接线归既有 application_smoke；application 主 owner 不增长，
不将测试素材或临时文件 owner 混入正式打开逻辑。

v231：scripts/build-native-candidate.ps1 拥有独立内部 Release 构建编排；
native-candidate/source.cjs 拥有源码快照，Tools.ps1 拥有工具与配置摘要，
provenance.cjs 拥有候选字节绑定与验证。复用 release 的路径/补丁来源校验，
不复制公共发布流程；合同测试归 scripts/tests/native-candidate.test.cjs。
PowerShell 启动失败、正常 stderr 和缓存锁生命周期合同归
scripts/tests/Test-NativeCandidateTools.ps1。
不新增运行时 owner，也不改变公共发布、ZIP、签名或人工验收状态。

v230：native/build_support/windows_resources 独立拥有 PE 图标和版本资源的
构建期生成/编译；build.rs 只接线到产品 bin，不影响业务模块或窗口图标
owner。tests/build_resources 验证生成边界，既有 exe_resources 读取真实
EXE；工具要求与非发布边界归 WINDOWS_NATIVE_BUILD.md。

v229：surface_svg_filter_reference_tests 独立拥有显式/隐式滤镜输入、内部
名称不可见、前向/未知回退、重复名称与 merge/in2 的回归。usvg 的
Primitive 私有标记与 parser 引用谓词拥有修复，不在应用层复制解析器，
不增加 worker 或资源 owner；worker 只增加 cfg(test) 模块接线。

v228：surface_svg_work_resource_tests 独立拥有显式串行 Release 资源探针，
复用生产 Job 创建、IPC host 与 worker rasterize，不另建 renderer 或进程
owner。测试 peer 和父探针均为 cfg(test)/ignored，不能通过产品 CLI 调用。
surface_webp_process 只新增 cfg(test) Job 峰值读数接口，仍单独拥有进程/Job
生命周期。新探针 142 有效行，process 265 行（单一进程 owner），worker
139 行；不新增生产准入或改变现有 128 MiB/15 秒限制。

v227：resvg filter/work_budget 独立拥有累计额度、溢出拒绝和 primitive 工作量
权重，filter_work_tests 独立拥有执行实例计数、嵌套共享、边界与参数回归，
分别为 118/158 有效行。checked_render 拥有每次请求的预算生命周期；worker
仅设置额度和转换错误，不复制计费算法。上游 filter/mod.rs 从 892 到 905
有效行，增加的是执行前计费与 feImage 上下文传递接线；算法留在独立模块。
本次不兼拆旧上游调度器，以免扩大语义风险；由库、嵌套回归和 native worker
合同保护。宿主 worker 为 139 行，无新增进程、字体或窗口 owner。

v225：resvg checked_render 拥有每次渲染的滤镜错误通道，独立
checked_render_tests 拥有正常像素及嵌套传播回归。turbulence 保有实际
坐标/整数运算检查，worker 仅把错误映射为既有 InvalidData；不复制算法。
上游 filter/mod.rs 必要接线净增 21 有效行至 892，受五项数值反例、嵌套
传播与库回归保护；本次不兼做旧调度器拆分。两个新增模块为 31/100 行。

v222：surface_svg_filter_budget 独立拥有 XML 到 usvg 之间的滤镜解析预算，
流式计数、不复制属性列表；测试归独立 filter_budget_tests。既有
surface_svg_reference_policy 扩展滤镜模板 href 与 filter 属性边，继续
复用非递归引用图，不新增资源、字体或 child owner。功能白名单不变。

v221：third_party/resvg-0.48.1 拥有渲染器补丁，九个新增模块分别负责颜色传递、
分量融合、有限 Gaussian、联合缩放卷积、矩阵预乘、形态学测试、结果保留期、
缩放帧和小核二维卷积。来源清单/重放补丁仍归既有 release 校验器；新增源码
独立测量，不继承上游行数豁免。surface_svg_tests 增补依赖接入后滤镜继续拒绝
的真实 child 合同；不新增 worker、字体、父侧 XML 或 Application owner。

v220：surface_svg_paint_order_tests 拥有绘制顺序的显式分层像素等价、简写补全、
非法值拒绝和后续请求合同。policy 仅校验 token，不复制 renderer 的顺序实现；
实际 EXE 的入口验证沿用 surface_webp_smoke，不新增进程、字体或图片 owner。

v219 单位回归：surface_svg_length_units_tests 独立拥有绝对单位、继承字号 em 和
根/嵌套 viewBox 百分比的用户单位像素等价合同；复用原 decode，不增加单位换算实现。

v219 准入：surface_svg_text_length_tests、length_combined_tests、length_cluster_tests、
direction_tests、bidi_tests、length_policy_tests 分别拥有基础长度、嵌套约束、排版簇、
方向、混合文字及拒绝/资源合同。均复用 surface_svg_tests 的真实 worker decode，
不增加字体/布局实现；policy 只负责元素/数值/枚举准入，布局仍由受校验依赖拥有。

v219：surface_svg_depth_tests 拥有深层文字的真实 child 像素/后续请求和非法 XML
回归。固定 usvg 的五个新增模块分别拥有长度求解、片段归属、长度导出、路径 ID
导出与叶 span 构造；roxmltree 固定副本拥有迭代 XML tokenizer，不移植到业务层。
来源门 schema 2 记录新增文件；新增手写模块单独测量，不新增 worker/字体/UI owner。

v218：surface_svg_text_overlap_tests 拥有重叠 glyph 的浏览器等价关系、真实
child 像素及 4096 glyph 预算回归；usvg flatten 来源补丁保留逐 glyph paint
顺序，不新增渲染器、字体、进程或 Application owner。textLength 未开放。

v217：surface_svg_text_path_tests 增补祖先/tspan 负 dx 的三种锚点、跨段逆序
采样和 textPath 自身 dx 忽略回归。usvg 来源补丁负责局部跨度锚定与原索引
回写；不新增排版、字体、进程或 Application owner。

v216：surface_svg_text_path_tests 拥有路径文字的浏览器等价关系及
真实 child 拒绝测试；policy/reference_policy 负责 textPath/startOffset 与引用
准入，既有字体 resolver 和 worker 不新增 owner。第三方 usvg 的最小直线弧长
修复由现有来源清单/哈希门维护，不复制排版引擎到业务模块。

v215：surface_svg_marker_tests 拥有 context paint 与窄画布描边回归。
third_party 下两个精确 crate 副本承载受控 renderer 补丁；sources.json 与逐文件
manifest 记录来源。scripts/release/third-party-sources.cjs 拥有完整性校验和源码
树身份，SBOM/许可证门复用；不增加业务、图片或进程生命周期 owner。

v214：surface_svg_marker_tests 拥有 marker 的浏览器像素/子进程拒绝测试；
surface_svg_reference_policy 继续统一拥有引用图和递归准入，新增 marker 边。
实际安装与 EXE 冒烟沿用既有入口，不新增业务或进程 owner。

v213：surface_svg_normalize 继续拥有 renderer 语义适配，增加 pattern 描述
子内容的模板选择修正；surface_svg_pattern_tests 拥有百分比、模板与 overflow
浏览器基线。未增加模块、图片或进程 owner。

v212：surface_svg_pattern_tests 拥有静态图案浏览器像素与隔离 child 回归；
surface_svg_reference_policy 统一拥有图案、paint 与模板依赖的非递归准入。
生产渲染仍由既有 worker/renderer 执行，不增加图像、进程或授权 owner。

图片迁移合同与真实基线归 docs/development/SURFACE_IMAGES.md。
v211（开发中）：surface_svg_mask_policy 更名为 surface_svg_reference_policy，
对含 mask/symbol/use 文档统一拥有引用图准入；symbol_tests 拥有独立浏览器
像素基线。normalize 拥有 symbol 到 defs/svg 的 XML range 适配及有效 use 零尺寸
处理；不复制第三方 renderer，不引入新的图片或进程生命周期 owner。
v210：surface_svg_normalize 独立拥有已准入 XML 的 renderer 语义适配，从 viewport
提取根/嵌套规范化并处理 mask 定义属性差异；viewport 继续拥有尺寸和预算。
surface_svg_mask_policy 拥有非递归结构引用图/环与深度门；mask_tests 属于 SVG
解码测试 owner，实际 Application 与 EXE smoke 沿用既有入口，不新增生命周期 owner。
v209：surface_svg_viewport 除根尺寸规范化外，修正 renderer 的零尺寸嵌套 viewport
绘制差异；保留子树/ID，不增加解析器、图片 owner 或父侧 XML 逻辑。嵌套像素
基线继续归 surface_svg_tests，授权安装沿用原 inline Application 测试。
v207（开发中）：surface_svg_text_fonts 拥有 usvg resolver、设计空间 metrics 准入
及布局后缺字检查；仍复用 Fonts、原 Job child 和 Application 图片安装路径。
不新增常驻字体缓存、系统目录扫描器或进程 owner；最终验证状态见 SURFACE_SVG。
v206：surface_svg_font_fallback 拥有系统映射范围/scale/缺字结果；私有
surface_svg_text_source 拥有 COM 分析源及有界 UTF-16/locale 生命周期。Fonts 的
select/map_text 共用查询预算和 load_font 文件准入；fallback_tests 复用原私有
font child，不新增 launcher 或生产 CLI。仍未接 SVG 排版/renderer。
v205：surface_svg_fonts 是 worker 的按需字体来源模块，持有 DWrite collection、
查询/文件/累计字节预算；私有 surface_svg_font_file 拥有可信目录及同句柄读取。
两个独立 tests 模块保护文件准入及共享 launcher 的真实 Job；尚未接 text renderer，
不在 Application 添加字体逻辑，不新增 launcher/字体系统扫描器。
v204：SVG v2 wire 拥有逻辑尺寸扩展及双侧验证；client 在分配 Pixels 前接收验证。
from_surface_sized/Decoded/Ready 转移尺寸与像素，窗口以同一 bitmap owner 安装；
通用 Image 不增加格式元数据。surface_svg_wire_tests 属于共用 client 的 cfg(test)
子模块，实际故障 peer 验协议拒绝/回收，不新增生产启动参数或容量池。
v203：Surface Bitmap 拥有独立逻辑尺寸及栅格 crop，通用 Image 保持纯像素语义。
NativeMeasure 内部图片尺寸改为浮点，原整数入口只转换后共用同一实现；Presentation
传递逻辑尺寸。新测量/真实 D2D/HWND 测试仍归各自原 owner。child/Ready 元数据尚待
接通，不在父进程重解析 XML，不增长 Application 根模块。
v202：surface_svg_worker 拥有私有 CLI/renderer，私有 policy 叶模块拥有严格 XML 与
静态子集准入，viewport 叶模块拥有自然尺寸及根尺寸规范化；都只在 Job child 运行。
surface_image_worker_client 归 image_decode::surface，统一父侧帧/预算/退出/EOF 门；
WebP wrapper 仍拥有 RIFF 预检匹配，不复制 launcher。surface_svg_tests 归现有解码
测试 owner；真实 inline 安装复用 Application 夹具，根模块未增长。入口 smoke 同时验两 codec。
v201：surface_image_worker_host 拥有受限 child 的 Job 自检/读包/调用 renderer/回包；
surface_image_worker_wire 拥有共同数值边界，不解析编码图片或分配像素。
surface_image_worker_process 作为根模块引用历史 surface_webp_process.rs，只有一份
CAPACITY 和进程生命周期实现。surface_webp_worker 只保留入口、格式解码/ICC/PBGRA；
surface_webp_decode 仍拥有 WebP 预检匹配、共享 Pixels 租约和父侧最终接收门。
v200：surface_webp_lut 是 color 的私有叶模块，拥有限定 mft2 的表校验/直接求值，
不拥有 ICC parser、child/Job 或来源授权；surface_webp_lut_tests 是 color_tests 的
子模块，固定六个浏览器编码/像素对照。无第三方源码复制，不增长 Application 根模块。
v199：surface_webp_color 是 worker 私有子模块，拥有 ICC 借用、parser 错误分类和
有界分批转换，只在受 Job 限制的 child 生产入口运行；不修改公共 Image/其他格式。
surface_webp_color_tests 归解码测试 owner，固化浏览器 P3 字节及 fallback 对照；
叶模块单元测试保护 alpha/批次/资源错误，Application 复用现有 inline 安装夹具。
v198：surface_webp_exif_tests 为 WebP 解码测试子模块，拥有浏览器生成的固定基底及
32 个方向/字节序/前缀案例；实际 child 仍走原 worker，不新增通用 TIFF parser 或旋转 owner。
v197：surface_webp_wire_tests 属于父侧 webp 解码模块的 cfg(test) 子模块，仅替换
child 启动接缝，不复制响应解析/预算/回收；生产 run 仍固定绑定当前 EXE 私有入口。
process::start 只向直接父模块开放，测试 child 不进入正式构建。现有 process_tests
继续拥有共享容量的取消/截止时间检查；不把故障注入移入 Application 或 worker 生产入口。
WebP 解码器审查、资源隔离实现及剩余验收归 SURFACE_WEBP_ISOLATION.md。
v196：surface_webp_header 拥有纯 RIFF/尺寸准入；surface_image_decode 的 webp 子模块
拥有父侧 IPC/Pixels，surface_webp_process 拥有 child/Job/句柄/共享名额生命周期；
surface_webp_worker 拥有私有入口、二次校验和 RGB(A)→PBGRA。live_recovery_pipe 仅
复用低层 overlapped 管道并添加显式可选取消，不复用 recovery 业务协议或改变旧等待。
webp/browser/process 测试分别保护格式、独立像素基线、Job 限额和回收；真实应用接线
测试仍归已有 inline/resources owner。surface_webp_smoke 验实际产品 EXE 分流，
不复制第三方实现、不增长 Application 根模块。
v195：surface_image_format 是 Surface 专属的纯字节/声明识别叶模块，拥有有限静态
签名、MIME essence 和 SVG 分流，不获取字节或持有授权。surface_image_data 复用它，
仍拥有 data URL 来源准入、规范化及载荷解析；surface_image_decode 使用实际格式并
继续拥有 WIC 容器/帧数/动画检查。没有把普通桌面策略复制到 Tile/Wall 或通用 Image。
surface_image_format_tests 属于解码测试子模块；完整 TLS/内联安装回归留在原应用
测试 owner，复用窗口/资源 fixture，不增加 Application 根模块职责。

v194：surface_image_decode 的子模块 surface_image_orientation 拥有 JPEG/TIFF EXIF
方向查询、PROPVARIANT 生命周期和 WIC 变换源链；不拥有普通打开/Relay 元数据策略。
image_decode::decode_source 只负责最终源尺寸、格式转换和受预算像素分配，普通 decode
仍传入原始帧。surface_image_orientation_tests 拥有内存 WIC 编码夹具与八方向像素
位置表、取消及兼容性回归；cfg(test) JPEG 夹具复用于现有 Application 内联测试。
没有新增依赖、网络来源或跨线程 COM，也没有增长已满 500 行的 Application 根模块。

v193：Scene 拥有本地 image_placeholder 投影及纯数据 inline_image_source 选择，
资源 Request 复用该选择，不让场景模块反向依赖网络/runtime。NativeMeasure 子模块
surface_image_placeholder 拥有替代内容测量和有界 alt 布局，surface_image_paint 子模块
surface_image_placeholder_paint 仅绘制本地无像素标记。TextFrame/PaintFrame 复用既有
有序文字、裁剪及透明层，替代内容不是成功 Bitmap，也不创建事件/UIA 子控件。
surface_image_placeholder_tests 负责真实 D2D 像素和成功替换；场景/测量、系统 UIA、
完整 Application 与同场景选区验证分别留在已有 owner 的测试模块。
surface_selection_cross_node 继续拥有端点/文本有效性，忽略不可选择绘制项的增减，
仍拒绝可选择成员、顺序、内容或端点权限改变；不在图片模块复制选择逻辑。

v192：surface_resource_runtime 子模块 surface_image_request 拥有类型化 Resource/Inline
请求及 Resource/Node 键；其 surface_image_inline 子模块拥有来源优先级、原始快照
身份，surface_image_data 只拥有有界 data URL 字节解析，不获取 URL/文件。
surface_resource_decode 复用 worker COM/WIC，Sessions、应用队列和授权窗口仍拥有
原任务/安装生命周期。surface_image_inline_tests、surface_image_data_tests 与
application_surface_inline_tests 分别保护授权/取消/容量、解析、完整 poll 接线。
没有新增依赖，也没有向已满 500 行的 Application 根模块添加职责。

v191：Scene::visible_children 拥有 Image/Icon 呈现叶节点规则；原始 children 仍归
协议预算与合同指纹。surface_accessibility 只读映射图片 alt，surface_uia_fragment_navigation
负责装饰 Image 的 control/content 属性；不把 Name/类型属性变成 Invoke 或输入授权。
surface_uia_image_client_tests 复用现有 MTA 客户端/STA 泵夹具，拥有真实 HWND 图片
语义、过滤、替换撤销回归，不复制 UIA provider 实现。
v190：surface_window_image_requests 拥有当前 Binding/revision 的有界资源 ID 队列；
surface_windows_images 拥有窗口轮转、Ready 精确路由及终态错误投递。应用接线归
application_remote_live 子模块 application_surface_resources，不增加已达 500 行的
Application 根文件。它只准备 Config/Lease 并调用既有 Sessions，不复制 HTTP/runtime。
application_surface_resources_tests/fixture 负责完整应用 poll 的真实 TLS 检查，复用
cfg(test) 的 device_session_client TLS/临时身份夹具；surface_window_image_request_tests
负责当前版本、队列消费与轮转。没有新增生产凭据注入后门。
v189：surface_window_resources 的子模块 surface_window_image_install 拥有同 Grant
Ready 转交后的安装表、resourceId 呈现投影和到期撤图；不发网络请求、不持有任务槽。
Ready::transfer 拥有原授权到窗口授权的身份检查，目标窗口 Lease 接管撤权责任。
surface_window_scene/Host/Presentation 复用同一提交路径提供同场景图片刷新，保留
文字未变的选区；新场景 event/batch 仍清空图片，不复用上个 revision 的像素。
v188：Scene 拥有不修改协议属性的图片可用性投影；Node 的 serde(skip) 状态只影响
统一 visible 门，不授予来源读权。AuthorizedWindow 与 event/batch delivery 在呈现
边界选择 pending 投影，ResourceSnapshot 保留原始来源；低层测量没有隐式缺图降级。
v187：surface_window_scene 拥有窗口替换事务接缝（身份撤销/宿主提交/重入/UIA/失效），
由窗口根模块提取，不复制另一份实现。SurfaceWindow/Host 显式传递 Images，来源
授权仍归应用 owner；surface_window_image_tests 只验真实 HWND/像素/target 生命周期。
v185：surface_resource_snapshot 为纯身份/有界快照模型，保持跨平台，不依赖 Win32；
Windows mount 适配单独 cfg。mount_admission 与 convergence 仅从同源 raw 构造它。
AuthorizedWindow 拥有当前资源快照；surface_window_resources 子模块负责当前授权
资源请求/Ready 消费门，不负责网络、调度、D2D 或图片安装。event/batch delivery
拥有成功呈现后提交点；关闭/撤权归原窗口 owner。模型及真实 HWND 测试分别归
surface_resource_snapshot_tests 和 surface_window_resource_tests。
v184：surface_image_measure 为 NativeMeasure 子模块，仅拥有自然尺寸和内容比例反馈；
不接触网络、Pixels 或 D2D。图片双轴收敛与文字反馈共同受 reflow 的八轮门约束。
Presentation 拥有本场景 Images 表，replace 不隐式继承；各滚动/显露路径向同一
scroll_frame 投影传递该表。surface_image_measure_tests 拥有浏览器数值基线及
替换/重排/滚动 owner 回归；图片像素测试改用真实生产测量，不增加并行测量实现。
v183：surface_image_paint 拥有 UI 线程 Bitmap、精确 target 缓存、上传预算与 contain
绘制；surface_image_paint_tests 使用真实 WIC/D2D 测试后端，不替代生产场景测量。
PaintFrame 拥有图片命令次序与帧退场释放，Presentation 只转发设备资源释放，Host
在 target 重建/绘制失败前清缓存。来源授权和网络仍归资源 runtime，不进入绘制层。
v182：image_decode 拥有共享 WIC Image/预算分配，desktop/image 只重导出同一类型；
surface_image_decode 子模块拥有 Surface 静态容器准入、局部预算、APNG 检查及 RGBA
转换。surface_resource_decode 归资源 runtime，拥有 worker COM/解码前后授权校验；
Ready 只保留像素与资源描述。集合拥有资源到期清理，应用现有 wait 合并最早 deadline。
v181：surface_resource_runtime 拥有图片 worker/原租约/完成句柄和 Ready；
composition_relay_surface_resources 拥有应用集合准入、容量、取消和有界投递。
两者共用既有 Live Slot，不另设线程池。Ready 保留槽至消费/丢弃，避免结果绕过容量。
device_authorization_lease 仅追加原租约消费门，窗口集合拥有当前场景有效性判断，
应用结算与退出点负责过期任务取消。runtime/TLS 与集合测试分别在对应子模块。
v180：surface_resource_http 是 live_relay_http 的子模块，拥有从 mount 取得的私有
ResourceLease、验证后字节与消费身份门；不拥有线程、缓存、解码器或 UI。
device_session_client 仅接受该类型扩充固定租约头/二进制预算，普通请求预算不变。
surface_resource_http_tests 归现有真实 TLS 夹具。测量与 PaintFrame 不得自行访问
网络或文件；解码和像素预算复用现有 owner，不另起不受限缓存。

v179：selection_cross_node 的 ProjectedSelection 只拥有本次 draw 的四个范围
标量；PaintFrame 解析一次并将原有 Text index 传给高亮。旧逐项扫描仅 cfg(test)
用于独立 oracle；range_bench_tests 归 selection tests，显式执行不进入默认性能门。

v178：surface_text_vertical 拥有实际行移动结果，保留原公开二元返回接口；
selection_keyboard 复用相邻 owner 搜索并转换列坐标，成功后提交唯一 Selection。
没有新增全局焦点、缓存、线程或跨 Surface owner。

v177：surface_selection_keyboard 为 selection 子模块，单独拥有水平端点的相邻
可选择 owner 搜索；不持有额外状态、节点缓存或全局焦点。实际索引修改继续归
唯一 Selection，HWND text_key 仅路由并复用上下键 readiness。

v176：Layout 的 word_segment 拥有未扩展空白的 ICU 词段，word_range 只叠加
初始双击空白规则；WordDrag 拥有初始坐标与是否移动的手势阶段，无新增 owner。

v175：surface_selection_word 继续拥有初始词范围与当前 focus 的单份有界边界表，
跨 owner 时重查，不新增共享缓存或状态 owner。跨节点模型和 HWND 词手势测试
分别追加在既有 cross_node_tests 与 surface_window_text_word_tests 中。

v174：surface_selection_cross_node 归 Presentation selection，拥有场景顺序范围
投影和跨节点重排失效校验；Selection 仍唯一，新增 focus_node 归属。PaintFrame
仅将现有 TextFrame 传给逐项高亮；reveal/autoscroll 使用 focus owner。专项 tests
归既有 selection tests，像素夹具复用 WIC/D2D，不增加生产窗口或覆盖绘制通道。

v173：surface_text_double_click_probe_tests 归既有 system_mouse 测试子模块，
复用有界观察器与注入收尾，仅拥有系统双击序列和消息间 first-click 消费断言。
探针 launcher 新增固定白名单入口，无生产 owner/线程/协议变更。

v172：surface_text_word/word_icu 归 DirectWrite selection，分别拥有词范围和系统
DLL/iterator 的有界生命周期；surface_selection_word 归 Presentation 唯一选区，
持有仅手势期有效的边界表和初始词。Host 只做坐标/viewport 桥接；HWND 沿原捕获
通道接双击，pointer/scrollbar 保留非文字按下语义。独立 word tests 覆盖两层。

v171：`surface_text_autoscroll_timer_tests` 归既有 HWND autoscroll tests 子模块，
复用自有窗口/选区观察，仅拥有真实 timer 消息等待与停止检查；不新增生产 owner。

v170：`surface_selection_autoscroll` 归 Presentation selection，拥有边缘候选与
滚动投影提交；`surface_window_text_autoscroll` 归 HWND selection，拥有按需 timer
与手势身份复核，独立同名 tests 覆盖真实消息/生命周期。Host 复用 viewport/EDIT
同步；Pointer 只持有当前手势身份与 timer owner，clear 必须释放两者。
滚动条 repeat 与文字拖选共用单调 timer 身份分配，窗口根路由和行数不增长。

v169：`surface_text_line_edge` 归已有 DirectWrite selection 子模块，独立拥有
行度量、逻辑首尾、换行排除与有界临时数组，测试同名独立模块；Presentation
仍只改唯一选区 focus/视觉行点，Host 沿原显露同步路径，键盘仅新增 ShiftHome/End。
不新增常驻缓存、窗口、线程或跨节点选择模型。

v168：`surface_text_vertical` 归 DirectWrite selection 子模块，拥有上下行命中、
视觉 caret/cluster 几何；TextItem 只做坐标转换，Presentation 唯一选区持有可选
局部列/行探针。原键盘子模块接 Shift+Up/Down，`surface_window_text_vertical_tests`
归其测试子模块；纯滚动保留点、成功 reflow 重置点，无新增窗口或后台资源。

v167：`surface_selection_reveal` 归现有 selection 子模块，拥有端点 region 与
滚动投影提交准入；`surface_selection_reveal_tests` 验证嵌套、静态裁剪与缓存。
`surface_scroll::reveal_region` 与原 focus reveal 共用 nearest，不复制滚动算法；
Host 仅在显露提交后调用既有 `sync_edits`。没有新增常驻缓存、线程或窗口 owner。

v166：文字 cluster 移动仍归 `surface_text_selection`，选区更新归
`surface_presentation_selection`，Host 只做当前 viewport 同步桥接。
`surface_window_text_key` 作为既有选择模块子模块独立拥有键盘准入/路由，
`surface_window_text_key_tests` 保护焦点、重复、撤销及实际 HWND 接线；
键盘根仅在 Copy 和 Scroll 之间接入，不新增窗口、线程或第二份选区模型。

v165：`surface_window_cursor` 复用 `surface_host_selection`/Presentation 的
选择起点查询，不另建命中模型；`surface_window_text_cursor_tests` 验证光标
准入。`surface_text_mouse_probe_tests` 归属于已有文字选择测试，独立拥有有界
系统输入观察器；现有显式可见探针 launcher 以固定白名单模式选择 touch/text-selection，
不引入新的生产线程、窗口或持久状态，窗口根模块保持 499 有效行。

v164：`surface_presentation_selection` 拥有选区与范围高亮，`surface_paint_frame`
只把它接入原 Text 指令；`surface_host_selection` 负责现有 viewport 下的桥接。
`surface_window_text_selection`/`surface_window_text_copy` 分别拥有 HWND 拖选
和本地复制准入，不存第二份文字模型；`clipboard_text` 复用既有暂存预算/转移。
各自测试独立拆分，窗口根模块仅增加一行归还 Host 前的选择取消收尾。

v163：`surface_text_selection.rs` 作为 surface_text 子模块拥有 DirectWrite
命中、范围验证与几何查询；`surface_text_selection_tests.rs` 使用真实布局验证。
`surface_text_frame` 只追加 selectable 准入和既有坐标/裁剪转换；全链路需求
记录于 `SURFACE_TEXT_SELECTION.md`，此阶段没有新增 HWND/剪贴板/后台任务 owner。

v162：`surface_slider_gesture_lifecycle_probe_tests` 归属于系统手势测试模块，
负责实际活动 pan 后撤权/换场景、后续惯性与结束；正式 Surface 夹具拥有状态
断言和有界数值 prepare/settle。仍挂在既有 scroll_tests 下，不扩张窗口 owner。
只读消息观察器保存有界前置诊断，失败保留，不用重试直到通过掩盖原因。

v161：Slider messages::install 拥有创建期轴向手势配置；surface_window_gesture
拥有目标准入和 target+sequence 的 Pan 生命周期，继续委托 Host 的滚动模型。
`surface_slider_gesture_probe_tests` 负责实际轴向消息及 Surface 注入；
`surface_slider_scroll_probe_fixture` 是 cfg(test) 的正式 Surface/挂载夹具，仅
借出自有 HWND 给输入闭包，核验滚动、数值队列与 click，不开放生产测试 API。

v160：`surface_slider_messages::is_touch_promotion` 只解读兼容鼠标消息的系统
来源标记；Control 的 drag_touch 与 drag 同生命周期，cancel 一起清理。
`scripts/tests/Invoke-NativeTouchProbe.ps1` 拥有显式交互启动、当前 artifact 定位、
自启子进程超时及证据保存；不向生产引入注入 API，不按进程名字清理窗口。

`surface_slider_touch_probe_tests` 独占显式 Windows 触摸输入探针；通过系统 API
注入并只读观察生产子类，拥有测试窗口、观察子类和触点取消的生命周期。
默认 ignored，环境变量再次准入；不得绕过自有 HWND 命中核验向外部窗口注入。
Pointer 绑定仅为 dev-dependency feature，不在生产路径初始化触摸注入器。

`docs/development/SLIDER_TOUCH_BASELINE.md` 记录真实浏览器模拟输入的观测与
原生接入退出条件，不是生产支持声明。未来触摸接缝须复用现有 Slider 数值/
队列和父滚动 owner，不能复制业务模型或让兼容鼠标消息重复进入队列。

v157：`surface_slider_messages::checkpoint_drag` 负责正常切焦的提交检查点，
不释放捕获；finish_drag 只负责抬起收尾，cancel 负责真正撤销。父 NAVIGATE
核验实际捕获与挂载类型后保留原生鼠标按压，不能按消息参数泛化绕过失效机制。

v156：`Activation::CapturedPointer` 只表示已准入原生 Slider 捕获手势的完成，
不得从普通点击或外部请求构造。`surface_window_slider_click` 在 begin 校验
控件内坐标，在 end 保留已记录节点；Host 合同/事件准备以该节点验证，不使用
释放位置重定向。其来源不是 Keyboard，因此不要求完成时仍有逻辑键盘焦点。

`surface_window_slider_click` 处理 Slider 私有 begin/cancel/end 边沿，父 Pointer
持有带场景身份的 native_press；不在子 HWND 保存业务凭证或独立 click 队列。
begin/end 都验证实际挂载 Slider、权限和命中，end 在自身 change 后捕获原
form snapshot/fence，复用原 Click 单槽。`surface_slider_messages` 正常抬起
保留 click 完成边沿，真正取消及 NCDESTROY 明确撤销父按压。
`surface_slider_bubble_tests` 验证正式准备/结算顺序及失效边界。

`surface_slider_messages::finish_drag` 只负责用户完成手势（鼠标抬起、Tab）
的提交边沿；取消路径继续使用 cancel。键盘成功提交后更新 drag 基线，避免
之后抬起重复 change。`surface_slider_mixed_tests` 保存 Chromium 对照的
边沿回归和回调销毁保护，Mount 测试验证队列与 EDIT 焦点导航，不绕过父路由。

`surface_window_range_readonly` 管理 Slider 权限差量的入队与当前值复核，
使用 `surface_window_notifications` 既有 Pending/消息/flushing 护栏，不另建
调度器。Mount 的 reported_readonly 只记已提取状态，编辑权限仍由原 owner
持有。`surface_uia_range_readonly_tests` 通过有界指令/确认通道让 UI owner
修改策略，MTA 只调用 UIA，不跨线程借用窗口；通用客户端泵增加可选 tick。

v152：测试专用 Observer 位于 `surface_uia_range_event_tests`，复用 Handler
和 RAII Subscription；`surface_uia_process_range` 持有它跨越退出/重启边界。
先验证收到旧实例数值序列，再检查新实例操作期间旧订阅安静，最后检查释放。
通道容量、等待期限和重复事件排空有界，不暴露生产 API 或创建常驻资源。

`surface_uia_process_range` 拥有 Slider 跨进程读写/退出/重启验收；
`surface_uia_process_fixture` 的显式 range 模式校验正式队列的数值边沿，
保持子进程、管道、超时和退出清理的原 owner。复用 `surface_uia_range_event_tests`
真实订阅逻辑，不复制生产实现或添加公共生产测试入口。

v150：`surface_edit_sliders` 的 reported 仅跟踪最后提取的已接纳数值，扫描上限
受既有 64 字段准入约束，不读取 HWND 或复制业务队列。`State::run` 恢复 Host
后才将差量放入 `surface_window_notifications`；Switch/Slider 共用一个 posted
消息和 flushing 护栏，分别保留有类型的数值快照。发送前复查身份及当前值，
Raise 调用期间不持有 Host、通知队列或 UIA root 的 RefCell 借用。
数值通知测试在 `surface_window_range_notification_tests`，真实 MTA 订阅与
释放检查在 `surface_uia_range_event_tests`，不引入额外线程或 timer 到生产代码。

`surface_uia_range_abi` 负责 Slider RangeValue COM ABI 与数值属性；
`surface_accessibility_range` 负责当前身份、UI owner 及交互准入桥接；
`surface_edit_sliders` 仍是唯一数值和队列 owner。新接口共享 Provider 的
IUnknown/引用计数/撤销机制，Slider HWND 走原 override，不增加全局索引。
`surface_uia_range_tests` 验证真实 COM/FIFO/撤销，`surface_uia_range_client_tests`
通过 MTA 系统客户端验证合并树与实时读写，不替代属性通知/读屏验收。

`surface_edit_sliders` 是 Slider 的 Mount owner，保存合同、权威 source、已接纳值、
已提交值及几何快照；复用全字段 64 个准入和唯一 32 槽 FIFO，不新增文本 draft。
每次 input 前检查可容纳 input/change 双边沿，拒绝时恢复已接纳 HWND 值；
prepare 只编码不可变数值快照，沿用原队列取消/重试/结算。先清旧合同意图，
再创建同 ID 新控件，包含 Slider/Select/Switch 跨类型替换。
`surface_edit_controls` 从 Mount 提取 HWND 身份/焦点路由，不维护另一个映射。
Slider 同几何同步只更新裁剪，不取消捕获；几何变化、父取消/失活及权限撤销
终止手势。父私有消息只进入已挂载 HWND 的校验，不凭消息参数授予编辑权限。

`surface_slider_control` 拥有单 UI 线程子 HWND、私有 marker 和 HWND 生命周期
内的 State；初始化失败即使尚无 State 也须销毁自有窗口。State 已释放时所有
公开读写必须拒绝。`surface_slider_messages` 负责键盘、局部指针和系统捕获，
不持有跨 Win32/父通知调用的 Rust 借用；input 回调可重入关闭/禁用/改值，
之后必须复核 owner、存活和值才允许 change。`surface_slider_paint` 只绘制
实际控件，不创建画刷、线程或 timer；SaveDC 恢复调用者状态。
`startup_entry` 是从 desktop 根模块原样迁出的公开启动参数适配层，保持
run/run_capture/run_session 导出与资源打开顺序；COM、窗口和消息循环仍由
原 owner 管理，startup 测试保护这次行数拆分，不增加 501-700 例外。

`surface_slider::Range` 独立拥有有限数值声明、range 钳制/步距归一和位置换算，
不复用 Number 的文本编辑态，不创建 draft 或表单值。无循环步进和整数刻度
截断；不能可靠表达的极端步距返回 Precision，不静默改变声明步长。
Slider wire 复用 surface_input_event，允许有限 number 的 input/continuous 与
change/commit，拒绝 null、字符串和非有限值。当前尚未挂载 HWND，不增加能力声明。
数值夹具在 surface_slider_tests，wire 在 surface_slider_event_tests；浏览器
range 的归一结果和控件操作事件频次是不同层次，不能互相代替验收。

`PaintFrame` 在既有颜色解析中为 Select 保存有效的显式文字色，Presentation
只读暴露给 Mount；不在 HWND 或滚动消息中再建 CSS 解析器。控件 class 默认色
不继承父色覆盖，显式 currentcolor 使用父级计算值，与主干 CSS 白名单一致。
`surface_select_color` 以 HWND 私有属性拥有小型 RGBA 快照，更新失败保留旧值，
相同值不分配；NCDESTROY 释放，不依赖 Rust marker 存活。普通/hover 背景分别
合成 alpha，列表选中行和 disabled/high-contrast 仍保留各自可读状态色。
`surface_select_declared_color_tests` 验证真实字段/列表像素、父计算值更新与
合同替换后的新 HWND；不可为了测试方便复用已撤销 HWND 或绕过合同重建。

`surface_select_items` 拥有固定行高与列表 owner-draw 接缝：父 HWND 一个无堆
状态 subclass，仅处理具有 Select owner 标记且属于当前父窗口的 COMBOBOX。
hook 生命周期到父 HWND 的 NCDESTROY，不随单个字段卸载重复安装资源。
SaveDC/RestoreDC 保持借用 DC 状态，系统 stock 画刷不得删除；高对比度在
owner-draw 内显式使用系统颜色。`surface_select_text` 统一有界读取系统字符串，
供收起区域和列表共享，不保存第二份选项/选择状态。CBS_HASSTRINGS 不可删除，
原生键盘和 UIA Selection/ExpandCollapse 仍由系统负责。

`surface_select_hover` 只解释自有 HWND 的局部 mouse/leave/disable/hide/cancel
边沿，状态存在已有 subclass ref-data 的一位，不加堆对象、队列或自有 timer。
`surface_select_messages` 负责状态写入和仅边沿重绘，销毁时取消系统 leave
跟踪；`surface_select_paint` 只读状态选择 canonical controlHover 背景。
负客户区坐标必须按有符号 16 位解析；测试不移动系统光标或注入全局输入。

`surface_select_paint` 只负责 COMBOBOX 收起区域的原生 GDI 绘制，不更改控件
类型或接管输入/选择/UIA。WM_PAINT 使用 BeginPaint/EndPaint，WM_PRINTCLIENT
绘制到调用者 DC，SaveDC/RestoreDC 保证早退也恢复字体/颜色/stock 选中对象。
文本通过当前系统索引读取，长度受 Options 总预算约束；不缓存第二份选择状态。
默认/焦点/禁用像素和调用者 DC 恢复在 surface_select_color_tests 中验证。

`surface_control_colors` 是 Surface 系统字段共用的 Neuro 文本/背景颜色接缝，
借用 DC_BRUSH，不拥有画刷；高对比度或探测失败交回系统。Select 子类只响应
GetComboBoxInfo 验证属于本 COMBOBOX 的 list HWND，不扩展通用字段 owns 范围。
`surface_select_color_tests` 绘制自有列表到有界内存 DIB，验证真实未选中行
背景与文字像素；可通过 OTTER_NATIVE_TEST_ARTIFACTS 显式导出此测试 PNG。

Select 字体继承复用 NativeMeasure.resolved_style，不在控件重新解析 em/%。
`surface_scroll_frame::Cache` 仅为 Select 保存 owned ControlFont（family/size/weight），
与场景重排一起替换，纯滚动复用；Presentation 只读暴露给 Mount。GDI 接缝按
字号/字重/字体名判定复用，字体名转换使用固定 32-unit 栈缓冲，拒绝 NUL、
控制字符、越界和非有限尺寸，失败不替换当前字体。

`surface_select_font` 独立拥有 Select 的 DPI GDI 字体，以 HWND 私有属性保存
唯一句柄；只在字号变化时创建，在 WM_SETFONT 返回后释放旧字体，在默认
WM_NCDESTROY 返回后释放当前字体。即使 Rust Control marker 被撤销，仍由
存活 HWND 保持字体，不发生提前释放。`surface_select_font_tests` 验证真实
句柄缩放/复用/销毁，Mount 测试验证 WM_DPICHANGED 到字体及零 change 事件。

`native/build.rs` 与 `native/windows.manifest` 负责 Windows MSVC 宿主及测试
一致的 Common Controls v6 激活，不新增运行时组件。Select UIA 验证不得用
旧 MSAA 导航诊断替代真实 UIA 树；v6 已排除先前 UIA 越界，下一门禁为最终
change 准入已通过。`surface_edit_selects` 同时核对真实 SELCHANGE/CLOSEUP 的索引，
复用 accepted 去重及原 FIFO，不构造系统通知。

`surface_uia_select_client_tests` 覆盖 Select 系统模式、唯一逻辑父级
及纯系统 HWND UIA 对照；v6 构建下展开后的导航、选择和收据断言已通过。
`surface_uia_process_fixture` 增加显式 Select 场景模式，复用有界 Child/管道
owner，并在 child 验证真实 change 收据；不访问外部窗口或伪造业务通知。
已撤回无效 popup/MSAA provider 试验，不再实施无必要的自建 COM 适配。

Select 的 Mount `Placement` 仅保存单控件几何/DPI，用于重定位前收起旧弹出层；
`Control::close_popup` 执行系统消息并复核 owner。回归在
`surface_select_scroll_tests`，覆盖无变化同步保留、滚动关闭和裁剪后的恢复。

`surface_select_messages` 是 COMBOBOX 的窄 Tab 子类：收起下拉、复核 marker、
转发现有 NAVIGATE，不拥有字段会话指针。焦点候选/实际控件路由仍由
`surface_focus` 与 Mount 管理；`surface_select_navigation_tests` 验证混合双向
导航、下拉收起、重复按键和 disabled/hidden 跳过。

`surface_edit_selects` 管理 Select 挂载、source 同步、真实选择通知和共享 FIFO
准入，独立于 EDIT 草稿与 Switch ticket；`surface_edit_prepare` 复用不可变事件
编码和既有顺序。真实 Surface 通知链在 `surface_select_mount_tests`，不直接调用
notify 代替系统 WM_COMMAND；跨类型旧意图撤销另有针对性回归。

`desktop/surface_select_control` 是单线程 COMBOBOX 生命周期 owner，持有不可变
Options 和私有 HWND marker，管理索引/显示/禁用/位置/销毁，不拥有 draft、
live event 或提交权限。真实系统控件回归位于 `surface_select_control_tests`；
后续通知接线必须由独立宿主会话负责，不能在窗口资源 owner 中发送业务请求。

`surface_select` 只拥有 Select 的有界声明快照和安全索引取值；不创建 HWND、
文本 draft 或按钮表单集合。Select 事件复用 `surface_input_event` 的不可变编码，
只允许字符串 change/commit；模型和 wire 回归分别在 `surface_select_tests` 与
`surface_select_event_tests`，控件宿主尚未接入，不以模型存在宣告控件可用。

`surface_uia_process_focus` 独立拥有跨进程焦点订阅和释放验证；系统回调先按
cache PID 过滤为自启 Child，再读取节点 ID，不查询其他应用实时属性。它不
设置桌面前台，仅对已取得的本实例 Switch/Button 调用 UIA SetFocus。

`surface_uia_process_structure` 拥有跨进程稳定根结构订阅、替换后树重读与 handler
释放验证；仅经 fixture 管道请求 UI owner 替换，不直接访问子进程 State。
fixture 的有界命令读取只识别 replace/close，替换只允许在动作收据成立后一次。

跨进程生命周期测试可保留旧 `Subscription` 穿过 Child 终止和新实例启动；新
实例持有独立订阅 owner，旧订阅安静断言区分 timeout 与 channel 断开，再检查
旧/新 handler 各自释放。进程终止仍仅由 fixture 的 Child owner 执行。

`surface_uia_process_events` 独立拥有跨进程 ToggleState/Invoked 订阅和 handler
释放检查，回调只读缓存并使用有界 channel；部分注册失败由 RAII 撤销既有订阅。
它不管理 Child、不触碰 UI State，测试动作仍由 `surface_uia_process_tests` 发起。

跨进程 fixture 的 `terminate` 仅终止所持 Child，正常/强制路径共享
`wait_for_exit` 并校验退出成功状态。父测试保持旧 COM 引用穿过新实例启动，
验证旧实例失效与新实例可操作性，不依赖 PID/HWND 不复用假设。

`surface_uia_process_tests` 负责独立进程的系统 UIA 树、Value/Toggle/Invoke 和退出
撤销验证；`surface_uia_process_fixture` 只负责自启 Child、限量管道收据、窗口
owner 与失败清理。父进程不读取子进程内部 State，不扫描应用或向缓存 HWND
发送关闭消息；正常关闭走所持 Child 的 stdin，由子进程 UI owner 关闭窗口。

Mount 的 `fields_changed` 与 `take_structure_change` 只表示实际原生字段集合
变化，经 Host 转发到现有结构通知 owner，不维护另一个 HWND 注册表。
`surface_window_edit_structure_tests` 覆盖同场景绑定/重建及无变化抑制；
`surface_uia_structure_event_tests` 同时覆盖整场景替换和晚绑定真实订阅。

`surface_accessibility_edit` 负责当前 Mount 的 HWND/节点双向映射及显露后聚焦，
不缓存句柄；`surface_uia_hwnd_override_abi` 负责根 override 接口头与 nullable
输出，沿用共享 IUnknown。Fragment navigation 只纳入已挂载 EDIT，系统 host
负责原生模式与属性。`surface_uia_hwnd_override_tests` 验证 ABI/撤销/焦点，
`surface_uia_edit_client_tests` 通过真实 MTA 验证唯一父级与 Value/Text。

`surface_window_structure_notifications` 拥有场景子树失效的单消息排队、busy 与
重入保护，在稳定 HWND 根发送 ChildrenInvalidated；触发点是 replace 撤销旧
Owner，不复制节点树。`surface_window_structure_notification_tests` 验证生命周期，
`surface_uia_structure_event_tests` 验证 MTA 订阅、UI owner 替换、客户端重读和释放。

`surface_window_focus_notifications` 负责焦点通知排队、runtime identity 去重、
busy/重入及失焦边界；只查询 `surface_accessibility_focus`，不拥有业务焦点。
键盘 owner 和窗口焦点消息是通知触发点。`surface_window_focus_notification_tests`
验证 owner 边界；`surface_uia_focus_event_tests` 负责真实 MTA FocusChanged 订阅、
缓存节点过滤、SetFocus/Tab/鼠标路径和 COM handler 清理，不保存回调 sender。

`surface_switch_session` 记录实际值变化，`surface_edit_mount`/`surface_edit_switches`
拥有 dirty 标记与变化提取；`surface_window_notifications` 负责身份绑定合并、私有
消息排队、重入防护与撤销清理。它不维护第二份业务值或动作队列。
`surface_window_notification_tests` 覆盖净零合并、Host busy 重排、重入旧值跳过、
场景替换、结算与关闭；真实系统接收仍在 `surface_uia_toggle_event_tests`，包括
UIA Toggle 和仅向测试自有 HWND 分派的鼠标/键盘消息。

`surface_uia_toggle_event_tests` 负责 ToggleState 的真实 MTA property 订阅、cache
身份、i32 VARIANT 状态及 handler 生命周期。回调通道有界、非阻塞，注册/注销
同线程；不把测试接收器提取成生产事件层，不把发送 API 次数与系统回调次数混同。

`surface_accessibility_toggle` 负责 Switch 实时会话读取、显露后权限复查、无 Win32
重入的提交点和祖先 click 前缀捕获；不复制 Switch 会话或 FIFO 实现。
`surface_uia_toggle_abi` 管理 Toggle 接口头、实时属性及被共享窗口出口调用的通知 ABI。
`surface_uia_toggle_tests` 验证身份/权限/回压/祖先 click，`surface_uia_toggle_client_tests`
经 MTA 系统客户端验证模式；测试必须先建立真实编辑 binding，不能以默认 false
代替缺失会话。

`surface_uia_event_tests` 负责真实 MTA 事件订阅/缓存属性/COM handler 生命周期回归。
回调使用有界非阻塞通道；不把 sender 接口带回 UI 线程，也不依赖恰好一次交付。
`surface_uia_root_abi` 的通知计数仅存在于测试构建，用于区分 provider 发送和系统
重复回调，不增加生产 telemetry 或持久状态。

`surface_accessibility_invoke` 拥有 Button 自动化激活准入、Host 查询后身份核验及
原候选入槽；不持有独立动作队列，不发送网络。`surface_uia_invoke_abi` 仅管理
Invoke 接口偏移、共享 COM 生命周期和接纳通知。`surface_window_pointer::Activation`
区分三种候选来源，Automation 不冒充物理键盘焦点。回归在 `surface_uia_invoke_tests`，
系统 MTA 调用复用 `surface_uia_client_tests`；鼠标 press 与自动化入槽必须互斥。

`desktop/surface_accessibility_focus` 是 UIA 焦点权限与状态桥：复用键盘 owner，
不维护第二份逻辑焦点，不生成 click/change；系统调用与 Host 查询后重新核验
场景身份、本地编辑权限和 HWND 焦点。`surface_focus_navigation::reveal_focus`
由 Tab 和 UIA 共用。COM 专项在 `surface_uia_focus_tests`，系统 MTA 客户端聚焦
在 `surface_uia_client_tests`；不把焦点接入等同模式、通知或 EDIT 重定位完成。

`desktop/surface_host_accessibility` 负责 UIA 实时 viewport 同步、屏幕/DIP 转换、
完整 bounds 与裁剪可见性；不持 State 或另外缓存布局。`surface_window_accessibility`
是 UI 线程 Weak 注册表与查询生命周期桥，前后校验 runtime-id，表借用不跨系统
调用。State Rc 的每个窗口回调独立保活，stop 按数据地址注销，不让旧 HWND owner
误删新项。模型/窗口回归在 `surface_accessibility_bridge_tests`，COM 几何回归在
`surface_uia_geometry_tests`，系统客户端继续复用 `surface_uia_client_tests`。

`surface_accessibility` 提供不可变快照的关系导航和不回绕 runtime identity，
Text/Button 名称回退到实际显示文本，不复制输入值。`surface_uia_fragment_navigation`
负责语义映射、原生 EDIT 排除及子 provider 构造；子只强持根，不建立根子环。
`surface_uia_fragment_abi` 管理 Fragment/FragmentRoot 独立接口偏移、nullable 输出
和每调用 SAFEARRAY 所有权；共享 Simple 的规范 IUnknown/refcount。对应测试在
`surface_uia_fragment_tests`，系统 UIA 树遍历复用 `surface_uia_client_tests` 消息泵。
不得将未实现的子几何/焦点/命中包装为零矩形、成功或任意根节点。

`desktop/surface_uia_root` 拥有窗口级 COM provider 和撤销标记，按需由 WM_GETOBJECT
创建；不把声明式 scene 的身份当窗口身份。`surface_uia_root_abi` 只实现窄
IRawElementProviderSimple/IUnknown ABI、引用计数、nullable 输出与线程/撤销门禁。
拆分原因是 windows 0.60 生成 trait 无法表示 S_OK + NULL pattern，不允许伪造空
COM 接口。ABI/隐藏 HWND 测试在 `surface_uia_root_tests`，系统 UIA MTA 客户端与
STA 消息泵集成在 `surface_uia_client_tests`，二者均不冒充完整 Fragment/动作支持。

`surface_accessibility` 拥有 UIA 前置只读语义快照及可撤销 Weak 句柄，不持有
HWND/Host/编辑草稿。SurfaceWindow 负责创建、替换前撤销、成功后发布和关闭撤销。
模型测试归 `surface_accessibility_tests`，真实隐藏窗口生命周期测试归
`desktop/surface_window_accessibility_tests`。后续 COM provider 不得保存 State
裸指针，也不得把静态描述当作动作授权或当前编辑值。

`surface_switch_visual` 是 Switch 纯状态色计算子模块，读取 checked/disabled/
hover/pressed，不持有草稿/授权或资源。`surface_pixels_fixture` 是像素测试的
共享 WIC/COM owner，保留原画刷恢复和后续绘制标记验证；旧像素入口及新的
`surface_switch_visual_tests` 共用它，避免复制原生渲染器或扩大原 500 行测试文件。

`desktop/surface_edit_prepare` 只负责共享字段 FIFO 的提交准备和可选 click 前缀
截止；队列、Session、合同及结算仍由 Mount 持有。无 handler 意图被跳过后仍须
重查 fence，不能把边沿之后的 change 提前准备。`surface_switch_bubble_tests`
是 Switch HWND 测试的独立子模块；旧 JSX 对照在
`tests/unit/DeclarativeSurfaceSwitchContract.test.tsx`，记录 DOM label 转发差异，
不要求原生重复业务动作。

`desktop/surface_switch_session` 是无 HWND 的布尔 Draft/ticket owner；
`surface_edit_switches` 将其挂到 Mount 的共同字段预算与意图 FIFO，合同失效时
必须先清队列再同 ID 重建。不得建立独立的无序 Switch 提交队列。
`surface_window_switch` 只桥接本地激活；鼠标 capture/取消及键盘 Space/焦点仍由
既有 pointer/keyboard owner 管理。Host 绘制借用当前 bool 值，PaintFrame 不存
第二份可变草稿，也不以绘制成功授予提交权限。`surface_switch_window_tests`
独立承载真实 HWND 的混合 FIFO、撤权、旧回执与权威值回归。

`surface_switch` 拥有 Switch 声明值和固定 DIP 静态几何，挂在 PaintFrame 命令序列，
不拥有草稿、焦点或授权，也不分配子窗口。布局/style 默认视图归 node_defaults，
颜色复用 control_colors 的 Neuro 角色。`surface_switch_tests` 是测量测试的独立
子模块，复用已有真实 WIC 渲染夹具，不扩大已到行数边界的像素测试文件。
`surface_input_event` 接受 Switch Boolean/change，但完整交互 owner 尚未接入，
不得仅扩大 focus/Click 判定而绕过本地编辑与提交分离、FIFO 或合同身份校验。

`desktop/surface_number_preflight` 同时拥有无修饰 Delete/Backspace 和
WM_CLEAR/WM_CUT 的删除范围候选：Delete 单次，Backspace 按重复计数且先删
选区。仅非组合 Number 使用，不执行修改或剪贴板操作。`surface_number_erase_tests`
以真实 EDIT 保护删除合同、undo 和事件；剪切只验证共享候选，不发送 WM_CUT。
修饰键与组合状态仍由既有 Session fallback 处理，不能扩大撤销保护结论。

`desktop/surface_number_paste` 拥有显式非组合 Number 粘贴的短期事务标记，不拥有
草稿或事件。读取前后通过 CAN_PASTE 检查当前 Host/字段类型/本地编辑和组合资格，
同时检查 enabled/readonly；编辑、选区、焦点、组合、权限和窗口变化令标记失效。
`surface_number_clipboard` 仅一次打开 Unicode 剪贴板并复制有界 NUL 结尾 UTF-16，
先解锁再关闭，不释放系统句柄。锁外同一快照经 EM_REPLACESEL 复用 preflight 与
原生 undo，不重读剪贴板。`surface_number_paste_tests` 注入读取结果，通过真实
HWND 验证重入/撤权/关闭和事件；不操作用户剪贴板，不替代真实跨进程剪贴板验收。

`desktop/surface_number_preflight` 在非组合 Number 的 WM_CHAR/EM_REPLACESEL
进入默认 EDIT 过程前，以当前 UTF-16 选区构造有界候选并验证编辑 grammar。
WM_CHAR 候选展开低 16 位重复计数，与真实 classic EDIT 的重复插入一致。
拒绝不发送编辑边沿，不回写文字，因此保留系统 undo。替换指针遵守 Win32 同步
消息的有效 NUL 结尾 UTF-16 合同，最多读取 MAX_UNITS + 1，不保留或跨线程传递；
预检模块本身不读取剪贴板；显式粘贴由独立事务 owner 负责。Session 为不可预判
路径兜底，不能把预检通过当作提交权限。

`surface_number::editable` 定义数字编辑态 grammar，与有限数值提交解析分离。
`surface_edit_messages` 对非组合 Number 的可预判插入调用有界候选预检；
粘贴和未预检的整段结果仍在 Session 边沿验证，不在此层读取系统剪贴板。
Session 在最外层编辑/组合开始保存选区，结构非法时恢复旧草稿和范围；相等值
不产生 input。`surface_number_filter_tests` 保护字符、整段替换、排队值、未完成
指数和组合结束接缝；不把文本回写当作撤销历史或滚动位置无损恢复。

`desktop/surface_edit_placeholder` 只拥有空字段提示的有界 UTF-16 缓冲和 EDIT
subclass 生命周期，不拥有值/草稿/事件。Session 创建时安装，合同变化重建。
窗口 property 标记同一 owner；callback 临时 Rc 防止默认过程重入销毁后的悬空
引用，NCDESTROY 移除标记与 subclass 并释放窗口引用。提示按空值/组合状态绘制，
不使用 WM_SETTEXT；借入 DC 保存/恢复，取得 DC 成对释放。Input/Number 单行
去除 CR/LF，Textarea 提示保留换行。`surface_edit_placeholder_tests` 用真实
WM_PRINTCLIENT/DIB 像素证明显示与真实文本隔离，不代表桌面视觉或真实 IME 验收。

`desktop/surface_number_spinner` 是 Number 系统 up-down 子窗口的单线程 RAII owner，
仅拥有 HWND、捕获和系统重复行为，不拥有数值、网络或提交权限。Field 先销毁
spinner 再销毁 EDIT；marker 防止已销毁 HWND 被复用后误清理。共用 geometry
按同一像素边界分区，并减去祖先/兄弟/滚动覆盖层，最终空区域禁用 spinner。
`surface_window_edit` 在 Host 借用外切焦，再复核身份/门禁，复用 STEP 与原队列。
`surface_number_spinner_tests` 保护实际 common-control 鼠标消息、撤权、组合、
Host busy、区域遮挡与关闭；未覆盖真实长按重复或可见桌面人工视觉验收。

`surface_number` 拥有 Number 的严格浮点字符串转换和有界 min/max/step 计算，
不持有 HWND、网络或动作权限。`surface_edit_session` 继续保存原始本地字符串，
事件准备时转换为 number/null，表单快照使用经校验的字符串；空/未完成输入不
伪造为零。EDIT subclass 只发 STEP_UP/DOWN，Mount 复核资格并在改可见值前
预检 input/change 双边沿容量；Host 取出期间同步通知不能成为第二个 queue writer。
`surface_number_edit_tests` 覆盖真实 EDIT、异步回执、撤权、IME 和容量边界。

Icon 是无资源访问的文字叶子：`surface_node_defaults` 提供有效默认样式，
`surface_button::content` 共用文字内容入口读取 glyph；`surface_scene` 保留原始
合同但忽略 Icon 本地 style/layout 和子树渲染，与旧 JSX 入口一致。测量、文字帧
和缓存投影复用 DirectWrite；内容盒居中不覆盖继承的多行 textAlign。
`surface_icon_tests`、`surface_icon_pixels_tests` 与窗口 owner 测试分别保护
合同/缓存、真实 WIC 像素和 HWND 生命周期；不新建资源下载或动作 owner。

`desktop/surface_window_gesture` 拥有单个 Windows pan 序列与已处理手势句柄的
RAII 清理；泛型 BEGIN/END 和未处理消息交默认窗口过程。Host 只做坐标转换、
anchor 获取及 EDIT 同步，Presentation 在同一滚动副本上计算双轴并一次投影，
复用文字布局缓存。pointer 取消链负责清序列，不另建惯性线程或定时器。
`surface_window_gesture_tests` 覆盖真实 HWND 配置读回和解码后状态机接缝；不
代替真实触控设备、系统消息句柄或 EDIT 子窗口互操作验收。

`native/tests/startup.rs::run_bounded` 以进程内互斥门禁隔离完整真实桌面 smoke，
保留每 child 15 秒期限与全部功能断言；此处不是多实例负载性能门禁，不影响产品
运行时并发。子进程和 reader 的有界诊断/失败回收仍由 `tests/support/process.rs`
负责。测试二进制之间及外部桌面负载不在此门禁隔离范围内。

`surface_scroll_keyboard` 将本地滚动命令映射到共用轴准入和祖先消费规则；
`desktop/surface_window_scroll_key` 只在父 HWND 实际持焦且有 local_edit 时接收
无修饰键的方向/Page/Home/End。Keyboard 分别持有动作焦点与滚动 anchor；
背景点击可建立 anchor，不能借此获得动作提交权。滚动保留焦点/anchor，撤销
旧按压和候选。系统 EDIT 保留自身按键。`surface_scroll_keyboard_tests` 和
`surface_window_scroll_key_tests` 保护模型与真实 HWND 接缝。

`desktop/surface_scroll_repeat` 拥有按住轨道期间的唯一 HWND timer、单调到期时间和
RAII 清理；`desktop/surface_window_repeat` 每 tick 复核捕获/本地资格，再请求 Host
按当前轨道身份、轴和原方向分页。不追赶积压消息，不跨手势复用 timer ID。
`surface_window_scrollbar::clear` 同时清拖动及重复 owner，所有 pointer 失效路径
共用它。`surface_window_repeat_tests` 覆盖可控时序及真实 WM_TIMER 分发。

`surface_scrollbar` 拥有覆盖式滚动条范围、轨道/thumb 几何、分页和绝对拖动计算；
`surface_scrollbar_presentation` 统一绘制/命中和呈现偏移提交，不创建动作。
`desktop/surface_host_scrollbar` 转换 DPI 坐标并同步 EDIT；
`desktop/surface_window_scrollbar` 拥有单槽捕获，独立于提交权限，以 local_edit 准入。
取消、失焦、场景/尺寸/DPI/权限变化沿 pointer 失效链清理；没有条拖动时不得吞掉
普通 pointer 的 move/up。EDIT 区域减去同一覆盖层矩形，避免子 HWND 截走拖动。
`surface_scrollbar_tests`、`surface_scroll_pixels_tests`、`surface_window_scrollbar_tests`
分别保护模型、实际 D2D 像素与 HWND 生命周期。滚动条不拥有独立线程或定时器。

`surface_focus_navigation` 拥有顺序候选显露事务，navigation 集合不可替代可见动作
资格。`surface_scroll::reveal` 只调整候选偏移，程序显露与 wheel 的轴准入分离；
未显露目标的滚动不提交。Host 在成功显露后同步 EDIT，再转移系统焦点。
`surface_focus_reveal_tests` 与 `surface_edit_navigation_tests` 保护模型和真实 HWND 接线。

`surface_scroll` 拥有当前场景有界滚动偏移/范围，不持有 HWND、网络或权限。
`surface_layout_snapshot` 拥有无自引用的未滚动布局；`surface_scroll_frame` 拥有同场景
测量缓存，复用 DirectWrite 布局并统一投影文字裁剪、几何、绘制与命中快照。
缓存仅随 Presentation 的不可变场景使用，场景替换及尺寸/样式重排均重建；结构
匹配检查不替代样式失效规则。`surface_scroll_cache_tests` 保护测量次数及失败保留。
`desktop/surface_window_scroll` 仅处理系统滚轮偏好和本地门禁，
Host 同步 EDIT 位置；它不复制工具面板或画布的滚动业务。`surface_scroll_tests`、
`surface_scroll_pixels_tests`、`surface_window_scroll_tests` 分别保护模型、像素和 HWND。

`surface_window_deferred` 拥有单槽点击快照、原绑定/合同摘要和激活期限；
`surface_edit_intents::Fence` 仅跟踪当前队列作用域内的编辑前缀，不授予发送权限。
`surface_authorized_window` 在前缀完成后重新核验当前授权并准备点击；
`surface_edit_dependency_tests` 保护鼠标/键盘前缀、失败、合同变化和后续输入顺序。

`surface_host_edit::capture_form_values` 不只是值查询：真实鼠标/键盘激活调用它时
通过 Mount 封住编辑队尾，禁止 continuous 合并跨越离散边沿。纯值查询仍使用
Mount 的 values；不得把会改变队列边界的接口当作普通渲染访问器。

`surface_edit_intents::Intent.expires` 记录未发意图单调期限，Queue 只在新输入合并
队尾时续期，取消保留原值。Mount 的 prepare_at 在编码前截断过期依赖链并保留
草稿；生产 prepare 使用 Instant::now，测试可注入边界时刻，不修改系统时钟。

`surface_edit_contract.rs` 流式生成编辑节点合同 SHA256 摘要，只忽略根 value；
不拥有授权或生命周期。Mount 在修改字段前完成所有摘要计算，`Field.contract`
作为会话复用条件，替代仅 events 的比较。`surface_edit_action_tests` 覆盖相同
action 下属性/布局/样式/可访问性变化使旧意图失效；纯 value 更新保护由原回归覆盖。

`surface_batch_delivery` 将非成功/非明确未启动/非有效确认等待的编辑结果交给
`surface_edit_mount::fail`；`surface_edit_intents::Queue::failed` 仅按当前登记事件
截断依赖队列。Session 只归还本地 ticket，不成功结算或自动重放；相关链路由
`surface_edit_delivery_tests` 的真实 HWND/合成回执回归保护。

`desktop/surface_edit_intents.rs` 拥有有界编辑 FIFO、相邻 continuous 合并及在途
容量预留；不拥有 HWND/授权。`surface_edit_mount` 捕获 phase 与边沿值、绑定
队列生命周期，`surface_edit_session::prepare_value` 为快照编码且保护后续草稿。
`surface_edit_order_tests` 覆盖真实通知的 change 边界、跨字段顺序和取消归还。

`surface_edit_mount::Field.actions` 固定挂载会话的事件声明，sync 仅复用同 ID、
同类型、同 events 的字段。动作变化由 Mount 重建 Session，不让旧草稿改投新
action；`surface_edit_action_tests` 覆盖未发 phase 清除及旧回执不能结算新会话。

`surface_window_pointer::Click` 拥有激活时的有界表单快照，鼠标和键盘入口共同
使用 `form_snapshot`，不保存 Host 借用。`surface_host_edit::candidate_events`
消费该快照并沿用 Presentation 目标、布局与编码门禁；不得在调度时用新草稿
覆盖旧点击的值，也不得为保留快照绕过场景/授权失效。`surface_edit_values_tests`
覆盖真实鼠标/键盘消息后继续输入时，候选值与当前草稿分别保持。

`surface_form_values.rs` 拥有每个绑定的有界提交缓存（16 字段、48KiB UTF-8），
不拥有可见草稿、HWND 或发送权限。`surface_edit_mount` 更新/换绑清空该缓存，
`surface_host` 将它交给 Presentation 的鼠标/键盘事件准备；`surface_click_event`
只在 includeSurfaceValues 开启时合并 payload，沿用既有 wire/批次预算。
`surface_edit_values_tests` 保护真实字段到按钮 wire、payload 与结果结算接线。

`surface_edit_session` 的用户 maxLength 校验与权威值准入分开；读回先规范化换行，
超限拒绝恢复既有可见值。`surface_edit_messages` 用 EDIT_BEGIN/EDIT_END 包围
原生编辑命令，`surface_edit_mount` 只保存嵌套深度并在外层结束捕获最终值，不
保存逐条文本队列。`surface_edit_limit_tests` 保护真实替换通知、Unicode/LF、
权威超长值缩短和 IME 拒绝后的恢复；不能在 EN_CHANGE 中先提交中间删除状态。

`desktop/surface_edit_mount.rs` 拥有每 Surface 窗口有界字段集合、通知合并和待发
phase；`surface_host_edit.rs` 连接已提交 Presentation 与字段。`surface_edit_geometry`
负责 DIP/像素转换和 region 所有权，`surface_edit_messages` 只子类化 EDIT 的
IME/Tab/Enter 边沿，保留系统默认处理。`surface_window_edit` 连接父窗口通知与
Host，程序同步回写的重入通知不再派发。它们不拥有网络或授权。
AuthorizedWindow 确认激活后绑定字段；Application 仍走原批次 owner/容量合同。
`surface_batch_delivery` 只在证据门禁接纳场景后结算字段 ticket。新模块不能以
字段可编辑代替动作可提交，也不能把无 ACK/未知结果视作可安全重试。

`desktop/surface_edit_session.rs` 组合单字段 EDIT、草稿与事件 ticket，持有绑定和
节点身份，不拥有授权、网络发送或父窗口消息路由。相邻 `surface_edit_session_tests`
验证隐藏 HWND 上的连续编辑、容量、回执、取消和组合期间状态协调；不是实际
IME 驱动或生产挂载验收。无 handler 的 phase 不得释放其他 phase 的未提交值，
组合预留的取消不能冒充远端结果。`surface_input_draft::source` 仅供会话判断
组合期间权威值是否改变，不提供绕过结果门禁的写入路径。

`application_surface_input` 分别计算本地编辑意图和动作提交意图；
`AuthorizedWindow::sync_input` 再施加租约、已确认生命周期和独占 owner 条件。
`Activation::local_edit_active` 只描述本地编辑资格，不能用于领取动作 owner。
`SurfaceWindow::set_input_policy` 区分完整失效和提交候选失效；场景更新后的键盘
焦点协调位于 `surface_window_keyboard.rs`。`surface_edit_policy_tests.rs` 保护
真实授权/撤销边界；不要用保留本地焦点绕过动作结果/commit 门禁。

`surface_input_event.rs` 拥有文本输入的 phase/class/value 合同，复用
`surface_click_event.rs` 的内部 Declaration 序列化和不可变事件；不建立第二套
发送通道。`surface_input_draft.rs` 只拥有单字段权威值、本地值和有界异步 ticket，
不拥有 HWND、授权或整个 Surface 的字段容量；外层仍需限制字段数量/总量。
未启动的派发 cancel，真实结果 settle；不能把后台提交开始等同于草稿 scope 失效。

`desktop/surface_edit_control.rs` 只拥有一个 Windows EDIT 子窗口、身份标记和
有界读写，不拥有 Surface 绑定、草稿、网络事件或应用级输入权限。内置 EDIT
承担实际编辑基础；调用方必须在绑定身份和通知处理后才显示。所有程序写入需
独立容量检查，不能依赖 EM_SETLIMITTEXT 限制 WM_SETTEXT。相邻测试验证真实
Unicode 编辑及父子窗口销毁；不要把基础 owner 测试当作完整 IME/Surface 验收。

`surface_button_visual.rs` 拥有借用的绘制状态及默认/悬浮/按压颜色选择，构建时
记录显式样式优先级，不在鼠标移动时重建 PaintFrame 或 DirectWrite 布局。
`surface_window_pointer.rs` 拥有 hover/leave 跟踪与按压反馈，状态变化才触发绘制；
`surface_window_cursor.rs` 只设置客户区的共享系统按钮光标。真实像素及 HWND
状态回归分别在 `surface_button_visual_tests.rs`、`surface_window_feedback_tests.rs`。

`surface_focus.rs` 从已提交场景/几何生成有界按钮焦点序列；`surface_focus_draw.rs`
只持有焦点线几何，作为 PaintFrame 节点命令绘制，不得改成覆盖全部兄弟节点的
场景顶层浮层。`desktop/surface_window_keyboard.rs` 拥有逻辑焦点、Space 状态和
HWND 按键处理；动作候选共用 pointer owner 的单槽及场景身份，不新增旁路派发。
`surface_window_keyboard_tests.rs` 验证真实消息链，像素合同仍由 WIC 测试 owner 保护。

`surface_button.rs` 拥有 Button 静态默认样式、label 选择、双轴内容 inset 和默认
最小尺寸。`surface_scene::Node` 按节点生命周期惰性缓存计算样式，不改写原始
props/layout/style。测量、文本帧和布局下限共用这些规则，避免测量/绘制尺寸分叉。
`surface_button_tests.rs` 验证布局与样式；WIC 像素和 HWND 点击测试留在各自已有
测试 owner。控件能力声明必须等待完整交互，不以基础绘制通过替代键盘验收。

`desktop/blur_tile_padding.rs` 拥有设备像素足迹到源纹理邻域的映射与二次幂分桶。
`annotation_blur_tiles.rs` 保持固定源网格，但缓存区域必须包含当前足迹所需邻域，
不能把缩小时只有 1 个源像素的边缘误作足够的插值保护。上传区域裁到源尺寸。
`blur_local_tile_tests.rs` 从完整局部 blur 绘制入口对照整图路径，并验证重复缓存，
不是仅对 tiles 函数单测。极端缩小和实际上传峰值不能由一般倍率回归代替。

`export_source_resolution::Layout.interpolation` 明确区分源图取样行为。默认最近邻
保护底图/擦除/普通导出；`export_blur_local.rs` 单独选择 LINEAR。共用
`export_source_sample.rs` 不自行推断调用目的。`export_source_sample_tests.rs`
验证真实读回灰阶，局部资源测试验证平滑投影进入卷积，不能只测枚举字段值。

`desktop/annotation_blur_geometry.rs` 拥有局部遮罩的双轴几何变换和 wash 范围。
必须先加宽完整路径再变换，不能用单一笔宽替代非等比缩放；比例由局部资源准备
阶段计算并存入 BlurImage，不在绘制时重新猜测输出密度。局部旋转取持久 frame
中心；annotation_blur 的两层矩形几何 clip 防止旋转退化为轴对齐包围框。几何
layer 有 D2D 内部资源成本，不能将 CPU 像素预算视作完整 GPU/驱动峰值测量。

`desktop/export_blur_sampling.rs` 只负责懒恢复未翻转原图，无翻转时共享像素。
`export_blur_boxes.rs` 按有界 RawValue 解析持久效果框，不复制 points；通过稳定
标注 ID 匹配，不按恢复排序下标配对，也不重建 points bounds。`export_blur_local.rs`
拥有独立源投影、现代 crop 的直接像素偏移和局部透明邻域卷积，不读取传播内容框。
局部纹理 frame 加一次 editor 可见原点；投影颜色 clip 与整个效果框 clip 分开，
后者约束白色覆盖。缺失/非法持久框明确失败，不回退整框模糊。
`export_source_sample.rs` 提供底图专用 D2D 取样和预算读回，供裁剪模糊/擦除
共用；拒绝携带边框/圆角的 layout，不提前合成透明度、标注或装饰。
`session_workflow_blur_pixels_tests.rs` 保护原始方向、裁剪及取整；子模块
`session_workflow_blur_local_tests.rs` 保护排序后的身份映射、独立颜色与白色覆盖边界。
`export_blur_local_resource_tests.rs` 直接测试生产入口的尺寸、共享、取消和失败原子性。
这些测试不能替代 kernel、浏览器离屏 rasterization/插值及复杂组合的一致性验证。

`desktop/export_source_content.rs` 负责传播内容框到底图 placement/clip 的映射，
只读取固定大小的借用会话字段，不持有图片副本。source_resolution::Frame 区分
底图 clip 与完整标注框；export 在绘制底图后立即弹出 clip。工作流擦除使用新
底图布局，模糊通过独立效果局部框路径准备，不把两种主干坐标合同混为一谈。
`session_workflow_content_pixels_tests.rs` 用实际工作流 PNG 保护 contain、翻转、
逻辑裁剪及擦除组合。缺少 sourceSize 的旧式裁剪由 `image_projection.rs` 表达
有界源/目标矩形及 Empty 状态，`export_base_draw.rs` 统一 D2D 源矩形绘制和
clip 生命周期。不得通过放大完整图片 placement 支持极窄裁剪。
`export_source_erase_projection.rs` 仅在有笔迹时，用同一 D2D 路径物化最终尺寸
底图并带预算逐行读回，成功后切换 Frame；随后擦除使用可见目标坐标。不能把
放大后的局部笔迹压回一个源像素，也不能把无效笔迹变成另一套取样算法的开关。
无笔迹不物化像素副本；模糊完整对齐继续单独跟踪。

工作流分量当前使用完整 data 替换合同。Loom 尚无图片字段缺失即保留的接收能力，
不能仅依据图片签名或成功 ACK 缓存省略 src/previewSrc；此限制由 batch wire 回归
保护。未来增量协议须先明确接收端能力和重连/缓存失效语义，不在 serializer 偷改。

`session_workflow_sources.rs` 将保存会话适配为现有 graph_image 借用视图，负责
工作流合成底图身份，不读取文件。预览、正式输出、手动图片及中继开关遵守图规则；
独立栅格标注层保留自身 src 优先例外。参数/输出视图持有解析字节预算，选中引用
只在消费回调期间有效。workflow prepare 复用本节点已解码图片；其他底图由
session_prepare::workflow_image 经显式 Root/inline 解码并应用目标翻转，逐张释放。

归档 worker 在资源发布前将 Ready 消费为私有 ArchiveInput，立即释放已不再用于
发送的烘焙快照及其编码租约；原会话字节及对应租约保持到归档暂存完成。转换在
后台执行，不把大快照析构移回 UI；ArchiveInput 不提供 batch/发送接口。

工作流 UI 的目录选择前后使用同一 Receipt 复核工作区、保存基线及 Loom 目录
身份；模态选择结束不能重新捕获新身份并沿用旧选择授权。资源 Batch 保留已验证
Plan 的来源采用列表，归档 writer 在同一候选内安装 originWorkflowId/originNodeId，
提交成功后通过 Record.original 成为下一次保存的合并基线，不另建第二份 UI 来源状态。

`application_workflow_cycle_tests.rs` 负责应用级成功链路集成验证：真实隐藏 HWND、
隔离临时会话及固定证书 TLS/WebSocket 服务共同验证先归档提交、后发送新 revision。
服务在 ACK 前读取磁盘会话与归档索引；客户端检查完成消费及保存基线交换。
该测试不连接用户 Loom，不替代正式菜单、安装服务或完整迁移验收。

`application_workflow_ui.rs` 拥有菜单请求、显式可写目录选择和本会话保存后同步
配置。保存成功只排队最新 Source；Cycle 运行期间合并保存意图，不积压图片。
完成后消费报告与恢复警告，失败不自动重试；关闭自动同步不伪称取消已启动任务。
权限配置随 Record 释放，不持久化会话内路径为写权限。对话框仅在 UI 消费请求时打开。

`application_workflow_cycle.rs` 是当前 Record 的工作流周期 owner，按准备归档、
归档提交、新 revision 重准备、同步、完成消费推进。一次请求只传显式 Root，
周期结果保留归档 Published（含恢复警告）及部分同步报告；失败不自动重试。
消息循环通过 process_workflow_sync 服务周期，普通保存触发由 application_workflow_ui 负责。

`session_workflow_archive_job.rs` 负责后台资源归档、下一 revision 组装和原会话
精确基线暂存，不自行 commit；Staged 先释放替换 owner 再释放图片/目录句柄。
`application_workflow_archive.rs` 持有 UI Receipt，在不泵消息的区间复核、提交
并交换 Source/保存 Stamp。归档前 Ready 不得继续用于网络发送，必须按新 revision
重新准备。恢复文件警告不可丢弃，也不可伪报整个保存失败。

`desktop/session_asset_archive.rs` 将单资源发布组成逐张批次，持有 source/preview
的 Published 句柄并生成索引；普通引用保留但不隐式读取。`session_archive_write.rs`
把完整引用集合和索引安装为下一修订候选，复用 session_write 的预算编码与拓扑
复验。Prepared 只借出候选，资源句柄持续保护至磁盘提交结束；失败不删除已发布资源。

`desktop/session_asset_publish.rs` 在显式 Root 内发布资源；复用 Staged 的同目录
暂存与无覆盖提交，返回持有只读句柄的 Published。上层必须持有它直到会话提交
完成。同名文件须核验字节，不能仅信任 FNV 文件名；不删除已有资源或执行 GC。
`loom_output_inline::decode_bytes` 只拥有有界编码字节，发布前仍必须经 WIC 验证。

`application_workflow_completion.rs` 保留完成结果与原 Receipt，领取只转移 owner，
不授予来源写回权限；过期/失败结果仍可读取部分成功报告。`session_workflow_archive.rs`
只构造已发布引用的索引候选，整体替换命中工作流、保留其他工作流，不负责资源
写入或 revision 提交。计数编码检查大小，不为检查再分配完整 JSON 字节副本。

`desktop/session_workflow_sync.rs` 拥有同步 worker 和容量为 1 的准入邮箱；每条
请求使用独立一次性 reply，等待有截止期限，取消/丢弃/断开都不得批准下一条。
`application_workflow_sync.rs` 将 Receipt 留在 UI，start_sync/service 复核保存
状态与目录身份；后台只接收固定 Identity 和 Ready，不传 Rc 或窗口引用。

`session_workflow_dispatch.rs` 顺序消费批次，每条固定身份连接完成后重新调用
准入门禁；只在严格成功回执后推进，失败保留当前消息及已确认数量。门禁不得
跨线程读取 Application/Rc/RefCell；应用接线应采用 UI 握手，不是整批一次授权。

`session_workflow_exchange.rs` 是单条工作流控制请求的后台交换 owner，消费调用方
已准入的 BridgeSocket，独立预留发送副本预算；socket/待发送缓冲先释放，租约后
释放。成功必须匹配协议、请求和工作流身份；不自动重试、推进批次或写回来源。
固定证书 TLS/WS 回环验证位于 `loom_workflow_exchange_tests.rs`。

`session_workflow_batch.rs` 拥有全局/来源工作流的惰性请求批次，借用 Snapshot，
单槽持有编码字节及预算租约。编码前按消息上限预留预算；同一待确认消息复用
请求 ID 和字节，只接受匹配 ID 的显式成功确认推进。发送 owner 仍须验证真实
回执和最新授权，不能把 current/acknowledge 当成网络验证或来源归档提交。

`session_workflow_wire.rs` 负责工作流控制协议 envelope 和共用有界 JSON 编码，
不得将前端 action payload 当作 Loom 控制请求。Snapshot/Component 提供借用式
encode_request；请求 ID 分配、重试/ACK 和发送授权归发送 owner，不由编码器猜测。

`desktop/application_workflow.rs` 是应用层单槽与 Receipt owner，归属当前会话 Record，
不把准备结果直接当作发送授权。必须同时核对保存状态、Source 存储身份、工作区
Guard 和能力目录；消费结果后仍保留可复核凭证。`application_workflow_smoke.rs`
通过隐藏 HWND 保存链验证该入口，不能替代正式调度/网络验收。

`desktop/session_workflow_job.rs` 拥有工作流准备线程、取消和结果，统一从会话到
快照/来源计划。调用方必须传已准入 Root，不能让任务自行从数据推导路径权限。
Ready 只读提供快照及精确来源匹配；UI 持久化 receipt、发送资格仍由上层验证。
生命周期和真实图片回归位于 `session_workflow_job_tests.rs`。

兼容遗留字段必须以实际调用链为准，而非仅凭类型定义推测功能。主干图片合成
不消费 imageEditState.rotation，原生工作流不应新增整图旋转或因此拒绝保存。
它与标注 rotation 不同；原会话的遗留元数据继续保留，不由烘焙阶段迁移写回。

`desktop/export_beautify_style.rs` 负责美化预设与样式读取；`export_beautify.rs`
负责最终内容的背景、白卡片、圆角和阴影包装。必须放在内部内容合成之后，不改
标注/节点几何；直接读取 WIC 像素，避免 PNG 往返。`export_beautify_tests.rs`
覆盖实际像素与边界，工作流测试另验证最终 PNG 接线。所有大缓冲必须先预算准入。

`desktop/export_source_blur.rs` 按持久框四元组与取整绘制半径缓存局部导出资源，通过
`Editor::export_snapshot_with_blur_strokes` 按稳定身份安装到独立快照，禁止写回活动历史。
`export_source_resolution::Frame` 保留持久裁剪的自然输出尺寸与原始 contain
映射，擦除、效果准备及合成必须共用，不得在拉伸后的源框中重新猜测原图几何。

`blur_image.rs` 拥有模糊像素与逻辑采样框，两者不能混为同一尺寸。桌面整图绘制
及 `annotation_blur_tiles.rs` 都须使用该框映射，分块缓存仍按源像素网格建立，
不能因绘制密度变化重复拷贝整图。默认构造维持原像素坐标合同。

`blur_scaled.rs` 负责逻辑模糊强度到双轴像素标准差的转换及有界核构建，复用
`blur.rs` 的环形行缓冲卷积。旧入口不放宽工具强度范围；新入口不以静默钳制
掩盖过大密度。核权重拥有预算租约，等轴共用，不在 UI 绘制回调计算。

`desktop/export_source_erase.rs` 负责历史擦除路径的有界解析、可见框到自然底图
的映射和逐笔候选替换，不修改共享源图或独立标注。`erase_frame.rs` 的历史入口
允许贴图缩放后的笔宽；交互入口继续约束工具范围，不得为了恢复兼容放宽 UI 输入。

`desktop/export_frame_clip.rs` 负责工作流圆角半径准入与 D2D 蒙版层生命周期。
蒙版必须覆盖整个合成栈，并独立预留像素预算；先 PopLayer 再释放预算，且必须
早于 EndDraw。裁剪坐标沿用 Layout.visible，不重新从窗口尺寸猜测。

`desktop/export_frame_border.rs` 负责工作流边框样式准入与 D2D 绘制，复用有界
纯色解析，绘制到可见逻辑框。它不修改编辑器或普通导出；合成器在底图之后、
独立标注之前调用，保证透明度与叠放顺序。像素验证由真实工作流与源分辨率测试负责。

`desktop/session_workflow_prepare.rs` 是恢复候选的工作流内容准备接线层，组合
参数策略与真实原生烘焙。`Prepared` 持有快照及编码租约，只允许借用快照，禁止
在释放预算后转移大图片内容。候选继续编辑或含草稿时拒绝准备；该模块不负责
持久化确认、授权、来源更新或发送。贯通测试位于 `session_workflow_prepare_tests.rs`。

批量内容回调必须将同一会话已解析的 Window 交给内部烘焙方法，不得逐张重新
解析完整 JSON；内部方法不向外暴露，公共单张入口继续按 UnitId 解析查找。

`desktop/export_source_resolution.rs` 负责自然源分辨率布局与后台 PNG 编码，复用
export 的合成器但不改变普通导出合同。`session_workflow_bake.rs` 负责恢复候选
准入及源选择，继续编辑后不能复用旧源；未实现效果必须拒绝。相关真实 WIC
测试覆盖像素、透明度、裁剪密度及候选编辑隔离。

`session_workflow_metadata.rs` 负责来源采用候选和 archive hints，仅输出身份映射，
不修改活动工作区或生成已持久化资源索引。计划以原会话摘要绑定，空全局工作流
仍产生空提示。`session_workflow_metadata_tests.rs` 覆盖形状、来源隔离和取消。
工作流 source-resolution 烘焙不得直接复用按逻辑框输出的 png_payload；须保留
原图身份、contain placement、裁剪密度及底图透明度，不能改变普通导出合同。

`session_workflow_content.rs` 是工作流快照内容策略入口，复用 session_parameters
按 capability 过滤参数，并按主干图片规则选择直接字段或显式烘焙回调。不得
在烘焙失败时发送旧底图，不得在能力缺失时直接复制 Art 参数。此模块不执行
网络访问；实际原生烘焙由桌面层提供。测试在真实 Snapshot 构造入口验证策略。

`session_workflow_components.rs` 负责来源工作流 BFS 选择和身份重映射，组件视图
借用全局 Snapshot，禁止复制图片/输出树或在准备阶段更新应用来源。断开分量
同 workflow 合并，目标 ID 冲突拒绝；源会话摘要与版本必须同时匹配。
`session_workflow_component_tests.rs` 覆盖票数/平票、合并、重映射和来源隔离。

`session_workflow_snapshot.rs` 属于 session_document 的同步映射子模块：从指定版本
会话构造全局 hook-live 快照，内容回调提供已过滤参数/已准备图片，有界编码 payload。
它不得自行保存、执行烘焙、读取资源或发送请求；上层必须验证持久化身份与授权。
`session_workflow_snapshot_tests.rs` 对照主干字段合同、版本/取消及内容隔离边界。

`loom_result_restore_images.rs` 仅物化会话内联图片，复用 session_image 的 WIC/
共享像素预算，重复 Data URL 共享像素，JSON 与图片共同计入结果预算。安装前
整体准备并检查取消；`Results.restore_outputs` 保留显式预览优先级且不恢复权限。
`loom_result_restore_image_tests.rs` 覆盖实际 Candidate、预览优先及失败原子性。

保存时 `save_value_outputs` 与 `save_output_images` 必须成对捕获：JSON 值与共享
像素引用分开冻结，由 `application_save_assemble` 后台编码并在同一 outputs 对象
合并。`WindowCandidate.insert_encoded_output` 管理嵌套字符串预算及重复端口拒绝。
`application_save_output_image_tests.rs` 覆盖真实 WIC 编解码、混合值及失败原子性。

`loom_result_restore_values.rs` 负责有界、拒绝重复键的会话正式 JSON 端口恢复，
仅接收尚未执行/绑定的新 Results，安装不改变 Idle 状态或获取 Surface 权限。
`session_candidate_results` 从 Window.outputs 原文准备 Art 结果；任何失败不发布
部分候选。`session_candidate_output_tests.rs` 覆盖实际会话恢复与重新保存语义。

`loom_result_save_values.rs` 冻结全部为 JSON 值的正式端口，区分未提交与正式空结果。
`application_save_snapshot` 持有冻结后的 map；`application_save_assemble` 整体替换
会话 outputs。沿用 Guard 的 OutputStamp 校验，不跨线程共享活动 Results。
不下载 resource/stream 描述符；含本地图片的混合端口尚待图片输出编码迁移。

`AuthorizedWindow.bind_node_results` 管理新窗口的一次性注册资格。它刷新授权后
才调用节点绑定回调，调用方不得提前持有 Results/View 借用。应用在回调内复核
服务、退出/授权界面和 Art 节点；失败保留资格，成功永久消费。普通交付不再
首次绑定或换绑，普通 Loom 执行撤销资格后须新挂载窗口显式注册。
`surface_result_binding_tests.rs` 负责此生命周期边界的真实 HWND 回归。

Surface 输出应用在结果借用释放后调用既有 propagate_result，复用下游开关、
锁定、祖先防环和执行准入，不另建通知执行器。同轮每节点最多通知一次，旧/
重复版本直接跳过；没有新结果不分配通知缓存，首次更新前预留全部通知空间。
工作流同步不能用该通知冒充，仍需独立接入既有持久化/协议合同。

`composition_relay_surface_outputs.rs` 只借出成功呈现且交付有效的正式 Commit，
不从收据删除。`application_surface_outputs.rs` 在正常结算复核当前服务、窗口
授权/显示版本和 Art 节点后应用结果；普通交付只能复用已注册附件，不能
冒充重挂载，普通 Loom 执行撤销后不能被迟到结果重新绑定。输入门禁通过节点
已应用版本判断 Commit 是否仍阻塞，恢复证据继续保留。工作流同步及完整重挂载验收待完成。

`desktop/surface_node_results.rs` 是既有 Results 的 Surface 写入子模块。应用须先
确认当前附件并绑定节点，再按完整 binding 和单调 resultRevision 原子替换
正式端口，空输出清除旧端口；与普通 Loom finish 的端口合并语义分开。相同
绑定不重置版本，换附件废弃旧写入资格，普通 Loom begin 撤销 Surface 写入资格。
失败/旧结果不修改输出凭证或状态。应用授权交付、传播与同步尚待连接。

`surface_result_values.rs` 负责把已校验 Surface Commit 转为节点输出值，行为对齐
前端 appSurfaceListeners 的 surfacePortValue：value 解包，resource/stream 保留
原描述符及扩展字段。不调用 Loom 图片/共享内存解码器，不因描述符而联网。
先复用端口数量/身份/重复规则，再一次建立借用原文索引，避免逐端口重扫输出。
输出映射失败不消费 Commit；节点原子应用与结果版本管理尚待连接。

原生 pointer 仍以关闭状态创建，由 application_surface_input 按当前服务、模态/
退出状态、容量及未结算证据请求启用。AuthorizedWindow.sync_pointer 重新校验
授权、可见性与确认激活 owner；移交批次前关闭输入，等待 owner 返回后才可再开。
未呈现收据或待应用输出阻止同附件新输入。相同 pointer 状态不清空按下/候选，
实际状态切换仍失效旧候选；无候选不准备配置。真实端到端交互仍待验收。

`desktop/application_surface_input.rs` 将已产生的 HWND 点击候选接入批次调度，
仅正常可见结算调用，退出泵不发新输入。无候选时不准备租约或客户端配置。
`composition_relay_batch_input.rs` 在回调移交 owner 前检查容量并预留结果位置，
应用校验事件数、服务/设备/窗口版本后移交；启动失败进入 NotStarted 批次结果。
pointer 已接入授权生命周期，端到端运行及输出应用仍需完成。

`desktop/application_batch_resume.rs` 是显式接续菜单的应用处理入口。候选发布
复核当前窗口，菜单只排队原批次 ID；应用重新检查服务/设备/配置、退出/新请求
和窗口版本，才移交 owner。`batch_continuation_for` 只审核不消费，供候选与
执行共享；不得跨 refresh 持有授权 View 借用。此接线不启用生产点击。

`desktop/composition_relay_batch_resume.rs` 管理显式接续选择事务，以原批次首事件
ID 标识整批，候选须满足接续边界并通过调用方窗口审核。取消原位返还，进入
回调前预留结果空间；启动失败将 owner 和完整收据转入既有批次交付队列。
收据转接续任务不重复计预算，不自动提交；原生菜单已接线，实际操作尚待验收。

`desktop/composition_relay_batch_continuations.rs` 负责接续任务应用集合及所有
批次类任务的完成缓存预留。接续按完整事件数占未保存预算，接入正常/退出
等待、取消、收割与原批次交付；在途禁止恢复快照，收割后保留原前缀决定。
普通批次、决定与接续共用 reserve_batch_outcomes，不遗漏并行完成容量。
显式接续选择事务已连接，菜单入口尚待连接。

`surface_batch_continuation_runtime.rs` 是批次 runtime 的接续子模块，共享网络
Slot、Lease 取消与线程等待。重新核对接续边界后只提交后缀，并在网络副作用
前预留原收据追加空间。原前缀 ACK/决定/输出不重查或覆盖，新 ACK 和观察逐项
追加；失败观察使用独立错误，不伪造成功版本。结果可还原统一批次 Outcome，
外层任务错误另行通知。应用任务集合已连接，显式接续入口尚待连接。

`desktop/surface_batch_continuation.rs` 审核原批次的显式接续边界：连续已尝试
前缀必须全部成功，最后待确认项已有被接收的批准，后缀全部未尝试，身份/版本
一致且无重复事件。窗口重新复核授权/版本后，Activation 独占完整有序事件
列表，BatchContinuationInput 同时持有唯一 owner 和完整收据。失败完整返还，
入口不发送或标记后缀；后续 worker 已实现，应用接线尚待完成。

`desktop/application_batch_confirmation.rs` 将明确菜单请求接入批次确认事务。
弹窗前核对服务/设备/配置和窗口，返回后复核退出、服务切换与新请求，再准备
决定并移交 owner；移交失败保留未提交决定证据。`surface_confirmation_dialog.rs`
复用单项/批次的 Win32 选择框及文本安全处理，批次弹窗只返回选择，不消费提示。
菜单候选同时考虑单项和批次收据，窗口不可用的项跳过。真实用户操作仍待验收。

`desktop/composition_relay_batch_confirmation.rs` 按明确事件 ID 选择整批收据中的
待确认项。BatchReceipt.confirmation_ready 统一证据审核，窗口另查当前授权/
版本；候选选择可跳过不可用窗口。回调消费前预留完成/异常缓存，取消原位返还
整批收据，启动失败保留 NotStarted 决定证据。收据转任务不重复计入预算。
该模块本身不显示弹窗、不隐式选择批准，原生 UI 由 application_batch_confirmation 承接。

`desktop/composition_relay_batch_decisions.rs` 管理批次决定应用任务，按整批事件数
共享未保存预算，接入正常/退出等待、取消和收割。启动前为普通批次与决定任务
共同预留完成缓存；成功回填后沿用 batch_outcomes 的窗口交付与恢复日志。
回填异常完整保留在独立有界队列，占容量并阻止生成遗漏证据的退出 bundle；
显式取走接口移交保存责任，不自动重试。收据选择事务与确认 UI 已连接，运行验收尚待完成。

`surface_batch_decision_merge.rs` 将单项决定结果按原索引并回完整批次，保留唯一
owner、其他项和未提交后缀；身份或观察数量不一致时完整返还输入。逐项 decision
保存决定接收证据，批次交付独立复核已确认、提交成功及版本/身份，恢复记录
仅提取脱敏 DecisionHint。合并不启动后续点击；应用任务调度已连接，确认 UI 尚待接入。

`surface_batch_decision_runtime.rs` 为批次选中项提供一次决定提交及原动作收敛
worker，完整 BatchDecisionInput 随结果返还。只更新选中项最后 ACK，其他项
和未提交后缀不动；提交/收敛/交付授权分开。复用共享 Slot、Lease 取消和等待
句柄，启动失败返还输入，显式收割才释放槽位，Drop 回收线程。不自动提交
剩余点击；结果并回批次、决定恢复元数据和应用决定任务集合已连接。

`desktop/surface_batch_decision_transfer.rs` 将 PreparedBatchDecision 与唯一 owner
一起移入 BatchDecisionInput，窗口复核授权/版本/提示期限，失败完整返还准备
结果。Activation.take_batch_decision 允许已尝试原事件，但只接受未尝试且
身份匹配的决定，整批身份/顺序仍保持独占。普通批次提交仍拒绝已尝试事件，
决定移交不会把原点击变成可重放命令；返还继续使用完整批次身份检查。

`desktop/surface_batch_confirmation.rs` 按项审核原批次收据的提示，复核当前窗口
授权/版本、ACK/观察索引和提示期限；部分完成导致 Evidence 拒绝呈现不等于
有效确认项失效。prepare_batch_decision 消费提示后返回 PreparedBatchDecision，
同时拥有完整收据、索引与决定；失败完整返还收据，不移交 owner 或发送请求。
不得只摘走单项而丢失其他事件/输出/后缀；批次接续任务与 UI 尚需接入此接口。

`surface_event_batch_record.rs` 将批次结果/收据展开成逐事件脱敏记录，并附加
可选 BatchHint（首事件 ID、序号、数量、整体呈现阶段）。严格拒绝缺项、错序、
重复 ID、绑定/版本不一致及矛盾整体成功；未尝试后缀不伪造 ACK。只有真正应用
的最高版本快照对应项标记单项呈现成功，其他项仍保留整体呈现提示。旧无 batch
记录可继续读取；记录不含 payload/提示正文/场景/凭据，不授予重放权限。

`desktop/composition_relay_surface_batches.rs` 负责批次任务、完成结果及收据缓存。
未保存预算按批次事件数计入全应用 32 条限制；结果在交付前占网络容量，收据
不占网络槽位但仍占动作预算。开始/结算前预留结果/收据内存，拒收原样保留。
批次已接入等待、取消、退出泵与窗口结算；恢复 bundle 已包含所有批次事件
与未交付 owner 的附件提示。在途任务仍拒绝快照；显式取走结果后调用方承担保存责任。

`desktop/surface_batch_delivery.rs` 分离批次 owner 接纳与场景呈现。身份/顺序
拒收完整返回 Outcome；接纳后各事件、ACK、观察结果、commit 和提示进入
BatchReceipt，场景不重复缓存。只有 Complete、全部 ACK 成功且匹配、全部
观察成功、版本一致及当前授权通过才替换最高版本场景；替换后仍复核撤权。
Windows.accept_batch 仅路由，应用仍需有界保存收据与退出恢复元数据。

`surface_activation_batch.rs` 管理整批事件对唯一 owner 的独占，复用提交前
完整批次校验，保存有序事件 ID 列表；返还须匹配完整列表及绑定，不要求全部
已尝试（中途停止合法）。单事件/生命周期/缺项/乱序结果不能取回批次 owner。
AuthorizedWindow.begin_batch 先复核窗口授权与当前版本，finish_batch 仅恢复
owner，不应用场景或丢弃批次结果；调用方仍须持有和结算全部事件证据。

批次 Item 在 ConfirmationRequired 时另存 confirmation 结果，读取失败不覆盖
原收敛错误或 ACK。surface_confirmation_until 允许调用方缩短 3 秒读取上限，
批次传入共同截止，开始/读取后/校验后均检查超时；不因多个提示重新延长预算。
仅 GET 原提示，不批准、不重发事件、不自动继续未提交的冒泡后缀。

`surface_event_batch_observation.rs` 在批次 worker 内观察已收到 ACK 的连续前缀，
整批共用 25 秒收敛截止，逐项保存结果版本/错误及 commit，并更新最后 ACK。
仅保留最高 revision 的一份场景，后读到的低版本不能覆盖它；该场景仍只是
交付候选，不代表其他动作成功。观察失败与提交结果分离，不重放事件，也不
为未提交后缀伪造结果。确认提示读取已连接，批准后恢复和应用交付仍待连接。

`surface_event_batch_runtime.rs` 为批次提交提供单 worker 与唯一 owner 输入，
复用共享 Slot、Lease 取消和线程等待句柄。容量/spawn 失败完整返还输入；
线程完成后显式收割才释放槽位，迟到撤权保留原事件/ACK 并拒绝交付。Drop
取消并回收线程，不替代正常收割与证据保存。批次 HTTP 的单项异常捕获保留
此前 ACK，以 Panicked 停止；后台提交完成不代表各动作已收敛或已经呈现。

`surface_event_batch_http.rs` 负责一次点击冒泡事件的顺序 HTTP 提交。整批先检查
同一 owner 绑定/当前版本、未尝试、唯一 ID、MAX_DEPTH 数量与 64 KiB 总字节；
预留 ACK 空间后再发送。整批共享 10 秒提交预算，等待确认/终态失败/提交错误
立即停止，后缀不尝试，已接收 ACK 保留。Complete 仅表示提交循环结束，不代表
动作或场景成功；调用方仍持有原事件列表并负责后续收敛/确认/窗口交付。

应用候选选择使用 pending_surface_confirmation_for 的窗口审核回调，只有
匹配附件的 AuthorizedWindow.confirmation_for 当前通过才进入菜单。窗口关闭、
场景变化或 owner 不可用的旧收据仍保留，但不挡住后续可处理提示。审核可能
同步关闭 HWND，因此不跨该调用持有授权 View 借用；发布候选前复核服务未变。

EventReceipt.confirmation_ready 统一菜单候选和实际调度的收据有效性检查，
排除过期/身份或 ACK 不匹配、失败交付、已有决定/输出及已呈现或已收敛快照。
旧收据仍保留供恢复记录使用，不因菜单过滤而删除。dispatch 在调用 UI 前
重新读取系统时间，不能把此前菜单显示当作长期许可；窗口/租约仍另行校验。

`desktop/application_surface_confirmation.rs` 连接授权菜单的 ConfirmSurface 请求、
当前服务/设备租约、原生对话框、窗口 owner 移交与决定任务。应用循环消费请求，
不在菜单回调提交网络；模态期间不持有 View 借用。返回后退出、关闭、服务改变
或新请求都阻止提交并保留 NotStarted 决定证据。菜单仅显式处理当前服务的一条
待确认事件，不自动弹窗；取消后可重新选择，已经形成决定证据的收据不重复提示。

Sessions.dispatch_surface_confirmation 按明确选择的 event ID 消费一条待确认
收据；关闭/容量不足时不得进入 UI 回调。回调返回 Keep 时放回原位置；Start
启动失败时以 NotStarted 保留完整 Input 到统一事件结果队列，之后结算 owner。
失败队列空间在 UI/owner 移交前预留。移除旧收据后才计算新任务预算，因此同一
动作转换不额外占一个未保存动作名额。回调必须负责原收据及选择的所有权。

`desktop/surface_receipt_decision.rs` 将已消费提示的原收据与未尝试 Decision
移交为后台 Input。拒绝残留提示、旧决定、输出 commit、已呈现场景或失败交付，
避免静默丢弃证据。失败通过 ReceiptDecisionError 完整返还收据/决定；成功
移交唯一 owner，沿用原 event/ACK 并读取当前窗口版本，不复制或重放点击。

`desktop/surface_confirmation_dialog.rs` 提供以 Surface HWND 为 owner 的原生
确认框，默认取消，仅明确是/否生成批准/拒绝决定。展示前与返回后分别复核
授权/期限，原提示仅在准备成功后消费。远端字段转义控制和双向文本字符；
超过 8192 UTF-16 单元拒绝展示，不截断后批准。应用已接入模态互斥、退出
状态与任务调度；本模块不执行网络请求。自动化使用注入选择，不弹真实对话框。

`AuthorizedWindow.prepare_decision` 是显式选择后的提示所有权移交入口：从原
EventReceipt 消费提示，准备失败保留收据，成功不移交 owner 或执行网络请求。
不得为准备决定重新解析一份确认提示，也不得把准备成功视为后续提交授权；
begin_decision 和网络提交仍各自复核当前状态。原生对话框已有显式菜单和应用调度入口。

`desktop/composition_relay_surface_decisions.rs` 接入决定 Job 的应用等待/退出泵，
开始前预留完成缓存，完成结果仍占共享网络容量直到显式交付；拒收原样保留。
决定与点击共用 32 条未保存动作预算，取消只撤销待交付资格，不改写决定/ACK。
surface_recovery_bundle 同时保存未交付决定 owner 的附件提示和动作元数据。
动作记录增加可选 decision 提示（confirmationId、approved、attempted、confirmed），
严格拒绝未尝试却声称已确认等矛盾；不保存确认 payload，也不从磁盘重建批准
权限。决定结果转换为统一事件 Outcome，复用窗口交付、有界收据和退出日志，
不另设结果队列；事件与决定任务共同预留完成缓存。窗口呈现另验决定已接收、
事件/ACK 身份和当前版本，拒收完整返还证据，收据保留决定元数据。
原生 UI 与应用发起决定的显式交互入口已接入，真实服务交互仍需验收。

`surface_decision_runtime.rs` 为显式决定提供单 worker 事务，复用共享 Slot，
输入完整持有 owner/event/ACK/Decision 和当前呈现版本。spawn/容量失败返还
全部输入；提交结果、后续收敛、最后 ACK 与收割授权分别保存，取消/异常不
抹去已尝试或已接收决定的事实。完成后仍占 Slot，显式 join 才释放；Drop
取消并回收线程。Activation.take_decision 与窗口 begin_decision 使用同一
pending event 独占 owner，不通过伪造未尝试点击绕过重放保护。应用任务集合与
收据缓存与原生 UI 的显式选择已接入决定 Job。

`surface_confirmation_decision.rs` 将显式 Choice 与原确认请求绑定，消费 Request，
无 Clone/Deserialize/默认选择；准备失败返还提示。决定提交前检查当前 owner、
场景版本、来源/设备、ACK 与有效期。`surface_confirmation_decision_http.rs`
最多一次 POST，截止不晚于提示到期；开始网络尝试后不得自动重放或翻转选择。
HTTP 2xx 只标记决定已确认，不标记动作成功。converge_decided_surface 只接受
已确认且身份匹配的决定，继续 GET 原动作状态并独立刷新，复用 25 秒总预算，
不重发原事件、不再次批准。后台决定任务与原生 UI 已接入这些接口。

事件 Job 在 ConfirmationRequired 后于同一 worker 内读取一次确认提示，独立
捕获异常并保留最后 ACK；读取结果通过 Outcome、窗口交付和有界 EventReceipt
传递。读取失败不重放动作，取消不弹通用错误。确认 payload 不写入退出提示。
AuthorizedWindow.confirmation_for 在展示前复核窗口存活/显示/激活、完整绑定、
当前场景版本、收割授权以及请求状态/有效期；缓存成功不代表永久展示许可。
此链路不提交批准，原生对话框与决定提交仍需后续接入。

`surface_confirmation.rs` 定义不透明待确认提示，最多 64 KiB；核对事件/request、
完整附件/设备/节点及 action 身份。风险仅 low/medium/high，使用时再次要求已
尝试事件、accepted + awaiting_confirmation ACK 和未过期时刻。payload 是
不可信 JSON，不可作为默认批准理由。Tracker 整批扫描确认提示，同一确认 ID
不能变更风险、期限或 payload；后续失败清除候选。
`surface_confirmation_http.rs` 只在明确待确认状态执行一次设备过滤 stream GET，
复用 3 秒网络上限并在读取后复核取消/会话/有效期；保留最新 ACK，不发批准、
不重放 POST、不常驻轮询。后台 Job 已调用该接口，确认 UI 仍待接入，不能直接从
反序列化提示推导批准权限。

`surface_result_ports.rs` 是统一的类型化输出解析器，Commit 接纳与 outputs()
读取共用它，不保留另一套宽松外壳校验。value 必须显式存在（null 合法），大
JSON 借用原文；端口名先解码再去重，最多 64 个。资源校验小写 sha256 哈希、
五类 kind、MIME 长度、安全整数 size、可选 1..16384 尺寸；流校验 ID、类型/
MIME 长度和安全整数 sequence。类型通过不提供任何资源下载/流订阅授权。

`surface_result_commit.rs` 负责 result commit 的协议/instance/request/generation
身份、有界原文和类型化 outputs 校验：原文最多 1 MiB、最多 64 个唯一端口。
outputs/resource/statePatch 仍是不可信数据，不在此层下载、执行或更新本地图。
Tracker 保留当前动作最高 resultRevision，成功后随独立刷新快照交付；后续
失败清除候选。窗口收据保留 commit，不把它当作已应用输出。32 条缓存的 commit
原文最多 32 MiB，另有有界事件/ACK；无动作时不分配这部分内存。退出记录只
增加可选 resultRevision，不保存输出原文；旧记录缺失该字段仍可解析。

`surface_event_record.rs` 定义动作恢复提示：最多 32 条、总 JSON 64 KiB，记录
附件绑定、事件 ID、基准版本、提交尝试、ACK 摘要和收敛/呈现阶段。不保存
事件 payload、Scene、凭据或远端错误文本；磁盘格式不能还原可重放 ClickEvent
或有效授权。解析拒绝未知字段、无绑定、无效版本及矛盾的成功证据。
`surface_recovery_store.rs` 复用只增文件写入，将动作文件放在独立目录
surface-event-recovery（128 槽），与旧附件格式隔离。应用正常退出在后台
保存两类快照，第二步失败保留第一步文件并报错；不是原子双文件事务，也不是
运行中崩溃日志。快照不消费内存 owner/收据，不自动读取后重发动作。

应用在 activate_surface_windows 的普通结算和退出泵入口，先通过 Sessions
结算事件结果，再处理附件生命周期。Sessions 在触碰窗口前预留收据空间；
拒收保留 Outcome，接纳保留 EventReceipt。未保存动作（收据 + 在途 + 完成
结果）最多 32 个，达到上限拒绝新动作；收据不占网络槽位，避免阻塞附件停用。
收据没有等待句柄，不触发周期唤醒；关闭集合不改写已经完成的历史检查结果。
旧附件日志拒绝未保存动作证据；正常退出使用同时承接两类提示的 bundle，不能静默遗漏。

`desktop/surface_event_delivery.rs` 属于授权窗口的交付边界。接纳必须匹配当前
pending event 与完整绑定；拒收归还原 Outcome。返还 owner 后，应用 Scene
还需复核当前租约、显示状态、已尝试事件、成功且匹配的 ACK、基准/目标版本。
替换失败保守关闭窗口，替换后再检查撤权。EventReceipt 保存事件/ACK、收敛
版本或错误及呈现结果，不保留第二份 Scene；调用方负责有界保存/持久化收据。
Windows 集合只路由并返回收据，不自动丢弃未匹配结果或批准确认请求。

`surface_event_runtime.rs` 在同一有界 worker 内编排一次提交和最多 25 秒的后续
收敛。Outcome.result 保存最后有效 ACK 或提交错误；convergence 单独保存场景
或收敛错误，None 表示未取得提交 ACK。收敛异常也不能抹去提交证据。
delivery 是收割时授权检查，不能代替 UI 消费时复核。Sessions 缓存完整结果，
收敛错误只通知一次；取消和等待确认不弹通用错误，也不自动批准或重放动作。

`surface_convergence_http.rs` 只在一次动作期间读取设备流，总截止最多 25 秒，
每次网络请求使用剩余绝对截止。已确认成功后立即独立刷新；其他读取间隔
使用 Cancel 的可唤醒等待，不创建轮询辅助线程。等待前释放整个批次，避免
无用保留 8 MiB 响应。取消/过期/截止在读取及解析后再次检查；确认状态立即
返回给调用方，保留最后 ACK，不重放动作 POST，不隐式确认高风险动作。

`surface_convergence.rs` 为单事件借用型收敛状态机，核对 ACK/request、附件、
generation 和生命周期。成功后强制单独 after=0 刷新，不能复用观察成功前
的快照；整批检查完才提交最新有效 Scene。批次内快照只借用 RawValue，不
重复复制大 JSON；交付一次后终止。历史丢失/失败/unknown/代际改变均终止，
不自动重试动作。该模块不发网络、不应用结果 commit、不负责 UI 授权。

`surface_stream.rs` 解析设备过滤的消息批次，保留原始有界消息，不应用场景或
推进调用方游标。8 MiB 响应、512 条消息、单条 1 MiB；序列 visitor 在构建
消息集合时执行数量限制，禁止先分配任意长度数组再检查。协议/reset/游标
必须验证，具体事件/附件/generation 由后续收敛 owner 再校验。
`surface_stream_http.rs` 仅单次读取设备 Surface stream，复用固定来源 HTTPS，
401 不回退管理员接口，不启动常驻轮询或重试线程。

Click 候选携带私有 Rc 场景身份，不能跨窗口使用；场景替换、尺寸/DPI、禁用、
失焦或取消会替换身份，使已领取但未准备的候选也失效。消费以所有权移入，
在 State::run 内重新命中并核对节点，准备后再复查身份。AuthorizedWindow
仅接受这种候选，take_prepared_click 单次领取并走当前授权检查，不接受任意
坐标直接绕过候选来源。Rc 只在 UI 线程存在，不传给后台网络任务。

`desktop/surface_window_pointer.rs` 负责 HWND 左键候选，不执行动作。入口默认
关闭，按下/抬起同节点且未超过系统拖动阈值才入单槽；未消费候选不被覆盖。
失焦、捕获丢失、取消、尺寸/DPI 或场景替换均清理，关闭释放捕获。候选仅含
客户区坐标、节点身份和修饰键，消费端仍需重新验证当前场景及授权。生产
应用尚未启用该入口，不得因消息合同测试通过就扩大宿主输入能力声明。

Presentation.click_events 必须先完成整批冒泡事件编码再返回，总输出 64 KiB，
祖先动作超限时拒绝全批，不允许调用方拿到部分动作。Host 在当前 DPI/视口
同步后从客户区像素命中；SurfaceWindow 经可重入 State::run 并复核存活。
AuthorizedWindow.prepare_click 前后检查当前显示、授权、active 及场景身份，
只是准备，不发送；真正提交仍须 begin_event 与后台发送各自的复核。

Activation 的 take_event/restore_event 与激活/停用共用唯一 owner。事件在途
时生命周期任务不得再次取得 owner；返回必须匹配 pending eventId、完整绑定
和 confirmed_active，拒收返还原 owner。AuthorizedWindow.begin_event 先复核
租约、显示/存活及场景版本；finish_event 仅归还 owner，不授予 ACK 交付资格，
不重新显示已关闭窗口，调用方仍须保留并结算事件结果。

`desktop/composition_relay_surface_events.rs` 将 click Job 纳入 Sessions 的共享
容量、等待句柄、正常 poll_surfaces 与退出取消/收割链。未消费结果仍占容量，
拒收原位保留；取消只撤销 delivery，不覆盖 ACK/错误证据。收割时授权仅为
当时快照，最终 UI 提交仍必须重新检查当前租约/绑定/场景版本。
旧附件恢复记录不能表达动作 event/request：有未结算事件时拒绝导出该旧
记录格式，后续必须补动作级恢复持久化，不能静默丢弃事件后宣称完整结算。

`surface_event_runtime.rs` 持有单个后台提交及共享 Slot，线程结束后仍占容量，
直到 try_join 收割。owner 与 ClickEvent 唯一移交，启动失败返还，取消/完成
返还原事件和 owner；不复制、不重发。Outcome.result 保留执行证据，delivery
单独按收割时租约与时刻校验，撤权不得丢弃已收到 ACK，也不得据 ACK 应用更新。
正常调用方必须登记线程句柄并收割；Drop 只作取消/join 兜底，不能作为 UI
消息线程的正常等待路径。此模块不自行轮询、重试或推进场景收敛。

`surface_event_http.rs` 只供后台 owner 调用。提交要求 confirmed_active、完整
Binding 一致及当前场景版本一致，固定 origin/device，经现有设备认证 HTTPS
发送。ClickEvent 标记 attempted 后禁止再次提交同一个对象；没有匹配 ACK
时保留事件用于后续查询，不自动重放或生成新 ID 重试。响应在传输层即限制
64 KiB，不只在 JSON 解析后检查。该 API 不替代 UI owner 的当前租约复核。

`surface_action_ack.rs` 负责有界 ACK 解析、事件/实例/request 身份匹配和状态
表达。公开 Ack 不可直接反序列化，必须经 parse 校验；update 先验证后替换。
accepted、awaiting_confirmation、unknown 均不等于成功，succeeded 仍不等于
场景/结果已收敛。远端 failure 文本只保留为数据，不执行、不自动写日志。
ACK 本身没有 attachment/generation 或顺序号，发送/流 owner 必须另验来源、
生命周期与消息顺序，不能用该解析器证明这些协议未携带的信息。

`surface_click_event.rs` 将 ClickAction 编码为 loom.surface.v1 的离散 click
事件。绑定身份/generation 来自 Binding，baseRevision 必须来自当前呈现快照。
eventId 使用系统安全随机源；一个事件重用原字节和 ID，不应在重试时重新编码。
输出 writer 在写入前执行 64 KiB 整体事件预算，payload 原位序列化，不先生成
无界 JSON 副本。编码不授权、不发送，发送 owner 必须再次复核租约与生命周期。

`surface_click_route.rs` 负责命中节点到祖先的声明式 click 路由；固定深度数组
借用当前 Scene，不复制 payload、不生成事件、不执行动作。Presentation 先用
已提交几何命中再生成路径，禁止从失败候选场景或旧快照获取动作。容器冒泡、
按钮阻断和文字无独立 click handler 对齐主干；选择手势、穿透和权限由输入
owner 在调用前处理，发送端仍负责协议验证及生命周期复核。

Surface Host 的 paint/hit 共用视口同步；命中使用客户区像素和当前 HWND
DPI，不得使用过期布局或为查询创建 render target。SurfaceWindow 查询必须
经 State::run，并在回调后复查存活，忙碌/关闭不得当作可执行旧目标。

`surface_hit_test.rs` 与 Presentation 几何/绘制同批准备，按逆绘制顺序匹配
节点及祖先圆角裁剪，复用 Geometry 身份和父索引。命中只返回节点，不授予
执行动作权限；输入 owner 必须另查当前授权、生命周期、捕获和焦点状态。

`Node::visible_children` 是渲染准备阶段的可见子项入口；测量、布局和样式
必须统一过滤。Scene 原始 children 保留用于安全校验，不得因隐藏删除原始
输入边界。visible=false 不等于 opacity=0，前者完全不占位与 gap。

`surface_progress.rs` 只负责有界数值归一、预计算轨道和只读绘制，颜色来自
`surface_control_colors.rs` 的 Neuro 语义映射。禁止为确定进度创建轮询或
持续动画；画刷归调用方持有，裁剪栈必须配对，UIA/输入另由各自 owner 接入。

`surface_node_defaults.rs` 提供特殊节点的有界有效属性视图，当前仅 Divider。
Node 原始 wire 字段不得删除或跳过容量校验；有效属性只影响测量/绘制。
Divider 对齐前端固定 hr，不接收自定义 layout/style 覆盖；主题切换须后续
由统一主题 owner 提供，不能在每个绘制路径独立猜色。

Spacer 是无内容的可伸缩叶子，NativeMeasure 只记录边框/padding 固有尺寸；
surface_container 给默认 grow=1，显式 grow 优先。不要为 spacer 创建专属
窗口或绘制器，复用既有背景/边框指令；能力报告只追加实际已实现的节点。

文字测量允许零布局宽高；不要用 epsilon 或假字符宽度替代真实零尺寸。
最终 TextFrame 仍跳过零面积内容绘制，保留节点几何/边框；字体大小和
其他输入安全上限不因此放宽。

`surface_layout_value.rs` 统一节点类默认宽度与显式 auto；根盒、测量、正常
流、Stack 和绝对定位必须共用，不能自行把缺失 width 当作固有宽度。
不在此模块执行 CSS 或扩展未知节点控件能力。

根 absolute 自动尺寸必须先从宿主扣除两端锚点，再应用外盒下限和 min/max。
容器默认 width:100% 不等于显式 width:auto，不得仅凭缺少 width 字段就伸展。

根盒未声明高度时必须使用 Measure 提供的内容高度，不得拿宿主 viewport
高度代替。绘制测试若要固定画布大小，应显式声明根高；自然高度与宿主
裁剪需有独立回归，避免夹具把布局缺陷掩盖成“填屏”。

根节点定位由 `surface_root_box::origin` 在尺寸约束求解后计算。absolute
末端锚点必须扣最终尺寸，relative 末端偏移为负；根偏移传给子树但不能
移动 Tree::viewport 的宿主裁剪范围。

`surface_root_box.rs` 负责根盒显式/min/max/边框下限约束；Tree 的 viewport
属于宿主，不得用根盒 size 替代。根祖先裁剪和 Presentation::size 保留宿主
语义；根盒几何大小变化不应反向驱动窗口调整。

`surface_layout_tree::Placed::containing_width` 负责百分比 padding 基准，
不能从节点自身 size 反推。正常流传父内容盒宽度，绝对定位传父 padding
box 宽度；测量、布局与文字快照必须一致，且不得改写宿主 cqw/cqh 上下文。

子节点边框与 padding 的外盒下限统一使用 `surface_box::minimum`；行列、
Stack 和绝对定位必须在尺寸约束求解前纳入该值，不能只在绘制后修补尺寸。
父空间不足应保留溢出；右/下锚点需依据最终外盒尺寸计算。

`surface_overflow.rs` 统一 overflow 白名单与跨轴计算值归一。矩形裁剪和
圆角 mask 不得各自判断原始 layout 字段；hidden/auto 触发另一轴默认
visible 计算成 auto，但不能把计算值支持当作滚动输入或滚动条支持。

祖先 overflow 裁剪统一经 `surface_clip::ancestors_with_border`，文字、
绘制与 Presentation 几何都必须提供同一计算字号上下文。只收缩子内容的
padding-box 裁剪，不把父背景边框也缩进去；圆角 mask 另同步内轮廓。

`surface_border.rs` 负责预计算边框轮廓的几何缓存与绘制，不承担布局。
边框必须保留透明中心，使用 alternate fill 的内外轮廓，不以背景覆盖
伪造镂空；几何按工厂缓存且只保留当前一份，错误路径不得污染裁剪栈。

`surface_box.rs` 统一边框宽度与内容内缩，测量、行列/Stack 和文字快照共用。
绝对定位包含块只扣边框、不扣 padding；文字重排反馈必须恢复外盒高度及
基线，不能将内容度量直接当外盒，或在每轮重复累加内缩。边框绘制与该尺寸
模块分开负责，不允许渲染模块另建一套布局公式。

`surface_round_clip.rs` 按渲染目标工厂缓存圆角 mask 几何；同工厂复用、换
工厂整批重建，不得使用其他工厂的几何。PaintFrame 为双轴裁剪容器维护
mask/opacity 的嵌套层顺序，背景圆角不能替代子树 mask。cqw/cqh 始终保留
宿主查询容器上下文，不允许被普通父 row/column 尺寸覆盖。

`surface_background.rs` 持有预计算的完整背景边界、祖先裁剪与椭圆半径。
百分比使用本节点两轴，em 使用 NativeMeasure 已计算字号；超大半径等比
缩减。圆角背景不可通过裁短矩形后重算半径模拟，也不能隐式裁剪普通子内容。
绘制配对恢复轴向裁剪；圆角子树 overflow 裁剪须作为独立盒模型责任实现。

Surface opacity 必须作为子树合成层实现，不得简化为逐图元乘 alpha。
PaintFrame 提交时预计算子树可见范围和有界层指令；opacity 0 跳过子树，
opacity 1 不建层。绘制栈异常也要释放剩余层；组范围保留可见溢出，不能
把父盒子误当作强制裁剪。合成层资源开销需在 N11 实际资源测试中验证。

`surface_color.rs` 只解析有界纯色，不引入 CSS 执行器/网络资源或主题查找。
`surface_paint_frame.rs` 在布局提交时预计算背景/文字有序指令，保留父子及
兄弟顺序和祖先裁剪；Presentation 将绘制指令与几何/文字一起原子替换。
绘制阶段复用画刷并恢复原颜色，不重新解析样式或创建 DirectWrite 布局。

`desktop/application_surface_remount.rs` 负责显式候选的一次性消费、设备授权
和容量检查；只调用 Request::recover，不得在恢复失败后回退创建 Art。
确认界面必须在模态前后验证同一候选和服务，应用层再复核；仅结构相等的
新候选不能被旧确认消费。新建与恢复共用 application_surface_start::report。

`desktop/device_authorization_recovery.rs` 仅负责原生文件选择与记录菜单，
不读盘、不发网络请求。`composition_relay_recovery_file.rs` 在 Sessions
统一容量/等待/退出回收内后台读取，取消不删除源文件。
`application_surface_recovery.rs` 负责记录与当前服务/设备授权匹配及只读
查询结果交付；磁盘 unknown 不因未找到候选而清除，候选不自动重新挂载。

`surface_recovery_record.rs` 定义无凭据的本地恢复提示，不提供从磁盘构造
Binding/Owner 的入口；格式有版本、字段和大小上限。`surface_recovery_store.rs`
只增写固定槽位，禁止静默覆盖、淘汰或删除证据。应用退出由
`desktop/application_surface_journal.rs` 采集最新 owner 元数据并后台落盘；
正常退出快照不等于崩溃恢复日志，磁盘提示必须重新认证和查询才能使用。

`desktop/surface_failed_admission.rs` 独立持有未建窗附件的生命周期状态。
原挂载 Outcome 与停用 owner 分离，不覆盖原确认绑定；unknown/重复/身份
不一致记录不能生成第二个请求 owner。在途不可取出，回交按 Activation
合同验证；停用失败不重试。退出宽限同时等待挂载线程，但不临时建窗。

`desktop/application_surface_shutdown.rs` 在统一撤权前关闭 Surface 窗口并
复用一次性停用调度；只等待生命周期任务，固定 2 秒绝对截止，不重试 unknown。
消息泵必须保留 WM_QUIT，并在超时/错误后继续既有撤权/drain/附件结算。
优雅等待截止不代表驱动或取消回收的整体耗时上限。

`desktop/surface_windows_shutdown.rs` 在后台回收后执行无网络结算，回交匹配
owner 并报告未确认附件；不可调用 active/inactive 调度或消费未匹配结果。
Sessions::finished 仅说明线程回收；surface_cleanup_pending 另计未交付的
挂载、恢复及生命周期结果，不能将二者混为附件清理完成。此门禁不持久化。

本地 Surface 窗口关闭不是删除工作区节点：授权仍有效且状态已知时，应用只
调度一次 inactive，上报成功后保留附件，不隐式 disposed。active 尚在后台
则先等待回交再停用；unknown/撤权不猜版本、不自动重发。应用退出已复用
此路径进行有界停用，恢复持久化和真实服务退出验收仍待完成。

Live 与 Surface 共用 63 个应用等待句柄预算。保留原 Live 最大 16 路能力，
但组合场景须按最坏句柄数检查；Live 启动、Art 发起及挂载接纳均检查，
不允许接纳后才因追加撤权 HANDLE 超限导致消息循环退出。

`device_authorization_surface.rs` 只收集用户明确输入的 Art ID，绑定当前窗口
UnitId 并排队；取消和模态期间服务变化不提交。`application_surface_start.rs`
在有效租约及容量/节点检查后启动 Temporary 挂载；宿主报告只声明实际静态
容器/文字能力，不声明输入、滚动、图片、脚本或 Shader。网络不进入菜单回调。

应用 Surface 准入模块还负责调度 active 作业：窗口 owner 移交到现有 Sessions
后台任务，单独申请串行请求租约；完成后按附件回交窗口。失败记录禁止自动
重发，无法匹配/恢复场景结果必须退回原集合。仅成功确认 active 且当前授权
有效时显示，重复结算不重复 ShowWindow；恢复已确认 active 可直接通过门禁。

`surface_activation.rs` 负责一次 active 事务的 Owner 移交。移出期间只保留
确认绑定用于身份/授权，不允许显示；回交必须匹配附件身份和 generation，
仅接受原确认状态或下一版本 active。拒绝时返还传入 Owner，unknown 不自动
重发。授权窗口关闭但 Owner 尚在后台时不能提前从集合收割，防止丢失回交目标。

Binding 保留服务端确认的 Lifecycle 与版本；Owner::confirmed_active 必须同时
排除 unknown 和 disposed。AuthorizedWindow::show 同时检查当前授权及已确认
active，不能把 mounted、版本递增或请求已发出当作激活成功。

`application_surface_admission.rs` 在应用结算点收割挂载完成结果，按当前选择
服务申请新租约，再交给 Windows::admit。结果交付容量不足时原样放回 Sessions；
失败/未知/重复附件结果保留在有界失败记录中，不能每轮重新建窗或丢失证据。
接纳成功只创建隐藏窗口，正式 active 生命周期尚未完成时不能自动显示。

`desktop/surface_windows.rs` 管理最多 4 个授权窗口，关闭后仍占容量直到收割
附件 owner；容量失败归还原窗口。仅存活窗口贡献等待句柄和最近过期截止，
避免已触发的终态事件造成忙循环。Application 在消息等待前结算授权、登记
这些句柄/截止，并在退出时关闭本地窗口；挂载结果已由应用结算点接纳。

`application_live_wait::next_until` 支持绝对单调截止时间；到期返回 WM_NULL 结算，
优先于已排队输入，但不消费输入。系统唤醒不能延长截止，亚毫秒剩余向上
取整；无截止继续无限等待。调用方必须结算后移除失效截止，不能反复提交
同一已过期时刻造成忙循环；授权墙钟截止应在应用层转换并重新复核。

`desktop/surface_authorized_window.rs` 组合挂载准入、隐藏窗口、独立租约、撤权
事件及附件 Owner。调用方必须把事件加入应用等待集合、按截止时间唤醒并
调用 refresh；本模块不自行轮询。失效只关闭本地窗口，不伪造 disposed，
生命周期 owner 与租约可移交后台事务，原 Outcome 的恢复证据不被消费。

`device_authorization_watch.rs` 为租约提供独立惰性撤权事件；重复订阅共享一个
手动重置 HANDLE，终态不可重置，注册与撤销共锁避免漏唤醒。不占网络取消
的 native_wake 槽，不创建轮询线程。事件只表示撤销，过期仍由调用方结合
expires_at_ms 安排等待并调用 validate，不能把无信号当成仍有有效授权。

`surface_mount_admission.rs` 提供 Outcome::snapshot_for，交付前检查当前租约、
origin/device/node、已确认绑定、快照身份及声明式运行时。只借用挂载结果，
失败不得丢弃恢复证据或自动重发 POST。Lease::validate 不暴露 token；这是
时点检查，不能替代窗口存活期间的撤销/过期处理或附件生命周期确认。

AdmittedSnapshot 将 Scene 和同一响应的 revision 一并交付，字段私有且不允许
调用方自行构造。AuthorizedWindow 建窗成功时保留 snapshot_revision；生命周期
ACK、关闭、撤权不得将其改为 lifecycleRevision 或 generation。未来更新场景时
必须在呈现准备成功后一起提交版本，事件 baseRevision 只能取该场景版本。

`desktop/surface_window.rs` 持有隐藏创建的 Surface HWND 和 Host，连接 WM_PAINT、
尺寸/DPI 失效与关闭清理。调用 Host 时暂时取出 owner，防止同步重入借用冲突；
重入关闭后不得恢复 Host。窗口显示必须显式请求，场景更新成功后才请求重绘。
应用仍需接入授权后的 Surface 入口；本模块不能绕过绑定/生命周期协议。

Surface 的 DPI 消息应用系统建议矩形，使用 checked_sub 和尺寸上限避免
无效尺寸，保持非激活及 Z 序。Host 的画刷与绘制目标一起缓存/释放，不在
每次 paint 分配画刷；尺寸/DPI 或设备错误后的目标重建也必须重建画刷。

`desktop/surface_host.rs` 负责声明式文字状态到 HWND 客户区的 Direct2D 绘制，
按物理客户区和窗口 DPI 计算逻辑尺寸；相同视口复用布局，变化时重排并重建
目标。Host 限定窗口线程，调用方必须在 HWND 销毁前释放，管理消息循环与
BeginPaint/EndPaint（由 surface_window 实现）。绘制错误释放目标，不自行定时
重试；尚未接通应用入口。

`surface_geometry.rs` 从成功 Tree 保存独立节点几何：ID、类型、父索引、位置、
尺寸及自身矩形与祖先裁剪的交集。保留零面积和被裁剪节点，不能按可见性
删除布局项。Presentation 同时更新 Geometry 与 TextFrame，不能混用不同
准备轮次的结果；当前坐标不含滚动变换，也不等于事件命中或可操作权限。

`surface_presentation.rs` 持有 Scene、Geometry 与独立 TextFrame；候选场景及布局准备成功
后一起替换，重排失败保留旧快照。TextItem 仅保存节点 ID，不借用场景节点；
身份校验仍在构建时完成。窗口重绘应复用快照，不在每帧创建测量 owner。
它目前负责文字呈现状态，不代表资源控件或生产窗口已经接通。

`surface_clip.rs` 按布局树父索引累计根边界与逐轴祖先裁剪；scroll 裁剪双轴，
普通容器遵循 overflowX/overflowY 的 hidden/auto。裁剪仅限制绘制，不能删除
测量项或修改布局尺寸；完全不可见的非零面积文字仍参与高度反馈。

`surface_text_pixels_tests.rs` 使用 WIC 离屏位图验证真实 Direct2D 文字像素、
祖先裁剪和裁剪栈恢复；以同一 Layout 的无祖先裁剪绘制作为正对照，不对
特定字体的精确抗锯齿像素作快照断言。COM 与绘图资源在测试线程内释放。

`surface_reflow.rs` 负责文字最终高度/基线的有界反馈，最多 8 轮；稳定后一次
交付 PreparedScene（Tree + TextFrame），不交付未收敛的部分结果。每次
prepare 清理上一轮反馈，保留初始宽度基准；不得将断行后宽度反复作为新
flex basis。上下文/样式变化仍应重建 NativeMeasure。

`surface_text_frame.rs` 从同场景 Tree 的最终矩形创建文字绘制快照，直接使用
已解析样式和分配宽度，不重新采用节点显式 width。保留 Layout、位置、分配
尺寸和溢出；零面积不建布局，正小数尺寸不取整。draw 成对管理文字矩形裁剪，
快照包含祖先裁剪交集，调用方负责颜色、滚动变换与 BeginDraw/EndDraw。

Measure::font_size 提供节点已解析字号，Tree 经 arrange_with_fonts 交给
容器/stack/定位模块。父节点 padding/gap 使用父字号，子项宽高/偏移使用
各自字号，rem 与宿主查询容器不改写；不得再次解析 fontSize 相对单位。
纯几何调用方可使用 arrange 的显式单字体上下文，原生 owner 应使用字体表。

`surface_native_measure.rs` 是单场景原生测量 owner，组合 Styles/DirectWrite
与容器固有尺寸预遍历，实现 Tree 的 Measure 合同。仅支持当前已实现的
文字及容器，不为图片/控件返回占位假尺寸。容器缓存属于本次场景与测量
上下文，视口/样式变化应重建；分配后重排通过 prepare 执行有界高度反馈。

`surface_text_styles.rs` 预计算单场景的继承文字样式，表项绑定节点引用而非
只匹配 ID；新场景必须重建。layout_text 消费已解析样式，不能再调用会
解析相对单位的 layout_node，否则 em/% 会重复换算。字重/行高/字号边界
在构建时验证，无跨场景全局样式缓存。

`surface_layout_tree.rs` 负责有界节点树布局遍历，调用 Measure 合同获取固有
尺寸，再接入容器几何并累计场景坐标。输出保留父索引和深度优先顺序，
失败不返回部分树；测量实现的缓存/副作用不由此回滚。字体继承预计算、
容器固有尺寸和宽度分配后的文字重排仍由测量/渲染 owner 完成。

`surface_stack.rs` 是容器的共享区域重叠布局子模块，复用尺寸/交叉轴及定位
合同，保持场景输出顺序。普通子项不参与 flex grow/gap，绝对定位继续退出
共享区域。当前按给定内容区域布局；复杂 grid 轨道和 justify 分布尚未实现，
非默认 justify 显式拒绝，不能据此声明完整浏览器 grid 兼容。

`surface_position.rs` 是容器布局的定位子模块。relative 只改变输出坐标，
不改变兄弟项占位；absolute 不进入主轴/交叉轴分配，结果仍保留原场景顺序。
当前 absolute 要求直接父级为显式定位包含块、两个轴都有锚点；祖先包含块
搜索与 static-position 未实现时返回 Unsupported，不能猜测原点。

`surface_container.rs` 将 Scene 的 row/column/view/scroll 子项接到主轴与
交叉轴分配器。测量结果必须逐项匹配节点 ID，调用方负责重新测量过期尺寸。
百分比尺寸使用父内容轴，cqw/cqh 保留宿主查询容器，不被每层普通容器覆盖。
绝对/相对定位、stack 和列基线尚未实现时显式拒绝，不能静默降级。

`surface_cross_axis.rs` 处理 stretch/start/center/end/baseline 交叉轴几何。
显式尺寸不因 stretch 被覆盖；基线由测量 owner 传入，无基线盒子以底边
合成。输出保留负偏移及前后溢出，裁剪/滚动由上层决定，不在此修改尺寸
掩盖溢出。该模块尚不等于节点树布局或已接通窗口渲染。

Surface text 的 line_height 保存计算后的像素行高，节点 lineHeight 的 em/%
相对该节点解析后的字号，继承值不重复按子字号缩放。DirectWrite 布局读取
真实行度量，平分新增行距并设置统一行高；baseline() 供后续交叉轴对齐。
行度量数组先检查上限再分配，不能按不受限的 native count 分配内存。

`surface_text_node.rs` 将声明式 text 节点接到 DirectWrite，负责字体尺寸、
字重、whiteSpace 和 textAlign 的白名单映射。父样式显式传入，em 字号以
父字体为基准，不依赖过期上下文。它不负责行高、文本选择或全树样式级联；
其他节点不得自动按 text 渲染，原生字体尺寸限制仍由 surface_text 检查。

`surface_text.rs` 使用 DirectWrite 进行 Surface 文字测量，Layout 保存原生
布局并以同一对象调用 Direct2D 绘制。Engine/布局由渲染 owner 持有，无全局
缓存或定时器；调用方负责目标线程及 BeginDraw/EndDraw。UTF-16 输入有界，
字体与尺寸先校验，normal/nowrap 的空白折叠不得误折叠 NBSP 或破坏代理对。

`surface_flow.rs` 负责行/列主轴数值分配，输入必须已完成长度解析和固有尺寸
测量。伸缩在 min/max 边界冻结后重新分配，不叠加上一轮临时尺寸；返回
offset/size/overflow，不创建窗口或像素。它不替代节点树布局、交叉轴、
文本测量或完整 CSS flex/grid 语义，布局 owner 仍须完成这些环节。

`surface_length.rs` 负责声明式长度白名单和原生数值求解，包括字体/视口/
容器单位及主干受限 min/calc 结构，不是通用 CSS 表达式引擎。Scene Node
通过 layout_length/style_length 进入该合同。auto 和缺少 fraction 分配的
fr 不猜测尺寸，由布局 owner 决定；上下文和计算结果必须有限且有界。

`surface_scene.rs` 定义声明式节点树的纯数据合同，覆盖主干 18 种节点类型。
解析限制 1 MiB、2048 节点、32 层，拒绝重复 ID、无效事件标识和非对象属性；
只提供只读访问。`MountedSurface::declarative_scene` 是挂载响应到模型的入口，
拒绝其他 runtime。属性仍由具体原生节点实现验证；解析支持不是绘制、输入、
资源访问或能力声明，不能据此在 Report 中宣称全部节点已经实现。

`surface_recovery_runtime.rs` 拥有只读候选查询后台任务，配置准备和 HTTP 均在
后台执行；`desktop/composition_relay_recovery.rs` 负责统一等待、取消、结果
领取和退出回收。任务及未领取结果均计入四项总容量，撤权抑制旧候选交付。
查询结果保留原请求 Arc，但不能更新原挂载 unknown，也不自动发起 remount。

`surface_attach_runtime::Request` 区分创建和显式候选恢复，两者复用同一挂载
Job/应用集合，不另增线程池或等待句柄类别。恢复请求持有认证候选 Arc，
禁止失败后降级到创建；Outcome 继续保留请求上下文、确认绑定或 unknown。

`surface_recovery_http.rs` 只用设备认证 stream 查询当前 Art/节点候选，不读取
管理员实例列表，不重复创建。候选不携带伪造的 lifecycleRevision，也不证明
原请求成功；空结果不能清除 unknown。显式接受候选后的 remount 必须重新验证
服务/设备、实例/附件、generation、Art/runtime，随后才取得真实生命周期版本。
首次挂载与候选恢复共享 `surface_attach_http::validate` 的 Art/runtime 合同。

`surface_attach_runtime.rs` 独占首次创建/挂载后台请求，与目录和观看共享四槽；
取消不代表远端回滚。Outcome 保留 origin/device/Art/node、确认绑定或 unknown，
禁止自动重发创建。`desktop/composition_relay_attach.rs` 接入统一等待、取消和
退出回收，未领取结果仍占集合容量；错误提示不消费恢复证据。取走结果不代表
重新取得授权，渲染和后续生命周期操作必须重新核验上下文。

`surface_host_capabilities.rs` 仅定义显式能力 wire 报告，不能据此宣称 runtime
已经实现。`surface_attach_http.rs` 负责认证首次挂载并校验响应 Art/runtime；
`surface_binding_http.rs` 复用快照身份验证。生产 report 须来自真实 renderer，
不能复制 Art 的 requiredCapabilities 或旧 WebView 全能力声明。

`desktop/composition_relay_surfaces.rs` 管理远程集合中的 Surface 事务与完成
结果，复用现有消息等待/退出 drain；容量包含未取走结果，错误提示不丢
owner。服务切换/取消清除成功场景但保留恢复证据。UI 取走 Outcome 后仍须
复核 origin/device/node/generation，不能把保留结果当作新授权。

`surface_lifecycle_runtime.rs` 独占一次后台事务的生命周期 owner，与目录/观看
共享任务槽，必须等待完成后 try_join 回收并保存 owner；启动失败同样返还。
取消不得抹掉 unknown，撤权后仅抑制成功场景交付，不回滚已确认的版本。
UI 不得忽略 StartError/Outcome 中的 owner 后自行伪造一个新的已知状态。

`surface_lifecycle_owner.rs` 持有单附件的已确认版本、unknown 和 disposed；
`surface_lifecycle_http.rs` 负责后台认证 transition/recover。请求失败不清除
unknown、不自动重放，恢复必须来自实际认证 remount；响应描述和版本须
精确一致，同代次恢复不得回退版本。UI 确认实际挂载后才能请求 active。

`surface_binding_http.rs` 是 Live 绑定恢复的 HTTP 子模块，复用 Control 的
固定服务认证交换，不另造凭据或重定向路径。remount 返回的 MountedSurface
只证明绑定/快照身份，scene/resources 必须另行验证；调用应放在后台 owner，
不得在窗口回调联网，也不得在尚未渲染时提前上报 active。

`surface_live_binding.rs` 是 Surface 挂载响应与 Live 观看目标之间的合同，
仅从固定 origin/授权设备响应选择当前节点附件；拒绝旧设备、歧义或 disposed
绑定。缓存不等于永久授权，构造目标前须复核上下文，服务端仍须实时校验。
不能用随机 Surface ID 接通观看 UI，也不能从目录反推 Surface 授权。

ViewerSession 的 `poll_update` 是原生 Host 的帧/业务单 consumer 接口，同锁
确认通知并读取最新业务 Arc；`poll_frame` 仅作为无需业务的兼容读法。不要
新增第二个 reset 同一事件的 consumer，也不要把业务更新塞入无界队列。
业务发布校验固定 Permit，暂停/关闭清理缓存；Host 读取业务也校验许可。

认证 attach 在 `live_relay_http.rs` 同时验证身份/成员和 Business 快照，
`live_relay_join.rs::Prepared` 传递该状态给控制 owner；不可用目录数据替代。
`live_relay_control_owner.rs` 在业务与外部 consumer 都成功后才提交业务和
可靠游标，失败关闭媒体。外部 consumer 仍须有界、无 UI 阻塞，不能忽略
自身负责的事件；内建观察/审计处理不替代后续 UI 状态交付。

`live_relay_trigger_registration.rs` 负责注册、条件和目标合同及有界快照，不
执行触发逻辑。`live_relay_viewer_business.rs` 拥有观察/能力/审计/注册的组合
状态，跨业务批次必须整体校验后提交，不能分别提交观察和审计。控制 owner
仍负责 envelope 与 session_state；业务状态成功后才可推进游标。

`live_relay_observation_summary.rs` 负责八种观察能力的严格报告及状态汇总，
不将缺失报告视为空能力，也不从置信度推测能力。汇总借用已有错误原因，
优先级与主干一致；不改变 Observation 验证和单 ID 序号规则。

`live_relay_trigger_audit.rs` 仅镜像 Loom 触发审计，不执行 Surface action。
同键 upsert 的预留/最终结果规则必须同时用于快照和增量；最多 256 项，
批次失败不提交部分审计。外层业务事务须在其副本上调用，不能先提交审计
后再校验观察状态。合同与 FIFO/原子性测试在同名 `_tests.rs`。

`live_relay_observation.rs` 维护纯观察 wire 类型与安全校验，不访问 UIA 或窗口；
`live_relay_observation_store.rs` 维护有界快照/增量状态。批次使用 Arc 映射副本
原子提交，不复制大观察值，不把观察 sequence 与控制 envelope sequence 混用。
外层仍负责来源认证、session/epoch 和完整消息分流；当前尚未接入业务 consumer。

目录刷新命令属于 `desktop/device_authorization_ui.rs` 的有界请求，
`application_authorization.rs` 在 UI 回调外校验服务/租约并提交任务；
`application_remote_live.rs` 回收目录结果，只向当前 origin 对应共享视图
交付元数据。切换/清除/重新授权必须同步取消查询、清空目录；退出同样清空。
视图不持有凭据，目录不是加入授权，选择后仍须重新申请有效租约。

`live_relay_catalog_runtime.rs` 负责目录后台任务生命周期，不持有 UI。与控制
Runtime 共用四槽位，直到 join 才释放；结果绑定 origin，回收前撤权则丢弃。
调用者必须把完成句柄纳入消息等待并显式回收，不能依赖 Drop 在 UI 阻塞。
真实 HTTPS、取消、迟到结果和跨任务容量测试位于
`live_relay_catalog_runtime_tests.rs`。目录 Job 由远程 Sessions 集合持有，
与观看共用四项容量；poll_catalog 保存至多一份目录，take_catalog 转移给
未来选择界面。Application 已轮询完成/错误并等待句柄，退出 drain 统一
取消回收。服务切换须调用 cancel_discovery；选择目录不能替代新租约授权。

`live_relay_catalog.rs` 负责认证会话选择目录，返回身份、revision 和当前成员
标记，不暴露 token 或观察/触发业务快照。全量校验并拒绝重复 ID/关闭/损坏
条目，最多 512 项且沿用 1 MiB 响应上限。选择后必须填 Target.selected，
由 prepare_selected_viewer 在加入前校验 source/epoch；不能把 selected 当作
expected（续签必须已有成员）或只按 session ID 重新选择不同来源。

`desktop/composition_relay_sessions.rs` 拥有合计最多四个观看/目录任务，包括停止未回收项；
先检查关闭/容量并预留存储，再启动任务。Application 持有该集合，统一轮询
终态和等待句柄，退出交给 application_live_shutdown 在泵消息期间回收。
每个远程会话最多两个等待句柄（Ready+完成或帧+完成）；当前总上界 63，
编译期断言防止越过消息等待容量。不要在回调或普通 Vec 删除中隐式 join。

`desktop/composition_relay_session.rs` 配对控制 Runtime 和 RelayWindow，属于 UI
线程 owner。Ready 后才创建窗口，show 只记录明确显示意图；wait_handles 提供
就绪、完成及帧句柄，poll 在真实 join 后只交付一次 Outcome。用户关窗取消
任务，媒体失效关窗则等待原始终态，不能抢先取消把 DeviceRevoked 改写为
Stopped。窗口构造与撤权竞态同样保留后台原因。Drop 只作阻塞兜底，上层必须
等待并 poll 回收。跨真实 HTTPS/隐藏 HWND 的测试放 live_relay_desktop_session_tests。

`desktop/composition_relay_window.rs` 拥有远程观看 HWND 和 Relay Host，不拥有
后台线程。创建默认隐藏并启用系统捕获排除；应用等待 change_handle 后调用
refresh。回调只能绘制/失效/释放显示，不做网络或 join。Host 借出期间发生
重入关闭不得重新挂回；COM 资源释放后须重新校验 HWND 归属，防止句柄复用。
隐藏窗口消息/撤权/重入测试放 composition_relay_window_tests。

`desktop/composition_relay.rs` 负责远程 PBGRA 的 Direct2D 位图及 DirectComposition
呈现，`composition_relay_host.rs` 负责单窗口固定许可、帧事件消费和显示资源
生命周期。不得把网络/解码移到 Host；同尺寸新帧复用位图，普通重绘不重新
上传，上传后释放 CPU 帧预算。Host 必须先于 HWND 销毁；窗口关闭时上层会话
仍须停止后台 Runtime。真实隐藏 HWND/GPU 读回测试放 composition_relay_tests。

`live_relay_frame_notice.rs` 仅拥有帧/许可变化的 Win32 合并事件，不拥有线程、
定时器或消息队列。ViewerSession 在状态锁内设置/复位事件，单个 UI consumer
通过 change_handle 等待，再用 poll_frame 取得最新帧；暂停/关闭返回错误时
必须撤下旧显示资源。不能由多个窗口竞争同一复位事件或把 borrowed handle
保留到 Session 析构之后。`live_relay_frame_notice_tests.rs` 验证合并及复位竞态。

`live_relay_media_completion.rs` 是控制 Runtime 的媒体终态通知模块。媒体网络
线程先保存具体退出原因，再撤销租约并唤醒控制 I/O；不得先取消再写结果，
否则消费者可能只看到 Stopped。通知必须有界且不依赖 UI 或线程 join。控制
线程被唤醒后读取终态并回收媒体 Runtime；显式停止先发生时保持 Stopped。
真实终态/在途 HTTP 测试位于 `live_relay_media_terminal_tests.rs`。

`native/src/live_relay_authorized_media.rs` 是授权租约的媒体启动子模块：只允许
绑定到该租约的 ViewerSession，使用 Session 的固定 origin 凭据和已认证
Client 的 TLS/地址，不把 token 或 Prepared 请求送入 UI。控制 Runtime 的
viewer 启动入口在首批重放后启动媒体 Runtime，并在控制线程负责其停止/join；
Ready 只携带媒体访问入口、许可与序列基线，不代表首帧已到达。跨链测试放在
`live_relay_playback_tests.rs`，覆盖真实 HTTPS/WSS 到像素及撤销回收。

Live 控制运行时提供 `start_configured`：前台只捕获 Config，租约在后台检查
origin/过期后复用 `Config::prepare_client` 加载 CA 和解析 DNS，准备后再次
检查授权。授权任务也复用该客户端准备函数，再按需打开 Store；Live 不能为了
复用授权配置而额外创建身份文件或重新签发会话。配置型端到端夹具放在
`live_relay_control_configured_tests.rs`，仅写入 linshi 下的测试 CA。

`native/src/live_relay_control_runtime.rs` 拥有加入准备、首批重放与持续控制轮询
线程；必须使用授权租约，不接受裸 token。业务 consumer 显式传入并在后台
串行运行，不能操作 UI。一次性 Ready 队列容量为 1，独立事件通知就绪，线程
句柄通知完成；应用需将两者纳入等待集。完成未回收任务仍占四任务全局上限。
退出通过租约撤销媒体和请求；UI 应等待完成后 try_join，不能依赖阻塞 Drop。

`native/src/device_authorization_lease.rs` 管理共享 Session 的后台租约；每个
串行请求链独占 Cancel，不能让并行连接共用其中的单个 socket 登记槽。
授权缓存持有 Grant，替换、取消、重新配对或关闭时同步撤销其全部租约。
租约最多绑定一个 ViewerSession；绑定与撤销共锁，终态 close 防止迟到激活。
每个 origin 最多 16 个活跃租约；不复制 token，不提供 Debug/Serialize。
缓存移除不等待线程 join，任务回收仍由运行时负责；最后持有者释放 Session
时擦除凭据。lease_tests 验证生命周期/竞态，lease_http_tests 验证在途取消。

`native/src/live_relay_control_owner.rs` 独占控制游标及媒体许可，负责完整批次
校验、业务消费、撤权与最终游标提交；`live_relay_control_state.rs` 只归约
session_state 的 revision、成员、可见性和控制者，不执行网络或 UI 操作。
正常状态进度不能通过重新 activate 清空帧缓存。全部非状态事件必须交给
显式 consumer，不能在网络层悄悄忽略。owner_tests 覆盖许可/批次生命周期，
control_http_tests 复用既有回环 TLS 夹具验证 401 和 404 关闭确认。

`native/src/live_relay_join.rs` 负责 Viewer 发现与加入准备的认证请求顺序，
包括 actor 专属游标恢复、序列溢出检查和续签成员策略；返回 Prepared 不等于
控制历史已应用或媒体已获许可。`live_relay_resume.rs` 拥有 resume envelope
及成功响应验证，不修改调用方 baseline，不从 409 正文猜序列。两者作为
live_relay_http 子模块复用同一个传输与凭据边界。纯协议测试放 join_tests，
真实 HTTPS 串联测试放 join_http_tests；不得让测试中的固定序列代替恢复流程。

`native/src/live_relay_http.rs` 负责已认证 Live snapshot/attach/events/terminal
请求合同，借用已签发 Session 并生成每请求 nonce，不管理 UI 或激活媒体许可。
复用 `device_session_client::exchange` 的有界固定 origin HTTPS，禁止另造
TLS 实现或接受任意 URL/认证头。Device 401 必须在错误体读取前保留其认证
终态；404 不等于已关闭。`live_relay_http_tests.rs` 复用真实 loopback TLS
夹具验证线协议。`ControlCursor` 持续流拒绝 reset；bootstrap 的历史截断
容忍仅限于已验证本次加入事件之前，不能共用为跳过撤权的捷径。

`native/src/loom_socket_wait.rs` 只拥有 Windows socket 网络就绪和独立取消事件。
WSAEventSelect 建立非阻塞事件关联，WSAEnumNetworkEvents 原子复位；不得用
周期轮询或副本 socket shutdown 替代取消唤醒。`loom_bridge_stream.rs` 保留
标准 Read/Write 外观、绝对截止与永久取消语义；流独占 socket，析构顺序必须
先关闭 socket 再释放网络事件。Interrupt 只持有取消事件，不持有可读写 socket。
Windows 背压/EOF 回归位于 `loom_bridge_stream_windows_tests.rs`。

`desktop/device_authorization_ui.rs` 拥有无凭据的共享展示状态、单槽菜单请求和
模态选择事务；dialog/template 只处理 Win32 地址输入。网络与身份文件不得在
菜单回调中读取。`application_authorization.rs` 负责请求消费与安全状态发布，
授权 Owner 由 Application 独占；完成句柄进入现有消息等待集，退出先封闭入口
再取消并泵消息回收。`application_window_state.rs` 统一绑定新窗口共享状态。
本地授权缓存清除不等于服务端撤销，也不能替代既有媒体许可的显式失效流程。

`device_authorization_config.rs` 拥有明确 origin、原生 profile 与 TLS 环境快照，
构造时不得读文件/做 DNS。由 Runtime::Configured 在后台 prepare，复用同一
授权任务名额与取消 owner，不另造准备线程。默认身份与主干隔离，CA scope
不得冒充服务选择；解析、信任、会话和文件提交仍交给各自模块执行。

`loom_address_resolver.rs` 拥有 Windows GetAddrInfoExW 的稳定操作内存、事件和
结果链；取消后必须等用户完成通知才能释放，不能用游离 DNS 线程伪造超时。
`device_request_cancel` 的原生 wake 只供单个串行解析阶段使用，事件必须由
Arc<OwnedHandle> 保活。解析不授予 TLS 信任，后续连接仍按原始 hostname 验证。

`device_authorization_owner.rs` 拥有按 origin 的授权任务集合和 Session 缓存。
完成事件不得携带 token，撤销后的旧任务必须永久丢弃，不能因同 origin 新任务
启动而复活。配对成功即使 ID 未变也撤销旧缓存；失败保留原缓存。该 owner 的
缓存撤销不自动等同于媒体许可撤销，应用层必须显式联动并泵消息完成任务回收。

`device_authorization_runtime.rs` 拥有设备授权线程、未回收任务名额和结果发布。
Task 明确区分配对/批准/续签，取消后的结果不得交付；名额必须覆盖未回收结果，
不能在线程返回时提前释放。UI 借用完成句柄并 try_join，不在回调中 Drop 活跃
任务。对应 runtime 测试验证名额/退出；HTTP 子测试组合真实身份文件与 TLS。

`device_approval.rs` 只编排首次授权的批准等待，不能被普通续签隐式调用；
外层绝对期限须传到网络 I/O，而非只在轮询间隙检查。`device_request_cancel.rs`
拥有单个串行任务的 socket interrupt 与 Condvar，不可被多个并发任务共享使用。
每轮请求结束须释放当前 interrupt，取消必须覆盖等待和握手登记竞争。
`device_approval_http_tests.rs` 复用真实 TLS 夹具验证批准、失败、续签和取消。

`device_pairing.rs` 拥有显式注册请求、返回公钥关联校验及 Store 提交编排。
不生成替代密钥、不批准设备或申请 token；注册与默认绑定保存分别暴露，连接
专属关联不得覆盖默认身份。`device_pairing_tests.rs` 验证协议边界；
`device_pairing_http_tests.rs` 作为 HTTPS 测试的子模块复用 TLS 夹具，并只在
linshi 唯一目录验证真实落盘。上层负责成功后的旧授权撤销与批准轮询。

`device_session_client.rs` 拥有固定 origin 的真实 HTTPS 单次会话申请、响应限额
和请求总期限；只在后台调用，外层登记 interrupt。`loom_http_transport.rs`
仅将已验证 TLS 流适配给精确锁定的 ureq，禁止代理、重定向、二次连接和连接池；
其 unversioned API 升级必须重验真实网络夹具。地址解析、配对批准与 UI owner
不属于该适配器，不能将其误当作完整授权运行时。

`device_session_attempt.rs` 拥有单次 challenge/签发协议和 origin/期限绑定的
Session；消费 Attempt/SignedAttempt，禁止在此层自动重新配对或隐式重试。
`device_session_attempt_tests.rs` 覆盖主干签名兼容及失败边界。HTTP 传输必须
另行提供无重定向 TLS、有界读取与取消，不得把协议解析器作为来源认证器。

`device_identity_store.rs` 拥有显式目录、合作实例 lock、Loaded 快照与迁移/绑定
事务；`device_identity_file.rs` 只提交已保护文档的独占暂存文件，不提供通用覆盖
或伪 CAS。读取与初次创建分开，续签不能创建替代身份。失败不修改调用方 Loaded，
不得自动信任崩溃遗留 tmp；测试只写 linshi 独立目录。

`device_key_protection.rs` 仅拥有 Windows DPAPI 信封与 OS 缓冲清理；
`device_identity.rs` 拥有有界身份解析、seed、公私钥校验和保护文档，不隐式重建
损坏身份或执行配对；`device_session_proof.rs` 校验 challenge 并生成主干兼容签名。
文件持久化与网络授权由后续 owner 负责。不得记录私钥或将身份类型派生 Debug；
seed 清零边界不包含调用方的原始 legacy JSON 或密码库内部副本。

`live_relay_viewer_runtime.rs` 拥有固定 Permit 的后台网络任务和 Connection/Worker
生命周期。最多八个网络任务；stop 先撤销发布再取消 socket，线程内 join codec。
Connection::connect_cancellable 在握手前登记 interrupt；登记/取消竞争必须保持
先置停止标志的顺序。UI 借用完成句柄并 try_join，不在回调中 Drop 活跃任务。
此 owner 不获取凭据、不执行 bootstrap、不自动授权，不能用它绕过控制流。

`native/src/loom_tls_trust.rs` 拥有操作员 CA 的配置读取、严格 PEM 限额与 origin
隔离快照。公共/平台根由配置 owner 提供，清单不得注入新的 CA；错误配置禁止
静默回退。`loom_media_target::local_viewer_with_trust` 只取得自身 origin 的根。
新配置须显式构造新快照，不在网络或渲染热路径读取文件；测试采用独立临时证书。
默认 Trust::load_default 使用锁定的 webpki-roots 编译期 Mozilla 根，再叠加操作员
CA；调用方仍可显式注入根用于隔离环境。不能把该入口称为 Windows 系统根加载。

`loom_discovery::Config::load_media` 复用清单路径/读取优先级，不继承 hook bridge
的开关或 WS override；`loom_discovery_manifest::parse_media` 与桥接解析共用基础
验证，不要求 hookBridge。`loom_media_target.rs` 拥有清单 HTTP origin 与私有本机
Bearer，仅允许 loopback local_viewer；远端必须交给独立 Device 授权 owner。
localhost 仍受 transport 的回环地址检查约束，清单不能携带新的 CA 信任。

`native/src/live_relay_media_endpoint.rs` 拥有固定 origin、TLS 信任配置与 viewer
媒体请求构造；凭据申请/续签不属于它。token 不得写入 URL/Debug/错误，Device
nonce 每请求更新，明文仅允许 loopback IP 或 localhost，且连接地址必须为回环。
对应 `_tests.rs` 验证请求和边界。
`live_relay_media_transport.rs` 拥有真实 socket、实际协商 Profile、每连接 Liveness
与 I/O 截止时间；复用 BridgeStream 而不复制底层取消机制。解析地址由可信 owner
提供，TLS 按原始 hostname 验证。此连接只能在后台使用，不在 UI 回调中握手。

`native/src/live_relay_media_liveness.rs` 拥有单连接 Ping 序号、在途探测和 RTT，
由 socket service 显式驱动；重连必须新建状态，不创建额外定时线程。探测超时不
承担授权撤销职责；序号耗尽不能回绕。状态测试与模块同文件维护。

`native/src/live_relay_media_socket.rs` 服务已准入 WebSocket：限制消息/帧大小、
接管 Binary 缓冲、提交 worker、回送控制意图及处理精确 Device 撤销 Close。
连接/TLS/凭据/子协议验证和读写超时由 transport owner 在调用前完成；不得从
媒体包推断 Authorization 或 Profile。Frame::from_message 保留原 Vec，不复制
payload，按实际 capacity 计租约；WebSocket 读取时的临时分配不在此租约内。

`native/src/live_relay_media_worker.rs` 拥有真实媒体线程、两项输入/四项回复队列和
最多八个 worker 名额；线程内创建 Pipeline，退出时按固定许可暂停会话。停止
标志不排队，Wake 只唤醒阻塞接收；idle 以 100ms 超时兜底观察会话取消，非忙循环。
UI 必须等待借用的线程完成句柄并 try_join，不能在回调中 Drop 活跃 worker；Drop
的阻塞 join 只是避免遗留线程的最后兜底。回复满载终止，不静默丢控制意图。

`native/src/live_relay_media_pipeline.rs` 是 codec 线程内的组合入口：固定 Permit、
Receiver、静态解码和 ViewerSession 提交。它持有 MTA apartment，必须先释放
decoder 再退出 COM；不能跨线程搬运。网络循环必须在取消唤醒/读取边界调用 check，
以便无新帧时也释放失效 codec；发送返回的控制意图前再次复核，不能只信旧结果。
测试位于 `live_relay_media_pipeline_tests.rs`；该模块不创建 socket 或 HWND。

`native/src/live_relay_viewer_session.rs` 拥有会话媒体发布 gate、授权代次及私有
ViewerPixels，固定 state→frames 锁序；锁内不得解码、做网络 I/O 或 join。控制
owner 为新 worker 取得 Permit，worker 保留该许可，不得在失效后自行重新取得。
提交时同锁核对 owner/代次/epoch/终态。激活必须在真实认证及 bootstrap 重放后
调用；本模块不执行认证。关闭清缓存，但已经移交给 UI 的像素须由 UI owner 撤下。
每代 Permit 的 stop 信号供 codec/worker 响应取消，不得替代同锁发布检查。

`native/src/live_relay_viewer_identity.rs` 校验已认证 HTTP 快照的会话/来源身份、
epoch、revision 和续期成员关系；`live_relay_viewer_bootstrap.rs` 查找本次加入事件
并验证连续后缀，返回加入之前的重放 cursor。测试在
`live_relay_viewer_identity_tests.rs`。解析不等于认证，输入须来自验证过来源与 Device
凭据的 HTTP transport；这些模块不存凭据、不授予 UI 或媒体权限。

`native/src/live_relay_control_events.rs` 拥有控制事件 envelope 与两阶段 cursor，
bootstrap 复用同一 wire 类型；prepare 不改变位置，业务应用成功后由会话 owner
同锁 commit。batch 绑定创建它的 cursor，不能跨 owner 或覆盖新位置。
`live_relay_viewer_terminal.rs` 只校验权威 member snapshot 的关闭事实，不将
HTTP 404/断网当作关闭；这些边界的测试同属 viewer identity 测试文件。

`native/src/live_mf.rs` 管理线程绑定的 COM/MF、系统同步 H264 MFT 激活及每进程
一个 decoder 名额；必须在普通 codec owner 线程销毁，不能交给 MF 工作队列或
进程静态析构。系统 transform 不得逃逸至 runtime 关闭之后。

`native/src/live_mf_format.rs` 拥有 H264/NV12 类型协商、颜色合同与启动流；配置
失败必须消费并释放整个 owner。`live_mf_layout.rs` 拥有存储/可见尺寸及 aperture
校验，不分配像素。

`native/src/live_mf_decode.rs` 拥有 codec 前 AU 校验、四项时间戳队列、取消和失败
销毁，`live_mf_samples.rs` 拥有 MF sample/buffer/event 的局部 COM 生命周期；
`live_mf_pixels.rs` 校验 NV12 stride 和长度并转换到共享预算的 Pixels。视频核心
不得依赖 desktop 私有 Image 类型，输出不得携带可逃逸的 MF 对象；UI 适配由后续
viewer owner 完成。输出读取每次最多三步，stream-change 必须重新协商。

`native/src/live_mf_sequence_tests.rs` 拥有显式合成视频序列门禁：有界读取本地
fixture、检查 IDR/delta 分布、逐帧内容身份及中途 IDR 重建。AUD 切分仅用于约定
夹具，禁止将它当作生产网络帧解析器；生成媒体放在 linshi，不引入产品 FFmpeg 依赖。

`native/src/live_h264.rs` 拥有 codec 前的 AU/NAL 顺序与限额检查；
`live_h264_bits.rs` 拥有固定容量 RBSP 位读取，`live_h264_sps.rs` 拥有 SPS/VUI
尺寸、参考图像和颜色合同。三者不创建 MF decoder、不解码像素；测试在
`live_h264_tests.rs`，后续 decoder 必须先消费这些检查而非只相信 wire 尺寸。

`native/src/live_relay_video_chain.rs` 拥有每连接的 H264 参考链、四项描述符队列、
关键帧请求去重及 sticky fallback；不持有 codec、不发送消息、不授予权限。测试在
`live_relay_video_chain_tests.rs`。

`native/src/live_relay_video_receiver.rs` 组合 Chain 与线程绑定的 MF decoder，负责
断链释放、IDR 重建、输出描述符匹配和一次性降级。测试在
`live_relay_video_receiver_tests.rs`。它不拥有授权、socket 或 UI：调用方须传入
会话 owner 的授权 epoch，不能取包内 epoch 代替授权；返回的控制意图及像素提交
前还须同锁复核会话状态。JPEG/raw 返回原 Frame，继续交给静态媒体解码入口。

`native/src/desktop/live_relay_image.rs` 拥有 relay 静态帧到 Image/Pixels 的转换；
JPEG 容器/尺寸准入由 `image.rs` 的专用入口在分配像素前完成。测试在
`live_relay_image_tests.rs`，使用内存 WIC 编码生成夹具，不读取用户图片或剪贴板。

`native/src/live_relay_viewer_frames.rs` 拥有单会话 viewer 的 epoch/frame cursor、
三帧缓存及关闭状态，测试在 `live_relay_viewer_frames_tests.rs`。提交和关闭共用
一把锁；不得另建一个独立 stop 状态而绕过同锁提交判定。通用 FrameStore 复用该
状态机，ViewerFrames 保存接收字节，ViewerPixels 保存已解码像素；不要复制两套
关闭/淘汰逻辑。两种缓存均不拥有授权，提交前仍由会话 owner 复核。

`native/src/live_relay_pixels.rs` 的 PixelFrame 绑定 raw sRGB 描述符和 Pixels 租约，
验证尺寸/长度，不重新分配。H264 Receiver 直接输出该类型，静态媒体通过
`desktop/live_relay_image::decode_pixels` 交付，缓存移交测试在
`live_relay_viewer_pixels_tests.rs`；桌面 Image 适配和实际呈现仍属于 UI owner。

`native/src/live_relay_receive.rs` 拥有已协商 profile 的单帧接收缓冲与字节租约；
`live_relay_receive_tests.rs` 验证分片、拒绝、取消和预算。transport 应在后台使用，
该模块不拥有网络连接、不授予权限、不验证压缩码流，也不自行维护帧队列。

`native/src/live_relay_frame.rs` 负责 NLLV v1 帧头和封装校验，测试在
`live_relay_frame_tests.rs`；不拥有网络、授权或媒体解码。头准入必须先于 body
分配，借用 Envelope 中的压缩 payload 仍是不可信数据。

`native/src/live_input_dpi_bridge.rs` 是输入坐标模块的私有混合 DPI 转换回退，只有
系统转换失败且同显示器 PM root 的条件成立才外推；不拥有输入授权、窗口或任务。
`live_input_mixed_dpi_tests.rs` 覆盖自有 mixed-hosting 父子窗口的控件外拖动。

`native/src/live_input_dpi_runtime_tests.rs` 拥有输入坐标的显式 DPI awareness 矩阵
门禁；只创建自有窗口，作用域恢复线程上下文，不更改用户显示缩放设置。

`native/src/live_input_winforms_tests.rs` 负责显式托管控件运行门禁，复用仓库 C#
fixture；只控制所创建子进程的窗口，按 PID 校验，输出置于 linshi，RAII 回收子进程。

`native/src/live_input_button.rs` 是输入 Backend 的私有补偿模块，负责 WinForms
push-button 类型/遮挡/DPI 检查及受限父通知；普通发送成功后才能完成补偿。
`live_input_button_runtime_tests.rs` 使用自建模拟类验证消息链，不替代真实 WinForms
运行验收。取消清理不得进入补偿逻辑。

`native/src/live_button_gesture.rs` 拥有遮挡按钮补偿的纯手势策略，不查询 HWND、
不授予权限、不发送消息；目标类型、DPI 阈值及父窗口通知属于 Win32 后端补偿模块。

`native/src/desktop/composition_live_input_runtime_tests.rs` 负责自建源窗口与 Live
之间的真实 WGC/输入/恢复联合门禁。帧读取 accessor 仅在测试构建启用，不向生产
代码暴露额外 GPU 帧引用，也不在生产路径引入像素读回。

`native/src/desktop/composition_live_window_input.rs` 拥有 Live 系统菜单、窗口键鼠
翻译、有界 UI 队列、捕获/失焦撤销和简短控制状态。回调只排队或发送 StopSignal，
不执行 Backend/join；Session 在回调外消费，Viewport 只提供成功呈现的几何。

`native/src/desktop/composition_live_session_input.rs` 拥有 Session 显式控制入口、
ACK/错误结算和输入 worker 回收；主 Session 延迟源恢复与最终完成，Application
等待/关闭模块纳入输入完成句柄。UI 回复用线程消息，不堆叠超过系统上限的等待句柄。

`native/src/live_input_worker.rs` 拥有输入线程、有界命令/回复、停止标志、通知事件与
join；Backend/Controller 在线程内创建。独立调用方等待回复事件和完成句柄，UI
模式通过线程消息接收回复通知，并等待完成句柄；关闭保留 owner 至 take_exit，
不能依赖 Drop 阻塞清理。

`native/src/live_input_backend.rs` 将 `live_input::Controller` 对接目标安全、路由、
坐标与 Win32 消息模块，保留最后有效鼠标位置用于取消。它不拥有 UI，也不创建
后台线程；调用方必须提供独立输入 worker。联合测试只向自建透明窗口发送消息。

`native/src/live_input_routing.rs` 管理显式 root 内的有界深层命中、源线程焦点选择
及已按下目标的保留/回退；其测试在 `_tests.rs`。路由不抢焦点、不扫描桌面、不修改
隐藏样式，结果交由 Destination 重新验证权限与身份。

`native/src/live_input_coordinates.rs` 拥有源客户区/物理 frame 裁剪到子目标客户区
及屏幕坐标的转换，以及线程 DPI 上下文恢复；测试在独立 `_tests.rs` 中。不得混用
此坐标转换与输入授权，也不得把 Viewport 帧像素直接作为目标客户区坐标。

`native/src/live_input_target.rs` 是 `live_window_target::input_target`，拥有绑定子
HWND 的原 root、进程句柄、身份与祖先关系验证；其测试只操作自有隐藏窗口。
`input_message::send_to/release_to` 消费此目标，不得绕过校验直接发送至缓存裸 HWND。

`native/src/live_input_message.rs` 是 `live_window_target::input_message`，负责有界
键鼠消息编码和绑定 root 的 Win32 投递；`live_input_message_tests.rs` 负责编码边界
及自有隐藏窗口的显式运行测试。它不负责用户授权、子控件路由或帧坐标转换。

本文件面向开发者和 AI 协作者，是文件归属、模块导航和结构变更的长期规范，
不是一次迁移的执行日志。工程流程见 [CONTRIBUTING](../../CONTRIBUTING.md)，
运行时设计见 [技术架构](../architecture/overview.md)。

`native/src/live_viewport.rs` 拥有 Live 的等比适配矩形与帧像素映射；绘制和后续输入
使用同一几何，不拥有输入授权或 HWND 坐标转换。`composition_live_host.rs` 拥有
成功提交呈现的序号/几何快照，暂停和关闭清空；pending 帧不得提前替换该快照。

`live_input_security.rs` 是 `live_window_target::input_security` 的安全预检入口，
`live_input_token.rs` 管理有界 token/SID 读取，`live_input_desktop.rs` 管理只读桌面
检查与 desktop 句柄。三者不拥有用户授权或输入发送，不把捕获 Target 自动升级为控制权。

`live_input.rs` 拥有显式启用、递增序号、有界 held 输入、手势目标和撤销清理报告；
`live_input_tests.rs` 使用受控 backend 验证超时/失败状态，不发送系统输入。平台 backend
负责目标安全、DPI 转换、超时及独立 cleanup release；不得在 UI 线程直接运行阻塞发送。

`native/src/long_capture.rs` 拥有长截图 BGRA 帧准入和已定位帧接口，
`long_capture_compose.rs` 将水平/垂直、正反向扩展规划成共享帧片段，保留先到帧的
已覆盖像素。先验证连续覆盖、横轴一致、整数范围及主干帧/输入/输出上限，再一次分配
输出；单帧/完全重复直接共享像素，按行检查取消。输出同样受全局像素预算约束，
不承诺能同时驻留两个上限图像。`long_capture_tests.rs` 验证双向/包含/重复、真实源偏移、
BGRA 保留、预算与取消。此层要求调用方提供可信位置，不猜测重叠、不发送滚轮，自动
近似匹配、固定栏处理、捕获循环和 UI 仍待迁移，不能把底层测试算作长截图用户功能完成。
`long_capture_match.rs` 提供等尺寸帧的精确重叠快速路径：逐线 BGRA 签名与 KMP
搜索双向重叠，多个候选返回歧义，唯一候选再比较原像素。签名与验证按行检查取消，
临时签名和前缀表总量小于 1 MiB，不复制图像；对应测试覆盖双轴正反位移及合成结果。
`long_capture_approx.rs` 拥有等尺寸 BGRA 帧的有界近似位移搜索：每个候选至多
16×16 点，唯一低误差候选再以至多 128×192 点复核；沿轴纹理不足或第二候选过近
不输出位移。选项限定搜索范围、最低重叠及 0..12 通道平均误差；逐采样行检查取消，
无图像副本或候选数组。采样结果不是逐像素证明，也不处理固定栏；相应测试验证
轻微变化、双轴双向、零位移、重复纹理、搜索边界及粗采样漏检后的密集拒绝。
`long_capture_fixed.rs` 检查两帧轴向首尾逐像素不变的连续区域，借用原帧内部区间
调用近似分析，不复制裁剪图。只有非零运动通过评分才返回首尾长度；全帧相同单独
返回 Unchanged，失败不授权裁边。`long_capture_fixed_tests.rs` 覆盖双轴双向、内部
区间边界、取消和内容不足。`stitch_fixed_edges` 在逻辑输入总量准入后逐帧复核首尾
固定区域，变化则返回 ChangedEdges；内容用已有片段计划合成，首尾各输出一次，
无需中间裁剪图。输入 origin 指内容起点，输出 origin 减去 leading，含溢出检查。
两帧静止边缘不是永久固定栏证明，跨帧确认仍由后续会话层负责；浮动、半透明或
变化的固定栏尚不支持。
`long_capture_session.rs` 串联精确优先、近似/固定栏分析与合成，拥有共享输入帧、
累计内容坐标、逻辑像素总量及固定栏证据。追加先验证再提交；重复帧不增长，失败和
取消不改变参考帧。固定栏需两次非零运动确认，后续每帧与首帧固定区域复核；未确认
返回 UnconfirmedEdges，不能静默裁边。无固定栏允许两帧合成，单帧共享原存储。
会话不负责输入 hook、捕获线程、滚动或 UI；对应测试覆盖确认、重复、回滚和往返合成。
`long_capture_job.rs` 将 Session 移交独立 worker，命令/回复槽各为 1，调用方消费
回复前不能提交下一命令。追加和合成均在后台完成，回复带帧数/已确认边界快照；
取消关闭两端通道并丢弃迟到输出，不等待工作线程，最终 Drop 取消并 join。
Busy/Stopped 返回时传入帧随命令释放，不排队；调用方应先检查 is_pending 再采集。
`long_capture_job_tests.rs` 验证单在途、后台合成、失败后继续及已完成结果取消丢弃。
SessionJob 可持有采集闭包，仅显式 capture 命令在 worker 调用；失败返回 CaptureFailed
而不改变会话，源缺失返回 Unavailable。`desktop/long_capture_source.rs` 将冻结物理
选区与首帧接入该接口，复用 `capture::region` 的区域 GDI 捕获，不先分配全屏图。
取消在不可中断的 GDI 调用前后检查；宿主负责 DPI 初始化、布局变化终止及明确用户触发。
适配器测试不采集屏幕，只检查准入和不发命令的生命周期；worker 测试用合成源。
`desktop/long_capture_scroll.rs`（source 的子模块）提供选区目标绑定的单步滚轮传输。
绑定冻结 HWND/根窗口、PID/TID、根和子窗口屏幕矩形；发送前复核可见/启用及身份位置，
拒绝本应用目标、同线程发送、坐标截断和取消。只向该窗口发送一格消息，150 ms 超时，
不广播、不抢焦点、不移动指针、不自动重试或向父级重复转发。交付成功不是运动证明，
必须由后续截图分析确认；尚未连接自动循环/UI。测试只注入发送回调，不向实际窗口发消息。
单步命令已连接：SessionJob 用 CaptureMode 区分普通、正向和反向滚动采集；动作源
与普通源分开注册，保持单在途。source 的 start_scrollable 在显示控制窗口前绑定目标，
每步复核、发送一格、可取消等待 250 ms、再次复核再捕获，最多 511 次发送尝试，
失败/取消不自动重试。固定等待不等于稳定帧检测。对话框已有“滚动并采集”按钮；
绑定失败退回手动源并禁用该按钮，不丢弃选区。隐藏 UI 测试仅使用动作替身。
`long_capture_stable.rs` 在滚动后的固定等待之外，最多采样五帧，连续两次全像素一致
才接受（至少三张稳定帧）；比较分块检查取消，只保留前后两帧。失败不重发滚轮。
`long_capture_auto.rs` 拥有自动循环调度状态：只在上一帧 Added 后继续，重复/失败
停止，每轮最多 511 步或 300 秒调度窗口。UI 显式开始/停止，停止保留已接受内容并
等待当前任务收尾，不撤回已交付输入。worker 在滚动前检查帧数/逻辑输入预算。
自动停止不自动输出文件或新钉图，用户仍选择预览/完成；测试只用动作及采集替身。
自动循环在确认边界后的每次 Added 回复后先调度有界预览，再调度下一次滚动，
全过程仍是单在途任务。未确认固定边缘时跳过预览并继续收集证据，避免第二帧就因
UnconfirmedEdges 中断。滚动期间保留最近已确认预览，新增内容接受后再刷新；停止
期间完成的预览可显示，但不能恢复循环。预览失败停止后续输入，保留已有会话。
Session 追加前以短租约检查保留输入与最终输出能否同时驻留，失败不提交帧或边缘证据。
滚动前按最大允许位移检查下一帧与输出余量；source 另外检查稳定采样的三份帧峰值。
检查不分配像素，也不长期占用预算；其它窗口竞争后仍可能合成失败，实际分配需再次准入。
控制窗口拥有当前滚动方向：垂直向下/向上、水平向右/向左，单步与自动采集共享该选择。
仅空闲且存在滚动源时允许切换，禁用状态和消息分派都检查准入；切换不清空已接受会话。
`long_capture_spatial.rs` 拥有已定位二维帧的合成，`long_capture_coverage.rs` 用逐行直方图
寻找最大完整覆盖矩形，同面积优先靠上、靠左。先校验坐标/输入量/64 MP 画布上限，再
一次分配 BGRA 输出；较早帧覆盖优先，输入 alpha 不充当采集覆盖证据。最终原地压紧，
租约仍按实际 capacity 计费。位置匹配、取景框移动与会话接线不属于这两个底层模块。
`long_capture_spatial_match.rs` 验证已知取景框位移并识别剩余轴向滚动；`long_capture_view.rs`
提供经过边界校验的借用区域，近似匹配与固定栏分析共用，不分配裁剪像素副本。
已知位移逐像素检查 RGB，剩余滚动仍为有界采样推断；拒绝歧义，不直接提交会话状态。
`long_capture_spatial_session.rs` 持有二维已接受帧和累计坐标，追加先匹配、检查坐标与
输出余量，任何失败回退候选。`long_capture_spatial_preview.rs` 直接采样共享帧，最长边
384 像素，保留透明空洞；最终 compose 才裁成完整矩形。调用方移动量必须相对最后接受帧，
不能在拒绝后悄悄更新物理参考。该会话尚未接 worker 或替换一维 UI 会话。
`Session::to_spatial` 生成独立转换候选：一维聚合图作为优先覆盖种子，最后接受的原始帧
单独作为匹配参考；保留累计帧数与逻辑输入量。种子允许超过单次截图轴长，但受二维输出
上限约束，不能拿它冒充匹配输入。转换失败/取消不消费原会话，worker 尚待接入此切换。
`long_capture_active.rs` 现在拥有 worker 内的一维/二维活动状态；AppendAt 在候选和新帧
均接受后才替换，拒绝、重复或失败保留旧状态。原 worker/单在途通道复用，后续普通采集、
预览和输出分派到活动会话，回复携带二维位置。对话框可消费二维回复，但尚无取景框移动入口。
原生核心 `long_capture_source.rs` 定义 worker 独占 CaptureSource：prepare_move 只准备候选，
finish_move 由接受结果提交或回退区域，取消强制回退。MoveCapture 沿用单在途通道，旧闭包
源通过适配器保留原能力且不宣称支持移动。真实 desktop 源仍待实现该契约。
`desktop/long_capture_source_owner.rs` 现拥有真实区域、候选区域和可选滚动目标：候选稳定
采样后等待 worker 提交，失败/取消丢弃。移动必须保留重叠并位于虚拟桌面；留在原目标内
更新消息坐标，跨出原目标或滚轮坐标范围时降级手动，不重新命中其它窗口。
`desktop/long_capture_dialog_movement.rs` 现拥有四向按钮命令和 UI 物理参考坐标，按边长
五分之一、最多 64 像素移动并裁减到桌面边界。仅 worker 接受后提交坐标并请求预览，
失败保持原参考；在途/自动期间拒绝命令，无移动能力的源禁用按钮。可见取景框边框仍待完善。
`long_capture_dialog_preview.rs` 在接收缩略图时将透明空洞合成到棋盘格显示副本；全不透明
预览共享原像素，含透明图仅分配受预算约束的最多 384×384 副本，不在重绘时分配。
失败停止自动续采；原始帧、覆盖标记和最终导出不受显示处理影响。
`long_capture_outline.rs` 用四条窄顶层窗口显示边框，不创建整屏位图；设置 layered /
transparent / noactivate，生产端逐窗请求捕获排除。边框仅在采集矩形外，接受移动后更新，
跨 DPI 最多校正一次；创建、排除或定位失败结束会话，关闭时 RAII 销毁全部 HWND。
worker 回复现携带源的当前移动/滚动能力，消费回复后更新命令准入和按钮状态；跨目标
移动只有接受后才撤销滚动能力。拒绝/取消保留旧授权，目标身份失效不能借降级绕过。
`desktop/long_capture_dialog.rs` 与 `_layout.rs` 拥有手动长截图控制窗口，
作为 `capture_input` 的私有子模块，不扩大桌面根模块的职责和注册表。
区域选择后显式采集/完成/取消，复用后台任务和新钉图交付。仅在任务在途时开启完成检查计时器，
取消/关闭/布局变化终止；捕获排除设置失败则拒绝继续，避免控制窗口进入截图。
`capture_input` 复用隐藏宿主、模态和文档戳保护；新垂直/水平命令在菜单、路由和
可用性检查中统一接线。隐藏对话框测试仅用合成源，不设置真实捕获排除、不采集屏幕；
自动滚动、图像预览和可见多 DPI 交互仍待实现/验收。
预览已由 `long_capture_preview.rs` 直接采样片段计划生成，最长边 384 像素，像素缓冲
不超过 589824 字节；不先分配完整长图。Session/SessionJob 有独立预览命令，沿用
固定栏确认和单在途约束。对话框新增预览按钮与 `_preview.rs` 自绘区域，只在收到
结果时重绘；新命令清除旧预览，避免显示过期内容。最近邻仅用于预览，原分辨率合成
不受影响；测试比较预览与完整输出采样，并验证隐藏窗口预览不触发完成交付。

`native/src/session_capture.rs` 拥有主干 captureMeta 的有界只读解析：最多 4096 字节，
保留缺失语义，验证区域、HDR/SDR、位深、色彩空间和 backend。session_window 保留
原始 RawValue，并在会话准入时验证已知字段；未知扩展仍留在原始 JSON 中。此层不
读取 filePath，不建立原图租约，也不把元数据等同图片格式验证或文件授权。
Placement 保存有界原始 captureMeta，session_states 将其附到恢复后的文档身份；
CaptureSource 保存完整扩展对象供再次保存。application_save_snapshot 冻结当前来源
元数据，assemble 替换 captureMeta；来源为空时移除旧值，防止替换图片后旧来源复活。
新截图写入动态范围/位深/色彩空间/降级标记及 Context 来源字段；恢复的未知扩展不丢弃。
`capture_context.rs` 拥有 kind/sourceRect/scrollAxis/backend 生成。物理来源矩形明确
附带 otterNativeSourceCoordinates=physical；无坐标时不猜测。WGC HDR/同帧自动 SDR
记录 wgc，GDI 路径记录 gdi。长截图通过独立交付分支保留初始取景尺寸和轴，不使用
拼接输出尺寸；全屏替换也保留 origin，不在打开图片阶段丢弃来源坐标。
仅恢复元数据时不建立原图租约；有合法原生内联原图扩展时由独立原图恢复 owner 校验。
会话保存快照同时保留活动原图租约，后台以 capture_file::encode_inline 按显式文件
偏移读取，再用 loom_base64_stream 的 48 KiB 分块生成带共享预算的 Data URL。
WindowCandidate 将其写入 otterNativeCaptureOriginalSrc，不额外缓存完整压缩 PNG；
仍受 64 MiB 会话总大小限制，超限失败而非丢图。普通图片替换会清除旧原图字段，
只有已恢复的 HDR 元数据但尚无租约时保留会话中已有原图字段。
`session_capture_original.rs` 仅接受内联 PNG，流式 Base64 解码到 create_new 的私有
delete-on-close 临时文件，逐行验证 RGB16/PQ/BT.2020、尺寸、CRC、结束和尾部，不分配
完整解码图。工作区 8 MiB 加行缓冲计共享预算。Images 携带 Arc<File> 到恢复文档，
可继续原图另存/再次保存；失败或最后 owner 释放时自动删除。生产用系统临时目录，
测试固定使用 linshi，不写用户临时目录。元数据或会话路径不能授予外部文件访问。

`native/src/capture_file.rs` 拥有 HDR PNG 的 prepare/commit_new 两阶段新文件发布；编码
复用 capture_png，文件事务复用 session_file_write::Staged 的流式入口。无完整压缩
文件副本，取消/失败只清理本次暂存；提交前仍由 UI owner 复核请求和文档代次。
流式 Staged 不持 Pixels 候选身份，不能通过 is_for 冒充会话修订候选。

文档原图发布通过 commit_leased 在释放缓存排他锁之前取得 FILE_SHARE_READ 租约，
随 Captured → PreparedDocument → CaptureSource 移交；不复制文件、不持有全局缓存锁。
替换/销毁释放租约，sync_raster_source 也清理身份失效来源；租约存在时保守缓存清理
因共享冲突跳过原图，不删除正在使用的内容。普通剪贴板 commit 保持原有无租约语义。

`native/src/clipboard_cache_hdr.rs` 拥有 HDR 原图的缓存暂存，复用 clipboard_cache 的
配额准入、排他锁和 Prepared 发布；写文件时同步计算实际长度与 SHA-256，不保留完整
压缩 PNG 副本。clipboard_cache_receipt 接收所属编码器的统计并记录已发布文件身份。
准入按编码上界而非压缩后的大小预留，可能保守拒绝高度可压缩的大图；UI 仍负责提交前
的请求/文档代次复核，缓存模块不自行打开钉图或修改文档。

`native/src/desktop/capture_wait_dialog.rs` 提供 wait_dynamic 模态等待/取消入口，布局由
capture_wait_layout.rs 拥有。先设置窗口捕获排除再创建 CaptureJob；只有在途任务运行
25 ms 完成检查，取消继续泵消息直至任务退出，销毁时清理定时器和线程。
菜单路由与文档交付仍由 capture_input owner 接入，不由等待窗口自行发布。
wait_cached 现在接入 CaptureJob<PreparedCapture>，与原 Outcome 入口共用同一泛型
对话框过程和取消状态机。CachedRequest 持有捕获参数及缓存目的地快照；等待包括编码，
结果离开窗口后仍未发布，最终提交由截图入口复核身份后负责。
`desktop/capture_input_dynamic.rs` 将区域选择的物理矩形连接到 wait_cached，选择
预览释放后再最终采集；读取 OTTER_CAPTURE_DYNAMIC_RANGE，使用隔离 profile 缓存。
capture_input 的统一交付分支在窗口/文档戳检查后提交 PreparedCapture 并交给 open_job。
全屏和长截图继续走 Image 分支，不改变其选择/采集策略。

`desktop/canvas_capture_source.rs` 拥有文档的捕获来源记录：原图路径、降级原因、显示器
元数据及诊断，以 DocumentStamp 的文档身份绑定。open_image/open_job 从 Captured
直接移交预览，准备命中区域后，新钉图和替换流程把来源附到画布；原有 GDI 截图也复用
该交付类型。替换图片清除旧来源，直接切换 Editor 身份后 getter 拒绝旧记录。来源不是
文件访问授权，也不是应用编辑后的导出结果；当前没有加入会话序列化或编辑后的 HDR 导出。
`capture_file_original.rs` 从捕获租约句柄按显式偏移流式复制 PNG，最多 64 KiB 临时
缓冲，复用 Staged 的无覆盖提交；不根据记录路径重开文件，不重新编码 HDR 元数据。
`desktop/export_original.rs` 提供“另存 HDR 原图（不含编辑）”，复用 export_job 的
单任务/完成检查/退出 join；保存对话框后复核文档戳及租约身份，已有目标一律拒绝。
`desktop/capture_info.rs` 按需读取当前文档来源，提供系统菜单与信息框；降级时菜单
标签明确显示 HDR 已降级。信息框前释放画布借用，使用现有模态 guard，路径和诊断
只显示单行有界文本，不访问文件。来源读取方已接入，原局部 expect(dead_code) 已移除。

`native/src/capture_job.rs` 拥有一次后台动态范围采集线程及唯一结果，调用生产
desktop::capture::dynamic_region；不可撤销取消、拒绝迟到成功、Drop 取消并 join。
默认任务返回 Outcome，start_cached 返回 PreparedCapture；后者在同一 worker 内完成
原图缓存编码，但不发布文件。`capture_delivery.rs` 拥有同帧预览、暂存及诊断的两阶段
交付：UI 无消息泵地复核 owner 后提交；拒绝或丢弃会释放预览并清理未发布原图。
编码后释放 RGB16，不重新解码 PQ PNG 作为预览。任务不自行创建窗口或定时器，不发布
剪贴板；UI owner 必须限制活动任务并复核代次，区域菜单已接入此交付类型。

`native/src/capture_dynamic.rs` 编排 Auto/HDR/SDR：连接 Target、显示器元数据、WGC 与颜色
转换，并在可回退错误时只调用一次注入的 SDR 采集。`capture_dynamic_failure.rs` 将各层
取消/布局变化归为终止信号，不允许回退重新截图；输出保留原因、降级标志和诊断。
HDR Frame 同时拥有原始 RGB16/PQ 与同帧 SDR 预览，分别计预算；into_parts 只移交所有权。
预览用于当前 SDR 画布，不能替代原始 HDR 文件；交付前校验两份图像与选区尺寸一致。
`native/src/capture_platform.rs` 只读查询系统版本；Options::detect 生成当前系统选项，
显式 SDR/非标准工作负载跳过版本查询。版本条件不代替 WGC 的实际 IsSupported 检查。
动态范围入口在返回前核对输出尺寸和字节长度，拒绝不匹配的 SDR 回调结果。
`native/src/desktop/capture_dynamic_source.rs` 提供生产适配入口
`desktop::capture::dynamic_region`：实际探测系统版本，将同一物理选区转交已有 GDI
region 实现作为 SDR 回调，不复制采集代码。UI/文件交付尚未接入。

`native/src/capture_target.rs` 将桌面全局物理选区绑定到单显示器及局部 Region；查询期间
使用线程级 Per-Monitor V2 DPI 上下文并恢复，拒绝跨屏/越界，采集前后复核句柄、设备名
和显示器矩形。转换回调不得提前发布，只有成功返回后才能交给文件/剪贴板/UI owner。

`native/src/live_frames.rs` 拥有 Live 源的最新帧槽、最多 16 个消费者和独立读取游标，
没有积压队列或定时轮询；关闭源唤醒阻塞消费者，最后一个消费者退出清空保留帧。
`native/src/live_texture.rs` 从同设备 immediate context 复制独立 GPU 纹理交给该槽，
拒绝不兼容格式/尺寸/采样及跨设备，预算随整个共享帧存活。无消费者不分配或复制。
消费者不得脱离帧 owner 单独保留 COM 指针；源只保留最新帧不代表消费者持有的旧帧
免费，所有纹理仍受全局像素预算限制。持续 WGC 接线见下，原生呈现和输入授权仍待实现。
后续 `capture_wgc::live`（`live_wgc.rs`）已连接显示器持续 WGC 与上述 GPU 交付层：
同一后台 owner 持有两帧池、设备、事件 token 和 MTA；`live_wgc_signal.rs` 只负责容量
1 的唤醒与终止信号。回调不复制图像、不调用 UI；取消、源关闭或最后消费者退出唤醒
工作线程。停止时撤销事件/关闭 session 和 pool、释放池预算，owner 不能跨线程迁移。
当前仍无宿主启动入口/呈现/输入授权接线；测试不得调用有效显示器捕获。
`native/src/live_job.rs` 拥有持续捕获后台任务和最多 16 个任务准入，复用 `CaptureJob`
取消/join，不另建退出线程。启动返回初始消费者，可追加共享订阅；取消先设置标记再
关闭交付源以唤醒 WGC，错误/panic/退出均关闭源并释放名额。名额随 worker 生命周期
释放，不等 UI 提取结果；Drop 仍可能等待不可中断的 OS/驱动调用。测试只用合成源。
区域任务 `LiveJob::start_region` 复用 `capture_target::Target` 的物理坐标绑定与布局
复核；`Session::monitor_region` 仍持显示器帧池，仅 GPU 快照按选区尺寸分配。
`live_texture::snapshot_region` 校验完整源矩形后使用 CopySubresourceRegion，不做
CPU 裁剪或静默截断。`next_frame_checked` 在复制前后复核目标，未通过的候选不发布。
`desktop/composition_live.rs` 拥有同设备 BGRA8 Live 位图和完整共享帧引用，复用
`Composition::with_device` 创建透明合成链；同帧重绘不重建位图，换设备/元数据不符
拒绝且保留旧帧。设备必须已有 D3D 多线程保护，Composition 的线程 marker 禁止
迁移单线程 D2D owner。测试读回仅用于像素证明，生产绘制不回读；宿主事件/UI 尚未接线。
`live_frame_wake.rs` 合并消费者通知，使用通知代次避免旧投递失败清掉新预留；读取时
在源锁内重新允许通知，回调始终在锁外执行，退休消费者立即移除通知槽。
`desktop/composition_live_notify.rs` 将消费者绑定当前线程 HWND，PostMessage 只带
不可回绕的连接 token。窗口 owner 持有消费者，处理消息时核对 HWND/消息/token，
不能把旧排队消息当作新连接授权。失败计数可查询、后续帧重试，不增加定时器或等待线程。
`desktop/composition_live_host.rs` 拥有订阅、最新待绘帧和惰性 GPU surface；通知只更新
最新帧；suspend 释放 surface 并保留最新帧供恢复，窗口最小化/零尺寸调用此路径。
此职责只针对输出，不接管源应用的隐藏或最小化策略。
宿主 paint 才导入/绘制/提交，零尺寸不创建 surface。已有宿主事件可 refresh
补读，关闭与绘制错误释放订阅及所有帧 owner，不自动重试。它不拥有 HWND 或捕获权限，
也不代替 HWND owner；对应隐藏窗口/WARP 测试覆盖通知到呈现与资源释放。
`desktop/composition_live_window.rs` 拥有 Live 专用 HWND 与稳定地址 State，创建隐藏窗口、
设置 WDA_EXCLUDEFROMCAPTURE 成功后才绑定消费者。WndProc 分发通知、WM_PAINT/WM_SIZE
及关闭，错误保留给调用方并终止窗口；不发送 WM_QUIT、不拥有捕获 worker 或应用消息循环。
Drop 核对 HWND 身份，释放订阅并解除状态指针，避免已销毁句柄复用误伤。它已有 show 接口，
Win32/GPU 调用不持有 RefCell 借用，重入期间消费的通知在外层绘制结束后补失效区，防止漏帧停更；
应用托盘/选择器通过下述 Session 接入；测试保持窗口隐藏，不采集真实桌面。
`desktop/composition_live_session.rs` 将 LiveWindow 与 LiveJob 配对，利用
`LiveJob::start_region_bound` 在 UI 线程完成窗口绑定/捕获排除后才创建采集线程；绑定失败
关闭源并归还配额，不启动采集。Session 显式 stop 先释放窗口再取消 worker，poll 单次交付
最终结果；窗口因源失败关闭时不额外 cancel，保留原始采集错误。CaptureJob/LiveJob 暴露
带借用生命周期的真实线程 completion_handle，供后续应用消息等待集合使用，不能将源关闭
当作线程完成。Session 最终 Drop 仍 join，不分离线程。
`capture_live_input.rs` 是 capture_input 子模块，复用独占门禁、窗口隐藏和冻结选区，
只排队已确认的单显示器物理 RECT 或拥有进程句柄的窗口 Target，不替换文档；两者共用
单个类型化请求槽，不能叠加排队。`application_live.rs` 在回调外创建和持有
最多 16 个会话，结算后回收并报告失败；`application_live_wait.rs` 在无 Live 时使用
GetMessage，否则用 MsgWaitForMultipleObjectsEx 等待线程/源进程句柄与消息，固定 32 槽并去重、无周期
轮询。等待期间 owner 不变，线程就绪返回一次合成 WM_NULL 推动正常结算；应用退出先
停止所有会话再回收。托盘命令 0x106b 为“区域 Live”，0x106c 为“窗口 Live”；可见捕获与性能仍待验收。
`live_window_picker.rs` 是 Live 输入子模块：最多 64 个外部可见/非最小化/非 cloaked 顶层
窗口候选，跳过当前 Otter 进程。固定 257 UTF-16 单元读取标题，仅保留 128 字符展示，
转义菜单 & 并清理控制字符；PID/HWND 后缀区分同名窗口，不按标题再次定位。候选持有
Target，选择后再次验证，取消/过期释放所有候选；窗口标题只用于这次原生菜单，不写日志。
`application_shutdown.rs` 负责主循环结束后的统一任务回收，保持停止通知、停止 Live、
回收目录/输入/执行任务的顺序；`host_lifetime.rs` 同时拥有 UI COM apartment 和 HWND
生命周期，使应用入口只保留协调接线，不继续扩大状态宿主文件。
召回判断由独立 `live_reclaim.rs` 提供：无 OS 调用的有界意图记录及每源激活状态机，
时间与序号分离，取消/过期/旧序号拒绝。宿主必须只记录已验证的非注入 shell 输入，并在
同一 UI owner 上使用最新快照；该模块尚未接全局输入 hook，不自行恢复外部窗口。
`live_recovery_wire.rs` 拥有恢复进程的有界请求/ACK 数据合同及 framing：单帧 4096 字节，
非零 session/sequence/登记 id，拒绝未知字段及非法记录，区分正常 EOF/半包。
它不负责握手认证、进程绑定、幂等状态机、超时或窗口恢复；nonce 和结构合法不等于授权。
`live_recovery_service.rs` 负责已认证 session 内的单请求在途登记状态机：缓存最后请求/
ACK、连续序号、单调登记 id、最多 16 个源 owner，退休不会误删新代次；仍有记录不退出。
绑定与恢复通过明确回调边界接入，恢复失败保留 owner；它不启动进程、不判断 EOF 为父死。
`live_recovery_target.rs` 是 Target 的 recovery 子模块，将 Snapshot 转换为包含进程
创建时间的 Record；Binding 登记时核对 PID/TID/创建时间并保活 Target，恢复不重新选源。
查询权限单独申请，读取失败拒绝登记；只能接已认证通道，不能把身份一致视为用户授权。
`live_recovery_parent.rs` 持有原父进程句柄并核对直接父 PID、创建时间顺序、相同 executable
路径和输入/ACK 输出管道 server PID；普通恢复认证只申请查询/等待，不用 EOF 推断死亡。
紧急退出专属认证在同一身份门下额外申请 PROCESS_TERMINATE，只终止保活的原父句柄，
不按 PID 重新打开，不终止 Job 或恢复 child。`native/tests/live_recovery_parent.rs`
只启动同一测试 executable 的子进程验证真实管道准入/拒绝；辅助测试必须由父测试启动。
`live_recovery_handshake.rs` 只负责已认证管道的 Hello/Ready 数据合同，严格校验方向与
session，复用 wire framing；不负责进程启动或阻塞 I/O 截止时间，不替代父端认证。
`live_recovery_pipe.rs` 拥有 overlapped 管道操作、绝对截止时间和对端句柄联合等待；
`live_recovery_pipe_pair.rs` 创建仅限本机的单实例双向管道。无读写后台线程或空闲轮询，
取消后确认内核操作结束才释放缓冲区。超时可能已有部分传输，不保证协议流可直接重用，
也不承诺内核取消清理的硬实时上界。传输模块不负责父端认证或恢复决策。
`live_recovery_pipe_adopt.rs` 校验继承端点确为字节管道，并查询内核 FileModeInformation，
拒绝同步 I/O 模式；不凭 stdin、命令行或环境标志假设 overlapped。查询失败拒绝接收。
`native/tests/live_recovery_session.rs` 使用同 executable 子进程与实际继承端点测试认证
握手、无效源拒绝及空登记关闭；测试辅助入口不是生产 CLI。
`live_recovery_session.rs` 串联父端认证、Hello/Ready、登记状态机和 ACK。空闲首字节
事件等待不设超时，收到首字节后为剩余帧及 ACK 设置共享期限；失败流永久封闭，保留
已接受的 owner。恢复入口只在原父句柄退出后执行，失败记录保留供调用方重试。
`live_recovery_runtime.rs` 拥有私有 `--native-recovery-parent PID` 入口与恢复循环：
在配置/UI 初始化前分流，参数独占，验证继承端点后认证握手。坏流有登记时等待原父死，
恢复失败按 1/2/4/8/16/30 秒退避并保留 owner，无记录则可退出；不输出协议日志。
Binding 仅在原进程退出、HWND 消失或 PID/TID 改变时无写入退休，查询失败仍保留。
`live_recovery_client.rs` 拥有父端子进程、管道及请求序号；启动同 executable，Ready 前
失败可回收自建子进程，Ready 后 Drop 不杀进程，以保留 ACK 不确定时的恢复责任。
请求成功只接受精确 session/sequence ACK，失败流封闭；存活检查使用原始子进程句柄，
Shutdown ACK 后才能 join。握手/请求/join 仅限后台，UI 只借用退出等待句柄。
实际 Live 的登记、源隐藏及关闭调度现由 live_visibility_owner 和 Application 接入。
`live_recovery_spawn.rs` 是恢复/紧急退出私有进程的 CreateProcessW owner：STARTUPINFOEX 句柄
白名单仅含协议输入和 NUL，关闭临时副本及初始线程句柄。不能退回宽泛句柄继承。
`live_recovery_spawn_text.rs` 只编码私有同 EXE 命令行及测试环境覆盖；限制长度/NUL，
处理引号与末尾反斜杠，不经 shell、不允许替换 executable 或工作目录。生产直接继承
原环境；测试覆盖不修改父进程环境。这不是通用进程启动器。
`emergency_exit.rs` 独占 Esc+Delete 三次完整按放状态机：两键重叠、每次双键释放、
相邻组合严格小于 400ms，自动重复和单键释放不累计；主消息循环在普通业务分派前观察。
主通道仅在相关键消息时读取当前物理快照，不信任可能被失焦/嵌套模态泵漏掉的keyup历史；
延迟/缺失边沿只会少累计，不凭过期状态制造组合，完整物理观察仍由独立child承担。
`desktop/application_loop.rs` 拥有 Application 原消息泵及启动窗口显示/恢复顺序，
由 Application::run 的统一 shutdown 围住；系统结束围栏、热键交接和保存调度顺序不变。
`emergency_watchdog.rs` 独占配置/UI 前分流的同 EXE 私有 child、父认证、Hello/Ready 和
8ms 只读键位采样，另支持 Ctrl+Alt+Shift+F12；先见松开才 arm，不保存输入历史、不注入输入。
`emergency_watchdog_client.rs` 独占启动握手和自建 child 回收，正常 Drop 可终止此 child，
不能套用到仍承担 Live 恢复责任的 Client；紧急终止绕过保存，不能保证未保存工作保留。
原生未修改全局 ClipCursor/ShowCursor/SetSystemCursor，因此不无条件重置其他程序光标状态。
`emergency_watchdog_tests.rs` 保护参数/启动已按住/组合边沿；其 runtime 子模块
`emergency_watchdog_runtime_tests.rs` 仅 cfg(test) 提供确定性键位与同 EXE 牺牲父进程，
核认证失败不 arm、阻塞父进程终止、父死退出与正常 Drop 回收；不替代物理键盘/前台验收。
`live_recovery_async.rs` 是 Client 的单任务异步 owner：握手/请求期间才有 worker，
空闲仅保留 Client，无工作线程、队列或轮询。暴露任务完成/空闲子进程句柄，poll 只
join 已完成任务；请求结果连同 Client 返回，spawn 失败保留原 Client。关闭必须先泵
消息至非 busy；Drop 的防泄漏 join 可能阻塞，不能当作 UI 正常关闭路径。
`live_recovery_journal.rs` 在 UI 线程协调最多 16 个源的登记/退休意图，同一时刻只向
异步 owner 提交一个请求；Ticket 仅在登记 ACK 后授权，退休回调不执行管道 I/O，
未收到退休 ACK 返回 E_PENDING，使 lease 保留原快照。取消在途登记仍须结算并退休。
未知 ACK 或失去 Client 句柄不等于子进程已死，保留责任并禁止继续隐藏；仅原进程
句柄确认退出才可清除远端重放风险。正常关闭不擅自退休仍在保护的源，必须先恢复。
`live_recovery_journal_tests.rs` 拥有真实子进程协调及自有隐藏窗口 lease 联合测试。
请求 worker 在 Client 外设置 unwind 边界，panic 后保留原句柄并封闭协议流。故障
Journal 只有在全部已发送源请求退休后，才通过原句柄终止自建恢复进程；确认进程退出
前不清除快照。调用方必须先恢复窗口再请求退休，此终止路径不等于正常 Shutdown。
Journal 经 live_visibility_owner 接入 Application 的等待集合和关闭消息泵。
`live_visibility_owner.rs` 拥有 UI 线程的源表、Registry 和按需 Journal；同源共享
唯一登记及原始 Snapshot，每个 Session 持 Protection/Lease。ACK 后才 arm/hide，
最后 lease 退出恢复，退休确认后释放源表，最后源结束后关闭恢复子进程。恢复失败
不丢弃源表；待退休/关闭阶段不接受该源的新一轮隐藏，区域 Live 不启动恢复进程。
显式召回通过 Owner::observe_reclaim 将最新 Intent 快照交给每源 Activation；隐藏前
持续吸收旧意图。召回恢复共享源但保留所有 Protection/Lease，退休后存活的会话不再
隐藏源；全部登记退休即可关闭 helper，无需等待已召回的 Live 会话结束。OS 输入
watcher 提供已分类意图，不得以任意前台变化自行构造 Intent。Registry::restore_source
只恢复既有 owner 的源，不改变 lease 引用数；活跃召回的退休重试由应用 owner 执行。
后续接线已由 `live_reclaim_watch.rs` 完成：仅需求期间安装 UI 线程键鼠低级 hook
及前台/最小化 WinEvent，回调更新有界状态并置手动复位事件，应用等待集合消费该事件。
`live_reclaim_input.rs` 只分类物理 Alt+Tab/Alt+Esc、Win+Tab/数字和 shell 左键边沿，
维护修饰键位、不保存文本或输入历史；排除注入标志，不处理鼠标移动。Watcher
与源 owner 同线程析构，关闭/全部召回后注销，不增加空闲监听线程或轮询定时器。
`live_visibility_runtime_tests.rs` 为显式 WGC 可见性门禁：仅捕获自建纯色 HWND，
串联真实恢复进程、隐藏、颜色更新、召回后持续采集和退出；普通/初始最小化两种
状态均覆盖。像素 staging 读回仅用于测试断言，不进入生产帧路径。召回意图由夹具
注入，不安装全局输入 watcher，不能代替真实任务栏/Alt+Tab 手动验收。
`live_recovery_runtime_tests.rs` 是显式运行的三进程恢复门禁：观察者持有自有隐藏窗口，
登记父进程与恢复子进程分离，验证 ACK 后修改、EOF 不提前恢复、父退出后还原和子进程
退出。辅助入口只能由夹具启动；输出有界读取已到达字节，不能等待 EOF 停止消息泵。
`live_window_target.rs` 绑定显式选择的顶层 HWND、PID/TID 与原进程同步句柄，移动和标题变化
不改变目标，拒绝子窗口隐式提升；进程退出或身份变化拒绝后续交付。Target 可移交 worker，
不持有 UI 状态。`capture_wgc::live::Session::window` 复用现有池/事件/预算/关闭链，
通过 CreateForWindow 绑定 WGC item，不以屏幕矩形模拟窗口捕获；LiveJob 在快照前后复核
身份，呈现 Session 仍先排除自己的窗口再启动采集。窗口选择菜单已接入，完整最小化策略
尚未实现；同一存活 PID/TID 内的 HWND 快速复用存在启动竞态，不能声明唯一窗口代次。
其 `snapshot` 子模块由 `live_window_snapshot.rs` 实现，只读保存恢复所需的 rect、placement、
iconic、完整扩展样式和可读的 layered 属性。读取前后验证 Target，属性不可读则拒绝，
不猜默认值；它既不变更窗口，也不代表已有恢复 journal 或可见性 lease。
`live_window_restore.rs` 是 snapshot 的恢复写入子模块：校验快照后逐阶段复核 Target，
恢复原样式、layered 属性、topmost/几何及 placement。visible 独立保存，防止隐藏窗口
被 showCmd 意外显示。失败不销毁快照，不负责删除 journal；隐藏事务尚未接通。
其 restore_or_retire 由本地 lease 与恢复 Binding 共用：原进程已退出、HWND 消失或
PID/TID 改变时无写入退休，查询失败仍保留责任。源消失不跳过远端退休 ACK。
`live_window_visibility.rs` 是 Target 的共享可见性子模块，UI owner 持 Registry，每个
会话持一个 Lease；上限 16 源、每源 16 lease，不使用帧 Consumer 数控制外部窗口。
首次 arm 保存唯一快照及恢复记录退休回调，最后 lease 退出恢复，主动 restore 不减引用。
失败保留条目；已恢复但退休失败只重试退休，防止覆盖用户新修改，期间拒绝新隐藏。
不跨 Win32/回调持 RefCell 借用；Rc 限定 UI 线程，重入操作保守拒绝。通过应用级
live_visibility_owner 接入真实 Journal，不按帧消费者数量决定外部源恢复。
`desktop/application_live_shutdown.rs` 负责实际 Application 正常关闭的 Live 结算：
先停止全部会话，再联合等待任务句柄和分发 UI 消息，线程完成才移除 owner；不提前
clear 后阻塞 join。关闭期间 WM_QUIT 暂存并在结束后还给外层，不取消清理或忙循环。
该等待同时结算源恢复 Journal 的任务/子进程句柄，不能仅因采集 worker 已退出就结束。
对应 shutdown_tests 用跨线程同步窗口消息验证消息泵确实继续运行，不捕获用户桌面。
Application::run 将主循环放入借用 self 的 run_loop，统一在其正常/错误返回后清理；
`?` 或非零 WM_QUIT 不再绕过 shutdown。此边界处理 Result 返回，不承诺捕获任意 panic。
`live_window_hide.rs` 提供已 arm lease 使用的隐藏写入后端：LAYERED/TRANSPARENT、
alpha=1、置底且不激活，不使用 SW_HIDE。最小化源只在登记之后尝试恢复可采集状态，
没有固定 sleep；Win32 调用仍不承诺硬实时。Lease::hide 拒绝未 arm/退休待定状态，
同源 peer 重复隐藏不覆盖快照；部分失败尝试恢复，恢复或退休失败继续保留记录。
`live_wgc_resize.rs` 拥有窗口帧池尺寸准入与预算事务：旧帧先归还，保留旧池预算并预留
新池预算，池及捕获会话换代成功才提交尺寸并释放旧租约。失败由 Session 关闭源与资源，不把
池失败当作可继续使用。零内容只等待下一事件，保留最后一帧；固定显示器/区域不允许
尺寸变化自动扩大捕获边界。真实 WARP 帧池重建测试不创建捕获 session 或读取屏幕。
`live_window_watch.rs` 在任务 owner 的 UI 线程注册限定 PID/TID 的 EVENT_OBJECT_DESTROY，
再次匹配 HWND/OBJID_WINDOW/child 后只关闭共享帧源以唤醒 worker，不在回调中 join。
最多 16 个监听，Rc 限定同线程注销；完成/Drop 先注销监听，注册表借用外执行单次通知。
这补偿本机真实 WGC 测试中源窗口销毁后未收到 Closed、线程继续等帧的情况。Target 的
原进程句柄通过 Arc 共享给工作线程，LiveJob owner 因监听生命周期而限定在泵消息的线程。
`native/tests/live_window_runtime.rs` 是显式 ignored 集成测试，仅捕获自己的可见纯色 HWND，
验证首帧、颜色更新、快照不被覆写、尺寸重建、同设备呈现和源关闭回收；不枚举或保存
用户窗口内容。普通回归不启动此测试，运行方法和验收边界见迁移计划。
`native/tests/live_process_runtime.rs` 只启动并捕获自己拥有的纯色子进程，验证正常关闭和
强制退出的进程唤醒、句柄退休及资源回收；必须按迁移计划过滤运行父测试。
LiveJob 保留源 Target，poll 验证失效后关闭帧源并停止等待其进程句柄，避免忙循环。
原生启动入口要求显式 COM apartment；调用方须保活到任务退出，不能依赖 worker 的临时 MTA。
`native/src/capture_wgc.rs` 拥有显式请求的一次性显示器 FP16 WGC 会话；独立 D3D11 设备、
两帧池、一秒取帧等待及取消，成功后接读回层，不保存全局设备或会话。
`capture_wgc_lifetime.rs` 负责帧/session/pool 的 Close 和线程内 WinRT 初始化配对。
生产调用须来自已授权的后台截图请求；当前尚未接 UI 入口，测试不启动真实采集。

`native/src/capture_readback.rs` 拥有单次 FP16 D3D11 staging 读回与映射 guard；验证纹理
布局及设备一致性，在有界非阻塞 Map 重试后借用行缓冲给颜色转换，退出时 Unmap。
`Region` 使用源纹理局部物理像素；区域路径先校验，再只创建选区大小的 staging，
通过 CopySubresourceRegion 复制，不先读回全图；WGC 的 monitor_region 接口复用此路径。
其 unsafe 接口要求调用方独占 immediate context 并冻结源纹理；不启动或缓存捕获会话。

`native/src/capture_display.rs` 拥有显示器 HDR 元数据快照；`capture_display_config.rs`
负责有界 DisplayConfig 查询、设备名匹配及白电平读取。每次显式查询创建 DXGI factory，
不保留轮询或捕获资源；系统状态未知与白电平回退分别标记。此层尚未接入截图入口。

`native/src/capture_color_hdr.rs` 是颜色模块的 HDR 输出子模块：直接将借用的 scRGB 行转换为
BT.2020/PQ RGB16 大端数据，携带内容亮度和 mastering 亮度范围；输出租约跟随数据共享，
不可作为 BGRA 预览。该模块不编码 PNG，也不写文件或创建捕获会话。
`native/src/capture_png.rs` 负责 HDR RGB16 的流式 PNG 编码与色彩/亮度块；
`capture_png_sink.rs` 负责有界输出及失败/取消后的写入锁闭。编码器只借用 writer，
不分配文件路径、不发布文件；调用方负责目标缓存、sync_all 和部分写入的处置。

`native/src/capture_color.rs` 拥有借用 FP16 scRGB 行缓冲的亮度分析和 SDR/BGRA 转换：
校验尺寸、行跨度和长度，不复制原始浮点帧；负值/非有限通道、增益和白电平规则对齐
主干 screenshot/capture_pixels 与 hdr_analysis。输出计入共享预算并按行取消，不创建
设备、截图会话或编码文件；目前尚未接入实际 HDR 捕获源。

`desktop/screen_color.rs` 拥有画笔/边框及填充的屏幕取色菜单与模态会话。
颜色、宽度、圆角及其它样式菜单命令由 `style_input.rs` 统一分派，`context_menu`
继续负责统一准入和操作收尾，不在菜单模块重复属性实现。取色确认前复核
原文档、工具、样式和 HWND，取消不修改颜色。`screen_color_hook.rs` 只在显式会话
期间安装低级鼠标 hook，回调仅合并移动/转发点击，不采样、不绘制；会话代号阻止
上轮排队消息污染新会话，完成/取消/关窗卸载 hook。`screen_color_preview.rs` 负责
事件驱动的 GDI 单像素采样及不激活的预览，DC 即取即还，不保留桌面截图。
`screen_color_swatch.rs` 拥有预览窗口子类和原生色块/HEX 绘制，尺寸、字体、色块及
内距按 DPI 缩放；颜色未变不重复更新文本/主动请求重绘，自绘仍保留 STATIC 可访问文本。
跨屏定位最多校正两次，WM_DPICHANGED 采用建议矩形，无布局轮询。临时字体/画刷
通过 RAII 释放，销毁时移除子类。内存 DC 像素及 GDI 计数测试不替代真实混合 DPI 验收。
鼠标进入旧预览矩形时先隐藏并等待合成刷新再取色，避免采到自身。隐藏回归不安装
真实 hook、不采样桌面；跨应用输入、HDR、可见预览与混合 DPI 尚未人工验收。

`native/src/loom_bridge_stream.rs` 的流与所有中断句柄共享不可逆取消标记：Windows
取消先置位再触发独立事件，I/O 前检查；取消返回 ConnectionAborted，避免 read_exact
将 Interrupted 当作可重试错误。实际 socket 非阻塞，等待同时包含网络与取消事件。

`native/src/desktop/host_lifetime.rs` 统一拥有 HWND 的 RAII 回收与 UI 线程宿主计数；
`desktop/mod.rs` 只复用窗口所有权类型，不重复实现销毁逻辑。窗口及其绘制资源仍须在
COM apartment 退出前释放；显式销毁后 Drop 不重复销毁窗口。
`desktop/procedure_tasks.rs` 集中路由图像打开、导出及编辑任务的完成定时器；保持各任务
既有收尾逻辑及定时器生命周期，不在消息总入口继续累积完成分支。

`desktop/drag_transport.rs` 拥有文件拖出的 OLE 初始化、IDropSource 取消/松键策略及
仅 COPY 的结果分类；`drag_transport_data.rs` 用标准 Shell IDataObject 承载有界 CF_HDROP。
数据对象不访问文件、不写系统剪贴板、不删除来源；调用方负责导出快照的权限和寿命。
DoDragDrop 有嵌套消息循环，调用方不得跨调用持有 UI 借用，关闭时须取消。
`drag_transport_tests.rs` 验证真实 COM 数据对象与源回调，不执行真实鼠标拖动。
`desktop/drag_prepare.rs` 拥有后台 PNG 快照编码和暂存任务；输入包含冻结的像素、编辑器、
透明度、缓存策略及 stem，启动拒绝草稿并去除编辑器撤销/重做历史。`export_drag.rs`
复用 WIC 合成而不生成 DIB；暂存复用既有归属记录/容量/取消合同。完成只交付未发布
Prepared，调用方复核手势与版本后才能 commit；取消/Drop 负责 join 和回收未发布暂存。
`desktop/drag_input.rs` 将平移工具下的 Shift 按下/系统拖动阈值/活动完成定时器接到上述 worker；
选择/绘图工具保留 Shift 框选及约束，不由拖出入口抢占；该工具边界有独立回归。
拖出模板和显示版本在启动时冻结。松键、Escape、失焦、捕获丢失及关窗取消准备。
普通手势取消只发信号，不在输入消息中等待 WIC 编码；已取消任务继续占有单槽，
由原活动定时器在完成后回收，期间不接管新编辑的鼠标移动/捕获，不启动第二个准备任务。
HWND 销毁或发现 HWND 已失效时才强制 join；`drag_cancel_tests.rs` 用受控 worker 验证
取消返回、单 owner、后续捕获保留和销毁回收，不依赖毫秒级性能断言。
`drag_input_finish.rs` 在发布前核对 HWND/鼠标/显示版本/模态/锁定，进入 OLE 前再核对松键；
同步拖放期间持只读文件保护句柄，禁止其它缓存清理删除来源。全程不跨可重入调用
持有 RefCell 借用，OLE 活动期间拒绝本窗口编辑/命令重入，关窗使取消信号生效。
生产接线已有隐藏窗口及真实 worker 回归，最终 OLE 调用用注入回调验证；真实键鼠、
跨进程拖放、拖动图像预览和高 DPI 尚未人工验收，不能宣称完整主干拖出行为已对齐。
`desktop/drag_preview.rs` 在准备 worker 内从最终合成位图生成最长边 256 物理像素的
等比缩略图，不放大小图；内存纳入像素预算，输出直通 alpha BGRA 供 Shell 自行预乘。
`drag_transport_preview.rs` 创建有界 DIB 并交给 IDragSourceHelper，失败时本地回收，
成功时由 Shell 管理；预览失败记录错误但仍允许文件拖出。图像不在鼠标移动时重建，
UI 不重新解码 PNG。真实 Shell helper 及循环 GDI 计数回归不替代可见拖动验收。

`native/src/app_settings_document.rs` 拥有配置 JSON 的有界解码及命名字段合并，不执行 I/O。
接收缺省或 schemaVersion 1/2，拒绝未来版本、显式 null、错误类型及非法命名；不把失败
降级为默认配置。root 与 fileNaming 中不归本模块所有的字段按 JSON 值保留，旧版本号
不自动提升；不保证原始空白、键顺序或重复键的字节级保留。输入和序列化输出均限 1 MiB。
`desktop/application_naming_import.rs` 将其接到显式“导入命名设置…”菜单，
`naming_import_job.rs` 单槽 worker 复用 session_file 的普通绝对路径/非重解析磁盘文件准入，
并在分配读取缓冲前限制 1 MiB。命名设置应用前复核来源 UnitId 仍有窗口及原设置值；
取消、无效文件、来源关闭和过期结果保留当前值。模态期间延迟消费，重复请求合并；任务
通过应用 wake 通知，不新增空闲轮询，退出取消并 join。只导入命名字段，本次运行有效，
不写源文件或自动访问主干目录。测试使用隐藏 HWND 与 linshi 合成文件，
注入文件框选择，真实读取/解码/任务回收；可见文件框尚待人工验收。
`application_naming_save.rs` / `naming_save_job.rs` 提供显式“命名设置另存为…”：Application
保留成功导入的不可变 Document，worker 复制并合并当前命名字段，保留其它字段及旧版本号。
无导入来源时生成 schemaVersion 2 文档。编码上限 1 MiB，缓冲纳入共享字节预算；复用
`session_file_write::Staged::prepare_bytes` 的同目录暂存、不覆盖发布和取消清理，不伪造会话
修订或候选身份。UI 提交前复核原窗口 owner、设置值和来源 Document 身份；新导入即使
命名值相同也使旧保存失效。目标已存在或并发出现均拒绝，不改写源文件。另存结果可重新
导入，也可通过 --settings 显式启动加载；默认 profile 规则见下文。隐藏回归验证源不变、扩展保留、往返、
取消/过期/关窗/竞争失败及退出清理；不声明断电持久性或可见交互已经验收。
`native/src/startup_settings.rs` 解析可与图片/截图/工作区入口组合的 --settings 路径，保留
会话选项值的边界，拒绝重复/缺失/空路径。`desktop/settings_startup.rs` 在截图与建窗前
执行同一有界只读校验，坏配置直接失败，不回退默认值。初始窗口先应用设置，Application
持有原 Document，恢复窗口继承共享 owner。`native/tests/settings_startup.rs` 运行真实隐藏
EXE，覆盖普通启动与恢复、中文相对路径、无效/超限配置和句柄回收；无效配置的截图入口
在采集前失败，测试不执行真实截图。未指定路径时按隔离 profile 规则处理，不读取主干目录。
`settings_profile.rs` 选择优先级为 --settings、OTTER_NATIVE_PROFILE_DIR、LOCALAPPDATA 下
的 OtterNative/app-settings.json。显式文件缺失报错；profile 首次缺失使用默认值，启动不
创建目录或文件。无显式覆盖的隐藏检查不发现真实 profile；测试进程 runner 清除继承的
OTTER_NATIVE_PROFILE_DIR，仅保留测试显式传入值。profile 根须为普通绝对路径，存在时
不得为文件或重解析点；这不是对恶意祖先目录竞争的沙箱保证。
首次“保存命名设置”或开启自动保存后的实际变化由 worker 创建 profile 目录，再执行
不覆盖发布；成功后采用正常 Source 基线。并发首写者保留，错误不降级成覆盖或默认值。
创建失败/取消可能留下空目录，但不删除其他内容。profile_startup 与 naming_profile 回归
覆盖只读启动、显式优先、坏配置/恢复歧义拒绝、首次写入、后续更新和首写竞争。
`settings_source.rs` 保留显式来源路径、原始字节及不可变 Document；字节受共享预算管理，
用于核对磁盘内容，不用重新序列化后的等价 JSON 冒充原始基线。`settings_update.rs` 验证
当前命名并保留扩展，通过 `session_file_replace::Replacement::prepare_bytes` 复用同一
句柄基线租约、父目录固定、恢复重命名及竞争保护，不伪造会话修订。
“保存命名设置到已加载文件”只更新已有来源；无来源明确拒绝。另存为成功后新文件成为
活动来源；后续保存不再指向旧导入文件。只有发布成功才推进来源字节基线；外部修改、
UI 设置/来源改变、关窗和取消均不覆盖旧文件。旧恢复文件退休失败属于成功加警告，UI
显示保留路径；发布失败显示恢复路径。此替换不是原子覆盖，目标有短暂无文件窗口，
不能声称全平台持久性已验证。显式 --settings 启动在原文件缺失时，复用同目录有界恢复
发现（最多 4096 条目、严格 GUID 文件名、多个候选拒绝）；1 MiB 读取上限同样适用于副本。
原文件存在但损坏不回退，查找期间原文件重新出现则优先原文件。恢复只读，不自动恢复
原路径或删除副本；Source 记录 recovery_target，可见启动提示实际路径，隐藏检查只输出
状态。UI 与 settings_update 双层拒绝覆盖恢复来源，要求另存为后再更新新活动文件。
`native/tests/settings_recovery.rs` 使用合成中断现场和真实 EXE 检查优先级、歧义/坏格式/
超限拒绝、源字节不变及句柄释放；这不是实际断电实验，自动清理临时残留仍未实现。
`application_naming_autosave.rs` 提供可选的事件驱动自动保存，菜单默认关闭且不跨启动
保存开关。开启后仅对有效活动来源的实际命名变化排队，复用单槽保存 worker；模态、导入
或保存忙碌期间不加任务，不创建空闲计时器。显式请求优先，重复相同设置不写盘。关闭
开关会丢弃尚未发布的自动结果，不取消手动保存。任何保存错误（含过期结果）都停用
自动保存、保留运行时设置并取消待退出意图，要求用户处理后重新开启，不无限重试。
应用“退出 Otter”在已开启的配置保存完成前等待，再继续原工作区退出确认；单文档
WM_CLOSE/强制结束进程不在此保护范围。新建及恢复窗口共享开关，可见交互仍待人工验收。

`native/src/desktop/export_naming.rs` 拥有 PNG 导出框的默认建议名，按主干默认模式生成
`Otter_{date}_{time}_{width}x{height}.png`；本地时间精确到百分之一秒，仅显式导出时读取。
`export::dimensions` 是建议名和 WIC 合成共同的尺寸入口，使用当前显示来源与已提交裁剪，
不把宿主视口缩放/平移当输出尺寸。名称只是建议，用户可编辑；文件创建仍以 create_new
拒绝覆盖。设置支持隔离 profile、显式导入/另存/更新及可选自动保存；自动冲突递增尚未接入，不能等同完整命名迁移。
`native/src/file_naming.rs` 拥有主干 12 个占位符的纯渲染与校验；默认导出已调用该入口。
剪贴板文件协议沿用 `paste_input::file` 的有界 CF_HDROP 解析边界；新增
`clipboard_file_encode.rs` 编码绝对 Disk/UNC 路径为宽字符 DROPFILES，不访问文件系统。
拒绝 NUL、父路径跳转、设备路径、超过32项/32767路径字符/128 KiB包的输入。
`clipboard.rs` 的多格式发布接口可预分配 PNG、DIBV5、可选 CF_HDROP 的全部 HGLOBAL，
成功后才 Open/Empty 一次；未转移资源由 RAII 回收，Empty 后部分发布不能回滚。
调用方必须先完成磁盘文件并保持寿命；不能因部分发布失败立即删文件。现有复制任务
已由 copy_job 的文件模式提供文件列表；隐藏无文件 smoke 显式注入 None，不访问真实缓存。
缓存事务现由 `native/src/clipboard_cache.rs` 提供：隔离 profile/clipboard_cache，后台
暂存已编码 PNG，调用方复核请求后才递增发布；未发布 Drop/取消回收暂存，已发布文件保留。
`session_file_write::Staged::commit_incremented` 只允许无会话绑定的资源，复用同一个
暂存文件重试重命名，不重复编码或重写 PNG；会话候选禁止该路径。cache.lock 排他句柄
覆盖合作进程的扫描准入、暂存和发布；最多4096目录项、默认256 MiB，包括遗留文件，不递归、
不清理未知文件，遇超限明确失败。最终目录/条目拒绝重解析点，不声称恶意祖先竞态沙箱。
`desktop/copy_file_plan.rs` 在显式复制时冻结 clipboardFilePattern 与原生 profile 目录；
copy_job 编码一次，worker 暂存同一 PNG；主线程复核 HWND/文档/取消后提交缓存文件，
再将三格式交付系统。过期、取消、调度失败和退出回收未发布暂存；开始发布后即使失败
也保留文件。隐藏 HWND 回归只替换最终剪贴板副作用，检查文件字节、单次发布、递增及
取消/过期不发布；真实应用间粘贴仍未验收。清理后缓存仍满时明确报错，
不扩大删除范围、不静默降级为图片格式。
Application 共享 copy_authority 到新建/恢复窗口，新复制以取消信号取代旧请求；旧任务
仍由其窗口回收，取消新请求不会复活旧任务。生产入口同时冻结系统剪贴板序号，完成时
变化则丢弃未发布暂存；最终发布在 OpenClipboard 成功后、EmptyClipboard 前再次比较，
防止缓存提交/内存准备期间的外部更新被清空。隐藏测试注入序号和最终发布，不访问系统
剪贴板内容。缓存锁仅在后台争用时按10 ms间隔等待，最多1秒且可取消，让旧结果有机会
回收锁；无空闲轮询。跨进程未发布的用户意图不由应用内 authority 仲裁，序号检测遵循
Windows 延迟渲染和32位序号合同，不等同无限历史事件识别。
`clipboard_cache_receipt.rs` 为新发布 PNG 写入有界、GUID 命名的所有权记录，包含版本、
直接子文件名、字节数、SHA-256、创建秒数和卷/文件索引；摘要在后台对已有编码字节计算。
准入为 PNG 与最多2 KiB记录预留容量和两个目录项。PNG 发布后记录失败仍保留 PNG，并
报告路径；没有记录的旧文件不会被追认成可清理文件。记录是后续清理的核验数据，不是
抵御恶意本地用户伪造的授权凭证；清理在受控目录核对身份/内容并使用同一句柄。
`clipboard_cache_cleanup.rs` 现于后台暂存前、持缓存排他锁时执行默认7天保留期清理；
仅识别 GUID 命名的有界版本1记录，核对单个直接子文件名、字节数、卷/文件身份和SHA-256，
要求普通非重解析且单硬链接文件。记录/PNG始终持有拒绝写入/删除共享的句柄，验证后
通过同一句柄退休PNG和记录，不关句柄再按路径删除。未知/损坏/未来版本/时钟回拨、
外部修改/替换、占用或硬链接文件均保留；取消在读取块与删除前检查。每轮最多4096项、
512 MiB哈希读取，固定64 KiB缓冲；清理可部分成功，不提供回滚或恶意目录沙箱。
仅测试隔离linshi目录，未运行真实用户缓存清理。
默认超过7天的已验证文件可能失效，不承诺旧剪贴板文件引用永久有效。
`cache_policy.rs` 对齐主干 cache.tempCacheMaxBytes/tempCacheRetentionDays：默认256 MiB/7天，
0分别关闭磁盘软配额/按时间过期；非零容量32 MiB–16 GiB，天数最多3650。启动加载显式
校验并共享到新建/恢复窗口；copy_file_plan 冻结策略交给后台。无论配置如何，目录项、
单PNG和单轮哈希读取上限仍保留。命名导入不改变运行中的缓存策略；文件与缓存设置保存
主动合并当前两类偏好，保留未知回收站/扩展字段及旧schema。
`desktop/cache_input.rs` 拥有“临时缓存设置…”的内存事务；`cache_dialog.rs` 解析容量字节
和保留天数两个整数，`cache_dialog_template.rs` 只负责原生布局。精确保留非整MiB值，
统一校验后共享应用；取消/非法输入/模态期间变化或关窗保留当前值，不立即清理磁盘。
保存/另存/自动保存同时冻结命名和缓存策略，选路径及后台完成后均复核两者；缓存单独
变化也触发自动保存和退出等待。未知配置不被覆盖，自动保存仍默认关闭。隐藏HWND及
真实文件测试覆盖编辑取消/过期、字段保留、缓存单独自动保存及迟到保存拒绝；实际对话框
键盘和高DPI效果仍待人工验收。
`clipboard_cache_eviction.rs` 在保留期清理之后、同一排他锁内处理容量压力。仅当现存
总量加本次PNG/记录预留超过软配额才读取候选记录并按创建时间排序；目标为配额一半与
本次输出可用余量两者的较小值。删除前重新比较完整记录，再复用身份/摘要/同句柄核验。
零配额禁用压力淘汰；单次输出本身超限在任何目录操作前失败。未知/修改/占用/未来时间
文件不盲删；释放不足仍明确失败。压力与保留期清理共享每次512 MiB核验预算，候选索引
最多4096项，未超配额时不为压力淘汰读取记录。原始缓存文件可能在保留期前因容量淘汰失效。
其 `file_naming_allocate.rs` 子模块拥有输出文件分配：显式路径通过 create_exact 原子拒绝
覆盖；递增 API 按原名、_2 至 _9999 尝试，仅 AlreadyExists 可重试，支持分配前取消。
stem 清理后连后缀不超过 120 个 Unicode 字符，扩展名沿用主干 ASCII 规则。调用方拥有
成功文件和寿命；分配器不创建目录、不清理用户文件、不授予目录访问权限。PNG 显式导出
已复用 exact 分支，递增分配已接默认 PNG 保存和剪贴板缓存发布，拖出生产入口仍待接入。真实临时目录并发
回归验证不同写入者不覆盖；耗尽/权限错误/取消通过注入创建函数覆盖。
`desktop/export_default.rs` 拥有“保存 PNG 到默认目录”的快照和目录选择；输出为原生
profile 下的 saved 子目录，支持 OTTER_NATIVE_PROFILE_DIR，--settings 不重定向图片目录。
UI 只冻结已提交显示快照/命名/透明度；复用 export_job 的单任务、完成检查和退出 join，
目录创建、WIC 编码、递增分配与写入在 worker。无图像/模态/已有导出时不启动；编码完成
才分配文件，不重复编码来重试名称。最终目录拒绝已有重解析点，但不宣称祖先竞态沙箱；
失败可留下本次创建的空目录或部分新文件，错误含部分文件路径，不删除其它文件。
`file_naming_sanitize.rs` 增量合并非法字符替换并限制 stem 为 120 个 Unicode 字符，保留
主干尾空格/点处理及设备名规避。模板限 4096 UTF-8 字节、上下文字段各限 64 KiB，超限
明确失败，不分配完整展开字符串。此容量准入比旧实现严格，设置导入时须显式报告，不能
悄悄截短模板。标题长度仍为 1–240 字符；额外规避 CONIN$/CONOUT$、COM0 和上标数字
设备别名，与原生受管路径的保守准入保持一致，这比主干清理规则更严格。
`file_naming_settings.rs` 定义主干兼容字段/default/校验；`naming_input.rs` 提供当前运行的
文件命名设置菜单；`naming_dialog.rs` 拥有保存/拖出/剪贴板三个模板和标题长度的有界草稿、
统一校验及确认/取消，`naming_dialog_template.rs` 只负责 Win32 布局。取消/错误保留原值，
模态期间配置变化则拒绝旧草稿。字段统一应用并复用配置保存链路；拖出消费仍待接入。Application
拥有共享设置，新建和恢复窗口沿用同一 Rc，不将应用偏好混入工作区 dirty 凭证。
`desktop/naming_context.rs` 对齐主干 `buildUnitFileNamingContext`：label/title 来自所选能力
名称（trim 后），process 是 artId，不是 Windows 进程名；无名称时 label 回退 art/image、
title 留空。PNG 建议名现使用这组字段以及 ID、短 ID、实际尺寸、kind 和时间，titleMaxLength
实际约束能力标题。上下文借用不可变能力目录，不克隆参数/图像或额外查询 OS。会话恢复
将原 artId 带入离线状态，能力不可用仍可用于命名；新选择优先于旧 ID。只复制不超过
64 KiB 的恢复 ID，超限保留标记并拒绝建议名，不截短身份或影响原始会话数据保留。
模板编辑与显式导入影响当前运行，可另存文件但不自动读写主干配置；拖出模板字段尚无消费入口。

`native/src/session_coordinates.rs` 拥有根扩展 `otterNativeCoordinateSpace`，version=1、
origin=[i32,i32]、scale=有限正数。缺省表示旧格式；显式 null、错误类型及未来版本拒绝。
原生完整保存将冻结 Space 写入扩展；`session_document` 暴露解码结果。扩展未知子字段
移动保留，不复制大值；通用旧写入接口不自行猜测坐标。此元数据不授予资产路径权限。
恢复候选携带已验证 Space：调用方显式值优先（允许有意重定位），否则选文件元数据；
两者都缺省时，可见恢复提供原生坐标输入，隐藏检查则拒绝，不猜当前显示器 DPR。
CLI 原点/缩放必须同时提供或同时省略。
选择发生在创建候选 HWND 之前，恢复记录保存实际采用值。
`application_open_session.rs` 拥有“打开工作区…”菜单的有界请求、文件框与替换确认；
Application 在模态返回后复核完整恢复 Guard，再启动原有恢复 worker。读取失败不替换
原工作区，忙碌时拒绝新操作。普通文件选择不授权外部资产目录；旧格式仍可用 CLI 显式上下文，
`session_coordinate_dialog.rs` 为旧会话提供空默认值的原点 X/Y 和缩放输入，每项最多
64 个 UTF-16 单元，整数/有限正缩放须通过同一几何准入。输入取消释放候选、保留原工作区；
模态前后复核来源与能力目录，只有有效输入才进入建窗。未保存替换确认提供保存/放弃/取消；选择保存时只保留一个目标路径，
由正式保存结果一次性消费。保存取消/失败、后续编辑、新操作或退出均清除继续打开意图；
成功且状态仍有效才启动恢复。隐藏回归只注入选择/提示出口，真实读写/解码/建窗仍执行。
“打开工作区（指定图片目录）…”由 `session_asset_dialog.rs` 另行选文件系统目录，立即
建立 `session_asset_root::Root` 句柄。只准入该目录的直接子图片，不递归授权，不从 JSON
推导权限。保存后打开的 Target 拥有同一个 Root，取消/失败/新意图丢弃时自动释放；正式
恢复直接移动到 worker，不按路径重新创建授权。COM 路径缓冲通过 CoTaskMemFree 回收。
`native/tests/session_assets.rs` 使用真实隐藏 EXE 和独立 linshi 合成 PNG/会话，覆盖无授权、
正确目录、错误目录、同前缀兄弟目录、子目录、路径穿越及损坏图片；失败不得回退 filePath。
每次运行检查源字节不变，成功路径实际绘制并重建设备；退出后验证图片/目录可移动，并
明确清理登记的测试文件和目录。它不操作真实目录框，也不代表可见导入流程已经验收。

`desktop/application_save_ui.rs` 拥有工作区另存为请求、原生路径对话框、完成消费和
结果提示。窗口回调只置有界请求标记并唤醒线程，不在回调里借用 Application 或写文件。
对话框前后核对工作区凭证，不跨模态保留编辑器快照；后台结果只能在非模态消息循环中
复核并提交。另存为不覆盖已有路径；预检查只是用户提示，最终磁盘提交仍必须防止竞争。
`application_save_context.rs` 为新工作区建立 revision=0 的空基线和原生桌面物理像素
空间（origin=0、scale=1），不以某个窗口的 DPI 代表整个桌面。导入会话仍使用导入时
指定的空间；不能用新建默认值覆盖历史上下文。初始空宿主只在未编辑且无关联时省略。
菜单“保存工作区”在 saved_path 存在时调用替换准备，否则转到另存为；恢复副本不取得
默认覆盖路径。另存为始终选新文件，同批请求中另存为优先。分组锁允许两种保存命令，
但不解除编辑锁；Ctrl+S 仍为导出 PNG。可见对话框与完整桌面交互验收待补。

`native/tests/support/process.rs` 是原生启动和会话启动回归的子进程 owner：运行期间
并发排空 stdout/stderr，每路最多保留 256 KiB 诊断但持续读取超限输出，不能先等待
退出再读取管道。超时仅终止自有进程并回收 reader，错误附带已收集输出；本接口仅用于
不派生继承管道子孙的测试命令。`tests/process_runner.rs` 独立覆盖大输出与超时诊断。

`native/src/session_file_guard.rs` 拥有覆盖准备的只读来源文件句柄，拒绝末级重解析点，
仅共享读；核对完整字节而非仅修订号/长度，按固定 8 KiB 缓冲观察取消，提供句柄文件
身份。租约存续期间阻止普通写入/删除共享；释放后不再有此保护，不能先释放再 rename
便宣称实现原子条件覆盖。来源句柄由替换事务持有，事务准备已经进入保存 worker。
语义依据 [CreateFileW sharing mode](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew)。
显式 `Movable` 类型额外请求 DELETE 权限，普通 Guard 仍只读；`session_file_rename.rs`
使用该既有句柄执行不覆盖重命名，不关闭后再按来源路径寻找文件。原始 Win32 可变长
结构体只由该模块构建，路径缓冲有界，目标已存在时拒绝。此能力是恢复协议的底层组件，
不是原子覆盖：旧路径到恢复路径的移动与新内容发布之间仍存在间隔。
`session_file_replace.rs` 组合目录句柄、基线 Movable 和 Staged：准备期持有父目录以阻止
目录重命名，核对来源修订及完整字节，先移开旧文件再以“不覆盖”方式发布新文件。
失败尝试原句柄移回；若竞争者已占用目标，则保留双方文件并返回恢复路径，不强制覆盖。
成功后只通过旧句柄标记退休，不能关闭句柄后按路径删除；退休失败返回“已发布但保留
恢复文件”，不得冒充保存失败。panic 尝试不覆盖恢复，崩溃后的自动识别/恢复仍待实现。
该层已连接应用准备/提交接口及保存菜单；两个 rename 不是原子替换，
不要隐瞒目标路径短暂缺席或断电边界。
`session_file_trace.rs` 仅在显式 smoke timing 下观察 guard/write/replace 的原操作；
不记录路径、内容或错误文本，不参与文件身份/准入/取消或持久化策略，输出失败被忽略。
默认直接执行原 action 一次，无新线程/队列；不能以计时结果放宽完整比较或同步。
`application_save_disk.rs` 用排他枚举区分新文件和替换准备，保存 worker 只复制原基线
共享引用，编码后使用该引用核对磁盘来源，不能用新候选冒充旧基线。应用提交统一检查
Candidate 绑定和 Receipt，成功后再交换基线；失败保留旧基线并传递恢复路径，旧文件
退休失败保留“已发布”的成功状态和告警。request_save_existing 只用已提交 saved_path。

`session_file_recovery.rs` 只在显式目标缺失时有界扫描同目录（最多 4096 项），只接受
精确目标名前缀、GUID 和 .json 后缀的单一普通文件；歧义拒绝，不排序猜测或改写文件。
`desktop/session_file::read_recovering` 继续使用原有只读/非重解析点/大小上限准入，原目标
重新出现时优先读取原目标，存在但格式错误/访问失败不回退旧副本。后台 Candidate.origin
携带请求路径与恢复来源；正常导入保留 saved_path，恢复读取则 saved_path=None，保留
recovery_path 并提示另存为，不能把恢复文件悄悄当成覆盖保存目标。
`session_file_crash_tests.rs` 在独立测试子进程的四个事务边界直接退出，跳过 Rust Drop；
父进程使用有界 runner 观察退出码，再验证正式/恢复/暂存文件完整内容和句柄已回收。
探针仅 cfg(test) 编译，检查规范化路径属于唯一 linshi 测试目录，不给正式 EXE 增加
故障注入命令。该证据不等于断电持久性，也不证明遗留暂存文件已实现自动清理。

`application_save_state.rs` 拥有保存后变化凭证：仅保留编辑器身份/版本、正式端口OutputStamp、配置版本、
几何/DPI、坐标空间、视口、缩略/透明度及工作区拓扑，不持有像素或完整 Editor。正常文件导入
和成功提交建立凭证；失败不改旧凭证，新建/恢复副本仍未保存。按需查询，不新增常驻
轮询；模态/热键不影响状态，未提交编辑手势保守视为有变化。它不是内容相等判定，
撤销回到旧内容可能仍显示有变化；尚不能替代完整窗口关闭确认、状态 UI 或精确 dirty 模型。
恢复发布先清除旧凭证，coordinator 安装完整 Space 后才建立普通文件基线。
`application_commands.rs` 负责应用级菜单定义、命令路由和退出请求排队；Application
保留生命周期 owner，菜单拆分不改变命令编号、快捷键或请求消费顺序。
`application_exit.rs` 拥有应用级“退出 Otter”的保存/放弃/取消决策。默认取消；模态期间
不释放窗口，放弃前复核状态凭证，保存成功且状态仍未变化才排队继续退出。路径选择取消、
保存失败或后续编辑清除退出意图，不能让未来手动保存意外退出。此入口不覆盖单窗/分组关闭。
`application_exit_save_smoke.rs` 使用既有 linshi 合成会话，经过正式保存 UI coordinator、
worker 和提交验证保存后退出请求及失败保留；只替换通知出口，不替换保存结果。完成时
若还有新保存请求，先取消旧自动退出再消费新请求；不能在尚有用户操作时无条件退出。

`native/src/session_file_write.rs` 拥有同目录临时文件和新文件提交，不覆盖已有目标，
不把 revision 数字当成来源文件锁。调用方必须在提交前重新核对 UI 凭证；清理仅限
本对象 create_new 成功创建的临时路径，不扫描目录。测试只在 linshi，既有文件替换
必须另行设计冲突检测，不能给 commit_new 加覆盖标志了事。接口语义参考
[Microsoft MoveFileExW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw)。
Staged 必须与实际编码 Candidate 的共享字节 owner 和修订信息绑定；不能用字节长度或
修订号替代身份。`application_save_publish::Prepared` 保留 UI Receipt，在最终提交前
重查绑定与工作区状态，校验到 rename 之间不能泵 UI 消息。该层在提交成功后同步
交换保存基线；当前真实回归仅写 linshi 内的合成工作区。
`session_source::Source` 统一导入原文与已编码基线的共享所有权：导入保留 Vec allocation，
保存基线保留 Pixels 的预算租约，不复制大 JSON。Receipt 用同一 owner 身份判定基线，
不能以内容相同代替；只在磁盘提交成功后替换 Record.original 并更新 saved_path。

`native/src/session_write.rs` 的最终候选缓冲复用 `Pixels` 的共享字节/租约 owner，
先计数并预留容量，序列化不得扩容；不能将裸 Vec 移出后提前释放租约。此处复用的是
字节生命周期能力，不引入图像解码或像素语义；中间 JSON 树的开销须另行审计。
`session_write::WindowCandidate` 将新编码字符串的预算与窗口字段一起交付给
`assemble_owned`，提前失败时也必须先释放 JSON、后释放租约。保存图片的
`InlineRasters::install` 拥有 src、previewSrc、rasterizedAnnotationLayerSrc，以及新 PNG
对应的 imageEditState.flippedX/flippedY 清零：输入像素已物化翻转，不能保留旧标志。
它替换资源及清理过期引用，不推断源框、裁剪或标注坐标；其余状态保留，坏结构拒绝。
`Document::encode_images` 只复用 UI 捕获已验证的 restored_source；不能通过旧 src 或
文件名判断原图仍有效。无有效来源时编码当前底图，标注始终编码当前独立图层，不能
为了保留原图而复活旧栅格标注。
实际内容候选统一调用 `Data::encode`，由它持有后台 COM 生命周期并串接现有字段与
资源编码；`Data::assemble` 保留为底层构建入口，不能将测试的空内容回调用于正式保存。
注意 `VectorCandidate::prepare` 安装的是可见框栅格标注，效果计算用的完整源框投影
不是编辑器图层；写出前重复裁切会改变内容。该组合通过实际恢复/合成往返验证。
`application_save_job.rs` 拥有单个编码/暂存线程、取消和完成通知，绝不携带 UI Receipt。
显式提供另存为路径时，同一任务继续生成 Staged，不在 UI 线程执行整份文件写入和
sync_all；完成后一起交付 Candidate/Staged。取消或丢弃完成结果会释放暂存 owner，
不发布目标。最终提交仍须 UI 线程核对 Receipt 和候选绑定，不能在后台自动 rename。
`application_save_control.rs` 拥有 Application 单槽及交付前凭证复核，候选取走后槽清空；
恢复与保存准备不能并发。任务 Drop 会取消并等待不可强制打断的原生调用结束，通知
必须先于应用资源退休停止；协调层不执行最终提交，也不宣称暂存完成就是保存成功。
`desktop/application_save_assemble.rs` 从冻结 Data 构造当前拓扑并消费 Document，统一
先合并窗口几何和已绑定 Art 参数，再委托内容编码；不从原会话推断当前节点清单。
临时宿主省略只适用于仍为空且无编辑/关系的初始窗口。调用者必须实现完整内容编码，
不能把只合并几何的测试回调接入磁盘保存入口。
`desktop/application_save_vectors.rs` 拥有冻结普通窗口的 annotationState 安装，复用
`session_vector_write` 坐标映射及恢复端 DirectWrite 基线测量；不修改活动 Editor，
不处理图片/裁剪，不允许缩略双层上下文冒用普通视口。此阶段编码前后检查取消，
单次字体原生调用本身不可强制打断。
缩略标注改用 `Mapping::from_annotation_frame`，目标尺寸取已写回的 savedRect 或当前
窗口框，cropOffset 不参与这一步；不能把缩略 HWND 小视口直接序列化。非等比变换
目前拒绝，不能取平均比例或静默拉伸标量样式。
后台标注保存必须调用 `encode_mapped_cancellable`，不能仅在整个编码前后检查取消。
点集遍历、坐标映射和 JSON 输出流共享取消观察函数，取消返回专用错误、不发布部分
字节；观察函数只读取消状态，不承载编辑或外部副作用。
`native/src/session_crop_write.rs` 仅拥有 imageEditState 中 cropRect/sourceSize 的写回
和重置，消费 Crop 的源逻辑单位，不按 HWND/DPI 再缩放。保留其余字段与嵌套未知扩展；
不能据此宣称图片翻转、擦除、边框已正确物化，这些仍归内容编码 owner。

`desktop/canvas_minified.rs` 拥有恢复缩略视图的分层绘制和共享原图引用；只消费
`session_minified` 的双视图几何，不改变文档、撤销历史或导出。呈现绑定文档世代，
提交变化后不得复用旧原图。命中、交互切换及外部显示的取景由各自宿主路径接入，
不得用这条本地绘制路径的测试替代完整缩略交互验收。
`desktop/hit_region_minified.rs` 消费同一缩略呈现的原图和双层几何，分别变换后台
alpha 缓存及矢量命中边界，再裁剪到物理客户区。GDI 区域只在窗口线程创建，
SetWindowRgn 前释放状态借用，成功才转移句柄所有权；缓存键包含取景几何。
`desktop/pin_minified.rs` 拥有已恢复缩略 HWND 的展开行为，消费批次准备的初始缩略框、
展开物理框及正常透明度。移动位移只影响屏幕位置；不改文档坐标，不从目标显示器反推
旧会话 DPR。Win32 尺寸变更前释放所有状态借用，失败时恢复原呈现状态。
恢复批次须为所有缩略节点安装展开事务；缺少 savedRect 时展开目标就是当前框，不应
跳过安装导致节点无法退出缩略。`application_restore_unsaved_mini_smoke.rs` 通过真实
隐藏 HWND 双击消息验证该回退，不猜测原图尺寸，不改编辑器或会话原文。
`Editor::with_restored_size` 为新恢复文档安装精确逻辑框，必须与已装栅格缓冲的 ceil
尺寸一致；不伪造裁剪，不产生历史，不允许覆盖活动文档。普通与裁剪恢复都须保留
逻辑尺寸，显示、导出和后续视图切换不得把像素缓冲尾部留边当作内容。
`session_minified::View::document_mapping` 统一缩略标注从文档逻辑框到显示框的轴向比例
和偏移；D2D 呈现与 GDI 命中共用，不各自假定文档单位等于当前窗口单位。
`desktop/pin_minify.rs` 拥有普通钉图双击缩略的物理几何和呈现切换，保留展开框与两套
透明度；不调整文档、重编码图像或追加历史。恢复原图必须绑定文档凭证，不能在编辑
变化后继续显示过期来源；窗口尺寸变更失败须撤回本次呈现切换。
`desktop/canvas_minified_source.rs` 维护恢复原图与已投影底图的关联，按文档身份、底图
像素身份/尺寸及完整逻辑框验证，不把矢量/独立标注版本变化误判为底图替换。
`sync_raster_source` 负责退休跨文档引用；底图变化只停止采用来源，保留每文档至多一份
关联供撤销重新验证，不随历史操作累积。不复制像素或额外持有栅格标注层。
`desktop/pin_toggle_guard.rs` 是缩略与展开共用的切换准入，复用编辑任务 busy 判断并
核对显示接管、执行与未结算交互。拒绝只返回，不取消、取走或重排工作；切换准入
不替代缩略状态下各编辑入口自身的交互规则。
`desktop/pin_minified_input.rs` 拥有缩略窗口的指针/键盘路由边界，保留拖动、展开和
只读输出；`display_edit` 统一拒绝缩略文档编辑准备，基础菜单在 execute 入口复核。
不能仅禁用控件外观而让快捷键或重入菜单绕过规则。
`native/src/editor_annotation_id.rs` 拥有标注的稳定持久身份：不可变 Arc 字符串随对象、
历史和快照共享；新标注和用户复制生成新 GUID，预览不能生成新 ID。恢复安装只用于
未发布且无历史/草稿的新文档，先检查数量、唯一性与文本预算。后续持久化须从实际
Stroke 读取身份，不从恢复 Record 的初始索引映射推断已编辑对象身份。
`native/src/session_vector_write.rs` 负责标注内存序列化和按稳定 ID 合并未知字段，输出
有界主干 JSON，不访问文件、不修改编辑器、不推断窗口坐标。平台字体基线通过回调
提供；不能无损表达的样式返回错误。整会话 owner 必须负责源框/裁剪坐标映射、资源
引用和保存事务，不能将这个标注片段接口当成完整会话持久化。
`session_vector_write_frame.rs` 负责源框到可见框的写出平移，返回配套 frame/CropView；
不改宽高、线宽或未知字段，不按当前 HWND 尺寸隐式缩放，不裁掉框外标注。
坐标转换发生在本次输出候选上，失败不发布部分结果或修改活动编辑器。
`session_vector_write_mapping.rs` 拥有普通画布视口到会话局部单位的相似变换，显式消费
Viewport、DPI 和 Space.scale，不加入屏幕原点。几何与样式同步缩放，文字基线由字体
owner 重新测量；不可无损表达的值拒绝，不静默钳制。不得将缩略窗口双层视口直接套入
普通映射，也不得在映射后重复减去裁剪原点。
`minified_transition.rs` 拥有缩略展开事务、拖动后的目标计算和 savedRect 持久化来源。
恢复入口从原字段存在性设置标记，主动缩略设置真实展开标记；禁止按两框相等推断。
`application_save_snapshot::Document::geometry` 复用该目标计算并拒绝不一致的呈现/事务，
只产出几何候选，不改变窗口，不推断缩略图片与标注的坐标转换。
`session_placement_inverse.rs` 拥有物理框到逻辑框的反变换与取整余量保留，结果须通过
正向物理转换校验。`session_window_write.rs` 只合并窗口几何/透明度，消费 JSON 对象
所有权以保留图像字符串存储；不解析参数、重编码图片或写文件。捕获 owner 必须提供
实际拖动后的展开框，无 savedRect 时不得把运行时回退框当成真实保存框。
`desktop/application_save_snapshot.rs` 拥有活动工作区到保存候选的快照边界：UI Receipt
保留完整校验凭证，Data 仅包含可传 worker 的模型/共享字节及像素，不持有 HWND/Rc。
调用者必须保留 Receipt 并在提交前复核；不得把快照捕获成功等同于保存成功。
恢复 Record 共享原文 Vec 的所有权，保留显式 Space；不得从当前显示器猜测来源坐标。
`native/src/session_write.rs` 拥有整会话内存合并、schema/修订和输出预算；窗口内容通过
有界编码回调提供，拓扑字段由已校验 Topology 决定。未知字段按 ID 保留，不用最小拓扑
序列化覆盖整个文档。候选必须经过正式 decode 回读；expected_revision 不是文件锁，
原子磁盘 owner 仍须拒绝过期来源。回调不得写磁盘或提交其他外部副作用。
`desktop/export_raw.rs` 只将原始像素经 WIC 编码为 PNG，不复用成品图合成；WIC 像素
副本和输出字节有独立预算，取消后不得交付。`application_save_images.rs` 将保存快照
的底图/独立栅格标注层分别编码，保持各自尺寸，不裁剪、叠矢量或烘焙窗口透明度。
图像资源封装 owner 必须区分完整源图、可编辑底图和预览，不能把这组 PNG 当成已完成
的会话文件，也不能在此调用系统剪贴板或写磁盘。
`loom_base64::encode_png_data_url` 复用已有取消/预算循环，在一次分配中编码前缀及
Base64；纯 Base64 协议入口保持原行为。`Rasters::into_data_urls` 消费 PNG owner，
保留带租约的文本，失败释放已完成层。整会话 owner 仍须检查所有 URL 的聚合预算，
不能仅凭单资源通过就绕过 64 MiB 总输出上限。
`native/src/session_art_write.rs` 从已验证 Parameters owner 和执行配置构造已绑定 Art
保存字段；不能把旧 params 合并回来。未设置项省略、null/禁用标记保留。未知执行
配置仅在同能力 ID 时保留，更换能力不能携带旧扩展。未绑定离线节点由更高层保留
原文，不在此猜测 secret 策略或生成可执行能力。
保存 Data 中的 viewport/dpi 是当前呈现上下文，不是原会话 Space 的替代品。保存与
恢复共用的窗口凭证同时观察视口和 DPI；Receipt 另行核对来源 Space。后续坐标编码
必须区分 Editor 单位、窗口 DIP、物理像素和会话逻辑单位，不能将某一显示器 DPI
直接当作历史会话比例，也不能忽略仅视口变化造成的过期结果。
`open_image::can_replace` 统一当前文档的替换准入，打开/粘贴在读取前检查，后台启动
再次检查，提交凭证拒绝缩略文档；新钉图导入不修改来源文档，不应被同一规则禁用。
`Presentation::for_display` 从已有缩略几何与外部 Frame 准备仅显示的呈现，绘制和
命中共用；禁止把 Frame 图片装入本地编辑器。命中缓存持有 Frame 身份直到失效，
不能只用像素指针与 revision 判断不同外部标注快照相同。
普通外部显示宿主可使用相同窗口切换入口；窗口状态记录本地文档与展开位置，外部
取景临时派生自 Frame，不因缩略而触发显示接管或创建本地底图。
`desktop/minified_display_region_smoke.rs` 归属隐藏宿主启动验证，使用合成透明图和
真实 GDI 区域查询覆盖外部快照身份失效与断连回退；临时 HRGN 由局部 owner 释放。
`tool_panel_availability` 同时观察本地底图与外部显示是否可输出，缩略时只保留独立
新建与只读输出控件；不因判定可用性而接管图片。弹窗入口仍须复核，不能依赖旧面板状态。
`desktop/canvas_minified_crop_tests.rs` 用真实 WIC/D2D 像素验证裁剪、独立标注和效果
纹理的组合映射，效果标记纹理与卷积算法测试分离；不写用户图片或持久化会话。
`native/src/session_vector_color.rs` 只解析原生 RGB 可无损表示的会话颜色文本，最多
128 字节且无临时字符串分配；不能把不支持的颜色 alpha 丢弃后当作成功迁移。
笔画与填充共用该入口，标注 opacity 仍由样式 owner 独立恢复。

`graph_output.rs` 的 `Output::from_json` 为已解码 JSON 提供不复制容器的借用视图，
包装值及结构预算仍由图层统一管理；参数与协议消费者不得自行复制一套不同的解包规则。
`graph_canvas_display.rs` 拥有画布显示引用优先级，不修改底图、像素或编辑历史。显示与
正式执行来源必须分离，不能让 Art 预览进入正式输出链，也不能把显示回退当作重新启用中继。
`desktop/display_composition.rs` 拥有后台显示像素合成：复用导出渲染，直接返回受预算
约束的共享 PBGRA，不做 PNG 往返，不跨线程传递 COM，不在 UI 绘制回调执行。
`desktop/application_display_composition.rs` 拥有应用级单 worker、完成通知及来源版本
缓存；断连/关闭清除不再需要的像素，取消旧任务，不积累任务队列或常驻定时器。
`desktop/capability_parameter_menu.rs` 拥有公共标量菜单快照及提交准入；不泄露 secret，
不将旧菜单索引映射到新目录，修改继续使用公共参数 owner 与既有执行取消/触发策略。
`desktop/parameter_value.rs` 拥有数字/文本输入校验，`parameter_dialog.rs` 拥有原生单参数
模态窗口和局部输入缓冲；不得跨模态循环持有 State/Canvas 借用或提前提交候选。
`desktop/parameter_number.rs` 对齐主干有限数字解析、范围限制和十位小数规范化，
增减步进复用同一转换，不引入只存在于原生端的 integer/required 限制。
`desktop/parameter_slider.rs` 拥有系统滑条及有界位置到数值的映射；子控件由对话框
销毁，不自行创建工作线程或定时器，不把拖动草稿直接写进文档参数。
`desktop/parameter_action.rs` 区分直接赋值、文本/数字输入和颜色选择候选；统一模态
guard，返回后仍由菜单快照核对身份。`color_dialog.rs` 是工具与参数共用的系统颜色框
资源 owner，只返回 RGB，不持有文档，也不写配置。
`native/src/parameter_file.rs` 拥有文件参数的有界流式 Data URL 编码，供后台任务调用；
固定读取缓冲区，不解码图片、不上传、不把路径写入结果。提交时仍须由公共参数 owner
验证总预算，不能将单文件读取上限当作整批参数已合法的证明。
`desktop/parameter_file_input.rs` 拥有文件框及待处理请求；`parameter_file_job.rs` 拥有
单文件 worker/取消/回收。`application_parameter_files.rs` 统一调度并复核拓扑和参数
身份后提交；不得跨线程传递 HWND、State 或 COM 对象，不允许过期文件覆盖新配置。
`native/src/loom_parameter_disabled.rs` 拥有公共参数禁用/启用及有界原值暂存；暂存不作为
有效参数导出，候选预算失败不得修改 revision，secret 不得进入此 owner。
`desktop/parameter_display_value.rs` 仅处理参数显示回退与控件值转换；不得把显示初值
直接回写文档，也不得取代执行图层的连线参数解析。
`desktop/application_parameter_views.rs` 复用 `application_parameters.rs` 的借用图解析
生成有界显示值；`parameter_view.rs` 拥有显示值、连线锁定及快照身份。菜单不得把
这份解析结果写进手动参数，解析失败也不得自动解除连线的编辑锁定。
视图仅持有连线参数值；未连线手动值直接由参数 owner 提供，没有参数连线时跳过图
解析，避免把上传文件 Data URL 复制为第二份常驻显示数据。
`desktop/parameter_view_values.rs` 在复制连线显示值前计量 JSON 字节、节点和深度；
不得先 clone 大输出再做预算验证，拒绝显示副本也不得解除原连线锁定。
`native/src/workspace_session.rs` 拥有主干会话拓扑字段与原生工作区的候选转换，不拥有
完整会话或磁盘写入；禁止用拓扑切片覆盖旧会话，其他资源验证完毕前不得交换运行工作区。
`native/src/session_document.rs` 是主干 SessionData 的只读有界解析入口，借用原始字节，
验证版本/revision 与拓扑；未知业务字段留在原文，不提供降级覆盖或自动迁移写入。
`native/src/session_window.rs` 拥有持久窗口元数据的缺省/合法性检查，不创建 HWND 或
自动钳制坐标；显示器映射和创建窗口必须由后续桌面恢复事务统一处理。
`native/src/session_assets.rs` 拥有可借用的会话图片引用读取与预览去重，不自行读取文件、
访问网络或启动解码；原始 JSON 的生命周期由会话调用者持有，不复制大 Data URL。
`desktop/session_image.rs` 拥有会话内联图像的后台解码入口，复用
`loom_output_inline.rs` 的借用 Base64 解码和共享像素预算；拒绝路径/URL 隐式加载，
不创建工作区窗口或自行发布解码结果。
`desktop/session_image_file.rs` 仅消费已准入的 File 句柄，计量编码缓冲、检查读取边界，
释放句柄后解码；不负责路径授权或自动加载，也不允许 WIC 按路径再次打开文件。
`desktop/session_prepare.rs` 拥有单批会话图像候选和引用去重缓存，失败不返回部分结果；
文件打开通过显式准入回调，禁止用 original filePath 作为失败资源的隐式替代。
`desktop/session_asset_root.rs` 拥有显式受管目录句柄和直接子文件准入，拒绝设备/穿越/
重解析点并核对最终句柄路径；不得由会话内容自行扩大根目录，不替代外部资源授权 UI。
`desktop/session_candidate.rs` 拥有后台恢复候选（原文/元数据/拓扑/像素），不得丢弃
未迁移字段后直接发布；`session_job.rs` 仅拥有单个 worker 的取消、完成通知和回收，
应用级并发准入及版本凭证仍由后续桌面恢复 owner 负责。
`native/src/session_parameters.rs` 按已解析能力恢复会话公共参数，先检查原文预算、再
解析和过滤 secret/控制字段；`loom_parameter_restore.rs` 将未知公共参数纳入统一预算
并移入独立保留区，执行透传但不暴露为声明控件或端口。缺失值由执行解析器
回退能力默认值，不能把显示回退写进手动状态。
`native/src/session_art.rs` 将会话 Art 的公共参数与执行配置一起绑定到同一能力 ID，
只返回候选；不发请求、不安装窗口。`session_art_batch.rs` 按主干精确 ID 规则绑定
同一目录中的能力，缺失能力返回未解析身份，其参数继续保留在原文；任意已匹配节点
恢复失败则整批拒绝。`session_candidate::Candidate::resolve_arts` 提供桌面候选入口，
应用安装前仍需复核目录快照，不可将索引跨目录版本使用。
`desktop/session_binding.rs` 持有完整会话候选、共享目录和整批解析结果；释放给后续
窗口事务前检查目录 Rc 身份与取消状态。重绑定必须重跑参数过滤，失败保留旧 owner。
`desktop/session_candidate_results.rs` 将 Art 的恢复显示图转为 Idle 结果 owner，像素
共享但正式端口为空；Bound 将其与原始候选一起交付，不将钉图底图当作 Art 输出。
`desktop/session_raster_editor.rs` 将明确的 source 与独立栅格标注纹理恢复为 Editor
初始图层，复用正常擦除/撤销事务；不得以 preview 替代源图。无源图覆盖层的坐标框
和矢量/图像编辑状态尚未接通，不能仅凭该栅格入口发布完整恢复窗口。
`native/src/session_crop.rs` 借用 imageEditState 并提取逻辑 cropRect/sourceSize 和翻转
标志，复用 crop_geometry 计算 contain-fit 放置与重置框；不以自然像素替换逻辑坐标。
`desktop/session_image_flip.rs` 在后台准备阶段按持久标志翻转 source/preview，
相同引用和方向共享结果；已持久化的栅格标注不重复翻转。交付像素已应用翻转，
后续绘制不得再按裁剪元数据中的原始翻转标志重复变换。
`session_vectors.rs` 与 `editor_restore.rs` 将基础形状/路径恢复为新的 Editor 初始状态，
保留身份顺序和密集点，不经过鼠标采样或制造导入历史。未支持的类型明确拒绝，原文不变。
`session_vector_text.rs` 负责主干文字基线/序号圆心到原生布局框的转换，平台通过
restore_with_text 注入字体测量；模型层不依赖 COM，也不凭字符数估算字宽。
restore_with_resources 另外注入模糊纹理提供方，必须返回当前源框及精确强度的资源；
模型验证强度并共享像素，不能以空纹理或旧文档结果补齐缺失效果。
`raster_projection.rs` 提供预乘 BGRA 的有界双线性投影；`desktop/session_effect_source.rs`
将恢复底图按逻辑框 contain-fit/裁剪，再合成独立栅格标注，作为效果资源准备的输入。
`desktop/session_blur_resources.rs` 按单窗口拥有惰性投影源图和精确强度缓存，Candidate
恢复路径通过它提供模糊纹理；没有模糊标注不做投影/卷积，取消时缓存命中也不能交付。
`session_job` 现在返回 VectorCandidate 阶段结果，后台完成图片、字体测量、矢量及
模糊资源后才通知；`session_binding::Bound` 接收该阶段结果并一并持有编辑器，
UI 绑定目录时不重新测量字体或计算模糊。该阶段仍不是完整编辑状态/窗口提交证明。
`desktop/session_states.rs` 将目录绑定后的整批恢复候选装配为未发布 State，
保留原文与标注身份；不承担 HWND 创建、工作区替换或持久化提交。
`desktop/document_restore_smoke.rs` 验证显式物理框的隐藏宿主创建和释放，
由启动 smoke 调用，不读取真实会话或安装全局输入。
`desktop/application_restore_window.rs` 将隐藏候选窗口绑定到应用共享状态，
但不加入活动文档列表或替换工作区；该文件同时保留此边界的隐藏 HWND smoke。
`desktop/application_restore_batch.rs` 持有整批隐藏恢复窗口与原始候选，
失败/取消自动释放而不发布；对应 `_smoke.rs` 使用合成目录和会话验证中途失败回收，
不连接实际 Loom，也不读取发现凭据。
`desktop/application_restore_guard.rs` 捕获来源工作区、文档和窗口的有界凭证，
拒绝准备期间的过期状态；它不代替后续异步任务收尾和整批发布门禁。
`desktop/application_restore_idle.rs` 集中检查恢复前的异步任务、待处理请求和手势，
只拒绝忙碌状态，不消费请求、不隐式取消用户工作。
`session_placement.rs` 按主干 CSS/DPR/覆盖窗口原点合同转换会话框，
只接收显式坐标上下文，不发现显示器、不猜测保存时的 DPI。
`session_minified.rs` 解析缩略窗口的图片/标注独立 viewport，保留主干留白与偏移规则；
不修改文档像素、不把缩略模式当作非破坏性文档裁剪。
`desktop/session_editor_layers.rs` 为恢复文档建立逻辑框栅格及裁剪效果源；VectorCandidate
将其与矢量 Editor 合并，保留独立标注层。with_restored_layers 仅接受未编辑的新文档，
不能绕过活动文档的撤销/版本事务。
`desktop/link_catalog.rs` 拥有跨窗口共享的端口目录与创建请求凭证；`link_create.rs`
拥有原生三下拉选择框，只排队请求，不在模态循环修改工作区或持有画布借用。
`desktop/node_menu.rs` 汇集打开时的能力/参数/连线菜单快照；`link_menu.rs` 只拥有
有界分页菜单和请求入队。`application_links.rs` 是拓扑修改的应用层 owner，必须
重新验证 Link 内容、拓扑凭证、端点和锁定状态，不在菜单重入循环直接修改工作区。
`desktop/application_display.rs` 将图显示引用映射为文档的独立 `display_frame.rs` 快照；
像素共享，本地标注用编辑快照保留，不能通过替换目标原图破坏撤销历史。版本缓存归
`application_display_cache.rs`，依赖拓扑、配置、编辑、正式结果和预览身份；不引入定时重绘。
`desktop/canvas_display.rs` 负责快照绘制，复用 Canvas 的窗口私有目标和缓存，不新建
第二套 Canvas 或替换编辑器。不得将显示快照的绘制能力等同于编辑/命中/导出已适配。
`desktop/display_snapshot.rs` 统一复制/PNG 导出的显示捕获与新鲜度校验；显示代际令牌
不持有像素。后台交付和模态保存框返回都必须检查该令牌与目标文档，不能只检查底图版本。
`native/src/editor_adopt.rs` 负责显示来源到可编辑文档的同步接管事务；通过
`edit_history::Document` 保存可交换的内容，不替换 Editor 或清空旧历史。点数预算包括
历史持有的矢量，淘汰/分支丢弃必须正确扣减；UI/异步接线另外承担来源与文档身份核验。
`editor_adoption.rs` 拥有暂存接管：保留原 Editor 的完整历史与重做分支，结束后恢复或
以一次文档事务提交。宿主必须持有 Adoption 直到草稿和异步编辑任务都结束；身份失配
时不能覆盖新文档。临时编辑的点数准入必须包含尚未释放的原历史。
`desktop/display_edit.rs` 拥有钉图开始编辑时的显示接管；不能在平移时复制编辑快照，
不能修改来源窗口，不能通过替换整个 Canvas/Editor 清空旧历史。工具处理必须在接管后
使用同一视口；尚未支持的 Art 输出编辑不得悄悄写入不可见底图。
`hit_region.rs` 与 `pin_window.rs` 必须使用当前显示快照的图片和裁剪信息，而非无条件
读取原图。`hit_region_annotations.rs` 只负责指定编辑快照的矢量标注区域；切换显示来源
须退休命中缓存，局部 revision 不能作为跨来源身份。
`loom_capability_runtime.rs` 规范化能力的本地预览/正式执行归属与默认执行策略；运行
入口必须通过已有执行门槛，不按 framework 名称推断支持，也不能把未实现预览转为后端请求。
`desktop/sticker_propagation.rs` 管理钉图接收上游图片的原生菜单；状态归文档所有，图适配器
读取，配置凭证核验。菜单不执行图计算、网络请求或线程等待，不能与 Art 自动触发开关混用。
`desktop/execution_settings.rs` 管理所选 Art 的节点级执行策略菜单；策略由 Selection
持有，初始化复用能力默认值，运行与传播读取该值。修改只退休旧配置，不自动启动，
且必须纳入异步配置凭证，不能只更新菜单显示。
`desktop/loom_input_data.rs` 拥有 Raster/Encoded 输入载荷与 worker 内的解码/编码转换；
UI 只能捕获有界内联引用，不能同步解码或隐式打开路径/URL。解码后仍须检查批次像素
总预算，不能用压缩字节长度代替展开像素的预算。

`native/src/desktop/loom_result_state.rs` 管理单节点已解码正式端口与显示预览，预览不写
正式端口，最终结果原子合并；只持有本地资源，不负责远端租约、文档版本或网络请求。
该 owner 的 `OutputStamp` 仅在成功提交正式结果时更新；图片来源快照共享正式端口像素，
不使用 display/预览，也不能用编辑器 DocumentStamp 替代正式输出版本验证。
`loom_result_commit.rs` 是任务完成报告到结果 owner 的同步提交边界，核验节点、请求与
文档凭证后移交已释放的结果，成功时结束注册请求；不能在此运行网络或嵌套 UI 循环。
`application_execution.rs` 在主循环领取清理已确认的报告，查找仍存在的文档结果 owner
并调用提交/失败收尾；不取走仍待远端清理的报告，不在窗口回调中等待网络。
`loom_execution_prepare.rs` 将图准备结果与已编码输入合成正式 Loom 执行消息，全部
校验和编码完成后才注册请求；它不做能力发现、图片编码或网络连接。
`loom_art_config.rs` 保存异步准备期间的不可变公共参数编码、禁用项和图片端口名，
不持有图借用、图片或 secret；旧借用编码入口与快照入口共用同一正式协议实现。
该模块同时接受已筛选的 `loom_parameter_state::Parameters`，将编辑值捕获为独立配置，
不得直接接受原始目录 defaults 或在编码时回读可变 UI；图片端口校验与图准备入口共用。
两条 Shader 快照入口在强制写入 mode/output_mode 前拒绝同名真实图片端口；普通 Art
仍接受这些图片名称，不能在公共端口校验中全局禁用，也不能丢图片、改名或取消强制模式。
`application_shader_mode_tests.rs` 是原应用 Shader 测试的子模块，复用隐藏 HWND/COM/
TLS 夹具，验证配置拒绝先于 worker/请求替换且保留正式输出，不建立替代执行 owner。
`application_input_preparation.rs` 按目标与 ticket 领取准备结果，将固定配置和输入合成
待执行请求；正式编码及新鲜度检查成功前不能修改请求注册表，不在配对阶段联网。
`desktop/configuration_stamp.rs` 保存文档/能力选择身份与参数版本，准备计划退休和结果
领取均核对该凭证；不得只比较局部 revision 或只依赖异步取消消息防止旧配置提交。
`desktop/execution_configuration.rs` 持有请求令牌对应的配置绑定，应用在结果消费前退休
过期请求；只清理匹配代际，远端取消/租约释放仍由执行任务 owner 处理。
`desktop/execution_start.rs` 负责运行菜单和待启动配置凭证；`application_execution_start.rs`
在主循环核验并启动输入任务，不能在菜单回调联网，不得为未解析的图片输入发送伪数据。
待启动请求显式携带触发类型；参数提交只排队最终变化，主循环应用能力策略和自动缺图
检查。手动缺图豁免不能传递给参数/上游自动触发，繁忙重调度也不得在 UI 回调等待。
`application_image_inputs.rs` 将文档与工作区连线适配为短生命周期图视图，只捕获本地
钉图或正式图片输出；不执行外部路径/URL I/O，不将预览当正式来源。异步阶段只保留
拥有所有权的来源快照和版本凭证，不能保留窗口状态的 RefCell 借用。
其子模块 `application_shader_inputs.rs` 只将公共 resolved 字符串逐字匹配为本工作区
Unit 候选，再从同一 Graph 的正式 output 选择图片；不拥有独立图、文件读取或图片
解析器。返回纹理名称及实际使用的 Unit，供原 owner 捕获准确的上游依赖；被真实
同名图片连接覆盖的候选不增加来源生命周期依赖。`application_shader_unit_tests.rs`
归原应用 Shader 夹具，保护参数/来源同轮一致性和正式图/过期/连接优先边界。
`loom_input_stamp.rs` 区分编辑文档、正式输出和节点复合依赖；中继依赖必须同时验证编辑
版本与能力选择/参数版本，不能因编辑器未变化而放行已改配置的来源。
`Workspace::upstream_units` 提供有界、去重的反向可达身份集合，供输入适配器收窄快照
及依赖范围；不能根据当前是否中继剪掉可能改路的上游节点，也不替代拓扑版本校验。
`application_parameters.rs` 在 UI 调度段借用上游正式输出，通过已有图参数规则捕获
不可变执行配置与依赖；不回写手动值、不联网、不把图像内存当作路径参数。准备批次
独立持有参数依赖，不能把“没有图片资源”等同于“没有上游依赖”。
`application_propagation.rs` 在正式结果提交后排队下游 Art 通知；`graph_notifications.rs`
负责穿过可用钉图中继的有界目标解析，复用正式图像来源规则，不修改显示或执行网络。
不得在结果 RefCell
借用期间回调其它节点，不将预览当正式完成，不递归执行网络。导入环及重复通知必须有界。
`execution_cancel.rs` 只负责原生菜单与取消身份快照，应用在回调返回后处理取消，禁止
在菜单回调中 join/联网；分组锁定不阻止该生命周期命令。
`loom_handshake.rs` 负责能力握手请求与有界响应解码，保留原始 Art 定义用于后续参数
适配，不执行能力或记录远端数据；目录中的 secret 声明仍须经过公共参数筛选才能执行。
`loom_handshake_exchange.rs` 仅在后台通过固定发现身份建立认证连接并获取目录，拥有
总超时、取消和消息预算；它不管理 UI 状态或自动重连。
`desktop/capability_job.rs` 管理单个目录后台任务，目录与发现身份一起交付；
`capability_catalog.rs` 管理会话共享目录和原生加载菜单，失败保留旧目录，不自动重试。
`capability_selection.rs` 管理能力选择菜单快照与文档选择；旧目录索引不能用于新目录，
切换只捕获旧执行取消请求，不负责参数准备或联网执行。
`loom_capability.rs` 提供目录的规范化借用视图，拥有默认值优先级、secret 声明及 widget
推导规则；不复制目录、不处理用户覆盖或节点初始化类型转换，输入预算归握手边界。
`loom_capability_defaults.rs` 负责节点公共初始值转换，按 schema/端口/参数的类型优先级
处理字符串；复用已有数字解析，未转换值借用目录，不负责编辑状态或执行注册。
`loom_capability_ports.rs` 负责端口规范化及与同名参数的可见性规则；完整输入与可见连线
端口分别暴露，不负责输入来源解析，不能把隐藏端口等同于无需执行输入。
`loom_parameter_state.rs` 持有单文档公共初始值和局部修改版本，预算检查先于克隆/提交；
不接受未声明或 secret 字段。能力选择先构造完整参数候选再取消旧请求，不在失败时清空旧状态。
`desktop/capability_parameters.rs` 管理高级 JSON 参数编辑快照和原生菜单入口，提交绑定
选择身份及参数版本；不得跨模态消息循环持有文档 RefCell 借用，不负责启动执行。
`loom_input_tasks.rs` 归应用所有，管理最多两个输入准备槽位及准备 ticket；完成未消费
也占槽，关闭只取消，退出先取消全部再回收线程。它不创建执行请求或修改节点结果。

`native/src/desktop/loom_execution_signal.rs` 归后台执行任务所有，负责资源释放后的
完成栅栏及通知，不持有窗口或业务模型；UI 消息唤醒和结果新鲜度验证仍归应用 owner。
该栅栏作为 desktop 内部模块供输入准备和执行 worker 共用，输入准备不得反向依赖执行
worker 的网络生命周期；完成通知回调必须短小且不可阻塞。
`native/src/desktop/application_wake.rs` 拥有合并后的线程消息通知及停用状态；应用在
读取后台任务前确认通知，析构时先关闭通知再回收任务，不把 HWND 或文档借用交给 worker。
`loom_input_batch.rs` 同时提供不持有像素的文档新鲜度凭证；执行集合保存凭证并在应用
轮询时退休已变更的依赖，完成报告保留凭证供提交前再次校验，不把请求代际当作文档版本。
`loom_input_stamp.rs` 区分编辑文档和正式结果来源；正式输出只验证结果 owner，不用
Editor 检查代替。正式图片编码不再次应用来源窗口标注，最终提交前避免借用目标结果时
回读其自身依赖导致 RefCell 重入。

`native/src/desktop/application.rs` 拥有文档窗口集合和共享消息循环，新建请求在回调
返回后合并处理；先回收已关闭 owner 再创建新 HWND。文档模型不得反向拥有应用集合，
窗口创建/销毁不能跨越尚未释放的 Canvas 借用。新建工作窗口不是主干工作区对象模型。
`workspace_id.rs` 定义独立于 HWND/文档编辑版本的对象 ID；新 State 使用系统生成的
UUID，替换图片或撤销不重建 ID。已有不透明 ID 可原样解析，不自动大小写归一化。
应用请求的来源保存对象 ID，显示错误前查找仍存活的宿主，不缓存可复用的 HWND 作为
工作区身份。当前只接入会话内对象身份，不代表分组/链接、序列化或启动恢复已完成。
`workspace.rs` 拥有有界对象身份集合和有向链接，不持有窗口/像素，不运行节点。应用
通过其 `TopologyStamp` 标识对象/连线结构版本；结构事务成功才更新，失败或无变化不更新。
分组/显示变化不属于该凭证；准备计划和执行配置绑定捕获的拓扑，主循环拒绝过期结果。应用
创建和批量导入在显示前注册整批 ID；失败不留下部分注册。关闭回收同步移除对象及
其双向关联边，退出清空集合。端点存在性、链接 ID、端口长度和预算由模型验证；
端口类型/执行循环等属于后续能力与执行层，不能在此擅自禁止主干可表示的有向环。
`workspace_connect.rs` 单独承载用户交互连线规则，对齐 `useLinking.ts`：拒绝自连接和
目标可达来源的循环依赖，替换目标端口的首个占用边并将新边追加到集合末尾。验证和
容量预留在删除旧边之前完成；满预算允许等量替换，失败不丢旧边。遍历使用对象预算
限定的固定队列，已有导入环不会无限循环。导入保留使用 `add_link`，不能用交互规则
误拒绝已有图；端口类型、交互 UI 与执行传播不属于该事务。
`graph_image.rs` 只借用节点图像来源和正式输出值，执行有界来源解析，不下载、解码
或复制图片。Art 的可见预览和正式输出分开；钉图禁用输入、停止接收或本地编辑后停止
上游中继。能力端口的显式空列表覆盖对象端口，循环遍历固定限制在 128 对象内；每套
端口/输出元数据限 128 项，名称及类型限 256 字节。文档像素同步与调度仍由后续集成层
负责，不能把解析成功当成执行完成。
`graph_output.rs` 保留字符串、数值、布尔、空值、对象和数组的借用类型，图片消费端按
value/data/src/previewSrc/url/path 顺序只解包一层。空值可跳过，false、0 和空字符串
不能跳过；正式输出查询仍返回原始值。每节点最多 4096 个值节点、32 层深度、每对象
128 字段，拒绝重复字段和过长字段名。协议 JSON 解码仍须单独限制输入字节并完成适配，
这些值类型不代表已实现外部传输或执行服务。
`graph_params.rs` 拥有公共 Art 参数候选：声明默认值先合并，手动项最后覆盖（包括显式
null），保留 undefined 与 null 的区别。secret 与内部控制字段在默认、手动和连线写入
三条路径均排除；同名声明中任意一项 secret 就整体排除，避免冲突声明泄漏。连线只能
写声明过且未手动禁用的参数，候选先验证结构和 widget 转换再覆盖原值。
`graph_param_coercion.rs` 按 widget 转换数字、布尔、图片/文件及文本；有限数值按 min
再 max 限制。字符串与数值转换细节分别归 `graph_param_string.rs`、`graph_param_number.rs`。
生成文本由参数候选持有，替换/销毁自然释放；枚举候选借用输出，不再复制文本。转换
文本限 64 KiB，数值进制舍入与代表性 JS 字符串夹具经过本地 Node 对照；尚未穷尽
全部 JS 浮点文本边界。执行请求封装与外部服务仍需后续接入。
`graph_effective_params.rs` 将图正式输出绑定到参数候选，只有 Art 使用能力参数声明；
按边集合顺序独立读取来源并转换，后续有效值覆盖先前值，无值或转换失败保留先前值。
secret、内部控制、未声明和手动禁用项在读取来源前拒绝。缺失目标返回空候选，非 Art
只保留公开手动项；空来源端口按旧图约定取 output。结果不修改图或手动参数，生成文本
归候选所有。实际请求序列化、执行调度与外部服务仍未接入。
`graph_requests.rs` 拥有每对象当前 request ID、generation 和可选预览引用；每次 begin
替换旧请求并清空预览，完成/取消释放预览但保留代际。交付必须同时匹配对象身份及
完整请求令牌；旧完成事件不能结束新请求。应用 owner 持有注册表，文档关闭时按工作区
身份回收，退出不留预览。注册项最多 128，单预览引用 64 KiB、总引用 1 MiB；这些是
引用字符串而非图像像素预算。外部解码器仍须限制消息字节并生成新的请求 UUID。
`graph_execution_config.rs` 解析触发/传播布尔配置，保留高成本 MCP 图片搜索在无显式
配置时默认不随参数自动执行的规则。正式请求门槛区分手动、上游和参数最终提交，
纯 Shader/Surface 不进入正式后端路径；上游 listen/来源 notify 由传播入口另行检查。
`graph_dispatch.rs` 组合准入、缺图检查、图片输入和有效参数，全部成功才返回 Prepared。
自动触发缺图跳过，手动触发允许提交缺图候选；活动参数排除禁用值，disabled 列表排除
内部控制及 secret 名称。准备阶段不创建请求、修改状态或启动任务，后续发送层必须在
候选成功后再登记代际，不能把纯预览路径当成后端任务执行。
`graph_execution_wire.rs` 按现有 action/payload 合同构造 execute_art/cancel_art；
`graph_wire_json.rs` 负责 4 MiB 上限的 UTF-8 JSON 编码及控制字符转义，不引入 WebView。
先编码执行消息和旧请求取消消息，再调用注册表推进代际；编码失败保留旧请求和预览。
消息只带 Prepared 的活动公开参数、图片输入与禁用列表，不序列化内部声明。stage
必须在 UI owner 内串行调用，不能在预检与提交间重入/等待 I/O；它不代表网络发送成功。
后续发送层仍需处理取消失败、发送失败及只清理当前请求的错误交付。
`loom_art_wire.rs` 是实际 Loom WebSocket 的 method/params 编码层，不能把上述内部
action/payload 直接发送给 Loom。输入由 `loom_art_input.rs` 验证并编码为 inline_resource
或 shared_memory 描述符，与候选图片端口一一对应；来源 URI 不是外部输入描述符。
保留主干 32 输入、单图 32768 边长/64 Mi 像素、批次 256 MiB RGBA 预算，内联 PNG
按 64 MiB 编码预算及总 Base64 预算检查。JSON 输出按内联输入总预算单独有界，不沿用
仅 4 MiB 的内部控制消息上限。输出 transport 由调用方明确选择，未接好共享内存接收
时只能声明 websocket。描述符校验不证明 PNG 内容或共享内存 owner 已就绪，真实资源
准备、存活期和释放必须由后续传输 owner 负责。
`desktop/loom_inline.rs` 使用 `export::png_payload` 准备真实内联资源，复用裁剪/标注的
已提交合成路径，不生成额外 DIB，不写文件。`loom_base64.rs` 持有编码字符串及共享
预算租约，标准 Base64 编码分块检查取消；完成后不保留输入像素或 PNG 中间缓冲。
描述符借用资源 owner，调用方必须保持 owner 存活至请求发送完成。WIC 编码调用本身
不可中断，取消在阶段边界检查；本路径尚未绑定执行 worker，不能在 UI 热路径直接调用。
PNG/Base64 输出缓冲计入共享预算；WIC 内部流分配仍沿用现有导出机制，不据此声称所有
编码器临时内存已有精确计量。
`desktop/loom_input_job.rs` 提供独立的输入准备任务：每个 owner 单任务、全进程最多两个
worker，不排队；`loom_input_batch.rs` 顺序准备最多 32 个快照，检查整批 RGBA 和 Base64
预算，全部成功后才交付。失败、取消或 owner 析构会释放候选；析构取消并 join，仍可能
等待不可中断的 WIC 调用。候选保存目标和来源的文档身份/版本，调用方发送前必须通过
`is_current` 再检查关闭、替换、编辑和未提交草稿。此检查不代替图连接和参数版本校验。
输入准备不会提前注册执行请求，不能因准备失败覆盖原请求；应用调度和外部发送尚未接入。
真实协议的 `loom_art_wire::stage` 与内部协议共用 `stage_encoded` 注册事务：先构造新
执行消息及旧请求取消消息，全部成功才替换代际并清除旧预览。描述符或消息编码失败
保留原请求；返回待发送消息并不证明网络发送成功，发送失败/取消仍须由传输 owner
按完整请求令牌处理，不能清除随后启动的新请求。
`loom_bridge_stream.rs` 是 TLS/WebSocket 的后台流，Windows 以原生事件实现同步接口。
每个逻辑操作使用绝对截止时间，分片读写不刷新总预算；中断句柄仅暴露取消事件，
不得通过取消 owner 绕过认证层读写。回环测试覆盖读写、超时与中断；此模块本身没有
发现、证书校验或握手能力，不能作为已认证连接使用。主干的 TLS 1.3、固定证书及
TLS exporter 身份证明必须在后续连接层保留，不允许匿名或明文连接降级。
`loom_bridge_identity.rs` 验证协议、实例 UUID、规范 IPv4 回环 WSS 地址、令牌形状、
有界 Base64 证书及 SHA-256 固定值，并拒绝与发现地址不一致的环境覆盖。调试输出和
错误不含凭据。摘要匹配不等于证书有效，也不证明发现来源可信；后续发现/TLS 层仍须
验证文件来源、证书链和实际对端叶证书。Base64、SHA-256 和 UUID 使用主干锁定版本
依赖，不自写密码算法；没有引入 UI 框架或异步运行时。
`loom_bridge_client.rs` 在后台建立 TLS 1.3/WSS：发现证书作为唯一信任根并再次比对
对端叶证书；使用 TLS exporter 和实例/令牌上下文生成证明，原始令牌不放入 HTTP
头，认证头标记敏感。连接、TLS 和升级共享三秒截止时间，返回错误不携带底层请求。
Rustls/Tungstenite 使用显式固定版本，安全修订可高于主干；关闭不需要的默认特性，测试证书仅由 dev 依赖
rcgen 生成。连接 API 仍要求调用者保证发现来源可信，尚未接到应用执行调度。
`loom_discovery_manifest.rs` 以最多 1 MiB UTF-8 JSON 解析发现清单，保留主干清单版本、
应用身份、进程/启动字段、HTTP origin/auth 和唯一能力列表校验，再验证 hookBridge。
默认主干允许的远端 HTTPS 元数据不改变本机 WSS 桥接目标。原始 DTO 不实现 Debug 或
Serialize，错误只暴露类别；该入口不读取文件，来源路径与权限不能由 JSON 校验代替。
`loom_discovery.rs` 接管默认配置和文件读取：规范 OTTER 变量（包括空值）优先旧 HOOK
变量，候选顺序对齐主干。仅 NotFound 继续后续路径，拒绝读取/解析失败后静默降级；
最多读取 1 MiB 加一字节后交给清单解析器。`connect_default` 串联发现与 TLS/WS，
只允许后台 owner 调用。与主干一样，本地环境和清单存放位置属于部署信任边界；
目前没有 ACL/文件所有者校验，不得将格式校验称为文件来源认证。
`loom_art_response.rs` 解析最多 64 MiB 的单条文本响应，按主干协议/请求筛选包络；
预览及结果提交额外匹配对象 ID、代际和正整数 revision。无效预览忽略，无效最终提交
报错；进度和失败保留主干包络筛选行为，交付 owner 仍必须核对完整当前请求令牌。
输出 JSON 移动到事件而非复制图片字符串；错误文本最多 1024 个字符。此层不解码
图片、不打开共享内存、不发布 UI，也不负责 revision 单调性或资源释放。
`loom_art_exchange.rs` 在已认证 socket 上发送单次执行消息并读取到终态，不另建线程
或无界队列。更新回调即时接管进度/预览，返回 false 结束交换；最终结果/失败返回
调用方。发送和 Pong flush 有五秒上限且受总期限约束，无关消息与 Ping 不续期。
输入 owner 必须活到交换返回；外层 worker 仍负责取消协议、输入/输出资源释放、
revision 去重及当前请求令牌检查。当前 API 尚未接入应用调度。
`loom_art_control.rs` 编码资源释放并收取取消/释放回执：最多 128 个唯一且有界的
Loom_Buffer_ 句柄，携带执行身份和独立控制请求 ID；调用方负责生成新 release ID。
控制 socket 的 frame/message 上限收紧为 1 MiB，五秒总等待，处理 Pong 且不续期。
回执校验协议/请求、排除通知，区分成功、请求不存在、失败与取消；不记录远端错误原文。
此模块不打开共享内存，不证明释放句柄属于当前进程，资源 owner 必须提供真实租约。
`desktop/loom_output_inline.rs` 把 formal inline_resource 图片解码成现有 Image/Pixels，
接受主干 PNG/JPEG/WebP/GIF/BMP MIME 白名单，拒绝 data URI 和非法 Base64，编码
字节最多 64 MiB。编码缓冲及解码像素都持有共享预算，尺寸由 WIC 实际解码决定，
不以远端宽高提示分配。Base64/WIC 调用不可中途打断，在阶段边界检查取消；WIC
内部临时内存仍不精确计量。此路径不持有输出 JSON，不访问文件或修改文档。
`desktop/loom_output_shared.rs` 校验 shared_memory/rgba8 描述符及精确字节数，使用
只读 OpenFileMapping/限定长度 MapViewOfFile，把 RGBA 转为预算内 PBGRA 像素。
映射与本地 HANDLE 由 RAII 回收，分块检查取消；不建立跨进程内存的 Rust 引用。
Descriptor 保留远端句柄供调用方在成功/失败后释放租约，本地关闭不替代 release。
协议提供方必须在复制期间保持资源不变；前缀/大小检查不验证映射创建者身份。
`desktop/loom_output_batch.rs` 原子准备最多 128 个有界名称的输出端口，图片总像素
预算 256 MiB；支持普通 value、内联图片和共享图片，候选条目沿用主干 64 上限。
主输出按 output_image/output/image/首项选择。任何端口失败或取消都不返回半批，
已解码本地资源随局部批次回收。函数借用而非消费原始描述符，远端租约释放和候选
元数据仍归调用者；JSON value 当前复制保留，不能据此声称所有结果均零拷贝。
`loom_output_leases.rs` 按单次 formal 交付持有远端句柄、对象和完整执行身份；只扫描
顶层 shared_memory，不递归释放普通 value 中的同名字段。释放尝试串行登记，当前
控制 ID 的成功回执才清空租约；失败/取消/请求不存在保留句柄供有界重试。调用方
必须为每次尝试生成全新 ID。Drop 不发网络请求，外层执行 owner 必须在退出时处理
待释放列表；超量/非法描述符被拒绝，不能声称任意恶意响应中的资源都已回收。
`desktop/loom_delivery.rs` 在输出解码前创建租约 owner，同时保留对象/请求身份、
本地成功或失败结果。`release_with` 由后台调用者注入认证控制通道，传输/服务失败
保留租约供重试，无租约不连接。不能因解码取消或失败跳过 release；候选元数据仍由
响应 owner 保存。此组合尚未接入应用调度及退出重试队列，不在析构时隐式联网。
`Delivery::release_authenticated` 使用执行时保存的 Identity 建立控制连接，不重新
读取发现文件，防止重启/换实例后清理目标漂移。每次使用 CoCreateGuid 生成新的
release UUID；连接和回执失败均保留租约。只允许后台调用，连接三秒、控制五秒预算
沿用底层限制；调用方负责重试次数和退出策略，不在此方法内无限重试。
`desktop/loom_execution_session.rs` 组合认证连接、交换、响应解析、批次解码和释放。
会话仅同步后台运行，连接后把中断句柄交给外层任务 owner；预览 revision 单调递增，
旧预览仍提取/释放租约但跳过解码。每次交付先处理释放，失败返回带租约 Delivery，
不得当作普通错误丢弃；成功释放后的结果才交给回调。应用仍须检查当前请求/文档、
保存输入资源到会话结束、处理远端取消及最终候选元数据。会话不创建线程或 UI。
`desktop/loom_execution_job.rs` 创建最多两个全进程执行 worker，每 owner 单任务，
输入 PreparedBatch 活到会话退出；进度和已释放租约的预览各一个合并槽，不积压消息。
取消设置标志并 shutdown socket，连接后安装中断句柄时再次检查取消，阻断竞态。
正常关闭必须显式 shutdown/join 并接管 Completion 中的 PendingRelease；Drop 只做
线程回收和一次尽力释放，失败仅报告诊断，不保证远端回收。远端执行取消、应用重试
队列和 UI 调度仍须接入，不能以本地 socket 中断代替服务端取消回执。
执行 worker 现于已连接后的本地异常退出时调用 cancel_authenticated，使用
同一 Identity 和完整执行身份发送独立取消请求。Completion.remote_cancel 单独保存
回执或传输错误，不把本地 Cancelled 当作远端成功；连接未建立、会话成功完成或
明确收到服务失败终态则不多发取消。线程完成等待包含有界取消交换，输入资源保留至该步骤结束。应用仍须
处理失败回执/重试及关闭时的完成结果，Drop 只诊断未确认的远端取消。
可取消会话的读取使用 250 ms 阶段预算复核标志，避免仅依赖 shutdown 的唤醒竞态；
短读超时保留协议读取状态并继续，总响应期限不变。该轮询仅用于活跃可取消会话，
不是 UI 常驻定时器；发送、握手和资源释放仍按各自预算等待，不承诺所有取消都在
250 ms 完成。回归另验证不使用中断句柄、只设置标志也能结束空闲读等待。
`loom_execution_cleanup.rs` 在原 worker 中补偿失败取消及释放：首次失败后只再尝试
两轮，间隔 100/500 ms，正常成功无等待。Completion 记录实际补偿轮数和最终
cleanup_pending；清理恢复不把原执行失败改成成功。重试达到上限仍保留未释放 owner
供应用报告，Drop 不再追加第四次网络请求，仅回收线程并诊断未完成清理。显式
shutdown 可能等待这些有界网络尝试，不能在 UI 热路径使用或承诺立即退出。
`loom_execution_tasks.rs` 为应用保存最多两个任务槽，未消费完成结果也占槽；启动
要求请求仍为 registry 当前令牌，关闭/替换只标记退休并取消，不同步 join。领取报告
再次检查当前身份；应用还须检查内容 stamp 后才能写文档。Application 已持有集合，
文档回收及消息处理中取消过期任务并轮询，消息循环结束后显式回收并报告未确认清理。
目前没有 Art UI 启动入口或任务唤醒消息，不能将生命周期接线视为可交互执行已完成。
`graph_execution_images.rs` 是同一借用图视图的执行输入层：有连接但正式输出缺失时，
主输入不回退到钉图本地显示；无连接钉图才使用显示解析。全输入映射采用连接优先、
同名手动参数次之，辅助输入仅来自 Art 能力声明。缺失端口判定与一般参数映射分离，
对齐主干的主输入只查执行来源、手动参数只补辅助项规则。返回集合按端口预算有界，
保留声明顺序、重复名去重，任意名称只作数据键；不执行节点或访问引用指向的文件。
`workspace_groups.rs` 拥有分组元数据和单组成员关系。批量赋组先验证所有对象和目标组，
重复选择不得产生重复成员；删除分组只解除成员关系，关闭分组才移除对象与相关边。
窗口关闭由应用同步清理成员，退出清空分组；空组在普通成员关闭后仍保留。
`desktop/application_group_visibility.rs` 负责组隐藏事务：先建立成员托盘恢复入口，
失败回滚本次新隐藏的窗口，保留原本已隐藏的窗口；成功后才提交模型隐藏状态。
恢复组内任一托盘入口请求显示整组，移出或删除隐藏组恢复相关窗口。可见性变化必须
经过应用动作，不得仅写模型标志；隐藏释放绘制设备缓存，不删除文档或编辑历史。
当前沿用每文档托盘 owner，尚非统一应用托盘。锁定输入由 `desktop/group_lock.rs` 统一准入。
锁定前拒绝仍有编辑任务的组并取消手势，不能让旧后台修改在已锁定状态下提交；加入
锁定组沿用同一检查。窗口消息、菜单执行和面板可用性均检查锁定，生命周期/绘制仍
正常处理，复制/导出及创建新对象允许；替换当前图片在任务入口再次拒绝。
`desktop/group_menu.rs` 提供创建、加入、移出、重命名、删除及关闭组命令，每个宿主只
保留一个未处理请求。请求携带稳定组 ID，不能在处理时误用后来加入的组。名称输入
复用原生模态控件并执行长度校验；应用级模态结束后才由 `application_groups.rs`
处理模型和窗口关闭。删除组保留窗口，关闭组先关闭成员 HWND 再回收模型；失败时
保留未关闭成员。共享菜单目录只在组变化/窗口回收时同步，不添加空闲轮询。
`desktop/group_catalog_menu.rs` 按组建立按需子菜单，无成员组也能重命名、锁定、显示、
删除或关闭。子菜单附加前由局部 owner 持有，成功附加后交给父菜单递归释放；锁定准入
递归检查子菜单。命令解析后保存组 ID，名称对话框返回时再次验证目标存在，不能把目录
索引当作长期身份。目录最多 128 组，不建立常驻窗口或定时刷新。
应用退出使用共享请求，由应用循环在模态结束后关闭全部受管文档；托盘和菜单不得再把
“退出 Otter”实现为只向当前窗口发送关闭消息。退出失败的唤醒不能留下潜伏退出标志。
新钉图图像准备复用 `open_job` 的后台线程、取消与定时器生命周期，完成结果进入源文档
单槽 `new_pin`，由应用集合消费；`State::from_prepared` 不重复计算 alpha 区域，也不
把源文档历史拷入新钉图。新建与替换交付必须区分，不能让截图意外清空源文档。
剪贴板图片默认交付新钉图；`paste_input.rs` 在剪贴板锁释放后把 PNG、DIB 或文件路径
快照交给 `open_job::start_destination`，不得同步解码或另建加载线程体系。替换仅由显式
菜单命令进入确认流程；未消费的新钉图结果不能被第二次粘贴覆盖。
`clipboard_file.rs` 解析 CF_HDROP 的 Unicode/ANSI 列表，保留格式优先级和系统代码页
严格转换，限制 128 KiB 数据、32767 UTF-16 单元单路径及 32 文件总数。批量只用于
新钉图，显式替换必须恰好一个路径；解析阶段不访问文件系统或释放系统所有的句柄。
`open_image.rs` 拥有文件选择与打开菜单，Ctrl+O 默认新钉图，显式替换才确认丢弃编辑。
选择器返回后检查宿主和文档身份，再通过同一后台任务交付；取消不创建任务。
文件选择器与交付分离用于注入测试，不在自动化验证中显示系统对话框。
新钉图的选择器启用 Explorer 多选；`open_paths.rs` 按 `nFileOffset` 区分完整单路径与
目录/文件名列表，验证终止符、绝对目录、子文件名、拼接长度和批次上限，不做文件探测。
显式替换不启用多选，并在交付层再次拒绝多路径，不能静默取批次第一张。
Shell 拖入由 `file_drop.rs` 接管 HDROP 生命周期，快照路径后立即释放数据包；默认交付
新钉图，不调用替换确认。后台准备及单槽交付仍归 `open_job`，窗口创建归应用集合。
该交付槽保存一个有界批次，而非无界队列；`open_batch.rs` 在一个线程中顺序解码最多
32 张图片，全部完成才发布，失败/取消释放已准备候选。`application_import.rs` 先校验
128 窗口总预算，暂存全部隐藏窗口，成功后才更新集合/截图表并显示宿主和面板；任一
准备失败由暂存 owner 析构释放，不得先显示半批或把失败批次部分写入应用集合。
截图新钉图的屏幕原点随 `PreparedDocument` 传递，选区偏移计算归 `capture_crop.rs`，
创建尺寸/位置归 `document_window.rs`；不得在 UI 层把负坐标截为零或重复套用 DPI 缩放。
工具面板和全局快捷键共享 `capture_input::DEFAULT`，默认创建新钉图；替换图片必须
使用显式命名的命令及其确认流程。截图准入检查须发生在取得截图占用守卫之前，避免
用自身刚取得的占用标志把操作错误地阻止。
应用集合还拥有共享截图窗口表和截图占用标志；窗口表只在增删时更新，捕获前快照，
不能跨 ShowWindow 或截图选区消息循环持有 RefCell 借用。恢复可见性必须检查原状态
及 HWND 身份，不能让一个文档的截图重新显示另一个原本隐藏的窗口。
`desktop/modal_scope.rs` 只以弱引用观察各文档的模态标志，截图等应用级动作可以查询，
不能借此把所有本地编辑操作改成全局锁。新增用户输入对话框须通过文档的
`file_dialog::Guard` 管理完整生命周期，且不跨模态消息循环持有 Canvas 借用。
`desktop/application_hotkey.rs` 统一各窗口的截图快捷键开关请求，共享菜单状态不能替代
实际注册标志；系统注册/注销仍归 `global_capture.rs`，且非注册者的幂等清理不得修改
共享状态。测试通过注册适配器验证调度，不占用用户的系统热键。
`desktop/menu_transition.rs` 统一菜单打开/执行前的手势与捕获收尾，以及命令完成后的
面板、裁剪窗口和命中区域同步；命令目录只分派业务，不重复实现这些状态转换。

`native/src/desktop/document_state.rs` 统一单文档宿主状态的初始化；它不拥有进程级消息
循环或全局热键注册，不创建窗口。不同文档可共享不可变像素，但不得共享编辑身份、
历史、手势、任务提交目标或模态门禁。HWND 注册后状态地址必须保持稳定，且窗口应在
状态及 COM apartment 释放前销毁。
`desktop/document_window.rs` 的 `DocumentWindow` 拥有窗口、工具面板、图标和装箱状态；
移动 owner 不得移动 Win32 回调引用的 State。字段声明顺序也是释放合同：窗口先于
图标和 State 析构；新增窗口级订阅应在窗口销毁前停止，不得依赖进程退出清理。
`desktop/host_lifetime.rs` 管理 UI 线程的宿主存活计数，只有最后一个已登记宿主销毁
才能请求正常退出；工具面板和模态对话框不得加入计数。窗口集合未来须继续沿用此规则，
不得在单个文档关闭分支无条件退出共享消息循环。
`desktop/window_priority.rs` 的每线程系统 hook 由各窗口 Watcher 共享，目标和 pending
标志分别管理，确认通知必须带 HWND。文档状态持有自己的 Watcher，并在 WM_DESTROY
及 owner 析构路径中停止订阅；不得为每个钉图重复安装一组系统 hook 或轮询前景窗口。

原生运行中截图入口由 `native/src/desktop/capture_input.rs` 管理确认、窗口可见性守卫及
文档身份校验；冻结帧获取与选区分别归 `capture.rs`、`capture_region.rs`，后台准备与
提交继续复用 `open_job.rs`，不得在面板或菜单内复制图像加载实现。
全屏与区域命令共享该入口；`capture_route.rs` 统一获取模式及新钉图/替换交付策略，
全屏路径只获取一次冻结帧，跳过窗口枚举与区域选择器；
`capture_input_smoke.rs` 仅以内存夹具验证入口事务和真实后台提交，不读取桌面。
`capture_protection.rs` 独立拥有系统捕获排除开关、两窗口更新回滚和部分成功状态，
不将显示亲和性混入文档编辑历史；窗口销毁后该窗口的系统设置随之结束。
`desktop/tray.rs` 拥有按需通知图标、版本化回调、隐藏/恢复与 Explorer 重启恢复，
仅成功建立托盘入口后隐藏宿主，隐藏时释放画布设备资源；不另建事件循环或定时轮询。
`desktop/tray_actions.rs` 只拥有托盘常用操作的菜单投影与路由，复用区域/双轴长截图、
打开图片的新钉图入口。执行前与恢复宿主后分别复核模态、窗口和任务准入，不跨菜单
消息循环持有文档借用，不在托盘模块复制捕获、文件选择或加载实现。
`autostart.rs` 拥有登录启动命令、状态判定和更新后复核；`autostart_registry.rs` 只访问
HKCU 的 Otter Run/StartupApproved 值，读取上限 4 KiB，注册表句柄作用域释放。
`desktop/tray_startup.rs` 按需查询并呈现勾选状态，显式点击才写入，拒绝菜单期间状态
变化和其他安装的启动项。内存 Store 测试不得写真实用户启动设置。
`desktop/global_capture.rs` 拥有 Ctrl+1 与附属 Ctrl+3 的系统注册、消息筛选和共享菜单状态，
分别复用区域截图与垂直长截图；`global_capture_registration.rs` 管理两项注册的降级、
补偿与销毁独立清理，`global_capture_registration_tests.rs` 注入失败验证实际状态。
Ctrl+1 是唯一宿主 owner；Ctrl+3 冲突不撤销 Ctrl+1，转交允许重试附属键。
冒烟模式不注册全局热键，测试不得通过系统输入触发真实桌面捕获。
`desktop/popup_menu.rs` 统一宿主和托盘弹出菜单的模态范围，菜单打开期间阻止截图重入，
返回菜单命令前释放门禁；目标快照与实际业务命令仍归各菜单 owner。
`desktop/app_icon.rs` 负责内嵌品牌 ICO 的有界目录选择、对齐解码与独占 HICON 生命周期，
窗口和托盘只借用句柄；图标 owner 必须晚于所有使用该句柄的窗口和托盘清理析构。
`native/build.rs` 与 `native/resources/otter.rc` 拥有 EXE 的品牌图标和包版本资源，
只使用本机 Windows SDK，不下载构建工具；生成的版本头与 `.res` 写入 Cargo `OUT_DIR`。
`native/tests/exe_resources.rs` 只读映射实际构建的 EXE，校验图标目录与固定版本信息。
`desktop/blur_selection.rs` 统一选中模糊的增量/绝对强度请求及待处理值校验；
`desktop/blur_numeric.rs` 拥有精确输入快照和模态返回后的目标复核，计算与提交仍归
`blur_input.rs` 和原后台任务，不在对话框里复制卷积或历史实现。
`desktop/blur_default.rs` 负责新建模糊工具的精确默认值；`ToolProfiles` 存有限浮点强度，
所有准备缓存与落笔准入使用完整强度，不使用仅供兼容展示的整数舍入值。
`numeric_style::Kind` 统一普通/橡皮擦/效果笔刷的数值边界及马赛克色块校验，
`style_input.rs` 的增减命令复用相同边界，色块小数不能在 UI 命令层转成整数。
工具面板目录包含精确效果入口；`tool_panel_availability.rs` 按实际效果工具/选择目标
计算门禁，可用性用固定 64 位缓存并由目录编译期断言限制容量，不能静默截断新增按钮。
`tool_panel_actions.rs` 是操作按钮绘制标签与原生 HWND 文本的共同来源；样式变化时
同步窗口文本，不能只改变 owner-draw 画面而让原生名称仍保留过期属性值。
`tool_panel_scroll.rs` 负责内容范围、双轴偏移与焦点可见性；`tool_panel_position.rs`
只负责工作区内的窗口外框。前者不缩小字体或按钮以迁就小屏，后者不得以保留标题栏为由
让其余控件永久落到工作区外。`tool_panel_scroll_smoke.rs` 使用隐藏窗口验证可达性和恢复。

`tool_panel_children.rs` 消费相同纯布局与滚动 owner，按最终几何创建全部 BUTTON，
并以真实 HWND 几何跳过无变化的 SetWindowPos；不转移父面板的句柄生命周期。
`tool_panel_layout_tests.rs` 拥有真实隐藏 HWND 的去重/纠偏/resize/双轴滚动/释放回归，
不设置全局 hook、热键或前台窗口，也不把消息计数器带入生产运行时。

`tool_panel_content.rs` 在原创建/布局 owner 之上只负责隐藏 pinned BUTTON 的首次准备
事务、BUILDING/READY、防重入和展示前门；普通工作区及可见 Application 仍提交前准备。
`tool_panel_content_tests.rs` 拥有真实 HWND 的首次状态、失败回滚/重试、显示消息和释放；
布局/滚动/字体/截图保护/捕获窗口集合仍归原模块，不复制业务实现或新增运行时线程。

## 1. 从哪里开始

1. 阅读根目录 `README.md`、`AGENTS.md`、`CONTRIBUTING.md`。
2. 按下面的功能地图找到 owner，再读实现、调用点和对应测试。
3. 先确认当前工作区差异与基线，不覆盖其他人的修改。
4. 修改已有功能时沿用已有状态和生命周期 owner，不创建第二套实现。
5. 提交前更新受影响的导航、检查路径、执行匹配验证，并说明未验证范围。

## 2. 根目录放什么

| 位置 | 职责 |
| --- | --- |
| `README.md`、`README.zh-CN.md` | 项目介绍和用户/贡献者入口 |
| `AGENTS.md`、`CONTRIBUTING.md` | AI 与人工协作的强制规则、贡献流程 |
| `LICENSE`、`THIRD_PARTY_NOTICES.md`、`SECURITY.md`、`GOVERNANCE.md` | 许可、安全报告和项目治理 |
| `package.json`、锁文件、`tsconfig*.json`、`vite.config.ts`、`eslint.config.js`、`postcss.config.js`、`index.html` | 工具链约定入口；不为视觉整洁搬入通用 `config/` |
| `version-state.json` | 内部构建身份，不替代 npm/Cargo/Tauri 的三段版本 |
| 现有启动、停止和打包包装脚本 | 保留用户及自动化的稳定入口；内部逻辑放 `scripts/` |
| `src/`、`src-tauri/` | 前端与原生工程，保留现有构建边界 |
| `native/` | Windows 原生迁移的独立 Cargo 工程；不依赖旧 Tauri 宿主 |
| `tests/`、`scripts/`、`docs/` | 前端验证、工程工具、分主题文档 |
| `public/`、`protocol/`、`security/`、`.github/` | 随应用分发的静态资源、协议夹具、安全配置、GitHub 自动化 |

不把运行日志、截图证据、依赖目录、测试输出、临时计划或用户数据加入源码。
文档图片放 `docs/assets/`；应用图标继续由 `src-tauri/icons/` 管理。
新增根目录文件必须具有项目级入口或工具发现理由，而不只是暂时不知道放哪里。

## 3. 前端功能地图

| 目录 | 职责与常用入口 |
| --- | --- |
| `src/main.tsx` | 按启动模式加载桌面或 Tile；保持 Tile 不初始化桌面监听器 |
| `src/app/` | 应用组合、启动、跨功能事件协调；入口 `app.tsx` |
| `src/features/workspace/` | 画布、Unit、图、选择、拖动、会话与工作流持久化；`state/graphStore.ts`、`state/uiStore.ts` |
| `src/features/stickers/` | 贴图编辑、标注、工具条、几何、历史、导出；`components/StickerAnnotationLayer.tsx` |
| `src/features/capture/` | 区域/窗口/长截图的前端交互；`hooks/useSelection.ts` |
| `src/features/live/` | 实时捕获与 relay 的呈现、状态、输入与生命周期 |
| `src/features/projection/` | 二维码邀请、设备投送、接收策略、双向编辑、绑定与 journal |
| `src/features/art/` | Art 能力、节点参数、端口、OCR 呈现与 shader 预览 |
| `src/features/surfaces/` | 声明式/JavaScript Surface 宿主、资源与确认界面 |
| `src/features/extensions/` | 插件贡献、菜单、快捷键、bridge 与权限快照 |
| `src/features/tile/` | 独立显示终端、墙面协议、呈现与输入控制；`components/TileTerminal.tsx` |
| `src/features/settings/` | 应用/工具设置、文件命名、字体与启动配置模型 |
| `src/platform/tauri/` | invoke、命令客户端、浏览器 fallback 与本地图片 URL 适配；稳定门面 `api.ts` |
| `src/platform/desktop/` | 桌面输入、交互区域、焦点、快捷键与诊断边界 |
| `src/integrations/loom/`、`tea/` | Otter 侧连接与契约适配，不复制外部产品内部代码 |
| `src/shared/` | 无业务状态的 async、diagnostics、geometry、images 和全局样式原语 |

功能目录内按需要使用 `components/`、`hooks/`、`services/`、`state/`、
`types/`、`styles/`、`model/`，不为了模板创建空目录。
组件及其专属 CSS 可以相邻；领域类型跟随功能，不汇总到全局类型大文件。
例如修改投送接收 UI，先看 `features/projection/components/`，再看同目录树中的
`state/` 和 `services/`，测试从 `tests/unit/`、`tests/integration/` 按功能名定位。

## 4. 依赖与状态边界

- 目录表达职责，不宣称本次迁移已经消除了历史跨功能耦合。
- `app/` 负责组合。领域算法不要继续堆入应用入口。
- `shared/` 不依赖 `features/`、`app/`、`platform/` 或 `integrations/`。
  一个 UI 即使被多个功能使用，只要依赖工作区状态，就仍属于工作区，不能假装是纯通用组件。
- 外部服务通过 Otter 的 client/协议适配接入；不得复制 Loom、Talk、Tea 等仓库的实现。
- 功能间使用明确的现有接口；新增依赖不得引入运行时循环，也不要创建汇总所有模块的 `index.ts`。
- 不以文件迁移为理由引入全局 service locator、事件总线、第二份 store 或额外 package。
- 保留 `graphStore.actions` 的持久化写入口；高频指针预览不得直接写入持久化图。
- 移动 store、cache、监听器或 controller 时，必须保持实例数量、初始化时机、清理与取消语义。
- 延迟加载、Worker、CSS、字体、Tauri 命令字符串、协议字段和应用数据路径不是普通 import；逐项检查。

## 5. Rust 与原生目录

迁移分支新增 `native/Cargo.toml` 和独立 `Cargo.lock`。`native/src/desktop/` 拥有 Win32、
COM、WIC 和 D2D 资源；`native/src/viewport.rs` 是不依赖 Windows 的视口与解码准入模型。
`native/src/mosaic.rs` 拥有不读取源图的绝对坐标遮挡网格，输出使用共享像素预算；
Windows 绘制层只消费其纹理，不另写一套网格哈希或取样原图代替遮挡。
`native/src/blur.rs` 拥有只读源像素的区域模糊计算、邻域边界和临时预算；
异步宿主负责调度、取消及结果版本校验，不把大图卷积直接接入窗口绘制回调。
`native/src/blur_job.rs` 拥有模糊线程、单项最新请求队列与退出回收；
窗口宿主仍负责依据文档身份凭证、版本和源像素身份决定结果能否提交，连续强度请求也不得跨文档继承。
`native/src/blur_image.rs` 是已计算模糊图的不可变共享快照，`editor_blur.rs` 负责笔迹事务；
`desktop/annotation_blur.rs` 只绘制快照与遮罩，不执行卷积或调度后台任务。
`native/src/editor.rs` 拥有基础标注草稿与增量撤销记录，不依赖 Windows；`desktop/edit_input.rs`
将窗口手势映射到图像坐标，`desktop/annotations.rs` 负责 D2D 呈现。
`editor_document_stamp.rs` 拥有异步/模态提交的文档身份与版本凭证；宿主另外校验活动手势、源像素、取消和窗口存活，不用版本号或像素地址替代文档身份。
`desktop/file_drop.rs` 拥有 Shell 拖入句柄和有界路径提取，立即释放系统数据包，再复用 `open_image.rs` 的替换确认与 `open_job.rs` 的后台解码；不在窗口线程读取文件或执行导入路径。
`desktop/file_dialog.rs` 拥有每个窗口独立的文件模态门闩，打开、保存、拖入确认共用；不跨模态调用持有 Canvas 借用，重入拖入仍须释放 Shell 数据包。
`desktop/tool_commands.rs` 拥有工具顺序、中文名称、快捷键和同步工具映射的唯一目录；菜单按目录构建，模糊仍通过异步准备分支执行。`context_menu::execute` 是即时 UI 的命令执行入口，延迟菜单须先校验目标快照。`tool_panel.rs` 以受主窗口拥有的 Win32 工具窗口和 19 个原生按钮消费目录，负责 DPI 布局、F4/关闭显隐、键盘消息预处理和命令路由；`tool_panel_draw.rs` 负责 Neuro 深色/信号黄状态绘制，DC 状态保存失败时不选择字体。`tool_panel_font.rs` 通过面板窗口属性拥有单个 DPI 字体，同尺寸复用、DPI 变化替换并释放旧句柄，WM_NCDESTROY 释放最终字体；绘制只借用字体。`tool_panel_layout.rs` 根据目录数量统一计算分组行数、客户区和按钮矩形，使用 96 DIP 按钮宽度；实际 GDI 字体度量测试覆盖长标签和选中标记。`tool_panel_position.rs` 按物理像素和显示器工作区放置/校正工具面板，覆盖负屏幕坐标、底部空间不足和超小工作区，不轮询或强制跟随。工具状态在宿主消息处理完成后比较，无变化不刷新面板，重入 Canvas 借用由外层返回后补检查。`paint_scope.rs` 独立拥有 BeginPaint/EndPaint 配对。无空闲计时器，按钮随 owner 销毁；`tool_panel_availability.rs` 从文档、选中对象和任务 owner 计算按钮可用性，执行时复核以拒绝过期排队点击；面板缓存位掩码，仅可用性变化时调用 EnableWindow。`tool_panel_actions.rs` 定义复用菜单 ID 的 16 个属性/文档按钮及真实样式标签，刷新缓存同时跟踪工具与 StrokeStyle；已接入颜色、填充色、粗细、透明度、填充开关、应用样式、打开、复制、导出，`numeric_style.rs` 拥有粗细/透明度的有限数值解析、范围约束和模态目标校验，复用 `shape_dialog.rs` 的原生输入控件；完整属性编辑器与主干工具栏布局仍未迁移。

`desktop/menu_target.rs` 拥有菜单打开时的目标快照及过期校验；`context_menu.rs` 在嵌套消息循环返回后验证，避免命令作用于被替换的文档或改变的选择，相关回归在 `context_menu_tests.rs`。

`desktop/marquee.rs` 拥有标注框选的文档绑定、图像坐标范围、松手选择与临时选框绘制；不复制标注、不记录编辑历史，选择结果复用 `multi_selection.rs` 的组命令。`marquee_smoke.rs` 拥有隐藏 HWND 的框选消息链验收，测试后恢复文档、选择和当前线程键盘状态，不注入系统输入。

`desktop/paste_input.rs` 拥有显式图片粘贴的有界剪贴板快照，优先 PNG，其次 DIBV5/DIB，最后尝试 Unicode/ANSI CF_HDROP 单文件列表（`clipboard_file.rs` 负责有界解析）；只借用系统 HGLOBAL 并按时解锁/关闭，不释放系统拥有的句柄。`clipboard_dib.rs` 拥有有界位图布局解析、掩码转换和逐行取消，`clipboard_dib_rle.rs` 拥有 RLE4/RLE8 指令解码，直接写入预算内输出、无中间索引图，并在每条指令处检查取消。解码复用 `open_job.rs` 的后台生命周期。`clipboard_input.rs` 集中路由复制/粘贴快捷键与复制定时器，避免在窗口总过程堆积业务；菜单测试独立于生产菜单构建。
`desktop/copy_job.rs` 拥有显式图片复制的单个后台任务、取消与发布准入；`export.rs` 合成一次并编码 PNG，`clipboard_bitmap.rs` 转换为非预乘 BGRA 的 DIBV5，两个字节缓冲均受预算约束。`clipboard.rs` 在确认结果仍有效后，预先准备全部 HGLOBAL，再一次打开/清空剪贴板发布 PNG 与 CF_DIBV5。测试禁止写真实系统剪贴板；文件列表及缓存仍由 copy_file_plan/clipboard_cache 管理。
`desktop/clipboard_editable.rs` 拥有进程内单槽可编辑内容和发布后 sequence 凭证，
为普通展开钉图（含 resolved 显示快照）保留编辑状态；Art/mini 仍扁平，发布跳过不安装凭证，序号失配/零值不命中。
`editor_snapshot.rs::duplicate_snapshot` 重建粘贴文档和标注身份，不继承源撤销历史。
`duplicate_snapshot_with_fallback_base` 为无 raster 的显示快照绑定对应显示底图，
保留已有 layers、分数逻辑尺寸和覆盖层坐标；resolved copy 不附带目标 Canvas 来源。
粘贴复用 open_job 后台准备及 PreparedDocument/State/application_import 接纳链；
`clipboard_editable_tests.rs` 验证独立性/单槽/取消，`application_paste_editable_smoke.rs`
覆盖真实线程、隐藏窗口、裁剪视口、撤销隔离和取消，不访问系统剪贴板。
`clipboard_capture_tests.rs` 保护采集来源重绑定和共享只读原图租约寿命；
CaptureSource只克隆值与租约，不克隆源文档stamp，也不重开记录中的路径。
`desktop/clipboard_placement.rs` 拥有复制物理尺寸/鼠标偏移快照与提交后级联 session；
只保存 UnitId，application_import 交付时解析当前 HWND，不持有过期窗口句柄。
`clipboard_placement_tests.rs` 保护几何、整数边界、旧请求隔离；
`application_paste_placement_smoke.rs` 验证真实窗口尺寸、排队刷新和失败不推进。

`desktop/surface_capabilities.rs` 拥有实际声明式宿主能力报告，供新建/恢复共用，
不能由 Art 请求反向宣称支持。`surface_form_attach_tests.rs` 拥有六类表单的真实
attach worker/授权 HWND/batch worker/撤权组合验证；`surface_form_attach_peer.rs`
只拥有对应本地 TLS 协议响应与断言，复用既有 fixture，不接入外部服务。
`application_surface_forms_tests.rs` 复用资源测试的 Application/授权夹具，验证
正常 poll 自动提交及回填，不手工启动表单 worker；`smoke_timing.rs` 只拥有
显式诊断开关与分段计时，不拥有任何业务状态或启动期限策略。
`desktop/path_handles.rs` 统一直线/箭头端点与贝塞尔控制柄的命中、预览和提交；
`editor_line_endpoint.rs`、`editor_bezier.rs` 分别拥有对应模型事务，旋转中心补偿由 `rotation.rs` 共享。
`desktop/export.rs` 复用标注绘制生成原图尺寸 PNG；`desktop/export_job.rs` 拥有原生保存对话框、
单任务后台导出与完成通知。导出不依赖视口坐标，不覆盖已有文件。
`Canvas::normal_opacity` 保存独立的正常底图输出透明度，会话装配从 opacityNormal 恢复，
不能用当前窗口的缩略透明度代替。复制/导出 worker 捕获该值，`display_snapshot::Stamp`
负责变化失效；`export.rs` 仅在底图 DrawBitmap 时应用，不衰减后续标注。
`desktop/export_base_opacity_tests.rs` 验证真实 PNG 的底图/栅格/矢量分层透明度、
输入拒绝与源状态不变，不触碰系统剪贴板。
`desktop/application_restore_document.rs` 拥有恢复事务单窗口凭证，供原工作区 guard
与隐藏候选批次共用；记录身份/状态，不复制图像。`application_restore_readiness_smoke.rs`
通过真实隐藏窗口验证双侧属性、候选草稿与请求变化的失效和非消费行为。
`desktop/application_restore_manifest.rs` 在建窗前检查工作区与各恢复列表的一一对应、
标注身份和状态归属；节点数量有上限，不复制像素。成功批次的元数据由批次私有持有，
防止调用方绕过准备阶段直接修改候选清单。
`graph_requests::Requests::is_idle` 只观察活动请求，不清理历史代际或预览；
`application_restore_idle.rs` 用它阻止存在应用级活动请求时替换工作区，不能以
后台 worker 暂时为空代替该条件。
`desktop/application_restore_publish.rs` 拥有通过检查后的运行工作区替换、热键转交补偿
以及原文/标注身份留存。可失败准备先于窗口集合交换；成功后才释放旧 HWND，清理旧
显示缓存。`application_restore_publish_smoke.rs` 使用独立隐藏 Application 验证实际
所有权转移和失败保留，不操作用户窗口、文件或真实全局快捷键。
空会话的临时工作窗口由发布模块准备并绑定到同一应用；不改写原始会话、不生成标注 ID。
`application_restore_empty_smoke.rs` 验证重复空恢复、新建窗口及完整释放。留存的临时宿主
身份只描述恢复时状态，后续持久化不得据此忽略该窗口新增的用户内容。
`desktop/application_restore_job.rs` 拥有应用级单个会话恢复任务、启动时来源凭证、
目录/坐标上下文、取消与完成后的绑定/发布；`session_job.rs` 只承担后台解析和图像准备。
`application_restore_job_smoke.rs` 使用真实 worker 和独立隐藏 Application 验证整条协调
链，等待仅出现在测试中；生产使用完成通知唤醒，不以固定频率轮询。
恢复链的目录快照为 `Option<Rc<Loaded>>`，None 表示真正离线而非伪造空服务目录。
`session_binding::catalog_matches` 统一校验离线/在线状态和目录身份；离线 Art 保留原文
但不创建可执行 Selection，后续重新绑定必须经过真实能力定义的参数/保密策略解析。
`desktop/application_restore_rebind.rs` 拥有恢复后未绑定 Art 的迟到目录重试；候选列表、
已尝试身份和目录引用归恢复 Record 所有。常规目录处理释放借用后调用它，忙碌节点延后，
同一目录每个候选只解析一次；所有选择准备成功才安装，不覆盖已有选择、不触发执行。
`application_restore_rebind_smoke.rs` 用独立隐藏工作区验证目录重试、模态延后、保密字段
过滤和选择保持，不连接真实服务或改写留存会话。
`desktop/session_job.rs` 的 Input 区分内存字节与显式文件路径；`session_file.rs` 负责
worker 内的有界只读、磁盘/重解析点检查和分块取消，不推导图片资源目录权限。
`session_file_tests.rs` 只在 linshi 创建并清理自身测试文件，验证原文、预算与句柄释放。
`native/src/startup_session.rs` 解析显式会话启动参数及历史坐标上下文；`desktop::run_session`
复用原生宿主初始化并发起后台恢复。隐藏检查模式不进入注册热键的应用 run 路径。
`native/tests/session_startup.rs` 以真实 EXE 子进程检查文件到恢复发布的完整链，并为自己
创建的测试子进程设置超时回收；测试文件只存于 linshi。
会话检查模式还执行恢复 HWND 的绘制及设备资源丢弃/重建，渲染错误导致检查失败；
子进程测试使用自带合成 PNG 与标注，不暴露私有像素构造接口作为测试后门。
`State::is_art` 统一运行节点类型判断：已选能力或恢复的 Art 身份均属于 Art。
类型不代表能力可执行，执行和参数编辑仍检查真实 Selection；显示/依赖/传播图和
钉图专用编辑必须使用节点类型，不能用能力是否在线代替。恢复清单校验保留类型一致性。
`desktop/composition.rs` 拥有透明交换链、D2D target 和 DirectComposition 生命周期；
`desktop/render_surface.rs` 统一普通窗口与钉图绘制接口。`pin_geometry.rs` 管理有界物理像素缩放，
`desktop/pin_window.rs` 负责与 HWND 的接线。GPU 读回只用于测试，不进入正常钉图绘制。

v415：`render_surface.rs` 独占 Canvas 帧事务与成功后的显式提交，`resource_target` 仅供画刷/位图创建同步；
`render_frame.rs` 拥有 D2D BeginDraw/EndDraw 和局部 clip/transform 的栈上 RAII，`render_frame_tests.rs`
用真实 WIC 像素覆盖错误/panic 清理、EndDraw 错误及错误优先级。Canvas 四条绘制路径只发绘制命令，
缩略 Presentation 的自有事务包装仅用于原 WIC 测试；Target 仍统一拥有 surface/brush/raster/effects，
不迁移文档 owner，不因 backbuffer 轮换重建同设备缓存。

`desktop/composition_device.rs` 只拥有默认呈现设备的 UI 线程弱缓存和租约；共享健康
D3D11 device，不共享 D2D context/交换链/合成树，不接管 Live 显式捕获设备。
v413 将硬件/WARP 弱缓存分开；`desktop/composition_selection.rs` 拥有完整候选创建的
有限选择事务和失败阶段/HRESULT，不拥有窗口、文档或运行中恢复循环。
`composition_selection_tests.rs` 覆盖尝试次数、失败原因与清理顺序；
`composition_tests.rs` 用注入主候选失败后真实 WARP 绘制/读回验证透明像素与 resize。
v414 的 `composition_d3d12.rs` 拥有独立 D3D12 窗口候选和显式 draw/present/resize 状态，
尚未默认接线；`composition_d3d12_device.rs` 拥有唯一 direct queue、11On12 bridge、fence/event；
`composition_d3d12_buffers.rs` 拥有固定双缓冲视图与 Acquire/Release 的帧 RAII；
`composition_d3d12_lifetime.rs` 拥有有界候选租约，未确认 drain 的完整 owner 不归还配额。
`composition_d3d12_tests.rs` 覆盖真实 D3D12 窗口像素、纹理、文字、双缓冲、resize 和异常绘制清理。
`composition_device_tests.rs` 保护弱引用释放、失效/创建失败和回退/线程隔离；
`composition_sharing_tests.rs` 保护真实窗口身份、像素、resize/opacity 和最后租约释放。
`alpha_region.rs` 构建有界 alpha 行区间，`desktop/hit_region.rs` 拥有缓存、GDI 区域以及
`SetWindowRgn` 的所有权转移；区域设置前必须释放模型借用，避免同步窗口消息重入。
`native/tests/` 放真实子进程启动检查。当前入口支持图片显示、平移缩放与基础标注，不是完整产品替代。
Shader 显式候选运行 owner（不归下述构建脚本）：

| 责任 | 位置 |
| --- | --- |
| 固定候选身份、两文件集合和同句柄摘要 | `native/src/shader_candidate.rs` |
| 逐级本地固定盘路径、共享租约、硬链接/reparse 拒绝 | `native/src/shader_candidate_files.rs` |
| 句柄 owner/DACL、SID/ACE 边界 | `native/src/shader_candidate_acl.rs` |
| 顶层配置提取与截图/建窗前只读准入 | `native/src/startup_settings.rs`、`native/src/desktop/startup_entry.rs`、`shader_startup.rs` |
| Application 安装及显式产品 Shader smoke | `native/src/desktop/application_shader_startup.rs` |
| 候选路径/ACL/共享合同、真实 EXE 与独立 TLS peer | `native/src/shader_candidate_tests.rs`、`native/tests/shader_startup.rs`、`native/tests/support/shader_peer.rs` |

后续领域模块按功能添加，禁止把编辑、持久化或集成逻辑堆入窗口过程。
任务和验收见 [原生迁移计划](WINDOWS_NATIVE_MIGRATION.md)。

`src-tauri` 保留 Cargo、Tauri、build.rs、icons、capabilities、windows 和本地 crates 的位置。
现有 `screenshot`、`loom_connection`、`live_gpu`、`live_resources`、`voice` 等模块继续拥有自己的目录。

`src-tauri/src/native/` 按应用接线职责分为：

- `runtime/`：启动、日志、托盘与应用生命周期。
- `capture/`、`long_capture/`：截图命令和长截图编排，不与独立算法模块混为一谈。
- `input/`、`overlay/`：输入队列、hook、光标和 overlay 窗口/遮挡/交互路由。
- `session/`、`image_io/`：会话持久化与图片、剪贴板、拖放边界。
- `live/capture/`、`live/source/`、`live/relay/`、`live/observation/`：实时会话职责。
- `tests/`：现有 native 夹具与合同测试，保留各自 include 入口。

重要：这些文件中有许多由 `lib.rs` 的 `include!` 接入 crate 根作用域，
并非同名 Rust module。不得把 `include!` 顺手改成 `mod`，否则会改变私有项可见性、
宏展开和命令路径。需要这种重构时，应单独设计并证明行为不变。
移动 Rust 文件要检查 `include!`、`include_str!`、`include_bytes!`、`#[path]` 和调用方。
本地第三方 crate 的许可证、来源和锁文件必须随其 owner 保留，不借整理目录修改供应链。

## 6. 测试、脚本和文档

`tests/` 保留 Vitest 的 `unit/`、`integration/`、`performance/`、`fixtures/`、
`helpers/`、`setup/` 分组。`integration/` 同时包含源码合同与运行时测试，二者不能混称。
`scripts/tests/` 使用不同 runner 并包含真实环境探针，不并入 Vitest 默认收集。
Rust 集成测试仍在 `src-tauri/tests/`，模块单元测试留在所属模块。

`scripts/` 的现有顶层命令是稳定工具入口；现有 `release/`、`security/`、浏览器与
native 子目录放内部实现。新工具先明确用途、runner、输入/输出、副作用和退出码；
不要为一次任务增加永久 wrapper。修改脚本位置时检查 `$PSScriptRoot`、`%~dp0`、
`import.meta.url` 和 `__file__` 的根目录推导，不能只改调用命令。

ANGLE 构建接缝的长期 owner 如下；运行时加载不归这些构建工具：

| 责任 | 位置 |
| --- | --- |
| 显式路径/schema/关键文件摘要 | `scripts/native-angle/toolchain_paths.py` |
| 环境清单与无发现回退消费 | `scripts/native-angle/otter_toolchain_env.py` |
| 固定 setup adapter 变换及输出 | `scripts/native-angle/prepare_toolchain.py` |
| 固定 GN profile 变换/guards/args | `scripts/native-angle/gn_profile.py` |
| profile 与 adapter 编排/绑定回执 | `scripts/native-angle/prepare_profile.py` |
| 固定 source mount/commit/tree 与工具摘要 | `scripts/native-angle/stage_pins.py` |
| Windows 名称空间/流式写入/容量与文件清单 | `scripts/native-angle/stage_inventory.py` |
| 只读 Git 对象验证、gitlink 登记和进程回收 | `scripts/native-angle/stage_git.py` |
| 固定归档数据展开及工具身份 | `scripts/native-angle/stage_archives.py` |
| 本地暂存编排、profile 消费和总回执 | `scripts/native-angle/stage_build.py` |
| 构建前后外部 stage 身份、精确集合和输出保护 | `scripts/native-angle/build_inputs.py` |
| Win32 构建进程树、句柄、Job 和同 session mutex | `scripts/native-angle/build_job.py` |
| 纯 PE 导出/可执行地址与普通/delay-import CRT 门 | `scripts/native-angle/build_binary.py` |
| 固定 GN/Ninja 编排、诊断、载荷和构建回执 | `scripts/native-angle/build_fixed.py` |
| 已审阅构建的只读材料准入 | `scripts/native-candidate/angle_intake.py` |
| 声明原字节选择/预算与材料文件 SBOM | `scripts/native-candidate/angle_notices.py`、`angle_sbom.py` |
| 本地材料复制编排、写入预算与失败回执 | `scripts/native-candidate/angle_materials.py` |
| 封存证据关联和独立摘要/精确集合核验 | `scripts/native-candidate/angle_materials_evidence.py`、`angle_materials_verify.py` |
| 合成夹具和环境/adapter/profile/staging/build/materials 合同 | `scripts/tests/native_angle_fixture.py`、`native_angle_stage_fixture.py`、`native_angle_build_fixture.py`、`native_angle_materials_fixture.py`、`test_native_angle_*.py` |

实际入口、输入输出和副作用见 [ANGLE_TOOLCHAIN.md](ANGLE_TOOLCHAIN.md)。固定 profile
已有独立的本地暂存和显式构建 owner；完整默认源码/工具/许可闭包、获取及可信安装
仍不归本入口，不以固定输入暂存或两 DLL 构建成功冒充全部供应链完成。

文档目录按阅读目的划分：`guide/` 用户与开发上手，`architecture/` 当前设计，
`features/` 功能契约，`development/` 工程规范，`release/` 发布，`security/` 安全。
详细文档从 `docs/README.md` 导航；根目录保留 GitHub 可发现的贡献、安全和许可入口。
迁移文档要同时更新相对链接、脚本读取路径和 CI 的 docs-only 分类规则。
历史基线清单绑定历史提交，不得机械改写其路径或 hash。

文档清理应同时核对内容与引用，而非仅移动文件：

- 每项当前规则保留一个权威维护位置，其他入口链接过去；双语用户与开发指南同步。
- 以当前实现和 workflow 修正路径、参数、功能与产物说明；源码接入不等于原生验收通过。
- 已完成任务的逐轮日志、临时机器地址、测试计数和候选哈希留在 Git 历史，
  当前文档保留可重跑步骤、安全边界、未验证限制及必要历史证据定位。
- 保留仍在使用的兼容标识和未闭环限制；外部仓库的实现/部署状态不能由 Otter 单方面认证。
- 合并或删除文档前检查导航、测试、脚本和 CI 消费者；更新合同测试而不是恢复过期文案。

## 7. 变更与验证标准

- 源码、注释、文档和配置使用 UTF-8 无 BOM。保留现有命名，不在搬目录时批量改符号。
- 目标约 150 有效行，优先 100–250；251–500 需单一职责；501–700 需例外理由与保护测试。
  新增或完整迁移文件不得超过 700；完整规则以根 `AGENTS.md` 和官方计数器为准。
- 修改前测量并跑针对性基线；迁移优先整文件移动，只修改必要的路径和模块接线。
- 迁移与业务变化分批审查。不得通过删测试、改预期行为、减覆盖或放宽阈值掩盖回归。
- 检查类型、lint、相关测试、Rust formatter 和有效行数；最后运行 `git diff --check`。
- 功能迁移完成后运行完整前端测试、独立性能组、Rust 测试、静态构建与适用的浏览器门。
- 静态构建会清理其输出目录；打包、原生探针、真实服务连接具有副作用，先确认输出与环境。
- 保留稳定 npm 命令名。内部迭代只递增 `version-state.json`，不改变公开版本或应用身份。
- 分别报告代码检查、浏览器检查、原生运行、打包和人工验收；已有失败不等于新回归，未跑不等于通过。

当前命令和发布门见 [工程验证要求](ENGINEERING.md#6-tests-and-verification) 与
[发布来源规范](../release/release-provenance.md)。本文件不重复维护另一套命令清单。

## 8. 目录设计参考

参考的是职责表达方式，不是第三方实现。Piik 将工程入口与领域目录分开；Cap 保留桌面应用的
`src/` 和 `src-tauri/`；ShareX 按截图、编辑、历史等职责组织代码。Otter 不因此引入多应用 monorepo。

- [Piik 工程边界](https://github.com/TNTcraftHIM/Piik/blob/main/docs/standards/engineering.md)
- [Cap 桌面工程](https://github.com/CapSoftware/Cap/tree/main/apps/desktop)
- [ShareX 仓库](https://github.com/ShareX/ShareX)

### 原许可服务 dispatcher（当前实现，待集中验证）

- `native/src/art_service_dispatcher.rs` 只拥有从已下载 Prepared 移交的 Client、签名 envelope、绑定及原 Permit/Lease 引用；不新建信任或凭据。
- `art_service_http.rs` 的借用 Authority 统一原 Prepared 与 signed Local 服务路径，网络期间不持授权锁；原 Call 公开行为保持。
- `art_service_signed_tests.rs` 负责验签下载→受限 child→回环TLS的最终null/错误、scope与offline拒绝、HTTP后撤权丢帧；非实际 Loom daemon 联合证明。

### 跨仓实际服务联合门（input-2 定向通过，非完整产品验收）

- Otter art_service_joint_tests.rs 仅在显式环境指定的隔离Loom夹具中运行，使用真实设备挑战/签名续签、原Prepared下载/验签和真实受限child；不模拟服务回复、不写用户配置。
- Loom local_art_joint.rs 持有独立daemon、签名插件和精确包审批，协调Otter测试进程完成调用/null及服务端撤权拒绝；有界等待、仅回收自建进程，失败保留夹具证据。
