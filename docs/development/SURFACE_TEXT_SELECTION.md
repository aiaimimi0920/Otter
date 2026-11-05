# 原生 Surface 可选择文字迁移

## v179 每帧一次解析高亮范围

确认正式 Text 命令已持有 TextFrame 索引，旧路径却在每次高亮时重新线性查找
anchor、focus 和当前文字。现于每次 PaintFrame::draw 开头解析成四个局部标量，
逐项用已有 index 常数时间投影；本次范围投影总遍历从平方级降为线性级。
没有常驻缓存、逐节点数组或跨 reflow 的索引；实际高亮仍在原 Text 命令的 clip/
layer 内、文字之前绘制，保留不可选过滤、空范围、反向及同节点归一化和画刷恢复。

旧算法仅保留在 cfg(test) 作为独立 oracle，穷举测试比较正反端点、同节点折叠、
不可选项、空文字和缺失 owner。显式 ignored 微基准分别运行旧算法与新投影，
64/256/1024 节点各预热一次、采样五次，每次投影二十轮，校验覆盖长度相同。
这只测 debug 构建的范围计算，不包含 DirectWrite 矩形生成、D2D/GPU、总帧时间、
输入延迟或进程资源；不能将本结果当 N11 整机性能验收。

同一最终测试二进制中的 20 轮中位耗时如下（毫秒，预热一次后五次采样）：

| 文字节点 | 旧独立扫描公式 | 新每帧投影 |
| --- | ---: | ---: |
| 64 | 1.3505 | 0.0396 |
| 256 | 18.3899 | 0.1491 |
| 1024 | 284.5251 | 0.4436 |

初始改动前日志 `linshi/otter-v179-range-before.log` 保留；同二进制比较日志
`linshi/otter-v179-range-after.log` 包含全部样本和相同覆盖长度断言。源码静态
复核未发现语义回归；不能将上表直接换算成全应用加速或承诺帧率。
`linshi/otter-v179-range-evidence.json` 记录测试 EXE、关键源码及两份日志 SHA256
和实际 rustc 版本。最终可见全量 2589 passed/44 ignored（96.32 秒），包含独立
oracle 与正式 WIC/D2D 三节点高亮像素检查；新增 ignored 微基准已另行显式通过。
startup 7 passed（19.77 秒），Clippy、fmt、3071 文件严格行数和 diff 检查通过，
验证后自有 native 进程为零。

## v178 跨节点上下扩选与全局列保持

Shift+上下先使用 focus 的实际 DirectWrite 相邻行；到首末行才进入场景序相邻
可选择节点的末行/首行。跨 owner 时将旧局部 x 转成全局列，再转换为新局部 x，
短行和左/右边缘命中不丢失列意图。原导航 anchor 不变，范围不越出当前 Surface。
DirectWrite 新内部结果明确区分“实际换行”和“到边界”，避免软换行共享 UTF-16
偏移时误跨 owner；原 vertical_text 接口仍保留。失败前完成目标行及 hit-test，
成功后才一起提交 focus owner、偏移和列探针，不增加常驻缓存。

实际旧组件明确使用 whiteSpace=pre，三种布局的跨节点/内部多行/返回共 15 项
端点断言通过，原始证据在 `linshi/otter-cross-vertical-baseline-v178`。初始默认
whiteSpace=normal 的报告另存 normal-whitespace-report.json：换行被折为空格，
不能用来证明多行合同。原生夹具最初漏根 relative 定位导致布局失败，已修正；
没有放宽端点断言。新增三项测试覆盖三布局多行、软换行同索引 moved 标志、空
布局和 LF/CRLF 末尾空行，已有 HWND 路由测试增加上下跨节点与 keyup 断言。

旧浏览器可越过 Surface 进入页面其他文字；本次仍限定单 Surface owner，不能
把此边界宣称完全等价。跨滚动层、复杂 bidi、空 owner 跨节点实际命中、全局
Ctrl+A、系统输入和 UIA TextPattern 仍待验证/实现。只读审查未发现确认新缺陷。
最终可见串行全量 2588 passed/43 ignored/0 failed（96.43 秒）；startup 7 passed
（20.05 秒）。Clippy、fmt、3070 文件严格行数及 diff 检查通过；12 个相关文件
UTF-8 无 BOM，验证后自有 native 进程及探针监听为零。

## v177 跨节点水平键盘扩选与行端点

真实旧组件三布局 24 个键盘场景与六次端点连续扩选，完成 54 项水平/行端点断言。
普通 ArrowLeft/Right 不折叠跨节点选区；Shift+左右操作 focus，在首尾按场景顺序
进入前一节点末尾或后一节点开头，不额外吞掉字素。Shift+Home/End 跟随 focus
所在行，而非 anchor 布局。证据在 `linshi/otter-cross-word-baseline-v175` 的
keyboard-report、keyboard-edge-report、verify-keyboard 和 keyboard-evidence。
DOM Range 仅用于坐标；上下键结果已记录但依布局变化且可跨出 Surface，未将其
提升为统一端点规则。不是完整 Tauri、复杂 bidi 或所有布局键盘等价证明。

原生据此接入 focus owner 水平步进与 Home/End。跨节点和同节点起始均可 Shift
步进到相邻可选择文字，跳过不可选项，范围限制在当前 TextFrame；原导航锚点
不变，仍拒绝别的焦点节点、活动拖动和已替换场景。返回同节点恢复文字复制。
普通跨节点左右键保持放行且不修改选择。上下跨节点仍未实现；新增独立 readiness
使其 keydown/keyup 都放行，修复只读审查发现的 up/down 消费不对称问题。
新增四项测试覆盖反序、边界往返、不可选节点、Home/End 和 HWND 输入/copy 隔离。
最终可见串行全量 2585 passed/43 ignored/0 failed（88.24 秒），日志
`linshi/otter-v177-final-full.stdout.log`；Clippy、fmt、3070 文件严格行数通过。
startup 7 passed（13.20 秒），diff 检查通过，验证后自有 native 进程为零。

