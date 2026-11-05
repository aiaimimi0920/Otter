# 原生 Surface 图片迁移：合同与真实基线

## 当前状态

v310 的共用宿主报告现声明 image、loom_resource 和 remote_resources，解决实际
协商合同按缺失 requiredNodes/requiredCapabilities 拒绝图片 Art 的缺口。该声明
对应现有有界静态图片和租约传输，并非通用浏览器图片/来源支持；动画、未支持
格式、脚本、http/file/blob 不因此放行。来源优先级及全部授权/撤权规则不变。

v309 组合回归不再预造已激活挂载：真实本地 TLS 续签、attach 和 active ACK 后，
正常 Application poll 自动请求租约资源、准备内联 PNG、安装并上传当前窗口，
关闭后真实 inactive ACK 返还 owner。已完成但尚未安装的结果遇撤权会丢弃、
关闭 HWND，保留最后已确认 active 而不伪造 inactive/disposed。缺少租约的非空
resourceId 仍隐藏且不回退有效 src；对应旧测试错误断言已复现并修正，生产规则
未放宽。图片及表单应用定向共 17 项通过；完整门禁记录于迁移总计划。
这是隔离身份/本地 TLS 对端证据，不是用户 Loom 服务或完整图片能力声明。

v204 小数 SVG 的逻辑尺寸已通过 v2 worker 协议及 Resource/Inline Ready 安装链
到达 Bitmap，不再从 ceil 后栅格推导布局比例。安全边界及剩余视觉差异归 SURFACE_SVG.md。

v203 Surface Bitmap/测量/contain 已分开浮点逻辑尺寸和整数栅格；当前解码/安装
默认仍为整数尺寸。SVG child 元数据与边缘采样尚待接通，详见 SURFACE_SVG.md。

v202 首批静态 SVG 已通过共享 Job child 接入 Image 与 Inline；根自然尺寸独立计算，
全树安全准入后栅格化，父侧完整验证后发布。文字/动画/CSS/foreignObject/图片/滤镜
等尚未完成项明确失败，保留完整迁移合同。详细范围与验证归 SURFACE_SVG.md。

以下为历史检查点：

v201 现有 WebP 已改接共用 image worker host/wire/process owner，保留一个 Job/
容量池和原像素行为。SVG 仍未开放，下一接线为 viewport/资源策略与 renderer，
详见 SURFACE_SVG.md；不再重复实现受限进程生命周期。

2026-10-22 SVG 接入前实测：浏览器与 resvg 候选在自然尺寸、百分比、字体、
foreignObject/动画和本地 href 权限上存在明确差异。完整样本与接线边界归
[SURFACE_SVG.md](SURFACE_SVG.md)。尚未放开 SVG MIME，生产仍为 v0.2.33.200；
非 base64 data URL 的字节解码已有实现，不再误列为从零开发项。

v200 补真实 LUT-only WebP：限定 A2B0 mft2 RGB→XYZ 直接求值，修复与 Chromium 的
尺度及非线性差异。六个浏览器/child 对照、alpha 和边界检查见 SURFACE_WEBP_ISOLATION.md；
不是全 LUT/ICC 兼容，不改变其他格式或来源授权。

v199 静态 WebP 的嵌入 RGB ICC 已在受限 child 内转换到 sRGB，再预乘；借用 metadata、
每批最多 4096 像素，不新增全图输出缓冲。P3 浏览器对照、坏 metadata/非 RGB fallback
及内存错误不降级规则见 SURFACE_WEBP_ISOLATION.md；不是全格式/显示器/HDR 色彩管理。

v198 实测纠正 WebP EXIF 假设：Chromium 对八方向 × 大小端 × 两种前缀共 32 个
有效样本保持编码方向；独立 EXIF 读取确认标签有效。原生同编码夹具保持原始尺寸
和像素，不照搬 JPEG/TIFF 旋转。详见 SURFACE_WEBP_ISOLATION.md；这不是完整元数据支持。

v197 补齐八类 WebP 坏 IPC/异常退出注入与合法对照，全部经过真实 Job child 和
生产父侧接收链；额外覆盖共享名额等待中取消和 deadline。没有扩大格式/来源能力，
元数据、动画、Job 分配失败和全进程峰值仍待推进，详见隔离文档。

v196 已实际接通静态 WebP：有限 RIFF 预检后进入同 EXE 的短命 child，Windows Job
限制单 child 128 MiB、最多两个并行，其余在截止时间内可取消地等待；结果仍经过
共享 Pixels、原 Job/Ready、同 Grant、原始快照和 HWND 安装门。支持有损/无损/alpha，
不依赖可选 WIC WebP codec；资源和内联真实应用路径已验证。详见
[WebP 隔离接入条件](SURFACE_WEBP_ISOLATION.md)，含非权限沙箱、未覆盖故障及元数据边界。

2026-10-21，内部版本 v0.2.33.199。在 v179 旧实现来源/布局/事件基线之上，
已实现安全获取、后台静态解码、任务生命周期、到期回收、图片测量与呈现入口、窗口当前资源快照，
显式像素已接通真实 HWND 创建/替换，授权 Ready 安装和到期撤图入口已实现；
应用已接通当前 active 场景 resourceId 与受限 data:image/ 的自动请求/投递调度；尚未宣称完整图片
来源、格式、布局和在线业务能力。宿主能力报告未因此直接扩大为通用 image 支持。
未经过可用性投影的普通 NativeMeasure/Presentation 仍拒绝缺少当前解码尺寸的 Image；
本地明确标记的内联替代内容使用独立测量，不走成功图片自然尺寸入口。
显式 with_images 入口已走真实生产测量与绘制，不等同资源来源已获授权。

### v195 普通 Surface 的受限静态位图嗅探

旧普通桌面 `resource_transfer.rs` 只验大小/摘要及 MIME 非空 ASCII/160 字符，不要求
声明与容器匹配；除精确 `application/x-neuro-rgba8` 外直接生成 data URL。内联则仅
筛大小写敏感 `data:image/` 前缀，最终由 img 解码。Tile/Wall 的严格 MIME/容器匹配
是另一合同，本轮不修改。旧 resourceStore 的 data URL 正则会拒绝 MIME 参数；该缓存
字符串限制不是图片安全授权，原生解码不复制它，因此不把资源参数接受宣称为旧缓存
逐行为一致。原生参数不参与文件路径或解码器配置，也不改变原始资源摘要。

