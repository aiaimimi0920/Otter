# Surface WebP：解码器审查与隔离接入条件

2026-10-22，内部版本 v0.2.33.201。静态 WebP 已接通 Resource/Inline → worker →
受限 child → Ready → HWND；下述前置审查解释为何不能在主进程直接调用解码器。
这不是完整 WebP 元数据/动画支持或 N01–N12 迁移完成。

## v201 共用图片 child 宿主

Job 自检、管道读包、输入清理、renderer 调用和回包已归 surface_image_worker_host；
wire 层统一输入/像素数值检查，父侧还保留 RIFF 预检尺寸的一致性验证。
进程 owner 作为根模块 surface_image_worker_process 编译一次，原 CAPACITY 和
启动/回收逻辑不变；当前仍只有 WebP 固定入口和 v1 协议，不接受 SVG 或任意 codec。
WebP worker 保留原解码/ICC/EXIF/预乘，host 不重复预乘。SVG 下一接线见 SURFACE_SVG.md。
重构前后 WebP 专项均 18 passed，实际 Application 资源/内联/P3 安装仍通过。
最终全量 2698 passed / 48 ignored（120.85 秒），startup 7 passed（19.09 秒），
Clippy/fmt/3128 文件严格行数/diff 通过；原 44 项专项未运行，四个私有 child 由父测试调用。

## v200 真实 mft2 LUT 的浏览器兼容

自建仅含 A2B0(mft2)/wtpt、没有 matrix/TRC 标签的 profile，分别测试 identity 输出曲线、
五点非线性输出曲线、非可分离三维网格，RGB/RGBA 共六个 WebP。LittleCMS 独立接受并
转换这些 profile；它与原 moxcms 的亮度接近，而 Chromium 155 明显不同。因此本次目标
明确是普通 Chromium Surface 兼容，不能把浏览器行为称为唯一 ICC 标准正确值。

实测同一 opaque 像素原 moxcms 为 [218,188,124]，Chromium 为 [160,137,89]。
仅在目标矩阵抵消 PCS 倍率的实验仍不能通过非线性样本：红通道浏览器 0、实验为 10。
moxcms 0.8.1 普通路径先预合成 33³ 网格及目标 gamma，再运行插值，不能保证相同求值顺序。
因此没有采用全局亮度修正、篡改 ICC 或放宽容差。

新增 worker 私有 surface_webp_lut，仅接管显式 Perceptual 的 A2B0、mft2、RGB→XYZ。
复用受限 child 内 moxcms 解析出的表，复核通道/网格/精确表长；按输入曲线、三线性
CLUT、输出曲线、固定 sRGB 逆矩阵、gamma、最终 clamp/量化顺序直接求值。
表值按当前浏览器归一化尺度使用；RGB mft2 matrix 不参与。实现无逐像素堆分配和
第二张全图，只写 RGB，alpha 保持；既有 Job/父侧截止时间及取消负责回收，不声明库内抢占。
Display-P3 matrix 与其他 LUT 类型仍走原通用分批路径，依赖版本/锁文件不变。