## v176 真实词拖动基线与初始空白阶段修正

实际旧 DeclarativeSurface + useShortcuts 的隔离 Vite 夹具，使用 Edge 155.0.4283.45
完成 column、row、absolute-reverse 各首词/内部词共六次真实鼠标手势；DOM Range
仅计算字形坐标，不设置选区。30 项断言证明：初始双击包含尾随空格，但移动后
跨节点端点及反向 anchor 收到词干边界，返回初始词不重新包含尾随空格。
例如 middle 初始 6..13，跨 c 的 final 为 b:6→c:13，反向为 b:12→a:6，
返回为 b:6→b:12。复制事件被拦截，不发布剪贴板；最终同节点复制走浏览器。
证据目录 `linshi/otter-cross-word-baseline-v175` 包含 probe、原始 report、验证脚本
和 source hashes。六次手势不是完整 Tauri 或全部 Unicode/bidi/空白命中的证明。
另补七项阶段探针：不移动直接抬起保留空白，横向移动 0.1、1、2、3、4、6 CSS
像素均转为词干范围；原始 phase-report.json 保留，合计 37 项断言通过。原生测试
使用同组偏移核对阶段切换。最终页面 console error 为零，隔离浏览器与服务器已关闭。

修复 v175 把尾随空白规则用于整个拖动的问题：Layout 分离 word_segment 与
初始 word_range，WordDrag 保存词干锚点和初始坐标。未移动抬起保留初始空白；
一旦移动即使用词段，返回原点也不会恢复空白。新增真实命中坐标测试覆盖非首词
跨节点正反向、回到初始节点左右侧和原词，保留旧 HWND 无移动双击测试。
空白自身命中、系统输入、复杂 bidi 和性能仍待专项验证，不宣称全量等价。
最终可见全量 2581 passed/43 ignored（77.22 秒），startup 7 passed（12.14 秒）；
Clippy、fmt、3069 文件严格行数和 diff 检查通过。独立只读审查未发现确认新缺陷。

## v175 跨节点词级拖动

词手势现允许切换 focus owner，使用目标文字自己的 ICU 边界表；保留双击时的初始
词范围，正向以初始词起点为锚点，反向以其终点为锚点。返回原节点恢复同节点扩展。
只保留当前 focus 的一份有界表，切换时先查询成功再替换，不按经过节点数量积累。
普通拖动命中同一节点时也不再重复分配 owner 字符串。跨节点复制仍拒绝聚合。

新增测试覆盖场景/视觉反序、中文与 emoji UTF-16 归属、返回初始词及 HWND 捕获、
抬起和 click/copy 隔离。独立只读审查未发现确认缺陷；跨节点词手势的旧浏览器
精确范围对照、高频长文字性能、ICU 故障注入和内部词多次往返组合仍缺专项证据。
跨节点键盘扩选/折叠仍未完成，v173 系统双击前台保护失败仍保留；不宣称完全等价。
最终可见全量 2580 passed/43 ignored（77.47 秒），startup 7 passed（13.20 秒）；
Clippy、fmt、3069 文件严格行数和 diff 检查通过。

## v174 跨节点普通拖选端点与逐项高亮

唯一 Selection 现区分 anchor 的 node 与 focus_node，普通拖动命中另一个 selectable
文字时更新 focus owner。按当前 TextFrame 的场景先序投影起点部分、中间完整节点和
终点部分，不用局部索引直接 min/max，也不按视觉位置排序；逐 TextItem 的正式 D2D
绘制调用使用相同范围。反向拖动和返回原节点保留端点语义；返回同节点后恢复文字
复制。跨节点 selected_text 明确返回 None，不聚合文字、不发布剪贴板。

抬起以“不同节点或不同局部索引”判断有效选择，避免相同偏移跨节点时误发祖先
click。重排校验比较端点闭区间的节点顺序、成员、selectable 与规范化文字；范围内
变化则清理旧选择。自动滚动和显露查询改用 focus owner，不再错误操作 anchor 的
滚动祖先。每个绘制项当前需要线性定位端点，复杂场景帧耗时尚未测量，不能保证
常数成本或宣称性能验收通过。

本增量接入普通鼠标拖选；跨节点词级手势和键盘扩选/折叠仍未完成。词手势暂保留
自身节点的边界表，键盘对跨节点选区拒绝沿旧单节点索引路径处理，不能把这项保护
当成最终键盘行为。空白区域的最近端点、复杂 bidi、离屏多层滚动仍需专项补证。
不把该增量当作完整跨节点迁移或全局 Ctrl+A 已完成。v173 系统双击保护失败照旧保留。
验证：新增 5 项进入最终全量 2577 passed/43 ignored（81.14 秒），其中实际 WIC/D2D
像素覆盖三个文字节点并在清除后恢复原图；范围成员/顺序/文字/权限变化均有失效
断言。独立只读审查未发现确认正确性缺陷，绘制遍历成本仍需性能证据。

## 2026-10-21 跨节点布局与遮挡基线

使用实际 DeclarativeSurface、样式映射、CSS 和快捷键模块，完成 6 种布局的正反向
实际鼠标拖选，共 12 项断言通过。DOM Range 只用来计算字形位置，不设置 Selection；
同时记录 elementFromPoint 命中、anchor/focus 节点与偏移，以及 Ctrl+C 路由。

