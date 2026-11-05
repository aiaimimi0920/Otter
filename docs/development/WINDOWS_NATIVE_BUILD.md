# Windows 原生开发构建

## v545 迁移收口与本地手测交付

用户已认可迁移基本完成并要求停止细化，授权提交推送当前分支后生成本地完整手测包，由其自行使用反馈。本轮仅执行提交、来源核对和已有官方候选／归档链，不追加功能、环境实验或资源采样；沿用v0.2.33.545，Release配置内部包不等于公开发行或全部延期门通过。提交前已有的固定包和失败回执保留；本次包的确切提交及SHA以新包内来源文件和本地交付回执为准。

## v545 普通菜单退出与自然回收通过

用户按新测试贴图右键菜单“退出 Otter”指引操作后反馈“已退出”。固定SHA的v545完整包解包普通EXE、零产品参数、全新profile及自有静态图片工作区，由用户实际操作，运行器没有UI输入。主进程exitCode0；启动后103.672秒观测到退出（这是等待人工操作的总时长，不是退出耗时），包含根的4个自有进程在原10秒排空门内自然归零，监听为空，Job强制兜底为false；保存后的图片、身份及revision保留。普通静态工作区的正式菜单退出和自然回收子项关闭；菜单路径依据本次人工完成反馈，不冒充自动UI逐步轨迹。

证据 `main-sticker-menu-fix-1/input2/ordinary-retest-2/{source-adapter.json,owner.json,started.json,lifecycle.jsonl,result.json,exit-audit.json}`；原result不改写。前次复测用户明确表示忘记操作，归为未执行人工动作，其超时和Job兜底回执保留，不判产品缺陷；更早不一致按用户要求不再追查。本次产品源码、版本、构建及完整包均未变化，ZIP与EXE摘要仍为下节v545值，无需重建或重封。

范围只覆盖普通静态工作区；不推定活动Art/Live退出、未保存选择、资源/性能长稳或物理环境矩阵通过。A/B/C和中文IME保持用户通过，资源仍按用户要求后置，Canvas暂缓非阻塞，P4缺条件延期；不重测本项或已过Loom关闭，不称总迁移100%。以下旧“退出未验”表述是当时状态，以本节为准。

## v545 贴图鼠标右键菜单消息修复

用户确认v544测试窗口可见，但贴图鼠标右键不弹菜单，不能继续归为“未点退出”。无边框钉图在Pan模式的WM_NCHITTEST返回HTCAPTION；自有隐藏WS_POPUP实验确认客户区右键会生成WM_CONTEXTMENU，而标题栏右键WM_NCRBUTTONUP不会，原宿主只监听前者。procedure现仅将pinned且HTCAPTION的右键松开转发到原WM_CONTEXTMENU，完整保留屏幕坐标；先释放Canvas借用再同步重入，不改样式、菜单、捕获保护或退出入口。v544两次退出原回执保留，第二次显式停止且自有Job归零，不是产品崩溃或正常退出通过。

三个新增真实隐藏HWND回归保护标题栏松开恰一次／负屏幕坐标／文档不变、客户区及键盘菜单不重复、非pinned标题栏不误分流，并在菜单边界实际取得可变Canvas借用。定向库共59项通过、工作区6项及归档拒绝合同9项通过；不是整库或普通前台菜单手测。fmt、版本、strict行数、strict all-targets Clippy、测试库、官方candidate来源与完整归档核验通过。首轮仅新测试误用DocumentStamp的assert_eq而编译失败；input2改用原matches身份与revision比较，生产修复未变，未重分版本或放宽断言。

当前完整内部包`Otter-native-v0.2.33.545-windows-x64-internal.zip`，ZIP SHA-256 `F3173811EAB693894600A65304965C4918927FDEB5A2705E3968994B409220AE`；解包普通EXE SHA-256 `9FFC2BA548A3019BAF1B04F50C73C0F036AC420616A8FD1293F23DF808859B92`。包含v544 Live修复及本次菜单修复，来源为当前dirty工作树与一次分配的v0.2.33.545；未签名、非正式发布，不提交推送部署或替换用户安装。证据目录main-sticker-menu-fix-1/input2。

开发侧消息路由已验证，普通前台右键显示与正式退出仍须在新完整包实测，不能由隐藏回归推定通过。A/B/C与中文IME用户通过保持；Canvas按用户决定暂缓非阻塞，资源统一后置不采样、不记达标，P4缺设备环境保持延期，不扩大迁移范围。

## v544 资源后置与用户菜单退出观测

用户要求继续非资源的开发侧验证，CPU/内存/GPU/物理磁盘和资源/性能长稳留待后续功能开发后统一检查；不记资源达标，不据此跳过功能、数据或进程回收。本轮无产品源码、版本、编译或封包变化。

用户同意在新测试实例手动选择一次正式“退出 Otter”，因此不重试旧Computer Use菜单绑定。既有Owned Job链启动固定SHA完整包解包普通EXE，零产品参数、全新profile，读取从旧自有测试profile复制的正常保存工作区；不是测试Application或注入退出状态。开发侧只观测PID、根退出、整树、监听及工作区，不做资源采样、Computer Use或替代输入；未重测Loom关闭。

600秒内未观测到根退出，原失败为 `AssertionError('Human ordinary exit deadline')`，finally前active4、由自有Job兜底后归零、产物摘要不变。没有用户实际看到/点击的回执，不能判定其未操作或产品无响应，也不能记自然退出通过；已集中询问具体到达步骤，未确认条件前不重跑。证据 `main-normal-exit-544-2/{scope-decision.json,owner.json,started.json,lifecycle.jsonl,result.json,exit-audit.json}`。普通退出仍开放，资源延期不是通过，A/B/C及中文IME用户通过和Canvas暂缓决定保持。

## v544 控件树范围的普通退出补验

用户明确允许在截图不可靠时，仅通过现有Computer Use控件树/键盘菜单验证全新profile自有窗口正常退出；不包含截图坐标、Live或其他输入API。本批复用固定SHA的完整包解包Otter、普通Loom壳/daemon及原Owned Job运行器，新建双profile和真实bridge，无生产/版本/构建/ZIP变化。

普通Loom首次控件树点击因 `coordinate input geometry is unavailable` 被工具拒绝；不重试点击。重新枚举确认唯一同路径窗口及正常控件树后，使用原API的Alt+F4。主进程exitCode0，运行器384.953秒观察到root退出，386.953秒样本整树active0，清理前监听为空、Loom无Job兜底。相同壳构建输入的 `runtime/app.rs` / `runtime/daemon.rs` 摘要已核：CloseRequested调用begin_desktop_exit，停止订阅并对仍存活的自有daemon执行Child.kill/Child.wait后exit0。因此这是普通壳正常关闭及所属树回收，不是daemon自愿退出。

普通Otter通过Alt+Space及Shift+F10取得系统/业务菜单树，但未取得“退出 Otter”项。业务菜单只返回在宿主树下；方向键后菜单消失，列表仍无独立菜单目标。停止该分支，不猜句柄、改样式/保护或换输入API；Otter最终active4由自有Job兜底归零，不能算Otter或双产品正常退出。独立精确EXE路径核两产品/daemon无残留。没有截图/坐标输入、Live验证或用户已有窗口操作；工具/运行器停止失败原回执保留，不作为产品崩溃证据。

证据 `main-tree-exit-544-1`：result.json、exit-audit.json、loom-exit-before-fallback.json、ui-audit.json及process-closure.json。仅关闭P2的普通Loom关闭/树回收子项；普通C、Otter退出、完整普通业务/资源/长稳、Canvas历史250ms及P4仍开放。不将本轮含UI查询的样本用作资源验收，不复测已过Loom关闭，不重编或封包。

## v544 有限收口审查与普通退出工具阻塞

复核实际分支、revision544、最新终态和无在途后，继承A/B用户通过及已封存WGC修复，不重跑旧门。普通完整包/EXE摘要保持下文v544值。全新双profile启动该解包普通Otter、固定普通Loom壳/daemon及真实bridge，尝试独立补齐正常退出。Computer Use仅枚举及观察一次；自有Otter标题/控件树与截图内容不一致，四象限夹具未在截图出现，未激活/点击/发送按键。立即停止，不重复捕获、Ctrl+2或更换输入API；两树Job兜底归零，独立精确EXE路径核无残留，不计普通退出、C、资源或长稳通过。共享helper未动，原失败保留。

Canvas仅做一次有界只读审查：原日志摘要匹配，CanvasDraw为519705µs，返回后原250ms准入失败；当前相关5份源码与v543诊断输入一致。服务快照在dispatch之前，350ms外部等待不是该次draw超时的解释；原脚本为2×1和两次填充，不是已证的大图负载。软件target首次初始化仍是合理假设，但原失败没有target细分、线程CPU或调度记录；后续59.548ms/55.258ms样本不能唯一归因或关闭原失败。产品影响是合法小Canvas可能被超时拒绝，返回错误而非成功帧；发生频率与唯一根因未知。250ms、heap/Job、取消/回收及父期限不变，不移动计时、不预热绕门、不重跑刷绿。调查到此停止，门禁仍开放；只有新真实失败证据/具体可证伪机制才继续，风险接受须用户明确决定，未自行放行。

只读环境枚举为1个2560×1440逻辑显示器，存在中文/英文layout不代表人工IME候选、混合DPI或物理热插拔已验。P1–P4的有限清单已登记修订计划，证据main-delivery-close-544-1。没有生产源码/依赖/版本/构建/ZIP变化、没有新增Rust通过数；本批仅文档和临时调查产物。资源合同仍要求受控基线后确定数值预算，不把既有Private峰值当合格线。普通C、完整普通业务/资源/自然退出/长稳、Canvas原失败及物理/跨机/人工验收继续阻止最终完成。

## v544 真实 WGC 与普通 Loom 持续 Art 负载

新增一个显式运行门，以自有白色窗口的真实WGC帧，经普通Loom壳/daemon和回环echo Art完成600秒持续业务。120次交替128×96/160×112尺寸及显式执行，加上一次初始自动执行，共121次上游调用；逐轮核正式图像尺寸、全部白色像素、准备/执行worker回收、Live仍存活且源控制关闭，结束核源样式/可见性/正常位置恢复。最终1 passed/0 failed/0 ignored，不是121个测试。测试树自然归零、无Job兜底；普通Loom树仍由自有Job回收，其从未切换的私有desktop已关闭。

只有cfg(test)接线和新运行叶；版本544、普通EXE和完整ZIP字节不变，不重跑旧成功批次或封包。首轮新测试字段访问错误、两轮恢复child夹具启动前置失败、echo参数层级及正式端口数量断言原失败均保留。测试库不具普通main私有入口，既有fixture预建journal又须先登记再poll，后续复用原fixture启动方式并先调用原start_live_window/admit_live；echo按当前扁平image字段读取，正式结果按现有output/output_image双端口及共享像素所有权断言，完整白色像素要求保持。未改生产恢复/取消/期限或协议，未增加兼容分支。最终strict静态/测试库及文档合同通过，证据main-live-art-load-544-1。

这是WGC+普通Loom+自有回环Art的有界持续链，不是普通Otter/普通C前台全链、完整业务长稳或资源达标。资源包括测试消息泵，GPU/物理磁盘和整段外部UIA条件未核实，不能与普通产品空闲占用直接比较。A/B人工通过保持，普通C、Canvas原250ms、普通可见资源/完整联合长稳、物理多屏跨机混合DPI/人工IME和全面验收仍开放。

## v544 Live 连续生命周期运行门

新增一个显式运行门，复用现有自建源／Live HWND、真实 WGC、GPU 像素读回和恢复 child，完成 popup／带标题栏两种窗口、保存可见选区后源被最小化／保持可见两种启动边界，每种连续三轮，共 **12 个完整会话、36 次尺寸变化、24 次源控制启停**。每轮核新尺寸及单调帧序号、黑白像素、点击反馈、按住键在失焦后释放、输入 worker 回收，以及停止后的原样式／可见性／正常位置恢复；原本最小化的源按既有合同展开。消息仅发送至自有测试 Live HWND，没有全局输入、前台激活、用户应用或系统设置操作。

最终新增运行门 **1 passed / 0 failed / 0 ignored**，测试体 7.94 秒；这是有界循环回归，不是长稳、普通产品 C 全流程或资源达标。自有测试树自然归零、无 Job 强制兜底，独立精确 EXE 路径检查无残留，不代普通产品自然退出。input1 的测试先最小化、再读可见外框，被既有 `E_ABORT: Live source minimized before placement` 正确拒绝；input2 仅把选区读取移到最小化前，保持 Application 的原入口顺序，未改产品拒绝逻辑、像素／清理断言或阶段期限，原失败保留。

只有两份 cfg(test) Rust delta：既有输入运行门增加子模块接线，新 `composition_live_cycles_runtime_tests` 叶拥有连续场景；官方 fmt、全仓 strict 行数、strict all-targets Clippy 和测试库构建通过，无新行数例外。版本仍 v0.2.33.544，普通 EXE／完整 ZIP 字节不变，不重跑已过的 43 项或封包。证据 `main-live-cycles-544-1`；A/B 人工通过保持，普通 C、Canvas 原 250ms、普通可见资源／完整联合长稳、物理多屏跨机混合 DPI／人工 IME 及全面验收仍开放，总迁移未完成。

## v544 私有 desktop 双产品空闲观测

复用最新完整包 v544 普通 Otter 与已验普通 Loom 壳／daemon 的固定 SHA，全新双 profile、静态合成图片、空能力目录及真实 bridge 订阅。现成隔离运行器创建从未切换的私有 desktop，不申请 SWITCHDESKTOP；启动器只读取自身线程 desktop 名，并在各自 suspended→Job→resume 的 STARTUPINFO 显式指定同一 desktop。没有 UI 查询、输入、用户窗口／显示／输入法设置变更，也未停止共享 helper。

一次完整 600.016 秒／301 点观测，排除首60秒后的样本区间单核 CPU：Otter 0.287%、Loom 整树 3.241%，合计 Private 峰值 359.215MiB。两个自有主进程各一次30秒 PID／545、546事件过滤的 UIAutomationCore ETW 均0事件，start／enable／stop及lost均0；这仅证明所测主进程的有限窗口，不扩为全部子进程整段无查询，更不证明所有历史CPU原因。

首轮跨进程 desktop 只读探针失败，采样尚未开始；input2在114秒后遇到PDH状态0xC0000BBA，Windows SDK明确为PDH_CSTATUS_INVALID_DATA，采样器中止时普通产品仍存活。两轮原失败及Job回收记录保留。input3只将CPU／内存／句柄观测与GPU计数器解耦，GPU明确未采集／未验证，不填零、不拼接旧采样、不降低产品预算；WebView重解析目录仍不遍历。进程I/O不是物理磁盘，Working set可能重复共享页。

最终两树由自有Job兜底归零，独立精确进程／线程和专属ETW session检查无残留；不是自然退出。证据main-private-joint-544-1及input3原回执。本批只补齐隔离条件下的有限双产品空闲测量，不代普通可见资源基线、Art／Live业务负载、GPU／物理磁盘或联合长稳。A/B人工通过保持，普通C、Canvas原250ms、物理环境／人工IME及全面验收仍开放。无产品源码／依赖／版本／构建／ZIP变化，完整包及摘要仍为前文v544，不做兼容维护或重开冻结副线。

## v544 最新完整内部包包含 Live 停帧修复

沿用 v0.2.33.544，在既有独占 target 通过官方 candidate、来源核验、许可证材料 staging、完整 ZIP 和独立归档核验。没有源码／依赖／版本新增修改，没有把裸 EXE 塞入旧包或改写旧来源。官方契约要求显式目标构建，故执行一次 candidate 增量构建；不重复未变化的 43 项 Live 定向回归，也不重编工作区 driver。

新完整内部归档为 `Otter-native-v0.2.33.544-windows-x64-internal.zip`，SHA-256 `EE728AE306AB04C219C77AAC30DE28B4E37EE232B5543AAD4ABA63768042F4C6`；解包 EXE SHA-256 `EACBDAA7AFCA09C70A93BD27F729433743A0EA9E81AF942FC7C7FC960C025649`。该包已包含前文 v544 静止源尺寸变化修复，取代 v543 作为最新完整内部包。归档拒绝合同 9 项通过，解包 EXE 的原工作区 6 项通过；不混报为新增产品功能或整库测试。

全新 profile、自有四象限图的解包普通窗口已由 Computer Use 绑定实际路径并观察。正常退出=false，Job 兜底=true，最终自有 Job 归零且独立精确 EXE 路径检查无残留；有限启动不等于 Live 全流程、完整业务自然退出或资源达标。

A/B 人工通过保持，C 原失败与普通前台完整流程待验边界保持。当前可绑定窗口不含托盘／Live 选区，不重复相同 Ctrl+2 失败或改产品入口绕过；未操作用户已有应用。Canvas 首次背板创建在原 250ms 墙钟期限内，现有旧失败缺细分证据，不移动计时边界、不增大期限、不凭后续成功关闭原 519ms 失败。干净资源基线／联合长稳、物理多屏／跨机／混合 DPI、人工 IME 与全面验收仍开放。dirty、未签名、非正式发布，不提交推送或安装替换；以下旧 ZIP 状态仅历史。

## v544 静止源尺寸变化后的 Live 停帧修复

用户合并手测确认 A（公共 Art capsule）和 B（跨节点键盘）通过、C（普通 Live）失败，并要求自动定位。A/B 保留人工通过，不要求重测；C 原失败保留，以下原生运行门不等于普通前台完整流程已经通过。

原 v543 真实 WGC 隐藏／召回门的初始最小化分支超时。匹配 EXE/PDB 的线程栈证明采集 worker 停在帧通知等待，而非 GPU 复制或 Art 执行。阶段诊断及严格 Size-before-restore API 对照证明：读取旧尺寸后恢复窗口，唯一到达帧的 ContentSize 已变化；原实现丢弃该帧并只 Recreate 原池，静止源没有后续重绘便可能永久等帧。同时新建两帧池和捕获会话的对照会交付完整新尺寸帧；只替换 session、复用原池的失败对照保留，不用于产品。

`live_wgc_pool` 叶统一安装与尺寸换代；旧帧先归还，解绑旧事件并关闭旧 session/pool 后建立新代。继续复用同一 item、device、Signal、Source/Consumer、单调帧序号、取消及原旧＋新峰值预算事务。失败沿原 Session 关闭链回收部分资源；不增加轮询、重试、应用线程、队列、权限、依赖或新旧兼容分支。新增显式原生回归使用自有静止白窗，分别强制 237×39、64×48、256×192 陈旧池，要求不重绘源也交付 128×96 全尺寸帧及中心白像素；原 10 秒期限、像素和生命周期断言没有放宽。

v0.2.33.544 仅分配一次。当前定向 Rust **43 passed / 0 failed**：6 项运行门覆盖真实 WGC 输入／失焦取消／关闭恢复、局部源移动及 resize、隐藏更新与召回、新静止尺寸回归，以及 Live 子 Art 的真实 TLS 执行和同 ID 冻结；后两项使用既有应用夹具，不冒称普通产品前台全链。原隐藏／召回失败门现已通过。6 个运行 Job 均自然归零，无强制兜底；其余 37 项为捕获、帧交付、任务、显隐和目标快照直接依赖。显式运行的新门虽默认 ignored，本批已实际通过，不重复计数。

官方 fmt、版本、全仓 strict 行数、strict all-targets Clippy、测试库与普通 Release 通过。新普通 EXE 固定 SHA-256 为 `43213BB72F46ED69375B41E2B788C2362837FB65FE45CFAC70CDD3C260E209C6`；以该路径和摘要注入既有固定 SHA 工作区 driver，另 **6 passed / 0 failed**，合计本批 49 个去重 Rust 用例，非整库。首次续跑误用 Loom 输入清单导致 driver 复用断言失败，产品构建本身成功；核对正确 native 输入清单及 driver 字节后仅补跑工作区门，没有重新编译产品。原 input1 导入编译失败、诊断原失败和运行器错误均保留。

本批只生成新的内部普通 EXE，未封新 ZIP；最新完整内部 ZIP 仍是 v543，不能当作含本修复的新包。普通 C 完整前台流程尚未关闭；先前 Ctrl+2 选区未被工具返回，未选源，不重复同条件尝试，也不改产品入口、捕获保护或用其他输入 API 绕过。干净资源基线／联合长稳、Canvas 原 250ms 失败、物理多屏／跨机／混合 DPI、人工 IME 及最终整包和全面验收仍单列。以下旧版本记录保留各自历史边界；所有 dirty/untracked、原失败及冻结副线保持，不提交、推送或公开发布，总迁移未完成。

## v543 全新 EXE 路径未能隔离后台查询

一次有不同前提的有界对照：将已验普通 Otter、Loom 壳和 daemon 按完全相同 SHA256 复制到从未被 Computer Use 选中的自有路径，建立全新双 profile；没有窗口枚举、UIA 查询、激活或输入 API，没有产品修改、编译、版本分配或封包。预定最长600秒，初次追踪发现查询即停止，实际122.328秒/61点，不冒充600秒完成。

自有 Loom PID 的 UIAutomationCore ETW 仍有26,515条545/546事件，foreign=0、唯一对端为同一 codex-computer-use.exe，有效事件跨度22.136秒。16MiB上限提前触顶，stop4201/exit5保留；45.906秒是含解析的墙钟时间，不是完整事件跨度。新路径假设已被否定，不再换名或复制重复实验，也不据此认定全部CPU因果已解释。磁盘统计因已知WebView重解析边界未遍历，不是零磁盘。两树由自有Job兜底归零，独立CIM精确副本/trace路径及ETW会话核无残留，不称自然退出；共享helper及可访问性均未更改。

证据main-untracked-identity-543-1。同目录已准备一次合并手测入口及清单，涵盖真正公共Art capsule、完整跨节点键盘和自有动态源Live；仅静态检查、PREPARED_NOT_RUN，尚无人工业务通过结果，不擅自启动。普通业务需不同且可核实的窗口操作条件；干净资源需无持续外部查询的测试会话。Canvas原250ms失败、物理多屏/跨机/DPI、人工IME、完整资源长稳与用户全面验收仍开放；本轮不新增Rust通过数或产品功能，完整内部包仍v543，约94%仅工程粗估，总迁移active。

## v543 联合空闲观测：未调用工具仍有后台 UIA 查询

使用最新完整内部包的普通解包 EXE、普通 Loom 壳和 daemon，全新双 profile，普通合成四象限图、空工具目录及真实本地 bridge 订阅；没有 Art/Live 执行，没有窗口枚举、UIA 查询、激活或输入调用，不改显示/输入法。一次600秒整树观测，排除首60秒后的采样区间单核 CPU：Otter 0.217%、Loom 树 49.395%，两树合计 Private 峰值 373.688MiB。此值包含当前外部查询环境和一次有限诊断，不是可推广的纯空闲/长稳预算；Working set 共享页、GPU 实例和进程 I/O 不当作唯一物理内存或真实磁盘写入。

关键新证据是“没有 Computer Use API 调用”并不等于“helper 没有查询”：自有 Loom PID 过滤的 UIAutomationCore ETW 在有效事件跨度 22.188秒记录 27041 条545/546事件，对端全为 codex-computer-use.exe，foreign=0。16MiB有界 trace 提前触顶，stop=4201/exit5 原回执保留；不称完整30秒跟踪或所有 CPU 都由它造成。只读线程差分显示高占用主要在 Loom 主线程和 WebView2 CrBrowserMain，而非 Otter 图像线程；不据此改产品、结束共享 helper、关闭可访问性或继续相同条件采样刷绿。无查询风暴基线仍未通过，后续必须先有不同且可核实的外部查询条件。

600秒/301点采样完成后，附加profile磁盘统计遇到WebView生成的INetCache/Content.IE5重解析点，原安全检查拒绝跟随，runner最终exit1；不删或放行该链接、不重跑产品，不把整轮伪装通过。CPU/内存/GPU原采样可只读分析，磁盘总量明确不可用。两树仍由finally中的原自有 Job 兜底归零，独立 CIM 精确 EXE/trace 及 ETW session 核无残留，不是普通自然退出。保全部原始记录；PowerShell重定向的UTF-16日志原字节另存raw，内容不变转UTF-8无BOM；首次通过node函数管道未执行行数脚本，核实际文件缺失后改用精确node.exe执行，未重跑产品或采样。临时脚本语法/UTF-8/官方lexer、文档合同及diff单独检查。无产品源码、版本、编译或包变化，最新完整包仍v543。

库存重新按实际场景分类，不新增测试通过数：lease_probe是内部child入口，其真实父门在旧desktop-profile-input原日志通过，receipt/库SHA及相关当前源码/Cargo输入一致，父门已覆盖互斥、exit81异常退出和OS回收，不空跑无环境入口。另一项为冻结退役的ANGLE历史门，明确未跑/未判通过，不再列当前功能。当前未闭环回归仍为Canvas原250ms失败；原3名库存不能解释为3个迁移功能，也不称整库全绿。真正公共capsule本轮因捕获身份不匹配尚未打开，未发编辑输入。普通Live/完整键盘、人工IME/物理多屏跨机DPI、完整双产品业务资源长稳和用户验收仍开放，总迁移active。证据main-quiet-joint-543-1及main-ordinary-capsule-543-1。

## v543 两项系统 UIA 焦点夹具已因果收口

原 EDIT/COMBOBOX 失败日志明确记录 WM_SETFOCUS 时 foreground=HWND(0x0)，独立系统控件对照也在无前台条件下出现线程 GetFocus 命中而系统 HasKeyboardFocus=false。原测试只 show、不建立真正前台，却断言系统级焦点。现在仅在两项 cfg(test) 用例增加原共享 SERIAL 及既有自有 focus_foreground guard；保留全部原 UIA 焦点、Value/Text/父级唯一性、Selection/ExpandCollapse 和唯一 change 提交断言，不修改系统 provider 或生产实现。

复用现有 READY→真实自有按钮点击→精确子 PID AllowSetForegroundWindow→G 授权链，各用独立测试进程，原 120 秒安全限额不变；guard 仍断言实际 GetForegroundWindow 命中自身窗口，仅恢复同进程旧窗口，不附加输入线程或注入全局输入。两项原完整测试名各运行一次并通过，测试树自然 active=0、forced=false，独立精确 EXE 核无残留；不代普通产品自然退出、人工 IME 或普通完整键盘验收。

input1 strict Clippy 因本批写错测试夹具模块路径而失败，尚未构建库；原回执保留。input2 只纠正为原 surface_javascript::fixture 路径，官方 fmt、全仓 strict 行数、strict all-targets Clippy、测试库构建及上述两项运行通过。临时 runner 首次导入了无关 network 依赖，建进程前失败；改为仅导入既有 owned Job 后启动，错误记录保留，未重编。源码 diff 机械核对只增加两项前台前置与串行锁，原断言逐字未变；未增加测试、生产功能、依赖、线程、协议或预算。只有测试库重编，不分配版本、不重编普通产品或封包；当前 v543 完整 ZIP 和解包 EXE hash 未变。

原库存 5→3，剩内部 lease_probe、冻结 ANGLE 历史项和 Canvas 原 250ms 失败；这是回归证据库存，不是迁移功能数量。原无前台失败保留，但不再将其当作未定位的产品焦点缺陷。普通 Live、真正公共 Art capsule/完整跨节点键盘、无查询风暴资源基线、物理多屏跨机混合 DPI/人工 IME、完整双产品资源长稳及用户全面验收继续开放。证据 main-uia-foreground-543-1 的原始前台授权/测试日志及 runtime/final audit；保 dirty/untracked 和冻结副线，总迁移 active。

## v543 最新完整内部整包与解包运行

沿用 v0.2.33.543、不分配版本，在现有独占 target 完成一次官方 candidate 构建，补齐 v541 参数绘制/下拉框、v542 选图取消语义及 v543 IME Art 入口尚未进入完整包的交付缺口。正式脚本核构建前后源码、工具、命令、日志和 EXE 字节，再 stage 许可证、材料清单及 provenance；不能把原裸 EXE 改名冒充这份来源。没有新增产品源码、协议兼容层、依赖或权限。

最新 ZIP：`C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-parallel-20261029/main-package-543-1/package/Otter-native-v0.2.33.543-windows-x64-internal.zip`；SHA-256 `8E8E5A708881B3722B9E9336049DE7EDFC427DF81AF7D38EDE89ECEC2580FFC9`。解包 EXE SHA-256 `9121C6E4739C3284F0BF07C0C858FEACF1274FFDE0B3E22CA75869C8A969B446`。候选身份、正式打包和独立归档验证通过；归档合同 9 项、候选/许可证 Node 合同 13 项通过，含篡改、缺失、多余、路径遍历、大小写重名、伪布尔和退役 ANGLE DLL 拒绝。解包 EXE 通过原 6 项工作区实跑，使用既有固定 SHA driver 和显式 EXE/hash 注入，未重编 driver、未放宽原断言。

全新 profile、自有四象限图、Computer Use 唯一绑定本次解包路径，实际普通窗口已显示正确四象限；不是旧 EXE 截图、隐藏 HWND 或仅编译成功。首次未激活捕获返回其他应用画面，未对该画面输入；仅激活已唯一返回的本轮 Otter 后重新捕获，身份与四象限图一致，原边界记录保留。runner 未注入 UI 输入。自有 Job 已归零，独立精确 EXE 路径核无残留；naturalExit=false、forcedJobFallback=true，不把兜底回收写成自然退出。此次有限启动不代 Art/Live 完整业务、资源基线或长稳。

证据 main-package-543-1 的原始 candidate/package/workspace/ordinary 回执及 runtime/final audit。当前完整内部包由 v540 更新为 v543；dirty、未签名、非正式发布，不提交/推送/安装替换。普通 Live、真正公共 Art capsule/完整跨节点键盘、UIA/Canvas 旧失败、无查询风暴资源基线、物理多屏跨机混合 DPI/人工 IME、完整双产品资源长稳及用户全面验收仍开放；原库存仍 5 名，不等只剩 5 个任务，总迁移 active。

## v543 普通公共参数文本提交与正式结果

复用固定 SHA 的 v543 普通 Otter、现有普通 Loom 壳和 daemon，使用全新双 profile、自有合成图片及回环 HTTP Art。通过普通菜单选择 Synthetic Public 543，Tab 完整参数面板返回标准 Query EDIT；以 Computer Use 已支持的 set_value 将 synthetic 改为 capsule-中文-543，再点击原“执行一次”。上游恰好收到一次该 Query、mode=first、enabled=true、amount=0.5；合成服务按 Query 返回 320×240 绿图，普通正式画布和第二候选勾选均已观察。并非仅修改控件值或把缩略图当正式结果。

本批实际验证的是完整参数面板，不是单字段 capsule，也不是人工 IME 输入。没有重复旧 root-targeted type_text。小宿主的子菜单超出工具边界，键盘调用后仍未选定能力；只最大化自有测试宿主，按新观察重新选择能力后成功。最大化及执行按钮的 AX 缓存拒绝保留，刷新后使用实际截图中的同一普通按钮；未复用失败索引、猜 HWND、修改产品样式或换自动化 API。清单仍只返回宿主，没有独立端口目标；不重放既知丢焦的端口键盘条件，本批未打开真正 capsule，后者与完整跨节点键盘仍开放。

运行 615.985 秒、306 点，合计 Private 峰值 396.094MiB、Working set 峰值 590.422MiB（共享页可能重复计），样本区间单核 CPU 平均 21.052%。包含 UI 查询、窗口状态变化及一次合成业务，不是无查询风暴基线、完整业务长稳或资源达标。以本批 stop 文件触发 runner 清理，两棵自有 Job 均兜底归零，独立精确 EXE 路径核无残留；不是自然退出通过。

证据 main-public-parameter-543-1 的原始 UI/请求/采样、runtime/final audit。无产品源码、依赖、版本、编译或 ZIP 变化；普通 EXE v543、官方内部 ZIP v540。相关文档合同与 diff 检查另附回执。仅关闭普通完整参数面板中文值修改→提交→正式结果这一未验子路径；Live、真正 capsule/完整跨节点键盘、UIA/Canvas 历史失败、实际资源、物理多屏跨机 DPI/人工 IME、完整业务长稳、最终包及用户验收仍开放，原库存仍 5 名，总迁移 active。

## v543 Canvas 真实 child 阶段与线程 CPU 诊断

原 Canvas 服务测试曾在 2×1 小画布的 CanvasDraw 阶段耗时 519.705ms，超过原 250ms 操作期限。旧阶段记录不足以区分 target 创建、实际绘制和等待。本批只扩展 cfg(test) 的既有固定阶段报告：GetThreadTimes 提供当前线程 CPU，显式 ignored 诊断 worker 才回传成功报告，普通测试 worker 仍只回传失败。开关限定测试线程并在嵌套/展开时恢复，父侧沿原有界管道及固定元数据日志，不记录源码、图片、异常正文或业务值。64 样本上限、250ms/32MiB heap/128MiB Job、父期限和原错误/回收语义均不变；没有产品入口、依赖、权限、线程、重试或兼容层。

仅构建一次测试库，沿原脚本/Oracle/实际 child 运行一次显式诊断：call-time 红/半透明蓝快照、350ms dispatcher 等待后独立绿像素、dispose/进程退出、Job 峰值与配额释放断言通过。首次绘制 59.548ms，其中软件 D2D target 创建 55.258ms、命令 0.264ms、最终 EndDraw 1.650ms；第二次绘制 0.384ms。target 阶段当前线程 CPU 记录为 62.5ms，GetThreadTimes 的离散记账不能当作精确阶段占比，也不包含其他 D2D 线程或证明调度原因。实际新建的是 worker 进程，不代表操作系统冷缓存或重建历史负载；完整 mount 包含原本应排除的服务等待，不能把其 418.743ms 直接判作 250ms 违规。

本批相关 9 项 Rust passed/0 failed/0 ignored，包含上述真实 child 父门；不混加其私有入口。原引擎隐私/中断/快照与粘性失败、Trace 限额/CPU 单调/非法报告/坏管道和开关展开回归均通过。strict all-targets Clippy、官方 fmt/全仓 strict 行数/diff 通过；4 份 Rust delta 最大 268 有效行，无新文件或行数例外。仅测试库构建 209.360 秒，没有普通产品重编、版本分配或新包。

当前未复现原 519.705ms 慢阶段，不能以本次通过关闭旧失败或唯一归因冷初始化/CPU/驱动。缺少原失败当时的细分阶段/CPU/调度证据就是当前因果边界；不继续同条件重跑等绿。证据 main-canvas-cpu-543-1 的原始日志及 runtime/final audit；分析脚本首次 Match 切片错误已只读修正，未重跑测试，首轮进程清单误匹配调查命令自身也保留，精确 EXE 路径的独立核验为无残留。库存仍 5 名，普通 Live/公共 Art capsule/完整跨节点键盘、实际资源基线、UIA/Canvas 历史失败、物理环境、完整业务长稳及最终包/用户验收仍开放。普通 EXE v543、官方内部 ZIP v540，dirty/untracked 与冻结副线保持，总迁移 active。

## v543 Surface 输入回归集中补验

复用固定 v543 测试库与现有自有 WinForms sponsor：仅编译临时启动器，不重编 Otter/Loom、不分配版本。Computer Use 唯一选中本批启动器并实际点击一次按钮，原 READY→G 启动前提只授权本批精确 PID；测试仍逐项断言真实前台，不附加输入线程、不注入全局输入、不恢复用户窗口。24项真实自有HWND回归通过，覆盖前台guard、CSS/hover像素与几何、EDIT失焦唯一提交、过期/ABA/隐藏拒绝、Enter取消/默认/FIFO/repeat、指针拖拽/背压/取消/捕获丢失及同步重入。另1项消息身份唯一性在从未切换的私有desktop通过，前后台HWND未变。日志里的expected foreground unwind probe是catch_unwind回归的预期panic，父测试及进程均成功，不是新失败。

按原清单完整名对账新增25项库存通过，30降为5；这不是25个新功能或整库全绿。剩余为两个系统EDIT/COMBOBOX UIA焦点属性旧失败、内部lease_probe入口、冻结ANGLE历史项及Canvas原250ms失败，不为清零重开冻结线、空跑内部入口或改旧断言。启动器/子测试Job自然active=0、forced=false，独立进程核无本批残留；这是测试夹具终态，不代普通产品自然退出。

本批另用全新Otter/Loom profile、自有动态色块源和回环合成Art目录尝试普通Live。第一次按键被工具user-input guard拒绝；重新观察并仅激活自有Otter后，一次目标明确的Ctrl+2调用成功返回，但原宿主随后捕获报window is not a usable app window，list_windows未返回独立选区窗，list_apps没有本批可选目标。没有选源、继续发送输入、换API或据此认定产品缺陷；Live实际帧、Live Add Art及控制均未验。190.328秒/95点仅保留有界运行观察，三棵自有Job均兜底归零并独立核无残留，不能称自然退出或完整资源验收。一次停止命令的PowerShell引号错误未执行成功，随后以本批stop文件触发原runner清理，不重放产品操作。

证据为main-foreground-input-543-1与main-ordinary-live-543-1的原始日志、runtime/final audit。普通EXE仍v543、官方内部ZIP仍v540；普通Live完整流程、公共Art capsule编辑/完整跨节点键盘、无查询风暴的实际资源基线、UIA/Canvas旧失败、物理多屏跨机DPI/人工IME、完整业务合计资源长稳及最新整包/用户验收仍开放。兼容维护/Talk构想排除，dirty/untracked和冻结副线保留，总迁移active。

## v543 前台 CPU 对照与 UIA 请求对端

本批没有修改产品源码、分配版本或重新编译普通产品，复用固定 v543 EXE。使用全新 profile、自有四象限图片和仅含默认 DefWindowProc 根窗/40 个标准 BUTTON 的独立对照；全部前台激活/观察仍由 Computer Use 完成，不操作用户既有窗口，不注入无目标输入。对照源码/编译/Job/逐秒原始数据与 PID 过滤 ETW 均保留在本批本地证据中，不进入产品实现。

标准 Win32 对照实际连续前台 105.016 秒消耗 60.0625 CPU 秒，单核平均 57.194%；同期收到 552,984 次 WM_GETOBJECT，平均每秒 5,265.712 次，根窗和全部 child 的 WM_PAINT 计数增量为零。首次显式 AX 之前已经发生；最小化后的稳定 53 秒 CPU/GETOBJECT 均不再增长。因此 Canvas 并不是这种前台高 CPU 现象的必要条件，不能再据它盲修 Canvas 或继续判参数 paint 循环。

通过 Windows 已注册的 Microsoft-Windows-UIAutomationCore provider，仅启用自有 PID 和 545/546 两种服务端请求/响应元数据事件；标准对照记录 67,695 个事件（43,552 请求），唯一 PeerProcessId 对应 codex-computer-use.exe。固定普通 v543 EXE 的独立新 profile 对照也取得 91,615 个事件（59,191 请求），唯一对端仍是同一工具进程；实际前台采样 52.187 秒/29.75 Job CPU 秒，约 57.007% 单核，包含追踪开销，不是纯业务资源基线。没有读取请求正文、上传符号或终止共享 helper。

产品追踪在原 16 MiB 上限结束，实际保留事件跨度 16.119 秒；ControlTrace stop=4201、tracer exit=5 如实保留，不能称完整 30 秒追踪通过，也不扩大预算重跑。标准对照跟踪 start/enable/stop 均为 0；最终独立核本批所有目标 EXE 和专属 ETW 会话均无残留。标准窗口两次由自身有界消息泵正常结束，普通产品两次仍由自有 Job 兜底，不能称普通产品自然退出通过。

首次产品对照因引用了不存在的合成夹具路径而在建进程前失败；第二次追踪窗口结束前没有实际前台样本，ETL 仅 header，不计有效业务证据。第三次改为 CUA 已激活/截图后显式 arm，取得上述有效但触顶的原始事件。失败与空追踪均保留，不以回执摘要刷绿。

本批定位了当前高 CPU 复现中的验收工具请求路径，不证明全部历史 CPU 开销、所有资源瓶颈或其每个 CPU 周期都已解释。不得关闭产品可访问性、改系统设置、杀共享工具或放宽预算来获得低占用；无查询风暴的实际前台/完整业务资源基线仍需另验。普通 Live、公共 Art capsule 编辑及完整跨节点键盘、UIA/Canvas 旧失败、物理环境、完整业务长稳及最新完整整包/用户验收继续开放；普通 EXE 仍 v543、最新官方内部 ZIP 仍 v540，库存 30 名未变，总迁移 active。

## v543 宿主 IME 快捷键与普通独立 Art

当前输入法下，自有标准观察窗实测Shift+1交付WM_IME_CHAR U+FF01，再交VK_PROCESSKEY而非VK_1；ImmGetVirtualKey也为229，工具扫描码为0。宿主现在只在IME字符为!或全角！、数字1与Shift确实同按、无Ctrl/Alt/Win时识别既有Art快捷键；不消费WM_CHAR或VK_PROCESSKEY，不改标准EDIT、输入法设置、全局输入、协议或依赖。Live沿原公共识别入口消费，原owner/暂停/锁定/单槽和repeat围栏不变。

只分配一次v0.2.33.543。input1静态/lib通过，新增3项测试失败；input2只纠正标准EDIT最终WM_CHAR的系统转译边界，该项通过，但另外2项仍失败。独立无窗口、无输入的私有desktop探针证实SetKeyboardState能设置Shift、不能在本环境把数字1设为按下，故“先创建消息队列”的假设被证伪，旧失败不覆盖。input3把生产相同判定抽为纯键态边界；3项新回归分别验证精确组合、真实隐藏宿主缺数字键不误开窗及标准EDIT最终字符保留，另扩展Live负向/repeat回归。不是伪造键态的正向IME验收。最终13项库测试和新普通EXE的6项工作区门共19 passed/0 failed/0 ignored，非整库；strict all-targets Clippy、fmt、官方版本与全仓strict行数4712文件通过。4份Rust delta最大223有效行、1个新测试叶，无新增例外。普通Release构建508.172秒，EXE SHA-256为`ECC02D2FCD9FC258BB9A1F449CDFFCD9563DCA5ACF26B0AFB55A504BBE4178F7`，dirty、未签名、非正式发布。

新普通EXE、全新双profile、自有四象限图片及回环合成HTTP API联合实跑：画布聚焦后一次Shift+1真实打开“添加 Art”，刷新目录后再次打开并选Synthetic Native 543，原图保留，右侧出现独立红色子Art与可见连线。上游恰好收到一次默认query=synthetic、mode=first、enabled=true、amount=0.5、result_index=0，对应320×240合成红图；不是把当前源节点直接替换为Art，也不是缩略图冒充正式结果。刷新按钮AX索引缓存拒绝保留，重新观察后使用实际模态截图坐标成功；没有重试旧索引。当前生产服务/跨机/人工IME仍未验。

F6、Tab已观察从源image到output，再跨至子Art image；End也到子Art output。但工具清单不返回可独立绑定的新钉图/端口窗，以root为目标的后续输入会丢失子节点端口焦点。两次Ctrl+Enter不能据此确认为完整重连提交，公共参数capsule未到编辑阶段；不刷相同绑定或据此改产品。一次helper active-request错误后，等待并重新只读清单/绑定成功；原错误保留。关闭调用或随后捕获报`window is not a usable app window`，新清单已无本批可选窗口，但进程树尚在；不推断自然退出或产品退出缺陷。停止本轮runner后两棵自有Job归零，独立核原PID及三个固定EXE无残留，forcedJobFallback=true。

运行515.125秒/256点，有效区间513.109秒合计208.75 CPU秒，单核平均40.683%；合计Private峰值387.871MiB、Working set峰值601.551MiB（共享页可重复计），Handles/GDI/USER峰值5053/122/277。236点前台属Otter、20点属Loom，但负载和阶段变化，不与旧批次直接比较或称CPU改善/根因关闭、完整业务长稳或资源达标。证据main-art-ime-543-1/input3及ordinary。最新普通EXE为v543，官方ZIP仍v540，不为本批反复封包；普通Live/公共参数编辑/完整跨节点连线、CPU因果、系统UIA/Canvas旧失败、物理多屏跨机DPI、完整合计资源长稳及最终包/用户验收仍开放。库存仍30名，兼容维护/Talk构想排除，总迁移active。

## v542 普通 Art 目录、执行与候选结果联合实证

复用固定SHA的v542普通Otter、Pong修复普通Loom壳和当前普通daemon，未改产品源码、版本、依赖或重编。全新双profile、自有合成图片和回环HTTP合成API下，普通节点菜单加载并选择Synthetic Native 542，宿主Tab打开可读完整参数面板；关闭参数变化自动执行后，真实点击“执行一次”，上游收到query=synthetic、mode=first、enabled=true、amount=0.5及result_index=0，正式画布显示320×240红图。两张候选显示后，点击第二候选，上游仅再收到一次result_index=1，正式画布变为320×240绿图；后续截图及AX确认第二候选勾选、Result index为1。候选按既有合同修改索引并显式重执行，不是直接发布缩略图；此前“选择不应再次执行”的临时预期不成立，两次请求分别对应首次执行与选择候选，不是意外重复。

这只关闭普通当前节点目录、默认参数提交、正式结果和多候选选择链路，不等新增独立子Art/Live、参数编辑或跨节点键盘通过。Shift+1未建立响应；参数EDIT的AX索引被工具缓存拒绝，清单不返回可独立绑定的owned参数窗。坐标聚焦Query虽获AX确认，随后以root为目标的type_text重新激活宿主并触发“剪贴板中没有可用的图片”，Query仍synthetic；只关闭提示一次，不刷相同绑定、不换输入API，也不据此判产品文本编辑有缺陷。

900.5秒/447点运行中，两棵Job全部必要子进程合计Private峰值408.180MiB、Working set峰值607.828MiB（共享页可能重复计算）；898.484秒样本区间合计450.046875 CPU秒，单核平均50.090%，其中Otter44.998%、Loom5.092%。句柄/GDI/USER合计峰值4801/129/273。含UI观察、仅两次合成执行且无同步前台身份采样，不能当严格前台对照、完整业务长稳或资源达标；高CPU原因仍开放，旧529.840MiB Private峰值不被覆盖。

退出菜单实际显示正常未保存确认，但未选择保存/放弃，随后两棵自有Job到期限兜底归零，naturalExit=false；独立只读核无路径/profile相关残留，不将此当自然退出通过或退出缺陷。当前上游不是外部生产服务、不是跨机验收，无用户文件上传/读取、显示/输入法变更或既有应用操作。证据main-ordinary-art-542-1的runtime-audit、原始请求/UI事件和末次AX，产物hash均前后独立核对。官方ZIP仍v540，不为本批重复封包。普通Add Art/Live、公共参数capsule/完整跨节点键盘、CPU因果、系统UIA/Canvas旧失败、物理环境、双产品完整业务资源长稳及最终包/用户验收仍开放；库存仍30名，非功能数量，兼容维护/Talk构想排除，总迁移active。

## v542 图片文件取消不再误报配置错误

针对v541普通产品真实复现的取消误报，原文件选择owner现在区分Accepted、Cancelled、Rejected：取消不排路径请求、不读取文件、不变更配置或提交凭证；仅准入拒绝保留配置错误，系统文件框的非零扩展错误仍正常传播。选择成功仍沿原owner/配置Stamp、锁/隐藏/暂停、单槽及worker准入；Art入口仍仅在请求接纳时返回true。面板只投影结果，只有实际配置变化才标记committed。没有新依赖、协议、权限、线程、timer、兼容层或资源预算变化。

一次分配v0.2.33.542，集中一次测试库构建387.031秒、一次普通Release构建514.891秒；strict all-targets Clippy 187.515秒通过。57项相关库测试及新EXE的6项工作区门共63 passed/0 failed/0 ignored，包含4项新增取消/拒绝/提交凭证回归，不是整库。官方fmt、版本、全仓strict有效行数4711文件及diff通过；5份Rust有效行为194/42/406/135/87，一个新测试叶，无新增例外。未变workspace driver按测试输入及固定SHA复用，没有为验收重复编译。

普通Release使用全新profile和自生成四象限图，经Computer Use唯一选窗打开图片参数窗及系统文件框；刷新AX确认“对话框 打开”且无位置错误，仅取消、不选择或上传文件。取消后再次刷新确认文件框消失、仍为“图片 · 未选择”、原配置错误不存在，可见参数各行正常。首次点击取消索引遭工具缓存拒绝，重新观察确认模态仍在后，只向自有窗口发送一次Esc完成取消；原工具错误保留。随后正常关闭按钮调用未建立程序退出，窗口捕获两次返回`no monitor found for window`后停止UI；512.891秒由自有Job兜底，naturalExit=false、forcedJobFallback=true、ownedJobEmpty=true，独立查无本批产品残留。取消修复通过不等于自然退出通过，也不据工具错误推断产品退出缺陷。

新EXE SHA-256为`80B2427A66721FC6290DFC029C8BE8F75AD12CD1653783AA7037E1C49921E448`，是dirty、未签名内部runtime候选，非新的官方ZIP。最新官方ZIP仍v540、不含v541/v542修复；不为文档重编、重分版本或重复封包。证据main-file-cancel-542-1。普通Art/Live/多候选及完整跨节点键盘/公共参数capsule、人工IME、CPU具体因果、两项系统UIA焦点失败、Canvas原250ms失败、物理多屏跨机DPI、双产品完整业务合计资源长稳及最终包/用户验收仍开放；库存仍30名，非功能数。兼容维护/Talk构想排除，冻结副线和全部dirty/untracked保留，总迁移active。

## v541 普通端口键盘、图片模态与热点实采

固定普通EXE、新profile和自生成640×480四象限图，经真实CUA选窗观察：F6进入image端口，Tab到output，Ctrl+Enter出现选源标记，Esc取消，Shift+Tab返回image，F2显示图片单字段child并聚焦上传按钮。仅关闭这些普通图片局部入口/导航/选源取消/capsule可见缺口，不等跨节点连线、Art公共参数、多候选、Live或人工IME通过。图片右键菜单实际可滚动，滚动后可见Loom服务及打开Art入口；上轮仅凭初始捕获条目推断“不是节点菜单”不成立。Shift+1仍未建立响应，没有刷键或改输入法。

首次CUA前、排除启动10秒的后台108.375秒消耗0.171875 CPU秒；仅清单/截图的后台37.125秒消耗0.203125秒。激活后、首次显式AX前的同stage且前后均自有前台有效间隔106.984秒消耗58.84375秒（单核55.002%），已排除采栈区间；显式AX后93.172秒为52.046875秒（55.861%）。不能再声称首次显式AX才触发，也不能证明helper内部无UIA。最小化stage中仅后台有效间隔207.313秒消耗0.640625秒；真实关闭工具窗后82.984秒仍消耗20.015625秒（24.120%），但窗口大小等条件已变化，不能据此唯一归因工具窗。错误命名的toolbar-closed-foreground阶段发生在关闭失败后，绝不当作已关闭对照。

这次热点真实命中：两线程20秒窗口合计10.609375 CPU秒，716/716份栈暂停前后均属自有前台，最长暂停12.734ms且finally恢复。主线程增6.078125秒，358份中269份顶层GetMessage、67份含UIAutomationCore；另线程增4.53125秒，358份均含UIAutomationCore。仍只有模块/邻近export/产品RVA，不能把等待顶帧当CPU归属或邻近导出名当私有函数；CPU因果和观察者影响未关闭。1634点Private最高152.227MiB、Working set最高214.398MiB，不是双产品业务长稳或资源达标。

首个文件模态因隔离USERPROFILE缺Desktop子目录出现位置提示，保留前提失败。第二个全新profile预建该目录后，图片文件框正常打开、Esc返回且图片仍未选择，但界面错误显示“图片输入配置已变化或暂不可用”；源码确认用户取消与准入拒绝均为false，本缺陷待修，不将完整取消语义记通过。未选择/上传或读取用户文件。第二次产品正常关闭201.469秒、naturalExit=true、Job归零；第一段1801.031秒到原期限由Job兜底，非自然退出。无自有残留、版本/产品源码/产物未变、不重编不封ZIP，证据main-cpu-contact-541-1。库存仍30名、Canvas原250ms及两UIA失败不关闭；完整业务、物理环境、合计资源长稳和最终包仍开放，兼容维护/Talk构想排除。

## v541 普通参数复验与系统焦点对照

复用固定v541库及普通EXE，没有分版本、改产品源码或重编。六类声明式表单input/textarea/number/switch/select/slider的回环TLS attach、所属HWND编辑、授权事件batch、权威revision 4替换及撤权销毁父门实跑1 passed/0 failed。生命周期Active ACK仍由夹具注入，不冒称真实Loom联用、JS或普通物理输入。两个原生EDIT/COMBO UIA焦点失败仅开既有有界trace诊断，仍2 failed；原失败不覆盖。独立C++标准控件同用CUIAutomation8/MTA，在未切换desktop实测SetFocus成功、owner线程GetFocus和指定tid的GetGUIThreadInfo均命中child，但系统UIA HasKeyboardFocus=false。它证明当前环境中两种焦点观察不等价，不唯一证明desktop因果、不排除产品另有缺陷，不能改断言或把两门记通过。三个私有desktop回执均closed、无SWITCHDESKTOP权限、输入前台HWND前后不变。

用户明确恢复仅全新profile、自有Otter/Loom测试窗口前台。最新普通v541经Computer Use真实清单唯一匹配路径/标题、画布聚焦，一次Tab已显示可读参数各行；端口显示模态打开/Esc取消、宿主Tab关闭均观察确认。即时AX滞后不靠重复按键补偿；一次element 58缓存拒绝保留，重新取新截图后点击同进程参数按钮成功。没有操作用户既有应用、改显示/输入法或换输入API。只验普通图片参数初始绘制，不代表Art多候选、内嵌capsule、Live、完整端点键盘或人工IME通过；Shift+1本轮未建立Art响应。

本轮参数展开48.093秒安静片段的自有Job消耗26.3125 CPU秒；独立前台60点均属产品，但仅部分覆盖该48秒，不称完整同条件前台基准。参数关闭后60.047秒独立对照消耗32.46875 CPU秒，60/60前台样本均属产品，单核口径54.072%；因此高CPU仍开放且不是参数窗存在的充分归因。两个已观察高累计CPU线程的后续12秒栈采样时，分别只增0/0.015625 CPU秒，热点未复现；448份栈、最长暂停12.414ms且finally恢复，无匹配热点时不能拿UIAutomationCore邻近export当具体根因。样本不是双产品业务资源达标或长稳。

普通产品901.812秒达到原自有Job期限，forcedJobFallback=true、ownedJobEmpty=true、naturalExit=false；末次CUA输入报foreground window did not report a process id，时间接近期限不当确定因果，不重试旧目标。无产品残留，不以Job兜底称自然退出。本轮只更新两份台账和外部证据，最新官方ZIP仍v540且不含541修复。剩余库存30名含前台/系统UIA未验、内部lease入口、冻结ANGLE历史项和Canvas原250ms失败，不为清零重开冻结路线；完整可见业务、CPU因果、合计资源长稳、物理多屏跨机DPI、最终完整包及用户验收仍开放。兼容维护/Talk构想排除，不提交推送安装部署发布，不启动Loom原生化。证据main-uia-focus-541-1。

## v541 参数初始绘制与下拉框修复

已确定性复现聚焦路径：宿主先SetFocus，再打开参数窗，IsDialogMessageW可同步处理父paint；随后无条件reveal→place→InvalidateRect使父paint自续。input6记录507次参数父窗口paint，其他11个child仍未绘制；宿主render_active/render_deferred均false，不能归因主Canvas自续。input7仅在滚动位置改变时place，保持原导航、等待顺序和预算；新增真实Application聚焦消息泵回归通过。此机制已证，但不是所有历史CPU/白行问题的唯一根因证明。

下拉框独立修复：标准COMBOBOX保CBS_DROPDOWNLIST/CBS_HASSTRINGS，无CBS_SORT；owner-draw负责选中/禁用列表项，收起区域复用既有Surface像素层。系统继续拥有字符串、类型映射、索引、弹出、键盘和UIA；标签按原128字符清洗，固定257个UTF-16单元缓冲，DPI高度与subclass在原HWND寿命内更新/解除。没有新依赖、协议、权限、线程、timer或资源池。实际WM_PRINTCLIENT/DIB像素回读和消息泵测试不代替普通Art/可见业务。

最终input7固定同源库83项通过：参数49、capsule8、共享下拉控件26；另复用未变driver对新普通EXE执行工作区/系统会话6项，按完整名共89 passed、0 failed/ignored，非整库。7项新增回归覆盖未聚焦/聚焦初始绘制、EDIT/COMBO收起区、列表选中/禁用/不同标签、Unicode有界文字及原值/DPI。参数命令中的旧capsule过滤前缀未匹配，8项由正确根组另行实跑，不重复计数。strict all-targets Clippy、fmt、全仓strict有效行数4710文件及diff通过；10份Rust最大261有效行，3个新文件，连同版本11项delta均UTF-8无BOM，无新增例外。版本检查按input4至7仅Rust变化的精确输入复用，只分配一次v0.2.33.541。

普通Release构建578.125秒，profile.test=false，EXE SHA-256为`C96B25618A6AC92949BF204861FF53EDA30D5DEC5D9D8E670783CE2DC93A975D`；当前是dirty、未签名内部runtime候选，不是官方新ZIP。最新已验证ZIP仍v540，不包含本修复；不把runtimeVerified=false改成假成功。input1至6的编译/像素/聚焦失败及夹具纠正均保留，最终通过不覆盖原证据，也不提高任何时间门槛。

普通新profile产品已启动，但首次Computer Use清单即因用户物理Escape停止，没有选窗、激活或发送UI输入。自有runner随后中断，产品只读核无残留；缺ordinary/result.json，不称自然退出或严格Job终态已证。普通白行视觉、修复后CPU及完整Art/Live/候选/capsule/键盘、人工IME/模态、双产品业务合计资源长稳、物理多屏跨机DPI与最终用户验收仍开放，Canvas旧250ms失败不关闭。证据批次为main-parameter-paint-1/input7的verification/source-delta/affected-lines及原始receipts，前台停点见同批STATUS/handoff。全部兼容维护和Talk构想排除；保dirty/untracked及冻结副线，不启动Loom原生化，不提交推送安装部署发布，总迁移active。

## v540 普通参数入口与当前前台边界

复用最新已解包内部候选，不改产品源码/版本或重编。CUA从实际清单唯一选中自有路径/标题，新profile点击真实画布并确认焦点后，一次Tab打开参数窗；即时AX曾滞后，后续截图和AX确认时没有再次按键。普通图片参数控件及输入image/输出output的端口显示模态可打开，Esc取消后宿主恢复，再由宿主Tab关闭参数窗。只确认入口与取消，不等完整端口编辑/保存、Art/Live/capsule或完整键盘通过。

参数窗多行白底文字不可读、仅焦点reload正常，视觉验收未通过，根因未证；对子窗Raise的工具缓存拒绝保留。F4后出现输入法方案浮层，Ctrl+E未建立工具显隐结果，没有改用户输入法或注入无目标输入。600.375秒自有产品树由原Job期限兜底归零，非自然退出；最后关闭调用报前台PID不可读，时间接近期限不是确定因果，见`main-visible-keyboard-540-1/clarification.json`。300点样本Private最高140.238MiB、Working set最高149MiB；30秒片段单核CPU最高82.563%，300–390秒片段仅0.330%且I/O增量0，不能推定参数窗持续空转。进程I/O包含管道等，不冒称物理磁盘写入；这不是普通双产品业务合计资源或CPU根因关闭。普通完整业务、人工IME/模态、物理环境、长稳及旧Canvas250ms失败仍开放，总迁移active。证据为`main-visible-keyboard-540-1`的运行、CUA、clarification与resource/final audit；旧失败不覆盖，兼容维护/Talk构想排除。

## v540 原生窗口初始标题与最新内部整包

普通直接图片入口的对照查明：宿主、工具面板、参数面板和工作区端口层在WM_NCCREATE安装userdata后直接返回1，跳过DefWindowProcW保存初始标题的系统处理。仅本进程隐藏窗的最小负/正对照分别读回空标题/完整中文标题；修复保留原宿主独占绑定及重复候选拒绝，只让成功创建继续默认处理。不改style、捕获保护、焦点策略、协议、依赖、线程、队列或持久化。

8份Rust中4份生产接线、4份测试；新增2项标题测试并扩展原宿主/端口测试，实际GetWindowTextW核对首次显示前的标题，保护原身份/取消/清理。82项相关Rust精确名passed，0 failed/ignored；strict all-targets Clippy、官方fmt和全仓strict有效行数通过，受影响Rust最大471有效行，无新增例外。同v0.2.33.540集中构建一次测试库及一次官方Release候选，不重新分配版本。原Canvas超时等历史失败不被这些成绩关闭。

官方candidate完整绑定源码前后快照、工具及编译日志，最新内部ZIP包含此前多候选选择和本次标题修复，311个归档文件；ZIP独立核验后真正解包，6项固定SHA普通产品工作区/重启/系统会话门通过。EXE SHA-256为`b4fe2d7a9684176d99203b8fb8b15909a07c82ff45a40e9b18a63715b54d6aac`，ZIP为`02fbbb6bf8092e04197f722467612f90e5887c094c1e4d0c8a7fcd29cef91eba`。dirty、未签名、不可正式发布，不修改官方runtimeVerified=false元数据来扩大验收。

普通前台先用旧固定EXE直接打开fixture，观察到自身画布、工具及右键菜单，最后自然退出、Job归零；本次部分外来画面有实际遮挡对照，不将所有历史截图异常归为同一根因。最新解包EXE再次用新profile启动，Computer Use清单正确返回Otter Native，AX含真实工具/端口标题，提升已核自有窗口后实际观察到图片、画布和标题。裸Tab未建立参数面板响应；一次工具按钮被缓存状态拒绝，后续截图完全被其他应用遮挡时没有发送坐标输入，停止本轮UI并由自有Job回收，不能称这次自然退出或完整键盘/参数验收。未操作用户应用、全局注入输入、修改显示/输入法/隐私设置或持久化截图。

构建本身及官方provenance已成功；外层临时runner最后因对相对`powershell.exe`求摘要而未写出自己的receipt，原异常保留。只读重验官方provenance后继续打包，没有重编、补造旧receipt或伪称原runner严格清理已证。后续脚本均使用绝对可执行路径。

证据为`main-direct-image-540-1`的title-probe、verification-audit、candidate/provenance、package/archive-verification、workspace-runtime、两份普通运行及CUA回执。剩余库存仍80，不是功能缺口数；普通Art/Live/候选/capsule/完整键盘、人工IME/模态、物理多屏跨机DPI、完整业务合计资源长稳及最终用户验收仍开放。兼容维护/Talk构想排除，三冻结副线/v464/用户release97及全部dirty/untracked保留；没有Loom原生化、提交推送安装部署或正式发布，总迁移active。

## v540 Tile真实运行回归与前台观察边界

复用固定诊断库，不改产品源码/依赖/版本或重编。此前库存中的5项Tile真实TLS/WSS/产品Runtime回归在Default desktop自有(-30000,-30000)窗口分别隔离通过：成功heartbeat ACK前不授输入、非法ACK拒绝、图片冻结/黑场/恢复、慢Art图片超过5秒仍维持Live单连接，以及停止/撤权取消在途并释放像素。原D3D12 Present均返回S_OK并有confirmed记录；没有发送输入、激活测试窗或改显示设置，5项前台HWND前后相同，自有Job归零。库存85降到80，不等于普通可见Tile/键盘或80项功能缺失；Canvas旧超时仍开放。

普通固定SHA Otter使用两个全新profile启动：恢复钉图未被Computer Use窗口清单返回；空工作区虽返回一个候选，但AX不可用，截图未建立自有可编辑工作面。未向Otter发任何输入，不使用截图中的其他应用、不猜HWND、不改style或换输入API；两次均自有Job兜底归零，不称自然退出或可见参数/capsule验收通过。用户旧release97与其热键不动。

另以固定普通Loom壳/daemon及新profile进行421秒、316点CPU观察，无Art/Live负载、不暂停线程。一次Raise未建立计划中的持续前台；随后仅点击自有MCP控件，quiet_b中实测连续前台有效间隔65.953秒，整棵自有树消耗1.796875 CPU秒（单核口径2.724%），其中壳main为0.1875秒、CrBrowserMain为0.34375秒。本次安静前台片段未复现旧热点，但不能证明修复或排除间歇。8次有界只读UIA观察实际发生在后台，前台UIA同条件对照未完成；末次读取在原420秒Job期限已结束后返回`foreground window did not report a process id`，未重试。样本不是双产品业务长稳或最终资源验收，仍不关闭CPU根因。

证据为`main-ordinary-controls-540-1`的tile-selected/tile-result/verification-audit、两组ordinary结果、cua回执、observer-result/cpu-audit及原日志。只有两份仓库Markdown变化；普通EXE和诊断库继续各自固定SHA，无新ZIP、提交推送安装部署发布。普通可见交互、人工IME/模态、多屏跨机DPI、完整双产品业务合计资源、历史超时与最终包/用户验收仍开放；兼容维护及Talk构想排除，总迁移active。

## v540 Canvas 分阶段诊断与剩余回归补证

本批只在4份Rust的`cfg(test)`路径补充固定Canvas阶段与顺序断言：DIB背板、软件D2D target、BindDC、初始EndDraw、图片准备、命令和最终EndDraw。沿原Trace及64样本上限，不记录脚本/业务值/异常文本；未改普通产品行为、250ms操作预算、heap/Job/父期限、依赖或权限。一次测试库构建821.172秒、strict all-targets Clippy 267.188秒通过，官方全仓strict有效行数4707文件通过、无新增例外；普通EXE不重编、不重分版本。

按完整Rust名去重184项通过，其中178项接受关闭原库存，库存263降至85；其余为5项相关重验及原Canvas child的单次诊断通过。147项原选定组最初为146 passed/1 failed：UIA用例明确要求当前及input desktop均为Default，未切换私有desktop必然被安全preflight拒绝。仅该项改在Default的自有离屏NOACTIVATE控件对照后通过，未发送输入或扫描用户应用，前台HWND前后不变，自有Job归零；原失败保留。另有17项背板/模型/诊断、15项Canvas service/result及4项真实回环TLS device session通过，不冒充普通可见Tile或完整焦点链验收。

原CanvasDraw的519705µs失败仍开放。复制真实叶源码的有界独立探针中，三次软件target创建为76.458/7.649/11.389ms，像素与预算释放断言通过；它不是原worker，不能唯一解释旧520ms，也不归因GPU。细分诊断后的原真实child本次单次通过，但成功worker不回传阶段，成功模型只是暖路径；没有因果修复证据，不刷绿消除旧失败、不从剩余库存删除该项。85个库存名不是85项缺失功能或完成率。

证据在`main-canvas-draw-540-1`的verification-audit、diagnostic-audit、各原始失败及分组回执。当前普通EXE仍为多候选input4固定SHA，诊断测试库另存，不产生新完整ZIP。普通可见候选/Art/Live/capsule/完整键盘、人工IME/模态、多屏跨机DPI、完整双产品业务合计资源长稳、CPU具体根因及最终包/用户验收仍开放；兼容维护及Talk构想排除，总迁移active。

## v540 最新普通产品联合与剩余回归核查

复用多候选input4普通Otter、Pong修复普通Loom壳及当前普通daemon的固定SHA，新profile/自有Job联合运行900.016秒、383点；没有分版本、改产品源码或重编。只恢复一张普通图片，未执行Art/Live业务负载。Computer Use只激活并观察唯一返回的自有Loom窗口；最新Otter仍未被返回为可操作目标，未猜HWND或换输入API。合计Private为221.652–391.602MiB、Working set为392.500–580.934MiB（共享页可能重复计）；含UI观察和短时调试采样，不是无探针CPU基准、资源达标或完整长稳。订阅样本均为1；两棵自有树末尾Job兜底归零，不冒充自然退出。旧529.840MiB峰值仍保留。

当前同源库新增57个库存普通完整名通过：31内存/合同、13文件ACL及合成故障child/图片Job、13原锁污染项隔离进程通过。原27项集成批为13 passed/14 failed；首个Canvas服务真实child在原250ms门失败，诊断CanvasDraw阶段约520ms，随后13项仅SERIAL PoisonError，未真正执行业务断言。未改期限、断言或生产实现；13项只为解除污染分别隔离，原日志保留。另显式执行最新普通EXE的Canvas服务worker门通过，SHA前后核对一致；这是不同的产品worker对照，不关闭原test child间歇或证明根因。320旧库存减57，当前仍有263个待审/未通过名，含这1项真实失败和不能空跑的内部入口；不是263个功能缺口或整库通过。

前台仅针对自有Loom main/CrBrowserMain分别取得696份有界线程栈；短暂停顿后finally恢复，最长约28.6/23.8ms。有CPU增量的栈中分别7/26份含UIAutomationCore，另有窗口消息分派/同步调用和browser侧RPC。只有模块/邻近export及偏移，当前匹配PDB不可得，不能将邻近导出名当私有函数名，不能凭少量命中断言具体循环或工具是唯一原因；观察者效应尚未排除。初次指令地址样本和llvm-symbolizer缺PDB原报错保留；未据此修改Loom或关闭CPU问题。

证据为`main-visible-joint-540-1`下verification-audit、ordinary-runtime-audit、cpu-stack-audit、原集成失败与isolated-result。普通可见候选/Art/Live/capsule/键盘、人工IME/模态、真实多屏跨机DPI、完整业务合计资源、Canvas原失败、CPU/历史间歇及最终最新整包/用户验收仍开放。未提交推送安装部署发布，不重开冻结副线或Loom原生化，全部兼容维护及Talk构想继续排除。

## v540 Art 多候选预览与选择

补齐原版已经实现、原生生产UI此前未消费的多候选结果：完整参数窗显示两列原生候选按钮，包含缩略图、标题、序号和选中标记，沿原滚动/键盘管线。选择仅修改已声明、可编辑的数值result_index并显式重跑；绝不把缩略图发布为正式端口。单字段capsule不复制候选列表，linked/secret/未声明/文本或超范围索引不可选。

loom_output_batch::candidates在原执行worker准备不可变Arc集合，最多64项/唯一非负index，标题清洗并截128字符；仅接受内联PNG/JPEG/GIF/BMP，不联网、不读文件或cachedThumbnailPath。单图编码前字节限4MiB、总encoded尝试预算16MiB，WIC先验容器/8192边长，直接缩到最长边64；不分配完整源像素，不对WIC内部工作内存另作保证。新执行、无候选正式结果和Local替换清除旧候选；正式提交成功后绑定当前配置Stamp。参数窗保集合/配置/Snapshot身份，入队和实际提交均核dirty/IME，保原有界队列、owner/锁/隐藏/暂停/执行准入；Spec必须精确解析到候选index，不能clamp成另一个值。无新依赖、协议、权限、线程、timer或资源池。

最终input4同源70项相关Rust passed，包含13项新增；另6项旧driver对最新普通Release EXE的工作区回归通过，非整库。新增覆盖真实PNG像素、有界准备/取消/非法URL与路径、实际BUTTON选择与唯一Manual请求、同index换代/配置/锁/linked围栏、入队后新草稿和IME、越界/文本Spec禁用、标准Space消息管线及真实GDI绘制像素回读。键盘测试只在未切换独立desktop/自有窗口运行，不等于物理键盘或普通可见验收。复用driver前核全部依赖测试文件，仅已知无关JS夹具与539不同，未重编未变driver。

官方fmt/全仓strict行数、strict all-targets Clippy、固定SHA库及普通Release通过，15份Rust源码delta含5新文件，最大390有效行，无新增例外。EXE `C:\Users\Public\nas_home\AI\GameEditor\linshi\otter-native-parallel-20261029\main-candidate-picker-540-1\input4\artifacts\product-build.exe`，SHA-256 `154ECDAD3C8EB185296789864A6B6766E602A2526274A3F4F0DD6CCBEA553306`；它含候选新功能，旧B3 ZIP/EXE不含。dirty内部候选，未新分版本、未组最终ZIP、未签名发布。input1编译可见性/测试lint、input2产品因已证边界问题被主动停止、input3 cmp_owned失败均保留；没有竞争同target或改动在途源码。

另一次自有普通Loom壳线程采样198点/239.235秒：同stage前台106.001秒中CrBrowserMain消耗17.796875 CPU秒、Loom main 15.75秒，renderer仅0.640625秒；非前台54.719秒对应browser/main为0.28125/0.109375秒。热点收敛到前台native shell/WebView browser-side，不凭无堆栈证据断言具体循环或React根因，也没有改Loom产品源码。证据main-foreground-cpu-540-1/cpu-audit.json。

普通可见候选/Art/Live/capsule/完整键盘、人工IME/颜色文件模态、多屏/跨机/DPI、最新双产品完整业务与合计负载长稳、CPU具体根因及历史订阅间歇、最终完整隔离包和用户验收仍开放。旧普通双产品Private峰值529.840MiB不被单壳/后台基线替代。全部兼容维护和Talk构想排除，三冻结副线/v464/用户release97与全部dirty/untracked保留，不启动Loom原生化，不提交推送安装部署发布，总迁移active。证据main-candidate-picker-540-1/input4。


## v540 订阅写期限与追加回归

Loom订阅收到Ping后，Pong的flush原先继承读slice，若读完后被调度暂停至期限已过，会以TimedOut退出连接。当前同owner内为flush重设独立250ms写期限，不增加重试、线程、兼容分支或服务端idle预算。新TLS回归确定性验证旧flush在已到期读期限下失败，同一连接用修复路径交付空/非空Pong；连同原确认/订阅/重连/停止共11项passed。不是历史30秒零样本的唯一根因证明，原失败不抹除。

本批新增442个此前未启动的Otter完整名通过，复用固定v540库，无Otter源码/版本/EXE/daemon重编。224项自有回环TLS/worker通过；218项desktop批次原为216 passed/2 failed，两项D3D11/D3D12在未切换私有desktop返回0x087A0001（DXGI_STATUS_OCCLUDED），严格呈现断言仍失败。仅这两项改在当前desktop自有2×2离屏NOACTIVATE窗口对照后通过并实际读回目标像素，无焦点、输入或显示配置操作，不冒充普通可见产品验收。原失败及全部成功项不重跑。780项旧库存减前批18项再减本批442项，剩320项待审核/执行；内部worker入口须按合同驱动，不能直接空跑算通过。

普通Loom新Release壳SHA-256 `3CFD80A7225A00A65359C992F8674B9503C0F43B11DE09E79488A5F7CF03DCB6`，profile.test=false，沿用绑定SHA的已验前端，无前端重构/版本分配。Tauri fmt及受影响源码有效行164/236通过；strict all-targets Clippy仍仅原daemon.rs:229和file_io.rs:217两项诊断，整仓strict行数仍被ArtEditDialog.tsx的2026-10-31过期例外阻断，未加allow或伪造审批。首次224项launcher因Windows命令行长度在CreateProcess前失败，改成两组精确串行名后通过；GPU对照runner两次仅在启动前因ctypes类型/缺CREATE_UNICODE_ENVIRONMENT失败，修正后首次实际测试通过，原回执保留。

当前普通壳独立新profile实测：设置修复版空闲124.719秒/124点约6.156 CPU秒；另239.594秒/237点页面观测约55.891 CPU秒，主要在桌面壳与WebView2，daemon约2.109秒；前台MCP短样本同样升高，尚不能归因某一页面或判泄漏。Pong修复版124.719秒/124点未见订阅零样本，含约8秒MCP前台，整树约9.625 CPU秒；这些都是单Loom壳、不含Art/Live满负载，不能替代前批普通Otter+Loom合计Private最高529.840MiB/Working最高594.730MiB。三轮均Job兜底回收，不称自然退出或长稳。原页面runner误记整体uiInput=false；runner本身未输入，但Computer Use实际操作了自有导航。独立审计已纠正且原记录保留。

证据在`main-runtime-stability-540-1`的remaining-audit、三组runtime样本、pong测试/壳artifact、GPU对照及final-audit。普通可见Art/Live/capsule/完整端点键盘、人工IME/颜色文件模态、多屏/跨机/DPI、完整业务合计资源长稳及最终隔离整包/用户验收继续开放。不重开冻结副线/v464/用户release97，不启动Loom原生化、不提交推送安装部署发布；兼容维护和Talk构想排除，总迁移active。

## v540 表单换行热点修复与最新普通daemon

针对v539真实表单child的250ms失败，先复用现成production rlib定位，不盲重编库。原22case诊断一次全过但case12达229233µs；同原form工厂/树/事件与污染条件的隔离对照在case12真实失败（265241µs），候选完成32912µs。对照不是原测试EXE或普通产品验收，不能唯一还原旧失败当时的case/CPU调度。已证的32768段JS/RegExp往返热点改为两次捕获native split/join；分隔符冻结且无原型，仅自有固定toString，不查询插件Symbol.split/toPrimitive。原raw65536、UTF-8 48KiB、Unicode先完整验证、原子拒绝、250ms/32MiB heap/128MiB Job均保持。临时数组至多65537个连续自有字符串槽，中间字符串不超raw；没有新依赖、权限、线程或队列。

边界夹具从22扩至28case，增加CR密集、混合/尾部换行、48KiB边界/溢出、Unicode换行和不合法代理项；同时污染公开split/replace/join/exec、数字槽0/32768、Symbol.split/toPrimitive、toString/valueOf、constructor/species。当前新库原测试完成28case（case12为41476µs），另12项文档树/输入/焦点通过；解包后的真实产品worker同样完成28case（case12为12026µs），固定工作区driver6项通过，其中只有2项启动完整普通产品。按完整Rust测试名本批20项通过，不是整库，28case不当作28项Rust测试。原失败保留，不扩大期限、不添加ignore、不重跑刷绿。

一次分配v0.2.33.540；一次lib构建701.828秒，一次官方显式target候选构建680.203秒，未重复编host Release。官方fmt/版本/全仓strict行数、strict all-targets Clippy、JS语法、diff通过。3份源码有效行为74/64/64，无新文件/例外。首次量测调用传错language名、首次diff固定Git路径不存在的工具启动失败保留；改用官方languageForExtension与实际Git路径后通过，未重跑成功组。

官方内部ZIP、独立verify及9项归档合同通过，实际解包311文件/165份依赖许可，逐文件SHA一致，无ANGLE DLL。ZIP SHA-256 `A869D327A7DACC7136B2C1AF8E3DD0635A96F134F2361D25704BD38B213D7FDB`；EXE SHA-256 `166504FF0DC20E4549E4C430E518D61E8A920EF8F8AC12C290934A0CDB219B99`。未签名dirty内部开发包，不是正式发布或完整验收。

最新普通Loom daemon独立target release构建455.953秒，profile.test=false，SHA-256 `4D0F53415446ACA9808481B5C6705F18B81CB60F3D91F01641D9AA4386E40481`。实际新profile启动/认证目录接口/自有随机回环bridge/CLI通过。当前普通Otter+daemon+必要child在未切换desktop进行60秒idle/hidden/resumed后台基线，33点，Private commit合计12.289–12.695MiB，Working set合计70.754–70.988MiB（共享页可能重复计）。没有请求业务目录/建立Art业务连接，没有Loom桌面壳、GPU量测或Art/Live负载，不是长稳或完整双产品验收。Otter以自有HWND系统消息正常退出；daemon为自有Job兜底清理，不冒充其自然退出。

普通可见Art/Live/capsule/完整键盘、模态/人工IME、普通Loom桌面壳及完整业务联合、物理多屏/跨机/DPI、剩余全量/历史间歇、合计负载长稳及最终用户验收仍开放。仍取消全部新旧兼容维护/Talk构想；保全部dirty/untracked、旧失败、三冻结副线/v464/用户release97，不启动Loom原生迁移，不提交推送发布安装，总迁移active。证据main-form-budget-539-1/diagnostic-audit和main-form-fix-540-1。

## v539 追加回归与官方内部ZIP

本批未改产品源码、依赖或版本，复用v538 input7固定库并核3571项当前输入，只有7项文档/版本记录差异；编译源码等价。新增选定807个完整名，去重806 passed、1项真实失败、0 selected ignored。覆盖协议、原生量测、JS模型/真实child、网络生命周期、会话存储、自有菜单/控件、Tile状态及Live恢复；不是整库通过。原95项JS批次26 passed/69 failed，其中form_value真实child click在原250ms operation预算内失败，另外68项仅SERIAL锁PoisonError；只隔离这68项后全过，不重跑form或前26项、不改预算，原失败保留。具体case和CPU/调度原因仍未定位，不宣称已修复。

对原1745个未启动名实算：1743仍存在，2个旧fallback名已消失；44个后续日志命中只用于去重，未经本批完整receipt重认证；本批807项已终态，112项默认ignored未选，仍有780个当前普通名未启动。此为旧库存去重边界，不是全库完成率或780项缺实现功能。

现有host EXE缺官方构建前后工具/源码链，不能事后伪造接纳；本批同版本539只做一次真正成功的官方显式target Release构建，独占原target锁，919.484秒，源码/工具前后一致。前两次在Cargo前因隔离PATH/PATHEXT不能解析cargo失败；探针证实cargo.exe可解析但cargo不可，补固定工具目录及PATHEXT后成功，失败原件保留。没有重分版本或冒充旧EXE官方 provenance。

官方内部ZIP、独立verify及9项归档合同通过；真实解包311个文件、165个许可证依赖材料，逐文件大小/SHA一致，无ANGLE DLL。ZIP SHA-256 `403AF9E36643AF85836706EB2CD5605770FEF2C878BA015CE78203EA05A9D8D2`；解包EXE SHA-256 `544858911675A1B5CBBC55407120625847CEFB3B317265EA7EBED7D750EDD779`。固定workspace driver对解包EXE的6项通过，其中2项启动普通产品验证持续检查点/强制结束重开/删除最后Unit及系统消息取消/确认，另4项为绑定/枚举合同；另workspace-check及3项CLI拒绝通过。不是6次完整业务验收。产品profile/cwd/TEMP隔离；构建/许可证材料仍读取既有Cargo/Rust工具缓存，不宣称完全不访问用户工具目录。沿用32×32普通工作区PNG输入不重开PNG专项。

仅未切换desktop/自有Job，无普通前台输入。Job退出兜底不证明所有child自然清理，普通可见Art/Live/capsule/完整键盘、人工IME/模态、最新普通双产品/物理环境、全量/历史失败、合计资源长稳和用户最终验收仍开放。最新Loom daemon现有539产物是测试库，不能冒充普通daemon；旧524普通daemon不等价，需后续最小普通bin构建，不做Loom原生迁移。该ZIP为未签名dirty内部开发包，不是最终功能验收或正式发布。取消全部新旧兼容维护/Talk构想、保dirty/untracked和三冻结副线/v464/用户release97，总迁移active。证据：main-remaining-539-1的runtime-audit、unpack-audit和extracted-runtime-audit。

## v539 撤权取消与进程所有权

修复Loom runtime在本次调用惰性初始化成功、但Command派发前已取消时遗留新进程的路径；取得包锁后再次核取消。只回收本次初始化创建的child，不撤销未使用的共享预热进程，不新增线程、权限、依赖或扩大期限。初始化仍沿原manifest timeout，不声称可即时中断initialize。

原设备撤权HTTP测试混淆了submit受理与Command已运行：persistent插件启用时已预热，撤权早于派发可以合法留下共享空闲进程。现在先观察真实progress，再撤权；撤权后仍是原3秒、无poll的回收门。新增预热预取消保原PID/后续调用、真实初始化中取消/无Command marker/释放/再次调用，以及未使用设备撤权后共享插件可用三项测试。新增初始化回归在未修产品上真实失败，修复后通过；不能据此唯一还原v538旧现场的具体交错，原失败不改写。

本批按仓库/完整名去重 **65项相关Rust passed / 0 failed / 0 selected ignored**：runtime生命周期/权限/进程树39项、HTTP服务/图片/确认19项、当前v3包真实TLS/LAN与停服离线联合父门1项、普通固定SHA EXE工作区6项。联合父子不混加。HTTP新增测试首轮误期待403、实际撤销Session为401；仅修该cfg(test)断言、重建受影响daemon并重验该项，18项成功结果及未变runtime按精确delta复用，不重分版本。

普通Release EXE及workspace driver已集中构建；EXE SHA-256为`DB0FC290E6AD36DD0C7E656577A237419AFA7714724F5C50397A11EF4D1EFE03`。固定开发产物及输入位于本地main-service-cancel-539-1证据目录，不安装、发布或替换用户版本。Release实际构建1032.500秒；后续同源验收直接复用固定产物，不为测试轮次重新分版本或重复构建。不是最终隔离整包或普通可见双产品验收。

两仓fmt、官方版本、Otter全仓strict行数、15项行数checker测试、两仓diff通过；Otter strict Clippy复用v538 input7未变编译源码的回执。4份受影响Rust最大250有效行，无新例外。Loom全仓行数仍被ArtEditDialog.tsx的2026-10-31过期例外阻塞；runtime strict Clippy实际被未改动model_provider.rs一条、schema_tests.rs两条诊断阻塞，daemon strict Clippy未运行，不称两仓全绿。首次diff工具路径不存在的启动失败及其修正回执保留。

只分配一次v0.2.33.539；取消所有新旧兼容维护的决定保持。普通可见Art/Live/capsule/完整键盘、颜色文件模态/人工IME，最新双产品完整联合/物理多屏跨机DPI、剩余全量/历史清理、合计资源长稳及最终完整隔离包/用户验收仍开放。保dirty/untracked及旧失败；三冻结副线/v464/用户release97不动，不启动Loom原生迁移、不提交推送部署，总迁移active。

## v538 当前签名包与端口默认显隐

2026-11-04 用户明确取消全部新旧兼容要求：配套软件同步升级最新架构，不维护旧协议/模式/程序的协商、探测、降级、适配或新旧互通验收。此决定优先于下文历史兼容设计；历史失败及数据不删除，也不把兼容维护另列为迁移待办。

Local Art 主链现在只 POST `/v3/art/local/prepare-package`，协议 `loom.art.local.prepare-package.v3`，独立签名域 `loom.art.local.package.v3` 加 NUL。完整包包含原 grant/device key/resources、真实 ports 和按方向顺序一一对应的 visibility。显隐与端口共享原8 KiB预算，总签名/资源/期限不扩大；不再发现版本、遍历旧路径或因404重放。Loom Device准入、LAN白名单及16 KiB请求门均接当前路径；旧准备入口不再服务。

默认值只读当前单端口 `defaultVisible`，缺省显示，不读旧 `defaultVisibility` 映射。用户按名覆盖优先；默认值随包证明保存/重开和有限离线携带，隐藏不改变执行拓扑与权限。当前包内复用的 grant/resource/describe/service v1 合同仍有效，不能按版本字符串机械删除。离线、签名、审批/撤销、精确身份、资源预算及进程清理属于当前产品安全，不是兼容层。

本批相关门 **118项Rust passed / 1 failed / 0 ignored**（按仓库/完整名去重，联合父门只计一次，不混加子进程）。input5→6只改Loom旧入口负向夹具的403断言；input6→7只修Otter测试恢复helper，按原生产restore流程读取已保存坐标元数据、转换窗口并保留上下文，未放宽第二次保存准入。Otter库仅为受影响测试重建，其他未受影响成绩及Loom库/联合门按精确输入差异复用，不重编普通产品或重跑成功组。另9项签名安全结果以不变的security/protocol源输入和固定产物复用，1项默认ignored不算通过。普通隐藏Application、真实TLS/Device/LAN、signedPorts/signedVisibility、完整资源、正式结果与2×2像素、停Loom后新进程有限离线及缺资源拒绝已定向验证。不是整库、普通可见双产品或用户验收。

相关服务回归 `local_art_service_http_device_disable_cancels_without_polling` 本次仍失败：`revocation did not reap service process`。原失败保留，不以input4曾通过或Job退出后归零覆盖；没有定位根因、延长期限或重跑刷绿，作为剩余回收问题继续开放。本批实现与兼容范围决定不等于所有相关回归全绿。

官方两仓fmt、版本、Otter全仓strict行数、Otter strict all-targets Clippy及固定SHA测试库通过；Loom daemon库通过。Loom全仓strict行数仍被无关旧UI例外到期阻塞，strict Clippy仍有已有依赖诊断，不称两仓全绿。没有依赖/lockfile变更，没有普通Release EXE/workspace重编或新整包；最近已验证普通EXE仍为v537。本批不提交、推送、发布、部署、安装。

失败如实保留：input1测试clone编译问题、input3删除回退后的旧缓存死代码、input4测试模块位置lint；Loom旧入口拒绝测试误期待404，实际v1/v2均在Device准入层为403，input6只修该夹具；隔离runner缺rustc导致18项服务夹具失败，已以编译器/SDK白名单环境及最小探针修复，未放宽产品门。appearance混组时chrome夹具保留同名窗口class，Live夹具首次RegisterClass失败并引发第二项PoisonError；按已证夹具寿命分为独立进程，两项通过，不改产品。input6新保存测试漏登记恢复坐标，input7按真实恢复合同修夹具并重验直接依赖。行数汇总曾误与runner回执同名，原回执保留，以affected-lines-7-final.json为准；Loom行数旧阻塞exit2而非预期1导致runner拒绝，不改写原结果。input7集中验证后仅文档收口，不重分v538。

代码中仍有未清理的旧独立API及历史夹具，不对其承诺兼容，也不为其恢复服务端旧路由。后续只在实际调用点需要时直接迁到当前架构，不另开全仓兼容清理项目。剩余普通可见Art/Live/capsule/完整键盘、颜色/文件模态与人工IME，普通双产品完整联合、物理多屏/跨机/DPI，剩余全量/历史间歇与清理问题、合计资源长稳，最终完整隔离包及用户验收仍开放。

本地证据：`AI/GameEditor/linshi/otter-native-parallel-20261029/main-local-visibility-538-1` 的runtime/final audit与交接。前台仅新profile/自有窗口；本批没有普通前台输入。保所有dirty/untracked、旧失败、三冻结副线/v464固定包和用户release97；Talk构想及新旧兼容均排除，不启动Loom原生迁移，总迁移active。

## v537 普通贴图图片输入

普通sticker的Tab与image单字段child共用上传、清空、启用/禁用事务。文件在原唯一worker读取，真实容器及解码校验后提交；上游优先，未连线回手动图，禁用/停止接收/本地编辑回自身图。手动图不替换原画布显示；保存及后台恢复保配置、接收策略和未知字段。

单文件32 MiB、单边32768、100MP；PNG/JPEG/WebP/BMP与原MIME别名由专属准入处理。image_path是唯一大值例外，其余params保1 MiB/128项/深度节点限制；整会话64 MiB、共享512 MiB及执行准备256 MiB不变。无图片时移除两个图片owner键，避免合法旧128项或临界字节参数无操作保存失败；修改先借用计量再原子发布，拒绝不推进身份。clear后旧导入不复活，配置变化使旧执行/保存Receipt失效。

本批同源定向 **250项Rust passed / 0 failed / 0 ignored**，含16项新增；不是整库通过。官方fmt、版本、全仓strict行数、strict all-targets Clippy、固定SHA库、普通Release与workspace门通过。只分配一次v0.2.33.537；先集中实现与测试，再统一Clippy/产品构建。46项源码/版本delta，11新文件，Rust最大448有效行，UTF-8无BOM，无新增行数例外。input1编译类型/可见性/布尔检查失败、input2新增Map转换类型失败均保留。input3已完成所有组：一项新测试误将未完成草稿当可捕获输入，原断言失败保留；graph组误选一项既有默认ignored而被严格runner拒绝，62项实际通过不改判失败。input4仅修该cfg(test)叶的草稿/提交oracle并排除默认ignored；生产源码相对input3不变，不重分版本。

证据目录：`AI/GameEditor/linshi/otter-native-parallel-20261029/main-sticker-input-537-1`。产品SHA-256：`1BEA7C15AC4D850072CE4964082B60E6B51004F8D60B9A34B89A6145E3C39AE5`。产物路径见同目录runtime-audit.json，不安装或替换用户版本。

本批未启动普通前台交互，不以隔离HWND/消息回归替代普通可见上传对话框、键盘或人工IME。前台恢复仍仅限新profile、自有测试窗；保旧工具边界/失败及dirty/untracked，三冻结副线/v464/用户release97不动。Talk构想和旧Web/Tauri混用排除，不启动Loom原生迁移，不提交推送发布部署，总迁移active。

## v536 真正单字段端口内嵌控件

公共Art参数输入展开后可挂真正单字段原生child，复用EDIT、COMBOBOX、bool、步进、slider、颜色和文件控件及原参数事务；不是弹完整面板冒充内嵌。工作区仅一个活动editor，宽300逻辑像素，放图片左侧、空间不足翻右侧，两侧不足不挂，不覆盖图像。端口`Enter`/`Space`仍切换展开，`F2`从已聚焦参数端口进入控件，child内`F6`返回端口；文本Ctrl+Enter仍提交文本，不能误连线。linked有效值只读、secret不呈现；只允许已确认自身同步提交重建新Catalog后继显示，不继承旧草稿。

`chrome::capsule`拥有工作区单槽Rc候选、目录/宿主/parent身份及后继显示；`parameter_panel::window`拥有完整owned窗和单字段child的共同构造与资源寿命，`controls::capsule`仅筛选字段动作，复用原Snapshot/Spec和唯一参数提交。dirty/IME/pending纳入原chrome idle及保存/恢复/结束会话门。外部配置、隐藏、锁、捕获、端口显隐及失焦退休旧资格；同步focus/place/close/request返回只可取消同一Rc，旧栈不能撤销或覆盖后继。模态返回再次核panel owner及配置，旧panel不继续消费后缀；child继承端口条affinity，不对child调用顶层DisplayAffinity。没有新依赖、网络合同、权限、线程、timer、每端口HWND或像素/队列预算。

本批选定**98项Rust passed**，14项新增，选定终态0 failed/0 ignored；1项既有默认ignored未选、不计通过，非整库。覆盖真实child/单字段、多种公共widget、原子提交/linked只读、IME边沿/DPI草稿/父先销毁、region及图片外几何、单槽/输出/secret、dirty空闲门、自身提交后继、F2/F6/两段Esc、外部状态拒旧草稿、同步focus/place/close/request安装后继和退休owner拒发布；原chrome/参数/系统会话/Live输入取消依赖同批验证。官方fmt/版本/全仓strict行数、strict all-targets Clippy、固定SHA库、普通Release与workspace目标通过。23项源码/版本delta均UTF-8无BOM，6个新文件，Rust最大397有效行，无新增例外。input1首次Clippy因子模块可见性、测试未用导入和Stamp断言编译失败，原回执保留；input2只修这些编译问题，不重分版本。input2的chrome为47通过/1失败：后继回归夹具在同尺寸SetWindowPos后误等CHANGED；input3只在该cfg(test)分支先改变child高度以实际触发几何消息，原失败保留。guard回归验证返回后退休资格，不冒充真实颜色/文件模态人工验收。

前台限定恢复有效，仅新profile、自有测试窗。按用户本轮快速收口要求，只完成已启动批次的定向回归和产品构建，不重开与旧批次相同工具边界的普通前台尝试。本批没有启动新可见fixture或发送UI输入；普通可见capsule/颜色文件模态/完整端点键盘及人工IME未验，不能由隔离HWND回归推定通过。旧前台工具边界和清理失败回执继续保留。当前capsule仅公共Art参数；普通sticker的image上传入口及Local签名默认显隐元数据仍未完成，输出不提供参数editor。普通完整Art/Live/端点键盘及人工IME、普通双产品、物理多屏/跨机/DPI、剩余全量/合计资源长稳、最终完整隔离包及用户全面验收仍开放。Talk构想和旧Web/Tauri混用不列阻塞；三冻结副线/v464固定包/用户release97保持，不启动Loom原生迁移，不提交推送发布部署，总迁移active。

普通开发EXE SHA256 `3756120B86A66D315A5C498A34C4ED171C69C2C9CBA88D7877CB60CFC77A70CF`；本地证据main-port-capsule-536-1的runtime-audit/final-audit及交接。dirty开发候选，不是最终整包或正式发布。

## v535 端点键盘选源、连线与取消

Windows原生分支现在支持从当前文档/Live宿主按`F6`进入本节点可见端口，`Shift+F6`从末端口进入；端口条内`Tab`/`Shift+Tab`及方向键遍历当前工作区可见且可交互的端口，`Home`/`End`到首尾。`Enter`/`Space`切换名称展开，`Ctrl+Enter`从输出选择来源，或从已有连线输入选择原来源，再在另一节点输入提交。`Esc`先取消选源，再次退出；`F6`直接退出并回到仍属原owner的宿主。长按/松键不泄漏到文档删除或Live源输入，Ctrl+Shift+4也可从端口条操作。

`chrome::keyboard`独占线程内焦点、原宿主身份、Catalog和单槽选源；`keys`只识别精确键位、方向与松键位图。使用原每节点两条owned窗口，只SetFocus本线程已核窗口，不SetForegroundWindow、不注入全局输入。同步SetFocus前释放借用，返回核同一Rc候选；目录/显隐/锁/隐藏/捕获/宿主销毁和外部失焦退休，鼠标接管取消键盘选源。键盘选源计入原idle门；连线复用原Creation及唯一Application提交，提交前再核当前目录、两端owner与Workspace，取消保旧边。不新增依赖、协议、权限、线程、timer、每端口HWND或像素/队列预算。

本批选定**75项Rust passed**，12项新增，选定终态0 failed/1既有默认ignored，非整库。覆盖实际自有端点HWND焦点、遍历/首尾/反向、展开、长按和松键、唯一连线提交、重连取消、提交前锁定、隐藏端口、过期owner、捕获、外部失焦、鼠标接管及同步SetFocus重入；原chrome/参数面板/结束会话/Live输入取消直接依赖同批验证。官方fmt、版本、全仓strict行数、strict all-targets Clippy、固定SHA库、普通Release及workspace目标通过。9项源码/版本delta均UTF-8无BOM，3个新文件，Rust最大337有效行，无新增例外。首次runner因误选原默认ignored鼠标捕获项而拒绝，原回执保留；只读重解析原日志并核未改源码声明，未重跑成功测试、不把ignored计通过。

前台限定恢复有效，仅新profile、自有测试窗。本批只运行未切换独立desktop/自有Job回归；没有普通前台输入。v534工具未返回普通产品窗口的边界仍开放，不猜HWND、不换其他自动化API、不重试同条件失败，不将隐藏HWND通过称普通可见键盘通过。可编辑capsule内嵌控制、普通完整Art/Live/键盘及人工IME、普通双产品、物理多屏/跨机/DPI、剩余全量/合计资源长稳、最终完整隔离包及用户全面验收仍开放。Local签名默认显隐字段当前合同未提供；Talk构想和旧Web/Tauri混用不列阻塞。三冻结副线/v464固定包/用户release97保持，不做Loom原生迁移，不提交推送发布部署，总迁移active。

普通开发EXE SHA256 `E7E21CF722A4F2D23723A9B5DDC787244F2C1E7E418D120B2E2AFA11A8DFC409`。本地证据main-port-keyboard-535-1的runtime-audit.json、final-audit.json及交接；dirty开发候选，不是最终整包或正式发布。

## v534 端口默认显示、用户覆盖与保存恢复

端口显示现在支持Backend单端口`defaultVisible`优先、旧能力级`defaultVisibility[name]`回退、最终缺省显示；用户`portVisibility`按名称最高优先，同名输入/输出共用覆盖。Document节点菜单、参数面板和Live系统菜单共用原生“端口显示…”入口，列表可选使用默认值/显示/隐藏，取消丢弃草稿。参数面板先原子提交有效文本，再重绑定本次后继操作。隐藏端口从绘制、稀疏region和命中移除，可见位置紧排但保目录索引；原连线/执行权限/拓扑不变。

`workspace::ports`拥有有界不可变覆盖和独立`AppearanceStamp`；保存Receipt/dirty/恢复Guard及UI Catalog核外观身份，执行图仍只沿原拓扑/配置凭证。`chrome::ports`拥有单槽Open及Application模态前后准入，dialog/model/template只持草稿和标准控件；重新核owner marker、锁/隐藏/结束会话/捕获/退出/保存恢复及目录后才提交。保存用当前覆盖替换旧字段，reset删除旧字段，有外观编辑的初始空宿主不再省略。Live快照和同ID冻结沿同一Workspace保存，不另造State副本。没有新依赖、协议、权限、线程、timer、每端口HWND或像素/队列预算。

本批选定**84项Rust passed**，21项新增，选定终态0 failed/0 ignored，非整库。覆盖中文/非法/重复/null/容量、缺省优先级、外观独立、真实LISTBOX/COMBOBOX通知、取消/关闭/DPI/affinity/字体回收、单槽/过期owner与Catalog、实际参数按钮事务、隐藏端点和原边保留、保存重开/未知字段/reset及Live真实worker保存/Receipt失效/冻结后再保存重开。官方fmt、版本、全仓strict行数、strict all-targets Clippy、固定SHA库、普通Release与workspace目标通过。39项源码输入delta均UTF-8无BOM，10个新增文件，受影响Rust最大497有效行，无新增例外。

input1模块可见性/缺失x64指针索引和测试越层访问、input2Win32强类型索引、input3测试clone lint失败回执均保留；input4只收窄所属接缝并改用正确类型/借用，没有放宽门禁、预算或期限。

前台权限仅本批新profile、自有测试窗。普通Release使用预置隐藏output覆盖的独立profile，本次没有取得普通产品可见外观证明；工具输入调用为0，不操作用户旧窗口、不注入全局输入、不改显示设置。该观察不冒充普通产品对话框交互/完整键盘或人工IME；本次自有Job终止后的原5秒父进程等待失败，未通过严格清理/自然退出门；随后只读核本批产品残留为0，不重放终止或UI，也不以稍后归零覆盖原失败。

Local v2签名端口当前没有默认显隐元数据，仍缺省显示，不从未签名提示伪造包默认，本批不改Loom/wire。可编辑capsule、完整端点键盘、普通完整Art/Live/键盘与人工IME、普通双产品、物理多屏/跨机/DPI、剩余全量/合计资源长稳、最终完整隔离包及用户全面验收仍开放。Talk外部会话控制构想及旧Web/Tauri混用按用户决定排除。三冻结副线/v464固定包/用户release97保持，不启动Loom原生迁移，不提交推送发布部署，整体迁移active。

普通开发EXE SHA256 `11D3EDC8CCDCD5F9E02987625BC43E37D9F79A57BBEB6CA70E32D8FC80F68F78`。本地证据main-port-visibility-534-1的runtime-audit.json、final-audit.json、STATUS/NEXT_SESSION；dirty开发候选，非最终整包或正式发布。

## v533 Local Art 签名自定义端口

Local Art 现在从 Loom 已验证的不可变安装包取得输入/输出的真实名字和类型；Otter 接入目录、图像输入、来源配置凭证、自定义主图片输出继续 Add Art、正式结果及保存重建/有限离线。显式零输入/零输出保持为空，不伪造 input_image/output_image；只有无新证明的旧包保留默认端口。首次自定义主输入迁移在候选 Workspace 内提交，冲突保原图；零输入只撤回本次工厂创建且仍完全一致的暂定边，保存恢复不继承删除旧边的权限。端口保存记录仅是包绑定提示，不是执行/离线授权。

新增精确 Device POST `/v2/art/local/prepare-package`、请求 `loom.art.local.prepare-package.v2`，签名 payload 为独立 `loom.art.local.package.v2` 域，严格包含 protocolVersion、原 package 和 ports；旧 package.v1 与原权限/设备/审批/包/资源/期限保持，不在同协议偷换语义。ports.inputs/outputs 每侧至多32项，名字UTF-8至多128字节、类型64字节，不接受空白/控制字符/同侧重名；端口JSON至多8KiB，并与资源共享原签名payload预算。

执行前沿原Lease/Control读取 `/v1/art/local/issuer`，新增可选 `supportedPrepareProtocols`；响应至多4096字节，必须与独立可信Pin的algorithm/keyId/公钥一致，不从网络自动授信或新建Lease。旧响应缺能力直接走v1，不能先探测未知v2路由；advertised v2 后任何拒绝（包括404）都失败关闭，不降级重放。原v1 package到resume的精确404回退仍保留。

`art_execution_ports`和`art_local_ports_record`拥有独立证明及包绑定记录；`document_local_art::Config`只拥有Local身份、共享Arc记录和本次暂定边，非Backend selected_art。`desktop/art_local_ports`独占候选拓扑发布，并在无Win32/网络/脚本回调区更新依赖和配置凭证；`application_art_source`用真实图片输出名及配置Stamp围住连续Add Art。Loom `source_ports`只投影resolver验证的不可变manifest，`local_art_ports`与签发器复核预算/独立签名域。没有新依赖、权限、常驻线程、timer或扩大像素/队列预算。

本批最终选定 **83项Rust passed**，其中17项新回归；当前选定终态failed=0，不是整库。签名域/篡改/预算、旧能力协商、拒绝不重放、图拓扑、中文自定义输入、真实2×2红像素、Live配置换代拒迟到、零输入不交付像素、自定义rendered输出继续建子Art及Save/窗口重建/有限离线均已验。真实Loom LAN HTTPS跨进程联合在线和停服后离线独立核signedPorts、完整资源、正式像素和正式结果；不完整缓存拒绝，父子不混加。Otter strict all-targets Clippy、两仓fmt、官方版本/strict行数、Otter lib/普通Release/工作区与Tile驱动构建通过；Loom签名安全/daemon测试库通过。66项源码delta均UTF-8无BOM，Rust最大500有效行，无新增行数例外。两个原跨仓grant门及两个SDK示例默认ignored未运行；Loom整仓strict Clippy旧债未重跑，不称两仓全绿。

| 选定Rust组 | passed |
| --- | ---: |
| ports-proof | 21 |
| ports-tls | 10 |
| ports-discovery | 2 |
| ports-topology | 3 |
| local-regression-01 | 1 |
| local-regression-02 | 1 |
| local-regression-03 | 1 |
| local-regression-04 | 1 |
| local-regression-05 | 1 |
| local-regression-06 | 1 |
| local-regression-07 | 1 |
| local-regression-08 | 1 |
| local-regression-09 | 1 |
| local-regression-10 | 1 |
| local-regression-11 | 1 |
| local-regression-12 | 1 |
| local-regression-13 | 1 |
| local-regression-14 | 1 |
| local-regression-15 | 1 |
| local-regression-16 | 1 |
| local-regression-17 | 1 |
| local-regression-18 | 1 |
| configuration-regression | 3 |
| ports-application-01 | 1 |
| ports-application-02 | 1 |
| ports-application-03 | 1 |
| ports-application-04 | 1 |
| issuer-regression | 8 |
| daemon-ports | 2 |
| daemon-package | 3 |
| lan-art-boundary | 1 |
| workspace-runtime | 6 |
| real-loom-package-parent | 1 |
| ordinary-visible-mixed-parent | 1 |

input1/2编译与lint失败、input4首项5秒等待/后续锁中毒及三新Application等待、daemon测试错读顶层code均保留。独立socket实验证明IPv4-only peer的localhost重解析先::1并各等约2秒，新增GET使原测试5秒门被跨过；input5只把cfg(test) peer改用既有显式127.0.0.1/SAN入口，daemon测试读原结构化error.code。连续Art测试另用Authority.replace保留两origin，不放宽产品撤权。未延长原期限、改系统网络或吞错误。input5新保存重建测试还误用未钉住的1000×720主窗作为2×2源，恢复后却要求仍按2×2导出；input6改为真实2×2钉图，并独立断言保存w/h仍为2，保原像素oracle，不改生产保存/导出语义。不相关成功组按完整cfg(test)差异和固定SHA复用，相关Application逐项独立进程重验，不伪称全部重跑。可见混合首轮runner只查TEMP目录却漏查Windows实际TMP目录，未收READY、未执行激活，原30秒父门失败；原日志/READY/严格Job回收保留。仅修本地runner覆盖TEMP/TMP，以同一产品/driver SHA和原期限在全新目录补验一次，不重编产品。

用户已恢复仅新profile、自有测试窗口的前台验收。本次普通固定SHA Tile EXE三类图片/Live/Art混合可见门通过；Computer Use只激活工具唯一返回且路径/标题匹配的本批标准启动器，刷新成功后用单会话nonce同步，driver再核真实前台并只向已核自有Tile转交。独立桌面采样核三路换帧、冻结/黑场/恢复、正常关闭及来源附件/值/Live保留与连接回收，不以后台paint或源像素代呈现。没有操作用户旧应用、注入全局输入或修改显示设置；不等完整键盘/人工IME或普通源EXE全部通过。

逐端口显隐/默认值与持久化、可编辑capsule/完整端点键盘仍未完成；普通完整Art/Live/键盘与人工IME、普通双产品、物理多屏/跨机DPI、剩余全量/合计资源长稳、最终完整隔离包和用户全面验收仍开放。Talk外部会话控制构想和旧Web/Tauri混用按用户决定排除。三冻结副线/v464固定包、用户release97保持；不启动Loom原生迁移，不提交推送发布部署，整体迁移active。

普通开发EXE SHA256 `4A120BD6D3629228DE851B3D9DEC2AE081095B2BFECD2BF43490FC8C179C802D`。本地证据批次main-local-art-ports-533-1包含runtime-audit-verified.json、visible-independent-audit.json、final-audit.json及STATUS/NEXT_SESSION，实际EXE位置仅记于本地回执；dirty开发候选，不是最终整包或正式发布。

## v532 Live 源 Add Art 与独立子节点

Live窗口的精确Shift+Digit1和系统菜单“添加 Art”复用原搜索目录窗；原Live保持存活，创建独立子Art和真实Live→Art边，不冻结或转换源。Backend保默认参数和Upstream自动门、Live快照及原输入worker/TLS执行；Local仅沿显式HTTPS、lease/issuer、describe→prepare→签名child原链。打开选择前沿原通道停止源输入，取消/来源或目录/分组/工作区/会话变化及容量失败保源，不留下半条边。

`application_add_art::source`统一文档marker与Live稳定Arc输入身份，拒绝同ID冻结后的文档继承旧Live资格；`application_live_add_art`拥有Picker/来源围栏，共用唯一Backend发布事务与原Local链。`art_picker_dialog::Context`只在同步DialogBox栈上借用allowed/session，不给Live伪造State或保存Application裸指针。`composition_live_art`只停止源输入并排单槽意图，Live停止/保存/恢复/退出/系统会话空闲门明确纳入它；`capability_catalog::request_for`复用唯一目录任务。没有新依赖、协议、权限、线程、timer或像素缓存。

最终input4同源定向 **62项Rust passed / 0 failed / 0 ignored**，含9项新测试，不是整库。真实owned目录窗/搜索/IME合成/affinity、独立子节点/默认值、取消/过期/容量、输入停止/单槽/空闲门、Backend真实回环TLS执行和Local签名child原始2×2红像素/源关闭后迟到拒绝已验。官方fmt/版本/全仓strict行数、strict all-targets Clippy及lib/普通Release/workspace三目标构建通过；21份Rust最大496有效行，没有新增行数例外。只分配一次v0.2.33.532，Loom源码未改。

| 精确选定Rust组 | passed |
| --- | ---: |
| new-live-add-art | 7 |
| new-live-local | 1 |
| new-live-window | 1 |
| picker-regression | 9 |
| document-add-regression | 7 |
| live-link-execution-regression | 11 |
| local-input-regression | 8 |
| session-regression | 12 |
| workspace-runtime | 6 |

input1私有方法/Option编译错误、input2 nonminimal_bool和input3测试失败均保留。input3首帧测试误把原自动等待当手动缺帧错误，另两项共享锁中毒；隔离补证来源复核通过，会话测试另证缺生产with_owner scope。input4仅修新测试：保自动等待与精确手动错误oracle，正确安装query/end owner并断言pause/resume；没有改变生产策略、放宽断言或提高期限。

仅新profile、自有窗口的前台恢复授权仍有效。本次以新空profile启动普通固定SHA产品，而非重放旧image/chrome启动。Computer Use的list_windows只返回一个无标题自有对象，其截图包含透明层下其他应用内容，accessibility为null；list_apps重新枚举仍只有同一自有对象，并混列用户release97。没有可独立选择的自有普通文档/Art/Live目标，故未激活或发送UI输入，没有选择用户release97，不改样式/捕获保护/输入法或换API绕过；不判定产品显示根因。 自有Job保底清理不等于自然退出；普通前台与人工IME不从隐藏测试推定。

Local 自定义主输入名仍缺完整目录/图/状态/保存恢复接线；逐端口显隐及默认值/持久化、可编辑capsule和完整端点键盘仍开放。普通Art/Live/完整键盘与人工IME、普通双产品、物理多屏跨机DPI、剩余全量/合计资源长稳、最终完整隔离包和用户全面验收仍须继续。Talk构想与旧客户端混用排除，三冻结副线/v464固定包保持，不启动Loom原生迁移，总迁移active。

普通开发EXE：`C:\Users\Public\nas_home\AI\GameEditor\linshi\otter-native-parallel-20261029\main-live-add-art-532-1\artifacts\input4\otter.exe`，SHA256 `2CDFDB9E52B5EA4C1AE46742A7BCB1A2EBCBE019B8430163DFDEB9AF39202482`。证据main-live-add-art-532-1/runtime-audit.json、final-audit.json及STATUS/NEXT_SESSION；这是dirty开发候选，不是完整隔离整包或正式发布，无提交推送安装部署。

## v531 工作区三态与跨窗端口连线

原生工作区新增独立Rc chrome owner：Ctrl+Shift+4按all→border→none循环，Unit外框、公共端点及端口名称展开、5 CSS px拖动阈值、跨窗拖线和已连接输入重接接入原唯一拓扑提交。三态同时控制Live旧绿框/黄角及其命中，none不留下旧Live框；参数窗内组合输入不抢键。共享提交重新核Catalog、端口、工作区及两端owner，保文档自身锁/隐藏和Live组门；失效或cycle不删旧边。**重接明确采用有效drop才替换、空白或取消保旧边的安全语义**，不同于legacy拖动开始即删旧边，不冒称旧取消行为完全相同。

一张始终穿透的GDI线层和每节点至多两个owned端点条，不按每端口建HWND；稀疏region以外不占输入。scene只保目录/索引/标量，不复制图片，不持Application或文档可解引用裸指针。复用相同Catalog的连线图及宿主几何消息更新；隐藏时构图仍保边、绘制才过滤，恢复不丢线。拖动仅扩线层范围，不按每次move重建端点region；线层及端点条各限33,554,432像素，超限保文档并给必要短错误。混合线层按最严格捕获affinity，端点继承各自宿主；模态/捕获/恢复/保存/退出暂停、宿主销毁、Rc弱引用与系统会话空闲门均有接线。没有新依赖、协议、权限、线程、timer或全局输入。

最终input5同源定向 **90项Rust passed / 0 failed / 2默认ignored**，其中19项新chrome回归全过；不是整库。官方fmt/版本/全仓strict行数、strict all-targets Clippy及lib/普通Release/workspace三目标构建通过。29份受影响Rust最大500有效行，没有新增行数例外；30个源码/版本路径，Loom源码未改，版本只分配一次v0.2.33.531。

| 精确选定Rust组 | passed |
| --- | ---: |
| workspace-chrome | 19 |
| link-regression | 8 |
| live-chrome | 15 |
| panel-regression | 30 |
| session-regression | 12 |
| workspace-runtime | 6 |

真实自有HWND验证owned/稀疏region/捕获affinity、直接几何跟随、100次同Catalog复用、隐藏恢复、精确快捷键/长按抑制、点击展开、实际落点恰一次提交、重接取消保边、捕获暂停和销毁清理；Live GPU像素验证去框/恢复且源帧不污染。两个既有Live显式门仍默认ignored（交互桌面真实最小化与鼠标捕获），没有计为通过。库运行均在不切换的自有desktop/Job，不代表普通前台或人工IME。

input1编译、input2模块遮蔽/collapsible_match、input3测试slice写法失败均保留。input4严格静态/lib已过，但19项chrome为18过1失败：夹具在layered端点首paint前即要求WindowFromPoint命中。独立custom-class Win32实验实证paint前命中host、paint后命中strip、透明topmost不挡；首次STATIC实验因其透明命中特性不能用于该结论。input5只给cfg(test)夹具补真实UpdateWindow，生产字节未变，保原断言/失败；新19项重验全过。Live组Rust进程exit0/无失败，旧通用runner只因遇到2个默认ignored而拒绝；保留原result及interpretation，不刷绿或重跑已过15项。

用户自有前台恢复授权有效。本次普通固定SHA EXE已启动，但Computer Use仅返回一个本轮候选，截图包含其他应用内容且没有可访问性树；重新枚举仍无独立自有文档/端口目标，未激活、未发送UI输入，不选list_apps混列的用户release97，不改样式/捕获保护或换API。**普通可见三态/连线、Art/Live和完整键盘仍未验；工具边界原因未定位，不据此判定产品显示失败。** 自有Job保底清理后active=0，forced=true，不证明本轮自然退出。

per-port显隐/capability默认值及持久化、可编辑端口capsule和完整端点键盘仍缺；Live源Add Art、Local自定义主输入名、普通双产品/人工IME、多屏跨机/DPI、剩余全量及合计资源长稳、最终完整隔离包和用户全面验收继续开放。Talk构想、旧混用、三冻结副线/v464不重开，总迁移active。

普通开发EXE：`C:\Users\Public\nas_home\AI\GameEditor\linshi\otter-native-parallel-20261029\main-workspace-chrome-531-1\artifacts\input5\otter.exe`，SHA256 `BC147253F9C49E1ABEFF515C4BB0D2F175470B8C8CB47CCEC9F49FA3E8515A16`。证据在同批runtime-audit.json、final-audit.json及STATUS/NEXT_SESSION；不是完整隔离整包或正式发布，未提交/推送/安装/部署。

## v530 自有前台恢复：可见参数与自然退出补证

用户已明确恢复仅新profile、自有测试窗口的前台验收；该授权优先于下方v530物理Escape历史停点。不操作既有应用窗口、无无目标全局输入、不改显示/输入法设置，工具越界/停止仍立即停。复用原input3固定SHA普通Release，不重分版本、不重编或重跑已通过Tile父门。
本次实际观察四象限图片和原生工具面板；裸Tab后可访问性树出现钉图、重新加载、接收上游图片及输入image/输出output。未取得可独立选择的owned参数窗及完整控件/视觉证明，不算完整Tab验收。点击已观察的自有文档标题栏关闭后，产品和Job在关闭Job前自然active=0；forced=false，gracefulProductShutdownProven=true，只证明本轮普通文档自然退出，不覆盖旧v527清理失败或其他Live生命周期。
Ctrl+E与Shift+1均未取得可确认的产品响应。独立自有Tk诊断窗只记录自身事件：Tab送达；两种Ctrl+E写法没有本地Ctrl/E事件，Shift+1记录65281/229及49的keyup，而非预期49的keydown。此为当前送达/IME路径异常证据，不足以判断原生产品收到预期快捷键，更不能宣布产品根因修复；没有改输入法、全局热键、窗口样式、捕获保护或使用另一输入API绕过。普通Art搜索/添加、完整键盘/人工IME仍未验，不能把隐藏测试或本诊断当作通过。
证据位于linshi/otter-native-parallel-20261029/main-foreground-resume-530-1，含ui-observations.json、两个自有Job终态、key-events.jsonl和final-audit.json。没有产品源码/版本/依赖变化；原69项定向Rust及构建成绩仍只属于原v530回执，未新增运行计数。Live源Add Art、Local自定义主输入名、三态外框/可视端口连线、普通双产品/物理多屏DPI、剩余全量/资源长稳与最终整包/用户验收继续开放，整体active；Talk构想、三冻结副线和v464不重开。

## v530 Shift+1 原生 Art 入口

精确Shift+Digit1已接原生同步owned列表，搜索基于原完整能力目录、保原index，disabled拒绝添加；刷新退出后才交付原异步目录任务，模态不联网。选择后创建独立Art和源连接，不转换源图/源Art；普通能力保原默认参数、主图片输入选择及源notify/子listen/upstream自动门，不制造Manual执行。Surface只消费已显式选择的HTTPS origin，并走原本地describe/验签/审批链，WSS目录不派生HTTPS权限；同Art也强制创建子节点而非重启源。
候选窗口/拓扑先准备，发布前复核工作区、源HWND/State/configuration/MenuTarget、目录Rc及显式origin/会话状态；失败回收候选且不留半连接。每文档单槽意图纳入restore/系统会话/退出准入；真实EDIT/LISTBOX、首显前捕获排除、DPI保草稿、IME合成边沿和嵌套模态暂停过滤有定向保护。
同源定向 **69项Rust passed / 0 failed / 0 ignored**，含16项新增；官方fmt/版本/全仓strict行数、strict all-targets Clippy及三目标构建通过，不是全库。普通前台状态`STOPPED_BY_USER_PHYSICAL_ESCAPE_BEFORE_UI_INPUT`：Computer Use 在首次 list_windows 时报告用户按下物理 Escape，本轮前台验收立即停止；未选择窗口、未发送任何 UI 输入，普通产品 Art 模态/搜索/键盘仍未验。 合成IME/隐藏窗口不代人工输入法或普通Art完整业务，兜底Job清理不代自然退出。
本批只覆盖普通图片/Art文档源的Add Art入口；Live窗口源的Shift+1尚未接通。Local沿既有input_image/output_image合同，未实现任意目录自定义主输入名Surface的泛化映射；不能据此宣称完整Add Art对等，也不将metadata或只读端口摘要冒充完整可视端口/连线。Ctrl+Shift+4三态chrome、参数外围/候选预览/Live发布观看、普通双产品/物理多屏跨机DPI、剩余全量/合计资源长稳及最终整包和用户全面手测仍开放。Talk构想、旧Web/Tauri混用、三冻结副线/v464固定包不重开，总迁移active。详见[构建记录](WINDOWS_NATIVE_BUILD.md#v530-shift1-原生-art-入口)。

| 精确选定Rust组 | passed |
| --- | ---: |
| add-art | 16 |
| art-contracts | 6 |
| automatic-chain | 5 |
| session-regression | 12 |
| local-node-regression | 1 |
| parameter-panel | 23 |
| workspace-runtime | 6 |

只分配一次v0.2.33.530，20个源码/版本路径；保全部dirty/untracked、Loom源码未变。input1的Win32 Option参数/EM常量导入及测试借用编译失败、input2的测试callback type-complexity失败字节和回执均保留，未增加allow或放宽门禁。runtime首次预检因Local回归完整名前缀不匹配而在启动任何产品测试前失败；已保留脚本/清单SHA和失败回执，按真实naming_import路径修正，复用同SHA清单，不重编产品或覆盖失败。Local最终目录/origin/会话围栏和DialogBox嵌套输入冻结均在最终冻结前补齐；原版从源传播的合同保留。
前台只用本批新profile/合成四象限/固定SHA普通Release，未操作用户release97或既有应用、无无目标全局输入/显示设置更改；Computer Use 在首次 list_windows 时报告用户按下物理 Escape，本轮前台验收立即停止；未选择窗口、未发送任何 UI 输入，普通产品 Art 模态/搜索/键盘仍未验。 自有Job原5000ms清理门通过，gracefulProductShutdownProven=False，不覆盖v527旧失败。跨DPI和人工IME的物理表现仍未验。
普通开发EXE `C:\Users\Public\nas_home\AI\GameEditor\linshi\otter-native-parallel-20261029\main-add-art-530-1\artifacts\input3\otter.exe`，SHA256 `58032E01DEA75584E90DAD0DEFB6A2E59C6D57011734C269623C34C4D328C94B`；不是完整隔离整包或正式发布。证据main-add-art-530-1/runtime-audit.json、final-audit.json及STATUS/NEXT_SESSION；整体仍active。

## v529 Tab 原生参数编辑窗

宿主裸Tab已接到文档专属、惰性modeless参数窗；普通文本/数字/JSON为真实EDIT，选项保原Value类型，Slider/step、禁用/重置和执行设置复用原业务。颜色/文件仍走原显式对话框，不新增执行worker或参数状态。最多4个面板、每页16行、4项意图；同页文本先完整验证到候选，再取消旧执行并原子发布，同次Run可升级当前参数触发为Manual。后缀选择按参数id/动作合同唯一重绑定，不沿用可能移位的旧索引。
外部配置/上游更新及DPI不能静默销毁dirty/IME；旧凭证仍拒绝，明确重新加载才丢草稿。已知旧连线视图不能当成无连线；捕获保护在首次显示前继承，失效HWND按userdata marker拒绝，销毁同步撤捕获登记，托盘/组隐藏不自动复活草稿。
同源定向 **91项Rust passed / 0 failed / 0 ignored**，含23项新增；官方fmt、版本、全仓strict行数、strict all-targets Clippy及三目标构建通过，不是全库通过。普通前台已观察Tab参数窗的真实可访问性，但工具未返回独立owned目标，按钮点击被边界拒绝；状态`PARTIAL_UI_OWNED_WINDOW_OUTSIDE_TARGET_BOUNDS`，完整视觉/面板控件键盘/人工IME仍未验，不修改样式或捕获保护迎合工具。自有Job严格清理通过，未证明自然退出。
本批不是完整Tab对等：常驻可视端口/跨窗连线、候选预览、Live发布/观看面板仍开放；只读端口摘要不冒充连接画布。Ctrl+Shift+4三态Unit chrome、Shift+1 Art入口、其余N01–N12、普通双产品/物理多屏跨机DPI、剩余回归/合计资源长稳和最终整包/用户全面验收继续；Talk构想、旧混用、冻结副线及v464固定包不重开，总迁移active。详见[构建记录](WINDOWS_NATIVE_BUILD.md#v529-tab-原生参数编辑窗)。

| 精确选定Rust组 | passed |
| --- | ---: |
| parameter-panel | 23 |
| parameter-contracts | 9 |
| execution-regression | 5 |
| panel-regression | 16 |
| session-regression | 3 |
| panel-and-checkpoint-supplement | 29 |
| workspace-runtime | 6 |

源码只分配一次v0.2.33.529，共34个源码/版本路径；保全部既有dirty/untracked，Loom源码未改。input1原11个编译错误及失败字节保留；input2修SS_LEFT零样式、共享字体可见性、ODS_FLAGS位判断及affinity输出类型，并在冻结前补齐事务/句柄缺口。只读交叉审查发现的JSON pretty超限导致choice移位和tray裸HWND漏网，均有本批修补/相关回归，不用后来成功覆盖原失败。
本次前台仅本批新空profile/固定SHA普通产品，主窗画面身份已核；Tab后可访问性包含钉图、重新加载、接收上游图片和真实端口名。UIA按钮位于主窗bounds之外，点击被工具明确拒绝；list_windows仍只有主窗，不猜HWND、不改产品样式/保护、不借其他API重试面板输入。未成功编辑参数、未验证Art参数视觉/焦点/Enter/Esc/真实IME，普通产品完整键盘不判通过。没有操作用户release97/其他应用、无无目标全局输入或显示设置更改。
自有4进程Job兜底清理active=0、原5000ms等待通过；这不是自然产品退出，不能覆盖v527旧清理失败。普通开发Release EXE `C:\Users\Public\nas_home\AI\GameEditor\linshi\otter-native-parallel-20261029\main-parameter-panel-529-1\artifacts\input2\otter.exe`，SHA256 `E5FE9E7C059E75281C8DB43C65B7049BFCB2A73F3DDD4F1A261BE61B544DB48C`；不是完整隔离整包或正式发布。证据main-parameter-panel-529-1/runtime-audit.json、final-audit.json、STATUS/NEXT_SESSION；整体仍active。

## v528 Ctrl+E 工具面板键盘闭环

原版README承诺的Ctrl+E在原生缺接线，本批复用现有编辑工具面板完成，不新建另一套工具/参数框架。普通宿主按Ctrl+E显隐，面板及子按钮焦点下同键/F4/Esc沿原WM_CLOSE隐藏并把线程焦点交回同owner；显示使用SW_SHOWNOACTIVATE，不请求全局前台。首次按下才显隐，bit30重复消息消费但不动作；F4兼容保留，普通E/Shift/Alt修饰、Tab、Art、clean view不冒充同一入口。新模块核lifetime、双方GWLP_USERDATA和当前panel，fit同步重入后复核，截图/共享模态/组锁隐藏/系统会话冻结拒绝动作；未改变图像、文档、工具选择、权限、依赖、线程、持久化或外部协议。

| 精确选定Rust组 | passed |
| --- | ---: |
| 新键盘纯合同与真实隐藏宿主/子按钮/门禁/身份/清理 | 7 |
| 原工具面板内容、布局及可用性 | 16 |
| 会话owner | 3 |
| 固定SHA普通Release工作区/系统会话/产品绑定 | 6 |

合计 **32 passed / 0 failed / 0 ignored**，非整库。官方fmt、版本、strict全仓行数、strict all-targets Clippy及lib/普通Release/workspace构建通过；一次v0.2.33.528，公共版本未变。workspace固定产物按target.name/kind/profile.test选择，不重复v527的多产物提取假设。源码只改五路径（含两个新增叶和内部版本），desktop/mod.rs未增长；保全部dirty/untracked，Loom未改。早期baseline脚本误用不存在的Loom同级路径而exit=1，先前八份备份/输入已完整保存，之后只读核字节并补正确仓库状态；失败回执保留，不原样重跑。

普通前台只用新空profile/本批固定SHA EXE，由Computer Use选真实返回的自有窗口，当前 `UNVERIFIED_CAPTURE_IDENTITY_MISMATCH`；未操作用户release97或其他既有应用、不猜HWND、不改窗口样式/保护/显示设置、不注入无目标全局输入。具体观测见visible/ui-result.json和tool-observation.json；严格清理门 `True`，普通正常退出证明 `False`，后续成功不覆盖v527旧5000ms失败。普通前台完整键盘未宣称通过。

普通开发Release EXE `C:\Users\Public\nas_home\AI\GameEditor\linshi\otter-native-parallel-20261029\main-toolbar-keyboard-528-1\artifacts\otter.exe`，SHA256 `410C442A61AC880955300712AD3C91D9EBB009C32E40C66BC04896242D1A12BA`；不是新完整包或正式发布。证据main-toolbar-keyboard-528-1/runtime-audit.json、final-audit.json、STATUS/NEXT_SESSION。Ctrl+Shift+4三态外层chrome和常驻可视端口/连线仍缺原生对等owner，Tab参数/Shift+1 Art入口未补，不将标注handles或上下文菜单冒充完整UI对等。其余前台/物理设备/完整回归/合计资源长稳/最终整包和用户验收继续，总goal active；Talk构想、旧混用、冻结副线/v464不重开。

## v527 原版 Ctrl+H 截图与取色历史

补齐原版已实现的同屏两列表历史，不是系统剪贴板历史或两模式互切。成功截图提交及成功取色才入栈，各最多24条；颜色按hex去重刷新，截图按Unit身份替换。截图保存原始宽高，但持久化/复用的是最长边240且不放大的PNG缩略图，不包含原图访问权或编辑状态；普通打开、粘贴、取消/迟到/失败及历史复用不重复入栈。复用截图建立独立新钉图，颜色复用设置当前色并复制hex，复制失败不回滚颜色。删除须确认且复核原条目，只删历史，不删原图/画布单位。

`capture_history`拥有有界数据，image/store叶分别拥有共享Pixels预算下的240复用/64显示预览和独立原生profile的history.json。文件独占、完整字节竞争基线及原Replacement/Staged事务保护旧源；坏JSON、非对象、超过8MiB、竞争写或恢复文件未退休均停止后续保存，不覆盖旧源。只解码嵌入图片，不执行URL/路径/SVG；暂态预览失败保留原条目/原data URL，后续取色保存不能静默丢截图。不自动读取或迁移旧Tauri目录，不新增依赖、协议或授权。

desktop的ui/worker/window和application history叶分别独占单槽意图、单个串行惰性后台任务、modeless普通owned窗及应用接线；回调不读盘/建线程，Win32同步调用外释放RefCell借用。Origin绑定UnitId和原宿主Rc，旧宿主动作不得落到恢复后的新文档；截图队列最多4份共享Image，退出先排空/join历史再关闭wake。标准按钮提供名称和Tab/Enter/Space，Esc/Ctrl+H仅关闭面板；按压期间不换条目身份，DPI消息应用建议RECT后重排。

| 本批精确验证 | passed |
| --- | ---: |
| 历史数据、兼容JSON、240尺寸/alpha及保数据事务 | 8 |
| 后台槽、隐藏历史窗及真实隐藏Application | 8 |
| 成功取色/取消/旧宿主接缝 | 2 |
| 原打开/批次/粘贴回归 | 28 |
| 会话所有权回归 | 3 |
| 固定SHA普通Release的工作区/系统会话及产品绑定 | 6 |

合计 **55项Rust passed / 0 failed / 0 ignored**，含16项新增历史测试；当前库5315项，不是整库全绿。全部以同一冻结输入、固定测试库和独占profile/自有Job执行；private desktop从未切换，runner兜底不等每个子孙自然退出。官方fmt、版本、全仓strict行数、strict all-targets Clippy、lib/普通Release/workspace构建全部通过；仅一次v0.2.33.527，公共版本未变。早期编译/测试检查失败保留。集中构建脚本最终exit=1仅因workspace Cargo同时输出test和产品两个EXE，单候选提取断言失败；按target.name/kind/profile.test唯一固定已经成功编译的workspace目标，未重新构建或更换源码刷绿。

普通前台只使用本轮新profile和四象限合成fixture。Computer Use唯一选择普通主窗并发送Ctrl+H，但list_windows/list_apps没有返回历史owned窗，关联截图无法辨认历史内容；未猜HWND、未改产品样式/捕获保护迎合工具、未继续Tab/Enter/Space/点击或确认删除。因此两列表视觉、键盘导航与复用仍为 **UNVERIFIED_UI_TOOL_NOT_SELECTABLE**，不能用隐藏窗口成绩代替。正常主窗可选也不解除普通Tile的工具边界。未操作用户release97或其他既有应用，不注入全局输入、不改显示设置。

前台fixture结束时原自有Job终止后的5000ms等待断言失败，原exit=1及位置保留；随后只读进程对账确认该固定路径的自有产品树已不在运行，没有再次终止任何进程。未取得原Job最终active计数，不宣称严格清理或正常退出通过；原因仍未知，不归咎于产品/工具/驱动。旧v526样式对照探针也已核其exit=0、activeBeforeClose=0、forced=false：当时工具仅返回Normal，不返回Tool/PopupTool，不修改产品样式绕过。

证据集中在 `main-capture-history-527-1/runtime-audit.json`、visible/ui-result.json、visible/reconcile-terminal.json、build-terminal.json和STATUS/NEXT_SESSION。普通EXE、库与workspace产物各固定SHA；本批不是新完整包或正式发布。v525/v464旧包、全部dirty/untracked和失败保留，Loom未改。历史普通前台/人工IME/旋转EDIT、物理多屏/跨机/DPI热插拔、其余N01–N12、剩余回归/资源长稳、最终完整包及用户全面验收仍开放；Talk构想、旧客户端混用及三冻结副线不重开，总迁移active。

## v526 前台夹具所有权收紧与八项真实断言

本轮只修改 `cfg(test)` 的 `surface_javascript_fixture.rs`：焦点目标必须属于测试进程；不再 `AttachThreadInput`；guard 只保留及恢复仍属本进程的 previous，失焦后不覆盖后继选择。沿用原 `OTTER_NATIVE_FOREGROUND_SPONSOR` / READY→G，保留真实 `GetForegroundWindow()==parent` 断言；不改产品键盘、授权、网络、超时或资源预算。内部版本仅分配一次到 v0.2.33.526，公共版本未变。

新增所有权负向/隐藏自有 HWND 回归 **3 passed**。在用户已恢复的“仅新 profile、自有测试窗口”范围内，Computer Use 明确选择本轮 WinForms 启动器；每项按钮点击后才创建自己的 child/profile，READY 时仅向该 PID 授予一次前台许可并发送 G。测试期间不再激活启动器，不操作用户现有应用、不注入全局输入、不更改显示设置。

| 真实前台限定测试 | passed | 外层秒数 |
| --- | ---: | ---: |
| Enter/default/cancel、TLS grant 与 FIFO/同 child | 1 | 25.336 |
| Focus/blur、真实 EDIT change 与 FIFO/同 child | 1 | 25.551 |
| 在途 focus ABA 不重放 | 1 | 9.363 |
| 在途撤权先于 reply | 1 | 9.287 |
| Hover 祖先 leave | 1 | 24.971 |
| Canvas pointer drag | 1 | 25.125 |
| guard 不覆盖后继前台选择 | 1 | 0.943 |
| unwind 仅恢复自有前台 | 1 | 0.578 |

八项全过，原业务断言和 Rust 期限不变；每项原日志 SHA、receipt 和准确完整名复核通过，外层 Job 在关闭前自然 active=0，无超时或强制清理。加新三项合计 **11 项 Rust passed / 0 failed / 0 ignored**；启动器的三项输入负向检查另计，不混入产品测试。此前两次“child 早于窗口选择启动”尝试均未授予 G，0 passed，原 120 秒安全门回收，未进入业务主体；记录保留。第三次改的是启动时序，不是延长期限或绕过前台断言。

官方 fmt、版本、全仓 strict 行数、strict all-targets Clippy 及 lib --no-run 通过。固定测试库 SHA256：`859814720F183B749AED022DED7AC686B698C0DECE2B7E67BC03E3D688503F15`；4441 双仓运行输入前后未变。没有重编普通产品 EXE、生成新整包或重验全部旧成绩；v525/v464 固定产品及 ZIP 保留。

当前库枚举 5299 项（原 5296 + 新 3）。跨版本历史台账为 **3494 至少一次通过、61 默认 ignored、0 无通过证据的已终态失败、1743 未启动、1 原整体上限中断**；不是 v526 单次全量成功，Art 原 settle 间歇性及 Live 原严格清理未知仍独立开放，不被累计通过消除。

集中证据 `main-foreground-ownership-526-1/runtime-audit.json`、`visible-click-start-3/visible-result.json`、STATUS.md、NEXT_SESSION.md、final-audit.json 和 handoff-state.json。只关闭六项 JS 的前台前提与主体补证、两项 guard 和新所有权回归；新启动器可选不等普通产品旧工具边界已解除，不宣称完整键盘/人工 IME/旋转 EDIT、普通 Source/Loom 桌面壳、物理多屏/跨机/DPI、剩余回归/合计资源长稳或最终整包/用户验收完成。Talk 构想、旧客户端混用、三冻结副线及固定包不重开；总体 active，不提交/推送/安装/部署/发布。

## v525 剩余回归分组续接（914 项终态，非全量通过）

先领取原在途 `models`，随后仅选择上一基准中尚未启动、已审查安全边界的完整测试名；每根独立进程/新 profile/未切换私有 desktop/自有 Job。先枚举核对再 `--exact`，原测试体、Rust 期限和默认 ignored 均不变，不重跑原全库或已通过名称。

| 本批 | passed | failed | 默认 ignored | 秒数 |
| --- | ---: | ---: | ---: | ---: |
| 擦除/翻转/缩放及 Surface 纯模型 | 202 | 0 | 0 | 36.938 |
| DirectWrite/WIC/D2D 离屏、内存 wire 及独占记录文件 | 129 | 0 | 4 | 10.344 |
| Device/Live Relay/Loom/投送协议与自建 loopback | 478 | 0 | 7 | 90.609 |
| 合成长截图、命名、参数及 session 保存文件 | 85 | 0 | 0 | 6.093 |
| 文件替换及真实子进程四阶段中断恢复 | 9 | 0 | 0 | 2.157 |

合计 115 个隔离进程组、**903 passed / 0 failed / 11 默认 ignored**；914 个完整名全部取得终态。子进程探针只用 GUID 专属 linshi 路径，外层清空旧配置/凭据/专属 crash 环境并由自有 Job 覆盖子孙；独立 interruption probe 不是 ignored，无专属路径时正常返回。长截图只输入合成帧，离屏 WIC 不截用户桌面；不能把这些结果当作真实捕获、输入或物理显示验收。

所有 115 份 receipt/原 stdout/stderr SHA、精确清单及 4441 个双仓输入已核，固定库字节未变；没有产品源码修改、版本分配或编译。私有 desktop 全部关闭；runner finally 会兜底请求终止自己的 Job，因此不声称本批每个子孙均自然退出，也不消除上一批 Live 原清理未知。

完整名去重台账更新为 **3483 至少一次通过、61 已观察默认 ignored、6 尚无通过证据、1745 尚未启动、1 项原整体上限中断**，总数仍 5296。它不是单次全量成功，也不是迁移完成比例。六项 JavaScript foreground fixture、Art 原 settle 间歇性和 Live 原严格清理原因仍开放，旧失败不改写。

证据和下一入口：`main-remaining-regression-525-3/runtime-audit.json`、STATUS.md、NEXT_SESSION.md、final-audit.json 和 handoff-state.json。自有前台恢复授权继续有效；本批没有前台输入，未重放普通产品工具不可选的旧启动。完整键盘/人工 IME/旋转 EDIT、普通双产品、多屏/跨机/DPI、剩余全量、合计资源长稳及最终用户验收仍开放；Talk 构想、旧客户端混用、冻结副线和固定包保持不变，总迁移 active。

## v525 分组回归补证与剩余边界

接住并完成原 162 项 SERIAL 连锁失败的逐项独立进程回归：**154 passed / 8 failed / 0 timed out / 0 PoisonError**，586.594 秒。未吞 poisoning、修改锁或测试体；实际至少涉及 `loom_execution_job_tests::SERIAL` 和 `surface_javascript_fixture::SERIAL` 两个 owner，不能再概括成一把锁。所有私有 desktop 关闭，原失败和两次测试启动前的 preflight 拒绝记录保留。

对已审查的自有窗口路径，在当前 default desktop 保持同一库、源码与原 Rust 期限，每项新 profile/进程/Job：先前 43 项非 poisoning 失败中尚未对照的 **32 项呈现测试全部通过**；独立回归里 CPU 编辑/undo/保存/恢复/再呈现失败另作第 33 项，也通过，共 132.922 秒。没有请求焦点、注入全局输入或改变显示设置；不重跑此前已通过的 manual/reporting 两项，不把本机对照推广为物理多屏或所有设备验证。

Live Source 单列，在原安全预检允许的 default/input desktop、新建自有 source HWND 上运行受控 Win32 消息与只读 UIA，**9 项 Rust 行为断言 passed**，11.907 秒。未绕过 SecureDesktop、用户/SID/完整性级别检查，没有 SendInput、操作用户窗口或移动系统光标。严格清理门仅 **8 项通过**：workspace-change interaction 在父进程已结束后立即读到 Job active=1，runner 对自有 Job 兜底回收，不能冒充自然退出；缺少当时 PID 快照，不能断言是残留 child 或 accounting 延迟。

另外精确选择此前未启动的 32 个纯模型根模块，**124 passed / 0 failed / 0 ignored**，1.812 秒；完整名 `--exact` 清单先枚举核实，无 substring 碰撞、无既有终态名称重叠，不强制执行默认 ignored。

剩余的六项 JavaScript 失败都发生于 `focus_foreground` fixture 的 `GetForegroundWindow()==parent` 前提（实际 HWND 为零），尚未执行后续主体；该 fixture 会 SetForegroundWindow/SetFocus，必要时 AttachThreadInput，pointer 还使用 mouse capture，故未混入无焦点呈现批次。Art 离线授权 case-089 的原五秒 settle 失败仍无法定位在线预热或重开后的阶段。仅补一次 `RUST_BACKTRACE=full` 诊断时它通过，未获得失败栈，不作修复证明、不循环重试。一次独立 Job PID 时间线诊断也观察到该 Live 测试自然 active=0/空 PID，但未重现原清理问题；原失败均保留，两个根因仍未关闭。

纠正上一份错误摘要的首 panic 误标：`rejected_or_panicked_draw_cannot_present_and_next_frame_recovers` 的 :419 是预期 catch 的 injected drawing panic；原真正末端失败在 `composition_d3d12_tests.rs:425` 的 `surface.present().unwrap()` 断言。本次当前 desktop 对照通过不改写那次原始错误。

按 5296 个完整名跨运行去重，当前为 **2580 项至少观察过一次通过、50 项已观察为默认 ignored、6 项尚无通过证据、2659 项尚未启动、1 项原整体超时中断**。这不是单次全量通过；Art 间歇性失败和原严格清理失败另列，不能被“至少一次通过”消除。所有运行前后 4441 个双仓输入与固定库 SHA 一致；无产品源码修改、版本分配或编译，随后只追加三份所属 Markdown。

证据：`main-serial-isolation-525-1`、`main-current-desktop-regression-525-1`、`main-live-source-default-desktop-525-1`、`main-pure-models-regression-525-2`、`main-regression-diagnostics-525-1`；集中审计和下一入口为 `main-desktop-regression-closeout-525-2/runtime-audit.json`、STATUS.md、NEXT_SESSION.md。普通产品键盘的工具不可选边界仍未解除；完整 IME/旋转 EDIT、普通 Source/Loom 桌面壳、物理多屏/跨机/DPI、全量/合计资源长稳及最终用户验收仍开放，总迁移 active。Talk 构想、旧客户端混用、冻结副线与 v464 固定包不动。

## v525 默认库审计与呈现环境对照（全量尚未通过）

在途默认原生库回归已按原 1800 秒安全上限结束，未重启、延长期限或竞争 target。
固定同源库枚举 5296 项；终止前按完整测试名解析为 **1891 passed / 205 failed / 48 ignored**，另有 **1 项在途被终止、3151 项尚未启动**，exit=124、timedOut=true。
这些是日志终止点的已观察结果，不是完整 libtest 成功摘要；private desktop 已关闭，自有测试进程均已结束。
205 个失败中 **162 个**原始 panic 明确为共享测试串行 Mutex 的 `PoisonError`，不能解释成 205 个独立产品缺陷；其余 43 项仍须逐项核查，不统称环境问题。

最先出现的两项 checkpoint 恢复回执失败取得 D3D12 `0x087A0001 confirmed=false`，没有新帧确认，随后 receipt 等待失败。
对照保持同一库 SHA、源码、完整测试名及原 Rust 期限，仅改为当前 desktop 上的新建测试窗口/profile/Job；未注入输入、操作用户窗口或更改显示设置。
两项各 **1 passed / 0 failed / 0 ignored**，8.016 / 7.360 秒，均取得 D3D12 `0x00000000 confirmed=true`，Job 关闭前 active=0。
该对照只支持这两项的直接呈现条件阻塞，不覆盖其他失败；`SecureDesktop`、Live 输入、CPU/GPU 恢复及无效句柄等其他输出均保留待核，不降低原安全围栏。

另一个独立进程只运行原全库当时尚未开始的 editor、image_decode、graph_image、session_document、session_vectors 五个根模块，先枚举并精确核 368 名称以排除 substring 同名模块。
原测试体不变，**366 passed / 0 failed / 2 默认 ignored**，139.422 秒；ignored 不计通过，不与全库或对照结果直接相加为“全量通过”。
以上运行的 4441 个双仓输入前后完全一致，固定库 SHA256 为 `EE84F3557BDFFF94A66D8384F6896B1D577F637605A2D790D76DDB6453E403E1`，没有产品源码修改、版本分配或编译。

普通产品键盘尝试已单独收口为 **UNVERIFIED_UI_TOOL_NOT_SELECTABLE**：实际解包 EXE 与普通 daemon、新 profile 及六个夹具包已启动/安装，但 Computer Use 精确窗口/app 筛选均未取得可选对象。
未猜 HWND、绕过工具约束或改变产品样式；尚未配对、打开输出或发送键盘。wall revision=0、endpoints/layouts/sources 为空，临时 TLS 已禁用；自有 Otter 4 进程和 Loom 2 进程由 Job 兜底回收，不冒充 graceful shutdown。
前台验收授权仍有效，仅覆盖自有测试窗口；当前键盘边界是工具不可选，不是缺少用户授权。

证据分别在 `main-full-regression-525-1`、`main-native-pure-regression-525-1`、`main-projection-visible-recheck-525-1` 和 `main-visible-keyboard-525-1`；完整名审计集中于 `main-native-regression-closeout-525-1/runtime-audit.json`。
后续应按真实窗口/输入环境和共享资源分组，保留原失败，再推进未执行项与已定位根因；不原样重跑这条私有 desktop 全量命令刷绿。
两条普通可见 Tile 成功和既有内部 ZIP 不受影响；完整键盘/人工 IME/旋转 EDIT、普通 Source/Loom 桌面壳、物理多屏/跨机/DPI、全量、合计资源长稳及最终用户验收仍开放，总迁移 active。

## v525 解包产品与恢复授权后的普通可见验收

用户已明确“恢复自有测试窗口的前台验收”。该授权覆盖本页历史暂停状态，但只限全新 profile、自有测试窗口；不操作用户现有窗口、不注入全局输入、不更改物理显示设置。
本批没有修改产品源码、重新分配版本、重编产品或竞争 target；所有旧 dirty/untracked、冻结副线、失败回执及旧包保留。

同源普通 Release 已经原候选/归档门打成内部 ZIP，并实际解包逐文件核大小和 SHA：311 文件、165 份依赖许可证包，无 ANGLE DLL；归档合同 **9 passed**。
ZIP：`main-package-525-1/package/Otter-native-v0.2.33.525-windows-x64-internal.zip`，SHA256 `7D3AA821768DC1B441377C4F1EE18FF79482BF1007F733B94C5832EBD6C599B1`。
解包 EXE SHA256 仍为 `E3B3D14CCE5B91B07E8D0F4CEA2BFFC8BEFC35F01B0296398029AB763F60915C`。
该实际解包 EXE 通过保原工作区字节的检查、3 项错误 CLI 拒绝及 **7 passed / 1 foreground ignored** 工作区运行门；不把 ignored 计为通过，也不是完整最终手测包或可发布正式版本。

普通可见门使用这份解包产品、与当前四份 integration/support 叶一致的 ACK 驱动和源码匹配的 Loom 测试库；未用测试库替代 Otter 产品。
Computer Use 仅按返回对象的精确自有 app 路径和唯一启动器激活一次、刷新后确认 nonce ACK，驱动另核真实前台及产品 PID/类再转交。
先前两次编排未完成分别为激活前原期限到达、激活后 `ReferenceError: btoa is not defined` 导致未写 ACK；原尝试保留，不计业务通过、不扩大 30 秒期限。

| 原可见父门 | 实际结果 |
| --- | --- |
| 普通图片 running / rotated / black / resumed / cropped / frozen、正常停止 | 1 passed，28.57 秒 |
| 普通图片 + Live + Art 首帧、三路换帧、冻结、黑场、恢复、来源保留与清理 | 1 passed，20.78 秒 |

两门均在原 30 秒内完成；服务端 applied 与每阶段四个前后均核无遮挡的桌面像素独立匹配，实际 RGB 与 oracle 完全一致。
普通产品日志记录 D3D12 `hresult=0x00000000 confirmed=true`、`Confirmed`、`HeartbeatPreparedWithApplied`，未接受 OCCLUDED、强制 CPU 或用源像素代呈现。
两次 Otter 正常退出、窗口消失、endpoint offline、applied 清空，自有 Job 关闭前 active=0。
混合门还核 Tile 关闭后来源附件、权威值和 Live Source 保留，随后 Source 正常清理、媒体连接归零；Source 是原生测试库夹具，不是第二个普通 Source EXE。
Loom 服务使用真实生产库 HTTPS/签名授权/媒体合同，但仍是测试库宿主，不冒充普通 daemon + Loom 桌面壳双产品联合。

证据：`linshi/otter-native-parallel-20261029/main-visible-525-3` 的 `static.json`、`mixed.json`、两份 `evidence-*/product-*-service-oracle.json`、像素、激活及产品日志；组包证据在 `main-package-525-1`。
本次关闭当前 v525 本机单屏普通 Tile 图片六阶段及 Art/Live 混合可见验收缺口，不改写 v517 历史失败或 v525 未切换 desktop 的 OCCLUDED 诊断。
完整键盘/人工 IME/旋转 EDIT、v511/v518 历史根因、普通 Source 与 Loom 桌面壳联合、物理多屏/跨机/DPI、全量回归、合计资源长稳及最终用户验收仍开放，总迁移 active。
Talk 构想/旧客户端混用仍排除；没有提交、推送、安装、部署或发布。下一项沿自有窗口范围推进普通产品键盘验收，不因历史暂停文字重新停工。

## v525 普通 Release 有界呈现诊断（证据收口，业务仍 PARTIAL）

针对上一批普通 Release 已准备但未 applied 的未知停点，只新增有界、无用户内容的只读诊断。
`desktop/composition_present_status` 每交换链最多记录 16 次原始 HRESULT 变化；`tile_shared::diagnostics` 每 Shared owner 每固定阶段至多一次，清理不重置位图，Shared 锁外写日志。
原可见性条件等价分类，D3D11/D3D12 仍仅 `S_OK` 确认；不改变后端、授权、时钟、媒体期限、ACK 或队列预算，不新增线程、timer、协议、重试或依赖。
`HeartbeatPrepared*` 只表示本地待发送正文，不等于已发送或严格 ACK。

前批 4438 个双仓源码/工具输入开始时全部精确匹配；本批仅分配一次内部版本 `v0.2.33.525`，没有升公共版本或修改 Loom。
input1 strict Clippy 因两处 `nonminimal_bool` 失败（368.953 秒），原回执保留；等其终态后改为等价 `is_none_or`，未添加 allow。
input2 的 4441 个输入冻结，strict all-targets Clippy（273.375 秒）、lib 测试目标编译（551.610 秒）、官方 fmt、版本、全仓 strict 有效行数及 diff 门通过。
六组相关 Rust 回归共 **31 passed / 0 ignored**，含 4 项 Tile trace 与 1 项 Present trace 新回归；另覆盖原 Tile、applied 准入、Surface worker 及窗口识别，不是整库成绩。
新驱动严格期限工具测试另 **3 passed**：HTTP 前后均核原 15 秒，边界时不发请求，拒绝迟到成功；旧驱动与失败回执不修改，工具数不混入产品测试数。

使用原 `build-native-candidate.ps1` 在同缓存串行构建普通 Release，846.328 秒通过，前后 input2 一致。
实际 EXE 为 `main-tile-presentation-evidence-525-1/candidate/payload/otter-native.exe`，SHA256：
`E3B3D14CCE5B91B07E8D0F4CEA2BFFC8BEFC35F01B0296398029AB763F60915C`。
只封存候选及原 provenance，没有新建 ZIP、完整手测包或正式发布；v524 固定 EXE/ZIP 和 Loom daemon 字节不变。

只执行一次最窄普通产品诊断，沿原配对→批准→签名授权→输出注册→真实 4×4 RGBA 图片与 revision=2 布局合同，仍用未切换独立 desktop / 自有 Job。
产品日志实际记录 `selected=d3d12`、`hresult=0x087A0001 confirmed=false`、`Visible` 和 `FramePaintUnconfirmed`，没有 `Confirmed` 或 `HeartbeatPreparedWithApplied`。
本机锁定 `windows-0.60.0` 的 Foundation 常量确认 `0x087A0001` 为 `DXGI_STATUS_OCCLUDED`。
这证明**本次运行已经取得业务帧，但实际 Present 未获确认，直接停在本地呈现确认之前**；不能把它归为 heartbeat ACK 拒绝，也不能仅按常量名称断言某个用户窗口遮挡。
日志按变化/阶段去重，不是每一帧的完整序列；本次 HRESULT 不追溯替代旧 v524 未观测的历史根因。

共 15 个服务端观测，最初 3 点没有 endpoint scene；其后 **12 点**（约 3.110–14.172 秒）为 revision=2、prepared=true、appliedAtMs=null，时钟不确定度 10–11ms，全部 appliedRevision=null。
原 15 秒业务门仍失败，driver exit=1：`AssertionError('observation deadline after HTTP')`。
外层 21.157 秒包含启动与清理，180 秒外层上限未被触发；runner 无内部异常不是业务成功。没有进入完整图片六阶段、Art/Live 混合、像素读回、GPU/资源采样或长稳。

失败后 Otter 正常关闭 exit=0、自有树归零，endpoint offline、applied 清空、临时 TLS 入口关闭；Loom 仅由本次 Job 回收，不证明 daemon graceful shutdown。
独立 desktop 无 SWITCHDESKTOP 权限且已关闭；本次诊断前后 foreground HWND 相同，但不足以单独证明整个区间绝无瞬时变化，不关闭 v518/v517 或恢复前台自动化。
证据集中于 `linshi/otter-native-parallel-20261029/main-tile-presentation-evidence-525-1`：`diagnosis.json`、`final-audit.json`、`runtime-result.json`、`scene-observations.jsonl` 及原 stderr/各门回执。
最终文档合同、输入差异、精确有效行数、UTF-8 无 BOM、旧产物 SHA 与自有进程/target owner 核对另见最终审计，不用文档收口触发重复构建或产品重跑。

本批只关闭“缺少精确呈现状态证据”的缺口，**不是功能修复或迁移完成**。相同非输入 desktop 条件不反复重跑，不以 prepared 替代 applied，不借 cfg(test) CPU 接缝或接受 OCCLUDED 刷绿。
普通可见 Tile 图片全生命周期、Art/Live 混合链、Loom 桌面壳、完整键盘/人工 IME/旋转 EDIT、物理多屏/跨机/DPI、合计资源长稳及最终全面手测仍开放。
Talk 外部会话构想和旧客户端混用排除；Shader/CAPTURE/PNG、v464 固定包、真实 profile/设备不动。无提交、推送、安装、部署或发布，总迁移 active。

## v524 普通 Release Tile 业务接入与呈现未确认（PARTIAL）

不修改产品源码、版本或原包，复用下文两份固定 SHA 的普通 Otter Release 与普通 Loom daemon。
与上一批单纯空闲驻留不同，本批通过普通 `--tile` 管理器的实际命令完成新设备配对、管理员批准和签名会话授权；
再由管理器启动同 EXE 的普通输出进程，沿生产签名续签、HTTPS/Device/nonce 完成唯一 endpoint 注册。
仅使用全新 profile、自有 Job 和未切换独立 desktop，没有恢复前台自动化、操作用户应用或修改物理显示设置。

正式 daemon 的 LAN 校验不接受测试库所允许的 loopback 地址。本批明确选择 OS 已核归属的现有虚拟网卡 RFC1918 地址及动态端口，
本地管理 bearer 仍只发到核明 PID 的 loopback HTTP；TLS CA 来自同一新 daemon，未改防火墙、广播或接入真实设备。
管理员上传手写 4×4 RGBA 图片并提交 revision=2 布局；普通输出枚举尺寸为 3840×2160。
原 15 秒内未收到 `appliedRevision=2`，完整图片旋转/黑场/恢复/裁剪/冻结六阶段没有通过，原失败不改写。

仅追加一次同期限诊断：15 个观测跨 14.110 秒，同一十进程树持续在线，其中 13 点服务端报告
`scene.revision=2`、`prepared=true`，时钟不确定度 9–10ms，但所有点 `appliedRevision=null`、已准备点 `appliedAtMs=null`。
普通产品日志选择 D3D12，只能证明后端创建，不能证明 Present 成功。生产链只将 DXGI `S_OK` 经窗口/Shared/时钟围栏和心跳确认后报告为 applied；
没有外部读到本次 Present HRESULT，也没有交换链或可见桌面像素证据，不能径称 `DXGI_STATUS_OCCLUDED` 或唯一产品根因。
测试专用 `private_surface` 强制 CPU 的接缝不编译进普通 EXE，本批未借用它刷绿或降低呈现断言。

| 等待 applied 的诊断观测（非通过负载） | 实际范围 |
| --- | --- |
| 合计 Private commit | 130.637–165.500 MiB |
| 合计 Working set | 188.230–192.383 MiB |
| 合计单核 CPU 平均 | 7.087% |
| 自有 PID GPU dedicated / shared | 177.031–208.910 / 26.059–26.434 MiB |
| GPU 最大单引擎瞬时利用率 | 0.081–3.037% |
| Handles / GDI / USER | 1783–1804 / 43 / 29–30 |

GPU 三类在 15 点均有自有 PID 实例，首次补得这条普通产品尝试的可用 GPU 观测；这不是 GPU/满载/长稳门通过。
Private 不是独占物理 RAM；Working set 可重复共享页；GPU dedicated/shared 不能和进程 Private 相加当作物理总内存。
树包含管理器、输出、两个 watchdog、daemon 与五个 conhost，和上一批六进程空闲基线口径不同，不能直接比较为资源回归。
采样含等待呈现时的反复绘制及状态查询，不是独占机器 benchmark，也不是已成功显示 4K 内容的性能成绩。

诊断后向本次自有 Tile/管理器正常 WM_CLOSE：Otter exit=0、子树归零，服务端 endpoint offline 且 applied 清空；
临时 TLS 入口已通过原管理员合同关闭。daemon 仍仅用本次 Job 回收，未证明其 graceful shutdown。
外层无 SWITCHDESKTOP 权限，所有独立 desktop 均关闭。诊断前后全局 foreground HWND 不同，来源未归因；
没有调用激活/切 desktop/全局输入 API，不据此宣称产品绝不改变前台，也不关闭 v518 原失败或解除暂停。

证据位于 `linshi/otter-native-parallel-20261029/main-tile-release-business-524-5`：
`diagnostic-summary.json`、`awaiting-applied.jsonl`、`diagnostic-runtime.json`、`runtime-result.json`、`state-stopped.json`。
前四目录完整保留：前三次分别止于新配置默认本地主机、正式 LAN 拒 loopback、默认路由建议地址不适用的驱动前提；
第四次为普通产品 ACK 失败，第五次仅诊断同一失败，不算五个通过场景。所有原业务期限不变，没有重编产品。
汇总/地址围栏 8 项工具测试通过；首次负向测试发现 Python `is_private` 可包含通配地址，
后续选择器已收紧为 RFC1918 并保留原测试/失败，本批实际绑定始终是明确的本机虚拟网卡地址，冻结运行模块及产品未改。

本批关闭普通 Release 配对/签名授权/输出注册/准备与失败后正常回收这一有限证据缺口；
实际呈现确认、完整图片生命周期、Art/Live 混合业务、Loom 桌面壳、GPU/资源长稳和最终用户验收仍未完成。
不重复相同条件重跑此失败；下一项先补精确呈现状态的只读因果证据，或在另行明确恢复前台验证后运行原可见门，不能用 prepared 替代 applied。
Talk 构想和旧客户端混用仍排除，三副线/v464 固定包/真实 profile 与设备不动，无提交、推送、安装、部署或发布，总迁移 active。

## v524 普通 Release 后台合计资源基线

沿用同一解包 Otter Release，不改产品源码或版本；配套 Loom 首次封存当前普通 Release daemon，而非 debug 测试库或旧桌面壳。
4438 个双仓源码/工具输入前后核对。首次冷缓存单线程编译在原 900 秒界限退出，已完成 230 个 artifact、没有 compiler error；原回执保留。
确认无 target owner 后复用缓存，将编译并发由 1 改为 4，保持原 900 秒、locked/offline 与 Release 配置，684.750 秒完成。
daemon EXE 为 40628736 字节，SHA256 `177BEEACB92D2F51BDF78180FC471367D7C770EA9CBB414215FEBC2C227CF08B`；Otter EXE SHA 仍为下节固定值。

普通 Release 双后台在未切换的独立 desktop/自有 Job 上实际驻留 240.079 秒、124 点，分为 idle、autosave、hidden、resumed 各 60 秒。
六进程为 Otter 主进程/watchdog、Loom daemon 及三个 conhost，PID/EXE 全程不变。全新 profile 不导入凭据或能力包；4 个 framework 候选的 installed/enabled 均为 0，注册 tools、Capability Plugins 和 Python Arts 均为 0。
Loom 使用默认 bounded_workers=4、queueCapacity=32；HTTP 与启用的私有桥接均为动态 loopback 端口，先核 OS PID/端口归属再发送本批 bearer，无 LAN 广播或桌面 UI。
桥接服务就绪但空闲客户端为 0：源码只在显式加载目录时建立短握手，不自动维持业务连接。本批没有请求目录或执行 Art/Live/Tile，不将同时驻留说成业务互通通过。

| 模式 | 合计 Private commit MiB | 合计 Working set MiB | 合计单核 CPU 平均 |
| --- | --- | --- | --- |
| idle | 12.277–12.668 | 71.105–71.312 | 0.755% |
| autosave | 12.297–12.691 | 71.113–71.332 | 1.172% |
| hidden | 12.270–12.328 | 71.230–71.254 | 0.911% |
| resumed | 12.270 | 71.230 | 1.042% |

Private commit 不等于独占物理 RAM；Working set 合计可能重复计算共享页。CPU 含自有辅助进程及周期性状态查询，非独占机器 benchmark；采样时其它构建现场另行记录，未结束外部进程。
Handles 在 950–955，GDI=33/USER=18；30 次自有 HWND 移动后逐次核自动保存，最终保留唯一 Unit/extension。
新 profile daemon 就绪 0.235 秒、Otter 工作区就绪 0.625 秒，仅是本次启动观测，没有清 OS 缓存，不能称标准冷启动性能。
GPU Dedicated/Shared/Engine 三计数器均为 124 点无自有实例，明确 unavailable，不填零、不关闭 GPU 资源门。

实际文件系统分配量另通过 FileStandardInfo 读取：Otter 解包载荷 311 文件，logical=19946152、allocated=20648376 字节；daemon 单 EXE logical=40628736、allocated=40632320 字节。
新 profile 合计由 logical=65885/allocated=79000 变为 logical=65886/allocated=74904 字节；不含目录元数据、不去重硬链接，不代表卷空间变化、物理 I/O 或完整 Loom 安装体积。
Otter 沿原 QUERY/END 正常退出，exit=0、自有树归零、工作区保留；daemon 测后只终止本次 Job 并核归零，不冒充 daemon graceful shutdown 验收。

证据在 `linshi/otter-native-parallel-20261029/main-joint-resources-524-2`：`runtime-result.json`、`resource-samples.jsonl`、`measurement-summary.json`、`disk-before.json`/`disk-after.json` 及最终审计。
首轮在 `main-joint-resources-524-1` 因探针误将按需短连接当常驻连接而在原 15 秒界限失败，未开始采样；源码查证后只修探针假设，以新目录重验，不改产品或期限。
原汇总结果与 runner 回执同名导致摘要被覆盖；保留成功原回执/输出，只从同一原始 journal 重算到不同文件名，没有重跑产品。计数器/PID 采集 6+3 项工具测试按相同源码哈希复用，非新增产品测试数。
本批只完成上述普通 daemon 模式的有限 CPU/内存/文件分配基线；Loom 桌面壳、Art/Live/Tile 满载、GPU、物理多屏/跨机/DPI、人工 IME/旋转 EDIT、v518 前台来源/v517 普通可见混合门和完整长稳/最终用户验收仍开放。
Talk 构想和旧客户端混用仍排除；前台自动化暂停、三副线/v464 固定包/真实 profile 与设备不动。无提交、推送、安装、部署、签名或发布，总迁移仍 active。

## v524 Release QuickJS 产品 worker 验收

复用上一批 ZIP 真正解包出的同一 Release EXE，SHA256 仍为 `4F9D9180D273074903FF0FF3A8364FD1E544CBB7829D09A5571BAAC19FED21FB`；4425 个最终源码/工具输入全匹配，不修改产品源码或版本、不重编译、不使用共享 target。
固定 v524 debug lib 驱动只负责发请求与核断言，26 项显式产品门均通过外部路径和预期 SHA 启动 Release 的真实 `--native-javascript-worker`，不是以测试 child 或旧 debug 产品成绩冒充 Release 运行。
覆盖原生 Canvas 像素/rAF/暂停恢复、服务调用前 Canvas 冻结及回收、文档和跨调用状态、事件/表单/文本/Enter/焦点请求、CSS/hover/边框/grid/空间/字体/溢出布局、resize/动画/定时器及原子进程容量/退出断言。这里的输入、焦点和服务均为宿主合同，不是物理输入、Win32 前台焦点或真实 Loom 服务验收。
从固定驱动 ignored-list 枚举并按完整名逐项运行，26 项均取得独立 `1 passed; 0 failed; 0 ignored` 回执；各原始业务/worker 期限、预算和像素断言未改。所有执行限未切换的独立 desktop 与自有 Job，无 SWITCHDESKTOP 权限；没有恢复前台自动化或访问真实 profile/设备。

首项产品实际通过，但独立汇总器把 nocapture 插入的产品诊断误当成同一行缺失 `ok`；原脚本、冻结输入及原始成功回执保留，只重新解析该回执，未重跑首项。修正汇总器另通过 5 项边界测试，覆盖正常/插入诊断成功、非零退出、错误完整名命令及日志篡改拒绝，不与 26 项产品运行数混加。
证据位于 `linshi/otter-native-parallel-20261029/main-release-runtime-524-1`，见 `worker-runtime-audit.json`、`final-audit.json`、逐项 `worker-XX.json` 和原始 stdout/stderr。包内 `runtimeVerified/publishable/signed` 仍为 false，旧整包与固定 v464 不覆盖。
本批仅补齐现有 26 项 QuickJS 产品 worker 门在该 Release 字节上的运行证据，不代表签名 Art 授权/正式结果全链、普通可见 UI、完整 SDK、真实 Loom 联合、GPU/合计资源长稳、物理多屏跨机/DPI/人工 IME 或最终用户验收。Talk 构想和旧客户端混用继续排除，三副线冻结，总迁移仍 active；没有提交、推送、安装、部署或发布。

## v524 内部 Release 整包与解包后隔离运行

在前批最终 4425 个输入文件全部匹配、共享 target 无竞争后，首次为现有 v0.2.33.524 源码构建 Release 候选；不改产品源码、不重新分配版本。
使用原 `build-native-candidate.ps1`、`package-native-candidate.ps1` 和 `verify-native-package.ps1`，保持 locked/offline、LTO、优化配置及原 900 秒构建界。
候选构建 890.610 秒通过，前后源码/工具快照一致。包内 311 文件、165 个 normal/build 依赖许可材料，实际 ZIP 8355484 字节、EXE 18320896 字节；许可材料完整性不等于法律或安全批准。
原归档基线及篡改/缺失/额外/越界/重复/错误布尔类型/两种 ANGLE DLL 拒绝共 9 项通过；真正解压到全新目录后逐文件核大小和 SHA，不以暂存目录代替解包。

EXE SHA256：`4F9D9180D273074903FF0FF3A8364FD1E544CBB7829D09A5571BAAC19FED21FB`。
ZIP SHA256：`75742919B8218108EC7EE55AFAC112E7E884929140823B7495948D89B904F48C`。
证据与候选位于 `linshi/otter-native-parallel-20261029/main-package-524-1`；ZIP 为其 `package/Otter-native-v0.2.33.524-windows-x64-internal.zip`，实际解包程序为 `unpacked/otter-native.exe`。
PE 公共版本仍为 0.2.33，四段内部版本由 provenance 与精确 EXE SHA 绑定；不是签名、正式发布或最终完整功能手测包。

新 Release 解包产物实际运行：隐藏工作区加载/两次绘制及资源重建通过，工作区原字节保持；缺图 `--pin`、未知参数和已退役 worker 三项 CLI 按原合同拒绝。
原固定 SHA 工作区驱动在该 Release 上重新运行，7 passed / 1 真实前台退出 ignored，覆盖保存、强制中断重开、末 Unit 删除、QUERY/END 取消后继续保存及确认结束。
前批 debug 的 106 项不计入本次 Release 重跑数；本批没有全库回归或普通可见前台验收声明。探针汇总器独立 4 项边界测试通过。

解包 Release 在未切换的独立 desktop/自有 Job 驻留 600.922 秒、349 点：idle120 秒与 24 轮 hidden10 秒/resumed10 秒；产品、watchdog 和两个 conhost 始终为同一四进程树。
Private commit 合计 8.270–8.688 MiB，Working set 合计 49.676–50.285 MiB（共享页可能重复计），各模式单核 CPU 约 0.163%–0.241%；Handles652/GDI33/USER16 稳定。
初末 resumed 块中位 Private 增长 140 KiB，句柄/GDI/USER 增长为零，通过事先冻结的有限增长护栏；不能据此宣称没有泄漏或完整长稳通过。
驻留后十次同 profile 的恢复位置→QUERY 取消→移动/检查点→正常结束均通过，保留唯一 Unit、extension 和最终工作区；每轮产品退出后自有进程树归零，外层 desktop 关闭。
进程 I/O 计数在驻留采样期间 read/write 增量均为零，不等于物理磁盘占用验收。原 Ctrl+1 热键占用日志保留，不冒充快捷键测试。

本批只关闭内部 Release 组包及上述限定解包运行门；不修改包内 `runtimeVerified:false`、`publishable:false`、`signed:false`。
v518 全局前台变化来源、v517 普通可见混合门、人工 IME/旋转活动 EDIT、物理多屏/跨机/DPI 热插拔、完整资源长稳及 Otter+Loom+辅助进程的 CPU/内存/GPU/磁盘、最终整包/用户全面手测仍开放。
前台自动化继续暂停，Talk 构想及旧 Web/Tauri 混用仍排除，三副线/v464 固定包/真实 profile 与设备不变；没有提交、推送、安装、部署或正式发布。
独立原始样本与退出审计见本批 `runtime-audit.json`、`soak-result.json`、`resource-samples.jsonl`；总迁移仍 active。

## 原版紧急退出双通道（v524，隔离定向通过）

补齐原版已实现的两条紧急退出：完整 Esc+Delete 三次组合（相邻严格小于400ms，双键重叠且完整释放），以及独立 watchdog 的 Ctrl+Alt+Shift+F12。主消息循环与 child 共用纯状态机；child 在配置/UI 前分流，先见松开才 arm，以原版8ms周期只读键位，不注入输入、不记录文本。
复用原同EXE受限白名单 launcher 和 Hello/Ready；直接父PID、创建时间、同EXE路径及pipe创建者共同认证。普通恢复权限仍只有查询/等待，只有紧急认证额外取得 PROCESS_TERMINATE，并只使用保活原父句柄。正常Drop和握手失败只回收自己新建watchdog，不终止Job或承担Live恢复责任的child。紧急退出绕过保存/析构，不能保证未保存工作保留；原Live父死恢复机制继续负责已登记源。原生未修改全局光标约束，故不无条件重置其他程序的 ClipCursor/ShowCursor/SetSystemCursor。
当前选定Rust按crate/完整名去重 106 passed、6 默认ignored，非整库；11项新增回归覆盖严格400ms边界、完整释放、启动已按住、缺失keyup/迟到消息、参数、认证失败不arm、握手失败、正常Drop及阻塞牺牲父进程的三连/备用终止和父死child退出。另复测原恢复认证/握手/管道/launcher、三进程父死恢复、系统会话消息/Checkpoint和普通固定SHA EXE工作区。全部在未切换的独立desktop/自有Job执行，不申请SWITCHDESKTOP，不向用户应用发送输入。
strict all-targets Clippy、lib/普通product/workspace/恢复认证及session目标构建、官方fmt/版本/全仓strict行数/diff通过。input1因Application达到503有效行而在构建前失败，原回执保留；input2按职责提取application_loop，保持原消息泵/显示恢复/保存/退出顺序，不增行数例外。input2 Clippy通过后，因只读审查发现局部消息可漏收keyup，停止本轮自建在途构建树并保留中止记录；input3主通道按当前键位快照保守累计，不凭历史消息制造重叠，新增负向回归。源码只分配一次v0.2.33.524，Loom未改。
新增常驻进程单独实测180秒（97点）：产品及watchdog全程存活，含idle60秒和三轮hidden20秒/resumed20秒；正常隔离QUERY/END后整棵自有树归零、desktop关闭、工作区保留。该4进程树（产品、watchdog及两个conhost）Private commit为8.691–8.957MiB，Working set合计61.430–61.578MiB（共享页可能重复计），模式内单核CPU约0.156–0.260%，Handles658/GDI33/USER16稳定。Ctrl+1仍记录已有热键占用0x80070581，不冒充快捷键验收。资源明细见main-emergency-exit-1/runtime-audit-3.json；不是Otter+Loom总资源、GPU、物理输入或长稳验收。
本批仅关闭源码与限定隔离门，不恢复前台自动化，不证明物理三连/F12或普通可见混合链通过。v518前台身份变化来源及v517普通可见混合门未关闭，不拿当前隔离通过覆盖历史失败。人工IME/旋转EDIT、物理多屏/跨机/DPI热插拔、资源长稳及最终隔离整包/用户全面手测仍开放。Talk外部会话控制按用户决定排除本轮，旧Web/Tauri混用不重列必做；三冻结副线、v464固定包及真实profile/设备不动，总迁移active，不是正式发布。

## 当前范围澄清：不补做 Talk 构想

2026-11-03 用户明确原版 Otter 未实现 Talk 外部会话控制，本轮不实施或验收该构想。
历史构建记录中的 Talk 待办保留原文，不再作为当前阻塞；以[修订计划](WINDOWS_NATIVE_REVISED_PLAN.md#talk-录音开关构想不属于本轮迁移)为准。
这是范围修正，不改变既有运行成绩或版本，不构成其余功能验收通过。

## 系统结束会话持久化准入（v523，隔离消息链验证）

WM_QUERYENDSESSION只消费原Save/Checkpoint已提交凭证；未保存、在途/排队请求、Live待交付操作及本地Art输入/确认/结果未结算时立即拒绝，设置短ShutdownBlockReason，不在同步回调等待保存或弹框。批准后共享Queried阶段封住新业务和标准子控件编辑；END FALSE恢复，END TRUE不可逆进入Ended并沿原统一shutdown排空。系统结束不等于删除最后Unit，不写空工作区。
选定Rust按crate/完整名去重271 passed、7默认ignored，非整库；17组定向及普通固定SHA EXE工作区回归通过。全部窗口测试在未切换的独立desktop/自有Job运行，未申请SWITCHDESKTOP。普通产品验证启动/移动/检查点→QUERY→取消→继续移动/检查点→QUERY→确认结束，核原字节、唯一Unit/id及extension；不调用系统退出API。
strict Otter all-targets Clippy、lib/普通product/workspace构建、官方格式/版本/全仓strict行数/diff与精确UTF-8无BOM/空白通过；36份Rust最大499有效行。构建秒数{"clippy": 190.593, "lib": 316.375, "product": 106.203, "workspace": 108.953}。input1 lib原900秒时限、input2 Rc路径及行数、input3新夹具PID类型失败均保留，没有放宽时限；行数按责任提取，未加例外。input4五项Application新准入断言和普通EXE初始QUERY原15秒界失败，实证新校验混淆保留输入与已提交候选身份；现分别核原stamp来源与published Proof，新增同字节不同allocation仍拒绝的负向门，受影响目标与运行重验。其余input4回执保留，不拿它们冒充最终输入成绩。复审发现WM_GESTURE/WM_TOUCH携句柄不能被无lp过滤器吞掉，现交默认过程释放并保分类回归。仅给已锁定windows crate增加Win32_System_Shutdown feature，无新crate/线程/hook/timer/队列/权限/预算。Loom未改，不重跑或宣称修复旧Clippy/到期行数失败。证据main-session-end-1/runtime-audit-5.json、review-diff-5.txt及STATUS.md。
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

## v520：本地Art Live自动触发（原生限定联合通过）

原生显式配置→原Live唯一控制worker Configure/Claim→签名child onLiveTrigger→原Application/Results实际事务→ACK及Stop/回收已接通。独立不可逆自动授权绑定完整实例/工作区/包、会话/epoch与条件revision；Preview不等授权，重连不复活旧gate。单槽/16KiB、原期限与共同容量门不变，不新增线程、timer、持久队列、权限或WebView。
用户在既有观看系统菜单配置严格Condition并明确“允许自动回调”；停止先撤本地gate。回调不伪造click/gesture、不请求焦点或恢复暂停timer。Applied只在正式端口原子提交后产生，formalRevision为当前执行实例成功提交序号，不是磁盘版本；NoResult只在实际present后报告。原Results最终predicate再核Runtime/Lease及固定期限，Stop与canonical事务共用无Win32/网络/脚本回调的短锁。

input4相关Rust按crate/完整名去重87 passed、7默认ignored，非整库；新自动回调子门虽默认ignored，已由真实LAN父门显式运行且父子不混加。真实签名child验证暂停回调/缺handler/迟到拒绝；隐藏Application验证提交前无Applied、取消保canonical、正式像素及Save恢复、实际present后NoResult。原本地Art缓存/有效授权离线/结果/宿主退休和Live观看/控制、普通固定SHA EXE工作区5项直接回归通过；前台退出门未运行。
新真实Loom LAN HTTPS/WSS联合6.656秒通过：只改自有离屏EDIT value-seven→value-eight，独立像素[17,83,211,255]、公开SDK同份源码签名执行、data计数1及deliveryId/观察匹配、原Results/Save恢复、Editor不变、独立Device Inspect确认服务端acknowledged/applied(1)，显式Stop及两端owner/媒体permit回收。经产品菜单的同一提交接缝，不宣称真实用户点击了模态菜单；无真实设备或用户程序输入注入。
Otter strict Clippy、lib/普通product/workspace及Loom测试lib构建通过，耗时158.110/408.172/184.781/140.235/50.453秒；官方格式、版本、SDK类型/JS语法、Otter全仓strict行数、两仓diff及精确UTF-8无BOM/空白门通过。44份Rust及5份JS/TS增量均≤500有效行，最大470；不重跑未变SDK等静态面。Loom strict Clippy101条原诊断及旧ArtEditDialog.tsx到期例外仍失败，签名/位置/次数和前态SHA无新增，不称两仓全绿。
input1 large_enum_variant用Box修复；input2行数失败按测试职责提取shape_dialog_tests、原测试体/身份不变；input3旧无条件包装仅剩测试使用而触发dead_code，input4将其限定cfg(test)，生产始终走最终授权predicate。首项Application测试因既有TLS夹具父目录缺失报Os code3；实证缺失后只创建linshi内声明父目录，原失败保留、同输入/同SHA重验通过，不改产品或超时。
证据：main-art-live-trigger-native-1/runtime-audit-4.json、inputs-4.json、joint-4/server-ack.json和最终审计；source-4为运行输入，后续仅所属Markdown变化。
本批仅关闭本地Art自动触发的源码/定向/隐藏Application及真实LAN联合接线，不是普通可见双产品、真实跨机或最终包验收。v518全局前台身份失败、v517可见混合门/v511历史键盘根因仍开放，前台自动化继续暂停；人工IME候选窗/旋转活动EDIT、物理多屏/跨机/DPI热插拔、长稳、其余迁移退出条件、最终隔离整包与用户全面手测仍开放。三旧副线、v464固定手测包与真实profile/设备保持；旧Web/Tauri混用不重列必做，总迁移active。

## v519：本地Art Live自动触发服务（服务端限定闭环通过）

local_art_live_trigger协议及Reply叶独占版本化Device-only自动回调合同；local_art/live_trigger只接原签名宿主与当前审批，runtime/live_session_local_trigger及operations独占登记、单槽、Claim/ACK和退休。Surface与本地目标共用live_session_trigger_evaluation唯一条件状态转移，合计沿原64注册门；不伪造Surface附件、不改grant四权限。
宿主显式automatic_local_callback授权绑定完整Art实例、工作区/代际/包、Live会话/epoch、条件revision及独立authorizationId；Preview不等自动授权。Claim在HTTP响应前消费payload，重复/未知领取不重放；ACK仅报告宿主结果，不证明canonical保存或接收端呈现。本地delivery至多16KiB，超限明确停用；原观察新鲜度30秒、grant期限、墙钟高水位/每事件期限与单调期限共同围栏，查询不续期。停止、最后Viewer断连、审批修订变化、过期和Session close回收或拒绝执行，重连/重批不复活旧授权。没有新增线程、timer、持久队列、依赖或脚本权限。

input2选定相关Rust完整名按crate去重75 passed、1默认ignored，非整库；protocol39/daemon36，其中新增协议3项、服务12项。原Art观看/控制、Surface触发/观察、媒体断连清理与LAN边界回归通过，普通daemon构建及--version/--help通过。HTTP测试用owner直接登记Viewer在线，不是真实WSS或Otter联合。20份Rust最大457有效行；官方格式、版本、Otter全仓strict行数、两仓diff及精确UTF-8无BOM/空白门通过。
input1新服务11项中10过1失败：stale夹具未同步stableSince，原wire先拒400而非freshness409；input2修成一致旧时间，保留409断言和原生产新鲜度校验。审查另补每事件墙钟expiresAt校验，覆盖Ready和Claimed墙钟前跳先于单调期限的过期回归，不改系统时钟或放宽超时。input1失败与全部回执保留，input2只改两份生产叶和一份测试叶，protocol完整输入/固定SHA与成功回执精确复用。审计器初版误将默认ignored当失败，修正后只解析原日志，未重跑测试。
Loom全仓strict行数仍被旧ArtEditDialog.tsx于2026-10-31到期的例外阻塞；daemon/protocol strict Clippy仍各101/10条旧诊断，按完整签名、全部primary位置、次数及前态源码SHA核实无新增。普通daemon有原otter_canvas/document.rs::read未使用警告，其源码未变；不称严格门或两仓全绿。
input1协议/lib/普通daemon编译58.047/178.109/134.953秒；input2协议按SHA复用，受影响lib/普通daemon重建69.968/42.250秒，新12项服务测试16.734秒。逐门回执与冻结输入见main-art-live-trigger-1/runtime-audit-2.json和final-audit.json；Otter生产字节未变，不重复原生构建，不生成或覆盖手测包。
本批只关闭Loom本地Art Live触发服务实现及HTTP定向验证；Otter实际配置菜单、独立自动授权owner、onLiveTrigger私有command/SDK、签名child执行、正式结果/取消接线和真实HTTPS/WSS联合仍由MAIN继续完成，不能以此宣布本地Art触发产品完成。v518全局前台身份失败、v517可见混合门、v511历史键盘根因仍开放，不恢复前台自动化。人工IME候选窗/旋转活动EDIT、物理多屏/跨机/DPI热插拔、长稳、其余迁移退出条件、最终隔离整包与用户全面手测仍开放。三旧副线、v464固定手测包及真实profile/设备保持；旧Web/Tauri混用不重列必做。

## v518：Tile Esc、Tab续接与按钮回执（PARTIAL）

Tile Popup明确区分Select，真实打开的下拉列表先消费Esc并抑制同次长按；keyboard_window以Rc身份围住取消/同步销毁，只在同一已paint快照保留Tab位置，迟到epoch不得取消后继owner；外部失焦仍完整退休，父Tile拒同次Esc长按连带关闭，新鲜Esc仍退出。取消退休未发Edit后缀，不重放未知写或丢已接受本地值。
keyboard_joint_access及keyboard_fixture_tests仅cfg(test)，Tab后才计按钮受理，避免字段change冒充Enter成功；测试线程键表完整RAII恢复，生产Ctrl+Enter拒绝不放宽，并记录实际焦点/paint/修饰键。专属Combo实验只在未切换desktop及新测试线程运行，不申请SWITCHDESKTOP；只禁用该线程真实IME，人工组合消息不等人工候选窗验收。

input4当前264项相关Rust完整名去重passed、1 failed、5 ignored，非整库；Tile窗口93项实跑92 passed/1 failed，8项取消及3项夹具新增回归全过。8条真实Loom LAN父门通过，父子不混加：两种文本链在input4重跑62.406/69.218秒，六个非文本门按精确三份cfg(test)差异/SHA复用。普通固定SHA产品工作区5项通过证据复用，前台退出门未运行；product/workspace未因夹具变化重编。Otter strict Clippy/lib、官方格式/版本/全仓strict行数、diff及精确UTF-8无BOM/空白通过，9份Rust最大461有效行。
input1类型编译失败、Win32探针销毁失败、Slider环境变量遗漏及文本聚合外层误截断均保留；原文本四会话各30秒，外层恢复v515既有180秒，不延长业务/网络或单会话期限。input3键盘number仅input/change正式成功而没有click的30秒截止保留；helper误报已由确定性测试证实并修复，不能据此认定历史丢Enter唯一根因。input4 scoped行数初次缺少新增owner的前态定位，改为使用已保存且SHA匹配原manifest的fixture-fix-before，不减弱行数门；修正独立回执通过。Loom全源码未变，旧strict Clippy101条及过期行数例外仍失败，不称两仓全绿。
当前唯一终态失败为tab_edit_to_button_commits_local_draft_and_enter_repeat_does_not_duplicate的全局GetForegroundWindow前后身份断言，尚未确认是用户切窗、产品行为还是其他来源，不用历史通过覆盖、不反复重跑、不恢复前台自动化。故本批仅PARTIAL，不发布INTEGRATION_VALIDATED或完整键盘/整项迁移完成。v517混合可见门、v511历史根因、本地Art Live触发独立授权/执行/回执合同、人工IME/旋转活动EDIT、物理多屏/跨机/DPI热插拔、长稳、最终完整隔离包和用户全面手测仍开放；旧Web/Tauri混用不重列必做，三旧副线/v464固定包及真实profile设备保持。
逐门耗时及SHA见main-tile-keyboard-cancel-1/runtime-partial-audit-4.json；开发EXE SHA256 51FB903657629362A80225DEB6378C8FF7D2DEB56F80AFE372BA45D73346AB16，不是新的完整手测包。

## v516：Tile普通产品可见链与已提交像素（限定闭环通过）

presentation_readback_test独占cfg(test)有界staging/Map/预算与读回像素；composition_d3d12_readback_test读成功Present后的上一物理buffer，composition_readback_test读同一D3D11双缓冲flip-sequential链的已提交逻辑buffer1。software_surface/render_software/render_surface仅有cfg(test)委托。tile_presented_pixel_tests复用产品Renderer，独立矩阵核图片/Art/旋转/crop/透明叠层/换帧/黑场恢复。唯一生产结构变化为Renderer::new复用私有from_surface初始化，测试建链前选定后端，不在同HWND叠建两条GPU链。
native/tests/tile_product_runtime与support/tile_product_window独占普通固定SHA EXE、真实CLI/物理输出、隔离profile及自启PID/精确类名的窗口验证；Loom tile_product_native_joint只负责真实生产库LAN HTTPS/签名配对与续签、layout/presentation及权威ACK。四象限独立RGBA常量对应六阶段running/旋转/黑场/恢复/crop/冻结，外部仅采四个前后均确认无遮挡的桌面点，随后正常关闭和endpoint offline。桌面合成像素不冒充产品交换链直接读回，库内各后端读回也不冒充普通产品可见运行；父子不混加。子产品由Kill-on-close Job持有，失败/超时只清理自己的进程。初始可见门因遮挡明确失败后，按用户授权由Computer Use仅激活驱动的普通自有启动器一次，驱动核实际前台后向已核PID/类的同测试子Tile转交一次。不以该显式激活声称产品自动置顶或全程无激活；不移动鼠标、不注入用户应用、不改物理显示或真实数据。没有新产品诊断开关、依赖、路由、权限、线程或资源预算，旧三副线及v464固定包冻结。

本批最终选定回执136项相关Rust完整名去重passed、2默认ignored，非整库；早期失败另行保留。三后端实际提交目标和自有窗口负向门四项新回归通过。4条真实Loom LAN父调用通过：新普通EXE单屏可见链，以及原图片/Live/Art三链回归；普通固定SHA EXE工作区5项通过，前台退出门未运行。17份Rust最大489有效行；Otter strict Clippy/格式/版本/全仓strict行数、两仓diff和精确UTF-8无BOM/空白门通过。
首轮D3D11读回全零，原失败保留。flip_probe在隔离2×2窗口连续红/绿/蓝实测：Present后原backing/逻辑buffer0已轮转到下一帧，逻辑buffer1才是本次提交，D3D11 GetCurrentBackBufferIndex始终0，不能套用D3D12物理索引算法。修正仅cfg(test)读回接缝，不改产品绘制、不放宽oracle或超时。同时将新Windows专属Loom父测试以cfg(windows)登记，避免向Linux测试引入Windows文件锁导入；未声称实际跑过Linux。input2普通可见门running超时；input3仅加有界单槽诊断确认Err(Occluded)，没有读取遮挡窗口像素。input4及input5初次尝试仅按可选title筛选而未匹配，未执行激活且超时，不能由此断言工具窗口不被枚举。在源码/产物/配置未变的input5上，独立attempt按返回app绝对路径唯一选到普通自有启动器，Computer Use只激活一次；驱动核真实前台并转交自启Tile，六阶段及关闭通过，耗时25.157秒。激活后首次状态刷新因同时禁用text/screenshot被API拒绝，未重复输入；后续list仍见该启动器，最终再获取时因测试正常结束而窗口已消失。转交/像素/清理由独立产品和服务回执证明，不把失败刷新写成成功。原30秒、独立oracle/每通道≤1及全部遮挡检查保持，原失败证据保留，不改foreground-lock或附加用户线程。
input1严格Clippy/lib/产品/workspace/新integration/Loom库构建分别耗时184.532/282.093/81.547/93.438/3.437/59.843秒；input2 strict Clippy/lib/Loom库重验140.188/296.360/74.203秒；input3只重编诊断驱动，input4重编前台握手驱动/Loom父测试，input5 strict Clippy/驱动/Loom库耗时4.406/5.406/83.422秒。每轮完整manifest仅精确test-only差异，未变产品、库、工作区及对应测试按SHA复用，0秒是复用记录，不冒称重编。
Loom strict Clippy仍101条同签名/位置/次数旧诊断；strict行数仍被2026-10-31过期ArtEditDialog例外阻塞，不延期/绕过，不称两仓全绿。
证据main-tile-presented-output-1含全部冻结输入、首轮失败、最小实验、逐门回执、像素/service-oracle及runtime/final审计。固定开发EXE SHA256 8E2DD7C0C0673F059883A73F91E5E711F2870BFDAD82D8F60D9ADD07D66CD253，不是新完整手测包，不覆盖v464。
本批只关闭上述本机单屏普通EXE静态图片可见链和产品Renderer各后端提交目标读回，不关闭普通EXE可见Art/Live、完整Tile键盘、人工IME候选窗/旋转活动EDIT视觉、物理多屏/跨机/DPI热插拔、长稳、本地Art Live触发、其余迁移退出条件、最终完整隔离包或用户全面手测。v511历史超时根因仍开放；旧Web/Tauri混用按用户确认排除。 总迁移active；无commit/push/发布/部署/安装或删除用户文件。

## v515：Tile滚动条拖拽与轨道重复翻页（限定闭环通过）

tile_surface_scrollbar独占已paint滚动条身份和捕获坐标；desktop/tile_surface_scrollbar_window独占单槽Thumb/Page手势、Rc token、捕获及timer回收。复用surface_scroll的Bar/Drag、唯一surface_scroll_repeat allocator/Repeat、原Presentation COW计划与tile_surface_scroll_window安装/实际paint，不复制Surface业务。Repeat登记提升至desktop，普通Surface保同名alias；tile_window_test_access只承接cfg(test)窗口入口。
滑块可拖出原track/窗口，轨道立即翻页并在400ms后每50ms至多一步，不追赶积压；thumb到指针、移出track或反向时暂停，回到原方向再继续。活动EDIT由滚动条提交一次，保中文emoji草稿及无handler表单；IME组合拒绝接管。prepare空隙只被动保留且不滚动，空隙抬起释放、不采用迟到位置。同步paint前移出owner，取消轮换token，旧回调不得恢复旧手势或覆盖successor；隐藏/冻结/黑场/过期/撤权/DPI/关闭/丢capture及wheel/Tab接管都清捕获和timer。初始资格仍核topmost/crop/旋转/clip/布局/view/代际，捕获后屏外坐标不授新命中资格。不改scene/canonical、网络动作、权限、原队列/像素/TTL预算；无新hook、线程、依赖、WebView或用户应用输入注入。

当前245项相关Rust完整名去重passed、3 ignored、0当前运行终态failed，非整库；12项新回归覆盖四向旋转/crop/非等比映射、COW计划、真实EDIT提交、timer逻辑时钟、prepare/生命周期与同步paint取消/接管/销毁。普通Surface原轨道重复回归同批通过。11条真实Loom LAN父调用通过，父子不混加；新drag/page各有handler与无handler四独立会话，独立常量核唯一input/change/click、实际framework/Art正式结果、已paint及view/线程清理；原十条输入联合重验。固定SHA普通EXE工作区5项通过，前台退出门未运行。
input2严格all-target Clippy/lib/普通产品/workspace/Loom库实际构建分别耗时163.469/385.297/117.453/145.922/61.469秒。仅framework及四handler按完整Loom delta与固定SHA复用，不冒称重编。18份Rust最大487有效行；官方格式、版本、Otter strict行数、两仓diff和精确变更UTF-8无BOM/空白通过。
Loom strict Clippy仍101条同签名/位置/次数旧诊断，全仓行数仍被2026-10-31过期ArtEditDialog例外阻塞，不延期/绕过，不称两仓全绿。

首轮编译失败为根测试误引用desktop私有Apartment；改为测试自身COM RAII，不扩大生产可见性。原失败JSON/stdout/stderr及各冻结输入保留；后续修复和重验情况见本批STATUS，不用放宽超时或取消断言换绿。
证据main-tile-scrollbar-1含输入/源码delta、原始定向/LAN回执、四独立service-oracle和runtime/final审计。固定开发EXE SHA256 48400DBA1695B0B3DCEA76984C6A66C1D8F86E2F0CAC56E22752E2D409FA4D5D，不是新完整手测包，不覆盖v464。本批仅关闭Tile滚动条实现、定向与隔离LAN集成；inputCapabilities仍pointer。完整键盘、人工IME候选窗/旋转活动EDIT视觉、普通可见Tile与最终交换链读回、物理多屏跨机/DPI热插拔、长稳、本地Art Live触发合同、其余迁移退出条件、最终完整隔离包及用户全面手测仍开放。v511历史超时根因仍未确立，后续成功不反证旧故障已修复；旧Web/Tauri混用按用户确认排除。 总迁移active；三旧副线、真实profile/设备不动，无commit/push/发布/部署/安装或删除用户文件。

## v514：Tile本地滚轮与Tab屏外自动显露（限定闭环通过）

surface_scroll及surface_presentation_scroll_state独占COW滚动快照、未滚动布局SHA与受crop约束的显露；desktop/surface_scroll_input只共享原系统滚轮解码。tile_surface_scroll_render/window复用原Presentation与唯一HWND，先准备、提交/隐藏活动EDIT并退休旧手势，再安装和实际paint；同步Win32调用前释放借用。tile_surface_focus/text/local按未滚动布局保草稿，不借屏外保留身份激活输入。tile_surface_actions在原预算中携带实际激活时的scroll，原唯一worker新鲜布局恢复该几何后再校验。tile_surface_scroll_memory只保留至多四个同key/view/代际/尺寸的COW偏移跨越prepare空隙，不保留旧Art/scene/像素或呈现资格；减权/过期/身份变化清理。
滚轮复用系统行/字符/整页偏好、Shift水平与Ctrl让出，嵌套祖先只消费一次；Tab/反向Tab能显露屏外字段，仍受crop、四向旋转、最上层命中和有界搜索约束。重投影重装嵌入文本/Select/Slider绘制缓存，保中文emoji草稿和无handler本地值；同view/代际/尺寸新snapshot继承并钳位scroll，换代重置。IME组合不提交或换焦；隐藏/冻结/过期/准备空隙拒新滚动，旧按压不重新命中新位置。不改权威scene/canonical、表单字段范围、协议、权限、队列/像素预算、原1500ms/120秒门或未知写不重放规则；无WebView、新依赖、hook、线程或输入注入。

当前232项相关Rust完整名去重passed、3 ignored、0当前运行终态failed，非整库。15项新回归覆盖COW、crop/hidden轴、滚动后嵌入缓存、新snapshot钳位、实际EDIT复用/表单、IME、旧press、无handler值、减权、四向旋转与隔离TLS恰一次POST，以及真实prepare空隙后的显露焦点/草稿/待发表单、纯偏移清理与身份隔离。10条真实Loom LAN父调用通过，父/子不混加；新滚动鼠标/键盘各覆盖有handler和无handler两独立会话，独立常量核中文emoji输入记录、唯一change/表单、实际framework/Art正式结果、已paint和view/线程清理。旧文本/IME、离散值、Slider、本地值两种入口重验；普通固定SHA EXE工作区5项通过，前台退出门未运行。
当前input6严格all-target Clippy/lib/普通产品/workspace耗时分别133.328/454.687/133.485/109.781秒。Loom仅专属cfg(test)叶变化；其库首次按新输入构建，后续若该仓输入未变则按完整manifest/SHA精确复用。原framework和四handler源码未变，固定SHA复用，不伪称重编。
40份Rust最大493有效行；官方格式、版本、Otter全仓strict行数、两仓diff及变更文件UTF-8无BOM/空白门通过。Loom strict Clippy本次仍101旧诊断，逐签名/位置/次数无新增；全仓strict行数仍被2026-10-31到期的ArtEditDialog.tsx例外阻塞，不延期或绕过，不称两仓全绿。

原失败保留：input1测试多余String转换触发strict Clippy，仅移除.into()；input2窗口入口达到502有效行触发行数门，将原鼠标按下准备/提交/捕获接线归回既有actions owner后重验，不新增例外或压缩代码。input2旧本地值几何回归暴露same_local忽略了非滚动rect变化，现仅在scroll真实变化时允许平移，并在被动保留中刷新实际几何；旧负向断言和部分显露后再点击保值均通过。input2/3新Switch夹具在初始点击前已零高度；两次复用同源Presentation的最小几何实验确认其既有合同忽略自身layout，改用固定行容器并核独立19/9像素可见高度，不改产品布局或放宽断言。scoped行数首次缺两份原始备份，仅按初始manifest旧SHA从已有归档恢复。全部原日志保留，不把更早输入通过当当前输入证明。

input4键盘滚动LAN超时；input5只加cfg(test)Tab目标断言与变化时队列诊断，单次复现：真实连续input已ACK，但prepare空隙清掉Presentation后丢失scroll，新快照重绘使屏外按钮焦点失效并退休change/click后缀。input6增加纯COW偏移Memory跨同会话空隙，并以确定性真实HWND换snapshot回归保护原三意图及新鲜派发；另核空cache、九类生命周期与五种身份变化。不扩大30秒会话、1500ms/120秒门或资源预算，不重放未知写；v511历史根因仍未确立。

证据main-tile-scroll-1保存冻结输入、定向/联合原始日志、service-oracle、逐门耗时、runtime/final审计。固定开发EXE SHA256 2BC9BC5C21EAFCB57EB77CC7048251819657DF394CDDC799C80972DE067DB501，不是新完整手测包，不覆盖v464。本批关闭Tile本地wheel与Tab屏外reveal的实现、定向和隔离LAN链；不等完整键盘、滚动条拖拽/重复翻页、人工IME候选窗/旋转活动EDIT视觉、普通Tile输出EXE可见链、最终交换链读回、物理多屏跨机/DPI热插拔、长稳、本地Art Live触发合同、其余迁移退出条件或最终完整隔离包/用户全面手测。inputCapabilities仍为pointer；旧Web/Tauri混用互通按用户确认排除，v511旧超时根因仍未确立。 总迁移仍active，三旧副线/真实profile设备不动，无commit/push/发布/部署/安装或删除用户文件。

## v513：Tile重绘待派发意图与写前生命周期围栏（限定闭环通过）

确定性红回归证明：新Art候选已publish却尚未paint时，旧逻辑会先消费意图再因无presented而拒绝。现在只有对应Art实际已paint才取出普通意图；短暂prepare空隙只恢复未发原Command，保事件ID、顺序及原期限。原唯一worker栈上命令以弱Ticket计入原32槽/8MiB，撤销不提前归还实际owner预算；clear/suspend/discard、新failure、代际和Edit取消永久退休旧身份。昂贵准备后至HTTP提交前再次核生命周期；提交后拒迟到结果，不回队、不重放未知写。保1500ms意图、120秒前缀、原HTTP期限与冻结Cancel语义，无新线程、hook、依赖、权限或另一队列。

当前144项相关Rust完整名去重passed、1 ignored、0当前终态failed，非整库。11新回归含prepare后stop/clear-resume/suspend-resume/freeze/black/generation/expire/deadline/lease九种同步屏障、真实TLS恰一次POST、R2 baseRevision/原eventId、冻结取消、HTTP后断连不重放，以及预留预算/顺序/前缀等待。9条真实Loom LAN父调用全部通过，覆盖鼠标/键盘文本、Switch/Select、Slider、本地值及确认/拒绝/取消，核实际handler、权威/正式结果、已paint与清理；父/子不混加。普通固定SHA EXE工作区5项通过，前台退出门ignored；--tile-outputs只验枚举，不等普通输出窗口启动。
input2 strict Clippy/lib/普通产品/workspace实际构建127.921/245.438/82.485/89.750秒。input3仅两cfg(test)叶变化，Clippy/lib重新构建115.469/202.641秒；普通产品/workspace按完整manifest差异和固定SHA精确复用，不伪称重编。Loom全输入未变，库/framework/handler精确复用v512，不重复其历史检查；原101条Clippy诊断及过期ArtEditDialog行数例外未清除，不称两仓全绿。8份Rust最大434有效行，Otter官方格式、版本、全仓strict行数、Git/scoped空白和UTF-8无BOM通过；运行后仅同步五份owning Markdown并执行组织/命名/Web合同与本地链接门，不为文档重编。

失败原证据保留：proof红回归、input1 E0624跨模块可见性编译失败；input2两项新测试oracle误判（后入队意图原期限稍晚，Cancelled ACK按既有规则归拒绝终态）。input3仅修两份测试，统一逻辑原期限并核恰一次http_ack/原分类，不改生产、超时、预算或验收条件。v511 input6历史超时缺少当时队列/派发证据，根因仍未确立，不能将本次确定性缺陷等同旧根因，也不以成功重跑改写旧失败。

证据main-tile-dispatch-fence-1保存inputs-3（4347项）、runtime-audit-3、native-reuse-3、loom-reuse-2、原始失败和逐门耗时及最终审计。固定开发EXE SHA256 782F2FC4138A3F15574C232A2EE52316FFC69DB53B180ABE7349F89EE57EE7A4，不是完整手测包，不覆盖v464。完整键盘/屏外scroll与reveal、人工IME候选窗/旋转活动编辑视觉、普通Tile输出EXE可见链/最终交换链读回、物理多屏跨机/DPI热插拔、长稳、本地Art Live触发合同、其余迁移退出条件及最终隔离包/用户全面手测仍开放；inputCapabilities仍为pointer。旧Web/Tauri混用互通已按用户确认排除。总迁移active，旧三副线/真实profile设备不动，无commit/push/发布/部署/安装或删除用户文件。

## v512：Tile无handler本地值与权威同步（限定闭环通过）

tile_surface_local独占无handler的Switch/Select/Slider本地值与源值SHA stamp，desktop/tile_surface_local_window独占UI线程cache、paint身份校验和生命周期，tile_surface_local_render只投影到原Presentation绘制缓存；不改权威scene或持有Art像素。复用tile_surface_focus、既有鼠标/键盘控件及原唯一Art串行worker，不新增输入框架。
无handler值可鼠标/键盘编辑；同合同同源换快照保本地值，新权威源值替换，view/代际/合同/布局/几何变化退休。准备空隙仅被动保留且拒新输入，隐藏/冻结/黑场/停止/撤权/过期/DPI或窗口换代清理；Select取消不变值，Slider取消恢复此前接受值。与文本共享每view64/总256字段，不加倍预算；本地操作不offer空动作。surfaceValues仍只含input/textarea/number字符串，不加入Switch/Select/Slider，保原16字段/48KiB表单门、祖先click、队列/权限/资源预算。

当前274项相关原生/普通工作区Rust完整名去重passed、6 ignored、0当前终态failed，非整库。9条真实Loom LAN父调用通过，共15个独立会话，不把父/子重复累计；新本地值鼠标/键盘联合分别13.875/13.079秒，核零网络动作、同源保值/新权威采用、恰好一次仅含当前文本的表单、实际framework/Art正式结果、已paint以及保Source/回收view和线程。旧文本/离散值/Slider的键盘与鼠标、点击/确认/拒绝/取消联合均重验。真实WIC/D2D像素使用独立常量及既有权威Select基准；自有离屏HWND不冒充普通可见产品或最终交换链。
input2严格Otter all-target Clippy/lib/普通产品/workspace实际构建113.625/189.657/84.266/74.656秒；普通固定SHA EXE工作区5项通过、前台退出门ignored，--tile-outputs只验枚举。input3完整manifest相对input2只有Loom新cfg(test)叶变化，Otter全部源码/配置/版本与固定产物SHA未变，精确复用input2回执而非伪称重跑；Loom库在input3重建38.890秒。25份Rust最大499有效行，官方格式/版本/Otter全仓strict行数/两仓diff及变更文件UTF-8无BOM/空白门通过。
Loom strict Clippy本次36.437秒仍101旧诊断，逐签名/主位置/次数无新增；全仓strict行数本次仍被ArtEditDialog.tsx于2026-10-31到期例外阻塞，不延期/绕过，不称两仓全绿。

失败保留：开发期15条测试私有访问错误；input1 Switch像素误采y32而非既有19DIP轨道中心y9，仅修测试坐标，颜色oracle不变。input1 Loom脚本遗漏cwd未实际编译，改fixedcwd新回执。input2新LAN只更新Source附件导致Tile同源快照等待超时；实际put_snapshot只更新指定附件，input3仅修测试向两个隔离宿主分别PUT，不改生产或原30秒期限。另补无change handler拒绝伪造离散commit的生产门和负向测试后，已重新验证input2全部受影响目标；不复用input1成绩。有效行数证据文件与命令回执同名覆盖的工具错误已独立修正并保留，不重跑产品。

v511的键盘/Tab接线及当前回归虽已通过，input6的values LAN派发超时仍缺根因证据；后续成功不等生产修复，不发布该批INTEGRATION_VALIDATED，也不把旧失败改写为通过。
证据main-tile-local-values-1包含inputs-2/3、native-reuse-3、runtime-audit-3、独立service-oracle、完整日志/失败/耗时及最终审计。运行后仅同步owning Markdown，不重编未变源码。固定开发EXE SHA256 197BC8013A8C7B7BBDFD03FC4729484D39CA127B407F1E8427C98DA3B4CDC071，不是新完整手测包，不覆盖v464 ZIP。本批关闭无handler值控件的实现、定向及隔离LAN集成；不关闭完整键盘/屏外reveal或scroll、人工IME候选窗/旋转活动编辑视觉、普通Tile输出EXE可见链、最终交换链读回、物理多屏/跨机/DPI热插拔、长稳、本地Art Live触发合同、其余迁移退出条件和最终完整隔离包/用户全面手测。inputCapabilities仍为pointer；旧Web/Tauri混用互通已按用户确认排除。 总迁移active，三旧副线/真实profile设备不动，无commit/push/发布/部署/安装或用户文件删除。

## v510：Tile文本、IME边沿与草稿表单（隔离闭环通过）

tile_surface_text独占真实已paint字段身份/合同和源→crop→Wall/输出旋转几何；desktop/tile_surface_text_owner/window/popup独占有界EDIT会话、IME、草稿/ticket、表单装饰与HWND回收。surface_edit_session只复用unbound本地编辑/编码入口，真实Surface仍要求原Binding，不能互相绕过权限。tile_surface_edit独占跨字段Group与原worker栈上Dispatch清理，tile_surface_edits沿原32槽动作队列串行化前缀，不新增动作worker。
文本、textarea、number以真实EDIT通知生成input/change；IME组合抑制中间态，textarea归一CRLF，number拒无效编辑并支持step。无handler字段只保本地draft，不伪造change；实际按钮沿原16字段/48KiB表单门读取当前草稿，已移除或合同不匹配字段不复活。连续input只合并同Edit尾项，跨字段及后续按钮保顺序；在途/ACK等待保固定期限，终态只授一次原1500ms就绪派发窗口。原64个120秒跟踪/事件64KiB/32槽8MiB不扩大，重复GET不续期，未知写不重放。
新权威未paint时不按旧scene结算ticket；准备空隙只被动保留同租约/view/代际草稿。新paint后完整复核合同/几何，失败、取消、隐藏、冻结、停止、撤权、DPI/窗口换代清理编辑组和后缀。每view最多64/总256字段；owned popup marker/单调epoch拒旧HWND消息，Win32同步调用前移出owner避免RefCell重入，活动EDIT直立显示，非活动草稿复用原唯一D2D/crop/旋转/clip绘制。没有WebView、新线程、hook、依赖或权限。

相关Rust父测试完整名去重188 passed/0终态failed/2 ignored，不是整库。新真实Loom LAN联合62.797秒，分别在四个独立原30秒会话验证input中文emoji/IME边沿、textarea CRLF、number4及无handler草稿表单；独立实际handler记录核input/唯一change/按钮当前surfaceValues、权威/正式结果/已paint及保源/线程/view回收。旧Slider、Switch/Select、点击/确认/取消联合分别16.703/13.938/21.329秒通过。自有离屏HWND/EDIT消息不注入用户应用输入，不等普通可见产品或最终交换链像素读回。普通固定SHA EXE工作区5项通过，前台退出门ignored；--tile-outputs只验枚举。
input4只修专属cfg(test)表单夹具，严格Otter all-target Clippy与lib实际重建122.718/243.313秒。普通产品为input3生产源码，input3→4仅测试叶变化；workspace独立驱动/Loom库来自input2，外置text-handler在input3重编，均按完整delta/配置和固定SHA复用并在当前input4实际运行联合/工作区，不伪称重建。33份Rust最大471有效行，格式/版本/Otter strict行数/两仓diff通过。Loom strict Clippy仍101旧诊断逐签名/位置/次数无新增；旧ArtEditDialog.tsx于2026-10-31到期的例外仍阻塞全仓strict行数，不延期/绕过，不称两仓全绿。

失败原证据保留：开发期Windows API参数/类型、input1可见性和Clippy；input2十项EDIT入口失败因测试把2×2输出改64×64却沿同key内容去重未换实际帧，只修初始夹具；input2真实联合已过input/textarea而number字符串4.0与独立oracle4不符，只改外置handler数值规范化。input3定向7过3败；独立复用已编译Presentation的几何实验确认textarea/button默认30px最小高度挤压首字段到4px，两个编辑坐标命中同一字段。input4只明确测试最小高度并核节点矩形/实际命中，保所有跨字段/表单/取消断言，不改生产布局或扩大超时。原workervalidate拒绝/早退的Dispatch guard和跨Group在途依赖另有确定性回归。

main-tile-text-input-1保存冻结输入/补丁、逐门日志与耗时、独立oracle、失败及runtime/final审计。运行后只同步owning Markdown；固定开发EXE SHA256 34B364183598F84FD094E465F6236FEA2ADEB7619D2F2ACBC4E8B55984C72635，不是新完整手测包、不覆盖v464 ZIP。本批关闭Tile文本/textarea/number、IME合成边沿及文本草稿表单按钮的隔离闭环，不等完整键盘/Tab导航、已安装输入法候选窗人工验收、旋转活动EDIT视觉、全部值控件本地draft/完整表单。inputCapabilities仍为pointer。普通Tile输出EXE可见链、最终交换链读回、物理多屏/跨机/热插拔/真实DPI、本地Art Live触发合同、长稳、其余迁移退出条件、最终完整隔离包和用户全面手测仍开放；旧Web/Tauri混用互通已按用户确认排除。 总目标active，三旧副线、真实profile/设备不动，无commit/push/发布/部署/安装或用户文件删除。

## v509：Tile Slider 拖动、串行提交与异步预览（隔离闭环通过）

tile_surface_slider 独占 canonical Range/编辑合同及单手势身份；tile_surface_edits 独占原队列尾 input 合并、同 Edit 后缀退休及前缀就绪接缝。desktop/tile_surface_slider_window 独占原 HWND 的鼠标捕获与释放、抬起提交、单槽异步预览和取消；surface_paint_controls 只准备嵌入控件绘制缓存/paint-only 预览，不改 scene 或普通 Surface 子 HWND 权限。
原 Art 唯一串行 worker 先 GET 新鲜 state 再至多一次 POST；32槽/8MiB、64个120秒跟踪、事件64KiB不变。Slider 同手势依赖仅在固定在途/ACK跟踪期限内等待，前缀终态 GET 只授一次原1500ms派发准入，重复GET不续期；失败、拒绝、取消、合同/窗口换代及等待到期都退休后缀，不重放未知写。新快照撤旧呈现、Art准备完成及候选发布但未paint的过渡只被动保留同租约/同view/代际手势，不发送空白期输入；实际paint后完整复核合同/几何，准备期抬起不提交。没有扩大网络/业务超时、鉴权或资源预算。
轨道在canonical源坐标内按5px thumb/step归一，保非等比输出、原crop/四向旋转/clip/最上层命中。连续input只合并同Edit未发送尾项，不跨commit/click；抬起change先于祖先click，控件外抬起仅clamp值、不合成外部click。预览跨抬起保留到有界请求和新鲜画面收敛；input-only控件没有change时同样保留待结果预览，不伪造change。权威场景与正式结果仍归Loom。

相关 Rust 父测试完整名去重 237 passed / 0终态failed / 6 ignored，不是整库。新 Slider 真实 LAN 联合 14.875秒通过：50 continuous→144次鼠标移动有界合并→唯一100 commit，核真实handler记录、权威/正式结果、已paint场景及停止后view/线程清理；实际自有离屏HWND消息，不注入用户应用输入，不冒充最终交换链像素读回。旧离散值和点击/确认/取消联合分别13.703/20.359秒通过。普通固定SHA EXE工作区5项通过，前台退出门ignored；--tile-outputs枚举不等普通Tile输出启动。
最终input5严格Otter all-target Clippy/lib/产品/workspace实际重建耗时127.531/198.656/85.750/108.860秒。27份改动Rust最大472有效行；官方格式、版本、Otter全仓strict行数和两仓diff通过。Loom所有源码/配置在input2→5逐SHA未变，库、专属handler、定向及静态回执精确复用；三条LAN联合用新Otter child在input5实跑。Loom strict Clippy仍101旧诊断逐签名/位置/次数无新增，旧ArtEditDialog.tsx于2026-10-31到期的行数例外仍阻塞全仓门，未延期/绕过，不称两仓全绿。

保留失败：input1模块可见性编译失败；input2 ACK测试夹具误用不存在的rejected状态（两重叠pattern均记录）；新真实联合捕获连续input100成功但抬起commit在等前缀时被入队TTL错误淘汰。input3补前缀等待/一次就绪准入状态与独立逻辑时钟回归；其联合又在权威换值后原手势持续断言失败。源码核实Art新快照会清空呈现槽；input4只保留Art=None的第一段，定向通过但真实联合仍失败。input5按真实Art worker/Assets/paint顺序补齐Art已准备和候选未paint两阶段；确定性真实HWND逐阶段核保捕获、不采样、重新paint后完整复核及提交，并补input-only抬起预览。保原1500ms/120秒门和所有最终断言，不用扩大期限/反复重跑刷绿。行数预检发现surface_paint_frame增长到505后按嵌入控件缓存职责拆分，未新增例外。

main-tile-slider-1保存冻结输入、逐门回执/复用依据、失败、scoped源码/补丁及runtime/final审计。运行后只同步owning Markdown，不重编未变源码。开发EXE SHA256 889CDC0893427BB8CB5F06E5A8B7DC1E34445CB5396B999A2DEACE818721D09D，不是新完整手测包、不覆盖v464 ZIP。只覆盖声明了 input/change 的 Slider 鼠标交互；无 handler 仍只读，文本/IME/完整键盘与完整表单仍开放，inputCapabilities 保持 pointer。普通 Tile 输出 EXE 可见链、最终交换链读回、物理多屏/跨机/热插拔/真实 DPI、本地 Art Live 触发合同、长稳和最终整包/用户全面手测仍开放；旧 Web/Tauri 混用互通已按用户确认排除，不再计为本轮待办。 总目标仍active，三旧副线与真实profile/设备不动，无commit/push/发布/部署/安装或用户文件删除。

## 当前验收范围澄清（2026-11-02 用户确认）

以[修订计划的旧客户端互通排除规则](WINDOWS_NATIVE_REVISED_PLAN.md#旧客户端互通不属于本轮验收)为准：
新原生 Otter 不要求与旧 Web/Tauri 客户端混用互通。下方历史“旧 Web 实际互通”等未验声明保留为当时覆盖边界，
不得再计入本轮必做剩余任务或作为完成、手测包及最终验收的阻塞项；历史测试结果不改写。
新原生端与配套 Loom（含远程 Loom）、新原生端之间的功能与真实跨端、稳定性和最终整包验收仍须完成。

## v508：Tile Art Switch/Select 离散值（隔离闭环通过）

tile_surface_value 独占已呈现字段命中、Switch 布尔翻转、Select 有序索引和值合同及新鲜派发校验；surface_input_event 只共享 wire 编码，不伪造普通 Surface 挂载资格。desktop/tile_surface_value_window 独占原生菜单 RAII、取消和模态重入复核，原 Tile pressed 生命周期负责退休手势。沿实际 paint 的 view/generation/snapshot/placement/node 身份、crop/旋转/clip/最上层命中和原 Art 唯一串行 worker，不新增线程、hook、依赖、权限、像素面或预算。
Switch 与普通 Surface 一致读取 props.value，祖先 click 在 change 前；Select 菜单保声明顺序/重复值，转义远端菜单控制文本，取消或选择当前索引不发动作。模态返回后重新核停止、撤权、黑场、冻结、DPI/窗口销毁及当前来源；worker GET 新鲜 state 后核同节点声明、合法 payload/action/class 和命中，再至多一次发送。原 32槽/8MiB 队列、64KiB 事件、1500ms 意图期限及未知写不重放保持。

相关 Rust 父套件完整名去重 160 passed / 0终态failed / 1 ignored（Otter 135、Loom 20、普通固定 SHA EXE 工作区 5），不是整库。新真实 LAN 联合16.500秒通过：Select cancel/current零执行；真实 Switch down/up 和 Select 新项各执行一次原 executor/framework/Art，独立 payload oracle 恰为 true 和 b，权威/正式结果与实际已 paint 场景依次101/102；停止回收 view/线程且保源附件。Select 选择由测试回调注入，真实 Win32 菜单项/勾选/转义/销毁另有定向门，不冒称鼠标实际操作了 popup。旧点击/确认/取消、三类动态停止/撤权、Art view/撤权五联合分别28.453/11.219/7.359/18.141/8.172秒通过，保持原30秒父门。

Otter strict all-target Clippy157.312秒、lib307.375秒、普通产品127.844秒、workspace目标174.453秒通过；后续仅 Loom 专属测试叶修正，Otter全部输入及固定产物SHA逐项未变，精确复用构建与实际 input3 定向/工作区回执，不称 input4 重跑。Loom lib input4重建77.157秒；workspace实际4.594秒核普通启动、Checkpoint、中断重开、末Unit删除不复活及SHA准入，另有1项前台退出 ignored；普通产品 --tile-outputs 枚举通过。17份改动Rust最大432有效行。格式/版本/Otter strict行数/两仓diff通过；Loom strict Clippy实际57.828秒仍101旧诊断，签名/主位置/次数无新增，旧 ArtEditDialog.tsx 行数例外2026-10-31到期仍阻塞全仓strict门，不延期/绕过，不称两仓全绿。

真实失败保留：input1菜单测试 GetMenuItemCount 需 Option<HMENU>；input2新Loom测试误用不存在的 stop 而非 finish；input3新联合把 patch 权威值先到误当正式提交，已亲核生产分别加锁，改为原期限内等权威/正式/成功ACK共同终态。后两次只改Loom专属测试叶，不改变生产、期限或验收断言。main-tile-discrete-values-1 保冻结输入、scoped差异、旧失败、复用依据、独立oracle、逐门耗时及最终审计；运行后集中同步owning文档，不为纯文档重编。

开发 EXE SHA256 5D83D32E7D2334B07EFB8914BF8725BE653DFBA650512135AC70F06CDBD7D8FC，按固定 artifacts 保存，不是新完整手测包，不覆盖 v464 ZIP。只覆盖声明了 change 动作的离散值；无 handler 仍只读，没有新增本地 draft，不广告文本、IME、连续 Slider 或完整键盘导航，inputCapabilities 仍为 pointer。普通 Tile 输出 EXE 可见链、最终交换链像素读回、物理多屏/跨机/热插拔/真实 DPI、本地 Art 触发、长稳及最终整包/用户全面手测仍开放。 总目标active，三旧副线、真实profile/设备保持不动，无提交/推送/发布/部署/安装或删除用户文件。

## v507：图片/Live/Art三类动态共同联合通过

tile_mixed_joint_tests/tile_mixed_native_joint专属cfg(test)叶复用原真实Source系统菜单、签名配对/续签、Loom LAN HTTPS/WSS、Tile Runtime和唯一原生呈现。静态图片通过新资源/layout、Live通过原Source受预算纹理、Art通过原snapshot/资源分别换像素；独立手写PBGRA核三路首帧和变化后同一已paint帧，不以帧号增长冒充内容变化。冻结保实际Frame Arc并停Tile媒体但保Source，黑场/恢复沿原链；停止/撤权核view/像素/Shared/线程/媒体permit清理和源附件/权威值7保留。

本批相关Rust完整名去重70 passed/0 ignored，无终态失败，不是整库；26份input1实际定向回执经input2仅两专属新叶delta/测试体与生产未变/SHA精确复用。八条真实LAN联合在input2实跑：三类动态共同更新后停止8.688秒、变化后撤权9.032秒、三类冻结/黑场/恢复/停止16.563秒，分别为三个独立完整Source会话。另两旧Live、两旧Art及原点击/确认/取消联合重验通过。
strict Otter all-target Clippy128.422秒、Otter lib294.047秒、Loom lib58.875秒通过；格式/版本/Otter strict行数/两仓diff通过。Loom strict Clippy仍101旧签名/主位置/次数无新增，全仓strict行数仍被旧ArtEditDialog.tsx于2026-10-31到期例外阻塞，不延期/绕过，不称两仓全绿。7份cfg(test) Rust最大421有效行，生产协议/权限/预算/依赖和后端选择不变。

生产和普通工作区EXE沿v506最终输入/原SHA精确复用，不重新构建或重跑未变普通工作区历史门，不把旧成绩计入本批统计；版本507为本批内部身份，不冒称已生成507普通产品或新完整包。测试编排最初误用不存在tile_shared前缀，失败说明保留，仅改工具后核输入/命令/日志SHA复用已过三组。input1新联合已核三路变化及冻结/黑场，但合成Source原15秒测试寿命在恢复前触发、父30秒失败保留；input2仅两新专属叶改为上述三个独立完整会话，仍保原15/30秒及所有验收检查，并补单槽语义状态回执，不抬门限或称原连续用例通过。

main-tile-mixed-joint-1保存冻结input1/2、完整delta/产物SHA、独立oracle、逐门耗时、编排失败、STATUS及最终审计；运行后只同步两仓五owning Markdown，并跑组织/Web/本地文档门，不重编未变源码。普通Tile输出EXE可见链/最终交换链读回、物理多屏/跨机/热插拔/真实DPI、完整Art值控件/IME/键盘、本地Art触发、长稳、其余迁移退出条件、最终完整包和用户全面手测仍开放。自有离屏NOACTIVATE HWND actual present不是上述验收。总目标active，三旧副线/v464包/真实profile与设备不动，不提交/推送/发布/部署/安装或删除用户文件。

## v506：Tile Art 点击、显式批准/拒绝/取消（隔离闭环通过）

tile_surface_hit/click只将实际已paint帧的最上层interactive placement映射到canonical Art，保crop/四向旋转/clip与按钮down/up身份；共用原纯wire编码，不伪造普通Surface挂载资格。tile_surface_actions独占32槽/8MiB动作和64个120秒确认锚点，UI只排意图，原Art唯一串行worker先GET新鲜state再核view/generation/节点/命中/期限，至多一次Device/nonce POST，不重放未知写。
原生左键点击声明按钮/点击动作；右键菜单显式审阅批准/拒绝或取消已运行可取消请求，不自动弹窗或自动批准。模态返回复核完整提示、来源、已呈现布局及当前授权；短暂识别/冻结/黑场只减权，未发Event/Decision丢弃、有界确认锚点保留，真实布局/view/generation换代/停止/撤权仍清。当前只广告pointer，不广告值输入/键盘。

新真实Loom LAN→原生管理器配对/独立自有离屏NOACTIVATE HWND真实WM_LBUTTONDOWN/UP→原executor/真实framework与Art进程联合23.390秒通过，沿原30秒父门：None不拒绝、待确认经真实识别/同view恢复后显式Reject不执行，Approve后权威/正式结果及实际paint文本101，慢执行后显式Cancel且晚结果不改变101，执行记录恰fs，view/控制/线程回收且保源附件。选择回调复用生产choose，不冒称实际MessageBox点击；不是普通可见双产品、最终交换链读回或物理扫描。
相关Rust完整名去重175 passed / 0终态failed / 1 ignored，不是整库；25组定向在input9实际通过后，input10仅两份Loom专属联合夹具/外置handler变化，按完整delta、测试体/生产未变及产物SHA复用原回执，不伪称新库整套重跑。11条Tile真实LAN联合和普通固定SHA EXE工作区5项均在input10实际运行。旧十条联合耗时依次16.031/4.375/17.469/7.766/14.484/4.954/6.234/9.203/16.844/7.438秒；普通EXE工作区2.891秒，覆盖启动/Checkpoint/中断重开/末Unit删除不复活及SHA准入，--tile-outputs枚举通过，不等普通Tile输出启动。

真实失败保留：generation=0为Loom合法初态，移除本批误加正数限制并补0→1旧动作退休测试，没有伪造初始1。input6/7/8慢动作失败先用test-only受理/派发证据排除未命中，input9服务原错明确首轮handler在安装包cwd写executions.txt导致锁定包完整性校验拒绝。input10仅把夹具TEMP/TMP以ENV_LOCK下RAII设到独占root、handler写该root，沿原受管环境白名单，不放宽包完整性/鉴权；独立实际handler101/102、fs和包cwd未变化oracle通过，原30秒门不延长。
最终Otter strict all-target Clippy133.640秒/lib269.672秒实际通过；普通product82.875秒/workspace76.969秒为input6生产字节，后续cfg(test)差异经剥离/完整delta/SHA证明后精确复用；Loom lib input10 40.860秒、strict独立handler0.656秒重建。格式/版本/Otter strict行数/两仓Git whitespace通过，43份Rust最大458有效行。Loom strict daemon Clippy实际35.422秒仍101旧诊断，按签名/主位置/次数无新增；全仓strict行数旧ArtEditDialog例外2026-10-31到期仍exit2，不延期/绕过，不称两仓全绿。

main-tile-art-actions-1保完整冻结输入/各历史失败/精确复用/独立oracle、耗时和最终STATUS/final-audit；运行后仅同步两仓五owning Markdown，组织/Web合同和本地链接由最终回执登记，不重编未变源码。开发EXE SHA256 96D79962AA485FFE1FD553E49ACE2ED9E91C2FECA7549D8053474D8CE1B23170，按artifacts-10保input6字节，不是新完整手测包、不覆盖v464 ZIP。完整Art值控件/IME/键盘、本地Art触发、真实Loom图片/Live/Art三类动态共同联合、普通Tile输出EXE可见链/最终交换链读回、物理多屏/跨机/热插拔、资源长稳、其余迁移退出条件、最终整包和用户全面手测仍开放。总目标active，三旧副线冻结，不提交/推送/发布/部署/安装或删除用户文件；以下旧条目保留当时范围。

## v505：Tile 物理输出识别（实现、定向与隔离真实联合通过）

从实际输出枚举广告display name/canIdentify；未分配layout也能识别，不等待图片/Live/Art资源。tile_identification拥有精确endpoint/lease/request、稳定Marker Arc、单槽报告和从状态GET开始保守计的单调期限，重复观察只缩短、不延期，过期/关闭/clear保最后身份防迟到重开。原Presenter唯一控制线程串行消费一次applied/dismissed，沿固定Device/nonce HTTPS POST /v1/walls/endpoints/identify/report；未知结果不重放，原错误链清屏/退休。识别期间layout application为null、无scene应用回执，恢复须新鲜普通控制/心跳周期；已接受Art工作不因识别被取消。
原Tile HWND与RenderSurface内DirectWrite绘名称、设备/输出身份、像素尺寸、向上取整倒计时及关闭按钮；不加线程、timer、像素面、依赖、权限或预算。隐藏HWND即使收到真实WM_PAINT也不得回报，只有可见且实际提交才确认；关闭/Escape/Enter/Space绑定实际显示Arc，旧画面输入不能取消未绘制successor，恢复围栏中的repeat不顺便退出输出。普通无marker的Escape仍退出，DPI/display重入、停止/撤权清旧marker/renderer。Loom仅将既有report加入精确LAN Device白名单，管理员identify仍不开放，业务/TTL/鉴权沿原Wall store；输入能力仍空。

最终运行input3共4275项，相关Rust完整名去重166 passed / 0终态failed / 1 ignored（Otter库143、Loom库18、普通固定SHA EXE工作区5/1前台退出ignored），不是整库。20组Otter及3组Loom定向通过；新模型5项、TLS1项、自有离屏HWND4项和实际D2D/WIC独立像素1项覆盖期限/身份/单次报告/释放、hidden paint、旧画面successor、按钮半开边界及同步DPI重入。像素expected为独立PBGRA常量，不调用新renderer计算预期，不截图桌面。
两新真实Loom生产库LAN HTTPS→原生管理器配对/独立测试输出联合通过：识别/关闭/自然10秒过期/恢复15.687秒，撤权/清理4.203秒。先核未分配layout的applied=false，再核真实可见paint后service applied=true且layout application=null；Escape清识别后2×2红绿蓝白图实际提交，第二次自然过期后恢复，最终marker/线程清理。原八Tile联合重验：Art16.860/撤权7.750秒、Live14.046/撤权4.625秒、恢复5.766/恢复后撤权8.609秒、图片完整链17.016/撤权7.156秒。普通EXE工作区5项2.375秒核启动/Checkpoint/中断重开/末Unit删除不复活及SHA准入，实际--tile-outputs枚举通过。

严格Otter all-target Clippy152.250秒、lib283.579秒、普通产品112.781秒、workspace109.594秒、Loom lib50.031秒全部构建通过；23份Rust最大451有效行。格式/版本/Otter strict行数/两仓Git whitespace通过。首次input1 Clippy127.015秒因close_rect跨子模块可见性E0603失败；input2仅修为实际祖先tile_window可见后，strict Clippy168.219秒发现新测试field_reassign_with_default。input3仅两cfg(test)叶改结构初始化及补hidden HWND真实WM_PAINT负向断言；失败/输入均保留，同一505不重复分配版本，全部所需目标集中重建，没有沿用失败成绩。未放宽期限、预算、鉴权或检查。
Loom strict daemon Clippy实际重跑31.375秒仍101条旧诊断，逐签名/位置/次数与v503 input6一致，无新增；strict全仓行数仍因旧ArtEditDialog.tsx例外2026-10-31到期exit2，未延期/调时/绕过，不称两仓全绿。集中证据main-tile-identification-1包含完整输入、scoped源码/补丁、逐门日志/耗时、独立oracle与final-audit/STATUS；运行后只追加两仓五owning Markdown，组织/Web合同及本地链接由最终回执登记，不重编未变源码。
开发EXE SHA256 5238985F00F322C964F0FF2CE2A8CBEEE3721A31124C37D19A8085E547A74D79，按artifacts-3固定，不是新完整手测包，不覆盖v464固定ZIP。此批关闭识别实现/定向/隔离LAN联合，不等普通Tile输出EXE可见链、最终交换链像素读回、物理屏幕扫描/多屏同步/跨机/热插拔或长稳。Art输入/确认/取消、真实Loom图片/Live/Art三类动态共同联合、本地Art触发、其余迁移退出条件、最终整包及用户全面手测继续开放。总目标active，三旧副线冻结，不提交/推送/发布/部署/安装或删除用户文件；以下旧范围保留历史事实。

## v504：Tile 慢 Art HTTP 与 Live 隔离（实现、定向及相关联合通过）

每个在途 Tile job 有 Art 才创建一个有界串行 Art worker，独占 COM/Views 与同 Grant 第三 Lease；没有 Art 不建新线程，仍受原16租约总门，不复制 token 或增加像素预算。原 Assets 持续轮询 Live，Art 只有一个稳定 Prepared Arc 槽，无待办资源/采样队列。Shared 同时核 generation/key 与准备身份，Art 换版先清旧候选，迟到 paint 不得确认另一 Art；退出先撤发布资格再关闭可信 view，冻结等待不持共享锁，旧 job 实际退出才归还容量。保原 HTTP8秒、Live5秒、旧帧1秒期限，不增加依赖、权限、WebView 或第二网络框架。
完整场景仍需所有来源齐备：新 Art 图片待备时清旧完整画布，Live继续收包但不单独发布不完整场景；不把网络活性说成此时仍连续显示。真实 TLS/WSS 可控 peer 挂住 Art 图片超过5.2秒，期间原 Live 连接至少两次真实 Ping，资源到齐后图片/Live/Art 原生实际提交；独立常量分别核红色静态图、[17,83,211,255] Live 与绿色 Art。没有重连掩盖，最后不同 Live storage 证明新解码帧；同组另测在途停止/撤权、取消唤醒及迟到不发布/像素回收，共7.641秒。六项专属模型覆盖准备换代、冻结/期限、无Art无线程、异Grant拒绝、Held排空、退休不碰新代际与迟到paint围栏。

最终运行 input2 共4267项，SHA256 D58FD20870749F8F39EB7A5F430BD8B4990C9D007FD8406CBBFAED82245D4083；147项相关Rust完整名去重通过/0终态failed/1前台退出ignored（Otter库132、Loom父联合10、普通固定SHA EXE工作区5），不是整库，也不把v503的696项旧成绩重新计入本批。18组定向、原八条 Tile LAN 全过：Art17.297秒/撤权7.719秒、Live14.500秒/撤权4.859秒、输出恢复6.000秒/恢复后撤权8.562秒、图片完整链16.829秒/撤权7.593秒。Source/Viewer输入释放4.156秒、H2644.250秒直接回归通过；普通EXE工作区5项3.047秒，核启动、Checkpoint、中断重开、末Unit删除不复活及SHA准入，实际 --tile-outputs 枚举通过。
最终严格 Otter all-target Clippy121.781秒、lib260.687秒、普通产品116.985秒、workspace120.328秒通过；格式/版本/Otter strict行数/两仓Git whitespace通过，17份Rust最大449有效行。初轮input1 Clippy130.922秒通过后审查发现Held分支临时锁跨wait及旧准备退休缺口，登记后仅停止自己尚未完成的过时lib构建；没有运行已知缺陷输入或用其成绩覆盖修复。集中补围栏及测试后input2全重建重验，未调整阈值。Loom生产/测试库源码字节与v503最终完全未变，按完整delta/SHA复用其固定库，不重编或重跑无关旧定向；旧strict daemon Clippy101原诊断/过期ArtEditDialog行数例外仍作为既有阻塞保留，不称本批重新执行或全仓通过。

main-tile-art-live-isolation-1 保存完整输入/补丁、逐门回执/耗时、初轮中止现场及最终审计；运行后仅修订两仓五owning Markdown，组织/Web与本地文档门由最终回执登记，不重编未变源码。不制作新整包或覆盖v464固定ZIP，开发EXE按artifacts-2.json的SHA固定。慢Art不拖停Live的可控网络隔离闭环关闭，真实Loom图片/Live/Art三类动态共同联合、普通Tile输出EXE、最终交换链读回、物理多屏/跨机/热插拔、Art输入/确认/取消与识别、本地Art触发、其余迁移退出条件、长稳、最终包及用户全面手测继续开放。旧三副线冻结，总目标active，无提交/推送/发布/部署/安装或用户文件清理。

## v503：Tile 只读 Art 显示与 view 生命周期（实现、定向及隔离真实联合通过）

当前广告 image/raw_bgra/surface_v1/scheduledPresentation，inputCapabilities 仍空。Art 沿精确 Wall view 路由、原 Device/nonce/Lease 与 binding/instance/attachment/generation/sequence；snapshot/preview/formal revision 单调且同版内容不可变，snapshot:null 保稳定 Arc。最多4个 Art 来源、16 placements、512累计节点、16唯一图片，混合16M像素；图片仅取 snapshot 声明的哈希资源，不接受 URL/inline 绕权。后台纯数据 Arc、UI 独占 Presentation/COM，稳定身份缓存布局；原 Tile 唯一帧混绘图片/Live/Art，复用 crop/四向旋转/clip/z 序，不执行终端 JS、不增加 WebView/子 HWND。Input/Textarea/Number/Select/Slider 只读绘声明值，沿原字体/归一/canonical Insets/圆角/opacity，不开放 draft 或输入。
未知 open 停止且不盲重发；冻结保实际 paint，迟到 state 不替换，迟到新 open 只关闭可信 view；新 snapshot 先清旧帧，每步最多下载一张资源且分别核 width/height。停止/撤权排空 view、资源与原线程，保源实例。单张 Art 同步 HTTP 仍最多8秒，可能使同线程 Live 超过原5秒期限；没有放宽期限、预算或宣称慢网混合平滑已验。

最终 input6 共4263项，manifest SHA256 C1B916C82BAAEDC1CDDD29B33BEA5D083B11FD2C4417FB02514CAF3553E43E44。相关 Rust 完整名去重696 passed / 0终态failed / 7 ignored（Otter库657/6 ignored、Loom库34、普通固定SHA EXE工作区5/1 ignored），不是全库；库列举5065项，不用定向结果代替全库。34组 Otter 定向、3组 Loom 相关组、扩大的公共 Client 组77.875秒、公共 Surface 测量/绘制3.688秒、Art 暂态分类及实际 Application“失败保实例/正式保存/显式重试”1.281秒通过。17项 Source Application 逐例独立进程通过，Source/Viewer 输入释放6.390秒及H264策略/关键帧/回退8.031秒回归通过。
八条真实 Loom 生产库 LAN→原生 Tile 联合全部通过：Art更新/冻结/黑场/恢复/停止17.203秒、Art撤权8.484秒、Live14.391秒/撤权4.859秒、输出恢复5.625秒/恢复后撤权8.766秒、图片完整链16.797秒/撤权7.672秒。独立构造像素 oracle、实际原生窗口提交与服务 scene 回执分别核验，唯一 view/资源更新及停止后 view/像素/线程/permit 回收且源实例保留；Art与图片联合不是三类来源动态共同联合。自有离屏 NOACTIVATE 窗口不是最终交换链像素读回、普通 Tile EXE 可见输出、物理跨机/多屏扫描。普通固定SHA EXE工作区5项4.813秒，核启动、Checkpoint、中断重开、末Unit删除不复活及SHA准入；实际 --tile-outputs 枚举通过。

失败均保留：input2 strict Clippy 两项191.656秒失败；input3发现 Client 新 Surface 请求仍落普通8MiB ceiling，TCP前 Error::Request，input4仅精确 Device POST 接回既有10/24MiB专属上限并补方法/身份/别名/超限负向门。两个空 View 夹具按既有 auto-height=0 合同明确尺寸，保独立像素 oracle 并补 geometry 断言，未改生产布局。input4 Art已actual present后联合误读 #[serde(skip)] ephemeral 字段失败；input5仅改 Loom 专属联合叶，改核公开descriptor精确device/wall hook/attachment并保回收/源保留断言。
input5扩大公共Client回归256 passed/26 failed/5 ignored；首个Art暂态断流错误在fresh process同SHA仍复现，其余25项为SERIAL中毒。根因是旧Client新增 Error::Transport 后 art_service_failure 未接原 service_transport Promise 语义；input6唯一源码变化补该映射/分类测试，不重放submit、不改授权/原call一次cancel或切离线。Otter全部所需目标重建后重验；Loom输入与固定库按input5完整delta/SHA复用，没有把旧生产成绩套给新源码。

最终严格 Otter all-target Clippy129.703秒、lib238.641秒、普通产品50.468秒、workspace51.891秒通过；Loom库input5重编19.125秒并精确复用。格式/版本/Otter strict行数及两仓Git whitespace通过，36份Rust最大495有效行；组织/Web合同和Otter三文档本地链接由最终回执登记。Loom strict daemon Clippy仍101条原诊断且逐签名/次数无新增；strict全仓行数仍因旧 ArtEditDialog.tsx 例外2026-10-31到期exit2，未延期、调时或绕过，不称两仓全绿。
集中证据 main-tile-art-1 保存输入、scoped补丁、逐门回执/耗时、独立oracle、全部历史失败与最终审计；运行后仅更新两仓五Markdown，源码/产物不变。固定开发产物 artifacts-6/product.exe SHA256 259C7BBFC4BE7BD5D40DB84E3291EF7CD322B80008DCA59F57C6A40A829ADD08，不是新完整手测包，不覆盖v464固定ZIP。慢Art HTTP与Live隔离、三类动态共同联合、Art输入/确认/取消、识别、普通Tile输出EXE、最终交换链读回、物理多屏/跨机/热插拔、长稳、本地Art触发、其余迁移退出条件、最终整包及用户全面手测仍开放。总目标active，三旧副线冻结，不提交/推送/发布/部署/安装或删除用户文件。

## v502：Tile Live 混合内容（实现、定向与隔离真实联合通过）

原生Tile当前广告image/raw_bgra/scheduledPresentation，在原图片Assets单线程串行接最多四个Wall媒体。精确GET /v1/walls/live/media、Device认证/新nonce、原endpoint/lease/revision/session绑定和loom.wall.media.v1必须全部成立；直接复用旧生产NLWM80校验器，不伪造NLLV64或Surface Viewer资格。raw与PNG分别走唯一受预算BGRA→PBGRA和原Tile WIC，PNG≤1280×720/4MiB，编码接收缓冲与payload并存纳入原512MiB像素池。产品默认raw，本批PNG通过独立Receiver核验，没有新增PNG模式选择UI。
队列最多三pending加一current，按daemon receive时间及含RTT/漂移/舍入的公共时钟，80ms软件延迟、250ms迟到丢弃/1000ms旧帧失效；不声称硬件同步。Frame独占不可延长的单调期限，窗口独立watchdog清旧Live，新候选或重复发布不能延长已paint旧帧/场景回执。全部可见来源就绪才发布完整画布；冻结只保实际paint快照并停旧媒体，黑场、恢复、布局/时钟换代、停止和撤权按原代际退休。原Assets线程/网络/像素/呈现与取消设施保持，无新线程、hook、依赖、权限或第二截图/输入框架；可见Art仍明确拒绝。

真实Source系统菜单/Application→Loom生产库LAN HTTPS/WSS→原生4×2图片/Live混合输出联合14.453秒通过：终端自行注册/租约/心跳，左2×2固定图片、右2×2 Live，独立[17,83,211,255]×4核raw与PNG解码像素，实际窗口paint确认raw新Pixels storage。冻结时Tile媒体全部退出且Source继续发布；黑场、running重新接媒体、停止后连接/像素Weak/线程回收通过。另一真实撤权联合5.094秒核旧帧不发布、相同清理归零。原输出恢复/恢复后撤权6.157/10.109秒、图片冻结/黑场/恢复/停止17.390秒和图片撤权7.703秒四联合全部重验；普通Source/Viewer输入释放7.922秒与H264策略/关键帧/回退8.547秒共同接缝回归通过。自有离屏NOACTIVATE窗口/原生实际提交不是最终交换链像素读回、真桌面WGC、普通双EXE可见输出、物理跨机或多屏扫描；PNG只核首帧，不冒称连续更新。

相关Rust完整名去重192 passed / 0终态failed / 1 ignored（Otter库155、Loom库32、普通固定SHA EXE工作区5），不是整库。17项Source Application逐例独立进程通过；packet/queue/Endpoint、旧媒体运输/socket/像素、LAN原始请求、授权、Tile几何与窗口清理均实际重验。普通EXE工作区5.469秒核启动、Checkpoint、中断重开、末Unit删除不复活及SHA准入，--tile-outputs实际枚举通过，不等普通Tile输出进程可见启动。
input1严格Clippy缺少提取前由父模块传入的io/BridgeStream/StreamOwned/Read导入，exit101失败保留，未运行测试。input2补显式导入并完成不可变Frame期限/实际paint快照围栏，冻结4249项后严格Clippy227.797秒、lib386.187秒、普通产品232.969秒、workspace180.766秒、Loom库67.797秒全部构建通过。审查确认新streaming测试在最后Consumer退出与100ms超时发布间有竞态；input3只修改该cfg(test)叶，将Closed/NoConsumers正常退休而其它错误仍失败，并在Loom专属测试补LAN单子协议列表拒绝。严格Clippy137.609秒、Otter库250.032秒、Loom库52.203秒重编通过；生产/普通产品/workspace按完整test-only delta/SHA复用，全部本批运行测试均对应input3，没有套用旧源码成绩或放宽15/30秒期限、预算或鉴权。
格式/版本/Otter strict行数及两仓Git whitespace通过，37份Rust最大478有效行；组织/Web合同与本地文档检查由最终回执登记。Loom strict daemon Clippy仍101条原诊断、逐签名/次数无新增；全仓strict行数仍因旧ArtEditDialog.tsx例外2026-10-31到期exit2，未延期/绕过，不称两仓全绿。
集中证据main-tile-live-1含冻结输入、scoped源码/补丁、独立联合oracle、逐门耗时和final-audit/STATUS；运行后只更新两仓五Markdown。普通开发EXE按SHA另存，未制作新完整手测包，不覆盖v464固定ZIP。Art/输入/识别Tile、普通可见双产品、物理输出热插拔/跨机/多屏同步、跨epoch/重新授权、本地Art触发合同、旧Web互通、历史前台根因、资源长稳、最终整包和用户全面手测仍开放；总目标active，三旧副线保持冻结，无提交/推送/发布/部署/安装或删除用户文件。

## v501：Tile 输出失效后的恢复（实现、定向与隔离联合通过）

一个稳定物理输出由tile_output_owner统一管理：枚举错误/缺席/重复身份/超预算/坐标溢出不绑定其它屏；尺寸/位置变化或WM_DISPLAYCHANGE/WM_DPICHANGED先隐藏并销毁旧窗口、停止旧Runtime。只有旧线程实际结束、原Grant仍有效才申请两份新Lease、重建唯一Runtime/窗口；缺席期间每500ms重新枚举，短原生状态窗保留停止。用户关闭或停止永久终态，过期/撤权不得借几何变化重新授权。同步paint中收到DPI消息后不回挂旧renderer、不再次ShowWindow旧窗；状态窗只在隐藏时显示，不反复调整z-order。复用原网络/授权/呈现/像素预算，无新线程、hook、依赖、权限或第二框架。

真实Loom生产库LAN HTTPS联合6.078秒通过：原管理器首次配对/批准/签名续签后退出，独立输出测试进程走原authorize及生产Output owner；只向owner注入测试枚举快照，不修改系统显示布局。2×2变4×2及位置变化使原生自行重注册，服务catalog/layout推进到3；旧Shared/像素Weak归零后才有新代际，保持endpoint身份、换新presenter lease，实际原生present与固定PBGRA源oracle均通过。临时缺席实际disconnect/offline，不同ID拒绝接管，同ID返回后重新连接且不无故改catalog。第二项9.016秒联合在恢复后撤销合成Device，核不复活/像素和线程回收/服务offline。旧图片呈现/冻结/黑场/恢复/停止17.015秒和原撤权7.329秒联合重验通过，仍沿30秒总期限与独立固定像素oracle。

相关Rust完整名去重136 passed / 0终态failed / 1 ignored（Otter库106、Loom库25、普通固定SHA EXE工作区5），不是整库；前台退出门ignored不计通过。定向覆盖稳定ID/预算/溢出/授权终态、自有状态窗停止/关闭和同步重入、display/DPI隐藏及同步paint围栏、原几何/四向旋转/alpha实际WIC像素、TLS图片/租约/取消与授权。普通EXE工作区2.937秒，实际启动/Checkpoint/中断重开/末Unit删除不复活和SHA准入通过；--tile-outputs实际枚举通过，不等普通Tile输出启动或物理多屏。

input1严格Clippy拒绝新状态窗控件ID的整数指针写法manual_dangling_ptr；原失败保留，未运行测试。input2仅按建议改为std::ptr::dangling_mut（控件ID仍1），不allow或放宽检查；重新格式化/冻结4237项，所有相关产物重新构建，生产变化没有沿用旧通过成绩。严格Otter all-target Clippy303.344秒、lib391.906秒、普通产品122.657秒、workspace147.781秒、Loom库52.078秒通过。格式/版本/Otter strict行数/两仓Git whitespace通过，11份Rust最大365有效行；组织/Web合同和文档本地检查由最终回执登记。Loom严格daemon Clippy仍101条原诊断、签名/次数无新增，strict全仓行数仍因旧ArtEditDialog.tsx例外2026-10-31到期exit2；未延期或绕过，不称两仓全绿。

集中证据main-tile-output-recovery-1保留输入、scoped源码/补丁、逐门耗时、四项联合oracle及final-audit/STATUS。运行后只更新两仓五Markdown，普通开发EXE另按产物SHA固定，不是新完整手测包，不覆盖v464固定ZIP。此批完成恢复实现与枚举注入的隔离运行，不冒充物理显示器拔插/DPI设置变更、普通Tile EXE可见输出、最终交换链读回或物理扫描。完整Live/Art/输入/识别Tile、旧Web实际互通、普通可见双产品/物理跨机/多屏同步、长稳、本地Art触发合同、跨epoch/重新授权、历史前台根因、最终整包及用户全面手测继续开放；总目标active，旧三副线冻结，不提交/推送/发布/部署/安装或删除用户文件。

## v500：真实 Loom 静态 Tile 联合（隔离原生进程链通过）

本批让原生管理器的真实 WM_COMMAND 消费点共用原 Config/Owner，首次配对、Loom 批准等待和签名续签后关闭管理进程；另一个独立原生测试进程从同一隔离 profile 走输出 authorize/Runtime/图片 owner/自有离屏 NOACTIVATE HWND。管理员仅启用隔离 loopback LAN、上传固定 RGBA 和提交布局，不替终端 register/connect/heartbeat，不改真实 profile、设备或用户窗口。
真实 Loom 生产库 LAN HTTPS 联合 17.016 秒通过：原生自行登记 image/scheduledPresentation、申请租约、取回 2×2 红绿蓝白源图，独立手工 PBGRA oracle 核源像素；实际 D3D12 present 后才生成回执，服务确认 layout2/scene prepared、应用时间不早于 1000ms activationDelay、冻结 rev3 保留已应用图、黑场 rev4 无 appliedRevision 且有实际黑帧回执、恢复 layout5，再显式停止/断开。第二项真实撤权联合 7.750 秒通过：已呈现后禁用合成 Device，原授权链永久停止旧 Runtime，窗口清候选，像素 Weak 归零、线程 join 且服务 offline/appliedRevision=null。
这是管理器和输出两个 lib-test 子进程，不是普通 Tile EXE 的输出选择/启动验收；nativePresentConfirmed 证明实际窗口提交返回，sourcePbgra 证明取图/解码，不是交换链最终像素读回或物理屏幕扫描。几何/旋转/crop/alpha 的独立 WIC 像素门沿 v499 未变 renderer 的同 SHA 证据，不把它算作本批重新运行。没有新增网络/截图/输入框架、依赖、权限、线程或资源预算。

本批相关 Rust 完整名去重 108 passed / 0终态failed / 1 ignored（Otter库80、Loom库23、固定SHA普通EXE工作区5），不是整库；ignored 为普通产品前台退出门，不计通过。普通 EXE 工作区回归 5.484 秒，覆盖启动、Checkpoint、中断重开、末 Unit 删除不复活及产物 SHA；实际 --tile-outputs 枚举通过，不代表真实多屏显示。集中证据 main-tile-native-joint-1 的输入、scoped、联合 oracle 与逐门回执保存完整失败和复用边界。
input1 联合在资源上传处因测试把既有 201 Created 错写成 200 而失败；原日志/隔离目录保留。input2 只修/格式化 Loom 的 tile_native_joint.rs 测试叶，生产/鉴权/超时/预算不变，Loom 库 34.812 秒重编后两项联合重新运行通过；完整 4232 项输入差异及 SHA 证明其余 Otter 三产物/80项、Loom未变21项可复用，未重复整套构建或分配新内部版本。
Otter 严格 all-target Clippy 149.875 秒、测试库 302.782 秒、普通 EXE 136.031 秒、workspace 141.531 秒及初次 Loom 测试库 107.391 秒构建通过。格式、版本、Otter strict 行数与两仓 Git whitespace 通过，5份Rust最大379有效行；组织/Web合同和文档文本/链接检查由本批最终回执登记。Loom strict daemon Clippy 仍101条旧诊断、签名/次数无新增，全仓 strict 行数仍因 ArtEditDialog.tsx 旧例外于2026-10-31过期 exit2；没有延期、绕过或扩为无关重构，不称两仓全绿。
普通开发 EXE artifacts-2/product.exe SHA256 BAD6C58B3F1286E2C3008308B02151D78EFD8741BFDF8272DC70C41F8E0C5B9A，按 input1 原字节复用；不是新的最终手测整包，不覆盖 v464 固定 ZIP。本批关闭隔离真实服务/原生管理器配对/独立测试输出的静态图片联合，不关闭普通可见双产品、物理跨机/多屏同步、普通 Tile 输出启动、自动热插拔恢复、Live/Art/输入/识别 Tile、旧 Web 实际互通、资源长稳、本地 Art 触发、跨 epoch/重新授权、历史前台根因、最终整包及用户全面手测。三旧副线冻结，总目标 active，无提交/推送/发布/部署/安装/删除用户文件。

## v499：原生 Tile 静态图片终端（定向与隔离窗口链通过）

新增独立入口 `otter-native --tile-outputs`、`otter-native --tile --loom https://host:port [--profile absolute-directory]` 与 `--tile-output monitor-<64 hex> --loom https://host:port [--profile absolute-directory]`。管理窗口请求配对/等待 Loom 批准/续签并启动独立输出；关闭管理器不结束输出，输出 Escape/关闭清理自身。原生输出不加载 WebView、不注册普通截图/托盘/输入 hook，不操作真实配置或设备作为自动测试。
本批静态图片链包含原设备身份、精确 Wall HTTPS、CAS 注册、唯一 presenter、后台单图片 owner、源摘要/MIME/尺寸及 16图/16M像素预算、非等比墙面裁剪/四向旋转/透明 z-order、场景时钟、冻结/黑场、实际呈现回执、授权超时/撤权/旧任务失效与退出排空。沿既有产品 D3D12→D3D11→CPU 选择和关联准入；没有另造截图/输入/图片/网络框架或恢复 ANGLE。Loom LAN 仅新增图片终端必需 Device/nonce 路由，仍执行原 512KiB 请求及 10MiB 状态/24MiB 图片响应专属上限；其它客户端默认上限不变。
显示输出失效或 DPI/几何变化目前先清屏、安全停止旧代际并退出，显式重新启动后重新注册；不称已完成自动热插拔恢复。只广告 image/scheduledPresentation；Live/Art/输入/识别 Tile、物理同步/跨机、旧 Web 实际互通、长稳及最终整包仍开放。集中输入/失败/构建/定向回执位于 main-tile-image-terminal-1；本批结果如下，不代表完整 Tile 或最终迁移验收。

最终相关Rust完整名去重174 passed / 0终态failed / 2 ignored（Otter库115、Loom库54、固定SHA普通EXE工作区5），不是整库。18项新增Otter门覆盖严格DTO/几何/时钟/代际、固定HTTPS/图片完整性与PNG实际WIC像素、旋转/crop/alpha独立手工矩阵、未知connect/401和真实关闭清理；Loom两项新原始LAN边界门与既有Wall/Surface回归通过。TLS/图片四项定向组合计13.891秒，其中Runtime→独立TLS夹具→图片owner→自有离屏NOACTIVATE HWND→actual present回执→heartbeat的图片/冻结/黑场/恢复/停止链通过，immutable源只读一次、最后Weak Pixels归零。它不是实际Loom图片联合，也不证明物理扫描同步。
管理器消息泵同时消费命令与关闭时不再启动旧待办，标题不写已毁HWND；同步绘制重入关闭后不再执行旧窗口显隐操作。input3新重入门因隐藏窗UpdateWindow未同步派发paint而在alive断言失败；input4仅修cfg(test)叶的前置条件，先显示自有离屏NOACTIVATE窗再安装关闭回调，保所有断言/期限/预算，复核通过。该失败、初始check与input1 Clippy失败都保留，未改生产以迎合测试。两个ignored分别是Loom显式PNG比较基准和普通产品前台退出门，不计通过。
Otter最终strict all-target Clippy 169.515秒、lib 226.813秒通过；生产输入在input3后不变，普通产品111.953秒/workspace目标141.328秒按完整test-only delta和SHA复用，普通EXE工作区3.359秒5项通过并实际运行--tile-outputs。Loom源码始终沿input2，库84.5秒与54项结果按完整manifest/SHA复用，没有再次编译；没有构建新的配套普通Loom daemon包。格式、Otter strict行数、版本、两仓Git whitespace以及组织/Web合同和本地文档检查由本批最终回执登记；33份Rust最大485有效行。Loom严格daemon Clippy仍101条原诊断且无新增，strict全仓行数仍因ArtEditDialog.tsx旧例外于2026-10-31到期exit2，不延期或绕过，不称两仓全绿。
续接摘要误记Loom target，导致一次错误中断原有效阶段和一次冗余目录编译；核v498原始artifact source后纠正，全部部分日志/两次自有进程中断证据保留，不冒称原runner有目录缺陷，不终止他人进程或删除目录。最终4228运行输入、运行后只更新两仓五Markdown，源码/产物无漂移；main-tile-image-terminal-1/STATUS.md与final-audit.json给出精确范围。普通开发EXE artifacts-4/product.exe SHA256 05B6783BD054C2A44F9C3065DC7435BC2F21C41AA96FA7238B6DB7B140AE7F1D，沿input3原字节，不是新的完整手测包，不覆盖v464固定ZIP。
真实Loom静态Tile联合/原生管理器实际配对与独立输出、自动热插拔恢复、Live/Art/输入/识别Tile、旧Web实际互通、普通可见双产品/物理跨机、多屏同步/资源长稳、历史前台根因、本地Art触发合同、跨epoch/重新授权、最终整包及用户全面手测继续开放。Shader/CAPTURE/PNG冻结、真实profile/设备不动，无提交/推送/发布/部署/安装/删除用户文件；总目标active。

## v498：Source控制与观察服务安全恢复（定向及隔离联合通过）

Source控制GET只在同Grant/来源/epoch恢复，保原可靠cursor与捕获/Publication；首次失联先实际Release/suspend，再通过原媒体owner受控WSS重连让服务产生可靠controller=None屏障，不发额外撤权POST。旧输入只验证消费，恢复后仍须新的显式获取。四次400/800/1600/2000ms退避、完整成功读周期后才清控制恢复；正常终态、TLS/协议/历史reset/错epoch/容量与授权错误继续失败关闭。媒体/控制/观察标志独立，socket尚未安装时单重连意图不丢失；不增加线程、timer、队列、权限、依赖或预算。
观察使用冻结ObservationSubmission保原候选/actor；未知POST后只认证GET对账，不因快照仍为旧cursor而重发原POST。仅Client在任何HTTP/TLS发送前产生的Connect可重试未发请求。完整Source来源/几何/epoch/能力和唯一完整Observation均须匹配；保原20秒候选期限、3秒HTTP及四次暂态退避，原Lease停止唤醒，只有真实确认且未取消才commit Tracks。迟到/取消不新发候选；已到服务端的请求不可撤回，关闭仍按原游标对账、未知关闭不伪报Confirmed。

当前input1相关191项Rust完整名去重通过/0终态失败/9 ignored，含42项Application，不是整库。29组定向包含真实TLS控制503恢复/可靠cursor/独立nonce/旧输入不执行/None之后新grant、四次耗尽、401/reset/错epoch终态、停止/撤权唤醒；观察500→旧cursor→GET503→完整确认过程中恰一次POST、错ack/错payload/重复/来源/几何/epoch/能力/超前cursor与取消均通过。观察故障是确定性TLS矩阵，不冒称真实Loom丢观察回复联合。
真实Source菜单→Loom生产库LAN HTTPS/WSS控制GET断流联合2.562秒：原worker实收A键/鼠按下后实际释放，同一捕获/Source身份、窗口、epoch1继续发布；服务controller=None且精确2×2 raw像素仍为[17,83,211,255]，新的显式acquire后B键/鼠实际执行并释放，关闭后角色/permit归零。控制侧沿既有权威store夹具，不冒充Viewer公共HTTP/UI全链。原UIA观察到真实Loom稳定value-seven→value-eight/密码排除联合5.469秒；Source媒体持键鼠恢复1.969秒、Source H264恢复5.391秒、Viewer媒体/控制恢复5.875/6.031秒和原完整Viewer→Source输入释放5.735秒回归通过。
普通固定SHA EXE工作区5 passed/1前台退出ignored，4.031秒，核启动、Checkpoint、中断重开和末Unit删除不复活；另外8 ignored为两旧显式联合、系统codec专门门、两UIA child入口及三外部H264 fixture门，不计通过。自有离屏NOACTIVATE窗口/合成WARP源不等普通可见双产品、真桌面WGC/用户应用或物理跨机。

严格Otter all-target Clippy183.844秒、lib443.812秒、普通产品177.656秒、workspace150.360秒及Loom测试库87.485秒构建通过；格式、Otter strict行数、版本及两仓Git whitespace通过，12份Rust最大446有效行，无生产修复后沿用旧构建。Loom只有既有cfg(test)联合叶变化；同口径daemon严格Clippy仍101旧诊断，签名/次数无新增，strict全仓行数仍因ArtEditDialog.tsx于2026-10-31过期的旧例外exit2，不延期、调时或跳过，不称两仓全绿。
首轮组织/Web合同44 passed/1 failed：OtterNamingContract在普通产品构建并行期间以原5000ms限制超时（5026ms）。失败原日志保留，没有增加timeout或修改合同；随后只读执行原扫描函数实测1160文件/约7.9MB、819.958ms且无命名违规，这只是诊断，不代替Vitest，资源竞争仍仅假设。构建完成后的最终组织合同和文档链接复核以本批final-audit/STATUS实际回执为准，不用单次复核抹掉原失败或声称已定位其根因。

证据main-source-service-reconnect-1/inputs-1.json、scoped、原失败日志、命名诊断、联合oracle与最终STATUS/final-audit。运行冻结4203项；运行后仅更新三Markdown，生产/测试及四产物不变。普通开发EXE artifacts-1/product.exe SHA256 D425EA2D595B1176A0383346749F6F03E335C3C168E63395E3C50AAE9DBB1997，不是新最终手测整包，不覆盖v464固定ZIP。跨epoch/重新授权、本地Art触发合同、完整LiveRelay/Tile、旧Web实际互通、普通可见双产品/真桌面WGC/物理跨机、历史前台根因、资源长稳、最终整包及用户全面手测仍开放。三旧副线/真实profile与设备不动，不提交/推送/发布/部署/安装/删除用户文件。

## v497：Source媒体发布同身份断流恢复（隔离联合通过）

同Grant/来源/epoch的异常WSS断链复用原Consumer、捕获、Publication、单媒体线程和单调frame_id；每次新nonce并重核原Lease，不再次create/续签，不跨epoch或重放未知完成的AU。静止图只保一份已有GPU Arc，重连等待新策略并发新raw/独立IDR，codec失败跨恢复仍sticky。400/800/1600/2000ms最多四次，握手/本地send不重置；只有认证快照核完整来源/几何/epoch、sourceConnected=true且lastFrameId等于本次新帧，才退出恢复并刷新次数。确认期间有界GET，不每帧常驻轮询；原3秒请求限额及确认循环、250ms分片、预算不扩张。
原串行输入owner在媒体generation变化后实际Release并suspend，旧输入只验证/消费、不重放；可靠controller=None后才接受新的显式获取。恢复短状态“Live 重连中…”保留停止，禁止重复发布；原控制/观察线程join、同身份close与清理状态保留。正常Close、授权/TLS/协议/容量错误仍终态，无新线程、timer、hook、队列、依赖或权限。Source控制GET/观察写入失败仍沿原终态，本批没有实现它们的恢复、跨epoch或重新授权。

相关177项Rust完整名去重通过、0终态失败、7 ignored，含41项Application；不是整库。26组定向覆盖实际TLS双WSS、新nonce/一次create/帧1→2、落后水位不误确认、握手成功但确认503恰耗尽四次、停止/撤权唤醒和资源归零、终态分类、认证来源/几何/epoch替换拒绝、旧控制屏障与新显式grant、系统encoder重连IDR/sticky回退。统计逐日志校验libtest passed/failed/ignored总数，支持nocapture多行输出，不将ignored计作通过。
真实Source/Viewer菜单→Loom生产库LAN HTTPS/WSS→系统H264原生解码/上传联合10.797秒：仅断本测试Source WSS，保原捕获身份、窗口、epoch1、Source actor及单Viewer连接；服务独立确认新IDR帧3，实际像素[17,84,212,255]沿独立[17,83,211,255]/预定有损容差10核验。继续关键帧请求、旧Viewer raw/恢复、显式fallback，停止后角色/permit归零。持键鼠断流联合2.109秒核原worker实收A键/鼠按下后实际释放、Source继续发布且controller=None，再显式acquire并实收B键/鼠按下与释放；控制侧沿既有权威store夹具，不冒充Viewer公共HTTP/UI全链。
Viewer媒体恢复回归7.328秒、控制GET恢复6.047秒、原Viewer→Source菜单键鼠/释放4.828秒通过。普通固定SHA EXE工作区5 passed/1前台退出ignored，4.532秒，覆盖启动、Checkpoint、中断重开、末Unit删除不复活及产物身份。其余6 ignored为三项外部H264 fixture门及三项旧Source只读/观察/Viewer触发联合，真实新联合另显式执行；自有NOACTIVATE离屏窗口和合成WARP源不是真桌面WGC、人工菜单导航、普通可见双产品或物理跨机验收。

input1两个cfg(test)编译错误保留于clippy-1：TLS夹具流误写TcpStream、分类测试越过私有wrapper；input2仅修测试流类型/共用分类入口并补既定显式再获测试，生产不变。input2严格Otter all-target Clippy144.859秒/lib387.500秒/普通产品202.282秒/workspace172.563秒及Loom库89.437秒通过。26组、系统codec、两新Source联合、媒体回归、44项Application计划/工作区/旧输入均实跑。控制恢复回归在初始source_connected断言失败：101响应先于服务媒体worker启动，child ready只证明本地send，旧父门缺远端接收屏障。input3仅将既有首raw帧接收及独立像素校验移至该断言之前，保全部断言/3秒接收/30秒总期限；不改生产或刷绿。Loom库43.218秒重编，共用夹具三项恢复联合全部重验；全部Otter输入/三产物、未受影响的input2结果按完整delta与SHA复用，不冒称全在input3重跑。
Otter strict Clippy/所需构建、格式/strict行数/版本及两仓Git whitespace通过，本批23份Rust最大435有效行。Loom只有两份cfg(test)联合叶变化；同历史daemon all-target --no-deps严格Clippy仍101旧诊断、签名/次数无新增，全仓strict行数仍因ArtEditDialog.tsx的2026-10-31过期例外exit2，未延期、调时或绕过。组织/Web六suite45项、本地链接/UTF-8/scoped与最终输入审计由本批收尾回执记录。

证据main-source-reconnect-1/STATUS.md、final-audit.json、tests-terminal.json、inputs及scoped补丁保留失败/复用边界。4201项最终输入，运行后只更新三Markdown；27项差异/23份Rust。普通开发EXE artifacts-3/product.exe，SHA256 669C633B63DEDDFE20258C92F96045F64D014B8DFEE440B519533D8838B9FA3C；不是新最终手测整包，v464固定包不覆盖。Source控制/观察恢复、跨epoch/重新授权、本地Art触发合同、完整LiveRelay/Tile、旧Web实际互通、普通可见双产品/真桌面WGC/物理跨机、历史前台根因、资源长稳、最终整包及用户全面手测仍开放。三旧副线/真实profile与设备不动，不提交/推送/发布/部署/安装/删除用户文件。

## v496：Viewer控制GET暂态断线恢复（隔离联合通过）

只恢复同Grant/来源/epoch的控制事件GET和事件触发的注册刷新GET。原Owner/cursor/Permit/媒体不替换，每请求新nonce，不再次join/续签；400/800/1600/2000ms最多四次退避，完整认证/解析/应用的成功读周期才退出恢复。已提交事件批次的注册刷新只重试GET，不重复消费或丢失刷新，也不用新快照跳过可靠事件。POST结果不明仍失败关闭，不重放写请求。
控制/媒体恢复标志独立，任一未恢复都禁输入；损失清待发/减权/保释放屏障，恢复不自动重获，迟到ACK或另一链恢复不能复活控制。trigger待发意图被拒绝，Unknown不伪确认。停止/撤权唤醒原Lease等待；没有新增线程/timer/队列/依赖/权限或资源预算。证书/协议/历史reset/错误epoch/超限仍终态。

input2同源集中运行153项Rust完整名去重通过、0终态失败、7 ignored，含41项Application，不是整库。实际TLS 503/截断body→同cursor恢复、新nonce/无第二join、恰一次消费、注册GET恢复、四次耗尽、停止/撤权唤醒及终态分类通过；原Client证书/主机名/header限额/期限/取消/资源组均重验。真实Source/Viewer菜单→Loom生产库LAN HTTPS/WSS→原生H264联合6.265秒：只中断自有在途控制GET，原cursor4→服务独立确认的5，保同HWND/epoch1/actor序号/单媒体连接，画面[17,84,212,255]沿独立[17,83,211,255]/预定有损容差核验，未自动重获控制；后续关键帧、旧Viewer raw/恢复、显式fallback及角色/permit归零通过。
受影响媒体断链联合回归5.859秒、Viewer→Source键鼠/释放联合5.000秒通过。普通固定SHA开发EXE工作区5项通过/1前台退出ignored，4.203秒，覆盖启动、Checkpoint、中断重开、末Unit删除不复活及产物身份。其余6 ignored是三项外部H264 fixture门及三项旧Source只读/观察/Viewer触发联合，不计通过；真实H264联合另行显式执行。不是人工菜单导航、普通可见双产品或物理跨机验收。

input1既有Client响应头限额断言失败保留于main-control-reconnect-1/focused-13-1：满16KiB接收缓冲的零长read被误作可重试EOF。input2只修loom_http_transport.rs，缓冲耗尽明确InvalidData/Http终态，不改断言或扩大容量/超时；Otter全相关产物重建并重跑。input2严格all-target Clippy260.438秒/lib437.969秒/普通产品192.000秒/workspace189.422秒通过；Loom输入完全未变，固定input1库按完整delta及SHA复用，不称重新构建。Otter格式/版本/strict行数、两仓Git whitespace通过，18份Rust最大427有效行。Loom仅cfg(test)联合变化；同口径daemon严格Clippy仍101旧诊断、精确签名/次数无新增，全仓strict行数仍因ArtEditDialog.tsx的2026-10-31过期例外exit2，未延期、调时或绕过。组织/Web六suite45项、本地链接及最终输入审计由本批集中回执收口。

证据main-control-reconnect-1/STATUS.md、final-audit.json、tests-terminal.json、inputs和scoped补丁记录失败、同源运行及复用。4198项最终输入，运行后只更新三份Markdown；22项变化/18 Rust。普通开发EXE artifacts-2/product.exe，SHA256 79252BAFAF10D4AC11824781B43DCD2AB3A0A487783A8D57D7C58A58E9502E61，不是新最终手测整包，v464固定包不覆盖。Source发布恢复、跨epoch/重新授权恢复、本地Art触发合同、完整LiveRelay/Tile、旧Web实际互通、真桌面WGC/普通可见双产品/物理跨机、历史前台根因、资源长稳、最终整包及用户全面手测仍开放。三旧副线/真实profile与设备不动，不提交/推送/发布/部署/安装/删除用户文件。

## v495：Viewer同授权/同来源/同epoch的媒体断链恢复（隔离联合通过）

只恢复异常WSS媒体链：原控制owner、Grant、Session、Permit、窗口及codec worker不替换；每次核原绑定/固定origin/期限/撤销并签发新nonce，不再次join、不续签。复用原network线程park/unpark退避400/800/1600/2000ms，一个无新像素进度故障周期最多四次；握手成功或重复预热不重置，只有当前epoch真正提交的新像素才退出“重连中”。停止立即唤醒退避；正常Close、Device撤销、TLS/upgrade/profile/协议/帧/worker错误仍终态。
raw/JPEG只忽略新连接初始已知水位内的重复预热，新帧后继续严格拒绝回退；H264沿原参考链，worker sticky fallback跨恢复保持legacy offer，不复活失败codec。媒体损失清待发输入并减权，保释放屏障；迟到acquire不能复活控制，恢复后仍须显式重新获取。保旧图不代表已恢复，只有旧静态raw时控制仍禁用。无新增线程/timer/队列/依赖/权限，不扩大原四槽、编码或像素预算。

本批input2同源集中运行：124项Rust完整名去重通过、0终态失败、7 ignored，含41项Source/Viewer Application逐例隔离通过；不是整库。定向核同Permit/游标/worker、重复raw、不跨epoch刷新次数、四次上限、停止唤醒、工厂授权失败、新nonce且无第二join、撤权/预算归零、输入释放屏障和fallback保持。真实Source/Viewer菜单→Loom生产库LAN HTTPS/WSS联合4.437秒：本测试socket断开后原生H264帧2→3，同一个HWND和epoch1，实际像素[17,84,212,255]沿独立[17,83,211,255]/预定有损容差核验，未自动重获控制；随后关键帧请求、旧Viewer raw/恢复、显式fallback及Source/Viewer角色/permit归零通过。原Viewer→Source键鼠/释放联合4.125秒通过。
普通固定SHA开发EXE工作区5项通过/1前台退出ignored，3.078秒，核启动、Checkpoint、中断重开、末Unit删除不复活及产物身份；不是普通可见双产品。其余6 ignored为三项缺外部H264 fixture的显式门及三项旧Source只读/观察/Viewer触发联合，不计通过；缺fixture没有通过安装工具或构造同实现自证绕过。

input1严格编译的Recovery可见性/两项测试导入失败保留，input2集中修复并补Worker fallback回归，随后全部所需产物重建并实跑，未沿用input1成绩或放宽timeout/鉴权/预算。Otter严格all-target Clippy154.844秒/lib394.984秒/普通产品158.328秒/workspace133.781秒及Loom库47.657秒通过。Otter格式、版本、strict行数及两仓Git whitespace通过；本批22份Rust最大417有效行。Loom仅cfg(test)联合变化，同历史daemon all-target --no-deps严格Clippy仍101条旧诊断、精确签名/次数无新增，非全仓通过；strict行数仍因ArtEditDialog.tsx的2026-10-31过期例外exit2，未延期、调时或跳过门。组织/Web合同、本地链接和最终输入审计由main-viewer-reconnect-1集中回执收口。

普通开发EXE：main-viewer-reconnect-1/artifacts-2/product.exe，SHA256 00F2CF77CB9E4812C2F4FF2F28F29F3E5EF4676852464F49DE65BE7E572EF079；不是新最终手测整包，v464固定包不覆盖。4195项冻结输入，运行后只更新三份Markdown。此批不包含控制HTTP断线恢复、Source发布恢复、跨epoch/续签/重新加入、本地Art触发、完整LiveRelay/Tile、旧Web实际互通、普通可见双产品/真桌面WGC/物理跨机、历史前台根因、资源长稳、最终整包或用户全面手测。三旧副线/真实profile与设备不动，不提交/推送/发布/部署。

## v494：Live Source H264编码、策略与关键帧、raw回退（隔离产品链通过）

Source在原媒体线程创建系统同步baseline编码器，成功才广告H264/30fps/30帧IDR；不支持的小图/奇数尺寸或预算不足保raw。BGRA直接写BT709 limited NV12，无第二份全帧Vec；透明输入不丢alpha而退休codec，发送原raw。逐AU限1MiB、Annex-B/SPS/PPS/IDR及精确timestamp校验，单输入/输出，失败不积压sample。MFT临时输出分配独立计费：实际1080p GetOutputStreamInfo声明3916800字节，不能误用1MiB wire上限拒绝。四槽、共享编码预算和Pixels预算均不扩大；较大尺寸/系统不支持仍可raw，不承诺所有尺寸必用H264。
真实video_policy由Loom按实际Viewer能力聚合；没有Viewer或有旧Viewer时raw，全部支持才H264。epoch固定本连接且请求序号不回退，变化及周期要求IDR。原stream仅保留一份已有不可变GPU帧Arc，静止画面也能响应请求，不新增截图/网络线程或高频采样。实际选中子协议必须属于真实offer；H264连接继续接受raw。codec错误sticky raw；只有仍健康的codec在服务策略恢复时从IDR继续，不由策略复活失败codec，也不包含自动重连/epoch恢复。原Device/nonce、Lease、100ms控制、250ms分片、取消/线程join/同身份close保持。

相关Rust完整名去重108 passed / 0 failed / 6 ignored，不是整库。50项使用input3的未变生产/定向测试终态，58项在input5实际运行；input4只加cfg(test)无敏感数据诊断，input5只修两份cfg(test)联合oracle叶，完整输入差异与产物SHA证明后复用，不能称全部在input5重跑。41项Application通过，其中37项常规逐例独立进程，4项由两真实联合父门显式运行。系统编码→既有decoder的64×64/680×430/1920×1080独立红色oracle、IDR/delta、周期/请求IDR、alpha/cancel/重放/预算与租约回收通过；原decoder协商/取消/名额回归亦通过。
真实Source/Viewer菜单→Loom生产库LAN HTTPS/WSS联合3.984秒通过：仅一份静止64×64 GPU源；无Viewer先raw，首H264帧2实际呈现[17,84,212,255]，独立源oracle为[17,83,211,255]且沿预定有损容差；WS新Viewer与显式keyframe_request分别产生IDR帧3/4。旧Viewer引发raw帧5精确像素、退出后恢复IDR帧6、显式video_fallback后raw帧7，原生窗口均按服务确认的精确frame_id核像素。停止后Source/Viewer角色和media permit真实归零；不把解码后RawBgra当wire codec。原键鼠/菜单及失焦释放联合4.094秒回归通过；普通EXE工作区5 passed/1前台退出ignored，2.703秒，覆盖启动/Checkpoint/中断重开/末Unit删除不复活及SHA准入。

input1测试编译缺Arc导入、input3联合及带backtrace诊断失败均保留。联合失败是测试错误地在已解码像素层等待codec H264，不是通过增采样、放宽期限、删断言或改生产合同刷绿；input5改由真实服务/WS确认wire身份，再关联原生像素。系统低延迟属性实际要求VT_BOOL，拒绝的B帧/GOP属性未强行写入。input3严格Clippy112.344秒/lib190.312秒、普通产品79.656秒/workspace82.219秒通过；input4/5仅测试变化，Clippy分别112.172/107.954秒、lib177.296/223.562秒通过。Loom库input2 52.922秒、input5测试修复后40.375秒；未变生产产品/workspace沿input3固定SHA复用，不另分内部版本。
Otter严格all-target Clippy、所需构建通过；格式、行数、版本、组织/Web合同、本地链接及最终文档/输入审计见集中回执。Loom本批只有cfg(test)新联合和登记；同历史daemon all-target --no-deps严格Clippy仍101条旧诊断，无新增，不称全仓绿。Loom strict行数仍因ArtEditDialog.tsx的2026-10-31过期例外exit2，未延期、调时或取消门。
集中证据main-source-h264-1的STATUS.md、final-audit.json、tests-terminal.json、inputs及scoped补丁记录真实dirty基线/失败/复用。普通开发EXE SHA256 B9B2BAA7AF0FEE248DD73145EBA488C5BD3A011155162F0EDE447D752FE438CE，非新手测整包，不覆盖v464。6项ignored不计通过：既有两项系统H264 fixture/sequence专门门、三项旧Source只读/观察/Viewer触发联合、本批未跑的普通产品前台退出门。真桌面WGC、人工菜单导航、普通可见双产品/物理跨机、本地Art触发合同、重连/epoch、完整LiveRelay/Tile、旧Web实际互通、历史前台根因、资源长稳、最终整包及用户全面手测仍开放。三旧副线、真实profile/设备不动，不提交/推送/发布/部署。

## v493：Viewer触发配置到真实Surface动作终态（隔离完整联合通过）

已授权真实Surface宿主的Viewer系统菜单新增“配置触发条件”，可显式新增/编辑/禁用高级JSON配置；固定本次Surface/attachment，不给LocalArt填伪身份。完整JSON最多16KiB，operand沿原4KiB/16层/512节点，最多64项注册，单pending/inflight。先释放旧键鼠控制，再以原串行线程/Lease和Sender唯一actor游标发送；前置核身份/成员/双游标/恰好下一revision，仅POST一次，再GET完整唯一注册及本人control恰好next/input不变后确认。明确拒绝保观看，未知结果关闭旧观看且不重放；stop清待发并阻止迟到发布。
GET只刷新注册，不导入可能超前的observation/audit或推进可靠cursor；可靠trigger_condition事件另触发注册刷新。Viewer不自行求值或执行Art；禁用不承诺撤回已reserved动作，Fired不等正式业务结果。无新依赖、权限、线程、hook、timer、网络或截图框架。

真实联合13.390秒通过：实际自有Source UIA读取value-seven→value-eight且排除密码，Viewer从真实HWND经过生产配置提交接缝新增rev1/更新rev2；Loom生产库LAN HTTPS/WSS求值，通过原Surface executor启动当前真实runtime-host与rustc构建的隔离Art handler，不使用MZ文本或runner注入。父oracle独立核actionRequestId与eventAck requestId相同且succeeded、权威price=101/实际attachment快照文本="101"/正式outputs.price=101，磁盘marker恰一个字节证明一次执行。禁用rev3/actor4后另一个新观察心跳不再执行，Viewer/Source关闭后role与media permit归零。此门不是人工菜单导航、普通可见双EXE、Surface最终绘制像素或物理跨机验收。

相关Rust完整名去重113 passed/0 failed/2 ignored：41项Application实际通过（37项常规逐例独立进程与4个联合子门）、新原生模态/菜单、14场景TLS矩阵所归属的两项HTTP测试、唯一actor释放→配置→再获取、可靠观察不被GET污染及直接回归均通过。原真实Viewer→Source键鼠/菜单及失焦释放联合3.218秒通过。普通EXE工作区5 passed/1前台退出ignored，2.703秒，覆盖启动/Checkpoint/中断重开/末Unit删除不复活及SHA准入；另一ignored是本批未单独重跑的旧只读Source发布联合，不计通过。14场景不是14条测试，编译不计passed。

input2严格Otter all-target Clippy147.406秒/lib298.297秒、普通产品112.890秒/workspace118.282秒、Loom lib51.469秒、实际runtime-host40.203秒/handler0.516秒构建通过。input3只修观察模式不授输入能力的共用联合夹具，Clippy103.906秒/lib170.578秒；input4只补新夹具显式能力字段并修旧文字模态oracle，Clippy114.438秒/lib185.187秒；input5只把bootstrap batch与同一个cursor共同移交Owner，Clippy115.781秒/lib202.141秒，interaction组9项重验通过。其余104项终态在本批input3/4实际运行，按完整cfg(test)差异、未变生产前缀/测试依赖及固定SHA复用；普通产品/workspace、Loom/framework/handler始终沿input2未变产物，不反复构建或把全部结果冒称新库重跑。最小check/input1编译及input3/4夹具失败全部保留，没有修改生产校验、放宽timeout/预算/鉴权或重试刷绿。

Loom本批仅cfg(test)联合/fixture变化，生产输入不变。包含依赖的strict Clippy仍失败；同历史口径daemon all-target --no-deps为101条原诊断，code/message/primary-file及次数比对无新增，不称全仓成功。当前Loom strict行数仍因ArtEditDialog.tsx例外于2026-10-31到期exit2；未延期、调时或扩大到无关UI重构。Otter strict行数通过，手写变化31份Rust最大488有效行；原manifest额外列出的三份native/target生成Rust仅是输入审计附项，不是本批实现或交付文件。

格式/版本/UTF-8无BOM/scoped与Git whitespace、组织/Web合同、本地链接、最终文档输入与产物身份由main-live-trigger-1/STATUS.md、final-audit.json集中收口。普通开发EXE SHA256 1BAC4339D54834C558BBC3CD9C9A170CED1BFBEBEF7AB01D4871A357B4A19B96；不是新手测整包，不覆盖v464。本批关闭真实Surface的高级JSON触发配置/求值/动作正式终态隔离闭环；本地Art触发合同、H264 Source、重连/epoch、完整LiveRelay/Tile、旧Web实际互通、历史前台、资源长稳、普通可见双产品/物理跨端、最终整包及用户全面手测仍开放。三旧副线、真实profile和设备不动，不提交/推送/发布/部署。


## v492：Source显式UIA观察采集与发布（定向及隔离LAN联合通过）

原生Live系统菜单新增显式“发布 Live 并共享窗口观察”。普通发布和允许控制不默认读UIA，观察不授输入能力；Region/裁切窗口拒绝整窗观察，避免读取选区以外内容。通过原Device/nonce/HTTPS观察路由发布，不新增依赖、权限、全局hook或第二截图/网络框架。
live_source_uia/sample只在受限MTA child读取显式窗口；process复用bounded_worker_process与overlapped Pipe，128MiB/child、四份512MiB池、单在途扫描2秒、256节点/16层/512KiB响应，密码与跨PID子树排除，不执行Pattern动作。live_source_observation拥有两样本稳定/1秒心跳/缺失Stale与失败Error、256项/512KiB累计快照，仅真实ack或精确只读对账后commit；runtime独占同Grant独立Lease/串行观察线程和child回收。
原Source媒体owner在停止后join观察/回收child，再以原媒体Lease对账actor并close；不再固定close=2，观察ID另有序号。迟到扫描不新发、迟到ack不commit；已经到达服务端的请求不能撤回，close前按同锁member快照核source/device/hook/epoch、cursor与完整唯一payload，不盲重放。2秒是扫描pipe期限，不是包含驱动/内核清理在内的硬实时保证。

当前同源相关Rust完整名去重71 passed / 0 failed / 3 ignored，Source及Viewer观察19项Application逐例独立进程。自有离屏NOACTIVATE标准EDIT到受限child的真实值变化、密码排除、取消/Job/容量回收通过；状态/HTTP覆盖稳定、心跳、locator变化、失败/缺失、丢ack精确对账。实际Source菜单→真实Loom生产库LAN HTTPS观察路由的stable seven→eight/密码排除/close联合4.67秒通过；原Source输入/释放0.982秒及Viewer→Source菜单输入/释放4.076秒回归通过。普通EXE工作区5 passed/1前台退出ignored，2.809秒，覆盖启动/Checkpoint/中断重开/末Unit删除不复活及SHA准入。
严格Otter all-target Clippy 105.117秒、库175.194秒、普通产品45.512秒/workspace目标47.016秒构建、格式/strict行数通过。input1 Clippy失败和input2 UIA/三项观察Application失败保留；最小实验定位windows-core0.60成功null接口表现为Error::empty(S_OK)，input3仅修walker空端点并补真正E_POINTER等错误传播回归，没有放宽期限/安全门。Loom仅扩cfg(test)联合消费者，生产输入未改；联合库以完整Loom输入manifest/固定SHA复用input2成功产物，不重复编译。Loom旧daemon101/protocol10 Clippy及ArtEditDialog过期行数例外仍开放，不称两仓全绿。
格式/版本/UTF-8无BOM/scoped与Git whitespace、组织/Web合同和本地链接由main-source-uia-1/final-audit.json与STATUS.md集中收口。普通开发EXE SHA256 4096F288F5EAA8548879CF8F5330D054B0BE1D36A238FADDE25F69D78EB750EC；不是新手测整包，不覆盖v464。
本批关闭Source显式UIA观察采集/发布/停止的定向与隔离LAN联合，不关闭实际Viewer可见观察窗口/触发配置与执行全链、H264 Source、重连/epoch恢复、完整LiveRelay/Tile、旧Web实际互通、历史前台、资源长稳、普通可见双产品/物理跨端、最终整包及用户全面手测。三旧副线/v464固定包/真实profile与设备不动，不提交/推送/发布/部署。


## v491：原生Viewer观察与触发审计查看（定向及共同控制链回归通过）

原生远程观看系统菜单“查看观察与触发记录”现可显示观察状态/值/来源/置信度/时间、加入时触发注册快照及最近16项审计。reservation显示“派发中”，不伪报Surface动作完成；Viewer不重新计算条件或执行Art。只在用户显式打开时格式化，单值512字符/整窗128Ki UTF-16单位，超限明确标记，NUL/换行/bidi控制可见转义；敏感文本不写普通日志/磁盘。不新增线程、timer、hook、依赖、权限、网络端点或资源预算。
打开查看窗先释放原远端键鼠控制；关闭面板保留观看，撤权/关Viewer/退出销毁子控件和共享快照。更新不持Relay/Host的RefCell借用跨Win32同步调用；重入关闭不回填旧owner。相同语义身份不重写EDIT，语义更新保留选区/垂直阅读位置；失败只关闭查看窗并留下可重试入口，不将只读面板错误升级为文档或媒体失败。

当前同源相关Rust完整名去重85 passed / 0 failed / 1 ignored，20项Viewer Application逐例独立进程通过，其中新增两项Surface/本地Art观察快照→可靠增量→reservation/final替换→关闭重开→撤权，以及一项打开查看先释放控制。新真实EDIT/菜单/捕获排除/选区/最大容量/控制符/同步销毁回归通过；Application观察数据来自独立TLS/WSS常量peer，不是实际Loom UIA。另原真实Loom生产库LAN HTTPS/WSS→Viewer→Source执行/释放联合3.357秒通过，作为共同观看/控制链回归，不冒充新观察服务端或触发执行全链。普通EXE工作区5 passed/1前台退出ignored，核启动/Checkpoint/中断重开/末Unit删除不复活及SHA准入，不是普通可见双产品或物理跨机。
严格Otter all-target Clippy 108.311秒、库227.942秒、普通产品84.046秒/workspace目标97.07秒构建及Otter全仓strict行数通过。首次常量导入编译失败留存；input2修UI Controls导入，同时把测试容量夹具限制在原1MiB快照门内并补更新阅读位置，未放宽协议或测试断言。最终input2集中验证；库编译后复用其冻结EXE先跑定向，不与同一target竞争编译，后续联合/普通产品沿同输入完成。Loom全部源码/配置字节未变，按输入/SHA复用其原库，不重编或重跑旧门；旧daemon101/protocol10 Clippy和ArtEditDialog例外到期的全仓行数受阻仍未关闭，不称两仓静态全绿。
源码职责/格式/版本/UTF-8/scoped/Git diff、组织/Web合同、本地链接及产物身份由main-live-observation-view-1/final-audit.json与STATUS.md集中收口。普通开发EXE SHA256 FF1DB4803DC6F1AF366EBA34E4711A0CEA770175D18B5986B150A4E5F25D4CC4；不是新手测整包，不覆盖v464。
本批只关闭已授权Viewer的原生观察/注册快照/审计查看，不关闭Source UIA生产、触发配置/执行、H264 Source、重连/epoch恢复、完整LiveRelay/Tile、旧Web实际互通、历史前台根因、资源长稳、普通可见双产品/物理跨端、最终整合包和用户全面手测。三旧副线与v464固定包保持冻结，不触真实profile/设备，不提交/推送/发布/部署。以下旧版本只保留各自输入与验收边界。
## v490：本地Art宿主显式Live控制与撤权后释放（定向及真实HTTPS联合通过）

本批接通本地Art宿主已加入Live→原生系统菜单显式获取控制权→键鼠HTTP转发→释放。Preview不自动授权、不自动join、不增加JS输入/网络API，也不伪造Surface。新loom.art.local.live-control.v1及精确Device-only POST /v1/art/local/live-viewer/control限32KiB，仍核真正签名/精确binding、在线Art审批/工具/Device、已有Viewer成员、Source输入能力、原controller排他/epoch/control与input双序号。acquire/续租重核immutable源码；input不逐键重复加载JS。attach/control共享原policy围栏，不复制授权实现。
Art撤审批/过期/宿主停止后，只有自己的release允许减权，Device会话/签名/身份/原controller与序号检查仍保留；不能据此acquire/input。原Ready观看独立生存，宿主reset后禁新输入/再获取、释放控制并保留帧。复用同一串行线程、16项队列、Lease/四槽/像素预算，无新依赖、配置、timer、hook或输入框架。

17项Viewer Application逐例隔离通过，含6项新增本地Art菜单、键鼠、释放、失焦、stop、远端controller撤销、HTTP500不重放/Editor不变、宿主reset后继续观看场景。真实Art LAN HTTPS联合1.082秒通过：真正签名proof、配对Device、已有成员，acquire2/input3:1；父门独立核controller/cursor后撤审批，acquire/input403，release4成功、controller为空而membership保留。此联合明确mediaTested:false、sourceExecutionTested:false；另跑原完整Surface→原生Viewer→真实Loom LAN HTTPS/WSS→Source原worker联合4.032秒，实际输入及菜单/失焦释放无回归。不把API接受描述成Source执行，不把合成捕获/自有NOACTIVATE离屏窗口消息描述成真桌面WGC、普通可见双EXE或物理跨端。
本批相关Rust去重122 passed / 0 failed / 10 ignored，其中当前组合80项实跑、42项input2原生夹具按完整未变输入/独立依赖/产物SHA复用，不称新库整库重跑，未重复未变Shader/CAPTURE/PNG矩阵；protocol local_art17项由protocol-tests-2回执记录，protocol字节未变复用，不混入上述artifact统计。新普通产品工作区验证5 passed/1前台门ignored，daemon version/help通过。Otter严格all-target Clippy 138.943秒、库298.308秒、普通产品90.312秒、workspace目标102.525秒；Loom库40.347秒/普通daemon18.048秒构建通过。Loom严格Clippy仍daemon101/protocol10条原诊断，按code/message/primary-file及次数比对无新增，非全仓静态通过。

input1 protocol严格Clippy发现新large_enum_variant两条（lib/lib-test）；input2仅将Input的envelope改为Box及原store适配移动解引用，JSON/wire不变，无诊断抑制。原失败与scoped-1.patch保留。Loom相关静态/协议测试/库/普通daemon和两仓行数重验；全部Otter输入未变、独立Cargo不依赖Loom，按完整delta与SHA复用input1 native Clippy/编译产物，相关运行测试则全部本批实跑，不重复分配版本。
input2原Source联合结束断言失败并非只需刷绿：close先于LiveMediaRoleGuard Drop时，set_media_connected(false)被closed围栏拒绝，可能永久残留source_connected/viewer计数。input3仅放行已核身份的false退休，true仍活动会话限定；不在close时伪清计数。新增真实RoleGuard/Permit确定性测试，验证双viewer/Source逐项释放、伪身份不能清理及closed不能重开。联合继续原30秒总期限，等服务端role/permit真实归零，不延长期限；相关media/授权/原Source联合集中重验，保留input2失败profile和日志。
行数门必须分报：Otter全仓strict通过，本批全部变更Rust小于500有效行；Loom输入未改策略却在UTC进入11月后，既有apps/desktop/src/components/art/ArtEditDialog.tsx例外于2026-10-31到期，当前全仓strict exit2受阻。未延长例外、伪造独立批准、调时、跳过门或扩大到无关Art UI重构；旧input2在到期前通过不能充作当前全仓成功。原失败和scoped审计均保留，不是最终发布门通过。
最终职责文档、格式/严格行数/版本/UTF-8/scoped/Git diff、六套组织/Web合同45项、本地链接、冻结输入和全部产物SHA统一见main-art-live-control-1/STATUS.md、final-audit.json及closing-audit.json。普通Otter开发EXE SHA256 940FA755D4542370BD40679D599124EBD2D57C9142428CA89E869D27B0C903D0，配套Loom daemon SHA256 411F35C16BF4D008CFE3F4D2517A3FE8EA0F01A9C688D9B9AEC0EFD4FDA708CC；不是新手测整包，不覆盖固定v464包。未触真实profile/设备，不提交/推送/发布/部署。三旧副线继续冻结。
本批关闭本地Art原生Live菜单控制/减权的所述验证边界，完整Surface LAN API、观察/trigger、H264 Source、重连/epoch恢复、完整LiveRelay/Tile、旧Web实际产品互通、普通可见双产品/物理跨机、历史前台根因、资源长稳、最终整合手测包和用户全面手测仍开放。以下旧版本仅保留历史边界。

## v489：原生Viewer经真实Loom LAN控制Source及释放（完整隔离联合通过）

本批接通真实Surface公共attach/active→原Viewer目录/菜单加入→获取控制权/HTTPS control/input→Loom可靠事件→Source原Session唯一输入worker→自有离屏WndProc。联合发现LAN虽支持Live却拒绝Surface宿主入口，已由Loom lan_http_surface补齐精确POST attach、实例lifecycle及GET设备stream；强制Device/nonce，16KiB请求门，拒绝管理/任意事件/mount/资源、别名/重复参数/浏览器/GET body。原设备/审批/附件/nonce重放/生命周期鉴权不改，不伪造Binding、不引入WebView或全局输入。

新联合3.886秒通过：真正配对的Source/Viewer、同一真实Loom生产库LAN HTTPS/WSS、真实Surface归属及Mounted1/Active2/Inactive3，独立核80字节NLLV/2x2 BGRA精确[17,83,211,255]x4。Viewer两次获取控制权，发送A/B键与鼠标按下；第一轮系统菜单释放、第二轮自有WM_KILLFOCUS，Source在仍发布时实际实收KeyUp/MouseUp且原worker完成/成功cleanup，Viewer仍观看。最终actor控制序号9/input4，viewer停止与Surface inactive、Source再停止后closed/source_connected=false/媒体连接0；原本地捕获保留至shutdown，Editor戳不变。
这是两个lib-test Application与真实生产库服务的联合；捕获仍为合成WARP纹理，自有NOACTIVATE离屏窗口消息不是真桌面WGC、普通可见双EXE或物理跨机验收，不移动鼠标、不抢前台、不向用户程序注入输入。HTTP acceptedSequence不是Source执行回执，本门另以实际WndProc/worker及仍在发布的权威状态证明；Bridge发布资格与短命输入worker寿命明确分开。

相关389项Rust完整名去重终态为365 passed/0 failed/24 ignored：21项本批实跑通过（Loom18/Otter3），344项历史未变模块及固定SHA普通EXE工作区按完整delta/cfg边界/产物SHA复用，不称全部新库重跑。8项LAN边界含2项新增、7项设备附件/stream隔离/生命周期/可靠输入/nonce回归通过；两条旧Source联合0.844/1.135秒通过。11项viewer及14项Source Application的原v488回执按未变输入复用，本批未重复运行；工作区5 passed/1前台ignored也为同产物历史复用，不冒充本批新产品运行。
input3 Otter all-target严格Clippy107.744秒/测试库186.607秒通过，后续Otter全部输入不变。Loom input4库75.555秒/普通daemon40.498秒构建通过；input5仅修parent独立oracle后库27.654秒，普通daemon按精确delta/SHA复用且version/help实跑通过。Loom严格Clippy input4/5分别37.033/53.418秒，仍101条旧诊断且精确签名/次数无新增，不称全仓静态通过。两仓严格行数通过，11份Rust最大436有效行。
实现审查先修新Source联合观察：不把remote_input_idle所含Bridge退休当作worker释放，改核原线程句柄/cleanup和仍在发布。input2新profile夹具误传canonicalize的Windows verbatim路径，被普通路径规则拒绝；input3仅保持canonical作linshi范围验证、运行沿原普通绝对路径。input3真实400进一步定位并补齐上述LAN接缝。input4全链两轮控制已完成，但parent误把Inactive版本预期2；原store挂载已推进Mounted1，input5固定独立oracle为Active2/Inactive3，生产不变。原失败/夹具/回执保留，不重试刷绿或放宽期限、预算和鉴权。临时脚本引用错audit编号已补正确input3/最终审计，旧误编号数据明确标记，不用于当前结论。

最终冻结输入/职责文档、UTF-8/scoped/Git diff、版本、组织/Web合同、本地链接及产物/固定包哈希见main-live-viewer-source-joint-1/final-audit.json与STATUS.md。Otter生产未改，普通开发EXE沿v488 SHA256 80C68B14C0FC786F6EDBEF2705FACCBB8F4B4FE1B9F46894CDA21587674B9C39；Loom须使用本批配套生产输入/产物，不把旧Loom与其误配。未生成新手测整包、不覆盖v464固定包；Shader/CAPTURE/PNG冻结，不触真实profile/设备，不提交/推送/发布/部署。
旧Web/Tauri源码实际也使用vk:1..254/modifiers0，不再把DOM键码转换列为缺实现；旧Web真实产品互通仍须运行证明。本地Art控制授权、完整Surface LAN API、观察/trigger、H264 Source、重连/epoch恢复、普通可见双产品/物理跨端、完整LiveRelay/Tile、历史前台根因、资源长稳、最终整合包和用户全面手测仍开放。以下v488等仅保留历史输入边界。

## v488：Surface原生viewer控制权、输入转发与清理（定向闭环通过）

本批接通真实Surface宿主加入的viewer系统菜单“获取远程控制权／释放远程控制权”→原串行Control线程→原Device Lease/HTTPS control与input→actor控制/输入双游标→可靠状态及原媒体窗口。只有本实例显式acquire确认才启用，不隐式接管同device已有controller。acceptedSequence必须单调且不落后已消费可靠游标，但只表示服务接受事件，不是Source执行回执。当前vk:NN与Source一致，不冒充现有Web DOM键码已经互通。
UI队列最多16项、连续move合并；非法/未广告能力/溢出撤销并清队列，不能丢释放边沿后继续控制。取消→立即重新启用仍先释放旧控制，停止封住队列，原线程有界尝试释放；未知HTTP结果失败关闭且不重放。30秒控制租约每15秒续期，有控制意图时50ms事件等待/10ms空批退让，普通只读保持750ms。复用原线程、Lease、媒体change通知、四槽/预算和退出泵，无新输入线程、句柄、hook、依赖或真实配置。
窗口只用当前已实际present且客户区尺寸匹配的Viewport；黑边不变点击，已有drag/up才clamp。本地Live和远程viewer共享纯Win32解码，权限/投递/资源owner仍独立。失焦、进入菜单、尺寸/可见变化和关闭释放；远端撤销控制保留观看，输入失败关闭并向Application上报。测试只给自有离屏NOACTIVATE HWND发窗口消息，不抢用户前台、不移动鼠标、不向用户应用注入输入。本地Art proof只授观看，仍只读，不能伪造Surface附件来使用控制端点。

最终input4相关372项Rust完整名去重终态为348 passed（Otter库341/Loom2/固定SHA普通EXE工作区5）、0 failed、24 ignored，不是整库。15项新增全部通过：能力拒绝/真实grant、16项边界/move合并、停止终态/释放屏障、同device不隐式接管、真实隔离HTTPS绑定/nonce/双序号/错响应、共享mapper黑边/释放/键码和五项Application控制路径。11项viewer（原Surface/Art六观看及新五项）逐用例独立进程通过；14项Source Application通过。负向HTTP500明确核一次错误/一次发送、viewer回收及Editor文档戳不变，不扩称canonical逐字节不变。
两项真实Loom生产库Source LAN HTTPS/WSS联合分别1.043秒/1.385秒通过，作为共同HTTP/本地输入直接回归；仍是合成WARP捕获与原Source控制夹具，不是新viewer公共control/input→真实Source的全链联合、真桌面WGC、普通可见双产品或物理跨端验收。普通固定SHA产品工作区2.874秒，5 passed/1前台退出ignored，核实际启动、Checkpoint、中断重开、末Unit删除不复活和产物身份，不冒充完整菜单/网络产品验收。

input3严格all-target Clippy177.639秒/lib420.080秒、普通产品165.403秒/workspace目标143.422秒构建通过；input4严格Clippy118.009秒/lib250.219秒通过。input4实际运行13条Otter库、2条Loom和6条工作区终态，其余263条input3终态及88条上批未变pure模块终态按完整delta/cfg依赖/产物SHA复用，不把所有结果冒称新库重跑。Loom全部输入不变，复用已核SHA测试库，未重编daemon或重跑旧Clippy；其原101条诊断仍非全仓静态通过。
input1严格Clippy的测试块位置失败、input2测试误用DocumentStamp相等运算的编译失败保留，改为原matches合同，不增加生产trait。input3负向夹具对预期HTTP500仍unwrap成功而失败，input4只修错误领取，明确要求恰好一次正确错误并保留原五秒、全部清理/不重放/Editor断言；受影响11项viewer完整重验，生产产物按精确test-only差异复用，没有放宽产品错误、超时或预算。
4104项冻结输入，收尾仅三份Markdown变化；25份Rust最大472有效行。格式、严格行数、版本、UTF-8/scoped/Git diff、组织/Web合同、本地链接及新旧产物/固定包SHA由`main-live-viewer-input-1/final-audit.json`集中收口。普通开发产品SHA256为`80C68B14C0FC786F6EDBEF2705FACCBB8F4B4FE1B9F46894CDA21587674B9C39`，不是新手测整包，不覆盖v464固定包。Shader/CAPTURE/PNG冻结；不触真实profile/设备，不提交/推送/发布/部署。
本地Art控制合同、Web键码互通、新viewer真实Loom控制联合、观察/trigger、H264 Source、重连/epoch恢复、H264 LAN完整矩阵、普通可见双产品/物理跨机、完整LiveRelay/Tile、历史前台根因、资源长稳、最终整合包和用户全面手测仍开放；以下v487等只保留历史输入边界。

## v487：Live发布端窗口消息控制与释放（定向及真实Loom LAN联合通过）

本批接通显式“发布 Live 并允许远程控制”→原Application来源/工作区/Device授权/容量围栏→Source能力广告→独立Lease/Cancel可靠事件轮询→整批协议/controller预检→单在途UI桥→原Session唯一live_input_worker→实际执行/释放回执后推进游标→停止/撤权/换工作区/源关闭/退出回收。普通发布仍只读，Region不授控制；媒体和控制须same_grant，不用相同origin/device冒充凭据代际。复用原四槽、像素/编码预算、消息泵和退出设施，不新增全局hook、SendInput、焦点切换、鼠标移动或等待句柄。
桥的busy覆盖UI已取请求但未确认期间，controller切换/cancel先排空并释放旧worker；远程占用时禁本地Live输入。Bridge保存原StopSignal，不依赖UI继续消费才能取消；配置早退/panic和空闲Bridge撤销也停止原Source，不能等下一条输入才发现会话失效。当前仅窗口消息pointer move/button/double/drag/wheel/keyboard/cancel和vk:NN键码，不支持text/focus、X1/X2、非零modifiers或同次双轴wheel，尚未验收现有Web viewer DOM键码兼容。

最终input5相关365项Rust完整名去重终态为341 passed（Otter库334/Loom2/固定SHA普通EXE工作区5）、0 failed、24 ignored；不是整库。14项Source Application逐用例独立进程通过，包含新增7项真实键鼠按下/释放、应用退出、源关闭、控制者切换、空闲Bridge取消、整批拒绝、撤权与工作区换代场景及7项只读回归；原Surface/Art六项观看也逐用例独立运行通过。整批无部分输入、单在途/迟到、HTTP身份/授权、原媒体/本地输入/Win32 Target/Session与退出等待直接回归通过；ignored为各自显式运行条件，不计通过。
两项真实Loom生产库/LAN HTTPS/WSS父子联合分别1.055秒和1.404秒通过：原菜单/原Application以真正配对Device发布，原媒体worker独立核80字节NLLV及2x2 BGRA精确[17,83,211,255]x4。新增输入联合经真实HTTPS events→桥→原Session worker→自有离屏NOACTIVATE WndProc，实收A KeyDown/KeyUp、MouseDown/MouseUp；停止后Source closed、source_connected=false、media_connections=0，输入owner回收，本地采集保留至shutdown。控制侧夹具直接调用原Loom store构造membership/controller/input，不是viewer公共HTTP或原生控制UI验收；捕获是合成WARP纹理，不是真桌面WGC、普通可见双产品或物理跨端。回执仅证明结束时来源不在前台，不伪称持续焦点监测。

input5严格Otter all-target Clippy121.151秒、测试库219.446秒通过；最终生产为input4，普通产品79.928秒/workspace目标79.880秒构建通过，input4严格Clippy127.768秒/库274.883秒也保留。Loom仅修改cfg(test)联合父门，生产路由/授权/预算未变；测试库129.799秒构建通过，严格Clippy33.313秒仍为101条既有诊断且精确签名/次数无新增，不称全仓静态通过，普通daemon不重复构建。
input5的22项Otter库终态实际运行于新库；239项input4和73项input2已通过终态按完整delta/cfg依赖/各测试EXE SHA复用，不把它们全部冒称新库重跑。普通产品/workspace从input4、Loom库从input2复用，test-only-repair-5.json及各identity文件保留证明链。固定SHA普通产品工作区3.609秒，5 passed/1前台退出ignored，核实际启动、Checkpoint、中断重开、末Unit删除不复活和产品身份，不冒充前台完整菜单、网络跨端或最终包。

input1可变借用/测试HMODULE转换编译失败保留；input2退出夹具只等KeyDown却断言MouseUp、peer误将10053取消断连当panic，input3仅修cfg(test)同步/明确断连处理及单次原子回执，没有放宽timeout、协议错误或释放断言。input4审查发现空闲Bridge撤销后Source仍可能空轮询，修复生产并重建三个Otter目标，新增空闲取消回归；之后捕获关闭夹具丢弃Session.poll一次性终态却保留owner，shutdown报`Live shutdown has no completion owner`。input5只改测试pump为原Application.process_live(false)领取/退休，不改生产shutdown或错误门；受影响交互完整重验通过，原失败均保留，不升微版本或刷绿。
4094项冻结输入，收尾仅四份Markdown变化；本批26份Rust最大465有效行。格式、两仓strict行数、版本、UTF-8/scoped/Git diff、组织/Web合同、本地链接、复用产物及固定手测包SHA由`main-live-source-input-1/final-audit.json`集中收口。普通开发产品SHA256为`8E324A1A5945D49B8AFA92C91B728F5870AFB435A6B0D08EAFA2D3FDF3A7E934`，不是新手测整包、不覆盖v464固定包。三旧副线继续冻结，不触真实profile/设备，不注入用户输入，不提交/推送/发布/部署。
原生viewer控制权/输入UI及Web协议键码对齐、观察/trigger、H264 Source、重连/epoch恢复、普通可见双产品/物理跨机、H264 LAN完整矩阵、完整LiveRelay/Tile、历史前台根因、资源长稳、最终整合包及用户全面手测仍开放；以下v486等条目只保留其历史输入边界。

## v486：普通Live显式发布与停止（定向及真实Loom LAN联合通过）

本批完整闭环为Live系统菜单→Application当前来源/工作区/授权/容量围栏→原HTTPS创建→原WSS Source握手→同一捕获Consumer→后台GPU读回→有界分片raw BGRA→显式停止/撤权/换工作区/源关闭/应用退出回收。只广告raw_bgra/srgb、空输入/观察能力；不新开WGC，不自动恢复发布授权或重连。Source共享原远程/Control四槽、全进程编码预算和原Pixels/staging；网络缓冲从共享预算预留512KiB，16KiB continuation分片共用完整帧250ms期限，不扩大原64KiB写缓冲或复制完整raw Vec。Device媒体仍只用认证头/nonce，无旧deviceId query。
菜单只排单槽bool请求，UI不执行CA/DNS/网络/GPU读回，WndProc不join。Unit与稳定Arc绑定来源，旧stop不能停止新owner；停止未join仍占名额。显式stop不立即撤销原Lease，以便同身份有界close，分Confirmed/Unconfirmed/AuthorizationEnded/NotCreated，不把close失败/丢回复当关闭成功。审查修复停止后的Ready event不能继续留在退出等待集合，只保真实线程handle，避免常亮信号忙转及超过原退出等待预算；同批加入active Source直接应用退出回归。

最终input4相关279项Rust完整名去重终态为270 passed（Otter库264/Loom1/固定SHA普通EXE工作区5）、0 failed、9 ignored；不是整库。7项新Application逐用例独立进程通过：菜单精确帧/停止保采集、撤权、工作区换代、close失败不伪报、create在途stop/迟到、源关闭、active Source退出。HTTP身份/几何/能力响应校验、真实TLS大帧分片/中途取消、预算不足关闭/预撤权、原捕获/媒体/Session直接回归通过；原Surface/Art六项观看独立补验通过。另11项原退出泵/等待容量与消息回归复用同一冻结测试EXE补验，不重编、不重复已过完整名。
新增真实Loom生产库服务/LAN HTTPS/WSS父子联合1.760秒通过：原Application系统菜单使用真正配对Device创建并发送，原媒体worker接收80字节NLLV+2x2 BGRA；父门以固定字节和[17,83,211,255]x4独立核尺寸/像素及source_connected/空输入能力。原菜单停止后closed、source_connected=false、media_connections=0，子门核本地采集仍在、退出后owner归0。捕获输入是合成WARP纹理，不是实际桌面WGC、普通双EXE可见或物理跨机验收。9 ignored分别为另一真实Loom描述符、三H264、两真实WGC输入、两真实窗口/鼠标和普通产品前台退出门，不计通过。

严格Otter all-target Clippy112.507秒、lib350.821秒、普通产品127.104秒/workspace目标130.725秒、Loom lib65.469秒构建通过；Loom生产源码未改，不重复构建普通daemon。Loom严格Clippy34.016秒仍失败，101条既有诊断的精确签名/次数无新增，不称全仓静态通过。固定SHA普通产品工作区3.955秒为5 passed/1前台退出ignored，核实际启动、Checkpoint、中断重开、末Unit删除不复活及产物身份，不冒充完整发布或前台菜单验收。
input1跨模块引用/缺Write import及input2测试fixture三元组解构失败保留；input3严格Clippy通过后发现上述Ready等待问题，冻结input4重新构建并集中重验，未沿用旧输入运行成绩、提高超时或放宽资源/鉴权。4086项冻结输入，34份Rust最大460有效行；收尾只更新四份Markdown，不改变已验生产/测试或产物。格式、两仓strict行数、版本、UTF-8/scoped/Git diff、组织/Web合同及本地文档链接由main-live-source-1/final-audit.json集中收口。

普通开发产品SHA256为AB802FD23D2C48C3A29E67413739B54B5E6C93ECB18E55616A842ACA880A7381，不是新手测整包、不覆盖v464固定包。三旧副线继续冻结，不触真实profile/设备，不注入用户输入，不提交/推送/发布/部署。远程输入/观察、H264 Source、重连/epoch恢复、普通可见双产品/物理跨机、H264 LAN完整矩阵、完整LiveRelay/Tile、历史前台根因、资源长稳、最终整合包及用户全面手测仍开放；以下v485等Source未实现描述只保留当时输入边界。

## v485：Live LAN HTTPS控制与WSS原媒体接入（隔离真实联合通过）

Loom仅为既有Live目录、创建、会话、事件及POST控制开放精确LAN白名单；媒体只接受明确session/role/游标参数，拒绝query deviceId、重复/未知参数、路径别名、GET body及管理/浏览器凭据。保持Host、16KiB/48头及原请求预算；Device/nonce/成员/撤权仍由原owner核验。已验TLS stream直接移交既有媒体grant/permit/worker与视频协商循环，不建透明代理或第二媒体框架，升级后不占四个短HTTP worker。LAN Manager只持runtime Weak，停用/重配经原stop及有界IO结束媒体，不在registry锁内join；session路径只解码一次，合法编码斜杠保持同一身份。
真实联合发现Otter Device WSS仍带旧deviceId query，与严格LAN边界不匹配；现只从认证头/nonce取得Device身份，内部Prepared身份与授权检查保留。本地非Device兼容路径继续原query，不为联合放宽LAN身份。旧Application peer测试同步修正并加强32字符nonce断言。

最终input7相关421项Rust完整名去重终态为405 passed（Otter234/Loom166/固定SHA普通EXE工作区5）、0 failed、16 ignored；只覆盖本批相关回归，不是整库。新真实Loom LAN联合2.353秒通过：真正配对/challenge/session/issuer/prepare、目录→加入→bootstrap/events→原媒体Runtime/Session，独立2×2 raw BGRA经D2D/GPU读回精确[17,83,211,255]。一个source及五viewer在线时HTTPS目录/read仍完成，nonce重放409；禁用Device后Close1008/live_media_device_revoked、原生窗口与编码预算回收，LAN停用/新端口重启后连接permit/角色归0、禁用viewer仍401。联合明确physicalCrossDevice:false、ordinaryDualProduct:false，测试publisher不替代普通Windows捕获/控制。
原Surface三项与本地Art三项Application观看逐用例独立进程补验全过；普通固定SHA产品工作区3.931秒，5 passed/1前台退出ignored，核真实启动、Checkpoint、中断重开、末Unit删除不复活和产物身份。其余ignored是显式真实Loom/服务示例与H264门，不计通过；默认ignored不覆盖已经显式运行的联合passed。

Otter严格all-target Clippy150.317秒、input7测试库271.043秒通过；同生产input6普通产品90.524秒/workspace目标106.004秒构建通过。Loom input3库89.257秒/daemon49.730秒通过；后续Loom源码未变，四组相关运行与旧真实HTTPS加入联合按完整manifest/日志及产物SHA复用。Loom严格Clippy仍失败：101条既有诊断且精确签名/次数无新增，不称全仓静态通过。
input1测试方法/unit表达式与input2 decoder可见性编译失败、input3测试nonce形状、input4缺退出诊断、input5 Media(Transport(Upgrade))和input6旧Application query断言失败全部保留。input6生产修复后重建三个Otter目标并重验；input7仅旧peer测试叶变化，重建库并补验六项受影响用例，已通过且输入未变的联合/直接回归及普通产品明确复用，不刷绿或延长原期限。收尾只有四份Markdown变化，4070项冻结输入、21份Rust最大500有效行，原live_session_routes仍500→500；格式/两仓strict行数/版本/UTF-8/scoped/Git diff、组织/Web与本地链接集中证据见main-live-lan-1/final-audit.json及STATUS.md。

普通开发产品SHA256为356D0085DBFCD17009621B172AE3DB96CE84561837E20D0745D2986DD0CADD38，不是新手测整包，不覆盖v464固定包。三旧副线继续冻结，不触真实profile/设备、不注入用户输入、不提交/推送/发布/部署。普通Source发布/输入控制、普通可见双产品/物理跨机、H264 LAN完整矩阵、完整LiveRelay/Tile、历史前台根因、资源长稳及最终整合包/用户全面手测仍开放。以下v484等条目的“LAN未支持”为当时历史边界，不作为当前运输层状态。

## v484：在线本地Art宿主原生观看（隔离观看及真实HTTPS加入合同通过）

本批为在线已验签且含Preview的Prepared产生私有typed proof；只由当前ready的Art窗口/owner暴露稳定Arc，重复菜单快照不换身份。原工作区、输入依赖、图像身份、设备及新Lease在Application消费点重核；离线恢复没有viewer proof。ViewerHost显式区分Surface与LocalArt，不造假附件。新Loom Device-only `POST /v1/art/local/live-viewer/attach` 使用独立v1合同和32KiB门，复核完整绑定/issuer签名/在线期限、当前审批、immutable包/源码摘要、工具enabled及当前Device guard，最终审批与加入在原policy锁下串行；旧Surface viewers端点仍要求真实附件，admin或无凭据不能代替Device。
Surface与Art候选宿主最多八项，先选宿主再列至多512会话；两路径复用原发现、actor游标、ack响应校验、Session四槽、编码单份预算、Pixels解码预算、控制/媒体及窗口/退出泵。重复origin/session拒绝，旧stop Arc不影响新owner；Ready前宿主关闭/替换取消并丢迟到加入，Ready后观看独立于发起Art/工作区，沿原观看membership/Device Lease和显式停止/撤权回收。Preview只证明宿主并配合明确原生菜单意图，不授JS网络/控制/输入能力。UI资格使用只读Permit检查和独立时钟观察，不推进worker的共享授权高水位，避免并发取样误报时钟回退。

最终input3相关459项Rust完整名去重终态为440 passed（Otter340/Loom95/普通EXE工作区5）、0 failed、19 ignored；ignored为其他显式真实Loom/公开SDK与服务图像示例、三H264及普通产品前台退出门，不计通过。新增六项Otter测试均通过，包含授权只读时钟、稳定proof/过期菜单及三项Art观看和本次联合子门。原Surface三项回归与新增Art三项逐用例独立进程，真实菜单交付函数→队列→Application→隔离HTTPS/WSS→原生D2D/GPU精确[17,83,211,255]，显式停止、Device授权取消、Ready前宿主关闭/迟到与句柄回收均通过；不向用户当前应用注入输入，不用屏幕截图代替帧像素。
真实Loom生产库服务/LAN HTTPS与Otter客户端加入父子联合1.093秒通过，覆盖签名issuer/prepare、真正Device challenge/session、独立native attach、真实membership及撤审批后403；子门明确`mediaTested:false`。Loom本地Art HTTP组57.060秒、Device/media授权回归79.809秒通过。当前LAN未支持完整Live目录/控制路由或媒体WSS升级，该联合不冒充真实Loom目录→控制→媒体产品闭环；普通可见双产品、Source发布/控制及完整LiveRelay仍后续。
普通固定SHA新EXE的原工作区门3.442秒通过，核实际启动、Checkpoint、中断重开、末Unit删除不复活及产物身份；5 passed/1前台退出ignored。Otter严格all-target Clippy241.215秒、lib527.705秒、普通产品190.123秒/workspace目标174.071秒构建通过；Loom lib197.687秒/普通daemon93.228秒构建通过，protocol14项30.536秒通过。Loom严格Clippy41.608秒仍失败，精确对照101条既有诊断且无新增签名，不称全仓静态通过或普通daemon业务验收。

input1的Proof返回类型失败、input2的Loom include测试叶内层注释/变量遮蔽编译失败均保留。审查另修测试会话为真正配对source Device，不用admin省略Surface；没有为夹具放宽生产权限。input2→3只变两份Loom cfg(test)叶，完整manifest/cfg边界及SHA证明后复用Otter产物与未变protocol/静态成绩，不机械重编全套。运行脚本的旧测试名过滤为空、Cargo多产物误取普通EXE当workspace harness两处工具错误及原回执保留；修正过滤和精确target/kind/profile选择后只续跑缺失项，未重复通过用例，也未重编未变源码。误取EXE的Usage失败没有执行工作区测试，不能计入产品功能失败或工作区通过。
4064项冻结输入、收口前37项差异含34份Rust（10份新模块），最大439有效行；收口仅Markdown变化，没有编译/测试输入漂移。格式、两仓strict行数、版本、scoped/Git diff、UTF-8、组织/Web与文档链接及产物身份集中收口见`main-native-art-viewer-1/final-audit.json`。普通开发产品SHA256为`679B0AC16EA5DECE4DA27E18CE1F13931A01DA5876AC6062876FCC9696F424FE`，不是新的手测整包，不覆盖v464固定包。三旧副线继续冻结；历史前台根因、完整LiveRelay/Tile、普通可见双端/物理跨机、资源长稳、最终整合包与用户全面手测仍开放，不触真实profile/设备，不提交/推送/发布/部署。

## v483：有效Surface加入原生远程观看（新增闭环通过，旧前台门仍开放）

本批接通已有真实挂载且确认active的Surface→原目录菜单选择→原授权请求队列→Application重新准入→原控制/媒体Session→原生Viewer HWND与停止回收。Application核当前目录/附件/设备/新Lease、模态/退出/恢复和等待预算；不生成假附件，不把本地Art的Binding::local_art冒充Loom Surface，不修改Loom协议。先选Surface再列最多512会话，避免4×512子菜单越过公共1024项预算。
同origin/session重复观看拒绝，停止未join仍占原四槽和身份；Arc key绑定单次owner，旧停止请求不能停止后来重开的观看。Ready前发起附件失效则取消，迟到Ready不建窗；Ready后观看独立于发起面板/工作区，显式停止、清除/切换授权或原租约终止才关窗回收。编码帧预算进程共享一份MAX_FRAME_BYTES+HEADER_LEN，解码沿原Pixels预算，不新建网络/线程框架。只读consumer显式接受原Owner已校验的状态/观察/审计，不执行source输入或重复登记trigger，未知业务失败关闭。

9项新增UI/Session/Application测试全部通过，覆盖过期目录/附件/授权、菜单容量、重复与旧stop key、错误Target、已知事件策略、实际菜单交付函数→队列→HTTPS/WSS→原生窗口的帧像素、显式停止/撤权/迟到及句柄回收。Application夹具使用原合成认证Surface，服务为隔离TLS/WSS peer；原D2D绘制/GPU读回中心像素精确[17,83,211,255]，不使用屏幕截图，不冒充真实Loom或普通可见跨端产品通过。
实际去重273项Rust终态：263 passed（库258、固定SHA普通产品工作区5）、4 failed、6 ignored。4项失败中1项是旧Enter夹具未取得真实前台，另外3项前台用例在同一SERIAL锁PoisonError处退出、测试主体未验；不称4项功能失败已修或整组全绿。首次Surface组257.652秒的12项失败/原profile保留；其中8项非前台锁中毒用例以同SHA产物隔离补验通过，没有重复已完成的用例、修改产品、延长超时或放宽断言。原字符串过滤另命中3项desktop WIC解码测试，已从原日志补齐统计，不重跑刷绿。6项ignored为真实Loom目录/挂载、三项显式H264夹具及普通产品前台退出门，均不计通过。
普通产品工作区4.181秒通过，验证本批新EXE启动、Checkpoint、中断重开、末Unit删除不复活与产物身份准入；不冒充普通可见网络观看或前台菜单退出。严格all-target Clippy180.018秒/lib410.060秒/普通产品168.094秒/workspace目标140.627秒通过。input1的TranslateMessage must-use编译失败保留，input2只显式处理返回值后重新构建；冻结后只有三份仓库Markdown变化，没有沿用旧源码成绩。

4054项冻结输入、18项源码/版本差异含17份Rust（7新文件）、最大483有效行；最终增加三份模块/构建文档。官方格式、strict行数、版本、scoped/Git diff、UTF-8和组织/Web/本地链接收口见main-live-viewer-1/final-audit.json。普通开发产品SHA256为`8BCD81DE24B1CF476CD8A1C61C1E03ABCCEF3D05BC45CB9DF38846B1CF744838`，不是新的手测整包，不覆盖v464固定包。
本批只关闭有效已有Surface的只读观看闭环；新本地Art观看附件合同、Source发布/控制、完整LiveRelay、Tile、普通可见双端/真实Loom联合、历史前台根因、资源长稳、物理跨机器及最终整合包/用户全面手测继续开放。Shader/CAPTURE/PNG与Loom本批输入不动，不触真实profile/设备，不提交/推送/发布/部署。

## v482：双真实Loom与原生Application协作联合（隔离联合通过）

本批完整场景为source原菜单发送32x32蓝图/红框→receiver原菜单接纳与真实NOACTIVATE帧→source冻结纯蓝basis并切two_way→source和receiver经原750ms调度分别自动Apply→真实默认Checkpoint/后台journal ack→两端同profile保存重开→source切one_way→receiver保本地冲突且不写远端→原菜单采用远端/重开→source原停止菜单/receiver保最后内容。没有fixture直接ack或伪造服务预算快照。
双真实服务父门独立核document revision2/3/4/5、modeRevision2/5、对象x4/6/9和one_way权威不变；解码basis核全像素纯蓝、32x32/metadata/digest一致，不将活动红框重复烘焙。source重开、receiver重开及终态三份实际D2D/WIC PNG逐像素核固定2px红框/蓝底，expected由父门常量几何独立构造，不调用Otter merge或renderer。
input3完整联合17.469秒通过，source/receiver子门均自然结束；本批完整名去重29项Rust通过（Otter库10、Loom14、固定SHA普通EXE工作区5），0失败。含被动focus检测/注销、source automation4、receiver automation3与Loom edit/LAN直接回归；普通EXE工作区6.344秒为5 passed/1前台菜单退出ignored，ignored不计通过，不把本批当整库或全部产品验收。
首次input1联合4.610秒因新夹具未等起步图真实Checkpoint稳定即发送、quiet误作关联成功而失败；仅修cfg(test)同步，实际durable binding才发布created。input2联合7.672秒推进到真实receiver帧后，外部foreground从ChatGPT切Explorer，旧前后相等断言失败；HWND owner证据不支持归为Otter抢焦点。input3改新场景为本测试PID的被动EVENT_SYSTEM_FOREGROUND hook，整段监测receiver未激活并核当前前台不属于本PID，允许用户切换其他程序；Drop注销hook，正向单测只调用回调、不切用户前台。旧receiver联合门原断言未改，两次失败/隔离profile全部保留。
input3严格Otter all-target Clippy186.922秒/lib389.375秒，Loom lib83.031秒通过；Loom严格Clippy64.546秒仍失败，101条既有诊断、精确签名/次数无新增，不称全仓静态通过。input1普通产品22.204秒构建，input2/3完整delta证明仅cfg(test)变化，按固定SHA复用，不重复未变产品构建。input1/2既有构建回执保留；验证中未提高原期限/预算或为刷绿放宽断言。
4047项冻结输入、本批9份Rust最大211有效行；source/receiver/common/focus按职责分叶，既有joint只做测试登记和最窄可见性。只新增cfg(test)验证、最小接缝和内部版本，不改生产发送/保存/授权/PNG业务；收尾仅四份仓库Markdown变化，两仓格式/严格行数/差异、组织/Web合同、版本/UTF-8/本地链接及产物身份集中收口见final-audit。
证据`linshi/otter-native-parallel-20261029/main-projection-edit-application-joint-1`，普通开发产品SHA256为`B73B320B7C8A310B55AA96611879347385292A044445A3B0DF42712F9305518D`，不是新手测整包，不覆盖v464固定包。Otter至各Loom为pinned HTTPS，peer仍signed loopback HTTP；这是两真实生产库服务及原生Application测试消费者，不是两普通Loom EXE或普通Otter可见双端/全链TLS/物理跨机验收。自动服务Checkpoint预算阈值未在本联合累积实跑，不据snapshot自证。
普通可见双端、LiveRelay/Tile、资源长稳/历史前台门、物理跨机器、最终整合包及用户全面手测继续开放；三旧副线保持冻结，不动真实profile/设备，不提交/推送/发布/部署。后文v481及更早的当前真实服务联合未验描述保留其历史输入边界。

## v481：协作模式切换、自动Apply与服务Checkpoint（定向及普通产品工作区通过）

source可从原生菜单设置单向/双向修改；receiver菜单只显示已确认模式并禁用切换。Mode使用原写前请求/Job/四Slot，HTTP前不伪更新权威；Mode与服务Checkpoint的acknowledged只取原远端基线，本地未发修改及请求期间的新编辑仍保留。显式Submit可重试原Apply/Mode/Checkpoint，原opId/基线/payload不重造；新的Mode/Automatic遇未知pending拒绝，重开不自动重放。
自动Apply只挂在成功的source probe/attach/ack与receiver read之后，先回收原任务许可，再沿已有750ms调度每次最多32项；不增加timer、线程、等待名额、网络/保存框架或资源预算。source在receiptCount至少224，或对象至少240且存在墓碑时优先请求原服务Checkpoint。receiver必须two_way且无真实冲突/暂停；source两模式均可提交。最多64项工作区/Unit暂停记录，失败/取消保持，只有真实默认Checkpoint后的ack解除，避免未知结果被换opId自动重试。
明确可恢复的Mode/Checkpoint拒绝沿原持久标记→只读对账→默认Checkpoint→后台退休处理；未知HTTP/超时仍留原请求。原Editor/图源/Save/Checkpoint与授权身份不变。13项新模型/磁盘/实际Application边界包括32项与阈值、未发对象不误确认、receiver模式勾选/禁用、shared/offline成功read自动Apply及重开、one_way/对象冲突不写、Mode丢回复同profile恢复和原请求重试、自动失败/取消及64项暂停界限。
最终input2相关596项Rust完整名去重通过（库591、固定SHA普通EXE工作区5），0失败；5项显式真实Loom协作/发送/接收及前台退出门未运行，不计通过。input1相关库588通过/2失败/4 ignored，余下回归继续同冻结输入完成，没有重跑失败刷绿；发送Application285.860秒、持续接收411.265秒通过。两失败为本批夹具同步错误：同一测试线程的第一个重开成功后旧WM_QUIT打断第二个恢复，以及磁盘轮询碰到原两阶段rename可见间隙。修为独立shared/offline测试线程及既有TLS请求观察点，保留失败/profile，不改产品退出、保存合同或超时。
input2相对input1只变两份cfg(test) automation叶；严格Clippy117.235秒/lib207.547秒重新构建，两个受影响模块7项重验全过（receiver24.750秒/source1.891秒）。584项未变库结果及4项ignored按完整delta和原产物身份复用；普通产品沿input1的161.172秒构建及固定SHA复用，不重复未变构建。input1严格Clippy168.453秒/lib381.813秒成绩与原失败分别留存，不冒充当前测试叶通过。
普通EXE工作区目标101.172秒构建、运行2.781秒通过，验证实际产品启动、Checkpoint、中断重开、末Unit删除不复活与产物身份准入，不冒充前台菜单退出或普通可见网络协作。4041项冻结输入，收尾前23项差异含22份Rust最大468有效行；格式/strict行数/版本/UTF-8/scoped通过，三Markdown最终变化与组织/文档/diff收口见final-audit。
证据main-projection-edit-automation-1；普通开发产品SHA256为`8AD72EFA549627F811CA89BFBB80A3B2C56FEF80609A55478EFF079F15B1BE2F`，不是新手测整包，不覆盖v464固定包。三旧副线、Loom与真实profile/设备不动；模式/自动维护的隔离闭环关闭，但普通可见双端/当前真实Loom显式联合、LiveRelay/Tile、资源长稳/历史前台门、物理跨机器、最终包与用户全面手测仍开放。后文v480及更早的模式/自动提交未实现描述保留其历史边界。

## v480：源端初始化与冻结底图多目标接入（定向及普通产品工作区通过）

显式源端入口沿原Editor/renderer冻结basis PNG，活动矢量与普通opacity不烘焙，栅格标注/crop/flip/erase/外观保留；initial、anchor、session和PNG在任何CAS之前持久写入原projection_edit journal。只有明确404 projection_edit_not_found允许替换旧flattened PNG；普通PNG不会自行进入协作，未知或不支持响应保原图和stage。后续目标复用同一PNG/session，不重复烘焙标注，活动blur/mosaic明确拒绝。
初始化/attach继续原Job/四Slot、单在途Application与当前Lease；每个目标精确默认Checkpoint/ack后才处理下个目标。网络期间对已有对象的修改保留；one_way源端可以提交，receiver仍只读。普通sender识别合法editing后不能降为flatten更新，缺editing但probe已有合法文档时只读恢复。调度按具体projection推进，不使其他目标饥饿，缓存只保弱内容身份。
本批修复两处真实生命周期漏洞：candidate发布后旧durable sender scope暂时失配，Checkpoint轮询误中止；旧Job把Source Ready当普通metadata，析构误撤销已移交的Active租约。现过渡期仅核持有Candidate的原/新allocation、两份精确Index/Binding及原意图，真正commit后重新要求durable包含关系才ack；Source Ready显式接管原Active/Lease，普通metadata仍取消回收。重复poll不提前退休，同字节新allocation/取消/撤权均不能借旧候选追认。
最终input6相关583项Rust完整名去重通过（库578、固定SHA普通EXE工作区5），0失败；5项显式真实Loom协作/发送/接收及前台退出门未运行，不计通过。8项实际Application源端测试覆盖原菜单/隔离TLS/初始化/既有及后来目标/期间编辑/明确拒绝/丢回复后真实重开原请求重试/取消撤权/过期候选；PNG采用独立几何和双线性常量oracle，普通本地导出、发送/更新/停止/组/离线、接收、Save/Checkpoint/恢复的直接回归通过。普通EXE只验工作区启动、Checkpoint、中断重开、末Unit删除不复活与产物身份准入，不冒充可见双端。
input1至5的编译、IPv4/IPv6夹具、栅格oracle及上述真实生命周期失败均保留；只修已证机制，不提高5秒/10秒/HTTP或900秒期限、不放宽认证/预算。input4私有sender字段访问错误修为原owner只读接缝，不扩大字段可见性。生产变化后input6重建产品/库并重跑本批相关回归，不沿用旧生产成绩；冻结后仅三份Markdown变化。
input6严格all-target Clippy152.640秒、lib366.453秒、普通产品122.719秒、workspace目标120.375秒构建通过；新Application8项4.000秒，发送Application回归276.468秒、持续接收回归408.344秒，普通EXE工作区3.109秒。4036项冻结输入、收尾前53项差异含52份Rust最大467有效行；格式/严格行数/版本和scoped UTF-8检查通过，组织/文档/diff集中收口见本批final-audit。
证据main-projection-edit-source-1；普通开发产品SHA256为`A7B7301C06F11B2F29769B708B88F71CB62EB75D18E18EDE10C155200167DBEF`，不是新手测整包，不覆盖v464固定包。三旧副线、Loom与真实profile/设备不动；模式切换、自动协作提交、普通可见双端/真实Loom显式联合、LiveRelay/Tile、资源长稳/历史前台门、物理跨机器、最终包及用户全面手测仍开放。

## v479：已有协作冲突恢复（定向与普通产品工作区通过）

本批补齐原生保留本地/采用远端菜单及明确拒绝收尾：原Apply写前意图保留，只有已知冲突/只读/预算拒绝才写拒绝标记并只读对账，权威/本地冲突落入精确默认Checkpoint后退休。未知失败不被read或选择退休；重开只恢复，已拒绝请求的显式重试只read，不重复Apply。选择时重新读权威与当前Editor，采用远端也保留read期间编辑并记录真实竞态；保留本地不得绕过最新单向限制。
沿原Job/四Slot/63句柄、原Editor、Save/Checkpoint和Scope围栏；不新增线程/框架/依赖或扩大预算。最终input4相关363项Rust完整名去重通过（库358、固定SHA普通EXE工作区5），0失败、两项显式联合/前台门未运行；含3项新journal、4项新视图和7项实际Application恢复边界。其中未变journal3项按完整delta复用input2，其余360项运行当前对应产物，不把历史全库成绩累加。
独立wire/磁盘oracle及原生菜单/原Job/TLS/Editor链验证明确拒绝先保原请求、只读失败重开后不重复Apply、默认Checkpoint前不清pending、手动Save不授ack、shared/offline两种选择、最新单向模式拒绝旧双向keep、read期间编辑保留/真实冲突，以及取消/撤权/底图换代/未知pending不发布或退休。普通EXE五项只验证原工作区启动、Checkpoint、中断重开、末Unit删除不复活及产物身份准入，不冒充前台菜单退出或普通可见双端。
input1漏登记公共authorization请求的穷尽匹配、input2负向用例经外层错误呈现打开自有模态、input3测试跨模块调用私有UI叶的失败均保留。模态现场核精确PID/EXE/SHA后只结束本批隔离测试，不算该组通过、不延长900秒或产品期限；修为真实菜单存在/禁用断言，不为测试扩大私有接口。input2后仅三份cfg(test)叶与独立modeRevision oracle变化，生产/版本/配置未变，普通产品按完整manifest及SHA复用，不重编未变产品。
input4严格all-target Clippy126.360秒、lib246.531秒、workspace目标137.750秒构建通过；普通产品input2构建106.032秒，固定SHA工作区运行2.704秒。新Application七项71.515秒，直接持续接收回归339.531秒，未提高期限或缩减必要检查。4022项输入、26项差异含22份Rust最大466有效行；格式/严格行数/版本与UTF-8/scoped差异通过，组织/文档最终收口见本批final-audit。
证据main-projection-edit-resolution-1；普通开发产品SHA256为`3FC40328551660E59FD399C0152CA9756CAB9924E303BE498F67E776F4C1A64F`，不是新手测整包，不覆盖v464固定包。三副线、Loom与真实profile/设备不动。源端冻结basis/初始化/多目标、模式切换、普通可见双端/真实Loom联合、LiveRelay/Tile、资源长稳/历史前台门、物理跨机器、最终包及用户全面手测仍开放。

## v478：已有协作会话本地提交（定向与普通产品工作区通过）

本批闭合原生提交菜单→原Job/四Slot内写前Apply→当前Editor重读并保留期间修改→精确默认Checkpoint→后台确认原请求→有界续进。每批最多32项，acknowledged只含实际发送项，每次显式提交最多八批；剩余内容仍可显式继续。失败/取消/撤权/底图或会话换代不退休pending，不自动重发；重试恢复原opId/基线/payload，同profile重开只恢复，不自动重放。
HTTP成功不授落盘事实；session_projection_scope::Proof沿原保存worker校验最多64份轻量协作内容身份并绑定精确allocation，只有默认Checkpoint实际磁盘commit才能标为durable。ack沿原后台Job/Slot但不重复CA/DNS/HTTP；手动Save不清pending。Windows等待集合仍限63句柄，edit与receiver显式恢复/自动收件/持续read互斥，复用原等待名额，不增加句柄、像素、线程或队列预算。
最终input3相关469项Rust完整名去重通过（库464、固定SHA普通EXE工作区5），0失败；含4项新模型、7项实际Application提交测试。shared/offline原菜单/TLS/Job、期间编辑保留及续进、503后原请求重试、取消/撤权/换底图、ReadOnly不发写请求、未确认请求Checkpoint重开不自动重放、精确提交后pending清理均核验。手动Save成功/取消及请求阶段保存、同字节不同allocation不授身份的新保护矩阵通过。双真实Loom协作联合及普通产品前台菜单退出两项显式门未运行，不计通过，不称整库或完整双向产品验收。
input1编译与运行oracle失败、input2的真实shared保存失败均保留。input2已通过5/6 Application；剩余原因是实际手动Save替换original却未同步只读接收Index，后续Fence误判失效并丢弃Checkpoint阶段。input3只在Save真实commit成功后共同接管已核同候选Index，不从轮询重抓来源，不授手动文件ack资格；新增和原失败测试一并重验。生产输入变化后重新构建产品及库、集中运行受影响回归，不沿用input1/2旧生产成绩，也不提高超时/权限/资源上限。
input3严格all-target Clippy122.453秒、lib240.125秒、普通产品101.297秒、workspace目标131.969秒构建通过；固定SHA普通EXE工作区运行2.828秒，核启动/Checkpoint/中断重开/末Unit删除不复活及产物身份准入，不冒充前台菜单退出。接收组148项在原900秒总期限内772.609秒自然完成，没有中断、延时或分组重跑刷绿。4015项冻结输入、33项差异含31份Rust，最大481有效行；格式、严格行数、版本与UTF-8/scoped差异通过，最终组织/文档收口见本批final-audit。
证据main-projection-edit-submit-1；普通开发产品SHA256为`A36250841C7CBA2FD80B0E2DB95E95E8B2016244755586354B18DB3F2036B1C7`，不是新手测整包，不覆盖v464固定包。源端冻结basis初始化/多目标、模式和冲突菜单、普通可见双端/真实Loom联合、LiveRelay/Tile、资源长稳/历史前台门、物理跨机器、最终整包及用户全面手测仍开放。三旧副线、Loom/真实profile/设备不动；下文v477及更早的无edit写入描述为历史边界，不改写旧回执。

## v477：协作文档接收闭环（定向与普通产品工作区通过）

本批复用原accept/read、后台Job/Slot/Lease、Editor、工作区Scope与Save/Checkpoint/重开。校验PNG响应内完整Document及basis摘要/实际尺寸，metadata-only同PNG版本不丢标注；初始矢量/权威记录与原PNG分离，不烘焙。持续读取以当前Editor重读本地修改，保留ReadOnly/同对象冲突，恢复后的canonical留白/尺度沿原session仿射接缝；相同文档不制造undo/无意义保存换代。原PNG切到冻结basis时，矢量和新底图完整预检后一起交换，undo不回旧像素。
首次接入旧PNG的metadata-only、同版带snapshot和新版同digest三种形态，均核真实自然像素尺寸/原Binding证明及破坏性历史；不能以远端PNG有效代替本地源可安全绑定。已有owner继续弱像素/几何身份围栏，不逐轮hash整图。本地已有对象按未观察新增保留；不同宽高比的冻结basis沿与像素相同的居中内容框映射矢量，不把letterbox当图像尺寸。
最终input3相关450项Rust完整名去重通过（库445、普通EXE工作区5），0失败；双真实Loom协作联合与普通产品前台菜单两项显式门未运行，不计通过，不称整库或完整双向产品完成。实际隔离Application原TLS/Job/Lease接收、D2D/WIC像素、同PNG版本标注更新/无变化不造undo、本地修改/ReadOnly或待提交保留、取消/迟到/撤权及原Save/Checkpoint/同profile重开均核验；旧PNG首次接入矩阵保原图/Editor/破坏性redo，3:1新basis内容框有独立数学oracle，底图不烘焙标注。
input1两项运行失败分别为JSON的0/0.0词法oracle和多个反例共用退避导致合成peer接入期限冲突，回执/profile保留；修精确数值断言与独立owner，不加超时或缩退避。input2定向11项Editor/9项Application通过后，只读复查补齐带snapshot的共同首次源证明，冻结input3重编并集中重验，不沿用旧输入绿灯。
input3严格all-target Clippy177.812秒、lib225.422秒、普通产品116.921秒、workspace目标136.750秒构建通过；固定SHA普通EXE工作区运行3.407秒，核启动/Checkpoint/中断重开/末Unit删除不复活及产物身份门，不冒充前台菜单退出通过。4006项输入、34项差异含30份Rust，最大453有效行；官方格式、严格行数、版本与UTF-8/scoped差异通过。最终组织/文档收口见本批final-audit，冻结后无生产或测试变化。
证据main-projection-edit-receiver-1；普通开发产品SHA256为`EDD210743DBC683C39A741C53694A79A3DF96091B31BE5D60291F784F75FF5A2`，不是新手测整包，不覆盖v464固定体验包。源端初始化/多目标、模式与冲突菜单、two_way本地提交和精确网络ack仍后续；本批只读接收，不自动发edit或退休pending。三旧副线、Loom/真实profile/设备不动；LiveRelay/Tile、长稳/历史smoke/前台门、真实跨端、最终整合包及用户全面手测继续开放。后文v476及更早的editing拒绝为历史状态，不改写旧回执。

## v476：协作会话工作区事务（定向与普通产品工作区通过）

本批将Document/basis、当前可见view、角色/路由/Context及冲突接入原Application Save/Checkpoint/恢复；只绑定原Editor，不新增编辑器/栅格管线或恢复undo。记录摘要绑定同一canonical的图源/标注/编辑状态及展开尺寸；坏/过期/错scope或Unit拒绝，元数据换代也使旧保存/恢复Receipt失效。
原视口到canonical映射与basis参考尺寸分离，保存组合逆映射，保留20DIP偏移/会话比例；不支持的本地底图/几何/标注修改保存为suspended，普通保存不阻断，重开不复活旧basis。最多64份有界记录，图源仅弱身份；原写前pending不因普通保存自动退休。
最终input2相关276项Rust完整名去重通过（库271、普通EXE工作区5），含本批10项新增；双真实Loom协作联合与普通产品前台菜单两项显式门未运行，不计通过，不称整库/完整双向产品或最终包验收。真实隐藏Application手动Save/Checkpoint/同profile重开核权威与可见修改/冲突/原wire值、无合成undo及底图不烘焙；元数据移除淘汰已ready的旧保存候选，坏canonical/取消恢复不改活动窗口/Editor/原文，crop保存为suspended并保新图，原journal请求与磁盘字节不因普通保存退休。
input1 Clippy因新record只读引用未重借用而编译失败，回执保留；input2仅修该接缝及新字段的存在性解码，显式null不能默认为旧工作区缺字段。专属null/预算/身份/布局测试已随批次写好，未为刷绿放宽断言、期限或资源上限；input2冻结后无生产/测试变更。
严格all-target Clippy200.360秒、lib467.796秒、普通产品213.609秒与workspace目标184.125秒构建通过；固定SHA普通EXE工作区运行3.860秒，核启动/Checkpoint/中断重开/末Unit删除不复活和退出。4000项输入、28项差异含24份Rust，最大471有效行；官方格式、严格行数、版本及UTF-8/scoped差异通过，最终文档/组织收口见本批final-audit。
证据位于`linshi/otter-native-parallel-20261029/main-projection-collaboration-1`；普通开发产品SHA256为`CE0768ED7D1A1C0A14DE3F92F33A11FA53C94ED8E5C603E31BBF593C2612A282`，不是新手测整包。三旧副线、Loom/真实profile/设备及v464固定包不动。网络初始化/模式/冲突菜单与精确网络确认、LiveRelay/Tile、长稳/历史smoke、真实跨端及最终整包/用户手测继续开放，原editing拒绝保留。

## v475：协作标注原生适配（定向与普通产品工作区通过）

View→Editor原子候选→编辑/history→View回读复用现有session与AnnotationReplacement；未改字段回填原wire值、缺省和稳定ID/zIndex，原生f32表示不替代basis真值。普通canonical保留扩域样式、完整rotation、小数序号半径、正文与层级语义；全部wire可选字段/词法原值由View桥拥有，Application持久协作生命周期仍待接，不冒充已完整保存协作权威。
恢复/变换域与普通UI创建域分离；零宽/透明/零形状不绘制或生成不可见命中，64边大圆角、完整rotation、宽域样式/字体、空白/NUL/CRLF原文及小数序号半径沿原Editor和预算处理。Win32 EDIT无法表达NUL时明确拒绝，不截断原文；secondaryFill由View保留，不宣称新增原生显示能力。
Bridge与canonical共用一个OrderPlan；删除不改其他对象层级，合法最大zIndex在未新增时保持原值，真正重排或新增溢出才按实际Editor顺序重编号，新增不按GUID字典序决定层级。旧数组先核容量再排序，不扩大对象/正文/点/像素预算。
最终相关489项Rust完整名去重通过（库484、普通EXE工作区5），含18项新增。独立常量oracle、真实D2D/WIC/DirectWrite、仅标注失败原子性与历史、实际Application手动Save/Checkpoint/同profile重开及底图不烘焙均实际核验；默认双真实Loom协作联合与普通产品前台菜单两项显式门未运行，不计通过，不称整库或完整协作验收。
input1有测试Result别名遮蔽导致的编译失败；input2/3的五项旧域/数值oracle和两项补验oracle失败均保留。修复依据包括恢复域1_000_000上限、真实1.5会话尺度以及redo仍持有的点预算，不改变生产来迎合测试、不提高超时或资源上限。input2后只改四个测试文件（editor_resize仅cfg(test)内），input3至4仅两测试文件；完整manifest/cfg边界/SHA证明后，未变模块及普通产品按输入复用，最后16项重跑全部通过。
input4严格all-target Clippy108.016秒/lib139.000秒通过；普通产品由input2构建154.828秒，工作区目标115.906秒/运行4.125秒，实际覆盖启动、Checkpoint、中断重开、末Unit删除不复活和退出。3994项输入、42项差异含38份Rust，最大487有效行；格式/严格行数/版本及UTF-8/scoped差异通过，文档与组织收口见本批final-audit。
证据位于`linshi/otter-native-parallel-20261029/main-projection-editor-1`；普通开发产品SHA256为`0C3B8E00589B5164BAB655BC6B5E2836B0E7C7B41868A81AEC4BA7D9B12B063F`，不是新最终整合包，不覆盖v464固定体验包。原editing拒绝、三旧副线和Loom本批只读边界保持；Application网络/模式/冲突与精确协作checkpoint、LiveRelay/Tile、长稳/历史smoke、真实跨端及最终包和用户手测仍开放。

## v474：多点路径与仅标注事务（定向与普通产品工作区通过）

本批闭合多点line/arrow在既有Editor内的恢复、移动/非等比缩放、撤销/重做、canonical保存恢复，以及D2D/WIC实际路径/箭头、命中、组边界与窗口区域一致性。
仅标注替换以文档身份和提交revision冻结一次性候选，只转移矢量/serial，不接管来源raster/crop/logical_size/history；原History统一维护点预算、redo与50步淘汰。来源换代、活动草稿及预算失败不部分发布，不借全量adopt清底图。
箭头几何借用最后有效段前缀，按实时Web的2单位锚段回溯和既有头尺寸，不复制采样点；普通两点创建规则及polyline单点dab保持。单点/全部重合的line/arrow不画平头零长路径、不生成不可见命中区域。
最终相关408项Rust完整名去重通过（库403、普通EXE工作区5），含18项新增；前台菜单门1项未运行，不计通过，不称整库或完整协作产品验收。专属测试覆盖身份/过期/草稿/预算/50步历史淘汰、固定L形坐标与D2D/WIC像素、路径/箭头命中和窗口区域、非等比缩放/undo/保存往返，以及实际Application手动Save/Checkpoint/同profile重开与底图不烘焙。既有Web纯几何直接运行的独立oracle核四组锚段/trim场景。
input1的402项库测试通过，新Application测试因把图像坐标误当canonical坐标而失败，真实回执显示既有20DIP视口留白；失败profile与回执保留。input2只改该测试叶，以保存前视口/DPI/会话空间独立构造oracle，不调用writer/Mapping、不改变生产、断言或期限；重编后完整测试0.547秒通过，未改模块的402项与普通EXE按单文件delta/SHA复用，不重复刷绿。
input2严格all-target Clippy78.156秒、lib133.391秒通过；input1严格Clippy128.953秒、lib290.250秒和普通产品73.890秒。workspace目标68.156秒构建、固定SHA普通EXE工作区2.515秒通过，覆盖启动/Checkpoint/中断重开/末Unit删除与退出；不是实际可见双向菜单或物理跨端验收。
3987项冻结输入、24项差异含20份Rust，最大484有效行；格式、严格行数、版本与scoped UTF-8/空白检查通过，收尾只更新文档。证据位于`linshi/otter-native-parallel-20261029/main-editor-paths-1`；普通开发产品SHA256为`1E3517B4EEDE863E31956572CFA07296D4F80C8CD60D387F8E0BACBF9062C77B`，不是新最终包，v464固定包不覆盖。
完整文字/样式合法域（含secondaryFill、零宽/零形状等）与Application协作模式/冲突/真实checkpoint仍后续，原editing拒绝保持；本批不扩大普通session的UI范围，不冒充所有wire标注已可呈现。Loom与三旧副线不改；LiveRelay/Tile、资源长稳/历史smoke/前台门、真实跨端、最终整合包和用户全面手测继续开放。

## v473：协作视图几何与异步编辑合并（定向与普通产品工作区通过）

本批新增有界basis/local视图映射和纯合并层，复用现有v1 validator/Request/journal；不借普通session裁剪合法wire，不重编号zIndex、不将标注烘入底图。
文字/序号按basis缺省语义物化local字号，再按wire锚点分轴与标量尺寸分别映射；反向除以正向平均倍率，反复非等比resize保留不可变basis，不累乘漂移。未改数值保留原值，容差仍在当前local空间；原wire未写缺省字号时不强塞字段，合法临界16KiB对象仍可读写。
权威响应与本地可见overlay分离，同对象竞态保留冲突；每次异步保存后须重读current，undo不从旧payload复活修改。本层无网络/像素/线程/磁盘副作用，不签发Application checkpoint回执。
最终input2下54项Rust完整名去重通过（lib49、普通产品工作区5），含23项新增；两项显式双真实Loom/前台菜单门未运行，不计通过，不称整库或完整双向编辑产品完成。独立数值oracle覆盖非等比、文字/序号、全部合法类型与边界、稳定zIndex、对象竞态与undo；真实隔离journal重开保持原请求，实际pinned TLS核发送几何及响应后本地修改保留。
input1审查发现大倍率下floor(n+0.5)误合并相邻坐标及原wire补字号导致临界对象超限，input2修复三份Rust后重新冻结/构建/验证；独立Node Math.round oracle保留。oracle首稿的符号零比较和初次TLS测试模块前缀零用例错误均保留，只修辅助核验/选择器，已过22项未重跑、零用例未计通过。
input2严格all-target Clippy94.031秒、lib165.312秒、普通产品71.110秒、工作区目标127.579秒构建通过；普通EXE按固定SHA验证启动、Checkpoint、中断重开、末Unit删除不复活与退出，2.640秒。格式、严格行数、版本及scoped UTF-8/空白门通过，11份受影响Rust最大354有效行。
证据位于`linshi/otter-native-parallel-20261029/main-projection-edit-view-1`；3978项输入、15项差异，普通产品SHA256为`8E7314010CD4D6FFC1E75FA520F4B5780DDAAAD1B219FC2DD7535A5CD4EDC113`，只是开发产物，不是新最终整合包。
本批Loom及三副线冻结实现只读，v464固定包不覆盖；Editor最终适配与Application/模式/冲突菜单、真实checkpoint接线仍开放，原editing拒绝保持。

## v472：独立标注颜色透明度（定向、保存与普通产品工作区通过）

本批闭合描边alpha、填充alpha和整体opacity的导入、Editor样式/history、原Canvas/PNG与canonical保存恢复；不另造渲染器或把标注烘入底图。
只抽出已达500有效行的Stroke样式桥，原Editor/工具控件与预算不扩展；显式RGB替换仅重置对应颜色alpha。
最终input2下444项Rust按完整名去重通过（lib439、普通产品工作区5），包含13项新增边界；5项显式GPU故障/真实Loom/前台菜单门未运行，不计通过，不称整库或完整双向产品验收。
独立source-over常量核实际D2D/WIC PNG的填充/描边/组透明度及重叠次序，路径自交/箭头连接不重复加深；荧光笔既有35%wash不变。
四轮vector保存恢复像素不变、清填充/undo不复活旧alpha、真实Application手动Save/Checkpoint/同profile重开与底图不烘焙、强制CPU编辑/undo/redo/保存/恢复/再呈现均通过。
原128字节canonical纯色门保留长旧十进制及最小f32 alpha往返，不扩大Surface/网络64字节门。input1审查发现该兼容缺口后，input2重新冻结/构建/验证；初次nocapture终态解析误报保留并按串行libtest与原始计数复核，不重跑刷绿。
input2严格all-target Clippy91.469秒、lib174.265秒、普通产品61.203秒、工作区目标127.844秒构建通过；普通EXE工作区运行3.047秒，发送直接回归277.078秒。格式/严格行数/组织/版本/文档及scoped UTF-8/diff集中收口。
本批3966项输入、27项差异含23份Rust，最大472有效行；源代码冻结后仅更新三份文档。66份隔离TLS接收PNG沿既有独立像素oracle通过，并另核IHDR/metadata/digest一致，不投真实设备。
证据位于`linshi/otter-native-parallel-20261029/main-editor-paint-1`；普通产品SHA256为`480F11E4771FC4E4957F4D76694170F07B13510F7A092C293DB5EA33561BC564`，不是新最终整合包，不覆盖v464固定体验包。
协议其余几何/文字差异及Application协作/方向/冲突/checkpoint仍待后续；原PNG editing拒绝保留，三副线和固定体验包不变。本批Loom无改动，不重复其未变构建/历史验证。

## v471：原生协作编辑事务链（事务及真实双Loom联合通过）

新增独立v1文档/六操作、严格标注/版本/身份校验、profile独占写前日志、固定shared/offline HTTPS及原请求恢复。
HTTP成功不清pending；调用者必须在工作区精确checkpoint后显式确认。失败/冲突/取消/外部磁盘换代不发布结果、不替换原view或自动rebase。
颜色复用有alpha的纯色解析，并限制Web v1词法；不错误借用只支持不透明RGB的旧保存颜色解析器。
Loom现有LAN edit路由不重复新增，仅将两条edit入口收紧到原合同256KiB；鉴权、PNG和peer预算不扩展。
证据位于`linshi/otter-native-parallel-20261029/main-projection-edit-1`；3961项双仓输入，input2后只修Loom新联合测试叶的精确鉴权/固定2x2源oracle，生产与Otter输入/产物未变，按SHA复用，不重复构建。
按仓库/完整名去重227项Rust定向及直接回归通过，其中Otter213/Loom14；含26项新模型/真实文件/TLS和显式父子联合。不是整库通过，初始check/Clippy及两个父门夹具失败均保留。
完整父门4.875秒、原生child0.75秒通过，实际覆盖六操作、原请求重开重试不增revision、跨shared/offline不同对象合并、checkpoint后围栏、删除墓碑、409冲突/本地view与原请求保留及再次重开拒绝。
Otter严格all-target Clippy199.875秒、lib298.703秒、普通产品119.891秒构建通过；Loom最终lib28.204秒，严格Clippy仍101条既有诊断无新增签名，不称全仓静态通过。双仓严格行数/官方格式、组织/版本及scoped UTF-8/diff门集中记录。
显式联合夹具复用双真实Loom/临时身份/pinned HTTPS，以独立固定authority oracle核原请求重开重试、不同对象合并、checkpoint、墓碑与冲突保留。
该夹具显式ack只是事务测试，不是Application/canonical保存证明；原生Application/标注渲染/方向及冲突菜单未接，原PNG拒绝editing保持，不宣称双向产品完成。
三副线与v464固定包不变，真实profile/设备不动；peer仍signed loopback HTTP，物理跨机/全链TLS/普通业务EXE和最终整包未验。

## v470：双普通产品离线菜单与重开停止闭环

本批将v469的隔离Application场景推进到两个固定SHA的普通`otter-native.exe`，两端经原UIA菜单分别选择真实Loom HTTPS并续签，接收策略从显式reject经菜单持久切为confirm；真实接收确认、可见NOACTIVATE呈现及source端displayed均实际核验。
同envelope首帧crop为16x8，菜单reset为32x16，实时flip后当前物化Editor为320x160；三份receiver实际PNG由Loom独立固定色带与另一次Python struct/zlib解码核全像素/尺寸/metadata/digest，未从实际产物反推预期。接收端原32x16 source像素不变，接收Unit身份不变。
完整流程16.250秒通过，包含正常退出接收端、同profile新进程恢复/续签、发送端菜单停止、canonical移除、receiver保最后图并持久stopped、原read返回410，以及终态再重开和两端正常退出。
证据位于`linshi/otter-native-parallel-20261029/main-offline-product-1`。inputs-5共3945项；input5仅修测试叶的canonical source唯一矩形定位，不改共享Art定位或产品恢复。受影响Otter integration重编15.359秒、严格all-target Clippy4.265秒通过；未变Loom lib与普通产品按完整输入/SHA复用，不重复构建。
按完整名/仓库去重12项通过：默认8项、专属Job1项、普通Art serviceImage/正式像素/save/reopen/退出1项、联合父子2项；默认其余两条可见Art场景本批未跑，不计通过。Job复用input3、Art复用input2，产物与直接依赖未变；不是整库通过。
初期前台准入、默认confirm策略no-op、Job退出码假定、SDK示例环境缺配、旧自然源flip oracle和硬编码重开坐标的失败均保留；修复其已证测试机制后重验受影响完整链，不提高15秒期限、放宽认证/像素或改产品行为。父持有kill-on-close Job，spawn前握手；已验关闭只终止assigned child、不影响unassigned合成control。
普通产品SHA256为`E6EDBA3EADB0126511B898AA0EFE90C5C9F2C613455EF42544103833E61DF8C4`；本批未生成新整合包，不覆盖v464固定体验包。官方格式、严格行数、组织/版本/文档与scoped差异门另集中收口；Loom严格Clippy仍101条旧诊断无新增，不称全仓通过。
Otter至各Loom为真实HTTPS，对等互传仍signed loopback HTTP；Loom是生产库服务而非普通业务EXE。物理跨机/全链TLS、双向编辑/Tile、资源长稳/历史smoke及最终整合包和用户手测继续开放；三副线冻结，不动真实profile/设备，不提交/推送/发布/部署。

## v469：双真实Loom离线投送联合闭环（隔离Application通过）

Loom LAN HTTPS adapter现在放行既有offline device与signed peer固定POST端点，仍拒绝管理API、Bearer/cookie/origin、query与路径别名；原device/pinned proof认证不变。offline create/update及peer raster/transfer沿6MiB图像限额，其余offline控制1MiB、peer handshake/catalog 16KiB，不提高后端预算。
本批证据位于`linshi/otter-native-parallel-20261029/main-offline-loom-joint-1`；3940项双仓输入，input5只修联合测试叶，生产与Otter测试维持input2。Otter严格all-target Clippy115秒、lib273.125秒通过；Loom lib最终30.891秒、普通daemon29.719秒构建通过，普通daemon仅另验version/help，不冒充业务运行。
按完整名/仓库去重44项通过：Loom41、Otter3；包括原有单LoomHTTPS门与两个新双Loom父门，五次原生子进程执行只计三个唯一测试名。未变生产/测试模块的input2回归按明确delta复用，不重跑历史461项或把默认ignored算通过。
双端原菜单/Job/Checkpoint验证首帧保存后真实NOACTIVATE呈现及displayed、16x8 crop持续更新、同profile重开、持久停止与删后重加peer仍撤销且终态保图。源端撤销分支实际核磁盘字节、完整binding、crop与32x32绿色源像素后才登记保留。
四种signed peer请求另经真实HTTPS成功并独立验签，核篡改/重放拒绝、offer幂等与诊断不投递/不保图；五份真实接收PNG用独立struct/zlib解码及固定蓝/绿像素oracle核尺寸/metadata/digest。
Otter至各Loom为真实HTTPS，联合互传仍为协议允许的signed loopback HTTP，不能称全链TLS或物理跨机器。双daemon是生产库服务、原生双端是lib-test Application，不是普通产品EXE可见离线验收；本批未重建或重验普通Otter EXE，也未生成新整包。
初轮预算偏宽及历史日志路径错误、catalog签名oracle字段位置错误、误从source metadata-only read取PNG的失败回执全部保留。仅修夹具合同并重建受影响Loom lib，未更改生产签名/读取语义、超时、像素断言或验收门。
Loom严格Clippy仍101条既有诊断，精确code/message/primary file及次数比较无新增，不称全仓静态通过。最后仅更新文档；三副线、真实profile/设备及v464固定体验包不动，未提交/推送/发布/部署。
普通可见离线流程、真实跨机器、双向编辑/Tile、资源长稳/历史smoke及最终整合包和用户手测继续开放；本批关闭选定双仓隔离联合门，不关闭整个迁移。

## v468：OfflinePeer接收侧闭环（定向及普通产品工作区通过）

接收inbox、accept、恢复/持续read、displayed与补偿unlink沿原Job/Lease/Journal/Checkpoint/呈现owner，固定当前配对Loom的静态离线路由；source签名origin不改，不接受网络offlineTransport或shared降级。
路由/peer随原意图和兼容Web的binding保存，缺字段兼容旧在线记录，null/残缺/错peer/device失败关闭。共享/离线收件合计64项，失败保本次共享并标记离线未查，取消/撤权不能发布partial。
本批证据位于`linshi/otter-native-parallel-20261029/main-offline-receiver-1`；inputs-1.json共3072项，严格all-target Clippy148.203秒、lib构建275.563秒、普通产品构建113.109秒通过。
63组完整名去重461通过/0失败/1真实Loom联合门默认ignored，运行总计969.302秒；新增31项均通过，覆盖模型/原Job隔离TLS与原Application实际菜单、保存/真正重开、真实呈现后回执、accepted恢复、持续更新/终态/本地编辑、取消/迟到/撤权与原补偿。
发送侧直接回归保留66份PNG并独立核实际尺寸/metadata/digest，其中离线发送12份及在线持续20份按独立固色/crop oracle逐像素复核；不把这些发送侧PNG数量当作离线接收新增验收数。
普通EXE显式绑定SHA256，以89项输入及EXE字节未变的workspace_runtime验证，5通过/0失败/1前台门ignored，3.812秒；覆盖启动、Checkpoint、中断重开、末Unit删除和正常退出，不代替普通可见离线菜单或真实跨端。
官方格式、4221文件严格行数、组织合同3项、版本及tracked/untracked scoped差异门通过；44份受影响Rust最大473有效行，无新增501+例外。没有新增依赖、生产线程、无限队列、权限或提高预算/期限。
输入冻结后无生产/测试修改或失败重跑；收尾仅本节说明变化。三副线与Loom源码、对方信箱、真实profile和v464固定体验包不变，未提交/推送/发布/部署。
本批关闭接收侧实现和选定集成验证，不关闭真实双Loom/跨设备、普通可见离线产品流程、双向/Tile、资源长稳/历史smoke及最终新整包；当前EXE是内部产物而非新手测整包。

## v467：OfflinePeer发送侧路由闭环（定向与普通产品工作区通过）

固定目录的peer/remote身份随原写前日志保存；兼容binding保留独立origin与远端设备，原签名不改。
首发/read、持续CAS、取消/稳定停止沿原Job/Lease/Store/Checkpoint，只选离线静态端点，不在线降级；组准入共享八项容量。
日志/绑定/响应身份、丢回复、撤权、迟到、重开与混合组菜单/停止沿原入口验证，不另建发送或补偿框架。
原生接收侧离线收件、真实双Loom/跨设备、可见UI、长稳与最终包不在本批完成声明内；v464固定体验包不覆盖本批变化。

本批证据位于`linshi/otter-native-parallel-20261029/main-offline-sender-1`。input2共3066项冻结，严格all-target Clippy185.032秒、lib构建339.937秒、普通产品构建94.438秒通过。
九组完整名去重库回归224通过/0失败/1真实Loom联合门默认ignored；含新增13项：Application/实际隐藏菜单8、路由/日志/兼容binding3、原Job隔离TLS2。原发送/协议/目录/后台/UI/Checkpoint直接回归通过，不是整库通过。
接收端保存66份PNG，独立解码核实际尺寸、发送metadata及digest；其中离线12份与在线持续20份另按独立固色/crop oracle逐像素复核。混合组另在TLS测试内独立解码两份首发图，验证同名本地/远端分别绑定及持久停止。
普通产品按显式EXE/SHA256复用89项输入未变的workspace_runtime：5通过/0失败/1前台门ignored，3.266秒；覆盖启动、Checkpoint、中断重开、末Unit删除和正常退出，不代替可见投送或跨端。
官方格式、4215文件严格行数、组织合同、版本与tracked/untracked scoped差异门通过；26份Rust改动最大336有效行，无新增501+例外。没有新增依赖、线程、队列、像素副本、权限或预算；三副线及Loom源码不变。
初次Clippy测试初始化失败保留，仅修测试后冻结input2重验。运行编排器重复选择4项目录HTTP测试被去重保护阻止累计，原回执保留，只续跑未执行组；PNG审计器初次只识别旧crop测试名，补列新场景精确全名后重解析原产物，像素/尺寸断言不变。二者不计产品失败，也不重复测试刷绿。
最终仅本节说明变化，生产/测试字节不漂移；内部EXE独立封存，未生成新整包、未覆盖v464 ZIP或独立profile，未提交/推送/发布/部署。

## v466：设备组停止与单项失败隔离

实际组菜单固定来源/目录身份，沿原stop单槽逐项持久减权；原公平轮转继续canonical pending→unlink/持久回执→canonical移除→ack。
网络失败不阻止其他成员持久受理或收尾；只有完整确认才计为已停止。取消/来源失效放弃未受理项，不撤已持久意图，原重开恢复继续有效。
专属tests/guard及原组TLS夹具覆盖全链、失败、取消、迟到与菜单身份；无新依赖、权限、常驻线程、协议或整图副本，三副线冻结实现不变。
本批证据位于`linshi/otter-native-parallel-20261029/main-projection-group-stop-1`。input4生产修复后严格Clippy107.312秒、lib173.625秒、普通产品98.922秒通过；input5仅更新旧菜单oracle，严格Clippy133.765秒、lib244.266秒通过，生产字节/普通产品按哈希复用。
完整名去重相关库回归134通过/0失败/1真实Loom默认ignored：新组停止8项81.25秒、原sender71项194.938秒、UI18项及Checkpoint37项11.484秒。末两项变化UI用input5重验，其余生产/测试模块未变的证据不机械重跑。
普通产品以显式EXE/SHA256和未变workspace_runtime验证：5通过/0失败/1前台ignored，3.296秒；覆盖启动/移动/Checkpoint/中断重开/末Unit删除及正常退出，不冒充可见组菜单或跨设备验收。
修复真实Checkpoint编码canonical的独立allocation回执缺口：只在disk.commit及原Receipt/Workspace/Index均核验后登记，避免底层停止完成但组报告残留待确认；取消模式由当前owner重新推导。
最初私有接缝编译失败、测试peer误读link.envelope、组回执缺口及旧三项菜单oracle失败均保留。末UI测试明确核四命令顺序和无资格禁用，不放宽断言/超时/权限/预算。
官方格式、4210文件严格行数、组织合同3项、版本及tracked/untracked scoped差异通过；20份源码最大453有效行，无新增501+例外。最终仅本节文档变化，生产/测试与产物不漂移。
当前目录只提供不可用计数而不提供那些成员身份，停止组只操作明确成员的原生关联，不能猜测或宣称所有未知远端已停止。OfflinePeer、完整组开关视觉语义、双向/Tile、真实跨端/长稳、历史smoke及最终包仍开放；v464固定体验包不变。

## v465：设备组串行发送与逐目标结果

组菜单接入原Application/Job/Lease/PNG/Request/Checkpoint链，最多64目录成员，仍每Unit八项关联；整组容量在首份签名/网络前检查。
每台canonical确认后才推进，已有绑定跳过；原邀请Retry暂停剩余目标，取消/来源失效不再发送剩余项且保留已确认关联。
无新增依赖/线程/协议/权限，不修改PNG/Shader/CAPTURE冻结实现、真实profile或v464固定体验包。
本批集中验证回执位于`linshi/otter-native-parallel-20261029/main-projection-groups-1`。input1共3057项无漂移，严格all-target Clippy144.797秒、lib构建309.969秒、普通产品构建103.687秒通过。
实际菜单到隔离TLS独立解码PNG的9项新测试全部通过；全发送/域/运行时/目录/UI/Checkpoint按完整名去重195通过、1项真实Loom联合门默认ignored未运行。
另复用已核字节且native/tests输入未变的workspace_runtime，以显式EXE/SHA256验证本批普通产品启动、移动、Checkpoint、中断重开、末Unit删除和退出；5通过/1项前台门未运行，耗时2.813秒。
官方格式、4206文件严格行数、3项组织合同、版本、tracked与untracked scoped差异门均通过；19份源码改动最大448有效行，无501+新增例外。
新组队列无新线程/像素副本，原Retry阻住后续；取消不解绑先前canonical关联。实际产品集成不等于可见组菜单/真实跨端/最终新包验收；v464固定ZIP摘要未变。
组级全部停止、OfflinePeer真实发送、可见菜单操作/跨端及完整迁移退出条件继续开放，已保存项仍可走原逐目标停止。

## v464：固定原生手测候选与包路径运行验证

2026-10-31 release重建10m46s通过，3046项源码/构建输入及工具前后摘要相同；EXE15550976字节，SHA256
`9FF4CA2E054367EE727CAC38DF17C0405B69107A59A0DE80DFC599211ADC8B8C`。主仓仍dirty/untracked，未提交或正式发布，不再为本轮候选升级版本。
原生内包311文件/ZIP静态身份门通过；手测外包装提供独立profile、固定摘要启动器、两张合成图片、测试范围和真实限制，profile不位于固定程序包内。

实际包empty/pin/双图workspace-check分别12.974/13.545/2.032秒通过；普通图片完整smoke在原15秒期限失败，停于末段，未增加期限或重复刷绿。
两次普通菜单场景都在启动器前台准入前失败，Computer Use未能激活，产品业务未执行；停止前台尝试，不把旧debug可见成绩套给新release。
tests/support/product新增显式产品路径+SHA256/只读文件固定，workspace_runtime沿原离屏流程检查真实包Checkpoint、进程中断、重开、删除最后Unit与正常退出。
目标严格Clippy40.26秒、测试构建50.52秒通过；实际6项为5通过/1前台门ignored，原120秒外层和内部15秒期限未变。
辅助解析器误期待4项通过而返回1，原测试退出0/实际5通过日志保留；仅重解析原回执，不重跑测试或把解析失败写成产品失败。

证据及固定交付位于`linshi/otter-native-parallel-20261029/main-manual-candidate-464-20261031`。
产品源码/版本/依赖及三副线冻结实现未改，编译后仅增加上述测试接缝与说明；候选仍绑定其构建时完整前态清单。
在线文档按用户最新要求暂停；真实Loom/跨设备、物理输入、历史smoke根因、全部回归和长稳仍未验收，本候选不改变原迁移退出条件。

## v464：正式输出保存与普通产品持续投送补验

保存专属png_checkpoint显式把PBGRA转换到原WIC预算内的straight副本；普通png_raw/Shader straight合同不变。
对应输入6的完整alpha/channel矩阵、普通产品图标四次WIC保存恢复和302项去重直接回归通过，原失败保留。
后续普通EXE运行定位到独立正式端口未使保存dirty变化，application_save_state现复用原OutputStamp；预览/进度不推进该身份。
新增两项真实Application/Checkpoint门覆盖图像/值/清空、重开、在途替换及退出取消，沿原Receipt/worker回收，不新增像素缓存或线程。

恢复阻断另定位到无变更自动维护误报：完整UI Receipt在首次呈现后失配，旧poll_reference在识别library_changed=false前弹阻塞模态框。
现先核worker取消/策略/候选，无变更不发布/不置脏；真正写库的Receipt与退出围栏不变。两个maintenance测试及普通产品无输入探针通过。
输入16集中严格Clippy114.438秒、lib构建201.437秒、产品/integration构建49.234秒及startup补链接2.219秒通过；
10组直接依赖/公开SDK/UIA/startup共175项去重通过，官方格式、严格行数/scoped行数和diff通过；本批20项源码均不超过500有效行。
产品SHA256为`D348478B224C0B5986B8BD203074C53C0BD41C7ACCE9D62B9C81CE736BA02F5B`，仅本批内部产物，非最终手测包。

同一input16产物经Computer Use显式激活唯一自有启动器后，三条普通可见完整场景分别26.141/22.610/11.265秒通过：
Art正式红图首发、预览隔离、同ID绿图更新、持久停止、停止后蓝图保存恢复与正常退出；另完整包/服务图片/有限离线重开。
closure-audit-activated-16独立解码两份接收2×2 PNG，RGBA分别为[254,0,0,191]和[0,254,0,191]，宽高及digest一致；未放宽oracle。
原失败均保留，含未及时激活的两次启动器准入失败；没有延长原15秒、改foreground lock或把SetFocus成功当真实前台。
历史其余前台/物理输入、真实Loom可见联合/跨端、长稳及最终内部手测整包仍开放；三副线冻结，不提交/推送/发布/部署。

## v460：普通产品可见链与库回归补验

本批新增测试专属前台启动器和同份serviceImage公开示例门，不改产品权限、Loom或三副线源码。
integration输入3的fmt、严格all-target Clippy、链接及严格行数门通过，UIA释放门1项通过；
普通菜单15秒前台门和启动器15秒前置门分别失败，新serviceImage可见场景未运行，不反复刷前台。
产品与integration测试身份独立封存；cargo rustc --test自动重建普通产品，不能假设旧EXE字节未变。
完整默认lib使用已核同源EXE，900.063秒累计期限终止，569项ok/7项ignored/3999项无终态，无最终test result。
最后未返回的事件用例单独25.078秒通过。分批只补无终态项，保留原生Enter前台失败及11项串行锁中毒；
这是分进程覆盖证据，不等于单进程完整回归通过，不延长900秒或运行默认ignored物理输入门。
托盘补跑暴露旧oracle仍期待8项；现测试独立核七命令顺序及两分隔线，保留模态/待交付约束，生产菜单不变。
修正后input4严格Clippy93.907秒、lib链接158.218秒、格式及4171文件严格行数门通过；
托盘/截图入口/可用性74项通过、3项既有显式运行门ignored未运行，保留此前失败证据。
旧lib在修改前另按原SHA256封存，新lib与当前input4单独绑定；不以新定向成绩覆盖旧全量失败或2414项无终态。
真实跨端、资源长稳与最终内部手测整包仍未完成。

## v457：独立原生 Art 作者入口

Loom/sdk/otter-art提供原生类型、无DOM严格正反类型用例与直接分发JS示例；不加载旧Surface类型。
Otter显式SDK示例门要求OTTER_ART_SDK_EXAMPLE和独立冻结的OTTER_ART_SDK_SHA256，默认ignored不算运行通过。
同份JS经过原签名包/真实child/隔离TLS/隐藏Application，覆盖服务成功、503隔离、原期限完整缓存离线及正式像素/值与Save恢复。
TypeScript5.9.3无DOM严格正反例及JS语法通过，SDK门从新EXE列表核名后显式运行；其内部200/503和两次离线重开均通过。
本批共82项去重Rust运行通过：SDK示例1、Application17、Art owner21、服务28、保存15；默认ignored的SDK门已显式验证，不重复累计。
strict all-target Clippy125.625秒、lib构建243.954秒；格式、双仓严格行数、diff、版本及9项文档合同通过。
没有生产API、权限、依赖或预算变化；Rust与示例冻结后未改，不为相同产品源码重复打包。首轮类型收窄失败证据保留。
类型/隐藏Application不证明可见UI、真实Loom、资源型/敏感完整SDK、跨端/长稳或最终包通过。

## v456：远程服务暂态失败的本地隔离

实现固定暂态错误到Promise拒绝、原call一次有界取消和清理前后授权复核；不自动重试、换离线或延长预算。
同批测试覆盖真实signed child/TLS、本地交互继续、用户显式新调用、旧输出拒绝及Application正式结果/保存恢复。
本批7组119项去重定向通过：服务28、分类1、完整Application17、signed20、Art owner/窗口21、Save组装15、授权17；
授权producer夹具1项ignored未运行，不计通过。新Application用例在同HWND/Unit上验证服务503后正式2x2像素/42值、
Save/Restore、原图不变、一次cancel及仅显式第二次submit，随后shutdown；不使用真实profile或向真实设备发送。

input1严格Clippy模式写法失败已修；input2服务29项通过，Application14过/3失败，首项仅为夹具漏等输入恢复、其余锁中毒。
input3只修该测试等待原pointer_enabled，保留原10秒和所有断言；完整Application17及其余相关回归重验通过，
未变服务29项按输入哈希复用。最终严格all-target Clippy97.000秒、lib构建172.500秒、格式检查通过，失败回执保留。

同源v0.2.33.456 debug产品构建89.281秒、实际Canvas worker显式1项通过（2.422秒），合计120项去重Rust运行通过。
产品47718912字节，SHA256 `DDB9C82A7F5A545052B1A19F703B3D8ADF48450BC41234FA08FC062E7CB87442`，
封存在main-art-service-failure-1/product-3；不是资源型服务、完整SDK、普通EXE可见菜单、真实Loom联合或最终整包验收。
本批三副线源码保持冻结，无提交、推送、发布、部署、权限/预算放宽或真实配置修改。

## v455 集成补验：默认原生 Shader 下的启动与恢复

main-startup-cost-1 对当前产品做原15秒完整smoke计时，空窗口/图片/钉图分别为7.953/8.703/9.000秒，
没有未返回阶段；这不能关闭历史间歇超时根因。完整smoke包含89次宿主创建，不能把整个场景耗时当作单窗口冷启动。
组合回归发现startup仍断言没有默认Shader配置，与已集成的原生默认合同冲突；只修该测试oracle，
在三种入口都核唯一native配置且无产品Shader执行标记，原期限、完整场景及其余断言不变，不另增内部版本。
修正后startup7、session9、workspace5、settings4、profile3、settings恢复3、runner2及Shader边界5通过；
另显式运行实际产品原生Shader首次取码/正式PNG提交/缓存重渲染1通过，合计39项，旧ANGLE可信候选夹具仍未运行。
all-target严格Clippy3.062秒、相关integration构建21.579秒、fmt/4155文件行数/diff通过；未变产品及六组恢复测试按SHA复用。
生产源码和产品EXE未改，无新包、前台输入或真实设备投送；初次6过/1失败回执保留，不把本批当整体迁移或长稳验收。

## v455：完整资源Art的Application生命周期与普通产品测试目标

新增同份包/TLS夹具的隐藏Application门，实际覆盖交互队列、服务37%进度不提交正式结果、终态独立2x2图像/42值、
原截图存储不变、正式Save/Restore、同Art节点无Device Session离线重开和shutdown；不是物理点击或可见菜单验收。
input7实际运行30项通过：新完整包1、其余Application15、Art owner7、Art窗口6及UIA COM释放1；未运行全4544项lib测试。
严格all-target Clippy107.906秒、lib no-run182.578秒、普通产品测试目标链接62.640秒及官方fmt/有效行数/diff通过。
input7普通产品integration目标已重编译并运行释放门；可见门没有在相同失败条件下刷重跑，原15秒前台门未放宽。

普通EXE可见菜单门未通过：曾进入实际菜单/续签，TLS夹具继承非阻塞导致WouldBlock，已显式恢复阻塞；
其余尝试前台准入失败。首轮panic清理触发AV，UIA现先释放接口再CoUninitialize，释放门通过；
旧自有测试PID仍被系统持有且拒绝结束，不提权、不结束其他进程，只为本integration目标链接唯一文件名避锁。
全部失败及profile保留；可用原生控制面没有暴露新测试窗口，未绑定日志HWND猜测或操作用户窗口。

证据main-art-product-lifecycle-1；生产代码只增加两个cfg(test)访问器及测试注册，无产品启动后门、鉴权/预算放宽或新依赖。
最终文档输入与已编译input7的生产/测试/构建字节另作审计，不为收尾文档重复构建；v454未变合同成绩按哈希复用。
PNG/CAPTURE/Shader保持冻结。本批不是可见产品/Loom UI联合、跨端、长稳或最终手测整包验收，总目标active。

## v454：受控Art资源包、共享缓存及有效授权下完整资源离线恢复

独立package证明绑定同次完整grant及资源清单；旧grant/resume不换绑。资源下载沿原后台worker、当前设备/审批及取消链，
支持有界多文件JS、文本/JSON、二进制和PNG；与图输入及源码共用原容量，只有全部提交后才发布原期限内的完整离线凭证。
Otter673项相关范围为668 passed/5默认ignored，串行790.107秒；Loom protocol9、security6、daemon25 passed，另5项默认ignored。
三个HTTPS联合入口另显式通过：在线服务/null/撤审批；原单入口停服恢复；新资源包在线正式结果→首进程退出→Loom停服→缺块拒绝→新进程完整离线恢复。
新门用独立2x2像素oracle和正式42值，离线无Device Session；不是实际产品菜单或可见UI验收。

input5仅修Schema测试的传递引用装配，input6/7仅修显式ignored联合消费者的端口/Registry寿命；未变668项主体及Loom成绩按输入和封存EXE哈希复用。
input8只由官方rustfmt合并上述Client换行，完整文件去空白字节相等；重验格式，不重复编译相同语义测试。所有原失败回执保留。
最终Otter严格all-targets Clippy71.812秒、lib-test构建96.578秒通过；本批产品构建Otter73.500秒、Loom50.547秒通过。
封存product-8内两份debug EXE，实际Otter Canvas worker1 passed（2.046秒）。Otter EXE为47714304字节，SHA256
`3FA553B995A40F66326D9B1B893B1F36A805A737DD1AA1457121D867E8745D52`；Loom daemon为72043520字节，SHA256
`096DD7F19293B396E5C406529290C869072EC325D9EE07D4E5B332CC618E721F`。仅批次内部产物，不是最终手测整包。

Otter格式/4147文件行数门无501+，Loom相关格式/1373文件行数门含11项既有有效软例外；版本、双仓diff及9项文档合同通过。
Loom daemon严格Clippy仍exit101（相同42未改文件101诊断），protocol直接严格检查仍exit101（相同3未改文件10诊断）；security严格检查通过。
诊断文件均与前批字节相同，本批文件无新诊断，不取消门禁、不称全仓全绿。最终文档/输入及产物复核见main-art-package-resources-1。
完整SDK、可见产品联合、真实跨端、资源长稳、历史启动间歇根因及最终用户包仍开放；不提交、推送、部署或发布。

## v452：持久Art源码缓存与新请求授权闭环

profile内只保存已验签JS，复用原下载worker串行describe→prepare。Loom显式prepare.v2命中摘要才省略源码，
仍签发当前绑定的新grant；旧prepare.v1不改义，只有明确404回退。缓存32项/32MiB，单项1MiB，暂存最多额外1MiB。
不保存grant/token、不自动淘汰或删用户文件；忙/损坏/不可写只失去传输优化，不获得离线执行权。

Otter最终Clippy90.083秒、lib-test构建164.398秒通过，18组230项去重定向通过，含12项真实隐藏Application。
源码缓存跨独立进程读取、同profile重建Application的新绑定/正式结果保存恢复、旧绑定/损坏引用/403/取消拒绝均有覆盖。
另显式真实Loom HTTPS消费者1项和封存产品Canvas worker1项通过；辅助reader由父测试受控调用，不算默认ignored通过。
Loom本地Art HTTP15项、LAN边界3项、真实HTTPS联合1项通过；最终测试构建37.747秒。
Loom严格Clippy仍失败：lib46/lib test55项诊断所在42个文件与本批基线字节一致，本批文件无诊断，未放宽门禁。

初版取消TLS夹具误unwrap已被原Lease断开的连接，修为撤销后不要求成功响应。原512x512高熵Canvas快照约296ms触发250ms门，
独立debug实验定位PNG自适应滤波耗时；仅改为Fastest/Up无损滤波，原用例及独立RGBA/全alpha像素门重新通过。
首轮失败、Loom include注释E0753和错误审计解析器回执均保留；没有放宽超时、减小图片或重跑刷绿。

最终生产EXE构建67.575秒，封存在main-art-source-cache-1/product-4；SHA256为
`001474BF01901629BBEACC4F2979DB2F2B61EA1B41D3A07B4F858D647126F37C`。它是本批内部产品构建，不是最终手测整包。
证据目录main-art-source-cache-1最终记录双仓6013项输入；inputs-4覆盖实际编译，收尾文档另冻inputs-6并复核编译字节不变。
Otter格式/行数门4127文件无501+、Loom行数门1364文件含11项既有有效例外、版本与文档合同通过。
本批不代表跨重启离线授权、完整资源包/图像SDK、可见跨端、长稳或最终用户手测通过。

## v449：精确 Art 描述到本地准备请求的双仓库合同

Loom 新增设备认证的 `POST /v1/art/local/describe`，只描述当前 enabled 的精确安装包，先核显式审批，
再复用 immutable resolver、publisher trust、依赖锁、manifest 与源码验证；不静默交集权限，不签发执行许可。
返回前重新核对审批、设备代际与安装身份。Otter 严格限制响应字段与大小，复用请求合同校验；
origin/device 来自原 Client/Lease，workspace/instance/generation 来自宿主，prepare 仍独立完整复核。

本批一次冻结、各仓库一次测试编译：Otter 491.756 秒，Loom 138.604 秒。Otter 定向回归 35 passed
（grant 7、describe 3、真实 TLS/child 6、Registry 5、Tasks 6、signed Local 8）；Loom local_art 7 passed。
另显式执行真实 Loom HTTP 描述 producer 与 Otter 独立 DTO consumer，各 1 passed。原 Lease 在描述请求
进行中撤销、describe→prepare 的不同 nonce 与原 Lease 保留均有测试。未将 ignored 测试算作通过。

Otter all-targets Clippy `-D warnings` 245.718 秒通过；双仓库相关格式、行数、diff 门通过，
Otter 组织合同 3 项、版本门及 Loom 行数检查器 15 项通过。Otter 行数扫描 3990 文件，无 501+；
Loom 扫描 1347 文件，11 个既有 501–700 文件，无 701+。16 个本批修改文本验证 UTF-8 无 BOM。
Loom `--no-deps` daemon 严格 Clippy 失败（141.989 秒，lib 46 / lib test 55 项诊断），位置均在本批
描述合同之外；没有增加 allow、取消门禁或重跑刷绿，旧全依赖 Clippy 债务也未关闭。

源码冻结 SHA-256：Otter `a9f94bbf947cefdd322e0973ddcd0a0b6198207c0b50775e0985e09fbc7d6f58`；
Loom `0c21b5d5f67d21b03b8a15bcd9bc8a3443618f274c6ede94cf83a06a22505c08`。
Otter 测试程序 SHA-256：`347d8984d80aef0dfc1ca0735332460dd148e9e8954ad974ec5a3b95ca3f9b3b`。
运行后复核冻结输入未变，无本批测试进程残留；结果文档在验证后补记，不属于编译输入变更。
证据：`linshi/otter-native-v449-art-describe`。

本批不是 Application 节点调度、UI 挂载或完整产品验收；未重跑未改动的 startup。resolver 中途审批/
设备/安装变化及 describe↔prepare 双向竞争尚未做确定性故障注入，不能宣称竞态实测完成。
后续集中接入节点启动、选择新鲜度、workspace wire 身份、删除/最后宿主替换/恢复撤销和本地挂载。

## v448：有界本地 Art 后台下载、完成交付与回收

desktop/art_local_tasks 复用原 Registry 和 CompletionSignal，同时最多两个 worker；完成未领取、取消清理中的任务
仍占槽。配置/DNS/TLS/验签在后台执行，配置失败、下载失败和线程 panic 均带原 token 回来；旧失败不能影响新实例。
节点保留集同步撤销被删除节点的原 Lease 并释放 registry 代际预算；shutdown/Drop 先撤销全部链再逐个 join。
通知必须短且不阻塞；ready 先于 notify 发布，领取时可能等待短通知返回，不宣称严格无等待。

最终 input-2 编译 332.951 秒通过，相关原生回归共 33 项通过：Registry 5、Tasks 6、真实 TLS/child 5、graph_requests 7、
CompletionSignal 2、signed Local 8。all-targets Clippy -D warnings 208.878 秒、fmt 20.508 秒通过；行数门 3988 文件无 501+，
组织合同 3 项、版本与 diff 门通过。input-1 编译也通过（349.421 秒），但复核发现退出测试的 worker 断言可能被 join
忽略，因此补主线程完成计数断言后重新冻结/编译；最终测试只使用 input-2，不沿用旧输入成绩。

源码 SHA-256：`f131e30d8ace003d9534e50b64054ae899557d9fc3122895ef7355f37cf98c13`。
测试程序 SHA-256：`9731b101c7c0ffd0d0ad48a1fdbce5eef4150fc2dd8f32fc2d4d92414a24ea92`。
证据：`linshi/otter-native-v448-art-tasks`。这不是产品打包或 Application 节点/UI 挂载验收；本批未重跑 startup。

生产接入仍需经过验证的 local package descriptor、宿主 workspace wire 身份、选择/节点新鲜度和窗口生命周期接线。
Loom 已有 resolver、审批和 manifest 来源，应在现有 local_art 子系统复用，不能从 prepare 响应反推 Expected，
也不把旧 capabilities 的任意 metadata 当成已通过 local 审批的描述。线程创建失败分支尚无故障注入测试；旧成功
迟到与旧失败共用 token 门，但未单独构造两个真实 TLS 成功竞争的回归。历史 startup 超时根因和 Loom Clippy 债务仍开放。

## v447：本地 Art 请求代际与一次性交付

art_local_requests 复用 graph_requests，不新增第二套代际算法。Registry 为工作区保存原 Lease，begin 生成新的宿主
requestId，成功建立新代际后才撤销旧链；Pending 在后台通过 Authority 下载，不借用 UI owner。
成功和失败都以带 token 的 Completion 返回，旧失败不能取消新请求。接纳一次后仍保留 Lease，取消/clear/Drop
可撤销已交付结果与 Local。真实 TLS 测试已从 Registry→Authority→prepare→验签→child 运行。

冻结一次 input-1，编译 407.921 秒通过；新 registry 4 项、真实 TLS/child 5 项、原 graph_requests 7 项共 16 项通过。
all-targets Clippy -D warnings 179.473 秒、fmt 31.152 秒通过；行数门 3986 文件无 501+，组织合同 3 项、版本、diff 通过。
修改 Rust 文本有效 UTF-8 无 BOM，运行后源码未变，无本批测试或实验进程残留。
源码 SHA-256：`0367b9f392bf1c91ec77a86d529792b776978347b86f0f6fff4c4143263df674`。
测试程序 SHA-256：`0b747f0ef8522c04902c4b86626bf93777e8f7564af467ae87840813246b14ec`。
回执目录：`linshi/otter-native-v447-art-requests`。

另做隔离 MessageBox 布局实验：当前 144 DPI、32 项变更、5654 UTF-16 字符；Computer Use 本机管道缺失，按工具
恢复流程重试仍失败，未取得截图或进行确认操作。只停止已核对路径的自建实验进程；视觉/可读性仍未通过。
实验日志在 `linshi/otter-issuer-layout-probe`，不是产品验收证据。
Registry 的 Application 图节点启动、节点删除保留集、工作区恢复调度及 UI 挂载仍待接入，旧 startup/Clippy 失败未关闭。

## v446：Application 信任生命周期与显式导入确认

Application 实际持有 art_issuers，默认无信任；启动配置安装验证 issuer 文档并更新 owner，退出析构撤销原租约。
设置导入在信任有变化时单独确认 origin/keyId/公钥 SHA-256，MessageBox 默认拒绝；普通命名导入不能静默授予执行权。
复用文件模态 guard，确认前后核对窗口 owner、来源文档和设置；拒绝或坏信任文档不部分安装命名/回收设置。
接受只更新本次运行，持久化继续走原设置保存链；未增加网络自动信任或后台密钥轮询。

冻结一次 input-1，编译 338.774 秒通过。39 项运行测试全部通过：naming_import 全组 30 项（包含新增 Application
启动、拒绝、接受、移除、退出及确认文案 5 项），底层信任 4 项，真实 TLS/child 链路 5 项。
all-targets Clippy -D warnings 164.071 秒、fmt 11.912 秒通过；行数门 3984 文件无 501+，组织合同 3 项、版本、diff 通过。
修改 Rust 文本有效 UTF-8 无 BOM，运行后源码未变，无本批测试进程残留。

源码 SHA-256：`4d958e10b376a70417e416a38d403450da31597083d453c070c8a0e736cb3cc2`。
测试程序 SHA-256：`3e2a58ad878b9018f361c428fd536fe64099a2acffd7a8d9c5ccf7de9610352f`。
回执目录：`linshi/otter-native-v446-application-issuer`。
Application 测试使用真实隐藏 HWND/后台读取，但确认结果注入；实际 MessageBox 的视觉、交互和长内容可读性仍未验收。
issuer 获取/配对流程、Art 请求 registry、UI 挂载、完整 SDK/服务/结果与缓存仍未完成；旧 startup/Clippy 失败继续开放。

## v445：显式 issuer 配置持久化与原请求链撤信任

新增 art_issuer_trust，按 canonical HTTPS origin 精确绑定 keyId/publicKey；无配置默认无信任。
复用 app_settings_document 与 settings_update 的保字段、完整字节基线和恢复事务，不新建身份文件。
Authority 签出后台 Authorization 前登记原 Device Lease；轮换/移除/Drop 撤销对应租约，不影响未改变 origin。
Authorization 不持 owner 借用跨网络，原低层 Prepared::fetch 收为 crate 内部；真实 TLS 测试已从 Authority 入口进入。

input-1 首次编译失败于测试夹具调用未启用 feature 的 Uuid::new_v4；改用已有 Win32 CoCreateGuid，不添加依赖。
修正后重新冻结 input-2，编译 474.508 秒通过；原失败证据保留，不增加第二个内部版本。
38 项运行测试通过：信任与真实配置落盘/重读/冲突 4 项，TLS 下载/child/撤信任中断 5 项，设置文档 6 项，
原生设置覆盖/自动保存 15 项，signed child 8 项。没有放宽超时或取消检查。
all-targets Clippy -D warnings 213.734 秒、fmt 17.223 秒通过；行数门 3982 文件无 501+，组织合同 3 项、版本及 diff 门通过。
修改 Rust 文本有效 UTF-8 无 BOM，运行后源码未变，未发现本批测试进程残留。

源码 SHA-256：`726c9375139ffd02eea825c083e83617f56a0b4477bfd7bdd53af986fb0d9339`。
测试程序 SHA-256：`03ff5d7d3a6bc32bef0d1a60aabba65718c9933510e2259ab1acf9eafbfb43d3`。
证据目录：`linshi/otter-native-v445-issuer-trust`。未变的 Loom 验证未重复执行。
仍缺确认/配对 UI、应用启动及设置提交后的 Authority 生命周期装配、workspace/request registry、完整 SDK/服务/结果和缓存；
当前不监视其他进程设置变更，不宣称跨进程即时撤信任，也不自动信任网络 issuer 响应。旧 startup/Clippy 未关闭。

## v444：客户端授权下载到真实 JS child 的后台闭环

art_prepare_http 复用固定 origin Client、Control 的 Device/nonce 与原 Lease 取消链。请求身份由宿主传入并保存，
响应严格限长、拒未知字段；独立可信 Pin 验签和源码完整性检查通过后才产生消费式 Prepared。
挂载继续使用原 Lease，保留首次准入 Permit 的单调时间及不可逆状态；不从响应建立 issuer trust，不写授权缓存。
signed Local 的新内部入口仍复核字节，并保留原 Permit，不通过延迟挂载重新开始许可期限。

冻结一次 input-1，编译 334.231 秒通过。真实回环 TLS 测试 5 项通过，覆盖 Device/nonce、固定路由、源码和 pin
篡改拒绝、未知响应字段、403、下载前错误身份/撤销、下载中撤权中断、准备后撤权/取消，以及真实 child 挂载与原租约终止。
signed child 8 项、Permit 7 项回归通过；显式跨仓 fixture 项 ignored，本批未重复执行未变 Loom 验证。
all-targets Clippy -D warnings 175.249 秒、fmt 12.876 秒通过；行数门 3980 文件无 501+，组织合同 3 项、版本门、diff 通过。
修改 Rust 文本有效 UTF-8 无 BOM，未发现本批测试 child 残留。

源码 SHA-256：`ae7dcadc022ea893941c867a6994fbc66339e0564d70b1d338f416f87d419988`。
封存测试程序 SHA-256：`d2d6f63e4adb25197b25e6df27546a94813b8b8264b3929f9d28e26623fd2f0f`。
构建后及运行后输入一致，回执位于 `linshi/otter-native-v444-art-download`。
本批是后台 API 链路，不是生产 Loom HTTP 到 Otter UI 的联合验收；issuer 配对/pin 持久化、workspace 请求 registry、
UI owner 调度、完整 SDK/服务/正式结果与离线缓存仍待完成。v440 startup 两项失败和 Loom strict Clippy 旧债保持开放。

## v443：显式审批后的生产 Art 源码准入路由

本批将 Loom 认证设备、独立执行审批、精确 immutable package/source 解析及签名响应合并验证。
Otter 与 Loom 统一真实 publisher/art 身份，拒绝路径别名；不增加客户端默认 JS 入口。
Loom 3 项真实 HTTP 测试、7 项协议/签名测试通过；Otter Permit 7 项、真实 signed child 8 项通过。
新生成的 Loom 签名许可由 Otter 消费通过（producer/consumer 各 1 项），不是 HTTP 响应直接跨客户端挂载测试。

Otter 编译 336.158 秒，all-targets Clippy -D warnings 178.505 秒、fmt 13.565 秒通过。
Loom 首轮编译失败于私有模块引用和 ZIP name 返回类型；一起修正后编译 177.461 秒通过，保留失败日志。
Loom strict Clippy 在未修改的 protocol live/control.rs、live/h264.rs、surface/manifest.rs 共 5 个旧 lint 处失败，
未到达 daemon 的完整 Clippy 检查，不使用 allow 或缩减检查冒充通过。Loom 新模块 scoped rustfmt 通过。
双方行数门、Otter 组织合同 3 项和版本门、双方 git diff --check 通过；Loom strict 行数门含 11 个既有有效例外。

Otter 源码 SHA-256：`7710e09110472c6c636e9f1660b495e9998df0852c09e023378a68147b9bf5d9`。
测试程序 SHA-256：`2b63868febbb6134be7f28ac0bcf4e6bc4695f4a4a5890cd1c4ee1f4e94208c6`。
构建后及测试后核验源码和封存程序一致。Loom 的 936 个源码/清单输入在第二轮编译期间记录，收尾一致；
该清单不冒称构建前冻结。完整回执在 `linshi/otter-native-v443-art-prepare`，本批只分配一次内部版本。

未部署路由，未完成 issuer 配对/pin 持久化、客户端自动下载缓存、真实 workspace registry/UI 挂载、
完整 SDK/服务与正式结果。publisher trust/依赖锁解析检查不等于与撤信任原子互斥，离线撤权仍受已发许可期限约束。
未重复启动产品或打包；v440 两项 15 秒 startup 失败继续开放。不提交、推送或发布。

## v442：签名许可保护真实 JS 本地预览（接缝验证通过，生产入口未接）

复用现有 QuickJS Session 与有界 child，不增加第二套引擎、线程池或默认挂载入口。新 Local 在分配 child 前
验证签名、精确 Binding、源码和设备 Lease；操作前、管道取消轮询中、成功交付前复核，失效不可逆并回收自有进程。
当前只接受 local_state+preview；旧 emit/focus 和 loom_service 明确拒绝，不把旧 Surface 事件当作新服务或正式结果。
宿主生命周期、闲置唤醒、真实在线状态仍由后续 owner 接线；这不是完整 SDK 或跨重启离线支持。

同批仅分配一次 v0.2.33.442。input-1 编译后纠正测试中不受支持的 button.focus，未冒充最终测试成绩。
input-2 的 signed 组 1 passed / 7 failed：首个真实失败是夹具误用 DOM id 调用要求 Scene 原生 id 的接口，
其后 6 项被 SERIAL 锁污染；保留全部失败。单独消费原封存程序诊断 3 项被遮蔽测试通过，不替代最终输入验证。
所有点击夹具改用实际返回 Scene 的节点 ID，保持产品行为和断言；重新冻结 input-3 后集中验证。

最终源码 SHA-256：`abeb9d6fa7ec787144ea419ad8c15a4a9498db9dfaf9b55c0600ada5ac8aeb73`。
测试程序 SHA-256：`2c10a334c3f712fb9674906eb87bbe406e79f487eae0e15299f8c8b0ce422c84`。
8 项新真实 child 测试、6 项许可合同、9 项既有 Session 回归通过；许可组显式跨仓库项 ignored，本批未重复运行。
覆盖本地计数状态/suspend/resume、源码篡改、操作前/中/后撤销、换请求、离线拒绝、禁止 emit/focus、Drop/stop 回收。
最终编译 257.664 秒，all-targets Clippy -D warnings 208.928 秒，fmt 12.056 秒；3978 文件行数门无 501+，
组织合同 3 项、版本门通过。两新增文件各自单一责任且低于 500 行，原 Session 仅增加模块注册。
本批没有 Loom 源码变更、不重跑同输入 Loom 历史门；未打包产品或重跑启动集成。v440 startup 失败和完整迁移缺口仍开放。
证据：`linshi/otter-native-v442-signed-child/STATUS.md`。

## v441：Art 签名许可与源码准入合同（非生产 Art 执行验收）

本批合并独立协议、Loom 签发/验签、Otter 精确请求绑定/权限/源码校验、会话内终止及对应负例。
签名使用独立可信 issuer pin、Ed25519 与域分隔原始载荷；device token、resource lease 和包 publisher
签名都不能替代执行许可。墙钟和单调时间共同限制有效期/离线期限；撤权、换绑定、回退时钟和 stop 不可逆。
未接生产 HTTP 路由、issuer 配对/轮换、下载缓存、全局请求防重放、QuickJS 挂载或异步服务/正式结果链。
旧 Surface v1 广告/执行归属不变。本批只分配一次内部版本，不重复产品打包或此前同输入 startup 场景。

最终 input-2：`183ad0eaa4e62aa13851798d9429b7ab0ed9f0d1c283e4367b9f58c8c7cf3813`；编译 252.199 秒。
测试程序 SHA-256：`68a84abc26d378a1806749ad6f50262869aa6ec12aa640f04948c517c088f001`。
Otter 普通许可测试 6 passed（跨仓库项默认 ignored，随后显式运行 1 passed）；能力广告/资源快照回归 4 passed。
Loom 新合同 7 passed / 1 ignored，既有合同回归 100 passed，显式签名 producer 1 passed；Otter consumer
消费该真实文件，不以两边各自生成相似 fixture 冒充互操作。测试密钥仅为固定测试 seed，不用于生产信任。

Otter 行数门 3976 文件、无 501+，组织合同 3 项、版本门通过；Loom 行数工具测试 15 项通过，
strict 报告 1340 文件、violations 为空。Loom 报告与包装回执重名导致 EEXIST，保留原报告和错误，
只审计已有报告，不伪造成功 gate 回执、不重复执行同输入行数扫描。
Loom 严格 Clippy 失败于未改动的 live/control.rs、live/h264.rs、surface/manifest.rs、
loom_security 的 filesystem.rs/network.rs；全仓 fmt 失败于两个未改动的 loom_tool_registry 测试文件。
受影响 loom_protocol/loom_plugin_security 的 scoped fmt 通过；不加 allow 或扩展无关重构来刷绿。
完整静态检查结果及最终审计见本批 STATUS.md；不把 scoped 格式通过当作全仓通过。
v440 startup 两项 15 秒超时继续开放，完整迁移仍未完成。证据：`linshi/otter-native-v441-art-grant/`。

## v440：普通工作区显式 D3D11 兼容闭环（功能回归通过，启动集成 2 项超时）

普通窗口和钉图的兼容入口统一使用既有 D3D11 owner，移除隐式 HWND target；保持 D3D12 优先与惰性 CPU 兜底。
两类窗口均消费类型化创建失败准入；未知关联不直接回退。运行中 D3D11 失败必须实际退休，不能凭 HRESULT
把普通窗口视为无合成图并立即切换 CPU。复用原设备弱共享、退休预算和窗口消息调度，不增加第二套常驻渲染链。
本批同时更新菜单往返、普通窗口真实 EndDraw 失败及退休、创建失败分类与动态背景可见像素回归。
仅分配一次 v0.2.33.440；首轮编译后审查发现可见门仍用钉图式 CPU 提交，补普通重定向 HWND 的
D3D11→BitBlt CPU→D3D11、144 DPI 和前台不变门，同批不增加版本；最终输入不沿用旧回执。
最终 input-2：`9e8bf2ad0ba27f258ba054adc434ddd6cf6d402846a5850374baff51d9e8370a`。
255 项普通回归和 6 项显式可见门通过；剩余 7 项 ignored 未运行。编译 226.145 秒、严格 all-targets Clippy
144.404 秒、集成目标集中构建 124.846 秒；fmt、3973 文件行数门（无 501+）、组织合同 3 项和版本门通过。
产品直接复用集成构建输出，workspace-check 2.003 秒通过；产品 SHA-256：
`9c57128e8d7e0782514ccff71ae104225dbb1becff5a4ba09270beee699931fc`。
build_resources 2、exe_resources 1、session_startup 9 通过；startup 5 passed / 2 failed：
transparent_pin_creates_compositor_and_rebuilds_device_resources 与 wic_image_upload_survives_two_paints
分别报 `Native startup child timed out; elapsed=15.0814109s`、`elapsed=15.0592424s`。
保持原 15 秒期限并保留全部失败输出，不重跑刷绿。日志在退出前仍运行完整应用 smoke；不能只凭尾部截断
把根因归于最后一个日志项，也不能因历史存在间歇超时便排除本次回归。产品集成未全通过，完整迁移仍未完成。
证据：`linshi/otter-native-v440-workspace-d3d11/`。

### v440 后续单次启动诊断（不替代失败验收）

复用同字节 product-final，启用既有 OTTER_NATIVE_SMOKE_TIMING，对图片+pin 的完整 smoke 做一次隔离诊断。
`timing-pin/receipt.json` 明确 diagnosticOnly=true、acceptancePassed=false；诊断用外部上限收集完整调用链，
未改 startup 的 15 秒期限。样本耗时 14.976 秒，不是修复、稳定性或启动验收通过证明。
2097 对同线程阶段显示累计成本：34 次 tool-panel-controls 2.340 秒、1131 次 worker-pause 2.208 秒、
88 次 tool-panel-host 0.697 秒、34 次 tool-panel-project 0.603 秒。嵌套区间按原归因脚本减去直接子区间，
不把父子耗时重复相加；这不是 CPU profiler，也不能据此排除未埋点成本或证明历史超时根因。
只读核验确认普通 staged workspace 的提前控件准备是生产事务合同，READY 已防同面板重复创建，字体也已按面板/DPI复用。
当前没有足够依据删控件、延后必要准备或改变轮询来换取通过；4 个动态标签的初始重复投影虽可定位，
尚无秒级收益证据，不为此另开微小优化迭代。原 2 项失败及 scopedChecksPassed=false 保持不变。

## v439：手动 GPU 重选与 CPU 回升（相关闭环验证通过）

同批合并自动/兼容/CPU 选择目的地、菜单、退休完成发布、CPU→GPU 样式切换、失败重试凭证和回归。
先进行同 HWND 最小实验：D3D12/D3D11 均在 CPU 呈现后释放旧 owner、只移除 LAYERED 并重建真实 GPU。
早期两个桌面像素门通过，但返回 GPU 的红色与旧 CPU 红色区域相同，不足以证明新帧可见；该证据已降级而非算作完成。
input-1 的 254 项普通回归通过，扩展双向透明度门两项失败：实际全红，预期半透明红/蓝。
diagnostic-input 改用旧 CPU 从未绘过的青色、匹配实际 32×32 HWND，并增加全新 HWND 对照：
两后端全新 HWND 的 0.5 透明度均正确；原 HWND 的新青色不透明帧可见，但半透明帧均为 `[0,191,127]`，
而不是 `[0,128,255]`。问题已收窄至经历软件分层呈现的原 HWND/底层内容，不能宣称 GPU 回升的透明度验收通过。
reset-input 四项独立诊断：显式 RedrawWindow 两后端均仍失败；在样式提交后隐藏、GPU 首帧后恢复可见性，两后端均通过。
以上属于历史诊断，不冒充产品修复。最终已将可见性重建接入绘制 owner：SetWindowPos 使用 NOACTIVATE，
只对本次移除 LAYERED 的原可见窗口临时隐藏；失败恢复、原隐藏/最小化、分组/托盘意图和宿主身份共同约束恢复。
SHOW 自身重入产生隐藏意图时只纠正一次，销毁后不补写 HWND；不跨 Win32 调用持有 Canvas 借用。
最终像素门使用 CPU 从未绘制的青色，并将背景从蓝色动态改为黄色，验证两后端不透明和半透明的新 GPU 帧。
所有原失败、诊断输入和 EXE 保留，不改像素断言或延长等待。
普通兼容窗口仍为原 HWND target，兼容钉图为 D3D11，不把它描述为所有窗口的强制 D3D11 选择。
创建前失败凭证来自类型化 Failure，附着失败保持 sticky gate；相关注入不是模拟真实驱动移除。
证据目录：`linshi/otter-native-v439-gpu-reselect/`。本批只分配一次内部版本；最终冻结 input-2：
`7925c0940ee0da02ffd7b965585e2de6b2349f9a5651721b883fa86a5e7be221`。
run-2 普通回归 255 passed / 12 ignored，另显式执行 5 项可见门全部通过，剩余 7 项未执行。
显式门覆盖双后端往返像素、焦点/前台/失败恢复、最小化，以及生产 WM_PAINT 的 HIDE/SHOW 重入销毁或分组隐藏。
编译 261.451 秒、严格 all-targets Clippy 135.051 秒、产品/集成集中构建 90.257 秒；fmt、3972 文件行数门
（无 501+）、组织合同 3 项通过；版本检查复用输入一致的已有回执。
产品直接复用集成构建输出，workspace-check 1.390 秒通过；产品 SHA-256：
`6687319cbc3bd58bf8350a1afee8524069e969d849330ed054130873f068eeab`。
四组集成 build_resources 2、exe_resources 1、startup 7、session_startup 9 全部通过；后两组分别耗时
34.268 秒、9.213 秒。单 child 的原 15 秒期限未改变，本次通过不代表历史启动间歇超时根因已解决。
本批实现闭环不等于全原生迁移、全部测试或正式发布验收；未知关联终态恢复及 Art/Shader/ANGLE 等退出条件仍开放。

## v438：手动 CPU 选择/重试与编辑保存闭环（相关验证通过，历史启动超时仍开放）

新增当前窗口菜单“使用 CPU 渲染”“重试 CPU 绘制”。活动 D3D12/D3D11 先进入原退休事务，Complete 后
才接续 CPU；已知无合成图的 CPU/传统 HWND 可重建。创建期关联拒绝、Pending、Failed GPU owner、
未知缺失 owner 都不能通过手动操作解锁；已明确选择 CPU 且创建前失败的无 target 状态允许显式重试。
入口再次校验宿主、活动绘制和 Canvas 借用，分组锁定不阻止不改文档的渲染维护；只在成功帧后清原错误。
选择不写全局偏好，不改变新窗口默认 D3D12 优先，也未提供 CPU→GPU 回升或未确认 GPU 关联的宿主替换。

同一批次合并菜单、恢复状态、错误/清理和测试。CPU 业务测试经过真实编辑/撤销/重做、保存 worker、
GUID 文件落盘、Input::File 恢复及 CPU 再呈现，核对映射后的笔画位置/线宽/样式和文档绑定原图的全部像素。
首个行数门失败（context_menu 501 行）保留；抽出既有 context_menu_shapes 模态输入职责，无例外豁免。
两次 CPU 测试失败也保留：原测试误将会话逻辑坐标当作原 Editor 坐标、误将窗口投影底图当作原始资源。
按 session_vector_write_mapping/session_states 的真实合同修正断言，不删除内容检查或修改生产持久化逻辑。

收尾审查发现形状输入提取后，Guard 拒绝只退出局部函数，仍会进入菜单末尾刷新。已恢复原调用者早退出语义，
并增加真实 HWND 回归，验证两个自定义形状命令在模态占用时不弹窗、不改文档、不产生刷新失效区。
同批修错不另增版本；旧 input-4 的回执保留，不冒充最终源码验证。

证据：`linshi/otter-native-v438-manual-cpu/`；本批只分配一次内部版本。最终冻结 input-5：
`2ae75b30e0445e02de9548f0ad517ee87a166f7ddc40ab27592622f7b1a7e4c3`。
run-4 同源 11 组定向回归 249 passed / 9 ignored，另显式可见 GPU→CPU 门 2 项通过，剩余 7 项未执行。
编译 211.077 秒、严格 all-targets Clippy 147.673 秒、产品/集成目标集中构建 127.698 秒；fmt、3969 文件行数门
（无 501+）通过。组织合同 3 项和版本检查输入未变，复用已有回执，不重复执行。
非测试产品直接取集成构建输出，未额外重建；product-final-2 workspace-check 2.129 秒通过。
产品 SHA-256：`4aac51300ca84406699736865072d94e31e3b140867e4ba7bec45ffa81541484`。
integration-final-2：build_resources 2、exe_resources 1、startup 7、session_startup 9 全部通过；
startup 整组 35.460 秒，session_startup 8.807 秒，单个 startup child 的原 15 秒门限未改变。

历史 input-4 的 integration-final/startup 仍保留 6 passed / 1 failed：wic_image_upload_survives_two_paints，
原报错 `Native startup child timed out; elapsed=22.4919661s`。本次源码修正的是菜单拒绝语义，不是启动性能，
因此最终样本通过不能关闭历史间歇超时根因；未对同输入失败刷绿。CPU→GPU 回升、未知关联终态恢复及完整迁移仍未完成，
本产品不是正式发布物，也不代表全部功能或全部测试验收通过。


## v437：启动等待归因（诊断回归通过，启动超时未修复）

本批只增加 opt-in 等待计时：25 个 smoke 文件的 31 处等待仍执行原 std::thread::sleep，
不改变请求时长、场景、断言和 15 秒门限。单次诊断记录 820 次等待共 1.656138 秒，仅 4 次达到 10 ms；
独立 PowerShell 的粗粒度 Sleep 现象不能外推为产品根因。已埋点文件操作约 0.326 秒，不削弱持久化；
隐藏按钮延后/批量显示实验未见稳定收益，未采用。保存/恢复 poll 先检查完成标记，未发现每轮重复保存/解码。
诊断 14.769 秒完成不是修复或验收通过，上一输入 18.969 秒的诊断也保留，不以自然波动宣称优化收益。

证据：`linshi/otter-native-v437-startup-wait/`。本批仅分配一次内部版本。
源码 SHA-256：`b93e570fd11af7cb2f850bc0ba6e921e1216c4df31a92ef1a1d6c2ad1e09d12f`。
编译 292.560 秒、严格 Clippy 133.439 秒、集成目标构建 98.611 秒、fmt、3965 文件行数门（无 501+）、
组织合同 3 项及版本检查通过。定向测试共 80 项：smoke_timing 5、session_job 13、workspace_checkpoint 33、tool_panel 29。
集成构建产出非测试产品 EXE，与早期诊断 EXE 字节不同，已拒绝冒充同字节复用；未额外重建产品，
直接封存集成构建的匹配 EXE/PDB，并针对该产品执行隔离 workspace-check（1.530 秒）和四组集成。
最终产品 SHA-256：`89377df976321ddbe11a691a7869394a4a864425fd0e2eca9a73e4f1e96a7697`。
build_resources 2、exe_resources 1、session_startup 9 通过；startup 5 通过/2 失败，隐藏空窗口与图片 smoke
仍超原 15 秒期限。透明钉图本次通过不证明历史超时已修复。未重跑刷绿，未运行无关可见 GPU 门。
启动性能根因及最终验收继续开放；停止无证据的等待策略优化，下一功能批次回到手动渲染恢复和 CPU 编辑/保存业务闭环。


## v436：CPU 钉图与兼容链故障恢复（相关回归通过，启动集成仍失败）

本批合并 D3D11 的实际 EndDraw/Present/Resize/opacity 失效标记、退休后 CPU 目的地、
传统 HWND 设备错误降级及初始未关联失败的软件钉图候选；任一关联不明仍阻止下一候选。
CPU 钉图保留整体透明度，使用有宿主身份 guard 的分层样式/首帧提交事务。Canvas 借用不跨样式或
UpdateLayeredWindow 调用；Target/DocumentStamp 在提交前后复核，重入换源不确认旧帧，销毁不回滚旧句柄。
此凭证不覆盖同文档全部 viewport/展示瞬态，不泛称任意旧展示状态均被拒绝。
新增测试覆盖 D3D12→D3D11→CPU、D3D11→CPU、传统 HWND 注入设备错误分类、初始 CPU 钉图、
样式重入换图/销毁，以及初始候选关联拒绝。真实驱动移除未注入，不以错误分类注入冒充硬件故障实测。

证据：`linshi/otter-native-v436-cpu-recovery/`。compile-1 的 E0277 失败保留；无 guard 的 unwind
测试夹具使用局部 AssertUnwindSafe 后冻结 input-2，未给生产类型添加 blanket UnwindSafe。
源码 SHA-256：`14f512b9562a7e011d957338dd162a4344102708298b5150bac6f27e0487ddaa`。
compile-2（661.745 秒）、严格 Clippy（252.129 秒）、fmt、3965 文件行数门（无 501+）、组织合同 3 项及版本检查通过。
9 组普通 runtime 共 239 passed / 9 ignored；另显式运行可见 GPU→CPU 门 2 项通过，故剩余 7 项 ignored 未执行。
集成目标构建 211.173 秒、产品构建 238.411 秒、隔离 workspace-check 1.897 秒通过。
产品 SHA-256：`319f717a3df253733b81a926dc6d300758191601c7b6f3b75a87aa26d82aeb24`。
build_resources 2、exe_resources 1、session_startup 9 通过；startup 4 通过/3 失败，仍为隐藏空窗口、
透明钉图、图片 smoke 超原 15 秒期限，不重跑刷绿或放宽超时。局部 CPU 恢复回归通过不代表完整产品验收。
手动恢复、真实驱动故障、同文档展示瞬态凭证及原计划完整退出条件仍未完成。

启动问题仅做只读调用链复核：存在重复投影计算，但无证据证明其解释秒级超时，未据此更改隐藏普通工作区
预创建合同、缓存或验收期限。v435 原启动失败保留，不重跑同输入历史验证。本批只分配一次内部版本。

## v435：运行中退休与兼容恢复事务（相关回归通过，启动集成仍失败）

按功能批次合并退休 owner 保活、绘制准入、窗口定时调度、成功后兼容续接、终态保文档、
销毁清理及对应测试。普通 discard/最小化/换源不清除恢复门禁；晚到完成不解锁失败/超时。
非恢复绘制错误不再发 WM_CLOSE，而保留文档与错误；完整 CPU 降级、手动恢复仍待后续批次。
本批只分配一次内部版本，冻结前集中完成测试与接线，不继承 v434 成绩。
无 DComp 关联的 HWND/CPU target 保留原有一次可恢复错误重建；有 DComp 的失败不直接 discard 后重建。
DComp 等待异步化不代表整个恢复全程无阻塞：确认后旧 D3D12 owner 的 GPU drain 仍沿用最多 5 秒 fence 等待，
失败时保留图与预算。此边界未在本批扩展为 GPU 异步回收，不宣称驱动卡死时 UI 无停顿。

证据：`linshi/otter-native-v435-render-recovery/`。首轮 compile-1 因父模块调用私有方法报 E0624，
保留失败；仅将方法收紧为父模块可见后冻结 input-2，再运行 compile-2，未为修复另增版本。
最终源码 SHA-256：`07ee10c918f184042cfde7b950f48d29dc4c9237a6809bbce2bb30b9eafe9303`。
编译（497.927 秒）、严格 Clippy、fmt、3963 文件有效行数门（无 501+）、组织合同和版本检查通过。
同源 8 组 runtime 共 231 项通过/9 项显式 ignored 未运行：Canvas 24、composition 123、host_lifetime 3、
software_surface 9、render_surface 5、workspace_checkpoint 33、projection_present 5、tool_panel 29。
新增窗口测试使用真实 D3D12 EndDraw 错误与生产 WM_PAINT/WM_TIMER，验证跨最小化/换源保活、
确认后兼容呈现、原错误直到成功帧才清除、旧 timer 不重启恢复；失败/超时使用确定性状态注入，
不声称真实驱动卡死或 SetTimer 系统分配失败已经实测。未重复运行未改动的显式可见 GPU 门。

集成目标构建 126.998 秒、产品构建 121.219 秒、隔离 workspace-check 2.491 秒通过。
产品 SHA-256：`38094ee15d541c0c2f671f51d8390f81411898f9d2df632c82fea528c1c96152`。
build_resources 2、exe_resources 1、session_startup 9 通过；startup 4 通过/3 失败，仍为隐藏空窗口、
透明钉图、图片 smoke 超原 15 秒期限，原始日志保留，不放宽/重跑。故本批恢复行为的相关回归通过，
但产品集中验证未全通过，不能标记完整迁移、产品验收或发布就绪。启动超时根因、CPU 钉图兜底、
完整自动降级/手动恢复及其余原计划退出条件仍开放。

## v434：启动计时归因补全（启动集成未全部通过）

内部版本 v0.2.33.434。仅增加 opt-in 计时的 process/thread 标识和面板定位/字体/首次投影、
共享捕获/快捷键/钉图阶段，不改变正常控制流、原错误、事务准备或 15 秒启动验收期限。
离线归因器按进程/线程严格匹配嵌套区间，扣除直接子区间以区分自身耗时；截断/错配日志拒绝出具结果，
用跨进程同 id、跨线程及嵌套合成样本验证，不把含嵌套累计当墙钟总耗时。
证据目录：`linshi/otter-native-v434-startup-attribution/`。v433 原启动失败继续保留，当前尚未宣称根因修复。

冻结输入 SHA-256：`d685e084c92a258865af57ebfbe593d6d15b96552f746d9c50dfd61ec47c7597`；
产品 SHA-256：`513b14fd08015b7c16e4b6523daa4d7c1e9f60c86219afc8fb1da228a7ae67c5`。
编译、严格 Clippy（约 190 秒）、集成目标构建（135.543 秒）、产品构建（146.217 秒）、
workspace-check（2.162 秒）、五组相关 runtime、格式/行数/组织/版本/diff 检查通过。
归因器两项测试通过；单次诊断 18.134 秒退出 0，仅诊断、不算 15 秒验收通过。
953 个配对区间中 controls 自身累计约 2.956 秒，restore-job 自身约 1.245 秒，
frozen-remove 自身约 0.987 秒；并非已证明根因。最终 verification.json 的 scopedChecksPassed=false，
startup 失败及完整集成回执保留；没有重复运行刷绿。本轮未运行可见 GPU 门，不宣称完整迁移或发布就绪。

## v433：Target 初始资源准备的退休事务（启动集成未通过）

内部版本 v0.2.33.433。新 canvas_target 将 surface 成功创建后的首个画刷准备纳入 Canvas 准入闭包，
完整 Target 成功后才发布及提交后端选择标志。画刷失败时仍持有完整 surface，先请求合成退休，
存在本项目 DComp 图则返回关联拒绝凭证，后续重绘由 v432 状态保文档并阻止重建；无图不伪造关联失败。
不新增线程、GPU 设备、预算或状态复位入口。运行中资源绑定/绘制错误与完整 CPU 恢复仍待后续实现。

新增隔离测试使用真实 GPU surface 加合成画刷失败，验证原错误、Target 未发布、文档/计数保持及再次 paint
被拒绝；软件对照验证无 DComp 图不进入关联阻止。此为注入闭包失败，不是真实驱动故障。
证据目录：`linshi/otter-native-v433-target-preparation/`，input-1 SHA-256：
`7985a6f043790a7bf4c346c84a83ec5fed4b0b57b4a61f96f8e9c96e27814981`。
编译、新增 GPU 失败门、D3D12 创建保留门和双后端可见迁移已通过。composition 123 项通过/8 ignored，
Canvas 16 项通过/1 ignored，render_surface 5、software_surface 9、render_frame 2、host_lifetime 3、
workspace_checkpoint 33、projection_present 5 项通过；显式隔离门单独计证，不冒充默认组全量执行。
严格 Clippy、fmt、版本、组织合同和 3960 文件有效行数门通过，无 501+ 文件。源码增量为 3 个 Rust 文件
（1 新增）及内部版本，UTF-8 无 BOM。

产品构建和隔离 workspace-check 通过，selected=d3d12；封存 EXE SHA-256：
`33c3c92d6bc0c0bfbe25ece6a25385e1e82d6b37f9f5cbe5be918c1cf79452ea`。
build_resources 2、exe_resources 1、session_startup 9 项通过，但 startup 为 4 通过/3 失败：隐藏空窗口、
透明钉图及图片三个完整 smoke 均超过原 child 15 秒期限。保留原失败，不放宽期限、不刷绿，v433 尚未完整验收。
一次隔离计时对照分别运行封存 v432 和 v433 的空窗口 smoke，两者退出成功但外层耗时约 15.095/15.730 秒；
这是最多 90 秒的诊断而非验收，不能覆盖原 15 秒失败。主要聚合阶段 application 分别约 9.839/10.597 秒，
restore-batch 约 3.220/3.287 秒；阶段有嵌套，不可直接求和。对照显示问题不只出现在本轮产物，
但未隔离机器负载/工具开销等变量，不能据此宣称根因已定或本轮必无回归。下一步按计时拆解启动瓶颈。

后续仅离线聚合上述两份日志，没有再次运行完整 smoke：每份均有 88 次 document-panel、35 次
tool-panel-controls；v432 累计分别为 4396/2701 毫秒，v433 为 4002/2362 毫秒，后者嵌套于前者，
不可相加。该样本中 v433 的面板累计成本反而较低，因此 application 总时间增加不能直接归因于面板退化。
独立隐藏 STATIC 父窗的固定 35 个 owner-draw BUTTON 基线（3 次）为 44.9384、31.3953、32.4114 毫秒，
全部自有 HWND 已随父窗销毁。该探针不含产品回调/主题/字体/DPI等完整条件，不是严格等价基准或根因证明。
证据为 timing-previous/current 的 aggregate.json 与 probe-controls.json。

源码复核确认普通工作区隐藏候选的控件提前准备属于事务合同（tool_panel_content_tests 明确保护）；
不能为让启动测试变绿直接套用隐藏钉图的延迟按钮策略。下一次有界测量应细分面板准备中尚未覆盖的
定位/字体/首次状态投影，并对 application 阶段按非重叠区间归因；当前不以未经证明的缓存或减少场景作修复。

## v432：Canvas 跨重绘准入与关联受阻保文档

内部版本 v0.2.33.432。创建关联失败凭证贯穿 composition/render_surface 两层选择器，直到 Canvas
才转回原始错误；补充上下文不丢失 fallback_allowed。Canvas 准入保存首个受阻错误，discard_device
不复位，后续重绘不能重新建立同 HWND 呈现链。WM_PAINT 在受阻时消费失效区但不清错误、不确认帧、
不自动重试，也不发送历史终态的 WM_CLOSE。仅这一失败类别采用新终态，其他绘制错误仍待迁移。

新增准入单测覆盖相同 HRESULT 的准备失败仍可重试、关联失败后重复创建闭包不执行；真实自有窗口测试
注入受阻状态后反复 WM_PAINT，核验 HWND 存活、没有 WM_CLOSE、Target 为空、文档与呈现计数不变。
这是合成状态测试，不是真实驱动失效，也不证明所有可见编辑/保存入口在故障状态下可用。
另验证兼容后端拒绝凭证经过外层错误上下文仍保留。首轮编译成功后补此合同测试并调整测试模块位置，
最终重新冻结 input-2，不能沿用 input-1 成绩。

证据：`linshi/otter-native-v432-canvas-admission/`；input-2 SHA-256：
`a929e2636b9fa9cc0643b5dcc38a72593e2174257efc5f1e4703394471d322d6`。
尚需退休 owner 的异步续接、手动恢复、成功创建后的绑定失败保护、运行中 GPU 降级与 CPU 业务验收。

最终同源编译、严格 Clippy、fmt、组织合同与版本检查通过，3959 文件有效行数门无 501+ 文件。
composition 123 项通过/8 ignored，Canvas 15、render_surface 5、software_surface 9、render_frame 2、
host_lifetime 3、workspace_checkpoint 33、projection_present 5 项通过；另显式执行 D3D12 创建失败保留门
1 项及双后端可见迁移 2 项通过，未执行的 ignored 项仍不计为通过。
全部 Cargo 构建结束后封存产品，EXE SHA-256：
`e72228e6017ca8e6690e70d024190d363a358a05a2ea9464bc7620b6063cb1e4`。
隔离 workspace-check 实际 selected=d3d12；build_resources 2、exe_resources 1、startup 7、session_startup 9
项通过，原 child 15 秒期限未改，集成前后产品与 target 同字节。源码增量为 11 个 Rust 文件（1 新增）
及内部版本，UTF-8 无 BOM。历史启动间歇根因未关闭，本切片不是完整恢复或 CPU 核心业务验收。

## v431：创建事务的关联状态准入

内部版本 v0.2.33.431。新增私有 composition_creation::Failure，分别携带原错误和 fallback_allowed。
未关联的准备失败允许下一个候选；尝试 SetContent/SetRoot/Commit 后失败一律拒绝当前创建事务的立即回退，
不因异步退休开始或错误 HRESULT 与准备失败相同而误放行。D3D12 默认 Canvas 选择器与 D3D11 hardware/WARP
选择器均消费凭证，原始 HRESULT/消息仍返回调用方；不增加同步等待、线程或资源预算。
这一保护只覆盖当前创建事务；跨 WM_PAINT 的重建、退休后的异步续接与最终保文档状态仍待窗口 owner 接线。

新增两个选择器回归证明关联未确认时兼容/WARP 闭包不被调用，并扩展真实 D3D12 部分关联失败门检查拒绝凭证。
已有相同 HRESULT 的未关联失败回退测试继续保留，防止以错误码黑名单替代关联状态。
首轮 compile-1 因旧 WARP 测试夹具仍传裸 Error 而失败；修复为类型化错误和关联前持有 shared owner，
保留原失败，重新冻结 input-2 验证。证据目录为 `linshi/otter-native-v431-candidate-admission/`。

最终 input-2 SHA-256：`f650e877fa1cf5ae9bc3667be00762992064439cb04dad59c5da2e71fa812acd`。
同源编译、严格 Clippy、fmt、版本与组织合同通过；3958 文件行数门通过，无 501+ 文件。
composition 123 项通过/8 ignored，render_surface 4、Canvas 13、software_surface 9、render_frame 2、
host_lifetime 3、workspace_checkpoint 33、projection_present 5 项通过；另外显式执行的 D3D12 创建失败
保留门 1 项和双后端可见迁移 2 项通过。未执行的 ignored 项仍保留，不当作全量测试成功。
全部 Cargo 构建结束后封存产品，EXE SHA-256：
`667f4005583fb8049d9d1ec2101865a14879513685aa9b74b1a2a5c6e80723ed`。
隔离 workspace-check 确认 selected=d3d12；build_resources 2、exe_resources 1、startup 7、session_startup 9
项通过，child 原 15 秒期限未改变，集成前后核产品同字节。相对 v430 为 10 个 Rust 文件（1 新增）及内部版本
的源码变化，增量 UTF-8 无 BOM。历史启动间歇根因、跨重绘准入与完整恢复继续开放。

## v430：D3D12 半完成构造的完整资源 owner

内部版本 v0.2.33.430。将 SetContent/SetRoot/Commit 移至完整 CompositionD3d12 owner 建立之后，
任一关联错误先发起既有异步退休，再返回原错误；未确认时由 ManuallyDrop 保留完整图和租约。
准备期 State 尚不执行合成关联；正常成功创建的调用顺序仍为资源准备、关联、提交。
新增隔离测试在真实图部分关联后注入失败凭证，检查原 HRESULT/消息、共享设备租约、图预算及 HWND 销毁。
这是合成故障测试，不是驱动故障或候选切换完成证明；未确认关联的后续候选准入仍需接线。

证据目录：`linshi/otter-native-v430-d3d12-creation/`。input-1 源码 SHA-256：
`09f74676e1a89887549fbbe8892953fbf415992f3252bc73567952ee41271395`。
同源编译、新增隔离资源保留门、D3D11/D3D12 两项可见迁移通过。composition 122 项通过/8 ignored，
Canvas 13、render_surface 3、software_surface 9、render_frame 2、host_lifetime 3、workspace_checkpoint 33、
projection_present 5 项通过；ignored 门按独立进程记录，不冒充默认组全量执行。
严格 Clippy all-targets、fmt、版本、组织合同和 3957 文件有效行数门通过，无 501+ 文件。
全部 Cargo 构建完成后封存产品，EXE SHA-256：
`a458c012236ae07cda25f80e63e799e2db79fe613dba6fe88824d250e74d219a`。
隔离 workspace-check 实际 selected=d3d12；build_resources 2、exe_resources 1、startup 7、session_startup 9
项通过，启动 child 原 15 秒门限未改变，集成前后产品与 target 同字节。源码增量为一个生产文件、
一个新增测试文件及内部版本，UTF-8 无 BOM。历史启动间歇根因、候选准入和完整自动恢复仍开放。

## v429：统一 RenderSurface 退休入口与双后端可见迁移

内部版本 v0.2.33.429。RenderSurface::retire_composition 统一转交 D3D12/Pin owner，
Workspace/Software 返回 None（没有本项目的 DComp 图，不是 GPU drain 完成凭证）。
入口不改 HWND 样式、不自动切换后端，后续 Canvas 事务必须持有原 owner 直至结果确认。
默认绘制和选择策略未改变。

composition_software_transition_tests 移至 desktop 的逻辑模块入口，复用同一套自有可见色块测试，
分别使用真实 RenderSurface::D3d12 和 RenderSurface::Pin。首帧及退休后的绘制拒绝也走 RenderSurface，
不为 D3D11 复制一套测试绘制实现。两种后端只在各自测试期间存在，不要求双后端常驻。
检查同 HWND/NOREDIRECTIONBITMAP、异步退休完成、CPU alpha/窗口 opacity、resize 和 144 DPI；
原 GPU owner 持有到最后一次软件像素检查完成。此门不等于驱动故障或产品自动降级验收。

证据：`linshi/otter-native-v429-shared-retirement/`。input-1 SHA-256：
`b0662a1f715ee2e726040753edc98fa7b711d302ca7f3dc049c6a35fca046fad`。
首轮两后端可见门及定向回归通过，但 Clippy 指出派发方法只有测试调用；未屏蔽 dead-code。
随后将真实 Canvas 退休接入产品 --smoke-test 末段，保留 Target 并核验文档凭证/paints/confirmed_paints
不变，再销毁窗口；启动集成增加对应标记断言。此轮询只用于隐藏自检，不进入默认 WM_PAINT。
原错误状态存在时保留原错误检查顺序，不用退休错误覆盖它。重新冻结输入并验证，不继承旧源成绩。
构造失败未确认关联的候选协调、保文档终态与手动选择仍开放，历史启动间歇根因也未关闭。

最终 input-3 源码 SHA-256：
`a30e20727a5e7e9ab4055518d4300d27e5636f6d883c346f0926ecbe76afca55`。
隐藏自检另设三秒外层上限；owner 两秒终态不等于停止系统等待。最终同源编译、Clippy all-targets
（-D warnings）、fmt 与 3956 文件行数门通过，无 501+ 文件。两项显式可见迁移门及独立进程的
D3D11 失败凭证保留门通过；composition 122 项通过/7 ignored，Canvas 13、render_surface 3、
software_surface 9、render_frame 2、host_lifetime 3、workspace_checkpoint 33、projection_present 5 项通过。
这些定向组不是全量测试，显式 ignored 门与默认组分别记录，不重复计算为额外独立覆盖。

全部 Cargo 构建结束后封存产品，EXE SHA-256：
`f933b7352be988c20ed7b392f3c2bdc28d29ad7bbbd876bf2fd6ddbacb69ca56`，位于证据目录
`product-final/otter-native.exe`。隔离 workspace-check 通过，实际 selected=d3d12；产品集成
build_resources 2、exe_resources 1、startup 7、session_startup 9 项通过，原启动 child 15 秒期限未改。
集成前后核 target 与封存产物同字节；相对 v428 冻结输入共 8 个 Rust 文件及内部版本变化，无新增物理源码文件，
增量文本 UTF-8 无 BOM。首轮 Clippy 失败保留，不以本次成功关闭历史间歇启动问题。
本轮仍只是显式退休入口及测试接线，不是默认产品 GPU→CPU 自动恢复完成，也不是正式发布包。

## v428：D3D11 完整退休 owner 与共享预算

内部版本 v0.2.33.428。Composition 将完整资源集合置于私有 ManuallyDrop<State>，
显式退休未确认时保留 D2D/交换链/合成图/共享设备租约及窗口图配额；正常关闭仍释放全部字段。
两后端共用 desktop::composition_retirement（最多两个 worker）和 composition_lifetime
（合计 MAX_UNITS 个窗口图，包含保留者；D3D12 设备代际仍另限两代）。历史 lifetime 文件路径未删除或复制实现。

D3D11 的 resize/opacity/present 拒绝退休状态；RenderSurface 与 Live 在 BeginDraw 前拒绝，relay
通过先执行 resize 的原入口得到相同保护。renderer 仍为可信内部资源适配接口，不宣称阻止任意裸 COM 绕过。
D3D11 构造先完成 D2D bind，再 SetContent/SetRoot/Commit；尝试关联后失败保留原错误并发起退休，
默认共享设备 Rc 已在失败前进入完整 owner。该保护不等于所有构造失败都已确认解绑：后续候选准入仍需
识别未确认关联，D3D12 半完成构造路径也仍待收口，不能据此直接授权 CPU 同 HWND 切换。

首轮编译发现 relay 测试遗漏的字段访问和 ManuallyDrop 借用拆分错误，原 compile-1 保留；
修正并补充独立进程的未确认 owner 保留测试后，input-2 编译、定向回归与 Clippy all-targets -D warnings 通过。
新测试持有真实 D3D11 图但注入失败凭证，确认析构后共享设备 Weak 仍有效、窗口图配额未归还、HWND 可销毁；
有意保留资源仅存于该独立测试进程，退出后由系统回收，不把它当真实驱动错误注入或长期回收验收。
原 hardware/WARP 像素/resize 测试额外验证真实退休完成和 RenderSurface 拒绝重绘，Live 测试额外拒绝退休后的 draw/present。
composition 122 项通过、6 项 ignored；保留 owner 测试另行单独通过，D3D12 同 HWND 可见门也重新通过，
这些显式执行项不可同时记作默认组已通过。Canvas 13 项、选择 3 项、软件 9 项、帧事务 2 项、宿主 3 项、
workspace_checkpoint 33 项、projection_present 5 项通过。产品/集成构建完成后封存 EXE：
`75e3f2deb85ceb376b69f41d70f1768d12586d12fa524f8c37bd3d60b4ca546c`。
隔离 workspace-check 通过（selected=d3d12，paints=2、device-recreated=true），build_resources 2 项、
exe_resources 1 项、startup 7 项、session_startup 9 项通过；未改变启动 child 的原 15 秒期限。
3956 文件行数、fmt、组织合同 3 项、版本及 diff 检查通过；源码增量 13 文件（1 新增），另更新内部版本，
均 UTF-8 无 BOM；Composition 221→284 有效行，新隔离测试为 54 行，其余变化详见 source-audit.json。
此轮启动通过不关闭历史间歇根因，也不是全产品功能/资源长稳验收。

证据：`linshi/otter-native-v428-d3d11-retirement/`。input-2 SHA-256：
`f59ef2b6b79396cc0c6d6539715f64c3792448c60f2827e05f9994cd8654a22d`；lib EXE SHA-256：
`2f07635b0f9dfe8e79a381c2cb9a77a27c1b1c2dda3cc97bc9e8aba9de0b7a0e`。
尚未启用 Canvas 自动退休/CPU 切换；D3D11 专门可见软件迁移、保文档终态、手动选择及 v425 启动间歇根因仍开放。

## v427：显式退休后的同 HWND 可见软件迁移

内部版本 v0.2.33.427。本轮仅修改 composition_software_transition_tests 验证入口与内部版本，
不改产品恢复策略。用 CompositionD3d12::retire_composition 替换测试中的手写同步
SetRoot/SetContent/Commit/WaitForCommitCompletion，Pending 时处理自有窗口消息，
只有 Complete 才进入分层软件提交；同时核验退休后 draw/present 被拒绝。
GPU owner 保持至软件像素检查结束，继续保留 NOREDIRECTIONBITMAP 和原 HWND。

显式 ignored 可见门通过：实际红帧→后台退休完成→半透明软件帧、窗口 opacity、32×32 实际尺寸、
144 DPI 下 8 DIP=12px 边界均通过，输出 async-retirement=true。该门仅显示并采样自有测试色块，
不发系统键鼠。健康设备正常退休成功不等于驱动卡死/超时恢复，也不代表默认 Canvas 已接自动降级。
同源 composition 122 项、Canvas 13 项、软件 9 项通过；composition 默认跳过 5 项，
其中本次单独执行了上述 1 项，剩余 ignored 不能算通过。Clippy all-targets -D warnings、fmt、
3955 文件行数门及版本一致性通过；修改测试有效行 224→243。

证据：`linshi/otter-native-v427-retirement-visible/`。input-1 SHA-256：
`ae6253ded5f40f871f6dc474a1e99f40309ace6c4f611c74520ba37829c63df1`；lib EXE SHA-256：
`fdfa38ec20d6ab89eb0f169368221ce31f7e324373720bc221883ae6b51d8299`。
本轮为验证入口迭代，没有构建/交付 v427 产品 EXE，也没有将 v426 产品集成成绩算作 v427 成绩。
v425 间歇启动根因仍开放。D3D11 完整保留与绘制准入的调用点审查见证据目录 d3d11-owner-review.md；
下一步处理该 owner 的预算/析构和创建失败边界，再复用共享退休 worker，继续 Canvas CPU 恢复主线。

## v426：D3D12 合成图显式退休

内部版本 v0.2.33.426。composition_retirement 只将每窗 DComp device/visual/target 引用交给专用工作线程，
按 SetContent(None)、SetRoot(None)、Commit、WaitForCommitCompletion 顺序执行；失败保留阶段/HRESULT。
GPU/D2D/Rc/窗口所有权不跨线程。依据 Microsoft 官方 architecture-and-components 的线程安全合同，
仅为这组三个 DComp 接口定义 Send 边界，不给完整 renderer 或任意 COM 指针添加 Send。

CompositionD3d12::retire_composition 为单向入口，调用后拒绝 draw/present/resize/opacity，重复调用只轮询。
进程最多两个在途 worker，无排队；UI 轮询两秒后为终态 TimedOut，不取消系统等待，也不接受晚到成功。
worker 真正结束前不释放其并发名额。未得到 Complete 而析构时完整保留 State 与既有图预算租约；
不再次 Commit、不在此路径额外 drain。此保守保留可能持续到进程结束，不声称正常回收成功。
Complete 只证明 DComp 处理完成；正常销毁仍需既有 GPU drain，二者不能混同。

此阶段只接入 D3D12 owner 的显式 API，不自动发起退休、不改变默认后端、未接 D3D11/Canvas CPU 切换。
创建失败的半完成合成图、完整软件恢复状态机和资源长稳仍待后续处理；v425 startup 间歇超时仍开放。
证据目录：`linshi/otter-native-v426-dcomp-retirement/`。input-1 编译及首轮同源定向验证通过，
源码 SHA-256：`f9561b810610648fa496d98df86e9dfa66b590eea8058ea2a99b5e4790ff19da`；lib EXE SHA-256：
`fae17943883ec2e1e63a9f7c89baec7c79c66a286eedf04dffafa45a40876675`。
退休协议 5 项、真实 D3D12 7 项（含新增真实退休）通过；composition 总组 122 项通过、5 项 ignored，
其中包含前述专门组，不能重复累计。Canvas 13 项、软件 9 项、选择 3 项、帧事务 2 项通过。
新增模块为 174/99 有效行，D3D12 owner 为 285 行、其测试为 436 行；3955 文件行数门、fmt 通过。
这些测试未注入真实驱动卡死，也未验证未确认完整 GPU 图的长期保留/进程退出回收，不冒充故障恢复验收。
Clippy all-targets -D warnings、组织合同 3 项、版本一致性及 git diff --check 通过。
产品及集成构建完成后封存 EXE，SHA-256：
`7c4a1b39f3d2c6e5e82aeb6da0258e0f48e9597698ad35475eb9089da365d3cf`。
隔离 workspace-check 通过（selected=d3d12，paints=2、device-recreated=true），build_resources 2 项、
exe_resources 1 项、startup 7 项、session_startup 9 项均通过；startup 原 child 15 秒期限未改变。
本次启动通过不关闭 v425 等历史间歇根因。源输入及集成前后产品同字节已核对，未发现 linshi 下残留产品进程。
`verification.json` 只声明本切片检查通过，fullMigrationComplete/publishable 均为 false。
下一步将现有同 HWND 可见迁移测试的手写同步解绑替换为此 API，完成 D3D11 owner 与失败构造路径，
再接 Canvas 软件状态机；具体缺口见证据目录 next.md，不重复把隐藏退休测试当可见 CPU 迁移验收。

## v425：普通工作区 Canvas 软件兜底（启动集成未通过）

内部版本 v0.2.33.425。真实 Canvas/RenderSurface 新增 software 分支：D3D12 创建失败仍先尝试
原 HWND target，后者创建失败才创建软件后端；钉图原 D3D11 hardware/WARP 链保持不变。
软件选择在 discard_device/后续重绘后保持，不反复探测 GPU。render_software 只生成独立帧，
procedure_paint 在释放 Canvas/resolved_display 借用后提交；成功后复核宿主、Target 身份及文档凭证。
只有正常本地文档帧增加 confirmed_paints，外部 display/缩略不计入；未提交、失败或过期帧不计成功。
WM_PAINT owner 从 procedure 拆出，嵌套保护只记 deferred，不清错误或创建第二后端，外层退出有界补失效。
现有最终 WM_CLOSE 错误策略尚未替换；这不是运行中 GPU 故障恢复、钉图 CPU 或完整保文档终态完成。

新增 3 项真实 Canvas/生产消息测试，内部显式选软件，不伪造本机硬件不可用；覆盖 DPI/resize、
延迟确认、选择保持、过期 Target、同 Target 换文档、无效 HWND 失败以及嵌套保护分支。
嵌套用例显式设置 active，验证的是保护分支，不是声称 BitBlt 实际触发嵌套 WM_PAINT。
创建失败选择另由惰性/单次/保错误的合同测试覆盖。静态审查未发现 raster/effect 必须 GPU DeviceContext，
但 blur/mosaic/erase 全部效果的 CPU 视觉验收不因接口兼容而自动完成。
证据根 `linshi/otter-native-v425-canvas-software/`。首轮编译因子模块导入层级错误失败，
并报告测试未使用导入，已修正并保存原日志。input-2 已重新冻结、编译并通过定向测试，不继承旧输入成绩。
源码 SHA-256：`56550a58056203540ae5a9877010e9058277594af916a83c2218731781e83fac`；lib EXE SHA-256：
`259301d856518dd8b3b69a37752b662dff522fc07fbc7b502a3f21727083daad`。
软件 Canvas 专门组 3 项、软件/样式 9 项、选择合同 3 项通过；扩展 Canvas 13 项、composition 116 项
（5 项 ignored 不计通过）、host_lifetime 3 项、帧事务 2 项、workspace_checkpoint 33 项、
projection_present 5 项通过。显式可见的同 HWND GPU→软件透明度/尺寸/DPI 门也重新通过。
增量 11 文件/4 新增，新增文件 47/49/99/140 有效行；procedure 480→438、Canvas 385→398，
增量 UTF-8 无 BOM/空白、3953 文件行数门、组织合同 3 项通过。
上述为 input-2 结果。Clippy 随后发现生产 Canvas::paint 未被调用（dead-code）；已恢复普通文档走 paint、
外部显示走 paint_display 的原入口区分，不屏蔽 lint。input-3 已重新冻结、编译并重跑上述全部定向组及
显式可见门，均通过；Clippy all-targets -D warnings 通过。procedure_paint 现为 103 有效行，原失败回执保留。
最终源码 SHA-256：`8c67a23fb701eab176463064c4f0daa1d661af72a972256659768f2d7495e31b`；lib EXE SHA-256：
`e004276047eed2be2d39b7992153e263380ed340f783a72fa1df411d3a5be560`。产品及集成测试构建成功。
封存 EXE：`product-final/otter-native.exe`，SHA-256：
`994af71f441283f8de0d74a3516d5f97ecbab76fc07de4338432b3cf58b86eb6`。
隔离 workspace-check 通过，实际 selected=d3d12，paints=2、device-recreated=true；不是软件产品验收。
产品集成 build_resources 2 项、exe_resources 1 项、session_startup 9 项通过；startup 为 5 通过/2 失败。
hidden_window_renders_and_destroys_without_webview 与 transparent_pin_creates_compositor_and_rebuilds_device_resources
均超过原 child 15 秒门。原始回执保留在 `integration-final/`，不生成全部成功的 closeout。

随后只对 v424/v425 的封存 EXE 各运行 hidden/pin 一个计时诊断样本，仍为 15 秒期限，
不覆盖共享 target、不继承真实 profile；证据为 `startup-diagnostic-1/`。
v424 hidden/pin 分别 exit 0（13.238/13.544 秒）；v425 hidden exit 0（13.043 秒），pin 超时。
这些时间包含 RTK 启动；pin 超时回执 17.350 秒还包括超时后的自有进程回收检查，不是延长验收期限。
v425 pin 的 application 阶段用时 10004 ms（v424 pin 为 7820 ms），随后完成 crop-smoke，
最后记录 flip-smoke begin，尚无对应 end；原始 pin 失败则已经完成 marquee-smoke。
不同停点及单次对照不足以确认固定死锁、新回归或机器负载根因；诊断通过样本不抵消原失败。
封存产品前后哈希未变，诊断结束后未发现 linshi 下遗留 otter-native.exe。
下一步针对耗时阶段与最终 paint/discard/destroy 边界补精确证据；不反复重跑刷绿、不放宽门限。

2026-10-29 继续诊断：input-4 只改 startup_smoke/flip_smoke 两个既有验证入口，
沿用显式 OTTER_NATIVE_SMOKE_TIMING=1，增加最终首次绘制、discard、重建绘制、destroy 及
flip worker/paint/hit-region/cancel 的计时，不改变操作顺序、错误检查或验收期限。
有效行分别 104→113、184→187，官方全仓行数门及 fmt 通过；不是超时根因修复。
新输入 SHA-256：`581c43f68e04499d66a237a81f783189e5f0c0bf719024556dbd24d4f8cd94a7`。
诊断产品 `product-diagnostic-4/otter-native.exe` SHA-256：
`e9deb52a500141de52a388115b55cbdd36e96d587e685784ee4f4e417d3994a2`，与 input-3 产品分别保留。
隔离 workspace-check 通过；startup-diagnostic-2 的 hidden/pin 单次样本均 exit 0（13.308/13.049 秒）。
样本的最终 discard 为 65/66 ms、destroy 为 70/71 ms，重建绘制为 371/401 ms；
pin flip-worker-wait 为 43/35 ms，flip-cancel-wait 为 38/30 ms。它们只证明本次没有在这些边界长时间停留，
不能用来排除历史故障或认定机器负载根因。application 阶段为 9802/6712 ms，嵌套阶段不可累加计时。
同源 run-4 的计时工具 5 项、Canvas 13 项、软件 9 项、选择 3 项、帧事务 2 项、宿主 3 项均通过。
Clippy all-targets -D warnings 通过；源码与 input-4 一致，修改文本 UTF-8 无 BOM，git diff --check 通过。
诊断后未发现 linshi 下残留 otter-native.exe。计时 ok=true 仅表示该调用返回，像素/错误状态仍由原断言判定。
input-3 的完整集成成绩不转记为 input-4；原 startup 失败仍开放，本轮不是正式候选收尾。
同步联网核实的 DComp 完成边界见证据目录 `dcomp-docs/review.md`：GetFrameStatistics 不提供特定 Commit
完成凭证，WaitForCommitCompletion 无 timeout 参数，不据此实现 WM_PAINT 内同步等待或未经证明的跨线程迁移。

## v424：软件帧独立提交与重入生命期

内部版本 v0.2.33.424。新增 software_submission，software_surface::prepare_present 不调用窗口 API，
返回单次消费的 Rc DIB/DC/像素预算租约。绘制入口拒绝覆盖尚在途的同一 DIB；resize 可绑定新 DIB，
旧帧仍有独立寿命；取消、提交成功或失败均释放凭证，没有新增像素复制或后台队列。
便利 present 仍委托此路径；未来 Canvas 接线必须先释放模型借用，再使用独立凭证提交并确认计数。
裸 COM resource_target 出口收窄为内部测试；绘制闭包是可信边界，不允许保存 target 或自行启停帧，
不能把 Rc 检查宣称为对任意 unsafe COM 调用的类型级隔离。

证据根 `linshi/otter-native-v424-software-submission/`。input-1 编译通过，软件组 8 通过/1 失败：
测试期待 UpdateLayeredWindow resize 同步发送 WM_SIZE，实测没有发生，原日志保留。
input-2 改以真实 WM_STYLECHANGING 通知验证软件迁移必经的样式重入；计数在创建 HWND 之后清零，
回调实际重新借用模型并释放原 renderer，随后凭证仍成功提交，窗口实际为 32×32。
不可覆盖/取消/失败释放及 resize/target 析构后的旧新像素独立测试同样通过；总软件/样式组 9 项通过。
显式可见的同 HWND D3D12→软件、alpha、窗口透明度、尺寸与 144 DPI 几何门重新通过。
源码 SHA-256：`cc8a00d7cc0bfd7c1cf313a4f3b03fd733344ed2e02698064064d0072984c67b`；lib EXE SHA-256：
`ff2940a08dac77674f057a9f6b6acaf4b228c4b61e6c4dd0cd4c479685da7b93`。

增量 4 文件/2 新增：submission 106 有效行、测试 181 行；software_surface 从 181 降至 125 行，
原测试 230→233。增量 UTF-8 无 BOM/空白、3949 文件行数门和组织合同 3 项通过。
同源 Canvas 10 项、composition 116 项（5 项 ignored 不计通过）、host_lifetime 3 项、帧事务 2 项、
呈现选择 2 项、workspace_checkpoint 33 项、projection_present 5 项通过；Clippy all-targets -D warnings 通过。
最终 fmt、版本与空白检查通过。全部 Cargo 完成后封存开发 EXE（45332992 字节），SHA-256：
`28292cbae655eeedff358f7db85d070f6b64b891ad30ea0d7263d4eab2a93939`。
隔离 workspace-check 1.729 秒 exit 0，实际 selected=d3d12、paints=2、device-recreated=true；
四项产品集成 build_resources 2 项、exe_resources 1 项、startup 7 项（31.027 秒）、session_startup 9 项
（9.338 秒）通过；集成前后核 target 与封存产品同字节，不改变原 child 超时门限。
本切片尚未接 Canvas/WM_PAINT 的软件状态机，不是 CPU 产品底线或真实驱动故障恢复验收，也不是正式发布。

## v423：同 HWND 合成到软件呈现

内部版本 v0.2.33.423。软件分层提交与样式事务允许 pinned 窗口保留 NOREDIRECTIONBITMAP，
始终只修改 LAYERED；普通 BitBlt 工作区仍拒绝无重定向窗口。尚未启用 Canvas 自动 CPU 回退。
新增显式 ignored 可见门：自有 NOACTIVATE/TOPMOST 蓝色背景与 D3D12 红色前窗，逐项确认真实
SetRoot(None)/SetContent(None)/Commit/WaitForCommitCompletion，保留 GPU 图，再在同 HWND 上
提交软件半透明绿色，读取测试区域中心的桌面像素，检查 alpha、窗口透明度、resize 和 144 DPI。
该测试等待合成提交是验证手段，不是生产有界恢复 API；不发送全局输入、不修改游戏或真实 profile。
证据根 `linshi/otter-native-v423-same-hwnd-software/`。首轮编译因 WNDPROC 的 Rust/system ABI
不匹配失败，已增加显式 extern system 转发函数。联网核实 WNDCLASS 官方文档 HTTP 200，
窗口类背景刷在 UnregisterClass 时由系统释放，注册成功不重复 DeleteObject；注册失败自行释放。
input-2 编译和软件/样式 6 项通过，可见门在初始红帧失败（实际采到蓝色），尚未进入软件迁移。
input-3 补线程 DPI scope（结束恢复）、显式无激活层序和有界消息处理；不据此断定原失败单一根因。
补强 resize/DPI 断言：实际 HWND 为 32×32，8 DIP 红块在 144 DPI 下边界为 12 物理像素，
分别在 10/14 像素处检查红块/绿色半透明背景，避免纯色 Clear 假阳性。
重新冻结、编译和软件/样式 6 项、显式可见门 1 项通过；源码 SHA-256：
`8f49803beb447fa2f592291b956d3ff7ecc38b3e7f96b8537bcb32cf532e7b8f`，lib EXE SHA-256：
`d5f7cd846331d430e31b14b6fc2f81401feb3d955d462784f67f68aee8ef8e65`。
新增测试 224 有效行，5 个 native 文件增量 UTF-8 无 BOM/空白检查、3947 文件行数门和组织合同 3 项通过。
正常解绑后同 HWND 软件显示已获本机证据；真实 GPU 故障、生产有界解绑和 Canvas CPU 接线仍未完成。
同源回归：D3D12 专门组 6 项、Canvas 10 项、composition 广义组 116 项通过（5 项 ignored 不计通过，
其中本次迁移门已单独显式运行）；host_lifetime 3 项、render_frame 2 项、render_surface 2 项、
workspace_checkpoint 33 项、projection_present 5 项通过。两次过窄过滤器匹配 0 项，脚本正确判失败；
改用实际模块过滤器后重跑，不把零测试算通过。Clippy all-targets -D warnings、最终 fmt 和版本检查通过。
全部编译结束后封存开发 EXE（45332992 字节），SHA-256：
`49dd7603d7158891eb231041734defe6656fdc4c989003b11cb980d8d4f10639`。
隔离 workspace-check 1.16 秒 exit 0，确认实际 selected=d3d12、paints=2、device-recreated=true；
产品集成 build_resources 2 项、exe_resources 1 项、startup 7 项（28.696 秒）、session_startup 9 项
（8.198 秒）通过，集成前后核 target 与封存产品同字节。历史启动超时根因仍开放；不是正式发布或完整包验收。

## v422：文档宿主绑定前置保护

内部版本 v0.2.33.422。host_lifetime::Registration 在 WM_NCCREATE 阶段绑定精确 HWND；
同一 State 的重复候选在写入 GWLP_USERDATA 之前拒绝，不等到 WM_CREATE 才发现宿主重复登记。
消息业务路由与工具面板刷新均校验绑定身份，正常 WM_NCDESTROY 后释放绑定；无 State 的隐藏候选
仅走默认窗口过程、不计入文档宿主数。它为后续替代宿主提供隔离前提，尚未实现替换/转移操作。
新增真实隐藏窗口测试，在持有原 Canvas 借用时拒绝重复候选、创建并销毁无状态候选，验证没有
ABI panic/WM_QUIT、原宿主登记与文档身份保留、仍能绘制，最终正常关闭仅产生一次成功退出。
修改前有效行 host_lifetime 111、procedure 480；证据根 `linshi/otter-native-v422-host-binding/`。
input-1 冻结 2794 项并编译成功，源码 SHA-256：
`4cd806cf952babd1a0f91831a34dfbb9fd17e4b6858d808d406701aafbed852b`。
同源 lib EXE SHA-256：`b75aa665322771fa6125692a80412f77eec812d1fa8b67812e2392c450d7ae86`。
host_lifetime 3/0/0、window_priority 4/0/0、Canvas 10/0/0、software_surface 6/0/0，
window_geometry_wake 3/0/0、workspace_checkpoint 33/0/0；后者复用真实隐藏 Application 检查点链。
增量 3 文件/1 新增，新测试 108 有效行，host_lifetime 111→154、procedure 保持 480；
UTF-8 无 BOM/增量空白、版本、组织合同 3 项及 3946 文件行检查通过。
扩展回归 D3D12 7/0/0、composition 109/0/4、render_frame 2/0/0、render_surface 2/0/0、
projection_present 5/0/0、隐藏合成 TLS 子节点链 1/0/0 均通过；4 项既有交互门未执行，不计通过。
上述为 input-1 的历史结果，不继承给修改后的输入。Clippy 首轮因新增测试的整数哨兵指针触发
`clippy::manual_dangling_ptr` 而失败；已改为两个本地槽位的地址（只用于身份模型，不传 Win32），
未屏蔽 lint。重新冻结 input-2、重编及全套同源定向回归；原 Clippy 失败回执保留。
input-2 已重新编译封存，源码 SHA-256：
`84b4e39a8da277301a0afef82b746e792478c3578ab9323c375cc3019b2068be`；lib EXE SHA-256：
`52bc834a6bbcc992e3d6c9c171a9440982282ef3bf26a2bffb9b7405e6c33e9f`。
修正后 host_lifetime 3/0/0、window_priority 4/0/0、Canvas 10/0/0、software_surface 6/0/0、
window_geometry_wake 3/0/0、workspace_checkpoint 33/0/0，以及 D3D12 7/0/0、composition 109/0/4、
render_frame 2/0/0、render_surface 2/0/0、projection_present 5/0/0、隐藏合成 TLS 子节点链 1/0/0
已重新通过；新测试为 109 有效行。最终输入的格式/行数/增量 UTF-8 与空白检查、
Clippy all-targets -D warnings 复核通过。全部 Cargo 完成后封存产品，EXE 45332992 字节，SHA-256：
`9688fc792fdd6a1a6c6fb09b3691a49fad219423c086d2efd6414efdef229e70`；匹配 PDB 294334464 字节，
SHA-256 `b4093855a927e582ed3cfe8b5a1e62366b59175948b0a1c473ec5fe887c9eebb`。
隔离隐藏 --workspace-check exit 0，2.353 秒，两次 selected=d3d12、paints=2、device-recreated=true，
未拒绝主链或选择兼容链。集成 build_resources 2/0/0、exe_resources 1/0/0、startup 7/0/0（37.401 秒）、
session_startup 9/0/0（9.074 秒），集成前后核同字节，保原 child 15 秒门；历史启动超时根因仍开放。
隐藏 TLS 链不替代真实 Loom 双端验收。

本轮额外探针 `probe-layered-redirection-1` 的四种初始样式（80/80080/200080/280080）均可在
保留 NOREDIRECTIONBITMAP 的情况下增加 LAYERED 并成功 UpdateLayeredWindow（styleError/presentError=0）。
因此 v421“不能移除标记，所以必须替换 HWND”的推断证据不足，现撤回必然性判断。此探针使用真实
隐藏 HWND 和预乘 BGRA DIB，但没有旧 DComp 图；不证明屏幕观感、GPU→CPU 切换或故障恢复。
下一步优先验证真实合成树解绑后同 HWND 的软件提交与可见像素，再决定是否需要完整宿主替换。

## v421：软件窗口样式事务

内部版本 v0.2.33.421。新增 software_window_style 及测试，只事务性迁移可重定向自有窗口的分层标记，
不修改系统配置、外部窗口或前台焦点。调用者必须保证 UI 线程及 HWND 生命期、已解绑旧合成树，
并释放会被重入窗口过程借用的 State/Canvas。失败和 panic 时恢复原来的分层位，不覆盖其他 owner 的位。
提交检测位被重入修改；回滚失败保留诊断且不循环尝试。软件后端自动选择/运行中降级仍未接线。
证据根：`linshi/otter-native-v421-software-window-style/`。首轮 input-1 编译成功，定向测试 3 通过/2 失败；
begin 返回 ERROR_INVALID_PARAMETER (87)。独立 probe-style-2 在 STATIC/自定义窗口类上复核：
已有 NOREDIRECTIONBITMAP 无法原地移除，失败写入还可能已经修改 LAYERED。不把失败调用当成原子无修改。
现改为任何写入前拒绝此类窗口并要求替代 HWND；合法可重定向窗口才允许分层事务。
同时修正 unwind 测试的假阳性：begin 必须在 catch_unwind 外成功，不能把 begin.unwrap 的 panic 当清理测试通过。
原失败保留；input-2 重新冻结并编译验证，不继承 input-1 成绩。2793 项源码 SHA-256：
`4e644f2b3ad9f3d30596f3a679b6cb7522dde9d263fc615013b499f23e2b0290`。
同源 lib EXE SHA-256：`4b0a824df17ae158bbd0ac88a37afb89dcccc1deef7bf1232e74405485975620`。
软件呈现/样式事务 6/0/0，render_frame 2/0/0、Canvas 10/0/0、D3D12 7/0/0、composition 109/0/4、
render_surface 2/0/0、projection_present 5/0/0、隐藏合成 TLS 链 1/0/0；4 项既有交互门未执行。
Clippy all-targets -D warnings、版本、组织合同 3 项及 3945 文件行检查通过。增量 3 文件、2 新增，
software_surface 178→180 有效行，新 owner/test 为 114/195 行；增量 UTF-8 无 BOM/空白检查通过。
测试窗口保持隐藏，不代替真实屏幕观感、生产窗口同步重入、替代 HWND 保文档或故障恢复验收。
全部 Cargo 完成后封存产品，EXE 45331968 字节，SHA-256：
`015bafd7e46503007484479c13981292442804946c7487958d28d0a1db283dc1`；匹配 PDB 294391808 字节，
SHA-256 `7b65da78010e9d895bdbf99bc904c963ab50582ea15706079dc9da5e5c254f85`。
隔离隐藏 --workspace-check exit 0，2.209 秒，两次 selected=d3d12、paints=2、device-recreated=true，
未拒绝主链或选择兼容链。四项集成 build_resources 2/0/0、exe_resources 1/0/0、startup 7/0/0（38.962 秒）、
session_startup 9/0/0（12.524 秒）；集成前后产物同字节，保原 child 15 秒门，不关闭历史启动超时根因。
下一步由窗口 owner 实现替代宿主的保文档事务，不以本轮样式组件或正常 D3D12 启动验收代替 CPU 自动降级。

## v420：软件 D2D/GDI 呈现候选

内部版本 v0.2.33.420。新增 SoftwareSurface/Bitmap，显式 D2D1_RENDER_TARGET_TYPE_SOFTWARE
绘制到 top-down BGRA DIB，普通 HWND 用 BitBlt，透明钉图用 UpdateLayeredWindow；
不创建本项目的 D3D11/D3D12 device、DXGI 交换链或 DComp 树，仍要求 Windows Direct2D/GDI 可用。
候选不修改 HWND 样式、不接入默认 Canvas，也不宣称 CPU 编辑/保存链已经验收。

2026-10-28 联网获取官方文档，HTTP 200 原文保留于证据目录 official-1/2/3.html：
[Direct2D/GDI 互操作](https://learn.microsoft.com/en-us/windows/win32/direct2d/direct2d-and-gdi-interoperation-overview)、
[UpdateLayeredWindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-updatelayeredwindow)、
[渲染类型](https://learn.microsoft.com/en-us/windows/win32/api/d2d1/ne-d2d1-d2d1_render_target_type)。
官方明确 SOFTWARE 为 software rendering only，DC target 有内部位图再经 GDI 提交。

DIB 实际像素和两份内部位图历史高水位预留共享原 512 MiB 像素预算，限制 8192 边长/16M 像素；
缩小时不假定内部缓存已释放，resize 短时同时持有旧/新 DIB 均占预算。该预留不等于全部 COM/GDI 内存已精确计量。
BindDC 失败保留旧/新两个 DC 到 target 析构，避免猜测回滚后释放仍被引用的 DC；
释放前排空 GDI 批次、恢复旧选入对象，再释放 bitmap/DC/预算。未执行长稳 GDI 句柄/总内存峰值验收。

证据根：`linshi/otter-native-v420-software-surface/`。初次编译期间强化高水位预算和失败绑定寿命，
随后重新冻结 input-4 并重编，不继承旧输入成绩。最终 2791 项，源码 SHA-256：
`b980df0aed34a1c0919617c69ad67dc61954bf42bfa06acee2eb206d279149b5`。
同源测试 EXE SHA-256：`f3d3fe41ae36c838625ae1b4edca0035790c59e881bff846eee3e369e53a4f16`。
software_surface 2/0/0，验证预乘 alpha/图片/文字、resize/非法尺寸/高水位、错误及 panic 后恢复、
真实隐藏 HWND/分层窗口提交、无完整帧拒绝、样式/透明度拒绝及整体透明度不修改位图；
隐藏提交成功不代表最终屏幕观感或禁用所有 GPU 后的整机验收。render_frame 2/0/0、canvas 10/0/0。
增量 4 文件、3 新增；新模块有效行 119/178/230，mod 447→448。UTF-8 无 BOM/空白、fmt、版本、
组织合同 3 项及 3943 文件行检查通过。扩展回归 D3D12 7/0/0、composition 109/0/4、
render_surface 2/0/0、projection_present 5/0/0、隐藏合成 TLS 子节点链 1/0/0，以及
Clippy all-targets -D warnings 通过。4 项既有交互门未执行；TLS 链不替代真实 Loom 双端验收。
全部 Cargo 完成后封存产品，EXE 45331968 字节，SHA-256：
`e8d400427a0fd6f1512c5cd15181a95a27eb1f64a0545634563d6317385f0976`；匹配 PDB 294350848 字节，
SHA-256 `f2909935b24f2122b657de4e69568f0bf7404b63cd2a7ff9c6c29ccd3f6d2d4d`。
隔离隐藏 --workspace-check exit 0，1.858 秒；两次 selected=d3d12，paints=2、device-recreated=true，
无主链拒绝或兼容选择。集成 build_resources 2/0/0、exe_resources 1/0/0、startup 7/0/0（37.056 秒）、
session_startup 9/0/0（10.186 秒），保留原 child 15 秒门；新成功不关闭历史启动超时根因。
下一切片须接入软件 Canvas 与窗口样式迁移，并补全兼容路径失败后的保文档降级，不能以候选验收替代 CPU 产品底线。

## v419：运行中 D3D12 单向降级

内部版本 v0.2.33.419；后端明确报告帧/资源/设备代际失效时，Canvas 锁定兼容选择并退休 GPU Target，
不替换文档、编辑历史、图像或呈现计数。WM_PAINT 为该单向降级安排一次重绘，原可恢复 HRESULT
的单次重建规则保留，日志仍记录原始错误。创建期已回退的窗口也锁定兼容路径，普通 discard_device
不重新探测 D3D12，避免最小化/重绘在后端之间反复切换。
业务闭包错误不因 E_FAIL 自动降级；查询后端 failed、backbuffer 完整性及设备状态决定是否允许降级。
兼容后端仍失败后的最终保文档处理、手动选择、真实 GPU hang/移除和 CPU 底线尚未完成。

真实隐藏 Canvas 场景扩展了无匹配 Push 的 Pop 导致 EndDraw 错误、切换兼容链后重绘、再次丢弃 GPU
资源仍保持兼容、文档凭证不变；另覆盖普通业务错误不降级。该测试直接调用 Canvas 恢复入口，
不是实际驱动故障，也不把它冒充完整 WM_PAINT 故障注入或屏幕观感验收。

证据根：`linshi/otter-native-v419-render-recovery/`。input-3 冻结 2788 项，源码 SHA-256：
`0838f7206223cdcbebdf3b78987115d2185242c6d4beb0fbc1e9fa9849dd8e07`。
增量 6 份源码、1 份新增，最大 480 有效行；procedure 479→480 仅接线，Canvas 376→385，
新增 recovery 17 行独立拥有单向降级，测试 89→125。UTF-8 无 BOM/空白、fmt、版本和 3940 文件行检查通过。
同源 lib 编译并封存，测试 EXE SHA-256：
`e8e5d1e5488507e6f236f4f6ec4432bd97d2dcb9e94daa99983ce71adfb6ac51`。
canvas 10/0/0、render_surface 2/0/0、D3D12 7/0/0、composition 109/0/4；4 项既有交互门未执行，
不计通过。组织合同 3 项、projection_present 5/0/0、隐藏合成 TLS 子节点链 1/0/0、render_frame 2/0/0，
以及 Clippy all-targets -D warnings 均通过；TLS 链不代替真实 Loom 双端/Shader 验收。
全部 Cargo 完成后封存产品，EXE 45331968 字节，SHA-256
`05f8f63f52e96369a50f8386abd94c285a538ffa561c71dcc552a068018aa109`；匹配 PDB 294342656 字节，
SHA-256 `eda0b39891e5c1038f885926f6486c3434a45085ad029a89225548d8aa019bc3`。
隔离隐藏 --workspace-check exit 0，1.542 秒；两次 selected=d3d12，paints=2、device-recreated=true，
无主链拒绝或兼容链选择，证明正常产品启动仍走 D3D12。
集成 build_resources 2/0/0、exe_resources 1/0/0、startup 7/0/0（29.278 秒）、session_startup 9/0/0（7.409 秒）；
集成前后核同字节，保原 child 15 秒门。上述成功不证明真实驱动故障恢复或历史启动超时根因已修复。
兼容链也失效后的最终保文档兜底及其他完整迁移缺口继续开放。

## v418：Canvas 默认 D3D12 接线

内部版本 v0.2.33.418；RenderSurface 在工作区/钉图创建时优先尝试完整 D3D12 链，创建失败后
尝试原兼容链一次。钉图兼容链继续拥有 D3D11 hardware/WARP，普通工作区继续保留 HWND target。
成功主链不创建兼容链，日志区分所选后端/创建阶段及失败 HRESULT；最终失败保留两条链的原因。
D3D12 绘制通过自身帧事务提交，不额外嵌套 render_frame 的 BeginDraw/EndDraw。
这仅是每窗创建期选择：手动选择、运行中保文档恢复、原生 Shader 和 CPU 独立底线均未完成。

新增 canvas_d3d12_tests 覆盖真实 Canvas 工作区/钉图的默认分支、图像、resize/DPI、丢弃设备后重绘
和文档凭证保留；render_surface_selection 测试覆盖主链成功时兼容链保持冷态、有界回退和双错误。
新窗口测试仍是隐藏 HWND，不证明最终屏幕观感；候选不可用后的回退模型也不代替真实驱动故障。

证据根：`linshi/otter-native-v418-canvas-d3d12/`。首次编译期间补正两处旧后端注释，未继承其成绩；
最终重新冻结 input-3，2787 项，源码 SHA-256：
`d33b8c53f50c4dda80745695c3e151999350eb22a62084d15804c42662f464f6`。
增量 6 份源码（2 份新增、2 份仅注释），最大 376 有效行；新文件 89/65 有效行。
UTF-8 无 BOM、增量空白、fmt、版本及组织合同 3 项通过，3939 文件行检查无超限。
最终 input-3 同源测试程序已编译/封存；EXE SHA-256：
`4fab4ad0e400e0af99633d2677bb0f2740119d0bee4124dbac9e8cb7499d075c`。
render_surface 2/0/0、canvas 10/0/0、D3D12 7/0/0、composition 109/0/4；Canvas 新场景断言实际
D3D12 分支且在 DPI/resize/重建设备后继续提交，文档凭证不变。4 项既有交互门未执行，不计通过。
扩展回归 projection_present 5/0/0、隐藏合成 TLS 子节点链 1/0/0、render_frame 2/0/0；
TLS 链不代替 Loom 真实双端或 Shader 验收。Clippy all-targets -D warnings 通过。
全部 Cargo 构建完成后封存产品，EXE 45330432 字节，SHA-256
`af44b08af59f25c80db9fb803302568796caf2935b0c656e9b713d16fc947970`；匹配 PDB 294244352 字节，
SHA-256 `2659f70f8062239de228584662accb1d6f29af9663dc98b64c89984f129918df`。
隔离隐藏 --workspace-check exit 0，2.109 秒；日志两次 selected=d3d12，且 paints=2、device-recreated=true，
没有 rejected=d3d12 或兼容链选择记录，证明本次产品启动/重绘实际走 D3D12，而非回退后启动成功。
集成 build_resources 2/0/0、exe_resources 1/0/0、startup 7/0/0（39.694 秒）、session_startup 9/0/0（9.912 秒）。
集成前后核同字节，仍保原 child 15 秒门；这些耗时不是对照性能基准，不能宣称性能改善或历史超时已修复。
真实屏幕观感、跨硬件覆盖、手动选择、运行中保文档恢复和完整迁移验收仍未完成。

## v417：D3D12 多窗口共享设备

内部版本 v0.2.33.417；同一 UI 线程的候选共用 D3D12 device/direct queue、11On12 device/context、
D2D device/fence/event。窗口独占 D2D context、双缓冲交换链和合成树，文档仍由原业务 owner 持有。
Weak 缓存不延长最后一个正常窗口租约；MAX_UNITS=128 个在途/保留窗口图和最多两代设备分别计额。
资源排空失败毒化共享代际，旧窗口不能继续绘制/呈现/改变透明度，acquire 不再借出旧代；
健康设备未确认关闭时仍完整保留窗口图和租约，不强制移除设备、不无限重试或启动后台等待线程。
帧准入禁止嵌套 D2D/11On12 绘制，包装资源释放和 Flush 完成后才归还帧租约。
这不等于总显存预算/性能已验收，也没有默认后端切换或应用级自动重建/降级。

证据根：`linshi/otter-native-v417-d3d12-shared/`。input-2 冻结 2785 项，源码 SHA-256：
`c9872e1abaf467681b323ac146d556eb95aa0cb8608ad7d49e5cf453ce7d861e`。
同源 lib EXE SHA-256：`70a45ea54b1947d9338fe8c1465a30f58ae33fd0275cb67534a521e54cf30052`。
D3D12 7/0/0（真实四窗口共享/独立/关闭中间窗口后继续绘制、最后租约释放、帧准入与显式毒化拒绝等），
render_frame 2/0/0、canvas 9/0/0、composition 109/0/4；4 项既有真实捕获/焦点交互门未执行，不计通过。
毒化验证为无 GPU 在途的显式状态注入，不是实际 GPU hang/device removal/WAIT_FAILED。
Clippy all-targets -D warnings、fmt、版本、组织合同 3 项及 3937 文件行检查通过。
五份源码改动、无新增：有效行分别 243/100/177/48/417；UTF-8 无 BOM 与增量空白检查通过。
全部 Cargo 完成后封存产品：EXE 45250560 字节，SHA-256
`3f8e323e207af633c96a1eac8f3d0f6a47a3e6125ab286dca851cc95ba4bcd8c`；匹配 PDB 294105088 字节，
SHA-256 `0f4b0a3d52f147ec46523fa9e90ee28be8ea198bed9fd813357de6b1e23038c2`。
隔离隐藏 --workspace-check exit 0，0.622 秒；集成 build_resources 2/0/0、exe_resources 1/0/0、
startup 7/0/0（20.964 秒）、session_startup 9/0/0（4.476 秒）。集成前后产品同字节，
保留原 child 15 秒门；历史启动超时根因仍开放。默认产品通过不表示 D3D12 已接入 Canvas。

## v416：D3D12 候选视觉透明度

内部版本 v0.2.33.416。composition_d3d12 增加同设备 resource_target 和 set_opacity；
后者用 DComp effect 控制整个视觉，不缩放 backbuffer alpha 或修改文档/导出像素。
有限 [0,1] 输入在 API 调用前校验，相同值不重复提交；提交链错误使候选失效且清除待呈现帧，
不把旧已提交数值当作 DComp 待提交状态已经回滚。无新依赖、线程、系统配置或默认后端切换。
新增真实隐藏窗口/GPU 测试覆盖透明度 0/0.5/1、resize、纹理像素不变及非法值/失效拒绝；
纹理读回不能证明最终屏幕合成观感，未注入真实 DComp Commit 错误。

证据根：`linshi/otter-native-v416-d3d12-opacity/`。首个 metadata 使用 --no-deps 导致冻结器
拒绝不完整依赖解析，保留该失败，不放宽校验；改用完整 locked metadata 后冻结 input-2，2785 项。
源码 SHA-256：`293714d04df3370dbcd5721cdaf933438d13540fd25e502b4731f969419747c3`。
有效行检查 3937 文件无超限；增量两份源码分别 203→241、280→321 行，UTF-8 无 BOM/空白检查通过。
fmt、版本检查、组织合同 3 项及 Clippy all-targets -D warnings 通过。lib 测试程序编译并同源封存，
D3D12 候选 5/0/0、render_frame 2/0/0、canvas 9/0/0。run-1 测试 EXE SHA-256：
`ceaa10c200fd2b84e7459cc8d6ef1261fd04fcf7eaabf1cf6cc5a408d3fc1d16`。
全部 Cargo 构建结束后封存 product-final，EXE 45250560 字节，SHA-256
`2ed848df4bcef6475f4ba5e288cbd52f47f599fbbc81d52f30f95e7e44f16671`；匹配 PDB 294088704 字节，
SHA-256 `37b95468b98403544c8561f6dacbca24ec3cba7b1cc516923cdba7a2ff338271`。
隔离隐藏 --workspace-check exit 0，用时 0.571 秒；build_resources 2/0/0、exe_resources 1/0/0、
startup 7/0/0（18.594 秒）、session_startup 9/0/0（4.302 秒）。集成前后核产品同字节，
每个 startup child 仍保留 15 秒门。当前成功不关闭 v415 或更早间歇超时根因，也不代表候选已默认接管产品。
共享设备 owner、默认/手动选择和设备恢复仍未完成。

v415 启动失败的后续诊断另存原证据目录：单场景 10.72 秒，4 次阶段采样 8.762/11.688/9.253/8.710 秒；
原顺序完整 startup 开启计时为 7/0/0，总计 26.96 秒。均是原 v415 程序，不能继承为 v416 成绩，
也不覆盖原 6/1/0 失败。原失败未复现且根因未定，停止无界重跑，后续同源集成仍保留原 15 秒门。

## v415：Canvas 帧事务统一

内部版本 v0.2.33.415；本地/外部、完整/缩略 Canvas 绘制统一经过 RenderSurface.draw，
由 render_frame 唯一负责 BeginDraw/EndDraw。绘制失败也结束帧，同时失败时保留绘制错误；
clip/transform 栈上 guard 覆盖错误和 panic 清理。资源同步仍在帧前，缓存和计数语义保留。
只有成功帧才显式提交钉图；旧 HWND target 在 EndDraw 内部呈现，不能保证回滚已绘像素。
默认仍为 D3D11/WARP，本切片没有启用 D3D12、增加线程或整图像素副本。

input-1 冻结 2785 项，源码 SHA-256：`e0863d292ae463ab6f095b4867531160edf3f45923518ada7661e4d4b377e667`。
v414 封存程序与同源 v415 程序前后对照：canvas 9/0/0、projection_present 5/0/0、
committed_image_automatically_executes_child_through_sticker_over_tls 1/0/0。
新帧事务 WIC/D2D 像素测试 2/0/0，composition 109/0/4，D3D12 候选 4/0/0。
四项既有捕获/焦点交互门未执行，不计通过；隐藏 TLS 链不是 Loom 真实双端，real_shader=false。
Clippy all-targets -D warnings、fmt、版本、组织合同 3 项与 3937 文件行检查通过。
源码增量 7 份（新增 2 份），UTF-8 无 BOM 和增量空白检查通过；新增帧模块/测试为 72/141 有效行。

证据根：`linshi/otter-native-v415-frame-transaction/`。全部 Cargo 构建完成后封存 product-final；
EXE 45250560 字节，SHA-256 `24c9982a2bbd0b94ab2e033f39f44ad9bd7b82513fca417fc152d39076101ded`；
匹配 PDB 294064128 字节，SHA-256 `834fff910e203cc1bf0e4f99a3b8207cc1cce1e8cef807d017aa0f4c0621b207`。
隔离隐藏 --workspace-check exit 0，用时 0.999 秒。
集成 build_resources 2/0/0、exe_resources 1/0/0、session_startup 9/0/0；
startup 为 **6/1/0，exit 101**：transparent_pin_creates_compositor_and_rebuilds_device_resources
子进程触发原 15 秒门，实测 elapsed=15.0487178s。原始 stdout/stderr 与失败 receipt 全部保留，
未放宽时限、未重跑覆盖失败。超时前最后标记为 native-restored-job-smoke；这不足以确定阻塞位置或根因。
当前不能宣布集成全绿，也不能断言与 v410/v411 同源或与本次修改无关。

D3D12 共享设备 owner、透明度、默认/手动选择、运行中恢复、原生 Shader、CPU 全底线及 Art 真实接入
仍待完成。未修改真实 profile、系统配置或 Loom runtime；未发布或部署，完整 N01–N12 验收仍开放。

## v414：D3D12 窗口候选与共享 D2D 绘制

内部版本 v0.2.33.414；新增真实 D3D12 device/direct queue/composition swapchain 候选，
当前没有接管默认窗口或改变 D3D11/WARP 选择顺序。已锁定 windows 0.60.0 只增加
Direct3D12/Direct3D11on12 API feature，Cargo.lock 不变，无系统安装或第三方 DLL 分发。
图片、文字与矢量使用同一 D3D12 队列上的 D3D11On12/D2D 适配，不是第二套独立呈现链。
尚无原生 D3D12 Shader 绘制；不能称为无适配、完整双后端或 ANGLE 替代完成。

官方文档于 2026-10-28 联网获取并保留在证据目录：
[D2D using D3D11On12](https://learn.microsoft.com/en-us/windows/win32/direct3d12/d2d-using-d3d11on12)、
[Direct3D 11 on 12](https://learn.microsoft.com/en-us/windows/win32/direct3d12/direct3d-11-on-12)。
当前无先行原生 D3D12 命令，包装资源输入/输出均为 PRESENT；Acquire → D2D EndDraw →
Release → Flush 后才允许 Present。每帧选择实际 backbuffer，失败/异常绘制不提交旧帧；
固定双缓冲，生产路径无整图 CPU 读回，每帧无 CPU fence 等待。未测整体性能收益。

静态复核发现初版在 fence 未确认完成时仍析构资源的风险，已在最终输入中修正：完整 State
含 COM/event/配额，以 ManuallyDrop 保持；健康设备的任意 drain 错误不继续析构，不归还配额。
候选期最多两个在途/保留 owner，保留至进程退出，不创建后台无限等待线程，也不 RemoveDevice。
这是明确受限的候选保护，不是最终多窗口共享/设备恢复；不能把 5 秒等待期限当作所有驱动析构上限。
实际 GPU hang/device removal/WAIT_FAILED 未注入；配额测试只是模型，不能冒充真实故障验证。
测试读回的 Acquire 与 Map 也分别使用异常安全 guard。底层失败后的 candidate 仍须重建，
resize 不作为清除 failed 标记的恢复承诺。

input-1 冻结 2783 项，源码 SHA-256：`73afb0bbb78e2afc6bcbd87dbd76e5728e8f8748adf3ec4a055f90fb80680793`。
同源 lib：D3D12 候选 4/0/0（3 个真实 HWND/GPU 场景、1 个配额模型），原 composition 109/0/4，
canvas 9/0/0；四个既有交互门忽略，未计通过。D3D12 场景验证交换链返回的实际 D3D12 device、
两个 backbuffer、透明/预乘像素、图片、文字、连续 resize、异常绘制清理及非法尺寸拒绝。
窗口为隐藏自有 HWND，不是最终屏幕观感或完整业务操作验收。
Clippy all-targets -D warnings、fmt、版本、组织合同 3 项和依赖安全合同通过；
3935 文件有效行检查无超限。native 增量 7 文件（含 Cargo.toml），5 新增；新文件最大 280 有效行，
原 mod.rs 445→446，仅接线一行。增量 UTF-8 无 BOM 和空白检查通过。
审计脚本首轮未支持 TOML，按失败留存；随后仅对确切两项 API feature 变化设专门核验，未放宽 Rust 行门。

证据根：`linshi/otter-native-v414-d3d12-present/`。全部 Cargo 构建完成后封存 product-final；
EXE 为 45242368 字节，SHA-256 `bd086f4d65039bbb75667ea136458739f1373e71f13f05615f4bfb3f66401f5f`；
匹配 PDB 为 293924864 字节，SHA-256 `e2d197b61f4127b8eb35fb39b34617b99d2114614c5c53ffd1787923ef676356`。
隔离隐藏 --workspace-check 为 exit 0，用时 0.898 秒；集成 build_resources 2/0/0、exe_resources 1/0/0、
startup 7/0/0、session_startup 9/0/0，集成前后产品同字节；未放宽原 child 15 秒期限。
默认产品的集成通过不等于 D3D12 已接入，也不关闭历史启动间歇超时或 v398 呈现根因。
默认 Canvas 帧事务、透明度、共享设备 owner、手动选择、原生 Shader 与运行中恢复均未完成；
无真实 profile、Loom runtime、系统配置、发布或部署变更，完整 N01–N12 和最终整包验收仍开放。

## v413：完整呈现候选选择

内部版本 v0.2.33.413；按统一修订计划进入 P1。默认窗口不再只在 D3D11 设备创建失败时
回退：HWND/DXGI/D2D/DirectComposition/bind 整条候选建立失败也允许尝试 WARP。
每次创建事务硬件/WARP 各最多一次，主候选成功不启动第二套；失败原因保留后端、阶段与 HRESULT。
两类设备各用 UI 线程弱缓存，最后租约结束后不常驻；显式 Live 同设备路径不自动换设备。
没有新增依赖、后台重试循环、额外像素副本或文档 owner。D3D12、运行中设备代际恢复、
应用级统一选择与手动选择、CPU 核心全功能底线仍未完成；WARP 不等于脱离 Windows 合成设施。

input-1 冻结 2778 项，源码 SHA-256：`f102301346f1d388b6e9fda0199cbee01e1997049203e3f92fe273777456cf07`。
同源封存 lib 定向验证：composition 109/0/4、export 115/0/0、save_snapshot 23/0/0、
session_candidate 59/0/0、application projection_sender 28/0/0（通过/失败/忽略）。
四项忽略为既有真实 WGC/窗口焦点与鼠标捕获交互门，本轮未执行，不计通过。
主候选链失败使用故障注入，后继 WARP 设备、窗口合成链、预乘 alpha 读回与 64/96/32 resize 为真实执行；
不将注入当真实驱动故障，不将测试候选路径当运行中自动恢复。清理顺序模型另有 Drop 断言保护。
初次过滤名 application_projection_sender 匹配 0 项，包装门按失败处理并保留日志；
查明实际 Rust 模块名后运行上述 28 项，无源码、断言、超时或产品行为修改，不把零匹配计为通过。

Clippy all-targets -D warnings、fmt、版本与组织合同 3 项通过；3930 文件行检查无超限。
相对 v412 的 native 增量为 6 份变化（含 2 份新增），最大 221 有效行；UTF-8 无 BOM、
增量 untracked 空白与 tracked git diff --check 通过。证据根：`linshi/otter-native-v413-presentation-selection/`。
全部 Cargo 构建完成后封存 product-final：EXE 45243392 字节，
SHA-256 `61d1416e84568f77f1087bc98e54448f098d0c2452493ef915c821800b939e4a`；
匹配 PDB 292433920 字节，SHA-256 `13ae3a6ecb41c4716d1ee65d397d5d05741c536d1c66190f7d2b7f8ec5c65d30`。
隔离隐藏 --workspace-check 为 exit 0，0.626 秒；集成 build_resources 2/0/0、exe_resources 1/0/0、
startup 7/0/0、session_startup 9/0/0，集成前后产品同字节，原 child 15 秒门限未调整。
本轮通过不关闭 v410/v411 startup 超时或 v398 间歇呈现根因；不是全量 lib、完整包或最终用户验收。
Loom runtime 未改，ANGLE 未退役，完整 N01–N12 继续开放；无 commit/push/release/deploy。

## v412：无 raster 目标实时外框

内部版本 v0.2.33.412；public version 仍为 0.2.33，不是正式 release 或最终用户整包。
本轮先按用户新决策更新 [统一修订计划](WINDOWS_NATIVE_REVISED_PLAN.md)，并在独立 Loom 协议入口登记迁移边界；
Loom 本轮只有文档修改，没有变更当前 hook/surface v1 wire 行为，也未实现新源码 Art 分发/执行合同。
随后收尾暂停中的几何修复，不继续扩建 ANGLE 长期路线，不把该修复称为双后端实现。

仅当目标 editor 无 full logical frame 且无 raster layers 时，捕获实时 HWND 物理框和原 session Space；
后台仍在原发送 Job 中按 canonical x/y/w/h 提示保留小数余量，不能按显示器 DPI 猜比例。
minified Sticker 使用当前有效 presentation 的 annotations expanded size；Art 忽略该 Sticker 分支。
原 full source/raster/crop 优先级不变，避免拿可见 crop 窗口再裁一次。原 Document 凭证与完整 Space
共同过滤 resize/minify/viewport/会话坐标变化，已过期 mini presentation 捕获时即拒绝。
不新增 renderer、网络、持久化 owner 或像素副本；原 PNG 限额、共享预算及取消围栏保持不变。

修复前 input-regression：四项专门回归 0 过/4 失败，三个真实 TLS PNG 整图不符，另一个遗漏 HWND resize 失效。
input-1 修复后：5 项中 2 过/3 失败；普通未保存 resize 和 stale mini 拒绝通过，13 项 PNG 门通过。
其余失败来自夹具：mini 缺少保存必需的 Transition，Art 未同步 canonical type，resize 返回同框后误认为旧 viewport 凭证复活。
input-1→2 只修改 application_projection_sender_geometry_tests，补合法元数据/transition 并捕获新凭证；
未改生产逻辑、PNG oracle、超时、保存断言或错误处理，旧输入/程序/PDB/失败日志全部保留。

最终 input-2 冻结 2776 项，源码 SHA-256：`12dc45e666de8ad296f2bb53a33573e81737d22fc0728f976b1a4339be93ac7f`。
同一封存 lib 程序：26 组选定门全部通过，过滤组重叠不累计独立总数；sender 86/0/0、模型 sender 53/0/0、
export 104/0/0、projection PNG 13/0/0、workflow 77/0/0、save_snapshot 23/0/0、session candidate 55/0/0。
新几何专门组为 5/0/0，已包含在 sender 中；三个菜单/TLS 用例核实际 PNG/digest、canonical checkpoint 与重启，
仍是隐藏合成 profile/loopback 测试，不是真实 Loom 双端、完整图拓扑或用户全面手测。
Clippy all-targets -D warnings、fmt、版本、组织合同与 scoped diff 通过；3928 文件行检查无违规/警告，
native 相对 v411 为 11 份变化/3 新增，最大 274 有效行，文本 UTF-8 无 BOM。
证据根：`linshi/otter-native-v412-live-target-frame/`，最终产品在全部 Cargo 构建完成后封存于 product-final。
EXE 为 45234688 字节，SHA-256：`d9115724cb81cac7174147324686fecd469951465462788b8adbaf5770f2cd77`；
匹配 PDB 为 292335616 字节，SHA-256：`7b7817539e1d286748d7646bb1718419377654ef7728ea2ff5e6566482bc2c5f`。
隔离隐藏 --workspace-check 为 exit 0，用时 1.346 秒；正式集成 build_resources 2/0/0、exe_resources 1/0/0、
startup 7/0/0、session_startup 9/0/0，原 child 15 秒期限不变；集成前后 target 与封存 EXE/PDB 同字节。
这些成绩只归属于本轮 input-2 和封存产物，不继承或替换 v411 失败；不是全量 lib、完整 34 门或正式整包。

本轮未修 startup 根因，v410/v411 超时及 v398 间歇呈现继续开放；不放宽原 15 秒门限或自动重试刷绿。
完整 PNG parity、持续发送/设备组、真实双端、双向编辑/LiveRelay/Tile、资源长稳及 N01–N12 仍未完成。
无真实 profile、系统安装、注册表/证书变更、默认 DLL 启用、commit/push/publish 或部署；下方 v411 为历史。

## v411：稳定发送关联显式停止

内部版本 v0.2.33.411，public version 仍为 0.2.33；dirty 开发迭代，不是正式 release 或最终 portable 整包。
已接稳定单目标显式停止的原 Store/Job/canonical 事务；最终同源定向 lib 门已通过，但 startup 正式集成仍有一项超时。
首次 compile-1 失败为新增兄弟模块访问 Request 私有字段与未标注 HWND 检查的 unsafe 边界；已改用现有只读 accessor 并保持 unsafe 调用边界，原日志保留。
中间 input-1 编译期间继续修正审计发现的 deadline/Ack 结算并补测试，其输出只作诊断，不是最终输入成绩。
input-2/run-1 新模型组 2 过/1 失败，隐藏停止组 0 过/4 失败；两处测试前提错误已按现有合同修正：
保留的 primary receiver 仍须拒绝发送，不能因另一个 sender 被移除就授源角色；Record metadata 已编辑后，
projection_sender_inclusion 必须失效，因此“旧 canonical 尚未改”应核精确 canonical Source/Index，而不是借当前 Record 取 inclusion。
input-2→3 仅这两份测试文件变化，不改生产逻辑、期限、网络或断言阈值，失败输入/程序/原始流保留并重新编译。
input-3 严格 Clippy 发现 StopSelection 使共享 Request enum 扩至至少 488 字节；input-4 将该变体装箱，
保留固定 Binding/Source 身份，没有添加 allow 或降低 lint。最终门按修正输入重建，不继承旧源测试成绩。
最终 input-4 冻结 2773 项，源码 SHA-256：`73470f77dbbb5ac652b19947d1d78a5394733d4eb2a91c9cb57e62c6cb4662c9`。
同源 run-2 的 26 组选定 lib 门全通过，过滤组重叠，不累计独立测试总数：广义 sender 81/0/0、
模型/HTTP sender 53/0/0、export 100/0/0、projection PNG 9/0/0、workflow 77/0/0；
保存快照 23/0/0、session candidate 55/0/0、Live checkpoint 20/0/0，其余 scope/绑定/原文/档案/崩溃/几何/目录门逐项封存。
另用同一程序显式跑新 stop 模型 3/0/0、隐藏菜单/TLS 4/0/0，已包含在上述 sender 中，不再累计。
四个菜单用例覆盖正常停止、持久 unlink 回执后重启、最终 canonical 移除后 ack 前重启、Begin 基线失败/明确重试；
同时核无目录目标、等字节不同 Source 失效、pending 候选不授网络、内存移除不授 ack、原设备身份/envelope 不变及同目标重发准入。
这是隔离合成 profile/loopback TLS 的重启窗口验证，不是真实双端、OS 强杀全崩溃注入或最后全面手测。
严格 Clippy all-targets -D warnings、官方 fmt、版本/组织合同及 scoped untracked diff 检查通过；
3925 文件 ratchet 无违规/警告/新例外，相对 v410 native 21 份变化/6 新增，最大 409 有效行，新增最大 215。
最终产品在所有 Cargo 构建结束后封存于 `linshi/otter-native-v411-sender-stop/product-final/`，集成前后核 target 与封存 EXE/PDB 同字节。
EXE 为 45228032 字节，SHA-256：`eef0d5a9ea2f5cbb48321af9a524a44ba19cb0bc86d8b9474e9c2aab81788e81`；
匹配 PDB 为 292237312 字节，SHA-256：`478b37f2f7df308e7216c180f1c1d2e436677cd9dc567bf082d0df86c803d8e4`。
同一封存 EXE 的隐藏离线 `--workspace-check` 在隔离合成 profile 下 exit 0，用时 0.889 秒，不是真实 profile 或最终用户整包验收。

正式集成：build_resources 2/0/0、exe_resources 1/0/0、session_startup 9/0/0；startup 为 6/1/0、exit 101。
失败是 `transparent_pin_creates_compositor_and_rebuilds_device_resources`，child 超过原 15 秒期限，
超时回执 elapsed=15.606166s，最后完整输出为 native-flip-smoke；原始测试 EXE/PDB、stdout/stderr 和回执均保留。
未因失败自动重试、扩大期限、降低断言或删场景；不同历史运行停点不同，不能据此认定 flip、面板或机器负载为根因。
该功能迭代未新增启动计时/栈采样，也没有启动根因修复；后续只读补充诊断见下节，v410 正式失败和诊断证据、v398 间歇呈现仍开放。

Begin 基线失败后的显式重试已运行验证；Ack 已发布但保留恢复副本的结算分支仅静态复核，没有完整文件故障注入成绩。
最终 scoped 进程核验为本轮及共享 target 自有原生进程 0，保留用户旧 Otter 实例；最终文档与源码/产物证据分别绑定。
本轮 allRequiredSelectedLibraryGatesPassed=true、allRequiredSelectedGatesPassed=false，失败门为 startup；不是全量 lib 或完整 34 门成绩。
目标仍 active、objectiveComplete=false、publishable=false；完整迁移、真实双端、资源长稳与最终全面手测未完成。
无新依赖、真实 profile、系统安装、注册表/证书变更、默认 DLL 或 commit/push/publish。下方 v410 为历史。

### v411 启动补充诊断：离线展开与单次 pin 计时

未修改生产源码、测试源码、构建输入或内部版本，也没有重编译；v411 正式 startup 6/1/0 及 v410 失败回执原样保留。
证据在 `linshi/otter-native-startup-unwind-v411/`，这是诊断，不是新的功能迭代或启动性能验收。

对既有 v410 自有 child 的两个 MiniDumpNormal，先按 PE timestamp、SizeOfImage、checksum 核对本地映像，
原产品路径明确替换为已封存 v410 EXE；使用 PE `.pdata` 的函数表与模块基址回调，不再扫描栈上的数值猜返回地址。
两份主线程分别展开 31/47 帧，均到 main、BaseThreadInitThunk、RtlUserThreadStart，无未读取地址；
第一份定位到回收关闭的 poll_recycle_using→DestroyWindow，第二份为恢复发布测试退出时，
Application/DocumentWindow/Window 析构进入 DestroyWindow 后的系统回调链，不是控件创建链。
系统 DLL 只具本地导出符号时的最近符号名可能不精确；未联网取符号。展开不是内核等待原因证明，
也不将成功运行时的这两份样本说成正式超时现场，completeUnwindVerified 仍保守为 false。

另以同字节 v411 target 产品、相同 image/--pin/--smoke-test 参数，单次开启已有 OTTER_NATIVE_SMOKE_TIMING。
诊断外层期限仍为 15 秒且包含 RTK 包装开销；原 startup test executable 没有重跑，不能替代原 runner 成绩。
本次 exit 0、9.077 秒，747 个 begin/end 全配对；Application 为 5225ms，restore-batch 为 1638ms，
flip 为 165ms、crop-effect 为 156ms。此结果只能否定“该输入每次必然卡死/必然超时”，不能关闭间歇超时。
无自动重试、期限放宽、场景删除或阈值下降；源码和封存/target EXE/PDB 前后同字节。

只读审查另确认 flip/crop-effect 的消息排空内层没有单独期限/条数上限，且多个阶段的局部 10 秒预算串行叠加；
但没有捕获消息洪泛、长回调或固定超时的证据，不据此修改产品行为。后续需要慢运行临近超时的精确线程与阶段现场，
而不是继续用成功样本推断根因；startupTimeoutRootCauseIdentified/Fixed 仍为 false，完整目标仍 active。

## v410：投送目标编辑与外观归属

内部版本 v0.2.33.410，public version 仍为 0.2.33；这是 dirty 开发迭代，不是正式 release 或最终用户整包。
resolved 底图与目标 editor 已分离；目标边框、全栈圆角与 beautify 从 canonical 原文在原后台发送 Job 中读取。
当前 crop/reset 仍属于活动目标 editor，不从持久旧 crop 复活。无 logical frame/raster 的目标读取 canonical 展开逻辑框回退。
完整历史 erase/flip、传播内容框、跨 Unit 自然图源、活动 blur、未保存无 raster 窗口缩放及完整 PNG parity 仍开放。
实现与归属边界见 [v410 迁移记录](WINDOWS_NATIVE_MIGRATION.md#v410投送目标编辑与外观归属)。

最终 input-3 冻结 2767 项构建输入，源码 SHA-256：
`c04856f23291f6a063a359b6b4984baeb17cc661d5d239da0a76536dce0e10f9`。
产品 EXE 为 45142016 字节，SHA-256：`538e4101e3c5c5684421b14926123caafdbe39d40c2f8f23f09005261e4c6631`；
匹配 PDB 为 291786752 字节，SHA-256：`6a5a0c62424dd038191a0a018caaf82aa5448b5ad0a0a7e389829e0d9629f5fa`。
产物位于 `linshi/otter-native-v410-target-projection/product-final/`；所有 Cargo 构建结束后才封存，
集成 child 使用编译时固定 target 产品，集成和诊断前后核与封存 EXE/PDB 同字节。
封存产品隐藏离线 --workspace-check 为 exit 0，使用显式隔离合成 profile，不是真实 profile 或真实 Loom 双端验证。

同源 run-3 的 26 组选定 lib 门全部通过，过滤组重叠，不累计为独立测试总数。
广义 sender 74/0/0、模型/HTTP sender 50/0/0、export 100/0/0、projection PNG 9/0/0、workflow 77/0/0；
保存快照 23/0/0、session candidate 55/0/0、Live checkpoint 20/0/0，scope、sender/receiver checkpoint、binding、
runtime、原文/保存/档案/崩溃/几何/目标目录等其余选定门均逐项封存。
独立 source 专门组 7/0/0、target 专门组 2/0/0，也使用同一封存 EXE；这些用例已包含在广义 sender 中，不再累计总数。
新增两项隐藏菜单/TLS 用例注入带冲突上游 editor 的 resolved frame，核目标 live crop、raster、边框、3px 输出像素 padding，
整图精确 PNG 像素、实际尺寸、envelope digest、canonical checkpoint 与重启 binding；不是实际图拓扑或真实 Loom 双端验证。
五项直接后台读取/合成回归另覆盖未知大字段跳过、重复/坏值/容量、圆角全栈、边框不随底图淡化、最终尺寸与无 raster 目标逻辑框。

失败证据均保留：run-1 的两个新 TLS 夹具从 Record.original 查尚未进入的新 Unit，发送前 unwrap 失败；改取已提交 canonical 基线。
input-2/run-2 的广义 sender 为 71 过/3 失败：旧 guard 隐含上游逻辑框，调整目标 fixture 宽高比后保留原全部 opacity/失效断言；
两个新 oracle 则误按底图 source crop 处理 raster layer。TypeScript 主干实际把完整 raster layer 绘到可见框，
对原始失败 PNG 独立解码的两图核验按该合同均零像素差异，故只修 oracle，未改生产 renderer 或放宽像素/期限断言。
input-2→3 仅两份测试文件变化；新输入重新编译并重跑全部选定门，没有沿用旧输入的绿成绩，无新 allow/ignore。

正式集成门并未全部通过：build_resources 2/0/0、exe_resources 1/0/0、session_startup 9/0/0；
startup 为 5/2/0，两项 transparent_pin 与 wic_image child 超过原 15 秒期限，分别停在 copy smoke 之后和 recycle prepare 之后。
原始 `integration-final/` 四项回执、测试程序/PDB、stdout/stderr 保留，不以 lib 通过替代启动集成。
同一封存程序仅另开现有 OTTER_NATIVE_SMOKE_TIMING 诊断，仍保留原 15 秒期限；结果为 6/1/0，
这次失败为 hidden_window。737 个 begin 中 736 个完成，Application 完成用时 10599ms，
tool-panel-smoke 从计时起点 12.210154s 开始，直到 child 总计约 15.047s 被回收仍未返回。
不同运行停点不同，尚不足以认定面板死锁、机器负载或其他根因；没有源码修复、自动重试或门限放宽。
`startup-diagnostic-1/` 的原始流及 analysis.json 只作诊断，不替换正式失败；startupTimeoutRootCauseIdentified/Fixed 均为 false。

严格全目标 Clippy -D warnings、官方 fmt、版本/组织合同和 scoped diff/空白检查通过；
3919 文件 ratchet 零违规/警告/新例外；相对 v409 native 9 变化/3 新增，最大 210 有效行，文本 UTF-8 无 BOM。
证据根为 `linshi/otter-native-v410-target-projection/`，最终产物及集成身份由同目录封存回执绑定。
本轮结算只证明证据一致性与定向 lib 成绩，allRequiredSelectedLibraryGatesPassed=true、allRequiredSelectedGatesPassed=false；
startup 集成门仍未通过。最终 owned 核验只计本轮/共享构建路径，保留用户正在运行的旧 Otter 实例。
未跑全量 lib/完整 34 门，不关闭 v398 间歇呈现及历史前台/启动根因；完整迁移、资源长稳和最终用户全面手测仍开放。
目标 active、objectiveComplete=false、publishable=false；无新依赖、真实 profile、系统安装、注册表/证书变更、默认 DLL 或 commit/push/publish。
下方 v409 为历史。

## v409：本地保留原图生产捕获

内部版本 v0.2.33.409，public version 仍为 0.2.33；这是 dirty 开发迭代，不是正式 release 或最终用户整包。
本轮只接本地 retained natural source 的生产捕获，完整目标 metadata/上游传播/历史编辑/beautify parity 仍开放。
实现与边界见 [v409 迁移记录](WINDOWS_NATIVE_MIGRATION.md#v409本地保留原图生产捕获)。

最终 input-2 冻结 2764 项构建输入，源码 SHA-256：
`60d96301b6e64edf10153bf4c70f7c4078d52bfd698d675bc00ff33b03f7aced`。
产品 EXE 为 45104128 字节，SHA-256：`8334b302245d9c3f506d95ca0bb145f60f541fcda88a3669f6c6662cb752bc91`；
匹配 PDB 为 291622912 字节，SHA-256：`162ba3fdf0d39508dd379c6d3fa0da906ccdea9467a0760fdea126d29c99a4ae`。
产物位于 `linshi/otter-native-v409-retained-projection/product-final/`，不是最终 portable 整包或正式 release。

同源 run-2 的 26 组选定 lib 门全部通过，过滤组重叠，不累计为独立测试总数。
广义 sender 72/0/0、模型/HTTP sender 50/0/0、export 95/0/0、projection PNG 4/0/0、workflow 77/0/0；
新增 source 专门过滤组 7/0/0，已包含在广义 sender 中。其中四项走真实菜单→TLS read/Create→canonical checkpoint→重启 binding，
核 natural/内容擦除/undo/crop 的传输 PNG 尺寸、解码全图精确像素与 envelope digest；另三项直接验证 Scope 弱身份、
resolved 不借目标 retained、当前 raster/vector annotations 与 normal opacity 的独立合成。
原保存快照 23/0/0、session candidate 55/0/0、Live checkpoint 20/0/0，以及 scope、sender/receiver checkpoint、
binding、runtime、原文/档案/崩溃/几何/目录等选定门均通过，逐项回执封存。

四项集成 build_resources 2/0/0、exe_resources 1/0/0、startup 7/0/0、session_startup 9/0/0。
所有 Cargo 构建完成后才封存产品；集成 child 使用编译时 target 产品，前后核与封存 EXE/PDB 同字节。
封存产品隐藏离线 --workspace-check 通过，显式使用隔离合成 profile，不是真实用户 profile 或真实 Loom 双端验证。
严格全目标 Clippy -D warnings、官方 fmt、版本/组织合同、文档与 scoped diff/空白检查通过；
3916 文件 ratchet 零违规/警告/新例外。相对 v408 native 8 变化/2 新增，最大 319 有效行，文本 UTF-8 无 BOM。

首轮 input-1/source 专门组为 4 过/3 失败：默认 Editor 无 full_logical_frame 时误以 natural 像素作逻辑框，
造成 crop 宽度 16 而非 64、vector 标注位置偏移。生产回退改为 editor.layers.size，新增默认逻辑框直接回归；
不是改小预期尺寸。擦除用例另有 512 像素中 8 个绿色通道的 207→206 差异，alpha 不变，来自 WIC PBGRA/BGRA 往返舍入。
TLS oracle 改为独立 raw WIC 编码/解码预期图，不调用待测选图/布局/D2D 合成，仍整图逐字节比较，未增加像素容差。
原失败输入、程序/PDB、原始 stdout/stderr/日志保留；新输入重建并重跑全部选定门，未放宽期限或新增 allow/ignore。

证据根为 `linshi/otter-native-v409-retained-projection/`。完整目标 metadata/上游传播、历史编辑、beautify、
设备组/持续发送/稳定 binding 显式停止、真实双端/离线互信、双向编辑/LiveRelay/Tile、资源长稳和最终全面手测仍开放。
未跑全量 lib/完整 34 门，不关闭 v398 间歇呈现及历史前台/启动根因。目标 active、objectiveComplete=false、publishable=false。
无新依赖、真实 profile、前台/全局输入、系统安装、注册表/证书变更、默认 DLL 启用或 commit/push/publish。下方 v408 为历史。

## v408：有界源分辨率编码出口

内部版本 v0.2.33.408，public version 仍为 0.2.33；不是正式 release 或最终用户整包。
本轮范围为 source-resolution 发送专用准入/编码出口及当前底图接线，不是 retained-source/目标 metadata/
完整 beautify 生产 parity。实现与未完边界见 [v408 迁移记录](WINDOWS_NATIVE_MIGRATION.md#v408有界源分辨率编码出口)。

最终 input-4 冻结 2762 项构建输入，源码 SHA-256：
`a3731281b19a1125973fc0c82ac5f9af6ed6451f4d94e9dbb630351874b94a44`。
产品 EXE 为 45099008 字节，SHA-256：`d6e8fb0c9b8860e7536adeced079f92728da737ddc2604446fa2e7a97df050d3`；
匹配 PDB 为 291459072 字节，SHA-256：`beda1a8d6a3d84fe8d3a0b69593aff3275bd7b081a1f3d303f57684e3af4e948`。
产物位于 `linshi/otter-native-v408-source-encoding/product-final/`；是 dirty 开发产品，不是正式 release 或最终 portable 整包。

同源 run-3 选定 26 组 lib 门全部通过；过滤组重叠，不能相加为独立测试总数。
完整 export 过滤组 94/0/0、workflow 77/0/0、广义 sender 65/0/0（模型/HTTP sender 50/0/0）、投送 PNG 3/0/0，
包含自然像素/逻辑标注、源/内框/外框限额、真实大 PNG 拒绝、独立预算与普通/发送释放顺序，以及菜单→TLS 的 PNG/digest 验证。
Live checkpoint 20/0/0，包含无 poll/reap 的退休 owner 回归；原保存快照 23/0/0、session candidate 55/0/0、
sender checkpoint 3/0/0、scope 模型 4/0/0、隐藏 scope 8/0/0、receiver checkpoint 9/0/0、binding 6/0/0、runtime 9/0/0，
其余原文/保存/档案/几何/崩溃及目标目录逐项回执均封存。

四项集成门 build_resources 2/0/0、exe_resources 1/0/0、startup 7/0/0、session_startup 9/0/0；
全部 cargo 构建完成后才封存产品，集成 child 使用编译时固定 target 产品，前后核与封存 EXE/PDB 同字节。
封存产品的隐藏离线 --workspace-check 通过，使用显式隔离合成 profile，不是真实用户 profile 或真实 Loom 双端验证。
严格全目标 Clippy -D warnings、官方 fmt、版本/组织合同通过；3914 文件 ratchet 零违规/警告/新例外。
相对 v407 native 9 变化/3 新增，最大 302 有效行；逐文件基线 diff、空白与 UTF-8 无 BOM 检查覆盖 untracked 源码。

失败与中间输入保留：run-1 的 Live 17 过/2 失败，首错为退休 worker owner 被下一次保存纳入，第二项为 SERIAL poison。
只在 application_save_live 三处统一按 Workspace 身份及当前图成员选择，不以 is_alive 静默漏存异常关闭的当前 Unit。
run-1 的投送 PNG 2 过/1 失败及广义 export 92 过/1 失败是同一个新增夹具：80×40 层不能经恢复接口声明为 20×10；
改用合法小数逻辑框 79.5×39.75，保持实际像素/尺寸断言，未放宽生产约束。自然 80×40 与逻辑 20×10 的独立出口测试保留。
input-3/run-2 的 26 组虽已全部通过，复核又确认输出 lease 延寿会收紧普通 workflow 的预算准入（未实测用户 OOM）；
最终改为消费 ComposedImage、编码后先释放合成输出再拷贝 PNG，发送 HGlobal 租约仍跨 copy 保留，新增确定性断言后重建并重跑全部门。
首次 integration/product 构建期间发生源码变化，其日志不归最终源；第二次构建属于已被替代的 input-3，也不冒充最终产物。
audit-2 因 EEXIST 被独占证据保护拒绝，另用新报告名；scoped-diff-1 将 --no-index 的正常差异退出码误判失败，
修正 harness 后仍要求 --check 输出严格为空。上述失败回执/原测试程序/PDB/日志不覆盖，无新增 allow/ignore 或期限放宽。

证据根目录为 `linshi/otter-native-v408-source-encoding/`；最终结算包含输入/产物/日志身份、旧 v407 产物及原失败保留核验。
完整 PNG retained-source/目标 metadata/历史编辑/beautify 生产 parity、设备组/持续发送、稳定 binding 显式停止、真实双端/离线互信、
双向编辑/LiveRelay/Tile、资源长稳、完整迁移门及最终用户全面手测仍开放。未重跑完整 34 门或全量 lib，
不关闭 v398 间歇呈现及历史严格前台/启动根因。目标 active、objectiveComplete=false、publishable=false。
无新依赖、真实 profile、前台/全局输入、系统安装、注册表/证书变更、默认 DLL 启用或 commit/push/publish。下方 v407 为历史。

## v407：单目标发送生产接线

最终 input-3 冻结 2759 项构建输入，内部版本 v0.2.33.407，public version 仍为 0.2.33。
源码 SHA-256：`79832dfcc2ef3bfac76fc43ee3f8eed22e22c73439e922fccd423c3dee9b4334`。
产品 EXE 为 45089280 字节，SHA-256：`be2d282773b8da9773d4e368927a83d30f0561addde2e2d3804cbcffcc731b48`；
匹配 PDB 为 291467264 字节，SHA-256：`3b4106956c19bff5347358e24fe9ef233206f575404db6fe1808c25c8c74a163`。
这是独立封存的 dirty 开发产品，不是正式 release、最终 portable 整包或用户验收版。

同源选定 24 组 lib 门全部通过；过滤组重叠，不能累加成独立覆盖总数。
sender 模型/HTTP 过滤组 50/0/0，广义 sender 过滤组 65/0/0，包含生产 Application 的 10 项隐藏菜单/TLS/保存/重开用例，
以及原 Job 四槽、preparation error/panic 后 Request 交接、晚撤权、取消/退休和同 ID 显式重试。
独占未改内容的 outbox 句柄，确定性验证 worker 已完成后的 UI join 不重读磁盘，后续 Ready 完整验证仍须读原基线；
没有用计时断言代替无 I/O 证明。PNG 合成 2/0/0，Application 目标目录 7/0/0，UI 目标目录 2/0/0。
原有 sender checkpoint 3/0/0、scope 模型 4/0/0、隐藏 scope 8/0/0、保存快照 23/0/0、session candidate 55/0/0、
Live checkpoint 19/0/0、receiver checkpoint 9/0/0、binding 6/0/0、runtime 9/0/0，及原文/保存/档案/几何/崩溃逐项门均通过。

四项集成门 build_resources 2/0/0、exe_resources 1/0/0、startup 7/0/0、session_startup 9/0/0；
全部 cargo 构建完成后才封存产品，child 使用编译时固定 target 产品，前后核与封存 EXE 同字节。
封存产品的隐藏离线 --workspace-check 通过，使用显式隔离合成 profile；不是实际用户 profile 或真实 Loom 双端验证。
严格全目标 Clippy -D warnings、官方 fmt、版本/组织合同通过；3911 文件 ratchet 零违规/警告/新例外。
相对 v406 native 45 变化/17 新增，最大 481 有效行；新增和修改文本保持 UTF-8 无 BOM。

失败未抹除：input-1 的广义 sender 为 60 过/2 失败，均是同线程关闭旧实例后的 WM_QUIT 残留令重开报
`Workspace startup interrupted`。只修测试夹具，沿既有有界 drain_messages 排空旧实例消息；
input-2 的 Application 10/0/0 已单独验证。随后发现 UI join 的真实 outbox I/O，input-3 将最终完整校验移至原 worker，
补独占句柄回归并重建、重跑全部选定门；没有把 input-2 成绩继承给最终生产改动。
中间编译/Clippy 的可见性、类型、旧夹具、Config.clone、large_enum_variant/type_complexity 失败及原程序/PDB/日志保留；
修正结构与夹具，不增加 allow/ignore、不放宽断言/期限。一次重复 fmt 回执名被独占证据保护拒绝，换新回执名后执行，旧证据未覆盖。

实现与未完边界见 [v407 迁移记录](WINDOWS_NATIVE_MIGRATION.md#v407单目标发送生产接线)：
PNG 完整 source-resolution/beautify parity、设备组/持续发送、稳定 binding 显式停止、真实双端/离线互信、
双向编辑/LiveRelay/Tile consumers、资源长稳、完整迁移门及最终用户全面手测仍开放。
未重跑完整 34 门、不关闭 v398 间歇呈现及历史严格前台/启动根因；目标 active、objectiveComplete=false、publishable=false。
无新依赖、真实 profile、前台/全局输入、系统安装、证书变更、默认 DLL 启用或 commit/push/publish。下方 v406 为历史。

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

仅linshi合成profile/身份、回环TLS、自有隐藏HWND；不碰真实游戏/profile或全局输入。
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

沿原隔离工具链，仅 linshi 合成 profile/身份、回环 TLS、自有 HWND；不碰真实游戏/profile。
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

沿原隔离工具链，测试仅 linshi 合成 profile/身份、回环 TLS 与自有隐藏 HWND。
不系统安装 SDK/证书、不修改真实 profile/游戏、不替换用户程序、不发全局/CUA 输入。

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

沿原隔离工具链；只用 linshi 合成身份/profile、回环 TLS/自有隐藏 HWND，
无全局/CUA 输入、不触真实 profile/游戏、不替换用户程序。网络取消、原认证重授权、
最后 Unit 直接回收、shutdown drain 及激活后的 synthetic worker panic 分别验证；
不把该故障接缝测试说成生产任意位置 panic 或突然断电全链覆盖。

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

沿原隔离工具链，测试仅 linshi 合成 profile/身份、回环 TLS 与自有隐藏 HWND；不系统安装 SDK/证书、不扩许可或预算、不发全局/CUA 输入。新增真实 1 秒退避、原认证解绑、撤权后重新授权门；不触真实 profile/游戏或替换用户程序。

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

沿原隔离工具链，仅 linshi 合成 profile/身份、回环 TLS 和自有可见 HWND；SW_SHOWNOACTIVATE 并断言前台不变。不系统安装 SDK/证书、不扩许可/预算、不发全局/CUA 输入。实际用户模态点击、真实 Loom、长稳及正式用户包仍开放。

最终严格静态/3799 文件 ratchet 零违规/警告/新例外；同源八件 45 选定门全过。
24 变更/7 新增、最大 381 有效行，UTF-8 无 BOM；visible 3/0、present 5/0、
placement 2/0、确认 1/0，过滤组不累计覆盖。344 旧件/七诊断、开发失败/输入说明、
最终冻结输入/七文档与源码三次匹配见迁移台账 v393/closeout。startup 7/0 不关旧根因，
正式前台菜单仍 ignored；无真实时钟到期/worker panic 全链注入，不代完整迁移。

本入口构建 `native/Cargo.toml` 的独立 Win32 程序，不调用 Tauri/WebView2，
也不是公共发布、签名或最终用户验收包。迁移总计划仍以
[WINDOWS_NATIVE_MIGRATION.md](WINDOWS_NATIVE_MIGRATION.md) 为准。

## 工具链与资源

v392 沿原隔离工具链落实唯一接收准备/隐藏追加/checkpoint 生命周期；网络任务先回收，
再以保原四槽的单个后台线程复用会话候选与原文追加。原 Wake 同时承接完成与撤权，
到期合并原 deadline；无新依赖/SDK 用途/权限/等待句柄/第二任务池或产品轮询。
仅 linshi 合成身份/profile、回环 TLS/自有隐藏 HWND，不运行真实 profile/抢前台。
最终严格静态/3,792 文件 ratchet 零违规/警告/新例外，同源八件 41 选定运行门全过；
25 变更/6 新增、最大 454 有效行，UTF-8 无 BOM。owner 10/0、watch 2/0；过滤组重叠，不累计覆盖。
四份开发 exact 输入、最终 2,640 冻结输入/七文档、首个静态候选与失败程序/析构诊断、
336 旧件/五诊断和后续 344 件/七诊断保护见迁移台账 v392/closeout。startup 7/0 不关旧
偶发根因，前台菜单仍 ignored；不代正式发布/最终用户包。

最终 source SHA-256 为
`484481f67dbde39b34d1c36729bcdabd48fcb0b977f0be2d35466a6d64e0a601`。
debug EXE 为 43,501,056 字节，SHA-256 为
`b3c1b7d1f58ef1d20500e2d46071cdfbe094730ce4f1a6e082972d0a0a066ec4`，
本轮 final-run2/artifact 八件 publishable=false；状态为
projection-receive-owner-hidden-checkpoint-verified-visible-consumer-pending。

v391 沿原隔离工具链补接收意图↔Unit 关联与真实 checkpoint 包含凭证，不增加依赖/
SDK 用途/权限/网络槽、等待句柄或线程。解析沿原后台 worker，产品热路径无读盘，
仅用 linshi 合成 profile、回环 TLS/隐藏 HWND；不占前台或发全局输入。最终严格
静态/3,786 文件 ratchet 零违规/警告/新例外，同源八件 39 选定运行门全过；28 变更/
4 新增、最大 349 有效行，UTF-8 无 BOM。关联模型 5/0、checkpoint 新组 5/0/全组
18/0、恢复菜单 4/0、投送桌面 25/0/客户端 55/0；过滤组重叠不累计独立覆盖。
startup 7/0 不关旧偶发根因，workspace_runtime 2/0/1 ignored 不代正式前台菜单退出。
四份 exact 开发输入、最终 2,634 冻结输入/七文档、328 旧件/五诊断及后续 336 件/
五诊断保护见迁移台账 v391/closeout。首个 final-run 静态/构建通过但未运行验收，
旧版 qrProjection:null 混合会话修正后另建 final-run2，不覆盖原证据。

最终 source SHA-256 为
`a33cab341f3a386db74f8303ceeb30fdff5ac294c78d0f717c2b2cb31cfb4557`。
debug EXE 为 43,378,176 字节，SHA-256 为
`6cd1fb75107bfc0357718d4e4d7f784c15f3acaa5de61943d9f5e5eae77c9c1d`，
本轮 final-run2/artifact 八件 publishable=false，状态为
projection-checkpoint-inclusion-verified-receive-consumer-pending；不组正式发布/用户包。

v390 沿原隔离工具链接默认 profile 的单次后台 journal 与显式未结解绑消费者；
不增依赖/SDK 用途/权限/网络容量或等待句柄，完成通知复用 Wake。仅 linshi 合成
profile/身份、回环 TLS 与隐藏 HWND，不运行真实 profile 或抢前台；检查模式不
发现投送 Store。最终严格静态/3,782 文件 ratchet 零违规/警告/新例外，同源八件
37 个选定运行门全过；19 变更/5 新增、最大 453 有效行，UTF-8 无 BOM。profile
4/0、恢复菜单 3/0、Application 解绑 8/0、journal 15/0、投送桌面 24/0/客户端
50/0，过滤组不累计覆盖。startup 7/0 不关旧偶发根因，workspace_runtime
2/0/1 ignored 不代正式前台菜单。两份 exact 开发输入、最终 2,630 冻结源码输入/
七文档、初始空过滤/夹具失败程序及 320 旧件/四诊断由 closeout 复核；后续保护
328 件/五诊断。不发布、替换用户程序或把显式减权恢复当完整接收。

最终 source SHA-256 为
`66bf526a30bc7e69c4651e9a96cde5d9cfca9dad0fd17d5c26932657308bcc4c`。
debug EXE 为 43,269,632 字节，SHA-256 为
`d8bda8e6b8f4ce6036a0d7c1e1f49c2f7f262b641b5b9a55095a67a8885f20ef`，
本轮 final-run/artifact 的八件 publishable=false，状态为
projection-profile-recovery-verified-receive-consumer-pending；逐门结果见迁移台账 v390/closeout。

v389 沿原隔离工具链补接收写前意图/显式认证解绑，不增依赖、SDK 许可用途或 OS
权限。复用 session 文件事务和共享 Pixels 预算，队列限 128 项/512 KiB，固定容量
读取；仅 linshi 合成数据/回环 TLS/自有隐藏 HWND/原子进程夹具，不安装系统证书或
触碰真实 profile。最终严格静态/3,777 文件 ratchet 零违规/警告/新例外，同源八件
的 34 个选定运行门全过；16 变更/6 新增、最大 440 有效行，UTF-8 无 BOM。journal
14/0、认证补偿 9/0、投送客户端 49/0，过滤组重叠不累计覆盖；startup 7/0 不关闭
v387 偶发根因，workspace_runtime 2/0/1 ignored 不代正式前台菜单退出。六份开发
输入及最终 2,625 冻结输入/七文档另存；312 旧封存程序/四诊断复核不变，后续保护
320 件/四诊断。接受菜单、工作区意图恢复、真实显示/displayed
与退出补偿调度仍缺，不构造正式发布/最终用户包或替换运行程序，不重跑前台。

最终 source SHA-256 为
`e3a95758ce5889b3146be49eb46a8da339b2b5aa191afd168368b21098667422`。
debug EXE 为 43,197,440 字节，SHA-256 为
`9288b8ed4ba19f82593bde268c1244f4d8fa2a713e5ce4af062c87f929d79463`，
本轮 final-run/artifact 的八件为 publishable=false。三次源码、旧失败完整输入和
逐门结果见迁移台账 v389/closeout；状态 projection-receive-journal-verified-consumer-pending。

v388 沿原隔离 SDK/工具链完成传统接收准备内核，无新依赖、许可用途或资源预算。
严格 fmt、全目标 Clippy -D warnings/check、版本和 3,771 文件 ratchet 均通过，
同源八件的 32 个选定后台/隔离运行门全过。接收 TLS/PNG/同 Grant、单贴纸追加、
checkpoint 重启逐像素、冻结库/工作流归档及资源/三类启动均通过，过滤组重叠。
31 变更/12 新增、最大 367 有效行，UTF-8 无 BOM；首轮大枚举 Clippy 失败/exact
输入另保，Box 化后 final-run2 重建，不继承旧字节结论。startup 7/0 不关闭 v387
偶发超时根因，前台菜单仍 ignored；接受菜单/持久补偿/真实显示/displayed 尚缺。
最终冻结 2,619 输入，source SHA-256 为
`7c42f34f3647106b5898609819c77a495751a4842890bc585ba9272169b7d5ae`。
debug EXE 为 42,998,784 字节，SHA-256 为
`bcabcba772c1d6c0d45dc2bf8913139e74439342af87b429b5897dac154324ec`，
本轮 final-run2/artifact 的 publishable=false 八件及 304 旧件/四诊断/后续 312 件
保护见迁移台账 v388/closeout。无系统 SDK/证书安装、真实 profile 或 Loom 变更，
不提交/推送/发布/替换用户程序，不代完整迁移/最终用户包。

v387沿原隔离SDK/工具链接传统投送桌面菜单查询/拒绝；测试使用自有隐藏/离屏HWND、
原回环TLS/身份夹具和linshi隔离profile，不安装系统证书、不访问真实Loom或重新配对。
缺坐标合成会话导致的开发失败已封存完整输入，仅修合法夹具；最终静态/八件同源
封存/296旧件及四诊断/选定相邻门见迁移台账v387。原workspace_runtime的15秒及断言
不变，不重跑前台、不组正式发布/最终用户包或替换用户程序，SDK许可/资源预算不扩。

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

v386沿原隔离SDK/工具链验证纯移动结算；RED使用隐藏自有HWND，不发全局/CUA输入。
原workspace_runtime仍以同源EXE、隔离profile、原15秒检查移动/中断重启/删除链，
不拿开发green代封存候选。最终静态/三次源码/288旧件及四诊断身份见迁移台账v386；
不系统安装SDK/证书、不重跑菜单前台、不发布或替换用户程序。

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

v385沿原隔离工具链，为传统投送inbox/rejected客户端运行真实回环TLS与后台任务
验证；复用测试CA/linshi唯一文件夹，不安装系统证书、不访问真实Loom/profile或
重新配对。未接桌面接收，不重跑前台；最终严格静态、同源程序、280旧件/四诊断
身份及相关运行见迁移台账v385。不是正式发布/最终用户整包，不替换运行版本。

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

v384沿原隔离工具链，仅修真实产品菜单测试的类名大小写matcher；新离屏NOACTIVATE
回归核PID/owner/visible，不改产品/依赖/SDK许可或原等待预算。开发matcher真实
RED0/1→GREEN1/0，显式正式菜单退出1/0；无CUA输入或全局输入注入。最终严格静态、
同源八件封存、显式菜单门及272旧件/四诊断核验见迁移台账v384，默认ignored不计
通过；不是正式发布/最终用户整包，不替换运行版本。

v384最终严格静态/3,742文件ratchet通过；同源八件19门18过/1前台失败。正式菜单
0/1在owned-product-foreground原15秒失败，不继承开发1/0，状态仍native-exit-menu-
foreground-pending。2,590输入source摘要为
`71f9cdf9b548a81b120558fe239058c99ceb6a69799f1069ed30094baa9457c5`；debug产品
42,710,016字节，SHA-256为
`57007f89a88e2d3febf7bc27d8f2eaa97fcbbc4961cea7a8ef40203ea2890f29`，位于本轮
final-run/artifact且publishable=false。三次源码/失败完整输入与七文档/八件/旧272件/
四诊断及280件后续保护见closeout，不将后台成功或默认ignored当最终菜单验收。

v383沿原隔离SDK/工具链验证最终退出检查点与Live一次性批准；合成WARP/隐藏HWND
和隔离profile不触及真实用户数据。新增普通产品正式菜单→确认→最终提交→进程
退出门只操作自启离屏窗口及本进程所属确认框，不注入全局输入或操作游戏。最终
静态/同源封存/定向运行/256旧件及四诊断的状态见迁移台账v383；不是正式发布或
最终用户整包，不替换运行版本，也不扩大SDK许可/预算。

v383首轮19门17过；startup4/3及产品前台0/1保留于final-run，严格closeout拒绝成功。
其完整2,589输入/七文档复制到failed-source，八件封存程序和failure-receipt另保。
final-run2沿用同一内部版本，分离无profile退出兼容性与严格profile快照空闲；最终
定向后台结果另记。菜单测试默认ignored且单独显式门已失败，当前候选不重复争抢
前台或继承首轮原生结论，不拿普通启动/隐藏HWND成功替代真实菜单退出验收。

v383最终final-run2严格静态/版本/3,741文件ratchet与18个选定后台/隔离门均过；退出
11/0、相邻Live58/0（含退出11）、startup7/0，workspace_runtime1/0/1 ignored。真实
菜单退出未重跑，状态exit-checkpoint-native-pending。冻结2,589输入，source SHA-256
为`bf8ba2bb5b59f75f5e1af330cd457a1103209531bd71e778bd119723a8a57ab2`；debug EXE
42,710,016字节，SHA-256为
`394128b414aa55675238577701b3958680d1cc8c8f292efe28a81f819ca0b64e`，位于本轮
final-run2/artifact，publishable=false。三次源码/八件/旧256件/四诊断及首轮失败
完整输入/八件由closeout复核，下一轮保护272件另保四诊断；不组最终用户包或替换程序。

v382沿原隔离SDK/工具链，Live自动时间点只用合成WARP帧、自有隐藏HWND与linshi GUID
profile/文件测试；不捕获用户桌面、注入全局输入或读取真实profile。最终冻结源码、
严格静态、封存程序、相关运行和248旧件/四诊断核验见迁移台账v382；开发中间编译
不代表候选已验。无依赖/权限/预算扩张，不组最终用户包或替换运行程序，不发布。

v382最终严格静态/版本/3,735文件ratchet与17相关运行门全过，18变更/4新增最大437行。
冻结2,583输入，source SHA-256为
`849d646103b495acf2dccba8266f1fbfd00bedc92bce880d576f0ca4f18cb3f8`；debug产品EXE
42,658,816字节，摘要为
`10f16609227c9c0229e735a1c600ba903512b3275d569572e4bc65c85c4c6fee`，位于本轮
final-run/artifact，publishable=false。三次源码/八件/旧248件/四诊断及256件后续保护
由closeout核验；只短暂运行自启离屏产品窗口，无全局输入，不替代最终用户整包。

v381沿原隔离工具链/SDK许可，没有新依赖或预算。普通无来源启动默认写独立native
profile的workspace.json；本轮自动测试仅在linshi GUID profile内运行，不访问真实用户
数据或旧Tauri目录。--workspace-check必须显式OTTER_NATIVE_PROFILE_DIR，只读workspace
内容（仍创建隔离目录/lease）。真实产品入口、原生Application检查点/中断/最后Unit、
profile租约及相关保存/恢复门与身份由本轮final-run2封存，不用build green代业务验收。
普通启动门可能短暂激活自启离屏窗口，不注入全局输入。证据在本轮linshi目录；不组
最终用户包或替换运行版本，不以N06静态成功关闭旧前台/完整迁移门禁。

v381最终严格静态/版本/3,731文件ratchet及14相关运行门全过，25变更/8新增最大447行。
冻结2,579输入，source SHA-256为
`0a77734aba522bdda0f24e304a6d8eef90b5c1198f226e65ee0669bc9346607f`；debug产品EXE为
42,653,696字节，摘要为
`650cd64514a07c429ea9b2fe126b88822b684b12a5cc13e68ed245cf94570fb0`，在本轮
final-run2/artifact。真实普通启动1/0和只读启动5/0通过；三次源码/八件/旧240件/四
诊断、首轮Clippy失败完整输入、248件后续保护由closeout核验，publishable=false。
未执行旧前台焦点门，不打最终用户包、不改默认JS/SDK用途或既有预算。

v380复用原隔离工具链和已批准SDK用途，证据在linshi/otter-native-v380-workspace-autosave。
新增静态工作区自动保存只在当前进程显式开启，不默认访问/写入旧版目录；隐藏产品
smoke使用linshi独占GUID会话，验证菜单/worker/落盘/恢复，无前台或全局输入。开发
失败保留，最终冻结/静态/程序封存/定向运行另记；debug不是公开发布或最终用户包。
本轮不重跑v379前台候选，也不以新N06通过关闭旧失败、默认JS或完整N01–N12门禁。

v380最终final-run2严格静态/版本及3,723文件ratchet与10个N06相关运行门全过；
19份native变更/4新增最大432有效行。源码摘要为
`b76da3f775b2eecb0212ffd34d278b4515b11dd2d94aec88e7cbb3cee0f88711`；debug EXE为
42,606,080字节，摘要为
`0a9838258ace0d424fbf4dd1227596ac7de52f8540606b2fe749a01fa44cdb89`，位于本轮
final-run2/artifact。三次源码/六件/旧234件与四诊断及240件后续保护由closeout核验；
首轮行数失败完整源码保留，未新增豁免。真实startup7/0和session_startup9/0不是
默认自动恢复/崩溃恢复的证明，未组最终用户包、发布或替换用户运行程序。

v379仅测试前台启动/双向许可候选，沿原隔离工具链和SDK用途；证据在
linshi/otter-native-v379-foreground-handoff。显式OTTER_NATIVE_FOREGROUND_SPONSOR=1才
启用就绪/G握手，Application必须在原TLS夹具计时前等待；默认和产品不等待。诊断
launcher仅启动摘要已核实的自有测试进程，经自有按钮实际点击授予精确PID；不使用
ASFW_ANY或改系统锁设置。最后真实输入导致前台前提失败，停止前台测试后四门明确
未执行，仅后台/静态与封存身份另报，状态foreground-handoff-native-pending。无新
依赖/SDK/预算，不将debug候选当发布、最终用户包或历史失败修复，详见迁移台账v379。

最终严格静态/版本/3,719文件ratchet与八个后台门均通过，四份native测试变更最大467
有效行，冻结2,567输入。source SHA-256为
`0176bdb12c84679f8b705b65b358815bcb801d9b059414dfeb3faa352969832b`；debug产品EXE为
42,539,520字节，SHA-256为
`5123127eb3003110811bf6617eeebd7103d2777cd4cbe40b2765e4f501af8b8b`，在本轮final-run/
artifact。三次源码/六件/旧228件及四诊断分别核验，下一轮保护234件并另保四诊断；
同源前台0/1与四项未重跑门仍开放，不以后台成功宣称修复，publishable=false。

v378沿原隔离工具链/SDK用途，开始前核v377源码/七文档及222份程序。证据独立在
linshi/otter-native-v378-border-sides；最终同源六程序22相关门全过，含摘要绑定产品及四
原规模/原预算压力门，严格静态/3,719文件ratchet和版本通过。45变更/11新增最大445
有效行，冻结2,567输入，source SHA-256为
`895f5f7499b00986e3ac4bf6aaa9e876b89b4c946223b4710a192ec4eddffea6`。debug EXE为
42,539,520字节，SHA-256为
`4d143dff4eaffa5e4d0a2e55643f096e5026473dc4ab938a8103a58aa8058f81`，位于本轮
final-run/artifact/otter-native.exe。三次源码/六件/旧222件/行为RED与后续228件保护见
迁移台账v378；ignored不计通过，旧前台/压力根因未关闭。无新依赖/SDK/权限，不提高原资源预算。
只操作自有隐藏HWND，无全局输入；debug证据不等正式发布、最终用户包或完整迁移。

v377沿原隔离工具链/SDK许可，开始前核v376源码/七文档/216件程序。新增单行文字
ellipsis不引入依赖或新进程/线程/缓存，不提高原heap/stack/250ms/Job/startup预算。
RED/开发失败与最终同源静态、真实DWrite/WIC、child/产品/HWND和相邻运行门分别
保存在linshi/otter-native-v377-text-overflow；详细终态/身份见迁移台账v377。debug
证据不等正式发布或最终用户包，本轮隐藏HWND不要求前台，也不发送全局输入。

最终19相关门全过，严格静态/版本/3,708文件ratchet通过；17份native变更/7新增最大
440有效行。冻结2,556输入，source SHA-256为
`cbc2403cb3cff4c19d870523766ffde4275ea285fa9658bceba43ab19293a105`；debug EXE为
42,488,320字节，SHA-256为
`ae0717ded68f302ad31191cf556c4b28be02ce9707de2a1c632069c3db99aaf9`，在本轮
final-run/artifact/otter-native.exe。三次源码/六件/旧216件/行为RED与探针分别核验，
下一轮保护222件；产品和四压力门显式执行，ignored不计通过，未重跑的v376两项前台
失败继续开放。完整迁移/真实股票Loom/最终用户包仍未验收，publishable=false。

v376沿原隔离工具链，冻结2,549输入，严格静态/版本/3,701文件ratchet通过。仅四份
native变更最大386行，新测试116行；同源六程序固定20门18过/2前台前提失败，四个
原规模压力门本次通过，但固定15样本未显示整体加速，不放宽预算或重试求绿。三次
源码/六件/旧210份及RED、两项失败和性能诊断见迁移台账v376；后续保护216份。debug
EXE为42,479,104字节，SHA-256为
`a69293c673a11cba258597bc2d4a3a7b841a0581954c471dbd11b9f1e9597239`，位于
`linshi/otter-native-v376-css-collect/final-run/artifact/otter-native.exe`，仍partial-runtime-failure、
publishable=false，不是正式发布或最终用户包。用户授权的前台测试仅自有HWND定向
消息，无全局输入，旧完整窗口/Application失败不由单项成功关闭，SDK许可用途不变。

v375最终final-run2沿同一工具链，只修旧拒绝样本，严格静态与3,700文件ratchet通过；
21相关运行门17过/4压力失败，功能成功不代压力/长稳或完整迁移。十一native变更
最大437行；2,548输入/三次源码/六件/旧204份及首轮源码/七文档/六件分别核验，
下一轮保护210份。debug EXE为42,479,104字节，SHA-256为
`c6180005b49c8b0975bd061efd04fc446728c8bce702308eda868271072230d6`，位于本轮
final-run2/artifact；partial-runtime-failure、失败分解/未重跑完整窗口及精确身份见
迁移台账v375，不使用首轮压力成功覆盖最终失败，也不是正式发布/最终用户包。

v375沿原隔离工具链，保护198份旧封存程序；证据位于linshi/otter-native-v375-css-hover。
开发RED/诊断与最终冻结/六件程序/实际产品worker摘要绑定矩阵分开记录，失败不覆盖。
用户已允许短暂占用前台，测试仅定向自有HWND，不注入全局输入。最终静态、行数、
运行成绩和身份见迁移台账v375，debug证据不是正式发布或最终用户包；原SDK许可及
heap/stack/250ms/Job/startup预算不变，不以测试分类ignored代替显式产品/压力门。

首轮final-run严格静态/3,700文件ratchet通过，十变更最大437行；22运行门18过/
4失败的旧拒绝样本和完整窗口前台失败分别见迁移台账v375。六件/三次源码/旧198份
封存核验，debug EXE为42,479,104字节，SHA-256为
`971a4ae50a2c700e219e125697092659c1d67268cbcd337634294213a70462cd`；首轮身份不能与后续
只修负向样本的候选混用，不以相关成功关闭旧完整窗口或迁移/用户包验收。

v374沿原隔离工具链，保护180份旧封存程序。证据在linshi/otter-native-v374-focus-admission；
诊断1/0、同线程控制前提失败0/1与独立进程控制1/0各自绑定源码/程序身份，不覆盖
日志。用户允许短暂占用前台，只激活自有窗口、不注入全局输入。最终静态/相关封存
运行矩阵及身份见迁移台账v374，debug证据不是正式发布或最终用户包，原SDK许可不变。

首轮final-run的11门6过/5失败与六件程序已封存；后续final-run2只修测试验证边界和
track_caller，预定10相关门，显式跨进程负向仍强制运行，不将ignored计通过。旧完整
Application的Enter前台获取失败不重跑求绿，首轮13/12仍开放；两套身份不混用。

final-run2为10门7过/3失败；final-run3复用版本只修Child目标线程的单次激活确认，
仍按10门执行并保护旧192份。原5秒确认/命令容量/生命周期不扩大，失败不覆盖；
各候选最终身份、静态与运行结果见迁移台账v374，非正式发布或最终用户包。

最终final-run3严格静态/3,696文件ratchet及10相关运行门全过；Application focus3/0、
完整默认窗口129/0/1 ignored，显式跨进程负向另有1/0。六变更最大321行，2,544输入/
三次源码/六件/旧192份、前两候选和开发诊断分别核验，下一轮保护198份。debug EXE
42,475,008字节，SHA-256为`4ebc250e39ee700488d9fa0bf261f6717df0a0ad973b2384b3fca451f94119eb`，
位于本轮final-run3/artifact。未重跑的旧完整Application失败不由本次成功覆盖，完整
迁移/用户包未验收，详见迁移台账v374/最终closeout。

v373沿原工具链只修测试线程的modifier状态夹具，保护旧174份封存程序；证据在
linshi/otter-native-v373-keyboard-state。诊断与确定性RED各用独立源码/程序身份，
不与最终候选混用；最终静态/八个相关运行门及身份见迁移台账v373，不重复完整
旧矩阵求绿、不改变原SDK许可或把debug开发证据当正式发布/最终用户包。

最终严格静态/3,696文件ratchet通过，8门7过、1失败；窗口129/0、fixture7/0及产品
keyboard通过，Application14/11的焦点断言/十项锁中毒保留。debug EXE为42,475,008
字节，SHA-256为`f6beb6e01d61afcaafe17750a72dd1a6698cf861968901e2a832e49c807ba096`，
位于上述final-run/artifact。2,544输入/三次源码/六件/旧174份与三件诊断程序分别
核验，见迁移台账v373/closeout；下一轮保护180份最终封存件，仍partial-runtime-failure。

v372复用已分配版本与原隔离工具链，保护旧168份封存程序；证据位于linshi/
otter-native-v372-hover。普通hover测试仅定向自有HWND，不移动系统光标、不注入全局
输入，用户允许短暂占前台；原预算/SDK许可不扩大。最终源码冻结/六件封存程序、实际
产品worker及Application/相邻回归按各自真实成绩记录在迁移台账v372，不把编译通过
当作原生/业务验收，失败不覆盖，debug开发证据不是正式发布或最终用户包。

最终`final-run2`的严格静态/3,696文件ratchet通过，冻结2,544输入、六件程序；34运行
门32过、2失败。独立Hover Application1/0不能覆盖完整Application13/12或窗口122/4；
失败分解/三原规模压力门、三次源码/旧168份/七文档/进程见迁移台账v372与closeout。
debug EXE为42,475,008字节，SHA-256为
`e484a50c5370f1ce562cfbe7c6296e6b1aa8bebde6da534aaac1152a451a4ada`，位于
`linshi/otter-native-v372-hover/final-run2/artifact/otter-native.exe`。首轮final-run无
封存程序，Clippy失败不覆盖；下一轮保护174份，仍partial-runtime-failure，不作用户包。

v371沿原隔离工具链实现Canvas左键Pointer子集，保护旧162份封存程序。版本仅分配
一次，证据在linshi/otter-native-v371-canvas-pointer。真实HWND测试仅短暂使用自有
窗口前台，不发送全局输入；最终源码冻结、六件摘要绑定程序及完整相邻运行门见迁移
台账v371。编译/夹具失败保留，原预算/SDK许可不变，debug证据不是正式发行或最终用户包。

最终静态/3,684文件ratchet通过，2,532源码输入与六件程序封存；27运行门26过、1失败，
新增Pointer的独立Application1/0另列，不覆盖完整Application13/11。debug EXE
42,429,440字节，SHA-256为`fcee278114ad964417518cf093a3470c68c52e0cbae8046f7e9d39bf6e5a3479`，
位于上述final-run/artifact。行数/运行分解/三次源码/旧162份/收尾见迁移台账v371，
下一轮保护168份，仍为partial-runtime-failure，不作为正式发布或最终用户包。

v370沿原工具链优化CSS cascade，保护旧156份封存程序，版本仅分配一次，证据位于
linshi/otter-native-v370-css-cascade。原压力规模/预算、固定15样本和阶段观察分报；
最终冻结源码、六件程序和真实产品worker摘要绑定成绩见迁移台账v370。仍为debug
开发证据，不是正式发行、默认完整JS或最终用户包，不扩大已有Otter隔离SDK许可。

最终严格静态/3,671文件ratchet通过，六件无重编24门23过、1失败，三诊断分报；压力
三路径各1/0，Application前台获取失败保留。debug EXE摘要/2,519源码身份、两文件
行数及新旧固定矩阵见迁移台账v370，三次源码/旧156份/文档/进程由final-run closeout
核验，下一轮保护162份，仍为partial-runtime-failure，不以本机短样本冒充长稳。

v369沿原工具链修测试前台guard退出后覆盖新选择的所有权错误；先核v368身份和150份
封存程序，证据独立在linshi/otter-native-v369-click-boundary。实际RED与首次HWND私有性
编译失败分别留证，未公开HWND/增加依赖或改变产品准入。候选诊断1/0不关闭历史失败；
最终静态/封存完整组及三次源码/六件身份另记，原资源预算和Otter隔离SDK许可不变。

首轮Clippy冗余import失败留证，最终final-import-correction冻结2,519输入，source
SHA-256为`dd88d9abb1ee988b6960e4d3309ff82c77a6d8080c384cdb2e4634ef9c39ff63`。严格静态/
3,671文件ratchet通过；12无重编运行门10过、2失败，Application23/0、values5/0及
startup7/0等通过，foreground1/1与完整窗口93/18的前台获取/锁中毒分别保留，不重试。
debug EXE为42,295,296字节，SHA-256为
`db2f88a2fd86c39fc48c03b0f39b5f78cae949998c8c992d77344fa870c3bfc5`，位于
`linshi/otter-native-v369-click-boundary/final-import-correction/artifact/otter-native.exe`。
三次源码/六件/原150份及收尾由closeout核验，下一轮保护156份，保持partial-runtime-failure；
不是正式发行、默认完整JS、前台稳定性或最终用户包，不扩大SDK许可。

v368沿原工具链调查Enter Application失败，先核v367身份及旧144份封存程序；证据独立
在linshi/otter-native-v368-enter-drain，版本只分配一次。只改测试驱动/后台不重放证明，
真实前台条件和原16次poll、250ms/Job/startup均不放宽；开发失败分别保留。最终完整
Application/相邻门和源码/程序身份见迁移台账v368，非正式发行/最终用户包，不扩大SDK许可。
开发负向门1/0、Application前台前提门0/1分别留证。确认外部前台属于当前运行的游戏
后不操作该进程，先暂停交互验证并完成后台构建。用户随后允许短暂占用前台，最终
恢复完整10门矩阵，不用background-only；实际成绩另记，不以授权或启动算通过。

最终final-run冻结2,518输入，source SHA-256为
`5d6cf347de731f05dff9c16b4c6c05ab5c7661a877296ac44c44ddff1df9bc38`。严格静态/3,670文件
ratchet通过；无重编10门8过、2失败，Application13/10、SurfaceWindow90/19的直接失败/
锁中毒分解见迁移台账v368。startup7/0不关闭历史期限根因，新增负向最终取锁失败。
debug EXE为42,295,296字节，SHA-256为
`f804a610730bfecd1ee002090a179b8b155a88d4861e3d4d3806e02668a863c3`，位于
`linshi/otter-native-v368-enter-drain/final-run/artifact/otter-native.exe`。三次源码/六件/
原144份及收尾由closeout分别核验，下一轮保护150份，保持partial-runtime-failure；
不是正式发行、默认完整JS或最终用户包，不重试求绿、不提高预算或扩大SDK许可。

v367保持既有QuickJS/Win32/D2D与原资源预算，优化原规模CSS压力路径，无新依赖/SDK。
独立证据在linshi/otter-native-v367-css-projection，先核并保护旧138份封存件；固定诊断
不是预算成功门，原压力与真实child/实际产品完成断言保留。最终静态/源码/六件与运行
见迁移台账v367，不覆盖开发失败、不把debug证据作为正式发行或最终用户包。

v367最终final-timer-correction冻结2,518输入，source SHA-256为
`a0fd15d623b79d97abd75593f25cee991f6a58cb55d9eaad271ed2de30792370`。严格静态/3,670文件
ratchet通过；24实际运行门18过、6失败，另两诊断exit0不等吞吐通过。原压力三路径均
失败，Application13/10、JS137/38/26 ignored、startup4/3的直接失败/锁中毒见迁移台账。
debug EXE为42,295,296字节，SHA-256为
`3a8c29f3a198e30af383cca7f75cabe2209643365d211313e537d1cfb73f84f5`，位于
`linshi/otter-native-v367-css-projection/final-timer-correction/artifact/otter-native.exe`。
三次源码/六件built/sealed/原138份由closeout分别核验，保持partial-runtime-failure；
下一轮保护144份，不重编覆盖本轮证据、不扩大SDK许可或当作完整迁移/最终用户包。

v366沿既有QuickJS/Win32/Direct2D接入有界author CSS nth-child，不新增依赖/SDK/默认
能力或预算；证据独立在linshi/otter-native-v366-nth-child，保护此前126份封存程序。
真实RED、最终静态/运行、源码/六件身份与边界见迁移台账v366，不覆盖旧日志/程序。
仍是debug开发证据而非正式发行或最终用户包，SDK许可继续仅限既有Otter隔离构建。

v366最终夹具修正轮冻结2,512输入，source SHA-256为
`b26396bbccec4a6807fe89a743e634b9da0a473d31181ffddceea76418392ec9`。严格静态/3,664文件
ratchet通过，14个封存运行门13过、1压力失败；原250ms不改。首轮Application13/10与
固定一次Enter复核1/0另保留，根因仍开放，未重复完整Application组。debug EXE为
42,291,200字节，SHA-256为
`46de2d9a1375472810757a9cec2a934bac7370ec397d946bb8febdbea55a1abb`，位于
`linshi/otter-native-v366-nth-child/final-fixture-correction/artifact/otter-native.exe`。
最终三个产品worker门绑定当前SHA；最终EXE不同于首轮，不继承首轮Application整组
验收。三次源码/六件/原132份、七文档/组织合同和进程由closeout核验，仍为partial，
不作正式发行、字节可复现、默认完整JS或完整迁移/最终用户包声明，不提交或推送。

v365沿既有QuickJS与原form owner优化有界值校验，无新依赖/SDK/默认能力或预算。
独立证据在linshi/otter-native-v365-form-value-budget；修改前核v364源码/收据及旧
120份封存件，不覆盖程序或日志。定向前后探针仅是本机固定样本，不代预算稳定性
或正式包；最终身份/运行/失败见迁移台账v365，SDK仍仅限既有Otter隔离构建。

final-run冻结2,506输入，source SHA-256为
`aa1b025d7f08b72c7f90440598a4e3b73aae555015993df205f67d5e01125758`。
严格静态/3,658文件ratchet和43选定封存门通过；十四摘要绑定产品worker各1/0，固定
探针另记，不把短样本当作资源长稳。debug EXE为42,291,200字节，SHA-256为
`b2bc7a0862c58ea48fd592c353d7d8ecc08b9487f1a356057cb9f81173b3e087`，位于
`linshi/otter-native-v365-form-value-budget/final-run/artifact/otter-native.exe`。三次源码/
六件/旧120份、前后探针和失败分别绑定；仍非正式发行、默认完整JS或最终用户包。

v364继续使用既有QuickJS/Win32 EDIT与原单槽，接入受限Enter默认change取消，无新
依赖/SDK/权限或预算。独立证据在linshi/otter-native-v364-native-enter，保护旧114份
封存件，版本仅分配一次；开发失败和最终构建/封存运行分开留证，见迁移台账v364。
仍是debug开发证据，不是正式发行或最终用户包；SDK许可继续只限Otter的linshi隔离
构建，不安装/注册/分发SDK、操作证书或默认启用新DLL。

final-run冻结2,502输入，source SHA-256为
`dd980ec888213d6c604d66f5ecd287ebd66c5d43cbb0e2901046307a47a0ad9d`。
严格静态/3,654文件ratchet通过；41封存运行门首轮40通过、1窗口门失败，同字节固定
六门复核通过但不覆盖mount deadline失败，closeout仍为partial-runtime-failure。Enter
11/0/1 ignored、Application23/0、十三产品worker各1/0及startup7/0等精确实绩见迁移
台账v364。debug EXE为42,291,200字节，SHA-256为
`89f12f7b9a1c1af6b0b08a6a8993358651f3d0384a50ad062cb8e55e123d98fc`，位于
`linshi/otter-native-v364-native-enter/final-run/artifact/otter-native.exe`；三次源码/六件
程序/旧114份和诊断分别绑定，不是正式发行、默认完整JS或最终用户验收包。

v363沿既有QuickJS/原回复/Win32 EDIT接入受限focus/blur，无新依赖/SDK/原预算变化。
证据独立保存在linshi/otter-native-v363-native-focus；同源child RED与开发编译失败保留，
最终严格静态、冻结/六程序封存和运行结果见迁移台账v363。保护旧108份封存件，不
重分配版本、不覆盖旧日志；仍是debug开发证据而非正式发行或最终用户手测包，SDK
许可继续只限Otter的linshi隔离构建，不安装/注册/分发SDK或默认启用新DLL。

final-run冻结2,493输入，source SHA-256为
`f6cff4cde34170b1621b56e82aaf984bdfc9d4fff79602d3db3a2cc82ac628c6`。
严格静态/3,645文件ratchet和37个无重编封存运行门通过，精确子集见迁移台账v363。
debug EXE为42,211,328字节，SHA-256为
`41285065ba134cfe80dcf9eaa87f1a5ea25dca1ccdee485aee6c8c19acb15540`，位于
`linshi/otter-native-v363-native-focus/final-run/artifact/otter-native.exe`。同目录closeout
复核三次源码/六程序/旧108份/文档/进程；仅开发切片，不是正式发行或最终用户包。

v362文本 input/change 继续使用既有 QuickJS/Win32 EDIT/Grant/FIFO，无依赖/SDK或
原预算扩张。独立证据在 linshi/otter-native-v362-native-input，最终静态/冻结/封存
运行结果见迁移台账v362，不覆盖此前102份封存件、不重复分配版本。debug开发
证据不是正式发行/最终用户手测包；SDK许可仍仅限Otter的linshi隔离构建。

final-run冻结2,485输入，source SHA-256为
`2e66985cc4f1c4346e02f5b06f8846f5a7cf594e45d4c733aef6f48377be947e`。
全目标严格静态/3,637文件ratchet及33封存门通过，输入7/0/1 ignored、JS153/0/16、
Application19/0、十一产品门各1/0、startup7/0等精确成绩见迁移台账v362。debug EXE
为42,158,080字节，SHA-256为
`e3687b227145e6abc90ab515eb20500339333aa89699b0f85a379360d832a0b4`，位于
`linshi/otter-native-v362-native-input/final-run/artifact/otter-native.exe`。三次源码/六件
摘要与旧102份、文档/进程由该目录closeout绑定，不作正式包/完整用户验收声明。

v361没有新依赖/SDK或原预算变化。EventTarget/原 native click 复用同一 QuickJS/child/
HWND/Grant/FIFO 路径，证据独立位于 linshi/otter-native-v361-event-target；聚焦测试、
最终冻结/六程序封存与无重编运行见迁移台账v361，不沿用v360程序或重分配版本。
此前96份封存程序必须保留；实际debug产物不是正式发行/最终用户手测包。SDK仍
仅限已批准的Otter隔离构建，不系统安装、修改注册表、分发或默认启用新DLL。

final-run冻结2,477输入，source SHA-256为
`a9e4fa0dfdd121864e65babb0c45256ba9ece637348769e169635f4ba1d8a892`。
全目标严格静态/3,629文件ratchet、六程序构建与无重编29封存门通过，含事件10/0/1
ignored、JS149/0/15、Application18/0、十产品worker各1/0及startup7/0。19个native变更
最大386有效行、八新增最大148，无新例外；完整精确成绩及失败保留见迁移台账v361。
debug EXE为42,067,456字节，SHA-256为
`930ee1f69ee68b9a6b3d3d58f4dca7863c1cb8cc60d0264d18870f2d7cc9d32c`，位于
`linshi/otter-native-v361-event-target/final-run/artifact/otter-native.exe`。
同目录closeout核三次源码/六件built与sealed/旧96份/文档及相关进程；只是开发留证，
不是正式发布、可复现构建或最终用户验收包，不重复分配版本或清除旧未知根因。

v360无新依赖/SDK或预算变化，四边内容盒继续复用 DirectWrite、D2D 和原 EDIT owner。
证据独立位于 linshi/otter-native-v360-padding，源码/构建/封存运行的实际成绩见迁移
台账 v360。旧84份及本轮首轮六份共90份封存程序须保留；不是正式发行、完整原生
迁移或最终用户手测包。SDK仍仅限获准隔离构建，不系统安装、注册表、分发或
默认启用新DLL；旧Canvas/启动未知失败保持开放，版本不重复分配。

首轮final-lint-correction的26门仅25门通过，JS99/42/14 ignored的过期测试预期和
锁中毒失败保留。只修两个测试输入后，最终final-fixture-correction冻结2,469输入，
source SHA-256为`9855a0958c95c32191ce0624a1524c5322c50856a99cc9e9dd32a1f0b3256b91`。
全目标严格静态、3,621文件ratchet和六程序构建通过；40个native变更最大455有效
行、无新例外。封存后无重编26门通过，padding43/0/1、JS141/0/14、九产品worker
各1/0与startup7/0等成绩见迁移台账v360。最终debug EXE为42,059,264字节，SHA-256
为`2f5df413a045112d5ee1732672265a5846e3885e1c1aed29cbbcdd1fbdbb359b`，位于
`linshi/otter-native-v360-padding/final-fixture-correction/artifact/otter-native.exe`。
三次源码/六件/旧90份与零相关残留由该目录closeout核验；生产输入相对首轮不变
不等字节可复现，不将此debug证据称正式发行、完整迁移或最终用户验收包。

v359没有新依赖/SDK或预算扩张。字体shorthand和unitless/normal行高复用已有
QuickJS、长度resolver与DirectWrite；证据独立位于linshi/otter-native-v359-font-typography，
静态/最终冻结/六件封存及无重编运行见迁移台账v359。不覆盖v358最终六件或
此前72份，旧78份封存件须保留；不是正式发行或最终用户手测包。SDK仍仅限已批准
隔离构建，不系统安装、修改注册表、分发或默认新DLL，历史未知故障不清零。

最终冻结2,460输入，source SHA-256为
`f8d63b4e5bc236abce4d0fa670034e16150091293395ec906287f712f3926d8f`。
全目标严格静态、3,612文件ratchet和六程序构建通过；16个native变更最大380有效
行，无新例外。封存后21运行门全部通过，JS136/0/13 ignored、typography8/0/1、
八产品worker各1/0、startup7/0等精确成绩见迁移台账v359。debug EXE为42,053,120
字节，SHA-256为`39ce2d62bc6c572ca5c80e6d98f049ecea4bf40f4120e9aa05f2f2ef9b5ff5e2`，
位于`linshi/otter-native-v359-font-typography/artifact/otter-native.exe`。同目录closeout
核验三次源码、六件、旧78份和相关进程；开发失败/不足修正保留，不作正式包、
可复现构建或完整用户验收声明，内部版本不重复分配，旧未知故障不清零。

v358无新依赖/SDK或预算变化。font-family仅走已有DirectWrite/GDI，按需查询本机
字体集合、不下载/安装字体；集合及64项缓存归原文字Engine。证据独立位于
linshi/otter-native-v358-font-family，最终源码冻结/六程序封存与运行成绩见迁移台账。
原有66份及本轮首轮六份共72份封存程序保留；成功子集不结清旧Canvas/启动未知根因，也不等正式发行或
完整用户验收包。SDK仍仅限获准隔离构建，不系统安装/注册表/分发/默认新DLL。

首轮19门18过，JS旧夹具拒绝有效Arial产生两项错误及37项锁中毒，JS90/39/12；
首轮程序和closeout保留。仅改同一夹具sheet/inline两项拒绝值的独立followup完成
冻结/构建，生产输入不变，但未封存、无新六件或运行成绩。随后捕获原始NBSP字节计数缺口，
实际最终构建位于final-admission：values先验字符域、共用fonts夹具增加回归，与
首轮合计三个输入变化，生产JS也变化。最终同源性/封存运行单独留证，仍保护旧72份。

final-admission最终冻结2,453输入，source SHA-256为
`a075f11245bd45e6a70eb59cc8fac9cc173b9547faaf52a94da1d1cfce3601e8`。
fmt、全目标严格Clippy/check、版本门、3,605文件ratchet、产品及五个测试程序构建
通过；18个native变更最大377有效行，无新例外。六件封存后无重编19门全部通过，
JS129/0/12 ignored、七产品worker各1/0及startup7/0等精确成绩见迁移台账v358。
最终debug EXE为42,052,608字节，SHA-256为
`b4c3913c33ca725fb9a00290794ff1ea164b41881b6bbf28b635eeb78d2beb42`，位于
`linshi/otter-native-v358-font-family/final-admission/artifact/otter-native.exe`。
该目录closeout核对三次源码、六程序、旧72份、文档及进程；仅开发留证，不是
正式/最终手测包，不重分配v358或追加版本，不以通过矩阵清除旧未知故障。

v357只增加cfg(test)的跨child固定失败阶段，实际产品协议、原预算和资源owner不变。
证据独立位于linshi/otter-native-v357-js-failure-phases；诊断回归/真实Application/
最终冻结与六程序封存分别记录于迁移台账v357，不复用旧包成绩或把重测成功当修复。
无依赖/SDK变更，仍仅限获准隔离构建，不安装系统SDK、改注册表、分发或默认DLL。

首轮严格静态/构建通过，但20封存门仅19门通过，JS124/1/11 ignored的直接Engine
Canvas模型mount失败未定位。只补接该测试apply错误诊断后，独立补接目录再次
冻结2,447输入，source SHA-256为
`8397fac8416527a0c8e9dbcd348d9261dcda6c86df72eebbf4738e79bad002a6`。
严格静态/3,599文件ratchet/六程序构建及无重编16门通过，三次JS各125/0/11、六产品
worker与startup7/0。最终debug产品41,990,144字节，SHA-256
`e999e84a70beb876d05f5e41c506a923ededd4a7848a2aed3fea135fa682efdb`。
首轮/新轮不同字节分别绑定摘要；旧60封存件、最终三次源码/程序身份与进程查询
见补接目录closeout，成功不清除模型/v356 Application/旧未知故障，整轮仍partial。

v356只调整原CSS公开入口的输入转换位置，并补真实Engine/child/摘要绑定产品的
重入及失败原子性回归；不改QuickJS依赖、原stack/heap/时间/Job/几何/startup门。
证据独立位于linshi/otter-native-v356-reentrancy-stack，最终冻结、六程序封存及
无重编成绩见迁移台账v356。SDK仍仅限获准隔离构建，不安装/注册表/分发/默认DLL；
不是正式发行或最终用户验收包，成功子集不清除旧未知失败。

sheet嵌套解析的值校验还在原values内去除validate/test冗余层，不增加栈预算，
接受/拒绝和规范化由原回归保护；失败诊断及最终同源性分别留证。

v356最终冻结2,445输入，全目标严格静态、3,597文件ratchet和产品/六测试程序构建
通过。六程序封存后19门中18门通过：geometry29/0/1 ignored、JS118/0/11、六产品
worker各1/0及startup7/0；Canvas Application初始回复失败及三项锁中毒保留，partial。
新旧ABBA隔离四次成功不清除该失败。debug产品41,990,144字节，SHA-256
`d8de08e8c098071df7e83ca004c785966adf3b29f06468859e7cfe706bd3fcbe`；
三次源码/六程序/旧48封存件及进程查询见closeout，不代表正式包或完整迁移验收。

v355只修改原JS文档owner的mutation通知和geometry同operation source缓存，不变更
依赖、SDK或原heap/stack/时间/Job/帧/startup门。证据独立位于
linshi/otter-native-v355-geometry-projection；模型fake boxes不当作原生布局证明，
最终静态/构建、六程序封存及无重编运行另记，见迁移台账v355。SDK仅限已批准
隔离构建，不系统安装、修改注册表、分发或默认启用新DLL；不是正式发行包。

v355最终2,443输入冻结/构建后相同，严格静态、3,595文件ratchet、产品/六测试程序
构建通过。封存无重编18门仅16门通过：实际HWND/child/Application及五产品worker
通过、startup7/0，但geometry与JS共享一个新增重入模型栈溢出失败，保持partial。
debug产品41,990,144字节，SHA-256为
`68ab5ac3ae9d75ad6f12bc14c8e6fb7e4e515311028aed962f13ebe13f8a0fb5`。
三次源码/程序/旧42封存件身份及进程查询见closeout；预算未放宽，旧失败不清零。

v354无依赖/SDK或原预算变化；通用timer工厂嵌入同一产品child，TimerTick复用
原单槽/应用等待。证据独立放在linshi/otter-native-v354-timers，最终冻结、构建、
六程序封存和无重编运行分别记录；实际成绩见迁移台账v354，不继承旧绿灯。
旧启动失败保持开放，SDK仅限获准隔离构建，不安装/注册表/分发或默认启用新DLL。

第二轮final-run-v2严格静态/3,594文件行数门与产品/测试构建通过；与首轮只差旧
counter测试夹具，生产输入不变。六程序封存后timer13/0/1 ignored、JS104/0/11及
五个实际产品worker各1/0通过，但geometry10/5/1与startup6/1失败，18门仅16门
通过，保持partial。最终debug产品41,986,048字节，SHA-256
`72b89349ef3bb7a136ec377f5ecc2db067f2547a1d0e41c823c7ece515f6bdba`。
完整同源性/旧36封存件/进程查询绑定本轮最终closeout，不冒充可复现或可发布包。

v353继续定位启动失败：可配对计时和管道错误隔离不改默认行为或原child15秒门。
旧隐藏验收序列迁入startup_smoke.rs以控制入口文件规模，场景/断言/释放顺序保留。
linshi/otter-native-v353-startup-waits中的diagnostic-artifact仅用于初步分段观察，
不是最终候选；后续封存件必须与自身源码/构建和运行门绑定，不沿用该程序成绩。
无SDK/依赖、默认能力或系统安装变化，旧失败与SDK许可边界不变。

v353最终严格静态/3,587文件ratchet、产品和六测试程序构建通过；2,435输入冻结，
六程序封存后无重编执行相关门。计时5/0及资源回归通过，startup6/1仍超原15秒，
整轮partial。最终debug产品41,959,936字节，SHA-256
`c5933ba689ad88f271a144f74a19b7a0c0b89f4f4b2744830f5026b398eca4ab`。
源码/旧24封存件/初步诊断程序和进程查询见本轮closeout，不以零项筛选或原诊断
产品冒充最终运行证明；没有放宽期限、提交推送、发行或交付全面手测。

v352 Canvas2D沿现有WIC/Direct2D/DirectWrite和PNG依赖，不安装系统SDK或增加依赖。
child背板/命令/PNG和父侧像素均有界；原heap/stack/250ms/父两秒/Job/4MiB帧门
不增。证据独立放在linshi/otter-native-v352-canvas2d，实际源码/构建/封存/运行状态
见迁移台账v352；内部debug不是默认JS、正式发布或完整用户验收包，旧失败不清零。

v352第二轮final-run-v2严格静态/3,586文件行数门和产品/测试构建通过，冻结2,434
输入。六程序封存后16门中15门通过，Canvas14/0/1 ignored、JS94/0/10及四个摘要
绑定实际产品worker各1/0；startup6/1在原15秒门失败，整轮partial。debug产品为
41,901,056字节，SHA-256
`d3e20dd3b4ba783f17c2e040a7550c5eb88c88d741f4b26f1f135f53939abe22`。
完整同源性、旧封存件身份及进程查询见该目录closeout；不复用首轮或旧版运行成绩，
不以重试覆盖超时，未交全面手测或发行。

v351 新增 observer 私有工厂、原生内容盒和消息代次排程；无新 SDK/依赖/默认能力，
原 heap/stack/250ms/父两秒/Job/startup 门不增。证据目录为
linshi/otter-native-v351-resize-observer，源码门与真实产品 EXE/测试库封存分别绑定，
实际验证状态和摘要见迁移台账 v351；旧程序不覆盖，旧失败不因新切片通过清零。

v351最终2,421个构建输入在冻结/构建后/运行后一致；全目标严格静态、版本、
3,573文件ratchet和产品/测试构建通过。六程序封存后无重编执行11个选定运行门
全部exit0，含resize13/0/1 ignored、JS82/0/9 ignored、摘要绑定的resize/geometry/
animation实际产品worker各1/0及startup7/0；集合有重叠，未另跑ignored不计通过。
debug产品41,195,520字节，SHA-256
`a09c4f48f847595e7e8f7d4e4f7193e395d10c7f5b35504dfa2823510f90b120`。
六程序构建件/封存件和旧v350封存件身份均复核，相关测试进程残留零；逐门/源码
身份见本轮closeout.json。这不是全库回归、完整迁移、正式发布或最终用户验收包。

v350 续进仅修复测试的成功回复/线程终态等待，并补测试专用几何阶段和批次收据
诊断；新证据在 linshi/otter-native-v350-runtime-investigation。旧 v2 六份封存件保留，
旧产品摘要不宣称对应新诊断源码；不增加依赖、SDK授权、超时或默认能力。此次测试库
构建与定向诊断不是新的产品发行/完整运行门，实际结果见迁移台账的 v350 续进。
随后修正 stale-frame 测试把 inactive 清理计入“不得发事件”的错误断言；新封存测试库
在 runtime-investigation-v2 下通过七套定向门，并实收 inactive。原失败原件保留，
产品EXE未重建，旧child/geometry/TLS/UIA及v348启动未知根因不因本次成功结清。

v350 将私有 animation 工厂和同源 Instant 时钟嵌入同一持久 child，复用原单槽/
消息等待，无新线程/SDK/依赖或默认能力。heap32MiB/stack256KiB/JS250ms/父两秒/
Job128MiB 和 startup 每 child15秒不增。证据在 linshi/otter-native-v350-javascript-animation-v2；
严格静态/相关回归/全部 Cargo 分阶段有界执行，之后封存六程序并无重编运行，
实际产品 worker 显式绑定 EXE 与外部预期摘要，旧 .339–.349 件不覆盖。最终身份/
结果见迁移台账 v350；早期编译失败保留，原 v348 startup 根因仍未定位/修复。
SDK 许可仍仅限 linshi 内 Otter 隔离构建，不安装/注册表/分发/默认 DLL；debug 不是
发布或最终用户验收包，rAF 不是 ResizeObserver/Canvas/真实股票/Loom 完成证明。

v350最终debug产品41,174,528字节，SHA-256
`115b37853128459a700a1ffc74daf81b58313fba4049742e01f4150c43c9b489`。
2,415源码输入冻结/构建后和六程序/旧.339–.349件身份一致；严格静态189.69秒、构建
268.08秒通过，但封存运行652.00秒exit1，animation4/5/1及其他child/geometry/TLS
集合失败，UIA回归也未全绿。摘要绑定实际产品animation等七个worker单项及startup7/0
通过不清除失败；所有预算不增，身份/矩阵/未定位根因与最终查询见迁移台账v350和
partial closeout，不能推广成可交全面用户手测或正式发布的包。

v349 将 spatial 私有工厂嵌入同一产品，保持原 heap32MiB/stack256KiB/JS250ms/
父两秒/Job128MiB 门；不增加 SDK/依赖或默认 JS/能力。真实 child/HWND/Application
和产品 worker 不能由 Node syntax 代替。证据目录为 linshi/otter-native-v349-javascript-spatial，
按冻结→严格/回归/全部 Cargo→六程序封存→无重编运行；产品门必须绑定外部预期摘要。
旧 .339–.348 封存件不覆盖。原 v348 startup 失败仍开放，固定三轮7项通过不是根因
修复；本轮结果见迁移台账 v349/closeout。SDK 授权仍仅限 linshi 内 Otter 隔离构建，
不安装/注册表/分发/默认启用 DLL；内部 debug 不是发布或最终用户验收包。

v349实际debug产品为该目录artifact/otter-native.exe，41,141,248字节，SHA-256
`ff53aa6ab6a513e872f9c84a8ab82b1d4e5699aea878ceef31c7500860d0d817`。
严格源码/编译总门838.91秒通过；六程序封存后无重编运行总门254.05秒通过，含
spatial11/1 ignored、实际产品spatial worker、Application JS10及原预算/资源/
startup7/0。startup整套外层22.05秒，原单child15秒不变。2,407源码输入及六程序
目标/副本身份一致，旧.339–.348产品不变、相关进程残留零；原v348超时根因仍未
定位/修复，本轮成功不清除旧失败。完整证据和非验收边界见迁移台账v349/closeout。

v348 将 geometry 私有 JS 工厂和同一原生测量 callback 嵌入产品；真实 HWND 测试不能
由 Node syntax 或离线假 viewport 代替。源码冻结后完成严格静态/相关回归和全部
Cargo，再封存六程序、无重编运行 geometry/实际产品 worker/原 JS/Application/预算/
资源/startup；产品门显式绑定 EXE 与外部预期摘要。证据目录为
linshi/otter-native-v348-javascript-geometry，最终结果见迁移台账 v348；旧 .339–.347
封存件保留。原 heap32MiB/stack256KiB/JS250ms/父两秒/Job128MiB 均不增，栈问题
压平调用修复。无依赖/SDK/系统安装/注册表/分发或默认 DLL/JS 变更；SDK 授权仍
仅限 linshi 内 Otter 隔离构建，内部 debug 不是完整迁移/正式发布/最终用户验收包。

v348实际debug产品为该目录 artifact/otter-native.exe，41,137,152字节，SHA-256
`9fb8561e8ea7c8101196ef013bfe6004b26e0a62458c8973c4a2376fd3e48da8`。
2,400源码输入冻结/构建后/运行后一致；六程序目标/副本和旧.339–.347产品不变，
相关进程残留零。严格源码门840.34秒通过，封存运行267.14秒exit1：geometry/实际
产品JS worker/Application/预算/资源门通过，但startup4/3，三个smoke超原15秒。
后来同字节单场景诊断通过不能覆盖失败矩阵，根因未定位，closeout保持partial；
不能称整轮runtime通过或交付最终用户验收包，完整事实见迁移台账v348。

v347 将两个私有 Grid JS 工厂嵌入同一产品，Node syntax 不能代替真实 QuickJS/
child/HWND。保持 32 MiB heap/256 KiB stack/250 ms/父侧两秒/Job 128 MiB 原门；
栈耗尽改非递归/压平解析，不以提高预算修复。按冻结→全部 Cargo→六程序封存→
无重编 Grid/旧 JS/Application/预算/资源/startup 运行，实际产品 Grid worker 门
显式绑定 EXE/外部预期摘要。证据在 linshi/otter-native-v347-javascript-grid，最终
结果见迁移台账 v347；旧 .339–.346 件保留。无依赖/SDK 变更、安装/分发/注册表或
默认 DLL/JS/能力放行；SDK 授权仍仅限 linshi 内 Otter 隔离构建，不是发行验收包。

最终实际 debug 产品为该目录 artifact/otter-native.exe，40,910,336 字节，SHA-256
`9d2188d88eb77eeed691f00d7cdd0aee254e52b8d5de04c716d90ffc98a5190b`。
2,391 源码输入冻结/构建后/运行后一致；六程序目标/副本与旧 .339–.346 产品摘要
不变。无重编的 JS Grid 17/1 ignored、实际产品 Grid/CSS/文档/旧 worker、Application
七项/预算三轮/资源/startup 门通过，相关进程残留零。startup 整套 17.09 秒，原单
child 15 秒不增；完整静态/回归、源码摘要与失败保留见迁移台账 v347/closeout。

v346 没有依赖/SDK/工具链变更。原生 Grid 仍由同一 Cargo 产品消费；定向隐藏 HWND/
DirectWrite/D2D 测试不代替产品重建。按源码冻结→相关 Cargo 门→六程序封存→无重编
运行 Grid/JS/预算/资源/startup 的顺序验证，不能沿用 .345 摘要；最终结果见迁移台账
v346 和 [Grid 合同](SURFACE_GRID.md)。旧 .339–.345 封存件保留；默认 capability/JS
不扩大。SDK 仅限 linshi 内 Otter 隔离构建，不系统安装、改注册表、分发或默认启用
新 DLL；内部 debug 不是发布/迁移验收包。

v346 实际 debug 产品封存于 linshi/otter-native-v346-grid/artifact/otter-native.exe，
40,902,144 字节，SHA-256
`30e1f92d53577a198d1e3066516fb19b2acd69c21b21c4c25faa8a2cfce3d35b`。
六程序封存后 Grid 25、Application JS 七项、文档/CSS、实际产品 worker、三轮预算
与资源/startup 门全部通过，运行阶段无 Cargo。2,383 源码输入冻结/构建后/运行后
相同；六程序和旧 .339–.345 封存产品摘要不变，相关进程残留零。startup 整套
17.22 秒，原每 child 15 秒不变；完整结果、失败保留与非验收边界见迁移台账 v346。

v345 的五个 CSS 工厂也由同一 engine concat! 内嵌；Node 只作 syntax/诊断，不能代替
实际 QuickJS child、生产 HWND/DirectWrite/D2D 像素或产品 worker 门。没有 SDK/依赖/
工具链变更；CSS 子集不默认启用 JS。源码冻结后执行全部 Cargo，封存实际产品及五个
测试程序，再无重编运行 Application/文档/CSS/预算和资源/startup 门；CSS 产品门显式
接受封存 EXE 和外部预期 SHA-256。旧 .339–.344 封存件不改写，结果见迁移台账 v345。
SDK 授权仍仅限 linshi 内 Otter 隔离构建，不系统安装/改注册表/分发或默认启用新 DLL。

v345 最终 v2 产品封存于
linshi/otter-native-v345-javascript-css/artifact/otter-native.exe，40,653,824 字节，
SHA-256 `6af4287d587572eea6ff44e34224298320fbdff96dedf908fee7c9deb2d6df9b`。
2,373 源码输入的冻结/构建后/运行后身份一致；六程序封存后 Application JS 七项、
文档 11/1 ignored、CSS 12/1 ignored、显式实际产品 CSS/文档/旧 worker 各一项、
预算三轮各两项及资源/内嵌/startup/裸入口 2/1/7/2 项均通过，运行期间无 Cargo。
startup 整套 18.07 秒，原单 child 15 秒不增；CSS 产品小夹具 Job 峰值 4,800,512
字节，不能外推完整产品内存。六程序及旧 .339–.344 封存产品摘要保持一致，相关
进程残留零；v1 未封存/推广，完整产品/股票/Loom 与迁移仍未验收，精确结果见台账。

v344 不改变依赖/SDK/工具链。五个私有 JS 工厂与 bootstrap 由 engine 的 concat!
作为同一编译输入组合；Node syntax 只检查语法，真实语义/预算必须由 QuickJS child
验证。递归/RegExp 嵌套的栈耗尽通过压平实现修复，不提高原 256 KiB/250 ms/两秒门。
本切片仍按源码冻结→全部 Cargo→六程序封存→无重编运行验证；新增实际产品文档
worker 门须显式传入封存 EXE 绝对路径和外部预期 SHA-256，不自选缓存授权。
旧 .339–.343 封存件保留，结果与证据见迁移台账 v344。SDK 仅在 linshi 内用于 Otter
隔离构建，不系统安装/改注册表/分发或默认启用新 DLL；内部 debug 不是发布/迁移包。

最终 v2 产品：linshi/otter-native-v344-javascript-document/artifact/otter-native.exe，
40,629,248 字节，SHA-256
`93ee5846be2c58597f1109adc594ebb380ee468b42fb472de446446c1b36ec2e`。
六程序封存后 Application JS 七项、文档子集 11/1 ignored、显式实际产品文档 worker
一项、旧产品 worker 一项、三轮预算各两项和资源/startup/裸入口 2/1/7/2 项均通过。
startup 总计 18.50 秒，不放宽每 child 15 秒；源码冻结/构建后/运行后、六程序目标/
副本及旧 .339–.343 产品摘要一致，运行期间无 Cargo。v1 测试 identity 失败未推广，
完整产品/Loom/股票与迁移仍未验收，精确结果及失败保留见迁移台账 v344/closeout。

v343 不改变依赖/SDK/工具链；显式受限 JS 的 Application listener/FIFO/收敛/update
复用同 child 和原授权/资源/批次 owner，默认 JS 仍关闭。测试同步覆盖原批次 10 秒
提交加 25 秒观察预算，不改产品期限或 JS 的 250 ms/两秒；业务 state 解析缺陷及
五秒夹具失败保留，不能用重试覆盖。最终源码/编译/运行须分别冻结和绑定摘要，全部
Cargo 结束后才封存六程序并无重编运行；旧 .339–.342 封存件保持不变。
本切片结果与证据见迁移台账 v343；SDK 仍仅在 linshi 内隔离使用，不安装/分发或
默认启用 DLL，不把内部 debug 产物当作完整迁移、用户手测或正式发布包。
最终 v2 的实际 debug 产品封存于
linshi/otter-native-v343-javascript-input/artifact/otter-native.exe，40,604,672 字节，
SHA-256 `82a2ee8977ab3b92975ec0354af3746c4e6c64ecede76132fe775456a481c44d`。
六程序封存后无 Cargo 重编：Application JS 七项、预算固定三轮各两项、实际产品私有
worker 一项及资源/内嵌/startup/裸入口 2/1/7/2 项通过；startup 整套 17.65 秒，原每
child 15 秒和 JS 250 ms/两秒不变。源码冻结/构建后/运行后、六程序目标/副本和旧
.339–.342 产品摘要一致。v1 静态失败集合保留但未封存/推广；没有完整桌面库、默认
产品 JavaScript/真实 Loom 或迁移放行，精确边界见本轮 closeout 与迁移台账。

v342 不改变工具链/SDK/依赖；Application 的持久后台 JS 挂载复用原 prepare，
默认产品 capability 仍关闭 JS，显式测试报告不构成完整功能批准。运行门须在全部
Cargo 编译结束后封存并按同字节执行，不能继承 .341 的产品/测试摘要；旧 .339–.341
封存件不改写。首轮编译/测试协议夹具及结算顺序失败保留，不能以重试覆盖原件。
SDK 仍仅用于 linshi 内 Otter 隔离构建，没有系统安装/分发或默认启用新 DLL。

v342 最终采用 v3 源码集，产品/集成程序编译通过后才封存，运行阶段没有再调用
Cargo。实际 debug 产品位于
linshi/otter-native-v342-javascript-application/artifact/otter-native.exe，40,542,720 字节，
SHA-256 `8e4e622bacefc93f9759e71493ade28b1315012e2b32c0b8470d5d0f5d0a603e`。
六程序目标/副本和旧 .339–.341 产品摘要前后一致；Application JS 四项、预算三轮各
两项、实际产品私有 worker 一项、资源/内嵌/startup/裸入口 2/1/7/2 项通过。startup
整套 33.41 秒，不改原每 child 15 秒；JS 的 250 ms/两秒期限亦不改。v2 验证期间
源码变化已被身份门拦截且未封存/推广；最终 v3 冻结/构建后/运行后身份相同。
该产品仍为不可发布内部产物，默认 JS 和真实 Loom/完整桌面门未通过；完整定向结果、
旧失败与源码边界见迁移台账 v342，不以这次通过追溯放行。

v341 没有依赖/SDK/工具链变更，新增授权入口获取和后台会话 owner，原图片传输复用
不等于准入放宽。JS 定向门包含真 TLS→同 Grant→真实 child→隐藏 HWND/D2D/定向点击
与销毁路径，不是默认产品 Application/真实 Loom。首次预算失败和夹具编译错误原件
保留；等价 UTF-8 计数优化未改变任何期限。实际产品须在全部 Cargo 门结束后封存，
资源/startup/私有 worker 再按同字节核验；详见迁移台账 v341 和
[原生 JavaScript](SURFACE_JAVASCRIPT_NATIVE.md)。SDK 授权不扩大，不组包或发布。

v341 实际 debug 产品位于 linshi/otter-native-v341-javascript-authorized/artifact/otter-native.exe，
40,332,288 字节，SHA-256
`b8d1f6afe4ab9aa6e7975501ea46cbecb8e753b57e3fbcde4c40741f2a120ae6`。
编译结束后封存的六程序目标/副本前后摘要一致，旧 .339/.340 sealed 产品保留；固定
三轮预算各 2 passed，实际产品私有 worker 1、资源/启动/裸入口 2/1/7/2 passed。
startup 总计 25.43 秒，未放宽原每 child 15 秒。该产品 worker 门不经过授权获取/
Application JS UI/FIFO/真实 Loom，仍是不可发布的内部开发产物。

v340 在 Windows 精确加入 rquickjs/core/sys 0.14.0（std-only，无 loader/dyn-load/
rust-alloc），实际编译 vendored QuickJS-NG 0.16.2 C 引擎；没有新 DLL 下载/默认激活，
不改变 SDK 仅限 linshi 内 Otter 隔离构建的授权。私有 JS worker 在正常设置前分流，
无有效 Job 时必须拒绝；父侧仍不执行脚本。API、定向测试和后续缺口见
[原生 JavaScript](SURFACE_JAVASCRIPT_NATIVE.md)。真实五锁 OSV 首轮发现后，仅升级 native
rustls-webpki 0.103.16 / webpki-roots 1.0.11；复扫 native 无返回匹配，整门仍因其他
锁告警失败。两次失败原件与许可证通过回执均留存，不以原生无匹配冒称全门通过。
旧封存 EXE/回执不改写，
正式产品/集成产物必须在全部编译结束后重新封存并分别绑定实际摘要。

候选材料门发现 core/sys crate 未带仓库级 MIT 后，增加精确版本/来源/原文 SHA-256
绑定的 javascript_licenses 补充 owner；同图 rquickjs LICENSE 和 sys/quickjs/LICENSE
均保留原字节。原失败不覆盖，既有全图缺失门、总预算和 false 法律批准不放宽。

v340 实际 debug 产品封存于 linshi/otter-native-v340-javascript/artifact/otter-native.exe，
40,334,336 字节，SHA-256
`64e59b570d05e412d7730e88ee54f7cf30dd1687ade9c5aa508c6de622efe6f4`。
同目录封存 library-test 与四套 integration EXE；无后续 Cargo 重编，真实产品 JS
worker 显式门 1 passed，build_resources/exe_resources/startup/javascript_worker
分别 2/1/7/2 passed，startup 原每 child 15 秒期限保留。六程序及各目标/副本前后
摘要一致，源码冻结身份前后相同；该门不是默认 Application/JS/真实 Loom 或正式包。
全部定向回归/静态/材料合同及未完成项见迁移台账，不借本次通过追溯结清旧失败。

v339 续进已实际重建产品，不改变上一 library-test 回执：封存到
linshi/otter-native-v339-product-loom/artifact/otter-native.exe，38,558,208 字节，SHA-256
`5c49c06ee632ed9023c77f244b2b629ae1ad7ee3514cf46ab42674b59898367b`。
同源码 `cargo build --manifest-path native/Cargo.toml --locked --offline` 及
`cargo test --manifest-path native/Cargo.toml --locked --offline --test build_resources --test exe_resources --test startup --no-run`
通过；随后封存三套集成 EXE，无重编顺序执行 `--test-threads=1`，2/1/7 项通过，
产品及集成程序前后摘要相同。startup 原每 child 15 秒期限不变，旧超时不追溯放行。
主 target 的 cached 产品由本次构建正常更新，不能再要求它匹配旧回执的 mutable 路径；
旧产品已另存 prior-program，旧 sealed 程序不改。菜单→真实 Loom 夹具仍停于前台
准入，未取得产品业务成功；修正后夹具未重跑，完整记录见迁移台账。未改产品源码/
版本或 SDK 安装/许可边界，没有组包、默认 DLL 激活、发布或推送。

v339 仅新增显式 ignored 的真实 Loom Application 门；生产源码/网络期限/默认能力不变，
未重编产品 EXE。新鲜隔离 daemon/原始包和 loopback TLS 夹具的实际通过记录在
linshi/otter-native-v339-real-loom/run-3。真实门 1、resources 17/1 ignored、非 desktop
Surface 606/10 ignored、授权 84、启动 2 及严格 Clippy/check/fmt/版本/3467 文件行数门
通过；首次编译、路径/等待夹具及误指版本脚本的错误保留，不以重试覆盖。原生产品
EXE、完整桌面/UIA、JavaScript、资源长稳及最终迁移验收仍需各自门禁。

v338 修复声明式可选属性的 null 解析，不改变 SDK、构建器或默认能力。定向验证使用
`cargo test --manifest-path native/Cargo.toml --locked --offline --lib surface -- --skip desktop:: --test-threads=1`
（606 passed / 10 ignored），另跑 Scene 11 项及 `attach_images` 两项；没有把该过滤
集合称为完整桌面门。Clippy -D warnings/all-target check/fmt、版本/3466 文件行数门
通过；旧产品/副本/集成九摘要不变，本轮未重编产品 EXE，不声称新的 EXE 矩阵通过。
外部公共库驱动用独立缓存、产品 Common Controls v6 manifest 和生产能力报告源，
消费真实隔离 daemon 的原始 fallback 快照，绑定封存字节及输入摘要后验证隐藏窗口
呈现/未绑定输入关闭/关闭撤销；驱动最初两项错误假设的失败源码/程序/日志全部保留。
该驱动不是产品启动或设备 HTTPS/Application 实际 daemon 链，也不执行 JavaScript。
证据在 linshi/otter-native-v338-regression，完整库 Select/UIA 及旧启动/证据链问题
仍开放，不追加正向重试掩盖失败。无组包、SDK 系统安装/分发、默认 DLL 激活或推送。

v337 只改变隐藏 pinned 工具面板 BUTTON 的首次使用准备，不改工具链、SDK、DLL pin、
默认 Shader 或构建入口。普通工作区和可见事务仍提交前准备；失败按原窗口所有权回滚，
不把延迟创建用作删除测试覆盖或放宽 15 秒期限。新增 HWND 七项（含真实 ShowWindow）、面板初轮 28 项与
composition 102 passed / 4 ignored 已通过；正式运行必须先结束全部 Cargo 门并封存
同字节，再固定三轮正向和六套无编译集成。证据在 linshi/otter-native-v337-regression；
旧 v335/v336 失败、完整桌面/长稳及最终验收仍开放，没有组包、系统安装或 SDK 分发。

最终严格门及面板 29、composition 102/4 ignored、Shader 65/15 ignored 通过；固定
Shader 三轮 3/3、六套 EXE 26/26、拒绝 3/3，九摘要一致。追加诊断后的 gates.json
与四份诊断材料出现全零/NUL 尾段，原件不覆盖、原因未证；原索引不再可用，不能
伪造重建或声称完整证据链通过。本轮 partial，独立现态/日志摘要复核入口在
linshi/otter-native-v337-recovery，仍不编译、重跑正向或改写已封存产物。

v336 增加默认钉图 D3D11 设备的 UI 线程 Weak 租约池，不改 Cargo/SDK 工具链、候选
pin 或默认 Shader 能力。新真实 GPU/HWND 四项与 owner 五项、composition 102 passed /
4 ignored、Shader 65 passed / 15 ignored、面板 22 项及严格 Clippy/check/fmt/版本/
3,464 文件行数通过。首轮六套 EXE 集成 25 passed / 1 failed，普通完整 smoke 仍在
原 15 秒超时；本轮 partial，不以资源 owner 修复声称完整启动或 N11 验收通过。
所有 Cargo 门结束后先封存实际 EXE，再执行固定三轮正向和六套无编译复验；结果与
九个程序/副本的摘要单独绑定。完整记录位于 linshi/otter-native-v336-regression。
最终三轮正向 2 passed / 1 failed，六套无编译复验 24 passed / 2 failed，拒绝三项
通过；普通与带图片完整 smoke 仍超时。九摘要、最终 2,312 项输入不变，不追加
编译/正向重试，不重标旧失败，也不宣称稳定启动或排除本轮回归风险。

v335 修复真实工具面板的重复 HWND 定位，不改构建工具链/候选 pin 或默认能力。
工具面板 22 项、Shader 65 passed / 15 ignored、严格 Clippy/check/fmt 通过，但预先
固定三轮无计时诊断 Shader 正向门为 2 passed / 1 failed，原 15 秒期限未改。
收尾额外 Cargo build 改写产品 EXE 的情况由摘要断言发现并保留；新字节先复制到
linshi/otter-native-v335-regression/final-artifact，再单独执行一次身份验证和集成
复验，不能用此结果覆盖原三轮失败。本轮仍 partial，不称为稳定启动或完整迁移通过。
后续构建可能重写 target 中的产品，运行证据必须与各自实际 SHA-256 绑定；新产物
须先封存再运行，测试后不得把另一次编译的字节冒称已验收。详情见
[本地 Shader 候选](SHADER_LOCAL_CANDIDATE.md)，无组包/发布或系统 SDK 安装。
最终封存 EXE 单次正向身份验证、拒绝三项、六套集成 26 项通过，九个程序/副本摘要
不变；版本、3,461 文件行数、六文档 UTF-8/本地链接和 diff check 通过，最终 build/
产物验证前后 2,309 输入相同。原三轮中的失败仍使本轮保持 partial。

v334 的实际产品支持显式 `--shader-candidate`，只读消费代码固定的 v332/v333 DLL，
不从材料自动导入摘要，也不安装/复制/持久启用。正常、截图、会话和 smoke 共用
启动准入；默认仍 `Installation=None`。目录必须从固定卷根之后一直处于可信私有
权限下；本机 Public/linshi 不满足门槛，不自动收紧现有 ACL。详细命令、两文件
pin、历史句柄/reparse 边界及真实 EXE 正向门见 [本地 Shader 候选](SHADER_LOCAL_CANDIDATE.md)。
实际首次取码/两 child/正式 PNG 与缓存有通过运行，但最终默认计时正向门仍在原
15 秒超时；中间复验/最终一次计时诊断通过不替代正式失败，本轮 partial。最终
Shader 65 passed / 15 ignored，参数/默认拒绝各三项、严格 Clippy/check/fmt/版本/
3,459 文件行数及最终 EXE 摘要绑定六套集成 26 项通过，八程序身份不变。
不放宽期限、删覆盖或结清旧失败；私有 HOME 新夹具保留不删，其余构建/证据在
linshi/otter-native-v334-regression。此入口不是组包或正式安装/许可批准。

v333 的 `scripts/native-candidate/angle_materials.py create/verify` 显式消费已独立
审阅的 stage/build 摘要，收集两 DLL、白名单证据/源码和原字节声明。独立 verify
只需材料目录与调用者另行审核的 manifest 摘要，不需要原 stage/VS/SDK，也不执行
构建工具或快照源码。写入前预算和核验期精确文件/目录集合拒绝额外 SDK DLL/空目录。
命令、失败保留与信任边界见 [ANGLE 工具链](ANGLE_TOOLCHAIN.md)。
真实候选 159 清单文件、109 份声明，独立核验及官方 CycloneDX/SPDX schema 通过；
新鲜外部驱动直接消费材料 payload 的硬件/WARP 六 contexts/18 像素对照、12 成功
child/两无效 GLSL 拒绝通过，零像素差异。只读 Ninja compdb 展开响应不是重新链接。
这是本地材料目录，不是产品 EXE/可信安装或完整 DLL 依赖/许可闭包；所有批准和
安装/激活边界仍为 false。原生程序与默认能力未改，v332 builder 未改、DLL 未重编。
105 Python/9 Node 合同、fmt/版本/3,451 文件行数与 diff check 通过；九新文件
30–120 有效行，未加例外。34 Python AST/七文档 UTF-8/本地链接通过，原生 1,767
项输入及旧 EXE 不变，相关进程零残留；完整桌面库未重跑，旧失败不追溯放行。

v332 的 `scripts/native-angle/build_fixed.py` 是独立的显式构建入口：消费已审阅的
fresh stage 和外部预期 stage SHA-256，实际执行固定 GN/Ninja、Clang/lld 和获准的
SDK 工具。父进程需明确设置 `DEPOT_TOOLS_WIN_TOOLCHAIN=0`；完整命令和边界见
[ANGLE 隔离构建](ANGLE_TOOLCHAIN.md)。环境仍由原 adapter 重建，不继承工具链回退。
构建进程先 suspended、挂入 kill-on-close Job 后才 resume，仅继承三个标准句柄；
超时/日志轮询上限/异常/root 退出都有有界回收，Job 不是权限沙箱。交付 payload 只含
两 DLL，不含 SDK d3dcompiler；后者只由上游 Ninja 复制至获准的隔离构建缓存，不分发。
纯 PE 门检查必需函数的有效可执行地址及普通/delay imports
的动态 CRT；`/MT`、`/WX`、编译诊断与无工作门独立保留。成功构建回执仍不授权安装、
运行、签名或发布，修改 owner 后须全新暂存，不复用旧缓存/覆盖失败或中止现场。

最终源码绑定的新树已实际完成 12,569 文件暂存、GN 513 targets/135 files 和 504 步
完整两 DLL 编译（558.765 秒），后续无工作，编译诊断为零。加固 PE/静态 CRT 与
新显式夹具的硬件/WARP 六 context/18 像素对照、12 成功 child/两无效 GLSL 拒绝
通过，像素零差异；不是实际产品 EXE/Loom/完整 UI 验收。81 合同、fmt/版本/3,442
文件行数门通过，原生输入/旧产品 EXE/默认能力未改。79 个 gitlink 仍有 66 个未提供，
完整默认源码/工具/许可、可信安装、旧 UIA/启动失败和完整迁移门继续保留。

v331 的 `scripts/native-angle/stage_build.py` 已把固定 profile 的源码/工具暂存提升为
仓库入口：从显式本地 Git 对象、固定归档和已有 VS/SDK 配置生成全新外部构建树，
复用原 profile，不下载、不执行构建工具或激活 DLL。严格配置、命令、输出和失败
合同见 [ANGLE 工具链入口](ANGLE_TOOLCHAIN.md)。最终加固树实际有 12,569 个构建
文件；与首轮树逐项内容相同，真实 GN 的 504 条命令计划也逐字节一致。完整 DLL
编译在首轮树实际通过（857.063 秒），最终树没有重复编译。新 DLL 的 PE/静态 CRT
及显式外部驱动硬件/WARP 六 context/18 像素对照、12 成功 child/两无效 GLSL
拒绝通过。54 合同、fmt/版本/3,434 文件行数门通过；不是新产品 EXE 或完整 UI。
79 个 gitlink 中仍有 66 个未提供，完整默认源码/工具/许可闭包、编译执行 owner、
可信安装和实际产品重放尚未完成。SDK/用户程序/默认关闭 capability 的边界不变。

v330 新增 `scripts/native-angle/prepare_profile.py`，复用 adapter 生成固定
Clang/lld + MSVC STL、x64 D3D11 Release 静态 CRT 的 GN overlay；它不获取或
暂存完整源码、不执行工具。真实新实验树已消费输出，GN 513 targets/135 files、
504 命令的完整 DLL 构建及原生 Ninja→GN 自动再生通过，无生成文件 prefix patch。
再生前后 toolchain 字节相同，C++/资源头依赖 5/24/24 与三项失效/恢复门通过；
Clang `/showIncludes:user` 不记录 system headers，不据此声称完整 SDK 自动失效。
新 DLL 的 x64/52 必需导出/无动态 MSVC CRT 检查通过，完整日志无 warning/error
诊断。显式夹具复用已绑定公共库外部驱动，硬件/WARP 六 context/18 像素对照、
12 成功 child/两无效 GLSL 拒绝通过，零像素差异且 child 父进程不加载 ANGLE。
这不是 v330 产品 EXE 或完整应用验收；完整 staging/工具/许可闭包与可信安装仍缺。
没有 SDK 系统安装、用户程序替换或默认 DLL 激活；详情及命令见下述 ANGLE 入口。

Shader 的 ANGLE 供应链另见 [显式 Windows 工具链 adapter](ANGLE_TOOLCHAIN.md)。
该准备器只生成固定上游的数据消费适配目录，不安装或执行 SDK/编译器，也不让 GN
自动改用外部目录。v329 在 linshi 的另行审阅实验树已完成授权 SDK 的双架构真实
工具探针、GN 111 targets、完整 DLL 计划 dry-run 和四个静态 target 的实际编译。
中文 MSVC 的头依赖经实验 post-GN prefix 接缝由 0 修正到 121，失效/恢复门通过，
不保证生成文件再生后仍正确；也不称 static archive 自包含或编译 warning-free。
v329 后续另取得固定 Clang/Chromium rc，实际完整两 DLL 构建与外部公共库驱动的
硬件/WARP 像素和隔离 child 重放通过；C++/资源头依赖为 121/24/24，失效/恢复门
通过。详见上述 ANGLE 入口及其原始回执。仓库并未新增完整 staging/build owner，
GN 再生安全、完整工具/许可闭包、实际产品 EXE/可信安装验收仍缺；44 条 D9002
与首次直接加载卸载断言失败保留，不重标旧 EXE 或默认启用新 DLL。

- Rust MSVC 工具链及链接器。
- Windows 资源编译器 `rc.exe`，或 LLVM 的 `llvm-rc.exe`。
- 默认从 PATH 依次查找这两个工具；只有工具不存在时才回退。工具执行失败
  会使构建失败，不能静默输出缺图标/版本资源的 EXE。
- 可设置 `OTTER_NATIVE_RC` 为明确的资源编译器可执行文件路径；不填命令行
  参数，不经过 shell 展开。显式配置失败不会改用其他工具。
- 当前接线针对 Windows MSVC；不宣称 Windows GNU 或非 Windows 交叉构建
  的 PE 资源已验证。

`native/build_support/windows_resources.rs` 只负责构建期资源生成/编译。
图标复用 `src-tauri/icons/icon.ico` 的品牌资产，不引入 Tauri 依赖。
`build.rs` 将 `.res` 仅链接到 `otter-native` 产品二进制，Common Controls
manifest 保持原有接线；运行窗口图标继续由 desktop/app_icon 拥有。

PE FileVersion/ProductVersion 保留三段公共版本（固定字段第四段为零）。
`version-state.json` 的四段内部版本必须由候选 provenance 与最终 EXE 哈希
另行绑定，不能把公共 PE 版本当作内部构建身份，也不能把未签名产物说成
签名 UIAccess 包。内部构建身份入口见下节；ZIP 组包与解包启动探针已有
证据，完整包级功能、干净机器及签名验收仍未完成。

## 独立内部 Release 候选

`scripts/build-native-candidate.ps1` 不调用旧 Tauri 发布链，不要求清空或提交
当前工作区。输出必须是仓库外尚不存在的目录，且与可复用构建缓存分开：

```powershell
$env:OTTER_NATIVE_RC = 'C:/Program Files/LLVM/bin/llvm-rc.exe'
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-native-candidate.ps1 `
  -OutputDirectory 'C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-v231-candidate' `
  -TargetDirectory 'C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-candidate-build/target'
node scripts/native-candidate/provenance.cjs verify 'C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-v231-candidate'
```

构建固定 `--locked --offline --release --bin otter-native --target x86_64-pc-windows-msvc`。
前后源码快照包括原生源码、锁、资源、构建脚本和四份已校验补丁依赖；元数据
必须实际解析到这些本地依赖。源码、工具/选定环境/外部 Cargo 配置摘要或 HEAD
在构建期间变化会拒绝候选。工具路径可能指向 rustup shim；此记录不是所有
SDK、链接器或 registry 源码的封闭环境证明，也不能发现中途修改后恢复原字节。

`provenance.json` 绑定最终 `payload/otter-native.exe` 与构建记录的字节，允许
dirty 内部候选，但固定 `publishable: false`、`runtimeVerified: false`、
`signed: false`。无签名收据只能检查一致性，不能抵抗收据与载荷一起被改写。
失败留下 `FAILED.txt`，不能通过验证；保留失败目录，重试使用新目录。
入口对缓存目录持有独占文件锁直到候选完成，并在异常退出时释放句柄。
同一缓存只能通过本入口复用；不要同时用裸 Cargo 或其他工具改写缓存，
这些外部写入者不遵守候选锁，不在此并发保护合同内。
当前入口不生成 ZIP，不收集完整分发许可证，不执行包级便携或运行验收。

合同测试需显式指定临时目录，不在仓库内产生合成 EXE：

```powershell
$env:OTTER_NATIVE_CANDIDATE_TEST_ROOT = 'C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-candidate-contracts'
node --test scripts/tests/native-candidate.test.cjs
```

## 内部 ZIP 组包与验证

以下为 v0.2.33.305 历史候选记录；当前 v524 整包见本文顶部。历史构建目录位于
`linshi/otter-native-v305-candidate`，ZIP 位于
`linshi/otter-native-v305-package/Otter-native-v0.2.33.305-windows-x64-internal.zip`。
归档及真正解包后的 298 个文件均已核对，独立 cwd/TEMP/TMP 下 smoke、缺少
图片的 --pin、未知参数三个探针通过，TEMP 无残留。运行证据单独保存在
`linshi/otter-native-v305-runtime/verification.json`，不改写包内 provenance
的 runtimeVerified=false，不将局部探针提升为完整运行验收。

本次 EXE 为 11380736 字节，SHA256 为
`46f08058a490125be347fb5c9f49319a776ac2e758cf0587e686c7f662770463`；
ZIP SHA256 为
`2f9fbed217f12beb559a57f8af29c5d4bc4ad6c44de6ea1b053ff68bd7b4dc9e`。
构建前后源码身份核验及独立归档验证通过。smoke 退出 0，两个无效参数探针
均按预期退出 1；历史文本投送、隔离 SVG/WebP 子进程标记通过。此候选未签名、
不可公开发布，不代表干净机器、完整功能等价或长期运行验收。
新增可编辑粘贴、capture metadata未知字段及新文档绑定标记已在解包EXE实际
stdout中验证；不是仅验证开发目录的debug程序，也不代表系统剪贴板实测。

构建完成后，使用当前源码对应的候选生成全新的分发输出目录：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package-native-candidate.ps1 `
  -CandidateDirectory 'C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-v234-candidate-complete' `
  -OutputDirectory 'C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-v234-package-new'
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-native-package.ps1 `
  -ZipPath 'C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-v234-package-new/Otter-native-v0.2.33.234-windows-x64-internal.zip'
```

旧候选若与当前源码快照不同，组包会拒绝；应重新构建，不改写历史收据。
包内含 otter-native.exe、README.txt、licenses、build-provenance.json 和
package-manifest.json。原始机器环境、Cargo metadata、构建日志和源码快照
保留在构建证据目录，不作为用户分发材料。组包前后验证候选及源码，ZIP
统一使用正斜杠，验证实际文件集合、逐文件大小/摘要及构建/许可记录一致性。
输出不覆盖已有目录，失败目录保留 FAILED.txt；重试必须用新目录。

归档验证不运行 EXE，也不将内部候选升级为正式发布。包级运行探针证据应
另行保存：真正解包到新目录后核对文件，在独立 cwd/TEMP/TMP 下运行；
仅复制暂存目录或仅验证 ZIP 摘要不能代替这一步。干净机器、完整功能、
性能、安全和签名验收仍是独立边界。

## 分发许可证材料明细

`node scripts/native-candidate/licenses.cjs <repo> <metadata.json> <new-output>`
从候选的离线元数据选择 normal/build 依赖闭包，保守包含所有平台，不声称
这些包全部被最终 EXE 链接。dev-only 分支不随原生产品材料收集。
每个包必须有实际许可文本；根项目使用仓库 LICENSE，补丁依赖先通过来源门，
registry 包限制在当前 CARGO_HOME 的 registry/src 下。拒绝路径越界/链接、
未知来源、缺失文本、重复包，以及超出容量或非 UTF-8 的材料。
`inventory.json` 记录许可证表达式、元数据摘要、补丁来源和每份文本的字节摘要。
此命令尚不组 ZIP，也不表示法律、安全或正式发布检查已通过。

## 本地构建与验证

在仓库根目录执行，临时产物放到 linshi；按机器实际安装位置选择编译器：

```powershell
$env:CARGO_TARGET_DIR = 'C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-build/target'
# 仅在工具不在 PATH 或需要固定工具时设置，例如：
# $env:OTTER_NATIVE_RC = 'C:/Program Files/LLVM/bin/llvm-rc.exe'
cargo build --manifest-path native/Cargo.toml --locked --offline
cargo test --manifest-path native/Cargo.toml --locked --offline --test build_resources --test exe_resources --test startup -- --test-threads=1
```

- `build_resources` 检查三段版本、资源文本与路径输入边界。
- `exe_resources` 读取实际产品 EXE 的 PE 图标与版本，不能用窗口运行时图标
  或源 `.ico` 文件存在代替这个检查。
- `startup` 经真实 EXE 入口检查隐藏窗口、双次绘制、透明钉图/合成器、
  child 分流及错误命令行；不代表最终 ZIP 便携性或全部功能人工验收。
- Release 构建可添加 `--release`，仍不自动成为正式发布。

从 v232 起，--smoke-test 的打开/粘贴/拖放/分组锁定场景使用内嵌 PNG，
需要文件路径时在当前 TEMP 下独占创建短生命周期副本，不读取编译机器的
源码图片。验证时应把 TEMP/TMP 指向 linshi 下的独立目录，并检查临时图片
无残留；从独立 cwd 运行不等于干净机器或完整功能验收。

不要把 `Invoke-OtterNativeCandidateAcceptance.ps1` 当作这份独立程序的
包级验收器：现有脚本仍针对历史 Tauri/WebView2 候选及 CDP 探针。

## 显式真实 Loom Application 门

`application_surface_real_loom_tests` 默认 ignored，不能将普通 resources 的跳过项
报为真实服务通过。仅在 linshi 下提供全新绝对路径的 `OTTER_REAL_LOOM_TEST_DIR`：
根目录有 `test-input.json`（`origin` 为 `https://localhost:<port>`、`artId`、`nodeId`），
`otter-profile/fixture-ca.pem` 是该精确 origin 的显式 CA，且没有已存在的设备身份。
使用独立 daemon/control-plane/profile 和已核对摘要的原始 Art/framework/MCP 包；
不连接现有用户 daemon，不导入系统证书或注入 session。测试写 `pairing-ready.json`
后，外部夹具只批准其中新 pending 设备 ID，后续签名/session 与业务请求由生产代码完成。

```powershell
rtk proxy cargo test --manifest-path native/Cargo.toml --locked --offline --lib `
  desktop::application::remote_live::resources::tests::real_loom::isolated_real_loom_pairs_mounts_activates_and_closes `
  -- --ignored --exact --test-threads=1 --nocapture
```

全链等待包含配置/DNS/多地址回退和 HTTP，不复用单地址合成 peer 的五秒 helper；
不放宽生产请求期限。测试只显示自有屏幕外非激活窗口，不发全局键鼠或股票动作；
验证 EDIT/两个 COMBOBOX、两次绘制、inactive ACK 及句柄回收。外部编排必须有界
回收自身 daemon/test 子树，保存真实响应、原始失败和输入前后摘要，但不得保存
admin/session token。v339 的封存 runner/TLS 源码在对应 run 目录，所有输入另经独立
核验；TLS 终结代理不是 daemon 的原生 HTTPS listener，library-test Application
不是产品 EXE/完整菜单入口，也不证明 JavaScript、完整桌面、长稳或最终手测通过。
