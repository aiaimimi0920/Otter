# Windows 原生架构迁移

## 当前目标：只对接最新架构（2026-11-04）

用户已明确：所有相关软件同步升级，不需要旧协议、旧模式、旧程序兼容。取消版本协商、旧路由探测、自动降级、新旧互通的开发与验收流程，不再纳入进度分母或阻塞。以下历史记录中的兼容要求已被本决定取代；不删除用户数据或改写旧失败。当前架构自身的离线能力、签名、授权、资源预算与清理围栏仍有效。

v538 Local Art主链仅使用当前v3完整签名包；单端口默认显隐、用户覆盖及保存/重开/离线消费已定向接通。验证和未验边界集中见[构建记录](WINDOWS_NATIVE_BUILD.md#v538-当前签名包与端口默认显隐)。后续按完整功能批次集中集成和验证，不为旧兼容另开流程。

## 当前 v537：普通贴图输入闭环

普通贴图上传/清空/禁用已贯通真实文件worker、图像优先级、下游通知、配置凭证与保存重开；完整参数窗及image child共用同一事务。保原画布独立显示、未知参数和既有全局预算。

本批同源定向 **250项Rust passed / 0 failed / 0 ignored**，含16项新增；不是整库通过。官方fmt、版本、全仓strict行数、strict all-targets Clippy、固定SHA库、普通Release与workspace门通过。只分配一次v0.2.33.537；先集中实现与测试，再统一Clippy/产品构建。

本批未启动普通前台交互，不以隔离HWND/消息回归替代普通可见上传对话框、键盘或人工IME。前台恢复仍仅限新profile、自有测试窗；保旧工具边界/失败及dirty/untracked，三冻结副线/v464/用户release97不动。Talk构想和旧Web/Tauri混用排除，不启动Loom原生迁移，不提交推送发布部署，总迁移active。

剩余实现重点为Local签名默认显隐合同；其余普通产品/物理环境、全量与合计资源、最终包和用户验收仍开放。详见[本批构建记录](WINDOWS_NATIVE_BUILD.md#v537-普通贴图图片输入)。

## 当前 v536：公共Art参数内嵌控件

公共Art参数输入展开后可挂真正单字段原生child，复用EDIT、COMBOBOX、bool、步进、slider、颜色和文件控件及原参数事务；不是弹完整面板冒充内嵌。工作区仅一个活动editor，宽300逻辑像素，放图片左侧、空间不足翻右侧，两侧不足不挂，不覆盖图像。端口`Enter`/`Space`仍切换展开，`F2`从已聚焦参数端口进入控件，child内`F6`返回端口；文本Ctrl+Enter仍提交文本，不能误连线。linked有效值只读、secret不呈现；只允许已确认自身同步提交重建新Catalog后继显示，不继承旧草稿。

`chrome::capsule`拥有工作区单槽Rc候选、目录/宿主/parent身份及后继显示；`parameter_panel::window`拥有完整owned窗和单字段child的共同构造与资源寿命，`controls::capsule`仅筛选字段动作，复用原Snapshot/Spec和唯一参数提交。dirty/IME/pending纳入原chrome idle及保存/恢复/结束会话门。外部配置、隐藏、锁、捕获、端口显隐及失焦退休旧资格；同步focus/place/close/request返回只可取消同一Rc，旧栈不能撤销或覆盖后继。模态返回再次核panel owner及配置，旧panel不继续消费后缀；child继承端口条affinity，不对child调用顶层DisplayAffinity。没有新依赖、网络合同、权限、线程、timer、每端口HWND或像素/队列预算。

本批选定**98项Rust passed**，14项新增，选定终态0 failed/0 ignored；1项既有默认ignored未选、不计通过，非整库。覆盖真实child/单字段、多种公共widget、原子提交/linked只读、IME边沿/DPI草稿/父先销毁、region及图片外几何、单槽/输出/secret、dirty空闲门、自身提交后继、F2/F6/两段Esc、外部状态拒旧草稿、同步focus/place/close/request安装后继和退休owner拒发布；原chrome/参数/系统会话/Live输入取消依赖同批验证。官方fmt/版本/全仓strict行数、strict all-targets Clippy、固定SHA库、普通Release与workspace目标通过。23项源码/版本delta均UTF-8无BOM，6个新文件，Rust最大397有效行，无新增例外。input1首次Clippy因子模块可见性、测试未用导入和Stamp断言编译失败，原回执保留；input2只修这些编译问题，不重分版本。input2的chrome为47通过/1失败：后继回归夹具在同尺寸SetWindowPos后误等CHANGED；input3只在该cfg(test)分支先改变child高度以实际触发几何消息，原失败保留。guard回归验证返回后退休资格，不冒充真实颜色/文件模态人工验收。

前台限定恢复有效，仅新profile、自有测试窗。按用户本轮快速收口要求，只完成已启动批次的定向回归和产品构建，不重开与旧批次相同工具边界的普通前台尝试。本批没有启动新可见fixture或发送UI输入；普通可见capsule/颜色文件模态/完整端点键盘及人工IME未验，不能由隔离HWND回归推定通过。旧前台工具边界和清理失败回执继续保留。当前capsule仅公共Art参数；普通sticker的image上传入口及Local签名默认显隐元数据仍未完成，输出不提供参数editor。普通完整Art/Live/端点键盘及人工IME、普通双产品、物理多屏/跨机/DPI、剩余全量/合计资源长稳、最终完整隔离包及用户全面验收仍开放。Talk构想和旧Web/Tauri混用不列阻塞；三冻结副线/v464固定包/用户release97保持，不启动Loom原生迁移，不提交推送发布部署，总迁移active。

## 当前 v535：端点键盘连线

Windows原生分支现在支持从当前文档/Live宿主按`F6`进入本节点可见端口，`Shift+F6`从末端口进入；端口条内`Tab`/`Shift+Tab`及方向键遍历当前工作区可见且可交互的端口，`Home`/`End`到首尾。`Enter`/`Space`切换名称展开，`Ctrl+Enter`从输出选择来源，或从已有连线输入选择原来源，再在另一节点输入提交。`Esc`先取消选源，再次退出；`F6`直接退出并回到仍属原owner的宿主。长按/松键不泄漏到文档删除或Live源输入，Ctrl+Shift+4也可从端口条操作。

本批选定**75项Rust passed**，12项新增，选定终态0 failed/1既有默认ignored，非整库。覆盖实际自有端点HWND焦点、遍历/首尾/反向、展开、长按和松键、唯一连线提交、重连取消、提交前锁定、隐藏端口、过期owner、捕获、外部失焦、鼠标接管及同步SetFocus重入；原chrome/参数面板/结束会话/Live输入取消直接依赖同批验证。官方fmt、版本、全仓strict行数、strict all-targets Clippy、固定SHA库、普通Release及workspace目标通过。9项源码/版本delta均UTF-8无BOM，3个新文件，Rust最大337有效行，无新增例外。首次runner因误选原默认ignored鼠标捕获项而拒绝，原回执保留；只读重解析原日志并核未改源码声明，未重跑成功测试、不把ignored计通过。

前台限定恢复有效，仅新profile、自有测试窗。本批只运行未切换独立desktop/自有Job回归；没有普通前台输入。v534工具未返回普通产品窗口的边界仍开放，不猜HWND、不换其他自动化API、不重试同条件失败，不将隐藏HWND通过称普通可见键盘通过。可编辑capsule内嵌控制、普通完整Art/Live/键盘及人工IME、普通双产品、物理多屏/跨机/DPI、剩余全量/合计资源长稳、最终完整隔离包及用户全面验收仍开放。Local签名默认显隐字段当前合同未提供；Talk构想和旧Web/Tauri混用不列阻塞。三冻结副线/v464固定包/用户release97保持，不做Loom原生迁移，不提交推送发布部署，总迁移active。

## 当前 v534：端口显隐与持久化

端口显示现在支持Backend单端口`defaultVisible`优先、旧能力级`defaultVisibility[name]`回退、最终缺省显示；用户`portVisibility`按名称最高优先，同名输入/输出共用覆盖。Document节点菜单、参数面板和Live系统菜单共用原生“端口显示…”入口，列表可选使用默认值/显示/隐藏，取消丢弃草稿。参数面板先原子提交有效文本，再重绑定本次后继操作。隐藏端口从绘制、稀疏region和命中移除，可见位置紧排但保目录索引；原连线/执行权限/拓扑不变。

本批选定**84项Rust passed**，21项新增，选定终态0 failed/0 ignored，非整库。覆盖中文/非法/重复/null/容量、缺省优先级、外观独立、真实LISTBOX/COMBOBOX通知、取消/关闭/DPI/affinity/字体回收、单槽/过期owner与Catalog、实际参数按钮事务、隐藏端点和原边保留、保存重开/未知字段/reset及Live真实worker保存/Receipt失效/冻结后再保存重开。官方fmt、版本、全仓strict行数、strict all-targets Clippy、固定SHA库、普通Release与workspace目标通过。39项源码输入delta均UTF-8无BOM，10个新增文件，受影响Rust最大497有效行，无新增例外。

前台权限仅本批新profile、自有测试窗。普通Release使用预置隐藏output覆盖的独立profile，本次没有取得普通产品可见外观证明；工具输入调用为0，不操作用户旧窗口、不注入全局输入、不改显示设置。该观察不冒充普通产品对话框交互/完整键盘或人工IME；本次自有Job终止后的原5秒父进程等待失败，未通过严格清理/自然退出门；随后只读核本批产品残留为0，不重放终止或UI，也不以稍后归零覆盖原失败。

Local v2签名端口当前没有默认显隐元数据，仍缺省显示，不从未签名提示伪造包默认，本批不改Loom/wire。可编辑capsule、完整端点键盘、普通完整Art/Live/键盘与人工IME、普通双产品、物理多屏/跨机/DPI、剩余全量/合计资源长稳、最终完整隔离包及用户全面验收仍开放。Talk外部会话控制构想及旧Web/Tauri混用按用户决定排除。三冻结副线/v464固定包/用户release97保持，不启动Loom原生迁移，不提交推送发布部署，整体迁移active。

## 当前 v533：Local Art 自定义签名端口

Local Art 现在从 Loom 已验证的不可变安装包取得输入/输出的真实名字和类型；Otter 接入目录、图像输入、来源配置凭证、自定义主图片输出继续 Add Art、正式结果及保存重建/有限离线。显式零输入/零输出保持为空，不伪造 input_image/output_image；只有无新证明的旧包保留默认端口。首次自定义主输入迁移在候选 Workspace 内提交，冲突保原图；零输入只撤回本次工厂创建且仍完全一致的暂定边，保存恢复不继承删除旧边的权限。端口保存记录仅是包绑定提示，不是执行/离线授权。

本批最终选定 **83项Rust passed**，其中17项新回归；当前选定终态failed=0，不是整库。签名域/篡改/预算、旧能力协商、拒绝不重放、图拓扑、中文自定义输入、真实2×2红像素、Live配置换代拒迟到、零输入不交付像素、自定义rendered输出继续建子Art及Save/窗口重建/有限离线均已验。真实Loom LAN HTTPS跨进程联合在线和停服后离线独立核signedPorts、完整资源、正式像素和正式结果；不完整缓存拒绝，父子不混加。Otter strict all-targets Clippy、两仓fmt、官方版本/strict行数、Otter lib/普通Release/工作区与Tile驱动构建通过；Loom签名安全/daemon测试库通过。66项源码delta均UTF-8无BOM，Rust最大500有效行，无新增行数例外。两个原跨仓grant门及两个SDK示例默认ignored未运行；Loom整仓strict Clippy旧债未重跑，不称两仓全绿。

用户已恢复仅新profile、自有测试窗口的前台验收。本次普通固定SHA Tile EXE三类图片/Live/Art混合可见门通过；Computer Use只激活工具唯一返回且路径/标题匹配的本批标准启动器，刷新成功后用单会话nonce同步，driver再核真实前台并只向已核自有Tile转交。独立桌面采样核三路换帧、冻结/黑场/恢复、正常关闭及来源附件/值/Live保留与连接回收，不以后台paint或源像素代呈现。没有操作用户旧应用、注入全局输入或修改显示设置；不等完整键盘/人工IME或普通源EXE全部通过。

逐端口显隐/默认值与持久化、可编辑capsule/完整端点键盘仍未完成；普通完整Art/Live/键盘与人工IME、普通双产品、物理多屏/跨机DPI、剩余全量/合计资源长稳、最终完整隔离包和用户全面验收仍开放。Talk外部会话控制构想和旧Web/Tauri混用按用户决定排除。三冻结副线/v464固定包、用户release97保持；不启动Loom原生迁移，不提交推送发布部署，整体迁移active。

## 当前 v532：Live 源独立添加 Art

Live窗口的精确Shift+Digit1和系统菜单“添加 Art”复用原搜索目录窗；原Live保持存活，创建独立子Art和真实Live→Art边，不冻结或转换源。Backend保默认参数和Upstream自动门、Live快照及原输入worker/TLS执行；Local仅沿显式HTTPS、lease/issuer、describe→prepare→签名child原链。打开选择前沿原通道停止源输入，取消/来源或目录/分组/工作区/会话变化及容量失败保源，不留下半条边。

最终input4同源定向 **62项Rust passed / 0 failed / 0 ignored**，含9项新测试，不是整库。真实owned目录窗/搜索/IME合成/affinity、独立子节点/默认值、取消/过期/容量、输入停止/单槽/空闲门、Backend真实回环TLS执行和Local签名child原始2×2红像素/源关闭后迟到拒绝已验。官方fmt/版本/全仓strict行数、strict all-targets Clippy及lib/普通Release/workspace三目标构建通过；21份Rust最大496有效行，没有新增行数例外。只分配一次v0.2.33.532，Loom源码未改。

input1私有方法/Option编译错误、input2 nonminimal_bool和input3测试失败均保留。input3首帧测试误把原自动等待当手动缺帧错误，另两项共享锁中毒；隔离补证来源复核通过，会话测试另证缺生产with_owner scope。input4仅修新测试：保自动等待与精确手动错误oracle，正确安装query/end owner并断言pause/resume；没有改变生产策略、放宽断言或提高期限。

仅新profile、自有窗口的前台恢复授权仍有效。本次以新空profile启动普通固定SHA产品，而非重放旧image/chrome启动。Computer Use的list_windows只返回一个无标题自有对象，其截图包含透明层下其他应用内容，accessibility为null；list_apps重新枚举仍只有同一自有对象，并混列用户release97。没有可独立选择的自有普通文档/Art/Live目标，故未激活或发送UI输入，没有选择用户release97，不改样式/捕获保护/输入法或换API绕过；不判定产品显示根因。 自有Job保底清理不等于自然退出；普通前台与人工IME不从隐藏测试推定。

Local 自定义主输入名仍缺完整目录/图/状态/保存恢复接线；逐端口显隐及默认值/持久化、可编辑capsule和完整端点键盘仍开放。普通Art/Live/完整键盘与人工IME、普通双产品、物理多屏跨机DPI、剩余全量/合计资源长稳、最终完整隔离包和用户全面验收仍须继续。Talk构想与旧客户端混用排除，三冻结副线/v464固定包保持，不启动Loom原生迁移，总迁移active。
详见[构建记录](WINDOWS_NATIVE_BUILD.md#v532-live-源-add-art-与独立子节点)。

## 当前 v531：三态、公共端点与实际拖线

Ctrl+Shift+4三态已接独立原生chrome owner、Unit外框/公共端点/名称展开、跨窗拖线及已连接输入重接；Live旧绿框/黄角及其命中同时服从none。菜单和手势共用唯一Catalog/端口/两端锁与隐藏/工作区提交围栏，取消保旧边、有效drop才替换；这是相对legacy立即删旧边的明确安全差异。弱引用UI、owned稀疏端点条、图/几何复用、捕获保护、预算及系统会话/保存/退出清理接线见[构建记录](WINDOWS_NATIVE_BUILD.md#v531-工作区三态与跨窗端口连线)。
input5相关Rust去重90 passed/0 failed/2默认ignored，含19项新chrome真实HWND回归及Live去框/恢复GPU像素；严格静态与三目标构建通过，29份Rust最大500有效行，不是整库。input4首paint夹具失败由独立custom-class Win32实验定位，input5只补cfg(test)真实paint并保原断言/失败；没有改生产来迎合测试。Live runner对默认ignored的拒绝另行解释，不能计为通过。
普通Release已启动，但工具仅返回单一本轮候选、截图包含其他应用，不能独立选中自有文档/端口；本次未激活或输入，不操作release97/已有应用，不改样式/捕获保护绕过。自有Job清理归零但非自然退出。前台恢复授权仍有效；普通可见三态/连线、Art/Live、完整键盘/人工IME未验，不能从隐藏测试或工具截图反推通过/产品根因。
per-port显隐/默认值/持久化、可编辑capsule和完整端点键盘仍缺；Live源Add Art、Local自定义主输入名、普通双产品、多屏跨机DPI、剩余全量/合计资源长稳、最终完整隔离包和用户全面验收继续开放。Talk构想/旧混用排除，三冻结副线/v464保持，总迁移active，无提交推送或发布部署。

## v530 自有前台恢复：可见参数与自然退出补证

用户已明确恢复仅新profile、自有测试窗口的前台验收；该授权优先于下方v530物理Escape历史停点。不操作既有应用窗口、无无目标全局输入、不改显示/输入法设置，工具越界/停止仍立即停。复用原input3固定SHA普通Release，不重分版本、不重编或重跑已通过Tile父门。
本次实际观察四象限图片和原生工具面板；裸Tab后可访问性树出现钉图、重新加载、接收上游图片及输入image/输出output。未取得可独立选择的owned参数窗及完整控件/视觉证明，不算完整Tab验收。点击已观察的自有文档标题栏关闭后，产品和Job在关闭Job前自然active=0；forced=false，gracefulProductShutdownProven=true，只证明本轮普通文档自然退出，不覆盖旧v527清理失败或其他Live生命周期。
Ctrl+E与Shift+1均未取得可确认的产品响应。独立自有Tk诊断窗只记录自身事件：Tab送达；两种Ctrl+E写法没有本地Ctrl/E事件，Shift+1记录65281/229及49的keyup，而非预期49的keydown。此为当前送达/IME路径异常证据，不足以判断原生产品收到预期快捷键，更不能宣布产品根因修复；没有改输入法、全局热键、窗口样式、捕获保护或使用另一输入API绕过。普通Art搜索/添加、完整键盘/人工IME仍未验，不能把隐藏测试或本诊断当作通过。
证据位于linshi/otter-native-parallel-20261029/main-foreground-resume-530-1，含ui-observations.json、两个自有Job终态、key-events.jsonl和final-audit.json。没有产品源码/版本/依赖变化；原69项定向Rust及构建成绩仍只属于原v530回执，未新增运行计数。Live源Add Art、Local自定义主输入名、三态外框/可视端口连线、普通双产品/物理多屏DPI、剩余全量/资源长稳与最终整包/用户验收继续开放，整体active；Talk构想、三冻结副线和v464不重开。

## 当前 v530：Shift+1 可搜索 Art 列表与独立子节点

精确Shift+Digit1已接原生同步owned列表，搜索基于原完整能力目录、保原index，disabled拒绝添加；刷新退出后才交付原异步目录任务，模态不联网。选择后创建独立Art和源连接，不转换源图/源Art；普通能力保原默认参数、主图片输入选择及源notify/子listen/upstream自动门，不制造Manual执行。Surface只消费已显式选择的HTTPS origin，并走原本地describe/验签/审批链，WSS目录不派生HTTPS权限；同Art也强制创建子节点而非重启源。
候选窗口/拓扑先准备，发布前复核工作区、源HWND/State/configuration/MenuTarget、目录Rc及显式origin/会话状态；失败回收候选且不留半连接。每文档单槽意图纳入restore/系统会话/退出准入；真实EDIT/LISTBOX、首显前捕获排除、DPI保草稿、IME合成边沿和嵌套模态暂停过滤有定向保护。
同源定向 **69项Rust passed / 0 failed / 0 ignored**，含16项新增；官方fmt/版本/全仓strict行数、strict all-targets Clippy及三目标构建通过，不是全库。普通前台状态`STOPPED_BY_USER_PHYSICAL_ESCAPE_BEFORE_UI_INPUT`：Computer Use 在首次 list_windows 时报告用户按下物理 Escape，本轮前台验收立即停止；未选择窗口、未发送任何 UI 输入，普通产品 Art 模态/搜索/键盘仍未验。 合成IME/隐藏窗口不代人工输入法或普通Art完整业务，兜底Job清理不代自然退出。
本批只覆盖普通图片/Art文档源的Add Art入口；Live窗口源的Shift+1尚未接通。Local沿既有input_image/output_image合同，未实现任意目录自定义主输入名Surface的泛化映射；不能据此宣称完整Add Art对等，也不将metadata或只读端口摘要冒充完整可视端口/连线。Ctrl+Shift+4三态chrome、参数外围/候选预览/Live发布观看、普通双产品/物理多屏跨机DPI、剩余全量/合计资源长稳及最终整包和用户全面手测仍开放。Talk构想、旧Web/Tauri混用、三冻结副线/v464固定包不重开，总迁移active。详见[构建记录](WINDOWS_NATIVE_BUILD.md#v530-shift1-原生-art-入口)。

## 当前 v529：Tab 原生参数编辑窗与事务闭环

宿主裸Tab已接到文档专属、惰性modeless参数窗；普通文本/数字/JSON为真实EDIT，选项保原Value类型，Slider/step、禁用/重置和执行设置复用原业务。颜色/文件仍走原显式对话框，不新增执行worker或参数状态。最多4个面板、每页16行、4项意图；同页文本先完整验证到候选，再取消旧执行并原子发布，同次Run可升级当前参数触发为Manual。后缀选择按参数id/动作合同唯一重绑定，不沿用可能移位的旧索引。
外部配置/上游更新及DPI不能静默销毁dirty/IME；旧凭证仍拒绝，明确重新加载才丢草稿。已知旧连线视图不能当成无连线；捕获保护在首次显示前继承，失效HWND按userdata marker拒绝，销毁同步撤捕获登记，托盘/组隐藏不自动复活草稿。
同源定向 **91项Rust passed / 0 failed / 0 ignored**，含23项新增；官方fmt、版本、全仓strict行数、strict all-targets Clippy及三目标构建通过，不是全库通过。普通前台已观察Tab参数窗的真实可访问性，但工具未返回独立owned目标，按钮点击被边界拒绝；状态`PARTIAL_UI_OWNED_WINDOW_OUTSIDE_TARGET_BOUNDS`，完整视觉/面板控件键盘/人工IME仍未验，不修改样式或捕获保护迎合工具。自有Job严格清理通过，未证明自然退出。
本批不是完整Tab对等：常驻可视端口/跨窗连线、候选预览、Live发布/观看面板仍开放；只读端口摘要不冒充连接画布。Ctrl+Shift+4三态Unit chrome、Shift+1 Art入口、其余N01–N12、普通双产品/物理多屏跨机DPI、剩余回归/合计资源长稳和最终整包/用户全面验收继续；Talk构想、旧混用、冻结副线及v464固定包不重开，总迁移active。详见[构建记录](WINDOWS_NATIVE_BUILD.md#v529-tab-原生参数编辑窗)。

## 当前 v528：Ctrl+E 工具面板键盘闭环

原生 Ctrl+E 已接到现有编辑工具面板，保留 F4；面板/子按钮焦点下可关闭，首次按下才显隐，重复消息仍消费。普通 E、Ctrl+Shift+E、AltGr/Ctrl+Alt+E、Tab、Shift+1 与 Ctrl+Shift+4 不被挪用。沿同宿主/面板身份和同步重入复核，保截图、共享模态、组锁/隐藏及系统会话冻结；不修改文档、样式、图像或编辑选择，不增加线程/依赖/全局输入。
同源定向 **32项 Rust passed / 0 failed / 0 ignored**，含7项新增键盘测试；fmt/版本/全仓strict行数/strict all-targets Clippy及lib/普通Release/workspace三目标构建通过，不是整库通过。普通前台状态为 `UNVERIFIED_CAPTURE_IDENTITY_MISMATCH`，不能以隐藏窗口测试替代实际验收；前台清理严格门为 `True`，兜底清理不等正常退出。详见[构建记录](WINDOWS_NATIVE_BUILD.md#v528-ctrle-工具面板键盘闭环)。
本轮另核明：旧三态Unit chrome及常驻可视端口/连线没有查到原生对等owner；标注选框不是Unit外框，不能简单隐藏标注冒充完成。Tab参数面板、Shift+1 Art入口也仍缺原版对等接线。普通Tile/历史完整键盘、人工IME/旋转EDIT、普通双产品、物理多屏/跨机/DPI、剩余回归/合计资源长稳、最终整包与用户验收仍开放。Talk构想、旧客户端混用、三冻结副线和固定包不重开，总迁移active。

## 当前 v527：补齐截图/取色历史，前台仍部分未验

原版Ctrl+H同屏两列表已接入原生：成功截图/取色各24条、颜色去重、240像素缩略图复用、确认后仅删历史。普通打开/粘贴/历史复用不入栈；独立原生profile持久化，坏源/竞争停止写，暂态预览失败保旧条目，不自动迁移旧目录。单后台槽/四份共享截图队列和宿主Rc围栏保护重入、迟到与退出。
同源定向 **55项Rust passed**（含16新增历史测试），官方格式/版本/strict行数/strict Clippy及三目标构建通过。普通Release已重编；workspace产物提取脚本失败后按Cargo目标唯一固定现有产物，不重复构建；失败保留。当前库5315项，不以此计算迁移百分比或宣称全绿。
前台主窗可选且已发送Ctrl+H，但工具未返回历史owned窗，视觉/Tab/Enter/Space/复用未验；自有fixture的原5000msJob清理等待失败，随后只读核无该产品进程，不冒充严格/正常清理通过。详见[本批构建记录](WINDOWS_NATIVE_BUILD.md#v527-原版-ctrlh-截图与取色历史)。自有前台恢复授权有效，不反复询问或改样式迎合工具。
其余N01–N12、完整键盘/人工IME/旋转EDIT、普通双产品、物理多屏/跨机/DPI、剩余回归/合计资源长稳和最终完整包/用户验收仍开放；Talk构想、旧客户端混用、冻结副线及固定包不重开，总迁移active。

## 当前 v526：六项 JS 前台主体补证通过，整体仍 active

收紧测试夹具的 HWND 所有权并移除 AttachThreadInput 兜底，沿用原 READY→G，保留真实前台断言。新增 3 项安全回归、原先仅在 foreground 前提失败的 6 项 Application 主体及 2 项 guard 共 **11 项 Rust passed**；八项前台测试由本轮自有启动器逐项点击启动，使用独立新 profile，Job 关闭前自然 active=0，没有全局输入或用户窗口操作。
官方 fmt/版本/全仓 strict 行数/strict all-targets Clippy/lib 构建通过，只分配一次 v0.2.33.526；产品源码和普通 EXE/整包未变。两次未授 G 的旧启动失败保留，详见[构建记录](WINDOWS_NATIVE_BUILD.md#v526-前台夹具所有权收紧与八项真实断言)。
新库 5299 项；跨版本台账 3494 至少一次通过、61 默认 ignored、1743 未启动、1 原整体中断，不是新库全绿或迁移百分比。Art 间歇性和 Live 原清理原因仍开放；新启动器可选不等普通产品键盘工具边界已解除。完整键盘/人工 IME/旋转 EDIT、普通双产品、多屏/跨机/DPI、剩余回归/资源长稳及最终包/用户验收仍开放。自有前台授权有效，Talk 构想、旧客户端混用及冻结线不重开。

## 当前 v525：再补 914 个未启动项，整体仍 active

接住在途纯模型后，按已审查边界分五批运行此前未启动的完整名，115 个隔离进程组取得 **903 passed / 0 failed / 11 默认 ignored**。覆盖离屏字体/绘制、协议、loopback、合成长截图、独占文件及四阶段进程中断恢复；不代表真实截图或前台键盘。原测试体、期限、安全围栏和 ignored 不变，4441 输入与库 SHA 一致，没有新产品源码、版本或编译。
跨运行完整名台账为 3483 至少一次通过、61 默认 ignored、6 无通过证据、1745 未启动、1 项原整体上限中断；不是单次全量通过或迁移百分比。原 Art 间歇性、Live 严格清理未知和六项 JS foreground 前提仍不关闭；runner 自有 Job 的兜底清理不冒充自然退出。详见[构建记录](WINDOWS_NATIVE_BUILD.md#v525-剩余回归分组续接914-项终态非全量通过)。
自有前台授权持续有效，但本批没有前台输入或重放工具不可选旧启动。完整键盘/IME/旋转 EDIT、普通 Source/Loom 桌面壳、多屏/跨机/DPI、剩余回归、合计资源长稳及最终用户验收仍须继续；Talk 构想、旧客户端混用与三冻结副线不重开。

## 当前 v525：分组补证完成，焦点与间歇性边界仍开放

原 162 项 SERIAL 连锁失败逐项独立进程运行已终态，154 passed / 8 failed / 0 PoisonError；没有修改产品、锁或断言。当前 desktop 的 32 项呈现与 1 项 CPU 编辑保存恢复均通过；9 项 Live Source 行为断言通过，但其中 1 项原严格 Job 清理门未通过，不能称自然退出全绿。另补此前未启动的纯模型 124 项全部通过。
六项 JavaScript 失败定位在真实 foreground fixture 前提，而非后续焦点/键盘/拖拽主体；未把其 SetForegroundWindow/AttachThreadInput/SetCapture 路径混入无焦点呈现批。Art 原 settle 超时在一次完整回溯诊断中未复现，Live 原清理异常在一次 PID 时间线诊断中也未复现；不因单次后来通过宣称根因修复。
全部运行保持 4441 输入及固定库 SHA，未重编或重分版本。完整名累计尚有 2659 未启动、1 项被原整体上限中断、6 项无通过证据；已通过集合还保留 Art 间歇性和原清理未知，不是单次全量成功。证据与边界见[构建记录](WINDOWS_NATIVE_BUILD.md#v525-分组回归补证与剩余边界)。
自有前台授权有效，用户现有窗口/全局输入/物理显示设置不操作，SecureDesktop 安全门不放宽。普通产品键盘工具不可选、人工 IME/旋转 EDIT、普通双产品、多屏/跨机/DPI、合计资源长稳、最终完整包和用户验收仍开放；N01–N12 整体 active，不重开 Talk 构想、旧客户端混用或冻结副线。

## 当前 v525：全库审计未通过，区分环境失败与连锁失败

在途默认库运行已于原 1800 秒上限结束：5296 项中观察到 1891 passed / 205 failed / 48 ignored；1 项在途被终止、3151 项尚未启动，不是全量通过。
其中 162 个失败的原始错误为共享测试串行 Mutex 的 PoisonError；其余 43 项不自动归为环境问题。两项最初 checkpoint 失败用相同库/源码/期限，仅在自有当前 desktop 条件对照后均通过并取得 D3D12 S_OK；不覆盖原失败或推及其他测试。
独立执行原先尚未开始的五个纯模型/图像/文档根模块另得 366 passed / 2 默认 ignored，未重编或分配版本。完整名、错误与清理证据见[构建记录](WINDOWS_NATIVE_BUILD.md#v525-默认库审计与呈现环境对照全量尚未通过)。
普通产品键盘尝试因工具返回无可选窗口而未验，尚未配对、打开输出或输入；临时 TLS 已关闭，自有 Job 已回收，不冒充正常产品退出。
自有前台授权继续有效；不绕过 SecureDesktop/输入安全围栏，不操作用户窗口、全局输入或显示设置。下一步按真实环境分组处置连锁失败、独立失败及剩余未执行项，而非原样重跑私有 desktop 全量命令。
普通可见 Tile 两门和内部 ZIP 成绩保留；其余 N01–N12、完整键盘/IME/旋转 EDIT、普通双产品、多屏/跨机/DPI、资源长稳及最终用户验收仍未完成。Talk 构想、旧客户端混用和冻结副线不重开。

## 当前 v525：自有前台已恢复，普通可见 Tile 两门通过

用户明确恢复本轮新 profile、自有测试窗口的前台验收；下方历史“继续暂停”不再是当前边界，但用户现有窗口、全局输入和物理显示设置仍不操作。
实际解包的 v525 普通 Release 已通过图片六阶段及图片/Art/Live 三路换帧、冻结/黑场/恢复和来源保留清理两条可见父门，原 30 秒及桌面像素/遮挡/服务 ACK 断言不变。
内部 ZIP 及解包工作区门另已通过；没有新产品源码、版本或构建。证据与包 SHA 见[构建记录](WINDOWS_NATIVE_BUILD.md#v525-解包产品与恢复授权后的普通可见验收)。
这关闭当前本机单屏的两项可见验收缺口，不改写历史失败；混合 Source 为测试库夹具，Loom 不是普通桌面壳，不等普通双产品/跨机验证。
完整键盘/人工 IME/旋转 EDIT、历史根因、物理多屏/DPI、全量回归、合计资源长稳及最终完整包/用户验收仍开放；N01–N12 整体未完成，Talk 构想与旧客户端混用仍排除。

## 原版紧急退出双通道（v524，隔离定向通过）

补齐原版已实现的两条紧急退出：完整 Esc+Delete 三次组合（相邻严格小于400ms，双键重叠且完整释放），以及独立 watchdog 的 Ctrl+Alt+Shift+F12。主消息循环与 child 共用纯状态机；child 在配置/UI 前分流，先见松开才 arm，以原版8ms周期只读键位，不注入输入、不记录文本。
复用原同EXE受限白名单 launcher 和 Hello/Ready；直接父PID、创建时间、同EXE路径及pipe创建者共同认证。普通恢复权限仍只有查询/等待，只有紧急认证额外取得 PROCESS_TERMINATE，并只使用保活原父句柄。正常Drop和握手失败只回收自己新建watchdog，不终止Job或承担Live恢复责任的child。紧急退出绕过保存/析构，不能保证未保存工作保留；原Live父死恢复机制继续负责已登记源。原生未修改全局光标约束，故不无条件重置其他程序的 ClipCursor/ShowCursor/SetSystemCursor。
当前选定Rust按crate/完整名去重 106 passed、6 默认ignored，非整库；11项新增回归覆盖严格400ms边界、完整释放、启动已按住、缺失keyup/迟到消息、参数、认证失败不arm、握手失败、正常Drop及阻塞牺牲父进程的三连/备用终止和父死child退出。另复测原恢复认证/握手/管道/launcher、三进程父死恢复、系统会话消息/Checkpoint和普通固定SHA EXE工作区。全部在未切换的独立desktop/自有Job执行，不申请SWITCHDESKTOP，不向用户应用发送输入。
strict all-targets Clippy、lib/普通product/workspace/恢复认证及session目标构建、官方fmt/版本/全仓strict行数/diff通过。input1因Application达到503有效行而在构建前失败，原回执保留；input2按职责提取application_loop，保持原消息泵/显示恢复/保存/退出顺序，不增行数例外。input2 Clippy通过后，因只读审查发现局部消息可漏收keyup，停止本轮自建在途构建树并保留中止记录；input3主通道按当前键位快照保守累计，不凭历史消息制造重叠，新增负向回归。源码只分配一次v0.2.33.524，Loom未改。
新增常驻进程单独实测180秒（97点）：产品及watchdog全程存活，含idle60秒和三轮hidden20秒/resumed20秒；正常隔离QUERY/END后整棵自有树归零、desktop关闭、工作区保留。该4进程树（产品、watchdog及两个conhost）Private commit为8.691–8.957MiB，Working set合计61.430–61.578MiB（共享页可能重复计），模式内单核CPU约0.156–0.260%，Handles658/GDI33/USER16稳定。Ctrl+1仍记录已有热键占用0x80070581，不冒充快捷键验收。资源明细见main-emergency-exit-1/runtime-audit-3.json；不是Otter+Loom总资源、GPU、物理输入或长稳验收。
本批仅关闭源码与限定隔离门，不恢复前台自动化，不证明物理三连/F12或普通可见混合链通过。v518前台身份变化来源及v517普通可见混合门未关闭，不拿当前隔离通过覆盖历史失败。人工IME/旋转EDIT、物理多屏/跨机/DPI热插拔、资源长稳及最终隔离整包/用户全面手测仍开放。Talk外部会话控制按用户决定排除本轮，旧Web/Tauri混用不重列必做；三冻结副线、v464固定包及真实profile/设备不动，总迁移active，不是正式发布。

## 当前范围：不实施 Talk 录音开关构想

2026-11-03 用户确认原版 Otter 未实现 Talk 外部会话控制，本轮不实现该构想，
也不把 Ctrl+Alt+Space/start/stop/toggle/cancel 列为必做或验收阻塞。
本说明优先于下方历史批次的 Talk 待办；其余目标不变，详见[当前范围](WINDOWS_NATIVE_REVISED_PLAN.md#talk-录音开关构想不属于本轮迁移)。

## 系统结束会话持久化准入（v523，隔离消息链验证）

WM_QUERYENDSESSION只消费原Save/Checkpoint已提交凭证；未保存、在途/排队请求、Live待交付操作及本地Art输入/确认/结果未结算时立即拒绝，设置短ShutdownBlockReason，不在同步回调等待保存或弹框。批准后共享Queried阶段封住新业务和标准子控件编辑；END FALSE恢复，END TRUE不可逆进入Ended并沿原统一shutdown排空。系统结束不等于删除最后Unit，不写空工作区。
本批只关闭源码、选定回归及普通固定SHA EXE的隔离系统消息链，不实际注销/关机，不恢复前台自动化，不证明强制结束/断电可保存。v518前台变化来源、v517普通可见混合门、人工IME/旋转EDIT、物理多屏/跨机/DPI热插拔、资源长稳、Talk真实start/stop/cancel合同、其余退出条件及最终隔离整包/用户全面手测仍开放。总迁移active；三旧副线、v464固定手测包及真实profile/设备不动。


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

## v520 本地Art自动触发原生集成

原生显式配置→原Live唯一控制worker Configure/Claim→签名child onLiveTrigger→原Application/Results实际事务→ACK及Stop/回收已接通。独立不可逆自动授权绑定完整实例/工作区/包、会话/epoch与条件revision；Preview不等授权，重连不复活旧gate。单槽/16KiB、原期限与共同容量门不变，不新增线程、timer、持久队列、权限或WebView。
input4相关Rust按crate/完整名去重87 passed、7默认ignored，非整库；新自动回调子门虽默认ignored，已由真实LAN父门显式运行且父子不混加。真实签名child验证暂停回调/缺handler/迟到拒绝；隐藏Application验证提交前无Applied、取消保canonical、正式像素及Save恢复、实际present后NoResult。原本地Art缓存/有效授权离线/结果/宿主退休和Live观看/控制、普通固定SHA EXE工作区5项直接回归通过；前台退出门未运行。
新真实Loom LAN HTTPS/WSS联合6.656秒通过：只改自有离屏EDIT value-seven→value-eight，独立像素[17,83,211,255]、公开SDK同份源码签名执行、data计数1及deliveryId/观察匹配、原Results/Save恢复、Editor不变、独立Device Inspect确认服务端acknowledged/applied(1)，显式Stop及两端owner/媒体permit回收。经产品菜单的同一提交接缝，不宣称真实用户点击了模态菜单；无真实设备或用户程序输入注入。
本批仅关闭本地Art自动触发的源码/定向/隐藏Application及真实LAN联合接线，不是普通可见双产品、真实跨机或最终包验收。v518全局前台身份失败、v517可见混合门/v511历史键盘根因仍开放，前台自动化继续暂停；人工IME候选窗/旋转活动EDIT、物理多屏/跨机/DPI热插拔、长稳、其余迁移退出条件、最终隔离整包与用户全面手测仍开放。三旧副线、v464固定手测包与真实profile/设备保持；旧Web/Tauri混用不重列必做，总迁移active。 见[构建记录](WINDOWS_NATIVE_BUILD.md)。

## v519 本地Art Live触发服务

宿主显式automatic_local_callback授权绑定完整Art实例、工作区/代际/包、Live会话/epoch、条件revision及独立authorizationId；Preview不等自动授权。Claim在HTTP响应前消费payload，重复/未知领取不重放；ACK仅报告宿主结果，不证明canonical保存或接收端呈现。本地delivery至多16KiB，超限明确停用；原观察新鲜度30秒、grant期限、墙钟高水位/每事件期限与单调期限共同围栏，查询不续期。停止、最后Viewer断连、审批修订变化、过期和Session close回收或拒绝执行，重连/重批不复活旧授权。没有新增线程、timer、持久队列、依赖或脚本权限。
input2选定相关Rust完整名按crate去重75 passed、1默认ignored，非整库；protocol39/daemon36，其中新增协议3项、服务12项。原Art观看/控制、Surface触发/观察、媒体断连清理与LAN边界回归通过，普通daemon构建及--version/--help通过。HTTP测试用owner直接登记Viewer在线，不是真实WSS或Otter联合。20份Rust最大457有效行；官方格式、版本、Otter全仓strict行数、两仓diff及精确UTF-8无BOM/空白门通过。
本批只关闭Loom本地Art Live触发服务实现及HTTP定向验证；Otter实际配置菜单、独立自动授权owner、onLiveTrigger私有command/SDK、签名child执行、正式结果/取消接线和真实HTTPS/WSS联合仍由MAIN继续完成，不能以此宣布本地Art触发产品完成。v518全局前台身份失败、v517可见混合门、v511历史键盘根因仍开放，不恢复前台自动化。人工IME候选窗/旋转活动EDIT、物理多屏/跨机/DPI热插拔、长稳、其余迁移退出条件、最终隔离整包与用户全面手测仍开放。三旧副线、v464固定手测包及真实profile/设备保持；旧Web/Tauri混用不重列必做。 见[构建记录](WINDOWS_NATIVE_BUILD.md)。

## v518 Tile取消实现与当前验证边界

Tile Popup明确区分Select，真实打开的下拉列表先消费Esc并抑制同次长按；keyboard_window以Rc身份围住取消/同步销毁，只在同一已paint快照保留Tab位置，迟到epoch不得取消后继owner；外部失焦仍完整退休，父Tile拒同次Esc长按连带关闭，新鲜Esc仍退出。取消退休未发Edit后缀，不重放未知写或丢已接受本地值。
input4当前264项相关Rust完整名去重passed、1 failed、5 ignored，非整库；Tile窗口93项实跑92 passed/1 failed，8项取消及3项夹具新增回归全过。8条真实Loom LAN父门通过，父子不混加：两种文本链在input4重跑62.406/69.218秒，六个非文本门按精确三份cfg(test)差异/SHA复用。普通固定SHA产品工作区5项通过证据复用，前台退出门未运行；product/workspace未因夹具变化重编。Otter strict Clippy/lib、官方格式/版本/全仓strict行数、diff及精确UTF-8无BOM/空白通过，9份Rust最大461有效行。
当前唯一终态失败为tab_edit_to_button_commits_local_draft_and_enter_repeat_does_not_duplicate的全局GetForegroundWindow前后身份断言，尚未确认是用户切窗、产品行为还是其他来源，不用历史通过覆盖、不反复重跑、不恢复前台自动化。故本批仅PARTIAL，不发布INTEGRATION_VALIDATED或完整键盘/整项迁移完成。v517混合可见门、v511历史根因、本地Art Live触发独立授权/执行/回执合同、人工IME/旋转活动EDIT、物理多屏/跨机/DPI热插拔、长稳、最终完整隔离包和用户全面手测仍开放；旧Web/Tauri混用不重列必做，三旧副线/v464固定包及真实profile设备保持。 见[构建记录](WINDOWS_NATIVE_BUILD.md)。

## v516当前普通Tile可见与提交目标闭环

native/tests/tile_product_runtime与support/tile_product_window独占普通固定SHA EXE、真实CLI/物理输出、隔离profile及自启PID/精确类名的窗口验证；Loom tile_product_native_joint只负责真实生产库LAN HTTPS/签名配对与续签、layout/presentation及权威ACK。四象限独立RGBA常量对应六阶段running/旋转/黑场/恢复/crop/冻结，外部仅采四个前后均确认无遮挡的桌面点，随后正常关闭和endpoint offline。桌面合成像素不冒充产品交换链直接读回，库内各后端读回也不冒充普通产品可见运行；父子不混加。子产品由Kill-on-close Job持有，失败/超时只清理自己的进程。初始可见门因遮挡明确失败后，按用户授权由Computer Use仅激活驱动的普通自有启动器一次，驱动核实际前台后向已核PID/类的同测试子Tile转交一次。不以该显式激活声称产品自动置顶或全程无激活；不移动鼠标、不注入用户应用、不改物理显示或真实数据。没有新产品诊断开关、依赖、路由、权限、线程或资源预算，旧三副线及v464固定包冻结。
本批最终选定回执136项相关Rust完整名去重passed、2默认ignored，非整库；早期失败另行保留。三后端实际提交目标和自有窗口负向门四项新回归通过。4条真实Loom LAN父调用通过：新普通EXE单屏可见链，以及原图片/Live/Art三链回归；普通固定SHA EXE工作区5项通过，前台退出门未运行。17份Rust最大489有效行；Otter strict Clippy/格式/版本/全仓strict行数、两仓diff和精确UTF-8无BOM/空白门通过。
本批只关闭上述本机单屏普通EXE静态图片可见链和产品Renderer各后端提交目标读回，不关闭普通EXE可见Art/Live、完整Tile键盘、人工IME候选窗/旋转活动EDIT视觉、物理多屏/跨机/DPI热插拔、长稳、本地Art Live触发、其余迁移退出条件、最终完整隔离包或用户全面手测。v511历史超时根因仍开放；旧Web/Tauri混用按用户确认排除。 见[构建记录](WINDOWS_NATIVE_BUILD.md)。

## v515当前滚动条闭环

滑块可拖出原track/窗口，轨道立即翻页并在400ms后每50ms至多一步，不追赶积压；thumb到指针、移出track或反向时暂停，回到原方向再继续。活动EDIT由滚动条提交一次，保中文emoji草稿及无handler表单；IME组合拒绝接管。prepare空隙只被动保留且不滚动，空隙抬起释放、不采用迟到位置。同步paint前移出owner，取消轮换token，旧回调不得恢复旧手势或覆盖successor；隐藏/冻结/黑场/过期/撤权/DPI/关闭/丢capture及wheel/Tab接管都清捕获和timer。初始资格仍核topmost/crop/旋转/clip/布局/view/代际，捕获后屏外坐标不授新命中资格。不改scene/canonical、网络动作、权限、原队列/像素/TTL预算；无新hook、线程、依赖、WebView或用户应用输入注入。
当前245项相关Rust完整名去重passed、3 ignored、0当前运行终态failed，非整库；12项新回归覆盖四向旋转/crop/非等比映射、COW计划、真实EDIT提交、timer逻辑时钟、prepare/生命周期与同步paint取消/接管/销毁。普通Surface原轨道重复回归同批通过。11条真实Loom LAN父调用通过，父子不混加；新drag/page各有handler与无handler四独立会话，独立常量核唯一input/change/click、实际framework/Art正式结果、已paint及view/线程清理；原十条输入联合重验。固定SHA普通EXE工作区5项通过，前台退出门未运行。
本批仅关闭Tile滚动条实现、定向与隔离LAN集成；inputCapabilities仍pointer。完整键盘、人工IME候选窗/旋转活动EDIT视觉、普通可见Tile与最终交换链读回、物理多屏跨机/DPI热插拔、长稳、本地Art Live触发合同、其余迁移退出条件、最终完整隔离包及用户全面手测仍开放。v511历史超时根因仍未确立，后续成功不反证旧故障已修复；旧Web/Tauri混用按用户确认排除。 见[构建记录](WINDOWS_NATIVE_BUILD.md)。

## v514当前滚轮与屏外显露闭环

滚轮复用系统行/字符/整页偏好、Shift水平与Ctrl让出，嵌套祖先只消费一次；Tab/反向Tab能显露屏外字段，仍受crop、四向旋转、最上层命中和有界搜索约束。重投影重装嵌入文本/Select/Slider绘制缓存，保中文emoji草稿和无handler本地值；同view/代际/尺寸新snapshot继承并钳位scroll，换代重置。IME组合不提交或换焦；隐藏/冻结/过期/准备空隙拒新滚动，旧按压不重新命中新位置。不改权威scene/canonical、表单字段范围、协议、权限、队列/像素预算、原1500ms/120秒门或未知写不重放规则；无WebView、新依赖、hook、线程或输入注入。
当前232项相关Rust完整名去重passed、3 ignored、0当前运行终态failed，非整库。15项新回归覆盖COW、crop/hidden轴、滚动后嵌入缓存、新snapshot钳位、实际EDIT复用/表单、IME、旧press、无handler值、减权、四向旋转与隔离TLS恰一次POST，以及真实prepare空隙后的显露焦点/草稿/待发表单、纯偏移清理与身份隔离。10条真实Loom LAN父调用通过，父/子不混加；新滚动鼠标/键盘各覆盖有handler和无handler两独立会话，独立常量核中文emoji输入记录、唯一change/表单、实际framework/Art正式结果、已paint和view/线程清理。旧文本/IME、离散值、Slider、本地值两种入口重验；普通固定SHA EXE工作区5项通过，前台退出门未运行。
本批关闭Tile本地wheel与Tab屏外reveal的实现、定向和隔离LAN链；不等完整键盘、滚动条拖拽/重复翻页、人工IME候选窗/旋转活动EDIT视觉、普通Tile输出EXE可见链、最终交换链读回、物理多屏跨机/DPI热插拔、长稳、本地Art Live触发合同、其余迁移退出条件或最终完整隔离包/用户全面手测。inputCapabilities仍为pointer；旧Web/Tauri混用互通按用户确认排除，v511旧超时根因仍未确立。 详见[构建记录](WINDOWS_NATIVE_BUILD.md)。

## v513当前待派发与生命周期闭环

已修复重绘过渡中候选publish但未paint导致的未发意图丢失；原worker弱预留在原预算内保事件ID、顺序和期限，实际paint就绪后派发，昂贵准备后至HTTP前再核生命周期。提交后拒迟到结果且不重放未知写；停止/清理/换代退休旧身份，冻结Cancel合同不变。144项相关Rust去重、11新回归、9条真实LAN父调用及普通工作区通过，详见[构建记录](WINDOWS_NATIVE_BUILD.md)。
此闭环不等v511历史超时根因已确立，也不关闭完整键盘/屏外scroll与reveal、人工IME/旋转活动编辑视觉、普通可见Tile与最终交换链、物理多屏跨机/DPI热插拔、长稳、本地Art Live触发合同、其余迁移退出条件、最终隔离包和用户全面手测。总迁移仍active，旧副线与固定包冻结。

## v512当前本地值闭环

无handler值可鼠标/键盘编辑；同合同同源换快照保本地值，新权威源值替换，view/代际/合同/布局/几何变化退休。准备空隙仅被动保留且拒新输入，隐藏/冻结/黑场/停止/撤权/过期/DPI或窗口换代清理；Select取消不变值，Slider取消恢复此前接受值。与文本共享每view64/总256字段，不加倍预算；本地操作不offer空动作。surfaceValues仍只含input/textarea/number字符串，不加入Switch/Select/Slider，保原16字段/48KiB表单门、祖先click、队列/权限/资源预算。
当前274项相关原生/普通工作区Rust完整名去重passed、6 ignored、0当前终态failed，非整库。9条真实Loom LAN父调用通过，共15个独立会话，不把父/子重复累计；新本地值鼠标/键盘联合分别13.875/13.079秒，核零网络动作、同源保值/新权威采用、恰好一次仅含当前文本的表单、实际framework/Art正式结果、已paint以及保Source/回收view和线程。旧文本/离散值/Slider的键盘与鼠标、点击/确认/拒绝/取消联合均重验。真实WIC/D2D像素使用独立常量及既有权威Select基准；自有离屏HWND不冒充普通可见产品或最终交换链。
v511的键盘/Tab接线及当前回归虽已通过，input6的values LAN派发超时仍缺根因证据；后续成功不等生产修复，不发布该批INTEGRATION_VALIDATED，也不把旧失败改写为通过。
本批关闭无handler值控件的实现、定向及隔离LAN集成；不关闭完整键盘/屏外reveal或scroll、人工IME候选窗/旋转活动编辑视觉、普通Tile输出EXE可见链、最终交换链读回、物理多屏/跨机/DPI热插拔、长稳、本地Art Live触发合同、其余迁移退出条件和最终完整隔离包/用户全面手测。inputCapabilities仍为pointer；旧Web/Tauri混用互通已按用户确认排除。 详见[构建记录](WINDOWS_NATIVE_BUILD.md)。

## v510当前文本/IME闭环补充

Tile文本/textarea/number原生EDIT、本地草稿、IME合成边沿、跨字段input/change串行与按钮当前表单值已实现并通过定向和真实Loom LAN隔离联合。普通Surface权限不变，无handler不伪造change；权威/正式结果仍归Loom。详细回执见[构建记录](WINDOWS_NATIVE_BUILD.md)。本批关闭Tile文本/textarea/number、IME合成边沿及文本草稿表单按钮的隔离闭环，不等完整键盘/Tab导航、已安装输入法候选窗人工验收、旋转活动EDIT视觉、全部值控件本地draft/完整表单。inputCapabilities仍为pointer。普通Tile输出EXE可见链、最终交换链读回、物理多屏/跨机/热插拔/真实DPI、本地Art Live触发合同、长稳、其余迁移退出条件、最终完整隔离包和用户全面手测仍开放；旧Web/Tauri混用互通已按用户确认排除。

## v509 当前 Slider 闭环补充

已完成声明了input/change的Tile Slider鼠标拖动、同手势尾合并、抬起change、异步预览及失败/取消依赖退休；新鲜前缀终态只授一次原1500ms派发准入，等待不越过原120秒跟踪门。实现、定向和真实Loom LAN联合已验证，详细证据与剩余边界见[构建记录](WINDOWS_NATIVE_BUILD.md)。只覆盖声明了 input/change 的 Slider 鼠标交互；无 handler 仍只读，文本/IME/完整键盘与完整表单仍开放，inputCapabilities 保持 pointer。普通 Tile 输出 EXE 可见链、最终交换链读回、物理多屏/跨机/热插拔/真实 DPI、本地 Art Live 触发合同、长稳和最终整包/用户全面手测仍开放；旧 Web/Tauri 混用互通已按用户确认排除，不再计为本轮待办。

## 当前验收范围澄清（2026-11-02 用户确认）

本轮不要求新原生 Otter 与旧 Web/Tauri 客户端混用互通，详见
[修订计划的旧客户端互通排除规则](WINDOWS_NATIVE_REVISED_PLAN.md#旧客户端互通不属于本轮验收)。
下方历史兼容/未验说明不构成此项必做需求，不得据此增加剩余任务或阻塞迁移完成、手测包及最终验收；
原生端与配套 Loom、新原生端之间的已承诺功能、数据安全和真实产品验收不取消，历史回执不改写。

## 当前计划：2026-10-28 架构方向修订

[统一修订计划](WINDOWS_NATIVE_REVISED_PLAN.md) 是当前目标与实施顺序入口：D3D12 优先、D3D11 兼容/按需辅助、
CPU 兜底；功能替代后退役 ANGLE；Art JS 源码包由 Otter 单侧执行，Loom 管包/授权/服务。
用户已允许必要 D3D11On12、自动有界恢复、有效授权下缓存离线运行、默认实例独立及指定 Loom 跨仓库修改。
下方冲突的旧兼容前提由新计划替代，但历史成绩/失败和未冲突的 N01–N12 退出条件保留。
v412 已完成同源定向 lib 回归，产品/集成结果见 [构建记录](WINDOWS_NATIVE_BUILD.md#v412无-raster-目标实时外框)。
不继续扩建 ANGLE 长期路线；新架构尚未实现或验收。

## v412：无 raster 目标实时外框

投送 fallback 已接 HWND+原 Space 的实时几何，canonical 只作后台取整提示；缩略 Sticker 使用当前 expanded，Art 不套用。
已有 editor full source/raster/crop 仍优先，几何变化使用原 Document 与 Space 围栏，保存/网络 owner 不变。
5 项专门组及 26 组相关 lib 门通过；不等完整 PNG parity、双后端、源码 Art 单侧执行或最终迁移完成。

## v411：稳定发送关联显式停止

已接稳定 Binding 的独立停止入口，选择保留精确 Binding 与 canonical Source，不依赖仍在线的目标目录。
原 Store 先持久显式减权，canonical 保存 stopPending 后才允许原设备授权下 unlink；成功回执保留到最终 canonical 移除与后台 ack。
失败、换工作区与重启保留原 Request/envelope/PNG，不从绑定重建签名，不将缺记录或远端 404 当作解绑证明。
原文大字段借用、未知扩展、手动文件基线、唯一 Save/Checkpoint、原 Job/Lease/四槽保持不变。
新的持久 stop 字段仅用于显式稳定停止；普通退出、授权取消及在途取消仍维持原合同。旧程序不识别新字段时失败关闭，不能无损降级。
HTTP 已成功但本地回执尚未写入的分布式窗口仍可能停留 pending；不会以自动重签/清日志掩盖。
同源 26 组选定 lib 门及新 stop 专门组通过；四项正式集成中 startup 为 6 过/1 超时，未放宽原 15 秒期限。
产品已封存并通过隔离隐藏 workspace-check，但本切片全部选定门仍未通过；实际证据见 [v411 构建记录](WINDOWS_NATIVE_BUILD.md#v411稳定发送关联显式停止)。
完整 PNG parity、设备组/持续发送、真实 Loom 双端/离线互信、双向编辑/LiveRelay/Tile、资源长稳、N01–N12 与最终全面手测仍开放。
v410 正式 startup 失败及 v398 间歇呈现未关闭；下方 v410 为历史。

## v410：投送目标编辑与外观归属

发送生产捕获已分离 resolved 底图与目标编辑：继续使用原 resolved 权威像素，不借目标 retained 替换上游；
标注、栅格覆盖层、当前 crop/reset 与 normal opacity 来自目标 canvas，而不是 resolved 的上游 editor。
纯外观 borderWidth/borderColor/cornerRadius/beautify 从已证明的 canonical Source 按目标 UnitId 在原后台 Job 中读取，
不在 UI 解析完整会话、不解码图片或重建活动工作区。未知大历史字段不进入外观 Value。
原 source-resolution 合成负责边框、全栈圆角与最后的 beautify 包装，实际最终宽高进入 PNG 准入和签名内容。
canonical 中旧 crop 不覆盖当前尚未保存的 crop/reset；历史 erase/flip/传播内容框不能在此无条件再物化。
目标 editor 没有 logical frame/本地 raster 时，逻辑尺寸回退到 canonical 目标展开框，不取上游自然像素；
缩略目标读取 savedRect 尺寸。无 raster 目标尚未保存的窗口缩放与真实图拓扑仍需后续运行验证。

完整 PNG parity 仍为未完成：历史编辑归一化、传播内容布局、自然图源跨 Unit 恢复与活动 blur 采样还需接通及验证。
设备组/持续发送/稳定 binding 显式停止、真实 Loom 双端/离线互信、双向编辑/LiveRelay/Tile、资源长稳、
完整 N01–N12 与最终用户全面手测继续开放，不以本切片关闭 v398 间歇呈现失败。
同源 26 组选定 lib 门通过，但正式 startup 集成为 5 过/2 超时；同字节计时诊断仍为 6 过/1 超时，原 15 秒期限未放宽。
本切片不具备全部选定门通过的结论，启动根因未识别/未修复；原失败与诊断证据均保留。
当前验证范围见 [v410 构建记录](WINDOWS_NATIVE_BUILD.md#v410投送目标编辑与外观归属)。下方 v409 为历史。

## v409：本地保留原图生产捕获

无 resolved frame 的单目标发送已在活动窗口捕获阶段选择权威底图：先对 live editor 调用
minified_source::Source::matches，再共享保留图与 export_snapshot；没有匹配原图时优先当前 layers.base，
不从尚未同步的宿主 image 复活已擦除内容。Content erase 换底图 owner 后原图失配，undo 恢复原 owner 后可重新使用。
后台 export_projection 只消费显式选定图源，不再将它覆写为低分辨率编辑底图；普通复制/文件导出不变。
显式 full_logical_frame 缺失时，逻辑尺寸回退到当前 editor.layers.size，而不是 retained 原图像素尺寸，
以保持默认 Editor 的裁剪密度与矢量坐标。自然像素尺寸仍只决定图源与输出分辨率，不覆盖逻辑标注框。

Scope 在原文档/显示/opacity/工作区围栏之外核所选像素尺寸与 Weak allocation 身份。
即使 editor revision 未变，相同字节的新 retained allocation 也使旧结果失效；Scope 不增加解码像素的强引用寿命。
UI 不解析完整会话、解码图片或执行 PNG 合成；原 Job/Lease/四槽、持久意图和唯一 Save/Checkpoint owner 保持不变。

本切片只接本地图源。resolved frame 仍沿 v408 的 image/editor 语义选择其当前底图，绝不借目标 retained 原图替换上游。
完整目标 metadata/上游传播分离、历史 erase/flip 归一化、传播内容框/边框圆角/beautify 生产接线仍开放，
不能把本轮 natural source 捕获称为完整 PNG parity。设备组/持续发送/稳定 binding 显式停止、真实双端、资源长稳及最终全面手测也未完成。
实际输入、产物与验证范围见 [v409 构建记录](WINDOWS_NATIVE_BUILD.md#v409本地保留原图生产捕获)。下方 v408 为历史。

## v408：有界源分辨率编码出口

发送 PNG 已复用原 source-resolution 合成/编码出口，不再维护另一份发送编码器。
当前生产入口仍显式选择已提交 layers.base，无 layers 才使用捕获的 resolved image；
不从旧宿主图或未经活动 editor 身份验证的 retained source 回退。输出取当前权威底图像素尺寸，
逻辑框只负责 contain/标注坐标；裁剪沿源密度计算尺寸。普通文件导出尺寸合同不变。

新增 export_source_png 只拥有最终尺寸准入/编码资源：发送模式在合成前分别检查 source、inner、
beautify outer 的 8192 边长/16M 像素，编码前预占 HGlobal 保守上界，实际 PNG 仍限 4 MiB。
美化包装的 WIC 像素副本与返回 PNG 沿原共享预算；取消在合成/编码/拷贝前后检查，
不能即时打断 WIC WriteSource/Commit。普通 workflow 不套用发送 4 MiB/16M 限额。
出口领取 ComposedImage 所有权，编码后先释放合成位图/输出租约，再申请返回 PNG 的预算；
发送的 HGlobal 编码租约仍保留到 stream 拷贝结束。独立预算回归覆盖两种生命周期，避免重构收紧普通 workflow 准入。

这是可复用编码层及当前底图接线，不是完整 PNG parity：生产 retained natural source、
目标/上游 metadata 分离、历史内容擦除/传播内容框/边框圆角/beautify 捕获仍未接通。
恢复 source 已物化 flip，历史 contentErase 未由 editor layers 重放，后续不能混为一类或盲目重复处理。
设备组/持续发送/稳定 binding 显式停止、真实双端、资源长稳、完整迁移与最终全面手测仍开放。

本轮回归另暴露 Live 回收/自动保存竞态：Unit 已从工作区图移除，但后台 worker 未退出，旧 owner 仍需保留待 reap。
保存的 automatic_ready/capture/matches 原先只核 Workspace 身份，误将退休 owner 纳入，报 `Live session is closing during save`。
现三处统一要求身份相同且 Unit 仍在图内；不以 is_alive 静默过滤仍在图内却异常关闭的 Live，不改全局 belongs_to 或 UI join。
新增不 poll/reap 的确定性回归同时保护退休成员排除、当前关闭成员失败、捕获/receipt 成员一致及编码后的 Unit 清单。
最终 input-4/run-3 同源 26 组选定 lib 门全通过（Live 20/0/0、export 94/0/0、workflow 77/0/0），四项集成与隐藏产品启动通过；
不是全量 lib、完整 34 门或最终用户验收，完整 PNG parity 与迁移目标仍未完成，原失败及中间输入/程序/日志保留。
结果及输入身份见 [v408 构建记录](WINDOWS_NATIVE_BUILD.md#v408有界源分辨率编码出口)。下方 v407 为历史。

## v407：单目标发送生产接线

已接通固定来源 Unit/当前 Directory 的原生发送菜单、取消及“重试原投送邀请”。
当前 resolved image、committed editor 与 normal opacity 在原 Sessions/Job/Lease/四槽内后台编码 PNG，
先经 canonical scope/发送容量准入，再持久登记原意图、read/明确 404 后 Create/同 ID reconcile，
后台生成 Binding candidate；Application 只发布内存 metadata，沿原 canonical checkpoint 精确包含后才显示发送成功。
普通网络失败保留单槽 Retry，显式重试复用原 Request/ID/签名/PNG，不重新编码或自动二次 Create。

Request tracker 覆盖持久发布后的 RecoveryRetained、panic、晚撤权与结果过滤；领取发生在原 Job 回收后、
closed/discarded 过滤前。源失效、撤权、切工作区或退出先交后台持久取消，再用当前原设备授权清理；
配置/准入/网络失败保原记录并有界退避。确认 unlink/retire 已发布但 UI 结果被过滤时，按同 Store 的真实退休事实结算，
不以外部删文件冒充成功。普通退出/换工作区不全量解绑稳定已保存关联。

Scope.source 保原 Record allocation 作 stale/rollback 围栏；binding_source 只取同 scope/proof 下已提交 canonical 原文，
以支持新 Unit 已进入检查点、尚未进入 Record.original 的情况。手动 Save As 或未知 index 的真实提交只防误解绑，
不冒充 canonical inclusion。最终完整 outbox 基线核验放在原 worker 返回前；UI join 只核当前租约并领取内存结果，
Ready 后续完整验证仍读基线，不把 metadata 当新的网络许可。
worker 结束后的外部磁盘变化不在 UI 再次读盘检测；metadata 保存不是后续请求许可，下一次完整后台验证仍须拒绝基线变化。

PNG 复用普通原生 export 合成，边长 8192、16M 像素、实际 PNG 4 MiB 上限，编码前预占保守共享预算。
这不是 source-resolution/minified retained source 或完整 beautify parity；设备组、持续发送、稳定 binding 显式停止、
真实 Loom 双端/离线互信、双向编辑/LiveRelay/Tile consumers、资源长稳、完整 N01–N12 和最终用户全面手测仍开放。
本轮仅隐藏 HWND、隔离合成 profile 与 loopback TLS；未重跑完整 34 门，不关闭 v398 间歇呈现及历史前台/启动根因。
目标 active、objectiveComplete=false、publishable=false；无新依赖、真实 profile、前台/全局输入、系统安装、默认 DLL 或 commit/push/publish。
最终输入/产物与选定门结果见 [v407 构建记录](WINDOWS_NATIVE_BUILD.md#v407单目标发送生产接线)。下方 v406 为历史。

本轮最终同源 24 组选定 lib 门全部通过（广义 sender 65/0/0，组间重叠不相加）；四项正式集成门与隔离隐藏产品启动通过。
严格全目标 Clippy/fmt/版本/组织合同通过，3911 文件 ratchet 零违规/警告/新例外；旧失败输入/产物/日志保留。

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

本轮只闭环原 Accept 网络 Job 在成功移交 Ready 之前的本次新建意图。
Store 在同一个 state 锁内返回 created/reused；复用记录可能属于之前工作区的稳定关联，
不凭当前 checkpoint、邀请状态或 projectionId 推断为未保存，也不自动减权。
原 Job 共享的 Attempt 保本次原 Store/完整 Ticket/活动许可到 join；HTTP/解码失败、
激活后的 synthetic worker panic、撤权后迟到成功均不在回收前释放许可。成功 Ready
独占原 Ticket/Lease/Slot，没有残留自动补偿副本；begin/activate 失败未发 HTTP，仍保
原 journal 的显式恢复责任，不扫其他 pending 意图。

Sessions 用单个 Option 交回新建且未移交的精确无许可快照；领取前禁止启动下一项
projection 任务，closed/discarded 不能丢交接，finished 不把该无等待句柄状态当线程。
Application 在 UI 来源过滤之前领取，复用 v394 后台 marker/原 Sessions 解绑网络单槽。
普通取消不 join UI；shutdown drain 后（含错误兜底）和最后 Unit 终态先回收网络，
refresh 原 profile，再 resolve/flush。只有持久 cleanup:true 后、原 origin/device 的
当前有效授权才能解绑，typed unlinked:true 才退休；不自动恢复授权或重新发布旧像素。

进程在 marker 落盘前崩溃仍只保证原写前意图保留，不声称取消标记跨进程原子提交。
复用意图、Ready 已移交后的 preparing/checkpoint/reporting 自动补偿、displayed
恢复重试、read/持续同步、发送端/策略/离线互信/双向 journal/Tile 仍未完成。
v394 严格前台门失败仍开放，本轮不重复其三个 projection_visible_ 用例，不豁免断言。
完整 N09/N01–N12、真实 Loom、资源长稳及最终用户整包均未验收。

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

## v394：已保存接收贴图删除、单向持久清理与原认证自动解绑

已保存接收贴图删除后，由真实 checkpoint 完整 Binding 和工作区 IdentityStamp 授予
减权资格；同工作区原文 allocation 变化不漏清理，新工作区不能借旧索引。先保删除
证据再回收 HWND；profile 仍 Opening 时不漏单，待原 Store 打开后匹配原 Ticket。
单个后台 mark worker 复用共享四槽和原 Wake，原 journal 完整 CAS/同 intent 许可
成功后才持久 cleanup:true；尚未落盘时阻止新 checkpoint/恢复/接收越过。最后 Unit
和终态先回收原 receive Hold，再完成 Opening/本地标记，失败不提交空 checkpoint。

网络仍归原 Sessions 单槽/Job/Control；只用当前所选原 origin、同原 device 的有效
Lease 减权，不自动获取授权或接收旧结果。仅 typed unlinked:true 退休；负回执/
403/丢回复保原 intent 和清理标记，1/2/4/8/16/30 秒封顶退避，无授权不持续返回
过期 deadline。普通退出和整体工作区切换不解绑全部稳定关联；终态清网络/退避
引用并回收 mark，不遗留 Store 租约。未持久删除前崩溃仍可能恢复旧 checkpoint，
本轮不声称原子跨文件提交或任何删除动作都已落盘。

旧 journal 缺 cleanup 字段默认 false，false 不序列化；true 是单向标记，不换原
intent/envelope/revision/digest/device/unit。旧 native reader 的 deny_unknown_fields
遇新 true 字段会 fail closed，不能称完全双向兼容，也不能删字段假装安全回滚。
在途 receive 取消/工作区切换自动补偿、displayed 回执恢复重试、read/持续同步、
发送端/策略/离线互信/双向 journal/Tile 仍未完成；完整 N09/N01–N12、真实 Loom、
资源长稳/历史前台启动压力根因和最终用户整包仍开放。

最终 final-run2 严格 fmt、全目标 Clippy -D warnings/check、版本和
3806 文件 ratchet 均通过，零违规/警告/新例外；27 份 native 变更/
7 新增，最大 456 有效行，UTF-8 无 BOM。同源八件 47 选定运行门
46 过/1 失败；cleanup 13/0/0、新 TLS 4/0/0、
投送桌面 48/1/0，严格前台断言仍失败，不豁免或以其他重叠组覆盖。
过滤组重叠不累计独立覆盖。
workspace_runtime 2/0/1，正式前台菜单退出仍 ignored；startup
7/0/0 不关闭旧偶发根因，原期限/断言不变。无真实 Loom/真实 profile 或
全局/CUA 输入，无新依赖/SDK 用途/权限/预算，完整迁移和最终用户整包未验收。

仅分配一次 v0.2.33.394。开发首轮 check 仅两处方法可见性失败，最小接缝修正后
另名验证；所有开发输入/日志保留，运行构建期间不编辑源码，不覆盖失败现场。
首个 final-run 严格静态/构建通过，同源 47 门中 46 过/1 abort：原 visible 测试的
严格前台 HWND 不变断言失败，随后 peer/Fixture Drop 次生 panic 导致 0xC0000409，
没有终态测试汇总，不能伪写 0/1。事后查询两个仍存 HWND 分属 Tabby/msedge，
不是事件时刻归属证明，不关闭前台变化根因；后续重叠组成功也不抹除首次失败。
仅修夹具 unwind 时不能再次 panic，并补保主 panic/现场/Store 释放的确定性回归；
原前台不变断言不放宽，只补事件前后 PID/class/有效性诊断。旧八件、完整冻结输入/
原日志和 failed-candidate.json 均保留，final-run2 使用新源码，不覆盖原候选。
第二候选不再 abort，原严格前台门给出正常 48/1/0 终态；失败发生于 displayed 正向
用例。事件时刻 before/after HWND 不同，但 PID 均为 30368/Chrome_WidgetWin_1，
与测试 PID 51988 不同；该 PID 的事后只读查询是 Edge。证明观察到外部进程窗口切换，
不证明所有间接触发原因，也不宣称完整焦点合同通过。无第三次重跑、不操纵该外部
窗口，原源码/程序/失败 profile/日志保留，焦点门仍开放。
Opening 竞态/原 intent/活动许可/外部写者/旧字段兼容、真实 TLS 先标记后解绑、
false/403 真实 1 秒退避、错 origin/撤权不发网和真实 Renew 后恢复均有运行门。
七文档/最终 2654 冻结输入/源码三次一致、352 旧封存程序和七诊断由
closeout 核验；不提交、推送、发布或替换用户程序。本轮仍未注入生产接收准备 worker panic/
真实授权到期全链，不冒充额外故障覆盖。

source SHA-256：`ddbd55ae2ddbde20c0902a9dad5497e7be51043560b6fd589baf0e7a8e90db09`。
debug EXE：43,603,968 字节，SHA-256：
`3b308bb38ea870fd1f4235a119d925c433acc685fa237deb08322ef1839a4c84`，封存在
otter-native-v394-projection-delete-cleanup/final-run2/artifact，publishable=false。
状态 projection-saved-delete-cleanup-focused-verified-foreground-gate-pending，总目标仍 active。

## v393：显式接受、保存后的可见呈现与原 Grant displayed

v393 开发接入显式确认接受→原 Scope/Grant→隐藏追加→精确 checkpoint→不激活的
可见原图新帧→认证 displayed。呈现复用原 Document/像素/窗口 owner，回执复用原 Job
并移动 Hold，不重取 Slot/intent 许可；取消保留至回收，撤销后取消过期 deadline。
当前工作区首窗口所在显示器工作区居中，等比缩小、不放大；显示要核同 HWND/State/
文档/像素、正尺寸/opacity、非 cloaked/最小化/隐藏及屏幕工作区相交，只有 EndDraw 和
S_OK 新帧提交才确认。此证明不等物理扫描输出或用户肉眼确认。

displayed 成功不是 unlink，不退休 journal；失败/取消/保存冲突保本地图片和意图，
沿现有显式认证解绑恢复。尚无自动生命周期补偿/回执恢复重试、持续同步、发送端、
自动/拒绝策略、离线互信、双向 journal/Tile。正常退出不全解绑已保存关联；不改变
完整 N09/N01–N12、旧前台/启动/压力根因、真实 Loom/资源长稳/最终用户包的开放状态。

仅分配一次 v0.2.33.393。最终 final-run 严格 fmt、全目标 Clippy -D warnings/check、
版本和 3799 文件 ratchet 均通过，零违规/警告/新例外；
24 份 native 变更/7 新增、最大 381 有效行，UTF-8 无 BOM。
同源八件 45 选定运行门全过；visible 3/0、present 5/0、placement 2/0、确认 1/0，
原接收 owner 10/0。过滤组重叠，不累计独立覆盖。workspace_runtime 2/0/1 ignored，
正式前台菜单退出仍未验；startup 7/0 不关闭历史偶发根因，原期限/断言不改。
本轮没有真实授权到期等待或 worker panic 的 Application 全链故障注入；取消后不再
返回过期 deadline 的运行断言不等真实时钟到期全链证明。没有新增依赖/SDK 用途/
像素或网络预算，不注入全局/CUA 输入；可见测试只用合成 profile/身份、回环 TLS/
自有窗口，并核前台窗口不变，不代真实共享服务或实际用户点击模态框验收。

开发三次编译可见性失败均保留原日志/门前输入；只扩大必要接缝可见性或在共同发布
后从 Application 原 Document 取凭证，未复制窗口 owner。首个 all-targets check
未结束前曾继续编辑，门前快照不冒称每个编译器消费的 exact 输入，单独注明。
其余开发与最终冻结输入分开，最终 2647 输入/七文档、源码三次匹配、
344 旧封存程序/七诊断保护由 closeout 核验；不提交/推送/发布/替换用户程序。

最终 source SHA-256：`8475b11a08ad73e8f49f2d6f9c0ee0b054f93267832b523f4b629bd5d1d12eef`。
debug EXE：43,542,016 字节，SHA-256：
`a471e8f4b24b8e0492e181ac49b34163419e8c394cfc9c3015d7c3266a0c7f75`，封存在
otter-native-v393-projection-visible-receive/final-run/artifact，publishable=false。
状态 projection-manual-visible-receive-displayed-verified-lifecycle-sync-pending，
总目标仍 active，不代完整接收体系、迁移或最终用户包。

## v392：Application 唯一接收准备、隐藏追加与 checkpoint 生命周期

本轮仅分配一次 v0.2.33.392。接收目标必须在发网前捕获原 Source allocation、坐标
空间和恢复 Guard；响应到达时不得补造新工作区。Ready 移交为 Hold，持续拥有原
Grant 的 Lease、全局四槽之一及同 intent 活动许可，跨后台候选准备、隐藏追加和
精确 Binding 的 checkpoint 包含保留。Application 另核同 profile 的同 Store/Record，
不会用相同字节、UUID 或同 origin/device 的新 Grant 接管旧结果。

唯一状态按 Waiting→Preparing→Checkpoint 推进，后台只复用原 PNG/会话/原文追加
准备，不复制图片/工作区/保存 owner。网络任务回收后才启动单个有界准备线程；
撤权回调和完成共用原 Wake，到期合并既有绝对 deadline，不增等待句柄、轮询器或
第二任务池。取消只撤权/标记，UI 不等待准备；Job 到一次领取或终态 shutdown 回收
前保 Hold，错误/panic 也不得提前归还容量或许可。Preparing 纳入恢复/快照/退出
围栏；poll 先取走自身状态再复核，避免追加与保存自阻塞。成功隐藏追加后只比较
真实 checkpoint 的完整 expected Binding，成功仍保 Hold 等待未来呈现消费者。

切换服务、取消/新授权、成功工作区交换、最后 Unit drain 和 shutdown 都停止旧接收。
失败/撤权/保存冲突保已追加的本地 Unit、图片与 journal，不自动解绑/退休意图；
状态“投送已保存，等待呈现”不等于可见 present 或 displayed。本轮仍不开放接受
菜单、不显示窗口、不发 displayed、不自动补偿或接入持续同步/发送端/离线互信/
双向 journal/Tile，完整 N09/N01–N12、旧前台/启动/压力根因、真实 Loom/资源长稳/
最终用户包仍开放。暂用目标首窗口真实 RECT 转候选位置，正式接受放置策略待后续。

开发首轮 check 因共用 same_space 可见性失败；只扩至 Application 范围，不复制算法。
首轮 owner 测试 8/1：最后 Unit 夹具跳过 run_loop 已消费 WM_QUIT 的前提，只补真实
主循环过渡，不改产品期限/断言。另修正常移交后的引用 drop warning。静态交叉核验
发现错误 worker 提前释放 Hold，已改为 Job 独占、worker 只持无许可记录快照，并补
线程已结束而 UI 未领取时的确定性容量/intent 负向测试。原失败输入/程序另保。
修正后开发 owner 10/0。首个 final-run 严格静态/构建/源码匹配通过，尚未运行验收；
随后独立 Rust 诊断实证 closure 捕获可晚于完成通知析构。将准备输入显式移到通知
之后声明的局部 owner，确保正常/错误/panic 都先清理输入再通知；不是已证泄漏或
丢通知，也不新增等待句柄。原静态候选/诊断保留，最终另建 final-run2，不沿用旧
字节结论。

最终 final-run2 严格 fmt、全目标 Clippy -D warnings/check、版本及 3,792 文件
ratchet 均通过，零违规/警告/新例外；25 变更/6 新增、最大 454 有效行，UTF-8 无 BOM。同源八件
41 个选定后台/隔离运行门全过：新 owner 10/0、watch 2/0、投送桌面
35/0、客户端 55/0，原接收 TLS、隐藏追加/checkpoint/重启像素、
库/工作流/保存/恢复/共享容量/退出 drain/资源和三类启动同过。过滤组重叠，不累计
独立覆盖。workspace_runtime 2/0/1 ignored，正式前台菜单退出仍未验；startup
7/0 不关旧偶发根因，原 15 秒期限/断言不变。本轮没有真实授权到期等待或准备
worker panic 的 Application 全链故障注入；静态回收保证不冒充这些额外运行证明。

四份开发 exact 输入、最终 2,640 冻结源码输入/七文档及源码三次匹配均核。
336 旧封存程序/五诊断不变，首个静态候选、首轮失败程序与独立析构诊断另保；后续
保护 344 件/七诊断。最终 source SHA-256 为
`484481f67dbde39b34d1c36729bcdabd48fcb0b977f0be2d35466a6d64e0a601`。
debug EXE 为 43,501,056 字节，SHA-256 为
`b3c1b7d1f58ef1d20500e2d46071cdfbe094730ce4f1a6e082972d0a0a066ec4`，封存在
otter-native-v392-projection-receive-owner/final-run2/artifact，publishable=false。
状态为 projection-receive-owner-hidden-checkpoint-verified-visible-consumer-pending。
不提交/推送/发布/替换用户程序，总目标仍 active，不代完整 N09/迁移或最终用户包。

## v391：接收意图与 Unit 持久关联、真实 checkpoint 包含凭证

本轮仅分配一次 v0.2.33.391。原接收 worker 在完整验图后把 Verified 绑定原意图
UUID，不能将旧结果重新贴到同投送的新意图。候选保留 qrProjection 完整快照及
otterNativeReceive 的 intentId/deviceId；绑定核 exact envelope、确定 Unit、设备、
原意图和 revision 下界，后续精确快照证明还必须比较完整 Binding（含 revision/digest）。

原保存/恢复 worker 从真实不可变字节解析有界关联索引，借用大型图片原文；元数据
限 8 KiB、沿原 128 Unit/意图容量。旧关联缺原生标记不推断，新标记坏格式/重复
或未知字段只使索引不可用，不覆盖原数据或阻止普通会话保存。索引持共享 Source，
提交前核同一 allocation；只有原 Receipt/staged/candidate 复核后 disk.commit 真正
成功才进入 checkpoint owner。已发布但旧恢复副本未退休时先保留提交事实再报告错误。
断电保证仍取决于实际文件系统，不扩张原事务保证。

规范默认 checkpoint 的启动恢复只领取原 worker 的同源索引；唯一恢复副本不是规范
提交凭证，普通导入也不会直接提升。菜单只按当前原文/Unit 作用域将明确已提交项
短标为“解除已保存投送绑定”，其余仍是未结；相同字节的新 Source、新 intent、删除
或切换不能借旧证明。包含关系是本地提交历史事实，不是持续读盘/外部删除探测，
更不是当前授权、可见呈现或 displayed。没有新增网络请求、轮询、线程、容量或依赖，
不自动 accept/unlink/退休意图或删除图片。

开发阶段关联组 4/0、真实 checkpoint 组 5/0；交叉核验后补 sticky intent 与菜单/
容量负向测试。首个 final-run 严格静态及构建均过，随后核真实旧版
sessionStickerPayload 的 qrProjection:null，修正普通贴纸 null 不应屏蔽同会话合法
原生关联；加混合会话回归，原冻结输入/日志保留且未冒充运行验收，最终另走 final-run2。
最终 final-run2 严格 fmt、全目标 Clippy -D warnings/check、版本和 3,786 文件 ratchet
均通过，零违规/警告/新例外。28 份 native 变更/4 新增、最大 349 有效行；新关联/
索引各 65、模型测试 158、Application checkpoint 测试 210 行，UTF-8 无 BOM。
四份 exact 开发输入及最终静态门前的 2,634 冻结源码输入/七文档保留，不继承开发 green。

同源八件的 39 个选定后台/隔离运行门全过：关联模型 5/0、真实 checkpoint 5/0、
profile 4/0、恢复菜单 4/0、Application 认证解绑 8/0、journal 15/0、原补偿 9/0；
投送桌面 25/0、客户端 55/0，原 TLS 接收 5/0、隐藏追加/checkpoint/重启像素 1/0、
checkpoint 全组 18/0、库/工作流/保存/恢复/容量/退出 drain/资源及三类启动均过。
过滤组重叠，不累计独立覆盖。真实隐藏 HWND/菜单不代真实前台弹窗手测；
workspace_runtime 2/0/1 ignored，原移动提交→中断→重启→删除最后 Unit 链通过，
正式前台菜单退出仍未验。startup 7/0 不关闭 v387 偶发根因，原 15 秒期限/断言不改。
本轮没有新增 Application 级 retained-recovery 故障注入，不把顺序审查当该全链实测。

最终 source SHA-256 为
`a33cab341f3a386db74f8303ceeb30fdff5ac294c78d0f717c2b2cb31cfb4557`。
debug 产品 EXE 为 43,378,176 字节，SHA-256 为
`6cd1fb75107bfc0357718d4e4d7f784c15f3acaa5de61943d9f5e5eae77c9c1d`，
封存在 otter-native-v391-projection-checkpoint-binding/final-run2/artifact，publishable=false。
三次源码/八件/328 旧封存程序及五诊断、v388 Clippy/v387 启动失败完整输入由
closeout 复核；后续保护 336 件/五诊断。本轮状态为
projection-checkpoint-inclusion-verified-receive-consumer-pending。首个 final-run 的严格
静态/构建及原冻结输入另保，未运行验收，不冒称其字节与最终候选一致。

接受菜单/接收唯一状态及原 Grant 跨异步准备/提交的生命周期、真实可见
present/displayed、自动生命周期补偿、持续同步/发送端/离线互信/双向 journal/Tile
仍缺，完整 N09/N01–N12、旧前台/启动/压力根因、真实 Loom/资源长稳/最终用户包
仍开放；不提交/推送/发布/替换用户程序，goal 仍 active。

## v390：默认 profile 后台 journal 与显式未结投送解绑消费者

本轮只分配一次 v0.2.33.390。普通默认 profile 完成工作区恢复屏障后，Application
在后台打开一次接收 Store，完成通知复用原 Wake，不增加等待句柄、轮询器或网络槽。
长期 owner 保持独占租约与未结意图；服务/工作区切换不重开 Store，不自动发 accept
或 unlink。检查模式、无默认 profile 的显式启动不发现 journal；坏格式/锁冲突只
停用投送恢复，不覆盖坏数据或停用已正常恢复的工作区检查点，也不隐式循环重试。

现有共享授权菜单增添“解除未结投送绑定”：终态风险确认默认选否，确认前后检查
来源、同一 Arc、busy/退出/模态与发起窗口，不能持 View 借用跨对话框。Application
再核 profile owner 的确切候选，沿原当前授权 Lease/Sessions 投送单槽发认证 unlink；
worker 的原设备/来源/磁盘基线/移动许可与 unlinked:true 退休条件均不放松。本地
Unit、图片和工作区不删除。完成后只在原任务回收边界读取 Store 内存提交状态，
即使成功退休后发生撤权/切换而丢弃结果，也不复活已退休菜单项；失败保同一候选。

这是未结副作用的显式减权恢复链，不是 Unit 持久化关联/自动恢复接收。接受菜单、
checkpoint 目标包含凭证、可见 present/displayed、自动退出/删除补偿、持续同步、
发送端/离线互信/双向 journal/Tile 仍缺；完整 N09/N01–N12、旧前台/启动/压力根因、
真实 Loom、资源长稳和最终用户包仍未完成。本轮只用 linshi/合成身份/回环 TLS/
隐藏 HWND，不提交/推送/发布或替换用户程序，总目标尚未完成。

最终 final-run 严格 fmt、全目标 Clippy -D warnings/check、版本和 3,782 文件
ratchet 均通过，零违规/警告/新例外。19 份 native 变更/5 新增、最大 453 有效行；
新 profile owner 139、恢复 UI 113、新测试最大 316 行，UTF-8 无 BOM。两份 exact
开发输入及最终静态门前的 2,630 冻结源码输入/七文档完整保留，不继承开发 green。

同源八件 37 个选定后台/隔离运行门全过：profile 4/0、恢复菜单 3/0、Application
认证解绑链 8/0、journal 15/0、原认证补偿 9/0、投送桌面 24/0/客户端 50/0；原
TLS 接收 5/0、隐藏追加/checkpoint/重启像素 1/0、checkpoint 13/0、库/工作流/
保存/恢复/共享容量/退出 drain/资源和三类启动同过。过滤组重叠，不累计独立覆盖。
菜单测试使用真实隐藏 HWND/菜单与可控确认回调，不冒充真实前台弹窗手测。
workspace_runtime 2/0/1 ignored，正式前台菜单退出仍未验；原 15 秒期限/断言不变。
startup 7/0 不关闭 v387 偶发根因、旧前台/完整组/压力或完整迁移/用户包门。

开发首次过滤未命中，仅证明编译成功，不计覆盖；首次真实恢复组 10/1，因独立
State 夹具未注册生产 Application 已注册的模态标记而失败。只补夹具注册，保原
断言，并将本地内容保护加强为真实图片 Unit/像素 storage 身份；随后开发 11/0。
原日志、失败测试程序和两份 exact 输入均保留，不用后续绿色覆盖失败证据。

最终 source SHA-256 为
`66bf526a30bc7e69c4651e9a96cde5d9cfca9dad0fd17d5c26932657308bcc4c`。
debug 产品 EXE 为 43,269,632 字节，SHA-256 为
`d8bda8e6b8f4ce6036a0d7c1e1f49c2f7f262b641b5b9a55095a67a8885f20ef`，
封存在 otter-native-v390-projection-recovery-profile/final-run/artifact，publishable=false。
三次源码/八件/320 旧封存程序及四诊断、当前失败程序、v388 Clippy/v387 启动失败
完整输入由 closeout 复核；后续保护 328 件/五诊断。状态为
projection-profile-recovery-verified-receive-consumer-pending，不代完整桌面接收或迁移验收。

## v389：传统接收写前意图与认证解绑补偿内核

本轮只分配一次 v0.2.33.389。接受请求先复核当前 Lease/固定 origin，再经显式后台
打开并共享的 Store 持久写入原 envelope、设备、确定 Unit、原 expectedRevision/digest
与意图 ID；未成功写入不得发送 accept。队列最多 128 项/512 KiB，不存 token/PNG，
不读取、迁移或删除旧 WebView/localStorage 数据。幂等重试保原请求期望值，仍核完整
磁盘基线；坏格式、外部修改、满队列或不可写都不能降级为空或覆盖竞争写者。

profile_directory 从原 workspace_profile 提取逐层非 reparse 祖先 pin。journal 复用
原 session 暂存/文件句柄 CAS/无覆盖发布与恢复 owner；唯一合法恢复副本可重建规范
路径但不删除旧副本，多副本歧义拒绝。新队列已发布而旧恢复文件退休失败时，内存
保留已提交基线并明确返回 RecoveryRetained，不伪报全部回滚，也不继续发 accept。

同 intent 的内存许可在发网前核当前 Record，跨 HTTP/Ready/移交 Ticket 保持独占；
不持状态 Mutex 跨 HTTP，不同 projection 仍共享原四槽并行。丢弃/失败/撤权只归还
内存许可，不删除落盘意图。Compensate 只允许当前明确授权的原设备/原 origin 调用
POST /v1/projections/unlink，只有认证成功且 typed unlinked:true 后才持久退休；
403、false、坏回复、取消或失败一律不当成功。服务器仍裁决 epoch；不通过重新
accept“修复”解绑失败，不借新 Grant 发布旧 Ready。Loom 的 unlinked 为终态，不能
推断同 projection 可以重新绑定；本地许可还防旧 Ticket 发网或抢先回收 Ready。

这只是接收安全内核：尚无接受菜单、工作区/Unit 与意图的完整恢复对账、真实可见
present/displayed、自动退出补偿调度或持续同步消费者。原隐藏窗口追加测试仍只证
落盘/恢复像素。完整 N09/N01–N12、真实 Loom、旧前台/偶发启动/压力问题、资源长稳
和最终用户包仍开放；不提交/推送/发布或替换用户程序，总目标尚未完成。

最终 final-run 严格 fmt、全目标 Clippy -D warnings/check、版本和 3,777 文件
ratchet 均通过，零违规/警告/新例外。16 份 native 变更、6 新增，最大 440 有效行；
journal/record/activity 分别 262/100/52 行，workspace_profile 121→72 行并共用
profile_directory.pin，UTF-8 无 BOM。六份 exact 开发输入及最终静态门前的 2,625
冻结输入/七文档均保留；最终结论不继承 v388 或开发阶段 green。

同源八件的 34 个选定后台/隔离运行门全部通过：journal 14/0（含三项许可测试和
环境 probe）、认证补偿 9/0、投送客户端 49/0、桌面收件箱 13/0、TLS 接收 5/0、
隐藏 HWND→checkpoint→重启恢复 1/0、冻结库 32/0、frozen 14/0、Candidate/工作流
55/0；其他授权/共享容量/退出 drain/文件/保存/恢复/资源及三类启动门同过。过滤
组重叠，不能累计为独立业务覆盖。workspace_runtime 2/0/1 ignored，正式前台菜单
退出未验，原 15 秒期限/断言不改。startup 7/0 不关闭 v387 偶发超时根因，也不
关闭旧前台/完整组/压力失败。测试只用 linshi、合成身份、回环 TLS 与自有隐藏
HWND，不安装系统证书、SDK 或更改注册表/真实 profile，不占用游戏前台。

最终 source SHA-256 为
`e3a95758ce5889b3146be49eb46a8da339b2b5aa191afd168368b21098667422`。
debug 产品 EXE 为 43,197,440 字节，SHA-256 为
`9288b8ed4ba19f82593bde268c1244f4d8fa2a713e5ce4af062c87f929d79463`，
封存在 otter-native-v389-projection-receive-journal/final-run/artifact，publishable=false。
三次源码、八件身份、312 旧封存程序/四诊断、v388 首轮 Clippy 失败输入和 v387
启动失败完整输入由 closeout 复核，后续保护 320 件/四诊断。状态为
projection-receive-journal-verified-consumer-pending，不代完整接收或迁移验收。

## v388：传统接收 PNG 与原生工作区追加准备内核

本轮只分配一次 v0.2.33.388。accept 沿原认证 Control，独立使用 6 MiB wire/
4 MiB PNG、8192 边长/16,777,216 像素合同；核 exact envelope、本机接收身份、
确定 delivery-ID、revision 下界及原压缩字节 SHA-256，再经 WIC 完整解码和尺寸校验。
幂等恢复允许服务器返回更新快照，不把过期 accepted 邀请提前排除。Ready 保留原
Lease 与四槽之一，Job Drop 不撤销已移交结果，同 Grant 后续 owner 才可领取。

接收单贴纸保留 qrProjection，已验证像素直接移交原 session candidate/矢量准备；
Candidate 原文改用现有 Source owner，保留编码预算，工作流及归档继续移动同一原文。
session_append/共同 application_append 从原冻结库事务提取，只追加单窗口，未知
字段、原窗口和磁盘基线保留；库快照匹配/回收站消费仍由原库模块拥有。

当前只实现接收准备内核，TLS→隐藏 HWND→检查点→重启恢复的同源测试已通过。
菜单接受尚未开放，远端 accept 副作用的持久补偿/退出解绑、真实可见呈现与 displayed
消费者仍未接入；不得将这些低层 API、隐藏窗口或本轮定向测试称为完整接收可用。
v387 的启动 15 秒超时继续开放，不用本轮成功覆盖；完整 N09/N01–N12 和最终用户包
仍未完成，不提交/推送/发布或替换用户程序。

最终 final-run2 严格 fmt、全目标 Clippy -D warnings/check、版本和 3,771 文件
ratchet 均通过，零违规/警告/新例外。31 份 native 变更、12 新增，最大 367 有效行，
新增模块最大 257 行，UTF-8 无 BOM；库原文/窗口追加提取后分别保留 83/44 行薄接线。
同源八件的 32 个选定后台/隔离运行门全部通过：接收 checkpoint 1/0、accept 5/0、
真实 TLS 接收 5/0、共同原文追加 3/0、冻结库 32/0、frozen 14/0、Candidate/工作流
55/0，包含归档源预算保留。原投送客户端 26/0、桌面收件箱 13/0、checkpoint 9/0、
保存/恢复/授权/共享容量/退出 drain/资源及三类启动门通过；过滤组重叠，不累计独立覆盖。
原 workspace_runtime 2/0/1 ignored，继续覆盖移动提交→强制中断→重启恢复→删除最后
Unit；原 15 秒/断言不改。正式前台菜单退出仍 ignored 未验。本轮 startup 7/0，
不关闭 v387 偶发超时根因、旧前台/完整组/压力失败、完整 N09/N01–N12 或最终用户包。

开发阶段三次 Source 传播遗漏的编译失败及一次错误比较显示图/原图的测试失败均保留。
后者只修测试合同：分别逐字节验证落盘 2×1、恢复原图 2×1 和编辑器 160×80 投影，
未改生产绘制。首个编译失败输入为 formatted-equivalent、不是 exact；后续失败为
exact。首轮 final-run 的 large_enum_variant 失败和 2,619 冻结输入/七文档完整保留，
只对两处大枚举载荷 Box 化，无 warning 豁免；最终结果来自不同摘要的 final-run2。
本轮 304 旧封存程序/四诊断和全部开发输入另由 closeout 核验，后续保护 312 件。

最终冻结 2,619 输入，source SHA-256 为
`7c42f34f3647106b5898609819c77a495751a4842890bc585ba9272169b7d5ae`。
debug 产品 EXE 为 42,998,784 字节，SHA-256 为
`bcabcba772c1d6c0d45dc2bf8913139e74439342af87b429b5897dac154324ec`，
封存在本轮 final-run2/artifact，publishable=false。状态为
projection-receive-core-verified-consumer-pending，不广告完整接收可用。

## v387：传统投送收件箱的桌面菜单、查询与拒绝消费者

本轮只分配一次v0.2.33.387，开始前核v386同源闭环/七文档、296份旧封存程序及四诊断，
并保留v385完整失败输入和原运行现场。沿N09传统投送合同接入现有共享授权菜单，
显式“刷新投送收件箱”只发一次policy=confirm的元数据查询；待确认项提供拒绝操作，
accepted项仅禁用展示。来源名有界截断并转义菜单助记符，不持token或另建身份存储。

device_projection_inbox_ui只拥有有界菜单数据/请求，composition_projection_inbox
复用原Sessions四容量/全局四Slot/完成句柄/退出drain，Application消费者拥有准入
和结果投影。取消只标记停止和丢弃，未完成Job不在UI take/join，完成后才回收槽；
origin切换/撤权/更新授权/成功workspace交换/退出均先取消并清候选。拒绝必须是
当前列表同一个Arc对象且未accepted/同origin，防相同ID新候选ABA；只有recorded:true
才删除候选并显示成功。失败保exact候选供显式重试，offlinePeers不冒充跨Loom完整结果。
模态/退出/恢复启动阻止新请求，重复请求不清旧busy/候选，不增加轮询或资源预算。

开发首轮8/1因合成空会话缺坐标元数据而失败，完整2,606构建输入及七文档已另存；
只补合法坐标夹具，不放宽原恢复准入。随后12/0覆盖真实菜单→授权→回环HTTPS→
typed结果及切换/撤权/同origin恢复/HTTP取消；新增共享容量回归另纳最终同源门。
最终严格静态、源码摘要、八件封存和选定相邻运行门另行记实，不继承中间green。

没有accept、图片下载/持久化呈现/displayed、持续同步/恢复、离线互信、双向journal
或Tile消费者；不广告默认接收可用。不重跑前台，不关闭v384菜单前台、v379 return、
旧完整组/压力未知失败、完整N06/N09/N01–N12、真实Loom/资源长稳/最终用户包门。
不提交/推送/发布/替换用户程序，goal仍active。

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

最终冻结2,607输入，source SHA-256为
`dbafb3134515abf7c5e1fae858b8ed581c68016c1a6a39ee1eca5e67f80f2dbb`。debug产品EXE为42,917,888字节，SHA-256为
`ff6d6fccc510f76cbebd3892618b3adc7038176eafabe92c85c42e5f80981c1c`，位于本轮final-run/artifact，publishable=false。
开发首轮缺坐标夹具8/1及完整输入保留，合法夹具后12/0不替代最终13/0；最终startup
6/1及完整冻结输入/七文档/八程序保留，单项诊断1/0不替代该失败。没有修改
恢复准入、扩容量/SDK用途或重跑前台。不提交/推送/发布/替换用户程序，goal仍active。

## v386：纯窗口移动的有界队列结算唤醒

本轮仅分配一次v0.2.33.386，先核v385失败完整输入/七文档、288份封存程序及四诊断。
隐藏自有HWND的确定性RED证明：跨线程SetWindowPos已经改变真实RECT，但没有queued
message、保存任务和落盘变化；测试不把worker句柄纳入产品等待、不补WM_NULL或主动
tick。默认启动在显示前已建立检查点stamp，不能归因于启动stamp竞态或仅有的热键日志。

新增window_geometry_wake仅拥有每文档窗口active/pending两个Cell，位置或尺寸变化
合并最多一条WM_APP+117；ack后可再次通知，失败不锁住pending，销毁后不投递。纯
Z-order/show/hide/frame且NOMOVE|NOSIZE不通知。WM_WINDOWPOSCHANGED仍交DefWindowProc
派发原WM_MOVE/WM_SIZE，不直接在WndProc保存、不增加线程/定时器/空闲轮询或资源预算。
State/构造/窗口过程只薄接线；原Application/单槽/Receipt/文件CAS/独立checkpoint
revision仍唯一负责结算与持久化。新增隐藏HWND和单元回归复用原Application夹具。

最终严格静态、同源封存和原workspace_runtime的15秒移动/中断重启/删除链另行记实，
不更改该原门或把隐藏测试替代产品链。不重跑菜单前台门；v384前台、v379 return、旧
完整组/压力未知失败、完整N06/N09/N01–N12、真实Loom、资源长稳和最终用户包仍开放。
本轮没有N09新桌面消费者，不提交/推送/发布/替换用户程序；goal仍active。

v386最终final-run严格fmt、全目标Clippy -D warnings/check、版本及官方ratchet
均过，零违规/警告/新例外；7份native变更/3新增，最大479有效行，UTF-8无BOM。
同源八件21个选定后台/隔离门全过：wake3/0、checkpoint8/0（含两项新隐藏HWND），
原workspace_runtime2/0/1 ignored完成移动提交→强制中断→重启恢复→删除最后Unit
落空；保原15秒和断言。正式菜单退出仍ignored未验，不关闭v384前台或旧完整组/
压力失败。三次源码/288旧件/四诊断/v385完整失败输入及原现场由closeout复核，
后续保护296件；状态window-geometry-wake-and-workspace-runtime-verified，不代完整迁移。

开发RED0/1明确无queued结算，窄修复后该原断言通过；首轮扩展HWND测试1/1因
GetUpdateRect不提供隐藏窗口尺寸派发凭证失败，旧输入/日志另保。改用RAII subclass
直接观察100次真实WM_MOVE及一次WM_SIZE，保全部合并/ack/负向断言，定向2/0。
较宽geometry开发过滤147过/6 ignored不转移为最终验收，也不与最终门重复累计。

最终冻结2,600输入，source SHA-256为
`55d57c620e6bcc9b0877abc1af3aebe7963002913289199c97b79f8a758048e3`。debug产品EXE为42,707,968字节，SHA-256为
`eb750c2a6dc3006c0e32da278f08472102ada7fa77ffade23ee9668aa2aee375`，位于本轮final-run/artifact，publishable=false。
相邻保存/恢复/Live/消息等待/文件/资源/三类启动门均过，逐门数字见closeout；不
重跑菜单前台，不声称N09桌面/真实Loom/长稳/最终用户包完成，goal仍active。

## v385：传统投送收件箱／新邀请拒绝的认证后台客户端

本轮沿用一次分配的v0.2.33.385，开始前核v384当前源码/七文档及280份封存程序、
四份诊断。继续N09传统PNG投送，不以已有LiveRelay替代。新增projection_delivery
独占邀请/收件箱与recorded回执，envelope子模块仅验形状/固定HTTPS来源，签名真实
性仍由认证Loom验证；Debug不输出nonce/签名或完整wire。全量解析后才发布，最多
64项/1MiB；坏字段/重复ID/跨origin/版本/摘要/状态/route整批拒绝，不静默截断。

原Control的child复用唯一send_limited：POST /v1/projections/inbox提交显式policy，
POST /v1/projections/receipt只提交projectionId及rejected，12秒有界HTTPS、Device/
独立nonce、固定origin、请求前后token期限及取消不变。只有typed recorded:true才
算回执成功。拒绝不增加接收权，过期pending邀请仍交服务器裁决，不加旧版没有的
本地邀请期限门；accepted项不作为新邀请拒绝，不能借策略改变停止已有投送关联。

Job复用原4任务全局Slot与授权Lease，CA文件/DNS/TLS均在后台；完成未领取仍占槽，
领取时复核当前撤权/到期，Drop取消并join而不detach。调用者仍须另核workspace/
selection，正常owner应等完成句柄后领取，不能在UI线程阻塞回收。没有第二凭据/
网络/解析/身份存储owner，不重新配对，不扩SDK许可、依赖或资源预算。

旧版通用bridge为6MiB，本轮收件元数据限1MiB、64项；sourceName非空/≤1024 bytes/
无控制字符、route限absent/shared_loom、重复ID整批拒绝，属于明确的fail-closed收紧，
不是声称逐字无差异迁移。offlinePeers保留为尚需查询的离线来源标记，不冒充完整
跨Loom收件箱。当前仅单次查询/拒绝客户端，未接桌面入口、自动接收、accept、图片
下载、持久化/真实呈现后displayed、持续同步/恢复、离线互信、双向journal或Tile。

只读核现有Loom：record创建取source.revision、update只允许prior+1、磁盘加载也核
当前revision≥source锚点，因此原生下界不是两套版本混用。服务端rejected receipt
确实要求未过期/仍待确认/未unlink；本地保留旧客户端的提交语义，不保证过期邀请
会被服务端接受。inbox本身刷新presence，无需另发前置presence请求；未做在线验收。

开发首轮因测试Result::is_err_or不存在而编译失败（非业务RED），旧日志保留；改合法
表达式。字段反例另分配合法ID并精确断言Entry，避免Duplicate先失败的假阳性；配置
路径测试复用原回环CA/唯一linshi文件夹，构造时不读CA、后台真实请求且身份文件字节
不变。最终定向门/严格静态/同源封存另行记实，不继承开发green。

首轮final-run严格Clippy因Operation内嵌368字节邀请触发large_enum_variant，完整
2,597冻结输入/七文档与失败日志另保。改为Box持有邀请，不新增allow/豁免；同一
v385的final-run2重新冻结/验证，不覆盖首轮，也不把首轮开发green当最终批准。

本轮不重跑前台、不关闭v384菜单前台失败、v379 return、旧完整组/压力未知根因、
完整N06/N09/N01–N12、真实Loom联调、资源长稳或最终用户包。goal仍active，不提交/
推送/发布或替换用户程序。

v385最终final-run2严格fmt、全目标Clippy -D warnings/check、版本及3,749文件ratchet
均过，零违规/警告/新例外。10份native变更/7新增，最大362有效行；四生产模块
168/111/29/128，三tests186/151/196，UTF-8无BOM。同源八件的13个选定后台/隔离
门12通过/1失败，projection16/0、device HTTP144/0（包含新9项，不能累计独立覆盖）。
workspace_runtime1/1/1 ignored在checkpoint-moved-window原15秒超时，尚未进入中断/
重启/删除链；失败profile仍revision7原坐标，stderr仅热键冲突，不足以确定根因。
菜单前台门没有重跑，ignored不计通过。三次源码/
旧280件/四诊断、首轮静态失败完整输入及288件后续保护见final-run2/closeout；
状态projection-client-implemented-workspace-runtime-pending，不代桌面/N09或完整迁移。

冻结2,597输入，source SHA-256为
`ef882ee712ea757d8a46a6a900aad41c5ea442a6a92c97656cc55184282adddc`。
相邻授权配置4/0、Lease69/0、TLS信任5/0、地址解析5/0、
取消2/0、资源2/0及1/0、startup7/0、session_startup9/0、
workspace_startup5/0均过。产品debug EXE为42,706,944字节，SHA-256为
`635dd024d9f7b5289785d738aae37ae5a5c09c78548d114cbcc6159f215b3f87`，位于本轮final-run2/artifact，publishable=false。
最终同源/当前七文档/精确native diff/相关进程由closeout复核；开发编译失败和首轮
Clippy失败分别保留，不声称桌面接收、在线Loom、旧前台或最终用户包已通过。

## v384：正式菜单退出链与真实 Win32 类名发现回归

本轮沿用已分配一次的v0.2.33.384，开始前核v383最终源码/七文档、272份封存程序及
四份历史诊断。只修workspace_runtime测试发现层：Win32实际返回标准类名Static，
旧STATIC逐字比较漏掉合法菜单；改ASCII大小写不敏感，PID/精确owner/visible和确认
框标题条件不放宽。新增matcher子模块只拥有离屏NOACTIVATE窗口及RAII销毁；真实
RED为0/1，GREEN为1/0，错误PID/owner/隐藏窗口仍拒绝，不改产品代码或前台门禁。

开发阶段显式正式菜单退出1/0：自启隔离profile产品取得真实前台，经正式WM_CONTEXTMENU
和UIA“退出 Otter”→确认→最终revision+1及几何/扩展字段保持→进程成功结束。不用
删除Unit的WM_CLOSE冒充退出，不延长15秒等待或注入全局输入；CUA仅观察清单，未
点击/按键。最终冻结候选的严格静态、同源封存运行及源码/旧程序身份另行核验，不
把开发阶段通过或默认ignored当最终验收。完整N06/N01–N12、旧前台return/完整组/
压力未知失败、N09传统投送/接收策略/Tile消费者、资源长稳和最终用户包仍开放。

最终final-run严格fmt、全目标Clippy -D warnings/check、版本及3,742文件ratchet均过，
零违规/警告/新例外。仅两份native测试变更/一新增：exit194→197、matcher66有效
行，最大197，UTF-8无BOM。冻结2,590输入，source SHA-256为
`71f9cdf9b548a81b120558fe239058c99ceb6a69799f1069ed30094baa9457c5`。

同源封存八件的19运行门18过/1失败：退出11/0、Live checkpoint19/0、相邻Live58/0
逐层包含；本地手势9/0、原checkpoint6/0、profile3/0、autosave8/0、保存25/0、恢复
3/0、文件29/0、写入7/0、加载13/0、资源2/0和1/0、startup7/0、session_startup9/0、
workspace_startup5/0均过。workspace_runtime2/0/1 ignored通过移动/中断重启/最后Unit
删除及matcher；默认ignored仍不计通过。单独--exact --ignored正式菜单退出0/1，
失败在owned-product-foreground原15秒前提，未进入菜单。开发阶段1/0不能转移为
最终候选原生批准；本轮不再争抢前台、不延长期限、不关闭v379或旧前台失败。

debug EXE42,710,016字节，SHA-256为
`57007f89a88e2d3febf7bc27d8f2eaa97fcbbc4961cea7a8ef40203ea2890f29`，位于本轮
final-run/artifact，publishable=false。三次源码、built/sealed八件、旧272件/四诊断、
v383失败完整输入/七文档/日志及本轮失败完整2,590输入/七文档另行封存，七文档/组织
合同/精确native diff/进程收尾由closeout复核；下一轮保护280件和四诊断。状态为
native-exit-menu-foreground-pending，selectedRuntimeGatesPassed=false；无CUA点击或
键盘输入，不提交/推送/发布或替换用户程序，goal保持active，不代完整迁移/用户包。

## v383：正常退出的最终工作区检查点与 Live 一次性退出批准

本轮只分配一次v0.2.33.383，开始前核v382 final-run源码/七文档、256份封存程序及
四份历史诊断，证据在`linshi/otter-native-v383-exit-checkpoint`。继续N06，不改N09/
Loom。真实隐藏HWND与合成视频的RED为0/1：Live只变视频帧、模型不变时，旧Discard
仍被手动dirty判定拒绝；旧Save后续退同样因Live恒Changed不能完成。

退出Intent现在独立保存轻量持久模型/来源身份/Live完整RECT与opacity，不保GPU帧、
视频序号或捕获/控制权。Discard和保存后的一次性批准都重验该Intent，不把Live永久
标为clean。确认文案明确“不保存到文件”，不再承诺丢弃仍由默认恢复点保存的内容。

可写默认profile确认退出后，强制通过原保存单槽/worker/Receipt/CAS提交最终时间点，
不因旧自动stamp命中而跳过最新Live帧；等待态独立于exit_requested，不放松原guard。
最终提交之前与下一轮销毁之前都核意图/预期revision。新save/open/new-window及Live
freeze/recycle等操作取消旧退出、保请求和窗口；poll后先保留退出状态，不让文件UI
抢先消费。仅worker通知/原终态WM_NULL续进，无新timer/队列/忙轮询。

无首帧明确报告并保窗口、不建立潜伏退出；正常stale取消本次退出但不停止检查点；
真实冲突停止一次并保外部文件。没有profile或只读/已停止owner不获新写权，仍可
经明确Discard退出。只保证获准且空闲时捕获的最终静态时间点，不是持续视频、晚于
快照的所有帧或断电零丢失。开发首轮编译因checkpoint子模块exit遮蔽旧测试Choice
路径失败，已改为final_exit。第二轮冲突夹具在worker取得Windows共享锁后写目标，
触发Os code 32，随后八项共享锁中毒；现将外部冲突放在worker取锁前，不放宽产品锁，
第三轮新退出组9/0。集成测试移动后的include_bytes相对层级编译失败也已修正，所有
开发失败日志保留，不以修正后的成功抹除首次失败。

真实产品退出门走自启PID的正式WM_CONTEXTMENU→UIA“退出 Otter”→确认框→最终修订
提交→进程成功结束，不用删除Unit的WM_CLOSE替代。前台条件测试默认标ignored，最终
矩阵必须以--exact --ignored单独执行并记录，不将ignored计通过。开发product-exit1–7
未有一次证明整链成功：早期错误WM_COMMAND合同已修，随后有真实前台前提/菜单发现
超时，第7次终态为owned-native-menu witness timed out。菜单STATIC不用跨进程不可靠
的GetWindowTextW辨认，改核PID/精确owner/类/visible；不放宽产品前台门禁或延长期限。
已停止循环争抢前台；CUA只观察窗口，无实际点击/键盘注入，不操作游戏或旧版Otter。

首轮final-run严格fmt、全目标Clippy -D warnings/check、版本和3,741文件ratchet均过，
零违规/警告/新例外。20份native变更/6新增，最大450有效行为Application447→450；
Live window437→449、exit143→200、checkpoint220→234。两新增生产模块Intent62/
final_exit57行，三新增退出测试131/116/92、产品菜单测试194行，均UTF-8无BOM。
首轮源码冻结/构建后/运行后三次摘要一致，但19相关运行门仅17通过：startup4/3，
三失败同为“Unchanged saved workspace did not resume exit”；产品菜单退出0/1，
在owned-product-foreground前提超时。原closeout严格失败，不生成成功回执；完整2,589
输入/七文档/八程序与17/19失败回执另存final-run/failed-source及failure-receipt.json。

只读交叉核验确认旧smoke刻意保留另一Unit的execution并验退出清理，不应提前清理
夹具或删断言。完整restore idle误用于无profile退出批准，缩紧了既有Discard语义。
后续final-run2只分离“批准稳定性”和“最终profile快照空闲”，不放宽原保存Receipt/
restore guard/CAS；无profile明确放弃仍清理execution，profile忙时仍保窗口。新增真实
RED为0/1：无profile且execution仍活跃时Discard返回false，旧清理语义被真实复现。
修正后Intent保modal/来源/模型/Live和new-window/save/open等明确意图；完整idle仍在
profile捕获、最终提交前和销毁前核验。另加提交前/后晚到execution回归，旧smoke
断言/等待预算保持；最终同源后台实绩另记。前台已失败，不对修正候选重复争抢，也不继承旧字节
的原生通过结论；菜单整链保持未验，默认ignored不计通过。本轮版本不再递增。

最终final-run2严格fmt、全目标Clippy -D warnings/check、版本及3,741文件ratchet全过，
零违规/警告/新例外。仍为20份native变更/6新增，最大450有效行；最终两新增生产模块
Intent/final_exit均64行，三退出测试131/116/176行、产品菜单测试194行，UTF-8无BOM。
冻结2,589输入，source SHA-256为
`bf8ba2bb5b59f75f5e1af330cd457a1103209531bd71e778bd119723a8a57ab2`。

同源八程序的18个选定后台/隔离运行门全过：退出11/0、Live checkpoint19/0、完整
相邻Live组58/0，三者为逐层包含关系，不累计作独立覆盖；本地手势9/0、原检查点
6/0、profile3/0、显式autosave8/0、保存25/0、恢复3/0、文件事务29/0、写入7/0、加载
job13/0、资源2/0及1/0、startup7/0、session_startup9/0、workspace_startup5/0及
workspace_runtime1/0/1 ignored。最后一项通过的是原移动提交/中断重启/删除最后Unit
链，ignored仅为菜单退出，不计通过；修正候选没有重跑该前台门，不能声称19/19。

debug产品EXE为42,710,016字节，SHA-256为
`394128b414aa55675238577701b3958680d1cc8c8f292efe28a81f819ca0b64e`，位于本轮
final-run2/artifact/otter-native.exe，publishable=false。三次源码、八件built/sealed、
旧256件/四诊断、首轮失败完整输入/七文档/八件/19门日志、当前七文档/精确native
diff及相关进程由final-run2/closeout复核；下一轮保护272件并另保四诊断，不覆盖首轮。

本轮状态为exit-checkpoint-native-pending：只完成退出检查点实现与选定后台回归，
真实正式菜单→确认→最终提交→进程退出仍未验，不关闭v379前台return、v376两失败、
历史完整组/压力未知根因、N09传统投送/接收策略/Tile消费者、完整N06/N01–N12、总
资源长稳及最终用户包。无全局输入、提交/推送/发布或用户程序替换，SDK许可与预算
不扩，goal仍active。

## v382：Live 的事件驱动时间点检查点

本轮只分配一次v0.2.33.382，开始前核v381 final-run2源码/七文档、248份封存程序及
四份历史诊断；证据在`linshi/otter-native-v382-live-workspace-checkpoint`。继续N06，
不改N09或Loom。旧版Live仅首次可用帧写静态src，bindings/流/控制能力只在内存；
本轮自动路径同样只把Live物化为静态贴纸，不持久化目标/HWND/捕获或控制权限。

默认检查点和显式文件自动保存共享原单槽/Receipt/编码/原子事务。无首帧、拖动、菜单
或待处理freeze/recycle时延后，不因正常busy停用；已提交基线只保实际Receipt中的
轻量Live身份/几何/透明度，不保GPU帧或流序号。首帧和持久模型变化触发时间点快照，
单纯后续视频帧不逐帧写盘。手动dirty仍为Changed，默认检查点不修改手动文件目标/
原文，不自动执行workflow；恢复仍只保证最近成功提交，不是持续视频或退出最终flush。

静态审查发现原freeze/recycle先take全部flag却只处理首个，freeze还在worker忙时吞
新请求。现在只在空闲时短路领取一个，其他意图仍归原HWND；自动/显式操作归还单槽
后发一次内部WM_NULL续进，不新增队列/定时器或忙轮询。旧工作区/关闭owner仍须原
Receipt/身份复核，不把延后当跨工作区授权。新增真实WARP/隐藏HWND/磁盘/中断回归；
最终严格静态、封存同源程序和定向实绩待验证，不把开发编译通过当本轮完成。

开发首次过滤器遗漏live模块层级，实为0 tests，不计通过；修正选择后8项3过/5失败，
首个失败是夹具把Windows规范化目标与原路径字符串比较，其余四项为共享锁中毒。
现改比较保存前后真实target，非放宽产品路径校验；原日志保留。中断/图恢复/双请求
已分别独立通过，修正源码仍须最终同源全组验证。普通manual save或其他library忙时
仍沿原显式错误报告，不新增静默跨工作区排队；内部消息续进回归不声称单独证明每
个WM_NULL不可删，仍有其他内部窗口消息。原预算不扩，不关闭历史未知失败。

最终final-run严格fmt、全目标Clippy -D warnings/check、版本及3,735文件ratchet全过，
零违规/警告/新例外。18份native变更/4新增，最大437有效行为Live window429→437；
Live快照148、Stamp244、checkpoint220、autosave161；四新增测试分别246/108/91/92
行，均UTF-8无BOM，不新增生产owner。冻结2,583输入，source SHA-256为
`849d646103b495acf2dccba8266f1fbfd00bedc92bce880d576f0ca4f18cb3f8`。

同源八程序的17相关运行门全部通过且无ignored：新Live检查点8/0、完整相邻Live
freeze/save/refresh/recycle组47/0（包含上述8项，不累加为独立覆盖）、本地手势9/0、
原checkpoint6/0、profile3/0、显式autosave8/0、Application保存25/0/恢复3/0、文件
事务29/0、写入7/0、加载job13/0、资源2/0及1/0、startup7/0、session_startup9/0、
workspace_startup5/0、workspace_runtime1/0。包含真实PNG/同ID/分组连线、无首帧延后、
流不重复写、静态恢复不授权、多请求内部消息续进及跳过Drop的两阶段中断；普通EXE
仍验证移动提交/强制终止/重启恢复/删除最后Unit写空。未注入全局输入；普通启动门
只短暂激活自启离屏窗口，定向集合不代完整库、完整N06或用户验收。

debug产品EXE为42,658,816字节，SHA-256为
`10f16609227c9c0229e735a1c600ba903512b3275d569572e4bc65c85c4c6fee`，位于本轮
`final-run/artifact/otter-native.exe`，publishable=false。三次源码、八件built/sealed、
旧248件/四诊断、开发失败日志、七文档/精确native diff和相关进程由最终closeout核验；
下一轮保护256份并另保四诊断。最近成功Live静态时间点已接通，不承诺连续视频或
退出最终flush，不提交/推送/发布或替换用户程序，goal仍active。

v379前台return、v376两失败、历史完整组/压力未知根因、正常退出最终flush、N09传统
投送/接收策略/Tile整版消费者、完整N01–N12/总资源长稳与最终用户包仍开放。未提交/
推送/发布或替换用户程序，SDK许可与资源预算不扩，goal保持active。

## v381：默认静态工作区检查点与启动恢复

本轮只分配一次v0.2.33.381，开始前核v380 final-run2源码/七文档、240份封存程序及
四份历史诊断，证据在`linshi/otter-native-v381-workspace-checkpoint`。普通无来源启动
使用隔离`%LOCALAPPDATA%/OtterNative/workspace.json`（可由OTTER_NATIVE_PROFILE_DIR覆盖），
不扫描/迁移/改写旧Tauri目录。恢复采用原有有界文件读取、候选校验/隐藏建窗/发布链；
完成前不注册全局捕获、不显示工作区或接收正常后台内容。坏文件、未知schema、缺少
坐标或含混恢复副本终止启动，不降级为空再覆盖；唯一恢复副本只读并停用检查点。

静态编辑后事件驱动调度，复用同一保存单槽/后台编码/Receipt/原子文件事务，没有新
timer/worker队列/序列化owner。独立磁盘修订仍由session_write一次编码；检查点只更新
自身全字节基线和轻量Stamp，不改手动保存目标、dirty或workflow。无变化不写，busy
延后，正常stale丢弃后保存当前快照，错误/冲突停用并报告一次；手动/open/exit不提前
消费。profile持有跨进程排他文件租约与逐层非reparse目录句柄（最多128组件），进程
退出后OS回收租约，不靠删除lock文件解锁。检查点与手动JSON文件各自保冲突基线。

最后Unit删除在WM_QUIT之后、Wake关闭之前回收失效保存/恢复候选并提交空工作区；
整体退出有独立终态标记，不把批量销毁当删除全部。与旧版一样，只承诺最近成功提交
快照，未实现正常退出最终flush或断电零丢失。显式--session/image/capture是独立启动，
不读取/写入默认workspace；用户在默认工作区内主动打开文件则持续检查点跟随当前
工作区，但不自动覆盖导入文件。Live仍未有稳定自动检查点签名，出现Live即停用并
提示手动时间点快照，不恢复捕获/控制权限，不以静态链通过宣称完整N06完成。

新增--workspace-check为隐藏、workspace内容只读入口，必须显式隔离profile；会创建
该隔离目录/lease文件，但不创建或改写workspace.json，不发现真实用户profile。开发
首轮2/2，失败为同进程夹具残留WM_QUIT及只过滤低优先级WM_QUIT，已按原smoke的
有界队列排空修正；独立审查补上最后Unit删除时其他保存任务占槽的真实缺口和回归。
首轮编译可见性失败原日志保留；最终静态/同源程序/重启中断/相关运行与身份另记，
不预称通过，不提高旧预算；普通启动门仅短暂激活自启离屏窗口，不注入全局输入。
不改Loom、不提交/推送/发布。旧v379前台归还、
完整组/压力未知失败、Live自动保存、完整N01–N12/资源长稳与最终用户包保持开放。

真实普通启动开发首轮/次轮超时日志保留；次轮保留目录证实检查点已提交revision8，
但测试线程DPI unaware使跨进程SetWindowPos坐标被1.5倍虚拟化。沿现有DPI测试加入
PMv2 RAII并恢复原上下文，数字断言读取as_f64，不改产品几何或延长期限；第三轮
普通启动1/0。其后只读启动4/1，恢复副本夹具的one/two不符合原GUID命名合同，现改
为合法固定GUID，生产matcher不放宽。Owner缓存已验证磁盘revision，启动只解码一次、
提交成功才与全字节基线一同推进，UI调度不重复解析大JSON；恢复后继续保存另加断言。
最新源码仍须最终严格静态、同源封存及相关运行门，不继承开发中间通过。

最新开发检查点六项通过；首轮final-run的严格Clippy拒绝同一process测试夹具在lib test
中多处加载。该轮2,579个冻结输入完整保存后，lib根仅cfg(test,windows)单点加载原
夹具，旧文件中断及本轮profile/checkpoint测试改引用，不复制实现或allow警告。修正
候选使用final-run2，与首轮失败身份分开；最终实绩另记。

final-run2严格fmt、全目标Clippy -D warnings/check、版本及3,731文件ratchet通过，
零违规/警告/新例外。25份native变更/8新增，最大447有效行；Application432→447、
desktop入口416→443仅接线，检查点217、启动55、profile121、生命周期tests318、
中断83、真实EXE运行202、只读启动151行，UTF-8无BOM。318行测试单一负责Application
检查点状态/文件隔离/最后Unit生命周期，中断子进程已独立，不新增豁免。

冻结2,579输入，source SHA-256为
`0a77734aba522bdda0f24e304a6d8eef90b5c1198f226e65ee0669bc9346607f`。同源八程序的
14相关运行门全部通过：检查点6/0、profile3/0、显式自动保存8/0、Application保存
25/0、恢复3/0、文件事务29/0、会话写入7/0、加载job13/0、build/EXE资源2/0及1/0、
startup7/0、session_startup9/0、workspace_startup5/0、workspace_runtime1/0，均无
ignored。普通EXE实跑默认发现/移动事件提交/强制结束/重新启动位置恢复/删除最后Unit
落盘为空；自启离屏窗口核PID，无全局输入，不将这些定向集合当完整库或用户验收。

debug产品EXE为42,653,696字节，SHA-256为
`650cd64514a07c429ea9b2fe126b88822b684b12a5cc13e68ed245cf94570fb0`，位于本轮
`final-run2/artifact/otter-native.exe`，publishable=false。三次源码、八件built/sealed、
旧240件与四诊断、首轮Clippy失败完整2,579输入、开发失败日志、七文档/精确native
diff/相关进程由独立closeout核验；下一轮保护248份并另保四诊断。最近成功快照是
恢复保证，Live自动持久化/最终退出flush/N09传统投送与Tile整版消费者/旧前台归还及
完整组和压力未知失败仍开放；完整N01–N12、总进程资源长稳及最终用户包未验收，
goal保持active，不提交/推送/发布或替换用户运行版本。

## v380：已保存工作区的事件驱动自动保存

本轮只分配一次v0.2.33.380，开始前核v379完整源码/七文档、234份封存程序及四份诊断。
证据独立在`linshi/otter-native-v380-workspace-autosave`。N06已有显式保存/后台编码/
原子替换/恢复，缺的是用户编辑后的自动调度；本轮连接原生菜单、应用共享开关、
既有单任务和Receipt、磁盘原文冲突保护及重新恢复，不另建序列化或文件发布实现。

“自动保存当前工作区”默认关闭，只在当前进程中显式开启，要求已有成功保存目标。
多个工作窗口共享同一开关/代际；普通编辑消息后只保存空闲的已提交静态内容，同值
不重存，无空闲定时器/轮询。正常编辑令在途候选过期时丢弃旧候选，空闲后保存当前
快照；关闭再开启使旧代际失效，不复用旧授权。手动保存/另存为及打开请求保留到自动
任务回收后，由原UI路径处理。退出先取消并回收在途自动候选，再走原未保存确认；
不会在退出标志有效时绕过原Receipt。成功恢复另一工作区或另存新路径均关闭开关。
保存失败/外部磁盘冲突停用并只报告一次，当前内存和原磁盘基线保留；自动提交不
触发Loom工作流。Live仍要求显式时间点快照，开启自动保存会明确拒绝并停用，不能
拿恒为Changed的Live状态形成无限写盘。自动重开上次工作区、崩溃前持续恢复、完整
Live自动持久化仍未接通，N06/N01–N12不因此完成，也不改写旧版用户数据目录。

开发首轮4/2：真实隐藏HWND复现手势busy误停用，已复用原idle判定在快照前延后；
冲突测试在暂存后写文件被既有共享锁拒绝，现改在起线程前制造外部冲突，不放松文件
锁。编译时发现的菜单ID重复及smoke可见性错误已修正，原日志保留；打开请求提前
消费由静态交叉审查发现并增加回归。开发修正后八项自动保存测试通过；再复查补上
代际耗尽时原本已开启也必须停用的边界，并增加两种起始状态断言。Clippy大枚举
分支改为单个Box间接持有，不关闭警告或新增队列。首轮严格静态中fmt/Clippy/check
通过，行数门因Application494→501失败，完整冻结输入另存后将new构造职责整体移至
application_initialize；不新增豁免、不改字段/析构和初始化顺序。最终候选在final-run2，
保存/恢复/文件事务及真实产品startup/session_startup是本轮相关门，不重跑无关JS
集合充数。最终静态/封存程序/定向运行和身份结果另记，
不预称通过。v379前台归还路径、旧完整窗口/Application/压力未知失败全部保持开放；
本轮不占前台、不修改Loom、不扩SDK许可/资源预算，不提交、推送或发布。

final-run2严格fmt、全目标Clippy -D warnings/check、版本及3,723文件ratchet通过，
零违规/警告/新例外。19份native变更、4新增，最大432有效行；Application494→432，
完整构造模块74行，自动保存148行、smoke158行、边界tests244行，UTF-8无BOM。
构造逐token比较只允许依赖限定名调整，字段布局/析构顺序不变，接口和时序保持。

冻结2,571个输入，source SHA-256为
`b76da3f775b2eecb0212ffd34d278b4515b11dd2d94aec88e7cbb3cee0f88711`。同源六程序的
10相关门全部通过：自动保存8/0、Application保存25/0、恢复3/0、文件事务29/0、
会话写入6/0、加载job13/0、build/EXE资源2/0及1/0、startup7/0、session_startup9/0，
均无ignored。它们是定向集合，不把汇总计数当完整库测试或全功能验收。

debug产品EXE为42,606,080字节，SHA-256为
`0a9838258ace0d424fbf4dd1227596ac7de52f8540606b2fe749a01fa44cdb89`，位于本轮
`final-run2/artifact/otter-native.exe`，publishable=false。三次源码、六件built/sealed、
旧234件和四诊断、首轮行数失败完整源码、开发失败日志、七文档、精确native差异及
相关进程由独立closeout复核，下一轮保护240件并另保四诊断。默认启动恢复、崩溃
恢复、Live自动保存、N09传统投送/接收策略/Tile整版消费者和v379前台归还仍开放；
完整JS/业务、总进程资源长稳、完整N01–N12与最终用户包未验收，goal保持active。

## v379：前台许可诊断与后台验证候选

本轮只分配一次v0.2.33.379，开始前核v378源码/七文档及228份封存程序。证据独立在
`linshi/otter-native-v379-foreground-handoff`；现有v378仍是最近选定回归全部通过的候选。
v379保持`foreground-handoff-native-pending`，不把测试夹具改动或后台成功当作产品修复、
完整窗口/Application验收或完整N01–N12完成，不提交/推送/发布或替换用户运行版本。

先直接运行v378封存测试库，两项v376失败均复现：跨进程负向在Child激活前提失败，
Enter在下一轮前台获取失败。观测源码/程序分别封存后记录：AttachThreadInput临时
取得真实前台不保证解除关联后能授予另一进程前台许可；只加AllowSetForegroundWindow
仍返回E_ACCESSDENIED，故“补一行授权即可修复”的假设被反证。Enter还记录到guard
归还前台后下一段读到HWND(0x0)，不是产品错误接受后台输入的证据。

依据已联网核实的[AllowSetForegroundWindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-allowsetforegroundwindow)、
[GetForegroundWindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getforegroundwindow)与
[跨输入队列激活时序](https://devblogs.microsoft.com/oldnewthing/20161118-00/?p=94745/)，
显式诊断以自有启动按钮的真实点击授权已就绪的精确Child PID，不用ASFW_ANY、系统
设置修改或全局键盘。首次挂起进程尚未初始化USER，授权返回ERROR_INVALID_PARAMETER；
后续先建立USER队列再报告就绪。首次把就绪等待放在TLS计时中导致peer原期限失败，
现Application在Files/服务端计时前执行启动握手，不提高原网络/operation/Job预算。

原surface_javascript_fixture仅增加显式环境开关的一次启动握手，默认不等待；收到G仍
必须通过原真正前台/焦点断言，原有条件归还算法未放宽。外层诊断launcher只持有本
测试进程，120秒就绪期限/900秒外层期限、日志/队列有界；留在linshi，不增加永久工具
入口。Computer Use只点击经当前窗口/截图核验的自有按钮，不向游戏或其他应用发消息。

真实启动授权后的跨进程诊断已进入产品后台拒绝主体，但父进程恢复前台仍失败：下一
次AllowSetForegroundWindow会替换上一授权接收方。现有Child owner增加显式return命令，
当前前台Child仅归还自启父进程PID许可，由父线程自行实际激活；父→Child也只授权仍
持有的精确进程并核当前前台属于本测试。原5秒确认、64KiB输出、64字节/四条命令、
30秒Child生命周期及Drop不变，正常foreground/return/close只用三条，不建立新owner。

最终开发原生尝试被Computer Use报告的实际用户输入打断；日志证实在发Child激活命令
前父窗口已丢前台，未到达新增return路径。此后停止争抢前台，最终四项前台相关门不
执行且明确未验证，不能称双向交接已通过或关闭v376两项失败。开发编译失败、授权
失败、peer计时失败及所有封存源码/程序/回执均保留，不重复请求或删断言求绿。

最终仅四份native测试/测试接线变更，无新增文件：Application harness466→467、Child
fixture321→363、focus HWND240→241、共用fixture79→97有效行，最大467；仍各归原
owner，UTF-8无BOM。3,719文件ratchet零违规/警告/新例外，严格fmt/全目标Clippy/check
与版本通过。生产focus/keyboard/FIFO/权限源码不改，无新依赖、SDK、产品权限或资源
预算；原Otter隔离SDK用途不变，不开发Loom。

冻结2,567个输入，source SHA-256为
`0176bdb12c84679f8b705b65b358815bcb801d9b059414dfeb3faa352969832b`。同源六程序八个
后台门全过：JS Engine121/0/5 ignored、Session77/0/26 ignored、显式跨进程UIA1/0、
摘要绑定实际产品Enter worker1/0、资源2/0和1/0、startup7/0、worker2/0；ignored不计
通过，两个显式门另用--ignored实际执行，不将重叠集合相加冒充唯一测试总数。

debug产品EXE为42,539,520字节，SHA-256为
`5123127eb3003110811bf6617eeebd7103d2777cd4cbe40b2765e4f501af8b8b`，位于本轮
`final-run/artifact/otter-native.exe`，publishable=false。同源测试库与最后开发原生
失败程序字节相同，SHA-256为
`13b3d13ab568ac24bc3af2a5a745de26b9c713dc352f6dadd4604be56d86b0c4`；该0/1不被后台
八门覆盖。三次源码/六件built及sealed/旧228件、五套诊断源码树和四件诊断程序、
全部失败/七文档/精确diff与相关进程由独立closeout核验，后续保护234份封存件并另保
四件诊断。完整前台四门未重跑；两项v376失败及归还路径仍未关闭，不代表默认完整
JS、真实股票/Loom、Lease、OS隔离、全进程资源长稳、历史压力根因、完整迁移或最终
用户包验收，goal保持active。

## v378：四边实心边框的原生纵向链路

本轮复用已分配一次的v0.2.33.378，开始前核v377源码/七文档和222份封存程序；证据
独立在`linshi/otter-native-v378-border-sides`。只读Loom股票Surface需要四边边框，但本轮
通用夹具不复制股票业务，也不证明该应用或完整N01–N12已可验收。最终22相关运行门
全部通过，切片状态为`border-sides-selected-regression-passed`，不是完整迁移验收。

CSS新增border、border-top/right/bottom/left及width/style/color的四值简写和各边长属性。
宽度仅非负px/0、thin/medium/thick（1/3/5px）；线型仅solid/none/hidden，后两者used
width为0。省略简写成分重置为3px/none/currentcolor，展开为独立cascade键，保原important/
specificity/源序、反射/remove及原子拒绝。非继承；有正used width仍须显式border-box。
不支持dashed/dotted/double、border-image、逻辑方向、百分比/相对单位CSS边宽及完整CSS。
依据已联网核实的[CSS Backgrounds 3](https://www.w3.org/TR/css-backgrounds-3/)，不扩大支持声明。

原surface_box的Insets统一拥有四边宽度；旧声明式uniform/非法长度回退合同不变，原生
字体相对长度仍由原Context解析。测量、Grid/flex/stack、absolute padding box、文字/图片/
按钮/textarea、client geometry、scroll page/reveal、clip和hit全部消费各自起边及轴总量。
圆角内边界复用Shape的四角独立椭圆，CPU命中与D2D裁剪同形，不新增几何状态owner。

Border仍唯一拥有同factory缓存：四边共享一个ring和至多四个分区。颜色不同才分绘，
分区按距边/边宽的半平面交覆盖整个外盒，由ring去内洞；内部分色采用单像素owner，
外圆角沿ring mask抗锯齿，防半透明重叠与圆角漏洞。EDIT复用原投影/区域owner，新
border_clip只拥有临时HRGN并RAII释放；字段合同变化仍会失效旧草稿，不放宽安全门。

真实RED为0过/1失败：each border side must consume its own space，旧flow起点[11,3]
而非[19,5]。RED源码/程序已独立封存。开发编译类型错误、根选择器误匹配、rounded
内角alpha0而非128、两个HWND测试前提错误及过时负向样本均保留，不覆盖失败求绿。
开发第5次相关回归13过/0失败/1 ignored；新产品门须以最终EXE摘要绑定后显式执行。
旧CSS负向将缺solid的正border-width误作不支持，现保断言改为正used width但缺border-box，
并新增默认none/hidden的正向覆盖；Session首个失败后的锁中毒不冒充独立产品缺陷。

开发已验证真实child同步客户盒/稳定identity/更新恢复及回收，隐藏HWND的WndProc/同帧
WIC四边像素、EDIT不变字段合同下的字体重算/四角region/草稿保留；原生分色与单色
参考逐像素alpha一致，零宽/超厚边和非对称内角受保护。初步相邻measure/scroll/EDIT
通过；最终严格fmt/全目标Clippy/check/版本及3,719文件ratchet通过，零违规/警告/新
例外。精确native变更45份/11新增，最大445有效行为paint_frame455→445；Session
440→443仅三行测试接线，text_frame249→253仍为最终文字帧/缓存重投影同一责任。
新增CSS94、border_style50、partition58、EDIT clip111、EDIT测试93、HWND76、JS模型71、
child/产品68、native几何55、像素62、通用fixture24有效行，均UTF-8无BOM。

冻结2,567个源码输入，source SHA-256为
`895f5f7499b00986e3ac4bf6aaa9e876b89b4c946223b4710a192ec4eddffea6`。六件同源程序无
重编执行22门：native measure137/0、layout tree2/0、scroll21/0、round clip1/0、文字
Engine31/0、JS Engine121/0/5 ignored、Session77/0/26 ignored、EDIT62/0、border HWND
1/0、CSS HWND2/0、spatial/geometry/原ellipsis HWND各1/0。实际产品border worker1/0，
四原规模/原预算压力门各显式1/0；资源2/0和1/0、startup7/0、worker2/0。产品与压力
均另用--ignored实际执行，不将默认ignored或重叠集合相加冒充唯一测试总数。

debug产品EXE为42,539,520字节，SHA-256为
`4d143dff4eaffa5e4d0a2e55643f096e5026473dc4ab938a8103a58aa8058f81`，位于本轮
`final-run/artifact/otter-native.exe`，publishable=false。三次源码、六件built/sealed、旧
222件、真实RED源码树/EXE、全部日志摘要、七文档、组织合同3项、精确native diff与零
相关进程由独立closeout核验；后续保护228件封存程序，不提交/推送/发布。

不重跑或关闭v376两项前台失败、更早完整窗口/Application失败与压力未知根因；不因
局部通过放行默认完整JS、真实股票/Loom、Lease撤销、OS权限隔离、总资源长稳、完整
迁移或最终用户包。goal active，不开发Loom，不增加依赖/SDK/权限，不提高原32MiB
heap/256KiB stack/250ms/Job/startup预算，已授权Otter隔离SDK用途不变。

## v377：单行文字省略的原生纵向链路

本轮只分配一次v0.2.33.377，开始前核v376源码/七文档及216份封存程序；证据独立在
`linshi/otter-native-v377-text-overflow`。真实只读Loom股票Surface多处需要nowrap/hidden/
ellipsis，但此通用切片不是该应用已可运行的证明，也不缩小完整N01–N12退出条件。

CSS的text-overflow仅接受clip/ellipsis，非继承；文字Element叶允许同轴overflow:hidden。
ellipsis限定nowrap、left或缺省对齐、实际单行。容器、canvas/input/button、混合轴、
显式文字overflow:visible、其他white-space/align、压平后代的独立非继承样式仍拒绝。
Unicode强制分隔符不能仅凭nowrap标签误判单行，最终原生布局继续校验实际lineCount。

新增surface_text_overflow仅在最终text_frame分配宽度后对新建、尚未共享的DWrite布局
设置CHARACTER trimming和ellipsis sign；固有测量不走该入口，不截断完整规范化UTF-16，
不更新trimming前metrics/baseline，也不回写父盒。native高度至少容纳整行，实际像素
沿TextItem/祖先clip裁剪，纵向矮盒不触发末行省略。后帧重新创建，滚动只重投影原缓存，
不能原地修改共享旧COM布局。隐藏后缀作为一个视觉cluster命中/跳跃，词级选择省略号
返回整个隐藏后缀，不把其内部某个不可见词冒充省略号内容；复制仍可获得完整文本。

实现依据已联网核实的[DirectWrite trimming](https://learn.microsoft.com/en-us/windows/win32/api/dwrite/nf-dwrite-idwritetextformat-settrimming)、
[ellipsis sign](https://learn.microsoft.com/en-us/windows/win32/api/dwrite/nf-dwrite-idwritefactory-createellipsistrimmingsign)与
[CSS Overflow 3](https://www.w3.org/TR/css-overflow-3/#text-overflow)。不宣称完整CSS/bidi/inline
排版或UIA TextPattern覆盖。最终状态为`text-overflow-selected-regression-passed`：固定
19个相关运行门全部通过，只关闭本轮受限文字省略切片，不是完整迁移/正式发布/用户包。

真实行为RED为0过/1失败，原错误missing native ellipsis, not just a saved CSS string；
源码2,550输入、SHA-256为
`1225da99914454de5d722b9efe72e20e68763dc8ea322102c9e615c9acd1b5eb`，实际RED程序SHA-256为
`8ea5e3dd8f044763caae83baa02a856d7929847776899c0eb8565379302c042c`。首个同名意图的
controlled-ellipsis-red其实因不存在的Context::default编译失败，不把它冒充行为RED。
后续开发固定点命中失败、HWND测试越界访问私有Session字段的编译失败，以及
overflow-focused2的9过/2失败/1 ignored均留存，不覆盖或删断言求绿。

失败推动独立DWrite探针定位：省略号HitTestPoint返回isTrimmed=true、isText=false、
height=0、inside=false，而HitTestTextPosition返回相同隐藏范围的实际行高。新adapter
只恢复异常trimmed命中，普通路径不变；inside恢复仅覆盖sign实际矩形，nearest sign
之外的盒内空白仍不算文字。适配接入caret/point/vertical/selection，解决选择起点
丢失及End/reveal的Invalid Surface text layout。诊断探针1/0不当产品门，原测试源码/
程序单独保留，临时输出已移除并替换为正式回归；最后开发相关回归13/0/1 ignored。

最终冻结2,556个输入，source SHA-256为
`cbc2403cb3cff4c19d870523766ffde4275ea285fa9658bceba43ab19293a105`。fmt、全目标严格
Clippy/check、版本及3,708文件ratchet通过，零违规/警告/新例外。精确native差异17份、
7新增、最大440有效行，均UTF-8无BOM：新省略模块108、DWrite测试232、WIC像素118、
JS模型56、child/产品127、HWND65、通用fixture22；旧文件仅准入/最终呈现/命中及测试
接线，不增加进程/线程/持久缓存或共享COM修改，不引入新依赖/SDK/权限。

最终同源六程序无重编执行19门：省略native7/0、文字引擎31/0、原生测量131/0、选择
25/0/1 ignored、JS Engine117/0/5 ignored、Session76/0/25 ignored；省略HWND1/0、CSS
HWND2/0、spatial与typography HWND各1/0，摘要绑定实际产品省略worker1/0。四个原规模/
原预算压力门（CSS Hover Engine、nth Engine、CSS child/实际产品）分别显式1/0；资源
2/0和1/0、startup7/0、worker2/0。ignored不计通过；产品与压力门以--ignored另行实际
执行，重叠集合不相加冒充唯一测试总数，默认组其他ignored未被声称已验收。

WIC glyph像素与独立DWrite trimming参考一致且不同于clip；极窄盒不越界，完整文本/
combining/surrogate/ZWJ/bidi后缀、sign两侧命中、整后缀词选、End完整扩选及无谓滚动
防护通过。真实scroll缓存平移/恢复不增measure，resize矮盒不触纵向省略、旧clone不
被后帧修改；Unicode强制多行/非法native值拒绝。QuickJS原子性/非继承/原型污染、
同child身份/同步几何/更新与回收、真实隐藏HWND像素/命中/坏候选保旧帧均通过。

最终debug EXE为42,488,320字节，SHA-256为
`ae0717ded68f302ad31191cf556c4b28be02ce9707de2a1c632069c3db99aaf9`，位于本轮
`final-run/artifact/otter-native.exe`，publishable=false。收尾独立核三次源码、六件built/
sealed、旧216件、行为RED源码树/程序与探针；七文档、组织合同3项、精确diff及零相关
进程由收尾门核验，后续保护222件最终程序，RED/探针另保留，不提交/推送/发布。

本轮仅隐藏自有HWND，无全局输入，未重跑或关闭v376跨进程focus与Application Enter
两项前台失败，也不覆盖更早完整窗口/Application失败或压力未知根因。四压力本次过
不证明性能不退化/加速或总资源长稳。goal active；默认完整JS、真实股票/Loom、真实
Lease撤销、OS权限隔离、完整N01–N12与最终用户包仍未验收；不开发Loom、不提高原
heap/stack/250ms/Job/startup预算、不扩大已授权的Otter隔离SDK用途。

## v376：CSS 临时 winner 容器与原预算复核

复用本轮已分配一次的v0.2.33.376，先核v375最终源码/七文档和210份旧封存程序；证据
独立在`linshi/otter-native-v376-css-collect`。最终20个相关运行门18过/2失败，仍为
`partial-runtime-failure`；四个原规模压力门本次均过，不据此关闭历史压力根因或完整
迁移。全部前台测试仅定向自有HWND，不向游戏发送消息或全局输入，失败没有重试。

先用v375封存库作固定背景诊断，再增加受控RED：64节点/16条双声明规则的旧winner Map
实际产生2,048次get、128次set和65次entries；RED源码/实际程序单独封存。生产仅将原
css_sheets.computed内唯一winner容器换成捕获intrinsic创建的null-prototype record。
key仍只来自原声明准入后的canonical CSS名称，优先级、inline、三项specificity和源序
不变；记录只活在本次计算，不增加跨帧缓存、第二样式owner、队列或权限。

新真实QuickJS测试2/0：上述winner Map调用降为0，完整输出与仅恢复旧Map的测试副本
一致；canonical属性名及constructor/__proto__的原型accessor/枚举污染、公开Object/
Map intrinsic替换不影响私有记录。原投影六项oracle继续保留，reference先恢复Map再
执行原rank/全表扫描差分，32轮Rust优先级oracle与14夹具不删除；开发投影6/0、CSS
模型9/0。没有改变JSON getter/toJSON清洗或原32MiB heap/256KiB stack/250ms/Job预算。

最终冻结2,549个输入，source SHA-256为
`acd7c4a55567a579ceffdc9d0d72088b4b0b06d102f35e1c346ee4b28d9b8e0f`。fmt、全目标严格
Clippy/check、版本和3,701文件ratchet通过，零违规/警告/新例外。四份native变更：
新增cascade_record_tests116行，sheets75→74、projection tests300→301、Engine测试接线
383→386有效行，最大386；均UTF-8无BOM。无生产模块移动或新依赖/SDK/权限。

同源六程序无重编完成固定20门：Engine114/0/5 ignored、Session74/0/24 ignored，包含
上述新测试/原CSS投影模型及Hover功能；CSS Hover HWND1/0、旧Hover窗口6/0、focus窗口
2/0/1 ignored、Hover Application1/0、focus Application3/0，三项摘要绑定实际产品
CSS Hover/Hover/Enter各1/0；四项显式压力CSS Hover Engine、nth Engine、原CSS child/
实际产品各1/0。资源2/0与1/0、startup7/0、worker2/0。ignored不计通过，集合不相加，
Engine/Session并非默认完整JS、真实业务或完整迁移验收。

两门各0/1：显式跨进程后台focus负向在自有Child激活确认失败，尚未进入产品拒绝
主体；Application Enter第一轮已完成两项原batch回执，第二轮prepare在真实前台前提
失败（原错误为focus fixture requires an interactive desktop），不能说完整键盘合同
已通过。未核失败HWND的进程归属，不归因游戏；不重试、放宽产品前台检查或用普通
focus/产品Enter通过覆盖。旧完整窗口/Application组未重跑，历史未知根因继续开放。

前后固定15样本均完成256节点，仍不构成整体加速证据：墙钟中位数no-rules104.995→
110.536ms、tag195.499→195.387ms、class203.898→214.071ms、tag-nth176.022→179.740ms、
class-nth193.197→208.132ms；多数略慢，不能把内部调用减少直接写成性能提升。单次
阶段诊断Collect151.2134→164.3549ms、computed79.8216→88.8018ms；该阶段副本观察上限
仍为2秒、非生产预算门。未测CPU/墙钟拆分，无证据将变化归因负载，也未建立性能
不退化或长稳结论。四压力本次通过不覆盖v375四失败，保留后续性能调查。

最终debug EXE为42,479,104字节，SHA-256为
`a69293c673a11cba258597bc2d4a3a7b841a0581954c471dbd11b9f1e9597239`，位于上述
`final-run/artifact/otter-native.exe`，publishable=false。三次源码/六件built与sealed/
旧210份/RED分别核验；七文档、组织合同、精确diff与零相关进程由收尾门独立检查，
下一轮保护216份最终程序，RED另保留。goal active；默认完整JS、真实股票/Loom、真实
Lease撤销、OS权限隔离、总资源长稳、完整N01–N12及最终用户包仍未验收，不提交/推送/
发布，也不开发Loom或扩大原Otter隔离SDK许可。

## v375：author stylesheet 的可信 Hover 状态

最终为`partial-runtime-failure`：修正候选final-run2的21门17过/4压力失败；功能/产品/
原生Hover和旧负向样本闭环通过，但压力及首轮完整窗口前台失败仍开放，不能宣布
完整迁移或本轮全部验收成功。下述首轮与修正候选的成绩/身份分别保留，不互相覆盖。

仅分配一次v0.2.33.375，先核v374最终源码/七文档和198份旧封存程序；本轮证据位于
`linshi/otter-native-v375-css-hover`。只读真实Loom stock-monitor的样式消费方，发现
`.stock-control:hover`等规则被author matcher拒绝；不翻译股票业务或开发Loom。
CSS Selectors当前规范已联网保存，HTTP 200，祖先因被指向后代匹配:hover；只实现
现有非捕获Hover输入所能证明的author子集，不将规范链接当完整浏览器符合性。

原Hover owner复用上一可信目标，沿当前私有父链最多32节点匹配；脱树为false，移动
不保留旧祖先的样式。matcher只增加:hover及原class级specificity，未知pseudo仍
原子拒绝；bootstrap完成工厂间私有接线，路径变化/clear通知原DOM mutation identity。
不读取公开parent/contains，不从selector递归读取computed，也不新建DOM/几何缓存、
输入队列、宿主权限或跨帧历史。suspend/dispose/原生leave沿原owner清理状态。

受控RED在旧matcher挂载失败0/1，源码/实际程序已封存。首轮开发夹具失败0/3/1 ignored
含两项锁中毒；Engine诊断1/1/1 ignored定位到同步synthetic分派内几何读取的
`Maximum call stack size exceeded`。分开验证synthetic状态不变与native回调几何，
不扩大256KiB栈；修正后相关开发6/0/2 ignored，实际产品和压力门必须另行显式运行。
最终严格静态、选定封存矩阵、源码/程序身份与行数按真实终态另记，失败不覆盖。

本轮仅author stylesheet，不扩DOM querySelector/matches/closest的:hover语法，
不实现静止光标下因布局改变而自动重新命中；现有可信采样/Host归还接缝仍是边界。
默认完整JS、真实股票/Loom、Lease撤销、OS隔离、总资源长稳、完整N01–N12与最终
用户包仍未验收，goal active；旧完整Application等历史未知失败不被相关通过关闭。

首轮`final-run`冻结2,548输入，source SHA-256为
`f07079510a33303b3f9e04bc9f2f5f56dbb7bf98648e6cc92d0142e4c009c329`；严格静态及3,700
文件ratchet通过，十份native变更最大437行，新文件分别132/124/152/44行，全部UTF-8
无BOM。原三份生产owner为hover44→55、selector135→136、bootstrap147→148；其他
原文件仅测试接线，不增加生产状态owner。六件封存程序完成预定22门，18过/4失败。

新增CSS Hover child3/0/1 ignored、Engine2/0/1 ignored、HWND1/0、实际产品1/0，旧
Hover child4/0/1 ignored、窗口6/0、Application1/0与focus Application3/0通过。
新Hover及原nth/child/product四个显式压力门各1/0，旧Hover产品1/0、nth模型4/0/1
ignored、资源2/0和1/0、startup7/0、worker2/0通过；ignored不计通过，集合重叠不相加。

CSS模型8/1、投影差分5/1、Session12/62/24 ignored的直接失败均指向旧
javascript-css-rejection.js仍要求span:hover被拒绝；Session另外61项为锁中毒。
完整窗口103/27/1 ignored为旧Hover用例获取前台失败及26项锁中毒，实际前台为
HWND(0x10b32)，没有验证其进程归属，不归因游戏，也不以独立6/0代替完整窗口通过。
下一候选只将过时的负向样本改为仍未支持的pseudo，保原子拒绝断言和本轮正向证明；
不重跑未修改的旧完整窗口前台失败求绿，首轮结果仍开放。debug EXE为42,479,104
字节，SHA-256为`971a4ae50a2c700e219e125697092659c1d67268cbcd337634294213a70462cd`，位于
本轮final-run/artifact，publishable=false；三次源码/六件/旧198份与收尾分别核验。

修正候选`final-run2`复用v375，仅将原拒绝fixture的span:hover改为span:hovering，
保原子替换/adoption/inline断言；三个生产owner与首轮逐字节一致。首轮2,548源码输入/
七文档/六程序均另存，不覆盖失败。新候选不重跑未改的旧完整窗口组，预定21相关
门一次执行。最终冻结2,548输入，source SHA-256为
`e94659b83ac116be917fa54c4f0b3f442ba1adf219cb50ffabe64b34b365d2f1`；fmt、全目标严格
Clippy/check、版本和3,700文件ratchet通过，零行数违规/警告/新例外。十一份native
变更最大437行；四新增132/124/152/44行，旧拒绝fixture27→27，全部UTF-8无BOM。

六程序无重编完成21门，17过/4失败：新Hover child3/0/1 ignored、Engine2/0/1 ignored、
HWND1/0、产品1/0，旧Hover child4/0/1 ignored、窗口6/0、Application1/0、focus
Application3/0、CSS模型9/0、投影差分6/0、nth模型4/0/1 ignored、Session74/0/24
ignored、旧Hover产品1/0、资源2/0与1/0、startup7/0、worker2/0通过。显式产品门确实
使用--ignored执行；默认ignored不计通过，Session不是全部Engine/完整JS或业务验收。

四压力门各0/1：新Hover Engine、原nth Engine、原CSS child的Report均在Collect后
engine_expired=true，geometry/canvas期限也已失效；新Hover失败发生在LocalInput，
尚未分清enter/leave。原CSS产品仅返回Invalid JavaScript Surface operation，没有
子进程详细阶段，不能把三路径观测无证套用到产品。原256节点/16规则/250ms及所有
完成断言不变；首轮四门各1/0不覆盖最终失败，也不据此归因游戏/负载或关闭历史压力
根因。不再次重试求绿、不提高heap/stack/operation/Job/startup预算；本轮保持partial。

最终debug EXE为42,479,104字节，SHA-256为
`c6180005b49c8b0975bd061efd04fc446728c8bce702308eda868271072230d6`，位于
`linshi/otter-native-v375-css-hover/final-run2/artifact/otter-native.exe`，publishable=false。
三次源码/六件built与sealed/旧204份、首轮源码/七文档/六件/日志及RED分别核验；
七文档/组织合同/精确diff/相关进程独立收尾，下一轮保护210份封存程序，RED另保留。
完整窗口首轮103/27/1 ignored与旧完整Application等未知失败未关闭；goal active，
默认JS/真实股票/Loom/Lease/OS隔离/总资源长稳/N01–N12/最终用户包仍未验收，不提交/
推送/发布，不以功能与静态通过冒充压力或完整迁移完成。

## v374：焦点测试区分线程焦点与真正前台

复用已分配的v0.2.33.374，先核v373源码/七文档和180份旧封存程序。证据独立在
`linshi/otter-native-v374-focus-admission`，不覆盖旧失败。一次带阶段/epoch观测的
Application focus/blur诊断1/0未复现v373，不能据此关闭历史根因。同线程双窗口
控制实验0/1：SetFocus同时切回前台，实验前提失败，尚未进入产品负向主体。

改用既有UIA Child owner的独立进程foreground模式，PID按所持Child验证；不要求
UIA action，不复制进程框架，仍走原管道/容量/30秒生命周期/Drop清理。修正控制
实验1/0证明线程焦点仍为parent但真实前台是自有子进程时，当前epoch的请求被拒绝，
未抢前台、未入队，恢复前台后空请求不重放。原产品前台/焦点/权限准入没有放宽。

Application focus测试的每段prepare及stale负向使用原Foreground guard建立/恢复
前台，在发送和交付关键点断言真实条件；不再以SetFocus代替前台，也不让stale/policy
负向因另一前台拒绝条件而虚假通过。无请求重试、额外等待、全局输入或预算放宽。
临时产品/测试trace已移除并留在诊断副本；产品focus源码应与本轮before完全一致。
最终相关矩阵、静态/行数、源码与六件封存身份按实际终态另记。完整N01–N12、默认
完整JS、真实股票/Loom、真实Lease撤销、OS隔离、长稳与最终用户包仍开放，goal active。

首轮`final-run`冻结2,544输入，source SHA-256为
`db5899f3113d9b4542a884e04378521c273b698959bb7a0f4884fdb1887cedfa`；严格静态/3,696
文件ratchet通过，五份测试/仅测试接线变更最大281行。六件程序完成固定11门，6过、
5失败：跨进程负向0/1及focus窗口组2/1都在Foreground前提失败，缺调用阶段快照，
不归因外部游戏；独立Application焦点0/3为完成两轮交付后的updated旧GetFocus断言
失败+两项锁中毒。完整Application13/12为Enter前台获取失败+十一项锁中毒，完整
窗口97/33为新跨进程前提失败+三十二项锁中毒。UIA默认跨进程1/0、产品keyboard1/0、
资源2/0与1/0、startup7/0、worker2/0通过。集合重叠不相加，失败不覆盖。

首轮debug EXE为42,475,008字节，SHA-256为
`2a487e98a5838ee3c0c62e404bc391fed2b626da1d96cd29abf206df846e7f14`，位于上述
`final-run/artifact/otter-native.exe`，publishable=false。源码/六件/旧180份及开发
三件诊断分别核验；首轮状态partial-runtime-failure，不能作为后续修正源码的成绩。

复核原JS夹具update只改field.value、不派focus请求；切焦/blur已在prepare实际交付
边界断言。移除updated跨网络批次仍要求parent持焦的旧假设，保留EDIT identity、
server值、原change文本、同child/FIFO/TLS/Grant断言。原Foreground夹具仅增加
track_caller，使获取失败直接定位具体调用阶段，不增加重试或等待。新增跨进程
负向沿仓库现有跨进程测试惯例标为显式ignored，但最终矩阵仍用--ignored强制运行
其独立门，失败必须保留；默认组的ignored不计通过，不能借此宣布该负向成功。

修正候选使用全新`final-run2`，仍为v374，不覆盖首轮六件或receipt。预定10门：
显式跨进程负向、默认focus窗口组、独立Application focus三用例、默认完整窗口、
旧UIA跨进程、实际产品keyboard及四个资源/启动/worker门。为避免重跑未修改的旧
Enter前台获取失败求绿，不重复完整Application组，其首轮13/12仍为开放失败；
后续相关成绩不覆盖首轮完整矩阵，最终静态/源码/十二件新旧候选身份按实际终态另记。

`final-run2`冻结2,544输入，source SHA-256为
`f5d0e20dcf0b6fc800ea5d7d179d1195ca98e16b94ad6fb326aebc3513cf31e2`，严格静态/3,696
文件ratchet通过，六变更最大281行。10门7过/3失败：focus窗口2/0/1 ignored，UIA/
产品keyboard/资源/启动/worker均过；显式跨进程0/1通过track_caller定位到从父线程
获取子进程前台的调用，尚未发送产品请求。Application焦点1/2：两轮focus/blur主
用例通过，ABA在交付前丢前台，其后policy锁中毒；完整窗口115/14/1 ignored为
Pointer发送前丢前台+十三项锁中毒。二者当时前台均为HWND(0x480c02)，未验证其进程
归属，不归因游戏。六件新旧候选/三次源码/旧186份由独立closeout核验，仍partial。

第三候选`final-run3`只修跨进程夹具的激活完成边界：父线程不再调用面向本地
窗口的focus_foreground(child HWND)，而通过所持Child管道发一次foreground命令；
子线程本地SetForegroundWindow/SetFocus后核实际前台再确认。沿原5秒事件确认、
64KiB输出/64字节四条命令/30秒生命周期/Drop，不循环激活、不睡眠重试，也不
放宽任何产品预算。共用Foreground owner增加被动guard构造以恢复原前台，原
本地获取流程/AttachThreadInput/有条件归还算法不变，防止为测试复制清理owner。
同一预定10门另用新身份运行，不重复旧完整Application组；前两候选十二份程序
和失败均保持，最终成绩、行数与旧192份保护按实际终态记录。

最终`final-run3`冻结2,544输入，source SHA-256为
`6fb4ccd21e8d499fd92786a0789be7b51d26422e07657d4d7d45ceda3147301e`。fmt、全目标严格
Clippy/check、版本及3,696文件ratchet通过，零违规/警告/新例外。六份测试/仅测试
接线变更：Application focus93→128、Child fixture277→321、process tests193→193、
uia root99→101、focus HWND173→240、共用fixture75→79；全部UTF-8无BOM。321行
fixture仍单责拥有原Child/管道/事件/清理，不为新模式另建进程框架或生产状态。

六件封存程序无重编完成预定10门，全过：显式跨进程后台focus1/0，默认focus窗口
2/0/1 ignored，Application focus3/0（两轮blur/change/TLS/Grant/FIFO/同child，以及
ABA/policy负向），完整默认窗口129/0/1 ignored，原UIA跨进程1/0、摘要绑定实际
产品keyboard1/0、build_resources2/0、exe_resources1/0、startup7/0、worker2/0。
ignored不计通过；同一个跨进程负向另用--ignored的独立门确实通过，集合重叠不相加。
不重复首轮完整Application的旧Enter失败求绿，首轮13/12与第二候选115/14等历史
失败保持开放，不能用相关门全过关闭未知前台/keyup/Pointer等历史根因或声称长稳。

最终debug EXE为42,475,008字节，SHA-256为
`4ebc250e39ee700488d9fa0bf261f6717df0a0ad973b2384b3fca451f94119eb`，位于
`linshi/otter-native-v374-focus-admission/final-run3/artifact/otter-native.exe`，publishable=false。
三次源码/六件built与sealed/旧192份、前两候选的源码和七文档副本/十二份程序、
三件诊断身份分别核验，七文档/组织合同/精确diff/相关进程独立收尾；下一轮保护
198份封存程序，诊断件另保留。状态focus-fixture-selected-regression-passed，goal
active；产品focus源码与本轮before完全一致，未扩大权限、预算、默认JS或SDK许可。
完整N01–N12/真实股票/Loom/Lease/OS隔离/总资源长稳/最终用户包仍未验收，不提交/
推送/发布，不把当前焦点测试闭环替代原始完整Windows原生目标。

## v373：声明式键盘测试显式拥有 modifier 状态

先核v372源码/七文档和174份旧封存程序，仅分配一次v0.2.33.373；独立证据位于
`linshi/otter-native-v373-keyboard-state`。一次有观测的原四键盘测试4/0：线程焦点
正确、四modifier为false，而前台不是目标窗口，不能把v372四项直接失败简单归因
前台。其历史日志缺实际modifier/准入快照，本轮不推断已经找全历史根因。

原测试的普通Tab/Enter直接继承GetKeyState。构造线程内Ctrl按下的确定性RED后，
原夹具Tab得到None而非Some("a")，证明其无修饰键语义没有显式建立。只修该测试
owner：每次普通消息保存完整256字节键盘表，只清Ctrl/Alt/Win/Shift及左右变体的
按下位，保留其他键/toggle状态，正常返回和unwind均恢复原表；不注入全局按键、
不激活前台、不重试消息，不改产品GetFocus/modifier/策略/候选准入。

新增受控Ctrl/Alt/左右Win/Shift环境下普通Tab/Enter及嵌套恢复，raw消息保产品快捷
键拒绝/Shift反向导航，另精确核unwind后的完整256字节恢复。临时观测输出只留证据
副本，不留在最终测试。开发RED、最终静态/封存运行/身份分开记录，不以新夹具
通过覆盖v372或旧Application前台/keyup/候选等未知根因。未改生产模块、依赖、权限、
默认JS、资源预算或SDK隔离用途；完整N01–N12/真实股票/Loom/长稳/用户包仍开放。

最终`final-run`冻结2,544输入，source SHA-256为
`351327a33a6cd2f39f7a9cc0933ec891757795abe1724b877ec0f76091758156`。fmt、全目标严格
Clippy/check、版本和3,696文件ratchet通过，零违规/警告/新例外。native仅一份旧
测试文件152→296有效行，增长144行，单一责任为声明式键盘消息状态/准入/候选与
夹具恢复验证，无新增生产模块，全部UTF-8无BOM；不为该局部夹具引入共享状态owner。

六件封存程序无重编完成8个相关运行门，7过、1失败。键盘fixture7/0、完整窗口
129/0、摘要绑定实际产品keyboard1/0、build_resources2/0、exe_resources1/0、
startup7/0、javascript_worker2/0。集合重叠不相加，压力三路径本轮未重跑。完整
Application14/11：本次Enter通过，但focus/blur在prepare期望EDIT持焦时GetFocus
为HWND(0x9f0cf6)而非HWND(0x92010c)，随后十项PoisonError，含Hover/Pointer未进入体。
日志没有该时刻完整前台/准入快照，不能直接归因游戏或宣布产品focus机制已修；
不重发该失败步骤或用窗口129/0覆盖Application失败。unwind probe的预期panic已
精确捕获并恢复完整键盘表，不是额外失败。v372四项历史根因也不因本轮通过而关闭。

最终debug EXE为42,475,008字节，SHA-256为
`f6beb6e01d61afcaafe17750a72dd1a6698cf861968901e2a832e49c807ba096`，位于
`linshi/otter-native-v373-keyboard-state/final-run/artifact/otter-native.exe`，publishable=false。
三次源码/六件built与sealed/旧174份，以及独立诊断、RED、GREEN三件程序和测试
源码副本由closeout分别核验；七文档/组织合同/精确diff/零相关进程独立收尾。
下一轮保护180份最终封存程序，诊断件另保留。状态为partial-runtime-failure，
goal active；旧前台/keyup/候选/form/startup/压力根因、默认JS、真实Lease撤销、
OS权限隔离、真实股票/Loom、完整N01–N12、总资源长稳/最终用户包仍开放，不提交/
推送/发布，不扩大SDK许可。

## v372：普通鼠标 Hover 的包装节点闭环

先核v371源码/七文档和168份封存程序，复用已分配的v0.2.33.372，不重复递增。
证据位于`linshi/otter-native-v372-hover`。真实股票consumer的chartWrap监听无按键
pointermove/leave，使用Canvas的client rect与rAF更新；本轮只接其通用输入/渲染前提，
不修改Loom或把股票业务写成Rust。默认JS、真实Loom/股票、完整N01–N12仍未验收。

无按键HWND move/leave沿原Mount FIFO/Runtime进入同child，通用最上层可见DOM命中
允许包装节点监听。共享原32槽/sequence，同时计入Canvas terminal与hover leave预留；
仅同target/epoch且未seal的队尾hover合并，prepared证据不改写。硬取消撤销未派样本，
已派路径补一个leave，复用waiting槽/序号并保非hover顺序；重复取消更新epoch，不凭空
为未交付enter生成leave。Canvas up只续结已有hover生命周期，不为纯拖动新增事件。

hover不夺焦点或capture，允许同前台Surface内原生EDIT持焦；其他capture和带按键move
不走此路径。真实失活/隐藏/几何变化仍硬取消；软背压及同节点JS回帧沿原生命周期。
JS仅保存最多32节点的上一条hover路径，不复制DOM/几何；原EventTarget派over/out/
enter/leave/move，relatedTarget经同owner的brand检查。enter/leave不冒泡、不可取消、
不composed，其余三项冒泡/可取消/composed。mouse/primary/id1、buttons0、button-1、
DIP与四modifier沿私有有界wire，不包含窗口或远端authority；suspend/dispose清路径。

开发初轮私有重导出、Application测试缺WM_MOUSELEAVE导入/AuthorizedWindow转发的编译
失败均保留；新夹具曾使用不支持的文字叶overflow与缺少初始Canvas布局尺寸，保留其
child/HWND/Application失败，修正夹具不弱化hover断言。最终静态、封存运行和身份在
实际执行后另记，不继承旧版本或开发中间字节成绩。32MiB heap/256KiB stack/250ms、
Job/startup、JSON/emit预算及SDK隔离许可不变；Job不是OS权限沙箱，policy关闭不代真实
Lease撤销。完整Pointer/MouseEvent、touch/pen、JS capture API、长稳和最终用户包仍
开放，不提交/推送/发布，goal保持active。

后续开发验证child4/0/1 ignored、HWND5/0，包含后台/带键负向、真实hide及Host借出
leave归还接缝。Application定向断言证明前台/编辑有效而pending为false，定位原Mount
pending仅识别Pointer/表单、遗漏Hover；补原调度可见性并新增消息后pending断言，不
以手动prepare的HWND通过掩盖Application未调度。双Canvas夹具的images断言从误写1
校正为严格2，不改事件顺序/像素断言。旧Pointer开发第二轮前台丢失及随后锁中毒保留，
不以其他轮通过清除该未知根因。最终封存字节成绩另记。

首轮`final-run`严格Clippy因wire的test module位于impl之前失败，保留原日志；仅按
lint要求移动测试模块，不豁免规则，后续最终矩阵改在全新`final-run2`目录。原目录
没有封存程序，不能把两次冻结身份混用。开发第四轮Hover Application1/0完成两轮
原TLS/Grant/FIFO/同child；其HWND组先丢前台再四项锁中毒，0/5不被前轮5/0覆盖。

最终`final-run2`冻结2,544输入，source SHA-256为
`c88852329144131146fc618eef3746ab8a15cb47873e647dfd16794ec849cb28`。fmt、全目标严格
Clippy/check、版本和3,696文件ratchet通过，零违规/警告/新例外。35份native变更，
其中12份新增，最大500有效行；Mount由497到500只增加Edge/调度分型等薄接线，新
hover责任在独立模块，不再向该owner累积实现。全部变更文本UTF-8无BOM。

六件封存程序无重编完成34运行门，32过、2失败。Hover capture-leave1/0、HWND6/0、
child4/0/1 ignored、wire1/0、queue5/0，预先安排的独立Application1/0完成两轮原
TLS/Grant/FIFO/同child；实际Hover产品worker1/0。原Pointer HWND9/0、child5/0/
1 ignored、model1/0、wire1/0，原Mount/Queue20/0，完整JS189/0/28 ignored。十四个
实际产品worker各1/0；build_resources2/0、exe_resources1/0、startup7/0、
javascript_worker2/0。集合重叠不相加，ignored不计通过。

完整Application JS13/12：Enter发送前真实前台断言失败，实际HWND(0x4c0716)不等
目标HWND(0x590d0c)，随后十一项PoisonError，包含Hover/Pointer未进入主体。未核该
HWND进程归属，不直接归因游戏；未重发Enter或重复完整组求绿。独立Hover成功使用
同封存字节且在完整组之前执行，不覆盖此失败。完整SurfaceWindow122/4的四项旧
keyboard tests分别为Tab后逻辑焦点None而非Some("a")，以及三项take_click为None；
这些是各自直接失败，不是该组锁中毒，现有日志不足以确定其根因，不能一并归为
前台竞争。expected foreground unwind probe为通过用例的预期panic，不计失败。

原256节点/16规则/250ms压力Engine、child、实际产品三门各1/0，仅证明本次固定
原规模门通过，不关闭历史压力根因或代替完整进程树资源/长稳验收。最终debug EXE
42,475,008字节，SHA-256为
`e484a50c5370f1ce562cfbe7c6296e6b1aa8bebde6da534aaac1152a451a4ada`，位于
`linshi/otter-native-v372-hover/final-run2/artifact/otter-native.exe`，publishable=false。
三次源码、六件built/sealed摘要、旧168份封存件、七文档/组织合同/精确diff/相关进程
由同目录closeout单独核验，下一轮保护174份。状态为partial-runtime-failure，goal
active；默认完整JS、真实股票/Loom、真实Lease撤销、OS权限隔离、完整N01–N12、
总资源长稳和最终用户包/人工验收仍未完成，不提交/推送/发布或扩大SDK许可。

## v371：Canvas 左键 Pointer 的原生纵向闭环

核验v370源码/七文档与162份封存程序后，仅分配一次v0.2.33.371。独立证据位于
`linshi/otter-native-v371-canvas-pointer`。本轮从真实HWND鼠标消息，经原Mount FIFO、
Runtime/child、JS PointerEvent到Canvas像素，再接原Application/TLS/Grant/batch；不
新增线程、mailbox、平行输入队列或远端authority，不把能绘制Canvas等同完整交互。

只接当前JS Canvas的左键捕获拖动：down/move/up/cancel、pointerId=1、mouse/primary、
客户区DIP及四modifier。普通按钮、文字选择和原生EDIT/select/slider仍由原owner负责。
move只合并同gesture/target/viewport epoch且未seal/未prepared的队尾；原32槽与sequence
为terminal同时预留空间。已派发手势硬取消复用最早waiting Pointer槽/序号，保留非
Pointer相对顺序、prepared证据并使旧fence失效；MAX序号和混用取消的epoch更新有回归。

捕获前登记provisional owner，SetCapture返回后复验owner/窗口/target/epoch，不能
释放同步重入建立的后来owner。Host暂借出时由原State::run归还接缝结算pending cancel。
仅JS背压、同Canvas回帧及同HWND下一Canvas手势保留原FIFO；真正失焦、隐藏、resize/DPI、
捕获丢失和策略关闭仍硬取消。正常ReleaseCapture豁免只针对WM_CAPTURECHANGED，不
屏蔽同栈隐藏/失焦。自有HWND同步重入、连续双手势与取消前FIFO快照分别保护该边界。

MouseEvent/PointerEvent沿原EventTarget私有状态、trusted、传播和finally；新family字段
使用自有属性及brand检查，native carrier保持null-prototype和捕获intrinsics。Pointer
preventDefault只改变取消状态，本轮无宿主默认动作，不抑制旧阈值click。up/microtask后
复验同target连接、disabled和自身/祖先computed display，再派原普通Event("click")。
touch/pen、hover/over/out/enter/leave、完整MouseEvent字段和JS Pointer Capture API未做。

首轮check的7项Rust接线错误及开发Pointer组13过/5失败/1 ignored均留证：两项直接失败
分别是strict模块改写不可写构造器和纯文档模型误用无raster后端的Canvas，另三项为共享
锁中毒；改夹具不弱化事件断言。后续最新源码与封存完整组实绩另记，不继承开发green。
用户允许短暂前台测试，仅定向自有Otter HWND，不操作游戏或发送全局输入，不重发失败
键盘事件。32MiB heap/256KiB stack/250ms、Job/startup、JSON清洗和emit预算均不放宽。

补充开发门build-dev6因测试试图读取私有active而失败；移除越界内部断言，以capture/
pending等外部状态验证，不公开生产字段。build-dev7通过；Pointer组11过/12失败/1 ignored，
Application两轮及七Queue/model/wire通过，但第二项HWND测试发送前丢前台，后十一项
锁中毒，新双手势/重入当时未进入体。失败不重发或覆盖。另补stale领取在同次prepare
先取消再读新队首，避免已生成cancel却因未启动child而等待外部唤醒；回归精确注入epoch，
不借WM_SIZE提前取消掩盖该路径。最终分开HWND/child/model/wire等进程门，故障不互污染。

默认完整JS、真实Lease撤销、OS文件/网络权限隔离、Loom/股票、完整N01–N12、总资源长稳
与最终用户包/人工验收仍开放；本地policy关闭不替代真实Lease撤销，Job不是OS权限沙箱。
未提交、推送、发布或扩大Otter隔离SDK许可，goal仍active。

最终`final-run`冻结2,532输入，source SHA-256为
`a50e18304acb9dc10e786d5d80f180c0ded32bd7bb15903f660c997cfdf7b9e1`。fmt、全目标严格
Clippy/check、版本和3,684文件ratchet通过，零违规/警告/新例外。37份native文件变更，
其中13份新增，最大497有效行；新增HWND Pointer tests为273行，单一责任是消息/FIFO/
像素/取消生命周期，重入子责任另置105行模块，其他新增均不超过214行，全部UTF-8无BOM。

六件封存程序无重编完成27运行门，26过、1失败。Pointer HWND9/0、child5/0/1 ignored、
model1/0、wire1/0、原Mount/Queue15/0，完整SurfaceWindow120/0、JS184/0/27 ignored通过；
集合重叠不相加，ignored不计通过。十三个实际产品worker门各1/0，含Pointer/Enter/
EventTarget/Canvas；build_resources2/0、exe_resources1/0、startup7/0、javascript_worker2/0。
原256节点/16规则/250ms压力Engine/child/产品各1/0，Engine Complete188.642ms，child/
产品Job峰值7,454,720/7,364,608字节；不是完整进程树资源/长稳验收，不关闭历史压力根因。

唯一失败门完整Application JS13/11：Enter在原Foreground获取前台断言失败，实际
HWND(0x406aa)不等于目标HWND(0x2b0dd4)，随后十项PoisonError，包括新增Pointer未进入体。
没有独立核该HWND进程归属，不直接归因游戏；未重发Enter，也未重复完整组求绿。为
使新增Pointer在最终字节上实际进入体，固定一次独立进程运行该唯一用例，1/0通过，
两轮TLS/Grant/FIFO/同child完成。该独立实绩单列，不替代完整Application失败；前台/
旧候选/keyup/form/startup未知根因仍开放，状态保持partial-runtime-failure。

最终debug EXE为42,429,440字节，SHA-256为
`fcee278114ad964417518cf093a3470c68c52e0cbae8046f7e9d39bf6e5a3479`，位于
`linshi/otter-native-v371-canvas-pointer/final-run/artifact/otter-native.exe`。收尾closeout
分别核三次源码、六件built/sealed、旧162份、七文档/组织合同/精确diff及相关进程；
下一轮保护168份。本轮Canvas左键子集已有最终产品/原生/独立Application证据，但完整
N01–N12、默认JS、真实Loom/股票/Lease撤销、OS隔离、长稳及最终用户包仍未验收。

## v370：原规模 CSS 压力的 cascade 分配热点

先核v369源码/七文档及156份封存程序，内部版本只分配一次v0.2.33.370；独立证据在
`linshi/otter-native-v370-css-cascade`。以v369同字节库再跑固定15样本，仍仅无规则3/3
完成，tag/class/tag+nth/class+nth各0/3；原256节点/16规则/250ms Engine压力0/1。
阶段诊断collect约225.672ms，其中computed约110.809ms；Engine另有约46ms mount，
阶段探针的独立2秒观察预算不是产品预算，也不把探针exit0视为吞吐通过。

只改原css_sheets owner：important、inline、三项specificity逐层比较，源序由原有
顺序遍历且相等时后来覆盖保证，不再为每条匹配声明构造六项rank数组。specificity
由原编译器固定生成三项，直接逐项比较，不压成可能碰撞的数值。每次computed仍新建
私有winners；同一属性的后来获胜声明只更新其已有自有数据字段，减少对象分配和
Map替换，不改变sheet/声明/节点或跨帧状态，不新增计算样式缓存。

首轮开发Engine和真实child压力各1/0，但固定矩阵仍有失败，不能用单项成功覆盖；
第二步原地winner更新后的开发Engine/child亦各1/0，Engine Complete约203.958ms。
所有固定样本与源码身份保留，短样本受机器负载影响，不归因全部计时变化或外推长稳。
测试将新rank和比较器精确替换回旧算法，与原CSS/Grid/spatial/padding/fonts/nth夹具
完整输出差分；另用32组独立Rust词典序oracle保护important/inline/匹配specificity/
跨sheet源序。补多节点/多属性、replaceSync/inline移除/adopt切换及同步几何重算，
并对新增winner字段安装抛错原型accessor，保护计算局部性和自有字段边界。

私有JSON encode/copy、getter/toJSON拒绝、payload标准JSON语义均不动；只读数据流
审查确认“私有生成”不等于“可直接stringify”，不借性能优化放松该边界。heap/stack/
250ms/Job/startup、默认JS能力和权限不变。最终冻结/封存实际产品与完整相邻门另记；
旧前台/候选/keyup/form/startup未知根因及完整N01–N12、真实Loom/股票、总资源长稳/
最终用户验收仍开放，不提交、推送、发布或扩大SDK隔离许可。

最终`final-run`冻结2,519输入，source SHA-256为
`fc64ceb41c207b1f17514602b46e9735ceedb2b969b5055dbaa9160336c52e1d`。fmt、全目标严格
Clippy/check、版本及3,671文件ratchet通过，零违规/警告/新例外。仅两份native文件
变化：css_sheets73→75、projection tests186→300有效行，均UTF-8无BOM；300行文件
仍只负责真实QuickJS投影/层叠的参考算法差分和oracle，不新增生产owner。

六件封存程序无重编完成24个实际运行门，23过、1失败；另三门为阶段/旧库对照/新库
固定矩阵诊断，不混入运行门成绩。原256节点/16规则/250ms的Engine、真实child及
外部SHA绑定实际产品三个压力门各1/0，Engine Complete约223.908ms，child与产品均
验证完整帧、同节点身份更新、同child和退出回收，Job峰值分别7,401,472/7,274,496字节。
这只是child Job数据，不是完整Otter/Loom进程树或总资源长期验收。

完整JS177/0/26 ignored、SurfaceWindow111/0、projection6/0/4 ignored、CSS18/0/5、
nth-child6/0/2、JSON2/0、Grid16/0、geometry29/0/1通过；八个其他摘要绑定产品worker
各1/0，build_resources2/0、exe_resources1/0、startup7/0、javascript_worker2/0通过。
集合有重叠不相加，ignored不算通过。窗口组本次包含后台Enter负向及两项guard回归。

唯一失败门Application JS13/10：Enter在原Foreground fixture获取前台失败，实际
HWND(0xa09fc)不等于目标HWND(0x160c96)，随后九项PoisonError；本次未独立核该HWND
进程归属，不直接归因游戏，不重发输入/重跑求绿。完整窗口组通过不覆盖该失败，也
不关闭旧前台、候选、keyup、form Apply和startup未知根因。

最终新库固定15样本全部完成；紧邻执行的v369旧封存库也15/15完成，必须同时报告。
旧→新中位数为无规则110.431→104.805ms、tag221.014→180.215ms、class237.424→
201.743ms、tag+nth174.299→174.720ms、class+nth186.222→182.270ms。简单tag/class
样本降低，但nth短样本变化小且不全向好；不能把早期失败→本次通过全部归因此次
优化，历史压力根因/稳定性仍未关闭。阶段诊断collect144.489ms仍仅是观察数据。

最终debug EXE为42,295,296字节，SHA-256为
`b3e913fcc8026d827d069fce181c1da112f8374daabc20c379ee03ef54096e20`，位于
`linshi/otter-native-v370-css-cascade/final-run/artifact/otter-native.exe`。收尾closeout分别
核三次源码、六件built/sealed、旧156份、七文档/组织合同/精确diff及相关进程，下一轮
保护162份。状态保持partial-runtime-failure、goal active；不是完整迁移或最终用户包。
下一项优先补真实JS Pointer输入纵向闭环，使已能绘制的Canvas获得实际拖动/取消交互，
而不是把再跑前台用例当作产品开发；默认完整JS与OS权限隔离仍需另行完成并验收。

## v369：前台测试guard的清理所有权

先核v368三次源码/七文档封账及150份封存程序，内部版本仅分配一次v0.2.33.369；
独立证据在`linshi/otter-native-v369-click-boundary`。原点击候选用例加有界状态观测后
一次1/0通过：mouse/keyboard两分支均在真实前台不是本窗口时形成候选，mouse取得原
capture、keyboard有本线程焦点且无modifier。该观测未复现v368丢失，不能把它归因为
“后台一定不能取得候选”。最终只保留分支/焦点/逻辑目标/capture/modifier/错误断言，
不保留逐步打印、不改变输入设置或原草稿/激活快照/后续输入/提交语义。

另发现并以三个自有屏外HWND证实独立机制：原Foreground guard退出时无条件激活旧窗口，
即使前台已切到第三个窗口，也覆盖后来选择；确定性RED在该精确断言0/1，而非前台获取
前提失败。首次RED支架因HWND私有性编译失败，迁回desktop owning test模块后再构建，
未公开HWND或扩展产品API；两份失败证据均保留。

只改测试guard：记录previous/target，当前仍是target时才归还；target已销毁且前台为
NULL时仍尝试恢复，其他窗口成为前台则不覆盖。保留原获取流程、AttachThreadInput
边界和真实前台断言，不加激活重试、等待、键盘重放或预算。新回归保护后来选择/普通
归还与精确panic载荷的unwind恢复；测试自身cleanup也只从本线程自有窗口切回anchor，
不覆盖外部新前台。系统前台不是原子可锁资源，此修复不保证前台独占或解决获取失败。

最终静态/封存完整组实绩和身份另记；v368候选丢失、前台获取失败及旧keyup、压力/form/
startup未知根因仍开放。无生产功能、权限、依赖或SDK扩张，默认JS、真实股票/Loom、
完整N01–N12、总资源/长稳及最终用户验收仍开放，不提交/推送或发布。

首轮final-run在封存前被严格Clippy的冗余WindowsAndMessaging import拒绝，保留原门与
aborted-before-sealing收据；只删该import、不加allow、不重分配版本，最终另用
`final-import-correction`。冻结2,519输入，source SHA-256为
`dd88d9abb1ee988b6960e4d3309ff82c77a6d8080c384cdb2e4634ef9c39ff63`。fmt、全目标严格
Clippy/check、版本与3,671文件ratchet通过，零违规/警告/新例外；四份native变更为
values173→192、fixture65→75、测试接线158→160和新增foreground tests67有效行，UTF-8无BOM。

六件封存程序无重编完成12门，10过、2失败，无deferred门。完整Application JS23/0，
含原Enter取消/default change/keyup、两轮TLS/Grant/FIFO/同child；values5/0，原候选
用例亦在完整窗口组通过。keyboard child3/0/1 ignored、队列两门各1/0、外部SHA绑定
产品keyboard worker1/0、build_resources2/0、exe_resources1/0、startup7/0和
javascript_worker2/0通过；ignored不计通过、重叠集合不相加，不据本次成功关闭旧根因。

专门foreground门1/1：后来选择/正常归还用例通过，unwind用例在初始获取anchor前台时
报原`focus fixture requires an interactive desktop`，真实前台HWND(0x0)，未到故意panic。
完整SurfaceWindow93/18：新增两项guard回归均通过，unwind确实到预期panic并核精确载荷
后恢复；但原focus stale/ABA用例获取前台失败，实际HWND(0x24c1082)不同于目标
HWND(0x4e214ea)，随后17项PoisonError。后台Enter负向仍取锁即失败、未进入测试体。
完整组的unwind成功不覆盖专门门的获取失败；不重跑失败门、不放松前提、预算或锁语义。

最终debug EXE为42,295,296字节，SHA-256为
`db2f88a2fd86c39fc48c03b0f39b5f78cae949998c8c992d77344fa870c3bfc5`，位于
`linshi/otter-native-v369-click-boundary/final-import-correction/artifact/otter-native.exe`。
收尾由该目录closeout分别核验三次源码、六件built/sealed、原150份、七文档/组织合同/
精确diff及相关进程，下一轮保护156份。保持partial-runtime-failure与goal active；
本轮只证实并修正测试清理覆盖新前台的机制，不声称前台获取或历史候选/keyup均已修复。
后续回到原规模CSS压力的生产执行路径，不把反复前台重测当作完整迁移开发的替代。

## v368：Enter Application 驱动的真实前台前提

先核v367三次源码/七文档封账和144份封存程序，内部版本仅分配一次v0.2.33.368；
独立证据在`linshi/otter-native-v368-enter-drain`。不以v366单次独立通过覆盖v366/v367
完整Application的keyup失败，也不先提高原16次poll、250ms、child/Job或startup预算。

首个只增强release错误的诊断在更早的canceled阶段失败，尚未到目标断言，原日志保留。
随后有界状态观测真实复现keyup失败：GetFocus仍为同一EDIT，控件enabled/visible及
local_edit均true，但GetForegroundWindow已不同于Surface parent；原消息入口因此按
既定前台门拒绝入队。后续poll没有在途JS或新候选，status仍为server:45:1。此机制由
本次观测和原入口共同证明，不能把未记录这些状态的所有历史同名失败都归为同一原因。

只修Application测试驱动：prepare/release每段各用已有测试专用Foreground guard建立并
恢复真实前台，在每次定向Enter发送前断言当前前台、EDIT焦点、可见与启用。TLS批次
期间不假设前台永久不变；不向产品增加激活/强制聚焦或重放键盘，未改消息入口安全门。
失败消息不重新发送，原取消/default change/keyup顺序、两轮TLS/Grant/nonce/双emit/
收敛/同child/原EDIT身份和原16次poll完成断言全部保留。诊断逐步打印不留在最终驱动，
失败仍报告实际焦点/前台/控件/JS状态，不把桌面前提失败伪装成队列超时。

原HWND键盘测试补一项：另一个自有屏外窗口处于前台时，直接发给后台EDIT的down/up
不得进入原FIFO；恢复前台/焦点后不能重放，随后新发的一次合法keyup只产生up且无emit。
测试复用原Session、SurfaceWindow、原键盘入口及Foreground guard，句柄/child按原owner
回收，不注入系统全局输入、不改系统设置。最终静态/封存完整Application与相邻运行门
成绩和身份另记；此前压力、form Apply和startup未知根因不因本切片通过而清零。

后台不入队/不重放的开发用例1/0通过；新Application驱动在发送前准确报真实前台已
丢失，仍0/1，不能宣称修复验证通过。对失败中同一外部HWND的只读核查确认其属于当前
运行的游戏。未结束、暂停或操作该进程；先暂停会争抢前台的验证并完成后台构建。
用户随后明确允许短暂占用前台，故最终恢复完整10门矩阵，不用background-only或跳过
Application、SurfaceWindow和startup。实际终态另记，不把授权或运行启动当作通过，
也不把当前游戏解释成所有历史失败的已证根因。

本轮没有产品功能、依赖、权限、SDK或资源预算扩张。默认完整JS、pointer、真实Loom/
股票、完整N01–N12、总资源/长稳及最终用户验收仍开放，不提交/推送或发布。

最终`final-run`冻结2,518个输入，source SHA-256为
`5d6cf347de731f05dff9c16b4c6c05ab5c7661a877296ac44c44ddff1df9bc38`。fmt、全目标严格
Clippy/check、版本与3,670文件ratchet通过，零违规/警告/新例外。仅两份native测试文件
变化：Application驱动112→134、真实HWND键盘测试282→329有效行；全部UTF-8无BOM。

用户恢复前台授权后，六件封存程序无重编完成原定10门，8过、2失败，无deferred门。
完整Application JS为13/10：Enter用例在原Foreground fixture报
`focus fixture requires an interactive desktop`，实际前台HWND(0x341008)不同于本次
Surface HWND(0x4971438)，随后九项PoisonError；本次未通过Enter完整路径，不能声称
旧keyup根因已修。未对该新HWND作进程归属证明，不沿用先前游戏HWND的归属结论。

完整SurfaceWindow为90/19：原surface_values_candidate_freezes_activation_value_for_mouse_and_keyboard
在`surface_edit_values_tests.rs:155`报`activation must queue a candidate`，未记录该次
mouse/keyboard分支及准入状态，根因未知；另一focus stale/ABA用例在原Foreground fixture
失败，实际前台HWND(0x0)，随后17项PoisonError。新增后台不入队/不重放用例亦在取锁时
失败、未进入测试体；开发单项1/0仍单列，不能计为最终完整窗口组的负向证明通过。

通过门为keyboard child3/0/1 ignored、默认槽替换/FIFO1/0、容量/期限/scope/cancel1/0、
外部SHA绑定实际产品keyboard worker1/0、build_resources2/0、exe_resources1/0、
startup7/0及javascript_worker2/0。ignored不计通过，重叠集合不相加；startup本次通过
不关闭历史原15秒期限未知根因。不重复失败门、不重发键盘消息或抬高预算求绿。

最终debug EXE为42,295,296字节，SHA-256为
`f804a610730bfecd1ee002090a179b8b155a88d4861e3d4d3806e02668a863c3`，位于
`linshi/otter-native-v368-enter-drain/final-run/artifact/otter-native.exe`。收尾由
`final-run/closeout.json`分别核验三次源码、六件built/sealed、原144份封存程序、七文档/
组织合同/精确diff及相关进程；下一轮保护150份。保持partial-runtime-failure及goal active，
不覆盖开发失败/v367压力与form失败，不作完整迁移、长稳、正式发行或最终用户包声明。

## v367：原规模CSS压力下的匹配、层叠与稀疏投影热点

先核v366最终源码/文档与138份封存件，内部版本仅分配一次v0.2.33.367，独立证据在
`linshi/otter-native-v367-css-projection`。不是缩小压力规模或只重跑求绿；固定15样本
沿生产Engine保留256节点、16规则和原250ms，对照无规则/tag/class/tag+nth/class+nth。
原无规则3/3完成、中位209.913ms，其余四类各0/3；因此失败不只由nth公式引起。

原projection对每个节点扫描全部支持定义，现仅枚举私有null-prototype计算样式中的
已有声明/继承值，仍走原上下文校验、别名和native转换，不调用公开keys或iterator。
首轮窄修无规则中位140.150ms，tag+nth3/3完成、中位226.353ms，但class+nth仍0/3，
真实child压力亦失败，全部留证，未把局部改善当压力门通过。

继续减少同一路径的重复工作：class token在编译时形成私有ASCII空白边界正则，标识符
已准入且无正则元字符，不调用可覆盖的RegExp构造/exec或逐字符拆词数组；constructed
sheet解析后只持一份有序声明，匹配时不重建Map iterator。inline仍读取原可变Map，
important/inline/specificity/源序、原子替换和原资源上限不变，不引入持久计算样式缓存。
同一computed调用现共享当前目标的私有state/ordinal，避免每条规则重复get/revision/
WeakMap；仅纯谓词使用，祖先仍各自核对，无插件回调或跨computed/帧的状态持有。

三处CSS缩减后压力仍有失败；一次真实Engine异常已到私有JSON的copy/encode，另批次
仍在prepare/validate，不能把采样栈或不稳定短时提速当成单一根因关闭。JSON owner现
通过捕获的native getOwnPropertyDescriptors按对象批量取自有描述符，描述符表去原型后
只复制可枚举字符串数据属性；getter/toJSON不执行，非数据属性以captured own拒绝。
clone/decode/payload语义、原深度/heap/stack和wire大小限制不改。新增模型检查普通数据、
空洞数组/非枚举/Symbol、污染与accessor拒绝；初次128深度的最终stringify触发原256KiB，
现只在测试副本暴露copy以单独测其128/129门，不宣称native stringify能处理任意128层。

阶段诊断仅在测试副本插入时钟，独立2秒上限用于观察超250ms的尾部，不替代生产Engine、
child或产品的原250ms完成门；其退出0不能算压力通过。固定矩阵的所有批次均保留，
含后续仍失败或短时回退的观测，不挑选最优样本、不外推普遍性能或资源长稳。
初次阶段探针的copy计数误落到prepare返回，原始累计数无效；computed/prepare/project/
encode独立计时不据此相加。随后在封存前审查又发现计时插入顺序问题，主动停止本轮
首次final-run，保留已完成门和aborted-before-sealing收据；只修测试计时并补copy≤encode
断言，最终使用全新final-timer-correction目录，不覆盖旧日志、不重分配版本。

真实QuickJS差分将三处策略替换回原算法，对比通用CSS/Grid/spatial/padding/fonts/nth
夹具的完整输出及上下文拒绝。首轮差分支架未绑定spatial夹具所需原生几何而失败，
补用原Geometry/NativeMeasure及原250ms后通过，不删该夹具或用假矩形绕过。压力另有
真实持久child/外部SHA绑定产品的完整256节点帧、同节点身份、重复投影和Job回收门。
child压力独立显式运行，原完成断言不变，避免其失败污染功能集合的共享锁；ignored本身
不算通过，Engine/child/产品压力三个结果分别报告。

本轮只优化已证明的CSS执行热点，不扩展CSS/DOM/API或权限，不提高heap/stack/250ms/
Job/startup。最终静态/封存实绩与身份另记；旧Application keyup、其他未知失败不因
吞吐改善清零。默认完整JS、pointer、真实Loom/股票、N01–N12、总资源/长稳和最终用户
验收仍开放，不提交/推送或发布，不扩大SDK隔离构建许可。

最终`final-timer-correction`冻结2,518个输入，source SHA-256为
`a0fd15d623b79d97abd75593f25cee991f6a58cb55d9eaad271ed2de30792370`。fmt、五份JS语法、
全目标严格Clippy/check、版本与3,670文件ratchet通过，零违规/警告/新例外。12个native
变更最大428有效行，六新增为budget47、phase91、pressure88、projection186、json52及
通用压力夹具5行；原selectors128→135、sheets70→73、projection112→111、json31→32，
其余两处仅测试模块接线，全部UTF-8无BOM。

六件程序无重编完成24个实际运行门：18通过、6失败；另外两个诊断均执行结束、exit0，
不计吞吐验收。通过集合为JSON2/0、projection4/0/4 ignored、nth-child6/0/2、CSS16/0/5、
完整SurfaceWindow108/0、Grid16/0、Geometry29/0/1；八个外部SHA绑定非压力产品worker
（Grid/spatial/padding/fonts/typography/nth-child/CSS/geometry）各1/0；build_resources2/0、
exe_resources1/0、javascript_worker2/0。集合有重叠，不能相加，ignored不计通过。

六个失败门分别保留：完整Application JS为13/10，再次在原Enter用例报
`keyup did not drain`（application_surface_javascript_keyboard_tests.rs:113），随后九项
PoisonError；本轮没有独立重试，v366单次复核通过不覆盖本次完整组再次失败。完整JS为
137/38/26 ignored，form_value_boundaries用例在Apply阶段engine_expired=true并报
`Invalid JavaScript Surface operation`，随后37项PoisonError；不把38项当38个独立根因，
也不据此归因为环境、历史问题或本轮某处改动。startup为4/3，hidden_window、transparent_pin
及wic_image三项分别触发原15秒child期限，根因未解释、不重试或提高期限。

其余三个失败门为原256节点/16规则/250ms压力的Engine、持久child及实际产品路径，
各0/1，原完成断言仍在，不能称性能问题已修。最终固定15样本仅无规则3/3完成、中位
162.083ms；tag/class/tag+nth/class+nth各0/3，中位256.079/257.553/254.524/253.156ms。
所有前后批次和回退保留，不仅挑最好结果。测试副本阶段诊断collect为275.2552ms，
computed139.0567、prepare70.4917、project219.8629、copy42.5855、encode54.8189ms；
copy≤encode及computed+prepare≤project+1计时自检通过。project/encode包含其子阶段，
不能相加重复计算；该独立2秒诊断仍超250ms且不含mount，不算Engine预算门通过。

最终debug EXE为42,295,296字节，SHA-256为
`3a8c29f3a198e30af383cca7f75cabe2209643365d211313e537d1cfb73f84f5`，位于
`linshi/otter-native-v367-css-projection/final-timer-correction/artifact/otter-native.exe`。
测试库为94,260,224字节，SHA-256为
`9f7f7d66f6540269f37bcdb8d5bb5edb0e7307ee4d1390715e2a448bcc8e8c70`；其余四件见同目录
artifacts.json。最终closeout核验三次源码、六件built/sealed、原138份封存件、七文档/
组织合同/精确native差异及相关进程；状态必须保留partial-runtime-failure、goal active。
不是完整迁移/默认JS/真实股票/总资源长稳或用户验收通过，也不是正式发行/最终用户包；
不提交或推送。下一轮需保护含本轮六件的144份封存程序，不覆盖开发失败或首轮中止证据。

## v366：真实四列网格需要的有界 CSS 位置匹配

先核v365源码/文档收据及126份封存程序，内部版本仅分配一次v0.2.33.366，证据独立
保存在`linshi/otter-native-v366-nth-child`。只读真实Surface确认四列单元使用
`:nth-child(4n)`；原matcher拒绝该选择器，新增Rust整数oracle用例在修改生产代码前
真实QuickJS中失败于compile/list/replaceSync，原RED日志保留，不修改或复制股票业务。

原CSS selector owner现支持author constructed sheet中的有界`:nth-child(An+B)`：
odd/even、带符号整数、n/+n/-n、带连续整数系数的n及可选带符号B；公式内仅ASCII，
外侧及B符号周围允许CSS空白，A/B绝对值不超过2147483647。按从1开始的所有元素
兄弟计数，文本不计，display:none仍计数；root无父时位置为1。specificity按一个
伪类计入原cascade，沿原祖先/直接父matcher，不新增DOM树、几何或样式owner。

私有WeakMap位置索引只借原dom mutation identity，每代每父扫描一次；换代丢弃索引，
移动、移除、重插及样式替换后不复用旧位置。原first-child共用该索引且保留其有父
前提。所有集合/字符串/正则操作使用既有捕获intrinsic，原512字符/8段/16测试和规则/
节点/heap/stack/250ms/Job/startup预算不变。Rust oracle覆盖128公式×12混合元素位置，
另验非法替换原子性、隐藏兄弟/祖先/specificity、同operation同步几何及公开原型污染；
通用夹具贯穿child、外部摘要绑定产品与真实HWND/Direct2D像素、重排/命中和关闭回收。

开发失败分别留证：HWND误用带固定计数器文本断言的旧helper造成一次直接失败和共享
锁中毒，只改用原通用指针消息helper；真实Engine随后精确记录同步几何中的Maximum
call stack size exceeded，现把位置索引计算放在调用谓词前，消除predicate→position→get
额外嵌套，不提高原256KiB。原256节点/16规则压力样本先在innerHTML拆移阶段超时，
改为逐节点append以单独诊断selector后，仍在collect/project触发原250ms；单段matcher
直接路径省去无用祖先工作数组，仍未让该样本达标。压力用例保留原规模、完成断言和
预算，仅与功能集分开，最终显式运行，不把ignored或期限拒绝算成吞吐通过。

首轮封存还发现旧CSS拒绝夹具仍把现已支持的`span:nth-child(2n)`列为非法，真实模型
报not rejected并使child共享锁中毒；随后仅改为仍不支持的of-selector反例，不改生产准入。
Application首轮13/10，其中Enter用例报keyup did not drain，后九项锁中毒；原因尚未证明，不能
按CSS夹具错误合并解释。固定一次同封存字节的独立Enter复核1/0通过，但不覆盖首轮
失败或宣称根因关闭；夹具修正后的定向集合不重复该完整Application组，原13/10单列。

仅扩展author CSS位置伪类，不支持of-selector/nth-last-child/nth-of-type、DOM query
位置选择器、hover/focus-visible或native pointer。真实股票仍有color-scheme、margin、
单边border、渐变/阴影、其他DOM/CSS/指针等缺口，默认完整JS、真实Loom/股票、N01–N12、
总资源/长稳/最终用户验收未放行；不把本切片当整个插件已经可运行，旧未知根因不清零。

最终夹具修正轮在`final-fixture-correction`冻结2,512个输入，source SHA-256为
`b26396bbccec4a6807fe89a743e634b9da0a473d31181ffddceea76418392ec9`。fmt、四份JS语法、
全目标严格Clippy/check、版本和3,664文件ratchet通过，零违规/警告/新例外。11个native
变更最大425有效行；六新增分别为Rust模型130、child/产品121、HWND75和三夹具21/20/18
行，selector为88→128，其余仅测试接线或一处旧反例，全部UTF-8无BOM。

六件程序无重编执行最终14个运行门，13通过、压力门失败：nth-child为6/0/2 ignored、
CSS12/0/1、完整SurfaceWindow108/0、完整JS169/0/22、Grid16/0、Geometry29/0/1；三项
外部SHA绑定产品worker各1/0，build_resources2/0、exe_resources1/0、startup7/0、
javascript_worker2/0。压力门显式以`--ignored --exact`运行，0/1、exit101，collect/project
中engine_expired=true；原256节点/16规则、250ms及完成断言不变，不算吞吐通过。
集合有重叠，不能相加；ignored不计通过。

首轮15门11通过、4失败的程序/收据保留，其中旧CSS反例造成的CSS/full JS失败仅由
修正轮证明已修；首轮Application13/10与固定一次独立Enter1/0仍分列，根因开放。
两轮source inputs仅旧CSS拒绝夹具一项不同，最终EXE字节与首轮不同，不能把首轮
Application整组或单次复核当成最终EXE整组验收，也不宣称字节可复现构建。

最终debug EXE为42,291,200字节，SHA-256为
`46de2d9a1375472810757a9cec2a934bac7370ec397d946bb8febdbea55a1abb`，位于
`linshi/otter-native-v366-nth-child/final-fixture-correction/artifact/otter-native.exe`。
最终closeout按三次源码、六件built/sealed、原132份封存件（旧126加首轮6）、七文档/
组织合同和相关进程分别核验，状态保持partial-runtime-failure，goal仍active。不是
正式发行/最终用户包，不提交或推送，不覆盖失败；下一轮需保护含最终六件的138份。

## v365：原输入值校验热点与不变预算下的余量恢复

先核v364三次源码/封账和120份封存程序，内部版本仅分配一次v0.2.33.365，证据在
`linshi/otter-native-v365-form-value-budget`。同一生产Engine固定21样本诊断表明，原
输入夹具mount中位215.077ms，48KiB ASCII单次写入225.033ms，线程CPU亦接近墙钟；
原form逐字符建字符串/数组再join，使合法边界或溢出拒绝距原250ms期限很近。

原form owner现先用捕获native exec扫描常见ASCII，无换行时返回已验证原字符串；
Unicode仍按原UTF-16/UTF-8规则完整校验，再按连续保留段去CR/LF。未增加JS/native
API、默认能力、权限、依赖或资源预算。首次保留段写法改善ASCII但使换行样本变慢，
该样本保留后修成native分段扫描；最终开发探针所有七类均较原中位改善，不挑绿样本。
开发语义门3/0/2 ignored通过：22个child边界/污染操作和Rust UTF-16 oracle的784组
差分。两轮失败来自夹具把Array.prototype数值槽污染留到QuickJS自身JSON.stringify；
真实QuickJS模型定位到native stringify的array setter，Node工厂未复现。现只在被测
值校验调用内注入并finally恢复该数值槽，其他公开方法污染仍持续；不改引擎或错误
脱敏，未宣称所有全局污染都可继续执行。最终静态、封存实绩与身份另记。

本轮是已实测输入校验热点的窄修，不把v364单次254.273ms失败改写为已解释或关闭
所有历史未知根因。完整键盘/pointer/表单、默认JS、真实股票/Loom、N01–N12、总资源/
长稳/最终用户验收仍开放，原SDK许可/heap/stack/250ms/Job/startup门禁不变。

最终final-run冻结2,506个输入，source SHA-256为
`aa1b025d7f08b72c7f90440598a4e3b73aae555015993df205f67d5e01125758`。
fmt、全目标严格Clippy/check、版本和3,658文件ratchet通过，零违规/警告或新例外。
七个native变更最大422有效行；四新增为Engine探针92、Rust oracle模型62、child/
产品测试57和通用边界夹具48行。原form61→76，另外仅测试模块接线+3/+6，均UTF-8无BOM。

六件程序无重编完成43个选定运行门，全部通过：值校验3/0/2 ignored、Enter11/0/1、
键盘49/0/1、完整窗口107/0、原生输入7/0/1、EDIT68/0、number37/0、Application JS23/0、
JS164/0/20、十四外部SHA绑定产品worker各1/0及startup7/0等。过滤集合不能相加；
ignored不计为通过，产品门和21样本Engine探针分别显式执行，探针不冒充预算/长稳门。

同一固定探针每类三样本，原开发基线→最终封存测试程序中位：原输入mount墙钟
215.077→11.420ms；ASCII边界写入225.033→4.5207ms、ASCII溢出拒绝221.6393→4.5899ms、
BMP边界75.7313→49.9587ms、astral边界85.5259→56.6630ms、混合换行165.7691→118.3108ms、
纯换行100.5301→7.8699ms。基线/首次回退写法/修正/封存四份摘要均保留；当前线程CPU
计时有系统粒度，零tick不表示零CPU。本机短样本不能外推普遍性能或历史超时全部关闭。

debug EXE为42,291,200字节，SHA-256为
`b2bc7a0862c58ea48fd592c353d7d8ecc08b9487f1a356057cb9f81173b3e087`，位于
`linshi/otter-native-v365-form-value-budget/final-run/artifact/otter-native.exe`。同目录closeout
核三次源码、六件built/sealed、旧120份、四组诊断/开发失败、七文档/组织合同和相关
进程；v364失败收据不覆盖、不提升为所有历史根因关闭，goal仍active，不提交/推送或发行。

## v364：真实 Enter 边沿与可取消的异步默认 change

继续 v363 未完成的键盘接缝，内部版本仅分配一次 v0.2.33.364，独立证据在
`linshi/otter-native-v364-native-enter`，精确继承并保护此前114份封存程序。文本 input
的非IME Enter keydown/keyup进入原Session/ticket/32槽FIFO与原Runtime单槽；UI线程不
等待child。KeyboardEvent复用原EventTarget的可信分派/取消状态，默认change只有在
keydown未取消、非repeat且原dirty代次尚未提交时才允许。默认动作替换原键盘槽/序号，
不append到keyup、后续文本或click fence之后；后续草稿与原远端emit/Grant/收据分离。

父侧在DOM帧呈现、ticket结算和focus/blur意图之后重验当前字段会话、原队列scope、
30秒期限、真实focus epoch/HWND、可见/前台/输入策略及租约。ABA/撤权/隐藏/换合同或
过期默认不重放；重复keydown仍交listener但不重复提交。键盘carrier不带文本值，
避免keyup回写旧快照覆盖listener改值；默认change使用已接纳DOM帧的值。IME、number、
声明式Enter和Tab仍沿原路径，未开放普通字符/Tab键盘取消、完整浏览器表单提交或指针。

原窗口Host临时取出/归还与stop等价迁入surface_window_state_lifetime.rs，保持消息
入口低于500有效行，不迁移业务或新建状态owner。首轮两项测试支架编译错误留证；
首轮实际运行3/6/1 ignored中一个直接失败源于新消息与通知通道编号冲突，另五个为
共享锁中毒。真实child取消独立通过、真实HWND独立失败定位冲突，改用已核唯一编号并
补消息身份回归。最终静态/封存运行与全部开发失败另记，不继承中间green。

后续失败也保留：Application第二轮夹具误保留上一轮keyup日志，现只在新一轮首次
input清理测试trace；隐藏窗口引发真实失焦change的断言原先过严，现按实际focus epoch
分辨已有失焦边沿，不删产品change。污染夹具有一处多余闭括号，随后错误地试图在已
封闭DOM实例上新增value描述符；用隔离生产工厂探针得到原TypeError后，改为污染原
HTMLInputElement.prototype，未放宽DOM封闭性或产品错误脱敏。另一轮编译发现测试
poll缺unsafe声明，均只修测试支架。默认值/keyup不回写与同dirty代次只提交一次另由
真实HWND测试保护；产品默认资格要求非空且匹配的当前焦点，不以None==None准入。

本轮只承诺文本input的受限Enter默认change取消；默认JS、真实股票/Loom、更多表单
属性/普通键盘/pointer、完整N01–N12、总资源/长稳/最终用户验收和旧未知根因仍开放。
不新装SDK、改证书/注册表、默认新DLL或扩大原heap/stack/250ms/Job/startup等预算。

final-run冻结2,502个输入，source SHA-256为
`dd980ec888213d6c604d66f5ecd287ebd66c5d43cbb0e2901046307a47a0ad9d`。
fmt、全目标严格Clippy/check、版本和3,654文件ratchet通过，零违规/警告或新例外。
38个native变更最大493有效行，九新增依次为Application键盘112、EDIT键盘92、队列
测试56、窗口键盘55、HWND测试282、窗口生命周期41、child/产品测试143、wire93和
通用夹具33行，全部UTF-8无BOM；原窗口入口499→465，原Mount483→493。

六件程序无重编运行41个选定门，首轮40通过、1失败，不记为全部通过。Enter11/0/1
ignored、键盘49/0/1、焦点9/0/1、较广焦点53/0/2、原生输入7/0/1、EDIT68/0、number37/0、
EventTarget10/0/1、Application JS23/0、JS161/0/18、十三外部SHA绑定产品worker各1/0、
startup7/0等通过；过滤集合有重叠，不能相加，未执行ignored不计通过。

失败门surface-window-sealed为104/3/0：旧文本输入夹具在mount返回处失败，诊断记录
Apply617μs→Microtasks254273μs，原250ms期限已过；timer/typography随后锁中毒。
这只定位到mount/apply阶段超时，不证明调度、算法或旧未知根因。预先固定六门方案
用同一封存字节复核：原失败用例独立三次、两项中毒用例各一次及完整窗口集合107/0，
均通过。保留首轮失败和window-deadline-plan/diagnostics，不改源码、预算或重试到绿；
closeout仍标partial-runtime-failure/selectedRuntimeGatesPassed=false，Enter聚焦门通过
单列，未知deadline根因保持开放，不用复核覆盖首轮收据或冒充稳定性验收。

debug EXE为42,291,200字节，SHA-256为
`89f12f7b9a1c1af6b0b08a6a8993358651f3d0384a50ad062cb8e55e123d98fc`，位于
`linshi/otter-native-v364-native-enter/final-run/artifact/otter-native.exe`。同目录closeout
绑定三次源码、六件built/sealed、旧114份、七文档/组织合同与相关进程；本切片不是
正式发行、默认完整JS、完整原生迁移或最终用户验收，goal继续active。

## v363：同一 child 回复中的有界焦点请求与真实失焦 change

先核 v362 最终封账/源码 owner，继续实读 EDIT Return/Tab、父窗口消息桥和输入 FIFO。
现有 Enter 先同步 COMMIT，不能事后补发 keydown 冒充 preventDefault；本轮先完成它
所需的安全焦点闭环，键盘默认动作延迟/取消留作下一独立闭环。内部版本仅分配一次
为v0.2.33.363，证据目录linshi/otter-native-v363-native-focus，保护旧108份封存程序。

新增私有焦点 factory/carrier 和 HWND 接线；每 operation 最多32请求，沿原回复/单槽
运行。父侧复用当前 Mount/语义句柄、focus epoch、前台/可见/输入策略门，DOM帧呈现与
本地 ticket 结算后才调用 SetFocus。Host 不跨系统切焦被借用，原 EN_KILLFOCUS 进入
同一 change/FIFO。应用输入策略窄提取并在回复前收紧，旧资格和 ABA 请求不重放。
不支持同步 activeElement/焦点事件/滚动/options，精确边界见原生JS合同v363。

开发聚焦最终9/0/1 ignored通过：真实child请求/污染/32项边界/生命周期回收、原生
HWND失焦change/重复blur不重复提交、ABA/后台/禁用/隐藏/关闭，以及Application两轮
真实input listener调用blur→原EN_KILLFOCUS→change listener→四emit/TLS/Grant/FIFO/
收据/同child更新，另两项在途ABA/忙碌策略丢弃均过。ignored产品门留待封存后显式验。

开发三轮运行失败均保留，分别是测试进程未获前台、错误假设change批次不能在同一
poll开始，以及用真实鼠标点击blur按钮时系统已先失焦且留下后缀点击。每轮1/8/1中
仅首项是直接失败，七项为共享锁中毒，不算八个独立根因。只修测试支架/接缝：临时
关联前台线程队列获取自有屏外窗口后立即解除，并unwind恢复前台；保产品前台检查，
不注入全局输入或改系统设置。通用夹具改为真实input listener主动blur，避免把按钮
默认失焦当成DOM方法证明；批次按实际pending状态观察，不放宽原权限、顺序或预算。

实现前同源真实 child 夹具0/1 RED；开发中错误的wire模块路径和dispose测试返回类型
分别造成编译失败，原日志保留。基线脚本首次漏读v350的非标准manifest文件名，只
沿已捕获源码摘要补齐旧108份，未覆盖原基线或旧产物。最终运行/冻结/封存成绩另记。
无新依赖/SDK/权限/预算、提交推送或发行；keydown/默认动作、pointer、更多表单、默认
JS、真实股票/Loom、完整N01–N12/总资源/长稳/最终用户验收与旧未知根因仍开放。

最终final-run冻结2,493个输入，source SHA-256为
`f6cff4cde34170b1621b56e82aaf984bdfc9d4fff79602d3db3a2cc82ac628c6`。
fmt、全目标严格Clippy/check、版本和3,645文件ratchet通过，零违规/警告或新例外。
24个native变更最大499有效行（原消息桥494→499，仅新增字段/初始化/接线），八新增
分别为JS焦点26、wire18、HWND owner102、child/产品测试135、HWND测试168、Application
helper93和通用夹具28/10行，全部UTF-8无BOM；原生状态/队列/授权owner不重复。

六件封存程序无重编运行37个选定门全部通过：原生焦点9/0/1 ignored、较广焦点52/0/2、
原生文本输入7/0/1、EDIT66/0、number37/0、EventTarget10/0/1、Application JS22/0、声明式
表单1/0、完整JS157/0/17。padding/图片/Grid/textarea/typography/font-family/文字/measure/
EDIT字体/Select/Canvas/geometry亦通过；timer、Canvas、resize、animation、geometry、CSS、
font-family、typography、padding、EventTarget、native-input和native-focus十二项外部SHA
绑定的实际产品worker门各1/0，资源2/0及1/0、startup7/0、worker入口2/0。集合重叠不
相加，未显式执行的ignored不算通过；这不是全库、真实股票/Loom或完整迁移验收。

最终debug EXE为42,211,328字节，SHA-256为
`41285065ba134cfe80dcf9eaa87f1a5ea25dca1ccdee485aee6c8c19acb15540`，位于
`linshi/otter-native-v363-native-focus/final-run/artifact/otter-native.exe`。同目录closeout
绑定三次源码、六件built/sealed、旧108份封存件、七文档/组织合同/diff及相关进程。
全部开发失败保留；只完成受限异步焦点切片，目标仍active，不清除旧Canvas/启动未知
根因，不开启默认JS、不提交推送、不安装SDK或扩大许可、不作为正式发行/最终手测包。

## v362：文本 input/change 贯穿原生 EDIT 与同一 JS child

先实读原 EDIT/Session/Mount/Queue/fence、AuthorizedWindow、Runtime/worker 和私有 DOM
owner，再补真实本地 carrier，不把合成分派或场景投影当作完整输入接入。文本 input
复用原 HWND/草稿/ticket/32 槽 FIFO；本地 ticket 在 DOM 帧接纳后结算，不伪造远端
action，也不等待 emit 的远端回执。click 前缀与后续输入共用原 fence，后台忙时允许
本地编辑但不越过单槽/授权/策略，IME 半成品不派发。精确子集见原生 JS 合同。

仅分配一次内部版本 v0.2.33.362，证据目录 linshi/otter-native-v362-native-input。
实现前通用夹具的真实 child mount 为0/1 RED，随后新增 carrier/HWND/Application/
产品回归；开发/静态/最终冻结封存结果在本节后续记录，未运行项目不继承 v361 绿灯。
新增 form 只管文本值 dirty/属性反射，TextInput 只传有界 target/phase/value，原授权、
Job、时间/heap/stack/布局预算不变。keydown/默认动作、focus/blur、pointer、默认JS、
真实股票/Loom、完整N01–N12/总资源/长稳与最终用户验收仍开放，旧未知故障不清零。
不提交推送、正式发行、系统安装 SDK、改注册表或扩大已批准的 SDK 使用范围。

开发中两次编译失败分别是共享准入方法的Rust可见性和测试helper的unsafe接线，均
只作窄修。首轮实际聚焦5/2/1 ignored：Application两轮EDIT/change/TLS/四emit与两项
HWND/IME/fence通过，新增污染夹具因value描述符不可重定义在mount失败，另1项锁
中毒。将新input反射属性设为可重定义，私有值读写仍不依赖公共描述符；同一夹具
再验污染而不删断言。原始失败和日志分别保留，不计作旧未知故障或独立两根因。

修正后开发聚焦7/0/1 ignored、完整JS153/0/16通过。新增反例还覆盖48KiB值/64Ki
原始UTF-16长度、NUL/畸形序列原子拒绝、dirty/default反射、disabled/脱树/异常和CPU
超限回收、无handler本地值及可覆盖value/String/WeakMap/Event/Promise污染。local
carrier准备后如提交结果不确定则关闭自有会话，不归队重放；无效click fence消费后
报错，不形成失效候选忙循环。最终严格静态、同源构建/封存与运行另记，不提前
放行默认JS、真实股票/Loom、完整迁移或最终用户验收，旧未知失败保持开放。

最终final-run冻结2,485个输入，source SHA-256为
`2e66985cc4f1c4346e02f5b06f8846f5a7cf594e45d4c733aef6f48377be947e`。
fmt、全目标严格Clippy/check、版本及3,637文件ratchet通过，零违规/警告或新例外。
36个native变更最大483有效行，八新增为form55、wire carrier57、Mount本地出口75、
child/产品测试175、HWND145、Application helper69及通用夹具35/14行，均UTF-8无BOM。

产品与五个测试程序封存后无重编运行33个选定门全部通过：输入7/0/1 ignored、
EDIT66/0、number37/0、事件10/0/1、Application19/0、完整JS153/0/16、padding43/0/1、
图片9/0、Grid16/0、textarea4/0、typography8/0/1、font-family11/0/1、文字33/0、measure131/0、
EDIT字体2/0、Select1/0、Canvas14/0/1、geometry29/0/1。timer/Canvas/resize/animation/
geometry/CSS/font-family/typography/padding/event-target/native-input十一项外部摘要绑定
实际产品worker门各1/0，资源2/0及1/0、startup7/0、worker入口2/0。集合重叠不相加，
只将显式执行的ignored产品门记为通过；不是全库、真实股票/Loom或完整迁移验收。

最终debug EXE为42,158,080字节，SHA-256为
`e3687b227145e6abc90ab515eb20500339333aa89699b0f85a379360d832a0b4`，位于
`linshi/otter-native-v362-native-input/final-run/artifact/otter-native.exe`。同目录closeout
绑定三次源码、六件built/sealed摘要、此前102份封存件、七文档/组织合同/diff及相关
进程；全部开发失败保留。只是本输入切片，目标active；keydown/默认动作/focus/blur/
pointer、默认JS、真实股票/Loom、完整N01–N12/总资源/长稳/最终用户验收仍开放。
没有新依赖/SDK、提交推送或正式发行，不将新切片成功解释成旧未知故障已修复。

## v361：真实 click 共用的 EventTarget 分派基础

沿真实消费者输入事件缺口推进，不继续只扩 CSS。新增文档内 Event/EventTarget，
将旧 click-only listener Map 与手写冒泡移入唯一私有事件 owner，Node/独立 target
及原宿主 click 共用身份、capture/target/bubble、快照移除、once/passive/取消/停止
与异常 finally 清理。公共 dispatchEvent 不受信任，原 native click 保持受信任，
时间戳复用原单调 clock；不复制树、布局、授权、远端动作或后台任务 owner。

内部版本仅分配一次为 v0.2.33.361，证据目录 linshi/otter-native-v361-event-target。
实现前四个真实 QuickJS 模型全部失败；新增 returnValue 污染反例亦在修复前失败，
改为共享私有取消逻辑后验证，不以覆盖公开 preventDefault 改写宿主语义。通用
夹具进入同 child 更新、原 native click、隐藏 HWND 及原 Application TLS/Grant/两条
emit FIFO/收据/同 child 收敛接缝；聚焦、最终静态/冻结/封存实绩另在下文记载。

精确预算/API/拒绝见原生 JS 合同。合成 keydown/change/pointer* 不等原生事件接入，
input 元素、真实键盘/pointer 数据和默认动作仍待后续完整 carrier/FIFO 实现；原32MiB/
256KiB/250ms/Job/read/layout/startup预算不增，默认JS不启用。完整N01–N12、真实
股票/Loom、总资源/长稳与最终用户验收仍开放，旧Canvas/启动未知故障不清零，无
新依赖/SDK、提交推送或正式发行。

开发首轮事件聚焦6/3/1 ignored：Application TLS/Grant/两次点击四个emit及模型五项
通过，但新HWND测试错误要求replace后旧UIA句柄仍有效，另两项SERIAL锁中毒。
原窗口合同明确替换撤销旧provider，只修测试为旧句柄失效/新句柄可读并保关闭撤销，
不修改生产UIA或放宽身份；失败日志独立保留，修正后成绩另记。

修正后的开发事件聚焦9/0/1 ignored通过，包含真实child四组模型/污染/两次native
click更新与回收、同步callback抛错/CPU超限回收、隐藏HWND像素/命中/新旧UIA撤销，
以及真实Application TLS/Grant/四个emit FIFO/收据/同child收敛。ignored产品门在
最终EXE封存后按外部SHA显式执行；此开发门不代替最终同源封存或完整迁移。

完整开发JS首轮116/32/15 ignored失败：原同步geometry child在click返回InvalidData，
另25项为同一SERIAL锁中毒、6项timer模型工厂接线遗漏，不记为32个独立根因。该次事件聚焦成功不
覆盖共享分派对同步几何链的回归；追加直接Engine的同源native click诊断，先取
原始异常再修机制，仍保256KiB/250ms，最终修正与封存成绩另记。

直接Engine使用同一geometry夹具确定复现Maximum call stack size exceeded，位置是
click callback中同步设置Grid声明的tokenize链，不是宿主几何/250ms期限失败。只将
私有click直接绑定dispatcher、把单调用invoke的阶段/回调循环合入同一owner，少两层
JS调用，不复制阶段状态或提高原栈；原同步读→变更→再读断言作为永久直接Engine
回归保留。此机制与本次扩展对应，不外推旧Canvas/其他启动未知失败。

压平两层后geometry直接Engine/child通过，事件聚焦10/0/1 ignored，但完整JS仍为
128/21/15：spatial child点击失败、14项锁中毒及6项timer模型接线遗漏。直接Engine同一spatial夹具再次
确定复现Maximum call stack size exceeded，位于同步几何投影的selector/words链，
因此不能把首个geometry修正外推为全部同步读链已修复，两个原始异常均保留。

最终native click在原Engine的JS job边界直接运行同一dispatcher，bootstrap显式
await再交付；公开dispatchEvent仍同步。只增加本operation内一个已被原Engine排尽
的JS job，不新增任务/线程/计时或跨operation队列，不重置原250ms总期限。私有
非thenable carrier避免读取Node.then，私有species/constructor及捕获then避免插件
Promise原型劫持；新增污染夹具同时验证原生点击、取消与同child更新。同步抛错
仍经await拒绝原apply，CPU超限仍由原中断/Job回收，不改错误或授权合同。

job边界修正后的事件聚焦10/0/1 ignored通过；完整JS为143/6/15，所有child及直接
geometry/空间点击已过，剩余六项均在timer纯模型初始化失败：该独立helper仍按旧
tree(fail)构造，遗漏新的事件工厂参数。逐项审计已将它们从前两轮错误总数中分离，
不是锁中毒或旧未知故障。只补helper的同一生产事件工厂参数，不放宽模型断言、
生产parser、事件语义或预算；最终完整回归与同源封存另记。

补齐独立timer模型工厂接线后，完整开发JS149/0/15 ignored通过；最新事件聚焦
10/0/1 ignored已包含直接Engine的geometry与spatial两条同步链，以及Promise原型
污染下真实child点击。生产源码停止扩展，最终严格静态/冻结/六件封存与无重编
矩阵见后续记录；历史失败不覆盖，不据此提前放行完整插件或最终用户验收。

最终final-run冻结2,477个输入，source SHA-256为
`a9e4fa0dfdd121864e65babb0c45256ba9ece637348769e169635f4ba1d8a892`。
fmt、全目标严格Clippy/check、版本及3,629文件ratchet通过，零违规/警告或新例外。
19个native变更最大386有效行，八新增分别为事件owner129、模型62、child/产品148、
HWND58及四个通用夹具49/40/44/35行，全部UTF-8无BOM；无新依赖或预算变化。

产品及五个测试程序封存后无重编运行29个选定门全部通过：事件10/0/1 ignored、
Application JavaScript18/0、完整JS149/0/15、padding43/0/1、图片9/0、Grid16/0、textarea4/0、
typography8/0/1、font-family11/0/1、文字33/0、measure131/0、EDIT字体2/0、Select1/0、
Canvas14/0/1、geometry29/0/1。timer/Canvas/resize/animation/geometry/CSS/font-family/
typography/padding/event-target十个外部摘要绑定的实际产品worker门各1/0，资源2/0及
1/0、startup7/0、worker入口2/0。集合重叠不相加；仅这些选定ignored产品门显式执行，
其他ignored不算通过，也不是全库或完整真实插件验收。

最终debug EXE为42,067,456字节，SHA-256为
`930ee1f69ee68b9a6b3d3d58f4dca7863c1cb8cc60d0264d18870f2d7cc9d32c`，位于
`linshi/otter-native-v361-event-target/final-run/artifact/otter-native.exe`。
同目录closeout绑定三次源码、六件built/sealed摘要、旧96份封存件、七文档/组织合同/
diff及零相关进程。全部开发失败留存：UIA夹具、两次同步栈机制和timer模型接线
分别说明，不覆盖或外推旧未知故障。仅本事件基础切片完成，目标active；native
input/键盘/pointer、默认JS、真实股票/Loom、完整N01–N12/总资源/长稳与最终用户
验收仍开放，无提交推送或正式发行，SDK许可范围不变。

## v360：四边 padding 的原生内容盒

本轮沿真实消费者的多值 padding 缺口推进完整布局链，不只接收声明字符串。原
surface_box 用 Insets 分离 left/top 起点与横纵总内缩，原单值声明式 padding 保留，
四边 longhand 按边覆盖；行列、Stack、Grid/空轨道 extent、文字反馈/基线、按钮默认值、
图片自然比例/contain/替代内容及 textarea 格式框均消费同一内容盒。绝对定位仍以
padding box 为包含块，overflow 仍裁到 padding edge，不误改成 content edge。

CSS values 一次验证 1–4 个 0/非负 px 值再展开四项，沿原 declarations/cascade 完成
priority、反射、移除与 mutation 失效；侧属性仍要求 border-box。JS 同步几何及
ResizeObserver 分别交付 left/top、两轴内容尺寸和独立边取整的设备像素。图片的
四个 content-edge 圆角独立减去相邻边，几何缓存仍归原 Clips/工厂生命周期。

内部版本仅分配一次为 v0.2.33.360，证据位于 linshi/otter-native-v360-padding。
选定验收包括真实 QuickJS、同 child 更新/重入/污染/ResizeObserver、真实 DirectWrite、
隐藏 HWND 像素/命中及 textarea 字段合同不变时的同 HWND/草稿保留，实际成绩与失败修正见下文。
CSS 百分比/相对 padding、margin/单边 border、input/number/select/slider 的新格式合同、
真实股票/Loom、完整 N01–N12、总资源/长稳及最终用户验收不在本切片完成声明内。
原预算和默认 JS 不变，无新依赖/SDK、提交推送或正式发布，旧未知失败不清除。

开发首轮37/6/1 ignored：新夹具不支持的cyan色名、Grid默认拉伸cell误断言、
textarea自身合同改变却要求保留旧HWND，以及对应模型/两项锁中毒连带失败；
未把它们归为旧未知故障。只修测试/夹具后40/3/1，剩余为把默认按钮圆角外点当成
命中及两项锁中毒。采样改到按钮直边，保留默认圆角与全部内容盒断言；textarea
改验父宽变化下四边百分比重算，字段自身合同保持不变，不放宽fingerprint。所有
失败收据保留，后续最终同源静态/封存成绩独立记载，不用隔离child成功覆盖失败。

修正后的完整 padding 聚焦门43/0/1 ignored通过，包含真实 child 三组更新/回收、
QuickJS 模型、原生布局/图片四角和隐藏 HWND；ignored 的摘要绑定产品门待最终
EXE封存后显式执行。该开发门不代替最终全目标静态、同源构建与封存运行。

首轮最终严格Clippy因新增双轴循环触发needless_range_loop而失败；仅将size与
padding.total改为zip迭代，不加allow或放宽门限。失败与首轮源码冻结保留，最终
修正链使用独立final-lint-correction目录，重新冻结/静态/构建，不复用失败前身份。

该链fmt、全目标严格Clippy/check、版本及3,621文件ratchet通过，无违规/警告或新
例外；38个native变更最大455有效行，九新增最大168行，全部UTF-8无BOM。证据
审计脚本首次因Windows ESM import未使用file URL而失败，仅修linshi脚本URL后用
独立v2门通过，未改产品源码；原失败日志保留，后续源码/程序身份仍分别核对。

final-lint-correction冻结2,469输入，source SHA-256为
`cfa62d4087cab3236a5642e1cf1b93a0f536f71151ab5f4d77ee03600de89ea0`，构建后相同。
六程序封存且无重编运行26门，25门通过、整轮partial：完整JS99/42/14 ignored，
旧CSS拒绝夹具仍要求拒绝新支持的双值padding，影响child和模型两项；空间模型
仍期待旧scalar投影，另39项SERIAL锁中毒连带。它们是本次功能扩展遗漏的旧测试
预期，不归入旧Canvas/启动未知根因，不削弱生产准入或输入合同。

padding43/0/1 ignored、图片9/0、Grid16/0、textarea4/0、字体8/0/1和11/0/1、文字
33/0、measure131/0、EDIT2/0、Select1/0、Canvas14/0/1、geometry29/0/1及九产品worker
各1/0、资源2/0和1/0、startup7/0、worker入口2/0。重叠集合不相加，精确成绩以
独立门为准。该轮debug产品42,059,264字节，SHA-256为
`1ccbba00a17a0bed3da0f6bc4a732af6e4bc114017eb9241a89feddaf019d4bc`；程序/失败日志
保留在该目录，不把25门成功当成完整矩阵通过；后续修正另用独立身份。

最终仅修两处测试输入：旧双值拒绝改为仍非法的五值padding，空间decimal模型改验
四个side均为0.25px且无旧scalar字段。开发完整JS141/0/14 ignored通过；不删原子
拒绝断言、不改生产解析或字段合同。final-fixture-correction重新冻结2,469输入，
source SHA-256为`9855a0958c95c32191ce0624a1524c5322c50856a99cc9e9dd32a1f0b3256b91`；
与上一封存轮逐项比较仅上述两个测试输入变化，生产输入不变，但不声称程序字节相同。

最终fmt、全目标严格Clippy/check、版本及3,621文件ratchet通过，无违规/警告或新
例外；40个native变更最大455有效行，九新增分别为round shape100、padding168、
图片像素46、JS模型68、child/产品118、HWND61及三夹具43/31/11行，全部UTF-8无BOM。
产品及五个测试程序构建后封存，无重编运行26门全部通过：padding43/0/1 ignored、
图片9/0、Grid16/0、textarea4/0、typography8/0/1、font-family11/0/1、文字33/0、
measure131/0、EDIT字体2/0、Select1/0、JavaScript141/0/14、Canvas14/0/1、geometry29/0/1；
timer/Canvas/resize/animation/geometry/CSS/font-family/typography/padding九个摘要绑定
产品worker各1/0，资源2/0和1/0、startup7/0、worker入口2/0。集合重叠不相加；九个
选定ignored产品门均以最终EXE/外部SHA显式执行，其他ignored项不算本轮通过。

最终debug产品42,059,264字节，SHA-256为
`2f5df413a045112d5ee1732672265a5846e3885e1c1aed29cbbcdd1fbdbb359b`，位于
`linshi/otter-native-v360-padding/final-fixture-correction/artifact/otter-native.exe`。
三次源码、六件built/sealed摘要、旧90份封存件、七文档/组织合同/diff与零相关
进程由最终目录closeout核验。首轮partial和全部失败保留；仅四边padding切片完成，
默认JS、真实股票/Loom、完整N01–N12/总资源/长稳和最终用户验收仍开放，目标active，
旧Canvas模型/Application及其他启动未知根因不清零，无提交推送或正式发布。

## v359：字体 shorthand、无单位行高与 normal 重置

继续真实消费者的 typography 合同，不修改 Loom 或忽略未知 CSS。新增通用 CSS fonts
owner，把原 family 准入迁入同一模块，统一 font-size/font-weight/line-height/font-family
及 font 展开；原 declarations/cascade/继承、文字 Engine/Styles、测量和呈现仍是唯一
各自状态/资源 owner。font 一次验证四个 longhand，再由原声明存储提交，保留 important、
单项显式覆盖、反射和整体移除；省略 weight/line-height 分别重置为400/normal。

无单位行高继承乘数，逐节点投影到原生 em resolver，子字号变化不误用父像素行高；
normal 显式清除祖先固定行距，由实际字体返回度量/基线。精确语法/预算见原生JS
合同，不支持的 italic/stretch/variant、字体下载及作者逐字形链不冒充实现。原时间/
heap/stack/Job/read/layout/startup 预算不增加，默认JS仍关闭。

内部版本仅分配一次为v0.2.33.359，证据目录linshi/otter-native-v359-font-typography。
新增四个真实QuickJS模型测试在实现前全部因原拒绝而失败，收据typography-before
保留；最终静态、child/同步几何/隐藏HWND/摘要绑定产品和同源成绩如下。
完整N01–N12、股票/Loom、长稳/总资源与最终用户验收仍开放；v357模型、v356
Application及其他旧Canvas/启动未知失败不由本轮成功清除，无提交推送或正式发布。

开发过程中首个child矩阵的污染夹具在update调用了不属本字体验证范围的
document.querySelector(':root')，已改为保留mount的root引用，继续验证同一字体API。
随后增强通用几何夹具，在font值转换中主动改字号并同步读真实几何；真实Engine
确定复现Maximum call stack size exceeded，阶段/原始测试异常单独留证。只把属性的
String转换移到公开Proxy setter尚不足，后续把仅有一个调用者的native投影层合入
prepare，减少冗余栈，保原顺序/状态owner/原子呈现和全部断言；未放宽256KiB或250ms。
聚焦8/0/1 ignored通过，包含直接Engine、真实child同文档更新/回收及隐藏HWND。
与本次新夹具直接对应的重入栈机制已修复，不外推旧Canvas/启动未知故障同根。

只读提取真实stock-monitor template中的22条字面font/line-height，均由生产值工厂
接受；消费者输入摘要与展开结果绑定consumer-fonts.json。不执行真实插件或其他
业务模块，不把纯Node值探针当作完整CSS、native或股票/Loom验收。

最终冻结2,460输入，source SHA-256为
`f8d63b4e5bc236abce4d0fa670034e16150091293395ec906287f712f3926d8f`。
fmt、全目标严格Clippy `-D warnings`、check、版本及3,612文件ratchet通过，无违规/
警告或新例外；16个native变更最大380有效行，七新增分别为CSS fonts64行、模型
88行、child/产品151行、HWND73行及三夹具39/38/11行，全部UTF-8无BOM。六程序
构建后封存，不再重编，运行前后built/sealed摘要一致。

21个选定封存运行门全部通过：typography8/0/1 ignored、font-family11/0/1、text32/0、
measure124/0、EDIT字体2/0、Select样式1/0、JavaScript136/0/13、Canvas14/0/1、geometry
29/0/1；timer/Canvas/resize/animation/geometry/CSS/font-family/typography八个摘要绑定
产品worker各1/0，资源2/0和1/0、startup7/0、worker私有入口2/0。集合重叠不相加，
ignored产品门已分别以最终EXE/外部SHA显式执行，不把未执行项算作通过。

最终debug产品42,053,120字节，SHA-256为
`39ce2d62bc6c572ca5c80e6d98f049ecea4bf40f4120e9aa05f2f2ef9b5ff5e2`，位于
`linshi/otter-native-v359-font-typography/artifact/otter-native.exe`。三次源码、六程序、
旧78份封存件、七文档/组织合同/diff与零相关残留由同目录closeout核验。新重入栈
失败及中间不足修正均保留，不解释其他旧未知失败；只完成本字体子集，默认JS、
完整迁移/真实股票Loom、总资源/长稳及最终用户验收仍开放，目标active，不发布。

## v358：CSS 字体列表到原生文字/字段的已安装字体选择

不继续用重复 Canvas 观察替代功能开发。本轮沿真实消费者已确认的 font-family
缺口，接通原 CSS cascade/继承→Scene fontFamily→原 Engine 按序查询已安装 family→
Styles→DirectWrite 测量/绘制及 EDIT/Select 字体。集合按需初始化，缓存最多64项、
无全局 owner/字体枚举/下载；独立 Rc 绑定场景，避免自引用和逗号列表由各后端猜测。
精确名称/长度/generic 子集见原生 JS 合同v358；不是逐字形作者字体 fallback 链，
也不修改 Canvas font、font shorthand、unitless line-height 或真实插件业务。

内部版本仅分配一次为v0.2.33.358，证据位于linshi/otter-native-v358-font-family。
模型/真实child/同步几何/隐藏HWND像素、原生字段及摘要绑定产品门分别验证，最终
冻结、静态、行数和封存运行成绩如下。原资源/时间/栈/启动预算不放宽，
无新SDK/依赖、Loom修改、默认JS放行、提交推送或正式发布。v357模型、v356 Canvas
Application及其他旧未知故障未结清；完整N01–N12、股票/Loom、长稳与最终手测开放。

首轮冻结2,453输入，source SHA-256为
`6a8eab7de639b37abae592a15319f0d1944d608a214b0cdea2806a255ae6c807`。
严格静态/3,605文件ratchet与六程序构建通过；封存无重编19门中18门通过。字体
11/0/1 ignored、文字/measure/原生字体、Canvas14/0/1、geometry29/0/1、七产品worker
和startup7/0通过。但JS90/39/12：旧CSS拒绝夹具仍要求拒绝现在有效的Arial，child
与直接模型各报错，后续37项为锁中毒连带。模型明确报not rejected，不归因旧
Canvas或系统负载。完整首轮closeout和六封存件保留，不用成功子集覆盖失败。

随后仅将该旧夹具的sheet/inline两处字体拒绝改为仍不支持的cursive，保留原子性
和既有其他拒绝断言。独立rejection-fixture-followup重新冻结/构建，但未封存，未重复
分配版本、未改生产输入；不能把该中间构建或仅改过期测试升级为生产故障修复。

该fixture-only构建完成严格静态/源码一致性检查，但尚未封存运行时，复查捕获另一
确定边界：255个NBSP加a共256 UTF-16 units、511 UTF-8 bytes，原values计数只补算
汉字，随后trim成a，错误接受。直接调用生产values factory的Node探针保留前后结果，
不冒充native验收。最终准入在计数前先验ASCII/汉字字符域，禁止用未计入的Unicode
空白绕过原始字节门；同一反例加入通用fonts夹具的实际模型/child/产品拒绝断言。
最终证据位于final-admission，重新冻结/构建/封存；相对首轮变化为该生产JS值模块、
新fonts夹具及旧CSS拒绝夹具三个输入，不再称最终生产输入不变。旧72封存件保留。

最终冻结2,453输入，source SHA-256为
`a075f11245bd45e6a70eb59cc8fac9cc173b9547faaf52a94da1d1cfce3601e8`。
fmt、全目标Clippy `-D warnings`、check、版本门及3,605文件ratchet通过，零违规/
警告、无新例外；18个native变更文件最大377有效行，六新增中字体选择模块136行、
字体选择测试158行、JS字体测试156行，全部UTF-8无BOM。产品及五个测试程序共六件
构建/封存后不重编，运行前后核对built/sealed摘要；中间fixture-only构建无封存件。

最终19个选定封存运行门全部通过：font-family11/0/1 ignored、text31/0、measure124/0、
EDIT字体2/0、Select样式1/0、JavaScript129/0/12、Canvas14/0/1、geometry29/0/1；
timer/Canvas/resize/animation/geometry/CSS/font-family七个产品worker各1/0，资源2/0和
1/0、startup7/0、worker私有入口2/0。集合重叠不相加；ignored产品门另以最终EXE
和外部SHA显式执行。NBSP反例纯值探针由accepted:true变为false，实际模型/child/
产品亦覆盖严格错误身份及原子拒绝，不以Node探针代替原生验收。

最终debug产品42,052,608字节，SHA-256为
`b4c3913c33ca725fb9a00290794ff1ea164b41881b6bbf28b635eeb78d2beb42`，路径为
`linshi/otter-native-v358-font-family/final-admission/artifact/otter-native.exe`。
三次源码一致性、六程序与旧72封存件、七文档、组织合同、diff及零相关残留由该目录
closeout核验。本切片实现并通过选定矩阵；首轮失败收据仍保留，v357模型、v356
Application及其他旧Canvas/启动未知根因仍开放，不据此宣称已修复或默认JS可用。
完整N01–N12、真实股票/Loom、整进程树总资源、长稳和最终用户验收仍未完成，目标
保持active；无新版本分配、提交推送、正式发布或中间手测要求。

## v357：跨 child 的固定失败阶段诊断

继续定位v356 Canvas Application初始回复的Execution(InvalidData)。原worker将
Engine错误折叠为output:null，且stderr不继承到父侧；本轮只在cfg(test)私有回复
增加可选固定Report，实际产品仍使用原sequence/output协议。Engine和Canvas原
owner记录阶段、单调微秒及期限/粘性失败状态，各最多64项；不读取JS异常属性、
脚本、路径、节点、快照或凭据。父侧仅在原序号/停止检查通过后的失败回复打印，
有界/单调校验和写入错误隔离不改变原InvalidData、child终态或容量回收。

内部版本仅分配一次为v0.2.33.357，证据位于linshi/otter-native-v357-js-failure-phases。
诊断覆盖异常脱敏、原250ms中断、真实PNG阶段与Canvas粘性失败；静态、真实child/
Application和最终同源封存成绩另记，不预称通过。原32MiB/256KiB/250ms、几何/
父侧/Job/startup门不增，无生产行为、SDK/依赖或默认能力变化。新观测即使成功也
不清除v356及旧未知失败，完整N01–N12、真实股票/Loom、长稳和最终手测仍开放。

首轮冻结2,447输入，source SHA-256为
`17669702ec11f38903a362171a3cd434dac4daca951f4970b42e261bd8f9cf4f`。
严格静态、3,599文件ratchet与六程序构建通过，封存无重编20门中19门通过。
JS集合124/1/11 ignored，首个且唯一失败为直接Engine的
`javascript_canvas_model_attribute_reset_detach_and_reconnect_keep_identity`初始mount
返回原InvalidData；不是锁中毒连带，底层根因未定位。Canvas集合14/0/1、geometry
29/0/1、六个产品worker各1/0、startup7/0均通过；不把成功集合覆盖该失败，也不
把它归因为v356 Application故障。首轮三次源码/六封存件和失败绑定first-run-receipt。

随后只在原Canvas模型mount helper的apply错误返回处复用固定Report，未创建新
异常/资源owner，也不冒称覆盖Engine::new尚未创建的状态。独立engine-phase-followup
重新冻结/构建/封存；与首轮2,447输入逐项对照，仅该测试文件变化，生产输入不变。
最终source SHA-256为
`8397fac8416527a0c8e9dbcd348d9261dcda6c86df72eebbf4738e79bad002a6`。
九个native变更最大374有效行，两新增为188/107行，UTF-8无BOM，无新行数例外。

补接后全目标严格静态/3,599文件ratchet和六程序构建再次通过；封存无重编16个
选定门全部通过。预先固定的三个独立完整JS观察各125/0/11 ignored，diagnostic7/0、
Canvas14/0/1、geometry29/0/1、六产品worker各1/0、资源2/0与1/0、worker私有入口
2/0、startup7/0。集合重叠不相加，没有重跑的首轮子集不冒充本次源码的独立成绩。
未再复现只表示新观察未捕获：首轮模型失败、v356 Application及其他旧未知根因
均未结清，没有生产Canvas修复，整轮仍partial，完整迁移目标保持active。

最终debug产品41,990,144字节，SHA-256为
`e999e84a70beb876d05f5e41c506a923ededd4a7848a2aed3fea135fa682efdb`。
两轮同内部版本不同字节各绑定自身摘要，不冒充可复现/正式构建。最终三次源码、
六程序、旧60封存件（含本轮首轮六份）、七文档与零相关残留由补接目录closeout核验；
实际产品不含诊断日志前缀/测试敏感标记，产品Response不增加字段。

另只读核对真实stock-monitor：template/main的color-scheme等CSS声明及lifecycle的
input、keydown/change/pointer listeners与现有准入仍有明确源码合同缺口，逐输入
摘要保留于consumer-boundary；未执行真实插件、修改Loom或以Canvas子集替代业务验收。

## v356：CSS 输入转换重入的真实 Engine 修复

将v355失败接到真实Engine后，原cssText转换内的同步几何读也在原256KiB栈下
溢出；不再将其视为纯模型问题。最小修复把捕获的String转换移到公开cssText/
Element.style setter、进入私有声明parser之前。原重入模型与真实Engine共12项
随后通过；扩展真实回归又复现sheet转换内嵌套replaceSync的同类溢出，因此sheet
公开入口同样先转换再进入私有规则parser，接收者验证仍先于用户转换。

单移sheet外层转换后，真实Engine/child仍在嵌套规则声明的color校验栈溢出，
26/3/1 ignored中的一个失败为SERIAL中毒连带，不计独立根因。继续将仅由expand
调用的validate合入原expand，color直接用同一捕获exec、不再经过test包装；保留
grid/spatial先分派、原始长度校验、auto数组结果及所有值准入/规范化顺序。

同一组通用夹具覆盖cssText、整体style、sheet成功替换及转换主动变更后抛错/
解析拒绝；主动mutation保留，失败外层替换不部分提交。拒绝夹具要求转换确实
完成重入几何检查并返回，且捕获原固定解析错误，不能把任意异常当作拒绝成功。
真实Engine、持久child与摘要绑定产品门分层验证，最终冻结/封存成绩另记。

内部版本仅分配一次为v0.2.33.356，证据位于linshi/otter-native-v356-reentrancy-stack。
原32MiB/256KiB/250ms、reads/layout、父侧/Job/startup预算和mutation cache合同不变；
不承诺任意深度重入，不放行默认JS、完整股票/Loom、N01–N12、长稳或最终手测。
旧失败原件保留，其他未知根因不清零，无依赖/SDK范围、提交推送、组包或发布变化。

压平后完整geometry源码门29 passed/0 failed/1 ignored，含七个真实child模式、
真实Engine、投影/预算模型、HWND与Application TLS/FIFO/viewport；ignored产品门
仍待后续封存EXE显式运行，不以此预称最终产品或全部运行门通过。

最终冻结2,445输入，source SHA-256为
`e386caaeb3b5a9fcafcf2b7e8094d65feacd35bad11fbc9ce02ef84d06026692`。
fmt、全目标严格Clippy/check、版本、3,597文件ratchet及产品/六测试程序构建通过。
七个新增/修改native文件最大214有效行，两份新夹具14/34行，无新例外、UTF-8无BOM；
Engine原heap/stack/时间、Rust几何、startup/process、依赖和原投影模型字节未改。

封存后无重编19门中18门通过：geometry29/0/1 ignored、JS118/0/11、timer13/0/1、
resize13/0/1、animation9/0/1、image/measure145/0/1、grid16/0、scene11/0；六个摘要
绑定产品worker各1/0，geometry产品门包含七个重入模式，资源2/0与1/0、私有worker
2/0、startup7/0。集合重叠不相加；此前重入失败已由真实Engine/child/产品回归覆盖。

Canvas集合10/4/1：Application初始等待回复时报Execution(InvalidData)，随后三个
SERIAL锁中毒为连带，底层根因未定位，整轮仍partial。固定旧v355→新v356→新→旧
四次独立封存Application Canvas对照各1/0，只表明该隔离矩阵未复现；不证明改动
无回归、不清除首轮失败，也不以产品Canvas单项成功替代Application验收。

最终debug产品41,990,144字节，SHA-256为
`d8de08e8c098071df7e83ca004c785966adf3b29f06468859e7cfe706bd3fcbe`。
三次源码、六程序、旧48封存件、七文档与最终进程状态另由closeout.json核验。
默认JS、完整迁移/股票/Loom、原启动及其他未知失败、长稳与最终手测继续开放。

## v355：同 operation 几何投影复用与 mutation 失效

沿v354真实geometry失败继续处理已确认的冗余：Rust已复用相同source的原生布局，
但每个JS box getter仍重算完整CSS/DOM Scene并编码。geometry现仅缓存同operation、
同私有mutation identity token的encoded source；tree、文字、属性、inline CSS、
constructed/adopted sheets及Canvas尺寸在完整准入后通知失效。begin/dispose释放缓存，
project/encode前后复核token；连接且可度量节点的每次box getter仍调用原生read，
不缓存最终矩形，不绕过原256reads/8layouts/250ms/viewport或粘性失败准入。

内部版本只分配一次为v0.2.33.355，证据位于linshi/otter-native-v355-geometry-projection。
同生产JS工厂的fake-box探针为21operations×8getters，projection从168降至21，
nativeReads仍168；不是原生量测/产品性能或超时根因证明。模型、真实child/HWND/
Application、冻结/封存门另记，不预称通过。旧启动/几何及其他未知失败保留，默认
JS、完整股票/Loom、N01–N12、长稳和最终用户手测不放行；无依赖/SDK范围变化。

最终冻结2,443个输入，source SHA-256为
`f7f017006a37649b915bac4a4be2ab329276c0354b7c4ba381107811e9202a6c`。
fmt、全目标严格Clippy/check、版本、3,595文件ratchet和产品/六测试程序构建通过。
八个新增/修改native文件最大289有效行，新测试模块194行，无新增例外，UTF-8无BOM；
原startup/process、依赖清单/锁文件和Rust几何预算原字节保留。

封存后无重编18门中16门通过：timer13/0/1 ignored、Canvas14/0/1、resize13/0/1、
animation9/0/1、image/measure145/0/1、grid16/0、scene11/0；五个实际产品worker各1/0，
资源2/0与1/0、私有worker2/0、startup7/0。geometry25/1/1及JS114/1/11因同一新增
输入转换重入模型在原256KiB下`Maximum call stack size exceeded`失败；不是两个
独立根因，仍未完成模型额外栈与实际生产重入边界的归因，整轮保持partial。

原真实HWND mount/click/resize-update、持久child/viewport/DPI/回收和Application
TLS/FIFO/held-emit/stale-viewport均在本轮geometry集合通过；原生reads/layout/期限
粘性拒绝门也通过。真实Engine本轮八次read的固定轨迹为首读15,733us、末次lookup
21,510us；只是当前样本，不作为跨版本性能保证或v354全部失败的根因证明。startup
本轮通过不清除旧未知超时。开发模型已去除额外eval/async/计数包装栈，修正不属于
合同的padding多值与classList.value赋值夹具，但未放宽预算、删重入场景或改绿旧日志。

最终debug产品41,990,144字节，SHA-256为
`68ab5ac3ae9d75ad6f12bc14c8e6fb7e4e515311028aed962f13ebe13f8a0fb5`。
三次源码、六程序构建/封存、旧42封存件、文档与最终进程查询绑定本轮closeout.json；
失败/封存件不覆盖，下一步继续定位输入转换重入的预算边界。无提交推送、组包或发布。

## v354：通用 JavaScript timers 沿原单槽与应用 deadline 接线

继续N08真实消费者链，补函数形式timeout/interval、交叉取消、参数、嵌套clamp与
同源单调计时，不翻译股票业务或修改Loom。timer工厂只拥有有界登记/到期快照，
Engine仍逐callback排尽microtask并守原总期限；私有协议、Session、Mounted、Runtime
和AuthorizedWindow贯穿同一child/Grant/单槽/viewport与emit FIFO，非新的调度owner。
timer-only task不提前rAF，也不把既有rAF期限不断后推；背压/模态/无owner不贡献
定时唤醒。精确支持、拒绝及生命周期见 [timers合同](SURFACE_JAVASCRIPT_NATIVE.md#v354-有界-javascript-timers)。

内部版本仅分配一次为v0.2.33.354，证据独立位于linshi/otter-native-v354-timers。
源码门、真实child/HWND/Application TLS与摘要绑定产品门分开记录；本段是开发
范围，不预称验证通过。旧v353/v352/v350/v348启动及其他未知根因继续保持开放，
默认JS、完整股票/Loom、N01–N12、长稳和最终手测不升级；不提交推送或发布。

第二轮final-run-v2仅修正counter过期的无timer断言，增加四API类型检查并保留
无网络/进程等断言；逐输入比较确认与首轮只有该测试夹具一项不同，生产输入不变。
重新冻结2,442输入，source SHA-256为
`e224c12c4371b3a4b7e508afa18ea8e6e39ea1e5108c8e62ac2eb59a65cdeb18`。
严格全目标静态、版本、3,594文件ratchet和产品/六测试程序构建通过，24个native
新增/修改文件最大382有效行，七新增最大139，无新增行数例外，UTF-8无BOM。

第二轮封存后无重编执行18门，16门通过：timer13/0/1 ignored、JS104/0/11、Canvas
14/0/1、resize13/0/1、animation9/0/1、image/measure145/0/1、grid16/0、scene11/0；
集合重叠不相加。timer/Canvas/resize/animation/geometry五个摘要绑定实际产品worker
各1/0，资源2/0与1/0、worker私有入口2/0。geometry集合10/5/1：首个真实HWND在
click时失败，后四项为SERIAL锁中毒连带；未定位该click失败根因。startup6/1，
钉图smoke超原15秒（15.0897918s）。保持partial，不用首轮startup7/0或单项产品
成功覆盖第二轮失败，未知几何/启动根因和完整迁移/手测门继续开放。

最终debug产品41,986,048字节，SHA-256为
`72b89349ef3bb7a136ec377f5ecc2db067f2547a1d0e41c823c7ece515f6bdba`。
源码冻结/构建后/运行后、六程序构建件/封存件、旧36封存件及最终进程查询分别
绑定final-run-v2/closeout.json；两轮同版本不同字节各按自身摘要保留，不冒充可复现
构建。首轮partial收据/诊断也保留，无提交推送、组包/发布或SDK授权范围变化。

首轮严格静态/3,594文件ratchet和产品/测试构建通过，2,442输入冻结/构建后相同。
23个native新增/修改文件最大382有效行，七新增最大139，无新增行数例外。封存18门
中16门通过：timers13/0/1 ignored，五个摘要绑定产品worker各1/0，startup7/0。
geometry为13/2/1，JS为95/9/11；JS首个counter mount失败，后八项为SERIAL锁中毒
连带，不计独立根因。geometry一次原生轨迹在第三次读取已到298317us，超过原
250ms，底层耗时根因未定位，不能归因于timer或系统负载。原失败矩阵/封存件保留。

固定AB/BA隔离对照中，旧v353两次counter abort场景通过，新v354两次快速失败；
源码确认旧counter夹具仍断言setTimeout未定义，与本轮明确新增能力冲突。将修正
该过期断言并保留无fetch/process/require/WebSocket等安全断言；后续使用独立
final-run-v2重新冻结/构建/封存，不改写首轮。geometry隔离两代各两次通过不清除
首轮失败或未知根因；startup本轮通过也不结清旧启动问题。

## v353：启动失败的可配对计时与无副作用诊断

沿v352封存startup失败继续定位，不延长原child15秒、删场景或清除旧失败。现有
smoke_timing增加进程内配对ID和单调时间戳，跨线程阶段不再按隐含调用栈推断；
诊断写入错误被忽略，不能把原成功/失败变成BrokenPipe panic。只记录固定阶段、
时间、状态和ID，不记录路径、像素、脚本、凭据或原错误详情；默认计时仍关闭。

document-window增加图标/宿主创建边界，Canvas/render-surface增加factory及真实
呈现目标边界，Window RAII增加销毁边界；并覆盖原启动smoke余下阶段。为不继续
扩大desktop/mod.rs，将原隐藏启动验收序列迁入startup_smoke.rs；窗口/COM所有权、
操作/断言/标记顺序、参数和最后释放不变，不新增第二套业务实现或测试开关。

先在linshi独立核对Win32 Sleep(1)与当前Rust thread::sleep(1ms)：两者结果不同，
不能将Win32约15ms的样本归因于本工程的Rust等待，也未据此修改等待循环。初步
诊断产品的固定无图/图片/钉图三场景均超15秒，保留profile原日志与程序，不冒充
最终构建门。已完成阶段显示factory不足1ms、耗时分布涉及控件创建/窗口销毁及
恢复/回收；未完成阶段不是根因。只读采样也观察到同时存在用户图形负载及另一
仓库编译，未中止或修改它们，不能仅凭并存断言因果。最终源码/封存和验证另记。

内部版本仅分配一次为v0.2.33.353，证据位于linshi/otter-native-v353-startup-waits。
旧v352/v350/v348失败保持开放；默认JS、完整N01–N12、最终手测与发布不放行。

最终冻结2,435个构建输入，source SHA-256为
`2cef44e0fddf1f797d13142a994706bfa31479ff352f3fa98bdcd0f3fb7cad55`。
fmt、全目标严格Clippy/check、版本、3,587文件ratchet和产品/六测试程序构建通过。
初次Clippy指出迁出后多余的document引用，已移除而未加allow，原失败日志保留。
七个新增/修改Rust文件均不超过415有效行：startup_smoke新增104，smoke_timing
为168；desktop/mod.rs从498降至415。源码审计确认只有这七文件和内部版本变化，
原startup测试/15秒实现、依赖清单/锁文件及原smoke标记顺序均不变，UTF-8无BOM。

所有Cargo结束后封存六程序，无重编运行：计时5/0（含断管道及并发ID）、文件
计时4/0、生命周期1/0、图标2/0、默认设备5/0、build_resources2/0、exe_resources1/0、
javascript_worker2/0。设备首次用物理文件名筛选得到0项，不计通过；按实际模块
composition::devices重跑才得到5/0，两个零项诊断及原日志均保留。九个有实际用例
的选定门中八门通过，startup为6/1：钉图smoke超原15秒（15.0377259s），最后输出
到marquee完成，仍不能确定耗时根因。旧失败与整轮partial状态不因上述回归通过清除。

最终debug产品41,959,936字节，SHA-256为
`c5933ba689ad88f271a144f74a19b7a0c0b89f4f4b2744830f5026b398eca4ab`。
构建前/构建后/运行后源码、六程序构建件/封存件、旧24份封存件及初步诊断程序
和日志分别核验；最终文档、组织合同、scoped diff与相关进程查询绑定本轮closeout。
这是诊断缺陷修复与启动序列保持性验证，不是启动超时根因修复、全库回归、真实
股票/Loom或N01–N12验收；没有提交推送或发布，SDK授权范围不变。

## v352：Canvas2D 从脚本到真实原生像素

继续N08同一真实消费者链：只读核对stock-monitor的Canvas使用点后接入通用位图、
路径和文字绘制，不修改Loom或复制其业务。JS canvas/context/path分别拥有identity/
尺寸重置、绘图状态/保存栈和有界路径；child raster/commands/draw/path分别拥有
WIC背板生命周期、提交准入、D2D/DirectWrite执行和几何。私有wire负责尺寸/PNG/
当前Scene身份复核，UI仍沿原Bitmap/Presentation及授权/viewport/代次/FIFO门提交。
同步几何从同一Canvas自然尺寸求值，CSS缩放使用fill而非普通图片的contain。

支持与明确拒绝、零尺寸、容量和原预算见 [Canvas2D](SURFACE_JAVASCRIPT_NATIVE.md#v352-原生-canvas2d-绘制链)。
本轮内部版本只分配一次为v0.2.33.352，证据独立放在
linshi/otter-native-v352-canvas2d，不覆盖旧收据或封存件。开发、最终静态/冻结、
实际产品及运行门分别记录，未验证项不升级。默认JS、真实股票/Loom完整链、
原v350 child/geometry/TLS/UIA与v348启动未知根因、N01–N12和最终手测仍开放。

开发期修复了rect解构无迭代器私有数组导致的挂载失败，保留无原型数组隔离；HWND
测试改为精确使用真实客户区宽度，未放宽布局或像素断言。补充伪造receiver先验检查、
8身份/总背板/路径/命令限额、同尺寸属性重置/移除/重接、私有帧身份与重复/缺失拒绝、
raster失败粘性和child失败回收测试。新增夹具的括号及update误取root问题仅修夹具，
原始编译/运行失败日志均保留，不冒充产品修复或独立SERIAL根因。

首轮final-run严格静态/3,586文件ratchet和产品/测试构建通过，2,434个输入冻结与
构建后相同；封存16门中15门通过，JS集合64/29/10 ignored保持失败。真实child和
模型共同定位旧document-rejection夹具仍期待canvas非法（not atomic）；其余27项
为共享锁PoisonError连带，不修改锁恢复或删除断言。该首轮封存/摘要及partial
closeout保留，后续修正必须使用独立final-run-v2，不追溯把首轮改为通过。

第二轮final-run-v2仅将旧拒绝夹具的canvas负例换成仍不支持的video，并增加Canvas
markup正向像素/尺寸反射/sibling保持测试；逐输入比较确认生产源码没有变化，原
拒绝与原子性断言保留。重新冻结2,434个输入，source SHA-256为
`d5ccca1005c629bcba9aca6578e65b09be28d7396081d2954decad8ac2491173`。
JS syntax、fmt、全目标严格Clippy/check、版本、3,586文件ratchet及产品/测试构建
通过。29个新增/修改native文件中，13新增的生产模块最大257有效行，既有修改
最大374；无新增行数例外，UTF-8无BOM及scoped diff检查另由最终收口复核。

第二轮六程序封存后无重编执行16门：15门通过，startup失败，整轮保持partial。
Canvas14/0/1 ignored、resize13/0/1、geometry15/0/1、animation9/0/1、JS94/0/10、
image/measure145/0/1、grid16/0、scene11/0；集合重叠不相加。摘要绑定的实际产品
Canvas/resize/animation/geometry worker各1/0，build_resources2/0、exe_resources1/0、
javascript_worker2/0。未另跑的ignored项不计通过，这不是全库回归或真实Loom验收。

startup为6/1：hidden_window_renders_and_destroys_without_webview中的无图smoke在
原15秒期限失败（elapsed=15.0365742s）；最后stdout为restored-batch完成信号，
stderr为空。该信号不能确定阻塞点或根因，不能归因Canvas或用后续成功覆盖失败。
原v348启动问题和v350未知根因仍开放，不放宽期限、删场景或改变锁恢复以刷绿。

第二轮debug产品41,901,056字节，SHA-256为
`d3e20dd3b4ba783f17c2e040a7550c5eb88c88d741f4b26f1f135f53939abe22`。
最终源码/构建件/封存件、旧v350/v351及首轮v352封存件、七文档与相关进程查询
绑定final-run-v2/closeout.json；其selectedRuntimeGatesPassed必须为false。默认JS、
完整N01–N12、全面手测和发布不放行；本轮不提交推送，SDK授权范围不变。

随后startup-diagnosis用同一封存字节启用已有smoke计时，不重编、不改生产或期限。
无图smoke固定三次为10.218/10.578/10.797秒、均exit0；完整startup集合固定三轮
却为6/1、5/2、4/3，均exit101。超时覆盖无图、钉图、WIC图片场景，最后阶段不同；
第一轮钉图的application已完成且耗时11,105ms，不能声称卡在原restored-batch。
记录到的文件阶段不足以解释整段耗时，嵌套时长不可相加；并发stderr还存在阶段
记录交错，不能把未配对begin/end当调用栈。根因未定位、未修，不再以额外重试刷绿。
三轮原日志、逐阶段分析及诊断后源码/程序/文档/进程复核独立绑定该目录closeout；
final-run-v2原partial收据和其中的文档快照保持原样，不追溯覆盖。

## v351：ResizeObserver 到真实宿主尺寸变化的接线

沿真实 stock-monitor 的 observer→rAF 消费合同补上通用原生能力，不修改 Loom，
不把股票业务翻译为 Rust，也不假造 Canvas。observe 在 operation 收尾读取同一
NativeMeasure/PreparedScene 的内容盒，初始/隐藏/移除/重接、选定 box 变化按真实
尺寸判断；内容盒扣原生 padding/border，按钮默认内缩也沿原 owner。rAF 仍下一帧执行。

真实 WM_SIZE/DPI 只记录代次；observer 存在与需排程分开，原 sync_input/策略/唯一
owner/输入、emit、presentation 空闲门才提交同一后台单槽。模态保留 dirty、不定时
轮询；呈现和最终 emit 同时核 viewport 与代次，resize 后回到原尺寸仍不能接纳旧帧。
合同和明确拒绝项见 [ResizeObserver](SURFACE_JAVASCRIPT_NATIVE.md#v351-事件驱动的-resizeobserver-子集)。

本轮独立证据位于 linshi/otter-native-v351-resize-observer，不覆盖旧证据或程序。
首次开发门发现新 fixture 使用未支持的 border shorthand，导致 mount InvalidData，
其余 SERIAL poison 为连带；已将 fixture 改成现有 longhand，并用 removeProperty 恢复
display，不为夹具扩张 CSS 支持。第二次开发门修正夹具首次恢复策略后缺少生产 poll、
relative 目标后静态 label 违反既有 paint-tail，以及按钮预期漏扣默认 1px border；
未删除断言或放宽准入/资源预算，两次失败日志均保留。第三次开发门13通过、0失败、
1 ignored；需外部摘要绑定的产品单项随后另跑，不把 ignored 当作通过。
原 v350 child/geometry/TLS/UIA 与 v348 startup 未知根因保持开放，完整迁移仍 active。

最终官方冻结2,421个构建输入，构建前/构建后/封存运行后三份快照完全一致，source
SHA-256为 `381377f65ae678d05c71665f0c0f9b6766f5cfc6abe6defc5cd0d160866038b1`。
JS syntax、fmt、全目标严格Clippy/check、版本门、3,573文件官方ratchet及产品/测试
构建均通过。22个新增/修改native文件中，六新增为35–160有效行，既有修改最大494，
无新增行数例外；UTF-8无BOM、git diff --check通过。

全部Cargo结束后封存六程序，直接执行封存件的11个选定运行门全部exit0：resize
13/0/1 ignored、geometry15/0/1 ignored、animation9/0/1 ignored、JS82/0/9 ignored；
集合有重叠，不相加。显式绑定本轮实际产品EXE和外部预期摘要的resize/geometry/
animation worker各1/0；build_resources2/0、exe_resources1/0、startup7/0、
javascript_worker2/0。未逐一另跑的其余ignored项不升级为通过；这不是全库回归。

实际debug产品为本轮artifact/otter-native.exe，41,195,520字节，SHA-256为
`a09c4f48f847595e7e8f7d4e4f7193e395d10c7f5b35504dfa2823510f90b120`。
六程序构建件/封存件摘要一致，原v350六份封存件不变；本轮/共享target相关测试
进程只读查询为零。完整逐门日志、源码身份和收口见本轮closeout.json；默认JS仍
关闭，未提交推送或发行，SDK授权范围不变。真实ResizeObserver→rAF接线不等于
Canvas、剩余CSS/timer、真实股票/Loom完整链或N01–N12迁移和最终用户验收完成。

### v350 续进：先修复失败观测，不用重试消除原失败

恢复时已逐字节复核 v2 的2,415源码输入和六份封存程序。本次继续修复同一未完成
切片，内部版本不重复分配；新证据独立放在 linshi/otter-native-v350-runtime-investigation，
不修改原 v2 日志、收据或封存件。以下 v2 摘要仍属于原冻结输入，不代表本次诊断源码。

已确认测试的等待盲区：后台 Runtime 在 prepare/click/update/frame 失败时直接退出，
不会发布成功 reply；原测试只等 reply，因而可能把已发生的错误掩盖成八秒等待超时。
测试接缝现在同时等待线程终态与 reply，终态优先，保留原错误；成功不消费 reply。
新增真实 TLS/child 负例提交不存在的目标，核验 InvalidData 和线程/child/句柄回收。
等待仍为原八秒，生产消息泵、协议、默认能力和资源预算不改。

批次测试补充 submission stop/ACK数量及逐项 attempted/revision，不记录事件 payload。
geometry 只在 cfg(test) 保留最多32项固定阶段/累计微秒，区分准入、解析、原生测量、
重排、目标查找和返回时过期；原250ms/父两秒门不变，不把阶段名当成原生错误根因。
首轮编译的测试 Error 名称误解析已修正，失败日志保留，最终定向结果另记。

原 TLS runtime-switch 失败仅走到 active ACK，尚未提交事件或取得切换 runtime 的
快照；不能称 runtime-switch 产品拒绝逻辑失败。UIA Toggle 缺跨步骤事件关联，Select
在 Expanded 断言之后变为 Collapsed，具体触发者仍未知。本次不改用户旧 Otter、不
猜改 UIA 产品逻辑，不以新定向成功清除 v2 或 v348 未知根因；完整迁移仍为 partial。

本次定向诊断另捕获并修复了 stale-frame 测试的错误口径：窗口已关闭且 revision
仍为1，但 JS 回收较快时，同轮会合法启动 inactive lifecycle，聚合网络句柄便可为1。
现改为直接断言原 event_backlog 为0（覆盖事件/批次任务、outcome和收据），不把0
改成1、不删“不发送事件”要求；原 mode7 peer 仍只接受 inactive/revision3、零events，
统一退出仍验证最终 Inactive、child/HWND回收和无额外请求。初次失败日志保留。

最终诊断使用独立 runtime-investigation-v2 目录与新封存测试库，无重编运行七套
选定门全部通过：等待错误负例1、Runtime owner8、geometry模型5、真实引擎1、runtime
切换负例1、过期viewport1、两轮animation FIFO1；集合有重叠，不相加。过期场景实收
stage5 inactive 后成功，runtime切换负例实收两事件/两观察且拒绝错误runtime。
测试库SHA-256为 `b5597069cd6b9fef803b1256dc6aeb41c70df165abcd8ce400d0954ea1fd5ee1`；
未重建或发布产品EXE。最终静态/行数/源码身份/进程与旧封存件复核见该目录closeout。
这些结果关闭上述两处测试缺陷，不关闭原child/geometry/TLS/UIA或v348未知根因。
本次十个修改源码为34–364有效行；3,567文件官方ratchet、fmt、全目标严格Clippy/
check和git diff --check均通过，无新行数例外。运行前后2,415输入一致，原六份封存
程序不变，最终UTF-8无BOM和相关进程零残留复核绑定诊断closeout；未提交推送。

## v350：同 child 的 animation frame 与原应用 deadline/FIFO 接线

真实 Loom stock-monitor 消费者的 ResizeObserver→requestAnimationFrame→drawChart 链仍
缺原生执行入口。本切片先补实际 rAF/取消/单调时钟，沿原私有 Session/Mounted/Runtime
单槽交付，不造 Observer/Canvas 空壳，不把股票业务翻译成 Rust。精确64容量、逐 callback
microtask checkpoint、下一帧快照、暂停/销毁和原预算见
[animation 合同](SURFACE_JAVASCRIPT_NATIVE.md#v350-宿主驱动的-animation-frame)。

原 AuthorizedWindow 仅在 sync_input 核定当前应用策略后排程，复用消息阻塞等待的
nominal16ms deadline；无 callback/背压/外借 owner 不贡献帧唤醒。原 expires_at 与
wake_at 分离，不新增线程、周期 timer、动作/授权/FIFO owner，不保证 VSync/60fps。
已经在途的帧仍可收割，但仍核原授权/快照/viewport；过期关闭、不重放 listener。

通用 fixture 覆盖同 child 两帧/取消/viewport/update；真实 HWND 比较同步 bounds、
D2D 绿蓝像素与实际 resize。Application 复用原 TLS peer/两轮 batch，覆盖授权页 busy 暂停、
点击只排 rAF、帧 emit 原两事件、外借 owner 阻止下一帧、同 child update 后恢复及
未消费帧遇 resize 关闭。早期编译夹具 unsafe 前提和恢复策略同步顺序修正保留日志，
不删除场景或放宽原 heap/stack/时间/Job/startup 门。另一次运行失败是 mount 结尾已
启用输入并提交首帧，夹具随后才设 busy 却断言尚未排帧；现于原 mount 前先设策略，
恢复时先同步策略再等 frame deadline。后续 SERIAL poison 是首个失败的连带结果，
不计独立缺陷；最终成绩另记，不继承旧绿灯。

首次冻结的严格 Clippy 拒绝否定 is_some_and 的非最简布尔式；改为等价 is_none_or，
不添加 allow 或豁免。最终采用全新 v2 证据目录，重新冻结并重跑全部选定门；原目录
的失败日志、冻结输入和较早通过均保留，不覆盖、不混用，也不再分配内部版本。

证据目录 linshi/otter-native-v350-javascript-animation-v2；当前内部版本仅分配一次。
冻结后分阶段严格静态/相关回归/全部 Cargo，再封存六程序、无重编运行并绑定实际
产品外部预期 SHA-256；旧 .339–.349 封存件不覆盖。本轮无依赖/SDK 或新在线 OSV、
安装/注册表/分发/默认 DLL/JS、提交推送或发布；SDK 仍仅限 linshi 内 Otter 隔离构建。
指定证书仅只读核不存在。原 v348 startup 超时未定位/未修，新成功不能清除旧失败。
ResizeObserver/Canvas/timer、完整 CSS/真实股票/Loom、默认 JS、权限隔离、长稳、最终
手测和 N01–N12 仍开放，继续原生迁移，不把本切片当完整交付。

当前严格静态/行数门通过，但较大回归不是全绿：SurfaceWindow 为222 passed/
1 failed/11 ignored，原 UIA Toggle 期待 Off 却收到 On。固定三次单项均通过，不清除
原失败；另一次完整 UIA 子矩阵为40/1/9，Toggle 通过而 native Select 重定位读取
0项而非2项。日志观察当时前台属于另一个已运行的旧版 Otter，已只读核进程身份并
保留运行；不能据此断定夺焦或输入就是根因，不修改用户程序或删断言。

Application resources 为27/2/1：runtime-switch 负例的 TLS peer 等待下一请求到原
deadline，随后 batch 的 attempted 断言失败；下一 update-failure 负例为 SERIAL
poison 连带失败。此前同源码 JS 集合中这些负例通过，仍不能以成功重试结清此次
失败。UIA Toggle/Select 与本次 TLS/batch 时序根因均未定位、未修。其余回归和封存
产品继续独立采证，本轮 allRequiredSelectedGatesPassed 必须为 false，不能称整轮
回归通过或开发完成；原 v348 startup 未知根因也继续开放。

最终 v2 严格静态阶段189.69秒、构建阶段268.08秒均exit0：34 JS syntax、fmt、全目标
严格Clippy/check、版本/依赖/28材料合同和3,567文件ratchet通过；八新增文件21–142
有效行，既有修改最大450，无新例外。冻结源码下rAF9/1 ignored、JS112/8、非desktop
Surface717/18、设备会话134/授权84、Host/授权窗口各2、shutdown4/Live预算3通过。
集合重叠，ignored不计通过；SurfaceWindow和Application resources失败仍如上保留。

全部Cargo结束后封存六程序，无重编运行652.00秒、exit1：animation为4/5/1、spatial
5/6/1、geometry6/9/1、JS Grid8/9/1、Grid33/9/1、Application JS4/8/0，斜线依次为
passed/failed/ignored。多组首个失败是原pipe/等待deadline或原生geometry拒绝，后续
SERIAL poison为连带，不能按失败总数推断独立根因，也不能仅凭资源采样归因系统负载。
根因未定位/未修，封存运行和整轮选定门均保持partial，不再盲目重试覆盖这些失败。

同一封存产品、显式外部预期SHA-256绑定的实际animation/spatial/geometry/Grid/CSS/
文档/旧worker各1项通过；文档11/1、CSS12/1、固定三轮emit预算各2、build/exe/worker
入口资源2/1/2和startup7/0通过。startup测试报告39.32秒，原每child15秒不变；这些
成功不清除本轮封存失败或原v348启动未知根因。

2,415源码输入冻结/构建后原字节一致，SHA-256
`71eea047f17f871fa367ec159a603d21f679e6f6b8782a5aaf5f780fd2e9c772`。
六程序目标/副本和旧.339–.349封存产品摘要不变；debug产品41,174,528字节，SHA-256
`115b37853128459a700a1ffc74daf81b58313fba4049742e01f4150c43c9b489`。
最终源码/进程/指定证书只读查询、七文档UTF-8无BOM/链接及diff完整复核另见closeout；
allRequiredSelectedGatesPassed和sealedRuntimeGatesPassed均为false。该debug件不发布、
不交全面手测，完整N01–N12仍未验收，无提交推送或依赖/SDK授权范围变更。

## v349：空间 CSS 接入现有原生定位、裁剪和同几何命中

新增私有 spatial 工厂，position/四边/inset 与容器 visible/hidden 沿原 declarations/
cascade→projection→surface_position/overflow→NativeMeasure/Presentation 消费，不另造
布局或业务。精确直接父、双轴锚点/尺寸、relative 百分比、paint-tail 和同轴裁剪
限制见 [空间 CSS 合同](SURFACE_JAVASCRIPT_NATIVE.md#v349-受限定位与容器裁剪)。不能把
DFS 顺序当完整 CSS stacking，也不接未实现的滚动/Canvas/股票模板。

只读复核找到旧尺寸 `.N`/`N.` 可能获准但 native 回退 intrinsic，统一规范化到同一
Length decimal；新 shorthand 的可选 quad 增加 own 门，阻止 inherited getter。首次
开发失败是测试把不受支持的 style HTML 属性写入 fragment；改用已支持的 style API，
并同步修正负例，避免它们在错误入口提前失败。失败日志保留，最终结果另记录。

Application 原 geometry/TLS/FIFO 夹具在同一代码路径使用定位按钮、root 裁剪和同步
位置断言，不修改原事件 ID/字节/顺序/Grant owner。无新线程/宿主 I/O/依赖/SDK、
系统安装/注册表/分发/DLL 或 JS 默认启用、提交推送或发布；原预算不提高。
SDK 授权仍仅限 linshi 内 Otter 隔离构建。证据目录为
linshi/otter-native-v349-javascript-spatial；全部 Cargo 后再封存六程序，无重编运行。

此前 v348 启动调查已封存固定三次完整矩阵，各7 passed；2,400源码输入和六程序
原件/副本一致，原15秒 child 门不变。原4/3超时失败仍保留，根因未定位/未修，不能
用成功矩阵或 CPU 采样归因并清除旧失败；证据另在 v348-startup-investigation。
ResizeObserver/rAF/Canvas、完整 CSS/真实股票/Loom、默认 JS、权限隔离、长稳、最终
手测和 N01–N12 仍开放，目标 active；本轮验证成绩另记，不继承 v348 绿灯。

本轮冻结源码/编译总门838.91秒、exit0：spatial11 passed/1 ignored、geometry15/1、
JS103/7、JS Grid17/1、Grid42/1、非desktop Surface711/17、SurfaceWindow222/11、
Host/授权窗口各2、设备会话134/授权84、Application resources27/1、shutdown4/
Live预算3通过。集合重叠，不相加冒充全库，ignored不计通过。31 JS syntax、fmt、
全目标严格Clippy/check、版本/依赖合同、28材料合同和3,559文件行数门通过。
17项输入变更中七项新增为17–204有效行，既有修改最大328，无新增行数例外；
依赖、默认能力、原生Grid与旧runtime_tests原字节不变。

全部Cargo完成后封存六程序，无重编运行总门254.05秒、exit0。spatial11/1 ignored、
geometry15/1、JS Grid17/1、Grid42/1、Application JS10、文档11/1、CSS12/1、固定
三轮emit预算各2，以及显式绑定EXE/外部预期SHA-256的实际产品spatial/geometry/
Grid/CSS/文档/旧worker各1项通过。资源/内嵌/裸worker入口为2/1/2项，startup7/0，
整套外层22.05秒；原每child15秒不变。真实HWND比较定位/裁剪/同步bounds、D2D
像素与同几何命中；这不是真实股票/Loom或完整桌面库验收。

2,407源码输入的冻结/构建后/运行后原字节一致，SHA-256
`71ff8461ad1ef70e5d28949b907d2f3858a097f5d2705232de1a467ca0d19a13`。
六程序目标/副本和旧.339–.348封存产品摘要不变，相关进程残留零。debug产品为
该目录artifact/otter-native.exe，41,141,248字节，SHA-256
`ff53aa6ab6a513e872f9c84a8ab82b1d4e5699aea878ceef31c7500860d0d817`。
七文档/完整输入的UTF-8无BOM/本地链接、diff和closeout独立复核；指定localhost
证书在CurrentUser My/Root均不存在，本轮只读查询、没有再次删除。早期夹具失败
和原v348启动超时保留，旧根因未识别/未修，本轮startup成功不能结清旧问题。
默认JS仍关闭，完整N01–N12/最终手测未验收，目标active，无提交推送或发布。

## v348：同步原生几何读取与同 viewport 的事件交付

新增私有 geometry 工厂/callback，Element rect/client/offset 与 devicePixelRatio 使用
真实 HWND 客户区/DPI，并复用同一 NativeMeasure→PreparedScene/DirectWrite；Host 原字体/
context 提取为唯一 surface_viewport owner，不以外框或猜测的默认 viewport 代替。精确
返回、取整、隐藏/断开/压平 inline、缓存和拒绝范围见
[同步几何合同](SURFACE_JAVASCRIPT_NATIVE.md)。没有第二套浏览器布局或股票业务 Rust。

生产 initial/click/update 均传当时 viewport；读布局的 reply 在 replace 前后核对，
staged revision/resources 不提前推进。只读审查发现已接纳 emit 暂存期间可能丢失该依赖，
已将 events_viewport 绑定原批次，take/cancel 保留，begin_batch 在关闭 submission 的同步
重入后再次核对。过期终态关闭、不重跑 listener、不提交旧尺寸动作，原 ID/字节/FIFO/
Grant 与关闭 owner 不变。Application 用真实 HWND resize 分别覆盖接纳前和接纳后暂存。

保留开发失败原件：提取 Host import 导致测试 GetDpiForWindow 未显式导入；clipped 夹具
忘记 minHeight 而被真实 flex shrink；getter 深入既有 CSS/生命周期导致原 256 KiB 栈
耗尽，改捕获 Map iterator 和直接生命周期调用，不提高预算；按钮原生圆角使角落像素
透明，夹具显式 border-radius:0；新成功模式误等待线程退出而非 update reply，修正
测试等待句柄。后续 SERIAL poison 是首个失败的连带结果，不计成独立缺陷。早期行数
脚本误传不支持的 javascript 语言名已改官方 extension 解析，不作为源码问题。

原生回调每 operation 256 reads/8 layouts、单投影缓存，失败/超限/返回时过期粘性拒绝；
原 heap/stack/时间/Job 预算不变。准入层无 viewport/独立 inline 的读取可 catch，但不
返回伪尺寸。同步 native 工作无法中途由 JS interrupt 打断，父两秒门仍负责进程兜底。
本切片无依赖/SDK 变更、新在线 OSV、系统安装/注册表/分发/DLL 默认启用、提交推送或
发布；SDK 授权仍仅限 linshi 内 Otter 隔离构建。

证据目录为 linshi/otter-native-v348-javascript-geometry；先冻结并完成全部 Cargo，再
封存六程序、无重编运行新增 geometry/原 JS/Application/预算/资源/startup 门，旧
.339–.347 封存件不改写。最终结果另记录，不继承旧绿灯。ResizeObserver/rAF/Canvas、
完整 CSS/真实股票/Loom、默认 JS、OS 权限隔离、长稳/用户手测与 N01–N12 仍开放，
目标 active；viewport 过期保守关闭不是完整自动 resize 恢复。

本轮冻结源码/编译门 840.34 秒、exit 0：geometry 15/1 ignored、JS 92/6、JS Grid
17/1、Grid 42/1、非 desktop Surface 701/16、SurfaceWindow 221/11、Host/授权窗口
各2、设备会话134/授权84、Application resources 27/1、shutdown4/Live预算3通过。
集合重叠，不相加冒充全库；ignored 未作为运行通过。27 JS syntax、fmt、全目标
严格 Clippy/check、版本/依赖合同、28材料合同和3,552文件行数门通过，无新增例外。
九新增文件25–155有效行，既有修改最大450；依赖、默认能力和旧 runtime_tests 原字节不变。

六程序封存后的运行总门267.14秒、**exit 1，partial**。geometry 15/1、Application
JS10、原 Grid/文档/CSS、固定三轮 emit 预算各2项、实际产品 geometry/Grid/CSS/文档/
旧worker各1项及资源/内嵌/裸worker入口2/1/2通过；startup 为4 passed/3 failed。
三个完整smoke均超原15秒期限，失败尾部进度不同。后来同字节隐藏场景单跑及直接
opt-in计时通过，直接smoke约8.98秒，但没有重现并定位原失败；不能据单跑成功清除
失败矩阵，也未判定为环境抖动或几何实现回归。期限不增加、不跳过场景，不以重试
覆盖旧证据；阶段计时和未定位状态保存在 startup-analysis，根因修复仍待继续。

2,400源码输入冻结/构建后/运行后原字节一致，SHA-256
`4b627a0290d55558502e5f27331b386646b692568778b76867588ed7cafe237a`。
本轮六程序目标/副本及旧.339–.347封存件摘要不变，相关进程残留零。debug产品为
该目录 artifact/otter-native.exe，41,137,152字节，SHA-256
`9fb8561e8ea7c8101196ef013bfe6004b26e0a62458c8973c4a2376fd3e48da8`。
七文档/完整输入 scoped 复核、UTF-8无BOM/链接/diff与partial closeout另验；不是整轮
runtime通过、可发布包或N01–N12完成。默认JS关闭，目标active，无提交推送或发布。

## v347：受限 CSS Grid 接线、声明层叠与真实动态呈现

css_grid_tracks/css_grid 在原私有 CSS 工厂链分别拥有非递归轨道准入和 shorthand/
端点/对齐。所有声明先全量验证再展开 longhand，逐属性 cascade 后投影到 v346 原生
Grid；不另造布局树、浏览器或业务分支。grid-row/column 的另一端 reset、两值 gap、
place-* 与混合 important、反射/移除均走同一声明 owner。Grid self normal 映射 stretch，
auto 仍继承父级，不改 Scene 原协议。精确支持/拒绝见
[JS Grid 合同](SURFACE_JAVASCRIPT_NATIVE.md) 和 [原生 Grid](SURFACE_GRID.md)。

保留开发失败：初始多层 parser 耗尽原 256 KiB 栈，改为非递归扫描与压平调用，不
提高预算；污染夹具的 descriptor 自继承 value，改无原型 descriptor；HWND 夹具
误把外框宽当 client 宽，改用真实原几何计算 fr 剩余量，不改变生产布局。修正 token/
repeat 展开后长度与正 fr 的 f32 下溢准入，加入原子拒绝和 self normal/auto 反例。
相关锁 poison 为首个夹具失败的连带结果，不能计作独立生产故障。

同 child 通用夹具真实执行 sheet/class/inline 的 Grid、点击 emit、显隐与更新；隐藏
HWND 的生产 Presentation 检查二维几何、继承文字、D2D 像素、原候选和同几何命中。
隐藏自动项不占格；语法合法但原生不支持的不确定轴百分比/前置负隐式轨道在呈现
阶段失败并保旧帧。默认 JS/Grid capability 不扩大，完整 CSS/股票/Loom/Canvas、OS
权限隔离、长稳/用户手测与 N01–N12 仍开放，目标 active。无依赖/SDK 或新在线 OSV，
无安装/注册表/DLL 默认启用、提交推送或发布；SDK 授权仍仅限 linshi 内 Otter 构建。

本轮证据位于 linshi/otter-native-v347-javascript-grid。冻结、静态/相关回归、全部
Cargo 后六程序封存和无重编同字节产品门分别记录，不能继承 .346 绿灯或改写旧件。

最终冻结源码门 595.08 秒、封存运行门 134.16 秒，均 exit 0。JS Grid 17/1 ignored、
Grid 宽筛选 42/1、JS 77/5、非 desktop Surface 690/15、SurfaceWindow 220/11，设备
会话 134/授权 84/授权窗口 2、Application resources 24/1、shutdown 4/Live 预算 3
通过；集合重叠且 ignored 未冒充执行，不代表完整桌面库。24 JS syntax、fmt、全目标
严格 Clippy/check、版本/依赖合同、28 材料合同及 3,543 文件行数门通过。八新增文件
30–204 有效行，既有修改源码最大 278，无新例外；旧 runtime_tests/原生 Grid/依赖
和默认 capability 原字节不变。2,391 源码输入冻结/构建后/运行后相同，SHA-256
`f479642232a02c14676ad0ffc5e19375298dac3a56ff8e73cf5c310f619ef971`。

六程序封存后无 Cargo：JS Grid 17/1、Grid 42/1、Application JS 7、文档 11/1、CSS
12/1、实际产品 Grid/CSS/文档/旧 worker 各一项、emit 预算三轮各两项，以及资源/
内嵌/startup/裸 worker 入口 2/1/7/2 均通过。startup 整套 17.09 秒，不增加原每
child 15 秒；实际产品 Grid 小夹具峰值 Job 4,964,352 字节，不能外推全产品或长稳
内存。六程序目标/副本及旧 .339–.346 封存产品摘要一致，相关进程残留零。实际
debug 产品为 artifact/otter-native.exe，40,910,336 字节，SHA-256
`9d2188d88eb77eeed691f00d7cdd0aee254e52b8d5de04c716d90ffc98a5190b`。
八文档、完整输入 scoped 复核、UTF-8 无 BOM/本地链接及 diff 门另验；最终回执为
本轮 closeout。失败原件保留，不结清旧 OSV/权限/完整迁移或发行门，目标保持 active。

## v346：原生 measured-content Grid 与同一呈现/命中链

新增 `Kind::Grid`，通用轨道语法、稀疏放置、二维分配、item/content 对齐和单行
baseline 沿原 Scene→NativeMeasure→Tree→Presentation→DirectWrite/D2D→Geometry/
Hits 消费，不嵌浏览器、不复制股票业务。图片百分比/自然比例使用实际 cell，快照
保留相同包含块；无 Grid 场景维持原测量行为，有 Grid 才启用每轮有界缓存。
精确属性、预算与拒绝范围见 [Grid 合同](SURFACE_GRID.md)。

开发中修复了 row-locked 错误回填前洞、空 Grid 凭空创建隐式轨道、非法 JSON 对齐
值落入默认、fr 活动集合嵌套 contains 和 baseline 合成高度不一致。搜索/跨轨道
贡献工作量均有显式预算；不确定高度百分比行、跨行/水平 baseline 和 baseline
百分比高度明确拒绝，不以 auto 或父整高冒充实现。baseline 预量与最终放置共享
约束，保护显式高/minHeight 容器与真实文字同行。

原失败日志保留：首次编译漏接测试中的 `Placed.grid_cell`；首次 HWND 像素夹具用了
不受支持的 `backgroundColor` 原生键，改用实际 paint owner 的 `background` 后复验，
没有修改生产 paint 支持范围；严格 Clippy 拒绝八参数 axis 入口，改为轴空间参数
结构，不增加 lint 豁免。模型、DirectWrite/图片/缓存和真实隐藏 HWND 像素/命中/
click route/替换失败回滚/隐藏/resize/close 定向通过，整轮冻结验证另行记录。

JS CSS `display:grid` 尚未接入，默认 capability 不广告 Grid，默认 JS 仍关闭。
这不是完整 CSS min/max-content 或 Grid 算法；完整股票/真实 Loom、DOM/CSS/Canvas、
OS 权限沙箱、资源长稳、用户手测和 N01–N12 仍未验收。没有 SDK/依赖变更、新在线
OSV、系统安装、注册表修改、DLL 默认启用、组包/发布或提交推送。SDK 授权仍仅限
linshi 内 Otter 隔离构建，旧 .339–.345 封存件不改写；目标保持 active。

冻结源码后的整轮开发/编译门 700.17 秒、exit 0：Grid 筛选 25 项（22 项为本切片
专属、另三项是历史名称匹配），非 desktop Surface 676/14 ignored、SurfaceWindow
217/11 ignored、JS 60/4 ignored、设备会话 134/授权 84、授权窗口 2、Application
resources 24/1 ignored、shutdown 4/Live 预算 3 通过。集合有重叠，不相加冒充全库；
未运行完整桌面库。fmt、全目标严格 Clippy/check、版本/依赖合同、28 项材料合同及
3,535 文件官方行数门通过；十新增文件 84–306 行，仅单一 axis 职责为 306，其余
均不超过 249，既有修改最大 353，无新增例外。2,383 源码输入冻结/构建后相同，
SHA-256 `17fd06c4917a212e43b531477f8aee139956552fa2c11b2d42dc1e0960a40ebd`。

全部 Cargo 结束后封存六程序，无重编运行门 124.28 秒、exit 0：Grid 25、Application
JS 七项、文档 11/1 ignored、CSS 12/1 ignored、实际产品 CSS/文档/旧 worker 各一项、
预算三轮各两项及资源/内嵌/startup/裸 worker 入口 2/1/7/2 项全部通过。startup 整套
17.22 秒，原每 child 15 秒不增；产品 CSS 小夹具 peak Job 4,775,936 字节，不能
外推全产品/完整 Grid/股票或长稳内存。源码冻结/构建后/运行后相同，六程序目标/
副本及旧 .339–.345 封存产品摘要一致，相关进程残留零；七文档及 scoped diff 另验。
证据位于 linshi/otter-native-v346-grid，实际 debug 产品为 artifact/otter-native.exe，
40,902,144 字节，SHA-256
`30e1f92d53577a198d1e3066516fb19b2acd69c21b21c4c25faa8a2cfce3d35b`。
这是不可发布内部产物，不是默认产品 Grid/JS 或完整迁移/用户手测批准；完整结算见
本轮 closeout，早期失败保留，不以本次通过追溯结清旧缺口。

## v345：constructed CSS、author cascade 与真实原生样式

本切片不以 API 空壳或未知 Scene 键冒充兼容。css_values/selectors/declarations/sheets/
projection 五私有 owner 将 sheet adoption/important/specificity/source order/inline
更新转换为真实原生布局、绘制、继承和隐藏；原 HWND/Presentation/几何/输入、Grant/
FIFO/收敛/child 生命周期 owner 不变。无需新浏览器/依赖/线程/宿主 I/O，默认 JS
仍关闭；支持值、严格拒绝、盒模型/文字差异与预算见
[原生 CSS 合同](SURFACE_JAVASCRIPT_NATIVE.md)。

通用夹具让同 child 真实执行 sheet 创建/采用/替换、class/data/inline 更新、点击 emit、
隐藏和恢复；真实 HWND/生产 Presentation 的 DirectWrite/D2D 检查 border-box、均一
border/padding/gap、宽度/剩余空间 grow、继承白色字形和纯色像素，并复用同几何命中/
原候选消费。隐藏按钮退出几何，原位置可回落根容器，不错误要求原始 hit_test 为 None。
无任意 HTML UA 样式/完整 Flexbox、Grid/media/gradient/overflow/position/inline run/
字体/Canvas/observer/timer/rAF；真实股票和完整产品/Loom/权限隔离/N01–N12 仍未验收。

开发失败原件保留：首轮 HWND 断言把根命中误写为空，随后串行锁 poison 的连带失败
不冒充独立生产故障；仅修正预期，保留隐藏按钮不在几何/像素/候选中的断言。交叉核验
修复 none+保留 flex-direction 的组合拒绝、超大 RGB 通道与 native f32 不一致，并限制
行高/字号、显式 solid/border-box 和 flex item 的 auto width。私有 CSS 解析改用预先
捕获的字符串/RegExp exec/数值 intrinsic，不走可变 Symbol.split；篡改夹具首轮在自有
for-of 覆盖 iterator 后自损，改索引循环，不撤销跨 collect/update/dispose 的污染。
未提高 32 MiB/256 KiB/250 ms/父侧两秒预算。原 runtime_tests、依赖与默认 capability
保持原字节；旧 .339–.344 产物/回执不修改。

v1 全部门通过后，最终复核发现 rgb 通道内的 Unicode 空白被 JS trim 接受、但原生
ASCII 色值 parser 会拒绝并回退；诊断反例保留。仅增加 CSS 颜色 ASCII 门和拒绝夹具
反例，重新冻结 v2；v1 不封存/推广，不能把前一源码集的绿灯混入新产物证明。

本轮证据在 linshi/otter-native-v345-javascript-css。十一新增文件 27–142 有效行，既有
修改最大 275，无新增例外。完整门按冻结源码→全部 Cargo→六程序封存→同字节运行
顺序执行；最终 v2 的完整源码/编译门 589.56 秒、封存运行门 124.39 秒均 exit 0。
JS 60 passed / 4 ignored、非 desktop Surface 655/14 ignored、SurfaceWindow 216/11
ignored、设备会话 134/授权 84、AuthorizedWindow 2、业务状态回归 1、Application
resources 24/1 ignored、shutdown 4/Live 预算 3 均通过；集合重叠，不相加冒充全库。
19 个 JS syntax、fmt、全目标严格 Clippy/check、版本/依赖合同、28 个候选/材料合同
及 3,525 文件行数门通过，未执行新在线 OSV 或完整桌面库。

最终 v2 共有 2,373 源码输入，冻结/构建后/运行后身份一致，SHA-256
`9a1785f82797d3fc96b715f4f8ef03d13302eac34f29e94aec309bea0b66ef72`。
实际 debug 产品为本轮 artifact/otter-native.exe，40,653,824 字节，SHA-256
`6af4287d587572eea6ff44e34224298320fbdff96dedf908fee7c9deb2d6df9b`。
六程序封存后无 Cargo：Application JS 七项、文档 11/1 ignored、CSS 12/1 ignored，
实际产品 CSS/文档/旧 worker 各一项、emit 预算固定三轮各两项、资源/内嵌资源/
startup/裸 worker 入口 2/1/7/2 项均通过。startup 整套 18.07 秒，不放宽每 child
15 秒；产品 CSS 小夹具 Job 峰值 4,800,512 字节，不是全产品/进程树内存基线。
六程序目标/副本及旧 .339–.344 封存产品摘要不变，相关进程残留零。七文档和 scoped
diff 另行检查并绑定 closeout；v1 通过集合、早期失败及诊断原件均保留，不追溯放行。
默认 JS、完整股票/产品/Loom、权限沙箱、资源长稳及 N01–N12 验收仍开放；SDK
授权仅限获准隔离构建，没有组包、发布、替换用户程序或提交推送，目标继续 active。

## v344：同一 JS 文档树上的 fragment、查询反射与委托

本切片扩展通用文档模型，不改写插件业务，不嵌入浏览器。tree 独占稳定 identity、
live 集合、节点移动与 Fragment 消费；attributes/selectors/markup 分别拥有元数据
同源反射、严格简单匹配及先脱离解析再交换。bootstrap 只保留生命周期、listener 与
原生投影，json 保护私有协议。五工厂以私有参数组合进同一 QuickJS bootstrap，不将
state/nodes/bridge 放到 globalThis；父侧动作权威、原 HWND/Grant/FIFO owner 不变。

支持的标签、选择器与拒绝范围见 [原生 JS 文档子集](SURFACE_JAVASCRIPT_NATIVE.md)。
class/data/title/ARIA 仅元数据，不声称 CSS、字号/粗体、tooltip 或完整 UIA。文字叶/
按钮仅接受文本和内联文字，原生命中 target 是按钮；查询结果静态、children/childNodes
live，重复 Fragment/单节点移动按顺序规划，原子准入按最终树检查深度。
Application 原 JS 夹具现在真实使用 innerHTML、query、dataset/classList 和祖先委托，
仍沿原两次点击、每次两项 emit、不可变批次/FIFO/收敛与同 child update 链。

开发失败保留：不合法的测试预期改成真实循环插入反例；QuickJS stack 定位到递归
遍历和 Map callback 内的 RegExp.replace 嵌套。遍历/投影/JSON copy 改显式栈，转义
与属性枚举分步执行，不增加 256 KiB 栈、32 MiB heap、250 ms 或父侧两秒期限。
捕获 collection intrinsic、私有无原型数组/状态与数据编码，防止插件覆盖 WeakMap/
Map/Array 或 inherited setter/toJSON 暴露状态、伪造私有协议；emit payload 仍保留
标准自有 toJSON 语义。这不是 C 引擎漏洞审计或 OS 权限沙箱。

首轮综合门还发现旧 interaction 测试写死 dom:3；Text 纳入 identity 后该 ID 不再是
按钮。仅将测试点击目标改从初始 Scene 查找实际 Button，保留原事件/update/回收断言；
定向复验通过。v1 冻结/失败原件保留，最终 v2 与其唯一源码差异是此测试，不改生产
命中/授权逻辑，不封存或推广 v1。原 product runtime_tests 始终保持旧字节。

本轮证据：linshi/otter-native-v344-javascript-document，最终只采用 v2 集合。JS 定向
48 passed / 3 ignored，非 desktop Surface 644/13 ignored、SurfaceWindow 215/11 ignored、
设备会话 134、授权 84、Application resources 24/1 ignored、shutdown 4、Live 预算 3、
业务 state 严格边界 1 和授权窗口 2 通过。集合有重叠，没有运行完整桌面库。
严格 Clippy -D warnings/all-target check/fmt、十一 JS 文件 syntax、版本/安全合同、
28 项候选材料合同及 3,514 文件行数门通过；十个新增文件为 27–180 有效行，既有
修改源码最大 272，无新增例外。2,362 项源码输入冻结/构建后/运行后相同，SHA-256
`cd055f23c86e7ae654a5d9c4f48313dd8b5839aeb5d7d6407fb83bcb448091c6`。

全部 Cargo 完成后封存实际 debug 产品和五个测试程序；产品 40,629,248 字节，SHA-256
`93ee5846be2c58597f1109adc594ebb380ee468b42fb472de446446c1b36ec2e`，位于本轮
artifact/otter-native.exe。无重编运行同字节 Application JS 七项、文档子集 11/1 ignored、
固定三轮 emit 预算各两项、旧实际产品私有 worker 与新实际产品文档 worker 各一项、
资源生成/内嵌资源/startup/裸入口 2/1/7/2 项，均通过。原产品文档 ignored 门以封存
EXE 和外部预期摘要显式运行，不把默认忽略当作通过。新产品文档小负载 peak Job 为
4,595,712 字节，不代表整个应用/真实股票或长稳；startup 整套 18.50 秒，原每 child
15 秒与 JS 250 ms/两秒门不变。六程序目标/副本及旧 .339–.343 产品摘要前后一致。
七文档/改动文本 UTF-8 无 BOM/本地链接、scoped whitespace、源码身份和相关进程清零
另由本轮 closeout 核对。首个进程查询脚本的命令拼接错误原件保留，修正后仅查询，
未终止任何进程；不涉及产品修改或重编。无新在线安全扫描或完整 C 引擎/法律批准。

默认 JS、初始 emit/图片宿主、CSS/Canvas/表单/focus/尺寸/timer/rAF/包内模块仍缺；
真实股票入口先调用未实现的 CSSStyleSheet，不能声称已可运行。完整 N01–N12、真实
产品/Loom、权限隔离、资源长稳与最终手测仍开放；目标 active，不提交推送或发布。
SDK 授权仍只限 linshi 内 Otter 隔离构建，不系统安装、改注册表、分发或默认启用 DLL。

## v343：Application JS 输入、原批次 FIFO 与同 child 收敛更新

Runtime 的单槽命令增加 click/update，直接使用原 Mounted/同一 child，不重新取码/
mount。原 HWND 候选经场景身份、重新命中、键盘焦点门进入后台 listener；DOM 不能
直接声明远端 action。宿主权威校验和编码后的 emit 保留原 ID/完整字节/顺序进入原
dispatch_surface_batch/begin_batch，最多 32 项、整批 wire 64 KiB，不另建网络旁路。
容量不足保留原 emit、不重跑 listener；监听器/update、未消费事件与待呈现期间不
捕获第二次点击，原唯一 owner、ACK、确认暂停与显式接续合同保持不变。

收敛 runtime 由原宿主事件的内部来源标记决定，响应不能升级默认能力或静默换栈。
快照 scene 仅是远端动作权威；完整证据通过后暂存 resources/revision 并异步 update。
真实 DOM replace 后再核原 Grant/完整 binding/显示状态才提交版本和原 Pending 收据，
此前输入/输出门保持关闭。恢复格式复用既有 NotStarted；DOM 与收据跨结算点复用
原回复事件保留最后唤醒，不增等待槽；关闭仍只等线程完成，避免持续信号忙循环。
owner、预算和未实现能力见 [原生 JavaScript](SURFACE_JAVASCRIPT_NATIVE.md)。

开发失败均保留：backtrace 将首个超时定位为批次完成等待，阶段日志证明旧五秒单请求
夹具不足以覆盖两次 POST/两次 GET；测试同步改为覆盖原 10+25 秒预算，不改产品期限。
随后收据诊断定位收敛 Header 把业务 snapshot.state 对象误当 lifecycle 字符串，合法
更新因 Protocol 被拒；只在收敛 owner 分离解析，并保护生命周期非字符串仍严格拒绝。
测试 setup 不再手动开启输入，真实 HWND 定向点击须经过 Application 生产准入。

本轮证据：linshi/otter-native-v343-javascript-input。最终采用 v2 源码集：JS 定向
37 passed / 2 ignored、非 desktop Surface 633/12 ignored、SurfaceWindow 215/11 ignored、
设备会话 134、授权 84、Application resources 24/1 ignored、shutdown 4、Live 等待预算
3，以及业务 state 严格边界一项、授权窗口提示两项通过；集合有重叠，不冒充全库。
严格 Clippy -D warnings/all-target check/fmt、JS syntax、版本/安全合同、28 项候选材料
合同及 3,504 文件行数门通过。八个新增文件为 27–234 有效行，修改源码最大 447 行，
没有新增例外。实际产品与五个测试程序编译完成，2,352 项源码输入冻结/构建后相同，
源码 SHA-256 `c6be0233a4e923b9e6b9df168b7b15a392a8d641babcb4dabc6e1248e5b45d4d`。
v1 严格 Clippy 的 nonminimal_bool 失败保留，仅等价调整 Pending/确认保护的布尔分组，
没有封存/推广旧集合。一次零匹配诊断不作为验收，正确选择器及最终 Surface 门另验。
全部 Cargo 结束后封存实际 debug 产品和五个测试程序，运行期间不再重编。产品为
40,604,672 字节，SHA-256
`82a2ee8977ab3b92975ec0354af3746c4e6c64ecede76132fe775456a481c44d`，位于本轮
artifact/otter-native.exe。封存后同字节 Application JS 七项、固定三轮预算各两项、
实际产品私有 JS worker 一项及资源生成/内嵌资源/startup/裸入口 2/1/7/2 项通过。
startup 整套 17.65 秒，原每 child 15 秒及 JS 的 250 ms/两秒未改；私有 worker 小负载
peak Job 4,198,400 字节，不是整机/应用/真实插件资源验收。六程序目标/副本及旧
.339–.342 封存产品摘要前后不变，源码冻结/构建后/运行后身份相同。七文档 UTF-8/
本地链接、diff/文本边界和进程清零由本轮 closeout 独立复核。
Application 门使用固定设备会话的真实 loopback TLS peer、真实 child/HWND 与生产
调度，不是实际 Loom 配对/批准或产品菜单入口；产品私有 worker 也不替代此业务门。
没有新增依赖或在线安全扫描，不结清旧全仓 OSV/启动/UIA/证据链问题；默认 JS、
初始 emit/图片宿主、完整 DOM/CSS/Canvas/模块/timer/rAF、权限隔离、产品真实 Loom
与完整 N01–N12/资源长稳/最终手测仍开放。SDK 授权只限 linshi 内 Otter 隔离构建，
不安装/分发、不默认激活新 DLL，不组包、发布或提交推送，目标保持 active。

## v342：Application 的持久后台 JS 挂载、等待与关闭

沿原授权/资源 owner，prepare 供单次 Job 与持久 Runtime 共用；Runtime 只开一个
后台线程并持同一 child，单槽命令/回复有独立完成信号。后台空闲等待原租约撤权、
资源/凭据最早期限与 child 退出，UI 不取码/求值/等待活线程。窗口准入要求原请求
显式支持 JS；权威 scene 只准备隐藏 HWND，真实 DOM 帧完成才可 active，Active ACK
的后台绑定同步完成后才可 show。关闭只 stop，线程完成前不允许 take_closed；
退出泵纳入完成句柄，失败仍保留原附件 owner，不伪造 disposed/inactive。

资源/目录/观看先回收，再排 JS，最后分类生命周期；修复 JS 终态关闭发生于分支选择
之后、可能丢失最后唤醒而漏停用的接线缺口。关闭窗口只等待线程终态，不因未消费
回复持续有信号而忙循环；任何呈现失败都关闭，不借后续空回复冒称已呈现。
补查等待预算后，未启动的 JS 也预留撤权/回复/线程完成共三槽，Live/新建/恢复统一消费预留预算，
避免先接纳多个 pending 窗口再启动任务时溢出；只预留、不把虚构句柄加入等待。
本轮默认 JS、初始 emit/输入/异步收敛更新桥仍关闭，不是完整插件或迁移验收；
完整安全、owner 与预算见 [原生 JS](SURFACE_JAVASCRIPT_NATIVE.md)。SDK 授权不扩展，
旧封存产物保留，不组包、发布或提交推送。

最终只采用 v3 冻结源码集：JS 定向 31 passed / 2 ignored、非 desktop Surface
629/12 ignored、SurfaceWindow 215/11 ignored、设备会话 132、设备授权 84、Application
resources 21/1 ignored、shutdown 4、Live 等待预算 3 通过。集合重叠，不相加冒充全库。
严格 Clippy -D warnings/all-target check/fmt、JS syntax、版本/安全合同、28 项候选材料
合同和 3,496 文件行数门通过。十个新增文件为 48–236 有效行，修改源码最大 433 行，
无新增例外。本轮无依赖图变更或新在线 OSV 扫描，不结清此前全仓安全门失败。

全部编译结束后封存实际 debug 产品和五个测试程序；产品 40,542,720 字节，SHA-256
`8e4e622bacefc93f9759e71493ade28b1315012e2b32c0b8470d5d0f5d0a603e`。
无 Cargo 的同字节 Application JS 四项、固定三轮预算各两项、实际产品私有 JS worker
一项及资源生成/内嵌资源/startup/裸入口 2/1/7/2 项通过。startup 整套 33.41 秒，原每
child 15 秒、JS 父侧两秒/引擎 250 ms 未改。六程序目标/副本及旧 .339–.341 封存产品
摘要前后相同；v3 冻结/构建后/运行后源码身份一致。测试 peer 为固定设备会话的真实
loopback TLS，不是新设备配对/服务端批准或真实 Loom JavaScript 验收。

此前 v2 验证运行期间补入等待预留修复，最终源码不一致断言正确失败；未封存/推广
该集合，八项变化和失败原件留存。既有产品 worker 测试误写后已按历史原字节恢复，
最终身份再次核对；首轮编译/协议夹具/生命周期收割/Clippy 失败均保留，不覆盖成绿。
证据位于 linshi/otter-native-v342-javascript-application。七文档 UTF-8/本地链接、最终
diff 和相关进程清零另由 closeout 复核；默认 JS、输入/FIFO/异步 update、完整 DOM/
权限隔离/产品真实 Loom 与完整迁移仍未验收，当前切片 partial，目标保持 active。

## v341：授权 JavaScript 入口获取与后台会话移交

沿原设备授权和资源 HTTP owner，新增当前 snapshot.entryResourceId 的严格脚本租约
选择；仅 binary/application/javascript、固定同 origin HTTPS/Device/nonce/lease 路径、
精确大小/摘要及 UTF-8/NUL 门通过后，后台 Job 才启动真实持久 child。Ready 仅能移交
同 Grant，Mounted 在操作前后复核租约/当前快照/绑定；资源过期、撤权、换程序、旧版本
或不合法生命周期立即丢弃执行状态。共享任务槽与 child 容量沿既有 owner，不新建轮询。
初始帧必须先消费；挂起的初始 emit 不得穿过生命周期切换或被后续 update 越过。
完整 owner、阻塞 API 和空闲撤权依赖未来等待集合的边界见
[原生 JavaScript](SURFACE_JAVASCRIPT_NATIVE.md)。图片专属准入不放宽。

首轮旧 emit 预算测试失败及 mutex poison 级联保留；旧封存测试程序单跑同样失败。
隔离阶段诊断定位 mount/emit 的 UTF-8 逐字符热点，等价优化计数而不修改 250 ms
墙钟中断、2 s 父侧期限或 60 KiB payload。新增精确 Unicode/JSON 转义边界验证。
这不是完整 Application 调度/FIFO/收敛/关闭接线，默认 capability 仍不启用 JS，资源
字符串表仍为空；完整 DOM/CSS/Canvas/股票插件、权限沙箱和 N01–N12 验收继续开放。
SDK 只用于已批准的 linshi 隔离构建；不安装/分发、不默认激活新 DLL，不提交或发布。

开发侧最终源码门：JS 定向 22 passed / 2 ignored、非 desktop Surface 624/12 ignored、
SurfaceWindow 215/11 ignored、设备会话 127、设备授权 84、Application attach/image 2；
过滤集合重叠，不相加冒充全库。严格 Clippy -D warnings/all-target check/fmt、版本/
安全合同、28 项候选材料合同和 3,486 文件行数门通过。五个新增文件为 102/175/154/
203/211 有效行，无新例外；未改变依赖图，不把本轮合同检查当作新一次在线 OSV 扫描。
资源网络门使用固定测试设备会话与真实 loopback TLS peer，不是新的真实设备配对/
服务端批准或实际 Loom JavaScript 验收；前述 HWND 门也不是前台全局输入验收。
首轮预算失败、诊断夹具 RefCell 错误、COM 测试可见性编译错误和 needless_borrow
Clippy 失败原件保留，修复/复核记录在 linshi/otter-native-v341-javascript-authorized。

全部 Cargo 门结束后，实际 debug 产品与五个测试程序封存。产品 40,332,288 字节，
SHA-256 `b8d1f6afe4ab9aa6e7975501ea46cbecb8e753b57e3fbcde4c40741f2a120ae6`。
同字节预算测试固定三轮各 2 passed；真实产品私有 JS worker 另 1 passed，资源生成/
内嵌资源/startup/JS 裸入口门分别 2/1/7/2 passed。startup 整套 25.43 秒，不是单 child
期限；原每 child 15 秒、JS 父侧 2 秒和引擎 250 ms 均未改。六程序目标/副本前后
摘要一致，旧 .339/.340 sealed 产品不变，运行期间没有 Cargo 重编。产品门只证明
私有引擎，不等于授权入口已接到产品 UI；完整目标仍 active，本切片 partial。

## v340：真实 JavaScript 持久 child 与文本/按钮 DOM 原生执行切片

新增真实 ES module/Promise 引擎及保持状态的 mount/update/点击/suspend/resume/dispose，
不把 JS 定义改成 JSON，也不翻译股票业务函数。DOM 投影复用原 SurfaceWindow 的
DirectWrite/D2D/命中与候选门；emit 在父侧重新校对权威 scene，绑定/generation/
revision/随机事件身份由原宿主编码器填写。完整设计、运行入口与预算见
[原生 JavaScript 执行边界](SURFACE_JAVASCRIPT_NATIVE.md)。

当前只是显式库 API 与私有产品 worker，不是默认 Application attach/FIFO/真实 Loom
执行链；CSS、Canvas2D、HTML/selector、表单、timer/rAF 和完整不可信权限沙箱仍缺。
默认能力仍只报告既有实现，不广告完整 JavaScript。旧 WebView 产品/数据/协议不改，
不撤销 .339 的产品前台门 partial 或旧 UIA/启动失败。完整 N01–N12、资源/长稳/
供应链及最终手测继续开放；本轮不组包、发布或提交推送。

真实五锁严格 OSV 首轮发现 native TLS 两项后，仅定向升级 rustls-webpki 0.103.16 /
webpki-roots 1.0.11，其他依赖不变。复扫 native lock 无返回匹配，但其他既有锁仍使
整门因 24 个漏洞 ID 失败；两次完整原件保留，无新增忽略。许可证审计通过不等于
完整分发 notice/供应链批准。开发回归和实际产物证据保存于
linshi/otter-native-v340-javascript，验证收尾记录另列，不能以中间绿色覆盖失败。

本轮开发侧收尾：JS 定向 12 passed / 2 ignored（含真实隐藏 HWND），非 desktop Surface
614/12 ignored、SurfaceWindow 215/11 ignored、TLS trust 5、设备会话 118、设备授权 84、
Live media 22/2 ignored、Loom bridge 28/1 ignored；这些过滤集合有交叉，不相加冒充全库。
严格 Clippy/all-target check/fmt、版本/安全合同和 3,481 文件行数门通过，无新例外。
候选许可证材料首次失败后，在原 owner 增加精确 core/sys 0.14.0/MIT/registry/上游
原文 SHA-256 补充；28 项材料/身份合同通过，165 个全平台 normal/build 包的材料可
收集，不放宽缺失门或授予完整来源/法律批准。首次调用环境错误与真实失败均保留。

实际产品及五个测试程序在全部编译结束后分别封存；显式运行实际产品私有 JS worker
的门 1 passed，保留闭包/async/update/emit/dispose/reaped，测试负载 peak Job 为
4,263,936 字节，不等于整个应用或真实插件资源验收。资源生成/内嵌资源/startup/JS
私有入口正式门分别 2/1/7/2 passed，startup 整套 28.29 秒，每 child 原 15 秒未改。
六程序及目标/封存副本前后摘要相同；旧 .339 sealed 产品不变，无编译中途放行。
这不结清历史间歇失败、不证明默认产品 Application/JS/真实 Loom、完整桌面库或
N01–N12 验收；完整迁移目标继续 active，当前切片仍 partial，不发布或推送。

## v339 续进：实际产品重建与正式启动门；菜单真实服务链仍 partial

同一 `.339` 源码新增一份实际产品构建证据，不再把上一节 library-test EXE 当作产品。
`cargo build --manifest-path native/Cargo.toml --locked --offline` 及三套集成目标编译通过；
封存产品为 38,558,208 字节，SHA-256
`5c49c06ee632ed9023c77f244b2b629ae1ad7ee3514cf46ab42674b59898367b`。
封存后三套正式集成 EXE 顺序运行、没有重编或重试：build_resources 2、exe_resources 1、
startup 7 项通过。startup 为 14.41 秒（整套），每个 child 的原 15 秒期限未改；
该次通过不证明旧间歇超时的根因已修复。三个集成程序和产品前后摘要一致。

产品菜单→真实 Loom 的四轮夹具仍失败，业务尚未开始，不能继承上一节已通过的六请求链。
首轮误判空桌面枚举返回值；第二轮使用的 Python STARTUPINFO.copy 不保留附加 lpDesktop，
并非 RTK 已被证明重置 desktop。显式 Win32 STARTUPINFO 加自有 kill-on-close Job 的探针
证实 RTK 子进程可留在私有 desktop。第三轮错误发送主窗并不处理的普通 WM_COMMAND；
真实入口必须经过自绘菜单及 foreground/capture 门，不能改用工具面板消息绕过授权菜单。
第四轮在正常 SetForegroundWindow 处失败，没有配对或业务请求，自己的产品 Job 已回收。

后续单独探针区分建窗与可见时序：窗口初见确实尚未显示，但等到可见后，当前输入桌面和
未切换的私有 desktop 正常激活仍返回 false；当前桌面的 UIA SetFocus 也失败。
只观测到未取得前台，不归咎某个用户应用，不修改前台限制、生产菜单 guard 或系统设置，
不发全局输入、不切换 input desktop。最后的夹具修正增加可见等待，并将 Win32 的零返回
报告为激活拒绝而不是 WinError 0；修正后的整条产品夹具尚未重跑，不记为通过。

证据单独存于 linshi/otter-native-v339-product-loom，旧 real-loom closeout 和失败原件不改。
本续进仅更新开发文档与 linshi 夹具，没有修改产品源码、依赖或版本；目标保持 active。
完整桌面/UIA、实际产品菜单与真实服务交互、JavaScript、4K/进程树内存/长稳、完整供应链
及最终手测仍开放。没有组包、发布、替换正式程序、提交推送、额外证书清理、系统 SDK
安装或 SDK 分发，已有隔离构建许可不扩大。

## v339：隔离真实 Loom 的 HTTPS、设备会话与 Application 挂载门

新增显式 ignored 的 application_surface_real_loom_tests，复用生产 authorization_owner、
surface_start、Application poll 和 AuthorizedWindow；不注入 token、Outcome、绑定或 ACK。
独立 2026-10-18 内部 daemon 安装原始 framework/MCP/股票 Art，通过仅 loopback 的显式
TLS 终结夹具转发真实响应。Otter 按精确 origin 信任临时 CA，不修改系统证书库或选择
真实 Otter/Loom profile；这不是 daemon 已原生提供完整 HTTPS listener 的证明。

新 Ed25519 身份公开申请后，外部夹具只批准本次 pending ID；生产 challenge/signature/
session 返回真实设备会话。三个业务请求 attach/active/inactive 均为 Device 授权、独立
nonce 和 TLS 1.3，没有用管理员身份替代。真实 fallback 的原始 null 形态经过 v338
解析入口，15 节点、隐藏挂载、active ACK 后授权 show、一个 EDIT/两个 COMBOBOX、
两次 WM_PAINT、关闭后 HWND 失效及 inactive revision 3 ACK 均通过；最终没有远程任务
句柄或未回收授权任务。自有窗口按虚拟屏幕几何移出屏幕，显示不激活、不发全局输入，
不执行股票动作或 JavaScript。

首轮编译缺少 WaitForSingleObject 导入；第一轮运行把 canonicalize 的 verbatim 路径
误传普通 profile 门；第二轮在合成单地址 peer 的五秒等待超时。失败及各自源码/程序
保留。夹具保留普通绝对路径、另用 canonical 路径核验 linshi 归属，并使用覆盖真实
配置/DNS/多地址回退/HTTP 链的独立等待，不修改生产网络期限。诊断运行 attach worker
5.722 秒、其中 daemon 请求约 3.703 秒；不能由此回推第二轮全部时序或结清旧启动性能问题。

实际真实服务门 1 passed；Application resources 17 passed / 1 ignored（该门需显式运行），
非 desktop Surface 606 passed / 10 ignored、device_authorization 84、surface_start 2 passed。
严格 Clippy/all-target check/fmt、正式 verify:version 和 3467 文件行数门通过；一次误指
不存在版本脚本的命令错误另存。新测试 199、接线测试 239 有效行，无新增例外、依赖或
生产行为变更。证据在 linshi/otter-native-v339-real-loom，以 run-3 封存字节为准。

这是 library-test EXE 中的真实 Application，不是产品 EXE/菜单入口或用户在线 Loom 验收。
本轮没有重编产品 EXE、组包、发布、替换正式程序或提交推送；旧完整库 UIA、启动超时和
v337 证据链 partial 不追溯放行。完整 N01–N12、JavaScript、4K/进程树内存/长稳、
供应链权利与最终手测仍开放，SDK/系统安装/分发/默认 DLL 和 capability 批准边界不变。

## v338：真实 Loom 声明式快照的可空属性兼容

隔离的 2026-10-18 内部 Loom daemon 安装真实股票 Art 及其 framework/MCP 依赖，
按生产原生能力报告协商得到 declarative fallback；原始快照在 Node 反序列化时报
`invalid type: null, expected a map`。Loom 的 SurfaceNode 将 props/layout/style/
accessibility 定义为默认可空 Value，真实 divider 四项均为 null；不是非法事件集合。

surface_scene 仅把这四项的 missing/null 投影为空 Map，已有对象原值保留；非对象、
非 null 仍拒绝，events/children 的类型、属性数/事件数及字节/深度/节点总数门不变。
不在 HTTP/驱动中删除 null，不执行脚本、增加网络/资源读取或改变授权、默认能力。
五项新回归先出现两项预期失败，再通过；完整 Scene 11 项、非 desktop 命名空间的
Surface 定向 606 passed / 10 ignored、Application attach/image 两项通过。TLS remount
回归保留原始 null 快照，验证同一生产场景入口，不把合成 TLS peer 当真实 daemon。

外部公共库驱动消费未经修改的真实快照，能力报告与实际请求一致；15 节点、隐藏 HWND、
两次 WM_PAINT、中文标题和关闭后 accessibility handle 撤销通过。未绑定窗口的本地
编辑/指针门关闭且没有编辑子控件；没有伪造设备会话或绕过 bind_edits。驱动初轮
误用 EnumChildWindows 返回值，随后误认为未绑定窗口已有 EDIT/COMBOBOX；两次失败、
驱动源码及程序均保留，按联网核实的 Win32 合同和实际挂载生命周期纠正夹具，不改产品
输入权限凑绿。最终运行绑定封存驱动和九项输入前后摘要，不是实际产品 EXE 验收。

本次恢复时的完整库为 3223 passed / 1 failed / 74 ignored：Select/UIA relocation
在展开/列表查找环节失败。一次输入追踪又观察到自有前台进程收到 Enter 后正常收起；
Enter 来源未证，不能归咎其他应用抢前台。失败与诊断均保留，未再重跑凑绿；本修复
没有重跑完整桌面库或追溯结清 UIA、v337 原证据链异常及旧启动超时。

Clippy -D warnings、all-target check、fmt、版本和 3466 文件行数门通过；三项修改
Rust 为 236/226/91 有效行，没有新增生产模块、依赖或行数例外。2314 项构建输入
只改三项 Rust 和一次内部版本分配；旧九项产品/副本/集成程序摘要保持不变，本轮
没有重编产品 EXE。证据在 linshi/otter-native-v338-regression，真实 daemon/驱动
原始记录保留在 linshi/otter-native-loom-fallback-20261024-with-dependencies。
这是管理员 HTTP 协商加原生公共库呈现，不是设备 HTTPS、Application 实际 daemon
链、股票动作或 JavaScript 运行；上述生产连接、完整 N01–N12、资源长稳、供应链和
最终手测仍开放。SDK 仅限获准隔离构建，无系统安装/分发/默认 DLL 激活、组包或推送。

## v337：隐藏钉图工具面板的 BUTTON 首次使用准备

沿 N11 继续降低未使用的多窗口资源，不将计时热点冒称为旧超时唯一根因。
封存 v335/v336 的固定 ABBA 诊断分别为 9.391/12.329/11.031/10.438 秒，
各创建 84 面板，BUTTON 累计 2.073–3.884 秒；诊断不是正式启动验收或回归排除。

隐藏 pinned 面板继续立即创建 panel HWND、字体、图标并接入截图保护/registry/owner；
仅子 BUTTON 延至首次展示/显式准备。普通工作区即便隐藏仍提前准备，所有原生控件、
标签、TABSTOP、命令和绘制不变。可见 Application 的 prepare_window/prepare_window_at
在原事务提交前完成可失败准备，包含 Live freeze；恢复交换后的非 pinned 面板仍已准备，
不把创建失败移到恢复提交后。隐藏钉图的自动化子树在首次使用前不再枚举 BUTTON；
这是明确的资源生命周期变化，不宣称隐藏 UIA 枚举完全兼容。

tool_panel_content 拥有 BUILDING/READY、防同步重入、首次投影和展示前门；临时
Vec<Window> 在失败时释放所有已建 BUTTON，清除标记后可重试，全部成功才交给父 HWND。
未 ready 时不更新工具/样式/可用性缓存，首次投影强制读取当前状态、DPI 与滚动位置。
SWP_SHOWWINDOW 的 WM_WINDOWPOSCHANGING 统一接线，失败取消展示，不发布半面板。
不增加线程、队列、轮询、配置或依赖；关闭仍由原 panel/DocumentWindow 资源 owner 清理。

新增七项真实 HWND 回归，其中隐藏钉图不分配 BUTTON 的断言在旧实现先红后绿；包含
原状态/普通工作区提前准备、首次样式与忙碌门、
重复准备、当前滚动几何、部分失败回滚/重试、展示消息与重入拒绝/释放。原工具和截图
断言未删；全工具面板 28 项、composition 102 passed / 4 ignored 已通过。
第七项额外真实调用 ShowWindow（自有屏幕外、非前台窗口），已验证重入展示被取消、
失败保持隐藏、真实重试后当前 caption/disabled 状态、重复展示不重复分配与最终释放。
READY 只表示控件集合创建完成；refresh 保持既有 best-effort，模型重入借用时仍依赖
后续 owner 消息补刷。上述正常展示验证不等于完整 UIA/TAB 或所有借用时序验收。
证据在 linshi/otter-native-v337-regression，封存对照在 v337-investigation；完整 N01–N12、
完整桌面/UIA、4K/进程树内存/长稳、真实用户 Loom/JavaScript、供应链与最终手测仍开放。
不改旧失败回执、15 秒期限或默认 Shader/SDK/安装/发行批准，不组包、发布或提交推送。

最终固定三轮 Shader 正向 3/3、六套同字节 EXE 集成 26/26（startup 7/7）、拒绝 3/3，
最终面板 29 项、composition 102/4 ignored、Shader 65/15 ignored 及严格 Clippy/check/fmt
通过，运行前后九项程序/副本摘要一致。后续追加诊断期间，进程句柄消失；收尾发现
原 gates.json、诊断第 4 轮 result.json 和汇总 comparison.json 全零，另外两份诊断
日志有 NUL 尾段。原文件保留，原因未证；不补造原索引、第四轮结果或运行耗时。
因此本轮收尾保持 partial，新独立复核在 linshi/otter-native-v337-recovery，不能以
仍完整的单项通过记录冒称原始证据链完整。已核验的前三轮诊断显示 controls 准备
由 84 次降至 33 次，但 ABBA 矩阵不完整、不据此声称整体速度提升或排除性能回归。
完整源码/逐项日志/九项程序身份和旧失败另行保留；不追加 Shader 正向重试。

## v336：默认钉图 D3D11 设备弱共享与资源隔离

沿 N11 多窗口资源缺口推进，不将其冒称为 v335 超时的根因修复。新增
`composition_device`，同一 UI 线程只复用仍存活且 GetDeviceRemovedReason 成功的
默认 D3D11 设备；hardware→WARP 顺序保留。缓存只持 Weak，最后一个默认 surface
租约释放后不保留 COM 设备；重开重新创建。失效设备替换和创建失败不破坏旧窗口
仍持有的租约，不跨驱动调用持有 RefCell 借用，也不增加线程、队列或空闲轮询。

只共享 D3D11 device；每窗交换链、D2D context、DComposition device/target/visual、
backbuffer 和透明度 owner 不变。Live 的显式 with_device 继续绑定捕获设备，不接入
默认池。窗口私有 COM 资源先析构，再释放共享租约；不是把 GPU 开销转移到辅助进程。

新真实 GPU/HWND 门先红后绿，四项覆盖共享身份、窗口资源独立、alpha/resize/opacity/
存活窗口像素、显式 WARP 设备隔离、十个 64×64 钉图的最后租约释放与重开。五项
owner 测试涵盖线程隔离、健康/失效/失败恢复及 hardware/WARP 回退。故障判定用注入
验证缓存状态机，不冒称真实显卡 reset；小图身份门也不替代 4K/长稳/进程树内存测量。

完整 composition 命名空间 102 passed / 4 ignored（含新九项），Shader 65 passed /
15 ignored，工具面板 22 项及严格 Clippy/check/fmt/版本通过。三个新文件 65/104/194
有效行，原 composition 204→198 行、测试 161 行，无例外；3,464 文件行数门通过。
首轮六套 EXE 集成 25 passed / 1 failed，普通完整 smoke 在原 15 秒超时，最后到
unsaved-mini；不由最后标记推断根因。固定三轮正向与最终字节复验独立保留，不能
用后续成功覆盖初次失败，本轮仍 partial。证据在 linshi/otter-native-v336-regression。

最终封存字节的固定三轮 Shader 正向为 2 passed / 1 failed（第 1 轮 15.029 秒超时，
两轮 Shader commit 已输出）。默认拒绝三项通过，六套无编译复验为 24 passed /
2 failed：hidden_window_renders_and_destroys_without_webview 与
wic_image_upload_survives_two_paints 分别在 15.039/15.052 秒超时；没有追加重试。
产品/封存副本/七测试程序共九个摘要不变，不声称完整启动稳定或排除本轮回归风险。
最终 2,312 输入（原生 1,780）前后相同，五文档 UTF-8/本地链接和 diff check 通过；
初次编译闭包类型错误也保留，不重写任何失败。当前没有完整库/4K 内存或长稳证明。

完整桌面/UIA、真实 GPU 故障与资源基线、用户 Loom/JavaScript、完整供应链及 N01–N12
最终验收仍开放；默认 Shader、SDK/材料批准和既有失败边界不变，无组包、发布、
正式程序替换或提交推送。

## v335：工具面板实际 HWND 布局去重，完整 smoke 仍为 partial

从真实产品计时观测确认工具面板重复定位存在可消除的开销；新增
`tool_panel_children`，按最终 DIP/滚动几何直接创建原有 BUTTON，保留最后校正布局。
后续布局读取实际 HWND 位置，只对不匹配项调用 SetWindowPos；读取失败仍执行原
best-effort 定位。没有缓存几何/句柄、延迟创建、跳过控件或改变父窗口 owner。
缩放、WM_SIZE/WM_DPICHANGED、滚动及焦点可见性继续复用原调用链。

三项真实隐藏窗口回归验证不变布局零重复定位、错误位置纠正、resize/双轴滚动、
全部按钮几何及子 HWND 清理；先红后绿，最终工具面板相关 22 项通过。Shader 定向
65 passed / 15 ignored，严格 Clippy/all-target check/fmt 通过。诊断中 85 个面板的
layout 累计由 683 ms 降至 1 ms，但运行负载有抖动，不把不同轮耗时当作稳定加速保证。

预先固定的三轮无计时诊断 Shader 正向门为 2 passed / 1 failed；第 2 轮仍在原
15 秒超时，两次 Shader commit 标记已出现，最后到 execution-owner smoke。
不据最后标记推断唯一阻塞点，不放宽期限或追加重试凑绿。收尾时额外 Cargo build
改写产品 EXE，被摘要断言拦住；新字节已先封存，再单独做一次身份验证与集成复验，
不覆盖原三轮失败。证据、两套程序身份及原始失败位于
linshi/otter-native-v335-regression；本轮仍 partial，v334 partial 历史保留。

最终封存 EXE 的单次身份验证、默认拒绝三项及六套集成 26 项通过；产品、副本和
七测试程序共九个摘要前后相同。全库 3,461 文件行数门通过，两新文件 69/189 有效
行、两修改 Rust 为 432/181 行，无新例外。六文档 UTF-8/本地链接和 diff check 通过；
最终 build/产物验证前后 2,309 项输入相同，其中原生 1,777 项；document_state 与
v334 无净变化。它们是定向证据，不是完整桌面或原三轮矩阵全绿。

候选 pin/ACL/租约/默认关闭、材料 false 批准和 SDK 隔离授权边界均不变。没有新依赖、
组包、发布、替换正式程序或提交推送；完整桌面/UIA、实际用户 Loom、JavaScript、
source/tool/notice/CRT 权利闭包、长稳及最终全面手测仍开放，不结清整个 N01–N12。

## v334：显式固定 Shader 候选接入实际产品启动

产品增加只读 `--shader-candidate` 入口，普通/截图/会话 Application 与独立 smoke
共用安装方法；无参数仍为 `Installation=None`，不扫描、复制、安装或持久启用 DLL。
代码固定 v332/v333 两 DLL 身份，目录仅可选不能传入任意摘要。逐级本地固定盘路径、
句柄 owner/DACL、精确两文件、硬链接拒绝及同句柄摘要/租约归独立 candidate owner。
固定 pin 的 child 再做准入并持有自己的租约，父进程不加载 ANGLE。
完整用法、安全前提与验证入口见 [本地 Shader 候选](SHADER_LOCAL_CANDIDATE.md)。

真实反例确认目录共享不能阻止 `FILE_WRITE_ATTRIBUTES` reparse 控制，因此每一级
祖先 ACL 门不可删除；固定卷根是外部信任锚，持续私有权限和无历史恶意句柄也是前提。
本机 Public/linshi 祖先不符合门槛，不修改现有 ACL；正向夹具仅新建于私有用户 HOME，
只改新副本权限且保留不删，构建和其余证据仍在 linshi。

实际产品 EXE 的通过运行验证认证目录→生产菜单→正式输入→首次取码→真实 child→
正式提交，再改 gain 走缓存及第二个 child；PNG/显示像素、两次认证连接、无第三次
缓存连接、父侧未加载 ANGLE及退出锁释放通过。但最终默认计时正向门在原 15 秒
超时；中间同源码复验及最终一次计时诊断通过，不能替代此失败，当前为 partial。
最终源码 Shader 为 65 passed / 15 ignored，参数三项、默认拒绝三项通过，严格
Clippy/check/fmt/版本/3,459 文件行数门通过。八新文件 53–178 有效行，修改 Rust
最大 498，无新例外；九文档 UTF-8/本地链接通过。最终 EXE 摘要绑定无编译六套
集成 26 项通过、八程序身份不变；完整桌面库未重跑。证据在
linshi/otter-native-v334-regression，失败与诊断保留，不延长期限或结清旧问题。

这是显式本地开发运行，不修改 v333 材料中的 false 批准，不是实际用户 Loom、可信
发行安装、完整桌面或 N01–N12 验收。JavaScript、供应链/CRT/66 gitlink、资源长稳、
旧 UIA/启动问题及最终手测仍开放；未组包、发布、替换正式程序或提交推送。

## v333：本地 ANGLE 材料候选、独立核验与真实 payload 消费

新增 native-candidate/angle_materials 的显式 create/verify：独立审核的 stage/build
摘要→复用 v332 准入→白名单证据/源码、两 DLL、声明原字节及材料文件 SBOM→外部
预期 manifest 摘要独立核验。六个生产模块按准入/声明/SBOM/编排/证据/核验分责，
不改 builder 或应用 owner。写入前和核验期实施预算、Windows 安全路径、精确文件
与目录集合，不执行工具或复制的源码。完整命令及边界见 [ANGLE_TOOLCHAIN.md](ANGLE_TOOLCHAIN.md)。

实际候选为 159 清单文件、109 声明载体；create、无需原 stage/VS/SDK 的独立 verify
和联网取得的官方 CycloneDX 1.6/SPDX 2.3 schema/format 门通过。新鲜公共库外部
驱动直接消费材料 payload，硬件/WARP 六 contexts、18 次像素对照、12 成功 child/
两无效 GLSL 拒绝通过，零像素差异、child 父进程不加载 ANGLE。固定 Ninja 的只读
compdb 已展开两链接响应内容；对象 DEFAULTLIB/最终成员仍未闭合，不冒充重新链接。

这不是完整默认 source/tool/notice closure、法律批准、可信安装或产品验收；材料
批准/运行/安装/激活始终 false，外部驱动回执另存。Chromium root attribution、
静态 CRT/完整来源权利和 66 未提供 gitlink 保留。证据在 linshi/otter-native-v333-regression，
未重编 DLL/产品 EXE、分发 SDK、替换程序、默认启用能力或提交推送。
N01–N12、JavaScript/实际 Loom/资源长稳/最终手测及旧 UIA/启动失败仍开放。
最终 105 Python/9 Node 合同、fmt/版本/3,451 文件行数及 diff check 通过，九新
文件 30–120 有效行、无新例外。34 Python AST/七文档 UTF-8/本地链接通过，原生
1,767 项输入/旧 EXE 不变，相关进程零残留；没有重跑完整桌面库。

## v332：固定 GN/Ninja 显式构建 owner 与必需 PE 函数门

固定 DLL 构建从外部实验脚本提升为 `build_fixed.py`，复用现有 stage/profile/环境；
`build_inputs` 绑定外部已审阅 stage 摘要和 fresh 集合，`build_job` 独占 suspended→
kill-on-close Job→resume、精确句柄继承和有界清理，`build_binary` 独占纯字节
PE、必需函数可执行地址及普通/delay-import 静态 CRT 门。父进程必须明确设置
DEPOT_TOOLS_WIN_TOOLCHAIN=0，不下载/安装 SDK，不把仅用于隔离构建缓存的 SDK
d3dcompiler 放入 payload 或默认激活 DLL。
完整命令、失败/中止现场、权限和源码闭包边界见 [ANGLE_TOOLCHAIN.md](ANGLE_TOOLCHAIN.md)。

导出名称存在不足以证明函数入口：本轮先红后绿修复零函数数/非法 ordinal/EAT、
delay-loaded 动态 CRT 和必需 API 指向不可执行数据节；可选合法数据导出仍允许。
Job 合同只运行当前 Python 的有界父子进程，GN/Ninja 编排合同使用 tripwire/mock，
合成编译工具绝不执行；真实构建和显式外部驱动重放是独立证据，不能拿 mock 代替。

最终源码绑定的全新 stage 实际暂存 12,569 文件，GN 513 targets/135 files，504 步
完整两 DLL 编译通过（558.765 秒），后续无工作、编译诊断零项，不修改生成后的 Ninja。
PE x64/52 必需可执行导出/普通及 delay-import 静态 CRT 通过。新显式夹具的公共库
外部驱动硬件/WARP 六 context、18 像素对照、12 成功 child/两无效 GLSL 拒绝通过，
像素零差异、child 父进程不加载 ANGLE；不是实际产品 EXE 或 Loom 服务验收。
旧弱 PE 门绑定的首轮构建已主动中止，原现场保留，不计为最终成功。

81 合同、Rust fmt、版本与 3,442 文件行数门通过；八个新文件 74–167 有效行，原
JSON owner 的必要窄接线 94→96 行。七份文档同步 owner/边界，证据在
linshi/otter-native-v332-regression，最终构建/重放位于 final-run。原生 1,767 项
输入及旧产品 EXE 不变，没有新产品编译/完整桌面回归或默认能力变更。79 个 gitlink
仍有 66 个未提供，完整默认源码/工具/许可闭包、可信安装、N01–N12/JavaScript/
长稳/最终手测开放；v324 UIA/v325 启动失败不追溯放行。未组包、替换程序或提交推送。

## v331：固定本地 Git/归档暂存 owner 与实际消费验证

本轮新增 stage_pins、stage_inventory、stage_git、stage_archives、stage_build，
分别拥有固定身份、名称空间/预算、只读 Git 对象、数据归档及编排回执；复用现有
profile/adapter，不改原生架构、Rust 或用户程序。入口只运行显式只读 Git，不下载、
跟随缺省 gitlink、运行 hooks/构建工具、安装 SDK 或默认激活 DLL。固定 profile
的暂存不再完全依赖外部实验脚本；完整默认 source/tool/许可闭包和编译执行 owner
仍未完成。命令与长期信任边界见 [ANGLE_TOOLCHAIN.md](ANGLE_TOOLCHAIN.md)。

最终加固树实际暂存 12,569 个构建文件；79 个 gitlink 中 13 个独立 pinned 挂载，
66 个未提供且不可达性未全审计。两棵树实际文件/清单一致，最终树真实 GN 生成
513 targets/135 files，504 条命令计划与首轮原始日志逐字节相同。首轮树完整两 DLL
编译通过（857.063 秒），随后无工作；以内容等价关联最终加固版，不称最终树重编译。
PE x64/52 必需导出/无动态 MSVC CRT 和全部构建日志诊断检查通过。显式新 DLL
夹具以已绑定公共库外部驱动验证硬件/WARP 六 context/18 次像素、12 成功 child/
两无效 GLSL 拒绝，零差异、child 父进程不加载 ANGLE；不是实际产品 EXE/Loom。

设备名别名与 timer 启动失败泄漏的回归先红后绿；外部 Git alternates 明确拒绝。
最终 54 passed / 0 failed / 0 skipped，fmt/版本/3,434 文件行数门通过；新八文件
23–132 有效行。输入/输出/源码及旧产品身份另行绑定，证据在
linshi/otter-native-v331-regression。首次比较器 CRLF 文本/原始字节混用失败保留，
原始计划字节确实一致，不改计划。完整 N01–N12、JavaScript 呈现、实际服务、可信
安装、长稳/最终手测继续开放；没有新产品 EXE、完整桌面复验、组包或提交推送，
默认 Installation=None/关闭 capability、用户批准的 SDK 范围与旧 UIA/启动失败不变。

## v330：可复用 Clang profile、实际 DLL 与 GN 原生再生闭环

本轮新增构建侧 `gn_profile`/`prepare_profile`，复用已有 adapter，不增应用运行
owner 或修改 Rust。固定 x64 D3D11 Release 静态 CRT profile 使用上游已有
Clang/lld + MSVC STL 接线，避免旧 MSVC 中文 prefix 的生成后补丁被 GN 覆盖；
不关闭 `/WX`，不变更 Win32/Shader/JavaScript 架构或裁剪迁移范围。准备器只生成
外部 overlay，绑定输入/源码/Python/输出摘要，双架构、根与 profile 断言保持拒绝
回退；不是完整源码获取、staging 或编译 owner。

独立新实验树补齐固定 tools/win 后，GN 513 targets/135 files，504 命令的两 DLL
实际编译通过（602.235 秒），完整编译日志无 warning/error 诊断。仓库 profile
输出已实际消费，Ninja 真实触发原生 GN 自动再生，再生前后 toolchain 字节一致、
无 prefix patch，DLL 无工作门与三项项目头 mtime 失效/恢复门通过。C++/资源依赖
5/24/24；Clang `/showIncludes:user` 排除系统头，不与 MSVC 的 121 混同比较，
不据此声明 SDK 全内容自动失效。初次 tools/win 缺失及 CLI 兄弟导入失败均保留。

新 DLL 的 x64、52 必需导出和无动态 MSVC CRT 检查通过；复用已绑定公共库外部
驱动的显式夹具在硬件/WARP 六 context、18 次像素对照、12 成功 child/两无效
GLSL 拒绝通过，像素零差异、child 父进程不加载 ANGLE。不重编或重标旧产品 EXE，
不把外部驱动冒充实际应用/Loom 服务。证据在 linshi/otter-native-v330-regression，
准备与消费命令/摘要见 [ANGLE_TOOLCHAIN.md](ANGLE_TOOLCHAIN.md)。

此固定 Clang profile 的 GN 再生边界已验证，不再沿用 v329“此边界未完成”作为
当前停点；完整源码/工具 staging owner、内容/许可闭包与可信安装仍未完成。
SDK 仅在用户获准的 linshi 隔离构建范围内使用，原 checkout、用户程序及默认
Installation=None/关闭 Shader capability 不变。完整 N01–N12、JavaScript 呈现、
实际服务、资源长稳和最终手测继续开放；v324 UIA/v325 间歇启动失败不追溯放行。

构建合同合计 35 passed / 0 failed / 0 skipped，Rust fmt、版本一致性与全库
3,426 文件行数 ratchet 通过，无新增例外。未重跑完整桌面库，未组包或提交推送。

## v329：真实 SDK/GN、固定资源工具与 DLL 夹具重放（不是完整迁移放行）

用户明确授权后，已验签 SDK common/x86/x64 只在 linshi 内展开；6,887 文件绑定
摘要，真实 adapter 环境完成两架构的 C++20/D3D import library/rc/mt 探针。
没有系统安装、注册表修改、SDK 分发或默认 DLL 激活。固定源码逐 Git blob 复制
到实验树，补齐真实 GN imports、DEPS 导出配置和固定 build commit 的 LASTCHANGE，
不执行 gclient/hooks，不改原 checkout。实验 profile 明确限定 x64 D3D11 desktop
Release /MT、无符号、MSVC/STL；精确根/配置断言取代发现及假 WDK 路径，仅此
profile 省去无关 CRT/debugger/DIA staging，不缩减应用 Shader 或 JavaScript 目标。

实际 GN 暴露空 runtime_dirs 位置参数被丢失；prepare_toolchain 接受精确
otter-no-runtime-dirs marker 后立即还原空值，不允许附加路径或近似值。新回归
先红后绿，连同显式 VSLANG 请求合同，adapter 为 25 passed / 0 failed / 0 skipped；
真实 GN 生成 111 targets /
102 files，libEGL/libGLESv2 的 402 条 Ninja dry-run 命令通过。GN trace 只有已审阅
的 setup/时间戳/CPU/内存数据脚本，无 vs_toolchain/vcvarsall。angle_common、
translator、libGLESv2_static、libEGL_static 目标已实际编译成功，但不称单个
archive 自包含或 warning-free：zlib 日志含忽略 Clang 选项的 D9002。

真实 MSVC 缺英文资源，VSLANG=1033 仍输出中文；首次 Ninja 对象的头依赖为 0。
保留中文原文的 post-GN prefix 适配后，同一对象记录 121 个头依赖，头文件 mtime
变化触发重编译计划、恢复后无工作。此生成文件接缝仍是 linshi 实验，尚须纳入
可复用 owner 并保护 GN 再生；不把初次静态编译当作增量构建门已通过。

后续已从官方固定对象获取并审查 Clang/Chromium rc，资源 wrapper 完整消费后，
两 ANGLE DLL 实际编译通过（482.688 秒），x64/52 项所需导出/无动态 MSVC CRT
检查通过。C++ 与两资源对象分别记录 121/24/24 头依赖，三项真实头文件 mtime
失效/恢复及全 DLL 无工作门通过。44 条 D9002 仍保留，不称最终 profile 全绿。
新鲜公共库外部驱动的硬件/WARP 六 context、18 次像素对照及 12 次成功隔离
child/两次无效 GLSL 拒绝通过，实际像素零差异，child 父进程不加载 ANGLE。
首次直接诊断的立即卸载 DLL 断言失败保留；上游 lazy EGL loader 保留 GLES 引用
直到进程退出，不改产品隔离策略。这些不是实际产品 EXE 或 Loom 服务验收。
完整 staging/build owner、GN 再生安全、工具/许可完整闭包、可信安装仍须收口；
没有 v329 新产品 EXE，不重标旧 EXE；本次未改原生及 adapter 源码，Rust/用户程序与
默认 Installation=None/关闭 capability 不变。完整 N01–N12、JavaScript、实际
服务、资源长稳及最终手测未完成；旧 UIA/启动间歇失败不追溯放行，未组包或提交推送。
入口及证据边界见 [ANGLE_TOOLCHAIN.md](ANGLE_TOOLCHAIN.md)，本轮回执在
linshi/otter-native-v329-regression，外部实验在 linshi/otter-angle-fixed-source-20261024。

## v328：ANGLE 显式工具链消费 adapter（合成数据门，不是 DLL 构建）

新增 [ANGLE 工具链入口](ANGLE_TOOLCHAIN.md)，在全新仓库外目录生成固定 Chromium
setup_toolchain 的适配副本和实际导入依赖，不修改原始源码。显式清单约束 SDK
10.0.26100.0、MSVC 版本、GN 根、架构、关键文件及隔离环境，拒绝缺失/不匹配，
不回退 vcvarsall、注册表或 ambient SDK。移除上游空 runtime_dirs 的 PATH 拼接，
防止 CWD 的 cl.exe 抢占；固定 gn_helpers 随输出绑定，不靠测试注入原始模块。

合成 SDK/MSVC 数据与已审阅上游 main、CLI/独立 Python 进程的 22 项合同通过；
原上游在相同准入期望下触发发现 tripwire 的失败证据保留。没有运行编译器/SDK 工具、
GN/ninja 或安装 SDK，没有真实 SDK layout/DLL/重放证明，外部片段尚未接入完整 GN
暂存树。关键 anchors 不是完整内容闭包或 ACL/并发恶意替换保护。证据位于
linshi/otter-native-v328-regression；原生 Rust 和用户程序未改，本轮不重跑原生运行门，
不将 v327 EXE 标为 v328。默认安装/capability、完整 N01–N12/JavaScript/长稳/最终
手测仍未完成，旧 UIA/启动间歇失败不追溯放行，未组包、提交或推送。

## v327：Shader 强制模式与真实图片端口冲突提前拒绝

loom_art_config 的 resolved/Parameters 两条 Shader 快照入口现在均在写入强制
mode/output_mode 前检查真实图片端口；重名返回原有 InvalidValue，即使没有声明
同名参数也拒绝。普通 Art 仍允许这些图片名称，同名标量仍由图片替代；不丢图片、
不改名、不取消 Shader 强制模式，也不放宽 Loom adapter 的 inputs/parameters
合并检查。application_shader_mode_tests 复用隐藏应用夹具，覆盖两种单名及组合，
证明配置捕获失败先于输入 worker；该调度入口保留旧注册令牌/preview/正式输出，
不以 busy 门掩盖拒绝，不改变原有显式取消语义。没有新生产模块、线程、队列或句柄。

新回归先在旧实现精确失败（0 passed / 1 failed），修复后协议/配置门 13 passed，
Shader 定向 60 passed / 15 ignored。硬件 D3D11/WARP 应用平台门 1 passed，16 次
真实 child 中 8 次认证取码、8 次缓存无新连接；这是本地 TLS/ANGLE 夹具，不是实际
Loom 服务验收。最终 EXE 在集成编译完成后先绑定摘要，再无编译运行资源/EXE/会话/
设置恢复/设置启动/startup，结果 2/1/9/3/4/7 passed，原启动 15 秒期限未改。
Clippy all-targets -D warnings、fmt、3417 文件行数与 diff check 通过，修改 Rust
均不超过 500 有效行，源码及最终 EXE 在验证期间不变；证据位于
linshi/otter-native-v327-regression。定向门存在交集，不相加当作完整库测试数。
完整桌面库未重跑，v324 UIA 和 v325 间歇超时原证据保留，不声称本轮修复了它们。
默认 Installation=None/Shader capability 仍关闭；可信供应链/安装、实际服务、
完整 N01–N12/JavaScript/资源长稳和最终手测仍未完成，未组包、替换程序或提交推送。

## v326：匿名文件事务计时与原期限启动复验（不是性能修复）

session_file_trace 在现有 OTTER_NATIVE_SMOKE_TIMING=1 下记录静态阶段、匿名 ID、
字节数、耗时及成功状态；默认关闭，不输出路径/内容/错误文本，不新增线程或队列。
诊断输出失败不改变事务结果；完整基线比较、取消、共享限制、sync_all、无覆盖发布、
恢复与清理顺序保留。一次诊断 hidden smoke 11.547 秒正常退出，231 个文件操作完整
配对，操作耗时合计 183.739 ms（并发和而非墙钟）；baseline-read 合计 0.603 ms，
sync 最大 10.090 ms。该样本不支持移除校验/同步，也没有确证 v325 超时的根因。

文件事务 29、capture-file 5、实际保存调用方 19 项通过；settings-update 初次 filter
为 0 tests，不计覆盖。正式资源/EXE/会话/设置恢复/设置启动/startup 为 2/1/9/3/4/7
passed，启动仍用原 15 秒期限。严格 all-target Clippy、fmt、3416 文件行数及 diff
check 通过。Cargo 集成编译改变产品 EXE 摘要，原收尾身份检查失败被保留；随后绑定
最终 EXE 摘要，无编译直跑上述六套 26 项全部通过且 EXE 不变，源码前后相同。
证据位于 linshi/otter-native-v326-regression；诊断 EXE 与最终 EXE 分别绑定，不混用。
本轮没有性能修复，不能以复验通过宣称旧间歇超时或 v324 UIA/外部前台问题已修复。
完整桌面库未重跑，默认安装/Shader capability 仍关闭；完整 N01–N12、JavaScript、
正式供应链、实际服务、资源长稳和最终手测继续未完成，未组包或提交推送。

## v325：有界 Shader 配方缓存与无网络本地重渲染（Shader 定向通过，启动门未通过）

可信纯 Shader 的首次认证取码、真实 child 渲染、PNG 准备和正式提交全部成功后，
应用才缓存原始 vertex/fragment、uniform 初值及响应纹理/LUT；参数覆盖不写回配方。
后续菜单/参数执行仍通过原输入准备、请求/配置/来源/拓扑检查，再将当前图片和参数
绑定到配方，经原受限 child 和正式结果事务提交；命中不创建远端 Session/Delivery、
不发送执行或取消/租约控制。hybrid/workflow 的正式远端执行门没有放宽。

shader_recipe_cache 是 UI 线程唯一 LRU owner，最多 16 项、64 MiB 保守 payload
字节额度。键包含 Unit、Selection、Art、catalog 实例和可选参考上下文，而非参数
revision；窗口关闭/选择或目录更换/安装关闭会清理条目。worker 持有 Recipe Arc 及字节租约，
驱逐不能提前归还仍在使用的额度；不保留输入、像素、GPU 程序或 child。额度不是
整个进程、catalog、驱动或编译器内存硬上限。

参考上下文对齐旧预览启发式和图片端口筛选，reference 名称匹配不区分大小写。
需要上下文时，仅在原输入 worker 内对已准备主图/参考 PNG 的尺寸及 Base64 分块
计算 SHA-256；缺少摘要不缓存、不回退独立配方。摘要不是来源授权/新鲜度凭证。
无参考依赖的配方可对新输入本地重渲染，有参考依赖的主图/参考内容变化则不命中。
本地 worker 上限与统一 Tasks 两槽对齐为 2，实际 child 池仍只容纳一个 child；
容量等待仍计入原 15 秒期限，不保证两个任务在任何工作负载下都成功。

最终源码 Shader 定向 57 passed / 15 ignored；应用平台 16 次真实 child，其中
8 次认证取码、8 次缓存命中且 fixture listener 无新连接，均验证 PNG/显示提交。
双缓存任务在硬件/WARP 共四场景通过：两项同时准入、单 child 容量及取消隔离，
取消者须在容量释放前以 Cancelled 回收，另外六项真实 child 正式提交。输入
worker 15 项、严格 all-target Clippy、fmt、3415 文件行数及 diff check 通过，
修改 Rust 模块均不超过 500 有效行；首轮 Clippy 的大 Completion 枚举问题已用
可选配方元数据的 Box 间接持有修正，原失败日志保留，未屏蔽警告。原生源码前后
相同，最终 EXE 另绑定摘要通过六项产品/后端、两项本地平台和两种私有入口拒绝。
资源/EXE/会话启动为 2/1/9 passed；startup 为 5 passed / 2 failed，隐藏窗口与
透明钉图冒烟各在原 15 秒期限超时，无 stderr，WIC 图片冒烟成功，根因未确证。
启用既有阶段计时后，两项各一次精确隔离诊断仍超时：隐藏窗口的 application 阶段
在 12766 ms 成功结束且基础绘制/重建标记已打印，透明钉图最后计时为 groups begin。
阶段时长随运行变化，尚不能确认单一阻塞点或与缓存/v324 前台问题的因果关系。
不放宽期限、不将 Shader 定向成功当成启动门通过；证据及 partial 回执位于
linshi/otter-native-v325-regression，首轮 Clippy 与完整原始启动失败均保留。

本轮完整桌面库未重跑，不继承 v324 完整库成绩；此前 UIA/外部前台失败证据保留，
不操纵用户应用来取得全绿。默认 Installation=None、Shader capability 关闭。
这只是配方缓存，不是 GPU 编译程序复用、常驻 renderer 或完整连续拖动实时预览。
正式供应链/安装、实际 Loom 服务、源资产保真、完整 N01–N12/JavaScript/资源长稳
和最终人工验收仍未完成；未组包、替换程序或提交推送。

## v324：公共参数的本地 Unit 图片引用（定向验证通过，完整桌面回归未通过）

公共字符串参数现在可逐字匹配当前 workspace 的 Unit ID，包括保留的历史未声明
参数，不以 image_link/file widget 推断权限。application_shader_inputs 仅负责身份
筛选及正式 Graph 输出选择；图片仍归 application_image_inputs 的同一 Snapshot/
Origin/Graph→Source→PreparedBatch 链，不读路径、URL 或显示 resolver。Art 必须
具有正式 output 图片；Sticker 保留原中继/编辑/自身底图语义，Live 仍走原帧来源。
主图、input/reference、secret、内部和禁用字段不成为辅助纹理；未知字符串被忽略。

同轮 resolved Params 同时决定图片候选、远端配置和本地 Runtime，避免参数标量连线
已改变值时仍使用旧手动 inline 图。真实同名图片连接优先；候选 Unit 可进入短期图
视图，但只有实际使用的引用及其上游进入 Source freshness，遮蔽的 Unit 草稿/配置
不支配连接图片的生命周期。实际来源的 Node/Output/Live 凭证、拓扑/参数校验、
32 输入/31 辅助纹理与既有像素预算不变；不新增线程、队列、缓存或网络 owner。

定向核验当前本地 Loom adapter 发现 inputs 与 parameters 重名会被拒绝，原
loom_art_config 的三个编码接缝现统一省略同名图片参数，本地 Runtime 仍使用完整
resolved 值；TLS 夹具同步检查该约束。Unit 纹理不提升 inputs/params 与强制
mode/output_mode 名称。adapter 接受附加图片，但 framework 按 tool.params 声明
把它分入 params 或 inputs，并不通用地转换为文件路径；实际 Art 消费仍需独立
验收。这是绑定本地 Loom 源码摘要的静态合同核验，不是实际服务执行证明。通用
配置 API 对显式声明 mode/output_mode 图片端口的强制模式冲突未在本增量解决。

新增 application_shader_unit_tests 复用原 Class/COM/TLS 夹具，覆盖历史未声明、
声明图片参数和标量连线覆盖旧 inline 三类来源，未连接目标的 Sticker→Art 正式
PNG、中继配置/草稿/断线、新 OutputStamp、参数替换与 preview-only Art 拒绝。
真实硬件/WARP 平台门扩展 Unit 参数的菜单/最终参数两次执行。最终源码的 Shader
定向门 50 passed / 14 ignored / 0 failed，应用平台门 1 passed（16 次真实 child）；
Clippy all-targets -D warnings、资源/EXE/会话启动/startup 2/1/9/7（startup
33.32 秒）、fmt、3411 文件行数与 diff check 通过，修改 Rust 模块均不超过 500
有效行。原生源码前后摘要一致；最终 EXE 另绑定摘要，六项产品/后端、两项本地
平台及两种私有入口拒绝复验通过。证据位于 linshi/otter-native-v324-regression，
使用 verification-partial.json / closeout-partial.json，不生成全绿回执。

完整库两次分别为 3181 passed / 5 failed / 73 ignored（231.98 秒）和 3185 passed /
1 failed / 73 ignored（284.23 秒），未通过完整桌面门。首轮 wire 的旧 JSON 期望已
随同名参数规则修正；另四项 Surface Number/activation 隔离重验通过，但失败根因
未确证、未声称修复。第二轮 UIA select 重定位/展开断言失败，精确隔离复验仍失败，
日志记录外部 TFTTencentClient-Win64-Shipping 成为前台；未关闭或操纵该用户应用。
两轮完整库间生产源码不变；后续仅有 wire 期望和 Clippy 所需测试类型别名整理，
最终别名整理后未再运行完整库。定向/隔离通过不能替代完整回归或宣称间歇问题已修复。
收尾复查本次原生测试/产品/worker 进程及原生运行临时文件均无残留，Node 编译缓存
另行分类保留；不反复抢占前台重跑，不把外部前台证据解释为所有失败的共同根因。

默认 Installation=None、Shader capability 关闭。固定依赖/工具/SDK 的隔离构建、
DLL/许可/SBOM/安装、实际 Loom 服务、实时缓存、源资产保真、完整 N01–N12/
JavaScript/资源长稳和最终人工验收仍未完成；不组包、替换程序或提交推送。

## v323：Shader 图连接图片参数与内联 PNG 原字节保留

公共 image_link/file 参数若具有同名已准备图片输入，现由 shader_parameters 冻结
绑定名称，shader_prepare 从同一 PreparedBatch 取 PNG，优先于手动内联值与取码
响应初值。主输入、reference、secret、内部/禁用字段和未声明参数不自动成为纹理；
已捕获绑定若在批次中缺失则失败，不回退至响应 URI。合并纹理仍限 31 个辅助项，
共享既有解码/像素预算、任务槽、取消及正式提交 owner，没有新线程、队列或网络来源。
这接通已声明图片输入的图连接参数，不是任意 Unit 字符串、未声明图片参数或实时缓存。

新增低 alpha/透明隐藏 RGB 回归实际发现 Data::Encoded 的 PNG 会经 PBGRA 再导出，
造成颜色量化。该路径现复用 ImageValue 的同份 PNG 解码 owner 和 Formal 准备链，
既验证显示图像，又保留原 PNG 字节并计入像素加 PNG 的原有总预算；其他格式仍走
原转换路径。没有放宽像素断言；原 PNG 中的附加元数据一并保留，不据此宣称完整
profile/方向/动画兼容，也不能恢复已仅剩 PBGRA 的自身底图原始源颜色。

普通回归覆盖保密/禁用/端口筛选、数量并集、缺图/取消、原 PNG 字节与预算拒绝；
应用夹具新增同像素新 OutputStamp、断线拓扑的准备后拒绝，并在真实硬件/WARP
平台门增加图纹理的菜单/参数两次执行，独立验证实际发送的 PNG 及最终 PNG/显示。
v323 完整库复测 3181 passed / 73 ignored / 0 failed（269.14 秒），普通 Shader
45 passed / 14 ignored，纹理准备 6、应用菜单/参数平台 1（12 次真实 child）通过。
Clippy all-targets -D warnings、资源/EXE/会话启动/startup 2/1/9/7（startup
33.43 秒）、fmt、3408 文件行数与 diff check 通过；修改 Rust 模块均不超过 500
有效行。原生源码前后一致，最终 EXE 另绑定摘要复验六产品后端、两个本地平台及
私有入口拒绝；回执位于 linshi/otter-native-v323-regression。

首轮完整库为 3175 passed / 6 failed / 73 ignored，lib-initial.log 与原 gates
均保留：长截图收到屏幕布局变化后取消，菜单检测到前台变化，另有两项 UIA 事件
断言失败。同份源码的六项隔离重验及后续完整库通过；隔离成绩不替代完整库，
也不宣称 UIA 的异步重复/时序敏感断言已修复。

固定供应链也取得实际增量：ANGLE、depot_tools 及固定 build/buildtools/jsoncpp
源码的 commit/tree 已对照官方 HTTPS 记录，三份 ANGLE 原文和三个 build 工具链
文件逐字节对照成功。首次 Windows text=auto 转 CRLF 的失败现场保留，恢复使用
首 checkout 前固定 core.eol=lf 的新目录，而非跳过摘要检查。递归 DEPS 与 bootstrap
审查确认 DEPOT_TOOLS_UPDATE=0 不能阻止 CIPD/vpython 下载；未来同步须同时审查
--nohooks 与 --noprehooks。证据位于 linshi/otter-angle-fixed-source-20261024。
另按父 DEPS 固定 revision 获取 jsoncpp 实现源码，不使用 wrapper 的 dummy master，
第六个 checkout 的官方 commit/tree 和 clean 状态已核验。固定 SDK 10.0.26100.1
的 common/x64 NuGet 包已实际下载，dotnet nuget verify --all 均验证 Microsoft
作者签名及 NuGet.org 副署；ZIP 清单确认内部 10.0.26100.0 头文件和 x64 库。
这不是已安装/可构建的隔离 SDK：固定 setup_toolchain 仍调用 vcvarsall 指定版本，
本机 VS 会重新从注册表发现 SDK，仅设置 GN SDK 路径不足。尚未执行包内工具、
接受新增许可、安装 SDK 或修改上游；完整依赖/CIPD 实例闭包、显式环境 adapter、
DLL 构建/重放与发行批准仍未完成。

Application 默认 Installation=None，Shader capability 仍关闭；实际 Loom 服务、
完整 N01–N12/JavaScript/长稳/最终人工验收不缩减。未组包、替换用户程序、提交
或推送，最新实际已验证 ZIP 仍为 v305。

## v322：Shader 自身底图回退与预览隔离

可信 Shader 应用链在声明主图和 input/input_image/image 均无可用准备输入时，按旧
controller 最后的 data.src 分支回退节点自身底图；采用能力声明主端口，未声明时
使用 input。原有连接/内联主输入优先，普通 Art 不获得此回退，缺失参考图等其它
图片仍阻止自动触发。回退复用原输入准备 worker、PNG 编码与 PreparedBatch，本地
Shader 解码的仍是发给取码上下文的同份输入，不新增任务槽、线程或 IO 通道。

自身 src 只能来自编辑 owner 的真实 raster base，不能从 Results.display、resolved
display 或 Canvas.image 的 preview-only 恢复画面反推。capture_base 复用原来源准入，
保留真实文档身份/版本，但不把标注、裁剪覆盖层再次烘焙进 u_input；它们仍归原
显示/覆盖层 owner。共享底图不复制像素，编码只在原后台 worker 中执行，草稿、
未完成接管或准备后的文档替换继续阻止执行/提交，不能仅凭同像素/同版本放行。

普通合同覆盖原始尺寸/像素不含标注裁剪、缺参考图、preview-only 恢复、普通 Art
隔离及同像素新文档拒绝。应用平台门同时覆盖“有连接但自身底图不同”和“无主
连接但有真实自身底图”，并放入不同的 display-only 图像；硬件/WARP 各执行菜单
和最终参数触发，真实认证 TLS/child 到正式 PNG/显示提交。本轮自身底图用例为
不透明像素，复用既有 PBGRA→PNG 导出；不将正式输出的完整透明矩阵成绩挪用为
自身源资产的原始 straight RGB/透明隐藏色、profile/方向无损恢复证明，这些仍需
后续源资产保真门。v321 已完成最终源码/EXE/进程收尾。

完整库 3177 passed / 73 ignored / 0 failed（230.43 秒）；普通 Shader 41 passed /
14 ignored，认证取码组合 4、纹理准备 4、应用菜单/参数平台 1（八次真实 child）、
ANGLE 6、产品 child/图像桥接 6、本地平台 2、WebP 21、SVG 69 passed。Clippy
all-targets -D warnings、资源/EXE/会话启动/startup 2/1/9/7、fmt、3407 文件行数
和 diff check 通过；startup 合计 24.65 秒。前后原生源码身份一致，最终 EXE 另
绑定摘要复验后端、本地平台和私有入口拒绝，证据位于 linshi/otter-native-v322-regression。

正式 ANGLE 来源/可重放构建/许可证/SBOM/安装、实际 Loom 服务、通用图片与 Unit/
graph-image 参数、缓存式本地实时重渲染仍未完成。应用默认 Installation=None，
Shader capability 继续关闭；完整 N01–N12/JavaScript/资源长稳/最终手测不缩减，
未组包、替换用户程序、提交或推送，最新已验证 ZIP 仍为 v305。

供应链下一边界已联网读取固定 ANGLE commit 的 DevSetup/DEPS/LICENSE 和 DEPS
所钉 build revision 的工具链配置；固定构建脚本要求 SDK 10.0.26100.0，本机当前
仅发现 10.0.22621.0。原文/URL/摘要保存在 linshi/otter-angle-source-research-v322。
没有下载/执行新 DLL、获取完整源码、运行上游构建脚本或修改机器工具链；源码
标识存在不证明当前测试 DLL 的渠道或未修改构建，尚未取得发行供应链批准。

## v321：冻结公共参数接入菜单、参数触发与应用正式提交

应用在同一次 `with_parameters` 解析中同时冻结取码 Configuration 与本地 Runtime；
连线标量转换/夹取、secret/内部字段筛选与参数依赖凭证继续共用原 owner。Shader
配置强制 mode/output_mode，禁用列表不与强制字段冲突；PNG/数值快照随 Pending
输入准备和带注册代次的 Execution 单次移交，不在联网前重新读取 UI 参数。
`Tasks::start`/原 Job 直接消费可选 Shader 描述，取代独立的开发期取码入口，原网络、
两槽、释放/取消、完成通知、应用提交事务和失败原子性不新增 owner。

`shader_parameters` 对齐旧 ShaderPreview 的参数优先级：number 转有限 f32，bool
转 0/1，跳过 __* 和 reference；内联静态 PNG 参数覆盖同名 response texture，
不再依赖两个异步加载的完成顺序。文本枚举/非 data 字符串不读取网络或文件，
非 PNG data URI 明确拒绝，参数 input 纹理覆盖拒绝以保住同份 PreparedBatch 来源。
参数纹理计数/累计文本与合并绑定继续受限；冻结 String 和远端 JSON 的分配不属于
Pixels/child Job 的硬内存保护。主端口按能力 shaderInput、命名/首个图像声明推导，
仅在同份已准备输入中按旧调用链回退 input/input_image/image，不自动绑定 reference。

只有持有可信本地 Installation 且为纯 Shader、非 Surface、无需另做正式执行时，
应用才解除原 ShaderOnly 门；参数/上游自动开关仍先验。Installation 只是可信 owner
提供的描述，不验证来源/ACL/许可证，也不是安装器；应用构造仍为 None，没有用户
设置、环境变量或 Art 数据可绕过此门，用户 capability 继续关闭。

隐藏 HWND 的普通门验证未安装、自动参数关闭和配置变更均在联网前拒绝；已成功远端
终态后的本地不支持结果正常失败。显式硬件/WARP 各验证菜单与第二次数值/布尔参数
状态，经真实 exporter 认证 TLS、正式 input/reference、连线数值夹取、冻结 PNG
覆盖、真实 child 与应用 pending_windows 提交，显示和 PNG 解码像素精确。
这使用测试 EXE child 和自建 peer，不冒充实际 Loom 服务、产品用户入口或完整阻塞
GetMessage 循环验收。实际服务、可信 ANGLE 来源/构建/许可证/SBOM/安装、通用图片
和 Unit/graph-image 参数、旧自身图片 fallback 及缓存式实时重渲染仍待完成；当前
每次最终参数触发仍是有界取码/child，不新增常驻渲染循环。

完整 N01–N12、JavaScript、资源长稳与最终人工验收不缩减。完整库 3174 passed /
73 ignored / 0 failed（213.16 秒）；普通 Shader 38 passed / 14 ignored，认证取码
组合 4、纹理准备 4、应用菜单/参数平台 1、ANGLE 6、产品 child/图像桥接 6、
本地平台 2、WebP 21、SVG 69 passed。Clippy all-targets -D warnings、资源/EXE/
会话启动/startup 2/1/9/7、fmt、3406 文件行数及 diff check 通过；startup 合计
21.30 秒。前后原生源码身份一致，最终 EXE 另绑定摘要重验后端、本地平台及私有
入口拒绝；证据位于 linshi/otter-native-v321-regression。未组包、替换用户程序、
提交或推送，最新已验证 ZIP 仍为 v305。

## v320：认证取码、同源 PNG 纹理与本地派生正式结果

`Tasks::start_shader_fetch` 在原远端 worker/两个槽内复用固定发现身份、TLS exporter
认证、Session、取消中断和租约补偿；成功取码且释放完成后才在同一 worker 进入
`shader_fetch_render`。原 Delivery 保留真实节点/请求身份；只在原批次已单次取走、
租约为空时安装完整本地派生 PNG 批次，不伪造远端释放确认或为本地渲染创建租约。
远端已成功终态后的本地失败/取消不再误发远端 cancel；未确认释放仍走原失败补偿。
正式提交继续重验注册代次和来源凭证，不给取码 JSON 本身正式图像传播权。

`shader_payload` 按原正式主端口优先级消费 value，接受直接 Shader 或 content[0].text
JSON；不搜索其它端口/content。direct Value 类型化前检查源码/映射上限，wrapper
直接用有计数上限的字段反序列化，不先展开完整第二棵 Value 树。源码每份 1 MiB，
最多 31 个支撑纹理/256 个 uniform，非法/重复名字、非有限值和 response.input 歧义
拒绝；这消除旧异步 response.input 与外部 input 的覆盖竞争，不声称旧实现已有此规则。
上游 JSON/字符串分配仍受消息上限而非 Pixels/child Job 保护，不声称父进程硬内存隔离。

`shader_prepare` 从发给 Loom 的同份 PreparedBatch PNG 解码 GPU input，支撑纹理只
接受内联静态 PNG；不读取路径、HTTP、用户 profile 或 Art 提供的 DLL。WIC 共用原
像素分配/转换 owner，但 Shader 显式取 straight RGBA，不冒充 PBGRA Image、不预乘
或另翻 Y。自然尺寸先验 8192/32 MP、剩余纹理总预算后才分配；uniform 先取响应初值，
再由可信调用方的数值快照覆盖。reference 传给真实取码上下文，不擅自当 u_reference。
真实旧 ShaderRenderer 的 PNG→DOM Image→texImage2D 完整 256² channel/alpha 矩阵
（包括透明隐藏 RGB）与 straight RGBA 摘要一致、零字节差异。

标准 inline_resource 输入已静态追踪真实 Loom adapter：保留键转 data URL，合并公共
parameters 后按 manifest 分流，再由 Color Transfer runtime 在自身 tempDir 物化。
样例正式端口为 input/reference，runtime 也接受本轮 TLS fixture 的 input_path/
reference_path 别名；旧 prefetch 的 parameters 路径也经 adapter 分流，不据字段位置
误判旧实现失效。此为生产源码合同核查，不是实际运行 Loom 服务的联合验收。

真实 TLS fixture 验证 exporter 证明/实例 header、模式强制、secret/disabled 筛选、
同份源图/参考图传输，再返回原样 Loom GLSL/LUT；硬件/WARP 的三组合同和完整 alpha
矩阵均进入同一任务集合/正式端口，PNG 解码像素和显示精确。另验证文档过期、非法
payload/纹理、本地失败不误 cancel、网络取消与释放失败仍持有 lease/占槽。此新组合
门用测试 EXE child；产品 EXE 后端仍由原六项平台门另验，不冒充产品用户入口验收。

用户入口、动态参数/字符串纹理集成、其它图片格式及 profile/方向合同、实际 Loom
服务验收、正式 ANGLE 来源/可重放构建/许可证/SBOM/审计/安装 owner 仍未完成，
capability 与 pure-shader 拒绝继续关闭。完整 N01–N12、JavaScript、资源长稳和最终
手测不缩减；没有组包、替换用户程序、提交或推送，最新已验证 ZIP 仍为 v305。
完整库 3169 passed / 72 ignored / 0 failed（203.75 秒）；普通 Shader 33 passed /
13 ignored，认证取码组合 4、纹理准备 4、显式 ANGLE 6、产品 child/桥接平台 6、
本地平台 2、WebP 21、SVG 69 passed。Clippy all-targets -D warnings、资源/EXE/
会话启动/startup 2/1/9/7、fmt、3403 文件行数和 diff check 通过；startup 合计
22.41 秒。前后原生源码身份一致，最终产品 EXE 另绑定摘要重验六项后端、两个
本地平台门及私有入口拒绝，证据位于 linshi/otter-native-v320-regression。

## v319：本地 Shader 共用执行生命周期与应用提交事务

`shader_process::OwnedInvocation` 拥有跨线程源码/绑定，纹理共享预算像素，可信本地
bundle 与 Art 内容保持分离；线程启动前复用原 wire 几何、源码、名字、uniform
和字节准入，不复制完整纹理组装帧。`shader_execution_job` 仅按需创建一个父侧
worker，经原受限 child 执行，child 退出后在同一 worker 准备 Image+原始 PNG。
启动前取消、执行后取消和迟到领取均不得发布成功；捕获纹理、输入和许可在完成
栅栏/通知前释放，关闭统一取消并 join，不增加常驻渲染或第二套应用队列。

`loom_execution_tasks` 的同一两个槽现在显式区分 Remote 与 Shader，包括退休中
worker 和未领取完成结果；远端租约/取消补偿仍归原 owner，本地不伪造远端身份、
delivery、lease 或 cleanup 确认。两类共用唤醒、来源凭证、退休、领取、应用主循环
和正式结果事务；本地 Completion 独立保留节点/请求并只可消费一次。应用复用
原配置/拓扑绑定、来源 OutputStamp/文档凭证、注册代际和失败原子性，旧报告不得
清除替换请求的绑定或终止新请求。启动失败仍由调用方回收已注册请求，未偷偷注册。

五项普通生命周期测试覆盖准入、伪造/过期身份、替换/取消/关闭/编辑退休、容量、
未领取结果、异常、shutdown 和许可回收。隐藏 HWND 原应用链增加七个确定性场景：
当前、来源正式输出变化、目标文档替换、配置变化、拓扑变化、取消、请求替换；
实际经 pending_windows 主循环回收/提交，不只直接调用 Results::finish。替身 runner
只证明生命周期护栏，不冒充真实 GLSL。另两项显式平台测试在硬件/WARP 各验证
真实 child→共享任务集合→正式端口，以及真实 child→隐藏 Application 主循环→
正式端口/显示/中继，完整 alpha 矩阵 PNG/显示均精确；本地成功测试关闭下游通知，
不额外请求测试远端。真实本地任务测试用测试 EXE child，产品 EXE 后端另由原六门验证。
隐藏应用夹具显式驱动原 pending_windows 调度段；完成回调与真实线程消息队列另由
通知/队列回归保护，不把这些组合测试当用户交互或阻塞 GetMessage 循环的最终验收。

这是本地执行/提交生命周期接通，不是用户能力入口或 Loom 取码/纹理准备完成。
输入凭证只证明来源新鲜度，调用方还须从同份输入构造纹理；不能把任意 texture 与
凭证配对当同源证明。正式 bundle 来源/可重放构建/许可证/SBOM/审计和 Loom 获取/
staging 继续待完成；Shader capability/pure-shader 拒绝仍关闭。原 child 正常 15 秒
期限不变，WIC 阶段和内核/驱动清理无硬实时保证，Job 不是权限沙箱或全树/GPU 预算。
完整 N01–N12、JavaScript、资源长稳和最终手测不缩减；未组包、替换用户程序或
提交推送，最新已验证 ZIP 仍为 v305。

完整库 3160 passed / 71 ignored / 0 failed（201.90 秒）；普通 Shader 25 passed /
12 ignored，本地生命周期 5 passed / 2 ignored，显式本地平台 2、ANGLE 6、原真实
child/产品桥接平台 6、WebP 21、SVG 69 passed。Clippy all-targets -D warnings、
资源/EXE/会话启动/startup 2/1/9/7、fmt、3395 文件行数和 diff check 通过；startup
合计 23.17 秒，原每 child 15 秒上限不变。前后原生源码身份一致，证据位于
linshi/otter-native-v319-regression；最终 EXE 另绑定摘要重验全部六项后端平台门、
两个本地平台门和私有入口拒绝，不把不同 EXE 字节的证据混用。

## v318：正式图像保留原始 PNG，经下游输入与会话恢复不再量化

`loom_image_value` 成为正式图像的不可变 owner：显示仍使用 PBGRA `Image`，可选
原始 PNG 与像素一起共享；`shader_image::Prepared::into_batch` 转为本地正式批次，
不伪造远端 delivery/lease。`Results::finish` 的 output/output_image 别名、显示、
中继解析、传播准入和保存快照统一识别该 owner，原 OutputStamp 和请求代次仍生效。
预览不成为正式端口，失败/过期提交不改旧输出；正式预算按别名保守计入像素与 PNG。

下游输入 worker 直接将保留的 PNG 编为 Base64，会话后台保存复用同份 PNG，
恢复从同份受限内联字节解码后保留原字节；重复 Data URL 共享 owner。不读取路径、
网络或用户 profile，不在 UI 上编码。普通无原始 PNG 的正式图片走既有 WIC raw
owner，完整 alpha 矩阵证明解码像素与旧空编辑合成兼容；来源窗口标注不重复叠加。
内部 from_png 调用方负责像素/PNG 同源证明，不能把格式和几何检查当任意配对验证。

五项 owner/真实输入 worker 测试、实际会话编码→候选恢复→下游 inline 测试通过。
原隐藏 HWND/TLS 链 fixture 另临时安装完整矩阵 Shader 输出，验证显示/中继共享、
Application::image_inputs 的原 PNG 和传播 pending_start，再恢复原 fixture；
该新增 PNG 路径没有启动第二次网络请求，不冒充 PNG 新路径的 TLS 全链验收。
原真实产品 child 的矩阵/Loom/4K 桥接门增加正式批次共享 PNG 断言。

这补齐本地异步事务所需的数据 owner，不是应用 Shader 调度已接通；来源/配置/
拓扑/输出 stamp 的本地异步事务、Loom 取码/纹理和正式 ANGLE 供应链仍待完成。
Shader capability 与 pure-shader 拒绝继续关闭，完整 N01–N12、JavaScript、资源长稳
和最终手测不缩减。未组包、替换用户程序或提交推送，最新已验证 ZIP 仍为 v305。

完整库 3155 passed / 69 ignored / 0 failed（202.27 秒）；普通 Shader 20、显式
ANGLE 6、真实 child/产品桥接门 6、WebP 21、SVG 69 passed。Clippy all-targets
-D warnings、资源/EXE/会话启动/startup 2/1/9/7、fmt、3389 文件行数与 diff check
通过；startup 合计 26.67 秒，每 child 原 15 秒上限不变。原生源码前后一致，
证据位于 linshi/otter-native-v318-regression；最终产品 EXE 另绑定摘要重验平台门。

## v317：真实 Canvas alpha 合同到原生显示与 PNG

新增 `desktop::shader_image`，消费受限 child 的 top-down raw RGBA，一次准备原生
PBGRA `Image` 与可持久化 PNG，不发布应用状态、不创建线程或第二套编码器。
旧 WebGL2 Canvas 的 `premultipliedAlpha=true`，所以 raw 不能被当成 straight RGBA
再乘 alpha。显示字节为 `[min(B,a), min(G,a), min(R,a), a]`，alpha=0 必须清零；
PNG 则独立保留反预乘后的 straight BGRA，经原 `export_raw` WIC owner 编码，避免
PBGRA 再反预乘造成额外舍入。原普通 PBGRA 导出入口和取消/预算/大小上限不变。

实际旧生产 `ShaderRenderer.ts` 在隔离 Chromium 中渲染 256×256 矩阵，覆盖全部
channel/alpha 组合；立即重绘再 `toBlob("image/png")`，PNG 用独立 Pillow 解码。
整数最近舍入有 873 个 RGB 字节差异、数学 ties-to-even 有 417 个差异；逐步 f32
归一化、倒数、乘法和 ties-to-even 打包与 PNG 解码后的全部 straight RGBA 字节
精确一致，不放宽像素差异；不要求不同编码器的压缩文件/元数据二进制相同。
PNG→Canvas2D 后的 `getImageData` 又有 483 字节不同，不能误作编码
PNG straight oracle；旧三组合同中的 `legacy_png_rgba` 属于这一后值。

桥接在分配前重验 Shader 尺寸/完整像素，共享预算分别覆盖两份像素与 WIC 副本，
逐行及结束检查取消；编码失败、配额不足、转换中取消或 WIC 返回后的迟到取消
均不交付部分结果。raw 输入不被准备结果保留。WIC 内部缓存/编码流不是这些
逻辑像素租约的总内存保证，WIC 调用本身也不保证即时中断。

五项普通测试通过；显式真实产品 child × D3D11/WARP 的完整 alpha 矩阵、三组
Loom raw 合同组合 alpha oracle、3840×2160 identity→显示/PNG 均逐字节通过，
共十组桥接场景，PNG/alpha/方向没有容差。原三项 child 平台门一并重跑通过。
这些是后端准备闭环，不是应用图结果发布或用户桌面验收。正式 ANGLE 来源、
可重放构建/许可证/SBOM/审计、Loom 取码/纹理和带 stamp 的本地正式端口事务
继续待完成；Shader 能力报告和 pure-shader 拒绝保持关闭。完整 N01–N12、
JavaScript、资源长稳和最终手测不缩减。证据位于 linshi/otter-native-v317-regression；
未组包、替换用户程序或提交推送，最新已验证 ZIP 仍为 v305。

完整库 3149 passed / 69 ignored / 0 failed（202.13 秒）；Shader 普通门 19 passed /
10 ignored，显式 ANGLE 6、原有及新增真实 child/桥接平台门 6、WebP 21、SVG 69
passed。Clippy all-targets -D warnings、资源/EXE/会话启动/startup 2/1/9/7、fmt、
3386 文件行数与 diff check 通过；startup 合计 22.96 秒，每 child 原 15 秒上限
不变。原生源码前后身份一致；集成测试刷新后的最终产品 EXE 另绑定 SHA 并重跑
全部六项 Shader 平台门和直接入口拒绝探针，不混用不同 EXE 字节的证据。

## v316：Shader 独立 wire、受限 child 与真实产品 EXE

`shader_process` 将可信本地 bundle 与 Art 的原样源码/纹理/uniform 分离；父进程不
加载 ANGLE。`shader_worker` 在 settings/profile/UI 初始化前独占分流，先反查 Job，
再读取单请求并创建离屏 Renderer。固定 OSQ1/OSR1 协议独立于图片 PBGRA 协议：
成功只返回精确尺寸的 top-down raw RGBA，编译/link 失败返回有界 typed failure，
坏状态、截断、非零退出、额外字节或最终取消均不能发布成功帧。请求逐项计数，
在纹理预算/分配前检查剩余 body 及后续描述符最低尾部，拒绝短帧触发大额无效分配。

`bounded_worker_process` 统一 suspended→Job→resume、继承句柄白名单、容量、终止
与同步退出 owner；旧 WebP/SVG 保持单 child 128 MiB / 图片池 256 MiB 和空环境。
Shader 使用独立的单 child 256 MiB 池，一次至多一个，不扩大图片并发。一个绝对
15 秒正常期限覆盖容量/传输/退出；失败 Drop 终止 Job 并等待真正退出，但不承诺
包含内核/驱动清理的硬实时上界。Job 是资源/故障隔离，不是权限沙箱，也不代表
GPU 显存总上限。原 8192 / 32 MP、单纹理/输出 128 MiB 和总纹理 256 MiB 只是
wire 安全 ceiling，不能宣称所有合法组合均能在 256 MiB child 内成功执行。

真实 child 首轮暴露 D3D11 context 失败。独立同程序无 Job 诊断证明空环境仍失败，
仅补 SystemRoot 即通过，WINDIR-only 仍失败；未通过提高内存或只测 WARP 掩盖。
最终仅 Shader child 的环境包含由 Windows API 读取的真实 SystemRoot，不继承
父侧 SystemRoot、PATH、用户目录或任意环境。图片 child 的空环境合同不变。

显式平台门分别启动测试 EXE child 和同 target/profile 新构建的产品 EXE 私有入口，
均走同一 launcher/client。两入口 × 硬件/WARP × 三组真实旧浏览器/Loom 合同共
十二组 raw RGBA 逐字节相等；真实产品 3840×2160 identity 逐字节验证方向、全部
alpha 与 alpha=0 的 RGB，不缩小 4K 范围。API 返回后验证进程退出、容量租约归还
与父进程未加载 ANGLE，并读取独立 Job 峰值；这些不是整个进程树/GPU 的资源长稳
验收。测试 EXE 的停滞/超时/取消故障夹具另行覆盖，不能冒充产品驱动取消专项。

旧 Canvas PNG 与 raw RGBA 的 alpha 差异、PNG/PBGRA 桥接、正式 ANGLE 来源/构建/
许可证/SBOM/审计、Loom 取码/纹理和正式图端口发布仍未完成；测试 DLL 不入仓库
或包。Shader 能力报告与 pure-shader 拒绝保持不变，完整 N01–N12、JavaScript、
Shader 应用与资源长稳仍未完成。证据位于 linshi/otter-native-v316-regression；
未组包、替换用户程序或提交推送，最新已验证 ZIP 仍为 v305。

最终 Shader 普通门 14 passed / 7 ignored，原 ANGLE 门显式 6 passed，新真实 child/
产品 EXE 平台门显式 3 passed；WebP 21 passed / 4 ignored、SVG 69 passed / 4 ignored。
完整库 3144 passed / 66 ignored / 0 failed（202.48 秒），Clippy all-targets
-D warnings、资源/EXE/会话启动/startup 2/1/9/7、fmt、3383 文件行数和 diff check
通过；改动 Rust 模块均不超过 500 有效行。startup 合计 22.50 秒，每 child 原
15 秒期限不变。首次正式 4K 测量硬件/WARP 的 PeakJobMemoryUsed 分别为 235044864 /
187068416 字节，仅表示该次 child Job 的提交峰值，不含父进程/GPU 全树总量。
前后原生源码身份一致，真实产品 EXE 的构建/摘要与源码回执绑定；无 Job 和混用
settings 的直接入口均拒绝，隔离目录零文件。完整回执见同目录 verification.json。
集成测试重新生成产品 EXE 后，再次对最终 SHA 执行三项真实 Shader 平台门与
两项直接入口拒绝探针；保留刷新前后的两个 EXE 身份，不混用不同字节的回执。

## v315：原样 GLSL 的原生 ANGLE 后端与浏览器像素合同

新增独立 `shader_angle` 后端，以 EGL ES3 pbuffer 经 ANGLE/D3D11 离屏执行原样
GLSL ES 3.00；没有 WebView、浏览器运行时、HWND、固定滤镜替代或字符串改写。
`shader_angle_api` 只绑定固定 ABI，`shader_angle_library` 拥有两个 DLL 的可信
长度/摘要核验、只读共享文件锁、加载/卸载和实际模块路径复核；完整绝对路径加载
仅允许 System32 依赖搜索，先加载已核验 GLES，再加载 EGL，不搜索 PATH 或 bundle
中的额外依赖。调用方仍须提供可信 ACL 目录和可信清单，不能采用 Art 响应中的
DLL/指纹或把一次目录枚举当作抵抗并发写入的安全边界。

Renderer 是进程内唯一 EGL owner，且不可跨线程迁移；同线程和另一线程的第二个
owner 都明确拒绝，防止 current context 被替换或共享 display 被提前 terminate。
初始化失败、正常关闭及渲染事务均有 RAII 清理；每次渲染/描述查询重绑本 context，
任意 GL API 错误有界排空后永久 poison，不让后续合法请求继承上一事务的错误。
编译/link 错误、输入拒绝和取消与驱动错误分开，合法编译失败后仍验证可以重用。
图像长度/维度、名字、重复绑定、finite uniform、源码/纹理数量与逻辑像素预算
显式准入。预算不涵盖驱动/编译隐藏分配，GL 删除也不等于同步释放底层 GPU 内存。

真实旧生产 `ShaderRenderer.ts` 在独立临时 Chromium/SwiftShader 中生成对照，读取
只读 Loom Color Transfer 的实际 vertex/fragment、16³ LUT 和三组 uniform；输入
先核验 Canvas2D/PNG 往返不改字节，避免将不同输入的误差当作 GPU 误差。原生硬件
RTX 2080 Ti 和 WARP 各比较 identity、exposure/alpha、inverted-LUT/tone 三个案例，
本机六组 raw RGBA 全部逐字节相等，alpha 精确，最大差为零；复杂 tone 合同只允许
明确的一字节跨 GPU RGB 舍入，alpha 不放宽。另验证两后端各三次重建/每次八次
渲染、取消、无效 shader、uniform 类型错误 poison、DLL 错误摘要和写锁。

返回值明确是 top-down raw RGBA，不是 PBGRA，也不是旧 Canvas 导出 PNG。真实
旧 PNG 会将 raw `[64,32,16,128]` 变成 `[128,64,32,128]`，alpha=0 的非零 RGB
会归零；不能擅自预乘或把两种输出当成同一合同。显示/PNG 桥接仍须独立实现验证。
当前 DLL 仅为显式、固定摘要的本机测试夹具，未进入仓库、产品目录或分发包；
正式来源固定/可重放构建/许可证/SBOM/审计、隔离 child/Job/期限、Loom 请求与
结果发布链均未接通。驱动调用不能在线程内可靠中断，应用不得直接调用此后端。
Shader 能力报告和 pure-shader 派发拒绝保持不变；完整 N01–N12、JavaScript、
Shader 应用闭环、资源长稳和最终验收仍未完成，最新已验证 ZIP 仍为 v305。

最终定向 6 passed（显式包含两个平台门）；完整库 3133 passed / 61 ignored /
0 failed，Clippy all-targets -D warnings、资源/EXE/会话启动/startup 2/1/9/7、fmt、
3371 文件行数和 diff check 通过，改动 Rust 模块均不超过 500 有效行。startup
合计 22.29 秒，原每 child 15 秒期限未变；前后原生源码身份相同，回执位于
linshi/otter-native-v315-regression，测试进程零残留。未组包、替换用户程序或提交推送。

## v314：真实局部 WGC 输入闭环与投递 DPI 修复

实际启动自有源窗口的 WGC 后，局部裁剪的输入坐标先稳定失败：预期源子窗口收到
`[60,35]`，实际为 `[90,52]`。独立投递 oracle 又在发送端/接收端 Unaware、
SystemAware、PMv2 的九种组合中复现已映射坐标被二次缩放，例如 `[60,35]` 变为
`[40,23]`；不是只凭颜色变化推断输入正确，也没有把 worker 固定为 PMv2 掩盖问题。
最终 `live_input_message` 在鼠标消息发送期间复用 `input_coordinates::Dpi`，进入
接收窗口上下文，成功、失败或超时返回时恢复调用线程；键盘、授权、身份预检、
同线程拒绝、发送期限和清理顺序不变。DPI 上下文失败单独报告，不冒充送达成功。

新真实 WGC 用例覆盖 popup/overlapped 两种自有窗口，各运行正常停止、缩小源和
销毁源三种结束路径，共六个场景。正式 Session/输入 worker/恢复 child 验证固定
60×30 输出、源移动后的中心像素反馈、点击精确送达 `[60,35]`、源放大后保持裁剪，
以及持键结束时的释放和回收；源已销毁明确记 skipped，不谎报成功释放。停止或
缩小时验证先释放输入、再恢复原源窗口快照。真实 WGC 像素 oracle 读取中心颜色，
不是逐像素图像比较；精确 GPU 裁剪仍由原独立纹理测试保护。原整窗实际 WGC 的
焦点取消、持键/鼠标关闭释放和恢复用例重新通过，不复用修改夹具前的旧绿灯。

DPI 投递 1 项（九组合、横/纵滚轮及实际超时后上下文恢复）、实际 WGC 2 项、消息
超时/关闭 1 项、坐标 6 项定向通过。完整库 3129 passed / 59 ignored / 0 failed
（198.71 秒），Clippy all-targets -D warnings、资源/EXE/会话启动/startup 2/1/9/7、
fmt、3365 文件行数和 diff check 通过，无超过 500 有效行文件；startup 合计
23.42 秒，每 child 原 15 秒期限不变。原生源码前后身份一致，测试进程零残留，
回执位于 linshi/otter-native-v314-regression。首次全量脚本的 PowerShell 5 正常
stderr 处理问题已修正为读取实际退出码，不改产品、不放宽门禁，不计作产品测试通过。
这些是自有窗口与合成 Win32 消息证据，不是任意应用、物理热键、跨显示器移动、
资源长稳或完整 N01–N12 验收；JavaScript/Shader 等剩余任务不缩减。本轮未组包、
替换用户程序或提交推送，最新已验证 ZIP 仍为 v305。

## v313：同一 Live 选择表面与窗口局部像素绑定

Ctrl+2 的冻结选择表面支持拖动区域完成、双击悬停整窗和 Esc 取消；普通静态截图的
选区调整/Enter 合同不变。冻结前按 Z 序绑定有界窗口身份快照，排除自身、桌面/
任务栏/提示和 tool+noactivate 窗口，不读取标题。区域完全包含于候选窗口时生成
同一 Target 的局部 Crop；选区期间来源移动、缩放或关闭会拒绝旧选择，不按标题
重找、不改选下层窗口，也不静默回退为桌面采集。其他区域仍经单显示器屏幕准入。

Application 对整窗和局部裁剪共用源恢复保护、唯一 Session/worker 及关闭回收链。
Crop 的物理偏移同时进入 WGC GPU 裁剪和原输入 Backend，不按显示窗口位置反推。
局部像素随源移动而保持，窗口放大不拉伸裁剪；缩小到无法包含原区域时明确停止。
DWM frame 与 WGC item/内容尺寸必须一致，不猜阴影或边框偏移；resize 的旧尺寸帧
不发布，沿原预算事务重建帧池，最小化零内容仍等待后续事件。Live 选择只交付
矩形意图，不再复制一份无用的冻结裁剪像素；正常截图仍由原 capture_crop 交付。

开发侧已验证自有屏幕外 popup 与 overlapped 窗口的 WGC item 几何：物理 frame 与
item 分别为 200×100 和 182×91，无启动捕获；另验证隐藏源移动后的 crop 输入坐标、
缩小拒绝和线程 DPI 恢复。Live 来源选择 4 项、隐藏选择器 2 项、Crop/自有窗口
3 项（包含显式运行的两个平台用例）、WARP 精确 BGRA 裁剪/源池复用 1 项通过。
完整库 3129 passed / 57 ignored / 0 failed（198.94 秒），Clippy all-targets
-D warnings、资源/EXE/会话启动/startup 2/1/9/7、fmt、3363 文件行数及 diff check
通过，无超过 500 有效行的文件；startup 合计 24.99 秒，每 child 15 秒期限不变。
原生源码前后身份一致，回执位于 linshi/otter-native-v313-regression，测试进程零
残留。不据本轮通过声明历史 UIA 间歇根因已修复。WGC item 几何不是实际连续
捕获像素证据，不将这些测试当任意程序/浏览器、实际热键、可见桌面、
混合 DPI 或完整 Live 等价验收。完整 N01–N12、JavaScript/Shader 与资源长稳仍未
完成；未组包、替换用户程序或提交推送，最新已验证 ZIP 仍为 v305。

## v312：Ctrl+2 全局 Live 入口与三键所有权

固定 Ctrl+2 已接到既有 Live 区域选区生产入口，仍由 Ctrl+1 的唯一宿主注册，
不新增 hook、轮询或另一套采集实现。命令核验实际附属注册状态、精确键/修饰键、
宿主存活/启用、应用采集与模态状态、当前任务及 pending Live 请求；托盘恢复后
重新检查，最终由原 Live 入口持守卫、隐藏/恢复窗口并交给 Application 启动会话。
Ctrl+1 和 v311 活跃长截图的再次 Ctrl+3 路由保持原合同。

注册 owner 现在明确区分 Primary/Live/Long：两个附属键独立尝试，任一冲突不撤销
主键或跳过另一个；重试只补缺项。正常关闭失败时，恢复本次已释放的附属键，
一个补偿失败不阻止另一个，并记录准确降级状态；销毁则逐项独立清理全部键。
应用所有窗口共享菜单可用性，新建/恢复窗口复用同一状态；恢复来源/候选凭证
纳入 Live 注册和共享状态，移交成功只保留一个 owner，失败恢复旧三键状态。

旧命令映射缺少 Ctrl+2 已先红灯复现；global_capture 定向 11 项通过，覆盖两个
附属键冲突矩阵、关闭/补偿/销毁、共享菜单、真实隐藏 HWND 的准入和 pending
去重。最终选区回调注入，未注册或注入用户系统热键、未读取桌面或启动真实 WGC。
EXE 冒烟另核验应用三键切换、恢复移交/补偿及凭证变更拒绝。完整库 3125 passed /
55 ignored / 0 failed（200.00 秒），Clippy all-targets -D warnings、资源/EXE/会话启动/
startup 2/1/9/7、fmt、3358 文件行数与 diff check 通过，无超过 500 有效行文件。
startup 合计 27.49 秒，每 child 原 15 秒期限不变；前后源码身份相同，回执位于
linshi/otter-native-v312-regression。未据本轮通过声明历史 UIA 间歇原因已修复。
现有区域入口仍是屏幕区域 Live，不是旧版窗口内区域锚定/
双击整窗选择的完整等价；该合同继续待迁移，不能以屏幕区域或显式窗口菜单
替代。JavaScript/Shader、完整 N01–N12 和最终资源/桌面验收仍未完成。

## v311：长截图再次快捷键完成与在途任务收尾

补齐活跃长截图的 Ctrl+3 结束路由：注册宿主先向同一 Application 采集身份的
当前长截图排队通知，不再被新截图专用的独占/模态/禁用宿主门禁挡住。新会话
仍沿原入口进入固定垂直长截图；菜单开启的水平会话也能被同一结束路由领取。
共享采集身份、线程、HWND 属性和不回绕代次共同核验，至多一个通知在队列中；
销毁/关闭/成功交付退休路由，迟到消息不能结束下一会话。不新增 hook、线程或
空闲轮询，不接管其他 Application 或用户真实全局热键。

完成按钮与快捷键共用 Ready/Draining/Composing/Closed 状态：自动运行或后台任务
在途时仍可请求完成，先停止后续自动滚动，接收唯一当前任务后再提交原 worker
拼接；重复请求不启动第二次拼接，收尾期间拒绝其他采集、方向或取景框命令。
当前采集/重叠验证失败时保留会话并显示原错误，要求显式重试，不静默交付部分
结果；取消、关闭、销毁和显示布局变化仍丢弃未交付结果。沿用原拼接预算、首帧、
初始取景区 metadata、文档新鲜度、后台加载和窗口可见性恢复责任。

先加真实 worker 回归，旧代码实际失败于完成输出为 None；随后修改生产边界。
capture_input 定向 40 项通过，包含纵/横轴与手动/自动、重复请求、在途预览/二维
追加、采集错误和取消清理；真实屏幕外 DialogBox 消息循环验证禁用 owner 收到
合成 WM_HOTKEY 后，经生产路由、后台拼接、timer 和 EndDialog 返回图像。路由测试
另核验通知合并、失效采集身份、窗口属性替换及旧 owner 析构隔离。这不是实际
键盘注册/滚动应用或混合 DPI 桌面手测，也没有读取用户桌面或系统剪贴板。
完整库 3122 passed / 55 ignored / 0 failed（229.89 秒），Clippy all-targets
-D warnings、资源/EXE/会话启动/startup 2/1/9/7、fmt、3357 文件行数 ratchet 与
diff check 通过；没有超过 500 有效行的文件。startup 合计 26.94 秒，每 child
原 15 秒期限不变，不据单轮耗时声明性能改善。验证前后原生源码身份相同，
证据位于 linshi/otter-native-v311-regression。本轮未组包、替换用户程序或提交推送；
自动方向、用户滚轮驱动的自动录制、JavaScript/Shader 和完整 N01–N12 退出条件
仍未完成，最新已验证 ZIP 仍为 v305。先前间歇 UIA 问题本轮未出现，不称根因已修复。

## v310：原生图片能力进入真实挂载协商

检查实际 Loom 挂载合同发现：requiredNodes/requiredCapabilities 不满足会返回
surface_host_capability_missing；仅有图片实现但报告缺项，正常图片 Art 仍进不来。
在 v309 的真实 attach/active/GET/内联安装/inactive 与撤权组合链通过后，共用宿主
报告现增加 image、loom_resource transport 和 remote_resources capability；新建
和显式恢复使用同一报告。不从 Art 请求反向复制能力，也不改变 Scene runtime 校验。

image 仅对应已实现的有界静态解码/测量/显示合同，不承诺所有浏览器格式、动画、
CSS 或 HDR 行为。resourceId 优先不回退、原租约/摘要/字节预算、同 Grant 安装、
到期/撤权回收和受限 child 均不变；报告不是资源读取权限，不增加 http/file/blob
来源或脚本执行。scroll、touch、JavaScript/Shader runtime 仍未宣告。

先修改报告回归，在旧生产报告上确实失败于缺少 image；再接生产报告。TLS 图片
对端现在还独立检查网络请求中的 image/transport/capability/runtime，避免仅将报告
与自身函数比较的同源断言。报告定向 1 项、Application 图片/表单定向 17 项
通过；完整库 3114 passed / 55 ignored / 0 failed（216.68 秒），Clippy all-targets
-D warnings、资源/EXE/会话启动/startup 2/1/9/7、fmt、3352 文件行数 ratchet 和
diff check 通过。startup 总计 20.50 秒，各 child 原 15 秒期限不变；不据单轮时间
宣称性能改善。验证前后原生源码身份相同，证据位于 linshi/otter-native-v310-regression。
本轮未重新组包或替换用户运行程序，最新已验证 ZIP 仍为 v305。实际用户 Loom、
可见桌面/中文 IME/WGC/混合 DPI、资源与长稳及完整 N01–N12 验收仍未完成。

## v309：Application 自动表单调度与启动分段诊断

完整 poll 使用同一私有配置回调顺序准备表单输入与资源请求，生产仍默认
Config::from_env，退出泵保持原 wrapper。回调仍在可见/模态/退出/pending/active
以及 lease、device 校验之后调用；不提前读取、不跨 origin 缓存、不修改进程环境。
同一配置回调现在也贯穿初次挂载和 lifecycle activation/deactivation，正式入口
仍默认环境配置；后台请求、租约与设备校验继续由原 owner 执行，不新增认证通路。

新增代表性 input 应用闭环：真实本地 TLS challenge/续签取得 Application 授权，
真实 attach 响应创建隐藏窗口，接纳 active ACK 后才显示；真实 EDIT 消息由正常
poll 自动领取、batch POST、观察快照并结算 receipt；关闭后真实 inactive ACK 返还 owner。测试不构造
Outcome、不注入激活 ACK、不手工 begin_batch 或启动 batch worker。验证隐藏空闲/
可见空闲不准备配置，配置错误保留 pending 和 owner，重试只提交一次，权威响应
删除字段会销毁旧控件，多次空闲 poll 不再联网，关闭父窗口及 worker 均回收。
六类逐控件协议覆盖仍由 v308 组合测试承担，不将本次 input 当六类应用层全验。

图片组合链同样经过真实 TLS challenge/续签、attach、active ACK、带原租约的
资源 GET、内联 PNG 后台解码、正常 poll 安装/上传与 inactive ACK。明确验证
非空 resourceId 优先且不回退 src：缺少租约的资源节点不进入可见投影，不能将
其误断言为“可见且非 placeholder”。旧错误断言已在续进时复现，修正的是测试，
未改变生产来源优先级。撤权发生在资源完成但尚未安装时，丢弃结果、销毁 HWND，
不提交失去授权的 inactive，也不伪称远端 disposed。全部使用隔离身份与本地 TLS
对端，不连接用户 Loom；仍不扩 image/scroll 能力报告或真实服务端验收范围。

增加 OTTER_NATIVE_SMOKE_TIMING=1 显式诊断开关，仅 smoke 路径输出 stderr 的
固定阶段 begin/end、elapsed_ms 和 ok；默认不开启计时、不改变 stdout marker、
执行顺序、错误返回或正式15秒期限。覆盖Application、回收/恢复、粘贴/open/drop
边界，计时包含被调函数退出析构。它是诊断工具，不是放宽启动门禁。
续进验证：Application 定向 17 项通过，完整库 3114 passed / 55 ignored /
0 failed（247.44 秒）；先前 UIA EDIT 单项失败这次在全库中通过，不将历史间歇
原因标成已修复。Clippy all-targets -D warnings、资源/EXE/会话启动/startup
2/1/9/7 通过；startup 共 30.71 秒，每 child 原 15 秒期限未变。fmt、3352 文件
行数 ratchet、diff check、11 文件 UTF-8 无 BOM 通过。验证前后原生源码快照
完全相同；证据位于 linshi/otter-native-v309-resume。最新已验证 ZIP 仍为 v305。

## v308：声明式表单能力报告与实际挂载链一致

新建和显式恢复共用 desktop/surface_capabilities 的宿主报告，在既有布局/文本/
按钮节点之外明确声明 input、textarea、number、switch、select、slider。节点声明
表示现有有界原生表单合同，不等于全部 HTML/CSS/IME 体验兼容；runtime 仍仅
declarative，touch=false，transports/capabilities 不扩张，image/scroll/脚本/Shader
不随本轮宣告。报告发送给远端协商，不是客户端 Scene parser 的节点硬白名单。

组合回归复用现有本地 TLS fixture 与真实 attach worker，将响应交 AuthorizedWindow
授权准入并创建自有窗口；逐类输入经 Win32 消息、准备事件、唯一 owner 移交、真实
batch worker 和 HTTP POST，再从独立 stream 响应接纳 revision=4 场景；撤权检查
窗口/子控件销毁且不伪称远端 disposed。生命周期 activation ACK 在夹具中注入，
不宣称真实 Loom 服务端或 Application 自动调度全链验收；没有注入用户桌面输入。
六类 TLS 组合定向通过；首轮全库出现既有 Live 回收透明度断言失败，随后
48 项因同一共享锁 PoisonError 连锁失败。新表单组合当轮通过，首个失败发生在
它执行之前；Live 用例单独复测通过。未更改生产滚轮、断言容差或共享锁策略，
全库原条件重跑 3108 passed / 55 ignored / 0 failed（252.94 秒）。间歇原因尚未
确认，不把重跑通过当作修复；原始 lib.log 与复测 lib-recheck.log 均保留。
Clippy all-targets -D warnings 通过，资源/EXE/会话启动 2/1/9 通过。startup 两轮
均为 4 passed / 3 failed，三个完整 smoke 都触及现有 15 秒期限，停在不同阶段；
不放宽门禁、不重试到偶然全绿。独立诊断自有进程约12.7秒到达最终 destroyed
marker（该包装未捕获可靠 exit code，不称退出码验收）；之后系统 CPU 观测100%，
资源竞争仅是可能原因，未停止其他应用。正式启动门禁仍未通过。
fmt、3348 文件行数 ratchet、diff check、11 文件 UTF-8 无 BOM 检查通过。证据目录：
linshi/otter-native-v308-regression。最新已验证 ZIP 仍为 v305，v308 未组包。

## v307：显示快照复制为独立静态可编辑钉图

非 Art、非缩略窗口的 resolved display 复制不再统一扁平化；复用当前已解析的
Frame image/editor 快照，粘贴保留标注/裁剪并重建文档与标注身份。已有 raster
layers 保持原编辑底图，无 layers 的默认/overlay Editor 仅绑定对应显示图像作为
fallback base，不重排坐标、不覆盖分数 logical_size、不继承历史。这样避免新
State 安装无 layers Editor 后 sync_raster_source 把图像清空。

resolved 来源不领取目标 Canvas 的 capture metadata 或原图租约；当前 Frame 没有
来源租约合同，不能凭目标底图猜测。粘贴结果是普通静态钉图，不复制连线、Live
owner 或 Art 身份。既有帧替换/取消的新鲜度门禁不变，Art 和 mini 仍扁平。
这不是复制时强制刷新 Live 帧的完整等价；使用的是已解析显示快照。

先增加回归，旧门槛下两项 displayed 测试实际失败（editable=None），再修改生产
接线。新增真实 copy worker→注入发布→跨 State cache→paste worker→State 验证，
同时验证无 editor 的帧不会丢图、未知目标来源不传播、Art 仍清除可编辑凭证。
验证：copy_job 定向 14 项通过；全库 3107 passed / 55 ignored / 0 failed
（242.48 秒），Clippy all-targets -D warnings 通过，资源/EXE/会话启动/startup
分别 2/1/9/7 通过，startup 本轮未超时（36.54 秒）。fmt、3345 文件行数 ratchet、
diff check、本轮 12 文件 UTF-8 无 BOM 检查通过。证据位于
linshi/otter-native-v307-regression。v306 间歇启动超时记录仍保留，不据本轮通过
宣称根因已修复。未组包，最新已验证 ZIP 仍为 v305。

## v306：可编辑粘贴物理尺寸与提交后级联

静态展开单钉图复制时保存物理窗口尺寸、鼠标相对偏移与 UnitId；20 DIP 级联步长
按源窗口 DPI 转成物理像素。粘贴入口采样鼠标，后台不重新读取；交付时按 UnitId
查当前存活源矩形，仅用于 inside 判断，找不到则回退快照。边界沿用 Web inclusive
规则；级联坐标和尺寸仍取复制快照，不随源后续移动/缩放改写，不额外乘目标 DPI。

只有窗口暂存和 workspace 注册全部成功才推进锚点；排队请求在交付前刷新锚点，
失败/取消不消耗一步，新复制的 session identity 不被旧请求推进。不保存旧 HWND。
当前 editable 请求每批只有一张图；同 session 的批内多图级联不在本轮支持范围。
真实隐藏窗口 smoke 覆盖物理尺寸、连续交付刷新、建窗失败和新 copy 隔离，不访问
系统剪贴板。最新已验证 ZIP 仍为 v305，v306 未组包。

开发验证：placement 定向 5 项；全库 3105 passed / 55 ignored / 0 failed
（208.49 秒），Clippy all-targets -D warnings 通过，资源/EXE/会话启动 2/1/9 通过。
首次完整启动单独运行 7/7 通过；全库后的集成运行有一个完整 smoke 触及 15 秒
期限，日志停在既有 restored-publication 之后、尚未到新增 placement；其余 6 项通过。
保留失败日志，不修改期限，再次串行运行 startup 7/7 通过（41.28 秒）。间歇超时
原因尚未确认，不能把重跑成功视作已修复。fmt、3345 文件行数 ratchet、diff check
及本轮 18 文件 UTF-8 无 BOM 检查通过。日志：linshi/otter-native-v306-regression。

## v305：可编辑复制保留采集来源与原图租约

静态展开钉图的Content现在保存CaptureSource快照，保留有界capture metadata及
未知JSON字段、来源记录、诊断和只读原图租约。复制使用Canvas当前文档getter，
不读取已退休来源，不把本Canvas来源附到其他显示帧。每次粘贴仅克隆来源内容，
不复制源DocumentStamp；安装新Editor后由attach_capture绑定新文档身份。

原图Arc<File>租约与源文档、剪贴板单槽、粘贴文档共享，不重开来源路径、不复制
文件、不借用共享游标。源窗口关闭/图片替换或剪贴板失效不会提前释放仍由粘贴
文档持有的租约；最后一个owner退休后才能清理原文件。来源路径仍不是读写/删除
授权，没有租约的HDR metadata不会凭空恢复原图导出能力。capture_origin不复制，
原采集位置保留在metadata，不作为新窗口定位指令。

继续保留v304范围边界：Art/显示帧/缩略仍扁平，定位/级联、多节点和跨进程
可编辑协议未补齐。最新已验证ZIP已更新为v305：独立Release构建身份核验、
298文件归档/真正解包校验、隔离cwd/TEMP运行探针通过。smoke退出0、两类非法
参数退出1，TEMP零残留；新增editable/capture metadata标记在解包EXE中通过。
收据：linshi/otter-native-v305-runtime/verification.json。候选未签名、不可发布，
包内runtimeVerified=false不改写，不等于干净机器/全面功能或系统剪贴板验收。

开发验证：clipboard定向73项，全库3100 passed / 55 ignored / 0 failed（203.36秒）；
Clippy all-targets -D warnings通过；资源/EXE/会话启动/startup分别2/1/9/7通过。
三类完整startup均强制检查metadata未知字段与新文档绑定标记。原图租约测试
使用linshi中的真实只读共享句柄，验证持有期间写入/删除失败、最后释放后可写；
不宣称已完成系统剪贴板或真实HDR原图导出交互。13个改动文件UTF-8无BOM，
fmt/diff check和3342文件行数ratchet通过；日志：linshi/otter-native-v305-regression。

## v304：静态展开钉图复制粘贴保留编辑内容

复制 worker 在生成原 PNG/DIB/可选文件格式时保留同一已提交 Editor 快照；
只有最终持剪贴板锁、所有格式转移成功后取得的 sequence 才安装应用共享单槽。
最终序号失配返回 skipped，不把 Ok 当成真正发布；部分发布错误无成功凭证。
GetClipboardSequenceNumber 的官方合同已联网核对：内容变化/清空递增，权限不足
返回0；本路径立即呈现，不使用 delayed rendering。0不准入可编辑缓存。
参考：https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getclipboardsequencenumber

粘贴新钉图在锁内核对当前 sequence，只借共享 Content，锁外沿 open_job 后台
准备；每次新建 Editor document identity、annotation IDs、空撤销历史和 UnitId。
共享不可变像素，保留矢量、栅格标注、裁剪与底图透明度；不烘焙后重复画标注。
后台准备含栅格标注的命中区域并查取消；新窗口按裁剪尺寸/视口初始化，applied_crop
阻止首轮重复移动。复用原 staged-window/MAX_UNITS/取消交付，不创建旁路窗口。

本轮范围为进程内静态展开单钉图；Art/显示帧/缩略来源仍走既有扁平复制，
显式图片替换仍读取系统图像格式，editable replacement明确拒绝。跨进程编辑格式、
Live/缩略完整编辑状态、复制定位/级联偏移、capture metadata和多节点图仍未补齐。
外部剪贴板变化后旧Content在下一次匹配/发布时退休，期间仅有界保留一个槽；
未添加常驻轮询。最新实际ZIP仍为v302，v304尚未组包。

验证：clipboard定向71项、copy_job定向13项通过；全库3098 passed / 55 ignored
/ 0 failed（214.06秒）。首次Clippy暴露生产编译的smoke夹具误用cfg(test)专用
Pixels From<Vec>，已改用既有有预算bytes入口；首次日志保留，修复后
Clippy all-targets -D warnings通过。资源/EXE/会话启动/startup分别2/1/9/7通过。
真实隐藏窗口smoke覆盖重复粘贴、独立Unit/annotation/document身份、crop视口、
首轮crop sync不重复位移、opacity、撤销隔离与取消；不访问系统剪贴板。
日志：linshi/otter-native-v304-regression。自动化发布凭证为注入测试，不能当作
实际Windows剪贴板/跨应用交互验收；官方sequence文档也不替代该运行证据。
新增editable标记已进入三类完整startup的强制断言，独立复跑7 passed（26.95秒），
见startup-receipt.log。fmt/diff check及3341文件行数ratchet通过，无超过500有效
行项；本轮22个文件UTF-8无BOM，后续修改文件已复查。

## v303：全局 Ctrl+3 垂直长截图入口

固定 Ctrl+3 精确验证注册 ID、键值与修饰键后进入既有 LONG_VERTICAL 流程，
托盘恢复前后均复核 owner、模态、任务与待交付门禁；复用现有选区、隐藏恢复、
长截图对话框与新钉图交付，不引入键盘 hook 或另一套采集实现。

Ctrl+1 保持唯一 primary owner，Ctrl+3 为同 owner 的附属注册。附属键冲突不
牺牲普通截图，所有窗口菜单显示 Ctrl+3 不可用；底层只记录诊断，不在工作区
转交事务中弹窗。正常关闭先释放附属键；主键释放失败时恢复附属键，恢复失败
保留准确实际状态及双错误。销毁独立尝试两项清理。转交新 owner 允许重试附属
键，不承诺迁移原 exact 注册集合；恢复凭证同时冻结共享状态和本地实际两项状态。

本轮只实现固定垂直入口：Web 默认自动判断方向以及录制中再次 Ctrl+3 完成
尚未迁移；当前再次触发被模态/任务门禁拒绝，仍用原对话框完成或取消。
测试不抢占用户系统热键、不发送真实截图输入；真实注册冲突/可见交互待集中验收。
最新实际 Release ZIP 为 v302，v303 尚未组包。

开发侧验证：global_capture 定向8项通过；全库3092 passed / 55 ignored / 0 failed
（234.34秒）；Clippy all-targets -D warnings通过；资源/EXE/会话启动/startup
分别2/1/9/7 passed。日志：linshi/otter-native-v303-regression。
fmt与diff check通过；3338文件行数ratchet通过，无超过500有效行项；本轮20文件
UTF-8无BOM及尾空白检查通过。没有扩大真实按键、可见会话或完整兼容性验收范围。

## v302：textarea 盒模型与声明配色进入真实 EDIT

surface_textarea 为 textarea 补齐 Neuro 控件/正文/分隔线默认色、1 DIP 边框、
7 DIP 圆角和横9/纵6 DIP padding；保留显式覆盖。测量、布局最小尺寸、父 D2D
背景/边框与子 EDIT 投影使用同一解析结果。百分比 padding 使用包含块宽度；
圆角复用 Background 计算，不另造半径规则。固有高度计入一次上下 inset。

真实 EDIT 内缩边框，保留父 D2D 绘制；内容格式矩形在扣除实际滚动条后应用
padding，未变矩形不重复设置。内圆角与已有祖先/后绘制节点/滚动条遮挡区域
取交集，不绕过原输入裁剪。颜色在提交绘制帧时解析，CTLCOLOR 不重新解析 JSON。
EDIT/Select 共用 HWND palette owner 和 alpha 合成；EDIT 校验父窗口和自身
marker 后读取颜色，不依赖同步重入期间暂时移出的 Host。保留高对比系统回退、
stock DC_BRUSH 借用及销毁清理，不为每次绘制创建 brush。

边界：半透明背景暂对 Neuro 控件底色预合成，不是任意图片/父像素透明；GDI
圆角为整数 region，不能声称与 D2D 亚像素边缘逐像素等价。lineHeight、完整
透明度层、placeholder 白色背景角色和祖先对齐仍待后续；report 不扩大，
input/number 不顺带应用 textarea 默认样式。v302 Release ZIP 已构建并真正解包验证：
298文件校验，独立cwd/TEMP下smoke退出0，两类非法参数退出1，TEMP零残留。
收据位于linshi/otter-native-v302-runtime/verification.json；未签名、不可发布，
包内runtimeVerified=false不改写，不代表完整功能或干净机器验收。

验证：全库 3087 passed / 55 ignored / 0 failed，包含5项新增盒模型/配色测试；
Clippy all-targets -D warnings 通过，资源/EXE/会话启动分别2/1/9 passed。
首次 startup 为6 passed/1 failed：透明贴图完整 smoke 超过原15秒期限，runner
回收自有子进程并保留输出。未改代码/期限/断言，同一构建独立复跑7 passed，
耗时34.16秒；未复现不等于根因已修复，启动时延稳定性仍须后续观察。
fmt、diff check、3336文件行数ratchet通过，受检文件无超过500有效行项；本轮
24文件UTF-8无BOM。日志及首次失败：linshi/otter-native-v302-regression，
复跑另存startup-rerun.log。颜色像素断言在系统允许自定义颜色时运行，不修改
系统高对比设置；没有宣称所有DPI/滚动/销毁重入排列或真实透明合成已经验收。

## v301：textarea 正文与 placeholder 对齐

生产挂载映射 textarea 自身明确的 textAlign left/center/right；非法或缺省值
回到左对齐。只修改 EDIT 对齐位，保留多行/滚动/Tab/Return 位，不回写文字、
不主动结束 IME、不为呈现变化创建事件。无变化不调用 Win32 更新；同步样式
消息返回后复核 Control owner，再安排重绘。placeholder 从实际 EDIT 样式
读取同一对齐方向，仍按实际字体和格式矩形绘制，不变成字段值。

Web 的 SurfaceTextInput 并未应用自身 surfaceNodeStyle，textarea 才明确应用；
所以本轮不向 input/number 增加自身 textAlign。祖先 CSS 对齐的浏览器 computed
style 尚未核验：本轮 data URL 探针被浏览器工具策略拒绝，未绕过限制，不将
源码推断当实测。完整继承/lineHeight/圆角/透明合成仍待继续完成，report 不扩大。
字段自身样式变化仍使原编辑合同失效；同一合同重复同步保留可提交的本地草稿。
最新已打包候选仍为 v299。

验证：2 项真实 EDIT 像素/生产挂载定向测试通过；全库 3082 passed / 55 ignored /
0 failed；Clippy all-targets -D warnings 通过；资源、EXE、会话启动、程序启动
分别 2/1/9/7 passed。fmt、diff check、3331 文件行数 ratchet 通过，没有超过
500 有效行的受检文件；本轮 13 文件为 UTF-8 无 BOM。日志：
linshi/otter-native-v301-regression。像素验证来自 WM_PRINTCLIENT，不读取用户
桌面；未声称实际 IME 组合期间行为或样式消息中销毁的故障注入已验。

下一生产边界已由独立源码审查确定：textarea 的布局/父 D2D 样式与真实 EDIT
仍未统一，完整子窗口覆盖声明边框，默认 padding 未进入文字格式区域，固定
CTLCOLOR 忽略声明 color/background。应成片补齐 box/配色投影，并保留高对比
回退与原遮挡裁剪；任意 lineHeight 不可仅加入缓存字段就宣称原生行距已实现。

## v300：声明式 EDIT 字体与 DPI 接线

input/textarea/number 现在从 Presentation 的已解析字体缓存取得继承后的字体、
字号和字重，挂载/重排时同步真实 EDIT；相对单位沿用布局解析，不另造样式算法。
EDIT 与 Select 共用同一个 HFONT owner 实现，未变字体复用句柄，替换后释放旧
字体，父窗口销毁时在默认 NCDESTROY 完成后释放，stock 字体始终只借用。
placeholder 继续读取 EDIT 的 WM_GETFONT，不向文本值写入提示。

保留既有合同：祖先字体/DPI 更新不更换字段 owner；字段自身 style 变化仍使
完整编辑合同失效并重建，不为保留草稿绕过事件凭证。report 仍不新增控件声明；
它不是现有节点硬准入白名单。完整 textAlign、lineHeight、圆角及透明合成尚未
因此完成，也没有宣称真实 IME、跨显示器人工或在线 Loom 验收。最新已打包候选
仍是 v299。

开发侧验证：全库 3080 passed / 55 ignored / 0 failed；Clippy all-targets
（-D warnings）通过；资源/EXE/session_startup/startup 分别 2/1/9/7 passed。
终审后增强了祖先字体更新保留真实本地草稿的断言，两项字体定向测试再次通过；
生产代码未在全库验证后修改。fmt、diff check、3328 文件行数 ratchet 通过，
没有超过 500 有效行的受检文件。本轮修改的 14 个文件为 UTF-8 无 BOM。
日志位于 linshi/otter-native-v300-regression。DPI 往返使用真实 EDIT 的显式
字体更新，不冒充物理跨屏消息链；共享更新函数所有人为递归 WM_SETFONT 排列
尚未验收，当前子类链未发现此类递归调用点。

## v299：Live 无框绿边黄角及无标题操作入口

后续包级闭环：v299 已完成独立 Release 候选构建、前后源码身份核验、298 文件
ZIP 归档核对和真正解包后的同数量文件验证。独立 cwd/TEMP/TMP 下 smoke 退出
0、缺图 --pin 和未知参数均按预期退出 1，TEMP 无残留。运行回执为
linshi/otter-native-v299-runtime/verification.json；EXE SHA256
1c47022ec3890e22b874b7827d08300e86b93a2119f588b199000da195ca8f5b，ZIP SHA256
8f9c2463a3d6dbbf4561b2d4939c94557cd7dc2965c428a442711f42caa665e4。
包内 runtimeVerified=false 保持不变；未签名、不可公开发布，不代表干净机器、
完整 Live 可见交互、功能等价或长时间资源验收。下文“未重新打包”是开发完成时
的历史停点，最新实际 ZIP 已更新为 v299，详见 WINDOWS_NATIVE_BUILD.md。

HWND 使用 WS_POPUP | WS_SYSMENU，不再呈现标题栏或系统拖边。成功呈现帧后
在同设备 backbuffer 内绘制 #22c55e 绿边和 #d9ff38 黄角；沿既有 96 DPI D2D
物理像素坐标使用 2 像素线宽、16 像素角臂和 5 像素 L 形命中带，不另乘 DPI。
框围绕客户区，letterbox 仍由 Viewport::fit 负责；角臂/线宽在极小窗口内收缩，
命中不超出客户区，也不把 16×16 的整个角方块占为本地操作。无成功呈现帧时
不提供不可见角命中；源停止仍沿原 owner 关闭，不展示最后帧为健康状态。

黄角左键复用 v298 本地移动，Alt/Alt+Shift 手势保持。Alt+Space 或黄角右键
弹出原系统菜单，保持分组、连线、控制、快照、冻结和回收入口；Alt+Delete
走 WM_CLOSE，不绕过回收。菜单前停止源输入和本地拖动；嵌套菜单期间阻断新
源启用/几何事务，选择返回后检查原 HWND/State owner，取消或销毁不派发。

控制状态文字同步到菜单；失败 latch 与单槽通知分离，同一失败态只通知一次，
重试进入 pending/启用后清旧提示。应用在 Session 借用结束后复用 MessageBox
报告错误，输入失败不停止采集；成功不弹模态提示，避免提示自身使控制失焦。
隐藏或非交互运行不弹窗且不延后补弹，失败菜单状态仍保留。此为既有原生提示
复用，不声称已实现非模态 toast，也不声称一次弹窗包含所有稍后 cleanup 诊断。

定向 live_ 回归 401 passed / 0 failed / 29 ignored。已检查实际 WARP backbuffer
读回图 linshi/otter-native-v299-regression/chrome.png，并以像素断言验证 2 像素
线宽、精确主题色、L 内凹底图和源 readback 未混入装饰；这不是用户桌面或真实
WGC 截图。完整回归日志同目录。实际可见菜单焦点、物理输入、混合 DPI、多时长
资源验收仍需完成；N01–N12 未退出，最新内部 ZIP 仍为 v294，未重新打包。

完整库 3078 passed / 0 failed / 55 ignored（213.80 秒）。本轮 17 文件 UTF-8
无 BOM、fmt、规模 ratchet（3326 文件，无 >500 有效代码行）与 diff 检查通过。
串行 build_resources 2、exe_resources 1、session_startup 9（6.46 秒）与真实
EXE startup 7（30.54 秒）通过；Clippy all-targets -D warnings 通过（49.79 秒）。

## v298：Live 本地移动与等比拖动

沿 FEATURES.md 的本地操作合同新增 Alt+左键拖动移动和 Alt+Shift+左键等比
缩放。多 agent 核查发现现有 Web LiveCapture 的 Alt+Shift 实际仍走普通移动，
因此本轮是补齐合同，不声称复制现存缩放公式。等比拖动固定初始客户区左上角，
按归一化位移绝对值较大的轴调整，所有采样均基于开始快照；共用 pin_geometry
尺寸预算（边长 8192、总像素 16,777,216），不改变采集帧范围和像素。

composition_live_drag 独占本地鼠标捕获和手势状态。精确 Alt/Alt+Shift 开始前
撤销源控制，后续仍需原启用流程；Session 刷新源控制状态不再释放本地捕获。
左键抬起完成；右/中键抬起仅消费，不改变几何或结束左键 owner。Esc、失焦、
取消、菜单、隐藏、最小化、捕获丢失、销毁和分组锁定/隐藏均退休本地手势。
只释放自己仍拥有的捕获，不恢复或抢夺其他窗口的捕获。

开始标记防止撤销源输入时的嵌套 begin 覆盖 owner；每次移动使用 revision，
定位每轮 SetWindowPos 前后复核，过期调用既不继续校正，也不回滚覆盖更新位置。
mouse-up 用独立 owner 身份确认退休对象。六个隐藏 HWND/WARP 定向测试通过
（0.43 秒），包括真实捕获移交、连续屏幕坐标移动、等比往返、精确组合键、
锁隐藏和生命周期，以及确定性的 begin/定位同步重入边界注入；注入不是完整
Win32 hook，也不代表所有消息重入排列已实测。完整回归日志记录在
linshi/otter-native-v298-regression。

完整库 3073 passed / 0 failed / 55 ignored（190.27 秒）；串行执行的
build_resources 2、exe_resources 1、session_startup 9（5.24 秒）、真实 EXE
startup 7（25.03 秒）通过。本轮不与全库同时运行启动探针，未出现超时。
Clippy all-targets -D warnings 通过（58.81 秒）；fmt、规模 ratchet（3323
文件，无 >500 有效代码行）、diff 和本轮 14 文件 UTF-8 无 BOM 检查通过。

本轮仍保留标准窗口外观；无框、绿框黄角命中、Alt+Delete 及真实物理多屏拖动
另行收口，N01–N12 未退出。未重新打包，最新实际内部 ZIP 仍为 v294。

## v297：Live 本地滚轮缩放与透明度

多 agent 分别核对几何/失败语义、源输入优先级和透明度生命周期，主线程集成。
Ctrl+wheel 复用 pin_geometry::zoom，以物理客户区和屏幕指针为锚点，定位沿
实际 HWND DPI 的非客户区有界校正。定位失败尝试恢复原外框，不关闭采集；
若系统拒绝恢复，保留实际 HWND 几何，后续消费者重新读取而非提交虚构尺寸。
Alt+wheel 按 Win32 delta * 0.001 调整整体 opacity，限制在 [0,1]。
Ctrl/Alt/Shift/Win 精确匹配，锁定/隐藏不变更窗口；本地操作先取消源输入、
清队列并释放捕获，因此之后仍需原控制启用流程，不声称自动恢复控制。

State 在可能重入的合成调用前记录期望透明度，补偿 paint 同步给 Host；
Host 跨首帧、suspend/recreate、resize 和下一帧保持值，不乘进帧像素。
保存/回收写入已有 opacityNormal，并将值加入异步 receipt 重验；冻结在
提交时读取，创建静态窗口后再次核对。mini opacity 保持原默认 0.9。
转换后的静态贴图继续遵循 normal_opacity 的既有导出语义。

完整库 3066 passed / 0 failed / 55 ignored（198.75 秒）；新增/扩展的真实
隐藏 HWND、WARP、PNG 保存恢复、冻结和回收用例通过。日志保留在
linshi/otter-native-v297-regression。build_resources 2、exe_resources 1、
session_startup 9 通过。startup 与全库并行时两个 15 秒超时；保留原日志，
不修改代码/超时值，等待全库结束后独占重跑 7 passed（22.86 秒）。这证明
独占运行通过，不将并发超时隐去或声称已定位全部环境因素。Clippy all-targets
-D warnings 通过（47.34 秒）；fmt、ratchet（3321 文件，无 >500 有效代码行）、
diff 和本轮 20 个文件 UTF-8 无 BOM 检查通过。无框/绿边黄角、
本地拖动与等比拖动、真实 WGC/物理输入/混合 DPI、多时长资源测试仍待完成；
不声明 N01–N12 总计划退出。最新实际内部 ZIP 仍为 v294，未重新打包或发布。

## v296：Live 初始物理位置与尺寸

多 agent 只读交叉核对 FEATURES.md 与生产入口后确认：Live 仍为默认位置的
960×640 标准窗口，尚不符合原版贴图宿主合同。本轮先接通源几何到输出窗口，
不以去掉标题栏但丢失操作/错误反馈的方式冒充无框迁移完成。

区域 Live 使用原物理选区作为客户区位置和尺寸；窗口 Live 在恢复保护登记前
读取源可见物理外框，前后复核 HWND/进程身份，拒绝已最小化来源。读取复用
输入坐标模块已有 DWM 外框及 PMv2 GetWindowRect 回退，不重复建立另一套规则。
可见外框仅决定初始输出区域，不替代 WGC item/ContentSize 的纹理尺寸权威。

LiveWindow::new_at 保持隐藏且捕获排除成功后定位，先进入目标显示器，再按
实际 HWND DPI 补偿非客户区；最多三次校正处理接缝的 DPI 变化，最终精确
核对 ClientToScreen 和客户区尺寸。非法/超预算/坐标溢出或无法精确定位时
返回错误，局部窗口和消费者由原 owner 释放，不启动采集 worker。

本机 144 DPI 的真实 HWND 回归曾复现 1×1 客户区被默认最小跟踪宽度扩大为
37×1（原点保持正确）。现仅在初始精确定位阶段使用 SWP_NOSENDCHANGING，
保留 WM_WINDOWPOSCHANGED/WM_SIZE，不改变用户拖边规则；原用例修复后通过。
五个定向测试通过（0.20 秒）：真实隐藏客户区/正方形 WARP 首帧、1×1 与极窄
客户区、非法/超预算矩形、创建 HWND 后注入失败的窗口/消费者释放、96/144/192
DPI 非客户区算术，以及自有窗口源移动/最小化拒绝/恢复/关闭。多 DPI 算术不
等于真实混合缩放显示器验收；标准窗口源 WGC/DWM 像素比较仍未进行。

完整库 3061 passed / 0 failed / 55 ignored（194.78 秒）；build_resources
2 passed、exe_resources 1 passed、session_startup 9 passed（5.98 秒）、真实
EXE startup 7 passed（30.12 秒）。Clippy all-targets -D warnings 通过
（47.49 秒），fmt、规模 ratchet（3319 文件，无超过 500 有效代码行）、diff
及本轮 14 个文件 UTF-8 无 BOM 检查通过。日志为
linshi/otter-native-v296-regression；未提交、推送或发布。

无标题栏、绿框黄角、本地移动/缩放/透明度以及控制状态反馈仍需后续接入。
本轮不声称混合 DPI 物理多屏、窗口 WGC 与 DWM 像素一致性或完整 Live 验收。
最新实际内部 ZIP 仍为 v294，未重新打包。

## v295：从 Live 自身选择并断开连线

Live 系统菜单新增“断开 Live 连线…”。窗口只合并单槽请求，应用复用
link_menu::Snapshot 的分页和打开时命令索引，在模态退出后核验来源 HWND、
工作区身份/拓扑、边成员关系及两端存活/锁定/忙碌状态，才移除选中边。
静态与 Live 共用 can_remove_link，不复制图状态或运行取消机制。菜单 HMENU
树由局部 RAII 释放，空目录直接返回；过期、取消或错误不更改现有连线。

同时收紧 Live 创建/断开连线的应用准入：来源分组隐藏时拒绝，确认后若其他
模态仍活跃亦拒绝提交；本次 guard 不清除其他窗口的模态状态。断线后的显示
缓存、待执行任务和会话保存继续跟随既有拓扑凭证，不额外通知下游自动运行。

开发侧验证使用隐藏 HWND/WARP、真实 HMENU 和注入选择，不等同于可见菜单
人工交互、自绘视觉或完整 WGC 验收。本轮未重新打包，最新实际 ZIP 仍 v294。

完整库回归 3056 passed / 0 failed / 55 ignored（189.05 秒）。新增四个 Live
断线回归覆盖选择单边后其他边保留、目标显示退休、保存包含剩余边、准备好的
执行在联网前取消；以及取消/空目录/错误/伪造边、关窗/同边重建/工作区替换、
锁定/隐藏/其他模态等拒绝路径。真实 HMENU 回归验证 65 条边跨页索引、禁用
状态及越界选择。日志位于 linshi/otter-native-v295-regression。

build_resources 2 passed、exe_resources 1 passed、session_startup 9 passed
（5.80 秒）、真实 EXE startup 7 passed（29.98 秒）；Clippy all-targets
-D warnings 通过（51.11 秒）。fmt、规模 ratchet（3317 文件，无超过 500
有效代码行）、diff 和本轮文本 UTF-8 无 BOM 检查通过。未提交、推送或发布。

## v294：从 Live 主动创建出向连线

Live 系统菜单新增“从 Live 创建连线…”，复用既有三个下拉框的连线对话框。
入向模式固定目标、选择来源；出向模式固定 Live 来源、选择目标。切换可选端点
只重置其关联端口，保留固定端的已选端口，不复制第二套对话框/端口目录实现。
Catalog::make_link 共用有界索引、端口及自连拒绝，实际接线仍由 Workspace 校验。

应用模态域内只收集选择，返回后核验 Live HWND、完整目录/拓扑、两端存活、
锁定、目标待接管/模态状态和应用保存/恢复/冻结/退出状态，随后一次提交。
取消、对话框错误、过期来源或无效端口均不产生半条边；错误沿既有连线日志
路径报告，不对已关闭来源弹出无 owner 的阻塞提示。窗口消息只排队，不联网。

三个定向回归通过：真实系统命令和隐藏出向选择控件配合注入选择，验证模态
释放、生成出边、真实红色 GPU 帧显示及会话保存；取消/错误/越界/自连拒绝；
来源/目标关窗、工作区或配置变化、两端锁定、退出和模态忙碌拒绝。固定端口
选择保留亦加入原入向选择框回归。没有进行可见模态人工验收，系统对话框仍
非完整 Neuro 自绘；Live 断线仍可通过目标节点原入口操作。总计划 N01–N12
仍需按功能/性能/包级/人工证据收口，不能把本项当完整迁移完成。

完整库 3051 passed / 0 failed / 55 ignored（197.04 秒）；build_resources
2 passed、exe_resources 1 passed、session_startup 9 passed（5.87 秒）、startup
7 passed（32.72 秒）。整理请求迭代写法及选择框内部命名后，Clippy all-targets
-D warnings 最终通过（45.37 秒），Live 连线 3 passed（1.07 秒）、原连线/自动
执行链 5 passed（1.72 秒）再次通过。fmt、规模 ratchet（3314 文件，无超过
500 有效代码行）、diff 和 14 个本轮文件 UTF-8 无 BOM 检查通过。日志为
linshi/otter-native-v294-regression；未提交、推送或发布。

本轮已构建 v294 Release 内部候选并完成 ZIP 组包、独立归档验证及真正解包
后的 298 文件大小/摘要校验。独立 cwd/TEMP/TMP 下 smoke 退出 0，缺图 --pin
和未知参数均按预期退出 1，TEMP 无残留。证据为
linshi/otter-native-v294-runtime/verification.json；包路径与 EXE/ZIP 摘要见
WINDOWS_NATIVE_BUILD.md。包内 runtimeVerified=false 保持不变；未签名、
不可发布，未替代干净机器、完整 WGC/物理交互、性能或人工验收。

## v293：Live 显式快照刷新与下游通知

Live 系统菜单新增“刷新连线快照并通知下游”。窗口回调只合并请求，应用在
模态域外核验工作区、窗口、输出连线和隐藏/锁定状态后捕获固定 GPU 帧。显示
缓存为每个 Live owner 保留至多一个最新刷新请求，受会话上限约束；复用原
显示单 worker，不新增线程池。新请求取消同来源未发布的旧任务，等待槽空闲。

刷新期间保留旧显示；成功读回且来源仍有效后才替换缓存并通知。失败保留旧
像素和显示身份，不通知、不重试循环。无帧/预算错误保留日志及旧画面，不伪造
图片。首次显示快照和显式刷新成功使用同一下游通知边界，复用现有目标监听、
自动执行、能力运行时和锁定门槛；普通流帧不通知。模态期间保留有界通知，
工作区替换/断线/关窗退休请求与通知。UI 不在快照回调直接编码或联网。

三个隐藏 HWND/可控 WARP 帧流测试通过：红帧缓存后流变绿不触发，重复菜单
请求合并并捕获蓝帧，后台处理时流再变绿也不改变本次蓝色快照；成功通知一次。
真实 GPU 描述符错误读回保留旧显示且无通知；关闭监听、来源锁定和同 UnitId
工作区替换也已覆盖。物理菜单、实际 WGC、自绘视觉及 Live 自身连线菜单仍待
继续，不把定向消息测试当作全面手测。未重新打包，内部 ZIP 仍为 v270。

完整库 3048 passed / 0 failed / 55 ignored（210.02 秒）；build_resources
2 passed、exe_resources 1 passed、session_startup 9 passed（4.70 秒）、startup
7 passed（25.95 秒）。随后移除测试比较表达式的冗余引用，Clippy all-targets
-D warnings 最终通过（47.28 秒），刷新定向 3 passed 再通过（0.26 秒）。fmt、
规模 ratchet（3312 文件，无超过 500 有效代码行）、diff 及 13 个本轮文件
UTF-8 无 BOM 检查通过。日志 linshi/otter-native-v293-regression，未提交或发布。

## v292：Live 连线后的按需快照显示与静态中继

显示图纳入当前工作区的 Live 来源。存在输出连线且首次需要显示时取得 GPU 帧，
复用原显示合成单 worker 读回；缓存仅持有 CPU Image 与会话 owner 凭证，不保留
GPU 帧或增加线程池。后续普通消息复用同一像素/显示身份，不按帧序号持续读回。
Live 来源的显示快照可以经过静态钉图中继；不会覆盖目标 Canvas 原图，Art 的
显示覆盖层仍由目标节点拥有，不把标注写回 Live 或沿正式结果传播。

Live 与静态显示合成共用任务槽，各自按 owner/编辑版本退休缓存。断开全部
输出连线、关窗或工作区替换会清除旧 Live 缓存；尚未完成的读回只取消，完成
通知后领取，不在 UI 更新路径等待线程。无帧不伪造图片，也不启动空读回任务。

三个隐藏 HWND/WARP 定向回归通过，覆盖红色像素实际显示及中继、重复消息共享
像素与显示身份、断线清屏、读回期间关窗、同 UnitId 的工作区替换、Art 覆盖层
归属及无帧。这里是按需缓存的显示快照，不是逐帧实时投影；显式刷新/下游通知、
Live 自身连线菜单及真实 WGC/物理交互验收仍待继续。未重新打包，ZIP 仍 v270。

最终完整库 3045 passed / 0 failed / 55 ignored（243.26 秒）；build_resources
2 passed、exe_resources 1 passed、session_startup 9 passed（5.30 秒）、startup
7 passed（34.63 秒）。Clippy all-targets -D warnings 通过（56.74 秒），fmt、
规模 ratchet（3311 文件，无超过 500 有效代码行）、diff 和 12 个本轮文件
UTF-8 无 BOM 检查通过。日志 linshi/otter-native-v292-regression；未提交或发布。

## v291：Live 连线来源与真实节点输入执行

静态节点的既有连线选择目录现在包含当前工作区存活 Live 的 output，应用统一
核验来源 owner、锁定状态和目录快照后建边/断边。Live 当前仅作为捕获来源，
未声明输入端口，不能通过连线覆盖正在采集的窗口内容。

图像执行解析包含 Live 节点，运行请求捕获固定 GPU Frame Arc，同一请求中多个
端口共享该帧边界。复用既有最多两个输入 worker 执行 GPU 读回及 PNG 编码，
继续遵守 32 输入及 256 MiB 批次预算，不新增实时读回线程或每帧 CPU 复制。
准备、启动、运行更新和结果提交均核验 Live owner 身份及工作区；流继续更新
不会取消固定帧执行，停止/替换会话会使旧输入失效。无帧手动运行给出错误，
自动运行跳过，不伪造图片、路径或远程引用，也不序列化采集授权。

定向测试已用隐藏 HWND/WARP 2×2 红色帧，从生产连线请求和运行入口进入输入
worker，再由回环 TLS/WebSocket 对端解码核对像素、返回结果并提交到节点。
这不是用户 Loom 服务、真实桌面 WGC 或物理菜单点击验收。Live 连线后的静态
画布显示、首帧/显式快照驱动的传播，以及 Live 自身连线菜单仍待接通；本轮
不把执行数据流已接通写成完整 Live 图像交互完成。未重新打包，内部 ZIP 仍 v270。

最终完整库 3042 passed / 0 failed / 55 ignored（221.66 秒）；build_resources
2 passed、exe_resources 1 passed、session_startup 9 passed（5.54 秒）、startup
7 passed（25.76 秒）。Clippy all-targets -D warnings 通过（46.59 秒），fmt、
规模 ratchet（3308 文件，无超过 500 有效代码行）、diff 及 21 个本轮文件
UTF-8 无 BOM 检查通过。四个 Live 执行回归覆盖真实像素执行、准备/启动前关窗、
无帧、旧目录/工作区和来源锁定。共享 TLS 测试对端仅在 application 注册一次，
避免重复模块加载。日志 linshi/otter-native-v291-regression，未提交或发布。

## v290：Live 创建与重命名分组事务

Live 系统分组菜单现在支持创建分组并加入，以及重命名当前组，复用既有名称
输入框。名称编辑进入应用模态域，保存、恢复、冻结或退出期间拒绝启动；确认
后复核 Live HWND、工作区身份和完整分组快照。创建及赋组在 Workspace 候选中
一起完成后才发布，空名、超长名称、容量耗尽与过期来源不留下半成品分组。
重命名保留 group ID、成员及隐藏/锁定状态，显式会话保存包含更新后的组名。

定向 Live 分组回归 6 passed，覆盖真实系统菜单请求、注入名称选择、取消、
名称/容量错误以及对话期间关窗、工作区替换和目录变化。注入名称选择不是
可见输入框人工验收；系统菜单仍非 Neuro 自绘。Live 连线与图像执行数据流、
真实 WGC/物理输入/混合 DPI 验收仍待继续，未重新打包，内部 ZIP 仍为 v270。

最终完整库 3038 passed / 0 failed / 55 ignored（165.79 秒），session_startup
9 passed（3.94 秒）、startup 7 passed（17.31 秒），Clippy all-targets
-D warnings 通过（43.94 秒）。fmt、规模 ratchet（3307 文件，无超过 500 有效
代码行）、diff 及 11 个本轮文件 UTF-8 无 BOM 检查通过。日志归档于
linshi/otter-native-v290-regression；未提交、推送或发布。

## v289：Live 分组菜单及共享隐藏/锁定/批量关闭

Live 系统菜单新增分组子菜单，可加入既有分组或移出当前组；分组仍由静态宿主
原有创建/重命名入口维护。菜单按当前组标记，最多 128 项按 16 对齐的独立系统
命令映射；选择只排队，应用核验创建工作区、Live 存活及完整分组快照后赋组。
旧菜单、重命名后迟到选择及旧工作区 Live 不得误改新状态。

共享分组隐藏/显示现在同步 Live HWND，恢复只还原原本可见的窗口且不抢焦点。
隐藏或锁定会取消 Live 输入请求/释放信号，并拒绝重新启用或发送输入；锁定拦截
用户系统移动/调整大小命令。删除分组保留 Live。批量关闭覆盖静态和 Live owner，
沿用既有静态组关闭的不回收语义，并将菜单明确标为“不移入回收站”；它不同于
单个 Live 的正常关闭回收。采集/输入 worker 仍由原 Session 回收，不另建线程。

菜单保留 Win32 系统样式，本轮未完成 Neuro 自绘视觉一致性；物理键鼠、多 DPI、
Live 自身创建/重命名分组入口、连线和执行数据流仍待继续。没有将隐藏/离屏 HWND
消息回归称作可见桌面交互验收。

最终完整库 3035 passed / 0 failed / 55 ignored（156.31 秒），session_startup
9 passed（3.88 秒）、startup 7 passed（17.22 秒），Clippy all-targets
-D warnings 通过（49.75 秒）；fmt、规模 ratchet（3305 文件，无超过 500 有效
代码行）、diff 和 15 个相关文件 UTF-8 无 BOM 检查通过。新增三个回归覆盖
加入/离开、第 128 组、stale/旧工作区、离屏 HWND 隐藏恢复、锁定/隐藏拒绝输入、
删组保留和混合 owner 批量关闭。日志归档 linshi/otter-native-v289-regression；
没有实际捕获用户桌面、操作用户文件、重新打包、提交或发布。

## v288：Live 与静态文档共享工作区身份生命周期

Live 启动通过应用准入 owner 登记真实 graph UnitId，受工作区总容量和 Live
句柄预算共同约束；注册失败销毁局部窗口并回收采集任务。停止转静态改为同身份
owner 交接，不再重复 add_units 或删除重建身份，已有连线/分组得以保留。
保存 Live 快照同时携带当前 groupId，仍不序列化采集权限。

工作区清理同时核对静态文档和同实例 Live。关闭普通静态窗口不再误删 Live；
回收最后静态文档使用单 Unit 替换保留其他 Live、分组及无关连线，而不是清空
整个拓扑。Live 终止/明确放弃/回收会清理其失效身份和相关边；转成静态文档的
同身份则继续存在。Workspace 替换候选保留实例凭证，准备失败不改原模型。

这是 graph 身份与生命周期接线，不代表 Live 已有完整分组/连线交互、图像输入
执行、自动首帧持久化或最终 WGC/混合 DPI 验收；这些仍继续迁移。

最终完整库 3032 passed / 0 failed / 55 ignored（156.67 秒），session_startup
9 passed（3.95 秒）、startup 7 passed（16.82 秒），Clippy all-targets
-D warnings、fmt、规模 ratchet（3302 文件，无超过 500 有效代码行）、diff 检查
及 17 个本轮相关文件 UTF-8 无 BOM 检查通过。Live/Workspace 跨模块定向测试
32 passed；日志归档 linshi/otter-native-v288-regression。未重新打包、提交或发布。

## v287：Live 关闭进入回收事务

应用绑定工作区的 Live 将 WM_CLOSE（包括系统关闭按钮/Alt+F4 的消息路径）转成
有界回收请求，系统菜单也提供“关闭 Live 并移入回收站”。窗口回调不读回或
等待线程；应用使用 v286 的当前帧快照和既有保存单槽/回收策略准备冻结记录。
候选完成后复核 Receipt，确认 Live HWND 已销毁才发布库原文，再由原 Session
停止输入/采集并回收 worker。静态宿主不关闭，回收库仍通过显式会话保存落盘。

无首帧、取消、旧工作区来源、窗口几何或静态文档变化均拒绝回收并保留 Live；
普通磁盘保存消费者不能领取该结果。为避免无帧或旧工作区窗口无法退出，另有
明确的“放弃 Live（不保留画面）”命令，只销毁该 Live，不写回收库。源捕获失败、
源关闭、应用退出仍沿用原有资源清理路径，不冒充用户关闭入库成功。

本轮不宣称 Live 交互 graph/分组连线、自动首帧持久化或真实 WGC/物理键鼠已完成。

最终完整库 3028 passed / 0 failed / 55 ignored（168.16 秒），session_startup
9 passed（4.27 秒）、startup 7 passed（19.02 秒），Clippy all-targets
-D warnings 通过（39.80 秒）；fmt、规模 ratchet（3300 文件，无超过 500 有效
代码行）、diff 和 14 个相关文件 UTF-8 无 BOM 检查通过。三个新增隐藏回归
验证关闭/菜单、回收写盘/恢复像素、失败保留及显式放弃。初次测试将恢复后的
显示栅格误当成原始源图，已改为分别验证独立 source 的 2x2 原像素及显示像素；
不修改恢复器以迎合断言。首次编译缺少 GetSystemTime 的 unsafe 块也已修正。
失败与最终日志均保留于 linshi/otter-native-v287-regression，未重新打包或发布。

## v286：运行中 Live 进入显式会话保存

核对旧版 liveCaptureUnit：运行授权/流句柄不序列化，保存内容仍是普通 sticker。
原生保存此前只遍历 DocumentWindow，会漏掉独立 Live；现在 UI 快照移交当前
工作区 Live 的完整 GPU 帧 owner 和物理图像区域，既有单槽保存 worker 负责
读回、PNG 编码与磁盘暂存。保存不停止 Live；恢复得到同 UnitId 的静态画面，
不自动恢复桌面采集权限。再次覆盖保存沿用已有磁盘字节基线，不重复追加身份。

Receipt 在后台完成及最终磁盘提交前核对 Live 集合、存活状态和几何。无首帧、
关闭、窗口移动/缩放、新增 Live 或取消使整次保存失败，不静默漏掉部分图像。
保存仅记录一个明确时间点，仍有 Live 时保守显示 Changed，不把持续变化的流
标为永久干净；即使之后 Live 结束，也不能错误复用该静态保存基线为 Unchanged。

这接通保存入口，不代表 Live 已完整进入交互 graph：分组/连线、直接关闭回收、
自动首帧持久化、真实 WGC 与混合 DPI 用户验收仍未完成，内部 ZIP 仍为 v270。

隐藏 HWND/WARP 的三个新回归覆盖实际新建/覆盖磁盘保存、PNG 恢复像素、同身份
去重、采集保持存活及取消/无帧/关闭/移动/新增会话/交付后变化拒绝。最终完整库
3025 passed / 0 failed / 55 ignored（166.56 秒）；session_startup 9 passed
（3.99 秒），startup 7 passed（18.15 秒），Clippy all-targets -D warnings
通过（41.99 秒）。fmt、规模 ratchet（3298 文件，无超过 500 有效代码行）、
diff 检查和本轮 12 文件 UTF-8 无 BOM 检查通过。日志位于
linshi/otter-native-v286-regression；没有读取/写入用户会话或捕获真实桌面，
没有重新打包、提交或发布。

## v285：Live 停止转静态的位置与显示尺寸继承

沿用 v284 的单槽读回及同 UnitId 发布事务，提交时读取 Live 客户区的屏幕物理
坐标，并复用呈现使用的 Viewport::fit 去除留黑边。静态 HWND 使用该图像区域
创建，不重采样原始帧；后台读回期间移动/缩放窗口，以提交时几何为准。建窗后
再次核对来源及几何，发生变化则丢弃隐藏候选，不提前停止 Live。最小化、空区域、
坐标溢出或超出既有静态窗口预算均拒绝转换，不静默裁剪或换到默认位置。

隐藏 HWND/WARP 回归补充后台读取后移动/缩放、真实静态窗口位置/尺寸、原像素
不变及空客户区失败保留源；测试线程显式使用 per-monitor-v2 DPI 上下文并恢复。
这不是多显示器不同 DPI 的物理交互验收，也没有接通运行中 Live 的 graph 保存、
直接关闭回收或最终用户验收。v285 版本号沿用恢复现场中已分配的一次迭代。

最终完整库测试 3022 passed / 0 failed / 55 ignored（168.23 秒），
session_startup 9 passed（3.92 秒）、startup 7 passed（17.49 秒）；Clippy
all-targets -D warnings、fmt、规模 ratchet（3296 文件，无超过 500 有效代码行）
和 diff 检查通过。11 个本轮相关文件通过 UTF-8 无 BOM 检查。定向 freeze
筛选 9 passed；首次测试编译漏导入 ClientToScreen，补齐后重新验证通过。
日志归档 linshi/otter-native-v285-regression，未重新打包、提交或发布。

## v284：Live 系统菜单到同身份静态贴纸的生产接线

Live 窗口创建时分配稳定 UnitId，系统菜单新增“停止 Live 并保留当前画面”。
窗口回调只排队；应用单槽后台取得显式最新帧并准备像素/alpha 命中区域，复用
add_prepared_with 的隐藏建窗与工作区登记事务，以原 Live UnitId 创建静态文档。
全部成功后才 stop 原 Live，继续由既有会话轮询回收采集、输入和可见性保护。
没有首帧、读取失败、来源关闭、工作区切换或文档准入失败均不得提前停止 Live。

独立复核发现仅绑定“点击时 Source”会允许旧 Live 跨恢复后写进新工作区，已补
Workspace::IdentityStamp：普通拓扑/分组编辑保留实例身份，整体恢复/替换产生
新身份。Session 只绑定创建时实例一次；开始、提交与建窗后的检查均要求匹配。
读取期间另外保留 Source 字节 owner，阻止已过期候选覆盖新会话来源。
回收最后静态文档的空宿主替换使用 replacement_host 保留工作区实例身份，仅
重建拓扑，避免同工作区仍存活的 Live 被误判为旧来源。另将 source_window 查找
移入既有 application_window_state owner，协调器不因新增任务字段越过规模门槛。

隐藏真实 HWND + WARP 测试从生产 WM_SYSCOMMAND 开始，检查像素与 UnitId、
提交前 Live 存活、成功后 HWND 销毁/worker 回收，无帧、源关闭、读回中来源
变化、恢复后再点击旧 Live、建窗后的重复 ID 准入失败。没有捕获用户桌面。
Live 运行中仍不是已登记可保存的 graph Unit，本入口仅在停止时接入工作区；
实时状态保存、直接关闭入回收、静态化窗口位置/显示缩放完整继承、真实 WGC
交互和全面验收仍待继续，不将这一条显式入口当作 Live 全功能迁移完成。
证据归档 linshi/otter-native-v284-regression，未重新打包、提交或发布。

最终完整库测试 3018 passed / 0 failed / 55 ignored（194.18 秒），
session_startup 9 passed（6.03 秒）、startup 7 passed（38.72 秒）；Clippy
all-targets -D warnings 通过（41.02 秒），fmt、规模 ratchet（3295 文件，无超过
500 有效代码行）与 diff 检查通过。早期行数检查发现 application.rs 502 行，
已按窗口所有权拆出来源查找并重新检查，不使用例外豁免。

## v283：Live 显式快照的真实 GPU 像素边界

核对旧版 liveCaptureController/liveCaptureUnit：正常停止在读取最新帧后解除运行
绑定，保留同一 graph sticker ID，不能用新建无关静态贴纸替代。当前原生 Live
Session 尚未绑定持久工作区 Unit；停止转静态/回收的端到端迁移仍未完成。

本轮补齐缺失的 BGRA8 生产读回模块 live_texture::readback。调用方移交完整
Arc<Frame<Texture>>，保留独立 GPU 快照和预算 owner；读回验证元数据及 D3D
多线程保护，分别准入 staging/输出预算，按 RowPitch 逐行复制，Map 使用非阻塞
重试、250ms 等待上限和取消检查。映射后由 RAII 在所有退出路径 Unmap；输出
保留原始 BGRA/alpha 字节，不借助屏幕截图、JPEG 或旧静态缓存。

Host/LiveWindow/Session 新增显式 snapshot_frame 边界：取得最新已投递帧，可能
尚未呈现，不改输入使用的 presented_viewport。源关闭、窗口关闭、重入占用或
没有首帧时不伪造旧画面；已移交的帧允许跨源关闭完成后台读回。测试使用真实
WARP GPU 和隐藏 HWND，验证不同最新/已呈现帧、暂停、源关闭、后台移交、原池
纹理复用、精确像素、取消、预算及 Map 后失败。未新增用户冻结命令，也未把此
底层能力称为 Live 停止/保存/回收功能完成。证据在 linshi/otter-native-v283-regression。

最终完整库测试 3014 passed / 0 failed / 55 ignored（187.62 秒），startup
7 passed（29.17 秒）；Clippy all-targets -D warnings 通过（46.38 秒），fmt、
规模 ratchet（3292 文件，无超过 500 有效代码行）与 diff 检查通过。没有重新
打包、提交或发布，没有用合成 WARP 证据替代真实 WGC 源及用户交互验收。

## v282：N06 原生回收保留设置与配置持久化

原生菜单新增“回收站保留设置…”，通过 Win32 对话框编辑最多条数和保留天数。
0 禁用对应软限制，输入限制为 500 条/3650 天；取消、非法输入、来源窗口关闭
或设置来源变化均不发布草稿。确认后更新应用级策略并触发已有后台裁剪；设置
另存为、覆盖保存及自动保存复用现有事务，保留未知 JSON 字段，异步旧结果不能
覆盖较新的策略。内存回收裁剪仍须保存会话才会写回会话文件。

隐藏真实 HWND smoke 覆盖菜单、取消、非法策略、裁剪、保存、自动保存、stale
及 origin 关闭，路径/对话框选择为注入，不是实际可见表单交互验收。对话框暂用
现有 Win32 系统控件样式，尚未宣称完成 Neuro 深色视觉一致性。验证证据归档
linshi/otter-native-v282-regression；未重新打包，内部 ZIP 仍 v270。

最终完整库测试 3009 passed / 0 failed / 55 ignored（184.56 秒）；
session_startup 9、settings_recovery 3、settings_startup 4、startup 7 全通过。
Clippy all-targets -D warnings 通过（45.87 秒），fmt、规模 ratchet（3290 文件，
无超过 500 有效代码行）与 diff 检查通过。新增回归确认非法回收配置不会部分
安装命名设置，异步导入也不能覆盖较新的回收偏好。


## v281：N06 加载与设置更新后的自动回收裁剪

session_library_prune 对已有回收记录先按天数过滤，再保留末尾 N 条，复用关闭
回收的日期/数量规则；未知日期保留，参考库、活动窗口、未知字段和 revision
不改。无变化不产生模型候选。应用在会话恢复及设置安装/导入/新建或覆盖保存
后排队维护，空闲时使用现有保存单槽后台处理，不增加计时器或额外线程池。
完成后核验来源 Receipt 和请求时回收策略，取消/stale/策略变更不发布；失败报告
一次，不自动无限重试。无变化保持原来源 owner 和保存状态；有变化使用
edit_original 保留已提交磁盘比较基线，只有随后显式保存才写回文件。

真实隐藏窗口 smoke 覆盖加载策略、设置更新、无变化不置脏、取消、编辑/策略
stale 和覆盖保存。主循环及隐藏会话检查先 acknowledge 再读取邮箱/启动 worker，
避免快速完成通知被旧 pending 合并丢失；隐藏检查在没有任务却仍忙碌时明确失败，
不无限等待不存在的完成消息。证据归档 linshi/otter-native-v281-regression。
本轮不增加回收策略编辑界面、不删除任何用户文件，不覆盖 Live 冻结/键盘关闭。

扩展真实 EXE 检查覆盖无用户输入的加载后裁剪（20→15，源文件不变）。配置
失败测试暴露图片 child smoke 先于设置读取，已将它移至配置准入之后；保持私有
worker 的早期分流不变。最终完整库测试 3006 passed / 0 failed / 55 ignored
（190.74 秒），session_startup 9、settings_recovery 3、settings_startup 4、
startup 7 均通过；Clippy all-targets -D warnings 通过（47.35 秒），fmt、
规模 ratchet（3286 文件，无超过 500 有效代码行）和 diff 检查通过。
中途有启动时限失败，保留 integration-final.log；不改代码/时限的最终重跑
integration-recheck.log 全通过。单独 smoke 约 9 秒完成，先前时序波动根因未证实，
不能据一次重跑宣称所有负载条件稳定。未重新打包或进行用户全面验收。

## v280：N06 显式关闭入回收站与最后宿主

原生菜单新增“关闭贴纸并移入回收站”。命令只排队，应用协调器捕获当前快照，
读取已加载设置中的回收策略，经后台编码后复核 Receipt。回收提交 owner 先准备
最后一个窗口所需的隐藏空宿主、工作区与捕获登记容量，再次复核后转交实际注册
热键并同步销毁目标；目标仍活着则回滚热键且不发布原文，确认目标已销毁才入库。
复用 edit_original 保留磁盘比较基线，清除目标 annotation/执行元数据，保存仍
由独立保存事务完成。空宿主不写为活动贴纸，并继续提供回收恢复入口。

隐藏真实 HWND smoke 覆盖菜单到关闭、取消、stale、注入销毁失败、最后空宿主、
连续覆盖保存及从空宿主恢复。最后宿主失败的热键转交/反向回滚使用注入注册后端，
不占用用户 Ctrl+1；不能将其称为真实系统热键注册验收。独立 Live/远程 HWND 不在
本次关闭对象内，不宣称当前静态 Document 快照就是 Live 当前帧；目标待启动 Live
请求必须拒绝关闭。通用 WM_CLOSE、应用退出、分组关闭及键盘删除路径未改。
启动/设置变更自动裁剪、Live 冻结关闭以及其他关闭入口仍需后续推进。
验证证据归档 linshi/otter-native-v280-regression，未重新打包或正式发布。

最终源码完整库测试 3003 passed / 0 failed / 55 ignored（197.52 秒），startup
7 passed（30.60 秒），Clippy all-targets -D warnings 通过（2 分 08 秒）；fmt、
规模 ratchet（3282 文件，无超过 500 有效代码行）、git diff --check 通过。
交叉复核发现最后宿主原位替换会绕过旧执行代际清理，已补清理并加入非活动历史
记录的关闭后断言。当前证据不涵盖真实 Ctrl+1 注册转交或用户全面验收。

## v279：N06 回收后台快照与消费者隔离

回收候选现在可由保存任务 owner 在后台从当前模型编码，复用单槽、完成唤醒、
取消和 join 回收。任务用途显式区分 Save、LibraryUpdate、Recycle(unit)，
普通保存领取仅允许 Save；收藏/删除轮询只允许 LibraryUpdate，不能误发布
会裁掉活动窗口的回收候选。候选不推进磁盘 revision，不暂存文件。

新增隐藏真实 HWND/PNG smoke：当前透明度及图片进入冻结快照，第二任务被拒绝，
普通保存及收藏消费者不能领取，完成后取消仍丢弃结果，编辑后 Receipt 失效，
结果仅交付一次，准备期间原窗口与原文均保留。验证日志位于
linshi/otter-native-v279-regression。此步仍不接关闭菜单或 WM_CLOSE；后续需完成
关闭发布、最后空宿主、热键交接及销毁失败处理，不能把后台准备当作关闭功能完成。

最终源码验证：完整库测试 3003 passed / 0 failed / 55 ignored（198.18 秒），
startup 7 passed（30.78 秒），Clippy all-targets -D warnings 通过（45.11 秒）；
fmt、规模 ratchet（3280 文件，无超过 500 有效代码行）、git diff --check 通过。
初次 smoke 中测试引用路径及 opacity 字段名写错，已按真实模块和 opacityNormal
契约修正后完整重跑；没有为测试放宽生产准入。未重新打包、发布或人工验收。

## v278：N06 关闭前回收候选与保留规则

新增 session_library_recycle::prepare，输入是当前已编码会话快照、待关闭
身份、新 entryId/时间、回收策略和当前时间。收藏与回收共享 freeze::capture，
保留图片/标注等当前字段并剥离运行时节点/分组/端口配置；不从磁盘旧窗口重建。
回收候选仅移除原文中待关闭窗口和其关联链接，其他窗口、剩余拓扑、参考库及
未知根值保留；不推进 documentRevision、不触碰磁盘或 HWND。

读取已有 AppSettings.cache 的 recycleBinMaxEntries/recycleBinRetentionDays，
默认 15/0；数量 0 不限软条数、天数 0 不过期。先过滤超龄，再保留末尾 N 条，
截止时间包含等于；未知日期保留。4096 导入硬预算、256 根字段、共享输出字节
预算独立存在；不限软数量不等于允许超过硬预算。非法策略/身份/Art/重复条目/
取消均不交付候选。ISO UTC、显式偏移与纯日期解析不依赖系统时区；无时区或
本地化旧日期保留不猜测，目前不宣称覆盖 JS Date.parse 的全部历史输入。

七个新增测试验证当前快照而非旧窗口、只裁目标链接、默认/零值/时间边界/
非法日期/尾部顺序、满库软裁剪与硬拒绝、设置字段保留及日期闰年/偏移解析。
证据归档 linshi/otter-native-v278-regression。实际关闭生产入口仍待接线：
最终源码完整库测试 3003 passed / 0 failed / 55 ignored（174.24 秒），
startup 7 passed / 0 failed（28.93 秒），Clippy all-targets -D warnings
通过（1 分 06 秒）；fmt、规模 ratchet（3278 文件，无超过 500 有效代码行）、
git diff --check 与本轮 12 文件 UTF-8 无 BOM 检查通过。这些验证覆盖共享冻结
实现未破坏已有收藏/复制/删除链路，不证明真实关闭提交已经实现。
WM_CLOSE 当前直接销毁，关闭提交需先准备并复核，最后窗口还需处理空宿主与
热键归属；不能先销毁再后台补存。启动恢复/设置变更后的自动裁剪也尚未接入。
本轮未打包、未写用户会话、未提交/推送，实际内部 ZIP 仍为 v270。

## v277：N06 冻结库条目删除的原生生产入口

两库条目改为动作子菜单：恢复/复制与“删除此条”明确分离。每页 64 个条目
仍完整可达，普通命令占 BASE..BASE+127，导航占 BASE+128..131，删除占
BASE+256..383；间隙及超范围命令不映射为条目。子菜单进入时遵循现有键盘
聚焦首项语义，不修改通用导航行为。两库完整页约 392 个节点，整菜单继续由
现有 1024 节点/深度/文本预算检查；最大业务菜单组合尚未额外枚举。

删除退出菜单后经同一有界请求，核验窗口目标/来源，复用保存槽中的库更新
worker。只生成指定库按 entryId 删除的原文候选，不编码图片、不创建 HWND、
不将同 sourceStickerId 的其他条目一起删掉；另一库、其他条目未知字段和
磁盘 revision 保持。普通磁盘候选入口拒绝所有 library_update 模式，完成后
核验取消/来源/编辑凭证，成功才经 edit_original 发布，保留真实磁盘基线。

新增纯测试验证精确身份、另一库及未知原文保留、取消/缺失/非法输入拒绝。
分页测试同时遍历普通/删除两套 4096 项映射。隐藏原生 smoke 经键盘选择
两库第二页的删除动作，分别验证取消、stale 和成功，活动 HWND 不关闭，
再使用 linshi 临时文件覆盖保存，最终 revision=3、两库各剩 64 条。
首轮 smoke 多按一次向下键而选错动作；已依据现有 Enter 自动聚焦首项修正
测试驱动，未为测试改写生产菜单导航。证据归档 linshi/otter-native-v277-regression。
最终源码完整库测试 2996 passed / 0 failed / 55 ignored（176.98 秒），
startup 7 passed / 0 failed（23.91 秒），Clippy all-targets -D warnings
通过（41.63 秒）；fmt、规模 ratchet（3274 文件，无超过 500 有效代码行）、
git diff --check 和本轮 22 文件 UTF-8 无 BOM 检查通过。
本轮未写用户会话、未打包/提交/推送。关闭入回收站、回收保留策略、Live
专用实时快照和其它 N01–N12 缺口仍未完成，实际内部 ZIP 仍为 v270。

## v276：N06 当前贴纸收藏写入与磁盘基线分离

原生菜单新增收藏/更新快照与取消收藏。窗口回调只投递一个绑定来源、贴纸身份
和编辑目标的请求；Application 捕获已有保存 Receipt/Data，复用保存 worker
槽在后台编码当前图片、标注和几何，再构造仅替换 referenceLibrary 的原文。
取消收藏按 sourceStickerId 移除，不编码图片、不关闭贴纸。同源更新去旧追加，
新 entryId 和 UTC 时间绑定新快照，其他条目与未知根值按 RawValue 保留。
两种操作均保留磁盘 revision；普通 save candidate 入口拒绝收藏模式，取消、
来源/工作区变化或退出不发布。容量/字段/共享字节预算均保留失败关闭边界。

交叉审查发现库内存更新后的 original 被误当作已提交磁盘字节，导致随后覆盖
保存必然失败。Record::edit_original 现在在首次库编辑时保留 disk_original，
连续编辑不覆盖，Destination::Replace 显式使用它；成功保存或归档才清除。
v273 冻结追加同步使用此入口。不通过取消精确文件比较来回避冲突，外部修改
仍拒绝覆盖。隐藏 smoke 使用 linshi 独占临时文件验证首存、收藏、覆盖、
参考复制、覆盖、更新/取消与再次覆盖，最终 revision=4；外部追加空白也被拒绝。
最终源码完整库测试 2994 passed / 0 failed / 55 ignored（179.80 秒），
startup 7 passed / 0 failed（21.61 秒），Clippy all-targets -D warnings
通过（45.46 秒）；fmt、规模 ratchet（3271 文件，无超过 500 有效代码行）、
git diff --check 和本轮 25 文件 UTF-8 无 BOM 检查通过。五个新增纯测试覆盖
原文保真、同源替换/删除、错误拒绝、4096 项容量及新增根字段预算。

实现仍复用整工作区编码/Guard，未优化成单贴纸资源编码，其他窗口的未提交编辑
可能阻止操作。已核对旧前端右键关闭与键盘删除的 Live 失败策略不同，分组关闭
不入回收站；本轮没有擅自统一这些行为。正常关闭入库、Live 专用实时快照及
回收数量/时间策略仍待实现。证据归档 linshi/otter-native-v276-regression，
未写用户会话、未发布或推送，实际内部 ZIP 仍为 v270，总迁移目标保持未完成。

## v275：N06 冻结库原生菜单到后台追加入口

弹出和托盘菜单增加回收站/参考库，展开时启动只读索引线程；每库按 64 项
分页，所有 4096 个准入条目均可到达，不以截断满足菜单树预算。局部导航
命令只替换子页，不关闭根菜单或丢失捕获；键盘、指针、UIA 走同一控制器，
未消费的导航使模态泵失败关闭，导航不能误交付为业务命令。

菜单只捕获不可变来源；退出模态范围后投递单槽请求，Application 再验证
原窗口及编辑目标，复用 request_frozen 的来源/坐标/互斥检查和后台追加链。
后台索引不解码图片、不读取文件，失败按库隔离。析构取消并 join 唯一线程，
但正在进行的有界 JSON 解析不可中途取消，最大输入的关闭等待仍未实测。
整体菜单仍接受现有 1024 项/文本预算检查，最大业务菜单组合尚未枚举。

新增完整分页映射、末页身份、坏库隔离及三种导航输入测试。隐藏原生 smoke
实际选择两个库的第二页，经过队列/worker 追加图片 HWND，并验证去重、
过期目标拒绝、回收消费及参考保留。证据归档 linshi/otter-native-v275-regression。
最终源码完整库测试 2989 passed / 0 failed / 55 ignored（176.65 秒），
startup 7 passed / 0 failed（20.03 秒）；Clippy all-targets -D warnings
通过（38.25 秒），fmt、规模 ratchet（3266 文件，无超过 500 有效代码行）、
git diff --check 及本轮 25 文件 UTF-8 无 BOM 检查通过。新窗口请求函数
原样归回 application_commands，避免 Application 主文件增长至规模例外区。
本轮未打包、未写用户会话、未提交或推送；实际内部 ZIP 仍为 v270。
正常关闭入库、参考新增/删除、保留策略及其他 N01–N12 缺口继续保留，
不能将本轮入口连通记为完整迁移或用户手测通过。

## v274：N06 冻结恢复/参考复制的后台协调链

新增 session_job::Input::Frozen 与 session_frozen_input：请求只共享不可变
来源 owner，在线程中读取冻结条目、生成新身份单窗口候选，再进入既有图片/
矢量资源准备。输入字节预算、条目身份和非有限坐标在起线程前拒绝；取消
检查贯穿候选准备和字节交接。沿用既有候选 Vec owner，仅复制所选窗口字节，
不复制整库；完成通知、任务回收和析构 join 沿用原有单任务实现。

Application::request_frozen 捕获菜单来源 storage、当前会话空间、原工作区
和目录凭证。物理桌面鼠标先转换为逻辑坐标，再按旧前端 +50 规则定位。请求
复用 save/restore 互斥，后台期间编辑、来源或坐标过期会失败且不消费条目。
完成分支绑定状态、准备隐藏 HWND 后走 v273 追加事务，不调用整工作区替换或
record_loaded_save_state。取消仍是发信号，结果回收后不发布。

交叉核验发现普通 request_restore_input 若接受裸 Frozen 会缺失追加模式；
现已明确拒绝该输入，只有 private start_restore 同时原子建立 worker 和
Selection，且检查输入与发布模式一致。独立隐藏 smoke 通过此负例、并发/
保存拒绝、取消、编辑/坐标 stale、未知条目、非默认空间图片追加、旧菜单来源
拒绝、回收消费/参考保留、Unsaved 和 HWND 清理；startup 明确断言通过标记。
四个 worker 回归另验证跨线程完成通知、真实图片、路径无隐式授权、取消回收及
起线程前预算拒绝。证据位于 linshi/otter-native-v274-regression。完整原生库
2984 passed / 0 failed / 55 ignored（176.37 秒），startup 7 passed /
0 failed（20.34 秒），Clippy all-targets -D warnings 通过（39.94 秒）；
其余静态门禁和源码/日志哈希见 verification.json。

边界：本轮连通的是后台应用协调链，不是菜单入口；仍需菜单条目读取/完整分页、
选择请求交付、关闭时冻结、保留策略和用户交互验收。没有访问用户文件或写入
用户会话，没有组包、提交、推送或发布；实际已组包运行的候选仍为 v270。

## v273：N06 冻结贴纸单窗口追加与库消费事务

在 v272 的候选准备之后新增 application_frozen_append，复用既有隐藏窗口
准备、资源恢复、窗口/工作区 guard，不调用会替换整工作区的 publish_restored_batch。
成功提交只新增一个文档及 workspace 身份，保留原 HWND、分组、连接和热键
owner；同时追加 annotation 身份并更新保留原文，清除旧 saved stamp。

session_library_retention 先构造有界内存原文：保留新贴纸及根未知 JSON 值，
移除已经关闭的旧原文窗口及相应失效链接；不提前推进磁盘 documentRevision。
实例必须与所选 snapshot 的未改字段逐项原文匹配，不能用任意新窗口消费
另一条记录。root/snapshot 最多 256 个唯一字段，超限拒绝而不裁剪扩展。
参考复制保留条目；回收恢复只在窗口和原文共同提交成功后才消费条目。正式
磁盘保存仍走现有完整快照及提交机制，本轮没有对用户会话执行写盘。

事务校验同一 Source storage、工作区/窗口 stamps、目录及准备时坐标空间；
正在保存/恢复、模态或其他不空闲状态拒绝。可失败容量预留及优先级监听安装
在最终 guard 前完成；最后一次 guard 后先原子登记 workspace 身份，再完成
不派发消息的 owner/原文提交，显示在提交之后。失败隐藏窗口按 owner 回收。

独立隐藏 Application smoke 使用真实内嵌 PNG 验证新 HWND、旧窗口/拓扑
保留、stale 编辑/同字节不同来源/缺失或变化坐标拒绝、库成功消费、未知字段
保存组装和 HWND 清理；startup 明确断言其通过标记。纯会话回归另覆盖闭窗
链接裁剪、参考保留、容量/身份/取消拒绝及篡改候选拒绝。证据目录为
linshi/otter-native-v273-regression，最终结果见 verification.json。最终会话
相关 402 passed / 0 failed（20.92 秒）；startup 7 passed / 0 failed
（18.83 秒）；Clippy all-targets -D warnings（37.44 秒）、fmt、行数 ratchet
（3253 文件，无超过 500 行文件）和 diff 检查通过。完整库本轮未重跑，最新
全量仍是 v272 的 2976 passed / 55 ignored。提交时还会裁掉已关闭窗口的
annotation 身份，避免反复关闭/复制积累无 owner 元数据；隐藏 smoke 覆盖此项。

仍未完成：菜单入口与后台请求调度、关闭时自动冻结、缓存保留策略、用户真实
交互验收。当前调用验证为独立 smoke，不将事务 API 当作完整 N06 交互完成。
本轮未组包、提交、推送或发布；实际已组包运行的内部候选仍为 v270。

## v272：N06 冻结条目的独立身份候选准备

新增 session_library::candidate::prepare：从指定库的指定条目生成独立的
单窗口会话候选。调用方提供新身份，拒绝来源身份与当前活动窗口冲突；位置按
旧前端 mouse + 50 规则，保留尺寸、透明度、savedRect/cropOffset、捕获及
快照扩展原始值。移除分组、节点端口/执行、Art 身份及编辑传播所有权，params
重置为空。未知值借用 RawValue，避免重编码超大数字或扩展内容。

快照最多 256 个唯一字段，超限或重复键拒绝而非静默裁剪；沿用 64 MiB 输出
上限、取消检查及 session_write 共享预算。坐标元数据保留，输出经既有
session_document 再校验。回收站恢复准备和参考复制准备均只读：失败、取消
和成功准备都不消费条目，原文不变；文件引用仍必须经过既有资源根准入。

六个新增回归覆盖原文值、运行所有权隔离、冲突/无效位置/取消、重复参考复制、
缩略裁剪、字段限制和既有 session_job worker 接入。最终完整原生库
2976 passed / 0 failed / 55 ignored（177.43 秒）。证据目录：
linshi/otter-native-v272-regression，其他门禁结果以 verification.json 为准。

边界：这不是完整库交互功能。尚未连接菜单、单窗口追加发布、成功发布后的
回收站消费/保存、关闭时冻结和保留策略。现有 publish_restored_batch 会替换
整工作区，不能拿来实现单条恢复。当前 source 为内部 v0.2.33.272，实际已
组包运行的候选仍为 v270；本轮未组包、提交、推送或发布，N06/N01–N12 未完成。

## v271：N06 回收站与参考库的会话读取入口

核对实际 FrozenStickerEntry 基线（stickerSnapshot.ts、stickerLibraryModel.ts、
apiTypes.ts）及原生会话调用，确认此前只有未知根字段保留，没有冻结库读取
owner。新增 session_library，经 Document::library 按需读取 recycleBin 或
referenceLibrary，保留 entryId/sourceStickerId/createdAt 与完整 snapshot
原文，复用 session_window::Window 的元数据校验，不复制另一套恢复算法。

安全边界：沿用 64 MiB 会话上限，单库最多 4096 项，反序列化时即限长；这是
导入安全上限而非用户保留设置，超限整库返回错误，不删除/裁剪历史。校验条目
身份唯一、来源与快照身份一致、窗口几何/透明度/缩略/捕获元数据及 sticker
类型。按旧前端行为保留无法解析的时间文本，只限制长度与控制字符，不在读取
时自动过期条目。未知条目/快照字段原文借用，读取不授权 filePath 或图片解码。

坏库不会阻止普通活动窗口恢复，也不会阻止另一个有效库读取；旧会话原文不变。
新入口尚未接菜单、图片准备、参考复制、回收恢复、保留策略或持久化修改，
不能据此关闭 N06。新建/关闭钉图也尚不会自动进入回收站。

最终会话相关 392 passed、0 failed（20.17 秒），包含五个冻结库回归；startup
7 passed、0 failed（17.37 秒），Clippy all-targets -D warnings 通过
（53.30 秒）。fmt、行数 ratchet（3247 文件，无超过 500 行文件）、diff
检查通过。证据在 linshi/otter-native-v271-regression。完整库本轮未重跑；
最新全量仍是 v270 的 2965 passed / 55 ignored，最新实际 ZIP 仍为 v270。
未组包、提交、推送或发布，下一纵向继续冻结条目的受控恢复/复制。

## v270 包级续进：当前 Release ZIP 与真正解包运行

本轮沿既有构建/组包入口实际构建当前 v0.2.33.270，没有产品源码变更或
另增内部版本。构建前后来源/工具/环境核对通过，dirty 内部候选身份已绑定：
sourceSha256=99fcb3a4fff887d145a758b00edcc1f81387e6396684f9b5c59e3eb46a262117。

- 构建：linshi/otter-native-v270-candidate。
- ZIP：linshi/otter-native-v270-package/Otter-native-v0.2.33.270-windows-x64-internal.zip。
- EXE：10813952 bytes；SHA256
  ce1ccdf2c300aa140df31de7da4fb1b3a4979b94bf1bb5829193558b1b33934f。
- ZIP SHA256：c3affdd411cbe2c58db0887abdfeea046749d426531fb226e5d9eaf69303f169。
- 运行：linshi/otter-native-v270-runtime/verification.json。

正式验证器检查归档后，实际解包到新目录并逐一核对清单文件，298 个文件
一致。解包 EXE 在独立 cwd/TEMP/TMP 下执行 --smoke-test（exit 0）、缺少
图片的 --pin（exit 1）、未知参数（exit 1），状态与预期一致。smoke 核验
真实无 WebView 隐藏窗口、受限 SVG/WebP child 和共用历史交付路径等输出；
TEMP 为空。运行脚本与日志保留在 linshi，不读取用户历史或改动用户工作区。

该证据替代“最新包仍是 v234”的旧事实；已同步 BUILD 和 PARITY 中过时
阶段说明。包内 publishable/signed/runtimeVerified 仍为 false，运行探针
收据独立保存，不修改历史或包内身份以伪造全面验收。干净机器、物理输入、
真实辅助技术、完整功能性能与签名验收仍未完成。未提交、推送或发布。

## v270：区分订阅范围边界，跨进程 Invoked 两种范围通过

生产 provider、窗口关闭与快照撤销保持不变。对照发现，仅把客户端订阅
从即将销毁的菜单根改为精确菜单项（TreeScope_Element）或稳定 desktop
root（TreeScope_Subtree），三条跨进程选择路径均收到事件。available()
失败 backtrace 诊断没有发现撤销后的 provider 读取失败，因此 v269 将问题
优先归因于 provider 数据撤销的假设降级；当前证据证明订阅范围相关性，
未证明 UIAutomationCore 内部具体机制，也未修复/保证瞬态根子树交付。

[Invoke 合同](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-implementinginvoke)
允许元素在 Invoke 后立即消失，并建议缓存读取；[订阅说明](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-eventsforclients)
明确子树监听关联根而非逐个后代注册。未查到锚点销毁后仍保证交付的合同，
因此没有以固定延时、保留可执行失效节点或跳过清理修改生产行为。

跨进程测试每次启动六个独立事件客户端，覆盖精确源/稳定祖先乘 UIA Invoke、
键盘消息、鼠标消息，保持缓存名称、单次通知、业务命令仅一次和 handler
释放断言。回调先按缓存 PID 过滤，不将其他进程名称放入通道；缓存读取失败
显式报告，不再仅表现为 channel 超时。同进程原菜单根订阅回归保留。

另一个诊断边界：同进程改为精确源订阅时出现同名重复通知，日志见
same-process-diagnostic.log，未定位/修复。没有宣称所有范围普遍单次投递；
此项及瞬态菜单根的跨进程子树订阅仍须在完整辅助技术验收前评估。

最终菜单 native_menu-scoped-final.log：70 passed、0 failed、2 ignored
（4.53 秒）；完整库 2965 passed、0 failed、55 ignored（174.44 秒）；
startup 7 passed、0 failed（17.14 秒）；Clippy all-targets -D warnings
通过（37.64 秒）。两个菜单 ignored 是父测试精确启动的子入口，均实际执行。
fmt、行数 ratchet（3245 文件，无超过 500 行文件）、diff 检查通过。
证据在 linshi/otter-native-v270-regression，保留最初编译导入遗漏、同进程
精确源重复及各对照日志，不将中间失败覆盖为成功。

这是测试合同修正与更完整跨进程证据，不是生产事件实现修复或完整菜单 UIA
验收；未组包、提交、推送或发布，总迁移仍进行。

## v269：跨进程 Invoked 事件失败已复现，尚未修复

新增回归复用同进程事件合同，通过独立子进程订阅 Invoked；计划覆盖 UIA、
键盘消息、鼠标消息三种路径。实际首个 UIA 路径失败：监听状态为 true，
UiaRaiseAutomationEvent 返回 Ok，但客户端三秒内没有进入 handler，不能
把发送成功当成交付成功。父进程回收失败子进程；该失败测试保留在普通测试
集合中，没有 ignore 或放宽断言掩盖失败，另外两个输入路径尚未实际执行。

诊断对照证据均保留在 linshi/otter-native-v269-regression：
- events-diagnostic.log：有监听且发送成功，客户端超时。
- events-delay-diagnostic.log：发送后保持窗口并额外泵消息 200ms，三条路径
  均可收到缓存事件；只是时序诊断，不是可交付修复。
- events-registration-diagnostic.log：订阅后、Invoke 前等待 200ms 仍失败。
- events-revoke-diagnostic.log：仅跳过快照撤销仍失败。
- events-cleanup-diagnostic.log：仅跳过 WM_NCDESTROY 的 UIA 清理仍失败。

证据指向事件发送后的窗口/封送生命周期，但尚未定位具体失效调用；不能据此
断言唯一根因。所有诊断日志打印、延时、跳过撤销及跳过清理已撤回，生产逻辑
保持原行为。微软说明 [事件函数会持有 provider 至处理完成](https://learn.microsoft.com/en-us/windows/win32/api/uiautomationcoreapi/nf-uiautomationcoreapi-uiaraiseautomationevent)，
后续须继续核验 HWND、provider 和异步缓存构建的实际生命周期，不用 sleep
或保留可执行失效节点来换绿灯。

最终菜单回归 native_menu-final.log：69 passed、1 failed、2 ignored
（5.90 秒）。startup 7 passed、0 failed（17.28 秒）；Clippy all-targets
-D warnings 通过（37.93 秒），fmt、行数 ratchet（3245 文件）和 diff 检查
通过。完整库本轮未跑；v268 全量成绩不能证明当前新增失败已解决。

本轮是可复现缺陷与诊断证据增量，不是已通过的开发候选；未组包、提交、推送
或发布。下一步优先修复跨进程事件交付，再继续其他迁移边界，总任务保持进行。

## v268：独立进程 UIA 菜单封送与失效验证

新增独立客户端进程回归，父进程拥有真实菜单窗口和 STA 消息泵；子进程
使用 Windows UIA MTA 接口验证 CurrentProcessId 等于父 PID，沿逻辑树
展开子菜单，滚动到末尾，读取单选状态并 Invoke。父侧仅交付一次命令 39，
滚动前旧节点和关闭后的叶子均返回 UIA_E_ELEMENTNOTAVAILABLE。

客户端由当前测试二进制的精确 ignored 入口启动，未增加生产 CLI 开关。
父侧 20 秒期限，子侧 3 秒 COM 超时与 5 秒状态轮询期限；Child RAII 在
退出或 unwind 时 kill/wait 自己启动的进程，不扫描或终止其他进程。
正常退出与回收本轮已验证，超时/强制退出故障注入尚未验证。

跨进程专项父测试 1 passed（0.23 秒），其中子测试实际运行 1 passed。
完整库 2964 passed、0 failed、54 ignored（172.61 秒）；新增 ignored
项是上述仅供父测试启动的子入口，父测试在全量中也实际通过，不是跳过验证。
startup 7 passed、0 failed（16.56 秒），Clippy all-targets -D warnings
通过（35.10 秒）；fmt、行数 ratchet（3245 文件，无超过 500 行文件）和
diff 检查通过。证据位于 linshi/otter-native-v268-regression。

该证据补齐独立进程的展开/滚动/状态/Invoke/撤销路径，未覆盖跨进程事件
订阅，也不替代 Narrator/NVDA、物理输入或发布包验收。选择模式角色建模、
ScrollItem/虚拟化等仍待完成。未组包、提交、推送或发布，总迁移继续进行。

## v267：菜单勾选与单选标记的 UIA 只读状态

通过 ItemStatus 呈现已有标记：checked 为已勾选，radio 分别为已选中或
未选中；禁用项保留标记，根与分隔符返回空字符串。无 radio 且未勾选的
HMENU 项不能证明 checkbox 角色，故返回空状态，不猜测或伪造 Toggle /
SelectionItem 模式。该增量只完成标记文本呈现，不等于完整选择模式支持。
属性语义参考 [Microsoft UIA 属性合同](https://github.com/MicrosoftDocs/win32/blob/docs/desktop-src/WinAuto/uiauto-automation-element-propids.md)。

直接 COM 回归覆盖状态、禁用、分隔符、代次失效与未支持模式 S_OK + NULL；
真实 Windows MTA UIA 客户端读取菜单窗口四类状态，证明只读访问不执行命令。
复用已有生命周期、结构通知及有界消息泵，没有新增业务 owner、线程或队列。

原生菜单 68 passed、0 failed（2.64 秒）；startup 7 passed、0 failed
（17.11 秒）；Clippy all-targets -D warnings 通过（38.10 秒）。fmt、行数
ratchet（3244 文件，无超过 500 行文件）与 diff 检查通过。证据目录为
linshi/otter-native-v267-regression。本轮完整库未重跑，最近完整库仍为
v265 的 2958 passed、53 ignored，不将历史全量成绩冒充本轮验证。

Toggle/SelectionItem 角色建模、ScrollItem/虚拟化、独立进程和真实辅助技术
验收仍未完成；没有组包、提交、推送或发布，N01–N12 总迁移继续进行。

## v266：统一叶子选择的 UIA Invoked 通知与重入防护

叶子命令在统一 activate 路径发送 Invoke_Invoked，鼠标、键盘和 UIA 请求
共享通知，不在 COM Invoke 入队时提前发送。通知表示菜单接受选择，外层文档
目标准入与业务完成仍独立核验。发送后复核窗口身份、前台、捕获、取消/DPI
队列和过载状态；通知中发生重入失效则关闭菜单但不交付命令，不能因最终句柄
数值或捕获恢复原值而忽略已经发生的取消。

Panel 的 provider 懒建和通知桥归入 native_menu_panel_uia，WM_GETOBJECT
复用同一入口，系统回调前不保留 RefCell 借用。新增 MTA 系统 Invoked 订阅
分别验证 UIA Invoke、键盘消息和鼠标消息三条路径，读取关闭后的事件缓存，
每次只交付一个命令，注销后 handler 已释放。额外 HWND 回归验证通知期间
销毁和未销毁但收到 WM_CANCELMODE 时均拒绝继续交付。

最终原生菜单 66 passed、0 failed（2.48 秒），startup 7 passed、0 failed
（17.46 秒）。Clippy all-targets -D warnings、fmt、行数 ratchet（3242 文件）
与 diff 检查通过。证据在 linshi/otter-native-v266-regression，最终菜单日志
为 native_menu-final.log。本轮完整库未重跑，最近全量为 v265 的 2958 passed、
53 ignored。

ScrollItem、checked/radio 呈现、独立进程及真实辅助技术验收仍待完成；注入
键鼠消息不冒充物理设备验收。未组包、提交、推送或发布，总迁移继续进行。

## v265：菜单 UIA 焦点、结构和属性事件交付

在控制器完成全部页面关系、焦点和展开状态同步后，比较有界快照并发送 UIA
通知：活动行焦点变化、可见行代次改变后的 ChildrenInvalidated、展开/收起
属性及对应子树变化、垂直滚动百分比和可见比例属性。无监听者不保留通知基线，
不为了发送事件改变业务行为。系统调用前释放 RefCell/模型锁，逐项复核存活，
通知失败不重放已经执行的菜单操作。

新增独立 MTA 系统订阅回归：缓存读取并写入 32 项非阻塞通道，验证 A/B 两次
焦点通知、展开/收起属性通知、滚动产生根结构与百分比通知；RAII 注销三个
订阅后实际 handler Drop 已观察到。测试没有依赖业务日志或模拟事件接收。

原生菜单 63 passed、0 failed（1.81 秒）；完整库 2958 passed、0 failed、
53 ignored（183.20 秒）；startup 7 passed、0 failed（24.95 秒）。Clippy
all-targets -D warnings、fmt、行数 ratchet（3240 文件）和 diff 检查通过。
证据在 linshi/otter-native-v265-regression。

尚未完成 Invoke 通知、ScrollItem、checked/radio 呈现、独立进程和真实辅助
技术验收；可见比例通知本轮已实现但新增系统订阅测试重点验证滚动百分比。
未组包、提交、推送或发布，完整 Windows 原生迁移继续进行。

## v264：跨页焦点汇总与父行重新聚焦

修正父子页面同时报告键盘焦点的问题：保留祖先行的绘制高亮，但可访问性只
让最深的实际聚焦页报告焦点；仅展开、尚未聚焦子行时，焦点仍可留在父行。
FragmentRoot::GetFocus 沿现有弱关联、最多 16 层查找活动焦点，返回真实子页
节点。active 改变不更新节点代次，父行身份及现有 provider 保持有效。

UIA SetFocus 到已经高亮的父项时，先关闭子页再应用焦点，不能被 focus 的
同值快路吞掉；清除旧点击/拖动，不执行命令。新增 COM 回归验证子焦点汇总、
父焦点恢复与身份保留，独立 MTA 系统客户端验证子项 SetFocus 后父项不重复
报焦点、父项 SetFocus 收起子页并使旧子节点不可用。

原生菜单 62 passed、0 failed（1.54 秒），startup 7 passed、0 failed
（17.79 秒）。Clippy all-targets -D warnings、fmt、行数 ratchet（3238 文件）
和 diff 检查通过。证据在 linshi/otter-native-v264-regression。完整库本轮
未重跑，最近全量仍为 v263 的 2955 passed、53 ignored。

UIA 焦点/结构/属性事件通知、ScrollItem、checked/radio 呈现和真实辅助技术
验收继续待做；本轮不证明全局系统焦点事件发现，也不替代 Narrator/NVDA 手测。
未组包、提交、推送或发布，整体原生迁移继续进行。

## v263：跨子菜单 HWND 的逻辑 UIA 树

依据 Microsoft 的弹出窗口重设父级合同，保留各弹出窗口自己的 host provider，
将其 Fragment 父关系指向对应菜单行，父行的首末子节点指向子菜单根。
参考：[Server-side provider / Assigning a New Parent](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-serversideprovider#assigning-a-new-parent)。
页面关联由控制器同步，使用双向弱引用和代次校验，不保活已关闭的 HWND/COM
provider，不同时持有父子两把锁。内容替换或页面关闭使旧关系不可访问。

新增弱关系撤销测试及系统 MTA 客户端回归。客户端只取得根 HWND，随后沿
RawViewWalker 从父行进入子菜单、遍历首末与兄弟行，反向回到相同父行/根节点，
以 CompareElements 核验身份，最后 Invoke 子项交付命令；不直接查找子 HWND。

原生菜单 60 passed、0 failed（1.58 秒），完整库 2955 passed、0 failed、
53 ignored（185.03 秒），startup 7 passed、0 failed（26.52 秒）。Clippy
all-targets -D warnings、fmt、行数 ratchet（3237 文件）与 diff 检查通过。
证据在 linshi/otter-native-v263-regression。

跨页焦点汇总、UIA 事件通知、ScrollItem、checked/radio 呈现及真实辅助技术
验收仍待完成；本轮为系统客户端独立线程，不是 Narrator/NVDA 或独立进程验收。
未组包、提交、推送或发布，总迁移继续进行。

## v262：菜单 UIA 垂直滚动模式与滚动后节点撤销

根菜单新增 IScrollProvider：支持垂直小步/整页和 SetScrollPercent，按整行
对齐并钳制至有效范围；水平不可滚动，短/空列表报告不可滚动、NoScroll 和
100% 可见。非法枚举、NaN、无穷、越界百分比和水平滚动请求拒绝，不进入队列。
滚动请求绑定当前快照代次，仍受 32 项共享队列限制；行节点不能冒充根滚动。

控制器复用 set_first 关闭旧子页、清除选择手势、刷新可见行；滚动后的旧节点
失效，新节点才可 Invoke。新增 COM ABI 回归验证参数和状态，独立 MTA 系统
客户端将真实 40 项菜单滚至末尾，确认旧行 Invoke 拒绝，再执行新可见行。

原生菜单 58 passed、0 failed（1.33 秒），startup 7 passed、0 failed
（17.17 秒）。Clippy all-targets -D warnings、fmt、行数 ratchet（3235 文件）
和 diff 检查通过；证据在 linshi/otter-native-v262-regression。本轮完整库
未重跑，最近全量仍为 v260 的 2949 passed、53 ignored。

跨子菜单 Fragment 树、ScrollItem、UIA 事件通知、checked/radio 呈现及真实
辅助技术验收继续待做；没有将有限客户端回归视作完整 UIA 或原生迁移完成。
未组包、提交、推送或发布。

## v261：子菜单 ExpandCollapse provider 与实际状态同步

子菜单节点新增 IExpandCollapseProvider，叶子保留 Invoke，不混淆模式。
Expand/Collapse 复用有界操作队列，禁用/过期节点仍拒绝；状态读取来自控制器
已经建立的子页面。入队不会乐观改变 expanded 状态，纯展开状态或焦点变化
不撤销父节点，内容替换/滚动的身份代次规则保持不变。

Cascade 在页面建立、输入处理和轮询时同步实际 expanded 状态，鼠标、键盘、
助记符及可访问性请求共享结果。新增 COM ABI 回归证明请求与已完成状态分离、
旧/禁用节点拒绝；独立 MTA 系统客户端驱动真实子窗口展开和销毁，读取同一父
节点的状态变化，并验证关闭后旧子节点失效且没有执行父命令。

原生菜单 56 passed、0 failed（1.32 秒），startup 7 passed、0 failed
（17.52 秒）。Clippy all-targets -D warnings、fmt、行数 ratchet（3234 文件）
和 diff 检查通过，证据在 linshi/otter-native-v261-regression。本轮未重跑
完整库，最近全量为 v260 的 2949 passed、53 ignored。

尚未整合跨子菜单 Fragment 树、滚动模式和 UIA 事件通知，也没有完成
Narrator/NVDA、独立进程及动态 DPI 验收。未组包、提交、推送或发布，完整
Windows 原生迁移继续进行。

## v260：菜单 Windows UIA 发现与叶子 Invoke 接通

WM_GETOBJECT 在 STA 中懒建菜单专用 provider；Simple 提供菜单/条目名称、
快捷键、类型和可用/焦点属性，Fragment 提供父子/兄弟导航、当前 HWND 几何、
运行时身份、SetFocus，FragmentRoot 提供点命中和焦点节点。叶子 Invoke
只进入 v259 的有界队列，业务命令仍由菜单控制器及外层目标校验交付。

采用窄 COM ABI 保证未支持模式返回 S_OK + NULL、未知属性 VT_EMPTY、规范
IUnknown 及共享引用计数。面板唯一序号加入 runtime ID，窗口/行撤销后旧
provider 返回 ELEMENTNOTAVAILABLE。UIA 系统调用前释放 RefCell 借用，
销毁先撤销快照再清理 provider，不在同步窗口消息中调用 UiaDisconnectProvider。

新增真实 COM ABI 合同回归和独立 MTA 系统 UIA 客户端回归：客户端通过真实
菜单 HWND 发现根与行、读取名称/类型并 Invoke；客户端返回时命令尚未执行，
菜单线程随后 poll 才关闭菜单并返回命令。未使用伪造 provider 替代系统发现。
原生菜单 54 passed（1.09 秒），完整库 2949 passed、0 failed、53 ignored
（184.65 秒），startup 7 passed（25.03 秒）。Clippy all-targets -D warnings、
fmt、行数 ratchet（3233 文件）及 diff 检查通过，证据在
linshi/otter-native-v260-regression。

UIA 尚未完整：ExpandCollapse provider、滚动模式、跨子菜单树整合、事件通知、
完整 checked/radio 呈现及真实辅助技术验收继续待做。本轮客户端是独立线程，
不是独立进程或 Narrator/NVDA 手测；也未完成动态 DPI。未组包、提交、推送
或发布，总迁移目标继续进行。

## v259：菜单可访问性身份与有界操作链

开始补齐菜单 UIA 缺口，先建立菜单专用快照 owner 和可撤销节点，不复用
Surface 业务树。快照分离名称与快捷键，保留 enabled/checked/radio/focus 等
状态，不包含业务命令 ID、图片或裸 HWND。32 项操作队列支持 Focus、Invoke、
Expand、Collapse，跨线程只读取快照和排队，实际执行留在现有菜单线程。

Panel 刷新及销毁同步撤销状态；模型索引与代次防止滚动后的同名可见槽位冒用
旧节点。单纯焦点变化保留身份和排队 Invoke，允许同一节点先聚焦后执行。
Cascade 再核验当前可见行和 enabled/separator/子菜单类型，复用既有动作；
业务授权和交付仍在菜单退出后。关闭子页即撤销其节点，入队不等于业务完成。

测试覆盖属性解析、同名替换/关闭撤销、禁用与错误模式拒绝、跨线程队列上限、
焦点身份保留，以及真实级联 HWND 的展开/执行/收起和滚动失效。最终原生菜单
52 passed、0 failed（0.95 秒），startup 7 passed、0 failed（17.17 秒）。
Clippy all-targets -D warnings、fmt、行数 ratchet（3228 文件）和 diff 检查
通过；初次 Clippy 的 collapsible_match 已修复，最终日志带 -final 后缀。
证据在 linshi/otter-native-v259-regression，完整库本轮未重跑。

这是接入生产控制器的 UIA 前置基础，不是完整 UIA：WM_GETOBJECT、COM
Simple/Fragment/Invoke/ExpandCollapse provider、客户端发现、通知及真实
辅助技术验收仍未接入。下一步沿此身份/队列实现 provider，不能以本轮测试
宣称屏幕阅读器已可使用。未组包、提交、推送或发布，总迁移继续进行。

## v258：异步移动后的原生菜单命中区域同步

按 FEATURES 的异步重定位合同修正：之前级联命中测试和子菜单锚点只使用创建时
缓存的矩形。现在消费面板队列、鼠标、键盘与助记符前，统一核验实例并读取实际
HWND 矩形；平移更新缓存。移动前的按下/滑块拖动、半个代理字符和排队指针事件
失效，键盘与取消事件保留，避免跨坐标系误执行。外部改尺寸则安全取消，尚不
提供动态 DPI 字体/行高重排。

新增真实 HWND/SetWindowPos 回归：移动子菜单后新位置可选择命令，移动前按下
而移动后抬起不执行；排队旧点击被丢弃而键盘继续工作；外部改尺寸不执行命令。
原生菜单 44 passed、0 failed（0.90 秒），startup 7 passed、0 failed
（17.92 秒）。Clippy all-targets -D warnings、fmt、行数 ratchet（3226 文件）
与 diff 检查通过。首次测试编译中 SendMessageW 的可选参数类型不匹配，修正
测试调用后完成上述验证；没有因此修改依赖或生产 API。

证据在 linshi/otter-native-v258-regression。本轮未重跑完整库，最近全量仍为
v257 的 2936 passed、53 ignored。未做物理输入/真实多屏 DPI/UIA 验收，未读取
用户历史，未组包、提交、推送或发布。N01–N12 大任务继续进行。

## v257：历史文字与当前显示接管事务一致

修正 v256 中仍存在的显示来源偏差：文字准入中心应取当前 resolved_display，
不能取可能为空或尺寸不同的底层 canvas 图片。接管完成后再按目标文档中心
插入。display_edit 新增 Text intent，避免 Pan 工具的平移免接管分支让文字
落在隐藏底图；保留锁定、模态、后台编辑及 Art 来源约束，不改变指针平移行为。
全空白输入在接管前直接返回；非法正文仍先验证再接管。

新增回归使用真实 State/Editor/显示事务与 DirectWrite 测量，不访问用户历史：
验证空底图及 80×80 底图显示 8×4 快照时，即使工具为 Pan，文字仍居中落入
显示快照；结算后一步撤销恢复原底图。空白、NUL 与锁定目标保持显示身份和
未接管状态。新增测试位于 popup_history_adoption_tests.rs。

应用菜单定向测试 10 passed；完整库 2936 passed、0 failed、53 ignored
（178.22 秒）；startup 7 passed、0 failed（23.44 秒）。Clippy all-targets
-D warnings、fmt、行数 ratchet（3224 文件）和 diff 检查通过。日志及哈希在
linshi/otter-native-v257-regression。本轮无真实系统历史、Shell 托盘、UIA 或
动态 DPI 验收，无组包、提交、推送或发布；完整原生迁移继续进行。

## v256：历史菜单末尾入口与真正居中的文字交付

对照 docs/FEATURES.md 修正两处偏差：入口追加于现有菜单末尾，分隔行之后
命名为“剪贴板”；动态替换跟随实际父行索引，不改变已有命令。文字不再将
右键点击点或图片中心当作文字左上角，而是在测量后逐行水平居中、整批垂直
居中，保留空行间距、长行内容和单次批量撤销。托盘与右键入口共用同一实现，
托盘仍在恢复后重新核验原目标，移除托盘独有的重复坐标换算。

新增末尾顺序/命令保留与批量居中几何测试，真实 Application 隐藏窗口冒烟
进一步核验每行水平中心及首末行纵向中心。库测试 2934 passed、0 failed、
53 ignored（174.52 秒）；startup 7 passed、0 failed（17.37 秒）。应用菜单
定向测试 8 passed。Clippy all-targets -D warnings、fmt、行数 ratchet（3223
文件）与 diff 检查通过。证据在 linshi/otter-native-v256-regression。

未读取真实用户历史，未执行真实 Shell 托盘交互、屏幕视觉或可访问性验收。
本轮不包含 UIA 或动态 DPI 的实现，不组包、提交、推送或发布。N01–N12
完整迁移仍未完成，不能以本轮回归通过替代最终原生功能及资源手测。

## v255：托盘历史入口与恢复后目标复核

画布/托盘共用 popup_menu::choose 的历史会话、子菜单、缩略图和绘制 ID 映射；
托盘激活模式新增 track_tray_live，复用同一动态消息泵。MenuTarget 归到
popup_menu/target，由 desktop 内两种入口共享，不复制凭证字段或业务导入。

仅浏览或取消菜单不恢复主窗口。图片选择直接复用新钉图交付；文字选择先恢复
原窗口，再验证窗口可用、模态门禁和原 MenuTarget，按当前图片/标注框中心
确定位置并交付。恢复重入期间凭证变化则不执行。当前无图片的窗口不能承接
文字导入，返回明确错误；不凭空创建白底文档或修改原业务状态。

托盘相关 8 passed、应用菜单 6 passed、原生菜单 41 passed；startup 7 passed
（17.79 秒），均无失败。测试覆盖恢复顺序、恢复后失效拒绝、图片不恢复及
错误不重放，Clippy all-targets -D warnings、格式、行数和 diff 检查通过。
证据在 linshi/otter-native-v255-regression。首次测试编译的泛型门禁断言存在
类型推断歧义，已明确 usize 后完成最终验证；未增加业务类型可见性到 desktop 外。

本轮未重跑完整库；最近全量结果仍为 v254。真实 Shell 托盘点击/前台激活、
系统历史来源、可访问性和动态 DPI 仍未验收。未读取真实用户历史，未组包、
提交、推送或发布；整个 Windows 原生迁移目标保持进行中。

## v254：共享正文预算下的长单行文字支持

TextAnnotation 单对象上限从 4096 提高至 65536 UTF-16 单元，同时新增跨文档、
撤销/重做和导出快照共享的 32 MiB UTF-8 正文保留预算。正文与租约放在同一个
Arc owner，复制和字体调整共享正文，最后一个 owner 释放才归还预算；分配前
预留，拒绝不改变预算。CRLF 归一化按输入字节保守计费，正文相等比较不受租约
身份影响。预算不覆盖字体、COM/DirectWrite、JSON 输入和完整进程开销。

历史导入保持一行一个对象，不拆段、不截断；读取端仍保留 64 KiB 文本字节和
256 行上界。输入对话框容量沿用模型常量并更新提示，保存/恢复复用同一模型
准入。正文预算耗尽可能拒绝新的编辑/导入或大文档恢复，不能宣称任意大文档
兼容；拒绝不改写源文件，几何和其他既有文档限制仍有效。

65536 ASCII 字符单行完成真实 DirectWrite 布局准备和会话编码/恢复内容核验；
模型测试覆盖 UTF-16 边界、共享计费、拒绝原子性及 Editor 历史/快照保留 owner。
Application 冒烟新增 5000 字符单行实际导入，整批仍可单步撤销。完整库测试
2930 passed、0 failed、53 ignored（182.39 秒），startup 7 passed（17.96 秒）。
Clippy all-targets -D warnings、格式、行数（3222 文件）和 diff 检查通过。
证据在 linshi/otter-native-v254-regression。未读取真实用户历史，未组包、提交、
推送或发布；托盘历史、可访问性、动态 DPI 与整体迁移验收仍未完成。

## v253：历史内容的真实宿主交付冒烟

历史选择共用 deliver_history 准入入口：窗口失效、模态门禁尚未释放或调用者
原目标凭证不匹配时不交付。Application 隐藏窗口冒烟用自建文字/PNG 条目
经过这条生产入口，验证两行文字进入真实 Editor、保留空行间隔的准备链、
一次 undo 恢复原元素数；旧凭证不能启动图片任务，忙碌门禁不调用目标回调。
图片实际经过后台解码、pending_windows 创建独立钉图，并核验原文档不变、
钉图尺寸/模式正确，再关闭本次生成窗口。

三种启动冒烟场景强制断言 native-history-delivery-smoke 标记，不把存在函数
当作已经运行。最终 startup 7 passed、0 failed（17.46 秒），Clippy all-targets
-D warnings、格式、行数（3221 文件）和 diff 检查通过。新增模块最初注册到
application.rs 导致 502 有效行，已归入所属 application_smoke 子模块，未豁免。
证据在 linshi/otter-native-v253-regression/startup-final.log 和 verification.json。

此证据覆盖真实 HWND/State、生产交付和后台线程，但不覆盖真实系统历史读取、
菜单物理点击或前台激活。本轮未重跑完整库，未组包、提交、推送或发布。
长单行文字、托盘历史入口、可访问性、动态 DPI 及完整迁移验收仍待完成。

## v252：历史图片缩略图接入菜单绘制

后台读取复用已经完成 WIC 解码的原图，生成最长边不超过 64 像素、保持比例
的 PBGRA 小图；小图通过 Pixels 共享租约，不把原图解码缓冲留在菜单。原始
编码和缩略图一起计入历史总内容预算，小图另受共享像素预算约束。图片交付
仍使用原始编码，新钉图内容不因预览采样改变。

历史菜单行在标记栏绘制小图。绘制使用最多 16 KiB 固定栈缓冲，根据当前
普通/焦点行背景合成预乘 alpha，再交给 StretchDIBits；不在 UI 线程解码，
不新增 HBITMAP 或跨帧 DC。当前采用有界最近邻采样和现有行高，并非高质量
大图预览布局；真实截图视觉和高 DPI 仍需验收。

缩略图专测 3 passed；历史模块 23 passed、菜单 41 passed、应用菜单 6 passed，
均无失败（过滤集存在重叠，不相加当作独立全量测试）。真实 BMP/PNG 读取
夹具同时核验原编码不变及小图尺寸；合成像素和真实 HWND 双背景绘制通过。
Clippy all-targets -D warnings、格式、行数（3220 文件）和 diff 检查通过。
证据在 linshi/otter-native-v252-regression。本轮未重跑全量库/startup，最近
完整结果仍为 v251。未读取真实用户历史，未组包、提交、推送或发布。

## v251：历史会话接入画布菜单与内容交付

画布右键菜单增加“剪贴板历史”。仅在展开时订阅/启动读取，关闭子菜单或整个
菜单释放 Session；加载、空、禁用、拒绝和不可用状态显示为禁用项。文字预览
有界截取并转义 &、控制符，图片当前显示尺寸标签。动态命令使用 x64 下不与
HMENU u32 命令重叠的范围，映射绑定绘制时 ID；选择时再消费历史变更事件。

菜单关闭后先销毁历史会话、释放模态门禁，再核验 IsWindow/MenuTarget。
图片移动原编码缓冲到现有 open_job 的新钉图目标，不替换当前文档、不写剪贴板；
文字按当前字体逐行预先测量，保留空行间隔及首尾空白，最后一次批量事务提交。
组锁、显示接管和编辑忙碌条件复用 display_edit，导入失败不提交部分文字。
每行仍受通用 TextAnnotation 4096 UTF-16 单元限制；超长行返回错误，不截断。

新增无真实历史访问的回归覆盖预览注入、命令隔离、绘制 ID 不符、历史变更后
拒绝选择、只移动一次，以及行布局/超限拒绝。最初接线使 context_menu 达到
507 有效行，已把历史交付与目标复核移至所属 popup_menu，未添加行数豁免。
最终源码完整库测试 2924 passed、0 failed、53 ignored（172.26 秒），startup
7 passed（17.49 秒）；Clippy all-targets -D warnings、格式、行数（3218 文件）
及 diff 检查通过。证据在 linshi/otter-native-v251-regression；library-verified.log
是最终库结果，library-final.log 保留中间一次目标模块路径编译失败，已改为
调用者提供的目标复核闭包，未扩大 MenuTarget 的私有模块可见性。

这不是历史功能全面验收：图片缩略图、超长单行文字支持、真实 Windows 历史
来源及物理交互仍待完成。托盘菜单尚未加入历史项。自动测试不读取用户真实
历史，不记录私有条目正文；未组包、提交、推送或发布，整体迁移仍未完成。

## v250：异步子菜单替换和本地更新回调

新增 replace_submenu/submenu_open，允许历史等异步内容替换指定根条目的
子菜单。根 HWND 和鼠标捕获保持不变，已展开子页按新内容重新布局；旧按下、
滑块拖动和 UTF-16 半个字符状态清除，不能用旧手势执行新条目。未展开的
更新不额外开窗，空子菜单不转成父命令；窗口重建失败关闭菜单。

替换前检查包括静态业务条目在内的整树预算（1024 条目、16 层、单标签
4096 UTF-16 单元、总标签 128 Ki 单元，拒绝 NUL）。不合规更新保留旧模型
和旧子窗口；Cascade::open 同样验证手写树，不能绕过 HMENU 快照边界。
track_live 将非阻塞本地更新回调接入现有消息/定时器泵，没有新增线程。

最终菜单相关 39 passed、0 failed（1.12 秒），包含真实捕获保持、旧点击
拒绝、预算回滚、空/未展开替换和真实队列中的更新后选择。Clippy all-targets
-D warnings、格式、行数（3216 文件）和 diff 检查通过。日志和摘要位于
linshi/otter-native-v250-regression。未重跑全量库/startup；历史 Session
尚未与应用菜单连接，缩略图和内容交付仍未完成，不能据此宣称历史功能可用。
未访问真实剪贴板，未组包、提交或发布；大任务保持进行中。

## v249：托盘入口迁移至自绘菜单根面板

托盘入口不再调用 TrackPopupMenu，也不尝试激活隐藏业务窗口。显式托盘菜单
手势只请求激活已创建的临时菜单根面板，并校验实际前台归属；拒绝、前台在
创建期间变化或已有捕获时直接退出。主窗口/工具面板的隐藏状态不因此改变，
原恢复、退出、开机启动和工具命令分发保持不变。未添加额外常驻窗口。

画布与托盘共用 run_prepared 消息泵、捕获和清理逻辑。新增测试覆盖激活准入
拒绝后根面板销毁、隐藏 owner 不被显示，以及准入后真实队列选择和捕获释放。
测试替换激活准入而不抢占桌面前台，不证明真实 Shell 托盘手势的前台授权。
WS_EX_NOACTIVATE 窗口仍可由程序显式激活的依据已联网核实：
[Microsoft 说明](https://devblogs.microsoft.com/oldnewthing/20240919-00/?p=110283)。

菜单相关 34 passed（0.87 秒）、托盘相关 6 passed、模态门禁 2 passed，均无
失败。Clippy all-targets -D warnings、格式、有效代码行数（3213 文件）和
diff 检查通过。证据在 linshi/otter-native-v249-regression；本轮未重跑全量
库/startup，最近全量证据仍是 v248。真实托盘前台申请、可访问性、动态 DPI、
历史内容交付及整个迁移计划仍未验收完成；未组包、提交、推送或发布。

## v248：画布右键入口接入自绘菜单

desktop/popup_menu 在原 file_dialog 模态门禁内读取业务 HMENU 的有界快照，
调用 native_menu::cascade::track。业务菜单构建、动态节点快照、返回后的
IsWindow/MenuTarget 校验及 execute 分发保持原 owner，不复制业务实现。
快照/宿主错误向上传播，门禁自动释放，不回退重放到另一套菜单。

保留原 TPM_RIGHTBUTTON 的右键选择能力；模态泵记录按下按钮，仅匹配的
松开可交付点击，混合左右键松开不会执行命令。新增真实消息队列回归和
快照失败后的门禁释放回归。首次编译暴露托盘共用入口的返回类型差异；
核查其隐藏 owner 后，将托盘明确保留在 track_tray 系统入口，未把隐藏窗口
强行套入自绘宿主的前台准入，托盘自绘迁移仍待实现。

完整库测试 2913 passed、0 failed、53 ignored（197.34 秒）；startup 7 passed
（22.85 秒）。Clippy all-targets -D warnings、格式、有效代码行数（3213 文件）
和 diff 检查通过。日志及源码摘要在 linshi/otter-native-v248-regression。
自动测试证明编译、模型及消息注入链，不替代实际前台画布右键人工操作；
未访问用户剪贴板历史，未组包、提交或发布。历史交付、托盘迁移、可访问性、
动态 DPI 重排及全迁移计划仍未完成。

## v247：自绘滚动条与越出面板的滑块拖动

菜单按整行滚动，绘制、命中和拖动映射共享轨道/滑块几何；条目内容不覆盖
滚动条。轨道点击翻页，滑块以起始行偏移和指针位移计算绝对位置，极端坐标
经 i64 运算并夹取到合法范围。拖动保持根窗口捕获，越出面板的移动/松开仍
交给原页面；松开只结束拖动，导航结束拖动且清除旧点击，不能误交付行命令。

菜单相关测试 31 passed、0 failed（0.83 秒），包含真实 HWND/GetMessage
队列中的越界拖动、松开、随后新点击，以及轨道翻页、极端坐标和窄视口。
Clippy all-targets -D warnings、格式、有效代码行数（3213 文件，无超过 500
有效行项）和 diff 检查通过。证据位于 linshi/otter-native-v247-regression。
本轮未重跑完整库/startup，也未做物理鼠标或跨屏 DPI 验收。应用入口仍是
系统 TrackPopupMenu；历史内容交付、可访问性及动态 DPI 重排仍未接通。
未读取用户真实剪贴板历史，未组包、提交、推送或发布；整体迁移仍未完成。

## v246：菜单助记符解析、Unicode 输入与执行

新增助记符 owner，只解析标签中的首个未转义 &，&& 保持字面含义，不从
快捷键列提取命令。忽略禁用项和分隔符；唯一匹配执行或展开子菜单，重复匹配
循环焦点并等待 Enter。焦点显露与箭头导航统一，不复制滚动切片或业务执行。

模态循环接入 WM_CHAR/WM_SYSCHAR，非导航键经 TranslateMessage 产生字符；
UTF-16 代理对有界组装，非法单元不执行。导航/指针事件清除半个字符，不能将
前一目标的残余字符带到另一菜单；owner 销毁时再次阻止字符执行。大小写匹配
采用 Unicode 小写迭代比较，不宣称已覆盖所有键盘布局或区域性排序规则。

测试包含转义、快捷键排除、重复匹配、禁用项、屏外焦点、中文、非 BMP 字符、
代理对跨导航拒绝、子菜单和 owner 销毁，并通过真实模态队列验证 WM_CHAR
命令交付。最终菜单相关 27 passed、0 failed（0.51 秒），Clippy all-targets
-D warnings、格式、行数（3210 文件）及 diff 检查通过，证据位于
linshi/otter-native-v246-regression。本轮未重跑完整库/startup，未验证物理键盘
布局/IME 或完整视觉。应用入口仍未替换，滑块捕获、可访问性及历史交付待接通；
未读取真实历史、组包、提交或发布，总迁移保持未完成。

## v245：DPI 字体、默认/单选状态与快捷键分栏

面板拥有普通/粗体 Microsoft YaHei UI 字体，按 owner DPI 创建和释放；绘制
通过 RAII 恢复 DC，退出后字体不再被选中。默认项用粗体、radio 用圆点，
勾选、标签、快捷键和子菜单箭头分栏裁剪，窄面板也不互相覆盖。标签继续按
菜单 & 助记符显示，快捷键按字面显示；键盘助记符执行仍未接入。

Cascade 用真实 GDI 测量整页所有条目和快捷键，按工作区约束调整宽度，不因
滚动到另一切片改变宽度；没有空间时各栏省略。字体描述测试覆盖 96/144/192/
288 DPI，列布局覆盖窄/宽边界，并在真实 HWND 绘制测试中使用默认/单选项。
这是初始 owner DPI 支持，不是跨显示器 DPI 无缝重排或完整视觉验收。

首次测试 21 passed/1 failed，原因是 GetObjectW(handle,0,NULL) 删除后仍返回
结构大小 92。独立实验确认 DeleteObject 成功、GDI 资源数由 1 回到 0，实际
描述读取返回 0；测试改用实际读取验证句柄释放，不改正确的生产释放逻辑。
另按 windows 0.60 签名修正 GetTextExtentPoint32W 的 BOOL 检查。

最终菜单相关 22 passed、0 failed（0.31 秒），Clippy all-targets -D warnings、
格式、行数（3208 文件）与 diff 检查通过。证据在 linshi/otter-native-v245-regression，
首次失败日志保留；本轮未重跑完整库/startup，不能沿用 v244 回归冒充本轮结果。
应用入口尚未替换，历史内容交付等仍待接线；未读取真实剪贴板历史、组包、
提交推送或发布，总计划保持未完成。

## v244：级联菜单模态消息循环与退出清理

新增 cascade::track，公共入口要求前台 owner，不主动抢前台，拒绝覆盖已有
鼠标捕获。根面板捕获鼠标，按最终屏幕矩形路由到父/子面板；键盘在菜单循环
内消费，外点/捕获丢失/前台变化取消。30 ms 定时器只用于生命周期检查，不读
系统剪贴板。GetMessage 错误向上传递，WM_QUIT 保留退出码重新投递给外层。

处理取出的输入前重新核验窗口存活和捕获/前台状态；清理时核验根面板实例，
不对已复用的裸 HWND 操作。WM_NCDESTROY 发无载荷 WM_NULL，唤醒可能正在
GetMessage 内派发 sent 销毁通知的循环。回调不携带后台内容或裸 payload 指针。

测试使用真实窗口、GetMessage、PostMessage 和捕获，包括键盘选择、根捕获
坐标跨入 hover 子面板后点击、外点取消、已有捕获保护、捕获丢失、owner 销毁
以及 WM_QUIT 转交。测试内部不抢系统前台，因此公共前台门及物理设备输入未
验收；也不把注入消息当成人工跨应用操作证明。应用入口尚未替换，仍待 DPI
字体/助记符等呈现完善、滑块捕获、可访问性、历史 Session 与内容交付接线。

新增销毁唤醒后，首次回归出现 18 passed/1 failed：测试只筛选 WM_QUIT，
错误假设它立即可取。独立 Win32 实验重现 PostQuitMessage(73) 后再投递
WM_NULL：filtered_quit=0，正常取消息依次得到 WM_NULL 和 WM_QUIT(73)。
因此保留生产唤醒逻辑，测试改为有界读取正常消息队列验证退出码，不掩盖退出。
另外在创建窗口前固定前台身份，创建后再次核验，避免把中途切换的新前台当作
本次菜单的授权前台。

最终菜单相关 19 passed、0 failed（0.32 秒）；完整库 2899 passed、0 failed、
53 ignored（171.48 秒），实际 EXE startup 7 passed（18.47 秒）。Clippy
all-targets -D warnings、格式、行数（3206 文件）和 diff 检查通过。日志及摘要
在 linshi/otter-native-v244-regression，失败的首次 focused.log 保留，最终使用
focused-final.log。未读取真实系统历史、重新组包、提交推送或发布，总计划未完成。

## v243：真实级联面板与命令导航

Cascade 连接菜单快照和 Panel，按路径拥有真实父/子 popup；hover 展开子菜单，
上下导航跳过禁用/分隔符，左右展开/退回，Enter/Space 返回命令，Escape 退回
或关闭。超过可见范围的条目通过滚动切片显露，不以面板行数限制截断菜单树。
滚轮保留不足 120 的累积量，读取系统行数/整页偏好；焦点使用 Neuro 黄色标记。

位置使用 CalculatePopupWindowPosition 和工作区约束；按下和抬起须对应同一
当前条目，命令只移交一次并先销毁面板。外层 keyboard 路由前检查所有面板
实例仍存活，owner 销毁不能通过先到的 Enter 返回旧命令。处理消息有次数上限，
过载取消；DPI 改变暂安全关闭，不冒充已支持跨 DPI 无缝重排。

真实 HWND 测试覆盖 hover 子面板、命令返回前窗口关闭、禁用/分隔符跳过、长菜单
焦点显露、错位点击、owner 销毁、左右导航和 Escape。尚无完整模态消息循环，
物理键鼠路由、外点/失焦取消、滑块捕获、DPI 字体及可访问性仍未完成；当前
应用入口保持系统菜单，尚未连接历史 Session 或执行剪贴板内容交付。

最终菜单相关 15 passed、0 failed（0.23 秒），Clippy all-targets -D warnings、
格式、行数（3204 文件）和 diff 检查通过，证据在 linshi/otter-native-v243-regression。
没有物理键鼠或完整视觉验收；未重跑完整库/startup、读取真实系统历史、组包、
提交推送或发布，总迁移保持未完成。

## v242：真实 Win32 菜单面板窗口基础

新增自有 STATIC 子类的非激活 owned popup，分开 HWND owner、GDI 行绘制和
输入记录。稳定堆状态由 Panel 持有，先销毁窗口再释放；子类实例身份校验防止
误操作复用 HWND。创建只允许当前线程 owner，部分失败销毁刚创建的窗口。
基础绘制映射既有 Neuro panel/text/muted 色，显示文字、选中标记、分隔符和
子菜单指示；每个面板只接收最多 64 个可见行，不是菜单树总条目限制。

鼠标/键盘/滚轮/取消/DPI 消息进入容量 64 的本地队列，连续移动合并，溢出
锁定取消；owner 销毁优先于已经排队的输入，不能执行旧点击。窗口过程不调用
编辑业务，panic 在 ABI 边界转成取消。编译时修正 windows 0.60 的
GetWindowSubclass 输出参数要求 Option<*mut usize>，没有绕过身份核验。

真实 HWND 测试覆盖 ShowWindow/UpdateWindow 绘制、消息坐标/按键、更新、
窗口销毁、owner 销毁、输入过载和移动合并；这不是物理鼠标/键盘或完整视觉
验收。当前入口仍用系统菜单，尚未接齐级联控制器、hover/focus 高亮、DPI 字体、
快捷键分栏、滚动条绘制/捕获、可访问性与完整取消循环；不会提前替换原入口。

最终菜单相关 11 passed、0 failed（0.09 秒），Clippy all-targets -D warnings、
格式、行数（3201 文件）及 diff 检查通过，日志和摘要在
linshi/otter-native-v242-regression。未重跑完整库/startup，未读取系统历史、
重新组包、提交推送或发布；总迁移保持未完成。

## v241：保留业务构造的原生菜单快照

新增 native_menu::snapshot，从既有 HMENU 树提取独立所有权的只读展示数据，
保留命令 ID、中文/助记符/快捷键标签、enabled/checked/radio/default、分隔符
和子菜单；空子菜单保持 Some(empty)，不能被当作普通命令执行。不销毁调用者
菜单、不复制业务 append 逻辑，快照在原菜单销毁后仍有效。

解析限制 16 层、1024 条、单标签 4096 UTF-16 单元及总计 128 Ki 单元，拒绝
重复句柄、无效句柄和未支持的 owner-draw/bitmap 类型，避免静默丢失呈现。
测试使用真实进程内 HMENU（不显示窗口），覆盖嵌套命令/状态、空子菜单、句柄
销毁后的内容、无效/深层树及单条/总量预算。当前 desktop 未使用 bitmap 或
owner-draw 菜单，既有系统菜单展示入口保持不变。

这是替换菜单宿主时保留现有功能的数据接缝，尚未完成可见自绘窗口、输入循环
或剪贴板内容交付；未访问用户历史，未组包、提交或发布，总计划保持未完成。

复查修正 HBMMENU_CALLBACK=-1 的边界：通用 HBITMAP::is_invalid 也将 -1
视为无效，但菜单语义中它要求回调绘制，因此只允许空位图指针，增加真实
HMENU 反例。最终 native_menu 过滤测试 8 passed（含 2 项已有测试），Clippy
all-targets -D warnings、格式、行数（3197 文件）与 diff 检查通过。证据在
linshi/otter-native-v241-regression；未重跑完整库或 startup。

## v240：历史面板几何、滚动和原生宿主接缝核查

新增 List，将最终屏幕 bounds、可见行裁剪和命中放在同一模型；右侧空间不足
时左翻，纵向限位到工作区。滚动偏移、轨道翻页及 thumb 绝对拖动均有边界，
布局参数拒绝无效视口/行高和超过 25 条的数据。异步重建布局不继承旧拖动。
四项测试覆盖负坐标显示器、左右展开、底部上移、滚动后的行选择、超大拖动
坐标、空/极小视口和异步替换后的命中位置。

实际接缝核查确认，当前 context_menu 最后调用 TPM_NONOTIFY TrackPopupMenu，
没有 WM_MENUSELECT/WM_INITMENUPOPUP 或自绘菜单宿主；SurfaceWindow 是独立
overlapped window，screen_color_preview 是禁用输入的预览，均不能直接冒充
交互子菜单。不能以“点击后另开窗口”替代 hover/focus 展开的合同。

下一生产边界是菜单宿主和输入生命周期：保留 HMENU 构造 owner、命令 ID、
enabled/checked 状态、子菜单和 MenuTarget 校验，不复制业务命令；实现可交互
原生菜单面板或先用独立原型证明系统菜单与自绘面板的混合输入可靠。必须验证
跨面板指针、滚轮、thumb capture、键盘、Escape/外点和 owner 销毁，不能只验
悬停通知。现有 List/Session 仍未接入真实可见窗口，不宣称 UI 验收完成。

最终历史相关测试 23 passed、0 failed（3.08 秒）；Clippy all-targets -D warnings、
格式、行数（3195 文件）及 diff 检查通过，证据在 linshi/otter-native-v240-regression。
未读取真实系统历史，未重跑完整库/startup、重新组包、提交或发布，总计划未完成。

## v239：历史菜单会话整合读取与事件

Session 组合已有 Events 和 Task，提供 Loading/Ready/Empty/Disabled/Denied/
Unavailable 状态。UI tick 仅查本地事件标记和结果通道，没有变化时不重复读取
系统内容；读取期间事件合并成下一次刷新，旧结果不进入可选列表。启动失败
保留刷新请求，重试间隔至少 250 ms；UI 等待 6 秒取消接收但不 join 提供方。

选择前再次核对事件标记、索引和最后绘制的条目 ID，防止列表变化或重新排序
后误选旧命中位置。成功选择移交内容所有权并清空会话快照；窗口/文档目标
核验和之后的图片/文字交付仍归尚未接入的 UI owner，不以此替代目标授权。

新增五项会话测试，用合成订阅和真实线程验证初次/空闲、单次交付、事件后的
选择失效、读取期间刷新合并、失败重试节流、各终态和 UI 超时。本轮尚未接入
可见菜单，未注册真实系统监听、未读取用户历史，未重新组包或提交推送发布。

最终相关测试 19 passed、0 failed（2.83 秒），Clippy all-targets -D warnings、
格式、行数（3193 文件）和 diff 检查通过；证据在
linshi/otter-native-v239-regression。本轮未重跑完整库或 startup，总计划未完成。

## v238：历史变化事件订阅 owner

新增 Events 订阅 HistoryChanged、ContentChanged 和 HistoryEnabledChanged，
初次及后续变化只设置一个原子刷新标记，不排队内容、不读取历史、不直接调用
UI。线程绑定的 STA owner 持有注册令牌，在退出 apartment 前逐个注销；中途
注册失败也清理已成功注册的令牌。一项注销失败不妨碍其他注销，迟到回调仅
持有 Weak，不能保活已关闭的 owner。系统注销失败时不能保证系统已移除回调，
此时只保证回调不再持有内容或 UI，不将 best-effort 注销说成硬成功。

历史相关 14 项测试通过（2.99 秒），包含事件合并、迟到回调以及部分注册失败
的清理协议。测试没有注册真实系统事件，也未读取用户历史；真实事件送达和
注销仍须随菜单接线验证。代码行数检查通过（3191 文件）。

UI 尚未接入：它应仅在可以开始刷新时消费标记，并在读取期间保留新事件，
启动失败时保留待刷新状态；还需列表、滚动、定位、目标校验与选中交付。
本轮未重跑完整库/startup、未重新组包、未提交推送或发布，总计划仍未完成。

## v237：历史读取异步任务生命周期

新增 clipboard_history::task::Task，在启动后台线程前获取进程独占许可，
防止取消后仍被提供方阻塞的读取引发重复线程。每个任务通过容量为 1 的独立
通道返回一次结果，poll 不阻塞；cancel/Drop 发出取消并关闭接收端，不 join。
后台失败或 panic 导致通道断开时返回固定错误，不输出剪贴板内容。

测试以真实线程和通道模拟提供方，覆盖菜单销毁不等待、阻塞期间拒绝新任务、
返回后许可释放、结果单次交付、panic 断连、显式取消丢弃已排队结果。
最终 clipboard_history 相关 12 passed（2.92 秒），Clippy all-targets
-D warnings、格式、行数（3189 文件）及 diff 检查通过。日志位于
linshi/otter-native-v237-regression/focused.log。本轮未重跑完整库或 startup，
v236 的完整回归不能当作 v237 已重跑。测试不访问真实系统剪贴板历史。

尚未连接菜单 UI；独立通道只隔离任务结果，应用结果前仍须验证窗口、文档和
菜单目标身份。事件订阅、自绘列表、文字预算与选中交付继续待实现。
总迁移保持未完成；未重新组包、提交、推送或发布。

## v236：WinRT 剪贴板历史只读后端与取消生命周期

新增独立后台 STA 历史快照读取，只处理 Bitmap/Text，不写系统剪贴板、不落盘
或记录内容。提供 Ready/Disabled/Denied 状态，最多 25 条、单图编码 4 MiB、
快照内容总计 16 MiB、单条文字 64 KiB/256 行；Bitmap 优先，保留真实编码，
不将 BMP 冒充 PNG。复用 WIC 解码，在输出像素分配前限制 8192 边长和
64 MiB 像素预算；这不是 COM 字符串分配或 WIC 内部工作区的硬内存隔离。

进程级独占许可保护整个读取生命周期。5 秒阈值触发协作取消，Cancel 请求后
等待异步操作离开 Started，避免提供方仍运行时释放许可并积累重复读取。
提供方同步调用和取消确认没有硬超时保证，后续 UI 任务 owner 不得阻塞 join；
必须在启动线程前取得许可，并在菜单关闭时取消、丢弃迟到结果和安全脱离。

测试仅使用内存 DataPackage、合成 PNG/BMP 和模拟异步操作，不读取真实系统
历史。实际发现 DataPackage 在 SetText/GetText 时会截断嵌入 NUL，因此修正
原先把不可观测原文当成输入的错误夹具；仍拒绝读取器实际可见的 NUL。
最终 focused 9 passed，全量库 2868 passed、0 failed、53 ignored（186.76 秒），
实际 EXE startup 7 passed（19.17 秒）。Clippy all-targets -D warnings、格式、
行数和 diff 检查通过；日志及摘要位于 linshi/otter-native-v236-regression。

历史列表 UI、事件订阅、菜单定位/自绘滚动条、选择后的目标授权与图片/文字
交付仍未接通；长文本与编辑模型总预算仍待完成。N06 和 N01–N12 总计划均
未完成，不能将模拟测试作为实际 Windows 历史功能验收。最新交付 ZIP 仍是
v234，v235/v236 尚未重新组包；没有提交、推送或发布。

## v235：N06 剪贴板历史缺口核对与多行文字原子事务

从实际 FEATURES 合同和 native 调用链确认，当前原生只读取当前剪贴板图片/
文件，尚无 Windows 历史列表生产接线。智能文件复制已存在，不因旧表记录
重复开发。N06 下一完整边界仍是只读 Bitmap/Text 历史、状态/列表与事件更新、
图片独立钉图、文字居中批次、目标失效、订阅/任务清理及实际 Windows 验收。

本轮先补真实编辑模型前置：Editor::insert_text_batch 最多 256 条，所有几何、
文档/历史预算和 ID 准备成功后才提交，一次 revision、一次 undo；任一后续行
失败不会留下前半批元素或清空 redo。复用既有历史 batch 所有权转移，不克隆
整份文档；单条文字插入复用同一 Stroke 构造逻辑，保持既有行为。

修改前全量 2854 passed、0 failed、53 ignored（200.35 秒），日志位于
linshi/otter-native-v234-full-regression。修改后全量 2859 passed、0 failed、
53 ignored（194.75 秒）；实际 EXE startup 7 passed（19.36 秒）。Clippy 初次
发现测试的 field_reassign_with_default，已用结构初始化修正，最终 all-targets
通过；格式、行数与 diff 检查通过。证据在 linshi/otter-native-v235-regression。

这不是历史菜单完成：WinRT 读取、列表、布局和异步目标授权尚未接通；未读取
用户的真实剪贴板内容。现有 TextAnnotation 单条 4096 UTF-16 单元限制与历史
总文本合同需后续一并处理，不能直接抬高限制而绕开整文档/历史内存边界，也不能
以缩小支持范围冒充 64 KiB/256 行合同。当前交付 ZIP 仍是 v234，未将 v235
模型变更冒充已组包版本；没有提交、推送或发布，总计划保持未完成。

## v234：独立原生内部 ZIP 与实际解包启动验证

新增 package-native-candidate / verify-native-package 入口，stage 先核对候选
和当前源码，再收集许可证、复制 EXE、生成构建身份摘要与包清单；Archive
门拒绝安全路径错误、重复、缺失/额外文件、字节摘要不符、构建身份或许可
清单不一致。保留 unsigned/internal/non-publishable 状态，不调用 Tauri 发布链。

Windows PowerShell/.NET 初次 CreateFromDirectory 产生反斜杠归档路径，导致
按正斜杠查找条目失败；已改为显式写入统一路径，并显式加载 Compression
程序集。失败输出均保留，最终候选/包位于 linshi/otter-native-v234-candidate-complete
及 linshi/otter-native-v234-package-complete，不沿用失败收据。

最终 ZIP：Otter-native-v0.2.33.234-windows-x64-internal.zip，298 个文件，SHA256
`8453e165dd7589a79f71c8d2c1a8179718824bcef1bb95ac0cd7e406055168d9`。
EXE SHA256 保持 `04fa72e2c877bb44be34e1c093f96ed0478a3930343c70247ce0780b0b0c9123`。
实际解包到 linshi/otter-native-v234-unpacked，重新校验清单中的 297 个载荷
文件（清单自身另占 1 个条目），在独立 cwd/TEMP/TMP 下启动 --smoke-test
退出 0，输入/锁定、SVG/WebP child 与最终销毁标记齐全，临时图片残留 0。
运行日志和验证记录在 linshi/otter-native-v234-runtime-Gcjm2E；可重复脚本为
linshi/verify-native-unpacked-v234.cjs。归档合同 7 项通过，覆盖有效包、篡改、
缺失、额外、越界、大小写重复和字符串假布尔值。

这是实际 ZIP 解包后的本机冒烟证据，不是干净机器、完整功能、性能、安全
或签名验收；构建与包清单 runtimeVerified=false 继续表示未通过整体运行门。
N10 尚不能全项关闭，N01–N12 总迁移继续进行；没有提交、推送或公开发布。

## v233：原生分发许可证文本收集

新增 native-candidate/licenses.cjs，以离线 Cargo metadata 的 native 根为
起点遍历全部平台 normal/build 边，排除 dev-only 分支；不复用 npm/Tauri
清单，也不把 license 字段非空当作已有分发文本。受补丁依赖先验来源身份，
registry 路径限制在当前 Cargo 缓存内；文件遍历有深度、文本大小和总容量边界。
所有文本按原字节复制，inventory 绑定元数据、补丁来源和逐文件摘要。

实际图有 164 个包，选中 157 个；7 个 dev-only 包（含 rcgen/yasna）不纳入。
材料位于 linshi/otter-native-v233-license-materials-verified，共 293 份文本、
1,421,756 字节，逐份重新检查字节数及 SHA256 通过。收集结果明确保留
legalClearance=false，不冒充许可证义务或安全批准已完成。

新增 4 项图/材料合同与已有 20 项候选/来源/SBOM 合同全部通过，覆盖循环、
dev/normal 混合边、不完整图、未知依赖类型、目录筛选、实际材料和路径越界。
证据与日志在 linshi/otter-native-v233-tests；行数、语法与 diff 门通过。
独立 Release v233 候选重新生成并验证，EXE 与 v232 字节相同（本轮仅构建
材料脚本变化），源码身份更新为
`1c8392db0bb68dfdb80485744f7a5a6d38bef0872d52d0dfef6f743fb4674e95`。

ZIP 组包、归档内容核验和从解包产物启动仍是下一交付边界；本轮没有生成
正式分发 ZIP、签名或发布，也没有重跑原生全功能套件。N10 和总迁移继续未完成。

## v232：移除启动探针对编译机器图片路径的运行时依赖

open/paste/drop/group_lock 四条启动探针原先通过 CARGO_MANIFEST_DIR 读取
仓库 PNG；现改用 application_smoke_image 的内嵌字节和独占临时文件。
文件模式仍进入真实 WIC、工作线程、文件拖放与窗口链，不用模拟成功替代。
create_new 成功后才获得清理责任，写句柄先关闭，作用域结束仅删除本次文件。
模块挂在既有 application_smoke 下，application 主 owner 保持原有行数。

新增生命周期/WIC 测试 1 项、实际 EXE startup 7 项通过；Clippy all-targets、
fmt、行数 ratchet 通过。初次单测漏初始化 COM 导致 CO_E_NOTINITIALIZED，
已按现有 Apartment 测试约定修正；不是放宽断言或绕开 WIC。

v0.2.33.232 独立 Release 候选成功构建并通过源码前后与字节身份检查：
`linshi/otter-native-v232-candidate/payload/otter-native.exe`，10,652,672 字节，
SHA256 `04fa72e2c877bb44be34e1c093f96ed0478a3930343c70247ce0780b0b0c9123`。
复制到 `linshi/otter-native-portable-v232-cFMPiS`，用独立 cwd/TEMP/TMP 运行
完整 --smoke-test，退出 0，四条输入/锁定探针、SVG/WebP child 和最终窗口
销毁标记齐全，图片临时文件残留 0。该目录保存 stdout/stderr 与 verification.json；
可重复验证脚本为 linshi/verify-native-portable-smoke-v232.cjs。

本机源码仍可访问，故本次是脱离仓库工作目录运行与已知路径依赖修复的证据，
不是干净机器/系统隔离证明。ZIP、分发许可证、完整功能/性能、正式签名与
安全 clearance 仍待完成；候选构建收据仍不授予整体 runtimeVerified 或发布资格。

## v231：独立内部 Release 候选与构建字节身份

新增 build-native-candidate 入口，固定 offline/locked/MSVC Release 产品 bin，
不借用旧 Tauri 发布通道。源码、受审查补丁依赖、工具/配置摘要前后比较，
收据绑定最终 EXE、构建日志和 Cargo metadata。输出必须是仓库外新目录，
缓存独占锁覆盖 Cargo 退出后的复制窗口；PowerShell 原生命令启动失败不能
沿用旧成功退出码。正常 stderr 不误判成构建失败。

实际 Release 编译用时 5m 40s。编译期间修正脚本后，首次候选因源码身份变化
被拒绝，保留 linshi/otter-native-v231-candidate 的 FAILED.txt 和 build.log。
稳定脚本用同一缓存重新运行，Cargo 0.40s 验证完成，候选通过最终身份检查：
`linshi/otter-native-v231-candidate-verified/payload/otter-native.exe`，
10,650,112 字节，SHA256
`b61558f1d10a6c4b636bebec8eabc3575df16fada026e1d991d9eba0f35440de`。
源码集合 SHA256 为
`17b4561140daa4c33a8f1542ed7d13b434e20a4e4f6968c7dc12ef2171d08545`。

合同验证：新增 Node 收据 9 项、现有来源/SBOM 11 项、PowerShell 工具与锁
5 项通过；仓库内输出、缓存重叠、既有候选输出均实际拒绝。语法、Rust fmt、
行数 ratchet 通过。证据归 linshi/otter-native-candidate-v231-tests。

这是 dirty、未签名的内部构建候选，明确 publishable/runtimeVerified/signed
均为 false。没有增加运行时功能或重跑完整原生功能套件；本轮不声称 ZIP、
分发许可证收集、包级便携、签名、安全 clearance 或人工验收已完成。
缓存锁只协调本入口，不能同时允许裸 Cargo 改写同一缓存。收据不是密封、
签名或全部工具链可复现性证明。N10 和 N01–N12 总计划继续保持未完成。

## v230：补齐真实 EXE 的 PE 图标与版本资源接线

v229 实际 EXE startup 7/7 通过后，进一步运行已有 exe_resources 门，
复现 Missing resource：旧 build.rs 仅嵌入 Common Controls manifest，
没有产品 PE 的图标/版本资源。窗口运行时图标存在不能替代这条包身份门。

新增构建期 windows_resources owner，用系统 rc.exe 或 llvm-rc.exe 生成
资源，仅链接到 otter-native 产品 bin；支持显式 OTTER_NATIVE_RC 路径，
工具缺失/失败必须报错，不悄悄跳过资源。复用现有品牌图标，公共版本仍
0.2.33，内部迭代 v0.2.33.230；无 Cargo 依赖变化，无 Tauri/WebView 接入。
详细构建约束见 [WINDOWS_NATIVE_BUILD.md](WINDOWS_NATIVE_BUILD.md)。

本轮证据目录 linshi/otter-native-pe-resources-v230。N10 尚不能整体勾选：
独立原生候选组包、最终字节 provenance、包级便携验收和安全 clearance
仍需完成；历史 NativeCandidateAcceptance 脚本仍是 WebView2 验收，不得
误用于独立原生宿主。SVG 小数 offset 与其他迁移剩余项未因本次构建修复关闭。

实际验证：build_resources 2、exe_resources 1、profile_startup 3、startup 7
均通过。exe_resources 再加强为各尺寸图像 payload 逐字节等于品牌 ICO；
planes/bitcount 按图像头校验，而非误要求资源编译器保留 ICO 目录零值。
Clippy all-targets、fmt、行数 ratchet 与 diff 检查通过。四个受影响 Rust
文件有效行为 22/100/30/93；锁和第三方身份未变，SBOM 重生 1257 项。

保留的实际 Debug EXE 位于该证据目录 runtime/otter-native.exe，SHA256
b4b4c4a23f0496d22ed760a7713eaa427b6271180457357bc8dbe6f0d33b6cb1，
大小 35,547,136 字节；verification.json 绑定源码集合、EXE、日志和来源。
Windows 读取文件描述为 Otter Windows Native，FileVersion/ProductVersion
为 0.2.33，OriginalFilename 为 otter-native.exe。没有遗留本次产品进程。
本轮实测 LLVM rc 后端；Windows SDK rc 后端尚未实测。未重跑全量库测试、
未构建 Release/ZIP、未签名或发布，收据不是正式 release provenance。

## v229：修复作者结果名与 renderer 内部结果名混淆

在引用语义检查中复现并修复 usvg 未命名结果的内部 result1 名称被显式
in/in2/merge 输入误引用的问题。保留规范的未知/前向引用默认回退，不以
拒绝这些输入代替正确实现；同名结果取最近前项、SourceGraphic/SourceAlpha
及隐式输入不变。旧代码 5 组回归中 3 组失败，候选 Debug/Release 均通过；
8 个 Chrome 纯色夹具的 DOM、Canvas 显示和读回均支持预期行为。

两文件窄修复经 45 文件来源重放后导入，来源补丁/清单同步，版本推进至
v0.2.33.229。生产滤镜准入仍关闭，offset 像素差异和完整参数验证仍未完成。
原生回归与身份收据见 linshi/otter-filter-reference-candidate；不能将这个
局部引用修复视为 N08 或完整 N01–N12 迁移完成。

导入后原生库全量串行 2853 passed / 0 failed / 53 ignored；Clippy、fmt、
行数、来源合同和许可证门通过，1257 项 SBOM 已绑定新 usvg 源码身份。
本轮没有重建产品 EXE 或发布，完整滤镜准入与迁移剩余项仍继续推进。

## v228 后续：offset 的路径差异已分离，仍不导入回退候选

已将语义参考固定到实际 Chrome 对应 Skia revision，确认 Offset 明确使用
nearest 采样而非通用双线性。新增 30 个三路径观察：60 项显示/读回一致，
旧 DOM/Canvas 半像素邻域 48 项重采不变，6 项零位移控制一致；但额外的
willReadFrequently Canvas 与默认 Canvas 仅 12/30 相同，与 DOM 仅 9/30
相同。不能用额外路径替换原验收，也不能用统一 .5 特判掩盖路径区别。
源码身份、逐项像素及边界记录见 SURFACE_SVG.md 顶部。生产代码、版本、
准入均未变；下一实现仍需分离取样、几何 AA 与裁剪传播，不宣称完成迁移。

## v228 隔离开发：offset 元数据候选尚未达到导入条件

已将源几何边界接入隔离 renderer 候选并实际运行双基线对照。24 个样本
Canvas 精确项从 5 到 12，DOM 从 3 到 10，但两个负半像素中心样本回退，
故没有导入生产或放宽验收。新增半像素邻域实测进一步排除“只给 .5 加特判”
的捷径。候选代码、逐项差异和浏览器观察见 SURFACE_SVG.md 顶部及对应
linshi/otter-offset-geometry-candidate、otter-offset-half-boundary 收据。
本轮生产源码与 v0.2.33.228 不变；下一步仍需补齐采样/裁剪组合语义。

## v228 后续：定位 offset 的源几何边界信息缺口

新增 40 项实际浏览器观察/控制，并用当前固定 renderer 完成 16 项原生
对照。四类零位移输入像素完全相同，小数位移后浏览器按输入边界分成两类，
而原生结果未保留该区分；usvg 的 object/stroke bbox 本身仍有正确区别。
下一修复必须贯通源几何边界，不能只替换像素插值或用 alpha 扫描代替几何。
详见 SURFACE_SVG.md 顶部及 linshi/otter-offset-input-bounds/verification.json。
这是新定位证据，不是 offset 修复或准入完成；本轮没有改生产代码与版本。

## v228：累计预算在真实受限 child 中的执行证据

最终 Release 资源探针已通过：10 组滤镜样本及各自后续正常请求，共 20 个
真实 Job/IPC child；5 个精确 FilterBudgetExceeded，15 个成功。最大 Job
提交内存 37,736,448 字节，最慢样本 3,029 ms。两项审查缺口已收紧：不能
用其他 renderer 错误冒充超额，也不能因解析丢失滤镜而假通过。

测试入口与 Job 峰值查询仅 cfg(test) 可用；生产仅把错误映射移到 decode
边界，保留原 InvalidData 行为，不开放滤镜、不改变额度、Job 或截止时间。
该探针绕过功能白名单且成功只回传小确认帧，因此不代表完整生产滤镜请求、
大图传输、像素等价或全部极值资源已经验收。范围详见 SURFACE_SVG.md v228。
证据目录为 linshi/otter-filter-work-resource，最终身份及 gates 见 verification.json。

收紧后的直接后端合同 1 passed；native SVG 串行回归 146 passed / 0 failed /
5 ignored（新增两个显式探针入口）；进程容量/Job 内存/超时取消回收合同
3 passed。Clippy all-targets、格式、行数及 diff 门通过，结束时未发现残留
Otter 原生进程。本轮未重跑完整 native lib、未重建产品 EXE、未重新生成
SBOM 或执行在线漏洞扫描；不能把 v227 全库/启动证据改称 v228 重验结果。

内部版本 v0.2.33.228；没有提交、推送或发布。Windows 原生总计划保持未完成，
下一生产边界仍包括浏览器像素差异、资源准入与最终整体功能验收。

## v227：共享累计滤镜工作量预算已接入原生 worker

完成上一轮待补的 renderer 累计预算接线：按实际执行次数、source 栅格面积
和 primitive 参数扣减额度，嵌套渲染共用同一份状态，整数溢出/超额均失败；
worker 将错误映射为 InvalidData 并丢弃部分像素。新模块与精确边界、重复
执行及嵌套传播回归已从固定 resvg 归档重放导入；具体模型和局限见
SURFACE_SVG.md v227，不将策略积分冒充 CPU/耗时或内存实测上界。

本轮 native SVG 串行为 146 passed / 0 failed / 3 ignored；完整 native lib
串行为 2847 passed / 0 failed / 51 ignored。此前 UIA 间歇失焦的失败证据
仍保留，本次全库通过不是该问题的根因修复。生产滤镜准入未开放，不改 Job、
字体 owner、解析进程边界或 UI 交互。内部版本 v0.2.33.227；证据目录为
linshi/otter-filter-work-budget，最终 gate/源码/二进制身份记入 verification.json。

重建产品 EXE 的 hidden_window_renders_and_destroys_without_webview 启动检查
1 passed；native all-targets Clippy（-D warnings）、格式、行数和 diff 门通过。
来源/SBOM 合同 11 passed，双 SBOM 核验 1257 个锁定包身份；许可证核验
749 个 Rust 包、40 类表达式。本轮没有运行新的在线漏洞扫描，许可证检查
不等于无漏洞证明。启动检查后未发现残留 Otter 原生进程。

下一边界仍是 SVG 浏览器像素差异、资源标定和准入验收；N01–N12 总计划
没有标记完成。本轮未提交、推送或发布，没有把开发检查代替用户全面手测。

## v226：全库复核与 UIA 间歇失败现场

最终源码的完整 native lib 串行测试为 2846 passed / 0 failed / 51 ignored，
Clippy all-targets、格式、行数及 diff 门通过。本轮仅增强测试失败诊断：
点击依赖失败记录 keyboard/焦点；可选 UIA trace 和 Select 失败现场记录
前台 HWND/PID/thread，避免事后用旧句柄猜测 owner。不修改生产事件排序、
焦点行为、超时、重试或通过条件，内部版本为 v0.2.33.226。

点击编辑九项测试组连续十轮通过。UIA 的 EDIT→Select 最短顺序第九轮
曾失败；跟踪复现显示 ComboBox 展开后发生失焦及收起，原生条目仍为 2，
但列表窗口已隐藏。现有证据不能确定夺焦来源，也不能证明上一轮点击失败
与此同因。新增 owner 诊断后的二十轮跟踪及二十轮普通顺序通过，不等于
消除间歇问题。保留全部失败记录，不能把最新全库通过写成根因修复。
证据在 linshi/otter-v225-suite-order-audit/verification.json。生产 SVG
依赖与拒绝 policy 不变，累计像素工作预算等主线工作继续；N01–N12 未完成。

## v225：实际执行坐标检查与 worker 错误通道

已修复上一轮五项 turbulence 坐标/stitch 溢出反例，Debug/Release 均返回
明确失败，正常两项保持修复前逐字节像素。render_checked 贯通嵌套渲染
错误，真实 worker 将其映射 InvalidData；完整来源重放已接入仓库。
不开放 filter/fe*，不把数值拒绝当作极值像素兼容通过。累计工作预算、
其余像素差异和 N01–N12 仍未完成。证据与范围见 SURFACE_SVG.md 和
linshi/otter-turbulence-coordinate-fix。
本次 SVG 专项串行 145 passed / 3 ignored；完整 lib 串行仍有 2 failed
（2844 passed / 51 ignored），分别涉及 UIA 焦点和点击依赖事件快照。
两项单独复跑通过，不替代全库通过或根因定位，留作后续验证缺口。
实际 EXE 隐藏启停冒烟 1 passed，Clippy/格式/行数/来源与双 SBOM 门通过。

## v224 后续：坐标执行风险已有实际反例

七项 16×16、最多九 octave 的 renderer 对照确认五项 Debug 整数溢出，
包含频率 .2 配合有效小缩放；当前解析预算会接受，但生产功能 policy
仍拒绝这些 SVG。对齐 native 锁后的 Release 不 panic 不等于像素正确，
未将测试成功当作安全或兼容放行。故障点/夹具和哈希证据在
linshi/otter-turbulence-coordinate-audit，详见 SURFACE_SVG.md。
这是已确认的待修问题，本轮无生产代码/版本变更，N01–N12 未完成。

## v224：修复滤镜候选已复现的 seed 整数溢出

真实 16×16 turbulence 渲染复现 i32::MIN 取负 panic；resvg 改用无符号
绝对值取模后，16 类渲染与正规化正 seed 逐像素一致，库测试 21 passed。
已从官方归档重放完整补丁并接入仓库，刷新来源与双 SBOM；没有升级
依赖或开放滤镜。其他坐标/累计预算问题仍未完成，详见 SURFACE_SVG.md。
证据在 linshi/otter-turbulence-seed-audit，N01–N12 未完成。
接入后 SVG 145 passed / 3 ignored，来源合同 11 passed；库/native 静态
检查及格式/行数门通过，双 SBOM 核对 1257 身份；未重跑产品 EXE 或全库。

## v223：修正滤镜预算与真实 parser 的转换差异

真实 usvg 对照复现 order=9.5 对应 9×9 核却被预算错误拒绝。修复将 order
保留为 f64/trunc，octave 仍按 f32/round；相同测试从失败变为通过，全部
SVG 相关测试 145 passed / 3 ignored，静态/格式/行数门通过。未修改依赖
或开放滤镜；证据在 linshi/otter-filter-order-audit，详见 SURFACE_SVG.md。
这不是剩余坐标安全、累计工作预算或完整迁移验收，N01–N12 未完成。

## v222：滤镜进入 renderer 前的预算与模板依赖保护

真实 SVG worker 已接入流式滤镜解析预算：限制定义/节点、数值列表、
卷积阶数与 octave，不分配整份数值列表、不改写输入。原非递归引用图
增加滤镜模板/应用边，保护环、深度及重复 ID。功能白名单仍未开放滤镜；
变换后数值安全、总像素工作预算、语义与像素等价仍待补齐，不把预算
单测当作滤镜功能验收。详见 SURFACE_SVG.md，N01–N12 未完成。
全部 SVG 相关测试 144 passed / 3 ignored，产品 EXE 冒烟 1 passed；
Clippy、格式、行数与 diff 门通过。四个依赖身份不变，证据位于
linshi/otter-v222-filter-budget；不是全原生套件或全迁移验收。

## v221：原生渲染依赖从实验目录接入仓库

已将重放一致的 resvg 41 文件与最新 usvg 45 文件接入固定 third_party，
native Cargo 解析不依赖 linshi；只变更 resvg 锁来源，不变版本/features。
生产 filter/fe* 准入保持关闭，真实 child 增补普通/极值滤镜拒绝合同。
最终原生库 2839 passed / 51 ignored / 0 failed，SVG 专项 55 passed，
实际 EXE 的隐藏 Win32/D2D/图片 child 冒烟 1 passed；Clippy all-targets、
格式、来源/SBOM 合同、许可证和行数门通过。联网 Advisory 仍有 25 个漏洞 ID，
没有新增安全豁免。回执在 linshi/otter-v221-source-import-20261022T093212858814Z，
详见 SURFACE_SVG.md。这是实际仓库接入，不是滤镜功能或整个迁移验收；
N01–N12 未完成，没有提交、推送或发布。

## 候选依赖：原生 SVG 实际链路非回退集成

外部测试工程直接编译真实 native 源码，使用候选 resvg/usvg 与仓库
tiny-skia/roxmltree；生产文件哈希保持不变。按二进制清单补齐全部含
svg 名称的测试，138 passed、3 个父测试调用的私有入口 ignored，覆盖
真实 child/字体 broker、拒绝合同、文本和应用安装绘制。证据位于
linshi/otter-native-svg-integration-20261022T091640934885Z。不是完整原生
套件或新滤镜验收，filter/fe* 准入仍拒绝，N01–N12 未完成。

## 候选数值审计：turbulence 非有限频率修复

165 个小 SVG 解析组合复现 baseFrequency=1e39 的 unwrap panic，隔离
副本增加有限性检查后通过，四项非有限频率渲染全透明。原型回归 152
passed、morphology 专测 2 passed；未宣称全部数值组合的执行资源安全。
证据在 linshi/otter-filter-numeric-audit，生产源码与准入未变，完整来源
包/资源门尚未更新，N01–N12 未完成，详见 SURFACE_SVG.md。

## usvg 修复：完整来源补丁与重放验证

原始 usvg crate 的完整补丁已包含 morphology 修复和既有文本改动，
45 文件重放精确一致（9 修改、5 新增、无删除），仓库 schema-2 文件/
补丁校验通过。消费重放目录的普通回归 152 passed，修复专测 2 passed；
资源 Job 未在此次重放 EXE 重跑。产物位于
linshi/otter-usvg-replay-20261022T090245436190Z。未生产导入或修改准入，
正式安全/发布门与 N01–N12 未完成，详见 SURFACE_SVG.md。

## morphology 修复：大图资源边界验证完成

修复解析器的新 Job 覆盖十项 2048² 极大半径/零轴组合，全图结果断言
通过，峰值 68177920 字节。修复后完整原型另重跑九项资源 Job，全通过，
最高峰值 69857280 字节；十个独立进程均正常退出，无 15 秒超时，128 MiB
上限不变。证据为 linshi/otter-morphology-parser-audit/resource-verification.json。
尚未更新正式来源包或生产准入，N01–N12 未完成，详见 SURFACE_SVG.md。

## 候选数值边界：morphology 半径溢出修复

实际复现小 SVG 的非有限/缩放溢出半径导致 parser unwrap panic。在新
隔离副本仅修正数值转换，有限大半径保留饱和语义，非有限值透明处理。
相同两项测试旧版 2 failed、新版 2 passed；完整原型 152 passed /
9 ignored / 2 filtered。补丁和前后证据在 linshi/otter-morphology-parser-audit。
尚未重新打包或跑新资源 Job，生产源码/滤镜拒绝不变，N01–N12 未完成。

## aligned 源码包：库测试与静态检查完成

在源码/manifest 不变的独立 library-suite 中，20 项 resvg 库测试及
Clippy -D warnings 通过。测试副本 lock 允许补齐上游可选依赖清单，
metadata 确认实际仅 text feature，未启用图片/系统字体可选依赖；
roxmltree 仍为仓库补丁路径。记录位于 aligned staging/library-suite。
这是额外的内部测试证据，不替代生产准入、安全门或 N01–N12 验收。

## aligned 候选：新依赖图九项资源测试通过

仓库 roxmltree 接线后的新 EXE 已独立执行全部 9 项资源 Job，9/9 passed、
退出码均 0、无超时；128 MiB 上限不变，最大峰值 69873664 字节。证据
绑定新 EXE 与前一轮源码/Cargo metadata，未沿用旧依赖图资源结果。
普通 152 项回归沿用同一构建，资源日志在 aligned staging 的
resource-suite-20261022T0844409669443Z。未生产导入或开放滤镜，正式准入、
等价与 N01–N12 未完成，详见 SURFACE_SVG.md。

## 候选依赖图：补齐仓库 roxmltree 接线验证

发现旧原型仍解析 registry roxmltree；新建隔离 harness 对齐仓库补丁
roxmltree/tiny-skia，usvg 从仓库副本重放唯一 morphology 差异。三个
正式来源清单校验通过，Cargo metadata 确认四个候选依赖路径，新旧 lock
仅 roxmltree 来源改变。全新构建 152 passed / 9 ignored / 2 filtered；
新图资源 Job 尚未重跑。未修改生产依赖或准入，N01–N12 未完成。
证据位于 linshi/otter-resvg-aligned-20261022T084155683062Z，详见 SURFACE_SVG.md。

## source-only 候选：九项资源 Job 全部补齐

同一新 EXE 的 9 项 ignored 已逐项在独立进程实际执行，9/9 passed，
无超时、退出码均 0；均确认 128 MiB Job 限制，峰值最大 69873664 字节。
覆盖文本、blur/morphology、矩阵链、component 四种负载及极端 offset。
普通 152 项回归沿用上一轮同一构建；此次没有冒充生产 IPC 或宿主验收。
证据位于 source-only staging/resource-suite-20261022T0837382175982Z，
详见 SURFACE_SVG.md。未生产导入或开放滤镜，N01–N12 未完成。

## resvg：source-only 候选完成干净构建与回归

新源码包恢复上游 Cargo 元数据/lock，排除两个未接线本地试验模块；
41 文件重放精确一致，补丁仅含 6 个 src 修改和 9 个 src 新增。外部
harness 在全新 target 上 152 passed / 9 ignored / 2 filtered；新 EXE
的全 alpha 2048² Job 通过，峰值 53223424 字节。新增模块均不超过
136 有效行，既有 mod.rs 历史债务从 898 降至 871。未生产导入或放开
滤镜；来源审查、安全/发布门、等价及 N01–N12 未完成。详见 SURFACE_SVG.md。

## resvg 候选：完成原始归档到编译的诊断重放

原始 crate 与 native lock 固定 SHA-256 一致；独立补丁重放后 43 文件
精确匹配当前候选（8 修改、11 新增、无删除），从重放目录实际执行
20 项库测试通过，编译后文件哈希保持。修正仅为命令级 Git 换行设置，
未改变全局配置。证据位于 linshi/otter-resvg-replay-20261022T082906812184Z。
这是 diagnostic 候选的可重建证据，尚含诊断 Cargo 接线及试验模块，未
生产导入或重跑整个原型/资源 Job；正式来源准入及 N01–N12 未完成。

## component：双后端完整灰阶回归

相同 40 项灰阶在软件浏览器实测：候选 31 项精确，总差 288；默认浏览器
仍为 22 项精确，总差 1212。两个浏览器基线本身只有 22 项一致，均保留，
未替换验收标准。二进制探针进一步确认 linearRGB 有 4 个输入位置出现
后端索引差 1。新增 31 项软件严格断言后原型 152 passed / 9 ignored /
2 filtered；renderer、生产滤镜拒绝和 N01–N12 状态不变。详见 SURFACE_SVG.md。

## component：输出误差出现浏览器后端差异

固定函数表与输出 alpha 的新探针中，默认浏览器 sRGB 有 25 个灰阶
位置偏离数学预乘；禁用 GPU 后四行全部精确，原生候选也逐字节匹配
软件浏览器。新增严格回归后原型 151 passed / 9 ignored / 2 filtered。
linearRGB 两路径仍有不同误差，未为硬件输出拟合固定偏置。renderer
和生产准入未改，N01–N12 未完成；后端状态和证据详见 SURFACE_SVG.md。

## component：真实浏览器查表索引与新版基线

Chrome 156 新会话中，四组二进制函数表探针重建的 1024 个索引均匹配
当前模型，候选四张 SVG 逐字节匹配浏览器。新增严格回归后原型为
150 passed / 9 ignored / 2 filtered。另重采原 40 项灰阶，三种浏览器
观测与冻结旧基线全部一致，排除本次版本漂移。剩余误差继续定位输出
阶段，未改变 renderer 或生产准入；N01–N12 未完成。详见 SURFACE_SVG.md。

## 滤镜候选：component 查表精度修复

独立模型与候选 40 项灰阶逐字节一致；浏览器精确项 18→22，总绝对差
2964→1212，无单项总差回退，新增四项浏览器严格断言。原型 149 passed /
9 ignored / 2 filtered，候选 20 项单测和 Clippy 通过。全 alpha 2048²
独立 Job 两种工作空间 185/192 ms，峰值 53841920 字节，128 MiB 限制下
正常退出。仍有 18 项灰阶不精确，未开放生产滤镜；N01–N12 状态不变。
证据及浏览器基线使用范围见 SURFACE_SVG.md 和
`linshi/otter-component-lut-evidence.json`。

## 滤镜验证：component 缓存覆盖全部输入 alpha

未过滤输入实测覆盖 0–255 alpha；两种工作空间下的大图逐像素匹配小图
空间放大参考，并验证零输入生成输出 alpha。2048² 独立 Job 两项耗时
243/224 ms，峰值 53792768 字节，128 MiB 上限未提高且正常退出。原型
149 passed / 9 ignored / 2 filtered，新增 Job 已独立执行。没有修改算法
或放宽浏览器验收，生产 v220/filter 拒绝与 N01–N12 未完成状态保持；
完整范围与证据见 SURFACE_SVG.md。

## 滤镜候选：component alpha 变换取得浏览器回归依据

为旧 12 项负载补浏览器基线后，确认原像素仅 3 项精确；不是以旧输出作为
最终正确性。扩展融合路径处理输出 alpha 与透明输入生成覆盖率，12 项现
全部浏览器精确。另 24 项 RGBA 灰阶 9→13 项精确，总差 13140→1488，
无单项总差回退；先前 RGB-only 和恒等结果不变。原型 148 passed /
8 ignored / 2 filtered，20 候选单测、Clippy 及独立 128 MiB Job 通过。
仍有灰阶差异和线性输入路径未解决，生产 v220/filter 拒绝保持；详见
SURFACE_SVG.md，N01–N12 未完成。

## 滤镜候选：RGB-only component 融合修复两个离散边界

从 sRGB 输入融合去预乘、工作空间转换与函数表索引，避免中间线性预乘
量化；仅限 alpha 保持不变的非恒等 RGB 变换，其余路径保持。16 项灰阶
3→5 项精确，总差 10476→1476，无单项总差回退，两个半透明 discrete
严格通过；仍有 11 项未精确。原型 147 passed / 8 ignored / 2 filtered，
新增 2048² Job 独立执行通过，峰值 52891648 字节；候选 20 单测、Clippy
通过。范围、模型和资源证据见 SURFACE_SVG.md。生产 v220/filter 拒绝及
N01–N12 未完成状态保持。

## 滤镜诊断：完整灰阶排除输入差异并暴露分段边界

16 项非恒等 component 灰阶覆盖全部 256 个字节值；三种浏览器观测一致，
候选仅 3 项精确，半透明 discrete 最大差 76。独立模型重现全部原生输出；
未过滤输入与 PNG 解码字节相同，矢量/PNG 经过滤镜后浏览器结果也相同，
排除本组输入栅格化差异。精确去预乘模型修复 sRGB 分段，但使部分 linearRGB
回退，故未改 renderer。原型 147 passed / 7 ignored / 2 filtered；证据
及后续边界见 SURFACE_SVG.md。生产 v220/filter 拒绝与 N01–N12 未完成保持。

## 滤镜候选：降低非恒等 component 的逐像素计算开销

大于等于 256 像素时采用固定 256 项通道表，小图保留标量；沿用原计算函数，
不改公式或舍入。9×256 字节比较及 12 项跨切换边界 SVG 回归通过。2048²
gamma 独立 Job 三次实测 579/559/553 ms→136/152/160 ms，像素相同，峰值
内存均为 52822016 字节；仅代表该负载。原型 146 passed / 7 ignored /
2 filtered，新增 Job 独立执行通过；候选 19 项单测和 Clippy 通过。
详见 SURFACE_SVG.md。生产 v220/filter 拒绝不变，N01–N12 仍未完成。

## 滤镜候选：component 恒等链保留颜色转换并省去重复处理

60 项新基线三种浏览器观测一致；整段跳过颜色转换造成 10 项回退，已拒绝。
保留工作色彩空间转换、只省去恒等函数的去预乘/处理/再预乘后，60 项原像素
不变，40 项浏览器精确，20 项未解决。共享输入裁剪回归通过；20 节点 2048²
链在 128 MiB Job 内正常退出，峰值 52789248 字节。原型 145 passed /
6 ignored / 2 filtered，新增 Job 已独立执行，候选 18 单测与 Clippy 通过。
详见 SURFACE_SVG.md；生产 v220/filter 拒绝和 N01–N12 未完成状态保持。

## 滤镜候选：修复算术精确选择输入的颜色损失

严格选择第一个/第二个输入的 arithmetic 现在直接移交原 Image，保留真实
色彩空间，不做有损往返转换。新增浏览器基线 12/24→24/24 精确，覆盖输入
alpha、第二输入位移和 primitive 裁剪；24 项后续颜色矩阵串联对照通过。
原有两组 96 项普通 composite 像素不变。原型 143 passed / 5 ignored /
2 filtered，候选 17 项单测、Clippy 通过。证据见 SURFACE_SVG.md。
修改仍仅在隔离候选；生产 v220/filter 拒绝与 N01–N12 未完成状态保持。

## 滤镜覆盖纠正：补上真正的 composite 输入 alpha

先前 48 项 composite 使用整体 `opacity`，不是滤镜输入透明度。保留原组，
另增 `fill-opacity` 48 项：30 项浏览器结果与原组不同，三种显示/读回观测
全部一致；候选精确 29 项并加入严格回归，19 项未解决。独立模型重现旧组
18 项算术输出，未找到完整像素等价的统一量化方案，不据最低总体误差改规则。
原型 141 passed / 5 ignored / 2 filtered。详细纠正与证据见 SURFACE_SVG.md；
renderer/生产 v220/filter 拒绝不变，N01–N12 未完成。

## 滤镜验证：composite 全运算符组合基线

新增 48 项 over/in/out/atop/xor 与三种算术组合，覆盖两种色彩空间和三种
透明度；DOM/Canvas 显示/读回全部一致。候选 34 项精确，已新增严格回归；
余下 14 项未解决。颜色转换融合试验存在单项回退，已撤回并验证 48 项原
像素恢复，不以总体误差下降冒充完成。原型 139 passed / 5 ignored /
2 filtered、Clippy 通过。详细证据见 SURFACE_SVG.md；生产 v220/filter
拒绝与 N01–N12 未完成状态保持。

## 滤镜验证：offset 候选的极值和资源边界

36 组极大位移/缩放透明输出断言通过；独立 Windows Job 内 8 项 2048²
位移工作负载通过，峰值 69648384 字节，128 MiB 限制未提高，15 秒截止
未触发且进程正常退出。普通原型 137 passed / 5 ignored / 2 filtered，
新增 ignored Job 已单独执行；候选 resvg 16 项单测通过。证据和范围见
SURFACE_SVG.md。本轮仅补验证，生产 v220/filter 拒绝不变，未完成显示
等价、生产 IPC 准入或 N01–N12。

## 滤镜候选：offset 浮点平移与双路径回归

排除本组差异只来自 Canvas 读回或默认 primitive 区域：Canvas 自身截图
和读回 48/48 一致，SVG img/Canvas 仍有差异，PNG 输入也能出现差异。
候选保留默认采样，以浮点 Transform 替代绘制前整数截断；24 项对照
Canvas/DOM 的单项总差均无回退，分别精确 5/24、3/24，尚不等价。
新增半像素严格回归与 25 组整数位移断言，原型 136 passed / 4 ignored /
2 filtered，Clippy 通过。证据见 SURFACE_SVG.md；生产 v220/filter 拒绝
不变，N01–N12 未完成，不把候选改善当作正式准入。

## 滤镜诊断：offset 小数位移尚未形成可接纳修复

24 项新基线暴露整数截断；独立双线性试验有 10 项回退，已撤回接线。
实际 DOM 与默认 Canvas 仅 19/24 一致，五项最大通道差 127，不能沿用
blur/flood 的读回等价结论。证据与待解边界见 SURFACE_SVG.md。
恢复后原型 134 passed / 4 ignored / 2 filtered，Clippy 通过；生产 v220
及 filter 拒绝不变，N01–N12 未结项，不把本轮诊断计作迁移完成。

## 滤镜候选：修正 flood 的提前 opacity 量化

候选在颜色预乘前保留浮点 opacity，64 项总差 6016→5248，无单项总差
回退，精确数仍 22。alpha 边界未解决，没有特判。预乘顺序回归及原型
133 passed / 4 ignored / 2 filtered、Clippy 通过；v220/filter 拒绝保持，
N01–N12 未结项。证据见 SURFACE_SVG.md。

## 滤镜诊断：透明度扫描排除全局截断

1031 次 alpha 观测中 feFlood 与普通矩形仅 6 次相差 1（包含重复的 .5）；
其余符合最近整数舍入。65 个独立 SVG 图像重现合并扫描结果，未发现合并
绘制伪差。量化机制尚未确定，未加特判或改准入；v220 与 N01–N12 未完成
状态保持。完整数据见 SURFACE_SVG.md。

## 滤镜诊断：feFlood 的 DOM 显示基线

64 项颜色/透明度基线中原生仅 22 项精确，其余最大差 1；16 项真实页面
截图全部匹配默认 Canvas，不能归咎于读回伪差。.5 与 .501 的不同结果
也不支持简单截断修补。本轮保存证据、未改 renderer；v220/filter 拒绝
保持，N01–N12 未结项，详见 SURFACE_SVG.md。

## 滤镜候选：回收已无后续引用的中间图

按命名结果最后使用位置释放旧中间图，保留真实长寿命分支和同名解析语义。
1024²、20 级交换链 Job 峰值约 90.5→18.3 MiB，输出不变、退出 0；
命名分支回归通过，96 项模糊快照不变。原型 131 passed / 4 ignored /
2 filtered，Clippy 通过；v220/filter 拒绝保持，N01–N12 未结项。
证据和限制见 SURFACE_SVG.md。

## 滤镜候选：实际 Job 下的两组资源负载

1024²/2048² 独立进程在 128 MiB Job 上限、15 秒外部截止下执行四类
实际 SVG 滤镜负载，均退出 0、无超时；峰值约 18.3/67.2 MiB。普通回归
129 passed / 3 ignored / 2 filtered，新增两项忽略的 Job 测试另行通过。
这是候选诊断而非生产 IPC/产品验收；v220/filter 拒绝保持，N01–N12 未结项。
限制与 EXE/源码/日志哈希见 SURFACE_SVG.md 和 otter-filter-job-evidence.json。

## 滤镜候选：小双轴核一次量化

依据公开 Skia 路径线索，独立实现有界小二维卷积。40 项白色样例精确数
16→18，新增两项严格断言；32 项彩色总差略降，两组无样例总差回退。
候选库 16 项、原型 129 项通过，Clippy 通过；不是生产 Job 或跨设备
验收，v220/filter 拒绝保持，N01–N12 未结项。详见 SURFACE_SVG.md。

## 滤镜诊断：排除全局舍入修补

固定卷积核的独立模型未支持把颜色转换改为截断，或只把卷积改为浮点；
八项小 sigma linearRGB 总差均大于现有 round/round。高 alpha 区间同样
存在误差，不能只对近透明像素修补。保存定位证据，未改 renderer/生产
版本/准入；N01–N12 未结项，详情见 SURFACE_SVG.md。

## 滤镜候选：跨空间链路与通道选择保真

新增八组矩阵/模糊跨空间串联基线。纯 RGB 选择矩阵不再往返颜色空间，
单次 linearRGB 交换和两组双交换精确；其余六个链路仍有差异。27 种映射
及系数扰动反例通过，候选库 15 项、原型 129 项通过，Clippy 通过。
v220/filter 拒绝不变，N01–N12 未结项；详见 SURFACE_SVG.md。

## 滤镜候选：矩阵与颜色转换融合

候选显式矩阵不再量化中间工作颜色图，返回像素保留正确的输入颜色空间
标记。六组输出与独立模型一致；sRGB 三组精确，linearRGB 三组最大差
均降至 1，尚非完全一致，多级串联仍待验证。候选库 14 项、原型 128 项
通过，Clippy 通过；v220/filter 拒绝不变，N01–N12 未结项。

## 滤镜候选：非恒等矩阵分离验证

新增六组彩色矩阵浏览器基线，候选合并显式矩阵中间量化后 sRGB 增益、
alpha 增益、通道交换三组精确一致；linearRGB 增益出现回退，仍不放行。
候选库 14 项、原型 128 项通过，Clippy 通过。生产 v220/filter 拒绝不变，
N01–N12 未结项，证据和剩余边界见 SURFACE_SVG.md。

## 滤镜候选：修复恒等颜色矩阵的额外损失

64 色阶×8 alpha 的五组浏览器基线揭示 linearRGB 恒等矩阵不必要往返
引入最大差 6。候选精确识别恒等矩阵并保留输入后五组全像素一致，另验证
任一系数的微小变化不被误跳过。候选库 13 项、原型 127 项通过，Clippy
通过；非恒等转换仍待验证。v220/filter 拒绝保持，N01–N12 未结项。

## 滤镜诊断：先否证全浮点假设，避免无效管线重写

独立模型的 8 位路径逐字节重现 16 项 Rust 候选；同样例改全浮点后 sRGB
总差 228→1013、linearRGB 6036→11992，未支持“全浮点即可兼容”的假设。
暂不引入更大中间缓冲，继续分离颜色往返转换与采样。模型断言通过，未改
renderer/生产版本/准入，N01–N12 未结项。详情见 SURFACE_SVG.md。

## 滤镜候选：合并颜色转换中的量化

隔离候选将反预乘、传递函数、再预乘合并到一次写回舍入。32 项彩色样例
总差 40219→37416，仍 10 项精确，一项大 sigma 双轴贴边回退，颜色与
性能问题仍未结清。候选库 12 项、原型 126 项通过，Clippy 通过；没有
导入生产来源，v220/filter 拒绝保持，N01–N12 未结项。详见 SURFACE_SVG.md。

## 滤镜候选：新增彩色半透明浏览器基线

32 项默认 Canvas 对照中，8 项零模糊控制组及 2 项非零样例精确一致；
其余 22 项仍有差异。小 sigma 单轴 linearRGB 的 alpha 一致但颜色有差，
明确需要独立排查颜色路径，不能继续只优化大半径缩放。10 项精确结果已
固定严格断言；原型 126 passed / 1 ignored / 2 filtered，不是全像素或
生产验收。v220/filter 拒绝不变，N01–N12 未结项；证据见 SURFACE_SVG.md。

## 滤镜候选：输入缓冲借用与像素保持

移除联合缩放候选首次降采样前的整图复制，明确借用输入到拥有中间帧的
生命周期边界。24 项归档像素逐字节不变；新增所有权测试，候选库 10 项、
原型 124 项通过，Clippy 通过。不是生产峰值内存证明；v220/filter 拒绝
保持，N01–N12 未结项。证据及限制见 SURFACE_SVG.md。

## 滤镜候选：双轴联合缩放，生产仍不放行

隔离候选改为双轴降采样、低分辨率卷积后一次重建。24 项浏览器对照总差
85968→72416，但仍仅 6 项完全一致、最大通道差 8，两个贴边样例轻微回退。
独立二维模型 23/24 逐字节一致；候选库 9 项、原型 124 项通过，后者另有
1 ignored / 2 filtered，不是生产完整库验收。整图副本和中间帧引入新的
峰值内存验证责任，尚未通过生产 Job/来源重放/产品验收。v220 与 filter
拒绝状态不变，N01–N12 未结项；证据及准确限制见 SURFACE_SVG.md。

## 滤镜候选：逐步降采样替换大 sigma 近似

独立模型后实现保留浮点边界/像素中心的逐步降采样与重建。24 项完全一致
从 1→6，最大通道差 12→8；仍有一项总差回退和 18 项未完全一致。
候选库 9 项、原型 124 项通过，大参数 debug/release 断言及 Clippy 通过。
生产保持 v220/filter 拒绝，未进行来源重放/Job/产品验收，N01–N12 未结项；
证据与边界见 SURFACE_SVG.md。

## 滤镜基线：默认 Canvas 与真实页面显示一致

64 项浏览器复核发现默认/readback Canvas 有 59 项不同，不能把读回路径
当作唯一显示语义。16 个真实 DOM img 截图全部逐像素匹配原默认基线，
64 项默认样例也重现旧证据，因此不改换较易匹配的验收目标。三个归档合同
通过；生产/候选 renderer 与 v220 不变，仍需继续验证大 sigma 缩放/采样，
不是原生滤镜或真实 WebView2 验收。详情见 SURFACE_SVG.md，N01–N12 未结项。

## 滤镜候选：大 sigma 继续修复而非放行

24 项新浏览器样例复现大 sigma 全透明错误。隔离候选把同轴三盒卷积合并，
用固定项前缀矩计算，避免中间裁剪/量化及半径规模循环；大参数 debug/release
断言通过。最大通道差 65→12，但仅 1/24 完全一致，sigma 9 中心两项总差
还有回退，不能生产放行。候选库 8 项、原型 123 项通过；v220/filter 拒绝
状态不变，N01–N12 未结项，完整证据与限制见 SURFACE_SVG.md。

## 滤镜候选：高斯模糊核误差收敛

40 个白色图形浏览器样例分离出 kernel/边界误差。隔离候选为较小 sigma 引入
有限可分离 Gaussian 卷积，两个轴独立选择；完全一致从 0/40 提升至 16/40，
最大通道差从 64 降至 1，其余 24 项仍未算通过。候选库 6 项、原型 121 项
通过，Clippy 通过。本机内核测量不能替代生产 Job 预算；v220/filter 拒绝
状态不变，N01–N12 未结项，完整证据与剩余范围见 SURFACE_SVG.md。

## 滤镜候选：morphology 半径边界与资源开销

56 项新浏览器基线使上一候选暴露零轴、小数取整、透明边界和大半径窗口问题。
隔离候选修复后 56 项全像素一致，另通过 2450 组独立朴素 oracle；改为两遍
单调队列，不让采样工作随半径面积增长。候选库 4 项、原型 119 项通过，
Clippy 通过。尚非生产 Job/来源重放验收，v220 与 filter 拒绝状态不变，
N01–N12 未结项；证据和剩余滤镜范围见 SURFACE_SVG.md。

## 滤镜阶段：定位真实差异，生产仍 v220

18 项 Chrome 滤镜像素基线揭示原 renderer 的小数区域少列与 morphology
窗口偏移；linshi 隔离候选修复后 13 项完全一致，并以严格断言固定。
普通/单轴模糊、颜色转换及半透明取整仍有五项差异，未开放生产 filter/fe*。
候选原型 118 passed / 1 ignored / 2 filtered，不是 Job/产品 EXE 或来源门
验收。证据与后续边界见 SURFACE_SVG.md；未新分配版本，N01–N12 未结项。

## v220：SVG paint-order 正式准入与真实 EXE 像素验证

在浏览器和隔离 renderer 各 24 项显式分层等价通过后，接入生产属性准入并
增加严格 token 校验，避免库的非法值默认回退。真实 child 26 项通过；当前
完整库 2839 passed / 51 ignored / 0 filtered，产品 EXE --smoke-test 退出 0，
新增先描边后填充的逐像素检查通过。Clippy、行数与来源完整性门通过，未改动
第三方源码或预算。内部分配 v0.2.33.220，未提交推送发布，N01–N12 未结项；
具体证据、CSS 与人工验收边界见 SURFACE_SVG.md。

## v219：长度单位和修复后产品 EXE

新增三项实际 child 单位等价回归，覆盖绝对单位、继承字号 em、根/嵌套视口
百分比及两种 lengthAdjust，全部通过。当前 scalar policy 下重新构建实际
EXE，--smoke-test 退出 0、RTL 长度像素检查通过。不是完整库重跑或完整单位
算术验收，版本未变，N01–N12 未结项；证据见 SURFACE_SVG.md。

## v219：修复 textLength scalar 溢出/下溢静默准入

真实 child 复现 1e100 被接受后，在 policy 补 f64→f32 表示检查，拒绝溢出和
非零转零下溢，保留 0/-0/普通负值语义。当前完整库 2810 passed / 51 ignored /
0 filtered，Clippy/行数门通过；不等于全部单位换算算术边界已证明。N01–N12
未结项，版本未变，证据和 EXE 验证边界见 SURFACE_SVG.md。

## v219：新属性经过实际应用安装与 EXE child

Application 新增长度/方向 SVG 的授权安装/逻辑尺寸/上传/重复绘制场景，内联
9 项通过；重新构建的 startup 7 项通过，实际 EXE --smoke-test 退出 0 且新增
RTL 长度像素比较通过。Clippy/行数/diff 通过；本轮不是完整库重跑或人工验收。
版本未变，未提交推送发布，N01–N12 未结项；详情见 SURFACE_SVG.md。

## v219：长度与方向正式准入

textLength/lengthAdjust 与 ltr/rtl direction 进入原 SVG policy，六模块新增 57 项
真实 child 测试；CSS style 仍拒绝且有明确反向测试。默认完整库 2809 passed /
51 ignored / 0 filtered，Clippy/来源/行数门通过。并非完整 SVG/bidi 或新 EXE
UI 安装验收；Advisory 未放行，N01–N12 未结项，详细证据见 SURFACE_SVG.md。

## v219：候选依赖迁入仓库固定路径

usvg 45 文件、roxmltree 210 文件完成固定路径接入与来源/Cargo 实际解析核验。
新增深层文字及非法 XML 的生产 child 回归；默认库测试 2752 passed / 51 ignored /
0 filtered，全目标 Clippy 通过。SVG 新属性白名单未开放，未改变 Job/授权 owner。
来源/SBOM 合同 11 项、行数门及许可证通过，双 SBOM 1257 身份一致；Advisory
仍有 25 漏洞 ID，不能发布放行。完整证据及边界见 SURFACE_SVG.md，N01–N12 未结项。

## v219：来源门补齐新增源码模块合同

分配内部版本 v0.2.33.219。正式来源校验支持 schema 2 的 upstream/local-added
和精确 added 清单；7 项来源测试、3 项 SBOM 测试、安全合同/版本/行数门通过。
生产依赖身份与 SVG 白名单不变；候选 usvg/XML 快照的独立文件哈希核验通过，
但不是签名认证或生产接入。N01–N12 未结项，详细范围见 SURFACE_SVG.md。

## v218 后续：双层递归问题的组合修复

现场地址及 A/B 复现定位 XML tokenizer 递归和其后的文字收集栈压力。
隔离 roxmltree 迭代 tokenizer + usvg 叶节点提取组合修复原 512 层失败：
逐像素及后续请求通过；完整候选库 2806 passed / 51 ignored / 1 filtered，
Clippy 通过。XML 原有测试及新增 4096 层结构测试通过，补丁重放 210 文件一致。
生产未接入；不是任意深度或完整迁移验收，N01–N12 未结项。证据见 SURFACE_SVG.md。

## v218 后续：发现候选深层文字子进程栈溢出

32/128 层嵌套 textLength 的真实 child 像素及后续请求通过，512 层失败；
Windows 事件确认 0xc00000fd。拆出叶 span 构造的尝试未修复，已恢复原型源码，
保留失败测试和诊断快照。未收紧准入来隐藏故障，也未声明完整回归通过。
下一步需定位实际递归栈；生产未接入候选，N01–N12 未结项，见 SURFACE_SVG.md。

## v218 后续：候选长度范围查询索引

候选 usvg 用源字节区间索引代替每个约束的全簇筛选，并恢复视觉顺序保护 bidi。
原型 92 项通过，新的可重放快照接入副本后真实 child 55 项通过；4096 约束
单次 debug 耗时 4128 ms，不宣称显著提速，后续位置平移的二次增长尚未解决。
生产依赖/入口未变，完整库回归尚未针对新快照重跑；详情见 SURFACE_SVG.md。

## v218 后续：候选属性真实 child 压力验证

隔离副本新增同级约束压力测试，32/512/4096 约束均通过原 IPC/字体代理/Job
链路；最大 8192 字符、135303 字节，debug 解码 4488 ms，随后普通请求像素正确。
没有放宽 15 秒/128 MiB 预算，未采集峰值内存，不代表最坏输入验证完成。
生产入口和版本不变，N01–N12 未结项；详细范围与证据见 SURFACE_SVG.md。

## v218 后续：隔离准入副本完整回归

格式化后重建副本并执行完整库测试：2804 passed / 0 failed / 51 ignored /
1 filtered out，148.61 秒；唯一主动排除项是未准入 CSS style 的正向样例。
原有库测试及新增 54 项 child 用例同轮通过，重建二进制哈希与日志已留证。
生产来源门再次通过，版本和准入入口不变，N01–N12 未结项；详见 SURFACE_SVG.md。

## v218 后续：隔离准入副本的真实 child 验证

native 源码副本仅改实验 policy 和测试注册，其余 1513 原文件逐字节一致。
新增属性经原 Image::from_surface、IPC、字体代理及 Job child 的 54 项测试
通过，副本 lib/tests Clippy 通过。CSS style 仍拒绝，一项正向样例明确排除。
生产源码/入口未变，正式来源门和完整语义仍待完成，N01–N12 未结项。
完整证据与范围见 SURFACE_SVG.md 顶部。

## v218 后续：候选依赖的真实 native 回归

独立 target 下，以已验证重放快照覆盖 usvg，真实 native 库测试 2750 passed /
51 ignored，startup 7 passed，全目标 Clippy -D warnings 通过。生产 policy、
Cargo.toml/lock 与默认依赖身份不变；新长度和方向属性仍未开放。
这是现有链路兼容性验证，不是新增功能 child 验收或发布，N01–N12 未结项。
完整证据与候选身份见 SURFACE_SVG.md 顶部。

## v218 后续：混合文字段落方向修复

确认测试字体覆盖后，新增六项 Arial 拉丁/希伯来混合基线，定位固定 LTR 的
shaping 基础方向并修复，同时保留 writer 段落方向和逻辑锚点。release 回归
92 passed，独占 Job 探针未重跑；嵌套 bidi/完整字体回退和生产接入仍未完成。
生产 v0.2.33.218 不变，N01–N12 未结项，证据见 SURFACE_SVG.md 顶部。

## v218 后续：RTL 锚点修复

十二项浏览器像素基线定位并验证 RTL start/end 归一化，覆盖自然长度与两种
lengthAdjust、方向继承和覆盖；补三种锚点的 writer 往返。release 常规回归
85 passed，独占 Job 探针本轮未重跑。未宣称完整 bidi/字体回退完成，生产版本
与入口不变，N01–N12 未结项；证据与当前补丁重放目录见 SURFACE_SVG.md 顶部。

## v218 后续：组合符号与单排版簇修复

新增七项浏览器确认的排版簇像素基线，修复单簇 spacing 子约束漏记 resolved
而被父级重复缩放的问题。release 回归 72 passed，独占 Job 探针本轮未重跑，
两个 policy 测试跳过。候选补丁重新重放验证通过，生产版本/入口保持不变。
复杂 shaping/bidi/字体回退及生产接入仍待完成，N01–N12 未结项。

## v218 后续：新增模块补丁来源重放

隔离验证器从官方归档核对 40 个原文件、8 个修改和 4 个明确 local-added 模块，
实际重放补丁后全清单/全字节一致；八项拒绝测试通过。实验清单 schema 尚未进入
生产 CI，不能视作依赖接入完成。生产来源门复核通过、版本仍 v0.2.33.218，
N01–N12 未结项；证据见 SURFACE_SVG.md 顶部。

## v218 后续：release 与诊断 OS Job 验证

原型 release 回归 65 passed；新增独占进程 OS Job 探针实际核对 128 MiB、
单进程、kill-on-close 限制并在 15 秒外部 deadline 内完成。4096 约束解析、
光栅渲染、导出约 166/2.3/5.8 ms，系统 Job 峰值约 21.6 MiB，exit=0。
这是诊断进程的限额实测，未替代生产 IPC/policy/font broker 和完整输入验收。
生产仍为 v0.2.33.218，N01–N12 未结项，证据与边界见 SURFACE_SVG.md 顶部。

## v218 后续：包围盒热点修复

阶段计时定位到逐 span 重复构造完整 chunk bbox；局部单条缓存保留原 bbox
语义并在变换变化时重算。1024 约束 debug parse/layout 约 2.13 秒降至 291 ms，
4096 约束/8192 字形首次约 4.71 秒。65 项适用原型测试通过，真实 Job 资源门
尚未完成，不能据此关闭 N01–N12。生产版本/入口不变，证据见 SURFACE_SVG.md。

## v218 后续：长度约束规模验证

原型实测 1024 兄弟约束/2048 字形，受限复用同参整形后 debug parse/layout
约 3.95 → 2.15 秒，64 项适用测试通过。仍存在明显超线性增长，未通过真实
Job 内存/超时与 release 性能门；不可把单次优化当作资源验收完成。
生产仍为 v0.2.33.218，N01–N12 保持未结项，证据见 SURFACE_SVG.md 顶部。

## v218 后续：嵌套缩放分段修复

新增标签/零 dx/零旋转浏览器基线确认分段边界，隔离原型修复上一轮三项失败，
并修复 writer 丢弃零值定位导致的回读差异。目前 61 passed / 0 failed，
两项 policy 拒绝测试跳过；复杂整形/字体回退/bidi、资源与来源门仍待完成。
生产 v0.2.33.218 及入口未变，N01–N12 未结项；证据见 SURFACE_SVG.md 顶部。

## v218 后续：嵌套缩放反例与部分修复

新的 Chrome 组合基线暴露四项原型失败；已修复父级重复缩放子级字形的一项，
其余三项父子 spacingAndGlyphs 的位置分组问题保留启用测试继续处理。
当前原型 54 passed / 3 failed，不能继续使用历史 50 passed 作为全绿结论。
生产 v0.2.33.218 不变，尚未开放入口，N01–N12 未结项。证据见 SURFACE_SVG.md。

## v218 后续：导出路径身份修复

隔离原型以导出局部路径表修复重复 ID，并保留独立引用几何及可见源路径。
七项 writer 测试覆盖 38 次往返，原型全部可用测试 50 passed；两项生产 policy
拒绝测试不适用于 direct renderer，明确跳过。usvg 与原型 lib Clippy 通过。
生产仍未接入长度约束，复杂语义、预算与来源门尚待完成，N01–N12 保持未结项。

## v218 后续：长度约束 writer 与路径状态回归

隔离原型新增约束 writer，16 组输入的 32 次有/无 ID 前缀往返通过结构和像素
检查；曲线路径多 chunk 回读失败已先复现再修复。五组回归共 43 项通过。
重复路径定义、复杂 shaping/组合语义、来源门和真实 Job 预算仍未完成。
生产仍为 v0.2.33.218，详见 SURFACE_SVG.md 顶部；N01–N12 不据此结项。

## v218 后续：长度约束进入隔离渲染链路

实际原型 renderer 原 18 项由 9/9 改为 18 passed，新增八项嵌套/跨 chunk
真实像素断言通过，六项解析与五项路径回归保持通过。生产尚未接入，仍为
v0.2.33.218；writer、复杂 shaping、组合语义、来源清单和真实 Job 门未完成。
实现与证据边界见 SURFACE_SVG.md 顶部，不据此关闭 N01–N12。

## v218 后续：元素级长度约束原型

隔离原型已保留独立于样式的元素约束、跨 chunk 字节范围和嵌套身份，六项
解析测试通过；独立 spacing 探针复现八组新的 Chrome 坐标/像素来源基线。
真实 renderer 尚未接入该求解器，旧 18 项用例仍 9 passed / 9 failed；
生产版本/来源不变、textLength 仍未开放。下一接入边界见 SURFACE_SVG.md 顶部。

## v218 重叠文字绘制修复

最终全量 2750 passed / 51 ignored（142.27 秒），startup 7 passed（15.54 秒）；
Clippy/fmt/3151 文件严格行数及来源/许可证/SBOM 门通过。OSV advisory 25 个
漏洞 ID 仍未放行，N01–N12 仍在进行中，没有提交推送发布。

usvg flatten 不再合并相邻 outline glyph，保留重叠 AA、透明度与描边顺序。
真实 child 修复前失败，修复后 SVG 34 项通过，包括 4096 glyph 预算回归。
代价是每 glyph 独立 Path；未改变 Job/IPC/授权安装，未开放仍缺跨样式与
跨 chunk 约束的 textLength。详情与剩余边界见 SURFACE_SVG.md。

## v217 后续：textLength 接入前调查

已建立 18 组 Chrome 155 像素对照，隔离原型最终 15 passed / 3 failed。
确认长度约束不能依附样式 TextSpan：需要保留嵌套与跨 chunk 范围；压缩文字
另有坐标一致但重叠绘制不同的问题。生产仍为 v0.2.33.217，尚未开放该属性、
未接入实验补丁。失败证据与下一实施边界见 SURFACE_SVG.md 顶部，不计为完成。

## v217 路径文字负 dx 修复

最终完整库测试 2748 passed / 51 ignored（140.81 秒），真实 EXE startup
7 passed（15.37 秒）；Clippy/fmt/3150 文件严格行数与来源、许可证、SBOM 门通过。
OSV advisory 仍有 25 个漏洞 ID，不能作发布放行。

修复 v216 已确认的负 dx 差异：按位移后跨度锚定，并把路径距离查询顺序与
原字符索引分离；使用局部 f64 避免公共起点参与跨度相减。真实 Job child
修复前 5 passed / 1 failed，修复后六项专项通过；浏览器对照、来源回放与
剩余边界见 SURFACE_SVG.md。N01–N12 总迁移仍未完成，没有提交、推送或发布。

## v216 路径文字接入

SVG textPath/startOffset 已接入引用准入与真实字体 child，并修复直线弧长定位。
沿用现有字体预算、Job、像素协议和授权安装，不引入 WebView 或新的资源 owner。
最终全量 2746 passed / 51 ignored（136.75 秒），startup 7 passed（15.47 秒）；
祖先负 dx 的浏览器/renderer 差异已实测但尚未修复，详见 SURFACE_SVG.md。

## v215 context paint 与窄画布修复

仓库固定 usvg/tiny-skia 补丁分别修复上下文 paint 丢失和窄画布 AA 描边跳过。
完整 native 2742 passed / 51 ignored（139.32 秒），startup 7 passed（16.14 秒）；
来源/实际 Cargo 解析绑定、许可证及双 SBOM 身份门已接入。OSV advisory 仍有
25 个漏洞 ID，不能作发布放行。证据见 SURFACE_SVG.md；N01–N12 仍未完成。

## v214 路径端点标记

最终全量 2739 passed / 51 ignored（143.71 秒），startup 7 passed（17.48 秒）；
Clippy/fmt/3148 文件严格行数/diff 通过，继承 marker 递归与 none 截断已补回归。

静态 SVG marker 接入原生产 worker，十组浏览器基线覆盖位置、朝向、尺寸及
裁剪；图门拒绝 marker/use 递归，Application/EXE 夹具已接入。最终验证和
剩余项见 SURFACE_SVG.md；不将本增量替代 N01–N12 总体验收。

## v213 pattern 兼容性续进

最终全量 2736 passed / 51 ignored（145.51 秒），startup 7 passed（17.55 秒）；
Clippy/fmt/3147 文件严格行数/diff 通过，保留首轮模板失败摘录。

新增十组浏览器基线，开放 pattern 四种 overflow 值，修复仅 title/desc
子内容错误继承模板图案的问题；normalize 保留属性继承与原安全入口。
完整回归与证据归 SURFACE_SVG.md。N01–N12 总任务继续进行，未提交推送或发布。

## v212 静态 pattern 增量

SVG 图案填充进入生产 Job child，八组浏览器对照已通过专项；图关系新增
fill/stroke 和 pattern/gradient 模板引用，前置拒绝图案递归。Application/EXE
夹具已接入。完整库测试 2734 passed / 51 ignored（142.09 秒），startup
7 passed（16.98 秒），Clippy/fmt/3147 文件严格行数/diff 通过；
范围与剩余项见 SURFACE_SVG.md。N01–N12 未完成。

## v211 symbol/use 开发中

本段最终回归：2731 passed / 51 ignored（132.13 秒），真实 EXE startup
7 passed（14.93 秒），Clippy/fmt/严格行数/diff 通过。symbol/use 本轮修复完成，
不代表 SVG 全语义或 Windows 原生大计划完成；最终证据见 SURFACE_SVG.md。

续进已使用 defs/svg 适配修复百分比与零 use 尺寸，保留全部失败夹具；
八组浏览器原文/改写逐字节一致，SVG 专项 27 passed / 2 private ignored。
双 href 优先级回归已补入，完整回归仍需记录。以下为修复前停点，细节见 SURFACE_SVG.md。

symbol 已初步接入，引用图门推广至含 mask/symbol/use 文档。八组浏览器基线
发现 usvg 对 use 引用 symbol 的百分比尺寸重复解析，实际像素不一致；必须修复
动态 viewport 语义，不能把失败夹具改为固定尺寸。当前不宣称 v211 全量通过，
最近完整验证为 v210，细节与后续证据归 SURFACE_SVG.md。未提交推送或发布。
最新专项 2 passed / 1 failed，除百分比外还发现零 use 宽度继续绘制；Clippy、
fmt、3146 文件严格行数及 diff 通过，不替代失败的像素验收。本轮不重跑全量/startup。

## v210 SVG mask 与引用图准入

mask 进入生产 Job child，八组浏览器基线保护 alpha/luminance、区域和空/零遮罩。
mask 定义自身属性差异通过独立 normalize 适配；引用图非递归拒环/重复 ID/超过
128 层展开。亮度量化仍有最多 1 个 u8 等级差异，未宣称 N11 逐字节等价关闭。
专项 23 passed / 2 私有 child，全量 2727 passed / 51 ignored（126.81 秒），
真实 Application 安装/绘制通过；Clippy 与 3145 文件严格行数通过。
startup 7 passed（15.53 秒），含实际产品 EXE 的 mask smoke；fmt/diff 通过。
没有新增依赖，未提交推送或发布；全迁移仍未完成，详见 SURFACE_SVG.md。

## v209 SVG 嵌套 viewport

生产 worker 开放嵌套 svg；11 组实际浏览器像素基线保护位置、百分比、
preserveAspectRatio、overflow 与两层嵌套。修复零尺寸嵌套在 usvg 中错误绘制的
实测差异，保留子树/ID，不改变根自然尺寸或父侧协议；详情归 SURFACE_SVG.md。
专项 18 passed / 2 私有 child，全量 2722 passed / 51 ignored（133.41 秒），
真实 inline Application 安装/绘制通过；Clippy、3142 文件严格行数通过。
startup 7 passed（16.50 秒），含实际 EXE 的嵌套/零尺寸 smoke；fmt/diff 通过。
本轮不新增依赖、不提交推送或发布，全迁移仍未完成。

## v208 删除撤销测试的键盘前提

真实 HWND 对照复现：无修饰键 Backspace 的 EM_CANUNDO 为 1→1，当前线程
Ctrl/Alt 状态下为 1→0，而文本均保持 1e+2。number_preflight 只对无修饰键
删除预检查，其他路径由 Session 恢复草稿；恢复文字不等价于恢复系统撤销。
原测试未固定这个前提，现用可恢复的线程键盘快照明确无修饰键，不注入桌面
输入，不更改生产逻辑，也不削弱撤销断言。首次 v207 失败缺少键盘快照，不能
据此确定其唯一根因。修改后 focused 4 passed（0.28 秒），完整原生测试
2721 passed / 51 ignored（141.68 秒）；Clippy all-targets -D warnings、fmt、
3142 文件严格行数及 diff 检查通过。首次 v207 失败证据不覆盖、不删除。
生产 SVG 文件及 Cargo 清单/锁文件 SHA256 与 v207 相同；本轮未重跑产品 startup，
不把 v207 的 7 项 startup 当成本轮新结果。修饰键路径恢复草稿但可能清空系统撤销
仍是生产行为边界，本轮仅纠正无修饰键测试前提，不宣称全部撤销行为已经一致。
证据：linshi/otter-v208-full.*、otter-v208-evidence.json。未提交推送或发布。

## v207 SVG 文字接线与未关闭验收项

有限 text/tspan 已接入原有受限图片 child，实际渲染中英文、多种 generic、
阿拉伯文、天城文和彩色 emoji；不新增 Application 字体 owner。加载 metrics
准入与存活布局缺字检查已接入，完整性边界见 SURFACE_SVG.md。
专项 17 passed / 2 私有 child；全量 2719 passed / 1 failed / 51 ignored，
失败为 surface_number_erase_rejection_retains_undo_for_delete_backspace_and_clear
的 EM_CANUNDO 返回 0。独立 focused 1 passed（0.09 秒），不能据此宣称根因已修复。
失败日志 otter-v207-full.* 保留。Clippy/fmt/3142 文件严格行数通过。
startup 7 passed（20.74 秒），含实际 EXE 的 SVG 文字 child smoke。
未提交、推送或发布；全迁移及本轮全量绿色门仍未完成。

## v206 SVG 系统字体 fallback

已实现有界 UTF-16 分析源及系统范围 fallback，保留映射长度、缩放、缺字结果，
复用原字体文件准入和查询/字节预算。不是 SVG 文字已开放；usvg 回调的范围/
缩放/模拟样式表达缺口及后续排版接线责任见 SURFACE_SVG.md。无依赖变更或发布。
字体 focused 5 passed / 1 私有 child（父测试调用），含八组真实系统映射。
首轮全量 UIA Select 失败一次；保留失败与 trace 诊断日志，独立复测 2717 passed /
51 ignored（134.24 秒），不宣称偶发问题根治。详情归 SURFACE_SVG.md。
Clippy/fmt/3141 文件严格行数通过，startup 7 passed（19.39 秒）；证据归
otter-v206-evidence.json。无提交推送或公开发布。

## v205 SVG 字体来源前置

原生 crate 已新增按名称选择、按需加载、保留 TTC/simulation 的字体模块，文件
准入以可信目录及最终打开句柄为准，并在分配前限制字节。必须在图片 Job 内创建。
尚未接通文字排版器或放开 text；generic/default/fallback 和缺字处理仍待实现。
具体范围及符号链接测试环境限制归 SURFACE_SVG.md；无依赖变更、提交推送发布。
全量 2715 passed / 51 ignored（133.03 秒），七个私有 child 由父测试调用；
原 44 项专项未执行。Clippy/fmt/3138 文件严格行数通过。
startup 7 passed（16.94 秒），证据归 otter-v205-evidence.json。

## v204 SVG 小数逻辑尺寸接通 child 与安装链

SVG v2 附加逻辑尺寸，双侧验证后经 Decoded/Ready/授权窗口安装；栅格 ceil 只用于
存储，不缩放原几何或作为布局比例。小数拒绝已解除，详细范围与剩余 AA/DPI 等
差异归 SURFACE_SVG.md。仍非 SVG/Windows 完整迁移完成，无依赖变更或发布。
全量 2712 passed / 50 ignored（138.33 秒）；Clippy/fmt/3134 文件严格行数通过。
六个私有 child 由父测试调用，原 44 项专项未执行。
startup 7 passed（18.28 秒），含产品 EXE 小数 SVG child，证据归 otter-v204-evidence.json。

## v203 Surface 逻辑尺寸与栅格尺寸分离

Bitmap/Presentation/NativeMeasure/contain 已分开逻辑尺寸与像素尺寸，保留整数
默认行为，D2D sourceRect 排除尾部 padding。SVG child/Ready 元数据接线及边缘
采样仍待完成，本轮不宣称小数 SVG 已开放。详见 SURFACE_SVG.md。
全量 2709 passed / 49 ignored（132.18 秒），三项小数呈现专项通过；
Clippy/fmt/3133 文件严格行数通过，无依赖变更、提交推送发布。
startup 7 passed（19.15 秒），证据归 otter-v203-evidence.json。

v202 后续小数 SVG 实测发现 naturalWidth、DOM 布局宽及绘制像素范围并不等同。
例如 width=1.4 的 naturalWidth=1，实际仍覆盖第二个像素；不能只 round 放开。
下一实现须贯通尺寸元数据/测量/contain 绘制，详见 SURFACE_SVG.md 小数尺寸节。
本次保留真实探针与结果，无生产改动或版本递增，完整迁移目标不变。

## v202 首批静态 SVG 接入

SVG renderer/policy/viewport 已接到现有 Job child，父侧统一接收门，Inline 沿既有
授权/安装链。真实尺寸/像素/拒绝测试和依赖门见 SURFACE_SVG.md。文字、CSS、动画、
foreignObject 等能力仍未完成；不缩减原迁移合同，不宣称 N01–N12 完成。
内部版本 v0.2.33.202，无提交、推送或公开发布。
全量 2706 passed / 49 ignored（119.07 秒），startup 7 passed（15.09 秒）；
Clippy/fmt/3133 文件严格行数及依赖门通过，五个私有 child 由父测试调用，
原 44 项专项未执行。范围/剩余能力见 SURFACE_SVG.md，证据归 otter-v202-evidence.json。

## v201 图片受限 child 共用宿主落地

现有 WebP 已改为调用共用 image worker host，独立 wire 层统一长度/像素帧校验，
process owner 提升到根模块并保留唯一容量池。保留原 CLI/magic/Job/截止/取消/
ICC/预乘/最终接收行为，未引入 SVG 依赖或开放 MIME。详细归 SURFACE_SVG.md。
内部版本 v0.2.33.201；本轮是 SVG 所需的生产职责提取，不是 SVG 或 N01–N12 完成。
最终全量 2698 passed / 48 ignored（120.85 秒），startup 7 passed（19.09 秒）；
WebP 重构前后 18 passed，共用层专项 6 passed，Clippy/fmt/3128 文件严格行数/diff
通过。原 44 项专项未执行，四个私有 child 由父测试调用；证据归 otter-v201-evidence.json。

## SVG 下一生产阶段的真实基线（2026-10-22）

本轮从 ICC 转向未接通的 SVG：12 个浏览器基础样本、5 个比例样本和动态采样，
加 resvg/usvg 0.48.1 临时原生实验，确认直接接库会丢文字/foreignObject/动画、
错误推导自然尺寸并可能读取本地 href。受控 sentinel 已实际证明默认本地读取。
结论、证据和共享 child/viewport/资源策略的下一接线归 SURFACE_SVG.md。
这些是改变实现方案的实测证据，不是 SVG 已完成；生产源码/依赖仍为 v0.2.33.200。

## v200 mft2 LUT 浏览器差异修复

真实 LUT-only WebP 发现 PCS 尺度与预合成 gamma 网格两类差异；限定 A2B0 mft2
RGB→XYZ 的直接分阶段求值已经接入受限 child，不修改第三方源码或其他格式。
六个 Chromium 对照样本覆盖非线性曲线、非可分离网格、RGB/RGBA、黑白/暗部/高光；
详细机制、独立 LittleCMS 证据和边界见 SURFACE_WEBP_ISOLATION.md。
内部版本 v0.2.33.200；不是全部 ICC/LUT 兼容或 N01–N12 完成，无提交推送发布。
最终全量 2695 passed / 48 ignored（119.96 秒）；ICC 专项 7 passed，Clippy/fmt/
3126 文件严格行数/diff 通过。原 44 项专项未执行，四个私有 child 由父测试实际调用。
startup 7 passed（19.12 秒）；本轮证据归 linshi/otter-v200-evidence.json。

## v199 WebP ICC 到 sRGB 的实际转换

Chromium Display-P3 夹具证明旧 worker 直接预乘编码 RGB 会产生色差；已在受限
child 内接通 ICC→sRGB，之后才预乘。moxcms 固定 0.8.1，通用转换分批 4096 像素，
不新增第二份整图缓冲，不将资源不足误当坏 ICC identity。P3 RGB/RGBA、四种
无效/缺失 metadata、alpha 边界、批次与实际 Application 路径形成验证。
完整边界见 SURFACE_WEBP_ISOLATION.md；真实 LUT profile、其他格式、监视器/HDR
与全尺寸峰值仍需推进，N01–N12 未完成。内部版本 v0.2.33.199，无提交推送发布。
新增五项回归。最终源码全量 2692 passed / 48 ignored（139.59 秒），startup 7 passed
（17.40 秒），Clippy all-targets -D warnings、fmt、3124 文件严格行数、diff 通过。
首次 Clippy 的测试 padding 写法已等价修正，最终二进制重新编译并全量重跑。
原 44 项 ignored 未执行，四个私有 child 入口由父测试调用；UIA 本轮通过不称根治。
全锁文件扫描/许可证/SBOM 及进程清理/源码哈希归 linshi/otter-v199-evidence.json。

## v198 WebP EXIF 方向兼容证据

先测浏览器再决定实现：32 个有效 EXIF 方向样本在 Chromium 155 均不旋转/镜像，
Pillow 12.3.0 独立读出全部正确 tag，排除坏夹具。原生新回归经真实 child 保持同一
尺寸和像素；没有添加与旧 img 行为相反的旋转，也没有修改 JPEG/TIFF 规则或 IPC。
源码佐证、样本范围和下一 ICC 边界见 SURFACE_WEBP_ISOLATION.md。此前“下一步
实现 WebP 旋转”仅为假设，现按实际证据纠正；不等同完整元数据/动画实现。
内部版本 v0.2.33.198，N01–N12 未完成，未提交推送发布。
验证：32 案例 child 专项通过（2.05 秒）；最终全量 2687 passed / 48 ignored
（112.31 秒），startup 7 passed（14.62 秒）。Clippy/fmt/3122 文件严格行数/diff
通过，原生测试进程零残留，专用浏览器已关闭。原 44 项专项未执行，另外四个
ignored 为父测试实际调用的私有 child；UIA 本轮通过不等同历史问题根治。
证据归 `linshi/otter-v198-evidence.json`。

## v197 WebP 父侧协议故障验证

新增真实 Job child 的坏响应夹具，复用生产父侧接收、像素预算及退出门，不另造
测试解析器。一个合法对照核对精确像素；错误 magic/宽度/长度、短头/短像素、尾随
输出、合法回复后失败退出、无回复失败退出全部必须拒绝 Image，并逐项检查进程
句柄已 signaled。生产启动入口仍固定，测试参数不进入正式构建或用户配置。
共享容量增加 deadline 和等待中取消，取消必须明显早于 deadline 返回。该增量
没有扩大图片来源/格式，没有声称 Job 分配失败、codec 全部崩溃路径或整组峰值已验。
内部版本 v0.2.33.197；N01–N12 未完成，无提交、推送或发布。
最终源码全量 2686 passed / 48 ignored（115.28 秒），startup 7 passed（16.67 秒）；
Clippy all-targets -D warnings、fmt、3121 文件严格行数、diff 通过。首次 Clippy 指出
夹具手写 unwrap_or，已按建议等价简化并重编译、重跑完整全量，不用旧二进制代验。
48 ignored 中四个私有 child 入口由父测试实际调用，原 44 项专项未执行。UIA Select
本轮通过但不宣称历史间歇性根治。证据归 `linshi/otter-v197-evidence.json`。

## v196 UIA Select 失败诊断续验

保留首轮 2684/1/47 失败日志，没有回滚、放宽断言或修改生产焦点策略。客户端仅
增加 receive_traced 阶段标签，错误返回时才输出阶段；成功路径不额外查询 COM、
激活窗口、泵送或重试。先前含额外只读焦点查询的临时诊断一轮通过，不作为修复证据，
该临时查询已移除。最终阶段记录版本独立三轮通过，全量 2685 passed / 47 ignored
（120.10 秒），Clippy/fmt/3120 文件严格行数/diff 通过；本轮仅测试诊断变化，
产品 EXE 的 startup 7 passed 仍使用前一轮同生产源码证据，未假称重新执行。

已核实当前 windows 绑定：FindFirst 等接口返回值经 Type::from_abi 转换，COM 的
S_OK + null interface 会生成 Error::empty，即 Err(HRESULT(0))；SetFocus 的绑定
仅 HRESULT.ok()，S_OK 自身不会生成该错误。List FindFirst 是候选失败点，但没有
在带阶段标签的版本上重现，不能定论。Select 使用系统 HWND/proxy 焦点，未设置
ProviderOwnsSetFocus；此路由事实不构成修改依据。旧二进制焦点断言失败与该空接口
错误也不能未经证明视作同一原因。当前确认间歇性，未宣称根治或完整 UIA 验收。
续验证据归 `linshi/otter-v196-uia-diagnostic-evidence.json`，首轮 evidence 不覆盖。
下一步继续 WebP 坏 IPC/崩溃等故障门；若 UIA 再现，按阶段诊断定位而非盲目重跑。

## v196 静态 WebP 生产接线

已核对 image-webp 0.2.4 源码，并用 2×1 内存夹具实证 memory_limit=0 仍可静态解码，
以及 VP8X/VP8L 内外尺寸不一致在 new 后仍需另行拒绝。静态工作分配不能由该 API
保证；现已以受限子进程和 RIFF 前置检查接入 Resource/Inline → Ready → HWND。
单 child Job 128 MiB、最多两个并行，容量等待/管道/退出均遵守撤销和截止时间；
没有主进程回退，不把 Job 当完整权限沙箱。真实 Chromium 有损/alpha 像素、无损
RGB/RGBA、Application 内联/TLS 安装及 Job 拒绝超限分配/停滞回收均有验证。
详情、源码位置和剩余故障注入/峰值/元数据验收门见 SURFACE_WEBP_ISOLATION.md。
内部版本 v0.2.33.196；WebP 元数据/动画、完整图片及 N01–N12 继续进行中。
未提交、推送或发布；全锁文件 Advisory 扫描仍有 20 个漏洞 ID，不称零漏洞发布门通过。
最终全量为 2684 passed / 1 failed / 47 ignored（118.14 秒），不是全绿：
失败为 uia_select_system_relocation_preserves_selection_expand_and_change_queue，
UIA 客户端返回 HRESULT(0x00000000) 的 Err；独立复测同样失败。保留的旧测试 EXE
执行同一项也在 CurrentHasKeyboardFocus 失败，但不能据此宣称根因已定位或修复。
WebP 专项 9 passed / 3 private ignored，另两项 Application WebP 测试在全量中通过；
新增三个 ignored 是由父测试按固定入口启动的 child 夹具，原 44 项专项未执行。
startup 7 passed（14.97 秒），包含实际产品 EXE WebP child smoke；Clippy/fmt、
3120 文件严格行数、diff 通过。CycloneDX/SPDX 的 1228 个去重锁身份均验证一致。
日志和源码哈希归 `linshi/otter-v196-evidence.json`；下一步需定位该 UIA 失败及继续
WebP 故障注入/元数据边界，而非把当前源码证据当可发布完成态。

## v195 普通 Surface 静态格式兼容

已补普通桌面图片的受限实际位图识别，不套用 Tile/Wall 严格 MIME 匹配合同。声明
大小写、参数或错误格式不再误拒已有静态容器；实际 APNG、帧数、EXIF、像素预算
仍按真实容器判断，HTML/SVG/真实 WebP 不因此获得支持。内联来源和原始字节身份门
不放宽，data URL 在规范化后分流 SVG。真实浏览器 32 案例和应用 TLS/内联路径均
有对应验证，旧资源缓存参数限制及标准/Chromium 差异见 SURFACE_IMAGES.md。
内部版本 v0.2.33.195；真实 WebP/SVG/动画/blob/在线业务及 N01–N12 总迁移未完成。
无提交、推送或发布。
新增 9 项回归；最终全量 2674 passed / 44 ignored（109.94 秒），startup 7 passed
（15.24 秒）；Clippy/fmt/3112 文件严格行数/diff 通过，24 个文本 UTF-8 无 BOM，
原生测试进程残留零。源码/日志哈希见 `linshi/otter-v195-evidence.json`；不把本轮
绿灯说成历史 UIA/点击不稳定性的根因修复。

## v194 Surface EXIF 方向

静态 JPEG/TIFF Orientation 1–8 已在后台 WIC 源链处理，自然尺寸随方向交换；原始
尺寸先准入，最终像素仍受共享预算。普通打开/Relay 不旋转，元数据缺失/非法回退原图。
新增八方向、异常元数据、取消、原尺寸超限、Relay 和真实 Application 安装回归；
独立 Chromium JPEG 八方向基线一致。详情与边界见 SURFACE_IMAGES.md。
内部版本 v0.2.33.194；XMP/PNG eXIf/ICC、完整格式/blob/在线业务仍未完成，
N01–N12 总任务保持进行中，无提交推送发布。
新增 7 项回归；最终全量 2665 passed / 44 ignored（96.91 秒），startup 7 passed
（13.72 秒），Clippy/fmt/3110 文件严格行数/diff 通过，22 个文本 UTF-8 无 BOM，
原生测试进程残留零。首次全量的预算报错文本兼容性失败已修复并保留日志；最终结果
和哈希见 `linshi/otter-v194-evidence.json`，历史 UIA/点击不稳定性不宣称因此根治。

## v193 内联图片不可用时的原生反馈

修正 data:image/ 无像素时被完全隐藏的问题：保留 Image/alt，使用本地标记和替代
文字独立测量，等待或失败期间可见，成功图片安装自动移除；resourceId unresolved
仍不回退。不是浏览器错误图标逐像素复刻，也不新增远端事件或伪造终态失败。
旧行为已由两项新增模型回归实际复现；真实 D2D 像素、系统 UIA、Application 接线
及跨节点文字选择由对应 owner 验证。详见 SURFACE_IMAGES.md。
同时修复图片 alt 绘制项退场误清空跨节点选区；端点及可选择文字顺序/内容门保留，
不扩大跨节点复制能力。专门失败回归修复后通过，cross_node 筛选 16 项通过。
内部版本 v0.2.33.193；完整格式/blob/在线业务仍待推进，N01–N12 未完成，无提交推送发布。
新增 6 项回归，扩展 3 项既有应用测试；全量 2658 passed / 44 ignored（96.50 秒），
startup 7 passed（13.88 秒），Clippy/fmt/3108 文件严格行数/diff 通过，24 个文本
UTF-8 无 BOM。日志与源码/截图哈希见 `linshi/otter-v193-evidence.json`。

## v192 内联图片应用路径

data:image/ 静态源现沿完整应用 poll、既有后台任务和 Ready 自动安装上传，不准备
HTTP Config/Client，也不创建假的资源租约。Resource/Node 类型键避免碰撞，非空
resourceId 优先且失败不回退。完整快照摘要与 Binding/revision 保护来源，原 Lease、
同 Grant、取消、共享容量和窗口授权门保留；每轮安装刷新只散列一次快照。
新增 8 项数据 URL、身份/Grant/取消/容量和真实应用接线回归；详见 SURFACE_IMAGES.md。
内部版本 v0.2.33.192，无提交推送发布。SVG/WebP/动画/blob/错误占位仍待迁移，
宿主通用 image 能力不扩大，N01–N12 总任务没有完成。
最终全量 2652 passed / 44 ignored（101.37 秒），startup 7 passed（14.43 秒）；
Clippy/fmt/3105 文件严格行数/diff 通过，23 个文本 UTF-8 无 BOM，原生测试进程残留零。
日志与哈希见 `linshi/otter-v192-full.stdout.log`、`linshi/otter-v192-evidence.json`。

## v191 图片无障碍和叶节点合同

图片名称改为仅使用字符串 props.alt，不套用旧 img 分支未接的 generic label/disabled。
装饰图片保留 UIA raw provider，但从 control/content view 排除；图片不变成可调用按钮。
Image 统一呈现为叶节点，测量不再因原始 children 非空拒绝；原始协议预算/指纹不变。
改动前已用两项模型回归复现名称及子树错误；真实 HWND 系统 UIA 回归覆盖 alt、视图
过滤与替换撤销。详见 SURFACE_IMAGES.md；内部版本 v0.2.33.191，无提交推送发布。
完整图片来源、格式、错误图像和在线业务验收仍待推进，N01–N12 总任务未完成。
新增 5 项回归；最终全量 2644 passed / 44 ignored（99.04 秒），startup 7 passed
（16.09 秒）。Clippy、fmt、3099 文件严格行数和 diff 检查通过；日志及哈希分别见
`linshi/otter-v191-full.stdout.log`、`linshi/otter-v191-evidence.json`。

## v190 图片自动请求与生产 poll 投递

当前 active 场景的 resourceId 已沿完整应用 poll 自动 GET、解码、Ready 安装。
原始来源、完整 Binding/revision、同 Grant 和 HWND 后授权门保留；每窗口有界队列
对同身份只尝试一次，Busy 留队，换身份重建，窗口顺序轮转。没有新增前台网络或
退出启动路径；详见 SURFACE_IMAGES.md。内部版本 v0.2.33.190，无提交推送发布。
新增 5 项回归：两项完整 poll 真实 TLS 成功/失败链、一项关闭后资源结算子路径，
两项队列/场景版本/轮转测试。测试挂载/激活输入是合成的，生产消息等待整链与在线
Loom 尚未完整运行验收，通用 image 能力报告不扩大；整个 N01–N12 仍未完成。
最终源码串行全量 2639 passed / 44 ignored（102.08 秒），startup 7 passed
（17.42 秒）；日志 `linshi/otter-v190-final-full.stdout.log`，证据与源码哈希见
`linshi/otter-v190-evidence.json`。调度已移到目录/远程观看回收之后，真实目录占据
最后一槽的回归保护同轮释放后启动；当前身份空队列不再复制/解析原始快照。
Clippy、fmt、3098 文件严格行数和 diff 检查通过；Application 根 500 行未增长。

## v189 授权 Ready 安装与图片到期撤除

AuthorizedWindow 已提供 install_resource：验证原请求及当前完整身份后，仅允许同
Grant 的 Lease 接管图片授权；消费释放请求槽。安装表最多 64 个资源 ID，使用当前
快照投影，不从 src/路径猜测来源。HWND 提交后复核撤权，失败关闭 owner。
图片期限进入既有 deadline，到期或换绑撤去失效图片；换场景、关闭、撤权清表。
同场景刷新保留未变文字选区，控件拖动连续性尚未完整验收。自动请求/投递调度仍待
接通；详见 SURFACE_IMAGES.md。内部版本 v0.2.33.189，无提交推送发布；N01–N12 未完成。
新增 4 项回归，资源过滤 35 passed、选区刷新 1 passed；独立串行全量 2634 passed /
44 ignored（93.78 秒）。Clippy、fmt、3092 文件严格行数检查通过，安装模块 134、
授权窗口 449 有效行；完整日志 `linshi/otter-v189-full.stdout.log`。
随后 startup 7 passed（21.06 秒）；本轮源码/日志哈希与编码、进程复核结果保存在
`linshi/otter-v189-evidence.json`。未提交、推送或发布，下一步仍为应用自动调度。

## v188 图片 pending 阶段与应用挂载

应用初始挂载、事件和 batch 不再因未解码图片拒绝整个场景；通过本地 Scene 图片
可用性投影略去未就绪图片，保留其他节点与完整原始资源描述。布局/命中/UIA 统一
遵守 visible 门，协议输入不能伪造投影状态，显式 hidden 不被重新启用。
原绑定/版本/授权/成功提交顺序不变；普通低层测量仍拒绝缺像素的可见 Image。
已解除建窗与资源 owner 的初始化环路，自动请求/Ready 安装/到期撤除仍未接通。
内部版本 v0.2.33.188，无提交推送发布；N01–N12 总任务未完成。详见 SURFACE_IMAGES.md。
新增 4 项回归，专项过滤 unresolved 5 / resource_window 3 / batch 1 均通过（有重叠）。
独立串行全量 2630 passed / 44 ignored（91.13 秒），Clippy 与 3091 文件严格行数
检查通过；日志 `linshi/otter-v188-full.stdout.log`。历史 UIA/点击不稳定性仍未宣称修复。
随后 startup 7 passed（16.62 秒），fmt/diff 与 14 个文件 UTF-8 无 BOM 检查通过；
源码/日志哈希见 `linshi/otter-v188-evidence.json`，收尾自有 native 测试进程为零。

## v187 显式图片表进入真实 HWND

SurfaceWindow 与 Host 的创建/替换已接受当前场景 Images，沿既有呈现链真实上传。
普通 API 仍传空表，不按相同 ID 继承旧图片。窗口缩放/设备 target 生命周期、UIA
身份撤销与重入检查复用既有规则。替换逻辑提取至 surface_window_scene，根文件不超限。
真实 HWND 回归覆盖上传、resize 释放/重建、替换/关闭回收、缺图/忙/关闭拒绝。
详见 SURFACE_IMAGES.md；自动资源调度、授权 Ready 安装与到期图片撤除仍待接通。
内部版本 v0.2.33.187；无提交、推送或发布，整个 N01–N12 迁移未完成。
窗口专项 73 passed（4.98 秒），独立串行全量 2626 passed / 44 ignored（91.26 秒）；
Clippy、fmt、3091 文件严格行数检查通过，窗口根 486 有效行。原 UIA/点击不稳定性
未宣称修复；完整日志 `linshi/otter-v187-full.stdout.log`。
随后 startup 7 passed（17.64 秒），12 个本轮文件 UTF-8 无 BOM，diff 检查通过；
源码/日志哈希及验证边界保存在 `linshi/otter-v187-evidence.json`。

## v186 受管资源并发发布共享冲突

v185 失败调查发现：commit_new 冲突后重开错误被丢弃，旧 B7 掩盖了后续错误。
真实 DELETE 句柄持有的受控窗口用例在旧实现失败（0.21 秒）；修复后资源目录专项
11 passed（0.85 秒）。Root 保留 Win32 HRESULT，publication 仅对共享冲突执行
500ms 重试预算，每轮检查取消，最终错误同时保留 commit/reopen 阶段。
仍只共享读取，仍逐字节核验，不覆盖或删除目标；权限、目录、重解析点检查不变。
500ms 不是底层文件系统调用或线程调度的硬超时。200ms 锁释放用例不是所有调度下
必红的严格同步竞态测试；取消用例可能在解码/暂存阶段触发，不冒充等待分支注入。
新增永久锁错误身份/有界返回、取消、pin 防写/防删及临时文件清理检查。

机制依据：[Microsoft CreateFile 共享合同](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilea)
与 [重命名的多阶段过程](https://devblogs.microsoft.com/oldnewthing/20211022-00/?p=105822)。
原失败没有记录 reopen 错误，因此不能反推该历史现场必然就是共享冲突；本次修复
的是已复现的具体缺陷。Select UIA 与编辑点击顺序失败尚未根因闭环，不宣称修好。
图片自动调度与 HWND 安装仍待接通；内部版本 v0.2.33.186，无提交、推送或发布。
本轮独立串行全量 2623 passed / 44 ignored（90.48 秒），日志
`linshi/otter-v186-full.stdout.log`；Clippy、fmt 与 3089 文件严格行数检查通过。
本次全绿不是历史 UIA/点击不稳定性根因已修复的证据；保留 v185 三轮原始失败。
随后 startup 7 passed（15.06 秒）；6 个本轮文件 UTF-8 无 BOM，diff 检查通过。
源码/日志哈希与验证边界见 `linshi/otter-v186-evidence.json`。

## v185 Surface 当前资源快照

补齐 attach/事件收敛 → AuthorizedWindow 的资源描述丢失：同一 raw 生成场景和
有界 ResourceSnapshot，事件/batch 只有场景 replace 成功才提交当前资源快照。
资源读取取当前生命周期 Binding，不沿用 mounted 值；同设备/节点/附件/代际与
场景版本必须匹配。关闭/撤权清理，Debug 不打印 raw，RefCell 不跨 HWND 调用。
窗口已提供当前资源请求与 Ready 像素复核入口，但自动调度和 HWND 图片安装尚未
接通；来源授权仍独立于快照解析。详见 SURFACE_IMAGES.md。
内部版本 v0.2.33.185；未提交、推送或发布，N01–N12 整体迁移仍未完成。
新增 5 项回归，资源专项 31 passed；三轮全量均 2620 passed/1 failed/44 ignored，
分别失败于并发资源发布、编辑点击顺序、Select UIA 枚举。第二轮与 startup 重叠，
第三轮独立运行仍失败；不声称全绿或根因已修复。具体原文/日志与边界见 SURFACE_IMAGES.md。
startup 7 passed（17.50 秒），Clippy、fmt、3089 文件严格行数检查通过。

## v184 Surface 图片自然尺寸与生产呈现入口

已解码图片可通过显式 with_images 进入 NativeMeasure/Presentation/PaintFrame；
新增内容盒比例、交叉轴反馈、双轴收敛、默认 max 与 auto 主轴 minimum。
所有滚动/显露投影保留当前图片表；场景替换不按同 ID 继承旧图，候选失败保旧帧。
真实 Chromium 旧组件 DOM 样式补测 13 项，含 row/双图/auto 父高/padding/absolute
百分比；四项 WIC/D2D 像素用例已升级为生产测量链。详见 SURFACE_IMAGES.md。
应用自动请求与授权结果向 HWND 安装仍未接通，不扩大 image 能力声明。
内部版本 v0.2.33.184；未提交、推送或发布，N01–N12 整体迁移仍未完成。
新增 7 项测试，图片专项 11 passed；可见串行全量 2616 passed/44 ignored
（87.15 秒），`linshi/otter-v184-full.stdout.log`。Clippy、fmt、3085 文件严格行数、
diff 与 22 文件 UTF-8 无 BOM 检查通过。
startup 7 passed（16.49 秒）；源码/浏览器基线/日志哈希见 `linshi/otter-v184-evidence.json`。

## v183 Surface 图片绘制后端

新增 UI 线程共享像素 Bitmap，按精确 target 缓存上传，并纳入 PaintFrame 原有绘制顺序、
contain、内容圆角、祖先透明层与裁剪。宿主 target 重建/错误和帧退场清缓存；上传预算
128 MiB 不代表驱动总显存。自然尺寸布局、自动资源请求和 Ready 安装尚未接通，
NativeMeasure 仍拒绝 Image，不扩大能力声明。详细边界见 SURFACE_IMAGES.md。
内部版本 v0.2.33.183；未提交、推送或发布，N01–N12 整体迁移仍未完成。
绘制专项 4 passed；可见串行全量 2609 passed/44 ignored（86.98 秒），日志
`linshi/otter-v183-full.stdout.log`。Clippy、fmt、3083 文件严格行数、diff 与 12 文件
UTF-8 无 BOM 检查通过；真实图片节点端到端仍待后续链路接通。
startup 7 passed（16.28 秒）；源码/日志哈希见 `linshi/otter-v183-evidence.json`。

## v182 Surface 后台静态解码与到期回收

资源 worker 已复用共享 WIC/Pixels，Ready 现在持有解码像素与原请求身份，编码缓冲
解码结束后清零释放。原始 RGBA 转预乘 BGRA；局部 64 MiB 像素门和全局预算在分配前
验证。COM 留在 worker；取消/授权/到期在解码前后与消费前复核。资源最早到期时间
接现有 wait，并主动取消/丢弃，不添加轮询线程。静态 MIME/多帧拒绝边界见 SURFACE_IMAGES。
尚未接图片节点自动启动、布局与 PaintFrame；动画/其他 MIME 必须继续补齐，不删减目标。
内部版本 v0.2.33.182；未提交、推送或发布，N01–N12 整体迁移仍未完成。
新增 4 项测试；解码专项 3 passed，资源专项 11 passed。最终可见串行全量
2605 passed/44 ignored（88.83 秒），`linshi/otter-v182-full.stdout.log`。
Clippy、fmt、3081 文件严格行数、diff 与 20 文件 UTF-8 无 BOM 检查通过。
startup 7 passed（18.57 秒），收尾自有 native 进程为零；证据与哈希见
`linshi/otter-v182-evidence.json`。

## v181 Surface 图片后台资源生命周期

图片获取 Job 已接入应用 Sessions 的共享容量、正常/退出 wait handle、poll、
取消与 drain；Ready 持有原授权 Lease 和槽，未消费结果不释放容量。应用结算点
按活窗口 Binding 和场景 revision 取消旧任务；退出先取消资源 GET 让出停用槽。
完成后撤权仍阻止消费，不允许改传新 Cancel 绕过原任务终态。
尚未自动为图片节点发起请求，也未接 WIC/布局/PaintFrame，不扩大 image 能力。
内部版本 v0.2.33.181，未提交、推送或发布；N01–N12 整体迁移仍未完成。
新增 7 项生命周期/集合测试；资源筛选 44 passed。最终可见串行全量
2601 passed/44 ignored（85.51 秒），`linshi/otter-v181-full.stdout.log`。
Clippy、fmt、3077 文件严格行数、diff 检查及 20 文件 UTF-8 无 BOM 检查通过。
startup 7 passed（19.70 秒），收尾自有 native 进程为零；证据与哈希见
`linshi/otter-v181-evidence.json`。

## v180 Surface 图片安全资源获取

已实现绑定 mount/revision/lease 的 loom_resource 字节获取 API，复用固定来源 TLS、
Device/nonce、取消和 401 语义；16 MiB 预算只对受限资源请求开放，普通请求不变。
精确长度和 SHA-256 验证后才产生结果；消费时复核当前 mount、期限与 owner Cancel。
尚未接入生产后台 job/队列、WIC 解码、布局或 PaintFrame，image 能力仍不声明。
shared_memory、data/blob 和 Tile 路径仍须继续实现。详见 SURFACE_IMAGES.md。
内部版本 v0.2.33.180；未提交、推送或发布，N01–N12 整体迁移仍未完成。
资源专项 5 passed；最终可见串行全量 2594 passed/44 ignored（84.54 秒），
`linshi/otter-v180-full.stdout.log`。Clippy、fmt、3073 文件严格行数及 diff 检查通过。
startup 7 passed（16.57 秒）；11 个相关文件 UTF-8 无 BOM。源码/日志哈希见
`linshi/otter-v180-evidence.json`。

## 2026-10-21 Surface 图片功能链基线

从文字选择转向仍明确拒绝的 image 叶节点。已核对旧来源优先级、Loom lease/摘要/
大小鉴权与 Tile 独立边界；真实旧组件 14 场景通过，证明 column 默认 stretch、
固定 contain、resourceId 不回退 src、禁直接远程/文件 URL 和仅祖先 click。
现有前端相关测试 12 passed，原生拒绝 image 的现有测试 1 passed。没有生产功能
改动，不提升内部版本（仍 v0.2.33.179），不扩大能力声明。详见 SURFACE_IMAGES.md。
下一实施链是绑定资源获取→后台有界解码→自然尺寸测量→有序绘制，不允许在
图片测量层直接读取路径/下载。未提交、推送或发布，整体迁移仍未完成。

## v179 高亮范围投影去除平方级查找

正式绘制每帧只解析一次 Selection 端点，逐 Text 命令复用已有索引投影范围；
不添加常驻缓存或逐节点分配，实际高亮/遮挡/画刷恢复路径保持不变。旧公式仅
留作测试 oracle，新增端点穷举和显式微基准。详见 SURFACE_TEXT_SELECTION.md。
仅范围投影的性能工作，不替代 N11 整机/长时间运行或 N01–N12 全部验收。
内部版本 v0.2.33.179；未提交、推送或发布。
同二进制 debug 微基准：1024 节点二十轮中位范围计算旧 284.5251 ms、新 0.4436 ms，
覆盖长度相同；不包含 D2D/GPU、帧耗时或输入延迟。源码/EXE/日志哈希在
`linshi/otter-v179-range-evidence.json`。最终可见串行全量 2589 passed/44 ignored
（96.32 秒），包含真实像素与旧公式穷举；日志 `linshi/otter-v179-final-full.stdout.log`。
新增 ignored 是已显式运行的微基准。Clippy、fmt、3071 文件严格行数及 diff 检查通过。
startup 7 passed（19.77 秒）；12 个相关文件 UTF-8 无 BOM，验证后自有 native 进程为零。

## v178 跨节点上下扩选

接入 focus 实际排版行与相邻 selectable owner；跨节点保持全局列意图，区分软
换行同索引移动与真正节点边界。旧组件三布局多行对照 15 断言通过；原生增加
三项多行/软换行/空布局与末尾换行测试，HWND 路由补上下跨节点。详细证据与
初期夹具失败见 SURFACE_TEXT_SELECTION.md。只读审查未发现确认新缺陷。
仍限定当前 Surface，不能替代页面跨 Surface 选择、复杂 bidi/滚动、系统输入、
全局 Ctrl+A 和整体 N01–N12 验收。内部版本 v0.2.33.178，未提交、推送或发布。
最终可见串行全量 2588 passed/43 ignored/0 failed（96.43 秒），日志
`linshi/otter-v178-final-full.stdout.log`；startup 7 passed（20.05 秒）。Clippy
all-targets -D warnings、fmt、3070 文件严格行数和 diff 检查通过；12 个相关文件
UTF-8 无 BOM。验证后自有 native 进程与探针端口监听均为零。

## v177 跨节点水平键盘扩选

真实旧组件三布局键盘/边界基线 54 项断言通过；原生接入 Shift+左右跨 owner 与
Shift+Home/End 跟随 focus 行端点。导航锚点不变，跨节点复制门禁不变，普通左右
不折叠跨节点范围。只读审查发现上下 keydown/keyup 门禁不一致，已统一 readiness
并增加 HWND 断言。四项新增测试覆盖跨端点、返回、反序、不可选项与 Surface 边界。
跨节点上下导航、全局 Ctrl+A、复杂 bidi/滚动和系统输入仍待完成；不将该增量
当作完整键盘或 N01–N12 验收。内部版本 v0.2.33.177，未提交、推送或发布。
最终可见串行全量 2585 passed/43 ignored/0 failed（88.24 秒），日志
`linshi/otter-v177-final-full.stdout.log`；Clippy all-targets -D warnings、fmt、
3070 文件严格行数和 diff 检查通过，11 个相关文件 UTF-8 无 BOM。
startup 7 passed（13.20 秒）；结束时自有 native 进程与探针端口 15471 监听均为零。

## v176 词拖动真实基线与尾随空白修正

真实旧组件在 Edge 上完成三布局、六次双击后跨节点/返回手势，30 项断言通过，
发现 v175 原生保留尾随空白与旧行为不符。已分离初始双击范围和拖动词段：
不移动抬起保留空白，拖动后按词干端点扩展，返回原词不再包含尾随空白。
新增内部词多次跨节点往返契约测试；详见 SURFACE_TEXT_SELECTION.md。
这不是完整框架验收，跨节点键盘、复杂 bidi/滚动、系统双击和性能仍待完成。
内部版本 v0.2.33.176；未提交、推送或发布。
最终可见串行全量 2581 passed/43 ignored/0 failed（77.22 秒），日志
`linshi/otter-v176-final-full.stdout.log`；startup 7 passed（12.14 秒）。Clippy
all-targets -D warnings、fmt、3069 文件严格行数、diff 检查通过；10 个相关文件
UTF-8 无 BOM。只读审查未发现确认新缺陷；另补七项微小移动阶段浏览器探针，
合计 37 断言通过，验证后浏览器/服务器已关闭，15471 监听和自有 native 进程均为零。

## v175 跨节点词级拖动

词手势切换 focus owner 时查询该节点自己的有界词边界，锚点始终来自双击初始词，
按场景顺序选择正反方向；返回原节点恢复词扩展。只保存当前 owner 的一份表，
不累计经过节点；同节点 move 不再重复分配节点字符串。跨节点复制门禁不变。
新增三项测试覆盖视觉反序、中文/emoji 索引和 HWND 捕获/click 隔离。
跨节点键盘、浏览器词范围精确对照、复杂滚动/bidi、性能与系统输入仍待验证。
本增量不代表 N01–N12 完成；内部版本 v0.2.33.175，未提交、推送或发布。

最终可见串行全量 2580 passed/43 ignored/0 failed（77.47 秒），证据位于
`linshi/otter-v175-final-full.stdout.log`；startup 7 passed（13.20 秒）。Clippy
all-targets -D warnings、fmt、3069 文件严格行数、diff 检查通过；10 个相关文件
UTF-8 无 BOM，验证后自有 native 进程为零。只读审查未发现确认新缺陷。

## v174 跨节点普通拖选与真实范围高亮

Selection 区分 anchor/focus 节点，普通鼠标拖选跨节点后按场景先序投影端点及中间
文字范围；反向和返回原节点都保留语义。正式 PaintFrame 逐项绘制这些范围，跨节点
selected_text 拒绝聚合复制，抬起不因局部索引相同而误发祖先 click。reveal 和
autoscroll 转用 focus owner；重排时核对范围内成员、顺序、权限与规范化文字。
词级跨节点及键盘扩选/折叠仍未完成，复杂离屏/bidi/空白命中与性能仍待专项验证。
详见 SURFACE_TEXT_SELECTION.md；不将这一步当完整跨节点或 N01–N12 完成。

新增 5 项随最终可见串行全量 2577 passed/43 ignored（81.14 秒）通过，包含真实
WIC/D2D 三节点高亮/清除像素、反序/返回、范围变化失效和 HWND click/copy 隔离。
日志 `linshi/otter-v174-final-full.stdout.log`；初期夹具末字符换行出可见区域导致
失败，修正夹具字号使目标实际可命中后通过，未放宽断言。Clippy 单 Range 数组
警告通过删除冗余区域约束修正，未加 lint 豁免。最终 Clippy all-targets -D warnings、
fmt、3069 文件严格行数和 diff 检查通过。v173 系统双击前台保护失败继续保留。
独立只读审查未发现确认正确性缺陷；多节点绘制的线性查询成本仍缺性能测量。
内部版本 v0.2.33.174，未提交、推送或发布。
startup 7 passed（13.74 秒）；16 个相关文件 UTF-8 无 BOM，结束时自有 native 进程为零。

## 2026-10-21 跨节点六布局真实拖选基线

实际旧组件/样式/快捷键模块完成 12 项正反向鼠标拖选断言：column、row、stack
分离/重叠、绝对定位反序、含不可选文字和 input 的嵌套布局。视觉位置反序不改变
节点范围顺序；覆盖文字无法穿透命中；跨节点 Ctrl+C 仍是 Unit copy。结果和源码
哈希见 SURFACE_TEXT_SELECTION.md。没有修改生产代码/内部版本，不把此基线当成
跨节点原生实现已完成；v173 系统双击前台保护失败继续单独保留。

## v173 系统双击探针：前台准入失败，保留现场

新增显式 text-double-click 系统探针和 launcher 白名单入口，复用自有可见窗口、
真实命中、输入空闲、完整批次与有界消息观察。首次运行 exit=101，前台身份检查
失败且尚未注入输入；不改保护、不抢焦、不重试直到通过。失败证据与后续跨节点
依赖核对见 SURFACE_TEXT_SELECTION.md。补充激活返回值和前台时序诊断，但尚未
完成新的交互运行；系统双击验收仍未通过，不能用默认全量替代该 ignored 探针。
生产逻辑不变；当前可见串行全量 2572 passed/43 ignored（80.96 秒），日志
`linshi/otter-v173-full.stdout.log`；Clippy all-targets -D warnings、fmt、3067 文件
严格行数与 diff 检查通过。整体 N01–N12 未完成，未提交、推送或发布。
startup 7 passed（12.35 秒）；9 个相关文件 UTF-8 无 BOM，launcher PowerShell
语法检查通过。交互探针失败与默认测试结果分别记录，不用后者抹去前者。

## v172 原生双击词选择与词级拖动

实际接入 CS_DBLCLKS/WM_LBUTTONDBLCLK，DirectWrite 命中后按系统 ICU 边界选择
单节点词，捕获拖动支持按词前向/反向扩展。沿用唯一选区、自动滚动和取消/撤权/
换场景/关闭清理；失败不回退为祖先点击。非文字和滚动条同时接第二次按下，避免
窗口类样式改变吞掉原输入。系统目录按需加载，手势边界表有界且完成即释放。
完整责任与未完成范围见 SURFACE_TEXT_SELECTION.md：跨节点、emoji 与浏览器旧
范围差异、真实系统双击生成及缺 DLL 实机路径尚未验收，不将单节点功能替代大任务。
新增 8 项纳入可见串行全量，2572 passed/42 ignored（88.06 秒），日志为
`linshi/otter-v172-full.stdout.log`。v169 旧 UIA 间歇性异常仍未宣称根因修复。
startup 7 passed（20.76 秒）；Clippy all-targets -D warnings、fmt、3066 文件严格
行数、22 个相关文本 UTF-8 无 BOM 和 diff 检查通过，结束时自有 native 进程为零。
独立只读审查未发现确认缺陷；缺 DLL/导出/ICU 错误的可控故障注入仍缺运行证据。
N01–N12 整体未完成；内部版本 v0.2.33.172，未提交、推送或发布。

## 2026-10-21 系统 ICU 候选词边界验证

本机系统 ICU 72.1.0.4 实际 C API 的 60 组文本/locale 边界断言通过，中文/泰文
词干与浏览器样例相符；另验证空文本、65536 单元上限与超限拒绝。尾随空格、emoji
范围和跨节点末端仍不能直接等同 ICU segment。详见 SURFACE_TEXT_SELECTION.md；
候选基础已验证，尚未完成生产双击接线，版本不变，不将此结果算作 Rust 全量回归。

## 2026-10-21 双击分词实际候选核验

15 类文本、288 个实际浏览器双击位置与 60 组 Windows SelectableWordsSegmenter
输出补证，发现中文、泰文、emoji/空白及边沿命中的差异，排除直接替换方案。
详情与可复跑证据见 SURFACE_TEXT_SELECTION.md；不是原生双击已完成或全量通过。
下一候选系统 ICU 尚待实际对照及最低 OS/加载边界核查。没有生产代码变更，仍为
v0.2.33.171；N01–N12 未完成，不提交、不发布，原 UIA 未确认异常继续保留。

## 2026-10-21 文字选择浏览器基线补证

v171 后进行了独立真实 Edge 组件/快捷键夹具验证，6 项通过。跨节点视觉选择存在，
但 Ctrl+C 进入现有 Unit copy；text 上 Ctrl+A 扩至夹具页面范围而非仅当前 Surface，
同节点与 input 的复制仍交浏览器。双击词选择能复制，原生尚无双击接线。
保持跨节点视觉范围待迁移项，不把现有复制门禁当成删功能的依据，也不擅自增加
聚合文本发布。详细结果、边界与 source-hash 证据见 SURFACE_TEXT_SELECTION.md。
本次仅补基线与台账，不改生产代码、不提升内部版本、不重跑无源码变化的 Rust 全量。

## v171 自动滚动的实际系统 timer 验证

在 v170 正式路径上补 Win32 消息队列证据：静止指针使用实际生成的 WM_TIMER
驱动滚动与扩选，松键后不继续生成；取消或撤权前已生成的 timer 消息，在其后
派发也不能改变选区/滚动。测试不改 due、不合成 WM_TIMER、不注入外部输入，
只读取自有 HWND 的 timer 消息。未改生产逻辑；物理输入和繁忙负载调度仍未验收。
N01–N12 整体迁移未完成，未提交、推送或发布。
新增 2 项系统 timer 专项通过（1.14 秒）；关闭 UIA 消息跟踪的可见全量
2564 passed/42 ignored（90.11 秒），日志 `linshi/otter-v171-full.stdout.log`；
startup 7 passed（15.37 秒）。Clippy all-targets -D warnings、fmt、3061 文件
严格行数、8 个本轮文本 UTF-8 无 BOM 与 diff 检查通过。v169 UIA 旧异常仍未称修复。
后续跨节点/全选先核对旧产品实际合同：现有复制门禁要求同一个 selectable root，
不能仅据 DOM Selection 能力就自行扩大原生文本聚合范围，详见文字选择文档。

## v170 文字拖选边缘自动滚动

正式捕获路径新增按需 50ms timer，复用祖先滚动与缓存投影，静止指针的选区持续
跟随滚动扩展，成功后同步原生控件。每轴一步最多 16 DIP，不追赶积压；hidden 轴
不新增鼠标滚动资格，无活动边缘拖选时没有 timer。松键、取消、失焦、撤权、替换、
关闭及边界停止；新手势有独立身份，防止同步撤销后的旧 pending 结果复活 timer。
不增加线程或文字副本，窗口根模块不增长。完整边界见 SURFACE_TEXT_SELECTION.md。
整体 N01–N12 未完成，既有 UIA 间歇性异常仍按 v169 证据保留；未提交、推送或发布。
新增 8 项 HWND 专项通过（0.74 秒），覆盖静止指针、上下/左右方向、hidden 轴、
嵌套内外消费、边界停机、回到内部、松键/撤权/失焦/替换/关闭、旧 timer 及
pending 查询同步取消。审查发现的撤销后 timer 复活路径已加手势身份复核。
关闭 UIA 消息跟踪的可见全量 2562 passed/42 ignored（87.00 秒），日志
`linshi/otter-v170-full.stdout.log`；Clippy all-targets -D warnings、fmt、3060
文件严格行数、15 个本轮文本 UTF-8 无 BOM 与 diff 检查通过。测试使用真实 HWND
和直接 WM_TIMER 消息，不冒充物理鼠标或长时间性能验收；候选查询仍有有界滚动
快照成本，未宣称最大场景鼠标延迟已经测量。
startup 7 passed（20.09 秒）；未创建公开发布版本或操作旧用户数据。

## v169 当前排版行的行首/行尾扩选

Shift+Home/End 已接正式键盘路径，以 DirectWrite 行度量定位逻辑行首/尾；End
保留尾随空格但排除换行符，软换行继续保存当前视觉行归属。原选区 anchor、
自动显露、EDIT 同步与焦点/权限门禁沿用；普通 Home/End 仍滚动。查询使用有界
临时行数组，非恒定成本，未冒充最大文本性能验收。完整边界见 SURFACE_TEXT_SELECTION.md。
仍未完成词边界、全选、跨节点、bidi 水平视觉导航、拖选自动滚动和 UIA；
N01–N12 整体原生迁移未完成，未提交、推送或发布。

本轮新增 5 项专项通过，startup 7 passed（16.51 秒），Clippy all-targets
-D warnings 与严格行数检查通过。但全量门未通过：首轮 2553 passed/1 failed/
42 ignored（91.01 秒），Select 完成 UIA 操作后没有 pending edit；独立进程
复跑该项 3 次通过，不代表修复。第二轮 2552 passed/2 failed/42 ignored
（82.77 秒），分别为 Edit 的 CurrentHasKeyboardFocus=false 与 Select 的
ListItem 数量 0（预期 2）。隔离 UIA client_tests 组仍为 15 passed/1 failed，
复现 Select 列表项缺失，说明不必运行新文本测试即可出现该症状，尚不能确认根因。
保留 `linshi/otter-v169-full.stdout.log`、`otter-v169-diagnostic-full.stdout.log`、
`otter-v169-uia-group.stdout.log` 与 `otter-v169-uia-select-anomaly.json`。
现阶段只增加失败分支状态采证，不放宽断言、不重试 UIA 操作、不改变生产激活策略；
不能把当前候选报告为全量通过或可发布。
增加 ListItem 失败分支只读状态记录后，UIA 分组 16 passed（日志
`linshi/otter-v169-uia-group-diagnostic.stdout.log`），未触发诊断分支，不能算根因修复。
下一判别点是实际 ComboLBox 的可见性/条目与 relocated UIA 子树是否一致；
pending 缺失另需区分消息尚未派发与 Host 取出期间的同步通知，尚无证据选定原因。
最终 Clippy、fmt、3057 文件严格行数、14 个本轮文本 UTF-8 无 BOM 与 diff 检查通过。

后续排查补充：只在 ListItem 原断言失败时读取 COMBOBOXINFO、原生条目数和
直接 ComboLBox 的 UIA 子项数，不重新展开或聚焦。第一次隔离运行却在 Edit
初始值断言读到 `2hello`（预期 `hello`，尚未执行测试写入），见
`linshi/otter-v169-uia-native-list-1.stdout.log`：15 passed/1 failed（8.30 秒）。
这增加了输入干扰或写入路径异常的候选，但不能证明来源，也不能解释全部旧失败。
现为测试自有 EDIT/COMBOBOX/父 HWND 增加每窗最多 32 条输入/焦点/通知消息记录，
无裸 Rust 指针，RAII 移除 subclass，所有消息仍交默认过程，不屏蔽输入或放宽断言。
加记录后三个独立 UIA 分组各 16 passed，日志 `otter-v169-uia-input-trace-{1,2,3}.stdout.log`；
没有捕获到失败消息链，不能据此宣称根因修复。
随后带消息记录全量 2554 passed/42 ignored（83.22 秒），日志
`linshi/otter-v169-input-trace-full.stdout.log`。只读复核指出同步日志会扰动时序，
因此消息记录改为仅 `OTTER_UIA_INPUT_TRACE=1` 显式启用，普通回归不安装该 subclass。
直接 ComboLBox 查询与原 Select 查询是先后采样，结果差异也不能单独证明合并树错误。
关闭消息跟踪后的最终全量 2554 passed/42 ignored（93.19 秒），日志
`linshi/otter-v169-final-untraced-full.stdout.log`；这是当前代码的通过证据，不抹去
上述失败，也不代表间歇性故障已修复。Clippy、fmt、3057 文件严格行数及 diff
检查再次通过。本轮只修改 3 个 UIA 测试文件与相关台账，不改生产逻辑，沿用 v169。
显式启用跟踪的独立冒烟 3 passed（3.51 秒），真实记录到 SETFOCUS、SETTEXT、
SHOWDROPDOWN 与父 WM_COMMAND，证据 `linshi/otter-v169-opt-in-input-trace.*.log`。
跟踪记录通知来源 HWND，但尚不记录 Host 是否被取出，不能声称重入分支已排除。

## v168 上下行文字扩选与软换行归属

Shift+Up/Down 已接正式父 HWND/Host/Presentation 路径，复用 DirectWrite 实际
行边界、cluster 命中与已有自动显露。唯一选区保存局部目标列及行探针，经过
短行仍能回原列，连续和首次鼠标后软换行导航不只依赖有歧义的 UTF-16 索引。
显露使用同一视觉行；水平操作/重排重置导航几何，纯滚动不清除。普通 Up/Down
滚动、EDIT 与焦点/权限边界保持。完整范围见 SURFACE_TEXT_SELECTION.md。
仍未完成 bidi 水平视觉导航、词边界、全选、跨节点、拖选自动滚动及 UIA；
N01–N12 原生迁移未完成，不能以本次键盘增量代替完整业务与人工验收。
新增 7 项回归通过；可见全量库 2549 passed/42 ignored（89.00 秒），日志
`linshi/otter-v168-full.stdout.log`。Clippy all-targets -D warnings、fmt、3055
文件严格行数、19 个本轮文本 UTF-8 无 BOM 与 diff 检查通过。审查发现的首次
鼠标软换行归属缺口已修复并加回归；无系统输入注入或剪贴板写入，未提交、推送或发布。
startup 7 passed（14.51 秒）；测试目标目录下无残留进程。

## v167 键盘选区端点自动显露与控件同步

已有水平键盘扩选/折叠现在显露实际端点 cluster；复用原滚动 nearest 和缓存
投影，嵌套祖先逐层消费，候选最终完全不可见则不提交。纯显露不重新测量文字，
hidden 轴不提升滚轮权限；仅滚动实际提交后同步既有 EDIT region/位置与草稿。
窗口测试按真实 region 验证，不误用 WS_VISIBLE 作为裁剪状态。详见
SURFACE_TEXT_SELECTION.md。仍不等于拖选越界自动滚动、bidi 视觉导航、上下行、
词边界、全选、跨节点或 UIA TextPattern；N01–N12 整体迁移未完成。
新增 6 项回归通过，可见全量库 2542 passed/42 ignored（87.44 秒）。Clippy
all-targets -D warnings、fmt、3053 文件严格行数、13 个本轮文本 UTF-8 无 BOM
及 diff 检查通过。日志 `linshi/otter-v167-full.stdout.log`。静态审查未发现确定
生产缺陷；未注入系统输入或写入剪贴板，未提交、推送或发布。
startup 7 passed（15.49 秒）；测试目标目录下无残留进程。

## v166 已有文字选区的水平键盘操作

Shift+Left/Right 已从父 HWND 接入唯一 Presentation 选区，按 DirectWrite 实际
cluster 沿文本逻辑顺序扩展/缩回；普通左右键折叠非空范围，无范围则保留原滚动。
准入要求实际父焦点、本地编辑及导航锚点匹配，不抢占 EDIT 或 Shift+Tab；活动
拖选、撤权、换场景不能复活旧选区。无第二份文字模型、远端事件或剪贴板写入。
完整边界见 SURFACE_TEXT_SELECTION.md：尚非 bidi 视觉导航、上下行/词扩选、
自动 reveal、全选、跨节点或 UIA TextPattern，N01–N12 整体迁移仍未完成。
新增 6 项回归；focused 19 passed/1 ignored，可见全量库 2536 passed/42 ignored
（83.46 秒）。Clippy all-targets -D warnings、fmt、3051 文件严格行数、16 个
本轮文本 UTF-8 无 BOM 与 diff 检查通过。全量日志 `linshi/otter-v166-full.stdout.log`。
只读交叉审查未发现确定阻断缺陷；无系统键盘注入或真实剪贴板写入。未提交、推送或发布。
startup 7 passed（14.77 秒）；测试目标目录下无残留进程。

## v165 文字 I-beam 与系统鼠标验证

正式光标命中与选择起点共享 DirectWrite/最上层节点准入，不修改选区，保留
遮挡按钮光标与本地编辑/提交独立门禁。显式自有可见窗口 SendInput 探针实际
收到标记 move/down/move/up，选中 Native selection、抑制祖先 click、释放捕获，
且实际系统 I-beam 已验证；不读写剪贴板，不冒充物理输入或第三方粘贴验收。
探针证据 `linshi/otter-native-text-selection-20261021-115903-b07aaa45`。
键盘扩选/全选、跨节点、双击、触摸与 UIA TextPattern 仍待推进，N01–N12 未完成。
最终可见全量库 2530 passed/42 ignored（76.56 秒），startup 7 passed（15.76 秒）；
Clippy all-targets -D warnings、fmt、3049 文件严格行数、13 个本轮文本 UTF-8 无 BOM
及 diff 检查通过。最终日志 `linshi/otter-v165-final.stdout.log`。交叉审查的应急
LEFTUP 返回值遗漏已修复并加回调测试，修正后系统探针再次通过（1.79 秒），
证据 `linshi/otter-native-text-selection-20261021-120415-2894da12`。未提交、推送或发布。

## v164 原生文字拖选、高亮与本地复制接线

接续 v163，Presentation 持有单节点选区，真实父 HWND 拖选复用现有捕获；
非空选择抬起不产生祖先整块 click，collapsed 单击保留原路径。高亮进入已有
文字绘制顺序与裁剪，规范信号黄派生 42% 透明度，不改变字色或另建隐藏 EDIT。
Host 重入取消通过单标记在归还前落实；完成选区跨同内容重排/resize 保留，
换场景、撤权或规范化文本变化不保留旧索引。
Ctrl+C 已接有界 CF_UNICODETEXT 准备/发布，清空前复核窗口焦点、场景和选区；
不走远端动作。自动验证不访问真实剪贴板，真实粘贴、键盘扩选/全选、跨节点、
触摸与 UIA 选择能力仍未验收。完整边界见 SURFACE_TEXT_SELECTION.md。
新增 14 项回归；可见全量库 2527 passed/41 ignored（75.93 秒），startup
7 passed（14.22 秒）；Clippy all-targets -D warnings、fmt、3047 文件严格
行数（全部不超过 500）、本轮 23 个文本 UTF-8 无 BOM 与 diff 检查通过。
日志 linshi/otter-v164-full.stdout.log。未提交、推送或发布。
显式跨进程 UIA：Slider 4、Select 1、通用 3 项通过；不等于新增文字选区 UIA 已实现。

## v163 OCR 可选择文本的原生排版基础

核实真实 OCR 场景输出 text.selectable=true，但原生此前只绘制文字；开始补齐
部分文字选择，而非用整块复制替代。Layout 现复用 DirectWrite 提供 cluster
命中、范围几何与规范化 UTF-16 选区文本；TextItem 负责 selectable 准入、当前
origin 与祖先裁剪。详见 [完整接线范围](SURFACE_TEXT_SELECTION.md)。
本增量尚未接入 HWND 拖选、可见高亮和本地复制；完整功能仍待后续实现。
新增 5 项真实 DirectWrite/最终文字框回归；可见全量库 2513 passed/41 ignored
（76.95 秒），Clippy all-targets -D warnings、fmt、3038 文件严格行数通过。
保留本轮夹具父定位上下文错误的排查记录，未为测试放宽布局准入。
全量日志 linshi/otter-v163-full.stdout.log；未提交、推送或发布。
startup 7 passed（13.23 秒）；本轮 11 个文本 UTF-8 无 BOM 与 diff 检查通过。

## v162 活动触摸滚动撤销与编辑队列实证

新增显式系统生命周期探针：先确认真实 pan 已活动且内容确已移动，再撤销本地
编辑/提交权限或替换场景；继续移动、抬起，并实际观察 GF_INERTIA 和 GID_END。
两种路径均保持停止位置 [0,-32]，无 pan、数值编辑、祖先 click 或错误残留。
水平探针进一步走正式 prepare_edit/settle_edits，实测 input continuous 后一次
change commit，最终值 72、共 2 个事件且队列清空；不代表网络服务端已验收。
本轮仅增强测试与诊断，不修改生产输入算法，不提升 touch capability。

稳定性保留项：v161 水平父滚动异常连续三轮未复现，根因仍未知；本轮
110914-ad3e28fb 在撤销前未观察到活动 pan 而失败，不能算生命周期通过。
保留原断言并增加撤销前消息、pan、origin、前台状态诊断；最新
linshi/otter-native-touch-20261021-111117-0c0d6ffe 四项通过（10.99 秒），
两种撤销前均 pan=true、origin=[0,-32]、foreground_own=true。后续通过不等于
已修复偶发前置条件失败；继续保留此项，不无证据归因于外部输入或时序。

常规门：可见全量库 2508 passed/41 ignored（79.44 秒），startup 7 passed；
显式跨进程 Slider 4、Select 1、通用 3 项通过。全量日志
linshi/otter-v162-full.stdout.log。最终忽略探针另行显式执行，不能混同常规全量。
最终源码 Clippy all-targets -D warnings、fmt --check、3036 文件严格行数
（全部不超过 500）通过；本轮 10 个文本 UTF-8 无 BOM、git diff --check
通过，目标测试目录进程残留 0。N01–N12 总体迁移仍未完成；未提交、推送或发布。

## v161 Slider 单指垂直父滚动接通

系统探针确认默认过程转发子 HWND 手势时保留子目标；据此在 Slider 创建时
配置单指垂直 pan/惯性、阻止单指水平 pan。父 Surface 只额外接纳当前挂载
Slider，BEGIN/END 也先验目标；Pan 绑定目标与序列，拒绝同序列跨目标干扰。
继续由系统完整管理 pointer 序列，不选择性吞消息，不新建触点表或滚动模型。

真实 Surface 注入证明垂直移动滚动父内容约 48 逻辑像素且 Slider=50、无编辑
队列/祖先 click；水平移动 Slider=72 且父内容不动、有数值队列、无 click。
配置读回、未知/外部/替换/忙碌目标拒绝、跨目标序列回归新增覆盖。
证据：linshi/otter-native-touch-20261021-104012-238ecac3，详见触摸基线 v161。
多指/笔、斜向阈值、复杂滚动边界和物理设备仍待验，不扩大 capability。

保留一次运行异常：104637-4351cfe7 的水平场景中，Slider=72 但父 origin 变为
[0,-48]，断言正确失败。当时未记录实际父窗口 gesture/wheel 来源，原因未定，
不能直接归因于外部鼠标或系统惯性。新增有界消息来源诊断、未放宽断言或改
生产行为后，104907-4b530308 的三项显式测试通过；其水平场景无父 gesture/
wheel，origin 保持 [0,0]。摘要 linshi/otter-v161-horizontal-anomaly.json。
此异常仍是稳定性待核验项，不以一次后续通过宣称已根治。

最终门：显式可见启动的全量库 2508 passed/40 ignored（80.20 秒），startup
7 passed（16.51 秒），显式跨进程 Slider 4、Select 1、通用 3 项通过；3 项
系统触摸探针通过。Clippy all-targets -D warnings、fmt、3035 文件严格行数
（均不超过 500）、本轮 14 个文本 UTF-8 无 BOM、PowerShell 语法与 diff 检查
通过。测试夹具接在已有 scroll_tests 下，未给 498 行窗口 owner 增加体积豁免。
未提交、推送或发布；全量日志 linshi/otter-v161-final-full.stdout.log。

## v160 系统触摸实际到达与兼容鼠标来源隔离

显式可见启动解决 v159 自有 HWND 命中失败；新入口
scripts/tests/Invoke-NativeTouchProbe.ps1 从 Cargo artifact 定位当前测试 EXE，
真实注入/真实 GetPointerInfo 校验不再停在模拟窗口消息。
系统实测抓到 tap/horizontal/vertical 错误发出父 click 候选，现按系统触摸标记
抑制兼容鼠标 click；另保存拖动来源，拒绝另一来源的 move/up/down 干扰。
鼠标/笔原行为、数值 FIFO 与取消路径保留，不更改生产依赖版本或 capability。
常规来源测试和实际四组系统触摸探针覆盖修复；父滚动、完整 pointer 序列 owner
及物理触摸设备仍未完成。详见 SLIDER_TOUCH_BASELINE.md 的 v160 增量。

验证：显式可见启动的全量库 2505 passed/38 ignored（79.19 秒），startup
7 passed（18.59 秒）；跨进程 Slider 4、Select 1、通用 3 项通过；显式触摸
四场景探针通过，最新证据 linshi/otter-native-touch-20261021-102252-10cad655。
Clippy all-targets -D warnings、fmt、3033 文件严格行数（全部不超过 500）、
本轮文本 UTF-8 无 BOM 与 diff 检查通过。未提交、推送或发布。

保留异常：普通后台启动的首轮全量为 2503 passed/2 failed/38 ignored，失败为
EDIT 焦点即时属性及 Button 客户端返回后逻辑焦点，两项随后单独复测通过。
同代码后台第二轮为 2504 passed/1 failed/38 ignored，Select 展开后条目数为 0。
此后只改变启动为显式可见方式，全量 2505 通过；未放宽或删除 UIA 断言。
这支持交互启动/焦点环境影响的排查方向，但不证明后台焦点不稳定已根治。
原始记录见 linshi/otter-v160-first-full-test.json、otter-v160-visible-full.stdout.log，
第二轮 RTK 日志为 1792549352_cargo_test.log。该问题仍需后续运行门稳定性处理。

## v159 显式 Windows 触摸系统探针

新增真实 InjectTouchInput/GetPointerInfo 探针（默认 ignored、环境变量双重
启用），用于补足 v158 浏览器模拟不能证明的原生消息接线。探针的 HWND 命中
门禁在本机发现外部窗口命中不符并拒绝注入；此次显式运行失败，尚无真实
触摸到达证据。保留保护与失败事实，不以合成消息或 API 成功代替接线通过。
详情、运行命令与证据边界见 [Slider 触摸基线](SLIDER_TOUCH_BASELINE.md)。
本轮不改变生产触摸实现或 capability；下一步先查清实际交互桌面命中问题。
最终常规验证：库 2503 passed/38 ignored（85.00 秒），startup 7 passed
（17.39 秒），Clippy all-targets -D warnings、fmt --check、3032 文件严格行数
（全部不超过 500）、本轮 9 个变更文本 UTF-8 无 BOM 与 diff 检查通过。
这些常规通过不覆盖上面的显式触摸失败；跨进程 UIA 本轮未重跑，生产代码未变。
本地失败摘要：linshi/otter-v159-touch-probe-evidence.json。未提交、推送或发布。

## v158 Slider 触摸基线与接入边界核验

实际 Chromium CDP 触摸输入验证：水平拖动 input/change 不产生 click；垂直
拖动取消 Slider 接触并滚动父容器；触摸取消保留已更新值但不 change；同值
轻触 Slider 不 click，而普通 button 轻触阳性对照产生 click。每组观察 400 ms，
不能将即时结果误判为兼容 click 尚未到达。具体证据和退出条件见
[Slider 触摸基线](SLIDER_TOUCH_BASELINE.md)。
此证据排除了“禁用全部手势、无条件提升为鼠标”的简单接法；生产应区分
水平 Slider 更新、垂直父滚动和取消，并完整管理 pointer 序列及兼容消息去重。
本轮仅修改基线/设计文档和内部迭代身份，不新增生产触摸实现、不提升 capability，
也不把浏览器模拟当物理 Windows 触摸验收。未重跑无源码变更的构建/运行门。

## v157 Slider 切焦提交与捕获手势分离

真实 Chromium 对照 Tab→抬起、Tab→继续拖动→抬起、程序性切焦→继续拖动→抬起
三组：切焦先 change 再 blur，后续移动仍产生 input，抬起产生必要的 change 和
原 Slider click。脚本 linshi/otter-slider-tab-probe.js，浏览器会话已正常关闭。
据此修正 v154 的“Tab 提交后释放捕获”：正常切焦只是提交当前边沿，不结束手势。

新增 checkpoint_drag，在 Tab 和 WM_KILLFOCUS 更新拖动提交基线并提交，不释放
鼠标捕获、不清原 click 按压；后续移动/抬起沿用原节点。父 NAVIGATE 仅对实际
捕获中的挂载 Slider 保留 native_press，其他控件保持原失效规则。切焦回调仍
复核存活；取消/禁用/隐藏/父失活等真正撤销路径继续 cancel，不复用 checkpoint。

真实 Surface 回归覆盖 Tab 和程序切焦到 EDIT 后保留捕获、继续拖动两组数值
事件、结算后祖先 click，以及 Tab 后父取消撤销捕获和迟到 click；已有 Tab
测试按实测更新为鼠标抬起才释放。新行为不把键盘焦点强留在 Slider。
另覆盖 Tab 后直接抬起不重复 change、切焦提交回调销毁 HWND；新增 4 项回归。
最终全量库 2503 passed/37 ignored（75.61 秒），startup 7 passed（13.43 秒）；
Clippy all-targets -D warnings、fmt --check、3031 文件严格行数（全部不超过
500）、1379 个原生 Rust 源码 UTF-8/无 BOM/空白及 diff 检查通过。
显式跨进程 Slider 4 项、Select 1 项及通用 3 项通过，原生测试进程残留为 0。
其他失焦边界、触摸混合输入、真实读屏和多显示器人工验收仍未完成。
未提交、推送或发布。

## v156 Slider 捕获释放保留原始 click 目标

真实 Chromium 对照修正此前对“共同祖先路由”的待确认假设：在 Slider 按下并
保持捕获时，释放于自身、兄弟按钮、内层容器、外层容器及容器外，5 组 click
目标均为原 Slider，然后沿其祖先链冒泡。脚本为 linshi/otter-slider-release-probe.js，
浏览器会话已正常关闭。因此不能按抬起坐标重新命中或改投释放位置的祖先。

增加 CapturedPointer 激活来源，仅由已校验的原生 Slider 按压生成。begin 仍须
实际捕获且命中控件内；end 允许越界，但重新核对原 HWND、挂载节点、场景身份、
编辑/发布权限和可聚焦资格。准备候选/合同沿用原节点及 FIFO fence，不对抬起
位置重新 hit-test；当前已挂载 Slider 的只读状态在 Host 再复查。普通 Pointer
和 Keyboard/Automation 的原有资格保持不变，不扩大通用坐标点击权限。

新增越界左右释放、原候选在替换后失效、控件外伪造 begin 拒绝以及真实兄弟
EDIT 上释放不改投的回归。原先 v155 越界释放拒绝是未完成状态，现已按真实
浏览器行为替换，不作为继续保留的产品限制。
最终全量库 2499 passed/37 ignored（78.07 秒），startup 7 passed（14.63 秒）；
Clippy all-targets -D warnings、fmt --check、3031 文件严格行数（全部不超过
500）、1379 个原生 Rust 源码 UTF-8/无 BOM/空白及 diff 检查通过。
显式跨进程 Slider 4 项、Select 1 项及通用 3 项通过，原生测试进程残留为 0。
Tab 后鼠标抬起、其他失焦/触摸混合输入、真实读屏和多显示器人工验收仍未完成。
未提交、推送或发布。

## v155 Slider 控件内鼠标完成后的祖先 click

核对主干 Slider 没有自身通用 click handler，祖先容器在 input/change 后接收
鼠标 click。新增 surface_window_slider_click，复用父 Pointer 单槽、场景身份、
form snapshot 和原 FIFO fence，不创建第二条业务发送路径。子 HWND 的私有
begin/cancel/end 仅表示输入边沿；父端重新验证挂载 Slider、权限、捕获和命中。
原生按压会参与 automation_busy，普通父场景失效同时撤销；NCDESTROY 显式撤销。

正常鼠标抬起先完成 change，再生成祖先 click 的 fence；因此祖先必须等待自身
数值边沿，而不是复制 Switch 的前置 fence。只在控件内完成手势时创建本路径，
拖动距离不套用普通按钮 click 阈值；同值点击可冒泡但不创建假 change。
无祖先 handler 不占槽；停止发布仍允许本地数值编辑但不产生 click；不覆盖
原有未消费候选。cancel/禁用/替换/销毁和伪装为 EDIT 的消息均不能产生旧 click。

新增 6 项真实 Surface 冒泡/准入回归；最终全量库 2495 passed/37 ignored
（77.47 秒），startup 7 passed（13.92 秒）。Clippy 指出的布尔表达式已等价
简化，最终 all-targets -D warnings 与 fmt --check 通过；3031 文件严格行数
全部不超过 500，1379 个原生 Rust 源码 UTF-8/无 BOM/空白及 diff 检查通过。
显式跨进程 Slider 4 项、Select 1 项及通用 3 项通过，原生测试进程残留为 0。

本增量不宣称完整 DOM click 对齐：跨控件/控件外释放的共同祖先路由，以及
Tab 完成后迟到鼠标抬起的 click 尚需后续对照；触摸、真实读屏和多显示器人工
验收亦未完成。未提交、推送或发布。

## v154 Slider 拖动与键盘混合输入对照修复

使用真实 Chromium 的 input[type=range] 采集 5 组事件顺序，每组导航新文档
避免上一手势状态污染；脚本位于 linshi/otter-slider-mixed-probe.js。
拖动中方向键改变值会立即 input/change，单纯抬起不再 change；之后继续拖动
新值仍在抬起时提交。Tab 提交当前拖动值后转移焦点；Escape 不结束拖动；
边界方向键没有改值时不提前提交。click 冒泡也观察到，但本增量未实现。

先加入原生回归，明确复现同值重复 change 和 Tab 漏 change 两项失败。
随后只修改 surface_slider_messages：键盘提交更新当前拖动基线，Tab/抬起
共用 finish_drag；释放捕获后复查 owner、存活、enabled 和数值，避免重入
更新/销毁后发送旧事件。Tab 提交回调销毁 HWND 时不得继续导航。
不改变 WM_CANCELMODE、禁用/隐藏等撤销路径的无提交语义。

新增 6 项自有 HWND 混合输入回归和 1 项真实 Surface 队列/焦点回归；
surface_slider_ 专项共 26 项通过。Surface Tab 先产生唯一 input/change 数值对，
再切到 EDIT，捕获释放且迟到鼠标抬起不重复提交。Browser 会话已正常关闭。
最终全量库 2489 passed/37 ignored（75.96 秒），startup 7 passed（14.62 秒）；
Clippy all-targets -D warnings、fmt --check、3029 文件严格行数（全部不超过
500）、1377 个原生 Rust 源码 UTF-8/无 BOM/空白及 diff 检查通过。
显式跨进程 Slider 4 项、Select 1 项及通用 3 项通过，原生测试进程残留为 0。
完整点击冒泡、其他失焦/触摸混合输入、真实读屏及多显示器人工验收仍未完成。
未提交、推送或发布。

## v153 Slider 只读通知与范围合同替换验收

Mount 跟踪已提取的 IsReadOnly 状态，编辑权限变化在恢复 Host 后进入同一延迟
消息调度；首尾同值合并、发送前检查当前身份及权限，关闭/替换清旧通知。
`surface_window_range_readonly` 只处理有类型的权限差量，不新增消息、timer
或业务状态。值通知和权限通知均在无 Host/root/队列借用时发送。

真实 MTA 客户端订阅 RangeValue.IsReadOnly，UI owner 经有界通道接收测试策略
指令：停止发布但保留本地编辑不变为只读且不发通知；撤销编辑收到 true，
重复撤销不重发，恢复编辑收到 false。只读时 SetValue 拒绝且原值 50 保持，
测试不生成业务事件；订阅 handler 最终释放。新增队列净零/关闭回归。

核对现有编辑合同仅忽略 value，min/max/step 都参与合同摘要，变化会重建 HWND
并撤销旧 provider，而非同身份内修改 Range。新增真实 COM 回归验证旧值事件
清空、旧引用拒绝、新 provider 读取 -10/10/0.5/2 的范围、步距、翻页及初始值。
因此这里遵循结构替换通知，不向已撤销身份伪造范围属性变更；不是漏发同身份
元数据变化。若未来支持原位范围修改，必须新增对应属性通知与身份兼容证明。

RangeValue 普通专项 10 项通过。全量库 2482 passed/37 ignored（75.40 秒），
startup 7 passed（14.36 秒）；Clippy all-targets -D warnings、fmt --check、
3028 文件严格行数（全部不超过 500）、1376 个原生 Rust 源码 UTF-8/无 BOM/
空白和 diff 检查通过。真实读屏、完整点击
冒泡、混合输入和多显示器人工验收仍未完成；未提交、推送或发布。
显式跨进程 Slider 4 项、Select 1 项及通用 3 项通过，原生测试进程残留为 0。

## v152 Slider 存活订阅跨进程退出隔离

在真实子进程退出前保留额外的系统 RangeValue.Value 订阅，先实际收到并验证
60/61/70 数值序列，证明订阅确实活跃。正常关闭与强制结束自有 Child 后均
保留该订阅，重启同名 Slider 并再次完成数值/键盘/通知及六个业务边沿验收；
旧订阅必须保持安静，不能接收新进程的同名节点事件，旧 provider 仍不能读写。
随后从系统 UIA 移除旧订阅，确认事件处理对象释放。Observer 复用既有 handler
和 RAII Subscription，通道 16 槽、接收期限与重复排空次数均有界。

新增 2 项显式跨进程回归，两种退出方式均通过；Slider 专属跨进程共 4 项通过。
全量库 2479 passed/37 ignored（74.67 秒），新增 ignored 均已显式通过；
startup 7 passed（13.93 秒）。Clippy all-targets -D warnings、fmt --check、
3026 文件严格行数（全部不超过 500）、1374 个原生 Rust 源码 UTF-8/无 BOM/
空白及 diff 检查通过。
既有 Select 跨进程 1 项及通用跨进程 3 项通过，原生测试进程残留为 0。
本轮不改变生产架构。只读/范围元数据通知、真实读屏、点击冒泡、混合输入及
多显示器人工验收仍待完成，未提交、推送或发布。

## v151 Slider 专属跨进程 UIA 验收

独立 STA 子进程挂载真实 Slider，父 MTA 客户端只通过系统 UIA 和已验证属于
该 Child 的 HWND 操作，不读取子进程内部 State。复用数值订阅验收，检查唯一
节点、进程身份、范围读数、SetValue、真实键盘变化及取消订阅后的回调释放。
子进程经正式 prepare/settle 出口逐项校验 60/61/70 的六个 input/change 数值
边沿及 continuous/commit 类型，全部结算后才发 accepted；不是只看最终 HWND 值。

显式跨进程 2 项通过：正常关闭和强制结束自有 Child 两种路径均使旧 RangeValue
读写失败；重启同名场景后新实例回到 50，旧引用不能修改新实例，再次完成完整
订阅与业务边沿验收。Fixture 沿用管道命令、生命周期/输出上限及 Child 句柄
清理，不按进程名称扫描终止，不新增生产线程或 timer。

全量库 2479 passed/35 ignored（77.82 秒），新增 2 个 ignored 为已显式运行的
跨进程测试，不是跳过失败；startup 7 passed（13.77 秒）。Clippy all-targets
-D warnings、fmt --check、3026 文件严格行数（全部不超过 500）、1374 个原生
Rust 源码 UTF-8/无 BOM/空白及 diff 检查通过。
既有 Select 跨进程 1 项及通用跨进程 3 项通过，原生测试进程残留为 0。

只读/范围元数据通知、存活订阅跨进程退出隔离、真实读屏、点击冒泡及混合输入/
多显示器人工验收仍待完成。本轮不宣告全量 Windows 原生迁移完成，未提交发布。

## v150 Slider 数值变化通知与系统焦点修正

从 Mount 已接纳数值提取 RangeValue.Value 差量，在 State 恢复 Host 后进入原
单消息通知调度。每节点保留首个旧值和最新值，净零变化不发；忙时重新排队，
发送前复核 runtime 身份及最新值。所有 Raise 均不持有 Host/root/队列借用，
回调重入不能发送旧批次值，替换和关闭清除旧身份通知。生产不新增 timer/线程。

新增 3 项通知队列回归及 1 项真实 MTA 订阅测试；与既有 RangeValue 共 7 项。
系统订阅验证 SetValue、真实子 HWND 键盘数值通知、同值/越界无通知和回调释放。
首次全量暴露键盘通知超时；保留诊断后复现为 UIA SetFocus 未保留 Slider 焦点。
修正 Slider 自定义属性的 interactive 判定，并设置 ProviderOwnsSetFocus，
使用原有子 HWND 焦点桥，避免 UIA 代选最近窗口。新增实际焦点和键盘接受断言，
修正后完整 UIA 客户端组 15 项通过，不用延时或忽略测试规避。

最终全量库 2479 passed/33 ignored（74.93 秒），startup 7 passed（14.50 秒）；
Clippy all-targets -D warnings、fmt --check、3025 文件严格行数（全部不超过
500）、1373 个原生 Rust 源码 UTF-8/无 BOM/空白及 diff 检查通过。
既有跨进程 Select UIA 1 项及通用 UIA 3 项通过，原生进程残留为 0。

当前只覆盖 Value 通知；只读/范围元数据通知、Slider 专属跨进程订阅、真实读屏、
完整点击冒泡及混合输入/多显示器人工验收仍未完成。未提交、推送或发布。

## v149 Slider UIA RangeValue 实时读写

Slider 复用现有 HWND override 和逻辑节点身份，提供 Slider ControlType、
RangeValue 模式与实时 value/min/max/step/page/read-only；不创建第二份数值状态。
SetValue 经 UI owner 身份、权限、交互占用和共享 FIFO 双边沿容量检查，
拒绝非有限及越界值，范围内按声明 step 归一，相同值不产生事件。
接纳后生成原有 input/change 数值快照；不点击、不抢焦点、不直接联网。
捕获中或队列已满拒绝写入并保留原值；合同替换和关闭撤销旧 provider。

新增真实 COM 回归与独立 MTA 系统客户端共 3 项通过，包含 canonical IUnknown、
范围读数、数值归一、事件对、权限撤销、捕获冲突、32 槽容量和 provider 失效。
系统树中按 AutomationId 只找到一个 Slider，RangeValuePattern 可实际读写。
本轮全量库 2475 passed/33 ignored（75.18 秒），startup 7 passed（14.50 秒），
Clippy all-targets -D warnings、fmt --check、3023 文件严格行数（全部不超过
500）、diff 和 1371 个原生 Rust 源码 UTF-8/无 BOM/空白检查通过。
既有跨进程 Select UIA 1 项及通用 UIA 3 项通过，原生进程残留为 0。
属性变化通知、Slider 专属跨进程测试、真实读屏及完整混合输入验收仍未完成；
不能据此声明整个 Windows 原生迁移完成。未提交、推送或发布。

## v148 Slider 接入真实 Surface 和共享 FIFO

将 Slider 加入原生字段测量、64 字段准入、Mount 和焦点候选；新
surface_edit_sliders 管理 HWND、合同、source、accepted/committed 和几何快照。
父窗口私有消息先路由到实际挂载 owner，再验证 editing/disabled/visible；
input 与 change 使用唯一 32 槽 FIFO，prepare 按原 phase 编码有限 number。
不创建文本 draft，不加入 surfaceValues，不直接联网发送。键盘 input 前复用
can_push_step 预留双边沿空间；拒绝时恢复已接纳值，使子窗口不继续发出该次
change。队尾同字段 input 合并，commit 不合并，取消/重试/结算沿用原收据。

source 未变时不覆盖本地值，几何未变时只刷新裁剪而不打断捕获；source 变化
静默回写，几何变化取消手势。父取消/失活路径同时取消 Slider 捕获。合同或
类型替换先清旧 waiting/prepared 意图，再建同 ID 字段；Select/Switch 的保留
逻辑同时识别 Slider，避免每次重排误清有效队列。HWND 身份/焦点方法提取至
surface_edit_controls，使 Mount 保持严格行数门禁；没有第二份窗口索引。

真实 Surface 回归 6 项通过：键盘 input/continuous→change/commit 数值 payload；
取消后原快照重试；拖动 input 合并；同场景同步保留同一 HWND 和捕获；32 槽
满后下一键恢复原值且没有半对事件；权威 value/合同/权限更新；Tab 跳过禁用
Slider 并往返 EDIT；父取消释放捕获及 Slider↔Select 替换清旧意图。
测试通过真实子 HWND 消息进入生产父处理器，不直接调用 notify 或发送网络。

验证：全量库 2472 passed/33 ignored（74.39 秒），startup 7 passed（14.27 秒），
显式跨进程 Select UIA 1 项及原有 UIA 3 项通过。Clippy all-targets -D warnings、
fmt --check、3019 文件严格行数（全部不超过 500）、diff、14 个本轮源码
UTF-8/无 BOM/空白检查通过，原生进程残留为 0。当前仍缺 Slider UIA RangeValue、
完整点击冒泡及触摸/混合输入/高对比度/多显示器人工验收，不提升整体 capability，
未提交、推送或发布。

## v147 Slider 真实原生子窗口与输入生命周期

新增 surface_slider_control/messages/paint，采用原生子 HWND 与 GDI 自绘，
仍用 Range 的浮点值，不引入整数刻度近似或文本草稿。键盘方向、Home/End、
PageUp/PageDown 的 input→change 顺序及数值与真实 Chromium 探针对照；探针
位于 linshi/otter-slider-input-probe.js，仅使用自建页面，已关闭浏览器会话。
鼠标坐标按有符号 16 位解析并钳制，系统捕获期间只在值变化时 input，正常
抬起按手势初值判断一次 change；取消、捕获丢失、禁用、隐藏、失焦、销毁
清除手势/捕获，不伪造 commit，已产生的本地值不回滚。

State 归 HWND 生命周期所有，Control marker 只授权当前窗口操作。所有可能
重入的 Win32/父通知调用期间不保留 State 引用；input 接收方可同步销毁、禁用
或改值，后续 change 必须复核。初始化尚无 State 时 Drop 仍销毁自有 HWND；
State 已移除时公开读写拒绝，防止销毁重入读取释放内存。真实回归覆盖按键、
端点去重、拖动、重复抬起、取消/disable/hide/销毁、input 回调销毁、owner
撤销和 State 缺失。绘制使用 stock 对象并恢复 DC，默认/焦点/禁用像素与
值移动已验证；已查看 linshi/otter-v147-colors 的 middle、disabled 图像。

新增根模块声明触发 desktop/mod.rs 501 行严格门禁，未添加豁免。原公开启动
参数适配和 SessionStartup 完整迁至 startup_entry；公开导出名、资源打开顺序、
窗口/COM owner 不变，由 startup 回归保护。交叉检查发现初始 Slider 私有
消息与 EDIT COMMIT 重号，已改为 WM_APP+0x55d，加入 EDIT 命令不重号断言。

当前仅独立控件 owner，尚未挂入 Mount 的共享 FIFO、Surface 布局/输入准入或
UIA RangeValue，不声明端到端 Slider 可用。连续手势混合键盘、触摸、自动化
和真实多显示器/高对比度交互仍需后续验收。未提交、推送或发布。

补充 1×1 客户区绘入更大调用者 DC 的回归，明确 IntersectClipRect 防止最小
拇指越界绘制。最终全量库 2466 passed/33 ignored（73.93 秒），其中 Slider
模型/wire/真实窗口回归共 13 项；启动入口拆分后的 startup 7 passed（13.51 秒）。
Clippy all-targets -D warnings、fmt --check、3016 文件严格行数（全部不超过
500）、diff 与本轮 7 个源码 UTF-8/无 BOM/空白检查通过。

## v146 Slider 数值模型与事件基础

核实当前前端 DeclarativeSurface.tsx 的真实 Slider 分支：有限 number 属性
min/max/step/value 缺省 0/100/1/0；input/continuous 和 change/commit 均取
valueAsNumber，不进入文本草稿或 includeSurfaceValues。该控件不调用
surfaceNodeStyle，不把 Select 的样式接线直接复制到 Slider。
原生过去仅有 Kind::Slider 枚举，无数值模型或事件编码支持；本次新增 Range
处理钳制、step 网格、位置换算和单步，wire 复用 surface_input_event。

联网核对 [HTML range 规则](https://html.spec.whatwg.org/multipage/input.html#range-state-(type=range))，
并通过独立 Playwright CLI Chromium/about:blank 创建临时 range 验证实际结果：
0..100/step20/value50→60，0..10/step3/value10→9，-10..10/step4/value-8→-6，
0.1..0.9/step0.2/value0.4→0.5，max<min→min，step<=0→默认 1，越界钳制。
裸 HTML 未声明 value 的 50 不适用于 JSX 显式缺省 value=0。探针位于
linshi/otter-slider-probe.js，仅操作自建页面，浏览器会话已关闭。

Range 不分配、不循环步进、不截断为 i32 刻度；跨度溢出、网格数量达到
2^52 或无法区分一步的浮点范围返回 Precision。这是明确的未完成兼容边界，
不是静默放大步距。测试覆盖严格数值默认、归一夹具、单调位置、边界步进、
非有限输入，以及两类事件数值 payload/绑定/禁用隐藏/缺少 action/错误类型。
当前仅模型和 wire，尚未挂载 Slider 原生控件、键盘/鼠标或 RangeValue UIA；
不宣告 Slider 已可用，不扩大 capability。下一步接原生控件和共享事件准入。

验证：Slider 专项 5 passed；全量库 2458 passed/33 ignored（74.86 秒），
startup 7 passed（14.23 秒），Clippy all-targets -D warnings、fmt --check、
3010 文件严格行数（全部不超过 500）、diff 和 6 个本轮源码 UTF-8/无 BOM/
空白检查通过。未提交、推送或发布，Windows 原生整体任务保持进行中。

## v145 Select 声明文字色到真实 HWND

核对主干 declarativeSurfaceStyle.ts 后确认 fontFamily 不在样式白名单内，
不把旧增量的“待支持”理解为扩大协议授权。本次补既有 color 合同：PaintFrame
复用 Color::parse 和父级计算值，保存 Select 有效的显式文字色，由 Presentation
到 Mount 更新 HWND 私有 RGBA 快照。控件默认色由 class 提供，不因父 color
自动改变；显式 currentcolor 使用父计算值。非法声明/无声明回到默认，alpha
相对当前正常/hover 背景合成。收起区域及未选中列表行使用声明文字色，选中行、
disabled 与 high-contrast 维持各自状态色。未实现声明 background/opacity 或
尚未支持的主题变量，不将此项表述为全部声明式样式完成。

颜色快照按 HWND 生命周期释放，SetProp 失败保留旧值，相同值不重复分配。
初版像素测试沿用样式替换前 HWND，真实返回 0x80070578；检查 fingerprint
确认 style 属于编辑合同后，改为检查重建后的实际 HWND，没有改变生产合同。
另测仅父级颜色变化时保留同一 HWND、更新 currentcolor，且零 change。
已查看 linshi/otter-v145-colors/select-parent-color.png 与 select-alpha-hover.png。

验证：Select 专项 32 passed；全量库 2453 passed/33 ignored（73.91 秒），
startup 7 passed（14.60 秒），显式跨进程 Select 1 passed。Clippy all-targets
-D warnings、fmt --check、3007 文件严格行数（全部不超过 500）、diff 和本轮
10 个源码 UTF-8/无 BOM/空白检查通过。Windows 原生整体任务仍进行中，
人工读屏、高对比度切换、多显示器及其余样式验收仍未完成；未提交、推送或发布。

## v144 Select 列表选中行主题与字体行高

采用 CBS_OWNERDRAWFIXED + CBS_HASSTRINGS，保留系统选项字符串、声明顺序、
键盘、Selection/ExpandCollapse 与原 change 去重链。父 HWND 安装一个无堆
状态 subclass，仅处理当前父窗口下带私有 owner 标记的 COMBOBOX WM_DRAWITEM；
父窗口销毁时移除。选中行使用 #f3f5f1 背景、#20252b 文字，未选中行仍为
#111720/#f7f8ef。借用 stock 画刷、SaveDC/RestoreDC 恢复调用者状态，clip
限制在行内；高对比度显式使用系统颜色，不改用户全局辅助功能设置。
surface_select_text 统一从系统字符串表有界读取，空选择不查询无效索引。
字体更新后用实际 GDI text metrics 设置固定行高和选择区域高度；相同高度
不重设，避免关闭展开列表。新增 16/32/48 字号增长与相同样式保持展开回归，
原列表像素测试增加选中背景和文字断言。已查看 linshi/otter-v144-colors/
select-popup.png，浅底深字选中行与深底浅字未选中行均正确显示。

验证：Select 专项 29 passed；同进程 UIA 2 passed/1 ignored，显式跨进程
Select 1 passed；全量库 2450 passed/33 ignored（71.76 秒），startup 7 passed
（14.10 秒），原有跨进程 UIA 3 passed。Clippy all-targets -D warnings、fmt
--check、3005 文件严格行数（全部不超过 500）、diff 与本轮 7 个源码
UTF-8/无 BOM/空白检查通过。未做人工读屏、高对比度切换或实际跨显示器验收；
声明式颜色等尚未完整，Windows 原生整体迁移仍未完成。未提交、推送或发布。

## v143 Select hover 进入/离开与状态清理

修复前真实内存绘制在局部 mouse move 后，目标 controlHover #18222c 像素
为 0/6496（RTK 1792536539）。新增窄 hover 接缝，仅当前 client 内、enabled
控件进入热态；已有 subclass ref-data 保存 bool。先写状态再请求系统
TrackMouseEvent(TME_LEAVE)，失败清除；重复 move 不重设状态或重绘。
leave、disable、hide、cancel 或客户区外有符号坐标清除；NCDESTROY 取消
跟踪并由原流程移除 subclass。没有自有 timer、堆 owner 或全局指针注入。
真实像素回归覆盖进入/离开、禁用后重启用、隐藏后重显示、取消和负 x 坐标，
要求值保持且零 change。已查看 linshi/otter-v143-colors 中 hover、leave、
disabled 的实际内存绘制图像。Select 专项 28 passed；全量库 2449 passed/
33 ignored（70.80 秒）；startup 7 passed（14.24 秒）；显式跨进程 Select
1 passed。Clippy all-targets -D warnings、fmt、3003 文件严格行数（全部不超过
500）、diff、7 个本轮源码 UTF-8/无 BOM/空白检查通过，原生进程残留为 0。
列表选中行完整主题、声明式颜色和人工读屏仍待后续，未提交、推送或发布。

## v142 Select 收起区域默认/焦点/禁用绘制

实际 WM_PRINTCLIENT 图像确认收起控件仍为白底黑字，6496 像素中 Neuro
背景为 0（失败 RTK 1792535823），不能由上轮弹出列表通过推断收起主题完成。
新增 surface_select_paint，只绘制当前 COMBOBOX 收起客户区，复用系统索引、
标签、字体与 GetComboBoxInfo 几何。默认深底浅字、箭头、12% 白派生边框，
focus 使用至少 2 DIP 信号黄边框，disabled 文字/箭头使用 textMuted。
系统仍拥有事件、键盘、弹出与 UIA；高对比度使用原系统绘制。WM_PAINT 直接
BeginPaint/EndPaint，不先绘制白底；失败回退系统 WM_PRINTCLIENT。调用者
DC 状态由 RAII 恢复，只借用 stock pen/brush，不新增 GDI 对象所有权。
验证：Select 专项 26 passed；全量库 2447 passed/33 ignored（71.95 秒）；
startup 7 passed（14.36 秒）；显式跨进程原有 UIA 3 项、Select 1 项通过。
Clippy all-targets -D warnings、fmt --check、3001 文件严格行数（全部不超过
500）、diff 和本轮 5 个源码 UTF-8/无 BOM/空白检查通过，进程残留为 0。
已实际查看 linshi/otter-v142-colors 的 select-closed.png、select-focused.png、
select-disabled.png，默认深底浅字、黄色焦点框、禁用灰字/箭头均可见。
图像仅来自自有 HWND 的内存绘制，不是其他应用或用户桌面截图。
hover、选中列表行、声明式颜色与人工读屏等仍待后续；未提交、推送或发布。

## v141 Select 弹出列表未选中行配色

按 Neuro 唯一设计源核对控件背景 #111720、文字 #f7f8ef。真实列表内存绘制
修复前 9324 像素中目标背景为 0，断言失败（RTK 1792535271）。Select 子类
现校验 GetComboBoxInfo.hwndList 后响应 WM_CTLCOLORLISTBOX，复用新增的
surface_control_colors 与原 EDIT 色值，不扩大 Mount.owns，不合成选择事件。
使用系统 DC_BRUSH，无新增画刷生命周期；探测到高对比度或无法读取状态时
交回系统。像素回归同时验证未选中行背景和文字，可显式导出自有窗口内存 PNG。
验证：Select 专项 23 passed，全量库 2444 passed/33 ignored（71.39 秒），
startup 7 passed（13.69 秒）。Clippy 提示将测试的 iter().any 改为等价 contains，
改后 Clippy all-targets -D warnings、fmt、像素专项与显式跨进程 Select 各 1 项
均通过。3000 文件严格行数（全部不超过 500）、diff、6 个本轮源码 UTF-8/
无 BOM/空白检查通过，原生进程残留为 0。已实际查看测试导出的内存绘制 PNG：
`C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-v141-colors/select-popup.png`，
未选中行深底浅字，选中行仍为系统高亮；没有截取其他应用画面。
选中行/箭头/完整焦点边框及声明式颜色仍保留系统行为或待后续，
不把此增量声明为完整 Select 主题验收；未提交、推送或发布。

## v140 Select 声明式字号与字重

复用现有 Styles 的继承、em、百分比解析，NativeMeasure 将已解析 Style
提供给场景 Cache；仅为 Select 保存 ControlFont owned 快照，滚动复用。
Mount 使用快照更新 GDI 字体，默认与宿主 Segoe UI/16/400 对齐，不再使用
stock 字号代替已测量字号；仅字重变化也会替换字体，同样式复用原句柄。
字体名固定栈缓冲并限制到 GDI 31 UTF-16 units，拒绝控制字符/NUL、非有限
尺寸和越界字重，失败保留旧资源。新增真实 HWND 父级 20→150%=30、2em=40、
继承字重及样式替换零 change 回归；另测字重替换旧字体释放和无效样式不变。
GDI 高度按像素取整且最少 1 像素；零字号完全隐藏的 CSS 语义、声明式
fontFamily、主题颜色/行高及实际跨显示器/读屏验收不在本增量完成范围。
验证：Select 专项 22 passed，全量库 2443 passed/33 ignored（72.22 秒），
startup 7 passed（14.11 秒），显式跨进程 Select 1 passed；Clippy all-targets
-D warnings、fmt --check、2998 文件严格行数（全部不超过 500）、diff 与
本轮 9 个源码文件 UTF-8/无 BOM/空白检查通过，原生进程残留为 0。
未提交、推送或发布，不扩大 capability 声明。

## v139 Select DPI 字体资源接缝

现状：Select 使用固定 DEFAULT_GUI_FONT，布局像素随 DPI 变化而字体不变。
新增 surface_select_font，基于固定 96 DPI stock LOGFONT 计算目标尺寸，
同字号复用，不累积变换；DPI 限定 48..768，拒绝非法值且不替换现有字体。
字体由 HWND 持有而非 Rust Control：WM_SETFONT 返回后释放旧字体，默认
WM_NCDESTROY 结束后释放当前字体，避免 owner marker 撤销造成悬垂句柄。
Mount 在几何更新时同步 DPI，不伪造业务通知。新增真实 GDI 缩放/复用/销毁
及 WM_DPICHANGED 端到端零 change 回归。未提交或发布。
验证：Select 专项 20 passed；全量库 2441 passed/33 ignored（69.59 秒）；
startup 7 passed（13.60 秒）；显式跨进程 Select 1 passed；Clippy all-targets
-D warnings、fmt --check、2997 文件严格行数（全部不超过 500）、diff 与
本轮 7 个源码文件 UTF-8/无 BOM/空白检查通过，原生测试进程残留为 0。
测试边界：人工发送 WM_DPICHANGED 不会改变系统窗口 DPI；宿主仍采用
GetDpiForWindow 的权威值。不同 DPI 的实际字体句柄通过 Control.set_dpi 驱动，
不能把该自动化等同于实际多显示器切换。释放检查必须给 GetObjectW 提供真实
LOGFONT 缓冲；零长度/NULL 查询可能只返回结构大小，不能证明句柄仍有效。
声明式字体样式、配色、实际多显示器/读屏体验仍待后续，不扩大 capability。

## v138 新证据：Common Controls v6 是 UIA 前置条件

本增量的软件门禁已通过：Select 专项 17 passed；UIA 系统基线/真实 Surface
2 passed；全量库 2438 passed/33 ignored（70.97 秒）；startup 7 passed
（14.46 秒）；显式跨进程原有测试 3 passed、Select 1 passed；Clippy
all-targets -D warnings 与 fmt --check 通过。版本仍为 v0.2.33.138，未发布。
此结果不等于整个 N08 或 Windows 迁移完成，主题/DPI/读屏人工体验仍另验。

v6 兼容性回归已处理：相同 Select 锚点只刷新遮挡区域，避免冗余 SetWindowPos
关闭列表；真实滚动仍先收起再移动。新增 CLOSEUP/键盘去重测试。placeholder
像素测试将 BI_RGB 的未定义高字节归零，保留全部 RGB 精确比较，3 项通过。
从链接后的测试 EXE 提取 manifest，确认 Common Controls 6.0.0.0 以及原
asInvoker/uiAccess=false。无旁置 manifest 运行依赖。
跨进程命令使用 uia_external_process_ 和 uia_select_external_；不要用整个
process_tests --ignored，否则会误直启必须由父进程设置专属环境的 child fixture。

临时 EXE 副本嵌入 v6 manifest 后，原失败测试通过 UIA 导航边界、两项列表、
Selection/SelectionItem 和折叠检查，推进到最终 change 队列断言。随后由
`native/build.rs` 为 Windows MSVC 的程序和测试统一嵌入 `windows.manifest`。
不依赖运行时旁置文件，不增加第三方 crate，也不改权限或 DPI manifest 策略。
微软 [标准控件 UIA 支持](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-controlsupport)
说明完整支持需要 Common Controls v6。此前“必须继续自建 Ex 身份适配”的推论
已被此对照推翻；下面的 MSAA 缺陷和失败原型保留作历史证据，不再作为下一步方案。

进一步实际通知显示，UIA SelectionItem 把 CB_GETCURSEL 从 0 改到 1，却没有
CBN_SELCHANGE；Collapse 发出 CBN_SELENDCANCEL/CBN_CLOSEUP。Mount 现也在真实
CBN_CLOSEUP 上核对最终索引，通过原 accepted 去重、权限与共享 FIFO 准入，
不伪造 CBN_SELCHANGE。该路径已通过上述软件门禁，不扩展 capability。
纯系统基线现在直接验证 v6 UIA 树；不再让已知系统 MSAA 诊断提前中断它。
原诊断源码和副本位于 linshi/otter-v138-v6-probe，日志 1792533203、1792533265。

## 历史排查：N08 Select UIA 接线与 MSAA 导航故障定位

继续验证得到：IAccessible 窄包装可使直接 MSAA NAVDIR_NEXT 返回正确结果，但
仅透传该接口会丢失系统 ExpandCollapse；补 IOleWindow/IAccIdentity 不足以
恢复，透传 IServiceProvider 后模式恢复而 UIA 导航仍越界。限制为公开
IAccessibleEx 服务或仅屏蔽 List 的 Ex 都没有修复。原型已撤回并保存在
`C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-v138-msaa-prototype/`，不进入
生产实现。不要重复只改 HWND popup、简单 MSAA 包装或关闭模式的已排除方案。
下一步必须处理 Ex/IAccessible pair 与模式返回对象的身份映射；直接 MSAA
通过不等于 UIA 通过。参见微软
[IAccessibleEx client 转换路径](https://learn.microsoft.com/en-us/windows/win32/winauto/using-iaccessibleex-from-a-client)。

内部版本 v0.2.33.138 尚未完成，不能作为通过验收的增量。Select 已接入当前
Mount HWND 映射、结构变更标记和既有 override 路由；真实客户端已走过唯一
AutomationId、ComboBox 类型、逻辑 group 父级、实际焦点及 ExpandCollapse。
展开后的 List sibling 导航仍会越出控件所属进程，Selection/change 队列断言
尚未到达；没有删掉失败断言，也没有把这个状态列为 UIA 完成。

对照证据：纯系统 STATIC 父 HWND 下的独立 Control 同样复现；独立进程客户端
也同样复现。进一步直接从系统 Combo IAccessible 的第三个 child 取得 List，
调用 accNavigate(NAVDIR_NEXT, CHILDID_SELF)，实际返回 VT_DISPATCH(9)，而非
VT_EMPTY(0)。直接从 hwndList 的 OBJID_CLIENT 取对象则未复现该方向错误，必须
区分 Combo 子对象路径与 popup HWND 路径。微软已记录对应缺陷：
[Combo Box MSAA Notes](https://learn.microsoft.com/en-us/windows/win32/winauto/combo-box)。

临时 popup Fragment 和 popup WM_GETOBJECT/UiaRootObjectId 包装均没有解决此
路径，已撤掉，不保留无效 provider。下一步针对 Combo MSAA 子 List 的导航做
窄适配，保留系统模式、nullable COM 输出、身份与销毁边界；不得靠跳过列表
断言或直接取前两个重复项验收。当前失败证据 RTK tee：1792531623（直接 MSAA）、
1792530925（跨进程）、1792531544（系统 HWND 对照）。最近完整通过仍为 v137，
本轮未重新运行全量库门禁，未提交、推送、打包或扩大 capability。

本轮已通过：既有 Select 专项 16 passed，Clippy all-targets -D warnings、
fmt --check、2995 文件严格行数（全部不超过 500）、diff/本轮源码 whitespace。
原生测试进程残留为 0。这些结果不抵消上述 UIA 失败，也不代表 v138 验收通过。

## N08 Select 滚动锚点与弹出生命周期修复

内部版本 v0.2.33.137。真实窗口回归先复现：Select 锚点滚动移动后，系统
COMBOBOX 的 CB_GETDROPPEDSTATE 仍为 1，SetWindowPos 不自动关闭列表。
Mount 现保存常数大小的 origin/size/clip/dpi 快照，仅真实几何变化前显式收起
下拉，成功放置后更新快照；不为每帧复制场景或维护另一份窗口列表。权威 source
变化前也先收起列表，避免旧弹出状态跨越新声明值。无变化同步不强制关闭。

修复后真实 HWND 验证：相同场景重同步仍保持展开；小幅滚动实际移动控件并
收起列表，选择保持；滚出可见区域时列表关闭/控件禁用，滚回后恢复可用且值
不丢失，全程没有合成 change 事件。原失败 0 passed/1 failed，修复后专项通过。
这覆盖滚动与祖先裁剪路径，不代表全部遮挡/DPI/主题或 UIA 选择模式已经验收。
验证：Select 专项 16 passed，全量库 2435 passed/32 ignored（67.38 秒），
startup 7 passed（12.71 秒）。Clippy all-targets -D warnings、fmt --check、
2994 文件严格有效行数（全部不超过 500）、diff/本轮源码 whitespace 检查通过，
原生测试进程无残留。未提交、推送或打包。

## N08 Select 双向 Tab 与下拉离焦清理

内部版本 v0.2.33.136。Select 纳入布局焦点候选，Mount 的 HWND→节点映射和
focus 路由识别当前 Select owner；disabled/hidden 不进入导航，编辑权限和可见性
仍在真实聚焦前复核。独立 `surface_select_messages` 只处理 Tab，不借用 EDIT
组合输入/数字逻辑；收起下拉后检查 owner marker，再复用既有 NAVIGATE 消息。
无 Rust 会话指针跨默认过程，NCDESTROY 移除子类；其他按键交系统 COMBOBOX。

真实 HWND 回归验证 EDIT→Select→自绘 Button 的正向 Tab 和反向 Shift+Tab，
实际展开列表后 Tab 收起，重复 keydown/WM_CHAR 不重复导航，disabled/hidden
Select 跳过且导航不产生 change。测试仅临时设置并恢复本线程键盘状态，不注入
全局输入；自绘 Button 的系统焦点在父 HWND，不把它误当丢失焦点。
主题/字体 DPI、弹出层与滚动、UIA 选择模式及实际服务端验收仍待推进，不扩大能力。
验证：Select 专项 15 passed，全量库 2434 passed/32 ignored（65.85 秒），
startup 7 passed（12.79 秒）。Clippy all-targets -D warnings、fmt --check、
2993 文件严格有效行数（全部不超过 500）、diff/本轮源码 whitespace 检查通过，
原生测试进程无残留。未提交、推送或打包。

## N08 Select 挂载与共享 change FIFO

内部版本 v0.2.33.135。Select 已进入原生测量与有界字段挂载，创建实际下拉
控件并复用字段几何裁剪。COMBOBOX 分配额外下拉高度，关闭部分仍按节点可见
区域裁剪；禁用/不可见撤销待处理意图并收起下拉。绑定变化销毁旧控件，合同
变化在新同名控件创建前移除旧事件，包含 Switch→Select 的跨类型替换边界。

真实 WM_COMMAND/CBN_SELCHANGE 进入共享 32 槽 FIFO，记录用户选择时的字符串
快照；不创建 Draft/ticket、不写按钮表单值。队列满时恢复最后接纳的选择，
避免 UI 值已变但事件丢失；prepare/cancel/settle 沿既有事件预算和提交门禁，
未启动取消后原快照仍先于后续选择。权威 source 变化才同步选择，同值同步不
冲掉本地值；action/options 等合同变更重建控件并清旧意图。

真实 Surface HWND 回归通过方向键驱动系统 COMBOBOX 自动产生通知，检查 wire
change/commit、revision、取消后 FIFO 顺序、权限关闭、合同替换和 32 槽回压。
这是无网络的宿主纵向链；主题/字体 DPI 精确对齐、Tab 导航、弹出层与滚动交互、
UIA 选择模式及实际服务端业务验收仍未完成，不扩大 capability。
验证：Select 专项 13 passed，全量库 2432 passed/32 ignored（65.34 秒），
startup 7 passed（12.78 秒）。Clippy all-targets -D warnings、fmt --check、
2991 文件严格有效行数（全部不超过 500）、diff/本轮源码 whitespace 检查通过，
原生测试进程无残留。未提交、推送或打包。

## N08 Select 真实 COMBOBOX owner

内部版本 v0.2.33.134。新增单线程 `surface_select_control`，消费已验证 Options
快照，以隐藏 CBS_DROPDOWNLIST 创建真实 COMBOBOX，不启用 CBS_SORT；插入
返回索引必须与声明顺序一致。显示标签与 wire value 分离，重复值仍保留独立
索引；声明同步不伪造 CBN_SELCHANGE，读取 CB_ERR 返回无选择，非法索引拒绝。

HWND 使用独立 owner marker，父窗口必须属于当前线程，尺寸采用 checked 算术。
创建中失败由 RAII 销毁已建控件；关闭/marker 撤销后的操作失败，Drop 不销毁
失去归属的 HWND。禁用前主动收起下拉，字体引用共享系统字体、不自行删除。
真实 HWND 测试覆盖 Unicode 标签/顺序/重复项、方向键消息选择、无选择/空列表、
实际展开后禁用收起、再次展开时父窗口关闭连同列表窗口销毁，以及 marker
撤销后不误操作或误销毁窗口。测试仅操作自有窗口，不注入桌面键盘输入。

仍未接入 Surface 测量/挂载、CBN_SELCHANGE 到 live 提交、主题/DPI/滚动裁剪及
UIA 选择模式；这是可运行控件 owner，不是 Select 全功能验收，不扩大 capability。
验证：Select 专项最终源码两次 9 passed（0.51/0.50 秒），全量库
2428 passed/32 ignored（66.39 秒），startup 7 passed（13.01 秒）。Clippy
all-targets -D warnings、fmt --check、2989 文件严格有效行数（全部不超过 500）、
diff/本轮源码 whitespace 检查通过，原生测试进程无残留。未提交、推送或打包。

## N08 Select 声明模型与 change 事件合同

内部版本 v0.2.33.133。定向核对旧前端 DeclarativeSurface.tsx:350–365：Select
仅接受字符串 value/label，options 非数组为空、无效字段回退空字符串；只发送
change 的字符串 payload。它不使用文本 draft，也不写入按钮 includeSurfaceValues
的字段集合，不能直接套用 EDIT 的草稿/表单汇总语义。Slider 是另一条数值
input/change 路径，本轮未实施，不混用两类控件生命周期。

新增 `surface_select` 保存有界声明快照，保留顺序、重复值和标签；声明值未匹配
时返回无索引，不静默换成首项。按索引取值有范围检查，错误输入不返回截断列表。
选项上限 2048、单字符串 48 KiB、总字符串 1 MiB，NUL 明确拒绝；这些是原生
安全边界，不宣称浏览器也有相同限制。所有文本在复制前完成预算/内容验证。
共享 input/change 编码增加 Select 的严格字符串 change/commit，input 不发送，
继续遵守 visible/disabled、binding/revision 和不可变事件预算，不添加表单值。

当前仅完成数据/事件前置层：NativeMeasure 仍拒绝 Select，尚无真实 HWND 下拉、
键盘/指针选择、弹出生命周期和 UIA 选择模式，不扩大 capability，也不宣称
Select 可交互或 N08 完成。下一步将真实控件接入这套合同，而非另建文本 draft。
验证：Select 专项 5 passed，共享字段事件 6 passed；全量库 2424 passed/32 ignored
（65.51 秒），startup 7 passed（12.68 秒）。Clippy all-targets -D warnings、
fmt --check、2987 文件严格有效行数（全部不超过 500）、diff/本轮源码 whitespace
检查通过，原生测试进程无残留。未提交、推送或打包。

## N08 跨进程自绘控件焦点通知

内部版本 v0.2.33.132。父 MTA 经系统焦点订阅接收自启子进程 Switch→Button
的实际焦点变化，核对缓存节点身份和实时 HasKeyboardFocus 的互斥变化；注销
后确认 handler 实际释放，再切焦验证没有后续交付，最后恢复按钮焦点继续动作。
`surface_uia_process_focus` 独立拥有订阅、证据和清理，回调只读缓存、有界传递。

焦点订阅接口本身是全局的，回调首先检查 CachedProcessId，仅匹配当前仍存活
的自启 Child 才读取/保存其节点 ID；不记录其他应用身份，不查询它们的实时属性。
SW_SHOWNOACTIVATE 和 UIA SetAutoSetFocus(false) 保持不抢占用户前台；只证明
本窗口线程控件焦点与通知，不宣称桌面全局前台焦点或 Narrator 已验收。
全部动态挂载组合、Narrator 与其余 N01–N12 退出条件仍需推进。
验证：三条显式跨进程用例最终源码连续三轮通过（6.95/7.09/6.95 秒），全量库
2419 passed/32 ignored（64.50 秒），startup 7 passed（12.47 秒）。Clippy
all-targets -D warnings、fmt --check、2984 文件严格有效行数（全部不超过 500）、
diff/本轮源码 whitespace 检查通过，原生测试进程无残留。未提交、推送或打包。

## N08 跨进程场景替换结构通知

内部版本 v0.2.33.131。父 MTA 完成稳定窗口根的结构订阅后，通过所持 Child
管道请求替换；子进程 UI owner 执行真实 SurfaceWindow.replace。父进程收到
ChildrenInvalidated，核对缓存根名，再从同一根重读唯一 replacement 原生 EDIT、
New tree 标签和 updated 实际 Value；旧 input/switch/button 不再出现在树中，
保留的旧 Toggle/Invoke 不能继续操作。注销后确认结构 handler 实际释放。

`surface_uia_process_structure` 独立拥有订阅/证据/释放检查；fixture 管道增加
replace 命令，命令输入限制总 64 字节、最多 4 条、队列容量 1；只允许动作收据
成立后替换一次，关闭仍由子进程 UI owner 执行，不向缓存 HWND 发消息。
这是跨进程场景替换和原生 EDIT 重读证据，不把 HWND 复用或全部动态挂载组合
视为已验收。跨进程焦点、Narrator 及其余 N01–N12 退出条件仍需推进。
验证：三条显式跨进程用例最终源码连续三轮通过（6.04/5.96/5.96 秒），全量库
2419 passed/32 ignored（65.86 秒），startup 7 passed（12.26 秒）。Clippy
all-targets -D warnings、fmt --check、2983 文件严格有效行数（全部不超过 500）、
diff/本轮源码 whitespace 检查通过，原生测试进程无残留。未提交、推送或打包。

## N08 活跃订阅穿过进程退出与重启隔离

内部版本 v0.2.33.130。新增显式跨进程用例，实际收到旧实例 ToggleState/Invoked
后不注销订阅，终止所持 Child，并保留旧 element/pattern/handler。启动同场景
新实例，为其独立注册订阅，验证新实例操作和新订阅事件均可用；旧模式仍失效，
旧订阅在新实例通知交付后保持安静，不能按同名 AutomationId 串接到新实例。
随后对已终止实例执行注销并确认两个 handler 实际释放，新实例订阅也独立注销
释放，最后正常关闭新实例。无全局事件注册、进程扫描或远端业务请求。

原有“先注销再退出”两条路径仍保留，新用例仅增加“活跃订阅时强制退出”证据。
安静检查要求有界等待超时而非 channel 断开，避免把 handler 意外丢失当隔离成功。
不要求 PID/HWND 永不复用；这不是应用业务崩溃恢复或 Narrator 全流程验收。
跨进程焦点/结构通知和其余 N01–N12 退出条件仍需推进。
验证：三条显式跨进程用例最终源码连续三轮通过（5.45/5.36/5.28 秒），全量库
2419 passed/32 ignored（65.83 秒），startup 7 passed（13.06 秒）。Clippy
all-targets -D warnings、fmt --check、2982 文件严格有效行数（全部不超过 500）、
diff/本轮源码 whitespace 检查通过，原生测试进程无残留。未提交、推送或打包。

## N08 跨进程 Toggle 与 Invoked 事件订阅

内部版本 v0.2.33.129。既有正常/强制退出跨进程测试在父 MTA 注册系统 UIA
订阅，子进程中的 Toggle 与 Invoke 实际操作后，父进程分别收到 ToggleState
属性载荷和 Invoked 事件，并核对缓存 AutomationId。回调不查询实时 UI、不访问
子进程 State、不阻塞，证据经容量 8 的 channel 交回测试线程；允许系统重复交付，
但收到的重复项也必须具有正确身份与载荷。

独立 `surface_uia_process_events` 拥有注册/注销及 COM handler 生命周期；第二项
注册失败时第一项也由 owner 清理。注销后等待两个 handler 实际析构，再进入
正常/强制退出流程，不把 Remove 成功返回误当实际释放。仅订阅自启子进程的
具体 Switch/Button，不注册全局事件或扫描其他应用。
本轮不涵盖仍订阅时进程崩溃、跨进程焦点/结构通知或 Narrator 全流程验收。
验证：两条显式跨进程用例在最终源码连续三轮通过（2.99/2.84/2.89 秒），
全量库 2419 passed/31 ignored（69.22 秒），startup 7 passed（13.87 秒）。
Clippy all-targets -D warnings、fmt --check、2982 文件严格有效行数（全部不超过
500）、diff/本轮源码 whitespace 检查通过，原生测试进程无残留。未提交、推送或打包。

## N08 跨进程强制退出与新实例隔离

内部版本 v0.2.33.128。显式跨进程测试增加强制退出路径：真实 Value 写入、
Toggle 与 Invoke 收据成立后，仅终止持有的测试 Child，等待并检查非成功退出。
保留旧 UIA 模式引用，启动相同场景的新实例，确认新实例值从 original/Off
开始，并能重新写入、切换和 Invoke。随后对旧引用再次读写/调用均必须失败，
且新实例的实际值保持 external/On，最后通过管道正常关闭新实例。

终止仅依赖自启 Child 的进程句柄，不按名称/PID 扫描，不向旧 HWND 发消息；
正常和强制退出复用带期限的退出等待和 stdout reader 回收。测试不要求 PID 或
HWND 永不复用，也不把异常退出测试误称为完整系统崩溃恢复或 Narrator 验收。
运行 `uia_external_process_ -- --ignored --test-threads=1` 可覆盖正常/强制两条路径。
跨进程事件订阅、Narrator 和其余 N01–N12 退出条件仍需推进。
最终源码两条显式用例连续三轮全部通过（2.62/2.64/2.56 秒）；全量库
2419 passed/31 ignored（73.62 秒），startup 7 passed（16.00 秒）。Clippy
all-targets -D warnings、fmt --check、2981 文件严格有效行数（全部不超过 500）、
diff/本轮源码 whitespace 检查通过，原生测试进程无残留。未提交、推送或打包。

## N08 独立进程 UIA 操作与退出撤销

内部版本 v0.2.33.127。新增显式桌面集成测试：父 MTA 通过系统 UIA 访问独立
STA 子进程的 Surface，核对进程身份、Otter.Native 根、唯一节点和 EDIT 逻辑
父级；实际执行 Value 写后读、Switch Off→On、Button 聚焦及 Invoke。子进程
独立核对 Automation 来源、表单值 external、待处理编辑及 Switch 实际状态。
正常退出后，保留的 Value/Toggle/Invoke 引用均返回错误，不能继续操作旧进程。

fixture 仅启动当前测试二进制的指定子测试，管道读取限 64 KiB、收据队列容量
4，并设置启动/动作/关闭期限。正常关闭走持有 Child 的 stdin，不向可能被复用
的缓存 HWND 发消息；失败仅清理持有的子进程，不扫描或操作其他应用。关闭读取
线程只在已收到命令时 join，意外关窗不等待尚未完成的 stdin 读取。

显式运行命令（默认全量库测试不会启动这个桌面子进程）：

```powershell
rtk cargo test --manifest-path native/Cargo.toml --locked --offline --lib uia_external_process_tree_edit_toggle_invoke_and_exit_revoke -- --ignored --test-threads=1
```

最终源码连续三次通过（0.92/0.84/0.92 秒）。本轮不包含跨进程事件订阅、强制
退出、Narrator、全局前台焦点或全部 N01–N12 验收；不扩大 capability 广告。
常规 UIA 39 passed/2 ignored；全量库 2419 passed/30 ignored（76.17 秒），
startup 7 passed（16.85 秒）。新增的两项 ignored 是上述显式父测试和专用子入口。
Clippy all-targets -D warnings、fmt --check、2981 文件严格有效行数（全部不超过
500）、diff/本轮源码 whitespace 检查通过，原生测试进程无残留。
未提交、推送或打包。

## N08 同场景 EDIT 挂载结构通知

内部版本 v0.2.33.126。Mount 在实际 fields 清理、retain 删除、零尺寸移除和成功
插入时置 fields_changed；普通值/几何同步不置位。State.run 无论 operation 返回
Ok 或 Err 都提取已发生的变化，在恢复 Host 后复用稳定根 ChildrenInvalidated
出口；关闭后不排队。提取是常数时间，不在每次 Host 操作扫描字段或复制 HWND 列表。

回归验证晚绑定首次挂载、同场景不同 binding 的重建、相同 binding 和纯同步无
多余事件、drain 不重复报告，以及新 HWND 的当前 override 映射。HWND 数值允许
复用，测试不把“永不复用旧数值”当合同；仅对已非当前控件的句柄断言 NULL。
真实 MTA 在未挂载 Input 场景订阅，UI owner 只调用 bind_edits、不 replace，随后
收到根 ChildrenInvalidated 并读取唯一新节点和原生 ValuePattern 的实际值。

这补齐 Mount 结构变化与已接入 UIA 树之间的通知接线；不宣称跨进程/Narrator、
全部 DPI/裁剪/重建组合或整个 N01–N12 验收完成。
验证：UIA 39 passed；全量库 2419 passed/28 ignored（76.95 秒），startup
7 passed（18.46 秒）。
Clippy all-targets -D warnings、fmt --check、2979 文件严格有效行数（全部不超过
500）、diff/本轮源码 whitespace 检查通过，原生测试进程无残留。
未提交、推送、打包或扩大 capability 广告。

## N08 原生 EDIT 的 HWND 重定位与系统模式保留

内部版本 v0.2.33.125。根实现 IRawElementProviderHwndOverride，通过当前 Mount
将 HWND 映射到声明式节点，未知/未挂载 HWND 返回 S_OK + NULL，不缓存裸 HWND。
挂载 Input/Textarea/Number 进入 Fragment 导航；HostRawElementProvider 返回系统
EDIT host，GetRuntimeId 使用 HWND 身份的 NULL 合同，不再与自绘节点混用追加 ID。
仅补 AutomationId、FrameworkId 和显式标签，其他属性及 Value/Text 模式交给系统。
参考：[HWND override 合同](https://learn.microsoft.com/en-us/windows/win32/api/uiautomationcore/nf-uiautomationcore-irawelementproviderhwndoverride-getoverrideproviderforhwnd)。

UIA EDIT SetFocus 沿既有显露/权限/真实控件 owner 执行，系统调用后重新核对权限、
场景身份、当前 HWND 与实际焦点。根 GetFocus 和命中返回本 Fragment 的承载节点；
焦点事件仍由系统 EDIT 发出，不重复走自绘通知出口。修正 Control::focus 对
SetFocus 返回 NULL 的误判：没有先前焦点时返回 NULL 不代表失败，按实际焦点复核。

真实 MTA 检查三类控件各只有一个节点、正确逻辑父容器、系统 Value 模式，验证
Input 的 Text 模式、SetFocus 和 Value 写后读。实测多行 EDIT 的系统 ControlType
是 Document，单行是 Edit；测试遵循真实类型，不通过伪造属性消除差异。
ABI 回归覆盖共享 IUnknown、未知 HWND/null out、未挂载排除、场景撤销、根焦点
及关闭。更广的同场景挂载重建、结构通知来源、跨进程/Narrator 仍需继续验收。
验证：UIA 37 passed；全量库 2417 passed/28 ignored（72.03 秒），startup
7 passed（16.46 秒）。
Clippy all-targets -D warnings、fmt --check、2978 文件严格有效行数（全部不超过
500）、diff/本轮源码 whitespace 检查通过，原生测试进程无残留。
未提交、推送、打包或扩大 capability 广告。

## N08 场景子树结构失效通知

内部版本 v0.2.33.124。场景替换撤销旧语义 Owner 时，在稳定 HWND 根排队
ChildrenInvalidated；客户端需要重新查询当前子树。即使新场景替换失败，旧身份
已撤销也必须通知，不把旧缓存继续当有效节点。成功路径在事务完成后才发送；
连续替换合并，Host busy 等外层 run 恢复，无每节点删除列表或无界事件队列。

结构通知独立 owner 保存 dirty/posted/flushing；回调重入替换排到下一轮，Raise
不持 RefCell/Host 借用，关闭后残留消息不发送。ChildrenInvalidated 不传子节点
runtime-id，遵守 [UiaRaiseStructureChangedEvent 合同](https://learn.microsoft.com/en-us/windows/win32/api/uiautomationcoreapi/nf-uiautomationcoreapi-uiaraisestructurechangedevent)。

回归覆盖连续替换、旧身份撤销、busy 重排、重入替换、失败替换与关闭；独立 MTA
订阅根结构事件后，由 UI owner 替换场景，再验证缓存根身份、ChildrenInvalidated
类型及重新读取的新节点。订阅 RAII 同线程注销并确认 handler 释放。
本项覆盖整场景替换，不等同 EDIT 精确重定位、动态可见性变化、跨进程或 Narrator
验收；其他控件与 N01–N12 退出条件继续。
验证：UIA 34 passed；全量库 2414 passed/28 ignored（66.81 秒），startup
7 passed（15.04 秒）。
Clippy all-targets -D warnings、fmt --check、2974 文件严格有效行数（全部不超过
500）、diff/本轮源码 whitespace 检查通过，原生测试进程无残留。
未提交、推送、打包或扩大 capability 广告。

## N08 焦点变化通知与真实 MTA 订阅

内部版本 v0.2.33.123。焦点通知观察既有 HWND/键盘 owner，不维护第二套业务焦点。
UIA SetFocus、Tab、鼠标逻辑焦点、失焦、场景 reconcile 和权限失效使用独立私有
消息合并发送 AutomationFocusChanged。当前节点按 runtime-id 去重，失焦清除已报
身份，同 ID 场景替换仍报告新身份；系统 EDIT 不被伪装成声明式节点。

Host busy 时保留 dirty，外层 run 恢复 Host 后重排；flushing RAII 防止通知重入，
先保存本次身份再调用 UIA，回调新变化只排下一轮。所有 Host/RefCell 借用均在
Raise 前释放，关闭后的残留消息不发送。每个窗口只有一个待排标记和一个已报身份。
参考：[服务端 UIA provider 事件合同](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-serversideprovider#events-from-providers)。

owner 回归覆盖合并、busy 重排、重复抑制、失焦后重获焦、重入切焦、同 ID 替换及
关闭。真实 MTA FocusChanged 订阅过滤本测试节点，核对 UIA SetFocus、Tab 和鼠标
消息产生的缓存身份及目标节点实时 HasKeyboardFocus；注册/注销同线程，确认
handler 引用释放，解除订阅后无旧回调。提交暂停下不产生 click。

测试使用 SW_SHOWNOACTIVATE、不抢桌面前台，不能用全局 GetFocusedElement 断言
本 HWND 线程焦点；这里验证窗口线程和 UIA 节点合同，不宣称完成跨进程、前台切换
或 Narrator 验收。结构通知、EDIT 精确重定位、其他控件和 N01–N12 退出条件继续。
验证：UIA 30 passed；全量库 2410 passed/28 ignored（66.70 秒），startup
7 passed（16.04 秒）。Clippy all-targets -D warnings、fmt --check、2971 文件
严格有效行数（全部不超过 500）、diff/本轮源码 whitespace 检查通过，原生测试
进程无残留。未提交、推送、打包或扩大 capability 广告。

## N08 Switch 状态通知统一出口

内部版本 v0.2.33.122。共享 Switch Session 记录 toggle/synchronize/settle 的实际
值变化，Mount 只在变值时设置 dirty。State.run 提取变化、恢复 Host 后通过私有
PostMessage 排队；不在 UIA Toggle 内同步 Raise，不打断鼠标/键盘/自动化祖先
click 的事务。当前身份每节点至多一条待发记录，净零变化合并，场景替换/关闭清理。

通知发送不持 Host/RefCell 借用。flushing RAII 阻止嵌套发送；回调改变后续节点时
复查实时值并跳过旧批次，退出后重新排队。Host busy 保留变化，外层 run 完成后
重排。通知可合并过渡值，不是动作 FIFO，也不表示远端提交完成。

新增回归覆盖合并、busy、重入、身份撤销、结算变值及关闭；真实 MTA property
订阅同时验证 UIA Toggle、自有 HWND 鼠标消息、键盘 Space 的 On/Off 交付。
不注入系统输入，不把窗口消息测试视作真人或 Narrator 验收。焦点/结构通知、
EDIT 重定位、跨进程/Narrator、其他控件与 N01–N12 退出条件仍未完成。

验证：UIA 27 passed；全量库 2407 passed/28 ignored（69.25 秒），startup
7 passed（15.52 秒）。Clippy all-targets -D warnings、fmt --check、2968 文件
严格有效行数（全部不超过 500）、diff/本轮源码 whitespace 检查通过，原生测试
进程无残留。未提交、推送、打包或扩大 capability 广告。

## N08 ToggleState property 通知真实接收

内部版本 v0.2.33.121。新增独立 MTA 的 IUIAutomationPropertyChangedEventHandler
订阅回归，只订阅目标 Switch 的 ToggleToggleState。回调从 cache 读取 AutomationId，
从事件 VARIANT 解码 i32 新值，经有界非阻塞通道传回测试；不持 sender，不同步
访问 UI State。真实 TogglePattern 产生 On/Off，事件值与随后查询的实时状态一致。
参考：[UIA property 事件示例](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-howto-implement-event-handlers)。

测试在 submitting 暂停但 local editing 有效时运行，证明本地状态通知不依赖远端
提交。临时禁用本测试 HWND 后，Toggle 被拒绝，值保持 Off 且测试发送计数不增加。
生产没有新增计数或 telemetry；计数只用于区分一次发送与系统重复交付。
同一 MTA 注册/注销，RAII 保障错误路径清理；解除订阅并释放持有引用后确认 handler
已销毁，再次 Toggle 仍成功但不会向旧订阅交付。没有注入桌面输入或发远端动作。

这补齐的是 UIA Toggle 引发的 property 通知交付证据。物理鼠标/键盘、权威场景
同步及结算后的统一状态变化通知仍未接通；焦点通知、EDIT 重定位、跨进程/Narrator
和其他迁移任务继续，不能据此宣称完整 Switch 无障碍合同完成。

验证：UIA 24 passed，连续三轮全部通过；全量库 2404 passed/28 ignored（65.80 秒），
startup 7 passed（16.45 秒）。Clippy all-targets -D warnings、fmt --check、2966 文件
严格有效行数（全部不超过 500）、diff/本轮源码 whitespace 检查通过，原生测试
进程无残留。未提交、推送、打包或扩大 capability 广告。

## N08 Switch Toggle 模式与实时本地状态

内部版本 v0.2.33.120。Switch 暴露 IToggleProvider、IsTogglePatternAvailable 与
ToggleToggleState；Button/根不开放此模式，Switch 仍不开放 Invoke。Toggle 的独立
接口头共享节点 IUnknown/refcount，支持接口集合固定，失效节点方法拒绝操作。
ToggleState 读取当前挂载的 Switch 编辑 Session，不使用旧 props.value 伪装实时值；
未建立编辑挂载/busy/已撤销会话返回不可用。二态按 Off/On 切换，不引入第三态。
参考：[IToggleProvider](https://learn.microsoft.com/en-us/windows/win32/api/uiautomationcore/nn-uiautomationcore-itoggleprovider)。

操作复用 change FIFO 与本地 editing 权限：仅暂停 submitting 仍可切换本地值，
但不能提交或创建祖先 click 候选。窗口隐藏/disabled、控件 disabled、本地撤权、
已有 pending/鼠标 press、旧句柄均拒绝；队列满在修改值前失败。
先显露和同步 EDIT，之后重新核验场景、权限和 pointer identity。实际提交方法
不再调用 Win32，避免检查与修改之间发生新的窗口重入。

有祖先 click 且允许提交时，Toggle 前捕获表单值与 fence，Toggle 后将祖先 click
排入原 Automation 单槽。该 fence 不包含自己的 change，保留 click-before-change
既有语义，不造成自依赖死锁。Toggle 自身不抢焦点，也不发送远端动作。

Toggle 成功后发送 old/new ToggleState property-changed，通知时不持 Host/RefCell
借用，通知失败不回滚或重复切换。当前只接通本次 UIA Toggle 引发的通知；物理
鼠标/键盘/权威同步状态变化的统一通知及实际 property 订阅接收尚待后续完成，
不能把查询/切换通过等同完整通知合同或整个 N08 完成。

专项覆盖真实 COM 状态/身份、提交软停止、本地撤权、busy、disabled、替换/关闭，
32 项 change 队列满时不变值，以及祖先 click 的前缀 fence。独立 MTA 客户端在
真实绑定编辑挂载后通过 IUIAutomationTogglePattern 读取 Off、切换 On 再切回 Off。

验证：UIA 23 passed；全量库 2403 passed/28 ignored（64.03 秒），startup 7 passed
（14.84 秒）。Clippy all-targets -D warnings、fmt --check、2965 文件严格有效行数
（全部不超过 500）、diff/本轮源码 whitespace 检查通过，原生测试进程无残留。
未提交、推送、打包或扩大 capability 广告。

## N08 Invoked 通知真实订阅交付

内部版本 v0.2.33.119。新增 MTA 事件订阅回归：以 IUIAutomationEventHandler
订阅指定 Button 的 Invoked，实际经 IUIAutomationInvokePattern 激活，回调读取
订阅 cache 中的 AutomationId/Name，不持 sender、不访问 UI State，也不在回调
中同步查询 provider。回调证据进入容量 4 的非阻塞 channel，订阅 RAII 在成功、
错误和退出路径均解除 handler。注册/注销在同一 MTA 线程，晚到回调遵循 COM
引用计数；验证对象已释放后再激活同一节点，不再收到该订阅的通知。
参考：[UIA 事件订阅](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-eventsforclients)。

实际观察到首次 Invoke 后、第二次 Invoke 前，系统会重复交付相同通知。因此不把
回调次数作为动作次数：仅测试构建的发送计数证明一次接纳只调用一次 raise，队列
已占用的拒绝调用不会再 raise；每个收到的副本均核对相同事件/节点/名称。
生产路径没有新增重试或重复通知修补。系统提供 CoalesceEvents 去重选项，当前
回归保留默认交付语义，而非通过开启去重隐藏观察结果。
参考：[CoalesceEventsOptions](https://learn.microsoft.com/en-us/windows/win32/api/uiautomationclient/ne-uiautomationclient-coalesceeventsoptions)。

事件专项连续三轮通过。联合 UIA 测试暴露旧 Invoke 测试对 AutoSetFocus 的过强
断言：消息泵结束后焦点可能已不在目标，不能把持续持焦当作 Invoke 输出合同。
现验证 true/false 均可生成正确候选，false 不抢焦点；显式 SetFocus 的实时焦点
合同仍由独立测试覆盖，没有删去该合同验证，也未更改生产焦点逻辑。

本轮补齐的是 Invoked 通知的同进程真实系统交付证据，不是跨进程/Narrator 或
所有 UIA 通知完成。焦点通知、Toggle 等模式、EDIT 重定位和其余迁移继续。

最终验证：UIA 19 passed，额外连续三轮均通过；全量库 2399 passed/28 ignored
（60.41 秒），startup 7 passed（12.15 秒）。Clippy all-targets -D warnings、
fmt --check、2961 文件严格行数（全部不超过 500）、diff/本轮源码 whitespace
检查通过，原生测试进程无残留。未提交、推送、打包或扩大 capability 广告。

## N08 UIA Button Invoke 与有界动作候选

内部版本 v0.2.33.118。声明式 Button 暴露 IInvokeProvider 和
IsInvokePatternAvailable；根、Text、Switch 不伪装成 Invoke。独立 Invoke 接口头
共用原节点 IUnknown/refcount，接口集合在构造时固定；场景撤销后 QI 身份不变，
Invoke 方法返回 ELEMENTNOTAVAILABLE。GetPatternProvider 未支持模式仍 S_OK/NULL。

Invoke 不同步发送动作或等待远端：校验场景、本地编辑、提交许可、disabled、窗口
可见/enabled 后，显露目标并捕获表单值/FIFO fence，复核身份再放入原单槽候选。
Activation 区分 Pointer/Keyboard/Automation，后者按节点准备但不要求键盘焦点；
既有 binding/revision/合同/依赖结算继续在消费链验证，不绕过授权。
参考：[IInvokeProvider::Invoke](https://learn.microsoft.com/en-us/windows/win32/api/uiautomationcore/nf-uiautomationcore-iinvokeprovider-invoke)。

有待处理候选或未结束的鼠标 press 时拒绝 Invoke，避免后续 mouse-up 静默覆盖
已接受请求。Host 同步可重入，前后检查 pointer identity，撤权后重新授权也不能
复活旧边沿。provider 不调用 SetFocus；系统客户端默认 AutoSetFocus 会先调用焦点
路径，测试同时覆盖 true/false，已有焦点变化撤销规则仍适用。
参考：[AutoSetFocus](https://learn.microsoft.com/en-us/windows/win32/api/uiautomationclient/nf-uiautomationclient-iuiautomation2-get_autosetfocus)。

接纳后若存在 UIA listener 则发送 Invoked 通知；因业务可能需要远端响应/用户确认，
该通知表示接受激活请求，不表示远端动作成功。通知时不持 State/RefCell 借用，失败
不重放已入槽请求。客户端事件订阅接收的专项验收尚未完成，不能将调用发送 API
等同通知交付已验证；焦点通知、Toggle 等其他模式、EDIT 重定位仍继续。

专项覆盖 COM 身份、模式边界、重复入槽/鼠标手势互斥、软停止/撤权、busy、隐藏、
disabled、同 ID 替换和关闭，直接/延迟动作消费，以及真实 EDIT 草稿冻结与前置
编辑 fence。独立 MTA 客户端实际取得 IUIAutomationInvokePattern 并执行 Invoke。
UIA 专项 18 passed；未提交、推送、打包或扩大 capability 广告。

全量验证：2398 passed/28 ignored（65.77 秒），startup 7 passed（14.60 秒）；
Clippy all-targets -D warnings、fmt --check、2960 文件严格有效行数检查（全部
不超过 500）、git diff --check 和本轮源码 whitespace 检查通过。测试进程无残留。

## N08 UIA 焦点查询与设置

内部版本 v0.2.33.117。窗口根声明 ProviderOptions_ProviderOwnsSetFocus，使 UIA
不在 provider 权限检查前自动设置 HWND 焦点；声明式 Button/Switch 的 SetFocus
复用键盘 owner、候选撤销与滚动显露，不创建第二套焦点状态，也不产生 click/change。
参考：[SetFocus](https://learn.microsoft.com/en-us/windows/win32/api/uiautomationcore/nf-uiautomationcore-irawelementproviderfragment-setfocus)。

本地编辑关闭、窗口隐藏/最小化/disabled、控件 disabled 均不能借 UIA 聚焦；
仅暂停提交仍允许本地聚焦。capture 释放、Host 查询、Win32 SetFocus 可能重入，
因此前后复核场景完整身份、closed、本地权限和真实 HWND 焦点。busy 或同 ID 替换
拒绝继续；属性在 Host 查询后读取当前权限，避免返回查询前的旧 enabled 状态。

GetFocus 与 HasKeyboardFocus 共用 HWND + 现有逻辑节点判定，无焦点返回 S_OK/NULL；
原生 EDIT 拿到 HWND 焦点时不伪装为自绘节点。IsKeyboardFocusable 包括可显露的
屏外候选，但零面积/disabled 不包含在内。Tab 也复用同一 reveal_focus 路径。
参考：[GetFocus](https://learn.microsoft.com/en-us/windows/win32/api/uiautomationcore/nf-uiautomationcore-irawelementproviderfragmentroot-getfocus)。

专项涵盖根/子焦点一致性、提交软停止、本地撤权、隐藏、disabled、busy、同 ID
替换和关闭，屏外 Switch 显露且不改变值/不入动作队列；独立 MTA 系统 UIA 客户端
实际调用 Button.SetFocus 并读取焦点属性。Switch 自身不接受 layout 的旧兼容语义
保持不变，屏外夹具使用固定高度容器，不能以零面积夹具冒充可聚焦控件。

这不是完整无障碍验收：焦点通知、操作模式、EDIT 重定位、跨进程/Narrator 和
混合 DPI 人工验收仍未完成。未提交、推送、打包或扩展 capability 广告。

验证：UIA 专项 13 passed；全量库 2393 passed/28 ignored（63.91 秒），startup
7 passed（15.25 秒）。Clippy all-targets -D warnings、fmt --check、2957 文件严格
行数检查（全部不超过 500 有效行）、git diff --check 和本轮源码 whitespace
检查通过。运行产物保留在 linshi；未将忽略测试或人工验收计为通过。

## N08 UIA 实时屏幕几何与点命中

内部版本 v0.2.33.116。子 Fragment BoundingRectangle 和 IsOffscreen 已连接当前
Host：查询先同步 viewport，读取滚动投影后的几何，按 HWND DPI 和客户区屏幕
原点转换坐标，不创建 D2D target、不等待下一次 WM_PAINT，也不缓存屏幕矩形。
BoundingRectangle 保留完整尺寸；祖先裁剪以及窗口隐藏/最小化状态单独决定
IsOffscreen，不把部分裁剪后的面积当完整 bounds。
参考：[BoundingRectangle](https://learn.microsoft.com/en-us/windows/win32/api/uiautomationcore/nf-uiautomationcore-irawelementproviderfragment-get_boundingrectangle)。

根 ElementProviderFromPoint 使用同一实时 Presentation 命中逻辑，拒绝非有限
坐标；隐藏/最小化/窗外点返回 NULL，窗口内空白返回根，声明式节点返回当前
快照句柄。空树可正常返回 NULL；Input/Textarea/Number 不制造重复声明式命中，
继续留给系统 EDIT provider，HwndOverride 精确重定位仍待完成。

窗口 State 从 Box 转为 Rc，Win32 userdata 仍指向稳定 State 数据地址；窗口过程
对每次回调独立保活。UI 线程 TLS 注册表仅保存 Weak<State>，COM 不持 Rc 或 State
指针。查询先升级 Weak 并释放表借用，进入 Host 前后检查 closed 和完整 runtime-id；
busy、同 ID 替换、重入关闭都拒绝返回旧结果。stop 先注销，并核对 State 数据地址，
防止旧 owner 的重复清理误删复用 HWND 的新项。注册发生在 Host 成功绑定后。

新增实际窗口测试覆盖移动、滚动、未裁剪尺寸、可见性、点命中、busy/外来/旧快照、
重入关闭不恢复 Host、Rc 释放和 registry 身份保护；可见测试短暂使用
SW_SHOWNOACTIVATE，不注入系统输入。COM vtable 验证 bounds/offscreen/命中及
关闭撤销，系统 UIA MTA 客户端也实际读取子节点 BoundingRectangle/IsOffscreen。
生产 desktop 初始化 PMv2；这些本机测试不是混合 DPI/跨显示器人工验收。

最终 UIA 针对性 10 passed，快照/窗口针对性 14 passed，无 otter-native 残留进程。
全量库 2390 passed/28 ignored（64.03s），startup 7 passed（14.21s）；
Clippy all-targets -D warnings、fmt、2955 文件严格行数、diff/源码 whitespace
检查通过，所有文件均未超过 500 有效行。只读审查未发现本轮 Rc/registry、
实时几何和命中范围的阻塞缺陷，不把它扩大为混合 DPI 人工验收。

GetFocus/SetFocus、操作模式和动态通知仍未完成，不扩大正式能力报告，未提交
推送、未打包。以下为此前检查点。

## N08 UIA 声明式 Fragment 只读导航

内部版本 v0.2.33.115。窗口根现在同时提供 Simple、Fragment、FragmentRoot 三个
独立接口头，共享规范 IUnknown 身份与引用计数。声明式节点提供 Simple/Fragment，
支持父、首末子、前后兄弟导航；不存在目标时 S_OK + NULL。节点的 FragmentRoot
返回原窗口根，非 HWND 节点 HostRawElementProvider 返回 NULL。窗口根只导航到
声明式树根，外部父/兄弟关系继续由 HWND host 负责。

快照新增不回绕的 u64 身份和节点索引；子节点 runtime-id 是 VT_I4 SAFEARRAY，
包含 UiaAppendRuntimeId、身份高低位和索引。每次独立分配，填充失败销毁，成功
转移给调用者；HWND 根返回 NULL。相同节点多次导航得到相同 runtime-id，同 ID
场景替换不复用身份。子 provider 仅强持根接口和 Weak 快照句柄，根不持子 provider，
没有引用闭环或 State/Host 裸指针；关闭及替换令旧子接口 unavailable。
参考：[GetRuntimeId](https://learn.microsoft.com/en-us/windows/win32/api/uiautomationcore/nf-uiautomationcore-irawelementproviderfragment-getruntimeid)、
[原生 provider 接口与导航](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-serversideprovider)。

语义名称优先显式 accessibility.label，Text/Button 缺省时使用实际可见 text/label，
不复制输入值。系统 UIA MTA 客户端经 FindFirst/TreeWalker 已读取子节点名称和
AutomationId，并在同 ID 替换后读取新树。Input/Textarea/Number 不再额外生成
声明式输入副本，继续由已挂载的系统 EDIT provider 表达；尚无 HwndOverride 精确
重定位，其层级不保证与声明式父节点一致，未挂载的 EDIT 也不凭空声称可访问。

本轮完成的是只读导航，不是完整 Fragment 合同：子节点 BoundingRectangle、根
ElementProviderFromPoint/GetFocus 仍返回 E_NOTIMPL；SetFocus 返回不支持。只有
HWND 根提供真实 GetWindowRect 屏幕矩形。操作模式、动态值/焦点/结构通知、原生
EDIT 重定位、跨进程与 Narrator 验收继续，不使用零矩形或根对象冒充未知命中。
最终 UIA 针对性 8 passed，快照/窗口生命周期针对性 9 passed；全量库
2383 passed/28 ignored（58.28s），startup 7 passed（13.15s）。Clippy all-targets
-D warnings、fmt、2951 文件严格行数、diff/源码 whitespace 检查通过，所有文件
均未超过 500 有效行；结束时无 otter-native 残留进程。只读审查未发现导航/生命周期
范围的阻塞性缺陷，不把明确未实现的方法计为完成。
未提交推送、未打包，正式能力报告不变。以下为此前检查点。

## N08 真实 UIA 窗口根 provider

内部版本 v0.2.33.114。SurfaceWindow 现在处理 UiaRootObjectId/WM_GETOBJECT，
按需创建唯一窗口级 IRawElementProviderSimple；它不随声明式场景替换而换身份。
只提供 Otter Surface 名称、Otter.Native framework 和 Pane 类型，未知属性返回
VT_EMPTY 交给 HWND host；所有未支持模式严格返回 S_OK + NULL，不伪造动作能力。
Rust windows 0.60 的生成 trait 不能表示成功的空接口，因此窄 ABI owner 明确处理
IUnknown/引用计数/输出指针，不构造空的 Rust COM 接口、不保存 State/Host 指针。

创建限定 STA/MAINSTA，声明 UseComThreading；功能调用检查创建线程和 alive。
根 owner 释放时先撤销，保留的 COM 接口功能调用返回 UIA_E_ELEMENTNOTAVAILABLE。
系统 host-provider 查询后再次检查撤销；UiaReturnRawElementProvider 调用不跨越
RefCell 借用。WM_DESTROY 和清掉 userdata 的 Drop 路径都执行官方 null-provider
清理入口，不在同步 SendMessage 路径调用 UiaDisconnectProvider。
参考：[UiaReturnRawElementProvider](https://learn.microsoft.com/en-us/windows/win32/api/uiautomationcoreapi/nf-uiautomationcoreapi-uiareturnrawelementprovider)、
[ProviderOptions](https://learn.microsoft.com/en-us/windows/win32/api/uiautomationcore/ne-uiautomationcore-provideroptions)。

针对性 5 passed：真实 vtable 的 NULL 模式、QI 身份/不支持接口、根属性/host、
引用释放、错误线程、WM_GETOBJECT 复用、场景替换与三种销毁路径。还通过独立
MTA 线程创建系统 UI Automation 客户端，在 STA 消息泵配合下读取真实隐藏 HWND，
验证名称/framework/control type 确实来自 provider，而非仅直接调用 Rust 对象。
此证据是同进程跨 apartment 的 UIA 集成，不是跨进程、Narrator 或人工验收。

最终全量库 2378 passed/28 ignored（63.16s），startup 7 passed（15.45s）；
Clippy all-targets -D warnings、fmt、2948 文件严格行数、diff/源码 whitespace
检查通过，所有文件均未超过 500 有效行。只读交叉审查未发现阻塞性 ABI/生命周期
缺陷。初轮 Clippy 的 SDK 常量命名和测试哨兵指针告警已修正后重新执行全部门禁。

声明式快照尚未接入 Fragment 导航、几何、焦点、Invoke/Toggle/Value 模式或
事件通知；这不是完整 UIA/控件对齐完成。未提交推送、未打包，正式能力报告不变。
以下为此前检查点。

## N08 UIA 前置语义快照生命周期

内部版本 v0.2.33.113。新增 `surface_accessibility` 只读语义快照：仅复制可见
节点的 ID、Kind、显式字符串 label 和父索引，不复制输入值、事件或任意 props。
过滤遵循 Scene visible/visible_children（含 icon 子树规则），不代表已经计算
UIA IsOffscreen、裁剪、屏幕坐标或完整可访问性树。Scene 的节点/深度/字节上界
限制快照规模；句柄只持 Weak，不因客户端保留句柄而保留整个场景。

SurfaceWindow 创建时持有快照 owner，替换前撤销旧 owner，成功且窗口仍有效才
安装新快照；失败保持不可用。close、Drop、WM_DESTROY/WM_NCDESTROY 共同撤销。
同 ID 重建使用全新 Arc 身份，不使用可能回绕的 generation。跨线程只能读取
静态描述，不接触 UI RefCell/Host/HWND，也不能据此执行动作或证明当前编辑值。
与撤销并发且已开始的读取可完成，撤销后开始的读取失败；已返回副本不被追回。

本轮只是接入真实窗口生命周期的 UIA 前置层，尚未实现 COM provider、WM_GETOBJECT、
Fragment 导航、Invoke/Toggle/Value 模式、事件通知或屏幕阅读器验收。尤其不能在
同步 SendMessage 路径盲目调用 UiaDisconnectProvider，也不能在断连重入时重新
返回同一 provider；后续 COM 层须遵守 Microsoft 的线程/断连合同。
参考：[UiaDisconnectProvider](https://learn.microsoft.com/en-us/windows/win32/api/uiautomationcoreapi/nf-uiautomationcoreapi-uiadisconnectprovider)。
新增针对性 7 passed；全量库 2373 passed/28 ignored（61.05s），startup
7 passed（15.82s）；Clippy all-targets -D warnings、fmt、2944 文件严格行数
及 diff/源码 whitespace 检查通过，全部文件未超过 500 有效行。只读交叉审查
未发现阻塞性生命周期缺陷，结束时无 otter-native 残留进程。
未提交推送、未打包，正式能力报告不变。以下为此前检查点。

## N08 Switch 动态视觉与本地值像素证明

内部版本 v0.2.33.112。Switch 已补 props.disabled、hover、pressed 的静态帧
反馈：未选中悬浮采用 Neuro controlHover；选中悬浮为唯一信号黄 86% 与白混合，
按下为基色 82% 与黑混合；disabled 使用控件/次级文本角色，不响应悬浮、按下或
焦点，并继续以滑块位置区分当前值。禁用滑块使用 textMuted 60% 与 control 40%
混合，不引入另一套强调色。绘制不改变任何 Session 值或编辑/提交权限。

每帧先复制原始几何再应用当前 bool 覆盖，不累积滑块位移、不重排文字、不新建
COM/子窗口/计时器。新增 5 项真实 WIC/D2D 像素回归，证明本地值正反向覆盖与
相应权威值画面一致、checked 派生色、unchecked 悬浮、disabled 抑制所有动态
状态但保留值位置，以及带本地值的键盘焦点。Switch 针对性 24 passed。
最终全量库 2366 passed/28 ignored（64.73s），startup 7 passed（15.14s）；
Clippy all-targets -D warnings、fmt、2941 文件严格行数、diff 和源码 whitespace
检查通过，所有文件均未超过 500 有效行。

为避免既有像素测试文件继续增长，将 COM/WIC/D2D 生命周期、画刷恢复及后续
绘制标记断言等价提取到 `surface_pixels_fixture`；旧入口使用无本地覆盖回调，
新测试复用同一夹具，不复制测试渲染器。状态色计算归 `surface_switch_visual`，
颜色角色继续来自共同 control_colors，无新增运行时状态 owner。

证据仍是 96 DPI 离屏像素及隐藏 HWND，不是用户屏幕/多 DPI/真实鼠标视觉验收；
这里只覆盖声明 props.disabled，未新增全局 editing 门禁的暗化样式。UIA、真实
硬件及其余整体迁移继续，正式能力报告仍不开放 switch，未提交推送/打包。
以下为此前检查点。

## N08 Switch 祖先 click 与字段前缀顺序

内部版本 v0.2.33.111。新增旧 JSX/jsdom 合同对照 3 项：直接 input.click 是
祖先 click 先于 change；label/span 激活则因 label 转发产生两次祖先 click 后
再 change。这里是 jsdom 对照证据，不等于真实 WebView/浏览器验收。原生明确
采用一次激活、一批由内至外的祖先 click，再处理 change，不复制 DOM label
转发导致的重复业务动作；这是记录在案的行为差异，不宣称逐事件完全一致。

鼠标和 Space 激活前封存原表单快照及前缀 fence，成功切换后只有提交允许且存在
祖先 route 才排现有 Click 单槽；change 仍走共享字段 FIFO。disabled、失败的
切换不产生 click，软停止提交仅保留本地 change，不在恢复时补造祖先动作。
使用既有 binding/revision/合同复核与授权通道，没有新增发送 owner。

交叉审查发现无 handler 前置意图可令旧准备循环跨过 fence，提前取出 Switch
change。现通过 AuthorizedWindow → SurfaceWindow → Host → Mount 传递前缀
截止，每次消费下一意图前复核 Ready/InFlight/Invalid；前缀耗尽返回空，让既有
授权层先处理祖先 click。普通无截止准备行为保持不变。提交准备按职责提取到
`surface_edit_prepare`，不复制队列或状态，避免 Mount 超过严格行数边界。

新增 5 项真实 HWND 回归：祖先批次顺序、有 handler/无 handler 前置文本、Space
与软停止、disabled 和合同替换；Switch 针对性 19 passed。真实硬件/UIA、多 DPI
视觉、完整 disabled/hover 仍未完成，正式能力报告不开放 switch，未提交推送/打包。
最终全量库 2361 passed/28 ignored（60.78s），startup 7 passed（14.08s）。
Clippy all-targets -D warnings、fmt、测试 TypeScript 类型检查、2938 文件严格
行数及 diff/源码 whitespace 检查通过，所有文件均未超过 500 有效行。
以下为此前检查点。

## N08 Switch 本地交互与共享字段事件生命周期

内部版本 v0.2.33.110。Switch 已从静态图形接入本地状态：鼠标同目标按下/释放、
Tab 焦点和 Space 释放可切换，Enter 不伪装成 Button，自动重复与取消按压复用
现有窗口规则。编辑和提交分离，暂停提交仍可本地切换；撤销编辑权限、disabled、
拖出目标和取消捕获不提交。ReleaseCapture 前后比较场景 Rc 身份，避免同步重入
后的旧按压落入新场景。只使用隐藏 HWND 消息测试，没有注入用户桌面输入。

`surface_switch_session` 复用 Draft ticket，布尔字符串只存在内部 FIFO，wire 仍
是严格 JSON bool。Switch 与文本字段共用有界 32 槽 FIFO、30 秒等待期限和依赖
fence，不合并离散 change；先检查容量再变更本地值，权威更新等待在途/后续本地
预留完成。cancel 可重试，失败截断依赖但保留本地值；重复/外来结算不生效。
所有字段共用 64 个挂载额度，binding 变化一起重置。未把 Switch 新增到文本表单
快照，遵循旧 JSX 的 Switch 不调用文本 draft/form 更新路径。

交叉审查发现并修复同 ID 合同重建顺序：先保留合同匹配的文本与 Switch Session，
再清旧 waiting/prepared，最后创建同 ID 新会话，防止旧 bool 改投新 action，
或旧 cancel/fail 回执污染新队列。绘制从当前 Session 查询布尔值，不重排文字、不
新建资源；结算采用权威值时显式请求父窗口重绘，焦点环复用有序绘制/裁剪。

新增 9 项真实 HWND 回归，Switch 针对性 14 passed：暂停提交、Space/Enter/重复、
取消、FIFO/新输入保护、cancel 重试、合同更换/旧回执、disabled、满队列/撤权/
拖动、失败依赖、文本混合 FIFO 与无 handler 的权威同步。
最终全量库 2356 passed/28 ignored（63.24s），startup 7 passed（16.60s）。
Clippy all-targets -D warnings、fmt、2935 文件严格行数、diff 与源码 whitespace
检查通过；所有检查文件均未超过 500 有效行。

尚未完成：UIA、真实硬件/屏幕视觉、多 DPI/窄盒专项验证、disabled/hover 完整
视觉以及与旧 DOM 祖先 click 的逐项冒泡对照；本地 Switch 激活当前消费为 change，
不另排普通 Click。正式能力报告仍不开放 switch，整体迁移继续，未提交推送/打包。
以下为此前检查点。

## N08 Switch 原生测量、静态绘制与布尔事件合同

内部版本 v0.2.33.109。开始接入此前仅能解析的 Switch：以旧 JSX/CSS 的
label/span 为合同，新增固定 34×19 DIP 轨道、13 DIP 滑块，按最终布局盒居中，
严格 JSON true 为 on，其他值为 off。不创建子窗口、DirectWrite 布局、动画、
计时器或后台任务；使用共享画刷并恢复状态，复用父 opacity、裁剪和兄弟绘制顺序。
节点 style/layout 与旧 label 一样不接收通用覆盖；带 children 暂保守拒绝。
颜色映射 Neuro 唯一信号黄、控件、次级文本、分隔线及 on-yellow 派生角色，
开关状态同时由滑块位置表达，不只依赖颜色。

字段事件编码器新增严格 Boolean，仅 Switch change 可编码为 JSON bool/commit，
不生成 input，不放宽 Text/Number 类型，也不自行授权或发送。新增 5 项回归覆盖
声明值/尺寸、真实 WIC/D2D 像素、画刷/裁剪恢复、父 opacity/兄弟遮挡、隐藏/子节点
边界及 bool wire 合同，针对性 5 passed。最终全量库 2347 passed/28 ignored
（61.86s），startup 7 passed（16.05s）；Clippy all-targets -D warnings、fmt、
2931 文件严格行数检查及 diff/源码 whitespace 检查通过。

本轮不是完整交互交付：Switch 尚无本地 toggle/草稿、鼠标/Space、焦点/UIA、
提交队列和权威值同步，disabled 暂沿用旧静态外观，无额外颜色分支；滚动与窄盒
专项像素尚未实测。现有 Click 队列受 submitting 控制，不能直接替代独立的本地
编辑 owner；下一步需复用编辑 FIFO/身份/撤权边界，再接鼠标和键盘。未加入 Tab
集合，正式宿主 report 仍不宣告 switch，整体迁移未完成，未提交推送或打包。

以下为此前检查点。

## N08 Number 删除预检与原生撤销保护

内部版本 v0.2.33.108。非组合 Number 的无修饰键 Delete/Backspace，以及
WM_CLEAR/WM_CUT，在默认 EDIT 修改前按当前选区计算删除候选。非法结果直接
拒绝，不依赖 Session 回写，保留此前原生撤销记录；有效结果仍由系统执行。
真实 classic EDIT 对照确认：Delete 不按重复计数多删；Backspace 按重复计数
删除，有选区时先删选区，再向前删除剩余次数。范围计算在首尾饱和，不越界。

新增 4 项回归：真实 EDIT 重复计数/选区合同、Delete/Backspace/CLEAR 拒绝后
仍可撤销、合法重复删除/清空的 number/null 事件，以及范围边界/剪切候选模型。
测试不发送 WM_CUT、不读取或写入用户剪贴板。WM_CUT 的拒绝路径仅经过代码
审查和共享候选模型验证，真实剪切与剪贴板副作用未实测。修饰键词删除、组合
状态仍走既有 Session fallback，不承诺这些路径的 undo 无损。局部化、真实
IME、完整视觉/UIA 与整体迁移仍未完成，number 能力报告未开放；未提交推送或打包。

最终全量库 2342 passed/28 ignored（62.73s），startup 7 passed（16.27s）。
严格行数检查 2929 文件，所有文件均未超过 500 有效行。
Clippy all-targets -D warnings、fmt、diff 与新增源码 whitespace 检查通过。

以下为此前检查点。

## N08 Number 单快照粘贴事务

内部版本 v0.2.33.107。非组合 Number 的 WM_PASTE 现只读取一次 CF_UNICODETEXT，
验证 HGLOBAL 分配上限、偶数字节、NUL 结束和 UTF-16 单元预算，锁内复制后先
解锁再关闭剪贴板；系统句柄不释放。锁外将同一份拥有型快照交 EM_REPLACESEL，
复用 Number preflight、原生 undo 和既有 input FIFO，不重新发送默认 WM_PASTE
读取另一份内容。没有剪贴板监听、写入、后台重试或新增线程。资源规则依据
[Windows 剪贴板操作说明](https://learn.microsoft.com/en-us/windows/win32/dataxchg/clipboard-operations)。

读取前后 CAN_PASTE 均复核当前 Host 的 Number owner、本地编辑、disabled 和
组合状态，另查 HWND enabled/readonly；Host 忙时不读。临时 property/Box 标记
在读取期间的编辑、选区、焦点、组合、权限、布局/可见性及销毁变化中作废，
防止延迟渲染重入后把旧快照写到新目标；嵌套读取拒绝。最终资格查询后再查标记。
禁提交但允许本地编辑时可粘贴，仍不能准备发送；无效快照拒绝不损坏此前 undo。

新增 6 项回归：有界剪贴板字节解码，以及注入快照经过真实 HWND 的有效/空粘贴、
一次 input、原生 undo、非法/忙/畸形拒绝、重入/选区/嵌套、撤权/组合往返、
Host busy、readonly、非 Number、禁提交和关闭。Number 针对性 32 passed；
最终全量库 2338 passed/28 ignored（63.20s），Clippy all-targets -D warnings、
fmt、2928 文件严格行数和 diff 检查通过。startup 7 passed（16.44s）。

测试没有读写用户剪贴板；真实 GetClipboardData 延迟渲染、格式转换、跨进程
owner 尚未实测。分配块超上限即拒绝，即便 NUL 前文本很短也不例外。组合中的
粘贴仍走系统/Session fallback，不宣称该路径 undo 无损；局部化、真实 IME、
完整视觉/UIA 和整体迁移继续进行，number 能力报告未开放，未提交推送/打包。
以下为此前检查点。

## N08 Number 可预判插入与原生撤销保护

内部版本 v0.2.33.106。先通过真实 EDIT 回归复现：此前非法 EM_REPLACESEL 经
Session 回写恢复后，EM_CANUNDO 从非零变为零，丢失上一笔合法编辑的撤销能力。
新增 `surface_number_preflight` 在非组合 Number 的普通可打印 WM_CHAR 和
EM_REPLACESEL 进入默认过程前，按当前 UTF-16 选区构造完整候选并验证 grammar。
非法候选不修改 EDIT、不发编辑边沿、不回写文字；合法候选仍由系统执行，保留
调用方 canUndo 参数，不新增自定义撤销栈。替换消息的同步 UTF-16 指针依据
[EM_REPLACESEL 合同](https://learn.microsoft.com/en-us/windows/win32/controls/em-replacesel)
有界读取，不跨消息保存，也不是任意外部地址安全解析器。

补充真实 classic EDIT 对照又复现 WM_CHAR 重复计数边界：计数 2 的小数点会
实际插入两次。预检现展开低 16 位重复数再验证完整候选，避免第二个小数点
绕过预检并触发回写；合法重复数字仍按系统行为插入。空替换、部分选区和总
容量同样受检查。普通 Input/Textarea 与 IME 中间态不进入预检。

新增 5 项回归，最终 Number 针对性 26 passed，全量库 2332 passed/28 ignored
（61.51s）；Clippy all-targets -D warnings、fmt、2925 文件严格行数、diff 和
源码 whitespace 检查通过。startup 7 passed（17.17s）。

范围明确限定为可预判插入：WM_PASTE、撤销/删除、WM_SETTEXT、IME 收尾等仍
沿 Session fallback，不能据此宣称所有拒绝路径都保留原生撤销历史。真实剪贴板、
局部化、IME、完整视觉/UIA 与整体迁移仍待完成；number 能力报告保持关闭。
未提交推送或制作发行包。以下为此前检查点。

## N08 Number 输入过滤与编辑态边界

内部版本 v0.2.33.105。非组合状态的 Number 普通 WM_CHAR 先筛 ASCII 数字、
符号、小数点和指数标记；普通字母/其他符号在默认 EDIT 修改选区之前拒绝。
粘贴、整段替换、删除等沿既有最外层编辑边沿读取最终文本，不直接读取系统
剪贴板或替换消息指针。结构不合法时恢复原草稿及编辑前选区，相等值不产生
input，既有排队值不被覆盖；这是普通输入拒绝，不抛成窗口错误。

编辑 grammar 允许缺失尾数/指数数字，保留负号、小数点、e、e-、1e- 等中间态；
数值事件仍由严格有限数解析产生，未完成/溢出串为 null，表单为空字符串。
组合过程不拦截中间文字，结束才验证，合法数字组合进入原队列。Input/Textarea
不进入这套 grammar。参照 [HTML Number state](https://html.spec.whatwg.org/multipage/input.html#number-state-(type=number))
区分可见输入与提交值；该标准没有规定唯一逐键 UI，不据此宣称浏览器输入等价。

新增 5 项模型/真实 HWND 回归，Number 针对性 21 passed；全量库 2327 passed、
28 ignored（64.97s），Clippy all-targets -D warnings、fmt、2924 文件严格行数、
diff/改动源码 whitespace 检查通过。startup 7 passed（15.61s）。

整段非法结果经 WM_SETTEXT 式回写恢复，目前只证明值和选区范围，不保证原生
撤销历史、水平滚动位置或 caret 方向无损。真实剪贴板、真实 IME、局部化数字
输入和浏览器逐键对照仍待完成，Number 完整能力报告保持关闭。总迁移继续进行，
未提交、推送或制作发行包。以下为此前检查点。

## N08 原生字段 placeholder

内部版本 v0.2.33.104。Input、Number 和 Textarea 创建 EDIT 时安装独立提示层，
只在实际文本为空且非组合状态时绘制；空字段持焦仍显示。单行去 CR/LF，多行
保留规范化换行；字节数、UTF-16 数量和 NUL 独立验证。颜色使用 Neuro 次级文本
#929a9f，文字不经过 WM_SETTEXT，不进入草稿、surfaceValues 或 input/change。
字段合同变化重建提示；编辑/组合/焦点/启用消息后请求重绘。只分配有界文本，
没有新增 HWND、线程、网络或计时器；回调 Rc 与 HWND property 共同保护重入
销毁，取得/借入 DC 分别释放/恢复。

没有使用 [EM_SETCUEBANNER](https://learn.microsoft.com/en-us/windows/win32/controls/em-setcuebanner)：
官方限制不支持多行且要求 v6 common-controls。实际测试还发现当前 classic
comctl32 的 GetWindowSubclass 导出为零，导致 STATUS_ENTRYPOINT_NOT_FOUND；
已移除这个静态导入，改用项目已有 property 身份模式，不通过全局 manifest
切换掩盖兼容问题。修正后真实测试进程能够启动并通过。

新增 4 项回归：真实 WM_PRINTCLIENT/DIB 提示像素、持焦与填值/清空、组合与
禁用恢复、NUL/容量/重复安装/关闭，以及 Number 提示不污染值或产生事件。
针对性 placeholder 5 passed（包含既有测试），全量库 2322 passed/28 ignored
（63.91s）；Clippy all-targets -D warnings、fmt、2923 文件严格行数、diff 和
改动源码 whitespace 检查通过。startup 7 passed（16.09s）。

仍未验证真实 IME、屏幕 WM_PAINT/caret/闪烁、多 DPI 和完整节点文字样式，
离屏像素不代替桌面视觉验收。Number 输入过滤/局部化、长按重复和 UIA 仍未
完成，能力报告不开放 number；总迁移继续进行中，未提交推送或制作发行包。
以下为此前检查点。

## N08 Number 鼠标步进子窗口

内部版本 v0.2.33.103。Number 新增系统 up-down 按钮，与 EDIT 按同一 DPI 像素
边界分区，复用祖先 clip、后绘节点和滚动覆盖层裁剪。完全空的最终区域隐藏并
禁用按钮，拒绝晚到通知。按钮单线程 RAII 与 EDIT 同属 Field，先取消捕获后
销毁；撤权/disabled、布局同步、WM_CANCELMODE、失活、尺寸/DPI 变化清捕获。

UDN_DELTAPOS 只接受所属且有本地编辑资格的按钮单步；Host 借用外切焦后再次
复核当前身份/权限，走既有 Session 浮点 min/max/step 及成对 input/change 容量门。
不设置 buddy 整数回写，通知始终否决系统 position 更新，不新建数值状态、网络、
应用线程或计时器。禁提交仍可本地编辑，组合期间不能步进，Host busy 不积压通知。

新增 4 项真实 common-control 回归：上下半区鼠标消息和浮点结果、事件顺序与
系统 position 不变、伪造来源/非法 delta/重入/IME/撤权、分区和捕获取消、字段
替换/关闭销毁，以及完全遮挡后的通知拒绝。全量库 2318 passed/28 ignored
（62.04s），startup 7 passed（14.92s）；Clippy all-targets -D warnings、fmt、
2921 文件严格行数、diff/改动源码 whitespace 检查通过。全量库后仅为测试点坐标
表达式补运算优先级括号，生产源码未变；针对性 Number 测试随后重新验证。

尚未完成系统长按重复/移出移回实测、真实鼠标人工验收、Neuro 主题完整视觉、
placeholder、输入过滤/局部化、UIA；number 仍不进入能力报告。N08 和总迁移
继续进行中；本轮未提交、推送、制作 EXE 或发布，不把这些测试计为完整迁移验收。
以下为此前检查点。

## N08 Number 编辑与键盘步进

Number 已进入真实测量/焦点/EDIT 挂载路径，复用现有身份摘要、草稿 ticket、
事件 FIFO 与表单预算。权威 props.value 仍只接受字符串；初始无效值为空。
可见本地未完成草稿继续保留，surfaceValues 为校验后的字符串，input/change
payload 则为有限 number 或 null，不把空/无效值变成 0，不把手工越界输入钳制。
浮点格式与空值依据 [HTML Number](https://html.spec.whatwg.org/multipage/input.html#number-state-(type=number))
及 [浮点微语法](https://html.spec.whatwg.org/multipage/common-microsyntaxes.html#valid-floating-point-number)
核实；旧 JSX 的 min/max 缺省均为 0，step 缺省 1（字符串 any 不被 mapper 接收）。

实际持焦、enabled、非组合且无修饰键时，上下箭头按 min/max/step 对齐步进，
支持重复 keydown；父窗口和 Mount 再次复核本地资格/字段身份。禁提交仍可本地
编辑，但不能准备发送。步进先预检整对 input/change 的槽位与序号容量，同步
EDIT 通知在 Host 取出期间不重入队列；容量不足时保留原可见值，不留下半对事件。
浮点网格容差设上限，max 仅在同一合法网格时吸附，防止大数容差跨格。超出
2^52 网格索引或非有限中间结果保守拒绝步进，不采用无界枚举。

仍未完成：鼠标 spinner、placeholder、原生输入过滤/局部化、完整视觉和 UIA。
目前普通 EDIT 可显示无效中间字符串，但它不会以文本冒充 Number payload；
这不是浏览器所有输入行为等价证明。number 仍未进入对外宿主能力报告。
最终源码验证：全量库 2314 passed/28 ignored（60.82s），startup 7 passed
（15.63s）；Clippy all-targets（-D warnings）、fmt、2919 文件严格行数及
whitespace 检查通过。新增 11 项覆盖严格格式、边界/大数网格、空值步进、
真实 EDIT 数值/null 与字符串快照、异步回执、焦点/撤权/组合、合同变化、
队列仅余一槽及序号容量；没有将 ignored 计为通过，也未声称人工 IME 验收。
内部版本 v0.2.33.102；N08/总迁移继续进行中，未提交推送或制作发行包。
以下为此前检查点。

## N08 Icon 字形节点原生呈现

对照旧 DeclarativeSurface.tsx:284-292 和 CSS 的 inline-grid/place-items:center，
Icon 现进入生产测量/文字帧/绘制：缺省或非字符串 glyph 为 •，空字符串为空，
继承父字体/行高/whiteSpace/textAlign；内容盒居中与内部各行对齐分开处理。
忽略节点自身 layout/style 和子树渲染，保留原始合同/可访问性元数据，主题主色
映射 Neuro #d9ff38。字符串仍受现有 UTF-16/UTF-8/NUL 上限保护，失败不替换旧帧。
没有新增下载、解码、网络或动作 owner；Icon 不独立聚焦/执行 click，祖先的
合法事件路由保持不变。滚动复用已测量字形并重新定位/裁剪。

新建/显式恢复的共同宿主报告新增 icon；仍不声明 image、number、switch、select、
slider、完整 touch 或脚本能力。真实屏幕阅读器/UIA、emoji 彩色绘制及浏览器逐
像素等价未验，不把 Unicode 可布局测试当这些能力的验收。内部版本 v0.2.33.101。
验证：全量库 2303 passed/28 ignored，startup 7 passed（17.37s）。最后仅将
布局宽度 getter 重命名为 allocated_width 以消除 Clippy 命名歧义，随后
Clippy all-targets（-D warnings）、10 项 Icon 测试、fmt、2917 文件严格行数
及 git diff --check 通过。全量库/startup 来自该纯命名调整前，未把 ignored
当作通过。像素门覆盖信号色、空/隐藏、多行继承对齐、窄容器裁剪、父 opacity
和兄弟遮挡；另有真实 HWND 创建/关闭及滚动无重测/位移断言。
总体迁移/N08 仍未完成，未提交推送或制作发行包。以下为此前检查点。

## N08 Windows 触控平移与系统惯性接缝

父 HWND 配置单指水平/垂直 pan 和系统惯性，通过 WM_GESTURE 解码物理屏幕坐标，
转客户区和 DIP 后复用原滚动轴准入、祖先消费及文字缓存。双轴在一个滚动副本
计算并一次投影，不新建惯性线程/定时器。首个 BEGIN 只建立 anchor，不移动；
后续固定 anchor，END 消费最后位移再清序列。local_edit 撤权、场景替换、尺寸/
DPI、失焦、捕获丢失及关闭都使旧序列失效，禁提交不禁止本地滚动。Host 暂忙
丢弃该位移，不积压补跳；sequence ID 只作内部相等匹配，不是权限或硬件身份。

已处理 PAN 的句柄由 RAII 关闭；泛型 GID_BEGIN/GID_END 和未处理消息交
DefWindowProc，遵守 [WM_GESTURE 所有权约定](https://learn.microsoft.com/en-us/windows/win32/wintouch/wm-gesture)。
惯性来自 Windows 后续 PAN 消息，不是应用自行模拟物理效果。
仅增加既有固定 windows 依赖的 Win32_UI_Input_Touch feature，未新增包或版本。

新增 4 项针对性测试已通过：真实 HWND 配置读回、双轴/惯性/END/序列失配、
取消和 Host 忙时不补跳、非有限位移拒绝及边界夹取。解码后 seam 不代替真实
系统手势句柄、触屏硬件和 EDIT 子窗口互操作验收；这些与性能/UIA 仍未关闭。
全量库测试 2293 passed/28 ignored，startup 7 passed（16.98s）；Clippy
all-targets（-D warnings）、fmt、2915 文件严格行数及 whitespace 检查通过。
Clippy 最后指出测试侧 count 指针的 unnecessary_mut_passed，改为显式 raw mut
指针后 Clippy 和 4 项针对性测试重跑通过；全量库/startup 结果来自该测试表达式
调整之前，产品源码未再变更。未将 ignored 项算为通过。
内部版本保持 v0.2.33.100，N08 与总体迁移继续进行中；未提交推送或制作发行包。
以下为此前检查点。

## startup 功能回归的真实桌面资源隔离

针对上一检查点的 15 秒超时，独立脚本记录同一 EXE 的逐行单调时间戳及 SHA256。
空窗口 smoke 单独运行 4.6008s；空窗口/图片/透明 pin 三个完整 smoke 同时运行
分别为 14.0779s/15.2523s/15.8061s，均正常退出。多个前置阶段同步变慢，不是
仅 effect-controls 后一段停止；该最后日志不足以定位具体阻塞 API。采样 runner
使用 45 秒诊断上限以观察最终退出，不用于替代正式 15 秒 startup 门禁。
证据：`AI/GameEditor/linshi/otter-startup-timing.json`，脚本同目录
`otter-startup-timing.py`；此前超时日志继续保留。

`native/tests/startup.rs` 现用进程内互斥准入串行执行真实子进程，避免默认 libtest
并发把三个完整功能 smoke 变成共享桌面/图形资源的负载测试。没有删场景/断言，
没有放宽每 child 15 秒期限，没有修改产品运行时；失败后的毒化锁可恢复，仍让
后续场景独立执行。此锁仅限该测试二进制，不宣称跨测试进程或外部负载隔离。
修改后默认 libtest 7 passed/18.40s，显式 test-threads=7 再跑 7 passed/17.96s；
总时长不是单 child 时间。Clippy all-targets（-D warnings）、fmt、2913 文件有效
行数及 whitespace 检查通过。本次仅测试调度/文档变更，未重跑全量库测试。

这修正的是功能测试调度边界，不证明并发启动性能改善，也没有定位底层具体是
CPU、GDI、D2D 或其他共享资源；N11 仍须按明确负载与机器条件做性能验收。
内部版本保持 v0.2.33.100，N08/总计划继续进行中，未提交推送或制作发行包。
以下为此前检查点。

## N08 父窗口键盘滚动

父 HWND 实际持焦且有本地编辑资格时，现支持无修饰方向键（40 DIP）、PageUp/Down
（一页）和 Home/End（纵向首尾）；系统重复 keydown 可连续滚动，keyup/WM_CHAR
不触发滚动。命令复用滚轮的 auto/hidden 轴门禁和内层到祖先边界消费规则，一次
只由一个容器消费。当前位置未变化时不重排，发生滚动仍复用文字测量缓存。

动作焦点与滚动 anchor 分开：按钮焦点或背景点击可确定内层容器，没有 anchor
时从根开始；失效 anchor 不猜测新目标。禁提交时背景点击仍可建立本地导航入口，
但不创建 press/click。滚动保留逻辑焦点，撤销旧 Space 按压、动作候选和鼠标滚动
手势；SetFocus/释放 capture 后重新检查实际焦点与本地权限，避免同步重入恢复旧状态。

系统 EDIT 持焦时，父窗口不截获方向、Home 或翻页键，保留光标/编辑行为与文本；
带修饰键的快捷操作仍不由本入口接管。此项是视口键盘操作，不等于独立滚动条
Tab stop、UIA 或屏幕阅读器支持。最终源码库测试 2289 passed、28 ignored；Clippy
all-targets（-D warnings）、fmt、2913 文件有效行数与 whitespace 检查通过。
startup 首次默认并行 4 passed/3 timed out（既有 15 秒子进程上限），未改源码或
放宽上限：串行 7 passed/18.59s，随后默认并行 7 passed/14.26s。失败日志保留在
`AI/GameEditor/linshi/otter-native-scroll-keyboard-startup-parallel-failure.log`；尚未证明
偶发超时根因，不能将复跑通过当作稳定性风险已消除。

触控、状态表现、完整性能、多 DPI 和可见桌面
验收仍待完成；N08/总计划继续进行中，内部迭代 v0.2.33.100，未提交推送或制作
发行包。以下为此前检查点。

## N08 轨道按住重复分页

滚动条轨道现在支持按住重复：按下立即一页，400ms 后首次重复，后续间隔 50ms；
每条到期消息至多翻一页，不追赶积压 tick。每次使用当前几何重新命中原节点、
轴与方向，thumb 到达指针或指针移出轨道时暂停分页，移回原方向轨道可继续。
这两个暂停条件保留按住期间的 timer 检查；松开才结束手势，不宣称暂停即无唤醒。

重复 owner 持有唯一 HWND timer，单调 deadline 防止过早重复，ID 不跨手势复用；
RAII 在释放、失焦、capture 丢失、撤权、成功替换场景、尺寸/DPI 变化和关闭时
销毁 timer。旧排队消息不能驱动新手势，capture 返回后复核本次 owner 身份。
Host 暂忙跳过本次分页、保留有界重复，错误则取消；不扩大提交权限或创建动作。

新增回归覆盖可控时序、暂停/恢复、不反向振荡、积压单步、旧 ID 失效与取消链；
另有真实 Windows timer 经 WM_TIMER 窗口分发的验证，不注入全局鼠标。
最终源码全量库 2283 passed、28 ignored，startup 7 passed；Clippy all-targets
（-D warnings）、fmt、2909 文件有效行数与 whitespace 检查通过。
触控惯性、滚动条键盘/UIA、hover/pressed、多 DPI 与可见桌面人工验收及完整
性能门禁仍未完成。N08/总计划保持进行中；内部迭代保持 v0.2.33.100，未提交、
推送或制作发行包。以下为此前检查点。

## N08 原生覆盖式滚动条

auto 且存在溢出的轴现在绘制轨道与 thumb；hidden/无溢出轴不显示。支持轨道单次
分页和双轴滑块拖动，按偏移比例计算、边界钳位，双轴轨道让出角区；极短轨道仍
保留非零拖动行程。条沿祖先矩形裁剪，绘制和命中共用同一几何。覆盖层不占布局
宽度；10 DIP 宽度及中性色为本轮原生选择，不冒充浏览器 thin 的固定像素合同。

拖动仅要求 local_edit，不授予提交权限，也不创建 click/scroll action。开始前
废弃旧动作候选；捕获丢失、取消、失焦、场景替换、尺寸/DPI变化、权限撤销和关闭
均清理单槽拖动。没有条拖动时 move/up 仍交普通 pointer，避免从内容拖到条上后
遗留捕获或误发点击。Host 不可用或同步重入关闭/撤权时不能新建拖动。

EDIT 区域减去同一轨道覆盖矩形，不让子 HWND 遮住滚动条或截获输入；偏移变更后
保留原字段 HWND 和草稿。纯拖动沿用文字测量缓存，不新增定时器或后台线程。
已增加模型、真实 WIC/D2D 像素和隐藏 HWND 消息/区域回归。最终源码全量库
2279 passed、28 ignored，startup 7 passed；Clippy all-targets（-D warnings）、
fmt、2906 文件有效行数与 whitespace 检查通过。

尚未实现触控惯性、轨道长按重复、独立滚动条键盘/UIA 语义和 hover/pressed 表现；
未完成可见桌面、多 DPI 人工验收及高频资源门禁。现为矩形覆盖层，不声明圆角
裁剪或浏览器视觉完全等价，EDIT 同步失败也不宣称整窗事务回滚。N08/总计划仍
进行中，内部迭代保持 v0.2.33.100；未提交推送或制作发行包。以下为此前检查点。

## N08 纯滚动复用文字测量缓存

Presentation 现持有未滚动的拥有型布局快照、字号上下文与共享 DirectWrite 布局。
wheel 和键盘焦点显露从基准快照重新投影文字、裁剪、几何、绘制和命中，不再
创建文字布局；始终从基准坐标计算，避免往返滚动累积平移。场景替换、尺寸或
样式重排重建缓存，准备失败保留旧呈现与缓存。快照不持有自引用或裸指针，
结构检查不能替代失效规则，因此缓存只随 Presentation 的不可变场景使用。

新增测量计数回归验证 240 次小幅往返 wheel、20 次焦点显露均不重新测量；
显式重排/样式/场景更新重新测量，失败重排后旧快照仍可滚动。既有像素和 HWND
用例随全量回归通过。每次仍有树、绘制和命中快照分配及 EDIT 同步，未声明
零分配、整窗事务回滚或高频性能达标；滚动条、触控和可见桌面验收仍未完成。

最终源码全量库 2270 passed、28 ignored，startup 7 passed；Clippy all-targets
（-D warnings）、fmt、2900 文件有效行数与 whitespace 检查通过。N08/总计划仍
进行中，内部迭代保持 v0.2.33.100；未提交推送或制作发行包。以下为此前检查点。

## N08 滚动容器中的键盘焦点显露

Tab/Shift+Tab 的候选集合现包含屏外 button/input/textarea，仍排除禁用、隐藏和
零尺寸目标。动作可用集合与导航集合分开，屏外候选不能直接编码为可提交动作。
导航沿内外滚动祖先选择邻近边界显露目标；hidden 轴允许键盘显露但仍禁止滚轮。
大于视口的目标采用邻近边界策略，不承诺完整可见；目标未能显露时跳过，不阻断
Tab 环，也不提交失败候选引起的滚动。场景/几何/文字/命中在成功准备后一起更新。

Host 显露后同步 EDIT 位置，再转移系统焦点；导航前撤销旧指针候选，保持草稿和
原字段 HWND。EDIT NAVIGATE 入口现在检查 closed/local_edit 和实际焦点来源，
关闭编辑后送达旧 EDIT 的 Tab 不得滚动或恢复逻辑焦点；禁提交状态仍可本地导航，
但不能生成按钮动作。新增回归覆盖反向导航、嵌套显露、hidden 轴、不可达目标
不滚动，以及真实 HWND 的 EDIT→按钮→EDIT 和草稿/权限保留。

仍未完成滚动条呈现/拖动、触控惯性与高频性能优化及可见桌面人工验收；本段不
证明 UIA/屏幕阅读器完整支持，N08/总计划继续进行中。内部迭代保持 v0.2.33.100。
最终源码全量库 2267 passed、28 ignored，startup 7 passed；Clippy all-targets
（-D warnings）、fmt、2897 文件有效行数和 whitespace 检查通过。未提交推送或制作发行包。
以下为此前检查点。

## N08 声明式 Surface 双轴滚轮与滚动快照

原生容器新增有界本地偏移及内容范围，沿用 overflow 的 auto/hidden 计算值；
普通容器的显式 auto 也能滚动，不限于 scroll 类型。嵌套命中优先滚动最内层，
该方向已到边界才交给祖先；一次消息由一个容器消费，不分发剩余 delta。
场景替换保留仍存在节点的偏移并按新范围钳位，移除节点不留下无限历史。

WM_MOUSEWHEEL/WM_MOUSEHWHEEL 已进入真实窗口路由，支持 Shift 转横向、系统
lines/chars/page 偏好以及高分辨率小幅增量。Ctrl 滚轮留给宿主快捷手势，不改变
系统设置。入口要求本地编辑资格，不增加动作权限、不发送 scroll action。
同一平移后的树生成文字位置、D2D 绘制、裁剪、命中与 EDIT 位置；保留字段 HWND
与未提交草稿。几何改变或 EDIT 同步失败均废弃旧 pointer 候选并请求重绘。

新增纯快照、真实 WIC/D2D 像素及隐藏 HWND 回归覆盖双轴/嵌套边界、精确范围、
末尾子节点边框、场景缩短钳位、亚像素增量、EDIT 移动保草稿与本地输入门禁。
当前滚动仍使用有界完整重排，不宣称高频输入性能已验收；滚动条呈现/拖动、
键盘焦点自动滚入视野、触控惯性和可见桌面人工验收仍未完成。N08/总计划保持进行中。
最终源码全量库 2262 passed、28 ignored，startup 7 passed；Clippy all-targets
（-D warnings）、fmt、2895 文件有效行数检查通过。
内部迭代推进至 v0.2.33.100；未提交推送、合并或制作发行包。以下为此前检查点。

## N08 前置编辑与依赖点击接线

已接通激活快照、编辑序号屏障及单槽依赖点击。鼠标/键盘点击只等待激活前的编辑
完整结算；之后的新输入不能抢先提交。前置成功后重新核验绑定、目标命中、按钮
合同、授权和当前 revision，再编码原激活时的表单值。没有前置编辑的点击也走
同一顺序门禁，无 handler 的本地编辑完成后可在同次调度释放点击。

点击期限固定为激活时刻起 30 秒，延迟捕获不续期；失败、合同变化、撤销本地
编辑资格或到期使旧依赖点击失效，不重放已尝试动作。测试使用真实 HWND 消息和
合成结果证据，不代表外部服务、可见桌面或完整 Tile 交互验收。N08/总计划仍未完成。
接续后最终源码全量库 2253 passed、28 ignored，startup 7 passed；Clippy all-targets
（-D warnings）、fmt、2890 文件有效行数及 whitespace 检查通过。
本次接续保留内部迭代 v0.2.33.99，未提交、推送或制作发行包。以下为此前检查点。

## N08 未发编辑意图等待期限

编辑 FIFO 现为每条新意图记录 30 秒单调时钟期限，准备发送前检查。任一未发意图
过期即截断整条未发依赖链并报 Surface edit queue expired，保留可见草稿，同时
清除失焦 change 标记，不能仅因恢复窗口/授权或 blur 自动重放。明确的新编辑可
创建新意图；只有队尾相邻连续输入被新输入替换时刷新期限，取消归还保留原期限。

在途 prepared 事件及其结果证据不按排队期限删除或重发；网络与确认门禁仍沿用
原链。没有新增后台定时器，过期检查发生在实际派发准备前。确定性测试注入时间，
不用 sleep 或系统时钟修改。按钮依赖队列仍未接通，N08/总计划继续保持未完成。
鼠标/键盘激活捕获表单快照时还会封住编辑队尾：后续 input 不得跨点击边沿合并
并覆盖点击前的待发值。新输入仍可在边沿之后彼此合并；这不等于按钮已能跨前置
结果等待和场景更新存活。按钮依赖保存/成功唤醒接线仍待继续完成。
最终源码全量库 2247 passed、28 ignored；Surface 定向组 384 passed、1 ignored，
startup 7 passed；Clippy all-targets（-D warnings）、fmt、2888 文件有效行数及
scoped whitespace 检查通过。内部版本 v0.2.33.99；未提交推送或制作发行包。
以下为此前检查点。

## N08 编辑节点完整合同复核

文本会话复用从 ID/type/events 扩展到节点合同摘要：props（仅忽略根节点 value）、
layout、style、accessibility、events 及完整子树均参与。仅权威 value 更新仍保留
原会话与本地待发草稿；其他合同变化重建字段，旧队列和 ticket 不跨合同复用。
这使相同 action 下的输入约束、布局/样式或可访问性变化也能阻止旧意图继续提交。

摘要在当前进程内使用 SHA256，JSON 结构/长度明确，流式计算而不复制整棵场景。
全量合同先计算成功再修改字段集合。控件 HWND 可随合同变化而重建，测试不再把
旧 HWND 在 disabled/约束变更后仍可用作为前提。按钮依赖队列与期限仍未完成；
本轮未放宽 lease/revision 检查，也不声称完整 Tile 交互等价。
最终源码全量库 2244 passed、28 ignored，编辑定向组 45 passed，startup 7 passed；
Clippy all-targets（-D warnings）、fmt、2888 文件有效行数及 scoped whitespace
检查通过。内部版本 v0.2.33.99；未提交推送或制作发行包。以下为此前检查点。

## N08 前置编辑失败截断依赖队列

批次交付现在将失败、未知或证据不足的前置编辑通知回 Mount：只有仍登记在当前
会话中的事件 ID 可清除后续未发意图，旧合同回执不能清掉新会话。清队列不修改
可见文字、不采用未验证权威值，也不重排已尝试事件；同时清除待失焦 change 标记，
防止失败后仅一次 blur 又自动提交旧依赖值。用户新的显式编辑仍可产生新意图。

NotStarted 且全批未尝试仍归还原队列；身份匹配的明确 awaiting_confirmation ACK
且无交付失败则保留等待，不把用户确认等待误判为失败。完整成功证据仍走原结算。
这是编辑依赖失败边界，不代表按钮依赖队列、超时、完整合同重验证已经完成。
最终源码全量库 2242 passed、28 ignored，编辑定向组 43 passed，startup 7 passed；
Clippy all-targets（-D warnings）、fmt、2887 文件有效行数及 scoped whitespace
检查通过。内部版本 v0.2.33.99；未提交推送或制作发行包。以下为此前检查点。

## N08 文本编辑 FIFO 与提交边沿值

文本通知不再通过 BTreeMap 字段顺序扫描 input/change 布尔标记派发。Mount 使用
统一的有界编辑 FIFO，跨字段保留通知顺序，只合并队尾相邻同字段 input；change
记录当时值并成为合并边界。prepare 使用已捕获值，不重新读取较晚的 HWND 文字。
排队与已准备未结算意图合计最多 32，保留取消所需槽位；容量不足明确返回错误。
每条文字仍受既有 48KiB 值上限，未新增线程、网络 owner 或同版本拼批。

Session 为较早快照与较晚本地草稿分开持有 ticket，较早结果不能抹掉仍排队的
后续值。未启动取消按原值归回队首；动作声明变化/字段移除清除对应意图，硬关闭
编辑门禁清理待发队列。完整证据成功后仍走原结算路径。尚未将按钮加入此队列，
依赖点击的完成 barrier、失败丢弃、期限及完整节点合同复核仍待实现，N08 未完成。
最终源码全量库 2240 passed、28 ignored，startup 7 passed；Clippy all-targets
（-D warnings）、fmt、2887 文件有效行数与 scoped whitespace 检查通过。新增四项
覆盖 change/取消的边沿值、跨字段 FIFO、容量预留及满队列时可见值/表单值一致。
内部版本 v0.2.33.99；未提交推送或制作发行包。以下为此前检查点。

## N08 动作声明更替隔离与事件顺序合同核实

Mount 现在将字段的 events 声明纳入会话复用条件。同 ID/同类型节点更换、增加或
移除动作时，重建对应 Session；旧本地草稿、待发 phase 和 ticket 不再被重定向到
新 action。新的权威值进入新会话，旧回执不能结算新会话。仅 value 更新且动作
声明未变时仍复用原会话，保留已实现的异步草稿保护。

已核实普通 UnitSurfaceContent 为 fire-and-forget，不能据此前端推断服务端承诺
同 revision 多事件执行顺序。Tile 的 tileSurfaceController 则有 per-instance
FIFO、只合并队尾相邻 continuous、前置编辑完成 barrier、失败清后续，以及
refreshTileSurfaceEvent 的节点合同复核/dispatch-time revision 更新。不能将这两条
路径混为“把全部待发事件拼成同 revision 批次”。当前原生输入/点击完整保序仍待
开发，本轮没有放松场景、版本或 action 资格来保存旧候选。

Loom 当前源码进一步确认：surface_store/validation.rs 的新事件准入要求 baseRevision
等于当前 attachment revision；准入本身不涨版本，但前一 patch 提交后旧版本即冲突。
executor 在准入时捕获 authoritative_state，Serial 协调键为 instance:action，而非整个
instance；不同 action 不具备统一执行顺序保证。因此保序开发须明确引入前置执行
barrier 与合同复核，不能把 click 冒泡批次直接推广为表单依赖队列。

最终源码全量库 2236 passed、28 ignored，startup 7 passed，动作更替定向组 2 passed；
Clippy all-targets（-D warnings）、fmt、2885 文件有效行数和 scoped whitespace 检查
通过。内部版本 v0.2.33.99；未提交推送或制作发行包。以下为此前检查点。

## N08 点击边沿固定表单快照

鼠标抬起确认点击、键盘 Enter/Space 激活时，单槽 Click 候选现在保存当时的有界
表单汇总；后续文字编辑不再替换这次动作携带的 surfaceValues。Host 暂不可用或
窗口已关闭时不以空表单冒充成功快照。候选仍须通过原场景身份、目标/焦点、租约、
版本及 owner 检查，场景替换/门禁关闭仍使旧候选失效，不跨新版本重放。

这只补齐激活边沿的数据身份，不解决 input/change 优先调度与按钮候选失效的
完整交互顺序；该缺口继续保留，N08 和总迁移计划仍未完成。

最终源码全量库 2234 passed、28 ignored，startup 7 passed，表单定向组 6 passed；
Clippy all-targets（-D warnings）、fmt、2884 文件有效行数及 scoped whitespace
检查通过。真实隐藏 HWND 测试覆盖鼠标与键盘激活后再编辑，候选仍携带激活值，
当前草稿保持新值。内部版本 v0.2.33.99；未提交推送或制作发行包。以下为此前检查点。

## N08 表单草稿汇总进入按钮提交

Host 的鼠标与键盘按钮事件已接入 includeSurfaceValues；开启且缓存非空时，对象
eventPayload 合并 surfaceValues，数组/标量保留为 configured，关闭时不改 payload。
Values 独立拥有最多 16 字段、48KiB UTF-8 总量，按更新时剩余额度截断且不拆开
Unicode 字符；预算只约束提交快照，不截断可见草稿。初始化、用户编辑及完整结果
结算后的权威值更新缓存，换绑清空；与当前前端一样，节点移除不清除已登记值。
控件挂载上限仍为 64，不等于提交缓存名额。事件编码及整个冒泡批次原有 64KiB
上限继续生效，转义后超限整批拒绝，不静默省略字段。未增加发送路径或放宽授权。

本轮不是完整表单验收：候选按钮与 input/change 的离散顺序、跨进程接收、真实
IME 和其他控件仍待推进。前端 JS 截断可产生单独代理项，原生选择完整 Unicode
边界，两者极端边界尚非逐字节等价；能力 report 不扩大。

最终源码全量库 2233 passed、28 ignored，startup 7 passed，新增表单汇总定向组
5 passed；Clippy all-targets（-D warnings）、fmt、2884 文件有效行数与 scoped
whitespace 检查通过。内部版本 v0.2.33.99；未提交推送或制作发行包。以下为此前检查点。

## N08 textarea 用户长度限制与原生替换通知合并

textarea 的 maxLength 已进入生产 Session：与当前前端一致，数值限制截断到整数
并裁到 1–65536，非数值使用默认值；计数使用规范化 LF 后的 UTF-16 单元，emoji
占两个单元，原生 CRLF 不重复计入。权威初始值/更新不受用户 maxlength 截断，
仍受既有资源上限保护；超过用户限制的既有值允许缩短或等长度替换，不允许增长。

原生一次 EM_REPLACESEL 可先通知删除再通知插入。现以有界嵌套计数合并一次
编辑命令的 EN_CHANGE，只捕获最终值；程序回写仍不触发业务事件。超限或无效
读回恢复先前草稿对应的可见文字并返回错误，不派发被拒绝的文字。IME 结束拒绝
时也释放组合预留、清理组合状态，下一次合法编辑仍可继续。

该实现拒绝整个超限编辑，不截断到可用长度；超限回滚的选择区/撤销栈与浏览器
粘贴行为尚需对照，不能据此声明所有编辑手感等价。字段汇总、placeholder、完整
样式和其他 N08 缺口不变，能力报告保持不扩大。

本次最终源码全量库 2228 passed、28 ignored，startup 7 passed；Clippy all-targets
（-D warnings）、fmt、2882 文件有效行数门禁与 whitespace 检查通过。新增四项
隐藏 HWND 回归覆盖 UTF-16/换行计数、权威超长值缩短、IME 拒绝后的继续编辑及
数值取整/裁界。内部版本仍为 v0.2.33.99，未提交推送或制作发行包。以下为此前检查点。

## N08 文本字段生产挂载与授权批次接线

本段属于总计划 N08（能力与扩展），不是 N09 投送/Tile 或 N10 发布门。已校正
最近按钮、编辑会话增量标题中混用的编号；总计划 N01–N12 的退出条件没有缩减。

已确认激活的 AuthorizedWindow 将当前生命周期 binding 传入 Host，按已提交布局
创建 input/textarea 的真实 EDIT 子窗口。创建/禁用/删除/替换与父窗口销毁均由
同一 UI owner 管理。每窗口最多 64 个字段，每字段保留既有 48KiB 草稿限制；
这不是前端完整 surfaceValues 汇总合同。后台 owner 移交只关闭提交，不禁用编辑。

EN_CHANGE、失焦和单行 Enter 接入 input/change；Tab 在字段与按钮间移动，组合
中的按键留给默认 IME 过程。IME 结束先经过系统 EDIT 再读回，硬关闭编辑门禁
显式收尾组合状态，避免同步结束通知在 Host 取出期间丢失后永久卡住。程序回写
不递归创建输入事件。textarea 在原生 CRLF 与协议 LF 间转换；rows 对齐 1–20。

输入事件复用 Application 的既有租约、设备、结果/commit、容量和唯一 owner
检查，经原批次后台任务发送。准备后拒绝/未启动归还本地预留，只有完整批次
证据通过并接纳场景后才结算；未知或错误结果不被当作成功，也不自动重放。
已验证激活 ACK 会改变 binding，故挂载在确认激活后，而不是提前捕获 mounted
版本。测试使用真实隐藏 HWND 和合成 ACK/结果证据，不声称已经连接真实 Loom。

布局按 DPI 转成客户区坐标，使用窗口 region 裁到祖先可见范围，并减去后绘制
节点的矩形命中盒，避免子 HWND 在覆盖节点上方截获输入。该遮挡处理是保守的
矩形策略，不等于任意圆角/透明度的逐像素合成等价。默认控件色使用 Neuro 角色。

仍待开发：placeholder（尤其多行）、完整字体/样式和圆角/透明度合成、textarea
maxLength 与总字段草稿预算/按钮 surfaceValues、连续输入和离散提交边沿的完整
调度对照；真实 IME、屏幕阅读器和跨进程交互另需运行验收。生产 report 暂不增加
input/textarea，避免将这条基础生产链误报为完整控件兼容。之后继续补其余表单、
图片/图标、滚动、JavaScript Surface 和 Shader；不能因文本接线通过就勾选 N08。

最终源码全量库 2224 passed、28 ignored；编辑定向组 31 passed；startup 集成组
7 passed。Clippy all-targets（-D warnings）、fmt、2881 文件有效行数门禁、版本
一致性及 scoped whitespace 检查通过。内部版本 v0.2.33.99；未提交、推送、合并
或制作发行包。源代码验证和真实远程/人工输入验收仍分开记录。

以下为此前检查点。

## N08 编辑会话组合与中断恢复

续接未完成的 `surface_edit_session`，补齐真实隐藏 EDIT 的会话回归。会话统一拥有
控件、绑定/节点身份、单槽未提交值和按不可变事件 ID 索引的有界 ticket；后台
结果不能覆盖更晚的未提交输入，取消派发恢复本地预留，重复/外来回执不生效。

修复两个状态丢失边界：缺少当前 phase 的 handler 不释放另一个 phase 待提交值
的保护；取消 IME 组合输入不被当成远端提交结果。组合期间仅在接纳了不同权威值
或真实回执后，才在取消组合时协调权威值；否则保留组合开始前的本地文字。
保留一个 ticket 给后续输入/IME，容量达到上限时不追加无界请求。

这里只完成编辑会话组合，不是生产 Surface 控件挂载。Windows IME 消息路由、
布局/裁剪/样式、实际远程提交和人工连续输入验收仍未完成，不新增能力声明。
最终源码全量原生库 2212 passed、28 ignored（包含 9 项编辑会话回归）；startup
集成组 7 passed。Clippy all-targets、fmt、2873 文件有效行数门禁及 diff 检查通过。
未提交推送、未制作发行包；以下为此前检查点。

## N08 本地编辑/动作提交门禁分离

生产 Application 现分别传入本地编辑意图和动作提交意图。可见性、modal、退出、
当前 origin 仍约束本地编辑；AuthorizedWindow 再核验租约、窗口状态和已确认活动
生命周期。本地编辑可跨已验证的 event/batch owner 移交存活，但提交仍要求唯一
confirmed-active owner、既有容量/结果/commit 门禁，不克隆 owner 或放宽发送检查。

SurfaceWindow 的 set_input_policy(editing, submitting) 将 false/true 归一为全禁用。
软停止提交撤销指针/按键候选及 Space 按压，更新候选身份，但保留逻辑焦点；关闭
本地编辑仍走完整清理。begin_batch 改用软停止，owner 在后台期间应用循环不会
仅因提交暂不可用清掉逻辑焦点。租约撤销/过期仍关闭窗口，晚到 owner 不复活窗口。

已通过证据门禁的场景 replace 后，若当前 OS 焦点仍在窗口、同 ID 节点仍可聚焦且
本地编辑仍有效，则保留逻辑焦点；旧候选和按压仍失效。删除/禁用节点不保留焦点，
新场景动作只能由新候选触发。这里尚未挂载 EDIT 或协调其 IME/草稿；该状态边界
只是文本编辑接线的前置，不能据此新增 input/textarea 能力声明。

Surface 定向组 341 passed、1 ignored，覆盖真实授权 owner 移交、未激活窗口、
后台提交时撤销、提交恢复、软停止后的旧候选拒绝及场景更新的焦点连续性。
以下为此前检查点。

最终源码全量原生库 2203 passed、28 ignored；隐藏启动/保存 1 passed（4.49 秒）；
Clippy all-targets、fmt、2871 文件有效行数门禁和 diff 检查通过。未提交推送或
制作发行 EXE，未作人工连续文本输入/IME 验收。

## N08 输入事件编码与异步草稿合同

`surface_input_event.rs` 已区分 input/continuous 与 change/commit，使用与 click
相同的不可变事件 ID、binding/revision 和 64KiB 序列化预算；文本/数值类型不互相
隐式转换，空或无效 HTML number 由显式 None 编码为 null，而非伪造 0。禁用、
不可见叶节点和未声明 handler 不生成动作；上层仍须验证祖先可见性、当前节点和授权。
输入 payload 使用实际 value，不套用按钮的 eventPayload，也不携带鼠标修饰键。

`surface_input_draft.rs` 实现单字段有界草稿：异步进行中保留本地值，但记录最新
权威值；所有当前请求结算后才回写。乱序/重复回执不会重复扣计数；新 scope、
外来 scope 的旧 ticket 不生效。同一权威字符串的重复同步不会冲掉没有异步
handler 的本地输入。每字段值/权威值各最多 48KiB，最多 64 个预留/待结算 ticket；
派发前预留容量，未启动时 cancel 保留草稿，真实结果完成时 settle。

真实隐藏 EDIT 回归已串起 Unicode 读回→草稿→input wire 编码→权威更新→结果
结算→控件回写；这仍是本地合同测试，不是生产 Surface 挂载或真实远程提交。
点击编码的既有合同测试继续保护共用序列化提取，能力报告不新增输入节点。

### 连续编辑接线的已确认约束

当前 `AuthorizedWindow::begin_batch` 会调用 set_pointer_enabled(false)，该方法
清除逻辑键盘焦点和候选身份；owner 移至任务期间，sync_pointer 也会持续要求关闭
输入。finish_batch 只返还 owner，不恢复原逻辑焦点；接纳快照的 replace 又会清理。
这里未观察到主动 SetFocus(None)，不能把逻辑焦点清理说成已经证实的 OS 失焦。

后续必须在 application/authorized-window 层分离本地编辑资格与动作提交资格，
维持独占 owner 和完整批次返还合同，不克隆 owner 或放松租约/版本检查。草稿和
IME 状态不能挂在按钮候选的统一 invalidate 上；服务端新快照仍先通过证据门禁，
再按 binding/node 身份协调草稿。现阶段尚未实现此接线，也不宣称连续输入已可用。

本阶段最终源码全量原生库 2198 passed、28 ignored；隐藏 Win32 启动/保存回归
1 passed（4.18 秒）；Clippy all-targets、fmt、2870 文件有效行数门禁和 diff 检查
通过。草稿 ticket 留在 UI owner，不直接跨线程传给网络任务；后台结果通过既有
事件身份/回执回到 UI 后再结算。未提交推送、未制作新的发行 EXE。

## N08 文本输入原生资源 owner（尚未完成 Surface 接线）

新增 `desktop/surface_edit_control.rs`，使用 Windows 内置 Unicode EDIT 进行实际
编辑，不引入 Qt/WebView 或另一套文本输入引擎。支持隐藏创建、布局定位、显示/
焦点、单行/多行模式及有界读写。owner 限于创建线程，通过私有窗口属性验证
HWND 身份；父窗口先销毁后，读写失败且 Drop 不会误操作已复用的 HWND。

UTF-16 单元上限最多 64K，UTF-8 内容最多 48KiB；读回先检查长度上限，再按实际
长度分配缓冲，短输入不分配整个 64K 缓冲。程序写入独立校验容量、NUL 和单行
换行边界；读取再次检查身份/长度，超限或畸形 UTF-16 返回错误，不静默截断。
上层尚须决定超限用户输入的可见反馈与草稿协调，不得吞错后发送不一致的值。

已联网核对 [EM_SETLIMITTEXT](https://learn.microsoft.com/en-us/windows/win32/controls/em-setlimittext)：
该消息不限制 WM_SETTEXT，因此程序写入和读回必须独立守卫。另外
[EM_SETCUEBANNER](https://learn.microsoft.com/en-us/windows/win32/controls/em-setcuebanner)
不支持多行 EDIT，后续 textarea placeholder 不能直接套用单行接口。

本阶段定向回归 4 passed，实际验证中文/emoji 读写、选择后替换、CRLF 多行、
UTF-16 与 UTF-8 独立上限、WM_SETTEXT 绕过控件限制后的拒绝、无效边界和父子
销毁。没有触碰系统剪贴板，未实际验收 IME 候选窗口。该 owner 目前未由 Surface
输入节点挂载，生产能力报告不新增 input/textarea/number；焦点选择、事件草稿、
IME、placeholder、样式/裁剪及实际可见窗口验收仍是后续必要工作。

本阶段全量原生库 2189 passed、28 ignored。Clippy 首次提示 ShowWindow 的此前
可见性返回值未显式消费，修正为有注释的忽略后 all-targets 通过，最终源码定向
4 passed、隐藏启动/保存 1 passed（5.22 秒）；fmt、2866 文件有效行数门禁与
diff 检查通过。全量测试之后的源码修改仅为该返回值处理及注释；未再次全量运行。
本轮未提交、推送、合并或制作新的发行 EXE。

## N08 按钮悬浮/按下反馈及生产能力接线

按钮动态颜色由预构建 PaintColor 在绘制时选择，不替换场景或重排文字。悬浮
采用主干按钮的强调色背景/边框和深色文字角色；按下采用 Neuro yellowPressed
（signal 82% + black），不是声称与旧 CSS fallback 逐像素相同。显式 background、
borderColor、color 优先于默认动态色，disabled 按钮不应用交互色。

鼠标移动只在命中状态变化时请求重绘，重复移动同一按钮不会持续 InvalidateRect。
使用系统 TrackMouseEvent 接收离开通知，不增加应用计时器。按下、释放、拖动
取消、捕获丢失、禁用及替换沿现有状态清理路径还原反馈；Space 按压也驱动同一
绘制状态。客户区按钮使用共享系统手形/禁用光标，不接管窗口非客户区光标。

生产 report 增加 button，并声明 pointer/hover/keyboard；仍不声明 touch、其他
表单控件、Image/Icon、滚动或 JavaScript runtime。此声明表示已有原生接线，
不代表完整迁移、屏幕阅读器可访问性或人工可见窗口验收已完成。以下为此前检查点。

Surface 定向组 323 passed、1 ignored；最终源码全量库 2185 passed、28 ignored；
隐藏启动/保存 1 passed（5.32 秒）；Clippy all-targets、fmt、2864 文件有效行数
门禁与 diff 检查通过。首次编译发现 WM_MOUSELEAVE 的 windows crate 命名空间
不在 WindowsAndMessaging，按本机绑定修正为 UI::Controls 后通过以上检查。
本轮未提交推送或制作发行包。

下一项推进文本输入控件。主干 SurfaceTextInput/SurfaceTextArea 不仅显示 value，
还拥有本地 draft、input/change 事件、placeholder/disabled、字符及 UTF-8 总量
约束；textarea 包含 rows、选择文本与独立滚动。不得用静态文字替代输入控件，
也不能遗漏 IME、焦点和草稿更新生命周期。其他控件与图片/脚本缺口保持原范围。

## N08 按钮键盘焦点与激活链路

在基础呈现之上增加 Tab/Shift+Tab 顺序切换、Enter 按下激活、Space 按下记录与
释放激活。焦点快照只收录当前可见布局中的非 disabled 按钮，随布局提交更新；
键盘热路径不重建整个场景。按键自动重复不追加动作，键盘与鼠标共用单槽候选。

键盘激活使用节点 ID，不伪造点击坐标；事件准备仍复核场景身份、焦点及当前可
聚焦资格，外层继续复核 owner/绑定/版本。焦点切换（包括切回相同 ID）、失焦、
场景替换、尺寸变化和关闭会取消旧候选/Space 状态；本地输入禁用时不响应键盘。

键盘焦点使用 2px Neuro signal 指示线。指示属于按钮自己的绘制命令顺序，继承
父级透明度与裁剪，后绘制的兄弟覆盖层可以遮挡它；不是绘制在场景最上面的浮层。
复用现有画刷，绘制后还原颜色和 clip，不创建动画计时器或重新排版。

Surface 定向组 319 passed、1 ignored，包含真实隐藏 HWND 按键、焦点切回旧候选
失效和真实 WIC 焦点线像素/遮挡/透明度回归。仍缺悬浮与按下反馈及完整人工交互
验收，能力报告暂不宣告完整 Button/keyboard/hover 支持。以下为此前检查点。

本阶段全量库 2181 passed、28 ignored；隐藏 Win32 启动/保存回归 1 passed
（3.69 秒）；Clippy all-targets、fmt、2860 文件有效行数门禁及 diff 检查通过。
所有源码修改之后完成这些检查；未提交推送、未制作 EXE 发行包，未作人工验收。

## N08 声明式按钮基础呈现与点击链路（进行中）

Button 已接入 NativeMeasure、DirectWrite 文本帧和现有 Direct2D 绘制帧，不再仅在
协议解析层接受。标签读取 label；默认水平/垂直 padding 为 12/6、边框 1、最小
高度 30，文字居中；显式布局和样式保留。静态默认色映射 Neuro 控件/正文角色，
并不声称与旧 CSS fallback 颜色逐像素一致。disabled 默认透明度 0.48。

新增专项回归 4 passed：内容盒/垂直居中、显式样式与最小尺寸覆盖、真实 WIC
背景/圆角边框/标签像素，以及隐藏 HWND 的鼠标按下释放到事件候选转换。按钮
阻止父容器 click 冒泡，disabled 不生成动作；测试不实际发送网络动作。

本阶段全量原生库 2176 passed、28 ignored；隐藏 Win32 启动/保存回归 1 passed
（5.03 秒）。Clippy 首次发现迁移后的未使用 import，移除后 all-targets 检查通过；
fmt、2856 文件有效行数门禁（无超过 500 行项）与 diff 检查通过。全量运行之后
仅清理该未使用 import 及更新文档，未重新执行全量测试。未提交推送、未制作发行包。

本阶段仍缺 Tab 焦点、Enter/Space 激活、焦点环及悬浮/按下反馈。因此生产能力
报告暂不新增 Button 或 keyboard/hover 支持，不能将该阶段登记为控件迁移完成。
Input/Textarea/Number/Slider/Switch/Select、Image/Icon 和 JavaScript runtime 的
缺口不变；后续继续补按钮交互，再扩展其他控件。以下为此前检查点。

## N08 大局部模糊组合回归与缩小接缝修复

新增真实 WIC 对照通过完整 blur 绘制入口，使用同一局部资源、分数双轴 mask、
多段圆连接、源投影几何 clip 和持久框旋转。生产尺寸阈值走 tiles，测试对照仅
强制整图绘制；比较全部输出字节，允许最多 1 个量化级差，不豁免内部接缝。

该回归首次运行捕获真实接缝：scale=1、angle=0 时 byte=231000，tiled=8、
full=10。逻辑 frame 缩小源纹理后，单个源像素邻域不足以覆盖设备像素，块边缘
的 DrawBitmap 覆盖率进入可见区域。修复后按当前 target 逆变换与纹理比例计算
设备像素足迹，另加线性插值邻点；邻域按二次幂分桶，并裁到源图尺寸。块原点仍
使用固定 512 网格，相同足迹桶及重复绘制复用上传，不随每次微小缩放变化创建键。

新组合回归覆盖 0°/17°/-23°、多个缩放比及重复绘制缓存；既有块回归同步验证
新邻域桶的实际上传数。定向 blur 组 81 passed；全量库 2172 passed、28 ignored；
隐藏 Win32 启动/保存回归 1 passed（3.70 秒）；Clippy all-targets、fmt、2854 文件
有效行数门禁与 diff 检查通过。本轮未提交推送、未制作发行包。
缓存仍受既有 32 MiB/32 项限制；更大的邻域有上传成本，极端缩小、设备尺寸上限
和真实 GPU/驱动峰值仍需验收，不能由该有限场景测试推导零成本或所有倍率正确。

### 后续主干功能优先项（已核对当前实现）

下一阶段转向声明式 Surface 的可操作控件，不继续仅围绕模糊像素细节推进。
该检查点 surface_scene 虽接受 Button/Input/Textarea/Number/Slider/Switch/Select，
surface_native_measure 的可见节点测量仍明确拒绝它们，application_surface_start
也未声明这些控件及输入能力。主干 DeclarativeSurface 已有对应用户控件，故这是
实际实现缺口，不是缺少手测。首先应完成按钮测量/绘制、disabled、焦点、指针/
键盘输入及已有事件提交路径的纵向闭环，再按实装情况更新能力报告。
Image/Icon 可见节点和 JavaScript runtime 准入同样尚有确定缺口；不能因协议
枚举存在就视为迁移完成，也不能用重新引入 WebView 绕过用户的原生架构目标。

## N08 模糊源投影的独立平滑取样

局部 blur 的 D2D 读回现在显式选择线性插值，不再继承底图/擦除的最近邻取样。
Layout 拥有插值模式，默认仍为最近邻；仅局部 blur 准备入口设为 LINEAR，避免
共用 readback 时顺带改变底图像素、擦除物化或普通导出的既有行为。不增加新的
CPU 中间缓冲，仍沿用 WIC 输出/上传/逐行读回预算和取消检查。

主干每个效果新建 Canvas context，未关闭 imageSmoothingEnabled；
[HTML Standard](https://html.spec.whatwg.org/multipage/canvas.html#image-smoothing)
定义其初始值为 true。此次补齐平滑取样行为，但不把 D2D 双线性认作浏览器所有
缩放比例和滤波实现的逐字节等价物，尤其缩小时仍需实际浏览器基准验证。

新增两项真实读回/生产准备回归：黑白两像素放大时默认路径保持最近邻，显式
线性产生中间灰阶；局部模糊结果与平滑源卷积对应，并与最近邻源卷积有可检测
差异。卷积回归使用多行图，避免单行透明邻域衰减把插值差异量化到一个字节内，
并明确断言与最近邻结果不同。两项定向回归通过；全量库 2170 passed、28 ignored；
隐藏 Win32 启动/保存回归 1 passed（4.41 秒）；Clippy all-targets、fmt、2852 文件
有效行数门禁与 diff 检查通过。本轮未提交推送、未制作发行包。

## N08 局部模糊遮罩缩放与旋转几何

局部 blur 资源现记录持久尺寸 / 局部画布逻辑尺寸的双轴比例。绘制先建立完整
圆帽/圆连接笔迹几何，再围绕效果框左上角做非等比变换；遮罩与纹理共同压回
持久框，避免只缩放笔宽或重复施加宿主变换。比例包含逻辑 ceil 和输出像素 ceil
两步。白色覆盖只填主干 ceil 后逻辑范围，不填输出像素 ceil 额外产生的透明边。

局部效果以持久框中心旋转，普通非局部标注仍保持既有笔迹中心。源投影和完整
效果框的裁切改为矩形几何 layer，防止非直角旋转后 AxisAlignedClip 扩成包围框。
此行为已核实 [Microsoft Direct2D 变换文档](https://learn.microsoft.com/en-us/windows/win32/direct2d/direct2d-transforms-overview)。
所有 layer guard 在错误路径仍逆序释放，源颜色裁切在绘制白色覆盖前释放。

新增实际 PNG 回归覆盖 90° 持久框中心、分数框的遮罩位置，以及 45° 源投影外
只保留白色覆盖；另验证非等比密度/双重 ceil 的缩放参数、非法比例拒绝和 wash
透明填充边界。定向 blur 组阶段检查 76 passed，之后另加参数/边界测试；最终
全量库 2168 passed、28 ignored；隐藏 Win32 启动/保存回归 1 passed（3.86 秒）；
Clippy all-targets、fmt、2851 文件有效行数门禁与 diff 检查通过。
本轮未提交推送、未制作发行包。

资源边界：分数效果增加一个 transformed geometry；局部绘制最多同时具有笔迹、
效果框、源颜色三个 layer，tile 路径还有逐块 ownership layer。没有新增整图 CPU
像素副本，但 D2D 内部层成本不由 pixels::reserve 直接计量，不能声称零开销或
已经验证实际峰值。kernel、浏览器离屏 rasterization/插值、非等比多段路径与旋转
组合以及大局部纹理 tile 仍需继续验收；几何公式对齐不等于全面像素一致。

## N08 持久效果框的局部模糊导出

工作流烘焙已从完整可见框卷积切换到单个持久 effect 框：按稳定 annotation ID
读取原始 x/y/w/h，保持恢复排序与原 JSON 顺序无关。禁止通过 points bounds
猜测被传播/裁剪后的持久框；缺失或非法框明确失败。读取器复用 8 MiB 标注输入和
4096 元素上限，不复制 points；Cow 字符串兼容 JSON 反转义后的身份和 type。

源方向只恢复一次；局部投影沿用主干的 contain 或现代 crop 直接像素偏移合同。
局部画布先 ceil 逻辑尺寸，再按输出密度 ceil 像素尺寸。卷积只读取该局部画布，
框外邻域透明；不同框不再仅凭 strength 共享纹理，相同框与取整半径仍共享。
纹理 frame 只加一次 editor 可见原点。原有非局部 BlurImage 默认行为不变。

绘制端使用两个 RAII clip：外层约束整个持久效果框，包括白色覆盖；内层只裁
源投影的模糊颜色，在白色覆盖前释放。D2D/WIC 取样、卷积输出和缓存共享仍受
像素预算约束；后续效果失败或取消只丢弃独立候选，不改原编辑历史或资源。

新增实际 PNG 回归使用与笔迹 bounds 不同的两个持久框，反转 JSON/zIndex 顺序，
验证各自颜色、透明邻域与框外透明像素。资源回归直接调用生产准备入口，验证
局部纹理尺寸、相同框共享、分数参数保留、后续超预算失败、取消及空源投影。
原整框准备实现及其 3 个旧测试已移除，避免测试仍在验证不再使用的生产替身。
原方向/现代裁剪/强度取整回归现在也经过局部生产路径。定向 blur 组 73 passed；
全量库 2163 passed、28 ignored；隐藏 Win32 启动/保存回归 1 passed（3.66 秒）；
Clippy all-targets、fmt、2850 文件有效行数门禁与 diff 检查通过。
本轮未提交推送、未制作发行包。

本节检查点遗留的分数 mask 缩放及 rotation 中心差异已由上节接入。滤波内核、
Canvas 插值及大于 2048 的局部 tile 组合尚需继续验证，不能宣布全部 blur 视觉
一致，更不能宣布完整原生迁移完成。

## N08 模糊绘制强度取整

工作流烘焙在准备导出纹理时按主干 `Math.max(1, Math.round(strength || 8))`
取整。恢复入口已有默认值及正数 1..64 校验，此处只对有效强度取整，再进行
输出密度缩放与缓存查找；不量化原始标注参数、不改编辑修订或保存数据。
同一个整数绘制半径可以共享本次导出的纹理，避免为视觉相同的分数强度重复卷积。

新增真实 PNG 回归覆盖普通及现代 crop 两条路径：7.49 与 7 的结果相同，7.5
与 8 相同，7 与 8 确实产生不同像素，并检查原始分数强度未被修改。
定向回归 1 passed；全量库 2160 passed、28 ignored；隐藏 Win32 启动/保存
回归 1 passed（4.31 秒）；Clippy all-targets、fmt、2846 文件有效行数门禁和
diff 检查通过。未提交推送、未制作发行包；下节统计是此前检查点。

单个 effect 的局部采样与透明邻域现已由上节接入；滤波内核和 Canvas 插值仍待对齐。
局部框必须从持久 annotationState 按 `Stroke::annotation_id()` 关联原始
`x/y/w/h`；不能按 points bounds 重建，也不能将原 JSON 顺序与按 zIndex
排序后的 editor 元素下标直接配对。当前 bake 已限制 revision 为 0 且无 draft，
因此这个关联只适用于未继续编辑的恢复候选。该关联已在上节生产路径实现。

## N08 模糊原始方向与独立裁剪取样

工作流模糊准备现在显式接收恢复裁剪和编辑标志。首次实际 blur 请求时才撤销
底图恢复链的翻转，按主干的未翻转原图取样；无 blur 不为此复制整图，无翻转
则共享像素。传播内容框仍不参与模糊采样。现代裁剪遵守主干
computeEffectSourceProjection 的 `cropRect.xy + 可见标注坐标`，不复用底图的
contain、sourceSize density 或翻转后布局。

纹理只覆盖 layout.visible，尺寸为最终 output，不再为裁剪外不可见的完整源框
分配模糊纹理；同强度仍在本次准备内共享。标注保持 editor 源坐标，纹理 frame
使用相同 visible 原点，避免重复偏移。新增颜色分区 PNG 回归覆盖 X/Y 翻转、
裁剪原像素偏移及传播框忽略，另验证纹理只分配可见输出且原编辑资源不变。

将已有 D2D 取样读回提取为 export_source_sample，擦除与现代裁剪模糊共享有预算、
逐行可取消的路径。入口拒绝已安装边框/圆角的 layout，避免提前合成装饰；不
提前应用底图透明度或标注。原擦除 Frame 仍只在成功读回后交换。

定向模糊像素回归 2 passed；全量库 2159 passed、28 ignored；隐藏 Win32 启动/
保存回归 1 passed（4.18 秒）；Clippy all-targets、fmt、2846 文件有效行数门禁
及 diff 检查通过。本轮未提交推送、未制作发行包。

范围边界：当前仍先模糊整个可见
纹理，而主干按单个 effect 的源投影/局部框执行；annotation 边缘、透明邻域、
滤波内核及 Canvas interpolation 不能宣称全面一致（strength 取整已在上节完成）。后续应处理
effect-local 采样与边缘规则，不把本轮坐标对齐当作完整模糊验收。

## N08 有界源矩形与放大后的局部擦除

旧式裁剪已改为 `image_projection::Projection` 的源/目标矩形映射，先在自然图内
求源交集再按比例映射目标；Full、Region、Empty 显式区分。D2D 直接接收源矩形，
不再构造整张图片放大后的巨大 placement。此前 8192 自然宽度、1 像素裁剪再
放大到 8192 的边界已加入横纵两轴、正反翻转的实际 PNG 回归。

有历史擦除笔迹时，在最终输出尺寸物化底图后再擦除，避免将局部笔迹压回原图
一个像素而擦除整片或完全丢失。物化复用同一 compose_layout/D2D 绘制，不维护
另一套 CPU 取样边缘规则；只有空编辑器和底图进入该阶段，透明度、边框、圆角、
标注仍由最终合成处理一次。WIC 目标、上传、逐行读回及擦除候选都有预算准入；
读回可取消，成功后才交换局部 Frame，无笔迹或全空点集不物化。

新增 3 项实际 PNG 回归和 3 项几何/取消回归。内容像素组 13 passed，包含分数
框加入未命中笔迹时逐像素不变。历史软边断言改为输出坐标下 4×4 覆盖，避免把
旧源分辨率的粗粒度擦除当成正确预期。全量库 2156 passed、28 ignored；隐藏
Win32 启动/保存回归 1 passed（3.69 秒）；Clippy all-targets、fmt、2843 文件
有效行数门禁及 diff 检查通过。
本轮未修改普通导出或全局 CPU 投影算法，未提交推送、未制作发行包。模糊独立
采样、可见 UI 和真实 Loom 验收仍不能由这些底图回归替代。

## N08 旧式自然像素裁剪兼容

工作流底图现在支持只有 cropRect、没有 sourceSize 的旧会话：cropRect 按自然
图片像素取样并拉伸到内容框，不误解释为逻辑源框裁剪，也不改变自然 PNG 输出尺寸。
实现复用完整图片 placement 与底图专属 clip，不另分配裁剪图片。恢复链已有的
像素翻转配合整个目标框的 placement/clip 镜像，保证先选原图区域、再翻转；
来源框越界的部分保持透明。无传播元数据、字段不齐或非正上游尺寸回退完整目标框。
退化/非法裁剪尺寸及超界仿射结果拒绝，原始会话与来源图片保持不变。

新增 6 项像素回归覆盖原图左右/上下非对称取样后翻转、传播内容框的 X/Y 组合
翻转、四边越界及完全不相交的透明、传播字段 fallback 和目标坐标擦除。
验证：内容像素定向组 10 passed；全量库 2150 passed、28 ignored；隐藏 Win32
启动/保存回归 1 passed（4.93 秒）；Clippy all-targets、fmt、2839 文件有效行数
门禁及 diff 检查通过。本轮未提交推送、未制作发行包。

当前完整图片仿射布局仍受 1,000,000 几何上限约束：例如 8192
自然宽度、8192 目标宽度、1 像素裁剪宽度会超过上限。后续须改为有界源取样矩形，
不能简单放宽大坐标而引入 f32 精度损失，也不能宣称所有有效裁剪比例已兼容。
下一实现应将已翻转图片中的源矩形先与自然图求交，再按交集比例映射到目标框，
由 DrawBitmap 同时接收有界源/目标矩形。擦除必须复用同一源原点和双轴比例，
不能只换绘制调用后继续把目标框当作整张原图的 frame；现有 erase_frame 接口
尚不能表达该合同，需要独立投影入口并保护跨边界笔迹，而不是过滤掉框外路径点。
模糊标注的独立取样对齐仍是后续工作，
不把底图兼容解释为整个原生迁移或可见 UI 已验收。

## N08 传播内容框与裁剪底图布局

新增 `export_source_content.rs`，从会话借用的传播元数据读取上游框尺寸与内容框，
先将上游框 contain 到当前目标，再将自然图片 contain 到内容框。未裁剪时位置随
整个目标框翻转；逻辑裁剪时图片翻转、内容框不动，独立 clip 只作用于底图。
边框、栅格及矢量标注不继承底图 clip，PNG 输出尺寸仍遵守自然源分辨率合同。
擦除继续使用最终底图位置映射目标笔迹，不把笔迹随图片翻转。

同时修正无传播元数据时的非等比逻辑裁剪：先构造当前 viewport 再 uniform contain，
不再把在旧 sourceSize 内 contain 的图片逐轴拉伸。传播几何只解析固定字段，
不复制图片或传播历史；异常或超界结果拒绝，不修改保存会话、来源文件或活动编辑器。

新增 4 项实际工作流 PNG 像素回归已通过，覆盖双层 contain、X/Y 翻转、逻辑裁剪
内容 clip、自然输出尺寸、非等比 viewport 和翻转后的目标坐标擦除。
全量库回归 2144 passed、28 ignored。首次回归发现旧测试要求非等比裁剪不产生
留白，这与主干 viewport-contain 公式冲突；已改为断言真实留白和内部像素后重跑通过。
Clippy all-targets、fmt 检查通过；隐藏 Win32 启动/保存回归 1 passed（5.17 秒）。
2839 文件有效行数门禁与 diff 检查通过，新增文件及本轮改动未跨越代码行数限制。

边界：主干模糊标注使用独立的完整 unit 框采样，不读取传播内容框。本轮保留该
分离，不声称模糊在 crop density/flip 下已经全面一致。旧式仅有 cropRect、没有
sourceSize 的 natural-pixel 裁剪兼容分支仍需后续对齐；可见画布和真实 Loom 尚未验收。
本轮未提交推送、未制作手测发行包，完整迁移仍在进行。

## N08 完整快照的图片字段保留合同

定向 wire 回归 1 passed；Clippy all-targets、fmt、2837 文件有效行数门禁及
diff 检查通过。本轮新增测试与协议说明，未重跑全量库/隐藏启动回归，未提交推送。

对 Loom 当前接收实现的只读核查确认：crates/loom_otter_bridge/src/lib.rs 的
sync_workflow 将本次 snapshot 直接转换为 YAML；workflow_store/src/encode.rs
仅将本次存在且非 null 的视觉字段写入 meta，store.rs 原子替换旧文件。没有
“缺失图片字段沿用旧值”的合并步骤，桌面事件处理也只触发刷新。filePath 与
rasterizedAnnotationLayerSrc 不在当前 YAML 视觉白名单，不能假定可从旧 YAML 补回。

原生保留来源工作流的完整图片字段，不照搬主干的签名命中后省略策略。新增 wire
回归跨两个 revision 发送全局和来源工作流：仅几何变化时 src/previewSrc 仍保留，
来源节点 ID 映射正确，每次请求身份独立、ACK 后消息预算释放。本轮不修改 Loom，
也不把静态接收合同核查表述成用户数据的实际丢失复现。

## N08 图感知的工作流烘焙底图

补充定向回归 1 passed：真实 PNG 位于 linshi 的显式授权 Root，上游 Art 正式输出
通过 path 包装引用文件，目标贴图使用不同本地图和 flippedX。回归验证最终像素
确实来自文件、按目标设置翻转且叠加目标透明度，原文件像素没有改动；释放后可
清理本次隔离目录。Clippy all-targets、fmt、2837 文件行数门禁及 diff 检查通过。
此次仅新增测试，未重跑全量库/隐藏启动回归。

上一生产实现检查点：库测试 2138 passed、28 ignored；隐藏 Win32 启动/保存回归 1 passed（5.68 秒）；
Clippy all-targets、fmt、2837 文件有效行数门禁及 diff 检查通过。
新增 4 项纯来源规则测试和 2 项真实像素/权限回归；接线、借用、预算与 Root
读取边界另经过独立只读复核。
本轮未提交推送、未制作手测发行包，完整迁移仍未完成。

新增 session_workflow_sources，将保存会话适配为 graph_image 借用图视图；复用
正式输出解包、能力端口和环路边界，不另写一套遍历。合成底图遵守主干
stickerExportSource 的 display/preview/src 顺序及独立栅格标注层例外，支持
手动 inline 图片、禁止输入、停止中继与本地编辑开关。

正式 workflow worker 将显式 Root 传入准备入口。本节点底图复用已解码像素，
其他引用逐张加载、应用目标翻转并释放；不会按上游路径自动扩权或获取 URL。
参数与正式输出视图保留解析字节预算，选中的图片引用只在回调内借用。

纯来源测试覆盖规则；真实 WIC/原生合成测试以不同颜色像素验证底图选择，并
验证上游正式输出中的未授权文件路径被拒绝。没有以交换 source/preview 顺序
替代图解析，也没有将本地回归当作真实服务或整套可见 UI 验收。

## N08 归档阶段图片快照释放

验证：库测试 2132 passed、28 ignored；隐藏 Win32 启动/保存回归 1 passed
（6.60 秒）；Clippy all-targets、fmt、2834 文件有效行数门禁及 diff 检查通过。
未制作手测发行包、未提交推送，整体迁移尚未完成。

归档 worker 原来持有完整 Ready 至会话暂存结束，包含归档阶段不使用的烘焙图片
快照及编码租约。现在在 worker 开始发布资源前消费为私有 ArchiveInput，释放
快照，只保留精确原文、采用/索引计划和原文预算租约。该类型没有网络发送接口，
归档提交后的新 revision 仍须重新准备；没有跳过输入验证或改动提交顺序。

独立预算回归验证图片租约先归还、原文租约仍持有、原文摘要仍匹配，最后释放
输入后预算归零。这证明资源生命周期，不是进程工作集或整机峰值内存的实测。
本项也未消除归档前后的两次图片准备，后续优化不能以发送旧 revision 为代价。

## N08 来源采用持久化与目录选择凭证

验证：库测试 2131 passed、28 ignored；隐藏 Win32 启动/保存回归 1 passed
（4.88 秒）；Clippy all-targets、fmt、2834 文件有效行数门禁及 diff 检查通过。
本轮未制作手测发行包、未提交推送，完整迁移仍未完成。

对照主干 syncService 的来源采用行为，修复原生 Plan.adoptions 只有生成者、没有
生产消费者的缺口：Batch 保留采用列表，归档候选将 originWorkflowId/originNodeId
与图片引用、索引及 revision 一起提交。后续普通保存使用新 Source 合并，不需要
额外维护窗口级来源副本。回归真实写盘、回读、移除连线后再组装保存候选，验证
贴图仍归属于采用的工作流，未知字段保留。

同步目录选择现在持有打开前的 Receipt；对话框返回后重新检查窗口、工作区、
保存基线与 Loom 目录身份，变化则拒绝启动。隐藏 HWND 回归在选择期间撤销目录，
确认未创建周期或自动同步配置，不连接用户服务。

## N08 应用级归档至同步成功链路验证

最新库测试 2130 passed、28 ignored；隐藏 Win32 启动/保存回归 1 passed
（5.88 秒）；Clippy all-targets、fmt 和 2834 文件有效行数门禁通过。
本次未制作手测发行包、未提交推送，完整迁移仍未完成。

新增真实隐藏 HWND 的 Application 集成测试：先保存合成图片会话，显式准入
临时归档目录及固定证书测试目录，推进完整 Cycle。测试 TLS/WebSocket 服务
在返回成功 ACK 前读取磁盘，验证请求 revision 与已提交会话一致、图片文件
存在且归档索引引用一致。随后验证成功数量、当前性、保存基线交换和结果只消费一次。

测试只使用 linshi 隔离数据与本地测试服务，不连接用户 Loom；生产可见菜单、
已安装 Loom 服务联调及完整功能/资源验收仍未完成。下面各节是阶段历史，
其中“尚未接线”等描述仅代表该阶段，不覆盖本节与最新入口实现的状态。

## N08 正式菜单与保存后同步入口

本轮库测试 2129 passed、28 ignored；隐藏 Win32 启动/保存回归 1 passed
（5.95 秒）；Clippy all-targets、fmt、2833 文件有效行数门禁及 diff 检查通过。
未制作手测发行包、未提交推送；完整原生迁移和真实服务成功链路仍未验收。

新增“同步工作流（选择归档目录）…”和“停止保存后自动同步”。前者要求当前
工作区已保存且 Loom 能力目录已准入，目录选择明确说明写入图片并启用保存后
同步；选择成功启动周期，权限仅保留在当前 Record。取消选择无副作用，停止
只取消后续保存意图，已启动周期继续完成。

正式保存 UI 提交成功且不在退出/打开流程时，合并最新 Source 为待同步意图，
由已有唤醒入口调度。源基线变化/未保存时不发送旧意图；周期完成消费部分成功
报告和归档恢复警告，后续自动成功不反复弹窗，失败不自动重试。隐式归档提交
不触发这条保存钩子，避免递归归档。

隐藏 HWND 回归通过真实命令排队与注入目录/通知出口验证配置、保存意图合并、
自动复用授权目录、停止后不再排队；在准备阶段撤销目录以确保不访问用户 Loom。
正式可见菜单和真实 Loom 完整成功链路仍待人工/集成验收。

## N08 应用级归档与同步周期协调

最终库测试 2128 passed、28 ignored；隐藏 Win32 启动/保存回归 1 passed
（4.19 秒）；Clippy all-targets、fmt、2831 文件有效行数门禁及 diff 检查通过。
本轮未制作发行包，未提交推送，完整迁移仍未完成。

新增 Record 所属 Cycle：PreparingArchive → Archiving → PreparingSync → Syncing
→ Finished，复用已有任务和 Receipt，不在 UI 中执行图片烘焙/网络等待。归档提交
后重新准备刚写入的 revision，再启动逐条准入发送；完成消费保留同步报告、当前性
与归档 Published，恢复警告不被吞掉。失败不重试，未消费周期阻止重复启动。

正式 process_workflow_sync 在服务同步 owner 前后推进周期，解决准备/归档完成
和最终同步完成的唤醒接线。入口仍要求显式 Root；普通保存后自动触发尚未启用。
隐藏 HWND 回归真实推进至归档提交，再撤销能力目录，验证新 revision 重准备被
拒绝、归档结果保留、没有启动网络。只操作 linshi 合成会话并恢复夹具，不宣称
该失败路径测试证明了带真实 Loom 服务的完整用户成功链路。

## N08 后台归档暂存与 UI 原会话提交

本轮库测试 2128 passed、28 ignored；最终隐藏 Win32 启动/保存回归 1 passed
（3.47 秒）；Clippy all-targets、fmt、2829 文件有效行数门禁及 diff 检查通过。
未制作手测发行包，未提交推送；完整迁移与自动同步仍未完成。

新增 archive Job：移动工作流 Ready，完成资源批次、会话组装、精确旧字节锁定
与 Replacement 暂存，后台不提交。取消/失败释放暂存和句柄，已发布图片不回滚。
Job 结果保留资源及目录 owner，显式 commit 返回恢复文件信息。

应用 Candidate 可启动归档，UI poll 交付携带原 Receipt 的 Prepared；commit
再次复核，在不泵消息区间替换文件，成功后交换 Source/saved_path/保存 Stamp。
失败不改 UI 基线，错误保留恢复路径。归档改变 revision，不能继续发送旧 Ready。

库回归真实验证原文件替换、取消和外部写者保护。隐藏 HWND 回归另验证 UI 基线
交换与迟到编辑拒绝；仅针对经路径核验的 linshi 合成会话，并恢复夹具供后续
恢复测试使用。普通保存后的自动触发、新 revision 工作流重准备和发送仍未串联。

## N08 归档资源批次与下一修订会话候选

最终完整库测试 2126 passed、28 ignored；Clippy all-targets、fmt、2825 文件
有效行数门禁及 diff 检查通过。本轮未重跑 HWND 启动或制作发行包，未提交推送。

新增资源批次：精确匹配工作流 Plan 的原始保存数据，借用解析 source/preview，
逐张发布 inline 图片并保留句柄，再用处理后的引用生成完整工作流索引。普通
路径/URL 原样保留，不隐式访问；这不宣称普通引用已被复制到归档目录。

Batch 消费后构造 Prepared，安装引用和索引，revision 只增加一次，保留未知
根字段、窗口字段与拓扑，并复用有界编码/解码校验。Prepared 只借出候选，
图片句柄持续保护至提交结束。测试真实执行资源发布、索引安装和 Staged 新文件
提交；批次失败不交付半成品，但此前已发布资源不会回滚删除。

尚未接入正式工作区的归档后台任务、原会话替换和 UI Receipt 基线交换；测试
新文件提交不代表用户原会话已完成原子更新，也没有启用自动同步或资源 GC。

## N08 归档图片的实际无覆盖发布

最终完整库测试 2124 passed、28 ignored；Clippy all-targets、fmt、2822 文件
有效行数门禁及 diff 检查通过。本轮未重跑 HWND 启动测试或制作发行包。

显式 Root 新增 publish_image/publish_inline：inline 复用预算化 Base64 解码，
WIC 验证格式/尺寸后释放解码像素，按主干 sanitizer 与 FNV-1a 64 位指纹命名。
文件后缀保持 png，但保留原编码字节，不冒称全部重编码为 PNG。同目录暂存并
无覆盖提交；同名文件或竞争发布结果必须逐字节匹配，损坏/冲突不覆盖、不删除。

Published 保留仅共享读的文件句柄，供上层会话提交期间防止改写/删除。取消
不会删除已发布资源；多资源发布与会话提交仍非全有或全无事务。测试只在 linshi
创建合成图片，验证真实落盘、inline 复用、句柄保护、损坏拒绝、取消、并发去重
与临时文件回收。未接入工作流资源批次/会话提交，没有访问用户资源目录或 GC。

## N08 完成结果凭证与归档索引候选

本轮完整库测试 2120 passed、28 ignored；隐藏 Win32 启动/保存回归 1 passed
（3.59 秒）。随后将归档解析收窄为只读取索引，最终索引专项 2 passed；Clippy、
fmt、2820 文件有效行数门禁及 diff 检查通过。未制作发行包或提交推送。

修复同步完成交接时 Receipt 被释放的问题：Completion 随结果保留保存基线、
Guard 和 Loaded 身份，领取后仍须复核。过期结果不能暴露可写回的 Ready，但
部分成功报告仍保留。隐藏 HWND 保存回归用显式合成报告验证消费一次、未消费
阻止覆盖、同字节不同 Source 失效；此夹具不代表网络已经确认成功。

只读核对主干归档合同：索引位于 session.json 的 workflowAssetArchiveIndex，
资源位于 images，先资源/会话保存成功再同步网络。新增有界索引候选转换：
按提示整体替换命中工作流的 nodes，保留未命中的工作流；字段使用 stickerId、
updatedAt、src、previewSrc，version 0 修正为 1。接收已发布的 source/preview
引用，拒绝尚未归档的 inline 图片，校验精确来源/标识/大小/取消。只移动旧索引
并计数编码，不复制完整编码缓冲；尚未写资源或提交新会话 revision。

不能现在启用保存后自动发送：资源归档 owner 与耐久提交接线仍缺失，提示与
内存索引均不等于归档完成。本轮未访问用户资源目录或修改 Loom。

## N08 同步 worker 与 UI 逐条准入握手

本轮库测试 2118 passed、28 ignored；真实隐藏 Win32 启动/保存回归 1 passed
（3.94 秒）；Clippy all-targets、fmt、2817 文件有效行数门禁及 diff 检查通过。
未制作手测发行包，未提交推送；当前仍不是完整自动同步/归档验收。

新增同步 Job，移动 Ready 与固定 Identity 进入后台，顺序派送每条连接完成后
通过容量为 1 的邮箱请求 UI 准入。应答为一次性 owner，五秒未应答即拒绝，
每 100 ms 检查取消；任务析构取消并 join，不遗留发送线程。完成返回原 Ready
及部分成功报告，取消不会覆盖已确认数量。该 Job 不自动恢复或重试失败批次。

应用新增 Candidate::start_sync 和 UI Sync::service：启动先复核 Receipt，
每次准入再复核保存基线/Guard/Loaded 身份，过期取消。准入是当前请求的授权
线性化点，不承诺远端提交时工作区永远不变，不伪造已发请求回滚。真实隐藏
保存回归验证过期 Candidate 不得启动同步 worker。同步 owner 归当前 Record，
正式 pending_windows 在处理能力目录后轮询握手/结果；未消费结果阻止新批次，
避免丢失部分成功证据。保存后的自动触发、结果消费及归档提交仍未接通，不能
宣称用户自动同步已启用。

## N08 工作流批次顺序派送与部分成功

最终完整库测试 2115 passed、28 ignored；Clippy all-targets、fmt、2813 文件
有效行数门禁及 diff 检查通过。新增测试含三种真实 TLS/WS 批次场景，以及
取消/编码失败不建立连接的验证。本轮未重新运行 HWND 启动或制作发行包。

新增后台 dispatch，将批次编码、固定身份连接、逐条准入和单条交换接通。每次
连接完成后重新请求准入；仅匹配成功回执推进。报告区分已确认数量、失败原因
及当前尝试是否可能到达远端；中途失败不回滚已确认项，不自动重试，待确认项
保留相同 ID。取消或编码失败发生在连接前时不建立连接。

应用接线核查确认：保存基线必须在 commit_disk 成功交换后才可触发准备，
pending_windows 的现有 execution_wake 可以承接结果；Receipt 中 Loaded 的
Rc 身份必须保留。尚未将 dispatch 接入正式应用：逐条门禁需与 UI 握手，禁止
后台直接读取 Application/Rc，且不能把一次授权描述成全批永久有效。

## N08 工作流控制请求 TLS/WebSocket 交换

本轮完整库测试 2113 passed、28 ignored；随后仅按 Clippy 建议等价简化 match，
最终源码的两项交换测试复跑通过（含四种真实 TLS/WS 场景），Clippy all-targets、
fmt、2811 文件行数门禁和 diff 检查通过。本轮未重跑 HWND 启动测试或制作发行包。

只读核对本地 Loom 的 otter_protocol_dispatch.rs WorkflowSync 分支，成功响应
包含协议、requestId、status 和 data.workflowId。新增原生单次交换，复用固定
证书 BridgeSocket，限制响应为 1 MiB、总等待五秒、每次读最多 250 ms 后检查
取消。发送副本独立准入预算，连接及待发送缓冲先销毁再归还预算；远端任意
错误文本不透传日志/UI。无关事件/请求不能成功确认，错误工作流或状态直接失败。

交换不自动重试或推进批次；网络超时仍可能已远端提交，需由上层处理。实际
用户调度、发送前 receipt/授权复核、归档提交仍未接通。TLS 回环测试不访问
用户发现配置，不连接运行中的 Loom，也不修改 Loom 仓库。

## N08 惰性工作流请求批次

本轮最终验证：完整库测试 2111 passed、28 ignored；真实隐藏启动/保存集成
1 passed（5.00 秒）；Clippy all-targets、fmt、2809 文件有效行数门禁和 diff
检查通过。未提交推送、未制作手测发行包，完整迁移目标仍未完成。

工作流 Ready 新增 batch 入口，先核对精确保存来源，再按全局 hook-live、来源
工作流依次准备控制请求。批次借用快照，不建立所有消息的大字节副本；只保留
一个待确认消息，编码前按配置上限预留预算，消息释放时归还租约。请求使用
系统随机源构建 UUID，重试保留同一 ID/字节；仅匹配 ID 的显式确认推进。
取消、预算失败和错误确认不推进游标；丢弃批次不修改工作区或来源元数据。

隐藏保存链现验证真实 Ready 到第一条控制请求及丢弃后的预算回收，不伪造远端
确认。此处仍无正式调度、网络发送或资源归档提交；acknowledge 是发送 owner
验证成功回执后的内部入口，不是接收未验证网络 JSON 的入口。

## N08 Loom 工作流控制请求编码

对照主干 action_dispatch.rs 的 SyncWorkflow 分支，新增全局及来源视图的
encode_request：method 固定 loom.hook.workflow.sync，params 使用 requestId、
workflowId、snapshot，而非前端 sync_workflow action 的 workflow_id 包装。
编码借用快照及组件视图，不复制大图片树，不修改节点来源。requestId 的唯一性
由后续发送 owner 负责；编码入口校验非空、字节上限和控制字符，serde 负责转义。

统一有界编码器限制整个消息（包括 envelope），支持取消，并使用有界几何增长
避免逐片段扩容复制。旧 encode_payload 合同保留。新增 2 项回归验证准确协议
形状、全局/来源 ID 重映射、请求 ID 转义及拒绝、精确字节边界和取消。
最终库测试 2109 passed、28 ignored；真实隐藏启动/保存集成 1 passed；Clippy、
fmt、2807 文件行数门禁及 diff 检查通过。尚未实际发送或验证远端 ACK，完整
调度/归档/授权发送仍未完成；本轮未制作手测发行包或发布。

## N08 Application 保存基线上的工作流准备入口

Application 新增准备、取消和领取入口，任务单槽归当前会话 Record 所有。只允许
有已提交路径且保存状态未变化的工作区进入；UI 只转交共享 Source，完整字节复制
及版本读取在 worker 进行，复制的缓冲有独立预算租约并随 Ready 保留。

结果领取和后续使用都可复核 Receipt：保存基线存储身份、保存状态、工作区 Guard、
能力目录身份和精确来源摘要必须匹配，不能只比较 revision。替换会话时任务 owner
随旧 Record 取消/回收；完成不等于网络发送资格。仍未接通用户触发/自动调度、
资源归档提交或实际授权发送，不应将隐藏回归调用当作正式同步已启用。

本轮运行真实隐藏 Win32 启动/保存回归，验证已提交状态、单槽、取消、工作区变化、
结果交付后再变化、同内容不同 Source owner 和能力目录变化。测试夹具显式使用
已绑定 Art 的匹配目录，不放宽生产目录过滤。共享源 worker 新增预算/所有权测试。
最终完整库测试 2107 passed、28 ignored；隐藏启动集成 1 passed；Clippy、fmt、
2805 文件行数门禁及 diff 检查通过。整体迁移未完成，未制作手测发行包或执行发布。

## N08 工作流后台准备任务

新增 WorkflowInput/WorkflowJob/WorkflowReady，后台从指定版本的会话准备真实图片、
过滤参数、全局快照及来源采用/归档提示。资源读取只接受调用方显式准入的 Root
owner，不从会话路径推导权限。Ready 保留快照租约及移动取得的原始字节，提供
精确来源匹配和借用的来源组件视图，不复制完整会话到另一份结果。

任务有取消令牌、一次完成通知、单次结果消费和 panic 转换；取消后即使计算已经
完成也不交付成功结果，Drop 取消并 join，不遗留无主工作线程。WIC 调用不可强制
中断，仍须在其返回边界回收。完成通知不是持久化确认或发送授权。

新增 4 项回归覆盖真实 worker PNG/来源计划、版本不符、资源目录准入、完成后
取消、单次通知/消费、Drop 回收和 panic。filePath 元数据只保留不自动读取，
真实 src 文件引用才进入受管根准入。最终完整串行测试 2106 passed、28 ignored；
Clippy、fmt、2803 文件行数门禁及 diff 检查通过。应用调度入口、归档提交、授权
发送及运行验收仍未接通，整体迁移未完成。本轮无新 EXE 或发布操作。

## N08 遗留图片旋转字段兼容纠正

重新核对主干 stickerEditing.ts、stickerCompositeRenderer.ts 和导出调用链：
imageEditState.rotation 虽仍在类型中，但图片合成器不读取它，不能把它列为尚待
新增的整图旋转能力。已移除原生烘焙针对非零遗留字段的整批拒绝，既不旋转源图，
也不改写原会话。标注自己的 rotation 仍由现有标注几何/绘制路径处理。

新增非对称图片真实 PNG 回归，验证 0/90/180/270 遗留值均不改变尺寸、像素或
原始会话。样式失败测试改为真正无效的边框颜色，未取消能力缺失/取消检查。
最终串行测试 2102 passed、28 ignored；Clippy、fmt、2801 文件行数门禁和 diff
检查通过。下方将整图旋转列为待开发的历史记录不再适用；完整同步调度/归档/
授权发送及运行/视觉验收仍未完成，整体迁移继续。本轮无新 EXE 或发布操作。

## N08 工作流美化包装接线

工作流内容合成后现支持美化包装：主干七种背景预设/默认背景回退、像素留白、
圆角白底卡片、预乘图片叠加和下移高斯阴影。内部节点的逻辑宽高与标注坐标不变；
直接从合成 WIC bitmap 读像素，不经过一次 PNG 编码再解码。关闭美化保持原路径。

圆角使用原生 4×4 子像素覆盖，阴影复用有界双轴高斯核。依据
[WHATWG Canvas 阴影规则](https://html.spec.whatwg.org/multipage/canvas.html#shadows)，
标准差为 shadowBlur/2，颜色 alpha 在模糊后应用；保留超出输出下边界的阴影源，
再裁剪卷积结果。像素复制、阴影源和输出均预算准入，逐行取消，非法/过大尺寸
失败不返回候选。未宣称与浏览器抗锯齿或无限高斯核逐像素一致。

新增 5 项回归覆盖透明内容白卡片、预乘叠加、圆角/渐变、阴影方向和小数留白、
取消/边界，以及真实工作流 PNG 美化开关与节点几何保持。最终串行测试
2101 passed、28 ignored；Clippy、fmt、2801 文件行数门禁及 diff 检查通过。
旋转、完整同步调度/归档/授权发送及浏览器视觉/人工验收仍待完成，整体迁移未完成。
本轮无新 EXE、提交、推送或合并。下方为历史检查点。

## N08 自然源模糊资源接入工作流烘焙

工作流烘焙现从原始自然底图准备双轴密度模糊，再安装到独立导出快照，移除此前
对所有模糊笔迹的统一拒绝。同一窗口相同强度共享一次卷积结果，底图仅惰性投影
一次；不混入后来绘制的边框/标注，也不受底图透明度影响。准备失败或取消时
整个候选丢弃，不修改恢复编辑器、修订号或撤销历史。

贯通测试暴露并修复了非等比拉伸裁剪的问题：自然输出尺寸及原始 contain 放置
必须来自持久 cropRect/sourceSize，再转换到编辑器逻辑坐标；不能在归一化源框
里重新 contain 或重算统一密度。Frame 现在将这套信息同时交给擦除、模糊和最终
合成。测试保护原本应输出 20×12 而不是 24×12 的场景，以及不应出现的透明留白。

新增 4 项回归覆盖自然资源/同强度共享、后续效果失败不部分安装、真实工作流
PNG（包含裁剪）和拉伸裁剪几何。最终完整串行测试 2096 passed、28 ignored；
Clippy、fmt、2798 文件行数门禁及 diff 检查通过。当前覆盖原生准备与合成，尚未
完成浏览器逐像素对照或人工验收；极端标准差仍受内核上限约束并明确失败。
美化、旋转及完整同步调度/归档/授权发送仍待完成。整体迁移未完成，无新 EXE 或发布。

## N08 模糊纹理像素与逻辑框解耦

BlurImage 新增可验证逻辑 frame，默认仍是像素尺寸；with_frame 只修改采样映射，
不复制共享像素。整图和大图分块绘制均按独立横纵密度映射逻辑框，分块可见范围
反算源像素，保持固定源网格及插值邻域。无效/下溢尺寸拒绝，不生成除零映射。

新增逻辑框共享/验证及真实整图 PNG 像素测试，扩展实际 WIC 分块与整图对照，
覆盖偏移、非等比尺寸、分数视口缩放和重复缓存使用；旧测试的旋转/接缝范围不变。
最终完整串行测试 2092 passed、28 ignored；Clippy、fmt、2797 文件行数门禁及
diff 检查通过。当前已补齐绘制映射，工作流自然源效果准备及调用仍待实现，
不能宣称源分辨率模糊已完成。整体迁移继续，本轮无新 EXE 或发布操作。

## N08 源密度双轴模糊内核

新增 blur::region_scaled，将逻辑强度分别乘以横纵像素密度后执行可分离高斯卷积，
避免将低分辨率模糊图直接放大。仍使用透明图外边界、浮点环形行缓冲与逐行取消；
双轴核/临时缓冲/输出均先预算准入，等轴共用一份核。实际标准差上限为 1024，
超出时拒绝，不钳制后冒充正确强度。

旧 region/region_precise 的 1–64 合同和核权重浮点计算顺序保持不变；新入口允许
缩放后的亚像素标准差，并处理极小值平方下溢。新增 3 项回归验证独立双轴脉冲
参考、等密度子区域兼容、非法密度及取消。最终串行测试 2090 passed、28 ignored；
Clippy、fmt、2797 文件行数门禁和 diff 检查通过。

这是源分辨率模糊的内核前置，不是功能接线完成：BlurImage 的像素/逻辑框映射、
自然源效果资源准备及烘焙调用仍待接入，当前工作流继续明确拒绝未准备的模糊。
整体迁移未完成，本轮无新 EXE、提交、推送或合并。下方为历史检查点。

## N08 批量烘焙复用已解析节点

工作流内容回调现在将已解析 Window 直接交给内部烘焙入口，不再为每张待烘焙
贴图调用 session_document::decode 解析整份会话。公共单张入口仍先解析并按
UnitId 查找，内部入口不对外接受任意 Window。此优化消除了随待烘焙图片数量
重复的整份 JSON 解析，但仍有其它准备阶段解析和逐节点查找，不宣称整个管线
只解析一次，也未作时间/内存数值承诺。

新增真实多图回归验证不同节点的透明度、圆角和身份不串用，公共入口仍拒绝不存在
的节点。最终完整串行测试 2087 passed、28 ignored；Clippy、fmt、2796 文件行数
门禁及 diff 检查通过。整体迁移未完成，美化/源分辨率模糊和完整同步链仍待实现。
本轮无新 EXE、提交、推送或合并。下方为历史检查点。

## N08 历史内容擦除的源分辨率恢复

工作流烘焙现物化 contentEraseStrokes 到自然底图，然后再合成边框和独立标注。
路径采用当前可见框坐标，通过 contain 放置与裁剪偏移映射回源像素；原共享图
不变，每笔只替换前一候选，不保留整套整图历史。输入限制为 4096 笔、总计 65536
点，路径和像素均使用预算准入，循环及内核支持取消，失败不返回半成品。

主干 stickerEditTransforms 会缩放历史笔宽，因此新增 apply_history_in_frame
支持有界的历史笔宽，不误用工具滑块 4–96 范围；交互入口保留原约束。单点仍有
最小半逻辑像素半径。沿用原生 4×4 子像素擦除内核，不宣称与浏览器抗锯齿逐字节
一致。四项新增回归覆盖裁剪偏移/源图不变、非法路径/取消、缩放笔宽与交互隔离、
真实工作流 PNG 中底图擦除且边框/栅格标注不受影响。

最终完整串行测试 2086 passed、28 ignored；Clippy、fmt、2796 文件行数门禁和
diff 检查通过。美化、源分辨率模糊和完整同步链仍待完成，整体迁移未完成。
本轮无新 EXE、提交、推送或合并。下方为历史检查点。

## N08 工作流圆角裁剪与合成层预算

源分辨率烘焙现支持 cornerRadius：圆角蒙版覆盖底图、边框和独立标注，使用
当前可见逻辑框，裁剪后的非零源坐标不会改变圆角位置。统一半径过大时缩小到
较短边的一半，保持圆弧而不是分别压扁两轴。普通导出保持不变。

D2D 蒙版层在创建前预留输出尺寸的独立像素预算；RAII guard 在 EndDraw 前
弹出层，随后释放预算。额度不足整次失败，不绕过资源约束。四项新增回归覆盖
输入范围、真实底图/边框圆角、裁剪后独立栅格标注及预算失败/成功释放。首轮
裁剪测试把抗锯齿边缘错误预期为全透明，已调整为几何上完全位于圆弧外的像素。

最终完整串行测试 2082 passed、28 ignored；Clippy、fmt、2795 文件行数门禁和
diff 检查通过。美化、历史内容擦除、源分辨率模糊及完整同步链仍未完成，整体
迁移继续。本轮无新 EXE、提交、推送或合并。以下为历史检查点。

## N08 工作流源分辨率边框合成

真实烘焙现在支持 borderWidth/borderColor，在可见逻辑框内按主干的半线宽内缩
绘制边框，随自然输出密度缩放。绘制顺序保持底图、边框、独立栅格/矢量标注；
边框不继承底图透明度。颜色复用有界纯色解析，不执行 CSS 表达式或访问资源；
无法解析的有效边框样式拒绝准备，缺少宽度或颜色按主干行为不绘制。

新增宽度/颜色输入检查和真实工作流 PNG 像素回归：逻辑 2px 在两倍源密度下
绘制为 4px，透明底图中央保持透明；扩展栅格回归证明标注覆盖边框且透明度独立。
完整串行测试 2078 passed、28 ignored；Clippy、fmt、2794 文件行数门禁及 diff
检查通过。圆角、美化、历史内容擦除、源分辨率模糊等效果和完整同步链仍待完成。
整体迁移未完成，本轮无新 EXE、提交、推送或合并。下方为历史检查点。

## N08 原生烘焙与工作流内容准备贯通

`VectorCandidate::prepare_workflow_snapshot` 现将真实源分辨率 PNG 合成接入
`workflow_snapshot::content::prepare`，统一执行 Art 参数过滤与贴图内容准备。
返回的 `Prepared` 只暴露借用快照，并持有全部新编码图片的预算租约；快照先于
租约释放，来源工作流视图可以继续借用同一内容，不复制大图片。

准备前拒绝继续编辑或含草稿的恢复候选，准备过程沿用取消检查；缺少 capability、
未支持的合成效果或图片编码失败均不返回可发送快照，不回退旧底图。此入口不证明
输入已经持久化，不获取发送权限，不修改来源，也不发起网络请求。应用工作线程
调度、资源归档提交、授权发送及未支持效果仍待完成。

新增 3 项贯通回归覆盖真实自然尺寸/透明度图片、参数过滤、来源 payload、效果和
能力失败、取消及编辑版本隔离。完整串行测试 2076 passed、28 ignored；Clippy、
fmt、2793 文件行数门禁及 diff 检查通过。整体迁移未完成，无新 EXE、提交、推送
或合并。下方为各阶段历史记录，尚未接线的描述以本节为准。

## N08 独立源分辨率 PNG 合成入口

新增 export::source_resolution，复用现有 WIC/D2D 绘制与预算，但使用显式自然源
图片、完整逻辑框、裁剪密度及 contain placement。未裁剪输出自然尺寸，裁剪按
max(natural/source-frame) 密度计算；保留小数坐标，底图透明度不衰减独立标注。
普通导出仍走原尺寸/权威编辑底图路径，未改变其合同。后台编码前后检查取消，
不分配拖拽预览。

VectorCandidate::bake_source_png 接入该入口，仅允许未继续编辑的恢复候选使用
保留原图，避免覆盖新擦除。圆角、边框、旋转、美化、历史内容擦除笔画和未准备的
源分辨率模糊仍明确拒绝，不能将近似图片当作完成；这些效果及工作流内容回调的
最终接线、调度和授权发送仍待实现。

新增 5 项回归：小数框/四种缩放下自然尺寸逐像素一致、裁剪密度与 contain 留白、
显式源不被投影层覆盖、独立栅格透明度，以及真实恢复候选自然尺寸/继续编辑拒绝。
最终源码完整串行测试 2073 passed、28 ignored；Clippy、fmt、2791 文件行数门禁
及 diff 检查通过。整体迁移未完成，本轮无新 EXE、提交、推送或合并。

## N08 来源采用计划与资源归档提示

新增 workflow_snapshot::metadata 生成只读来源采用计划及主干形状的
workflowAssetArchiveHints（workflows -> nodes -> stickerId）。全局提示覆盖所有节点，
来源提示使用已验证的目标节点身份；仅工作流归属变化时产生采用条目，不在准备
阶段修改活动节点。计划携带源摘要与版本，使用前可匹配精确会话；空画布仍保留
空 hook-live 提示。提示不包含图片内容，也不冒充持久 workflowAssetArchiveIndex。
资源索引应在实际文件归档后合并，不能把 hints 直接写成已完成索引。

并行只读审计确认现有 export::png_payload 不能直接作为 source-resolution bake：
session_editor_layers 将原图投影到逻辑框，export 优先 layers.base 且按逻辑框输出，
png_payload 固定底图 opacity=1。可靠接入须独立保留自然源/contain placement/
逻辑框/裁剪密度与底图透明度，不修改普通导出的既有尺寸合同；源身份不匹配时
不能恢复被擦除的旧底图。圆角/边框/模糊源分辨率也须单独对齐，不能成功返回近似
合成图。当前尚未连接真实烘焙、采用/归档提交、调度与网络发送。

新增 2 项回归覆盖真实来源计划/归档提示形状、私有内容不进入提示、空画布、
源摘要隔离和取消。最终源码完整串行测试 2068 passed、28 ignored；Clippy、fmt、
2788 文件行数门禁及 diff 检查通过。整体迁移未完成，无新 EXE、提交、推送或合并。

## N08 工作流参数与图片内容策略

新增 workflow_snapshot::content::prepare 统一入口，按精确匹配的 capability 复用
已有会话参数过滤，去掉 secret/内部控制项并保留 null、false、0、未设置语义。
Art 缺少能力或同 ID 能力重复时拒绝准备，不采用错误能力策略或直接复制原 params。
非 Art 参数仍需通过现有参数结构/大小预算。

图片策略对齐主干 syncedImagePayload：文件支持的图片不附带独立 previewSrc；
有矢量、栅格标注、透明度或编辑状态的贴图必须调用烘焙入口，缩略贴图使用
savedRect 尺寸。烘焙失败不回退旧底图，成功后不携带原 filePath/旧栅格层。
Art 图片不进入贴图烘焙。此入口已调用真实全局 Snapshot 构造，但实际原生烘焙
调用方、应用调度与网络发送仍未接入，不代表已有同步运行链路。

新增 3 项回归覆盖 secret/控制项过滤、缺失/重复能力拒绝、缩略烘焙尺寸、失败
不回退、直接图片字段及编辑触发条件。最终源码完整串行测试 2066 passed、
28 ignored；Clippy、fmt、2786 文件行数门禁及 diff 检查通过。整体迁移未完成，
本轮未生成新 EXE、提交、推送或合并。

## N08 来源工作流连通分量视图

工作流快照增加来源组件规划：按无向连通分量 BFS 统计 originWorkflowId，票数
相同时保留首次出现者；没有来源的分量仅由全局快照覆盖。来源视图使用 originNodeId
（缺省为当前节点 ID）并同步重映射连线，不修改全局身份或应用节点。

同一来源工作流的断开分量合并为一次完整快照，避免多个完整快照依次覆盖丢失
节点；重映射后的节点 ID 冲突使整个规划失败，不静默去重。hook-live 保留为全局
身份。组件只保存索引和 ID，编码时借用已准备的图片/参数/输出大字段；来源解析
只读取身份元数据，不再构建一份图片 JSON 树。

全局快照记录源会话 SHA-256，来源规划同时匹配精确原文和版本，避免同版本不同
文档串用。此模块仍只准备 payload，不执行乐观来源更新、归档、保存或网络发送；
内容准备、授权发送及调度尚待连接。

新增 3 项回归覆盖多数/平票选择、全局身份保持与来源连线重映射、同来源断开
分量合并、目标冲突、同版本不同源拒绝及取消。最终源码完整串行测试 2063 passed、
28 ignored；Clippy、fmt、2784 文件行数门禁及 diff 检查通过。整体迁移未完成，
本轮未生成新 EXE、提交、推送或合并。

## N08 全局工作流快照合同准备

新增 session_document::workflow_snapshot，将会话映射为主干 hook-live 的 nodes、
edges、viewport、documentSchemaVersion/documentRevision，并编码 sync_workflow
所需 workflow_id/snapshot payload。显式匹配非零且 JS 安全整数范围内的保存版本；
保留全局节点身份、正式输出、执行配置和端口连线，不借 originNodeId 改写全局 ID。

参数和图片必须由调用方准备：参数不从原会话自动复制，图片字段限制白名单，
避免将私有根字段或未知内容直接发出。贴图烘焙仍由独立内容准备入口负责，不能
把未编辑底图冒充已烘焙画面。序列化受调用方预算和会话最大字节数双重限制，
取消/内容准备失败不交付部分 payload。

此模块尚未接入应用或网络发送；不证明输入版本已落盘，不更新同步成功状态。
连通分量工作流、烘焙/参数过滤调用方、授权发送和调度仍须继续实现。

新增 3 项回归覆盖精确版本/全局身份与字段、内容隔离、取消/失败和编码预算。
最终源码完整串行库测试 2060 passed、28 ignored；Clippy、fmt、2782 文件行数门禁
及 diff 检查通过。整体迁移未完成，本轮未生成 EXE、提交、推送或合并。

## N08 内联图片正式端口恢复

会话 Candidate 现通过完整输出恢复入口，将 data:image/ 字符串经已有后台 WIC
与像素预算入口物化为正式 Image 端口；重复 Data URL 共享同一解码像素。网络 URL、
文件路径和资源描述符仍保持 JSON 值，不隐式读取。JSON 值和逻辑图片端口共同
遵守现有 256 MiB 结果预算，取消在解析前、逐端口与提交前检查。

全部端口准备成功后一次安装。已有显式恢复预览优先；无预览时按 output_image、
output、首个图片端口选择显示，并保持恢复预览锁定，直到既有执行入口解除。
状态仍为 Idle，不伪造请求完成或 Surface 授权。持久化的图片端口可再次参与
正式图片输入与保存；远端资源解析、自动同步、远端工作流同步和端到端验收仍待完成。

新增 3 项回归覆盖真实 Candidate 图片恢复/共享、取消及损坏图片失败原子性、
显式预览优先和外部引用保持纯数据。最终源码完整串行测试 2057 passed、28 ignored；
Clippy、fmt、2780 文件行数门禁及 diff 检查通过。整体迁移未完成，本轮未生成
EXE、提交、推送或合并。

## N08 图片与 JSON 混合输出保存

对照主干 artDeliveryOutputs 的图片输出字符串合同，原生保存现将正式图片端口
编码为 PNG Data URL，与 JSON 端口写入同一 outputs 对象，不再因含图片而沿用
旧输出。UI 快照只共享 Image 像素 owner，PNG/Base64 在保存 worker 中完成。
WindowCandidate 持有嵌套输出字符串的预算租约直到 JSON 释放，沿用整会话大小
门禁；重复端口、取消或编码失败均不交付部分候选。空宿主不能省略图片输出。

图片输出已可持久化，但当前恢复仍把这些字符串作为 JSON 值，尚未接入正式图片
端口解码和显示；远端工作流同步、自动调度及端到端验收仍未完成。

新增 3 项回归，覆盖像素共享/快照隔离、混合端口真实 WIC PNG 编解码逐像素一致、
重复端口及取消失败。最终源码完整串行库测试 2054 passed、28 ignored；Clippy、
fmt、2778 文件行数门禁及 diff 检查通过。本轮未生成 EXE、提交、推送或合并。

## N08 会话正式 JSON 输出恢复

会话 Window 以借用原文保留 outputs，Candidate 在后台准备 Art 结果时解析并安装
正式 JSON 端口。恢复保持 Idle、无活动请求、无 Surface 版本/授权；resource/stream
描述符只作为数据，不触发网络或磁盘资源读取。显示预览与正式端口继续分离。

解析前受会话字节上限约束，最多 128 个端口，拒绝重复/空/控制字符/超长名称及
非对象输出。全部解析成功才安装；后续节点失败使整批结果候选失败，不发布部分
工作区。活动或已执行的 owner 不接受磁盘结果覆盖。正式空对象恢复后仍按空结果
保存，不恢复历史端口。图片型输出的资源解析和远端工作流同步仍未完成。

新增 5 项回归涵盖纯数据/权限隔离、非法/重复/超额端口、活动及已执行 owner 拒绝
覆盖、真实 Candidate 往返与后续节点失败的整批拒绝。最终源码完整串行库测试
2051 passed、28 ignored；Clippy、fmt、2777 文件行数门禁和 diff 检查通过。
整体迁移未完成；本轮未生成新 EXE、提交、推送或合并。

## N08 正式 JSON 输出进入会话保存快照

对照主干 syncService/sessionStickerPayload 确认：工作流同步先持久化本地会话，
再发送带 documentRevision 的快照。原生保存此前未捕获当前正式 outputs，只会
沿用导入 JSON 的旧值。本轮将全部为 JSON 值的正式端口冻结到保存 Document，
后台编码时整体替换 outputs；resource/stream 仅保存描述符，不读取资源。

Results 显式区分“从未提交”与“正式提交空结果”，后者写入空 map，普通执行开始/
取消不会丢失这个清除语义。保存期间的新结果由已有 OutputStamp/Receipt 复核拒绝，
不以旧快照冒充最新会话。含本地图片的混合端口仍沿用旧保存路径，未在此增量中
解决图片输出编码；远端同步、自动同步调度与恢复结果接线仍待后续完成。

新增 3 项回归，覆盖未提交/空结果、后续执行取消、快照隔离、描述符字段保留、
会话历史端口替换及有输出宿主不得省略，并扩充 Surface 提交断言。最终源码完整
串行测试 2046 passed、28 ignored；Clippy、fmt、2775 文件行数门禁与 diff 检查通过。
本轮未生成 EXE、提交或推送；整体迁移未完成。

## N08 新挂载窗口的一次性节点结果注册

新建 AuthorizedWindow 持有一次结果注册资格，应用在确认授权、显示及激活 owner
后绑定当前 Art 节点。普通结果交付不再首次绑定或换绑；注册成功后，即使普通
Loom 执行撤销 Surface 写入资格，旧窗口也不能重新注册。失败或节点暂不可用
保留资格供后续结算重试，不能把普通结果到达当作重挂载授权。

注册回调只在 refresh 之后执行，再检查应用退出/授权界面状态及节点，避免持有
Results/View 的 RefCell 借用跨越 HWND 同步关闭。新增回归覆盖单次注册、失败/
延后重试，以及撤权、关闭和 owner 在途时禁止调用注册回调。

此增量连接了挂载 owner 与节点写入资格，不代表完整重挂载交互已验收；工作流
同步、真实 Surface 端到端测试及整体功能对齐仍未完成。未生成新 EXE 或提交推送。

本轮完整串行库测试 2043 passed、28 ignored；Clippy（all-targets、warnings denied）、
fmt、2774 文件有效行数门禁及 git diff --check 均通过。忽略项未计作通过。

## N08 Surface 正式结果的既有图传播接线

应用只在新版本实际写入后记录变化节点，全部 Results/View 借用释放后调用
现有 propagate_result，沿用下游通知开关、锁定、祖先防环、手动请求优先及
主循环执行准入。同轮同节点只通知一次，旧/重复版本直接跳过。无新结果时
不分配缓存，首次更新前预留全部通知空间，避免结果已写入但通知分配失败。

复用已有结果版本与图传播回归验证；未新增独立测试。最终源码完整串行测试
2040 passed、28 ignored；Clippy、fmt、2773 文件行数门禁及 diff 检查通过。
核查现有前端 syncService
后确认工作流同步是另一条包含会话/图片/协议的流程，不能用传播通知冒充。
工作流同步、重挂载换绑及真实 Surface 端到端验收仍待完成；整体迁移未完成，
无新 EXE、提交、推送或合并。

## N08 应用层正式输出交付与证据保留

Sessions 只借出成功呈现且交付有效的 Commit，批次另核整批 ACK/观察完整成功。
应用正常结算重查当前服务、窗口授权/显示版本、确认激活 owner 和 Art 节点，
再将输出写入 Results。Commit 保留在收据中，输入门禁按节点当前绑定的已应用
结果版本判断是否解除阻塞，避免清空 Commit 后恢复记录失去结果版本。
普通结果交付只能首次绑定或复用同附件，不能擅自换绑；普通 Loom begin 撤销
后即使请求完成，也不能被迟到 Surface 结果自动重绑，须挂载 owner 显式处理。

新增 2 项集合回归覆盖 Commit 保留/输入解锁、撤权及未呈现拒绝，并扩充换绑
和执行互斥断言。最终源码完整串行测试 2040 passed、28 ignored；Clippy、fmt、
2773 文件行数门禁及 diff 检查通过。图传播/同步、重挂载换绑和端到端验收仍待完成；整体迁移
未完成，无新 EXE、提交、推送或合并。

## N08 Surface 节点原子结果与版本所有权

Results 新增显式附件绑定和 Surface resultRevision，复用现有端口存储。只有
匹配当前完整 binding/实例代际的新结果可原子替换全部 outputs；空 map 清除旧
端口，不套用普通 Loom finish 的端口合并。所有解包/分配失败点先完成再更新
端口、版本、输出凭证和完成状态。旧版本不变更，相同绑定不重置版本；换附件
阻断旧提交，普通 Loom begin 撤销 Surface 写入资格但保留已有输出。

新增 3 项回归覆盖单调替换/空输出、重挂载旧提交拒绝及普通 Loom 执行互斥。
最终源码完整串行测试 2038 passed、28 ignored；Clippy、fmt、2770 文件行数
门禁及 diff 检查通过。
应用当前授权交付、传播/同步及端到端验收尚待完成；整体迁移未完成，无新 EXE、
提交、推送或合并。

## N08 Surface 正式结果的节点值适配

核对现有 src/app/appSurfaceListeners.ts 的 surfacePortValue 和正式结果监听：
先解包三类端口写入节点 outputs，再传播与同步；并非收到资源描述符就下载。
Commit.output_values 按此合同产生独立节点值映射，保留 null/false/0/空串、
空输出及资源/流扩展字段。仍先校验端口类型/数量/重复键，不接受共享内存等
其他协议描述符，不执行脚本、访问路径、下载或订阅。失败保留原 Commit。

新增 3 项回归覆盖标量/空输出/转义端口名、描述符保真、重复端口及未知类型
拒绝。最终源码完整串行测试 2035 passed、28 ignored；Clippy、fmt、2769 文件
行数门禁及 diff 检查通过。节点原子应用、版本管理、传播/同步与真实端到端验收尚待完成；整体迁移
未完成，无新 EXE、提交、推送或合并。

## N08 授权输入生命周期与条件启用

生产 pointer 从始终关闭改为按应用/授权状态启用：创建仍关闭，只有当前服务、
可见、确认激活 owner、可用容量且无未结算收据/待应用输出时可捕获点击。
模态阻塞、退出、服务切换、owner 在途均关闭输入；移交批次前立即关闭。
sync_pointer 复核授权，撤权关闭窗口。相同状态设置不再失效候选，实际切换
仍清除旧按下/候选，避免主循环重复同步吞掉正常点击。

新增 3 项 HWND/集合回归覆盖相同状态保留候选、在途/撤权禁止输入、未结算
收据阻止同附件新点击。最终源码完整串行测试 2032 passed、28 ignored；
Clippy、fmt、2768 文件行数门禁及 diff 检查通过。
输出应用、异常收据处理与完整端到端用户交互仍待完成；
整体迁移未完成，无新 EXE、提交、推送或合并。

## N08 窗口点击候选到应用批次调度

application_surface_input 在正常可见结算中消费已有 HWND 候选；检查服务、
设备授权、窗口当前版本和事件预算后移交整批 owner。Sessions 调度事务先查
容量并预留失败结果位置，启动失败保存完整输入为 NotStarted，再由既有批次
交付结算。不在窗口回调联网，不在退出泵启动新输入，无候选时不准备配置/租约。

新增 3 项回归覆盖容量/关闭前置拒绝、启动失败完整返还和无候选不启动任务。
适配无 std::error::Error 实现的配置/授权错误后，最终源码完整串行测试
2029 passed、28 ignored；Clippy、fmt、2768 文件行数门禁及 diff 检查通过。
生产 pointer 开关
仍关闭，授权输入生命周期和端到端运行待完成；整体迁移未完成，无新 EXE、
提交、推送或合并。

## N08 显式接续原生菜单与应用入口

新增“继续 Surface 批次的剩余操作”菜单，只有当前可接续且窗口审核通过的
候选才启用。菜单仅排队原批次 ID，应用执行前复核服务/设备/配置、退出/新请求
及窗口版本，再进入接续事务并移交 owner。batch_continuation_for 供候选与
执行共享审核，不消费收据；refresh 期间不持有 View 借用，服务切换后不发布
旧候选。不自动接续、不自动批准后续提示、不重放前缀。

新增原生菜单 HWND 回归覆盖启用条件、原批次 ID 排队、命令路由与退出拒绝，
扩充窗口审核不移交 owner 的断言。最终源码完整串行测试 2026 passed、28 ignored；
Clippy、fmt、2766 文件行数门禁及 diff 检查通过。
真实用户菜单操作及生产点击仍待验证/连接；
整体迁移未完成，无新 EXE、提交、推送或合并。

## N08 显式接续收据选择事务

Sessions.pending_batch_resume_for 使用原批次首事件 ID，筛选满足接续边界且
调用方确认窗口可用的收据。dispatch_batch_resume 在消费收据之前检查集合
状态/容量并预留结果空间；取消原位返还，启动成功不重复计预算，启动失败
将完整 owner/收据送入既有批次交付队列，不丢弃前缀决定或自动重发后缀。

新增 3 项回归覆盖候选/取消保持顺序、显式选择后任务收割，以及启动失败
完整返还。首轮测试发现夹具原本未激活而断言错误要求激活，已改为验证状态
原样保留。最终源码完整串行测试 2025 passed、28 ignored；Clippy、fmt、
2765 文件行数门禁及 diff 检查通过。
原生菜单接线和生产点击仍待完成；整体迁移未完成，无新 EXE、
提交、推送或合并。

## N08 接续任务应用集合与混合完成容量

Sessions 已连接接续 worker 的启动、完整批次预算、共享网络容量、正常/退出
等待句柄、取消和收割。结果进入原批次交付/收据/恢复记录；在途不能生成遗漏
证据的恢复 bundle。启动失败完整返还原收据和 owner，不自动重试。
普通批次、决定和接续统一使用 reserve_batch_outcomes 预留全部在途结果，
防止只按其中一种任务数预留导致收割时再次分配。

新增 3 项应用集合回归，覆盖完整预算/退出证据、关闭及容量拒绝完整返还、
三类任务混合完成缓存。最终源码完整串行测试 2022 passed、28 ignored；
Clippy、fmt、2764 文件行数门禁及 diff 检查通过。
显式接续选择和菜单入口尚待连接；整体迁移未完成，
生产点击默认禁用，无新 EXE、提交、推送或合并。

## N08 显式后缀接续后台任务

批次 runtime 新增 continuation::Job，复用共享槽位、授权取消、等待句柄和
显式收割。worker 重查边界及整批身份，只提交原列表的未尝试后缀；网络前
预留 ACK/观察追加空间，原前缀 ACK、决定和输出保留，新结果逐项追加。
观察失败独立标记，不擦除提交事实；外层任务错误与完整批次 Outcome 分开
返还，迟到撤权仅阻止交付，不伪造回滚。Drop 取消并回收线程。

新增 2 项真实本地 TLS/线程回归，覆盖仅发送后缀、HTTP 失败后前缀及恢复
记录保留、迟到撤权，以及已撤权任务保留未尝试后缀。最终源码完整串行测试
2019 passed、28 ignored；Clippy、fmt、2762 文件行数门禁及 diff 检查通过。
接续应用任务集合与
显式入口尚待连接，生产点击默认禁用；整体迁移未完成，无新 EXE、提交、
推送或合并。

## N08 显式接续边界与整批 owner 移交

BatchReceipt.continuation_boundary 只接受确认停止后的成功连续前缀及全未尝试
后缀。前缀 ACK/观察/决定身份与版本须一致，最后确认项必须已批准且被接收；
失败、未确认、重复身份或尝试过的后缀不能接续。窗口重新复核授权/版本后，
begin_batch_continuation 将完整收据与唯一 owner 一并移交，Activation 保持
完整有序批次独占，失败完整返还。本入口不发送、不提前标记任何后缀事件。

新增 2 项 HWND 回归覆盖完整移交/返还、前缀不确定、后缀已尝试与迟到撤权。
最终源码完整串行测试 2017 passed、28 ignored；Clippy、fmt、2760 文件行数
门禁及 diff 检查通过。
接续 worker、应用入口和生产点击链路仍待完成；整体迁移未完成，无新 EXE、
提交、推送或合并。

## N08 批次原生确认与菜单接线

现有确认菜单候选已包含批次待确认项，并跳过当前不可用窗口。明确菜单请求
进入 application_batch_confirmation，复核服务/设备/配置、模态互斥和当前
窗口后显示 Win32 确认框；取消为默认选择，复用不可信文本转义与长度限制。
弹窗返回后重新检查提示期限、授权、退出/服务切换与新请求，再准备决定和移交
owner。取消保留整批；准备或移交失败保留收据/未提交决定，不自动重试后缀。

新增 2 项真实 HWND 配合注入选择器回归，覆盖取消/批准/拒绝不提前消费提示，
以及选择期间过期/撤权拒绝继续。测试不等于真实 MessageBox 人工交互验收。
模块归入现有应用确认模块，避免 application.rs 越过 500 行门禁。最终源码
完整串行测试 2015 passed、28 ignored；Clippy、fmt、2758 文件行数门禁及
diff 检查通过。生产点击
仍默认禁用，显式后缀接续和完整原生迁移尚未完成；无新 EXE、提交、推送或合并。

## N08 批次收据选择与决定启动事务

BatchReceipt.confirmation_ready 统一待确认项的证据审核，窗口仍独立复核当前
授权和版本。Sessions 可按事件 ID 选择有效提示并跳过不可用窗口；分配完成/
异常缓存后才消费整批收据，取消原位返还，启动失败保存 NotStarted 决定证据。
收据转入任务不重复计算未保存预算，不自动批准，也不执行未提交后缀。

新增 3 项回归覆盖候选过滤、取消保留整批、无效选择/容量提前拒绝、完整收据
转入真实线程及证据收割。修正测试夹具错误类型和枚举体积告警后，最终源码
完整串行测试 2013 passed、28 ignored；Clippy、fmt、2757 文件行数门禁及
diff 检查通过。原生批次弹窗、菜单接线和显式后缀接续尚待完成，
生产点击默认禁用；整体迁移未完成，无新 EXE、提交、推送或合并。

## N08 批次决定应用集合与退出收割

Sessions 已接入批次决定 worker 的启动、共享网络容量、按事件数计的未保存
预算、正常/退出等待句柄、取消和收割。普通批次与决定任务共同预留完成缓存，
避免回收后才因分配失败丢失 owner。回填成功沿用批次交付/收据/恢复日志；
回填异常保留完整输入并占用容量，拒绝生成遗漏决定证据的退出 bundle，只有
显式取走接口才移交保存责任，不自动重试或重放后缀。

新增 3 项应用集合回归覆盖已撤权真实线程的退出收割、完整证据与恢复记录，
异常回填保留，以及关闭/满容量启动失败完整返还。完整串行测试 2010 passed、
28 ignored；Clippy、fmt、2756 文件行数门禁及 diff 检查通过。批次确认 UI、
收据选择事务及显式后缀接续仍未连接；生产点击默认禁用，整体迁移未完成。
无新 EXE、提交、推送或合并。

## N08 批次决定结果回填、交付审核与恢复提示

Outcome.into_batch 将选中项的决定与收敛结果并回原批次；完整保留唯一 owner、
其他事件和未提交后缀。不一致输入完整返还，不丢弃证据。逐项观察新增决定
证据，窗口呈现额外要求决定已接收、提交成功且身份/版本一致；已有决定的项
不能再次准备。恢复记录提取脱敏 DecisionHint，不保存可执行正文或授予重放权。

新增 2 项回归覆盖错误索引完整返还、模拟确认成功经真实 HWND 批次交付更新
场景及恢复元数据；扩充真实 TLS 拒绝/取消/迟到撤权回归，验证结果回填后仍保留
决定证据与未提交后缀。修正测试断言提前移动 convergence 的编译问题后，完整
串行测试 2007 passed、28 ignored；Clippy、fmt、2754 文件行数门禁与 diff
检查通过。应用批次决定任务集合、UI 和未提交后缀的显式接续仍待完成；生产
点击默认禁用，整体迁移未完成。无新 EXE、提交、推送或合并。

## N08 批次选中项的决定后台任务

surface_batch_decision_runtime::Job 持有完整 BatchDecisionInput，复核整批身份/
版本及目标 ACK 后，只提交明确选择的那项决定，再观察该原动作。只更新目标
最后 ACK，不重发点击、不自动提交后缀；提交结果、后续收敛和交付授权分别
保存。容量/spawn 失败完整返还，复用共享 Slot/Lease/等待句柄，显式收割释放
槽位，Drop 取消并回收线程。后续收敛异常不会抹去决定 HTTP 已被接收的事实。

新增 2 项真实 TLS/线程回归覆盖拒绝决定后 cancelled ACK、迟到撤权、未提交
后缀保留，以及已撤权任务不尝试决定并返回所有事件。最终源码完整串行回归
2005 passed、28 ignored；Clippy、fmt、2753 文件行数门禁及 diff 检查通过。
结果合并回原批次、决定恢复元数据、应用调度与点击启用仍待完成；无新 EXE、
提交、推送或合并。

## N08 批次决定的唯一 owner 接续移交

begin_prepared_batch_decision 在明确选择后再次核对窗口授权/显示、当前版本、
原 ACK 和提示期限，将完整 PreparedBatchDecision 与唯一 owner 移入
BatchDecisionInput；失败完整返还。Activation.take_batch_decision 允许引用
已尝试原事件，但拒绝已尝试或身份不匹配的决定，整批仍保存有序 ID 独占。
普通批次提交继续拒绝已尝试事件；该入口不发送、不伪造新点击，返还仍需
匹配完整批次列表。

新增/扩充 HWND 回归覆盖准备后过期完整返还、成功移交后单事件互斥、整批
返还，以及准备后撤权/已尝试决定拒绝。最终源码完整串行回归 2003 passed、
28 ignored；Clippy、fmt、2751 文件行数门禁及 diff 检查通过。批次决定 worker、
UI 接续和点击调度仍未完成；无新 EXE、提交、推送或合并。

## N08 批次明确选择后的提示准备

AuthorizedWindow.batch_confirmation_for 按索引复核批次项的窗口授权、当前
版本、交付状态、ACK/观察数量及提示身份/期限。整批因部分完成以 Evidence
拒绝呈现时，有效待确认项仍可读取；其他呈现错误不被当作批准理由。
prepare_batch_decision 消费原提示，返回同时持有整批收据、索引和决定的
PreparedBatchDecision；失败完整返还，不移交 owner、不发网络请求，不拆散
其他事件 ACK/输出或未提交后缀。

新增 2 项真实 HWND 回归覆盖错索引/过期失败保留、明确拒绝后仅消费目标提示、
其他项证据保留、owner 未移交及撤权关闭。最终源码完整串行回归 2002 passed、
28 ignored；Clippy、fmt、2750 文件行数门禁及 diff 检查通过。批次接续任务、
UI 调度和点击生产入口仍待连接，整体迁移未完成；无新 EXE、提交、推送或合并。

## N08 批次恢复元数据与正常退出 bundle

批次结果/收据已展开为逐事件恢复提示：尝试状态、ACK、收敛/输出版本分别
保留，BatchHint 记录首事件 ID、序号、数量和整体呈现阶段。最高候选版本的
实际应用与各项观察版本分开，未尝试后缀不伪造 ACK/收敛。严格拒绝缺项、
错序、重复 ID、绑定/版本不一致及矛盾整体成功。旧无 batch 字段记录仍可读。
不保存 payload、确认正文、场景或凭据，磁盘提示不提供重放/批准能力。

Sessions 的正常退出 bundle 已包含批次事件及未交付 owner 的附件提示，
不再一概拒绝批次；在途任务与不一致元数据仍报错，不提前消费内存证据。
旧附件专用接口仍拒绝遗漏事件。已有后台退出 writer 承接该 bundle。

新增 2 项批次元数据回归，并扩充成功呈现 round-trip 和应用 bundle 验证。
修正测试夹具缺少新增 batch 字段的编译错误后，最终源码完整串行回归
2000 passed、28 ignored；Clippy、fmt、2748 文件行数门禁及 diff 检查通过。
批次批准后恢复、点击调度与完整迁移验收仍未完成；无新 EXE、提交、推送或合并。

## N08 批次应用集合、等待与有界收据

Sessions 已接入批次启动、等待句柄、收割、关闭取消、退出等待和窗口结算。
批次按实际事件数共享 32 条未保存动作预算，不能用一个批次绕过上限；未交付
结果仍占网络容量，已结算收据不占网络槽位。开始前预留完成缓存，窗口交付前
预留收据空间，拒收原位保留，取消仅撤销待交付资格，不改写 ACK/尝试事实。

恢复元数据尚未适配批次，因此 bundle/旧附件日志有批次证据时明确报错，
不会报告保存成功却遗漏动作。应用点击生产入口仍未启用，不交付可发布版本。

新增 2 项回归覆盖 32 事件批次占满共享动作预算、等待/退出收割、停止结果
保留，以及拒收/取消/收据缓存与恢复拒绝遗漏。最终源码完整串行回归
1998 passed、28 ignored；Clippy、fmt、2747 文件行数门禁及 diff 检查通过。
批次恢复日志、批准后恢复与点击调度仍待完成。无新 EXE、提交、推送或合并。

## N08 整批窗口交付与收据

AuthorizedWindow.accept_batch 先按完整事件列表恢复 owner，再独立审查呈现；
错序/不匹配时完整返还 Outcome。只有批次 Complete、ACK 数量与身份一致且
全部成功、逐项观察成功、版本与最高候选快照一致及授权有效时才替换场景。
部分完成、等待确认、缺项、旧版本或撤权不能呈现；替换后再次检查撤权。
BatchReceipt 保留事件、提交/观察/提示/commit 和交付结果，不重复缓存场景。
Windows 集合已提供整批路由接口，尚未连接应用批次任务集合与恢复记录。

新增 3 项真实 HWND 回归覆盖完整批次一次呈现/证据保留，部分或不一致结果
拒绝呈现、撤权，以及错序拒收后完整返还并恢复正确交付。最终源码完整串行
回归 1996 passed、28 ignored；Clippy、fmt、2745 文件行数门禁及 diff 检查
通过。批次批准后恢复与应用缓存/退出日志仍待完成，点击默认禁用；无新 EXE、
提交、推送或合并。

## N08 批次唯一 owner 的窗口移交

Activation.take_batch/restore_batch 保存并核对完整有序事件 ID 列表，批次
独占唯一 owner；复用提交前整批身份/版本/容量检查。单事件、生命周期、
缺项或乱序结果不能取回 owner；原批次允许部分事件未尝试地返还，符合中途
停止的事实。AuthorizedWindow.begin_batch 复核当前授权/显示/版本，
finish_batch 仅恢复 owner，不更新场景、不消费结果证据。

新增 3 项状态机/真实 HWND 回归，覆盖空批次、版本不匹配、并发互斥、错序/
缺项拒绝、部分提交返还、重复提交拒绝及撤权关闭。最终源码完整串行回归
1993 passed、28 ignored；Clippy、fmt、2743 文件行数门禁及 diff 检查通过。
批次批准后恢复、应用集合与整批窗口交付仍待完成，点击默认禁用。无新 EXE、
提交、推送或合并。

## N08 批次待确认提示读取与共同截止

批次观察 Item 在 ConfirmationRequired 时于同一 worker 读取一次提示，独立
保存 confirmation 结果并更新最后 ACK，原收敛状态不被读取失败覆盖。新增
surface_confirmation_until，由批次传入共同截止并限制单次最多 3 秒；开始、
网络返回及校验结束均复核截止，预算耗尽不进入网络读取。单事件旧接口保留。
提示读取不发批准、不重放事件，也不自动继续未提交后缀。

新增 2 项回归覆盖真实本地 TLS 的一次 POST/提示 GET、收敛与提示证据分离、
后缀未尝试，以及已耗尽截止不读取网络且保留 ACK。最终源码完整串行回归
1990 passed、28 ignored；Clippy、fmt、2742 文件行数门禁及 diff 检查通过。
批次批准后恢复、应用集合与窗口交付仍未接入；点击默认禁用，整体迁移尚未
完成。无新 EXE、提交、推送或合并。

## N08 点击批次的逐项收敛观察

批次 worker 已在提交后对有 ACK 的事件前缀逐项 GET 收敛，整批共用 25 秒
截止。提交事实与 observation 分开保存，后续失败不重放 POST；每项保留最后
ACK、结果版本/错误和 commit，未提交后缀不伪造观察结果。场景仅保留最高
revision 的一份，避免重复缓存多个大场景，后读到低版本不覆盖已保存高版本。
该快照只是候选，不代表整批所有动作成功或已经获准呈现。

扩充真实本地 TLS 测试覆盖两次 POST 后逐项 GET、各项版本与最高场景保留，
新增逆序版本回归。最终源码完整串行回归 1988 passed、28 ignored；Clippy、
fmt、2742 文件行数门禁及 diff 检查通过。批次确认提示读取/批准后恢复、应用
任务集合与窗口交付仍待接入；点击默认禁用。无新 EXE、提交、推送或合并。

## N08 点击批次提交后台任务

surface_event_batch_runtime::Job 接管唯一 owner、当前呈现版本与整批原事件，
单 worker 复用共享 Slot、授权取消和等待句柄。容量/spawn 失败完整返还 Input；
完成后仍占槽位，显式 try_join 才释放。迟到撤权保留提交事实与 ACK，但交付
被拒绝；Drop 取消并回收线程，不替代正常保存结果。批次 HTTP 单项异常转换为
Panicked 停止，保留此前已收到 ACK 和当前事件 attempted 状态。

新增 2 项回归覆盖共享槽位满时输入完整返还、已撤权批次无网络尝试、显式
收割/句柄释放，以及真实本地 TLS 提交完成后撤权仍保留 ACK。最终源码完整
串行回归 1987 passed、28 ignored；Clippy、fmt、2741 文件行数门禁及 diff
检查通过。批次收敛/确认恢复/应用集合仍待接入，点击默认禁用，整体迁移未
完成。无新 EXE、提交、推送或合并。

## N08 点击冒泡批次顺序提交接口

新增 submit_surface_click_batch：先完整校验事件未尝试、身份/版本一致、ID
不重复、数量不超过 MAX_DEPTH、合计不超过 64 KiB，再预留 ACK 空间后发送。
复用单事件协议与认证，按原顺序提交，共享 10 秒提交预算；遇到待确认、动作
拒绝/终态失败或提交错误停止。原事件仍由调用方持有，ACK 对应连续前缀，
未提交后缀保持未尝试，不自动继续或重放。Complete 不是动作执行成功证明。

新增 2 项回归：本地真实 TLS 顺序提交/等待确认停止/后缀未尝试/整批重放
拒绝，以及混合身份批次在任何网络尝试前拒绝。最终源码完整串行回归
1985 passed、28 ignored；Clippy、fmt、2739 文件行数门禁及 diff 检查通过。

批次后台 owner、各事件后续收敛/确认与应用调度尚未接入；输出消费、运行中
日志和整体验收也未完成，点击仍默认禁用。没有新 EXE、提交、推送或合并。

## N08 确认候选的当前窗口审核

应用候选选择增加对应 AuthorizedWindow.confirmation_for 审核，关闭/旧场景/
owner 不可用的收据不再占住菜单入口，后续有效提示仍能选中。选择不消费旧
收据；窗口刷新可能关闭 HWND 并重入，因此不跨窗口调用持有 View 借用，发布
候选前重新核对服务和关闭状态。实际对话框与提交仍执行各自的新鲜授权检查。

新增回归覆盖跳过不可用窗口候选、过期收据不调用窗口审核及全部证据保留。
最终源码完整串行回归 1983 passed、28 ignored；Clippy、fmt、2737 文件行数
门禁及 diff 检查通过。真实服务交互、点击启用和完整迁移仍未验收；没有新
EXE、提交、推送或合并。

## N08 待确认候选的时效与积压检查

修正第一条过期/交付失败收据可能长期占据菜单入口、挡住后续有效提示的问题。
EventReceipt.confirmation_ready 统一候选与调度检查：提示身份/ACK/期限有效，
无决定/输出 commit、没有成功收敛或呈现场景且交付成功。候选选择传入当前
时刻，调度在进入 UI 回调前再次读系统时间；时钟失败不采用时间零。旧收据
保留，不因过滤丢失恢复证据；窗口和网络提交仍独立检查授权。

新增 2 项回归覆盖多收据中跳过过期/撤权提示、服务过滤、恰好到期、旧菜单
选择拒绝进入 UI，以及已呈现收据不能重新确认。最终源码完整串行回归
1982 passed、28 ignored；Clippy、fmt、2737 文件行数门禁及 diff 检查通过。
未新增 EXE、提交、推送或合并；真实服务交互与完整迁移仍未验收，点击默认禁用。

## N08 原生确认菜单与应用调度连接

授权菜单新增“处理待确认的 Surface 操作…”，只排队当前服务的一条事件 ID；
应用循环在不持有 View 借用时消费请求，获取匹配服务/设备的租约和配置，
通过共享模态 Scope 打开 Surface 原生确认框。明确选择后复核退出、关闭、
服务及待处理请求，再移交窗口 owner 并启动决定任务。状态变化或移交失败
保留 NotStarted 决定证据，不自动重试；取消保留提示供用户再次显式选择。

新增隐藏 HWND 菜单回归覆盖无提示禁用、有提示排队原事件、命令路由及退出
时拒绝请求。最终源码完整串行回归 1980 passed、28 ignored；Clippy、fmt、
2737 文件行数门禁及 diff 检查通过。确认框、收据转换和任务集合的已有回归
一并通过；尚未进行真实服务上完整人工确认交互验收，不以编译代替该验收。

原生点击仍默认禁用，输出消费、点击批次、运行中日志及完整迁移验收仍待
完成。没有自动批准；无新 EXE、提交、推送或合并。

## N08 选中收据的确认调度事务

Sessions.dispatch_surface_confirmation 按明确选中的事件 ID 处理待确认收据。
关闭、容量不足或没有匹配收据时不调用 UI；在消费提示/移交 owner 前预留结果
缓存。Keep 将取消的原收据放回原位置，Start 将原输入移入决定任务。启动失败
保存 NotStarted 和完整事件/ACK/决定/owner 到统一结果队列，不重试、不丢证据。
原收据先移除，转换仍只占一个未保存动作名额。

新增 2 项回归：取消/缺失事件/容量/关闭前置门禁，以及共享 Slot 已满时启动
失败仍保留原事件、拒绝选择、未尝试状态和恢复记录。最终源码完整串行回归
1979 passed、28 ignored；Clippy、fmt、2736 文件行数门禁及 diff 检查通过。

应用用户入口、模态互斥、退出检查和对话框到该事务的实际调用仍需连接，
原生确认交互尚未端到端验收。点击仍默认禁用，没有自动批准；无新 EXE、
提交、推送或合并。

## N08 收据到决定任务输入的所有权移交

begin_receipt_decision 接受原收据与显式选择后的未尝试决定，成功仅移交一次
owner，保留原 event/ACK，并从窗口读取当前 revision。残留提示、已有决定、
输出 commit、已呈现场景、成功快照或失败交付均拒绝转换；失败完整返回
ReceiptDecisionError 中的收据和决定，不静默丢弃其他执行证据。

扩充 HWND 回归验证过期/已呈现拒绝且原样返回、成功移交后阻止重复取 owner，
以及提交前取消的结果可经统一交付恢复 owner 和原 ACK。最终源码完整串行
回归 1977 passed、28 ignored；Clippy、fmt、2736 文件行数门禁及 diff 检查
通过。测试中错误使用 Preparation 枚举构造器导致的编译失败已修正并重跑。

应用模态/退出/收据选择与任务启动调度尚未连接，真实对话框手测及完整迁移
仍未完成。没有自动批准，点击仍默认禁用。无新 EXE、提交、推送或合并。

## N08 原生确认对话框入口

最终源码完整串行回归 1977 passed、28 ignored；Clippy、fmt、2735 文件
行数门禁通过。新增 2 项文本边界回归，并扩充 HWND 测试覆盖取消保留收据、
阅读期间过期拒绝决定和明确拒绝选择。不弹出自动点击的真实确认框。

prompt_decision 使用 Surface HWND 作为原生 MessageBox owner，默认取消，
仅是/否生成显式决定。展示前与返回后复核当前授权和期限；取消或失败不消费
原提示。远端字段转义控制/双向字符，完整文本超过 8192 UTF-16 单元拒绝
展示，不通过截断隐藏风险信息。本入口不发请求，不移交 owner。

应用模态互斥、退出状态和收据到决定任务的调度仍需接入；真实对话框手动
交互尚未验收。点击仍默认禁用，没有自动批准。无新 EXE、提交、推送或合并。

## N08 显式选择后的收据提示移交

最终源码完整串行回归 1975 passed、28 ignored；Clippy、fmt、2734 文件行数
门禁通过。无新 EXE、提交、推送或合并。

AuthorizedWindow.prepare_decision 在用户明确选择后，从原收据消费确认提示，
不重新解析或复制一份提示。准备前复核窗口授权、当前版本、交付许可和有效期；
失败保留原收据，构造失败返还提示。成功保留原 ACK，且不移交 owner、不发网络
请求；同一收据不能重复生成决定。后续 begin_decision 仍独立执行新鲜授权检查。

扩充 HWND 回归覆盖过期准备失败保留提示、显式 Reject、成功后重复准备拒绝、
owner 保持活跃及原提示身份保留。原生对话框与应用交互调度仍未接入，点击
入口仍默认禁用；这不是完整确认交互验收。

## N08 决定结果统一窗口交付与收据保留

最终源码完整串行回归 1975 passed、28 ignored；Clippy、fmt、2734 文件
行数门禁及 diff 检查通过。新增 4 项回归覆盖决定后的场景接纳、撤权/未确认
拒绝呈现、错误事件路由保留证据，以及应用结算后收据与恢复元数据一致性。

决定 Job 的结果转换为统一事件 Outcome，复用主循环与退出泵的窗口交付、
有界收据和恢复记录，不再维护独立决定结果队列。点击与决定预留共同的完成
缓存容量，仍共用 32 条未保存动作预算。转换保留原事件、owner、最后 ACK、
决定提交结果、选择/尝试/接收事实及独立收敛/交付状态。

窗口恢复 owner 不代表允许呈现：决定必须提交成功、被服务端接收、匹配原
事件/ACK 和当前场景版本，并通过原有授权、成功 ACK、版本及替换后撤权检查。
拒收原样返还完整证据；已接纳收据仍携带决定元数据，正常退出保存不遗漏。

原生确认对话框与应用发起决定的交互入口尚未接入；输出消费、点击批次调度、
运行中日志和整体验收仍待完成。没有自动批准路径，点击入口默认禁用。
无新 EXE、提交、推送或合并，整体迁移尚未完成。以下为历史检查点。

## N08 应用决定任务集合与恢复元数据

最终源码完整串行回归 1971 passed、28 ignored；Clippy、fmt、2733 文件
行数门禁及 diff 检查通过。新增 4 项回归覆盖应用等待/关闭/共享容量、拒收/
取消保留证据、收据积压限制决定任务及严格决定元数据；扩充真实 TLS 后的
元数据验证。Clippy 指出的测试多余 clone 已移除并重跑通过。无新 EXE、
提交、推送或合并。

Sessions 已接入决定 Job 的等待句柄、普通 poll、关闭取消和退出泵；开始前
预留结果缓存，已完成但未交付的结果仍占容量，拒收原样保留。点击与决定
共用 32 条未保存动作预算，不允许通过决定路径绕过收据上限。取消只撤销
待交付资格，不改写 ACK、用户选择、尝试状态或服务端接收事实。

surface_recovery_bundle 包含决定结果 owner 的附件提示，以及带可选 decision
字段的动作元数据：confirmationId、approved、attempted、confirmed。未尝试却
声称已确认等矛盾记录拒绝，旧记录仍可读取。不保存确认 payload、凭据或
决定 HTTP 原文，也不能从磁盘恢复批准能力。正常退出 writer 自动承接该 bundle。

决定结果到窗口收据的转换与原生确认 UI 尚未接通；输出消费、点击批次调度、
运行中日志与整体验收仍未完成。应用没有自动批准路径，点击入口默认禁用，
整体迁移尚未完成。

## N08 决定后台任务与唯一 owner 移交

最终源码完整串行回归 1967 passed、28 ignored；Clippy、fmt、2731 文件
行数门禁及 diff 检查通过。新增 3 项真实 TLS/线程回归覆盖拒绝决定被接收后
观察 cancelled、迟到撤权、尝试中取消和共享容量；扩充 HWND 测试覆盖过期/
撤权/重复决定 owner 移交。无新 EXE、提交、推送或合并。

surface_decision_runtime 的 Input 完整持有 owner、原事件、ACK、Decision 和
当前呈现版本，spawn/容量失败原样返还。单 worker 复用共享 Slot，先单次
提交决定，再观察原动作；分别保留提交结果、后续收敛、最后 ACK 和收割授权。
取消/异常不能把已尝试决定改成未尝试，收敛失败不抹去决定已被接收的事实。
完成后仍占容量，显式 join 才释放；Drop 取消并回收线程，不遗留后台任务。

Activation.take_decision 与 AuthorizedWindow.begin_decision 为同一原事件
移交唯一 owner，仍检查当前窗口授权、场景版本和提示有效期，不伪造新的
未尝试 click。决定任务在途时不能并发生命周期移交，返回可用原事件恢复 owner。

应用决定任务集合、结果收据/恢复记录与原生确认界面尚未接入；当前应用没有
自动批准路径。输出消费、点击批次调度、运行中日志和整体验收仍待完成，
点击入口默认禁用，整体迁移尚未完成。

## N08 显式确认决定提交与后续收敛接口

最终源码完整串行回归 1964 passed、28 ignored；Clippy、fmt、2729 文件
行数门禁及 diff 检查通过。新增 4 项回归覆盖显式选择/准备失败返还、真实 TLS
批准 POST 后两次 GET 收敛、拒绝/401 不重放，以及过期/版本不匹配/取消时
不尝试网络。无新 EXE、提交、推送或合并，未向真实服务发送决定。

Decision 消费原确认提示，绑定显式 Approve/Reject，没有默认值、Clone 或
Deserialize。提交前再次验证当前 owner、场景版本、来源/设备、原事件与 ACK、
提示有效期；请求仅编码协议规定的身份和 approved，不携带提示 payload。
网络期限不晚于提示到期且最多 10 秒；开始尝试后，即使 401/网络失败也禁止
自动重放或翻转选择。HTTP 2xx 仅标记决定已确认，不代表动作执行成功。

converge_decided_surface 只接受已 HTTP 确认、身份匹配的决定，继续观察原动作，
不因旧 awaiting_confirmation ACK 再次停在提示阶段，不重新 POST 点击。
沿用 25 秒收敛预算、取消/会话复核、整批失败优先和成功后独立快照刷新。

底层接口已具备，原生确认 UI、后台决定任务及其收据/恢复记录仍待连接，
当前应用不会自动批准。输出消费、点击批次调度、运行中日志和整体验收仍待
完成，点击入口默认禁用，整体迁移尚未完成。

## N08 后台确认阶段与窗口收据准入

最终源码完整串行回归 1960 passed、28 ignored；Clippy、fmt、2726 文件
行数门禁及 diff 检查通过。新增 HWND 回归覆盖确认收据使用时的过期/撤权；
扩充真实 TLS 任务测试，覆盖一次 POST 后确认 GET、读取失败保留 ACK、确认
GET 进行中取消。无新 EXE、提交、推送或合并。

事件 Job 遇到 ConfirmationRequired 后，在同一 worker 内读取一次确认提示，
不重放动作、不等待 UI、不提交批准。读取异常独立捕获，保留最后有效 ACK；
Outcome 分别记录收敛、确认读取与交付授权，确认结果一路传入 EventReceipt。
任务集合对读取失败通知一次，取消不弹通用错误，确认 payload 不落盘。

AuthorizedWindow.confirmation_for 是展示前的复核入口：必须仍有当前可见、
激活的窗口和同一绑定/场景版本，收割授权通过，ACK 仍待确认且提示未过期。
撤权会关闭本地窗口，不能因为缓存里曾成功读取提示就继续显示或批准。

原生确认 UI、用户决定提交与确认后的收敛恢复仍待实现；输出消费者、点击
批次调度、动作恢复交互和运行中日志也未完成。点击入口默认禁用，整体迁移
及真实 Loom/完整交互/性能尚未验收。

## N08 用户确认提示准入与设备流读取

最终源码完整串行回归 1959 passed、28 ignored；Clippy、fmt、2726 文件
行数门禁及 diff 检查通过。新增 5 项回归覆盖待确认状态/有效期、身份/风险/
容量边界、批次失败/提示变更否决、真实本地 TLS GET，以及无效状态不连接。
无新 EXE、提交、推送或合并。

新增不透明 Confirmation Request，最多 64 KiB，核对 instance/event/request、
attachment/device/node/action；仅接受三种声明风险。使用时复核已尝试事件、
accepted + awaiting_confirmation ACK、完整绑定和有效期。payload 作为不可信
数据保留，不解释成命令。Tracker 接纳确认消息，整批后续失败可否决提示；
同一 ID 不允许偷偷改变风险、期限或 payload，候选按所有权取走。

Control.surface_confirmation 在明确待确认状态执行一次 after=0 设备流 GET，
复用 3 秒网络上限。取消/会话有效期/确认过期在读取后再次核对，结果保留流中
最后有效 ACK。该接口不批准、不提交决定、不重放事件、不创建常驻轮询。

后台 Job 调度确认读取、原生确认 UI、决定提交、输出消费及点击批次调度仍待
接入；提示存在不代表批准，点击入口保持默认禁用。整体迁移及真实 Loom/
完整交互/性能尚未验收。

## N08 类型化 Surface 输出描述符

最终源码完整串行回归 1954 passed、28 ignored；Clippy、fmt、2722 文件
行数门禁及 diff 检查通过。新增 4 项回归覆盖借用大 JSON、有效资源/流描述符、
资源哈希/类型/尺寸/安全整数、流字段边界、null 与缺失 value、解码后的重复
端口名。无新 EXE、提交、推送或合并。

新增 surface_result_ports，Commit 接纳与 outputs() 类型读取共用一个解析器，
替换上一轮宽松外壳校验。最多 64 个端口，大 value JSON 直接借用 Commit 原文；
显式 null 合法，缺失 value 拒绝。端口名解码后校验与去重，避免转义绕过去重。

资源要求 sha256 小写 64 位哈希、已知 kind、有界 MIME、JavaScript 安全整数
size 和可选 1..16384 尺寸；流要求有效 streamId、有界 itemType/MIME、安全
整数 sequence。没有下载、订阅或执行副作用，不把描述符有效当成授权。

本轮提供后续输出消费者所需的类型接口，但本地图应用、资源授权读取、确认
交互、点击批次调度、动作恢复交互和运行中日志仍待接入。点击入口默认禁用，
整体迁移及真实 Loom/完整交互/性能尚未验收。

## N08 动作 result commit 接收与收据传递

最终源码完整串行回归 1950 passed、28 ignored；Clippy、fmt、2720 文件
行数门禁及 diff 检查通过。新增 4 项回归覆盖跨刷新保留最高结果版本、请求/
代际/协议隔离、后续失败优先、输出数量/重复键/原文容量；扩充 HWND 交付
测试验证 commit 进入收据、退出提示仅保存版本。无新 EXE、提交、推送或合并。

对照现有 SurfaceResultCommit 与设备流合同，实现 loom.surface.result
接收：匹配 instance/request/generation、固定协议和非零 resultRevision。
原文最多 1 MiB、outputs 最多 64 个唯一端口，校验 value/resource/stream
外壳；实际资源授权、完整资源类型和本地图输出应用仍由后续消费者负责。

Tracker 保留匹配动作的最高结果版本，跨成功后的独立快照刷新继续持有，
整批失败/代际变化仍阻止交付；成功时随 Snapshot 移交窗口收据，不复制大
结果原文。32 条收据的 commit 原文上限为 32 MiB（另有有界事件与 ACK），
无动作时不分配。退出提示只保存可选 resultRevision，不保存输出数据。

这不是本地图结果应用完成：输出消费者、确认交互、点击批次调度、动作恢复
交互和运行中日志仍待接入。点击入口保持默认禁用，整体迁移及真实 Loom/
完整交互/性能尚未验收。

## N08 正常退出动作恢复提示落盘

最终源码完整串行回归 1946 passed、28 ignored；Clippy、fmt、2718 文件
行数门禁及 diff 检查通过。新增 5 项回归覆盖动作元数据往返/配额/严格解析、
独立目录只增存储、结果与收据 bundle 不消费证据，以及仅有动作记录时后台
退出写入。无新 EXE、提交、推送或合并。

新增 surface_event_record：最多 32 条、总 JSON 64 KiB，仅保存附件绑定、
事件 ID、基准版本、是否尝试提交、ACK 摘要和收敛/呈现阶段。无 payload、
Scene、凭据或远端错误文本；磁盘数据不能恢复授权或自动重放事件。解析拒绝
未知字段、缺失绑定、无效版本及相互矛盾的成功证据。

Sessions 新增两类恢复提示的联合快照，保留未交付事件 owner 对应的附件记录，
不提前移走内存证据。应用正常退出在原有后台 writer 中写附件与动作提示；
动作使用独立 surface-event-recovery 目录、128 个 create_new 槽位和 sync_all。
旧附件读取器拒绝动作文件。两步不是原子事务；后续写入失败整体报错，并保留
之前写好的文件，不删除、不覆盖、不伪造全量保存成功。

当前只支持正常退出保存，尚未提供运行中崩溃/断电日志、动作恢复交互及缓存
运行中落盘回收；result commit、确认交互与点击批次调度仍待完成。点击入口
保持默认禁用，整体迁移、真实 Loom/完整交互/性能尚未验收。

## N08 应用事件结算与有界收据缓存

最终源码完整串行回归 1941 passed、28 ignored；Clippy、fmt、2716 文件
行数门禁及 diff 检查通过。新增 2 项回归覆盖拒收/匹配交付、收据保存、退出
证据门禁和容量耗尽。首轮新增测试把 mounted 夹具误断言为 active，已改为
核对原绑定完整返还，并重新执行全量通过。无新 EXE、提交、推送或合并。

普通应用结算和退出泵现在先把事件结果交给匹配窗口，再处理生命周期。
Sessions 在窗口产生副作用前预留收据内存；拒收保留原 Outcome，接纳保存
EventReceipt。未保存动作（收据、在途与完成结果）最多 32 个，满时拒绝
新动作，不覆盖旧记录。收据不占网络槽位，避免阻塞附件停用；没有线程或
等待句柄，不启动周期唤醒。关闭集合不改写已完成的历史授权/呈现检查。

旧版附件恢复日志现在也明确拒绝未保存收据，不能报告已保存全部恢复证据。
动作持久化仍待实现，因此这只是内存中的证据持有，不是崩溃恢复保证。
result commit、确认交互、点击批次调度和完整端到端验收仍待完成；点击入口
保持默认禁用，整体迁移尚未完成。

## N08 授权窗口事件结果交付

最终源码完整串行回归 1939 passed、28 ignored；Clippy、fmt、2716 文件
行数门禁及 diff 检查通过。新增 5 项本地 HWND 回归覆盖成功替换、错误事件
完整退回、撤权/过期/关闭/交付拒绝、旧版本/无成功证据，以及集合精确路由。
测试通过不代表真实 Loom、完整交互或性能验收；无新 EXE、提交、推送或合并。

AuthorizedWindow.accept_event 只接纳当前 pending event 的完整绑定结果，
返还唯一 owner 后才独立审核呈现。应用场景需要有效当前租约、可见且激活的
窗口、已尝试的事件、成功且匹配的 ACK、当前基准版本及不回退的快照版本。
替换失败保守关闭，替换后再复核授权，不自动重新显示已关闭窗口。

EventReceipt 分开保存 ACK、收敛版本/错误、收割授权与呈现结果，Scene 只
移入窗口一次，避免缓存第二份大场景。Windows 集合按精确事件路由，拒收则
归还全部 Outcome。接口已实现，但主循环收据缓存/持久化、result commit、
确认交互和点击批次调度仍待接入；点击入口仍默认禁用，整体迁移尚未完成。

## N08 后台事件提交与收敛串联

最终源码完整串行回归 1934 passed、28 ignored；Clippy、fmt、2714 文件
行数门禁及 diff 检查通过。扩充原有迟到撤权测试为真实 POST/GET 成功链，
新增 2 项回归覆盖流协议失败、等待确认与 ACK 后取消。无新 EXE、提交、
推送或合并。

事件 Job 在原有独占 owner、共享容量和单 worker 内接通收敛，提交成功后
最多再等待 25 秒；不新建轮询线程、不重复 POST。Outcome 分别保存最终
有效 ACK、场景/收敛错误和收割时授权，后续失败及撤权不能抹去提交证据。
收敛 panic 独立捕获，至少保留初始 ACK。未收到有效提交 ACK 时不启动收敛。

Sessions 收割缓存完整结果，收敛错误进入一次性通知；取消与等待确认不作为
通用错误弹出。结果消费仍须重新验证当前授权，成功快照不自动应用窗口。
result commit、确认 UI、窗口呈现提交、动作恢复持久化和点击批次调度仍待
完成；点击入口保持默认关闭，整体迁移与真实 Loom/可见 UI/性能尚未验收。

## N08 有截止的真实网络收敛

最终源码完整串行回归 1932 passed、28 ignored；Clippy、fmt、2714 文件
行数门禁及 diff 检查通过。新增 3 项真实 TLS/截止回归覆盖一次提交后的独立
刷新、截止/取消/确认不发请求，以及活跃读取取消后保留 ACK。
无新 EXE、提交、推送或合并。

Control.converge_surface 已将 Tracker 接入设备流，最多 25 秒总预算，调用方
只能缩短截止。共享 HTTP 增加绝对截止入口，流请求仍最多 3 秒且不得越过
总截止；读取和解析后复查取消、截止及会话有效期。成功后立即独立刷新，
其他读取之间用可取消的 200ms 等待，不启动常驻轮询或重试动作 POST。

ConvergenceOutcome 保留最后有效 ACK 和独立的场景/错误结果；需要确认时
立即返回 ConfirmationRequired，不在后台等满 25 秒，也不自行批准。批次
解析后立即释放，再进入等待。该 API 已用真实 TLS 串联一次 POST、成功观察
和独立最新快照读取；尚未改动后台 Job 的提交阶段和 UI 呈现交付。

## N08 动作后场景收敛状态机

最终源码完整串行回归 1929 passed、28 ignored；Clippy、fmt、2712 文件
行数门禁及 diff 检查通过。新增 6 项回归覆盖成功后的独立刷新、整批失败
优先、外来/旧快照排除、代际/销毁/生命周期失效、历史 reset 与错误游标。
无新 EXE、提交、推送或合并。

新增 Tracker，借用单个 ClickEvent 并保留有界 ACK 状态；Batch 记录真实请求
after，状态机拒绝错误查询游标及非零游标上的 reset 历史丢失。匹配事件 ACK
检查固定 request，失败/取消/中断与 unknown 均终止，不把接收或排队当成功。

观察成功后强制下一次独立 after=0 刷新，之前/同一观察批次的旧快照不交付。
刷新批次按附件/generation/最低场景版本筛选最新快照，整批扫描完才解析为
Scene；后续失败、generation 增加、dispose 或当前生命周期停用阻止交付。
刷新重放的早期 running 不降级已确认成功，但后续失败仍可终止。快照直接
借用批次 RawValue，仅最终 Scene 分配，取走一次后不再接受后续批次。

此模块不执行 HTTP、定时等待或 UI 更新；result commit、确认交互、后台
有截止的流读取循环、窗口呈现提交及动作恢复持久化仍需后续接入。

## N08 设备 Surface 消息流读取

最终源码完整串行回归 1923 passed、28 ignored；Clippy、fmt、2710 文件
行数门禁及 diff 检查通过。新增 6 项解析/真实 TLS 回归覆盖协议/reset/游标、
批次/消息预算、设备认证与超过 1 MiB 的响应、401 不回退及传输预算超限。
无新 EXE、提交、推送或合并。

设备 HTTP 通用传输允许的响应预算上限从 1 MiB 提高到 8 MiB，以支持主干
Surface stream 批次；已有调用方仍显式使用原小预算，动作 ACK 保持 64 KiB，
请求 body 上限仍为 1 MiB。真实 TLS 测试包含超过 1 MiB 的合法多消息批次，
另有超过 8 MiB 的请求预算在连接前拒绝测试。

Control.surface_stream 已实现单次 GET /v1/surfaces/stream?after=...&timeoutMs=0，
复用设备认证、nonce、取消与截止，不读取管理员完整实例，不启动轮询或重试。
传输层限制响应 8 MiB；Batch 校验 loom.surface-stream.v1、必需布尔 reset、
无倒退的 next，缺省 next 保留调用方游标，缺省 messages 为空。

最多 512 条消息、每条原始 JSON 1 MiB，visitor 在构建集合时检查数量，
不先反序列化无界数组。消息以 RawValue 保存，未知 method 不执行。批次
本身不推进游标、不授权应用消息；reset 历史丢失、事件/附件/generation
匹配、终态及成功后的最新快照获取仍需后续收敛 owner 处理。

## N08 点击候选消费与场景身份

完整串行回归 1917 passed、28 ignored；Clippy、fmt、2707 文件行数门禁及
diff 检查通过。新增 2 项隐藏 HWND 消费回归覆盖跨窗口、同 ID 换动作、有效
候选编码，以及领取后尺寸/失焦/取消/禁用失效。无新 EXE、提交、推送或合并。

Click 携带私有 Rc 场景身份，窗口之间不共享；场景替换、尺寸/DPI、禁用、
失焦与取消轮换身份，即使候选已被领取，也不能用于新场景。旧 Rc 由候选
持有期间不会复用，因此复用节点 ID 或 HWND 不能使旧候选重新有效。

SurfaceWindow.click_events_for_candidate 消费候选，在同一 State::run 内
重新命中、检查节点并准备事件，返回前再次核对窗口/场景身份。授权窗口
prepare_click 改为只接受候选，take_prepared_click 连接单槽领取与授权检查；
失败不重新排队、不发送动作。应用尚未启用鼠标入口，发送调度/收敛仍待完成。

## N08 原生左键消息与单槽候选

完整串行回归 1915 passed、28 ignored；Clippy、fmt、2707 文件行数门禁及
diff 检查通过。新增 3 项隐藏 HWND 实际消息/捕获回归，覆盖默认关闭、单槽
不覆盖、拖动/负坐标/失焦/取消/捕获丢失，以及替换/禁用/尺寸变化/关闭清理。
无新 EXE、提交、推送或合并。

SurfaceWindow 已处理 WM_LBUTTONDOWN/UP、WM_MOUSEMOVE 与取消消息。按下和
抬起必须命中同一节点，移动超过系统拖动阈值取消；即使没有收到 move，抬起
位移也会检查。按下设置捕获，抬起释放；候选只含坐标/节点/修饰键，单槽
待消费，不在窗口回调编码事件或发送网络。尺寸/DPI 和成功场景替换清理候选。

set_pointer_enabled 默认 false，应用尚未启用；take_click 只是领取候选。
完整权限/选择文本/穿透规则、消费时重新命中、事件发送调度与结果收敛仍需
后续接线。本地隐藏 HWND 通过不等于可见手势或混合 DPI 的人工验收。

## N08 客户区坐标到点击事件批次

完整串行回归 1912 passed、28 ignored；Clippy、fmt、2705 文件行数门禁及
diff 检查通过。新增 3 项回归覆盖事件顺序/版本/未尝试状态、整批预算失败，
以及隐藏 HWND 客户区准备/负坐标/忙碌/关闭边界。无新 EXE、提交、推送或合并。

Presentation.click_events 已将命中/冒泡和事件编码连接，顺序保持由内到外，
每项独立 eventId，整批事件总字节限制 64 KiB；任何项编码失败或合计超限
均拒绝整批，不交付部分动作。此为宿主本地整批预算，不宣称服务端相同上限。

Host.click_events 复用当前视口同步和 DPI 坐标转换，不创建绘制 target；
SurfaceWindow 使用 State::run，忙碌/重入关闭返回错误。授权窗口新增
prepare_click，准备前后检查显示/存活、租约、active、绑定和场景版本；
准备不等于提交，begin_event 与后台 HTTP 仍需再次检查。本轮未接鼠标消息。

## N08 窗口事件 owner 独占移交

最终源码完整串行回归 1909 passed、28 ignored；Clippy、fmt、2705 文件
行数门禁及 diff 检查通过。新增 3 项回归覆盖独占移交/后续停用、跨事件与
生命周期归还拒绝、失活/终态/已尝试/绑定不符拒绝。disposed 夹具经真实
生命周期 ACK 构造，不绕过 mount 拒绝终态的合同。无新 EXE、提交、推送或合并。

Activation 增加事件身份绑定的 take_event/restore_event，与激活/停用复用
唯一 owner 槽。只允许已确认 active、绑定匹配且未尝试的事件取得 owner；
在途事件阻止再次点击移交及生命周期移交。归还时核对 pending eventId、
完整绑定与 confirmed_active，跨事件/生命周期返回拒收并返还原 owner。

AuthorizedWindow.begin_event 已连接当前租约、窗口显示/存活及场景 revision
检查；finish_event 仅恢复 owner，不重开窗口、不把 ACK 视为授权。关闭期间
返回 owner 后可继续原停用流程。鼠标分发、结果接纳/收敛和动作恢复持久化
仍待接入，本轮不启用宿主输入能力。

## N08 事件任务接入应用集合

完整串行回归 1906 passed、28 ignored；Clippy、fmt、2704 文件行数门禁及
diff 检查通过。新增 3 项真实线程/集合回归覆盖等待与退出收割、共享容量、
关闭后拒绝启动、拒收保留、取消不丢 ACK 及旧恢复格式拒绝遗漏动作证据。
无新 EXE、提交、推送或合并。

Sessions 已接入后台 click 的启动、等待句柄、正常 poll_surfaces 收割、退出
取消与 finished 判定；事件任务/完成缓存与其他远程任务共用 MAX_SESSIONS，
不增加等待句柄上限。正常应用消息结算与退出 drain 会收割这些任务，不依赖
新定时器。拒收结果保留且不阻塞其他可交付项，取消缓存结果仅撤销 delivery。

有在途或未消费事件时 surface_cleanup_pending 为真。旧附件恢复记录没有
event/request 字段，因此 surface_recovery_records 拒绝将这些待处理事件
伪装成完整恢复快照。动作级持久化、窗口结果接纳/当前授权复核、原生鼠标
入口和远端状态收敛仍待完成；本轮未启用输入能力或发起真实服务动作。

## N08 后台事件提交 owner

完整串行回归 1903 passed、28 ignored；Clippy、fmt、2702 文件行数门禁及
diff 检查通过。新增 3 项回归覆盖真实 TLS 成功后的迟到撤权、已发请求取消
与事件返还、完成未收割容量及启动失败返还。无新 EXE、提交、推送或合并。

新增 surface_event_runtime，支持配置/已准备 Client 两种后台启动入口，复用
共享 Slot 容量与租约取消机制；提供线程等待句柄和非阻塞 try_join。任务独占
生命周期 owner 与 ClickEvent，启动失败返还二者，完成收割返还原事件身份和
attempted 状态。已完成未收割任务继续占容量，禁止无限积压。

Outcome.result 保存 ACK/错误执行证据，delivery 按收割时当前租约、来源及
时刻独立校验；迟到撤权不删除成功 ACK，但不得将其当成当前授权。取消不
重发，也不把动作取消猜成远端回滚。Drop 取消并 join 仅作兜底，正常应用
需登记句柄、等待并收割。UI 调度/输入接线与远端查询收敛仍待完成。

## N08 单次 HTTPS 点击事件提交

完整串行回归 1900 passed、28 ignored；Clippy、fmt、2700 文件行数门禁及
diff 检查通过。新增 3 项本地 TLS/交付边界回归覆盖准确请求/设备认证/路径
转义、失活/陈旧/跨来源拒绝，以及错配 ACK 后保持 attempted 并拒绝重放。
无新 EXE、提交、推送或合并。

Control.submit_surface_click 已接真实 HTTPS POST /v1/surfaces/instances/{id}/events，
实例 ID 按独立路径段转义，复用固定来源设备认证、nonce、取消和超时机制。
发送前要求生命周期 confirmed_active、完整 Binding 相同和当前场景版本相同。
响应在传输层即施加 64 KiB 上限，再进入 ACK 身份解析。

ClickEvent 在进入提交阶段后永久标记 attempted，同一对象后续提交明确拒绝。
网络/认证/ACK 错误不触发自动重发；attempted 仅表示开始尝试，不证明远端
已收到或未收到，失败需要后续查询收敛。未进入提交的陈旧/失活/跨来源事件
被拒绝且不标记 attempted。本 API 为后台基础设施，原生鼠标与任务调度、
当前 UI 租约复核及结果收敛尚未接通，不扩大输入能力声明。

## N08 动作 ACK 身份与状态

最终源码完整串行回归 1897 passed、28 ignored；Clippy、fmt、2698 文件
行数门禁及 diff 检查通过。新增 4 项回归覆盖十种状态/拒绝、交叉身份与
未知状态、失败更新的原子性、错误信息保留及预算/字段类型边界。
无新 EXE、提交、推送或合并。

新增 surface_action_ack，解析前限制 64 KiB，严格检查协议、instanceId、
eventId 和 requestId。后续更新还要求同一 request，验证失败保留旧状态。
公开 Ack 不能直接反序列化，私有 Wire 通过校验后才可产生 Ack。

完整保留十种主干状态及 accepted/rejected，不将 queued、等待确认或 unknown
当成成功。succeeded 只表示 accepted=true 且状态 succeeded，不表示场景已
收敛。远端错误 code/message/detail 作为不可信数据保留，不执行或自动记录。
ClickEvent 同时保留原绑定和场景版本，便于后续发送前复核，不依赖解析 body。

ACK 不提供附件/generation/顺序号，因此来源、生命周期和消息顺序仍须由
发送与流 owner 验证；本轮未发送 HTTP，不扩大宿主输入能力声明。

## N08 原生 click wire 编码

最终源码完整串行回归 1893 passed、28 ignored；Clippy、fmt、2697 文件
行数门禁及 diff 检查通过。新增 4 项回归覆盖完整 wire 字段/独立 ID、按钮
与容器差异、payload 默认/原值/转义膨胀，以及 writer 精确预算边界。
无新 EXE、提交、推送或合并。

ClickAction.encode 已生成 loom.surface.v1 事件，实例/附件/generation 取自
Binding，baseRevision 显式取当前场景版本，eventId 使用 128 位安全随机值。
容器事件携带四个 modifiers，按钮按主干省略；null/缺省 payload 使用空对象，
其他 JSON 类型原样保留。目前无表单草稿，因此不注入 surfaceValues。

新增 64 KiB 整体事件输出预算（包括头字段及 JSON 转义后的 payload）。有界
writer 先检查再写入，几何增长避免小块序列化反复分配；不先克隆 payload 或
构造无界 JSON 字符串。超限返回错误，不截断动作。该本地预算不宣称服务端
采用相同上限。编码不是授权或发送，尚未接原生鼠标/HTTP/ACK 与状态收敛。

## N08 场景快照版本保留

最终源码完整串行回归 1889 passed、28 ignored；Clippy、fmt、2695 文件
行数门禁及 diff 检查通过。新增 2 项交付边界测试，扩展隐藏 HWND 撤权与
迟到生命周期 ACK 测试，验证场景版本 57 不被生命周期版本 1/2 替代。
无新 EXE、提交、推送或合并。

挂载交付从仅返回 Scene 改为 AdmittedSnapshot，私有字段同时持有同一已验证
响应的 Scene/revision；原 scene_for 内部 API 已更名 snapshot_for，全部代码
调用点已迁移，不保留丢失版本的替代入口。AuthorizedWindow 建窗成功时保存
snapshot_revision，生命周期 ACK、关闭和撤权不修改该场景版本。

这是后续事件 baseRevision 的必要来源，不将 lifecycleRevision/generation
冒充场景 revision。身份、非零整数版本、声明式运行时和解析前后租约检查
继续生效；交付失败保留原始恢复证据。本轮不发送动作、不扩大输入能力。

## N08 声明式 click 路由

完整串行回归 1887 passed、28 ignored；Clippy、fmt、2695 文件行数门禁及
diff 检查通过。新增 4 项回归覆盖冒泡/阻断、隐藏和 payload 借用、最上层
命中以及失败/成功替换后的动作一致性。无新 EXE、提交、推送或合并。

Presentation.click_route 从当前命中结果构造只借用当前 Scene 的固定深度
路径，按从内到外顺序返回容器 click 声明；文字不执行自身 click，按钮
阻止冒泡，disabled=true 按钮不返回动作。隐藏子树没有路径，叠放只从最上层
命中节点冒泡，不穿透到后方兄弟。payload 原位借用，不产生副本或事件队列。

该接口不是动作发送或权限凭证；选择手势抑制、hostPointerPassthrough、
输入权限、版本/生命周期和 payload 协议验证仍由后续输入/发送 owner 接入。
按钮规则的合同测试不代表按钮绘制已完成，不扩大宿主输入能力声明。

## N08 HWND 宿主命中坐标与视口同步

完整串行回归 1883 passed、28 ignored；Clippy、fmt、2694 文件行数门禁及
diff 检查通过。新增 2 项隐藏 HWND 回归覆盖重绘前尺寸变化、零面积/负坐标、
渲染 target 延迟创建，以及窗口忙碌/关闭后的命中拒绝。无新 EXE、提交、
推送或合并。

Host/SurfaceWindow 已提供客户区像素命中查询，使用窗口当前 DPI 转为逻辑
坐标。查询前与 paint 共用视口同步：尺寸/DPI 变化先重排并丢弃旧 render
target，不等待 WM_PAINT；查询本身不创建 D2D target。零面积视口不返回目标。
场景 replace 也使用当前 HWND 视口，而非上次重绘尺寸。

SurfaceWindow 复用可重入 State::run，忙碌/关闭均返回错误，关闭过程中
不返回已取出 Host 的过期目标；成功返回身份副本，不暴露跨回调借用。
此 API 尚未接鼠标消息或动作发送，仍不扩大输入能力声明。当前 DPI 隐藏
HWND 测试不等于真实跨屏混合 DPI 或可见交互验收。

## N08 原生 Surface 命中快照

完整串行回归 1881 passed、28 ignored；Clippy、fmt、2694 文件行数门禁及
diff 检查通过。新增 4 项回归覆盖叠放/隐藏、自身和祖先圆角、可见溢出/
透明度以及失败更新/重排/隐藏替换。无新 EXE、提交、推送或合并。

Presentation 已将 surface_hit_test 的 CPU 快照与几何/文字/绘制同批提交。
hit_test 接收宿主逻辑坐标，按逆绘制顺序返回最上层节点 ID，检查矩形宿主/
祖先裁剪、自身圆角和祖先 padding-box 圆角 mask。可见 overflow 不被普通
父圆角误裁，opacity=0 仍可命中，visible=false 没有命中条目。

命中不创建 COM 或临时集合，查找受场景节点数及树深度上限约束；快照只
持有圆角信息，ID/父索引/矩形复用 Geometry。更新失败保留原命中快照，隐藏
替换清空目标。输入授权、捕获、焦点、动作分发尚未接入，不扩大宿主输入
能力声明；几何命中测试不等于真实鼠标或跨 DPI 验收。

## N08 visible=false 隐藏子树

完整串行回归 1877 passed、28 ignored；Clippy、fmt、2692 文件行数门禁及
diff 检查通过。新增 4 项回归覆盖不占 gap、隐藏/恢复快照、布尔值和输入
边界，以及真实像素不泄漏隐藏内容。无新 EXE、提交、推送或合并。

对齐前端 Show：只有布尔 props.visible=false 隐藏节点。隐藏节点与后代
不进入样式解析、测量、正常流/Stack 分配、几何/文字和绘制快照，不占 gap，
与仍占布局的 opacity=0 明确区分。隐藏根返回空树，Presentation 替换时
释放原快照，重新显示时重新准备，避免保留旧文字或背景。

Scene 仍校验完整原始输入的字节/节点/深度/身份限制；隐藏不意味着可信，
不能绕过重复 ID 或容量门禁。隐藏子树中的未实现控件不参与渲染准入，
重新显示后仍必须通过实际节点能力与测量检查。

## N08 Progress 原生只读指示器

完整串行回归 1873 passed、28 ignored；Clippy、fmt、2691 文件行数门禁及
diff 检查通过。新增 4 项数值边界、布局/字号、叶子约束与真实像素回归，
像素检查覆盖零/半/满进度及绘制状态恢复。无新 EXE、提交、推送或合并。

Progress 已接入满宽、继承字号 1em 高度的原生叶子布局。value/max 只取有限
数值，缺失 value 为零，非正 max 回到 1，比例钳制到 [0,1]。对齐前端始终
提供 value 的确定进度语义，不新增不确定动画。节点 layout/style 与前端
progress 一样不作为覆盖，原始字段仍接受输入容量校验。

surface_progress 预计算轨道/比例，绘制复用调用方画刷并配对裁剪；无需
子 HWND、文字布局、额外几何缓存或定时器。surface_control_colors 映射
Neuro 规范控件色 #111720 和信号黄 #d9ff38，不复制旧前端的竞争黄色。
动态主题和完整 UIA 可访问性尚未接入，像素测试不代表可见 UI 验收。

宿主报告增加 progress，输入能力仍全部为 false。其余控件、滚动和整体
迁移及真实 Loom/跨 DPI/性能验收仍待完成。

## N08 Divider 原生节点

完整串行回归 1869 passed、28 ignored；Clippy、fmt、2688 文件行数门禁及
diff 检查通过。新增 4 项布局、固定属性、原始输入/叶子边界及真实像素回归，
并扩展宿主报告检查。无新 EXE、提交、推送或合并。

Divider 已接入固定满宽、1px 线高的叶子测量和已有矩形绘制，使用前端暗色
--nt-border-subtle 回退色 #242b33；不创建文字布局、独立窗口或定时器。
对齐前端 hr 不应用节点 layout/style 的行为，surface_node_defaults 提供
有界共享有效属性视图，原始字段仍保留并参与 Scene 的容量安全校验。
无效子树拒绝，不能借固定叶子绕过未实现节点检查。

宿主报告增加 divider；尚未接动态主题变量，不宣称任意主题一致性。
其他控件、滚动、剩余布局边界和真实 Loom/可见 UI/性能验收仍待完成。

## N08 Spacer 原生节点

最终源码完整串行回归 1865 passed、28 ignored；Clippy、fmt、2686 文件
行数门禁及 diff 检查通过。新增 5 项回归覆盖双轴剩余空间、显式 grow、
叶子边界、真实像素绘制和能力报告不扩大输入/滚动声明。无 EXE 或发布操作。

Spacer 已接入真实 NativeMeasure、行列布局与既有样式绘制。固有内容尺寸
为零，边框/padding 仍占空间；默认 grow=1，显式 grow=0 或其他有界值优先。
多个 spacer 按既有 flex 分配剩余空间，不创建文字布局、额外 HWND 或定时器。
Spacer 按叶子处理，带子树的无效节点拒绝，不能借空白节点隐藏未实现控件。

新建/恢复共用的宿主能力报告增加 spacer，仍不宣称输入、滚动、图片、脚本
和 Shader 能力。布局/像素检查不替代真实 Loom 挂载与可见窗口验收。

## N08 零面积文字内容盒

完整串行回归 1860 passed、28 ignored；Clippy、fmt、2685 文件行数门禁及
diff 检查通过。真实 DirectWrite 回归确认零宽、零高、双零布局可以测量；
新增场景回归确认边框/padding 挤空内容盒不会使整个 Presentation 失败。

Engine 接受非负布局宽高，不再把零尺寸当成非法输入，也不伪造 epsilon
宽度。负数、非有限值和超上限仍拒绝；字体大小仍须为正。最终文字快照
继续跳过零面积内容盒，不创建无意义的绘制项，节点几何与边框仍保留。
核查接口：[DirectWrite CreateTextLayout](https://learn.microsoft.com/en-us/windows/win32/api/dwrite/nf-dwrite-idwritefactory-createtextlayout)；
零尺寸支持以本机真实调用结果为证据，不从文档未写限制推断成功。
无新 EXE、提交、推送或合并，整体迁移及交互/性能验收仍未完成。

## N08 容器默认宽度统一取值

最终源码完整串行回归 1858 passed、28 ignored；Clippy、fmt、2685 文件
行数门禁及 diff 检查通过。新增回归覆盖五类容器非 stretch 对齐的默认
满宽，以及绝对定位默认宽度与显式 auto 的差异。无 EXE 或发布操作。

新增 surface_layout_value，按现有 DeclarativeSurface.css 给 view/row/column/
stack/scroll 提供默认 width:100%，显式 auto 保持自动值。根盒、正常流、
Stack、绝对定位与原生测量共用取值器，不再各自猜测缺失宽度。
这修复非 stretch 对齐下容器错误缩到内容宽度，以及绝对定位容器被错误
按两端锚点拉伸的问题。根盒也移除重复的原始字符串 auto 判断。

单端 auto 收缩适配、固有尺寸循环百分比和其他节点默认样式仍待完成；
本增量不等于所有控件已实现或完整 CSS 解析支持。

## N08 根 absolute auto 双端尺寸

完整串行回归 1856 passed、28 ignored；Clippy、fmt、2684 文件行数门禁及
diff 检查通过。新增 2 项几何/约束回归与 1 项真实像素回归，覆盖自动轴、
默认容器宽度、显式尺寸优先、约束钳制与上下锚点。无 EXE 或发布操作。

根 absolute 节点的自动轴在两端锚点均存在时，使用宿主轴长减去两端偏移，
再应用 min/max 与边框/padding 下限。显式尺寸始终优先。宽轴区分容器类
默认 width:100% 与显式 width:auto；前者不会误触发双端自动伸展。

单端 auto 宽度的收缩适配、子节点默认样式完整对齐和循环百分比仍待完成，
不把双端尺寸接线当作完整定位算法或其他迁移任务完成。

## N08 默认根高度使用内容测量

最终源码完整串行回归 1853 passed、28 ignored；Clippy、fmt、2684 文件
行数门禁及 diff 检查通过。新增自然高度几何/像素回归，既有文字重排回归
验证根内容高度增长而宿主裁剪保持固定。无新 EXE、提交、推送或合并。

Tree 在求根盒尺寸前测量根节点，未声明 height 的根盒使用内容高度，再应用
min/max 与边框/padding 下限，不再默认填满宿主。文字根高度随断行反馈
变化；宿主 viewport 和裁剪仍固定，内容增长不会回写窗口大小。

满屏绘制合同的夹具现在显式提供根高，另有自然高度像素验证；固定容器
裁剪/定位测试也明确根高，不能依赖已移除的填屏默认。根测量属于正式
Measure 合同，因此测量失败仍不交付部分树。

absolute auto 双端尺寸推导、收缩适配及固有尺寸循环百分比尚待完成；
默认根宽度当前保持块级宿主宽度，不能据此宣称完整 CSS sizing。

## N08 根盒定位接线

完整串行回归 1851 passed、28 ignored；Clippy、fmt、2684 文件行数门禁及
diff 检查通过。新增 2 项几何回归与 1 项真实像素回归，覆盖末端锚点、
约束尺寸、计算字号、起始边优先和子树平移。无新 EXE、提交、推送或合并。

根节点 relative 的 left/top 正向偏移与 right/bottom 负向偏移、absolute
的宿主起止锚点已接入布局树。末端锚点使用最终 min/max/边框约束后的尺寸；
LTR 起始边优先，百分比基于宿主两轴，em 基于根计算字号。子树随根原点
平移，宿主 viewport 与裁剪范围不移动。

本阶段覆盖已有尺寸的根盒定位；默认内容自适应、absolute auto 双端尺寸
推导/收缩适配和固有尺寸循环百分比仍待对齐，不能标记完整定位规范完成。

## N08 根盒显式约束与宿主视口分离

完整串行回归 1848 passed、28 ignored；Clippy、fmt、2684 文件行数门禁及
diff 检查通过。新增 2 项根盒/宿主约束回归和 1 项真实背景像素回归通过。
无新 EXE、提交、推送或合并，未完成可见 UI、跨 DPI 或性能验收。

Tree 单独保存宿主 viewport；根盒的 width/height、min/max 与边框/padding
下限由 surface_root_box 求解，百分比仍基于宿主尺寸，em 使用根计算字号。
根盒可大于或小于宿主，但根祖先裁剪始终由宿主 viewport 决定；Presentation
size 继续报告宿主尺寸，不能因根布局变化触发错误窗口尺寸回写。

未声明尺寸仍暂用宿主分配；根节点默认内容自适应高度、固有尺寸循环百分比
和定位语义尚未全部对齐，不能将本增量当作完整根盒 CSS sizing。

## N08 百分比 padding 包含块接线

最终源码完整串行回归 1845 passed、28 ignored；Clippy、fmt、2683 文件
行数门禁及 diff 检查通过。新增 2 项回归覆盖嵌套正常流、文字断行/高度
和绝对定位 padding-box 基准。测试明确使用非 stretch 对齐验证固有高度，
避免将父级拉伸高度误当成文字重排结果。无新 EXE、提交、推送或合并。

Placed 显式记录 containing_width，正常流使用父内容盒，绝对定位使用父
padding box。容器排版、文字测量/快照和外盒下限按该宽度解析百分比
padding，不再以节点自身显式 width 为基准；cqw/cqh 继续使用宿主查询容器。
布局树测量子节点时也传入对应可用空间，而非无条件传父外盒。

独立 arrange_with_fonts 入口没有父上下文时仍以传入尺寸为基准；完整场景
通过 arrange_in 传递明确的包含块。根宿主分配与固有尺寸的循环百分比
解析仍需后续对齐，本增量不宣称完整 CSS sizing 或滚动功能完成。

## N08 子节点外盒最小尺寸

完整串行回归 1843 passed、28 ignored；Clippy、fmt、2683 文件行数门禁及
diff 检查通过。新增回归覆盖三类容器中的极小显式/max 尺寸，以及绝对
定位右/下锚点。未生成 EXE、提交、推送或合并，未执行可见 UI 验收。

正常流、Stack 和绝对定位共用 surface_box::minimum，将边框与 padding
作为不可压缩的外盒下限。显式 width/height、maxWidth/maxHeight 或 flex
收缩不能把内容盒压成负数；父空间不足时保留溢出。绝对定位右/下锚点
使用钳制后的尺寸，允许产生负原点而非破坏锚点。

这不是完整 CSS sizing 实现：根节点仍由宿主视口分配，百分比 padding 的
包含块基准与固有尺寸循环仍需对齐；滚动和其他迁移功能继续待办。

## N08 单轴 overflow 计算值归一

完整串行回归 1841 passed、28 ignored；Clippy、fmt、2683 文件行数门禁及
diff 检查通过。新增计算值表驱动回归和双方向真实像素回归，既有祖先裁剪
测试同步改为验证计算值而非旧临时行为。无新 EXE、提交、推送或合并。

新增 surface_overflow 统一前端白名单的计算值：一轴 hidden/auto 会使另一轴
默认 visible 计算成 auto；Scroll 默认两轴 auto。非法关键字仍由前端相同的
白名单规则忽略，不借机开放任意 CSS。矩形祖先裁剪与圆角 mask 使用同一
计算结果，替换之前“只裁声明轴”的临时行为。

依据：[W3C CSS Overflow 3 计算值与圆角裁剪](https://www.w3.org/TR/css-overflow-3/#overflow-properties)。
此处只完成计算值和绘制裁剪，不代表滚动条、滚轮、程序化滚动或圆角命中
测试已经实现。极小外盒 CSS 最小尺寸和其余迁移功能仍待完成。

## N08 padding-box overflow 裁剪

完整串行回归 1839 passed、28 ignored；Clippy、fmt、2682 文件行数门禁及
diff 检查通过。新增 2 项真实像素回归和 1 项几何/文字一致性回归通过。
无新 EXE、提交、推送或合并，未验收可见 UI、跨 DPI 或实际资源开销。

祖先裁剪现在接收基于已计算字号的 borderWidth，声明裁剪的轴收缩到
padding box，不裁掉父节点自身背景或边框。圆角 mask 同步内缩外轮廓，
半径扣除边框宽度，避免子内容覆盖圆角边框内侧。单轴继续仅裁声明轴。

Presentation 的持久几何、文字快照与 PaintFrame 使用相同边框上下文；
矩形几何仍不代表圆角命中测试。无样式上下文的底层 ancestors/from_tree
入口保留零边框行为，生产 Presentation 使用带边框的入口。
极小外盒 CSS 最小尺寸、完整单轴 overflow 归一和其余控件仍待完成。

## N08 边框环绘制

最终源码完整串行回归 1836 passed、28 ignored；Clippy、fmt、2682 文件
行数门禁及 diff 检查通过。新增 3 项真实像素回归验证透明中心、继承颜色、
圆角外轮廓、背景保留和过厚半透明边框。无新 EXE、提交、推送或合并。

Surface 绘制指令已在背景之后、文字和子节点之前绘制实心边框。外圆角
轮廓与扣除边框宽度的内轮廓使用 alternate fill 形成边框环；没有背景时
中心保持透明，不能使用不透明颜色覆盖来伪造洞。边框过厚使内盒消失时
只保留外轮廓，避免重叠矩形导致角落 alpha 翻倍。颜色默认继承 currentColor。

surface_border 按目标 Direct2D 工厂惰性缓存几何，重绘复用；换工厂整批
重建，创建失败不替换缓存。绘制配对恢复祖先裁剪，原有组层负责 opacity。
padding-box overflow 裁剪和极小外盒 CSS 最小尺寸仍待补齐，不宣称完整
盒模型，也未完成可见 UI、跨 DPI 或大型场景资源验收。

## N08 边框盒模型尺寸接线（绘制待续）

完整串行回归 1833 passed、28 ignored；Clippy、fmt、2681 文件行数门禁及
diff 检查通过。新增 4 项回归覆盖宽度规则、行列/Stack 内缩、绝对定位
包含块和文字重排高度。未生成 EXE、提交、推送或合并。

以现有 DeclarativeSurface.css 的 border-box 和 declarativeSurfaceStyle.ts
映射为依据，新增 surface_box 统一 borderWidth 与 padding 内缩。行列及
Stack 内容盒扣除边框和 padding；绝对定位使用扣除边框但保留 padding 的
包含块。文字使用内容宽度断行，原点移入内容盒，反馈高度和基线只加一次
内缩，避免重排迭代重复累加边框。零面积内容盒不创建最终文字布局。

未声明 borderWidth 不占边框空间；声明无效宽度仍对应 solid 的 medium
默认宽度。百分比、auto、fr 和定位表达式不能当作有效边框宽度求值。
本阶段尚未绘制边框颜色与圆角边框环，也未完成 padding-box overflow
裁剪调整、极小外盒的 CSS 最小尺寸规则；不能标记完整边框功能完成。

## N08 圆角子树裁剪与查询容器上下文

完整串行回归 1829 passed、28 ignored；Clippy、fmt、2679 文件行数门禁及
diff 检查通过。新增 3 项真实 Direct2D 像素回归与 1 项工厂缓存回归通过。
查询容器像素测试选择明确位于弧外的采样点，避免把抗锯齿覆盖误判为失败。
未生成新 EXE、提交、推送或合并。

双轴声明 hidden/auto 的圆角容器（及 Scroll 容器）已使用 Direct2D 几何
mask 裁剪子树；普通 overflow 可见节点不建立 mask，单轴继续已有轴向裁剪
语义。mask 在背景/自身文字之后建立，只包住子节点，并与 opacity 层按树
顺序配对弹出，不能把圆角或透明度遗留给后续兄弟节点。

圆角几何按目标工厂惰性整批创建并缓存，同工厂重绘复用；换工厂时重新
创建，不能跨 Direct2D 工厂复用几何。失败保留旧缓存但不在新工厂目标上
使用，不累积历史缓存。mask 仍有合成层成本，实际大型场景资源尚待验收。

同时修正上一轮圆角 cqw/cqh 使用直接父节点尺寸的问题：与既有布局一致，
必须保留宿主查询容器上下文，只有百分比使用自身两轴、em 使用计算字号。
新增像素回归覆盖此差异，避免嵌套容器让圆角意外缩小。

边框、多值逐角和完整单轴 CSS overflow 归一尚未完成；不能把本增量当作
完整 CSS 盒模型或可见 UI/跨 DPI 验收。

## N08 圆角背景

完整串行回归 1825 passed、28 ignored；Clippy、fmt、2678 文件行数门禁及
diff 检查通过。新增 2 项半径解析与 4 项真实 Direct2D 像素回归通过。
未生成新 EXE、提交、推送或合并。

节点背景已使用完整边界的 Direct2D rounded rectangle，绘制时再应用祖先
矩形裁剪；不能先裁短背景盒子再计算圆角，否则裁剪边缘会改变原始形状。
圆角百分比分别按自身宽/高解析，em 使用当前节点已计算字号，rem/视口/
容器单位复用长度上下文；超大半径按共同缩放系数归一，保持椭圆比例。

背景形状在提交布局时预计算，绘制不创建几何资源或重算长度；零圆角仍走
矩形快速路径。圆角只影响本节点背景，不隐式裁掉普通可见子节点。边框、
圆角 overflow 子树裁剪及多值/逐角语法尚未实现，不能宣称完整圆角盒模型。

真实像素验证透明角/不透明中心、祖先裁剪不变形、普通子内容不被误裁剪，
以及继承字号决定 em 圆角；后续标记继续验证裁剪栈恢复。可见 UI 与跨 DPI
仍需后续验收。

## N08 子树组透明度

完整串行回归 1819 passed、28 ignored；Clippy、fmt、2677 文件行数门禁及
diff 检查通过。新增 4 项真实 Direct2D 像素回归通过，原有颜色/裁剪/叠放
像素测试同步验证后续不透明标记不受层栈污染。无新 EXE 或发布操作。

节点 opacity 已按组语义接入 Direct2D 层：先绘制节点背景、文字及子树，再
整体合成，避免重叠图元逐次 alpha 导致变深。数值按主干规则限制在 0..1，
非数值使用默认 1；透明度 1 不建立组层，透明度 0 的整棵子树不发绘制指令。

提交时根据祖先裁剪和子树可见几何预计算组范围，不能仅使用父盒子而裁掉
overflow:visible 子项。按树深度平衡入/出层，绘制失败也由 owner 弹出剩余
层；复用原有画刷并恢复颜色。不增加定时器或后台动画；半透明组绘制时需
Direct2D 临时合成层，不能据此宣称没有额外 GPU/离屏资源开销。

新增真实像素验证组重叠只应用一次 alpha、嵌套乘积、零透明度子树、可见
溢出及数值默认/截断；后续不透明标记验证组层/裁剪未污染下一次绘制。
当前仍仅有 96-DPI 离屏证据，跨 DPI、大型嵌套场景资源及可见 UI 尚待验收。

## N08 原生文字颜色与节点背景

完整串行回归 1815 passed、28 ignored；Clippy、fmt、2677 文件行数门禁及
diff 检查通过。新增 2 项颜色合同和 3 项真实 Direct2D 像素测试通过。
未生成新 EXE、提交、推送或合并。

HWND Surface 绘制已从统一黑色文字扩展为按场景顺序交织节点背景和文字。
颜色在布局提交时预计算，文字继承父颜色，currentColor 使用当前文字颜色；
背景不继承。覆盖白名单命名色、3/4/6/8 位 hex、逗号 rgb/rgba/hsl/hsla 及 alpha。
颜色输入上限 64 字节，不执行 CSS 表达式或资源读取；不支持主题变量、现代
空格/slash 颜色语法，非法/未支持声明回到继承文字或无背景默认。

绘制指令与文字布局/几何一起原子替换，背景使用祖先裁剪，按深度优先顺序
绘制，避免把所有背景先画完导致 Stack 后层无法遮住前层文字。WM_PAINT
不解析颜色或分配新画刷；复用画刷并恢复调用方颜色，文字裁剪栈仍配对恢复。

真实 WIC/Direct2D 96-DPI 像素测试验证继承/覆盖、Stack 遮挡、背景 alpha 与
裁剪；这不代表跨 DPI 或可见窗口人工验收。边框、圆角、组 opacity、主题
变量、滚动与其他控件仍未完成，不能将本增量标记为完整 CSS 样式支持。

## N08 显式确认恢复候选并重新挂载

完整串行回归 1810 passed、28 ignored；Clippy、fmt、2674 文件行数门禁及
diff 检查通过。新增 5 项回归覆盖取消/明确提交/重复拒绝、模态候选/服务
变化、菜单禁用、无授权不启动及旧候选/容量拒绝。未生成 EXE 或发布。

原生菜单新增“重新挂载恢复候选”，默认否的确认框展示服务、Art、节点和
附件。模态结束后核对同一候选对象及当前服务，队列和应用启动边界再验证
busy、授权、设备、节点占用和等待句柄预算；旧确认不能消费新的同值候选。
发起尝试时消费候选，任何失败均需重新查询，不自动重发 remount 或创建 Art。

后台使用既有 Request::recover，并与首次打开共用真实宿主能力报告。完成
结果进入原有准入、active 确认和窗口显示链；不新增独立窗口或绕过授权。
恢复在途时禁用新的读取/查询/恢复动作，完成后清除“正在挂载”状态。

应用路径已连接文件读取→认证查询→显式确认→后台 remount→准入显示，
但隐藏 HWND/协议夹具不等于可见 UI 或真实 Loom 全链验收；历史记录管理、
失败记录处置、崩溃前日志和其余 N08 控件/运行时仍未完成。

## N08 按需读取恢复文件与认证查询

完整串行回归 1805 passed、28 ignored；Clippy、fmt、2673 文件行数门禁及
diff 检查通过。新增 5 项回归覆盖查询身份/无授权拒绝、文件任务容量/一次
交付、取消/退出回收及设备路径/ADS 拒绝。文件测试仅写 linshi 隔离目录。

原生 Loom 菜单新增“读取 Surface 恢复记录”。文件对话框只取得路径，
读取/严格解码在后台执行，与远程任务共用 4 槽容量和应用完成等待；取消、
服务切换或退出丢弃迟到本地结果但继续回收线程，不删除文件。单份最多
32 条，以查询菜单项显示；非当前服务或缺少 Art 的记录不可查询。

用户选择记录后必须取得当前有效授权，origin/device 与记录一致才发起
只读恢复查询。完成时再次核对当前服务/设备/授权；候选只保留待确认，
不会自动 remount。查询 None 不作为原创建未生效的证明，磁盘 unknown 不改写。
恢复文件读取拒绝设备路径、相对路径和 ADS，避免把非普通文件当成元数据。

按需读取替代启动时全目录扫描；尚未接入候选的显式重新挂载确认、历史文件
管理与崩溃前日志。原生可见文件对话框和实际 Loom 查询尚未人工/在线验收。

## N08 退出恢复元数据落盘

完整串行回归 1800 passed、28 ignored；Clippy、fmt、2670 文件行数门禁及
diff 检查通过。新增 5 项测试验证元数据往返/拒绝边界、只增写/半写保留、
并发配额和后台写入的 WM_QUIT 保留；失败附件回归也核对持久化最新停用版本。
实际文件测试仅写 linshi 隔离目录，无新 EXE、提交、推送或合并。

退出回收与附件结算后，从窗口、未建窗失败记录及未交付任务结果采集最小
恢复提示，并在后台写入原生 profile 的 `surface-recovery/surface-NNN.json`。
保留 origin/device/node/Art、绑定及已确认生命周期版本、unknown/在途标志，
不保存授权 token、场景、参数或错误原文。磁盘记录不是授权或可执行 Binding，
读取只返回提示，不会发网络请求或自动重放操作。

每份至多 32 条、64 KiB，目录采用 128 个固定独占槽位，多个进程并发也不
覆盖旧文件或超出槽位配额。写后 sync；失败/半写文件保留并占槽，严格加载
拒绝截断、额外字段、未知版本及超量输入。此方案不是崩溃前写入日志：
只能保留正常退出时成功落盘的元数据，不能承诺断电时的完整事务恢复。

UI 在后台写入期间继续泵消息并保留 WM_QUIT；无 Surface 记录时不访问 profile。
目录满时显式失败，不自动删除证据。下一步仍需接入启动读取、恢复列表和
显式授权后的查询/确认交互；当前没有自动恢复，也没有完整崩溃恢复保证。

## N08 未建窗附件的退出停用

完整串行回归 1795 passed、28 ignored；Clippy、fmt、2666 文件行数门禁及
diff 检查通过。新增 5 项回归覆盖无窗口停用、身份/unknown/重复拒绝、启动
失败与 unknown 不重发、在途记录保留和集合 ACK 回交。无新 EXE 或发布操作。

失败准入记录新增独立生命周期 owner，原始挂载 Outcome 不被改写。已确认
绑定且 origin/device/node 一致时，即使场景不可用也可进行一次 inactive；
unknown、重复附件及身份不一致记录不创建停用 owner。后台回交沿用同附件、
generation、目标状态和版本检查，在途记录不可提前取出，失败/unknown 不重发。

退出的同一 2 秒宽限阶段现在也等待挂载线程，迟到挂载按已关闭准入登记，
不为清理临时建窗；可确认附件调度停用，最终只有已确认 inactive/disposed
才通过结算。窗口与未建窗附件共用请求准备边界，先取得有效授权并校验
设备一致，再读取连接配置和启动后台事务。

容量不足、未知创建及重复证据继续保留为未确认；跨进程恢复持久化和失败
交互处置仍未完成。未连接真实 Loom 验收，不将协议测试等同线上停用成功。

## N08 退出前有界停用

完整串行回归 1790 passed、28 ignored；Clippy、fmt、2664 文件行数门禁及
diff 检查通过。新增 4 项测试覆盖空集合、截止、WM_QUIT 保留、后台线程
同步请求 UI 消息时继续泵消息。未生成新 EXE、提交、推送或合并。

退出在统一撤权之前关闭本地 Surface 窗口，并复用既有一次性 inactive 调度。
已在途 active 先回交，确认后才调度 inactive；unknown、撤权及已尝试失败的
停用不会自动重发。等待采用固定的 2 秒绝对截止，期间继续泵窗口消息，
仅等待生命周期线程，不会被无关媒体帧或未消费结果反复唤醒。

成功、失败、超时均继续原有撤权和后台 drain，再执行附件结算门禁；超时
不丢弃 owner、不伪造 ACK、不跳过线程回收。WM_QUIT 暂存，结束后恢复退出码。
2 秒只限制优雅停用等待，并非整个应用退出/驱动回收耗时保证。

尚未执行真实 Loom 退出停用验收；挂载中或准入失败附件不会为清理而临时
建窗，仍按未确认记录处理。恢复持久化和失败处置尚未完成。

## N08 应用退出后的附件结算门禁

最终完整串行回归 1786 passed、28 ignored；Clippy、fmt、2662 文件行数
门禁及 diff 检查通过。新增 5 项回归覆盖迟到 inactive/active ACK、unknown、
未匹配结果、失败准入及 mounted 窗口退出。

应用回收后台线程后，会将可匹配的 Surface 生命周期结果交回窗口 owner，
再检查未确认附件。此阶段不建窗、不显示、不发送请求、不重试 unknown，
也不清空失败证据。已确认 inactive/disposed 可通过附件状态检查；mounted、
active、unknown、仍在途、未匹配结果及失败准入均报告退出状态未确认。
取消请求返回 Stopped 不会抹去 owner 中已经确认的生命周期和版本。

该门禁解决“线程已退出就被当成附件已结算”的缺口，不等于实现退出停用
或恢复持久化。结果仍仅在本进程内保存，退出后不能据此承诺可恢复；在线
停用、持久化和失败处置仍是后续任务。测试使用隐藏窗口和已撤权作业，
不会读取真实服务配置或连接真实 Loom。

## N08 本地关闭停用与组合等待预算

最终完整串行回归 1781 passed、28 ignored；Clippy、fmt、2660 文件行数
门禁及 diff 检查通过。原前端删除节点才发送 disposed；本地 Surface 窗口
关闭不等同删除节点，因此应用在授权有效、确认状态已知时调度一次 inactive。
Activation 记录本次目标状态并验证下一版本；active 在途时先等回交，unknown
或撤权不猜版本、不重发。确认停用后菜单显示“Surface 窗口已停用”，保留附件。

纯状态及隐藏窗口测试覆盖 active→inactive 单调版本、关闭回交前不可收割、
unknown 不重发、撤权不发停用、回交后保持本地关闭且未 disposed。未执行
真实服务停用；应用退出网络收尾及恢复持久化尚未实现。

检查发现原 Live 最坏等待预算已达 63，新增 Surface HANDLE 不能无条件相加。
新增组合容量检查，覆盖 Live 启动、Art 请求及挂载接纳，保留无 Surface 时
16 路 Live 上限；测试覆盖 16+0、15+3、14+4 可容纳以及超限和算术溢出拒绝。

失败处置、恢复持久化、完整样式/滚动/资源控件仍未完成。未执行 ignored
环境或在线 Loom 验收，无新 EXE、提交、推送或主干合并。

## N08 用户打开 Art 的原生预览入口

最终完整串行回归 1777 passed、28 ignored；Clippy、fmt、2660 文件行数
门禁及 diff 检查通过。Loom 菜单新增“打开 Art（原生预览）”，复用 Win32
输入对话框，校验 Art ID 并绑定发起窗口 UnitId。取消、窗口失效或模态期间
服务切换不提交；菜单回调只排队，不执行网络。应用取得当前有效租约后
启动 Temporary 挂载，单次 busy 防重复，并拒绝已有节点/待处理记录或满容量。

宿主报告仅列出 declarative 与 view/row/column/stack/text，不宣称输入、滚动、
图片、脚本或 Shader。应用结算反馈激活中、失败、已打开和关闭待处理状态。
测试覆盖明确提交/取消、节点绑定、过期服务选择、无授权不启动、容量和
失败记录阻止重复挂载。本轮未向真实 Loom 服务发送挂载，也未验收可见对话框。

失败记录交互处置、关闭上报、恢复持久化和完整样式/滚动/资源控件仍未完成，
预览入口不代表全部 Art 已支持。未执行 ignored 环境或在线 Loom 验收，
无新 EXE、提交、推送或主干合并。

## N08 应用调度 active 与完成回交

最终完整串行回归 1775 passed、28 ignored；Clippy、fmt、2658 文件行数
门禁及 diff 检查通过。应用在准入后为 pending 窗口申请独立请求租约，将
唯一 Owner 交给现有 Sessions active 后台事务；启动失败回收原 Owner 并
记录失败，不自动重试。任务完成后按附件寻找 pending 窗口，再执行已有
身份/版本验证。仅成功且确认 active、授权仍有效时显示；显示状态避免
每次结算重复 ShowWindow，已确认 active 的挂载也可通过门禁。

无法匹配和恢复场景结果原样留在 Sessions；有界扫描避免未匹配项阻挡
其他激活结果。新增测试覆盖集合 active 回交、未匹配证据归还和跨拒收项
交付，既有迟到 ACK/撤权关闭测试继续通过。测试使用隐藏窗口与协议夹具，
尚未执行在线 active 请求或可见窗口验收。

用户发起入口、失败处置、关闭上报、恢复持久化与完整样式/滚动/资源控件
仍未完成。未执行 ignored 环境或在线 Loom 验收，无新 EXE、提交、推送
或主干合并。

## N08 激活事务的单 Owner 移交

最终完整串行回归 1773 passed、28 ignored；Clippy、fmt、2658 文件行数
门禁及 diff 检查通过。新增 Activation 保存确认绑定及可移出的唯一 Owner；
一次 take 后不重复发起，移出期间不能显示。restore 验证 origin/device/node、
实例/附件和 generation，仅接回原确认状态或下一版本 active；拒绝时归还
传入 Owner，不丢其他事务证据。失败/unknown 回交后不会自动重发。

AuthorizedWindow 接入 begin_activation/finish_activation，生命周期 getter
明确返回 Option；关闭但 Owner 尚在后台时，窗口集合不提前收割。测试覆盖
单次移交、错误转换拒绝、unknown 保留，以及撤权关闭后收到 active ACK
只保存证据而不重新显示。未启动真实后台激活或在线生命周期请求。

应用调度 active 作业、成功后显示、用户发起入口、失败处置、关闭上报与
恢复持久化仍待接通；完整样式、滚动和资源控件未完成。未执行 ignored
环境或在线 Loom 验收，无新 EXE、提交、推送或主干合并。

## N08 已确认生命周期状态与显示门禁

最终完整串行回归 1770 passed、28 ignored；Clippy、fmt、2657 文件行数
门禁及 diff 检查通过。Binding 保留挂载/ACK 确认的 Lifecycle，而不只保存
版本；Owner::confirmed_active 排除 unknown 与 disposed。AuthorizedWindow
显示前同时检查授权和已确认 active，mounted 不能显示；重新构造 disposed
绑定的 Owner 保持终态。测试覆盖错误 ACK、激活、停用、恢复未知和销毁。

首轮完整回归发现 Live playback 夹具在媒体先产帧时立即撤权，另一控制
连接仍可能未读完请求头，触发 UnexpectedEof。改为服务端确认两个请求头
已到达后再撤权，不改变生产行为；最终完整回归通过，该用例额外连续 5 次
通过。此修改固定撤权测试的时序前提，不声称已覆盖所有网络取消竞态。

后台 active 事务到窗口 owner 的交接及成功显示仍未接通，用户发起入口、
失败处置、关闭上报/恢复持久化、完整样式、滚动和资源控件仍待完成。
未执行 ignored 环境或在线 Loom 验收，无新 EXE、提交、推送或主干合并。

## N08 挂载完成结果进入应用窗口集合

最终完整串行回归 1768 passed、28 ignored；Clippy、fmt、2656 文件行数
门禁及 diff 检查通过。应用 poll_remote_live 在收割挂载任务后调用准入，
核对当前选中服务并申请新租约，再由有界集合创建隐藏 AuthorizedWindow。
窗口保留原 Request，避免丢失 Art/node 恢复上下文；不在准入时自动显示。

失败、未知和重复附件结果保留为 FailedAdmission，与窗口共享 4 项容量；
不反复建窗。Sessions::deliver_attach 在接收方容量不足时原样放回结果。
测试覆盖容量退回、未知不申请授权、成功准入、重复不建第二个窗口，以及
证据保留、撤权关闭和交付失败容量恢复。

应用用户发起挂载入口、失败记录交互处置、active 生命周期确认后显示与
关闭上报/恢复持久化仍待完成，不能宣称用户侧完整链完成。完整样式、滚动
与资源控件也待实现。未执行 ignored 环境或在线 Loom 验收，无新 EXE、
提交、推送或主干合并。

## N08 授权窗口集合接入应用等待

最终完整串行回归 1765 passed、28 ignored；Clippy、fmt、2655 文件行数
门禁及 diff 检查通过。新增最多 4 项的 Windows 集合，容量或预留失败归还
窗口；终态仍占容量直到 take_closed 收割，保留附件 owner。只为存活窗口
生成等待句柄与最近截止，关闭后不继续等待已触发事件或已过期截止。

Application 在 next_live_message 中先结算窗口授权，将撤权句柄加入现有
有界等待集合，使用 next_until 等待最近截止；shutdown 关闭本地 Surface
窗口。隐藏集合测试覆盖容量归还、撤权关闭、终态不再等待及附件证据收割。

当前应用尚未把挂载完成结果接纳到该集合，因此用户侧完整链仍未完成，
也未完成正式生命周期上报/恢复持久化。完整样式、滚动和资源控件待实现。
未执行 ignored 环境或在线 Loom 验收，无新 EXE、提交、推送或主干合并。

## N08 应用等待函数的绝对截止支持

最终完整串行回归 1764 passed、28 ignored；Clippy、fmt、2654 文件行数
门禁及 diff 检查通过。application_live_wait::next_until 同时等待句柄、消息
和显式单调截止时间；到期合成 WM_NULL 供应用结算，不投递额外消息，不
创建定时器或辅助线程。无截止时保持原无限等待路径，限制最多 63 个句柄。

截止先于读取排队输入，避免持续消息延迟到期结算；输入保留给下一轮。
每次系统唤醒重新计算剩余时间，向上取整亚毫秒，超长等待分段而不延长
原截止。真实 Windows 等待测试覆盖无句柄到期唤醒、到期优先且输入不丢失，
原 worker 完成及 WM_QUIT 语义测试继续通过。

此函数已被旧 next 包装复用，但应用尚未传入 Surface 截止与撤权句柄；
不能据此宣称授权窗口自动失效关闭完成。生命周期、完整样式、滚动与资源
控件仍待接通。未执行 ignored 环境或在线 Loom 验收，无新 EXE、提交或推送。

## N08 授权窗口 owner 与失效关闭

最终完整串行回归 1762 passed、28 ignored；Clippy、fmt、2654 文件行数
门禁及 diff 检查通过。AuthorizedWindow 组合 Outcome 准入、隐藏 SurfaceWindow、
独立租约、撤权事件与附件生命周期 Owner；创建后和显示前后复核，refresh
在撤销/过期时关闭本地窗口。失效关闭不修改远端 disposed，也不自动发请求。

隐藏窗口测试覆盖撤销事件触发后 refresh 关闭、截止边界关闭、时间回退
不复活、关闭后拒绝显示，以及保留绑定版本、原 Outcome 和租约移交。测试
显式调用 refresh，尚未证明应用自动等待结算；调用方必须登记事件与截止。

应用等待集合、挂载完成收割和服务端生命周期事务尚未接入该 owner；完整
样式、滚动和资源控件仍未完成。未执行 ignored 环境或在线 Loom 验收，
无新 EXE、提交、推送或主干合并。

## N08 租约撤销的本地等待事件

最终完整串行回归 1760 passed、28 ignored；Clippy、fmt、2652 文件行数
门禁及 diff 检查通过。Lease::revocation_notification 惰性提供独立的手动
重置事件；重复订阅共享同一 HANDLE，撤销后终态保持有信号，订阅者可在
Lease 释放后继续观察该终态，最后引用释放 HANDLE。不创建线程或轮询。

注册和撤销共锁，缓存失效及 Lease drop 都触发通知，不占网络取消的
native_wake 槽。测试覆盖同一 HANDLE 复用、释放后的终态、撤销先于注册，
以及 32 轮注册/撤销竞态。Lease::expires_at_ms 提供已有会话截止时间，不
暴露 token；事件仅代表撤销，过期必须由等待截止和 validate 另行处理。

该通知尚未接入应用等待集合和 Surface 窗口关闭路径，因此不能宣称持续
撤权已完成。附件生命周期、完整样式、滚动与资源控件仍未完成。未执行
ignored 环境或在线 Loom 验收，无新 EXE、提交、推送或主干合并。

## N08 挂载结果本地交付准入

最终完整串行回归 1758 passed、28 ignored；Clippy、fmt、2650 文件行数
门禁及 diff 检查通过。Lease::validate 提供不返回 token 的本地复核：固定
origin、过期边界和撤销。Outcome::scene_for 在场景解析前后检查租约，拒绝
unknown/失败/无绑定结果，核对当前设备、绑定 origin/node 和快照实例/附件
身份及正版本，限制快照大小，只接受声明式运行时，复用已有场景解析。

新增测试覆盖有效交付、过期、缓存撤销、unknown、设备/快照身份错配及 JS
运行时拒绝。准入只借用 Outcome，失败保留原绑定与结果，不重发网络请求。
该方法是给定时间的检查，不构成长期显示许可，也不替代服务端认证传输。

应用尚未将该准入和窗口创建连通；显示期间撤销/过期、附件生命周期仍待
接入，完整样式、滚动和资源控件未完成。未执行 ignored 环境或在线 Loom
验收，无新 EXE、提交、推送或主干合并。

## N08 DPI 建议矩形与绘制资源复用

最终完整串行回归 1755 passed、28 ignored；Clippy、fmt、2648 文件行数
门禁及 diff 检查通过。SurfaceWindow 对 WM_DPICHANGED 应用建议矩形，
checked_sub 和尺寸上限拒绝溢出/无效范围，不改变激活状态或 Z 序。
隐藏 HWND 测试以合成消息验证建议矩形应用、溢出矩形拒绝和后续绘制。
这不等于真实跨屏 DPI 验收，Host 仍使用 GetDpiForWindow 获取真实 DPI。

Host 将画刷与 HWND 绘制目标一起持有，不再每次重绘创建画刷；重复绘制
回归直接核对目标及画刷 COM 对象身份保持一致。尺寸/DPI 变化或绘制失败
释放成对资源，重建目标时重新创建画刷。未给出资源消耗或帧时性能结论。

应用授权复核、撤销和附件生命周期到窗口的连接仍未完成，不能直接消费
挂载结果就显示。完整样式、滚动及资源控件也仍待完成。未执行 ignored
环境测试或在线 Loom 联调，无新 EXE、提交、推送或主干合并。

## N08 Surface 窗口消息与关闭生命周期

最终完整串行回归 1754 passed、28 ignored；Clippy、fmt、2648 文件行数
门禁及 diff 检查通过。新增 SurfaceWindow，隐藏创建并设置捕获排除，显式
show 才显示；连接 WM_PAINT 到 Host，尺寸/DPI 消息使窗口失效，场景替换
成功后请求重绘。绘制前暂时取出 Host，处理同步重入，关闭后不恢复 owner。

隐藏窗口实测覆盖 WM_PAINT、场景替换、尺寸变化、取出 Host 期间收到
WM_CLOSE、资源释放、关闭后更新拒绝及重复 close。无定时绘制循环，绘制
错误保存在窗口状态供调用方取走。尚未做用户视觉、跨屏 DPI 或滚动验收。

应用授权绑定结果到窗口的创建入口仍未接通，完整样式与资源控件尚未完成。
未执行 ignored 环境测试或在线 Loom 联调，无新 EXE、提交、推送或合并。

## N08 HWND 文字绘制后端

最终完整串行回归 1753 passed、28 ignored；Clippy、fmt、2646 文件行数
门禁及 diff 检查通过。新增 desktop/surface_host，绑定同线程 HWND，按
客户区物理尺寸和 GetDpiForWindow 计算逻辑视口。重复绘制复用布局；视口
变化后成功重排再替换状态并重建绘制目标，零客户区跳过。BeginDraw 与
EndDraw 配对，绘制失败释放目标，等待外部重绘请求而非忙循环重试。

隐藏 HWND 实测覆盖创建、两次绘制复用文字布局、调整尺寸后重排、错误
场景替换保留旧内容，以及先释放 Host 再销毁窗口。窗口未显示；这不是
用户视觉或混合 DPI 验收。当前默认黑字白底，不代表完整样式实现。

应用 Surface 创建入口、WM_PAINT 消息连接、滚动与资源控件仍未完成。
未执行 ignored 环境测试或在线 Loom 联调，无新 EXE、提交、推送或合并。

## N08 呈现状态保留同轮节点几何

最终完整串行回归 1752 passed、28 ignored；Clippy、fmt、2644 文件行数
门禁及 diff 检查通过。新增独立 Geometry，从成功 Tree 保存每个节点的
ID、类型、父索引、位置、尺寸与可见矩形，不借用场景，不重新执行测量。
包含容器、零面积和被裁剪的节点，节点数量仍受场景上限约束。

Presentation 同时保留 Scene、Geometry 与 TextFrame；替换及重排只在整轮
准备成功后更新。新增嵌套场景回归核对父索引及文字几何/裁剪一致性，现有
失败替换和成功重排测试增加几何断言。几何不代表滚动后的坐标、事件命中
或可操作权限；未实现的资源控件仍拒绝准备，不以保存类型冒充控件实现。

生产窗口接入、滚动和资源控件仍未完成；未执行窗口、多 DPI、ignored 环境
或在线 Loom 验收，无新 EXE、提交、推送或主干合并。

## N08 独立文字快照与呈现状态所有权

最终完整串行回归 1751 passed、28 ignored；Clippy、fmt、2643 文件行数
门禁及 diff 检查通过。TextFrame 不再借用场景节点，TextItem 保存有界节点
ID 和原生布局；PreparedScene::into_text 可释放借用布局树并转移快照。

新增 Presentation 同时拥有 Scene、文字快照和尺寸。场景替换、尺寸/样式
重排先完整准备，成功后替换；失败保留旧场景与快照，不保留历史队列。
测试覆盖原场景和测量 owner 释放后快照仍可读、失败替换/重排保护旧内容、
成功更新，以及经 Presentation 实际离屏绘制文字的像素回归。

这是接入长期窗口状态的所有权前置工作，不代表生产窗口已接通；暂未保留
完整控件几何或实现资源控件。未执行窗口、多 DPI、ignored 环境或在线 Loom
验收，无新 EXE、提交、推送或主干合并。

## N08 文字离屏像素验证

最终完整串行回归 1749 passed、28 ignored；Clippy、fmt、2641 文件行数
门禁及 diff 检查通过。新增 WIC/Direct2D 离屏测试实际执行 TextFrame::draw：
裁剪范围内存在文字像素，范围外透明；随后在范围外绘制标记验证裁剪栈
恢复。同一 DirectWrite Layout 绕过祖先裁剪时，边界外存在字形像素，作为
正对照排除空绘制或文字本身未越界造成的假通过。

这是 96 DPI 离屏像素证据，不是窗口视觉验收或多 DPI 验证。PreparedScene
尚未接入生产窗口；滚动交互、资源控件和完整 Surface 运行链仍未完成。
未执行 ignored 环境测试、在线 Loom 联调，无新 EXE、提交、推送或合并。

## N08 祖先裁剪与不可见文字测量

最终完整串行回归 1748 passed、28 ignored；Clippy、fmt、2640 文件行数
门禁及 diff 检查通过。未执行 ignored 环境测试、在线 Loom 联调或窗口
视觉验收，无新交付 EXE、提交、推送或主干合并。

surface_clip 按 Tree 的父索引累计根宿主边界与逐轴祖先裁剪：普通容器
处理 overflowX/overflowY 的 hidden/auto，scroll 裁剪双轴。文字快照保存
祖先裁剪与自身矩形的交集，绘制时跳过空交集，并成对恢复裁剪栈。

裁剪不改变布局几何。完全不可见的非零面积文字仍建立 DirectWrite 布局，
参与高度与基线反馈；测试覆盖逐轴边界、scroll 双轴、布局不受裁剪影响，
以及真实文字换行和 prepare 保留不可见文字。未执行像素输出验证，不能
把这些检查视为视觉验收。滚动状态/变换、资源控件和窗口集成仍未完成。

## N08 文字高度反馈与有界场景准备

最终完整串行回归 1745 passed、28 ignored；Clippy、fmt、2638 文件行数
门禁及 diff 检查通过。未执行 ignored 环境测试、在线 Loom 联调或窗口
视觉验收，无新交付 EXE、提交、推送或主干合并。

NativeMeasure::prepare 将 Tree 分配、最终文字重排和高度/基线反馈组成
最多 8 轮的准备流程。每轮将最终文字度量反馈到文字测量及容器固有尺寸，
重新计算兄弟位置；稳定后一起交付 PreparedScene 的 Tree 与 TextFrame。
初始宽度基准不被断行宽度覆盖，避免 flex basis 逐轮漂移。每次 prepare
从初始反馈重建，不收敛返回错误，不交付部分场景。

新增真实场景测试覆盖两列文字收缩换行后，自动高度行容器增高并推开
后续兄弟，以及重复准备稳定、空场景一次完成。此流程仍不等于通用 CSS
布局收敛器；零字号、复杂 grid/包含块、资源控件、祖先裁剪/滚动与窗口
绘制仍未完成，环境/样式变化需重建测量 owner。

## N08 最终文字宽度重排与绘制快照

最终源码完整串行回归 1743 passed、28 ignored；Clippy、fmt、2636 文件
行数门禁与 diff 检查通过。测试覆盖收缩后实际换行、溢出、零面积、其他
场景拒绝及正小数字号/尺寸。绘制接口已编译，尚未窗口视觉验收；未执行
ignored 环境测试或在线 Loom 联调，无新 EXE、提交、推送或主干合并。

NativeMeasure::text_frame 根据同场景 Tree 的最终文字矩形重建 DirectWrite
布局，不重新采用节点初始 width，因此 flex 收缩后按实际分配宽度换行。
快照持有 Layout、节点身份、位置、分配尺寸和实际溢出量，失败不交付部分
结果。零面积文字不创建布局，正小数字号/尺寸允许进入原生测量。

TextFrame::draw 复用快照布局，按各文字矩形成对 push/pop 裁剪，即使入口
返回错误也恢复裁剪栈；颜色、祖先裁剪、滚动和绘制周期仍由宿主负责。
目前没有把重排高度反馈父容器，也未完成滚动/窗口集成或视觉验收，不能
把文字绘制接口视为完整 Surface renderer。

## N08 继承字号接通几何长度上下文

最终完整串行回归 1740 passed、28 ignored；Clippy、fmt、2634 文件行数
门禁及 diff 检查通过。未执行 ignored 环境测试、在线 Loom 联调或窗口
视觉验收，无新交付 EXE、提交、推送或主干合并。

Measure 新增只读 font_size 合同，NativeMeasure 从身份绑定的样式表提供
计算后字号；Tree 使用 arrange_with_fonts 接到容器、stack 和定位几何。
父 padding/gap 按父字号，子宽高/min/max/relative/absolute 偏移按子字号，
不重复解析 fontSize，rem 和 cqw/cqh 基准保持不变。原有纯几何入口兼容。

新增真实 NativeMeasure → Tree 测试，验证父 2em、子 2em 的不同计算字号
同时用于 padding、宽高和偏移；stack 与 absolute 测试验证子字号及非有限
字号拒绝。分配后重排与尺寸反馈、零字号、资源控件、滚动裁剪和窗口绘制
仍待完成，此处修复几何与文字测量上下文不一致，不代表 renderer 完成。

## N08 原生测量 owner 接入节点树

最终完整串行回归 1738 passed、28 ignored；Clippy、fmt、2634 文件行数
门禁与 diff 检查通过。新增真实原生嵌套场景布局及外部同 ID 节点/未实现
叶子拒绝测试。未执行 ignored 环境测试、在线 Loom 联调或窗口视觉验收，
无新交付 EXE、提交、推送或主干合并。

新增 NativeMeasure，组合单场景 Styles、DirectWrite Engine 和有界容器
固有尺寸预遍历，实现 Tree::build 的 Measure 接口。支持 text 与已有
view/row/column/stack/scroll；固有尺寸包含内边距、普通流间距和子项尺寸，
absolute 不计入容器普通流固有尺寸。图片和控件未实现则明确拒绝，不给假尺寸。

真实嵌套场景测试已走完 Scene → 继承样式 → DirectWrite → 容器固有尺寸
→ Tree 布局。缓存属于本次上下文，变化须重建；首次预遍历不等于最终
宽度下收敛，分配后文字重排、尺寸约束反馈、几何字体上下文完整接线和
窗口绘制仍未完成。零字号、资源/控件、滚动裁剪及 UI 挂载也仍待开发。

## N08 全树文字样式继承与节点身份绑定

最终完整串行回归 1736 passed、28 ignored；Clippy、fmt、2632 文件行数
门禁与 diff 检查通过。未执行 ignored 环境测试、在线 Loom 联调或窗口
视觉验收，无新交付 EXE、提交、推送或主干合并。

新增 surface_text_styles，对单个 Scene 有界预计算继承字体、字号、字重、
空白、对齐和计算后行高。子项 em/% 以父字号计算，父行高按计算后值继承。
表项同时保存节点引用，不能仅凭相同 ID 将旧场景样式用于新场景。

Styles::layout_text 直接消费已解析样式调用 DirectWrite，不再通过
layout_node 二次解析相对字号。测试从真实嵌套场景到 DirectWrite 验证
字号只换算一次、计算后行高继承，以及同 ID 替换节点拒绝。该表仍需与
完整原生测量 owner 集成；容器固有尺寸、文字重排、零字号、滚动裁剪和
窗口绘制仍未完成，不代表整棵场景已经可渲染。

## N08 节点树布局遍历与场景坐标

完整串行回归 1734 passed、28 ignored；最终树布局定向复核 2 passed。
Clippy、fmt、2630 文件行数门禁及 diff 检查通过。测试验证真实嵌套节点
的坐标累计、父索引、测量次数和失败重建。未执行 ignored 环境测试、在线
Loom 联调或窗口视觉验收，无新 EXE、提交、推送或主干合并。

新增 surface_layout_tree，将 Scene 的嵌套行列/stack 容器递归接到布局
入口。每个非根节点调用一次 Measure 合同，尺寸校验沿用容器边界；子项
局部位置累计成场景坐标。输出保留节点引用、父索引和深度优先顺序，
节点/深度/坐标范围有界，无固定点重试或重复测量循环。

几何构建失败不返回部分树，调用方可保留上一棵有效树；测量回调自身缓存
副作用不由布局层回滚。当前 Measure 为可替换接口，尚未完成原生宿主的
全树字体继承、容器固有尺寸测量和最终分配宽度下的文字重排。布局遍历
不等于完整 renderer，裁剪/滚动/绘制/UI 挂载仍需继续接线。

## N08 stack 共享区域重叠布局

完整串行回归 1732 passed、28 ignored；最终容器定向复核 8 passed。
Clippy、fmt、2628 文件行数门禁与 diff 检查通过。测试验证共享区域、场景
顺序、gap 不参与分配，以及垂直对齐和相对/绝对定位组合。未执行 ignored
环境测试、在线 Loom 联调或窗口视觉验收，无新 EXE、提交、推送或合并。

对照主干 .surface-stack > * 的共享 grid-area 规则，容器入口新增 stack
分支，子节点在给定内容区域内重叠并保持场景顺序。复用显式尺寸、min/max、
垂直对齐/基线及相对定位；绝对定位仍单独求解。gap/grow 不作为行列参数
分配给重叠项，不把 stack 静默当成 column。

当前是给定内容区域的单区重叠布局，复杂 grid 轨道固有尺寸和非默认 justify
分布尚未实现，非默认 justify 返回 Unsupported。递归测量、文字重排、
完整包含块、滚动裁剪及窗口绘制仍未完成，不能宣称完整 CSS grid 兼容。

## N08 行列内相对定位与显式绝对定位

最终完整串行回归 1730 passed、28 ignored；Clippy、fmt、2627 文件行数
门禁及 diff 检查通过。新增 relative 保留兄弟占位和 absolute 排除 gap/
普通流、边锚点/双边尺寸测试。未执行 ignored 环境测试、在线 Loom 联调
或窗口视觉验收，无新交付 EXE、提交、推送或主干合并。

容器现在支持 relative 子项：先完成普通流分配，再应用 left/right 和
top/bottom 偏移，保留兄弟占位。absolute 子项退出主轴/交叉轴和 gap 计算，
通过单独定位模块求解；输出仍按原场景顺序，便于后续稳定绘制顺序。

显式绝对定位支持边锚点、双边自动尺寸、显式宽高和 min/max。当前仅支持
直接父级为 relative/absolute 包含块，双轴至少各一个锚点；缺失包含块或
需要 static-position 推断时返回 Unsupported。祖先包含块搜索、stack、
递归测量/文字重排、裁剪滚动和窗口绘制仍未完成，不宣称完整 CSS 定位兼容。

## N08 场景容器接入主轴与交叉轴计算

最终源码完整串行回归 1728 passed、28 ignored；Clippy、fmt、2626 文件
行数门禁与 diff 检查通过。测试覆盖实际行列节点、padding/gap/grow/对齐、
百分比及查询容器单位、测量身份替换和非法输入。未执行 ignored 环境测试、
在线 Loom 联调或窗口视觉验收，无新交付 EXE、提交、推送或主干合并。

新增 surface_container，将 row/column/view/scroll 的实际子节点、测量尺寸
和基线组合到主轴/交叉轴分配。读取 padding/gap/justify/align、子项显式
尺寸、min/max 和 grow，输出保留节点引用的本地矩形，不复制场景树。
测量数量及 ID 必须逐项匹配，非有限尺寸不能被显式宽高掩盖。

百分比宽高使用去除内边距后的父内容轴；cqw/cqh 按主干 CSS 的宿主查询
容器计算，不随普通嵌套 row/column 重置。绝对/相对定位、stack 和列基线
尚未实现则返回 Unsupported，不静默按普通流布局。scroll 目前仅分配内容，
没有滚动交互。递归树测量、分配后文字重排、裁剪、绘制和 UI 挂载仍未完成。

## N08 容器交叉轴与基线几何

最终完整串行回归 1724 passed、28 ignored；Clippy、fmt、2624 文件行数
门禁与 diff 检查通过。新增拉伸与显式尺寸、边界约束、文字/非文字基线、
负偏移溢出及非法几何测试。未执行 ignored 环境测试、在线 Loom 联调或
窗口视觉验收，无新交付 EXE、提交、推送或主干合并。

新增 surface_cross_axis，处理 stretch/start/center/end/baseline。输入为
固有尺寸、可选显式尺寸、min/max 和可选实测基线，显式尺寸优先于拉伸；
无文字基线的盒子以底边合成。对齐输出位置、尺寸、公共基线和两侧溢出。
居中/末尾对齐超过容器时保留负偏移，不自行裁剪或改为起始对齐。

节点数、输入尺寸与基线均有界，非法几何在分配结果前拒绝。此处消费
测量数据，不创建文字布局或窗口；与完整节点树、滚动/裁剪和绘制 owner
的接线仍未完成，也不宣称完整 CSS flex/grid 兼容。

## N08 原生文字显式行高和基线

完整串行回归 1720 passed、28 ignored；Clippy、fmt、2622 文件行数门禁
与 diff 检查通过。未执行 ignored 环境测试、在线 Loom 联调或 Surface
窗口视觉验收，无新交付 EXE、提交、推送或主干合并。

Style 增加可选计算后行高，节点 lineHeight 复用长度解析，em/% 相对节点
已解析字号；父行高按计算后值继承。DirectWrite 先读取实际行度量，再将
额外行距平分至文字上下，设置 uniform line spacing，并重新读取尺寸。
Layout 暴露第一行 baseline，供后续容器基线对齐；测量和绘制仍使用同一
布局对象。行度量数量在内存分配前限制为 65537，拒绝非法行高和基线。

已核对 Microsoft SetLineSpacing 官方接口说明；测试覆盖两行文本的明确
行高、实际基线变化、非有限值拒绝以及字号变更后 lineHeight 百分比。
极端字体的负基线返回错误，不猜测替代度量。零字号策略、完整树布局、
跨字体视觉一致性、选择及 Surface 窗口视觉验收仍未完成。

## N08 声明式 text 节点接入 DirectWrite

最终源码完整串行回归 1719 passed、28 ignored；Clippy、fmt、2622 文件
行数门禁及 diff 检查通过。未执行 ignored 环境测试、在线 Loom 联调或
Surface 窗口视觉验收，无新交付 EXE、提交、推送或主干合并。

新增 surface_text_node，Engine::layout_node 读取场景 text 节点的 props.text，
将 fontSize/fontWeight/whiteSpace/textAlign 映射到真实原生文本布局。相对
尺寸复用 surface_length，em 和字号百分比明确相对父字号而非容器宽度；未提供或不在白名单的属性
保留传入父样式，字体家族继承调用方。非 text 节点返回错误，不静默降级。

新增测试从实际 Scene 解析到 DirectWrite，验证 2em 不采用过期 context.font、200% 相对父字号、
字重上限、字号实际测量差异、nowrap/normal 换行和无效属性继承。该入口
尚未接到整棵节点树的布局/绘制 owner；line-height、零字号显示策略、选择
与输入、其他节点、UI 挂载和视觉验收仍未完成。

## N08 DirectWrite 文字测量与绘制接口

最终完整串行回归 1717 passed、28 ignored；Clippy、fmt、2620 文件行数
门禁与 diff 检查通过。未执行 ignored 环境测试、在线 Loom 联调或 Surface
窗口视觉验收，无新交付 EXE、提交、推送或主干合并。

新增 surface_text：Engine 持有 DirectWrite 工厂，Layout 持有原生文本布局
及实际测量值。换行、字体尺寸/字重和左右/居中对齐由 DirectWrite 处理，
不以字符数量猜测字宽。Direct2D 绘制复用同一布局对象，调用方管理绘制周期。

支持 normal/nowrap/pre/pre-wrap 空白和换行模式，统一 CRLF，同时保留 NBSP
及 Unicode 代理对。UTF-16 最多 65536 单位，拒绝 NUL、非有限/超限尺寸和
非法字体名；不引入新依赖、全局缓存、窗口或定时器。

本轮测试直接调用 Windows DirectWrite 验证中英文实际换行、单行溢出、
多行测量、空白规范化及边界拒绝。绘制接口已经编译，但尚未在 Surface
窗口进行视觉验收；节点树布局、样式继承、line-height、文字选择、UI 挂载
及其他节点渲染仍未完成，不能宣称 Surface renderer 整体可用。

## N08 行列容器主轴空间分配

最终完整串行回归 1715 passed、28 ignored；Clippy、fmt、2618 文件行数
门禁及 diff 检查通过。新增 grow 小于 1 且触顶后的再分配、加权收缩与
最小尺寸、溢出、六种对齐、空/单项及非法输入测试。未执行 ignored 环境
测试或在线 Loom 联调，无新交付 EXE、提交、推送或主干合并。

新增 surface_flow 数值分配器：基础尺寸、最小/最大尺寸、grow/shrink、gap
及 start/center/end/space-between/space-around/space-evenly。按基础尺寸
加权收缩，在 min/max 处冻结后重新分配；grow 总和小于 1 时保留剩余空间。
中间累加用 f64，临时数组按节点数分配并复用，冻结循环有节点数上界。

最小尺寸和间距无法容纳时明确返回 overflow，不隐式裁剪；空集合和单项
对齐有定义。输入有界且检查非有限值。此模块只计算已测量子项的主轴，
尚未连接节点树布局、交叉轴、文字测量、滚动及绘制，不宣称完整 CSS
flex/grid 兼容或 renderer 完成。

## N08 声明式长度解析与原生数值求解

最终源码完整串行回归 1711 passed、28 ignored；Clippy、fmt、2616 文件
行数门禁及 diff 检查通过。测试覆盖全部单位、固定表达式数值结果、非法
表达式、输入上限及缺失/非有限布局上下文。未执行 ignored 环境测试或
在线 Loom 联调，无新交付 EXE、提交、推送或主干合并。

对照主干 declarativeSurfaceStyle.ts 实现无依赖长度解析：数值像素、px/%/
rem/em/vw/vh/cqw/cqh/fr、auto，以及容器 min 和受限百分比/偏移/边界
min/calc。像素与相对单位保留 8192/1000 上限，不执行任意 CSS、脚本或 URL。
上下文包含父轴、视口、容器、字体和布局分配的 fraction，缺失 auto/fr
决策时返回未定尺寸，不随意回退到零。Scene Node 提供长度解析访问方法。

此模块只完成布局尺寸计算基础，不等于 flex/grid 布局、滚动、绘制、DPI
映射或控件输入已实现。宿主能力报告不扩大，原生 renderer 和 UI 入口仍
需继续开发。

## N08 声明式场景渲染前模型

完整串行回归 1709 passed、28 ignored；Clippy、fmt、2614 文件行数门禁及
diff 检查通过。新增节点类型/属性保留、重复 ID/非法字段拒绝以及树深度、
节点数、字节数边界测试。未执行 ignored 环境测试或在线 Loom 联调，无新
交付 EXE、提交、推送或主干合并。

对照主干 surfaceProtocol.ts，新增纯 Rust 场景模型，识别 view/row/column/
stack/scroll/text/image/icon/button/input/textarea/number/slider/switch/select/
progress/divider/spacer 全部 18 种节点类型。保留 props/layout/style/
accessibility/events/children，提供只读访问；未知节点类型不静默降级。

解析前限制 1 MiB，节点总数 2048、深度 32，拒绝重复 ID、控制字符事件
标识、非对象属性以及超限字段。解析失败不交付部分场景。挂载结果通过
MountedSurface::declarative_scene 取得模型，非声明式 runtime 明确拒绝。
这是原生 renderer 的数据入口，不执行 JavaScript、不读取资源、不发送
事件，不宣称布局、绘制、交互和全部节点能力已完成。实际 renderer、资源
解析与 UI 挂载/恢复入口仍须继续实现。

## N08 恢复查询后台任务与应用回收

完整串行回归 1706 passed、28 ignored；Clippy、fmt、2612 文件行数门禁
与最终查询任务定向测试（包含正常候选交付、在途取消、迟到撤权三分支）通过，
diff 检查通过。新增真实 HTTPS 查询任务取消/撤权测试及集合容量、退出
回收测试。未执行 ignored 环境测试或在线 Loom 联调，无新交付 EXE、提交、
推送或主干合并。

新增只读 surface_recovery_runtime::Job，把配置准备、CA/DNS 和设备域候选
查询放在后台。完成 Outcome 绑定 origin/device，持有原挂载 Request Arc；
空结果不能消除原挂载 unknown。查询只提供候选，不自动接受或 remount。
在途取消中止网络，完成后撤权也抑制候选交付，但保留请求上下文。

Sessions::query_surface_recovery 已接入统一等待、poll_surfaces 错误提示、
服务切换取消和退出 drain。查询任务及未取走结果计入现有四项容量，提前
预留完成缓存；无新增轮询机制。恢复 UI 候选确认、跨进程恢复和实际原生
Surface renderer 仍未完成，应用集合 API 不等于用户可操作界面已交付。

## N08 候选恢复接入后台挂载事务

本检查点完整串行回归 1704 passed、28 ignored；Clippy、fmt、2609 文件
行数门禁及 diff 检查通过。未执行 ignored 特殊环境测试或在线 Loom 联调；
未构建交付 EXE，未提交、推送或合并主干。

surface_attach_runtime::Request 现在明确区分 Create 与 Recover。Recover
仅接受设备认证查询构造的候选及宿主能力报告，在既有挂载后台线程执行
remount，不调用 attach 创建接口，失败不降级到重新创建。请求保留候选 Arc，
启动失败时调用方可继续保有请求；完成 Outcome 保留候选和确认绑定/unknown。

恢复复用已有 Sessions::start_attach 入口，因此沿用四项容量、完成句柄、
服务切换取消和退出回收，不引入新线程池或轮询。取消在途恢复仍保留 unknown；
已确认响应后撤权只抑制场景交付，保留绑定证据。本轮真实 HTTPS 测试覆盖
查询后实际后台 remount、在途取消和完成后撤权，并断言不发送创建路径。

恢复查询本身的后台任务、实际 UI 候选确认入口、跨进程恢复和 Surface
renderer 尚待完成。此处接通后台执行机制，不代表已经交付可操作的恢复 UI。

## N08 首次未知创建的设备域恢复协议

本检查点完整串行测试 1703 passed、28 ignored；Clippy、fmt、2609 文件
行数门禁与 diff 检查通过。新增真实 HTTPS 测试验证设备认证只读查询、
相同身份去重、空结果、Art 冲突、歧义拒绝和 remount 的代次/Art/runtime/
快照附件替换拒绝。未执行 ignored 特殊环境测试或在线 Loom 联调；无新
交付 EXE、提交、推送或主干合并。

新增 surface_recovery_candidate，通过设备认证 GET /v1/surfaces/stream 的
after=0、timeoutMs=0 查询当前节点。固定协议和消息数量有界，匹配 Art/节点；
同身份快照重复可折叠，不同实例/附件/代次歧义直接拒绝，不猜测最新候选。
同节点出现其他 Art 也不擅自接管。只返回身份候选，不交付未经验证的场景。

候选不是上次创建回执，空结果不是未创建证据：Loom 的恢复流只返回有快照
的附件。原 uncertain 不应由查询清除，也不得自动重发 POST attach。候选
绑定服务/设备，不伪造 stream 没有提供的生命周期版本。

显式接受候选后可调用 remount_surface_candidate。该操作可能有远端副作用，
不自动执行或重试；响应重新验证当前设备/节点、实例/附件、代次不倒退、
快照身份和 Art/runtime，然后从认证响应取得真实生命周期版本。此协议仍
须后台任务/UI 接线，不代表跨进程恢复、完整恢复交互或实际场景渲染已完成。

## N08 首次挂载后台任务与应用回收

本检查点完整串行回归为 1700 passed、28 ignored；Clippy、fmt、2607 文件
行数门禁和 diff 检查通过。未执行 ignored 特殊环境测试，未构建交付 EXE，
未提交、推送或合并主干。

首次 attach 已移入可取消后台 Job，配置准备和网络请求不在窗口回调执行。
Job 与目录/观看/生命周期任务共用四个后台槽，提供完成句柄；应用 Sessions
把首次挂载纳入既有消息等待、服务切换取消和退出 drain，不增加轮询定时器。
首次挂载任务及未领取结果也计入集合的四项上限，启动前预留结果存储容量。

Outcome 保留固定 origin、设备、Art/节点请求、已确认绑定及 uncertain 状态。
在途取消不声称远端未创建，完成后撤权仅丢弃场景，不丢确认绑定；服务切换
同样清除缓存场景但保留恢复上下文。错误提示和线程回收均不消费这些证据。
首次 POST 不自动重试，不能用生命周期的已知绑定恢复路径冒充首次未知创建
恢复。当前上下文保存在进程内，尚未实现跨进程恢复存储或自动恢复策略。

新增测试覆盖真实 HTTPS 请求发出后的取消、已确认响应后的撤权，以及集合
容量共享、关闭后句柄回收、缓存取消保留确认绑定和 unknown。实际 renderer
能力来源、场景渲染、UI 消费结果和首次未知创建恢复仍未完成，不能宣称 N08
或整体迁移完成。下方章节是历史检查点，以此节为当前进度。

## N08 首次 Surface attach 与显式宿主能力报告

新增有界 Report wire 合同：API 1.0，runtime/nodes/transports/capabilities
列表去重并限额，输入能力必须明确；没有默认开启 JavaScript/WebView 或
全节点能力的预设。类型描述合同不代表实现完成，生产调用方只能报告真实
renderer 支持，不能从 Art 请求能力反向抄写。

认证 attach_surface 向 /v1/surfaces/attach 发送指定 Art、节点、当前授权
设备、能力报告与 persistence；使用既有认证取消交换。响应必须选择当前
设备/节点的附件，实例描述和场景的 Art 均匹配请求，场景 runtime 必须被
报告支持。返回身份校验后的 MountedSurface，不自动上报 active，不执行
JavaScript，也不把场景身份验证当作 scene/resources 验证。

当前为首次挂载 HTTP 协议，不是已接入 Application 的挂载任务或完成的
原生场景渲染器。首次挂载失败可能已有远端副作用，不自动重试或宣称已回滚；
后续仍需首次挂载任务/恢复策略、renderer 能力来源和 UI/观看入口接线。

最终代码完整串行回归 1696 passed、28 ignored；新增能力限制测试及真实
HTTPS attach 的请求字段、Art 替换和不支持 runtime 拒绝测试。Clippy、fmt、
2603 文件行数门禁及 diff 检查通过。未构建交付 EXE、提交、推送或合并
主干，未执行 ignored 特殊环境测试。

## N08/N09 Surface 任务接入应用集合与退出

远程 Sessions 集合已统一持有 Surface Job，其完成句柄通过既有等待集合
进入 Application；poll_surfaces 回收线程并保留 Outcome owner，错误提示
不消费恢复证据。观看、目录、Surface 任务及未取走 Surface 结果合计最多
四项。启动前为任务和完成结果预留容量，避免结果堆积或回收时丢失 owner。

退出继续泵消息直到所有任务回收，已完成结果不阻止退出但仍保留至显式
取走/集合析构。服务切换、重新授权、清除会话和退出都取消 Surface 请求，
缓存成功转换为 Stopped 并释放场景，保留已确认版本/unknown，不能把旧场景
交给新服务上下文。Surface 每项一个完成句柄，总等待上界仍为 63。

测试覆盖结果未取走仍阻止新增观看/目录、取消缓存成功不丢 owner，以及
实际 shutdown 同时回收观看、目录和 Surface 三类任务。首次 attach 能力
报告、场景渲染与 UI 消费 Outcome 的业务流程仍待完成。

最终默认串行回归 1693 passed、28 ignored；Clippy、fmt、2600 文件行数门禁
和 diff 检查通过，未执行 ignored 环境测试。无新交付 EXE、提交、推送或
主干合并。

## N08/N09 Surface 生命周期后台事务

新增 surface_lifecycle_runtime::Job，将配置准备、CA/DNS、transition/recover
放到后台线程。与目录/观看共用四任务槽，完成未回收仍占容量；提供线程
完成句柄、非阻塞 request_stop/try_join，Drop 仅作停止并 join 的防泄漏兜底。

任务完成必返还生命周期 owner：在途取消保留 unknown，启动容量或线程创建
失败也把原 owner 返回调用方。业务 panic 被捕获并保守标为 unknown，不能
丢掉远端结果不确定性后重新构造一个已知 owner。完成后撤权会抑制成功和
场景交付，但保留已确认版本作为恢复证据；不会把确认过的远端事实回滚。
StartError 使用 Box 保存 owner，避免大错误返回污染普通调用栈。

测试覆盖实际 HTTPS 在途取消、未回收容量及启动失败归还 owner、成功完成
后撤权不交付成功但保留版本。Application 集合尚未持有此 Job，首次 attach
能力报告、场景挂载与观看入口仍待完成；不得在 UI 中依赖阻塞 Drop 回收。

最终默认串行回归 1691 passed、28 ignored；Clippy、fmt、2599 文件行数门禁
和 diff 检查通过。未执行 ignored 环境测试，未构建交付 EXE、提交、推送
或合并主干。

## N08/N09 Surface 生命周期确认与 unknown 恢复

新增单附件生命周期 owner 和认证 transition/recover HTTP。只允许主动上报
active/inactive/suspended/disposed，created/mounted 由挂载流程负责。版本
由当前绑定 checked_add 取得，响应须精确匹配设备、节点、实例、附件、状态
和版本；有效确认后才更新本地绑定。

请求进入交换前将状态设为 unknown；网络、认证、响应解析或版本校验失败
均不自动重放，不猜测服务端是否提交。unknown 禁止继续 transition，必须
通过认证 remount 恢复。恢复本身失败也保持 unknown，同 generation 的生命
周期版本不得回退。确认 disposed 后不可再次转 active 或恢复旧附件。

真实 HTTPS 回归模拟服务端已切换但返回错误版本，验证客户端拒绝重试，
认证 remount 后使用恢复的下一版本并正确 disposed。此处是后台同步协议和
生命周期 owner，尚未接入 Application 任务集合或原生场景挂载回调；首次
attach 能力声明、渲染以及观看入口仍待完成。

最终完整串行回归 1688 passed、28 ignored；Clippy、fmt、2597 文件行数门禁
和 diff 检查通过。补测期间曾因旧测试 EXE 占用遇到 LNK1104，旧测试结束后
重新完整构建/回归通过，未删除构建目录或终止无关进程。未构建交付 EXE、
提交、推送或合并主干，未运行 ignored 特殊环境测试。

## N08/N09 已有 Surface 附件认证 remount

新增 Surface 绑定 HTTP 恢复路径，复用固定 origin 的 Device/nonce、取消、
1 MiB 响应限额及请求前后有效期检查。发请求前拒绝异服务/异设备绑定，
实例 ID 按单路径段转义，点段不允许改写路径。响应重新校验设备/节点，
必须保留指定 instance/attachment，实例 generation 不得回退。

返回 MountedSurface 包含已校验绑定和原始场景快照；快照协议、实例、附件
和非零 revision 必须一致，但 scene/resources 仍需专属 renderer 校验，
不能把身份验证等同于场景可执行。此恢复方法仅供后台 owner 调用，不自行
启动线程或上报 active；原生 UI 未挂载之前不能提前声称附件 active。

本轮只接通既有附件恢复。首次 attach 的原生能力报告、生命周期 owner、
场景渲染、目录选择与观看入口仍未完成；不照搬旧 WebView 的 JavaScript
能力声明，也不随机生成附件 ID。

新增两项真实回环 HTTPS 测试，验证认证/路径/响应身份与异服务前置拒绝。
测试中 nonce 头名已按实际 transport 修正为 x-loom-device-nonce，最终默认
串行回归 1685 passed、28 ignored；Clippy、fmt、2594 文件行数门禁和 diff
检查通过。未构建 EXE、提交、推送或合并主干，未执行 ignored 环境测试。

## N08/N09 观看所需 Surface 绑定前置合同

检查主干 liveRelayController 和本地 Loom attach_live_viewer 后确认：观看
必须使用已存在的 Surface instance/attachment，Loom 校验所属设备、实例和
disposed 生命周期。不能在目录菜单中随机生成 ID 来替代 Surface 挂载链路。

新增 surface_live_binding，从有界认证挂载响应中同时按当前 device/node
选择附件，排除旧设备同节点绑定，验证映射键/descriptor/instance 一致，
拒绝 disposed、缺失或歧义绑定。origin/device 来自固定请求上下文，响应
不能自行声明授权来源；保留 generation/lifecycleRevision，递增使用 checked_add。
viewer_target 复核当前服务/设备/节点/代次后产生 selected 身份约束目标，
仅用于显式首次观看，不冒充已有成员续签。

此模块完成挂载响应到 Live 目标的合同，不是 Surface HTTP 挂载、场景渲染
或生命周期 owner 的实现。后续先接通真实 Surface 绑定，再连接目录选择与
观看按钮；UI 业务面板和端到端人工验收仍待完成。

三项绑定测试及默认串行回归通过：1683 passed、28 ignored。全目标 Clippy、
fmt、2592 文件行数门禁及 diff 检查通过，未执行 ignored 环境测试，未构建
EXE、提交、推送或合并主干。

## N09 业务状态与帧共用 UI 通知

控制 owner 将通过完整业务/consumer 校验的新状态发布到 ViewerSession 的
单份最新值槽位，复用已有 change_handle，不创建额外等待句柄或事件队列。
新增 poll_update，在同一 state 锁内确认通知并读取最新帧/业务 Arc，避免
业务发布与 reset 交错丢失通知。没有新控制事件时不重复发布状态。

原生 Host 已通过 poll_update 接收并持有业务状态，读取时校验固定许可；
暂停换代、关闭和撤权清空 Session 缓存，Host 关闭释放自身引用。不为业务
更新重新上传像素；业务专用通知没有新帧时仍可消费。该步骤完成数据交付，
尚不代表已实现可视业务面板、会话选择或观看启动菜单。

新增最新值替换/释放、旧代次拒绝及实际 Win32 无帧通知测试；总等待上界
仍为 63，未增设独立业务通知池。

默认串行回归 1680 passed、28 ignored；随后收紧 Host 读取许可检查，并扩展
真实 HWND/GPU Host 测试验证接收业务/撤权拒读/释放引用，该定向测试通过。
未构建 EXE、提交、推送或合并主干，未运行 ignored 特殊环境测试。

## N09 认证 attach 与控制 owner 接入业务状态

attach 完成身份/成员校验后，从同一认证响应解析完整 Business 快照；缺失
observations/能力报告或无效触发数据不再被忽略。prepare_viewer 将业务状态
连同加入身份和控制游标交给 Prepared，控制 owner 独占持有该状态，因此
现有 Runtime 路径已实际执行观察/审计状态处理，不再只依赖外部 consumer。

控制批次在 session/epoch/序号及 session_state 校验后，先准备业务副本，
再调用外部 consumer，最后提交游标与业务状态。业务失败不调用 consumer，
consumer 失败同样不提交业务；任一失败关闭媒体许可，不能继续使用旧观看。
撤权/关闭仍先关闭媒体，不因增加业务处理延迟撤权。

真实 HTTPS 加入测试已验证非零观察序号从 attach 保留到 Prepared；新增缺失
业务快照拒绝测试，以及观察合法但审计/consumer 失败时游标不推进、业务不
部分提交、媒体关闭的回归。原非状态消息测试的占位 payload 改为合同合法
观察，未放宽新校验。UI 业务状态展示、会话选择和观看启动仍待接通。

最终默认串行回归 1677 passed、28 ignored；全目标 Clippy、fmt、2589 文件
行数门禁及 diff 检查通过。未构建 EXE、提交、推送或合并主干，未运行
ignored 特殊环境测试。

## N09 触发注册快照与跨业务状态事务

新增触发注册合同，验证 binding/condition 的 observation ID 与 revision 一致，
Surface target 标识符、稳定时长、数值比较操作数，以及 selector 的 path/value
形状；操作数最多 4 KiB、深度 16、512 节点，注册快照最多 64 项。观看端只
恢复注册配置，不本地求值、不重放 Surface 动作。

新增 Business 组合能力、观察、审计和注册快照。apply 在完整状态副本上处理
observation/trigger_event，任一子批次失败都不提交；注册使用 Arc 切片共享，
审计事件不会自动替换注册配置。session_state 继续由控制 owner 处理，输入
事件只有 source 执行，其余消息沿用主干 viewer 无副作用策略。此处依赖外层
先验证 envelope 身份/epoch/连续序号，成功后才提交可靠控制游标。

目前完成业务状态的组合与测试，尚未把 attach 原始业务快照传入 Prepared，
也未接入控制 owner/Runtime 与选择观看入口，不能视为完整观看链路已完成。

新增四项业务/注册测试通过，默认串行回归 1675 passed、28 ignored；全目标
Clippy、fmt、2589 文件行数门禁及 diff 检查通过。未构建 EXE、提交、推送
或合并主干，未运行 ignored 环境测试。

## N09 触发审计镜像与迟到结果保护

新增 `live_relay_trigger_audit`，严格解析 trigger_event 的审计字段、正数
计数器、标识符和 512 字节 reason 上限。观看端仅保存审计，绝不再次执行
Surface 动作。按 idempotencyKey 更新，同键最终结果可替换预留状态，迟到
预留不得覆盖最终结果；相同记录不重复分配。最多保存 256 项，新增项超限
按 FIFO 淘汰，修改现有键不改变顺序。

初始 triggerAudits 缺失沿用主干空列表兼容规则，null/类型错误仍拒绝；
快照本身超过 256 项拒绝。同键快照通过同一 upsert 规则归一化，防止重复
快照残留预留状态。增量子批次原子提交，使用 Arc 副本避免复制全部审计文本。
同时新增观察能力报告与汇总：只接受八种已知能力且不允许重复，缺失报告
不等同于空报告。汇总保留 unsupported/error/observing/stable/stale 的主干
优先级，错误原因借用已有数据，不额外复制值。审计四项定向测试通过，
另新增能力报告与汇总优先级测试。

上述模块尚未接入 attach 与完整控制 consumer，trigger registration 和
跨业务批次事务仍待实现；未提前接通不完整观看入口。

本轮新增六项测试，默认串行回归 1671 passed、28 ignored；全目标 Clippy、
fmt、2586 文件行数门禁和 diff 检查通过。未执行 ignored 环境测试，未构建
EXE、提交、推送或合并主干。

## N09 观察业务合同与原子状态存储

对照主干 viewer 与本地 Loom 合同，新增纯 Rust Observation/Locator 类型与
Observations 存储。保留 unknown/stale/error 不得携值、来源/置信度限制、
UIA 必须有 locator、严格字段、字符/路径/64 KiB 值上限和最多 256 个 ID。
attach 快照必须显式包含 observations，拒绝重复 ID，不把缺字段视为空。
增量按观察 ID 独立连续计数，与 envelope sequence 分离；checked_add 拒绝
序号耗尽重放。一批事件先在 Arc 映射副本上校验，失败不留下部分更新，
不逐次复制大 JSON 值，也不通过淘汰 ID 破坏序号历史。

本轮完成观察合同和状态存储，尚未接入 attach/控制业务 consumer。能力汇总、
trigger registration/audit 和完整消费事务仍需继续；不能使用空 consumer
提前接通观看入口。合同核查确认 viewer 的 trigger_event 只更新审计，不应
重复执行由 Loom 发起的 Surface action；迟到 reservation 不得覆盖最终审计。

四项观察定向测试及默认串行回归通过：1665 passed、28 ignored。全目标
Clippy、fmt、2583 文件行数门禁和 diff 检查通过；未构建 EXE、提交、推送
或合并主干，未运行 ignored 特殊环境测试。

## N09 原生目录刷新命令与授权选择联动

授权菜单增加“刷新远程会话目录”，回调只提交一个有界请求，不访问网络。
Application 校验当前服务与授权有效期后申请租约，再启动后台目录任务。
共享 UI 状态只接收固定 origin 的目录元数据与条目数，不接收 token。
切换服务、清除本地会话、重新配对/续签和退出均清空旧目录并取消查询；
正常取消也解除目录 busy，不把静默 Stopped 留成永久加载状态。
消息等待前先处理授权选择请求，再接收目录完成，避免旧选择覆盖新状态。

新增应用测试验证未授权不启动目录、服务切换/取消清空结果、静默停止恢复
状态；扩展菜单门禁测试覆盖新命令和组锁许可。此处只是目录刷新入口，
条目选择窗口、观看启动和完整业务 consumer 仍未完成，未构建新 EXE。

本轮默认串行回归 1661 passed、28 ignored，全目标 Clippy、fmt、2580 文件
行数门禁及 diff 检查通过。未执行 ignored 环境测试，未提交、推送或合并。

## N09 目录任务接入应用等待与退出

远程 Sessions 集合现在统一持有观看任务和至多一个目录 Job，两者合计最多
四项，包含完成未回收项。目录完成句柄已通过集合接入 Application 消息等待；
查询期间最多三个观看任务加一个目录句柄，不增加现有 63 句柄等待预算。
目录结果最多保留一份，刷新时清空旧结果，服务切换可显式取消；消费方必须
核对 origin 并使用新租约与 selected 身份发起加入，目录本身不是授权凭据。

退出先取消目录与观看，再在原有消息泵中一起回收；不会把目录 Job 留给 UI
线程的阻塞 Drop。新增混合容量/取消/关闭测试，并扩展实际 shutdown drain
测试同时回收目录和观看任务。菜单命令、目录选择界面和完整业务 consumer
仍待实现，本轮不声称已经具备可操作入口。

本轮全量串行回归 1660 passed、28 ignored；随后收紧刷新失败清空缓存与
测试插入容量断言，集合三项定向测试重新通过。未执行 ignored 环境测试，
未构建 EXE、提交、推送或合并主干。

## N09 会话目录后台生命周期

新增 `live_relay_catalog_runtime::Job`，将 CA 加载、DNS 和认证目录请求放入
后台线程。目录与观看 Runtime 共用四个任务槽位，完成未回收仍占容量；UI
通过完成句柄等待并用 `try_join` 一次性取走结果，不需要定时轮询网络。
结果固定携带请求 origin，交付前再次检查租约，授权撤销后丢弃迟到成功。
取消复用可中断网络等待的租约机制；Drop 仅作阻塞回收兜底，上层须显式等待。

三项定向测试覆盖真实 HTTPS 返回、在途取消、迟到成功抑制，以及目录与观看
共同占满四槽位后必须回收才能再启动。Application 尚未持有目录 Job，选择
界面和完整业务 consumer 仍未接入；不能把后台 API 完成视为可操作入口完成。
后续接线必须复用远程任务等待预算，不得在现有 63 句柄上界外另加独立池。

本轮定向测试 3 passed；默认串行回归 1659 passed、28 ignored。全目标
Clippy、格式检查、2580 文件行数门禁和 diff 检查通过；未运行 ignored
特殊环境测试，未构建 EXE、提交、推送或合并主干。

## N09 会话目录与选中身份约束

新增认证 catalog API，沿用固定 origin HTTPS/Device nonce 和 1 MiB 限额，
最多 512 项。目录完整验证 Snapshot 的来源、epoch、revision、成员字段和
开放状态，拒绝重复/不安全 ID；成功结果按 session ID 排序，只包含选择所需
身份、revision 和成员标记。本地 Loom 的 list 实现过滤 closed 并清空观察/
触发快照，本接口不把目录当作这些业务数据或加入授权。

新增 prepare_selected_viewer，区别于旧观看 expected 续签：首次显式选择
允许当前设备还不是成员，但重新 discovery 的来源设备、hook 和 epoch
必须与 selected 一致，否则不发送 POST viewers。Runtime Target 已接入
selected；expected 仍保留原续签成员检查，选中身份与续签身份冲突直接拒绝。
attach 返回身份继续由既有合同校验，不因增加目录而放松绑定；返回 revision
不得低于本次 discovery，否则拒绝进入 bootstrap。远端加入若已产生副作用，
本地失败不声称已撤销远端加入。

新增三项纯目录测试和三项真实 HTTPS 测试，覆盖空列表、排序/成员标记、
重复/关闭/缺字段/不安全 ID、协议/条目/字节上限，及首次非成员选择加入和
来源设备/hook/epoch 替换时在加入前拒绝，以及旧 attach revision 不进入
bootstrap。目录后台任务、可视选择界面和
完整业务 consumer 仍未完成；没有新依赖、新 EXE、提交、推送或主干合并。

最终默认串行回归 1656 passed、28 ignored；全目标 Clippy、fmt、2578 文件
行数门禁及 diff 检查通过。未运行 ignored 特殊环境或人工原生验收。

## N09 接入 Application 集合、等待与退出

新增最多四项的远程 Sessions 集合，容量包含停止但未回收任务，关闭集合后
禁止再启动；开始前检查容量并预留存储。Application 持有该集合，在 Live
结算点回收远程终态，将 Ready/帧/完成句柄加入原有消息等待集。普通关闭与
Stopped 不弹错误，设备撤权及窗口故障保留提示；不按 origin 盲目撤销可能
已续签的新授权缓存，当前终态只终止所属租约。

远程 Session 的 Ready 与帧句柄互斥，每项最多两个句柄；16 个本地 Live、
可见性、授权和四个远程会话合计上界 63，并加编译期断言。退出先关闭远程
窗口并请求停止，再与本地会话/授权一起泵消息等待真实完成；停止后远程只
保留完成句柄，不等待无用 Ready，不通过线程 detach 隐藏回收问题。

新增集合容量/终态分类及实际 shutdown drain 测试，验证完成未回收仍占
容量、集合关闭拒绝启动、终态只消费一次以及远程线程和等待句柄被回收。
菜单启动命令、能力/会话选择、具体业务 consumer、远程输入和重连仍未完成；
当前不是完整可操作的远程观看交付。无新依赖、新 EXE、提交、推送或合并。

最终默认串行回归 1650 passed、28 ignored；全目标 Clippy、fmt、2575 文件
行数门禁及 diff 检查通过。未运行 ignored 特殊环境或人工原生验收。

## N09 远程窗口与后台会话 owner

新增远程 Session，接收配置快照、租约、目标和显式业务 consumer，启动
控制/媒体 Runtime；只有收到有效 Ready 后才创建窗口，默认保持隐藏。
show 可在 Ready 前记录显示意图，停止/完成后不能复活。wait_handles 统一
提供 Ready、后台完成和帧变化句柄，停止后不再等待无用 Ready。

poll 在 UI 消息或句柄唤醒后结算：用户关闭仍有效许可的窗口会取消后台；
许可失效关闭窗口则等待后台原始终态，不抢先改写为 Stopped。Ready 校验
通过后、窗口构造前发生撤权时，也不会误报显示构造错误。显式 stop 先撤下
窗口再请求停止，不 join；只有真实线程完成并回收后交付一次 Outcome，
之后句柄集合为空。最终 Drop 仍是阻塞兜底，Application 必须泵消息回收。

新增两项实际 HTTPS/隐藏 HWND 测试，覆盖 Ready 创建窗口、用户关窗、授权
撤销、提前停止、一次终态交付和等待句柄释放。既有真实 WSS 终态测试提升为
经过 Session，验证普通关闭/设备撤权仍保留原因且在两秒内回收。当前尚未
把 owner 放入 Application 集合、等待/退出流程或菜单；不是完整用户入口。
无新依赖、新 EXE、提交、推送或主干合并。

默认串行回归 1647 passed、28 ignored；随后修正测试中恒真的无符号下界
断言，相关两项定向测试重跑通过。全目标 Clippy、fmt、2572 文件行数门禁
及 diff 检查通过；生产代码在完整回归之后未再改动。

## N09 远程观看专用 HWND

新增 RelayWindow，默认创建隐藏窗口并设置 WDA_EXCLUDEFROMCAPTURE 后绑定
固定媒体许可。窗口拥有 Host 和显示资源，暴露帧事件句柄供应用等待，并通过
refresh 消费新帧或关闭事实；没有新增定时器/通知线程。WM_PAINT、缩放、显示
和关闭均有处理，回调不执行网络或 join。显示前先刷新许可，撤权窗口不能
重新显示；关闭是窗口终态，上层仍须停止/回收后台 Runtime。

Host 调用前从 RefCell 取出，Win32/D2D 重入关闭后不会将旧 Host 放回。资源
析构可能重入，因此销毁 HWND 前重新校验 userdata 归属，避免销毁复用句柄。
隐藏/最小化不上传或绘制，消费最新待显示帧和撤权；已有 GPU 资源暂保留，
进一步隐藏资源策略尚未完成，也尚未实现远程输入。

三项隐藏窗口测试覆盖捕获排除、隐藏无 GPU 上传、事件消费、撤权关闭、
WM_CLOSE 不阻塞回收及重入销毁不复活 Host。没有显示窗口供人工验收，尚未
接入 Application 的会话集合/消息等待集和菜单，因此不代表完整用户入口。
无新依赖、新 EXE、提交、推送或主干合并。

最终默认串行回归 1645 passed、28 ignored；全目标 Clippy、fmt、2570 文件
行数门禁及 diff 检查通过，未运行 ignored 特殊环境或人工验收。

## N09 远程像素的原生呈现 Host

新增 Composition relay Surface/Host，复用现有 D3D11、Direct2D 和
DirectComposition 创建窗口私有合成链，不使用 WebView。Host 绑定固定
ViewerSession/Permit，按帧事件领取最新像素；普通控制状态不触发重新上传。
同尺寸帧复用 D2D 位图，尺寸变化才重建；CPU PixelFrame 上传后释放，不另存
完整像素副本。绘制按 viewport fit 缩放，透明清屏保留留边。

刷新及绘制前检查许可，失效时释放 pending 像素和 GPU 呈现链；不会通过
admit 自动取得新代次。Host 只拥有显示资源，上层仍需停止和回收后台 Runtime。
零尺寸 paint 不绘制，当前保留已有显示资源；隐藏/最小化资源策略仍需在
实际观看窗口中完成。借用 HWND 要求 Host 先于窗口销毁。

两项真实隐藏 HWND 测试验证 PBGRA 上传、透明留边、交换链像素读回、同尺寸
位图复用/变尺寸重建、CPU 帧释放、重复 paint 无上传及撤权释放显示资源。
这不是可操作的观看窗口交付；菜单、应用等待集、输入和业务事件仍需接线，
在线 Loom/人工验收和全功能资源测试也未完成。无新依赖、新 EXE、提交或推送。

最终默认串行回归 1642 passed、28 ignored；全目标 Clippy、fmt、2568 文件
行数门禁和 diff 检查通过，未合并主干。

## N09 远程帧与撤权的事件驱动通知

ViewerSession 新增 change_handle 和 poll_frame，为后续桌面等待集提供
手动复位合并事件。成功发布帧、暂停及关闭在既有状态锁内设置事件；单个
consumer 在同一锁内复位并领取最新帧，早期缓存随最新帧领取而丢弃。没有
按帧增长的通知队列、定时器或辅助等待线程，普通控制 revision 更新不唤醒。

暂停、权威关闭及租约撤销即使没有新帧也会唤醒；poll_frame 返回 Suspended/
Closed 时，窗口必须撤下旧显示资源。旧代次提交失败不会产生虚假帧通知。
句柄借用受 Session 生命周期约束；该接口是单 consumer，不是多窗口广播。

新增四项 Windows 测试覆盖突发合并、最新帧领取、暂停/关闭、终态快照和
64 次发布/复位竞态。实际 HTTPS/WSS 像素测试改为等待帧事件，不再每 5 ms
轮询缓存，并验证撤权唤醒。仍未实现远程观看窗口和完整桌面接线，也未完成
人工 UI/在线 Loom/全功能与资源验收。无新依赖、新 EXE、提交、推送或合并。

最终默认串行回归 1640 passed、28 ignored；全目标 Clippy、fmt、2565 文件
行数门禁及 diff 检查通过。未运行 ignored 特殊环境/人工原生验收。

## N09 媒体终态即时唤醒控制请求

媒体 Runtime 增加后台完成回调，正常退出及捕获到的运行 panic 都产生终态。
新增 Completion 在取消租约前保存媒体退出原因，然后关闭许可、唤醒控制
HTTP；控制循环在请求返回后优先读取该原因，避免被 Stopped 或请求超时掩盖。
结果仅保存一次，已由用户/授权缓存撤销的任务继续报告 Stopped。回调不等待
UI 或 join；控制线程随后正常回收媒体网络/codec 线程。

新增真实 HTTPS/WSS 测试，在确认控制 events 请求已经发出后，由媒体端发送
普通关闭或明确 live_media_device_revoked。两种情况都在两秒内完成控制
线程回收，保留 Closed/DeviceRevoked 原因，迟到 Ready 不可使用；控制响应
保持未返回，故不是靠三秒 HTTP deadline 或服务端主动结束控制连接通过测试。

最终默认串行回归 1636 passed、28 ignored；全目标 Clippy、fmt、2563 文件
行数门禁及 diff 检查通过。此次结果不代表在线服务或所有网络环境的时延保证。

仍未接入桌面观看窗口/命令、输入及具体业务消费者，重连和全功能/资源验收
也未完成。没有新增依赖、新 EXE、提交、推送或主干合并。

## N09 控制与授权媒体后台链路

新增 start_viewer/start_viewer_configured，在首批控制事件完整校验及消费后
由控制线程启动媒体 Runtime。live_relay_authorized_media 只允许租约已绑定
的 ViewerSession，并按真实当前时间复核 token 与 Client origin；WSS 请求
沿用已认证 Client 的 TLS 和解析地址，不另选服务或关闭证书验证。Device
nonce 继续由媒体 Endpoint 独立生成，认证请求不经过 UI Ready 队列。

Ready 表示控制许可及媒体任务已启动，不表示首帧已经收到。初始媒体游标为
当前 epoch 的零帧，不冒充 resume 成功。控制退出时在后台停止并回收媒体
网络/codec 线程；租约撤销先禁止像素发布，再取消控制请求，随后回收媒体。
媒体终态在控制循环边界检查，可能等待当前有界 HTTP 轮询结束，尚未实现
媒体终态的即时跨线程唤醒；也尚未接入桌面观看窗口、输入及具体业务 consumer。

真实回环测试串联 discovery/attach/bootstrap、首批事件、Device WSS upgrade、
raw BGRA 帧发布与授权撤销，验证旧许可失效、线程完成和媒体字节预算归零。
另测未绑定媒体、错误 origin 和已撤销租约在媒体连接前拒绝。无在线 Loom
联调或人工桌面验收；没有新增依赖、新 EXE、提交、推送或主干合并。

新增两项测试，最终默认串行回归 1635 passed、28 ignored；全目标 Clippy、
fmt、2561 文件行数门禁和 diff 检查通过。默认回归不包含 ignored 原生手动
或特殊环境检查，不能据此宣布整个迁移完成。

## N09 从配置快照启动控制线程

Runtime 新增 start_configured，接收前台捕获的 Config、既有授权租约及观看
目标；不要求 UI 提前加载证书或解析地址。授权配置中的 Client 准备被提取为
共享的 prepare_client，原授权任务仍在其后按需准备 Store，Live 路径则只
准备网络，不打开身份文件、不创建 profile、不重新配对或签发会话。

租约在 CA/DNS 前和准备完成后检查取消状态、origin 与当前到期时间。原生
DNS 使用同一租约取消状态，地址选择/取消可中止准备；错误地址和过期凭据
在文件加载或 DNS 前拒绝。后台准备错误通过独立 Preparation 退出原因返回。

配置型真实 HTTPS 夹具只监听 IPv4，本机 localhost 实测解析为 ::1 后
127.0.0.1；四次请求的地址回退使加入链耗时约 8 秒。原 5 秒整链等待因此
失败，改用 20 秒完成预算后通过；未放宽独立取消延迟断言，也未宣称已优化
这段回退耗时。后续性能验收仍须检查双栈地址回退和实际网络启动延迟。

新增两项配置型测试覆盖真实 CA/DNS/加入链和错误 origin、过期、预撤销的
准备前拒绝，并验证未创建身份 profile。最终默认串行回归 1633 passed、
28 ignored；全目标 Clippy、fmt、2559 文件行数门禁及 diff 检查通过。
没有新依赖、新 EXE、提交、推送或主干合并。

此轮仍未接入桌面 Application 的实际观看命令、会话列表和媒体启动；既有
手动 UI、在线 Loom、完整生命周期和资源性能验收不由配置测试替代。

## N09 Live 控制后台 Runtime

新增 live_relay_control_runtime：后台线程持有独立授权租约，完成真实
prepare_viewer 请求链，创建/绑定媒体 owner，执行首批控制重放，然后继续
events 轮询。网络与业务 consumer 均不在 UI 线程执行；consumer 必须显式
提供，未默认忽略 observation/trigger。控制终态、错误或 panic 都会撤销租约。

Ready 使用容量 1 的一次性队列，只在首批完整消费成功后发送媒体许可及序列
基线；独立 Win32 事件通知就绪，线程句柄通知退出。try_ready 再次校验撤销与
媒体许可，避免停止后交出旧结果。全局最多四个任务，完成但未回收仍占容量。
无进度空批次增加可取消的 100 ms 等待，防止服务端快速空响应造成忙循环。
UI 正常退出应请求停止、等待完成并 try_join，Drop 只是防孤儿线程的阻塞兜底。

新增四项测试覆盖真实 HTTPS 加入/轮询、就绪事件、缓存撤销/显式停止、迟到
Ready 抑制、consumer 失败/panic、预撤销无请求及完成未回收容量。最终默认
串行回归 1631 passed、28 ignored；全目标 Clippy、fmt、2558 文件行数门禁
和 diff 检查通过。测试中真实握手只连接回环夹具，不代表在线 Loom 联调。

此轮尚未连接 Application 的等待集、会话选择 UI 和媒体 Runtime，也未完成
具体非状态事件消费者、到期定时器及重连策略；不能将后台合同测试视作完整
远程观看验收。没有新增依赖或新 EXE，未提交、推送或合并主干。

## N09 授权缓存与后台请求租约

授权 owner 缓存改为持有 Grant，可为每个串行后台请求链签出独立 Lease。
租约共享不可序列化的 Session，不复制 token；各自拥有 Cancel，避免并行
请求互相覆盖单 socket 中断槽。单 origin 最多 16 份活跃租约，已释放弱引用
在申请时回收。签出前检查 origin/过期，Control 每次请求前后继续检查凭据。

缓存替换、显式 invalidate、成功重新配对（包括 device ID 不变）或关闭时，
Grant 释放会撤销全部旧租约；配对失败保留旧授权。每份租约最多绑定一个
ViewerSession，绑定与撤销使用同一锁，先终态关闭媒体再取消网络。迟到绑定
直接关闭传入媒体，旧任务不能借 activate 复活。单份租约退出不影响兄弟租约。

撤销只发出停止和关闭缓存，不在 UI 线程 join 后台任务；Session 的 token
直到最后一个后台持有者释放才由 Zeroizing 擦除。到期目前依赖请求检查，尚无
独立到期定时器。桌面 Live worker 编排、自动重连/重新加入和授权租约实际
接线仍未完成，因此不能将该机制测试当作完整远程观看或在线撤权验收。

新增四项租约生命周期/竞态测试及一项实际 HTTPS 在途取消测试，并扩展三项
授权 owner 测试验证关闭、配对及跨 origin 撤销。最终默认串行回归 1627 passed、
28 ignored，全目标 Clippy、fmt、2556 文件行数门禁及 diff 检查通过。
未新增依赖，未构建新 EXE，未提交、推送或合并 main。

## N09 控制事件 owner 与媒体许可

新增独占 ControlCursor 的控制 owner，接收 Prepared 后保持媒体暂停。首批
必须从匹配 attach revision、当前成员及 viewer_joined 的 session_state 开始；
完整批次的状态和游标归属校验通过、全部事件交给显式业务 consumer 成功后，
才激活 ViewerSession 并提交游标。后续控制进度只更新 revision，不替换媒体
许可代次或清空帧缓存；旧许可在外部 suspend 后不能被该 owner 重新取得。

成员缺失或 visibility=closed 会在 consumer 前关闭媒体，且同批次稍后的
重新加入不能抹去终态。协议错误、消费者失败、HTTP 错误也关闭本地观看，
失败不推进游标。正常 media_disconnected 或 controller 变化并不代表 Viewer
成员被移除。reason 保持可选开放字符串，不虚构服务端移除原因枚举。

当前 Loom 关闭后 events 可能直接返回 404，因此先关闭本地媒体，再请求
认证 member snapshot；只有确认 closed 才成功报告远端关闭，开放快照或
失败不等同关闭事实。所有 HTTP 错误均终止当前 owner，不自动重连复活。

新增八项纯生命周期及两项真实回环 HTTPS 测试；默认串行回归 1622 passed、
28 ignored，全目标 Clippy、fmt、2553 文件行数门禁和 diff 检查通过。
此阶段仍不包含桌面后台 worker、
授权缓存 lease/撤销联动、observation/trigger 的具体业务 consumer 或可见
会话选择器；consumer 的外部副作用没有通用回滚保证。没有新 EXE、提交、
推送或 main 合并；整体迁移仍在进行。

## N09 Viewer 发现、序列恢复与显式 resume

`live_relay_join.rs` 已将真实认证请求串联为 prepare_viewer：从 discovery 找到
目标会话，验证来源；若当前 device 已是成员，读取 member snapshot 中该 actor
专属 requesterControl，否则首次显式加入使用零基线。恢复要求原 source identity/
epoch、当前成员关系、不回退的 revision，以及匹配的 deviceId/epoch 和有效
controlSequence/inputSequence。checked_add 处理 control 溢出，input 必须低于
既有 MAX_SEQUENCE。缺少 viewerDevices 不再被解析为隐式空成员列表，避免错误
跳过已有成员的序列恢复；重复目标、协议错误和超过 512 项 discovery 会失败。

续签通过 expected identity 区分于首次加入：preview 已移除当前成员时不发
POST viewers；通过预检查后仍发送 requireExistingMembership=true，由服务端
封闭检查与加入之间的竞态。attach 使用恢复出的下一序列，最后取得本次加入
事件之前的游标。Prepared 只包含快照、初始 ControlCursor 和控制/输入序列，
不会应用历史或激活媒体许可。若远端加入成功但后续 bootstrap 失败，方法返回
失败，不声称已撤销远端加入；现有合同没有可据此臆造的 Viewer leave 请求。

`live_relay_resume.rs` 发送实际 resume_request，携带原控制序列、最后帧号、
输入序列和 requesterDeviceId。只有成功响应的身份、epoch、成员匹配后才返回
新控制序列；输入 baseline 不被修改，409 正文不参与游标猜测或自动重试。

新增两项纯协议测试和三项真实 HTTPS 测试，覆盖首次加入/已有成员续签的三或
四请求链、成员移除/错误 actor 在 attach 前失败、resume 成功/409/成员错误，
以及游标字段、成员字段、身份、revision 和序列上下限。最终默认串行回归
1612 passed、28 ignored；全目标 Clippy、fmt、2549 文件行数门禁及 diff
检查通过。没有新增依赖，没有新 EXE，未提交/推送/合并 main。

下一步仍是控制 owner 与桌面媒体许可联动：控制事件应用/游标提交、授权失效
关闭旧观看、取消后丢弃准备结果、后台任务/媒体线程回收，以及连接选择 UI。
本轮 discovery 提供目标 session 查找，不代表用户可见会话列表已完成。受限
daemon-busy 重试和完整跨环境验收也仍未完成。

## N09 已认证 Live HTTP 控制请求

核对主干 native/live/relay 的实际客户端协议后，新增 `live_relay_http.rs`，
实现认证 member snapshot、POST viewers、首次加入事件游标、持续 events
轮询和独立关闭事实确认。请求复用 device_session_client 的固定 origin/地址/
信任根 HTTPS 交换，不复制 TLS 或取消机制；GET/POST 参数在连接前验证。
设备签发仍使用原 256 KiB 响应限额，Live 使用 1 MiB；头部限额、无重定向、
直连策略、绝对期限和事件取消保留。

Control 借用已签发 Session，不接受 UI 任意 token；请求前后检查 origin 和
到期时间，每次请求生成独立 24 字节随机 URL-safe nonce，并发送 Device
Authorization 和 X-Loom-Device-Nonce。取得 Device 401 头部后立即返回
AuthorizationRequired，不读取错误体，避免超大/截断正文掩盖已知认证终态。
这只是分类，尚须上层 owner 撤销 permit、清空帧并停止旧 worker。

session ID 按单独 URL path segment 编码，不能用其中的斜杠改写路由。attach
请求包含 surface/attachment、session_ack、epoch、调用方 sequence；续签策略
显式设置 requireExistingMembership。返回快照同时核对来源 identity、epoch
和当前 device 成员关系。bootstrap 从匹配本次加入 revision 的 viewer_joined
前一位开始，不能直接跳至 next。持续轮询使用 timeoutMs=750 与三秒总期限。

修正 ControlCursor 的既有协议偏差：持续事件流 reset=true 必须拒绝，不能
因序列看似连续而接受；首次 bootstrap 仍允许加入之前的历史已被淘汰，并
要求从本次加入起的后缀完整。HTTP 读取只 prepare 批次，不自动推进游标或
激活媒体许可。terminal 方法使用另一份已认证 member snapshot 验证关闭，
不会把 404 或不属于当前 device 的 closed=true 当作关闭事实。

五项真实回环 HTTPS 测试覆盖 snapshot/attach/bootstrap/events 的方法、路径、
请求体、随机 nonce 和游标不提交；跨 origin/过期/非法输入/预取消不连接；
401 超大错误体；响应限额与重定向；关闭事实的成员与状态码边界。完整默认
串行回归 1607 passed、28 ignored；全目标 Clippy、fmt、2545 文件行数门禁
和 diff 检查通过。未新增依赖、未构建 EXE，未提交/推送/合并 main。

尚未完成整个 Viewer 启动流程：discovery、requesterControl 基线恢复及
controlSequence+1、受限 daemon-busy 重试、resume、后台控制 owner 和桌面
媒体许可联动仍待接线。测试手工传入的 sequence 不代表基线恢复已经实现。
现有主干未找到 Viewer HTTP leave 请求，因此没有臆造 DELETE/leave 路由。

## N09/N12 TLS 握手取消的原生事件修复

本轮在加强诊断后复现了此前三秒退出问题：将测试 peer 读取超时独立设为八秒，
确认 ClientHello 后再取消；第 4 次独立运行捕获到取消副本 socket 的 shutdown
返回 WSAENOTCONN (10057)，而真实读取继续等待约 3.008 秒，最终返回
WSAETIMEDOUT (10060)。旧实现把 NotConnected 归为幂等成功，且 Runtime
丢弃取消结果，因此“停止调用快速返回”不能证明实际读取被唤醒。已明确失效
机制，但未进一步断言 Windows provider 为何偶发报告副本未连接。

Windows `BridgeStream` 现使用 `loom_socket_wait.rs`：WSAEventSelect 将流设为
非阻塞，recv/send 遇 WouldBlock 时等待网络事件和独立 manual-reset 取消事件。
取消先设置永久标志再 SetEvent，不再复制 socket 或依赖 shutdown 唤醒读取。
使用 WSAEnumNetworkEvents 原子读取/复位网络事件；唤醒后重新尝试 I/O，
不把另一方向通知当作操作已成功，也不因 FD_CLOSE 丢弃缓冲数据。绝对截止
时间不因重试刷新，没有新增线程或周期性取消轮询。

socket 字段先析构以解除网络事件关联，再回收事件；迟到 Interrupt 只持有
取消事件 Arc，不延长 socket 生命周期。非 Windows 构建保留旧后端，不声称
本轮验证了其它平台。此变更覆盖复用该流的 HTTPS、TLS/WebSocket 与媒体传输，
不修改 TCP connect/DNS 取消界限，也不增加 TLS 信任例外。

定向流测试 11 项通过，含新增写背压取消/绝对期限/缓冲数据后 EOF，以及强化
已有读取消测试以确认确实进入 WouldBlock 事件等待。原 TLS 握手取消测试连续
50 次独立运行通过（每次两个时序），保持原小于两秒断言；服务端 fixture 先
回收后判断时延，失败不会因断言顺序遗留 peer。完整默认串行回归 1602 passed、
28 ignored；全目标 Clippy、fmt、2543 文件行数门禁与 diff 检查通过。

诊断和重复回归日志保留在 linshi/otter-native-auth-ui-20261019/ 下的
tls-cancel-diagnostic.log 与 tls-cancel-event-regression.log。当前证据支持本次
已复现取消路径修复，不等于所有 Windows provider、休眠/网络切换或 N12 整体
性能验收完成。未新增依赖、未构建 EXE，未提交/推送/合并 main。

实现依据：[WSAEventSelect](https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-wsaeventselect)
的非阻塞和通知重新启用约定，以及
[WSAEnumNetworkEvents](https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-wsaenumnetworkevents)
的原子事件复位约定；不把普通 shutdown 当作通用阻塞读取取消保证。

## N09 原生授权桌面入口与应用消息循环

工作窗口菜单现已提供选择 Loom HTTPS 服务、提交设备配对、等待管理员批准、
获取/续签会话及取消请求并清除本地会话。服务地址使用标准 Win32 单字段对话框，
只接受 origin，不接受账号、路径、查询或片段；选择保存在本次应用内存中，尚未
持久化为连接配置。菜单状态明确区分提交配对与取得授权，不展示 token 或响应正文。

`desktop/device_authorization_ui.rs` 拥有跨窗口共享展示状态和单槽请求，模态
编辑不持有 RefCell 借用，确认后再次检查窗口和选择是否有效。应用循环通过
`application_authorization.rs` 启动 Configured 后台任务；选择/取消先于结果
回收，旧任务完成也不能恢复已经清除的凭据。授权线程句柄加入现有消息等待集，
没有增加 UI 轮询定时器；退出时先关闭入口、清空请求、取消任务，再继续泵消息
直到回收完成。新窗口共享状态绑定统一放入 `application_window_state.rs`，
避免 Application 主模块继续超过 500 有效行。

四项新增测试覆盖隐藏 HWND 菜单路由、地址校验、取消/过期模态草稿、请求限额、
分组锁与退出限制，以及真实应用 owner 的线程回收、共享窗口状态和切换后丢弃
旧结果。集成测试仅使用 linshi 下不存在的独立 profile，不创建身份或发出 HTTP。
全目标 Clippy、fmt、2541 文件行数门禁和 diff 检查通过；没有新增依赖。

本轮首次默认串行回归为 1598 passed、1 failed、28 ignored：既有
`live_relay_viewer_runtime::tests::stop_interrupts_stalled_tls_handshake_and_retires_original_permit`
再次在 after_hello=true 出现约 3.001 秒退出，违反小于两秒的要求。后续包含
窗口状态提取的完整回归为 1599 passed、28 ignored，但这不代表间歇失败修复。
客户端握手与测试 peer 读取均为三秒期限，当前证据不能区分退出触发点；
底层 shutdown 返回值被丢弃，仍须补充取消结果和阻塞读取阶段证据。N12 稳定性
门禁继续保留此问题，不能将单次重跑通过当作全量稳定通过。

尚未把此授权缓存接入远端 Live 控制请求/媒体许可或能力目录连接选择；本地
清除不代表服务端撤销设备批准，也不自动终止既有媒体会话。对话框沿用当前
原生系统控件，尚未完成 Neuro 主题适配、实际键盘/焦点及在线 Loom 手动验收。
未构建新 EXE，未提交/推送/合并 main；N01–N12 整体迁移仍未完成。

## N09 授权后台配置准备接入 Runtime

device_authorization_config.rs 捕获显式 HTTPS origin、原生 profile 目录、设备名和
TLS 环境配置快照。默认身份目录沿用原生配置的 %LOCALAPPDATA%/OtterNative，
OTTER_NATIVE_PROFILE_DIR 可显式覆盖；不自动复用主干身份。TLS_ORIGIN 只限定
私有 CA 的作用域，不能成为隐式默认服务地址。Config 构造仅做配置校验，不
读取身份/CA 文件、不创建目录、不做 DNS；显式 Config::new 不继承环境 CA。

Runtime 新增 Configured Task：在同一个受限后台授权线程加载 CA 信任、执行
可取消 Windows 地址解析、构造固定 origin 的 Client/Store，再进入明确的配对/
批准/续签路径。没有为配置准备额外创建线程，也没有绕过已有任务名额、结果
撤销和完成句柄机制。CA/目录文件系统操作仍不具备强制中断；DNS 取消边界同
解析器说明。配置准备可发生在身份加载前，不声称缺失身份时连 DNS 都不会执行。

实际桌面菜单尚未接线，但生产 Task 已能接收配置快照而非要求 UI 提前构造网络
客户端。连接选择仍需提供明确 origin，不能把能力目录加载等同于用户已授权
远端配对。网络代理策略仍为直连，不继承环境代理。

四项配置测试与一项真实组合测试通过：固定 profile/origin、非法配置、取消和
TLS 优先级，以及 Configured 任务实际读取 linshi 中的临时 CA、解析 localhost、
加载测试身份并完成两次 HTTPS 请求取得 session。组合测试不是在线 Loom 验收，
也不是连接延迟性能门禁。默认 lib 串行回归 1595 passed、28 ignored；全目标
Clippy、fmt、2534 文件有效行数门禁和 diff 检查通过。未新增依赖、未构建 EXE，
未提交/推送/合并 main。

## N09 Windows 原生异步地址解析

loom_address_resolver.rs 使用 GetAddrInfoExW 的 OVERLAPPED/manual-reset event 模式。
输入来自已验证 origin 的 host/port；数值 IPv4/IPv6 直接构造，名称解析返回最多
四个去重地址并限制结果链遍历。新增 windows crate 的 WinSock feature，不新增
锁定包。WSAStartup/WSACleanup 成对，与系统引用计数共存，不依赖 std::net 预热。

device_request_cancel 增加由 Arc<OwnedHandle> 保活的原生唤醒事件，登记/取消
通过同一把锁协调。解析等待完成事件或取消事件，不创建自己的 DNS 等待线程。
输入缓冲、hints、结果指针和 OVERLAPPED 均保存在稳定 Box 中，五秒等待期限或
取消发生后调用 GetAddrInfoExCancel，仍等待用户完成事件再释放结果/结构。

五秒不是异常环境中的硬返回上限：取消后的安全回收可能等待更久，系统 provider
的内部查询也可能继续。若有效完成句柄发生异常等待失败，保留 Operation 内存
和 event，避免释放仍可能被 OS 写入的地址；该极端兜底会泄漏，尚无故障注入
覆盖，不能声称所有失败路径零泄漏。正常路径的结果由 FreeAddrInfoExW 回收。
当前解析器尚待接入后台配置准备与桌面授权入口，不在 UI 回调里调用。

五项测试覆盖实际 Windows localhost、数值地址/非法输入、取消事件登记竞争、
IPv4/IPv6 字节序及 scope、畸形长度与链表遍历限额。尚未用失联 DNS provider
强制验证 pending 取消完成尾延迟。默认 lib 串行回归 1590 passed、28 ignored；
全目标 Clippy、fmt、2532 文件行数门禁与 diff 检查通过。独立只读 FFI 生命周期
核验未发现明确缺陷，但保留上述异常等待/系统 provider 边界。

WinSock feature 调整后重新执行依赖合同、许可证审计和实际 OSV Advisory：五份
锁文件、1783 包、全仓仍报告 15 个漏洞 ID，不是严格发行验收。报告位于 linshi/
otter-native-dns-20261019。未构建手测 EXE、未提交/推送/合并 main。

## N09 按 origin 的授权结果 owner 与旧结果隔离

device_authorization_owner.rs 维护有界 pending Runtime 和内存 Session 缓存。
每个 origin 同时最多一个有效任务，总 pending 最多四个；缓存和有效在途任务
最多涉及八个不同 origin。请求目标从 Task 内 Client 取得，不能由另一份任意
字符串冒充。完成事件不携带 token；凭据取用再次核对 origin 与到期时间。

invalidate 只清除目标 origin 的缓存，并把其旧 pending 永久标记丢弃后取消，
因此即使旧任务已成功完成、尚未 poll，也不能恢复被撤销凭据；同 origin 的新
任务可在剩余名额内启动。显式配对成功（包括 ID 未改变）清除该 origin 的旧
缓存，失败保留旧缓存，与主干失败不撤销原授权的行为一致。close 清空全部缓存、
停止全部任务；仍须 poll 到 finished 后析构，避免在 UI 上触发阻塞兜底。

这里撤销的是本 owner 持有的授权缓存与未交付结果，不声称已经关闭由旧凭据
建立的媒体会话；媒体许可撤销仍需应用 owner 联动。实际桌面配置/命令/等待集
接线未完成。只读入口审计确认现有菜单只有能力目录加载，没有设备授权入口；
还缺明确的 HTTPS origin 来源、后台地址解析和身份目录选择，不能在 UI 回调里
同步补 DNS/CA 文件读取，也不能把 TLS CA scope 变量误当成默认服务地址。

三项真实 HTTPS/身份文件测试通过：旧任务完成后撤销再启动新任务、配对失败
保留原 token/成功且 ID 不变仍撤销、单 origin 撤销不影响其他 origin，以及
关闭/到期后拒绝取用凭据。默认 lib 串行回归 1585 passed、28 ignored；全目标
Clippy、fmt、2530 文件有效行数门禁及 diff 检查通过。未新增依赖、未构建 EXE，
未提交/推送/合并 main。

## N09 设备授权后台任务运行时

device_authorization_runtime.rs 将显式配对、首次批准等待、普通续签作为三个
独立 Task。工作线程拥有 Client/Store/临时身份，续签与批准只 load_existing，
仅显式配对可 load_or_create；缺失身份不创建替代密钥、不发网络请求。

每进程最多四个未回收任务。名额不仅限制运行线程，还覆盖已完成但尚未被 UI
取走的结果/线程句柄，避免堆积含 token 的完成结果。Runtime 提供借用的 Windows
线程完成句柄、request_stop 和非阻塞 try_join；panic 变成不含详情的错误，名额
在回收/Drop 后释放。任务返回时释放当前 interrupt，清理临时身份。

取消后 try_join 丢弃已完成结果，不能将旧 token 发布给 UI；即便网络完成早于
取消，也须遵守该发布检查。已提交的配对文件事务不因取消回滚，调用者后续应
重新加载实际状态。Drop 会取消并阻塞 join 作为防孤儿线程兜底，UI 正常路径
必须等待完成句柄后 try_join；文件系统操作不提供强制中断/硬时限，不能把此
运行时称为任何环境下均即时退出。桌面消息循环接线、旧授权撤销尚未完成。

三项生命周期测试与三项真实文件/TLS/Windows 完成句柄组合测试通过：覆盖
未回收名额、panic/Drop 回收、取消后结果抑制、实际续签、实际配对，以及身份
缺失的续签/批准不创建目录且不连接网络。默认 lib 串行回归 1582 passed、
28 ignored；全目标 Clippy、fmt、2528 文件有效行数门禁及 diff 检查通过。
未新增依赖、未构建手测 EXE、未提交/推送/合并 main。

## N09 批准等待与可唤醒取消

device_approval.rs 提供首次授权的九十秒批准等待，只有 challenge 返回精确
PendingApproval 才间隔 250 ms 再申请。其他 HTTP/TLS/协议错误立即返回，不
重新注册设备或重建身份。普通续签仍调用 issue_once，不进入批准轮询。
Client::issue_until 将外层绝对期限传入两次 POST，每次取剩余总预算与十秒
请求上限的较小值；最后一次请求不能额外超出批准等待总期限。

device_request_cancel.rs 拥有一个串行授权任务的停止标志、当前 socket interrupt
和 Condvar。取消既唤醒间隔等待，也中断已建立连接；安装 interrupt 与取消
共用锁，覆盖握手前登记竞争。请求结束后 clear 释放克隆 socket，不在批准
等待期间保留已完成连接。没有常驻定时线程或周期性检查取消的睡眠循环。
同一个 Cancel 不能同时服务多个并发请求链；任务数量限制与 UI 完成句柄仍需
后续运行时 owner 提供。TCP connect 仍受单候选两秒限额，不是即时取消。

四项真实 HTTPS 测试覆盖待批准到成功、不对致命响应重试、续签仍只请求一次、
总期限中断在途请求及取消在途批准；另有两项取消等待测试。默认 lib 串行回归
1576 passed、28 ignored；全目标 Clippy、fmt、2525 文件有效行数门禁与 diff
检查通过。未构建手测 EXE、未新增依赖、未提交/推送/合并 main。

## N09 显式配对注册与身份关联提交

device_pairing.rs 已接入真实 POST /v1/devices/requests。沿用主干 name/kind/address/
publicKey 请求形状，设备名称允许中文并限制长度/控制字符；不发送私钥或会话
token。响应只选择公钥精确匹配的 pending/devices 项，设备 ID 必须合法；同一
公钥返回不同 ID 时拒绝歧义，相同 ID 重复记录可接受，不猜测应绑定的目标。

register 只返回 endpoint 关联，不修改默认身份；register_and_save 接受显式
Store/Loaded，经验证后调用已有事务保存并报告 ID 是否改变。未匹配、HTTP
失败、预先取消均不保存绑定。保存前仍检查取消，但进入文件提交检查点后完整
执行事务，不在落盘中途取消。网络注册已成功而本地快照过期时返回 Store 错误，
保留调用方 Loaded 和磁盘上的较新关联，不能将这种部分成功报告为配对完成。

配对请求不代表批准，不申请 challenge/token，也不自动创建替代密钥。上层须
显式选择 Store::load_or_create，并在成功后撤销该 origin 的旧授权；授权缓存、
批准轮询与 UI owner 尚未接线。本模块不隐式读取实际用户目录。

三项协议测试与四项真实 HTTPS/文件测试通过：覆盖请求形状、公钥匹配/歧义、
响应失败、pending/已有设备保存、密钥保留、预先取消，以及远端注册成功但本地
快照已失效的部分失败。测试文件仅位于 linshi 独立目录，未触碰用户真实身份。
默认 lib 串行回归 1570 passed、28 ignored；全目标 Clippy、fmt、2522 文件行数
门禁及 diff 检查通过。未新增依赖、未构建手测 EXE、未提交/推送/合并 main。

## N09 真实 HTTPS 设备会话客户端

device_session_client.rs 已将单次 challenge/proof/签发协议接到真实 HTTPS。
固定 origin、显式信任根与最多四个已解析地址；loopback 目标拒绝非回环地址，
TLS 验证原始 hostname。每个 POST 从连接开始共享十秒绝对期限，单个 TCP
候选最多两秒；TCP 建立后、TLS 握手前交付 interrupt，外层负责登记及取消竞争。
TCP connect 本身仍不是可即时取消的系统调用。只允许后台调用，不启动常驻线程。

loom_http_transport.rs 适配固定 ureq 3.4.2 的 unversioned transport API，仅将
已连接并验证的 rustls/BridgeStream 交给 HTTP 解析器。禁用 ureq 全部默认功能、
环境代理、重定向和连接池；不引入 Tokio、额外 TLS 实现或 DNS 工作线程。
单次适配器精确匹配请求 URI，并且只能取出流一次，不自动重新发送签名请求。

响应头上限 16 KiB，正文上限 256 KiB，覆盖 Content-Length 和 chunked；网络
错误不包含 URL/正文/token。客户端持有的响应 Vec 使用 Zeroizing，但库内部
HTTP/TLS 缓冲不在清零保证内。签发后按响应接收时间检查到期与设备一致性。
issue_once 不注册新设备、不轮询批准、不写身份文件。

新增四个锁定包：ureq 3.4.2、ureq-proto 0.6.5、base64 0.23.1、utf8-zero 0.8.1。
依赖合同与许可证审计通过；实际 OSV Advisory 扫描覆盖五份锁文件、1783 包，
保留全仓 15 个漏洞 ID，不代表严格发行安全验收。报告在 linshi 下
otter-native-http-20261019。应用 owner、地址解析/代理策略、批准轮询与 Live
控制流/UI 接线仍待完成；本机 TLS 夹具不等于在线 Loom 跨设备验收。

七项真实 TLS/HTTP 测试通过，覆盖完整两次 POST/服务端验证签名、重定向不跟随、
待批准、Content-Length/chunked 超限、错误信任根/hostname、超长响应头、握手前
取消、等待响应时 interrupt 及请求总期限。默认 lib 串行回归 1563 passed、
28 ignored；全目标 Clippy、fmt、2519 文件行数门禁、diff 检查通过。native
125 个锁定包在本次 OSV 结果中未报告漏洞。未运行 ignored 专项、未构建手测 EXE，
未提交、推送或合并 main。

## N09 单次设备会话协议

新增 device_session_attempt.rs，以消费状态的方式连接 challenge 请求、已有的
Ed25519 proof 与 session 响应校验。只有 challenge 阶段的 403 加精确
device_not_authorized 才表示等待批准；签发阶段失败不会被重新解释为待批准，
续签不会在本模块内轮询、创建身份或重新配对。错误不包含响应正文或 token。

响应最多 256 KiB，签发结果必须匹配设备、包含可用于请求头的有界 token，且
在响应接收时尚未到期。Session 不实现 Debug/Clone/Serialize，拥有的 token
使用 Zeroizing；取用时再核对 HTTPS origin 与有效期，拒绝跨主机、跨端口或
明文目标。调用者的原始响应缓冲不在 token 清零保证内。

这是可测试的协议层，不是 HTTP 客户端。真实传输仍须无重定向、正确验证 TLS、
限制读取分配、绑定响应来源并处理期限/取消；不能把传入字节的测试当作远端
配对成功。HTTP 传输、批准轮询、应用 owner 与 Live 控制流仍待接入。

验证：四项协议测试通过；默认 lib 串行回归 1556 passed、28 ignored。
全目标 Clippy（warnings 为错误）、fmt、2516 文件有效行数门禁及 diff 检查通过。
未新增依赖、未运行 ignored 原生专项、未编译手测 EXE，也未提交/推送/合并。

## N09 设备身份文件事务

新增 device_identity_store.rs 与 device_identity_file.rs。Store 固定显式绝对目录
下的 device-identity.json，load_existing 缺失时报错、不创建目录/身份、不迁移
legacy；load_or_create 仅显式初次授权使用，只有不存在才生成，损坏/权限错误
不重建。legacy 迁移保留 seed、公钥与 device ID，只将文档改为 DPAPI schema 2。

写入只暂存已验证的保护文档，同目录 create_new、sync_all 后 MoveFileExW 提交，
不先删除旧文件、不退化为跨卷 copy/delete。失败只清理本次独占暂存；崩溃可能
留下加密 tmp，读取不会自动提升它。断电持久性仍取决于实际文件系统，不承诺
所有本地/网络文件系统均有相同原子性和落盘保证。

合作的新实现实例用 share_mode(0) 的持久 lock 文件协调，进程退出释放 OS 句柄。
加载快照绑定 Store owner 与原始文档 hash，保存设备绑定前重新核对，拒绝过期
或其他 owner 的快照；失败不改变调用方 Loaded。此机制不是对非合作旧版本/外部
写入的原子 CAS：最终核对到替换之间仍有外部竞争窗口，不能声称排除所有并发覆盖。
拒绝最终目录/文件的 reparse point，不声称验证所有祖先目录 ACL 或所有者。

当前模块不自动定位实际用户目录、不执行网络配对。默认测试只在固定 linshi 根下
的唯一目录中创建测试身份；正式身份文件不受影响。

最终默认 lib 串行回归 1552 passed、28 ignored，全目标 Clippy、格式、diff 与
2514 文件有效行数门禁通过。六项真实文件测试覆盖缺失续签不创建、初次创建稳定、
legacy 只读/显式迁移、设备绑定与快照失效、外部改写/损坏/超长保留、合作 lock、
禁止覆盖的新建失败/目标拒绝删除的替换失败、暂存清理及遗留 tmp 不被自动提升。
这些不是断电或强杀进程实验；本轮未构建 EXE、提交、推送或合并 main。

## N09 设备身份保护与 challenge 签名

新增 device_key_protection.rs、device_identity.rs、device_session_proof.rs。身份
内存入口兼容主干 schema 1 的 Base64 Ed25519 seed 与 schema 2 的 DPAPI1 信封，
验证公私钥一致、保留原 device ID，并显式标记 legacy 需要迁移。只生成 schema 2
保护后的存储文档，不在解析失败时偷偷创建替代身份，也不读取/写入用户身份文件。

DPAPI 使用当前 Windows 用户保护与 UI_FORBIDDEN，不使用 machine scope、不降级
明文。原生输出由 RAII 清理并清零；本模块持有的 seed/解码临时缓冲用 Zeroizing。
调用方拥有的 legacy JSON 和加密库内部临时副本不在该清零保证内，不声称清除整个
进程内所有密钥副本。身份类型不派生 Debug/Clone/Serialize，错误不含私钥内容。

challenge 校验绑定设备 ID，有界 JSON/字段且拒绝换行注入；生成新的 24 字节
URL-safe nonce，按主干 loom.device-session.v1 的逐行格式执行 Ed25519 签名，
输出 deviceId/challengeId/clientNonce/signature 请求体。签名前必须由外层从可信
HTTPS origin 获取 challenge；本模块不发 HTTP、不批准配对、不轮询也不缓存 token。

直接使用现有锁文件中的 ring 0.17.14 和 zeroize 1.9.1，不新增包版本；Windows
启用 Cryptography API。真实身份文件持久化/崩溃恢复、配对与会话签发仍待迁移。

最终默认 lib 串行回归 1546 passed、28 ignored，全目标 Clippy、格式、diff 和
2511 文件有效行数门禁通过。新增五项测试覆盖 RFC 8032 section 7.1 TEST 3 的
公钥/签名、实际 DPAPI 随机化往返与篡改拒绝、错误身份不重建、精确 challenge
签名格式/随机 nonce，以及跨设备/未配对/换行/超长输入拒绝；不接触用户身份文件。
重新执行依赖合同、许可证与实际 OSV Advisory 扫描，五锁 1779 包仍保留 15 个漏洞
ID，native 121 包无报告漏洞。证据位于 linshi/otter-native-device-identity-20261019/。
这不是全仓零漏洞或正式发布验收；未构建 EXE、提交、推送或合并 main。

## N09 后台观看任务与握手取消

新增 live_relay_viewer_runtime.rs：每个固定授权代次由一个后台网络线程持有媒体
Connection 与 codec Worker。每进程最多八个网络任务；连接前验证 Permit，连接
完成后再次复核，主循环消费媒体直到关闭、撤销、取消或错误。退出 guard 按原许可
暂停会话并关闭 socket，网络线程内停止/join codec worker，不让 UI 执行 codec 回收。
请求携带的 session/device 必须与 ViewerSession 一致，防止把同 epoch 的其他会话
媒体误接入当前缓存；origin 信任与真实授权仍由外层控制 owner 绑定。

Connection::connect_cancellable 在 TCP 成功后、TLS/upgrade 之前交付 interrupt，
Runtime 的 stop 标志与句柄登记受同步保护，停止发生在登记前/后均能中断握手。
TCP connect 本身仍依赖单次两秒期限，地址候选之间检查取消；不能声称可即时取消
正在执行的系统 connect。停止先撤销原代次的像素发布，再中断 socket；旧任务退出
不能暂停后来激活的新代次。UI 使用 wait_handle/try_join，Drop 的阻塞 join 仅兜底。

此 Runtime 仍要求调用方完成真实授权/bootstrap 并提供可信地址，不自行激活会话。
应用命令/UI owner、远端 Device 签发、控制流及重连尚未接通，不能把后台任务测试
当作完整应用 LiveRelay 验收。

初次取消测试记录过一次约三秒退出延迟，尚未确定根因。补充了 accept 后即取消与
收到 ClientHello 后取消两种时序及耗时日志，十轮完整 Runtime 回归均通过；这批
观测中的停止调用约 0.06–1.6ms，退出观测约 0.37–6.6ms（含 5ms 轮询粒度）。
这些数字不是最坏延迟保证，也不能抹去首轮异常；后续实际 UI/网络性能门禁保留
冷启动及取消尾延迟检查，不把该异常未经定位地宣称为已修复。

最终默认 lib 串行回归 1541 passed、28 ignored，全目标 Clippy、格式、diff 及
2506 文件有效行数门禁通过。Runtime 四项测试覆盖真实媒体→像素→停止回收、
旧任务不暂停新代次、握手取消两种时序、错误 session/device/Permit 在启动前拒绝，
以及八个网络名额的上限与归还。本轮未构建 EXE、提交、推送或合并 main。

## N09 默认公共根与依赖证据

Trust::load_default 已提供默认构造入口：使用固定 webpki-roots 1.0.9 的编译期
Mozilla 公共根，再按精确 origin 叠加操作员 CA。版本与主干 Cargo.lock 一致，
由 Cargo 离线解析更新 native/Cargo.lock，未复制根证书源码或修改 OS 证书库。
这不是 Windows 平台 verifier，不声称自动继承企业系统证书或平台吊销策略。
应用连接 owner 仍须在后台初始化一次并持有该快照，不能在每个媒体帧中重载。

依赖合同通过，许可证审计覆盖 21 个直接 npm 清单与 713 个 Rust 包；未发现明确
proprietary/unlicensed/unresolved 标记。实际 OSV 2.5.1 Advisory 扫描验证五个锁文件、
1779 个包，保留 15 个漏洞 ID；native 锁文件 121 个包无报告漏洞，新增 webpki-roots
1.0.9 无报告漏洞。全仓结果不是零漏洞，Advisory 成功不代表严格发行门禁通过。
证据保存于 linshi/otter-native-tls-roots-20261019/ 下的 osv-results.json 与
license-audit.json；没有重用旧报告，也没有发布版本。

最终默认 lib 串行回归 1537 passed、28 ignored，全目标 Clippy、格式、diff 与
2504 文件有效行数门禁通过。真实 TLS 回环覆盖按配置加载 PEM 后的本机媒体握手，
相同 host 的其他端口不能复用附加 CA。测试通过注入 reader 提供独立证书，不修改
用户环境或读取实际 CA 文件；生产入口另外检查绝对路径与 regular file。
本轮未生成 EXE、提交、推送或合并 main，完整应用迁移验收仍未完成。

## N09 按 origin 隔离的操作员 CA 快照

新增 `loom_tls_trust.rs`，迁移主干 HOOK_LOOM_TLS_ORIGIN/CA_FILE 配置规则，规范
OTTER 名称优先（包括空值），变量缺项或错误配置不会退回旧变量或静默跳过。
CA 路径必须绝对且实际文件为 regular file；有界读取最多 32769 字节，PEM 只接受
证书块，最多 32768 字节、八张证书，拒绝私钥、垃圾尾部、无效 DER 和空证书集。

Trust 持有默认根与可选操作员根的不可变快照。附加 CA 只作用于精确 HTTPS origin
（规范 host/默认端口），不跨端口/host、不下放给 HTTP，不修改 OS 信任库。CA
指纹可用于后续缓存身份；更换配置必须显式重新创建快照，不在媒体热路径轮询文件。
Target::local_viewer_with_trust 将它接入清单→媒体请求→实际 TLS 的链路。

默认公共根入口已由上述后续阶段补齐；注入空 RootCertStore 不代表加载公共根。
本模块不实现 Windows 平台根或 LAN 自动证书发现。真实设备签发、控制流、重连和
HWND 接线仍未完成。测试不修改用户环境变量、证书库或实际 Loom 配置。

## N09 本机媒体发现与主干授权边界

现有 Config 复用清单候选路径与有界读取，增加 load_media/load_media_with；仅
NotFound 才继续下一个候选，解析或权限错误立即停止。HTTP 媒体发现不依赖
hookBridge 字段，不受 hook bridge 的启停或 WebSocket override 控制。
清单基础 metadata/capabilities/transport 校验共用同一实现，不复制两套规则。

新增 `loom_media_target.rs`：本机目标使用主干 device-000-local 身份，按清单
none/bearer 模式构造请求；Bearer 保持敏感头标记，不公开 token。远端 HTTPS
清单可以解析，但不能通过 local_viewer 复用其 Bearer，必须进入 Device 授权流程。
支持主干原有 localhost 与 IPv4/IPv6 loopback，以及 HTTPS 本机匿名模式；此前
仅允许字面量 IP、所有 HTTPS 均拒绝匿名的限制过严，已修正。transport 对所有
本机 origin 再验证候选地址必须为回环，不能因 localhost 解析结果越界而发送凭据。

环境与清单仍作为本地配置，不声称新增了 ACL/文件所有者认证。证书根由可信配置
owner 提供，清单不能自行提升 CA 信任；真实远端配对/凭据签发仍待迁移。

本轮默认 lib 串行回归 1531 passed、28 ignored，全目标 Clippy、格式、diff 和
2501 文件有效行数门禁通过。真实本机 WebSocket 回环验证清单→Bearer 请求→固定
本机 device ID→实际 H264 子协议选择；该测试不包含 H264 解码。覆盖无 hookBridge、
bridge 关闭/override 隔离、首个坏清单不回退、远端不复用清单凭据，以及 localhost
解析至非回环地址时连接前拒绝。本轮未构建 EXE、提交、推送或合并 main。

## N09 真实媒体 TLS 与 WebSocket 协商

新增 `live_relay_media_endpoint.rs`：把媒体请求固定在显式配置的 Loom origin，
构造 viewer/session/device/恢复游标查询和主干一致的媒体 offer。Device 请求生成
新的 24 字节随机 nonce，凭据只进入敏感 Authorization 头，不进入 URL 或错误。
HTTPS 使用调用方提供的此 origin 信任根执行证书链和 hostname 验证，不接受关闭
验证的自定义 verifier。明文仅允许字面量 loopback IP，且拒绝 Device 凭据。
远端匿名、userinfo、额外路径/query/fragment、无效或超长 token 均拒绝。

新增 `live_relay_media_transport.rs`：最多四个候选地址、每次 TCP 连接两秒；
TLS 与 HTTP upgrade 共用三秒握手预算。实际响应必须选择已支持的单个媒体子协议，
不从 offer 或 payload 猜测。连接服务封装每步 250ms 总 I/O 预算与 Ping/RTT 状态，
提供底层 interrupt handle；失败停止本代次媒体 worker。复用 BridgeStream 的截止
时间和取消机制，不新增线程，也不修改系统信任库。

边界：地址解析与可信发现、系统/操作员 CA 配置加载、真实 Device 凭据申请/续签、
控制流激活、重连与 HWND 接线仍未完成。当前 TLS 回环使用临时自签证书和虚构 token，
证明 TLS/请求头/协商/媒体/取消机制，不代表 Loom 已签发或验证了真实设备授权。

本轮默认 lib 串行回归 1525 passed、28 ignored；全目标 Clippy、格式、diff 和
2498 文件有效行数门禁通过。实际回环覆盖 TLS→upgrade→raw 媒体→worker→像素、
interrupt 后停止回收；另覆盖不可信证书、可信证书但 hostname 不符、缺失/未知
协商值、三秒握手停滞截止时间。连接前 DNS 不在本模块执行；interrupt 句柄仅在
连接成功后提供，连接建立阶段依靠有限 connect/handshake 预算退出。未构建发行
EXE，未提交、推送或合并 main；显式 H264 门禁本轮未重跑。

## N09 主动 Ping 与 RTT 生命周期

新增 `live_relay_media_liveness.rs`，由每个连接独占状态。媒体 service 必须显式
传入该状态，读取前按两秒间隔发起八字节序号 Ping；同一时间最多一个在途探测。
仅精确匹配且未过期的 Pong 更新 RTT，十秒过期清除旧 RTT 并允许重新探测，
不将探测过期误判为 Device 撤销。序号耗尽或时间逆行返回错误，不复用序号。
状态无后台定时线程；由有限读取超时的连接循环驱动，实际发送时间受调用节奏影响。

真实本机 WebSocket 回环增加服务器接收主动 Ping、回送 Pong、客户端记录 RTT
的断言。纯状态测试覆盖单在途、错误/重复/迟到 Pong、精确过期边界与序号耗尽。
这不是实际远端延迟指标，也未完成 TLS、Device 握手、重连和 HWND 接线。

本轮默认 lib 串行回归 1518 passed、28 ignored；全目标 Clippy、格式、diff 和
2494 文件有效行数门禁通过。回环夹具显式将 accepted socket 设为阻塞模式并
保留有限超时，修复 Windows 继承 listener 非阻塞模式导致的握手中断。本轮未
重跑显式 H264 门禁，未构建发行 EXE、提交、推送或合并 main。

## N09 已准入 WebSocket 媒体服务与回环接线

新增 `live_relay_media_socket.rs`，服务已完成可信端点、TLS/凭据和实际子协议
检查的 WebSocket。读前验证消息/帧大小上限，Binary 交给有界 worker，Ping/Pong
维持协议响应，最多消费四项 worker 回复并发送 keyframe_request/video_fallback。
非二进制媒体或读取/提交错误停止本 worker；普通 Close 仅暂停当前代次。

只有连接上下文为 Device 且 Close code 为 Policy、reason 精确等于
live_media_device_revoked 才关闭当前会话代次；旧连接的 revoke 不关闭新代次。
读写 timeout、握手、连接建立、认证和重连策略仍由后续 transport
owner 接线，不能从 Authorization 枚举或 Profile 参数推断已认证。

Frame::from_message 直接接管 WebSocket Vec，payload 为 header 后的切片，不
再次分配或复制整帧；共享预算按原 Vec 实际 capacity（含 header）收费，释放时
归还。此路径的租约在 WebSocket 完成消息读取后生效，不覆盖其临时分配峰值；
必须保留读取前的消息/帧限额。原 PendingFrame 的先头后体有界接收入口保持不变。

LiveRelay 相关测试 43 passed、3 ignored。真实本机 TCP/WebSocket 回环将 raw
NLLV 消息送入 worker 并从会话取出正确像素，普通 Close 后回收线程；另验证
Vec/payload 指针不变与容量归还、精确撤销信号矩阵、无上限配置在读取前拒绝，
旧 worker revoke 不影响新授权代次。回环连接不含真实 Device 凭据或 TLS，不
冒充远端服务联调。未生成发行 EXE、提交、推送或合并 main。

最终默认 lib 全套串行回归 1515 passed、28 ignored；全目标 Clippy、格式、diff
及 2493 文件有效行数门禁通过。本阶段未重新运行显式 H264 codec/worker 门禁，
真实回环 socket 测试使用 raw 媒体，不把它说成网络 H264 播放验收。

## N09 真实后台媒体 worker

新增 `live_relay_media_worker.rs`，在后台线程内创建/销毁 Pipeline 与 COM/MF。
每进程最多八个媒体 worker；输入两项、回复四项，均为有界队列。输入满载会撤销
本代次并返回 Full，不静默丢失参考帧；回复满载结束处理循环，不阻塞等待消费者。
线程退出的作用域 guard 按原 Permit 暂停会话，旧 worker 的停止/Drop 不会误停
后来激活的新代次。

停止先置队列外标志并撤销本代次，再尝试发送 Wake；不会排在媒体帧后等待执行。
空闲 recv 使用 100ms 超时兜底观察其他 owner 发出的会话取消，不忙循环；系统
codec 调用本身仍不可强制中断。提供借用的 Windows 线程完成句柄与 try_join，
UI 应先等待完成再回收。Drop 会停止并 join 作为防 detach 兜底，可能阻塞，不能
在 UI 回调中用 Drop 代替正常异步关闭。

显式实际 worker 门禁 1 passed：真实后台 H264 解码发布正确像素，会话 idle
关闭后无需新帧即可退出并 join，decoder 名额重新可用。默认测试覆盖 raw 发布、
空闲关闭、旧 stop 不影响新代次及输入/回复满载。会话快照为合成测试输入，不是
真实认证服务证明。

最终默认 lib 全套串行回归 1512 passed、28 ignored；全目标 Clippy、格式、diff
和 2491 文件有效行数门禁通过。ignored 只显式运行上述具备夹具的真实 worker
门禁，不把其余未运行项计作通过。

worker 目前接收已构造的有界 Frame，尚无实际 socket transport、HTTP/Device
认证或 HWND 消费者。100ms 兜底不是 UI 唤醒方案，也不等于长时性能验收。未生成
发行 EXE、提交、推送或合并 main。

## N09 媒体流水线与会话发布 gate 接线

新增 `live_relay_media_pipeline.rs`，在同一后台 codec 线程持有 MTA apartment、
固定 Permit 和 Receiver，将 H264 结果或 JPEG/raw 静态解码结果直接提交给
ViewerSession。构造前和工作边界复核固定许可，不重新取得许可绕过旧代次失效；
解码期间的关闭/撤权由 commit 同锁拒绝。任何处理错误使流水线永久 retired。

网络循环须在取消唤醒/读取边界调用 check；即使没有下一帧，也能在该调用时
释放失效 decoder。返回的关键帧请求/降级仍只是意图，实际发送前需再次检查。
字段析构顺序确保 decoder 先于外层 COM apartment 释放，Rc marker 禁止跨线程
搬运。该模块尚不创建线程、socket 或 HWND；取消不能强行中断同步系统 codec。

LiveRelay 组合默认测试 36 passed、2 ignored；显式系统流水线门禁 1 passed。
真实 H264 经固定许可、解码、会话提交后可取出正确红色像素；暂停后 check 无需
新帧即可归还 decoder 名额，新代次的新流水线可解码，关闭后 check 同样释放。
默认测试覆盖 raw 像素发布、旧流水线永久失效及外来 Permit 在创建前被拒绝。
全目标 Clippy、格式、diff 及 2489 文件有效行数门禁通过；未重跑默认 lib 全套。

测试中的会话快照为本地合成输入，不是远端认证成功证据。真实 HTTP/Device、
socket 循环及 UI 撤下仍待接入；未生成发行 EXE、提交、推送或合并 main。

## N09 会话媒体发布 gate 与授权代次

新增 `live_relay_viewer_session.rs`，将本地授权状态、许可代次及私有 ViewerPixels
放在固定 state→frames 锁序下。新会话默认暂停；可信控制 owner 完成真实认证、
bootstrap 重放和业务校验后，才能用成员快照激活。控制 owner 为新 worker 取得
Permit，worker 保留该代次，不得失效后自行重新申请。提交时同锁检查会话 owner、
代次、epoch 与终态，并在锁内完成缓存提交；不在该锁内解码、I/O 或等待线程。

暂停/重新激活会取消旧代次、清空缓存并替换许可。每代有独立 AtomicBool 取消
信号，可交给 codec/worker；该信号不替代同锁授权检查。低 revision 的迟到快照
被拒绝且不破坏新代次；有效版本的来源/成员复核失败使本 owner 终止，不可再用
旧快照自动重新加入。关闭不可逆，迟到 open 不重启 gate；即使锁 poison，close
仍尝试清空缓存。已移交 UI 的像素不受缓存释放控制，仍需 UI owner 撤下。

新增 4 项本地回归，覆盖初始暂停、跨 owner/旧代次拒绝、取消信号、失效成员
清缓存/不可恢复、迟到 open、旧 revision 保留新代次，以及 32 轮 close/commit
竞争后无缓存像素残留。最终默认 lib 全套 1506 passed、26 ignored；全目标
Clippy、格式、diff 和 2487 文件有效行数门禁通过。未运行真实授权服务或系统
decoder 门禁，不将本地竞争测试冒充端到端撤权验证。

该 gate 目前是本地生命周期边界，未执行 Device 认证、HTTP、worker 启停或 HWND
撤下。后续可信控制/媒体 owner 必须实际使用 Permit 与取消信号，不能从可解析的
JSON 或包内 epoch 自行激活。未生成发行 EXE、提交、推送或合并 main。

## N09 控制批次提交与权威关闭事实

新增 `live_relay_control_events.rs`，bootstrap 与常规轮询共用 envelope 类型。每个
ControlCursor 固定会话身份；prepare 有界解析整个响应并检查每项身份、连续序列
及最终 next，不改变 cursor。业务层完成所有 payload 校验/状态应用后再同锁 commit；
丢弃失败批次可重试，错误的后半批不能使前半批的位置被提前提交。批次还绑定创建
它的 owner，不能跨会话 cursor 使用或覆盖已经推进的位置。未知事件保留原始
kind/payload 给对应业务 owner，不在 envelope 层静默吞掉。

新增 `live_relay_viewer_terminal.rs`，对照主干 member snapshot 的终止判定：
要求精确协议/会话、非陈旧 epoch、当前设备仍在可验证成员列表中及明确布尔
closed。成功响应可证明 open/closed；缺成员、错身份、旧 epoch 或普通 404 正文
只产生校验错误，不伪装成权威关闭；调用方仍须保证迟到的 open 响应不能恢复
已经关闭的本地 owner。

新增 2 项两阶段 cursor/终止事实回归，LiveRelay 组合 30 passed、1 ignored。
全目标 Clippy、格式、diff 和 2485 文件有效行数门禁通过。未重跑默认 lib 全套
或系统 codec 门禁；这些是本地合同测试，不冒充真实认证/撤权联调。

这两部分仍是控制数据合同，不执行 HTTP/Device 认证、不应用业务事件，也不
直接停止 worker。后续会话 owner 必须把授权、事件应用、cursor 提交及媒体发布
放在一致的生命周期和锁顺序下。未连接外部服务，未生成发行 EXE、提交或推送。

## N09 viewer 身份与加入历史校验

对照 main 的 LiveViewerIdentity、validate_live_session_snapshot 和
live_viewer_join_cursor，新增 `live_relay_viewer_identity.rs` 与
`live_relay_viewer_bootstrap.rs`。快照要求精确的协议/会话身份、显式开放状态、
正数 epoch/revision、有界合法来源标识。attachment 前后固定来源设备、来源 hook
和 epoch；自动续期必须仍在 viewerDevices 中，不把被移除成员重新自动加入。

bootstrap 必须找到与 attachment revision、当前设备、来源会话及 epoch 匹配的
viewer_joined。返回该事件前一位，而不是直接采用 response.next；从加入事件
开始逐项验证连续序列及最终 next，防止跳过加入后的控制事件。历史 reset 只有
在本次加入及后续完整存在时才可接受，缺加入/断档/身份变化一律拒绝。

HTTP JSON 上限沿用 main 的 1 MiB，结构化解析拒绝重复的已知身份字段、错误
类型及缺少必需字段。事件 payload 以 RawValue 保留到需要匹配时解析，不复制
整个 Value 树。错误仅返回固定枚举，不把响应或凭据写入诊断。

本阶段新增 3 项快照/续期/bootstrap 回归；LiveRelay 组合 28 passed、1 ignored。
全目标 Clippy、格式、diff 和 2483 文件有效行数门禁通过。未重跑默认 lib 全套
或显式系统 decoder 门禁；本次修改不触及 codec，检查结果不冒充认证服务联调。

这是已认证 HTTP 响应的内容校验，不是 Device 认证本身，也不授予媒体/UI 权限。
真实 HTTP transport、凭据获取、控制事件应用、授权同锁提交及 socket 接线仍待
迁移。未连接外部服务，未生成发行 EXE、提交或推送。

## N09 已解码像素与 viewer 缓存统一

新增 `live_relay_pixels::PixelFrame`，将 raw sRGB 描述符与 Pixels 租约一起移交，
验证尺寸/字节长度，不复制像素。H264 Receiver 直接输出该类型；静态媒体入口
`desktop/live_relay_image::decode_pixels` 将 WIC/JPEG 或 raw 解码结果转为同一类型，
保留采集身份，不再让已解码视频重新变成网络 payload。

原 ViewerFrames 缓存抽取为同一 FrameStore 实现，保留原接收帧类型别名，新增
ViewerPixels 像素类型别名，共享三帧淘汰、epoch/frame cursor、warmup、同锁关闭
及移交规则。未增加第二套并行缓存状态机。缓存依然不授予权限：新 epoch 的准入
和提交前授权复核属于会话 owner，不能将缓存中的 cursor 当作授权证明。

LiveRelay 相关测试 25 passed、1 ignored；显式系统 H264 组合门禁 1 passed，
验证真实解码结果存入/取出缓存后 Pixels::same_storage 成立。默认测试另用真实
WIC JPEG 和 raw 走相同缓存路径，验证存储不变与描述符保留；覆盖错误类型/颜色/
长度拒绝、淘汰/关闭释放、已移交输出在关闭后存活、迟到提交释放。原缓存全部
状态与并发关闭回归继续通过。

最终默认 lib 全套串行回归 1497 passed、26 ignored；全目标 Clippy、格式、diff
和 2480 文件有效行数门禁通过。显式系统门禁只运行上述已有夹具的组合测试。

这是解码→有界缓存接口连通，不是实际 HWND 呈现、网络接收或授权完成。后续
会话 owner 仍须消费此接口并实现授权/关闭同锁提交。未生成发行 EXE、提交或推送。

## N09 视频接收 owner 与实际 codec 组合

新增 `live_relay_video_receiver.rs`，将已有 Chain 与系统 MF decoder 组合为每媒体
连接的线程绑定 owner。接收前检查终态、stop 和调用方授权 epoch；不从媒体包
授予或升级权限。断链 delta 请求关键帧并释放 codec，IDR 重建；JPEG/raw 保留原
Frame 返回静态媒体路径并释放视频资源。codec/参考链/输出身份错误只返回一次
fallback 意图，此连接之后持续忽略 H264，图片不会清除 sticky fallback。

输出时间戳匹配队首描述符，保留 epoch、capture/encode 时间和帧号，只把呈现
表示改为 raw BGRA 并交付已有 Pixels 租约，不重新复制整帧。关闭不可逆，销毁
codec 并清空状态。返回值只是控制意图/媒体，不执行网络发送或 UI 提交；真实会话
owner 必须在提交前同锁复核权限/epoch/关闭状态，不能把此模块当作认证边界。

LiveRelay 默认相关测试 22 passed、1 ignored；显式实际系统组合门禁 1 passed。
实际证明 IDR 输出身份保留、codec 名额占用、跳帧请求后的名额归还、IDR 恢复、
图片切换释放、非法 AU 一次性降级、图片后视频仍忽略、新连接独立恢复及关闭释放。
默认测试覆盖授权 epoch 不符拒绝（不进入 codec）、请求去重、终态和图片原帧移交。

最终默认 lib 全套串行回归 1494 passed、26 ignored；全目标 Clippy、格式、diff
及 2478 文件有效行数门禁通过。ignored 不批量运行，只显式执行上述具备夹具的
系统组合门禁；不会把纯状态测试冒充网络授权或 UI 呈现验收。

该阶段尚未连接实际 socket、授权存储或 viewer。未生成发行 EXE、提交、推送或
合并 main；网络控制发送及同锁授权复核仍是后续接线工作，不能由本组合测试替代。

## N09 连续 delta 帧与 IDR 重建运行门禁

新增 `live_mf_sequence_tests.rs`，使用明确指定的本地合成输入，不连接网络或捕获
用户桌面。fixture 文件有 4 MiB 读取上限；AUD 切分仅供测试，逐帧确认 24 帧中
只有第 0、12 帧为 IDR，其余均为 delta。每帧亮度递增，避免恒定画面掩盖陈旧内容。

实际系统门禁 1 passed：64×64、680×430 各执行完整 24 帧和从第 12 帧 IDR
重建后的 12 帧，共验证 72 个输出。每次提交立即产生对应时间戳/尺寸的像素，
没有积压输入；逐像素检查三个灰阶通道及不透明 alpha，最大误差均为 3。
验证中途取消销毁 codec、清除 stop 后失效 owner 仍不可复用、旧 owner 未 Drop
时可创建新 decoder，以及初始 delta 拒绝。此次未调整生产解码算法。

输入位于 `linshi/otter-native-h264-gray-sequence-20261019`，设置
`OTTER_NATIVE_H264_SEQUENCES` 为该目录，显式运行
`actual_continuous_delta_and_idr_restart_preserve_pixels_and_timeline -- --ignored --nocapture --test-threads=1`。
该门禁需要 `gray64.h264`、`gray680.h264`；对应 FFmpeg 生成参数如下，第二份将
尺寸替换为 `680x430`、文件名替换为 `gray680.h264`，生成媒体不提交仓库：

```text
ffmpeg -hide_banner -loglevel error -n -f lavfi -i "nullsrc=s=64x64:r=30,geq=lum='32+4*N':cb=128:cr=128" -frames:v 24 -c:v libx264 -profile:v baseline -pix_fmt yuv420p -preset ultrafast -tune zerolatency -qp 18 -color_primaries bt709 -colorspace bt709 -color_trc iec61966-2-1 -color_range tv -x264-params keyint=12:min-keyint=12:scenecut=0:ref=1:bframes=0:aud=1:repeat-headers=1 -f h264 gray64.h264
```

该结果是同步 decoder 的连续帧与内容身份验证，不是实际网络抖动、长期内存、
多设备或 viewer 呈现验证。尚须连接 socket/授权/参考链 owner 及 UI；未生成发行
EXE、提交、推送或合并 main。

最终 MF 默认相关测试 5 passed、5 ignored；本阶段仅显式运行上述连续序列门禁，
未把其他 ignored 项算作通过。全目标 Clippy、格式、diff 及 2476 文件有效行数
门禁通过。未重跑默认 lib 全套；上一阶段全套结果为历史证据，不冒充本轮重跑。

## N09 系统 H264 实际像素解码

新增 `live_mf_decode.rs`、`live_mf_samples.rs`、`live_mf_pixels.rs`：先执行既有
AU/SPS/PPS/VUI 检查再提交 MF sample，时间戳必须严格递增，在途最多四项；每次
输出读取最多三步，stream-change 重新校验和协商格式。输出时间戳须对应队首，
取消、码流/预算/系统错误立即销毁 codec 并归还名额，失效链不可复用。

sample 与输出 event 仅在 codec 调用内持有，返回值不带 COM 对象或 desktop
私有 Image 类型，只带尺寸、时间戳和共享预算 Pixels。NV12 检查存储尺寸、stride、
长度和色度起点后直接转换成不透明 PBGRA，避免原路径的额外可见 NV12 整帧副本；
2D surface 的必要临时副本及最终像素均先预约像素预算。系统 codec 内部分配仍
不受该租约完全控制，取消也不声称能中断已进入的同步系统调用。

真实测试暴露了 32×32 可协商但持续不出帧的问题。联网核实
[Microsoft H264 decoder 约束](https://learn.microsoft.com/en-us/windows/win32/medfound/h-264-video-decoder)
后，配置入口新增最小 48×48 限制；wire 小图合同未改变，后续连接 owner 必须将
codec 拒绝转为 JPEG/raw fallback。上一阶段的 32×32 结果仅为历史类型协商证据，
不是解码支持证明。没有通过增大队列或忽略空输出掩盖此问题。

显式实际解码门禁 2 passed：本地 FFmpeg 仅生成自有纯色测试输入，产品依赖不
增加 FFmpeg。64×64 与 680×430 IDR 都产生对应尺寸/时间戳的红色像素；重复
时间戳被拒绝，codec 销毁后像素仍有效，新 decoder 可立即创建。另验证非法 AU、
预先取消及失效 owner 不复用。允许系统在四帧预算内延迟输出，不使用 drain 假装
实时结果。尚未验证连续 delta 帧、真实网络/viewer 或长时间内存稳定性。

测试输入位于 `linshi/otter-native-h264-20261019-aud`；设置
`OTTER_NATIVE_H264_FIXTURES` 为该绝对目录，显式运行
`live_mf::decode::tests::actual_ -- --ignored --nocapture --test-threads=1`。
目录需包含 `red64.h264`、`red680.h264`，可用本机 FFmpeg 分别按以下参数生成：

```text
ffmpeg -hide_banner -loglevel error -n -f lavfi -i color=c=red:s=64x64:r=30 -frames:v 1 -c:v libx264 -profile:v baseline -pix_fmt yuv420p -preset ultrafast -tune zerolatency -color_primaries bt709 -colorspace bt709 -color_trc iec61966-2-1 -color_range tv -x264-params keyint=30:ref=1:bframes=0:aud=1:repeat-headers=1 -f h264 red64.h264
```

另一份将尺寸改为 `680x430`，输出改为 `red680.h264`；输出须放 linshi，不提交
生成的媒体。失败的 32×32 夹具保留作调查证据。未生成发行 EXE、提交或推送。

最终默认 lib 串行全套 1492 passed、24 ignored；本阶段单独运行上述实际解码
两项及更新后的格式协商一项，均通过。全目标 Clippy（warnings 视为错误）、格式、
diff 检查和 2475 文件有效行数门禁通过。没有批量运行需要独立夹具的 ignored 项。

## N09 H264 输入与 NV12 输出格式协商

`live_mf_format.rs` 消费 decoder owner 完成低延迟设置、H264 输入类型和 NV12 输出
类型协商；只有尺寸、颜色与输出分配上限通过后才启动流。失败销毁整个 owner，
归还进程名额，不返回半初始化的 decoder。输出类型枚举最多 32 项，输出 buffer
声明最多 32 MiB；显式冲突的颜色属性和读取错误不会被当成缺省值忽略。

`live_mf_layout.rs` 区分存储尺寸与可见尺寸，拒绝非零 aperture 偏移及尺寸不符。
只允许右/下宏块对齐 padding，而且必须有准确的 aperture；存储像素仍受 8,294,400
上限约束。现有 windows crate 仅增加 Variant feature，不升级依赖或引入第三方 codec。

默认 MF 相关测试 4 passed、2 ignored；显式格式运行门禁 1 passed。实际协商结果为
32×32→32×32、680×430→680×430（可见→存储），并查询当前输出类型核对缓存。
因此该证据只证明实际 NV12 协商及非法尺寸失败后的名额释放，不声称系统产生过
padding surface；padding 的准入/拒绝由纯规则测试验证。

最终源码默认 lib 串行全套 1491 passed、22 ignored；全目标 Clippy（warnings
视为错误）、格式检查、diff 检查及 2472 文件有效行数门禁通过。22 项显式运行门禁
未作为一组执行，本阶段仅单独运行上述格式协商门禁；不把 ignored 计作通过。

尚未提交压缩 AU、处理 ProcessOutput/stream-change 或生成 H264 像素，也未连接
网络会话和 viewer。此次不是可播放视频交付；未生成发行 EXE、提交或推送。

## N09 系统 Media Foundation decoder 所有权

新增 Windows-only `live_mf.rs`，显式管理 COM MTA、MFStartup/MFShutdown、系统
同步 H264→NV12 decoder 枚举与激活。最多接受 64 项枚举结果，所有结果与枚举数组
按 Windows 所有权规则释放；激活失败尝试 ShutdownObject。字段顺序保证 transform
先 flush/end-stream/ShutdownObject 并释放，再关闭 MF/COM，最后归还进程名额。
owner 不暴露可克隆的裸 transform 接口，Rc marker 禁止跨线程搬运生命周期。

保留主干 C1 的每进程一个 decoder 名额；重入明确拒绝，正常退出及 unwind 归还。
仅为现有固定版本 windows crate 启用 MediaFoundation feature，未升级版本或加入
第三方 codec。此阶段只激活系统 MFT，尚未协商媒体格式、提交 AU 或解码像素，不能
当作 H264 播放完成，也不是 GPU 硬件解码承诺。

本阶段默认 owner 测试 1 passed、1 ignored；显式系统门禁 1 passed，实际完成
3 次激活/释放并拒绝并发第二个 owner，每次确认 1 input/1 output stream。最终
全目标 Clippy、格式、diff 和 2470 文件有效行数门禁通过。未重跑默认全套，未生成
发行 EXE，未提交或推送；没有把真实激活冒充真实解码。

## N09 H264 codec 前置码流边界

新增 `live_h264.rs`、`live_h264_bits.rs`、`live_h264_sps.rs`，对照主干 Annex-B 与
decode_bounds 的 baseline 合同迁移 AU/SPS/PPS/VUI 校验，不增加第三方 codec。
AU 最多 1 MiB、256 个 NAL；逐单元借用扫描，不建立 NAL 列表或复制整帧；参数集
读取使用固定 1024 字节缓冲，限制 Exp-Golomb 位数和 emulation-prevention 序列。

SPS 要求 baseline 8-bit 4:2:0、渐进扫描、最多四个参考帧，检查实际 coded surface
及裁剪后尺寸与 wire 一致；PPS 拒绝 CABAC/切片组和超限引用。VUI 保留主干颜色、
时序、HRD 和 decoded-picture-buffer 约束，网络 transfer=0 不作编码端规范化。
IDR 必须先有 SPS/PPS；拒绝混合 IDR/delta、迟到参数集及关键帧标记不一致。

这仍是系统 codec 之前的有界检查，不是完整 H264 语法验证或像素解码；Media
Foundation owner 尚未迁移，模块尚未接入真实视频接收任务。

本阶段 4 项前置检查测试通过；最终源码默认 lib 串行全套 1487 passed、20 ignored。
全目标 Clippy、格式、diff 和 2469 文件有效行数门禁通过。测试是参数集/状态合同
证据，不是 H264 像素验收；未生成发行 EXE，未提交或推送。

## N09 H264 参考链与降级状态

新增 `live_relay_video_chain.rs`，对照主干 socket-owned video viewer 迁移连续性
规则：epoch、frame_id、尺寸连续才能复用参考链；断链先清空在途描述符，非关键帧
只请求关键帧而不进入 codec。请求按 epoch 去重，避免旧 epoch 的等待状态吞掉新
epoch 的必要请求。首个关键帧要求重建 codec，最多保留 4 个描述符，frame_id 必须
可映射到 i64 时间轴，输出时间戳必须对应队首描述符。

codec/预算/输出身份错误进入 sticky fallback，继续忽略该连接在途视频；JPEG/raw
仍可走图片路径，但不恢复视频尝试。fail 只首次返回需要发送降级通知，内部错误
已终止准入也不吞掉这次通知。实际控制发送和 codec 销毁仍由连接 owner 执行。

本阶段只迁移状态合同，没有创建 Media Foundation decoder、验证 SPS/AU 或产生
H264 解码像素。调用前仍必须验证授权、终态及 epoch；不得把 Decode action 当作
已完成媒体校验或解码的证据。

本阶段 LiveRelay 组合 20 passed，新增 5 项参考链回归覆盖断帧/epoch/尺寸变化、
请求去重、队列上限、输出时间戳错配及持续降级。全目标 Clippy、格式、diff 和
2465 文件有效行数门禁通过；未重跑默认全套，未生成发行 EXE，未提交或推送。

## N09 raw BGRA 与 JPEG 媒体解码

新增 `desktop/live_relay_image.rs`，把带租约接收帧转换为现有 Image/Pixels 所有者。
raw BGRA 在像素预算预留后转换为 PBGRA，透明像素 RGB 清零；JPEG 复用 WIC 内存
解码，但增加独立 relay 入口，要求 SOI/EOI、JPEG 容器及实际尺寸与 NLLV 元数据
一致，尺寸核对先于像素输出分配。原有文件/通用编码图片入口保持原行为。

静态图像入口明确拒绝 H264 和非 sRGB，不将有状态视频或 HDR 数据误解为普通
图片；H264 解码及 HDR 对齐仍待迁移。此模块供后台 COM owner 调用，尚未接入
网络任务或原生呈现；输出像素预算不等于已测量 WIC 内部瞬时内存，也不保证系统
codec 调用可以即时中断。

本阶段 LiveRelay 组合 15 passed，使用内存 WIC JPEG 编码/解码验证真实像素、
尺寸不符、损坏和截断，并覆盖 raw alpha 预乘与 H264 明确拒绝。默认 lib 串行
全套重新通过：1478 passed、20 ignored。全目标 Clippy、格式、diff 和 2463 文件
有效行数门禁通过；未生成发行 EXE，未提交或推送。

## N09 Viewer 帧序与三帧缓冲

新增 `live_relay_viewer_frames.rs`，接收上一阶段带预算租约的完整 frame。单一锁
同时管理 epoch/frame cursor、最多三帧缓冲和 closed 状态，关闭清空缓冲且禁止
迟到提交重新填充。过期/重复帧拒绝；显式 warmup 可忽略，但不会推进游标或保留
预算。新 epoch 允许帧号从 1 开始；查询/取出按 epoch+frame_id 完整身份匹配，
不让不同 epoch 的同号帧混淆。

容量满时释放最老帧并累计覆盖次数；取出移交唯一 owner，同时丢弃此前缓存，
输出租约随下游持有。close 不等待网络，在锁 poison 后仍回收缓存，但后续操作
保持错误，不把异常状态当成可继续使用。此模块不授予会话权限，不验证压缩码流，
也尚未接入网络、解码或原生呈现。

本阶段 LiveRelay 组合 13 passed，包含 32 轮同时 close/commit、关闭后迟到提交、
poison 清理、帧序和租约转移。Clippy 指出的冗余迭代已改为 next_back，并通过局部
变量明确 Drain 先于锁 guard 析构，修复随之暴露的 E0597。最终源码重新通过测试、
全目标 Clippy、格式、diff 和 2461 文件有效行数门禁；未重跑默认全套或生成发行 EXE。

## N09 LiveRelay 分片接收与共享预算

新增 `live_relay_receive.rs`：保留主干 WebSocket offer 及 Legacy/JPEG/H264 profile
协商规则，拒绝未协商的压缩格式；不根据帧内容自动升级 profile。PendingFrame 先
解析固定头、校验 profile，再预留共享字节预算和分配 body，核算 Vec capacity。
分片写入不能超出声明长度；超长后候选作废，不能补写后继续交付。finish 只交付
完整唯一 owner，不复制整帧；取消、截断、错误或完成帧释放均归还预算。

这是供后台 transport 使用的单帧接收组件，不创建连接、不读取用户网络身份，也不
替代授权、epoch/帧序检查或 JPEG/H264 媒体验证。取消通过停止投递分片并释放 owner
完成，不声称能中断正在执行的一次内存复制；网络等待/任务关闭仍需后续 owner 接线。

本阶段 LiveRelay 封装和接收组合 8 passed，覆盖分片边界、截断/取消/超长作废、
profile 准入以及共享预算竞争与归还。全目标 Clippy、格式、diff 和 2459 文件
有效行数门禁通过；未重跑默认全套，未生成发行 EXE，未提交或推送。

## N09 LiveRelay NLLV 帧封装

新增 `live_relay_frame.rs`，对照 main 的 `live_relay_protocol.rs`、JPEG 上限和
`live_video::Format`，保持 NLLV v1 的 64 字节大端头、字段位置、保留位、epoch/
frame 身份、时间顺序和 raw BGRA/JPEG/H264 profile 边界。固定头编码不复制 payload；
解码返回借用视图，不分配整帧副本。单独的 decode_header 在读取或分配 body 前验证
声明大小及 codec 尺寸，最大 payload 64 MiB，JPEG 16 MiB，H264 1 MiB。

此模块只解析封装，不验证 JPEG/H264 码流内容，不授权会话、不连接 Loom，也未
接入媒体解码/采集。后续网络 owner 必须先取得授权、按头准入并预留预算，再读取
有界 body；压缩数据必须通过对应媒体验证，不可把 Envelope 当成可直接显示的帧。

测试以主干 2x1 raw BGRA 用例的固定字节头为基准，另覆盖所有截断、保留位、未知
tag、尾随数据、长度不符、零身份、时间倒序、尺寸及 profile 上限，并检查借用
payload 指针和仅凭头拒绝超大 body。N09 LiveRelay 整体仍未完成。

本阶段 4 项协议测试通过；首次 Clippy 提示两处 manual_is_multiple_of，修正后
全目标 Clippy 通过。格式、diff 和 2457 文件有效行数门禁通过；未重跑默认全套、
未生成发行 EXE，也未提交或推送。

## N07 mixed-hosting 嵌套命中回归

扩展 `live_input_mixed_dpi_tests.rs`：在 unaware 子面板右侧创建真实孙控件，以
PMv2 root 客户区坐标调用生产 `input_routing::pointer`，要求命中孙控件，而非因
逻辑/物理尺寸混用提前停在面板。真实 144 DPI root / 96 DPI 子面板回归通过，
没有复现路由缺陷，因此未对已有路由算法做猜测性修改。该用例同时保留上阶段的
控件内/外坐标转换和线程上下文恢复检查。

## N07 mixed-hosting 控件外拖动修复

新增自建 PMv2 父窗口/unaware 子控件运行回归，使用 mixed hosting 创建真实不同
awareness 的窗口树。修复前，控件内转换成功，但归一化 `[0,0]` 的控件外点返回
`Transform`，从而错误撤销拖动。直接 MapWindowPoints 返回物理偏移，不能替代
子控件逻辑坐标转换。

`live_input_dpi_bridge.rs` 保留系统转换为首选；其失败后，仅当 root 为 PM aware、
父子位于同一显示器且 DPI 有效时，取子窗口内部锚点执行系统转换，再以实际父子
DPI 比例外推拖动位置。使用 checked 整数运算并允许负坐标，不把拖动点截断进
子窗口，不用舍入后的窗口尺寸猜测比例；不满足条件继续明确返回错误。

修复后真实 root=144 DPI、child=96 DPI 的控件内/外四个采样点通过，且线程 DPI
上下文恢复。此证据是同进程 mixed-hosting，不代表跨进程不同 awareness 或多屏
迁移已完成；显示器不同或非 PM root 的系统转换失败仍无外推保证。

本阶段坐标组默认 3 passed、3 ignored；单独显式运行上述 3 项真实窗口测试全部
通过。全目标 Clippy、格式、diff 与 2455 文件有效行数门禁通过。未重跑默认全套，
未生成发行 EXE，未提交、推送或合并。

## N07 源 awareness 与调用线程 DPI 矩阵

新增 `live_input_dpi_runtime_tests.rs`，在当前真实显示环境创建 alpha=1/透明源窗口，
分别使用 unaware、system-aware、per-monitor-v2。每种源从三种调用线程 awareness
执行映射，共九种组合；检查中心和四角的 root/child 客户区坐标及物理 frame 裁剪。
裁剪像素和逻辑客户区的舍入差允许至 1 像素，客户区直接映射必须精确匹配；成功和
无效裁剪失败后都必须恢复调用线程 DPI 上下文。

显式运行 1 passed：实际源 DPI 分别为 96、144、144，五个采样点全部通过，故本次
不是仅在 100% 缩放下做同尺度检查。仍不代表不同 awareness 的跨进程父子 HWND、
不同缩放的多显示器迁移或显示配置变化已验收；生产坐标算法本轮无需修改。

## N07 真实 CLR/WinForms 补偿验收

新增 `live_input_winforms_tests.rs`，复用仓库
`scripts/tests/fixtures/LiveScreenshotPhaseZeroFixture.cs`，使用系统 .NET Framework
C# 编译器生成独立 WinForms fixture。测试直接创建并持有子进程，在使用 ready HWND
之前验证其 PID 等于所持有子进程；输出放在 linshi 的唯一目录，结束时只回收该子进程。
不枚举用户窗口，不发送全局输入。该门禁须显式选择，不加入默认桌面测试。

实际运行 1 passed：alpha=1/透明源的普通点击产生 1 次托管 Click；拖出再返回没有
额外 Click；cancel 释放没有额外 Click，pressedMouseButtons 为 0；最后恢复源样式。
证据目录：`C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-winforms-24320-1792373736487166200`。
这证明当前 fixture 的 .NET Framework WinForms 按钮兼容，不代表所有 WinForms
版本、自绘控件、混合 DPI、多屏或完整 Live UI/WGC 与 CLR 联合验收均已完成。

本阶段另重跑默认 lib 串行全套：1462 passed、18 ignored，无失败；真实托管门禁
为上述 ignored 中单独显式运行的一项。全目标 Clippy、格式、diff 及 2452 文件
有效行数门禁通过，fixture 进程已回收。未生成 Otter 发行 EXE，未提交或推送。

复现时使用 `C:\Windows\Microsoft.NET\Framework64\v4.0.30319\csc.exe`，参数为
`/target:winexe /r:System.Windows.Forms.dll /r:System.Drawing.dll`，源和输出使用绝对
反斜杠路径（旧 csc 的相对正斜杠路径在本次首次尝试中解析失败）。设置
`OTTER_NATIVE_WINFORMS_FIXTURE` 为编译出的 fixture EXE 后运行：

```powershell
rtk proxy cargo test --locked --manifest-path native/Cargo.toml --lib actual_managed_winforms_click_drag_cancel_and_restore -- --ignored --nocapture --test-threads=1
```

## N07 遮挡按钮补偿后端接线

`live_input_button.rs` 已接入 Backend 的普通发送路径：识别启用的
WindowsForms10.BUTTON push-button 样式 0/1/11，检查客户区点及遮挡，读取目标
DPI 下拖动阈值，保留绑定目标身份。普通 mouse-up 投递成功后才消费候选；重新检查
子控件、绑定父窗口、复核权限与父子关系，并以 250ms 上限发送 BN_CLICKED 通知。
不发送 BM_CLICK，不额外合成 down/up。失败清空候选，cleanup_target/release
一律先清空追踪，释放路径不能触发补偿。

自建 Win32 模拟类运行门禁已验证：三组 down/up 中，普通点击只产生一次父通知，
拖出返回和 cancel 释放不产生通知；正常鼠标消息没有因补偿重复。此模拟类不是
CLR/WinForms 实际控件，因此真实 WinForms Click/拖动兼容验收仍待完成。

本阶段后端显式运行测试 4 passed（包含新增补偿测试）、策略 4 passed、类型筛选
1 passed；全目标 Clippy、格式、diff 和 2451 文件有效行数门禁通过。未重跑默认
全套、未生成发行 EXE，也未提交或推送。

## N07 遮挡按钮补偿手势策略

对照主干 `live_source_button.rs` 新增纯策略 `live_button_gesture.rs`。仅显式左键、
无其他 held 按钮且目标为合格遮挡按钮时开始；坐标抖动允许至系统阈值，拖出再返回、
目标改变、取消、其他按钮或不再遮挡的释放均不产生候选。消费手势所有权保证候选
至多一次，坐标差使用 abs_diff 防止负坐标/极值溢出。

本阶段只实现可验证的手势策略，尚未接入 Backend 的目标类型检查、DPI 拖动阈值、
普通 mouse-up 成功后的 BN_CLICKED 通知。生产行为未启用补偿；后续必须重新验证
子目标与父窗口权限及祖先关系，且 cleanup 释放永远不得经过点击补偿路径。

本阶段 4 项策略测试通过，覆盖准入、阈值边界、拖出返回、目标变化、取消、错误
按钮、遮挡消失及整数极值。全目标 Clippy、格式、diff 和有效行数门禁通过；未重跑
上一阶段的全套测试，不将纯策略测试作为真实 WinForms 控件验收。

## N07 真实 WGC 与 Live 输入联合验收

新增 `composition_live_input_runtime_tests.rs`，通过自建源窗口及子控件验证真实
WGC、恢复子进程、alpha=1 源保护、Live 系统菜单授权、异步输入与画面反馈。
点击由 Live HWND 消息入口进入，子控件响应后由白变黑；读回实际显示帧，要求帧
序号递增且像素变黑，不以消息入队成功代替画面反馈。

显式运行 `actual_wgc_live_ui_input_feedback_focus_cancel_and_close_restore` 已通过：
失焦释放 held 键并回收输入线程；重新启用后关闭，两个 held 输入释放完成再恢复
源样式，采集和输入完成句柄均回收。该测试只操作自建窗口，不枚举用户窗口，不
合成全局键鼠输入。测试用 GPU 读回与轮询不进入生产路径。

本次重新运行默认 lib 串行全套：1457 passed、0 failed、16 ignored；上述联合
门禁单独显式运行 1 passed，不把其余 ignored 算作通过。全目标 Clippy、格式检查、
diff 检查及有效行数门禁通过（2448 文件，超过 500 有效行的文件为零）。未生成
发行 EXE，未提交、推送或合并分支。

此证据不覆盖 WinForms 补偿、混合 DPI、多屏、任意应用或物理键鼠端到端验收；
N07 和整体迁移仍未完成。下方各阶段记录保留当时的验证边界，已接线项目以本节
及紧随其后的 UI 接线阶段为准。

## N07 Live 窗口控制入口与键鼠翻译

窗口 Live 的系统菜单（标题栏菜单/Alt+Space）增加“控制源窗口”；区域 Live 项目
禁用。显式请求经 Session/worker ACK 后才显示“控制中”并准入；启用中和控制不可用
有简短标题状态，具体错误由 Session 保留。原有普通窗口操作不自动授权控制。

WndProc 把鼠标移动/三键/双击/双轴滚轮及键盘消息转成有界 Event 队列，Session 在
回调外投递；连续移动合并，16 项满载撤销而非丢边沿继续运行。只用成功呈现且与当前
客户区尺寸一致的 Viewport；黑边新按下/滚轮拒绝，已有捕获手势的移动/松开可截断到
边界。没有已呈现几何时撤销控制，不盲发键盘。正常最后一键松开释放鼠标捕获而不撤销。

失焦、捕获丢失、取消模式、最小化、进入系统菜单和窗口销毁都会撤销。StopSignal
可直接设置 worker 停止并唤醒 recv，不等待 Session 轮询，避免嵌套菜单循环留下 held
输入。Alt+Space/Alt+F4 保留本地系统菜单/关闭；进入菜单会先撤销源控制。UI 不发送
跨窗口输入、查询 token 或 join。此处已完成源码接线，尚未完成真实 WGC+可见性+
Live UI+源控件的一次完整联合验收，也未验证全部用户应用和 WinForms 补偿。

本阶段 Live 呈现/会话/窗口组 15 passed、2 ignored；单独运行自有窗口鼠标捕获门禁
1 passed（属于前组 ignored，不重复计数）。worker 7 项、Viewport 3 项和 Application
Live 7 项通过，证明 StopSignal 在 owner 不轮询/发送端仍存活时也唤醒并释放 held 键。
全目标 Clippy、格式、diff 与 2447 文件有效行数门禁通过；未重跑默认全套或生成发行 EXE。

## N07 Session/Application 输入生命周期接线

Session 已持有窗口源 Target 和可选输入 worker，提供显式 enable/send/disable 与
独立输入错误/清理报告。区域 Live 拒绝启用；Enable ACK 前不接受输入，Session
序号跨 worker 重新启用保持单调，关闭/源失效抑制迟到 ACK。输入错误只撤销控制，
不自动关闭仍有效的 Live 捕获；关闭时清理失败/未知会进入会话错误。

停止先关闭输出、请求输入和采集取消，保留输入 owner 至真实退出；源 Protection
在输入完成后才释放。采集先完成时保存结果，不提前从 Application 删除 Session。
主等待集加入输入线程句柄，最多 51 个去重句柄；关闭等待集最多 34 个，未超过
Win32 消息等待上限。UI 模式 worker 回复通过 WM_NULL 唤醒消息泵，不再为每个
会话另加回复事件句柄；PostThreadMessage 失败将停止 worker，由完成句柄唤醒回收。

Live 窗口菜单/控制开关、鼠标键盘消息翻译、失焦/捕获丢失事件仍待连接这些入口。
当前无自动开启控制，也不能把生命周期接线描述为用户已可在界面交互。

本阶段 Session 3 项、Application Live 7 项、worker 6 项测试通过；实际 Backend
3 项显式门禁也通过。新增证据包括采集先完成而输入清理仍在途时保持 Session，以及
UI 回复在线程仍存活时通过消息唤醒。最初错误测试过滤器匹配 0 项，随后已使用
`desktop::application::live` 正确运行 7 项，未把空运行当作验收。最终全目标 Clippy、
格式、diff 和 2446 文件有效行数门禁通过，未重跑默认全套或生成发行 EXE。

## N07 有界异步输入 worker

`live_input_worker` 在独立线程构造 Backend/Controller，创建后仍需 Enable 命令。
命令与回复各最多 16 项，未消费命令总量也限制为 16，进程最多 16 个输入 worker。
满载不丢边沿继续控制，而是关闭发送端并撤销整个 owner。停止使用队列外原子标志，
检查后尚未开始的队列命令不再执行；已进入的系统调用仍需返回，再清理 held 状态。

空闲时阻塞 recv，无定时轮询；回复使用手动复位事件唤醒 UI，真实线程句柄另用于
终止/回收。正常关闭保留 owner 并等待 take_exit；Drop join 仅防线程泄漏兜底，不能
当作无阻塞 UI 关闭。错误终止保留清理报告；panic 时报告未知清理（None），不冒充
全部已释放。初始化失败、满载停止或线程创建失败均须释放 worker 名额和句柄。

此 worker 已可驱动真实 Backend，但 Application/Session/UI 尚未接入其两类等待句柄、
控制开关及关闭状态。不得仅在 UI 关闭时直接 Drop 来代替消息泵中的异步结算。

本阶段 worker 5 项测试通过，覆盖异线程执行、满载撤销/抑制队列、错误清理、
初始化/panic/空闲退出以及进程级名额回收。实际 Backend 3 项显式门禁通过，其中
新增异步 worker 对自有 alpha=1 窗口发送、停止释放两种 held 输入并回收线程的证据。
最终全目标 Clippy、格式、diff 与 2445 文件有效行数门禁通过；未重跑默认全套。

## N07 Controller 与实际 Win32 Backend 联合

`live_window_target::input_backend::Backend` 已将安全预检、保留/命中/焦点路由、
源到子目标坐标和受限 Win32 消息接到 `live_input::Controller`。创建后仍需显式
enable，不从捕获或绘制自动启用。Controller 的 Target 约束从 Copy 改为 Clone，
固定容量 held 状态可持有共享原进程句柄的 Destination，不退化为裸 HWND。

后端在发送前保存最后有效鼠标客户区/root 坐标；取消释放使用已保存的位置，
不重新命中或重算最小化/移动后的坐标。子目标失效时只有原 root 可回退，释放
使用对应 root 坐标；失败/跳过仍通过 Controller 清理报告暴露。cleanup 不做点击补偿。

该连接目前可由独立测试调用线程驱动实际窗口，生产异步有界 worker、UI 开关与
失焦/捕获丢失接线尚未完成，禁止 UI 线程直接执行。WinForms 特例点击补偿、relay
控制权和混合 DPI 完整兼容仍待迁移，不能将 Backend 联合测试当作 N07 全部完成。

本阶段 Controller 默认 8 项回归通过；真实 Win32 联合门禁 2 passed：透明自有源
命中子控件、键鼠按下/拖出目标/取消释放、子控件销毁后按 root 坐标释放、源销毁后
跳过清理并拒绝新输入。所有消息只发给夹具窗口。最终全目标 Clippy、格式、diff 与
2443 文件有效行数门禁通过，未重跑默认全套或构建发行 EXE。

## N07 子控件命中、焦点与保留目标

`live_window_target::input_routing` 从显式 root 内只读下钻最多 64 层，跳过 disabled、
invisible、transparent 子控件，整次最多检查 256 个子窗口，按直接子窗口 Z 序与
父客户区裁剪检查矩形；每个结果经 Destination 绑定/安全预检，不调用桌面
WindowFromPoint 或全局枚举。候选不合法时最多 64 层向原 root 回退，不跨越原授权根。
键盘路由只读取源线程焦点，外部焦点回退 root，不 SetFocus 或 AttachThreadInput。
已按下手势优先保留原 Destination，失效只能回退同一个原 root，不重新命中另一控件。

路由自身不修改 WS_EX_TRANSPARENT，也不持有样式恢复锁；是否可替代主干临时样式
清除需依据真实源兼容性继续验证。最初测试夹具把 root 设置为真正不可见，因
CWP_SKIPINVISIBLE 回退 root；生产隐藏是 alpha=1 且窗口可见，测试须区分这两种状态。
实际 alpha=1/TRANSPARENT 可见夹具下，系统 ChildWindowFromPointEx 仍返回空；因此
采用上述有界几何遍历，不临时移除 root 透明样式。夹具证明可以命中两层子控件且
root 样式不变；这不是任意自绘控件/窗口 region/自定义命中协议的兼容证明。
当前仍未接入 Controller/worker/UI，不能据此宣称端到端输入完成。

最终显式路由门禁 1 passed，覆盖透明可见源的深层命中、禁用/隐藏/透明子节点跳过、
预算耗尽、DPI 恢复、手势保留及销毁回退。全目标 Clippy、格式、diff 与 2441 文件
有效行数门禁通过，未重跑默认全套或构建发行 EXE。

## N07 输入坐标与 DPI 作用域

`live_window_target::input_coordinates::map` 验证 root 与 Destination 属于同源，
全窗口模式沿用主干客户区归一化端点规则；窗口内裁剪使用 DWM 物理 frame 边界，
验证非空及区域包含后转换为源逻辑客户区。拒绝 NaN/Infinity、越界归一化点、
最小化过渡、无效区域及已改变的目标。像素先在局部非负范围舍入，再加屏幕原点，
避免负屏幕坐标的半像素舍入偏移。

源和子目标可能 DPI awareness 不同，坐标通过物理屏幕坐标桥接，再使用目标 DPI
上下文转换为目标客户区及滚轮屏幕坐标；所有线程 DPI 上下文均作用域恢复，包括
错误返回。DWM 失败时 GetWindowRect 回退临时使用 PMv2，避免把虚拟化边界当物理值。
依据：[GetWindowRect](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowrect)、
[PhysicalToLogicalPointForPerMonitorDPI](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-physicaltologicalpointforpermonitordpi)。

该层不授予输入权限，不替代 Viewport 的留黑边准入；message 层仍验证 signed 16-bit
范围。尚未接入 UI/worker，真实混合 DPI 跨进程子控件、多屏迁移和有边框 WGC 裁剪
尚待联合验证，不能以几何单测宣称全部兼容。

运行测试曾在隐藏夹具的冗余 DPI 桥接返回 Transform。修正为同 DPI context 直接
映射，PM-aware 坐标不重复转换；不同 context 转换失败仍保守拒绝，不猜测结果。
最终 2 项纯几何测试及 1 项显式 PMv2 自有隐藏窗口测试通过，覆盖客户区到子控件、
实际物理裁剪、成功/失败后的 DPI 恢复与源关闭拒绝。非 PM 隐藏裁剪和混合 DPI
跨进程仍未取得运行证据，不算兼容完成。

## N07 已绑定子 HWND 的身份与投递

`live_window_target::input_target::Destination` 只接受原 root 或其后代，保留原 root
身份以及子目标的 HWND/PID/TID、原进程查询/同步句柄。clone 共享句柄，不复制所有权。
每次预检核对 root、原进程仍存活、子目标身份和祖先关系，再单独检查子目标 token
的用户/session/完整性边界；子控件属于另一进程时也不能跳过安全检查。

`input_message::send_to/release_to` 已使用 Destination 预检并发送至实际子 HWND。
重新挂到其他 root、销毁或原进程退出都必须拒绝；不会自动重绑定缓存 HWND 或命中
其他窗口。与 root Target 一样，同一存活进程/线程中快速复用同一 HWND 仍是身份
粒度限制，不声称已经消除此系统竞态。

这一步是子目标绑定/投递，不是自动子控件路由：深层命中、焦点选择、透明样式协调、
DPI/帧坐标转换、失效后 root 回退及 Controller/worker/UI 接线仍待完成。

本阶段自有隐藏窗口的身份/clone/reparent/销毁门禁 1 passed；实际 root/子 HWND
发送与释放门禁 1 passed（保留零返回值、超时与关闭拒绝检查）。安全组 3 passed，
编码组 2 passed；真实跨进程子窗口仍未运行验收。全目标 Clippy、格式、diff 与
2437 文件有效行数检查通过；未重跑默认全套或构建发行 EXE。

## N07 Win32 输入消息编码与 root 投递

`live_window_target::input_message` 只构造键鼠消息：客户区鼠标点与屏幕滚轮点使用
不同入口，signed 16-bit 坐标溢出明确拒绝，不截断到另一显示器；按钮掩码、滚轮
delta 和虚拟键均验证。正常投递前进行 Target 安全预检，拒绝同线程路径及普通输入
的最小化过渡。release 只接受松开消息，不触发点击补偿。

SendMessageTimeout 使用 BLOCK/ABORTIFHUNG/ERRORONEXIT，移动/清理 50 ms、其他
250 ms；判断 API 完成标志，消息返回值为 0 不算失败。失败可能是超时、UIPI 或其他
未知投递结果，不能宣称目标未收到。独立 worker 不得 AttachThreadInput 到目标队列：
[Microsoft 文档](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendmessagetimeoutw)
指出同队列直接调用会忽略超时，仅检查不同线程不足以覆盖主动合并队列的调用方。

当前只实现绑定 root 的消息原语，不是完整 Backend：子控件命中/身份、DPI 和捕获帧
转换、透明样式协调、WinForms 点击补偿、worker/Controller/UI 接线仍待迁移。
原语不会自动取得显式控制授权，不能从捕获启动入口直接调用。

本阶段 2 项编码测试通过；显式运行自有隐藏窗口测试 1 passed，验证零消息返回值
仍成功、真实 release 成功、延迟窗口过程触发超时、同线程拒绝及关闭后拒绝。测试
仅向夹具发送键消息，不操作用户窗口。最终全目标 Clippy、格式、diff 和 2435 文件
有效行数门禁通过，未重跑默认全套。超时用例是调度环境下的运行证据，不是硬实时保证。

## N07 输入控制状态与撤销释放

`live_input::Controller` 默认禁用，仅显式 enable 且 backend 预检成功后准入输入。
序号限定 1..=9007199254740991 且严格递增；预检后、转换和投递前消耗序号，错误或
超时不允许重放。取消/重新启用不重置序号。坐标有限且位于 0..=1，虚拟键 1..=254，
滚轮非零且绝对值不超过 1200；最多持有 32 键和 3 鼠标键，生产状态无动态事件队列。

发送前记录可能已按下的状态，成功 up 才移除；任一 dispatch 错误撤销控制并尝试
清理。鼠标多键手势保持原目标，keyup 对应 keydown 目标；重复 keydown 也保持该目标，
这是对主干重复按下时可改目标行为的收紧，避免旧目标遗留按下。cancel/Drop 最多
35 次释放，报告成功、失败、跳过数量，不把本地清空当作系统确认。普通 send 与
cleanup release 接口分离，后者禁止延迟点击补偿；身份失效必须跳过，不能写入复用 HWND。

此层通过 backend contract 保留实际 HWND 安全/命中/DPI/发送责任，当前测试 backend
不发送系统输入。原生 transport、有界异步 worker、UI 控制开关及焦点/捕获丢失接线
仍待完成；不能把状态模型完成当作 Live 输入投射已可用。普通发送 250 ms、移动及
清理 50 ms 是传给 transport 的上限约定，尚不是实际 Win32 超时验收证据。

该增量 8 项聚焦测试通过，覆盖超时按下、失败松开、目标保持、序号、键上限、
非法载荷、权限撤销和 Drop 清理。最终全目标 Clippy、格式、diff 与 2433 文件
有效行数门禁通过；没有重跑默认全套，也没有构建新的发行 EXE。

## N07 输入安全预检基础

`live_window_target::input_security::preflight` 接收已绑定 Target，前后复核原进程存活
和 HWND 身份，检查当前线程/输入 desktop 均为 default，并通过只读进程 token 检查
同 session、同非空用户 SID、控制方完整性级别不低于源。读取失败保守拒绝；不申请
debug/elevation 权限，不切换桌面。预检结果不能缓存成永久授权，不替代用户显式启用，
也不能消除检查与最终发送之间的 OS 状态竞态，发送链仍需处理失败并释放 held 状态。

本阶段验证：安全组默认 3 passed、1 ignored；另显式运行自有隐藏窗口的预检/销毁
拒绝测试，1 passed。未发送输入，未测试真实跨账户/提权目标；这些拒绝分支当前由
策略测试覆盖。全目标 Clippy、格式、diff 与 2431 文件有效行数门禁通过，未重跑全套。

token 缓冲区按 usize 对齐、最多 64 KiB；内嵌 SID 指针只能读取实际返回长度，不能
进入对齐填充区。句柄采用 RAII，线程 desktop 仅借用，打开的 input desktop 单独关闭。
此模块尚未接入 UI/输入 transport，不代表输入投射完成；区域捕获不得因此获得控制权。

API 依据：[OpenProcessToken](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-openprocesstoken)
要求查询权限并关闭 token；[OpenInputDesktop](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-openinputdesktop)
要求关闭返回 desktop，断开的会话可能返回重连后将激活的桌面，因此名称检查不是当前
用户在场或会话已连接的证明。本阶段保留主干规则，不宣称用户在场认证。

## N07 输入投射准备：共用几何与呈现时序

`live_viewport.rs` 统一实际 D2D 绘制的 f32 等比适配矩形与帧像素映射，拒绝黑边、
右下半开边界、非有限坐标和零尺寸。拖动释放的 clamped 映射单独提供，调用方必须
先证明已有授权和按下状态，不能用于接受黑边的新按下。映射不分配图像或复制像素。

Live Host 只在 draw/present 成功后记录序号和几何；pending 帧不提前更新记录。
当前客户区尺寸不符时拒绝旧几何，暂停、关闭或渲染失败清空记录。Present 成功表示
提交成功，不是 compositor 扫描输出证明；记录不持有额外 GPU 帧或增加捕获预算。

此处只完成显示坐标基础，不是输入投射完成。后续仍需显式授权、同用户/会话/完整性
及桌面校验、帧到源客户区转换、严格事件序号、异步有界发送、held 状态与撤销释放，
以及 relay 权限对齐。尚未向任何外部 HWND 发送输入，也未增加默认输入权限。

## 最新运行证据：真实 WGC 与源隐藏/召回联合门禁

`live_visibility_runtime_tests.rs` 仅创建并捕获自己的 128×96 纯色 HWND，分别从
普通状态、初始最小化状态开始。使用实际 WGC、真实恢复子进程与应用级 Owner：
登记 ACK 后 alpha=1，白色帧后修改为黑色并断言新帧像素；受控召回恢复原扩展样式
及可见性，然后改回白色并确认 Live 仍持续交付；最后取消采集并结算恢复子进程退出。
最初最小化源召回后展开，与主干规则一致，不断言回到最小化状态。

首次运行暴露的是测试清理断言错误：CaptureJob 显式取消返回 `Capture cancelled`，
不能要求 Ok。修正夹具后两种状态连续三轮通过，回执均包含
`hidden-frame=true update=true reclaim=true capture-continued=true reaped=true`。
全目标 Clippy、格式、diff 和 2427 文件有效行数门禁通过；未重跑默认全套。

该测试默认 ignored，要求交互式 Windows 桌面，会短暂显示自己的小型纯色窗口：

```powershell
cargo test --locked --manifest-path native/Cargo.toml --lib live_visibility_owner::runtime_tests::actual_wgc_hidden_source_updates_and_reclaim_keeps_capture_alive -- --ignored --nocapture --test-threads=1
```

像素 staging 读回与 5 ms 等待只用于测试，不进入生产路径；不枚举或读取用户窗口。
Owner 使用测试恢复启动器与受控 Intent，不安装全局输入 watcher，因此没有证明
真实任务栏/Alt+Tab 手势；任意用户应用、HDR/DPI、多屏、长时间性能仍待验收。

## 当前接线状态：窗口 Live 恢复协调已进入 Application

最新 watcher 增量：`live_reclaim_watch.rs` 按隐藏需求安装键鼠低级 hook 和前台/
最小化 WinEvent；只在物理按键/按钮边沿更新有界 Intent 状态，不拦截或转发用户
输入、不保存文本历史、不处理鼠标移动。`live_reclaim_input.rs` 对齐主干的 shell
类名与 Alt+Tab/Alt+Esc、Win+Tab/数字组合；其他物理按下撤销旧意图，注入输入既不
创建也不取消。修饰键依据物理边沿维护，安装前已按下的键保守不授权。

回调只置手动复位事件，UI poll 读取最新快照并检查实际前台/最小化状态；应用等待
集合最多 35 个去重句柄。全部源召回或应用关闭即注销 watcher，无新增轮询线程。
回调重入/错误保守失败并进入应用清理。测试使用事件 owner 和合成分类输入，不发送
全局键鼠；真实任务栏/Alt+Tab 手动召回与长时可用性仍待验证。hook 安装/注销另由
显式生命周期门禁检查，不等于实际手势端到端验收。
shell 类名匹配沿用主干行为，不是 Explorer 进程签名认证。

API 依据：[Microsoft LowLevelKeyboardProc](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelkeyboardproc)
说明回调先于异步键状态更新，且低级 hook 超时可能被系统静默移除；本实现不使用
回调内 GetAsyncKeyState，不宣称可检测所有静默移除。[SetWinEventHook](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwineventhook)
要求安装线程保持消息循环并处理回调重入。本轮以下旧“watcher 未接入”是阶段记录，
当前以本段为准，不能把源代码接线与真实 shell 操作验收混为一谈。

默认串行回归在新增显式门禁前完成：1466 passed、9 ignored、0 failed。此后新增
1 项默认 ignored 的真实 hook 生命周期门禁；它短暂安装/卸载 3 次，不发送输入，
并检查卸载后原 hook 句柄不能再次注销，区分仅清空本地 TLS 与实际释放系统 hook。
该显式门禁已通过，回执为 `install=true unregister=true reinstall=true input-synthesized=false`。
最终 Clippy、格式、diff 与 2426 文件有效行数检查通过，超过 500 行的各分档均为零。
显式运行方式：

```powershell
cargo test --locked --manifest-path native/Cargo.toml --lib live_reclaim_watch::tests::actual_hooks_install_unregister_and_reinstall_on_owner_thread -- --ignored --nocapture --test-threads=1
```

召回增量：Owner::observe_reclaim 复用每源 Activation 和最新 Intent 快照，匹配
显式 shell 召回后恢复共享源，不减少 Protection/Lease 引用，不关闭采集。召回状态
先于恢复写入保留，恢复失败继续重试；退休待定不重新隐藏。全部登记退休后允许关闭
helper，即使已召回的 Live 会话仍存活。隐藏前持续吸收当前意图，防止握手期间的
旧手势在隐藏完成后立即召回。OS 输入 watcher 尚未连接此入口，真实 shell 手势
识别、注入输入排除和唤醒仍是后续工作；当前联合测试显式注入受控意图。
本增量实测：Live 聚焦组 101 passed、3 ignored；Application Live 7 项通过
（与前组有重叠，不相加为独立用例数）。两项新增 Owner 测试验证无意图/旧意图/取消
拒绝、有效召回恢复同源两个 peer、退休后 helper 退出但 peer 保持存活且不会再隐藏。
Clippy、格式、行数和 diff 门禁通过。本增量未重跑默认全套，后文 1460 项为上一检查点。

`live_visibility_owner.rs` 现持有同源共享的登记、原始快照、Registry 和按需启动的
Journal。Application 选择窗口 Live 时先取得 Protection，再启动采集；区域 Live
不启动恢复进程。Session 仅在登记 ACK 已到达、输出仍存活且任务未完成时 arm/hide，
避免关闭后的迟到 ACK 再次隐藏源。多个同源 Session 不重复登记或覆盖初始快照。

Session 关闭/结束释放其 lease；最后一个会话恢复源，收到退休确认后才清除本地
责任。最后源结束后发送 Shutdown 并等恢复子进程退出，不留下空闲恢复 helper。
Application 的正常等待集合最多 34 个去重句柄；关闭等待采集任务和恢复任务/子进程，
持续分发消息。隐藏/登记失败关闭对应 Live 输出，恢复失败保留源表，不伪造成功。

此轮完成实际接线，不等于 N07 全部验收：shell 激活召回、最小化用户应用兼容、
多屏/DPI/HDR、输入投射和长期资源曲线仍待完成或验证。下文为分阶段证据记录，
其中“尚未接 Application/未启用隐藏”等描述仅代表对应历史检查点，以本节为准。

本轮实测：应用级 Owner 3 项通过，Application Live 7 项通过；联合夹具使用真实
恢复子进程、自有隐藏窗口和可控采集 worker，验证同源共享、最后 lease 恢复、取消、
源销毁、窗口恢复与采集线程/恢复子进程一起退出。不把可控 worker 当作完整 WGC
用户应用验收。最终默认串行回归 1460 passed、9 ignored、0 failed；全目标 Clippy、
格式、diff 和 2423 文件有效行数门禁通过，超过 500 行的各分档均为零。

## Live 实际运行证据与关闭修复

`native/tests/live_window_runtime.rs` 显式创建一个小型可见纯色窗口，仅用 CreateForWindow
捕获该测试 HWND，不枚举用户窗口或读取桌面。验证 128×96 白色首帧、变黑后的更新、旧
GPU 快照不被覆盖、调整到 256×144 后继续交付、同设备 D2D/DirectComposition 呈现及
源关闭后线程/帧 owner 回收。输出窗口保持隐藏；没有保存图像或实际用户内容。

首次及诊断复跑均发现源 HWND 已销毁但 5 秒内 worker 不退出；临时诊断没有收到 WGC
Closed，测试失败后的显式取消才唤醒等帧。已增加限定 PID/TID/HWND 的销毁 WinEvent，
回调只关闭帧源，工作线程醒来后保留 TargetChanged 结果，完成时在安装线程注销 hook。
不使用周期检测或额外线程。修复后该真实捕获测试连续三次通过；这不是完整 N07 验收。

此测试默认 ignored，要求交互式 Windows 桌面，会短暂显示自己的测试窗口。显式运行：

```powershell
$env:CARGO_TARGET_DIR='C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-target'
cargo test --locked --manifest-path native/Cargo.toml --test live_window_runtime -- --ignored --nocapture --test-threads=1
```

通过标记为 `native-live-window-runtime: white=true update=true snapshot-stable=true resize=true present=true close=true`。
实际用户窗口、托盘/候选菜单手测、多屏/DPI、HDR、最小化/隐藏、投射输入、
长时间资源曲线仍待验证；测试总用时不是启动延迟、帧延迟或性能基准。

跨进程退出另由 `native/tests/live_process_runtime.rs` 验证：父测试只启动并捕获自己的
纯色子进程，核对 HWND 的 PID 后才绑定。正常 WM_CLOSE 和强制结束该子进程均验证实际
首帧、源进程句柄唤醒、等待句柄退休、worker 回收及最后帧释放；不操作用户应用。
UI 等待集合现在最多 32 个去重句柄（16 个会话各自的 worker 和源进程），源退出后
关闭帧源并移除已就绪的进程句柄，避免永久就绪导致忙循环，不用取消覆盖源失败。

首次重复采集的测试宿主未像实际应用那样保活 COM apartment，出现
`GraphicsCapture.dll_unloaded` / `0xc0000005`。对齐应用的持续 STA 生命周期后两种
退出测试通过；这不是对驱动崩溃根因的穷尽证明。Live 启动现在拒绝未初始化或仅隐式
MTA 的调用线程；调用方仍须保持显式 apartment 到所有任务退出，准入检查不代替所有权。
准入回归另验证其他线程持有 MTA 时仍拒绝未初始化的 owner；区域与窗口绑定失败测试
显式初始化 STA，并断言绑定回调确实执行及原始错误保留，防止提前准入失败造成假覆盖。

该测试也默认 ignored；必须用父测试名称过滤，不能单独运行其子进程辅助测试：

```powershell
cargo test --locked --manifest-path native/Cargo.toml --test live_process_runtime actual_wgc_external_process_normal_and_forced_exit -- --ignored --nocapture --test-threads=1
```

两条回执分别为 `forced=false` 和 `forced=true`，均有
`frame=true process-wake=true retired=true joined=true`。受控子进程证据不代表任意用户
应用、多屏/HDR、最小化/隐藏策略或长期性能已经验收。

## N07 下一阶段：外部源可见性合同

### Application 关闭及消失源结算

实际 Application::shutdown 已改为先停止 Live 会话，再等待线程句柄并持续分发
UI 消息，完成后才回收会话，避免跨线程窗口调用与 UI join 互等。清理时遇到 WM_QUIT
暂存退出码，清理结束后再还给外层；不重新入队忙循环，不启用轮询定时器。
该路径暂只覆盖现有采集 Session，Journal 的应用等待集合接线仍待完成；驱动阻塞
和等待 API 故障仍不能保证有界退出，最终析构也没有改为 detach。
Application::run 另将 run_loop 的正常与错误返回统一收拢，再执行 collect_closed 和
shutdown，最后返回原始错误；非零 WM_QUIT 及 `?` 不再直接进入字段析构绕过清理。
这不是任意 panic 的 unwind 保证。新增测试实际调用 Application::run 并注入非零
WM_QUIT，让 worker 在停止后同步请求 UI 响应，检查错误原文保留与线程回收。
错误出口增量实测：Application Live 6 项通过；startup、profile_startup、session_startup、
settings_startup 四组真实 EXE 回归共 22 项通过。Clippy、格式、2421 文件行数及
diff 门禁通过。本增量未重跑全套；下文 1455 项默认回归属于上一检查点。

本地 lease 的源消失处理统一复用 Snapshot::restore_or_retire，与独立恢复 Binding
保持相同判断。确认原进程退出、窗口消失或 PID/TID 已变才跳过窗口写入；查询失败
不视为消失。跳过写入后仍须远端退休确认，本地快照不能直接丢弃。聚焦测试使用
自有隐藏窗口，验证销毁后仍能退休实际恢复登记并正常 Shutdown，而不是强制杀进程。

本阶段验证：Live 名称聚焦组 95 passed、3 ignored；Application Live 组 5 项通过，
其中新关闭测试让 worker 在停止后同步 SendMessageTimeout 至 UI，并断言 UI 确实
响应而非超时、线程回收和 WM_QUIT 退出码保留。最终默认串行回归 1455 passed、
9 ignored、0 failed；all-targets Clippy、格式、diff 检查通过。有效行数检查覆盖
2421 文件，所有超过 500 行的分档为零。该证据不替代用户窗口/WGC 驱动异常验收。

### 当前增量：异步恢复登记协调

`client::journal::Journal` 在 UI owner 上管理最多 16 个登记意图，向异步通信 owner
最多提交一个请求。登记 ACK 前 Ticket 不授权隐藏；取消尚未发送的登记无需通信，
取消在途登记则等待 ACK 后退休。退休回调只设置意图并返回 E_PENDING，直到实际
退休 ACK 到达才成功，避免 UI 同步管道等待或过早清除恢复快照。

正常 close 撤销新隐藏授权，但不直接退休仍在保护的源：调用者必须先恢复源窗口。
通道错误后登记状态可变为 Unknown；没有 Client 句柄不证明恢复进程已经退出，
不能伪造退休成功。仅持有的原子进程句柄确认退出，才能消除旧快照重放风险。
请求 worker 现将 Client 保留在 unwind 边界之外，panic 后仍交回原进程句柄，并
封闭通信流。故障 Journal 只有在全部已发送源已恢复/取消并请求退休后，才通过原始
句柄终止自建恢复进程；确认退出前继续保留本地快照。仍受保护的源阻止终止，不用
重新打开 PID 或关闭管道代替退出确认。若意外失去 Client/句柄，仍保守保留 Unknown。
后续需接 Application 的事件等待、源恢复及正常/异常退出流程后才能启用用户源隐藏。

聚焦测试入口（不启用 ignored 辅助入口）：

```powershell
cargo test --locked --manifest-path native/Cargo.toml --lib live_recovery_client::journal -- --nocapture --test-threads=1
```

该组覆盖真实登记/退休 ACK、在途取消、16 源上限、最后 Ticket 释放、启动失败、
未知责任不误退休及 owner 丢失后授权撤销。联合测试使用自有隐藏窗口与真实恢复子进程，
检查共享 lease 恢复后仍持快照、退休 ACK 后才允许下一代登记，以及最后 lease 退出
后的异步退休。测试不代表实际用户窗口、最小化源或应用关闭路径已经验收。

登记协调首轮实测：上述 8 项聚焦测试通过；当时默认串行回归合计 1450 passed、9 ignored、
0 failed。ignored 包含受控运行入口，不能用全量 `--ignored` 替代显式父测试过滤。
all-targets Clippy（拒绝 warning）、格式检查及 diff 空白检查通过；有效行数门禁
检查 2419 个文件，501–700、701–1500、超过 1500 的违规计数均为零。
联合测试中的 `native-live-visibility-restore: HRESULT(0x8000000A)` 是最后 lease
释放时报告的预期异步退休待定，随后 ACK、retry_pending 与子进程退出均通过断言。

故障收尾补充验证：父端 Client 范围 19 passed、3 ignored；新增测试确认真实请求
ACK 后发生 panic 仍保留同一个子进程句柄、失败流不能继续通信。另一项使用真实
恢复子进程和两个登记源：只恢复一个源时禁止终止，全部请求退休后才终止并等待退出，
随后允许 lease 清除快照。协调层故障由测试注入，不宣称覆盖所有 OS 断管时序。
显式三进程运行门禁再次通过，正常返回和 process::exit 均恢复完整快照且子进程退出；
Clippy、格式、行数及 diff 门禁通过，未发现本次测试目标目录下的残留进程。
该补充未重跑默认全套测试；上面的 1450 项结果属于前一检查点，不能当作本次重跑。

父端认证增量：`live_recovery_parent.rs` 打开并持有原父进程同步/有限查询句柄，核对
ToolHelp 直接父 PID、父创建时间不晚于子进程、canonical executable 路径相同，以及
实际输入及 ACK 输出管道的 server PID。零/自身 PID、错误父 PID、任一非管道端点拒绝；不申请终止权限。
Parent::exited 使用原句柄，而不是 EOF 或重新打开 PID。集成测试显式创建同一测试
executable 的无窗口子进程，验证真实双向父管道通过、NUL 输入、NUL 输出与错误 PID 拒绝，子进程
均由测试 owner 有界等待和回收。认证只覆盖父端身份，不代表已有 session 握手超时、
完整恢复循环或二进制签名证明；这些接线仍待完成。

`live_recovery_handshake.rs` 已提供已认证通道的 Hello/Ready 编解码，区分握手方向与业务
请求，Ready 必须匹配非零 session；反射 Hello 不能冒充 Ready，未知字段、错 schema、
截断和空输入拒绝。握手复用 4096 字节 framing；codec 不启动进程，Ready 前不能登记或隐藏。

`live_recovery_pipe.rs` 与 `live_recovery_pipe_pair.rs` 已提供真实本地双向 overlapped
管道与 I/O owner：随机单实例名称、拒绝远程客户端，每次操作最多 4096 字节；绝对
截止时间不被短读写刷新，事件和对端原句柄联合等待，不创建后台读写线程或空闲轮询。
超时/对端退出先 CancelIoEx，再确认操作完成后释放 OVERLAPPED、事件及缓冲区；
取消清理可能等待内核，不能宣称硬实时期限。部分读写不是事务回滚，协议 owner 必须
废弃失败流或保留完整增量解码状态，不能在未知偏移上重发整帧。
同进程真实管道回归覆盖 Hello/Ready、读超时后零传输缓冲区复用、对端关闭、退出句柄
唤醒和不消费对端的写入背压；这不等于生产子进程握手或父死后的窗口恢复验收。
本阶段验证：管道定向测试 5 项通过；完整默认测试集以 `--test-threads=1` 运行，
1421 项通过、5 项忽略（14 个测试套件）。Clippy `--all-targets -- -D warnings`、
格式检查、有效代码行数 ratchet 和 `git diff --check` 通过。忽略的运行时/辅助入口
未在此次默认测试中执行，不将历史手动运行结果计入本次通过数。

会话 owner 增量：`live_recovery_session.rs` 认证交付管道的原父端后，使用五秒总预算
完成 Hello/Ready。业务空闲等待首字节不轮询；首字节到达后，剩余帧及 ACK 共享五秒
绝对期限，窗口绑定调用本身不是硬实时操作。请求/ACK 任一步失败即封闭该流，不猜测
半包边界，不删除已登记源 owner。`wait_and_recover` 只等待原父句柄，不依据 EOF
推断父死；恢复失败保留记录，调用方必须持有 Session 并安排重试。子进程入口与重试
生命周期现由下述 runtime 承接；父端启动接线尚未完成，禁止启用源隐藏。
本增量的真实管道定向测试 4 项通过：正常登记/重放/退休/关闭、半帧超时、断管、
绑定完成后 ACK 写失败；后三者均保留登记责任并封闭失败流。Clippy 全 target 检查
通过；上述 1421 项完整默认测试结果属于此前传输层检查点，不是本增量的全量复测。

继承端点接收与跨进程验证：`Pipe::adopt` 先核对字节管道类型，再使用内核
FileModeInformation 拒绝同步 I/O 模式，查询失败即拒绝，不以环境变量或命令行作为
overlapped 证明。同 executable 的真实测试子进程通过继承 duplex 端点执行实际
Session::accept，完成 Hello/Ready、不可绑定源 Rejected ACK 和空登记 Shutdown。
测试框架 stdout 与协议隔离，子进程由 fixture owner 有界等待及回收；辅助入口只由
父测试启动，不应单独全量执行 ignored 测试。实际父死恢复验收仍未完成。
FileModeInformation 的语义依据 [Microsoft NtQueryInformationFile 文档](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntqueryinformationfile)。
本增量定向验证：管道测试 6 项通过；真实会话集成测试 1 项通过，1 项子进程辅助
入口按设计忽略（由父测试显式启动）。测试结束复核未发现该 fixture 子进程残留。
本次未重跑完整默认测试集，不能把上述定向结果当作整体原生迁移已完成。

生产子进程入口增量：`live_recovery_runtime.rs` 提供独占参数
`--native-recovery-parent PID`，main 在设置、会话与桌面初始化之前分流，不读取用户
profile。runtime 认证继承管道并处理请求；正常空登记 Shutdown 退出，断管无登记可
报错退出，有登记则持有原 owner 等待父进程真正退出。恢复失败按 1/2/4/8/16/30 秒
退避并持续保留记录，不创建额外线程，不把失败限次后丢弃。Binding 增加明确的过期
退休：原进程已退出、HWND 不存在或 PID/TID 改变时不再写窗口；查询失败不能冒充
过期，仍由 runtime 重试。同一存活 PID/TID 内快速 HWND 复用仍是既有边界。
实际父端启动/健康检测和跨进程窗口恢复验证尚未完成，仍禁止启用外部源隐藏。
本增量验证：恢复协议/runtime 定向库测试 24 项、源绑定/过期退休测试 4 项通过；
跨进程与实际 EXE 私有入口拒绝测试 2 项通过，1 项受控辅助入口忽略。真实 EXE 测试
覆盖零 PID、混用参数、NUL 端点拒绝；不代表合法生产父端启动或故障恢复循环已验收。
Clippy 全 target、格式、行数 ratchet 和 diff 检查通过；未重跑完整默认测试集。

父端 owner 增量：`live_recovery_client.rs` 使用 current_exe 启动无窗口恢复进程，
显式交付 duplex 端点并立即释放 Command 持有的父端复制句柄。随机 session、五秒
握手预算、连续序号和精确 ACK 对应关系已接通；本地非法请求不消耗序号，传输失败
则封闭通道，ACK 不确定不能视为登记回滚。Ready 前无登记责任，失败可终止并回收
本次自建子进程；Ready 后 owner Drop 只关闭端点/句柄，不杀可能仍持有记录的子进程。
健康检查使用原进程句柄；空登记 Shutdown ACK 后才能等待退出。所有阻塞操作只能
在后台执行，不能在 UI 消息回调中握手或 join。真实测试子进程验证正常关闭和退出
拒绝；生产 Live owner 的调度接线及父死后窗口恢复验收仍未完成。
本阶段最新完整默认测试：以 `--test-threads=1` 运行，1433 项通过、7 项忽略，
共 15 个测试套件；其中父端 Client 定向测试 2 项通过，辅助入口由父测试受控启动。
Clippy 全 target、格式、有效代码行数 ratchet 和 diff 检查通过。忽略的实际运行时
场景不计入本次默认通过数，完整默认测试通过不等于真实父死窗口恢复已经验收。

三进程运行门禁增量：显式执行
`cargo test --locked --manifest-path native/Cargo.toml --lib live_recovery_client::runtime_tests::actual_parent_exit_restores_surviving_window_and_eof_does_not_restore_early -- --ignored --nocapture --test-threads=1`。
观察者创建并保留隐藏窗口，登记父进程经真实恢复 Client 收到 ACK 后才修改 TRANSPARENT
样式。关闭管道后仍存活 200 ms，确认未提前恢复；父进程正常返回和直接 process::exit
两种路径下，恢复子进程均还原完整快照并自行退出。该证据不覆盖用户窗口、真正的
透明隐藏样式组合、强制 TerminateProcess 或故障恢复退避的全部场景。
首次测试曾因观察者等待输出 EOF 而停止泵消息；有界读取父进程退出时已到达字节后
门禁通过。后代继承输出句柄是待核查原因，生产启动的继承句柄隔离仍需单独验证，
不能把夹具修复当作该项已经完成。测试只回收本次自建进程及窗口，不修改用户窗口。

句柄隔离后续核查已取得独立反例：可继承事件由父测试创建，旧 Command::spawn
启动的恢复子进程能直接触发它，测试确实失败。恢复专用启动改为 CreateProcessW +
STARTUPINFOEX + PROC_THREAD_ATTRIBUTE_HANDLE_LIST，只传递协议端点和 NUL；
同一测试修复后通过。属性列表、句柄数组和临时可继承副本活到 CreateProcess 返回，
初始线程句柄立即关闭；既不遍历/修改父进程其他句柄，也不靠进程级继承开关竞态规避。
参数编码独立于 shell，固定 current_exe、拒绝自定义 cwd、检查 NUL/长度，覆盖空参数、
引号、末尾反斜杠和 Unicode。生产继承原环境，测试环境覆盖不修改父进程全局环境。
API 合同参见 [Microsoft UpdateProcThreadAttribute](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute)。
该证明只覆盖恢复子进程的继承集合，不代表仓库中所有进程启动路径已经审计。
本增量验证：父端/启动文本定向测试 6 项通过；显式三进程运行门禁 1 项通过，包含
正常返回与直接 process::exit 两种路径。Clippy 全 target、格式、行数 ratchet、
diff 检查通过，未发现测试进程残留；本增量未重跑完整默认测试集。

后台调度准备：`live_recovery_async.rs` 为 Client 提供 Starting/Idle/Running/Closed
单任务生命周期，操作期间才创建 worker，空闲没有线程或请求队列。同一时刻只允许
一项操作，响应未取走前不能再提交。UI 可借用任务完成句柄；空闲可借用子进程退出
句柄，poll 只 join 已结束线程。请求成功或失败都交回 Client；线程创建失败通过共享
单槽取回原 Client，不因闭包丢弃而失去通信 owner。启动失败关闭入口；请求 panic
由 unwind 边界接住，保留 Client 但封闭通信流，不自动重启或重放命令。正常关闭
必须保留 owner 并泵消息至非 busy 后释放；异常 Drop
仍 join 防止脱离线程，因此可能等待当前 I/O，不承诺异常关闭无阻塞。
此模块尚未接 Application 等待集合与源可见性状态机，外部源隐藏仍未启用。
本增量定向测试 3 项通过：真实子进程生命周期/拒绝排队、启动失败单次交付、本地
非法请求后保留可用 Client。Clippy 全 target、格式、行数 ratchet、diff 检查通过；
未重跑完整默认测试集，未把异常 Drop 的 join 当作非阻塞 UI 关闭验证。

源隐藏写入后端增量：`live_window_hide.rs` 与 `Lease::hide` 已实现已 arm 才可隐藏，
使用主干的 LAYERED/TRANSPARENT、alpha=1 与非激活置底机制，不调用 SW_HIDE。
最小化源的恢复尝试位于登记之后，没有固定 sleep，但 Win32 同步调用不承诺硬实时。
共享 peer 重复隐藏不改首次快照；退休待定拒绝新隐藏；部分写失败尝试恢复，失败记录
继续保留。Application 尚未接真实 ACK 到 arm/hide 的调度，不启用用户源隐藏。
本增量共享可见性定向测试 8 项通过；三进程运行门禁改用该隐藏后端，正常返回和
process::exit 两种父退出路径均通过，仍只操作自有隐藏窗口，不代表可见/最小化用户
应用兼容已验收。Clippy 全 target 检查通过；未重跑完整默认测试集。

恢复通信准备：`live_recovery_wire.rs` 已定义 Register/Retire/Shutdown 请求和 ACK，绑定
非零 session 标识、单调请求序号及登记 id。四字节长度前缀，单帧最多 4096 字节，
拒绝未知字段、非法 schema/身份/几何/placement/layered 组合；半包 EOF 与正常 EOF
区分，ACK 必须匹配 session 和请求序号。源记录包含进程创建时间，但此层只校验结构，
不验证进程身份或授权，也不执行恢复、不启动子进程，不能把 codec 通过当作恢复链路已就绪。

`live_recovery_service.rs` 现有有界登记状态机：单请求在途、严格连续序号，只缓存最近一次
完整请求及 ACK；原样重发不再次绑定，改写重试/跨 session/跳号拒绝。最多 16 条记录，
登记 id 单调且不复用，已登记 HWND 不覆盖首次快照；迟到退休不能删除新 id。绑定失败
不增加恢复责任，拒绝结果也可重放。Shutdown 仅在记录全部退休后接受。可信宿主进入
终止恢复后，成功记录释放、失败记录及绑定 owner 留下重试，禁止后续新命令。
状态机测试用注入绑定/恢复回调验证协议和 owner 析构次数；恢复子进程尚未接入。

真实源绑定增量：`live_window_target::recovery::Binding` 将已认证通道的记录绑定原
Target，登记时核对 HWND/PID/TID 和 GetProcessTimes 创建时间，并保活原进程同步句柄。
`Snapshot::recovery_record` 查询创建时间前后校验原 Target，查询权限单独申请，不扩大
普通 Live Target 的权限要求。恢复使用登记时的 Target，不重新按 PID 选源。
自有隐藏 HWND 测试覆盖快照往返、错误 PID/TID/创建时间拒绝、销毁后拒绝，以及真实
Binding 接 Service 后先登记 ACK 再改变夹具样式、终止恢复还原。它不是来源认证：同一
进程/线程内另一个 HWND 的授权仍取决于可信父通道，同一存活 PID/TID 内 HWND 快速复用
也仍存在竞态。实际跨进程恢复与父通道认证未验收。

后续私有管道实现必须保留旧 watchdog 的真实直接父进程、同 executable、pipe creator
身份校验与有界握手；session nonce 不能替代认证。子进程在登记时保活经验证的原源
进程 handle，不能等父死后只凭 PID/HWND 重绑定。EOF 不等于父死，仍需持有原父进程
handle 并协调管道失败后的禁写状态；部分坏帧不能丢弃此前已确认的恢复责任。
Register ACK 前不隐藏，Retire ACK 丢失按结果未知处理，重试必须幂等且不能删除新代次。

共享源 owner 增量：`live_window_target::visibility` 现有 UI 线程 Registry/Lease，最多
16 个源、每源 16 个 lease。按原 Target 身份共享首次恢复快照；普通 peer 退出不恢复，
最后退出兜底恢复。显式召回恢复共享源但不减少 lease；重复隐藏不能覆盖原始快照。
恢复失败保留 registry 条目供重试；窗口恢复成功后还需恢复记录退休回调成功才清空状态。
退休失败只重试退休，不重复写回窗口覆盖用户后续修改，期间拒绝新的隐藏/准入。
该模块不建立 journal，不执行隐藏，arm 要求调用方事先完成独立恢复登记；测试使用
自有隐藏 HWND 和注入退休回调。实际 journal、watchdog 及宿主接线仍待实现。

输出窗口暂停并不等于主干的源窗口逻辑隐藏。下一阶段按以下依赖顺序迁移，不能以
`SW_HIDE`、真正最小化或只实现正常 Drop 恢复替代：

1. 迁移显式 shell 召回意图状态机：短期有效、单次消费；普通前台变化、鼠标移动或
   浏览器迟到激活不构成召回授权。参考 `src-tauri/src/native/live/source/live_source_activation.rs`。
2. 围绕 native Target 建立原始样式/layered 属性、位置、placement 和 topmost 快照；
   共享可见性 lease 按源身份管理，首次隐藏只拍一次快照，普通消费者退出不能提前恢复。
3. 建立有界、校验身份的独立进程崩溃恢复通道；必须在变更外部窗口前登记成功。
   正常析构不是崩溃恢复，不能把依赖项留到隐藏功能启用之后。
4. 接通隐藏事务与完整恢复：保留 shell entry，失败回滚；主动召回恢复所有 peer 的
   可见性但不中止采集，最后 lease 退出才执行停止兜底恢复。
5. 最后接宿主 shell intent/源状态事件、输入释放及会话生命周期，并验证独立恢复进程。

合同依据为旧实现 `live_source_window.rs`、`live_source_visibility.rs`、
`live_source_recovery.rs` 和 `emergency_watchdog.rs`；native 目前没有等价的外部源
逻辑隐藏或 watchdog journal；visibility lease 基础模块已补充，尚未接宿主。已有帧
Consumer 计数不能代替 visibility lease。
以上是完整迁移合同；下述底层增量不能代替尚未接通的用户功能和独立恢复验收。

召回状态机已新增到 `native/src/live_reclaim.rs`：保持 2000 ms 短期意图、隐藏前手势
吸收、显式激活及前台透明源任务栏最小化切换的单次召回。序号与单调时钟分离，支持
同毫秒多次手势；未来时间、过期、取消、旧序号、空目标和序号耗尽保守拒绝。该纯逻辑
模块没有全局状态、锁、分配或 OS 操作；输入 provenance 仍由未来宿主验证，不将前台
变化本身当作授权。尚未接输入 hook、visibility lease 或外部窗口恢复，不能声明该功能已可用。

恢复快照读取已新增 `live_window_target::snapshot::Snapshot::read`：保留原始矩形、
WINDOWPLACEMENT、iconic、完整扩展样式和 layered color/alpha/flags。读取前后校验原
Target；零扩展样式合法，Win32 错误与零值区分。无法读取 layered 属性时拒绝快照，
不以默认不透明值替代。普通测试仅创建自有隐藏 HWND，验证属性不变和销毁后拒绝。
快照不是跨进程原子查询，隐藏入口和崩溃恢复仍未启用。

`live_window_restore.rs` 已新增显式 `Snapshot::restore`：恢复完整扩展样式、layered 属性、
topmost 分类、位置尺寸和 placement；各写入阶段复核原 Target，非法尺寸/placement/
属性组合在写入前拒绝。错误不消耗快照、不删除恢复记录，供未来 owner 保留并处理。
实际自有 HWND 回归发现隐藏窗口的 placement.showCmd 不一定代表隐藏状态，故快照另存
visible，原本隐藏的窗口恢复仍保持隐藏；原本可见但最小化的源沿用主干展开恢复语义。
普通回归覆盖非 layered/layered、非 topmost/topmost 四种组合、重复恢复、几何与透明度
还原，以及非法快照/已销毁目标拒绝。这里只操作自建隐藏窗口；没有启用外部源隐藏事务，
跨进程阻塞/失败、真实最小化及崩溃后恢复仍需后续验收。

## Live 输出资源与实际最小化验证

Live 输出资源增量：Host 在零尺寸或输出窗口 SIZE_MINIMIZED 时释放呈现链及导入位图，
只保留最新共享帧和订阅；恢复时惰性重建，即使源没有新帧也能恢复画面。WM_PAINT 另查
IsIconic，避免最小化窗口的非零 client rect 引起重建。WARP 回归覆盖重复暂停、旧帧
释放、新帧优先、无新帧恢复、暂停后关闭，以及隐藏 HWND 的最小化消息接线。
此项不改变源应用可见性、不停止采集，不代表源最小化/逻辑隐藏策略或实际显存降幅已验收。

实际输出窗口补充：显式 ignored 测试以 WARP 合成帧创建自己的可见 Live 窗口，只发布
一次画面，连续三次真实最小化/恢复。已验证最小化立即释放 surface、此时强制 paint
也不重建、恢复无需新帧即可重建，以及最小化期间源关闭会清理窗口和订阅。没有捕获
桌面或修改外部窗口。此证据验证应用侧 owner，不测量驱动的实际显存回收时机。

```powershell
cargo test --locked --manifest-path native/Cargo.toml --lib actual_minimize_releases_surface_and_restore_reuses_static_frame -- --ignored --nocapture --test-threads=1
```

通过回执：`native-live-output-runtime: minimize=true no-hidden-surface=true static-restore=true cycles=3 close=true`。

## 目标与开发边界

- 开发分支：`feat/windows-native`。
- 固定功能基线：`2dfc73d107429c38e25a0c9e0a9fd062d99ab306`，不使用不断移动的 main 作为唯一对照。
- 最终产品：Windows 专用 Rust 应用，Win32 宿主与原生画布；不依赖 Qt、Tauri 或 WebView 呈现界面。
- 保持基线已有功能、用户数据和外部协议；低消耗不能通过删功能、降低安全边界或隐藏进程成本实现。
- 所有迁移开发留在本分支。合并 main 由用户在测试通过后手动执行；不自动合并、发布或推送。
- 本文件是进行中的开发计划，不是实现完成证明。已建立原生图片显示入口，完整功能迁移仍进行中。

## 架构与所有权

N07 HWND 帧通知增量：消费者可绑定非阻塞唤醒，未读取期间合并通知；读取与重新
允许通知处于同一源锁范围，回调在锁外，热路径使用固定数组而非增长消息队列。
WindowSignal 绑定当前进程/线程的 HWND，消息仅携带连接 token，拒绝旧连接消息。
投递失败释放本次预留并累计失败次数，下一帧可重试；没有额外等待线程或定时轮询。
真实隐藏窗口消息测试覆盖 100 帧合并、关闭通知、旧代次拒绝和已销毁 HWND 失败，
纯回归覆盖重入、失败重试、旧失败不覆盖新投递及退休槽复用。尚未接宿主可见 UI；
系统消息队列满后若再无帧变化，终止通知的宿主补偿处理仍待接线，不能声称可靠必达。

N07 呈现控制器增量：`composition::live::host::Host` 持有订阅、一个最新待绘帧和
surface，合并通知后只在 paint 中导入与提交；零尺寸不分配呈现链。已有绘制或其他
宿主事件可 refresh 补读遗漏消息，源关闭及渲染错误立即解除订阅并释放 GPU owner，
不忙重试。隐藏 HWND/WARP 回归验证消息到实际呈现、替换帧释放、关闭及无效尺寸失败；
应用入口及 worker 完成回收现已接入，见下述应用增量；仍未验证实际桌面捕获。
不可将此控制器单测当作 N07 用户功能完成。

N07 专用窗口增量：`LiveWindow` 已使用真实 WndProc 将通知、WM_PAINT、WM_SIZE 与关闭
接入 Host；窗口创建后先设置系统捕获排除，成功后才绑定消费者。源关闭或呈现错误会
释放订阅并销毁窗口，错误由 owner 单次取回；不发送 WM_QUIT。状态固定地址，owner 搬移
不改变回调指针；Drop 复核 HWND 身份并解除指针。隐藏窗口/WARP 回归覆盖真实消息分发、
捕获排除读回、关闭补读、WM_CLOSE、owner 搬移与不同设备导致的错误退出。窗口可显式 show，
重入期间收到的通知通过延后失效区补绘，独立回归验证消息不丢失且无 RefCell 重入 panic；
应用入口、选择器授权及 LiveJob 回收现已接线，没有进行可见窗口或真实桌面捕获验收。

N07 会话 owner 增量：Session 配对 LiveWindow/LiveJob。`start_region_bound` 同步执行
窗口绑定后才创建线程，保证捕获排除失败不会启动 WGC。stop 释放窗口并取消采集，但不
在调用中 join；poll 只在线程已结束时回收并单次交付结果。源失败引起窗口关闭不再追加
取消，避免覆盖后台原始错误。CaptureJob/LiveJob 提供借用真实线程句柄，供 UI 合并消息
等待；回归用合成 worker 验证源关闭后线程仍未完成、最终句柄就绪、原始错误及配额回收。
绑定失败/调用线程/先后顺序另有回归。没有增加线程或定时器来等待完成；应用现已持有
Session 并将句柄纳入消息循环，WGC 自建窗口验证见上，完整运行验收仍未完成。

N07 应用入口增量：托盘“区域 Live”复用独占门禁、隐藏应用窗口与冻结选区，只有确认
且单显示器物理 RECT 校验成功才排队，取消不改文档。Application 在回调外启动并持有
最多 16 个 Session，窗口先捕获排除再启动 WGC worker。每次消息后补读/结算，等待阶段
使用 MsgWaitForMultipleObjectsEx 合并真实线程、源进程退出和 UI 消息；无 Live 时仍为 GetMessage。
线程完成返回合成 WM_NULL，避免末次关闭通知早到/失败导致回收依赖下一次用户输入；
应用退出先停止所有会话再回收，不 detach。隐藏窗口/合成 worker 回归验证实际应用回收、
失败传播与无效请求消耗；等待回归覆盖无投递消息时线程退出唤醒和 WM_QUIT 退出码。
此入口当前是区域 Live BGRA 呈现，不是 HWND 跟随捕获、投射输入、HDR Live 或完整 N07。
真实托盘点击、选区、WGC 画面、跨屏/DPI、长时间运行与资源占用仍待验收。

N07 GPU 呈现底层增量：`Composition::with_device` 允许复用捕获设备建立合成链，
`composition::live::Surface` 直接从 BGRA8 DXGI 纹理建立 D2D 位图并保留完整共享帧。
同帧重复绘制复用位图，后续帧须同设备且元数据匹配；失败保留旧帧。按窗口尺寸等比
居中，透明背景/预乘 alpha，不做 CPU 回读或图片编码。线程 marker 约束单线程 D2D
owner，导入前要求 D3D 多线程保护。隐藏 HWND/WARP 像素与资源测试覆盖该链；宿主帧事件、
Live 窗口及区域确认已接入，但缺少真实捕获验收，不能视为完整可用 Live UI。

N07 区域 Live 后端增量：后台任务可接受桌面物理矩形，复用已有单显示器绑定，GPU
只复制区域到独立快照。复制前后复核显示器身份/位置/尺寸/设备名，变化拒绝交付并
结束会话；跨屏、越界、空区域和坐标溢出拒绝，不隐式裁剪。源 WGC 帧池仍为整屏，
不能把区域快照变小宣称为整屏捕获开销消失。WARP 不同像素区域回归与过期目标
候选释放测试已覆盖此层；区域选择和呈现已接线，混合 DPI、布局切换和真实捕获仍待验收。

N07 窗口 Live 后端增量：Target 在选择时绑定顶层 HWND/PID/TID 与原进程同步句柄；
子 HWND 不隐式提升为父窗口，位置/标题变化不重新授权。LiveJob::start_window_bound
先验证并在 UI 线程绑定呈现窗口，然后 worker 调用 CreateForWindow，GPU 复制前后
持续验证原目标，不转换成固定屏幕矩形。共享 Session 复用池、预算、事件与释放路径，
呈现 Session::start_window 已接该入口。隐藏 HWND 回归覆盖移动、跨线程传递、关闭、
身份改变、子窗口拒绝、无消费者及绑定失败不启动捕获。没有运行有效 WGC 窗口捕获；
窗口选择菜单已接入，最小化/隐藏完整策略仍待完成，尺寸重建已接入下述事务。同一存活 PID/TID 内的 HWND
快速复用存在启动竞态，不能将此身份检查当作唯一 HWND 代次证明。

N07 窗口入口增量：托盘“窗口 Live”使用原生菜单选择已绑定 Target，最多 64 个外部
可见、非最小化、非 cloaked 顶层窗口，跳过当前 Otter 进程；绑定失败不列入候选。
标题固定缓冲读取、显示长度有界、&/控制字符清理，并附 PID/HWND 区分同名来源；标题
只用于此次菜单。选择后再验证，取消或目标过期不启动采集。Region/Window 共用一个
类型化请求槽，由 Application 在回调外启动，沿用 16 会话预算和完成句柄等待。
真实 HMENU/隐藏窗口回归覆盖菜单条目、取消/非法索引/过期目标拒绝、标题边界；应用
回归覆盖已关闭窗口请求消耗且不启动 WGC。自动化没有枚举真实用户窗口或读取其标题，
也未运行可见选择与真实窗口捕获；最小化/隐藏策略、HDR Live、投射输入及手测仍待完成。

N07 窗口尺寸增量：窗口源 ContentSize 改变时先归还旧帧，预留新旧双帧池的保守峰值
预算，然后在同一 owner 线程 Recreate，成功后提交尺寸/新租约并等待新尺寸帧。失败
关闭会话并释放预算，不尝试继续使用可能失效的池。零内容暂不交付，保持最后一帧，
等待后续事件；负数/超限拒绝。显示器和固定区域仍保持尺寸变化终止，避免自动扩大
授权边界。事务测试覆盖峰值准入、失败不提交和租约释放；真实 WARP 帧池覆盖多次
缩放重建（不创建捕获会话），GPU 呈现测试覆盖同设备不同帧尺寸。真实 WGC 窗口拖动
缩放、最小化/恢复与资源曲线仍待验收，不能把无采集的池测试当作端到端通过。

N07 后台任务增量：`LiveJob<Texture>::start_monitor` 在专用工作线程创建并驱动持续
WGC owner，复用现有 `CaptureJob` 的结果/取消/join；启动返回初始消费者并支持追加
共享订阅。并发任务上限 16，超过上限不创建线程。取消关闭交付源以唤醒等帧线程，
最后消费者退出可结束任务；失败/panic/退出释放名额并关闭源。真实线程合成回归验证
交付、共享、取消、错误、panic、Drop join 和名额复用，未调用有效显示器捕获。
宿主托盘/区域目标及原生呈现已接入，HWND 跟随目标与真实运行验收尚未完成。

N07 持续 WGC 后端增量：新 `capture_wgc::live::Session` 复用一个设备与两帧 BGRA 池，
FrameArrived 只发送容量 1 通知，owner 线程阻塞等待后最多取两帧并交付最新 GPU 快照。
取消/源关闭/最后消费者退出都会唤醒；终止错误关闭会话与帧池并清空交付源，退出撤销
事件 token。窗口源尺寸变化按预算重建，显示器/区域仍明确终止；owner 不可跨线程移动，保留系统捕获
边框且不捕获光标。通知合并、静止等待取消、无需求准入及两槽消费有合成测试，真实
WGC 自建窗口会话现已显式运行；区域启动入口与呈现已接线，真实用户窗口与显示器场景仍待验收。

N07 Live 帧交付底层增量：每个源保留一个最新共享帧，最多 16 个消费者独立读取，慢
消费者跳过旧帧而不积压。关闭唤醒阻塞读取；最后一个消费者退出释放源的帧引用，
已经借出的帧随消费者持有期保留。GPU 适配器先复制独立 BGRA8/FP16 纹理再交付，
不把可复用的捕获池纹理直接发布，也不在生产路径做 CPU 回读/JPEG 编码；按需求准入
且预算随共享帧存活。持续 WGC 后端、区域选择与呈现已接；Live 窗口恢复未接入，不能
据此认定 Live 功能可用。合成 WARP 测试不采集桌面；跨设备同步和实际显存开销未验。

自启动迁移增量：原生托盘已有当前用户登录启动开关，保留主干任务管理器禁用语义。
按需查询，不在程序启动时写注册表；显式点击复核菜单状态并写入当前 EXE 命令，其他
安装的 Otter 启动项不自动接管。命令有引号、绝对路径和长度校验，注册表读取有界，
部分失败保留实际状态并报告。测试仅用内存替身，没有更改本机启动项；真实登录流程
仍待人工验收。仅启用既有 windows 0.60.0 的 Registry feature，未增加依赖包或升级版本。

托盘常用入口增量：区域截图、垂直/水平长截图和打开图片接入现有新钉图流程，不覆盖
当前文档。菜单呈现和执行分别检查准入；先恢复宿主，再复核窗口/任务/模态状态，避免
恢复消息重入后执行过期操作。测试用真实菜单和替代执行回调，不操作实际托盘或采集
桌面；Live、接收投射入口及可见体验尚未据此完成，自启动实现见上。

### HDR 原图会话恢复验证检查点（2026-10-19）

原生会话中的内联 HDR PNG 已接入恢复候选和文档原图 owner，可在恢复后再次保存，
且保留原始 PNG 字节。恢复逐行校验 RGB16 / BT.2020 PQ、CRC 和文件终点，拒绝
外部路径、截断、尾随数据及 SDR 内容；私有临时文件在最后一个 owner 释放后删除。
本检查点重新执行的原生测试共 1324 项、11 个套件全部通过；Clippy（禁止 warning）、
格式检查、有效行数 ratchet 和 `git diff --check` 均通过。行数检查覆盖 2348 个文件，
没有超过 500 有效行的文件。验证使用合成图像和隔离临时目录，没有实际采集桌面。

这些结果只证明当前源码的自动化门通过，不代表整个迁移完成，也不替代真实 HDR
显示器、可见会话保存/恢复、外部 HDR 查看器和最终便携包的人工验收。

后续主干对照修正：Auto 在 HDR 源采集成功后选择同帧 SDR 输出时，保留 `wgc` 后端，
并将 `downgradedFromHdr` 设为 true，与主干 dispatch 一致。此前原生实现错误地标为
false。该路径不会再次采集或生成 HDR 原图；合成帧测试覆盖动态范围选择、交付和
会话元数据，真实显示器行为仍需人工验收。

智能图片/文件复制的协议发布层新增 CF_HDROP 编码：宽字符路径、双 NUL 终止及有界
绝对路径准入，不读取磁盘。PNG/DIBV5/可选文件列表全部内存准备成功后才一次清空发布；
失败前不触碰系统剪贴板，清空后的部分失败不能回滚。合成协议往返及实际 HGLOBAL
预分配回归不访问系统剪贴板。生产复制任务现冻结剪贴板命名模板和隔离缓存目录，后台
编码一次并暂存同一 PNG；主线程复核文档/取消/窗口后提交缓存，再一次交付三格式。
隐藏 HWND 回归验证完整文件生成、发布回调字节、递增、单任务及取消/过期拒绝，系统
剪贴板副作用由测试回调替换。资源管理器及图片编辑器的真实粘贴仍待人工验收。
异步复制顺序新增两层保护：应用内跨窗口新复制取消旧请求，旧 worker/暂存仍按原owner
回收；生产入口冻结系统剪贴板序号，结果完成和持剪贴板锁后分别核对，变化时不覆盖。
后一次核对覆盖缓存发布/内存准备间的外部修改；若此前已发布缓存则保留该文件供后续
安全清理，不按路径撤销。后台缓存锁等待最多1秒、每10 ms检查取消，不增加空闲任务。
隐藏双窗口测试验证新请求优先、取消不复活旧请求、旧暂存回收后共享缓存可继续写入，
注入序号验证外部变化拒绝；未操作真实系统剪贴板。Windows 延迟渲染/序号回绕及真实
应用间竞争仍需人工验收，不把应用内仲裁宣称为跨进程全局用户意图排序。
缓存暂存/发布底层已补齐：使用隔离 profile 下 clipboard_cache，合作进程排他锁覆盖
有界扫描、暂存和发布；默认256 MiB及4096项上限，超限拒绝而不删未知内容。暂存时不暴露最终
文件名，取消/Drop 回收暂存；请求复核后的发布复用同一文件无覆盖递增重命名，成功后
文件不随 owner 析构。真实隔离文件回归验证竞争锁、取消/回收、递增和旧内容保留、容量
拒绝；会话绑定候选不能走资源递增发布。生产任务与命名已接线，默认保留期清理见下文；
缓存满时明确失败，不擅删文件或静默降级。原有隐藏 smoke 显式注入无文件模式隔离环境。
新发布缓存现附带最多2 KiB的独立所有权记录：文件名、摘要、字节数、创建时间和卷/文件
索引；容量与项数在暂存前为记录留出余量。取消/未发布不会生成记录；记录失败保留已发布
PNG并报告路径，旧文件不补写记录。回归验证摘要/文件身份绑定与外部同名文件不被认领。
记录用于保守清理核验，不是恶意本地修改的安全边界。后台暂存前持排他锁执行默认7天
清理，单次最多4096目录项和512 MiB哈希读取。仅身份、大小、摘要匹配的过期普通单链接
文件可通过已锁定句柄退休；无记录、未知版本、路径跳转、修改/替换、硬链接和占用均保留。
隔离测试覆盖实际删除、旧文件保留、取消与时钟回拨；没有运行真实用户缓存清理。
保留期和软配额已从启动配置cache字段读取，默认256 MiB/7天，0为关闭相应策略，范围
对齐主干32 MiB–16 GiB及0–3650天；无效配置在建窗/截图前拒绝，不静默默认。策略在
新建/恢复窗口共享并随复制任务冻结。命名导入不隐式切换缓存策略；“文件与缓存设置”
保存会合并当前两类偏好，保留cache未知扩展和回收站字段。
真实EXE合成配置验证非默认策略与恢复，文件测试验证0/30/7天和容量关闭/启用。
容量压力淘汰已接入：仅在现存总量加输出预留超配额时，按记录创建时间从旧到新核验，
目标降至配额一半且保证本次输出余量。完整记录再次匹配后才通过共用句柄核验退休，
未知/变更/占用内容保留；与保留期清理共享512 MiB读取预算。零配额禁用，单次输出超限
不触发任何清理。隔离真实暂存测试验证排序、半配额目标和外部内容保留；释放不足仍报错。
“临时缓存设置…”现有原生容量字节/保留天数输入，精确保留非整MiB配置；确认仅改内存，
由后续复制消费，不当场清理。取消、非法值、模态期间变化及关窗均不覆盖当前策略。
保存/另存/自动保存已将缓存纳入双阶段快照校验；仅缓存变化也保存并参与退出等待。
专项验证未知字段保留、自动保存一次、文件选择和后台阶段变更拒绝；实际对话框交互与
高DPI仍待人工验收。超过保留期或被容量淘汰的缓存引用不保证永久可用。

N05 长截图增量：已核对主干 `src-tauri/src/long_capture/`，新增原生已定位帧拼接底层，支持
水平/垂直、反向扩展、包含和重复覆盖，已有区域保留较早帧，使用片段计划避免每帧
重分配整个长图。单帧/重复结果共享原像素；一般结果一次分配并按行复制，取消释放输出。
准入对齐主干 512 帧、32768 单帧轴、64 Mi 单帧像素、128 Mi 输入/输出像素与 131072
输出轴，另受原生全局共享像素预算限制。测试不采集桌面、不滚动窗口；重叠分析、固定
栏、自动滚动、预览和 UI 尚未接入，本阶段不是可用的完整长截图功能。
后续增量已添加等尺寸帧精确重叠快速路径：双轴、双向线签名/KMP 搜索，唯一候选
逐像素复核；重复、歧义、无重叠分别返回。不会把重复纹理强制解释成某个滚动距离。
精确位移已通过合成链测试，近似匹配、固定栏与实际捕获循环仍待实现。
近似分析后续增量：已有双轴、双向及零位移采样评分，搜索范围/最低重叠/误差上限
由明确选项约束。16×16 搜索后以 128×192 复核唯一候选，低纹理和接近候选不猜测。
轻微像素变化与粗采样间动态变化有合成测试；这不是主干完整分析器的等价证明，
固定栏、动态区域权重、采集会话和真实内容场景仍需继续迁移与验证。
固定边缘增量：已有两帧轴向首尾静止区域检测，复用原像素内部区间评分，不复制
裁剪缓冲。全帧相同不当作裁边，内容不足/取消/分析失败不报告有效边缘；非零运动
成立后返回边缘长度。合成入口已支持固定首尾各输出一次，逻辑总量准入后逐帧复核
固定栏，不一致拒绝，不复制中间裁剪图；双轴正反、每个输出像素与 origin 溢出有测试。
尚缺跨帧确认及浮动/半透明/变化栏支持，不能据此勾选完整固定栏处理。
会话增量已串联精确优先及近似/固定栏分析、累计位移和最终合成。固定栏需至少两次
非零运动观察到一致边界，后续每帧与首帧复核；确认前拒绝裁边合成。重复不增加帧数，
拒绝/取消/超限不改变已接受状态。已有往返、重复、边缘变化、取消和单帧共享测试；
自动滚动、捕获线程、预览/UI 及复杂固定栏仍未接入，此项不表示 N05 完成。
后台任务增量：SessionJob 已拥有单 worker 和容量 1 的命令/回复通道，消费回复之前
拒绝新命令，追加分析和最终合成均在后台。取消关闭发布通道，不阻塞等待；最终 Drop
join 回收线程。专项测试验证 Busy、失败后恢复、后台合成及取消丢弃已完成结果；
此层没有真实截图、滚动输入或 UI 接线，不能作为实际桌面长截图验收。
选区采集增量：SessionJob 已支持后台显式 capture 命令，采集错误可重试且不推进
会话；原生适配器冻结物理矩形与首帧，复用区域 GDI 捕获，避免全屏中间图。GDI
前后检查取消，保留已有 DIB/像素双预算和作用域句柄回收。测试用合成源验证线程、
命令准入及失败恢复，没有读取真实屏幕；用户入口、滚动控制、布局变化和预览仍待实现。
手动入口增量：菜单已有垂直/水平长截图，区域选择后进入原生控制对话框，可手动
滚动目标再采集，完成为新钉图。复用隐藏/模态/文档戳保护；控制窗口捕获排除失败
终止，显示布局变化取消。隐藏对话框验证合成源采集/完成和布局变化，无真实截图。
自动滚动、图像预览和真实跨应用/混合 DPI 体验未验证，不能按完整长截图结项。
预览增量：控制窗口已有显式预览命令和按需绘制区域。worker 从片段计划直接生成
最长边 384 像素缩略图，不分配完整长图；原分辨率最终输出不变。双轴反向片段采样
与最终图采样一致，隐藏窗口验证预览不关闭/交付；自动滚动与可见体验仍待完成。
滚轮底层增量：已实现选区目标窗口绑定与每次发送前身份/根窗口/几何复核，单步
垂直或水平、正反滚轮，150 ms 超时，不广播/抢焦点/移动指针。消息坐标超出有符号
16 位时拒绝，不截断；同线程发送拒绝。测试用注入回调，未向实际应用发送消息。
依据 [WM_MOUSEWHEEL](https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-mousewheel)、
[WM_MOUSEHWHEEL](https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-mousehwheel) 和
[SendMessageTimeoutW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendmessagetimeoutw)。
尚未接入自动循环和界面，也不保证所有应用响应窗口滚轮消息，仍须真实兼容性验收。
单步接线增量：控制窗口已有“滚动并采集”，后台按 CaptureMode 调用动作源，复核
目标、发送一格、等待 250 ms、再次复核、采集并分析。等待按 10 ms 片段检查取消，
不承诺动画已经稳定；每会话最多 511 次发送尝试。绑定失败自动保留手动源并禁用
滚动按钮；没有失败自动重试。隐藏窗口及 worker 测试注入动作源，没有真实输入。
连续自动循环、稳定检测、真实应用兼容性与可见体验仍待完成。
连续采集增量：已有自动采集/停止按钮及单在途串行循环，Added 后继续，重复、失败
或 511 步/300 秒调度上限停止；滚动前检查逻辑输入与帧数余量。滚动等待后最多五次
采样、连续两次全像素一致才接受，采样间隔 60 ms，未稳定返回失败且不重发输入。
停止保留当前任务收尾结果和已接受会话，不自动完成/保存。纯状态、采样替身和隐藏
UI 回归覆盖边界、重复停止和用户停止；没有实际滚轮或截图，真实兼容性仍未验收。
自动预览增量：已确认边缘的新增帧先生成缩略图，再继续滚动，仍保持单在途；下一
采集期间保留最近有效预览并显示帧数。未确认固定栏跳过预览，避免早停；用户停止
后即使预览完成也不能继续滚动。隐藏集成测试覆盖普通内容和固定栏确认后预览，
没有真实输入/桌面采集，可见效果及真实应用兼容性仍需验收。
内存准入增量：追加前检查保留输入与最终输出的同时驻留预算，失败/取消不提交会话状态。
滚动前按最大允许位移检查下一帧与输出，并单独检查稳定采样三份帧的峰值余量。
检查只取得短租约、不实际分配长图，不承诺后续竞争下必然成功；最终合成仍重新准入。
回归通过局部小预算模拟不足、取消与重试，不依赖耗尽全局内存或真实桌面操作。
方向接线增量：控制窗口可切换垂直向上/向下、水平向左/向右，单步和自动均使用该方向。
在途/自动期间禁用切换并拒绝直接投递的切换命令；空闲切换不重建会话或丢弃已有内容。
隐藏 UI 回归在两个轴上验证反向单步、反向自动重复停止、预览尺寸及方向锁定；未发送真实输入。
二维覆盖增量：对照主干 spatial_capture/spatial_coverage，新增已定位二维 BGRA 帧合成。
负坐标/四向扩展保留早到像素，空洞不算采集内容；导出选择最大完整覆盖矩形，同面积
优先靠上、靠左。64 MP 画布限制、共享预算和逐行取消生效，裁剪原地压紧且按 capacity
保留租约计费。小图穷举覆盖掩码与独立矩形枚举比较；取景框移动、位置匹配、会话切换
及 UI 尚未接入，不能据此宣称二维长截图已可使用。
二维位置验证增量：已知物理位移的重叠区域逐像素检查 RGB，必要时分析剩余单轴滚动，
返回合计位移或明确拒绝；沿用纹理/歧义/误差准入。只读区域视图复用原帧 stride，固定栏
和现有近似分析也使用该借用路径，无额外裁剪图分配。测试覆盖四种对角方向、双轴正负
残差、零位移、极端坐标、取消、低纹理和周期歧义；仍待会话、worker、取景框 UI 接线。
二维会话增量：SpatialSession 已连接位置验证、累计坐标、输出余量准入和完整矩形导出。
失败/取消不推进最后接受帧；重复不增长帧数。预览最长边 384，直接采样共享帧且保留
透明空洞，不先分配完整二维画布。纯数据回归验证位置累计、覆盖像素、拒绝/预算/取消
后的原状态与重试；仍缺一维转二维会话衔接、worker 命令和取景框 UI，未进行桌面验收。
一维转换增量：已支持非消费式生成二维候选，保留聚合内容、最后帧原始匹配参考、累计
帧数/输入量和固定栏坐标。宽长种子不受单次截图轴长限制，但仍受二维输出限制；测试
覆盖双轴正负来源、固定边缘、未确认拒绝、取消保留和超过 32768 轴长后继续追加。
worker 应仅在移动帧也接受后替换旧会话；该命令与取景框 UI 尚未接线。
后台切换增量：AppendAt 已连接独占活动会话；仅候选追加 Added 才替换旧一维会话，
失败/拒绝/重复不切换。快照携带二维位置，普通采集、预览、完成与取消使用原 worker。
二维滚动继续走唯一精确匹配快速路径，避免低纹理精确位移退化。worker 和隐藏对话框
回归验证拒绝保留、成功后继续采集、二维预览/完成和单在途/取消；尚无真实取景框移动入口。
采集源事务增量：CaptureSource/MoveCapture 已提供准备、接受结果提交、失败回退协议。
重复帧可确认物理参考但不必切换模式；取消强制回退并关闭结果发布。回归注入准备失败、
匹配拒绝、成功后继续采集和准备后取消，核对源参考/候选清理；旧闭包源不宣称移动能力。
当前尚未由真实桌面源实现 prepare_move，也未新增用户移动入口。
桌面源增量：手动与已绑定滚动源均已实现候选区域稳定采集及提交/回退。移动要求保留
重叠、坐标无溢出且在虚拟桌面范围内；已绑定源必须留在同一目标窗口，不重新授权其它
窗口。每次采样及返回前复核身份，失败/取消不留下候选；移动不发送滚轮。注入帧测试
覆盖提交/回退、采集失败、尺寸变化、取消、负坐标/边界/溢出；未实际采集屏幕，UI 尚待接线。
移动 UI 增量：控制窗口已有取景框上/下/左/右按钮，步长为轴长五分之一（整像素）、
最多 64，桌面边缘裁减。仅空闲时分派，自动/在途消息拒绝；接受后提交 UI 物理参考并
刷新预览，失败不推进。隐藏窗口与合成源验证成功、失败、在途、自动禁用和能力禁用。
长截图菜单移除过期“手动滚动”限定。可见取景框边框、透明空洞显示及真实桌面验收仍待完成。
预览空洞增量：收到缩略图后将透明区域合成到棋盘格副本，保留真实黑色；GDI 不再将
空洞显示为截图中的黑色。只处理最长边 384 的预览，不透明路径不复制，副本计入预算；
准备失败停止自动循环。内存 DC 与隐藏对话框回归核对显示像素和导出隔离，可见边框及
真实桌面交互仍待验收。
取景框边框增量：四条窄顶层窗口位于截图矩形外，layered + transparent + noactivate，
逐窗请求 WDA_EXCLUDEFROMCAPTURE。创建/排除/定位失败结束本次会话，不继续显示不可靠
边框；接受移动后更新，跨显示器最多做一次 DPI 校正，关闭销毁。隐藏窗口测试不请求
真实捕获排除，验证样式、负坐标、多 DPI 几何、移动和句柄释放；可见效果尚未人工验收。
实现依据 Microsoft 的 [Layered Windows 鼠标穿透说明](https://learn.microsoft.com/en-us/windows/win32/winmsg/window-features)
与 [SetWindowDisplayAffinity 条件和限制](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowdisplayaffinity)；
捕获排除是受支持系统接口的行为，不是对所有捕获方式的安全保证。
跨窗口移动对齐增量：不再把取景框永久限制在初始滚动窗口；保留桌面边界和重叠检查，
跨出原目标或滚轮坐标范围时准备手动候选，接受后才撤销原滚动绑定。身份失效仍报错，
不自动授权新窗口。worker 回复携带实时能力，UI 同步禁用滚动并显示手动状态；隐藏
源回归验证动态撤销和继续移动，纯几何验证越界降级。真实跨窗口采集仍待验收。

会话 captureMeta 合同增量：对齐主干 StickerCaptureMeta 已有字段，最多读取 4096
字节元数据，backend 最多 256 字节且拒绝控制字符。缺省/历史部分字段保留 None，
未知扩展只在原始会话中保留，不从子集重写丢失；拒绝非法矩形、枚举、位深及明确
矛盾的 HDR/SDR 色彩组合。session_window.valid 纳入该校验，错误元数据不能悄悄
绕过会话准入。三项测试覆盖主干 HDR/长截图、历史部分值、容量/格式错误和整会话
读取保真。该步骤没有访问来源文件。

元数据保存/恢复增量：Placement 携带有界 captureMeta，恢复文档绑定来源记录；保存
快照冻结当前记录，组装时校验并替换原字段，无来源则移除旧值，避免替换图片后残留。
恢复记录再次保存时保留未知扩展；新截图写入动态范围/位深/色彩空间/降级标记。
仅有元数据时不会重开原图文件，租约为空；合法内联原图扩展现由独立恢复入口处理。
信息框显示 HDR 原图文件未恢复，缺失降级标记显示未记录。测试覆盖实际候选/绑定/
状态恢复链、扩展保留、保存写入和清除、非法快照拒绝。原图资产由下述内联保存/恢复
实现承接，不能仅凭元数据测试宣称完整可见会话操作已验收。

来源上下文增量：区域/全屏记录 kind=region 与原始物理取景矩形，明确附带原生
physical 坐标扩展；没有 origin 时不猜测。动态捕获按实际结果记录 wgc 或 gdi，
WGC 自动降为同帧 SDR 仍记录 wgc；失败后 GDI 回退记录 gdi。长截图记录 kind=long、
scrollAxis 和初始选区尺寸，不用拼接输出覆盖取景历史。全屏替换不再丢弃 origin。
纯数据测试验证负坐标/缺失语义，隐藏交付测试验证 3 像素拼接图仍记录 1 像素初始
取景宽度及方向；不访问真实桌面。

原图资产保存增量：UI 快照保留活动文件租约，后台从显式偏移 reader 流式生成 PNG
Data URL，写入 otterNativeCaptureOriginalSrc。复用同一 Base64 三字节编码循环，
48 KiB 输入块避免中途 padding；最终字符串计共享预算，不复制完整压缩 PNG。错误、
截断、超长输入和取消拒绝部分结果，完整会话仍受 64 MiB 限制。普通图片替换移除旧
原图扩展；尚未重建租约的已恢复 HDR 会话保留已有扩展。测试对照块边界/末尾 padding，
并实际保存 HDR PNG、核对会话中原图字节编码和替换清理。

原图资产恢复增量：会话只允许 HDR 元数据配套内联 PNG 原图扩展，不将路径/URL
作为回退。Base64 流式写入独占 create_new/delete-on-close 文件，逐行验证 RGB16、
BT.2020/PQ/full-range、非动画/非交错、尺寸预算、CRC 和文件尾；工作区预算 8 MiB
加行缓冲，不保留完整解码像素。Images 将 Arc<File> 移交恢复文档，原图另存和再次
保存复用现有句柄路径。错误/取消及最后 owner 释放回收文件。测试在 linshi 验证
字节保真、最后 owner 删除、CRC/截断/尾随内容/SDR/外部引用拒绝，实际恢复候选到
文档链验证原图句柄和再次编码一致；未操作真实桌面或用户临时目录。

HDR 活动来源租约增量：文档不再仅持原图路径。commit_leased 在缓存排他锁内发布并
获取只读文件句柄，拒绝写入/删除共享，随后释放全局锁；租约随交付进入文档来源。
关闭/替换释放，栅格同步也会清理身份失效的来源。真实隔离文件测试验证活动原图不能
写入/删除、过期清理跳过、其它截图仍可暂存、释放后清理恢复；文档测试验证跨后台
准备/新钉图移交后仍受保护，图片替换释放句柄。普通剪贴板发布不增加长期句柄。

HDR 缓存归属增量：stage_hdr 复用现有缓存配额、排他锁和 Prepared 发布，流式编码时
同步记录实际长度和 SHA-256，提交后写入同一格式的文件身份回执，供既有保守清理核验。
编码前按 PNG 保守上界准入，不等编码完成才申请容量，也不保留完整压缩文件副本。
临时目录测试核对回执长度/哈希、RGB16 回读、有效配额下的大图拒绝、取消、编码错误及
放弃暂存的清理。菜单与预览/原图交付仍待接入；缓存入口通过不代表真实 HDR 截图通过。

后台缓存交付增量：CaptureJob::start_cached 在同一任务内采集并编码 HDR 原图，返回
PreparedCapture 而不是提前发布文件；原 Outcome 任务仍供现有等待窗口使用。准备结果
保留同帧预览、降级原因和诊断，编码后释放 RGB16，避免同时持有两份长期像素。UI 接受
回调须先复核窗口/请求/文档代次，拒绝或迟到取消清理未发布缓存；成功提交返回预览和
原图路径，调用者必须继续保留原图来源。SDR 不创建缓存。合成帧与临时文件测试验证
预览零复制、188 灰度保留、原图释放、owner 拒绝和已完成任务取消。等待窗口现已增加
wait_cached 入口，共用同一泛型模态状态机，不复制取消/布局变化处理。请求参数和缓存
目的地在启动前快照；成功返回尚未发布的 PreparedCapture，窗口关闭则继续等待任务
退出并清理暂存。新增隐藏 HWND 测试验证实际文件未提前发布、结果移交后窗口销毁不
撤销结果、关闭期间尚未完成以及最终清理。区域菜单现已接入，真实桌面与 HDR 查看器仍未验收。

文档来源增量：PreparedDocument 可直接消费 Captured，同帧 SDR 预览零复制移交并
准备命中区域，原图路径和诊断记录到绑定文档身份的 CaptureSource。新钉图构造和普通
替换提交均保留该记录，旧 GDI 截图入口也走此交付类型。普通替换清除旧来源，直接换
Editor 身份后读取拒绝旧记录。测试使用不存在且不打开的来源路径，验证不误解码 PQ
原图、预览存储身份、负坐标 origin、诊断传递和旧文档隔离。记录不代表文件读写授权，
不代替已编辑输出；会话序列化及编辑后的 HDR 导出尚未接入。

原图另存增量：右键已有“另存 HDR 原图（不含编辑）”，仅当前本地文档有活动原图
租约且没有导出任务时启用。路径选择后复核文档戳及 Arc 租约身份，再复用 export_job
后台 owner 完成复制与无覆盖提交；关闭等待任务结束，不另加常驻轮询。复制从现有
句柄按显式偏移读，每次至多 64 KiB，不重开记录路径、不解码或重新编码，限制 PNG
扩展名、签名和输入大小。临时文件测试验证字节完全一致、相同句柄重复复制、同名不
覆盖、无效签名/扩展名与取消清理。未执行可见保存对话框；这不是编辑后 HDR 导出。

区域菜单增量：区域新钉图和区域替换现在在选区关闭后调用 wait_cached；冻结 SDR
截图只作为选区参考，最终内容来自确认后的采集帧。释放选择结果像素、DwmFlush 后
开始采集，外层继续隐藏宿主。OTTER_CAPTURE_DYNAMIC_RANGE 控制 Auto/HDR/SDR，
未知值默认 Auto；缓存使用隔离 native profile 和当前配额。最终物理矩形校验负坐标、
尺寸与 i32 加法溢出。返回后复核 HWND/文档代次才发布，再进入后台文档准备。全屏和
长截图保持现有路径，不隐式套用单屏 HDR 规则。注入准备结果及隐藏 HWND 测试覆盖
新钉图来源/位置交付与等待期间文档变化拒绝，不启动真实 WGC。

采集诊断增量：右键菜单已有“截图信息”，HDR 降级时标签直接标识，点击后按需显示
原图格式、SDR 预览、降级原因、原图路径及显示器元数据。未知配置和白电平回退值不
冒充已确认数据。来源读取绑定文档身份，无来源/身份变化时菜单禁用；对话框之前
释放画布借用，沿用模态 guard。路径/诊断替换控制字符并限制为 1024 字符，只展示不
访问文件。纯文本与真实系统菜单测试覆盖状态/原因/有界内容和身份失效，未显示可见
信息框、未采集桌面。生产读取方已接入，移除来源字段和 getter 的临时 dead_code 预期。

HDR 文件事务增量：新增 PreparedHdr::prepare/commit_new，编码器直接写入同目录唯一
暂存文件，sync_all 后等待 UI owner 复核，再使用现有无覆盖提交；不缓存完整压缩 PNG。
session_file_write::Staged 新增格式 owner 使用的 prepare_stream，流式结果不持会话
候选 Pixels 身份，不能仅凭相同字节冒充会话候选。已有会话写入与候选身份语义保留。
HDR 临时文件测试验证发布前不可见、发布后 RGB16 回读、同名不覆盖、取消与放弃清理、
格式/像素错误；事务测试补充部分写入错误、迟到取消和候选身份隔离。所有运行产物在
linshi 内，未触碰正式用户文件。成功提交后的文件不随迟到取消删除；缓存归属已有上述
独立入口，UI 请求复核与预览/原图的实际交付仍待接线，不能宣称整个截图菜单已可用。

HDR 同帧预览增量：动态范围 HDR 结果同时保留 RGB16/PQ 原图和从相同 scRGB 映射生成的
SDR 预览；预览使用当前白电平，不从 PQ PNG 误解码为 sRGB。两份输出各自计预算，
HDR 转换失败时已生成的预览自动释放，取消不交付半成品。Frame::into_parts 只移交
所有权；返回前校验原图、预览、选区的尺寸及字节长度一致。三项新增测试验证 160 nit
内容在 320 nit SDR 白电平下得到 188 灰度预览，同时原图保留 160 nit 信息，以及尺寸
拒绝、无复制移交和迟到取消的双份租约释放。未改变现有画布色彩格式，也未接菜单/文件发布。

HDR 等待窗口增量：新增 wait_dynamic 模态入口，界面只有截图状态和取消按钮；捕获
排除设置失败则不启动任务。窗口持有单个 CaptureJob，只在任务存在时用 25 ms 定时器
检查完成；取消/关闭不立即销毁在途 owner，而是禁用按钮、继续泵消息，等待取消完成后
返回空结果。显示布局变化则返回错误。成功、失败和销毁均清理定时器；外部销毁仍会
取消/join，不能保证中断驱动。三个隐藏窗口测试验证单次交付、取消、布局变化及退出
回收，未启动真实桌面采集，未验证可见窗口捕获排除。菜单路由和文档/HDR 文件交付仍待接线。

HDR 后台 owner 增量：新增 CaptureJob，使用命名线程调用生产 dynamic_region，后台
自行创建线程局部捕获资源，不把 HWND/HMONITOR 或映射传入线程。每任务持有一个结果，
不设置队列或定时器；调用方仅在任务存在时调度 take_ready。取消不可撤销，worker 返回
和提取时双重检查，迟到/已完成未提取结果均丢弃，像素租约正常释放。Drop 取消并 join，
不能中断驱动内调用，不承诺任意情况下立即关闭。四项无桌面采集测试验证线程边界、
单次交付、取消、结果释放、panic/错误及退出 join。UI 任务数量与代次保护、文件发布
仍待接入，不能仅凭后台单测宣称窗口关闭或真实采集已验收。

HDR 生产 SDR 适配增量：新增 desktop::capture::dynamic_region 后台入口，结合实际系统
探测和动态范围编排，在需要 SDR 时调用原有 capture::region，所有路径使用相同物理
选区。只移动已有共享像素，不复制 GDI 实现或新增整图副本。采集前/后取消不交付，
但不声称可以中断已经进入的 BitBlt/驱动调用；系统探测失败明确返回诊断，不猜测能力。
两项测试通过内存 DC 验证真实 GDI BGRA/alpha、负坐标参数、单次错误传播及迟到取消；
没有读取桌面。该入口已包含生产 GDI 回调，UI worker 和文件发布仍待接线。

HDR 系统探测与交付边界增量：通过已加载系统 ntdll 动态解析 RtlGetVersion，只读查询
数值版本，不加载外部 DLL、不释放借用的模块句柄。Options::detect 在标准区域且非显式
SDR 时探测；失败返回错误，不默认为支持。Mode::parse 对齐主干大小写/空白/未知值
语义。该版本门槛仍须结合 WGC IsSupported、显示器 HDR 状态与实际捕获结果。
动态范围真实入口返回前验证选区尺寸、像素长度及取消状态，拒绝回调返回的截断数据或
其它尺寸图像。新增当前主机只读版本查询、数值边界、选项解析及真实 SDR 编排入口的
注入测试；未调用 GDI/WGC 读取桌面。生产 GDI 回调已由适配层接入；UI 请求和文件发布仍待接线。

HDR 动态范围增量：新增 Auto/HDR/SDR 编排，复用主干的标准区域、Windows 11 和显示器
HDR 启用条件；显式 SDR/非标准工作负载不进行 HDR 探测。Auto 根据内容峰值是否超过
SDR 白电平加 5 nit 决定输出；正常 SDR 内容直接从同一浮点帧转换，不再次抓屏。
跨屏、配置不可用、系统不支持或 HDR 失败可转交单次同选区 SDR 回调，输出保留原因和
降级标志，失败诊断不丢失。取消、显示器消失、布局变化、WGC 尺寸变化必须终止，不得
触发回退采集；HDR 成功后复核颜色元数据，开关/白电平变化也丢弃结果。
五项测试覆盖模式/工作负载/显示器/系统矩阵、合成浮点帧输出、单次回退及嵌套终止信号。
真实入口已连接 Target/显示器/WGC、系统版本探测及生产 SDR 适配，UI/文件发布
尚未接线；没有进行真实显示器截图，不能声称端到端 HDR 已完成。

HDR 桌面坐标增量：新增 Target，将桌面全局物理 RECT 绑定到单显示器局部 Region。
使用 [MonitorFromRect](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-monitorfromrect)
选择候选后继续检查完整包含，负坐标使用宽整数差值，不再应用 DPI 倍率；零尺寸、反向、
超限、跨屏或越界直接拒绝。显示布局查询临时切换线程 Per-Monitor V2 并恢复原上下文。
采集前后复核句柄、设备名和矩形；改变时丢弃结果，不自动重选目标。转换回调只做内存
转换，成功返回前不得发布文件/剪贴板/UI。测试覆盖负坐标、精确边界、采集前阻断、
采集后丢弃/释放及取消；未运行真实显示布局变化或热插拔。跨屏 SDR 回退与 UI 请求仍待接线。

HDR 区域读回增量：新增源纹理局部物理像素 Region 和 WGC monitor_region 入口，
在创建 staging 前校验零尺寸、越界及加法溢出；不隐式截断跨边界选区。使用
CopySubresourceRegion 直接复制到选区大小的 staging，CPU 映射和颜色转换只处理选区。
WARP 合成四色纹理验证右列及角点实际像素、1×2 选区仅准入 16 字节逻辑 staging、
非法区域在分配前拒绝。WGC 两帧池仍使用整张显示器尺寸；源缓冲和驱动私有内存不会
因此消失。全局到局部坐标已有 Target 层；UI 请求、跨屏回退及真实截图仍待接线/验收。

HDR WGC 会话增量：新增显示器捕获入口，在后台 MTA 线程按显式请求创建独立 D3D11
硬件设备、WinRT device、FP16 FreeThreaded 两帧池和 session。保持系统边框、关闭光标，
不请求无边框权限。取帧只在本次请求期间轮询，期限一秒；取消或失败关闭会话，不留全局
设备或池。帧、surface、texture 在读回完成前保持存活，先归还帧再关闭 session/pool。
拒绝内容尺寸或底层纹理尺寸变化，避免转换未定义边缘。windows-core 0.60 成功空接口
仅按 S_OK 识别，E_POINTER 等真实错误继续传播。新增 Windows API 功能绑定，未新增 crate。
测试验证初始化前拒绝、尺寸/预算计算、空帧 HRESULT 和取帧超时/取消，未启动真实采集。
该入口已接读回、局部区域裁剪及全局坐标 Target 层，但还未接 UI 截图请求与动态范围/降级策略。

HDR GPU 读回增量：新增单次 FP16 staging 复制与 Map 接口，拒绝不同设备、deferred
context、多 mip/数组、多采样及非 FP16 纹理。使用
[D3D11 Map 的 DO_NOT_WAIT](https://learn.microsoft.com/en-us/windows/win32/api/d3d11/nf-d3d11-id3d11devicecontext-map)
识别 GPU busy，最多重试 250 ms，检查取消；其它 API 错误不重试。映射 guard 在成功、
失败、取消或 panic 退出时 Unmap，回调只借用当前映射，不复制整张浮点数据。
先预留逻辑 staging 字节，再按实际 RowPitch 补足预算；驱动私有显存仍不可精确计量。
unsafe 接口要求调用方独占 immediate context 并冻结源纹理，后续捕获 worker 必须遵守。
WARP 合成纹理验证真实 SDR/HDR 读回、不同设备拒绝及转换失败后的重试，另以注入测试
验证 busy 超时、永久错误和取消。未采集真实桌面，未接 WGC，也不声称软件期限可中断驱动调用。

HDR PNG 增量：使用与主干相同版本的 png 0.18.1 编码库，新增独立 writer 入口，写出
16 位 RGB 以及 cHRM、cICP、mDCV、cLLI 块，遵循 [PNG 第三版](https://www.w3.org/TR/png-3/)。
输入尺寸、长度、亮度范围在写入前验证；采用三行缓冲和 4 KiB chunk 的流式路径，
保守预留 4 MiB 加三行的工作区预算，不累积整张压缩输出。按行及底层写入检查取消，
失败后锁闭 sink，避免编码器析构继续写尾块。IEND 和 flush 完成才返回成功，但不表示
sync_all 或文件发布完成。测试验证像素解码回读、元数据与块顺序、无写入拒绝、失败锁闭、
取消及预算回收。真实截图接线、文件分配/发布和 HDR 查看器显示仍待验收。

HDR 输出像素增量：迁移主干 scRGB → BT.2020 矩阵及 ST.2084 PQ 转换，生成 RGB16 大端
数据，保留超过 SDR 白电平的高亮，携带内容/平均亮度与校验后的 mastering 范围。
依据 [Windows Advanced Color 的 scRGB/HDR 表示](https://learn.microsoft.com/en-us/windows/win32/direct3darticles/high-dynamic-range)，
scRGB 1.0 对应 80 nit；此输出不经过 SDR 归一化或 8 位预览。负值/非有限通道仍沿用主干
归零策略，不宣称完整保留负 scRGB 色域。输出按实际容量计入共享预算，逐行检查取消，
失败释放租约，克隆持有同一存储。测试将 0–10000 nit 整数采样与双精度 PQ 参考比较，
并覆盖带填充的浮点行、RGB 大端顺序、1000 nit 高亮、异常元数据、预算拒绝和取消后重试。
此增量没有新增依赖，也尚未实现 HDR 文件编码或实际浮点捕获接线。

HDR 显示器元数据增量：新增按需创建 DXGI factory 的只读查询，最多枚举 32 个适配器、
每个 64 个输出；DisplayConfig 最多 128 条路径、512 个模式、两次尝试。根据设备名匹配
活跃路径，组合 PQ 输出空间与高级颜色启用位；系统配置未知和白电平未知分别标记。
白电平遵循 [Windows 的 raw / 1000 × 80 nit 单位](https://learn.microsoft.com/zh-cn/windows/win32/api/wingdi/ns-wingdi-displayconfig_sdr_white_level)，
读取失败使用主干的 203 nit 回退，并校验亮度范围。新增测试覆盖配置组合、异常亮度、
枚举缓冲上限、设备名匹配和空 monitor；未打开捕获会话或改变显示设置。
只增加现有 windows crate 的 Win32_Devices_Display 功能绑定，没有新增第三方 crate。
本层尚未接入截图入口，真实 HDR 显示器查询和捕获效果仍待验收。

HDR 转换层增量：新增借用 R16G16B16A16_FLOAT 行缓冲的分析与 SDR/BGRA 回退，复用主干
80 nit 参考白、异常通道归零、增益限幅及 SDR 白电平规则；像素输出计入共享预算，逐行
检查取消，额外不复制浮点帧。穷举全部 65536 种 half 位模式对照独立数值参考，并测试
行填充、通道顺序、预算和取消。该层尚未接截图入口，不表示 HDR 捕获或 HDR 输出可用。
Microsoft 的 [HDR 屏幕捕获说明](https://learn.microsoft.com/en-us/windows/uwp/audio-video-camera/screen-capture)
要求注意完整捕获管线的浮点格式及后续输出处理；不能用本次转换测试替代实际设备验收。

N05 屏幕取色增量：右键菜单已新增画笔/边框及填充的屏幕取色入口。
显式开始后安装会话期鼠标 hook，移动合并、左键松开确认、右键/Escape/失焦/关闭取消，
以会话代号拒绝迟到消息。GDI 按事件读取一个像素并显示 HEX，不抓全屏、不设空闲定时器。
提交前检查来源样式/工具/文档，释放临时 hook、预览窗口与模态状态。隐藏测试仅注入
颜色和路由消息，未安装真实 hook 或读取桌面；HDR、跨应用鼠标及可见混合 DPI 尚待验收。
预览已补上原生色块与 HEX，按窗口 DPI 缩放尺寸、字体和内距；跨屏定位至多重新
校正一次，WM_DPICHANGED 同步窗口尺寸。相同颜色不重复更新文本，自绘保留可访问
文本；字体/画刷逐次回收，无持续渲染。内存 DC 在 96/144/192 DPI 下验证实际色块像素，
循环绘制核对 GDI 计数，未显示真实预览或执行全局取色。

N06 配置迁移新增纯 JSON 文档边界：从主干 app-settings 的 schema 1/2 或缺省版本读取
命名字段，拒绝未来版本/坏类型/非法模板，不沿用旧实现的静默版本归一化或损坏后重置。
编辑只合并已知命名字段，保留其它配置和命名扩展值，输入/输出限 1 MiB；格式化与重复键
不承诺逐字节保留。5 项合成回归覆盖未知字段、缺省和旧版本、拒绝路径、失败不变性、输出
容量。现已接显式“导入命名设置…”文件框和后台只读任务：只替换当前运行的共享命名偏好，
不写源文件，不自动访问旧用户目录。模态延迟、重复点击合并、取消、无效/超限输入、
过期选择/完成与请求窗口关闭均有隐藏 HWND 回归；真实 worker 读取合成文件并验证源字节
不变和句柄回收。新增“命名设置另存为…”：保留导入的其它字段/版本，合并当前命名值，
后台有界编码并复用现有暂存事务；UI 核对窗口 owner、命名值及来源文档身份后发布新文件。
已有目标和并发创建目标均不覆盖；失败/退出只清理本任务临时文件。实际文件往返验证
扩展保留、源文件不变、取消/过期/来源变化/关窗/竞争与回收。
可见文件框人工验收和完整设置功能迁移仍未完成。
显式“保存命名设置”现已接到安全替换：导入/启动保留原始字节基线，worker
核对磁盘完整字节并暂存更新，UI 复核 owner/设置/文档身份后发布。复用原有恢复事务，
外部修改不覆盖，成功才推进基线；另存为成功转到新活动来源。回归覆盖连续更新、未知
字段/版本保留、外部写入冲突、过期结果、退出回收及另存后不修改原文件。恢复文件清理
失败显示成功警告，提交失败显示恢复位置。替换有可见间隔，不是原子写；
真实断电持久性仍未完成。
显式启动的恢复扫描已接入：只有原配置缺失才查找唯一、严格匹配的同目录恢复副本，
扫描/读取仍有容量限制；原配置存在时优先，不因格式错误降级旧副本。恢复来源只读并
提示用户，不重建原路径或删除恢复文件；直接保存被 UI 和底层同时拒绝，另存为后才能
更新新文件。真实 EXE 使用合成中断现场验证唯一恢复、原文件优先、歧义及无效/超限拒绝，
源文件保持不变；隐藏宿主验证恢复来源只能另存及之后继续保存。未做真实断电测试。
现支持 `--settings path` 显式启动读取，可与普通/图片/截图/工作区启动组合；读取在建窗
和截图前完成，错误不降级默认值。初始窗口与恢复窗口保持同一命名 owner，原配置文档
仍用于后续另存为的未知字段保留。真实 EXE 回归覆盖普通隐藏启动、工作区恢复、中文
相对路径、无效/超限/缺失配置及重复参数；源文件字节与退出后可移动性均检查。
未指定 --settings 时，普通启动现使用 LOCALAPPDATA/OtterNative/app-settings.json，
与主干数据目录隔离；OTTER_NATIVE_PROFILE_DIR 可显式覆盖目录，--settings 优先级最高。
启动只读，首次缺失使用默认设置而不创建文件；坏配置和恢复歧义仍报错，不重置。首次
保存或用户开启自动保存后的变化才创建目录并发布新文件；并发首写不覆盖。隐藏检查无
显式 profile 覆盖时不访问真实目录，测试 runner 清除继承的覆盖变量；全部新回归使用
linshi。真实 EXE 验证缺失不建目录、已有配置加载、显式优先与无效根/配置拒绝，隐藏
宿主验证首次保存、后续更新和自动首写竞争。自动保存开关仍默认关闭且不跨启动保留。
另新增默认关闭的“自动保存命名设置”菜单。用户开启后，仅在活动来源有效或已有默认
profile 目标且命名值变化时排队保存，复用有界单任务和既有完成消息，无空闲定时器；
模态/导入/保存期间不追加。
关闭会丢弃未发布的自动结果，手动保存仍可继续；冲突或过期等错误会停用并提示，保留
当前设置、取消待退出，避免循环重试。应用级退出等待自动保存完成再走工作区确认，
不覆盖单窗口关闭或强制结束进程。隐藏回归覆盖默认关闭、实际保存、相同值不重写、
模态延迟、新窗口共享、关闭取消、外部冲突/过期停用、恢复来源拒绝及退出等待。

N06 文件命名新增默认 PNG 建议名：对照主干 `file_naming.rs` 的默认保存模式，使用本地
日期、百分之一秒时间及实际输出宽高。命名与 WIC 编码复用尺寸计算，裁剪后的尺寸不受
窗口缩放/平移影响；仅在打开导出框时取时钟，不新增轮询或图像副本。已有不覆盖输出语义
不变。拖出命名及其冲突递增现由 Shift 拖动入口消费，具体边界见下文。
拖出底层新增 `drag_transport`：按需 OleInitialize/对称释放、标准 Shell IDataObject
承载 CF_HDROP、IDropSource 响应 Escape/取消/鼠标松键，仅允许 COPY 而非 MOVE。
路径包复用 32 项/128 KiB 上限和普通绝对路径准入；不访问磁盘或系统剪贴板。
真实 COM 往返与取消回调有独立测试。另有 `drag_prepare` 后台准备任务：输入快照去除
撤销历史，拒绝草稿和非法图片/透明度/策略；仅生成 PNG，保持已提交裁剪和透明度。
准备完成返回未发布暂存，调用方复核后显式 commit；取消及 Drop join 并释放暂存与锁。
合成文件回归验证真实 WIC 解码、裁剪/alpha、名称递增、完成后取消和失败传播。
平移工具下的 Shift 手势现已接通，选择/绘图工具继续使用原有 Shift 框选/约束；
超过系统拖动阈值后冻结拖出模板与显示版本，后台准备期间仅有
活动完成定时器；松键、失焦、捕获丢失、Escape 或关闭取消准备。
普通取消不阻塞等待 WIC；取消任务保留唯一槽位直到完成，再由现有定时器回收。
回收不释放后续编辑手势的捕获，不阻止普通鼠标移动，窗口销毁仍强制 join。准备完成时复核
来源，发布并持有只读保护句柄后进入仅 COPY 的 OLE；不持有 UI 借用跨嵌套消息循环。
OLE 活动期间阻止本窗口编辑/命令重入，关闭触发取消；最终调用前再次检查鼠标仍按下。
隐藏测试验证专用模板冻结、仅发布一次、保护文件、防重入与失败清理；未注入真实
键鼠或执行真实 DoDragDrop，Explorer/编辑器目标和高 DPI 仍待验收。
拖动预览已接入：准备 worker 从同一最终合成位图生成最长边 256 物理像素缩略图，
保持裁剪与透明度，不放大小图、不额外解码 PNG；直通 alpha 避免 Shell 重复预乘变暗。
UI 将缩略图一次交给 IDragSourceHelper，Shell helper 不可用时记录错误并继续文件拖出。
测试验证尺寸边界、半透明像素、真实 Shell 私有格式初始化和连续释放后的 GDI 数量；
未实际显示拖动图像，固定物理像素上限及居中热点仍需跨 DPI 交互对照。
递增分配底层现已实现：与主干相同的原名、_2 至 _9999 顺序，原子 create_new 防止
并发覆盖，只有 AlreadyExists 重试，支持分配前取消；Unicode stem 含后缀最多120字符。
现有显式 PNG 导出复用 exact 分支，仍拒绝同名而不自动改名。新增“保存 PNG 到默认目录”
将冻结的显示快照与保存模板输出到原生 profile 的 saved 子目录，后台创建目录/编码并
消费递增 API；不打开路径框，不把 --settings 文件父目录当图片输出目录。已有导出或模态
期间不启动，复用单任务及退出 join，静止时无新增轮询。实际 worker 合成 PNG 回归检查
文件解码、名称递增、旧文件不变、命名冻结与目录冲突。拖出生产消费已接入 Shift 手势，
真实菜单操作仍待人工验收，不能据此标记完整命名迁移完成。合成并发文件、次数上限、权限失败、
取消和超限输入已建立回归，不接触真实用户目录。
命名基础层进一步支持主干 12 个占位符、默认 app/kind/label、短 ID 和标题长度规则，
默认 PNG 已改走统一渲染器。流式清理保持 120 字符输出，不先展开大字符串；模板4 KiB、
各上下文64 KiB为显式容量边界，旧超限配置不能静默导入。
“文件命名设置…”现有四字段原生输入入口（保存/拖出/剪贴板模板与标题长度），整体校验
后一次性应用；取消、无效及过期草稿不覆盖当前值。编辑先在本次运行生效，可通过上述保存机制
持久化；应用级共享设置随新建/恢复窗口继承。取消或无效输入不改设置，确认后的模板
用于下一次导出建议名。隐藏验证覆盖
窗口共享及恢复继承，专项覆盖 ID/尺寸/时间输入。现按主干 `buildUnitFileNamingContext`
接通能力名称到 label/title、artId 到 process，不误用 OS 标题或进程名；标题长度设置作用于
真实能力名称。普通图片使用 image/空标题/空 process，离线 Art 保留会话 artId 并回退 art
标签，重新选择能力优先使用新身份。上下文借用目录，恢复 ID 超过64 KiB不复制或截短，
仅拒绝默认建议名，原会话仍保留。测试覆盖真实 State 到导出建议名、Unicode 标题截断、
离线回退、新选择覆盖及实际恢复状态携带 ID。命名四字段解析覆盖非法模板、字节上限和
标题整数范围，隐藏宿主覆盖整体共享与模态期间变化拒绝。可见对话框和高 DPI 仍待人工验收，
其它设置 UI 仍待实现；
设备别名额外规避属于比主干更严格的安全行为。

原生保存现在持久化 `otterNativeCoordinateSpace` v1（原点、缩放），解决文件缺少保存
坐标上下文的问题。正式读写验证扩展类型/版本/数值，保留未知子字段；旧文件缺省仍可读，
但不能据此猜测旧 DPR。内存往返和实际保存合成文件均检查元数据与快照一致。恢复现可
自动选用已验证元数据，完整显式参数优先以支持有意重定位；无元数据的旧文件仍要求
明确原点/缩放。可见模式缺少上下文时弹出原生三字段输入框，默认值为空，不猜当前
显示器；取消保留原工作区。隐藏检查仍在建窗前拒绝。真实隐藏 EXE 检查负坐标/1.5 倍
缩放的实际 HWND 矩形及显式覆盖结果；注入输入结果的回归覆盖取消、模态期间编辑及
有效输入安装，实际坐标输入框的键盘/高 DPI 操作仍待人工验收。
“打开工作区…”菜单已接系统文件框、未保存替换确认及后台恢复。选择/确认后复核来源
凭证，取消或失败保留原工作区；保存/恢复进行中拒绝新打开，模态期间保留排队请求。
不根据 JSON 或所选文件父目录自动授权外部图片。隐藏集成用既有 linshi 保存文件，验证
正式菜单、合并、模态延迟、取消、过期来源、读取失败和恢复发布；不显示文件框，实际
交互验收仍待完成。
另增“打开工作区（指定图片目录）…”入口，通过原生目录选择框取得一次性显式 Root；
普通打开仍无外部目录权限。Root 沿用直接子文件、非重解析点和最终句柄路径核验，不扩大
为递归访问。目录句柄随保存后打开意图保留，取消/失败丢弃，成功移动到恢复 worker。
隐藏回归覆盖菜单合并保留显式目录意图，以及带真实 linshi Root 的保存后恢复；实际目录框
和用户外部图片会话操作尚待人工验收，不能据此声称完整资源迁移完成。
新增独立真实 EXE 外部图片回归：中文/空格目录内合成 PNG，仅正确显式目录成功恢复、
绘制两次并重建设备；无授权、错目录、同前缀兄弟目录、子目录、穿越和坏图片均拒绝。
失败夹具还提供有效 filePath，确认不会绕过 src 准入回退。全部源文件逐次核对字节未改；
进程退出后图片及目录可重命名，登记夹具完整清理。该检查走 CLI 授权，不展示目录选择框。
保存后打开的连续流程已接入：确认选择“是”后暂存目标路径，成功保存且状态未变化才
开始恢复；失败或路径选择取消不继续打开，后续保存/打开/退出意图取消旧继续操作。
隐藏集成使用同一合成会话实际保存再重新打开，验证 worker、磁盘提交及窗口替换；
另验证编辑导致保存过期时不写文件、不打开目标。可见保存框与连续交互仍待人工验收。

保存状态现有独立轻量凭证：新建/恢复副本为 Unsaved，正常文件导入及提交成功后建立
Unchanged 基线；编辑器/配置版本、窗口几何和工作区结构改变为 Changed。只按需查询，
不保留旧像素或整份编辑器快照，不增加空闲轮询。隐藏回归覆盖成功建立状态、提交拒绝
不清除变化、透明度改动与恢复、模态/热键忽略，以及草稿出现/取消。此为保守版本变化
判定，撤销后内容相等不保证自动变干净；可见状态展示和完整窗口关闭确认仍待实现。
坐标 Space 也参与变化判定；恢复时先安装 Space 再建立基线，避免记录不完整上下文。
真实隐藏会话入口分别断言普通文件/空会话为 Unchanged、恢复副本为 Unsaved。
应用级“退出 Otter”已接保存/放弃/取消确认（默认取消）：保存沿用后台保存路径，成功且
状态仍未变化才继续退出；失败、路径对话框取消、再次编辑均不销毁窗口。放弃前核对模态
期间是否变化；正在进行的 UI 保存完成后也重新核对。隐藏 HWND 注入决策覆盖状态转换，
不等于真实对话框已人工验收；单个窗口/分组关闭仍沿用既有语义，尚未接入该确认。
保存后退出另有真实隐藏集成：只替换消息提示出口，通过正式 coordinator 写入 linshi
合成会话并核对磁盘/内存基线，成功只排队退出，过期结果保留窗口和旧文件。完成时若
已有后续保存请求，撤销旧退出意图并完成后续保存；检查两个提交均成功而非忙碌丢弃。

原生菜单已接“保存工作区”：正常导入/已提交文件走既有路径替换，新建工作区或恢复
副本因无 saved_path 转入另存为。原有“工作区另存为”仍只创建新文件，同批请求优先
保留明确另存为意图；保存不改变 Ctrl+S 导出 PNG。分组锁不阻止保存，也不会因此解锁
内容。隐藏 HWND 通过正式菜单分发验证锁定窗口保存、请求合并与另存为优先，并检查
新建/已保存工作区的路径选择分支；可见操作与恢复提示仍待人工验收。

已新增原路径缺失时的只读恢复读取：同目录最多扫描 4096 项，精确匹配恢复命名，
仅唯一普通文件候选可继续原有有界读取和正式会话验证；歧义、坏内容和访问错误不猜测。
读取期间原目标重新出现时优先正常目标；不移动/删除恢复副本或重建原路径。来源信息
贯穿 worker/Candidate 到发布：普通文件保留保存路径，恢复副本明确标记并要求另存为。
隐藏 EXE 已验证模拟遗留状态可恢复且两个路径不被改写；可见提示及断电持久性仍待
验收，不能将只读回退称为完整自动磁盘修复。
真实子进程中断回归另覆盖暂存完成、旧文件移开、新文件发布、旧文件退休四个边界：
使用生产替换实现后直接 exit，跳过 Rust Drop，由父进程校验退出码、选择应恢复的旧
内容或应保留的新内容、完整暂存/恢复字节及句柄释放。只有原路径缺失时发现恢复副本，
正式目标存在时不回滚新版；遗留暂存文件尚无自动清理策略，测试由父进程清理自有目录。

覆盖准备新增只读 `session_file_guard::Guard`：打开既有普通文件并持有仅共享读的句柄，
固定缓冲核对全部来源字节和取消，记录 volume/file index。测试覆盖长度与 revision
相同但内容改变、已有写者冲突、租约期间写入/重命名拒绝、多读者以及释放后可写。
仅在 linshi 执行合成测试；替换准备已接保存 worker。释放租约到路径替换之间
仍需独立解决竞争，不能把此模块或仅比较 revision 当作覆盖提交完成。

已验证同一句柄上的不覆盖重命名：`Movable` 明确增加 DELETE 权限，在完整基线核对后
调用 `SetFileInformationByHandle(FileRenameInfo)`，不释放原锁或按来源路径重新打开。
linshi 回归验证移动到恢复路径、锁仍阻止写入、再移回原路径后重新打开仍是同一文件
身份，以及既有目标不会被覆盖。普通只读 Guard 不暴露移动接口。
接口依据 [FILE_RENAME_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_rename_info)。

`session_file_replace::Replacement` 已组合分阶段文件替换：持有父目录、来源基线和
暂存文件，旧文件移到可识别恢复名后以不覆盖方式发布候选；失败尝试移回，目标被
竞争者创建时保留对方文件与旧恢复文件并明确报告路径。成功只退休原句柄对应旧文件，
退休失败作为“已发布且有恢复文件”的结果单独报告。测试覆盖正常发布、准备丢弃、
移动前后取消、内容/修订冲突、竞争创建、panic 恢复、恢复名碰撞与退休失败。
替换已接 Application 保存 worker、提交接口和保存菜单；崩溃后的自动
识别恢复和真实断电验证仍待做；原路径短暂缺席，
不能把这两个 rename 称为原子替换。所有实际写入/移动/退休测试只在 linshi 的合成目录。

隐藏 Application 现在实际走通“新文件提交→按 saved_path 后台替换→更新基线”，
之后再准备一次替换并改变工作区，验证最终提交拒绝、磁盘与内存基线不变，且来源句柄
即时回收。磁盘准备使用排他 New/Replace 枚举，两种结果共用提交前 Receipt/候选绑定
检查；UI 消费层保留“已发布但恢复文件清理失败”的告警语义。

启动回归 runner 已修复“先等待退出、再读取管道”的阻塞隐患：独立大输出复现中旧
模式 3 秒后仍未退出，stdout 仅积累 4096 字符；新 runner 同时排空双流，每路有界保留
256 KiB，512 KiB/流的压力回归可完成，超时保留部分诊断并回收自有 child/reader。
startup/session_startup 分别保留原 15/30 秒阈值；这证明测试基础设施的缺陷与修复，
不能据此断言此前所有原生启动超时都由此引起，后续超时仍须读取阶段输出定位。

右键菜单现接入“工作区另存为”：请求排队后由应用循环打开 Win32 路径对话框，
选择前后检查凭证，后台编码/暂存，常规消息循环领取并复核提交，显示成功或失败结果。
目前仅允许新文件。新建工作区已有明确的 revision=0 基线及原生物理像素空间，
首次保存不再依赖导入会话；导入仍保留历史空间。覆盖与冲突处理仍待完成。
新建隐藏 HWND 回归验证空宿主省略、钉住后保留，以及首次候选 revision=1；
取消/过期恢复回归改为检查原基线 owner 保留，而不是假定新工作区没有基线。
隐藏回归验证请求合并及后端链路；可见菜单、路径对话框和消息提示尚未人工验收。

另存为准备已支持同一后台任务完成编码和同目录暂存，应用任务槽覆盖两个阶段，
与恢复互斥。UI 收到完成结果后仍须复核并提交；真实隐藏回归改用后台生成的 Staged，
测试验证已暂存结果遇到迟到取消或任务销毁时回收临时文件，不创建目标。

新文件提交成功后现同步采用实际提交字节为后续保存基线并记录目标路径，旧 Receipt
因源身份变化失效，下一次候选从已提交 revision 递增；失败不交换原记录。已写入会话的
临时宿主转为持久节点。`session_source::Source` 同时支持导入 Vec 与预算编码缓冲，
均共享原分配，成功提交不再复制整份 JSON。隐藏回归核对精确 owner、路径及下一修订。
最终提交失败也走实际调用验证：取消、晚到的工作区变化、目标竞争创建均保留旧基线、
路径及临时宿主标记；目标未创建或既有内容保持，不只检查提交前的布尔判断。

编码交付现在返回持有 Receipt 的 Prepared，而不是丢弃凭证的裸 Candidate。临时文件
绑定候选的不可变字节 owner 与修订号；最终新文件提交前再次核对候选绑定和 UI 状态，
不能仅凭修订号相同放行。隐藏回归已在 linshi 走通后台编码→暂存→提交前过期检测→
新文件提交→字节回读→保存基线更新，并清理本次合成文件；界面接线见本节开头。

磁盘层已新增 `session_file_write::Staged` 的新文件另存为路径：同目录唯一临时文件、
分块写入及 sync_all，提交时不启用覆盖/跨卷复制标志。若目标在准备后被其他写入者
创建，提交失败且保留对方文件；取消/失败只清理本次临时路径。所有执行测试仅在 linshi。
这不替代已有会话覆盖：来源身份/修订冲突检测与替换事务仍须继续实现；
没有宣称网络文件系统或断电持久性已经验证。

Application 保存准备已有单任务槽：UI 捕获 Receipt/Data，后台调用统一编码入口，完成
通知复用 execution_wake；交付前复核 Receipt，取消及过期候选不交付。恢复请求与保存
准备互斥；退出取消并回收 worker，不遗留无人持有线程。隐藏回归覆盖成功、重复请求、
取消和工作区变化拒绝；候选完成不等于保存完成，须经过最终新文件提交。

完整内容的内存候选入口已建立为 `Data::encode`，统一后台 COM 生命周期及几何、Art
参数、裁剪、标注、原图/栅格资源编码；隐藏 Application 已改用此入口，不再复制回调。
裁剪栅格语义重新核实：`VectorCandidate::prepare` 为编辑器保留可见框纹理，不使用效果
计算的完整源框投影，因此写出不得额外裁切。新增普通裁剪会话实际恢复→编码→再恢复→
D2D/WIC 合成像素比较，覆盖独立栅格、矢量及水平翻转；该范围已通过。非等比缩略、
所有缩放/缺失来源组合、磁盘事务和 UI 保存调度仍未验收，不能据此宣称迁移完成。

缩略标注的等比写回已接入：使用窗口几何 owner 输出的 savedRect（缺省时为当前框）
作为参考尺寸，将 Editor 标注框映射回展开局部坐标；cropOffset 只保存在窗口字段，
不重复减入标注。模型已通过整会话写出/恢复验证偏移只应用一次。非等比样式仍明确
拒绝，缩略图片层和栅格标注层的完整保存尚未完成，不能视为整个缩略功能验收。

普通标注后台写出已接入内部取消检查：旧对象合并、笔画/权重点遍历、映射和 JSON
输出流均可终止，返回专用 Cancelled，不交付部分字节。输入 JSON 的单次解析和字体
原生调用仍不可强制打断，完成后再检查；不把这一能力当作整个保存任务调度已完成。

图片编码现优先消费快照中已验证仍匹配底图的 restored_source，保留原图分辨率，不把
窗口投影缓存再次当作源图；来源失效则编码当前底图，栅格标注始终取当前独立图层。
普通隐藏快照已在同一候选中写入最新标注及图片，并回读高于编辑底图尺寸的源 PNG。
这仍不是完整裁剪/缩略保存验收，来源复用只依赖捕获时的像素身份，不依赖旧 src 字符串。

图片资源安装已处理翻转物化：原生恢复时翻转已进入底图像素，新 PNG 写出必须将
imageEditState.flippedX/flippedY 设为 false，否则重开会再翻转。栅格标注层保持独立，
不随底图重复翻转；其余状态字段保留，非法 imageEditState 结构拒绝而非覆盖。

裁剪元数据已接入活动快照组装：`session_crop_write` 消费当前 CropView 的 source，
只更新 cropRect/sourceSize，保留小数和未知扩展，不混用图像像素、窗口尺寸或 DPI。
重置裁剪同时移除两项旧字段；正式读取器回读覆盖裁剪/重置及源比例归一化。图片翻转、
擦除物化和独立栅格标注的裁剪区域仍由资源编码负责，本阶段未完成它们的一致性整合。

活动保存快照已接入整会话组装：拓扑来自当前文档/连线/分组，不复用原文件旧清单；
几何与已绑定 Art 参数先合并，再调用内容编码回调。仍为空的临时宿主不写出；发生编辑、
钉图或加入工作区关系后保留。隐藏 worker 已验证此接线，但回调中的完整图片/裁剪/
标注编码尚未全部整合，当前候选不能直接作为用户保存结果提交。
普通窗口标注已通过 `Document::write_annotations` 写入实际候选，使用恢复端同一
DirectWrite 字体基线，并在前后检查取消；隐藏回归核对新增笔画不被旧 annotationState
覆盖。缩略双层上下文明确拒绝走普通编码，图片/裁剪一致性仍须继续接通。

缩略展开事务现显式记录持久化来源：历史缺少 savedRect 的同框展开回退不会被保存为
伪造的展开框，主动缩略则保留真实展开框，即使两框尺寸恰好相同。保存快照的几何
适配与运行时双击展开共用拖动位移算法，窗口几何合并已有隐藏快照回读检查。
这不替代缩略图片/标注双层坐标编码；完整保存调度仍待完成。

保存快照已补齐视口/DPI 捕获和新鲜度校验，来源 Space 变化也使 Receipt 失效。
普通视口到会话标注坐标已有独立写出映射：组合 Viewport、窗口 DPI 与来源 Space.scale，
几何、线宽、字体和效果参数同步转换；普通文字重新测量缩放字体基线，并保留主干固定
4px 锚点偏移。缩略窗口双层视口和整会话编码接线仍待完成，不能直接套用普通映射。
真实跨屏 DPI 交互尚未验收，不能用隐藏窗口读取一个 DPI 值代替跨屏测试。

已绑定 Art 保存字段已接入公共参数 owner 和执行配置的写回模型，旧 params 不合并，
避免过滤后的 secret 复活；缺省/null/禁用值语义有回读验证。隐藏快照已调用这条路径，
完整会话编码调度和磁盘事务仍未接通。

保存图片资源新增原始 WIC PNG 编码，底图与栅格标注层分别处理，避免复用成品图导出
导致标注重复叠加；原始像素回读与隐藏快照 worker 已有验证入口。PNG 到 Data URL
已复用带预算的 Base64 owner，同次分配前缀并验证正式读取器回读。资源安装已接入
`assemble_owned` 候选入口：字符串与租约移动而非复制，旧预览和已清空栅格层不会复活。
隐藏 Application 的后台测试已通过候选整会话回读资源；完整坐标/源框整合、资源引用
策略和原子保存仍未完成，不操作用户文件。

整会话内存候选已有身份合并、修订递增、取消/容量限制和正式读取器回读检查。
`session_write::assemble` 保留未知根字段及对象扩展，并要求窗口编码回调提供实际内容；
活动快照完整编码与磁盘锁/原子提交仍待接入，不把候选成功当作用户数据已保存。
最终 JSON 缓冲先计数、预留共享预算并一次分配，序列化期间不扩容；候选字节与租约
由同一共享 owner 持有，跨线程或克隆不提前归还预算。这只覆盖最终字节缓冲，不表示
中间 JSON 对象树的全部内存开销已纳入共享预算。

保存快照已接入活动 Application，区分只留 UI 的 Receipt 和可传后台的 Data，复用
工作区/文档凭证保护异步期间的变化。原文与像素共享，矢量隔离，显式 Space 随恢复
记录保留。它仍是保存候选入口，后台编码、完整会话合并、原子磁盘提交未完成。

窗口几何写回模型已补齐显式 Space 的反向转换和单窗口 JSON 合并，保留小数取整余量、
实际物理框以及未拥有的字段。活动 HWND 快照、标注/图片/源框的一致性封装和原子保存
仍待集成；本阶段仅处理内存候选，不读取或改写用户文件。

标注裁剪坐标逆转换已接入 `encode_visible`：输出当前可见框局部坐标，并携带对应
frame/CropView。小数偏移、重置裁剪和撤销往返已有模型回归；窗口坐标、图像源引用
和整个会话的保存事务仍待集成，不把标注片段直接覆盖到原会话。

标注写回模型已开始实现：`session_vector_write` 输出有界主干标注 JSON，按稳定 ID
合并扩展字段，形状/路径/文字/效果使用各自逆映射。仅内存输出，不操作用户文件；
完整会话坐标/资源封装、后台保存和原子提交仍待接通。不能表达的样式拒绝而非降级。

会话写回前置：稳定标注身份已进入 Editor 对象生命周期，恢复原 ID、新建/复制生成
新 GUID，撤销/重做和快照保留。旧清单仅用于初始恢复校验，不再作为未来活动编辑
写回的身份来源；实际会话序列化与原子保存仍待实现，不操作用户会话文件。

恢复缩略窗口的缺省展开已补齐：无 savedRect 时依主干保留当前窗口框，只退出缩略并
恢复正常透明度；不推测历史原图尺寸。隐藏 Win32 双击回归覆盖移动、负原点和比例转换，
可见桌面跨屏交互仍待验收。

当前增量：离线恢复 Art 的迟到目录绑定已接入 `application_restore_rebind.rs`。
只处理尚未绑定且空闲的恢复节点；同一目录身份只尝试一次，新目录允许重试，重新按
真实定义过滤保密参数，不覆盖已有选择或自动执行。原始会话继续只读保留。隐藏宿主
验证不替代真实 Loom 重连和可见界面验收；下文保留阶段演进记录，最新状态见
[功能对照台账](WINDOWS_NATIVE_PARITY.md)。持久化写回和完整功能迁移仍未完成。

工作区恢复开始接入主干会话字段：`workspace_session.rs` 提供 stickers 的 id/groupId、
links 的 fromUnitId/fromPortId/toUnitId/toPortId 及 groups 元数据的拓扑适配。恢复只
构建独立候选，验证身份、数量、分组、端点和端口后返回，不直接修改当前工作区。
导入保留历史边顺序/身份与环，不使用交互 connect 的单输入替换规则悄悄改写旧数据；
新候选获得新的运行时拓扑凭证。合成字段往返与失败隔离已有回归。本模块不是完整
会话格式，不能覆盖旧会话文件；窗口几何、图片、标注、参数、持久写入及启动恢复仍待
接入。此阶段未读取或写入用户数据，旧实现保持不变，回退仍可使用旧分支。
`session_document.rs` 已按后端 SessionData 增加只读 JSON 入口，接受 schema 缺省 0/1，
保留 u64 revision，拒绝未来 schema。它借用完整原始 JSON，未迁移的标注、图片、
回收站、参考库和未知字段不因拓扑解析被重写。数组在解码期间受 128 窗口/分组、4096
连线限制；当前总输入上限 64 MiB，超限明确拒绝而非截断。这是原生读取准入预算，
不是已证明覆盖所有主干大体积会话的兼容性承诺；磁盘 owner 和实际大数据迁移仍待接入。
窗口元数据已纳入候选：保留负坐标/小数位置、尺寸、type/artId、minified、savedRect
和两套透明度。按主干读取映射提供 false/1/0.9 缺省，显式透明度 0 原样保留；候选
拒绝非有限值、无效尺寸、越过 Win32 整数坐标范围的边界及非法透明度，不静默钳制原文。
尚未进行显示器/DPI 映射、实际 HWND 创建或隐藏/缩略状态应用，不能计为窗口恢复完成。
原图/预览/栅格标注层/原始路径引用已进入恢复候选。未转义 JSON 字符串借用会话原文，
转义路径才持有解码文本，避免大 Data URL 重复分配；按主干保留空 src、去重相同预览，
不同 Art 预览设置恢复锁定标志。测试覆盖大字符串借用、中文转义路径和 null/空值，
路径和 URL 尚未加载，资源安全准入与实际显示恢复仍需接入；内联解码进展见下段。
内联 Data URL 已新增 `desktop/session_image.rs` 后台解码入口，复用 Loom 的有界
Base64/WIC 路径；共享解码器现在直接借用 mime/编码字符串，参数图片准备也不再
构造额外 JSON 包装。真实 worker 回归验证会话引用解码为正确尺寸/像素及最后引用
释放；取消、坏数据、路径和网络引用拒绝有回归。目前支持已有解码器的 PNG/JPEG/
WebP/GIF/BMP MIME，尚未接入整批恢复调度、路径加载或可见窗口，不代表完整图片恢复。
文件图像另有 `session_image_file.rs`：接收调用者已准入并打开的 File，不解释会话路径；
按元数据尺寸和 64 MiB 上限预留共享编码预算，分块读取检查取消及长度增减，关闭文件
后再从内存 WIC 解码。真实 scratch 文件 worker 回归验证像素与资源释放；仅完成句柄
加载层，路径授权/打开策略、自动调度和窗口恢复仍未接入，不能宣称任意旧路径已兼容。
整批图像准备已有 `session_prepare.rs`：按窗口身份准备原图、独立预览、栅格标注层，
重复引用在批内共享解码像素，结果不借用会话文本。失败/取消不返回部分结果，也不以
原始 filePath 悄悄替代失败图片；文件打开由显式准入回调负责。真实 worker 回归覆盖
跨窗口/图层共享、原文释放后结果存活、最后引用释放及失败无部分发布。该层尚未挂到
应用恢复任务、路径策略或 HWND 事务，仍不能计为完整会话恢复。
受管资源目录已有 `session_asset_root.rs`：根目录由外部配置/明确选择提供，而非从
会话推导；固定目录句柄，只准入直接子文件，拒绝穿越、设备名、ADS 和最终重解析点，
读取前以文件句柄最终路径复核目录归属。已联网核对 Windows
[GetFinalPathNameByHandleW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfinalpathnamebyhandlew)
的归一化/返回长度语义。新增 windows crate 的 FileSystem feature，没有增加依赖版本。
原生 scratch 集成覆盖准入→文件读取→WIC→整批共享；尚未连接应用恢复入口。当前仅
覆盖显式盘符/已存在映射盘目录下的直接子文件，外部路径、UNC 文本和授权 UI 仍需处理。
后台候选与任务 owner 已实现：`session_candidate.rs` 拥有原始 JSON、独立窗口元数据、
拓扑和准备图像，避免自引用会话对象；`session_job.rs` 通过完成通知唤醒，取消后即使
任务已完成也不交付旧候选，退出回收线程。原文保留供后续参数/编辑状态迁移，不把这些
字段遗漏后直接发布。生命周期回归覆盖通知、二次 poll、迟到取消、坏会话和析构回收；
应用层任务准入/版本凭证与 HWND 提交仍未挂接，尚无面向用户的完整恢复入口。
Art 参数新增恢复适配：会话 params 以借用 RawValue 保留（serde_json 仅开启 raw_value
feature，不改版本），取得对应能力后才有界解析；过滤 secret/内部控制字段，保留 null
和禁用标记。未声明公共值现在进入独立保留区，与声明值共享字节/节点/条目预算，
参与执行参数编码，但不会变成声明控件或连线端口；编辑声明值不会丢失这些保留值。
高级整批编辑允许修改已有保留值或通过省略删除，不允许凭空添加未知字段。
能力缺失/版本变化恢复 UI 仍待完成，不能将此视为完整会话恢复。
对照主干 nodeParamResolution 修正原生执行声明默认值：缺失手动项回退能力默认值，
显式 null 仍覆盖默认值。生产执行参数编码回归覆盖两者；参数恢复尚未自动安装到窗口。

文件参数迁移已核对前端 `ImageControl.tsx`：file/image_link 上传值是文件 Data URL，
不发送本地路径，filename 被上层忽略。原生 `parameter_file.rs` 已实现固定 12 KiB
缓冲区流式 Base64 编码、任意短读余数保留、取消检查和单值预算，不同时保留完整原始
文件与编码结果。常见扩展名映射 MIME，未知类型回退 application/octet-stream；
不声称已复刻所有 Windows 浏览器 MIME 注册映射。file/image_link 菜单现已接入系统
文件选择框和应用级单 worker，完成通知复用现有唤醒设施，无空闲轮询。提交复核能力、
公共参数、禁用/锁定、有效视图及拓扑；过期结果丢弃，新待处理选择取消旧读取。隐藏
窗口集成回归使用 scratch 合成文件验证实际 Data URL 提交、执行触发及配置变化后拒绝
写回。另以 128 KiB 二进制文件验证完整执行参数解析与 Loom 请求编码，文件值不受
普通文本转换 64 KiB 上限截断，不泄露本地路径或 secret；读取失败保留上次有效值且
不触发执行，secret/普通数字/未知参数及锁定窗口不能借用文件导入入口。
系统选择框尚未人工验收，图片缩略预览也未完成，不能计为完整上传功能对齐。
有效显示视图现仅保留连线驱动值，没有参数连线时跳过图解析；未连线手动值仍由菜单
从公共参数 owner 读取。128 KiB 上传回归确认视图不再重复持有 Data URL，同时执行
请求值保持完整；此为所有权/分配路径改进，尚未声称进程工作集或延迟已有实测下降。
连线值复制也已改成逐项先计费后分配，统一限制 1 MiB JSON 字节、4096 节点和 32 层
深度，避免先完整克隆大字符串/容器再拒绝。保留 JSON 整数精度以及对象省略/数组 null
哨兵语义；超限仅拒绝显示副本，不解除连线锁定，也不修改执行或手动参数。
现有公共参数总预算仍为 1 MiB，最终提交须计算其他参数与 JSON 包装开销；大文件兼容
策略仍需与执行传输预算一起审计，不能静默截断文件或宣称与主干无上限上传等价。

能力参数新增原生布尔/枚举菜单：从能力声明构建打开时快照，保留枚举 JSON 标量类型，
secret 不进入菜单，disabled/分组锁定禁止提交。命令绑定选择与参数版本，过期菜单
不修改新配置；有效变化先请求取消旧执行，再提交候选并触发参数执行策略。菜单最多
512 个选择，超出部分仍可通过既有高级 JSON 入口编辑。隐藏 HWND/系统菜单回归覆盖
勾选/禁用、布尔切换、数字枚举、其他参数保留、secret 隔离与过期拒绝。
数字和文本菜单现已接入原生单参数输入框。按主干 `NumberControl.tsx` 和共享 math
合同修正为有限值、依次 min/max 限制及十位小数规范化，不额外禁止 integer 声明的
小数输入。文本保留空白及空字符串，不额外强制主干控件未执行的 required 检查。
数字增减按钮使用正 step（非法/非正回退 1），只改对话框草稿，确认才提交。
switch/radio 复用布尔/枚举入口，path 复用文本且不访问文件。
number/slider 已增加系统 trackbar，使用 0–10000 的有界位置映射并从 min 按 step
量化；范围缺失时复用主干的当前值/100 回退。滑条、输入框和增减按钮同步局部草稿，
确认才提交，暂未提供拖动过程的后端预览触发。隐藏系统控件及隐藏对话框测试覆盖
位置/数值、按钮联动、草稿不提交和父窗口销毁释放子控件；极端非有限跨度回退到输入框。
color 参数已接入 Windows 颜色框，并按主干右键合同提供清空颜色（空字符串）入口；选择
返回规范化 `#rrggbb`，取消不提交，仍核验配置快照。工具颜色与参数颜色复用同一局部
系统颜色框 owner，不维护两套原生对话框资源代码。色值转换及隐藏窗口清空颜色回归
通过，真实颜色框交互待验收。核对 `UnitParamControl.tsx`/`StringControl.tsx` 后确认
主干 path 仅是文本输入，不把此前文档推测的“路径选择器”计为迁移缺口。
公共参数重置也已接入菜单：数字/布尔/文本按声明默认值或主干 0/false/空字符串回退，
枚举未声明默认值恢复未设置、显式 null 原样保留；file/image_link 清空字符串。
候选动作区分取消、未设置和显式 null，不泄露 secret，禁用项及过期配置不提交。
隐藏窗口回归覆盖重置提交、其他参数保留，模型回归覆盖各类型回退和 null/未设置边界。
参数禁用/启用已接入原生菜单与 Parameters owner：禁用暂存原值并写入 `__DISABLED__`，
启用优先恢复暂存值（包括显式 null），原先未设置则采用主干默认值/空字符串回退。
暂存区独立受 1 MiB、4096 节点、128 项预算约束，不进入执行 wire；预算失败不修改
值、暂存或 revision。禁用时普通编辑/重置不可用。隐藏窗口菜单回归证明禁用参数进入
`disabledParameters` 而不进入有效参数对象，重新启用恢复原值；持久化暂存恢复尚待接入。
JSON/object/array 声明现支持单参数原生文本编辑：对照 `artParamTextValue.ts`，对象和
数组格式化为 JSON，字符串原样显示，null/未设置显示空文本；提交解析 JSON，不强制
object/array 声明的值形状。非法 JSON/超长输入不提交，最终公共参数 owner 仍检查
总字节、节点数和深度预算。菜单生成及格式化/解析回归通过；新类型的可见模态交互
尚未验收，超长值仍使用高级整批 JSON 入口。
多行输入按声明启用。输入最多 16384
个 UTF-16 单元，超长现值继续使用高级 JSON，不静默截断。确认后重新
检查窗口、锁定及配置凭证，取消不提交。纯解析测试已覆盖边界；新输入框的实际模态
交互、IME、高 DPI、图片等控件和完整面板尚未验收或实现，N08 未完成。
参数显示初值已独立对齐主干：未设置/null/禁用优先采用声明默认值，布尔勾选使用
JavaScript 真值规则，数字初值接受布尔/数字字符串等控件转换，非法数字采用 min/0
回退，普通文本支持数组/对象显示转换。此转换不回写参数；JSON 文本继续使用专属
格式化。新增回归覆盖默认值、禁用回退、字符串 false 的真值和数组转换；尚未穷尽
所有 JavaScript Number/String 极值格式。
有效参数显示现已接入执行图同一解析器：版本变化时生成独立、有公共参数预算限制的
视图，不覆盖手动参数；无变化复用视图身份。连线驱动的布尔/枚举显示有效值，数字/
文本/颜色菜单显示有界值预览并锁定手动编辑与重置；禁用/启用仍按主干入口保留。
上游有效值或连线状态变化使旧菜单失效，解析暂不可用也保留连线锁定。真实执行链
回归验证下游手动 amount=1 保留而有效视图为上游 7，菜单回归验证缓存身份复用、
过期拒绝及断连后恢复手动初值。
现有连线的原生断开入口已接入：右键菜单分页列出本窗口入/出连线，每页最多 64 条。
菜单只排队携带 Link 身份和拓扑凭证的请求；应用层重验端点归属、连线内容、分组锁定
和临时编辑状态后移除，随后复用现有拓扑退休/显示刷新链。隐藏窗口集成回归覆盖真实
菜单、锁定拒绝、拓扑过期请求拒绝、旧菜单拒绝和断开后的画布回退；测试重连恢复夹具。
连线创建已接入原生来源窗口/来源端口/目标端口选择框，确认后排队，由应用层复核
共享目录的配置、输出和拓扑凭证再调用 Workspace::connect，复用循环拒绝和单输入
替换规则。目录不复制图片或参数值，secret 参数不进入可选输入。隐藏窗口链验证
非法索引、自连接、重复入队、拓扑过期及旧目录拒绝，并验证实际创建后恢复显示。
隐藏系统选择框回归已覆盖来源切换、输出端口重填、仅修改草稿及销毁后子控件释放；
应用链另验证循环请求拒绝后原图不变、占用输入原子替换且显示保留。打开选择框前
也会拒绝已被替换的目录。可见交互尚未人工验收；拖拽、画布连接线绘制和完整属性面板仍未完成。

上游编辑图片合成的数据接口已建立：`desktop/display_composition.rs` 复用导出绘制，
直接读回共享 PBGRA 像素，不编码/解码 PNG。无可见编辑时共享权威底图；有编辑时在
工作线程局部持有 COM/WIC，合成、上传和读回均受预算约束，取消在原生阶段边界检查。
测试覆盖实际线程中的栅格/矢量/裁剪合成与现有 PNG 导出结果逐像素一致、草稿及非法
输入拒绝、预算失败释放与最终像素租约生命周期。
现已接入单应用单 worker 的异步显示调度：不建立任务队列或常驻轮询，完成后使用
现有应用唤醒；按来源文档身份/版本及像素身份复用结果。来源变化、断连和关闭淘汰
缓存并取消旧任务，相同版本失败不循环重试。隐藏窗口链验证上游笔刷及裁剪合成、
目标 Art 追加独立标注、缓存共享、一步撤销、上游撤销裁剪后的尺寸刷新和正式输出
隔离。取消已完成未交付任务及同版本不同文档身份拒绝也有回归；尚无可见人工验收。

Art 自身正式图片的尺寸适配已接入显示与临时编辑：按新旧底图尺寸映射矢量坐标、
样式及裁剪框，文字使用 DirectWrite 重测。转换只生成快照，不写原文档历史，不复制
正式底图；映射失败保留上一显示帧，并等待下一次文档或结果变化，不循环重试。
回归覆盖非等比映射、文字测量失败、非法尺寸，以及隐藏窗口链中 2×2 到 1×1
标注/裁剪接管后取消恢复原文档身份。已有空覆盖层提交和一步撤销回归继续通过。
Art 无生成图片时，经连接回退到上游正式图片（包括透明钉图中继）也已支持本地覆盖层。
显示帧分别记录图片来源和覆盖层所有者，编辑接管只修改目标 Art；取消、提交和一步
撤销已有隐藏窗口链回归，源文档及目标正式输出代际不变。上游本地编辑图片通过上述
异步合成后再叠加目标覆盖层；视觉与主干逐像素对照、可见人工验收尚未完成。

Art 自有生成结果的“重置裁剪”现使用独立 Crop 接管意图，不再被位图 Content 命令
限制误拦截；菜单按当前覆盖层裁剪状态启用。隐藏 HWND 菜单回归验证重置及主循环
结算、正式结果像素不变、一步撤销恢复裁剪覆盖层，导出快照不带入旧栅格底图。此处
不包含生成结果尺寸变化和连接回退来源；这些场景的坐标/覆盖层适配仍待完成。

Art 自有生成结果上的普通标注鼠标入口已开放：临时底图绑定当前生成图片，标注与
裁剪通过覆盖层快照结算，不改变正式端口或下游正式中继。隐藏 HWND 回归验证实际
笔画、主循环结算、覆盖层导出快照、正式像素隔离与一步撤销，并验证橡皮擦不启动。
位图效果/整图内容命令继续受 Art 准入限制，翻转和栅格化菜单同步禁用。结果尺寸与
已有编辑底图不一致、以及连接回退图片时仍拒绝直接接管，避免未实现坐标适配时错误
写入；这两类场景及完整 Art 裁剪菜单语义仍是未完成项。

Art 编辑边界复核：旧 `UnitVisualOverlays.tsx` 同时为 sticker/art 显示标注，
`UnitView.tsx` 仅为 sticker 开启 supportsBitmapTools；工具条允许 Art 裁剪，但隐藏
橡皮擦、位图效果和栅格化入口。因此 Art 不能按普通钉图整图接管。现先接入 Art 自有
生成图片的本地矢量/裁剪显示快照：覆盖层剔除旧栅格层，避免复制/导出用旧底图覆盖
生成图片；正式输出及下游正式中继不包含该覆盖层。回归验证显示/导出快照保留标注、
正式像素共享不变、下游无本地覆盖、撤销后更新。覆盖模式也纳入快照匹配。
此增量尚未开放 Art 鼠标编辑；新结果尺寸变化、连接回退底图及覆盖层编辑仍待完整适配。

显示来源的模糊组缩放与裁剪后台链已有增量证据：组缩放在真实 worker 后同时更新模糊
sigma 与普通标注几何，结算保留两项选择；裁剪通过菜单及鼠标拖拽，在含栅格标注的
来源上触发模糊重算，完成前不改变裁剪视图，完成后替换模糊像素。两条路径均验证
等待期间保留暂存身份、来源不变、一步撤销，以及 Esc 取消恢复原文档/显示身份。
仅底图裁剪不必重算模糊，不能用这种同步用例冒充后台裁剪测试。所有验证仍为隐藏窗口，
不包含可见画面质量、复杂字体/效果穷尽组合或人工验收。

暂存接管提交后现保留有效单选/多选，将选择索引绑定到提交后的文档版本，不再无条件
清空选择。隐藏 HWND 回归经实际鼠标选择来源标注、保留暂存、缩放及主循环结算，验证
标注变大、选择仍有效、来源不变且一步撤销；另有双标注组缩放保留全部选择的回归。
取消仍清理临时选择。该增量不包含带模糊效果的异步组缩放全链路或 Art 输出编辑。

显示来源的“栅格化全部标注”已接入内容接管，菜单根据来源快照的矢量集合启用；尚未
接管的外部显示不能使用目标旧选择执行“栅格化选中”。隐藏 HWND 菜单→真实后台渲染→
WM_TIMER→主循环回归验证生成非空透明标注层、矢量移除、来源矢量保留和一步撤销；
Esc 分支验证任务回收后恢复原文档与显示身份。复杂效果/文字的全部组合尚未覆盖。

继承裁剪的重置命令已适配显示来源：菜单依据显示快照而非旧底图启用，RESET 先通过
内容接管再清除裁剪，命令后同步裁剪窗口。未知命令和分组锁定不触发修改。隐藏 HWND
菜单回归验证无底图钉图的重置项启用、主循环结算后显示完整图、来源裁剪不变，以及
一次撤销后重新显示来源裁剪。该证据不包含复杂模糊标注的裁剪后台计算或完整拖拽体验。

外部显示的异步擦除和整图翻转已有生产链回归：隐藏 HWND 接收鼠标/菜单命令，真实
后台任务及 WM_TIMER 交付后由应用主循环结算；工作未回收时保持暂存身份。成功路径
检查擦除 alpha、翻转像素顺序、来源不变及一次撤销恢复；Esc 路径检查取消后原文档和
显示身份恢复。翻转菜单现允许无底图的钉图显示来源，并通过内容命令接管，不受当前
Pan 工具的“不接管”规则误拦截。尚未验证可见交互、复杂标注翻转接管组合及裁剪/缩放
全链路，Art 输出编辑仍未支持。

暂存接管现已进入桌面生命周期：State 持有事务及原显示快照，应用在事件返回后的
pending_windows 中结算。模态、草稿、拖动及异步编辑未结束时保留临时身份；未改动
的选择状态也保留，避免选中后立即失去目标。取消恢复原文档和显示身份，提交将首笔
与接管合并为一步撤销。显示解析不覆盖暂存窗口；正式图片/参数输入拒绝未结算接管。
隐藏 HWND 首笔回归现验证一次撤销恢复无底图；另有模态/草稿等待、取消恢复显示身份、
选中保留及取消选择回归。异步擦除/裁剪/变换全过程及真实文字对话框尚需端到端验证，
不将这些准入与模型证据扩展为全部工具已完成。

暂存接管内核已新增 `Editor::begin_adoption` / `Adoption::finish`：原 Editor 连同历史/
重做分支由独立 owner 保留，临时编辑使用来源快照。无已提交变化时结算恢复原身份与
完整历史，存在变化时将最终内容一次性加入原历史，实现接管与首次编辑的一步撤销。
草稿未结束时保持 Pending；文档身份被替换时不能迟到覆盖新文档。临时编辑预算同时
计入原历史仍持有的点数。四项模型回归覆盖取消恢复重做、一步撤销、替换隔离及预算。
该暂存机制尚未接入桌面 owner；当前鼠标路径仍使用直接接管，取消和首笔历史体验尚未
改变。接线时必须等草稿与异步编辑任务全部结束再结算，不能因鼠标抬起就提前退休任务身份。

显示接管准入已统一覆盖普通按下和文字插入/命中编辑/F2 入口：分组锁定、模态框占用、
擦除/裁剪/栅格化/缩放/翻转后台任务期间不接管。文字坐标先按显示视口和来源裁剪检查，
图片外或非有限坐标不新增接管历史。回归验证被阻止入口保持原图、编辑凭证及显示身份，
图片外点击不产生历史、图片内可接管。没有打开文字对话框；取消后的接管回滚、首笔历史
合并和完整文字交互仍未完成，不应把准入测试当成模态交互验收。

钉图鼠标编辑已接入 `display_edit.rs`：WM_LBUTTONDOWN 在繁忙检查后、工具处理前
接管外部显示快照，保留视口、目标样式和旧历史，清理旧选择/命中缓存。平移不接管，
分组锁定拒绝接管；Art 外部输出暂不允许通过此入口编辑，避免修改不可见底图，完整
Art 编辑覆盖语义仍待实现。真实隐藏 HWND 的按下/抬起回归验证笔画落在上游像素上、
来源文档不变，撤销笔画及接管后恢复无底图并重新解析上游显示。栅格同步在模型无栅格
层时清除兼容图片视图，避免撤销后残留已接管图片。当前接管与首笔是两个历史操作；
取消/文本对话框、双击、所有变换工具和撤销体验的主干等价性仍需补齐，不能算完整编辑验收。

编辑接管的事务内核已新增 `Editor::adopt_snapshot`：将来源已提交矢量、栅格层、裁剪
和序号状态作为一次可撤销变更，不接管来源历史/草稿，不覆盖目标工具样式。原有历史
继续保留，撤销可恢复空底图；重做分支丢弃和 50 项历史淘汰释放对应资源。回归覆盖
接管前后编辑顺序、裁剪、共享像素、来源隔离、过期 revision、草稿拒绝、点数预算与
历史淘汰。这仍是同步模型入口，尚未接入鼠标编辑；异步调用者还必须核验文档身份，
不能将局部 revision 当作跨文档身份凭证。

当前显示图的复制与 PNG 导出已接入共享 `display_snapshot.rs`：优先捕获解析快照及
来源标注/裁剪，没有快照时使用本地图；复制菜单和保存入口允许无底图但有显示结果。
快照拥有独立代际令牌，复制完成及保存框返回时同时检查目标文档和显示代际，断链、
换源或替换快照均不能交付旧结果；凭证不持有图片像素。新增回归通过实际 PNG 编码/
解码验证外部显示像素和标注裁剪，并检查同像素新代际、显示消失和保存期间换源拒绝。
测试不访问系统剪贴板、不打开保存框或写用户文件。真实交互与编辑目标仍待完成。

最新显示接线：WM_PAINT 现选择解析快照或原有本地图路径，钉图 fit 和透明命中区域
同步读取快照的像素、裁剪与标注。换源时清空命中缓存，避免不同来源具有相同局部
revision 时误复用区域；来源借用释放后才调整视口或排队重绘。隐藏 HWND 回归验证
生产 WM_PAINT 成功、原图/编辑凭证不变，以及实际窗口区域包含图片内点、不包含外点。
这些证据不包含最终像素比对。外部显示图的编辑目标、导出和完整交互仍未适配，属于
本分支明确未完成项；下文“尚未接入 WM_PAINT”为此前阶段记录，以本段为最新状态。

`Canvas::paint_display` 现可将解析快照绘制到窗口既有 D2D 目标：共享原有纹理/效果
缓存，使用来源裁剪与标注，不替换文档编辑器，不带入目标选择手柄、草稿或擦除预览。
绘制失败也配对 EndDraw；恢复本地图时使用原有 paint 路径。隐藏 HWND 回归直接调用
该入口绘制标注快照、撤销后的快照，再绘制本地图，验证三次完成且原图/编辑凭证不变。
这不是像素比对或可见用户验收；WM_PAINT 尚未选择此入口，透明命中区域、视口与编辑
目标需要同步适配后才能启用，不能宣称窗口显示联动已完成。

显示解析现已接入应用状态：`application_display.rs` 在结果提交后解析工作区来源，
`display_frame.rs` 持有独立显示快照，复用共享像素；本地来源附带已提交编辑快照，
不覆盖目标原图、编辑器或撤销历史。配置、拓扑、编辑版本、正式结果与预览像素身份
未变化时，`application_display_cache.rs` 跳过图重建与快照捕获，不新增刷新定时器。
图片自动链回归验证 Art 与中继钉图共享正式图片、原图保持为空、重复同步直接命中缓存。
这是显示数据归属的接线，尚未将快照用于 WM_PAINT，也未完成外部显示图的编辑、导出
及命中坐标适配；不能称为可见显示同步已交付。本地标注快照的完整交互仍待验证。
显示快照变化或断链清除后现仅排队窗口失效区域，不同步绘制、不擦背景；缓存命中不
排队重绘。隐藏宿主回归覆盖断链清除、重连恢复、Art 切换为
本地标注来源、撤销后的快照更新和旧快照隔离；目标原图仍保持不变。隐藏窗口不提供
可见重绘验收，实际失效区域和最终像素仍须在可见绘制接线后验证。

显示来源解析已新增 `Graph::canvas_image`，对照主干 `canvasDisplayResolution.ts`，与
执行图片解析分离：未编辑钉图优先连接的正式输出，Art/已编辑节点优先自身预览，随后
按输入是否禁用回退上游或原图。停止中继与禁用输入不等价：停止中继且没有本地预览
时，主干仍允许显示连接图，但不能据此重新启用执行中继。回归覆盖这些优先级、预览
不冒充正式输出、输入别名、无关端口、缺失节点和循环回退。该解析只借用图片引用，
桌面像素 owner 的后续接线见上段；WM_PAINT 尚未使用解析快照。

执行策略现补充实际网络抑制证据：两节点回环链分别通过生产菜单关闭来源 notifyDownstream、
目标 listenUpstream、目标 upstreamDriven，首节点仍可手动联网并正常完成，下游保持
Idle，没有待启动/准备/执行请求，没有注册项，全部任务槽为空；服务端监听器在 UI
完成消费后仍无第二次连接。三组禁用用例与直接标量链、钉图图片链共五个网络回归通过。
这不覆盖参数触发开关的所有并发编辑情形，也不替代持久化或真实用户服务验收。

当前能力选择现拥有节点级执行策略，初始化复用能力默认规则（包括高成本 MCP 图片
搜索默认关闭参数触发）。原生菜单提供四个独立勾选项：上游变化触发、参数变化触发、
接收上游执行通知、完成后通知下游。运行入口和正式结果传播读取节点策略而非重新
使用目录默认值；策略变化纳入配置凭证并请求退休旧执行，不自动执行新任务。隐藏
窗口回归覆盖四项切换、菜单勾选、锁定/无效 HWND、旧凭证失效、取消令牌与不自动
排队。手动运行不受自动触发开关阻止，接收通知不等于钉图接收图片。策略尚未持久化。

正式图片完成通知现可穿过钉图中继到下一 Art：`Graph::notification_targets` 复用图片
端口与正式来源解析，最多访问 128 节点，重复边和导入环不重复入队；不使用显示预览，
非图片输出、停止接收和本地编辑的钉图不继续中继通知。应用仍在队列入口阻止反馈环
自动执行。新增 Art→钉图→Art 回环 TLS 回归，只手动启动首节点，首节点返回正式 PNG，
下游自动请求的 PNG 解码像素与首节点一致，最后结果和任务回收通过。钉图无需复制
底图来传播，当前仍未同步它的可见显示层；这不是完整画布联动或多分支图验收。

直接两节点自动链现有生产网络回归：两个独立隐藏宿主，仅对第一节点调用运行菜单；
自建回环 TLS 服务接收第一请求并返回正式值 7，生产提交路径自动排队第二节点，第二
请求实际携带派生参数 7 而不是手动默认值 1，返回 14 后两节点均 Completed，注册请求、
配置绑定和任务槽全部回收。UI 测试只在 worker 完成消息到达后推进应用循环，没有
手工调用第二节点启动或传播方法。第二节点手动值仍为 1。此证据仅覆盖直接标量链，
不是实际用户服务、可见画布或全部多分支工作流验收。

正式结果成功提交后现通知直接相连的下游 Art；失败、过期交付和预览不走该入口。通知
在结果借用释放后排队，复用 Upstream 触发类型、能力默认策略、本地预览门槛及自动
缺图准入；重复通知合并到待启动槽，已有手动请求保留优先级。若目标也是来源的祖先，
说明导入图存在反馈环，当前拒绝该自动通知以避免无限执行（手动操作不受此规则替代）。
隐藏窗口回归覆盖直接通知、手动优先、分组锁定和导入环终止；直接标量链网络证据见
上段。经钉图中继的通知已有上述增量；显示同步、节点级策略编辑和完整多节点自动工作流仍待完成。

待启动请求遇到旧执行或输入/执行任务槽繁忙时现保留最新配置，不再取走后直接丢弃；
过期配置仍立即丢弃。已编码的待执行请求在执行槽满时保留。应用循环改为先领取、回收
已有任务，再调度排队请求，使最后一个完成通知也能推进等待者，不新增定时器或忙轮询。
显式“停止节点执行”同时清除尚未启动的请求，与参数编辑仅退休旧执行区分；隐藏窗口
回归覆盖繁忙保留、停止移除及释放繁忙状态后继续准入。独立进程级 worker 配额冲突、
公平性与高频多窗口压力尚未穷尽验证，不能宣称所有繁忙/重调度边界已完成。

高级参数编辑成功提交后现会排入最终参数变化请求；取消旧执行与保存参数先完成，图片
准备和网络启动仍由应用主循环负责。待启动槽合并为最新配置，已有手动请求保留手动
优先级。请求显式携带 Manual/Parameter 触发类型；参数触发按能力默认执行策略和
本地预览门槛准入，缺图时静默跳过，不套用手动缺图豁免。生产回归验证缺图自动请求
不建任务/不联网，输入齐备的参数请求经过实际回环发送及结果回收；编辑事务测试验证
保存后捕获的是新参数版本。节点级策略编辑、上游完成驱动、连续编辑压力验证及
完整自动图传播仍未完成，不能把这一参数入口称为全图调度器。

手动缺图行为已按主干 `useNodeParameters.ts` 第 247 行的非手动触发条件修正：显式
运行只发送当前实际解析到的图片，缺少声明输入（包括 required 标记）不在客户端阻止
派发，也不填充伪图片。格式不支持/超预算的已提供内联输入仍报错。生产菜单回环用例
验证声明必需图片但未连接时发送空 inputs 并接收结果；无效内联图另行验证不创建任务。
此豁免只属于手动运行；参数触发已接入 missing_execution_images 检查，上游触发仍待
接入，不能把当前入口当作完整自动调度实现。

声明图片参数中的 `data:image/...;base64,...` 已接入执行输入，连接解析仍优先于手动值。
UI 只捕获最多 1 MiB 的单条内联引用；后台输入 worker 复用已有 WIC 解码边界，再编码
为正式 PNG 输入，取消、全局像素预算、批次解码后 256 MiB 和编码预算仍生效。载荷
显式区分 Raster/Encoded，不以虚拟像素代替图片；内联图不会叠加目标窗口标注。生产
菜单回环用例同时发送连接主图和手动 reference_image，服务端解码核对不同像素。
无效 Base64、取消和外部 URL/file 引用有拒绝回归；不读取参数路径或访问网络。格式
准入沿用 PNG/JPEG/WebP/GIF/BMP，实际解码仍取决于已有 WIC 支持；并非全部格式已逐项
验收。文件路径、远程 URL 和主干完整缺图策略仍待迁移。

钉图中继的 `acceptUpstream` 已接入窗口状态、原生“接收上游图片”勾选菜单和图片图
适配器，默认接收；关闭后使用该钉图自己的本地图像，不继续中继上游。开关变更进入
配置/依赖凭证，主循环据此拒绝旧准备和执行；菜单回调只修改状态并唤醒，不等待线程。
隐藏窗口回归覆盖菜单勾选、分组锁定、无效 HWND 和凭证失效；生产菜单回环用例构造
不同颜色的上游与本地图片，验证关闭中继后发送本地像素。该开关尚未持久化，画布中的
实时上游显示同步、输入禁用哨兵及其它传播策略仍待迁移。

原生运行入口现按主干 `artCapabilities.ts` 的能力元数据区分 shader/surface 本地预览
与正式执行，复用执行门槛；只有显式 requiresFormalExecution=true 或 workflow 类型
才允许预览能力继续正式派发。不根据 framework ID 猜测、不把字符串 true 当布尔值。
纯本地预览当前明确提示运行时未接入，不启动输入任务或后端请求；这不是本地运行时
已完成。目录执行配置也已支持真实 JSON 对象及高成本 MCP 图片搜索的默认参数触发
策略，但节点级策略编辑、自动触发和完整运行时仍待迁移。

正式 JSON 输出现可通过 `Output::from_json` 借用适配到图参数解析器：标量直接规范化，
容器不复制整棵树；包装字段只解包一层，false/0/空字符串不会被后续字段覆盖。数组的
文本/数字转换和有界 JSON 消息编码复用现有转换器，结构深度、节点数及字段名预算
同样适用。回归覆盖正式 JSON 包装值经图连线转换为数值参数、容器借用身份、数组
文本化和预算拒绝。桌面运行入口现通过 `application_parameters.rs` 读取上游正式标量
输出，复用 `Graph::effective_params` 的顺序、包装值和控件转换规则，编码为独立配置；
不回写手动参数，不使用显示预览，也不把图像像素冒充路径字符串。生产菜单回环测试
验证上游包装字符串 `12.5` 按 number 控件上限转换为 `10` 并实际发送，手动值仍为 `2`。
准备批次现可独立持有节点/正式输出依赖，即使没有图片输入，也会在领取、启动和执行
期间核对上游版本；回归覆盖零图片批次在上游正式标量变更后失效。标量正式端口已接线，
图片到参数引用、完整中继策略、自动图调度及可见连线 UI 仍待完成。

### 当前图片运行入口（覆盖下文早期阶段的接线状态）

原生运行菜单现通过 `application_image_inputs.rs` 将当前文档、工作区连线及正式输出
适配到已有图解析器，支持本地钉图和上游正式图片输出。钉图输入捕获已提交编辑快照；
正式输出不重复叠加来源窗口标注，也不使用 Art 显示预览代替正式结果。解析完成后最多
接受 32 个图片输入；显式手动运行允许缺失图片，不发送伪输入，提供无效内联图仍失败。

准备批次除来源凭证外，还记录目标及整个上游子图的编辑版本和节点配置身份，以发现中继
新增本地编辑、切换能力或修改参数导致的来源改变。节点依赖凭证只持有版本元数据，
不复制目录、参数或像素；只提供 Editor 的旧校验接口不能放行节点配置依赖。回归覆盖
能力选择/重选/清除及参数变更失效、无变化设置保持有效。依赖遍历排除下游和无关文档，
但保留当前停止中继的上游节点，避免其状态改变后漏检改路；导入环去重终止，最多包含
128 个节点。适配器也只构建该子图的快照，避免复制无关端口元数据。生产菜单回环测试
验证准备完成后无关文档编辑身份改变不取消执行。拓扑凭证仍覆盖整个工作区，新增或
删除无关对象/连线依然可能取消执行；尚未实现子图拓扑凭证。
拓扑、配置和正式输出版本继续分别校验，不以图片像素仍存活代替来源有效性。

隐藏 HWND 回归通过生产菜单解析真实工作区连接，后台编码图片，并从正式执行消息中
解码 PNG 核对像素；同一用例现继续通过生产调度器连接自建回环 TLS 服务，由服务端
再次解码并核对图片，返回标量结果，再验证正式结果提交、请求结束、任务槽回收和配置
绑定清除。无图片能力亦有回环执行证据；没有连接实际用户 Loom 服务，也未证明图片
结果可见呈现。尚未完成可见连线 UI、手动路径/URL 图片引用、标量连线参数、全部中继状态
适配及实际用户 Loom 服务验收；不能宣称完整工作流已迁移。

以下内容保留各阶段实现与验证边界；涉及“入口尚未接入”的早期描述，以本节为准。

工作区增加 `TopologyStamp`，覆盖对象集合和有向连线：成功增删对象、增删/替换连线及
清空时更新；失败事务和无变化操作保持凭证。独立工作区即使内容相同也不共享身份，
避免重建后误用旧凭证。回归覆盖 connect 替换、循环拒绝、重复 ID、级联对象清理和
空操作。该凭证现绑定准备计划与注册后的执行配置：启动调度捕获，准备退休/领取、
执行启动及每轮结果消费前重新核验，拓扑变化会结束匹配旧请求并交由任务集合取消。
回归验证参数未变化时，仅拓扑改变也会丢弃准备和退休执行。当前采用全工作区保守
失效策略，新建无关节点也可能取消任务；尚未优化为依赖子图范围，完整图片来源解析
和用户连线 UI 仍未接入，不能将凭证绑定等同于完整图执行已完成。

输入来源凭证现区分 Document 与正式 Output：`Source::capture_output` 接收正式图片
快照，用空编辑器编码，避免重复叠加来源窗口标注；凭证随输入批次传入执行任务。准备
注册、排队启动、运行期间退休、预览消费和结果提交均使用对应 owner 校验；只提供
Editor 的旧接口对正式输出来源拒绝放行。最终提交前先短借用检查依赖，再借用目标
结果，避免自引用结果依赖触发 RefCell 重借用。回归实际编码/解码 PNG，确认保留
原像素、上游换结果后输入失效；菜单的图片来源选择/图解析仍未接入。

正式结果 owner 新增独立 `OutputStamp` 和正式图片来源快照。成功提交输出时替换凭证，
预览、进度、取消或拒绝旧请求不会替换它；同名节点重新创建也不能复用旧凭证。图片
来源只读取正式端口，与显示预览隔离，像素使用共享 owner，不复制整图。回归验证预览
与正式图不同、别名共享、旧快照安全存活但凭证失效。凭证已接入准备及执行依赖检查，
但运行菜单的来源解析仍待完成，不能宣称用户已可运行完整上游图片链。

“运行所选能力”原生菜单已接入应用后台准备调度：捕获配置身份、主循环核验、共享固定
Loom 身份、生成请求/准备 ticket，再通过现有输入任务和正式执行队列继续。首次入口
仅支持无图片输入能力；完整输入声明存在图片端口时明确报错且不创建任务或连接，不能
把当前预览冒充输入。忙碌、锁定、过期配置不启动，当前执行超时预算为 120 秒，尚未
迁移主干全部超时/运行策略。隐藏 HWND 回归通过生产菜单完成准备和正式消息生成，
验证 secret 排除、配置绑定及需要图片时无连接。现进一步接入自建回环 TLS 服务，
生产后台发送正式 execute 消息，服务回传标量结果，应用仅在 worker 唤醒后消费并提交，
验证 Completed、正式输出值、任务槽回收和配置绑定清除。此证据覆盖测试服务上的
无图片执行闭环，不等同于实际用户 Loom 服务验收，也不证明结果已经绘制到可见画布。
图片来源、图连线和按类型参数控件仍是必须完成项，不能把此零图片阶段当作整体完成。

准备结果成功注册时，应用现将配置凭证与完整执行请求令牌一起保存在文档中。每轮主循环
在消费 worker 更新/结果前退休过期配置请求，再由任务集合取消旧 worker；启动排队请求
也校验绑定。已经选择能力的文档缺少绑定时拒绝启动。正常完成、取消及启动失败只清除
匹配令牌的绑定，旧绑定不能结束新代际或覆盖新结果状态。模型回归覆盖匹配请求退休与
旧/新代际隔离；真实服务和用户启动入口尚未验收，图依赖传播仍未接入。

能力输入/输出端口现由 `loom_capability_ports.rs` 统一规范化，保留 type/data_type/
execution_type 优先级、默认值及可见性元数据。按主干 `artPorts.ts` 区分同名参数控件
与可见连线端口：同名标量隐藏，同名图片只在主输入或 exposePort=true 时保留；独立
端口仍保留。完整输入声明与可见端口分别访问，不能用隐藏规则删除实际执行所需输入。
参数默认值类型转换已复用此端口视图。缺失/空 inputs 的区别有模型测试，但当前握手
边界仍要求输入数组；可见连线 UI 与运行时来源解析尚未接入。

固定 Loom 身份现使用不可变 Arc 存储，目录和后台任务可共享同一份已验证 endpoint、
实例、凭据和证书，而不是重新发现或逐任务复制敏感数据。Debug 仍脱敏、没有新增
序列化接口；最终 owner 释放后存储回收（不宣称内存安全擦除）。回归验证共享分配、
跨线程持有、拒绝端点覆盖、最终释放，以及原 owner 释放后使用克隆身份完成回环 TLS
能力握手。用户启动入口仍需把这一身份与已捕获配置一同交给后台准备计划。

准备计划现携带配置凭证（文档身份、本次能力选择身份、参数 revision）。主循环先丢弃
凭证过期的计划，再按 ticket 取消/退休输入任务；领取结果时再次核对配置，不能只凭
ticket 相同就注册执行。检查不复制参数或身份字符串。回归覆盖文档切换、能力重选、
参数变更、无变化编辑及清除选择；旧计划不依赖取消消息也会被丢弃，测试不连接实际服务。
该凭证只覆盖准备阶段，尚未完成用户启动入口、全部执行阶段和图依赖传播的配置绑定。

公共参数 owner 现可通过 `Configuration::from_parameters` 捕获正式执行配置；保留 JSON
值类型，省略未设置项，`__DISABLED__` 仅进入 disabledParameters，secret/内部字段仍由
参数 owner 排除。图准备入口与此入口共用图片端口校验和配置结尾编码，避免独立协议格式。
回归覆盖捕获后编辑/销毁原状态不改变消息、输入不匹配时保留旧注册请求，并从隐藏窗口
实际参数提交结果生成正式 execute 消息。这里仍是本地编码证据，不代表菜单已启动网络
执行；选择版本与准备 ticket 的绑定、启动入口和实际服务验收仍未完成。

原生右键菜单已接入“高级参数（JSON）”模态编辑。只展示已设置的公共参数，省略表示
未设置、null 保持显式空值；整批检查通过才提交，新添未知字段和 secret 字段拒绝，
会话恢复时已有的未知公共保留值可编辑或删除。编辑快照绑定
每次能力选择的独立身份及参数 revision，弹窗期间切换能力或发生其他修改后旧快照不能
覆盖新状态。成功变更才请求取消旧执行，无变化或失败不取消。原生对话框与隐藏 HWND
回归覆盖有效/无效 JSON、凭据拒绝、旧版本拒绝、切换选择拒绝和锁定/取消边界。
这是高级编辑入口，不是主干按类型控件的完整替代；数字框、复选框、选项控件和执行接线仍待完成。

资源回收验证记录：一次全量测试出现 `cancellation_interrupts_read_without_waiting_for_long_deadline`
等待 3 秒超时；该用例随后独立连续 20 次均通过，尚不能确定原失败根因。新增取消先于
worker 读取及完成一次读取后再取消的回归，并让原用例在断言前回收 worker，避免测试
失败时遗留后台读取。没有根据未复现现象修改生产 socket 行为，不能宣称该偶发超时已修复。

原生右键菜单现提供能力选择子菜单：目录与发现身份共享不可变快照，提交选择前校验
目录仍是同一版本；按主干语义仅 enabled=false 禁用，缺失默认启用，分组锁定时拒绝选择。切换能力仅保存
当前文档选择并捕获旧执行取消令牌，不自动准备图片或执行网络请求。隐藏窗口回归覆盖
旧目录拒绝、锁定拒绝、选择勾选、重复选择无操作及取消令牌捕获。参数编辑、启动入口
和目录更新订阅仍未完成；此阶段不代表完整节点执行 UI 已交付。

能力选择已共用 `loom_capability.rs` 的规范化借用视图；label 缺失回退 id，去除首尾空白
后的重复能力 ID 在握手边界拒绝。参数视图对齐主干的显式默认值（含 null）优先级、
目录 defaults 回退、widget 推导、选项和数值范围等元数据；默认值借用目录，不复制
整棵 JSON。type/data_type/secret 三种凭据声明都排除默认值，同名声明只要一项为 secret
便全部排除默认值。节点初始值转换现由 `loom_capability_defaults.rs` 实现：按 schema、
规范化输入端口、参数类型的顺序选择类型提示，支持 nullable type 数组和组合 schema；
只有可转换的字符串变成数字/布尔值，空白、无效字符串及非字符串值保持原值，不应用
控件 min/max 截断。复用现有数字转换，secret/内部字段不进入公共初始配置；schema
遍历有深度和节点预算，超限失败而不是静默使用另一类型。默认值转换回归已覆盖，
初始化结果现由 `loom_parameter_state.rs` 持有，实际选择能力时先完成公共参数初始化，
再捕获旧执行取消请求并提交新选择；失败保留原选择及原执行。状态区分未声明、未设置
和显式 null，只允许修改已声明公共参数，变更成功才增加局部 revision。参数数量、
JSON 深度/节点总量和编码字节均有预算，拒绝修改保持原值；初始化在克隆 JSON 前检查。
此 revision 是参数 owner 内的局部版本，现与文档/能力选择身份一起绑定异步准备 ticket，
不能脱离选择身份单独当作全局配置版本。
高级 JSON 编辑入口已接入，按类型参数控件与执行配置接线仍待实现，不能直接发送原始 defaults。

能力目录后台任务和原生“加载节点能力”菜单已接入应用主循环，任务全进程最多一个，
后台读取发现配置并通过固定身份获取目录，目录与身份一起保留。文档共享同一会话目录，
菜单显示未加载/加载中/失败/能力数量；失败不覆盖旧目录，不自动重试。退出停通知并
回收任务。回环 TLS 测试验证后台完成通知、领取一次及完成后取消拒绝交付，菜单唤醒
失败回滚也已覆盖。尚未验证真实用户发现文件与实际 Loom 服务；能力选择入口见上文。
发现文件读取受字节上限约束，但操作系统文件 I/O 不保证可即时取消；不能把网络超时
当作整个发现加加载流程的硬期限。

能力握手网络交换已接入现有固定身份认证客户端，包含连接阶段在内设置 10 秒总预算，
消息/帧限制 1 MiB；无关事件和 Ping 不刷新期限。等待响应时以 250 ms 读取片段检查
取消，取消/失败通过 socket owner 析构关闭连接，不创建自动重试。真实 TLS 回归覆盖
握手消息、通知忽略、Ping/Pong、正式目录返回，以及连接前取消无连接和响应等待取消。
取消不能保证整个过程 250 ms 完成：连接、TLS 握手和发送分别仍受其操作预算限制。
后台目录任务和原生选择菜单已接入；目录更新订阅仍未实现。

能力目录数据边界已按旧 `loom_otter/handshake.rs` 与 `protocol_models.rs` 建立：
正式 `loom.hook.handshake` 请求声明 websocket/shared_memory，响应限制 1 MiB、
目录最多 512 项、单能力参数/输入/输出最多 128 项，拒绝非法身份、重复能力 ID、
重复已知顶层字段和不受支持 transport；通知事件不作为握手完成。保留 Art 原始字段
及 secret 声明供后续参数适配，不能直接把目录 defaults 全部发送。新增回归覆盖正式
请求字段、目录保真及异常边界；认证握手、目录后台任务和选择菜单的接入边界见上文。

右键菜单已增加“停止节点执行”，仅在有准备计划或执行请求时启用，分组锁定不阻止
取消生命周期操作。菜单回调只捕获当前 ticket/完整请求令牌并发出消息，主循环再取消
准备、清除匹配的待执行请求、退休运行 worker；旧取消命令不能结束新代际。
回归覆盖唤醒失败回滚，以及隐藏 HWND 通过生产菜单入口取消准备/执行和旧取消拒绝。
实际远端取消仍由既有后台执行 owner 完成；此入口不表示可见启动/能力选择已完成。

应用现会退休不匹配当前目标/ticket 的输入准备任务，取消后仅在 worker 已回收时丢弃
结果并释放槽位，不等待编码完成来处理窗口事件。回归覆盖旧任务和同目标新任务并存、
只回收旧 ticket、重复回收，以及真实隐藏应用中旧结果不消费新配置、不创建请求或联网。
配置替换/取消的可见 UI 入口仍未实现，后续入口必须生成新 ticket，不能修改快照后复用旧 ticket。

不可变配置与准备 ticket 已通过文档待准备计划配对。主循环领取匹配结果后重新验证
文档凭证，成功编码并注册后写入待执行槽；槽已占用或准备/编码失败只写准备错误，
不覆盖其他待执行请求。应用集成测试覆盖后台图像准备、配置配对、正式 Art ID/代际
编码以及此阶段不联网。旧 ticket 的取消/回收和用户修改配置的入口仍需进一步绑定，
未匹配报告现由应用退休并在 worker 完成后回收；仍不能宣称完整配置生命周期已完成。

正式协议现支持独立于图借用的 `Configuration` 快照，保存已筛选的公共参数编码、
禁用项、Art ID 与最多 32 个图片端口名；参数编码上限 4 MiB，不复制图像数据。
旧借用入口也通过同一快照编码实现，避免两套协议漂移。回归在原 Graph/Params
离开作用域后生成完全相同的消息，核对 Unicode/转义、参数、禁用项，并验证缺少输入
时不替换旧请求。快照暂未绑定配置版本/ticket，也未由生产 UI 持有到准备完成。

输入准备任务集合已接入应用：最多两个槽，完成未领取仍占槽；每项保留准备 ticket
和目标身份，关闭目标只取消，完成领取时拒绝已退休结果，退出先取消全部再回收线程。
隐藏窗口图像链改为通过应用集合启动/轮询/领取准备结果，覆盖生产生命周期 owner，
不再由测试单独持有准备 Job。新增回归覆盖重复目标、满槽、完成后关闭、领取释放槽位
及退出回收。配置与能力 UI 仍需发起准备、绑定 ticket 并在领取后创建正式执行计划。

图像准备 worker 现支持完成通知，与执行 worker 共享完成栅栏：释放并发许可后发布
ready，再唤醒 owner，避免通知先到而 `is_finished` 尚未成立造成漏领。新增回归验证
收到通知即可领取、许可已回收且结果只领取一次；隐藏窗口图像链测试也改为通过同一
应用唤醒通道领取准备结果，不再主动轮询准备任务。生产准备调度尚未接线；join 仍可能
等待通知回调和线程最终退出调度，不能宣称严格零等待。

`loom_execution_prepare.rs` 已串联图准备结果、输入批次与正式 Loom execute 编码：
超时、文档新鲜度、资源描述符及编码检查完成后才更新请求代际，不提前清除旧请求。
隐藏窗口集成测试现通过此入口生成真实协议消息，服务端验证 method、节点、请求、
generation、artId 及 inputs；零超时、过期输入、非法 artId 均不创建活动请求。
该集成已扩展到真实图像：后台 WIC 输入任务准备 PNG，正式协议发送后服务端解码核对
尺寸和像素，再将同一正式图像描述符返回，应用解码后提交到两个图像输出别名及显示
状态，并检查原文档未被替换。输入准备仅由测试调度，能力目录 UI、生产图像准备调度
及配置版本绑定仍待接线；节点显示状态并不等于已绘制到可见窗口。

文档状态现有单槽待执行请求，应用在 UI 回调返回后统一领取：核验来源 UnitId、注册
令牌及全部输入文档凭证，才启动有界执行任务；过期准备/启动失败结束匹配请求并写入
失败状态。新令牌会退休旧 worker，但其未完成清理仍占槽。该启动接线尚无可见能力
选择/执行按钮，也没有自动触发网络。新增应用集成测试创建真实隐藏 HWND 和自建 TLS
服务，通过待执行槽启动，只在后台唤醒消息到达时消费报告，验证完成状态、正式输出、
请求结束、任务回收及原文档不变；准备后文档替换用非阻塞 listener 验证没有发起连接。
这条证据覆盖应用启动/消费链和图像正式消息准备，不覆盖能力选择及生产图像准备调度。

主循环现已在轮询线程完成后、领取最终报告前消费进度和预览，重新核对请求与所有
文档凭证，写入对应节点结果 owner。完成线程保留单槽邮箱直到应用领取，修复“最后
预览刚写入即完成，join 时邮箱被回收”的时序缺口；取消任务不保留完成邮箱。
真实 TLS 回归将最后进度/预览紧邻最终结果发送，验证 join 后仍可恰好领取一次。
此接线尚未把节点显示图像绘制到可见窗口，不等于可见预览功能已经完成。

应用主循环现已领取清理确认完成的报告并提交到对应文档的节点结果 owner；文档状态
按同一个 UnitId 初始化结果 owner，窗口关闭自然回收其本地结果。任务集合的
`take_settled` 不取走仍待远端清理的报告，保留有界槽位直至显式退出处理；不能声称
远端不可用时已经清理成功。提交失败写入简短状态，不暴露原始响应或认证信息。
成功报告的候选元数据也转交结果 owner，缺失候选字段保留原数据；完整候选缓存合并、
选择、全应用元数据预算仍待实现。可见执行入口与画布预览尚未接线，当前只接入主循环消费。

节点结果 owner 已补充 Idle/Running/Completed/Failed/Cancelled、进度及有界错误文本。
新执行清理上一轮进度/错误但保留画面与输出，旧请求进度和失败不能改变新执行；非有限
进度拒绝，错误最多 1024 个 Unicode 字符。完成报告提供显式 reject 收尾：当前节点和
请求匹配时结束请求，保留 completion 中的结果/清理责任，不会因为文档已变化而无法
结束失败执行。回环测试覆盖提交拒绝、错误节点、过期失败及重复收尾；可见错误提示尚未接入。

完成报告已通过 `loom_result_commit.rs` 接入单节点结果 owner：同步提交前重新检查目标
UnitId、当前请求、文档凭证，以及底层 completion/delivery 身份。待清理租约或执行失败
不消费结果；成功转移正式端口后才结束请求并禁止重复提交。回环 TLS 集成测试已贯通
后台执行、报告领取、节点结果提交和请求结束，覆盖错误节点、替换文档与替换请求拒绝。
候选元数据仍保留在 completion 中，节点预算失败不修改已有输出，但已取出的本地批次
会回收，调用方应将这类失败显示为执行失败而非盲目重试。可见节点窗口尚未调用该入口。

`loom_result_state.rs` 已建立单节点结果 owner，对照 `appArtDeliveryHandler.ts` 和
`artDeliveryOutputs.ts`：预览只更新显示，不写正式端口；最终结果合并旧端口，图像覆盖
`output`/`output_image`，显示优先保留本轮预览；新执行保留原显示和正式输出。
请求代际和预览 revision 拒绝过期更新，合并前验证 128 端口及 256 MiB 保守输出预算，
所有可失败准备在修改旧状态前完成。JSON 预算流式计数，不额外构建序列化副本。
真实 WIC 回归覆盖预览/正式像素分离和别名共享分配；候选状态、图传播、全应用总预算
与实际节点窗口接线仍未完成，不把此 owner 当作完整 Art 节点实现。

正式交付对象已提供一次性的 `take_released`，只有远端租约确认释放后才能移交本地
输出；租约未释放时不消费结果，解码错误同样保留到释放完成。多端口结果通过
`into_ports` 直接转移到后续节点 owner，不克隆 JSON 或像素，重复领取明确返回已消费。
回归检查未释放拒绝后的重试、失败解码的清理、JSON 字符串原分配转移，以及真实共享
映射释放后解码图像像素分配不变。此接口仍未接入节点结果持久状态和可见画布。

应用执行集合已开放进度/预览单槽领取接口，领取时核验当前请求及全部文档凭证，过期
更新只回收本地数据，不发布到 UI；最终报告领取前消费最后更新。完成报告的提交前
检查也重新读取请求注册表，封堵“领取后新请求替换，但缓存的 current 仍为 true”的窗口。
真实回环 TLS 回归覆盖进度和已释放租约预览的单次领取、完成报告领取后的请求替换及
文档关闭/替换拒绝。该接口仍需与实际节点状态和画布显示连接，不等于预览界面完成。

执行任务现保留目标及各输入文档的轻量身份/版本凭证，不额外保留编码图片或像素快照。
应用轮询时检查全部依赖：文档关闭、替换、编辑或出现草稿即取消并退休旧任务，即使
请求令牌尚未变化也不能继续交付。完成报告保留凭证供提交前复核；请求退休状态不能
通过文档检查重新变成有效。回归覆盖输入批次释放后的凭证检查、目标替换、来源丢失、
来源替换及草稿/提交变化，真实网络任务测试覆盖因文档变化退休后的取消和清理。
该检查仍不覆盖图结构、能力配置及参数变化；这些需要后续执行配置版本绑定。

Art 后台执行新增完成栅栏与可注入通知：输入和并发许可释放后发布 ready，正常完成及
panic 展开均通知 owner；更新先释放邮箱锁再通知，避免接收方读取邮箱时死锁。
完成轮询不再只依赖 `JoinHandle::is_finished`，避免通知早于线程退出导致漏领结果。
通知回调必须短小且不可阻塞；join 仍可能等待回调和线程最终退出调度，不宣称严格无等待。
应用现已把该通知接入 Win32 线程消息：同一时刻合并为一个待处理唤醒，读取任务前清除
待处理标记，无空闲轮询定时器；投递失败允许后续重试。正常退出及异常析构先停通知，再
回收任务，防止旧回调向复用的线程 ID 投递。真实线程消息测试覆盖后台重复通知合并、
确认后的再次唤醒和 owner 销毁后拒绝投递，不注入系统输入。模态窗口期间不提交结果，
返回主消息循环后才轮询。这仍不代表 Art 的界面入口或结果提交完成。

翻转基础内核已在 `raster_flip.rs` 建立：水平/垂直镜像逐像素保留预乘 BGRA，
按行检查取消、分配计入共享像素预算，两层全部完成才返回候选，单像素轴无变化共享源分配。
回归覆盖精确像素排列、双翻转恢复、透明栅格层、第二层中途取消和非法缓冲。
此内核只处理完整源图，尚未接入菜单/历史；主干的底图全源翻转、裁剪可见框栅格翻转、
文字保持可读及各类矢量位置变换必须在后续事务中分别适配，不能用整图镜像冒充完整翻转功能。
`editor_flip_geometry.rs` 已增加标注候选生成：盒形对象移动中心但保留自身朝向、样式及角度，
路径和模糊/马赛克遮罩镜像源坐标点，保留框外几何；效果像素仍共享，最终翻转事务需另行重算。
所有目标坐标先校验再克隆候选，失败不修改源对象。回归覆盖形状、旋转、路径、效果、
字体/序号内容不变、双翻转恢复及越界拒绝。序号的主干基线为圆心加半字号，转换后与
原生圆框中心镜像一致；普通文字已移除包围框近似，改为显式 `TextFlipMetrics` 的文本宽度、
左侧内边距和基线偏移，按主干 x'=W-x-textWidth、y'=H-y+fontSize 计算，缺少度量拒绝候选。
回归独立核对水平文本锚点和纵向基线公式。布局层 `desktop::measure_flip_metrics` 已提供真实
DirectWrite 文本宽度与第一行基线（包含绘制的 4 单位内边距），行数组受文本长度上限约束，
COM 对象局部释放，仅返回纯数值供候选准备使用，不跨线程传递接口。真实 DirectWrite 回归覆盖
英文、中文、尾随空格和多行文本，并将结果传入翻转模型验证锚点公式；普通测量与绘制路径不变。
浏览器与 DirectWrite 的完整排版/像素对照、整组历史与宿主翻转仍待完成，
不能把真实字体度量与公式回归作为完整翻转等价证明。
`editor_flip.rs` 已组合未裁剪整图的两阶段事务：准备阶段检查点预算并收集全部几何/文字度量，
计算阶段翻转两层栅格并复用 `RasterRefreshPlan` 重算模糊，最后一次提交全部栅格与标注。
共享历史现可同时持有普通对象和模糊对象的替换，不再假设所有替换项都是效果；无需模糊时不合成源图。
回归验证精确栅格/模糊结果、几何位置、三次撤销重做的原分配恢复、取消/过期拒绝、文字测量中途失败
和点预算失败不修改文档。无栅格化标注层的裁剪文档现可翻转：底图按完整源图翻转，矢量围绕保留源坐标中的裁剪框镜像，裁剪框本身不变。新增偏移/二次裁剪、框外几何、双翻转、重置裁剪、过期候选及历史回归；真实 WIC 编码/解码对照两个方向的逐像素坐标，覆盖导出误用旧 Image 缓存的风险。当前 384 项库测试和 7 项启动测试通过。
带栅格化标注层的裁剪翻转已接入并开放菜单：底图保持完整源图镜像，标注层通过 `resized_image` 缩放镜像到当前可见框的四舍五入尺寸，矢量围绕源坐标裁剪框镜像，随后按新层尺寸/画框重算模糊并一次提交。撤销恢复旧尺寸、原像素和模糊共享资源；无裁剪时也将独立标注纹理适配回完整逻辑框。新增回归覆盖偏移裁剪、2×1 到 4×4 层变换、分数裁剪尺寸、小数模糊、三次撤销重做及重置后的再次翻转；418 项库测试和 7 项启动测试通过。真实 HWND 裁剪效果冒烟已追加后台翻转及目标纹理尺寸验证。浏览器与原生重采样逐像素一致性及可见人工验收仍未完成。
`raster_flip::resized_image` 已提供一次输出分配的缩放加镜像内核：像素中心对齐双线性插值预乘 BGRA，共享全局像素预算，逐行及末尾取消，不修改输入；同尺寸走既有精确镜像。5 项回归覆盖透明边界插值、垂直下采样、单像素扩展、三个阶段取消和非法尺寸，389 项库测试及 7 项启动测试通过。该内核已接入裁剪翻转事务，尚未证明与浏览器 Canvas 的采样逐像素一致。
独立纹理尺寸的数据基础已建立：`Layers.annotation_size` 为可选标注像素尺寸，None 保持既有同尺寸合同；`annotation_dimensions/valid` 统一尺寸与缓冲检查，资源身份包含标注纹理形状。擦除、整层镜像及画框效果源合成已支持标注纹理与底图不同分辨率，并保留标注尺寸到候选结果。普通图片初始化维持 None，裁剪栅格化/翻转可生成独立尺寸标注层；Editor 创建/替换及 RasterRefreshPlan 候选校验现已使用统一的独立尺寸校验；GPU 上传按标注层自身宽高和行跨度创建纹理，缓存比较包含纹理尺寸；PNG 导出按独立尺寸上传，完整拉伸到可见框而非按纹理自然尺寸绘制。新增/扩展真实 D2D/WIC 回归覆盖 2×1 标注纹理到 8×8 底图与 4×2 裁剪框的显示导出、相同字节数切换为 1×2 后只重传标注纹理，以及候选提交/撤销/重做恢复尺寸和共享身份；413 项库测试及 7 项启动测试通过。命中缓存已接入独立尺寸：alpha 扫描使用标注层自身宽高，缓存失效比较包含尺寸，原生区域按标注纹理尺寸映射到可见框；相同缓冲在 8×4 与 4×8 下分别构建不同区域，真实 HRGN/PtInRegion 回归验证两种映射和错误行跨度/零尺寸拒绝。414 项库测试及 7 项启动测试通过。栅格化已贯通独立尺寸：矢量快照保留裁剪框，后台验证各层自身缓冲；已有标注层按自身宽高/行跨度读取并适配到输出框，矢量单独应用源坐标到裁剪框的变换。裁剪输出尺寸按可见框四舍五入并记录独立尺寸，无裁剪时输出底图逻辑框尺寸；底图共享身份保持不变。正式 RasterRefreshPlan 栅格化提交及兼容提交均支持独立尺寸。新增 WIC 回归覆盖 2×1 旧纹理加偏移裁剪矢量生成 8×8 图层、排除未选矢量、三次撤销重做恢复原尺寸/点分配，以及无裁剪 1×1 纹理适配全框；416 项库测试及 7 项启动测试通过。裁剪栅格层翻转现已接入，验证范围见上文。新增回归覆盖同字节数不同形状身份、无效尺寸、独立尺寸镜像/合成和逻辑画框擦除，412 项库测试及 7 项启动测试通过。
进一步核对 `stickerCompositeRenderer.ts:214-216`：主干把栅格化标注层完整绘制到当前可见框，而不是像底图一样保留源框后裁剪。当前原生 Layers 的两层共用尺寸/源坐标，已有裁剪测试只证明原生自身的合成裁剪一致性，不能证明这部分与主干等价。显示/导出已先行修正：`canvas_raster` 为底图和栅格标注分别接收目标矩形，Canvas 把完整标注层适配至可见裁剪框，底图仍按完整源框裁剪；导出同样使用独立目标矩形及线性插值。裁剪及重置仅改变绘制目标，不复制像素或重新上传纹理。原有“全合成图取局部”等价测试已限定为无栅格标注层，新增左右双色标注层的真实 WIC 导出与绘制目标回归，检查缩放后两色仍可见、底图目标不变、纹理身份复用、重置和撤销恢复；391 项库测试及 7 项启动测试通过。仍须补齐独立像素尺寸及擦除坐标、模糊采样、命中区域、栅格化和历史适配：当前这些消费者尚不能正确覆盖裁剪加栅格标注组合，尤其不能把本轮视觉修正当作擦除提交正确性的证明。裁剪栅格层翻转现已接入，验证范围见上文。
擦除内核已新增 `erase::apply_in_frame` 和 `Layers::erase_in_frame`：底图保留源坐标，标注层按独立画框映射路径，并在逻辑坐标中计算笔刷覆盖，正确保留横纵比例不同时的椭圆像素足迹。扫描使用保守圆外包，4×4 子像素取并集规则不变；映射点缓冲纳入像素预算，可取消，非法映射拒绝。回归以独立逻辑坐标逐子像素计算验证非等比/偏移框，并覆盖等尺寸合同、重复段及取消。该接口现已进入宿主 Request：鼠标按下时捕获 `annotation_frame`，后台通过 `Layers::erase_in_frame` 分别擦除源坐标底图和适配画框的标注纹理，预览仍保留到原子提交。新增真实线程测试覆盖两种擦除模式、不同层的实际擦除位置和原始快照不变；Canvas 指针到后台到提交的回归覆盖裁剪框捕获、一次版本推进、裁剪状态不变、撤销共享源恢复及重做。396 项库测试、7 项启动测试通过。`RasterRefreshPlan` 现捕获标注层画框，并通过 `raster_source::compose_in_frame` 按该画框合成候选效果源，擦除后模糊与两层栅格一次提交。新合成器仅分配一个输出缓冲，框外保持底图、框内采用预乘 BGRA 双线性采样，分数边界按像素覆盖面积处理；其分数边界合同尚未证明与浏览器/D2D 完全等价。真实线程宿主回归已加入 2.4 强度模糊，验证提交像素与画框合成后的模糊一致，撤销恢复原模糊共享资源。398 项库测试及 7 项启动测试通过。新建/调节模糊已统一接入：`blur_job::Request` 捕获独立画框，后台调用画框合成；宿主接受结果时同时核验画框，`blur_ready` 缓存也绑定画框，像素身份未变但裁剪/重置改变画框时，指针入口拒绝旧缓存且同步时清除。新增真实线程回归覆盖画框采样、重置后旧缓存拒绝、同版本/同源但画框错误的结果拒绝，以及小数强度调节与撤销；401 项库测试及 7 项启动测试通过。缩放任务也已接入：`ResizePlan` 捕获独立画框，`resize_job` 通过画框合成源图后重算缩放强度，相同强度继续共享结果；提交核验画框、文档身份、版本和源图。新增真实线程回归覆盖偏移非等比裁剪框、准确模糊像素、结果共享、单次版本推进、几何与效果一起撤销/重做，以及计算期间重置裁剪后的过期候选拒绝。403 项库测试及 7 项启动测试通过。裁剪/重置与已提交模糊的原子事务内核已建立：`prepare_crop_refresh` 仅准备新画框，`RasterRefreshPlan::compute` 按新画框重算已有模糊，`commit_raster_refresh` 一次交换画框和效果。历史支持画框加栅格/效果组合，撤销/重做不复制原始像素；无标注层时不做无意义的模糊重算。新增回归覆盖偏移裁剪、重置、三次撤销重做恢复共享资源、取消/过期/非法选区拒绝和无标注图片事务，406 项库测试及 7 项启动测试通过。宿主裁剪指针/重置菜单现已切换到该事务：`crop_apply` 在需要重算效果时启用单后台线程及活动期定时器，旧画框/旧画面保留到提交；无效果重算则同步提交轻量计划。完成后同步栅格缓存、钉图位置尺寸及命中区域；Esc 取消，销毁取消并回收线程，任务与翻转/缩放/栅格化及新鼠标编辑互斥。新增指针到真实线程到宿主接受的回归覆盖裁剪/重置等待、原子版本与历史、过期拒绝和销毁不提交，408 项库测试与 7 项启动测试通过。新增 `crop_effect_smoke` 已用内存中的栅格层加 2.4 强度模糊夹具验证真实 HWND：指针消息启动后台任务，定时器交付前版本/画框/窗口保持原状，完成后一次更新画框和模糊；钉图缩小、普通窗口保持外框，重置异步恢复原尺寸，Esc 取消不修改文档或窗口。普通窗口/钉图两项启动测试均强制要求这条证据；测试结束恢复原编辑器，不读写用户文件或注入系统输入。首次验证修正了测试拖动坐标错误（普通窗口应按图片 viewport 与 DPI 映射，而不是按整个客户区取点）。这不是可见 UI 人工验收或跨屏 DPI 测试；透明命中区域现已分层适配：`HitRegion` 分别缓存底图和栅格标注 alpha 区间，底图按源坐标缩放，标注层按可见框做非等比缩放和平移，GDI 区域合并后再统一施加 viewport 平移和裁剪。不分配合成整图，保留各层一像素线性采样边缘和有界碎片化降级。新增真实 HRGN/PtInRegion 回归覆盖偏移裁剪、非等比缩放、1/1.5/2 倍缩放、原标注位置穿透、底图位置不变及重置恢复；409 项库测试及 7 项启动测试通过。跨进程实际鼠标穿透、极小缩放边缘及 GPU 预览与 CPU 提交的缩放抗锯齿等价仍待专项验证，因此不能声称整个裁剪擦除组合已经完成。
窗口优先级测试的 PENDING 偶发断言本轮再次复现，已调整测试顺序：在读取消息队列之前检查直接调用忽略路径的 PENDING，随后注销真实 WinEvent hook，再检查队列；避免读队列时外部窗口事件进入该断言。此修改仅隔离测试的系统事件干扰，不修改生产窗口优先级策略。
翻转后台任务 `flip_job.rs` 已建立：单任务、无排队，后台仅计算私有候选；取消后即使结果先完成也不交付，销毁时取消并回收线程。最终提交仍由 Editor 校验文档身份、版本与源图，拒绝跨文档和过期结果。新增真实线程回归覆盖忙碌拒绝、取消后重启、计算完成后取消、过期/跨文档拒绝和销毁不发布；377 项库测试及 7 项启动测试通过，Clippy 与有效行数检查通过。桌面 `flip_input.rs` 已接入右键水平/垂直翻转及活动任务定时器；计算时保留旧画面，成功后同步栅格缓存、清空旧选择并更新命中区域。Esc 可取消，鼠标编辑、裁剪、缩放和栅格化入口与翻转互斥，过期候选由模型拒绝。宿主回归覆盖提交像素/选择同步、撤销、活动拖动拒绝和版本过期拒绝。裁剪栅格层翻转现已接入，验证范围见上文。隐藏 HWND 翻转冒烟现已验证水平/垂直翻转、真实 WM_TIMER 完成通知、独立坐标公式逐像素比较、撤销/重做以及 WM_KEYDOWN Esc 取消；普通窗口与钉图两种启动回归均强制要求该证据。设备重建的两次绘制单独输出并断言，不再把包含翻转绘制的累计次数误当成固定 2 次。此验证不注入系统输入、不访问用户剪贴板或配置，但仍不等于可见菜单鼠标操作、跨屏 DPI 人工验收或性能测量。整轮迁移未完成。一次全量测试出现窗口优先级测试 `invalid_destination_does_not_poison_registration` 的 PENDING 断言失败；其安装系统 WinEvent hook 后读取消息队列，可能受外部窗口事件干扰（推断，未完成根因验证）。随后完整重跑 380 项库测试及 7 项启动测试通过，不能据此声称该偶发问题已经修复。

非破坏性裁剪坐标内核已在 `crop_geometry.rs` 建立，对照主干 `stickerEditingGeometry.ts`、
`stickerCropImage.ts` 及 `stickerCropCoordinates.test.ts`：保留逻辑源框，缩放后二次裁剪映射回
源坐标，重置保持显示比例并清除旧源框；图片先按完整源框 contain-fit 再裁剪，保留留白与像素密度。
计算返回标注/擦除轨迹的平移量，不破坏框外几何、不分配图像；非有限、非法尺寸及溢出结果拒绝。
四组回归复用主干预期值并覆盖负坐标及非法输入。
`editor_crop.rs` 已接入非破坏性裁剪状态、重复裁剪、重置和统一历史，导出快照携带同一状态。
原生模型将标注/栅格保留在源坐标，仅保存固定大小的可见框与源框；裁剪不会复制几何或像素，
也不消耗保留点预算。后续呈现与输入必须通过裁剪视图转换坐标，不能直接照搬主干平移后的
可见框标注坐标。回归覆盖框外内容保持、历史交错、一次撤销重做、快照独立、非法/无变化/草稿拒绝。
当前 Editor 尚无整图逻辑框缩放接口，重置回到原始源框；缩放后重置的数学合同仅由坐标内核覆盖。
PNG 导出已读取裁剪源框，用同一个 D2D 变换合成底图、栅格层和矢量，输出尺寸按主干
四舍五入并至少 1 像素，两轴按实际输出尺寸缩放；编码器尺寸以实际合成位图为准。
模糊/马赛克可见区域改为完整目标变换的逆变换，覆盖裁剪偏移和对象旋转，避免裁掉有效效果。
`crop_export_tests.rs` 真实 WIC 编解码逐字节对照整数裁剪与完整合成图对应区域，覆盖栅格、
旋转形状、模糊和马赛克，并验证重复裁剪、重置/撤销、分数尺寸和快照独立。
非均匀缩放的复杂效果与浏览器完整像素一致性尚未证明。
屏幕绘制已在底图/栅格预览/矢量外围使用同一裁剪框；图像输入按可见源框拒绝框外起点、
夹紧拖动坐标并拒绝非有限值。钉图 fit 已按裁剪宽高计算比例并补偿源框偏移；透明命中区域
在合并栅格/矢量范围后统一平移并与裁剪框相交，避免不可见源图继续拦截鼠标。
`crop_input_tests.rs` 覆盖三种视口比例、偏移、框外拒绝、源坐标笔迹以及重置/撤销后的命中恢复。
`crop_input.rs` 已接入右键“裁剪选区/重置裁剪”，裁剪模式中拖动显示双线选框，松手一次提交；
支持反向拖动、连续裁剪、框外夹紧、零面积拒绝、取消与版本失效。草稿只存两个源坐标点，
提交门槛已按主干 pointer commit controller 对齐为宽高均至少 4 个逻辑单位，回归覆盖两轴门槛。
复用手势准入标记阻止拖动期间视口变化；切换工具/换图退出模式，Shift 与双击不再误入标注选择/文字编辑。
输入回归覆盖两次裁剪、一次撤销、取消保留重做、过期手势和模式切换，完整套件已通过。
`crop_window.rs` 已接入钉图裁剪提交、菜单重置以及键盘/菜单撤销重做后的物理窗口同步，
按源框变化调整位置和尺寸，完成后重新 fit 并更新命中区域。Win32 同步消息调用前释放模型借用，
成功后才记录已应用源框；大图恢复受 8192 边长和 16,777,216 像素交换链预算限制，必要时降低比例。
几何回归覆盖负坐标屏幕、四种物理比例、连续裁剪/恢复、大图预算和非法坐标。
`crop_smoke.rs` 已并入 `--pin --smoke-test` 并用仓库 32×32 图标实际运行通过：向隐藏 HWND
发送按下/移动/松开消息，验证拖动不提交、松手一次提交、窗口位置尺寸变化、源像素共享、
真实 GDI 窗口区域不越出客户区，随后实际执行撤销/重做/重置及窗口恢复和 D2D 绘制。
输出 `native-crop-smoke: pointer=true window=true undo=true redo=true reset=true region=true`。
此探针要求窗口至少 8×8 像素，不注入系统键鼠、不读取用户文件；不是可见视觉或跨进程点击验收。
普通编辑窗口保持原窗口大小，只改变可见内容；钉图实际跨屏/DPI、窗口位置恢复和 HRGN 鼠标穿透
尚未完成真实交互验收。选框控制柄调整、源图翻转仍未接入，不能将此作为完整裁剪交互或兼容证明。

新建模糊与手动调整强度的后台任务现已统一使用底图与栅格标注层的合成快照，
不再只采样底图；合成和卷积在同一可取消线程执行，沿用共享像素预算及单活动/单待处理请求。
请求、准备缓存、指针准入及结果交付都验证两层共享身份和尺寸；仅栅格层改变时也清除旧准备缓存，
不会因底图引用未变而复用过期结果。新增线程回归验证覆盖层确实参与卷积、源数据不变与引用释放；
宿主回归隔离版本因素验证栅格层变化后拒绝旧结果/旧输入，重新准备后恢复输入。
上述准备链路不负责既有模糊对象的自动刷新；擦除后的刷新进度见下，实时强度预览仍待完成。
手动调节选中模糊强度现已保留分数值，不再通过整数菜单值计算增量；后台请求和卷积使用
浮点强度，菜单读取实际值，连续请求在待处理强度上累计。选中对象使用内核 1–64 的范围，
新工具默认强度仍保留独立整数配置。`blur_precision_tests.rs` 验证 4.4→5.4→6.4 的最新请求交付、
像素卷积、菜单模型、撤销重做及非有限输入拒绝，不代表实时拖动预览已经实现。

既有模糊随栅格编辑刷新所需的事务内核已在 `editor_raster_refresh.rs` 建立：
准备阶段捕获文档身份、版本、两层源图与全部模糊几何，检查保留点预算；后台计算入口
只合成一次候选源图，同强度结果共享，保留分数强度。全部计算成功后，栅格与效果
作为一个历史项交换，撤销/重做同时恢复像素与几何分配；未计算、取消、跨文档和过期候选
不能提交。回归覆盖三次撤销重做、强度共享及卷积结果。擦除宿主已接入此事务：松手捕获
刷新计划，原擦除工作线程先计算候选栅格再重算全部模糊，不额外启动并行线程；预览持续
到整个结果交付，在同一回调内清除预览并原子提交。无变化擦除不写历史、不清空重做分支。
真实线程到 Canvas 的回归覆盖内容擦除、仅栅格标注擦除、分数强度、计算期间文档不变、
最终合成卷积结果及一次撤销/重做同时恢复。
栅格化宿主也已接入相同事务：WIC 工作线程先渲染选中集合，再重算未被栅格化的模糊对象；
栅格替换、矢量移除和剩余效果刷新一次提交。历史保存移除后的效果索引，撤销先恢复效果
再插回矢量，重做反序处理，避免非连续选择导致索引错位。真实 WIC/线程回归覆盖交错选择、
相同强度共享、最终卷积、反复撤销重做及放弃重做；模型回归覆盖选中模糊不重复重算、
全部栅格化和保留点预算回收。真实窗口视觉无闪烁和大图重算延迟尚待验收，
不能以线程/模型回归代替人工交互与性能证据。

多选的第一条交互链已接通：选择工具中 Shift+点击按主干默认手势增减选中对象，
显示各自选中框，Delete 批量删除，选中范围栅格化处理整个集合。
`desktop/multi_selection.rs` 将有序索引集合绑定文档版本和主锚点，历史或文档修改后旧选择失效。
批量删除事务位于 `editor_batch.rs`。
`remove_many` 接收严格递增的有效索引，一次修改文档版本并记录一个历史项；
逆序移出对象、撤销时按原索引正序恢复，重做再次移入历史，不克隆点数组。
与栅格化复用批量对象历史载体，但删除不修改底图或栅格层。非法索引、重复/乱序索引、
活动草稿及空选择不会部分删除或清空重做分支；历史淘汰释放已删除对象的保留点预算。
回归覆盖非连续选择、反复撤销重做的原分配身份、非法输入和历史容量淘汰。
框选及其余批量命令尚待接入；多选时仍拒绝已有单对象复制，避免只复制集合中的一个对象。
“应用当前样式”菜单与 Ctrl+Enter 已支持整个选中集合：`editor_batch_style.rs` 先验证全部索引，
一次提交/撤销，只记录发生变化对象的属性，不克隆几何或效果像素、不增加保留点预算。
无变化、活动草稿和非法集合不修改文档或清空重做分支；成功后保留新版本多选。
模型与宿主回归覆盖非连续集合、原几何分配保持、一次撤销、选择版本失效及无变化重复命令。
此命令应用完整当前样式，不等于已经实现混合属性面板或按字段批量编辑。
点击组内对象可开始整组拖动，点击其他对象折叠为单选。
回归覆盖反序增减选择、过期选择拒绝、单对象命令拒绝、批量删除和整组选中栅格化的一次历史提交。
组移动提交内核 `Editor::translate_many` 已建立：先验证整个有序集合的坐标和总保留点预算，
再一次替换全部候选，版本只增加一次。历史交换整个原对象集合，撤销/重做恢复原点数组分配，
不靠反向浮点计算；分支替换释放旧候选的点预算。测试覆盖非连续选择、精确恢复、
后续对象越界时不部分移动、非法索引与预算失败不修改文档，以及重做分支资源释放。
鼠标组拖动已接入：按下组内对象时只克隆一次选中几何，移动复用点缓冲并按原几何计算；
效果像素保持共享，合成仍保留原图层顺序和源图坐标。松开一次提交并保留组选择，
取消、捕获丢失或版本失效释放预览，导出只读取已提交文档。预览点数量受文档上限约束，
提交的额外历史点仍需通过共享预算检查。真实 WIC 测试覆盖模糊/马赛克与荧光笔组合的
分数位移预览和提交逐字节一致、导出不包含预览、一次撤销恢复；另覆盖非法位移整组拒绝、
取消与过期预览清理。实际鼠标视觉、大规模集合拖动性能与全部工具组合仍待验收。

`editor_group_rotation.rs` 已实现明确给定视觉旋转中心的批量事务 `rotate_many_around`。
对照主干 `stickerAnnotationTransforms.ts`，普通对象移动中心并更新旋转角；模糊和马赛克
直接旋转遮罩点，将已有角度吸收到几何，保持效果源图像素共享和源图坐标系不变。
所有候选和点预算检查通过后才提交一次历史，失败保留原文档及重做分支；整周旋转不写历史。
测试覆盖形状/效果组合、非有限参数、越界、预算失败、原分配身份撤销恢复，以及模糊旧角度
吸收和像素共享。`,` / `.` 快捷键和右键旋转菜单已支持多选，每次整组旋转 15°，
成功后保留组选择，一次撤销恢复。`group_bounds.rs` 按视觉边界并集计算中心，处理已有角度、
路径线宽和箭头头部；贝塞尔使用解析极值而非控制多边形，箭头头部复用既有几何。
解析曲线边界与主干采样边界可能存在小量差异，尚未完成所有工具的跨实现精度对照。
真实 WIC 回归验证旋转模糊遮罩后仍采样原图坐标，并计入既有 8% 白色覆盖；
模型/宿主测试覆盖曲线极值、非对称组中心、箭头边界、旋转后选择保留和整组撤销。
鼠标旋转控制柄/连续预览已接入基础链，完整人工交互仍待验收。
右键菜单已增加“分别围绕各标注自身中心变换”开关，默认关闭时保留整组中心行为；
开启后，多选旋转和缩放分别使用各对象的视觉边界中心，但仍一次提交/撤销整个集合。
同步缩放与含模糊的后台缩放使用相同模式参数。开关属于当前窗口，不写文档历史；
切换时取消活动预览和待交付缩放候选，避免按旧模式提交。模型/宿主回归覆盖形状与效果
自身中心保持、非等比缩放位置保持、样式变化和整组撤销；实际菜单交互仍待人工验收。
移动轴约束已在同一菜单接入“自由 / 仅水平 / 仅垂直”，默认自由；单选和多选拖动
共用约束函数，在更新预览前限制图像空间位移，松开提交使用相同位移。
切换移动轴取消当前预览，不写文档历史；它不约束画布平移，也不冒充旋转/缩放的轴模式。
宿主回归覆盖三种模式的单选与多选预览、提交、未选对象保持和一次撤销。
连续变换参数内核已在 `transform_gesture.rs` 对照主干 transform controller 实现：
自由旋转使用起点/当前指针相对中心的 atan2 角差，轴向旋转使用对应位移乘 0.5；
缩放按轴计算比例，近零分母返回 1，倍率限制 0.1–8，等比约束优先采用径向距离比。
每次从固定手势起点计算，不累加上一帧结果；有限坐标检查拒绝无效输入，计算不分配数组。
既有移动约束已复用该模块的轴类型。回归覆盖角度、轴倍率、等比覆盖、负倍率钳制和近零保护。
旋转和缩放控制柄均已调用此参数内核。
旋转几何已提取到 `editor_rotation_preview.rs`，现有组旋转提交复用该实现。
预览按固定原对象与角度原地更新点缓冲，不累计上一帧误差；全部坐标检查通过后才写入，
失败保持上一次有效预览。效果遮罩和普通对象沿用各自的源图/角度规则。
模型回归覆盖连续角度变化的缓冲复用、与提交结果一致、撤销恢复及非法参数不部分更新。
`desktop/rotation_handle.rs` 已接入选择范围上方的旋转柄，支持单选和多选，
按下固定手势起点/版本/中心模式，移动原地更新选中几何，松开一次提交并保留选择。
绘制复用原图层顺序与效果缓存；取消、失去捕获或版本变化清除预览，不写历史。
活动旋转共用变换准入标记，避免滚轮和异步交付破坏手势。真实 WIC 回归验证模糊/马赛克
鼠标路径的预览与提交像素一致、缓冲复用与取消；另覆盖组旋转一次提交/撤销及过期手势清理。
旋转柄菜单已提供自由角度、水平拖动、垂直拖动三种模式；轴向模式按图像空间位移乘 0.5
计算角度，与移动轴设置独立，切换会取消活动手势。回归在非单位视口缩放和偏移下验证轴向
位移转换、忽略另一轴、预览/提交一致和撤销。视口边缘可达性、混合 DPI 与实际鼠标视觉
仍待验收，不能认为全部控制柄交互已完成。
缩放预览几何已在 `editor_resize_preview.rs` 提取：单对象与组缩放候选生成共用该更新接口，
点缓冲可在连续预览中复用；样式、字号、测量尺寸和全部目标坐标先验证，随后才更新候选。
局部形状、序号统一尺寸、文字重排和图像轴路径变换保持各自规则，效果像素仍共享。
回归覆盖旋转后的多种工具连续缩放、缓冲身份、与提交一致，以及无效参数/越界失败后
几何和样式不变。`desktop/scale_handle.rs` 已接入选中范围右下方的缩放柄，支持单选/多选、
非等比拖动和 Shift 等比约束；使用固定起点、中心模式与版本，原地更新几何预览。
文字按目标字号重新测量，组内失败回滚预览；回滚测量也失败时取消手势，不保留半组。
松开生成 ResizePlan，宿主在释放鼠标捕获前接管：普通对象同步提交，含模糊时走后台重算；
取消、失焦或过期状态释放候选，未提交预览不进入导出。专用路径端点/延伸柄优先命中，
避免通用变换柄抢占既有贝塞尔编辑入口，相关回归已通过。
真实 WIC 测试覆盖马赛克预览/提交像素一致；宿主测试覆盖多选非等比候选、一次提交/撤销及取消。
当前模糊拖动预览复用旧强度像素。松开后保留最后一帧几何预览直到后台交付，
不再先回到旧尺寸；显示绑定原文档版本和选择，捕获丢失只结束手势而不清除已交付任务的显示。
任务完成/失败/取消时清除显示并重绘，成功提交与清除在同一消息回调内完成；Esc、模式切换、
换图和过期状态不会遗留预览。未交给宿主的候选取消时同步释放显示，导出始终读取已提交内容。
真实 WIC 与线程回归验证松开→捕获丢失→后台交付期间几何/像素预览保持、点缓冲身份不变、
未提交导出不变、最终强度更新及一次撤销恢复。实时预览强度尚未重算，最终效果仍可能变化；
缩放柄已增加独立的自由/仅宽度/仅高度菜单设置，Shift 等比约束优先于轴设置；
切换缩放轴取消当前手势和后台候选，不影响移动轴与旋转轴。宿主测试在非单位视口缩放、
非零偏移下验证轴向限制、Shift 覆盖和预览/提交尺寸一致。
通用控制柄的客户区边缘布局已在 `transform_handle_layout.rs` 接入：绘制尺寸按 DPI 换算为 DIP，
旋转/缩放柄的位置与命中共用限制后的坐标，保持距边缘 8 DIP；两柄挤到同一角落时分离，
小于 24 DIP 的客户区隐藏通用柄，避免倒置区间和不可命中区域。此布局不改变文档变换中心。
回归覆盖 96/144/192 DPI 等价布局、极小窗口、选择完全在视口外时柄仍在窗口内且可分别命中。
专用路径控制柄边缘处理、真实窗口缩放/DPI 切换和实际拖动性能/视觉仍待验收。
单选模糊/马赛克的旋转命令现已复用相同的遮罩几何旋转路径，不再通过普通对象角度变换
带动源图或马赛克网格一起旋转；普通对象仍沿用角度历史。真实 WIC 回归覆盖单选效果命令
与几何变换结果一致、撤销恢复，以及模糊旋转后新位置采样原图对应像素、原遮罩位置消失。

组等比缩放已接入 `+` / `-` 与右键放大/缩小命令。`editor_group_resize.rs` 围绕视觉组中心
移动各对象中心，并复用单对象缩放的候选生成逻辑；线宽、圆角、字号和马赛克单元同步缩放。
文字由宿主用 DirectWrite 测量目标字号，不直接拉伸旧文字包围框；全部排版、几何和预算检查
通过后才一次交换文档，失败不留下部分缩放。一次历史可恢复整组原对象及字号，成功保留选择。
模型测试覆盖组中心、样式、原分配身份恢复、测量失败原子性及测量尺寸采用；真实 DirectWrite
测试覆盖组内文字排版与一次撤销。非等比组缩放已接入右键增减宽度/高度命令：
形状保留自身角度并调整局部尺寸，路径先吸收已有角度，再按图像轴缩放点集；
字号、线宽和圆角使用横纵倍率平均值，文字重新排版。回归覆盖旋转路径的图像轴语义、
形状角度保留、非连续选择的中心变化、未选对象保持及一次撤销。
鼠标控制柄预览、全部效果变换像素对照和实际键鼠验收仍未完成。
此外，主干缩放会调整模糊 strength，原生当前仍共享原模糊像素快照；
对应的异步重新计算与整组原子提交已接入基础链路，连续强度精度及完整源图合成语义仍待对照。
其中原子提交基础已拆出：`prepare_resize_many` 生成不修改文档的 `ResizePlan`，
候选可替换经过尺寸校验的模糊快照；`commit_resize` 同时校验文档实例身份、版本、
源栅格身份、活动草稿和最新点预算，再一次提交几何与所有效果像素。不同文档即使版本相同
也不能接收候选，导出快照拥有独立身份。同步缩放复用该路径，未建立第二套提交逻辑。
回归证明准备不写历史、跨文档/过期/草稿拒绝、非法效果目标与尺寸拒绝，以及新模糊快照
和缩放几何一次撤销/重做恢复。
`resize_job.rs` 已实现单工作线程、无排队的后台重算，任务持有整组候选；同一任务中相同
目标强度只计算一次，多个标注共享输出。输入来自候选绑定的底图与透明栅格标注合成，
不从已模糊像素再次卷积。主干 `unitImageModel.ts` 的 displaySrc 优先 previewSrc，
栅格化操作由 composeRasterizedStickerPreview 生成对应合成预览；原生缩放重算现已补上该层。
`raster_source.rs` 使用预乘 BGRA source-over，逐行可取消，输出容量计入共享像素预算；
没有栅格层时直接共享底图，不额外复制。每个缩放任务只合成一次源图，多个强度复用。
测试覆盖透明通道合成、无层共享、取消/尺寸拒绝，以及真实工作线程输出采样覆盖层而非底图。
新建模糊、手动强度修改及底图变化后的既有模糊刷新尚需统一接入，不能据此声称所有来源已对齐。
取消标志传入逐行计算，任务结束前再次检查，取消/失败不交付部分候选；poll 不等待活动线程，
Drop 取消并回收线程。像素与中间缓冲使用既有共享预算，重复请求忙时拒绝。
真实线程测试覆盖强度变化、共享输出、与直接卷积结果一致、一次提交撤销、取消、过期结果拒绝
及拒绝后的任务复用。目标强度按平均缩放倍率计算并保留小数，超出原生 1–64 范围拒绝，
不静默截断。`BlurImage::strength` 保存精确 f32 强度，整数 sigma 入口仅保留给原有整数控件；
`blur::region_precise` 用小数标准差计算卷积，核半径取 ceil(3 sigma)，整数入口复用同一算法。
重算任务按强度位模式复用相同结果，连续 1.1 倍缩放验证 4→4.4→4.84，不再每次取整；
撤销恢复原强度。独立 f64 脉冲响应参考验证 1.4 强度的输出，证明不是只保存小数而仍按整数计算。
这不等于与浏览器 CSS filter 的全部像素精度、强度边界和色彩处理已对齐。
`desktop/resize_input.rs` 已统一接入缩放快捷键和菜单：选中集合含模糊时在后台重算，
不含模糊时保持同步；计算期间保留原画面，阻止新鼠标绘图和重复缩放/栅格化任务。
30ms 定时器只在任务活动时存在，完成后移除；Esc 取消，关闭由 Job 回收线程。
交付时还核验当前选择及活动手势，过期结果不提交。成功一次更新历史、选择和命中区域，
失败提示在释放 RefCell 借用后打开，原标注保留。真实线程→Canvas 测试覆盖模糊/普通对象
一起缩放、像素强度与几何一次撤销、选择变化和文档变化拒绝；实际菜单、取消时延和大图性能
仍需人工/运行时验收。完整主干模糊语义不能仅由这条重算链路证明。

标注栅格化已开始迁移，基线为 `stickerRasterizeActions.ts`：`editor_rasterize.rs`
提供选中索引的独立矢量快照，以及“替换透明栅格层并移除对应矢量”的单次原子历史操作。
索引必须严格递增且有效，提交必须匹配文档版本/源层身份、无草稿、底图身份不变；
撤销按原索引恢复原始矢量对象，重做重新移入历史，不复制点数组。点预算继续计入
已移入历史的矢量，历史淘汰和分支替换释放对应资源。
`desktop/rasterize.rs` 用独立 WIC 目标合成“已有透明栅格层 + 指定矢量”，不画底图、
不写 PNG 文件；WIC backing 与读回像素都预留共享预算。真实像素测试覆盖已有层保留、
未选矢量排除、底图共享身份及提交/撤销；模型测试覆盖全选、非连续选择、过期拒绝及
分支资源释放。右键菜单已开放“栅格化选中标注 / 栅格化全部标注”，选中范围支持
单选及 Shift+点击建立的多选集合。`rasterize_input.rs` 只允许一个后台合成任务，
工作线程独立建立/释放 COM，UI 保留旧帧直至版本和源层身份校验成功后一次提交。
活动任务期间不接受新鼠标绘图，Esc 取消结果交付；WIC/D2D 同步调用不可中途打断，
关闭时等待线程结束。临时 30ms 轮询在任务结束后移除，不产生空闲刷新。
真实工作线程回归串联选中标注栅格化→仅标注擦除→两次撤销恢复，另覆盖全部范围、
过期拒绝和取消。实际菜单点击、失败弹窗及大图合成时延仍待人工/运行时验收。

内容橡皮擦的数据内核已在 `native/src/erase.rs` 和 `erase_layers.rs` 实现，基础工具入口
与后台提交已接入，实时原生遮罩预览已接入，仍不能视为与主干完整对齐。基线依据为
`stickerBitmapLayers.ts` 的 destination-out 与 `stickerAnnotationEraseController.ts`：
内容模式擦除底图及已有栅格标注层，仅标注模式保留底图身份；矢量标注不被展平或重建。
内核读取共享原图，圆帽路径使用 4×4 子像素覆盖并集，所有预乘 BGRA 通道同步衰减；
自交/重复段不反复削弱同一手势的软边。该抗锯齿及手势并集语义尚未与浏览器逐像素对照。
点数上限 65536、尺寸范围 4–96、坐标有限且绝对值不超过一百万，段范围先裁到图像；
覆盖掩码只分配整次手势与图像相交的包围区（每像素 2 字节），而非整幅图；
例如 8192×8192 图片中的直径 20 整坐标单点手势，掩码载荷为 800 字节，而不是
整图掩码的 128 MiB。这是缓冲大小计算，不是进程 RSS 实测。
完全在图外或受影响区全部透明时，不分配掩码及候选整图；输出副本进一步延迟到首个
实际改变的像素才分配，包围框含内容但笔迹未命中也复用源图身份。
掩码与候选输出均预留共享像素预算，取消按段/行检查且不返回半成品；成功修改仍需
完整输出像素快照，跨全图手势的掩码仍可能很大，超预算会拒绝而非无界分配。
`erase_span.rs` 将每段胶囊与当前像素行相交，得到保守横向候选区间，避免细长斜线
扫描完整二维包围框；额外一像素保护浮点舍入，最终仍用相同 4×4 覆盖测试决定结果。
4096×4096、宽度 4 的对角线测试中，候选像素少于 50000，而原包围框为 16777216，
候选检查量缩减超过 300 倍；这不是实际延迟加速比。独立全图采样参考覆盖 70 组
分数坐标、逆序、水平/垂直、超大坐标及确定性随机路径，与优化结果逐字节一致。
两层处理全部成功后才返回候选，无变化结果共享原图；失败不发布只擦除一层的中间状态。
已有测试覆盖单点、连续长段、自交/重复/分段一致性、软边预乘、无变化共享身份、非法输入、
中途取消及两种层模式。CPU 内核尚未进行大图延迟实测，当前通过后台任务接线，
不能直接在鼠标消息或绘制回调执行整图计算；层快照模型不代表工作区持久化已经完成。
`editor_raster.rs` 已将栅格替换纳入现有 50 条有界编辑历史，不另外维护一条橡皮擦
撤销栈；画笔→栅格擦除→画笔按统一游标撤销/重做，转移共享快照而不复制像素。
提交核验文档版本、源层共享身份、无活动草稿、尺寸和像素长度；无变化不写历史，
分支编辑释放被丢弃的重做快照，历史淘汰释放旧像素引用，导出快照保留当前栅格身份。
`erase_job.rs` 提供一个活动线程、一个结果槽的后台任务，不排队或覆盖其他擦除手势；
忙时拒绝新提交，完成轮询不等待活动线程，取消后不交付结果，销毁先取消再 join。
后台任务及统一历史已有模型回归。PNG 导出现已优先读取编辑器的已提交栅格快照，
按底图→栅格标注→矢量标注合成；真实 WIC/PNG 测试覆盖两种擦除模式、三步撤销、
矢量位于栅格上方以及导出快照不受后续撤销影响，不再依赖可能过期的宿主底图参数。
透明命中区域现已读取已提交底图及栅格标注 alpha 并集，无需分配合成图；任一层身份
改变都会重建对应缓存，原有有界行区间和一像素采样边缘保护保留。纯模型及真实 GDI
区域测试验证底图擦空后栅格标注仍可命中、全部擦空后区域为空。
Canvas 新建/换图已初始化同一编辑器栅格模型，`canvas_raster.rs` 按源层共享身份更新
底图及栅格标注 GPU 位图。两层上传都成功后才替换资源，只改变底图时复用栅格标注位图；
不因栅格历史变化销毁绘制目标或效果缓存。键盘/菜单撤销与重做同步宿主底图视图，
绘制前再次校验，底图改变时使后续模糊准备快照失效；旧模糊笔迹仍持有其已提交快照。
换图先校验新栅格模型再取消旧手势和释放旧资源，非法替换保留原文档。WIC 回归覆盖
逐层位图身份复用、双层失败不部分替换、屏幕层合成次序及擦除/撤销/重做视图同步。
导出快照现在自行保留已提交底图，释放旧图必须等待所有导出快照 owner 释放。
按 W 或右键菜单进入内容橡皮擦，默认尺寸 20，方括号/宽度菜单调整到 4–96；
右键“仅擦除栅格标注”切换模式，无栅格标注层时此模式不开始手势。
按下/移动采集有界图像坐标，松手交由单工作线程计算并通过临时 30ms 定时器收取结果，
空闲时移除定时器；忙时阻止新的鼠标绘图提交，Esc 取消计算。完成后核验版本/身份，
一次写入历史、同步底图及命中区域。取消、失焦和捕获丢失丢弃未提交手势，缩放不会
改变活动手势坐标。真实工作线程→Canvas 测试覆盖单次提交、撤销、取消及过期结果拒绝。
拖动时显示双颜色尺寸环，并通过 `erase_preview.rs` 对图像矩形减去圆帽擦除路径，
实时裁掉底图/栅格层对应区域；仅标注模式先正常画底图，再对栅格标注应用遮罩。
内容模式对底图和栅格标注分别应用遮罩后再 source-over，不能先把两层合成后统一
衰减 alpha，否则双层软边会比最终提交更透明。真实 WIC 回归先复现此差异，再验证
双层预览符合逐层衰减的合成公式（最多 1 个通道量化级误差），且夹具确实包含软边。
矢量标注及模糊效果仍在遮罩之外独立合成，不重建其位图缓存。松手后保留同一手势
快照用于预览，直至后台结果成功提交或明确取消/失败，避免等待期间原图闪回；
只额外克隆一次有界点数组，不复制源像素。取消拖动只移除遮罩，不重新上传原图。
真实 D2D/WIC 测试验证两种模式的中心/外部像素、取消恢复及原纹理身份不变。
预览使用 D2D 几何抗锯齿、提交内核使用 4×4 子像素采样，边缘尚未逐像素对齐；
每个绘制目标只缓存一个预览几何：以手势身份、追加点数、宽度、视口及图像边界为键，
未变化的重绘不再构建路径；手势身份在松手克隆请求时保留，避免等待后台期间重复构建。
同一身份下路径只追加，新手势分配新身份；缓存不持有源像素或点数组，预览结束后的
下一次绘制及目标销毁释放缓存。真实 D2D 对象身份测试覆盖复用、追加/缩放/宽度/边界/
新手势失效与源像素释放。活动长路径追加后的完整几何重建成本、实际松手视觉连续性、
性能及点击穿透仍需完整验收。

效果合成层级已按主干 `stickerAnnotationLayer.ts` 对齐：荧光笔统一底层，随后为模糊、
马赛克，最后为普通标注；同类层级仍保持文档顺序。`Tool::render_rank` 同时供屏幕/
PNG 绘制与逆序选择命中使用，不分配排序后的标注副本。后画的模糊不能盖住已有
马赛克遮挡，选择也不会穿透可见上层去命中创建时间更晚的底层效果。
真实 PNG + D2D 命中测试遍历普通画笔、马赛克、模糊的全部 6 种创建顺序，
验证最上层像素与选择对象一致、移除普通画笔后马赛克仍遮挡模糊、撤销恢复一致。

预计算模糊快照现已接入模型和屏幕/PNG 共用绘制分发，并开放基础模糊入口。
按 G 或右键菜单“模糊”启动后台准备，强度默认 8，笔刷默认宽度 28。
模糊工具下 Shift+[ / Shift+] 或菜单可调整后续强度（2–64），菜单显示当前值；
变更会取消未提交手势、清理旧准备结果并启动新计算，触及边界且值未变时不重算。
快照保存计算时的 sigma，准备复用、结果准入和落笔均核验强度；新默认值不回写旧笔迹。
Canvas 回归覆盖强度独立校验、旧快照不能落笔、默认值隔离、工具切换保留，以及
不同强度笔迹的撤销/重做。
选择工具选中已有模糊笔迹后，同一组强度菜单/快捷键改为后台重算该笔迹；
计算期间旧快照继续显示，结果核验源图、文档版本、目标索引和当前选择后一次替换。
连续加减以待处理请求的强度为基准，不因计算尚未完成而反复停在同一个值。
替换只修改模糊快照，保留路径、旋转和样式，不回写新笔迹默认值，并记录一次撤销；
失败、改选、撤销或换图均不提交过期结果。真实后台任务与 Canvas 联合测试覆盖
连续请求、原子替换、几何/默认值保持、选择变化拒绝及撤销共享旧快照恢复；
模型测试覆盖相同快照无操作、尺寸不符与历史预算不足时无部分提交。
准备期间不允许落笔，右键菜单显示“模糊准备中…”；准备成功后单击/涂抹进入笔迹事务。
同一源图已有结果时直接复用；换图清空准备缓存。计算由单任务组件持有，只有忙碌期间
使用 50 ms 窗口定时器，结束后停止；Esc 取消，工具/源图/文档版本变化会在轮询时取消。
结果安装前再次核验工具、源像素身份、尺寸、全图区域、强度、文档版本及无活跃手势，
过期结果不改变图像、标注或历史。失败报告后保留原图与标注，不提交半成品。
Canvas 测试覆盖准备前不能落笔、成功后可创建、安装不写历史、换图清空，及源图/版本/
工具/区域不匹配时拒绝。真实定时器、快捷键、菜单与大图等待体验仍待人工验证。
`BlurImage` 校验尺寸和像素长度，笔迹、历史及导出通过共享 Pixels 持有快照；
`Editor::begin_blur` 必须提供已完成图像，普通 `begin(Blur)` 不允许生成缺失快照的笔迹。
连续路径、单击、提交和取消复用有界编辑事务，撤销/重做不重新卷积或复制整图。
原生绘制使用圆帽路径遮罩，遮罩内叠加模糊图与主干 8% 白色覆盖层，遮罩外不改原图；
图层范围裁到逆旋转后的可见范围，不在绘制回调中运行卷积。
真实 PNG 测试验证黑白边界被平滑、白色覆盖像素、路径外原样保留及撤销/重做精确恢复。
当前快照仍按完整图像表达；绘制目标私有 `effect_cache.rs` 采用总像素载荷最多 32 MiB、
最多 32 个条目的 LRU 模糊位图缓存。相同尺寸/共享像素身份命中时不重复上传，多个
不同快照不再因单槽缓存相互替换。不同存储身份即使字节相同也不混用；超限先淘汰最旧
条目再上传。任一维度超过 2048 的图像由 `annotation_blur_tiles.rs` 按固定 512 像素
源图网格分块，只遍历逆旋转可见范围与笔迹包围框的交集，并跳过确定不相交的路径块。
每块携带一像素插值邻域，最大上传 514×514；沿源图行跨度直接上传，不建立 CPU 块副本。
整图与块共享同一 32 MiB/32 条目 LRU 预算，块坐标随源图固定，平移或缩放不会改变键。
内部块使用不重叠的像素归属遮罩，避免分数位置的双重抗锯齿合成形成接缝。
每次合成先按当前文档（包含草稿及替换预览）裁剪缓存：撤销、删除或重算后的旧快照
下一帧释放 GPU 缓存引用，即使其他模糊仍存在；窗口设备重建/换图/销毁释放整个缓存。
CPU 准备结果与历史另有 owner，不把 GPU 淘汰等同于全部像素释放。
真实 WIC 测试覆盖多快照连续合成仅首次上传、删除单项后保留另一项、LRU 字节/条目
双上限及清理释放快照引用；32 MiB 是保留载荷策略，
不是驱动/图层总显存测量。分块对照真实 WIC 整图绘制覆盖平移、分数缩放和旋转：
非旋转高频纹理及旋转模糊图内部允许最多 1 个通道量化级差异；旋转图像外轮廓一屏幕
像素带内允许最多 16 级覆盖率差异，不宣称逐字节等价。重复绘制命中缓存、非法块坐标
拒绝及越界整数保护均有测试。视野超过缓存预算仍可能反复上传，分块层合成开销、GPU
峰值及实际交互性能尚待测量；CPU 预计算仍保留完整图像。移动/缩放/旋转时
源采样坐标语义尚待完成；已有笔迹强度重算已接通，异步结果入画布的版本校验已接通，仍不能视为完整模糊工具。

模糊标注计算内核位于 `native/src/blur.rs`，工具与画布接线状态见上文。
当前为可分离高斯卷积，sigma 支持 1–64 图像像素，核截断到 ±3 sigma；
只计算请求区域及其所需原图邻域，区域边界不会被当成图像边界。
输入/输出均为预乘 BGRA，图像外按透明处理，颜色与 alpha 同核处理；
中间缓冲保留浮点值，最终一次量化，分块结果与整图计算后裁剪一致。
浮点中间值改用循环行缓冲：最多保留 `min(邻域高度, 6×sigma+1)` 行，
每个源行仍只做一次横向卷积，覆写前保证旧行已不再被当前纵向卷积使用。
例如整幅 3840×2160、sigma=8 时，该浮点缓冲按尺寸公式从约 126.56 MiB
降至约 2.87 MiB；这是缓冲容量计算，不是进程内存实测，源图和输出另计。
测试将多次回绕的区域结果与独立单行计算逐字节比较，覆盖 sigma=1/3/16/64、
非零区域起点、透明像素及图像边界。计算核与最终量化未改变。
浮点缓冲及输出均在分配前申请共享像素预算，源像素只读，每行检查取消。
测试覆盖脉冲对称扩散、预乘颜色不越过 alpha、区域/整图一致、透明图像边缘、
取消及非法区域拒绝。尚未验证与主干浏览器滤镜的像素级一致性（包括核截断差异），
主干裁剪投影、超大图分块及完整变换仍未完成；白色覆盖和路径遮罩已在上述预计算快照绘制链接通，不宣称模糊工具完成。
该 CPU 内核目前不在 WM_PAINT 中调用；大图与大 sigma 的计算耗时仍需优化和测量。
新增 `native/src/blur_job.rs` 管理计算线程：最多一个运行任务及一个最新待处理请求，
新请求取消旧任务并替换待处理项，不积累指针移动队列。轮询只处理已结束线程，
旧线程真正退出后才启动下一个任务；启动前释放过期结果与源图引用，避免挤占新预算。
结果携带源像素身份、区域、sigma 和文档版本，供宿主提交时核验；已取消结果不交付。
Drop 主动取消并 join，任务取消延迟仍受内核每行计算耗时影响，不宣称瞬时退出。
测试覆盖最新请求覆盖、中间快照释放、取消后的结果丢弃、重新请求、Drop 线程回收，
以及错误返回后仍可继续计算。该任务组件现由 `desktop/blur_input.rs` 接入窗口按需定时器和结果准入；
宿主核验见本节开头，不仅依赖任务携带元数据。

马赛克已接通底层网格生成（`native/src/mosaic.rs`）、共用原生绘制和基础输入。
按 M 或右键菜单“马赛克”可涂抹或单击，笔刷默认宽度 28、格宽默认 12，与主干默认一致。
格宽以每条标注的样式字段保存，属性输入范围 2–64；马赛克工具下 Shift+[ / Shift+] 或菜单
“减小/增大格宽”调整后续默认值，菜单显示当前值。工具切换独立恢复格宽与笔刷宽度。
改默认值不写历史、不回写旧标注；现有 Ctrl+Enter 样式应用事务可修改目标标注，
撤销/重做恢复格宽及实际 PNG 输出。缓存键包含格宽，改变属性不会误用旧纹理。
缩放时格宽与线宽均按两轴平均比例变化；模型保存正有限小数（上限 100 万），
不把属性输入的 2–64 整数限制套到缩放结果上。绘制才按主干 round/max(2) 取有效格宽，
缓存使用相同规范化值；等效小数格宽共享纹理，但不丢失模型精度。
回归覆盖非等比缩放、格宽越过 64、小数保留、撤销/重做、越界拒绝无部分提交、
等效格宽命中同一位图及非有限输入拒绝。完整旋转/移动组合仍需继续与主干对照。
现有宽度命令可调整后续笔刷宽度，切换工具独立记忆；普通画笔宽度不被覆盖。
真实 Canvas 输入测试覆盖连续采点、单击、取消不写历史、单步撤销/重做、
圆帽端点命中和工具切换。PNG 像素验证不等于菜单、按键和指针的人工体验验收。
保留主干 `stickerEffects.ts` 的遮挡语义：固定蓝灰色调、不采样原图、完全不透明，
不能换成仍携带原图内容的降采样马赛克。整数溢出哈希按 JavaScript Math.imul / >>>
逐项实现，颜色取决于绝对网格坐标；局部区域生成与全图裁剪保持相同身份。
小数边界按主干向外覆盖并按行顺序覆盖重叠像素，避免透明接缝泄露底图。
目标纹理受现有尺寸、单图字节上限和共享像素预算约束，分配失败释放预算；
接口不接收源图，输出不透明 PBGRA。测试覆盖 Node 计算的哈希夹具、负坐标、
局部裁剪一致、小数边界全覆盖、无效输入和最小格宽。
`desktop/annotation_mosaic.rs` 使用原生路径遮罩及网格纹理绘制圆帽连续路径或单击圆点，
接入屏幕/PNG 共用分发、模型单次提交及撤销/重做。共享画笔的透明度与虚线不用于遮挡，
零透明度的马赛克仍按实际可见几何命中。真实 WIC/D2D PNG 测试验证不同底图上的
内部遮挡像素完全相同、未涂区域不变、单击可见及撤销/重做恢复。
绘制读取标注保存的格宽，先将目标边界逆旋转回路径空间，裁掉不可见部分，
再按最多 256×256 图像像素分块生成/绘制；路径几何只展宽一次并作为统一遮罩。
CPU 纹理暂存每块最多 256 KiB，不再按路径的巨大跨度分配整块纹理；
GPU 位图使用绘制目标私有的 8 项 LRU 缓存，键包含绝对区域与尺寸；重复绘制命中时
复用同一个 D2D 位图，不重新生成 CPU 网格或上传。缓存最多保留 8 个 256×256 位图，
对应像素载荷最多 2 MiB，不包含 D2D/驱动内部资源；满额先淘汰最旧项再分配。
缓存随 Target 在换图、设备丢失和窗口销毁时释放，PNG 合成使用自己的局部缓存，
不跨线程或跨目标共享 GPU 对象。真实 WIC 目标测试验证位图对象身份复用、LRU 淘汰、
尺寸键隔离、无效尺寸拒绝及容量边界。D2D/驱动仍可能持有内部提交资源，
遮罩层仍有可见区域成本，不能把上述上限当作总 GPU 内存测量。
真实 PNG 回归覆盖跨多块无接缝、
跨度超过 180 万坐标单位的路径正常裁剪，以及旋转后进入画面的路径不被误裁。
窗口已具备跨绘制缓存；超过 8 块的顺序扫描仍可能发生缓存抖动，极小缩放时也仍需
降低生成工作量。基础入口与格宽属性已接通；持久化、完整变换对照与大型画布性能验收
尚待完成，目前不计为完整马赛克工具。
分块生成前新增路径遮罩相交检查：块边界保守扩出 1 DIP，仅跳过确定不相交的块，
避免斜线/弯曲路径包围框中的空白块消耗网格生成、位图上传和缓存容量。
真实 700×700 WIC 目标测试验证 9 个候选块中只上传相交部分，第二次绘制零新增上传，
逐像素检查对角线内部保持完全不透明且未涂区域透明。上传计数仅在测试构建中保留。
几何相交检查自身也有成本，尤其复杂长路径；这不是帧耗时或 GPU 内存的实测结论。

序号标注基础链已接通：按 N 或菜单选择序号工具，在图片内单击创建。默认半径 14、
黑色底色、当前前景色和微软雅黑；字号按 round(radius×1.15)、边框按 round(radius/7)
计算，直接使用原生圆形与居中 DirectWrite 绘制，屏幕和 PNG 共用。序号从 1 递增，
插入与计数器属于同一次历史事务；撤销/重做恢复计数器，撤销后分支复用下一号，
删除/复制不重排或额外消耗编号。模型半径限制 8–999，最大对应字号 1149，失败不消耗编号。
圆框半径以两点包围框表达，不复用普通形状的 corner_radius；现有选择、移动、旋转、
复制、撤销及导出链可处理序号。选择工具下双击序号，或选中后按 F2/菜单编辑，可修改显示文字；字体菜单也可修改选中序号的字体。文字和字体修改保留圆框几何、位置、旋转及样式，不按文字测量结果撑大圆框，不消耗或重置自动编号，支持一次撤销/重做。文字工具点击序号仍按主干的工具语义新建普通文字，不误将序号当普通文字编辑。模型和 Canvas 回归覆盖圆框保持、编号连续及撤销恢复。序号已有独立会话样式：C 设置前景色，Shift+C 设置底色，F 切换底色透明；切入序号恢复专属样式，切出恢复之前的共享样式，并保存原工具的线宽等配置，避免经序号中转丢失修改。菜单“序号半径”支持 8–999 整数图像像素，Shift+[ / Shift+] 也可调整后续半径；专用原生输入框在内部拒绝越界值，取消不修改配置。配置只影响后续创建，不写文档历史。原生输入框、工具切换与实际 Canvas 创建回归已覆盖边界、颜色隔离、透明底色和半径生效。序号工具未选中文字时，字体菜单提供“新序号默认字体”：与普通文字默认字体独立保存，只选择字体族，原生对话框禁用字号和字重选择，创建字号仍由半径派生；修改默认值不改已有序号、不影响编号或撤销历史。字体名经过长度、空白与控制字符校验，非法值保留原配置；模型和 Canvas 测试覆盖普通文字配置隔离、半径字号关系及旧标注不被回写。真实字体对话框手测、磁盘持久化及完整变换对照仍待完成。
测试覆盖输入单击只提交一次、计数器历史、失败预算及真实 PNG 圆形/文字输出。

序号的模型轴向缩放现统一使用两轴平均比例，圆框不再因非等比操作变成椭圆；
中心、旋转和标签保持不变，字号及边框同步缩放，且不消耗编号。
回归覆盖撤销/重做精确恢复，以及字号越界时几何、历史和计数器均不部分提交。
这只确认了圆形不变量，尚不代表完整变换对齐：基线序号半径取整/限幅及
`stickerAnnotationTransforms.ts` 中半径再次经过样式缩放的行为仍需单独裁定；
普通文字选择缩放已接通 DirectWrite 目标字号重测，围绕原中心更新布局框，保留旋转，
并将字号、样式和几何合并成一次可撤销事务；不再将非等比因子直接用于文字框。
无效比例在排版前拒绝，测量失败、布局越界或预算不足不修改文档。
真实 Canvas 与 DirectWrite 回归覆盖多行文字重测、中心/旋转保持、选中引用更新、
单步撤销/重做及失败不污染历史。与主干浏览器字体度量的像素级差异仍待对照。

文字标注已开始迁移：新增不可变共享文字内容、字体与字号模型，单条最多 4096 UTF-16 单元，
字体名最多 64 单元，拒绝 NUL、空白内容及无效字号；复制和导出快照共享字符串而非重复分配。
插入/替换文字接入现有元素数、点数和 50 步历史约束；撤销、重做、复制及快照保持独立内容，
替换时不猜测字体度量，由宿主提供布局尺寸。文本数量上界来自元素数及历史数，文本存储不计入像素预算。
DirectWrite 基础绘制已接入屏幕/PNG 共用绘制链，文字框接入选择命中；排版对象局部释放，
尚无布局缓存。真实 WIC/D2D/DirectWrite 测试验证中文和英文产生像素、撤销移除及重做恢复。
文字输入入口已接通：按 X 或菜单选择文字工具，在图片内单击打开原生多行 EDIT 对话框，Enter 换行，确定提交，取消不写历史；默认 16 px 微软雅黑，与主干工具默认一致。提交前用 DirectWrite 测量含尾随空格的宽度及多行高度，加 4 px 四侧留白；显式换行，不自动折行。模态期间不持有 Canvas 借用，提交前核验窗口、工具、图像存储身份和文档版本，防止异步换图后写入旧坐标。原生对话框测试覆盖中英文/换行/emoji 及空白拒绝，排版测试覆盖多行和尾随空格。文字工具点击最上层已有文字时直接重新编辑，选择工具双击文字也可进入编辑；按绘制逆序命中，非文字标注遮挡时不穿透编辑底层文字，支持旋转后的文字框。另可在选择后按 F2 或菜单“编辑选中文字”重新编辑，保留字体、字号、粗体、颜色及旋转；尺寸重测后补偿旋转中心，保持原左上锚点世界坐标不变，确认只写一次历史，取消不写入。模型统一 CRLF/CR 为 LF，原样确认不因换行形式或重新测量改动文字框。Canvas 回归覆盖属性/锚点保持、撤销重做、换图/版本/工具变化拒绝以及无变化不写历史。原生字体选择框已接通：文字工具下未选中文字时设置后续新文字的默认字体，选中文字时修改该标注并形成一次可撤销事务；默认设置不污染文档历史，修改已有文字不反向改默认配置。模型使用 96 DPI 图像逻辑像素，字体对话框的十分之一 point 转换为模型字号；初始 LOGFONT 高度按窗口 DPI 换算。当前支持常规/中等字重和粗体，对斜体、下划线、删除线及其他字重明确拒绝，不静默丢弃样式。默认设置仅在会话内保留。转换与事务测试已覆盖字号换算、非法样式、默认配置隔离及选中文字字体的撤销；真实字体对话框、高 DPI 显示及输入法候选交互、行距/基线/主干像素级表现对照仍待完成；系统 EDIT 提供输入基础，不等于已完成人工输入法验收，也不是最终行内编辑体验。

目标使用 Rust 业务模型、Win32 窗口和输入、Direct2D/DirectWrite 画布；实时纹理与合成按需接入
Direct3D/DirectComposition。正式选定接口组合前验证透明窗口、设备丢失、DPI 和捕获排除。

`native/` 是独立 Cargo 工程，不依赖现有 `otter_lib`，避免隐式拉入 Tauri。
使用独立 `native/Cargo.lock`；已纳入依赖扫描清单、许可证审计及候选构建检查。

### 当前可运行入口

区域截图新增可见窗口预选：冻结桌面前按 Z 序枚举可见、非最小化且非 cloaked 的顶层窗口，
优先使用 DWM extended frame bounds，失败时回退 GetWindowRect；最多保存 2048 个边界，
不读取窗口标题或进程信息。边界转换到冻结帧坐标并裁剪，悬停选中最上层匹配窗口，
Enter 可直接确认，单击可固定为后续可调整的选区；拖动仍创建手动矩形。
合成窗口边界和隐藏 Win32 选区测试覆盖 Z 序、负坐标、裁剪、悬停确认和单击确认。
这只是冻结桌面中窗口可见范围的选择，不是独立窗口内容捕获；被遮挡部分仍是桌面上的遮挡画面。
窗口枚举和截图并非原子快照，期间窗口移动的竞态、特殊窗口过滤及真实多屏仍待验证。

新增 `otter-native.exe --capture`，在创建原生编辑窗口之前抓取一次完整虚拟桌面；
`--capture --pin` 将结果交给原生钉图窗口。该模式由用户显式启动，不设置常驻截图任务，
不写文件、不读写剪贴板，随后可使用现有编辑与 PNG 导出功能。
`--capture` 与图片路径或 `--smoke-test` 互斥，冲突在采集前报错。
当前使用主干已有的 GDI 回退思路，支持负坐标虚拟桌面，BGRA 结果转为不透明 PBGRA；
DIB 临时缓冲与最终像素分别计入共享预算，失败路径归还 DC、位图、选择对象和预算。
无显示器的虚拟桌面间隙先清零。测试只使用内存 DC 和合成像素，覆盖颜色通道、行方向、
alpha、填充失败及尺寸/整数溢出拒绝，未抓取真实用户桌面。
另提供 `otter-native.exe --capture-region [--pin]`：在冻结帧上拖动选择矩形，松手后按 Enter 确认，Esc 或右键取消，可重新拖动替换选区。原生选区窗口位于截图时记录的虚拟桌面原点，空选区不能确认；反向和越界拖动统一规范化/裁到帧范围。显示器布局改变时取消，避免错位裁剪；窗口销毁后才释放源帧，裁剪结果另受像素预算约束，取消不进入编辑文档。选区阶段不持续采集、无刷新定时器，仅消息触发绘制。合成图片的隐藏 Win32 窗口测试覆盖选择确认与取消，纯裁剪测试覆盖反向/越界拖动和行数据；真实屏幕上的显示与交互尚未手测。
真实多屏/HDR/受保护内容/捕获排除尚未验收；此入口不代表 DXGI/WGC、窗口截图、长截图、快捷键或截图工具栏已经迁移，选区已加入八个缩放手柄、框内拖动、宽高像素提示，以及方向键 1 px / Shift+方向键 10 px 移动。移动整体裁到帧边界，缩放保持至少 1 px 且不翻转；手柄容差命中保留按下偏移，原地点击不跳边。捕获丢失或 WM_CANCELMODE 回滚当前手势，取消模式同时释放捕获；拖动未松开时不能用 Enter 提交。模型回归覆盖八柄、边界移动、取消和容差点击；高 DPI 外观及鼠标手测仍待验收。选区窗口现提供冻结帧像素放大镜：固定 21×21 BGRA 栈上采样块（1764 字节），最近邻放大、中心像素标记和 #RRGGBB 显示；边缘重复最外侧像素，靠近右下边界时翻转浮层位置，缩放尺寸按窗口 DPI 有界调整。移动指针不重新截图，普通悬停只失效旧/新浮层区域，相同坐标不重复请求绘制；拖动选区仍请求整个窗口重绘，实际大屏开销待测。测试覆盖通道、行方向、边缘采样、浮层定位，并使用内存 DC 验证真实 GDI 绘制像素，不读取用户桌面。选区停止拖动后，Ctrl+C 将当前选区复制为顶向下 32 位 CF_DIB，C 将指针颜色复制为 #RRGGBB 的 CF_UNICODETEXT；成功后关闭截图窗口，不再创建编辑文档。未选区域或未取得指针位置时对应复制命令不执行。共享内存先分配、编码并计入暂存预算，再打开剪贴板；成功发布后由 Windows 持有并释放，失败路径释放仍由应用持有的内存。剪贴板忙或发布失败时显示错误并保留选区；EmptyClipboard 后发布失败无法保证恢复旧内容，错误提示明确告知。数据编码测试使用合成像素，未实际读写系统剪贴板；与画图等外部应用的粘贴互操作、真实取色和剪贴板竞争仍待人工验收。截图目前同步发生在窗口创建前，大桌面启动延迟待测量。

```powershell
cargo test --locked --manifest-path native/Cargo.toml
cargo run --locked --manifest-path native/Cargo.toml -- image.png
cargo run --locked --manifest-path native/Cargo.toml -- image.png --pin
cargo run --locked --manifest-path native/Cargo.toml -- --smoke-test
cargo run --locked --manifest-path native/Cargo.toml -- --settings "C:/path/to/otter-settings.json"
```

`--settings` 只读显式文件；可将“命名设置另存为…”生成的文件路径写入启动快捷方式。
普通启动默认使用 LOCALAPPDATA/OtterNative/app-settings.json，也可用
OTTER_NATIVE_PROFILE_DIR 覆盖目录。自动保存默认关闭，可在菜单中显式开启；仅 profile
首次缺失使用默认设置，已有文件读取/解析失败会停止启动而不是重置配置。
运行期间可用“保存命名设置到已加载文件”显式更新当前来源；外部修改冲突必须重新导入，
不会静默覆盖。另存为成功会切换活动来源到新文件。

本地设置 `CARGO_TARGET_DIR` 指向约定的临时产物目录。当前普通窗口以左键拖动画布、滚轮缩放；
工作区与钉图提供按需创建的 Windows 右键菜单，包含工具、颜色、填充、线型、透明度、
线宽、圆角、撤销/重做和导出入口；显示当前工具勾选及线宽、圆角、透明度值。
菜单操作直接调用模型，不模拟键盘，因此不受 Ctrl/Shift 当前按下状态影响。
菜单关闭后销毁 HMENU；已有原生菜单文字、命令映射及句柄释放测试，真实鼠标/键盘交互仍待人工验收。
`Ctrl+O` 或菜单“打开图片”可在运行期间选择文件；有标注或重做历史时先确认丢弃。
新图片解码及钉图 alpha 区域准备成功后才替换文档，取消/解码失败保留已提交内容和历史；
进入模态对话框会取消未提交草稿。成功替换清空旧历史、重置视口并保留工具属性，
正在导出的快照独立持有旧图像。WIC 支持格式取决于系统解码器，当前读取第一帧。
运行期间的解码与钉图区域准备由单个后台任务执行，worker 自行创建/释放 COM；只传递 CPU 数据。
任务存在时才启用完成检查，完成后停止定时器；菜单显示加载状态，重复打开不追加任务。
`Esc` 请求取消，结果提交前再次检查取消标记、源图片身份、文档版本及编辑手势，避免覆盖加载期间的新编辑。
取消会丢弃结果，但不能强制中断 WIC/文件系统调用；退出仍等待该调用返回，定时器创建失败时也同步收尾。
启动参数加载仍在建窗前完成；准备期间新旧图像同时驻留。已增加进程内共享的 512 MiB
解码像素预算，单图仍受 256 MiB 上限约束；分配前预留，按 Vec 容量计费，解码失败归还。
像素与租约封装在同一个 Arc 中，文档、命中缓存、异步请求令牌和导出快照共享引用，
最后一个引用释放才归还预算；克隆不重复计费。并发准入与释放已有测试。
该预算不包括 WIC 内部缓存、编码流、离屏位图、D2D/D3D/GPU 资源及系统开销，不能当作进程峰值内存上限。
原生文件对话框及替换确认尚待人工验收，自动测试仅覆盖解码准备、编辑状态与资源生命周期。
开发快捷键 `B`/`L`/`R`/`E` 分别选择基础画笔、直线、矩形、椭圆，`A` 选择直箭头，`V` 返回平移；
直箭头复用基线的箭头长度/宽度规则与杆身裁短规则，屏幕和 PNG 导出共享几何；
直线工具也可通过菜单“直线 / 贝塞尔箭头”启用箭头，开关分别随各自工具配置保留，独立箭头工具始终绘制箭头。当前样式可显式应用到已有直线并撤销；绘制、选择、透明层与钉图范围统一识别可选箭头。真实 PNG 测试验证可选直线箭头与独立箭头工具像素一致，并覆盖关闭后的命中及样式撤销。
箭头头部已纳入透明钉图的保守命中范围，原生像素测试覆盖实心头部及撤销/重做后的导出。
直线和箭头绘制支持 Shift 锁定 45° 角度、Ctrl 按 10 图像像素步进吸附；组合使用时先约束角度，再量化相对起点的横纵位移，与基线顺序一致，负半整数采用 JS Math.round 的舍入方向。鼠标移动与松开均读取修饰键，松开以最终坐标提交一次历史；Shift/Ctrl 按下与松开时以最近原始指针坐标立即重算约束型草稿，仅结果变化时请求重绘，不轮询、不写历史；释放修饰键可恢复未吸附端点，普通画笔和荧光笔仅在 Shift 下重算直线草稿，无 Shift 的按键变化不追加采样点。手势提交或取消清除保存的指针。原生菜单已提供 5° 角度吸附开关，直线与箭头分别保存会话内选择，默认关闭；Shift 临时优先使用 45°，Ctrl 仍在角度约束后量化位移。其他工具禁用此菜单项，开关不修改既有标注或历史，尚无磁盘配置持久化。普通画笔和荧光笔已支持 Shift 临时直线模式：当前草稿收束为起点和 45° 吸附端点，松开后在后续指针事件继续自由采样；与基线一致，不恢复被收束的旧草稿路径。Ctrl 不量化此类笔迹。模式切换不复制既有标注，不单独写历史，非法坐标和点数预算仍在变更前检查。压缩只减少 Vec 长度，保留的容量直到草稿/历史释放才归还，不宣称切换即释放内存。纯几何及 Canvas 输入测试覆盖组合键、缩放坐标、释放状态、撤销与取消；实际键鼠交互仍待人工验收。
`Ctrl+Z`/`Ctrl+Y` 撤销/重做，`Esc`、失焦和捕获丢失取消草稿。
`C` 打开 Windows 原生颜色对话框，`[`/`]` 调整后续标注线宽（开发入口暂限 1–256 图像像素）。
颜色和线宽在手势开始时复制到标注，切换工具属性不修改既有标注、草稿或重做历史；
显示与 PNG 导出读取同一份样式。`O` 在 25%/50%/75%/100% 不透明度之间切换。
透明度按整条标注合成，避免线段自交和箭头接合处重复加深；不同标注仍按绘制顺序累积。
普通画笔已对齐基线采样规则：不足 0.75 图像像素的移动不追加点，达到 2048 点时保留偶数索引后继续追加。菜单提供 0/25/50/75/100% 平滑设置，默认 0，当前会话切换工具保留；提交时按基线三轮相邻加权写入点集并保持两端，选择/变换/导出使用同一已平滑路径，撤销重做不再次平滑。平滑原地执行，无三轮点数组克隆，坐标按基线限制到 ±100000。荧光笔和折线不受此设置影响，完整连续数值输入及配置持久化尚未接入。已有采样阈值、2048 点降采样、权重结果、提交与撤销回归。
不透明标注跳过合成层，半透明层限定为标注包围范围并裁到目标尺寸；大图图层峰值仍待测量。
`Q` 选择圆角矩形，`Shift+[`/`Shift+]` 调整后续圆角半径；实际绘制按半边长限制半径。
矩形、圆角矩形、椭圆、三角形和多边形创建时，Shift 将包围框锁定为 1:1，Ctrl 按 10 图像像素量化尺寸。比例锁定取拖动两轴较大尺寸，并在图像边缘同时缩小两轴；先量化再限制边界，因此边界尺寸不强制为 10 的倍数。反向拖动按尺寸绝对值量化，与直线的有符号位移规则不同。最终鼠标事件决定提交坐标，零面积草稿不产生历史。已有纯几何和五种工具的 Canvas 输入回归。原生菜单新增会话内保留的等比例开关、0/5/10/20/50 px 尺寸步进，以及矩形/圆角矩形/椭圆的 1:1、4:3、3:4、16:9、9:16 比例预设；比例只有开启等比例绘制时生效，Shift 始终临时优先使用 1:1，三角形/多边形仍使用 1:1。已配置步进优先于 Ctrl 的临时 10 px，0 表示关闭配置步进。设置跨工具切换保留，不改变已有标注或撤销历史；边缘整体缩小保持比例。原生菜单另提供自定义比例和步进输入对话框：比例遵循基线的宽:高十进制语法，接受中英文冒号，每项限制 0.01–10000，输入最多 32 个 UTF-16 单元；步进接受 0–999 整数。输入错误留在对话框显示短错误，取消不覆盖原设置。同步系统对话框不跨消息循环持有 Canvas 借用，输入长度有上限，关闭释放窗口及栈上状态。真实 Win32 对话框自动化已覆盖合法提交与非法输入后取消；模型回归覆盖自定义值用于几何计算及失败保留原值。磁盘配置持久化及人工 DPI/键盘交互验收仍待完成。
选中矩形、圆角矩形、椭圆、三角形或多边形后，四角显示固定 DIP 尺寸的缩放手柄。拖动使用独立标注预览，不修改已提交文档；松手提交一次历史，取消不保留预览。四角调整沿用基线 16 图像像素最小宽高，不随尺寸修改线宽、圆角或其他样式；旋转后抵消中心变化，保持对角世界坐标固定。五类形状、四个角、0°/45°/90° 的 Canvas 输入回归覆盖预览、取消、提交、撤销重做及原地点击不写历史；模型测试覆盖反向创建、最小尺寸、非法柄编号与预算拒绝。实际鼠标手测和高 DPI 视觉验收仍待完成。

`T` 选择三角形，`P` 选择多边形，`PageUp`/`PageDown` 调整 3–12 条边，右键菜单提供相同入口。
三角形使用顶部中央顶点与满宽底边，多边形从顶部开始内接于包围框椭圆；
圆角复用基线的相邻边裁短与二次曲线规则。二者接入填充、虚线、透明度、历史和 PNG 导出，
测试覆盖几何边界、反向拖动和不同圆角的真实像素。尖角边框按 miter 上限保守扩展合成与命中范围。
`F` 切换形状填充，`Shift+C` 选择独立填充色；矩形、圆角矩形和椭圆均支持填充，
填充与边框整体应用标注透明度，屏幕与导出共享实现。像素测试覆盖反向拖动、空心/实心、
边框与填充色、圆角裁切和超大半径。默认设置只影响后续手势；`Ctrl+Enter` 或菜单“将当前样式应用到最后一个标注”可显式修改
最后一个标注，模型支持按索引修改。样式历史仅记录固定大小属性，不复制点数组；
修改可撤销/重做，无变化操作不制造历史或清空重做分支。`S` 切换选择工具，点击标注后显示选中框；此模式下 `Delete` 作用于选中集合，`Ctrl+Enter` 当前仅支持单选项，
无选择时不回退操作最后一个标注。`Esc` 清除选择，撤销/重做后旧选择按文档版本失效。
命中使用 D2D 几何、按绘制逆序查询，屏幕容差随缩放转换；空心形状内部不作为填充命中，
虚线间隙允许容差选择，完全透明标注不命中。多边形命中复用绘制路径，选择框不进入 PNG。
选择模式可拖动单个标注：普通标注移动过程中只记录预览偏移，不修改文档或历史；松开时以最终
指针坐标提交一次操作。取消、失焦和捕获丢失丢弃预览，导出快照不包含未提交位移。
模糊和马赛克例外：按下时只克隆一次选中笔迹的点缓冲，移动时由原始几何计算预览坐标，
保持视口和效果源图采样坐标不变，避免松手时采样位置跳变。`editor_effect_preview.rs`
负责原地更新及越界拒绝；Canvas 持有临时笔迹，取消、选择版本失效或提交时释放。
模糊像素仍共享原快照，不复制整图；真实 D2D/WIC 测试验证分数位移预览与提交逐字节一致，
并覆盖点缓冲复用、无累计位移、非法移动不部分更新、取消不写历史和撤销恢复。
这不等于完成主干模糊旋转/缩放采样语义或实际鼠标视觉验收。
提交仅复制被移动标注的点数组，原坐标由历史持有，撤销/重做转移所有权以避免浮点漂移；
额外点数计入同一预算，超限拒绝提交并清除预览。拖动期间禁止滚轮改变视口。
选中后可用 `+`/`-` 或菜单等比放大/缩小，以标注原始点包围框中心为基点，同时缩放线宽和圆角。
每次缩放提交一次历史，撤销精确恢复原点数组和属性；无选择时不操作其他标注。
当前线宽/圆角或历史点预算超限会拒绝整个操作，不静默截断比例；模型及真实 PNG 像素已有回归测试。
选中后用 `,`/`.` 或菜单每次旋转 15°。旋转角度独立保存，历史只记录角度；
屏幕与导出临时应用 D2D 变换并在结束/失败时恢复，命中使用逆变换，选中框和钉图范围同步更新。
半透明图层按逆变换后的目标边界限制，避免裁掉旋转后进入画面的内容。测试覆盖旋转像素、
后续标注不受变换污染、逆变换命中和精确撤销；任意角度及多 DPI 人工交互仍待验收。
菜单支持单独增加/减少选中标注的宽度或高度，以原局部轴包围框中心缩放，保留旋转角。
线宽与圆角按横纵倍率的平均值变化；非法轴参数或预算失败不会提交另一轴的部分变化。
已有中心/角度/样式/撤销模型测试及椭圆非等比导出像素测试；当前不支持负倍率翻转。
`Ctrl+D` 或菜单复制选中标注，沿用基线向右/向下各偏移 12 图像像素、追加到顶层的规则，
复制后选中新副本。副本保留全部样式和旋转角，点数组独立且受总预算约束，复制可撤销/重做；
无有效选择或预算不足时不改变文档，也不清空重做分支。该操作不涉及系统剪贴板。
直线和独立箭头选中后显示两端控制柄，可拖动任一端点，另一端在旋转后仍保持可见位置不变。与贝塞尔共享 `desktop/path_handles.rs` 的手势预览与取消/提交生命周期，模型事务分别拥有；临时预览不进入导出，松开一次提交，单击未移动不制造历史。已覆盖两种工具、两端、0°/45°/90° 旋转及预算失败，实际键鼠交互仍待人工验收。
尚未接入框选、多选组变换、完整鼠标变换控制柄及完整选中属性面板；
大文档命中耗时和拖动显示仍待人工/性能验收。
`D` 在实线、`dash-1`、`dash-2` 间切换，作用于直线、箭头杆身、形状边框及普通画笔，荧光笔和折线保持连续实线。
普通画笔、荧光笔和折线已改为单条 D2D 路径绘制，多点笔迹仅调用一次 DrawGeometry，实线使用圆端点/圆连接，虚线使用平端点且相位沿整条路径连续，不在每个采样点重置。单点笔迹保留圆点绘制。实线 stroke style 不传自定义虚线数组，避免 D2D 返回 E_INVALIDARG。真实像素测试覆盖两种线宽、两种虚线与不同采样密度的结果一致，以及荧光笔/折线不受虚线设置影响。路径仍在绘制时创建，实际长笔迹 CPU/GPU 性能收益尚未测量。
虚线使用基线的 `[8,4]` / `[4,2,1,2]` 图像空间长度，不随线宽改变；形状填充不受影响。
已有真实像素测试验证两种线宽的虚线周期一致。虚线样式目前在绘制时创建并释放，尚未做资源缓存。
`H` 或菜单选择荧光笔，笔迹作为连续圆端点路径参与历史、选择、变换与 PNG 导出。
所有荧光笔先在一个共享图层内绘制，再统一以 35% 叠加在其他标注下方，重复涂抹不突破该上限；
原生内部 opacity 表示层内相对透明度，旧数据导入时仍需转换，不可直接按同字段解释。
屏幕、导出和命中顺序已统一；像素测试覆盖多笔自交、半透明笔迹和普通标注覆盖顺序。
共享图层仅覆盖可见荧光笔笔迹的合并包围范围，包含线宽、旋转、拖动预览与抗锯齿安全边，且裁剪到绘制目标；完全透明或屏幕外的笔迹不触发共享图层。范围计算不创建临时点数组。模型范围测试及真实 PNG 像素测试覆盖旋转和目标边缘裁剪；实际 GPU 分配与大图峰值仍需性能测量，不能把包围范围缩小直接等同于已测得的显存节省。
`K` 或右键菜单选择贝塞尔曲线，拖动按基线规则生成四个三次控制点，移动时替换控制点而非追加采样。D2D 曲线路径在屏幕、PNG 导出和选择命中间共用；控制点保留在历史中，支持既有移动、旋转、缩放和复制模型，包围范围使用控制点的保守范围。已验证曲线中点的像素与命中、控制折线/端点弦不误命中、撤销重做及点数准入。当前鼠标入口只接入单段创建。编辑模型已实现局部控制点移动与两端多段延伸：移动锚点携带相邻切线控制点，延伸沿用上一段切线，最多 64 段；坐标与总点数准入在提交前验证，历史持有原曲线以精确撤销。真实 PNG 测试覆盖模型编辑后的多段路径及撤销重做结果。选择曲线后已显示锚点、切线控制点及辅助线，可拖动控制柄；命中优先于整条标注移动，拖动只持有单条曲线预览，松开提交一次历史。预览不进入导出，失焦/取消/版本变化丢弃预览，拖动期间禁止滚轮缩放；异步打开图片也检查控制柄手势，防止覆盖正在编辑的文档。局部坐标逆变换与包围中心补偿已覆盖旋转 0°/45°/90° 的输入测试，确认未编辑控制点的可见位置不漂移。辅助线只在屏幕显示，不进入 PNG。两端延伸手势已接入：端点外侧 28 图像像素显示加号延伸柄，拖动预览新增一段，松开一次提交；仅单击或取消不添加曲线段。旋转曲线延伸也补偿中心变化，测试覆盖两端、0°/45°/90° 旋转及原有控制点可见位置不变。达到 64 段后隐藏延伸入口，模型仍独立拒绝超限。菜单已提供贝塞尔曲线箭头开关，随工具配置在会话内保留，并通过当前样式显式应用到已有曲线；样式历史、复制和变换保留该属性。箭头按基线使用有界自适应采样、弧长裁短与末端切线方向，箭头长度不超过总弧长的一半；最多 4097 个采样点，杆身按裁短后的采样路径绘制，无箭头仍绘制精确三次曲线。屏幕、PNG 与命中共用箭头几何，半透明层和钉图范围保守覆盖头长及头宽。已有退化/反向曲率几何测试、头部 PNG 像素、整体透明度、样式撤销与原生命中测试。旧数据导入仍未完成，真实控制柄及菜单键鼠交互仍待人工验收，不能视为完整贝塞尔功能迁移。
`U` 或右键菜单选择折线，按基线拖动采集路径点并在松开时提交，不是多次单击创建。保留各段和闭合回路，单击零长度草稿不写历史；提交路径为实线，不套用直线角度/步进约束或贝塞尔延伸柄。选中后可拖动任意已有顶点，复用路径控制柄预览及旋转中心补偿，模型将整个目标点集计入历史预算。已有多段 PNG 像素、闭合路径、顶点编辑、撤销与真实 Canvas 输入测试；大量顶点的控制柄显示性能及人工交互仍待验证。
笔刷（区别于普通画笔）正在迁移：`brush_dynamics.rs` 已实现基线的速度权重与非递增时序处理；`paintbrush_samples.rs` 已实现起收笔半径、权重插值和四层 source-over 透明度补偿。采样为借用点集的流式迭代器，不分配整份派生点数组或前缀距离数组；输入最多 65536 点，每段最多八个样本，单点/总长度不足 1 像素保留圆点语义。已验证起收笔、中段半径、权重、长路径样本上限及四层中心透明度。已通过 `J` 和原生菜单接入笔刷入口，默认宽度 16、平滑 50%，与普通画笔分别保存会话内宽度、透明度及平滑设置。笔刷跳过不足 0.75 像素的采样，但不执行普通画笔的 2048 点减半，仍受单笔/总点数上限约束。每层将圆盘与连接带组成一个 winding 路径后一次填充，避免自交重复加深；四层逐次创建释放，不保留四份派生点数组。屏幕、PNG 与命中使用原生带状几何，已有起收笔、半透明自交、软边和收笔命中测试。已接入运动权重：窗口使用 Win32 鼠标消息时间戳而非处理时间，草稿使用吸附前图像坐标估算速度；仅实际接受的采样前进时序，拒绝或跳过采样不改变权重。每个已存点对应一个无量纲权重，提交/取消清除草稿时钟，撤销、复制、变换和导出保留权重而不重新计算速度。Shift 直线模式同步调整点和权重长度。真实 PNG 测试验证快速绘制比慢速绘制更窄，模型测试覆盖历史、缩放、快照及准入失败。每次按下重置手势时钟，展开 32 位毫秒时间戳回绕，重复/倒退时间不增加时差；修饰键重算沿用最近指针时间，不制造运动采样。测试覆盖回绕、陈旧时间与 Canvas 权重计算。Win32 消息时间不是设备原始采样时间，仍可能存在系统消息合并，真实高频输入及消息积压下的手感待验证；已接入基线的固定窗口 RDP 简化：每 128 段锚定一个窗口，同时约束中心线与半径误差（0.18 图像像素），保留折返、笔尖和窗口边界；流式适配器每次只持有 129 个样本，使用定长迭代栈而非递归或整份 keep 数组。简化在半径层缩放前执行，显示、PNG 和原生命中共用结果。测试覆盖窗口边界无重复、半径突变/折返保留及追加尾部不重分割完成窗口。65536 点直线夹具从 65816 个派生样本降为 519 个，迭代器状态在当前构建中为 1776 字节，另有简化函数的定长工作栈；这些是样本数与结构大小证据，不代表整进程或 D2D 峰值内存。大笔迹实际绘制耗时与几何内存仍待性能验证。
已接入当前十二种创建工具的内存内独立配置：线宽/线型分别保存，相关形状保存圆角，
多边形保存边数，画笔保存透明度；切换只恢复该工具拥有的属性，不覆盖当前颜色和填充。
选择/平移工具不写入创建工具配置，切换不修改文档历史。尚无完整属性面板、配置持久化
和旧配置导入。已实现的十二种创建工具默认值已对齐固定基线：线宽 3（笔刷为 16）、颜色 `#ef4444`、透明填充、圆角 0、实线、多边形 6 边与完整不透明度；荧光笔仍通过共享 35% 图层合成。默认属性与原生导出颜色已有回归测试。未迁移字段仍待基线对齐；当前开发行为不能代表完整设置兼容，
该开发范围也不是旧配置的导入限制；正式兼容仍需逐项核对。颜色对话框尚待人工交互验收。
`Ctrl+S` 打开 Windows 保存对话框，将已提交标注按原图尺寸导出 PNG；后台任务共享源像素，
复制有界标注快照，不包含草稿或重做分支。一次仅允许一个导出，完成检查只在任务期间启用。
当前拒绝覆盖任何已有文件；写入失败保留本次新建的不完整文件并报告错误，不静默成功。
导出使用 WIC 离屏合成和 PNG 编码，保留 alpha；已通过原生解码往返及标注像素测试。
落盘使用 64 KiB 分块缓冲，不复制整份编码 PNG。编码流与离屏位图仍有额外内存，
257 MiB 的编码结果限制不是导出峰值内存限制；大图峰值和编码过程限额仍需专门验证。
待补：保存对话框人工验收、可取消导出与原子发布；退出目前等待正在进行的导出，慢存储可能延迟退出。
一次手势仅提交一次历史；新增与删除共用有界操作记录，保留最近 50 步撤销。
`Delete` 或菜单提供删除入口；选择模式操作选中项，其他工具模式保留删除最后一个标注的开发入口。
撤销/重做通过转移标注所有权恢复顺序，不复制点数组或图片；丢弃重做分支、淘汰历史时
释放对应记录独占的点，并更新总点数预算。测试覆盖中间删除、原位恢复、分支释放和 50 步边界。
单笔最多 65,536 点、保留点总数最多 1,048,576、最多 4096 标注；超限取消草稿并报告错误。
这是开发入口，不替代基线的工具快捷键或钉图交互。WIC 在窗口创建前解码，尺寸及像素预算显式受限。
隐藏窗口 smoke 真实创建 D2D target、丢弃并重建资源、提交两次绘制并销毁 HWND；有图片时还绘制四种基础标注。
这不能代替像素正确性及人工显示验收，当前模型也尚未完成旧标注格式的序列化兼容。
`--pin` 以无边框置顶工具窗口启动单张图片，使用预乘 alpha 的 DirectComposition 交换链，
没有持续呈现循环；硬件 D3D11 创建失败时尝试 WARP。平移工具下左键拖动窗口，
`Ctrl+滚轮` 围绕指针缩放窗口；单个交换链尺寸限制为每边 8192 像素及合计 16,777,216 像素。
基础标注、撤销和 PNG 导出复用普通工作区实现。设备丢失只安排一次重建尝试，
持续失败关闭窗口并返回错误，避免不可见窗口继续拦截输入。
原生测试已读回透明/半透明 GPU 像素并验证交换链尺寸变化和资源重建。
平移模式通过 `SetWindowRgn` 限定图像 alpha 区域，并合并当前标注包围范围；编辑模式恢复
完整窗口区域，允许在透明内容上继续画图。alpha 区域启动时预计算、按图片身份缓存，
只有内容版本、窗口尺寸、DPI 或编辑/平移模式变化时才更新，不增加鼠标轮询或后台定时器。
线性采样边缘保留一像素安全带。精确区间超过 65,536 时保守降级为逐行包围段并记录诊断，
标注区域目前也使用保守包围矩形，因此不声称所有透明孔洞都已逐像素穿透。
钉图已接入 WinEvent 前景与窗口显隐/销毁通知，合并消息后按需更新窗口层级，不添加常驻轮询。
让位前取消草稿并释放鼠标捕获；主动点击可重新取得优先级，前景变化后重置该声明。
hook 由安装线程的 RAII owner 释放；已测试通知合并、无效目标、重复注册和释放后重新安装。
待补：跨进程真实点击穿透验收、复杂标注精确命中、真实系统对话框层级与安全桌面保护、
多屏/DPI 交互、多窗口共享设备与故障注入。当前事件测试不等于这些交互已完成验收。
尚未完成完整钉图交互、完整编辑、托盘、单实例及完整恢复，N02/N03 因此仍未标为完成。

| 模块职责 | 所有权与约束 |
| --- | --- |
| 宿主 | 消息循环、托盘、快捷键、单实例、退出；不得承载编辑算法 |
| 窗口与输入 | HWND、DPI、焦点、命中检测、输入捕获和释放；unsafe 集中封装 |
| 编辑模型 | Unit、标注、工具状态、命令、历史；不依赖 HWND 或图形 API |
| 渲染 | 图像、文字、局部失效、设备资源恢复；不修改持久化模型 |
| 媒体资源 | 解码、纹理、共享来源与字节预算；句柄和任务有明确唯一 owner 或租约 |
| 持久化 | 格式兼容、原子写入、资产引用、迁移恢复；不与渲染生命周期耦合 |
| 集成 | 复用既有 Loom/Talk/Tea 合同；不复制其他仓库内部实现 |

已有原生算法先查实际 Tauri 耦合再抽取，不直接复制整份实现到新宿主。
迁移期间旧入口作为对照保留；生产路径最终只保留一套业务 owner。
标准文本输入优先使用系统能力，保留中文 IME、剪贴板、键盘导航与无障碍语义。

## 开发任务与退出条件

源码分组、持久化差异和浏览器兼容风险见 [功能对照台账](WINDOWS_NATIVE_PARITY.md)。

所有任务初始为待完成；勾选必须关联实际源码、测试和证据，不能以文档或编译成功替代运行验收。

- [ ] N01：建立完整功能台账。逐项核对 `docs/FEATURES.md`、实际调用点和现有测试，记录迁移 owner、基线行为、新实现、测试与未验项。
- [ ] N02：建立独立原生工程与构建检查。无 Qt/Tauri/WebView 依赖，提供阻塞消息循环、退出清理和可核对的包身份。
- [ ] N03：原生画布纵向闭环。打开图片、钉图、移动缩放、基础标注、撤销、导出、关闭释放；测试多显示器、负坐标和混合 DPI。
- [ ] N04：迁移完整编辑模型。选择、裁剪、旋转、橡皮擦、画笔、高亮、形状、箭头、文字、编号、贝塞尔、模糊和马赛克及属性面板。
- [ ] N05：迁移截图与桌面输入。区域/窗口/长截图、取色、HDR 回退、捕获排除、系统对话框优先级、穿透、紧急退出和恢复。
- [ ] N06：迁移工作区与数据。链接、分组、历史、回收站、参考库、设置、文件命名、拖放、剪贴板及启动恢复。
- [ ] N07：迁移 Live。共享捕获、原生纹理呈现、资源准入、输入授权、源窗口恢复、隐藏/遮挡和关闭清理。
- [ ] N08：迁移能力与扩展。Loom、Art、OCR/翻译、Shader、声明式/JavaScript Surface、插件菜单和快捷键、Talk/Tea 相关入口。
- [ ] N09：迁移投送与 Tile。设备身份、配对、TLS、接收策略、双向编辑 journal、LiveRelay、墙面与输出进程生命周期。
- [ ] N10：独立原生构建与发布门。测试、锁定依赖、安全检查、版本、图标、清单、签名兼容、便携包与构建来源证据。
- [ ] N11：功能等价、性能和长时间运行验收；清理旧依赖前形成完整功能差异表，所有差异闭环或由用户明确批准。
- [ ] N12：交付可手测包及报告，由用户完成最终验收与 main 合并；移除旧代码的破坏性操作另行取得许可。

顺序：N01/N02 → N03 → N04–N09 → N10/N11 → N12。
每项按可运行、可回归的小批次实现；不把最小演示窗口当作完整替代版本。

## 必须先解决的兼容问题

基线包含 JavaScript Surface、WebGL Shader 和前端插件能力，不能假设迁移 Win32 后自然兼容。
N01 必须列出实际 DOM/CSS/JS/WebGL 依赖及协议约束，并分别选择原生适配、受限脚本运行时或经批准的合同迁移。
无 WebView 不必然等于无脚本运行时，但不允许暗中以嵌入浏览器完成“原生迁移”。
无法保持的功能必须明确记录为阻塞项，不得静默删去或将 mock 当完成。

基线前端状态、IndexedDB journal、原生文件、图片资产、凭据和设备身份需分别盘点读写方。
在证明转换往返、未知字段保留及中断恢复前，不允许新程序写入用户现有数据目录。
对照测试使用脱敏数据副本和独立测试身份；不得并发争抢真实快捷键、设备连接或恢复日志。
旧包和原始数据保持可用。试验失败通过停止候选、恢复旧入口回退，不通过 reset/覆盖用户数据回退。

## 资源生命周期规则

- 无活动内容时不运行连续渲染循环；系统重绘、模型变更或新帧触发必要绘制。
- Live 按活动消费者调度，静态钉图不因 Live 而持续重绘；遮挡后的恢复需要有效帧和状态。
- 图像、纹理、历史、缓存、队列与任务均有预算、owner、取消路径及异常释放路径。
- 高频指针预览不逐点写入持久化；移动可合并，按键边沿、退出和取消不可丢失。
- 异步结果携带版本/生命周期标识；关闭后迟到结果不得复活窗口、资源或授权。
- 设备丢失、分辨率变化、锁屏、休眠恢复、正常退出和异常退出分别验证。
- 模糊替换需正确就绪后切换，保留基线橡皮擦结束不闪烁的回归；不可展示过期弱模糊泄露内容。

## 测试与性能验收

图片粘贴最新增量：PNG 不可用时选择 CF_DIBV5，其次 CF_DIB。新增 `clipboard_dib.rs` 在后台解析 40/52/56/108/124 字节头、16/24/32bpp RGB/位域布局；先检查尺寸、完整缓冲、步长、可选色表偏移及连续/不重叠的掩码，再分配受预算约束的输出。保留 top-down/bottom-up 方向，规范化 555/565/32 位颜色并转为 PBGRA；普通 32bpp BI_RGB 保留字节不误作 alpha，V5 显式 alpha 掩码保留透明度。拒绝外部/嵌入 ICC、未实现的校准变换、压缩及索引色布局，逐行响应取消且不返回部分图像。新增夹具覆盖掩码、padding、所有截断位置、无效尺寸/配置、取消，以及本分支 DIBV5 输出→后台粘贴的无剪贴板回读。447 项库测试与 7 项启动测试通过。先前 PNG-only 描述为历史里程碑；索引色/RLE/ICC、智能文件列表与真实系统粘贴仍未完成。

PNG 粘贴已接入 `Ctrl+V` 和右键菜单。仅显式操作时读取注册 PNG，先校验 HGLOBAL 大小上限及签名，并在共享预算内复制字节；锁内不做 WIC 解码，锁外再确认文档替换。原系统句柄只解锁、不释放，不清空或改写剪贴板。`open_job.rs` 新增编码字节输入，与文件打开共用单任务、COM 线程、取消、文档身份和命中区域准备。WIC 文件/内存输入复用同一尺寸准入及 PBGRA 转换实现；内存流在 decoder/converter 释放前保持输入字节存活，不落盘。新增内存 PNG→后台解码回归覆盖裁剪、alpha、输入缓冲释放、损坏和预取消路径；自动测试不读取用户剪贴板。DIB/DIBV5、文件列表、文本/URL、多对象工作区粘贴尚未迁移，真实系统粘贴及第三方来源仍需人工验收。

复制宿主验证现已接入普通窗口和钉图启动冒烟：真实 HWND/State 启动后台双格式编码，重复启动不形成第二次交付；通过与生产共用的 `poll_using` 执行结果回收及准入，仅替换最终发布回调。普通复制恰好交付一次，真实 Esc 消息取消后零交付，同像素/版本的新文档替换后零交付，测试结束恢复原编辑器及历史。为保证不访问用户剪贴板，该测试不泵复制定时器消息，显式调用共用轮询；因此它证明宿主任务与准入链，不证明真实剪贴板发布或复制定时器消息调度。439 项库测试、7 项启动测试通过，新增标记为 `native-copy-smoke: prepared=true once=true escape=true stale=true clipboard-access=false`。

图片复制最新增量：同一个 WIC 合成位图现生成 PNG 与 CF_DIBV5，不重复绘制；DIBV5 使用 124 字节头、顶向下 32bpp BI_BITFIELDS、明确 RGBA 掩码与 sRGB，PBGRA 经 WIC 转为非预乘 BGRA，保留 alpha。全部 HGLOBAL 及预算先准备完成，再一次 Open/Empty 发布两种格式；发布中途系统失败可能只留下首个格式，不能承诺恢复原剪贴板。新增位图头和真实 WIC 转换测试，扩展后台任务测试逐像素比较 PNG 与 DIBV5，允许预乘往返的 1 字节舍入误差，并验证两份缓冲均被释放。439 项库测试与 7 项启动测试通过；所有测试均不访问真实系统剪贴板。下文 PNG-only 描述为先前里程碑，当前已增加 DIBV5；CF_HDROP、命名/文件缓存、第三方粘贴与真实系统发布验收仍未完成。

原生普通/钉图窗口新增 `Ctrl+C` 与“复制 PNG 图片”菜单，复制已提交文档的裁剪区域、栅格和矢量标注，保留透明度。`copy_job.rs` 单后台线程复用 WIC 导出，无待处理队列；活动期间才启用定时器，Esc 取消、文档身份/版本变化、活动手势或窗口关闭均阻止迟到发布。WIC 编码本身不保证即时中断，退出通过取消后 join 收尾。PNG 字节及发布 HGLOBAL 分别预留共享预算；注册格式/内存准备失败前不清空剪贴板，EmptyClipboard 后的系统发布失败仍无法恢复原第三方格式。三个无磁盘、无剪贴板访问测试覆盖真实 PNG 编解码的裁剪/标注/alpha、已完成结果取消和换文档拒绝、活动草稿与销毁回收；全量 437 项库测试、7 项启动测试通过。此阶段仅发布注册的 PNG 格式，CF_DIB/DIBV5 回退、智能 CF_HDROP 文件表示、命名/缓存清理、实际第三方粘贴兼容性均仍未完成，不能据此宣称主干智能剪贴板已迁移。

打开图片、保存图片及拖入确认现共用每窗口的 `file_dialog::Guard`，避免不同文件入口在嵌套消息循环中交叉启动；拒绝重入不会解除外层门闩，窗口之间互不阻塞，错误/取消/unwind 自动归还状态。重入的 `WM_DROPFILES` 仍先接管句柄，退出时释放，不因忙碌而泄漏。真实 HWND 冒烟在持有门闩期间分别调用打开、保存和拖入入口，断言没有启动工作线程、没有改文档、外层门闩仍有效；解除后实际拖入仍完成后台加载。434 项库测试与 7 项启动测试通过；此测试不弹出真实系统模态框，不替代人工交互验收。

原生保存对话框已增加重入门闩和文档凭证：同一窗口保存框活动期间拒绝再次启动导出，所有取消/错误返回自动解除门闩；保存路径返回后，只有原文档身份及版本仍匹配且没有活动草稿时才取得只读导出快照。模态期间切换或修改文档会在创建文件之前拒绝导出，避免新文档写入为旧文档选择的路径。提示框调用前释放 Canvas 借用，保留嵌套消息循环的安全边界。三个新增无磁盘测试覆盖重入/错误回收、同版本文档替换、普通编辑失效及快照独立性；全量 432 项库测试、7 项启动测试通过。系统保存框的人工交互和异常关闭仍需验收。

PNG 导出的合成阶段已补齐共享像素预算：在创建 WIC 输出与 D2D 底图/独立标注上传副本之前，分别预留裁剪输出字节及实际上传字节。上传资源只在合成期间持有租约，输出位图与租约一起存活到编码结束，字段顺序保证先释放 WIC 再归还预算。独立预算测试验证 4×4 底图、2×1 标注、2×2 裁剪的峰值准入为 88 字节，返回后只保留 16 字节输出租约；两阶段分别拒绝时全部归还，文档不变。完整测试为 429 项库测试与 7 项启动测试通过。本项只约束明确像素缓冲的逻辑字节，不宣称覆盖 D2D 驱动开销、WIC 编码器内部缓存或 PNG IStream 的全部分配；这些仍需受控峰值测量和进一步边界处理。

单文档原生窗口已接入 Shell 文件拖入：`WM_DROPFILES` 提取最多 32767 UTF-16 单元的单文件路径，所有成功/失败分支通过 RAII 调用 `DragFinish`，且在打开模态确认框前释放数据包。拖入复用编辑丢弃确认和 WIC 后台解码；加载中明确拒绝新请求，多文件批次明确拒绝、不静默只取首项，不执行拖入路径、不增加常驻服务。确认框及文件选择框返回后另核验文档凭证，避免重入期间文档已变化仍被替换。新增中文、空格、长路径、空/多文件/超限数据包测试；普通窗口与钉图启动测试通过自建 Shell 数据包向真实 HWND 同步投递，再由真实定时器完成后台加载与文档替换。全量 427 项库测试和 7 项启动测试通过。Explorer 实际拖动、不同完整性级别、编辑丢弃确认的人工交互未验收；多文件工作区导入、文件夹/URL/虚拟文件导入仍未迁移。

模糊后台请求也已接入文档身份凭证，覆盖选择强度连续调整的前项继承、定时器有效性检查和结果提交三处边界。新增真实计算线程回归证明：共享像素且版本相同的新文档，不能接收旧准备缓存，也不能继承旧请求的累积强度或替换旧索引的效果。两项回归修复前失败、修复后通过，完整测试为 424 项库测试和 7 项启动测试通过。凭证只共享身份，不增加像素复制；已有单线程、单待处理请求与取消回收模型不变。

异步加载与文字模态提交已补齐文档身份凭证：旧实现仅比较版本及共享像素，替换为空白文档或共享相同像素/版本的新文档时可误接收迟到结果。新增 `editor_document_stamp.rs` 复用 Editor 的 Arc 身份并保存版本，凭证不复制图像或引入轮询，存活期间防止身份地址被回收复用；打开图片与文字提交复用该验证。三个新增回归在修复前失败、修复后通过，覆盖共享图片替换、空白/导出快照替换以及文字模态返回。全量为 422 项库测试和 7 项启动测试通过；此增量不代表所有异步宿主均已完成审计。

本次增量补齐透明窗口命中区域缓存的视口依赖：平移或缩放改变但文档版本、客户区尺寸和 DPI 不变时，也必须重新生成区域。新增缓存键回归检查分别覆盖两个方向的平移和缩放。

本次完整复跑通过 419 项库测试和 7 项启动测试；格式、Clippy 和有效代码行数检查通过。此前两个启动测试曾报 `Cancelled flip changed document`，后续通过测试线程的 `SetKeyboardState` 固定 Ctrl 按下，稳定复现同样错误：原窗口过程把 Ctrl+Esc 排除在取消分支之外，后台翻转继续提交。现已修复为送达画布的 Esc 不受 Ctrl 状态屏蔽；普通窗口和钉图的真实 HWND 冒烟均增加普通 Esc、Ctrl+Esc 两种取消断言，检查版本及原图存储均不变。键盘状态只在同步发送消息时局部修改，并通过 RAII 恢复，不注入系统输入、不修改其他线程。原始失败未记录修饰键状态，不能断言所有历史间歇性失败均由此引起。上述检查也不等于跨进程点击穿透、跨屏 DPI 或完整功能人工验收。

功能权威清单从 [功能矩阵](../FEATURES.md) 展开，不仅覆盖截图与画笔。
同一输入夹具比较编辑模型、导出内容、数据恢复和授权结果；图像误差阈值按功能确定并记录。
Win32 输入、透明合成、中文 IME、DPI、跨机投送和异常恢复必须有原生运行证据。

性能场景至少包括：空托盘、单张 4K、十张钉图、复杂标注、连续擦除、长截图、单路/多路 Live、
投送、关闭全部内容后的恢复，以及长时间循环开关。
旧包与新包使用相同机器、图片、屏幕、工作负载和采样方法，分开运行并记录预热及重复次数。

记录整个进程树的 private working set、private bytes、CPU 时间、线程/句柄数量、GPU 内存归属、
启动时间、输入到显示延迟和帧时间分布。共享页和共享 GPU 资源不直接求和当唯一物理占用。
记录 EXE/源码哈希、测试场景和环境；不把 EmptyWorkingSet 后的数字作为节省分配内存的证明。
数值预算在受控基线后确定，目标为可重复的低开销且交互不退步，不预先承诺 13 MB。

源码测试、静态检查、编译、真实运行、打包和人工验收分别记录状态。
main 后续若变化，显式更新对照差异与基线；不自动吞并或覆盖本分支实现。