| 布局 | a 起点到 c 末尾 | c 起点到 a 末尾 | 复制路由 |
| --- | --- | --- | --- |
| column、row、stack-disjoint | Alpha / Bravo / Charlie，以换行分隔 | 换行 + Bravo + 换行 | Unit copy |
| absolute-reverse（视觉 c,b,a，节点 a,b,c） | AlphaBravoCharlie | Bravo | Unit copy |
| nested-excluded（中间含不可选文字及 input） | Alpha / Bravo / Charlie | 换行 + Bravo + 换行 | Unit copy |
| stack-overlap（仅顶层 c 可命中） | Charlie | Charli | 浏览器文字 copy |

反向操作端点是 c 的 0 到 a 的 5，所以不含两端文字，不是“反向全选丢字”。
绝对定位的结果证明不能按视觉 x/y 重排阅读序列；stack-overlap 虽以 a 的字形坐标
为目标，实际命中 c，选区两端也都在 c，不能穿透遮挡建立 a 端点。不可选文字和
input 值未进入选择结果。row 的可见文本分隔符也不能凭横排猜成空格。

对原生实现的约束：保留独立 anchor/focus 节点和 UTF-16 偏移，范围顺序使用场景
节点顺序，不取空间排序；逐 TextItem 绘制范围，但起点必须通过现有最上层命中。
跨节点复制仍不得聚合文本，不能因这份基线擅自改变快捷键门禁。本次没有验证
多层滚动离屏端点、复杂 bidi 或完整 Tauri 外壳，不能推导所有布局/键盘行为。

证据目录：`C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-cross-node-layout-baseline-v173/`。
report.json 保存 12 项结果，evidence.json 保存五个真实源码文件哈希；fixture.tsx、
server.mjs、probe.js 可复跑。copy 默认发布被阻止，没有系统剪贴板读写。结束时
专用浏览器与服务已关闭；本轮仅补合同与台账，不改生产代码或内部版本。

## v173 系统双击探针与下一跨节点边界

新增独立 ignored 系统输入探针，并在 Invoke-NativeTouchProbe.ps1 中加入显式
`-RunInteractive -Probe text-double-click`。复用自有窗口命中/前台/输入空闲保护，
完整单批 down/up/down/move/up 由系统生成双击；消息之间消费第一次普通 click，
但不发送远端动作。观察器识别带专用标记的 WM_LBUTTONDBLCLK，检查初始词、捕获、
最终扩选及只产生一次普通 click。没有绕过窗口保护或直接合成双击消息。

首次真实运行 exit=101：前台 HWND 与自有 HWND 不同，在发送任何输入前被保护
拒绝；因此**未证明系统双击路径通过**。失败证据保存在
`linshi/otter-native-text-double-click-20261021-141654-338639ba/`。后续只读查询
观察到前台为 Tabby，不能据此推断失败瞬间的全部抢焦原因。补充记录激活 API 返回值、
立即/消息泵后前台身份；没有强抢前台、修改系统双击时间或重试直到通过。

下一跨节点改动的当前依赖已核对：TextFrame.items 是布局 Tree 稳定先序的子序列，
Tree 按 children 原序深度优先，包含 absolute 节点原位置，不按屏幕坐标重排。
Scene 本身只保存树，没有现成的平铺 DOM 顺序数组；parse 验证栈不能当阅读序列。
PaintFrame 已逐 TextItem 调用 selection.draw，可复用高亮入口；需共同调整唯一
Selection 的节点端点、reconcile、键盘、word、reveal、autoscroll，不能只改 draw。
这只是实现依赖核对，row/stack 的浏览器合同仍需证据；不扩大跨节点文字复制权限。

## v172 原生双击与单节点词级拖动

Surface 注册 CS_DBLCLKS，并将 WM_LBUTTONDBLCLK 接入既有文字选择路径；实际
DirectWrite 命中得到 UTF-16 插入位置，再按系统 ICU 边界建立词范围，包含水平
尾随空白而不吞换行。拖动保存初始词范围，向前/反向按词扩展，复用现有捕获、
自动滚动、绘制及本地复制。第一个普通 click 的既有单槽投递不撤销；若尚有待消费
click，保持旧的拒绝新按下规则。第二次按下的词选择不会另发祖先动作。

启用双击窗口类也改变非文字输入的消息形式，因此 pointer 普通内容及 scrollbar
都将双击消息作为原有按下处理，保留普通 click 和滚动条优先级。文字分词失败时
沿既有错误/取消路径收尾，不回退为祖先点击。取消、撤权、换场景与关闭复用现有
唯一选区清理；完成时释放词边界表，键盘继续沿原有字符/行导航，不滞留词级模式。

surface_text_word_icu 仅按需使用 LOAD_LIBRARY_SEARCH_SYSTEM32 加载 icu.dll，
不添加静态 ICU 导入、Cargo 依赖、后台线程或全局词缓存。文本和边界数组有界至
65536 UTF-16 单元及最多 65537 个边界；iterator 先于文本和 DLL 释放。每次开始
词手势查询一次，后续移动二分边界并扫描尾随空白；未证明最大输入下的 UI 时延。
缺 DLL/导出按操作失败报告，不用 ASCII 假实现；旧系统缺库运行路径尚未实机验证。

本增量仍是单节点词选择，不代替跨节点视觉选区。浏览器 emoji 样例可能包含前一
片段，本实现按完整 ICU segment 选择，末端也不跨节点；这些差异继续保留为迁移
未完成项，不将 Unicode 样例或直接 HWND 消息测试宣称为完整浏览器/物理输入等价。
验证：新增 8 项随可见全量 2572 passed/42 ignored 通过，startup 7 passed；
Clippy/fmt/严格行数/diff/UTF-8 检查通过。独立只读审查未发现确认缺陷；静态错误
清理审查不替代缺 DLL/导出/ICU 错误的故障注入，后者仍需补证。

## 2026-10-21 系统 ICU 候选实测