新增 surface_image_format 先以有界头部识别 PNG/JPEG/GIF/BMP/TIFF/Icon 候选，再由
WIC 复核实际容器、单帧、原尺寸及最终像素预算。错误/未知/大小写/带参数声明可以
显示已支持的真实位图；声明 image/webp 但真实 PNG 可以显示，不等于支持真实 WebP。
实际 PNG 始终检查 APNG，实际 JPEG/TIFF 始终使用其 EXIF 规则，不因伪声明跳过。
HTML/SVG/WebP 等实际字节和仅有残缺签名不能形成成功图片；精确原始 RGBA 分支不变。
有效 image/svg+xml 声明不回退位图，含 TAB/CR/LF 的声明先按旧 data URL 语义规范化，
不让控制字符绕过该分支。WIC 仍使用系统解码器选择，未证明或保证第三方 codec 不参与。

data URL 保留原始前缀及 1 MiB 来源限制，先 URL 规范化/移除 fragment，再读取 MIME
essence；百分号和 forgiving base64 行为不改。未知 MIME 不再在取字节前被白名单误拒，
实际格式准入在后台完成。原始快照摘要、resourceId 优先不回退、同 Grant、租约/撤销、
16 MiB 资源/64 MiB 单图像素预算不变；不增加 http/file/blob 来源或改变 Tile/Wall。

