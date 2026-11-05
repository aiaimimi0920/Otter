# Windows 原生功能对照台账

## 本轮收口与交付决定（v545）

用户认可本次Windows迁移基本完成，要求停止细化，提交推送当前分支并生成本地完整手测包，由用户后续使用反馈驱动修改。本轮开发据此收口，不继续新增验收子项；已通过项和下方资源／物理环境等未验边界均保留，不改写为全面测试通过。交付沿用v0.2.33.545，提交推送不包含main合并、公开发布或替换现有安装。有效范围见[修订计划](WINDOWS_NATIVE_REVISED_PLAN.md#本轮迁移开发收口转入用户手测v545)。

## 当前v545：贴图菜单与普通静态工作区正式退出已通过

右键消息修复已进入完整v545内部包。用户本次按新测试贴图右键“退出 Otter”指引操作后反馈“已退出”；自动观测root exitCode0、4个自有进程自然归零、监听空、工作区图片及身份保留，无Job强制兜底，关闭普通静态工作区菜单退出子项。详见[本次真实路径与边界](WINDOWS_NATIVE_BUILD.md#v545-普通菜单退出与自然回收通过)。菜单路径依据人工完成反馈，不伪称自动UI轨迹；活动Art/Live、未保存选择、资源长稳及物理环境不由本次覆盖。源码/版本/包未变，不重建、不重测已过项。A/B/C和中文IME保持用户通过，Canvas暂缓非阻塞、资源后置、P4环境延期保持；以下旧“普通退出未验”和候选号仅为当时状态，总迁移未宣称完成。

## 当前 v544：资源排查后置，普通退出仍待操作确认

用户要求CPU/内存/GPU/磁盘及资源/性能长稳留待后续功能开发后全盘检查，本轮不采样、不优化、不作为推进前置门，亦不记达标。用户同意操作新测试窗正式退出菜单，由开发侧自动观测；600秒未见根退出、4进程由自有Job兜底归零。尚未收到实际看到/点击的确认，不能当产品缺陷或正常退出通过；不重试旧自动菜单、不重测Loom。详见[本轮边界](WINDOWS_NATIVE_BUILD.md#v544-资源后置与用户菜单退出观测)。A/B/C及人工IME保持用户通过，Canvas保持暂缓非阻塞，物理环境及非资源退出边界仍保留，无生产/版本/构建/包变化。

## 当前 v544：人工中文 IME 已获用户手测认可并关闭

用户按上轮中文输入手测指引反馈“中文输入正常”，本项按用户验收通过并关闭，不再要求重复测试或逐项补证。没有新增自动化执行，不推定未提供的具体编辑器、输入法、保存动作或其他模态/旋转/物理环境结果。A/B/C保持用户通过，P3保持用户决定暂缓且非阻塞；必需剩余仍为P2及P4未验环境，不把本次反馈扩大成全部迁移完成。以下历史记录中的人工IME未验以本结论为准，详见[当前验收清单](WINDOWS_NATIVE_ACCEPTANCE.md)。无生产源码、版本、构建或包变化。

## 当前 v544：普通 Live 已获用户手测认可并关闭

用户按普通Live清单手测后反馈“基本正常”，明确同意关闭测试，后续实际使用异常再提出。P1/C 按用户验收关闭，不再要求重复测试；原失败及自动化绑定限制保留为历史，不再代表当前普通Live待办。未取得逐项日志，不臆造具体Art结果、源事件次数或进程/监听/资源测量，也不据此关闭P2。A/B保留通过，P3按用户决定暂缓且非阻塞；当前必需剩余仅P2、P4，详见[当前清单](WINDOWS_NATIVE_REVISED_PLAN.md#当前交付收口有限剩余项v544)。

## 当前 v544：Canvas 历史失败按用户决定暂缓且非阻塞

用户已明确要求暂时忽略 Canvas 原 `519.705ms / 250ms` 历史失败，以后实际使用出现并报告时再处理。P3 不再属于当前必需验收或迁移阻塞，不继续专项调查、复测或单独要求验收；保留原始证据，不记为已修复或测试通过，也不修改生产期限、断言、内存/取消/回收合同。当前必需剩余为P2和P4，详见[有效收口清单](WINDOWS_NATIVE_REVISED_PLAN.md#当前交付收口有限剩余项v544)。以下旧记录中的 P1/P3 阻塞结论已由用户最新决定取代，不代表P2/P4通过。

## 当前 v544：普通 Loom 正常关闭已实证

在用户明确批准的无截图控件树/键盘菜单范围内，新profile普通Loom主窗Alt+F4后exitCode0，两秒后样本整树active0，监听空、无Job兜底。壳按既有kill/wait回收自有daemon，不称daemon自愿退出。Otter仍未选到退出项，菜单绑定/几何限制保持，最终自有Job兜底；不算Otter或双产品自然退出。详见[本批证据与边界](WINDOWS_NATIVE_BUILD.md#v544-控件树范围的普通退出补验)。

仅关闭P2的Loom关闭/树回收子项，不重跑该成功项；普通C、完整业务/资源/长稳、Canvas原250ms及P4继续开放。没有截图、坐标或替代输入API，没有生产/版本/普通产品/完整包变化，总迁移未完成。

## 当前 v544：有限剩余清单与有界调查结论

A/B用户通过保持，P1普通C、P2双普通产品完整业务/资源/退出/长稳、P3 Canvas原250ms失败、P4物理环境/跨机/人工IME为有限剩余组，不以版本或测试数推完成率。[当前清单](WINDOWS_NATIVE_REVISED_PLAN.md#当前交付收口有限剩余项v544)记录类别、证据不足、下一步和关闭条件。

普通双产品退出补验已启动，但首次自有Otter观察即出现标题/控件树与截图身份不符；无输入、无重试，两树Job兜底，不能记正常退出通过。Canvas本轮只读审查达到停止条件，当前5份相关源码未变、原慢阶段缺细分/CPU/调度证据；不再用快样本覆盖519.705ms失败或自行降门禁。当前只枚举1个逻辑显示器，不能验物理双屏/混合DPI。详见[证据与边界](WINDOWS_NATIVE_BUILD.md#v544-有限收口审查与普通退出工具阻塞)。无生产源码/版本/包变化，总迁移未完成，不称仅剩用户全面手测。

## 当前 v544：持续 WGC→普通 Loom→Art 业务取得实证

新显式门600秒内完成120次尺寸换代与显式Art执行、共121次真实上游调用，逐轮核正式输出全部像素及worker回收，结束核源恢复。仅一个Rust通过，测试树自然归零，普通Loom仍由Job兜底；两轮夹具前置失败和首次编译错误保留。详见[范围与边界](WINDOWS_NATIVE_BUILD.md#v544-真实-wgc-与普通-loom-持续-art-负载)。

仅cfg(test)变化，普通产品/包/版本544不变，不将夹具负载替代普通C、可见资源、完整业务长稳或人工/物理环境验收。A/B人工通过、Canvas原失败及其余开放边界保持，总迁移未完成。

## 当前 v544：Live 连续使用回归取得实证

新增一个真实 WGC 显式运行门通过：两种自有窗口／两种启动边界各三轮，合计12个会话、36次resize、24次源控制启停，逐轮验证新帧尺寸／像素、输入反馈、松键、源恢复和worker／恢复child回收。测试树自然归零且无Job兜底；首轮夹具在最小化后读可见外框的原失败保留，后续只纠正测试前置顺序。详见[运行范围与边界](WINDOWS_NATIVE_BUILD.md#v544-live-连续生命周期运行门)。

仅cfg(test)新增场景和接线，strict静态／测试库通过；普通EXE、完整ZIP及版本544不变。7.94秒循环回归不是普通C前台全流程、长期资源或全库通过，也不关闭Canvas旧失败。A/B人工通过、普通C及物理环境／完整业务长稳／全面验收的边界保持，不因新增测试上调为100%。

## 当前 v544：隔离空闲资源取得完整观测

私有 desktop（从未切换）内的普通 Otter＋Loom 已完成600秒有限空闲观测，两个主进程各30秒ETW没有UIA查询，观测与旧外部查询风暴条件区分开；详见[数据、原采样器失败及边界](WINDOWS_NATIVE_BUILD.md#v544-私有-desktop-双产品空闲观测)。

这不是普通可见资源验收或业务长稳；GPU／物理磁盘未验证、Job仍为兜底回收。没有产品源码／版本／构建／ZIP变化，不新增Rust通过数，不据此上调为100%。A/B人工通过、普通C原失败及Canvas／真实物理环境／人工IME／全面验收边界保持。

## 当前 v544：完整内部包已纳入 Live 修复

已验证的静止源尺寸变化停帧修复现已进入完整 v544 内部 ZIP，候选来源、许可证材料、归档核验／拒绝合同及解包 EXE 工作区门通过。全新 profile 的普通四象限窗口已观察，不再仅有裸 EXE；详见[最新整包与验证边界](WINDOWS_NATIVE_BUILD.md#v544-最新完整内部包包含-live-停帧修复)。

A/B 人工通过，C 原人工失败保持，普通 C 完整前台流程尚未关闭。Canvas 原 250ms 失败、干净资源／联合长稳、物理多屏跨机混合 DPI、人工 IME 和全面验收仍开放；不以打包成功提高为100%。无产品源码或版本新增变化，不重开兼容维护、Talk构想或冻结副线。以下旧版本记录保留当时边界。

## 当前 v543：停止重复路径隔离实验，准备合并验收

相同SHA256普通EXE的新路径/新profile对照仍收到共享Computer Use helper的持续UIA查询；实际122.328秒/61点即按预案停止，非600秒或干净资源基线。追踪26,515条事件、有效22.136秒，触顶stop4201/exit5保留。自有Job兜底及独立残留核验完成，共享helper未动；不再换路径刷同条件结果，详见[记录与边界](WINDOWS_NATIVE_BUILD.md#v543-全新-exe-路径未能隔离后台查询)。

公共Art capsule/完整跨节点键盘/普通Live合并手测入口已准备但未运行，不能记通过。无产品源码/版本/编译/包变化，完整内部包仍v543；Canvas旧失败、无外部持续查询的资源基线、物理环境/人工IME、完整业务长稳与最终用户验收保持开放，工程粗估仍约94%，总迁移active。

## 当前 v543：资源基线的持续外部查询边界

一次全新profile、普通完整包的600秒Otter＋Loom观测已完成，没有任何Computer Use API调用，后台helper仍对新Loom持续UIA查询。自有PID ETW有27,041条有效请求/响应事件，对端均为codex-computer-use.exe；有界trace提前触顶，不是完整30秒。不能把该环境当无查询风暴的资源基线，也不能将高CPU笼统判为产品空转。数据、线程归属、回收和测量限制见[构建记录](WINDOWS_NATIVE_BUILD.md#v543-联合空闲观测未调用工具仍有后台-uia-查询)。不结束共享helper、不关可访问性、不重复相同条件。

剩余3名原库存已分类为一个有父门原证据的内部child入口、一项未运行的冻结退役ANGLE历史门及Canvas原250ms失败；没有新增通过数，不是只剩一个迁移功能或整库全绿。普通capsule此次因捕获身份不匹配未打开，Live/完整键盘、物理环境/人工IME、真实合计资源长稳及用户验收继续开放。没有产品源码/版本/包变化，仍复用v543完整内部包，总迁移active。

## 当前 v543：系统 UIA 焦点两项旧失败已收口

原失败时 foreground=HWND(0x0)，缺少系统焦点断言所需的真实前台。两项测试现在复用既有自有前台授权/guard 和 SERIAL，原所有断言不变；分别经 Computer Use 实际点击授权后，以独立测试进程通过 EDIT 的焦点/Value/Text/唯一父级及 COMBOBOX 的焦点/选择/展开/唯一提交。测试树自然归零且无残留，不冒充普通产品或人工输入验收。详见[根因与当前证据](WINDOWS_NATIVE_BUILD.md#v543-两项系统-uia-焦点夹具已因果收口)。

只有两份 cfg(test) 源码改动；strict 静态/测试库通过，input1 模块路径编译错误保留。原库存 5→3，未新增功能或测试，最新完整内部 ZIP/普通 EXE 仍为 hash 未变的 v543，不重复封包。Live、真正 capsule/完整键盘、无查询风暴资源基线、Canvas 旧失败、物理环境、双产品完整业务长稳和用户验收仍开放，整体仍约 94% 粗估，总迁移 active。

## 当前 v543：最新完整内部包已补齐

正式 candidate 构建、源码/工具/日志/EXE 来源核验、完整 ZIP/许可证/材料清单/provenance、独立归档核验、归档拒绝合同与候选/许可证合同均通过。解包 EXE 通过既有 6 项工作区实跑，并在全新 profile 的普通窗口显示自有四象限图。详见[当前完整包与验证边界](WINDOWS_NATIVE_BUILD.md#v543-最新完整内部整包与解包运行)。

当前完整内部 ZIP 从 v540 更新到 v543，没有再次分配版本或增加产品源码。它仍 dirty、未签名且不可正式发布；不把有限普通启动或 Job 回收当完整业务/自然退出/资源达标。真正 capsule/完整跨节点键盘、普通 Live、系统 UIA/Canvas 历史失败、无查询风暴资源基线、物理环境、双产品完整业务资源长稳和用户全面验收仍开放。库存仍 5 名，整体仍约 94% 的工程粗估而非统计，总迁移 active；兼容维护/Talk 构想及冻结副线不重开。

## 当前 v543：普通公共参数编辑取得业务实证

全新 profile、普通 Otter/Loom 及自有回环 Art 中，完整参数面板 Query 经标准 EDIT 的 set_value 改为 capsule-中文-543，原“执行一次”提交后，上游恰好收到一次相同值，正式画布显示 320×240 绿图，第二候选勾选。关闭的是此前未验的普通完整参数文本修改→提交→正式结果，不是单字段 capsule、人工 IME 或完整跨节点键盘。详见[实际路径与工具边界](WINDOWS_NATIVE_BUILD.md#v543-普通公共参数文本提交与正式结果)。

无产品源码、版本、编译或新包；两树 Job 兜底归零且独立核无残留，不称自然退出。资源采样含工具查询，不算完整业务长稳或达标。真正 capsule、普通 Live、完整跨节点键盘、UIA/Canvas 旧失败、无查询风暴资源基线、物理环境、最终整包及用户验收仍开放；普通 EXE v543、官方 ZIP v540，库存仍 5 名，粗估约 94% 并非精确测量，总迁移 active。

## 当前 v543：Canvas 旧失败的真实 child 诊断

本批只有 cfg(test) 固定阶段/当前线程 CPU 诊断与显式成功报告入口，没有普通产品行为、版本或包变化。一次原脚本真实 child 的快照冻结、等待后像素独立性及回收通过；首次绘制 59.548ms，软件 target 创建占 55.258ms，第二次绘制 0.384ms。相关 9 项 Rust 与 strict 静态通过，详见[原始失败、测量限制及当前边界](WINDOWS_NATIVE_BUILD.md#v543-canvas-真实-child-阶段与线程-cpu-诊断)。

原 519.705ms 失败没有被复现或因果关闭；当前线程的离散 CPU 记账不是整棵进程资源/调度证据，新 worker 也不等于操作系统冷缓存。保留 250ms/heap/Job/父期限，不重跑刷绿或将暖路径成绩覆盖历史失败。原库存仍 5 名，并非只剩 5 个迁移任务。普通 Live、公共 Art capsule 编辑/完整键盘、无查询风暴资源基线、UIA/Canvas 旧失败、物理环境、完整业务长稳、最新完整包及用户验收仍开放；普通 EXE v543、官方 ZIP v540，总迁移 active。

## 当前 v543：Surface 输入库存补验25项

固定最新测试库经Computer Use实际点击自有启动器一次授权，24项真实自有HWND的前台/焦点、hover、Enter及指针FIFO/取消/同步重入回归通过；另1项消息身份检查在未切换的私有desktop通过。测试树自然归零、无Job兜底，独立核无残留；不把测试夹具成功代普通产品键盘或自然退出。原库存30降为5，含两个系统UIA旧失败、内部入口、冻结ANGLE历史项和Canvas原失败，非5项缺功能，不为清零重开冻结副线。详见[本批证据与边界](WINDOWS_NATIVE_BUILD.md#v543-surface-输入回归集中补验)。

普通Live新profile/自有动态源尝试在Ctrl+2后遇到工具无法捕获宿主且未返回独立选区目标，未选源、无后续输入或替代API，不判产品缺陷；三树Job兜底归零，不称自然退出、Live通过或资源达标。无产品源码/版本/重编变化，普通EXE v543、官方ZIP v540。普通Live/公共参数capsule/完整跨节点键盘、无查询风暴资源基线、UIA/Canvas旧失败、物理环境、完整业务长稳及最终包/用户验收仍开放，总迁移active。

## v543 前台 CPU 对照与 UIA 请求对端

标准 Win32 根窗/40 个系统按钮已独立复现与普通产品近似的前台高 CPU：105.016 秒平均 57.194% 单核、每秒 5,265.712 次 WM_GETOBJECT，WM_PAINT 没有增长；稳定后台 53 秒两者停止增长。自有 PID/事件过滤 ETW 同时在标准对照和固定普通 v543 中确认，请求唯一对端为 codex-computer-use.exe。这类现象不需要产品 Canvas，不能继续当成未经定位的 Canvas 自续绘制缺陷。

当前普通产品 ETW 在原 16 MiB 上限结束，只保留 16.119 秒实际事件；stop=4201/exit=5、第一次路径错误和第二次无前台空 trace 均保留，不称完整追踪通过。独立核自有 EXE/trace 会话无残留，普通产品仍是 Job 兜底退出。没有产品源码、版本、依赖、预算或可访问性变更，不重编或停止共享工具；详见[对照、对端及未验边界](WINDOWS_NATIVE_BUILD.md#v543-前台-cpu-对照与-uia-请求对端)。

仅关闭“当前复现没有请求对端证据”的调查缺口，不认定所有历史 CPU 原因已清或完整资源达标。下一步集中推进 Live/公共参数/完整键盘；无查询风暴的资源基线、UIA/Canvas 旧失败、物理环境、完整业务长稳及最终包/用户验收仍开放。普通 EXE v543、官方内部 ZIP v540、库存 30 名和冻结副线不变，总迁移 active。

## 当前v543：Shift+1 与独立 Art 获得普通产品实证

修复当前IME把Shift+1交为全角标点、宿主原先不能识别的真实入口。新普通EXE和全新profile下，快捷键打开目录、刷新并选择能力、保留源图、创建独立子Art及可见连线、默认参数一次执行和正式红图均取得联合实证；使用自有合成HTTP服务，不冒充生产/跨机或人工IME全面验收。13项相关Rust及6项新EXE工作区门通过，strict静态通过；原两轮夹具失败及线程键态独立证据保留，详见[实现和验证边界](WINDOWS_NATIVE_BUILD.md#v543-宿主-ime-快捷键与普通独立-art)。没有新旧兼容层或输入法设置变更。

跨节点端口遍历已有普通可见证据，但新钉图/owned端口不在工具独立目标清单，后续root输入丢焦，不能称完整连线提交或公共参数编辑通过。515.125秒/256点合计Private峰值387.871MiB、单核CPU平均40.683%，负载变化，非资源达标或CPU改善结论。关闭调用/捕获失败后本轮Job兜底归零，非自然退出。最新普通EXE为v543、官方ZIP仍v540；普通Live/公共capsule编辑/完整跨节点键盘、CPU因果、UIA/Canvas旧失败、物理环境、完整业务长稳及最终包/用户验收仍开放，库存仍30名，总迁移active。

## 当前v542：普通 Art 默认执行与多候选取得联合实证

全新profile的普通Otter/Loom真实菜单目录选择、Tab参数可读、默认参数手动执行及正式红图通过；点击第二候选后仅一次显式再执行，正式绿图、第二卡勾选和result_index=1均确认。候选选择按已有合同重新执行，绝不把缩略图当正式输出。使用自有回环合成HTTP服务及合成图片，不冒充生产服务或跨机；没有新产品源码、版本、构建或ZIP。

参数文本输入受owned编辑框目标绑定限制，root-targeted输入触发图片粘贴提示，Query未变化；不据此修产品或重复同条件尝试。独立Add Art/Live、公共参数capsule和完整跨节点键盘仍未验。双产品900.5秒/447点合计Private峰值408.180MiB、单核CPU平均50.090%，不是完整业务长稳或资源达标，高CPU仍开放。正常退出停在未保存确认，两棵Job期限兜底归零，不称自然退出；详见[实际路径与边界](WINDOWS_NATIVE_BUILD.md#v542-普通-art-目录执行与候选结果联合实证)。库存仍30名，旧失败/物理环境/最终包和用户验收保持，总迁移active。

## 当前v542：选图取消语义已在普通产品复验

文件选择现在以typed结果区分正常取消和准入拒绝，取消不排请求、不改配置或提交凭证，真实拒绝/系统错误仍保留。新增4项回归，57项库测试和6项新普通EXE工作区门通过，strict静态及一次集中库/产品构建完成；不把相关测试数当整体完成率。

全新profile、自有合成图片的普通参数窗→系统文件框→Esc取消已真实观察，刷新确认仍未选择图片且不再误报配置失效；没有选择/上传用户文件。工具取消索引缓存错误、关闭后无法捕获窗口的错误均保留；自有Job兜底归零但非自然退出，不拿清理成功代退出验收。最新普通EXE为v542内部runtime候选，官方ZIP仍v540。详见[本批实现、运行与未验边界](WINDOWS_NATIVE_BUILD.md#v542-图片文件取消不再误报配置错误)。完整可见Art/Live/多候选/跨节点键盘、CPU、系统UIA/Canvas旧失败、物理环境、双产品资源长稳和最终用户验收仍开放；库存仍30名，总迁移active，兼容维护/Talk构想不恢复。

## 当前v541：普通图片键盘/capsule有实证，取消误报待修

普通固定EXE、新profile、自生成图片已观察F6进入端口、Tab/反向Tab遍历、Ctrl+Enter选源及Esc取消，F2显示图片单字段控件；尚不是完整跨节点连线、Art公共参数/多候选/Live或人工IME验收。右键长菜单滚动后确实包含Loom/Art，纠正上轮“只有捕获菜单”的推断。真实文件框在预建隔离Desktop后可打开和取消、图片仍未选择，但取消被误报为配置失效，本轮发现的产品缺陷保留待修。

CUA前安静后台单核约0.159%，激活但首次显式AX前已约55.002%，显式AX后约55.861%；最小化后台约0.309%，真实关闭工具窗后仍约24.120%。条件变化/工具内部UIA未排除，不唯一归因。716份实际热点栈与同步前台证据均有效，两线程增10.609375 CPU秒，仍无匹配私有函数符号，不能据邻近export改产品。一个普通运行到期限Job兜底，另一模态运行正常关闭且Job归零；不是双产品资源验收。详见[证据与剩余边界](WINDOWS_NATIVE_BUILD.md#v541-普通端口键盘图片模态与热点实采)。无版本/产品源码/编译变化，库存30名及旧失败保留，总迁移active。

## 当前v541：普通参数可读，CPU问题仍开放

用户再次恢复仅全新profile、自有测试窗口的前台验收。固定普通v541 EXE经实际清单唯一选窗、画布聚焦和一次Tab，参数各行已正常显示，端口显示模态可打开/Esc取消，宿主Tab可关闭参数窗；即时AX仍会滞后。控件索引缓存拒绝后重新观察，按新截图点击同进程参数按钮成功，没有改输入法/显示设置或操作既有应用。只关闭本轮普通图片参数初始白行的复验缺口，不代表Art候选/内嵌capsule/Live完整键盘或人工IME通过。

参数展开的48.093秒安静片段消耗26.3125 CPU秒，但前台只读采样仅覆盖其中一部分，不能声称全程前台；关闭参数后的60.047秒对照消耗32.46875 CPU秒，60/60前台样本均属本轮产品。因关闭后仍高CPU，不能归因参数窗单一循环；随后有界线程栈采样时热点未复现，不凭UIAutomationCore邻近export断言根因。普通产品在901.812秒Job期限兜底归零，不称自然退出；Shift+1未建立Art响应，末次输入报前台PID不可读，未重试或绕过。

另有六类声明式表单的真实回环TLS→自有HWND→事件/权威快照→撤权父门通过。独立标准EDIT/COMBOBOX在未切换desktop也出现Win32焦点已取得但系统UIA属性false；两项原断言及诊断失败仍保留，不能据此判产品有缺陷或改判通过。库存31减去本次TLS父门，仍有30名，不是30项缺功能。证据与边界见[构建记录](WINDOWS_NATIVE_BUILD.md#v541-普通参数复验与系统焦点对照)，产品源码/版本/产物未变，不重复编译。

## 当前v541：参数绘制根因修复，普通可见复验未完成

聚焦宿主后打开参数窗的父paint循环已确定性复现：导航处理paint后无变化重排，再次失效父窗口，其他child得不到初始绘制。现在仅滚动位置变化时重排，真实聚焦Application消息泵回归通过；标准COMBOBOX列表及收起区的深色文字也由真实DIB像素回读验证，保系统键盘/索引和原参数事务。83项库测试及新普通EXE工作区6项通过，严格静态与普通Release已完成；不是整库或普通Art完整业务验收。

新产品首次前台清单被用户物理Escape停止，未选窗或输入，故普通白行视觉和修复后CPU仍未验；runner中断后只读核产品无残留，不冒称自然退出或严格Job终态。最新普通EXE为v541内部runtime候选，最新官方内部ZIP仍v540且不含本修复。详见[证据及剩余边界](WINDOWS_NATIVE_BUILD.md#v541-参数初始绘制与下拉框修复)。Canvas旧250ms失败、Loom前台CPU、完整可见业务/IME、双产品资源长稳与物理环境仍开放；兼容维护/Talk构想排除，总迁移active。

## 当前v540：普通 Tab/端口模态取得入口实证

最新实际解包EXE在全新profile、自有画布确认焦点后，一次Tab打开参数窗，端口显示模态能打开/Esc取消，宿主Tab关闭参数窗；即时AX滞后不能等同无响应。但参数窗多行白底文字不可读，F4触发输入法方案浮层且Ctrl+E未建立显隐，视觉/完整键盘验收仍未通过。600秒Job兜底归零，不是自然退出；资源高低片段并存，不把进程I/O当磁盘写入或认定参数窗持续空转。产品源码/版本/产物不变，详见[当前普通入口及证据边界](WINDOWS_NATIVE_BUILD.md#v540-普通参数入口与当前前台边界)。完整Art/Live/候选/capsule、人工IME/模态、完整双产品业务资源长稳、物理环境与历史失败仍开放，总迁移active。

## 当前v540：窗口初始标题已修复，最新内部整包已验证

查明4类原生入口跳过WM_NCCREATE默认处理，造成初始标题丢失；保原独占绑定/拒绝，恢复系统默认初始化。最小Win32负/正对照及真实隐藏HWND标题回归通过，82项相关Rust、严格静态与实际解包EXE的6项工作区门通过。官方内部ZIP已包含多候选选择及本次修复，不再只提供散落的最新开发EXE；仍是dirty未签名内部候选，不是正式发布或完整验收。

普通直接图片入口已观察自有画布/工具/菜单并取得一次自然退出。新解包EXE已被Computer Use按正确标题返回，AX与可见窗口均证实修复；这不是所有历史工具边界的唯一根因。裸Tab未建立参数响应、按钮缓存拒绝及后续遮挡仍如实保留，本次新包由Job兜底退出，不算完整参数/capsule/键盘通过。详见[证据及剩余边界](WINDOWS_NATIVE_BUILD.md#v540-原生窗口初始标题与最新内部整包)。普通Art/Live完整业务、人工IME/模态、物理环境、合计资源长稳和用户验收仍开放，总迁移active。

## 当前v540：Tile实跑补证，普通交互仍未验

5项此前未启动的真实TLS/WSS/Tile Runtime回归全部通过，Default desktop自有离屏窗口实际取得D3D12 S_OK/confirmed，覆盖ACK输入准入、冻结/黑场/恢复、慢Art与Live隔离及停止/撤权回收；无输入、前台不变、Job归零。库存85降至80，不是功能数量或普通可见验收。无源码/版本/编译变化。

普通Otter恢复钉图仍不可选；空工作区虽返回窗口但AX/截图不足以建立自有工作面，未发输入或绕过。普通Loom安静前台65.953秒实测整树1.796875 CPU秒，未复现旧高CPU；8次UIA观察实际在后台，同条件前台因果对照未完成，不关闭根因。详见[本批证据与边界](WINDOWS_NATIVE_BUILD.md#v540-tile真实运行回归与前台观察边界)。普通可见Art/Live/capsule/完整键盘、人工IME/模态、物理环境、完整业务资源长稳及最终包仍开放；兼容维护/Talk构想排除，总迁移active。

## 当前v540：Canvas诊断细化及库存补证

本批184个不同普通Rust完整名通过，其中178项接受关闭原库存，剩余263降至85；包含4项真实回环TLS device session，不含普通可见Tile完整呈现。私有desktop下UIA原失败由明确的Default desktop安全准入条件解释，仅该项自有离屏NOACTIVATE对照通过，前台未变、无输入，旧失败保留。4份Rust仅改`cfg(test)`诊断/断言，普通产品行为与EXE不变，不重分版本。

Canvas原519705µs超时仍开放；细分诊断后原child单次通过不是因果修复，成功暖模型及独立背板探针也不能代替原冷启动阶段证据。原250ms预算不变，该项仍留库存，85不是功能缺口数或迁移百分比。详见[本批构建与验证边界](WINDOWS_NATIVE_BUILD.md#v540-canvas-分阶段诊断与剩余回归补证)。普通可见业务/键盘/IME、完整双产品资源长稳、CPU根因和最终包仍未完成；兼容维护/Talk构想继续排除，总迁移active。

## 当前v540：联合观察和回归库存继续收口

最新普通Otter/Loom壳/daemon已用新profile联合运行15分钟，383点、合计Private最高391.602MiB；仅普通图片及空闲壳，含UI和调试采样，不是Art/Live业务、资源达标或长稳，也不覆盖旧529.840MiB峰值。Loom窗口可选，Otter仍不在工具可操作目标中；不以此推定普通可见候选/键盘/IME通过。57个此前库存普通完整名及1项显式最新产品Canvas服务worker门通过；原Canvas test child仍有1项250ms真实失败，13项后续锁污染隔离后通过，原失败不抹除。当前剩263个库存名，非功能数量。新增线程栈包含UIAutomationCore/消息/RPC路径，但无匹配私有符号且观察者效应未排除，CPU具体根因不关闭。详见[当前验证边界](WINDOWS_NATIVE_BUILD.md#v540-最新普通产品联合与剩余回归核查)。没有产品源码、依赖、版本或构建变化，总迁移仍active。

## 当前v540：Art 多候选选择已接入原生UI

完整参数窗已消费真实候选，显示有界缩略图/标题/序号/选中标记，选择按原版result_index并显式重跑；预览不冒充正式输出。只对当前配置/当前集合/已声明可编辑数值参数授权，dirty/IME及越界/文本/linked拒绝。13项新回归包含标准BUTTON Space消息管线，70项相关Rust及6项最新普通EXE工作区门通过；不是普通可见业务、物理键盘或整库验收。旧v540 ZIP不含本功能，最新普通EXE见[本批构建记录](WINDOWS_NATIVE_BUILD.md#v540-art-多候选预览与选择)。CPU已收敛到前台native shell/browser-side，具体根因、完整双产品长稳和最终包仍开放。取消全部兼容维护/Talk构想，总迁移active。


## 当前v540：追加回归与普通壳订阅边界

在此前18项模型之外，本批补充442个此前未启动的Otter完整名：224项回环TLS/worker、216项私有desktop自有窗口/模型、2项当前desktop离屏NOACTIVATE GPU对照；原私有desktop两项GPU呈现失败保留，不称单次全库通过。当前库存780减18再减442，仍有320个普通名待审核/执行，其中含不能独立空跑计分的内部worker入口；这不是320项产品功能缺实现或迁移百分比。

Loom原生订阅Pong独立取得原250ms写期限，11项定向回归与普通Release壳通过；修复版125秒订阅采样未见零点，不能关闭历史间歇或宣称长稳。前台壳/WebView2 CPU热点仍开放，旧普通双产品Private最高529.840MiB不被无壳基线或本批单壳短样本替代。普通Otter可见Art/Live/capsule/完整键盘及人工IME、多屏跨机DPI、完整合计资源与最终用户验收仍未完成。详见[本批证据](WINDOWS_NATIVE_BUILD.md#v540-订阅写期限与追加回归)。无新版本分配或Otter重编，兼容维护与Talk构想排除，总迁移active。

## 当前v540：表单热点已修，普通daemon已更新

原表单失败已有明确热点对照和产品修复，保原250ms；加强后的28case在新测试child及真实解包产品worker均通过，另12项直接依赖与6项工作区门通过。最新普通Loom daemon已独立构建并实际启动，不再用测试库或旧524替代；当前双后台60秒基线不是完整业务/可见产品/长稳验收。详见[本批证据与未验边界](WINDOWS_NATIVE_BUILD.md#v540-表单换行热点修复与最新普通daemon)。兼容维护/Talk构想仍排除，总迁移active。

## 当前v539：追加回归与内部ZIP边界

本批806个完整名通过，1个JS表单真实child的原250ms预算失败仍开放；68个SERIAL污染项隔离后通过，不覆盖原失败。官方内部ZIP及真正解包后的工作区限定运行已通过，不能代替普通可见完整交互、最新普通双产品、全量或长稳验收。没有产品源码/依赖/版本变更，不恢复兼容维护。详见[本批构建与失败证据](WINDOWS_NATIVE_BUILD.md#v539-追加回归与官方内部zip)。

## 当前 v539：取消回收及普通EXE

本批关闭已证的初始化取消进程持有缺口，原运行中设备撤权3秒无poll回收门通过；不能以预热PID误判Command已运行，也不为单设备撤权杀整个共享插件。当前普通Release/workspace已构建，65项相关Rust通过，不代替普通可见交互、全量、跨机和合计资源长稳。旧失败及无关静态阻塞保留，兼容维护不恢复。详见[当前构建台账](WINDOWS_NATIVE_BUILD.md#v539-撤权取消与进程所有权)。

## 当前v538：最新架构与默认显隐

Local签名默认显隐及在线/离线消费已定向验证；旧兼容要求全部取消，不再列待办。不是整库或普通可见产品验收。详见[当前构建台账](WINDOWS_NATIVE_BUILD.md#v538-当前签名包与端口默认显隐)及[当前目标](WINDOWS_NATIVE_REVISED_PLAN.md#当前目标只对接最新架构2026-11-04)。

## 当前 v537：普通image输入对等范围

普通image输入已有上传、明确清空和启用/禁用；上传不改disabled，清空不删连接。正式输入上游优先，手动图次之；禁用/停止中继/本地编辑使用自身图。保存恢复不复活已清图片，保未知字段；无图片省略image_path/image_filename，与旧空串同属未设置，不凭空消耗两个参数槽。真实WebP/BMP、像素、取消、迟到拒绝、容量原子拒绝和Receipt由定向回归验证，非仅扩展名过滤器。

本批同源定向 **250项Rust passed / 0 failed / 0 ignored**，含16项新增；不是整库通过。官方fmt、版本、全仓strict行数、strict all-targets Clippy、固定SHA库、普通Release与workspace门通过。只分配一次v0.2.33.537；先集中实现与测试，再统一Clippy/产品构建。

本批未启动普通前台交互，不以隔离HWND/消息回归替代普通可见上传对话框、键盘或人工IME。前台恢复仍仅限新profile、自有测试窗；保旧工具边界/失败及dirty/untracked，三冻结副线/v464/用户release97不动。Talk构想和旧Web/Tauri混用排除，不启动Loom原生迁移，不提交推送发布部署，总迁移active。

[验证与剩余边界](WINDOWS_NATIVE_BUILD.md#v537-普通贴图图片输入)。

## 当前 v536：可编辑capsule的实际范围

公共Art参数输入展开后可挂真正单字段原生child，复用EDIT、COMBOBOX、bool、步进、slider、颜色和文件控件及原参数事务；不是弹完整面板冒充内嵌。工作区仅一个活动editor，宽300逻辑像素，放图片左侧、空间不足翻右侧，两侧不足不挂，不覆盖图像。端口`Enter`/`Space`仍切换展开，`F2`从已聚焦参数端口进入控件，child内`F6`返回端口；文本Ctrl+Enter仍提交文本，不能误连线。linked有效值只读、secret不呈现；只允许已确认自身同步提交重建新Catalog后继显示，不继承旧草稿。

`chrome::capsule`拥有工作区单槽Rc候选、目录/宿主/parent身份及后继显示；`parameter_panel::window`拥有完整owned窗和单字段child的共同构造与资源寿命，`controls::capsule`仅筛选字段动作，复用原Snapshot/Spec和唯一参数提交。dirty/IME/pending纳入原chrome idle及保存/恢复/结束会话门。外部配置、隐藏、锁、捕获、端口显隐及失焦退休旧资格；同步focus/place/close/request返回只可取消同一Rc，旧栈不能撤销或覆盖后继。模态返回再次核panel owner及配置，旧panel不继续消费后缀；child继承端口条affinity，不对child调用顶层DisplayAffinity。没有新依赖、网络合同、权限、线程、timer、每端口HWND或像素/队列预算。

本批选定**98项Rust passed**，14项新增，选定终态0 failed/0 ignored；1项既有默认ignored未选、不计通过，非整库。覆盖真实child/单字段、多种公共widget、原子提交/linked只读、IME边沿/DPI草稿/父先销毁、region及图片外几何、单槽/输出/secret、dirty空闲门、自身提交后继、F2/F6/两段Esc、外部状态拒旧草稿、同步focus/place/close/request安装后继和退休owner拒发布；原chrome/参数/系统会话/Live输入取消依赖同批验证。官方fmt/版本/全仓strict行数、strict all-targets Clippy、固定SHA库、普通Release与workspace目标通过。23项源码/版本delta均UTF-8无BOM，6个新文件，Rust最大397有效行，无新增例外。input1首次Clippy因子模块可见性、测试未用导入和Stamp断言编译失败，原回执保留；input2只修这些编译问题，不重分版本。input2的chrome为47通过/1失败：后继回归夹具在同尺寸SetWindowPos后误等CHANGED；input3只在该cfg(test)分支先改变child高度以实际触发几何消息，原失败保留。guard回归验证返回后退休资格，不冒充真实颜色/文件模态人工验收。

前台限定恢复有效，仅新profile、自有测试窗。按用户本轮快速收口要求，只完成已启动批次的定向回归和产品构建，不重开与旧批次相同工具边界的普通前台尝试。本批没有启动新可见fixture或发送UI输入；普通可见capsule/颜色文件模态/完整端点键盘及人工IME未验，不能由隔离HWND回归推定通过。旧前台工具边界和清理失败回执继续保留。当前capsule仅公共Art参数；普通sticker的image上传入口及Local签名默认显隐元数据仍未完成，输出不提供参数editor。普通完整Art/Live/端点键盘及人工IME、普通双产品、物理多屏/跨机/DPI、剩余全量/合计资源长稳、最终完整隔离包及用户全面验收仍开放。Talk构想和旧Web/Tauri混用不列阻塞；三冻结副线/v464固定包/用户release97保持，不启动Loom原生迁移，不提交推送发布部署，总迁移active。

## 当前 v535：端点键盘范围

Windows原生分支现在支持从当前文档/Live宿主按`F6`进入本节点可见端口，`Shift+F6`从末端口进入；端口条内`Tab`/`Shift+Tab`及方向键遍历当前工作区可见且可交互的端口，`Home`/`End`到首尾。`Enter`/`Space`切换名称展开，`Ctrl+Enter`从输出选择来源，或从已有连线输入选择原来源，再在另一节点输入提交。`Esc`先取消选源，再次退出；`F6`直接退出并回到仍属原owner的宿主。长按/松键不泄漏到文档删除或Live源输入，Ctrl+Shift+4也可从端口条操作。

本批选定**75项Rust passed**，12项新增，选定终态0 failed/1既有默认ignored，非整库。覆盖实际自有端点HWND焦点、遍历/首尾/反向、展开、长按和松键、唯一连线提交、重连取消、提交前锁定、隐藏端口、过期owner、捕获、外部失焦、鼠标接管及同步SetFocus重入；原chrome/参数面板/结束会话/Live输入取消直接依赖同批验证。官方fmt、版本、全仓strict行数、strict all-targets Clippy、固定SHA库、普通Release及workspace目标通过。9项源码/版本delta均UTF-8无BOM，3个新文件，Rust最大337有效行，无新增例外。首次runner因误选原默认ignored鼠标捕获项而拒绝，原回执保留；只读重解析原日志并核未改源码声明，未重跑成功测试、不把ignored计通过。

前台限定恢复有效，仅新profile、自有测试窗。本批只运行未切换独立desktop/自有Job回归；没有普通前台输入。v534工具未返回普通产品窗口的边界仍开放，不猜HWND、不换其他自动化API、不重试同条件失败，不将隐藏HWND通过称普通可见键盘通过。可编辑capsule内嵌控制、普通完整Art/Live/键盘及人工IME、普通双产品、物理多屏/跨机/DPI、剩余全量/合计资源长稳、最终完整隔离包及用户全面验收仍开放。Local签名默认显隐字段当前合同未提供；Talk构想和旧Web/Tauri混用不列阻塞。三冻结副线/v464固定包/用户release97保持，不做Loom原生迁移，不提交推送发布部署，总迁移active。

## 当前 v534：端口外观对等与边界

端口显示现在支持Backend单端口`defaultVisible`优先、旧能力级`defaultVisibility[name]`回退、最终缺省显示；用户`portVisibility`按名称最高优先，同名输入/输出共用覆盖。Document节点菜单、参数面板和Live系统菜单共用原生“端口显示…”入口，列表可选使用默认值/显示/隐藏，取消丢弃草稿。参数面板先原子提交有效文本，再重绑定本次后继操作。隐藏端口从绘制、稀疏region和命中移除，可见位置紧排但保目录索引；原连线/执行权限/拓扑不变。

本批选定**84项Rust passed**，21项新增，选定终态0 failed/0 ignored，非整库。覆盖中文/非法/重复/null/容量、缺省优先级、外观独立、真实LISTBOX/COMBOBOX通知、取消/关闭/DPI/affinity/字体回收、单槽/过期owner与Catalog、实际参数按钮事务、隐藏端点和原边保留、保存重开/未知字段/reset及Live真实worker保存/Receipt失效/冻结后再保存重开。官方fmt、版本、全仓strict行数、strict all-targets Clippy、固定SHA库、普通Release与workspace目标通过。39项源码输入delta均UTF-8无BOM，10个新增文件，受影响Rust最大497有效行，无新增例外。

前台权限仅本批新profile、自有测试窗。普通Release使用预置隐藏output覆盖的独立profile，本次没有取得普通产品可见外观证明；工具输入调用为0，不操作用户旧窗口、不注入全局输入、不改显示设置。该观察不冒充普通产品对话框交互/完整键盘或人工IME；本次自有Job终止后的原5秒父进程等待失败，未通过严格清理/自然退出门；随后只读核本批产品残留为0，不重放终止或UI，也不以稍后归零覆盖原失败。

Local v2签名端口当前没有默认显隐元数据，仍缺省显示，不从未签名提示伪造包默认，本批不改Loom/wire。可编辑capsule、完整端点键盘、普通完整Art/Live/键盘与人工IME、普通双产品、物理多屏/跨机/DPI、剩余全量/合计资源长稳、最终完整隔离包及用户全面验收仍开放。Talk外部会话控制构想及旧Web/Tauri混用按用户决定排除。三冻结副线/v464固定包/用户release97保持，不启动Loom原生迁移，不提交推送发布部署，整体迁移active。

## 当前 v533：Local Art 端口对等与验收边界

Local Art 现在从 Loom 已验证的不可变安装包取得输入/输出的真实名字和类型；Otter 接入目录、图像输入、来源配置凭证、自定义主图片输出继续 Add Art、正式结果及保存重建/有限离线。显式零输入/零输出保持为空，不伪造 input_image/output_image；只有无新证明的旧包保留默认端口。首次自定义主输入迁移在候选 Workspace 内提交，冲突保原图；零输入只撤回本次工厂创建且仍完全一致的暂定边，保存恢复不继承删除旧边的权限。端口保存记录仅是包绑定提示，不是执行/离线授权。

本批最终选定 **83项Rust passed**，其中17项新回归；当前选定终态failed=0，不是整库。签名域/篡改/预算、旧能力协商、拒绝不重放、图拓扑、中文自定义输入、真实2×2红像素、Live配置换代拒迟到、零输入不交付像素、自定义rendered输出继续建子Art及Save/窗口重建/有限离线均已验。真实Loom LAN HTTPS跨进程联合在线和停服后离线独立核signedPorts、完整资源、正式像素和正式结果；不完整缓存拒绝，父子不混加。Otter strict all-targets Clippy、两仓fmt、官方版本/strict行数、Otter lib/普通Release/工作区与Tile驱动构建通过；Loom签名安全/daemon测试库通过。66项源码delta均UTF-8无BOM，Rust最大500有效行，无新增行数例外。两个原跨仓grant门及两个SDK示例默认ignored未运行；Loom整仓strict Clippy旧债未重跑，不称两仓全绿。

用户已恢复仅新profile、自有测试窗口的前台验收。本次普通固定SHA Tile EXE三类图片/Live/Art混合可见门通过；Computer Use只激活工具唯一返回且路径/标题匹配的本批标准启动器，刷新成功后用单会话nonce同步，driver再核真实前台并只向已核自有Tile转交。独立桌面采样核三路换帧、冻结/黑场/恢复、正常关闭及来源附件/值/Live保留与连接回收，不以后台paint或源像素代呈现。没有操作用户旧应用、注入全局输入或修改显示设置；不等完整键盘/人工IME或普通源EXE全部通过。

逐端口显隐/默认值与持久化、可编辑capsule/完整端点键盘仍未完成；普通完整Art/Live/键盘与人工IME、普通双产品、物理多屏/跨机DPI、剩余全量/合计资源长稳、最终完整隔离包和用户全面验收仍开放。Talk外部会话控制构想和旧Web/Tauri混用按用户决定排除。三冻结副线/v464固定包、用户release97保持；不启动Loom原生迁移，不提交推送发布部署，整体迁移active。

## 当前 v532：Live Add Art 实现与验收边界

Live窗口的精确Shift+Digit1和系统菜单“添加 Art”复用原搜索目录窗；原Live保持存活，创建独立子Art和真实Live→Art边，不冻结或转换源。Backend保默认参数和Upstream自动门、Live快照及原输入worker/TLS执行；Local仅沿显式HTTPS、lease/issuer、describe→prepare→签名child原链。打开选择前沿原通道停止源输入，取消/来源或目录/分组/工作区/会话变化及容量失败保源，不留下半条边。

最终input4同源定向 **62项Rust passed / 0 failed / 0 ignored**，含9项新测试，不是整库。真实owned目录窗/搜索/IME合成/affinity、独立子节点/默认值、取消/过期/容量、输入停止/单槽/空闲门、Backend真实回环TLS执行和Local签名child原始2×2红像素/源关闭后迟到拒绝已验。官方fmt/版本/全仓strict行数、strict all-targets Clippy及lib/普通Release/workspace三目标构建通过；21份Rust最大496有效行，没有新增行数例外。只分配一次v0.2.33.532，Loom源码未改。

仅新profile、自有窗口的前台恢复授权仍有效。本次以新空profile启动普通固定SHA产品，而非重放旧image/chrome启动。Computer Use的list_windows只返回一个无标题自有对象，其截图包含透明层下其他应用内容，accessibility为null；list_apps重新枚举仍只有同一自有对象，并混列用户release97。没有可独立选择的自有普通文档/Art/Live目标，故未激活或发送UI输入，没有选择用户release97，不改样式/捕获保护/输入法或换API绕过；不判定产品显示根因。 自有Job保底清理不等于自然退出；普通前台与人工IME不从隐藏测试推定。

Local 自定义主输入名仍缺完整目录/图/状态/保存恢复接线；逐端口显隐及默认值/持久化、可编辑capsule和完整端点键盘仍开放。普通Art/Live/完整键盘与人工IME、普通双产品、物理多屏跨机DPI、剩余全量/合计资源长稳、最终完整隔离包和用户全面验收仍须继续。Talk构想与旧客户端混用排除，三冻结副线/v464固定包保持，不启动Loom原生迁移，总迁移active。
原失败与夹具修正见[构建记录](WINDOWS_NATIVE_BUILD.md#v532-live-源-add-art-与独立子节点)，不继承为整库成绩。

## 当前 v531：工作区 chrome 实现与验收分界

**已实现并通过限定验证**：Ctrl+Shift+4的all/border/none、Unit外框和公共端点/名称展开、实际跨窗拖线提交及重接取消保边、Live旧边框/黄角/命中的三态接线。19项新chrome模型/真实HWND回归全过，含owned/region/affinity、几何/图复用、首paint后实际drop、隐藏恢复、锁/cycle/过期拒绝、捕获/销毁；Live GPU去框/恢复且源像素不污染已验。共享提交与单槽手势进入原生命周期门，不复制执行/保存业务。重接有效drop才替换、取消保旧边是相对legacy的明确安全改进，不声称旧取消行为完全一致。
本批同源去重90 Rust passed/0 failed/2既有默认ignored，严格静态/三目标构建通过，非整库。input1至4失败、首paint独立Win32实验及ignored解释保留；input5只改测试夹具真实paint，生产不因该修正变化。详细范围和普通EXE SHA见[构建记录](WINDOWS_NATIVE_BUILD.md#v531-工作区三态与跨窗端口连线)。
**仍缺实现**：逐端口显隐/capability默认值和持久化、可编辑端口capsule、完整端点键盘路径；不是完整可视端口对等。Live源Add Art和Local自定义主输入名也仍开放。
**仍缺验收**：用户已恢复仅自有测试窗口前台，但本轮工具无法独立选择自有文档/端口，截图包含其他应用，故没有激活或输入；不选用户release97，不改样式/捕获保护或换API。工具边界原因未定位，不能判产品显示失败；Job保底清理不等自然退出。普通可见三态/连线、Art/Live、完整键盘/人工IME、普通双产品、物理多屏跨机DPI、剩余全量/合计资源长稳、最终完整隔离包和用户全面验收仍开放。Talk构想/旧混用不计必做，三冻结副线/v464不动，总迁移active。

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

## 当前 v525：剩余回归新增 903 通过，不替代前台验收

此前未启动的 914 个完整名分组取得 903 passed、0 failed、11 默认 ignored；115 份原始 receipt/日志及 4441 输入 SHA 已核，源码/版本/产品库未变。真实文件中断恢复仅针对自有子进程与独占 linshi 文件，离屏绘制/合成长截图不当作用户桌面操作；自有 Job 兜底不当自然清理证明。
累计 3483 至少一次通过、61 默认 ignored、6 无通过证据、1745 未启动、1 原整体上限中断；不是单次全量成功，也不以测试数换算迁移进度。六项 JS foreground 前提、Art 间歇性和 Live 原清理未知仍保留，详见[构建记录](WINDOWS_NATIVE_BUILD.md#v525-剩余回归分组续接914-项终态非全量通过)。
自有窗口前台恢复授权有效，本批无前台输入，不重复普通键盘工具不可选旧启动；完整键盘/人工 IME/旋转 EDIT、普通双产品、多屏/跨机/DPI、全量/资源长稳及最终用户验收仍开放。Talk 构想、旧客户端混用、冻结副线与固定包不动，总迁移 active。

## 当前 v525：呈现与 Live Source 补证，不替代完整键盘

逐项隔离的 162 项原 SERIAL 连锁失败取得 154 passed / 8 failed / 0 PoisonError；当前 default desktop 的 32 项呈现及 CPU 编辑保存恢复共 33 项通过，原期限、库与源码不变。9 项 Live Source 在自有目标和原安全门下行为断言通过，但严格 Job 清理仅 8 项通过，1 项兜底回收；不冒充自然清理全绿或普通 Source 产品验收。
另有此前未执行的纯模型 124 项通过。六项 JavaScript 仍停在 foreground fixture，未验证主体；Art 原 settle 和 Live 原清理异常各一次诊断未复现，不视为根因关闭。按完整名累计仍有 2659 项未启动、1 项整体超时中断；“至少一次通过”不等单次全量成功，也不消除间歇性失败。详见[构建记录](WINDOWS_NATIVE_BUILD.md#v525-分组回归补证与剩余边界)。
本批无产品源码、版本或编译变化，保原失败与 dirty/untracked。自有前台授权有效，但普通产品完整键盘仍受工具不可选边界阻塞；人工 IME/旋转 EDIT、普通 Source/Loom 桌面壳、多屏/跨机/DPI、合计资源长稳与最终用户验收仍开放，总迁移 active。Talk 构想、旧客户端混用与冻结副线不计回必做。

## 当前 v525：回归与键盘边界仍为部分验证

默认原生库 5296 项的首次私有 desktop 运行在原 1800 秒上限结束，观察到 1891 passed / 205 failed / 48 ignored；另有 1 项在途被终止、3151 项未启动，不算全量通过。
205 个失败里 162 个明确为共享测试串行 Mutex 的 PoisonError 连锁；其他 43 项仍需分别核查。最初两项恢复回执门在同库/源码/期限、当前 desktop 自有窗口条件下各通过并确认 D3D12 S_OK，不抹掉原失败或扩大根因结论。
五个独立纯模型/图像/文档根模块另得 366 passed / 0 failed / 2 默认 ignored，ignored 不计通过。没有修改产品、重分版本或编译，详见[构建记录](WINDOWS_NATIVE_BUILD.md#v525-默认库审计与呈现环境对照全量尚未通过)。
普通产品键盘仍未验：Computer Use 未返回可选自有管理器对象，未配对、未开输出、未输入；临时 TLS 与自有 Job 已回收，Job 终止不冒充 graceful shutdown。
自有前台验收授权未撤回，不因旧暂停记录再次停工；工具不可选不等键盘产品缺陷，也不授权绕过 SecureDesktop 或输入安全围栏。
既有两条可见 Tile 父门和内部包成绩保留。全量、完整键盘/人工 IME/旋转 EDIT、普通 Source/Loom 桌面壳、多屏/跨机/DPI、合计资源长稳和最终用户验收继续开放；整体迁移 active。

## 当前 v525：普通可见 Tile 图片与混合生命周期

自有测试窗口前台验收已获明确恢复授权，仅限新 profile、自有窗口，不含用户现有窗口、全局输入或显示设置；本说明优先于下方旧暂停记录。
固定 SHA 的实际解包普通 EXE 已通过图片六阶段及图片/Live/Art 混合五阶段两条父门，均保原 30 秒、真实 D3D12 S_OK、服务端 applied 与无遮挡桌面像素独立 oracle。
正常关闭后 endpoint offline、applied 清空；混合来源附件/权威值/Source 保留至其显式停止，媒体连接最终归零。归档 9 项、解包工作区 7 项及 3 项 CLI 拒绝另行计数。
只关闭当前本机单屏上述限定可见门；测试库 Source/Loom 不冒充两个普通产品或 Loom 桌面壳，不关闭完整键盘/人工 IME/旋转 EDIT、历史根因、多屏/跨机/DPI、全量/资源长稳及最终用户验收。
没有产品源码修改、重分版本或重编，旧失败和冻结线保留；详见[构建记录](WINDOWS_NATIVE_BUILD.md#v525-解包产品与恢复授权后的普通可见验收)。

## 原版紧急退出双通道（v524，隔离定向通过）

补齐原版已实现的两条紧急退出：完整 Esc+Delete 三次组合（相邻严格小于400ms，双键重叠且完整释放），以及独立 watchdog 的 Ctrl+Alt+Shift+F12。主消息循环与 child 共用纯状态机；child 在配置/UI 前分流，先见松开才 arm，以原版8ms周期只读键位，不注入输入、不记录文本。
复用原同EXE受限白名单 launcher 和 Hello/Ready；直接父PID、创建时间、同EXE路径及pipe创建者共同认证。普通恢复权限仍只有查询/等待，只有紧急认证额外取得 PROCESS_TERMINATE，并只使用保活原父句柄。正常Drop和握手失败只回收自己新建watchdog，不终止Job或承担Live恢复责任的child。紧急退出绕过保存/析构，不能保证未保存工作保留；原Live父死恢复机制继续负责已登记源。原生未修改全局光标约束，故不无条件重置其他程序的 ClipCursor/ShowCursor/SetSystemCursor。
当前选定Rust按crate/完整名去重 106 passed、6 默认ignored，非整库；11项新增回归覆盖严格400ms边界、完整释放、启动已按住、缺失keyup/迟到消息、参数、认证失败不arm、握手失败、正常Drop及阻塞牺牲父进程的三连/备用终止和父死child退出。另复测原恢复认证/握手/管道/launcher、三进程父死恢复、系统会话消息/Checkpoint和普通固定SHA EXE工作区。全部在未切换的独立desktop/自有Job执行，不申请SWITCHDESKTOP，不向用户应用发送输入。
strict all-targets Clippy、lib/普通product/workspace/恢复认证及session目标构建、官方fmt/版本/全仓strict行数/diff通过。input1因Application达到503有效行而在构建前失败，原回执保留；input2按职责提取application_loop，保持原消息泵/显示恢复/保存/退出顺序，不增行数例外。input2 Clippy通过后，因只读审查发现局部消息可漏收keyup，停止本轮自建在途构建树并保留中止记录；input3主通道按当前键位快照保守累计，不凭历史消息制造重叠，新增负向回归。源码只分配一次v0.2.33.524，Loom未改。
新增常驻进程单独实测180秒（97点）：产品及watchdog全程存活，含idle60秒和三轮hidden20秒/resumed20秒；正常隔离QUERY/END后整棵自有树归零、desktop关闭、工作区保留。该4进程树（产品、watchdog及两个conhost）Private commit为8.691–8.957MiB，Working set合计61.430–61.578MiB（共享页可能重复计），模式内单核CPU约0.156–0.260%，Handles658/GDI33/USER16稳定。Ctrl+1仍记录已有热键占用0x80070581，不冒充快捷键验收。资源明细见main-emergency-exit-1/runtime-audit-3.json；不是Otter+Loom总资源、GPU、物理输入或长稳验收。
本批仅关闭源码与限定隔离门，不恢复前台自动化，不证明物理三连/F12或普通可见混合链通过。v518前台身份变化来源及v517普通可见混合门未关闭，不拿当前隔离通过覆盖历史失败。人工IME/旋转EDIT、物理多屏/跨机/DPI热插拔、资源长稳及最终隔离整包/用户全面手测仍开放。Talk外部会话控制按用户决定排除本轮，旧Web/Tauri混用不重列必做；三冻结副线、v464固定包及真实profile/设备不动，总迁移active，不是正式发布。

## 当前范围：Talk 不是已实现的对等功能

2026-11-03 用户确认 Talk 外部录音开关只是旧版未实现构想，暂不需要实施。
本轮不要求原生 Ctrl+Alt+Space 或 start/stop/toggle/cancel，不作为缺实现、待验或退出阻塞。
下方历史记录不改写，但不再累计其中 Talk 待办；详见[当前范围](WINDOWS_NATIVE_REVISED_PLAN.md#talk-录音开关构想不属于本轮迁移)。

## 系统结束会话持久化准入（v523，隔离消息链验证）

WM_QUERYENDSESSION只消费原Save/Checkpoint已提交凭证；未保存、在途/排队请求、Live待交付操作及本地Art输入/确认/结果未结算时立即拒绝，设置短ShutdownBlockReason，不在同步回调等待保存或弹框。批准后共享Queried阶段封住新业务和标准子控件编辑；END FALSE恢复，END TRUE不可逆进入Ended并沿原统一shutdown排空。系统结束不等于删除最后Unit，不写空工作区。
选定Rust按crate/完整名去重271 passed、7默认ignored，非整库；17组定向及普通固定SHA EXE工作区回归通过。全部窗口测试在未切换的独立desktop/自有Job运行，未申请SWITCHDESKTOP。普通产品验证启动/移动/检查点→QUERY→取消→继续移动/检查点→QUERY→确认结束，核原字节、唯一Unit/id及extension；不调用系统退出API。
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

## v520 本地Art自动回调的限定对等

用户在既有观看系统菜单配置严格Condition并明确“允许自动回调”；停止先撤本地gate。回调不伪造click/gesture、不请求焦点或恢复暂停timer。Applied只在正式端口原子提交后产生，formalRevision为当前执行实例成功提交序号，不是磁盘版本；NoResult只在实际present后报告。原Results最终predicate再核Runtime/Lease及固定期限，Stop与canonical事务共用无Win32/网络/脚本回调的短锁。
input4相关Rust按crate/完整名去重87 passed、7默认ignored，非整库；新自动回调子门虽默认ignored，已由真实LAN父门显式运行且父子不混加。真实签名child验证暂停回调/缺handler/迟到拒绝；隐藏Application验证提交前无Applied、取消保canonical、正式像素及Save恢复、实际present后NoResult。原本地Art缓存/有效授权离线/结果/宿主退休和Live观看/控制、普通固定SHA EXE工作区5项直接回归通过；前台退出门未运行。
本批仅关闭本地Art自动触发的源码/定向/隐藏Application及真实LAN联合接线，不是普通可见双产品、真实跨机或最终包验收。v518全局前台身份失败、v517可见混合门/v511历史键盘根因仍开放，前台自动化继续暂停；人工IME候选窗/旋转活动EDIT、物理多屏/跨机/DPI热插拔、长稳、其余迁移退出条件、最终隔离整包与用户全面手测仍开放。三旧副线、v464固定手测包与真实profile/设备保持；旧Web/Tauri混用不重列必做，总迁移active。 见[构建记录](WINDOWS_NATIVE_BUILD.md)。

## v519 本地Art触发的服务端与产品边界

宿主显式automatic_local_callback授权绑定完整Art实例、工作区/代际/包、Live会话/epoch、条件revision及独立authorizationId；Preview不等自动授权。Claim在HTTP响应前消费payload，重复/未知领取不重放；ACK仅报告宿主结果，不证明canonical保存或接收端呈现。本地delivery至多16KiB，超限明确停用；原观察新鲜度30秒、grant期限、墙钟高水位/每事件期限与单调期限共同围栏，查询不续期。停止、最后Viewer断连、审批修订变化、过期和Session close回收或拒绝执行，重连/重批不复活旧授权。没有新增线程、timer、持久队列、依赖或脚本权限。
input2选定相关Rust完整名按crate去重75 passed、1默认ignored，非整库；protocol39/daemon36，其中新增协议3项、服务12项。原Art观看/控制、Surface触发/观察、媒体断连清理与LAN边界回归通过，普通daemon构建及--version/--help通过。HTTP测试用owner直接登记Viewer在线，不是真实WSS或Otter联合。20份Rust最大457有效行；官方格式、版本、Otter全仓strict行数、两仓diff及精确UTF-8无BOM/空白门通过。
本批只关闭Loom本地Art Live触发服务实现及HTTP定向验证；Otter实际配置菜单、独立自动授权owner、onLiveTrigger私有command/SDK、签名child执行、正式结果/取消接线和真实HTTPS/WSS联合仍由MAIN继续完成，不能以此宣布本地Art触发产品完成。v518全局前台身份失败、v517可见混合门、v511历史键盘根因仍开放，不恢复前台自动化。人工IME候选窗/旋转活动EDIT、物理多屏/跨机/DPI热插拔、长稳、其余迁移退出条件、最终隔离整包与用户全面手测仍开放。三旧副线、v464固定手测包及真实profile/设备保持；旧Web/Tauri混用不重列必做。 见[构建记录](WINDOWS_NATIVE_BUILD.md)。

## v518 Tile键盘取消／联合回执

Tile Popup明确区分Select，真实打开的下拉列表先消费Esc并抑制同次长按；keyboard_window以Rc身份围住取消/同步销毁，只在同一已paint快照保留Tab位置，迟到epoch不得取消后继owner；外部失焦仍完整退休，父Tile拒同次Esc长按连带关闭，新鲜Esc仍退出。取消退休未发Edit后缀，不重放未知写或丢已接受本地值。
keyboard_joint_access及keyboard_fixture_tests仅cfg(test)，Tab后才计按钮受理，避免字段change冒充Enter成功；测试线程键表完整RAII恢复，生产Ctrl+Enter拒绝不放宽，并记录实际焦点/paint/修饰键。专属Combo实验只在未切换desktop及新测试线程运行，不申请SWITCHDESKTOP；只禁用该线程真实IME，人工组合消息不等人工候选窗验收。
input4当前264项相关Rust完整名去重passed、1 failed、5 ignored，非整库；Tile窗口93项实跑92 passed/1 failed，8项取消及3项夹具新增回归全过。8条真实Loom LAN父门通过，父子不混加：两种文本链在input4重跑62.406/69.218秒，六个非文本门按精确三份cfg(test)差异/SHA复用。普通固定SHA产品工作区5项通过证据复用，前台退出门未运行；product/workspace未因夹具变化重编。Otter strict Clippy/lib、官方格式/版本/全仓strict行数、diff及精确UTF-8无BOM/空白通过，9份Rust最大461有效行。
当前唯一终态失败为tab_edit_to_button_commits_local_draft_and_enter_repeat_does_not_duplicate的全局GetForegroundWindow前后身份断言，尚未确认是用户切窗、产品行为还是其他来源，不用历史通过覆盖、不反复重跑、不恢复前台自动化。故本批仅PARTIAL，不发布INTEGRATION_VALIDATED或完整键盘/整项迁移完成。v517混合可见门、v511历史根因、本地Art Live触发独立授权/执行/回执合同、人工IME/旋转活动EDIT、物理多屏/跨机/DPI热插拔、长稳、最终完整隔离包和用户全面手测仍开放；旧Web/Tauri混用不重列必做，三旧副线/v464固定包及真实profile设备保持。 见[构建记录](WINDOWS_NATIVE_BUILD.md)。

## v516 Tile像素对齐验证范围

presentation_readback_test独占cfg(test)有界staging/Map/预算与读回像素；composition_d3d12_readback_test读成功Present后的上一物理buffer，composition_readback_test读同一D3D11双缓冲flip-sequential链的已提交逻辑buffer1。software_surface/render_software/render_surface仅有cfg(test)委托。tile_presented_pixel_tests复用产品Renderer，独立矩阵核图片/Art/旋转/crop/透明叠层/换帧/黑场恢复。唯一生产结构变化为Renderer::new复用私有from_surface初始化，测试建链前选定后端，不在同HWND叠建两条GPU链。
native/tests/tile_product_runtime与support/tile_product_window独占普通固定SHA EXE、真实CLI/物理输出、隔离profile及自启PID/精确类名的窗口验证；Loom tile_product_native_joint只负责真实生产库LAN HTTPS/签名配对与续签、layout/presentation及权威ACK。四象限独立RGBA常量对应六阶段running/旋转/黑场/恢复/crop/冻结，外部仅采四个前后均确认无遮挡的桌面点，随后正常关闭和endpoint offline。桌面合成像素不冒充产品交换链直接读回，库内各后端读回也不冒充普通产品可见运行；父子不混加。子产品由Kill-on-close Job持有，失败/超时只清理自己的进程。初始可见门因遮挡明确失败后，按用户授权由Computer Use仅激活驱动的普通自有启动器一次，驱动核实际前台后向已核PID/类的同测试子Tile转交一次。不以该显式激活声称产品自动置顶或全程无激活；不移动鼠标、不注入用户应用、不改物理显示或真实数据。没有新产品诊断开关、依赖、路由、权限、线程或资源预算，旧三副线及v464固定包冻结。
本批只关闭上述本机单屏普通EXE静态图片可见链和产品Renderer各后端提交目标读回，不关闭普通EXE可见Art/Live、完整Tile键盘、人工IME候选窗/旋转活动EDIT视觉、物理多屏/跨机/DPI热插拔、长稳、本地Art Live触发、其余迁移退出条件、最终完整隔离包或用户全面手测。v511历史超时根因仍开放；旧Web/Tauri混用按用户确认排除。 136项相关Rust及4条真实LAN父调用通过，非整库；见[构建记录](WINDOWS_NATIVE_BUILD.md)。

## v515 Tile滚动条对齐范围

滑块可拖出原track/窗口，轨道立即翻页并在400ms后每50ms至多一步，不追赶积压；thumb到指针、移出track或反向时暂停，回到原方向再继续。活动EDIT由滚动条提交一次，保中文emoji草稿及无handler表单；IME组合拒绝接管。prepare空隙只被动保留且不滚动，空隙抬起释放、不采用迟到位置。同步paint前移出owner，取消轮换token，旧回调不得恢复旧手势或覆盖successor；隐藏/冻结/黑场/过期/撤权/DPI/关闭/丢capture及wheel/Tab接管都清捕获和timer。初始资格仍核topmost/crop/旋转/clip/布局/view/代际，捕获后屏外坐标不授新命中资格。不改scene/canonical、网络动作、权限、原队列/像素/TTL预算；无新hook、线程、依赖、WebView或用户应用输入注入。
本批仅关闭Tile滚动条实现、定向与隔离LAN集成；inputCapabilities仍pointer。完整键盘、人工IME候选窗/旋转活动EDIT视觉、普通可见Tile与最终交换链读回、物理多屏跨机/DPI热插拔、长稳、本地Art Live触发合同、其余迁移退出条件、最终完整隔离包及用户全面手测仍开放。v511历史超时根因仍未确立，后续成功不反证旧故障已修复；旧Web/Tauri混用按用户确认排除。 245项相关Rust及11条真实LAN父调用通过，非整库；见[构建记录](WINDOWS_NATIVE_BUILD.md)。

## v514 Tile滚动/Tab对齐范围

滚轮复用系统行/字符/整页偏好、Shift水平与Ctrl让出，嵌套祖先只消费一次；Tab/反向Tab能显露屏外字段，仍受crop、四向旋转、最上层命中和有界搜索约束。重投影重装嵌入文本/Select/Slider绘制缓存，保中文emoji草稿和无handler本地值；同view/代际/尺寸新snapshot继承并钳位scroll，换代重置。IME组合不提交或换焦；隐藏/冻结/过期/准备空隙拒新滚动，旧按压不重新命中新位置。不改权威scene/canonical、表单字段范围、协议、权限、队列/像素预算、原1500ms/120秒门或未知写不重放规则；无WebView、新依赖、hook、线程或输入注入。
本批关闭Tile本地wheel与Tab屏外reveal的实现、定向和隔离LAN链；不等完整键盘、滚动条拖拽/重复翻页、人工IME候选窗/旋转活动EDIT视觉、普通Tile输出EXE可见链、最终交换链读回、物理多屏跨机/DPI热插拔、长稳、本地Art Live触发合同、其余迁移退出条件或最终完整隔离包/用户全面手测。inputCapabilities仍为pointer；旧Web/Tauri混用互通按用户确认排除，v511旧超时根因仍未确立。 232项相关Rust及10条真实LAN父调用通过，非整库；详见[构建记录](WINDOWS_NATIVE_BUILD.md)。

## v513待派发意图保护对齐范围

重绘时普通操作等待对应Art实际paint，不把候选publish当呈现，也不提前消费后丢弃；未发原命令可跨短暂prepare空隙保留，原事件ID/顺序/期限不变。弱预留占用原32槽/8MiB，停止/清理/换代和Edit取消退休；昂贵准备后增加写前围栏，HTTP后无恢复或未知写重放，冻结Cancel语义保留。144项相关Rust去重、11新回归及9条真实LAN父调用通过，非整库，证据见[构建记录](WINDOWS_NATIVE_BUILD.md)。
v511历史超时根因仍开放；不将本批扩大为完整键盘/屏外scroll或reveal、人工IME/旋转活动编辑视觉、普通Tile输出EXE可见链/最终交换链、物理多屏跨机/DPI热插拔、长稳、本地Art Live触发、完整迁移或最终包验收。inputCapabilities仍为pointer，旧Web/Tauri混用互通排除规则不变。

## v512无handler值控件对齐范围

无handler值可鼠标/键盘编辑；同合同同源换快照保本地值，新权威源值替换，view/代际/合同/布局/几何变化退休。准备空隙仅被动保留且拒新输入，隐藏/冻结/黑场/停止/撤权/过期/DPI或窗口换代清理；Select取消不变值，Slider取消恢复此前接受值。与文本共享每view64/总256字段，不加倍预算；本地操作不offer空动作。surfaceValues仍只含input/textarea/number字符串，不加入Switch/Select/Slider，保原16字段/48KiB表单门、祖先click、队列/权限/资源预算。
v511的键盘/Tab接线及当前回归虽已通过，input6的values LAN派发超时仍缺根因证据；后续成功不等生产修复，不发布该批INTEGRATION_VALIDATED，也不把旧失败改写为通过。
本批关闭无handler值控件的实现、定向及隔离LAN集成；不关闭完整键盘/屏外reveal或scroll、人工IME候选窗/旋转活动编辑视觉、普通Tile输出EXE可见链、最终交换链读回、物理多屏/跨机/DPI热插拔、长稳、本地Art Live触发合同、其余迁移退出条件和最终完整隔离包/用户全面手测。inputCapabilities仍为pointer；旧Web/Tauri混用互通已按用户确认排除。 当前274项定向及9条真实LAN父调用通过，非整库；详见[构建记录](WINDOWS_NATIVE_BUILD.md)。

## v510当前文本与草稿表单对齐范围

原生Tile现支持文本/textarea/number输入、IME组合中间态抑制、CRLF归一、number校验/step、连续input合并、唯一change及按钮提交当前文本草稿。失败、取消与窗口/授权换代清理，独立实际handler和服务权威/正式/已paint由真实联合核验，不以ACK代成功。完整回执见[构建记录](WINDOWS_NATIVE_BUILD.md)。本批关闭Tile文本/textarea/number、IME合成边沿及文本草稿表单按钮的隔离闭环，不等完整键盘/Tab导航、已安装输入法候选窗人工验收、旋转活动EDIT视觉、全部值控件本地draft/完整表单。inputCapabilities仍为pointer。普通Tile输出EXE可见链、最终交换链读回、物理多屏/跨机/热插拔/真实DPI、本地Art Live触发合同、长稳、其余迁移退出条件、最终完整隔离包和用户全面手测仍开放；旧Web/Tauri混用互通已按用户确认排除。

## v509 当前 Slider 对齐范围

Tile声明式Slider已支持源坐标轨道/step归一、原HWND拖动/捕获、连续input有界合并、抬起change、异步待结果预览及失败/取消清理；实际handler/权威/正式结果和已paint场景由真实LAN联合核验，不以ACK代成功。完整回执见[构建记录](WINDOWS_NATIVE_BUILD.md)。只覆盖声明了 input/change 的 Slider 鼠标交互；无 handler 仍只读，文本/IME/完整键盘与完整表单仍开放，inputCapabilities 保持 pointer。普通 Tile 输出 EXE 可见链、最终交换链读回、物理多屏/跨机/热插拔/真实 DPI、本地 Art Live 触发合同、长稳和最终整包/用户全面手测仍开放；旧 Web/Tauri 混用互通已按用户确认排除，不再计为本轮待办。

## 当前验收范围澄清（2026-11-02 用户确认）

功能对照不等于要求兼容旧客户端。依据
[修订计划的旧客户端互通排除规则](WINDOWS_NATIVE_REVISED_PLAN.md#旧客户端互通不属于本轮验收)，
旧 Web/Tauri 与新原生 Otter 的混用互通不纳入本轮必做、进度分母或退出条件；
历史“未验证”仅为覆盖说明，不自动转成当前待办。新原生端与配套 Loom、新原生端之间的功能及正式验收仍保留。

## 当前目标补充：双后端与 Art 单侧执行

[统一修订计划](WINDOWS_NATIVE_REVISED_PLAN.md) 新增 D3D12 优先/D3D11 按需/CPU 兜底、
有界设备恢复、原生 Shader 替代及 ANGLE 退役、源码 Art 获取与 Otter 单侧执行验收。
这些是待实现/待验证项，不因已有 D3D11、QuickJS 或隐藏测试而标为完成。
原产品功能缺口、启动/呈现历史失败及最终用户全面手测仍保留；旧 GLSL 原样兼容不再是目标约束。

## v412：无 raster 实时外框的限定闭环

未保存 HWND resize、minified Sticker 当前 expanded 与 Art 普通框已接原发送 PNG；自然分辨率不冒充逻辑框。
独立整图菜单/TLS oracle、保存/重启及 resize/minify/Space/stale mini 围栏回归通过，原 crop/source 优先级保留。
非等比复杂传播布局、历史 erase/flip、跨 Unit 自然源、活动 blur 与完整 PNG parity 仍开放。
实际成绩和产品边界见 [v412 构建记录](WINDOWS_NATIVE_BUILD.md#v412无-raster-目标实时外框)。

## v411：稳定发送关联显式停止

稳定单目标绑定的独立菜单与减权→canonical pending→unlink/持久回执→canonical 移除/ack 已接线，同源定向 lib 回归通过。
停止入口不依赖设备目录，普通退出/换服务不隐式全解绑；只在实际解绑和最终 canonical commit 后报告完成。
原回执保留到精确 ack；终态移除后才允许同目标新发送，旧回执不删除新意图。
新增回归覆盖菜单/TLS、回执发布后重启、canonical 移除后 ack 前重启及 Begin 基线失败后的显式重试。
这些是隐藏合成 profile/loopback 场景，不是真实 Loom 双端、全崩溃注入或最终用户验收；结果见 [v411 构建记录](WINDOWS_NATIVE_BUILD.md#v411稳定发送关联显式停止)。
四项正式集成中 startup 为 6 过/1 超时，原 15 秒期限未改；其他三项通过，不等于启动根因或全部选定门已通过。
完整 PNG parity、设备组/持续发送、双向/LiveRelay/Tile、资源长稳、完整迁移及 startup/呈现旧失败仍开放。

## v410：投送目标编辑与外观归属

resolved 上游图像与发送目标编辑已在生产快照分开：目标当前 crop/reset、栅格和矢量标注不再被上游 editor 取代。
目标 canonical 纯外观已接原后台 PNG 链：边框、全栈圆角和 beautify 最后包装；最终尺寸与实际 PNG 一起签名。
这是目标编辑/纯外观的接线，不是完整 PNG parity：历史 erase/flip、传播内容框、跨 Unit 自然图源和活动 blur 采样仍未闭合。
隐藏合成 profile/loopback TLS 的证据不替代真实图拓扑、真实 Loom 双端、完整迁移门或最终全面手测。
同源定向 lib 门通过，但 startup 正式集成为 5 过/2 超时，同字节计时诊断为 6 过/1 超时；启动集成与根因仍开放。
实际测试和产物见 [v410 构建记录](WINDOWS_NATIVE_BUILD.md#v410投送目标编辑与外观归属)。下方 v409 为历史。

## v409：本地保留原图生产捕获

本地、无 resolved frame 的发送现在可选择经活动 editor 身份校验的 retained natural source，
而不是始终发送低分辨率 layers.base。底图擦除失配时只发送当前新底图；undo 后允许恢复匹配原图。
当前 crop 仍按原图密度计算输出，editable/raster annotations 与 normal opacity 继续独立合成。
普通 Editor 没有独立 logical_size 时，发送端使用当前 raster size 作逻辑框回退，不再误用自然图像像素框。
同尺寸同字节但不同 allocation 的 retained 替换也会使捕获 Scope 过期，Weak 凭证不持有像素强引用。

新增菜单/TLS 用例覆盖 natural/内容擦除/undo/crop 的实际 PNG 尺寸、全图像素及签名 digest，
并沿原 canonical checkpoint/重启 binding 链验证；标注/opacity 组合和 Weak 围栏另为直接 Scope→PNG 测试。
测试运行结果以 [v409 构建记录](WINDOWS_NATIVE_BUILD.md#v409本地保留原图生产捕获)为准，不混报两类覆盖。
resolved 分支维持旧语义，不借目标 retained；完整目标 metadata、上游传播、历史编辑和 beautify parity 仍为未完成。
本轮不关闭 v398 间歇呈现失败，不等于真实 Loom 双端、完整 N01–N12 或最终全面手测。下方 v408 为历史。

## v408：有界源分辨率编码出口

发送当前已提交底图已接原 source-resolution 合成出口；源像素尺寸与小数逻辑标注框分离，
不因宿主旧 image 或逻辑框更小而替换/降采样当前底图。裁剪仍按源密度；普通文件导出不改。
发送准入覆盖 source/inner/beautify outer，保留 8192/16M、实际 PNG 4 MiB、共享预算和迟到取消。
显式传入 geometry 的美化编码出口不等生产菜单已捕获 beautify 设置。

尚未完成：恢复/缩小 retained source 的安全捕获、目标与上游 metadata 分离、历史擦除/传播框/
边框圆角/beautify 生产接线。因此完整 PNG parity 仍为 false，不把独立合成测试当作此项完成。
本轮菜单/TLS 回归额外核实际 PNG 尺寸、全图像素及签名 digest；仍非真实 Loom 双端/最终用户验收。
Live 保存成员按同一 Workspace 身份和当前图成员资格共同选择，退休但未 reap 的 worker owner 不再阻断保存；
仍在图内却关闭的 Live 保持失败关闭。此项与 v398 间歇呈现失败不同，不将其一并关闭。
最终同源选定 26 组门、四项集成和隐藏隔离产品启动通过；原失败保留，未跑全量 lib/完整迁移门或最后全面手测。
结果见 [v408 构建记录](WINDOWS_NATIVE_BUILD.md#v408有界源分辨率编码出口)，下方 v407 为历史。

## v407：单目标发送生产接线

单个 SharedLoom 目标已从原生菜单接通当前显示 PNG→原 Job 网络链→后台 Binding→canonical checkpoint，
并有显式取消及复用原 Request 的重试；不是只给测试注入 candidate。成功文案以精确 canonical inclusion 为准，
不把 Create、手动另存、未知 index 或候选生成当保存完成，也不声称接收端已经呈现。
后台准备/完成核验复用原 Lease/四槽；UI join 不读完整 outbox，后续完整授权验证仍检查基线。
源失效/撤权/换工作区/退出保留原取消责任，确认 unlink 后才退休，不自动全量解绑稳定已保存关联。

当前 PNG 覆盖 resolved frame、裁剪/committed annotations/normal opacity；原始分辨率、缩小保留源、完整 beautify 尚未对齐。
设备组、持续发送、稳定 binding 显式停止、真实双端/离线互信、双向编辑、LiveRelay/Tile、资源长稳与最终全面手测仍开放。
仅选定隐藏 HWND/合成 profile/loopback TLS 证据，不替代完整迁移、真实游戏或用户验收；旧间歇失败未关闭。
本轮最终结果见 [v407 构建记录](WINDOWS_NATIVE_BUILD.md#v407单目标发送生产接线)，实现/边界见 [迁移记录](WINDOWS_NATIVE_MIGRATION.md#v407单目标发送生产接线)。
下方 v406 为历史。

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

## v394：已保存接收贴图删除清理

已保存接收贴图删除→单向持久标记→原设备当前授权自动解绑已接通，失败保标记并按 1/2/4/8/16/30 秒封顶退避，只有 typed true 退休。在途取消/工作区切换补偿、回执恢复重试、read/持续同步、发送/策略/离线互信/双向 journal/Tile 仍缺，完整 N09/N01–N12/最终用户包未验收。

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

显式接受菜单→原 Scope/Grant→隐藏追加→精确 checkpoint→可见原图新帧→认证 displayed 已接通。失败/取消保图片和 journal，成功不视为 unlink。自动生命周期补偿/回执恢复重试、持续同步、发送/自动策略/离线互信/双向 journal/Tile、完整 N09/N01–N12 和最终用户包仍缺。

最终严格静态/3799 文件 ratchet 零违规/警告/新例外；同源八件 45 选定门全过。
24 变更/7 新增、最大 381 有效行，UTF-8 无 BOM；visible 3/0、present 5/0、
placement 2/0、确认 1/0，过滤组不累计覆盖。344 旧件/七诊断、开发失败/输入说明、
最终冻结输入/七文档与源码三次匹配见迁移台账 v393/closeout。startup 7/0 不关旧根因，
正式前台菜单仍 ignored；无真实时钟到期/worker panic 全链注入，不代完整迁移。

## 最新状态

v392 接入 Application 唯一接收 owner：发网前原 Source/Space/Guard、同 Store/Record、
原 Grant/Hold 的容量与 intent 许可贯穿后台准备→隐藏追加→精确 checkpoint；错误/
取消在 UI 领取前不释放，终态回收，不隐式删图片/退休 journal。复用 Wake/绝对到期
等待，不增加轮询/等待句柄或第二任务池；Preparing 进入原快照/恢复/退出围栏但不
阻塞自身发布/保存。当前仅隐藏持久化，接受菜单/可见 present/displayed/自动补偿/
持续同步/发送端/离线互信/双向 journal/Tile 及完整迁移/最终用户包仍未完成。
最终严格静态/3,792 文件 ratchet 零违规/警告/新例外，同源八件 41 选定运行门全过；
25 变更/6 新增、最大 454 有效行，UTF-8 无 BOM。owner 10/0、watch 2/0；过滤组重叠，不累计覆盖。四份开发输入、
最终 2,640 冻结输入/七文档、首个静态候选/失败程序/析构诊断及 336 旧件/五诊断、
后续 344 件/七诊断保护见迁移台账 v392/closeout。startup 7/0 不关旧偶发根因，前台
菜单仍 ignored；原到期等待/准备 panic 没有新增全链故障注入，不代完整迁移。

v391 补接收意图 UUID/设备与 qrProjection 完整快照持久关联；原后台保存/恢复解析
有界共享 Source 索引，只有真实 checkpoint commit/规范启动才提升包含事实，暂存/
取消/过时 Receipt/竞争目标/恢复副本/普通导入/新 Source 或新 intent 不借旧证明。
恢复菜单仅短分已保存/未结，不获网络或显示权，不退休 journal/删除图片；精确
快照还须比较 Binding 的 revision/digest，提交事实不保证外部修改后文件仍存在。
最终严格静态/3,786 文件 ratchet 零违规/警告/新例外，同源八件 39 选定门全过；
28 变更/4 新增、最大 349 有效行，UTF-8 无 BOM。关联模型 5/0、checkpoint 新组 5/0/
全组 18/0、profile 4/0、恢复菜单 4/0、Application 解绑 8/0、投送桌面 25/0/
客户端 55/0；过滤组重叠不累计覆盖。四份开发输入、最终 2,634 冻结源码输入/七文档、
首个静态候选的保留与 null 兼容修正后 final-run2、328 旧件/五诊断及后续 336 件/
五诊断保护见迁移台账 v391/closeout。startup 7/0 不关旧偶发根因，workspace_runtime
2/0/1 ignored 不代正式前台菜单退出。接受入口/接收状态及原 Grant 跨异步生命周期/
可见 present/displayed/自动补偿/持续同步/发送端/离线互信/双向 journal/
Tile 与完整 N09/N01–N12、旧前台/启动/压力及最终用户包仍未完成。

v390 接通普通 profile 恢复屏障后的后台单次 journal 初始化、共享长期 owner 与显式
未结投送解绑菜单；确认默认否、原 Arc/profile/当前 Lease/原设备/来源复核后沿原
Sessions 单槽发 unlink，只在 unlinked:true 退休。取消/切换后的提交由内存状态对账，
本地图片不删除，不自动 accept/unlink。检查/显式无 profile 启动隔离、坏 journal
不覆盖；最终严格静态/3,782 文件 ratchet 零违规/警告/新例外，同源八件 37 选定
门全过，19 变更/5 新增、最大 453 有效行，UTF-8 无 BOM。profile 4/0、菜单 3/0、
Application 解绑 8/0、journal 15/0、投送桌面 24/0/客户端 50/0，过滤组不累计
覆盖。两份开发输入、2,630 冻结输入/七文档、初始零匹配/夹具失败程序、320 旧件/
四诊断及后续 328 件/五诊断保护见迁移台账 v390/closeout。startup 7/0 不关旧
偶发根因，前台菜单仍 ignored；接受入口、checkpoint 包含凭证、真实显示/
displayed/自动生命周期补偿/持续同步与完整 N09/N01–N12、最终用户包仍未完成。

v389 增补传统接收写前持久队列与显式认证解绑内核：成功落盘才允许 accept，原
expectedRevision/digest 幂等保留；同 intent 网络/Ready 许可防重复或过期 Ticket
操作，跨 key 仍用原四槽。只在原设备/原来源 unlinked:true 后退休，丢回复/撤权/
取消/403 保留不确定副作用，不自动换 Grant 重收。复用原文件 CAS/唯一恢复副本，
不覆盖外部修改或迁移旧版数据。最终严格静态/3,777 文件 ratchet 和同源八件的 34
个选定运行门全过，16 变更/6 新增、最大 440 有效行，UTF-8 无 BOM。journal 14/0、
认证补偿 9/0、投送客户端 49/0；过滤组重叠不累计独立覆盖。六份开发输入、最终
2,625 冻结输入/七文档、源码/程序身份及 312 旧件/四诊断由 closeout 复核，后续
保护 320 件/四诊断，详见迁移台账 v389。startup 7/0 不关闭旧偶发根因，正式
菜单仍 ignored；接受菜单、
workspace/Unit 意图关联与恢复、可见呈现/displayed、退出补偿调度、持续同步、
离线互信/双向 journal/Tile 和完整 N09/N01–N12 仍缺；不关闭旧前台/启动/资源门。

v388 已实现并验证传统接收准备内核：accept/有界 PNG/原摘要/完整 WIC 解码、同 Grant
Ready、确定 delivery-ID 与 qrProjection 单窗口候选，以及共用追加事务。新增测试
验证真实 TLS→隐藏 HWND→检查点→重启恢复，不算真实显示/displayed。接受菜单、
持久副作用补偿和 displayed 消费者尚未开放，整链/N09/完整迁移仍未完成；最终门
已封存：严格静态/3,771 文件 ratchet 和同源八件的 32 个选定门全部通过，31 变更/
12 新增、最大 367 有效行，UTF-8 无 BOM。接收原图/编辑器投影分别逐字节核对；
冻结库 32/0、frozen 14/0、Candidate/工作流 55/0，过滤组重叠不累计覆盖。本轮
startup 7/0 不关闭 v387 偶发超时根因；正式前台菜单仍 ignored，其他历史失败仍开放。
首轮 Clippy 大枚举失败/exact 输入另保，Box 化后 final-run2 通过；304 旧件/四诊断
与源码/八件身份及后续 312 件保护见迁移台账 v388/closeout，不代完整接收或迁移验收。

v387接入传统投送真实共享桌面菜单→Application→原授权Lease/后台Job→HTTPS→typed
结果：仅显式单次元数据查询和新邀请拒绝，recorded:true才删除候选。切换/撤权/
更新授权/成功workspace交换/退出丢弃旧完成结果，Arc身份防同ID替换，模态/恢复/
退出不启动新请求，取消未回收仍占原共享容量。开发夹具失败完整输入另保，不放宽
坐标准入；最终严格静态/同源八件/运行实绩见迁移台账v387，不拿中间green代批准。
accept/PNG下载/落盘显示/displayed/持续同步/离线互信/双向journal/Tile仍缺；完整
N09/N01–N12、真实Loom、旧前台/压力失败、资源长稳和最终用户包仍开放。

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

v386聚焦v385真实工作区移动超时：先用隐藏自有HWND取得sent-only移动无队列唤醒
的确定性RED，再补每窗口一条合并结算通知；保DefWindowProc尺寸派发、原保存单槽/
Receipt/CAS/检查点revision，不新增轮询。最终静态/同源原产品链见迁移台账v386；
不放宽原15秒、不重跑菜单前台、不关闭旧失败或完整迁移，N09桌面接收仍缺。

v386最终final-run严格fmt、全目标Clippy -D warnings/check、版本及官方ratchet
均过，零违规/警告/新例外；7份native变更/3新增，最大479有效行，UTF-8无BOM。
同源八件21个选定后台/隔离门全过：wake3/0、checkpoint8/0（含两项新隐藏HWND），
原workspace_runtime2/0/1 ignored完成移动提交→强制中断→重启恢复→删除最后Unit
落空；保原15秒和断言。正式菜单退出仍ignored未验，不关闭v384前台或旧完整组/
压力失败。三次源码/288旧件/四诊断/v385完整失败输入及原现场由closeout复核，
后续保护296件；状态window-geometry-wake-and-workspace-runtime-verified，不代完整迁移。

v385开始实现N09传统投送的共享Loom收件箱查询/新邀请拒绝：严格有界元数据、原
Control认证HTTP及共用4槽/Lease后台Job；过期pending仍可拒绝，不把accepted项当
新邀请，不伪造displayed。只是客户端/worker，桌面接入、accept/下载/持久化呈现、
持续同步/恢复、离线互信、双向journal及Tile仍缺，不广告默认接收可用。最终验证
见迁移台账v385；v384前台失败及旧完整组/压力/总资源/完整迁移门仍开放。

v385最终final-run2严格fmt、全目标Clippy -D warnings/check、版本及3,749文件ratchet
均过，零违规/警告/新例外。10份native变更/7新增，最大362有效行；四生产模块
168/111/29/128，三tests186/151/196，UTF-8无BOM。同源八件的13个选定后台/隔离
门12通过/1失败，projection16/0、device HTTP144/0（包含新9项，不能累计独立覆盖）。
workspace_runtime1/1/1 ignored在checkpoint-moved-window原15秒超时，尚未进入中断/
重启/删除链；失败profile仍revision7原坐标，stderr仅热键冲突，不足以确定根因。
菜单前台门没有重跑，ignored不计通过。三次源码/
旧280件/四诊断、首轮静态失败完整输入及288件后续保护见final-run2/closeout；
状态projection-client-implemented-workspace-runtime-pending，不代桌面/N09或完整迁移。

v384只修正式菜单退出测试的Win32 Static类名大小写发现，保PID/owner/visible负向
和原产品前台/保存准入。真实matcher RED0/1→GREEN1/0；开发阶段正式菜单→确认→
最终revision+1及内容保持→进程成功退出1/0。最终同源封存候选仍须单独显式运行，
不将默认ignored计通过；最终实绩见迁移台账v384，不关闭完整N06/N01–N12、旧前台
return/完整组/压力未知失败、N09传统投送/Tile、资源长稳或最终用户包。

v384最终严格静态/3,742文件ratchet均过，两份测试变更/一新增最大197有效行；封存
八件19运行门18过/1失败。workspace_runtime2/0/1 ignored含新matcher；正式菜单
显式门0/1在原15秒owned-product-foreground前提失败，未入菜单。开发1/0不能代
最终原生批准，状态native-exit-menu-foreground-pending；三次源码/完整失败输入/
八件/旧272件/四诊断及280件后续保护由closeout复核，不继续争抢前台或扩大旧门结论。

v383为N06正常退出增加原单槽最终检查点及独立Live退出Intent；确认后仍待Receipt/
CAS成功才销毁窗口，纯流更新不使一次退出批准失效、但最终强制捕获最新可用帧。
手动dirty恒Changed不变；晚到文件/建窗/Live请求、stale、无首帧或错误均保窗口并
消费本次退出意图。没有profile/只读owner不新增写权；不是持续视频或断电保证。
实现/RED/开发失败及最终验证状态见迁移台账v383；完整N06/N01–N12、旧前台/压力
失败、N09传统投送/Tile消费者、资源长稳与最终用户包仍开放。

v383首轮final-run为17/19门通过：startup4/3揭示完整restore idle不应阻止无profile
的既有Discard/退出清理，产品菜单0/1则在真实前台前提超时。保留完整失败输入/八
程序与回执；final-run2分离退出批准与profile快照空闲，不放宽Receipt或删旧断言。
修正候选不重复争抢前台、不继承旧字节原生结论；菜单整链仍未验，ignored不计通过。

v383最终final-run2严格静态/3,741文件ratchet及18个选定后台/隔离门均过；退出11/0、
Live checkpoint19/0、相邻Live58/0逐层包含，startup恢复7/0，workspace_runtime1/0/
1 ignored（菜单退出）。20变更/6新增最大450行，源码/八件/旧256件/四诊断及首轮
失败完整输入/八件由closeout复核，下一轮272件另保四诊断。状态仍为
exit-checkpoint-native-pending，不把18后台门或ignored当产品菜单、完整N06或迁移验收。

v382接Live首帧/持久模型变化的时间点检查点和显式文件自动保存；后续流序号不触发
逐帧落盘，首帧前/忙手势延后，恢复为静态贴纸而非重新授权捕获/控制。手动dirty和
默认检查点的独立文件基线不变。多Live freeze/recycle不提前消费全部请求，归还槽
以内部唤醒续进；实现/开发失败/最终实绩见迁移台账v382。
最近成功提交不是正常退出最终flush或断电零丢失；不关闭旧前台/完整组/压力失败、
N09传统投送/Tile缺口、完整N06/N01–N12、资源长稳或最终用户包验收。

v382最终17个相关运行门全过，新Live检查点8/0、完整相邻Live组47/0（含8项）、本地
手势9/0及真实产品四启动门均过，无ignored；严格静态/3,735文件ratchet通过，18变更/
4新增最大437有效行。三次源码/八件/旧248件/四诊断、失败日志及256件后续保护见
迁移台账和final-run/closeout；只关闭Live时间点自动持久化切片，不代完整迁移验收。

v381接默认普通启动的静态工作区持续检查点/隐藏恢复屏障，不只是记住手动文件路径。
独立profile/跨进程租约/目录pin、原单槽/Receipt/有界编码/文件事务负责全链；检查点
不标记手动文件已保存，不执行workflow，删除最后Unit写空、整体退出不写空。显式
CLI来源独立，不访问旧Tauri数据；坏/未知/含混副本失败保留，唯一副本只读。仅保证
最近成功快照，Live稳定自动保存/正常退出最终flush/断电零丢失和完整N06仍未完成。
开发失败、最终运行实绩及240旧程序/四诊断保护详见迁移台账v381，不关闭旧前台失败。

v381最终final-run2的14个N06相关门全过，含默认只读启动5/0、真实普通EXE移动提交/
强制结束/重启恢复/删除最后Unit写空1/0；检查点6/0含在途手动/open/exit、冲突、
stale及最近成功快照恢复，均无ignored。严格静态/3,731文件ratchet通过，25变更/8新增
最大447有效行；三次源码/八程序/旧240件/四诊断、首轮Clippy失败完整源码及248件
后续保护见迁移台账v381与closeout，不代Live/最终flush/旧前台或完整迁移验收。

v380补N06已保存静态工作区的显式事件驱动自动保存：菜单/多窗口共享代际→原保存
单槽/Receipt→原子替换→重新恢复，复用现有格式/冲突保护，不新增默认数据目录写入。
默认关闭，不持久化开关；手势/后台busy延后，过期候选不提交，off→on不复用旧授权；
manual save/open/exit保原意图与确认，失败只报告一次并停用，成功换工作区/另存路径
关闭授权，不自动执行外部workflow。Live时间点快照仍显式，未实现默认启动恢复、
崩溃前持续恢复或完整N06；开发失败和最终实绩见迁移台账v380，不继承旧green。
N09只读核查确认LiveRelay/Surface已有实现，但传统投送邀请/设备接收策略/Tile整版
定时呈现链没有对应原生消费者，仍是实现/接入缺口，不仅仅缺人工验收。

v380最终final-run2同源10相关门全过：自动保存8/0、保存25/0、恢复3/0、文件事务
29/0、写入6/0、加载13/0、资源2/0及1/0、产品startup7/0、session_startup9/0。
严格静态/3,723文件ratchet通过，19变更/4新增最大432行；构造整体移动未新增豁免。
源码/六件程序/旧234件及四诊断、240件后续保护见迁移台账v380与closeout，不将
本次静态自动保存成功替代默认启动/崩溃/Live恢复、旧前台失败或完整迁移/用户包验收。

v379为`foreground-handoff-native-pending`：v378同一封存库复现两项前台失败；诊断反证
只加授权可修复，真实自有点击后又观察到跨进程父方缺少许可归还。现仅测试夹具候选
增加显式启动握手、父/Child定向许可和return；Application握手位于原TLS计时之前，
产品前台拒绝/FIFO/预算不改。最后原生尝试被实际输入打断，父窗口在发Child命令前
已丢前台，新增归还路径尚未通过。停止争抢前台后仅做后台验证；四项前台最终门未
执行，v376两失败、旧完整组/压力未知根因仍开放。证据、身份与最终后台结果见迁移
台账v379；v378仍是最近选定回归全过的候选，不替换用户包或宣称完整迁移完成。

最终严格静态/版本及3,719文件ratchet通过，四份native测试变更最大467有效行；同源
六程序八个后台门全部通过，Engine121/0/5 ignored、Session77/0/26 ignored、跨进程
UIA与实际产品Enter worker均显式1/0、资源/启动/worker均过。忽略项不计通过；三次
源码、六件/旧228件、四诊断与234份后续保护见迁移台账v379。同源跨进程前台0/1仍
保留，不以后台成功关闭前台/归还路径或完整迁移验收。

v378已实现受限四边实心边框：CSS简写/长属性及cascade/reset→四边Insets→原生测量、
布局/client/scroll/clip/hit/EDIT→不同颜色D2D像素。只支持px/0及1/3/5px关键词、solid/
none/hidden；非继承，正used width必须border-box，其他线型/image/逻辑方向仍拒绝。
开发真实RED及失败保留；最终22相关门全过，含Engine121/0/5 ignored、Session77/0/
26 ignored、原生测量137/0、EDIT62/0、真实隐藏HWND、摘要绑定产品及四原预算压力门。
严格静态/3,719文件ratchet、45变更/11新增/最大445行、三次源码/六件/旧222件与后续
228件保护见迁移台账v378，不将ignored计通过或关闭旧失败。支持边界见同台账；默认完整
JS、真实股票/Loom、Lease/OS隔离、长稳、完整N01–N12及最终用户包仍未验收。

v377接通受限单行文字ellipsis：CSS准入/非继承/上下文拒绝→最终宽度→DirectWrite
省略像素与命中，保完整文本/原metrics/固有测量。仅文字Element叶、nowrap、left、
同轴hidden、实际单行；不是容器inline flow、多行/line-clamp、center/right或控件
内部文字支持。隐藏后缀为一个视觉cluster，双击省略号复制整个后缀；完整文本未截断。
真实行为RED0/1与开发失败留存；最终19个相关运行门全过，含DWrite/WIC/文字命中、
Engine117/0/5 ignored、Session76/0/25 ignored、同child/实际产品及隐藏HWND，四个原
预算压力门均显式1/0。17份native变更/7新增/最大440行，3,708文件ratchet零违规警告。
三次源码、六件/旧216件与222件后续保护和精确身份见迁移台账v377。默认完整JS、真实
股票/Loom、Lease、OS隔离、总资源长稳、N01–N12/最终用户包仍未验收；不以本次切片
通过关闭v376两项前台失败、旧完整组失败或压力未知根因，不作性能加速结论。

v376为partial-runtime-failure：仅将单次computed的winner Map收敛为空原型record，
不改变CSS支持集/优先级或新增缓存/权限。新模型2/0及原oracle通过，Engine114/0/
5 ignored、Session74/0/24 ignored与四个原规模压力门通过；固定20门18过/2失败。
跨进程focus负向在Child激活前提失败，Application Enter第二轮前台前提失败，均不
被其他focus/产品键盘通过覆盖。前后15诊断样本多数略慢，尚无整体加速/性能不退化
证据，不把winner内部调用减少或压力一次成功等同历史根因修复。详细计数、墙钟、
行数与源码/六件/旧210份见迁移台账v376；下一轮保护216份，完整N01–N12/默认JS/
真实股票和Loom/Lease/OS隔离/总资源长稳/最终用户包仍未验收，不发布或宣称迁移完成。

v375最终状态partial-runtime-failure。final-run2只修过时的span:hover负向样本，
21门17过/4压力失败；CSS模型9/0、投影6/0、Session74/0/24 ignored与新Hover产品/
原生/功能及相邻Application通过，不代表完整JS。四个原预算压力门各0/1，前三路径
观测Collect后超期、产品缺详细阶段，根因未关闭；首轮完整窗口103/27/1 ignored
未重跑。源码/两候选/210份后续保护及全部边界见迁移台账v375，不发布或宣布迁移完成。

v375补author stylesheet的:hover：可信Hover目标及当前连接祖先沿原owner匹配、原
cascade/投影/几何失效接线，合成事件不能授权。通用child/产品、32层边界、真实HWND
像素/命中/leave/隐藏和原压力/相邻门按最终实际结果记入迁移台账v375；开发RED与
synthetic嵌套布局栈失败保留。不扩DOM selector API或静止指针自动重命中，不默认
放行完整JS/真实股票/Loom/Lease/OS隔离/长稳/完整N01–N12与最终用户包。

首轮封存22门18过/4失败，新增Hover子集/产品/原生像素与四压力门通过；CSS模型/
投影/Session受旧span:hover负向样本影响，完整窗口有独立前台获取失败及锁中毒。
精确成绩与两类原因见迁移台账v375；修样本不改产品，完整窗口旧失败不重跑求绿。

v374只修焦点测试的前台前提：每段Application prepare/stale复用原Foreground guard，
发送/交付前后直接断言，不重放请求或修改产品准入。独立进程控制证明本线程parent
持焦不代表真实前台，后台当前epoch请求拒绝且恢复后不重放；同线程前提失败0/1与
未复现旧失败的诊断1/0分别保留。最终相关门/身份见迁移台账v374，不关闭历史未知
根因或把焦点测试闭环等同默认JS、完整N01–N12/业务/资源/用户包验收。

首轮11门6过/5失败已封存；修正只把updated的焦点断言留在实际focus/blur交付边界，
不要求跨TLS批次持续占前台。跨进程负向改为显式ignored但10门新矩阵仍强制独立
运行；默认ignored不计通过，不重跑旧Enter的完整Application组求绿。Foreground
仅加track_caller，首轮完整Application13/12/窗口97/33及最终成绩均见迁移台账v374。

第二候选10门7过/3失败：主focus/blur通过，ABA交付前丢前台；第三候选仅将自有
Child激活改为目标线程一次命令/确认，原Foreground被动guard负责归还。显式负向
仍不得省略，旧完整组失败不因默认ignored或后续单项成功而关闭。

最终final-run3的10相关门全过：显式跨进程1/0、focus窗口2/0/1 ignored、Application
focus3/0、完整默认窗口129/0/1 ignored、UIA/实际产品keyboard及四个资源/启动/worker
门均过。ignored用例另有显式独立门实际通过，非跳过；旧完整Application13/12未重跑，
旧未知根因不关闭。六变更最大321行、严格静态/三次源码/六件/旧192份与身份见迁移
台账v374/最终closeout；下一轮保护198份，相关测试已闭环，完整迁移仍未验收。

v373只修声明式键盘测试的普通消息modifier归属。受控Ctrl的RED证明原夹具会继承
线程快捷键状态；现逐条保存/中和modifier/恢复完整键盘表，并核raw消息仍拒绝
Ctrl/Alt/Win、支持Shift反向及unwind恢复。不改产品准入，不把该确定性夹具缺口
当作v372全部历史失败根因；最终实绩/身份见迁移台账v373，完整迁移仍未验收。

最终8门7过、1失败：fixture7/0、完整窗口129/0、实际产品keyboard1/0及资源/启动/
worker门通过；Application14/11为focus/blur的EDIT焦点断言失败+十项锁中毒，Enter
本次通过不关闭历史根因。仅一测试文件152→296行，严格静态/3,696文件ratchet、
2,544输入/三次源码/六件/旧174份见迁移台账v373/closeout，下一轮保护180份，仍partial。

v372补普通无按键hover到同child包装节点listener/rAF/Canvas的通用接缝：over/out/
enter/leave/move与relatedTarget，客户区DIP/四modifier、buttons0，EDIT焦点不被抢走。
与Canvas Pointer共用原32槽/sequence/FIFO，两个terminal预留不能被其他writer占走；
硬取消保prepared、撤销waiting，Canvas up只续结已有hover。最终实绩/失败/身份见迁移
台账v372，不继承开发或旧版本成绩。尚非完整浏览器Pointer、真实股票/Loom、默认JS、
真实Lease撤销、OS权限隔离或完整N01–N12/用户包验收。

最终34门32过、2失败：Hover HWND6/0、child4/0、独立Application1/0及产品1/0，
完整JS189/0/28 ignored、十四产品worker和三原规模压力门通过。完整Application
13/12为Enter发送前丢前台+十一项锁中毒；完整窗口122/4为四项旧键盘焦点/候选
直接失败，根因未定，不能全部归为前台或用独立Hover成功覆盖。严格静态、35文件/
12新增/最大500行、2,544输入与六件/旧168份见迁移台账v372/最终closeout；状态仍
partial-runtime-failure，下一轮保护174份，不放行默认JS或完整迁移。

v371接当前JS Canvas左键的真实HWND→原FIFO/child→PointerEvent/像素→Application
TLS/Grant/batch闭环，不接普通按钮/原生控件，不增加默认协商或第二套输入owner。
支持捕获down/move/up/cancel与DIP/modifier；terminal共享原32槽/sequence预留，取消
保prepared并撤销waiting，连续双手势/同步capture重入有专门回归。Pointer取消不抑制
旧阈值click，click仍为普通Event，up后重验连接/disabled/computed可见性。完整Pointer
浏览器语义、touch/pen/hover与JS capture API尚缺。最终实绩和身份见迁移台账v371；
本地policy测试不替代真实Lease撤销，默认JS、OS隔离和完整N01–N12验收仍未放行。

最终27封存运行门26过、1失败：Pointer HWND9/0、child5/0、完整窗口120/0、JS184/0及
十三产品worker均过；Application13/11为Enter前台获取失败+十项锁中毒。新增Pointer
未进入体后固定一次独立进程1/0，两轮TLS/Grant/FIFO/同child完成，不覆盖完整组失败。
严格静态、37文件/最大497行、三次源码/六件/旧162份及三压力门见迁移台账v371/closeout。

v370推进真实CSS压力，不增加CSS能力或修改默认协商。原cascade以分层比较和computed
内原地winner更新减少重复rank/对象分配，源序及important/inline/specificity不变。
独立32组Rust oracle、旧算法差分、多节点/多属性/样式更新与原型accessor保护其语义。
原256节点/16规则/250ms、Engine/child/实际产品三条路径分别验收；开发单项通过不
覆盖固定矩阵失败。最终实绩/身份见迁移台账v370，完整迁移和历史未知根因不据此关闭。

最终24运行门23过、1失败：原压力Engine/child/产品各1/0，完整JS177/0及窗口111/0、
startup7/0通过；Application13/10为前台获取失败后九项锁中毒。最终新旧库固定矩阵
均15/15，不能把早期失败全归为已修；三诊断/三次源码/六件/旧156份见迁移台账v370
及final-run closeout，仍为partial-runtime-failure，不作默认JS或完整迁移验收声明。

v369只修测试Foreground guard的清理所有权：三个自有HWND的RED证明无条件恢复会覆盖
后来前台选择，现只在仍持有target或target已毁且前台NULL时归还，不增加获取重试/
等待或产品激活。候选诊断一次1/0未复现旧失败；最终只增强原分支/焦点/capture断言，
不改产品准入。最终完整组/身份见迁移台账v369，不关闭v368候选/前台获取及旧未知根因，
不以测试支架修复替代默认JS、完整迁移/业务/资源长稳或用户验收。

最终12门10过、2失败：Application23/0、values5/0、child3/0/1 ignored、队列两门各1/0、
摘要绑定产品keyboard1/0、startup7/0及资源/worker门通过。foreground专门门1/1，
后来选择回归通过、unwind初始前台获取失败；完整窗口93/18中两项新guard回归均过，
但旧focus stale/ABA前台获取失败+17锁中毒，后台Enter负向未进入体。两上下文不互相
覆盖，历史根因不清零。严格静态/3,671文件ratchet通过，四变更最大192有效行；首轮
Clippy失败保留，最终身份/三次源码/六件/原150份见迁移台账v369及final-import-correction
closeout，保持partial-runtime-failure，不把完整Application本次通过当稳定性或迁移完成。

v368只修Enter Application测试对长期持有前台的错误假设。状态诊断复现同一EDIT仍有
线程焦点但真实前台已离开Surface，原入口正确拒绝keyup；现每段驱动以原Foreground
guard建立/恢复前台，每次发送前核验，不重发失败事件或提高原poll/资源预算。新增
后台EDIT down/up不入队、不在恢复后重放的真实HWND证明。最终完整组/相邻门实绩见
迁移台账v368；不据此关闭所有历史keyup、压力/form/startup根因或完整迁移验收。
开发负向用例1/0，但Application新驱动发送前仍因当前游戏占前台而0/1，保留该失败。
未操作游戏；用户随后允许短暂占用前台，最终恢复完整10门矩阵，真实终态见迁移台账，
不以授权、启动或更准确的前提断言代替完整回归通过。

最终10门8过、2失败，无deferred门：Application13/10为Enter前台获取失败+九项锁中毒；
SurfaceWindow90/19为原点击候选未入队、另一前台获取失败及17项锁中毒。新增后台负向
用例最终取锁即失败，未进入测试体，不能用开发1/0覆盖。child3/0/1 ignored、队列两门
各1/0、摘要绑定产品worker1/0、startup7/0及资源/worker门通过。严格静态/3,670文件
ratchet通过，两测试文件134/329有效行。三次源码/六件/旧144份及身份见迁移台账v368/
final-run closeout，保持partial-runtime-failure，旧压力/form/startup根因不自动清零。

v367沿原256节点/16规则压力优化CSS匹配/层叠/投影，不新增功能或放宽250ms。原无规则
约210ms、仅16条tag规则亦超时，稀疏投影初修仍不足，失败和固定前后样本分开留证。
私有计算样式只遍历已有属性，class用私有编译边界正则，sheet只持一份有序声明；
inline/优先级/原子准入和原native consumer不变。差分/child/实际产品的最终实绩见迁移
台账v367；旧keyup未知根因、默认JS/完整股票/完整迁移/总资源长稳和用户验收不放行。

v367最终24实际运行门18过、6失败，另两诊断exit0不算吞吐通过。完整窗口108/0、
JSON2/0、projection4/0/4 ignored、CSS16/0/5及八非压力产品worker各1/0等通过；完整
Application13/10（keyup直接失败+9锁中毒）、完整JS137/38/26（form Apply期限+37锁中毒）、
startup4/3（原15秒三项超时）及原压力Engine/child/产品三个0/1均保留。固定15样本仅
无规则3/3，其余四类0/3，阶段诊断275.2552ms不能代250ms预算门。严格静态/3,670文件
ratchet通过，12个native变更最大428行，源码/六件/原138份身份见迁移台账v367及最终
closeout，保持partial-runtime-failure，不清除未知根因或把ignored计为通过。

v366补齐真实四列Surface所用author CSS的`:nth-child(4n)`，并提供有界An+B/odd/even/
整数子集；文本不计、隐藏元素计入兄弟位置，移动/移除/重插按原mutation identity
失效。Rust整数oracle、原子替换、同operation原生几何、原型污染及child/实际产品/
HWND像素验证的精确实绩见迁移台账v366。DOM查询位置语法、hover/focus-visible、pointer、
其余股票CSS/DOM、默认JS、真实Loom/股票、完整迁移/总资源/长稳/用户验收仍开放；不以
样式伪类通过声称真实业务已可运行，不扩大预算或覆盖旧未知失败。

v366最终14封存门13通过、1压力失败：nth-child6/0/2 ignored、CSS12/0/1、完整窗口108/0、
JS169/0/22、Grid16/0、Geometry29/0/1、三摘要绑定产品worker各1/0与startup7/0等。
256节点/16规则压力显式运行仍超原250ms，0/1；首轮Application13/10及固定一次Enter
复核1/0保留，未知根因未关闭，最终不同字节不继承首轮Application整组验收。严格静态/
3,664文件ratchet通过，11个native变更最大425有效行；源码/六件/原132份与失败分别
绑定，closeout保持partial-runtime-failure，精确身份见迁移台账v366，不相加重叠集合。

v365仅优化原文本input的值校验分配/扫描，不增API或变更UTF-16/48KiB/CRLF/原子拒绝
规则及原250ms预算。固定前后探针、22个child边界/污染操作与784组Rust oracle差分
见迁移台账v365；完整静态/封存与旧120份分别核验。不是完整键盘/pointer、默认JS、
真实股票/Loom、完整迁移/总资源/长稳/用户验收或历史未知超时根因已清除的声明。

最终43选定封存门通过，含值校验3/0/2 ignored、完整窗口107/0、Application23/0、
JS164/0/20、十四产品worker各1/0和startup7/0。原输入mount固定中位215.077→11.420ms，
ASCII边界写入225.033→4.5207ms，仍受原250ms期限约束。七个native变更最大422行，
四新增最大92，严格静态/3,658文件ratchet通过。源码/六件/旧120份与四份性能摘要、
夹具失败见迁移台账v365；不覆盖v364失败或外推普遍性能、长稳/完整迁移验收。

v364把文本input的真实非IME Enter keydown/keyup接到原EDIT/FIFO/child，提供受限
KeyboardEvent与未取消/非repeat的默认change。默认回到原键盘槽，不越过keyup/后续
输入或click fence；接纳时重验原字段/dirty代次、scope/期限、焦点epoch/策略/前台。
不是完整浏览器同步键盘或form submit，普通字符/Tab取消、更多表单、pointer、默认JS、
真实股票/Loom、完整迁移/资源长稳/用户验收与旧未知根因仍开放。具体实绩/失败和
同源封存见迁移台账v364，证据不覆盖旧114份，不把切片成功当成N01–N12完成。

最终严格静态/3,654文件ratchet通过，38个native变更最大493有效行、九新增最大282。
41封存运行门首轮40通过、1失败：Enter11/0/1 ignored、键盘49/0/1、JS161/0/18、
Application23/0、十三摘要绑定产品worker各1/0及startup7/0等通过。完整窗口首轮104/3
包含一次旧文本夹具mount超时和两项锁中毒；固定六门同字节复核含完整窗口107/0均
通过，不覆盖失败或关闭deadline未知根因。closeout保留partial-runtime-failure，三次
源码/六件/旧114份/诊断及精确成绩见迁移台账v364，不外推稳定性或完整迁移验收。

v363新增文本input的受限 focus()/blur()：同operation有界请求经原child回复，在当前
前台Surface和输入策略下切换真实EDIT，失焦沿原change/FIFO而非手造事件。focus epoch
拒绝在途失焦/回焦ABA，应用同一输入策略在回复交付前收紧。只是异步宿主焦点意图，
不支持activeElement/焦点事件/滚动/options；实绩/失败/身份见迁移台账v363。键盘事件/
默认动作取消、pointer、更多表单、默认JS、股票Loom、完整迁移/资源长稳/用户验收
仍开放，旧未知根因不清零，不以本切片替代N01–N12验收。

最终37封存门通过：焦点9/0/1 ignored、较广焦点52/0/2、JS157/0/17、Application22/0、
原生文本7/0/1、EDIT66/0、number37/0、十二产品worker各1/0及startup7/0。24个native
变更最大499有效行、八新增最大168，3,645文件ratchet/严格静态通过；三次源码/六件
程序/旧108份与失败身份见迁移台账v363/closeout，不相加重叠集合或计未运行ignored。

v362新增文本 input/真实 EDIT input/change 的本地 carrier，沿原草稿/ticket/32槽FIFO/
click fence进入同一JS child，DOM帧接纳后结算本地预留，emit另经原远端authority/
Grant/收据。程序写value不伪造原生事件；前帧不能覆盖后续草稿，IME结束前不派发。
实际回归/身份见迁移台账v362；keydown/默认动作、focus/blur、pointer、更多表单、
默认JS、真实股票/Loom、完整迁移/资源长稳/最终用户验收和旧未知故障仍开放。

最终33选定封存门通过，输入7/0/1 ignored、JS153/0/16、Application19/0、EDIT66/0、
number37/0、十一摘要绑定产品worker门各1/0与startup7/0；不是全库/完整插件验收。
36个native变更最大483有效行、八新增最大175，3,637文件ratchet/严格静态通过。
源码/六程序/旧102份、失败留证与精确边界见迁移台账v362/final-run closeout，不能
把输入基础接入提升为键盘/默认动作/pointer、完整迁移或最终用户验收已通过。

v361把同文档 EventTarget 接入已有真实 native click，而非仅提供合成分派。Node 与
独立 target 共用有界 listener 身份、capture/target/bubble、once/passive、移除快照、
取消/停止及 finally 清理；emit 不改变原 authority/Grant/FIFO。通用夹具贯穿原 child、
隐藏 HWND 与 Application 接缝，实际成绩/身份见迁移台账 v361。不把能注册或合成
keydown/change/pointer* 说成原生键盘/pointer/input 已接入；完整N01–N12、股票/Loom、
总资源/长稳、最终用户手测和旧未知根因仍开放，默认JS保持关闭。

最终全目标严格静态、3,629文件ratchet与29封存门通过：事件10/0/1 ignored、JS149/0/15、
Application18/0、十个摘要绑定产品worker各1/0与startup7/0等精确成绩见迁移台账
v361。19个native变更最大386有效行、八新增最大148，无新例外。三次源码/六件/旧
96份、七文档/组织合同/零相关进程由final-run closeout绑定；UIA夹具、两次同步栈
回归及六个timer模型接线失败均留证，最终成功不外推旧未知故障或尚未完成的输入
carrier、完整迁移/真实插件/长稳/用户验收，默认能力不扩大。

v360将原单标量内容内缩改为明确四边 Insets，CSS padding 1–4 项/side longhand 沿
原 cascade 进入真实原生内容盒，覆盖布局/文字/图片/textarea 与同步几何。图片
content-edge 圆角按相邻边独立求值，不用统一圆角掩盖非对称内缩。验收范围和实际
证据见迁移台账 v360/原生 JS 合同；CSS 相对 padding、margin/单边 border、更多输入
事件、真实股票/Loom和完整 N01–N12/资源长稳/最终手测仍开放，默认JS和旧未知
故障不因新切片改变。

首轮封存26门中25门通过、JS99/42/14 ignored失败：旧双值padding拒绝夹具影响
child/模型两项，空间模型仍期待scalar投影，另39项锁中毒；首轮程序和失败收据
保留。仅修两处测试输入后，final-fixture-correction重新冻结/构建/封存，无重编
26门全部通过：padding43/0/1、JS141/0/14、图片9/0、Grid16/0、textarea4/0、文字33/0、
measure131/0、九产品worker各1/0与startup7/0等精确成绩见迁移台账v360。全目标
严格静态/3,621文件ratchet通过，40个native变更最大455有效行、九新增最大168行。
最终源码/六程序身份与旧90份保护由closeout核验；初次lint及证据脚本失败同样保留，
不以本轮选定green清除旧未知根因、默认能力或完整迁移/真实插件/长稳验收边界。

v359继续 typography 合同，font shorthand 原子展开四个longhand，沿原 cascade 接到
已安装 family 与原生文字 owner；unitless 行高按子字号重新求值，normal 显式重置
祖先行距，不用保存字符串冒充生效。实现前四个真实QuickJS模型测试全部失败，
最终开发/封存成绩见迁移台账v359/收据。精确子集见原生JS合同；默认JS、完整
N01–N12、股票/Loom、资源长稳/最终手测和旧Canvas/启动未知根因均不因此放行。

最终严格静态/3,612文件ratchet及六程序构建通过，16个native变更最大380有效行、
无新例外。21个封存门全部通过，typography8/0/1 ignored、JS136/0/13、text32/0、
measure124/0、八产品worker各1/0、startup7/0等精确成绩和身份见迁移台账v359。
新重入栈在真实Engine确定复现，公开属性转换前移尚不足，投影单调用层合入后
原断言通过；不是放宽原栈/时间或用重测解释旧失败。22条消费者字体值只读探针
不等完整插件运行，旧78份、失败收据及未完成门保留，不把本切片升级为完整迁移。

v358接通font-family的有界已安装字体选择子集，贯穿CSS继承、原生文字和字段；
不是完整逐字形作者字体链、font shorthand/Canvas font或完整真实插件支持。首次
有效请求才查询字体集合，缓存归原Engine且最多64项。具体合同、源码/运行证据见
原生JS合同和迁移台账v358；最终选定矩阵通过，不清除旧Canvas/启动未知失败，默认
JS、完整N01–N12、股票/Loom、长稳/资源和最终用户手测门不变。
首轮19封存门18过，JS90/39/12中child/模型两项因旧夹具仍要求拒绝有效Arial失败，
另37项锁中毒；字体11/0/1、七产品worker和startup7/0通过不覆盖该失败。只修旧
夹具两项拒绝值的独立followup不改生产输入，实际新成绩另见迁移台账v358/收据。
该中间构建未封存，又捕获NBSP原始UTF-8计数缺口；最终final-admission在生产values
计数前先验字符域，并由共用fonts夹具验证原子拒绝。与首轮共三个输入变化，不能
把最终状态说成仅改测试；精确身份和最终成绩见迁移台账v358/最终收据。

final-admission的19个封存门全部通过：font-family11/0/1 ignored、text31/0、measure124/0、
EDIT2/0、Select1/0、JS129/0/12、Canvas14/0/1、geometry29/0/1、七产品worker各1/0、
资源2/0及1/0、startup7/0、worker入口2/0，重叠集合不相加。全目标严格静态及3,605
文件ratchet通过，18个native变更最大377有效行、无新例外。最终源码/EXE摘要与
首轮失败、中间未封存历史及旧72份保护见迁移台账/closeout；此字体切片完成，不
提升完整迁移、真实插件、资源/长稳或最终用户验收状态，也不清除历史未知根因。

v357继续追查Canvas Application的原InvalidData：仅测试child通过原私有回复交付
固定阶段/单调耗时/预算状态，实际产品协议无新字段，不记录脚本或异常文本。
诊断回归与真实Application/封存结果见迁移台账v357；诊断能力
不等于故障修复，也不改变默认JS、完整迁移/股票/Loom及最终手测门。

首轮20门19过，JS124/1/11 ignored的唯一失败为Canvas直接Engine模型初始mount；
Canvas Application集合14/0/1通过，两个失败不作同根归因。只补接原模型apply失败
诊断后，独立新源码/六封存件的16门通过，三个固定完整JS观察各125/0/11、诊断7/0、
Canvas14/0/1、geometry29/0/1、六产品worker及startup7/0。与首轮仅一个测试输入
不同；成功不清除模型/v356 Application或其他旧未知根因，整轮仍partial。最终
身份、旧60封存件和真实消费者CSS/input/listener源码缺口见迁移台账v357/closeout。

v356已把v355重入失败复现到真实Engine，并将cssText/整体style/sheet的String
转换移至公开入口、私有parser之外；不提高原栈或时间预算。通用夹具分层验证
同步读40→主动变更50→最终60、抛错身份与非法解析外层原子性；同child update
验证最终状态保留，dispose验证回收。实际最终成绩见迁移台账v356，不将开发接线
当作全部运行验收，旧未知启动/其他失败及默认JS/完整迁移/最终手测门保持开放。

sheet嵌套解析另由原values压平validate/test冗余层，保原接受/拒绝与规范化；
单移转换后仍失败的中间矩阵保留，不用最终成功改写诊断历史。

v356最终严格静态/3,597文件ratchet及构建通过，七个native变更最大214有效行。
封存geometry29/0/1 ignored、JS118/0/11和六个产品worker各1/0通过，产品几何门含
七个重入模式，startup7/0。但Canvas集合10/4/1的首个Application初始回复失败，
三项锁中毒连带，19门仅18门通过、保持partial。固定新旧ABBA四次隔离成功不清除
原失败或证明无回归；完整身份、未定位边界与其他旧失败见迁移台账v356/closeout。

v355正验证geometry的同operation encoded projection复用；DOM/CSS/Canvas尺寸的
私有mutation token使写入立即失效，连接可度量box getter仍逐次通过原生准入。
21×8的fake-box探针投影168→21、nativeReads保持168，只证明算法冗余下降，
不替代真实布局/点击/期限验证。实际范围/证据见迁移台账v355，旧失败及默认JS、
完整迁移和最终手测门不清零，不把开发接线或局部性能样本当运行验收。

v355最终严格静态/3,595文件ratchet及构建通过，八个native变更最大289有效行，
新测试194行。封存18门中16门通过：原HWND click/resize、真实child/Application
及五产品worker门通过，startup7/0；geometry25/1/1 ignored与JS114/1/11共享一个
新增输入转换重入模型的256KiB栈溢出失败，保持partial。当前实际路径通过不清除
旧未知根因，新重入边界继续定位，完整身份及原失败保留见迁移台账v355/closeout。

v354已补入通用JavaScript timers，沿原同child、授权、单槽、应用deadline和emit
FIFO接入，不复制股票业务或改变Loom。具体子集/明确拒绝见原生JS合同timers节，
实际静态/child/HWND/Application/产品封存成绩见迁移台账v354；开发接线不等验收。
旧启动超时及其他未知根因保持开放，默认JS/完整N01–N12和最终用户手测仍不放行。

第二轮严格静态/构建及timer13/0/1 ignored、JS104/0/11、五个实际产品worker各1/0
通过；旧counter无timer断言已按新能力修正，生产输入未改，首轮失败原件保留。
最终18门仅16门通过，geometry10/5/1（首个HWND click失败，四项锁中毒连带）、
startup6/1钉图超原15秒，整轮partial；成功子集不能清除未知根因或升级完整验收。

v353继续定位启动超时，增加原资源owner的分段计时、跨线程配对ID和单调时间，
并修复诊断管道错误会panic的缺陷。原隐藏验收序列独立成startup_smoke.rs，场景、
断言、原child15秒和默认关闭的诊断不变。初步三场景仍失败，不称启动已修复；
最终验证和证据见迁移台账v353。此前Canvas成果保留，完整迁移/手测门不升级。

最终全目标严格静态、3,587文件行数门及产品/测试构建通过；封存计时5/0及相关
资源门通过，设备零项筛选不计成绩，实际模块另5/0。但startup6/1钉图smoke仍超
原15秒，九个有实际用例的选定门仅八门通过，整轮partial；未知根因与旧失败保留。

v352已接入通用Canvas2D真实像素：原JS绘图进入child WIC/D2D/DirectWrite，父侧
验证当前Canvas身份/尺寸/PNG后沿既有图片呈现owner提交；不改Loom或翻译股票业务。
精确子集、明确拒绝及预算见原生JS合同Canvas2D节，实际开发/封存结果见迁移台账
v352，不继承旧绿灯。第二轮Canvas14/0/1 ignored、JS94/0/10和四个实际产品worker
单项通过；16个选定封存门中startup6/1超原15秒，整轮partial，不能称运行门全绿。
旧canvas拒绝夹具已换video并补正向像素测试，生产输入未变，首轮失败证据保留。
同字节启用已有计时后，无图单跑三次约10.2–10.8秒成功，但完整启动固定三轮
6/1、5/2、4/3均失败，涉及无图/钉图/WIC及不同阶段；根因仍未定位或修复，不以
单跑通过清除失败。补充证据和文档快照见startup-diagnosis/closeout，原收据不改。
v351的真实ResizeObserver→rAF已完成定向门；Canvas子集仍不
等于真实股票/Loom、完整CSS/timer、默认JS或N01–N12验收，旧失败和最终手测保持开放。

v350 补入真实同 child 的 requestAnimationFrame/cancelAnimationFrame/performance.now，
逐 callback microtask checkpoint、同帧 timestamp 和有界下一帧队列。原 UI owner
只在当前策略/唯一 owner/单槽均允许时，以原消息等待 deadline 排程；外借 owner/
应用策略/emit 背压不连续唤醒。精确合同见 [rAF](SURFACE_JAVASCRIPT_NATIVE.md#v350-宿主驱动的-animation-frame)，
最终静态/child/HWND/Application TLS/FIFO/实际产品成绩见迁移台账 v350，不继承早期绿灯。
这是 ResizeObserver→rAF→Canvas 消费链的一个必要入口，不是 Observer、Canvas、timer、
VSync 或真实股票/Loom 支持。默认 JS、原 v348 startup 未知根因、完整 N01–N12 和
最终手测仍开放；没有提交推送或发布。

v350严格静态/编译与源码rAF9/1 ignored、JS112/8通过，实际产品animation worker另1项
通过；整轮不是全绿：SurfaceWindow222/1/11、Application resources27/2/1，封存后
animation4/5/1、Application JS4/8/0且其他JS集合有deadline/geometry失败。UIA/TLS/
child失败根因未定位/未修，SERIAL连带单独记录。startup7/0和部分产品门成功不清除
这些失败或原v348问题；完整矩阵、程序身份与partial closeout见迁移台账v350。

v349 将受限 position/anchors/inset 和容器同轴 visible/hidden 接到既有原生布局/裁剪，
真实像素、命中和 JS 同步 bounds 共用一个几何 owner。直接定位父、每轴锚点/单锚
显式尺寸、paint-tail、非根 relative 纵向百分比与混合 overflow 的拒绝范围见
[空间 CSS 合同](SURFACE_JAVASCRIPT_NATIVE.md#v349-受限定位与容器裁剪)。Application
沿原 TLS/同 child/FIFO 夹具消费定位与裁剪，不改变动作/授权 owner。它不是完整
stacking、滚动、Canvas 或真实股票/Loom 支持；默认 JS/capability 不扩大。
最终本轮证据见迁移台账 v349/closeout；原 v348 启动超时虽在固定三轮完整矩阵未
复现，仍未定位或修复，不能清除旧失败。完整 N01–N12 和最终用户手测保持开放。

本轮最终严格源码/编译和相关回归通过：spatial11/1 ignored、JS103/7、Surface711/17、
SurfaceWindow222/11、Application resources27/1及3,559文件行数门。六程序封存后
spatial/实际产品worker/Application JS10、原预算/资源和startup7/0通过；2,407源码
输入及六程序/旧.339–.348封存产品身份一致，相关进程残留零。集合不相加冒充全库，
ignored产品门另行摘要绑定运行；精确身份、早期失败和旧startup开放问题见台账v349。

v348 补入同步 rect/client/offset 和 DPR，真实 HWND 客户区/DPI 与原生 Host 共用
字体/context、NativeMeasure/PreparedScene。显式 click/update 使用新 viewport，
真实 HWND 对比脚本结果、生产几何、D2D 像素与同几何命中；Application 原 TLS/FIFO
夹具覆盖挂载/两轮 resize→点击→收敛更新，以及接纳前/接纳后暂存 emit 的 resize
拒绝。原 ID/字节/顺序/Grant/唯一 owner 不变。预算和精确语义见
[同步几何合同](SURFACE_JAVASCRIPT_NATIVE.md)，最终结果及失败见迁移台账 v348。
旧 viewport 结果终态关闭，不重跑 listener；不是 ResizeObserver/rAF/Canvas 或自动
resize 恢复。默认 JS/capability、完整股票/Loom/CSS、权限隔离、长稳、用户手测和
N01–N12 仍未放行，目标 active。

v348最终源码/严格静态及相关回归通过：geometry15/1 ignored、JS92/6、Surface701/16、
SurfaceWindow221/11、Application resources27/1和3,552文件行数门。封存后geometry/
实际产品worker/Application JS10及原预算/资源门通过，但startup4/3超15秒，整轮
runtime为partial；后来单跑通过没有定位或修复原失败。源码/程序身份一致、相关
进程残留零，不把开发侧子集通过或重试结果当启动/完整产品验收，详见迁移台账v348。

v347 将受限 CSS Grid 接入 v346 原生 consumer；shorthand 在 cascade 前展开，原
Scene/测量/Presentation/DirectWrite/D2D/命中 owner 不变。通用同 child 夹具涵盖
点击 emit、轨道/显隐更新、真实 HWND 像素与命中、隐藏不占位、失败替换保旧帧。
精确声明、上下文、CSSOM/normal 差异和预算见 [JS Grid](SURFACE_JAVASCRIPT_NATIVE.md)。
最终冻结/封存结果见迁移台账 v347；默认 capability/JS 不扩大，完整浏览器 CSS/
DOM/Canvas、股票/真实 Loom、权限隔离、长稳和 N01–N12 未验收，目标 active。

最终 JS Grid 17/1 ignored、JS 77/5、Surface 690/15、SurfaceWindow 220/11 及相关
授权/Application 回归、严格静态/材料合同/3,543 文件行数门通过。六程序封存后
JS Grid/实际产品 Grid worker、旧 JS/Application/三轮预算/资源/startup 门通过；
2,391 源码输入与程序身份一致，旧 .339–.346 封存产品不变、相关进程残留零。
早期失败保留；结果与非验收边界见迁移台账 v347 和 closeout，不相加冒充全库。

v346 的原生 measured-content Grid 核心已接入同一测量/呈现/命中链，支持固定/auto/
fr/minmax/repeat 轨道、显式与稀疏自动放置、跨格/负线、对齐、单行 baseline 和实际
cell 的文字/图片反馈。真实隐藏 HWND 已定向验证像素/文字/命中、原 click route、
失败替换保留旧帧、隐藏/resize/close；完整冻结回归和封存结果见迁移台账 v346。
合同与明确拒绝见 [SURFACE_GRID.md](SURFACE_GRID.md)，不把 measured-content 当完整
CSS intrinsic 算法。JS `display:grid`、默认 Grid capability、真实股票/Loom 和完整
N01–N12 尚未放行；默认 JS 关闭，目标 active，旧失败不追溯结清。

最终开发门 Grid 25（本切片专属 22）、Surface 676/14 ignored、SurfaceWindow 217/11
ignored、JS 60/4 ignored 和相关授权/Application 回归通过；严格静态/版本/材料合同
及 3,535 文件行数门通过。六程序封存后同字节 Grid、Application/文档/CSS/实际产品
worker/三轮预算/资源/startup 门通过，源码及本轮六程序/旧 .339–.345 封存产品身份
一致，相关进程残留零；没有运行完整桌面库或新在线 OSV，详见迁移台账 v346。

v345 开始实际消费 constructed CSSStyleSheet/adoptedStyleSheets、author cascade 与
动态 Element.style。计算值显式转换到已有原生 consumer，真实 HWND 验证 border-box/
padding/gap/尺寸/grow、继承文字颜色、纯色像素和隐藏后的几何/命中；同 child 的点击
和 update 改 class/data/inline/sheet 后重新呈现。没有为股票编写专用业务 Rust。
精确支持/拒绝与资源门见 [原生 CSS 子集](SURFACE_JAVASCRIPT_NATIVE.md)，最终开发/
封存产品证据见迁移台账 v345。默认 JS 仍关闭；Grid/media/gradient/overflow/position/
inline run/字体/Canvas/observer/timer/rAF 等缺口未补齐，真实股票、默认产品菜单/
Loom 与完整 N01–N12 未验收，不因本子集通过改变目标 active。

最终 v2：开发 JS 60/4 ignored、非 desktop Surface 655/14 ignored、SurfaceWindow
216/11 ignored 与相关授权/Application 回归通过；19 syntax、严格静态/版本/安全、
28 材料合同及 3,525 文件行数门通过。封存后 CSS 12/1 ignored、Application JS 七项、
文档 11/1 ignored、实际产品 CSS/文档/旧 worker、预算三轮及资源/startup 门通过。
2,373 源码输入和六程序/旧 .339–.344 产品身份一致，相关进程残留零；v1 未推广，
早期失败保留，无新在线扫描或完整桌面库/产品真实 Loom 验收，详见迁移台账/closeout。

v344 在同一 QuickJS/原生投影链扩展通用 fragment、移动/DocumentFragment、元数据
反射和简单查询，Application 原点击/FIFO/update 夹具真实消费该模型，不为股票业务
写专用 Rust。live 集合与静态查询分开，循环/深度/标签/属性失败先保原树；私有状态/
数组/intrinsic/JSON 编码不通过插件可变原型泄漏。原引擎和父侧预算不提高。
支持/拒绝合同见 [原生 JS](SURFACE_JAVASCRIPT_NATIVE.md)，精确验证与失败保留见
迁移台账 v344；最终 JS 48/3 ignored、同字节 Application 七项、文档 11/1 ignored，
实际产品文档 worker 显式一项及旧 worker/预算/资源/启动门均通过；六程序与旧
.339–.343 摘要不变、源码身份相同，静态/材料/3,514 文件行数门通过。集合不相加
冒充完整桌面库，v1 旧点击 identity 的测试失败原件保留。默认 JS 仍关闭，class/
ARIA 不等于 CSS/UIA，CSSStyleSheet/Canvas/表单/focus/尺寸/rAF/模块仍未实现，真实
股票和完整产品/Loom/权限沙箱/N01–N12 仍未验收，目标 active。

v343 接入显式受限 JS 的 Application 原生点击→异步 listener→原批次 FIFO→服务端
收敛→同 child update。原事件 ID/字节/顺序、唯一 owner、ACK/确认/接续和授权门不改；
服务端权威 scene 不能直接显示，Pending 收据须等真实 DOM/当前授权复核后才放行。
输入资格由 Application 生产调度开启，不由测试手动启用；原回复事件保留最后唤醒，
不新增线程/等待槽或改变存储格式。合法业务 state 与生命周期字符串独立解析。
当前开发侧切片的验证结果见迁移台账 v343；默认 JS、初始 emit、完整 DOM/CSS/Canvas、
图片/模块/timer/rAF 宿主、权限隔离、真实 Loom JS 和完整 N01–N12 仍未验收，目标 active。
最终 v2：JS 37/2 ignored、非 desktop Surface 633/12 ignored、SurfaceWindow 215/11
ignored、设备会话 134/授权 84、Application resources 24/1 ignored、shutdown 4/Live
预算 3 通过，集合重叠；严格静态/版本/安全及 28 材料合同、3,504 文件行数门通过。
封存后同字节 Application JS 七项、预算三轮各两项、实际产品私有 worker 一项及
资源/启动/裸入口 2/1/7/2 项通过；六程序及旧 .339–.342 摘要不变，源码身份前后一致。
v1 静态/协议/夹具失败保留；无新在线扫描或完整桌面库/产品真实 Loom 验收，不以
当前通过追溯结清旧 UIA/启动/供应链问题，详见迁移台账及本轮 closeout。

v342 接入显式受限 JS 的真实 Application 授权挂载/DOM/生命周期/等待/关闭链，
原 AuthorizedWindow 不复制 owner/FIFO；后台空闲撤权和资源到期自动回收 child。
初始 DOM 与 Active ACK 绑定同步分别约束 active/show，关闭线程未收割不移除窗口
记录；回收先于生命周期分类，不丢最后唤醒。默认报告仍只有 Declarative，JS 的
初始 emit、输入、异步服务端收敛更新、完整 DOM/CSS/Canvas、权限沙箱与真实 Loom
仍待后续实现/验收。详情与开发证据见迁移台账 v342 和
[原生 JavaScript](SURFACE_JAVASCRIPT_NATIVE.md)，完整 N01–N12 目标仍 active。

最终 v3：JS 31/2 ignored、非 desktop Surface 629/12 ignored、SurfaceWindow 215/11
ignored、设备会话 132、授权 84、Application resources 21/1 ignored、shutdown 4 和
Live 预算 3 通过，集合有重叠。严格静态/版本/安全及材料合同、3,496 文件行数门通过，
十新文件均不超过 236 有效行。封存后 Application JS 四项、预算固定三轮各两项、
实际产品私有 worker 一项、资源/启动/裸入口 2/1/7/2 项通过；六程序及旧 .339–.341
产品摘要不变。v2 源码变更导致的身份失败保留，仅推广 v3 冻结/构建后/运行后同一
源码集；无新在线扫描、完整桌面库或产品真实 Loom 验收，切片仍 partial。

v341 补齐原设备 Grant 下的脚本入口 HTTPS/租约/摘要获取、后台真实 child mount、
同 Grant Ready 移交及每操作的版本/生命周期/撤权复核。图片门不放宽，空闲撤权仍需
未来应用 owner 等待句柄并 refresh；尚无默认 Application/JS/FIFO/真实 Loom 链。
首轮旧 emit 预算失败已保留并经隔离阶段诊断，优化 UTF-8 计数，不放宽任何预算。
完整边界见 [原生 JavaScript](SURFACE_JAVASCRIPT_NATIVE.md)，开发验证记录见迁移台账
v341；完整迁移目标保持 active，旧 .339/完整桌面/资源长稳/供应链门不追溯放行。
最终定向为 JS 22/2 ignored、非 desktop Surface 624/12 ignored、SurfaceWindow
215/11 ignored、设备会话 127、授权 84、Application attach/image 2；严格静态/版本/
安全合同、28 项候选材料及 3,486 文件行数门通过。五新文件均不超过 211 有效行。
这些是开发前置链和有重叠的过滤集合，不是默认产品集成或完整桌面库验收。
封存实际产品的私有 worker 门 1、预算固定三轮各 2、资源/启动/裸入口 2/1/7/2
passed；六程序及其目标/副本摘要不变。完整产品 Application JavaScript 仍未接线，
不能以这一轮通过结清旧 UIA/前台/启动间歇失败、完整性能或最终手测。

v340 开始执行真实 JavaScript，而不是仅增加拒绝门：独立持久 child 保留闭包，
文本/按钮 DOM 投影复用原 HWND/DirectWrite/D2D，原生命中/候选返回 JS listener，
父侧按当前权威 scene 校验 emit 并填写身份。详见
[原生 JS 子集](SURFACE_JAVASCRIPT_NATIVE.md)。当前只供显式开发 API/私有 worker，
不广告完整 `surface.javascript.v1`，默认 attach 不启用。真实股票需要的 HTML/
CSS/Canvas/表单/rAF、Application/FIFO/真实服务及权限沙箱仍是缺口。五锁 OSV 首轮
发现后，仅将 native TLS 两包升级至 rustls-webpki 0.103.16 / webpki-roots 1.0.11；
复扫 native 无返回匹配，整门仍因其他锁 24 IDs 失败，不加忽略；许可证通过不等于发行。
旧 .339 产品菜单前台准入及历史失败不追溯结清，完整迁移仍未完成。

本轮最终 JS 定向 12/2 ignored、非 desktop Surface 614/12 ignored、SurfaceWindow
215/11 ignored 及 TLS/设备/媒体/bridge 回归通过；不是完整桌面库。封存实际产品
私有 worker 的显式门另 1 passed，资源/启动/裸入口正式门 2/1/7/2 passed，摘要不变。
Clippy/check/fmt/版本/安全合同、28 项候选材料合同及 3,481 文件行数门通过；原材料
缺失失败经精确原文补充修复，不放宽缺失检查或法律批准。默认 JS 和真实服务接线
仍未完成，完整目标 active；详见迁移台账 v340 收尾及原生 JS owner 文档。

v0.2.33.339 续进：同源码实际产品已重建并按 SHA-256 封存，资源生成 2、内嵌资源 1、
正式 startup 7 项通过，未重编重试或放宽每 child 15 秒期限。产品菜单真实 Loom 链
仍 partial：四轮夹具均未到业务请求；显式 Win32 桌面/Job 修正了 Python lpDesktop
未传入的夹具问题，但等可见后正常前台激活在当前及私有 desktop 都未成功。没有
绕过生产菜单 guard、切换 input desktop 或全局输入，不将库级通过当产品入口通过。
本续进未改产品源码/依赖/版本，证据在 linshi/otter-native-v339-product-loom；旧 UIA/
启动/证据链问题与完整迁移门仍开放，未组包或推送。详情见迁移台账同名续进节。

v0.2.33.339：真实隔离内部 Loom 经显式 loopback TLS 终结，已完成新 Ed25519 配对/
精确设备批准/真实 session，以及生产 Application 的 attach→active ACK→授权 show→
原生 EDIT/两个 COMBOBOX/两次绘制→关闭→inactive ACK。三个业务请求均为 Device
授权、不同 nonce、TLS 1.3；不注入 token/Outcome/ACK，不发全局输入或执行股票动作。
这是 library-test EXE，不是产品菜单入口、daemon 原生 TLS、用户在线 Loom 或 JavaScript。
真实门 1 passed，Application resources 17/1 ignored、非 desktop Surface 606/10 ignored、
设备授权 84、Surface 启动 2 passed，严格静态/版本/3467 文件行数门通过；新测试 199、
接线文件 239 有效行，生产行为/依赖/默认能力不变。编译/普通路径/五秒等待夹具错误与
误用版本命令均保留；真实 attach worker 5.722 秒不结清旧性能问题。证据在
linshi/otter-native-v339-real-loom/run-3；未重编产品 EXE、组包或推送。完整迁移、旧
UIA/启动/证据链 partial、资源长稳/供应链与最终手测仍开放，SDK/安装/分发批准不扩大。

v0.2.33.338：修复真实 Loom 声明式快照中 props/layout/style/accessibility 的显式
null 被 Map 反序列化拒绝；只接受 missing/null/object，不放宽其他类型、events/
children 或任何既有限制。五项新回归先红后绿；Scene 11 项、非 desktop Surface
606 passed / 10 ignored、Application attach/image 两项及严格静态/版本/行数门通过。
真实股票 Art fallback 快照的 15 节点、隐藏 HWND 两次绘制、中文标题、未绑定时输入
关闭/无编辑子控件和关闭后句柄撤销已由公共库外部驱动验证；原始 null 未被删除。
驱动自身的 Win32 返回值/未绑定控件假设错误及失败程序保留，修夹具不改权限门。
这是 2026-10-18 内部 daemon 的隔离管理员 HTTP 协商，不是设备 HTTPS/Application
实际 daemon 链、股票动作、JavaScript 或产品 EXE 验收；本轮没有重编产品 EXE。
恢复时完整库 3223 passed / 1 failed / 74 ignored 的 Select/UIA 失败未结清；一次
追踪只证实自有前台收到 Enter 后收起，输入来源未证，不归咎外部应用或追加重试。
完整迁移、旧启动/证据链异常、资源长稳/供应链及最终手测仍开放；SDK/安装/默认能力
与发行批准不变，无组包、发布或推送。证据在 linshi/otter-native-v338-regression。

v0.2.33.337：只将隐藏 pinned 工具面板的子 BUTTON 延至首次使用；panel HWND、字体、
图标、截图保护/registry 和 owner 立即存在，普通隐藏工作区仍提前准备。所有可见
Application 窗口在事务提交前准备完成；Live freeze 和恢复交换的失败边界不后移。
新 content owner 防重入、失败回滚/重试、首次缓存/样式/忙碌门同步和展示前准备，
不新建线程/队列/轮询或改变系统 BUTTON。隐藏钉图首次使用前的 UIA 子树不再含 BUTTON，
不宣称隐藏枚举兼容或完整可访问性验收。七项真实 HWND 新回归、面板初轮 28 项和 composition
102 passed / 4 ignored 通过；原工具/截图断言未删，旧超时不追溯放行。
证据在 linshi/otter-native-v337-regression；完整启动矩阵与最终产物另行绑定。
第七项真实 ShowWindow 已通过失败保持隐藏/重试、caption/disabled 和重复显示/释放；
不把消息模拟当作真实展示，也不将正常展示证明扩展到所有模型重入/UIA/TAB 时序。
完整 N01–N12、桌面/UIA、4K/内存/长稳、用户 Loom/JavaScript/供应链/最终手测仍开放，
无默认能力、依赖、SDK/安装/发行批准变更，不组包、发布、替换正式程序或提交推送。

v337 最终面板 29 项、composition 102/4 ignored、Shader 65/15 ignored 和严格静态门
通过；固定 Shader 正向 3/3、六套 EXE 集成 26/26（startup 7/7）及拒绝 3/3，九摘要
不变。但追加诊断后原门禁索引及四份诊断材料出现全零/NUL 尾段，原因未证、原件保留，
不能把剩余日志提升为完整原始证据链；本轮保持 partial，独立现态复核位于
linshi/otter-native-v337-recovery。不补造索引/第四轮结果，不追加正向测试掩盖异常。

v0.2.33.336（v336）：默认钉图同 UI 线程共享健康 D3D11 设备，缓存只有 Weak；最后
surface 释放后不保留设备租约，重开重新创建。每窗交换链/D2D context/合成树与
透明度仍独立，Live 显式捕获设备不进默认池。真实身份/像素/十钉图释放四项先红
后绿，五项缓存/回退/线程门通过；不是真实 GPU reset、4K 十钉图或 RAM 节省实测。
composition 102 passed / 4 ignored、Shader 65 passed / 15 ignored、面板 22 项与严格
Clippy/check/fmt/版本/3,464 文件行数通过。首轮六套 EXE 集成 25 passed / 1 failed，
普通完整 smoke 仍在原 15 秒超时；最终摘要绑定的运行矩阵另存，不覆盖失败，本轮
partial。证据在 linshi/otter-native-v336-regression；全迁移、旧 UIA/启动、资源长稳、
真实用户 Loom/JavaScript/供应链/最终手测不放行，无默认能力/安装或发行变更。
最终同字节 Shader 固定三轮 2 passed / 1 failed，六套无编译复验 24 passed / 2 failed
（普通和带图片完整 smoke 超时），拒绝三项通过。九程序/副本摘要和最终 2,312
输入不变，五文档 UTF-8/本地链接及 diff check 通过；不追加重试或排除本轮回归风险。

v0.2.33.335（v335）：工具面板按实际几何直接创建按钮，后续只定位真正变化的 HWND；
保留最终校正、原父窗口/字体/绘制/命令/滚动/DPI owner，不缓存几何或裁掉覆盖。
三项真实 HWND 新回归先红后绿，工具面板相关 22 项、Shader 65 passed / 15 ignored
及严格 Clippy/all-target check/fmt 通过。独立计时只用作热点诊断，不是稳定速度保证。
固定三轮无计时诊断 Shader 正向门 2 passed / 1 failed，第 2 轮仍在原 15 秒超时；
两轮 Shader 提交已完成，但完整门不能通过。额外 Cargo build 改写 EXE 的身份断言
失败另存，新字节先封存再单独验证，不把旧结果套到新 EXE，也不覆盖三轮中的失败。
当前仍 partial；详情见 [本地 Shader 候选](SHADER_LOCAL_CANDIDATE.md)，证据在
linshi/otter-native-v335-regression。旧 v334/UIA/启动失败、完整桌面/JavaScript/
供应链/实际用户 Loom/长稳和最终手测不追溯放行；无默认能力、许可、安装或发行变更。

v0.2.33.334：产品启动已接入显式固定 Shader 候选，默认仍关闭。候选路径/祖先
ACL/两文件摘要与租约、child 重验和普通/截图/会话接线见
[本地 Shader 候选](SHADER_LOCAL_CANDIDATE.md)。实际通过运行确认认证目录、首次
取码、两真实 child、正式 PNG/像素、缓存无第三次连接、父侧未加载及退出锁释放。
但最终默认计时正向门在原 15 秒超时；中间复验/最终一次计时诊断通过不替代失败，
本轮 partial。最终 Shader 65 passed / 15 ignored，参数/拒绝各三项、严格 Clippy/
check/fmt/版本/3,459 文件行数及摘要绑定六套 EXE 集成 26 项通过，源码/八程序
摘要不变；八新文件 53–178 行、修改 Rust 最大 498，无新例外。九文档 UTF-8/
本地链接通过。Public/linshi 祖先不可信时拒绝，私有 HOME 新夹具保留不删。
不改变材料 false 批准或默认安装，不访问用户 Loom，不替代完整桌面/JavaScript/
供应链/长稳/最终手测。证据在 linshi/otter-native-v334-regression，未组包或推送。

v0.2.33.333：新增本地 ANGLE 材料 create/独立 verify，输入须提供独立审阅的 stage/
build 摘要，输出只含白名单两 DLL、绑定证据/源码、原字节声明及材料文件 SBOM。
独立 verify 不依赖原 stage/VS/SDK、不执行快照源码，核对外部预期 manifest 摘要、
精确文件/目录、写入/核验预算及 false 批准边界；不是法律审核或原始构建执行证明。
真实候选 159 清单文件/109 声明，独立核验和官方 CycloneDX/SPDX schema/format
通过；外部公共库驱动直接消费材料 payload，硬件/WARP 六 contexts、18 像素对照、
12 成功 child/两无效 GLSL 拒绝通过，零差异，child 父进程不加载 ANGLE。
只读 Ninja compdb 展开两响应不等于最终 CRT/成员闭包；完整 notice/source/tool/
法律、可信安装、产品 EXE/Loom/完整桌面与 N01–N12/JavaScript/长稳/最终手测仍缺。
原生输入/旧 EXE/默认能力不变，builder 未改，DLL 未重编，旧 UIA/启动失败不追溯
放行；未分发 SDK、组包产品或提交推送。详情见 [ANGLE 工具链](ANGLE_TOOLCHAIN.md)，
证据位于 linshi/otter-native-v333-regression。
相关 105 Python/9 Node 合同、fmt/版本/3,451 文件行数和 diff check 通过；九新文件
30–120 有效行，无新例外。34 Python AST/七文档 UTF-8/本地链接通过，原生 1,767
项输入及旧 EXE 不变、相关进程零残留；完整桌面库未重跑。

v0.2.33.332：固定 profile 已有显式 `build_fixed.py` GN/Ninja 构建入口，要求独立审阅的
外部 stage SHA-256 和全新 stage/output。输入集合/环境与进程树分责，Job 先挂入后执行、
精确继承句柄、异常/超时/剩余子进程有界回收，不是权限沙箱。PE 门补齐必需导出有效
ordinal/EAT/可执行地址与普通/delay-import 静态 CRT；可选数据导出不被误拒绝。只在
授权隔离范围使用 SDK，不系统安装/分发 SDK、不把构建缓存的 SDK d3dcompiler 加入
payload，也不默认启用新 DLL。
构建回执与运行、可信安装、正式发布明确分开，详见 [ANGLE_TOOLCHAIN.md](ANGLE_TOOLCHAIN.md)。

最终源码的新树实际暂存 12,569 文件，GN 513 targets/135 files、504 步完整 DLL
编译（558.765 秒）/无工作/零编译诊断通过；加固 PE/静态 CRT 与新夹具的硬件/WARP
六 context/18 像素对照、12 成功 child/两无效 GLSL 拒绝通过，像素零差异，child
父进程不加载 ANGLE。81 合同、fmt/版本/3,442 文件行数门通过，新八文件 74–167
有效行。旧弱 PE 门构建主动中止并保留，未混用其回执。原生 1,767 项输入/旧产品
EXE 不变，没有新产品 EXE/实际 Loom/完整桌面回归；79 个 gitlink 中 66 个未提供，
完整默认源码/工具/许可、可信安装、N01–N12/JavaScript/长稳/最终手测和旧 UIA/
启动失败边界保留；默认安装/capability 不变，无组包或提交推送。证据位于
linshi/otter-native-v332-regression 的 final-run 及顶层汇总回执。

v0.2.33.331：固定 profile 新增仓库本地 Git/归档暂存入口，复用既有 adapter/profile；
不下载、不运行 hooks/构建工具或默认激活 DLL。最终加固树 12,569 个构建文件与首轮
逐项相同；79 个 gitlink 中 66 个仍未提供，不声称完整默认源码闭包。最终树真实
GN 513 targets/135 files、504 命令计划与首轮原始字节一致；完整两 DLL 编译在首轮
树实际通过（857.063 秒）并无工作，不称最终树重编译。新 DLL x64/52 必需导出/
无动态 CRT 及完整日志诊断门通过，显式公共库外部驱动硬件/WARP 六 context/18
像素对照、12 成功 child/两无效 GLSL 拒绝通过，零差异、child 父进程不加载 ANGLE。
54 合同、fmt/版本/3,434 文件行数门通过；新八文件 23–132 有效行。不是新产品
EXE/Loom/完整 UI 或可复现构建，完整工具/许可闭包、编译执行 owner、可信安装、
N01–N12/JavaScript/长稳/最终手测仍开放。Rust/用户程序/默认安装与 capability、
旧失败边界不变，未组包或提交推送；入口见 [ANGLE_TOOLCHAIN.md](ANGLE_TOOLCHAIN.md)。

v0.2.33.330：新增固定 Clang/lld + MSVC STL 的 x64 D3D11 Release 静态 CRT
profile 准备器，复用既有 adapter；只准备外部 overlay，不执行工具或暂存完整源码。
新实验树的真实 GN 513 targets/135 files、504 命令的完整两 DLL 构建通过，日志无
warning/error 诊断。仓库输出已实际消费，Ninja 原生 GN 自动再生通过，toolchain
前后字节一致且无 prefix patch；C++/资源依赖 5/24/24，三项项目头失效/恢复通过。
Clang 排除 system headers，不把 5 当成 MSVC 121 的退化或完整 SDK 变化自动失效证明。
新 DLL x64/52 必需导出/无动态 MSVC CRT 检查通过；显式夹具复用已绑定公共库
外部驱动，硬件/WARP 六 context、18 像素对照、12 成功 child/两无效 GLSL 拒绝
通过，零像素差异、child 父进程不加载 ANGLE。不是新产品 EXE/实际 Loom/完整 UI
验收；完整 staging、工具/许可闭包与可信安装仍缺，完整 N01–N12/JavaScript/
资源长稳/最终手测开放。原生 Rust、用户程序与默认安装/capability 不变；旧间歇
失败保留，不组包、发布或推送。证据与范围见 [ANGLE_TOOLCHAIN.md](ANGLE_TOOLCHAIN.md)。
构建合同 35 passed / 0 failed / 0 skipped，fmt/版本/全库 3,426 文件行数门通过，
无新增例外；没有重跑完整桌面库或将旧失败视为已修复。

v0.2.33.329：用户授权范围内的隔离 SDK layout 与真实 x86/x64 编译/链接/rc/mt
探针通过，不安装系统 SDK 或默认激活 DLL。实际 GN 空位置参数失败已在
prepare_toolchain 以精确无路径 marker 修复；两项新增回归先红后绿，adapter
25 passed / 0 failed / 0 skipped（含显式 VSLANG 请求合同）。实验树真实 GN
111 targets / 102 files、完整 EGL/GLESv2 计划的 402 条 Ninja dry-run 命令，以及
angle_common/translator/libGLESv2_static/libEGL_static 目标实际编译通过，不称 archive
自包含或 warning-free；zlib 有 D9002。中文 MSVC 的初次头依赖为 0，实验 post-GN
prefix 适配后为 121，真实头文件时间戳失效/恢复门通过；此接缝仍待可复用 owner。
明确的 x64 D3D11 Release /MT profile 不冒充全部上游默认后端；实验
接缝与原始源码分开，详细来源/许可/环境见 [ANGLE_TOOLCHAIN.md](ANGLE_TOOLCHAIN.md)。
后续固定 Clang/Chromium rc 已完成官方对象获取与安全展开，完整两 DLL 实际构建
通过（482.688 秒），x64/52 个所需导出/无动态 MSVC CRT 检查通过；C++/资源依赖
121/24/24 及三项失效/恢复门通过，44 条 D9002 保留。新鲜公共库外部驱动的硬件/
WARP 六 context、18 次像素对照、12 成功 child/两无效 GLSL child 拒绝通过，实际
零像素差异，父进程不加载 ANGLE。首次直接加载的立即卸载断言失败保留，已查明
上游 lazy loader 会保留 GLES 模块引用；不改隔离策略。这不是实际产品 EXE、
可复用仓库 staging/GN 再生、完整工具/许可闭包、可信安装或实际服务验收；
完整 N01–N12/JavaScript/长稳/最终手测继续未完成。
Rust/用户程序未改，不重标旧 EXE；默认 capability 和旧失败边界不变，未发布或推送。

v0.2.33.328：新增 [ANGLE 显式工具链 adapter](ANGLE_TOOLCHAIN.md)，接到固定上游
setup_toolchain 的数据消费边界；显式清单失败即拒绝，不回退系统 SDK，清除空 PATH
段并绑定独立导入依赖。合成数据/上游 main/CLI/隔离 Python 进程的 22 项合同通过。
这不是实际 SDK/GN/DLL 构建或安装证明，外部输出尚未集成完整 GN 暂存树，关键文件
摘要不是完整供应链/ACL 保证。证据位于 linshi/otter-native-v328-regression。
Rust/用户程序未改，未重跑原生运行门或重标旧 EXE；完整 N01–N12/JavaScript/
资源长稳/最终手测、默认关闭 capability、旧失败及未发布边界保持不变。

v0.2.33.327：两条 Shader 配置快照入口提前拒绝与强制 mode/output_mode 重名的
真实图片端口，未声明同名参数也拒绝；普通 Art 图片名/图片优先规则不变，不丢图、
改名或取消强制模式。隐藏应用回归确认配置错误先于 worker/请求替换，旧令牌、
preview 与正式输出保留；不改变原显式取消语义。新测试先红后绿；协议/配置 13
passed、Shader 60 passed / 15 ignored，硬件/WARP 平台 16 次真实 child（8 次
认证取码、8 次缓存无新连接）通过。最终 EXE 先绑定摘要再直跑六套集成 26 项通过，
原期限不变；Clippy/fmt/3417 文件行数/diff check 通过，修改 Rust 均不超过 500
有效行，源码/EXE 不变。证据位于 linshi/otter-native-v327-regression，定向门有
交集，不代表完整库。没有完整桌面库复验或旧间歇失败修复声明；默认安装/capability、
正式供应链、实际 Loom、完整迁移/JavaScript/长稳和最终手测边界不变，未组包或推送。

v0.2.33.326：新增默认关闭的匿名文件事务诊断，不改变完整基线校验、取消、同步、
无覆盖发布、恢复和清理。一次 hidden smoke 11.547 秒正常退出，231 个操作完整配对，
合计 183.739 ms；这是诊断而非性能修复或旧超时根因证明。文件事务/图片保存/实际
命名保存调用方为 29/5/19 passed；初次 settings-update 的 0 tests 不计覆盖。
正式资源/EXE/会话/设置恢复/设置启动/startup 2/1/9/3/4/7 passed，期限未改。
Cargo 测试重编译造成原 EXE 身份检查失败，已保留；最终摘要另绑定无编译复验同六套
26 项通过且 EXE 不变，源码前后相同。Clippy/fmt/3416 文件行数/diff check 通过。
证据位于 linshi/otter-native-v326-regression。没有完整库复验，不将本轮成功追溯为
v325 超时或 v324 UIA 问题已修复；默认 capability、正式供应链、实际服务、完整迁移/
JavaScript/长稳/最终手测边界不变，未组包、替换程序或提交推送。

v0.2.33.325：可信纯 Shader 已接入“首次认证取码并正式提交后缓存原配方，后续
重新绑定当前参数/图片并本地渲染”的应用链。命中不连接 Loom，不伪造远端交付或
租约；响应默认 uniform 和纹理/LUT 保留，参数覆盖不污染缓存。UI owner 的 LRU
限 16 项/64 MiB payload，按 Unit/Selection/Art/catalog/参考上下文隔离，运行
中的 Recipe Arc 继续持有字节租约。参考图启发式/端口筛选对齐旧版；摘要仅在原
输入 worker 计算，缺失上下文不误用独立配方，原来源/配置/拓扑/取消提交门不变。
本地 worker 与任务两槽对齐，真实 child 仍单容量、等待仍计入原期限。
最终 Shader 定向 57 passed / 15 ignored，应用 16 次真实 child 中 8 次认证取码、
8 次缓存无新连接；双缓存任务四场景/六次成功 child 和容量释放前取消回收通过。
输入 worker 15 项、严格 Clippy、fmt、3415 文件行数及 diff check 通过，修改
Rust 模块均不超过 500 有效行；原生源码前后一致，最终 EXE 摘要另绑定六产品/
后端、两本地平台及两种私有入口拒绝。资源/EXE/会话启动 2/1/9 passed，但
startup 为 5 passed / 2 failed：隐藏窗口与透明钉图各在原 15 秒超时，根因未确证，
不放宽期限。两项各一次阶段计时隔离诊断仍超时；隐藏窗口 application 阶段在
12766 ms 成功结束并已打印基础绘制/重建标记，透明钉图最后计时为 groups begin。
阶段时长变化，未确认单一阻塞点或与缓存/v324 前台失败的因果关系。
回执位于 linshi/otter-native-v325-regression，仅为 partial，首轮
Clippy 的大枚举失败及修正、原始启动失败日志均保留；不能作为完整启动放行。
完整桌面库本轮未重跑，v324 的 UIA/外部前台失败不视为已修复；不操作用户应用。
配方复用不等 GPU 程序/常驻 renderer 或完整连续拖动预览。默认安装/capability
关闭，正式供应链、实际服务、完整 N01–N12/JavaScript/长稳与最终手测继续未完成；
未组包、替换用户程序或提交推送。

v0.2.33.324（定向验证通过，完整桌面回归未通过）：公共字符串逐字引用当前 workspace Unit 的图片来源已
接入原准备链，包含历史未声明参数；Art 仅取正式 output，Sticker/Live 复用原图
来源，未知字符串/路径/URL 不触发读取。图片候选与远端配置/Runtime 共用同轮
resolved Params；真实同名 connected 图片优先，遮蔽的 Unit 不进入最终来源依赖。
实际引用及其上游保留 Node/Output/Live、参数与拓扑凭证；32 输入/31 辅助纹理和
原预算不变。新增应用来源/过期/正式图拒绝反例，真实平台门增加 Unit 菜单与参数
执行。对照本地 Loom 源码修正 wire 同名图片输入/参数碰撞，三种配置接缝均省略
同名标量，Runtime 不丢失原 resolved 绑定；inputs/params/强制模式名不提升 Unit
纹理。附加输入的 framework 分区及 data URL 处理归实际 Art，不据静态透传声称
实际服务业务兼容。通用 API 的显式模式名图片端口仍有更广的待验合同边界。
当前 Shader 定向 50 passed / 14 ignored，应用平台 1 passed（16 次真实 child），
Clippy、资源/EXE/会话启动/startup 2/1/9/7、fmt、3411 文件行数与 diff check 通过；
修改 Rust 模块均不超过 500 有效行，原生源码前后相同。最终 EXE 摘要已重新绑定，
六项产品/后端、两项本地平台及两种私有入口拒绝复验通过，无本次原生进程/临时
文件残留，Node 编译缓存保留。完整库两次分别 3181 passed / 5 failed / 73 ignored
和 3185 passed / 1 failed / 73 ignored，未通过：首轮 wire 旧期望已修正，另四项
Surface 失败虽隔离重验通过但根因未确证；第二轮 UIA select 隔离仍失败并记录外部
应用抢占前台，未操纵该应用。完整库间生产源码未变，最终测试类型别名整理后
未重跑完整库，不以定向成绩替代。回执为 linshi/otter-native-v324-regression 下
verification-partial.json / closeout-partial.json。未开放默认安装或 capability，
实际服务/实时缓存/完整迁移和最终验收仍未完成，未组包或提交推送。

v0.2.33.323：已声明公共 image_link/file 参数与同名 PreparedBatch 图片输入的本地
Shader 纹理桥接已实现，连接图优先于手动图与响应初值；主图/reference/secret/
内部/禁用字段不扩大准入，数量/预算/取消和来源/拓扑凭证复用原 owner。新增像素
回归发现并修复内联 PNG 经 PBGRA 重编码的低 alpha/透明隐藏 RGB 损失；PNG 在
既有 ImageValue/Formal 链保留原字节，预算同时计入像素和 PNG，其他格式不变。
新增应用来源/断线拒绝及硬件/WARP 图纹理门；完整库复测 3181 passed / 73 ignored /
0 failed，普通 Shader 45 passed / 14 ignored，应用平台门 12 次真实 child 通过。
Clippy、资源/EXE/会话启动/startup 2/1/9/7、fmt、3408 文件行数与 diff check 通过；
源码前后一致，最终 EXE 已另绑定摘要重验，回执位于 linshi/otter-native-v323-regression。
首轮完整库的 6 项桌面/UIA 失败与原始日志保留；同源码隔离及完整重验通过，不将
复测成功声称为 UIA 间歇断言已修复。
这不是任意 Unit 字符串/未声明图片参数、实时缓存或已丢弃源 PNG 的无损重建。
固定 ANGLE/depot_tools/build/buildtools/jsoncpp 源码已获取并校验官方 commit/tree；
CRLF 获取失败保留并已在新 LF checkout 修复，bootstrap/递归 DEPS 已静态审查。
父 DEPS 指定的 jsoncpp 实现源码另完成官方身份/clean 核验；SDK common/x64 的
10.0.26100.1 固定 NuGet 包已下载并通过完整签名验证，清单含所需 26100 头文件/库。
未执行包内工具、接受新增许可或安装 SDK；vcvarsall 仍重新走本机注册表 SDK，
隔离环境 adapter 尚未验证。无上游脚本/CIPD 或新 ANGLE DLL 执行，完整依赖闭包、
工具实例、SDK 构建适配与正式供应链仍待闭环。
默认 Shader 安装和 capability 继续关闭，实际 Loom、完整 N01–N12/JavaScript、
长稳与最终人工验收未完成；最新实际 ZIP 仍为 v305，未组包/提交/推送。

v0.2.33.322：旧 Shader 的最后自身 data.src 回退已接入原图输入准备链；只有可信
纯 Shader 且没有可用声明/标准主输入时才启用，连接与内联输入优先。自身底图只取
editor raster base，不从 display/预览推断；preview-only 会话恢复即使 Canvas.image
有图也不能作为执行来源。不二次烘焙自身标注/裁剪，真实文档凭证、草稿/接管和
来源新鲜度门保留。普通 Art 不获得回退，缺失其它图片继续阻止自动触发。
原 PNG worker/PreparedBatch/统一执行和提交 owner 不变。真实硬件/WARP 菜单/
参数平台门扩展连接优先和自身底图两组（八次真实 child）。完整库 3177 passed /
73 ignored / 0 failed，Clippy、资源/EXE/会话启动/startup 2/1/9/7、fmt、3407 文件
行数与 diff check 通过，源码前后一致，最终 EXE 另绑定摘要复验；证据位于
linshi/otter-native-v322-regression。自身底图复用既有 PBGRA→PNG，不据不透明样例
声称源 PNG 原始 straight RGB/透明隐藏色及 profile/方向保真。正式供应链/安装、实际服务、通用图片/Unit
参数、缓存实时重渲染及完整 N01–N12/JavaScript/长稳/手测继续待完成；默认安装
和 capability 仍关闭，未组包、替换程序或提交推送。

v0.2.33.321：菜单与最终参数触发已复用原输入准备、统一 Tasks::start/Job、远端
认证取码和应用正式提交链。同一次 resolved Params 同时冻结 Configuration 与本地
Runtime，连线数值夹取、布尔值和 PNG 参数覆盖在联网前确定；Pending.shader 经
Execution.shader 单次移交，不重读 UI，不增加执行槽或网络/取消/释放 owner。
主图按能力声明和旧 input/input_image/image 回退从同份 PreparedBatch 中选择，
reference 不自动充当 input。秘密、内部字段和 reference 不成为 Shader 参数绑定。
普通门验证未安装、参数自动执行关闭及配置变更在联网前拒绝；真实 TLS fixture
使用正式 input/reference，硬件/WARP 各验证一次菜单和一次参数触发、真实 child、
PNG/显示精确提交。测试 EXE/隐藏 HWND 不冒充产品入口、完整阻塞 GetMessage 循环
或实际 Loom 服务验收。应用默认 Installation=None，capability 仍关闭；可信安装
与供应链、通用图片/Unit/graph-image 参数、自身图 fallback、缓存式实时重渲染、
完整 N01–N12/JavaScript/资源长稳和最终手测继续待完成。完整库 3174 passed /
73 ignored / 0 failed（213.16 秒），应用菜单/参数平台 1 passed（四次真实 child）；
Clippy、资源/EXE/会话启动/startup 2/1/9/7、fmt、3406 文件行数和 diff check 通过。
前后原生源码一致，最终 EXE 另绑定摘要复验；证据位于
linshi/otter-native-v321-regression，未组包、替换程序或提交推送。

v0.2.33.320：认证取码→同份准备 PNG 的 straight RGBA input/内联 LUT→受限 child→
本地派生正式结果已组合到原远端 worker 和共享两个任务槽。真实 Delivery 身份/
释放 owner 保留，原结果消费且 lease 清空后才安装派生批次；本地失败不取消已经成功
终态的远端取码，未确认租约仍留在原补偿/占槽分支。原注册代次/来源/正式事务不放宽。
Shader 正式主端口/首个 content 合同、源码/映射流式准入、PNG 静态/自然尺寸/剩余
预算和不取路径/网络边界已有对应实现。完整浏览器 PNG 纹理矩阵与 straight RGBA
零差异；真实 TLS fixture、原样 Loom GLSL/LUT × 硬件/WARP 及完整矩阵验证组合链。
样例标准内联输入到实际 Loom runtime 的合同已静态追踪，fixture 使用其支持的别名，
不替代实际服务或 manifest 正式端口联调。用户入口、动态数值/字符串纹理接线、通用
图片 profile/方向和正式 ANGLE 供应链继续待完成，capability/pure-shader 门仍关闭。
完整 N01–N12/JavaScript/长稳/最终手测不缩减；未组包或替换程序，ZIP 仍为 v305。
完整库 3169 passed / 72 ignored / 0 failed（203.75 秒），认证取码组合/纹理准备
各 4 passed。Clippy、资源/EXE/会话启动/startup 2/1/9/7、fmt、3403 文件行数和
diff check 通过；前后原生源码一致。最终 EXE 另绑定摘要重验六项后端、两个本地
平台门及私有入口拒绝；证据位于 linshi/otter-native-v320-regression。

v0.2.33.319：本地 Shader 已有跨线程 owned request、按需父 worker 与真实受限
child→Image+原始 PNG；复用既有两个执行槽、完成唤醒、退休/关闭/回收和应用
pending_windows 提交循环，不新建应用队列。本地与远端 Completion 显式分型，
不伪造远端 delivery/lease；正式提交重验节点/请求/来源凭证，原应用配置/拓扑
绑定使变更退休旧请求，旧报告不清空新绑定。捕获纹理、输入和许可先释放再通知。
五项普通生命周期、隐藏应用七场景与两项显式硬件/WARP 平台测试已定向通过；
平台门分别验证真实 child 的共享任务提交和隐藏应用主循环→正式图像/显示/中继，
完整矩阵解码 PNG 和显示像素精确。本地平台用测试 EXE child，产品 EXE 后端另验；
生命周期替身不当 GLSL 证据，成功本地应用用例不额外发起远端执行。
用户侧取码/纹理 staging、正式 ANGLE 供应链和入口尚未接通，Shader capability/
pure-shader 拒绝继续关闭。完整 N01–N12、JavaScript、长稳和最终验收不缩减；
未组包、替换用户程序或提交推送，最新已验证 ZIP 仍为 v305。

完整库 3160 passed / 71 ignored / 0 failed，本地普通生命周期 5 passed，显式
本地平台 2、ANGLE 6、原真实 child/产品桥接门 6、WebP 21、SVG 69 passed。
Clippy、资源/EXE/会话启动/startup 2/1/9/7、fmt/3395 文件行数/diff 通过；前后
原生源码身份一致，回执位于 linshi/otter-native-v319-regression。用户侧获取/
staging、正式发行包与完整 N01–N12 验收仍未完成，不因本轮生命周期通过而放行。

v0.2.33.318：正式图像 owner 保留同源原始 PNG，output/output_image 别名和快照
共享像素/编码字节。下游输入、会话保存及后台恢复不再通过 PBGRA 重新编码，避免
v317 已实证的 straight RGB 精度丢失；普通图片仍由既有 WIC owner 编码，完整 alpha
矩阵与旧空编辑合成的解码值兼容。预算含像素+PNG，别名保守重复计数，OutputStamp、
请求代次、取消和失败原子性不放宽。预览仍不能传播为正式端口。
五项 owner/真实输入 worker、真实会话保存→恢复→inline 及隐藏 HWND 应用图测试通过；
后者只对新 PNG 路径验证显示/中继/传播准入/下游编码，不重新发起 TLS 执行。
本地异步 stamped 事务与应用 Shader 调度、Loom 取码/纹理、正式 bundle 供应链仍未
完成；能力报告/pure-shader 拒绝保持关闭。完整 N01–N12、JavaScript、资源长稳和
最终手测继续保留，未组包、替换用户程序或提交推送，最新已验证 ZIP 仍为 v305。

完整库 3155 passed / 69 ignored / 0 failed，普通 Shader 20、显式 ANGLE 6、真实
child/桥接门 6、WebP 21、SVG 69 passed。Clippy、资源/EXE/会话启动/startup
2/1/9/7、fmt/3389 文件行数/diff 通过，前后原生源码身份一致；完整回执位于
linshi/otter-native-v318-regression，不将数据链回归当完整 Shader 或资源长稳验收。

v0.2.33.317：child raw RGBA 到原生显示/PNG 的旧 Canvas alpha 桥接已实现，
独立 `shader_image` 只准备完整 Image+PNG，复用 WIC 导出 owner，不发布图端口。
真实浏览器全部 256² channel/alpha 矩阵证明不能使用普通整数反预乘；逐步 f32
归一化/倒数/乘法和 ties-to-even 与独立解码 PNG 精确一致。原生 PBGRA 为通道
clamp 到 alpha，不再乘一次 alpha；straight BGRA 直接编码，透明 RGB 清零。
编码 PNG 与 PNG→Canvas2D 后值严格区分，后者不能当 PNG straight 字节 oracle。
五项普通桥接测试及真实产品硬件/WARP 的十组矩阵/Loom/4K 桥接场景通过，
失败/配额/中途及迟到取消不交付部分结果，输入 raw 不保留。普通 PBGRA 导出
合同不变，未增加线程/队列/帧循环。正式 bundle 供应链、应用取码/纹理与带
stamp 的本地正式结果端口仍待完成；Shader capability 和 pure-shader 拒绝继续
关闭，不把后端桥接当 N01–N12 或完整 JavaScript/Shader/资源长稳验收。
证据位于 linshi/otter-native-v317-regression；未组包、替换用户程序或提交推送，
最新已验证 ZIP 仍为 v305。

完整库 3149 passed / 69 ignored / 0 failed，Clippy、资源/EXE/会话启动/startup
2/1/9/7、fmt/3386 文件行数/diff 通过；普通 Shader 19、显式 ANGLE 6、真实 child/
桥接平台门 6、WebP 21、SVG 69 passed。前后原生源码身份一致，最终产品 EXE
与构建/SHA 和重跑平台门绑定；不将本轮当可发布包或整个进程树/GPU 长稳验收。

v0.2.33.316：Shader 已有独立 OSQ1/OSR1 wire、父侧发布门、受限单请求 child 和
真实产品 EXE 私有入口，父进程不加载 ANGLE。共用进程 owner 保持原图片 128/256 MiB
策略；Shader 单独 256 MiB 单 child 池，正常单一 15 秒期限覆盖容量/传输/退出，
失败终止并同步等待，不将 Job 误称权限沙箱或驱动清理硬实时保证。短 body 在
纹理分配前拒绝，typed failure/坏帧/异常退出/额外字节/取消不发布成功像素。
真实测试与产品 child × 硬件/WARP × 三组旧浏览器合同共十二组最大字节差零；
产品 3840×2160 identity 验证全部 raw RGBA、alpha 与方向。首次硬件 context
失败已独立定位为空环境缺 SystemRoot，仅用 Windows API 的真实目录构造该白名单
变量，不继承父环境，图片 child 继续空环境。测试 DLL 仍非正式产品依赖。
应用 Shader capability、pure-shader 派发、PNG/PBGRA 桥接与正式图端口发布尚未
开放；正式供应链、JavaScript、N01–N12 及完整资源长稳继续保留，不将 child
平台门当最终应用验收。回执位于 linshi/otter-native-v316-regression，最新已验证
ZIP 仍为 v305；未组包、替换用户程序或提交推送。
完整库 3144 passed / 66 ignored / 0 failed；普通 Shader 14、显式 ANGLE 6、真实
child/产品平台门 3、WebP 21、SVG 69 passed。Clippy、资源/EXE/会话启动/startup
2/1/9/7、fmt/3383 文件行数/diff 通过，前后原生源码身份一致；产品 EXE 另绑定
构建与 SHA，不以同名文件存在当作新鲜构建证明。4K Job 峰值不代表全树/GPU 长稳。

v0.2.33.315：新增不含浏览器/WebView/HWND 的 ANGLE→D3D11 离屏原样 GLSL ES3
后端，返回 top-down raw RGBA。进程唯一且线程亲和的 EGL owner、失败清理、API
错误永久 poison、固定 DLL 摘要/文件锁/受限搜索/模块路径与像素预算已接通。
真实旧 ShaderRenderer + Loom Color Transfer/LUT 的三组浏览器 raw RGBA，在
RTX 2080 Ti/WARP 两后端全部逐字节相等（六组最大差零，alpha 精确）；定向六项
通过，还覆盖反复重建、uniform、编译失败、取消、poison、跨线程第二 owner 拒绝
及 DLL 写锁。旧 PNG 的半透明反向预乘与 raw 输出不同，已独立记录但桥接未实现。
测试 DLL 不是产品依赖，正式来源/供应链、隔离 child/Job/期限、Loom 取码/纹理和
图像发布仍待完成，未宣告 Shader runtime，也未解除 pure shader 的未实现拒绝。
完整 N01–N12、JavaScript/Shader 应用闭环与资源长稳仍未完成；未组包、替换用户
程序或提交推送，最新已验证 ZIP 仍为 v305。证据位于 linshi/otter-native-v315-regression。
完整库 3133 passed / 61 ignored / 0 failed，Clippy、资源/EXE/会话启动/startup
2/1/9/7、fmt/3371 文件行数/diff 通过，前后原生源码身份相同，测试进程零残留；
startup 合计 22.29 秒，每 child 原 15 秒期限不变，不据单次耗时宣称性能改进。

v0.2.33.314：自有 popup/overlapped 的实际局部 WGC 已运行六个停止/缩小/销毁
场景，验证 60×30 输出、源移动与放大、中心像素输入反馈、子窗口精确坐标
`[60,35]`、持键释放、原快照恢复和 worker/恢复 child 回收。先红灯复现坐标
`[90,52]`，再用独立九种 DPI 投递组合确认已映射 lParam 被 Win32 二次缩放；
最终投递层作用域切换接收窗口 DPI，并在成功/实际超时后恢复调用线程，不改变
键盘或授权边界。原整窗 WGC 焦点取消/关闭释放已重跑通过。定向 1/2/1/6、
完整库 3129 passed / 59 ignored / 0 failed，Clippy、资源/EXE/会话启动/startup
2/1/9/7、fmt/3365 文件行数/diff 通过，前后源码身份相同，测试进程零残留。
startup 合计 23.42 秒，原每 child 15 秒期限不变；证据位于
linshi/otter-native-v314-regression。真实 Crop 像素只检查中心，不冒充全图比较；
自有窗口/合成消息不是任意应用、物理热键或跨显示器混合 DPI 验收。完整 N01–N12、
JavaScript/Shader、资源与长稳继续未完成；未组包、替换用户程序或提交推送，
最新已验证 ZIP 仍为 v305。

v0.2.33.313：Ctrl+2 同一冻结选择表面支持拖动区域和双击整窗；冻结前保留来源
Target，窗口内区域绑定局部 Crop，移动跟随，选区期间失效拒绝而不换源。捕获
裁剪和输入共用偏移，DWM/WGC 尺寸不符不猜坐标，原区域被缩小时停止；整窗和
裁剪共用源恢复与会话清理。已验证自有屏幕外两种窗口样式的 WGC item 几何与
隐藏源移动后的输入坐标，但未启动实际捕获；来源选择/选择器/Crop/GPU 定向
4/2/3/1 通过。完整库 3129 passed / 57 ignored / 0 failed，Clippy、资源/EXE/
会话启动/startup 2/1/9/7、fmt/3363 文件行数/diff 通过，前后源码身份相同、
测试进程零残留。startup 合计 24.99 秒，每 child 15 秒期限不变；回执位于
linshi/otter-native-v313-regression。不据此宣称实际连续 WGC/任意程序/混合 DPI/
资源长稳或完整 N01–N12
验收通过；最新已验证 ZIP 仍为 v305。

v0.2.33.312：固定 Ctrl+2 通过唯一全局热键宿主接通已有屏幕区域 Live 入口，复用
任务/模态/恢复重验和 pending 去重。Primary/Live/Long 三键注册独立降级、失败
补偿及销毁清理，窗口共享菜单与恢复凭证纳入 Live 状态。命令缺项先红灯复现，
定向 11 项通过；选择回调注入，未占用实际系统热键或读取用户桌面。EXE 冒烟
覆盖应用切换、恢复三键移交/补偿和凭证失效；完整库 3125 passed / 55 ignored /
0 failed，Clippy、资源/EXE/会话启动/startup 2/1/9/7、fmt/3358 文件行数/diff 通过，
前后源码身份相同，测试进程零残留。startup 合计 27.49 秒，每 child 15 秒期限不变；
证据位于 linshi/otter-native-v312-regression，不将本轮通过当历史间歇问题修复。
旧窗口内区域锚定和双击整窗选择仍未等价，不以当前屏幕区域入口替代原目标；
完整迁移尚未完成，最新已验证 ZIP 仍为 v305。

v0.2.33.311：活跃长截图的再次 Ctrl+3 与完成按钮共用停止/排空/拼接状态；绕过
仅用于新选区的模态准入，但仍核验实际注册 owner、共享采集身份与当前窗口代次。
自动采集停止后只收下当前任务，拒绝额外采集/移动/方向命令；失败保留会话要求
显式重试，取消/关闭/销毁/布局变化不发布结果。40 项 capture_input 定向通过，
包含真实屏幕外 DialogBox 循环处理合成热键、原 worker/timer/EndDialog，及迟到
通知/窗口属性/owner 隔离；不等于真实系统按键或用户滚动应用验收。完整库
3122 passed / 55 ignored / 0 failed，Clippy、资源/EXE/会话启动/startup 2/1/9/7、
fmt/3357 文件行数/diff 通过，前后源码身份相同。startup 共 26.94 秒，每 child
原 15 秒期限未变；未据重跑通过声明历史间歇问题已修复。证据位于
linshi/otter-native-v311-regression；本轮未组包、替换用户程序或提交推送。
Web 自动方向及滚轮驱动录制仍未等价，完整迁移尚未验收，最新 ZIP 仍为 v305。

v0.2.33.310：共用挂载/显式恢复报告增加有界静态 image、loom_resource 和
remote_resources，补齐图片 Art 的远端能力协商缺口。读取权限和原资源/格式安全
门禁不变，不扩 scroll、touch 或脚本 runtime。旧报告的缺项已红灯复现；TLS 对端
独立检查实际发出的三项声明。报告 1 项、Application 17 项定向、完整库 3114 passed /
55 ignored / 0 failed、Clippy、资源/EXE/会话启动/startup 2/1/9/7、fmt/行数/diff
通过，前后源码身份相同。证据在 linshi/otter-native-v310-regression；未组包，
最新已验证 ZIP 仍为 v305，不等同实际用户服务或完整迁移验收。

v0.2.33.309：完整 Application poll 自动处理代表性表单输入，配置失败保留待提交
意图；续进将 input 与图片组合链扩展为真实本地 TLS attach/active/inactive ACK，
不再注入挂载结果或激活 ACK。图片验证资源 GET、内联解码、上传、来源优先级及
完成后撤权丢弃。配置回调沿原授权/device门禁后读取，与资源/生命周期共享，
生产默认和退出泵不变。不是实际用户 Loom 服务验收，不扩大 image/scroll 报告。
增加默认关闭的分段计时诊断，不改变15秒启动门禁。续进完整库 3114 passed /
55 ignored / 0 failed，Clippy、资源/EXE/会话启动/startup 2/1/9/7、fmt/行数/diff
通过；先前 UIA 单项失败在全库中通过，未据此声称间歇根因已修复。证据在
linshi/otter-native-v309-resume；最新已验证 ZIP v305。

v0.2.33.308：实际 attach/restore 宿主能力报告补齐六类已实现表单节点，新增
本地 TLS attach→授权窗口→真实控件消息→batch POST→快照回填→撤权组合回归。
激活 ACK 注入，不等于实际 Loom/应用自动调度验收；仅现有有界声明式合同，
不扩大触屏、图片、脚本/Shader、完整 CSS/IME 声明。六类 TLS 组合通过；全库首轮
既有 Live 回收断言失败并导致48项锁污染，单独复测与全库重跑通过（3108/55 ignored），
间歇根因未确认，保留失败证据。Clippy和资源/EXE/会话启动2/1/9通过；startup两轮
均4/7通过，三个完整smoke触及15秒期限，正式启动门禁未通过，未放宽超时。
独立诊断到达destroyed marker，系统高负载仅为可能因素；最新 ZIP 为 v305。

v0.2.33.307：非 Art/mini 的 resolved display 可复制为独立静态可编辑快照，
保留当前显示标注/裁剪；无 layers Editor 绑定显示底图，已有 layers 不被覆盖。
不继承目标 Canvas 来源、不复制 Live owner/连线、不强制刷新 Live 帧；Art/mini
仍扁平。copy_job 定向 14 项、全库 3107 passed / 55 ignored、Clippy及资源/启动
2/1/9/7 通过；本轮 startup 未超时，但不将 v306 的间歇超时标成已修复。
日志：linshi/otter-native-v307-regression；最新已验证 ZIP 仍为 v305。

v0.2.33.306：静态展开单钉图可编辑粘贴保留复制物理尺寸与鼠标偏移，补齐连续
粘贴级联；源当前矩形仅影响 inside 判断，成功提交后推进 UnitId 锚点，失败不
推进，旧请求不污染新 copy。20 DIP 按源 DPI 换算步长，目标 DPI 不重复缩放尺寸。
新增隐藏 HWND 的生产交付 smoke；全库 3105 passed / 55 ignored、Clippy、资源/
EXE/会话启动 2/1/9 通过。startup 曾一项触及原 15 秒期限，未放宽门禁，重跑
7/7 通过；失败记录保留，间歇原因未确认。最新已验证 ZIP 仍为 v305。
多节点、跨进程及 mini/Live/Art 完整可编辑复制不在此增量范围。

v0.2.33.305：进程内可编辑复制补齐capture metadata（含未知字段）和只读原图
租约；粘贴后绑定新文档，来源路径不作授权、不重开文件。源/缓存退休不影响
仍存活的粘贴文档原图租约。其他v304边界不扩大；最新已验证ZIP为v305，
298文件及隔离解包运行通过，新增editable/capture metadata标记通过；
收据linshi/otter-native-v305-runtime/verification.json。未签名、不可公开发布。
clipboard73项、全库3100 passed / 55 ignored、Clippy及资源/启动2/1/9/7通过；
日志：linshi/otter-native-v305-regression。实际系统剪贴板交互仍未验收。

v0.2.33.304：静态展开单钉图的进程内复制/粘贴保留矢量、栅格标注、裁剪和
底图透明度；真实发布后sequence绑定单槽，每次粘贴新建文档/标注身份且清空
源撤销历史。复用后台准备与窗口批次交付。Art/缩略/显示帧仍扁平，跨进程、
定位/级联、metadata和多节点复制仍未完成；系统剪贴板实际交互待集中验收。
最新实际ZIP仍为v302，v304未组包。
验证：clipboard71、copy_job13、全库3098 passed / 55 ignored、Clippy修复后通过，
资源/启动2/1/9/7通过。生产smoke包含真实隐藏可编辑粘贴窗口，但不写系统剪贴板。
首次生产编译夹具错误及修复后证据均保留在linshi/otter-native-v304-regression。

v0.2.33.303：Ctrl+3 接入固定垂直长截图生产入口，复用托盘恢复双重门禁和既有
采集交付。Ctrl+3 冲突保留 Ctrl+1，菜单共享实际可用性；注册补偿、独立销毁
清理和恢复凭证覆盖附属键。不是 Web 自动方向/再次按键完成的完整等价。
真实全局热键交互待验；最新实际 ZIP 为 v302，v303 未组包。
定向8项、全库3092 passed / 55 ignored、Clippy及资源/启动2/1/9/7通过；
证据：linshi/otter-native-v303-regression。无源码提交或公开发布。

v0.2.33.302：textarea 默认/声明盒模型与配色进入实际 EDIT：边框内缩、padding
格式矩形、内圆角/遮挡 region、声明 CTLCOLOR 与共享颜色 owner。半透明背景
仅对控件底色预合成，非父场景透明；lineHeight 等未完成，report 不扩大。
最新实际 ZIP 为 v302；298文件归档/真正解包校验及独立运行探针通过。
收据：linshi/otter-native-v302-runtime/verification.json。未签名、不可发布，
不代表完整功能等价或干净机器验收。

v0.2.33.301：textarea 自身 textAlign 映射到真实 EDIT，placeholder 使用实际
样式方向；不扩 input/number 自身样式。祖先对齐浏览器行为仍未取得实测证据，
不宣称完整文字排版兼容，report 不变。最新实际 ZIP 仍为 v299。

v0.2.33.300：声明式 input/textarea/number 接入与布局同源的继承字体/字号/字重
及 DPI 同步；复用 Select 的 HFONT 生命周期，不复制字体解析。未扩大 report，
未宣称完整 CSS/IME/在线挂载等价。v300 尚未重新打包，最新实际 ZIP 为 v299。

v299 包级证据更新：独立 Release ZIP 已构建并验证，真正解包后 298 文件核对、
独立 cwd/TEMP 的 smoke/两类非法参数探针通过，TEMP 零残留。最新实际候选为
v299，不再是 v294；见 linshi/otter-native-v299-runtime/verification.json。
包内 runtimeVerified=false、publishable=false、signed=false 均未改写，
不将有限运行探针提升为完整功能、干净机器或可见桌面验收。

v0.2.33.299：Live 无框窗口、2 物理像素绿边、16 像素黄角与 5 像素 L 命中
已接入；黄角左键复用移动 owner，内凹区继续保留源输入。Alt+Space/黄角右键
保留全部原系统菜单，Alt+Delete 走回收关闭。输入失败去重提示与菜单状态不
再依赖标题栏。仍待实际可见菜单/焦点、真实 WGC/物理输入/跨 DPI/性能验收；
失败提示目前复用原生 MessageBox，未新建非模态 toast。最新实际 ZIP 为 v294。

v0.2.33.298：Live 新增 Alt+左键移动和 Alt+Shift+左键等比拖动；统一物理
屏幕坐标，固定初始几何，锁定/隐藏/失焦/取消/捕获丢失可靠退休。本地捕获
独立于源输入状态刷新，源控制不自动恢复；begin 与定位有重入代次保护。
等比拖动固定左上角、归一化主导轴定比例，是补齐 FEATURES 合同，不是复刻
Web 尚未实现的拖动缩放。无框、绿框黄角、Alt+Delete 和真实跨屏验收仍待完成。

v0.2.33.297：接通 Live 精确 Ctrl+wheel 指针锚定客户区缩放和 Alt+wheel
整体透明度，锁定/隐藏阻断；本地手势取消源控制并释放捕获，不自动重新授权。
透明度贯通 surface 重建、保存、冻结和回收，原始帧像素不烘焙 opacity。
保存/冻结后沿静态 normal_opacity 语义。无框、绿框黄角、拖动及完整真实
WGC/物理输入/混合 DPI 验收仍未完成；最新实际 ZIP 仍为 v294。

v0.2.33.296：Live 初始化使用原物理选区/窗口可见外框，不再固定 960×640
和系统默认位置。输出客户区按实际 HWND DPI 校正并逐像素复核；定位失败
不启动采集。WGC 帧尺寸独立保留。标准窗口外观仍在，无框/绿边黄角/本地手势
及真实跨 DPI 多屏验收未完成；最新实际 ZIP 仍为 v294。

v0.2.33.295：Live 自身新增断线选择入口，复用静态连线快照/分页和应用删除
校验，取消、过期、两端锁定/忙碌不修改连线。来源隐藏与确认后其他模态活跃
均拒绝提交；显示、执行与保存沿既有拓扑凭证收口。可见菜单/自绘/WGC/性能
完整验收仍待完成，本轮未重新打包，最新实际内部 ZIP 为 v294。

v0.2.33.294：Live 系统菜单可主动创建出向连线，共用原连线选择框、目录与
Workspace 校验；模态确认后重查两端及来源版本。隐藏控件/注入选择已验证
实际连线、显示及保存，取消/无效/过期不发布。未进行可见对话框、自绘或
完整 WGC/性能验收；断线沿用目标节点入口。v294 内部 Release ZIP 已完成
构建身份、归档与真正解包后的 298 文件校验；独立 cwd/TEMP 下 smoke、缺图
--pin、未知参数三个运行探针通过且 TEMP 无残留。证据为
linshi/otter-native-v294-runtime/verification.json；不等同于干净机器或全面
运行验收，包内 runtimeVerified=false 保持不变，未签名、未发布。

v0.2.33.293：Live 系统菜单可显式刷新连线快照；最新请求按 owner 有界合并，
保留旧显示直到成功读回。首次/显式快照提交后按原策略通知下游，流帧不触发；
失败无通知，过期来源退休。可控 WARP 多色帧与真实读回错误回归通过，未替代
物理菜单/WGC/自绘验收；Live 自身连线菜单仍待完成，内部 ZIP 仍 v270。

v0.2.33.292：Live 输出连线可在静态画布和下游中继中显示按需快照，读回复用
显示合成单 worker，不逐帧复制；目标原图不覆盖，Art 覆盖层归目标。断线、
关窗、工作区替换清除旧显示。显式快照刷新/通知及 Live 自身连线菜单仍待
完成；不是实时逐帧投影或真实 WGC/物理交互验收，未重新打包，ZIP 仍 v270。

v0.2.33.291：既有目标节点连线目录可选择 Live output，运行节点经原输入
worker 读回固定 GPU 帧并编码真实图片。owner 身份保护贯通准备/运行/结果，
隐藏 HWND/WARP 与回环 TLS 对端已核对输入像素及结果回到节点。Live 到静态
画布显示、快照传播及 Live 自身连线入口仍未完成，不等同于真实 WGC/用户
服务/物理交互验收；未重新打包，内部 ZIP 仍 v270。

v0.2.33.290：Live 可自行创建分组并加入、重命名当前组，复用名称输入框与
应用模态域；确认后复核来源，候选原子发布，失败不留下半成品。组身份及保存
保持一致。名称选择注入测试不等于可见对话框验收；Live 连线/执行、自绘视觉、
WGC/物理输入/混合 DPI 仍待完成，未重新打包，内部 ZIP 仍为 v270。

v0.2.33.289：Live 系统菜单支持加入/离开既有分组，拒绝过期菜单和旧工作区。
共享隐藏/恢复、锁定输入和明确不回收的批量关闭覆盖 Live；组删除保留窗口。
创建/重命名仍从静态宿主入口操作。系统菜单视觉、物理交互、多 DPI、Live 连线
及执行数据流未完成最终验收，未重新打包，内部 ZIP 仍为 v270。

v0.2.33.288：Live 启动登记 graph 身份，冻结交接保留同 ID/连线/分组，保存携带
groupId。静态关闭与最后宿主替换保留存活 Live，Live 终止清理失效身份。分组/
连线用户入口及执行数据流尚未接通，不把模型级生命周期证明当作完整 graph
交互验收；未重新打包，内部 ZIP 仍为 v270。

v0.2.33.287：应用管理的 Live 关闭消息及显式系统菜单进入当前帧回收事务，
后台准备成功且窗口销毁后才发布；失败保留窗口。明确放弃命令不保存帧，保证
无帧/旧工作区仍有退出通道。源故障与应用退出不自动回收；Live 完整 graph、
真实 WGC/物理交互与最终验收仍未完成，内部 ZIP 仍为 v270。

v0.2.33.286：显式会话保存包含运行中 Live 的同身份静态帧，后台读回/编码复用
现有保存槽及磁盘事务，保存不停止采集、恢复不自动重新授权采集。窗口/集合
变化与取消拒绝旧候选；流运行时保持 Changed。Live 交互 graph、关闭回收和
最终真实 WGC/混合 DPI 验收仍待完成，未重新打包。

v0.2.33.285：Live 停止后的静态窗口继承提交时图像区域的屏幕位置和显示尺寸，
排除窗口边框/留黑边，原始像素不重采样；无效几何拒绝发布且保留 Live。隐藏
HWND/WARP 测试覆盖读取期间移动/缩放与空客户区，不替代真实混合 DPI/WGC
交互验收。运行中 graph 保存、直接关闭回收仍未完成，内部 ZIP 仍为 v270。

v0.2.33.284：Live 系统菜单可“停止 Live 并保留当前画面”。后台读回，复用原
Live 创建时 UnitId 发布静态文档，成功后才停止采集；旧工作区 Live 不得冻结进
恢复后的新工作区。隐藏真实 HWND/WARP 验证成功和失败路径。运行中 Live 的
graph 登记/保存、关闭回收、位置/缩放继承和真实 WGC 手测仍未完成，ZIP 仍 v270。

v0.2.33.283：补齐真实 Live BGRA GPU 帧的有界 CPU 读回及 Session 显式帧移交。
WARP/隐藏 HWND 回归覆盖最新待呈现帧与旧呈现帧分离、源关闭后已移交 owner 存活、
取消和失败释放；不抓取用户桌面。原生 Live 尚无持久 Unit 绑定，旧版停止后保留
同 ID 静态贴纸的语义仍待接入，未新增用户冻结命令，不算 Live 端到端迁移完成。

v0.2.33.282：新增原生回收保留设置入口，确认触发后台裁剪，回收偏好接入
设置另存为、覆盖和自动保存。非法值、取消、过期异步结果不得改变当前设置。
真实隐藏窗口验证使用注入选择，不涵盖可见表单手测或 Neuro 深色视觉一致性。
Live 冻结、其他关闭入口、完整迁移和最终人工验收未完成；内部 ZIP 仍 v270。

v0.2.33.281：会话加载和配置安装/导入/保存后自动安排回收保留规则裁剪；
后台复用保存单槽，来源或策略变化则拒绝旧结果。无变化不置脏，有变化保留磁盘
基线并等待显式保存。已有 cache 配置字段参与运行策略，尚未新增其专用编辑界面。
自动裁剪失败报告一次，不做无限重试；独立 Live、键盘删除及全面人工验收仍未完成。

v0.2.33.280：显式菜单“关闭贴纸并移入回收站”接通当前快照、worker、Receipt
复核和真实 HWND 销毁/原文发布。最后窗口预建空宿主并保留恢复入口；失败不提前
消费原文。默认回收规则来自已加载 cache 设置。通用 WM_CLOSE、分组退出、键盘
删除与独立 Live 窗口未并入此流程；启动/设置变更自动裁剪仍未实现。
真实隐藏 smoke 验证关闭、失败、保存和恢复；系统热键注册转交仍需专门运行验收。

v0.2.33.279：回收前快照进入现有后台 worker，任务显式区分保存、库更新与
回收；保存和收藏消费者不能领取回收结果。真实隐藏窗口 smoke 覆盖当前图片/
透明度、单槽、完成后取消、stale、一次交付和准备阶段窗口/原文不变。
尚未接用户关闭入口和最终发布；最后宿主与热键交接仍待完成。内部 ZIP 仍 v270。

v0.2.33.278：补齐关闭前的回收候选与 cache 回收策略读取。当前快照冻结、
单窗口/关联链接裁剪、先时间后尾部数量保留、非法日期保留和硬容量拒绝已有
纯模型验证；收藏继续复用同一冻结实现。尚未接实际关闭 owner，也未在启动或
设置变更时自动裁剪；这不是“关闭入库”完成。日期暂支持应用产生的 ISO UTC、
显式时区偏移和纯日期，其余旧格式保留，不宣称完全等价 JS Date.parse。
实际内部 ZIP 仍为 v270，总迁移及人工验收未完成。

v0.2.33.277：回收站/参考库每条记录新增“删除此条”动作，恢复/复制仍保留。
两库各 4096 项的普通/删除命令均按完整分页映射；删除按 entryId 精确移除，
不同条目即使同 sourceStickerId 也保留。复用库更新 worker/Receipt/磁盘基线，
取消和 stale 不发布，不关闭活动窗口。隐藏 smoke 覆盖两库第二页删除及实际
临时文件两次覆盖保存。关闭入库、回收策略、Live 实时快照和人工验收仍未完成；
实际内部 ZIP 仍为 v270。

v0.2.33.276：新增“收藏当前贴纸（更新快照）”及“取消当前贴纸收藏”原生
命令。同源收藏替换旧快照；取消按源身份移除且不关闭贴纸。使用当前保存模型
而非磁盘旧贴纸，后台执行图片编码，复用保存/恢复互斥与取消/stale 保护。
修复库内存编辑后的普通覆盖保存基线冲突：独立保留已提交磁盘原文，且不放松
外部改动检测。隐藏 smoke 验证创建/更新/复制/取消及真实临时文件多次覆盖。
全工作区快照/Guard 仍可能因其他窗口编辑而拒绝收藏；Live 专用实时快照、
关闭入库和回收保留策略未完成；没有人工交互验收，实际 ZIP 仍为 v270。

v0.2.33.275：原生弹出/托盘菜单已有回收站、参考库入口；后台索引、每页
64 项的原地导航、退出菜单后的单槽请求及冻结 worker/追加事务已连接。
测试覆盖两库各 4096 项身份映射，隐藏原生 smoke 覆盖 65 项第二页选择、
请求去重和目标过期拒绝、实际图片 HWND 追加、回收消费及参考保留。
仍不是人工交互验收或 N06 完成：关闭入库、参考新增/删除、保留策略待接线；
索引关闭最坏等待和最大业务菜单组合未实测。实际内部 ZIP 仍为 v270。

v0.2.33.274：冻结恢复/参考复制复用真实后台 worker 与现有单任务恢复协调器，
完成后追加而非替换工作区。隐藏 smoke 验证取消、stale、并发互斥、非默认
坐标空间、回收消费及参考保留；普通替换入口拒绝裸 Frozen。菜单条目与请求
尚未接线，仍不把后台 API 验证记为 N06 交互功能完成，实际内部 ZIP 仍为 v270。

v0.2.33.273：冻结贴纸新增单窗口追加/原文共同提交事务。独立隐藏窗口验证
旧 HWND/分组/连接保留、成功后回收消费、参考保留、stale 来源/编辑/坐标拒绝，
并验证新贴纸未知字段进入保存组装结果。菜单与后台请求调度尚未连接，未对用户
会话写盘，不能将事务 smoke 视为 N06 完成；实际内部 ZIP 仍为 v270。

v0.2.33.272：冻结库增加独立身份、鼠标偏移位置的单窗口候选准备，保留快照
未知值并剥离运行所有权；现有后台准备链路验收成功和未授权路径失败两种结果。
完整原生库 2976 passed / 55 ignored。菜单、追加窗口发布和回收站成功后
消费/持久化仍未接线，不代表 N06 完成。当前实际内部 ZIP 仍是 v270。

v0.2.33.271：新增会话回收站/参考库的有界只读入口与元数据准入，完整保留
FrozenStickerEntry 原文；坏库与活动窗口恢复隔离。尚无菜单恢复/参考复制、
新增冻结条目或保留策略接线，不把 JSON 可读记作 N06 功能完成。会话相关
392 passed，startup 7 passed；当前实际分发候选仍是下述 v270 内部包。

v0.2.33.270 当前：生产 popup_menu::choose 已调用自绘 track_live /
track_tray_live，并连接只读历史 Session 和菜单关闭后的目标核验交付；下面
v244 关于“仍使用 TrackPopupMenu、尚未连接历史”的段落是历史记录，不是
当前缺口。文本交付、单次撤销及独立图片钉图已由共用生产路径 smoke 验证，
不代表读取用户真实历史、物理键鼠或最终交互体验已经验收。

菜单的基本 UIA 树、展开、滚动、Invoke、只读标记及两类跨进程事件订阅已有
测试证据；Toggle/SelectionItem、ScrollItem/虚拟化、动态 DPI、真实辅助
技术验收及已记录的订阅范围差异仍未完成。v270 原生库 2965 passed / 55
ignored，startup 7 passed，具体事件范围见迁移记录，不泛化为全范围通过。

v270 当前 Release ZIP 已实际构建、验证及解包运行：298 个文件一致，隔离
cwd/TEMP/TMP 下隐藏 smoke 与两个错误参数探针通过，TEMP 无残留。
最新包位于 linshi/otter-native-v270-package，运行证据位于
linshi/otter-native-v270-runtime；unsigned/internal/publishable=false，
不是干净机器、完整功能或 N01–N12 最终验收。本轮不修改产品源码或递增版本。

### 历史阶段记录（以下不能覆盖上述当前核验）

v0.2.33.244：历史读取已具备后台 Task、Events 与 Session 组合，列表几何和
自绘菜单基础窗口、级联控制器及模态消息泵均已开发并有局部原生测试。但应用
入口仍使用系统 TrackPopupMenu，尚未连接历史 Session、滑块捕获和实际选中
交付；DPI 字体、完整菜单呈现/助记符及可访问性也未完成。真实 HWND/消息
测试不等于物理键鼠、跨应用焦点或完整 N06 验收，不能提前标记迁移完成。

v244 完整回归：原生库 2899 passed / 53 ignored，startup 7 passed，菜单相关
19 passed；只证明当前代码和已有启动链回归，不代表新菜单已在应用入口启用。

v0.2.33.236：系统剪贴板历史只读后端已实现，使用进程内 DataPackage 验证
Bitmap/Text、编码/尺寸/像素预算、取消与读取许可；尚未接历史列表、更新
订阅、目标授权和实际选择交付，也没有读取用户真实历史。不能把后端合同
测试记作完整 N06 或交互 Windows 历史验收。

v0.2.33.235：本轮核对确认 N06 系统剪贴板历史（Bitmap/Text、事件更新、
状态列表、选择后提交和异步清理）仍缺少原生生产接线；不是当前剪贴板图片
粘贴已经覆盖。已开始补整批文字一次历史的编辑事务，历史菜单尚未完成。
旧表关于“智能文件列表/命名/缓存尚未迁移”的记录不能用作当前事实：
copy_job 已经通过 file_plan::Plan 和 clipboard::copy_formats_checked 发布
有界文件列表；不要据此重复开发。v234 修改前全量复核实际为 2854 passed、
0 failed、53 ignored；v230–v234 构建/ZIP/解包证据见 WINDOWS_NATIVE_MIGRATION.md。

v235 编辑事务接入后全量 2859 passed、0 failed、53 ignored，实际 EXE startup
7 passed；仅证明本轮模型与现有启动链回归，不证明剪贴板历史 UI/WinRT 已完成。

以下为保留的阶段记录，不是当前功能缺口的独立验收依据。

v0.2.33.215 开发中：context-stroke 零宽 use 与 marker 均已复现浏览器差异；
六案例聚合测试汇总两项像素失败。未修复、未宣称全量通过，最近完整绿色仍为
v214，见 SURFACE_SVG.md。

v0.2.33.214：静态路径 marker 准入、递归保护及十组浏览器对照接入；
全量 2739 passed / 51 ignored（143.71 秒），startup 7 passed（17.48 秒）。
Application/EXE 验证与剩余行为见 SURFACE_SVG.md，整个原生迁移未完成。

v0.2.33.213：pattern overflow 和模板描述子内容兼容性续进；累计十八组
浏览器基线，首轮 title 模板像素失败已定位并适配。全量 2736 passed / 51 ignored
（145.51 秒），startup 7 passed（17.55 秒），最终验证见 SURFACE_SVG.md。

v0.2.33.212：静态 pattern 准入及递归保护，八组真实浏览器/child 像素基线通过；
全量 2734 passed / 51 ignored（142.09 秒），startup 7 passed（16.98 秒）。
完整回归见 SURFACE_SVG.md，不将首批图案接入当作全部 SVG 或原生迁移完成。

v0.2.33.211 最终：symbol/use viewport 与零尺寸修复、双 href 优先级回归通过。
全量 2731 passed / 51 ignored（132.13 秒），startup 7 passed（14.93 秒）；
不关闭已有 mask 量化、文本完整性以及 N01–N12 未验收项。以下续进记录按阶段保留。

v0.2.33.211 续进：defs/svg 适配已修复原 symbol 百分比和零尺寸失败，
SVG 专项 27 passed / 2 private ignored；八组浏览器改写前后逐字节一致。
下面 v211 失败描述保留为修复前记录，完整回归结果见 SURFACE_SVG.md。

v0.2.33.211 开发中：symbol/use 初步接入，但百分比 viewport 已复现浏览器差异，
未验收。引用图门现覆盖 mask/symbol/use；最近完整绿色记录仍为 v210。

v0.2.33.210：SVG mask 接入，alpha/区域/空遮罩等七组浏览器字节对照通过；
彩色 luminance 组保留最多 1 级量化差异。非递归引用图拒环与过深展开。
全量 2727 passed / 51 ignored（126.81 秒），不将已知差异或全部 SVG 记作完成。

v0.2.33.209：嵌套 SVG viewport 进入生产 worker，11 组浏览器像素基线及实际
Job child 对照通过；修复零宽/高错误绘制，保留原资源安全边界及根自然尺寸。
全量 2722 passed / 51 ignored（133.41 秒），不是全部 SVG 或全原生迁移完成。

v0.2.33.208：数字删除用例显式设置并恢复线程键盘快照，保持原撤销断言；
真实 HWND 已证实 Ctrl/Alt 可产生与 v207 相同的撤销状态变化，不能反推首次
失败唯一原因。全量 2721 passed / 51 ignored（141.68 秒），focused 4 passed。
不改生产 SVG 或依赖，不把夹具修复计作新渲染能力；全迁移仍未完成。

v0.2.33.207：有限 SVG text/tspan 已实际接入 worker；多语言与彩色 emoji 专项
通过，系统字体预算/TTC index 不变。并非完整浏览器文字一致性或整个 SVG 完成。
全量 2719 passed / 1 failed / 51 ignored，数字输入删除撤销用例 EM_CANUNDO 为 0；
独立 focused 通过但根因未关闭。准确范围及证据见 SURFACE_SVG.md。

v0.2.33.206：字体来源已有系统范围 fallback 和有界 COM 文本源，返回 UTF-16
长度、scale、可选 Face，继续保持文件准入和预算；尚未接 renderer、未开放 text。
不以字符候选选择冒充复杂文本/emoji shaping 或完整 SVG 支持。
首轮全量 UIA Select 出现一次失败；诊断后独立复测 2717 passed / 51 ignored，
偶发问题未根治，失败证据保留，详见 SURFACE_SVG.md。

v0.2.33.205：SVG 按需字体来源模块进入原生 crate，文件准入、预算、TTC 与模拟
样式元数据已有实现；尚未接 renderer，不将此前置当作文字支持或全迁移完成。
继续实现方向及测试边界见 SURFACE_SVG.md。
全量 2715 passed / 51 ignored（133.03 秒），Clippy/fmt/3138 文件严格行数通过。

v0.2.33.204：SVG v2 的逻辑尺寸元数据已接通 child→父侧验证→Decoded/Ready→
授权窗口 Bitmap；小数尺寸不再被直接拒绝。栅格 extent 与布局尺寸分离，剩余
子像素/AA/DPI 以及文字/CSS/动画等缺口见 SURFACE_SVG.md，全迁移仍未完成。
全量 2712 passed / 50 ignored（138.33 秒），Clippy/fmt/3134 文件严格行数通过。

v0.2.33.203：生产呈现层已有独立浮点逻辑尺寸，测量/contain 不再必须绑定整数
像素比例；整数格式默认路径保持。尚未接通 SVG child/Ready 尺寸元数据，不将呈现
夹具通过当作小数 SVG 端到端支持；全迁移仍未完成，详见 SURFACE_SVG.md。
全量 2709 passed / 49 ignored（132.18 秒），Clippy/fmt/3133 文件严格行数通过。

v0.2.33.202：首批静态 SVG 已接到 Image/Inline 的受限 child 路径，使用独立自然
尺寸与全树准入，共用父侧响应/预算/最终发布门。受限子集、依赖门及剩余功能见
SURFACE_SVG.md；文字/CSS/动画/foreignObject 等未完成，N01–N12 仍未完成。
全量 2706 passed / 49 ignored，startup 7 passed；Clippy/fmt/行数/依赖门通过。

以下为历史检查点：

v0.2.33.201：现有 WebP 已使用共用受限 child host/wire/process owner；一个 Job
launcher/容量池，保留原协议与颜色/像素行为，新增独立帧边界检查。SVG renderer/
viewport/资源策略尚未接入，完整迁移仍未完成。无依赖变更、提交、推送或发布。
全量 2698 passed / 48 ignored（120.85 秒），startup 7 passed（19.09 秒），
Clippy/fmt/3128 文件严格行数/diff 通过；详细验证范围见 SURFACE_SVG.md。

2026-10-22：SVG 浏览器/原生候选对照已完成接入前第一轮实测，真实差异及未完成
能力列于 SURFACE_SVG.md。确认原生生产尚未支持 SVG；不以仅静态矩形替代文字、
foreignObject、动画或权限合同，生产版本仍为 v0.2.33.200。

v0.2.33.200：修复真实 A2B0 mft2 RGB→XYZ WebP 与 Chromium 的亮度/非线性差异。
限定类型直接求值，保留原 child/Job/alpha/预乘边界；六个编码样本及具体限制见
SURFACE_WEBP_ISOLATION.md。其他 LUT 类型/混合 profile、峰值和 N01–N12 仍未完成。
本轮不新增依赖，不提交推送发布。
全量 2695 passed / 48 ignored（119.96 秒），ICC 专项 7 passed；Clippy/fmt/
3126 文件严格行数/diff 通过，48 ignored 中原 44 项专项仍未执行。
startup 7 passed（19.12 秒），本轮证据归 linshi/otter-v200-evidence.json。

v0.2.33.199：WebP 嵌入 RGB ICC 在受限 child 转为 sRGB，修复实测 Display-P3 色差。
先转换后预乘，alpha 保持、分批缓冲、Job/截止时间保留；坏 metadata fallback 与
资源/不支持错误区分。真实 LUT/更多色域/其他格式/监视器/HDR/峰值及 N01–N12 未完成。
详见 SURFACE_WEBP_ISOLATION.md；无提交推送发布。
最终全量 2692 passed / 48 ignored（139.59 秒），startup 7 passed（17.40 秒）；
Clippy/fmt/3124 文件严格行数/diff 通过，真实 Application P3 图片安装/上传通过。
安全 Advisory 仍有 20 个漏洞 ID，不称零漏洞或正式发布门通过。

v0.2.33.198：WebP EXIF 方向完成当前 Chromium 兼容基线核验，32 个有效样本均
保持编码方向，原生真实 child 对照同尺寸/完整像素。不是新增旋转功能，前一轮
旋转计划已因反证纠正；JPEG/TIFF 规则不变。ICC、元数据导出、动画和 N01–N12
仍未完成，未提交推送发布；见 SURFACE_WEBP_ISOLATION.md。
32 案例专项通过；最终全量 2687 passed / 48 ignored（112.31 秒），startup 7 passed
（14.62 秒），Clippy/fmt/3122 文件严格行数/diff 通过；原生进程零残留，浏览器已关闭。

v0.2.33.197：WebP 八类坏 IPC/非零退出经真实 Job child 与生产父侧链验证，合法
对照核对像素，每个场景检查进程确实退出；补容量 deadline/等待中取消。没有修改
来源/格式合同或开放可配置 child。完整崩溃/Job 分配失败/峰值、元数据及 N01–N12
仍未完成，详见 SURFACE_WEBP_ISOLATION.md；无提交推送发布。
最终全量 2686 passed / 48 ignored（115.28 秒），startup 7 passed（16.67 秒），
Clippy/fmt/3121 文件严格行数/diff 通过。四个私有 ignored child 由父测试调用，
原 44 项专项未执行；本轮 UIA Select 通过不代表历史间歇性已修复。

v196 诊断续验：UIA Select 新增仅失败输出的调用阶段标签，不改生产行为或断言。
最终版本独立三轮及全量通过（2685 passed / 47 ignored，120.10 秒），Clippy/fmt/
严格行数/diff 通过；原 2684/1/47 失败证据仍保留，不能把后续通过称作根因修复。
绑定层已确认 S_OK + null interface 会表现为 Err(HRESULT(0))；具体失败调用尚未
捕获。此轮不重跑同生产源码的 startup，不增版本、不提交推送，整体迁移仍未完成。

v0.2.33.196：真实静态 WebP 已走 Resource/Inline、受限子进程、Ready、HWND 安装绘制。
单 child 128 MiB Job、最多两个并行，额外请求可取消地等待；父侧像素/授权门不变。
有损/无损/alpha 与 Chromium 夹具及 Application 路径已验证，Job 超限分配拒绝及
停滞超时/取消回收有实际证据。不使用系统可选 codec，无主进程回退；不是完整权限
沙箱或整组内存峰值保证。EXIF/ICC/XMP/动画和完整故障注入尚未完成，N01–N12 未完成。
详见 SURFACE_WEBP_ISOLATION.md；未提交推送发布。
本轮全量 2684 passed / 1 failed / 47 ignored（118.14 秒），失败为既有 UIA Select
系统交互项，独立重现，旧测试 EXE 同项也焦点失败；原因未定位，未修改/放宽该测试。
WebP 专项 9 passed，实际 Application 两项通过；startup 7 passed（14.97 秒），
Clippy/fmt/3120 文件严格行数/diff、1228 身份的双 SBOM 核验通过。47 ignored 中三个
是父测试实际调用的私有 child 入口，原 44 项未执行。全量回归未通过，不称迁移完成。

v0.2.33.195：普通 Surface 以有限签名识别实际静态位图，再由 WIC 核验容器/帧数，
不因错误 MIME 跳过 APNG/EXIF。data URL 先规范化再提取 MIME，来源/摘要/授权门
不放宽；实际 SVG/WebP/动画/blob 仍未完成。Tile/Wall 不改，旧资源缓存参数限制
不复制；兼容性以已核对 Chromium 为准，不宣称完整 MIME Sniffing 标准实现。
浏览器 32 案例、真实 Application 内联与 TLS 资源安装/上传已验证；无提交推送发布。
新增 9 项回归；最终全量 2674 passed / 44 ignored（109.94 秒），startup 7 passed
（15.24 秒），Clippy/fmt/3112 文件严格行数/diff 通过。历史交互不稳定性没有宣称根治，
44 项 ignored 专项未执行，完整 N01–N12 迁移没有完成。

v0.2.33.194：Surface 静态 JPEG/TIFF 已处理 EXIF 八方向及变换后自然尺寸，保留原始
尺寸/最终像素预算与取消边界；普通打开和 Relay 仍使用原帧尺寸，不扩大协议能力。
JPEG/TIFF 像素位置回归与独立 Chromium JPEG 八方向基线一致，方向 6 JPEG 已走真实
Application 安装/上传。XMP fallback、PNG eXIf、ICC、完整格式/blob/在线验收仍未完成。
无提交推送发布，N01–N12 未完成；详见 SURFACE_IMAGES.md。
新增 7 项回归；最终全量 2665 passed / 44 ignored（96.91 秒），startup 7 passed
（13.72 秒）；Clippy/fmt/3110 文件严格行数/diff 通过。首次预算报错文本兼容性失败
已恢复原文本并全量重验，日志保留；未将历史交互不稳定性宣称为已修复。

v0.2.33.193：无像素 data:image/ 来源保留 Image/alt，原生缺图标记和替代文字按
独立尺寸测量，不伪造自然图片比例；成功安装移除，resourceId unresolved 不回退。
替代内容表示当前像素不可用，不冒充终态错误或浏览器 pending 外观。原事件/授权/
UIA 规则不放宽，不扩大通用 image 能力；完整格式/blob/在线验收仍待完成。
新增 6 项回归并扩展 3 项应用测试；修复 alt 退场误清空跨节点选区，跨节点复制能力
不扩张。最终全量 2658 passed / 44 ignored（96.50 秒），startup 7 passed（13.88 秒）；
Clippy/fmt/3108 文件严格行数/diff 通过，无提交推送发布，历史交互不稳定性不宣称根治。

v0.2.33.192：受限 data:image/ 静态源已接通应用自动解码/安装/上传，图片阶段零
HTTP Config/资源 GET；沿用共享任务槽/取消/同 Grant 窗口授权，不伪造 ResourceLease。
类型化资源/节点键，resourceId 优先不回退，原始快照摘要防止同版本偷换来源；安装
刷新只计算一次摘要。新增 8 项回归，保留 1 MiB 场景、64 项与原像素预算；SVG/WebP/
动画/blob/损坏占位/在线业务仍未完成，通用 image 能力不扩大。无提交推送发布。
最终全量 2652 passed / 44 ignored（101.37 秒），startup 7 passed（14.43 秒），
Clippy/fmt/3105 文件严格行数/diff 通过；历史 UIA/点击不稳定性不因本轮绿灯宣称修复。

v0.2.33.191：图片 UIA 名称使用 alt，忽略 image 分支未接的通用标签/disabled；
空/缺失/非字符串 alt 的装饰图片不进入 control/content view。Image 呈现遍历为
叶节点，原始 children 不再导致测量拒绝，后代也不能成为 UIA/焦点/click 目标。
原始节点预算与授权合同指纹未放宽；这不是完整图片来源/格式或屏幕阅读器人工验收。
新增 5 项回归，系统 UIA 2 / 模型 2 / 路由 1 均通过；全量 2644 passed / 44 ignored
（99.04 秒），startup 7 passed（16.09 秒），Clippy/fmt/3099 文件严格行数通过。
历史 UIA/点击不稳定性未宣称修复；无提交推送发布。

v0.2.33.190：应用 poll 已接通 active 场景 resourceId 自动 GET/后台解码/Ready 安装。
按完整 Binding/revision 一次尝试，容量不足留队、换版本重建，窗口起点轮转；没有
放宽同 Grant 安装与撤权。新增 5 项回归覆盖真实续签/TLS/完整 poll 两图安装上传、
损坏响应不重复请求、关闭资源结算、队列与轮转。挂载输入仍合成，不冒充在线 Loom、
整个生产等待链或完整图片来源/格式验收；宿主通用 image 声明尚未扩大。
最终源码全量 2639 passed / 44 ignored（102.08 秒），startup 7 passed（17.42 秒）；
真实目录占据最后一槽的回归已保护回收后再调度的顺序。历史 UIA/点击不稳定性不因
本轮通过而宣称根因修复；无提交推送发布。

v0.2.33.189：同 Grant 的 Ready 授权转交、真实 HWND 安装和到期撤图入口已接通。
消费释放任务槽，窗口 Lease 接管授权，完整 Binding/revision 门不放宽；新场景、
关闭和撤权清表。同场景图片刷新保留未变文字选区，自动 GET/投递调度仍待接线，
不算完整图片功能或 N01–N12 完成；spinner/slider 交互连续性尚未完整验收。
新增 4 项回归，全量 2634 passed / 44 ignored（93.78 秒），Clippy/fmt/3092 文件
严格行数通过；历史 UIA/点击不稳定性仍未宣称根因修复。

v0.2.33.188：应用挂载/事件/batch 先略去未就绪图片、继续呈现其他节点，解除缺图
导致整个窗口拒绝的初始化环路。纯 Scene 投影不改原始来源或授权，仍保留当前资源
快照；默认低层严格测量不变。自动 GET/Ready 安装/到期撤除待接通，不算图片功能完成。
新增 4 项测试，独立串行全量 2630 passed / 44 ignored（91.13 秒），专项、Clippy、
3091 文件严格行数通过。历史 UIA/点击不稳定性仍未根因闭环。

v0.2.33.187：显式 Images 已接入真实 SurfaceWindow/Host 创建与替换，复用现有
测量、绘制、target 重建与关闭清理；真实 HWND 测试检查实际 bitmap 上传及释放。
旧 API 缺图仍拒绝、不继承同 ID 旧图；失败窗口的无障碍身份仍按既有规则撤销。
自动 GET、Ready 授权安装及到期撤除未接通，不扩大能力声明；无提交推送发布。
新增 3 项真实 HWND 图片回归，窗口专项 73 passed；独立串行全量 2626 passed /
44 ignored（91.26 秒），Clippy/fmt/3091 文件严格行数通过。不冒充历史 UIA/点击修复。

v0.2.33.186：修复受管图片发布遇到短暂 DELETE 句柄时立即失败、并掩盖 reopen
错误的问题；只对共享冲突有界等待且可取消，不放宽只读 pin 或字节核验。
新增 2 项受控真实文件句柄回归，资源目录专项 11 passed。历史 B7 现场仍不能仅凭
原日志确认其具体 reopen 错误；UIA/点击不稳定性尚未修复，图片安装主线未完成。
本轮独立串行全量 2623 passed / 44 ignored（90.48 秒）；Clippy、fmt、3089 文件
严格行数检查通过。本轮全绿不覆盖 v185 失败，不构成历史 UIA/点击问题修复证明。

v0.2.33.185：初始挂载、单事件和 batch 保留同源当前资源快照；窗口在成功呈现后
提交新资源身份，使用当前生命周期 Binding 取得请求/复核 Ready，关闭与撤权释放。
Debug 脱敏且有界，未接自动调度/图片 HWND 安装，不扩大 image 能力，无提交推送发布。
资源专项 31 passed，startup 7 passed；三轮全量各有 1 项不同失败（2620 passed / 44 ignored），
详见 SURFACE_IMAGES.md，未认定根因或宣称修复，当前全量验收未通过。

v0.2.33.184：已解码图片的显式生产测量/呈现入口接通，支持自然比例与有界反馈，
滚动/显露保留图片 owner，场景替换不隐式继承旧图片。真实 Chromium DOM 样式补测
13 项；四项图片像素测试改走生产 NativeMeasure/Presentation，不再用固定测量夹具。
自动资源请求和 HWND 安装未接通，其他布局/来源待验，不扩大能力声明，无提交推送发布。
新增 7 项测试，图片专项 11 passed，全量 2616 passed/44 ignored（87.15 秒）；
Clippy、fmt、3085 文件严格行数、diff 与 22 文件 UTF-8 无 BOM 检查通过。

v0.2.33.183：增加 Direct2D 图片绘制后端，复用原 PaintFrame 顺序/裁剪/透明层，
固定 contain 和 content-edge 圆角，按 target 缓存与有界上传，重建/帧退场释放。
真实像素夹具不代表 Image 生产准入；自动请求、安装和自然尺寸布局仍未完成，
NativeMeasure 拒绝门与能力声明保持不变。未提交、推送或发布。
新增绘制专项 4 passed；全量 2609 passed/44 ignored（86.98 秒），Clippy、fmt、
3083 文件严格行数、diff 与 12 文件 UTF-8 无 BOM 检查通过。

v0.2.33.182：资源任务已在后台解码 PNG/受支持的静态容器和 RGBA，复用共享 WIC
及 Pixels；消费像素保留原授权与当前 mount 门。到期主动唤醒并回收，不增加轮询线程。
PNG/RGBA 有像素用例，其余格式未全验；动画、多帧、SVG/WebP 等仍待实现。
尚未接自动图片节点请求、测量与绘制，不扩大 image 能力；无提交、推送或发布。
新增 4 项测试；WIC/RGBA 解码专项 3 passed，资源专项 11 passed；最终可见全量
2605 passed/44 ignored（88.83 秒）。Clippy、fmt、3081 文件严格行数与 diff 检查通过。

v0.2.33.181：图片资源后台 Job 和 Ready 已接现有 Sessions 容量/等待/取消/回收。
Ready 保留原授权与任务槽，应用按当前活窗口/快照版本取消旧资源；退出统一 drain。
仍未接图片节点自动启动、解码、布局与绘制，不能声称 image 可用。详见 SURFACE_IMAGES.md。
新增 7 项测试；资源相关 44 passed，全量 2601 passed/44 ignored（85.51 秒）。
Clippy、fmt、3077 文件严格行数及 diff 检查通过；无提交、推送或发布。

v0.2.33.180：增加受 mount/revision/lease 约束的 loom_resource 图片字节获取 API；
固定来源 TLS、Device/nonce、精确长度、SHA-256、过期与消费前取消门已接线。
普通 HTTP 仍保留 8 MiB 上限。未接入生产 job/队列、解码/布局/绘制，image 仍不支持。
shared_memory/data/blob/Tile 不能据此标为完成；无提交、推送或发布。
专项 5 passed；最终可见全量 2594 passed/44 ignored（84.54 秒），Clippy、fmt、
3073 文件严格行数与 diff 检查通过。真实在线 Loom、生产队列和图片绘制尚未验收。

2026-10-21 图片基线：原生 image 仍不支持；旧组件 14 场景与相关前端 12 测试通过。
来源优先级、授权资源获取、默认 stretch/contain 和祖先 click 事实已固定在
SURFACE_IMAGES.md。无生产代码变更，仍 v0.2.33.179；接下来补完整安全资源链。

v0.2.33.179：正式高亮每帧一次解析范围，逐项复用已有 Text 索引，去除重复端点/
item 线性扫描；不改变绘制顺序或常驻资源。新增独立旧公式对照及显式微基准，
范围计算测量不代表整机帧时间或 N11 已验收。
最终可见全量 2589 passed/44 ignored（96.32 秒），真实像素与独立 oracle 通过；
新增微基准已单独运行，1024 节点二十轮范围计算中位 284.5251→0.4436 ms，非全帧。
Clippy、fmt、3071 文件严格行数和 diff 检查通过；未提交、推送或发布。

v0.2.33.178：Shift+上下已按 focus 的实际行和相邻 owner 接入，全局列意图跨
节点保留；三布局真实多行基线 15 断言通过。空布局/末尾换行与软换行同偏移
新增定向验证。单 Surface 范围不等价于浏览器跨页面范围；其余边界见文字选择文档。
最终可见全量 2588 passed/43 ignored（96.43 秒），startup 7 passed（20.05 秒）；
Clippy、fmt、3070 文件严格行数及 diff 检查通过，未提交、推送或发布。

v0.2.33.177：真实旧组件键盘 54 断言支持跨节点 Shift+左右与 focus 行 Home/End，
原生已接入；普通左右保持跨节点选区不变。上下 keydown/keyup 门禁已统一，
跨节点上下导航本身仍未实现。详见 SURFACE_TEXT_SELECTION.md，不代表整体迁移完成。
最终可见全量 2585 passed/43 ignored（88.24 秒），Clippy、fmt、3070 文件严格
行数与 diff 检查通过；未提交、推送或发布。

v0.2.33.176：旧组件六次真实鼠标词手势/30 项断言发现尾随空白差异，已修正
初始双击与后续拖动的范围规则，增加内部词跨节点往返测试。证据见
SURFACE_TEXT_SELECTION.md；未将此结果替代系统输入、跨节点键盘或完整迁移验收。
最终可见全量 2581 passed/43 ignored（77.22 秒），startup 7 passed（12.14 秒）；
Clippy、fmt、3069 文件严格行数与 diff 检查通过。另七项微小移动探针使浏览器
断言总数达到 37；不移动保留空白，移动 0.1 至 6 CSS 像素的本次取样转为词干。

v0.2.33.175：跨节点词级拖动已接入目标节点词边界，支持反向和返回初始节点；
中文/emoji UTF-16 归属及 HWND 捕获/copy/click 隔离增加三项测试。跨节点键盘、
旧浏览器词范围精确对照和性能仍待验证，系统双击探针旧失败没有被替代。
最终全量 2580 passed/43 ignored（77.47 秒），startup 7 passed（13.20 秒）；
Clippy、fmt、3069 文件严格行数及 diff 检查通过，未提交、推送或发布。

v0.2.33.174：普通鼠标跨节点选区已接入独立端点和场景序高亮；返回同节点可恢复
文字复制，跨节点禁止聚合。范围变化失效、真实 D2D 像素及 HWND 5 项新增测试纳入
最终全量 2577 passed/43 ignored（81.14 秒）。跨节点词手势/键盘仍未完成；系统
双击探针旧保护失败与复杂离屏/性能缺证未被本次结果替代。

2026-10-21 跨节点布局基线：六种布局、12 项实际拖选断言通过，确认选区按节点
而非视觉坐标顺序、stack 顶层命中和控件排除；跨节点复制仍走 Unit copy。
原生跨节点端点模型尚待实现，详见 SURFACE_TEXT_SELECTION.md，不提升内部版本。

v0.2.33.173：新增真实系统双击探针；首次因前台不是自有窗口，在注入前保护失败。
该交互验收未通过，失败日志保留。默认可见全量 2572 passed/43 ignored（80.96 秒）
不包含新 ignored 探针；生产词选择行为未变，不将默认回归替代实际系统双击证明。

v0.2.33.172：原生双击选择词及前向/反向词级拖动已接入，非文字 click 和滚动条
双击按下兼容。8 项新增测试纳入实际可见全量 2572 passed/42 ignored（88.06 秒）。
按需系统 ICU 不增加启动静态导入；跨节点、emoji 完整旧范围与系统输入验收仍待补齐。

2026-10-21 系统 ICU：实际 60 组分词边界及空/上限/超限检查通过，中文/泰文词干
符合所测浏览器样例。它仍只是词边界基础；尾随空白、emoji、跨节点与 HWND 双击
生命周期未据此完成。系统版本/可用性需显式处理，禁止静态加载失败影响应用启动。

2026-10-21 双击分词补证：15 类文本、288 个真实鼠标命中与 60 组系统分词结果
已保存；SelectableWordsSegmenter 的中文/泰文边界不同，不能直接声称等价迁移。
双击生产接线仍未完成；系统 ICU 仅为下一待核验候选，见 SURFACE_TEXT_SELECTION.md。

2026-10-21 浏览器基线：实际旧组件+快捷键 hook 的 Edge 夹具 6 项通过；跨节点
视觉选择是已观察差异，但 Ctrl+C 路由到 Unit copy，不是文字复制。Ctrl+A 范围
可超出当前 Surface；原生双击词选择尚缺。分别跟踪这些合同，不以单节点替代全局。
该结果不覆盖完整 Tauri 外壳或系统剪贴板，见 SURFACE_TEXT_SELECTION.md 的证据。

v0.2.33.171：补齐自动滚动实际 WM_TIMER 生成/派发、松键停止、取消/撤权后迟到
队列消息无效的验证；不改生产行为，不将系统 timer 证明混同物理鼠标或调度性能。
新增 2 项通过，全量 2564 passed/42 ignored（90.11 秒）、startup 7 passed；
跨节点/全选的复制范围尚需旧产品基线核对，不能擅自定义阅读顺序或扩大范围。

v0.2.33.170：单节点文字拖选支持边缘自动滚动，静止指针持续扩展，复用 auto 轴
祖先消费与缓存投影；仅活动手势按需持有 timer，撤销/关闭/边界及时停止。
这不是跨节点选择或物理输入验收，整体迁移仍未完成。
8 项新增专项通过，可见全量 2562 passed/42 ignored（87.00 秒）；既有 v169
间歇性 UIA 异常未因本轮通过而标记修复。

v0.2.33.169：Shift+Home/End 按实际排版行扩选，区分软换行、显式换行与空行，
End 不含换行符但保留尾随空白，保留视觉行归属与自动显露；普通 Home/End 仍滚动。
有界行数组为 O(行数) 查询，未做最大文本性能验收；全选/跨节点/词边界等仍未完成。
专项 5 项与 startup 7 项通过；前两轮全量出现既有 UIA Select 变更队列/列表项及
Edit 焦点失败，隔离组又读到非预期 `2hello`。补充可选消息采证后，关闭跟踪的
最终全量 2554 passed/42 ignored（93.19 秒）；旧异常原因仍未确认、未称修复。
完整失败记录与最新通过证据见 WINDOWS_NATIVE_MIGRATION.md 的 v169 节。

v0.2.33.168：Shift+Up/Down 按实际排版扩选，保留跨短行的目标列及软换行视觉
行归属；鼠标建立后首次导航和自动显露使用同一行信息。重排重置、纯滚动保留，
普通 Up/Down 仍滚动。尚非 bidi 水平视觉、词边界、全选、跨节点、拖选自动滚动
或 UIA TextPattern；物理键盘及完整浏览器语义未验收，整体原生迁移未完成。

v0.2.33.167：键盘选区端点按真实 cluster 自动显露，支持嵌套滚动与 hidden 轴
键盘显露，完全不可见候选不误滚外层；滚动后同步 EDIT region 且保留 draft。
纯显露不重测文字。仅接键盘路径，不冒充鼠标拖选自动滚动、完整键盘选择或
UIA TextPattern；整体原生迁移未完成。

v0.2.33.166：已有单节点选区新增 Shift+Left/Right 逻辑 cluster 扩展/缩回及
普通左右键折叠，真实 HWND 路由先于滚动；父焦点、local_edit、导航锚点共同
准入，保留 EDIT/Shift+Tab。尚非 bidi 视觉导航、上下行/词边界、自动 reveal、
全选或跨节点选择；线程键盘状态测试不冒充物理键盘验收，整体迁移未完成。

v0.2.33.165：文字 I-beam 已接正式命中与本地输入门禁；新增显式系统鼠标探针
验证实际消息到达、选区保留、祖先 click 抑制、捕获释放及系统光标。未读写
剪贴板，不等于物理设备/第三方粘贴验收；键盘、跨节点、双击、触摸及 UIA
选择能力仍待实现。具体证据见 SURFACE_TEXT_SELECTION.md，整体迁移未完成。

v0.2.33.164：原生单节点文字拖选、高亮、非空拖选抑制祖先 click 和 Ctrl+C
本地文本发布已接线；完成选区可跨同内容重排，活动取消/换场景/撤权不会复活。
真实 HWND、像素与无剪贴板发布回调测试分别验证，不冒充物理输入或第三方粘贴。
键盘扩选/全选、跨节点、文字光标、双击、触摸和 UIA 选区仍待推进，整体迁移未完成。

v0.2.33.163：OCR text.selectable 的真实缺口已定位并开始实现；DirectWrite
文字命中、规范化 UTF-16 范围及高亮几何基础已加入，TextItem 复用当前布局和
裁剪。尚无完整指针选择/绘制/复制交互，不能宣称部分 OCR 文本复制已经迁移。
退出条件和下一接线见 [可选择文字迁移](SURFACE_TEXT_SELECTION.md)。

v0.2.33.162：真实活动 pan 后撤权/换场景，再收到系统惯性与结束消息，内容
保持停止位置且无编辑/click 残留；水平数值经正式 prepare/settle 验证，末尾
仅一次 change commit、队列清空。四项显式探针最新通过；保留一次撤销前 pan
未活动的失败，及 v161 水平父滚动异常，均未定根因。不提升整体触摸能力声明，
不代表物理设备、多指/笔或网络服务端验收。详见触摸基线 v162。

v0.2.33.161：Slider 单指垂直 pan 经系统默认转发进入当前父 Surface 滚动，
实际注入验证“垂直滚父不改值、水平改值不滚父、均不 click”。系统保有完整
pointer 序列；父接收按当前挂载目标验证并以 target+sequence 隔离。此增量
取代此前“垂直父滚动未接入”，不代表多指/笔、复杂惯性或物理设备已验收。

v0.2.33.160：显式可见启动的系统触摸探针通过；抓到并修复兼容鼠标触摸的
祖先 click 误触发，以及捕获期间跨来源移动/释放/重复按下混入。四组真实系统
输入验证不残留捕获和数值边沿，未宣称垂直父滚动、完整触摸或物理设备通过。

v0.2.33.159：新增显式 Windows 系统触摸探针及真实 pointer 身份核验。实际运行
在 DOWN 前因命中外部窗口被安全门禁拒绝，未取得输入到达证据；不算原生触摸
通过，也未提升 capability。运行命令及失败边界见 SLIDER_TOUCH_BASELINE.md。

v0.2.33.158：完成浏览器模拟触摸基线核验，不是原生触摸实现。水平更新、垂直
父滚动、取消和轻触有独立语义，普通按钮阳性对照证明 click 观察有效。
不能无条件复用鼠标 click；接入约束见 [触摸基线](SLIDER_TOUCH_BASELINE.md)。
生产 touch capability 未改变，真实 Windows 消息和物理设备验收仍待完成。

v0.2.33.157：真实 Chromium 3 组对照确认切焦仅提交、捕获仍继续；原生 Tab/
程序切焦后可继续拖动并在抬起时产生原目标 click，父取消仍撤销手势。
修正此前 Tab 立即释放捕获的临时语义，键盘焦点实际进入 EDIT，不滞留 Slider。
其他失焦边界、触摸及人工验收仍未完成。
新增 4 项回归，全量库 2503 passed/37 ignored、startup 7 项及 Clippy/fmt/
3031 文件严格行数通过。

v0.2.33.156：真实 Chromium 5 组释放位置对照均保留 Slider 为 click 目标，
据此扩展 CapturedPointer 原始目标路径，越界/兄弟控件释放不再重新命中改投。
仍校验捕获起点、挂载/场景身份和权限；替换后旧候选失效，控件外起点拒绝。
Tab 后抬起及其他混合输入/人工验收仍待完成。
新增 4 项回归，全量库 2499 passed/37 ignored、startup 7 项及 Clippy/fmt/
3031 文件严格行数通过。

v0.2.33.155：Slider 内完成鼠标手势后接入祖先 click；复用单槽/场景身份/fence，
先结算自身 input/change 再准备内层→外层 click。无 handler 不占槽，同值不造
change，取消/替换/撤权不留迟到 click。跨控件释放及 Tab 后 click 尚未对齐，
不把本路径当作完整 DOM 输入验收。
新增 6 项回归，全量库 2495 passed/37 ignored、startup 7 项及 Clippy/fmt/
3031 文件严格行数通过。

v0.2.33.154：真实 Chromium 5 组混合输入对照，先复现后修复原生拖动中键盘
提交后抬起重复 change、Tab 漏提交；补齐继续拖动、边界键、Escape、同步销毁
和真实 Surface 队列/焦点回归，Slider 专项 26 项通过。点击冒泡和其他混合输入
及人工验收仍未完成，不把这 5 组行为等同完整输入对齐。
全量库 2489 passed/37 ignored、startup 7 项及 Clippy/fmt/3029 文件严格行数通过。

v0.2.33.153：Slider IsReadOnly 通知接入共享延迟调度；真实 MTA 验证仅停发布
不变只读、编辑撤销/恢复通知、重复无通知、拒绝写入及 handler 释放。
min/max/step 属于合同变更，新增 COM 验证旧引用撤销和新范围读取，不向旧身份
发送元数据变化。普通 RangeValue 专项 10 项通过；真实读屏/完整输入验收待完成。
全量库 2482 passed/37 ignored、startup 7 项及 Clippy/fmt/3028 文件严格行数通过。

v0.2.33.152：Slider 存活数值订阅在正常/强制退出后跨重启保持隔离，旧订阅
不接收新实例事件，最后解绑并释放 handler；新增 2 项，Slider 跨进程共 4 项通过。
元数据通知和真实读屏/完整输入人工验收仍未完成。
全量库 2479 passed/37 ignored（新增显式跨进程已运行通过），startup 7 项及
Clippy/fmt/3026 文件严格行数通过。

v0.2.33.151：Slider 专属跨进程 2 项通过，覆盖外部读写/通知/键盘/订阅释放、
子进程实际六个数值边沿结算、正常和强制退出撤销、重启同名节点旧引用隔离。
元数据通知、存活订阅退出隔离和真实读屏/完整输入人工验收仍未完成。
全量库 2479 passed/35 ignored（新增 2 项跨进程已显式通过），startup 7 项及
Clippy/fmt/3026 文件严格行数通过。

v0.2.33.150：Slider Value 变化接入无 Host 借用的单消息延迟通知，覆盖合并、
净零、忙时重排、重入、替换/关闭；真实 MTA 订阅覆盖自动化与键盘输入及释放。
全量初测发现 UIA SetFocus 不保留焦点，已修正 Slider 可聚焦属性和焦点 owner；
完整客户端组 15 项通过。尚缺元数据通知、专属跨进程及真实读屏验收。
最终全量库 2479 passed/33 ignored、startup 7 项及 Clippy/fmt/3025 文件行数通过。

v0.2.33.149：Slider 接入 UIA RangeValue 实时读写与属性，复用 HWND override。
3 项 COM/MTA 回归通过：单节点系统树、数值范围/归一、共享 FIFO 双边沿、
捕获/容量拒绝、权限和旧 provider 撤销。尚缺属性变化通知、专属跨进程和
真实读屏验收；不提升整体 capability，不代表 Windows 原生任务完成。
全量库 2475 passed/33 ignored、startup 7 项及 Clippy/fmt/3023 文件严格行数通过。

v0.2.33.148：Slider 接入真实 Surface 布局、Mount、权限、共享 FIFO 与 Tab。
6 项真实宿主回归覆盖数值 input/change、合并/重试、满队列无半对事件、捕获
保持/取消、权威同步、合同/类型替换和禁用跳过。全量库 2472 passed/33 ignored，
startup 7 项、显式跨进程既有 UIA 4 项及 Clippy/fmt/3019 文件严格行数通过，
进程残留为 0。Slider UIA RangeValue、点击冒泡及完整输入人工验收尚未完成。

v0.2.33.147：新增真实原生 Slider 子窗口、键盘/系统捕获/父通知和 GDI 绘制。
数值仍为浮点 Range；真实回归覆盖输入顺序、拖动提交、取消/禁用/隐藏/销毁、
重入关闭、State 缺失、像素/DC 恢复和极小客户区裁剪。最终全量库 2466 passed/
33 ignored，Slider 共 13 项，启动入口拆分后 startup 7 项通过，严格行数全部
不超过 500。尚未接入 Mount/FIFO/业务准入/UIA，不宣告 Slider 端到端可用。

v0.2.33.146：新增 Slider Range 数值模型和 input/continuous、change/commit
数值 wire，归一夹具与真实 Chromium 对照。专项 5 项，全量库 2458 passed/
33 ignored，startup 7 项及 Clippy/fmt/3010 文件严格行数通过。尚未挂载原生
Slider 控件、输入或 UIA，极端浮点网格显式拒绝仍是兼容缺口，不能宣称可交互。

v0.2.33.145：Select 显式 color/currentcolor/alpha 到真实收起区域和未选中
列表像素通过；父级颜色更新保留同一 HWND，非法声明恢复默认，零 change。
Select 32 项，全量库 2453 passed/33 ignored，startup 7 项，显式跨进程 Select
1 项及 Clippy/fmt/3007 文件严格行数通过。fontFamily 不在现有主干白名单，
不新增该协议；background/opacity/主题变量及人工辅助功能验收仍是独立缺口。

v0.2.33.144：Select 系统列表接入固定高度 owner-draw，保留 HASSTRINGS、键盘
和 UIA；选中行浅底深字的真实像素通过并查看图像，行高随字体变化且相同
样式不关闭弹出层。Select 29 项，全量库 2450 passed/33 ignored、startup 7 项，
同进程 UIA 2 项、显式跨进程 Select 1 项及原有 UIA 3 项通过；Clippy/fmt/
3005 文件严格行数通过。人工读屏、高对比度切换及全部声明式样式仍未验收。

v0.2.33.143：Select hover 背景及 leave/disable/hide/cancel/负坐标清理通过
实际像素回归；Select 28 项，全量库 2449 passed/33 ignored，startup 7 项，
跨进程 Select 1 项及 Clippy/fmt/3003 文件严格行数通过，进程残留为 0。
系统键盘/选择/UIA 仍不由 hover 模块接管，列表选中主题等未完成项继续保留。

v0.2.33.142：真实收起控件白底像素失败后接入窄 GDI 绘制，默认、焦点、
禁用及调用者 DC 恢复回归通过，已查看三种状态图像。Select 26 项，全量库
2447 passed/33 ignored，startup 7 项，显式跨进程 UIA 4 项及 Clippy/fmt/
3001 文件严格行数通过，进程残留为 0。系统选择/弹出/UIA 合同保持不变。

v0.2.33.141：Select 自有弹出列表的未选中行背景与文字接入 Neuro 字段颜色；
真实像素修复前失败、修复后通过，并查看了测试内存绘制 PNG。Select 23 项，
全量库 2444 passed/33 ignored，startup 7 项；Clippy/fmt/像素专项/跨进程
Select 与 3000 文件严格行数通过，进程残留为 0。完整主题状态和真实读屏
验收仍未完成，选中行继续使用系统高亮，不扩大 capability。

v0.2.33.140：Select 复用测量层已解析的字号/字重与基准字体，新增
继承百分比/em、样式替换、句柄复用和无效字体保留原资源的真实窗口回归。
Select 22 项，全量库 2443 passed/33 ignored，startup 7 项，跨进程 Select
1 项与 Clippy/fmt/2998 文件严格行数通过，进程残留为 0。
不宣称全部 CSS 字体/主题语义或实际读屏体验已完成。

v0.2.33.139：Select DPI 字体与 HWND 生命周期接线通过真实 GDI 验证；
WM_DPICHANGED 仍从系统取权威 DPI，不把人工消息视为实际显示器切换。
Select 20 项、全量库 2441 passed/33 ignored、startup 7 项、跨进程 Select
1 项、Clippy/fmt/2997 文件严格行数通过，进程残留为 0。此处不把字体缩放
等同于完整声明式字体、主题配色或实际多显示器验收。

v0.2.33.138 软件增量通过：Common Controls v6 的系统 Select UIA 树、模式、
真实选择到唯一 change 收据已验，同进程 2 项、跨进程 Select 1 项；原有跨进程
生命周期 3 项通过。Select 17 项，全量库 2438 passed/33 ignored，startup 7 项，
Clippy/fmt 通过。N08 后续主题/DPI/读屏体验与整体迁移仍未完成，不扩大能力声明。

以下为本增量的历史定位记录，不能覆盖上述新验证结论。

v138 后续对照：嵌入 Common Controls v6 后，实际 UIA 树边界、选项数量、
Selection/SelectionItem 与折叠断言已越过原失败点。构建脚本统一宿主/测试
manifest，Mount 增加真实 CLOSEUP 最终索引核对以接通无 SELCHANGE 的 UIA
选择路径；完整门禁仍进行中。以下 MSAA 失败记录是历史排查，不再要求自建代理。

v0.2.33.138 进行中：Select 的 UIA ComboBox 身份、逻辑父级、焦点及展开已接线，
但展开后的 List 导航失败，不能宣告 Select UIA 完成。纯系统 HWND 与跨进程
对照都复现；直接调用系统 Combo MSAA 第三个 child 的 NAVDIR_NEXT 得到
VT_DISPATCH 而非 VT_EMPTY，与微软已记录的 Combo List 导航缺陷对应。无效的
popup provider 试验已撤掉；保留失败回归，待窄适配该 MSAA 子对象路径。
最近全量通过版本仍为下述 v137；v138 未通过全部门禁，未提交、推送或打包。
补充验证：单独修正 MSAA 方法能通过直接断言，但不能封住 UIA/IAccessibleEx
对象转换路径；关闭服务会丢模式、透传服务仍会越界。未通过的原型已归档至
linshi，工作树不保留该包装。接下来需验证 Ex pair 和模式返回对象的身份映射，
不能把直接 MSAA 通过或禁用模式作为完成条件。

v0.2.33.137：修复真实滚动后 Select 下拉仍展开的问题。按 origin/size/clip/dpi
变化收起列表，无变化同步保留展开；滚出后禁用、滚回恢复，保留选择且不合成
change。已有修复前失败/修复后通过证据，其他遮挡/DPI/主题/UIA 验收仍继续。
验证：Select 专项 16 passed，全量库 2435 passed/32 ignored，startup 7 passed；
Clippy、fmt、2994 文件严格行数（全部不超过 500）、diff/源码 whitespace 通过，
原生测试进程无残留。未提交、推送或打包。

v0.2.33.136：Select 已接入 EDIT/自绘 Button 混合 Tab/Shift+Tab 导航，离开前
收起下拉，跳过 disabled/hidden；真实 HWND 焦点、重复按键抑制与零 change 验证
通过。主题/DPI、弹出与滚动、UIA 和真实业务验收继续，不扩大 capability。
验证：Select 专项 15 passed，全量库 2434 passed/32 ignored，startup 7 passed；
Clippy、fmt、2993 文件严格行数（全部不超过 500）、diff/源码 whitespace 通过，
原生测试进程无残留。未提交、推送或打包。

v0.2.33.135：Select 已接入 Surface 测量/真实挂载/CBN_SELCHANGE/shared FIFO，
实际方向键通知生成字符串 change/commit；取消重排、source 同步、合同变更、
权限和队列回压已有回归，不进入文本 draft 或按钮表单值。主题/DPI/Tab/弹出与
滚动、UIA 选择模式和真实服务端验收仍待完成，不扩大 capability。
验证：Select 专项 13 passed，全量库 2432 passed/32 ignored，startup 7 passed；
Clippy、fmt、2991 文件严格行数（全部不超过 500）、diff/源码 whitespace 通过，
原生测试进程无残留。未提交、推送或打包。

v0.2.33.134：新增真实 Win32 COMBOBOX owner，已验证 Unicode/顺序/重复值索引、
方向键选择、实际弹出后禁用收起、父窗口关闭时列表销毁和 owner 撤销保护。
尚未接入 Surface 挂载/通知提交、主题/DPI/滚动或 UIA 选择模式，不能把独立控件
验证当 Select 完成，也不扩大 capability。
验证：Select 专项 9 passed，全量库 2428 passed/32 ignored，startup 7 passed；
Clippy、fmt、2989 文件严格行数（全部不超过 500）、diff/源码 whitespace 通过，
原生测试进程无残留。未提交、推送或打包。

v0.2.33.133：Select 开始实现有界 options/value/label 模型和严格字符串
change/commit 编码；保留顺序/重复值，不发送 input，不混入文本 draft 或按钮
表单值。已核对旧前端，但真实 HWND 下拉、交互、弹出清理与 UIA 尚未接入，
NativeMeasure 仍拒绝 Select，不扩大能力声明。Slider 仍待开发。
验证：Select 专项 5 passed，字段事件 6 passed，全量库 2424 passed/32 ignored，
startup 7 passed；Clippy、fmt、2987 文件严格行数（全部不超过 500）、diff/源码
whitespace 通过，原生测试进程无残留。未提交、推送或打包。

v0.2.33.132：跨进程 Switch/Button 焦点事件已验证缓存身份、实时焦点互斥、
注销后 handler 释放和不再交付。全局焦点接口先按自启 Child PID 过滤，不记录
其他应用身份，也不抢前台；不把窗口线程焦点当桌面前台/Narrator 完整验收。
动态挂载组合、Narrator 和其他原生迁移内容继续。
验证：三条显式跨进程用例连续三轮通过，全量库 2419 passed/32 ignored，startup
7 passed；Clippy、fmt、2984 文件严格行数（全部不超过 500）、diff/源码 whitespace
通过，原生测试进程无残留。未提交、推送或打包。

v0.2.33.131：真实跨进程根结构订阅已验证 UI owner 替换场景后的
ChildrenInvalidated、同根重读唯一原生 EDIT/标签/Value、旧节点消失和旧自绘
模式失效，注销后 handler 实际释放。测试命令管道有字节/条数/队列边界。
跨进程焦点、Narrator 和其他原生迁移内容继续。
验证：三条显式跨进程用例连续三轮通过，全量库 2419 passed/32 ignored，startup
7 passed；Clippy、fmt、2983 文件严格行数（全部不超过 500）、diff/源码 whitespace
通过，原生测试进程无残留。未提交、推送或打包。

v0.2.33.130：增加活跃 ToggleState/Invoked 订阅期间强制退出自启进程的验证。
新实例使用独立订阅且实际交付事件，旧订阅不串收同名节点通知，旧模式仍失效；
退出后注销旧订阅和新订阅均验证 handler 实际释放。原有先注销再退出路径保留。
跨进程焦点/结构通知、Narrator 与其他原生迁移内容继续。
验证：三条显式跨进程用例连续三轮通过，全量库 2419 passed/32 ignored，startup
7 passed；Clippy、fmt、2982 文件严格行数（全部不超过 500）、diff/源码 whitespace
通过，原生测试进程无残留。未提交、推送或打包。

v0.2.33.129：跨进程正常/强制退出用例增加真实 ToggleState 与 Invoked 订阅，
核对缓存节点身份和事件载荷；两类 handler 注销后实际释放，再执行退出测试。
仅订阅自启实例，回调只读 cache 并有界传递证据。不等同订阅中崩溃、跨进程
焦点/结构通知或 Narrator 验收，其他原生迁移内容继续。
验证：两条显式跨进程用例连续三轮通过，全量库 2419 passed/31 ignored，startup
7 passed；Clippy、fmt、2982 文件严格行数（全部不超过 500）、diff/源码 whitespace
通过，原生测试进程无残留。未提交、推送或打包。

v0.2.33.128：跨进程 UIA 新增强制终止自启 Child、旧模式失效和新实例隔离验证。
相同场景重启后新值独立初始化，新模式可操作；旧 Value 写入、Toggle 和 Invoke
仍失败，不能改变新实例。正常/强制退出共享有期限等待，不扫描其他进程。
跨进程事件订阅、Narrator 与其他原生迁移内容继续。
验证：两条显式跨进程测试连续三轮通过，全量库 2419 passed/31 ignored，startup
7 passed；Clippy、fmt、2981 文件严格行数（全部不超过 500）、diff/源码 whitespace
通过，原生测试进程无残留。未提交、推送或打包。

v0.2.33.127：真正跨进程 UIA 已验证独立 Surface 的节点唯一性/EDIT 父级、
Value 写后读、Switch 切换、Button 聚焦/Invoke，以及正常退出后旧模式失效。
子进程核对真实表单值、Automation 动作来源和 Switch 状态。只操作自启 Child，
通过管道关闭，不向缓存 HWND 发关闭消息；显式桌面用例连续三次通过。
跨进程事件订阅、强制退出、Narrator 和其他原生迁移内容继续。
验证：常规 UIA 39 passed/2 ignored，全量库 2419 passed/30 ignored，startup
7 passed；Clippy、fmt、2981 文件严格行数（全部不超过 500）、diff/源码 whitespace
通过，原生测试进程无残留。未提交、推送或打包。

v0.2.33.126：同场景 EDIT 的实际创建/移除/rebind 经 Mount dirty 接入共享根
结构通知；重复 binding、普通同步和空操作不通知。真实 MTA 已验证不 replace
的晚绑定通知、新节点唯一性和原生 Value 读取；HWND 数值复用不被误当错误。
更广 DPI/裁剪组合、跨进程/Narrator 与其他迁移继续。
验证：UIA 39 passed，全量库 2419 passed/28 ignored，startup 7 passed；Clippy、fmt、2979 文件
严格行数（全部不超过 500）、diff/源码 whitespace 通过，原生测试进程无残留。
未提交、推送或打包。

v0.2.33.125：原生 EDIT 经 HwndOverride 重定位到声明式父级，未知 HWND 不接管，
未挂载节点不暴露；保留系统属性与 Value/Text 模式，根焦点/命中可返回承载节点。
真实 MTA 验证三类控件唯一性、父级和原生模式，Input 焦点与写后读；多行系统
类型为 Document，未伪造为 Edit。修复无先前焦点时 SetFocus NULL 返回误判。
同场景重建、更广动态结构、跨进程/Narrator 和其他迁移内容继续。
验证：UIA 37 passed，全量库 2417 passed/28 ignored，startup 7 passed；Clippy、fmt、2978 文件
严格行数（全部不超过 500）、diff/源码 whitespace 通过，原生测试进程无残留。
未提交、推送或打包。

v0.2.33.124：场景旧身份撤销时在稳定 HWND 根延迟报告 ChildrenInvalidated，
连续替换合并、busy 重排、通知重入、失败替换仍失效及关闭抑制纳入回归。
真实 MTA 订阅由 UI owner 替换场景后核对事件与新树读取；不等同 EDIT 重定位、
动态可见性变化、跨进程/Narrator 或整体迁移完成。
验证：UIA 34 passed，全量库 2414 passed/28 ignored，startup 7 passed；Clippy、fmt、2974 文件
严格行数（全部不超过 500）、diff/源码 whitespace 通过，原生测试进程无残留。
未提交、推送或打包。

v0.2.33.123：焦点变化通知已接入既有 HWND/键盘 owner，经独立私有消息合并发送。
runtime-id 去重、失焦重置、busy 重排、重入防护、替换新身份与关闭抑制已有回归。
真实 MTA 订阅验证 SetFocus/Tab/自有 HWND 鼠标消息、缓存身份与实时焦点属性，
以及 handler 注销释放。SW_SHOWNOACTIVATE 只验证窗口线程焦点，不冒充桌面全局
前台焦点或 Narrator 验收；结构通知、EDIT 精确重定位和其他迁移内容继续。
验证：UIA 30 passed，全量库 2410 passed/28 ignored，startup 7 passed。
Clippy、fmt、2971 文件严格行数（全部不超过 500）、diff/源码 whitespace 通过，
原生测试进程无残留。未提交、推送或打包。

v0.2.33.122：Switch 的实际变化统一经 Session → Mount dirty → 窗口私有消息
发送 ToggleState，覆盖 UIA/鼠标/键盘及同步/结算路径。通知按当前身份合并，
替换/关闭撤销；busy 重排、通知重入防护和旧批次实时值检查已有回归。
真实 MTA 订阅验证 UIA 与自有 HWND 鼠标/键盘消息；不保证每个过渡值交付，
不替代系统输入、跨进程或 Narrator 人工验收。焦点/结构通知与其他迁移继续。
验证：UIA 27 passed，全量库 2407 passed/28 ignored，startup 7 passed。
Clippy、fmt、2968 文件严格行数（全部不超过 500）、diff/源码 whitespace 通过，
原生测试进程无残留。未提交、推送或打包。

v0.2.33.121：ToggleToggleState property 通知已通过真实 MTA 订阅，核对 Switch
缓存身份、On/Off VARIANT 与实时查询；禁用 HWND 的拒绝操作不变值、不发送。
提交暂停不影响本地通知；注销后 handler 释放和再次切换无旧回调已验证。
仅覆盖 UIA Toggle 来源，物理输入/权威同步统一通知、焦点通知、EDIT 重定位和
跨进程/Narrator 仍继续。未提交、推送或打包。
验证：UIA 24 passed，连续三轮通过；全量库 2404 passed/28 ignored，startup
7 passed。Clippy、fmt、2966 文件严格行数（全部不超过 500）、diff/源码 whitespace
通过，原生测试进程无残留。

v0.2.33.120：Switch 的 IToggleProvider 与实时 ToggleState 接入共享编辑 Session，
允许提交软停止时本地切换，change FIFO 满则不变值；祖先 click 使用 Toggle 前的
表单/fence，不等待自身 change。COM 生命周期、权限、回压与 MTA 二态切换已加入
回归。仅 UIA Toggle 的 property-changed 发送已接线，统一物理/权威变化通知和
property 订阅接收仍待完成；焦点通知、EDIT 重定位和人工验收继续。
验证：UIA 23 passed，全量库 2403 passed/28 ignored，startup 7 passed；Clippy、
fmt、2965 文件严格行数（全部不超过 500）、diff/源码 whitespace 检查通过，
原生测试进程无残留。未提交、推送或打包。

v0.2.33.119：Invoked 事件已通过真实 MTA 订阅、cache 节点身份、拒绝不发送和
注销后 COM handler 释放验证；专项连续三轮通过。确认系统可能重复交付一次
raise，测试区分 provider 发送次数与回调次数，不把通知当作远端动作完成。
AutoSetFocus 的持续焦点不作为 Invoke 合同，显式 SetFocus 专项保留。跨进程、
Narrator、焦点通知、Toggle/其他模式及 EDIT 重定位仍待完成。
最终 UIA 19 passed，联合专项连续三轮通过；全量库 2399 passed/28 ignored，
startup 7 passed。Clippy、fmt、2961 文件严格行数及 diff/源码 whitespace 通过，
无超过 500 有效行文件、无原生测试进程残留。未提交、推送或打包。

v0.2.33.118：Button 的 UIA Invoke 模式接入原单槽候选/FIFO fence/动作消费链，
不直接派发远端动作；Pointer/Keyboard/Automation 来源明确分开。待处理候选或
鼠标 press 与 Invoke 互斥，撤权、busy、disabled、隐藏和旧句柄拒绝执行。真实
MTA 客户端同时验证 AutoSetFocus 开/关；真实 EDIT 值冻结和依赖结算回归通过。
UIA 专项 18 passed。已接线 Invoked 通知发送，订阅接收未专项验证；焦点通知、
Toggle 等模式、EDIT 重定位和人工验收继续。未提交、推送或打包。
全量库 2398 passed/28 ignored，startup 7 passed；Clippy、fmt、2960 文件严格
行数（全部不超过 500）、diff/本轮源码 whitespace 检查通过。

v0.2.33.117：UIA 根/声明式 Button、Switch 的 GetFocus/SetFocus 与焦点属性接入
现有键盘 owner；根声明 ProviderOwnsSetFocus，先过本地权限门禁再调用 Win32。
屏外候选沿用滚动显露，聚焦不产生 click/change；提交暂停仍允许本地操作。
Host/系统调用后复核身份和权限，旧节点、busy、隐藏和 disabled 被拒绝。
系统 MTA UIA 客户端聚焦回归已接入。通知、操作模式、原生 EDIT 重定位及人工
跨进程/Narrator/混合 DPI 验收继续，不代表整个 N08 或 Windows 迁移完成。
验证：UIA 13 passed，全量库 2393 passed/28 ignored，startup 7 passed；Clippy、
fmt、2957 文件严格行数及 diff/本轮源码 whitespace 检查通过，所有文件均不超过
500 有效行。未提交、推送或打包。

v0.2.33.116：子节点实时屏幕 bounds/IsOffscreen、根点命中已接入当前 Host，
沿用 viewport/滚动/裁剪和输入 hit-test，不依赖 paint 或过期矩形。State 改为
Rc，TLS Weak 注册表前后校验快照身份，重入关闭/busy/替换拒绝旧结果；COM 不持
State 指针。系统 UIA 客户端已读真实子 bounds/offscreen。GetFocus/SetFocus、
模式/通知、EDIT 重定位与混合 DPI/Narrator 人工验收仍待完成。
全量库 2390 passed/28 ignored，startup 7 passed；Clippy、fmt、2955 文件严格
行数、diff/源码 whitespace 检查通过，所有文件均未超过 500 有效行。

v0.2.33.115：UIA 声明式只读 Fragment 导航已接入，包含父子/兄弟、稳定 runtime-id、
名称/AutomationId 和窗口根身份；同 ID 替换/关闭撤销旧节点。系统 UIA 客户端
FindFirst/TreeWalker 已读取初始树和替换后的新内容。原生 EDIT 类型不重复暴露，
但 HwndOverride 重定位尚缺。子 bounds、根命中/焦点明确未实现，模式/通知、
跨进程/Narrator 继续；不得把导航通过等同完整 UIA 合同通过。
最终 UIA 8 passed、快照/窗口生命周期 9 passed，全量库 2383 passed/28 ignored，
startup 7 passed；Clippy、fmt、2951 文件严格行数和 diff/whitespace 检查通过，
无原生进程残留。

v0.2.33.114：真实 UIA 窗口根 provider 和 WM_GETOBJECT 已接通，根身份跨场景
替换保持稳定，关闭撤销且销毁清理 UIA map。属性仅名称/framework/Pane，未知
属性委托 host，未支持模式 S_OK + NULL。针对性 5 passed，含独立 MTA 系统 UIA
客户端读取隐藏 STA HWND；仅为同进程跨 apartment 集成，不是跨进程/Narrator。
声明式 Fragment、几何/焦点、模式与通知仍缺失，不能宣称完整可访问性已完成。
最终全量库 2378 passed/28 ignored，startup 7 passed；Clippy、fmt、2948 文件
严格行数、diff/whitespace 检查通过。

v0.2.33.113：UIA 前置只读语义快照已接入 SurfaceWindow 创建、替换、关闭和
销毁生命周期。同 ID 重建隔离身份，旧句柄撤销后不能读取；跨线程无 HWND/Host
或 RefCell 引用，保留句柄不会保留旧快照。描述仅含 ID/Kind/显式 label/父索引，
不复制输入值，不提供执行授权。尚无 COM provider/WM_GETOBJECT、模式或事件，
不得宣称 UIA、屏幕阅读器、当前编辑值或离屏几何已完成，能力报告不变。
针对性 7 passed；全量库 2373 passed/28 ignored，startup 7 passed；Clippy、
fmt、2944 文件严格行数、diff/whitespace 检查通过，无原生进程残留。

v0.2.33.112：Switch 增加声明 disabled、hover、pressed 视觉反馈，使用 Neuro
控件/次级文本及唯一信号黄派生色，不改值或权限。新增 5 项 WIC/D2D 像素回归，
包含本地 bool 双向覆盖、派生色、禁用动态抑制和焦点；Switch 针对性 24 passed。
共同像素夹具按职责提取，保留 COM/画刷/裁剪恢复断言。96 DPI 离屏证据不替代
多 DPI/真实硬件验收，全局 editing 暗化与 UIA 仍待处理，能力报告不开放 switch，
未提交推送/打包。最终全量库 2366 passed/28 ignored（64.73s），startup
7 passed（15.14s）；Clippy/fmt/2941 文件严格行数/diff/whitespace 通过。
以下为此前检查点。

v0.2.33.111：Switch 成功激活先封存字段前缀，复用普通候选派发一次由内至外的
祖先 click 批次，再处理自身 change。修复无 handler 前缀被循环跳过后 change
越过 click 的问题，字段准备现在支持 fence 截止。新增 5 项 HWND 回归，Switch
针对性 19 passed；旧 JSX/jsdom 对照 3 passed。原生不复制 label/span 转发的
两次祖先 click，这是明确差异；并非真实浏览器逐事件验收。UIA/硬件/完整视觉
仍待完成，能力报告不开放 switch，未提交推送/打包。最终全量库 2361 passed/
28 ignored（60.78s），startup 7 passed（14.08s）；Clippy/fmt/测试 TS 类型检查/
2938 文件严格行数/diff/whitespace 通过。以下为此前检查点。

v0.2.33.110：Switch 鼠标和 Space 本地切换、Tab/焦点环、editing/submitting
分离、动态布尔绘制已接通。复用字段 Draft ticket、32 槽 FIFO/期限/fence 和
64 字段额度，支持权威值延后、cancel/settle/fail；先清旧合同队列再重建同 ID
会话，防止旧值投新 action 与旧失败回执清新输入。新增 9 项真实 HWND 回归，
Switch 针对性 14 passed，包含混合文本 FIFO 与暂停提交。UIA、真实硬件/视觉、
多 DPI、disabled/hover 完整样式和 DOM 祖先 click 冒泡对照未完成，正式报告
仍不开放 switch，未提交推送/打包。最终全量库 2356 passed/28 ignored（63.24s），
startup 7 passed（16.60s）；Clippy/fmt/2935 文件严格行数/diff/whitespace 通过。
以下为此前检查点。

v0.2.33.109：Switch 已有原生固定 DIP 测量与 D2D 静态轨道/滑块，复用父裁剪、
opacity、兄弟遮挡和共享画刷；布尔 change 编码严格使用 JSON bool/commit，不发
input。新增 5 项模型/真实像素/wire 回归，针对性 5 passed。尚未接本地 toggle、
编辑 FIFO、鼠标/Space、焦点/UIA 与权威值同步；不能以提交 Click 队列代替本地
编辑权限。最终全量库 2347 passed/28 ignored（61.86s），startup 7 passed
（16.05s）；Clippy/fmt/2931 文件严格行数/diff 与源码 whitespace 检查通过。
Tab 与正式能力报告未开放 switch，未宣称交互完成，未提交推送/打包。
以下为此前检查点。

v0.2.33.108：Number 的无修饰 Delete/Backspace 及 WM_CLEAR/WM_CUT 删除候选
在非组合状态进入默认 EDIT 前预检；非法结果保留原生 undo，合法结果仍交系统。
真实 EDIT 对照覆盖 Delete 单次和 Backspace 重复计数（含先删选区）的差异。
新增 4 项范围模型/真实 HWND 回归，不发送真实剪切或操作用户剪贴板；修饰键
与组合状态仍有 fallback 边界，真实剪切副作用未验。局部化、真实 IME、完整
视觉/UIA 和整体迁移未完成，number 能力报告未开放，未提交推送/打包。
最终全量库 2342 passed/28 ignored（62.73s），startup 7 passed（16.27s），
严格行数检查 2929 文件，无超过 500 有效行的文件；Clippy/fmt/diff 与新增
源码 whitespace 检查通过。以下为此前检查点。

v0.2.33.107：非组合 Number 的粘贴改为一次有界 Unicode 快照，锁外复用
EM_REPLACESEL/preflight/undo/FIFO，不双读剪贴板。前后复核当前 Number owner、
本地编辑、组合、enabled/readonly；重入编辑/选区/焦点/撤权/窗口变化使旧读取
失效，Host busy 和嵌套读取拒绝。新增 6 项解码及真实 HWND 注入快照回归，
Number 针对性 32 passed；最终全量库 2338 passed/28 ignored（63.20s），
Clippy/fmt/2928 文件严格行数/diff 通过。startup 7 passed（16.44s）。
测试不操作用户剪贴板，真实延迟渲染/转换/跨进程 owner 未验；组合中的粘贴仍
使用 fallback。局部化、真实 IME、完整视觉/UIA 与总迁移未完成，能力报告未
开放 number，未提交推送或制作发行包。以下为此前检查点。

v0.2.33.106：Number 的普通可打印 WM_CHAR/EM_REPLACESEL 已在默认 EDIT 修改
之前按完整选区候选预检，拒绝保留系统 undo，不再用回写恢复这些路径。真实
测试复现并修复了撤销记录丢失及 WM_CHAR 重复计数未展开两处边界；合法插入
仍交原生过程执行。新增 5 项回归，Number 26 passed；全量库 2332 passed、
28 ignored（61.51s），Clippy/fmt/2925 文件严格行数/whitespace 通过，startup
7 passed（17.17s）。真实 WM_PASTE、IME 等未预检路径仍不保证拒绝后 undo 无损；局部化、
完整视觉/UIA 与总迁移继续进行。能力报告未开放 number，未提交推送/打包。
以下为此前检查点。

v0.2.33.105：Number 增加普通字符预筛及编辑边沿完整 grammar 检查；非法替换
恢复旧草稿/编辑前选区，不产生输入事件，不覆盖已排队值。未完成符号/指数仍可
编辑，严格有限数事件保持 number/null；IME 留到结束验证，普通文本字段不受影响。
新增 5 项回归，Number 针对性 21 passed；全量库 2327 passed/28 ignored
（64.97s），Clippy/fmt/2924 文件严格行数/whitespace 通过。startup 7 passed（15.61s）。
整段回写恢复尚不保证撤销历史/水平滚动/caret 方向无损；真实剪贴板、IME、
局部化和浏览器逐键对照仍待验，能力报告未开放，总迁移未完成，未提交推送/打包。
以下为此前检查点。

v0.2.33.104：Input/Number/Textarea 已有原生 placeholder 提示层，空值持焦可见，
组合时隐藏，填值消失、清空恢复；提示文字不进入真实 EDIT 值、表单或事件。
使用现有 HWND property 与 Rc 生命周期，不依赖当前 classic comctl32 缺失的
GetWindowSubclass 导出，不因提示层全局切换 common-controls manifest。
新增 4 项回归，针对性 5 passed，全量库 2322 passed/28 ignored（63.91s）；
Clippy/fmt/2923 文件严格行数/whitespace 通过。startup 7 passed（16.09s）。
真实 IME、屏幕重绘/caret/多 DPI 视觉和完整样式仍待验收，Number 过滤/局部化、
长按与 UIA 未完成，能力报告仍关闭，总迁移未完成，未提交推送或打包。
以下为此前检查点。

v0.2.33.103 新增 Number 系统 up-down 鼠标按钮。它与 EDIT 分区且共用裁剪，
最终空区域禁用；通知切焦后复核身份/权限，复用原浮点步进与双边沿容量门。
系统整数 position 不回写，撤权/布局/取消/失活/尺寸/DPI 与销毁清理捕获。
4 项新真实 HWND 测试覆盖鼠标上下半区、数值事件、重入/组合/撤权、分区、
销毁及完全遮挡；全量库 2318 passed/28 ignored（62.04s），startup 7 passed
（14.92s），Clippy/fmt/2921 文件严格行数及 whitespace 通过。全量库之后仅
调整测试点坐标括号，生产源码未变，针对性 Number 测试随后重验。
真实长按重复、人工鼠标与主题视觉、placeholder、过滤/局部化、UIA 仍未完成；
number 能力报告不开放，总迁移未完成，未提交推送或制作发行包。
以下为此前检查点。

Number 已接入真实 EDIT、焦点、草稿/表单字符串和数值/null 事件双通道，以及
上下箭头 min/max/step 步进。缺省 min=max=0 沿用旧 mapper，手工越界值不被
强制钳制；异步回执、撤权、组合状态和双边沿容量继续由既有 owner 约束。
鼠标 spinner、placeholder、输入过滤/局部化、完整视觉/UIA 仍未完成，宿主报告
不宣称 number 完整支持。最终源码全量库 2314 passed/28 ignored、startup
7 passed（15.63s），Clippy all-targets/fmt/2919 文件严格行数/whitespace 通过。
新增 11 项模型与真实 EDIT 测试，包含大数非网格 max 和双边沿容量反例。
内部版本 v0.2.33.102，未提交推送或制作发行包。
以下为此前检查点。

Icon 已接入原生 DirectWrite 测量/绘制、缓存投影和 HWND 生命周期；宿主能力报告
在新建与恢复路径共同新增 icon。glyph 默认/空值、父文字继承、内容盒居中、
忽略本地 CSS/子树的规则对照旧入口实现，颜色映射规范信号黄 #d9ff38。
这不是 image 资源、number/switch/select/slider、UIA 或字体/emoji 像素完全等价
的完成证明。全量库 2303 passed/28 ignored，startup 7 passed（17.37s）；最后
纯 getter 命名调整后 Clippy all-targets、10 项 Icon 测试、fmt、2917 文件严格
行数及 whitespace 门通过。全量库/startup 是该纯命名调整前结果。
内部版本 v0.2.33.101；未提交推送或制作发行包，总体迁移继续进行中。
以下为此前检查点。

N08 已接入父 HWND 单指双轴 pan 和 Windows 系统惯性，复用缓存帧一次投影，
无自建惯性线程/定时器；取消、撤权、换场景、尺寸/DPI、失焦、关闭清旧序列。
Host 忙时丢弃当次位移，不积压补跳。4 项新针对性测试通过，覆盖真实 HWND
配置读回和解码后状态机；真实触屏、系统句柄消息投递与 EDIT 子窗口互操作
尚未实测，不能据此勾选完整触控等价性。内部版本 v0.2.33.100，未提交推送，
没有制作发行包，总体迁移继续进行中。全量库 2293 passed/28 ignored、startup
7 passed（16.98s），Clippy all-targets/fmt/2915 文件严格行数/whitespace 通过。
最终仅调整测试 raw mut 指针表达式以满足 Clippy，之后 Clippy 和 4 项针对性
测试重跑通过；全量库和 startup 是该测试表达式调整前结果，产品源码未变。
以下为此前检查点。

startup 超时诊断发现三个完整 smoke 并发时各阶段普遍变慢：单独空窗口 4.6008s，
并发空窗口/图片/pin 14.0779s/15.2523s/15.8061s 且最终均退出。正式功能回归已
增加测试进程内的子进程串行准入，保留全部场景/断言和每 child 15 秒期限；产品
运行时未变。这不是并发性能达标证明，具体底层资源瓶颈和 N11 性能验收仍未关闭。
修改后默认与 test-threads=7 两轮 startup 均 7 passed（18.40s/17.96s）；Clippy、
fmt、2913 文件有效行数和 whitespace 检查通过；本次未重跑全量库测试。
逐行计时、EXE hash 和原超时日志均保留于 linshi。内部版本 v0.2.33.100，未提交
推送或制作发行包，整体原生迁移继续进行中。以下为此前检查点。

父窗口新增无修饰方向键、PageUp/Down 和 Home/End 滚动，支持重复 keydown；沿
焦点/背景 anchor 的滚动祖先消费，hidden 轴不开放。禁提交仍可本地导航，滚动
保留逻辑焦点但废弃旧按压/候选；实际焦点在 EDIT 时不接管其光标与编辑按键。
这不是独立滚动条焦点/UIA 完成证明；触控、状态表现、性能和可见桌面等验收仍
待完成。最终源码库 2289 passed、28 ignored，Clippy/fmt、2913 文件行数与
whitespace 检查通过。startup 首次默认并行 4 passed/3 timed out；未改 15 秒上限，
串行 7 passed 后默认并行复跑 7 passed（14.26s）。首次失败证据保留，偶发超时
根因未证明、稳定性风险未关闭。内部迭代 v0.2.33.100，N08/总计划继续进行中，
未提交推送或制作发行包。
以下为此前检查点。

轨道新增按住重复分页：400ms 首次、50ms 后续，每 tick 至多一页；当前命中必须
仍为原节点/轴/方向，移出轨道或 thumb 到达指针后暂停分页，返回轨道可继续。
单槽 RAII timer 只在手势期间存活，取消链销毁且不复用 ID，旧排队消息不能驱动
新手势。暂停分页时仍有 timer 检查；Host 暂忙跳过本次，不追赶积压消息。
最终源码全量库 2283 passed、28 ignored，startup 7 passed；Clippy、fmt、2909 文件
有效行数与 whitespace 检查通过。已增加可控时序和真实 WM_TIMER 分发回归；
触控、键盘/UIA、状态表现、完整性能
与可见桌面验收仍待完成，N08/总计划继续进行中。内部迭代 v0.2.33.100，未提交
推送或制作发行包。以下为此前检查点。

原生 Surface 已增加 auto 溢出轴的覆盖式轨道/thumb、轨道单次分页与双轴拖动，
hidden/无溢出轴不显示。拖动受本地编辑门禁约束，不生成动作；取消/失焦/重排/
场景与权限变化释放捕获，普通内容拖到条上抬起也不会遗留 capture。EDIT 区域
让出条覆盖矩形，滚动保留 HWND 和草稿，复用文字布局。10 DIP 覆盖式呈现是
原生选择，不是浏览器 thin 的精确合同；轨道长按、触控、滚动条键盘/UIA、
状态表现、性能与可见桌面验收仍待完成。最终源码全量库 2279 passed、28 ignored，
startup 7 passed；Clippy、fmt、2906 文件有效行数与 whitespace 检查通过。
N08/总计划保持进行中，内部版本
v0.2.33.100；未提交推送或制作发行包。以下为此前检查点。

纯 wheel 与键盘焦点显露已复用未滚动布局快照和 DirectWrite 布局，不再测量
文字；场景/尺寸/样式重排重建缓存，失败保留旧呈现。240 次小幅往返 wheel 与
20 次焦点显露的测量计数回归通过，基准坐标投影不累积平移。仍有几何/绘制/
命中分配及 EDIT 同步，不等于完整性能验收；滚动条、触控和人工验收仍待完成。
最终源码全量库 2270 passed、28 ignored，startup 7 passed；Clippy、fmt、2900 文件
有效行数及 whitespace 检查通过。内部迭代保持 v0.2.33.100，N08/总计划仍进行中；
未提交推送或制作发行包。以下为此前检查点。

Tab/Shift+Tab 已接通屏外按钮与文本字段显露，按嵌套滚动祖先定位并同步 EDIT；
hidden 轴可程序显露而不接受 wheel。导航候选与可见动作资格分开，不可达候选
不提交滚动。EDIT 导航检查本地资格和实际焦点来源，禁提交时不因导航获得动作权。
仍缺滚动条拖动、触控惯性、高频性能与人工验收，不声明完整无障碍等价。
最终源码全量库 2267 passed、28 ignored，startup 7 passed；Clippy、fmt、2897 文件
有效行数和 whitespace 检查通过。内部迭代保持 v0.2.33.100；未提交推送或制作发行包。
以下为此前检查点。

Surface 新增双轴滚轮：auto/hidden、嵌套边界向祖先传递、偏移保留与范围钳位；
几何/文字/绘制/命中/EDIT 共用滚动坐标，EDIT 保留 HWND 与草稿。真实窗口入口
读取系统滚轮偏好并受本地编辑门禁控制，不发送 scroll action。高频滚动仍完整
重排；滚动条拖动、焦点自动滚入、触控惯性和人工验收未完成，不声明浏览器等价。
最终源码全量库 2262 passed、28 ignored，startup 7 passed；Clippy、fmt、2895 文件
有效行数检查通过。内部迭代 v0.2.33.100，未提交推送或制作发行包。以下为此前检查点。

已接通前置编辑结算后的依赖点击：激活前编辑、按钮、激活后输入按序准备，
按钮保留激活值并使用结果更新后的 revision；无前置编辑也不得被后续输入超越。
发送前复核原按钮合同/绑定/授权，30 秒期限从激活计起，失败或过期不重放。
真实 HWND 消息与合成结果回归不替代可见操作和外部服务验收，N08 仍进行中。
最终源码全量库 2253 passed、28 ignored，startup 7 passed；Clippy all-targets、
fmt、2890 文件有效行数及 whitespace 检查通过。内部迭代保持 v0.2.33.99；
未提交推送或制作发行包。以下为此前检查点。

未发文本意图加入 30 秒单调期限，准备前过期则丢弃未发依赖链、保留草稿并报告，
blur 不重放；取消归还不续期，新连续输入只可刷新被合并的队尾。已尝试/在途
证据不受排队过期改写。按钮依赖队列仍未接通，不能据此声明完整表单交互完成。
鼠标/键盘捕获激活快照同时封住编辑队尾，后续 continuous 不再跨点击边沿合并。
按钮跨前置结果等待的保存/唤醒尚未接通，未缩减原验收范围。
最终源码全量库 2247 passed、28 ignored；Surface 定向组 384 passed、1 ignored，
startup 7 passed；Clippy、fmt、2888 文件有效行数和 scoped whitespace 检查通过。
内部版本 v0.2.33.99；未提交推送或制作发行包。以下为此前检查点。

编辑 Mount 已采用完整节点合同摘要，只忽略根节点权威 value；属性、布局、样式、
可访问性、events 或子树变化都重建会话并丢弃旧意图。单纯 value 更新仍保留草稿。
这补齐相同 action 下合同变化的失效边界，不等于按钮依赖/超时/完整交互已完成。
最终源码全量库 2244 passed、28 ignored，编辑定向组 45 passed，startup 7 passed；
Clippy、fmt、2888 文件有效行数及 scoped whitespace 检查通过。内部版本仍为
v0.2.33.99；未提交推送或制作发行包。以下为此前检查点。

前置编辑失败/未知/证据不足现丢弃未发依赖意图，保留可见草稿，清理失焦 change
标记防止自动再提交；旧会话事件不得清新队列。明确未启动仍归还，身份匹配且
交付有效的 awaiting_confirmation 保留等待。按钮依赖/期限/完整合同复核仍缺。
最终源码全量库 2242 passed、28 ignored，编辑定向组 43 passed，startup 7 passed；
Clippy、fmt、2887 文件行数与 scoped whitespace 检查通过。内部版本 v0.2.33.99；
未提交推送或制作发行包。以下为此前检查点。

文本生产挂载现使用跨字段 FIFO，只有队尾相邻同字段 input 可合并，change 固定
边沿值。意图与在途预留合计上限 32；取消恢复原值/顺序，后续草稿另持 ticket。
不再按节点 ID 排序派发，也不以最新控件文字替换较早 change。按钮依赖队列、
完成/失败 barrier、期限和完整节点合同复核仍待开发，未宣告完整 Tile 等价。
最终源码全量库 2240 passed、28 ignored，startup 7 passed；Clippy、fmt、2887 文件
有效行数及 scoped whitespace 检查通过。内部版本 v0.2.33.99；未提交推送或制作
发行包。以下为此前检查点。

N08 文本 Mount 现将 events 声明作为会话身份的一部分；动作更换/增删重建字段
会话，旧草稿/phase/回执不能投向或结算新合同。单纯权威 value 更新仍复用会话。
普通 Surface fire-and-forget 与 Tile FIFO/barrier/revision 刷新合同已区分核实；
原生完整保序仍待开发，未把同 revision 拼批当成两条路径的等价实现。Loom 当前
源码要求新事件版本等于当前 attachment revision；不同 action 无统一串行保证，
因此不能通过同版本拼批替代前置执行 barrier。
最终源码全量库 2236 passed、28 ignored，startup 7 passed，动作更替定向组 2 passed；
Clippy、fmt、2885 文件行数门禁和 scoped whitespace 检查通过。内部版本仍为
v0.2.33.99；未提交推送或制作发行包。以下为此前检查点。

鼠标/键盘 Click 单槽候选已固定激活边沿的 surfaceValues，后续编辑不改旧候选
payload；场景/焦点/授权门禁保持原检查，未加入过期候选重放。input/change 与
按钮的完整调度顺序仍待推进，不能将该快照修复视为 N08 全部完成。
最终源码全量库 2234 passed、28 ignored，startup 7 passed，表单定向组 6 passed；
Clippy、fmt、2884 文件行数门禁及 scoped whitespace 检查通过。内部版本仍为
v0.2.33.99；未提交推送或制作发行包。以下为此前检查点。

N08 的 includeSurfaceValues 已接入原生 Host 鼠标/键盘按钮提交：16 字段、48KiB
UTF-8 汇总预算，payload 合并/包装及 opt-out 行为已接线；事件及冒泡批次仍遵守
64KiB wire 门禁。权威初始化、编辑及结果结算同步缓存，换绑清空；节点移除保留
已有值，与当前前端一致。可见草稿与提交缓存预算分开。完整 Unicode 截断避免
孤立代理项；极端 JS 代理项边界、离散事件顺序和真实远程验收未宣告等价。
最终源码全量库 2233 passed、28 ignored，startup 7 passed，新增定向组 5 passed；
Clippy、fmt、2884 文件有效行数与 scoped whitespace 检查通过。内部版本仍为
v0.2.33.99；未提交推送或制作发行包。以下为此前检查点。

textarea 用户 maxLength 已接入：规范化换行后按 UTF-16 计数，数值取整/裁界与
前端一致，不截断权威值，超限现有值可缩短。一次原生替换的中间 EN_CHANGE
在命令结束前合并，避免拒绝超限输入时错误回滚到中间空值。IME 超限结束后可
继续编辑。超限采用整次拒绝；选择区/撤销栈、粘贴细节、字段汇总和完整样式仍
未验证等价，report 不扩大。
本次最终源码全量库 2228 passed、28 ignored，startup 7 passed；Clippy all-targets、
fmt、2882 文件行数门禁和 whitespace 检查通过，包含四项新增隐藏 HWND 长度回归。
内部版本 v0.2.33.99；未提交推送或制作发行包。以下为此前检查点。

总计划 N08 的文本字段已接入实际 AuthorizedWindow/Host、Windows EDIT 消息和
Application 原批次调度，完整证据接纳后才结算草稿；不再只是独立编辑会话测试。
隐藏 HWND 回归覆盖连续编辑/owner 移交、激活后 binding、暂停后的 IME 收尾、
Tab/Enter、LF/CRLF、裁剪/覆盖、禁用/删除、未启动归还及撤权晚到结果。测试
结果证据为合成数据，不是实际远程服务验收。完整样式/placeholder、字段汇总、
maxLength/总量及离散边沿对照尚缺，report 仍不宣告 input/textarea 已兼容。
近期增量编号已归回 N08；N09 投送/Tile、N10 构建/发布门未被这些测试代替。
最终源码全量库 2224 passed、28 ignored；编辑定向组 31 passed、startup 7 passed；
Clippy、fmt、2881 文件行数门禁、版本与 whitespace 检查通过。内部版本 v0.2.33.99。

以下为此前检查点。

编辑会话已组合 EDIT、草稿和有界事件 ticket，并补齐隐藏 HWND 回归；缺失 phase
handler 不再释放其他 phase 的未提交值，取消组合输入不会无条件回滚已有文字。
这不是生产 Surface 挂载或真实 IME 消息验收；input/textarea 能力声明保持未完成。
最终源码全量库 2212 passed、28 ignored（含 9 项会话回归），startup 组 7 passed；
Clippy all-targets、fmt、2873 文件行数门禁通过。未提交推送或制作发行包。

生产输入门禁已拆为本地编辑和动作提交两项；owner 在后台时只软停止提交，
保留逻辑焦点，但旧候选和按压失效。已验证的新场景可保留同 ID 可聚焦节点，
禁用/删除则清理。租约撤销仍关闭窗口；未确认激活不开放编辑。Surface 定向组
341 passed、1 ignored。EDIT/IME/草稿尚未挂载生产窗口，输入节点仍不声明支持。
以下为此前检查点。

最终源码全量库 2203 passed、28 ignored；隐藏启动/保存 1 passed（4.49 秒）；
Clippy all-targets、fmt、2871 文件有效行数门禁及 diff 检查通过，完整输入迁移仍未完成。

文本输入新增 input/continuous、change/commit 事件编码及单字段异步草稿合同，
复用既有不可变事件和输出预算；真实 EDIT 测试串起读回、草稿、编码和结果回写。
生产窗口尚未接线。已核实旧提交门禁会清逻辑焦点，后续必须分离本地编辑资格
与动作提交资格，不能直接套用点击的统一 invalidate 或克隆独占 owner。
输入节点仍不声明支持。以下为此前检查点。

最终源码全量库 2198 passed、28 ignored；隐藏启动/保存 1 passed（4.18 秒）；
Clippy all-targets、fmt、2870 文件有效行数门禁及 diff 检查通过。没有真实远程
输入提交、IME 或人工可见窗口验收，不能把这些合同测试当作完整输入迁移。

文本输入已新增真实 Windows EDIT owner，验证 Unicode 选择/替换、多行、有界
读写及父子 HWND 生命周期，定向 4 passed。尚未挂载到 Surface 输入节点，未
接通 input/change/draft，也未实际验收 IME，因此不新增输入控件能力声明。
这是一项资源基础实现，不是已经可用的完整输入控件。以下为此前检查点。

本阶段全量库 2189 passed、28 ignored；随后仅显式忽略 ShowWindow 的此前可见性
返回值，重新通过 Clippy、定向 4 项和隐藏启动/保存（1 passed，5.22 秒）。fmt、
2866 文件有效行数门禁及 diff 检查通过，未作人工 IME/可见窗口验收。

Button 悬浮、鼠标/Space 按压颜色与客户区光标已接入；动态颜色保留显式样式
优先级，禁用节点不高亮，同一按钮内重复移动不触发持续重绘。生产能力报告已
增加 button 和 pointer/hover/keyboard，触屏及其他未实现节点仍不声明。
尚未完成整套新架构及人工可见窗口/屏幕阅读器验收。以下为此前检查点。

本阶段 Surface 定向组 323 passed、1 ignored；最终源码全量库 2185 passed、
28 ignored；隐藏启动/保存 1 passed（5.32 秒）；Clippy、fmt、2864 文件行数
门禁及 diff 检查通过。后续转向文本输入的真实编辑/IME/draft/事件生命周期，
不以静态排版替代可操作输入能力。

Button 已接通 Tab/Shift+Tab 焦点序列、Enter/Space 激活与键盘焦点线。键盘候选
按节点身份验证，不伪造坐标，复用原授权/版本与单槽派发链；切换焦点后即使回到
同一节点也不复活旧候选。焦点线在节点绘制顺序内，保留父透明度和兄弟遮挡关系。
Surface 定向组 319 passed、1 ignored。悬浮/按下反馈、实际可见窗口交互验收及
其他控件仍未完成，能力报告保持保守。以下为此前检查点。

本阶段全量库 2181 passed、28 ignored；隐藏启动/保存 1 passed（3.69 秒）；
Clippy all-targets、fmt、2860 文件行数门禁与 diff 检查通过。未作人工可见窗口验收。

声明式 Button 已增加原生测量、label 绘制、内容盒/默认样式及 disabled 样式。
4 项新增专项回归通过，包含真实 WIC 像素和隐藏 HWND 点击候选→事件编码链路；
disabled 不生成动作，按钮不冒泡。尚未完成键盘焦点、Enter/Space、焦点环和
悬浮/按下反馈，生产能力报告不提前声明 Button。其他表单控件、Image/Icon、
JavaScript runtime 仍待迁移。以下为此前检查点，旧测试数量不代表当前全量结果。

本阶段全量原生库 2176 passed、28 ignored；隐藏启动/保存 1 passed（5.03 秒）；
Clippy all-targets、fmt、2856 文件行数门禁与 diff 检查通过。全量测试后的源码
变化仅为移除未使用 import，随后重新通过静态检查；尚无人工可见窗口验收。

大局部 blur 已补完整绘制入口的分块/整图对照，覆盖分数多段 mask、旋转源 clip、
多个倍率和重复缓存。测试捕获并修复缩小时源像素邻域不足造成的接缝，改用设备
像素足迹及二次幂邻域桶；定向 blur 组 81 passed，全量库 2172 passed、28 ignored；
隐藏 Win32 启动/保存回归 1 passed（3.70 秒）；Clippy all-targets、fmt、2854 文件
行数门禁与 diff 检查通过。
此项不是浏览器端逐像素对照或极端缩小/资源峰值验收。

此前优先项已核实为声明式 Surface 按钮/表单控件：该检查点生产测量拒绝且能力报告
未声明，属于真实用户功能缺口。Image/Icon 及 JavaScript runtime 同样不能仅凭
协议类型判定完成。以下为此前检查点。

局部 blur 的源投影改为显式线性平滑取样；底图/擦除/普通导出仍保持默认最近邻。
插值模式由 Layout 携带，不在共用 readback 中全局切换。新增读回灰阶及生产
卷积接线回归均通过；全量库 2170 passed、28 ignored；隐藏 Win32 启动/保存
回归 1 passed（4.41 秒）；Clippy all-targets、fmt、2852 文件行数门禁和 diff 检查
通过。此项不代表浏览器缩小过滤、kernel
或全部 rasterization 已逐像素一致。以下为此前检查点。

局部 blur 已同步缩放完整笔迹遮罩及纹理，比例覆盖两步 ceil；白色覆盖排除输出
取整填充。旋转中心采用持久框，旋转源投影/效果框使用几何 clip 而非轴对齐包围框。
新增 PNG 与参数/范围测试；全量库 2168 passed、28 ignored；隐藏 Win32 启动/
保存回归 1 passed（3.86 秒）；Clippy all-targets、fmt、2851 文件行数门禁与 diff
检查通过。几何已接入不代表
kernel、Canvas rasterization/插值或分数多段路径/旋转/tile 组合全面一致。
以下为此前检查点。

工作流 blur 已使用持久效果局部框，不再对完整可见框卷积。稳定 ID 查找保护
排序身份，相同框与取整半径共享，不同框独立取样；整个效果框 clip 约束白色覆盖，
颜色 clip 只约束源投影。缺失/非法框失败，合法 JSON 转义身份兼容。
新增像素/资源/转义回归，移除 3 个旧整框实现替身测试；定向 blur 组 73 passed。
全量库 2163 passed、28 ignored；隐藏 Win32 启动/保存回归 1 passed（3.66 秒）；
Clippy all-targets、fmt、2850 文件有效行数门禁与 diff 检查通过。
分数框 mask 缩放与 rotation 中心现已由最新路径修复；kernel、Canvas 插值和
局部大纹理 tile 的组合仍未完整验证。以下为此前检查点统计。

模糊导出现在只在绘制时将有效 strength 取整，然后应用输出密度缩放；恢复后的
原始分数值保持不变。普通/现代 crop 的 PNG 回归确认 7.49→7、7.5→8，且
两个整数半径的像素不同，避免空白图误报。定向回归 1 passed；全量库
2160 passed、28 ignored；隐藏 Win32 启动/保存回归 1 passed（4.31 秒）；
Clippy all-targets、fmt、2846 文件行数门禁与 diff 检查通过。
以下坐标修复的统计为此前检查点。

模糊取样坐标已单独对齐：读取未翻转原图，现代 crop 使用 cropRect.xy 加可见
标注坐标，不套底图 contain/density 或传播内容框。新增真实颜色分区回归
2 passed；另有可见纹理尺寸/原资源保护回归。全量库 2159 passed、28 ignored；
隐藏 Win32 启动/保存回归 1 passed（4.18 秒）；Clippy all-targets、fmt、2846 文件
有效行数门禁及 diff 检查通过。
模糊纹理仅覆盖可见输出，同强度仍共享；D2D 读回与擦除复用且带预算/取消。
effect-local 源框已由最新局部路径接入；kernel 与 Canvas 插值仍未完整对齐，
不能把坐标回归当作所有模糊视觉行为已经一致。下列统计为此前检查点。

极窄旧裁剪已使用有界源/目标矩形，8192 倍放大的横纵两轴及翻转已通过真实 PNG
回归。有笔迹时先在最终输出尺寸以同一 D2D 路径物化底图，随后局部擦除；无笔迹
不增加像素副本。分数框未命中笔迹不改变边缘像素，取消不交换 Frame 或源像素。
内容像素组 13 passed；全量库 2156 passed、28 ignored；隐藏 Win32 启动/保存
回归 1 passed（3.69 秒）；Clippy all-targets、fmt、2843 文件行数门禁及 diff 检查通过。
以下统计为此前检查点。

旧式仅有 cropRect 的自然像素裁剪在上一检查点接入工作流烘焙，当时使用完整图片
仿射布局与内容 clip；该实现现已被上述有界源/目标矩形替代。此前新增
6 项像素回归，内容像素定向组 10 passed；全量库 2150 passed、28 ignored；
隐藏 Win32 启动/保存回归 1 passed（4.93 秒）；Clippy all-targets、fmt、2839 文件
有效行数门禁及 diff 检查通过。不将旧式裁剪误转换成逻辑裁剪或修改保存数据。
此前完整图片仿射布局的 1,000,000 坐标上限不再用于旧式裁剪放大；当前仅将
有界源交集和目标矩形交给 D2D。非法/退化尺寸及已有输出像素预算限制仍保留。

传播内容框已接入工作流底图烘焙：双层 contain、非裁剪整框翻转、逻辑裁剪内容
clip、目标坐标擦除与自然输出尺寸有 4 项真实 PNG 定向回归通过。无传播字段的
非等比逻辑裁剪也改为在当前 viewport 内 contain，而不是拉伸旧 contain 结果。
最新全量库回归 2144 passed、28 ignored；隐藏 Win32 启动/保存回归 1 passed
（5.17 秒）；Clippy all-targets、fmt、2839 文件有效行数门禁及 diff 检查通过。

最新协议回归 1 passed：跨 revision 的全局/来源工作流替换快照均保留图片字段。
Clippy all-targets、fmt、2837 文件行数门禁及 diff 检查通过；本轮未重跑全量库测试。

最新补充定向回归 1 passed：已授权真实文件经上游正式输出进入烘焙，目标翻转与
透明度生效且原文件未修改。Clippy all-targets、fmt、2837 文件行数门禁及 diff
检查通过。本次只新增测试，下述全量结果为上一生产实现检查点，不是本次重跑。

来源工作流保留完整图片字段是当前接收协议下的有意行为差异，不是待照搬的省略
优化。已只读核查 Loom：loom_otter_bridge::sync_workflow 不加载旧文档，直接
转换并替换 YAML；workflow_store 的 encode 仅保存本次存在的 src/previewSrc。
前端更新事件只触发刷新，没有按节点补齐省略字段。因此 ACK 缓存本身不能赋予
省略权限；未来须先有明确的接收端合并能力合同，再考虑增量图片发送。

传播状态 upstreamSourceFrame/upstreamContentFrame 已接入 Frame，底图 clip 在
绘制边框及标注之前释放。主干模糊有独立采样规则，不随传播底图位置平移；其
裁剪和翻转取样已由上述独立路径处理，effect-local 边缘仍待对齐。旧式 cropRect 缺少 sourceSize 的像素裁剪已有
单独兼容路径；底图烘焙回归不代表模糊或可见画布已全部完成。

工作流烘焙已接入保存会话到 graph_image 的来源适配，覆盖预览/来源、上游正式
输出、手动 inline 图片、中继开关与能力端口，并保留独立栅格标注层的自身 src
优先规则。像素回归覆盖不同底图、上游/手动图片和栅格层例外；未授权文件路径
被拒绝。此项不等于可见画布所有显示行为或真实 Loom 联调已验收。

归档阶段现已提前释放不再使用的烘焙快照，原文及其预算租约继续持有到暂存完成。
新增独立预算回归确认释放顺序；此项不是整机内存性能验收，也不表示已消除两次准备。

本轮补齐来源工作流采用的持久化：新连入的贴图随归档提交保存来源归属，断开
连线后的保存候选仍保留归属。新增真实文件回归覆盖写入、回读和断线重准备。
目录选择入口另增加选择前后凭证检查，拒绝把模态期间失效的选择授权给新状态。

上一全量库测试 2138 passed、28 ignored；隐藏启动/保存回归 1 passed（5.68 秒）；
Clippy all-targets、fmt、2837 文件有效行数门禁及 diff 检查通过。

应用级完整成功链路已有隔离集成证据：真实隐藏 HWND 完成原会话保存、资源
归档、revision 提交、重准备、TLS/WebSocket 发送和 ACK 消费。测试服务在回执
前读取真实磁盘会话，确认提交先于发送；客户端验证新基线和一次性消费。
这不代表已安装 Loom 或正式可见菜单验收，也不代表全部迁移完成。

## 历史检查点（由新到旧）

以下“最新”与“尚未完成”均指各自记录时的阶段；当前状态以上节为准。

最新库测试 2129 passed、28 ignored；隐藏启动/保存回归 1 passed；Clippy、fmt、
2833 文件有效行数门禁及 diff 检查通过。

正式菜单可显式选择可写归档目录并启动周期，随后当前会话的普通保存成功会
自动排队同步；可停止后续自动同步。完成报告/恢复警告已接 UI 通知。隐藏
窗口验证配置、保存合并和停止入口，但真实 Loom 完整成功链路仍未验收。

最新库测试 2128 passed、28 ignored；隐藏启动/保存回归 1 passed；Clippy、fmt、
2831 文件有效行数门禁及 diff 检查通过。

应用级 Cycle 已串起归档准备/提交、新 revision 重准备、同步和结果消费，消息
循环已服务各阶段。隐藏回归验证归档后目录撤销终止发送并保留归档结果。普通
保存自动触发、真实服务完整成功链路与全面人工验收仍未完成。

最新库测试 2128 passed、28 ignored；隐藏启动/保存回归 1 passed；Clippy、fmt、
2829 文件有效行数门禁及 diff 检查通过，仍非完整用户功能验收。

归档已具备后台原会话暂存与 UI Receipt 复核提交，成功后交换保存基线；隐藏
窗口回归覆盖成功与迟到编辑拒绝。尚未启用普通保存后的自动归档及新 revision
重准备/发送调度，不能把阶段提交 API 当作完整自动同步已完成。

最新库测试 2126 passed、28 ignored；Clippy、fmt、2825 文件有效行数门禁及
diff 检查通过。真实临时文件归档回归不替代正式 UI 工作区验收。

工作流多资源批次已接通单资源发布、索引转换与下一 revision 会话候选；测试
用真实 Staged 新文件提交验证候选落盘。正式 UI 的原会话替换、基线交换和
自动同步仍未接线，失败留下已发布资源的边界保持明确。

最新库测试 2124 passed、28 ignored；Clippy、fmt、2822 文件有效行数门禁及
diff 检查通过。单资源测试真实写盘，但不等于完整工作流归档/同步验收。

受管资源 owner 已能实际发布编码图片/inline 图片，复用主干命名、无覆盖提交、
同名内容核验与句柄保护。此为单资源能力，工作流多资源批次、耐久会话提交及
保存后自动同步仍未接通。

本轮库测试 2120 passed、28 ignored；隐藏启动/保存回归 1 passed；最终索引
解析优化后专项 2 passed，Clippy、fmt、2820 文件行数门禁及 diff 检查通过。

完成结果现在保留 Receipt，领取后可复核而不丢部分成功报告；归档索引候选
已实现主干的命中工作流整体替换语义。资源文件发布、耐久会话提交和保存后
自动发送仍未接通，不能把候选索引当作已完成归档。

本轮库测试 2118 passed、28 ignored；真实隐藏启动/保存回归 1 passed；Clippy、
fmt、2817 文件有效行数门禁和 diff 检查通过，尚未完成自动同步及归档验收。

同步 worker 已具有单槽逐条 UI 准入握手；应用 Candidate 可启动同步 owner，
service 复核 Receipt 后应答，完成保留部分成功报告。真实 TLS 回环覆盖批准、
拒绝及等待准入时销毁任务。正式消息循环已处理当前 Record 的握手和完成结果；
保存后的自动触发、结果消费及资源归档提交仍未接通。

最新完整库测试 2115 passed、28 ignored；Clippy、fmt、2813 文件有效行数门禁
和 diff 检查通过。批次回环测试不替代正式 UI 调度和用户人工验收。

批次编码与单条交换已通过顺序 dispatch 接通，支持每条连接后的准入、部分成功
计数、失败保留及不自动重试。正式 UI 调度/逐条准入握手/归档提交仍未接线。

本轮库测试 2113 passed、28 ignored；Clippy 等价整理后，最终交换专项 2 passed，
Clippy、fmt、2811 文件行数门禁及 diff 检查通过；本轮未重新执行 HWND 启动回归。

已新增单条工作流控制请求的实际 TLS/WS 交换与严格回执匹配，并覆盖真实回环
发送、Ping/Pong、无关请求过滤、错误工作流、远端拒绝和取消。它仍未接入正式
应用调度及资源归档；不能据此宣称用户端到端同步已完成。

最新验证：2111 项库测试通过、28 项 ignored；隐藏启动/保存集成 1 项通过；
Clippy、fmt、2809 文件行数门禁和 diff 检查通过。此结果不代表全面人工验收。

工作流准备结果已接通惰性批次：全局优先、来源逐条编码，最多持有一份带预算
租约的消息；重试复用 ID/字节，匹配确认才推进。真实隐藏保存链覆盖第一条
请求生成及丢弃回收。尚未接通实际发送、远端确认验证或资源归档提交。

全局和来源工作流已支持真实 Loom 控制请求编码（loom.hook.workflow.sync），
区别于前端 action payload；整个 envelope 有界、可取消并校验 requestId。最终
库测试 2109 passed、28 ignored，真实隐藏启动/保存集成 1 passed；Clippy、fmt、
2807 文件行数门禁及 diff 检查通过。未实际发送或验证远端 ACK，整体迁移继续。

### 前一检查点：应用保存凭证

Application 已有基于已提交保存状态的工作流准备/取消/领取入口，单槽由会话拥有；
领取后仍保留工作区、来源和能力目录凭证。大段会话复制在 worker 中预算准入。
最终库测试 2107 passed、28 ignored，真实隐藏启动/保存集成 1 passed；Clippy、
fmt、2805 文件行数门禁及 diff 检查通过。用户触发/自动调度、归档与授权发送未
接通，整体迁移未完成；本轮未制作手测发行包或发布。

### 前一检查点：后台任务

工作流准备已有真实后台任务入口，整合图片合成、参数过滤和来源计划，支持精确
来源匹配、显式根目录准入、取消/通知/消费/回收。最终串行测试 2106 passed、
28 ignored；Clippy、fmt、2803 文件行数门禁及 diff 检查通过。应用调度与授权发送
尚未接入，任务成功不等于保存提交或同步成功；整体迁移未完成，无新 EXE 或发布。

### 前一检查点：遗留旋转字段

遗留 imageEditState.rotation 已按主干兼容：不拒绝、不额外旋转、不改写原会话。
当前主干合成器未消费该字段，不能把新增整图旋转当作迁移前置；标注旋转另走
既有路径。非对称图真实 PNG 回归覆盖四种遗留值。最终测试 2102 passed、28 ignored；
Clippy、fmt、2801 文件行数门禁及 diff 检查通过。完整同步链和运行验收仍未完成。

### 前一检查点：美化包装

工作流美化包装已支持留白、七种背景、圆角白卡片和下移阴影，保持节点逻辑尺寸，
并接入真实 PNG 输出。预算与取消保护已覆盖。最终串行测试 2101 passed、28 ignored；
Clippy、fmt、2801 文件行数门禁及 diff 检查通过。浏览器视觉/人工验收、旋转及
完整同步链仍未完成；本轮无新 EXE 或发布操作。

### 前一检查点：自然源模糊

自然源模糊资源已接入工作流烘焙：双轴密度卷积、逻辑框映射、同强度共享和导出
快照隔离已有实际 PNG 回归。同时修复拉伸裁剪的输出尺寸与 contain 坐标。最终
串行测试 2096 passed、28 ignored；Clippy、fmt、2798 文件行数门禁及 diff 检查
通过。未完成浏览器逐像素/人工验收，美化、旋转及完整同步链仍待完成；本轮无新 EXE 或发布。

### 前一检查点：纹理映射

模糊纹理现支持独立逻辑采样框，整图与大图分块绘制均使用双轴映射，像素共享和
固定网格缓存保留。真实 PNG/WIC 对照已覆盖偏移、非等比尺寸及分数缩放。最终
串行测试 2092 passed、28 ignored；Clippy、fmt、2797 文件行数门禁和 diff 检查
通过。工作流自然源效果资源准备仍未接通，整体迁移未完成；本轮无新 EXE 或发布。

### 前一检查点：双轴卷积

源密度双轴模糊卷积入口已实现，独立缩放两轴标准差，有界核预算及取消；旧入口
保留原强度合同。新增独立参考与兼容测试，最终串行测试 2090 passed、28 ignored；
Clippy、fmt、2797 文件行数门禁及 diff 检查通过。该内核尚未接入工作流效果资源，
不能宣称源分辨率模糊功能已完成。整体迁移继续，本轮无新 EXE 或发布操作。

### 前一检查点：批量解析

批量工作流烘焙已复用内容回调解析的节点，移除逐张图片重新解析完整会话的开销；
单张公共入口保留 UnitId 查找校验。新增真实多图像素回归保护透明度/圆角/身份
隔离。最终串行测试 2087 passed、28 ignored；Clippy、fmt、2796 文件行数门禁和
diff 检查通过。尚未测量实际性能收益；整体迁移仍未完成，本轮无新 EXE 或发布操作。

### 前一检查点：历史擦除

历史 contentEraseStrokes 已接入工作流自然底图烘焙，处理裁剪偏移、contain 映射
及缩放笔宽，后绘制边框和独立标注不被擦除。共享源保持不变；路径数量、内存与
取消均有边界。交互橡皮擦的 4–96 约束保持不变。最终串行测试 2086 passed、
28 ignored；Clippy、fmt、2796 文件行数门禁及 diff 检查通过。美化、源分辨率
模糊与完整同步链仍未完成。本轮无新 EXE、提交、推送或合并。

### 前一检查点：圆角

工作流源分辨率烘焙新增圆角蒙版，覆盖底图、边框与独立标注，支持裁剪偏移和
过大半径缩放。蒙版层独立计入像素预算，在 EndDraw 前释放；真实 PNG 像素及
预算失败/成功释放均有回归。最终串行测试 2082 passed、28 ignored；Clippy、fmt、
2795 文件行数门禁及 diff 检查通过。美化、历史内容擦除、源分辨率模糊和完整
同步链仍待完成，整体迁移未完成。本轮无新 EXE、提交、推送或合并。

### 前一检查点：边框

工作流源分辨率烘焙新增边框支持：保持逻辑线宽、源密度缩放、半线宽内缩，
底图透明度不影响边框，后绘制标注覆盖边框。未知颜色/无效宽度拒绝，不访问外部
资源。真实 PNG 像素与输入回归通过。完整串行测试 2078 passed、28 ignored；
Clippy、fmt、2794 文件行数门禁及 diff 检查通过。圆角等效果与应用调度、归档、
授权发送仍未完成；本轮无新 EXE、提交、推送或合并。

### 前一检查点

真实原生烘焙已接入恢复候选的工作流内容准备，返回快照同时持有图片编码预算，
支持参数过滤及来源视图复用。继续编辑、草稿、未支持效果、缺失能力或取消均拒绝
准备；尚未接入应用调度、资源归档提交和授权发送，不代表同步功能已经端到端完成。
完整串行测试 2076 passed、28 ignored；Clippy、fmt、2793 文件行数门禁及 diff
检查通过。整体迁移未完成，无新 EXE、提交、推送或合并。

## 历史检查点

独立 source-resolution PNG 合成及恢复候选入口已实现：自然尺寸/裁剪密度、
小数逻辑坐标、contain placement 和底图透明度独立处理，不改变普通导出。候选
继续编辑后拒绝旧源，未支持的外框/美化/模糊效果明确失败。工作流最终接线与
授权发送仍未完成。最终源码完整串行测试 2073 passed、28 ignored；Clippy、fmt、
2791 文件行数门禁及 diff 检查通过。整体迁移未完成，本轮无新 EXE、提交、推送
或合并。以下为历史记录。

来源采用计划与 workflowAssetArchiveHints 已有纯数据准备入口，精确源摘要绑定，
不修改活动文档、不冒充资源索引落盘。现有逻辑分辨率导出不能直接用于同步的
source-resolution 烘焙；自然源/裁剪密度/透明度等接线仍需独立实现。最终源码验证：
2068 passed、28 ignored；Clippy、fmt、2788 文件行数门禁及 diff 检查通过。
整体迁移未完成，本轮无新 EXE、提交、推送或合并。以下为历史记录。

工作流快照现有统一内容策略入口：匹配 capability 过滤参数，缺失/重复能力拒绝；
显式识别贴图烘焙条件和缩略尺寸，失败不回退原底图。直接图片遵循 filePath/
previewSrc 互斥规则。实际原生烘焙调用方、调度和授权发送仍未接通。最终源码验证：
2066 passed、28 ignored；Clippy、fmt、2786 文件行数门禁及 diff 检查通过。整体
迁移未完成，本轮无新 EXE、提交、推送或合并。以下为历史记录。

来源工作流已有连通分量选择、节点/连线身份重映射及借用全局大字段的 payload
视图。同来源断开分量合并，身份冲突整批拒绝，原会话 SHA-256 防止同版本串用。
应用来源更新、归档、内容准备与授权发送仍未接通。最终源码完整串行测试
2063 passed、28 ignored；Clippy、fmt、2784 文件行数门禁及 diff 检查通过。
整体迁移未完成，本轮无新 EXE、提交、推送或合并。以下为历史记录。

全局 hook-live 工作流快照映射与有界 payload 编码已实现，匹配指定保存版本，
参数/图片由显式内容回调提供，不自动复制私有原文。当前仅合同准备模块，尚未
接入应用、连通分量同步、烘焙/参数过滤调用方或授权发送。完整串行测试 2060 passed、
28 ignored；Clippy、fmt、2782 文件行数门禁及 diff 检查通过。整体迁移未完成，
本轮无 EXE、提交、推送或合并。以下为历史记录。

内联图片输出已在真实 Candidate 中恢复为正式 Image 端口，重复 Data URL 共享
像素；已有预览优先，否则从正式图片选择初始显示。外部 URL/路径/描述符仍为
纯数据，不恢复执行请求或 Surface 权限。远端资源解析、自动同步与远端工作流
同步仍待完成。最终源码完整串行测试 2057 passed、28 ignored；Clippy、fmt、
2780 文件行数门禁及 diff 检查通过。无新 EXE、提交、推送或合并，整体迁移未完成。
以下为历史记录。

正式图片端口现由保存 worker 编码为 PNG Data URL，与标量/描述符整体保存；
UI 快照不复制像素，嵌套编码字符串预算随 WindowCandidate 保留，失败不输出部分
候选。图片字符串恢复为正式图片端口、自动调度和远端同步仍待完成。
最终源码验证：2054 passed、28 ignored；Clippy、fmt、2778 文件行数门禁及 diff
检查通过。整体迁移未完成，本轮无新 EXE、提交、推送或合并。
以下为历史记录。

正式 JSON 输出现可经会话 Candidate 恢复至 Art 结果 owner，空结果可再次保存；
不恢复旧请求、Surface 写入资格或远端资源权限。重复键/非法名称/超额端口拒绝
整批结果准备。图片型输出资源解析、自动调度和远端工作流同步仍未完成。
本增量最终验证：2051 passed、28 ignored；Clippy、fmt、2777 文件行数门禁及
diff 检查通过。未生成新 EXE、提交、推送或合并，整体迁移未完成。
以下为历史记录。

正式 JSON 端口现已进入会话保存快照，包括 Surface 的 resource/stream 描述符；
整体替换旧 outputs，空正式结果清除历史端口，普通执行开始/取消不恢复旧数据。
复用已有结果凭证拒绝后台编码期间过期的保存候选。图片混合端口编码、恢复结果、
自动调度和远端工作流同步仍未完成。以下为历史记录。

本增量最终验证：2046 passed、28 ignored；Clippy、fmt、2775 文件有效行数门禁及
git diff --check 通过。未生成新 EXE、提交、推送或合并。

新挂载窗口现有一次性节点结果注册，普通交付不能首次绑定或换绑。注册前刷新
窗口授权，刷新后再借用并复核应用/节点状态；失败可重试，成功后不能因普通
Loom 执行撤销资格而再次注册。新增 3 项回归，完整串行库测试 2043 passed、
28 ignored；Clippy、fmt、2774 文件行数门禁及 diff 检查通过。完整重挂载交互、
工作流同步和真实端到端验收仍待完成，整体迁移未完成。以下为历史记录。

Surface 新结果已接入既有图传播，释放结果借用后通知，同轮每节点去重，旧/
重复版本不传播，无新结果不分配通知缓存。完整串行测试 2040 passed、28 ignored；
Clippy、fmt、2773 文件行数门禁通过。工作流同步、重挂载
换绑及真实端到端验收仍待完成。整体迁移未完成，无新 EXE、提交、推送或合并。
以下为历史记录。

正式输出已接入应用 Art 节点交付，重查服务/窗口授权与版本，Commit 保留用于
恢复，输入按已应用结果版本解锁。旧附件不能借交付换绑，普通 Loom 撤销后
不自动重绑。新增 2 项回归并扩充互斥断言，完整串行测试 2040 passed、28 ignored；
Clippy、fmt、2773 文件行数门禁通过。传播/同步、重挂载换绑和端到端
验收尚待完成。整体迁移未完成，无新 EXE、提交、推送或合并。以下为历史记录。

Results 已有 Surface 显式绑定、单调版本与原子端口替换，空 map 清旧端口，
旧附件或普通 Loom 执行期间的提交不能覆盖节点。新增 3 项回归，完整串行测试
2038 passed、28 ignored；Clippy、fmt、2770 文件行数门禁通过。应用授权
交付和传播/同步尚待连接，整体迁移未完成，无新 EXE、提交、推送或合并。
以下为历史记录。

Surface Commit 已有与现有前端一致的节点值适配，解包三类端口并保留空值和
描述符扩展字段，不触发下载/订阅。新增 3 项回归，完整串行测试 2035 passed、
28 ignored；Clippy、fmt、2769 文件行数门禁通过。节点
原子应用、版本管理与传播/同步尚待连接。整体迁移未完成，无新 EXE、提交、
推送或合并。以下为历史记录。

生产 pointer 已从始终禁用改为按服务/授权/可见 owner/容量和未结算证据条件
启用；在途、模态、退出与撤权关闭输入。相同状态保留候选，实际切换清除候选。
新增 3 项回归，完整串行测试 2032 passed、28 ignored；Clippy、fmt、2768 文件
行数门禁通过。输出应用、异常收据处理和完整端到端交互尚待完成。整体迁移
未完成，无新 EXE、提交、推送或合并。以下为历史记录。

窗口点击候选已接入应用批次调度，容量和结果预留先于 owner 移交，启动失败
完整返还，退出泵不启动新输入。新增 3 项事务回归；完整串行测试 2029 passed、
28 ignored，Clippy、fmt、2768 文件行数门禁通过。生产 pointer 开关仍关闭，
授权输入生命周期和端到端运行尚待完成。整体迁移未完成；无新 EXE、提交、
推送或合并。以下为历史记录。

显式接续菜单与应用入口已连接，候选和执行分别审核当前窗口/服务/授权，
只排队原批次 ID，明确操作后才启动接续；不自动继续或重放前缀。新增菜单
HWND 回归并扩充 owner 审核断言。完整串行测试 2026 passed、28 ignored；
Clippy、fmt、2766 文件行数门禁通过。真实交互及生产点击仍待验证/连接，整体迁移
未完成；无新 EXE、提交、推送或合并。以下为历史记录。

显式接续收据选择事务已完成，按原批次 ID 处理，取消原位保留，启动失败返还
完整 owner 与批次证据。新增 3 项回归覆盖选择/取消/任务转换和启动失败。
最终源码完整串行测试 2025 passed、28 ignored；Clippy、fmt、2765 文件行数门禁通过。
原生菜单接线及生产点击仍待完成，整体迁移未完成；无新 EXE、提交、推送或
合并。以下为历史记录。

接续任务已接入应用共享容量、完整批次预算、等待/取消/退出收割与既有恢复
记录，三类批次统一预留完成缓存。新增 3 项集合回归覆盖关闭/容量/混合完成。
完整串行测试 2022 passed、28 ignored；Clippy、fmt、2764 文件行数门禁通过。
显式接续选择与菜单入口尚待连接，生产点击默认禁用；整体迁移未完成，无新
EXE、提交、推送或合并。以下为历史记录。

显式接续已有后台 worker：只发送未尝试后缀，原前缀 ACK/决定/输出保留，
新结果追加；任务错误和交付权限独立返还。新增 2 项 TLS/线程回归覆盖 HTTP
失败和撤权。完整串行测试 2019 passed、28 ignored；Clippy、fmt、2762 文件
行数门禁通过。接续应用任务集合与显式入口尚待连接，生产点击默认禁用；整体
迁移未完成，无新 EXE、提交、推送或合并。以下为历史记录。

显式后缀接续已有成功前缀/未尝试后缀审核与整批 owner 移交接口，失败完整
返还，不发送或重放事件。新增 2 项 HWND 回归覆盖尝试边界与撤权；完整串行
测试 2017 passed、28 ignored，Clippy、fmt、2760 文件行数门禁通过。
接续 worker、应用入口和生产点击仍待连接。整体迁移未完成；无新 EXE、提交、
推送或合并。以下为历史记录。

批次确认已接入现有菜单与 Win32 弹窗，默认取消，提示前后复核授权/期限，
模态返回后审核退出和服务切换，再准备决定与移交 owner；失败保留证据。
新增 2 项 HWND/注入选择器回归，最终源码完整串行测试 2015 passed、28 ignored；
Clippy、fmt、2758 文件行数门禁通过。实际 MessageBox 交互尚待人工验收。生产点击
仍默认禁用，显式后缀接续及整体迁移未完成；无新 EXE、提交、推送或合并。
以下为历史记录。

批次收据选择与决定启动事务已实现：统一提示证据审核，候选选择跳过不可用
窗口，取消完整返还，启动失败保留未提交决定。新增 3 项回归覆盖选择、容量、
取消及完整收据转任务。最终源码完整串行测试 2013 passed、28 ignored；
Clippy、fmt、2757 文件行数门禁通过。原生弹窗/菜单及显式后缀接续仍待连接，生产点击默认
禁用，整体迁移未完成；无新 EXE、提交、推送或合并。以下为历史记录。

批次决定任务已接入应用集合的共享预算、正常/退出等待、取消和收割，成功
回填沿用批次交付/恢复记录；异常回填完整保留并阻止不完整退出 bundle。
新增 3 项回归后完整串行测试 2010 passed、28 ignored；Clippy、fmt、2756
文件行数门禁通过。确认 UI、收据选择事务和显式后缀接续尚未连接，生产点击
默认禁用。整体迁移未完成；无新 EXE、提交、推送或合并。以下为历史记录。

批次决定结果已按原索引回填，保留其他事件、唯一 owner 和未提交后缀；窗口
呈现独立核查决定接收/身份/版本，恢复记录保留脱敏决定提示。新增 2 项并扩充
TLS 回归后，完整串行测试 2007 passed、28 ignored；Clippy、fmt、2754 文件
行数门禁通过。应用批次决定任务集合、确认 UI 与显式后缀接续尚未完成，生产
点击默认禁用。整体迁移未完成；无新 EXE、提交、推送或合并。以下为历史记录。

批次选中项决定已有后台任务，完整收据/owner 返还，提交/收敛/授权分离，
只更新目标 ACK，不重发点击或提交后缀。新增 TLS/线程回归后完整串行测试
2005 passed、28 ignored；Clippy、fmt、2753 文件行数门禁通过。结果合并、
决定元数据、应用调度和点击启用未完成；无新 EXE、提交、推送或合并。
以下为历史记录。

批次明确选择后的 owner 移交已实现，完整收据与决定随输入持有，失败原样
返还；已尝试原事件仅可用于决定，不允许重发点击或重复决定。新增/扩充
回归后完整串行测试 2003 passed、28 ignored；Clippy、fmt、2751 文件行数
门禁通过。批次决定 worker、UI 接续与点击调度未完成；无新 EXE、提交、推送
或合并。以下为历史记录。

批次待确认项已有窗口审核/显式选择准备接口，成功时完整收据与决定一起
持有，失败完整返还，不发送或移交 owner。新增 2 项 HWND 回归后完整串行
测试 2002 passed、28 ignored；Clippy、fmt、2750 文件行数门禁通过。批次
接续任务、UI 和点击调度仍待连接；无新 EXE、提交、推送或合并。以下为历史记录。

批次恢复元数据已接入正常退出 bundle，逐事件证据与批次顺序/整体呈现分别
保存，缺项/错序/矛盾记录拒绝，旧记录仍兼容读取。新增/扩充回归后最终完整
串行测试 2000 passed、28 ignored；Clippy、fmt、2748 文件行数门禁通过。
批次批准后恢复、点击调度及完整迁移验收仍待完成，点击默认禁用；没有新
EXE、提交、推送或合并。以下为历史记录。

批次已接入应用任务集合、等待/退出泵、取消和窗口结算，按事件数共用 32 条
未保存预算，拒收及收据有界保留。批次恢复日志尚未适配，有批次证据时明确
拒绝生成遗漏记录的 bundle。新增 2 项回归后完整串行测试 1998 passed、
28 ignored；Clippy、fmt、2747 文件行数门禁通过。批准后恢复、日志和点击
调度仍待完成；无新 EXE、提交、推送或合并。以下为历史记录。

整批结果已有窗口接纳与 BatchReceipt：恢复 owner 不等于呈现，全部 ACK/
观察/版本/授权通过才替换场景；部分完成或待确认保留证据而不呈现。新增
3 项 HWND 回归后完整串行测试 1996 passed、28 ignored；Clippy、fmt、2745
文件行数门禁通过。批次批准后恢复、应用集合/缓存/退出日志未完成，点击
默认禁用；无新 EXE、提交、推送或合并。以下为历史记录。

批次唯一 owner 移交已连接窗口授权/版本检查和完整有序事件 ID 校验，错序/
缺项/单事件结果不能冒领，返还不更新场景。新增 3 项回归后完整串行测试
1993 passed、28 ignored；Clippy、fmt、2743 文件行数门禁通过。批次批准后
恢复、应用集合和整批窗口交付未完成，点击默认禁用；无新 EXE、提交、推送
或合并。以下为历史记录。

批次待确认项已读取一次提示，分别保存提示/收敛/最后 ACK，读取共用批次
截止且最多 3 秒，不自动批准或提交后缀。新增 TLS 与截止回归后完整串行
测试 1990 passed、28 ignored；Clippy、fmt、2742 文件行数门禁通过。批次
批准后恢复、应用集合与窗口交付未完成，点击默认禁用；无新 EXE、提交、
推送或合并。以下为历史记录。

批次 worker 已连接逐项收敛，共享 25 秒截止，分开保存提交/观察证据，各项
保留 ACK/结果/commit，只缓存最高版本的一份场景。新增/扩充 TLS 回归后
完整串行测试 1988 passed、28 ignored；Clippy、fmt、2742 文件行数门禁
通过。批次确认恢复、应用集合和窗口交付尚未完成，点击默认禁用；没有新
EXE、提交、推送或合并。以下为历史记录。

点击批次提交已有后台 Job，唯一 owner 与原事件完整返还，共用容量、取消和
等待机制；迟到撤权保留 ACK，单项异常不丢之前 ACK。新增线程/真实 TLS
回归后完整串行测试 1987 passed、28 ignored；Clippy、fmt、2741 文件行数
门禁通过。批次收敛/确认恢复/应用集合尚未接入，点击默认禁用，整体迁移未
完成。无新 EXE、提交、推送或合并。以下为历史记录。

点击冒泡新增有界顺序 HTTP 批次提交接口，整批预检、共享 10 秒预算、连续
ACK 保留，遇到确认/失败停止且未提交后缀不尝试。新增本地真实 TLS 与身份
预检回归后，完整串行测试 1985 passed、28 ignored；Clippy、fmt、2739 文件
行数门禁通过。批次后台收敛/确认/应用调度尚未接入，点击仍默认禁用，完整
迁移未完成；没有新 EXE、提交、推送或合并。以下为历史记录。

确认菜单候选已连接当前窗口审核，关闭窗口/旧场景的提示不会挡住后续有效
提示，窗口调用不持有授权 View 借用且发布前复核服务。新增回归后完整串行
测试 1983 passed、28 ignored；Clippy、fmt、2737 文件行数门禁通过。真实
服务交互、点击启用和完整迁移仍未验收；没有新 EXE、提交、推送或合并。
以下为历史记录。

确认候选与实际调度已统一有效性检查，过期/失败交付的旧收据不会挡住后续
有效提示，旧菜单选择在 UI 前按新时刻重验，历史证据不删除。新增 2 项回归
后完整串行测试 1982 passed、28 ignored；Clippy、fmt、2737 文件行数门禁
通过。真实服务交互及完整迁移仍未验收，点击默认禁用；没有新 EXE、提交、
推送或合并。以下为历史记录。

原生确认框已连接显式授权菜单与应用调度：事件 ID 排队、共享模态互斥、
服务/设备校验、返回后的退出/新请求检查及决定任务启动。取消保留提示，
选择后的中止保留 NotStarted 证据。新增菜单回归后完整串行测试 1980 passed、
28 ignored；Clippy、fmt、2737 文件行数门禁通过。真实服务确认交互尚未人工
验收，点击仍默认禁用，整体迁移未完成；无新 EXE、提交、推送或合并。
以下为历史记录。

选中收据的确认调度事务已实现：UI 前检查容量/关闭并预留结果缓存，取消
返还收据，启动失败保留 NotStarted 和完整输入到统一结果队列。新增 2 项
回归后完整串行测试 1979 passed、28 ignored；Clippy、fmt、2736 文件行数
门禁通过。应用用户入口/模态/退出到该事务的调用仍未连接，整体迁移未完成。
未生成 EXE、提交、推送或合并；以下为历史记录。

原收据到决定任务 Input 的所有权移交已实现：成功沿用原事件/ACK 并移交唯一
owner，失败完整返回收据和决定，拒绝可能丢失其他证据的收据形态。扩充 HWND
回归后完整串行测试 1977 passed、28 ignored；Clippy、fmt、2736 文件行数
门禁通过。应用调度和真实确认框手测仍待接入；未启用点击或自动批准，没有
新 EXE、提交、推送或合并。以下为历史记录。

原生确认框入口已实现：Surface HWND owner、默认取消、显式是/否、返回后
重新授权与检查期限，远端文本转义并限制显示长度。新增 2 项文本回归，扩充
HWND 选择/取消/过期验证；最终源码 1977 passed、28 ignored，Clippy、fmt、
2735 文件行数门禁通过。应用模态/退出/任务调度及真实对话框手测仍待完成。
没有自动批准，点击仍默认禁用；未生成 EXE、提交、推送或合并。以下为历史记录。

显式选择后的 prepare_decision 已接入原收据提示消费：准备失败保留提示，成功
保留 ACK 且不移交 owner、不执行网络请求，重复准备拒绝。扩充 HWND 回归后，
最终源码完整串行回归仍为 1975 passed、28 ignored；Clippy、fmt、2734 文件
行数门禁通过。原生确认对话框和应用交互调度仍未连接，没有新增 EXE 或发布。

最新最终源码完整串行回归 1975 passed、28 ignored；Clippy、fmt、2734
文件行数门禁及 diff 检查通过。决定结果已统一进入事件窗口交付、有界收据
和正常退出恢复记录；移除独立决定结果队列，保留提交、ACK、收敛、交付及
决定元数据。窗口另验决定已接收、身份和当前版本，撤权或未确认不能更新场景。
新增 4 项窗口/应用结算/恢复证据回归通过。

原生确认 UI 与应用发起决定的交互入口、输出消费、点击批次调度、运行中
日志和整体验收仍待完成。没有自动批准路径，点击入口默认禁用。
无新 EXE、提交、推送或合并。以下为历史记录。

最新最终源码完整串行回归 1971 passed、28 ignored；Clippy、fmt、2733
文件行数门禁及 diff 检查通过。应用决定任务已接入等待/收割/取消/退出泵，
与点击共用未保存动作预算；完成证据拒收保留。正常退出 bundle 已包含决定
owner 与选择/尝试/接收元数据，不含确认 payload。新增 4 项回归及 TLS 扩充通过。

窗口决定结果接纳/收据转换、原生确认 UI、输出消费、点击批次调度、运行中
日志和整体验收仍待完成。应用没有自动批准路径，点击入口默认禁用。
无新 EXE、提交、推送或合并。以下为历史记录。

最新最终源码完整串行回归 1967 passed、28 ignored；Clippy、fmt、2731
文件行数门禁及 diff 检查通过。决定后台 Job 已封装单次提交和原动作收敛，
独立保留提交/收敛/ACK/授权证据，复用共享容量并完整返还启动失败输入；
窗口与 Activation 已有唯一 owner 移交入口。新增 3 项回归及 HWND 扩充通过。

应用决定任务集合、收据/恢复记录和原生确认 UI 仍待接入，没有自动批准路径。
输出消费、点击批次调度、运行中日志和整体验收未完成，点击入口默认禁用。
无新 EXE、提交、推送或合并。以下为历史记录。

最新最终源码完整串行回归 1964 passed、28 ignored；Clippy、fmt、2729
文件行数门禁及 diff 检查通过。已有绑定显式选择的单次确认决定 POST，以及
决定 HTTP 确认后的原动作 GET 收敛接口；成功不重放点击，失败不重发决定。
新增 4 项准备/真实 TLS/截止与取消回归通过。

原生确认 UI、后台决定任务及其恢复证据仍待接入，应用没有自动批准路径。
输出消费、点击批次调度、运行中日志和整体验收尚未完成，点击入口默认禁用。
无新 EXE、提交、推送或合并。以下为历史记录。

最新最终源码完整串行回归 1960 passed、28 ignored；Clippy、fmt、2726
文件行数门禁及 diff 检查通过。后台 Job 已在等待确认后读取一次提示，保留
ACK 和独立读取结果并传递到窗口收据；新增展示前当前授权/版本/到期复核。
新增 HWND 回归及真实 TLS 确认成功、失败、活跃取消场景扩充通过。

原生确认 UI、决定提交、确认后收敛恢复、输出消费与点击批次调度仍待完成。
没有自动批准或重放动作，点击入口默认禁用。整体迁移及真实 Loom/完整交互/
性能未验收。无新 EXE、提交、推送或合并。以下为历史记录。

最新最终源码完整串行回归 1959 passed、28 ignored；Clippy、fmt、2726
文件行数门禁及 diff 检查通过。确认提示已有完整事件/请求/附件身份、风险、
有效期与容量校验，并通过一次设备过滤流 GET 读取；保留最新 ACK，批次失败
或提示被替换均可拒绝交付。新增 5 项回归通过。

后台确认阶段、原生确认 UI、决定提交、输出消费与点击批次调度仍待接入。
未发送批准或重放动作，点击入口默认禁用。整体迁移及真实 Loom/完整交互/
性能未验收。无新 EXE、提交、推送或合并。以下为历史记录。

最新最终源码完整串行回归 1954 passed、28 ignored；Clippy、fmt、2722
文件行数门禁及 diff 检查通过。result commit 的接纳和类型读取共用 outputs
解析器：借用大 value JSON、严格资源/流描述符、显式 null 与缺失值区分、
解码后端口名去重。新增 4 项回归通过。

资源授权读取、本地图应用、确认交互、点击批次调度、恢复交互和运行中日志
仍待完成；类型有效不代表资源已获授权或输出已经应用。点击入口默认禁用，
整体迁移及真实 Loom/完整交互/性能未验收。无新 EXE、提交、推送或合并。
以下为历史记录。

最新最终源码完整串行回归 1950 passed、28 ignored；Clippy、fmt、2720
文件行数门禁及 diff 检查通过。result commit 已有界接收并贯通 Tracker、
独立刷新快照和窗口收据；校验动作身份/代际/版本，保留最高结果版本，批次
后续失败优先。新增 4 项回归及 HWND 交付扩充通过，退出只保存结果版本。

输出资源授权与本地图应用、确认交互、点击批次调度、恢复交互和运行中日志
仍待完成；不以保存输出原文冒充已执行输出。点击入口默认禁用，整体迁移及
真实 Loom/完整交互/性能未验收。无新 EXE、提交、推送或合并。以下为历史记录。

最新最终源码完整串行回归 1946 passed、28 ignored；Clippy、fmt、2718
文件行数门禁及 diff 检查通过。动作恢复提示格式、严格有界解析、独立只增
存储及正常退出后台写入已接通；最多 32 条/64 KiB，不保存 payload、场景、
凭据或远端错误文本。磁盘提示不授权、不自动重放。新增 5 项回归通过。

正常退出的两类日志不是原子双文件事务，后续失败保留已有文件并报错。运行中
崩溃/断电保障、恢复交互、缓存运行中落盘回收、result commit、确认交互及
点击批次调度仍待完成。点击入口默认禁用，整体迁移及真实 Loom/完整交互/
性能未验收。无新 EXE、提交、推送或合并。以下为历史记录。

最新最终源码完整串行回归 1941 passed、28 ignored；Clippy、fmt、2716
文件行数门禁及 diff 检查通过。普通结算与退出泵已接入事件窗口交付，应用
有界缓存收据；未保存动作最多 32 个，满时禁止新动作，不占附件停用所需的
网络槽位，不启动周期唤醒。旧附件日志拒绝遗漏未保存动作收据。

动作持久化、result commit、确认交互及点击批次调度仍待完成；当前仅保留
内存证据，不宣称崩溃恢复。点击入口默认禁用。整体迁移及真实 Loom/完整
交互/跨 DPI/性能未验收。无新 EXE、提交、推送或合并。以下为历史记录。

最新最终源码完整串行回归 1939 passed、28 ignored；Clippy、fmt、2716
文件行数门禁及 diff 检查通过。授权窗口已实现精确 pending event 的结果
交付与集合路由，呈现前复核授权/成功 ACK/版本，呈现后复核撤权，返回保留
执行证据与呈现状态的收据。新增 5 项本地 HWND 回归通过。

接口尚未串入主循环收据缓存/持久化、result commit、确认交互及点击批次
调度；点击入口默认禁用。整体迁移及真实 Loom/可见交互/跨 DPI/性能未验收。
无新 EXE、提交、推送或合并。以下为历史记录。

最新最终源码完整串行回归 1934 passed、28 ignored；Clippy、fmt、2714
文件行数门禁及 diff 检查通过。后台事件 Job 已串联一次提交与最多 25 秒
后续流收敛，分开保存 ACK、场景/收敛错误、收割授权；失败/取消不丢失已收
ACK，不重放 POST，等待确认立即返回。新增 2 项回归及成功链扩充通过。

result commit、确认 UI、窗口呈现提交、动作恢复持久化及点击批次调度仍待
完成；点击入口默认禁用。整体迁移及真实 Loom/可见 UI/跨 DPI/性能未验收。
无新 EXE、提交、推送或合并。以下为历史记录。

最新最终源码完整串行回归 1932 passed、28 ignored；Clippy、fmt、2714
文件行数门禁及 diff 检查通过。真实设备流收敛 API 已连接 Tracker，收敛阶段
最多 25 秒，复查取消/会话/截止，成功后独立刷新，确认状态立即返回，等待前
释放批次；不重放动作 POST。新增 3 项本地 TLS/截止回归通过。

后台 Job 阶段编排、result commit、确认 UI、窗口呈现提交与动作恢复持久化
仍待完成；整体迁移及真实 Loom/可见 UI/跨 DPI/性能未验收。无新 EXE、提交、
推送或合并。以下为历史记录。

最新最终源码完整串行回归 1929 passed、28 ignored；Clippy、fmt、2712
文件行数门禁及 diff 检查通过。单事件 Tracker 已实现成功 ACK 后独立刷新、
整批身份/代际/终态验证和一次性 Scene 交付，借用批次快照避免重复 JSON 拷贝。
新增 6 项收敛合同回归通过，失败/reset/unknown 不产生成功快照。

有截止的后台流调度、result commit、确认交互、UI 呈现提交和动作恢复持久化
仍待接入；整体迁移及真实 Loom/可见 UI/跨 DPI/性能未验收。无新 EXE、提交、
推送或合并。以下为历史记录。

最新最终源码完整串行回归 1923 passed、28 ignored；Clippy、fmt、2710
文件行数门禁及 diff 检查通过。设备 Surface stream 已有单次 HTTPS 读取与
有界解析：8 MiB 响应、512 消息、单条 1 MiB，验证协议/reset/非倒退游标。
通用传输允许显式 8 MiB 预算，其他调用点保持原小预算，ACK 仍为 64 KiB。

新增 6 项解析/真实 TLS 回归通过，含超过 1 MiB 的实际批次和 401 不回退。
流消息身份匹配、状态/场景收敛与应用发送调度仍待完成；整体迁移及真实 Loom/
可见 UI/跨 DPI/性能未验收。无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1917 passed、28 ignored；Clippy、fmt、2707 文件行数
门禁及 diff 检查通过。Click 已携带窗口私有场景身份，已领取候选仍会因
场景替换/尺寸/失焦/取消/禁用而失效；消费时重新命中并核对节点/身份。
授权窗口 take_prepared_click 已连接单槽领取与授权事件准备，不发送动作。

新增 2 项隐藏 HWND 消费回归通过。入口仍默认关闭，应用发送调度、选择与
穿透、结果收敛及动作恢复持久化待完成；整体迁移和真实联调/可见 UI/跨 DPI/
性能未验收。无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1915 passed、28 ignored；Clippy、fmt、2707 文件行数
门禁及 diff 检查通过。SurfaceWindow 已接左键按下/移动/抬起和捕获/取消
消息，默认关闭；同节点且非拖动才排入单槽，替换/失焦/尺寸变化/关闭清理。
新增 3 项隐藏 HWND 实际消息/捕获测试通过，回调不发送事件或网络请求。

生产入口尚未启用，候选消费时授权/场景复核、选择/穿透、发送调度、收敛及
动作恢复持久化仍待完成。整体迁移及真实 Loom/可见 UI/跨 DPI/性能未验收。
无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1912 passed、28 ignored；Clippy、fmt、2705 文件行数
门禁及 diff 检查通过。客户区像素坐标已连接当前 DPI/视口命中、冒泡和事件
编码；整批输出限制 64 KiB，失败不返回部分事件，窗口忙碌/关闭拒绝准备。
授权窗口 prepare_click 前后复核当前状态，准备不等于授权提交。

新增 3 项呈现/隐藏 HWND 回归通过。原生鼠标消息、结果接纳/收敛、动作恢复
持久化仍待完成；整体迁移及真实 Loom/可见 UI/跨 DPI/性能未验收。无新 EXE、
提交、推送或合并。以下为历史记录。

最新最终源码完整串行回归 1909 passed、28 ignored；Clippy、fmt、2705
文件行数门禁及 diff 检查通过。窗口已有 begin_event/finish_event 接口，
检查当前显示/存活、租约及场景版本，Activation 以 eventId 绑定唯一 owner，
事件与生命周期事务互斥；归还不授予 ACK 交付资格、不重新显示已关闭窗口。

新增 3 项移交状态机回归通过。原生输入调度、结果接纳/收敛与动作恢复持久化
仍待完成；整体迁移及真实 Loom/可见 UI/跨 DPI/性能未验收。无新 EXE、提交、
推送或合并。以下为历史记录。

最新完整串行回归 1906 passed、28 ignored；Clippy、fmt、2704 文件行数
门禁及 diff 检查通过。事件 Job 已纳入 Sessions 的等待、正常 poll、退出
取消/收割及共享容量；拒收缓存继续占容量，取消保留 ACK 和事件证据。
旧附件恢复格式不能承载动作，存在待处理事件时拒绝导出不完整恢复快照。

新增 3 项真实线程/集合回归通过。窗口结果接纳、动作级恢复持久化、原生
输入与查询收敛仍待完成；整体迁移及真实 Loom/可见 UI/跨 DPI/性能未验收。
无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1903 passed、28 ignored；Clippy、fmt、2702 文件行数
门禁及 diff 检查通过。Surface click 已有后台 Job，复用共享容量和租约取消，
唯一移交/返还 owner 与事件。完成未收割保持容量占用，启动失败保留输入。
执行 ACK/错误与交付时授权分离，迟到撤权不丢证据、不授予更新权限。

新增 3 项本地 TLS/任务生命周期回归通过。UI 任务集合、原生鼠标接线、查询
收敛及整体迁移仍待完成；真实 Loom/可见 UI/跨 DPI/性能未验收。无新 EXE、
提交、推送或合并。以下为历史记录。

最新完整串行回归 1900 passed、28 ignored；Clippy、fmt、2700 文件行数
门禁及 diff 检查通过。Surface click 已有单次 HTTPS 提交 API，复用设备
认证/取消/超时，核对当前 active、绑定及场景版本，传输层限制 ACK 大小。
同一已尝试事件拒绝再次提交，错配 ACK 或网络错误不触发自动重放。

新增 3 项本地 TLS/边界回归通过，不代表真实 Loom 已验收。原生鼠标到后台
任务接线、当前 UI 租约复核、错误查询及结果收敛仍待接入。整体迁移和可见
UI/跨 DPI/性能未验收。无新 EXE、提交、推送或合并。以下为历史记录。

最新最终源码完整串行回归 1897 passed、28 ignored；Clippy、fmt、2698
文件行数门禁及 diff 检查通过。动作 ACK 已有 64 KiB 有界解析，核对事件、
实例与 request 身份；错误候选不覆盖当前状态，完整区分接收、确认、执行、
取消及 unknown。ClickEvent 同时保留原绑定和场景版本供后续发送复核。

新增 4 项 ACK 回归通过。ACK 成功不等于场景收敛；真实 HTTP、鼠标输入、
授权/生命周期复核与状态收敛仍待接入。整体迁移及可见 UI/跨 DPI/性能未验收。
无新 EXE、提交、推送或合并。以下为历史记录。

最新最终源码完整串行回归 1893 passed、28 ignored；Clippy、fmt、2697
文件行数门禁及 diff 检查通过。ClickAction 已能编码 loom.surface.v1 离散
事件，保留绑定/场景版本、payload 与修饰键语义，生成独立安全随机 eventId。
整体输出限制 64 KiB，序列化转义后的大小同样受限，不截断、不先复制 payload。

新增 4 项协议/预算回归通过；编码不等于授权或发送，原生鼠标、HTTP/ACK、
状态收敛及整体迁移仍待完成。可见 UI/跨 DPI/性能未验收。无新 EXE、提交、
推送或合并。以下为历史记录。

最新最终源码完整串行回归 1889 passed、28 ignored；Clippy、fmt、2695
文件行数门禁及 diff 检查通过。AdmittedSnapshot 保留同一响应的场景与
revision，AuthorizedWindow 保存该版本，关闭/撤权及生命周期 ACK 不覆盖它。
新增 2 项交付边界测试，扩展真实隐藏窗口的版本保留检查。

后续事件 baseRevision 已有准确数据来源，但事件发送、鼠标手势和输入授权
链路仍待接入；整体迁移、可见 UI、跨 DPI 及性能验收未完成。无新 EXE、
提交、推送或合并。以下为历史记录。

最新完整串行回归 1887 passed、28 ignored；Clippy、fmt、2695 文件行数
门禁及 diff 检查通过。Presentation.click_route 已连接最上层命中与当前
场景动作声明：容器由内向外冒泡、按钮阻断、文字不执行独立 click。
固定深度路径借用 Scene，不复制 payload；失败替换不泄漏候选动作。

新增 4 项合同/呈现集成回归通过；该接口不发送事件，也不代表按钮绘制、
鼠标手势、授权下动作分发或输入能力已完成。整体迁移及可见 UI/跨 DPI/
性能验收仍未完成。无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1883 passed、28 ignored；Clippy、fmt、2694 文件行数
门禁及 diff 检查通过。HWND 宿主命中使用当前 DPI 转换客户区像素坐标，
与绘制共用视口同步，重绘之前也能使用当前尺寸；零面积不返回目标。
查询不创建 D2D target，窗口忙碌或关闭时拒绝返回过期身份。

新增 2 项隐藏窗口回归通过。鼠标消息、授权下动作分发、捕获/焦点尚未
接入，不扩大输入能力声明；整体迁移及可见 UI/跨 DPI/性能验收仍未完成。
无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1881 passed、28 ignored；Clippy、fmt、2694 文件行数
门禁及 diff 检查通过。Presentation 已原子持有 CPU 命中快照，按逆绘制
顺序检查节点和祖先圆角/矩形裁剪，复用几何身份，不创建 COM 或查询集合。
新增 4 项叠放、圆角、溢出/透明度和更新一致性回归通过。

输入消息、授权下动作分发、捕获/焦点尚未接入，不扩大输入能力；交互控件、
滚动、主题/UIA、整体迁移及可见 UI/跨 DPI/性能验收仍未完成。
无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1877 passed、28 ignored；Clippy、fmt、2692 文件行数
门禁及 diff 检查通过。props.visible=false 已统一排除隐藏子树的测量、
占位/gap、几何、文字和绘制，隐藏根清空快照，恢复显示重新准备。
新增 4 项布局、替换、输入边界及真实像素回归通过；原始安全校验不放宽。

交互控件、滚动、动态主题/UIA 和剩余布局仍待完成；整体迁移、真实 Loom、
可见 UI、跨 DPI 与性能未验收。无新 EXE、提交、推送或合并。
以下为历史记录。

最新完整串行回归 1873 passed、28 ignored；Clippy、fmt、2691 文件行数
门禁及 diff 检查通过。Progress 已实现原生叶子布局、value/max 归一和
轨道/进度绘制，并加入宿主报告；复用画刷，不创建 HWND 或定时器。
新增 4 项数值、布局、叶子边界及真实像素回归通过。

采用 Neuro 规范默认控件色/信号黄；动态主题、UIA、其他控件、滚动及
剩余布局仍待完成。整体迁移、真实 Loom、可见 UI、跨 DPI 与性能未验收。
无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1869 passed、28 ignored；Clippy、fmt、2688 文件行数
门禁及 diff 检查通过。Divider 已实现固定满宽 1px 叶子布局与绘制，并加入
宿主报告；有效属性视图对齐前端 hr，同时保留原始 wire 字段容量校验。
新增 4 项布局/属性/安全/真实像素回归通过，能力报告检查同步覆盖 divider。

动态主题变量、其他控件、滚动及剩余布局边界仍待完成；整体迁移、真实
Loom、可见 UI、跨 DPI 与性能尚未完成验收。
无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1865 passed、28 ignored；Clippy、fmt、2686 文件行数
门禁及 diff 检查通过。Spacer 已实现原生测量、默认 grow=1 与既有样式
绘制，并加入新建/恢复共用宿主报告。新增 5 项布局、叶子安全、真实像素
与能力报告回归通过；不新增 HWND、定时器或文字布局。

其他控件、滚动、单端 auto 收缩适配和循环百分比仍待完成；整体迁移、
真实 Loom、可见 UI、跨 DPI 与性能仍未完成验收。
无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1860 passed、28 ignored；Clippy、fmt、2685 文件行数
门禁及 diff 检查通过。修复文字内容宽度归零导致场景准备失败的问题；
DirectWrite 测量允许零布局尺寸，最终快照仍跳过零面积文字绘制。
新增真实 DirectWrite 零尺寸与完整场景塌缩内容盒回归通过。

单端 auto 收缩适配、循环百分比、其他控件、滚动及整体迁移仍未完成；
可见 UI、跨 DPI 和性能仍待验收。无新 EXE、提交、推送或合并。
以下为历史记录。

最新完整串行回归 1858 passed、28 ignored；Clippy、fmt、2685 文件行数
门禁及 diff 检查通过。容器类默认 width:100% 与显式 auto 已通过统一
取值器接入根盒、正常流、Stack、绝对定位和原生测量。新增 2 项回归通过，
覆盖五类容器非 stretch 对齐以及绝对定位默认/auto 差异。

单端 auto 收缩适配、循环百分比及其他控件默认样式仍待完成；滚动和整体
迁移未完成。可见 UI、跨 DPI 与性能仍待验收。
无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1856 passed、28 ignored；Clippy、fmt、2684 文件行数
门禁及 diff 检查通过。根 absolute auto 双端尺寸已按宿主剩余空间求解，
再应用 min/max 与外盒下限；显式尺寸优先，默认容器满宽不误当 auto。
新增 2 项几何回归和 1 项真实像素回归通过。

单端收缩适配、子节点默认样式完整对齐和循环百分比仍待完成；滚动、其他
控件及整体迁移未完成。可见 UI、跨 DPI 和性能仍待验收。
无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1853 passed、28 ignored；Clippy、fmt、2684 文件行数
门禁及 diff 检查通过。默认根高度已改为内容测量结果，文字重排更新内容
高度而宿主视口/裁剪保持固定；固定高度绘制与定位夹具改为显式声明高度。
新增自然根高度几何和像素回归通过。

absolute auto 双端尺寸推导/收缩适配、循环百分比仍待完成；滚动、其他
控件及完整迁移功能未完成。可见 UI、跨 DPI 和性能仍待验收。
无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1851 passed、28 ignored；Clippy、fmt、2684 文件行数
门禁及 diff 检查通过。根节点 relative/absolute 偏移已接入布局树，末端
锚点使用最终约束尺寸，子树随根平移，宿主裁剪保持固定。新增 2 项几何
回归及 1 项真实像素回归通过。

默认根内容自适应、absolute auto 尺寸推导和循环百分比仍未完成；滚动、
其他控件及完整迁移功能未完成。可见 UI、跨 DPI 和性能仍待验收。
无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1848 passed、28 ignored；Clippy、fmt、2684 文件行数
门禁及 diff 检查通过。根盒显式/min/max/边框下限已独立求解，Tree 保存
宿主 viewport，祖先根裁剪不再误用根布局尺寸；Presentation::size 仍为
宿主尺寸。新增 2 项约束回归和 1 项真实背景像素回归通过。

默认根内容自适应、循环百分比和根定位仍需对齐；滚动、其他控件及完整
迁移功能未完成。可见 UI、跨 DPI 和性能仍待验收。
无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1845 passed、28 ignored；Clippy、fmt、2683 文件行数
门禁及 diff 检查通过。布局节点记录 containing_width，百分比 padding
在容器布局、文字测量/快照和外盒下限中统一使用包含块宽度。正常流与
绝对定位分别传父内容盒/padding box，不影响宿主 cqw/cqh 查询容器。
新增 2 项嵌套/文字/绝对定位回归通过。

根宿主尺寸和固有尺寸中的循环百分比仍待进一步对齐；滚动、其他控件及
完整迁移功能未完成。可见 UI、跨 DPI 和性能仍待验收。
无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1843 passed、28 ignored；Clippy、fmt、2683 文件行数
门禁及 diff 检查通过。子节点外盒下限已接入行列、Stack 与绝对定位，
防止显式/max 尺寸和 flex 收缩压扁边框/padding，空间不足保留溢出。
新增 2 项回归覆盖容器尺寸钳制与右/下定位锚点。

根宿主尺寸与百分比 padding 的包含块/固有尺寸循环仍待专项对齐；滚动、
圆角命中测试和其余迁移功能未完成。可见 UI、跨 DPI、资源实测仍未验收。
无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1841 passed、28 ignored；Clippy、fmt、2683 文件行数
门禁及 diff 检查通过。单轴 hidden/auto 已将另一轴默认 visible 归一为
auto，矩形裁剪与圆角 mask 共用计算值。新增表驱动合同与双方向真实像素
回归通过，替换旧的只裁声明轴行为。

滚动条/滚轮/程序化滚动未因本增量而实现；极小外盒尺寸、圆角命中测试及
其余迁移功能仍未完成。可见 UI、跨 DPI 和资源实测尚待验收。
无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1839 passed、28 ignored；Clippy、fmt、2682 文件行数
门禁及 diff 检查通过。overflow 矩形裁剪和圆角 mask 已缩到 padding box，
不让裁剪容器子内容覆盖父边框。持久几何、文字和绘制使用同一边框上下文。
新增 2 项真实像素回归及 1 项几何/文字一致性回归通过。

极小外盒 CSS 最小尺寸、完整单轴 overflow 归一、圆角命中测试及其余迁移
功能仍未完成；可见 UI、跨 DPI、实际资源开销尚待验收。
无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1836 passed、28 ignored；Clippy、fmt、2682 文件行数
门禁及 diff 检查通过。边框已接入 PaintFrame，以 alternate 内外轮廓绘制
实心环，透明中心不被填色，圆角背景保留；过厚边框不重复混合角落 alpha。
新增 3 项真实 Direct2D 像素回归通过。几何按工厂缓存，创建失败不换缓存。

padding-box overflow 裁剪、极小外盒 CSS 最小尺寸与其余迁移功能仍未完成；
可见 UI、跨 DPI 和真实资源开销尚待验收。无新 EXE、提交、推送或合并。
以下为历史记录。

最新完整串行回归 1833 passed、28 ignored；Clippy、fmt、2681 文件行数
门禁及 diff 检查通过。边框盒模型尺寸已接入容器及文字测量：正常流扣除
边框和 padding，绝对定位锚定 padding box，文字按内容宽度断行并恢复
外盒反馈高度/基线。新增 4 项回归通过。

边框颜色/圆角边框环绘制、overflow 内边界和极小外盒规则仍待接通；
本阶段不是完整边框功能，也不表示其余迁移功能或性能验收完成。
无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1829 passed、28 ignored；Clippy、fmt、2679 文件行数
门禁及 diff 检查通过。双轴裁剪圆角容器已用几何 mask 裁剪子树，并正确
嵌套 opacity 层、恢复兄弟节点绘制状态；同工厂复用几何，换工厂整批重建。
修正圆角 cqw/cqh 的直接父尺寸误用，保留宿主查询容器上下文。
新增 3 项真实像素回归和 1 项工厂缓存回归通过。

边框、多值逐角、完整单轴 CSS overflow 归一、滚动及其他迁移功能仍未完成；
大型嵌套资源成本、可见 UI 与跨 DPI 尚未验收。无新 EXE、提交、推送或合并。
以下为历史记录。

最新完整串行回归 1825 passed、28 ignored；Clippy、fmt、2678 文件行数
门禁及 diff 检查通过。圆角背景已使用完整几何加祖先裁剪，百分比按自身
宽高、em 按已计算字号解析，超大半径等比归一，不误裁普通子节点。
新增 2 项半径合同与 4 项真实像素回归通过；可见 UI/跨 DPI 尚未验收。

此增量不含边框、多值逐角语法或圆角 overflow 子树裁剪；主题、滚动、
其他控件和其余迁移功能仍未完成。无新 EXE、提交、推送或合并。
以下为历史记录。

最新完整串行回归 1819 passed、28 ignored；Clippy、fmt、2677 文件行数
门禁及 diff 检查通过。Surface 子树 opacity 已用 Direct2D 组层实现，
透明度 0 跳过子树，透明度 1 不建层，组范围保留可见溢出，错误路径恢复层栈。
新增 4 项真实像素回归覆盖组重叠、嵌套、零透明度、溢出及数值边界；后续
不透明标记验证绘制状态恢复。大型嵌套资源成本与跨 DPI/可见 UI 尚未验收。

边框/圆角、主题变量、滚动/其他控件及其余迁移任务仍未完成；半透明组有
临时合成层成本。无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1815 passed、28 ignored；Clippy、fmt、2677 文件行数
门禁及 diff 检查通过。Surface HWND 已按树顺序绘制纯色背景与继承颜色文字，
提交时预计算，重绘复用画刷和布局；支持白名单命名色、hex、逗号 rgb/hsl
及 alpha/currentColor。新增 2 项颜色合同和 3 项真实离屏像素测试通过，
覆盖文字继承/覆盖、Stack 遮挡和背景混色/裁剪。

未验收可见 UI/跨 DPI；主题变量、边框/圆角、组 opacity、滚动/控件和其余
迁移任务仍未完成。无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1810 passed、28 ignored；Clippy、fmt、2674 文件行数
门禁及 diff 检查通过。候选显式确认已连接后台 remount，复用既有准入和
active 显示链；默认否，模态前后校验同一候选，启动再次检查授权/设备、
节点占用和句柄容量，一次尝试后消费候选、不自动重发或回退创建 Art。
新增 5 项隐藏 HWND/状态回归通过，未执行真实 Loom 和可见 UI 全链验收。

历史记录管理、失败处置、崩溃前日志及其余 N08 控件/运行时仍未完成。
无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1805 passed、28 ignored；Clippy、fmt、2673 文件行数
门禁及 diff 检查通过。原生菜单已接入显式恢复文件选择、后台读取和记录
查询；文件任务共享现有远程容量/等待/退出回收，查询重新验证当前服务、
设备及授权。候选只缓存待确认，不自动 remount，None 不清除原 unknown。
新增 5 项回归通过；可见文件对话框和在线 Loom 查询尚未验收。

按需读取不增加启动全目录扫描。显式重新挂载确认、历史记录管理、崩溃前
日志及其余迁移功能仍未完成；无新 EXE、提交、推送或合并。
以下为历史记录。

最新完整串行回归 1800 passed、28 ignored；Clippy、fmt、2670 文件行数
门禁及 diff 检查通过。正常退出已把 Surface 最小恢复元数据后台写入原生
profile 的 surface-recovery；只增写，最多 128 份，每份 32 条/64 KiB，
不保存 token/场景/参数、不覆盖旧证据。严格读取不授予操作权限或自动请求。
新增 5 项回归通过，文件写入测试限定 linshi 隔离目录。

启动恢复列表、重新授权后的查询/确认交互及崩溃前日志仍未实现；本轮不能
证明完整恢复或断电持久性。未在线验收，无新 EXE、提交、推送或合并。
以下为历史记录。

最新完整串行回归 1795 passed、28 ignored；Clippy、fmt、2666 文件行数
门禁及 diff 检查通过。已确认但未建窗的附件独立保留生命周期 owner，退出
尝试一次 inactive；同一宽限阶段等待迟到挂载，不为清理创建窗口。重复、
unknown 和身份不一致记录不生成停用 owner；在途不可取出，回交校验版本，
原始挂载证据不被覆盖。新增 5 项回归通过，无实际 Loom 网络验收。

未解决容量不足/未知创建的恢复处置，跨进程持久化及其余 N08 功能仍待完成。
无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1790 passed、28 ignored；Clippy、fmt、2664 文件行数
门禁及 diff 检查通过。退出已在统一撤权前关闭 Surface 窗口，复用一次性
inactive 调度，以 2 秒绝对截止等待生命周期事务并继续泵消息；到期或失败
仍进入原有取消、回收及附件结算，不重放 unknown，不伪造远端停用确认。
新增 4 项 Win32 等待/消息泵测试通过，未执行真实 Loom 退出停用验收。

尚需处理挂载中/准入失败附件、恢复持久化、失败处置及其余 N08 功能。
2 秒不代表整体退出耗时保证；未生成新 EXE、提交、推送或合并。
以下为历史记录。

最新完整串行回归 1786 passed、28 ignored；Clippy、fmt、2662 文件行数
门禁及 diff 检查通过。退出回收后已接入附件结算：回交匹配 owner，已确认
inactive/disposed 才通过；active、mounted、unknown、在途及未接纳结果
报告未确认。不重试、不显示、不消费未匹配证据。新增 5 项回归覆盖迟到
停用/激活确认、unknown、失败准入及已挂载窗口退出，均不连接真实服务。

证据仍仅在本进程内保留，退出网络停用及跨进程恢复持久化尚未完成。
未执行 ignored 环境或在线 Loom 验收，无新 EXE、提交、推送或合并。
以下为历史记录。

最新完整串行回归 1781 passed、28 ignored；Clippy、fmt、2660 文件行数
门禁及 diff 检查通过。普通 Surface 窗口关闭已调度一次 inactive，保留附件，
unknown/撤权不重发；纯状态及隐藏窗口回归通过。Live/Surface 组合等待容量
在启动和接纳前受 63 句柄上限约束，避免消息等待超限退出。

未执行在线停用；应用退出网络收尾、失败处置、恢复持久化和完整样式/滚动/
资源控件仍未完成。未执行 ignored 环境或在线 Loom 验收，无新 EXE、提交、
推送或合并。以下为历史记录。

最新完整串行回归 1777 passed、28 ignored；Clippy、fmt、2660 文件行数
门禁及 diff 检查通过。Loom 菜单已有“打开 Art（原生预览）”入口，明确
输入 Art ID 后按当前窗口节点和有效租约启动临时挂载；有状态反馈和重复/
容量防护。能力报告仅含当前静态容器与文字，不宣称输入或其他运行时。

未操作真实 Loom 或验收可见对话框；失败处置、关闭上报、恢复持久化及
完整样式/滚动/资源控件仍未完成。未执行 ignored 环境或在线验收，无新
EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1775 passed、28 ignored；Clippy、fmt、2658 文件行数
门禁及 diff 检查通过。应用已调度 active 作业并回交窗口，成功且当前授权
有效时显示；失败不自动重发，未匹配结果有界扫描并保留，不阻挡其他结果。

隐藏/夹具回归不等于在线 active 或可见窗口验收。用户发起、失败处置、
关闭上报、恢复持久化、完整样式/滚动/资源控件仍未完成。未执行 ignored
环境或在线 Loom 验收，无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1773 passed、28 ignored；Clippy、fmt、2658 文件行数
门禁及 diff 检查通过。授权窗口支持激活 Owner 单次移交和身份/版本校验
回交，pending/unknown 不显示，失败不重发；迟到 active ACK 不复活撤权窗口。

应用 active 作业调度和成功显示尚未接通，用户发起、失败处置、关闭上报/
恢复持久化及完整样式、滚动、资源控件仍待完成。未执行 ignored 环境或在线
Loom 验收，无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1770 passed、28 ignored；Clippy、fmt、2657 文件行数
门禁及 diff 检查通过。绑定保留确认生命周期，显示同时要求有效授权与已
确认 active；mounted/unknown/disposed 不可显示。Live playback 夹具的请求
头/撤权竞态已修正，完整回归及额外连续 5 次该用例通过。

active 后台事务交接与显示、用户发起入口、失败处置、关闭上报/恢复持久化
仍待完成；完整样式、滚动与资源控件未完成。未执行 ignored 环境或在线 Loom
验收，无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1768 passed、28 ignored；Clippy、fmt、2656 文件行数
门禁及 diff 检查通过。挂载完成结果已接到应用准入与隐藏窗口集合；失败/
未知/重复结果有界保留，容量不足原样退回，成功窗口保留 Art/node 请求上下文。

用户发起入口、失败记录处置、active 确认后显示、关闭上报和恢复持久化
仍未完成；完整样式、滚动及资源控件待实现。未执行 ignored 环境或在线
Loom 验收，无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1765 passed、28 ignored；Clippy、fmt、2655 文件行数
门禁及 diff 检查通过。有界授权窗口集合已接到 Application 的消息等待、
到期结算和退出关闭路径；终态保留附件 owner，但不继续参与事件/截止等待。

挂载完成结果尚未接纳到集合，用户侧完整链、正式生命周期上报和恢复持久化
仍未完成；完整样式、滚动及资源控件待实现。未执行 ignored 环境或在线 Loom
验收，无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1764 passed、28 ignored；Clippy、fmt、2654 文件行数
门禁及 diff 检查通过。应用等待函数支持显式绝对截止，到期优先结算且不
丢排队输入，无句柄时也可到期唤醒；无截止保持无限等待，无周期轮询。

应用尚未将 Surface 截止与撤权句柄接到该函数，自动失效关闭及挂载结果
收割未完成；生命周期、完整样式、滚动和资源控件仍待接通。未执行 ignored
环境或在线 Loom 验收，无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1762 passed、28 ignored；Clippy、fmt、2654 文件行数
门禁及 diff 检查通过。AuthorizedWindow 已把准入、窗口和租约组合，隐藏
窗口测试确认显式 refresh 在撤销/过期后关闭并保留附件证据，不伪造 disposed。

应用自动等待、挂载结果收割和生命周期事务仍未接通该 owner，不能宣称
完整持续授权链完成；完整样式、滚动和资源控件仍待实现。未执行 ignored
环境或在线 Loom 验收，无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1760 passed、28 ignored；Clippy、fmt、2652 文件行数
门禁及 diff 检查通过。租约提供独立惰性撤权 HANDLE，重复订阅复用资源，
撤销后保持信号；注册/撤销竞态测试通过。会话截止时间可用于安排过期等待。

尚未接入应用等待集合和 Surface 窗口关闭路径，不代表持续撤权完成；附件
生命周期、完整样式、滚动和资源控件仍待实现。未执行 ignored 环境或在线
Loom 验收，无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1758 passed、28 ignored；Clippy、fmt、2650 文件行数
门禁及 diff 检查通过。挂载结果准入已检查当前租约、绑定/快照身份、确认
状态和声明式运行时，失败保留恢复证据，不自动重放。

这是交付时点检查，尚未连接应用窗口创建及显示期间撤销/过期和附件生命
周期；完整样式、滚动和资源控件仍未完成。未执行 ignored 环境或在线 Loom
验收，无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1755 passed、28 ignored；Clippy、fmt、2648 文件行数
门禁及 diff 检查通过。DPI 建议矩形已由合成窗口消息验证，溢出范围拒绝；
重复绘制复用目标和画刷，COM 身份断言通过。未做真实跨屏或性能量化验收。

应用授权复核/撤销、附件生命周期到窗口的连接，以及完整样式、滚动和
资源控件仍未完成。未执行 ignored 环境或在线 Loom 验收，无新 EXE、
提交、推送或合并。以下为历史记录。

最新完整串行回归 1754 passed、28 ignored；Clippy、fmt、2648 文件行数
门禁及 diff 检查通过。SurfaceWindow 已连接 HWND 绘制、尺寸/DPI 失效和
关闭消息，隐藏窗口回归包含同步重入关闭，确认已关闭 Host 不会被恢复。

应用授权绑定结果到窗口的入口尚未接通；完整样式、滚动、资源控件、跨屏
DPI 和用户视觉仍未验收。未执行 ignored 环境或在线 Loom 验收，无新 EXE、
提交、推送或合并。以下为历史记录。

最新完整串行回归 1753 passed、28 ignored；Clippy、fmt、2646 文件行数
门禁及 diff 检查通过。声明式文字 Host 已通过隐藏 HWND 绘制、布局复用、
尺寸变化重排与失败替换回归；按窗口 DPI 计算逻辑尺寸，但未跨 DPI 验收。

当前黑字白底后端尚未接通应用 Surface 入口与窗口消息，滚动、资源控件、
完整样式及用户视觉验收仍未完成。未执行 ignored 环境或在线 Loom 验收，
无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1752 passed、28 ignored；Clippy、fmt、2644 文件行数
门禁及 diff 检查通过。Presentation 已同时保留同轮成功布局的 Geometry
和 TextFrame，包含容器与被裁剪节点，失败更新不替换已有几何。

当前几何不含滚动变换或事件命中规则；生产窗口及资源控件仍未接通，不能
视为完整交互实现。未执行窗口、多 DPI、ignored 环境或在线 Loom 验收，
无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1751 passed、28 ignored；Clippy、fmt、2643 文件行数
门禁及 diff 检查通过。TextFrame 已独立持有节点 ID 和原生布局；Presentation
持有场景与快照，替换/重排失败保留旧内容，离屏像素测试已经由该状态执行。

生产窗口、完整控件几何、滚动和资源控件仍未接通；未执行窗口、多 DPI、
ignored 环境或在线 Loom 验收，无新 EXE、提交、推送或合并。以下为历史记录。

最新完整串行回归 1749 passed、28 ignored；Clippy、fmt、2641 文件行数
门禁及 diff 检查通过。文字祖先裁剪已增加真实 WIC/Direct2D 离屏像素验证，
包括裁剪外透明、无祖先裁剪正对照，以及后续绘制确认裁剪栈恢复。

当前证据限于 96 DPI 离屏输出；PreparedScene 尚未接入生产窗口，不代表
滚动、资源控件、窗口视觉或多 DPI 验收完成。未执行 ignored 环境测试或
在线 Loom 联调，无新 EXE、提交、推送或合并。以下为历史检查点。

最新完整串行回归 1748 passed、28 ignored；Clippy、fmt、2640 文件行数
门禁及 diff 检查通过。祖先逐轴裁剪已接入文字快照，根宿主与 scroll
裁剪双轴；完全不可见的非零面积文字仍参与测量和高度反馈。

未执行像素输出、窗口视觉、ignored 环境测试或在线 Loom 联调。滚动状态
与变换、资源控件及窗口集成仍未完成，无新 EXE、提交、推送或主干合并。
以下保留历史检查点，不代表当前未完成项清单。

最新完整串行回归 1745 passed、28 ignored；Clippy、fmt、2638 文件行数
门禁及 diff 检查通过。未执行 ignored 环境测试、在线 Loom 联调或窗口
视觉验收，无新交付 EXE、提交、推送或主干合并。

文字最终高度/基线已通过最多 8 轮反馈传回容器固有尺寸和兄弟布局，稳定后
一起交付 Tree/TextFrame，未收敛不交付。零字号、复杂 grid/包含块、资源
控件、祖先裁剪/滚动和窗口绘制仍未完成。以下保留历史检查点。

最终源码完整串行回归 1743 passed、28 ignored；Clippy、fmt、2636 文件
行数门禁与 diff 检查通过。未执行 ignored 环境测试、在线 Loom 联调或
窗口视觉验收，无新 EXE、提交、推送或主干合并。

最终文字矩形已可重排成持有 DirectWrite Layout 的绘制快照，记录溢出并
提供文字矩形裁剪绘制入口；零面积跳过，正小数尺寸保留。父尺寸反馈、
祖先裁剪、滚动和窗口视觉验收仍未完成。以下保留历史检查点。

最新完整串行回归 1740 passed、28 ignored；Clippy、fmt、2634 文件行数
门禁及 diff 检查通过。未执行 ignored 环境测试、在线 Loom 联调或窗口
视觉验收，无新交付 EXE、提交、推送或主干合并。

计算后继承字号已接通容器、stack 和相对/绝对定位的 em 几何，父内边距与
子尺寸分别使用对应节点字号，不重复换算。文字重排/尺寸反馈、零字号、
资源控件和窗口绘制仍未完成。以下保留历史检查点。

最新完整串行回归 1738 passed、28 ignored；Clippy、fmt、2634 文件行数
门禁与 diff 检查通过。未执行 ignored 环境测试、在线 Loom 联调或窗口
视觉验收，无新交付 EXE、提交、推送或主干合并。

NativeMeasure 已把单场景样式、实际 DirectWrite 测量和容器固有尺寸接到
Tree，真实嵌套文字场景可计算布局；图片/控件仍明确拒绝。分配后重排、
完整尺寸反馈、零字号、资源/控件和窗口绘制仍未完成。以下为历史检查点。

最新完整串行回归 1736 passed、28 ignored；Clippy、fmt、2632 文件行数
门禁与 diff 检查通过。未执行 ignored 环境测试、在线 Loom 联调或窗口
视觉验收，无新交付 EXE、提交、推送或主干合并。

全树文字样式已有有界预计算与节点身份绑定，DirectWrite 可直接消费
解析后样式，避免 em/% 重复换算。完整测量 owner、容器固有尺寸、文字
重排、零字号和窗口绘制仍未接通。以下保留历史检查点。

完整串行回归 1734 passed、28 ignored；最终树布局定向复核 2 passed。
Clippy、fmt、2630 文件行数门禁及 diff 检查通过。未执行 ignored 环境
测试、在线 Loom 联调或窗口视觉验收，无新 EXE、提交、推送或主干合并。

节点树已有有界布局遍历，嵌套容器输出场景坐标、父索引和稳定顺序，
失败不交付部分树。Measure 接口已建立，但宿主字体继承、容器固有尺寸、
文字重排和绘制 owner 仍未接通。以下保留历史检查点。

完整串行回归 1732 passed、28 ignored；最终容器定向复核 8 passed。
Clippy、fmt、2628 文件行数门禁与 diff 检查通过。未执行 ignored 环境
测试、在线 Loom 联调或窗口视觉验收，无新 EXE、提交、推送或主干合并。

stack 已接入共享内容区域重叠布局，保持场景顺序，支持尺寸约束、垂直
对齐和已有定位。不执行 flex gap/grow；复杂 grid 轨道和非默认 justify
仍未支持，完整树测量和窗口绘制仍未完成。以下保留历史检查点。

最新完整串行回归 1730 passed、28 ignored；Clippy、fmt、2627 文件行数
门禁及 diff 检查通过。未执行 ignored 环境测试、在线 Loom 联调或窗口
视觉验收，无新交付 EXE、提交、推送或主干合并。

行列布局支持 relative 保留占位偏移，以及退出普通流的显式 absolute；
绝对定位要求直接定位父级和双轴锚点，不猜测祖先/static-position。stack、
完整树测量、滚动裁剪和窗口绘制仍未完成。以下保留历史检查点。

最新源码完整串行回归 1728 passed、28 ignored；Clippy、fmt、2626 文件
行数门禁与 diff 检查通过。未执行 ignored 环境测试、在线 Loom 联调或
窗口视觉验收，无新交付 EXE、提交、推送或主干合并。

真实场景容器已接到主轴/交叉轴分配，支持内边距、间距、对齐、尺寸和
grow，测量结果按节点 ID 校验，cqw/cqh 保留宿主查询容器。递归测量、
文字重排、定位/stack、滚动、裁剪和绘制仍未完成。以下保留历史检查点。

最新完整串行回归 1724 passed、28 ignored；Clippy、fmt、2624 文件行数
门禁与 diff 检查通过。未执行 ignored 环境测试、在线 Loom 联调或窗口
视觉验收，无新交付 EXE、提交、推送或主干合并。

容器交叉轴几何已实现拉伸、起始/居中/末尾与基线对齐，保留显式尺寸、
min/max 和负偏移/两侧溢出。尚未接通完整节点树和绘制 owner，不能视为
原生 renderer 已完成。以下保留历史检查点。

本检查点完整串行回归 1720 passed、28 ignored；Clippy、fmt、2622 文件
行数门禁与 diff 检查通过。未执行 ignored 环境测试、在线 Loom 联调或
Surface 窗口视觉验收，无新交付 EXE、提交、推送或主干合并。

text 行高已接到 DirectWrite uniform line spacing，并提供实际字体基线；
节点 lineHeight 的 em/% 相对当前字号，父行高按计算值继承。完整树布局、
零字号、选择及 Surface 窗口视觉验收仍未完成。以下为历史检查点。

最新源码完整串行回归 1719 passed、28 ignored；Clippy、fmt、2622 文件
行数门禁及 diff 检查通过。未执行 ignored 环境测试、在线 Loom 联调或
Surface 窗口视觉验收，无新交付 EXE、提交、推送或主干合并。

声明式 text 节点已有通往 DirectWrite 的样式/文本适配入口，显式继承字体
并解析相对字号、字重、空白和对齐。真实节点测量/换行测试已增加；完整
树布局、line-height、零字号策略、选择、其他节点与窗口视觉验收仍未完成。
以下为历史检查点。

最新完整串行回归 1717 passed、28 ignored；Clippy、fmt、2620 文件行数
门禁与 diff 检查通过。未执行 ignored 环境测试、在线 Loom 联调或 Surface
窗口视觉验收，无新交付 EXE、提交、推送或主干合并。

Surface 文字已有真实 DirectWrite 测量和复用布局的 Direct2D 绘制接口，
支持字体、字重、对齐和四种空白/换行模式。实际 Windows 测量已测试；
Surface 窗口绘制尚未视觉验收，节点布局/继承/文字选择及其他渲染仍未完成。
以下保留历史检查点。

最新完整串行回归 1715 passed、28 ignored；Clippy、fmt、2618 文件行数
门禁及 diff 检查通过。未执行 ignored 环境测试或在线 Loom 联调，无新
交付 EXE、提交、推送或主干合并。

行/列主轴数值分配已支持伸缩、min/max 边界冻结、gap、六种对齐和显式
overflow。尚未接通节点树布局、交叉轴、文字测量、滚动和绘制；不是完整
CSS flex/grid 引擎，也不扩大宿主能力报告。以下为历史检查点。

最新源码完整串行回归 1711 passed、28 ignored；Clippy、fmt、2616 文件
行数门禁及 diff 检查通过。未执行 ignored 环境测试或在线 Loom 联调，
未构建交付 EXE、提交、推送或合并主干。

声明式尺寸计算已支持主干单位白名单和固定 min/calc 形式，Scene Node 可
解析布局/样式长度；不依赖浏览器，不执行任意表达式。布局分配、绘制和
输入仍未完成，不能据此宣称原生 renderer 已完成。以下保留历史检查点。

本检查点完整串行回归 1709 passed、28 ignored；Clippy、fmt、2614 文件
行数门禁及 diff 检查通过。未执行 ignored 环境测试或在线 Loom 联调，
无新交付 EXE、提交、推送或主干合并。

原生声明式场景模型已对齐主干 18 种节点类型，挂载结果提供受限模型解析
入口；限制树深度、节点数、数据大小并拒绝重复身份。类型解析不代表绘制
或交互实现，宿主能力报告不因此扩大。原生 renderer、资源与 UI 入口仍
未完成；以下为历史检查点。

本检查点完整串行回归 1706 passed、28 ignored；Clippy、fmt、2612 文件
行数门禁及 diff 检查通过。未执行 ignored 环境测试或在线 Loom 联调，
未构建交付 EXE、提交、推送或合并主干。

恢复查询已有可取消后台 Job，并接入应用任务集合、四项容量、统一等待、
服务切换与退出回收。成功候选仅供显式确认，取消/撤权不交付旧候选，查询
不修改原创建 unknown。恢复 UI、跨进程恢复与原生 renderer 仍未完成。
下面保留先前检查点。

当前完整串行回归 1704 passed、28 ignored；Clippy、fmt、2609 文件行数
门禁和 diff 检查通过。未执行 ignored 环境测试或在线 Loom 联调，无新
交付 EXE、提交、推送或主干合并。

候选 remount 已复用首次挂载后台任务和 Application 集合；Request 显式
区分创建/恢复，恢复失败不重新创建。取消保留候选与 unknown，完成后撤权
保留确认绑定但丢弃场景。恢复查询后台任务和 UI 确认入口仍未接通，跨进程
恢复与 renderer 仍未完成。以下为历史检查点。

最新完整串行测试 1703 passed、28 ignored；Clippy、fmt、2609 文件行数
门禁与 diff 检查通过。未执行 ignored 环境测试或在线 Loom 联调，无新
交付 EXE、提交、推送或主干合并。

首次未知创建恢复已增加设备认证 stream 候选查询和显式 remount 协议；查询
不重复创建，不把候选当原请求回执，不用空结果清除 unknown。remount 重新
验证来源/身份/代次/Art/runtime，取得实际生命周期版本。恢复后台任务和
UI 接线、跨进程恢复与真实 renderer 仍未完成。以下为先前检查点。

当前检查点完整串行回归 1700 passed、28 ignored；Clippy、fmt、2607 文件
行数门禁和 diff 检查通过。无新交付 EXE、提交、推送或主干合并。

N08 首次 Surface attach 已有可取消后台 Job，并接入 Application 的统一
等待、服务切换取消、错误提示和退出回收。完成未领取结果占四项总容量；
取消抑制场景交付，但保留已确认绑定或 unknown 创建上下文，不自动重发 POST。
上下文目前仅在进程内，首次未知创建恢复、实际 renderer 能力来源、场景
渲染与 UI 挂载/观看入口仍未完成。后续段落保留历史检查点，不代表当前缺项。

N08 首次 Surface attach HTTP 已实现，能力报告显式且有界，校验 Art/附件/
runtime 返回合同。不默认启用旧 WebView 能力，不提前 active。首次挂载
后台任务、实际 renderer 能力来源、场景渲染与观看入口仍未完成。
本轮最终完整串行回归 1696 passed、28 ignored；Clippy、fmt、2603 文件行数
门禁及 diff 检查通过，无新交付 EXE、提交、推送或主干合并。

N08/N09 Surface Job 已接入 Application 远程集合、消息等待和退出回收；
未取走结果保留 owner 并占四项容量，服务切换/退出抑制旧场景成功交付。
首次 attach、场景渲染及 UI 消费恢复结果的业务流程仍未完成。
本轮默认串行回归 1693 passed、28 ignored；Clippy、fmt、2600 文件行数门禁
及 diff 检查通过，未构建交付 EXE、提交、推送或合并主干。

N08/N09 生命周期 transition/recover 已有可取消后台 Job，共享四任务容量，
启动失败或完成返还 owner，在途取消保留 unknown，迟到成功撤权后不交付。
尚未接入 Application 集合，首次 attach、场景渲染与观看入口仍未完成。
本轮默认串行回归 1691 passed、28 ignored；Clippy、fmt、2599 文件行数门禁
及 diff 检查通过，无新交付 EXE、提交、推送或合并主干。

N08/N09 新增 Surface 生命周期 owner、精确响应确认和 unknown 恢复：失败
不能继续递增或自动重放，认证 remount 才能恢复，disposed 终态不可重开。
尚未接入应用任务集合/挂载回调，首次 attach、场景渲染与观看入口仍未完成。
最终完整串行回归 1688 passed、28 ignored；Clippy、fmt、2597 文件行数门禁
及 diff 检查通过。未构建交付 EXE、提交、推送或合并主干。

N08/N09 已实现已有 Surface 附件的认证 HTTP remount，校验固定服务/设备、
路径转义、返回绑定及场景快照身份，拒绝 generation 回退。不提前上报
active；首次挂载能力报告、生命周期 owner、场景渲染和观看入口仍未完成。
本轮默认串行回归 1685 passed、28 ignored；Clippy、fmt、2594 文件行数门禁
及 diff 检查通过，无新 EXE、提交、推送或合并主干。

N08/N09 已确认观看依赖真实 Surface 绑定，新增认证挂载响应的设备/节点/
实例/生命周期校验，以及绑定到 selected Live 目标的构造。未伪造 Surface
ID。Surface HTTP 挂载与生命周期 owner、目录选择、观看按钮仍未完成。
本轮默认串行回归 1683 passed、28 ignored；Clippy、fmt、2592 文件行数门禁
及 diff 检查通过，无新 EXE、提交、推送或合并主干。

N09 业务状态已通过 ViewerSession 最新值槽位与已有帧通知交付到原生 Host；
同锁消费帧/业务，暂停或关闭清缓存，旧许可不能发布。无新帧也能通知，不
增加等待句柄。可视业务面板、会话选择及观看启动菜单仍未完成。
本轮全量串行回归 1680 passed、28 ignored；最后收紧 Host 读取许可并扩展
实际 Host 定向测试后通过。无新 EXE、提交、推送或合并主干。

N09 Business 已从认证 attach 响应经 Prepared 接入控制 owner/Runtime，观察
与审计更新纳入可靠游标提交。业务或 consumer 失败保持旧业务/游标并关闭
媒体；缺失业务快照拒绝加入。UI 状态交付、会话选择和观看启动仍未完成。
本轮默认串行回归 1677 passed、28 ignored；Clippy、fmt、2589 文件行数门禁
及 diff 检查通过，未构建 EXE、提交、推送或主干合并。

N09 新增触发注册配置校验与 Business 跨观察/审计原子事务；观察合法但审计
失败时整批回滚。注册快照只读共享，观看端不执行输入或 Surface 动作。
attach 原始业务快照传递、控制 owner/Runtime 接线及观看入口仍未完成。
本轮默认串行回归 1675 passed、28 ignored；Clippy、fmt、2589 文件行数门禁
及 diff 检查通过，未构建 EXE、提交、推送或合并主干。

N09 新增触发审计类型与 256 项有界镜像，支持同键预留转最终结果、迟到
预留抑制、快照恢复和原子增量子批次。不执行 Surface 动作；尚未接入
attach/完整控制 consumer。观察能力报告与汇总优先级也已实现，触发配置、
跨业务事务和观看入口仍待完成。
本轮默认串行回归 1671 passed、28 ignored；Clippy、fmt、2586 文件行数门禁
及 diff 检查通过，未构建 EXE、提交、推送或合并主干。

N09 新增观察数据严格合同、attach 观察快照解析与有界原子增量存储，保留
主干 256 ID/64 KiB 值限制以及独立序号连续性。此模块尚未接入控制 consumer；
观察能力汇总、触发配置/审计和完整观看入口仍未完成。
本轮默认串行回归 1665 passed、28 ignored；Clippy、fmt、2583 文件行数门禁
及 diff 检查通过，无新 EXE、提交、推送或合并。

N09 已接通原生“刷新远程会话目录”菜单：使用当前服务的有效租约启动，
成功更新共享目录和条目数；服务切换、清除授权、重新授权和退出清空旧结果。
静默取消不会遗留 busy。目录条目选择、观看启动及完整业务 consumer 尚未
完成；下列“启动命令尚未接入”是之前阶段状态，不能据此认定观看入口完成。
本轮默认串行回归 1661 passed、28 ignored；Clippy、fmt、2580 文件行数门禁
及 diff 检查通过。未构建 EXE、提交、推送或主干合并。

N09 目录 Job 已由远程 Sessions 集合持有并接入 Application 完成轮询、消息
等待和退出 drain。目录与观看合计最多四项，总等待上界仍为 63；缓存目录
最多一份，刷新/取消清空旧结果。选择 UI、启动命令及完整业务 consumer
仍未接入。下面保留之前阶段的记录，不代表这些缺口已经完成。
本轮全量串行回归 1660 passed、28 ignored；最后两处窄调整后集合三项
定向测试再次通过。未构建 EXE、提交、推送或合并主干。

N09 新增目录后台 Job，支持固定 origin 结果、完成句柄、取消及迟到结果抑制。
目录和观看共享四槽位，已完成但未回收任务仍占容量。新增三项生命周期测试；
Application 目录 owner、选择 UI 与业务 consumer 仍待接入，没有新的可操作
入口、EXE、提交、推送或主干合并。下方记录为此前检查点。
本轮默认串行回归 1659 passed、28 ignored；Clippy、格式检查、2580 文件
行数门禁和 diff 检查通过，ignored 特殊环境测试未运行。

N09 已提供有界认证会话目录及 Runtime selected 身份约束。首次选择允许
非成员显式加入，来源/epoch 变化在 POST 前失败；续签成员规则不变。目录
后台任务、选择 UI 和完整业务 consumer 尚未接入，不代表可操作入口完成。
新增六项测试，默认串行回归 1656 passed、28 ignored；Clippy、fmt、2578 文件
行数门禁及 diff 检查通过。无新 EXE、提交、推送或主干合并。

N09 远程会话集合已接入 Application 的 Live 结算、消息等待和退出 drain。
容量四项含停止未回收任务，总等待句柄上界 63；退出继续泵消息直到回收。
终态区分正常关闭、撤权和窗口错误。菜单/会话选择及具体业务 consumer 尚未
接入，远程输入、重连和完整用户验收仍未完成。
新增三项测试，默认串行回归 1650 passed、28 ignored；Clippy、fmt、2575 文件
行数门禁及 diff 检查通过。无新 EXE、提交、推送或主干合并。

N09 远程 Session 已配对 Runtime 与窗口，统一 Ready/完成/帧等待句柄，实现
就绪后创建窗口、用户关窗停止、撤权原因保留及一次性终态回收。真实 HTTPS/
隐藏 HWND 和 WSS 终态已联测；Application 集合/等待/退出及菜单仍待接线。
默认串行回归 1647 passed、28 ignored；测试断言 lint 修正后两项定向重跑
通过。Clippy、fmt、2572 文件行数门禁及 diff 检查通过，无新 EXE 或主干合并。

N09 已新增远程 RelayWindow，处理隐藏创建、捕获排除、绘制/缩放、窗口关闭和
撤权，并提供帧事件句柄。隐藏窗口测试覆盖资源及重入安全；Application
会话集合、等待集、菜单入口、输入和完整隐藏资源策略仍待完成。
新增三项测试，默认串行回归 1645 passed、28 ignored；Clippy、fmt、2570 文件
行数门禁及 diff 检查通过。未构建新 EXE，未合并主干。

N09 远程像素已实现 Direct2D/DirectComposition 呈现 Host：同尺寸位图复用、
最新帧上传后释放 CPU 帧，许可失效清理显示资源。真实隐藏 HWND/GPU 读回
验证颜色/透明度、缩放留边、资源复用及撤权；实际观看窗口和应用接线仍未完成。
新增两项测试，默认串行回归 1642 passed、28 ignored；Clippy、fmt、2568 文件
行数门禁和 diff 检查通过。无新 EXE、提交、推送或主干合并。

N09 远程媒体已提供新帧与许可变化的 Win32 合并事件，poll_frame 同锁复位并
领取最新帧；关闭/暂停也唤醒且旧代次不会制造通知。真实 HTTPS/WSS 测试
已使用事件等待替代缓存轮询。远程 HWND 与应用等待集接线仍待完成。
新增四项测试，默认串行回归 1640 passed、28 ignored；Clippy、fmt、2565 文件
行数门禁及 diff 检查通过。没有新 EXE 或主干合并。

N09 媒体终态已直接唤醒控制请求：先保存原因再撤销租约，关闭和设备撤权不再
被控制请求超时掩盖；显式取消保持 Stopped。真实 WSS 测试在控制请求悬挂时
验证两类终态均在两秒内回收并抑制迟到 Ready。桌面入口和重连仍待完成。
默认串行回归 1636 passed、28 ignored；Clippy、fmt、2563 文件行数门禁和
diff 检查通过。未构建新 EXE，未提交、推送或合并主干。

N09 控制 Runtime 已连接授权媒体 Runtime：首批重放后启动 Device WSS，复用
固定 origin/TLS/地址；控制退出在后台回收网络和 codec，租约撤销禁止旧帧。
真实回环测试覆盖 HTTPS 加入到 WSS 像素发布及撤权回收。Ready 不等于首帧，
桌面窗口/输入接线、具体业务消费者、即时媒体终态唤醒和重连仍待完成。
新增两项测试，默认串行回归 1635 passed、28 ignored；Clippy、fmt、2561 文件
行数门禁及 diff 检查通过。未构建新 EXE 或合并主干。

N09 控制运行时已支持配置型启动：租约在后台执行 CA/DNS/Client 准备，前后
复核凭据，不创建身份 Store。错误 origin、过期和预撤销在准备前失败。
桌面观看命令与媒体 Runtime 的实际接线仍待完成。
新增两项测试，默认串行回归 1633 passed、28 ignored；Clippy、fmt、2559 文件
行数门禁及 diff 检查通过。双栈地址回退耗时仍待性能优化，详见迁移记录。

N09 Live 控制后台 Runtime 已串联加入请求、首批消费、Ready 许可通知与持续
轮询；授权撤销可中断请求，异常退出撤销媒体，完成句柄支持非阻塞回收。
就绪和退出分别提供 Win32 等待句柄，停止后的迟到 Ready 不再交给调用方。
尚未接入 Application/媒体 Runtime 和会话选择 UI，非状态业务消费仍待实现。
新增四项测试，默认串行回归 1631 passed、28 ignored；Clippy、fmt、2558 文件
行数门禁和 diff 检查通过。无新 EXE、提交、推送或 main 合并。

N09 授权缓存已提供有界后台租约，每个串行请求链独占取消状态；缓存替换、
取消、成功重新配对及关闭同步撤销旧租约和已绑定媒体。失败配对保留旧授权，
跨 origin 相互隔离。此为桌面 Viewer 后台任务的生命周期基础，尚未完成
任务接线、到期定时器及在线用户流程验收。
新增五项测试并扩展三项已有授权测试；默认串行回归 1627 passed、28 ignored，
Clippy、fmt、2556 文件行数门禁及 diff 检查通过。未构建新 EXE 或合并主干。

N09 控制 owner 已完成批次校验、显式业务消费、媒体许可与游标提交联动。
首次消费成功前媒体暂停；普通状态更新保留许可和缓存；成员缺失/closed、
协议或消费失败、HTTP 错误关闭旧观看，404 另做认证成员关闭确认。后续
仍需桌面任务编排、授权缓存失效联动、非状态事件业务实现及会话选择 UI。
该组件完成不等于完整 Viewer 流程或 N09/N12 验收完成。
本次新增十项测试，默认串行回归 1622 passed、28 ignored；全目标 Clippy、
fmt、2553 文件行数门禁及 diff 检查通过。没有新 EXE 或主干合并。

N09 已串联 discovery 目标查找、已有成员 actor 专属 requesterControl 恢复、
安全下一序列、attach 和 bootstrap cursor；续签拒绝已移除成员且要求服务端
保留现有成员检查。显式 resume 只有验证成功后返回新序列，不从 409 猜测。
Snapshot 缺少 viewerDevices 不再等同于空成员。新增五项测试通过，最终默认
回归 1612 passed、28 ignored。Prepared 不激活媒体，控制 owner/撤权/桌面
许可、可见会话列表与 daemon-busy 重试仍待完成。

N09 已新增实际认证 Live HTTP 请求：member snapshot、Viewer attach、加入
事件游标、持续 events 和权威关闭确认。复用固定 origin HTTPS、每请求 Device
nonce；401 在错误体读取前分类。持续流拒绝 reset=true，首次加入仍验证连续
历史后缀。五项真实 TLS 合同测试通过，完整默认回归 1607 passed、28 ignored。
尚未连接 discovery/actor sequence 基线、resume、控制 owner、授权缓存和
桌面媒体许可；不是完整 Viewer 用户流程。详见迁移文档最新条目。

N09/N12 已捕获此前 TLS 握手停止延迟的具体失效路径：副本 socket shutdown
返回 10057，真实读取仍等待到三秒 deadline。Windows 流现改为非阻塞 I/O 与
网络/取消事件共同等待，不再靠副本 shutdown。加强后的握手取消用例连续
50 次独立运行通过，11 项流测试通过，完整默认回归 1602 passed、28 ignored。
已复现路径得到修复；N12 的跨环境性能与稳定性验收仍不能由这些测试替代。

N09 原生菜单已接入明确服务地址选择、配对、批准等待、续签与本地取消。
授权任务由 Application 持有，完成句柄进入消息等待集；退出继续泵消息回收。
地址目前仅在内存中保存，尚未联动远端 Live 控制/媒体许可及能力目录；系统
控件尚未完成 Neuro 主题和人工 UI 验收。新增四项定向测试通过。

本轮默认回归曾出现既有 TLS 握手取消耗时约三秒的失败（1598 passed、1 failed、
28 ignored）；当时后续完整重跑 1599 passed、28 ignored 并未修复间歇问题。
本轮定位与修复证据见上段；仍没有新 EXE 或 main 合并。

以下为此前阶段记录，涉及“尚未接入”的描述以本段和迁移文档最新进度为准。

N09 Configured Task 已把显式服务 origin、隔离 profile、CA 快照和 Windows
地址解析接入授权后台线程，再执行配对/批准/续签。真实桌面连接选择、菜单与
消息等待集仍未接入；不能称为用户界面流程已经完成。

N09 已实现 Windows 异步地址解析与原生取消事件，正常完成/取消后回收 OS 结果。
五秒等待并非异常 provider 下的硬返回上限；极端完成等待失败会保留操作内存，
避免悬垂指针。后台配置准备和真实桌面授权入口尚未接线。

N09 已有按 origin 的授权结果 Owner：旧 pending 撤销后永不重新发布，配对成功
清除同 origin 缓存、失败保留原缓存；完成事件不携带凭据。尚未联动既有媒体
许可，也未接入真实桌面菜单/配置准备/等待集，不能视为用户可操作流程已完成。

N09 授权后台 Runtime 已组合真实 Store/HTTPS/批准等待，分别提供显式配对、
首次等待、普通续签 Task；最多四个未回收任务，Windows 完成句柄与 try_join
可用于 UI 非阻塞回收。取消后不交付旧结果；桌面消息循环与旧授权撤销仍待接入。

N09 首次授权已有九十秒总期限的批准等待：仅明确待批准才重试，间隔 250 ms，
请求共享剩余总预算。Cancel 同时唤醒等待与中断 socket；续签仍是单次请求。
任务运行时/UI 完成句柄、旧授权撤销、实际用户目录与 Live 控制流仍待接线。

N09 显式配对已通过真实 HTTPS 提交公钥，核对 registry 公钥/设备 ID，并可经
Store 事务保存默认关联。只注册、不批准、不签发 token；网络成功而本地保存
失败会明确报错。旧授权撤销、批准轮询、实际用户目录与 UI owner 仍待接线。

N09 已有真实 HTTPS 单次设备签发/续签客户端：两次 POST、固定 origin/信任根、
无重定向、响应限额、绝对期限及 socket interrupt；没有常驻异步运行时。
仍需应用 owner、地址解析/代理策略、批准轮询、用户目录与 Live 控制流/UI 接线。

N09 单次设备会话协议已连接 challenge/proof/签发响应：严格区分待批准与失败，
校验设备、token、到期时间，并限制凭据只用于原 HTTPS origin。协议测试不等于
真实 HTTP 配对验收；传输客户端、批准轮询、用户目录和应用 UI owner 仍未接通。

N09 显式目录的设备身份存储已实现：只读续签入口不创建/迁移，显式初次入口创建
或将 legacy 保护为 schema 2；同目录暂存刷盘后替换，错误不重建密钥。合作实例
独占 lock、快照 owner/hash 检查已接入，但不是对外部非合作写入的原子 CAS。
实际用户目录接线、HTTP 配对/签发/续签、控制流与 UI 仍未完成。

N09 已有 schema 1/schema 2 身份解析、Windows 用户级 DPAPI 保护、公私钥一致性
检查及主干兼容 Ed25519 challenge proof。尚未自动读写用户身份文件，也未接通
HTTP 配对/签发/续签；本地签名成功不等于已获得远端授权。

N09 已有后台 Viewer Runtime，持有媒体连接和 codec worker，支持握手期 interrupt、
原代次取消、完成句柄与回收；每进程最多八个网络任务。应用命令/UI owner、真实
授权/bootstrap、控制流、重连与 HWND 接线仍未完成。TCP connect 以两秒期限退出，
不是可即时取消的系统调用；活跃 Runtime Drop 仍可能阻塞，UI 必须等待后 try_join。

N09 Trust::load_default 已组合编译期 Mozilla 公共根与 origin 隔离的操作员 CA；
webpki-roots 1.0.9 与主干锁定版本一致，许可证/依赖合同及实际 Advisory 扫描完成。
全仓保留 15 个漏洞 ID，native 121 包无报告漏洞；不是零漏洞或严格发行验收声明。
应用 owner 的完整接线、远端设备授权、控制流/重连/HWND 仍未完成。

N09 操作员 CA 配置已实现有界、严格 PEM 加载和精确 HTTPS origin 隔离，可通过
local_viewer_with_trust 接入媒体 TLS。规范环境变量优先，缺项/损坏不静默降级；
默认公共根入口已补齐，Windows 平台根与 LAN 信任发现不在此实现中。

N09 媒体发现复用默认清单候选与有界读取，不要求 hookBridge；本机 none/bearer
及固定 device-000-local 已接入请求构造。localhost、IPv4/IPv6 loopback 与本机
HTTPS 匿名兼容主干，连接地址必须仍在回环。远端清单不自动变成 Device 授权；
真实配对/签发、CA 配置加载、控制流/重连/HWND 接线仍未完成。

N09 已有固定 origin 的 viewer 请求与实际 TCP/TLS/WebSocket transport，带随机
Device nonce、敏感认证头、真实子协议选择验证、每步截止时间和连接 interrupt。
信任根与解析地址由调用方显式提供；发现/系统 CA 加载、Device 凭据申请与续签、
控制流激活、重连及 HWND 仍待接入。TLS 回环使用虚构 token，不等于真实设备认证。

N09 已有已准入 WebSocket 的单步服务，真实回环 WS→Frame→worker→像素缓存
通过；接管 Vec 不复制整帧，精确 Device 撤销 Close 与普通断开区分。可信端点/
TLS/凭据/子协议握手、重连和 HWND 尚未完成，不是远端认证验收。
主动 Ping/RTT 已接入单步服务：两秒探测、单在途、精确 Pong 匹配，十秒过期清除
旧测量并重新探测。真实回环覆盖 Ping/Pong，不等于远端网络性能验收。

N09 已有真实后台媒体 worker：有界输入/回复、停止唤醒、完成句柄和 join，
实际后台 H264 发布及 idle 关闭回收门禁通过。尚未接入 socket、真实认证或
HWND；Drop 的阻塞 join 只是兜底，UI 应等待完成后 try_join。

N09 媒体 Pipeline 已把固定 Permit、Receiver、静态解码与会话缓存提交串联；
实际 H264 发布/取像素、暂停与关闭后 check 释放 codec、新代次恢复门禁通过。
真实 socket 循环尚未接入，因此不声称网络取消会自动调用 check 或已完成远端撤权。

N09 已有 ViewerSession 媒体发布 gate，默认暂停，Permit 绑定 owner/授权代次；
暂停、成员复核失败及关闭取消旧代次、清缓存，同锁拒绝迟到像素提交。低 revision
不覆盖新状态，迟到 open 不复活终态。真实认证、worker 和 HWND 接线仍待完成。

N09 控制流已具备整批 prepare/commit 与 owner 绑定，避免错误后半批推进 cursor；
权威 member snapshot 的 open/closed 判定已迁移，普通 404 不当作关闭。实际
HTTP、业务事件应用和会话生命周期接线仍未完成，不构成认证或撤权执行证据。

N09 已迁移 viewer 快照身份、来源固定、续期成员关系及本次加入事件的连续历史
校验。JSON 有界且不直接信任 next；尚未连接真实 Device 认证、HTTP 或控制事件
应用，结构校验通过不能当作授权证明。

N09 系统 MF decoder 已接通 AU 校验/提交、有界输出、stream-change、NV12→PBGRA
与共享像素预算。64×64、680×430 实际 IDR 解码和失败释放门禁通过；两尺寸的
24 帧 IDR/delta 及第 12 帧重建共 72 帧灰阶内容检查通过，最大误差 3。
网络/viewer 接线及长时运行仍未完成。32×32 可协商但不出帧的系统边界已
修正为配置时拒绝低于 48×48，wire 小图仍允许，连接 owner 的降级接线待完成。

N09 H264 已迁移 baseline AU/SPS/PPS/VUI 的 codec 前限额检查，实际 SPS 尺寸必须
匹配 wire；没有新增第三方 codec。该前置检查现已接入 MF 的 AU 提交路径。

N09 H264 已有参考链/四项描述符上限、关键帧请求及 sticky fallback 状态模块；
已通过 Receiver 与实际 MF decoder 组合，断链释放/IDR 重建、图片释放、一次性
降级及终态门禁通过。授权 epoch 由调用方提供；尚未与实际 socket、授权存储和
viewer 联合，不能视为视频媒体功能已完成。

N09 已有 raw BGRA/JPEG 到原生 Image/Pixels 的静态媒体解码入口，JPEG 校验实际
容器和声明尺寸，输出占用共享像素预算。H264/HDR、网络任务与呈现接线仍未完成。

N09 viewer 已有 epoch/帧号递增准入、三帧缓存和同锁关闭清理，支持 warmup 忽略
及带租约的唯一帧移交。现复用 FrameStore 同时支持网络 Frame 与已解码 PixelFrame；
实际 H264、WIC JPEG/raw 输出到像素缓存的同存储移交已验证。网络会话授权和
实际 UI 尚未接入，缓存 cursor 不能当作授权 epoch。

N09 已有按实际协商 profile 准入、共享字节预算约束的单帧分片接收组件；拒绝超长
或截断，不交付部分内容，取消和帧释放归还预算。网络/授权/媒体验证/UI 尚未接线。

N09 已新增与 main 布局一致的 NLLV v1 帧封装和头准入，支持 raw BGRA/JPEG/H264
的元数据边界。未连接网络、授权、媒体校验/解码或 UI，不代表 LiveRelay 已可用。

同进程 mixed-hosting 的嵌套控件命中已增加实际回归：144 DPI PMv2 root 下的
96 DPI 子面板右侧孙控件能够通过生产路由命中；本轮未发现需要修改的路由缺陷。

mixed-hosting 的 PMv2 父窗口/unaware 子控件外拖动已修复：原系统转换在控件外
返回 Transform，现在同显示器 PM root 可通过内部锚点与真实 DPI 比例回退。
真实 144→96 DPI 回归通过；跨进程和跨显示器情形仍未完成验收。

源窗口 unaware/system-aware/PMv2 × 调用线程三种 awareness 的九种组合已通过
真实坐标测试，当前源 DPI 为 96/144/144；中心和四角、物理裁剪及失败后上下文恢复
均验证。跨进程不同 awareness 的父子窗口和不同缩放的多屏迁移仍缺证据。

WinForms 遮挡 push-button 补偿已接入普通 Backend 发送路径，取消释放不补偿。
纯策略、类型筛选、自建模拟类及真实 .NET Framework WinForms fixture 已有验证：
普通点击触发一次 Click，拖出返回及取消释放不增加 Click，并清空鼠标按下状态。
其他 WinForms 版本、自绘控件和混合 DPI 兼容性仍未完成，不能外推至任意应用。

Live HWND 控制菜单、键鼠翻译、黑边准入和捕获/失焦撤销已接入 Session/worker，
默认仍关闭，区域 Live 不允许控制。真实 WGC+隐藏/恢复+Live UI+自建源子控件联合
门禁已通过：点击触发实际新帧像素改变，失焦释放，关闭前释放 held 输入并回收线程。
尚未覆盖任意用户应用、真实 CLR 与 Live UI/WGC 的完整联合、混合 DPI 或物理键鼠端到端，N07 未完成。

### 历史阶段边界

以下记录反映各阶段当时状态；已连接的入口以上方最新状态和迁移文档为准。

Session/Application 已连接输入 owner 的显式入口、回复结算、完成等待和关闭回收；
源保护待输入退出后释放，采集先退出不提前删除会话。窗口交互入口和消息翻译仍未
连接，默认不允许输入，区域 Live 明确拒绝控制。

有界异步输入 worker 已实现：独立线程构造实际 Backend，16 项未消费命令上限，
满载撤销、停止优先、事件唤醒和退出回收。Application/Session/UI 等待集与控制入口
仍待接线，不能把 worker 可运行当作用户已经能通过 Live 界面控制源窗口。

Controller 已通过实际 Win32 Backend 组合安全、子目标路由、坐标及消息投递；
Target 支持共享句柄所有权，取消使用最后有效坐标。生产 worker/UI、失焦接线、
WinForms 特例补偿及 relay 权限仍未完成，不能算作 N07 用户功能完成。

输入路由已有 root 内有界深层命中、焦点读取、保留目标及失效 root 回退；尚未
接入 Controller/worker/UI，也未完成任意用户应用兼容和混合 DPI 命中验收。

输入坐标层已有客户区归一化、物理 frame 裁剪、源/子目标 DPI 桥接及作用域恢复；
尚未连接实际输入 worker/UI。真实混合 DPI、多屏和带边框裁剪联合验收仍缺失。

子目标 Destination 已持有原进程句柄并校验 root/后代关系、HWND/PID/TID 和独立
权限；send_to/release_to 已能投递到已绑定子 HWND。自动命中/焦点路由、DPI 转换、
透明样式协调及整体输入接线仍未完成，不能把手动绑定子目标当作完整控件交互。

Win32 root 消息原语已有安全预检、同线程拒绝、键鼠编码和短超时投递；仍缺少
子控件路由、坐标转换与 Controller/worker/UI 接线，不代表输入投射可用。

输入状态层已实现显式启用、不可重放序号、32 键/3 鼠标键上限、按下目标保持及
撤销释放报告；真实 transport、worker 和 UI 接线仍缺失，N07 输入投射仍未完成。

输入安全预检已有原生模块：复核 Target、默认桌面、同 token session/用户 SID 和
完整性级别。尚未接入显式授权与输入 transport，不能算作输入投射用户功能完成。

N07 输入投射当前只新增显示几何基础：D2D 绘制与帧像素映射共用 Viewport，拒绝
黑边和非法坐标；Host 在成功呈现后更新序号/几何，拒绝 resize 未重绘的旧快照，
暂停/关闭清空。尚无输入授权、源 HWND 转换和发送链，不计为输入投射功能完成。

窗口 Live 已通过 live_visibility_owner 接入实际 Application/Session：同源共享登记
和原始快照，ACK 后隐藏，最后 lease 退出恢复；应用等待集合包含恢复 worker/子进程，
关闭同时结算采集和退休，最后源结束即退出恢复 helper。区域 Live 不启动该子进程。
输出关闭/任务结束后不因迟到 ACK 再隐藏源。shell 激活召回、真实最小化应用兼容、
输入投射等仍未完成，不能宣称完整 N07 对齐。以下为保留的阶段证据，旧“尚未接线”
结论只描述该检查点，不代表本节最新状态。
召回状态机与共享源 owner 已联合验证：恢复可见性不结束同源会话，退休后不再隐藏，
空登记 helper 可先于 Live 会话退出。无意图前台变化、隐藏前及取消意图被拒绝；
OS shell 输入 watcher 仍未接线，不能把测试注入意图当作真实任务栏/快捷键验收。
后续 watcher 接线现已完成：物理手势分类、注入标志排除、前台/最小化事件唤醒、
同线程注销及 Application 等待事件已实现。尚未完成真实 shell 手动操作验收，
不可将分类/事件夹具当作实际任务栏点击或 Alt+Tab 的运行证据。
WGC/可见性联合门禁现已在自有纯色源通过：普通与初始最小化源隐藏后仍持续采集
颜色更新，召回后 Live 不停止，取消后线程和恢复进程均退出。连续三轮通过；
召回意图仍由夹具注入，用户应用兼容、真实 shell 手势和长期曲线未据此结单。

父端认证已有真实子进程验证：同 executable、直接父 PID、创建时间与输入 pipe server
核对通过，输入/ACK 输出任一非管道及错误 PID 拒绝，保留原父同步句柄且不申请终止权限。
Hello/Ready 的方向、session、schema 和有界 framing 已有纯协议测试；同进程真实
overlapped 管道进一步覆盖握手往返、读超时取消后缓冲区复用、对端关闭与退出句柄唤醒。
写入背压另有不消费对端的超时回归。I/O 使用绝对截止时间，无后台读写线程或空闲轮询；
超时可能已经部分传输，不能假定整帧重发安全，取消清理不承诺硬实时上界。
生产跨进程握手和父死恢复循环仍未接通，不能据此启用外部源隐藏。
会话处理增量已接入认证、握手与 Service：真实同进程管道覆盖登记 ACK、相同请求
重放、退休与空登记关闭；半帧超时和断管不会丢弃已确认登记，失败流不能再次读取。
这不是生产子进程验收，恢复入口的原父句柄等待及窗口恢复仍需跨进程运行证明。
跨进程补充：同一测试 executable 的实际子进程通过继承的双向 overlapped 端点完成
Session::accept、不可绑定源的 Rejected ACK 及空登记 Shutdown。接收端增加内核
FileModeInformation 检查，不能把任意同步 stdin 当作 overlapped。私有子进程入口
已接通，并在父死后的恢复失败阶段退避保留记录；同 executable 测试已使用该 runtime
函数完成正常关闭。实际 EXE 另验证非法 PID、混用参数和 NUL 端点被拒绝；合法生产
父端启动、故障重试及父死后的真实窗口恢复仍未完成运行验证。
父端 Client 增量已有真实测试子进程验证：同 executable 启动/握手、合法拒绝 ACK、
空登记 Shutdown/join、握手失败回收、子进程退出后拒绝请求。本地非法请求不占用序号；
请求 I/O 失败封闭通道，Ready 后 Drop 不杀可能持有记录的恢复进程。生产 Live 后台
调度尚未接入，不能由此推定完整源隐藏/恢复链路已就绪。
三进程恢复门禁新增实际 Windows 证据：自有隐藏窗口留在观察者，登记父进程先关闭
管道并存活 200 ms，确认没有提前恢复；正常返回及直接 process::exit 后均还原原始
快照且恢复子进程退出。测试只修改自有窗口的 TRANSPARENT 样式，不等于生产透明
隐藏、用户应用兼容或强制 TerminateProcess 路径已验收。
恢复启动句柄隔离已通过实际反例验证：父端创建无关可继承事件，旧启动方式允许子端
触发该事件，改为 CreateProcessW 显式句柄白名单后不再触发。白名单仅含私有协议
端点与 NUL，不包含父端日志管道或任意其他可继承句柄；其他模块启动行为不在此证明内。
后台 owner 增量：同一时刻仅允许一次握手/请求，未取响应前不接受下一请求；空闲
没有 worker。真实子进程覆盖开始、请求、关闭、错误后 Client 返回和响应单次交付。
任务完成与子进程退出句柄已经暴露，尚未接 Application 的等待集合及关闭结算。
原生隐藏后端已补充：未 arm 拒绝，同源 peer 重复调用只隐藏一次，alpha=1、样式和
最后 lease 的快照恢复已在自有隐藏窗口验证。三进程门禁也已改用真实隐藏后端，而不
再仅设置 TRANSPARENT。可见用户窗口、最小化源、实际 UI 调度尚未完成运行验收。
恢复登记协调层新增 Journal/Ticket：16 源上限、单请求在途，登记 ACK 前不授权隐藏，
退休 ACK 前保留本地快照，取消和最后外部 Ticket 释放也进入结算。自有隐藏窗口联合
测试连接真实子进程与共享 lease，检查恢复、待退休拒绝新隐藏和下一代登记。
Unknown 责任不会因 Client 句柄缺失而退休。请求 panic 已保留 Client/原进程句柄并
封闭通道；故障收尾只在全部已发送源恢复/取消并请求退休后终止自建恢复子进程，
确认退出前不清快照。Application 接线和应用退出消息泵仍未完成，不启用用户源隐藏。
关闭接线增量：Application 已对现有 Live 采集会话执行停止、泵消息、完成后回收，
不再直接 clear/join；Journal 仍未进入应用等待集合，不能视为完整恢复关闭链。
源消失时本地 lease 已采用与 Binding 相同的无写入判断，真实登记在源 HWND 销毁后
能够正常 Retire/Shutdown，无需故障终止；待退休期间本地责任仍保留。
应用错误出口已纳入统一 shutdown：非零 WM_QUIT 或主循环 `?` 返回也先清理任务。
此范围是 Result 错误返回，不将其扩展为任意 panic/强制进程退出保证。

独立恢复协议已新增有界 Register/Retire/Shutdown 与 ACK codec，携带 session、请求序号、
登记 id 和源进程创建时间，拒绝跨 session/错序 ACK、截断及非法结构。这里只是数据合同；
生产恢复子进程启动、session 握手及父死恢复仍未接通，父端身份认证基础已补充。源句柄绑定已新增真实 Windows
后端：登记时核对 PID/TID/创建时间并保活 Target，已在自有隐藏 HWND 与 Service 联合验证；
尚未验证实际跨进程或可信通道。幂等服务状态机已新增，
覆盖 ACK 丢失重放、迟到退休/旧 id 拒绝、有界登记、未清空记录不得 Shutdown，以及恢复
失败保留 owner 重试；这些注入回调测试不代表真实独立恢复进程已运行。

源窗口主动召回新增纯状态模块 `live_reclaim.rs`，保留显式 shell 意图的 2000 ms 有效期
与每源单次消费，拒绝普通/迟到激活、取消、过期、未来时间和旧序号。它尚未连接宿主
输入来源、外部窗口共享可见性及崩溃恢复，不代表逻辑隐藏或任务栏召回已迁移完成。
源窗口恢复快照新增只读 Win32 后端，保存 rect/placement/iconic/扩展样式/layered 属性，
读取前后校验原 Target。新增恢复写入子模块，已在自有隐藏窗口验证样式/透明度/
topmost/几何还原和重复恢复；独立保留 visible，避免隐藏窗口被 placement 意外显示。
实际外部源隐藏、跨进程失败与崩溃恢复仍未接通。共享 visibility lease 已新增独立
Registry/Lease，验证首次快照不覆盖、最后 peer 恢复、主动召回保留 peer、失败重试和
有界准入。恢复记录退休失败保留状态且不重复覆盖已恢复窗口；真实 journal 尚未接入。

Live 输出窗口已支持最小化/零尺寸释放呈现资源、保留最新共享帧并按需恢复。WARP 测试
覆盖无新帧恢复、新帧优先和关闭回收，隐藏 HWND 验证最小化消息路由。捕获仍持续，
源应用最小化/逻辑隐藏、可见交互和实际内存曲线仍未验收，不据此勾选完整 N07。
另有显式运行证据：自建可见输出窗口连续三次真实最小化/恢复，通过无 surface 保留、
最小化 paint 不重建、单次发布帧恢复和关闭回收。该测试不操作外部源窗口，命令见迁移计划。

本轮新增 Live 实际运行证据：显式 ignored 测试只捕获自己的可见纯色 HWND，首帧、颜色
变化、旧快照稳定、128×96 到 256×144 重建、同设备 GPU 呈现及源关闭回收在修复后连续
三次通过。首次发现 HWND 销毁后 WGC Closed 未到、worker 等帧不退出，已用限定源身份的
销毁 WinEvent 唤醒修复；普通回归另有隐藏 HWND 事件单次性/注销测试。该证据不覆盖真实
用户窗口、菜单交互、多屏/HDR、最小化/隐藏与资源基准。命令和边界见迁移计划。

跨进程补充：独立纯色测试子进程的正常关闭及强制退出均已验证首帧、进程句柄唤醒、
句柄退休、线程回收和帧释放。应用最多等待 32 个去重的 worker/源进程句柄，不新增轮询。
Live 启动要求调用方持有显式 COM apartment，并保持到全部任务退出；测试宿主已对齐
应用的持续 STA 生命周期。这里只证明受控夹具，不等同于任意应用兼容性或 N07 完成。

固定基线与任务定义见 [迁移计划](WINDOWS_NATIVE_MIGRATION.md)。本表是首轮源码核对结果，
尚未穷尽所有调用点；每组仍需细化到操作与失败分支，不表示 N01 已完成。
旧实现未在本轮修改。测试入口可定位已有断言，但测试存在不代表新实现已经通过。

## 多文档宿主准备

长截图已有独立的已定位帧合成核心：BGRA 帧共享存储，支持水平/垂直及双向扩展，
重复覆盖不替换早到内容；先验证资源/几何边界，按片段一次输出，按行取消。单帧或
完全重复无需输出复制。已有非均匀像素、反向/包含、预算、取消及边界测试；没有
完整自动重叠识别、固定栏、滚动捕获或 UI 接线，不能宣称长截图已迁移完成。
现有精确匹配快速路径可识别等尺寸 BGRA 帧的唯一双向位移，像素复核后才能返回偏移；
多候选返回歧义。该路径不等于主干包含近似匹配和固定栏处理的完整分析器。
近似匹配已有独立有界搜索/密集复核内核，支持轻微 BGRA 变化的双向与零位移分析，
低纹理、误差超限或候选接近时拒绝推断；固定栏和动态区域权重仍缺失，未接 UI。
固定边缘现有成对分析：检测首尾完全静止区域，再对借用的内部区间识别非零运动；
未通过时不返回裁边依据。合成已有首尾各保留一次及逐帧固定栏复核，双向内容通过
逐像素测试；跨帧确认和非静止固定栏仍缺失，未接 UI。
纯数据会话已加入累计位移、重复/失败不推进及两次运动确认固定边缘，后续帧继续
复核首帧边缘，确认后可合成。没有实际捕获/自动滚动/预览/UI，非静止固定栏仍缺失。
后台分析/合成已有单所有者 SessionJob：最多一个在途命令，取消后不发布结果，Drop
回收线程；生产端采集、滚动和 UI 尚未接入。测试仅用合成图和真实本地 worker。
选区后台采集适配器已有代码，显式命令触发区域 GDI 捕获并追加 Session；未接用户
入口、自动滚动和预览，未执行真实桌面验收。测试仅注入合成源和检查无采集构造。
手动入口现已接线：垂直/水平选区、原生采集/完成/取消窗口、新钉图交付及显示布局
变化取消；隐藏窗口回归使用合成源。自动滚动、图像预览与真实桌面交互仍未完成验收。
显式图像预览已接后台有界缩略图和 GDI 自绘，不先分配完整长图；隐藏窗口通过
预览后继续完成的回归。自动滚动与真实跨应用/多 DPI 可见交互仍未验收。
单步窗口滚轮发送已有有界底层代码和纯参数/准入测试，仍未接自动循环/UI，未向
实际应用发送过滚动消息；消息响应和真实内容位移不能视为已验证。
单步“滚动并采集”已连接界面与 worker，含可取消固定等待、目标前后复核、动作次数
限制及手动降级。测试未向真实应用发送输入；连续自动循环/稳定检测仍未完成。
连续自动循环与有界稳定帧检查已接入：重复/错误/上限停止，用户停止不再排下一步，
保留当前收尾及已有帧。验证仅使用合成源和隐藏 UI；实际应用响应、捕获排除和 DPI
仍缺真实验收，不能据此勾选 N05 完成。
自动采集已串行刷新进度预览，未确认固定边缘时跳过预览继续采集；停止后预览回复
不恢复滚动。合成源隐藏集成已覆盖普通和固定栏路径，真实可见效果未验证。
追加现检查最终输出余量；滚动前检查下一帧/保守输出及稳定采样峰值。预算拒绝或取消
不改变已接受帧、位移与边缘证据，可重试。短租约不是持久容量保证，合成仍可能因其它
窗口竞争失败；验证使用小预算替身，没有实际滚动或高内存压力桌面验收。
方向入口已接通单步与自动采集：垂直可向上/向下，水平可向左/向右；任务在途和自动期间
锁定方向，空闲切换保留已有内容。双轴反向隐藏 UI 回归使用合成源，真实应用响应仍待验收。
二维移动取景框所需的已定位帧合成内核已加入：64 MP 覆盖画布、早到内容优先、负坐标，
按最大完整矩形裁剪空洞；覆盖算法与 3×3 全部 512 种掩码的枚举参考比较。此为纯数据
实现，仍缺二维位置验证、取景框移动、会话/UI 接线与真实验收，尚不能替代主干该功能。
二维位置验证已有纯数据实现：校验已知位移的借用重叠区域，必要时推断剩余轴向滚动；
复用已有有界分析并拒绝歧义，零裁剪像素副本。尚未接取景框移动、会话和 UI；真实动态图像
及应用兼容性未验收，主干完整二维长截图能力仍未迁移完成。
二维数据会话已持有验证后的累计位置，失败不推进，重复不增帧，并提供最大完整矩形
输出及保留空洞的有界预览。预览不分配完整长图。尚缺一维会话转换、后台命令和取景框
UI 接线；当前仅纯数据回归，不能视为二维功能可从界面使用。
一维到二维候选转换已实现且不消费旧会话；保留固定边缘、负坐标、原始匹配帧和历史
采集计数，支持聚合轴长超过单帧限制。仍待 worker 原子切换及取景框 UI，真实桌面未验。
worker 原子切换已接：显式空间帧 Added 后转换，拒绝或错误保持一维；后续采集、预览和
完成返回对应二维结果，对话框已能消费。合成源 worker/隐藏窗口回归覆盖该链，取景框
移动命令、采集源参考坐标事务及真实应用验收仍未完成。
MoveCapture 与采集源准备/提交/回退契约已接 worker，合成源回归覆盖失败、拒绝、重复、
接受、忙碌与取消。真实桌面源尚未实现候选区域移动，界面未开放按钮，故仍非可用功能。
真实桌面源已实现候选区域与稳定采样，接受才更新物理参考；已绑定滚动源不更换窗口
身份，越界或无重叠移动拒绝。验证使用注入帧和纯目标几何，未真实采集或发送输入；
取景框 UI 仍未开放，不能视为完整用户可用链路。
四向取景框移动按钮已接源与 worker，成功才更新 UI 参考并预览，错误保持旧位置，
自动/在途和不具备能力的源禁止移动。隐藏合成源通过链路回归；可见边框、透明预览空洞
显示和实际跨应用/多 DPI 桌面验收仍未完成，不能把按钮接线等同完整功能验收。
透明预览空洞已用有界棋盘格显示副本区分，真实黑色保持不变，原覆盖数据和导出不变；
内存 DC 与隐藏 UI 验证像素及隔离。不透明预览不复制，重绘不分配；可见取景框和真实
桌面交互验收仍缺失。
可见边框代码已接会话：四条区域外窄窗口、鼠标穿透/不激活、捕获排除请求、接受移动后
定位和关闭回收。隐藏测试已验样式/几何/句柄生命周期，不代表实际鼠标穿透、捕获排除
或可见多 DPI 验收通过。
跨目标取景现已解除原窗口边界限制：接受移动后降级手动，绝不自动重绑定其它窗口；
目标身份变化仍拒绝，失败不撤销旧绑定。worker/UI 实时能力同步及禁用状态已由隐藏
源测试覆盖，真实跨窗口和多显示器采集仍未验证。

HDR 原图已通过活动租约进入保存快照，并流式编码到原生会话扩展；测试验证真实 HDR
PNG 编码保真、块边界、错误/取消和普通图片替换清理。会话 64 MiB 总量限制不变，超限
明确失败。内联扩展读取与租约恢复已接候选/文档链，逐行 PNG 验证后可再次保存原始
字节；坏 CRC、截断、尾随数据、非 HDR、外部引用拒绝，临时文件最后 owner 释放删除。
该数据链已有自动化证据，真实完整会话操作、可见导出及 HDR 查看器仍待人工验收。

Auto 模式成功采集 HDR 浮点源、因内容未超过 SDR 白电平而输出 SDR 时，现对齐主干
`src-tauri/src/screenshot/dispatch.rs`：`downgradedFromHdr=true`，后端仍为 `wgc`。
这不是捕获失败，不再次调用 GDI，不创建 HDR 原图文件。合成浮点帧回归覆盖自动
SDR、自动 HDR、强制 HDR，以及准备/提交后会话元数据的降级标记与后端保留。

会话 captureMeta 的主干已知字段已有有界读取/校验，保留历史缺失和原始未知扩展，
非法字段/明确矛盾组合拒绝准入。当前文档格式/降级元数据已写入保存快照，并通过
恢复候选接入文档；扩展保留、旧来源清除和实际状态恢复链已有测试。不访问原图路径，
仅有元数据时不恢复文件租约，HDR 原图明确显示未恢复；合法内联原图则恢复句柄。
新捕获已补齐 kind/sourceRect/backend，
长截图另记 scrollAxis，初始取景矩形与拼接输出分离，物理坐标带明确扩展；缺失坐标
不推断。原图内联保存/恢复的数据链已接入上述实现，完整可见会话操作仍需验收。

HDR 文档原图已有活动文件租约，阻止缓存清理删除仍在使用的原图；不持有缓存全局锁，
不影响其它截图暂存。真实临时文件与文档移交测试验证写入/删除拒绝、过期清理跳过、
替换释放及释放后恢复清理。会话持久化仍未接入，租约不等于永久资产保存。

HDR 读回层已通过 WARP 合成 FP16 纹理的真实 D3D11 staging/Map 验证，接通现有
SDR/HDR 转换；覆盖不同设备拒绝、布局限制、busy 超时、错误/取消与预算释放。
该测试不读取桌面；设备丢失现场及 HDR 显示器仍未验收。
一次性 WGC 显示器会话实现已接读回层，采用独立设备、FP16 两帧池和有界取帧，按顺序
关闭帧/session/pool；边界测试验证空句柄、取消、空帧 HRESULT、尺寸和超时策略，
没有启动真实采集。显示器局部区域入口已接 GPU 裁剪，WARP 测试验证右列/角点像素、
越界拒绝及选区 staging 预算；帧池仍为整屏，不宣称整条捕获链仅占选区内存。
全局坐标绑定已有单显示器实现，支持负物理坐标和边界，拒绝跨屏/越界，采集前后复核
布局并丢弃过期结果。纯几何和注入生命周期测试已通过，不代表真实多屏热插拔验收。
动态范围编排已连接 Target/显示器/WGC：支持 Auto/HDR/SDR、同帧自动 SDR 转换，以及
跨屏/不可用/HDR 失败后的单次 SDR 回调；区分正常 SDR 与降级，取消/布局变化不回退。
表驱动与合成像素测试已验证；Windows 版本探测已实现并在当前主机只读执行，生产 SDR
入口的注入测试验证尺寸/长度不符会被拒绝。生产 GDI 回调已复用原区域截图代码；内存
DC 测试验证负坐标参数传递、真实 GDI 像素和取消/错误路径，没有读取桌面。
CaptureJob 后台 owner 已接生产入口，单结果、取消后不交付、Drop join；测试验证非调用
线程执行、提取一次、迟到结果释放、panic 和退出回收，没有启动真实采集。
HDR 交付结果已包含同帧 SDR 预览与独立 PQ 原图，验证白电平转换、尺寸/长度一致性、
无复制移交及取消后双份输出释放；预览仍是 SDR，不表示画布具备 HDR 显示能力。
等待/取消对话框已实现，并通过隐藏窗口和注入任务验证：成功只返回一次、取消保持消息
处理直到后台退出、布局变化报错、销毁清理任务。生产入口在启动任务前设置捕获排除，
但可见窗口排除效果尚未验证。HDR PNG 已有流式暂存和无覆盖提交入口，真实临时文件
测试验证 16 位回读、目标冲突保留及取消清理。HDR 缓存暂存已复用配额/排他锁和文件
所有权回执，实际临时文件测试核对长度/哈希、RGB16、配额拒绝和错误/取消/放弃清理；
尚未接 UI 请求复核与缓存文件交付。
后台缓存交付类型已连接采集和编码：迟到取消会丢弃未发布文件，UI 接受回调通过后才
提交；SDR 无缓存，预览直接移交且保留诊断。合成帧测试已覆盖原像素释放、预览零复制、
owner 拒绝和完成后取消。等待窗口已有 wait_cached 入口，隐藏 HWND 测试验证返回时
尚未发布、窗口销毁不撤销已移交的准备结果，以及关闭后等待后台退出并清理暂存。
测试使用合成 HDR 像素，不读取桌面；实际菜单还未消费该交付类型。
文档交付已有 Captured 输入，新钉图/替换保留同帧预览及原图来源记录，现有 GDI 截图
也复用该输入。测试验证无文件读取、像素零复制、origin/诊断传递和文档身份隔离；
来源绑定内存文档而非持久化资产，尚未支持会话恢复。HDR 原图已有独立另存入口，
通过租约句柄流式复制，不含后续编辑；字节保真、无覆盖和取消路径已测试，可见保存
对话框与外部 HDR 查看器尚未验收。区域新钉图和
替换菜单已调用 wait_cached，选区冻结图只作参考，最终内容在确认后采集；全屏和长
截图维持原路径。注入结果测试验证实际新钉图准备及文档变化拒绝，物理矩形测试覆盖
负坐标和溢出。右键截图信息已展示原图/预览区别、降级原因、路径和显示器元数据，
降级菜单标签、无来源及身份失效禁用已通过系统菜单测试；文本有界且不访问文件。
真实桌面、HDR 显示器、可见信息框与跨屏行为仍未验收。
菜单/UI 请求、文件交付接线及真实会话验收仍未完成，不能把
注入回调测试当作真实跨屏采集通过。

HDR 像素转换已有纯数据层：借用 FP16 scRGB 行缓冲进行亮度分析和 SDR/BGRA 回退，
处理行填充、白电平、增益、异常通道和取消/预算。全部 half 位模式及合成行已验证，
显示器元数据查询层也已实现，区分系统已知配置和白电平回退，并限制枚举及重试量；
元数据查询当前仅验证纯数据组合、边界与空句柄拒绝。仍缺实际 HDR 捕获接线和降级提示，
也未在真实 HDR 显示器上验收，不能宣称 HDR 已迁移完成。
HDR 输出像素已补上 BT.2020/PQ RGB16 大端转换，携带内容/平均亮度及 mastering 范围；
直接从浮点行生成，不使用 SDR 图像中转。数值参考、行填充、高亮、异常通道及预算/取消
已验证。新增流式 PNG 编码入口可将这些像素写成带 HDR 元数据的 RGB16 PNG；解码回读
及块顺序已测试，失败/取消不报告成功。尚未接实际截图、文件发布和 HDR 查看器验收。

屏幕取色新增独立菜单入口（画笔/边框、填充），会话期低级鼠标 hook 转发并合并输入，
事件驱动 GDI 单像素采样和 HEX 预览，左键松开确认；右键/Escape/失焦/关闭取消。
来源变化拒绝应用，旧会话消息不污染新会话，结束释放 hook/窗口/模态。隐藏回归验证
颜色应用、取消、过期拒绝和消息合并，不安装真实 hook、不读真实屏幕。跨应用、HDR、
混合 DPI 及主干完整交互对照仍待完成，不能据此宣称 N05 已完成。色块/HEX 自绘已接入，
按 DPI 缩放且保留可访问文本；真实内存 DC 验证 96/144/192 DPI 的色值和 GDI 回收，
没有进行可见窗口跨屏取色验收。

拖出图片现有原生 OLE 传递层：标准 CF_HDROP 数据对象、仅 COPY、取消及结果分类。
测试直接调用真实 COM 数据对象/IDropSource，不操作系统剪贴板或磁盘。Shift 手势及
拖出命名已接入平移工具，超过系统阈值后准备快照；不抢占选择/绘图工具的 Shift，
仍未验收真实外部目标。后台快照准备
worker 已独立实现 PNG 编码/暂存/取消回收，保留裁剪与透明度，不生成额外 DIB；
测试已覆盖暂存提交和名称递增。生产手势已连接 OLE 入口；隐藏回归替换最终拖放调用，
验证松键/来源过期/模态/关闭拒绝发布、文件保护、命令防重入和失败清理。
拖动图像预览现由 worker 生成最长边 256 物理像素的直通 BGRA 缩略图，UI 通过
IDragSourceHelper 交付；与导出共用最终合成，预览失败不阻止文件复制。真实 WIC/
Shell helper 及 GDI 回收已有回归，实际可见图像、热点与跨 DPI 对照仍未完成。
真实鼠标跨窗口拖动及主干全部行为对照仍未完成。
取消准备已区分普通输入与销毁：松键等仅发信号，保留单槽到 worker 完成；不阻塞
输入、不累积线程、不抢占后续编辑捕获，窗口销毁再 join。受控 worker 回归验证这一
生命周期，不把运行耗时当作“不等待”的唯一证据。

应用配置新增纯文档读写层，支持缺省及 schemaVersion 1/2，保留未知字段的 JSON 值；
无效输入和未来版本拒绝，不重置用户设置，命名修改不更改版本号及其它功能配置。
输入和输出均限 1 MiB。已新增显式命名导入菜单，单槽后台只读所选普通文件，仅应用命名
偏好；取消/失败/过期结果或原窗口关闭保留当前值，不改写源配置。隐藏窗口与真实 worker
回归不替代可见文件框人工验收。另存为菜单已可保存命名偏好并重新导入，保留来源配置
其它字段及版本号；后台暂存后检查 owner/设置/来源身份，不覆盖已有或并发创建的目标。
显式保存当前来源已支持基于原始字节的冲突检测和恢复替换，成功推进基线；另存为成功
切换活动来源。连续更新、未知字段保留、外部冲突、过期/退出回收已有真实文件回归。
双 rename 并非原子覆盖。显式启动现可在原文件缺失时只读加载唯一恢复副本，歧义拒绝，
原文件存在时不回退；恢复来源禁止直接覆盖，须另存为。真实 EXE 回归使用合成中断现场，
不能替代断电实验；完整设置功能仍未完成。
已有默认关闭的可选自动保存，只在活动配置的命名值变化时复用单槽写入，不添加空闲
计时器。关闭丢弃未发布自动结果，错误停用并保留当前设置；应用级退出等待保存后继续
工作区确认。单窗口关闭/强制结束不受此等待保护，开关不跨启动持久化。
`--settings path` 已支持启动时显式加载命名偏好，普通窗口与工作区恢复窗口均验证已应用
并共享 owner；坏配置在窗口/截图前失败，不静默回退。真实 EXE 测试覆盖中文相对路径、
错误/缺失/超限配置与源不变，不执行有效配置下的真实桌面截图。普通启动默认使用隔离的
LOCALAPPDATA/OtterNative/app-settings.json，显式文件和 profile 目录覆盖优先。首次缺失
不创建目录，实际保存才创建且不覆盖并发首写；隐藏检查仅使用测试显式配置的隔离目录。

PNG 导出框已有主干默认建议名（本地日期/百分之一秒/实际输出尺寸），共用 WIC 输出
尺寸准入；回归覆盖格式补零、精度和裁剪不受视口缩放影响。用户仍可修改路径，已有
文件不覆盖。另有默认 PNG 保存入口，将当前显示快照按保存模板写入原生 profile 的 saved
目录，后台编码并原子递增避让同名；--settings 不改变该输出目录。合成文件验证实际解码、
名称递增、旧文件不变及冻结命名，真实菜单待人工验收。命名字段已复用隔离配置持久化；
剪贴板命名已接生产复制任务，冻结模板后编码/暂存并在文档复核后递增发布缓存文件，
再提供 PNG/DIBV5/CF_HDROP。隐藏 HWND 测试不触碰系统剪贴板；实际应用粘贴仍待验收。
应用内跨窗口复制现在由共享取消owner仲裁，新请求使旧结果失效；序号变化在文件发布前
及持系统剪贴板锁后分别检查，避免迟到任务覆盖已观察到的新内容。双窗口和注入序号
回归通过，真实外部应用竞争及延迟渲染行为未验收。
默认7天缓存清理已接入后台暂存前的排他锁范围，仅通过句柄删除记录、文件身份和摘要
一致的过期文件；未知/修改/替换/占用/硬链接均保留，隔离文件测试已验证。启动配置现可
指定保留期和软配额，默认7天/256 MiB，0分别关闭；校验与主干范围一致，窗口共享、任务
冻结，命名导入不改变它们。压力淘汰已接入，按记录创建时间排序，重新核验后尽量降至
半配额并预留新输出；零配额禁用，未知/变更/占用文件保留，释放不足仍明确失败。
缓存设置原生两字段编辑已接入，容量使用精确字节整数，校验/取消/过期保护生效；保存、
另存和可选自动保存同时快照命名及缓存，缓存单独变化也持久化，未知字段仍保留。
真实对话框键盘/高DPI验收和拖出文件名消费仍未完成。
纯命名引擎已覆盖主干占位符和默认值，默认导出使用该引擎；校验括号/未知占位符，增量
清理非法字符并限制输出120字符。模板4 KiB/字段64 KiB容量限制是新增显式准入，超限配置
需报告不支持，不能悄悄迁移。文件命名设置原生菜单提供三个模板与标题长度四字段，统一
校验后整体应用，所有窗口共享并在恢复后保留；无效/取消/过期草稿不覆盖当前值，可通过
配置保存链路持久化。title/label 已按主干取能力名称，process 取 artId；离线
恢复仍保留 artId，普通图片使用空标题/空 process，未查询 OS 进程或窗口标题。标题长度
设置与实际建议名有回归覆盖，其它设置界面及其他命名消费入口仍待接入，
真实输入框交互未人工验收。新增 CONIN$/CONOUT$/COM0/上标数字设备别名保护是安全差异。

保存文件已携带原生坐标扩展 v1；解码暴露原点/缩放，拒绝 null、非对象、未来版本和
无效数值，保留未知字段。旧格式缺省兼容。保存快照到文件的上下文一致性已有回归；
恢复端已接自动选用该扩展，完整显式 Space 优先；旧文件无上下文时可见模式要求手动
输入，隐藏模式拒绝，不猜测 DPI。输入前后验证来源，取消/期间编辑不替换原工作区。
真实 EXE 检查自动/显式两条路径的实际 HWND 矩形。
打开工作区菜单已接文件框/替换确认和正式恢复链。隐藏集成验证锁定窗口仍可打开、
合并请求、模态延迟、取消/过期/读取失败保留原工作区，以及成功恢复的隐藏窗口和保存
路径。选择出口被注入，真实可见文件框未验收；旧格式坐标输入已实现但可见操作待验，
资产根授权现有独立菜单及原生目录选择框，普通打开不授予外部权限；所选 Root 随本次
恢复或保存后打开意图持有，仍仅准入直接子文件。隐藏验证不展示目录框，实际目录选择
和外部图片会话的完整可见流程仍待验收。
外部图片已有真实隐藏 EXE 文件回归：正确显式目录成功，缺省/错目录/同前缀兄弟目录/
子目录/穿越/坏图片拒绝且不回退有效 filePath。成功路径包含绘制与设备重建；源字节不变，
退出后文件和目录可移动，合成夹具清理通过。证据仅覆盖 CLI 与底层恢复链，不替代目录框交互。
未保存替换确认现支持保存后继续打开；保存完成前不读目标，失败/取消/新操作清除继续
意图。隐藏回归实际保存后重新打开同一个合成文件，并验证过期保存保留原窗口/文件；
尚未证明真实对话框操作或不同工作区之间的完整人工流程。

保存后状态凭证已接成功提交和正常磁盘导入；新建/恢复副本保持未保存，失败不覆盖
旧凭证。凭证不持有像素/完整编辑器，按需比较模型版本、几何、配置和拓扑；隐藏回归
覆盖状态建立、透明度/草稿变化及模态/热键忽略。仍是保守版本变化检测，未完成撤销
内容相等判定、可见 dirty 提示或完整窗口关闭确认，不将该基础层等同于完整未保存状态管理。
坐标上下文变化同样纳入凭证；普通文件恢复的凭证在 Space 安装完成后捕获，隐藏 EXE
分别覆盖普通/空文件的 Unchanged 与恢复副本的 Unsaved，不沿用旧工作区凭证。
应用级退出确认现支持保存/放弃/取消，并将保存完成与退出衔接；失败/取消/状态变化保留
窗口，放弃也拒绝模态期间新增变化。隐藏决策注入已覆盖基础分支；可见对话框、实际
保存后退出的桌面交互及单窗/分组关闭保护仍待验收或补齐。
隐藏保存 UI coordinator 集成已覆盖真实合成文件提交后排队退出、保存期间再次编辑导致
失败保留，以及完成时新保存请求取消旧自动退出、两次提交均完成。仅通知出口被替换，
不显示对话框、不实际关闭共享测试工作区，不能替代可见桌面端到端验收。

“保存工作区”菜单现选择已有路径替换或无路径时另存为；从恢复副本读取的工作区
不默认覆盖恢复文件。“工作区另存为”仍只写新文件，同批另存为请求优先；Ctrl+S
继续导出 PNG。锁定分组也可保存，隐藏正式菜单分发覆盖请求合并与锁定窗口保存。
这些是接线/隐藏回归证据，尚不代表可见文件对话框和完整桌面保存交互已验收。

缺失会话路径现支持只读查找唯一恢复副本，扫描有界且歧义拒绝；候选仍经正式读取/
解析/图片与窗口准备。普通文件来源保留 saved_path，恢复来源只保留 recovery_path，
提示另存为，不默认覆盖恢复副本。隐藏 EXE 验证模拟中断留下的文件状态可读取且原路径
仍缺失、恢复副本未改写；另有实际测试子进程在替换四阶段直接退出的回归，确认跳过
Rust 析构后正式/恢复/暂存文件内容和恢复选择符合阶段，系统回收句柄。尚未证明断电
恢复、可见提示验收或遗留暂存文件自动清理，不把进程退出证据扩大为这些结论。

既有文件替换已接 Application 后台准备、提交及保存菜单：核对字节与来源修订，
持有父目录及来源句柄，旧文件移入恢复名后发布新文件；失败不覆盖恢复，竞争目标
与旧文件同时保留并返回恢复路径。成功后按原句柄退休旧文件，退休失败单独报告。
七项回归覆盖成功、取消、冲突、竞争、panic、恢复名碰撞和旧文件清理失败。它不是
原子替换，崩溃后的自动恢复与断电持久性仍未验证，不算已有文件保存功能验收完成。
隐藏 Application 进一步验证真实后台替换成功后更新基线，以及候选领取后工作区变化
导致最终替换拒绝；失败保持磁盘/内存原文且立即释放句柄。UI 结果消费保留发布成功
与旧恢复文件清理告警的区别，可见覆盖操作仍待人工验收。

保存任务支持显式目标路径，后台串行完成编码和暂存后才通知 UI；准备期间仍保持单槽
和恢复互斥。隐藏新文件提交回归消费后台暂存结果；迟到取消及销毁任务会回收临时文件。
右键菜单、Win32 另存为对话框和主循环结果消费已接线；新建工作区使用明确物理像素
空间及空 revision=0 基线，恢复工作区使用历史上下文，二者均可生成新文件保存候选。
隐藏回归验证新建空宿主省略、钉住后纳入候选及 revision=1。已有文件覆盖仍待补；
可见对话框、结果提示尚未人工验收。

新文件提交的保存基线已接通：成功后记录原样提交的共享缓冲与路径，既有 Receipt
失效，下一次编码 expected_revision 使用已提交值；已持久化临时宿主取消临时标记。
隐藏回归实际调用最终提交的取消、过期凭证与目标已存在三个失败分支，检查旧基线、
保存路径和临时宿主标记不变，且不新建目标或覆盖既有哨兵内容。
测试验证导入/编码缓冲地址不变、预算到最后一个 owner 才回收，隐藏提交回归检查
基线身份与连续修订。覆盖已有文件及完整脏状态/保存状态模型仍未接通。

Prepared 保留 UI Receipt 到新文件提交；Staged 持有同一候选字节 owner，独立分配的
相同内容或仅同修订号不能冒充本次暂存结果。隐藏 Application 在临时文件准备后改变
窗口状态验证过期检查，再恢复状态并提交合成文件核对完整字节。已有文件覆盖
仍待实现；不把文件写出等同于整个应用保存生命周期完成。

新文件磁盘阶段已有 prepare/commit_new：完整候选及修订校验后创建同目录临时文件，
写完刷新后提交；Windows MoveFileEx 不传 REPLACE_EXISTING/COPY_ALLOWED。临时目录
测试验证提交前目标不可见、提交后正式读取成功、准备后目标竞争不覆盖、取消/Drop
清理及非法候选不写入。仅证明新文件路径，已有会话覆盖与可见 UI 验收仍未完成。

保存准备现由 Application 单槽协调，Receipt 留在 UI，Data 进入 `otter-session-save`
线程；worker 完成后通知，候选只交付一次，取消和凭证失效拒绝。模型覆盖迟到取消、
panic 通知及 Drop 取消/回收，隐藏 Application 覆盖重复启动和编码期间工作区变化。
这是后台候选阶段，菜单入口、正式事件消费和磁盘原子提交仍未完成。

`Data::encode` 已统一实际内容候选，隐藏快照不再使用临时拼接的编码回调。新增裁剪
往返集成用例含非对称原图、独立半透明栅格标注及矢量线，通过两次正式恢复和真实
D2D/WIC 合成比较输出尺寸与像素，同时核对标注像素/几何/身份；翻转开关两组均覆盖。
编辑器的栅格标注本来就是可见框纹理，禁止再按完整源框裁切。此证明限于该普通裁剪
组合，非等比缩略、缩放及来源失效组合和正式磁盘/UI 保存仍待验证。

缩略标注写出从“一律拒绝”推进到独立参考框映射：由已写回的 savedRect/当前框决定
目标尺寸，以 Editor.annotation_frame 或 visible_frame 为源，裁剪源原点扣除一次。
cropOffset 留给缩略取景，不烘焙进持久标注坐标；整会话回读验证展开坐标及可见偏移。
标量线宽/字体不能无损表示的非等比变换仍拒绝，图片/栅格双层保存仍未完成。

标注写出的取消已从外层深入点集与输出流，`write_annotations` 调用
`encode_mapped_cancellable`。测试在多阶段触发取消，核对不交付部分候选、不改 Editor，
取消关闭后的结果与原非取消接口一致；输出流取消后不追加字节。单次 JSON 输入解析
和 DirectWrite 调用不是强制可中断操作，仍依赖调用结束后的检查。

`Document::encode_images` 使用捕获时已验证的恢复原图，避免窗口投影底图降低保存分辨率；
无有效原图时写当前底图，独立栅格标注不被恢复旧图层替换。测试覆盖不同源/底图/标注
尺寸及像素、无图层但保有来源、无来源回退和取消。隐藏快照同一候选已写入当前标注与
原图，4×4 原图配 2×2 编辑底图回读验证未降采样；裁剪栅格区域与缩略双层仍待完成。

PNG 资源安装新增翻转归一化：已物化底图对应的 flippedX/flippedY 清零，其余状态与
独立标注层保持。非对称 2×2 像素覆盖无翻转、水平、垂直、双轴四组真实恢复→写出→
再恢复，比较底图与栅格标注层像素，防止保留旧标志导致第二次翻转；坏状态结构拒绝。

当前 CropView 已进入保存候选的 imageEditState，不再沿用旧 cropRect/sourceSize；
用户重置后两项均删除，下一轮裁剪使用新源框。模型覆盖小数源单位、未知嵌套扩展、
非法几何拒绝及正式读取器的归一化；快照组装测试覆盖当前裁剪和重置覆盖旧状态。
翻转/边框/擦除等字段未被裁剪模块改写，完整图片编码必须另外处理其物化状态。

`Data::assemble` 现以冻结后的实际工作区生成拓扑，按当前节点身份合并几何和 Art
公共参数，再将文档交给内容编码回调。模型测试覆盖新建/删除、分组/连线、未知字段、
重复身份拒绝与取消；临时空宿主仅在未编辑且无内容/关系时省略，不能永久排除原宿主。
隐藏 Application 后台候选已回读窗口数量及当前标注数量：普通窗口标注现使用实际
Editor 与视口/DPI/Space 映射写入，不再保留旧 annotationState。字体基线复用恢复端
DirectWrite，模型测试以真实字体测量回读缩放文字位置、字号和身份；取消及缩略上下文
拒绝有单独断言。图片/裁剪等内容尚未在这个回调中完整整合，不能替代完整保存验收。

保存几何已消费显式 `minified_transition::Transition`，不再通过初始/展开框相等猜测
是否存在 savedRect。恢复时保留字段存在性，主动缩略标记真实展开框；实际展开与保存
共用 target 位移/溢出检查。模型覆盖移动后几何写出回读、缺省框不复活和同框主动缩略，
隐藏恢复测试检查有/无 savedRect 的状态标记，保存快照回读核对实际物理窗口框。

保存/恢复凭证现同时检查画布 Viewport 和 HWND DPI；保存 Data 捕获这两项，Receipt
还核对原始 Space。仅平移视口但不改标注、替换来源坐标上下文或非法缩放均不能复用旧
快照。隐藏 HWND 回归覆盖视口单独变化、NaN 拒绝、实际 DPI 读取和 Space 变化后失效；
真实跨显示器 DPI 切换尚未手测。普通标注写出已有 `encode_mapped`，使用
`(point * viewport.scale + viewport.origin) * dpi / 96 / Space.scale` 保持物理显示位置；
局部坐标不叠加屏幕原点。几何/线宽/字体/效果参数按同一比例转换，文字基线重新测量，
固定 4px 文字锚点不缩放。测试覆盖多种 DPI/Space 组合、形状/文字/序号回读、效果参数、
未知扩展不变及不可表示半径/线宽拒绝。缩略双层映射与完整保存调用仍待接通。

已绑定 Art 的保存模型新增 `session_art_write::merge`：参数仅取当前公共 owner，旧
params 不参与合并，避免把已过滤 secret/内部控制键带回；未设置、null、禁用值保持
区别，未知公共参数沿 owner 保留。执行开关写为显式布尔，同能力保留未知配置扩展，
更换能力不继承旧能力执行扩展。模型回读与真实隐藏保存快照检查已覆盖参数/开关及
原文保密参数不复活；未绑定离线节点仍需整会话编码 owner 原样保留，不伪造定义。

保存图片资源已有独立 WIC PNG 入口：原始预乘 BGRA 像素直接交 WIC，不创建 D2D
合成设备、不把矢量/裁剪/窗口透明度烘焙进底图。快照的底图与独立栅格标注层分别
编码，保留各自尺寸；临时 WIC 像素副本计入预算，失败或迟到取消不交付结果。
真实 PNG 回读检查透明度、像素顺序和图层分离；隐藏工作区快照还在独立 worker 中
编码合成测试图片。WIC 调用本身不能强制打断；整会话调用链仍待接入，不代表已将
图片持久化到用户文件。

PNG 资源现可消费式转换成带预算 owner 的 Data URL：前缀和 Base64 同次分配，不先
生成一份字符串再拼接复制；PNG 释放后文本预算持续到 owner 释放。单资源上限计入
前缀并不得超过会话总上限，完整候选仍须做聚合预算检查。隐藏快照 worker 已执行
底图→PNG→Data URL→正式会话图片读取器的像素回读；取消/坏 PNG 标识与前缀预算
回收有模型测试。现已通过 `InlineRasters::install` 将 URL 移动安装进带租约的窗口候选，
`assemble_owned` 持有所有新增资源租约直到 JSON 树释放；清理过期 previewSrc 和已移除
栅格标注引用，保留未知字段。模型测试覆盖有/无独立标注层的整会话及像素回读，隐藏
Application worker 也通过候选读取图片。此阶段只接入资源，不代表完整活动文档的
坐标/裁剪已封装完成，也未写入磁盘。

整会话内存组装新增 `session_write::assemble`：先校验来源 schema/拓扑，再按当前节点
顺序消费旧窗口对象；分组与连线按 ID 保留扩展字段，删除项不复活，新对象不继承旧项。
根级未知字段、回收站等继续保留。候选携带 expected_revision 与 checked_add 后的新
revision，限制 64 MiB，分步检查取消，最终用正式会话读取器回读并核对完整拓扑。
模型回归覆盖增删、扩展字段、未来 schema/修订溢出、坏窗口和取消不交付部分结果。
最终候选缓冲已计入共享内存预算：先计数再预留实际容量，写出不扩容，字节与租约
一起转移给共享 owner。测试验证预算不足、迟到取消回收、缓冲地址不变和最后一个
克隆释放才归还预算；中间 JSON 对象树仍不在这项字节预算证明范围内。
窗口内容由编码回调提供；活动快照的完整图片/参数编码尚未串入该入口，磁盘提交者仍须
另行校验来源文件及修订号，内存候选成功不代表保存成功。

保存快照已接到真实 Application：使用恢复时保留的显式 Space，冻结实际 HWND 框、
已提交 Editor、缩略取景/展开状态、分组/连线及已绑定 Art 的公共参数/执行配置。
原文改为 Arc 持有，快照共享原文字节和像素，矢量副本隔离；后台数据通过 Send 编译检查，
不携带 HWND 或 Rc 目录。UI 凭证复用工作区/窗口校验，模态或后台工作期间拒绝捕获，
后续编辑或呈现变化使凭证失效。没有显式来源坐标的低层发布工作区暂不允许保存快照。
此处尚无后台编码/保存命令；完整字段封装、源图/预览处理及磁盘提交仍待接入。

窗口几何内存写回已新增 `Space::logical` 和 `session_window_write::merge`。反变换复用
显式恢复 Space，保留未改动的原始小数坐标，并以物理框往返验证移动/缩放后的结果；
无效提示不能绕过尺寸预算。普通/缩略框、展开框及透明度可合并到原窗口对象，图片、
参数和未知字段由所有权移动保留，不复制图像字符串。无 savedRect 的历史缩略节点不
伪造展开框，仍保留独立 cropOffset。模型验证多种比例、负坐标、拖动后缩略展开往返和
图像存储不复制。尚未从活动 HWND 汇集完整保存快照，也未接整会话或磁盘提交。

标注可见框写出已有 `encode_visible`：返回标注 JSON、逻辑可见框和当前 CropView，
将源框内标注减去当前可见框原点，逆转恢复时的平移。保留框外标注，不重新缩放，
不平移未知扩展字段里的同名坐标。回归覆盖带小数偏移的裁剪恢复→写出→恢复、重置
裁剪及撤销后的原点切换、稳定 ID 和编辑器不变。仍需整会话 owner 将窗口物理尺寸、
源框/裁剪字段与图片引用一起封装；这个接口不独立构成可覆盖旧文件的保存事务。

标注内存写出新增 `session_vector_write::encode`：按稳定 ID 合并原记录扩展字段，删除
对象不复活、复制对象不继承源 ID 的扩展；写出当前顺序和序号计数，输出限制 8 MiB。
形状/路径、文字/序号、模糊/马赛克笔迹与渐变笔样本已有写出入口；普通路径旋转烘焙
到点坐标，文字基线由平台提供，不猜字体。主干不能表达的普通粗体文字、小数马赛克
强度和小数序号半径拒绝写出，不静默取整。该层仅输出 Editor 坐标；整会话裁剪/源框
映射、图片引用、后台保存及原子磁盘提交尚未接入，不能把它直接用于覆盖旧会话。

编辑器标注现持有稳定身份：新绘制、文字、序号与用户复制生成独立 GUID；移动、样式
修改、删除/撤销/重做及快照随对象保留 ID。会话导入按实际 zIndex 顺序将原 ID 安装到
Stroke，建窗前清单核对要求每个 ID 与对象一致，而不只比较数量。原始 annotation_ids
仅是初始清单，活动编辑后的写回必须读 Stroke::annotation_id，不能按旧数组下标套用。
模型回归覆盖导入排序、删除后索引变化、复制新身份、历史恢复、快照和非法身份拒绝。
这是会话写回的前置实现，不代表整会话序列化、未知字段合并、原子保存或退出恢复已经完成。

离线/缺失能力 Art 的运行类型现与可执行能力绑定分离。恢复 State 保留 Art 类型，
显示解析、参数图、图片输入依赖图与通知传播不再把未绑定 Art 当成普通钉图；
钉图专用中继菜单和内容编辑/翻转/栅格化入口沿用 Art 限制。执行入口仍要求真实
Selection，不因类型标记而允许执行。模型/隐藏恢复回归确认未知 Art 类型保留、
无绑定/无待执行请求，并拒绝钉图专用内容编辑。

离线恢复后的迟到目录绑定已接入常规能力目录处理入口：只尝试仍未绑定的恢复 Art，
按真实能力定义重新解析原始参数并过滤保密字段，不自动执行，也不改图片或留存原文。
模态、编辑或执行忙碌时延后；同一目录身份每个候选只尝试一次，缺失能力可在新目录
到来时重试。已绑定或关闭的节点退出候选，不用旧会话覆盖已有选择。隐藏宿主回归覆盖
缺失目录能力、目录更新重试、文件对话框期间延后、保密字段过滤及已有选择保持；
使用合成目录，不代表真实 Loom 重连或可见桌面已验收。解析失败当前记录固定错误日志，
用户可见的绑定失败状态尚待完善。

`--session-check` 现对每个恢复宿主发送真实 WM_PAINT，然后释放绘图设备资源再绘制一次，
检查窗口仍存活、无绘制错误且绘制计数增加两次，成功才输出渲染检查回执。
真实 EXE 回归新增合成 PNG、文字、填充矩形、模糊路径的普通/缩略两种会话，在显式
1.5 坐标比例下完成读取、恢复、D2D 绘制及资源重建；不显示窗口、不读取真实截图。
这验证了运行渲染链可执行，不代表逐像素主干对照或可见桌面视觉已验收。

命令行会话入口已接通真实 EXE：`--session <文件> --session-origin X,Y --session-scale N`。
原点与比例必须显式给出，不推测历史覆盖窗口 DPR；可用 `--session-assets <目录>` 显式
授权引用图片的受管根目录。相对命令行路径在启动时转为绝对路径，保留 Unicode/空格。
追加 `--session-check` 时以真实消息循环完成后台读取、离线绑定、隐藏窗口发布后退出，
不进入全局热键注册/显示首窗口路径，输出 `native-session-check` 回执。真实子进程回归
覆盖普通会话、空会话、坏 JSON 和非法比例；可见运行与菜单入口尚未完成手动验收。
示例（以下原点/比例仅为示范，必须换成来源会话的真实坐标上下文）：

```powershell
.\otter-native.exe --session 'C:\test\session.json' --session-origin '-1920,-200' --session-scale 1.5 --session-check
```

移除 `--session-check` 即进入可见应用；后台恢复失败会保留初始工作窗口并报告错误，
启动参数或资源目录准入错误则在进入应用循环前返回失败。
可见启动必须先完成首窗口显示和全局热键注册，再发起恢复并捕获来源凭证；否则正常
启动动作本身会让恢复结果失效。此顺序已修正，隐藏检查仍绕过显示/热键路径。
隐藏子进程测试不证明可见启动和真实全局热键转交已验收，仍需单独手测。

后台会话输入现支持显式文件路径，与内存字节输入复用同一个单任务 owner。文件只在
worker 打开/读取：要求普通绝对路径、磁盘普通文件且末级非重解析点；读取前检查
64 MiB 上限，分块检查取消，拒绝短读/增长，不交付部分字节。读句柄禁止并发写入/删除，
任务结束释放句柄；文件输入不自动授予 JSON 所引用图片的目录权限。临时文件测试覆盖
跨块读取、UTF-8 原文、超限、取消、设备/目录拒绝及句柄回收。应用请求接口可传该输入，
文件选择菜单仍待接入，启动参数见上；原生文件系统调用本身仍不能强制中断。

应用级恢复协调已串接真实后台 Job → 目录绑定 → State/隐藏窗口准备 → 工作区发布。
`request_restore` 在启动 worker 前捕获来源凭证，避免只在解码完成后捕获而覆盖加载期间
的编辑；同时固定能力目录身份和显式坐标上下文。应用只接收一个恢复任务，完成通知
复用现有唤醒路径，无新增轮询定时器。取消仅发信号，完成后回收且不弹失败提示；
解析错误/来源或目录变化不发布，原窗口保留。隐藏集成回归覆盖并发拒绝、加载期间
编辑、取消、坏 JSON 和成功发布。目录绑定现明确支持离线 None 快照：无目录时保留
Art 身份和原始参数，不生成可执行 Selection，不伪造 Loaded 或连接身份。None/Some
之间的变化与目录 Rc 身份变化一样使准备结果失效，不能把后来出现的目录直接套入旧批次。
回归覆盖离线混合节点发布、原文保留、无活动执行及加载期间目录出现时拒绝。
命令行文件输入及目录恢复后的未绑定 Art 重试已接入；会话文件菜单入口仍待接入。
测试使用独立合成目录，不连接真实 Loom。

已接入 `Application::publish_restored_batch` 实际替换方法：重新核对双侧凭证，预先准备
窗口优先级监听及截图窗口登记，转交已有全局截图热键后，同步交换窗口 owner、工作区
图、分组目录和原文留存，并清除旧显示缓存/已结束请求记录；成功后释放旧窗口。
热键转交失败尝试恢复原注册者，原工作区不交换；补偿也失败时明确报告两项错误。
原始会话字节以所有权移动保留，标注身份随行，不复制图片或重新解析/执行 Art。
独立隐藏 Application 回归已验证过期批次拒绝且旧 HWND 保留、成功替换后的身份/登记、
原文字节及缓冲地址不变、首轮常规更新、退出清理；热键补偿使用注入调用，不抢真实
全局快捷键。可见窗口展示和实际热键转交仍需手测。空会话现创建非钉图临时宿主，
仅加入运行工作区和截图登记，原始空会话/标注清单不添加伪钉图；原文仍保留未知字段。
连续两次空会话发布回归验证旧窗口清理、新建窗口能力、常规更新与退出清理。
临时宿主身份单独留存，未来写回必须检查其当前内容，不能据初始身份丢弃后续用户编辑。
菜单入口、历史坐标上下文自动解析与持久化写回尚未接通，不能视为完整恢复功能。

恢复空闲准入现直接检查应用级执行请求表，不能仅以 worker 为空推断可替换工作区。
任一活动请求（包括未出现在当前工作区节点表中的请求）都使已准备批次失效，并阻止
新的窗口准备；检查不消费请求或预览。纯模型测试确认空闲检查保留请求与历史代际，
真实隐藏 HWND smoke 确认活动请求在物理框转换前阻止准备，正确结束请求后重新准入。

建窗前新增批次清单校验：工作区节点、State、Placement、图片、标注身份列表必须
数量一致且一一对应；还核对标注数量/身份合法性、执行结果归属、钉图及分组状态。
准备好的隐藏批次不再公开可写元数据。真实 HWND smoke 注入八类缺项/重复/归属错误，
确认均在物理框转换与建窗前拒绝，线程窗口数量不变。此校验不代替原始 JSON 的
字段解析；其消费者为上述发布方法，菜单和持久化入口仍待接入。

隐藏批次现同时保存原工作区与候选窗口的单窗口凭证。窗口身份/位置、编辑器和外部显示
身份、配置/输出之外，还检查正常/缩略/当前呈现透明度、缩略取景及展开目标、钉图状态、
分组锁定/隐藏状态；候选必须仍隐藏且空闲。候选草稿、待处理请求或属性变化会使批次
失去发布准备资格，不消费请求、不取消任务。真实隐藏 HWND 回归分别修改两侧属性，
检查恢复原值后重新匹配；候选双击展开后旧凭证明确失效。此处是发布前完整性检查的
推进，现由上述工作区替换方法在发布前消费；不能代替待完成的持久化流程。

原生显式 PNG 导出与复制现携带独立的正常底图透明度，对齐主干
`stickerCompositeRenderer` 的底图 globalAlpha；栅格/矢量标注保持自身样式，
不能用整个合成结果乘透明度替代。会话准备保存 opacityNormal，缩略呈现的
opacityMini 不覆盖该值；保存对话框和后台复制的来源凭证也核对正常透明度。
真实 WIC/PNG 回归验证 0/0.5/1、两类标注不被衰减、源像素共享及文档不变，
非法值在像素预算准入前拒绝。此修订限显式导出/复制，不表示 Loom/投影等
所有输出合同已经统一，也不代表正常透明度的属性编辑与撤销界面已完成。

会话颜色恢复在 #RGB/#RRGGBB 之外新增不透明 #RGBA/#RRGGBBAA 与逗号整数形式
rgb()/rgba(...,1)，对应主干 ColorPicker 可输出的部分文本形式。颜色解析独立有界，
保持标注自身 opacity，不把颜色 alpha 乘错或静默抹去；非不透明颜色、百分比/空间分隔
语法和上下文颜色仍明确拒绝整批标注恢复。回归覆盖颜色/填充、原始 JSON 保留、无导入
历史以及不支持透明色时不交付部分结果。rgba 的 alpha 仅接受 1 或 1 后带全零小数，
按文本精确判断，避免接近 1 的透明值被浮点舍入后误接收；指数写法暂不支持。
这不是完整 CSS Color 支持。

未裁剪恢复文档现也保留精确逻辑尺寸，不再用 ceil 后的像素缓冲范围代替会话画框。
候选装配安装独立 logical size，不生成裁剪或导入历史；尺寸必须有限、为正且与已准备
缓冲的 ceil 尺寸一致。真实会话候选到 WIC/D2D 合成回归覆盖 7.25×3.25 逻辑框、
8×4 缓冲和 7×3 输出，验证快照以及裁剪撤销仍保留原始小数框；活动文档拒绝恢复覆盖。

目录绑定 Parts 已可整批转换为未发布的 State：恢复节点 ID、编辑器、Art 空闲结果、
同目录参数/执行配置及分组锁定/隐藏状态，标注 ID 与原始会话由 Prepared owner 保留。
准备不创建 HWND、不注册热键、不执行能力、不扫描整图；取消、数量/身份缺失和
多余 Art 结果拒绝整批交付。缺失能力不创建可执行选择，其身份仍保留于候选原文。
透明命中 alpha 区间现随 VectorCandidate 在 worker 准备，逐行检查取消；底图与独立
标注分别扫描并共享像素，不创建跨线程 GDI 句柄。缓存经目录绑定进入 State，UI
装配不重复扫描；缺失缓存拒绝整批状态交付。实际窗口区域仍在 HWND 所在线程生成。
DocumentWindow 已增加显式物理框的隐藏创建入口，允许无图宿主；限制尺寸/面积和
坐标溢出，不自动显示或安装优先级监听。启动 smoke 增加真实无图 HWND、负坐标、
尺寸、隐藏工具面板、图标身份和析构释放验证；该入口要求调用方先完成 DPI 转换。
Application 的隐藏恢复窗口入口已复用普通窗口的共享状态绑定：目录/分组目录、
热键启用状态、退出请求、截图状态和窗口集合、模态登记均沿用应用 owner。
入口拒绝模态阻塞/退出状态，不向活动文档与工作区注册候选。真实 HWND smoke 检查
共享身份、隐藏和未发布状态，以及失败/析构后不残留模态阻塞、不修改活动集合。
整批状态现已串接隐藏窗口准备 owner：创建前后核对目录 Rc 身份、取消/模态/退出状态，
物理位置由显式转换回调提供；任一中途失败销毁本批已创建窗口，原活动工作区不变。
合成会话的真实 HWND smoke 覆盖两个隐藏宿主、第二个位置转换失败与第二个阶段取消，
按当前线程窗口数量检查清理，并验证原文/目录/标注身份仍由成功批次持有。
隐藏批次现捕获原工作区来源凭证：拓扑、分组/成员、窗口身份与物理框、编辑器版本、
节点配置及正式输出身份。每个创建边界和返回前均核对，活动笔画草稿也会拒绝；
成功 owner 保留凭证供后续发布前复核。真实 HWND smoke 覆盖配置变化及活动草稿失效。
已补入空闲检查：输入准备/执行、参数文件、显示合成、目录加载及各窗口的图片处理、
导出/复制、待处理窗口/链接/分组请求、接管和活动变换手势均阻止恢复准备，不自动取消。
来源凭证也保留外部显示 Frame 身份；回归覆盖待处理请求不被消费、执行中拒绝、
预览手势拒绝及显示快照替换失效。最终发布仍须复核候选窗口属性与持久化边界，
不能把当前空闲检查单独当作完整发布授权。
已核对主干 `syncService/backendRects.ts` 与 `overlay_state_commands.rs`：会话框为
覆盖窗口 CSS 坐标，乘该窗口 DPR 按 Math.round 取整后，再加客户区物理原点。
`session_placement::Space` 已实现该转换（含负半整数向正无穷取整）、溢出与面积限制，
隐藏批次 smoke 已验证合成原点/DPR 经转换后抵达实际 HWND。兼容覆盖窗口原点/DPR
仍需由真实启动入口明确解析；会话自身没有这些上下文，不按目标显示器猜测历史比例。
恢复 State 已按当前正常/缩略状态选取保存的不透明度，Canvas 在普通及外部显示绘制前
将其交给 DirectComposition。效果组按需创建，值未变不重复提交；不修改像素、历史或
导出透明度。真实合成链回归覆盖 0/0.35/1、非法值拒绝、效果对象复用和 resize 后保留，
批次 smoke 覆盖缩略透明度选择；桌面最终视觉与缩略切换仍待验收。
缩略视图已对照主干 `computeMinifiedStickerViewport` 与
`computeMinifiedStickerAnnotationViewport` 分开解析到 Placement：图片按 contain-fit
扣除留白并夹紧取景偏移，标注沿用 savedRect 与原 cropOffset；已有裁剪的图片使用
源逻辑框和裁剪偏移，无 savedRect 时保留两层不同的主干回退尺寸。
测试覆盖两层偏移不同、负偏移和已裁剪源框。恢复 State 现将独立取景呈现装入 Canvas，
普通本地显示在 WM_PAINT 分别绘制原图、栅格标注和已提交矢量，不把投影留白再次缩放。
呈现共享原图像素并核对文档身份/版本，活动草稿或过期快照拒绝绘制；渲染结束恢复变换。
实际 WIC/D2D 像素测试覆盖不同图层偏移、96/144/192 DPI、文档不变和旧快照拒绝。
标注映射现显式区分文档逻辑框与显示框，按两轴比例缩放后扣除裁剪源和缩略偏移；
绘制与 GDI 命中消费同一纯几何转换。新增像素/区域断言覆盖文档和显示宽度不同的矢量，
不会仅移动标注而遗漏缩放。该路径仍不代表普通窗口的重新缩略入口已经接通。
透明命中现接入同一呈现的双层几何：原图与栅格标注分别变换，矢量按标注偏移和裁剪
源偏移映射，最终与客户区相交。后台恢复缓存使用原图 alpha，不在正常绘制时重复扫描。
真实 GDI 区域测试覆盖两层不同位置、透明空洞、越界裁剪、矢量偏移和非等比物理缩放；
实际桌面跨进程点击仍待验收。恢复缩略窗口现支持平移工具双击展开，
正常框使用恢复批次同一坐标转换，并叠加缩略窗口后续拖动位移，不按当前 DPI 猜历史比例。
展开恢复正常不透明度和命中区域，不修改编辑器；组锁定/隐藏、外部显示或过期呈现拒绝。
真实隐藏 HWND smoke 通过非客户区双击消息检查锁定拒绝、拖动后展开、尺寸/透明度和文档不变。
缺少 savedRect 的历史缩略节点同样安装展开事务，对齐主干 useUnitActions：退出缩略但
保留当前框，不用图片尺寸或 cropOffset 猜测展开位置/大小。专用隐藏 HWND 回归覆盖
负屏幕原点、1.5 比例、非零 cropOffset、拖动后展开、正常透明度恢复和文档凭证不变。
普通钉图已接入平移工具双击缩略：以点击位置计算 100 个逻辑单位的取景框，小图允许
负偏移，按当前窗口 DPI 转换为物理框；标注不烘焙，原图和编辑缓冲按共享身份引用。
恢复原图来源现同时绑定文档身份、底图像素身份/尺寸和完整逻辑框，不因单纯标注编辑
或裁剪视图变化而失效；替换/擦除底图后停止采用旧来源，撤销回到相同底图时重新验证。
每个恢复文档最多保留一份共享原图关联供撤销，不随编辑累积；更换文档/图片时清除，
窗口销毁时释放。普通画布同步也检查文档身份，不等下次缩略才清理跨文档引用；正常与缩略透明度分别保留。
真实隐藏 HWND smoke 连续完成两轮“双击缩略、移动、双击展开”，检查屏幕位移、
原尺寸/透明度、文档版本和像素身份不变，且不显示窗口。几何回归覆盖小图负偏移、
150% 缩放和溢出拒绝。来源回归覆盖矢量编辑、独立标注擦除、裁剪/重置、底图擦除、
撤销和文档替换。外部显示取景、完整编辑准入及裁剪/效果视觉回归仍待完善。
缩略/展开的切换准入现已统一：图片编辑 worker、显示接管、执行准备/执行、截图、
文件选择和未结算手势/效果预览期间拒绝切换，不取消任务或消费待处理状态。
模型回归覆盖标志、草稿及活动执行不被修改；隐藏 HWND 双击链覆盖文件选择状态阻止展开。
已核对主干 StickerAnnotationLayer 的 interactionEnabled：缩略时不启用标注交互。
原生现拦截缩略窗口的标注指针与编辑快捷键，共用编辑准备入口拒绝内容/裁剪接管，
基础编辑/工具菜单命令拒绝执行；复制、导出、Esc、生命周期消息继续路由。
缩略窗口保持系统拖动命中，即使工具状态改变也能双击展开。隐藏 HWND 回归覆盖
画笔按下/松开、Delete、三类编辑准备拒绝、拖动命中与非平移工具下展开。
文件替换、动态能力菜单和外部显示等其他入口仍需逐项核对，不把此边界测试当作全部编辑准入完成。
文件/剪贴板导入现区分新钉图与替换当前文档：缩略时 Ctrl+O、Ctrl+V 和新建菜单仍可用，
显式替换在选择/读取前拒绝，菜单同步禁用，后台启动及提交凭证再次复核；不以拦截
所有导入来代替文档边界。隐藏 HWND 回归使用合成图片验证替换不启动 worker、新建
经真实 worker 完成并交付独立候选，原缩略状态、文档版本和像素身份不变。
已有缩略宿主的外部 Frame 现走分层取景，而非普通整图 fit：无编辑快照时按来源
宽高比计算图片取景，有编辑快照时按其逻辑可见框映射底图及标注，不接管本地文档。
透明命中采用同一外部呈现，并持有 Frame 身份区分同像素/同 revision 的不同快照。
WIC/D2D 像素回归检查外部绿色图替换显示而本地文档不变；隐藏 HWND smoke 实际执行
外部缩略绘制、命中更新和拖动后展开，外部 Frame 不被切换消费。普通外部显示宿主
现也可直接双击缩略，仍遵守后台任务与手势准入；宿主只保存自己的展开框和本地文档，
外部取景在绘制/命中时从 Frame 准备。隐藏 HWND smoke 分别验证有本地底图与无本地
底图两类宿主的两轮切换，外部 Frame 身份保留，本地像素/文档不变，无底图宿主不会
因切换生成本地栅格。不同外部裁剪/效果组合及真实服务更新视觉仍待验收。
新增真实 HWND 区域回归：连续安装共用透明像素、同 editor revision、但笔画位置不同
的外部快照，通过 GetWindowRgn/PtInRegion 验证旧笔画位置从可命中变为不可命中。
断开外部快照后验证本地/无图宿主区域恢复且本地文档身份不变；覆盖有/无本地底图两类窗口。
合成测试不连接真实服务，也不把区域查询当作跨进程人工点击或屏幕视觉验收。
面板可用性现纳入缩略状态：工具、数值样式、模糊设置和替换式截图禁用，打开新钉图、
新建截图及只读输出保留原 busy/锁定规则。数值/模糊弹窗沿用执行时可用性复核，
字体和序号弹窗入口另行拒绝缩略编辑。仅有外部 Frame 的宿主也被识别为有可输出图片，
不需先接管为本地底图才能启用复制/导出。回归覆盖命令矩阵、忙碌状态及外部输出准入不改文档。
外部裁剪/效果已有组合像素回归：完整 8×4 源图、偏移裁剪框、独立 2×2 栅格标注、
预计算模糊纹理经过缩略取景，在 96/144/192 DPI 下分别检查标注背景和效果中心像素。
效果使用可区分的颜色标记验证坐标/遮罩，不冒充卷积算法或真实截图效果质量测试；
绘制后源编辑器凭证不变，本地无底图文档仍无底图。其他效果组合与桌面视觉仍待验收。
恢复批次仍未发布，不能把此绘制路径当作缩略功能整体完成。
窗口事务仍需接入真实坐标上下文、恢复位置、
缩略与透明度并原子发布；当前入口不能直接等同于已恢复可见窗口。

裁剪恢复的后续准备已有 `Restored::editor_crop`：按当前显示比例还原完整逻辑源框，
保留非整数、非等比缩放与负屏幕坐标，不用像素取整改写矢量坐标。
`Images::prepare_cropped_editor_layers` 保留完整底图，并将独立栅格标注投影回裁剪位置，
不使用预览替代源图。合成测试覆盖框外底图保留、标注透明边界、源像素不变和取消。
后续已在 Candidate.prepare_vectors 接通完整源框的模糊纹理与矢量平移：
模糊资源惰性生成，按精确强度共享；独立标注只投影一次，不以裁剪预览替代完整底图。
回归检查实际候选的路径偏移、效果图尺寸、无导入历史、取消与原始会话字节保留。
整批 VectorCandidate 已接入裁剪栅格、矢量、模糊与初始裁剪视图；缺少完整底图、
源尺寸不匹配或非新文档拒绝安装，不通过裁剪/撤销操作伪造初始化。
候选回归验证整数源框的重置/撤销/重做不移动标注、不复制底图。
Editor 已独立保留完整逻辑尺寸，重置裁剪不再回退到 ceil 像素尺寸；快照和文档接管
连同撤销/重做一起保留或恢复该尺寸，再次裁剪沿用完整逻辑框。像素缓冲仍共享且保持
整数尺寸。统一 annotation_frame 已接入普通/外部显示的裁剪与标注映射、透明命中、
擦除请求及缓存校验、模糊请求、缩放与栅格刷新计划、栅格化和 PNG 导出。
重置裁剪的效果计划捕获完整逻辑框，不把 None 裁剪误当作整数像素画框。
新增实际 WIC 导出和效果卷积回归验证小数框重置及撤销/重做。
钉图 fit、裁剪窗口同步和编辑输入边界随后已使用该有效画框；合成回归覆盖
96/144/192 DPI 的 fit 数学、0.5/1/1.5/2 倍视口下拒绝像素留边及夹紧终点。
重置菜单仍按真实裁剪状态启用，逻辑框元数据本身不冒充裁剪。
后台显示合成的共享底图快速路径现核对有效画框：小数完整框不能绕过合成，
与像素尺寸完全相同的完整框仍共享底图。外部显示文字准入与 Loom 输出字节预算
已使用同一逻辑框；测试覆盖留边不接管文档、实际合成与 PNG 一致、重置/撤销预算。
跨 DPI 真实窗口视觉结果、其他显示来源边界及窗口恢复事务仍待验收/完成，
不代表裁剪会话验收完成。

会话恢复增量：已有与主干 SessionSticker/SessionLink/SessionGroup 字段一致的拓扑
适配和独立候选构建；合成测试覆盖中文旧 ID、分组归属、锁定/隐藏、历史环、字段
名称及失败不影响现有工作区。未接磁盘、窗口/图片/编辑状态与启动入口，不代表 N06 完成。
只读 SessionData 解析入口已有版本/revision、数组限长、坏字段及悬空连线检查；原始
字节借用保持不变，未知业务字段不丢失。64 MiB 原生读取预算的大会话兼容仍待验证。
位置/尺寸、缩略、savedRect、透明度及节点类型已进入只读候选，缺省值对照主干，
缩略 cropOffset 也已进入候选；展开框按当前位置减偏移计算，移动缩略图后不会跳回
旧 savedRect 位置。负屏幕坐标、缺失偏移回退与展开坐标溢出已测试，后台 Job 保留该元数据。
非法窗口元数据拒绝整批候选且原文不变；实际窗口恢复及显示器/DPI 适配仍未接通。
资源引用已解析并按主干处理预览去重/Art 预览锁定；大 Data URL 借用原始 JSON，
转义字符串按需解码。引用层不读取磁盘或网络资源，内联解码另由后台入口负责。
后续增量已完成内联 Data URL 的真实后台 WIC 解码及像素释放回归，复用共享预算且不
复制 Base64 JSON 包装。文件路径、网络资源、整批调度及窗口呈现仍未接入。
已打开文件句柄也可后台有界读取并解码，覆盖读取期长度变化、取消、真实文件和像素
生命周期；会话路径的授权/打开仍由后续恢复 owner 负责，不隐式接受任意路径。
整批图片准备已有共享缓存和失败/取消全批丢弃；输出持有独立像素身份，实际 worker
验证相同引用不重复解码、候选可脱离原文存活。应用调度与窗口事务仍未接入。
显式受管目录准入已有真实句柄校验及整批文件解码回归，重复路径只打开一次；仅支持
直接子文件，外部资源授权和应用恢复入口尚未完成，不把路径文本视作读取授权。
恢复候选已拥有原文、窗口元数据、拓扑和图像，并有可取消后台 Job 与完成通知；
迟到取消不交付候选。尚未接应用准入/提交，不将未迁移的参数/编辑历史当作已恢复。
params 已借用原文，按能力过滤 secret/控制字段并恢复公共值；未知公共字段在统一
预算内保留并参与执行编码，不成为控件或端口，能力版本兼容交互仍待接入。执行默认值回退与显式 null 已对照
主干修正，并通过实际生产请求编码回归；窗口安装和完整能力恢复尚未完成。
会话 executionConfig 现可借用读取，并与公共参数按同一能力 ID 构造 Art 候选；
显式执行/传播开关、展开状态及高成本 MCP 图片搜索默认值复用生产配置解析器。
禁用能力可恢复数据但不因此获准执行；错配能力拒绝。经核对主干仅按精确 ID 查找，
不存在别名规则。整批候选现已接入目录解析：已匹配节点恢复参数及执行配置，缺失能力
返回未解析身份且原文保留；后续已匹配节点失败或取消不会交付部分结果。
桌面 Candidate 已提供此解析入口；Bound owner 已将原文/图像和结果绑定到共享目录，
目录替换或取消拒绝交付并保留候选，重新绑定会重新执行 secret 过滤。
测试覆盖新目录将公共参数改成 secret 的情况；窗口安装和整批提交仍未完成。
Bound 已同时准备 Art 的初始 Results：优先显示保存的预览，否则使用保存的源图，
保持 Idle、无活动请求、无正式输出端口，不冒充已完成执行。共享像素，不额外复制整图。
恢复后的启动/失败保留显示，真实标量结果提交不会将旧预览变成图像输出端口；
回归覆盖身份错误和取消拒绝。Results 已保存恢复预览保持标记，参数/上游实际变化、
成功排队的手动运行和真实执行开始会解除保持，但不清空显示或生成正式端口。
隐藏窗口回归验证自动执行关闭仍会解除保持、锁定分组和无效 HWND 不误解除。
shader/surface 渲染器对保持标记的消费、编辑层合成和实际窗口安装仍需继续接入。
恢复图像现可建立原生栅格 Editor：source 和 rasterizedAnnotationLayerSrc 保持独立
共享像素与各自尺寸，不使用 preview 作为底图。测试已覆盖不同尺寸标注层的仅标注
擦除、底图共享身份不变、撤销/重做及初始恢复内容不可撤销丢失。
该入口尚未接窗口提交；无 source 的栅格覆盖层明确拒绝，等待独立坐标框恢复；矢量、
裁剪/翻转、持久擦除路径尚未恢复，不得将此增量当作完整编辑状态恢复。
裁剪布局候选随后已接入：imageEditState 从原文借用，提取逻辑源框和完整源图翻转
标记，按正常展开窗口框计算重置位置与 contain-fit 图片放置；不按自然图片像素猜测。
测试覆盖缩放后裁剪与正方形图片在长方形源框内的留白，缺失任一裁剪字段不伪造裁剪。
这里只完成布局元数据，尚未应用到 Editor、矢量/擦除轨迹坐标或实际绘制。
图片翻转随后已接后台整批准备：即使没有裁剪字段，也读取 flippedX/flippedY，
只处理底图/预览；主干保存时已翻转的栅格标注保持原样。共享缓存按引用和方向区分，
复用可取消、受像素预算限制的翻转内核。真实 PNG 编解码及后台准备回归验证像素
顺序、跨窗口共享、标注不二次翻转。输出像素已归一化，后续绘制不能再次应用该标志。
两轴同时翻转现使用单次分配/遍历的 image_axes 内核，避免先水平后垂直产生中间整图；
旧单轴入口复用同一实现。组合/退化尺寸回归与逐轴结果一致，无变化共享源像素，
取消不交付半图。这是分配路径优化证据，不是整机峰值内存或最终性能基准结论。
基础矢量恢复已接 Candidate.prepare_vectors：矩形/圆角矩形/椭圆/三角形/多边形、
直线/箭头/折线/普通画笔/荧光笔转换为新 Editor，按 zIndex 稳定排序并保留标注 ID。
支持 RGB 十六进制颜色、透明填充、线宽/透明度/虚线/旋转；密集点不经交互采样丢失，
恢复内容作为不可撤销的初始状态，后续平移可正常撤销。serialCounter 进入新编辑器。
模糊、无路径矩形马赛克以及其他颜色格式仍需实现，目前明确拒绝
而非跳过；裁剪后的坐标转换和栅格编辑器合并仍未接通，不能发布完整恢复入口。
贝塞尔和动态笔刷随后已接入：贝塞尔保留 3n+1 控制点及箭头标志，不转换折线，
遵守原生 64 段上限；主干 brush+tapered 转为 Paintbrush，保留密集点和 brushWeights，
复用原生起收笔采样。回归覆盖控制点/权重保留、实际采样与非法曲线整批拒绝。
文字/序号随后已接入：普通文字通过注入 DirectWrite 实际测量，将持久基线转换为
原生布局框；序号按主干圆心/半径/字号规则建立圆框，支持半径 999，不错误套用矩形
圆角上限。恢复文本/字体/旋转，保持初始无历史。真实 DirectWrite 回归覆盖中文和
emoji 基线位置；跨浏览器/DirectWrite 的字形外观及多行旋转仍需可见对照验收。
笔刷式马赛克已进入恢复 Editor，保留路径与独立 brushWidth；网格 strength 按主干
取整并至少为 2，缺省/零回退 12，复用现有不读取底图的原生网格绘制。
测试验证宽度不误用边框 style.width、透明度/路径/网格大小保留和初始无历史。
无路径矩形效果仍明确拒绝，不用圆头路径近似矩形遮挡范围。
笔刷模糊的模型恢复现有 restore_with_resources 入口：先取得与小数 strength 精确
匹配的 BlurImage 再构造原生路径，同强度可共享纹理，资源缺失/错误使整批失败。
回归使用实际高斯模糊内核生成纹理，验证笔刷宽度、强度和像素共享；普通无资源入口
仍拒绝模糊。桌面候选尚未接入底图投影/裁剪后的模糊资源提供方，不代表实际会话模糊已可用。
效果源图准备随后增加逻辑画布投影：透明留白、裁剪偏移、预乘 BGRA 双线性采样与
分数边缘覆盖，恒等变换共享像素，逐行可取消且分配受预算约束。恢复图片可按正常
窗口尺寸投影底图，再合成独立尺寸的栅格标注；像素测试覆盖留白、裁剪、半透明边缘
及 source-over 合成。该输入准备入口尚未串联到模糊纹理缓存和会话窗口提交。
随后 Candidate.prepare_vectors 已串联该投影源图与模糊资源提供方；单窗口惰性创建
源图，按精确 f32 强度缓存纹理，相同强度路径共享像素，不跨窗口/底图复用。
测试通过完整 Candidate 路径验证缺图拒绝、逻辑窗口尺寸纹理、路径坐标、相同强度
共享与取消；缓存回归另外验证 4.4/4.5 不混用。计算仍需由恢复工作线程调度，当前
尚未安装到可见窗口；无路径矩形效果、持久擦除和整体编辑状态提交仍未完成。
恢复 Job 随后已统一调度图片与矢量/效果准备，返回 VectorCandidate 而非仅图片候选；
Bound 持有已准备编辑器，不在目录绑定时重新做投影/卷积。后台真实 DirectWrite 和
高斯模糊回归验证通知来自工作线程、结果纹理尺寸/强度和析构释放；未知标注使 Job
失败而非发布不完整候选。完整编辑状态合并、窗口安装和正式数据写回仍未接通。
未裁剪文档随后已合并栅格与矢量：后台将底图 contain-fit 到正常逻辑框，保留独立
栅格标注纹理，再用初始化专用入口附加到已恢复矢量 Editor，不生成导入历史。
回归验证逻辑尺寸/留白、矢量坐标不二次缩放、仅擦除栅格标注及撤销保留矢量。
裁剪文档后续已接入候选，当前边界见本节开头；不以展平可见区域代替无损恢复。
Canvas 的独立标注保留可见框纹理，只有效果计算使用完整源框投影，避免二次映射。
无源图覆盖层、持久擦除、边框等其他编辑状态及窗口提交仍待完成。

文件/图片参数：已对照前端 Data URL 合同实现有界流式读取核心，覆盖短读、末尾填充、
字节预算、取消和读取失败；已接原生文件选择、单 worker 后台读取和版本核验提交，
隐藏窗口实际文件回归覆盖成功写入及过期拒绝。可见文件框、缩略预览仍未验收/完成。
128 KiB 合成文件已验证通过生产参数解析进入 Loom 请求，未混入路径/secret；读取
失败保留已有值，非法参数/锁定准入也有回归。此处未发起真实 Loom 网络执行。
显示视图不再复制未连线手动值，上传 Data URL 只由手动参数 owner 持有；连线值仍有
独立有效视图，未连线菜单读取原值。全量回归覆盖这一所有权调整，性能数值尚未实测。
连线显示副本在分配前检查字节、节点和深度预算；嵌套 JSON 类型/整数精度、转义字节
预算及容器边界有独立回归，预算失败不放开手动编辑权限。
常见 MIME 映射已有实现；浏览器扩展映射与大文件传输预算仍需进一步对齐。

能力参数交互增量：右键菜单提供布尔切换和枚举选择，保留值类型、禁用标记与 secret
边界，菜单快照过期不提交。实际隐藏 HWND/菜单测试已覆盖提交与取消旧执行的调度；
数字和文本已接入独立原生输入框，数字按主干限制范围并规范化精度，文本保留空白与
空字符串；增减按钮按 step 修改草稿。switch/radio/slider/path 声明已有对应入口，
number/slider 已有系统滑条，与输入框和按钮同步草稿，隐藏对话框消息链回归通过；
拖动期后端预览仍未接入。path 按主干提供文本输入，主干没有独立路径选择器。
color 已接入系统颜色框及主干清空行为，其他公共标量有默认值重置；枚举重置区分
未设置与显式 null。RGB 转换、清空和重置提交有回归；颜色框本身及
新输入框尚未执行真实模态交互验收，
JSON/object/array 也已接入单参数编辑，格式化/解析对照主干且不额外限制结果形状；
超长值保留高级整批 JSON 入口，完整面板未完成。
参数禁用与恢复已有原生菜单、带预算的原值暂存及执行 wire 回归：禁用不发送暂存原值，
启用恢复 null/普通值，未设置采用默认回退。暂存区预算失败保持原状态；持久化恢复未接通。
菜单勾选和输入初值已复用默认值/null/禁用回退，以及主干常用的布尔/数字/文本转换，
显示本身不写状态。连线有效参数已复用执行解析器进入原生菜单，驱动值只读显示，
保留手动值；有效视图变化拒绝旧菜单，断连恢复手动初值。真实执行链及隐藏菜单回归
通过。已有连线可从分页原生菜单断开，应用层验证拓扑/端点/锁定后更新，隐藏窗口
集成验证过期拒绝及画布回退。连线创建已有原生三下拉框与应用层排队提交，隐藏窗口
链覆盖端口索引、自连接、重复请求、过期目录拒绝和创建后恢复显示；可见选择框待人工
验收。隐藏系统选择框已验证来源切换、端口列表更新、不自动提交和子控件销毁；
实际应用链覆盖循环拒绝不改图、占用输入替换不增边。拖拽绘制和极值字符串格式仍需继续迁移/对照。

上游已编辑画面的直接像素合成已有原生接口与真实工作线程测试，复用导出渲染并避免
PNG 往返；草稿/取消/非法尺寸拒绝及预算租约释放已有回归。现已接入单 worker、完成
唤醒和按来源版本淘汰的显示缓存；隐藏窗口链覆盖上游已编辑图片合成及目标 Art 独立
覆盖层、撤销与来源尺寸更新。缓存只为仍被目标引用的来源保留，不后台定时重绘。
此为生产消息链回归，不代表可见人工验收或主干视觉全量对照通过。

当前增量：Art 自身生成结果支持已有矢量/裁剪覆盖层随图片尺寸映射；显示与临时编辑
复用同一转换，文字重测，取消恢复原文档身份，正式端口像素不变。核心测试覆盖非等比
映射和失败，隐藏窗口链覆盖标注与裁剪的接管/取消。连接回退到上游正式图片的 Art
也可编辑自己的覆盖层；新增链回归验证来源与覆盖层归属分离、取消、提交、一步撤销，
不修改上游文档或目标正式输出代际。上游本地编辑图片的叠层合成已接通；主干像素级对照及
可见人工验收仍未完成。下列历史阶段的编辑未适配描述不代表这些已验证路径的最新状态。

外部显示的全部标注栅格化已通过菜单及真实后台渲染回归：输出标注层、来源隔离、
暂存等待、取消和一步撤销通过；未扩展为所有效果/字体组合或可见人工验收。

继承来源裁剪的 RESET 菜单已接入暂存接管；隐藏窗口验证启用状态、重置、来源不变和
一步撤销恢复来源显示。未据此宣称全部裁剪拖拽、模糊重算或几何组合已完成。

暂存接管新增真实隐藏窗口的擦除/水平翻转后台链验证：成功结果像素、Esc 取消、来源
隔离、任务等待和一步撤销通过。翻转菜单支持无底图但有外部显示的钉图；可见交互、
复杂组合和全部变换工具尚未验收。

复制和 PNG 导出现在读取当前显示快照，并校验显示代际；外部像素、标注裁剪的编码/
解码和换源拒绝已有回归。系统剪贴板、保存框交互与可见人工验收未执行；编辑目标
尚未适配。下列旧阶段“导出未适配”以本段的复制/PNG 增量为准，不泛指全部导出路径。

最新增量：WM_PAINT、钉图视口 fit 和透明命中区域已选择解析快照；隐藏窗口生产
消息与 GetWindowRgn/PtInRegion 检查通过。编辑目标、导出、最终像素比对及可见
人工验收仍未完成。下列旧阶段“未接入 WM_PAINT”不代表最新状态。

快照绘制入口已有隐藏 HWND 冒烟：绘制本地标注快照及撤销后快照，再恢复本地图，
原图和编辑凭证不变。入口复用目标资源，但尚未接入 WM_PAINT，也未做像素比对或
透明命中区域适配；可见显示联动仍未完成。

显示数据层已接入工作区解析，独立快照共享来源像素并保留本地已提交编辑信息，
不改写原图或历史。图片链集成测试验证 Art 和中继钉图获得正式图片显示快照，
无变化时命中版本缓存。WM_PAINT、外部图片的编辑/导出/坐标适配仍未接线，
因此这一项只算显示状态增量，不算可见画布功能通过。
显示生命周期回归另覆盖断链/重连、来源类型切换、本地标注撤销与旧快照隔离；
变化才排队重绘已接线，但隐藏宿主不证明实际可见更新区域或最终像素。

直接 Art→Art 标量自动链已通过 `application_chain_tests.rs` 验证：第一节点唯一手动
启动，正式结果提交后第二节点自动联网，服务端核对派生参数，最终验证两节点结果和
全部任务回收。测试使用自建回环 TLS 与隐藏窗口；不代表可见图 UI、钉图中继或多分支
工作流已完成。

当前集成增量：运行菜单已使用实际工作区连线解析本地钉图及上游正式图片输出，
通过后台准备生成正式图片输入；隐藏窗口测试从生成消息解码 PNG 核对像素，并通过
生产调度器发送至自建回环 TLS 服务，再验证服务端收到的像素、标量结果提交及任务
回收。没有连接用户实际 Loom 服务。来源、配置及拓扑版本分别校验；编辑和配置依赖
覆盖目标及整个上游子图，无关文档编辑不取消执行；拓扑凭证仍为全工作区范围。
正式标量端口现已接入运行入口的参数解析，并由菜单回环测试验证控件转换后的参数实际
发送且不修改手动值；零图片准备也携带上游结果依赖。可见连线 UI、手动外部图片引用、
图片到参数引用及完整中继状态
仍待接入。下文按阶段保留的“尚未接入”描述不代表这一增量未实现，也不应反向
将已有模型测试视为完整用户功能验收。最新边界见迁移计划的“当前图片运行入口”。

原生 State 现有独立工作区对象 ID，新对象使用系统 UUID；换图、编辑和撤销不改变 ID。
`workspace_id.rs` 接受有界不透明字符串，兼容旧 ID 而不强制将其重写为 UUID。应用级
新窗口/导入请求以 ID 保留来源，错误反馈时只解析仍存活的宿主，关闭对象不再解析，
重新建窗获得不同 ID。真实隐藏窗口冒烟覆盖这些生命周期；当前未序列化 ID，也未
实现完整工作区对象图及启动恢复，因此不能称为持久化迁移完成。
`workspace.rs` 已接入应用 owner 的身份集合：单窗口创建与批次导入注册对象，回收关闭
窗口时移除身份及关联链接。链接值模型验证端点、唯一 ID、有界端口和总预算，删除规则
对照主干 `graphStore.ts` 的关联边级联清理。模型测试覆盖批次原子拒绝、无效链接、
双向清理、无关边保持及预算回收；隐藏 HWND 冒烟验证导入后的集合一致、关闭对象
清理双向边和退出清空。当前链接只能通过模型 API 创建，尚无连线 UI、端口类型合同、
图执行或序列化；不能把身份/边集合当成完整工作区迁移完成。
交互连线模型现已对照 `useLinking.ts`：拒绝自连接、按现有图检查目标到来源的可达性，
将目标输入首条旧边替换为新边并保持其余边顺序。原生事务在删除旧边前完成验证与容量
预留，失败不丢失原连接，满 4096 边仍允许等量替换。回归覆盖替换、不同输入端口、
非法端点/端口/重复 ID、循环拒绝、已有导入环的终止及 128 对象长链；底层导入仍允许
原有有向环。此接口尚未接入端口拖拽、连线绘制、端口类型校验或图像执行传播。
图像来源解析内核 `graph_image.rs` 已对照 `graphTraversalResolution.ts` 和
`imageGraphPrimitives.ts`：钉图优先解析允许中继的上游，其次 data URI 手动来源，最后
本地预览/底图；禁用输入、停止接收或本地编辑会阻止中继。Art 显示预览不能作为正式
输出回退；精确端口输出优先于通用 output，存在的非图像输出不会被预览替代。能力声明
空输入覆盖对象端口，主干的端口别名/大小写规则、连接顺序和等待状态中继也已保留。
回归覆盖已有环、缺失对象、128 节点链及借用来源身份；只处理归一化引用，不做图片 I/O。
原始协议 JSON 解码、完整执行参数解析、文档像素同步和桌面连线入口仍未接入，不能称为
图像执行/显示迁移完成。
包装值解析现已通过 `graph_output.rs` 接入图片连接及执行输入：保留原始正式输出值，
图片消费端按主干字段优先级解包一层，false/0/空字符串不会被后续 src 字段覆盖。
结构预算覆盖深度、值总数、对象字段数和字段名，并拒绝重复字段。回归验证包装输出
经真实图解析进入执行输入，以及非图片字段阻断错误回退；当前是借用类型适配，尚未
接入 JSON 解码、Loom 传输或节点调度，不能称为外部协议链已完成。
执行图片输入解析已在 `graph_execution_images.rs` 接入同一图视图，对照主干
`executionImageInputs.ts`：有连接但正式输出未就绪时不使用显示回退；图片输入映射
连接优先、手动同名参数次之，辅助输入仅使用能力声明。缺失端口判定保留主干与参数
映射的区别，主输入的手动值不自动消除缺失，辅助项才可由手动参数补齐。回归覆盖
未就绪 Art、能力空列表、声明去重与顺序、非图像值和输入预算；普通标量参数、协议
包装值、节点调度与文档像素同步仍未完成，不能把这些纯解析接口当成已运行工作流。
公共参数候选已在 `graph_params.rs` 对照 `nodeParamResolution.ts` 接入默认/手动覆盖、
显式 null、未声明手动参数、手动禁用及连线覆盖准入；secret、__exec_、__ui_resize、
force_update 在所有写入路径排除。原生端对冲突声明采用更严格的拒绝泄漏规则：任意
同名声明标记 secret 就不导出该参数。回归覆盖这些边界及非法结构拒绝后保留原值；
widget 转换现已接入参数候选提交：数字/slider 有限值转换与上下限、checkbox/switch
布尔词、file/image_link 非空字符串、text/color 标量及数组文本化、未知 widget 一层
解包。失败不覆盖旧值，生成文本由候选拥有。数值空白、进制舍入、科学计数阈值和负零
夹具已用本地 Node 验证；原生回归还覆盖转换大小限制与文本替换释放的所有权路径。
全部 JS 浮点最短文本边界尚未穷尽验证，执行请求封装仍未完成。
图输出到参数的绑定现已通过 `Graph::effective_params` 接通：参数声明携带 widget 元数据，
按边顺序查询正式输出，复用包装值转换和安全过滤，缺失/转换失败保留先前候选。
组合回归覆盖包装数值限制、后续有效/无效边覆盖规则、secret/控制/禁用阻断、Art 预览
不冒充输出、空来源端口兼容、候选文本所有权及缺失/非 Art 目标。仍未发送执行请求、
运行节点或同步桌面文档，不能把纯模型组合验证当作完整工作流执行。
请求生命周期已对照 `artExecutionRequests.ts` 在 `graph_requests.rs` 实现并由应用 owner
持有：begin 推进代际并替换旧请求，过期预览/完成拒绝，取消和结束释放预览，关闭对象
回收代际元数据。原生端进一步检查完整 request ID + generation，限制注册项和预览
引用缓存；容量/代际溢出失败保留当前请求。单元回归覆盖替换、过期、预算与清理；
隐藏 HWND 冒烟验证真实关闭后拒绝旧交付、不影响其他对象、应用退出释放预览。
尚未调用外部服务或建立执行工作线程，本注册表不是已完成的调度器。
正式执行准备已由 `Graph::prepare_dispatch` 组合上述模型：触发配置准入、图片缺失
检查、图片输入与有效参数全部成功才返回候选。配置对照 `nodeExecutionConfig.ts`，
正式触发门槛对照 `useNodeParameters.ts`：纯预览不派发、参数拖动未最终提交不派发、
自动缺图跳过而手动可继续。候选输出活动公共参数、图片输入和禁用参数名，secret 名称
不进入公开禁用列表。组合回归覆盖正式图片/数值转换/过滤、手动缺图、跳过原因和失败
不修改来源；此准备接口尚未绑定传输、执行工作线程或桌面触发入口。
execute_art/cancel_art 消息封装已按主干 action/payload 形状实现，包含对象、请求 ID、
generation、Art ID、图片输入、活动参数和禁用项。编码大小限制为 4 MiB，中文保留
UTF-8，控制字符/引号/反斜杠转义，非有限数值按 JSON.stringify 语义编码 null；无可
序列化值在对象中省略、数组中为 null。实际 Rust 输出已作为协议夹具验证，stage 在两条
消息编码成功后才替换请求代际。预算或 Art ID 验证失败保留旧请求及预览；尚未发送
消息，也未验证真实 Loom 的接收、执行和结果交付，不能称为传输链完成。
进一步核对 `art_transport.rs` 后，已明确区分内部 action/payload 和实际 WebSocket
method/params：`loom_art_wire.rs` 编码 loom.hook.art.execute/cancel、loom.hook.v1 以及
camelCase 字段；图片输入通过独立类型编码为内联 PNG 或 RGBA 共享内存描述符，而非
直接发送来源字符串。输入端口、数量、尺寸、共享内存字节数、Base64 格式及传输/驻留
预算都有检查，输出 transport 显式选择。协议回归只使用描述符夹具，不解码 PNG、创建
共享内存或连接真实服务；图像资源准备、握手、网络发送、输出租约与释放仍待接入。
真实内联图片准备已接入 `desktop/loom_inline.rs`：原生图像和已提交 Editor 快照经现有
WIC/D2D 合成编码为 PNG，再生成有预算 owner 的 Base64 描述符。回归覆盖精确像素、
裁剪后实际输出尺寸、独立标注纹理覆盖旧宿主图片、文档不变、取消及不保留输入像素。
实际生成的 PNG/Base64 另经 .NET Base64 解码和 System.Drawing 检查尺寸/颜色；不写
文件或访问系统剪贴板。Base64 租约回归验证预算失败/取消/析构归还。WIC 内部编码
不可中断且临时流尚非精确预算计量。
`desktop/loom_input_job.rs` 已提供单 owner 单任务、全进程最多两个 worker 的批次准备，
最多 32 个输入顺序编码，整批成功才交付，取消/失败/析构回收候选。真实后台 WIC 回归
覆盖端口顺序、重复/超量拒绝、并发准入、完成后取消、任务复用和输入像素释放；交付前
检查目标与来源文档关闭、替换、编辑及草稿，避免消费过期快照。该 API 尚未接入应用
执行调度，图连接/参数变化还需执行层校验；共享内存输入、执行传输 worker 和真实
WebSocket 发送仍未完成，不能视为完整资源传输验收。
外部协议现可通过 `loom_art_wire::stage` 原子生成 execute/cancel 消息并注册新请求；
与内部 action 编码共用注册事务。回归验证输入缺失、无效 Art ID 和取消消息编码失败
保留旧请求/预览，成功后使用正确的外部取消协议并拒绝旧请求完成回调。该增量仍不
打开连接，也不代表服务端已经接收执行；发送错误与响应流尚需实际传输层承接。
已核对主干 `loom_bridge_client.rs` 的认证要求：受信任发现、TLS 1.3、固定叶证书、
TLS exporter 身份证明，而非匿名 WebSocket。原生 `loom_bridge_stream.rs` 已提供带
绝对操作截止时间及 shutdown 中断句柄的底层流，自建回环测试验证分片不刷新预算、
已超时拒绝消费、显式续期、对端不响应超时和取消中断。未连接真实 Loom；发现、TLS
认证及 WebSocket 升级尚未迁移，该流不能单独证明安全连接或完整传输可用。
原生身份校验已对照主干 `loom_bridge_client/identity.rs`：仅接受规范的本机 WSS
地址，检查协议、UUID、64 位十六进制令牌和有界 Base64/SHA-256 证书固定值；凭据
调试输出脱敏，地址覆盖只能重复发现值。夹具覆盖非法端点、输入和固定值拒绝；
其证书字节只用于摘要合同，不是 TLS 握手证明。未读取真实发现文件，未完成来源
权限校验或证书链验证。新增依赖固定为主干锁文件版本；尚未运行新锁文件的漏洞扫描。
后续 `loom_bridge_client.rs` 已接通实际 TLS 1.3/WSS 握手，使用唯一发现证书信任根、
叶证书二次比对和 TLS exporter 证明；连接/TLS/升级共享三秒总截止时间。回环 TLS
服务验证认证头、实例身份、协议版本及消息往返，错误证书在应用数据发送前拒绝。
这些测试不访问真实 Loom，不能替代发现文件来源校验或应用执行链验收；发现解析、
开关策略、Art 响应处理及 UI 调度仍未接入。新依赖的漏洞扫描仍未执行。
清单字节解析已迁移至 `loom_discovery_manifest.rs`：1 MiB 上限、UTF-8/JSON 校验、
主干元数据/HTTP origin/auth/能力列表校验和 hookBridge 身份验证。夹具覆盖主干默认
允许的本机 HTTP 与远端 HTTPS 元数据、错误身份/能力、危险 URL、重复已知字段和
尾随 JSON 拒绝。该模块不公开序列化凭据，不读取环境或磁盘；发现路径选择、来源
权限和实际默认连接仍待完成，不能据此声称可信发现已全面迁移。
默认发现入口现由 `loom_discovery.rs` 实现：主干路径顺序、规范/旧环境变量优先级、
默认开启及显式禁用、有界文件读取、地址覆盖检查，最后调用已实现的 TLS/WS 连接。
注入测试验证禁用不读文件、首个存在清单优先、读取失败不降级及字节上限；回环连接
测试进一步经清单解析和身份校验后完成真实握手/消息往返。没有读取用户实际清单或
调用真实默认连接。复核主干发现实现也未校验 ACL/所有者；原生当前同样将本地配置
来源作为部署信任边界，而非声称已完成文件来源认证。应用调度和真实服务验收仍待接入。
Art 响应解析已对照 `art_transport.rs`/`formal_delivery.rs`：支持 progress、preview、
result、failure 方法消息及 succeeded/failed/cancelled 状态包络。预览/结果校验对象、
请求、代际和 revision；过期预览忽略，非法最终提交报错，保留 outputs/candidates。
解析使用有界单消息，移动输出所有权，失败文本截断为 1024 个字符。夹具验证两类
结果包络、身份不匹配、错误/未知消息、进度默认值及文本上限；尚未接入 socket 读取
循环、revision 去重、资源解码/释放或应用交付，不能据此认定执行结果功能完成。
随后 `loom_art_exchange.rs` 已接入 socket 读写循环：有界发送、Pong、进度/预览更新、
终态返回、总期限和回调取消。真实回环 TLS/WS 测试覆盖 Ping/Pong、无关消息、更新
顺序与最终结果，以及回调取消、对端关闭、持续无关消息不能续期。尚无应用 worker
调度、输出资源解码/释放和 revision 去重，真实 Loom Art 执行仍未验收。
取消/释放控制通道已加入 `loom_art_control.rs`：有界资源释放编码、1 MiB 控制帧/消息
限制、五秒总等待及严格回执身份筛选。真实回环 TLS/WS 测试发送取消与释放消息，
经历 Ping/Pong 和无关回执后确认对应成功回执；模型测试覆盖句柄边界和回执状态。
尚未连接执行 worker 的取消生命周期，也没有共享内存租约 owner 或实际释放服务验收。
内联图片输出已有真实解码路径 `desktop/loom_output_inline.rs`：formal 描述符白名单
及 Base64 校验、有界编码缓冲、共享像素预算、阶段取消检查，并复用 WIC 生成原生
PBGRA。回归把真实生成 PNG 放进 Art 最终结果包络，经响应校验后解码，验证精确
像素、忽略错误尺寸提示、像素 owner 释放、非法输入和预先取消。当前只验证 PNG
实际编解码，其他白名单格式依赖系统 WIC codec，尚未逐格式验收；共享内存输出、
多输出交付和应用执行调度仍未完成。
共享内存输出已有本机读取实现 `desktop/loom_output_shared.rs`：校验句柄、格式、
尺寸及字节数，以只读限定视图复制并完成 RGBA→PBGRA，返回预算像素后解除映射。
自建命名映射测试覆盖半透明/不透明精确颜色、映射释放后独立像素保持、无本地句柄
泄漏、超出映射范围拒绝和预先取消；测试不接触真实 Loom 映射。远端 release 仍待
资源 owner 调度，对端租约不可变及实际服务互操作未验收，不能视为完整资源生命周期。
多输出准备现由 `desktop/loom_output_batch.rs` 组合三种 formal 输出：value、内联
图片与共享图片，保留主干主端口优先顺序及 64 候选上限，增加 128 端口和 256 MiB
图片总预算。全部成功才交付；失败/取消不返回半批。回归验证真实 PNG 与结构化值
同批交付、主端口优先级、空值、后续端口失败、输入不变及边界拒绝。原始描述符和
候选元数据仍由调用者持有，尚未接入远端租约 owner、文档更新和完整 Art 工作流。
远端租约模型已加入 `loom_output_leases.rs`：绑定对象/执行请求/代际，去重正式输出
句柄，串行跟踪释放请求；只有匹配当前尝试的成功回执会清空列表，其余结果保留待
重试。模型回归验证重复句柄、普通 value 不误识别、旧回执拒绝和失败保留；回环控制
测试现经过该 owner 发起释放并消费成功回执。尚无应用级有界重试队列或退出清理
调度，Drop 不自动联网，不能据此宣称所有远端资源已完成生命周期管理。
交付组合 `desktop/loom_delivery.rs` 已在解码前记录租约，将本地结果与远端释放责任
分开保存。回归覆盖映射不存在导致解码失败、预先取消仍保留租约、传输失败后重试、
失败回执保留以及无资源不连接。真实自建映射的组合测试验证复制完成后提供方关闭
映射，本地结果像素仍有效。控制回调仍由调用者注入，应用 worker、退出重试及真实
服务的释放生命周期验收尚未完成。
交付对象现可通过 `release_authenticated` 直接调用实际 TLS/WS 控制通道，固定使用
执行时 Identity，并为每次尝试生成新 release UUID。回环测试让解码失败的交付对象
经历一次真实失败回执和一次成功回执，确认失败保留、重试 ID 不复用、成功才清空。
未重新发现或读取用户凭据；应用后台调度、自动重试上限与退出处理仍未接入。
同步后台会话现由 `desktop/loom_execution_session.rs` 组合上述路径。真实回环测试
验证进度、重复/乱序预览过滤、最终多输出和候选元数据；预先取消不连接。另一个
双连接回环测试在预览解码失败后返回释放失败回执，确认会话返回仍持有租约的交付
对象且不发布更新。该层未创建后台线程，应用任务调度、UI 状态提交、输入 owner
绑定和远端执行取消仍待接入，不代表完整可用的主干 Art 功能已迁移完成。
`desktop/loom_execution_job.rs` 现将会话放入后台线程：全进程最多两个 worker，输入
批次持有到退出，进度/预览各一个合并槽，完成结果携带执行身份与失败租约 owner。
回环测试验证真实后台执行、最新进度消费、最终结果身份及显式 shutdown 中断读等待
并回收线程。应用尚未创建这些任务，远端取消消息与失败租约重试队列仍未接入；Drop
只有尽力清理诊断，不能据此声称应用退出时远端执行/资源都已清理。
执行任务现已连接远端取消控制通道：本地中断后在固定身份的独立连接发送取消，
Completion 分开保存本地结果和取消回执；服务已返回终态或尚未建立连接不多发取消。
后台任务回环测试扩展为两个连接，验证取消请求对象/请求/代际正确，并分别检查
成功与失败回执都被如实保留；正常成功结果没有额外取消。应用级重试、窗口关闭
调度和真实 Loom 行为仍待验收，不能把某次回环成功扩大为全部退出场景已完成。
全量测试曾暴露 shutdown 后仍等待约 30 秒读超时的场景。可取消会话现以 250 ms
阶段读预算检查标志，保持总响应期限，不再只依赖 socket 唤醒；新增回归刻意不用
中断句柄，仅设置取消标志验证退出。握手/写入/释放仍可能等待各自预算，不能把读
检查间隔宣称为端到端取消延迟上界。
短读超时恢复另有真实 TLS/WebSocket 回归：先发送非最终文本分片，等待 400 ms，
再把最终 continuation 帧分成两次 TLS 写入，中间再次等待 400 ms。可取消会话经过
多次 250 ms 超时仍完整解析最终结果并保留中文/emoji，验证读取状态未因轮询丢失。
后台执行结束后现有有界清理补偿：失败取消/释放最多再尝试两轮，100/500 ms 退避，
不新增线程或无界队列。回环任务测试验证首次成功零重试、一次失败后恢复，以及
连续失败止于三次请求并保留 cleanup_pending；原执行失败不因清理成功而变成成功。
Drop 不追加第四次请求。应用仍需接管达到上限的失败状态和退出策略，不能保证服务
不可达时远端资源一定被回收，也不能将后台网络预算当作 UI 关闭延迟承诺。
应用集合现持有 `loom_execution_tasks.rs`：最多两个包含未消费结果的任务槽，文档
关闭或请求替换时取消但保留 owner，非阻塞轮询后领取带 current 标记的完成报告。
Application 的文档回收/消息处理/循环退出已接入取消、轮询和显式收尾；未确认清理
会返回退出错误而非报成功。真实回环任务测试覆盖关闭与替换后退休、旧完成报告非
当前、新请求保持及显式领取回收槽。尚无 Art 启动 UI、完成唤醒和画布应用结果入口，
当前窗口测试仍不启动真实网络任务，完整交互链和实际服务验收未完成。

### 原生依赖安全复核（2026-10-17）

`cargo audit --file native/Cargo.lock` 联网更新 RustSec 后发现 rustls 0.23.45 命中
RUSTSEC-2026-0346/0347/0348/0349。原生目标已将固定版本及锁文件升级到 0.23.46，
没有修改 Tauri 目标锁文件，也没有添加忽略项。复扫退出码为 0，覆盖 110 个依赖。
扫描器 cargo-audit 0.22.2；公告库提交 `64f92cdc64401fab47baa10e2e11fe382aa57155`；
锁文件 SHA-256 `1A294BD96600F9D1549342099C71979B7A82F5B0DBAC240F3086B6996C4D8973`。
这是该锁文件和公告库版本的 RustSec 证据，不代表零风险，也不替代正式发布要求的
固定 OSV 扫描、SBOM 和实际发布来源验证。此前“尚未扫描”的记录保留为历史状态。
分组模型已对照 `stickerGroups.ts` 和 `graphStore.ts`：支持元数据更新、单组成员关系、
整批赋组/解除、删除组保留对象、关闭组删除成员并级联边。窗口回收会清理已关闭成员，
保留其他成员及空组，最终应用退出清空分组。模型回归覆盖失败原子性、去重、迁组、
名称/数量预算及两类删除语义；隐藏 HWND 验证成员关闭和退出清理。
原生右键菜单现已提供创建、加入已有组、移出、重命名、删除组和关闭组。
原生分组目录子菜单提供任意组的管理入口，包括无成员组，不要求先将图片加入该组。
隐藏 HWND 回归覆盖空组重命名、删除、关闭以及无关文档保持；真实菜单结构测试覆盖
子菜单命令、成员勾选和锁定准入的递归检查。原生名称输入使用
专用标题的原生输入框，自动回归验证中文、空名称和字节上限；请求在应用循环中处理，
遇到任一应用模态暂缓，单宿主不累积队列。隐藏窗口冒烟经过生产菜单命令，验证目录
同步、删除保留窗口、关闭成员保留无关窗口以及请求合并/延后。真实可见菜单、键盘与
DPI 交互尚未人工验收；分组栏及持久化仍未接入。
组隐藏/显示已接入菜单和托盘恢复请求：先建立每个成员的恢复入口，再隐藏宿主及面板，
释放绘制设备缓存；中途失败回滚本次隐藏，原本已隐藏的窗口保持原状。恢复任一成员
请求显示整组，移出或删除隐藏组会恢复相关窗口。隐藏 HWND 冒烟注入托盘安装及显示
适配器，验证失败回滚、只影响组成员、整组恢复请求和状态清理，不注册真实系统托盘。
当前仍是每文档一个托盘 owner；实际可见窗口、Explorer 重启和系统托盘交互待人工验收，
不能把注入回归当成完整 Shell 行为或资源性能验收。
组锁定/解锁已加入菜单：锁定阻止标注指针、编辑键盘、滚轮变换、系统拖动/调整大小
及修改型菜单；保留分组恢复操作、复制/导出和创建新对象，不关闭或禁用整个 HWND。
加入锁定组或锁定当前组前检查全部相关宿主，仍有编辑后台任务则拒绝本次操作，原组
状态不变；允许用户完成或取消后重试。模型提交后同步成员标志和面板状态，取消旧手势
并使画面失效，不增加空闲轮询。真实隐藏 HWND 验证忙碌拒绝、只锁成员、输入不改文档
和解锁后恢复编辑；单元测试覆盖系统拖动拒绝、生命周期消息放行及替换任务入口拒绝。
实际可见拖动、键盘快捷键与锁定时输出操作仍需人工验收。

Ctrl+O 和“打开图片为新钉图”现通过原生文件选择器创建独立钉图，不清空源文档；
“打开图片替换当前图片”保留显式替换。选择器结束仍核验宿主存活及文档身份。
`application_open_smoke.rs` 注入仓库图片路径，验证真实后台解码和独立窗口，以及
选择取消、选择失败和嵌套文档变化后的过期拒绝；不显示文件选择器，不读取用户文件。
新钉图的文件选择器支持最多 32 张图片，复用批次后台准备和全部窗口暂存提交；显式
替换仍限单张。`open_paths.rs` 验证 Unicode/空格、顺序、终止符、路径偏移、子路径越界
和批次数量/长度限制；打开链冒烟注入双文件并验证两个独立窗口。真实系统选择器多选
交互与主干工作区布局仍未验收，不把缓冲解析和注入测试当成可见 UI 验收。

Ctrl+V 和“粘贴图片为新钉图”默认保留当前文档，PNG、DIB 和剪贴板文件路径均通过
现有后台加载任务交付独立钉图；“粘贴图片替换当前图片”保留显式替换及确认行为。
`application_paste_smoke.rs` 注入三种格式快照，验证真实解码线程、窗口创建、源文档版本
保持、单槽待交付拒绝覆盖、取消不发布及显式替换。启动测试要求该证据，不读取或修改
系统剪贴板；实际剪贴板交互与主干工作区对象布局等价仍需验收。
剪贴板文件列表现支持最多 32 个路径，Unicode/ANSI 双 NUL 结束和批次限制均有夹具
回归；双文件粘贴冒烟验证两个钉图、源文档保持及显式替换拒绝整批。PNG/DIB 优先级
不变，只有无这两类格式时才读取文件列表；真实 Explorer 复制/粘贴仍待人工验收。

托盘及文档右键菜单的“退出 Otter”现在提交共享应用退出请求，不再只关闭托盘所属
窗口。请求合并，存在已登记模态占用时保留请求并等待该调用结束；应用循环随后一次性
关闭受管宿主并回收 owner/截图窗口表，最后窗口的正常退出仍归宿主计数处理。
隐藏 HWND 冒烟通过生产菜单命令验证重复请求、模态延后、全部宿主和面板销毁及集合
清空；不点击实际托盘。关闭行为沿用当前文档关闭语义，未增加工作区保存/启动恢复，
后台任务清理仍可能等待不可立即取消的系统调用，不能据此承诺固定退出延迟。

工具面板“区域截图”和 Ctrl+1 现在统一使用 `capture_input::DEFAULT`，创建新钉图而非
替换当前图片。替换入口仅留在明确标注“替换当前图片”的菜单项中，保留原确认流程。
可用性模型现在识别新钉图命令、共享模态/截图占用和待建窗单槽；截图被阻止时不禁用
独立编辑操作。面板在鼠标激活/键盘处理前重算状态，避免另一个文档解除模态后按钮仍
使用旧禁用状态。真实 HWND 冒烟验证默认按钮的跨文档禁用和恢复，以及待建窗/交付后
准入变化；不通过按钮或系统热键调用真实桌面捕获。

新钉图现在保留截图选区的物理屏幕原点：冻结帧虚拟桌面原点加选区偏移，经后台准备
交给窗口创建，支持负屏幕坐标并检查矩形原点/范围相加溢出。带截图位置的新钉图按原
物理像素尺寸创建，不应用普通打开图片的 1200×800 初始缩放；旧无位置信息入口保持
原行为。隐藏原生选区测试核验实际选择结果的负坐标，后台建窗冒烟核验 HWND 的位置
与 4×4 夹具尺寸。真实混合 DPI 多屏与可见贴合仍待验收，当前仅传递启动定位信息，
不是完整捕获元数据/工作区持久化格式。

右键菜单新增“区域截图为新钉图”：不确认丢弃原文档，后台复用 `open_job` 准备截图
图像和 alpha 命中区域，完成后以单槽结果交给应用集合创建独立钉图。建窗直接使用准备
结果，不在 UI 线程再次扫描整图；新文档共享已捕获像素存储，但不共享源文档的编辑身份
或历史。准备任务仍由发起文档拥有，关闭发起文档会取消并回收任务；建窗失败保留原文档。
内存截图夹具覆盖真实 worker/WM_TIMER 交付、新钉图类型、共享像素、原标注及撤销重做
保留、关闭新钉图不影响其他文档，不调用真实桌面捕获。显式区域/全屏替换入口仍保留；
完整捕获元数据及主干工作区中的钉图对象仍待迁移。

`modal_scope.rs` 通过各文档模态标志的弱引用观察应用内占用，不共享各文档的本地锁、
不延长已关闭文档寿命。截图入口和全局热键在任何已登记模态标志占用时拒绝进入，防止
隐藏另一个文档正在使用的对话框宿主。文字输入、字体、序号半径及自定义形状参数入口
也已补接本地 Guard；文件/颜色/精确属性沿用原 Guard。双文档冒烟验证另一个文档本地
锁保持独立，但截图公共准入被阻止，释放后恢复；弱引用与重复登记有独立回归。
该检查不显示实际对话框，尚未穷尽所有错误提示框及可见交互，不宣称全部模态路径验收。

截图隐藏范围已扩展为应用集合中的全部宿主和工具面板，而非仅发起截图的文档。
集合只在增删窗口时更新共享 HWND 表；捕获前复制窗口快照并释放借用，再隐藏窗口。
恢复时保留原隐藏状态，并核对 HWND 用户数据身份，避免恢复已销毁/复用的窗口。
各文档共享截图占用标志，区域/全屏入口互斥，全局热键也在恢复托盘窗口前拒绝截图
重入。双文档隐藏 HWND 冒烟核验四个窗口无遗漏/重复、原隐藏状态不变、共享占用释放
及关闭后列表收缩；未采集真实桌面，也未覆盖可见窗口的 DWM 像素或跨文档全部模态场景。

`application_hotkey.rs` 现在统一处理各窗口的 Ctrl+1 开关请求，菜单读取共享启用状态，
真实系统注册仍只归一个宿主。其他窗口可以关闭已有注册；重新启用时选择请求窗口，
失败保留原注册和菜单状态，未注册窗口的销毁清理不会把共享状态清空。请求在应用消息
循环中处理，每源窗口合并为一个待处理开关，不在菜单回调里重复注册。隐藏 HWND 冒烟
读取真实菜单勾选状态，使用内存注册适配器验证跨窗口开关、重复请求合并、失败保留及
非注册者清理；不占用 Ctrl+1。真实系统注册冲突/转交和自定义快捷键配置仍需后续验收。

`application.rs` 已接入实际运行消息循环，右键菜单“新建工作窗口”创建独立空白文档，
可在新窗口继续使用打开图片等既有入口，不覆盖原文档。每个源文档只保留一个创建请求，
回调返回后集中处理，最多 128 个宿主；创建前回收已关闭 owner，owner 析构还会核验
HWND 的 State 身份，避免旧句柄复用造成误销毁。工具面板键盘消息按窗口分派。
初始 Ctrl+1 注册者关闭时会尝试转交剩余窗口，用户主动关闭热键不会因此重新开启；
此转交尚未用真实全局热键验收；各窗口菜单统一控制的增量范围见上文。
隐藏 HWND 冒烟通过生产菜单命令验证重复请求合并、原文档保留、关闭初始窗口后继续
绘制/再次建窗以及关闭后回收。该入口是多个独立工作窗口，不代替主干工作区内多个
钉图对象、分组、连线和持久化；默认区域截图创建独立钉图，增量范围见上文。

前景让位订阅已取消单目标限制：同一 UI 线程的 `Watcher` 共享两条 WinEvent hook，
各窗口独立维护通知合并标志，确认一个窗口不清除其他窗口的待处理标志。关闭一个目标
不会注销其他目标使用的 hook，最后一个目标释放后注销系统订阅并释放目标列表。
目标表最多 512 项，每目标最多一条未确认通知；无空闲轮询。订阅由文档状态持有，
用户关闭时在 WM_DESTROY 释放，RAII 退出时在窗口字段析构前释放。消息专用窗口回归
覆盖共享身份、独立确认、部分释放、重复/无效登记及生产销毁路由清理；未模拟切换用户
前景窗口。完整可见多钉图让位交互仍需后续验收。

`host_lifetime.rs` 现按 UI 线程登记实际创建的宿主，仅最后一个宿主销毁时发送退出消息；
未登记/重复销毁不扣减计数，工具面板不参与计数。隐藏窗口冒烟同时创建两个宿主，在
第二文档编辑后关闭它，确认第一文档身份未变、仍可绘制且没有退出消息，再核验最后
宿主关闭后得到正常退出消息。冒烟有界处理当前线程队列，避免仅过滤低优先级 WM_QUIT
导致错误判定；不注册全局快捷键或捕获桌面。这证明关闭隔离，不代表完整工作区已完成；
集合入口、热键与共享前景订阅的增量范围见上文。

`document_window.rs` 的 `DocumentWindow` 现在拥有单个宿主窗口、工具面板、图标和
`Box<State>`，实际启动流程已使用此 owner。状态先固定地址再传入 Win32；正常释放时
面板和宿主先销毁，随后释放图标与状态，构造失败路径亦保持该顺序。普通/钉图启动冒烟
将 owner 放入会扩容的集合再取出，核验状态地址、两窗口关联与图标仍有效，并继续执行
既有编辑和窗口销毁检查。退出策略已由宿主计数接管，窗口集合入口见上文。

启动路径改为调用 `document_state.rs` 的 `State::new`，将单个文档的 Canvas、编辑历史、
工具配置、任务槽、模态门禁和面板缓存初始化从进程启动流程中分离。此构造函数不创建
窗口、不注册热键、不启动工作线程；钉图 alpha 命中区域仍在首次绘制前准备。
回归使用同一份共享像素构造两个独立文档，核验文档凭证、编辑/撤销/重做、工具、模态
状态和滚轮余数隔离，以及释放一个文档后另一个仍能读取图像并替换自己的文档。
这是 N06 多文档生命周期的基础，不是多钉图 UI 已完成：完整快捷键配置、工作区布局
与持久化仍未完成，不能将独立工作窗口等同于完整主干工作区。

## 精确模糊强度增量

工具面板的 owner-draw 操作按钮现在同时同步原生 HWND 文本，粗细、透明度及填充状态
不再只更新画面而保留旧名称；绘制与原生文本共用 `tool_panel_actions::label`。
仅在工具/样式变化时检查，文本相同时不发送 `WM_SETTEXT`，不新增空闲工作。
隐藏 HWND 冒烟在实际属性命令完成和样式恢复后读取全部操作按钮文本，核对当前标签；
原有不变定时器不触发重绘检查继续保留。该证据不替代屏幕阅读器人工验收，
也不表示工具选中状态的完整辅助功能语义已完成。

面板内容溢出现在由 `tool_panel_scroll.rs` 管理：窗口外框限制在显示器工作区内，
内容保留原 DPI 尺寸，双轴系统滚动条支持行/页/首尾/拖动滑块及滚轮。无需滚动时禁用
滚动条而不反复改变非客户区尺寸；偏移由 Win32 保存，无后台轮询。Tab 导航处理后将
焦点控件滚入客户区。隐藏窗口冒烟主动缩小面板，投递水平/垂直滚动消息，核验末尾按钮
完整进入客户区、返回首项及原窗口尺寸恢复；不注入全局输入或修改显示器配置。
真实多屏 DPI 切换、可见键盘焦点导航及滚轮手感仍需人工验收。

`tool_panel_wheel.rs` 按事件读取 Windows 滚动行数/字符数及按页设置，零行设置不滚动；
小于 120 的高精度滚轮输入保留双轴独立余数，不逐次丢弃。设置或 DPI 变化清空余数，
滚动条直接操作清空对应轴余数。横向按钮网格以 8 DIP 作为字符单位，纵向以 35 DIP
作为行单位；计算使用宽整数并限制输出范围。纯计算回归覆盖正反向细分输入、方向反转、
禁用滚动、按页及极端参数；测试不修改系统偏好，也不注入全局滚轮输入。

三个精确效果入口现已进入工具面板：新建模糊、选中模糊、色块大小。面板共 19 个工具
与 16 个操作按钮，可用性缓存改为固定 64 位并有编译期目录容量检查，不增加空闲定时器。
新建模糊仅在模糊工具下可用，色块大小仅在马赛克工具下可用，选中模糊要求实际选中了
单个模糊对象；任务忙碌时禁用，排队命令执行时再次检查。布局与文字度量测试按实际目录
数量运行，隐藏 HWND 冒烟验证第 32 位以上按钮的启用/禁用随工具变化，并确认选择工具下
没有模糊目标时按钮禁用。未通过冒烟点击会打开模态输入的按钮，也未进行可见 UI 人工验收。

效果笔刷与马赛克属性也已对照主干 `EffectPropertyFields.tsx` 修订：模糊/马赛克笔刷
精确输入及增减统一为 4–200，普通线宽仍为 1–256，橡皮擦仍为 4–96。
马赛克工具下“精确色块大小…”接受 2–64 小数，原增减操作已去掉 `u8` 截断。
输入只改当前工具样式，不自动修改既有标注；既有对象仍通过应用样式入口处理。
测试覆盖边界、非有限值、其他字段保留、小数进入新笔迹、撤销重做及工具往返后的配置恢复；
原生对话框覆盖小数接受和越界拒绝。这不是浏览器/原生逐像素渲染等价或完整属性面板验收。

选中单个模糊标注后，右键“选中模糊强度…”可直接输入 1–64 的小数强度。
`blur_selection.rs` 将原增量请求与绝对值输入统一到同一目标/源图校验；绝对值直接写入
请求，不经“目标减旧值再相加”的浮点运算，也不取整。`blur_numeric.rs` 在原生对话框
返回后检查文档身份、选择与原强度，拒绝模态期间已变更的目标，再复用现有后台任务。
输入非法值或当前已有值不提交；与栅格编辑等互斥任务冲突时不打开对话框。

回归覆盖分数值位级保持、范围/非有限值拒绝、跨文档/工具/选择与待处理强度变化，
并通过真实工作线程验证计算像素、提交、撤销恢复原分配及重做。原生数值对话框测试
追加小数接受与越界拒绝。新建工具现也支持小数默认值，见下；实时滑块预览和完整属性面板
仍未据此宣称完成。

选择模糊工具后，“新建模糊强度…”允许输入 2–64 小数，与主干 `EffectPropertyFields.tsx`
的范围一致。`ToolProfiles` 保留有限浮点强度；请求、准备结果缓存和落笔准入均比较实际
`strength()`，不再使用整数 `sigma()`。修改默认值只清理准备缓存并重启既有后台任务，
不修改已经提交的标注或历史。对话框返回时拒绝换图、换工具和默认值已改变的旧输入。
真实线程回归验证 2.4 准备后新建标注仍为 2.4，并专门验证 2.4 的缓存不能供 2.49 使用，
即使二者整数舍入相同；工具往返切换保留小数，非法值不覆盖原配置。

## Live 原生帧交付底层

Win32 消费者通知桥已实现：当前进程/线程 HWND 绑定，一次未确认通知合并多个帧，
消息不含像素/裸指针，连接 token 防止旧消息误作用于新订阅。隐藏窗口验证 100 帧
只排一条通知、读取最新帧、关闭、旧 token 和 HWND 已销毁的失败计数；回调重入和
失败代次竞争、槽回收有独立回归。专用 LiveWindow 已消费该桥，应用托盘入口已接入；通知失败
由后续帧重试及已有事件补读，最终回收还等待真实线程句柄，不能将消息投递当作必达协议。

Live 呈现控制器已串联订阅与 GPU surface：通知只替换最新待绘帧，paint 惰性创建并
提交，零尺寸暂停；已有宿主事件可补读遗漏通知。关闭/绘制错误释放订阅与帧 owner，
不循环重试。隐藏 HWND/WARP 测试覆盖实际呈现、旧帧释放和错误终止。LiveWindow 已接真实
消息分发、捕获排除设置/读回、WM_CLOSE 与错误关闭，保留错误且不退出应用；owner 搬移及
析构均覆盖。可见应用入口、捕获授权、关闭通知失败后的无事件补偿与 worker 完成回收
现已通过应用托盘/选区与线程消息合并等待接线。未运行真实桌面捕获或可见窗口验收。

Live 会话配对已实现先窗口绑定/捕获排除、后启动 worker；失败不启动采集，stop 不在调用
中 join，poll 单次回收。真实线程完成句柄与源关闭通知分离，合成 worker 回归验证关闭
通知先到时仍等待真实结束、保留后台错误、取消退出与配额回收。Application 已持有 Session，
无 Live 时使用 GetMessage，有会话时合并等待线程句柄与消息；隐藏应用回归覆盖失败回收，
等待回归覆盖无投递消息时线程唤醒。区域 Live 托盘入口已接选区确认，取消不改文档；
此项尚非 HWND 跟随、HDR Live、投射输入或 N07 整体完成，真实捕获和性能仍未验收。

HWND 捕获后端已加入：显式 Target 保留顶层 HWND/PID/TID 与原进程句柄，WGC
CreateForWindow 绑定原窗口，LiveJob 交付前后复核；移动不按旧屏幕坐标继续捕获，关闭
或身份变化停止。窗口 Session 启动仍先排除呈现窗口再创建采集线程。隐藏窗口测试验证
移动/跨线程绑定、关闭/身份改变、子窗口拒绝、无需求和绑定失败不启动 WGC。窗口选择
菜单已接入，自建窗口真实 WGC 已验证，最小化/隐藏完整策略及真实用户窗口验收未完成；同一进程线程内的 HWND 复用
启动竞态未消除，不能宣称 HWND 跟随用户功能或 N07 已完成。

托盘窗口 Live 命令 0x106c 已接原生菜单，最多 64 个外部可见窗口，排除当前进程、
最小化/cloaked 候选。标题有界并清理菜单控制字符，附 PID/HWND；菜单选择直接移交
Target 并复核，不按名称再次查找。取消/过期不排队；Region/Window 共用单个请求槽。
真实菜单对象和隐藏窗口回归验证选择、取消、过期、标题边界，应用回归拒绝已关闭窗口
请求；未枚举用户窗口/读取标题，也未实际点击菜单或启动真实 WGC。可见交互与源窗口
运行状态变化仍待验收。

窗口源尺寸变化已接 Recreate：归还变化帧、预留旧+新池峰值预算、成功后提交尺寸，
同设备继续交付；预算/重建失败关闭会话。零内容暂停交付并保留最后一帧，固定区域和
显示器源仍拒绝尺寸变化。事务回归覆盖失败/峰值/回收，真实 WARP 帧池多次重建不启动
捕获，GPU 测试覆盖不同源尺寸。显式自建窗口 WGC 缩放已验证，最小化/恢复与实际显存峰值仍未验证。

同设备原生呈现后端已实现：共享 BGRA8 帧通过 DXGI Surface 进入 D2D/DirectComposition，
重绘不重建位图，持有帧 owner 防止提前释放预算。隐藏窗口/WARP 测试读回验证红色与
半透明绿色像素、resize、同帧复用、替换释放，以及未保护设备/异设备/元数据错误拒绝。
测试不采集桌面，也不代表可见合成或真实跨线程捕获+呈现已通过；宿主事件/UI、HDR
色彩呈现和输入授权仍待接线或实现。

区域 Live 后端已接 `start_region`：物理选区绑定单显示器，复制前后复核目标身份与
布局，变化候选丢弃。GPU 使用 CopySubresourceRegion 只复制有界选区，WARP 测试
逐像素验证输出，并拒绝空、越界及溢出区域；过期候选测试验证不交付且资源释放。
整屏帧池仍存在，区域 API 不等同于窗口源或 UI 选区入口，真实桌面/呈现尚未验收。

持续 WGC 后端现由 `live_job.rs` 的后台任务 owner 驱动，最多 16 个同时存活任务。
返回初始消费者并支持共享订阅；取消关闭源唤醒 WGC，Drop 复用 CaptureJob join，
worker 错误/panic 会关闭所有消费者并释放名额。四项真实工作线程合成测试覆盖共享
交付、最后消费者退出、取消不报成功、panic/错误与名额复用，不采集桌面。启动/关闭
中的 OS/驱动调用仍可能不可中断；宿主 UI、呈现、窗口恢复和输入授权仍未接入。

持续显示器 WGC 后端已连接 GPU 快照与共享交付源，使用同一设备/两帧池和容量 1 的
事件通知，不反复创建截图任务，也不周期轮询等待帧。最后消费者退出、取消和源关闭
唤醒 owner；错误或关闭撤销事件并关闭资源。尺寸变化当前终止，不伪称已适配 resize。
合成通知/队列回归与无效输入准入通过；没有调用有效显示器启动捕获，真实 WGC、
宿主工作线程、窗口/区域选择、呈现和交互授权仍缺验收或接线。

`live_frames.rs` 已实现单最新帧槽和最多 16 个独立消费者，读取不消费其他订阅者的帧，
慢消费者跳帧而不排队。源关闭唤醒等待者并拒绝后续发布，最后一个消费者退出清空
源持有帧；已借出的资源不会提前销毁。`live_texture.rs` 将同设备 BGRA8/FP16 纹理
复制到独立 GPU 快照，禁止跨设备、不兼容格式/采样，无消费者不分配；预算随共享帧
持有。多消费者/关闭/容量/序号及 WARP 纹理复用隔离已验证，测试未采集真实桌面。
持续 WGC 后端已连接；原生呈现、UI 和输入授权未接入，也未证明跨设备同步或真实显存峰值。

## 原生托盘与快捷键

### 默认全局截图快捷键

`global_capture.rs` 已按主干 `shortcut_config.rs` 的默认值接入 Ctrl+1，通过
`RegisterHotKey` 和 `MOD_NOREPEAT` 接收宿主消息，不安装键盘 hook、不轮询键盘。
普通运行启动时尝试注册，冲突不终止应用、不抢占或替换其他程序注册；右键菜单的
“全局截图快捷键”勾选表示注册成功，可关闭或重试。注销失败保留 owner 状态，退出清理
仍尝试注销。模态禁用宿主、文件对话框或互斥编辑任务期间拒绝截图重入。
托盘中触发时先恢复宿主，再走同一个区域截图入口，使取消后仍有可见恢复位置。

`--smoke-test` 在注册代码前结束，不占用系统 Ctrl+1。单元测试通过替代注册函数检查
冲突、幂等、注销失败与重试，以及消息 ID/键码/修饰键和忙碌门禁；未触发真实系统快捷键
或读取桌面。Loom 自定义快捷键、Live/长截图快捷键、配置持久化仍未迁移，不能将固定默认值
视为完整快捷键配置合同已完成。

### 托盘生命周期

自启动入口现已接入：沿用主干 HKCU Run 的 Otter 值及 StartupApproved 禁用判定，
只有用户显式启用才写入当前 EXE 的带引号命令并恢复已有审批项。关闭只删除当前安装
的 Run 值，不删键或其它应用；其他 Otter 路径显示禁用项目，不自动接管主干安装。
读取失败、菜单期间状态变化和部分写入失败均报告或禁用，不伪称成功/回滚。此为读前
复核而非跨进程原子事务，不承诺抵御恶意并发注册表修改。测试使用内存替身，实际登录
启动、任务管理器交互和用户注册表写入尚未验收。区域 Live 入口已接入，接收投射入口仍待迁移。

托盘菜单现已增加区域截图、垂直/水平长截图和打开图片为新钉图，复用原生现有入口，
保留当前文档。菜单按任务/模态状态禁用项目；选择后先恢复宿主，再次复核状态后执行，
取消后仍有可见恢复位置。真实 HMENU 测试与注入执行回调验证命令、准入和恢复后重入
拒绝，不触发系统托盘或实际捕获。区域 Live 已新增为托盘命令 0x106b，复用选区确认并由
应用在回调外创建会话；窗口 Live 命令 0x106c 已接原生候选菜单，接收投射仍待迁移。
自启动实现及验证边界见上。

`desktop/tray.rs` 已接入右键“隐藏到托盘”。只有成功添加通知图标并设置版本 4 回调后
才隐藏主窗口及工具面板；隐藏时释放可重建的绘制资源，保留文档与历史。点击或键盘激活
通知图标恢复主窗口，按隐藏前状态恢复工具面板；托盘菜单提供恢复与退出。
恢复后删除临时图标，退出时再次清理；删除失败保留 owner 状态，下一次安装先重试删除，
不把失败伪报为资源已释放。收到 `TaskbarCreated` 后重建图标，失败则恢复主窗口，
避免留下没有恢复入口的隐藏应用。没有新增轮询线程或定时器。

窗口与托盘已使用仓库品牌图标，完整主干托盘菜单、设置及快捷键仍未迁移。
单元测试用替代通知接口覆盖添加失败、版本设置失败、删除失败后重试，以及版本 4 图标 ID
与键盘事件解码；自动测试不操作真实系统托盘或重启 Explorer。实际托盘点击、菜单、恢复、
Explorer 重启与可见资源变化尚待人工验收，不能将已有启动冒烟当作这些交互已通过。

`app_icon.rs` 内嵌现有 `src-tauri/icons/icon.ico`，校验有界 ICO 目录与各帧偏移后，
将所选帧复制到 DWORD 对齐的临时缓冲，再调用 Windows 解码。主窗口和工具面板绑定
16/32 像素图标，托盘借用主窗口的小图标，不新增路径依赖、第三方库或重复持久图像。
图标 owner 先于窗口创建、晚于窗口及托盘清理析构。测试覆盖全部截断前缀、非法偏移，
真实 Windows 解码 16/24/32/48/64/256 像素尺寸，以及隐藏宿主/面板的图标绑定。
混合 DPI 的可见图标质量仍未验证。EXE 文件资源现已另外接入，见下节。

## 原生 EXE 资源增量

`native/build.rs` 调用本机 Windows SDK `rc.exe` 编译 `resources/otter.rc`，把现有品牌 ICO
和 VERSIONINFO 链接入原生二进制。公开包版本从 Cargo 的 major/minor/patch 生成，
Windows 固定版本第四位为 0；不擅自推进公开版本，内部迭代仍由版本状态和 provenance 表示。
构建要求 Windows MSVC 工具链及 Windows SDK。可通过 `RC` 指定资源编译器的完整路径；
否则依次查找 `WindowsSdkVerBinPath`、标准 Windows Kits 安装目录的最高版本及 PATH。
所有生成文件归 Cargo `OUT_DIR`，没有新增第三方构建依赖或运行时图标文件要求。

独立资源集成测试以只读数据文件方式映射实际构建的 EXE，不运行入口，检查图标组资源
及 VS_VERSION_INFO 的文件/产品版本与 Cargo 一致。此验证不等于签名、便携包来源、
干净源码或正式发布门通过；当前 dirty 开发版本不得作为正式发行版发布。

## 运行中区域截图增量

### 系统捕获排除

右键菜单的捕获排除开关通过 `capture_protection.rs` 同时设置当前主窗口和工具面板，
对应主干 `set_overlay_capture_exclusion` 的原生窗口保护能力。默认关闭，仅在当前运行期间生效，
不读取或覆盖旧设置；持久化与完整覆盖层模式仍属于后续工作。
任一窗口设置失败时回滚已成功窗口；回滚本身失败则保留部分状态并在菜单中明确显示，
后续点击仅补设缺失窗口。没有新增定时器、线程或图像缓冲。

实现使用 `WDA_EXCLUDEFROMCAPTURE`。Microsoft 明确其在 Windows 10 2004 起支持，旧版本
按 `WDA_MONITOR` 处理，且它不是绝对防录屏或 DRM 安全边界：
[SetWindowDisplayAffinity](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowdisplayaffinity)。
单元测试注入首次失败、第二窗口失败与回滚失败，验证准确状态及重试。
普通窗口/钉图启动冒烟对隐藏宿主和面板真实调用启用、关闭 API 并检查成功返回；
没有捕获桌面，因此不是截图像素排除效果或第三方录屏兼容性的验证。

### 全屏与区域入口

工具面板和 Ctrl+1 默认区域截图生成新钉图；右键菜单另保留明确命名的区域/全屏替换项。
右键菜单也提供“全屏截图为新钉图（所有显示器）”，保留源文档及其编辑历史。
全屏模式直接使用虚拟桌面冻结帧，不创建区域选择器、不枚举窗口。
`capture_input.rs` 仅在替换命令中确认丢弃原编辑，记录应用宿主与工具面板的可见状态，
隐藏后等待 DWM，再获取一次冻结帧；仅区域模式进入选区交互。
取消或失败通过 RAII 恢复原可见窗口；本来隐藏的窗口不被显示，销毁的窗口不恢复。
确认选区后校验文档身份，复用 `open_job.rs` 的后台图像准备和版本校验提交链。
截图获取本身仍是同步的一次性调用，不是后台连续捕获；新钉图与替换结果分别提交，
不代表多对象工作区、滚动截图或 HDR 已完成。

内存夹具测试覆盖图像共享存储、取消后引用释放；隐藏 HWND 测试覆盖窗口守卫清理。
`capture_route.rs` 的策略测试覆盖四种命令、全屏单次获取且跳过枚举/选区、负屏幕原点
和像素共享，以及区域取消和捕获失败；它们使用注入夹具，不读取真实桌面。
启动冒烟覆盖 19 个工具与 16 个操作按钮的创建及已有操作链，不触发真实桌面截图。
`capture_input_smoke.rs` 通过现有图片注入同一入口事务，覆盖取消、失败、嵌套换图后的
过期拒绝、真实加载线程和 WM_TIMER 提交，以及原本隐藏的宿主/面板不被显示；
普通窗口与钉图启动测试均强制检查此标记。它不证明真实桌面获取或区域选择器交互。
真实桌面捕获、选择器与宿主隐藏/恢复的完整可见交互仍需人工验收。

| 组 | 现有实现入口 | 对照测试或原生验收 | 新实现状态 |
| --- | --- | --- | --- |
| 宿主、托盘、快捷键、输入恢复 | `src-tauri/src/native/runtime/`、`native/input/`、`native/overlay/` | `docs/FEATURES.md` 的系统对话框、紧急退出与安全桌面矩阵 | 独立普通窗口；完整宿主未迁移 |
| 图片显示与变换 | `src/features/workspace/components/UnitView.tsx`、`hooks/useDraggable.ts` | 多屏、混合 DPI、缩放锚点、拖动与原生文件拖出 | WIC/D2D 单图显示、画布平移缩放；非等价钉图 |
| 标注与历史 | `src/features/stickers/types/stickerEditing.ts`、`services/stickerHistory.ts`、`services/stickerCompositeRenderer.ts` | `tests/unit/stickerHistory.test.ts`、`stickerAnnotationPersistence.test.ts`、`stickerBlurEraseHandoff.test.tsx`、`stickerPaintbrushInteraction.test.tsx`、`stickerBezierInteraction.test.tsx`、`stickerTextReeditInteraction.test.tsx` | 基础画笔、直线、矩形、椭圆及单手势撤销已接入；完整工具和格式兼容未完成 |
| 截图与长截图 | `src/features/capture/`、`src-tauri/src/native/capture/`、`native/long_capture/` | `tests/integration/LongCaptureSessionContract.test.ts`、HDR/捕获排除/四方向原生矩阵 | 未迁移 |
| Live 与输入 | `src/features/live/`、`src-tauri/src/native/live/` | `src-tauri/src/native/tests/live_capture_sessions_tests.rs`、真实 GPU 呈现及源窗口恢复 | 未迁移 |
| 工作区与持久化 | `src/features/workspace/types/unit.ts`、`services/syncService.ts`、`src-tauri/src/native/session/session_commands.rs` | `tests/unit/sessionStickerMapping.test.ts`、修订冲突/资产缺失/重启恢复 | 未迁移；新宿主不读写旧数据 |
| Art/Shader | `src/features/art/components/ShaderRenderer.ts`、`services/shaderCache.ts` | `tests/integration/shaderCache.browser.test.ts`、颜色/alpha/uniform/坐标及资源预算 | 未迁移；GLSL/WebGL2 合同待适配 |
| Surface | `src/features/surfaces/components/JavaScriptSurface.tsx`、`DeclarativeSurface.tsx` | `tests/integration/JavaScriptSurfaceCspContract.test.ts`、`ArtSurfaceInteractionContract.test.ts`、`tests/unit/DeclarativeSurfaceContinuity.test.tsx` | 未迁移；DOM 与浏览器沙箱语义待拆分 |
| 插件、OCR/翻译 | `src/features/extensions/`、`services/extensionNativeTransport.ts`、`services/extensionLifecycle.ts` | `tests/unit/extensionAuthorization.test.ts`、`ocrTranslationVisibility.test.tsx`、原生授权链 | 未迁移；不能只移植命令传输 |
| 投送与 journal | `src/features/projection/`、`services/projectionEditJournal.ts`、`services/projectionCreateJournal.ts` | `tests/unit/projectionReceivePolicyPersistence.test.ts`、两机 TLS/配对/取消恢复 | 未迁移 |
| Tile 与屏幕墙 | `src/features/tile/services/tileSurfaceController.ts` | `tests/unit/TileSurfaceController.test.ts`、`TileSurfaceIsolation.test.ts`、`TileSurfaceInputQueue.test.ts`、`scripts/tests/Invoke-TileWallNativeInputProbe.ps1`、`Invoke-TileWallArtProbe.ps1` | 未迁移 |

## 文件拖入增量

`native/src/desktop/file_drop.rs` 已接入普通/钉图窗口的 1–32 文件 Shell 批量拖入，复用后台解码并创建独立钉图，不再确认丢弃或替换源文档。`file_drop_tests.rs` 验证有界 Unicode 路径和批次拒绝；`file_drop_smoke.rs` 与启动测试验证实际 HWND 消息、工作线程、定时器和源文档保持；`application_drop_smoke.rs` 进一步验证应用集合接收候选、创建独立钉图和单独关闭不影响源文档。测试只投递到本进程隐藏窗口，不操作 Explorer 或系统输入。一个后台线程顺序准备全部图片，任一解码失败或取消均不发布半批结果。应用集合先检查窗口容量并准备全部隐藏宿主/面板，再统一接收；第二个窗口失败的注入测试验证第一个暂存宿主和面板被释放、集合与截图窗口表不变。超出 32 文件的批次明确拒绝；当前仍不是主干工作区对象与布局，文件夹、URL、虚拟文件、Explorer 手动拖放和权限等级组合仍未验收。不能据此将“工作区与持久化”组标为完成。

## 图片复制增量

原生 `Ctrl+C`/右键菜单已接入单任务图片复制，同一次合成生成 PNG 与 CF_DIBV5；真实 WIC 编解码回归覆盖裁剪、标注、透明度、位图行方向与非预乘颜色，取消及文档替换拒绝过期发布。基线 `src-tauri/src/native/image_io/clipboard_commands.rs` 的智能文件列表、命名和缓存策略尚未迁移；实际多格式系统发布及第三方应用粘贴未验收，不把这一增量视作智能剪贴板完成。

复制宿主另有 `copy_smoke.rs` 与普通/钉图启动回归，验证真实 HWND 下单次交付、Esc 取消、同版本换文档拒绝和任务回收。最终发布替换为测试回调，显式调用共用轮询而不泵复制定时器消息，保证测试不访问系统剪贴板；不能据此关闭真实发布和第三方粘贴验收项。

图片粘贴入口优先 PNG，再选择 CF_DIBV5/CF_DIB，使用有界快照与既有后台加载链；默认创建独立钉图，显式替换菜单才替换源文档。位图已支持 1/4/8bpp 未压缩索引色、16/24/32bpp 的未压缩 RGB/位域布局及顶向下/底向上转换，内存夹具验证透明度、掩码、padding、截断/配置文件拒绝、逐行取消及本分支 DIBV5 复制回读；不触碰真实剪贴板。索引色另有默认/缩减调色板、高位优先、行 padding、保留 alpha 忽略及越界索引拒绝回归。RLE4/RLE8 已接入直接写入预算内输出的解码器，覆盖重复/绝对模式、字对齐、EOL/EOB、Delta、行方向、截断和越界拒绝、指令间取消；未覆盖区域确定为调色板零色。实际第三方 RLE 剪贴板来源仍待验收。Unicode/ANSI CF_HDROP 的 1–32 文件批量粘贴已接入现有后台文件加载链，且仅在没有 PNG/DIB 图片格式时选择。解析限制输入大小、路径长度和双 NUL 结束符，拒绝超限批次和截断数据；显式替换入口拒绝多文件列表，不静默丢弃其余路径；ANSI 按系统代码页严格转换，非法多字节序列报错，不替换字符后继续打开；锁内只复制路径，不探测文件。内存夹具覆盖 Unicode/emoji、截断、非法偏移和批次拒绝，真实 Explorer 剪贴板仍待验收。ANSI 回归使用系统代码页的 ASCII 夹具和显式 CP936/1252 转换夹具，不改变系统区域设置。ICC 变换、文本/URL 和多对象工作区粘贴仍未迁移。

后台图片加载的定时器创建失败分支已改为取消并回收任务，返回错误而不替换原文档；`open_job_tests.rs` 通过调度结果注入覆盖运行中任务和已完成结果的释放。WIC/文件系统调用仍不保证可中断，回收线程可能等待当前调用返回，不据此宣称退出延迟已有上界。

## 原生工具面板增量

`tool_panel.rs` 已接入独立 Win32 工具窗口和共享目录中的 19 个原生按钮，默认随宿主显示，F4/关闭隐藏后可恢复，工具按钮通过消息路由复用命令执行入口；模糊仍进入异步准备。按钮以深色控件、信号黄及勾选文字表示当前工具，提供原生 Tab/Space 导航入口和 DPI 布局；没有新增空闲定时器。工具状态在宿主消息处理完成后比较，移除无差别键盘/定时器刷新；窗口冒烟清理更新区域后连续发送 8 次无状态变化的定时器消息，验证面板和按钮均未产生新更新区域，并验证工具变化在消息返回后更新缓存。启动冒烟验证 19 个控件、BM_CLICK 到宿主工具切换以及 owner 销毁时面板销毁。面板初始位置已按 owner 所在显示器工作区优先下方/其次上方放置，F4 恢复及 DPI 建议位置应用时校正越界；纯坐标回归覆盖负屏幕原点、任务栏可用区域、离屏矩形、稳定位置和超小工作区。超小工作区下窗口尺寸现限制到工作区，通过双轴滚动访问剩余按钮，不要求所有按钮同时显示。真实可见布局、跨屏 DPI、屏幕阅读器、遮挡/跟随定位和完整属性面板尚未验收，不将此视为完整工具栏完成。

面板新增 10 个属性/文档按钮，复用颜色/填充色对话框、粗细调整、透明度循环、填充开关、应用样式、打开、复制、导出命令；粗细/透明度/填充标签从当前 StrokeStyle 生成，样式改变才刷新。普通/钉图 HWND 冒烟通过真实 BM_CLICK 和宿主消息验证粗细、透明度、填充及状态标签，不点击真实剪贴板、文件对话框或颜色对话框。这里只是常用命令入口，粗细和透明度的精确数值输入已接入菜单与面板（面板操作按钮增至 16 个），支持小数，拒绝非有限数值及越界值；普通粗细 1–256、橡皮擦粗细 4–96、透明度 0–100%。`numeric_style.rs` 保留其他样式字段，模态返回后拒绝文档/工具/原样式改变的结果，取消不修改状态；真实 DialogBox 夹具验证小数、范围和错误输入留在对话框后取消，未注入系统键鼠或访问剪贴板。其他完整工具专属属性仍需继续迁移。面板已按无图片、无选择目标、打开/编辑任务及复制/导出任务分别禁用按钮；位掩码只在状态改变时更新原生控件，执行前复核可用性，禁用不只靠颜色表达。隐藏 HWND 回归通过模态门禁夹具验证禁用、排队命令拒绝及恢复；可用性模型回归覆盖空文档、编辑互斥和独立输出任务。颜色对话框复用模态门禁，返回时检查文档/工具/样式，真实颜色对话框重入行为仍待人工验收。

工具面板字体已从固定 DEFAULT_GUI_FONT 改为按 DPI 缩放的 Microsoft YaHei UI 字体，窗口属性持有一个句柄，绘制只借用；同目标高度不重建，DPI 变化先创建新字体再替换，WM_NCDESTROY 清理。真实 GDI 句柄回归验证 96/192 DPI 字体高度、同 DPI 复用、旧句柄失效及幂等释放；缩放取整覆盖 144/240 DPI。工具面板布局已统一为 96 DIP 按钮宽度、按目录数量推导行数，工具和操作从整行分组。布局回归覆盖 96/120/144/192/240/288 DPI 的边界及无重叠；真实 GDI 度量覆盖 96/144/192 DPI 全部工具选中标签、操作标签及粗细/透明度动态边界值，均能容纳于按钮内容区。粗细按钮显示两位小数，精确输入和底层样式保留原精度。实际跨显示器字形与视觉一致性仍待验收。

## 菜单目标生命周期

`context_menu.rs` 的弹出菜单绑定打开时的文档身份、修订号、工具、选中集合、当前样式及裁剪模式。`TrackPopupMenu` 嵌套消息循环返回后重新校验，状态改变则丢弃命令，避免后台加载替换文档后误删或改写新目标。回归覆盖共享像素/相同修订的新文档、撤销、切换工具/选中集合、无修订样式和裁剪模式变化。真实弹出菜单期间异步交付的人工交互仍待验收。

## 数据兼容审计

- `SessionSticker` 是展开的持久化格式，不等于运行时 `Unit`；必须保留标注、图像编辑、Art、附件和投送字段。
- `session_commands.rs` 的修订校验、进程间租约和 `images/` 资产引用必须保留，不能用普通 JSON 覆盖取代。
- 原生后端当前将 annotation 作为 JSON value 保存，不代表已有完整 Rust 编辑模型可直接复用。
- `syncService.ts` 当前对未知字段宽容读取但忽略；新写入路径必须单独证明未知字段保留，不能沿用“可加载即不丢数据”的假设。
- 双向编辑 journal 与投送创建 outbox 位于 IndexedDB；接收策略、delivery、receiver、QR cleanup 还有 localStorage 状态。
- 待补：工具设置、历史、凭据、设备身份完整读写路径，格式夹具和事务中断用例。

## 浏览器能力适配边界

- 自然画笔通过 brush 的 tapered tip 和 weights 表示，不能仅按 annotation 类型枚举迁移。
- JavaScript Surface 的 iframe、MessageChannel、焦点、输入和 watchdog 属于可观察合同；单独 JS 引擎不提供 DOM/CSS 等价性。
- 声明式 Surface 还包含草稿保留、滚动、文本选择与防误触；控件类型对应不是完整兼容证明。
- Shader 接收 WebGL2 shader 源码和纹理；原生渲染需验证语言与像素语义，不允许无声禁用预览。
- 扩展权限必须在读取字节和执行命令时检查实时授权；声明权限不能直接变成授权。

## 增量实现证据

标注组的当前细分状态以迁移计划为准，上表的初始核对不代表最新工具清单。
选择工具已接入 Shift+点击增减多选、多个选中框、批量删除及选中集合后台栅格化；
`native/src/desktop/multi_selection.rs` 与 `rasterize_input_tests.rs` 覆盖选择版本失效、
批量删除和整组栅格化的一次历史提交及撤销。组拖动已接入原子提交与取消，
`effect_preview_tests.rs` 验证效果/荧光笔组合的预览及提交像素一致。组旋转的 15° 快捷键/菜单
已接入视觉组中心和单次历史；组等比缩放也已接入快捷键/菜单及 DirectWrite 文字重排。
组非等比缩放已接入菜单并区分路径图像轴和形状局部轴语义；模糊缩放已接入后台 strength 重算和整组提交，连续强度精度与完整源图语义仍待对照。
自身中心模式已接入窗口菜单与旋转/缩放事务；连续旋转柄已接入基础预览/提交/取消链。
旋转轴模式已接入独立菜单设置；鼠标缩放柄已接入预览、Shift 等比及松开候选交付。
缩放轴模式已接入独立菜单，Shift 等比约束优先；模糊缩放已保留松手后的几何预览直至后台交付，实时预览强度及完整批量属性仍未完成。`marquee.rs` 已接入选择工具空白处拖框：按视觉包围盒完整包含选中、双向拖动、缩放坐标转换、3 DIP 阈值、松手交付、取消及文档身份失效；复用组删除/移动等既有命令，框选不创建历史。活动框选加入后台结果接受和滚轮互斥检查。Shift 空白拖动已接入追加框选，保留原选择并按索引去重，取消/空白点击不清空原选择，文档版本失效时不应用旧集合；Shift 命中点击仍保持单对象切换语义。回归覆盖追加、去重、取消、滚轮互斥和旧索引失效。`marquee_smoke.rs` 与普通/钉图启动测试进一步覆盖真实 HWND 鼠标消息、Shift 线程键盘状态、捕获释放、选框绘制、滚轮互斥、Esc 保留原选择及组删除撤销；修复了窗口 Esc 分支先清空原选择的问题。测试恢复原文档和线程键盘表，不注入系统输入。该增量尚未完成与主干所有框选语义和真实键鼠视觉的对照，
实际键鼠交互和大文档性能未验收，不能将此增量作为完整多选对齐证明。

后续每组增加原生 owner 与可重跑对照证据后再更新状态。未知或未测项目保持未完成。