通过系统目录加载 icu.dll，实际调用 C API ubrk_open/first/next/close，得到本机
ICU 72.1.0.4 的词边界。沿用前一节 15 类文本，und/en_US/zh_CN/th_TH 四种 locale
共 60 组边界断言通过；中文 `你好` / `世界`、泰文 `ภาษา` / `ไทย` / `ทดสอบ`
均与浏览器观察中的词干相符，不再出现 SelectableWordsSegmenter 的逐字差异。
组合字符与 ZWJ 家庭样例没有被拆开。空文本、65536 UTF-16 单元上限、65537 单元
拒绝也已实测。上限耗时仅保存在原始报告，不作 UI 热路径性能保证。

这只确定 ICU **可作为候选词边界基础**，并非完整双击等价：ICU 将空白单列，浏览器
选择可含尾随空格；浏览器 emoji 样例还可能包含前一个片段，末端可跨节点。因此不能
直接将一个 ICU segment 映射为最终 Selection，也不能删掉上述反例。需要将实际
命中/插入位置、词范围扩展和可见文本映射分层验证，再接 HWND 双击及词级拖选。
当前仓库文档没有查到明确的最低 Windows build；不能仅凭 Windows 10 文案静态
引入 1903 才有的 icu.dll。生产接入须显式处理系统 DLL 可用性及缺失错误，避免启动
加载失败；此轮没有新增产品依赖或提高最低系统版本。

