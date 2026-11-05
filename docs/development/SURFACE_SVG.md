# 原生 Surface SVG：实际基线与接入边界

## v229：滤镜内部结果名不再泄漏为显式输入名

检查结果引用时复现了独立于 offset 像素差异的解析错误：未命名 primitive
被 usvg 分配内部名 result1 后，后续显式 in="result1" 会误命中它。纯色
夹具中，红色未命名输出、蓝色命名输出、未知名称输入，本应退回前一蓝色
输出，却取到红色。前向引用、feMergeNode 与 in2 同样受影响。

固定 usvg 在 Primitive 内增加私有 explicit_result 标记，显式引用只匹配
已出现的作者命名结果；默认输入仍使用前一 primitive 内部名，首项默认仍
为 SourceGraphic。同名结果继续由 renderer 逆序选择最近前项；不是禁止
重复名称，也不是把未知/前向引用一律拒绝。输入关键字优先级保持不变。
这对应 Filter Effects Level 1 第 9.2 节的局部命名与回退规则：
https://www.w3.org/TR/filter-effects-1/#FilterPrimitiveInAttribute

隔离探针原代码 2 passed / 3 failed；候选 Debug 与 Release 均 5 passed。
8 个实际 Chrome 夹具的 DOM、Canvas 显示及读回均符合预期，包括显式与
隐式输入、未知/前向引用、重复名称、merge 和第二输入。45 文件从固定
registry archive 重放逐字节一致后导入，只改 parser/filter.rs 和
tree/filter.rs；完整补丁及来源清单同步，native 锁和依赖版本不变。
parser/filter 的既有 1091 行债务仅做字段接线和谓词修复，不借机重构；
新增行为回归独立归 surface_svg_filter_reference_tests。

内部版本为 v0.2.33.229。该修复是底层引用语义修复，生产 policy 仍整体
拒绝 filter/fe*，新增测试明确保护这一点；未宣称参数语义验证全部完成，
未导入 offset 的回退候选。证据目录 linshi/otter-filter-reference-candidate，
生产原生回归与静态检查结果记录于该目录收据。

导入后原生库全量串行回归 2853 passed / 0 failed / 53 ignored；此前定向
surface_svg 过滤运行 69 passed / 0 failed / 4 ignored，不能混称完整 SVG
生产功能验收。Clippy all-targets、fmt、行数 ratchet、来源/SBOM 合同
11 项及 diff 检查通过。SBOM 重生为 1257 项，usvg sourceSha256 为
9bc31c93c1c59e9184e7ed2e57941ad9477fff79d0e4cc8ffa7106e30a11a6a2；
许可证检查覆盖 749 个 Rust 包、40 类表达式。parser/filter 有效行
1091→1096（字段接线与格式化谓词），tree/filter 579→580，新测试 76 行，
worker 139→142。未重建产品 EXE、未跑新漏洞扫描、未公开发布。

## v228 后续：固定源码与三条绘制路径的 offset 对照

按实际 Chrome revision `1d17a2e2ee147bb7d6ae2ab6abdb3a2510c17775`
在线核对 Chromium DEPS 的 Skia revision
`886e42fda11610a56dd1b22e8697e27f9d1e34f2`，下载并哈希绑定 7 份对应源码。
该版本 SkImageFilters::Offset 明确构建 nearest 采样的平移；FilterResult
绘制阶段另外处理 AA、图像边界和 layer crop。源码引用不再仅依赖 main，
但仍不是本机浏览器执行分支的跟踪证据，也未复制上游实现到生产。

同场采集 30 个 SVG：15 个位移乘普通/透明扩展输入，同时显示 DOM、默认
Canvas 与 willReadFrequently=true Canvas。后者仅作为额外读回路径对照，
不凭属性就宣称已证明具体 CPU/GPU 执行分支，也不替换旧 oracle。
60 项 Canvas 截图/读回全部一致；既有半像素邻域 DOM/Canvas 48 项重采
逐字节保持一致；6 个零位移路径/输入控制完全相同。

三路径并不等价：DOM/默认 Canvas 14/30 相同，读回 Canvas/默认 Canvas
12/30 相同，读回 Canvas/DOM 9/30 相同。以普通矩形 -0.5 为例，默认
两路径的首个满 alpha 像素在 x=3；读回路径在 x=4 且 x=11 为 alpha128。
读回路径从 -0.501 到 -0.5 也发生边界变化，而原两路径该邻域保持相同。
这排除了“各路径共享一个半值舍入特判”的修补方案；不能以软件倾向路径
较易拟合为由放宽默认路径验收。最近邻取样和几何 AA 仍须分别建模，后续
还要核对裁剪/图像局部坐标与实际执行路径，不能把本轮观察写成根因已修。

证据为 linshi/otter-offset-sampling-paths/verification.json 与
linshi/otter-offset-pinned-skia/verification.json，保留全部路径、源码 URL
及 SHA256。生产 sourceHashes 复核未变，版本仍 v0.2.33.228；没有导入
回退候选或开放 filter。浏览器会话已关闭。

## v228 隔离候选：几何传递有效，但简单覆盖率模型仍有回退

在 linshi/otter-offset-geometry-candidate/package 实际实现并编译了候选：
group 源 stroke bbox 进入首个 filter，SourceGraphic/SourceAlpha 显式保留
几何元数据，颜色转换和命名引用保留，offset 平移后传递；未核实路径以
None 表示未知，不扫描 alpha 猜测。候选在既有 nearest 平移后应用几何
像素覆盖率；未知边界继续既有行为。它是诊断实现，不是完整结果边界模型。

24 个旧样本逐项对照原 DOM/Canvas：Canvas 精确 5→12，总差
27818→10538；DOM 精确 3→10，总差 28296→11016。但 -0.5-x-center
和 -0.5-xy-center 在两条基线下都回退，故未导入生产。新增带透明留白的
20 个矢量/零位移对照中，候选对两条基线均仅 10/20 精确。编译测试通过
只证明夹具执行完成，不能被写成像素回归通过。收据记录全部单项差异。

另采集 24 个半像素邻域浏览器观察：±0.499、±0.4999、±0.5001、±0.501
的普通矢量结果在各自 DOM/Canvas 路径下均与对应 ±0.5 逐字节相同。
因此两个回退不是仅由恰好半值的 ties 舍入触发，不能加 .5 特判绕过。
Canvas 显示/读回 24/24 一致，DOM/Canvas 仅 8/24 一致；±1.5 与透明
扩展对照同时保留，未选择更容易匹配的显示路径。浏览器会话已关闭。

候选与邻域证据分别位于 linshi/otter-offset-geometry-candidate/verification.json
和 linshi/otter-offset-half-boundary/verification.json。生产源码、版本、
filter 拒绝策略未变。下一实现还需要表达源图边界、采样映射和裁剪组合，
不能将几何覆盖率直接当作完整 offset 修复；N01–N12 仍未完成。

## v228 后续定位：小数 offset 丢失输入几何边界区分

重新采集默认 Chrome 156.0.8078.47 的 32 个小数平移观察及 8 个零位移
控制，保留 DOM 截图、Canvas 显示与读回；Canvas 显示/读回 40/40 相同，
DOM/Canvas 28/40 相同。原 8 个中心矢量样本对冻结两条基线均逐字节一致，
没有覆盖旧 oracle 或切换到软件读回来规避差异。

四种输入为普通矩形、同矩形加透明扩展矩形、紧边界 PNG 和带透明留白 PNG。
8 个零位移控制都等于同一份精确像素。非零位移中，紧边界 PNG 与普通矩形
8/8 相同，透明扩展矢量与带留白 PNG 8/8 相同，但两类之间均有差异；
例如 -0.25-x 的两类总差 720。仅扫描 alpha 非零区域会把两类混为一谈，
因此不能用它恢复浏览器所需的输入边界区分。

使用当前生产固定 resvg/usvg 源码和 native 对齐锁的独立探针实际渲染了
16 个矢量样本。usvg 保留了普通输入 [4,4,12,10] 与透明扩展输入
[0,0,16,16] 的 object/stroke bbox；两者 layer bbox 都为 [0,0,16,16]。
然而两类原生结果在 8 组平移中全部相同，普通输入仍匹配旧 translation
结果。当前 apply_offset 只收到像素与 filter-region 信息，未贯通源内容
几何边界。这定位了需要保留的元数据，不等于已经确定 Skia 每个采样阶段。

下一实现应从 group 的源几何边界进入 filter 调度与 SourceGraphic/结果
元数据，验证变换、引用和 primitive 裁剪传播；不能只在 offset 中扫描透明
像素或全局替换成双线性。半像素时 DOM/Canvas 自身仍有分歧，仍需保留双基线。
本轮未修改生产代码、版本或准入；证据在
linshi/otter-offset-input-bounds/verification.json，包含输入、截图、RGBA
读回、native 输出、来源与精确探针 EXE 哈希。浏览器会话已关闭。

## v228：真实 Job/IPC 中的累计预算拒绝实测

新增仅 cfg(test) 可用的资源 peer，经过生产进程 owner、128 MiB Job、
15 秒 IPC 截止和 host，再调用与生产相同的 rasterize。rasterize 保留
RenderError，生产 decode 边界仍映射原 InvalidData；测试 peer 仅把
FilterBudgetExceeded 编码为退出码 42，其余失败为 41。父侧同时检查没有
成功/部分 header、退出码和 Job 峰值，避免把超时或 Job kill 当预算拒绝。
每个夹具还断言解析后滤镜非空、primitive 个数与类型，避免静默忽略假阳性。

最终 Release 探针通过：10 组样本各跟一次正常请求，共 20 个独立受限 child，
5 个明确预算拒绝、15 个成功。拒绝样本为 2048² turbulence/9 octave、
2048² 9×9 Wrap 卷积、1024² flood 重复执行 64 次、2048² sigma=100
Gaussian、2048² 64 输入 merge。其余正常样本覆盖小 flood、512² 单 octave、
1024² 大半径 morphology、component gamma 和 sigma=3 Gaussian。

实测最高 Job 提交内存 37,736,448 字节，最慢样本为重复 flood 的 3,029 ms；
全部在现有 15 秒 deadline 内结束，拒绝后正常 child 成功。数据包含测试
child 和进程启动成本，不是 renderer 独占内存或稳定耗时保证，机器未做
独占性能隔离。额度仍是 4,000,000,000 策略积分，本轮没有据少量样本调整。

重要边界：测试回调直接生成/解析夹具，故意绕过尚未开放的生产 XML 白名单；
成功仅返回 1×1 确认帧，不测大图传输或像素等价，也不覆盖完整 normalize、
viewport、字体和 BGRA 转换管线。首轮较弱的 renderer-error-only 结果保留，
最终验收只使用补齐精确错误/类型断言后的 release-probe-final.log。
证据与源码/EXE 身份在 linshi/otter-filter-work-resource/verification.json。
filter/fe* 准入、浏览器像素差异及 N01–N12 总计划仍未完成。

## v227：累计滤镜工作量策略接入

新增 render_with_filter_budget；每次请求只有一份单调递减额度，按实际
执行的滤镜实例和 source 栅格面积计费，不按 XML 定义数或 primitive 的
裁剪子区域打折。pattern/mask、clip paint、SVG image 与 feImage 共享额度，
不会因嵌套 render 入口重置。预算溢出或不足均粘性失败，不退还额度；后续
节点/滤镜停止执行，返回 FilterBudgetExceeded，调用方丢弃整个目标像素。
既有 render/render_checked 保留入口，原生 worker 使用显式 4,000,000,000
积分上限，将失败映射为 InvalidData。该阈值是未做耗时标定的策略参数。

work_budget 独立拥有 checked 整数运算和权重：全 source 拷贝/颜色转换、
卷积 taps 与 Wrap 循环、turbulence octave、merge 输入数、Gaussian 的实际
小核策略及大 sigma 中间帧保守边界等。Wrap 参考图可能小于 source，采用
最小一像素轴估算负坐标循环，避免错误地按 source 宽高低估。此额度不是
总 SVG CPU、墙钟或峰值内存保证；metadata 扫描、组源图绘制/分配及失败后
部分清理不由积分完整覆盖，XML/引用图限制与 Job/超时仍各自负责。

库测试 33 passed；与 native 锁一致的 Debug/Release 外部合同各 9 passed。
覆盖精确临界、重复定义的多次执行、失败后新请求、pattern/mask、SVG image、
feImage、参数权重及微小子区域不折扣。两个故意重置嵌套额度的负对照均被
相应测试发现。五项坐标溢出仍返回 FilterFailed，两项普通像素仍逐字节匹配
修复前基线。worker 专项新增直接后端超大 octave 拒绝与后续正常请求检查；
它不是经生产准入的滤镜渲染测试。native SVG 为 146 passed / 0 failed /
3 ignored；最终全库与产品启动结果见同版本迁移记录及验证收据。

固定归档重放 45 文件，10 修改、13 新增，无删除且 byteIdentical；resvg
sourceSha256 为 3b43ac426b2c00ae76d747bf9a67c6e0edf817f9b344405b114ce213d321f1ed，
patchSha256 为 47e8c2cc9481587d99a49ae210a97eef6ff356dd762933c6220a851ff3997ee7。
证据在 linshi/otter-filter-work-budget，含 import.json、scoped.diff 和
verification.json。其余三个固定依赖源码未变。filter/fe* 与 filter 属性
仍被生产 policy 拒绝；浏览器像素差异、资源标定与最终准入尚未完成。

## v226：依赖接入后的全库复核

最终测试源码的 native lib 完整串行运行 2846 passed / 0 failed / 51 ignored；
生产 worker、四份依赖源码和滤镜拒绝策略保持 v225 字节。仅增强测试诊断，
没有调整产品行为或超时。此前全库失败未被覆盖：UIA 最短顺序曾再次复现
展开后失焦/收起，现场原生条目仍在；夺焦来源与点击依赖旧失败原因未证。
详见 WINDOWS_NATIVE_MIGRATION.md v226 与 linshi/otter-v225-suite-order-audit。
本轮没有重建产品 EXE、重做发布 SBOM 或运行新的漏洞扫描；不是滤镜准入。

## v225：坐标溢出拒绝与渲染错误传播已接入

resvg 在实际采样处检查浮点截断后的 i32 域、lattice +1、stitch 减法和
倍增运算，失败返回错误而非 wrapping/saturating 生成像素；最后一个
octave 不再计算不会使用的下一状态。正常合法域内保留原运算顺序。
新增 render_checked，以每次调用独立的 Cell 状态报告滤镜执行失败；
pattern/mask、clip paint、feImage 和嵌套 SVG image 贯通错误，局部 bbox
语义保留。该接口不声称报告所有不可绘制几何或分配失败；错误时目标可能
含部分内容，调用方必须丢弃。既有 render API 保留兼容签名。
异常路径存在有意收紧：feImage 子树发生滤镜执行错误时，整个外层
primitive 失败，不再保留其中其他局部内容。不能声称 legacy render
对所有异常输入保持旧像素；正常像素结论限于已验证的成功夹具。

真实 surface_svg_worker 已改用 checked 接口，将错误映射为 InvalidData，
不会输出伪成功 BGRA；不增加进程、权限、解析器或父侧 XML。filter/fe*
及 filter 属性仍被生产 policy 拒绝，所有新滤镜回归均是直接 renderer
测试，不是滤镜生产准入。累计像素工作预算与剩余浏览器像素差异仍未完成。

五项原有极值夹具在 Debug/Release 均明确 Err(FilterFailed)，两个普通
夹具与修复前完整 RGBA 字节相同。含原预算回归的外部测试两种构建均
8 passed；库测试 25 passed，覆盖嵌套失败和后续正常调用不受污染。
完整 43 文件从固定上游归档重放逐字节一致，新增两个模块独立测量为
31/100 有效行；上游 filter/mod.rs 因必要错误接线从 871 增至 892 行，
未增加独立业务职责，避免本次同时拆散滤镜执行调度。turbulence 为 297 行。
证据保存在 linshi/otter-turbulence-coordinate-fix；完整迁移仍未完成。

接入后串行 SVG 专项 145 passed / 3 ignored。首轮两套默认并发测试
重叠，出现 child 容量/管道超时，原始日志保留且未放宽时限。随后完整
native lib 串行结果为 2844 passed / 2 failed / 51 ignored：失败是
UIA select 系统重定位后的键盘焦点断言，以及 surface edit 依赖点击
快照得到 after click 而非 at click。同一源码分别单独复跑各 1 passed；
不能据此改写全库失败记录或判定根因。后续需定位整套运行下的不稳定性。
来源合同 11 passed，许可证 749 Rust/40 expressions，双 SBOM 核对
1257 身份；没有新增 Advisory 扫描或正式发布授权。
实际产品 EXE 的隐藏 Win32/D2D/图片 child 启停冒烟 1 passed；native
Clippy all-targets -D warnings、库 Clippy、官方格式、行数与 diff 检查通过。
verification.json 绑定最终源码、来源、日志、SBOM 和实际 EXE 摘要，明确
保留 fullNativeSuitePassed=false；不是发布候选或全面手测交付。

## v224 后续审计：确认坐标/stitch 溢出，尚未修复

在固定 16×16 输出、最多九个 octave 下建立七项实际渲染对照。Debug 中
两个普通场景成功，五个故障场景复现：大频率 x/y 的 noise2 索引 +1 溢出，
以及 stitch 宽度倍增溢出；频率 .2、scale(2e-7)、extent=8e7 的有效缩放
场景也触发索引溢出，证明只限制原始 baseFrequency 不是坐标安全边界。
调用当前仓库预算模块确认七项均通过解析预算，同时当前功能 policy 仍
拒绝全部七项。它们是直接 renderer 诊断，不是生产滤镜已开放或被绕过。

相同源码与 native 锁定依赖身份下，Release 七项不 panic，但三个极值
输出全透明；两个普通场景的 Debug/Release 像素一致。没有浏览器极值
oracle，不判定 Release 像素正确，也不以其测试退出成功覆盖 Debug 故障。
故障点为 turbulence.rs 的 bx1/by1 加法及 stitch.width 倍增。修复应在
实际执行坐标/整数边界与明确失败传播处完成，不能仅改为 wrapping 运算
来消除 Debug panic，再把静默结果当作兼容通过。

初始过小 scale 夹具被 transform 有效性门过滤、未产生滤镜，已单独标为
无效夹具而不计入 renderer 故障；最终七项均确认实际生成一个 filter。
诊断初始解析到更新的 smallvec/crc32fast/syn，已保留其日志并改从 native
锁文件派生，再独立重跑 Debug/Release；最终回执核对全部诊断包身份均
属于 native 锁集合。原错误/中间输出未删除，不混用旧锁结果。

证据在 `linshi/otter-turbulence-coordinate-audit/verification.json`，包含
生成 SVG、像素、源码、锁文件、日志及两个 EXE 的哈希。本轮未改生产
代码、依赖或内部版本，仍为 v0.2.33.224；这些溢出尚未修复，滤镜准入
保持关闭，累计工作预算、完整等价与 N01–N12 均未完成。

## v224：修复 turbulence 最小 signed seed 的实际渲染溢出

16×16、两个 octave 的真实 resvg 渲染复现 seed=-2147483648 触发
`attempt to negate with overflow`，位置为 init 的 `-seed`。该 seed 可由
有限 f32 精确表示，不能靠“非有限数值拒绝”覆盖。失败测试/日志已保留。

仅将非正 seed 的绝对值计算改为 unsigned_abs，先对 RAND_M-1 取模后再
转回 i32 并加一；结果必在正的合法范围内，原正 seed 分支不变。库内
新增宽 i64 算法对照，覆盖 i32::MIN、邻值、负值、零与 i32::MAX。

同一外部测试源码验证四类输入（最小 seed、解析后饱和到最小 seed 的
-1e38、零、普通负 seed）× turbulence/fractalNoise × stitch/noStitch，
共 16 个实际渲染与对应正规化正 seed 全像素一致；不只是“没有 panic”。
候选及仓库接入后的 debug 构建均通过，库 release 测试 21 passed。

完整来源补丁重新从固定官方归档重放，41 文件逐字节一致；相对前轮
只有 turbulence.rs 改变。来源清单/补丁及双 SBOM 已刷新，依赖版本与
features 不变。证据在 `linshi/otter-turbulence-seed-audit/`。
本修复不处理变换后坐标、stitch 累增整数或累计工作量边界，也不开放
filter/fe*；没有新 Advisory 扫描或正式发布放行，N01–N12 未完成。

接入后原生 SVG 相关测试 145 passed / 3 ignored / 0 failed，来源/SBOM
合同 11 passed；库与 native Clippy -D warnings、格式、行数和 diff 门
通过。修改后的 turbulence.rs 独立测得 270 有效行；许可证审计通过，
双 SBOM 核对 1257 身份。verification.json 绑定失败/成功日志、完整重放
及仓库文件，未沿用候选通过来替代接入后的实际渲染测试。本轮未重跑
整个原生库套件、产品 EXE 冒烟或大图资源 Job。

## v223：纠正卷积 order 的数值转换，不混用 octave 规则

复核固定 usvg 源码并用真实 Tree 解析确认：卷积 order 的数值经 f64
向零截断；numOctaves 则先转 f32 再 round。v222 预检错误地给两者共用
f32/round，实际将 order=9.5 的合法 9×9 核拒绝。保留失败测试与日志，
修复预检后不改变 renderer、限额或滤镜功能白名单。

数值流仍只保存前两个数，但保持 f64 到各属性自己的转换点：order 使用
trunc，octave 保留 f32/round。新对照测试直接读取 usvg 生成的卷积矩阵
行列数，覆盖 9.5、9.999999999、3.9 2.9、1.999999999 1；同时检查截断
为零/超限阶数被拒绝，以及 octave 9.49/9.5 的原边界保持不变。

同一份测试源码在修复前 1 failed（9.5 被拒），修复后通过；全部 SVG 相关
测试 145 passed / 3 ignored / 0 failed，Clippy all-targets -D warnings、
格式、行数 ratchet 和 diff 检查通过。证据在
`linshi/otter-filter-order-audit/verification.json`，包含原失败源码/日志。
依赖身份、worker 接线和功能白名单未变；本轮未重跑产品 EXE 冒烟或完整
原生库套件。后续坐标/总工作预算及滤镜准入仍未完成，N01–N12 未完成。

## v222：接线滤镜解析预算与模板引用图，不扩大功能准入

surface_svg_filter_budget 已接入真实 worker，位于 XML 解析之后、policy
与 usvg 之前。它不收集输入控制的数值 Vec，只保留两个数供阶数/octave
检查，超限立即失败；不截断列表、钳制输入或改写 XML。

本阶段预算为最多 64 个定义、每定义 64 个直接工作子节点、全图 256 个
fe 节点（包括 merge/function/light 子节点）；merge 最多 64 个输入，
component transfer 最多四个函数，lighting 最多一个光源。数值总数
16384，单 table 1024、kernel 81、matrix 20；order 两轴限 1–9，numOctaves
限 0–9。初版误将 order 也按 f32/round 转换，v223 已改为实际 usvg 的
f64/trunc；octave 仍为 f32/round。识别的纯数值
属性拒绝非法列表与非有限 f32；长度/枚举/引用不误按纯数值解析。

