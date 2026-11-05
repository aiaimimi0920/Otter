# Windows 原生 JavaScript Surface 执行边界

## v540：密集换行值校验

文本input保原65536 UTF-16/48KiB UTF-8及先验证surrogate、后去CR/LF语义。去换行改用预先捕获的native split/join，私有冻结null-prototype分隔符隔断Symbol.split/toPrimitive；连续自有字符串数组槽不触插件数值原型或species。两次扫描的临时数组最多65537项，中间字符串不超过原raw。原250ms/32MiB heap/128MiB Job不变，非法值不改dirty/defaultValue或产生emit。加强后的28case已在实际child和封存解包产品worker定向通过，原失败和对照诊断仍保留；不代替普通可见键盘/人工IME/完整JS验收。详见[构建记录](WINDOWS_NATIVE_BUILD.md#v540-表单换行热点修复与最新普通daemon)。

## 签名包端口与宿主输入（v533）

Local Art 现在从 Loom 已验证的不可变安装包取得输入/输出的真实名字和类型；Otter 接入目录、图像输入、来源配置凭证、自定义主图片输出继续 Add Art、正式结果及保存重建/有限离线。显式零输入/零输出保持为空，不伪造 input_image/output_image；只有无新证明的旧包保留默认端口。首次自定义主输入迁移在候选 Workspace 内提交，冲突保原图；零输入只撤回本次工厂创建且仍完全一致的暂定边，保存恢复不继承删除旧边的权限。端口保存记录仅是包绑定提示，不是执行/离线授权。

新增精确 Device POST `/v2/art/local/prepare-package`、请求 `loom.art.local.prepare-package.v2`，签名 payload 为独立 `loom.art.local.package.v2` 域，严格包含 protocolVersion、原 package 和 ports；旧 package.v1 与原权限/设备/审批/包/资源/期限保持，不在同协议偷换语义。ports.inputs/outputs 每侧至多32项，名字UTF-8至多128字节、类型64字节，不接受空白/控制字符/同侧重名；端口JSON至多8KiB，并与资源共享原签名payload预算。

执行前沿原Lease/Control读取 `/v1/art/local/issuer`，新增可选 `supportedPrepareProtocols`；响应至多4096字节，必须与独立可信Pin的algorithm/keyId/公钥一致，不从网络自动授信或新建Lease。旧响应缺能力直接走v1，不能先探测未知v2路由；advertised v2 后任何拒绝（包括404）都失败关闭，不降级重放。原v1 package到resume的精确404回退仍保留。

本批最终选定 **83项Rust passed**，其中17项新回归；当前选定终态failed=0，不是整库。签名域/篡改/预算、旧能力协商、拒绝不重放、图拓扑、中文自定义输入、真实2×2红像素、Live配置换代拒迟到、零输入不交付像素、自定义rendered输出继续建子Art及Save/窗口重建/有限离线均已验。真实Loom LAN HTTPS跨进程联合在线和停服后离线独立核signedPorts、完整资源、正式像素和正式结果；不完整缓存拒绝，父子不混加。Otter strict all-targets Clippy、两仓fmt、官方版本/strict行数、Otter lib/普通Release/工作区与Tile驱动构建通过；Loom签名安全/daemon测试库通过。66项源码delta均UTF-8无BOM，Rust最大500有效行，无新增行数例外。两个原跨仓grant门及两个SDK示例默认ignored未运行；Loom整仓strict Clippy旧债未重跑，不称两仓全绿。

用户已恢复仅新profile、自有测试窗口的前台验收。本次普通固定SHA Tile EXE三类图片/Live/Art混合可见门通过；Computer Use只激活工具唯一返回且路径/标题匹配的本批标准启动器，刷新成功后用单会话nonce同步，driver再核真实前台并只向已核自有Tile转交。独立桌面采样核三路换帧、冻结/黑场/恢复、正常关闭及来源附件/值/Live保留与连接回收，不以后台paint或源像素代呈现。没有操作用户旧应用、注入全局输入或修改显示设置；不等完整键盘/人工IME或普通源EXE全部通过。

## 当前目标：源码 Art 与 Otter 单侧执行

### 本地 Live 自动回调（v520，限定联合通过）

已有本地Art加入远程Live后，可在观看系统菜单“配置触发条件”选择“启用或修改 Art 自动触发…”。
复用严格Condition JSON；确认按钮为“允许自动回调”，每次确认生成新的独立authorizationId。
Preview/打开Art/加入观看不会默认自动执行。停止入口先在本地立即撤销gate，再由原串行控制线程减权。
媒体或控制失联先封住旧gate，两条链都恢复也不恢复旧自动授权，必须再次显式配置。

签名Art可声明`onLiveTrigger(event)`，事件只有sessionId/epoch和有界delivery：固定回调名、
deliveryId、conditionRevision、原observation、sourceDeviceId、expiresAtMs；不含Device或grant凭据。
不伪造click、用户gesture或Surface附件，不请求焦点、不恢复隐藏/暂停timer。
服务/正式结果仍沿原权限、原2秒operation及服务期限/资源门；自动回调不是敏感服务批准。
原worker只领取一次payload、同一时刻一个本地回调，未知HTTP结果不重放或从Inspect重造事件。
无handler明确失败并回收原child；停止/过期/宿主换代拒绝迟到帧和正式结果。

`commitResult()`本身仍只表示当前operation单槽排队。只有原Application/Results原子提交成功后
才报告`applied(formalRevision)`；这是本地实例的成功正式提交序号，不是工作区磁盘版本。
无正式输出只在实际呈现后报告no_result。ACK是宿主报告，不证明已经写盘或远端呈现；
保存恢复另由既有Save/Checkpoint验收。Stop与最终canonical事务使用短授权锁，锁内不运行Win32、网络或脚本。
SDK在Loom的`sdk/otter-art`；v520已通过真实child、隐藏Application/Results/Save及真实Loom HTTPS/WSS回执/停止联合，完整87项及7默认ignored边界见本批构建记录，不冒称普通可见产品或最终包通过。

### 持久源码缓存（定向闭环已验证，不是离线授权）

实际OpenArt后台worker在原profile下保存验签后的原始JS，不保存grant、token、输入或私钥。
索引包含origin/device/art/version/packageDigest/SDK及独立可信issuer；不包含当前request/workspace，
因为缓存只复用源码，执行仍须完整的新请求许可。缓存缺失、损坏、独占锁忙或不安全路径只产生miss。
每次仍实时describe，再以新请求获取授权；命中时使用`POST /v1/art/local/prepare-cached`与显式
`loom.art.local.prepare.v2`。只有明确404才一次回退旧prepare；403、网络、绑定/签名错误失败关闭。
响应`source:null`表示使用当前缓存字节，其摘要/大小/UTF-8仍通过原Permit完整检查；缺字段不等于引用。
Loom摘要不匹配时返回完整Base64源码修复缓存，签发前的设备/审批/包/源码复核不省略。
缓存最多32项/32MiB，单项1MiB；暂存最多额外1MiB，同目录原子替换，失败只清理本次拥有的tmp。
不自动淘汰旧项，满额只失去后续缓存优化；磁盘失败不阻断已有有效在线许可。
源码重启读取、Application重建及联合链各自验证，均不能替代跨重启离线授权、完整资源包或最终包验收。
本批230项Otter去重定向通过（含12项隐藏Application），另显式执行真实Loom HTTPS消费者和封存产品Canvas worker。
首次大图快照在原250ms门失败；同debug依赖的独立实验定位PNG Adaptive滤波，改为同Fdeflate的Fastest/Up后，
原512x512高熵Application用例及独立RGBA/全alpha oracle通过，不缩图、不增预算或期限。
Loom15项本地Art HTTP、3项LAN边界和1项联合门通过；严格Clippy仍有未改文件46/55项旧诊断。
完整证据位于main-art-source-cache-1，源码缓存没有持久化或复用旧请求grant。

### 本地图片句柄与Canvas输入（定向闭环已验证）

本地Art的`snapshot.state.inputs[port]`继续提供`resourceId/mimeType/width/height`。
脚本使用`const image = NeuroSurface.image(input.resourceId)`获得当前mount已安装图片的只读句柄，
再调用`ctx.drawImage(image,dx,dy)`、`ctx.drawImage(image,dx,dy,dw,dh)`或
`ctx.drawImage(image,sx,sy,sw,sh,dx,dy,dw,dh)`。支持当前transform/globalAlpha、
imageSmoothingEnabled及save/restore/Canvas尺寸重置；源区越界按比例裁目的区，负宽高不隐式翻转。
句柄由私有WeakMap核验，不接受伪造对象、URL/路径或来自另一个会话的ID；不广告完整浏览器Image/DOM兼容。

宿主复用原输入准备worker的不可变预算PNG，不在消息线程重新解码大图。
首次mount之前沿原私有pipe安装，JSON只含描述及SHA-256；每项PNG最多4MiB、合计8MiB，
最多8图，每边4096/单图4M像素，合计解码输入16MiB。描述尺寸在WIC分配像素前核对。
输入解码像素、每target原生位图与Canvas逻辑背板共用已有32MiB预算，并非额外增加32MiB；
因此多个大图/输出可能因合计超限明确失败。每target同一图片只创建一份原生位图。
安装与mount共享原2秒绝对期限；JS操作250ms、32MiB heap、128MiB child Job、
512KiB内联JSON及原Canvas输出PNG门均不变。失败/取消丢弃整个会话，不重放半帧。

小图仍可经`NeuroSurface.resource(id)`读取原Data URL；大图不进入该内联映射，必须使用图片句柄。
本地Art输入在mount后不可变，来源改变由原宿主owner退休而不是原地替换像素。
正式结果仍需显式workspace_result权限与`commitResult`，不因能绘制图片获得自动保存、网络或投送权。
当前闭环严格Clippy、格式、有效行数门及279项去重定向测试通过，含11项真实隐藏Application。
512×512高熵PNG超过内联门后仍按完整源像素输出；当前crop进入正式结果并保存恢复，来源或工作区退休拒绝旧结果。
其中2项显式封存产品worker门覆盖原状态/事件及Canvas绘制与回收；新图片输入专项由真实test child与隐藏Application证明，未冒称同一产品EXE的图片专项或可见UI已验。
5项既有引擎诊断/压力用例与1项真实Loom联合用例仍未运行；不代表完整图像处理SDK、授权缓存/离线或最终产品包验收完成。

v445 显式信任配置存放于既有应用设置 `artIssuerTrust.issuers`，每项只有 canonical HTTPS `origin`、
`keyId`、标准带 padding Base64 的 32-byte `publicKey`。最多 32 个不同 origin；缺字段配置默认无信任，
无效文档失败关闭。此配置应来自用户确认的独立可信来源，不由 `/v1/art/local/issuer` 响应自动填充。
`settings_update::Prepared::prepare_issuer_trust` 复用设置完整字节基线、保留未知字段及恢复事务；
发布成功后上层才把新文档交给 Authority.replace，不能保存失败却将新 key 激活。
Authority 为进程内 profile owner，Authorization 签出时登记原 Device Lease；密钥轮换、移除或 owner Drop
同步撤销相关 Lease，因此在途 HTTP、已准备结果和已挂载 Local 共用同一撤销链，不影响未变化 origin。
签出只做本地工作，后台网络执行不持信任 owner 借用。64 个活跃登记上限防止无限弱引用累积。
这些接口已供后台 prepare 链路使用，但确认 UI、应用生命周期装配和跨进程设置变更传播仍未接入。

v442 新增 `surface_javascript::signed::Local`：在真实既有 Session 分配前验签并核当前设备 Lease，
每次操作前、管道取消轮询中、成功结果交付前重复检查 Permit。任何失效都终止当前许可并回收自有 child。
它是后台同步接缝，不是新的执行引擎或 UI owner；不改变原默认挂载入口。
当前仅接受 local_state + preview，空服务范围；旧 emit、原生 focus 请求和 loom_service 权限明确拒绝，
不能借用旧 Surface 远程事件链来伪装新服务派发/正式结果。测试脚本使用既有 NeuroSurface 原生适配器，
只证明签名源码可以受控执行；完整 otter.art.v1 SDK、资源包与权限适配仍未完成。
宿主需传入来自当前请求的 Expected、可信 issuer 和视图身份；签名响应不得反向填充这些值。
窗口失活/销毁/附件切换必须通过原 owner 取消；grant Binding 本身不含 Surface 生命周期。
闲置过期/撤权需要 owner 唤醒 refresh，online 为调用方当次状态；没有新增自动联网检测或闲置线程。
离线执行同时受签名离线期限和当前设备 Lease 期限约束，不宣称跨重启缓存支持。

v441 新增独立签名执行许可的客户端准入与会话内期限 owner，合同为 loom.art.local.grant.v1。
Device token/resource lease 不是离线可验签的执行许可，响应自带公钥也不建立 issuer 信任。
当前新增模块尚未接入生产挂载，不改变下述 Surface v1 的 JS/授权语义；跨重启缓存、密钥信任管理、
真实 Art 本地执行/异步服务/保存仍需继续接通，不能将纯合同回归当 A1 端到端完成。

[统一修订计划](WINDOWS_NATIVE_REVISED_PLAN.md) 已替代“Loom 后台 JS / Otter UI JS”及完整旧浏览器兼容前提。
既有 QuickJS/事件/生命周期能力按责任复用，未来源码包获取、SDK、权限和结果 owner 必须版本化接线。
以下历史 Surface 切片不等于新 Art 包架构已实现；不扩充 DOM/CSS 来追求所有旧插件原样运行。

## v408：有界源分辨率编码出口的边界

本轮仅复用原 source-resolution 合成/有界 PNG 出口，不改 JavaScript runtime、Surface capability 广告、
脚本权限或跨端 consumer，不授脚本自动投送权。当前底图接线不等生产 retained-source/目标 metadata/
beautify parity；双向编辑、LiveRelay/Tile、真实双端、资源长稳与完整迁移仍开放。
结果见 [v408 构建记录](WINDOWS_NATIVE_BUILD.md#v408有界源分辨率编码出口)。下方 v407 为历史。

## v407：单目标发送生产接线的边界

本轮只接通固定 Unit 的单目标 SharedLoom 发送菜单、原 Job 后台当前显示 PNG、Binding 与 canonical checkpoint，
不修改 JavaScript runtime、Surface capability 广告、脚本权限或 Surface 跨端消费者。
UI join 不再读完整 outbox；后续网络授权仍需原后台基线。显式重试复用原 Request，不授脚本自动投送权。
当前 PNG 不等 source-resolution/完整 beautify；真实 Loom 双端、双向编辑/LiveRelay/Tile consumers、资源长稳、
完整迁移及最终用户全面手测仍开放；旧间歇呈现失败不因本轮选定门通过而关闭。
实际选定门及产物身份见 [v407 构建记录](WINDOWS_NATIVE_BUILD.md#v407单目标发送生产接线)。下方 v406 为历史。

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

本轮只补传统receiver PNG同步，不改变Surface/JavaScript广告、宿主或默认DLL。
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

本轮只补传统接收的显式回执恢复，不改变 Surface/JavaScript 广告、宿主、默认 DLL 或授权语义。
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

本轮只改传统接收的 Ready 后生命周期和真实保存稳定化，不改 Surface/
JavaScript 广告、宿主或授权语义，不默认启用新 DLL，不扩大 SDK 用途或权限。

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

本轮只改传统接收网络阶段的精确取消交接与原持久清理，不改 Surface/
JavaScript 广告、宿主或授权语义，不默认启用新 DLL，也不扩大 SDK 用途或依赖权限。

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

本轮只接已保存传统接收贴图删除后的持久清理与自动解绑，复用原 Application 保存/授权/四槽和消息等待；不改变 JavaScript/Surface 能力、默认启用或权限，不以局部门结果代完整执行/迁移验收。

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

本轮只接共享 Application 的传统投送显式可见接收链；未更改 JavaScript/Surface 执行能力、默认启用或权限。原保存/恢复/授权与共享四槽继续复用，不以接收链通过推断完整 JavaScript/Surface 或迁移验收。

最终严格静态/3799 文件 ratchet 零违规/警告/新例外；同源八件 45 选定门全过。
24 变更/7 新增、最大 381 有效行，UTF-8 无 BOM；visible 3/0、present 5/0、
placement 2/0、确认 1/0，过滤组不累计覆盖。344 旧件/七诊断、开发失败/输入说明、
最终冻结输入/七文档与源码三次匹配见迁移台账 v393/closeout。startup 7/0 不关旧根因，
正式前台菜单仍 ignored；无真实时钟到期/worker panic 全链注入，不代完整迁移。

v392 只接 N09 唯一接收准备/隐藏追加/精确 checkpoint 的原 Grant 生命周期，不改
本模块能力广告/默认挂载、权限/FIFO/前台/预算，不执行工作流或自动重发请求。
错误/取消保本地内容与 journal；隐藏保存不等于可见呈现/displayed。
最终严格静态/3,792 文件 ratchet 零违规/警告/新例外，同源八件 41 选定运行门全过；25 变更/6 新增、最大 454 有效行，UTF-8 无 BOM。
owner 10/0、watch 2/0；过滤组不累计覆盖。四份开发输入、最终 2,640 冻结输入/
七文档、首个静态候选/失败程序/析构诊断、336 旧件/五诊断与后续 344 件/七诊断保护
见迁移台账 v392/closeout。startup 7/0 不关旧偶发根因，前台菜单仍 ignored；
旧 return/前台/完整组/启动/压力根因、完整
JS/真实 Loom/总资源长稳/最终用户包仍开放，不发布或替换程序。

v391 只补 N09 接收意图↔Unit 关联与真实 checkpoint 包含，不改 JavaScript 能力
广告/默认挂载、权限/FIFO/前台或预算，不执行工作流/自动发网。菜单仅分本地已
保存/未结，不等于可见 present/displayed。最终严格静态/3,786 文件 ratchet 零违规/
警告/新例外，同源八件 39 门全过，28 变更/4 新增、最大 349 有效行，UTF-8 无 BOM；
关联模型 5/0、checkpoint 新组 5/0/全组 18/0、恢复菜单 4/0，过滤组不累计覆盖。
四份 exact 开发输入、最终 2,634 冻结输入/七文档、首个静态候选和 null 兼容修正后
final-run2、328 旧件/五诊断及后续 336 件/五诊断保护见迁移台账 v391/closeout。
startup 7/0 不关旧偶发根因，正式前台菜单仍 ignored。旧 return/前台/完整组/
启动/压力根因、完整 JS/真实 Loom/总资源长稳/最终用户包仍开放，不发布或替换程序。

v390 只接 N09 默认 profile 后台 journal 与显式未结投送解绑消费者，不改本模块
能力广告/默认挂载、权限/FIFO/前台/预算，不自动执行工作流或重发接收。终态
解绑保本地图片，最终严格静态/3,782 文件 ratchet 零违规/警告/新例外，同源八件
37 选定门全过；19 变更/5 新增、最大 453 有效行，UTF-8 无 BOM。profile 4/0、
菜单 3/0、Application 解绑 8/0，过滤组不累计覆盖；两份开发输入、2,630 冻结
输入/七文档、初始空过滤/夹具失败程序、320 旧件/四诊断及后续 328 件/五诊断保护
见迁移台账 v390/closeout。startup 7/0 不关旧偶发根因，前台菜单仍 ignored；旧
return/前台/完整组/启动/压力根因、
完整 JS/真实 Loom/总资源长稳/最终用户包仍开放，不提交/推送/发布或替换用户程序。

v389 只补 N09 接收写前意图与显式认证解绑内核，不改 JavaScript 能力广告/默认
挂载、权限/FIFO/前台或预算，不自动运行工作流。接受菜单、可见 present/displayed
和退出补偿消费者仍未接，不能用 journal/TLS 或隐藏 HWND 测试替代完整业务。
本轮严格静态/3,777 文件 ratchet 零违规/警告/新例外，同源八件 34 选定门全过，
16 变更/6 新增、最大 440 有效行，UTF-8 无 BOM；journal 14/0、认证补偿 9/0、投送
客户端 49/0，过滤组不累计覆盖。六份开发输入、最终 2,625 冻结输入/七文档、
312 旧件/四诊断及后续 320 件保护见迁移台账 v389/closeout。startup 7/0 不关闭
旧偶发根因，前台菜单仍 ignored。旧 return/前台/完整组/启动/压力未知失败、完整
JS/真实 Loom/总资源长稳及最终用户包门仍开放，不提交/推送/发布或替换用户程序。

v388 只补 N09 传统接收准备内核和共同单贴纸追加，不改变本模块的能力广告/默认
挂载、权限/FIFO/前台或预算，不自动执行 Loom 工作流。严格静态/3,771 文件 ratchet
与同源八件 32 个选定后台/隔离门通过，接收隐藏 HWND/落盘恢复不等于真实显示或
displayed。31 变更/12 新增、最大 367 有效行，UTF-8 无 BOM；首轮 Clippy 大枚举
失败/exact 输入、Box 化后 final-run2、304 旧件/四诊断及后续 312 件保护见迁移
台账 v388/closeout。startup 7/0 不关闭 v387 偶发根因，前台菜单仍 ignored；旧
return/前台/完整组/压力失败、完整 JS/真实业务/总资源长稳及用户包门仍开放。

v387仅接N09传统投送桌面元数据查询/拒绝菜单及生命周期消费者，不改JavaScript
能力广告/默认挂载、权限/FIFO/前台/预算，也不自动执行Loom工作流。最终同源相关
门见迁移台账v387；旧return/前台/完整组/压力未知失败、完整JS/真实业务/总资源
长稳门仍开放，传统accept/下载/呈现/displayed/持续同步/离线互信/journal/Tile仍缺。

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

v386只补普通文档窗口几何变化的有界结算唤醒，不改JavaScript能力、默认挂载、
权限/FIFO/前台或预算；保DefWindowProc默认尺寸派发。最终相关门见迁移台账v386，
不关闭旧return/前台/完整组/压力未知失败或完整JS/真实业务/资源长稳门。

v386最终final-run严格fmt、全目标Clippy -D warnings/check、版本及官方ratchet
均过，零违规/警告/新例外；7份native变更/3新增，最大479有效行，UTF-8无BOM。
同源八件21个选定后台/隔离门全过：wake3/0、checkpoint8/0（含两项新隐藏HWND），
原workspace_runtime2/0/1 ignored完成移动提交→强制中断→重启恢复→删除最后Unit
落空；保原15秒和断言。正式菜单退出仍ignored未验，不关闭v384前台或旧完整组/
压力失败。三次源码/288旧件/四诊断/v385完整失败输入及原现场由closeout复核，
后续保护296件；状态window-geometry-wake-and-workspace-runtime-verified，不代完整迁移。

v385仅实现N09传统投送inbox/rejected的认证客户端与单次后台Job，未接桌面接收；
不改JavaScript能力/默认挂载、权限/FIFO/前台或预算。最终门见迁移台账v385，不
关闭v384菜单前台、旧return/完整组/压力未知失败或完整JS/真实业务/资源长稳门。

v385最终严格静态及13门中的12门通过，workspace_runtime在移动检查点原期限失败，
不自动重试或延长超时，默认ignored不计通过；本模块能力
及旧前台/完整组/压力门不变。源码/八件/旧280件/四诊断、首轮静态失败完整输入
与288件后续保护见迁移台账v385及closeout，不扩大JavaScript/完整迁移验收结论。

v384仅修N06正式菜单退出测试的真实Static类名发现，不改JavaScript能力/默认挂载、
权限/FIFO/前台/预算。开发正式菜单退出1/0及最终同源验证见迁移台账v384；不关闭
v379前台return、旧完整组/压力未知失败，亦不代完整JS/真实Loom/资源长稳或用户包。

v384最终严格静态/3,742文件ratchet和18后台门通过；19门中的独立正式菜单0/1在
原15秒前台前提失败。开发1/0不替代最终同源原生批准，默认ignored不计通过；完整
失败输入/八件/三次源码/旧272件/四诊断与280件后续保护见closeout，旧return/完整
组/压力门及本模块能力边界不变，不继续循环争抢前台。

v383仅修N06正常退出最终检查点/Live一次性批准；不改JavaScript广告/默认挂载、
权限/FIFO/前台或预算，不自动执行Loom工作流。最终定向验证状态见迁移台账v383；
旧前台return/完整组/压力未知失败、完整JS/真实业务/OS隔离/总资源长稳及最终用户
包不由本轮保存结果关闭。

v383首轮17/19门通过，startup4/3和产品前台0/1的失败分别保留；后续只分离无profile
退出批准与最终profile快照空闲，不改变本模块权限/FIFO。修正候选不重复争抢前台，
不将ignored当通过或继承旧字节原生结论；旧return/完整组/压力边界仍不关闭。

v383最终final-run2严格静态/3,741文件ratchet及18选定后台/隔离门全过；退出11/0、
相邻Live58/0为包含关系。startup7/0只关闭本轮退出兼容性回归，菜单退出1 ignored
仍未验，状态exit-checkpoint-native-pending。三次源码/八件/旧256件/四诊断及首轮
失败完整输入/八件和272件后续保护见迁移台账/closeout，不扩大本模块能力或旧门结论。

v382仅继续N06的Live时间点检查点/显式文件自动保存与多Live意图生命周期，不改变
JavaScript能力广告、默认挂载、权限/FIFO/前台或预算，不自动运行Loom工作流。
最终源码/程序/相关门见迁移台账v382；旧前台return/完整组/压力未知失败和完整JS/
真实业务/OS隔离/总资源长稳/最终用户包仍开放，不由N06后台结果关闭。

v382最终17个N06/Live相关门全过，无ignored，严格静态/3,735文件ratchet通过；三次
源码/八程序/旧248件与四诊断、256件后续保护见迁移台账/closeout。没有重跑或关闭
旧JavaScript前台/完整组/压力门，不将Live静态时间点成功当默认JS或真实Loom验收。

v381推进N06默认静态工作区检查点/启动恢复，复用原保存/恢复owner；不改变本模块
产品能力、默认挂载、权限/前台/FIFO或预算，不自动执行Loom工作流。最终定向门与
身份见迁移台账v381；v379前台return、旧完整组/压力、完整JS/真实Loom股票/OS隔离/
总资源长稳/最终用户包均不由本轮N06结果关闭。

v381最终14个N06相关运行门及严格静态通过，含真实普通启动/移动检查点/强制终止/
重启恢复和最后Unit清空；源码/八程序/旧240件及四诊断见迁移台账和final-run2/closeout。
未重跑旧前台门，不将静态工作区闭环扩大为JavaScript默认能力或真实Loom业务验收。

本模块沿用旧 `NeuroSurface.define({ mount, update, suspend, resume, dispose })`
模块入口，实际求值 ES module 与 Promise，不把插件改写为返回 JSON 的协议或固定
Rust 业务逻辑。显式开发协商的文本/按钮 DOM 子集可进入 Application 挂载/生命周期，**不广告完整
`surface.javascript.v1`，不接入默认 attach，不替代旧 WebView 产品**。

## v379：测试前台启动与许可交接候选

v380转向N06静态工作区自动保存，不改变本模块的产品/测试能力或预算，不自动执行
Loom工作流。v379新增前台return路径仍未通过，旧前台/完整组失败不由N06后台结果
关闭；完整JS与真实Loom/股票、OS隔离/总资源长稳和最终用户包继续未验收。

v380的10个N06相关运行门及严格静态通过，只证明本轮自动保存/保存恢复与真实产品
启动链；没有重跑或关闭v379前台return候选，也不改变JavaScript能力广告/默认挂载。
精确源码/程序/旧234件与四诊断保护见迁移台账v380和final-run2/closeout。

产品focus/keyboard/FIFO/授权准入不改。原测试fixture增加显式可选的就绪/G握手，
Application在原TLS计时前建立测试启动前提；自有Child owner只向精确父/Child PID
交接前台许可，不以线程SetFocus或AttachThreadInput当作持久前台权限。归还命令仍
由当前前台Child处理，原生命周期/输出/队列/确认预算不扩。真实自有点击的诊断与
失败分别封存；最后实际用户输入打断父前台，新增return路径未到达。四项前台最终
门未执行，保持foreground-handoff-native-pending，不关闭旧失败或宣称产品修复。
后台/静态和身份见迁移台账v379；默认完整JS、真实业务及完整迁移/用户包仍未验收。

最终八个后台门全过，Engine121/0/5 ignored、Session77/0/26 ignored、跨进程UIA和
摘要绑定实际产品Enter worker各显式1/0，资源/启动/worker通过；ignored不计通过。
四份测试变更最大467有效行、严格静态/3,719文件ratchet、三次源码/六件/旧228件和
四诊断保护见迁移台账v379。同源前台0/1仍保留，四前台最终门不重跑，不称已修复。

## v378：四边实心 border 子集

border及四边简写、border-width/style/color四值简写与各side长属性，共用原declarations/
cascade；简写重置3px/none/currentcolor，反射/priority/remove不另建状态。宽度仅非负
px/0和thin/medium/thick；solid绘制，none/hidden used width0；非继承、正边宽仍须
box-sizing:border-box。未知线型、border-image/逻辑方向/相对CSS长度继续原子拒绝。
新私有CSS factory只负责语法展开及Scene四边used值，不读取公开String/Object污染入口。

scene四边宽度贯穿原生布局/同步client geometry/滚动/圆角clip与hit/EDIT及D2D四边色。
真实child几何初值[100,64,90,56,11,5]、左边8→4后[100,64,94,56,7,5]、border=0后
[100,64,100,64,3,3]及恢复由通用fixture验证，不以保存CSS字符串代替呈现。最终22相关
门全过，Engine121/0/5 ignored、Session77/0/26 ignored，真实child/隐藏HWND/EDIT、
摘要绑定产品与四原预算压力均通过，产品/压力另显式--ignored；开发失败不覆盖。
3,719文件ratchet、45变更/11新增、三次源码/六件/旧222件及后续228件见迁移台账v378；
不将子集通过关闭旧前台/性能未知失败，不扩大默认完整JS、业务、权限
隔离/Lease、原预算、完整迁移或最终用户包验收。

## v377：nowrap 文字叶的 text-overflow

text-overflow的准入值为clip/ellipsis，保持非继承；仅文字Element叶（非独立#text）
可声明，同轴overflow:hidden可接原生内容盒。ellipsis进一步要求white-space:nowrap、
text-align:left或缺省、实际单行。容器、Canvas/input/button、混合轴、显式文字visible、
其他white-space/align及压平后代独立声明继续拒绝，不接受字符串省略号/fade/双值语法。
语法失败保留inline/sheet原子性，最终原生准备失败不能替换已安装帧。

scene.style.textOverflow由新surface_text_overflow在最终text_frame宽度消费；固有测量
不变，trimming前metrics/baseline/完整UTF-16保留。只改尚未共享的新DWrite布局，以
ellipsis sign/CHARACTER trimming绘制，已有实际clip继续约束像素；矮盒不变成line-clamp。
隐藏后缀作为一个视觉cluster命中/导航，省略号词级选择返回整个隐藏后缀，复制不丢文本。
滚动复用原COM布局，重排重新建帧，不在draw/hit/reposition时懒修改共享状态。

真实DWrite探针发现trimming sign的HitTestPoint可返回零高/inside=false；仅该异常路径
借HitTestTextPosition恢复真实行高，inside仅限sign矩形，普通文字及盒内空白不扩大。
最终固定19相关门全过：省略native7/0、Engine117/0/5 ignored、Session76/0/25 ignored，
实际产品/隐藏HWND及四原预算压力门通过；产品和压力另显式--ignored，不将未执行的
ignored计通过。行为RED/开发失败、2,556输入/三次源码/六件/旧216件与后续222件身份
见迁移台账v377。不将同源子集通过解释为默认完整JS、UIA TextPattern、真实股票/Loom、
Lease/OS隔离、总资源长稳、完整迁移或用户包验收；不关闭旧前台/压力失败，原
32MiB/256KiB/250ms/Job预算不变。

## v376：单次 cascade winner 的私有记录

仅将computed内临时winner Map换成捕获intrinsic创建的null-prototype record；key仍由
原CSS声明准入/canonical化，important/inline/三项specificity/源序、继承和投影不变，
无跨帧缓存。真实QuickJS新模型2/0证明64节点/16规则的winner get2048/set128/entries65
降为0且完整输出与旧Map相同，CSS名称及constructor/__proto__原型污染不被读取/枚举。
原投影reference借测试副本恢复旧Map后保留原rank oracle，不移除32轮/14夹具保护。

最终Engine114/0/5 ignored、Session74/0/24 ignored、四个原预算压力及三项实际产品
worker通过；20门18过/2失败，显式后台focus负向的Child激活与Application Enter第二轮
真实前台前提未过，故仍partial-runtime-failure。固定15样本前后多数略慢、阶段Collect
151.2134→164.3549ms，未证明整体加速或性能不退化；诊断不作压力门，历史失败不覆盖。
精确成绩/身份/行数见迁移台账v376。默认完整JS、真实股票/Loom、Lease/OS权限隔离/
长稳/完整迁移和用户包仍未验收，heap/stack/250ms/Job与SDK许可不扩大。

## v375：author :hover 子集

最终final-run2为21门17过/4压力失败，保持partial-runtime-failure，不以功能通过
放行默认JS。三个生产owner与首轮一致，旧负向fixture仅改为未知span:hovering，
原子拒绝断言保留。CSS模型9/0、投影6/0、Session74/0/24 ignored及新Hover child/
产品/HWND/Engine、旧Hover窗口6/0、Application1/0与focus3/0通过；四压力门各0/1，
三个可观测路径Collect后期限失效，产品缺详细阶段。首轮压力成功与完整窗口103/27/
1 ignored不被覆盖或重跑求绿；行数/三次源码/六件/旧204份与身份见迁移台账v375。

author stylesheet可借原Hover owner的上一可信目标/当前私有父链匹配:hover与祖先，
沿原class级specificity、cascade、投影和geometry mutation token；最多32层，脱树
不匹配，suspend/dispose/leave清理。不读公开parent/contains或从predicate递归computed，
合成PointerEvent不改变物理状态；未知pseudo的原子拒绝和原型污染边界由通用测试保护。
DOM querySelector/matches/closest仍不接受:hover；没有静止指针布局变化后的自动重命中，
不是完整CSS/Pointer浏览器语义。开发RED/栈边界失败与最终child/产品/HWND/压力及相邻
门分别见迁移台账v375，不扩大默认JS、权限、heap/stack/250ms/Job或真实业务验收。

首轮22门18过/4失败：新增Hover child/Engine/HWND/产品及原Hover Application/focus/
四压力门通过；旧拒绝夹具仍把span:hover视为不支持，导致CSS模型/投影与Session
失败，另有完整窗口前台前提失败及锁中毒。精确分解和身份见迁移台账v375，首轮
失败不覆盖；仅修旧负向样本，不重跑未修改完整窗口求绿，不扩大默认JS或完整验收。

## v374：焦点交付的真实前台前提

产品focus准入不变。自有独立进程负向证明本线程parent仍有焦点、当前epoch有效时，
真实前台不属于Surface就拒绝请求，不激活/入队/恢复重放。复用UIA Child owner，
foreground模式不冒充UIA action成功，仍有PID验证/有界管道/原超时和Drop清理。
Application每段focus与stale测试显式建立前台并在交付点断言，不重试事件；产品
临时trace删除，旧失败根因仍未证明。开发实验、最终相关门/身份见迁移台账v374，
默认完整JS、真实股票/Loom/Lease/OS隔离/完整迁移和用户包仍不放行。

首轮失败仍保留。updated只验原server值/EDIT identity/change文本，不再要求焦点跨
网络批次保持；真实focus/blur的前台与EDIT/parent断言仍在prepare交付点。新跨进程
用例按现有惯例显式ignored，最终独立门仍用--ignored运行，失败不能被默认组跳过
覆盖。Foreground仅加track_caller，不重试；首轮/修正候选成绩与边界见迁移台账v374。

第三候选只修测试Child目标线程激活的一次命令/确认，被动guard共用原Foreground
恢复owner；原命令/5秒确认/生命周期有界，不是产品请求重试或准入放宽。两轮
focus/blur主路径在第二候选通过，不覆盖ABA丢前台或旧完整组失败；最终见迁移台账。

最终显式后台focus1/0、默认focus窗口2/0/1 ignored、Application focus3/0、默认完整
窗口129/0/1 ignored及原UIA/实际产品keyboard/资源/启动/worker通过，10门全过。
产品focus源码与before一致；默认ignored另有显式门通过而非省略。六变更最大321行、
严格静态/三次源码/六件/旧192份及诊断身份见迁移台账v374/最终closeout，旧完整
Application13/12未重跑，不以相关通过放行默认完整JS、真实业务或完整迁移/用户包。

## v373：相邻声明式键盘夹具边界

只在原surface_window_keyboard_tests建立普通消息的线程modifier状态与完整恢复；
raw消息另核产品快捷键拒绝/Shift反向，不改JS/Enter前台检查、FIFO、worker或产品
输入准入。受控Ctrl的RED与最终相关窗口/Application/产品门分报，见迁移台账v373；
不据此关闭v372未知失败或放行默认JS/真实股票/Loom/完整迁移与用户包。

最终fixture7/0、窗口129/0、实际产品keyboard1/0及资源/启动/worker通过；8门7过、
1失败，Application14/11为focus/blur期望EDIT持焦失败及十项锁中毒。本次Enter过
不关闭历史keyup/焦点根因，窗口成功不覆盖Application。唯一测试文件152→296行、
严格静态/3,696文件ratchet、2,544输入/六件/旧174份及诊断身份见迁移台账v373/closeout。

## v372：非捕获 Hover 子集

通用可见DOM命中沿原FIFO/Runtime/同child接无按键move与leave，包装节点可接冒泡move
及其自身非冒泡leave；EventTarget仍唯一拥有事件identity/派发，relatedTarget只能为
合法EventTarget或null。新hover factory只保上一条最多32节点路径，suspend/dispose清理，
不复制DOM/几何；原native carrier/捕获intrinsics/Promise边界不变。enter/leave非冒泡/
不可取消/非composed，over/out/move保持冒泡/可取消/composed，mouse/primary/id1与DIP/
modifier有界。原32槽/sequence共享两种terminal预留，硬取消保prepared、未派不伪造leave。
hover不取capture或焦点，允许原EDIT持焦；Canvas up仅续结已有hover，原拖动合同保留。

通用夹具覆盖wrapper/rAF/像素而非翻译股票业务。精确运行成绩/失败/同源身份另见迁移
台账v372，默认JS/完整Pointer/真实Loom与股票/Lease撤销/OS权限隔离/长稳/用户包不因此
放行，原heap/stack/250ms/Job及JSON/emit边界不变。

最终Hover HWND6/0、child4/0、wire1/0、queue5/0，独立Application1/0完成两轮原
TLS/Grant/FIFO/同child，产品worker1/0；完整JS189/0/28 ignored、十四产品worker与
三原规模压力门通过。34门32过、2失败：完整Application13/12的Enter前台失败/十一
锁中毒，以及完整窗口122/4的四项旧键盘焦点/候选直接失败，根因未定，均不被独立
成功覆盖。35变更/12新增/最大500行、严格静态、2,544输入/六件/旧168份见迁移台账
v372/最终closeout；保持partial-runtime-failure，不扩大默认JS或完整迁移结论。

## v371：Canvas 左键 Pointer 子集

真实HWND只为当前JS Canvas采集左键捕获down/move/up/cancel，经原Mount FIFO和Runtime
送入同一child。事件继承MouseEvent/Event，固定mouse/primary/pointerId=1、客户区DIP
及四modifier；自有family字段/brand检查不接受原型借字段。共享32槽/sequence预留
terminal，move只合并同手势未封存队尾，不改prepared，硬取消复用原槽并使旧fence失效。
capture acquisition的provisional登记/同步重入复验和正常release的窄豁免分别保护；
同HWND下一Canvas手势保留前一完整手势，失活/隐藏/几何变化仍硬取消，不后台重放。

Pointer preventDefault仅更新事件状态，没有宿主默认动作，也不抑制原阈值click；up
及microtask后核同target连接/disabled和自身/祖先computed display，再派普通Event
click。cancel允许到原已脱树Canvas identity。无touch/pen/hover/完整MouseEvent或JS
Pointer Capture API，不扩大默认surface.javascript.v1。Application仍走原TLS/Grant/
batch，policy关闭不能代表真实Lease撤销。最终实绩、源码/程序身份及开发失败见迁移
台账v371；原heap/stack/250ms/Job、JSON/emit和OS权限边界不变，完整迁移未验收。

最终HWND9/0覆盖同步重入/连续手势与stale同次领取，child5/0、Pointer实际产品1/0；
完整窗口120/0、JS184/0及其他十二产品worker通过。27运行门26过、Application13/11为
Enter前台获取失败及十项锁中毒；新增Pointer固定一次独立Application1/0，不覆盖该失败。
三压力门/严格静态、37文件/最大497行与三次源码/六件/旧162份见迁移台账v371/closeout。

## v370：保持层叠语义的局部分配缩减

css_sheets仍唯一拥有author cascade。important/inline/三项specificity按原层级比较，
输入按源序遍历，相等时后来覆盖；不以数值权重压缩元组。每次computed拥有新的winners，
只原地更新当前属性的自有字段，不修改声明/selector数组或共享到其他节点/帧。独立
Rust oracle、旧算法差分和多节点/更新/原型accessor回归保护该优化，实绩见迁移台账v370。
原256节点/16规则/250ms完成门与诊断分开，JSON copy/payload、heap/stack/Job、权限和
默认能力不变，不把单项压力通过视为完整JS、真实股票/Loom或最终迁移验收。

最终压力Engine/child/实际产品各1/0，完整JS177/0/26 ignored及窗口111/0、startup7/0
通过；24运行门23过、Application13/10为前台获取失败+九项锁中毒，三诊断独立分报。
新旧库相邻矩阵均15/15，原压力历史根因与长稳未关闭；严格静态、两文件73→75及
186→300有效行、三次源码/六件/旧156份见迁移台账v370/最终closeout，状态仍partial。

## v369：测试前台清理不覆盖后来选择

三个自有屏外HWND的RED证明原Foreground guard无条件恢复会覆盖第三窗口的新前台。
guard现记录previous/target，仅在仍持有target或target已毁且前台NULL时归还；不改获取
流程、AttachThreadInput或产品输入/权限/资源预算，也不承诺前台独占。新回归保护后来
选择、普通归还和精确panic载荷的unwind恢复，测试cleanup自身不覆盖外部新前台。
候选诊断1/0未复现v368失败，最终只增强分支/焦点/capture/逻辑目标/modifier断言，
不据此更改生产捕获规则。最终成绩/身份见迁移台账v369，旧未知根因与完整迁移仍开放。

最终12门10过、2失败，无deferred门。Application23/0、values5/0、child3/0/1 ignored、
队列两门各1/0、SHA绑定产品keyboard1/0、startup7/0及资源/worker门通过。foreground
专门门1/1中的unwind未取得初始前台，完整窗口93/18中两项guard回归均过，但旧focus
前台获取失败+17锁中毒，后台Enter负向未进入体；不以另一上下文成功覆盖失败。严格
静态/3,671文件ratchet通过，四变更最大192行；首轮Clippy失败、身份/三次源码/六件/
原150份见迁移台账v369/最终closeout，保持partial-runtime-failure及原全部未知根因。

## v368：测试键盘资格不等于定向消息

Application状态观测复现keyup失败时，EDIT仍有本线程焦点且启用/可见/local_edit均true，
但真实前台已不是Surface，原入口按既定安全门拒绝入队。只修测试prepare/release的
前台生命周期：每段用已有Foreground guard建立/恢复，每次发送前断言，不自动重发
事件或提高原16次poll/资源期限。另以真实HWND证明后台down/up不入队、恢复后不重放，
随后合法keyup才交同一child。生产准入、API、队列/权限及默认能力完全不变；最终
成绩/失败/身份见迁移台账v368，旧未观测keyup及压力/form/startup根因不一并清零。
当前游戏会持续取回前台；新驱动发送前仍0/1，开发负向用例1/0，均保留原证据。
未操作游戏；用户随后允许短暂占用前台，最终恢复完整10门矩阵，不用background-only。
实际终态另记，不能以授权或运行启动代替驱动修复验证通过。

最终完整10门8过、2失败，无deferred门。Application13/10为Enter在原前台fixture失败
及九项PoisonError；SurfaceWindow90/19含原点击候选未入队、另一前台fixture失败及17项
PoisonError，新增后台负向最终取锁即失败，不用开发单项1/0代替最终通过。child3/0/1
ignored、队列两门各1/0、SHA绑定产品keyboard1/0、startup7/0及资源/worker门通过。
严格静态/3,670文件ratchet与134/329有效行审计通过，身份/三次源码/六件/旧144份见
迁移台账v368/最终closeout，保持partial-runtime-failure，不关闭未知根因或完整JS验收。

## v367：保持原合同的CSS热点缩减

projection只枚举私有null-prototype计算记录的已有属性；class编译为捕获构造器创建的
私有ASCII空白边界正则；sheet解析后只持一份有序声明，inline仍读原可变Map。原准入、
上下文校验、层叠/继承和native值转换不改，不新增缓存/权限/资源预算或默认能力。
完整输出/拒绝差分、原256节点/16规则压力及child/实际产品最终成绩见迁移台账v367；
原型污染和同步原生几何分别验，不把诊断退出0、局部提速或期限拒绝当吞吐成功。
私有JSON批量取描述符后仍仅复制可枚举字符串数据属性，拒绝getter、保留原深度/堆栈
和wire限额；只在测试副本单独观察copy边界与超250ms尾部，不提高Engine/child预算。
旧Application keyup根因、完整JS/股票/pointer/迁移与总资源长稳/用户验收仍开放。

最终24实际运行门18过、6失败，另两诊断exit0不算吞吐成功。JSON2/0、projection4/0/4
ignored、CSS16/0/5、完整窗口108/0和八非压力产品worker各1/0等通过；原压力Engine/
child/实际产品均0/1。完整Application13/10为keyup直接失败+9锁中毒，完整JS137/38/26
为form Apply期限失败+37锁中毒，startup4/3触发原15秒三项期限，根因均开放。最终15
固定样本仅无规则3/3，阶段诊断275.2552ms不能放行250ms操作。严格静态、12文件行数/
3,670文件ratchet通过，三次源码/六件/旧138份与精确成绩见迁移台账v367/最终closeout；
保持partial-runtime-failure，不覆盖失败、不提高原预算或宣布完整插件/默认JS已可用。

## v366：author CSS 的静态位置伪类

原constructed sheet支持`:nth-child(An+B)`的有界ASCII子集：odd/even、带符号整数、
n/+n/-n、连续整数系数与n、可选带符号B，A/B绝对值不超过2147483647；外侧和B符号
周围允许CSS空白，不接受`+ n`、小数、of-selector、嵌套函数或其他位置伪类。按元素
兄弟从1计数，文本不计、display:none计入，root位置为1，specificity仍归原cascade。
位置WeakMap借原mutation token失效，不持第二份树或历史；first-child复用索引并
保留原有父条件。原选择器/规则/节点和全部资源预算不扩大。

这是样式表matcher能力，不扩展querySelector/closest/matches的选择器语法，也不
冒充hover/focus-visible/native pointer或完整股票支持。128公式的Rust oracle差分、
原子拒绝、同operation原生几何/跨update、原型污染、child/实际产品/HWND像素的精确
实绩和同源身份见迁移台账v366；默认JS、完整迁移/资源长稳及用户验收边界不变。

256节点/16规则压力用例仍保持原250ms和必须完成的严格断言，单独显式运行，失败
不能被功能集green或ignored覆盖。同步几何额外栈深已缩减，不提高256KiB；开发
innerHTML拆移与collect/project超时分别留证，不宣称复杂样式吞吐达标。

最终nth-child6/0/2 ignored、CSS12/0/1、完整窗口108/0、JS169/0/22、Grid16/0、Geometry
29/0/1与三外部SHA绑定产品worker各1/0等通过，14门中仅显式压力门0/1失败。严格
静态/3,664文件ratchet通过，源码/六件/原132份身份见迁移台账v366/最终closeout。
首轮Application13/10与单次同字节Enter复核1/0仍分列，keyup did not drain根因开放；
最终不同EXE未重跑完整Application组，不以功能集通过覆盖压力或历史未知失败。

## v365：值校验的有界扫描与分配

原form值校验对不含NUL/非ASCII的值走捕获native exec准入；无换行不建逐字符数组，
有换行按连续保留段归并。Unicode先完整验证代理项与UTF-8字节数，再去CR/LF，不能
把换行两侧的孤立代理项拼成字符。原65536 UTF-16 units/48KiB、原子拒绝和dirty/
defaultValue语义不变。新探针仅测试构建显式运行，观察原Engine墙钟/线程CPU，不增
产品诊断或预算；22个child边界与784组Rust oracle差分、实际产品门见迁移台账v365。
此优化不扩大API/默认能力，不替代资源长稳/真实插件验收或关闭历史未知根因。

最终值校验3/0/2 ignored、完整窗口107/0、JS164/0/20、Application23/0和十四产品
worker各1/0等在43选定封存门中通过；严格静态/3,658文件ratchet过，七个native变更
最大422行。原mount固定中位215.077→11.420ms、ASCII边界写入225.033→4.5207ms，
完整七类/四份摘要、原型数值槽夹具失败和同源身份见迁移台账v365。探针非预算/长稳
门，历史失败不覆盖，不以更快的受限输入路径替代完整键盘/pointer或默认产品验收。

## v364：文本 input 的真实 Enter 与默认 change 取消

KeyboardEvent继承同一Event，提供key/code/location、ctrlKey/shiftKey/altKey/metaKey、
repeat/isComposing与四修饰键的getModifierState；合成构造/dispatch仍不可信，无原生
默认动作。未知modifier返回false；未实现legacy keyCode/which/charCode等完整API。
原生入口仅是文本input的非IME WM_KEYDOWN/WM_KEYUP Enter，key为Enter，code按系统
extended bit区分Enter/NumpadEnter，location为0/3；keyup不repeat，均冒泡/可取消/composed。
普通字符、Tab、其他input类型/textarea、SYSKEY与IME中间态不广告为原生KeyboardEvent。

原Enter同步COMMIT在显式JS窗口中被截住，边沿与文本共用唯一32槽FIFO、ticket、
30秒期限和click fence，不另排键盘队列或同步等待child。dispatch返回的私有取消
结果仅绑定当前operation；父侧不读取可覆盖的公开defaultPrevented。非repeat首次
keydown未取消且尚有未提交dirty代次时，宿主把默认change替换到原键盘槽/序号，
顺序为keydown→默认change→keyup/后续输入，不能挪到FIFO尾。keyup的取消不产生默认，
重复keydown交listener但不重复提交。本地事件不自动联网，只有显式emit走原权限链。

DOM帧呈现和原ticket结算后先处理focus/blur，再复核字段合同/队列scope/期限、当前
真实焦点和epoch/前台/策略/租约；ABA/撤权/隐藏/换合同默认丢弃不重放。键盘不写
旧value快照，默认change消费已接纳DOM值，保留listener改值及后续原生草稿。此为
原Surface单行Enter的change默认动作，不是HTML form submit、同步浏览器时序或全键盘。
验证/失败/同源身份见迁移台账v364，原资源预算、默认JS/真实插件/完整迁移门禁不变。

最终Enter11/0/1 ignored、键盘49/0/1、Application23/0、JS161/0/18及十三摘要绑定
产品worker各1/0等通过；38个native变更最大493有效行，严格静态/3,654文件ratchet
通过。41封存门首轮40通过、1失败，旧输入夹具mount触发250ms期限后两项锁中毒；
固定六门同字节复核包含完整窗口107/0通过，但不覆盖首轮失败或关闭deadline未知
根因，closeout仍为partial-runtime-failure。完整身份/边界见迁移台账v364，不外推
完整浏览器键盘、稳定性/总资源、默认产品或最终用户验收。

## v363：受限的原生 focus()/blur() 请求

文本 HTMLInputElement 新增无选项的 focus() / blur()。它们返回 undefined，把当前
operation 内最多32项有序意图放入同一 child 回复；不是同步浏览器焦点模拟。断开或
disabled input 的调用不排请求，错误 receiver、focus options 和超容量明确拒绝。
私有请求只含稳定 DOM target 与 focus 布尔值，不读公开 id/disabled/parent 反射或
可覆盖方法；宿主不接收 HWND、激活指令、远端 action 或凭据。

父侧只在 DOM 帧呈现与本地 ticket 结算后处理意图，复用原 Mount 的当前 HWND 和
语义句柄，不建立第二份窗口索引。只有已确认 owner、当前应用输入策略允许、窗口
可见/启用且已在前台、键盘焦点已属于同一 Surface 时才能切焦；不调用激活 API。
目标还须是当前可见/启用 input，blur 只释放当前目标到 Surface 根，不操作外部窗口。
原 SetFocus 的 EN_KILLFOCUS 在 Host 已归还后进入原 change/草稿/32槽 FIFO，不伪造
change，也不自动联网。listener 的显式 emit 仍走原 authority/Grant/FIFO/收据。

operation 提交时捕获原生 focus epoch，父/子 HWND 的真实 focus 边沿即使在 Host 暂时
取出时也推进代次；失焦后回到同一 HWND 的 ABA 仍使旧回复失效。过期、后台、隐藏、
撤权和不存在的目标均不重放。应用沿唯一 surface_input_origin 策略先收紧输入，再
结算 child 回复，避免忙碌/模态/退出后用上一轮 local_edit 资格切焦。

尚未支持 document.activeElement、focus/blur/focusin/focusout 事件、自动滚动、options、
按钮焦点 DOM 方法或完整浏览器同步语义。keydown/keyup/KeyboardEvent 与真实默认动作
取消、更多表单属性、pointer、默认JS、真实股票/Loom和完整迁移/最终用户验收仍开放。
本轮验证/失败/身份见迁移台账v363；不继承旧绿灯或清除历史未知根因，原资源预算不增。

最终37个选定封存门全过：焦点9/0/1 ignored、较广焦点52/0/2、完整JS157/0/17、
Application22/0、原生输入7/0/1、EDIT66/0、number37/0、十二摘要绑定产品worker各1/0及
startup7/0。24个native变更最大499有效行，八新增最大168，3,645文件ratchet/严格静态
通过，无新例外。开发前RED、前台支架/时序/鼠标默认失焦夹具失败和完整身份留在
迁移台账v363/final-run closeout；不把请求子集等同同步浏览器焦点或键盘默认动作。

## v362：真实 EDIT 的文本 input/change

显式开发协商的 JS 子集新增文本 `input` / `HTMLInputElement`，支持 type=text、value、
defaultValue、disabled、placeholder 和严格 fragment 的 void input；其他 input 类型、
maxlength/inputmode、textarea/form、focus/blur 以及 InputEvent 的 data/inputType 仍未接入。
value 与默认属性按 dirty 状态分开，程序写值去掉 CR/LF，不触发输入事件；NUL、畸形
UTF-16、超限值原子拒绝。原始值最多65536 UTF-16 units，规范值最多48KiB UTF-8；
字段属性/值只由同一私有 form owner 反射到原 Scene input，公开描述符覆盖不改变
宿主私有状态。准备后提交结果不确定则关闭自有会话，不归队重放或泄漏ticket。

真实原生 EDIT 的通知沿原 Session/Draft/ticket 和 waiting+prepared<=32 的唯一 FIFO
进入私有 TextInput carrier，不借声明式 events 编码假 action。input 是编辑后的冒泡、
不可取消、composed 事件，change 是冒泡、不可取消、非 composed 事件；二者由同一
私有 dispatcher/Engine job 边界交付 isTrusted=true。程序 dispatchEvent 仍不可信，
本地事件不自动联网；listener 显式 emit 仍按原 snapshot authority/Grant/FIFO 提交。

本地 ticket 在对应 DOM 帧验证/呈现后结算，与 emit 的远端回执严格分离。后台忙时
保留本地编辑，提交仍受单槽/owner/策略门禁；前帧不能覆盖更晚草稿。点击继续使用
原 fence，顺序为 prefix edits → click → later edits，IME 组合结束前不派发半成品。
原挂载最多64个可见字段；同字段相邻input仍只合并FIFO队尾，change边沿保留，不
保证每个按键各有一次callback。change沿原失焦/单行Enter提交，不伪称KeyboardEvent。
真实 HWND/child/Application/产品门和原预算/精确成绩见迁移台账 v362；默认 JS、
keydown/preventDefault 默认动作、pointer、完整股票/Loom/迁移与最终用户验收仍开放。

最终33个选定封存门全部通过，输入7/0/1 ignored、JS153/0/16、Application19/0、
EDIT66/0、number37/0，十一项外部摘要绑定产品worker门各1/0，startup7/0。集合重叠
不相加，未显式执行的ignored不计通过。严格静态/3,637文件ratchet通过，36个native
变更最大483有效行、八新增最大175，无新例外；最终身份/全部失败和完整边界见
迁移台账v362/final-run closeout，不提升默认能力或清除历史未知根因。

## v361：同文档 EventTarget 与原生 click 共用分派

`Event` / `EventTarget` 是文档内的有界事件子集，所有 Node 继承同一 EventTarget。
支持 function 或 handleEvent object listener、boolean/dictionary capture、once、passive，
身份按 type/callback/capture 去重；重复 add 不覆盖 once/passive。每目标/阶段各取快照，
已移除项即使在旧快照中也不再调用，once 在调用前删除。路径在首个 callback 前固定，
按 capture→target→bubble 分派，非冒泡事件仍经过捕获/目标；支持 stopPropagation、
stopImmediatePropagation、preventDefault、returnValue/cancelBubble 与独立 composedPath 副本。

Event 提供 type/target/currentTarget/eventPhase/bubbles/cancelable/composed/timeStamp、
defaultPrevented、只读且不可重定义的 isTrusted 及四个阶段常量。公开 dispatchEvent
交付非 trusted 合成事件；已有宿主 click 从同一私有 dispatcher 交付 trusted click。
同步 listener 异常经 finally 清理 currentTarget/phase/path/active/停止/passive 状态后
原样重抛，保持原宿主 fail-closed，不伪装浏览器的异常 report-and-continue；已取消标志
保留。returnValue=false 与 preventDefault 共用私有取消逻辑，不调用可被覆盖的公开
方法，且不绕过 passive/cancelable。实际 Engine 复用原单调时钟，不增加计时线程。

单文档 lifetime 最多2048个 EventTarget（包含 Node/独立 target），每 target 同时最多
16个 listener，type 最多256 UTF-16 units，路径/嵌套 dispatch 各最多32，仍受原32MiB
heap/256KiB stack/250ms约束，不保证任意深度插件重入。signal 非空明确拒绝，未实现
AbortSignal、MouseEvent/KeyboardEvent/PointerEvent、document/window EventTarget、on*属性
或新默认动作。允许注册/合成 keydown、change 或 pointer*，**不等于原生键盘、输入框、
pointer 或取消默认行为已经接入**；本轮唯一真实 native 入口仍为已有 click。

native click通过原Engine的JS job边界直接运行dispatcher，bootstrap显式await结果；
公共dispatchEvent仍同步。原Engine排尽此单个job和listener的microtask，不增新调度
owner/跨operation队列或重置总期限。私有非thenable carrier、固定constructor/species
与捕获then隔离插件Promise原型；同步callback异常仍拒绝原apply而不变成无人观察的
Promise rejection。原栈下geometry/空间selector链的开发失败与最终验证见迁移台账。

事件不新增远端动作 authority：emit 仍须来自原 snapshot 声明，并由父侧重新验证
binding/revision/action，沿原 TLS/Grant/FIFO/收据与同 child update 链路。模型、通用
child/产品夹具、隐藏 HWND 像素/命中和 Application 的选定成绩见迁移台账 v361；默认
JS、真实股票/Loom、完整迁移/长稳/最终用户验收与旧未知根因保持开放。

最终事件10/0/1 ignored、完整JS149/0/15、Application18/0及摘要绑定的event-target
实际产品门1/0通过，原生/合成阶段、移除/once/取消/污染、两条同步CSS/几何链与
原异常/CPU回收均由选定门保护。全目标严格静态/3,629文件ratchet及29封存门通过，
19个native变更最大386有效行、八新增最大148，无新例外；源码/六程序/旧96份身份、
全部开发失败及精确边界见迁移台账v361/final-run closeout。不清除旧未知根因或
开启默认JS，不把这些选定门提升为完整浏览器、真实股票/Loom或完整迁移验收。

## v360：四边 padding

`padding` 接受 1–4 项，按 top/right/bottom/left 的 CSS 规则展开；支持
`padding-top/right/bottom/left` 及相应 style.paddingTop 等属性。每项仅接受 0 或
0–8192px 的非负 decimal，值输入最多256 ASCII bytes，规范化 .N/N. 后交原 Length。
百分比、em/rem、auto、负值、calc、逻辑边、margin/单边 border 均不在本 CSS 子集。
sheet 声明仍在原有 strip/空白/important 解析后进入值门，整 sheet 最多65536字符；
不将整个 sheet 字节数混同于 DOM setter 的原始 value 上限。

整项验证成功才保存四个独立 longhand，后来的 shorthand 重置先前四项；importance
仍逐项参与 cascade。style.padding 仅在四项存在且 priority 一致时反射最短1–4项，
显式 setter 可覆盖 important，removeProperty('padding') 移除四项。padding 不继承，
声明及最终投影仍要求 box-sizing:border-box，未知或不支持值不静默忽略。

Scene 投影只交付 paddingTop/Right/Bottom/Left；原生四边 Insets 统一拥有起点和
两轴总量，不复制 JS 布局算法。声明式原单值 layout.padding 保持兼容，side longhand
覆盖对应边且复用原 native Length（含原百分比/相对长度）；这不开放 CSS 相对值。
按钮和 textarea 未声明的边仍保持各自默认值。文字/图片的测量、重排、原点、
内容裁剪及原 textarea 格式框都消费四边；不改变其他原生字段的既有格式合同。
textarea 只在字段自身合同不变时保留 HWND/草稿；修改自身 padding 声明仍使原
fingerprint 失效，不能为几何变更绕过授权/草稿边界。父宽变化下四边百分比可在同
字段合同中重新求值，实际格式框由原 EDIT owner 更新。

ResizeObserver 的 contentRect.x/y 是左/上 padding，内容宽高分别减左右/上下及
border；device content 仍按实际内容两边独立取整。绝对定位/overflow 的 padding edge
不随内容边收缩；图片四个内容圆角则各减相邻边，并在原 Clips 中缓存真实几何。
通用夹具覆盖原子拒绝、重入变更/抛错、intrinsic 污染与同 child/产品路径，实际运行
成绩见迁移台账 v360。原 heap/stack/250ms/read/layout/Job/startup 门不增，默认 JS、
完整 CSS/股票Loom、完整迁移及最终用户验收仍不放行，旧未知故障不清零。

最终padding43/0/1 ignored、完整JS141/0/14及摘要绑定padding产品1/0通过；真实
child同文档更新/回收、原子拒绝/重入/污染、同步几何/ResizeObserver和隐藏HWND
像素/文字/命中均由选定门验证。首轮JS的过期双值拒绝与scalar断言造成99/42/14，
只修两个测试输入，完整失败和首轮程序保留。全目标严格静态/3,621文件ratchet及
26封存门通过，实际源码/六程序/旧90份身份见迁移台账v360/最终closeout；不是
全库、全部ignored、完整CSS或真实插件验收，也不改变默认能力及旧未知根因。

## v359：font shorthand 与继承行高

字体声明统一由私有 CSS fonts factory 准入/规范化，沿原 sheet/inline/DOM/cascade
路径接通，不开放新全局 API。font 接受可选 normal/bold/100–900 整数 weight、1–8192px
字号、可选 `/normal`、`/1–8192px` 或 `/1–1000` 无单位行高，以及 v358 的 family 列表。
font 原始输入最多512 UTF-8 bytes，其他字体项最多256 bytes；family 的8项/256 bytes/
31 UTF-16 units、原始字符域和规范化输出门不变。未提供 weight/行高重置400/normal，
不保留之前 longhand；整项成功才提交四个展开项。style.font 反射需要四项均存在且
priority相同，显式 setProperty 可以覆盖 important；removeProperty('font') 移除四项。

无单位行高保留数字继承，每个投影节点转为同数值 em，复用原 native 长度和文字
owner 按最终节点字号求值；normal 清除祖先显式行距，不用固定比例猜 baseline。
最终高度仍须不超过原32768门；px行高保留旧显式字号/不小于字号的非重叠合同。
不支持小于1的行高、百分比行高、相对字号、系统 font keyword、非normal字体样式、
variant/stretch、Canvas font 改造、字体下载或作者逐字形字体链，未知值明确拒绝。
原32MiB/256KiB/250ms、read/layout/父侧/Job/startup门不增，实际最终成绩见迁移台账
v359，不以模型通过代替真实child/产品或完整股票/Loom验收，默认JS仍关闭。

属性值的String转换在公开Proxy setter完成后才进入setProperty/私有解析，保留
主动重入变更及原抛错身份；投影中只有一个调用者的native层合入prepare，不增加
原栈/时间或另建owner。增强夹具在转换中写字号/同步读几何并抛错，真实Engine
已捕获原栈溢出，补接后的聚焦8/0/1 ignored通过；固定旧失败不被该机制外推解释。
真实消费者22条字面字体/行高值亦通过生产工厂的只读探针，不代表完整插件执行。

最终全目标严格静态/3,612文件ratchet与六程序构建通过；16个native变更最大380
有效行、无新例外。21封存门通过，包括typography8/0/1 ignored、JS136/0/13、
text32/0、measure124/0、八产品worker各1/0及startup7/0。原重入夹具未删断言或
加预算，源/程序摘要、旧78份与失败收据见迁移台账v359/closeout；默认JS、真实
完整CSS/股票Loom、完整迁移和最终用户验收不因该子集成功放行。

## v358：有界的已安装字体选择

`font-family` 可通过 sheet、inline CSS 和对应 DOM 属性进入既有 cascade/继承，
投影为 Scene 的 `fontFamily`。这是**按顺序选第一个已安装 family 的子集**，不是
完整浏览器的逐字形作者字体链：选定后缺字仍由 Windows 原字体回退处理，不能承诺
按作者后续 family 顺序补字。Canvas font、字体下载、@font-face、font shorthand、
italic/stretch、unitless line-height 和默认 JavaScript attach 均不在本轮支持范围。

最多8项，输入及规范化输出各最多256 UTF-8 bytes，每项最多31 UTF-16 units，
以匹配原生 EDIT/Select 的 GDI face 上限。接受 ASCII 字母/数字/空格/下划线/连字符
及 U+3400–U+9FFF 汉字，非引用名称的各词不能以数字或连字符开头；引用名称保留
大小写及内部空格。只允许无转义的名称，不接受空项、逗号嵌在引号内、URL、CSS
wide keywords 或未实现的 generic。`sans-serif`/`system-ui` 按 Segoe UI→Arial、
`serif` 按 Times New Roman→Georgia、`monospace` 按 Consolas→Courier New 查询；
引用的 generic 是普通名称。整列缺失回到同一 sans-serif 候选，仍无安装项则失败。
这不是系统字体枚举 API，不向脚本返回集合；声明失败不提交或使几何缓存失效。
原始名称输入在trim前先限制为ASCII/汉字字符域，再按该域计算UTF-8；不允许非
ASCII空白先少计字节、再被trim移除而接受超限输入。原始及规范化边界分别验证。

唯一 native 选择 owner 位于原文字 Engine 下，字体集合首次有效请求才查询，不检查
更新或下载；64项缓存随 Engine 释放。Styles 以独立 Rc 保持已选名称并在读取时借用，
不构造自引用；真实测量/绘制和原生字段共用同一名称，场景替换仍核对节点身份。
模型/child/同步几何/隐藏 HWND 像素及原生字段的实际成绩见迁移台账v358，不以
声明接线或历史测试代替本轮证据。原 heap/stack/250ms/read/layout/父侧/Job/startup
预算、完整股票/Loom、N01–N12及最终手测门不变，旧未知失败不因成功子集清零。

final-admission封存19门全部通过，字体11/0/1 ignored、文字31/0、measure124/0、
EDIT2/0、Select1/0、JS129/0/12及摘要绑定font-family产品1/0，Canvas/geometry及其他
六产品门、资源/startup亦通过，重叠集合不相加。NBSP原始字节门反例进入同一通用
夹具，模型/真实child/最终产品都验证严格错误身份及原子拒绝；纯Node探针只核值
分支。首轮过期Arial拒绝断言造成的39失败及未封存中间构建保留，最终生产修复、
源码/六件身份和18文件行数见迁移台账/closeout，不将本子集称为完整CSS或股票可用。

## v357：测试专用的失败阶段边界

测试worker的私有Response可携带固定Report；没有字段的实际产品回复仍兼容，
正式产品不编译该字段/轨迹。Engine/Canvas各最多64个固定enum和单调微秒样本，
附原期限/有效/粘性失败bool；不收JS异常、脚本、节点、snapshot、路径或凭据。
父侧仅在原序号/停止检查后的失败回复输出有界且单调的Report；诊断写入错误
不得影响原InvalidData、child回收或容量。不会包裹JS调用而增加重入栈层，原
stack/heap/250ms/reads/layout/父侧/Job/startup均不变。回归、真实Application
与最终封存成绩见迁移台账v357；仍不将新观测当作v356未知根因修复或能力放行。

首轮JS124/1/11 ignored在直接Engine Canvas模型初始mount失败，20门仅19门通过。
原模型helper现仅在apply错误返回后输出同固定Report，不读取异常或改错误身份，
也不冒称覆盖Engine::new前期失败。独立补接源码/封存矩阵16门通过，三次预先固定
完整JS观察各125/0/11，diagnostic7/0、Canvas14/0/1、geometry29/0/1、六实际产品
worker与startup7/0。只变一个测试输入，生产不再修改；首轮模型/v356 Application
及旧未知失败未结清，整轮partial，完整身份见迁移台账v357。真实消费者还需CSS、
input及keyboard/change/pointer listener等已确认源码缺口，不据此广告真实股票可用。

## v356：公开 CSS 输入转换与重入边界

cssText、Element.style整体赋值与CSSStyleSheet.replaceSync在公开setter/方法内
先用预捕获String转换，再调用只接收primitive string的私有parser；sheet接收者
验证仍在转换之前。重入几何不再额外挂着外层parser栈，不增加原256KiB栈预算。
转换一次→完整解析→预算检查（sheet）→mutation通知→提交的可观察顺序不变；
转换中主动完成的mutation不回滚，抛错或非法解析不能部分安装外层替换。

嵌套sheet仍过深的值校验由原values owner压平：validate并入唯一调用者expand，
color跳过test包装直接使用原捕获exec。grid/spatial分派和独立限额先于普通值检查，
原始长度先于trim/lower，width/height的auto仍返回私有items数组，其他值规则不变。

真实Engine已分别复现inline及sheet旧栈溢出，不以fake boxes解释掉真实失败。
通用回归验证40→50→60和主动mutation后抛错/拒绝；拒绝用例核转换完成标记与
固定解析错误，真实child/实际产品沿原Session/Job/单槽与dispose回收，不新增owner。
具体冻结/运行成绩见迁移台账v356；只保证已验证的有界场景，不承诺任意深度重入、
完整浏览器或默认JS。v355原收据和其他未知失败保持原样。

v356最终原预算下geometry29/0/1 ignored、JS118/0/11、六个实际产品worker各1/0
通过；七个重入模式都进入摘要绑定产品EXE，并经同child update与dispose验证。
但19个封存门仅18门通过：Canvas Application初始回复失败，三项锁中毒连带；
底层根因仍未知，新旧ABBA四次隔离成功不清除原失败。整轮partial及完整身份见
迁移台账v356/closeout，不扩大重入深度或默认能力承诺。

## v355：几何投影缓存的失效边界

缓存仅为一次Engine operation内的encoded Scene source，不是最终矩形或授权结果。
tree拥有私有无原型identity token；树/文字、元数据反射、inline CSS、sheet替换/
adoption及Canvas尺寸的实际owner在完整校验后通知，再提交私有状态。输入转换
期间的重入读取按当前已提交状态求值，后续提交必换token；投影与编码前后还需
token一致才保存。无token计数回绕、历史队列或对外API，最多保留新旧token及单份
source，仍受原child heap/期限约束。保守失效可由脱离节点写入或无效益删除触发。

begin无条件清缓存，避免跨viewport/DPI/生命周期操作复用；dispose释放引用。
连接且可度量元素的每个box getter仍进入原Rust Geometry的reads/layout/deadline/
viewport/target及粘性失败检查；断连节点零盒、inline拒绝和DPR绑定语义不改。
Canvas纯绘图不影响几何source，尺寸重置影响；collect仍按最终树重新投影。
fake-box模型只证明缓存/失效，真实布局/child/HWND/Application与产品门分别验证，
实际成绩见迁移台账v355；不新增默认能力、时间宽限或完整浏览器承诺。

模型仅在测试求值副本的cache miss处增加计数和一次性故障注入，不在生产增加
wrapper或公开token；由Rust交付operation，保持原32MiB/256KiB。最终11模型中10项
通过，含mutation失效、拒绝原子性、投影中变更拒绝、失败不缓存、cache hit仍交native、
intrinsic隔离与dispose；输入转换重入仍栈溢出，归因未完成，不能宣称任意深度重入
可用。真实HWND/child/Application和原预算粘性拒绝通过不覆盖该失败；geometry
25/1/1 ignored、JS114/1/11共享此根因，整轮partial。完整证据与身份见迁移台账v355。

## Owner 与调用链

- `surface_javascript::Session` 独占持久 child、pipe、序号、绑定及当前权威快照。
  阻塞方法只供后台 owner 或隔离验证使用，不应在 UI message handler 执行。
- `surface_javascript::authorized::{Job, Ready, Mounted}` 持有原设备 Grant 的请求租约：
  Job 在后台获取/核验入口并 mount，Ready 未消费仍占共享任务槽，只能移交给同 Grant；
  Mounted 的每个操作前后核对原租约、当前 attachment/代际/生命周期及源码资源身份。
  成功 join 后释放 Job 不撤销已移交会话；关闭/失败只回收自有 child，不重放远端动作。
- `authorized::Runtime` 是 Application 的持久后台 owner，复用 prepare 的取码/求值，
  不嵌套另一个 Job 线程。一个未消费操作占一个槽，回复/线程终态各有稳定句柄；
  初始结果消费前保留共享请求槽，之后只持有原串行请求租约与同一个 child。
  后台等待命令、撤权、child 退出及源码/凭据最早期限；UI 只排 rebind/click/update、
  消费完成帧，不重新获取或挂载脚本。
- `surface_window_admission` 与 `surface_window_javascript` 归原 AuthorizedWindow，
  不另造生命周期或动作 owner。只有请求明确支持 JS 才准入 JS 权威 scene，隐藏
  HWND 必须换成真实 DOM 帧后才可 active；Active ACK 的后台绑定同步完成后才可 show。
  rebind 只同步父侧授权身份，不冒称执行了 JS suspend/resume 回调。
- `surface_javascript_resource` 只选当前快照 entryResourceId 的 binary/application/javascript
  租约，复用 surface_resource_http 的固定 HTTPS/Device/nonce/lease/大小/SHA-256 门。
  图片入口仍有独立 image 校验，不因复用传输层而允许脚本伪装成图片。
- `surface_javascript_worker` 在正常 profile/UI 初始化前分流；先核对 Job 限制，
  再接收继承管道，不接受外部路径或 shell 参数。
- `surface_javascript_engine` 仅在 child 内创建 QuickJS-NG runtime/context，并持有
  私有 bridge。模块只求值一次，update/点击/暂停恢复使用同一 JS 堆和闭包。
- `surface_javascript_tree.js` 独占文档 identity、live 集合、移动/Fragment 与修改准入；
  attributes/selectors/markup 分别拥有元数据反射、简单匹配与严格 fragment 解析。
  `surface_javascript_bootstrap.js` 拥有 listener 调度、生命周期和原生投影。
  apply 的 Promise 与可运行 microtask 全部处理后，才 collect 最终 DOM 和 emit。
- `surface_javascript_json.js` 只处理私有协议数据；内部复制到无原型对象/数组再编码，
  不调用插件的 inherited toJSON/getter。对外 snapshot 是普通副本，emit payload 单独
  保留标准 JSON 序列化。树的 collection intrinsic 在求值前捕获，私有数组不走可变
  iterator/species；这不扩大 child Job 的权限保证，也不等于完整引擎沙箱。
- `surface_javascript_wire` 拥有有界本地帧；`surface_javascript_event` 在父侧根据
  **当前权威 snapshot.scene** 核对动作。DOM 场景从不授予远端 action 权限。
- 原 `SurfaceWindow`/Presentation 拥有 HWND、DirectWrite/D2D 呈现、命中与候选
  身份；现有 `surface_click_event` 拥有随机事件 ID 与不可变完整 wire。

父侧使用已认证调用者交付的 `Binding` 和 `{ revision, scene, ... }`，不从脚本
接收 binding、generation、baseRevision 或 eventId。prepare event 不是发送或授权，
Application 的输入/更新桥保留原租约、FIFO、确认和快照收敛，不建立 JS 网络旁路。

直接使用 Mounted 时仍须由后台调用者处理撤权等待/refresh；Application 已由 Runtime
自动处理空闲撤权/到期，不依赖 UI refresh 才回收 child。Mounted 执行 API 仍阻塞，
不可在 UI 回调直接调用。生命周期 ACK 可独立推进绑定，但不能用它推进 scene revision，
同生命周期 revision 不允许改变状态。显式新快照可续同源码资源租约，不可静默换程序，
也不可复活已过期会话。非 Active 不允许点击/resume/交付 emit；初始 mount 可准备隐藏
内容。当前资源字符串表仍为空，不声称包内模块或图片资源解析已接线。
初始帧必须先被 owner 消费，update/click/suspend/resume 不能越过它再取出旧帧；
初始 emit 不随等待期间的生命周期 ACK 静默换绑，也不能在非 Active 状态交付。

Application 消费帧时再次复核原 Grant、完整 Binding、scene revision、原始快照与
源码/凭据期限；任何呈现失败均关闭，不用后续空 rebind 回复伪造已呈现。初始 emit
暂时明确拒绝，不静默丢弃；外部图片资源宿主仍关闭，Canvas私有像素不是资源URL。
v343 的受限输入沿原 HWND 场景身份、
重新命中和键盘焦点门进入同 child 的异步 listener；DOM 不得直接声明远端 action。
listener 返回的宿主事件以原 ID/字节/顺序进入既有 batch FIFO，最多 32 项、整批 wire
合计 64 KiB；容量不足时不重跑 listener，不重新编码或合并。监听器/update、未消费
emit 与待呈现收据期间不捕获第二次点击。默认 attach/restore 的报告仍只有 Declarative，
没有新 JS 开关或默认准入。

事件的 JS 来源只由父侧权威校验标记，不进入 wire，也不能由响应自行声明或升级。
收敛器按原事件来源检查 runtime；JS 快照的 scene 仅是动作权威，不能直接渲染。
业务 snapshot.state 与严格字符串的 lifecycle state 独立解析，合法业务对象不应
被误判为生命周期协议错误。完整批次证据通过后先暂存 resources/revision 并排 update；
同 child 的 DOM replace 成功、再核原 Grant/绑定/显示状态后才提交快照版本和原收据。
`PresentationError::Pending` 不是呈现成功，不能放行输出或后续提交；恢复记录映射为
既有 NotStarted，不改变持久格式。DOM 交付与收据结算间复用原回复事件保持最后唤醒，
不增加等待槽；关闭后仍只等线程终态，不因保留信号忙循环。

窗口关闭只发 stop，不等待后台；关闭后仅等线程终态（未消费回复不参与等待，避免
退出忙循环），收割完成才可 take_closed/移交生命周期。原两秒退出泵包含 JS 完成
句柄；失败/撤权不冒称 disposed 或 inactive，无法停用的附件继续留恢复证据。
应用先回收目录/观看/图片完成槽，再安排 JS，最后结算生命周期，避免最后完成句柄
回收后关闭窗口却漏掉本轮 inactive；后台失败不自动重取源码或重建 child。
等待准入单独计算预留预算：尚未启动的 JS 也预留窗口撤权、回复、线程完成三槽，
Live/新建/恢复都使用此预算，而不是只数当前句柄；实际等待集合不含虚构句柄。

## v354 有界 JavaScript timers

2026-10-26 已联网核对 [WHATWG timers](https://html.spec.whatwg.org/multipage/timers-and-user-prompts.html#timers)，
并只读核对 Loom stock-monitor 的 initialRefresh、tick/pending 和 refresh interval 使用点。
同一持久 child 支持函数形式 setTimeout/setInterval、最多32个附加参数、globalThis
receiver、共享正整数 ID 和 clearTimeout/clearInterval 交叉取消。延迟按 Web IDL long
转换一次（signed32，负数归零，非有限数归零，BigInt/Symbol 抛错）；timer task 的
nesting 大于5时最小4ms，interval 重登记也累进 nesting。timer字符串 handler、超过
64个同时登记、超过32个参数或耗尽正 signed32 ID 明确拒绝，ID不回绕/复用。

timers 私有工厂复用 animation 的 Instant 单调时钟，不引入 Date/系统时间或宿主
文件/网络能力。一次 host task 快照已到期项，稳定按到期时间/登记顺序，回调中新
登记留到下一 task；每个 callback 后排尽 microtask，Promise 中取消后续 callback
有效。interval 每 task 最多一次，按本次执行完成时间重新计时，不积压补发；同步
异常/死循环/自续 microtask 沿原失败策略终态回收，而非浏览器的报错后继续。
显式 Session suspend 冻结剩余等待、resume 恢复，dispose 清空登记和参数引用。
Application 模态/背压不是 JS suspend，不回滚已在途 task，也不承诺冻结墙钟。
生命周期 Promise 仍须由本次可运行 microtask 完成；不支持在 mount/update 等操作
内部 await 下一 host timer/rAF task 来维持该操作，不能以此绕过原串行和时间预算。

私有输出只交付0..2147483647ms的下一剩余等待，父侧再次验界。TimerTick 与 rAF
分开：只有 rAF 到期或真实 observer resize 才排 AnimationFrame；timer-only 不提前
执行 rAF，也不把已有 rAF deadline 反复后推。AnimationFrame 可顺便消费已到期
timers（先 timer/microtask 再已快照 rAF）；不承诺完整浏览器 event loop/VSync。
沿原 AuthorizedWindow 当前策略、confirmed owner、输入/emit/收据背压和单槽提交，
不新增线程/轮询/业务队列/授权 owner。Mounted/Runtime 保原 Grant、binding、revision、
源码资源期限及 viewport/epoch 前后复核，emit 仍走原两事件 FIFO 和收敛链。

开发和最终封存成绩见迁移台账v354；早期失败不追溯覆盖。默认JS、完整股票/Loom、
剩余DOM/CSS/输入、OS权限隔离、完整N01–N12/长稳及最终手测仍未放行。无依赖/SDK
或原heap/stack/250ms/父两秒/Job/4MiB/startup15秒预算扩张。

第二轮timer13/0/1 ignored、JS104/0/11和五个摘要绑定实际产品worker各1/0通过；
counter旧无timer断言已修正，网络/进程禁用断言保留，生产输入未改。geometry集合
和startup仍失败、18门仅16门通过，整轮partial；原失败、未知根因和默认能力门
不因该子集可用而清除，完整身份与状态见迁移台账v354/最终收据。

## v352 原生 Canvas2D 绘制链

2026-10-25 已联网核对 [WHATWG Canvas](https://html.spec.whatwg.org/multipage/canvas.html#the-canvas-element)，
并只读核对 Loom stock-monitor 的 chart/chart-interaction 实际调用。Canvas 仍由原脚本
驱动，不翻译股票业务、不嵌浏览器、不以 SVG 字符串或固定截图冒充 Canvas。私有
context/path 工厂记录有界绘制批次，child 的 WIC/Direct2D/DirectWrite 持有实际离屏
位图；PNG 只用作进程间像素编码。父侧再次核身份、尺寸、PNG类型和总量后，以已有
Pixels/Bitmap/Presentation owner 呈现。普通声明式 Scene 不取得 JS Canvas 元数据。

首个子集包含 canvas 元素及 width/height 属性反射、稳定 getContext("2d")、同尺寸
赋值也清空位图/状态/当前路径/保存栈、save/restore、setTransform/resetTransform、
clear/fill/strokeRect、begin/move/line/bezier/close/rect/arc 路径、fill/stroke、纯色/
globalAlpha/线宽/端点/连接/虚线及 fillText。字体限定1–256px加字体族列表，默认10px
sans-serif；实际测量/字形由 DirectWrite 提供。textAlign 和 alphabetic/top/middle/
bottom 基线是原生字体度量子集，不承诺浏览器逐像素字形/抗锯齿一致。Canvas 缺省
object-fit 为 fill，普通图片继续 contain；同步盒与 observer 仍来自同一原生布局。

每文档最多8个已使用 Canvas 身份、单轴4096、单Canvas 4,194,304像素、总背板32MiB；
零尺寸不分配或上传伪造像素。每operation最多8192绘制命令，路径4096段、save栈32、
dash64项；单提交1MiB、单PNG2MiB、整批base64 PNG合计3MiB，最终仍受原4MiB
私有帧和250ms总期限限制。像素
准备失败或 native 提交失败粘性终止，不靠catch/下一帧重试绕过；不增加原heap32MiB/
stack256KiB/父两秒/Job128MiB门。离屏目标只在child使用；UI只收到当前帧纯像素，
替换/设备资源释放继续归原 owner。dispose释放引用、位图、COM与预算，无新线程。

尚未准入旋转/斜切clearRect、同一路径中混合坐标变换、Path2D、clip、gradient/
pattern、drawImage、像素读写、measureText、strokeText、复合/滤镜/阴影、Offscreen/
WebGL、完整WebIDL和Canvas可访问fallback。未知context类型返回null，非默认context
options明确拒绝。不把子集当作完整Canvas、真实股票/Loom或默认JS放行；本轮实际
开发/封存结果与早期失败保留见迁移台账v352，旧v350/v348未知根因不追溯结清。

v352第二轮封存Canvas14通过/1 ignored、JS94通过/10 ignored，摘要绑定实际产品
Canvas worker1通过；首轮旧canvas拒绝夹具改video并补markup实际像素正向测试，
未改生产输入或删除原子性要求。但startup6通过/1超原15秒，16门仅15门通过，
整轮保持partial，不能把Canvas定向成功当作完整运行、默认JS或真实Loom验收。

## v351 事件驱动的 ResizeObserver 子集

本轮新增真实 `ResizeObserver`，仍是显式开发协商的受限宿主，不是完整浏览器。
2026-10-25 已联网核对 [CSSWG Editor's Draft](https://drafts.csswg.org/resize-observer/)：
按当前草案的 `(-1,-1)` 初值处理首次观察（含零尺寸），不是旧 TR 的 `(0,0)` 初值。
observe 不同步调用 callback；原 operation 的 Promise/jobs 和本帧 rAF 完成后，Rust
在 collect 前驱动观察。每个 observer callback 后排尽 microtask，新排的 rAF 留到
下一帧。登记、移除、隐藏和尺寸变化都以真实原生盒为依据，不以 viewport 变化伪造通知。

`observe` 默认 content-box，另支持 border-box、device-pixel-content-box。只接受同文档
Element 与精确 box 字符串；未知值、显式 null box、非 Element/伪造 receiver 拒绝，
不承诺完整 WebIDL 参数转换。重复 observe 重新登记；unobserve 移除观察，但已收集的
本轮 active 快照仍可交付，disconnect 同时清除尚未交付的整批；重复 disconnect 无操作。
最多32个文档 lifetime observer 身份、64个 observer-target 登记对；disconnect 释放目标，
不回收身份槽。dispose 释放全部引用；suspend 不执行观察或帧，resume 按当前布局检查。

entry/rect/size 是冻结无原型快照，不冒充 DOMRect/ResizeObserverEntry 原型类。entries
及各 size 数组可迭代；contentRect 的 x/y 为 padding left/top，width/height 是扣除
border 和 padding 的内容尺寸，不是 client box。内容内缩复用 `surface_button::insets`
与 `placed.containing_width`，包含原生按钮默认 padding，不复制布局算法。device box
使用当前 DPR 下内容起止边分别取整的差；这是本宿主 snapping，不保证逐像素等同浏览器。
当前横向排版只返回单片尺寸；隐藏/断开节点全零。被压平的 inline 仍明确拒绝，无父盒替身。

交付按节点深度分轮收集，后续 observer 在调用前重新读取当前布局。只有 skipped
变化时终态失败，不伪造浏览器 ErrorEvent 或无限下一帧恢复；callback 异常同样终止。
共享原每 operation 256 reads/8 layouts、250ms、32MiB heap/256KiB stack 和父两秒门，
观察回调与原 rAF 不增资源预算。插件私有 collection/iterator/编码 intrinsic 在求值前捕获。

`WM_SIZE/WM_DPICHANGED` 只递增合并代次，不在窗口过程求值或发网络请求。父侧从正常
Frame 得到 resizeObserved，与 animationPending 分开；只有代次未观察才通过原
sync_input/当前策略/唯一 confirmed owner/单槽/输入与 emit/presentation idle 门排帧。
模态和背压保留脏代次但不贡献过去 deadline；静止 observer 不产生16ms轮询。
mount/click/update/frame 都捕获提交代次；读布局的结果及准备好的 emit 在呈现前后和
最终提交前同时核对 viewport 与代次，A→B→A 也拒绝，不重放已运行脚本。

开发与封存验证结果见迁移台账 v351。真实 Canvas、完整 CSS/timer/股票/Loom、默认 JS、
OS 权限隔离、长稳及完整 N01–N12 仍未完成；新成功不清除原 v350/v348 未定位失败。

## v350 宿主驱动的 animation frame

`requestAnimationFrame`、`cancelAnimationFrame` 与 `performance.now()` 在同一个持久
QuickJS child 内执行，不伪造 ResizeObserver、Canvas 或 timer 对象。单文档最多64个
待执行 callback，正 u32 handle 不复用、不回绕；耗尽或容量满明确拒绝。cancel 按
无符号32位转换后删除，未知 handle 无操作。同帧快照 handle 列表、共享一个 timestamp，
timestamp 与 performance.now 来自同一私有 Rust Instant 单调时钟，不是 epoch 时间。
Rust 逐个调用 callback，并在各 callback 之间排尽 microtask；因此 Promise 取消仍能
阻止同帧后续 callback，callback/microtask 新请求留下一帧。异常或原总期限超时终态
回收 child，不提高 heap32MiB/stack256KiB/JS250ms/父两秒/Job128MiB 原门。

私有 AnimationFrame 操作沿原 Session→Mounted→Runtime 单槽和原 Grant/binding/
revision/snapshot/viewport 校验。UI owner 在真实接纳并替换帧后，按 pending 标记安排
nominal16ms deadline，复用原阻塞消息等待；无 callback 不连续唤醒，无新线程或周期
Win32 timer，不保证 VSync/60fps。真实租约 expires_at 不变，wake_at 才取最早可执行帧。

只有 shown、pointer enabled、runtime/输入/emit/presentation idle、无 pending click、
唯一生命周期 owner 在场且 confirmed_active 才可排程，并且只在 sync_input 核定本轮
应用策略后提交。外借 owner、模态或背压期间不贡献已过期帧 deadline，避免忙循环；
不在 refresh/begin_batch 抢单槽。策略变化阻止下一次开始，已经在途的帧仍可能被收割
呈现，但仍须原授权/快照/viewport 检查；不承诺进入模态会冻结已在途 DOM。

suspend 保留队列而不执行，resume 恢复待帧标记，dispose 丢弃队列。无 viewport 的
Session 不准提交 animation；窗口结果依赖过期则关闭，不自动重放或实现 resize 恢复。
通用 child、真实 HWND 像素/同步 bounds/resize、Application 原 TLS/FIFO/owner 门与
显式产品摘要门分别验证，最终成绩见迁移台账 v350。ResizeObserver、Canvas、timer、
完整浏览器语义、真实股票/Loom、默认 JS 和完整 N01–N12 仍未放行。

v350最终证据为partial：冻结源码的rAF9/1 ignored、JS112/8和严格静态通过，摘要绑定
实际产品animation worker另1项通过；但封存animation集合4 passed/5 failed/1 ignored，
并有其他child/原生geometry/TLS/UIA失败及SERIAL连带。没有提高预算、删断言或用成功
单项清除失败；完整矩阵、身份和未定位根因见迁移台账v350/closeout，不作完整验收。

## v349 受限定位与容器裁剪

`surface_javascript_css_spatial.js` 只做空间声明准入、shorthand 展开和上下文检查；
最终坐标仍由原 `surface_position`、NativeMeasure/Tree 求值，裁剪、圆角、绘制和命中
仍由原 Presentation owner 消费。没有第二套 JS 布局、浏览器或股票业务实现。

| 声明 | 精确子集 |
| --- | --- |
| position | static、relative、absolute；fixed/sticky 不接受。 |
| top/right/bottom/left | auto、0、非负 px/百分比；px≤8192、百分比≤1000，最多256 ASCII单元。负位移可用 relative 的 right/bottom 正值表达；不接受负数/calc/var。 |
| inset | 一至四值，先完整验证，再独立保存 top/right/bottom/left；short/longhand 共用 cascade、important、反射和删除。 |
| overflow、overflow-x/y | visible/hidden；shorthand 一或两值。只允许容器，最终 winning 两轴必须相同；混合值会计算为 auto，未接 JS 滚动状态前明确拒绝，不暗中开放滚动。 |

absolute 只允许直接 relative/absolute 父，父不能是 Grid；根可用宿主 viewport。
每轴至少一锚；只有单锚时必须有显式非 auto 尺寸，不用近似 intrinsic 冒充 shrink-to-fit。
缺省 width 显式投影 auto，双锚才可缩成剩余空间；父包含块为扣 border 的 padding box，
不是扣 padding 的 content box。absolute 不占正常流，不接受被其忽略的 grow/self alignment。
relative 偏移保留原占位；尚无 definite-height 分析，非根 relative 的非零 top/bottom:%
明确拒绝，根相对宿主 viewport 可求值。static 的非 auto 锚点也拒绝。

这不是完整 CSS stacking/containing-block。除根及根直属 relative 项外，定位节点的
直接父必须定位；普通静态子树不能隐藏需要提升绘制的定位后代。每级可见兄弟只允许
静态前缀、定位尾部；none/空白文本不参与。违反即拒绝，不以原生 DFS 源码顺序冒充
CSS 定位绘制顺序。隐藏声明仍检查语法，恢复显示重新计算排序。压平 inline 仍无独立盒。
z-index、margin、transform、fixed/sticky、最近定位祖先搜索、自动滚动和完整格式化上下文
仍未实现；因此不能执行整份股票 stylesheet 或声称 Canvas/真实股票验收完成。

尺寸/长度的 `.N` 与 `N.` 十进制同步规范化为原生 Length 可消费形式，堵住单锚尺寸
已获准但落入 native intrinsic 默认值的组合入口。私有 shorthand 四边/两轴及原 Grid
共用有界 names 列表；可选 quad 仅读自有属性，不分派插件原型 getter。原资源/时间/
栈/Job 门不变，无依赖或默认能力变更。真实 child、隐藏 HWND 的像素/命中/同步 bounds、
Application 原 TLS/FIFO 和封存产品分别验证；最终结果见迁移台账 v349/closeout。
ResizeObserver/rAF/Canvas、完整 CSS/真实 Loom/股票、默认 JS、OS 权限隔离、长稳、
最终用户手测与完整 N01–N12 仍开放。

本轮最终源码/编译门通过，spatial 11 passed/1 ignored、JS103/7、geometry15/1、
相关授权/Application/原生窗口回归与3,559文件行数门通过。六程序封存后无重编运行
254.05秒、exit0：spatial11/1、Application JS10、实际产品 spatial/geometry/Grid/
CSS/文档/旧worker各1项及原预算/资源门通过；startup7/0，原单child15秒不变。
ignored不计作通过，产品门另行绑定EXE/外部预期摘要。源码、六程序和旧.339–.348
封存产品身份一致，相关进程残留零；精确身份见迁移台账v349/closeout。原v348
超时仍未定位/未修，本轮成功不清除旧失败，也不代表全库或完整迁移验收。

## v348 同步原生几何与 viewport 依赖

Element 的 `getBoundingClientRect()` 返回冻结的无原型快照，含 x/y/left/top/right/bottom/
width/height；不是 live DOMRect 或完整 CSSOM View。clientWidth/clientHeight 是 border-box
扣除两侧 border、保留 padding 后的正数 Math.round；offsetWidth/offsetHeight 是 border-box
的整数尺寸。当前无 scrollbar/overflow CSS 支持，不声称已实现浏览器滚动条度量。
断开节点返回零；投影内隐藏节点及其后代的 box 为零。叶内压平 inline 没有独立 box，
读取明确抛错，不以父框冒充。没有显式 viewport 时，连接节点几何和 devicePixelRatio
读取抛错，普通离线 DOM 仍可使用；这类准入拒绝可 catch，但不产生伪造尺寸。

`surface_viewport` 统一生产 Host 的字体/context 与客户区/DPI 合同；真实 HWND 使用
GetClientRect/GetDpiForWindow，逻辑尺寸为 pixels×96/dpi，devicePixelRatio=dpi/96。
后台 initial/click/update 捕获当时实际 viewport；child 私有 geometry callback 将当前
DOM 投影交给同一 Scene→NativeMeasure→PreparedScene/DirectWrite 八轮重排，不另造布局
算法。每 operation 最多 256 次原生读取、8 次不同投影测量，只缓存一份完整投影字符串；
DOM 修改后重新测量。原生失败、超限及返回时过期均粘性失败，即使插件 catch，末尾也
拒绝整 operation。原 32 MiB heap/256 KiB stack/250 ms/父两秒/Job 128 MiB 不提高。
同步 native 调用不能由 JS interrupt 中途打断，返回后检查期限，父进程门负责最终兜底。

仅实际读取布局/DPR 的帧绑定 viewport；UI 在 replace 前后复核客户区/DPI，staged revision/
resources 在成功后才提交。emit 暂存、取出和 cancel 返还保留同一依赖，最终 begin_batch
在关闭 submission 的同步重入之后再次核对。过期则终态关闭，不重跑 listener、不发旧
尺寸事件，也不改变原随机 ID/字节/顺序/FIFO/Grant owner。仅凭 viewport 数值相等不表示
未发生过 resize；本切片不提供 resize 历史或自动恢复合同。

为保持原栈预算，cascade/projection 的私有 Map 遍历使用预捕获 entries/iterator next，
生命周期由已有 async apply 直接调用，不额外套 async invoke；保留 this、单个 undefined
实参、await、异常与 cleanup 顺序，所有可运行 jobs 仍在 collect 前排尽；不承诺旧包装
层数的精确 microtask 排程。通用 child、真实 HWND/像素/命中、Application 原 TLS/FIFO 及产品摘要绑定
门分别验证；最终结果与早期失败见迁移台账 v348/closeout。

这是同步读取子集，不是 ResizeObserver、requestAnimationFrame、timer、Canvas、完整
position/overflow/media/inline flow 或真实股票/Loom 验收。resize 后只有下次显式 click/
update 才运行脚本；竞争中的旧结果保守关闭，不以自动重放修复。默认 JS/capability 不变，
完整 N01–N12、OS 权限隔离、长稳及最终用户手测仍开放。

本轮最终源码/严格编译门通过，geometry 15 passed/1 ignored、JS92/6、相关原生/
授权/Application回归和3,552文件行数门通过。封存后同字节geometry15/1、Application
JS10及显式实际产品geometry worker1项通过；ignored产品门是另行摘要绑定运行。
但整体封存运行仍为partial：startup4/3，三个完整smoke超原15秒。后来单场景诊断
通过未定位原失败，不能称启动问题已修。源码/六程序/旧封存件身份一致且进程残留
零；完整证据见迁移台账v348/closeout，默认JS和完整迁移门不因此放行。

## v347 CSS Grid 到同一原生布局链

`display:grid` 仅在既有 div/section/article/header/footer/main 容器上开放，原文字/按钮仍
是叶节点。css_grid_tracks 以非递归有界扫描规范化轨道；css_grid 独占 shorthand 展开、
端点与对齐语法，declarations 在层叠前保存独立 longhand，projection 才合成原生键。
负线、隐式轨道、测量、放置、对齐、像素与命中继续由 [原生 Grid](SURFACE_GRID.md)
求值，不复制浏览器布局或业务模板。默认 JS/能力不因此开放。

| 新增声明 | 原生消费与明确边界 |
| --- | --- |
| grid-template-columns/rows | none、auto、非负 px/零、正 fr、minmax、固定 repeat；百分比只在确定轴求值。固定 repeat 展开并规范化为轨道串，不声称完整 CSSOM 字符串序列化。 |
| grid-auto-columns/rows、grid-auto-flow | 单个轨道；仅 row 稀疏放置。 |
| grid-column/row、对应 start/end | auto、非零正负线、正 span；单项 shorthand 将另一端重置 auto。负线原样交原生，双 span 拒绝。 |
| gap、row-gap/column-gap | 一/两值非负 px，顺序 row/column；flex 仍只消费不换行主轴 gap。 |
| justify-items/self、align-self/content、place-items/self/content | 每轴支持值与原生合同一致；横轴 baseline 不支持。CSS self normal 对非替换 Grid item 映射 stretch，auto 跟随父级 items，不改变 Scene 历史 normal 语义。 |

gap、grid-row/column 与 place-* 在 sheet、cssText、属性赋值和 setProperty 共用先全量
验证再展开；各 longhand 独立比较 important/inline/specificity/order。setProperty 显式
覆盖旧 important；removeProperty(shorthand) 移除两项。只有两项均存在且 priority 相同，
getter 才重组 shorthand；length/item/cssText 暴露 canonical longhand，不伪造完整 CSSOM。

Grid container 声明要求显式 grid，item 声明要求真实 Grid 父；可见的普通 flow、flex
与压平 inline 后代不能保存后忽略。none 可保留已知声明，恢复可见时重新检查；隐藏
子树仍计算/验证。Grid item 缺省 width 投影 auto，避免普通原生容器 full-width 回退。
flex 保留旧单值 gap，并明确归一化 start/end/normal；未实现的 flex baseline/content
stretch 或 align-content 仍拒绝，不以新增 Grid 枚举放宽为完整 Flexbox。

轨道输入/展开输出均不超过 4,096 ASCII 单元，每轴最多 64 轨道，单 bound 至多 256；
px≤8192、%/fr≤1000，正 fr 还必须可表示为正 f32。端点绝对值/span≤2049；布局额外
范围/探测/贡献/cache/八轮收敛预算由原生 owner 检查。所有新操作复用捕获 intrinsic，
含 fround；不提高原 heap/stack/时间/Job 门，不新增宿主 I/O、线程或依赖。

验证 owner 为 grid_model_tests、grid_tests 与真实 HWND javascript_grid tests，包含
shorthand reset/混合 priority/拒绝原子性、intrinsic 污染、同 child 点击/update/关闭、
真实 DirectWrite/D2D 像素/同几何命中、隐藏不占位和原生失败替换保旧帧。实际产品门
单独绑定封存 EXE/外部预期摘要；最终结果见迁移台账 v347，不代表真实股票/Loom、
完整 CSS intrinsic/DOM/Canvas、权限沙箱、长稳或 N01–N12 验收。

最终 JS Grid 定向/同字节封存门为 17 passed / 1 ignored；ignored 的实际产品门在
外部摘要绑定后显式运行，一项通过。真实 HWND 三项、私有 intrinsic 跨帧/原子拒绝
和相关 JS/Surface/Application 回归通过；严格静态及 3,543 文件行数门通过。2,391
源码输入与六程序身份一致，旧 .339–.346 产品不变、相关进程残留零；精确产物与
早期失败见迁移台账 v347/closeout，不把小夹具内存或测试总量提升为完整产品验收。

## v345 constructed stylesheet 与原生样式子集（v347 扩展 Grid）

CSS 不再只保存字符串：五个私有工厂 css_values/css_selectors/css_declarations/
css_sheets/css_projection 分别拥有值准入、匹配、inline 声明、采用/层叠和真实 Scene
转换；bootstrap 仍拥有生命周期/listener，原 Presentation 继续度量、绘制与命中。
没有新浏览器、线程、网络/文件宿主、默认 capability 或股票业务分支。

最终 v2 的开发 JS 门为 60 passed / 4 ignored；六程序封存后 CSS 12/1 ignored、
Application JS 七项和显式实际产品 CSS worker 一项均通过，包含真实 HWND/生产
Presentation 的几何、DirectWrite/D2D 像素、隐藏命中和同 child 更新/回收。产品
CSS 小夹具 Job 峰值 4,800,512 字节，不代表完整应用内存；原 heap/stack/时间门
不增。源码和同字节产物身份、其他回归、失败保留见迁移台账 v345 与本轮 closeout。

`new CSSStyleSheet().replaceSync(source)` 和 `document.adoptedStyleSheets` 可采用同一
sheet；getter 返回副本，原数组的 push/filter 不直接改私有采用列表，setter 显式提交。
replaceSync、采用列表、style.cssText 都先完整解析/预算/brand 检查再提交。sheet 的
cssRules/insertRule/异步 replace、URL/@import/媒体查询未开放，未知声明/选择器失败，
不以空规则、默认颜色或原字符串假装成功。style 属性字符串/HTML style 属性仍拒绝。

层叠比较 important、inline、ID/class/type specificity、source order；selector list
仅计算实际匹配项的最高 specificity，background/background-color 共用同一声明。
CSS matcher 支持 type/`*`、复合 ID/class、属性存在/quoted 或简单 unquoted 精确值、
逗号列表、descendant/直接父、`:root/:disabled/:first-child`。它与文档查询的旧简单
子集分开，不因此广告 querySelector 已支持全部 CSS。不支持转义、hover/focus、nth、
兄弟组合器、伪元素或函数选择器；sheet 中的引号属性值也不接受花括号或 @，不是完整
CSS tokenizer。祖先可达状态去重，不做组合式回溯。

Element.style 提供 camelCase 属性、cssText、length/item、getPropertyValue/Priority、
setProperty/removeProperty；空值移除，重新回到当前 sheet cascade。class/data/disabled、
sheet replace/采用顺序和 inline 的变化在同 child 下一帧重新计算，不缓存过期样式。
color/font-size/font-weight/white-space/text-align/绝对行高按树继承，显式投给按钮以免
其原生默认色/居中截断继承。父 important 不会越过继承规则压过子节点的自有声明。

| 已开放声明 | 实际原生语义与限制 |
| --- | --- |
| display、flex-direction、gap、justify-content、align-items、flex-grow | flex/grid/none；Grid 见上节。flex 为 row/column 单轴不换行，direction 要求显式 flex（隐藏时允许保留），grow 要求 flex 父，范围 0–100。不是完整 CSS Flexbox/auto min-content 算法。 |
| width/height、min/max | px、百分比或零，只有 width/height 接受 auto；px ≤8192、百分比 ≤1000。尺寸/盒内缩要求该元素显式 box-sizing:border-box，不把缺省 content-box 当相同。flex item 的缺省 width 映射 auto。 |
| padding、border-width/radius/color/style | 单一统一值；padding/边宽/圆角仅 px 或零。正边宽必须显式 solid，solid 未给宽度取 medium=3px；其他 border-style、四边/shorthand 不开放。 |
| color、background/background-color | 原生有限 named/hex/传统逗号 rgb(a) 纯色，通道必须可表示为有限 f32；无 gradient/var/color-mix/URL。 |
| font-size/weight、white-space、text-align、line-height | px 字号 1–8192，整数 weight 100–900 或 normal/bold；normal/nowrap/pre/pre-wrap，left/center/right。行高必须 px、具有显式或继承字号且不小于字号；不声称 unitless/normal/小行高兼容。 |
| opacity | 0–1 的真实原生子树组透明度；零透明度不是 display:none，仍保留原命中语义。 |

未指定 CSS 的节点仍采用 Otter 原文档垂直容器和原生按钮基线，不声称 HTML UA 样式、
block/inline flow 或浏览器默认尺寸等价。文本内容始终裁自身内容盒；不开放 overflow、
position、margin、font-family/style、完整 Grid、Canvas、尺寸 observer、timer/rAF 等能力。
叶文字/按钮的 inline 后代仍被压平；若需要独立的 box/run 样式则拒绝，不丢样式继续显示。
`display:none` 真正退出原生几何/文字/绘制/输入，根容器仍可成为原始几何命中的回落值。

每会话累计至多 16 sheets，单源 65,536 UTF-16、单 sheet 128 rules、采用后合计 256
rules（重复采用也计数）；每 rule 至多 16 selectors，每 selector 512 单元/8 个复合项/
每项 16 tests。私有数组/记录无原型，CSS 的集合、字符串、正则与数值 intrinsic 在插件
执行前捕获；不经插件可变 Symbol.split/exec/toJSON 或 inherited setter 编码私有状态。
最大合法组合仍受原 32 MiB/256 KiB/250 ms/父侧两秒门约束，不保证全部最大组合成功。
这不扩大 Job 的 OS 权限保证。验证 owner 为 css_model_tests/css_tests 与真实 HWND
css_tests，实际产品门另需封存 EXE 和外部摘要；最终证据见迁移台账 v345。

## 已定义的文档与原生投影子集

支持 Node/Element/HTMLElement/HTMLButtonElement/Text/DocumentFragment 的稳定 identity。
div/section/article/header/footer/main 未指定 CSS 时映射原生垂直容器；span/strong/em/i/b/
p/h1/h2/h3 和 button 映射文字/按钮，不承诺浏览器字号、粗体或布局。文字叶/按钮只
接受文本及内联文字后代，不将可交互后代压平。容器间纯空白文本保留在文档树，但
不生成空白原生行；这不是完整 HTML whitespace/CSS flow 实现。

document 提供 createElement/createTextNode/createDocumentFragment/getElementById 和
querySelector/querySelectorAll。树支持 textContent、appendChild/append、replaceChildren、
removeChild/remove、parentNode/parentElement、contains，以及 live children/childNodes 与
first/last Child/ElementChild。插入会移动原节点，Fragment 仅转移其内容；重复 Fragment
与单节点参数按顺序模拟消费，按最终树检查深度，全部循环/深度/叶节点准入通过后才改原树。querySelectorAll
返回静态有序集合，支持 `*`、单标签、简单 `#id`/`.class`、`[attr]` 和 quoted `[attr="value"]`；
closest 包含自身。组合器、伪类、转义等未实现选择器明确失败，不伪造空匹配。

innerHTML 是最多 65,536 UTF-16 单元的严格 fragment 子集：配对标签、quoted 属性、
文本、注释、基本命名和数值 entity；先解析脱离树再交换。不是 HTML5 容错 parser，
不支持 unquoted 属性、void input/img、script/style、Canvas 或自动补标签。元数据仅
接受 id/class/title/role/data-*/aria-*，以及 button 的 disabled/type=button；每节点
最多 64 属性、每值 4,096 单元。id/className/title、classList、dataset 与属性同源，
data-* 通用反射 camelCase，Map/无原型字典不把 __proto__ 当原型操作。class/data/title/
ARIA 仅为文档元数据，不声称 tooltip 或完整 UIA；class/data 参与上述 CSS 匹配。style/URL/on*
属性仍拒绝。click listener 添加/移除与祖先冒泡保留，原生命中 target 是实际按钮，
不是标签内部 span/strong 的几何命中。投影只传输有界数据，父侧再次检查 Scene/动作。

文档遍历、序列化、查询、投影和私有 JSON 复制采用显式有界栈，避免合法深度与私有
intrinsic 调用叠加耗尽既有 256 KiB 引擎栈；不通过提高栈、250 ms 或两秒期限修复。

mount 接收 `{ root, snapshot, resources, emit }`；update 接收当前 snapshot/resources。
`NeuroSurface.snapshot()` 返回副本，resource 只读取宿主显式提供的字符串表，不读取
文件或发起网络请求。async mount cleanup 先于 module.dispose，随后撤销文档/监听器，
正常退出和 EOF 才完成 dispose；异常/取消/传输失败则终止并等待自有 child，不重放。

click listener 可使用 Promise；未决且没有可运行 job 的异步操作不伪造成功。
没有 timer/rAF 宿主。suspend 禁止输入/emit，resume 恢复同一闭包；update 只接受
严格增长 revision。旧 reader、存储与默认能力不变，丢弃显式开发 Session 即可回退。

## 资源与信任边界

| 边界 | 当前值 |
| --- | --- |
| 源码 / 权威快照 | 各最多 1 MiB |
| 资源字符串表 | 最多 256 项、512 KiB |
| 私有帧 / 返回场景 | 4 MiB / 原 Scene 1 MiB |
| 文档节点创建 / 投影深度 | 每会话累计 2,048 / 32 |
| fragment 源码 / 属性数 / 单属性值 | 65,536 UTF-16 单元 / 每节点 64 / 4,096 单元 |
| listener / emit batch | 每节点 16 / 每操作 32 |
| emit payload | 当前子集 60 KiB UTF-8 JSON，保留 envelope 余量 |
| 完整事件 wire / 一次 listener 的整批 wire | 仍为原 64 KiB 上限 |
| 引擎 heap / stack | 32 MiB / 256 KiB |
| 引擎单步墙钟 / 父侧操作 | 250 ms 中断 / 2 s 绝对 IO 期限 |
| Job 内存 / 进程数 / 容量池 | 每 child 128 MiB / 1 / 最多两 child |
| child 空闲 | 无轮询；断连退出，最长五分钟无操作后退出 |

终止后的 Windows 内核/驱动清理不承诺硬实时返回。帧/语义/序号/执行失败均使会话
失效，禁止沿可能部分传输的旧协议流重试。普通输入预校验失败没有发送帧，可保留
原会话。没有任意模块 loader、Node、fetch、WebSocket、系统文件或网络宿主 API。

**Job 不是权限/文件/网络沙箱。** QuickJS-NG 是本机 C/FFI 引擎，内存破坏后仍可能
取得当前用户权限；不能将不提供宿主 API 等同于 AppContainer。完整不可信插件的
默认准入、权限隔离及应用集成仍开放。不得在默认 capability 中提前启用本子集。

## 依赖、验证与未完成项

Windows 精确依赖 rquickjs/core/sys 0.14.0，关闭默认 feature，只开 std；未启用
loader/dyn-load/futures/parallel/rust-alloc。vendored QuickJS-NG 声明 0.16.2，固定
上游 commit `0fdea21ff1090084e91dad812b343d92e79ba9d9`；构建四个 C 引擎源，不编译
quickjs-libc/qjs。Windows MSVC 是上游 experimental 平台，必须依赖本机实际门。
MIT notice 保留于 [第三方声明](../../THIRD_PARTY_NOTICES.md)。现有 native lock 纳入
五锁安全清单；v340 曾定向修复 native TLS 两包为 rustls-webpki 0.103.16 /
webpki-roots 1.0.11，复扫 native 无返回匹配，但全仓严格门仍失败。OSV 的 Rust 包
结果不覆盖全部 vendored C 风险，见
[依赖安全](../security/DEPENDENCY_SECURITY.md)。这不是完整供应链或发行批准。

rquickjs-core/sys 的 crate 包未带仓库级 MIT，不能以 SPDX 字符串代替材料。
候选收集器由 javascript_licenses 显式补充同解析图 rquickjs 0.14.0 的原 LICENSE，
并保留 sys 的 quickjs/LICENSE；两份文本均按上述固定上游 commit 的联网原件 SHA-256
绑定。收集阶段不联网，不接受其他版本/源/表达式或篡改文本，仍保留缺失即失败、
64 MiB 总预算和 legalClearance=false。首次材料门失败原件保留，修复不等于法律批准。

定向库测试入口（独立 target/TEMP 应放仓库外 linshi）：

```powershell
cargo test --manifest-path native/Cargo.toml --locked --offline --lib surface_javascript -- --test-threads=1
cargo test --manifest-path native/Cargo.toml --locked --offline --lib javascript_dom_projects -- --test-threads=1
```

前者覆盖真实 child 状态/更新/emit/销毁、取消/drop、CPU/heap/非法 import、wire 与
父侧权限复核；后者使用自有隐藏 HWND 实际 D2D 绘制、客户区命中和定向鼠标消息，
消费原候选身份门后回到 JS listener，再投影新权威快照并关闭。它们不是自动产品
attach/FIFO/真实 Loom 动作或前台用户操作验收。私有 worker 裸启动必须在读 profile
前失败；完整默认产品 startup 必须独立验证，不以此处测试数量替代。

v340 最终定向门为 12 passed / 2 ignored。实际产品门另外显式运行 ignored 的
`javascript_actual_product_worker_preserves_state_and_reaps`：调用者提供封存产品绝对
路径与外部预期 SHA-256，经生产 start_at 启动其私有 flag，验证保状态/async/update/
emit/dispose/回收及前后摘要，1 passed；该负载 peak Job 为 4,263,936 字节，仅表示
这个小夹具，不是整个应用/股票/长稳资源指标。封存后的资源/启动/入口门 2/1/7/2
passed，原期限不变。默认产品 UI/attach/FIFO/真实 Loom 的 JavaScript 链仍未验证。

v341 的首轮预算测试失败保留。旧 v340 封存测试程序单独运行也失败，隔离阶段探针
显示 mount/emit 达到 250 ms 墙钟中断；没有证据将其归因于新增授权线程泄漏。payload
计数改为 UTF-16 长度下界、原生 ASCII 快路和有界 code-unit 扫描，正确合并 surrogate
pair；不改变 60 KiB UTF-8 JSON 上限、父侧 64 KiB 或任何执行期限。保留原负载，另验
ASCII、两/三字节字符、补充平面字符及 JSON 转义的精确边界，不通过扩大预算修复。

v341 最终定向 22 passed / 2 ignored，含真实 TLS/Device 请求头取码后的隐藏 HWND/D2D 点击、
同 child 更新/销毁、跨 Grant/摘要/重定向/非法 UTF-8、取消下载、撤权/过期/换程序和
初始 emit 顺序门。非 desktop Surface 624/12 ignored、SurfaceWindow 215/11 ignored、
设备会话 127、授权 84、Application attach/image 2 及严格静态/行数门通过。集合重叠，
没有运行完整桌面库，不证明 Application JS/FIFO/真实 Loom 已接通。
网络测试使用固定测试设备会话的 loopback TLS peer，核对实际请求头与字节；不等于
真实设备配对、服务端批准、Loom 资源授权实现或用户在线插件已通过。
所有编译结束后封存 .341 产品和五个测试程序：预算测试固定三轮各 2 passed，实际
产品私有引擎 worker 门 1 passed，资源生成/内嵌资源/startup/私有裸入口 2/1/7/2
passed；目标与副本摘要前后相同。小夹具 peak Job 为 4,243,456 字节，不是整个应用
资源验收。该产品 worker 门不走授权入口获取、默认产品 UI/FIFO 或实际 Loom。

v342 已接入真实 Application 的授权挂载/DOM/激活/等待/关闭子链，定向 JS 为
31 passed / 2 ignored；另在全部编译结束后封存的同字节 library-test 程序运行四项
Application 门，覆盖 TLS 请求次序、隐藏 DOM 后 active/show、空闲撤权、关闭 owner
收割，以及未广告/损坏源码不取码或不激活。peer 使用固定测试设备会话，不冒称
真实 Loom 配对/批准。设备会话/授权、窗口、资源、shutdown/Live 预算和严格静态门
也已通过，精确数量与源码集合见迁移台账 v342；未运行完整桌面库。
六个封存程序的固定三轮预算各两项、实际产品私有 worker 一项及资源/启动/裸入口
2/1/7/2 项通过，目标/副本和旧 .339–.341 产品摘要不变；250 ms/两秒/每启动 child
15 秒原期限不变。此前 v2 源码集变更由身份门正确拦截并保留，不混用其编译结果。
本轮没有新的在线安全扫描或完整产品 Application JavaScript/输入/FIFO/真实 Loom
验收；默认 JS、初始 emit 和图片宿主仍关闭。证据：
linshi/otter-native-v342-javascript-application。

v343 的 JS 定向 37 passed / 2 ignored；全部编译结束后再用封存同字节 library-test
程序运行 Application JS 七项，含两次真实 HWND 点击、每次两项原事件顺序提交/各自
观察、同 child 闭包与快照更新，以及 update 失败/响应 runtime 切换不呈现、不重放。
测试 setup 不手动开输入，Application 生产准入负责开启。peer 为固定测试会话的真
loopback TLS，不是实际 Loom 配对/批准。业务 state 对象解析缺陷的失败与修复回归、
五秒夹具超时和 v1 Clippy 失败均保留；最终仅采用 v2 相同源码集合。
严格静态/版本/材料/3,504 文件行数及相关回归通过，八新文件 27–234 有效行，无新
例外；详细数量见迁移台账 v343。六封存程序的预算固定三轮各两项、实际产品私有
worker 一项及资源/startup/裸入口 2/1/7/2 项通过，源码及旧 .339–.342 产品摘要不变。
私有 worker 负载 peak Job 4,198,400 字节，不是整个应用资源指标；没有新在线安全
扫描、完整桌面库/实际产品菜单 JavaScript/真实 Loom 或完整权限隔离验收。
默认 JS、初始 emit 和图片宿主仍关闭，本轮证据在 linshi/otter-native-v343-javascript-input。

v344 扩展上述文档子集，开发定向为 11 passed / 1 ignored。真实 child 覆盖树/反射/
查询/委托/update/回收、非法操作原子性、预算，以及插件改写 intrinsic/setter/toJSON
后 collect/update/dispose 的私有完整性；同一 BOOTSTRAP 的 QuickJS 模型门用于异常
stack 诊断，不替代 child 预算。原拒绝/栈耗尽失败保留，全部编译及产品封存验证另见
迁移台账 v344，不复用 Node 内存探针作为原生验收。旧 runtime_tests 保原字节。
最终只采用 v2：JS 综合 48/3 ignored、同字节封存 Application 七项、文档 11/1 ignored
通过；实际产品文档 worker 的 ignored 门以封存 EXE/外部预期摘要单独运行，一项通过，
新文档小负载 peak Job 4,595,712 字节。原产品 worker、固定三轮预算、资源/启动/裸入口
门通过；原期限不变。2,362 源码输入冻结/构建后/运行后相同，六程序及旧 .339–.343
产品摘要不变；严格静态/28 材料合同/3,514 文件行数门通过，无新例外。v1 的旧 dom:3
测试假设失败保留，仅改为从实际 Scene 取按钮后重冻，不改生产命中/授权。固定会话
的真实 TLS Application 路径不等于真实 Loom 配对/批准；默认产品/完整插件仍未放行。

后续沿同一引擎扩展剩余 HTML/selector、CSS/stylesheet/布局、表单编辑/focus、Canvas2D/
字体/坐标、ResizeObserver、timer/rAF、包内模块与完整应用生命周期。真实股票插件
依赖上述能力，当前不能运行；不为股票函数写专用 Rust 翻译来伪造兼容。