上游行为佐证：[skcms mft2 解析](https://skia.googlesource.com/skia/+/main/modules/skcms/skcms.cc)
与[三线性 CLUT 求值](https://raw.githubusercontent.com/google/skia/main/modules/skcms/src/Transform_inl.h)。
这些当前公开源码不冒充本机浏览器二进制的完全同一提交，最终基线是相同编码字节实测。
六个真实 child 案例检查 8×1 完整 PBGRA、通道容差 ≤1、alpha 精确；另有全 256 alpha、
坏表长/布局拒绝、三线性和曲线边界测试。样本与独立结果归 linshi/otter-lut-variants-results.json。

尚未覆盖 mft1、Lab PCS、mAB/mBA、多 intent、matrix/TRC 与 LUT 混合 profile 的完整浏览器
兼容，也没有验收最大图片的吞吐/峰值、监视器/HDR、其他格式或 N01–N12。
最终全量 2695 passed / 48 ignored（119.96 秒），ICC 专项 7 passed；Clippy all-targets
-D warnings、fmt、3126 文件严格行数和 diff 检查通过。原 44 项 ignored 专项未执行，
四个私有 child 入口由父测试调用；既有 UIA 间歇性不因本轮通过而宣称根治。
startup 7 passed（19.12 秒）；源码/样本/日志哈希及清理检查归 linshi/otter-v200-evidence.json。

## v199 嵌入 ICC → sRGB

实际 Chromium 155 Canvas 的 sRGB/Display-P3、opaque/alpha 四个编码样本确认差异：
P3 第一像素原始 RGB [179,127,51]，浏览器 sRGB 为 [188,124,27]；此前 worker 会
直接预乘原色。新增 surface_webp_color 在受限 child 内、预乘之前转换，使用固定
moxcms 0.8.1（BSD-3-Clause OR Apache-2.0），关闭默认 SIMD features，显式启用 lut。
这使用通用 matrix/LUT API，不以仅支持 matrix 的原地 API 冒充完整转换路径。

借用首个 ICCP payload，不复制完整 profile；遵守 VP8X ICC flag，checked slice 和
256 chunk 边界。profile 解析尺寸不超过原输入边界，库 TRC/CLUT 额度不等同实际
总内存：CLUT 配置在库内部分路径按网格点数检查。最终仍依赖既有 128 MiB Job、
最多两 child 和父侧截止时间/撤销；parser、transform 构造不具备库内抢占取消。
每次通用 transform 输入至多 4096 像素，输出栈区 16 KiB，不创建第二张完整图；
RGBA alpha 必须逐字节保持，否则拒绝，转换后才按原规则预乘/换为 BGRA。

实测 Chromium 对无 ICC、坏 signature、短 ICC、非 RGB profile 保持编码 RGB；
相同字节经 child 保持该行为。不能把所有 parser Err 都当坏 metadata：OutOfMemory、
可区分的 TRC 额度或不支持错误显式失败，已识别 RGB profile 的 transform 构造/执行错误也失败，
不能成功返回错误颜色。未对所有 malformed profile 声称逐行为浏览器兼容。
库将部分 CLUT 配置额度违规同样报告为 InvalidProfile，当前该类别仍按坏 metadata
忽略；没有宣称所有内部额度都可被调用方区分。现有合法 RGB CLUT 大表仍受 16 MiB
编码输入与 Job 限制，完整 LUT/额度行为需要独立夹具补证。

两个 P3 编码夹具覆盖 RGB/RGBA，与浏览器的预乘通道容差 ≤2、alpha 精确；另外有
四种 fallback、跨 4096 批次的 alpha 0/1/127/128/254/255，以及资源错误不降级回归。
实际 Application 的 P3 inline 安装/上传也进入验证。不宣称监视器 ICC 校准/HDR，
目标是当前普通 Surface sRGB 绘制链。真实 LUT profile 夹具、更多色域、全尺寸峰值
和 JPEG/PNG 等其他解码路径的色彩管理仍未验收，不能把启用 lut feature 当作已实测。

开发依赖门：全锁文件 Advisory 扫描 5 locks / 1790 packages，仍有 20 个漏洞 ID；
许可证检查 718 个 Rust package / 40 种表达式通过，双 SBOM 的 1229 个去重身份一致。
不是 clean exact-ref 正式发布证据。浏览器样本及差异归 linshi/otter-icc-browser.json，
临时转换实验归 linshi/otter-icc-audit，第三方源码没有复制进项目。
最终全量 2692 passed / 48 ignored（139.59 秒），startup 7 passed（17.40 秒），
Clippy/fmt/3124 文件严格行数/diff 通过；Application P3 安装/上传、alpha/批次和
错误分类五项新增回归均通过。OutOfMemory 分类测试不执行真实内存耗尽，不冒充
最大图片/复杂 ICC 的峰值实验。moxcms 缓存包 SHA-256 与 crates.io 元数据一致：
bb85c154ba489f01b25c0d36ae69a87e4a1c73a72631fc6c0eb6dde34a73e44b。

## v198 EXIF 方向基线纠正

先前“接下来应实现 WebP EXIF 旋转”的推断不成立，应以真实旧宿主行为为准。
Chromium 155 的 img → canvas 实测：3×2 六色无损 WebP，方向 1..8、TIFF 大小端、
裸 TIFF / Exif\0\0 前缀，共 32 个样本均保持 3×2 和同一像素顺序，不旋转/镜像。
Pillow 12.3.0 独立读取相同编码字节，32 个 tag 274 均与预期一致，排除标签写错。
浏览器输入/输出归 linshi/otter-v198-browser.json，独立读取归 otter-v198-exif-proof.json；
测试浏览器已关闭，没有使用用户 profile。

联网核对的 Chromium WEBPImageDecoder 源码 blob
881a918b4cbb2141a21f382a6239a873fc789152 未见 EXIF/orientation 路径，与实测相符：
[上游源码](https://chromium.googlesource.com/chromium/src/+/HEAD/third_party/blink/renderer/platform/image-decoders/webp/webp_image_decoder.cc)。
该外部源码是佐证，不冒充与本机浏览器二进制完全同一提交，也不据此宣称所有浏览器一致。

surface_webp_exif_tests 固定浏览器生成的无损基底并构造同一 32 案例，经真实解码
child 逐一检查原始尺寸及完整 PBGRA。生产不增加 TIFF 解析器、不旋转、不放宽 IPC；
JPEG/TIFF 的八方向 WIC 规则仍保留，不能把某格式的策略泛化到另一格式。
这是普通 Chromium Surface 的方向兼容证据，不是完整 EXIF/XMP 元数据支持或导出
能力。接下来应推进实际 ICC 色彩差异等未完成呈现边界，而非增加与基线相反的旋转。
验证：32 案例 child 专项通过，最终全量 2687 passed / 48 ignored（112.31 秒），
startup 7 passed（14.62 秒）；Clippy/fmt/3122 文件严格行数/diff 通过。完整证据及
源码/日志哈希见 linshi/otter-v198-evidence.json，不将本轮绿灯当完整迁移验收。

## v197 真实 child 故障注入

surface_webp_wire_tests 仅在 cfg(test) 下选择固定测试 child；它仍经生产 start 创建
Job、限制句柄并启动，父侧 run_with 复用真实发送/响应校验/退出/尾随数据检查，
不是在测试中另写解析器或直接构造成功 Image。生产 run 仍固定绑定 process::spawn，
没有给来源字节、环境或用户配置开放可执行文件/参数选择。

一个合法对照检查确切像素；八个故障模式分别为错误 magic、宽度、字节数、截断
header、截断像素、尾随输出、完整合法响应后非零退出、尚未响应即非零退出。每个
错误必须拒绝发布 Image；每轮保留独立 process handle，在父侧返回后零超时等待
必须为 signaled，直接证明该 child 已退出，不把父侧丢弃句柄当作已回收进程。
共享名额还补充了占满至 deadline 和等待中取消；取消分支用五秒 deadline，并要求
三秒内返回，避免把正常等到 deadline 误当取消生效。测试串行运行，保护共享预算。

这些夹具不证明 codec panic/access violation/OOM 的所有系统路径，也不测量所有
Job/pipe handle 的数量。实际 Job 分配失败注入、整组解码峰值和元数据仍需补证；
不把本次八种错误等同于全部故障覆盖。
最终 v197 全量 2686 passed / 48 ignored（115.28 秒），startup 7 passed（16.67 秒）；
Clippy/fmt/3121 文件严格行数/diff 通过。新增一个父测试和一个私有 ignored peer，
该 peer 由父测试实际启动九次；原 UIA 间歇性问题不因本轮通过而宣称根治。

历史下一步曾假设需要 WebP EXIF 旋转；已由上文 v198 的浏览器/独立元数据证据纠正，
不实施与旧宿主行为相反的 TIFF 解析及像素旋转。完整元数据/ICC/动画仍另行验收。

## v196 实现及边界

- 精确固定 image-webp 0.2.4；RIFF 总长、padding、最多 256 chunks、单载荷、内外
  尺寸一致在父/子两侧检查。输入最多 16 MiB，输出最多 64 MiB；拒绝动画标记及
  ANIM/ANMF，不将动画静默当首帧成功。内联仍受原 1 MiB 来源限制。
- 启动当前 EXE 的固定私有入口，先创建 suspended 进程、加入 Job，再恢复执行。
  只继承明确列出的管道/NUL 句柄，环境为空；入口先于 profile/settings/UI/network。
  每个 Job 的 process/job committed-memory 限制均为 128 MiB，active-process 为 1，
  kill-on-close；共享 256 MiB 名额最多允许两个 child。同时到达的其余请求可取消地
  等待名额，不把正常容量拥挤直接转换为永久图片失败。
- 父侧截止时间 15 秒，管道 pending 等待每至多 20 ms 检查撤销；取消 I/O 后等待
  完成才释放缓冲。退出路径终止并等待 child，成功也要求完整 IPC、尺寸/长度一致、
  exit 0 且无多余输出。没有主进程解码回退。系统进程清理可能等待内核，不能承诺
  包括清理在内的硬实时 15 秒上限。父侧 Pixels 及原来的任务/授权预算仍单独生效。
- 这是资源隔离，不是权限沙箱：child 保持同用户 token 和当前工作目录，没有
  AppContainer/restricted token，未从 OS 权限层禁止文件/网络访问。不能据此宣称
  总进程峰值低于 256 MiB 或所有恶意解码输入均无风险。
- lossless RGB/RGBA、有损 RGB/ALPH、错误 MIME、截断/重复/动画、容量等待、取消、
  停滞超时/回收均有测试；129 MiB VirtualAlloc 被真实 Job 拒绝，探针 Job 实测峰值
  4,022,272 bytes（只是探针，不是所有图片解码峰值）。有损两夹具逐像素对照 Chromium，
  预乘通道允许 2 的舍入差，alpha 精确；真实 TLS resource 和 inline Application 安装
  及绘制已验证。产品 EXE 的 smoke 另验证实际私有入口，不能用 libtest 入口替代。
- WebP EXIF/ICC/XMP、动画调度仍未实现；本轮浏览器夹具为 sRGB，不证明一般色彩
  管理。v197 已补上上述坏 IPC/异常退出；解码崩溃、Job 分配失败及整组进程峰值仍待补齐，
  不把已有超时/限额测试描述成全部故障覆盖。

全锁文件 OSV Advisory 扫描覆盖 5 locks / 1786 packages，仍有 20 个漏洞 ID，
不能称全仓零漏洞；本次三个新增 native 包没有返回已知条目。许可证审计通过，
717 个 Rust package / 40 个许可证表达式；正式发布仍需 clean exact-ref 的全门禁。
CycloneDX 1.6 / SPDX 2.3 已按当前锁文件重新生成，1228 个去重身份均通过官方核验。
WebP 专项 9 passed，Application WebP 两项通过；产品 startup 7 passed。全量仍有一个
UIA Select 系统交互失败（2684 passed / 1 failed / 47 ignored），详见迁移台账；
不能用图片专项通过替代全仓回归通过。
后续仅添加 UIA 失败阶段诊断的版本全量 2685 passed / 47 ignored（120.10 秒）；
保留前述失败，不宣称 UIA 根因修复。两轮生产 WebP 源码未变，详见迁移台账。

## 已验证证据

候选为纯 Rust `image-webp = 0.2.4`，不是 `webp`、`libwebp-sys` 或系统可选 WIC
扩展。crates.io 实时返回该版本未 yanked、许可证 MIT OR Apache-2.0；本地 crate
包 SHA-256 与 registry 一致：
`525e9ff3e1a4be2fbea1fdf0e98686a6d98b4d8f937e1bf7402245af1909e8c3`。
直接依赖为 byteorder-lite 与 quick-error。前置 OSV 单包查询未返回已知条目；
v196 全锁文件扫描结果见上文，不能用单包结果覆盖全仓既有发现。

审查源码是本机 Cargo registry 的原始 0.2.4 包，没有复制/修改第三方实现。
[上游仓库](https://github.com/image-rs/image-webp)；
[WebP 格式 RFC 9649](https://www.rfc-editor.org/rfc/rfc9649)。

1. `decoder.rs:551-555` 明确标注部分分配忽略 memory limit；静态 read_image 未把
   该限制交给 lossy/lossless 解码器。当前用途是 ICC/EXIF/XMP read_chunk，不是
   工作内存硬上限。
2. `decoder.rs:699-702` 先完整解码 VP8，再比较 VP8X canvas 与内部尺寸；
   `vp8.rs:1190-1205` 已按内部尺寸分配 YUV。只检查公开 dimensions 不够。
3. `lossless.rs:100-124` 会在 transforms 前检查 VP8L 尺寸，但 `:307-343` 的
   entropy image 可导出最多 65536 个 meta Huffman group，每组五棵树，没有公开
   的 group 额度接口。输入/像素上限不能冒充内部工作内存上限。
4. VP8 partition 会先依据声明长度分配，再读取；lossless 无 alpha 和 lossy alpha
   另有中间像素缓冲。调用方 output_buffer_size 不代表完整解码峰值。
5. `new` 会扫描扩展 RIFF chunk，不只是固定头；read_image 对动画返回首帧。
   读/seek 取消 wrapper 也无法打断已在内存中运行的解码 CPU 循环。

安全的小尺寸实验已经实际执行，而非仅据注释推断：

```text
memory_limit_zero_static_decode=true width=2 height=1 output_bytes=8
outer_inner_mismatch_new_accepts=true canvas=1x1 payload=2x1 read_error=Inconsistent image sizes
```

第一项把 memory_limit 设为零，仍成功解码 2×1 RGBA；第二项把真实 2×1 VP8L
放入 1×1 VP8X，new 接受外层尺寸，到 read_image 才拒绝不一致。第二项是小尺寸
VP8L API 实验，不冒充已实测 VP8 大分配或峰值。没有运行内存耗尽攻击样本。
可重跑夹具位于 `linshi/otter-webp-audit/`，结果为 `otter-webp-audit-result.log`。

## 接入决策

不能把该 decoder 直接放进主进程 worker 后，仅调用 set_memory_limit 就宣称维持
现有内存边界。v196 已采用短命、受限的本机子进程解码，不依赖用户安装 Store codec，
也不因失败退回主进程无限制解码。

## 生产链约束

- RIFF preflight：checked 长度/padding、输入上限、有界 chunk 数；拒绝冲突/重复
  载荷，校验 VP8X/VP8/VP8L 内外尺寸一致；拒绝 animation flag、ANIM、ANMF。
  这只减少准入风险，不能替代进程内存上限。
- 专用子进程入口在 settings/profile/桌面/网络初始化前分流；参数不接用户路径或
  URL，只接受父进程匿名管道中的有界编码字节。启动路径绑定当前候选可执行文件，
  不使用 PATH 搜索或 shell，不继承业务凭据环境，不传递无关可继承句柄。
- 父进程在投递不可信字节前确认 Windows Job 分配及内存限制成功；失败即关闭并
  回收 child。需有进程上限与全部解码 child 的总预算，而非每个任务各自无限扩张。
  Job 限制不是完整权限 sandbox；不能据此宣称阻断所有文件/网络系统调用。
- 父进程掌握截止时间、撤销及退出清理；stdout/stdin 同时有界泵送，不能因满管道
  让取消/超时失效。失败、崩溃、过预算、超时均回收句柄/线程/进程，不隐式重试。
- IPC 校验 magic、版本、尺寸、长度、完整结果和 child 成功退出。输出分配在父侧
  继续计入 Pixels，共享槽、Lease、同 Grant、原始快照/摘要和迟到投递门保持。
- 真正接入 Resource/Inline → Job → decode → Ready → HWND，而非只提供独立
  helper。无 alpha RGB、RGBA 均转换为预乘 BGRA；不同布局不得直接当 D2D 像素。

## 完整验收条件（未全部完成）

必须有 lossless/lossy/alpha 的独立浏览器像素基线及真实 Application 安装/绘制，
并验证错误 MIME、超大内/外尺寸、截断、重复 chunk、动画拒绝、撤销、超时、坏 IPC、
child 崩溃与 Job 内存准入失败。正常及每条失败路径确认无子进程/句柄/任务残留；
测量总峰值并记录实际配置的硬限制，不能把父侧 Pixels 计数当全部进程内存。

生产依赖必须固定精确版本并更新 Cargo.lock、许可证/SBOM及安全扫描证据。测试
夹具放 linshi，不改 Cargo registry；不以开发机可选 codec 存在代替便携性验证。
WebP EXIF/ICC/XMP、动画调度仍需各自证据，不用静态首帧冒充全部格式能力。