这些是 Otter 的开发预算，不是 SVG 标准上限，也不是总 CPU/内存承诺。
联网核对 [Filter Effects 草案](https://www.w3.org/TR/filter-effects-1/)：
卷积核增大有 CPU 代价；octave 的规范说明允许按色深限制其处理数量。
本实现选择明确拒绝超出预算的处理数量，而非隐式截断或宣称像素等价。

原引用图增加 filter 模板 href 和应用 filter 的边，沿用非递归环/深度
检查及重复 ID 拒绝。测试覆盖自环、互环、过深模板和共享无环模板。
预算测试覆盖各限额边界、unused definitions、子节点、累计列表、极大
数值及 XML 不变；它们不是完整结构/参数语义验证，也不会开放滤镜。

仍待：变换后的 turbulence 坐标/整数安全、卷积与 octave 的累计像素工作
预算、参数语义与结果引用、全部像素等价及生产准入。有限 baseFrequency
本身不能证明渲染安全；现有 Job/超时是最后一道资源边界，不能替代这些
前置工作。filter/fe* 和 filter 属性仍全部拒绝，N01–N12 未完成。

实际验证：全部 SVG 名称测试 144 passed / 3 ignored / 0 failed，六项新
预算/引用图测试均通过；三个 ignored 为父测试调用的私有入口。实际 EXE
隐藏窗口与图片 child 冒烟 1 passed。Clippy all-targets -D warnings、
格式、行数 ratchet 和 diff 检查通过。新增生产模块/测试分别 91/131 有效行，
原 worker/reference 模块分别 109/207 行。未重跑整个原生库套件，不沿用
v221 的全量成绩作本轮全量证明。四个依赖身份及功能 policy 字节保持不变。
源码、日志和实际 EXE 哈希回执在 `linshi/otter-v222-filter-budget/verification.json`。

## v221：候选渲染依赖进入仓库，滤镜准入仍关闭

resvg 的 source-only 41 文件和 usvg 的最新 45 文件已从固定官方归档重新
重放，逐字节匹配后接入 third_party。resvg 保持原 Cargo/许可证；usvg
包含 morphology 半径溢出与 turbulence 非有限频率保护。native lock 仅
移除 resvg 的 registry source/checksum，版本及其他包无变化；实际解析
核对四个仓库路径，resvg 仍只开启 text。内部版本为 v0.2.33.221。

这次不是开放滤镜：policy 字节不变，filter、fe* 和 filter 属性继续拒绝。
实际 child 合同新增六种内容的根/嵌套拒绝，包括未引用定义、普通滤镜及
极大半径/频率/octave 参数。依赖内算法的浏览器后端差异、滤镜预算与
完整准入仍是后续任务，不能把新依赖接入当成这些能力已经可用。

来源/SBOM 合同 11 passed，许可证审计通过（749 Rust 包、40 表达式），
双 SBOM 为 1257 身份。联网 Advisory 扫描仍有 25 个漏洞 ID；没有增加
豁免或声明正式发布安全通过。九个 resvg 新模块为 51–136 有效行，既有
上游调度文件从 898 降到 871，usvg filter parser 保持 1091 行；来源
豁免仅覆盖固定上游副本，新增模块另验。行数基线仍是原 42 个历史条目，
只因新增精确副本排除更新 policy 身份，不添加债务豁免。

最终源码的原生库全量回归为 2839 passed / 51 ignored / 0 failed；这 51 项
未作为普通测试直接执行，不能计为 passed。SVG 专项 55 passed / 2 ignored，
实际产品 EXE 的 hidden Win32/D2D 启动、SVG/WebP child 与销毁冒烟 1 passed。
Clippy all-targets -D warnings、cargo fmt、行数 ratchet 和 diff 检查通过。
检查前后 1510 个 native 输入哈希不变，回执绑定实际测试/产品 EXE 与日志。

回执与备份位于 `linshi/otter-v221-source-import-20261022T093212858814Z/`。
完整验证记录见本次 verification.json；不是公开发布、
用户全面手测或 Windows 迁移完成，N01–N12 继续保持未完成。

## 候选依赖与原生宿主 SVG 链路的隔离集成

不修改生产 Cargo/源码，创建外部 native 测试工程，直接编译真实
native/src/lib.rs 与原 build.rs/windows.manifest。依赖改为 source-only
resvg、包含 morphology/turbulence 有限性修复的 usvg，tiny-skia 与
roxmltree 仍用仓库正式副本；锁文件只改变 resvg 来源，版本及其他包
不变。TEMP/TMP、构建产物全部位于 linshi 下独立目录。

实际测试分三组补齐名称筛选：surface_svg 55 passed / 2 ignored，
SVG 子模块补充 79 passed，应用/MIME/内联数据补充 4 passed / 1 ignored。
核对二进制完整清单后，全部 141 个含 svg 名称的测试均有归属：138 项
通过，3 项为父测试调用的私有 SVG/字体/伪造元数据子进程入口。包含
真实 Job child、字体 broker、外部内容/异常尺寸拒绝、文本/路径以及
应用层 SVG 安装与绘制，不再只依赖直接调用 resvg 的原型结果。

Cargo metadata 核对实际四个依赖路径和 resvg text-only feature，
构建前后 native 源码、Cargo 与 manifest 哈希未变。证据位于
`linshi/otter-native-svg-integration-20261022T091640934885Z/verification.json`。
这是候选依赖在隔离构建中的现有 SVG 非回退集成，不是已部署宿主测试、
全原生测试套件或新滤镜功能验收：生产准入仍拒绝 filter/fe*，这些新
滤镜没有通过此入口执行。没有生产导入、版本变更或发布，N01–N12 未完成。

## 滤镜数值解析审计：修复非有限 turbulence 频率 panic

对 15 类参数入口 × 11 个零值/符号/极大值/非法数值，共 165 个组合只做
小 SVG 解析，不执行潜在大 octave/kernel 工作负载。实际复现唯一 panic
为 feTurbulence baseFrequency=1e39：Vec<f32> 数值转换溢出后进入
PositiveF32::new(...).unwrap()。此前 morphology 修复在此次组合中未回退。

新隔离副本仅在 turbulence 两轴转换前增加非有限检查，返回透明占位
primitive，不改变有限频率。原失败测试源码完整保留，修复后同一测试
通过，另四个非有限单轴/双轴组合实际渲染全透明。普通原型 152 passed /
9 ignored / 2 filtered，morphology 专测 2 passed / 1 ignored。格式检查
按 usvg edition 2024 通过。没有把 165 项无 panic 扩大为渲染语义、
CPU/内存预算或所有滤镜安全证明，numOctaves 等执行预算仍需准入约束。

增量补丁、失败日志、原测试快照和 verification.json 位于
`linshi/otter-filter-numeric-audit/`。本轮未改原重放副本或生产源码，未
重跑新图资源 Job，未更新正式来源包或开放滤镜；N01–N12 未完成。

## usvg 完整来源重放：纳入 morphology 溢出修复

从原始 usvg 0.48.1 crate 重新生成完整补丁，不只附加单文件差异。归档
SHA-256 与仓库来源清单一致；安全解包后实际 git apply --check/apply，
45 个文件与已测试修复副本逐字节一致：9 个上游文件修改、5 个本地新增、
无删除。包括既有文本/布局修复和新的 morphology 数值边界修复。

调用仓库 verifyPackage 对新 schema-2 清单执行真实文件/补丁哈希和
新增文件清单检查，通过；候选 sourceSha256 为
`b5fb93c0a5758a55568a9576a7d71ad516d23b7646e45be23943228989cd1686`。
这是清单与实际字节一致性，不代表来源批准或正式依赖接入。

外部 harness 改为消费重放目录，locked/offline/release 的普通原型
152 passed / 9 ignored / 2 filtered；同一半径修复测试入口另运行
2 passed / 1 ignored。编译后重新检查全部源码与修复副本一致。本轮未
在重放目录构建的 EXE 上重跑资源 Job，前轮十项资源证据仍属于原修复构建。

完整补丁、归档解包、重放内容及 verification.json 位于
`linshi/otter-usvg-replay-20261022T090245436190Z/`。生产 third_party、
native Cargo 与版本未修改；正式来源审查、生产准入、安全/发布门和
N01–N12 仍未完成。

## morphology 溢出修复：2048² 极值与全套资源验证

对修复解析器新增独立 Job，实际渲染 erode/dilate × 1e38/3e38/1e39/
0 3e38/3e38 0 共 10 项 2048² 图像，全图逐像素验证透明或保色结果。
每项耗时 48–760 ms，单 Job 峰值 68177920 字节，128 MiB 限制不变，
外部 15 秒截止未触发。证明有限值限幅仍按像素范围计算，不随巨大半径
分配同等大小的工作缓冲；不是所有任意输入的总资源安全证明。

同时对使用修复解析器的完整原型 EXE 重跑既有九项独立资源测试，9/9
passed、退出码均 0、无超时，最高峰值 69857280 字节。加上新增极值
Job，本轮十个独立进程全部成功；未沿用修复前的资源结果。

日志在 `linshi/otter-morphology-parser-audit/` 下的
resource-suite-20261022T0859008618264Z 与
resource-suite-20261022T0859064161999Z，resource-verification.json 绑定
两个实际 EXE、测试源码、解析器修改、日志和限制记录。源码仍为隔离
修复副本，完整来源补丁重放/正式打包尚待更新，生产源码与准入未变，
N01–N12 未完成。

## morphology 解析边界：复现并修复半径溢出 panic

16×16 SVG 即可在半径转换触发 Option::unwrap panic：userSpaceOnUse 的
1e39，以及 objectBoundingBox 的 1e38/3e38/1e39。根因是解析得到非有限
f32，或有限半径乘 bbox 缩放后溢出，再传入 PositiveF32::new 后 unwrap。
不是图像尺寸或工作缓冲分配问题。

在新的隔离 usvg 副本中仅修改 parser/filter.rs：非有限输入返回透明
占位 primitive；有限半径使用 f64 缩放并饱和到有限 f32 上界，随后仍由
morphology 的像素范围限制计算量。大有限半径不回退零半径，零轴保持。
两项完全相同的回归在旧解析器上 2 failed，在修复后 2 passed；覆盖
18 个解析组合及 10 个实际渲染组合，验证 erode 透明、dilate 保色、
零轴和非有限透明结果。普通原型 152 passed / 9 ignored / 2 filtered。
rustfmt 按 usvg 的 edition 2024 检查通过，没有借格式化改动旧 import。

补丁、同测试前后日志和验证记录在 `linshi/otter-morphology-parser-audit/`。
原 aligned/source-only 副本和生产源码均未修改；本修复尚未重新打包、
重放完整依赖来源或重跑资源 Job。已有生产拒绝 filter 的边界保持，
不将隔离解析器 panic 描述成生产可达漏洞；N01–N12 未完成。

## aligned 源码包：库自身测试与 Clippy 补齐

Cargo 不允许从原型 workspace 直接测试外部依赖包的 dev-dependencies，
因此创建独立 library-suite 测试副本，源码和 Cargo manifest 保持与
source-only 包逐字节一致，只为该测试副本生成 lock。现有版本/来源
身份未升级；新增 lock 条目属于上游可选依赖，Cargo metadata 确认实际
resvg feature 仅 text，gif/image-webp/zune-jpeg/memmap2/fontconfig-parser
均未进入当前 resolve 节点，不能把 lock 出现等同于启用功能。

新副本实际运行 locked/offline/release 的 20 项库测试全部通过，
Clippy --lib --tests -- -D warnings 通过。metadata 确认 roxmltree 仍为
仓库补丁路径；源码、manifest 未变和有效 feature 图均有独立核验。
日志及 verification.json 位于 aligned staging 的 library-suite/。
这补充库内部边界验证，不替代既有 152 项原型与九项资源 Job，也不代表
正式安全扫描、生产 IPC 或准入门通过；生产依赖及 N01–N12 状态不变。

## 依赖对齐后的新 EXE：九项资源 Job 实测完成

对 aligned harness 的新 EXE（仓库 roxmltree/tiny-skia，重放 morphology
改动的 usvg，source-only resvg）重新枚举全部 9 项 ignored，并逐项以
独立进程实际运行。9/9 passed，退出码全部 0，无超时；各测试内部查询
确认 128 MiB Job 限制，外部仍保持 15 秒截止。峰值范围
19230720–69873664 字节；2048 blur/morphology 四项耗时为
298/370/243/557 ms。这是新组合实测，不沿用旧组合资源数字。

证据位于 aligned staging 的 `resource-suite-20261022T0844409669443Z/`。
verification.json 校验实际测试名、每进程 1 passed、九个独立 PID、
内存日志与 receipt 一致、执行前后 EXE 哈希，以及上一轮全部源码、
manifest、lock、Cargo metadata 与构建记录没有漂移。普通 152 项回归
沿用上一轮同一构建，本轮补齐新图全部 9 项资源测试。

没有修改生产依赖或准入，未宣称生产 IPC/字体 broker/完整宿主、漏洞
扫描、发布门或双后端像素等价已完成。N01–N12 仍未完成。

## 候选依赖对齐：修正原型 roxmltree 来源差异

实际比对发现 tiny-skia 诊断副本与仓库正式副本完全一致；usvg 仅
src/parser/filter.rs 的 morphology 半径处理不同，其他文件及清单一致。
但原型 Cargo.lock 的 roxmltree 仍来自 registry，不是仓库补丁副本。
此前像素及资源结果仍是对应旧依赖图的有效诊断证据，不能当作已覆盖
仓库 XML 解析器的证明。

新建独立 aligned harness：resvg 使用 source-only 包，tiny-skia 与
roxmltree 使用仓库路径；usvg 从仓库副本复制并实际重放唯一 morphology
差异，逐字节核对等于诊断副本。仓库 verifiedSources 校验三个正式包的
文件清单和补丁哈希均通过；Cargo metadata 确认四个包均解析到预期路径，
不是仅看 manifest 声明。新旧 lock 的结构比较只允许 roxmltree 从
registry 改为本地来源，版本及其他包完全不变。

全新 target 下 locked/offline/release 回归 152 passed / 9 ignored /
2 filtered。证据位于 `linshi/otter-resvg-aligned-20261022T084155683062Z/`，
包含 morphology 增量补丁、Cargo metadata、构建日志和 verification.json。
本轮未对新依赖组合重跑九项资源 Job，不能沿用上一轮峰值作为新图实测。
没有修改生产副本、Cargo、准入或版本；尚需新图资源验证与完整准入审查，
生产滤镜拒绝和 N01–N12 未完成状态保持。

## source-only 新 EXE：完整九项独立资源 Job

从新 EXE 的 --ignored --list 枚举并核对全部 9 项，不共用进程；每项
独立启动、异步收集 stdout/stderr、15 秒截止，只清理本次启动的进程。
全部 9 项均明确运行 1 test、1 passed，exitCode=0，无超时。测试内部
查询实际 Job 限制，确认单进程、128 MiB 进程/Job 内存与 kill-on-close；
外部核验器再次检查日志、峰值、进程 receipt 和执行前后 EXE 哈希。

覆盖 textLength 工作负载、1024/2048 blur+morphology、20 层矩阵链、
20 层 component 恒等链、component gamma/颜色融合/全部 alpha 缓存，
以及 2048 图像的小数/极端 offset。峰值 Job 最大 69873664 字节，来自
2048 blur+morphology；该进程四项耗时为 340/421/265/724 ms。各进程
峰值范围 19218432–69873664 字节，没有提高 128 MiB 限制。

完整日志在 source-only staging 下
`resource-suite-20261022T0837382175982Z/`，其中 processes.json 记录九个
独立 PID/退出码，verification.json 绑定所有日志、既有构建证据及新 EXE。
入口为 `run_resvg_source_resource_suite.ps1` 和
`verify_resvg_resource_suite.py`。普通 152 项回归沿用上一轮同一 EXE，
本轮补齐全部 9 项 ignored；两项生产拒绝策略测试仍未在绕过准入的原型
中执行。本结果不覆盖生产 IPC、字体 broker、完整宿主或正式发布门，
未改变源码包、生产依赖或滤镜准入，N01–N12 仍未完成。

## resvg：源码改动与诊断接线分离的候选包

在新的 source-only staging 中恢复上游 Cargo.toml/Cargo.lock，保留
Cargo.toml.orig 和许可证原字节；排除没有 mod 声明的 box_gaussian 与
scaled_gaussian 两个本地试验模块。原诊断副本未修改或删除。对新包重新
从原始 crate 生成补丁并重放：41 文件精确一致，6 个上游文件修改、
9 个新增、无删除，所有补丁路径均在 src/ 下，Cargo 元数据不再含本机路径。

外部 harness 持有诊断依赖补丁和原型入口，从全新 target 完成
locked/offline/release 构建及 152 passed / 9 ignored / 2 filtered 回归。
新 EXE 单独运行全 alpha 2048² Job：sRGB/linearRGB 184/179 ms，
峰值进程与 Job 53223424 字节，128 MiB 上限和 15 秒截止不变，exitCode=0。
这是一项独立 Job，不代表其余 8 个 ignored 已在本轮执行。

仓库语言感知 lexer 检查全部变更源码，9 个新增模块 51–136 有效行。
既有 filter/mod.rs 从上游 898 降至 871 行，仍为历史尺寸债务；其余
修改文件不超过 205 行，没有新增跨越 700 的文件。

源码包/外部 harness/验证 receipt 在
`linshi/otter-resvg-source-only-20261022T083229456579Z/`；新补丁、上游及
重放内容、行数报告在 `linshi/otter-resvg-replay-20261022T083320440916Z/`。
生成器 `prepare_resvg_source_candidate.py` 和核验器
`verify_resvg_source_only.py` 绑定实际新 EXE、日志及诊断依赖哈希。
这是 source-only staging，不是生产来源批准；尚未完成完整依赖来源审查、
安全/发布门或全部滤镜等价，未写入 third_party/native Cargo，生产拒绝
策略及 N01–N12 未完成状态保持。

## resvg 候选：原始归档补丁重放与独立编译

从本机缓存读取 resvg 0.48.1 原始 crate，SHA-256 与 native/Cargo.lock
固定值 `67e3803f97b999e80cbf7c6ecdd07a8102204d92e1633cf48783720c521196bd`
一致。只读取常规 tar 文件，拒绝链接、越界和重复路径，并设置成员及总
解包大小限制；新建独立目录，不清理或覆盖原候选。

生成相对原始归档的补丁并实际 git apply --check/apply：43 个文件逐字节
一致，8 个原有文件修改、11 个新增文件、无删除；唯一排除项是 Cargo
缓存标记 `.cargo-ok`。首次重放受 Windows Git 换行配置影响，出现 CRLF
差异；命令级 `core.autocrlf=false` 后精确重放成功，未改全局配置。

从重放目录执行 locked/offline/release 的 20 项库测试全部通过，并在
编译后再次核对全部 43 文件未变。独立 Cargo 配置仅把诊断 usvg/tiny-skia
路径定位到现有副本，重放包本身未改；该配置和两依赖的文件哈希均绑定
到 build-evidence。此证据不认证两依赖来源，也不证明完整原型或资源 Job
已由新目录重跑。

产物位于 `linshi/otter-resvg-replay-20261022T082906812184Z/`，包括
`candidate.patch`、`source-manifest.json`、`build-tests.log` 和
`build-evidence.json`。生成/核验入口为 `replay_resvg_filter_candidate.py`
与 `verify_resvg_replay_build.py`。补丁仍包含诊断 Cargo manifest/lock
及未接线的试验模块，status 明确为 diagnostic；没有写入生产 third_party、
调整 native Cargo 或放开滤镜。正式来源整理、审查、准入和 N01–N12 未完成。

## component：40 项双后端对照及线性索引分歧

原 16 项 RGB 与 24 项 RGBA 灰阶在软件浏览器重新实测，逐项确认 SVG
源文相同，DOM/Canvas 显示/读回一致，CDP 确认软件合成与光栅化状态。
候选对软件基线 31/40 精确、总差 288、最大通道差 3；对默认基线仍为
22/40、总差 1212、最大差 1。两个浏览器基线之间也只有 22/40 精确，
总差 1452、最大差 3。没有覆盖旧基线，也没有将软件路径替代默认验收。

候选的全部 sRGB 灰阶均匹配软件浏览器，剩余 9 项均为 linearRGB。
为区分阶段，再在软件会话运行相同二进制索引探针：sRGB 索引没有差异；
linearRGB opacity=0.5 的输入 x=143/144，软件索引 70、默认 71；
opacity=1 的 x=220 为 182/183，x=243 为 228/229。其余索引一致。
因此先前“索引一致”的证据限定于默认后端，不能推广到软件后端；
输出阶段和线性输入转换均存在后端相关量化差异。

新增 31 项软件基线严格断言，与原默认基线断言并存。普通原型实测
152 passed / 9 ignored / 2 filtered。renderer 未改，未将固定偏置或
像素容差作为修复，资源 Job 未重跑，生产滤镜拒绝保持。证据为
`linshi/otter-component-software-ramps-evidence.json` 与
`linshi/otter-component-software-indices-evidence.json`；会话均已关闭。
双后端差异需要在等价验收中明确记录，不能声称一个软件实现同时精确
复现两个彼此不同的基线。正式准入和 Windows 迁移 N01–N12 仍未完成。

## component 输出探针：默认浏览器与软件路径存在差异

固定 256 项 RGB 函数表输出，并分别固定输出 alpha 为 64/128/216/255，
在 sRGB/linearRGB 各生成一张 256×4 探针。表项取目标字节区间内部值，
避免十进制到 float 的边界舍入。默认会话与 `--disable-gpu` 会话使用
完全相同 SVG；各自 DOM/Canvas 显示/读回均一致。

默认会话的 sRGB 输出有 8/8/9/0 个灰阶位置偏离数学最近预乘；软件
会话四行全部精确。原生候选实际解码也与软件浏览器逐字节一致，新增
严格断言同时核对整数预乘公式。linearRGB 两会话分别有 15/5 个位置
偏离当前模型，未宣称线性路径等价。这里计数是灰阶位置，不是 RGB
三通道误差总和。未将硬件路径偏差拟合为候选固定舍入偏置。

软件会话 CDP 报告 gpu_compositing/rasterization 为 disabled_software，
displayType 为 ANGLE_SWIFTSHADER；不把启动参数本身当作后端生效证据，
也不将该信息扩大为每个滤镜的逐阶段执行跟踪。全部浏览器会话已关闭。
普通原型 151 passed / 9 ignored / 2 filtered；证据为
`linshi/otter-component-output-evidence.json` 和
`linshi/otter-component-output-software-evidence.json`，包含源码、PNG、
读回、配置、测试日志和精确 EXE 哈希。renderer 未修改，资源 Job 未重跑。
下一步区分原 40 项灰阶在两后端的表现及线性转换误差；生产滤镜拒绝、
正式来源准入和 N01–N12 未完成状态保持。

## component 查表索引实测与 Chrome 156 基线复核

新浏览器会话通过 CDP 报告 Chrome/156.0.8078.47，revision
`1d17a2e2ee147bb7d6ae2ab6abdb3a2510c17775`，系统显示后端为 ANGLE_D3D11。
设备列表和 GPU feature 状态保存于 receipt；这不是单个 SVG 滤镜实际
使用 GPU 而非 CPU 的执行轨迹证明。

构造八个二进制函数表分别输出索引的八个位，强制输出 alpha=1，避免
半透明读回丢失位值。sRGB/linearRGB × 输入 opacity 0.5/1 的四组完整
灰阶探针，其 DOM、Canvas 显示和读回一致；1024 个重建索引全部匹配
当前去预乘及工作空间转换模型。实际候选解码四张 SVG 也与浏览器像素
逐字节一致，新增严格回归，普通原型现为 150 passed / 9 ignored /
2 filtered。探针未发现索引阶段差异，后续重点是查表后的输出阶段；
不能将此结论扩大到所有输入、滤镜链或浏览器后端。

由于浏览器版本已变化，另在独立 refresh156 目录重采原 40 项灰阶，
逐项验证 SVG 源文相同，DOM/Canvas 显示/读回均与冻结旧基线一致。
当前候选仍 22/40 精确、总差 1212，剩余 18 项不是本次版本漂移所致。
证据：`linshi/otter-component-index-evidence.json`、
`linshi/otter-component-refresh156-evidence.json`；浏览器会话已关闭。
renderer 未修改，未重跑资源 Job，生产准入和 N01–N12 状态不变。

## component 剩余误差：排除全局精度与舍入替换

对上一轮固定 40 项灰阶执行 15 组独立管线模型，覆盖 double、float、
归一化 float、half、归一化 half，以及 half-up、ties-even 与
floor(value×256) 查表索引。double/float 的两种最近舍入均维持总差 1212、
22 项精确；half 模型至少 14 项总差回退；floor(value×256) 模型至少
18 项回退。因此这些全局替换均不进入候选实现，不能凭少数像素修正。
模型只模拟指定阶段精度，不代表本机浏览器实际执行了这些管线。

联网核对 Skia main 的 SkTableColorFilter 与 SkRasterPipeline_opts：
CPU 路径具有去预乘、byte_tables 与预乘阶段，byte_tables 使用
to_unorm(value,255)。这仅作为排查依据，不证明本机 Chrome 的后端与版本。
模型、输入哈希和全部逐项结果保存在
`linshi/model_component_pipeline_precision.py` 与
`linshi/otter-component-pipeline-model.json`。本轮没有改变 renderer，
也没有新增浏览器采样或运行资源测试；上一轮结果未被重新包装成新测试。
下一步需要取得本机浏览器后端信息，并用能直接区分查表索引的探针定位，
而不是继续全局更换舍入或以一字节容差宣称等价。

## 滤镜候选：component 函数表的计算精度

独立数学模型与实际候选输出在 40 项 RGB/RGBA 灰阶中逐字节一致。
linear 改为字节域 float 运算，table 插值与 gamma 幂计算使用 double，
discrete 保留 float 系数乘法；不是统一改用 double 或放宽比较阈值。
核对的上游参考为 Chromium main 的
[fe_component_transfer.cc](https://raw.githubusercontent.com/chromium/chromium/main/third_party/blink/renderer/platform/graphics/filters/fe_component_transfer.cc)，
不将该分支当成本机 Chrome 精确版本证明。

固定浏览器基线下，精确项由 18/40 增至 22/40，总绝对通道差 2964→1212，
无单项总差回退。新增四项严格断言是 RGB/RGBA 两组的 opaque sRGB
linear/table；仍有 18 项不精确。原型 149 passed / 9 ignored / 2 filtered，
候选 20 项单测、Clippy、定点 rustfmt 通过。已有恒等和常量工作负载严格
断言保持通过；本轮使用冻结浏览器基线，没有重新运行浏览器。

全 alpha 的 2048² 独立 Job 两种工作空间为 185/192 ms，峰值进程/Job
53841920 字节，128 MiB 上限、15 秒截止不变，exitCode=0。资源数字仅
适用于此诊断进程。完整前后像素、源码、EXE 和日志哈希见
`linshi/otter-component-lut-evidence.json`，生成脚本为
`linshi/verify_component_lut.py`。生产 v220/filter 拒绝不变，正式来源接入、
剩余显示差异和 N01–N12 仍未完成。

## 滤镜验证：component 全 alpha 缓存的大图边界

新增一张图包含 0–255 全部输入 alpha 的条带工作负载。先对未过滤 256×1
参考输入逐字节断言 alpha 值，确保不只是 SVG 参数宣称覆盖；然后分别在
sRGB/linearRGB 下比较 512×3 和 2048×2048 输出与过滤后小参考图的空间
放大结果，逐像素保持一致与预乘约束。零输入 alpha 经函数偏置产生覆盖率
也作明确断言。大图每行重复全部 alpha，覆盖索引行建立及反复复用。

实际独立 Windows Job 内两项 2048² 工作负载为 243/224 ms，峰值进程/Job
内存 53792768 字节，128 MiB 上限未提高，外部 15 秒截止未触发，exitCode=0。
这是同一算法大小尺寸一致性和最坏缓存覆盖的验证，不是新增浏览器等价，
也不是整个原生宿主或生产 IPC 的资源验收。

普通原型 149 passed / 9 ignored / 2 filtered；新增 ignored Job 已单独实际
执行通过。renderer 本轮未修改。源码、精确 EXE、日志和进程 receipt 哈希
见 `linshi/otter-component-alpha-cache-evidence.json`。生产 v220/filter
拒绝保持，显示差异、正式来源接入及 N01–N12 未完成。

## 滤镜候选：component 融合扩展到 alpha 变换

先为旧 12 项工作负载补齐真实浏览器对照，逐字验证 Rust/浏览器使用同一
SVG，DOM/Canvas 显示/读回 12/12 一致。旧像素仅 3 项 gamma 精确；其余
linear/table/discrete 存在误差，最大 77。因此旧“保持原像素”断言不能
代替正确性，本轮有独立浏览器证据后才将这 12 项升级为浏览器严格断言。

融合路径新增 alpha 函数表：输入 alpha 决定去预乘索引，输出 alpha 决定
最终预乘。透明输入仍以直 RGB=0 参与函数，不能在 alpha 函数生成覆盖率
之前清空返回；输出 alpha 为零时清空。缓存单测扩展到 identity/改变 alpha
两种表，覆盖所有合法输入 alpha/channel 配对和两种工作色彩空间。
取消先前 RGB-only 限制，但仍仅对 sRGB 编码输入融合；已在线性空间的输入
保留旧路径，恒等节点也不改变已有处理方式。

旧 12 项常量工作负载由 3→12 项精确，总差 118895→0。新增 24 项 RGBA
完整灰阶（包含零输入 alpha）三种浏览器观测一致，旧路径/融合路径独立
运行比较：9→13 项精确，总差 13140→1488，无单项总差回退；透明输入生成
gamma 输出及两个半透明 discrete 已加入严格断言。仍有 11 项未精确。
先前 16 项 RGB-only 灰阶与 60 项恒等输出逐字节不变。

原型 148 passed / 8 ignored / 2 filtered；候选 20 项单测和 Clippy 通过。
2048² 改变 alpha 的 gamma 在独立 128 MiB Job 内三次 117/115/146 ms，
像素 [9,16,37,127] 全图一致，峰值 52895744 字节，正常退出、未超时。
证据见 `linshi/otter-component-alpha-fusion-evidence.json`，包含浏览器基线、
前后固定像素、精确 EXE/源码与日志哈希。生产 v220/filter 拒绝不变，
剩余显示差异、已在线性空间的输入、正式准入及 N01–N12 未完成。

## 滤镜候选：融合 RGB-only component 的颜色转换与查表索引

独立模型验证后，新增 component_transfer_color 模块：从 sRGB 预乘输入
直接求直通道、转换到工作色彩空间并选择 8 位函数表项；输出先转回 sRGB，
最后预乘，避免中间线性预乘字节的重复量化。仅接入 alpha 严格保持不变的
非恒等 RGB 变换；恒等节点仍保留原工作空间路径，alpha 变换和已在线性空间
的输入仍走原实现。先前试验曾改变 alpha 变换的旧输出，尚无完整浏览器
证据，故未纳入本次接线；这不是全部 component 链已等价的声明。

16 项完整灰阶由 3/16→5/16 全像素精确，总通道差 10476→1476，最大差
76→4，无单项总差回退；sRGB/linearRGB 两项半透明 discrete 均精确，已
升级为严格断言。Rust 16 项输出逐字节等于独立融合模型；旧 60 项恒等样例
和 12 项改变 alpha 的工作负载均保持原像素。仍有 11 项未精确。

转换不逐像素执行幂运算：为每个实际出现的 alpha 惰性建立索引行，最多
64 KiB 栈上索引，加上固定输出表；作用域仅限单次调用，没有持久缓存或
额外完整图像。单测遍历全部合法 alpha/channel 配对，比较缓存与直接计算，
并保持预乘约束。独立 128 MiB Job 内 2048² RGB-only gamma 的 sRGB/linearRGB
两项完成，最终复跑实测 121/120 ms，峰值 52891648 字节，正常退出且未触发 15 秒
截止；不与上一轮改变 alpha 的不同负载计算加速比。

原型 147 passed / 8 ignored / 2 filtered，新增 Job 已单独执行；候选
20 项单测、Clippy 通过。证据及固定 `.pre-fusion.bgra`/`.fused.bgra` 见
`linshi/otter-component-fusion-evidence.json`。生产 v220/filter 拒绝不变，
正式来源接入、剩余显示差异及 N01–N12 未完成。

## 滤镜诊断：完整灰阶定位 component 分段边界

新增 16 项 256×4 灰阶样例，逐列覆盖 0–255：linear/gamma/table/discrete、
sRGB/linearRGB、输入 alpha .5/1。DOM/Canvas 显示/读回 16/16 一致；候选
3/16 精确，已纳入严格回归，其余仅测量。半透明 discrete 最大通道差 76，
说明常量色块不能替代完整色阶边界检查；性能回归通过不等于显示等价。

独立模型以实际原生未过滤输入为起点，逐字节重现 16 项原生输出。额外获取
未过滤浏览器输入以及相同输入编码成 PNG 后再次经过滤镜的对照：未过滤
原生/浏览器、PNG 解码后预乘字节、矢量/PNG 过滤后浏览器结果，均 16/16
一致。三组浏览器截图和读回也各自一致。本组差异已排除输入栅格化或仅矢量
绘制路径的解释，下一步应处理 component 内部的量化与颜色转换。

模型发现 sRGB 半透明 discrete 的两处色阶输入为 c=64,a=128，旧 f32
去预乘中间值略低于半整数，落入错误分段；精确比值舍入可在模型中修复该组。
但对 linearRGB 全局使用同一变更，会使 discrete 总差 1584→2880，table
3336→3360；直接截断也有多项回退。因此没有据此改全局去预乘或扩大准入。
模型结果不等于已实现修复，也不证明浏览器内部具体指令。

本轮 renderer 未改。原型 147 passed / 7 ignored / 2 filtered，新增 3 项
完整灰阶精确回归通过。证据链为 `linshi/otter-component-ramp-model-evidence.json`
及 `otter-component-ramp-inputs-evidence.json`，包含三组原始浏览器数据、
截图和输入/输出哈希。临时浏览器已关闭；生产 v220/filter 拒绝不变，
N01–N12 未完成。

## 滤镜候选：非恒等 component 使用有界通道查表

原路径对每个像素重复计算 gamma/linear/table/discrete。候选现对至少 256
像素的输入，在栈上为每个实际转换通道建立 256 项 u8 表，并复用原标量
transfer 函数生成条目；小图保留直接计算，恒等/空表不建表。没有更改浮点
公式、舍入、颜色空间或 alpha 顺序，没有持久缓存、堆上像素副本或新线程。

九组函数分别穷举 256 个通道字节与原标量比较，覆盖负斜率、单项表、负/零
gamma 指数等边界；12 项完整 SVG 回归覆盖四类函数与 225/256/1024 像素，
切换边界两侧均逐字节等于修改前。该回归证明行为保持，不是新增浏览器等价。

同机独立 128 MiB Windows Job 内各执行三次 2048² gamma 工作负载：修改前
579/559/553 ms，修改后 136/152/160 ms；输出均为 [9,16,37,127]，且逐像素
验证全图一致。两次峰值进程/Job 内存均为 52822016 字节，进程正常退出，
外部 15 秒截止未触发。这是一个确定工作负载的观察，不推广为整个应用加速比。

原型 146 passed / 7 ignored / 2 filtered，新增 ignored Job 已单独执行；
候选 19 项单测、Clippy、相关格式检查通过。修改前精确 EXE/源码、修改后
EXE、固定像素与日志哈希见 `linshi/otter-component-lookup-evidence.json`。
生产 v220/filter 拒绝不变，剩余显示差异、正式准入和 N01–N12 未完成。

## 滤镜候选：component 恒等函数保留工作色彩空间

新增 60 项 feComponentTransfer 样例：空节点、显式 identity、恒等 linear、
gamma、两端点 table，交叉两种色彩空间、三种输入 alpha 和完整/裁剪区域。
DOM/Canvas 显示/读回 60/60 一致；原候选 40/60 精确，另 20 项总差 8460。
试验发现不能照搬算术输入选择的整段旁路：跳过工作色彩空间转换会让 10 项
原本精确的 linearRGB 不透明样例退步。数学恒等不保证浏览器像素路径恒等。

最终候选仍执行 input.into_color_space(cs)，只跳过严格恒等 component 的
去预乘、逐通道运算和再预乘。60 项输出与原候选逐字节一致，40 项同时作
浏览器精确回归；不宣称剩余 20 项显示差异已修复。系数、表格判定采用精确
比较，离散非空表、非恒等和近恒等参数不走旁路；其它运算路径保持不变。
新增共享 SourceGraphic 裁剪后再使用的回归，确认不会原地损坏共享输入。

独立 Windows Job 内实际执行 20 节点、2048×2048 恒等链，所有输出像素
正确，峰值进程/Job 内存 52789248 字节，128 MiB 上限未提高，外部 15 秒
未超时，进程正常退出。没有测量优化前同负载峰值，不宣称具体节省比例。
原型 145 passed / 6 ignored / 2 filtered，新增 ignored Job 已单独执行；
候选 18 项单测及 Clippy 通过。证据见
`linshi/otter-component-identity-native-evidence.json`。生产 v220/filter
拒绝不变，来源接入、剩余滤镜等价与 N01–N12 未完成。

## 滤镜候选：算术精确选择输入不再损失颜色

当 feComposite arithmetic 系数严格为 (0,1,0,0) 或 (0,0,1,0) 时，数学上
仅选择第一个或第二个输入，颜色转换可以省略。候选原先仍做颜色空间往返，
使低通道色阶丢失精度。现由 composite::selected_input 作精确系数判断，在
转换前直接移交对应 Image，保留其真实色彩空间标签，不克隆像素、不额外
分配结果缓冲。primitive 区域裁剪仍由原公共路径执行；其它系数完全走旧路径。

新增 24 项浏览器样例：两种色彩空间、三种真正输入透明度、两个输入选择、
完整/裁剪区域。第二输入有整数位移，色阶覆盖低颜色值。DOM/Canvas 显示/
读回 24/24 一致；候选由 12/24 精确、总差 5524，提升为 24/24 精确。
另以 24 项后续颜色矩阵串联对照检查所选输入的色彩空间标签与引用语义。
精确判断单测拒绝每一项系数的 EPSILON 扰动及非有限值，不使用近似相等。

原有整体 opacity 与输入 alpha 两组合计 96 项输出逐字节不变。原型
143 passed / 5 ignored / 2 filtered，候选 resvg 17 项单测及 Clippy 通过。
新增/修改的独立测试和 composite 模块均小于 100 物理行；遗留 mod.rs 仅
增加 8 行分派接线，不借本修复扩展其主要职责。固定像素、源码和日志哈希见
`linshi/otter-composite-selection-native-evidence.json`；浏览器证据见
`otter-composite-selection-evidence.json`。生产 v220/filter 拒绝不变，
普通算术量化差异、生产来源接入与 N01–N12 仍未完成。

## 滤镜覆盖纠正：整体 opacity 不等于输入 alpha

上一轮 composite 样例在带滤镜元素上使用 `opacity`，实际覆盖滤镜完成后的
整体透明度；此前将它描述成半透明输入不准确。现保留原 48 项回归，并新增
使用 `fill-opacity` 的 48 项真正输入 alpha 基线，不把原结果改写成新场景。
其中 30 项浏览器结果与整体 opacity 场景不同，证明两者不可互相替代。

新基线 DOM img/Canvas 截图/读回 48/48 一致。候选精确 29/48，总通道差
7376、最大差 13；新增 29 项严格像素回归，其余 19 项仍待解决。旧组仍为
34 项严格回归，两个组分别统计，不能合并宣传为 96 项兼容通过。

独立 Python 模型按真实顺序处理滤镜输入、算术、颜色转换及末端整体透明度，
逐字节重现旧组 18 项 Rust 算术输出。对 36 种阶段精度/舍入组合的模型检查
没有得到全部精确方案；最低总差组合也仅 13/18 精确，不能据此选取生产规则。
模型只解释旧组整体 opacity 场景，不声称覆盖新输入 alpha 组或 GPU 实现。
联网核对 [SkBlenders 算术合同](https://api.skia.org/classSkBlenders.html)，确认
算术表达式与可选 RGB/alpha 约束；该 API 文档不证明当前浏览器的中间量化。

本轮 renderer 未改，原型 141 passed / 5 ignored / 2 filtered，相关格式及
diff 检查通过。证据为 `linshi/otter-composite-precision-evidence.json`、
`otter-composite-input-alpha-evidence.json` 和
`otter-composite-input-alpha-native-evidence.json`。临时浏览器已关闭；
生产 v220/filter 拒绝保持，N01–N12 未完成。

## 滤镜验证：48 项 composite 基线与融合试验回退

新增 over/in/out/atop/xor 和算术 sum/product/bias，交叉 sRGB/linearRGB、
.25/.5/1 整体 opacity，共 48 项。第二输入为交换红蓝后的整数位移，覆盖独占、
重叠和透明区域。DOM img、Canvas 显示截图与 Canvas 读回三者 48/48 一致，
没有沿用 offset 中已发现分歧的假设。原候选 34/48 全像素匹配，剩余总差
3760、最大通道差 13；已将 34 项加入严格回归，另 14 项仍是测量而非通过。

试验将 sRGB 输入到 linearRGB 的转换和算术合成融合，避免中间 8 位量化。
总体差降至 1944、最大差降至 4，但精确数仍为 34，且 linearRGB 不透明
sum 单项总差 80→240，故撤回接线，不按个别系数特判。试验源码和固定
`.fused-trial.bgra` 保留在 linshi。恢复后 48 项原候选像素逐字节一致。

普通原型 139 passed / 5 ignored / 2 filtered，Clippy 通过，临时浏览器
已关闭。三路径、拒绝试验、恢复验证分别见 `linshi/otter-composite-evidence.json`、
`otter-composite-trial-evidence.json`、`otter-composite-restored-evidence.json`。
生产 v220/filter 拒绝保持，N01–N12 未完成；后续颜色/合成改动须同时保护
已精确项并核对剩余单项差异，不能只用总体误差下降作准入依据。

## 滤镜验证：offset 极值与受限进程运行

浮点平移候选新增 36 组极值测试，覆盖正负画布宽度、1e9、1e38，分别
作用于 x/y/双轴，并覆盖 viewBox 放大导致乘法超出有限 f32 的路径。
所有结果均为空透明图像，没有崩溃；这是当前夹具的行为验证，不是无限输入
安全证明。新增模块 `linshi/otter-v215-context-prototype/src/offset_bounds_probe.rs`。

真实独立进程在 Windows Job 的 128 MiB 进程/Job 内存上限、单进程限制和
外部 15 秒截止时间内，完成 8 项 2048×2048 SVG 位移：零位移、小数、整数、
完全移出及极大坐标。验证中心像素或全透明输出，并检查所有像素预乘约束。
单项耗时 54–94 ms，峰值进程/Job 内存均为 69648384 字节；exitCode=0，
未超时，进程已退出。不是完整原生宿主或生产 IPC/font broker 的资源验收。

普通原型 137 passed / 5 ignored / 2 filtered；新增 ignored Job 用例已单独
实际执行通过，不将普通测试中的 ignored 计作已执行。候选 resvg 16 项单测
另行通过。源码、精确 EXE、进程 receipt 和日志哈希见
`linshi/otter-offset-bounds-evidence.json`。本轮未改变渲染算法或生产来源；
v220/filter 拒绝保持，剩余显示差异与 N01–N12 仍未完成。

## 滤镜候选：保留 offset 平移精度，区分显示路径

对上一轮 24 项样例分别使用默认/显式 primitive 区域，获取 SVG img 与
Canvas 并排截图：Canvas 截图和自身读回 48/48 一致；SVG img 与 Canvas
仅 38/48 一致，最大通道差 127。显式 primitive 区域没有改变任何结果，
因此本组差异不是单纯的读回量化，也不能用补充区域属性消除。
另以矢量输入、相同内容内嵌 PNG、无滤镜 transform 作 36 项观察：Canvas
截图/读回 36/36 一致，img/Canvas 27/36 一致，PNG 输入也有差异，不能
归因成只发生于矢量输入。实验并未确定 Chrome 内部实现的精确根因。

保留默认采样，仅将候选 feOffset 的整数绘制原点改为浮点平移 Transform，
避免进入绘制前向零截断；不采用上一轮已拒绝的独立双线性方案，也不修改
tiny-skia 公共采样策略。原 24 项分别对照两种观测：Canvas 总差
62788→27818，精确 0→5；DOM 总差 55618→28296，精确 0→3。
两种观测下均无单项总差回退；这不是所有像素或所有 SVG 都无回退的证明。

新增三个双方一致样例的严格回归，以及 25 组整数位移/越界裁剪数学像素
断言。原型 136 passed / 4 ignored / 2 filtered，候选 Clippy 通过。
源码、固定像素和日志哈希见 `linshi/otter-offset-translation-evidence.json`；
显示路径证据见 `otter-offset-paths-evidence.json` 与
`otter-offset-inputs-evidence.json`。修改仅在隔离候选，生产 v220/filter
拒绝不变，未发布或完成 N01–N12。小数 offset 的剩余显示差异仍待解决。

## 滤镜诊断：小数 offset 与显示基线分歧

新增 24 项正负 .25/.5/1.25、x/xy、中心/边缘 feOffset 样例。现有候选
将位移转为整数，默认 Canvas 对照精确 0/24，总绝对通道差 62788。
直接指定 tiny-skia Bilinear 仍会被纯平移分支降为 Nearest；独立透明边界
双线性试验达到 2/24、总差 46244，但 10 项单项总差回退，因此未接纳，
已恢复原候选接线。试验源码保留在 linshi，不编译进入候选。

独立实际 DOM img 截图与默认 Canvas 只有 19/24 全像素一致；五个半像素
样例最大通道差 127。此前 blur/flood 的 DOM 一致结论不能推广到 offset。
因此先保留两种观测，不选择更容易通过的一种作为准入基线，也不宣称修复。
全部 SVG、截图、固定 `.before.bgra`/`.bilinear.bgra` 及哈希见
`linshi/otter-offset-evidence.json`，复核脚本 `verify_offset.py`。

恢复后的原型 134 passed / 4 ignored / 2 filtered，候选 Clippy 通过。
新增测量测试并非像素一致断言；临时浏览器已关闭。生产 v220/filter 拒绝、
N01–N12 未完成状态保持。下一步应分离半像素显示裁剪、采样和绘制路径，
不能直接把整数转换替换为插值就宣告浏览器等价。

## 滤镜候选：feFlood 预乘前保留浮点 opacity

候选原先先把 opacity 转为 u8，再与颜色预乘，给 RGB 引入额外量化。
现改为以归一化 RGB 和原浮点 opacity 构造 tiny-skia Color，最终填充时
再量化；没有添加针对浏览器少数 alpha 边界的特判。修改局限现有 flood
转换点，不新增滤镜、数据或进程 owner，也不改生产来源。

64 项基线总绝对通道差 6016→5248，单项总差无回退，精确数仍 22/64。
新增 .1/.3/.9 的预乘顺序回归，明确数学 RGB 与最终 alpha 的区别；不是
把原生 alpha 与浏览器不同的结果冒充显示验收。原型 133 passed /
4 ignored / 2 filtered，Clippy 通过。固定像素 `.before-float.bgra` /
`.float-opacity.bgra`，源码和日志哈希为 `linshi/otter-flood-float-evidence.json`。
生产 v220/filter 拒绝保持，未提交推送发布，N01–N12 未完成。

## 滤镜诊断：透明度扫描与独立图像复核

新增 1031 次浏览器 alpha 观测（0..1 的 1025 个二进制分数及六个十进制
检查值），每项并列 feFlood 和普通白色矩形。矩形全部符合 round(value×255)；
feFlood 只有 6 次观测相差 1，其余符合。这里 .5 出现两次，不能把六次
观测误报为六个不同取值。全局 floor255 产生 512 的累计差，未支持截断修复。

再将 65 个二进制分数拆成独立 8×8 SVG 图像，alpha 与合并扫描逐项一致，
严格断言通过；未发现本组差异来自合并绘制。尚未证明精确量化机制，不把
少数取值硬编码成生产规则。新增脚本/原始浏览器输出/哈希和全部扫描结果
保存在 `linshi/otter-flood-ramp-evidence.json`。临时浏览器已关闭，renderer
未修改，生产 v220/filter 拒绝保持，N01–N12 未完成。

## 滤镜诊断：feFlood 半透明像素与真实页面核验

新增 64 项浏览器 feFlood 基线：四种颜色、八种 opacity、sRGB/linearRGB。
原生当前仅 22 项全像素一致，余下最大通道差 1；测量测试执行成功不代表
兼容通过。原 .5 样例颜色一致但 alpha 为原生 128、浏览器 127；扩展后
发现 .501 的浏览器 alpha 又是 128，不能简单全局改为截断。

对两种颜色×八种 opacity 的 16 项实际 DOM img 获取透明背景 PNG，逐项
解码/预乘后与默认 Canvas 输出完全一致，排除这些样例只是读回差异的假设。
没有因此泛化到所有浏览器、GPU 或真实 WebView2。联网核对
[Chromium feFlood](https://raw.githubusercontent.com/chromium/chromium/main/third_party/blink/renderer/platform/graphics/filters/fe_flood.cc)
主分支，它以浮点颜色和 opacity 构造颜色滤镜；不能据此确定本机精确版本
的最终量化机制。本轮不加入针对 .5 的特判，renderer 未修改。

新增 flood probe 实际执行通过，16 项 DOM/Canvas 严格证据断言通过。
基线、PNG、脚本哈希见 `linshi/otter-flood-evidence.json`，临时浏览器已关闭。
本轮未重跑生产完整库，生产 v220/filter 拒绝保持，未提交推送发布，
N01–N12 未完成。

## 滤镜候选：按最后引用释放中间图

确认原候选保留全部 primitive 结果到滤镜结束，即使后续不再引用。新增
result_liveness 扫描所有输入种类（含双输入和 merge），记录每个结果名的
最后使用位置；当前 primitive 完成且 region 处理结束后释放无后续引用的
旧结果，并保留最终输出。引用解析仍按原顺序从后向前匹配，不改同名覆盖
或 SourceGraphic/SourceAlpha 语义；真实长寿命分支仍会保留，非统一常量内存保证。

1024²、20 级通道交换链在同一 128 MiB Job 限额下，修改前峰值
94908416 bytes，修改后最终留档为 19238912 bytes（约 90.5→18.3 MiB）；
两次均退出 0，全部输出像素验证不变。另新增远距离命名引用、composite/
merge 双输入、offset 引用与重复结果名回归，96 项固定模糊输出逐字节不变。

普通原型 131 passed / 4 ignored / 2 filtered，20 级 Job 单独运行通过，
Clippy 通过。证据 `linshi/otter-filter-liveness-evidence.json` 绑定最终 EXE、
源码与前后 Job 日志；仍非生产 IPC/任意滤镜图验收。生产 v220/filter
拒绝不变，未提交推送发布，N01–N12 未完成。

## 滤镜候选：独立 Windows Job 资源实测

复用隔离原型的 Job ABI 包装，新增独立进程诊断：128 MiB process/job
内存上限、单进程、kill-on-close，外部 15 秒超时。实际解析/渲染完整 SVG，
分别运行 .5 双轴、9 双轴、32 双轴 Gaussian 与 radius=100000 的 dilate；
验证输出尺寸、中心精确颜色及全图预乘约束。不是生产 IPC/准入/字体 broker。

1024²/2048² 两个进程实际退出 0、无超时，查询并断言 Job 配置生效。
最终留档峰值 Job 内存分别 19193856/70508544 bytes（约 18.3/67.2 MiB）。
四个 case 的渲染耗时分别 79/101/66/133 ms 与 331/400/261/663 ms。
这是两种固定尺寸和简单滤镜负载的实测，不能推断更大尺寸、任意图或真实
产品进程满足预算；未提高生产资源上限，也未开放滤镜准入。

脚本 `linshi/run_filter_jobs.ps1` 保存退出码、日志，证据
`linshi/otter-filter-job-evidence.json` 绑定测试 EXE 与源码哈希。普通回归
129 passed / 3 ignored / 2 filtered；新增两项 Job 测试默认忽略且已分别
单独运行通过，不能误称常规回归直接包含它们。生产 v220 保持，未提交
推送发布，N01–N12 未完成。

## 滤镜候选：小双轴核一次二维量化

联网核对 Chromium 的 paint_filter_builder/interpolation_space 及
[Skia blur engine](https://raw.githubusercontent.com/google/skia/main/src/core/SkBlurEngine.cpp)：
shader 路径对足够小的双轴核选择单次二维卷积，其余走分轴。这里只将公开
main 作为方向证据，不声称与本机 Chrome 155 完全同版；未复制第三方实现。

候选新增 small_gaussian_2d：仅对非零双轴且核面积不超过 28 的有限核执行
二维加权、透明边界、一次输出舍入；大半径和单轴路径不变。40 项白色样例
精确数 16→18，新增 .5-xy-center/edge 严格断言；总差 11340→11308。
32 项彩色样例总差 37416→37324，精确数仍 10，两组均无样例总差回退。
残余像素差继续保留，不据此推断其他设备或 GPU 同样行为。

新增透明边界/分派上限测试，候选库 16 passed；原型 129 passed /
1 ignored / 2 filtered，Clippy 通过。整图输出缓冲仍需生产 Job 资源验证。
证据 `linshi/otter-small-2d-evidence.json`，前后固定像素使用
`.before-2d.bgra` / `.small-2d.bgra`。生产 v220/filter 拒绝不变，未提交
推送发布，N01–N12 未完成。

## 滤镜诊断：转换舍入与部分浮点假设

独立 `linshi/blur_transfer_rounding_model.py` 固定卷积核，对八项小 sigma
linearRGB 样例分离进入/离开颜色空间的舍入规则。round/round 总差 6036，
round/floor 14789，floor/round 17629，floor/floor 30071；保持输入量化而
改用浮点卷积为 12180，仅颜色浮点、alpha 量化为 11703。未支持全局改为
截断或局部浮点卷积的假设，未修改 renderer，也未放宽准入。

按浏览器输出 alpha 分桶，0–15、16–63、64–127、128–255 的颜色最大差
分别 4/7/6/5，颜色总差分别 238/483/1974/3270；不是只在近透明像素出错。
这里统计仅 RGB 通道，不能与含 alpha 的总差混用，也不等价于视觉显著性。
模型继续通过先前 16 项 8 位路径重现断言及本次基线总差断言，原始结果和
输入/脚本哈希为 `linshi/otter-blur-transfer-rounding-model.json`。
本轮为定位证据，不宣称新增生产验证；v220/filter 拒绝与 N01–N12 未完成
状态保持，未提交推送发布。

## 滤镜候选：跨颜色空间串联与精确通道映射

新增八组实际浏览器链路基线：双交换、两种顺序的跨空间增益、模糊后交换、
增益后模糊。初始候选仅 sRGB 双交换精确；linearRGB 双交换总差 1644、
最大差 2，暴露了纯通道重排仍执行颜色往返的问题。候选新增精确识别 RGB
选择矩阵，要求每行恰好选择一个输入通道、无 alpha 混入/偏置且 alpha
完全不变；通道选择与逐通道颜色变换可交换，因此直接重排原空间预乘字节。

单次 linearRGB 交换和两种空间的双交换均精确匹配浏览器，增加严格断言。
27 种 RGB 选择及逐个系数加 EPSILON 的反例单测通过。linear blur→sRGB
swap 总差 5700→582，反向组合 1356→36；但最大通道差仍分别 13/1。
全部八链路仅 2 项完全一致，其余六项仍未通过，不能泛化为串联滤镜验收。
输出均满足预乘约束，增益和模糊误差没有被阈值掩盖。

候选库 15 passed；原型 129 passed / 1 ignored / 2 filtered，Clippy
通过。证据为 `linshi/otter-filter-chain-evidence.json`，浏览器已关闭。
生产 v220/filter 拒绝保持，未提交推送发布，N01–N12 未完成。

## 滤镜候选：融合矩阵两侧的颜色转换

独立 matrix_fusion_model 从无滤镜的浏览器输入像素出发，保留输入颜色、
工作颜色和返回颜色之间的浮点中间值，仅在最终写回时量化。候选显式矩阵
实现同一处理，不先分配/量化一个 8 位工作颜色图；输出 Image 的 color_space
保持实际输入空间，避免错误标记为矩阵工作空间。复用 color_transfer 的
标量传递函数，未复制一套颜色转换实现，也未扩大到整个浮点滤镜图。

六组 Rust 输出全部逐字节匹配独立模型。sRGB 三组仍与浏览器精确一致；
linearRGB 的 alpha/gain/swap 总差分别从上一候选 4602/7632/4008 降至
858/4212/1446，最大通道差均降至 1，alpha 仍一致。线性组未完全一致，
没有放宽验收阈值；多级滤镜串联与反向颜色空间转换仍需独立浏览器验证。

候选库 14 passed；原型 128 passed / 1 ignored / 2 filtered，Clippy
通过。模型为 `linshi/matrix_fusion_model.py`，固定像素 `.fused.bgra`，
归档 `linshi/otter-matrix-fused-evidence.json`。生产 v220/filter 拒绝保持，
未提交推送发布，N01–N12 未完成。

## 滤镜候选：非恒等矩阵的中间量化

以相同 64 色阶×8 alpha 新增六组浏览器基线，覆盖 sRGB/linearRGB 的
RGB 增益 .5、alpha 增益 .5 和红蓝通道交换。候选此前 sRGB 前两组总差
分别 1230/1260，交换一致。新增 matrix_premultiplied 模块，对显式矩阵
保留浮点直通颜色和矩阵结果，最终预乘输出时一次截断，避免中间 u8 损失。
三个 sRGB 样例现全部逐字节一致，已加严格断言；透明输入偏置和钳位单测通过。

linearRGB 仍不一致：alpha/交换总差保持 4602/4008，RGB 增益总差
6594→7632 回退。该候选不是全面修复，更不能生产放行；颜色空间转换和
矩阵融合仍需进一步验证。其他矩阵种类继续走原实现，没有伪称全部覆盖。
候选库 14 passed，原型 128 passed / 1 ignored / 2 filtered，Clippy 通过。
证据见 `linshi/otter-matrix-evidence.json`，浏览器已关闭。生产 v220/filter
拒绝状态不变，未提交推送发布，N01–N12 未完成。

## 滤镜候选：精确恒等矩阵不引入颜色损失

新增五组 Chrome 155 默认 Canvas 基线，每组含 64 色阶×8 alpha 的图形：
无滤镜、sRGB 一次/两次恒等矩阵、linearRGB 一次/两次恒等矩阵。无滤镜
控制组原生与浏览器完全一致；浏览器其余四组也与控制组一致。候选原先的
linearRGB 两组各产生最大通道差 6、总差 4008，alpha 没有差异。

候选现在对精确的 20 系数恒等矩阵保留输入，不进行不必要的颜色空间往返
转换；既有外层 primitive region 处理保留。不使用近似相等判断，逐个系数
加 f32::EPSILON 的反例均不视为恒等。五组浏览器结果现全部逐字节一致，
并固定严格断言。该结果说明恒等路径需要保真，不证明浏览器一般颜色转换
的内部精度；非恒等滤镜和模糊剩余误差仍待排查。

候选库 13 passed；原型 127 passed / 1 ignored / 2 filtered，Clippy
通过。证据为 `linshi/otter-transfer-evidence.json`，脚本/浏览器日志使用
`otter-transfer-` 前缀；临时浏览器已关闭。生产 v220/filter 拒绝保持，
未导入生产来源、提交推送或发布，N01–N12 未完成。

## 滤镜诊断：全浮点中间图假设未得到当前基线支持

新增独立 `linshi/blur_color_precision_model.py`，直接读取已验证的零模糊
预乘像素，分别计算 8 位逐轴量化、全浮点、浮点颜色但逐轴量化 alpha。
覆盖 sigma .5/2 的 16 项彩色样例，不包含大半径缩放。8 位模型在全部
16 项逐字节重现固定 Rust transfer 候选，并有严格断言保护这一前提。

与浏览器对照，sRGB 总差为 8 位 228、全浮点 1013、仅颜色浮点 826；
linearRGB 分别为 6036、11992、11868。全浮点仅改善其中 2 项，整体更差。
因此不把“增加浮点精度”当作已证实的兼容修复，不在缺少证据时引入更大的
中间缓冲。该实验不能证明浏览器内部使用何种精度；下一步需独立分离颜色
空间往返转换与模糊采样。没有改变 renderer 或生产准入。

模型输出、输入像素和脚本哈希保存于
`linshi/otter-blur-color-precision-model.json`；模型断言实际通过。
本轮未重跑生产完整库，v220/filter 拒绝保持，N01–N12 未完成。

## 滤镜候选：颜色转换合并中间量化

候选原有 Pixmap 转换依次反预乘到 u8、查表到 u8、再次预乘到 u8。
新增独立 color_transfer，以浮点完成反预乘和传递函数，再一次舍入写回，
保留 alpha 并清零透明像素。删除候选中已无调用的两张表和转换函数，
未修改生产来源。转换后仍是 8 位中间图，并非全浮点滤镜图实现。

32 项彩色半透明样例总绝对通道差 40219→37416，10 项精确结果保持；
sigma .5/2/9 三组总差分别 3372→2678、5304→3586、31543→31152。
9-xy-linearRGB-edge 的总差回退；没有新增完全一致项，不能据此认定颜色
兼容完成或所有样例改善。浮点传递函数的热路径开销也尚未通过生产资源门。

新增遍历全部合法 8 位 alpha/channel 的单调性、alpha 保持、预乘约束及
参考值/透明清理测试，候选库 12 passed；原型 126 passed / 1 ignored /
2 filtered，Clippy 与新模块格式检查通过。源码/日志/前后像素哈希归档为
`linshi/otter-blur-transfer-evidence.json`，前后像素分别保留
`.before-transfer.bgra` / `.transfer.bgra`。生产 v220/filter 拒绝状态不变，
未提交推送发布，N01–N12 未完成。

## 滤镜候选：彩色半透明对照定位颜色路径差异

新增本机 Chrome 155 默认 Canvas 的 32 项实际像素基线：两个重叠半透明
彩色矩形，sigma 0/.5/2/9，单轴/双轴、sRGB/linearRGB、中心/贴边组合。
记录上下文确认未启用 willReadFrequently；仍使用预乘 BGRA 对照，不改换
此前显示基线。8 项 sigma=0 控制组全部一致；非零的 24 项仅 2 项一致。
按 sigma .5/2/9 分组，最大通道差分别为 5/7/6，alpha 最大差为 1/1/4。

特别是 .5 单轴 sRGB 中心/贴边完全一致，而相应 linearRGB 两项 alpha
完全一致、颜色最大差 3。因此颜色转换/中间量化是需要独立排查的路径，
不能把剩余问题全部归因于大半径缩放；尚未证明具体量化步骤是根因。
10 项已精确样例增加严格断言；其余 22 项只记录差异，未算像素验收通过。
所有 32 项原生输出满足颜色通道不大于 alpha 的预乘约束。

原型当前 126 passed / 1 ignored / 2 filtered；测量测试执行成功不代表
32 项兼容。证据见 `linshi/otter-blur-color-evidence.json`，基线脚本、
浏览器日志和原型日志前缀 `otter-blur-color-`，临时浏览器已关闭。
未修改候选渲染算法或生产来源；生产 v220/filter 拒绝保持，未提交推送
发布，仍未完成实际 WebView2/Job/跨设备验收或 N01–N12。

## 滤镜候选：移除首次降采样前的整图复制

在联合缩放候选上，将首帧像素改为 Cow 借用输入；降采样结果仍拥有自己的
存储，最终重建前显式转换为独立所有权，避免读写输入别名。所有权测试验证
首帧指针等于输入、降采样结果拥有存储、into_owned 不再复制已拥有的存储。
因此移除了首次 width×height×4 字节的输入副本；这不是进程峰值内存测量，
中间帧、输出及其重叠生命周期仍需生产 Job 验证。

24 项输出与上一阶段固定 `.joint.bgra` 全部逐字节一致，原有浏览器误差
没有改善也没有回退。候选库 10 passed，原型 124 passed / 1 ignored /
2 filtered，Clippy 通过。证据见 `linshi/otter-blur-borrow-evidence.json`，
日志前缀 `otter-blur-borrow-`。生产 v220/filter 拒绝状态不变，未提交推送
发布；此项仅修复候选的非必要复制，不代表滤镜或 N01–N12 完成。

## 滤镜候选：双轴联合降采样与一次重建

隔离候选新增 joint_gaussian / scaled_frame：两轴联合降采样，在低分辨率
执行 X/Y Gaussian 后仅重建一次，替代上一候选每个轴各自重建的路径。
独立二维 Python 模型与 Rust 在 24 项中有 23 项逐字节一致；9-xy-center
仍有 8 个通道差 1，不能把模型对照报告为全部精确。模型仅覆盖白色矩形，
不是任意颜色、滤镜图或生产输入的完整 oracle。

24 项大 sigma 浏览器对照的总绝对通道差从 85968 降至 72416，完全一致
仍为 6/24，最大差仍为 8。8.01-xy-edge 从 16364 回退到 16416，
9-xy-edge 从 17264 回退到 17360；不放宽阈值，剩余 18 项仍未通过。
候选库 9 passed；原型 124 passed / 1 ignored / 2 filtered，后两项为
原型绕过生产准入而排除的策略测试，不是生产完整测试。大参数 debug 检查
1 passed，Clippy 通过。保存日志的 1024×1024 双轴 sigma 9/32/1e9/1e38
为 85/53/43/42 ms，不能与上一阶段单轴时间直接比较。

本实现拥有初始整图副本及中间降采样帧，内存模型不同于上一阶段逐行缓冲；
尚未完成生产 Job 峰值内存和预算验证。旧 scaled_gaussian / box_gaussian
实验文件仍保留但不编译。固定像素归档为 `.joint.bgra`，源码、模型、
日志和像素哈希见 `linshi/otter-blur-joint-evidence.json`，日志使用
`otter-blur-joint-` 前缀。生产仍为 v0.2.33.220，filter/fe* 继续拒绝；
未导入第三方生产源码、未提交推送发布，N01–N12 未完成。

## 滤镜候选：浮点边界与逐步降采样

继续核对 [Skia 缩放接口](https://raw.githubusercontent.com/google/skia/main/src/core/SkImageFilterTypes.cpp)
及 blur engine 的尺度约束，以独立 `linshi/blur_scale_model.py` 比较整视口与
内容边界两种一维模型。保留浮点逻辑边界、像素中心与逐步降采样，比直接
扩大 Gaussian 核更接近本机默认 Canvas；不是把 Skia main 当作 Chrome
155 的精确源码。边界 clamp 假设实际使部分样例更差，未保留为默认。

隔离 Rust 新增 scaled_gaussian：sigma>8 改为分轴逐步降采样、低分辨率
有限 Gaussian、带外延的双线性重建；不在重建前把模糊外延裁回输入大小。
工作缓冲是逐行复用的 Vec，不再为大 sigma 的每个轴创建整图暂存。
非有限或超出 f32 来源范围的 sigma 在计算步数前产生透明结果，防止转成
i32::MAX 次循环；这只是 renderer 防护，不能替代未来生产入口的数值拒绝。
原 box_gaussian 保留作实验文件但不再编入当前候选，生产来源重放仍未进行。

24 项大 sigma 浏览器样例完全一致从 1→6，严格断言覆盖这六项；最大
通道差 12→8，总绝对差 250360→85968。仍有局部回退：8.01-x-edge 总差
11360→12640，不能说所有样例改善。余下 18 项、双轴量化/顺序与贴边
采样仍待解决，没有扩大准入或改换基线。

当前候选库 9 passed；原型 124 passed / 1 ignored / 2 filtered；十个大参数
debug/release 无 panic/无拒绝断言通过，Clippy 通过。保存日志的单轴
1024×1024 在 sigma 9/32/1e9/1e20/1e38 下为 81/59/53/60/61 ms，非
生产 Job 或峰值内存验收。证据使用 `otter-blur-scaled-` 前缀，固定候选
像素另存 `.scaled.bgra`，源码/日志/像素哈希见 `otter-blur-scaled-evidence.json`。
生产仍 v220/filter 拒绝状态，未提交推送发布，N01–N12 未完成。

## 滤镜显示基线核验：不可改用较易匹配的读回路径

在进一步调整大 sigma 算法前，重新核验测量链路。同一 Chrome 155、同一
SVG 的 64 个样例分别使用默认 Canvas 与 willReadFrequently:true；记录
getContextAttributes 确认参数生效。59/64 的像素不同，最大通道差 98。
会话的 WebGL renderer 报告 RTX 2080 Ti / ANGLE D3D11；这只是环境记录，
不是 SVG 内部具体算法或 shader 的调用证明。

再对 sigma .5/1/9/100 的单/双轴、中心/贴边共 16 个真实 DOM img，等待
decode 与两帧后获取透明背景页面截图。16/16 与默认 Canvas 预乘 BGRA
逐像素一致，与 readback 路径仅 1/16 一致。默认 Canvas 的全部 64 项也
逐像素重现此前保存的基线。因此后续仍以默认 Canvas/实际页面显示为目标，
不能切换到 readback 来宣称原生兼容；其余原生差异仍然有效。

联网核对 [SkBlurEngine 官方接口](https://raw.githubusercontent.com/google/skia/main/src/core/SkBlurEngine.h)
及 [FilterResult 实现](https://raw.githubusercontent.com/google/skia/main/src/core/SkImageFilterTypes.cpp)：
框架允许后端选择算法、限制最大 sigma，并对较大 sigma 缩放处理。
这是算法方向线索，不是把 main 源码版本等同于本机 Chrome 155，也不是
已证明每项剩余误差都来自缩放。独立有限离散 Gaussian 计算仍不能完全重现
大 sigma 页面像素；例如 sigma 9 贴边双轴最大差仍为 9，不能只换成更精确
的数学核就算完成迁移。下一步要验证实际尺度/采样对齐，而不是继续拟合阈值。

新增 `linshi/verify_blur_display_oracle.py`，三项归档合同实际通过：上下文
明确配对、64 项旧默认基线重现、16 项 DOM 截图逐像素一致。脚本只验证
浏览器证据，不把原生候选判为通过。原始数据与截图使用
`otter-blur-backends-*` / `otter-blur-dom*`，哈希记录在
`otter-blur-display-oracle-evidence.json`；临时浏览器均已关闭。
本轮未修改生产/候选渲染器或版本，没有重跑原生完整库；仍未测实际部署的
Tauri/WebView2、其他 GPU、DPI 或浏览器版本，不能把本机 Chrome 结果泛化。
生产保持 v220/filter 拒绝状态，未提交推送发布，N01–N12 未完成。

## 滤镜候选：大 sigma 的中间裁剪与量化

新增 24 项 Chrome 155 基线：sigma 8.01/9/16/32/64/100，单轴/双轴、
中心/贴边白色图形。旧大 sigma 路径 0/24 完全一致，最大通道差 65；
部分 sigma 64/100 双轴结果被算成全透明，而浏览器仍有非零 alpha。
另对 8.01 到 1e38 的十个有限参数执行实际解码探针，旧 release 路径未
观察到 panic；不将源码整数溢出风险描述为已复现的 debug 崩溃。

新隔离 box_gaussian 候选将三个盒式核合成为分段二次核，通过 0/1/2 阶
前缀矩一次求值；每个输出通道固定八项，不生成 radius 大小的窗口，不在
同轴多遍之间裁剪或量化。窗口宽度用 f64 表示，并依据离散均匀核方差选择
相邻奇数宽度，避免大 sigma 转 i32 再加法。每轴仅在最终写回时量化，
内存为一份图像缓冲及一条轴的三个前缀矩。sigma<=8 的有限卷积保持不变。

独立显式三盒卷积与前缀算法在 sigma 9/16/32、水平/垂直输入上逐像素
一致；这证明候选所选近似核的实现，不证明与浏览器的 Gaussian 核相同。
当前 24 项最大通道差降为 12，总绝对差从 1517020 降为 250360，只有
1/24 全像素一致。特别保留回退：sigma 9 的中心单/双轴总差由 3920/7272
变成 5200/8652，虽然最大差仍为 3；不得描述成所有样例均改善。
全透明错误已在此样例集消除，但大 sigma 与切换边界仍不符合完整像素验收。

候选库 8 项通过，包含 1024×1024 单轴 sigma 9/32/1e9/1e20/1e38 的
数值与有界工作探针；原型全回归 123 passed / 1 ignored / 2 filtered。
十个大参数在 debug/release 两种构建均通过强制无 panic/无拒绝断言，
候选库 Clippy 通过。没有实际 Job 峰值或产品 EXE 验收。

证据使用 `linshi/otter-blur-large-` 前缀，24 组源码/浏览器/旧版/候选像素在
`otter-blur-large-fixtures/`，指标与源码/日志哈希见
`otter-blur-large-evidence.json`。生产仍 v220 且拒绝 filter/fe*，无新版本或
提交推送发布。候选不是全面放行：后续仍需减少近似核误差、处理颜色与变换、
完成输入/引用/Job 预算和完整源码重放后才可接入生产。

## 滤镜候选：模糊核误差隔离与有限卷积

依据 [feGaussianBlur 定义](https://www.w3.org/TR/filter-effects-1/#feGaussianBlurElement)
的可分离归一化 Gaussian 卷积，新增 40 项 Chrome 155 的白色图形样例，先排除
颜色转换影响：sigma .1/.3/.5/.7/1/1.5/2/3/4/8，单轴与双轴，中心与贴边。
原候选使用原库的 IIR/box 分支，0/40 全像素一致，最大通道差 64，总绝对差
539704。不能把此前颜色样例的偏差全部归因于色彩空间。

隔离候选新增 gaussian_blur 模块：每轴 sigma<=8 使用半径 ceil(3*sigma)
的归一化有限卷积，透明边界不重新归一化；较大 sigma 保留原 box 近似，
两个轴独立选择。每遍保持预乘通道，写回 8-bit 后执行下一轴；最多 49 个
tap、一份图像暂存，未将任意大 sigma 直接变成巨大卷积核。既有 tiny-sigma
边界保留，feGaussianBlur 与 feDropShadow 共用入口。不宣称大 sigma 的
box 路径、全部 transform 或 edgeMode 已完成兼容。

新结果为 16/40 完全一致，已增加严格断言；其余 24 项最大通道差均为 1，
全样例总绝对差降为 11340。没有以宽松阈值将其余样例计为通过，残余量化/
核精度差异的具体来源尚未证明。候选库还增加离散 impulse、零轴和透明边界
断言，共 6 项通过。保存日志的 1024×1024 本机 release 测量在 sigma
.3/2/8 下分别为 47/101/277 ms，不是生产 Job 或全尺寸峰值预算验收。
原型全回归 121 passed / 1 ignored / 2 filtered，候选库 Clippy 通过。

源样例/浏览器 BGRA/修改前后 BGRA 在 `linshi/otter-blur-fixtures/`；浏览器
脚本、日志、完整候选 resvg 补丁及哈希证据使用 `otter-blur-` 前缀，见
`otter-blur-evidence.json`。候选仍同时依赖上一轮未生产接入的 morphology
解析修改，不能用 resvg 补丁单独冒充完整可发布来源。生产仍 v220，filter/fe*
准入未开放，没有新版本、提交推送发布或产品 EXE 验收；滤镜颜色、量化、
大参数、引用与 Job 安全、来源重放和实际安装仍待推进。

## 滤镜候选：morphology 半径语义与线性采样

继续验证上一候选未覆盖的边界。Chrome 155 新增 56 个样例：dilate/erode，
中心及贴边多色图形，半径 0/.2/.49/.5/.9/1/1.1/1.9/2/8/40/100 和 0 2、2 0。
上一候选仅 22/56 完全一致，不能由此前半径 2 的通过推断一般半径正确。

隔离 usvg/resvg 候选现修复：不再把零轴半径强制改为 1；按本次浏览器行为对
变换后的半径 round；图像外样本按透明处理；大半径钳制不移动窗口中心。
矩形 min/max 分成水平/垂直两遍，以单调队列维护滑动极值，使每个通道的每个
像素入队/出队至多一次。内存为一份图像缓冲和一个不超过最长轴的队列，
不分配随 radius 面积增长的窗口。既有借用数据原地写回约定保留。

56/56 浏览器样例现全部通过严格全像素断言。新增独立朴素矩形 oracle，覆盖
1..5 的宽高、0..6 的两个整数半径、两种操作，共 2450 组，与优化实现完全
一致。候选 resvg 库共 4 tests passed，包含两个原有大区域裁剪测试。
1024×1024 均匀图的 radius 1/256/100000，在保存日志的复测中分别为
105/100/100 ms；这是本机候选 release 内核测量，不是生产 Job/完整图片链路
性能承诺，也没有测峰值驻留内存。原型全回归 119 passed / 1 ignored /
2 filtered，候选库 Clippy --lib --tests -D warnings 通过。

修改只在 linshi 的 resvg 副本和原型 usvg/src/parser/filter.rs，生产仍 v220。
新补丁/哈希/日志使用 `otter-morphology-` 前缀，56 组 SVG/浏览器像素在
`otter-morphology-fixtures/`，完整记录见 `otter-morphology-evidence.json`。
原型 usvg 现在含新增 filter parser 修改，不能再冒充上一轮已重放的生产来源
快照；后续接入须重新生成完整来源补丁和清单。剩余 blur/颜色差异、缩放及
非均匀 transform 半径、输入语法/有限数值/引用/资源安全和生产来源门仍待完成。
没有开放生产 filter/fe*，未分配新版本、未提交推送发布。

## 滤镜下一阶段：真实像素差异与隔离候选（生产仍 v220）

以 [Filter Effects Module Level 1](https://www.w3.org/TR/filter-effects-1/)
为语义入口，在 Chrome 155 的 SVG img→canvas 链路采集 18 个 64×48 样例，
将读回 RGBA 转为预乘 BGRA 后与隔离 renderer 比较。覆盖 offset/bbox 单位、
flood、merge、composite、color matrix、零/普通/单轴 blur、morphology、
零/用户坐标/primitive 区域以及模糊偏移合并阴影；不是全部滤镜验收。

原 renderer 仅 7/18 全像素一致。两个具体差异已有可重复的候选修复：

- 默认区域的小数边界：浏览器 flood 覆盖 [13,10,43,34)，原生为
  [13,10,42,34)，少一列。group 分配和 filter/primitive 裁剪必须一致地
  对左右边界向外取整，不能分别 floor(origin)+ceil(width)。只改 filter
  内部裁剪不生效，实际 group buffer 仍会先丢掉这一列。
- 半径 2 的 morphology：原实现使用偶数宽窗口，dilate 左/上与 erode 右/下
  出现一像素偏移。候选改为包含中心的奇数窗口，本次两项达到完全一致。
  巨大半径下的窗口截断/边界、非整数半径及运行复杂度尚未验证，不能据此
  宣称 morphology 已完整实现。

候选只存在 `linshi/otter-filter-resvg-probe`，未改 Cargo registry 或生产依赖。
修复后 13/18 全像素一致，已加入一个循环覆盖 13 样例的严格断言测试；
差异报告探针本身不把有差异算作语义通过。其余五项：半透明 flood 最大差 1，
linearRGB 饱和度最大差 2，普通/单轴 blur 最大差分别 18/8，阴影链最大差 14。
保留这些失败关系，不用统一放宽阈值掩盖。候选原型全回归 118 passed /
1 ignored / 2 filtered；过滤掉的 reject_ 是原型不具备的生产准入测试，
不是完整生产回归、真实 Job 预算或来源重放门。

证据：`linshi/otter-filter-browser.{js,log,json}`、`otter-filter-fixtures/` 的
源 SVG/浏览器 BGRA/原版与候选 BGRA、`otter-filter-{before,after}.stdout.log`、
`otter-filter-candidate.patch` 和 `otter-filter-evidence.json`。临时浏览器已关闭。
生产仍拒绝 filter/fe*，版本未分配新 revision，未提交推送发布；下一步继续
解决模糊/颜色差异和半径边界，再完成引用/数值/资源安全门与来源重放，不能直接
把实验副本接入正式仓库。本轮不重复宣称 v220 的生产完整库已在候选依赖上通过。

## v220：paint-order 进入真实 worker 与产品 EXE

生产准入接受 normal 和无重复的 fill/stroke/markers 部分或完整顺序，沿原
renderer 处理祖先继承。svgtypes 对非法值会返回默认顺序而不是错误，因此
入口独立校验有限 token 集合与重复项，避免静默降级；不增加解析器、第三方
补丁或资源预算。style 与 inherit 等全局关键字仍拒绝，不宣称完整 CSS 支持。

Chrome 155 的 24 个显式分层等价样例全部通过：六种排列分别覆盖 path、
整体 opacity、祖先继承及单字形描边。隔离 renderer 的相同 24 项也通过，
随后迁入真实 Job child；生产测试共 **26 passed / 0 failed**，7.39 秒，
包括部分顺序补全、XML 空白、重复/未知 token 拒绝和失败后新请求。比较原生
actual/reference 的全部像素，不是声称 Chrome 与原生字体栅格逐像素相同。

当前完整库 **2839 passed / 0 failed / 51 ignored / 0 filtered**，209.51 秒；
包括 v219 新增的三项单位回归。重新构建实际产品 EXE 后 --smoke-test 退出 0，
新增 8×8 矩形的先描边后填充逐像素检查，输出
`native-svg-paint-order-smoke: isolated=true pixels=true profile=false network=false`。
startup 集成测试另 **7 passed / 0 failed**，21.51 秒。
全目标 Clippy -D warnings、修改文件 rustfmt、3160 文件行数 ratchet、第三方
来源完整性门和 git diff --check 通过；未执行正式发行包或人工视觉验收。

证据位于 linshi：`otter-v220-paint-order-browser.{js,log}`、
`otter-v220-paint-order.{stdout,stderr}.log`、`otter-v220-full.{stdout,stderr}.log`、
`otter-v220-exe.{stdout,stderr}.log`；二进制/源码/日志 SHA-256 绑定在
`otter-v220-paint-order-evidence.json`。内部分配一次 v0.2.33.220，未提交推送
发布。完整 SVG、复杂 bidi、过滤器及其余 N01–N12 迁移仍未完成，依赖安全
Advisory 未放行状态不因本次来源校验通过而改变。

## v219：常用长度单位与当前 EXE 验证

新增 surface_svg_length_units_tests，三个真实 child 测试分别覆盖：
96px/1in/72pt/6pc/25.4mm/2.54cm 与 96 用户单位等价，继承 24px 字号下
2em 与 48 用户单位等价，以及根/嵌套正方形 viewBox 的 50% 与各自用户单位
等价。spacing、spacingAndGlyphs 均执行，比较尺寸、非空结果和全部像素。
**3 passed / 0 failed**，3.22 秒；这是单位等价回归，不是新的浏览器独立 oracle，
未覆盖 ex/x-height、所有非正方形视口或单位换算的极端算术输入。

另以当前 policy 重新构建产品 EXE 并执行 --smoke-test，退出 0，RTL 长度的
独立字形像素检查通过；补齐上一轮 scalar 修复后未重建实际 EXE 的证据缺口。
日志 `linshi/otter-v219-length-units.{stdout,stderr}.log` 和
`otter-v219-length-units-exe.{stdout,stderr}.log`；当前 exe、测试 exe、policy
哈希见 `otter-v219-length-units-evidence.json`。3159 文件行数门通过。
本轮没有再次执行完整库回归，2810 项是新增三个单位测试前的记录；不改写为
已执行的新总数。版本仍 v0.2.33.219，未提交推送发布，整体迁移未完成。

## v219：textLength 数字的 f32 表示边界

真实 child 的新增拒绝用例首先失败：`textLength='1e100'` 被接受，见
`linshi/otter-v219-length-number-before.stdout.log`。policy 原先只检查 f64 有限性，
而 usvg units::convert_length 先转换为 f32，超大值变成 infinity 后不再形成
有效长度约束。现 policy 在该数值转换边界拒绝非有限 scalar，以及非零输入
转换成零的下溢；显式 0、-0 和可表示负长度继续保留原忽略语义。
回归覆盖 ±1e100、3.5e38、1e100px、±1e-100，未修改布局算法或资源上限。

修复后的当前完整库 **2810 passed / 0 failed / 51 ignored / 0 filtered**，
169.83 秒，包含上一轮新增 Application 安装测试；全目标 Clippy、3158 文件
行数门通过。日志 `linshi/otter-v219-length-number-full.{stdout,stderr}.log`，
当前测试 exe/policy 哈希见 `otter-v219-length-number-evidence.json`。
此检查只覆盖单位换算之前的 scalar 表示，不宣称已证明字体相对单位、百分比、
物理单位乘法和全部后续布局算术均不会溢出。上一轮 EXE 冒烟未针对本次 policy
重新构建执行，不能替代新的产品 EXE 验收；版本和来源依赖不变，未提交推送发布。

## v219：新属性的 Application 安装与实际 EXE 冒烟

application_surface_inline_tests 新增 svg-text-length 场景，使用已授权的原
Application 轮询链路，将 spacingAndGlyphs + RTL 的内联 SVG 经 child 解码后
安装。断言原始逻辑尺寸 128×48、占位符清除、安装数/上传数、两次绘制无错误，
继续以 panic 回调禁止图片阶段准备 HTTP Config，并检查所有 owner/窗口收尾。
本轮内联应用 **9 passed / 0 failed**，7.21 秒。

实际产品 smoke 新增 RTL spacing 的独立字形像素比较，不仅检查非空输出。
重新构建后 `otter-native.exe --smoke-test` 退出 0，输出
`native-svg-text-length-smoke: isolated=true pixels=true direction=rtl profile=false network=false`；
该比较通过真实 EXE 的私有 child 入口，不使用 libtest 替身。
同一构建 startup **7 passed**，22.88 秒；全目标 Clippy -D warnings、3158 文件
行数 ratchet 与 diff 检查通过。应用测试验证安装/上传/绘制流程，EXE 像素比较
验证解码结果；没有将其宣称为人工截图验收、完整字体/bidi 或正式发布包验收。

日志 `linshi/otter-v219-ui-{application,startup,exe}.{stdout,stderr}.log`，实际
进程退出码、耗时及三个二进制 SHA-256 见 `otter-v219-ui-evidence.json`。
本轮只跑上述 focused/native gates，没有再次执行完整库；此前 2809 项完整回归
对应新增 Application 测试前的构建，不能把新总数视作已执行。版本仍 v0.2.33.219，
未提交推送发布，N01–N12 及依赖 Advisory 未放行状态不变。

## v219：长度与方向属性进入正式 SVG 准入

生产 policy 已接入 text/tspan/textPath 上的 textLength 与 lengthAdjust；前者
要求可解析且数值有限，后者只接受 spacing/spacingAndGlyphs。direction 接受
ltr/rtl 并可从祖先继承。零/负长度保持 renderer 忽略语义；错误元素、无效
数值/枚举仍整体失败。CSS style 仍不准入，不将本轮描述为 CSS 或完整 bidi 实现。

六个正式测试模块迁入已验证的浏览器像素、组合符号、嵌套缩放、方向/混合文字
以及非法值、同级 4096 约束、嵌套 512 层与后续请求合同。原型 style_rtl 正向
样例明确转换为生产拒绝合同，不再需要运行过滤器。新增 **57 项测试**。
仓库真实 native 完整库回归 **2809 passed / 0 failed / 51 ignored /
0 filtered out**，187.08 秒；全目标 Clippy -D warnings 通过，来源门再次通过。
新测试模块 84–229 有效行，policy 165 行；3158 文件 ratchet 与 diff 检查通过。

证据 `linshi/otter-v219-admitted-full.{stdout,stderr}.log`、
`otter-v219-admitted-full-evidence.json`（当前 exe/policy 哈希），行数报告为
`otter-v219-admitted-lines.json` 和 `otter-v219-admitted-file-lines.json`。
这次验证使用真实 Job/字体代理/IPC，不是实验目录的准入副本。此前 startup
7 项证据属于属性接入前的应用构建，不冒称本轮新 EXE 或新属性 UI 安装验收。
依赖身份/锁与上一接入轮不变，仍未解决全部嵌套 bidi、全部 SVG 语义与最坏资源
输入；Advisory 25 个漏洞 ID 的未放行状态不变，未提交推送发布，N01–N12 未完成。

## v219：固定依赖正式接入，新增属性仍未开放

将已验证重放的 usvg bgafnuep（45 文件）及 roxmltree _87bbt_7（210 文件）
接入仓库 third_party 固定路径。接入前备份位于
`linshi/otter-v219-before-source-import-20261022-102140`；未删除旧源码。
native Cargo patch 指向仓库，来源门核对实际 Cargo resolution 通过；native lock
唯一变化是 roxmltree 0.21.1 从 registry 身份改为已校验本地来源，包版本不变。
生产政策仍拒绝 textLength/lengthAdjust/direction/style，本轮没有开放候选属性。

新增 surface_svg_depth_tests 经真实 child 验证无 textLength 的 32/128/512 层
文字与平铺文字像素相同，以及深层非法 XML 拒绝后普通请求正常。
仓库默认配置完整库回归 **2752 passed / 0 failed / 51 ignored / 0 filtered**，
231.96 秒；编译 Clippy 同期运行，此时间不作为与前次候选的性能比较。
native Clippy --all-targets -D warnings 通过。新增模块各 33–169 有效行、XML
新增测试 39 行、native 新测试 30 行；来源合同新增自动检查，禁止 local-added
模块借上游目录豁免超过 700 行。行数 policy 增加精确 XML 目录后按原历史 commit
重建 baseline，42 个原债务条目及 source commit/tree 完全不变，仅 policy 哈希变化；
exceptions 仍为空。全库 ratchet 3152 文件通过，来源/SBOM 合同 11 项通过。

实际 startup 集成 **7 passed**，33.98 秒，覆盖隐藏窗口、透明合成器设备重建、
WIC 两次绘制和启动参数拒绝；日志 `otter-v219-production-startup.stdout.log`，
应用/启动测试二进制及生产 lock/policy 哈希见 `otter-v219-source-import-evidence.json`。

许可证审计通过：21 个直接 npm、749 个 Rust 包、40 个许可证表达式；双 SBOM
1257 身份一致。联网 Advisory 扫描五锁 1822 包、25 漏洞 ID，**不是安全放行**。
证据 `linshi/otter-v219-production-full.{stdout,stderr}.log` 与
`otter-v219-production-full-evidence.json`；SBOM、license、osv 使用
`otter-v219-source-import-*` 前缀，新增模块行数见 `otter-v219-added-module-lines.json`。
依赖已从实验路径迁入仓库，不是正式发布，不代表新属性或 N01–N12 全面完成。

## v219：正式来源校验器支持显式新增模块

本轮开始正式内部迭代 v0.2.33.219，仅推进依赖来源门，不替换生产渲染依赖。
third-party-sources.cjs 支持 manifest schema 2：upstream 记录保留有效原始哈希，
local-added 必须显式设 upstreamSha256=null，且新增路径与 added 清单精确一致。
modified 只包含修改的上游文件；旧清单继续兼容，不从缺失字段猜测新增来源。
拒绝未知/null schema、伪造 origin/hash、遗漏/重复/混入 modified 的新增清单。

来源门 7 项测试及直接依赖 SBOM 3 项测试通过；真实生产来源及 Cargo resolution
门通过，生产 usvg/tiny-skia 身份不变。以 verifyPackage 单独核验候选 usvg
bgafnuep 和 XML _87bbt_7 快照也通过，这不代表生产 Cargo 已绑定它们。
依赖安全合同、版本元数据校验、Node 语法检查和有效代码行 ratchet 通过。
没有更新第三方目录、native policy 或生产 lock，没有提交推送发布。

## 512 层复现的组合修复与完整候选回归

仅调试本次新建测试进程树，捕获 stack-overflow 事件并用本地 PDB 解析栈内候选
地址。第一处指向 roxmltree 0.21.1 tokenizer 的 parse_element/parse_content；
源码确认两者随 XML 元素深度互相递归。这里采集的是栈内地址，不是完整 unwind。
独立 XML 副本改为 start-tag 解析加迭代 content 深度计数，保留 XmlEvents 的
标签匹配、属性和文档结构校验。替换后再次捕获，故障路径转到 usvg 的
collect_text_chunks_impl/长度解析；与先前叶节点提取 bgafnuep 快照组合后，
原始 32/128/512 层测试均通过，像素与单层目标一致，后续请求也正常。
512 层 debug 解码 89 ms；未扩大线程栈、Job 预算或 deadline。

组合配置完整 native 库回归 **2806 passed / 0 failed / 51 ignored /
1 filtered out**，159.21 秒；仍只主动排除未准入 style_rtl 正向样例。
副本 Clippy --lib --tests -D warnings 通过。XML 副本原有 173+22+91+10 项
测试及 32 项 doc-tests 通过；显式注册的新增两个深层测试另通过，覆盖
512/4096 层结构、文本/兄弟保留和不匹配/截断拒绝。上游 lib-test 有既有
missing_docs 宏警告，不把它描述为无警告的 XML 全目标 Clippy。

当前 usvg 原型重新保留叶节点提取，与 bgafnuep 对应；此前“单独提取失败”
仍成立，不能据此否定其在组合中的作用。XML 工作副本为
`linshi/otter-v219-roxmltree-probe`。独立 Cargo.lock 为离线测试将 memchr
2.7.6 调到已缓存的 2.8.3；仅副本 native lock 改为 local roxmltree，生产锁未改。
XML 补丁从固定归档重新生成并重放到 `linshi/otter-v219-xml-replay-_87bbt_7`，
210 文件逐字节相同：3 修改（tokenizer、测试注册 manifest、独立测试 lock），
1 新增测试。此为实验来源证明，未接正式来源门；本次 native 实际链接工作副本，
并非宣称从重放目录重新构建。

证据：`linshi/otter-v219-debug-stack.json`、`otter-v219-debug-stack-symbols.log`、
`otter-v219-debug-second-symbols.log`，原始/第二次失败继续保留；组合 focused
`otter-v219-combined-stack.{stdout,stderr}.log`，完整回归
`otter-v219-combined-stack-full.{stdout,stderr}.log`，exe 身份
`otter-v219-combined-stack-evidence.json`；XML 测试 `otter-v219-xml-tests.log`
和 `otter-v219-xml-deep-tests.log`，重放目录内有完整 patch/manifest。
只证明已复现的 512 层 SVG 故障得到修复，不宣称任意深度、其他 usvg 递归或
全部 XML 安全问题完成。生产版本/入口仍未改，没有提交推送发布。

## 未解决：深层文字约束使候选 child 栈溢出

新增副本测试 candidate_admission_nested_length_workload_and_followup，将相同
80px spacing 目标嵌套 32/128/512 层，并与单层目标逐像素比较。32/128 层通过，
512 层返回 `Surface SVG decode: 管道已结束。 (0x8007006D)`，测试失败。
Windows Application 事件 1000/1001 对应候选 exe，异常码 `0xc00000fd`，
明确是子进程栈溢出；llvm-symbolizer 将故障 RVA 定位到 `_alloca_probe`，
尚未获得足够调用栈来确定具体递归函数。不能将管道错误视为正常策略拒绝。

尝试将文字叶节点 span 构造从递归收集函数提取为独立不内联模块后，原型
92 项回归与 Clippy 通过，但同一 512 层 native 测试仍失败，故该假设未证实。
未把尝试当作修复：原型已恢复到 32zk0_tu 对应的 parser/text.rs，抽取文件
保留为 linshi/otter-v219-unsuccessful-text_chunk_leaf.rs，不进入候选来源树。
失败试验的重放快照 bgafnuep 和 exe 仅作诊断证据，不作为新准入候选。

初次失败 `linshi/otter-v219-nested-native.{stdout,stderr}.log`，系统事件
`otter-v219-nested-crash-events.json`；未成功的提取实验日志名称为
`otter-v219-nested-fixed.{stdout,stderr}.log`（文件名不代表结果通过）。
新增失败测试保留；计划中的最新完整库回归尚未运行，先解决此故障。
生产仍未接入候选；尚不能确定问题只影响 textLength，需继续定位既有深层文本路径。

## 候选长度范围查询索引

原型 length_adjust 不再为每个约束扫描全部簇以筛选所属范围：预建源 UTF-8
字节索引，以两次 partition_point 取范围，再按原簇索引排序恢复视觉顺序。
因此 bidi 视觉顺序不被源顺序替代；位置调整、嵌套约束与后续位置平移不变。
仅消除范围筛选的重复全扫描，后续位置平移和嵌套遍历仍可能二次增长。

原型 release 回归 92 passed / 1 ignored / 2 filtered，usvg Clippy -D warnings
通过；补丁重新重放至 `linshi/otter-v219-source-replay-32zk0_tu`，40 个上游文件、
8 个修改、4 个新增全部逐字节验证。副本 native 改用该新快照后，新属性与压力
测试 **55 passed / 0 failed**，13.64 秒；未重跑完整库回归。
32/512/4096 同级约束实际 child 耗时 79/173/4128 ms。4096 样本较前次 4488 ms
略低，但这是单次跨运行样本，不宣称统计显著提速或性能问题已解决。

原型日志 `linshi/otter-v219-index-after.{stdout,stderr}.log`，native 日志
`otter-v219-index-native.{stdout,stderr}.log`，当前二进制与快照身份见
`otter-v219-index-native-evidence.json`。生产依赖和准入入口未变。

## 隔离准入副本的真实 child 大输入验证

在副本 candidate_policy_tests 增加一个有界压力测试，经原 decode 入口依次输入
32/512/4096 个同级 textLength 约束（64/1024/8192 字符），实际耗时
101/183/4488 ms；最大输入 135303 字节。检查 300×80 输出和非透明像素，
每组之后再解码普通红色矩形并精确断言 BGRA，三次后续请求均成功。
本次是 debug 构建的真实 IPC/字体代理/Job child，不是原型进程内渲染。

测试 **1 passed / 0 failed**，5.01 秒；仍复用原 15 秒 deadline 和 child
强制核验的至多 128 MiB Job/进程限制，没有修改限额，没有测量峰值内存。
测试仅证明这些同级输入在既有预算下完成，不证明最坏嵌套、最大 XML 输入
或完整语义；非空像素检查不是大输入逐像素正确性的独立 oracle。
证据 `linshi/otter-v219-admission-workload.{stdout,stderr}.log`，当前测试文件及
exe 哈希见 `otter-v219-admission-workload-evidence.json`。之前完整库回归对应
增加本测试前的二进制，不将本轮单项结果改写为新的完整回归计数。
生产源码、依赖和版本不变，未提交推送发布。

## 隔离准入副本的完整库回归

在末次 rustfmt 后重新构建隔离 native 副本，执行整个库测试而非只筛选新增
用例：**2804 passed / 0 failed / 51 ignored / 1 filtered out**，148.61 秒。
唯一主动排除项仍是未准入 CSS style 的正向 style_rtl 样例；51 项既有 ignored
未执行，不将其计入通过。此次覆盖原有库测试与新增 54 项真实 child 用例，
关闭了上一轮格式化后未重新生成测试二进制的证据缺口。

日志：`linshi/otter-v219-admission-full.{stdout,stderr}.log`；退出码、耗时及
重建 exe SHA-256：`linshi/otter-v219-admission-full-evidence.json`。
生产第三方来源门再次通过，usvg/tiny-skia 身份保持不变；git diff --check
通过（仅既有行尾转换警告）。没有修改生产准入白名单，也没有提交、推送或发布。
这仍是候选副本回归，不代替完整 SVG 语义、资源上界和正式来源门接入。

## 新属性经真实 IPC/字体代理/Job child 的隔离准入验证

在 `linshi/otter-v219-native-admission-probe/native` 建立源码副本，候选 usvg
仍使用已验证的 q0rk_xao 重放快照。仅副本 policy 增加 text/tspan/textPath 的
有限 textLength、合法 lengthAdjust，以及 ltr/rtl direction；生产白名单未改。
复制范围校验确认 1513 个原文件逐字节未改，修改仅 policy 与测试注册两文件，
新增六个测试文件。IPC、字体代理、Job 管理、Ready/grant 与解码器实现没有替身。

将浏览器基线接入副本现有 Image::from_surface 测试入口，实际跨进程渲染。
**54 passed / 0 failed**，9.27 秒：覆盖基本长度、嵌套缩放、组合符号、方向与
混合文字，另核验非法数值/枚举/错误元素/仍禁用的 style 被拒绝，以及零/负
长度保持原绘制结果。一个 style-rtl 正向样例明确不在本次准入范围，运行时
跳过，并由反向拒绝测试保护；不能称所有复制用例均准入。适配 Pixels 容器时
仅改测试 zip 为显式 iter，不改变像素断言。副本 lib/tests Clippy -D warnings
通过，新增测试及修改 policy 使用 rustfmt。

证据 `linshi/otter-v219-admission-child.{stdout,stderr}.log`，执行耗时/退出码/
exe 哈希见 `otter-v219-admission-child-evidence.json`，源码差异及文件哈希见
`otter-v219-admission-copy-evidence.json`。测试二进制在末次测试源码格式化前
构建（随后 Clippy 校验格式化后的源码），没有把复制目录替换生产工作树。
这是新增属性经真实机制的候选验证，仍不是正式来源门接入、完整 SVG 语义验收
或发布批准；生产保持 v0.2.33.218，未提交推送发布。

## 候选渲染器进入真实 native 回归（入口仍未开放）

通过仅本次命令的 Cargo patch 覆盖，将真实 native crate 绑定到已重放验证的
`linshi/otter-v219-source-replay-q0rk_xao/source`，不是修改 Cargo.toml 或 registry。
Cargo metadata 检查实际 manifest_path，Cargo.lock 保持不变；构建产物独立放在
`linshi/otter-v219-native-candidate-target`，不覆盖原 v218 target。

真实 native 库测试 **2750 passed / 0 failed / 51 ignored**，138.94 秒；startup
集成 **7 passed**，15.78 秒，覆盖隐藏窗口绘制销毁、透明窗口合成器/设备重建、
WIC 两次绘制及启动拒绝参数。候选配置 native Clippy --all-targets -D warnings
通过。构建后再次比较候选快照与原型的 44 文件，全部字节一致；默认生产来源门
复核通过，usvg/tiny-skia 生产身份未变。

证据 `linshi/otter-v219-native-candidate-{full,startup}.{stdout,stderr}.log`，
实际 source 路径 `otter-v219-native-candidate-source.json`，二进制/lock/policy/
候选 manifest 哈希及汇总 `otter-v219-native-candidate-evidence.json`。
这证明候选依赖对现有原生功能的回归兼容，不是新增语义的 production child
验收：surface_svg_policy 白名单未改，textLength/lengthAdjust/direction 仍拒绝。
候选不符合正式来源门发布条件，未替换生产依赖、未提交推送发布，版本仍 v0.2.33.218。

## 混合文字的段落基础方向与导出修复

先通过 Windows GlyphTypeface 核对：本机 Consolas 不含所测希伯来/阿拉伯字形，
Arial 含所测字形。隔离 decode/writer 测试显式加载 Consolas 与 Arial，不扫描
系统字体，不改生产字体代理；本轮混合测试指定 Arial 32px。
Chrome 六组拉丁/希伯来混合文本的 LTR/RTL × 自然长度/spacing/spacingAndGlyphs
均通过浏览器内部精确像素重建。原型最初 LTR 三项通过、RTL 三项失败，证据
`linshi/otter-v219-bidi-before.log`。

根因是 shaping 的 BidiInfo 基础 level 固定 LTR。原型 Text 现保留段落方向，
从 text 根继承的 direction 传入正常整形及字体回退的重新整形路径。writer 保留
段落 direction，并将内部物理锚点恢复为该方向下的逻辑锚点；补自然/spacing/
缩放三个混合文字往返，避免只修显示而丢失导出语义。六项浏览器基线和往返均通过。

release 常规回归 **92 passed / 1 ignored / 2 filtered**，usvg Clippy/rustfmt
通过。独占 Job 探针本轮未重跑，policy 拒绝测试跳过。浏览器证据
`linshi/otter-v219-bidi-browser.log`，会话已关闭；回归日志
`otter-v219-bidi-after.{stdout,stderr}.log`。最新来源重放目录
`linshi/otter-v219-source-replay-q0rk_xao/`（末次只更正代码注释，行为验证二进制未变）。
尚未处理嵌套 unicode-bidi embedding/override，也不把 Arial 指定字体验证当作
完整字体回退或阿拉伯连字验收。生产版本/入口保持 v0.2.33.218。

## 显式 RTL 的逻辑锚点修复

Chrome 155 十二项基线均通过浏览器内部精确像素重建：自然长度、spacing、
spacingAndGlyphs 各配 start/middle/end，以及方向继承、LTR 覆盖、行内样式。
初始九项原型测试为 3 passed / 6 failed，start/end 未按 direction=rtl 反转；
该缺陷也影响无长度约束文字。现 parser 将逻辑锚点归一为物理锚点，writer
写出归一后的值，避免回读时二次反转。另验证三种锚点的有/无前缀往返。

这不是完整 bidi 实现：unicode-bidi 的 embedding/override、显式方向对混合
文字重排的影响、字体回退仍待验证。浏览器证据
`linshi/otter-v219-direction-inherit-browser.log`，失败日志
`otter-v219-direction-before.log`；会话已关闭。
release 常规回归 **85 passed / 1 ignored / 2 filtered**，独占 Job 探针本轮
未重跑，policy 拒绝测试仍跳过。usvg Clippy/rustfmt 通过；完整回归日志
`linshi/otter-v219-direction-after.{stdout,stderr}.log`。当前候选补丁已重新从
官方归档重放，目录 `linshi/otter-v219-source-replay-_l3t65i4/`。
生产版本/入口保持 v0.2.33.218，未提交推送发布。

## 组合附加符与单排版簇约束修复

新增 Chrome 155 实测组合附加符、预组字符/分解字符、嵌套约束与单排版簇。
八个场景中七个通过浏览器内部“按完整排版簇重建”的精确像素校验，已纳入
原型回归；单簇 spacingAndGlyphs 重建仍有 32 个通道不同，尚未作为可靠 oracle。
日志 `linshi/otter-v219-single-child-browser.log`，浏览器会话已关闭。

实际复现父缩放/单簇 spacing 子范围的错误：count<2 提前 continue，漏记已解析
状态，导致父级再次拉伸子字形。现无间隙时 step=0，仍完成 resolved 状态更新。
失败证据 `linshi/otter-v219-single-child-before.log`。七项新增测试均通过，
release 常规回归 **72 passed / 1 ignored / 2 filtered**；ignored 为独占 OS Job
探针（本轮未重跑），filtered 为生产 policy 拒绝测试。usvg Clippy/rustfmt 通过。
新源码补丁重新从官方归档重放成功：`linshi/otter-v219-source-replay-vjrjzn6h/`，
不能再把上一份 replay 的字节身份称为当前候选身份。回归日志
`linshi/otter-v219-clusters-after.{stdout,stderr}.log`。

这些测试不等于全部复杂 shaping/bidi/字体回退已验；生产入口和 v0.2.33.218 不变。

## 新增模块的隔离来源清单与补丁重放

新增实验验证器 `linshi/otter-v215-context-prototype/verify_length_sources.py`，
从固定 SHA-256 的官方 usvg 0.48.1 crate 读取 40 个上游文件，显式允许 8 个
修改文件和 4 个本地新增模块。新增模块记录 origin=local-added、upstreamSha256=null，
不冒充上游原文件；强制 UTF-8 无 BOM、终止换行和新增文件不超过 700 物理行
（这是保守上限，不替代仓库语言感知行数门）。仅明确排除内容精确匹配的 .cargo-ok。

实际生成统一补丁，并在新隔离目录从原始归档执行 git apply --check / apply，
随后比较完整文件清单及全部字节。第一次发现本机 Git 自动 CRLF 转换导致不同，
改为该次命令显式 core.autocrlf=false/core.eol=lf 后通过，不改用户 Git 配置。
成功产物 `linshi/otter-v219-source-replay-iz03wwbv/{fix.patch,source-manifest.json}`，
其中 source/ 为重放结果；manifest schemaVersion=2、status=experimental-not-production。

八项测试通过，覆盖额外/遗漏文件、伪造上游归属、未授权/虚报修改、路径越界、
BOM/非法 UTF-8/缺少终止换行及超长新增文件。验证器不会修改 registry 或生产依赖，
不会删除旧证据目录。生产来源门本轮复核通过，usvg/tiny-skia 身份仍与 v218 一致。
该实验 schema 尚未接入生产 third-party-sources.cjs 或 CI，不把重放成功当作来源门
已发布，也没有替代复杂文字语义、完整性/许可证审核或真实 child 集成验收。

## release 与诊断 OS Job 限额实测

原型首次 release 构建完成；正常回归 **65 passed / 1 ignored / 2 filtered**。
ignored 为必须独占进程运行的新增 OS Job 探针，filtered 为生产 policy 拒绝测试。
另在专用隐藏进程显式运行 Job 探针，设置并查询核对 process/job 128 MiB、
ActiveProcessLimit=1、kill-on-close，外层持有进程句柄执行 15 秒 deadline。
Job 句柄刻意保留至该专用进程退出，避免测试结果落盘前自杀；不用于一般测试进程。

探针覆盖 32/128/512/1024/4096 个约束，并实际执行 parse/layout、300×80 光栅
渲染（确认非空）及 preserve_text 导出。4096 约束/8192 字形本次记录为
166.339 / 2.279 / 5.819 ms；系统查询峰值进程与 Job 内存均为 22687744 bytes
（约 21.6 MiB），专用进程 wall=319 ms、exit=0。不是工作集轮询的估算值。

证据 `linshi/otter-v219-release-render-job.{stdout,stderr}.log`，进程耗时、
上限和 exe 哈希见 `otter-v219-release-render-job-evidence.json`。新增探针位于
原型 `src/length_job_tests.rs`，不修改生产进程管理器。这里的 Job 与生产采用
相同限额种类，但未走真实 IPC、生产 policy、字体代理与 Ready/grant 协议；
不能称生产 child 验收通过，更不代表全部复杂输入均满足预算。生产仍为 v0.2.33.218。

## chunk 包围盒重复计算优化与 4096 约束探针

临时阶段计时确认：1024 约束的 prepare 约 7 ms、length adjust 约 108 ms，
span 阶段约 1986 ms；日志 `linshi/otter-v219-bbox-profile.stderr.log`。计时
插桩已移除。convert_span 原来为每个样式重新构建完整 chunk 的度量包围盒；
原型现使用 chunk 局部单条缓存，clusters 固定且 text_ts 相同时才命中，
变换改变即重算。没有将范围缩成 span bbox，也没有无界缓存或额外 glyph 副本。

同机 debug 记录：1024 约束 parse/layout 2133.748 → 290.671 ms，512 约束
529.722 → 82.304 ms。增加 4096 约束/8192 字形、135303 字节输入，首次
parse/layout 4707.459 ms，writer 39.135 ms。仍有超线性成本，非 release 保证。

以关闭缓存时的渲染输出记录五组像素哈希，保护渐变/描边、上下标基线、竖排、
旋转与隐藏文字；启用缓存后匹配。全部适用原型测试 **65 passed**，两项生产
policy 拒绝测试仍跳过，usvg Clippy/rustfmt 通过。相关日志
`linshi/otter-v219-bbox-after.{stdout,stderr}.log` 和扩大规模后的
`otter-v219-bbox-4096.{stdout,stderr}.log`。本轮仍未证明 Job 峰值内存和 deadline，
生产版本/入口保持 v0.2.33.218，未提交推送发布。

## 长度约束规模探针与重复整形优化（隔离原型）

新增 32/128/512/1024 个兄弟约束的实际 parse/layout/preserve_text 探针，字体为
Consolas，每约束两个字形；不是独立数学模型。1024 约束输入 33927 字节。
本机 debug 同轮前后记录：512 约束解析布局 1094.219 → 538.057 ms；1024
约束 3948.316 → 2152.028 ms。此为单次观测而非 release/perf 承诺；writer
1024 约束约 9 ms，不是该样本的主要热点。

检查真实 process_chunk 发现仅长度/绘制属性不同的 span 也反复整形完整 chunk。
原型在字体、字号、small_caps、kerning、optical sizing 全同且未发生异参覆盖时
复用已有整形；一旦有异参覆盖即不再跳过后续 span。不增 glyph 副本缓存。
这是 legacy layout.rs 的局部热路径修复，未新增整形职责或进行无关结构重写。
新增后续字号/特性 run 与大量等长片段像素回归保护，全部适用原型测试
**64 passed / 0 failed**；两项 policy 拒绝测试跳过，usvg Clippy/rustfmt 通过。

原始日志 `linshi/otter-v219-workload-{before,after}.{stdout,stderr}.log`。
剩余耗时仍明显超线性，范围扫描、片段转换等路径还需定位；没有测量真实 Job
峰值内存、15 秒 deadline 或 release 构建，不宣称预算达标。生产版本/入口不变。

## 嵌套缩放分段修复：原型 61 项通过

追加 Chrome 实测连续文字、嵌套标签、显式零 dx、显式零 rotate 的分段差异，
前三个新增场景均通过浏览器内部精确像素重建校验。现有十项可信组合 oracle
全部通过，上一节保留的三项失败已修复；三层组合的 oracle 精度问题仍未解决。
最新浏览器证据 `linshi/otter-v219-boundaries-browser.log`，会话已关闭。

新模块 `text/length_fragments.rs` 使用实际 clusters 的来源 span、长度约束 owner、
chunk、逻辑顺序及显式定位边界建立分组。父级移动已解析片段时保留其内部距离；
spacing 子范围仍逐 glyph 调整，不把任意子约束全部当作刚体。解析时额外保留
显式零定位的字节边界；旋转属性只在来源文字叶子级解析一次，不在逐字符热循环
重复分配。该分组尚未证明覆盖字体回退、复杂 shaping 或全部 bidi 片段。

新增四组嵌套缩放 writer 往返，先复现零 dx 被省略后布局改变，再通过保留叶子
分段边界修复。当前全部适用原型测试 **61 passed / 0 failed**，仍跳过两项生产
policy 拒绝测试。usvg Clippy -D warnings、修改文件 rustfmt 通过。日志
`linshi/otter-v219-fragment-writer-before.log`、`otter-v219-fragments-after.log`，
摘要和文件哈希 `otter-v219-fragments-evidence.json`。来源、资源预算和真实 Job
接入仍未完成，生产仍为 v0.2.33.218，未提交推送发布。

## 嵌套缩放新反例：原型当前仍有三项失败

新增 Chrome 155 实测八组 spacing/spacingAndGlyphs 组合，使用 DOM 字符位置与
advance 重建独立 glyph，再在浏览器内进行 image/canvas 全像素比较。七组等价；
三层嵌套一组有 56 个通道不同，尚不能作为精确像素 oracle，未自动加入断言。
浏览器脚本与日志：`linshi/otter-v219-combined-browser.js/.log`。

七组可信 oracle 加入启用的原型测试后，先得到 3 passed / 4 failed。修复父级
再次拉伸已解析子级字形的问题后，变为 4 passed / 3 failed；父级缩放搭配子级
spacing 已通过。父子均缩放的普通、middle、压缩三项仍失败，涉及字形片段的
内部位置保持，需要真实整形分组，不能把任意来源子范围粗略视作刚体。
W3C SVG2 text layout 与 Chromium SVG layout 实现已联网核查；没有复制其源码。

当前全部适用原型测试为 **54 passed / 3 failed**，两项 policy 拒绝测试仍跳过。
三项失败保持启用，不能沿用上一节 50 passed 作为当前全绿结论。原型 usvg
Clippy -D warnings 与修改文件 rustfmt 通过。前后日志为
`linshi/otter-v219-combined-before.log`、`otter-v219-combined-after.log`；生成器
`otter-v219-generate-combined-tests.py` 只接受浏览器自身精确等价的像素基线。
生产版本及入口不变，仍未接入这些实验实现。

## 导出路径定义身份与重复 ID 修复（隔离原型）

新增导出局部 `writer/text_path_ids.rs`：按同一不可变树中的 TextPath Arc 身份
登记一次路径定义，定义与 href 使用同一映射。不同引用不按来源 ID 盲目合并，
保留各自解析后的几何及 startOffset；新 ID 避开节点、路径引用和资源 ID，
并由现有 writer 统一添加 id_prefix。不改动公开 WriteOptions 或原 AST 身份。
非 preserve_text 分支不建立路径表。

先加入导出 ID 唯一断言，得到三项 writer 失败，再完成修复。新增共享路径、
可见源路径、不同 offset、变换几何、普通节点/gradient ID 冲突场景；七项 writer
测试覆盖 19 组输入、38 次有/无前缀往返。原型全部可用测试 **50 passed**，
两项生产 policy 拒绝测试明确跳过。直接对修改后的 usvg 与原型 lib 分别执行
Clippy -D warnings 通过；修改文件 rustfmt 通过。

证据：`linshi/otter-v219-pathids-before.log`、`otter-v219-pathids-after.log` 和
`otter-v219-pathids-evidence.json`。上一节记录的重复路径定义缺口已在原型修复，
不再是当前待办；复杂 shaping/组合语义、资源预算、新模块来源门及真实 Job
接入仍待完成。生产继续保持 v0.2.33.218，未提交推送发布。

## 长度约束导出回读与曲线路径状态修复（隔离原型）

原型 writer 新增独立 `writer/length_text.rs`，按元素范围重建长度约束，
不再将父约束重复写到每个样式 span。16 组输入分别使用无前缀和 `rt_`
前缀，32 次往返检查范围/目标/adjust/parent、chunk 路径状态及完整像素一致。
覆盖同范围嵌套、同值兄弟、跨绝对位置、缩放、路径切换、UTF-8、空约束和基线偏移。
这里保留的是布局约束，不承诺原 DOM 标签、ID 或 selector 身份。

曲线多 chunk 反例先实际失败：解析器退出嵌套 tspan 时无条件恢复 Linear，
导致回读后第二块丢失 Path 状态。原型现恢复进入子元素前的 flow；失败日志为
`linshi/otter-v219-writer-curved-before.log`。空约束子树也从范围索引中移除，
防止空 parent/sibling 消耗身份。新增逻辑未复制到生产依赖。

本次五项 writer 测试、七项范围测试、八项嵌套 spacing、18 项长度像素和五项
既有路径渲染测试全部通过，共 43 项；原型 Clippy --lib -D warnings 与修改文件
rustfmt 通过。日志 `linshi/otter-v219-writer-*.log`，摘要及文件哈希见
`linshi/otter-v219-writer-evidence.json`。拒绝测试跳过，未执行真实 Job/full native。

尚有导出器对多 chunk 重复输出同 ID 路径定义的问题，不能宣称 writer 完整收口。
复杂 shaping/bidi、组合语义、异常数值、资源预算与新增模块来源门仍待验证。
生产版本保持 v0.2.33.218，textLength/lengthAdjust 入口继续拒绝，未提交推送发布。

## textLength 已接入隔离 renderer（v219 实验，生产仍为 v218）

元素约束求解已不再只是独立坐标模型。原型先整形含长度约束文本的各 chunks，
再以真实 glyph advance/dx 求解元素范围，将相对位移与字形缩放分别交给原
水平/路径定位。没有长度约束的文字继续逐 chunk 处理，不提前保留所有 chunks。
旧的按样式 span 独立调整逻辑已从原型移除，零目标不再落回旧的错误分支。

实际原型 renderer 的原 18 项像素测试由 9 passed / 9 failed 改为 **18 passed**；
另八项嵌套/多层/同值兄弟、绝对位置、dx、尾随场景也新增真实渲染像素断言并
全部通过。六项来源范围测试、五项既有路径文字渲染测试保持通过。四份日志
`linshi/otter-v219-integrated-{textlength_cases,length_spacing_tests,
length_range_tests,surface_svg_text_paths}.log`；direct renderer 不含 Otter policy，
因此路径拒绝测试明确跳过，不能称真实 Job child 通过。

新增隔离模块 `usvg/src/text/length_adjust.rs`（格式化后 156 物理行）负责求解，
避免继续把新职责堆入 layout.rs。直接子范围预先索引，逐 glyph 查找改为二分；
每约束仍扫描 clusters，复杂度/峰值尚需大样本验证。最终转换为 f32 后检查
位移、缩放、width/advance 有限性；失败返回布局 None，不冒充已具备全 SVG
请求级错误传播。原型 Clippy --lib -D warnings 与显式 rustfmt 通过。

尚未开放生产：writer 保持约束层级的回读、复杂 shaping/bidi、嵌套
spacingAndGlyphs 与路径边界组合、异常数值的父侧报告、真实 Job 预算/全量
仍需完成。新手写库模块也须按实际来源更新补丁清单和完整性门，不能伪装成
官方归档原有文件或自动享受上游源码豁免。生产源码身份与 v0.2.33.218 不变，
不把本节 26 项像素通过解释为全部文字长度语义已完成。

## textLength 元素范围与 spacing 顺序原型（v218 后续，未进入生产）

生产仍为 v0.2.33.218。本轮在隔离 usvg 原型中新增元素级 TextLengthRange，
记录保留文本的全局 UTF-8 字节范围、目标长度、调整方式及父约束索引。解析器
在进入/离开来源元素时登记一次，不再依赖样式 span 推断来源身份。新增数据
尚未替换旧布局与 writer；当前仍保留旧 TextSpan 字段，不能据此宣称完整迁移。

六项解析测试通过：跨样式、跨绝对定位 chunk、嵌套/同值兄弟、隐藏内容与
UTF-8 范围、非正目标、textPath 边界。日志
`linshi/otter-v219-length_range_tests.log`。v219 是实验编号，没有递增生产版本。

另完成独立 spacing 顺序探针，消费真实原型解析出的约束，八项坐标测试通过。
基线来自 Chrome 155 新测的八组嵌套/多层/同值兄弟、绝对位置、dx、尾随字符
场景；浏览器 DOM 坐标重新绘制为独立 glyph 的 image→canvas 像素也八组一致。
特意使用 Consolas 16px，其 advance=8.796875 可精确表示为 1/64px，避免把
先前 20px DOM/图片度量舍入差异误作布局误差。完整来源与结果：
`linshi/otter-v219-length-ranges-browser.js/.log`、
`linshi/otter-v219-length_spacing_tests.log`。

[SVG 2 布局算法](https://www.w3.org/TR/SVG2/text.html#TextLayoutAlgorithm)与实测
均要求先处理 dx、再调整长度、最后应用绝对 x 重置。子约束先于父约束处理。
不能把“resolved child 算作一个单位”误解为整段刚性平移：实测嵌套案例的
父级调整会在子级首 glyph 后推进 shift。探针按这个顺序复现八组坐标，不把
自拟的刚性子段假设当作规范或浏览器行为。

验证边界：这八项是独立数学探针，不是已接入 renderer 的像素验证。实际旧
renderer 对原 18 项 textLength 用例仍为 **9 passed / 9 failed**，日志
`linshi/otter-v219-renderer-before-integration.log`。v218 集成时没有带入此前
实验的 length-adjust 补丁；旧实验保存在
`linshi/otter-v215-context-prototype/textlength-layout-experimental-v218.rs`，
不能将旧实验的 16/18 通过率误记为当前原型或生产状态。

下一步须将约束求解接入真实 glyph 数据、路径/锚定与绘制，完成
spacingAndGlyphs、bidi/ligature、writer 保结构回读及预算检查。当前独立探针
仍逐约束/字符查询子范围，尚未建立生产级范围索引；不能把测试中的小输入
线性外推为复杂度保证。原型 cargo check 与显式 rustfmt 通过；本轮生产来源
身份未改变，未重复跑无源码变化的生产全量或声称新增功能通过。

## v218 重叠 glyph 绘制修复

最终完整库测试 **2750 passed / 51 ignored（142.27 秒）**，真实 EXE startup
**7 passed（15.54 秒）**。Clippy all-targets -D warnings、fmt、3151 文件严格
行数、8 项来源/SBOM 测试、许可证审计与 diff 检查通过；双 SBOM 核对 1257
个身份，旧 v217 SBOM 被实际拒绝。联网 OSV advisory 仍为 5 锁/1822 包/
25 漏洞 ID，不是发布放行。完整日志与源码/二进制身份见
`linshi/otter-v218-evidence.json`；N01–N12 仍未完成，无提交推送发布。

已确认上节调查的压缩文字差异来自 flatten 把连续 glyph outline 合成同一条
路径，而不是排版坐标。原型改为逐 glyph 提交 Path 后，原来 18 个通道字节、
最大 48 的差异变为零；18 项 textLength 原型只剩两项跨样式约束失败。
生产只接入 flatten 修复，没有带入 textLength 实验布局补丁，属性仍未开放。

新增 Chrome 155 五组等价对照：普通 fill、fill-opacity=.5、纯 stroke、
fill+stroke、evenodd。原文和像素结果保存于
`linshi/otter-v218-overlap-browser.js/.log`。回归使用隐形前缀保留实际字体
advance，不硬编码不同字体路径的舍入值；实际 child 另覆盖外层 opacity=.5。
修复前真实 child 专项失败，日志 `linshi/otter-v218-overlap-before.stdout.log`；
修复后 SVG 34 项专项通过（21.66 秒），包含新重叠测试和 4096 glyph 预算测试。

每 glyph 一个 Path 增加节点、小分配、样式 clone、bbox 和绘制调用；不再用
有视觉差异的轮廓合并换取较少节点。既有 128 MiB Job / 15 秒 deadline 保持，
4096 glyph 成功返回且与短文本相同可见区域逐字节一致。这不是全字体/带复杂
描边的性能峰值验收。外层 group opacity 和整段 text paint-server bbox 不变；
彩色 glyph 混合顺序的代码审查不代替对应视觉实测。

累计 usvg 五文件补丁已从固定归档回放并校验 40 文件，新源码身份和 SBOM
已更新。下节“生产仍是 v217”是调查时停点；当前版本为 v0.2.33.218，但该节
跨样式、跨 chunk、嵌套约束和 writer 的剩余责任仍有效。

## textLength 接入前的真实失败基线（v217 后续调查，尚未开放）

生产内部版本仍是 v0.2.33.217；本节不代表 v218 已交付。当前 policy 仍拒绝
`textLength/lengthAdjust`，尚未将原型补丁复制到仓库第三方来源。没有为了通过
简单案例而开放会静默错误的属性，也没有把完整需求缩为叶子文字。

已联网核对 [SVG 2 文字规范](https://www.w3.org/TR/SVG2/text.html#TextLengthAttribute)，
并实际用 Chrome 155 的 SVG image → canvas 建立 18 组像素等价关系：三种
锚点 × text/tspan/textPath、单字符、零长度、spacingAndGlyphs 缩放、嵌套跨度、
不同 fill、压缩和约束后的尾随字符。最终 18 组浏览器对照全部成立，原文与结果
在 `linshi/otter-v218-textlength-browser.js/.log`。名称 v218 仅为实验编号。

隔离 renderer 原型最初 13 项中 6 通过、7 失败；仅处理零目标与末尾 anchor
gap 后，基础 13 项通过。扩展到 18 项后最终 **15 通过、3 失败**，证据
`linshi/otter-v218-textlength-prototype.log`，不能作为生产 child 已通过的证据。

明确剩余问题与下一实现边界：

- `parser/text.rs` 将 textLength 放在不重叠的样式 TextSpan 上。对于
  `<text textLength='80'>F<tspan fill='blue'>H</tspan></text>`，父元素的整体约束
  丢失；复制同一个数值给子 span 仍会把一个目标错误应用多次。必须保留来源
  约束身份、父子关系和跨 chunk 范围，与 fill/font 等绘制样式分离。
- `layout_text` 当前逐 chunk 整形并立即定位。绝对 x/y 和 textPath 边界会切
  chunk，不能按 chunk 重复完整目标；需要先准备约束涉及的 clusters，再执行
  约束调整、定位。嵌套约束次序、bidi/ligature 边界需继续来源对照，不能猜测。
- writer 当前按样式 span 写 textLength；AST 迁移必须同步 writer/回读验证，
  不能只修 parser/layout。原型将每源字节线性搜索 clusters 改为一次 cluster
  遍历，但这不是完整多约束复杂度门，也尚未进入生产。
- 压缩 FH 的原型与独立 x 坐标对照中，glyph 平移均为 `(20,40)` 和
  `(24.00390625,40)`，缩放相同，像素仍有 **18 个通道字节不同、最大差 48**。
  不能归因于布局精度或放宽像素断言；须继续检查合并 glyph 路径与分开绘制
  的重叠合成。探针 `linshi/otter-v215-context-prototype/src/bin/textlength.rs`。
- 初始尾随字符 oracle 假设紧接目标末端，浏览器实测否定了该假设。DOM
  探针与 image 像素对照确认还保留位移后的尾随间距；最终独立 oracle 已修正，
  不能简单删除所有约束末尾的 advance。DOM 测量与 image 测量的舍入差异保留，
  不互换为精确像素坐标。日志 `linshi/otter-v218-textlength-metrics.log`。

本轮仅改变实验与此失败基线；未改生产源码、依赖来源、版本或既有准入边界。
后续实际接入仍必须补真实 Job child、完整原生回归和来源/SBOM 门。

## v217 负 dx 路径文字修复

最终验证：路径文字 **6 passed（3.89 秒）**，完整库测试 **2748 passed /
51 ignored（140.81 秒）**，真实 EXE startup **7 passed（15.37 秒）**。
日志为 `linshi/otter-v217-textpath.stdout.log`、`otter-v217-full.stdout.log`、
`otter-v217-startup.stdout.log`。Clippy all-targets -D warnings、fmt、3150 文件
严格行数、8 项来源/SBOM 测试、许可证审计和 diff 检查通过。双 SBOM 验证
1257 个身份；旧 v216 SBOM 因 usvg 源码身份变化被实际拒绝。
联网 OSV advisory 仍为 5 锁、1822 包、25 漏洞 ID，不是发布放行。

v216 记录的负 dx 差异已定位并修复：锚定应使用相对位移之后的 advance 跨度，
不能只累计字符 advance；法线求解也不能假定字符的路径位置单调递增。现在
以局部 f64 计算跨度，再加公共路径起点；查询按距离排序，结果按原 cluster
索引回写，保留隐藏槽位和原字符顺序。普通文字与 Q/C 求解算法不变。

Chrome 155 的 12 组 scope/dx 矩阵及 12 组独立普通文字坐标对照保存在
`linshi/otter-v217-dx-anchors-browser.log`：text/tspan 的负 dx 参与布局，
textPath 自身的 dx 被忽略；start/middle/end 均通过独立坐标逐像素对照。
仓库回归另外把同一直线拆到 H130，让逆序字符跨越不同路径段，并保留零长段。
这项跨段变体通过原生精确像素等价检查，不冒充新增浏览器实测。

真实 Job child 修复前为 5 passed / 1 failed，首个差异字节 26250；证据
`linshi/otter-v217-dx-before.stdout.log`。修复后六项专项通过，断言没有放宽。
来源补丁从固定归档重放，全部 40 个 usvg 文件逐字节一致；未修改 registry。
锚定采用 advance 跨度，不是 ink bbox；排序只改变查询顺序，不改变 shaping。
仍不宣称完整 RTL、负 word-spacing、所有曲线路径或全部 SVG 语义一致。

验证结果与当前源码身份见 `linshi/otter-v217-evidence.json`。N01–N12 仍未完成；
下方 v216 的“未修复”描述是历史停点，已由本节覆盖，不能解读为当前状态。

## v216 路径文字增量与已知剩余差异

最终仓库验证：路径文字专项 **4 passed（1.57 秒）**，完整库测试
**2746 passed / 51 ignored（136.75 秒）**，真实 EXE startup
**7 passed（15.47 秒）**。日志为 `linshi/otter-v216-textpath.stdout.log`、
`otter-v216-full-fixed.stdout.log`、`otter-v216-startup.stdout.log`。
Clippy all-targets -D warnings、fmt、3150 文件严格行数、来源/实际 Cargo 解析绑定、8 项来源/SBOM 脚本测试、
许可证审计、1257 身份双 SBOM 和 diff 检查通过；旧 v215 SBOM 因源码身份变化被
实际拒绝。联网 OSV advisory 仍为 5 锁/1822 包/25 漏洞 ID，不是发布放行。
下述负 dx 差异仍未修复，不能将现有测试绿色宣称为完整路径文字一致性。
最终回执 `linshi/otter-v216-evidence.json` 保存实际 EXE/测试二进制和补丁源码身份。

新增 textPath/startOffset 准入；有限长度解析保留绝对值、百分比和负值语义。
textPath 纳入统一引用图触发与 href 边，外部引用、重复 ID、递归仍在 renderer
之前拒绝。实际字体和布局继续由既有受限 child 的 TextFonts/renderer 完成。

Chrome 155 实测九组：直线、绝对/百分比偏移、middle/end 锚点、缺失路径、超出
末端、垂直旋转和曲线平移。前五组与相同坐标的普通文字逐像素一致；后两组
几何变换等价，缺失/超出末端透明。完整源码和结果保存在
`linshi/otter-v216-textpath-expanded-browser.log`。这是等价关系验证，不是宣称
原生字体栅格与 Chrome 的所有像素相同。

真实 child 首轮 2 passed / 2 failed（0.96 秒），失败保存在
`linshi/otter-v216-textpath-expanded.stdout.log`。offset=40 的右侧边缘有 26 个
通道字节不同、最大差 63；直接 renderer 探针确认 glyph 平移为 49.951435，
普通文字则是 50。根因是 usvg 把直线转为三次曲线后用 0.5 容差逆弧长求点，
不是字体选错或简单抗锯齿容差。仅把控制点改为三分之一仍是 49.95592，不能
作为修复。现直线/Close 用精确端点距离参数化，零长度跳过，曲线分支保持原状。
原型三项等价/空结果回归通过；精确断言没有放宽。仓库实际 child 和全量结果见上。

修复后的真实 child 路径文字四项专项已通过。首轮全量 2745 passed / 1 failed /
51 ignored（137.28 秒），唯一失败是旧拒绝表仍把 textPath 本身列为未实现；现改为
尚未接入的 SVG 2 textPath `path` 属性拒绝夹具，不删除外部 URI、脚本、样式等
原有拒绝检查。首轮日志 `linshi/otter-v216-full.stdout.log` 保留，修正后全量结果见上。

只读复核指出的负 dx 边界已补浏览器/直接库探针：
`<text dx='0 -100'><textPath href='#p'>HH</textPath></text>` 在 Chrome 有 220 个
非透明像素，直接 renderer 为零。Chrome 也不等价于仅一个普通 H，因此不能
直接把“过滤负 offset 并重排”当作已经验证的修复；需先核对祖先 dx 在 textPath
边界的实际语义和库侧字符索引。证据为 `linshi/otter-v216-negative-dx-browser.log`
与 `linshi/otter-v216-textpath-layout-probe.log`。这是已确认的剩余差异，尚未通过
真实 Job child 专项覆盖，不以本轮有限测试绿色冒充完整路径文字一致性。

usvg 累计补丁已从固定归档重新回放并校验全部 40 个文件，只有四个文件变化。
新的源码哈希会使旧 v215 SBOM 不再匹配，必须生成 v216 的证据，不能复用旧放行。
当前仍不宣称全部 textPath 属性、复杂 shaping/曲线误差或完整 SVG 能力完成。

## v215 context paint 与窄画布描边修复

### 仓库正式接入与本地验证

native/Cargo.toml 现通过仓库 `third_party/usvg-0.48.1` 与
`third_party/tiny-skia-0.12.0` 的固定 path patch 构建，不再依赖临时 host。
Cargo 更新 native/Cargo.lock，实际 metadata 的 manifest_path 已与经哈希验证的
目录精确绑定；同名同版本但改指向另一 path 的解析会被来源门拒绝。
registry 缓存没有被修改，Tauri 的 Cargo 依赖来源不变。

仓库实际依赖重新编译后的完整库测试 **2742 passed / 51 ignored（139.32 秒）**；
真实 EXE startup **7 passed（16.14 秒）**。完整库测试包含三个 context paint/
窄画布测试和既有 SVG、Application 安装及生命周期回归。51 个 ignored 不是
全部执行通过的宣称，仍包括由父测试调用的私有入口与专用场景。
日志：`linshi/otter-v215-repository-full.stdout.log`、
`linshi/otter-v215-repository-startup.stdout.log`。

Clippy all-targets -D warnings、fmt、3149 文件严格行数门通过；行数策略只豁免
两个精确上游副本目录，42 条原历史基线记录逐项未变，基线/空例外表仅更新策略
哈希。来源/安全脚本测试 37 passed / 1 conditional skipped，行数检查器测试
16 passed。源码完整性覆盖 240 个上游文件，四个文件有受审补丁；首次补丁重放
证据与逐文件来源清单保留，构建/许可证/SBOM 校验不等同每次重新下载归档重放。

许可证审计覆盖 21 个直接 npm manifest、749 个 Rust 解析包，无禁止或未解析
许可证标记。CycloneDX/SPDX 均核对 1257 个身份，补丁包 PURL 含源码树 SHA256，
不与官方同版本身份混淆。最新联网 OSV advisory 扫描覆盖 5 个锁文件、1822 个
包、25 个漏洞 ID；相较 v207 新增 GHSA-qxg6-89mg-rqv8、GHSA-xp26-3454-mjgp，
均归现有 @vitest/mocker 4.1.11。扫描仍是有发现的 advisory，不是发布安全放行；
两个 patched crate 仍按上游名称/版本纳入扫描，不因本地来源跳过。

本轮未提交、推送、发布或修改真实用户数据。该修复不完成全部 SVG 兼容性及
N01–N12 迁移验收；既有 luminance 分数像素等边界和完整功能清单仍继续推进。

### 隔离双补丁验证进展（接入前记录）

临时 host 直接编译仓库 native 源码，通过 Cargo path patch 使用 usvg/tiny-skia
副本；真实受限 Job child 的 SVG 专项现为 **38 passed / 2 private ignored**
（15.61 秒），日志 `linshi/otter-v215-dualpatch-svg.stdout.log`。这不是仓库
生产依赖的结果：native/Cargo.toml 和 Cargo.lock 尚未切换，仍保留原版行为。
没有运行全量、startup 或正式发布门；最近生产依赖的完整绿色记录仍为 v214。

usvg 原型将上下文 stroke 解析与实际可绘制 stroke 的正宽检查分离，保留
UseNode/PathNode 坐标元数据。六个基础案例及四个 userSpaceOnUse 渐变/图案
案例通过，后者 Chrome 155 对照见 `linshi/otter-v215-context-paints-browser.log`。
原型的占位宽度仅作为上下文 paint 载体，未把零宽源路径变成可见描边。

新增正宽对照揭示另一独立缺陷：tiny-skia 0.12.0 的 hairline.rs 在实际 clip
向内缩 1px 失败时，直接跳过整条 AA 描边。原版库直接 probe 同样失败，不是
usvg 补丁回归；Chrome 155 的五组窄画布对照保存在
`linshi/otter-v215-stroke-browser.log`。临时 tiny-skia 补丁允许不存在内缩快接收
区域，但保留实际裁剪，并独立保留 Q/C 曲线的外扩快拒绝，避免离屏段额外细分。
测试覆盖零宽、2×1、2×2 的两个行位置，以及正常 3×3 画布；均经真实 child 通过。

两份临时副本位于 `linshi/otter-v215-context-prototype`，没有修改 registry。
已按 native lock 校验官方缓存 crate SHA256，逐文件核对 usvg 40 个文件仅改
3 个、tiny-skia 200 个文件仅改 1 个。`*-verified.patch` / `*-verified.json`
保留补丁与全部文件哈希；从 archive 重建后以 `git -c core.autocrlf=false apply`
回放，240 个文件逐字节哈希一致。首次回放发现全局 autocrlf 改变换行，未将
语义相同冒充字节一致；固定该选项后重新验证。正式接入仍需仓库内来源目录、
明确的第三方行数豁免、许可证/SBOM 来源身份及完整构建测试门，不能依赖 linshi。

### 原版生产依赖失败基线（保留）

原版 focused 回归 0 passed / 1 failed（0.54 秒）；该聚合测试执行六个案例，
汇总两个像素失败：零宽 use 和零宽 marker，另四例通过。完整输出为
linshi/otter-v215-context-expanded.stdout.log；初次 use 单项失败日志也保留。
Clippy/fmt/严格行数等静态门的通过不替代行为失败，未运行全量/startup。

context-fill/context-stroke 已被 svgtypes::Paint 和 usvg 支持，不是需要从零放开
的标签。Chrome 155 新实测七组：marker fill、marker 零宽 stroke、marker 隐形
stroke、两个不同颜色 use 的 fill/stroke、无上下文及 marker 简写属性探针。
证据为 linshi/otter-v215-context-browser.log；其中 marker 简写 XML 属性在该
浏览器被忽略，不等同尚未接入的 CSS marker 简写。命名浏览器已关闭。

真实 child 已复现零宽 use 的 context-stroke 丢失：浏览器输出红蓝两像素，
native 输出全透明。失败用例保留在 surface_svg_marker_tests；汇总所有案例
后统一失败，不删断言或把零改成非零。markerUnits=userSpaceOnUse 的零宽
调用路径经浏览器确认仍应有蓝色 marker，也已补入同类失败夹具。

库侧根因：usvg use_node.rs:32-41 用 resolve_stroke 构造 context_element，
style.rs:120 的正宽检查令已解析 paint 随 Stroke 一起丢失；converter.rs:1003-1013
对 marker 同样 clone 可绘制 stroke。Options 没有 context paint resolver，Tree
后处理时几何或 paint 已被丢弃。不能全局改 width、冻结共享定义颜色或声称可
用字体/图片 hook 修复。正确责任边界是分离上下文 paint 与实际可绘制 Stroke，
保留 UseNode/PathNode 坐标元数据，同时覆盖 use 与 marker；渐变/图案不能退化
成固定 RGB。尚未引入第三方 fork、修改 registry 缓存或新增依赖补丁。

上述原版失败阶段未通过降级或静默拒绝输入掩盖差异，当时最近完整绿色记录为
v214，未重复运行已知会失败的完整套件。后续修复和仓库验证结果见本节顶部。

## v214 静态路径 marker

最终验证：SVG 专项 35 passed / 2 private ignored（16.53 秒），完整库测试
2739 passed / 51 ignored（143.71 秒），真实 EXE startup 7 passed（17.48 秒）。
Application inline 安装/绘制随全量通过。Clippy all-targets -D warnings、fmt、
3148 文件严格行数、diff 及本轮十二文件 UTF-8 无 BOM 检查通过。
日志为 linshi/otter-v214-full.stdout.log、otter-v214-startup.stdout.log，
回执 otter-v214-evidence.json；命名浏览器已关闭。

生产 policy 开放 marker、marker-start/mid/end、本地引用、markerUnits、
markerWidth/Height、refX/Y、orient 和 marker overflow。图门增加 marker 文档
及三个 marker 引用边，拒绝 marker 自递归、跨 marker 与 use 混合递归，
不依赖 renderer 静默跳过递归而返回不完整图像；外部 URI 与 image 仍被拒绝。

已联网核对 [SVG 2 marker 定义](https://www.w3.org/TR/SVG2/painting.html#Markers)，
并实测 Chrome 155 十组 img→canvas：start/mid/end、auto、auto-start-reverse、
strokeWidth 尺度、参考点、viewBox、hidden/visible 裁剪。原始像素与源码保存于
linshi/otter-v214-marker-browser.log。surface_svg_marker_tests 独立拥有对照及
真实 Job child 拒绝测试；前置图门有直接单测，Application/EXE 已加入安装/像素夹具。
最终验证如上。不新增依赖、父侧 XML 处理、授权或生命周期 owner。

只读复核发现 marker 属性可从祖先继承，只有显式边会漏掉祖先 marker-end
指向子 marker、子内容再次继承该值的递归；usvg 会用 parent_markers 跳过，
不是已证明无限递归或崩溃。现按文档先序维护三个继承值，显式 none 截断，
避免逐节点回溯祖先的平方开销；新增前置拒绝和 none 正常截断测试。
图门仍为保守超集，不把静态 XML 继承检查宣称为所有 use 实例语义的完整证明。

这不是全部 marker 行为完成：复杂曲线切向、退化子路径、分数采样、context paint
及 marker 简写等仍需后续对照/接入；CSS、滤镜、动画等原迁移范围不缩减。
N01–N12 继续进行，未提交推送或发布。

## v213 pattern 内容选择与 overflow

最终验证：SVG 专项 32 passed / 2 private ignored（18.34 秒），完整库测试
2736 passed / 51 ignored（145.51 秒），真实 EXE startup 7 passed（17.55 秒）。
Clippy all-targets -D warnings、fmt、3147 文件严格行数、diff 和本轮九文件
UTF-8 无 BOM 检查通过。日志 otter-v213-full.stdout.log、otter-v213-startup.stdout.log
及回执 otter-v213-evidence.json 均在 linshi。没有提交、推送或发布。

在 v212 八组基础上，Chrome 155 实测新增十组：有/无 viewBox 的内容百分比、
title/desc/注释模板、尺寸覆盖及 hidden/visible/scroll/auto 四种 overflow。
证据为 linshi/otter-v213-pattern-browser.log；命名浏览器已关闭。
本夹具中四种 overflow 均裁剪到 tile；policy 现允许 pattern 上这四种值，
不是从规范猜测 visible 会跨 tile 环绕。内容百分比也按实际浏览器结果保留，
不改为从 viewBox 推断的不同几何。

首轮专项 3 passed / 1 failed：只有 title 子元素的 pattern 在 Chrome 透明，
usvg 却去掉描述元素后采用 href 模板内容而绘制红色。失败摘录保留在
linshi/otter-v213-first-failure.txt。normalize 对只有 title/desc 元素子内容的
pattern 插入 SVG namespace 空组，保留“有自身内容”并阻止错误的内容继承；
不删除 href、不改变模板属性继承、不影响仅注释/空白的模板。全部外部引用仍先
经过原 policy 和图门，适配不引入资源读取。新增测试不放宽逐像素断言。
完整回归已如上通过；这不是全部 pattern、SVG 或 N01–N12 完成。

## v212 静态 pattern

最终验证：SVG 专项 30 passed / 2 private ignored（14.92 秒）；完整库测试
2734 passed / 51 ignored（142.09 秒）；真实 EXE startup 7 passed（16.98 秒）。
Application inline 实际安装/绘制测试随全量通过。Clippy all-targets -D warnings、
fmt、3147 文件严格行数、diff 检查及本轮十二文件 UTF-8 无 BOM 检查通过。
完整日志归 linshi/otter-v212-full.stdout.log、otter-v212-startup.stdout.log；
回执为 otter-v212-evidence.json。命名浏览器已关闭，测试进程结束后复核清理。

生产 policy 开放 pattern、patternUnits、patternContentUnits 和 patternTransform；
继续使用原短命 Job child 和容量/时间/像素预算，不在父进程解析 XML，不开放外部来源。
引用图准入增加 pattern 文档，以及 fill/stroke 的本地 paint 引用、pattern/gradient
模板 href 边，拒绝直接/间接图案递归、use 交叉递归与重复 ID。

已联网核对 [SVG 2 pattern 合同](https://www.w3.org/TR/SVG2/pservers.html#Patterns)，
并实际运行独立 Chrome 155 img→canvas 八组基线：userSpaceOnUse、默认
objectBoundingBox、content bounding box、viewBox meet/none、平移、href 模板和
零尺寸。真实 child 像素逐字节通过；证据为 linshi/otter-v212-pattern-browser.log。
surface_svg_pattern_tests 独立拥有像素/拒绝测试，引用图单测直接验证前置拒绝。
Application inline 和实际 EXE smoke 已增加 pattern 夹具并随上述运行通过。

引用图属于保守准入：两种 href 均计边，可能拒绝含被忽略引用环的文档；不把
深度界限当作总展开量界限，Job 内存/期限门仍保留。无 mask/symbol/use/pattern
的纯 gradient 文档不进入本图门，不能宣称所有 paint server 均被本轮检查覆盖。

这是静态 pattern 的增量接入，不是完整 SVG/CSS/动画/滤镜/foreignObject 完成；
pattern overflow 属性尚未准入，分数采样、复杂嵌套与完整模板继承仍需扩展对照。
不以此关闭已有 mask 量化、文本审计或 N01–N12 的未验收项。未提交推送或发布。

## v211 symbol/use（修复后回归验证）

最终验证：symbol 专项 4 passed（2.27 秒）；完整库测试 2731 passed / 51 ignored
（132.13 秒）；真实 EXE startup 7 passed（14.93 秒）。Clippy all-targets
-D warnings、fmt、3146 文件严格行数、diff 检查和本轮七文件 UTF-8 无 BOM 检查通过。
最终日志为 linshi/otter-v211-adapter-fixed-full.stdout.log 和
linshi/otter-v211-adapter-startup.stdout.log，回执为 otter-v211-adapter-evidence.json。
首轮全量 2730 passed / 1 failed / 51 ignored 的日志 otter-v211-adapter-full.stdout.log
保留：双 href 用例揭示 roxmltree attribute("href") 不限定 namespace；最终显式筛选
无 namespace 属性修正此问题。未删失败用例、未降低像素断言、未发布或提交推送。
这些结果不关闭既有 mask luminance 量化差异、完整文本审计或 N01–N12 总体验收。

续进修复：normalize 使用原 XML range 把 symbol 定义适配为 defs/svg，保留
QName、ID、属性与子内容，复用 renderer 的 SVG 实例 viewport 分支，不固化百分比，
不复制或修改第三方源码。仅对引用 svg/symbol 的 use 判断有效零尺寸；普通图形
的 use 不受 width/height 影响。有效 href 优先于 xlink:href，与 renderer 一致。
ID 查找使用一次建立的索引，前置引用图拒绝重复 ID。

Chrome 155 对原文/适配后文档的八组逐字节比较全部相同，证据为
linshi/otter-v211-symbol-adapter-browser.log。原百分比与零尺寸失败夹具保留，
修复后 SVG 专项 27 passed / 2 private ignored（11.87 秒）。新增内容百分比、
无 viewBox、多尺寸实例、嵌套与间接引用、零定义尺寸被 use 覆盖、未引用定义、
自闭合与前缀 QName 等测试；随后补入双 href 优先级的双向回归。
以下失败记录为修复前证据，不代表当前修复后的专项结果；完整回归结果另行记录。

已接入 symbol 标签及其 overflow，增加真实 Application/EXE smoke 夹具。
引用图模块更名为 surface_svg_reference_policy，适用于含 mask/symbol/use 的文档，
继续拒环、重复 ID 和超过 128 层展开，不再仅在含 mask 时保护 use 引用。

Chrome 155 八组 img→canvas 基线归 linshi/otter-v211-browser-expanded.log：
meet/none/slice、默认裁剪/visible、百分比、零尺寸和两个不同颜色实例。
首轮 native 发现百分比差异：根 80×40、symbol viewBox 0 0 10 10、use 的
x/y=25%、width/height=50% 时，浏览器红色范围 [30,10,50,30]，native 在
[25,10] 已绘制。usvg use_node.rs:55-86 先改 use_state 的 viewBox，再用它
重新解析相同 use 百分比以构造 transform/clip；尚未修复，不能以绝对尺寸替换
夹具或放宽像素断言来宣称通过。多个实例/定义复用的动态 viewport 必须保留。

最新 symbol 专项 2 passed / 1 failed（1.07 秒）：多实例颜色/位置和循环/外部资源
拒绝通过；像素对照失败汇总出两个案例：百分比及零 use 宽度（native 在 0,0
错误绘制）。完整日志为 linshi/otter-v211-symbol-focused.log，不删案例、不放宽断言。

源码复核定位到两个需分离的上下文：调用处 viewport 必须一次求 use 的位置/尺寸，
symbol 内容有 viewBox 时必须用其用户坐标尺寸，没有 viewBox 时用本次实例尺寸。
只修 transform/clip 再解析还不足以修内容百分比；多个实例不能共享固定尺寸结果。
固定 usvg 的公开 Options 没有 use/symbol 长度解析 hook，错误 Tree 后处理也已失去
原百分比上下文。后续需评估可维护的转换层补丁或真正按实例展开的适配，不能全局
固化定义属性。新增验收须覆盖内容百分比、有/无 viewBox、多尺寸实例、嵌套 viewport、
use 引用 use、meet/slice/none、overflow 及零尺寸。

修复前 v211 不是已验收候选；当时最近完整通过记录为 v210。未发布。
Clippy all-targets -D warnings、fmt、3146 文件严格行数和 diff 检查通过；
回执为 linshi/otter-v211-evidence.json，不将编译通过计作 symbol 行为通过。

## v210 SVG mask

开放本地 mask 引用、alpha/luminance 类型及两种 maskUnits/maskContentUnits。
Chrome 155 实测八组 img→canvas：彩色亮度、alpha、半透明、objectBoundingBox
内容、userSpaceOnUse 区域、空定义、零区域、定义自身 mask 属性。证据归
linshi/otter-v210-browser.log；隔离命名浏览器已关闭。

浏览器忽略 mask 定义自身的 mask 属性，usvg 会应用关联；normalize 模块只移除
该位置属性，保留子内容正常遮罩，外部引用先经 policy 拒绝，不能借移除属性绕过。
原根尺寸/零尺寸嵌套规范化迁入同一独立模块；viewport 只负责尺寸与预算。

彩色亮度存在已知量化差异：tiny-skia 0.12 的 mask.rs:86-87 使用 ceil，红色
遮罩 native alpha=55、Chrome=54。该组测试明确允许最多 1 个 u8 等级误差，
其余七组逐字节比较；未修复或宣称逐字节等价，完整 N11 差异验收仍未关闭。

mask 子内容多节点环不能仅依赖 usvg 的局部环检查。新增非递归结构引用图，纳入
元素子树、mask/clip-path 和 use 两种 href，拒绝图中的任意长度环、重复 ID 及最长展开
路径超过 128 的含 mask 文档。此门保守覆盖未使用定义，不宣称完整浏览器无限
资源语义；仍受 100000 XML 节点与原 Job/期限约束。最终像素预算不能代表 mask
离屏中间峰值，内存最终边界仍由 Job 执行；共享 DAG 的总展开工作量仍受期限约束，
不是由深度门证明小常数上限。
最新专项 23 passed / 2 私有 child（父测试实际调用），含三节点环实际 child 拒绝、
use 间接环、重复 ID 和 128 层边界检查。全量 2727 passed / 51 ignored
（126.81 秒），包含真实 Application mask 安装/绘制；Clippy、3145 文件严格行数
检查通过。依赖清单/锁文件 SHA256 未变，没有新增依赖。
startup 7 passed（15.53 秒），实际产品 EXE mask smoke 通过；fmt/diff 通过。
证据归 linshi/otter-v210-evidence.json 和 otter-v210-full.*；未提交推送或发布。

## v209 嵌套 viewport

解除嵌套 svg 的整体拒绝，支持其位置、尺寸百分比、viewBox、preserveAspectRatio
及 overflow=visible/hidden/scroll/auto。自然尺寸只取根 viewport，嵌套不改变
图片逻辑/栅格尺寸或父侧协议。仍在原 Job 内转换/裁剪/渲染，未开放 CSS、外部
资源或新的文件读取路径；嵌套子树也完整执行原准入策略。

Chrome 155 的实际 img→canvas 基线共 11 组：默认裁剪、visible、meet、none、
slice、百分比、两层嵌套、零宽、hidden、scroll、auto。全部使用 80×40 根图片；
测量矩形位置和逐像素颜色，不仅比较覆盖数量。证据位于
linshi/otter-v209-browser-expanded.log，命名隔离浏览器已关闭。

首轮 native 暴露零宽差异：浏览器不绘制，而 usvg 0.48.1 退回父 viewBox 并绘制。
viewport 规范化因此将显式零宽/零高嵌套 svg 标记 display=none，保留节点/ID 与
子树，不删除可能被引用的定义。覆盖已有 display、百分比零及多层编辑的回归。
正常 viewport 不做重写；根零尺寸仍按既有无效图片规则拒绝。
专项 18 passed / 2 私有 child（父测试调用），全量 2722 passed / 51 ignored
（133.41 秒），包括真实 Application 的嵌套图片授权安装/绘制。
Clippy all-targets -D warnings 与 3142 文件严格行数检查通过。依赖清单/锁文件
SHA256 与 v207 相同，无新增依赖。未宣称所有 SVG/CSS/文字完整性已经完成。
startup 7 passed（16.50 秒），实际产品 EXE smoke 同时验证嵌套裁剪和零尺寸不绘制；
fmt/diff 检查通过。回执为 linshi/otter-v209-evidence.json，未提交推送或发布。

v208 跟进 v207 全量门：数字删除测试的线程修饰键依赖已用真实 HWND 复现，
测试固定无修饰键前提并恢复快照；详见 WINDOWS_NATIVE_MIGRATION.md。
本轮不改变 SVG 生产渲染范围，不把测试夹具修复计作 SVG 新能力。
v208 完整回归 2721 passed / 51 ignored（141.68 秒），含原 SVG 文字用例；
保留下面的 v207 首次失败记录，不以新通过结果抹去历史失败及剩余文字边界。

## v207 文字接线（开发中，尚未完成整轮验收）

启用 resvg 的 text feature，仍关闭 system-fonts、memmap-fonts、raster-images 和
svgz。text/tspan 进入有限 allowlist；字体只由既有 Job child 内的 DWrite 来源提供，
不由 fontdb 扫描目录。显式 TTC index、共享 Arc 字节、查询/文件预算保持不变。
新增 surface_svg_text_fonts 拥有 resolver、错误回传、字体 metrics 准入和布局缺字门。
skrifa 0.44.0 显式固定为与 usvg 相同的解析器，用于提前拒绝其 load_font 失败条件；
树转换后逐个检查实际布局 glyph ID 0，不能以已有其他像素作为文字成功证据。
这些保护不等价于所有字体/复杂 shaping 都与浏览器一致。glyph 检查仅覆盖最终
存活布局；usvg 的 convert_span 或零 bbox 可在最终树形成前丢弃 span/文字，
因此不能宣称已经完成原始文字到输出的完整性审计。此边界有源码证据，但当前
没有构造真实系统字体触发“残余缺字再被零 bbox 丢弃”的复现夹具。

本机浏览器 zh-CN/en-US 两个隔离 context 的实测 generic 族一致：默认/sans-serif
为 Microsoft YaHei，serif 为 SimSun，monospace 为 Consolas，cursive 为 Comic Sans MS，
fantasy 为 Impact；后三者中文回退至 Microsoft YaHei。这里使用该已测配置，不把
navigator.language 当作字体偏好的充分依据，也不宣称跨机器一致。

非零 simulations、非 unit fallback scale、无映射字体明确拒绝。usvg fallback
只有单字符候选接口，尚不等价于 DWrite 的完整范围布局；CSS、textPath 等未开放。
字体 variations 的默认 wght 不能误当不支持属性，已修复。metrics/布局严格门加入
后最新专项为 17 passed / 2 private-child ignored（父测试实际调用）；Clippy
all-targets -D warnings、fmt 和 3142 文件严格行数检查通过。Application inline 和
产品 EXE smoke 已添加文字夹具；startup 7 passed（20.74 秒），实际 EXE 的 SVG
文字 smoke 通过。完整 native 结果及未关闭失败见下文。

全量结果：2719 passed / 1 failed / 51 ignored（142.78 秒）。失败在
surface_number_erase_rejection_retains_undo_for_delete_backspace_and_clear 的
EM_CANUNDO 断言（src/desktop/surface_number_erase_tests.rs:47，返回 0）；
独立 focused 1 passed（0.09 秒）。不削弱断言，不把独立通过当成根因修复。
首次失败与 focused 日志分别为 linshi/otter-v207-full.* 与
otter-v207-erase-focused.*；本轮全量绿色门未完成。

## v206 系统 fallback 范围合同

Fonts::map_text 通过 IDWriteFontFallback::MapCharacters 映射输入开头的文本范围，
返回 UTF-16 单元数、原始 scale 以及可选 Face；不把 UTF-16 长度当 UTF-8 字节数，
拒零进展、越界、切断 surrogate pair、非正/非有限缩放。未找到字体保留 None，
调用方必须显式处理缺字，不能直接当作成功的透明文字。

私有 surface_svg_text_source 持有有界 UTF-16 和 NUL-terminated locale，完整分析
调用期间保持 backing store 有效；locale 和段落方向由调用方明确传入，不读取
用户浏览器 profile。每次最多 16384 UTF-16 单元，空/超限文本及非法 locale 拒绝。
不做隐式数字替换；完整 shaping/数字替换/双向布局仍属于后续排版合同。
fallback 与显式 family 共用 256 查询、16 文件、48 MiB 总字节和同句柄文件准入，
没有绕过可信目录加载字体的第二条路径，也没有单独增加常驻字体缓存。

联网核对的官方 API 合同：
[MapCharacters](https://learn.microsoft.com/en-us/windows/win32/api/dwrite_2/nf-dwrite_2-idwritefontfallback-mapcharacters)
返回 mapped range/可空字体/em scale；
[GetTextBeforePosition](https://learn.microsoft.com/en-us/windows/win32/api/dwrite/nf-dwrite-idwritetextanalysissource-gettextbeforeposition)
在位置 0 或超出全文时返回空块；
[GetLocaleName](https://learn.microsoft.com/en-us/windows/win32/api/dwrite/nf-dwrite-idwritetextanalysissource-getlocalename)
要求 locale 指针在分析期间保持有效。

重要接线限制：本地 usvg 0.48.1 的 text/mod.rs 中 fallback 回调只接收单个 char
和已用字体 ID，返回 Option<ID>；它没有范围、scale 或 simulations 返回通道。
text/layout.rs 的 fallback 会重新 shape 整段并尝试逐 glyph 替换，不等价于 DWrite
范围映射。primary 返回 None 可跳过 span；fallback 返回 None 只退出查找并保留
缺字 glyph，不会自动令整个 SVG 失败。后续必须有显式错误/缺字门与排版接线，
不能丢弃这些字段后称为无损接入。当前仍未开放 text，没有 text feature/依赖变更。

共享 launcher 的真实 Job 已执行八组映射：Hello、中文、A中文😀、单 emoji、
阿拉伯文、印地文、👩‍💻 ZWJ 序列和 U+10FFFF。本机混合样本按 UTF-16 长度
1/2/2 分别返回 ARIAL.TTF index 0、MSYH.TTC index 1、SEGUIEMJ.TTF index 0；
中文基线选择的是 YaHei UI face，不能自动替成同文件 index 0。印地文映射到
NIRMALA.TTC，U+10FFFF 明确返回 None/长度 2。所有本轮样本 scale=1，并不证明
所有系统映射都为 1。只验证映射和文件来源，没有验证 glyph 轮廓、连字或呈现。
原始 JSON 输出归 linshi/otter-v206-focused.stdout.log。

这份系统映射合同不是新的产品视觉基准：最终仍须对照旧 img 宿主的实际结果，
不能把“与 DirectWrite 全部语义一致”替代“保留旧宿主文字表现”的目标。排版接线
应先用相同输入验证组合文字/字体模拟/emoji 等，不因一个库回调方便就删除差异。

v206 首轮全量 2716 passed / 1 failed / 51 ignored（137.00 秒）：UIA Select
在已展开后丢失焦点并收起，原生仍有 2 项，UIA 枚举为 0。保留 full 原始日志；
定向 trace 诊断通过，未修改该测试或重试其断言；独立 full-retry 为 2717 passed /
51 ignored（134.24 秒）。不能据复测通过称该偶发问题已根治。七个私有 child 由
父测试调用，原 44 项专项仍未执行。
Clippy all-targets -D warnings、fmt、3141 文件严格行数通过；startup 7 passed
（19.39 秒）。源码/全部失败与复测日志哈希归 linshi/otter-v206-evidence.json。

## v205 按需字体来源模块

新增 surface_svg_worker::fonts（surface_svg_fonts.rs）和私有 font_file 叶模块。
Fonts::new 先要求现有图片 Job 限制；select 仅接受 family/weight/stretch/style，
不接受 SVG 文件路径。使用 IDWriteFont3 的 face reference 获取文件/索引/模拟标记，
不先调用 CreateFontFace。系统 collection/family 元数据仍由 DirectWrite 管理，
不能声称 OS 在来源准入前完全不会读取其他已安装字体元数据。

GetWindowsDirectoryW 定位机器级 Fonts 目录，不依赖 child 已清空的环境变量。
持有目录句柄；字体文件仅允许本地固定磁盘、同目录直接子文件、ttf/ttc/otf。
打开前拒设备/UNC/ADS/DOS 保留名，打开后拒 reparse/非磁盘/非普通文件，并比较
最终句柄路径的父目录。只读共享拒并发写入/替换，取长度和读取使用同一句柄。
单文件 24 MiB、累计 bytes 48 MiB、最多 16 个文件和 256 次查询；分配前拒超额。
同文件 TTC face 共享 Arc 字节，index/simulations 分别返回；不静默丢失模拟样式。

这是已进入原生 crate 的文字接入前置，不是已完成 SVG 文字：renderer 尚未调用
此模块，生产 text/tspan 仍拒绝，text feature 未开启，没有新增依赖。
接下来仍需排版器按需 resolver、generic/default/fallback 合同和缺字失败门。
新增测试通过现有共享 launcher 的真实 Job 运行，不新增生产 CLI 或容量池。
字体索引/模拟样式断言针对本机安装基线，不冒充跨系统固定编号。

初次 focused 测试发现正斜杠输入不兼容，已修正路径分隔符规范化。文件符号链接
夹具创建实际返回 Windows 1314（无所需特权），未静默跳过后计通过；改用真实
junction 目录拒绝测试。文件 symlink 和祖先目录并发替换攻击仍没有运行验收。

v205 原生全量 2715 passed / 51 ignored（133.03 秒），新增三个测试全部通过；
51 ignored 中七个私有 child 由父测试实际调用，原 44 项专项未执行。
Clippy all-targets -D warnings、fmt、3138 文件严格行数检查通过。
startup 7 passed（16.94 秒）；源码/日志哈希归 linshi/otter-v205-evidence.json。
没有提交推送或公开发布。

## v204 后续：文字字体与受限 Job 实测

本次为生产接入前的受控实验，不改变 v0.2.33.204、生产依赖或 XML allowlist。
生产仍明确拒绝 text/tspan；不能把实验成功当作已支持文字。

隔离 HeadlessChrome 155 对 7 种 family × 英文/中文执行 14 个 img→canvas 案例。
无 family 和 sans-serif 的 Hello 均为 556 个非零 alpha 像素、alphaSum=86212、
bbox=[2,8,59,28]；Times New Roman 为 432、67326、[0,11,52,27]。
另对 inline SVG 的 `Hello 中文测试` 调用 CSS.getPlatformFontsForNode，实际返回
Microsoft YaHei / MicrosoftYaHei。此字体身份直接证据来自 inline SVG；img 的
结果支持默认字体差异判断，但不冒充已直接读取 img 内字体或跨 locale 默认值。
serif 与 SimSun 样本一致；不把本机默认族硬编码成所有 Windows/浏览器的默认族。

临时 resvg/usvg 0.48.1 程序装载固定 Windows 字体，并把默认族设为 Microsoft
YaHei 后，Hello 为 472、88128、[2,8,58,28]。中文浏览器为 1205、166267、
[1,8,95,31]，候选为 947、168046、[2,8,95,31]。几何更接近但不相同；不能把
差异全部归为抗锯齿，也不能继续用 Times New Roman 的旧结果评价默认字体排版。
本机 msyh.ttc 与 msyh.ttf 在这 14 个案例输出一致，不证明所有字体版本等价。

进一步编译并运行独立 font_job 探针，先把自身加入 128 MiB process/job memory、
active-process=1、kill-on-close Job，再创建 isolated DirectWrite factory。
实际完成 system collection → family → matching font → face → files → local loader
→ path，正常体/粗体共 14 次匹配，另一个不存在的 family 明确 exists=false。

- Microsoft YaHei UI 与 NSimSun 实际返回 TTC face index=1；对应 YaHei、SimSun
  为 index=0。仅传文件路径或只取第一个 face 会丢失字体身份。
- SimSun/NSimSun 的粗体返回 simulations=1，仍是同一 simsun.ttc。只加载字体
  bytes 不会自动表达 DirectWrite 的模拟粗体合同。
- Times New Roman、Arial、Segoe UI 的 HasCharacter('中') 为 false，YaHei/SimSun
  为 true。family 匹配成功不等于覆盖中文，必须独立处理字符 fallback。
- 只装载五个固定正常体文件，去重后 41,217,632 bytes；小栅格运行 Job 报告
  peak process/job memory=42,835,968 bytes。另一个独立运行使用 4096×4096、
  67,108,864-byte 栅格，峰值为 110,063,616 bytes，正常退出。两次 backing extent
  不同，不比较 painted 数作为渲染等价证据。数值是 Job 峰值，不是全机工作集。

这证明当前机器的受控 DWrite/字体加载/文字绘制能在该资源限制内运行，不证明
恶意 SVG、复杂文本、多个字重、最大树/滤镜等组合都能成功。探针自行入 Job，
未走生产父子管道、授权安装、取消/超时链；未提高生产 128 MiB 限额。
临时探针只读取硬编码允许的自选字体文件，路径比较不是可复用的生产安全准入。

生产实现的下一边界：按需、有总量上限地解析和加载字体；保留 TTC index 与
simulation 信息；字体名称只用于选择，不能作为路径。local loader 本身不证明
文件可信，必须验证最终打开句柄的磁盘文件类型和系统字体目录归属，排除网络、
用户字体及重解析越界；不能用先 canonicalize 后重新打开代替句柄校验。缺字必须
有明确 fallback/失败行为，不能静默生成透明文字。默认族、generic family、
fallback 的范围与缩放仍须对照旧宿主；正式启用 text feature 后重跑依赖安全门。

原始证据均在 linshi：otter-svg-text-browser.json、otter-svg-text-native.log、
otter-svg-text-fonts.json、otter-svg-text-font-job.log；可执行探针归
otter-svg-audit/src/bin/{text,font_job}.rs，浏览器脚本为 otter-svg-text-browser.js。
这些是受控测试输入与字体元数据，不复制或分发字体文件。浏览器会话已关闭。

## v204 小数尺寸端到端接线

SVG 请求/响应升级为 OSI2/OSP2：16-byte 栅格 header 后附加 8-byte little-endian
f32 逻辑宽高，再发送 PBGRA。WebP 保持 OWI1/OWP1，不增加扩展。child 写成功
header 前和父侧 pixel_buffer 分配前均验证有限正值、最大逻辑尺寸、安全缩放和
ceil 与栅格对应；旧版/跨格式响应不兼容，不自动猜测。
每次解码启动当前 EXE 的私有 child，没有常驻旧 worker 或持久化协议数据；若进程
二进制版本意外错配则失败并回收，不降级解析。回退需回退父/child 同一完整构建，
不能仅替换协议一端；无需迁移或删除用户数据。

viewport 保留小数逻辑尺寸，像素 extent 向上取整，root transform 使用逻辑尺寸
而非取整栅格。Image::from_surface_sized 把尺寸与像素一起交给 Resource/Inline
Decoded，Ready::transfer_image 通过原 mount/Grant/lease 检查后一并转移，窗口
使用 Bitmap::with_logical_size 安装。通用 Image 不增加 SVG 字段；旧 from_surface
及 Ready::transfer 是仅栅格兼容接口，不能用于 Surface UI 的小数图片安装。

已解除小数自然尺寸的临时拒绝：显式小数、cm/mm 和 viewBox 推导小数可沿实际
child/安装路径进入呈现。自然 DOM 属性的四舍五入值不等于本协议的 raster extent；
例如逻辑 80.5×40.25 用 81×41 栅格保存，不能按浏览器 naturalHeight=40 截断。
不足一像素的非零逻辑尺寸也不再因整数 naturalWidth=0 直接当作无布局。

仍未证明全部浏览器像素一致：CSS 布局的子像素量化、0.5 宽的百分比图形边缘
采样、非整数 viewBox 抗锯齿及 DPI 重栅格化仍需继续对照。文本/CSS/动画/
foreignObject/嵌入图片等原有缺口不变。当前一逻辑单位一栅格像素，不把 ceil
extent 当作高 DPI 或完整 SVG 迁移完成。无依赖变更、提交推送发布。

v204 全量 2712 passed / 50 ignored（138.33 秒），包含实际 child 小数几何、cm/mm、
viewBox 比例、Application 内联小数安装及绘制、合法/九类故障元数据 peer 与回收。
50 ignored 中六个私有 child 由父测试实际调用，原 44 项专项未执行。
Clippy all-targets -D warnings、fmt、3134 文件严格行数通过。
startup 7 passed（18.28 秒），实际产品 EXE 的 SVG child 同时验证 2.5×1.5 逻辑尺寸、
3×2 栅格及未放大的 1×1 像素。证据和源码/日志哈希归 otter-v204-evidence.json。

## v203 呈现层尺寸分离

Bitmap 现在单独持有浮点 logical_size，整数 Image.width/height 仍只表示受预算约束
的栅格像素，不修改通用截图/Relay Image。默认 Bitmap::new 按整数像素初始化，
保持既有格式。Surface 私有构造器验证逻辑尺寸有限、正值、可安全缩放且 ceil 与
栅格对应；当前仅支持每逻辑单位一个栅格像素，不冒充 DPI 重栅格化。
Presentation 把浮点尺寸传给 NativeMeasure；原整数 with_images 入口保留为转换
适配，不复制布局算法。contain 比例使用 logical_size，DrawBitmap 的 sourceRect
裁掉栅格尾部 padding，上传预算、target cache 和销毁仍按真实像素管理。

这是生产测量/绘制层的前置改动，不是小数 SVG 已开放：child 协议与 Ready 安装链
仍需携带并独立验证尺寸元数据，SVG viewport 仍保留原小数拒绝。下一步接通该链路，
再比较浏览器边缘采样；不得把注入元数据的呈现测试说成 SVG 端到端通过。

三项专项已通过：小数尺寸的测量/reflow、真实 WIC/D2D contain 与彩色列 sourceRect
映射、真实隐藏 HWND 的缩放比例/上传释放。NaN、无穷、零、ceil 不匹配和极小
正浮点数明确拒绝；最大 32768 布局范围除以逻辑尺寸必须有限，避免 contain 溢出。
彩色列夹具可区分错误的全栅格拉伸，但不代表所有 padding 边缘抗锯齿都与浏览器一致。
v203 全量 2709 passed / 49 ignored（132.18 秒），Clippy all-targets -D warnings、
fmt、3133 文件严格行数通过。49 ignored 中五个私有 child 由父测试实际调用，
原 44 项专项未执行。无依赖变更、提交、推送或公开发布。
startup 7 passed（19.15 秒）；源码/日志哈希与进程复核归 otter-v203-evidence.json。

2026-10-22，初始基线取自 v0.2.33.200；v202 已接入首批静态矢量，完整 SVG 与 N01–N12 尚未完成。

## v202 静态矢量生产接入

固定 resvg 0.48.1（关闭全部默认 features）与 svgtypes 0.16.1。SVG MIME 进入固定
OSI1/OSP1 child 协议，不回退位图；Inline data URL 已接通同一 Image 路径。XML、
自然尺寸、树转换、栅格化均在原 128 MiB Job 内，复用唯一 256 MiB child 容量池。
父侧 image worker client 先验证响应 codec/尺寸/count，再领取共享 Pixels，继续
完整读取、成功退出、EOF、取消与回收门。WebP 额外保留 RIFF 预检尺寸匹配。

首批接受基本几何/path、分组、渐变、clipPath 和内部 use 引用。所有后代及属性
经过显式 allowlist；text、foreignObject、动画、script、image/feImage、CSS/style、
滤镜、嵌套 SVG、未知属性/命名空间明确失败。href 和 FuncIRI 仅限本树 fragment；
两个图片 resolver 均拒绝。允许属性仍保留库的值容错，不能说所有非法属性必定失败。
严格 XML parser 使用 allow_dtd=false、nodes_limit=100000、无 entity resolver，
把同一安全 Document 交给 from_xmltree，不经过会重新启用 DTD 的 from_str。
空 DOCTYPE 可能被 roxmltree 接受；本轮不宣称拒绝任何 DOCTYPE。处理指令整体拒绝。

独立 viewport owner 依据已有浏览器基线计算自然尺寸/内部 viewport，再仅替换
根 width/height 并安全重解析；没有按绘图包围盒推导尺寸。绝对单位采用 96 DPI，
无尺寸默认 300×150、viewBox 比例容纳、单确定轴及百分比已有受限 child 覆盖。
小数自然尺寸、字体相对单位目前明确失败，未冒充浏览器舍入或字体支持。
额外验证 f64→f32 内尺寸、缩放有限正值及 tree.size 一致性。tiny-skia 已预乘，
只交换 R/B 并 take 原缓冲，不二次预乘或复制整图。

仍需继续：文字/中文 fallback、CSS、foreignObject、动画、嵌入图片/失败标记、
滤镜、更多尺寸语义、布局/DPI 变化的重栅格化。首批不是完整浏览器 SVG 替代。
Job 是资源/生命周期边界，不是 AppContainer 或文件系统权限沙箱。

本轮依赖门：五锁 1810 包、20 个现有漏洞 ID，Advisory 验证通过而非零漏洞；
许可证检查 737 个 Rust 包/40 个表达式，双 SBOM 1245 个唯一身份；无提交推送发布。

最终原生全量 2706 passed / 49 ignored（119.07 秒），包括真实 Application Inline
安装/绘制、10 组自然尺寸/覆盖量、精确半透明 PBGRA、本树渐变/裁剪、超限节点、
极小百分比、外部 href/xlink/CSS、DTD entity 与不支持内容拒绝。WebP 坏响应/回收
回归通过。49 ignored 中五个私有 child 由父测试实际调用，原 44 项专项未执行。
Clippy all-targets -D warnings、fmt、3133 文件严格行数通过。
startup 7 passed（15.09 秒），其中正式产品 EXE 同时完成 WebP/SVG Job child 像素
smoke；不是仅 libtest 分流。源码/锁文件/日志哈希归 otter-v202-evidence.json。

## 小数尺寸后续实测：不能只取整放开

v202 后续使用独立 HeadlessChrome 155 会话完成 7 个尺寸/绘制案例、15 个裁剪边缘
案例及 6 个真实 DOM 布局案例。原始数据归 otter-svg-fraction-baseline.json，探针
分别为 otter-svg-fraction.js、otter-svg-fraction-edge.js、otter-svg-fraction-layout.js。
DOM 复用 DeclarativeSurface.css 的 display:block/max-width/max-height/object-fit:contain；
原渲染入口是 DeclarativeSurface.tsx 的 img，不以 Canvas 代替实际布局测量。

- 2.5×1.5 返回 naturalWidth/Height=3×2，但原 1×1 矩形仍仅覆盖一个完整像素。
  因而把最终栅格宽度除以原小数 viewport 当作根缩放，会错误放大几何。
- width=1.4 返回 naturalWidth=1，DOM 宽为 1.390625；在足够大的 Canvas 上绘制
  覆盖满宽的图形，前两个 alpha 为 [255,102]。整数自然尺寸不是绘制范围上界。
- width=0.49 返回 naturalWidth=0，drawImage 本探针没有绘出像素，但 DOM 宽仍为
  0.46875；不能把整数自然尺寸为零直接等同于无布局或损坏图片。
- width=0.5 返回 naturalWidth=1，DOM 宽为 0.5。rect width=1 或 99 时首像素
  alpha=127，rect width=100% 时为 96；不能只拿面积相乘代替浏览器裁剪/采样。
- 1cm×1mm 返回 38×4；1cm×1 的 DOM 宽为 37.78125，而非整数 38。
- viewBox=0 0 7 3 的自然尺寸为 300×129；具体边缘抗锯齿亦已保存，不能只比尺寸。

v202 定位的原生接缝：surface_image_paint::Bitmap::natural_size 当时返回 [u32;2]，
Paint::new 的 contain 比例直接使用 Image 的像素宽高；SVG wire 响应也只有整数
像素尺寸。因此需要区分布局/自然尺寸与 raster extent，贯通安装、测量和
绘制，不应仅将 viewport 的 fract 拒绝改为 round。现有小数拒绝暂时保留，不将
错误裁切/拉伸当作兼容完成。这次基线实测本身无生产改动；后续 v203 见顶部。

## v201 已落实的生产前置（历史）

内部版本 v0.2.33.201 将现有 WebP 真正接到 surface_image_worker_host：Job 自检、
有界请求、Zeroizing 输入、renderer 回调与完整 PBGRA 帧发送由共用宿主拥有。
surface_image_worker_wire 统一 16 MiB 输入、64 MiB 像素和请求/响应数值检查；
父侧 WebP 仍额外比较 RIFF 预检尺寸，不放宽原格式约束。进程 owner 作为根模块
surface_image_worker_process 唯一编译，沿用原文件及同一个 CAPACITY/Job launcher。

这是无新 codec 的职责提取：v1 magic、私有 CLI 入口、15 秒等待、128 MiB Job、
两 child 共享预算、成功退出/尾随数据/取消门均保留。WebP renderer 仍负责 ICC 和
预乘，host 只发送已预乘像素。新增共用帧/长度测试在分配或写成功 header 前拒绝
无效尺寸、溢出、超限和错 count。SVG 的请求类型、viewport、字体和资源策略尚未
接入，不能因共用层完成就开放 SVG MIME。下一步不应再次复制/提取 launcher。
重构前后 WebP 专项均 18 passed / 3 private ignored；共用层专项 6 passed。
最终全量 2698 passed / 48 ignored（120.85 秒），startup 7 passed（19.09 秒）；
Clippy all-targets -D warnings、fmt、3128 文件严格行数、diff 通过。48 ignored 中
四个私有 child 由父测试实际调用，原 44 项专项未执行；UIA 偶发问题不因通过称为根治。
证据与源码/日志哈希归 linshi/otter-v201-evidence.json。

## 为什么不能只放开 MIME

v200/v201 的 surface_image_data 明确拒绝 SVG 声明，surface_image_format 也阻止该声明回退
为位图；当时 surface_image_decode 尚无 SVG 分流。非 base64 data URL 的字节解码已经
实现，不能误列为需要从零开发。真正的缺口是安全 SVG 解析、自然尺寸、呈现及生命周期。

旧宿主使用 img，不是将 SVG 插入 DOM。必须按这个上下文核验脚本、外部资源与动画；
不能套用顶层 SVG 网页的权限，也不能把候选库的默认行为当旧产品合同。

## 当前浏览器实测

隔离 HeadlessChrome 155.0.0.0，无用户 profile。12 个基础 SVG 使用 img.decode 后
drawImage/getImageData；另外 5 个比例案例和 1 个动态 SVG 经过独立采样。
数据来自 linshi/otter-svg-browser.json 和 otter-svg-dynamic.json。

| 来源属性 | 浏览器自然尺寸 | 重要观察 |
| --- | --- | --- |
| width=80 height=40 | 80×40 | 红绿矩形有 3200 个非零 alpha 像素 |
| viewBox=0 0 80 40，无宽高 | 300×150 | 不是 viewBox 的 80×40 |
| 无宽高、无 viewBox | 300×150 | 80×40 矩形仍只占 3200 像素 |
| 只有 width=80 | 80×150 | 未指定轴不能从内容包围盒推导 |
| 只有 height=40 | 300×40 | 同上 |
| width=50% height=50% | 300×150 | 该样本绘制矩形占 12800 像素，不能仅更改返回尺寸 |
| width=1in height=0.5in | 96×48 | CSS 绝对单位基线 |
| square viewBox=0 0 40 40 | 150×150 | 仅比例按默认区域容纳 |
| tall viewBox=0 0 40 80 | 75×150 | 同上 |
| wide viewBox=0 0 120 40 | 300×100 | 同上 |
| width=80 + square viewBox | 80×80 | 确定轴和比例共同决定自然尺寸 |

文字样本有 556 个非零 alpha 像素。foreignObject 中 XHTML 红色区域确实绘制为
80×40；不能当作不受支持的死内容直接忽略。动画样本挂入 DOM 后，每 100 ms 采样，
得到 [255,0,0,255] → [99,0,156,255] → [0,0,255,255]，证明 img 内 SVG 动画实际运行。

脚本样本未改变宽度、未发起 sentinel fetch；外部 image 样本也没有请求到受控
otter-svg.invalid 路由。外部 image 仍绘制出失败标记，整个样本非零 alpha 数是 1459，
而只剩其 10×10 红矩形时应为 100；“不联网”和“视觉上当元素不存在”并不相同。
这些结论仅覆盖实际夹具，不冒充全部脚本/网络/CSP 标准或恶意 SVG 安全验收。

## 原生候选实验：resvg/usvg 0.48.1

初始实验仅在 linshi/otter-svg-audit 建立临时 Rust package，当时未修改 native/Cargo.toml/lock。
固定 resvg=0.48.1，关闭默认 features，仅启用 text/raster-images；不开 svgz、
system-fonts、memmap-fonts。该包解析到 63 个 registry packages，2026-10-22 对
精确版本执行 OSV querybatch，返回零命中；这不是仓库全锁文件扫描、许可证/SBOM
或生产依赖验收。上游声明只支持静态 SVG，不支持动画：
[resvg 项目说明](https://github.com/linebender/resvg)。

实验复用相同输入，输出尺寸和非零 alpha 数归 otter-svg-prototype.stdout.log。
临时程序只处理本轮自建夹具，没有接入生产 Job launcher；不作为恶意输入、OOM、
截止时间或进程权限隔离的验收。对应能力仍须通过实际共用 child 路径验证。

- 显式尺寸矩形和绝对单位样本的尺寸/覆盖量相符；尚未做全部逐像素对照。
- 默认配置的 default_size 字段是 100×100，但**实际**无尺寸样本被内容包围盒还原
  成 80×40。将该选项调为 300×150 后，这组样本仍是 80×40，单改配置无效。
  viewBox-only 也保持 80×40，与浏览器 300×150 不符。
- 未装载字体时文字完全消失（0 像素）；实验只装载固定受信任 Windows Times New
  Roman 文件后为 373 像素，仍不等于浏览器 556。尚未核对具体默认字体/抗锯齿，
  不能把一个固定字体冒充完整字体选择、中文 fallback 或浏览器一致性。
- foreignObject 结果完全透明；动画仅静态初始态。候选库本身不能完成这两项旧能力。
- 默认 image href resolver 允许本地文件。用本轮自建的 8×8 洋红 PNG sentinel，
  默认 resolver 实际绘制 64 像素；显式拒绝 string resolver 后为 0。没有访问用户文件。
  这证明 Job/空环境不能代替资源权限控制，不只是根据文档推测。

上游解析源码还表明 from_data 在未开 svgz 时拒绝 gzip，from_str 默认允许 DTD。
外部实体行为、实体扩张限额、嵌套 data SVG/位图、滤镜中间缓冲仍需独立探测。
不能依据“Rust”或零 OSV 命中推断安全。所有生产 XML/树解析必须在受限 child 内。

## 已确定的实现方向与职责

1. 共用现有受限 image child process owner，不复制 CreateProcess/Job/句柄白名单，
   不为 SVG 新建独立容量池。总预算仍由一个 owner 控制。
2. 提取 worker host 与 WebP renderer，保留 WebP RIFF 检查和现有 ICC/EXIF 行为。
   SVG 不能经过 RIFF inspect，也不能在父进程用 XML/正则预解析自然尺寸。
3. 请求使用固定类型和版本，响应先送尺寸/字节数。父侧在 pixel_buffer 分配前独立
   验非零尺寸、checked_image_bytes、64 MiB 和精确 count；再沿原完整读取/成功退出/
   尾随输出拒绝/取消/Job Drop 回收。不能让 child 返回多少就分配多少。
4. SVG renderer 必须有独立 intrinsic viewport owner。浏览器 natural size、SVG 内部
   viewport 和最终 raster target 不是同一量；保持比例/百分比/单位语义，不能只拉伸
   已按错误内容包围盒栅格化的图。后续还须覆盖布局尺寸和 DPI 改变的重绘质量。
5. 只允许原已授权字节内的资源；默认禁用任意字符串 href 的本地/网络读取。
   data image 保留独立递归/输入/工作内存预算。字体来源由宿主受信任策略控制，
   不能接受 SVG 提供的路径，也不能加载全系统字体后忽略 128 MiB Job 限额。
6. rasterizer 输出是否预乘必须明确；tiny-skia 输出是预乘 RGBA，接入时只换为 BGRA，
   不能再次沿 WebP 的 straight RGBA 预乘路径。仍沿原 Ready/当前 mount/Grant 安装门。
7. 静态 SVG 可作为增量，但不能将 foreignObject、动画、文字、失败标记差异从总合同
   删除。它们必须继续实现或取得用户明确的范围调整，resvg 不能被称为完整替代。

共用 child host 在 v201 落地，v202 首批接线见本文顶部。整个 SVG 阶段尚未完成；
不能把缺失内容当作成功图片，也不能把首批静态支持当作完整迁移。