独立 Chromium 155.0.8059.80 基线对 16 种声明分别使用 data URL/Blob，共 32 个案例：
实际 PNG 除有效 image/svg+xml 声明外均得到 2×1；SVG 声明两个案例按预期解码失败。
这是 img 解码基线，不是绕过 DeclarativeSurface 来源筛选的授权证据：例如大写
`data:IMAGE/PNG` 仍不符合旧/新组件的内联前缀。浏览器测试会话关闭，结果和脚本保存为
`linshi/otter-v195-browser.log` / `otter-v195-browser.js`。
[WHATWG MIME Sniffing](https://mimesniff.spec.whatwg.org/#sniffing-in-an-image-context)
对 XML 有专门规则；实际 Chromium 接受 application/xml 与 image/x-test+xml 声明的
PNG，本轮选择已核对的旧 Chromium 显示行为，不自称完整规范实现或跨浏览器一致。

修改前两项 WIC 回归实际失败（大小写声明拒绝、JPEG 声明 PNG 导致错误签名检查）；
修改后检查 PBGRA、自然尺寸、实际 EXIF、伪声明 APNG 拒绝、无效/脚本字节拒绝及
规范化 SVG 拒绝。完整 Application 覆盖三种内联 PNG 声明安装/上传，以及真实 TLS
资源声明 IMAGE/JPEG、实际两张 PNG，经原摘要/大小/授权后自动安装上传。
新增 9 项回归；Application 筛选 9 passed，最终全量 2674 passed / 44 ignored
（109.94 秒），startup 7 passed（15.24 秒）。Clippy all-targets -D warnings、fmt、
3112 文件严格行数及 diff 检查通过；format 模块含测试 86、WIC 格式测试 59、data
解析 123 有效行，Application 根仍 500 行。24 个本轮文本 UTF-8 无 BOM，原生测试
进程残留零。证据及源码哈希见 `linshi/otter-v195-evidence.json`；修改前失败结果
为工具输出摘要 `otter-v195-before.txt`，不冒充完整原始日志。全量 stderr 中损坏 JPEG
诊断来自拒绝夹具，测试均通过；startup 日志的 NativeCommandError 是 PowerShell
包装 cargo stderr，退出码为 0。未执行 44 项 ignored 专项，无提交/推送/发布，
完整图片格式/来源和 N01–N12 总任务仍未完成。

### v194 JPEG/TIFF EXIF 方向

Surface 静态 JPEG/TIFF 现在读取 EXIF Orientation（1–8），输出已变换的 PBGRA 与
自然尺寸，方向 5–8 交换宽高。查询只接受 VT_UI2；缺失、非法值或其他类型使用原方向。
JPEG 查询 `/app1/ifd/{ushort=274}`，TIFF 查询 `/ifd/{ushort=274}`，依据
[Microsoft 方向元数据文档](https://learn.microsoft.com/en-us/windows/win32/wic/-wic-photoprop-system-photo-orientation)。
本轮不实现该文档中的 XMP fallback，也不声称 PNG eXIf、ICC 色彩管理或全部元数据兼容。

原始帧在查询元数据及初始化变换前通过 64 MiB 像素准入，最终变换源再次检查；WIC
源链留在后台 COM 线程，最后一次 CopyPixels 写入共享预算下的唯一显式 PBGRA 缓冲。
方向 5/7 使用先旋转再翻转的两个源链节点，避免组合枚举顺序歧义；PROPVARIANT 在
成功/失败路径均清理。检查取消边界，不宣称单个 WIC 调用可即时中断、无内部缓存或
精确测量 WIC 内部分配。普通文件/编码图及 Relay 保留原帧行为和原尺寸严格校验。

修改前 JPEG/TIFF 两项回归均在方向 2 失败；修改后八方向真实 WIC 像素位置、宽高及
alpha 检查通过。另覆盖缺失/非法类型和值、八个取消边界、超限原帧早拒绝、普通编码
图/Relay 不旋转。真实 Application 以方向 6 内联 JPEG 验证授权、后台解码、安装后
32×48 自然尺寸及 D2D 上传，无资源 GET；不是完整八方向应用端逐像素断言。
独立 Chromium 155.0.8059.80 的 JPEG 八方向尺寸/像素顺序全部一致；浏览器只产生
Canvas readback 性能提示，无页面错误，独立测试会话已关闭。此基线不代表 TIFF 的
浏览器支持。脚本和结果见 `linshi/otter-v194-browser.js` / `otter-v194-browser.log`。

新增 7 项测试；最终 Surface 解码筛选 9 passed，完整原生库 2665 passed / 44 ignored
（96.91 秒），startup 7 passed（13.72 秒）；Clippy all-targets -D warnings、fmt、
3110 文件严格行数及 diff 检查通过。方向模块 83、方向测试 219 有效行，Application
根仍 500 行。13 个仓库文本及 9 个证据文件 UTF-8 无 BOM，原生测试进程残留零。
首次全量 2664 passed / 1 failed / 44 ignored：前置尺寸门改变旧预算报错文本；已恢复
原文本并重跑最终全量，保留 `otter-v194-first-full.*.log`，不是跳过或放宽旧断言。
证据/源码哈希见 `linshi/otter-v194-evidence.json`。startup 日志的 NativeCommandError
是 PowerShell 对 cargo stderr 的包装，进程退出码 0、七项均通过。没有提交/推送/发布，
未运行 44 项 ignored 专项，不宣称在线业务或 N01–N12 全部验收完成。

### v193 无像素内联图片的可见替代内容

旧 imageSource 只以来源字符串是否存在决定是否生成 img，不以解码成功决定节点存在。
损坏 data 来源仍有 img/alt；旧 resourceId unresolved 则没有 img，不能混为同一种隐藏。
改动前新增的 Scene/NativeMeasure 两项回归均实际失败，确认内联图片被完全移出布局。

with_image_availability 现在区分未解析资源与内联替代内容。无非空字符串 resourceId、
大小写敏感 data:image/ 来源且无当前像素时，保留 Image，显示原生缺图标记和字符串
alt；成功安装清除替代内容。无源、远端 URL、file、resourceId 失败均不因此获得回退，
visible=false 仍隐藏。两个呈现标志均 serde(skip)，原始来源与授权合同不改写。

替代内容是“当前没有可呈现像素”的中性反馈，同时用于等待与失败，不伪造已收到
terminal error，也不声称浏览器加载中外观完全相同。本轮不是逐像素复刻浏览器错误
图标或其平台相关默认尺寸。缺图标为 16 DIP 本地线条，继承当前前景色，alt 使用
DirectWrite 单行布局；显式尺寸及原 Image max 约束保留，不给它假自然尺寸或宽高比。
显示副本最多 4096 字符并滤除 NUL，原始 alt/UIA 名称不变。绘制留在原有背景/文字
指令顺序，复用内容盒/祖先/圆角裁剪和透明层，不创建图片纹理、控件或独立窗口。

替代文字不可选择，Image 仍为叶节点、无焦点/Invoke，UIA 名称仍只读 alt；没有新增
远端 load/error/click 事件。真实 Application 测试覆盖等待时占位、损坏后占位保留、
成功后移除、缺资源始终不回退；系统 UIA 客户端与真实 D2D 像素测试覆盖名称/无控件
后代、空 alt、窄盒裁剪、圆角、透明度和成功图片替换。合成绘制截图保存在
`linshi/otter-v193-placeholder.png`，已实际查看。

审查发现替代 alt TextItem 消失会让跨节点选区因“所有绘制文字项数量变化”而被误清空。
修复后的有效性检查仍要求两个端点可选择，只比较区间内可选择项的 ID、顺序和文字，
不可选择 alt 的出现/消失不再打断范围。专门回归在修复前实际失败、修复后通过，既有
16 项 cross_node 筛选回归也通过；跨节点复制仍按既有合同拒绝，没有顺手扩大剪贴板
能力。最初测试误用了只支持单节点的 selected_text，已先修正测试再复现实际失效。

新增 6 项回归，并扩展既有 3 项 Application 内联测试。最终串行全量 2658 passed /
44 ignored（96.50 秒），startup 7 passed（13.88 秒）；Clippy、fmt、3108 文件严格行数
与 diff 检查通过。新测量/绘制模块 56/75 有效行，PaintFrame 439 行；24 个本轮文本
UTF-8 无 BOM。日志 `linshi/otter-v193-full.stdout.log`、`linshi/otter-v193-startup.log`，
证据 `linshi/otter-v193-evidence.json`。没有提交/推送/发布；历史 UIA/点击不稳定性仍
未宣称根因修复，44 项交互/系统专用门未自动执行。

### v192 内联静态图片的应用生产路径

Request/Key 显式区分 Resource 与 Inline/Node，节点 ID 不能与资源 ID 字符串碰撞。
原资源租约构造及 HTTP 验证不变；先完成保持行为的类型重构，37 项资源与 3 项应用
图片测试通过后才增加内联来源。非空字符串 resourceId 始终优先，缺失/失败不回退 src。
否则只接受大小写敏感 data:image/ 前缀；blob、http、file 与相对路径不产生读取任务。

内联请求从原始有界快照提取可见 Image，保留完整 Binding、revision、节点及来源；
以原始快照 SHA-256 绑定消费身份，同 revision 替换源、可见性、类型乃至快照其他
内容也会使旧请求失效。已安装表每轮只散列一次当前快照，不按每图重解析完整 Scene。
原请求 Lease、同 Grant 转交、目标窗口 Lease 和 HWND 后复核都保留；不创建假的
ResourceLease，也没有虚构的一小时图片期限，内联期限只受真实授权截止约束。

应用在 Config 构造前分流，worker 在 Client 准备前分流；内联不读取 CA/profile，
不解析 DNS、不发 HTTP、不落盘。复用既有共享四槽、完成句柄、取消、Ready、窗口
安装和 Pixels 预算，不另建任务系统。原始 Scene 仍限 1 MiB，来源及序列化 data body
各限 1 MiB，队列/安装表仍最多 64 项；WIC 静态像素仍限 64 MiB。

解码按 [Fetch data URL](https://fetch.spec.whatwg.org/#data-urls) 去除 fragment、
百分号解码后处理 base64，保留字面 +；按
[Infra forgiving base64](https://infra.spec.whatwg.org/#forgiving-base64-decode)
处理空白、可省略 padding 与尾部未使用位。URL 解析及字节转换在 worker，循环有
取消检查，WIC 前后复核授权。成功的编码缓冲使用 Zeroizing；不声称所有错误路径
或原始 Scene/src 副本均安全擦除。

这是受限静态图片路径，不是浏览器 MIME 嗅探/完整 MIME 解析兼容：只接受既有
PNG/JPEG/BMP/单帧 GIF/TIFF/单图 ICO 白名单；特殊 MIME 写法、SVG、WebP、动画、
blob 对象生命周期、损坏图片可见占位和在线 Loom 业务验收仍未完成，能力报告不扩大。
新增 8 项回归覆盖数据 URL、身份/Grant/取消/容量，以及真实续签后的完整应用 poll：
PNG 自动安装上传，图片阶段零 Config/零资源 GET，缺资源不回退、损坏图片不重试。
授权续签仍使用真实回环 TLS，不能把它描述成整个测试零网络。

最终串行全量 2652 passed / 44 ignored（101.37 秒），startup 7 passed（14.43 秒）；
Clippy、fmt、3105 文件严格行数和 diff 检查通过。新生产模块 request/inline/data
分别为 85/120/128 有效行；Application 根仍 500 行。23 个本轮文本文件 UTF-8 无 BOM，
本轮目标目录原生进程残留为零。日志 `linshi/otter-v192-full.stdout.log`、
`linshi/otter-v192-startup.log`，哈希与范围记录见 `linshi/otter-v192-evidence.json`。
44 项显式交互/系统门未偷跑，历史 UIA/点击不稳定性未宣称根因修复，无提交推送发布。

### v191 图片 alt、装饰视图与叶节点语义

旧 DeclarativeSurface image 分支仅将字符串 props.alt 传给 img，不使用通用
accessibleProps/disabled，也不呈现 children。原生快照现按该合同读取 alt，不让
accessibility.label 覆盖图片名称；空/缺失/非字符串 alt 视为装饰，不从 src、资源 ID
或路径猜测名称。非空字符串保持原文，不额外 trim 或翻译。

Image 仍使用 UIA ImageControlType，不支持 Invoke/键盘焦点。纯装饰图片保留 raw
provider，但 IsControlElement/IsContentElement 为 false；带 alt 图片为 true。
此处按 2026-10-21 联网核实的 Microsoft Win32 文档 Remarks 中纯装饰图片规则，
不采用较旧 .NET 页面“装饰仍在 control view”的笼统描述：
[Image Control Type](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-supportimagecontroltype)。

Image 与 Icon 一样在呈现遍历中为叶节点；忽略 children，不允许这些后代进入布局、
UIA、焦点或 click 路由。同步移除图片自然尺寸测量对原始 children 非空的拒绝，否则
仅改遍历仍会让图片安装失败并关闭窗口。原始 Scene 节点数/深度/字节预算及输入合同
指纹仍统计原始 children，没有把显示过滤变成协议/授权校验豁免。

改动前两项模型回归均实际失败：图片名称错误取到 generic label，图片后代被暴露。
新增系统 UIA 客户端用真实 HWND/像素验证中文 alt、装饰过滤、无 Invoke/焦点、
带后代图片仍正常呈现、替换后旧 provider 失效；不等同屏幕阅读器人工验收或错误图
标/alt 替代绘制。损坏图片的可见占位、来源/格式剩余差异仍未完成。

新增 5 项回归，系统 UIA 2 / 模型 2 / click 路由 1 均通过。最终串行全量
2644 passed / 44 ignored（99.04 秒），startup 7 passed（16.09 秒）；Clippy、fmt、
3099 文件严格行数和 diff 检查通过。新 UIA 测试文件 127、语义快照 150 有效行。
日志 `linshi/otter-v191-full.stdout.log`，源码/日志哈希与编码复核见
`linshi/otter-v191-evidence.json`。历史 UIA/点击不稳定性仍未宣称根因修复。

### v190 应用自动请求与 Ready 投递

完整 poll_remote_live 在挂载/激活/事件结算及过时资源取消、目录和远程观看回收后，
先消费 Ready，再启动有界请求。目录/观看也释放共享容量，不能在它们回收前安排
图片，否则可能先遇 Busy、随后消耗最后完成句柄却未启动图片。Config 生产取当前
环境快照，CA 文件/DNS/获取/解码仍在后台，不在 UI
线程读取网络或信任文件。测试仅替换 Config provider，不替换任务或安装实现。

每个已确认 active 的窗口按完整 Binding + 场景 revision 保存最多 64 个可见 Image
资源 ID，去重且不从 src/路径回退。未激活不预取；启动成功即移出候选，同身份失败
不自动重试。容量不足保留队首，终态配置/授权/启动失败移除并保存有界状态文案；
换绑/换 revision 重建队列。Windows 每轮轮转起点，每个窗口最多启动一个任务；
无效候选在本轮有界跳过，避免没有任务句柄却永远堵住后续候选。
当前身份队列为空时直接返回，不在每次普通消息结算中复制/解析原始快照；窗口
撤权仍由主循环 refresh 与安装校验负责，没有为减少热路径工作而放宽请求准入。

Ready 只投递给 alive 且完整 Binding/revision 匹配的窗口，不匹配直接释放；匹配仍
经过 v189 同 Grant 转交和 HWND 后授权检查。所有任务继续使用既有共享容量、完成
句柄、到期截止与退出取消/回收；退出泵不调用新增自动调度。

新增三项应用回归和两项队列回归。成功与损坏响应测试走真实授权续签、loopback TLS
GET 和完整应用 poll；成功链检查两张图实际安装并上传，损坏响应后再 poll 五次
没有新连接。关闭晚到结果测试走生产资源结算子路径，不包含 inactive HTTP。
队列测试检查 mounted 不请求、active 请求、换场景重新入队、关闭停止、Busy 时
窗口起点轮转和终态失败不重复。实际挂载输入与 activation ACK 仍为合成夹具。

测试等待真实任务完成句柄后继续 poll，没有额外用户输入；未直接运行整个
next_live_message/MsgWait 等待链。外部占满四槽后释放再显式 poll，证明候选未丢，
不证明外部 owner 释放容量能自动唤醒 Application。两窗口顺序检查不是长期公平性
压力测试；不替代在线 Loom、inline/blob/共享内存/动画等仍缺的图片能力验收。
另有真实目录 GET 占据最后一槽的回归：等待目录完成但不手工回收，完整 poll 必须
先释放目录，再启动图片并提供新的完成句柄，直接保护本轮结算顺序。

最终源码串行全量 2639 passed / 44 ignored（102.08 秒），随后 startup 7 passed
（17.42 秒）。完整日志 `linshi/otter-v190-final-full.stdout.log`；初版全量日志保留
为 `otter-v190-full.stdout.log`，不能用它替代最后调度顺序/空队列快路径修改的验证。
授权窗口 454、队列模块 105、窗口调度 55、应用接线 48 有效行；Application 根仍
为 500 行且本轮未增长。证据与源码哈希见 `linshi/otter-v190-evidence.json`。

### v189 Ready 授权转交、窗口安装与到期撤图

Ready::transfer 先验证原请求的取消/租约/完整 Binding/revision，再要求目标窗口
Lease 来自同一个 Grant（Session Arc 身份，而不是相同凭据值），并重新验证目标
租约。消费后释放原任务槽及原请求 Lease；窗口 Lease 接管已安装图片的授权，
Grant 撤销仍生效，不承诺已消费请求的取消句柄继续拥有安装后的图片。

surface_window_image_install 由 AuthorizedWindow 持有最多 64 个资源 ID 的 Bitmap；
只从当前原始快照重建节点 resourceId 映射，不接受 src/路径回退。安装前验证当前
身份，HWND 提交后再次 refresh；提交失败关闭 owner，不留下半提交窗口。
图片最早期限进入现有 Windows deadline；到期/换绑撤去失效图片，窗口本身授权
仍有效时保留其他节点。新场景提交、关闭和撤权清空安装表，绝不按相同 ID 继承。
所有 RefCell 借用在 HWND 调用前结束。

同场景图片刷新沿原 Host/Presentation 链执行，保留滚动和文字未变的选区。
真实 HWND 测试使用合成 Ready 检查 bitmap 上传、图片先到期后的撤图和 CPU 像素
释放；真实 TLS runtime 测试验证同 Grant 转交、四个任务槽释放及异 Grant 拒绝。
这些不是在线 Loom 或完整自动下载验收。既有 sync_edits 会取消 spinner/slider
进行中的交互；本轮文字选区测试不能代表全部控件交互连续性已经验收。

新增 4 项回归，资源过滤 35 passed，选区刷新 1 passed；独立串行全量
2634 passed / 44 ignored（93.78 秒），`linshi/otter-v189-full.stdout.log`。
Clippy、fmt、3092 文件严格行数检查通过；授权窗口 449、安装模块 134、Host 309、
Presentation 355 有效行。全绿不替代历史 UIA/点击不稳定性的根因调查。

### v188 未就绪图片的呈现投影与应用挂载

Scene::with_image_availability 仅设置本地图片呈现状态，不改 props、资源 ID、src、
节点总数或原始资源快照。协议输入不能设置该状态；默认 parse 仍保留严格可见图片
测量规则。显式 props.visible=false 不会被 availability=true 覆盖。
未就绪图片通过统一 visible 门从布局、命中和无障碍树中略去，不占用 gap 或尺寸；
其他节点照常呈现。投影在已通过深度/总数限制的原树上执行，不复制整树或像素。

AuthorizedWindow 初次创建、单事件和 batch 提交均使用全 pending 投影，原始
ResourceSnapshot 与场景版本仍按原成功提交点保存，身份验证与撤权检查不变。
这解除“必须有像素才能建窗、必须有窗口才能准备资源请求”的初始化环路；混合场景
和 root Image 均可先建立 owner，随后正常激活并取得当前 Binding 的资源租约。
普通低层 SurfaceWindow::new 不自动投影，缺图片尺寸仍拒绝；不会静默扩大所有调用方。

这是 pending 阶段，不是完整图片能力：当前自动 GET、Ready 安装和到期撤除仍未接通，
因此应用中的图片仍等待后续链路；不把暂时略去所有未就绪图片当成迁移最终状态。
旧 DOM 无来源不生成 img 的合同在前述基线中已确认，本轮只对齐这一阶段。
专项：unresolved 过滤 5 passed、资源窗口 3 passed、带图片 batch 提交 1 passed
（过滤集合有重叠，不相加为独立测试总数）。新增 4 项测试；既有初始/事件/batch
资源用例升级为含实际 Image 的候选。Clippy 与 3091 文件严格行数检查通过。
独立串行全量 2630 passed / 44 ignored（91.13 秒），见 `linshi/otter-v188-full.stdout.log`；
本次通过不替代历史 UIA/点击不稳定性的根因证据。

### v187 显式图片表进入真实 HWND

SurfaceWindow::with_images/replace_with_images 将本场景图片表传给同一个 Host，
Host 再进入既有 Presentation/NativeMeasure/PaintFrame，不新增绘制层或线程。
旧 new/bind/replace API 仍传空图片表，不能按相同 ID 偷用旧像素；缺图候选继续失败。
窗口 replace 的既有身份撤销、UIA 更新、键盘状态协调、忙/关闭/重入检查没有放松。
候选失败时底层旧呈现保留，但窗口旧无障碍描述已撤销；应用 owner 仍须依既有规则
关闭提交失败的窗口，不能把底层旧帧保留解释为整个 HWND 状态事务回滚。

真实隐藏 HWND 测试检查 WM_PAINT 实际建立 bitmap 上传、resize/hit 同步释放旧上传、
重绘重新上传、替换/关闭释放缓存，以及外部 Rc 释放后 CPU owner 回收。还检查旧 API
拒绝缺图、忙/关闭拒绝候选不留像素。不等同于屏幕截图像素对照或在线 Loom 验收。
所有入口只消费已准备像素；应用必须验证当前来源/租约/revision，并在可重入调用后
再次复核。自动 GET、Ready 到窗口授权安装、到期图片撤除及应用公平调度仍未接通。

为遵守根文件 500 行门，将既有 replace 逻辑原样提到 surface_window_scene；
纯提取后既有窗口专项 70 passed（4.45 秒），再添加显式图片参数与新回归。
最终窗口专项 73 passed（4.98 秒）；独立串行全量 2626 passed / 44 ignored
（91.26 秒），`linshi/otter-v187-full.stdout.log`。Clippy、fmt、3091 文件严格行数
检查通过；窗口根 486、场景提交模块 42、Host 293、图片窗口测试 119 有效行。
此轮全绿不关闭 v185 尚未根因定位的 UIA/点击不稳定性。

下一生产接线的真实阻塞已核对：Windows::admit → AuthorizedWindow::new 仍调用
普通 SurfaceWindow::new，visible Image 缺像素会在保存窗口资源 owner 之前建窗失败；
event/batch delivery 也仍调用普通 replace，缺图即关闭。旧 DOM 在来源未就绪时仅
不生成 img，不拒绝整个场景，此差异必须补齐，不能把底层显式入口当应用可用。
Application 尚未调用 start_surface_resource/deliver_surface_resource；现有轮询只有
到期/取消/完成回收。mounted → active 会推进完整 Binding，retains_resource 与
Ready 验证都会拒绝旧 Binding，不能为穿过初始化环路而放宽这些身份检查。

### v185 当前窗口资源快照与消费身份

原先 attach admission 仅交付 Scene/revision，事件收敛也丢弃 raw 中的资源描述；
现在 ResourceSnapshot 从同一份 raw/header/binding 保存有界完整快照，初始挂载、
单事件和 batch 都将其送到 AuthorizedWindow。模型不访问网络，Debug 仅输出
revision/字节数，不输出 raw、租约或 binding。它验证大小与身份，不提前认可所有租约。

窗口保存当前场景资源快照；事件/batch 在 replace 前校验资源身份，只有 replace
成功才一起更新 scene revision 和资源快照。失败不更新候选；关闭、撤权、观察到
HWND 已销毁都释放资源描述，不影响附件恢复证据和生命周期 owner。
纯模型检查 origin/device/node/instance/attachment/generation 不变，场景 revision
精确相等，只允许 lifecycle revision 非倒退；资源 mount 采用当前 Binding，不能
永久沿用 mounted 阶段的值。RefCell 借用不跨任何 HWND 调用。

AuthorizedWindow::image_resource 从当前快照取得请求租约；resource_image 经当前
窗口授权与 Ready 原租约再次核验后共享像素。它尚未执行 HWND 图片安装；安装方
在可重入窗口调用之后仍必须再核对授权和版本。mounted 阶段预取若迟至 active 后
返回，现有完整 Binding 比较会拒绝旧 Ready；调度方需丢弃并按新身份重新准备，
不能把这个阶段的预取当成 guaranteed ready，也不能绕过原请求身份检查。

新增本地模型、真实 HWND 和流收敛回归；batch 既有回归追加资源快照断言。
窗口夹具使用合成协议，未访问远端；本轮不宣称自动 GET 调度、像素安装或在线 Loom
图片端到端已完成。额外 Linux cargo check 尝试被 ring 缺少 x86_64-linux-gnu-gcc
阻挡，不算 Linux 通过；未为此安装或更换工具链。

验证异常保留：首次全量为 2620 passed / 1 failed / 44 ignored，并发资源发布
`concurrent_publication_reuses_one_exact_resource_without_temporary_leaks` 收到
`0x800700B7`；随后独立运行 30 次通过，根因未确认，未修改发布逻辑或宣称修复。
第二次全量同为 2620/1/44，编辑依赖点击用例得到 `after click` 而非 `at click`；
该轮与 startup 窗口测试重叠，存在焦点干扰可能，但尚无证据将其认定为根因。
两次原始日志分别保留在 `linshi/otter-v185-full.stdout.log` 与
`linshi/otter-v185-full-rerun.stdout.log`，不以之后通过覆盖失败证据。
第三轮无 startup 并行的串行全量仍为 2620/1/44（96.94 秒）：原生 Select UIA
枚举得到 0 项，原生列表实际为 2 项，前两轮失败用例本轮通过。日志
`linshi/otter-v185-full-isolated.stdout.log` 保留完整诊断。停止重复全量碰绿；
以上不稳定性均未确认根因或修复，全量验收未通过。startup 7 passed（17.50 秒），
Clippy 与 fmt 通过；当前资源快照专项与全量状态必须分开报告。

### v184 自然尺寸反馈与生产呈现

NativeMeasure::with_images 接收本场景按节点 ID 索引的已解码自然尺寸，图片比例按
content box 计算，再加 border/padding。重排使用最终交叉轴盒子反馈 auto 主轴，
文字和图片分别判定稳定，图片比较双轴；每次 prepare 清旧反馈，仍限 8 轮。
默认 maxWidth/maxHeight=100% 位于统一取值层；普通流图片 auto 主轴 minimum
保留受 max/显式尺寸约束的内容基准，不把两图强制压扁。极端比例的 auto 基础尺寸
以既有布局引擎 32768 上限为界，最终尺寸仍由普通布局 max/min 约束。
absolute 图片反馈与 Tree/position 一致使用父 padding box，而不是 content box。

Presentation::with_images/replace_with_images 原子提交同一场景的图片表、测量和绘制。
普通 replace 不继承旧表：同节点 ID 的新场景缺资源仍失败并保留整个旧帧。
reflow、wheel、pan、滚动条/键盘滚动、焦点显露、选区显露/自动滚动都传当前图片表，
不会只重建文字而丢图。图片表受 MAX_NODES 数量门约束；来源、revision、租约到期
及撤权仍须由未来应用安装 owner 复核，不能把此低层 API 当资源授权接口。

Chromium 155.0.8059.80 补测实际旧组件，并在已生成 DOM 上施加对应布局样式，
共 13 项：row 默认/单轴/max-height、auto 高父容器、两个方向 center、padding、
双图、absolute 四种百分比组合。它验证 CSS 布局语义，不替代 props 白名单测试。
例如双图各 300x150（超出 180 高父容器而不压成 90）；带 padding:20 的定位父容器中
width:50% 图片为 150x75，图片再加 padding:10% 为 150x105。
脚本与结果：`linshi/otter-image-layout-v184.js`、`linshi/otter-image-layout-v184.log`；
浏览器会话与临时服务均已关闭。原 v183 四项像素夹具已移除固定 Measure，改走
真实 NativeMeasure/Presentation；新增布局与 owner 生命周期测试。

这不是全部布局/来源验收：Stack、absolute 双侧锚点、复杂 min/max 与不确定百分比
包含块仍需独立浏览器对照；应用自动请求/安装、公平调度、data/blob/shared_memory、
Tile 及逐编码格式一致性未完成。跨滚动投影仍可能重新上传，不宣称滚动零上传。

验证：新增 7 项布局/生命周期测试，图片专项合计 11 passed；可见串行全量
2616 passed/44 ignored（87.15 秒），`linshi/otter-v184-full.stdout.log`。
Clippy all-targets -D warnings、fmt、3085 文件严格行数、diff 与 22 文件 UTF-8
无 BOM 检查通过；未执行在线 Loom、图片 HWND 安装或 ignored 系统输入验收。
startup 7 passed（16.49 秒）；源码、浏览器基线和日志哈希见 `linshi/otter-v184-evidence.json`。

### v183 有序图片绘制后端

surface_image_paint 的 UI 线程 Bitmap 共享已解码 Pixels，不复制像素；首次绘制才上传，
按精确 render target 而非 factory 缓存。单图 64 MiB，跨窗口上传像素字节预算 128 MiB；
该预算不是实际驱动/GPU 总显存测量。来源授权仍归 Ready/current mount，Bitmap 不是授权入口。
PaintFrame 图片命令遵循既有背景、边框、祖先透明层、裁剪与兄弟顺序；内容扣 border/padding，
固定 contain 居中。图片自身圆角按 content edge，容器子树仍按 padding edge；依据
[CSS Backgrounds 的圆角裁剪规则](https://www.w3.org/TR/css-backgrounds-3/#corner-clipping)。
上传失败发生在 axis clip 入栈之前；外层 layer 守卫及画刷恢复路径不变。
宿主 target 重置/绘制错误和 PaintFrame Drop 均清图片缓存，避免外部 Rc 像素 owner
把旧 target 留在已移除或不可见节点。此阶段缓存只保证同一帧反复绘制复用；跨滚动投影
复用和自动安装 owner 仍待接入，不宣称已优化到滚动零上传。

真实 WIC/D2D 后端用例覆盖 contain 留白与居中、透明层、兄弟遮挡、圆角、padding
内容圆角、同 target 复用/同 factory 不同 target 重建、显式清理和帧退场释放；还核对
画刷恢复及后续不透明标记不受遗留 clip/layer 影响。测试采用独立固定测量夹具，
不绕过生产 NativeMeasure 的 Image 拒绝门。自动请求、Ready 安装、自然尺寸/比例反馈、
data/blob/shared_memory/Tile 及逐格式浏览器一致性仍未完成，不扩大 image 能力声明。

验证：新增绘制专项 4 passed；可见串行全量 2609 passed/44 ignored（86.98 秒），
日志 `linshi/otter-v183-full.stdout.log`。Clippy all-targets -D warnings、fmt、3083 文件
严格行数与 diff 检查通过；12 个相关文件 UTF-8 无 BOM。未执行在线 Loom 或真实图片
节点端到端验收，也未运行 ignored 系统输入测试。
startup 7 passed（16.28 秒）；源码与日志哈希记录在 `linshi/otter-v183-evidence.json`。

### v182 后台静态解码和主动到期

共享 WIC Image 实现归 image_decode.rs，desktop/image.rs 重导出同一类型以保留旧
调用点。旧打开/内联/Relay JPEG 的 257 MiB 编码与 256 MiB 像素门保持不变；
Surface 采用独立 16 MiB 输入、64 MiB 像素输出上限，仍受全局 Pixels 预算约束。
WIC 在 worker 的 STA apartment 内初始化/释放；解码结果不携带 COM 对象。
Ready 现在保留 Image 和原资源描述，不再保留编码缓冲；编码字节返回前清零释放。
Ready::image 继续以原授权和当前 mount 复核，解码前后另检查撤权和资源到期。

RGBA 来自受摘要约束的声明尺寸，单边最大 16384，大小必须精确；逐行检查取消，
按 +127 四舍五入换为预乘 BGRA，透明像素的 RGB 清零。编码图使用 WIC 自然尺寸，
不把描述符 width/height 当权威；分配前检查尺寸、局部上限和共享实际 capacity 预算。
WIC 调用本身不能强制中断，取消在调用前后检查，Drop 兜底仍等待当前调用结束。

当前静态 MIME 路径为 PNG/JPEG/BMP/GIF/TIFF/ICO，要求 WIC 容器匹配且仅一帧；
PNG 另拒绝 acTL。此列表不是逐格式完整验收：当前真实像素用例覆盖 PNG 和 RGBA。
SVG/WebP、动画、多帧、多尺寸 ICO、MIME 嗅探兼容与参数形式仍未完成，必须继续
补齐旧合同，不能把当前拒绝作为最终功能删减，也没有扩大 image 能力声明。
JPEG EXIF 方向、色彩配置以及逐静态格式的旧浏览器像素一致性仍须补证；不能拿
桌面旧 Image 解码路径的既有行为代替 Surface 浏览器兼容验收。

Sessions 将资源/授权最早到期时间并入现有 wait deadline；到期取消在途 GET，
丢弃未消费像素并释放槽。已取消线程不再贡献过去的 deadline，避免退出/取消忙循环。
四槽背压仍保留，自动调度接入时仍须实现及时安装与交互任务公平性。

验证：解码专项 3 passed，资源专项 11 passed；新增 4 项测试。真实 WIC PNG
验证自然尺寸与预乘像素、共享像素 owner 释放；raw 验证透明/半透明舍入、尺寸与
逐行取消。修改 IHDR/CRC 的夹具证明 约 95.4 MiB 解码尺寸在像素 CopyPixels 前被
64 MiB 门拒绝；APNG 明确拒绝。真实 TLS→worker WIC→像素交付和结果释放通过。
集合成功结果用例追加主动到期、deadline 消失与槽位全释放；取消线程不贡献过期
deadline。最终可见串行全量 2605 passed/44 ignored（88.83 秒），日志
`linshi/otter-v182-full.stdout.log`。Clippy、fmt、3081 文件严格行数、diff 检查通过，
20 个相关文件 UTF-8 无 BOM；未跑在线 Loom、真实图片节点显示或 ignored 系统输入。
startup 7 passed（18.57 秒），收尾自有 native 进程为零；源码/日志哈希见
`linshi/otter-v182-evidence.json`。

### v181 后台任务与原租约消费门

surface_resource_runtime::Job 使用既有共享四槽 Slot、配置快照与独立授权 Lease；
CA/DNS/HTTPS 留在 worker。完成后的 Ready 仍持有槽和原 Lease，因此最多四份
16 MiB 传输结果，不可因线程已结束而无限积累。Job 正常 join 将 owner 移交 Ready；
未完成 Drop 只作 stop/join 兜底，正常 UI 路径使用 wait handle/try_join 非阻塞回收。
Ready::bytes 内部使用原租约，复核会话授权、当前 mount/资源租约/期限，不再接受
调用者提供的新 Cancel。缓存撤权在任务完成且 Job 已释放之后仍可使结果失效。

Sessions 已接资源准入、重复请求拒绝、容量、正常等待、退出等待、poll、拒收保留、
关闭与 finished/drain。资源 GET 无附件事务副作用，取消后可丢弃字节，不新增
unknown 附件恢复记录。应用结算点在场景结算前后比较活窗口的完整 Binding 与
snapshot revision，关闭/替换会取消旧任务与结果；退出先取消图片，避免占用停用槽。

仍未接图片节点的自动请求、WIC 解码、布局、PaintFrame 或缓存。Ready 是有界的
待消费结果而不是图片缓存。下一步解码必须留在后台，不允许 UI 收到字节后同步解码。

容量后果：四份未消费 Ready 会占满共享控制槽。自动图片调度接入前，必须同时
实现及时解码消费/丢弃以及交互任务不被长期阻塞的验收；不能把 Ready 当常驻缓存。
v181 当时尚未把资源到期加入应用 deadline/淘汰调度；v182 已补主动唤醒与回收。

验证：新增 7 项生命周期/集合测试；资源相关筛选共 44 passed。真实回环 TLS 覆盖
读取 body 时取消、running Job Drop 回收、完成后 join 前撤权、Job 释放后 Ready
仍受原 Grant 撤权、场景版本拒收、四份结果容量，以及集合拒收后保留、定向取消、
成功交付后释放全部槽。取消租约夹具另覆盖重复准入、正常/退出等待、close/drain。
最终可见串行全量 2601 passed/44 ignored（85.51 秒），日志
`linshi/otter-v181-full.stdout.log`。Clippy all-targets/-D warnings、fmt、3077 文件
严格行数和 diff 检查通过；20 个本轮相关文件 UTF-8 无 BOM。未跑在线 Loom 或
ignored 系统输入探针，不把运行时测试当图片渲染验收。
startup 7 passed（19.70 秒），收尾自有 native 进程为零；源码/日志哈希保存在
`linshi/otter-v181-evidence.json`。

### v180 资源获取边界

MountedSurface::image_resource 从最多 1 MiB 的快照中选择资源，最多 64 leases；
拒绝同 resourceId 的歧义租约。ResourceLease 私有字段绑定完整 Binding、snapshot
revision 和 lease 描述，校验 sha256 ID、固定路径、租约头、期限、大小与 RGBA 尺寸。
Control::fetch_image_resource 复用固定来源 TLS、Device/nonce、20 秒绝对截止时间、
取消与 401 认证终态；仅此路径可读 16 MiB，普通 HTTP 保留 8 MiB 上限。
返回前校验实际长度与原始字节摘要，缓冲由 Zeroizing 拥有。消费 bytes 必须传当前
mount、时间及该任务 owner 的 Cancel，重新检查身份/租约/到期/取消。

v180 时这是仅供后台 owner 调用的阻塞 API，尚未接入后台 job、缓存或 HWND 提交。
调用者必须使用当前 owner 的取消状态与 mount，不能保留旧 mount 冒充当前状态；
v181 已加入上述任务/消费门；生产图片 HWND 安装仍未完成。v180 本身没有新增线程或缓存。
shared_memory/stream 明确拒绝，不降级读取文件；它们仍是待实现边界而非已迁移。

验证：资源专项 5 passed；真实回环 TLS 覆盖 Device/nonce/lease 头、摘要损坏、
短/超长响应、302/401、路径/租约拒绝、快照 revision/lease 替换、到期、外源、
连接前及消费前取消，另实际传输 9 MiB RGBA 并校验原始字节。首次专项中一小时
边界夹具只多留 1 ms 导致时钟竞争，改用明确超限时间后通过，未放宽生产校验。
最终可见串行全量 2594 passed/44 ignored（84.54 秒），日志
`linshi/otter-v180-full.stdout.log`。Clippy all-targets/-D warnings、fmt、3073 文件
严格行数及 diff 检查通过。未运行在线 Loom 或系统双击探针，不代表这些边界验收。
资源专属传输中取消/到期、chunked 超限、生产 job 提交竞态仍待后续补证。
startup 7 passed（16.57 秒）；本轮 11 个相关文件 UTF-8 无 BOM。

## 来源与授权合同

`DeclarativeSurface.tsx` 的 imageSource 按以下顺序处理：

1. 非空字符串 resourceId 优先，调用宿主提供的 resolveResource；未取得资源不回退 src。
2. 没有 resourceId 时，仅接受以 `data:image/` 或 `blob:` 开头的 src，匹配大小写敏感。
3. 不直接接受 http/https、file、本地路径或相对路径。来源缺失时没有 img 元素。
4. blob URL 是浏览器对象标识，不是本地路径或原生下载授权；原生必须接宿主拥有的
   字节/资源句柄，不能把 blob 字符串改写为文件名。

正常桌面资源链是 unitSurfaceController → loomOtter surface_resource action →
旧 Host `resource_transfer.rs` → surface/resource delivery → surfaceResourceStore。
应复用合同，不另建可任意下载的 image 网络入口：

- ID 是 sha256 摘要形式，声明字节数 1..16 MiB，流式读取有上限，精确大小和摘要
  必须一致。缓存只校验 ID 形状和 data URL，不替代传输字节验证。
- lease 未过期且不超过一小时，取回后再次验证到期；网络使用既有 Loom manifest、
  device session 鉴权与 `X-Loom-Surface-Lease`。
- loom_resource 的 path 必须精确为 `/v1/surfaces/resources/{digest}`，无 query/
  fragment；这是受限 HTTP 资源路径，不是磁盘路径。
- shared_memory 只接受 `application/x-neuro-rgba8`，受限 OS 映射名称和精确字节数，
  width×height×4 必须匹配。摘要验证的是传输原始字节，不能拿转换后的 PNG 重新
  与原 RGBA resourceId 比较。stream transport 在旧获取函数中也拒绝。
- Tile/Wall 是另一条带 deviceId/view/resourceId 的受限 host 路径；不接受内联 src，
  不能把它与桌面 data/blob 合同混为一谈。

权威源码入口：`src/features/surfaces/components/DeclarativeSurface.tsx`、
`unitSurfaceController.ts`、`src-tauri/src/loom_otter/resource_transfer.rs`、
`src/features/surfaces/state/surfaceResourceStore.ts`、
`src/features/tile/services/tileSurfaceImages.ts`。

## 实际浏览器基线

隔离 Vite 直接导入当前 DeclarativeSurface、协议与实际样式。测试图片是本地 canvas
生成的 80×40 红/绿 PNG；测试 blob 也在该页面内创建，离页撤销。Edge 155.0.4283.45，
14 个场景断言通过。对 example.invalid 的路由只用于拦截检测，实际请求数为零，
没有联网取图、Loom 获取、系统文件读取或用户图片访问。

基线根为宽 300、高 180 的 column；以下是 img 元素盒，而非内部像素的 contain 框：

| 配置 | 实测元素盒 |
| --- | --- |
| 不指定图片尺寸 | 300×150，受 column 默认 stretch 影响，并非自然 80×40 |
| layout.width=160 | 160×80 |
| layout.height=80 | 300×80 |
| width=120、height=120 | 120×120 |
| 根宽=40 | 40×20 |
| 根高=20 | 300×20 |

固定 CSS `object-fit: contain`、max-width/max-height=100%，没有 props.objectFit 或
style.objectFit 接线；传 cover/fill 不改变 contain。120×120 真实截图显示中间
120×60 红绿图像及上下空区，已人工工具查看。不能把元素盒直接当作拉伸像素目标。

来源/事件断言同时证明：resourceId 缺失不回退有效 data src；有效资源优先于
被拒绝的 remote src；直接 remote/file 不产生 img；blob 正常；visible=false
不产生 img。损坏 data PNG 仍有 img/alt，naturalWidth/Height 为零；这次盒为
300×21，但错误图标/字体相关尺寸不作为跨环境常量。

图片 draggable=false。图片本身的 click/load/error 声明没有触发 Surface event；
点击只产生声明动作祖先 root 的 click。alt 为 img 原生属性，图片分支未使用一般
accessibleProps；不能擅自把图片做成独立按钮或额外派发远端 load/error。

证据目录：`C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-image-baseline-v179`。
保留 fixture、server、probe、report、verify、evidence.json 与 contain.png；evidence
含实际源码/夹具/报告 SHA256。末页 console error 为零，浏览器和服务器已关闭。
另执行 DeclarativeSurface 与 surfaceResourceStore 两个现有测试文件：12 passed。
不是完整 Tauri、真实 lease 传输、完整 MIME/动画或 Tile 像素验收。

## 后续实现顺序与所有权

先完成经绑定的资源字节获取，再连接解码、测量和绘制，不在 NativeMeasure 里直接
访问网络/文件或同步解码。MountedSurface 现在可解析 resourceLeases 并生成受限请求；
已复用安全 Loom HTTP transport，不能绕开其 TLS/设备身份。后台 job 已接应用集合，
后台静态解码已接入，但图片节点自动准入、像素安装 owner 尚未接入。

1. 为当前 mount 的 instance/attachment/revision/来源身份绑定资源请求与结果；
   lease、路径、大小、摘要、过期和取消均在后台验证，替换/关闭后的结果不得回挂。
2. 复用 WIC 字节解码和共享 Pixels 预算，COM decoder 生命周期留在后台 owner；
   解码前检查尺寸与总预算。资源可共享，但失败重试、并发和保留量必须有界。
3. 让已验证图片快照提供自然尺寸；测量实现默认 stretch、单轴尺寸、contain 内容
   框与 max 约束。PNG 固定尺寸演示不能代表完整图片功能。
4. 接入原 PaintFrame 的有序 Text/背景同级图片命令、裁剪/透明层和 target 重建，
   不建立额外窗口或独立覆盖绘制层；D2D 上传与释放归 target owner。
5. 复核 src 更新、资源过期/替换、祖先 click、alt/无障碍和错误图像，再扩能力报告。

仍须补证：SVG、GIF/动画、其他 MIME、非 base64 data URL、CSP 影响、blob 生产替代、
资源共享/淘汰、Tile/Wall、DPI/滚动/透明裁剪及设备丢失。旧 Scene 的 1 MiB 原生
解析预算与 16 MiB 资源传输预算属于不同边界，不得为了内联图片静默放大场景上限。
这些未完成项必须继续实现或获得用户明确批准，不能直接删去当作原生迁移完成。