复跑脚本为前述 word-selection 证据目录的 compare-icu.ps1，icu-report.json
包含系统/ICU 版本、60 组词边界及上限断言。探针显式保持 UTF-16 缓冲存活至
ubrk_close 之后，finally 释放 iterator 与缓冲；只在临时目录编译 C# 调用层。
[ICU C API 定义](https://unicode-org.github.io/icu-docs/apidoc/released/icu4c/ubrk_8h.html)
用于核对 ABI、索引与关闭合同。未改 Rust 生产代码，内部版本仍为 v0.2.33.171。

## 2026-10-21 双击 Unicode 分词候选排除

继续使用实际 DeclarativeSurface 的隔离 Edge 夹具，采集 15 类文本、288 个真实
鼠标双击位置：每个可见 grapheme 的 1/4 与 3/4 位置分别命中，DOM Range 仅用于
计算目标坐标，不设置选区。记录 anchor/focus 的节点身份与 UTF-16 偏移，避免把
末端跨节点换行误当成单节点逆向范围。浏览器 155.0.4283.45，navigator.language
为 zh-CN；这是本机观察，不是不同语言/系统版本的普遍保证。

覆盖 Latin、连续空白、标点、撇号、连字符、下划线、小数、组合字符、emoji、ZWJ
家庭、中文、混合脚本、NBSP、阿拉伯文和泰文。观察到 `can't `、`foo_bar `、
`12.34 `、`café ` 保持整体；中文产生 `你好` / `世界 `，泰文产生
`ภาษา` / `ไทย` / `ทดสอบ`。实际命中字符的左右半部会改变结果；最后字符右侧
可能选择节点间换行，不能仅根据 grapheme 索引指定词。连续空格还受 CSS 折叠影响。

同时实际调用 Windows.Data.Text.SelectableWordsSegmenter，分别使用 und、en-US、
zh-Hans、th-TH，共 60 个文本/语言组合。**不能直接将该接口作为浏览器等价实现**：
und 将上述中文逐字分段；本机 zh-Hans 返回 `你` / `好世界 `；泰文结果也与浏览器
不同。ASCII 样例相同不足以证明 Unicode 等价。保留原始输出，不用按空格切分、
按脚本猜语言或给这些反例写特判来掩盖差异。

[SelectableWordsSegmenter 官方说明](https://learn.microsoft.com/en-us/uwp/api/windows.data.text.selectablewordssegmenter)
明确依赖系统可用的语言规则；[ScriptBreak 的 fWordStop](https://learn.microsoft.com/en-us/windows/win32/api/usp10/ns-usp10-script_logattr)
是词级键盘移动位置，也不能未经实测当成双击边界。下一候选为系统 ICU 的词边界，
但尚未实测或选定；[Windows ICU 版本边界](https://learn.microsoft.com/en-us/windows/win32/intl/international-components-for-unicode--icu-)
要求区分 1703 的双 DLL 和 1903 才有的 icu.dll，不得静默提高支持版本或增加加载失败。

证据目录：`C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-word-selection-baseline-v171/`。
browser-report.json 保存全部命中，native-report.json 保存 60 组系统分词结果，
evidence.json 保存覆盖断言及实际组件/CSS 哈希；fixture.tsx、probe.js、server.mjs、
compare-native.ps1 可复跑。覆盖数量及中文/泰文关键观察断言通过；不是 288 项原生
兼容性测试通过。未读写剪贴板，专用浏览器和服务已关闭。未修改生产代码或版本；
双击选择仍待实现，跨节点视觉选区需求不变。

## 2026-10-21 旧组件的真实浏览器选择基线

使用仓库实际 DeclarativeSurface、CSS、useShortcuts 与 ShortcutManager，在隔离
Edge 155.0.4283.45 中运行 6 项断言。夹具使用 unit-selected 上下文，只把最终
Unit copy 回调替换为计数器，并阻止浏览器 copy 默认发布，未读写系统剪贴板。
不是完整 Tauri/WebView2 外壳验收，也不修改产品代码或内部版本。

| 操作 | 实际范围及复制路由 |
| --- | --- |
| 同节点 Range 后 Ctrl+C | Alpha one；浏览器 copy 1 次，Unit copy 0 次 |
| 跨节点 Range 后 Ctrl+C | Alpha one 与 Beta two，以换行分隔；Unit copy 1 次，浏览器 copy 0 次 |
| 实际鼠标跨节点拖选后 Ctrl+C | 同上，跨过不可选文字和 input，不包含其文本/值 |
| 点击 text 后 Ctrl+A/C | 选择包含夹具 Surface 外部段落；Unit copy 1 次，不是场景文字聚合复制 |
| input 中 Ctrl+A/C | 输入值 0..14 全选，浏览器 copy 1 次，Unit copy 0 次 |
| text 双击后 Ctrl+C | 选择 `Alpha `（含尾随空格），浏览器 copy 1 次，Unit copy 0 次 |

这证明跨节点**视觉选区**确实是待迁移差异，不能因复制门禁而删去；但现有产品
没有把该选区交给文字复制，不能擅自扩展为跨节点文本聚合发布。Ctrl+A 的范围也
不是当前单节点或当前 Surface。后续要分别保持视觉范围、快捷键路由和控件隔离，
而不是用单节点 Ctrl+A 代替全部行为。当前 column 夹具结果不定义 row/stack、绝对
定位或完整应用外壳的阅读顺序；这些仍需补测。原生尚未处理 WM_LBUTTONDBLCLK，
词级双击选择是已经观察到的明确生产缺口。

证据目录：`C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-text-selection-baseline-v171/`。
`report.json` 保存浏览器版本、六项结果和四个实际源码文件的 SHA-256；`fixture.tsx`、
`server.mjs`、`probe.js` 可复跑，`selection.png` 为结束时截图。专用浏览器及本地服务已关闭。

## v171 实际系统 timer 消息证据

新增独立测试从自有 HWND 的 Win32 队列取得 WM_TIMER，再经 DispatchMessageW
进入正式窗口过程，不改 due、不合成 timer 消息。静止指针的选择随系统调度扩展，
松键后排空既有消息，再观察 200ms 不产生后续 timer、不改变选区或滚动。
另一用例先以 PM_NOREMOVE 保留系统生成的消息，再取消/撤权并派发旧消息，验证
旧身份不能改变状态。这符合 [KillTimer 的消息队列边界](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-killtimer)：销毁 timer 不会清掉已入队消息。
起止鼠标仍是直接 HWND 消息，不宣称系统鼠标注入、物理设备或负载下调度时延验收。

后续选择范围的基线核对：`src/platform/desktop/useShortcuts.ts:61-81` 的现有
Surface 复制门禁要求 anchor/focus 属于同一个 selectable root；现有
`tests/unit/DeclarativeSurface.test.tsx:407-422` 也只验证同一 text.firstChild 范围。
因此不能把“跨节点/全选待完成”解释成已经定义好的全场景复制合同，更不能随意按
绘制顺序拼接文本或把单节点 Ctrl+A 宣称为全场景全选。跨节点 Range 的浏览器行为、
控件隔离、分隔符和实际产品复制链仍需补基线证据，再决定对应迁移或合同扩展。

## v170 捕获拖选的边缘自动滚动

鼠标拖到当前文字可见 clip 的边缘时，沿可消费的 auto 轴启动唯一 HWND timer；
边缘带最多 12 DIP（小区域不超过四分之一），每 50ms 每轴最多移动 16 DIP，
不追赶积压 tick。复用正式 Scroll::wheel 的内层优先/边界交外层规则，hidden 轴
不会获得鼠标滚动资格；缓存投影后用同一鼠标客户坐标更新选区，并同步原生 EDIT。
不重新测量文字、不复制文本、不建立后台线程；当前仍是单节点选区，不冒充跨节点选择。

仅边缘有滚动候选时保留 timer；回到内部、到边界、松键、取消、失焦、撤权、换场景
或关闭均停止。消息使用最后一次捕获鼠标坐标，静止时也继续扩展；迟到 timer 身份
不能复活旧手势。手势 Rc 身份与滚动 timer 身份独立，pending 查询期间同步取消或
替换后丢弃旧结果，不启动或清掉新手势 timer。候选使文字 owner 完全不可见时不提交。
词边界、全选、跨节点、bidi 水平视觉导航、UIA TextPattern 与物理鼠标验收仍待完成。
验证：8 项 HWND 专项与全量 2562 passed/42 ignored；直接消息验证不替代物理
鼠标、系统 timer 调度时延或最大场景性能验收。v169 UIA 旧异常仍按原证据保留。

## v169 行首/行尾扩选

Shift+Home/End 现在把已有选区 focus 移至当前排版行的逻辑首/尾，保留 anchor。
只接带 Shift 的行边界命令；普通 Home/End 仍走原页面滚动，Ctrl/Alt/Meta 门禁
不变，不把单节点行边界操作当作全选或跨节点操作。继续复用自动显露和 EDIT 同步。

边界来自 [DirectWrite DWRITE_LINE_METRICS](https://learn.microsoft.com/en-us/windows/win32/api/dwrite/ns-dwrite-dwrite_line_metrics)：
length 包含尾随空白和换行，End 仅减去 newlineLength，因此保留尾随空格，不把
换行符误纳入本行 End。显式换行、软换行、空中间行、末尾空行分别测试；返回
边沿保留当前视觉行中心 y，重复 End 与 End→Home→Down 不因同索引误换行。
emoji/组合字符行尾使用真实 trailing hit，不拆 UTF-16 范围。

行数组按现有文本长度限制分配，最多 65537 项，操作结束释放；这是 O(行数)
查询/扫描，不是常数成本。本次不增加常驻行缓存，也未做最大文本的按键延迟
性能验收。实际 HWND 消息测试仍不是系统键盘注入或物理键盘验收。
词边界、全选、跨节点、bidi 水平视觉导航、拖选自动滚动和 UIA TextPattern 仍待完成。
验证边界：新增 5 项专项通过；先前全量及隔离 UIA 组出现 Edit/Select 失败。
补充可选采证后，关闭消息跟踪的最终全量 2554 passed/42 ignored；此前间歇性
异常根因仍未确认，不将后续通过当作修复证明，详见迁移台账 v169。

## v168 上下行扩选与视觉行归属

Shift+Up/Down 现在使用实际 DirectWrite 行边界及字符命中扩展已有选区，重复
keydown 连续移动，keyup 不重复修改；未修饰 Up/Down 仍交给原滚动路径，不
抢占 EDIT、Ctrl/Alt/Meta 快捷键或其他窗口焦点。沿用本地编辑与选区 owner 准入。

选区只增加一个可选的布局局部 `[目标列 x, 行探针 y]`：经过短行仍保留目标列，
返回长行不会永久停在短行末尾；记录 y 防止软换行行尾/下一行行首共用索引时
跳行。鼠标创建和更新选区也保存实际命中的 caret 边沿/视觉行，覆盖首次上下
操作，不能只从 UTF-16 索引重新猜行。显露路径使用同一行探针，不另认成下一行。
水平键重新选择导航列，成功 reflow 重置旧几何点，纯滚动投影则保留；换场景、
撤权、取消沿原生命周期清理。无额外文字副本、布局缓存或定时器。

测试使用实际 DirectWrite 和自有 HWND 消息，覆盖长短行往返、软换行连续导航、
鼠标建立后的首次导航、Unicode cluster、重排重置、重复键与自动显露。线程键盘
状态通过 RAII 恢复，不是系统键盘注入或物理键盘验收。bidi 水平视觉方向、词
边界、全选、跨节点、鼠标越界自动滚动及 UIA TextPattern 仍未完成；不宣称混合
字体、全部复杂脚本和浏览器默认选择行为已逐项验收。

## v167 键盘选区端点自动显露

Host 在水平键盘选择成功后显露当前端点所在的实际 DirectWrite cluster；正向
扩展使用最后选中 cluster，反向或折叠使用当前边沿所在 cluster。不把整个长
文本框当显露目标，也不按固定字宽或行高猜测。空文本、活动拖选不启动此路径。

Scroll::reveal_region 复用原焦点导航的逐祖先 nearest 算法；内层消费后修正目标
位置，再处理外层。先克隆滚动状态并投影缓存，只有端点与投影后 TextItem clip
相交才提交；零高祖先或文本自身非零静态裁剪使端点完全不可达时，不误滚外层。
这是至少部分可见保证；cluster 大于可用视口时不能声称整个 cluster 完整可见。
hidden 轴仍可通过键盘显露，但不因此获得滚轮资格。

只在滚动实际提交后同步现有 EDIT 几何/region；不改变其草稿，不另建窗口。
回归检查滚动前后真实 HWND region、父焦点及 draft，不能用 WS_VISIBLE 代替
裁剪可见性。初始测试误用了 WS_VISIBLE，核实既有 SetWindowRgn 实现后仅修正
测试判据。纯显露复用布局投影，测试证明反复显露不增加文字测量次数。

本次只接键盘路径，不等于鼠标拖选越界自动滚动；bidi 视觉导航、上下行/词扩选、
全选、跨节点及 UIA TextPattern 仍未完成。没有系统输入注入或剪贴板写入。

## v166 水平键盘选择接线

已有单节点选区或折叠锚点可通过 Shift+Left/Right 沿文本逻辑 cluster 扩展、缩回；
重复 keydown 连续移动，keyup 和 WM_CHAR 不重复修改。普通 Left/Right 在非空
选区上折叠到范围边沿，折叠后普通方向键仍交回原页面滚动。未抢占 Shift+Tab。
此为原生键盘交互增量：前端 span 仅配置 user-select:text，无专属键盘处理器，
不能据此宣称已测得浏览器全部默认选择语义一致。

Layout::step_text 复用 [DirectWrite HitTestTextPosition](https://learn.microsoft.com/en-us/windows/win32/api/dwrite/nf-dwrite-idwritetextlayout-hittesttextposition)
取得完整 cluster 首尾，不按 UTF-16 单元或平均字宽移动；检查范围、覆盖关系、
进度和 surrogate 边界。无额外全段文字复制、几何数组或常驻缓存。
Presentation 仍拥有唯一选区；Host 同步 viewport 后操作；HWND 只允许实际父
窗口焦点、本地编辑准入及当前 keyboard.anchor 与选区 node 一致时处理，活动
拖选、换场景或撤权不会复活。处理完成后沿旧路径撤销候选，不产生 click/edit。
软停提交不影响本地选择；系统 EDIT 保留自己的键盘输入。

本轮没有完成 bidi 视觉方向导航、上下行/词边界扩选、移动后自动 reveal、全选、
跨节点选择及 UIA TextPattern。实际 HWND 测试使用可恢复的线程键盘状态与
SendMessage，不是物理键盘或 SendInput 键盘验收；没有访问系统剪贴板。

## v165 文字光标与系统鼠标证据

可选择文字现使用 Windows 共享 IDC_IBEAM；光标查询与开始选择复用相同最上层
命中及 DirectWrite 可见字符判断，不创建选区。被按钮遮挡时保留按钮光标，文字
尾部空白及普通文本不抢占；软停提交但允许本地编辑时仍可显示文字光标。

显式运行 `scripts/tests/Invoke-NativeTouchProbe.ps1 -RunInteractive -Probe text-selection`
可复现系统鼠标探针，原 touch 模式默认不变。探针只在自有窗口位于前台且起终点
实际命中该 HWND 后注入，完整 down/up 单批发送；观察器只记录带本探针标记的
真实鼠标消息，不消费消息，不用 SendMessage 代替系统输入，不访问剪贴板。
本机记录为 `[512, 513, 512, 514]`，即 move/down/move/up；选区为
`Native selection`，没有 click/edit 残留，捕获释放，实际系统光标为 I-beam。
证据：`linshi/otter-native-text-selection-20261021-115903-b07aaa45`，1 passed，
1.89 秒。此为 SendInput 系统注入证明，不是物理设备手测或第三方粘贴验收。
只读审查发现部分注入成功后的应急 LEFTUP 原先忽略返回值；已改为最多 3 次、
检查实际插入数量，并新增无系统输入的失败/恢复回调测试。若系统持续拒绝恢复，
显式报错提示检查物理鼠标状态，不能承诺突破系统拒绝。此分支没有在桌面实际
制造卡键；回调验证不冒充系统故障注入。修正后的系统探针再次通过（1.79 秒），
证据 `linshi/otter-native-text-selection-20261021-120415-2894da12`，仍无剪贴板访问。
输入批次及返回数量依据 [Microsoft SendInput 合同](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput)。

## v164 当前接线

Presentation 现在持有一个节点内的 anchor/focus/dragging 选区。按下使用正式最上层
命中，复用 TextItem 的 cluster 定位；拖动可延伸到文字边沿。高亮在该文本绘制
指令前执行，继承现有组透明度、圆角/祖先裁剪及兄弟覆盖顺序，保持原字色。
颜色复用原生焦点的规范信号黄 #d9ff38，以前端选择合同的 42% 透明度派生。

真实 HWND pointer 路径已接：非空选择先结束手势再释放捕获，不产生祖先 click
或表单事件；collapsed 选择回到原有单击路径。取消、活动手势失去捕获、换场景、
撤销本地输入会撤销相应选择。已完成选区可跨窗口 resize、同内容 reflow 和软停
提交保留；规范化文本变化则取消旧索引。Host 暂时被同步回调取出时，通过 Pointer
的单个有界撤销标记在 State::run 归还 Host 前落实，不丢失重入取消。

Ctrl+C 已接本地 CF_UNICODETEXT 发布，不调用远端 click 或数值 FIFO。只允许当前
窗口焦点及本地输入有效时复制非空选区；准备后与持剪贴板锁后复核场景身份和当前
选区文本，清空前失效则保留原剪贴板。编码规范化 CRLF、追加 NUL，并复用既有
HGLOBAL/预算 owner；成功交给系统，失败释放未转移内存。EmptyClipboard 之后的
系统发布失败不能恢复原第三方格式，错误向窗口 owner 返回。
格式依据 [Microsoft CF_UNICODETEXT 合同](https://learn.microsoft.com/en-us/windows/win32/dataxchg/standard-clipboard-formats)。

这些是实际生产接线，不代表所有浏览器选择能力已齐备。键盘扩选/全选、跨节点
选择、双击选词、触摸选择、UIA TextPattern 与物理输入人工体验仍需
继续对照；本轮自动测试不改写真实系统剪贴板，不把替代发布回调当第三方粘贴验收。

## 真实基线与退出条件

`src/features/workspace/services/legacyOcrAttachmentScene.ts` 的 OCR 文本块已经输出
`text.props.selectable=true`；父 stack 的 click 为 `neuro.official/ocr.copy-block`。
这不是新设计的附加功能，而是 N08 声明式 Surface 的现有兼容需求。

`DeclarativeSurface.tsx` 在 text 上启用原生浏览器选择；存在非空选区时阻止父容器
click。`src/platform/desktop/useShortcuts.ts` 对同一 selectable root 内的非空选区
放行文本复制，不能被全局图片 Copy 抢占。扩展附件的整块本地复制是另一条合同，
不可用它替代部分文字选择。当前原生版本尚未完成以下整条交互链。

- 复用实际文字排版的字符命中及多行、双向文字高亮，不按平均字宽猜测。
- 按下、拖选、抬起保留选区；非空拖选不触发祖先整块 click，普通单击仍遵循原合同。
- 仅当前有效选区可经用户复制操作写入文本剪贴板；不生成远端 Surface 动作。
- 正确处理捕获丢失、取消、场景替换、关闭、滚动、DPI/reflow 与焦点变化。
- 不可见/被裁剪或被上层节点遮挡的文本不得抢占输入；不绕过宿主输入准入。
- 键盘操作、选区复制、真实 HWND 消息及可见高亮必须单独验证；无障碍选区模式
  与物理输入体验不能以几何单元测试代替。

## v163 已实现：实际排版索引与选择几何基础

`surface_text.rs::Layout` 与 DirectWrite 同生命周期保存规范化 UTF-16 文本，克隆
布局共享 Rc，不复制整段文字。索引不能直接用于原始 UTF-8 props.text：原布局
已经规范化 CRLF，并按 whiteSpace 折叠空白。UTF-8 选区文本只在请求时构造。
原有每段 65536 UTF-16 单元、256 KiB 原文及场景 1 MiB 限额保持；新文本快照
每布局最多 128 KiB，重新测量的临时布局随原 owner 释放，无全局缓存。

`surface_text_selection.rs` 使用 [HitTestPoint](https://learn.microsoft.com/en-us/windows/win32/api/dwrite/nf-dwrite-idwritetextlayout-hittestpoint)
取得字形 cluster 首尾插入位置；拖出文字返回最近边沿。使用
[HitTestTextRange](https://learn.microsoft.com/en-us/windows/win32/api/dwrite/nf-dwrite-idwritetextlayout-hittesttextrange)
取得换行及 bidi 分段矩形，先探测所需数量，再在文本长度约束内分配；拒绝无效
坐标、逆向/越界范围与切开 surrogate pair 的端点。不用替换字符掩盖非法索引。

`surface_text_frame.rs::TextItem` 仅对 text 且 selectable 为布尔 true 开放选择命中，
把宿主逻辑坐标转换到既有布局；开始选择要求位于可见文字，拖选延伸允许越界。
高亮矩形应用当前 origin 和祖先/内容裁剪，不重新排版，不修改任何持久数据。
实际遮挡准入仍须由后续 Host 命中路由负责，不能仅遍历 TextItem 抢占指针。

## v163 下一实现边界（历史检查点）

在现有 Presentation/Host 生命周期内维护单个选区，并在现有绘制顺序中显示
高亮；随后接入 Surface HWND pointer/keyboard 与本地复制。复用既有场景身份、
输入捕获与撤销机制，不放到数值编辑 FIFO，也不创建隐藏 EDIT 形成第二份排版。
当前基础 API 尚未接入这些生产交互，不宣称 OCR 部分选择可用，不提高 capability。

## 验证记录

v168 新增 7 项回归，focused 7 passed；可见全量库 2549 passed/42 ignored
（89.00 秒），日志 `linshi/otter-v168-full.stdout.log`。覆盖实际 DirectWrite
长短行往返、软换行连续/首次鼠标后导航、Unicode cluster、reflow 重置及正式
HWND 消息。审查指出首次鼠标软换行归属丢失，已保留真实 caret/行点并补回归；
改为 `[x,y]` 状态时一项测试曾误传上一行旧点，已更正为逐次传递实际返回值。
Clippy all-targets -D warnings、fmt、3055 文件严格行数、19 个本轮文本 UTF-8
无 BOM 与 diff 检查通过。没有系统键盘注入/剪贴板写入，历史 ignored 未重跑。
startup 7 passed（14.51 秒）；无测试目标目录进程残留。未提交、推送或发布。

v167 新增 6 项回归，focused 6 passed；可见全量库 2542 passed/42 ignored
（87.44 秒），日志 `linshi/otter-v167-full.stdout.log`。真实 HWND 回归验证
EDIT region 空/非空切换、父窗口焦点及 draft 保留；初始 WS_VISIBLE 判据错误
已依据既有 region 实现修正。Clippy all-targets -D warnings、fmt、3053 文件
严格行数、13 个本轮文本 UTF-8 无 BOM 与 diff 检查通过。静态审查未发现确定
生产缺陷，但不替代任意 Win32 重入或 sync_edits 故障注入；历史 ignored 未重跑。
startup 7 passed（15.49 秒）；无测试目标目录进程残留。未提交、推送或发布。

v166 新增 6 项回归：实际 DirectWrite emoji/组合字符/规范化换行边界、空文本与
越界，Presentation 扩展/反向/折叠/owner，HWND 焦点/准入/重复/撤权/换场景，
以及真实消息入口先于滚动、Shift+Tab 仍可导航。focused 19 passed/1 ignored；
可见全量库 2536 passed/42 ignored（83.46 秒），日志 `linshi/otter-v166-full.stdout.log`。
Clippy all-targets -D warnings、fmt、3051 文件严格行数、16 个本轮文本 UTF-8
无 BOM 与 diff 检查通过。只读交叉审查未发现确定阻断缺陷；不把静态审查或
线程键盘状态测试当物理键盘/全部 Win32 重入时序证明。历史 ignored 本轮未重跑。
startup 7 passed（14.77 秒）；无测试目标目录进程残留。未提交、推送或发布。

v165 新增 2 项正式光标准入测试、1 项有界应急释放回调测试和 1 项显式系统鼠标
探针。最终可见全量库 2530 passed/42 ignored（76.56 秒），startup 7 passed
（15.76 秒）；Clippy all-targets -D warnings、fmt、3049 文件严格行数全部通过。
13 个本轮文本 UTF-8 无 BOM、diff 检查通过；全量日志 `linshi/otter-v165-final.stdout.log`。
新增 ignored 探针已单独运行通过，不把全量中的 ignored 计作通过；其余历史 ignored
本轮未全部重跑。未提交、推送或发布。

v164 新增 14 项回归。真实自有 HWND 消息验证拖选抬起/捕获释放、collapsed 单击、
取消/换场景/撤权、Host 取出时重入取消及 resize 保留；真实 WIC/D2D 比较证明
高亮改变选区像素但不越过祖先裁剪，清除后恢复原帧且画刷颜色复原。复制测试走
同一发布准备与最终门禁，以回调代替系统发布；HGLOBAL 测试实读暂存 UTF-16
与 CRLF/NUL，但不调用 Open/EmptyClipboard。无真实系统剪贴板写入。
可见全量库 2527 passed/41 ignored（75.93 秒），startup 7 passed（14.22 秒）；
Clippy all-targets -D warnings、fmt --check、3047 文件严格行数通过。
本轮 23 个文本 UTF-8 无 BOM、diff 检查通过。全量日志
linshi/otter-v164-full.stdout.log。只读交叉审查未发现可确认的实质问题，但不把
静态审查当任意 Win32 重入时序或第三方粘贴的运行证明。未提交、推送或发布。
显式跨进程 UIA 回归 Slider 4、Select 1、通用 3 项通过；新增文字选区 UIA 模式仍待实现。

v163 新增 5 项测试使用实际 DirectWrite：覆盖 CRLF/折叠空白后的索引、中文、
emoji surrogate pair、组合字符 cluster、换行、希伯来语 bidi、越界拖选、非法
范围/坐标、布尔属性准入及祖先裁剪。初始布局夹具漏写绝对定位父容器的 relative
上下文，实际返回 Container(Unsupported)；只修正夹具，生产布局规则未变。
可见启动全量库 2513 passed/41 ignored（76.95 秒）；Clippy all-targets
-D warnings、fmt --check、3038 文件严格行数（全部不超过 500）通过。
全量日志：linshi/otter-v163-full.stdout.log。系统触摸 ignored 探针本轮未重跑，
本轮没有修改 Slider 路径；这些结果不替代后续真实选择手势与剪贴板验收。
startup 7 passed（13.23 秒）。本轮 11 个文本 UTF-8 无 BOM 与 diff 检查通过。
