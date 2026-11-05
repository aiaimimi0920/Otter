# 原生 Surface Grid 合同

v346 引入声明式 `type: "grid"` 的 measured-content 布局核心。它使用同一 Scene、
NativeMeasure、Tree、Presentation、DirectWrite/Direct2D、Geometry 和 Hits，不创建
另一棵渲染树、浏览器、线程或业务实现。默认 capability 尚不广告 Grid。v347 将受限
JavaScript CSS `display:grid` 接入本核心，精确声明/层叠/上下文边界见
[JS Grid 合同](SURFACE_JAVASCRIPT_NATIVE.md)；不能据此推断完整 CSS/股票插件已经可用。

v347 的 JS Grid 定向 17/1 ignored、实际产品 Grid worker 显式一项，以及相关回归/
严格静态/3,543 文件行数门通过。冻结/构建后/运行后源码和六程序身份一致，旧封存
产品不变，相关进程残留零；完整结果、失败与默认能力边界见迁移台账 v347。

## 数据入口

以下均为原生 Scene 的 `layout` 字段，不是任意 CSS 声明：

| 字段 | 当前支持 |
| --- | --- |
| `gridTemplateColumns` / `gridTemplateRows` | 空缺或 `none`；空格分隔轨道、固定次数 `repeat(n,...)` |
| `gridAutoColumns` / `gridAutoRows` | 空缺默认 `auto`，或单个轨道 |
| 轨道 | 非负 px/裸 `0`、`auto`、正 `fr`、`minmax(min,max)`；min 不接受 fr |
| 百分比轨道 | 只在对应轴确定时求值；不确定高度的百分比行明确拒绝 |
| 子项 `gridColumn` / `gridRow` | 字符串正负线、`start/end`、`span n`、`start/span n`、`span n/end`、`auto` |
| `gridAutoFlow` | 空缺或字符串 `row`，稀疏前进放置 |
| `gap` | 非负像素数值或一/两值 px 字符串；两值顺序为 row、column |
| `columnGap` / `rowGap` | 单个非负 px 值，覆盖对应 gap |
| `justifyItems` / `align` | 水平/垂直 item 默认对齐，默认为 stretch |
| 子项 `justifySelf` / `alignSelf` | auto/normal、stretch、start/end、flex-start/end、center；垂直轴另支持 baseline |
| `justify` / `alignContent` | 水平/垂直轨道分布；normal/stretch、start/end、center、space-between/around/evenly |

轨道和放置字符串限 ASCII；非法新属性类型/语法整次失败，不保存后忽略。通用
width/height/min/max、padding、relative 等仍沿原 Scene 长度白名单和默认回退策略，
没有全局收紧旧合同。Scene 本身保留未知有界属性，不等于这些属性已被消费。

## 放置和尺寸

先保留全部双轴显式项，再处理固定行/自动列，最后按单个 row cursor 处理自动行。
固定行的游标对其覆盖行只向前，不回填前洞。负线相对于显式轨道末端解析，不随
隐式增长漂移；不能在网格前方扩展负隐式轨道。允许双轴显式重叠，绘制仍按 DOM
顺序；隐藏子项不占位、不进入几何或命中。

列先按已测量贡献求值，再按 cell 宽重新测量高度和 baseline；保留初始宽度贡献，
避免断行后的短行反过来收窄 auto 列。固定下限造成的溢出保留；fr 分配剩余空间并
冻结达到下限的轨道。不确定高度的 fr 行按已测量内容贡献求相同比例，非像素迭代。
空 Grid 没有凭空生成的隐式轨道；仅保留显式轨道和 padding/border。

本实现不是 CSS min/max-content 算法：auto minimum/内容贡献采用当前原生测量，
不是浏览器的 intrinsic track sizing；初量使用父可用尺寸，不能把所有百分比与 auto
轨道的循环依赖声称为浏览器等价。默认 normal 图片不 stretch，宽度默认受 cell 宽
约束。图片百分比、自然比例和 padding 使用实际 cell 包含块，而非整个父 Grid。

单行 baseline 组先用与最终分配相同的 height/minHeight/maxHeight 约束，缺少真实
baseline 的项以分配后的底边合成，避免显式高 view 被错误上推。跨行 baseline、
水平 baseline、baseline 项的百分比高度约束尚未实现，明确拒绝。center/end 的
内容溢出保留负偏移；分布类在负剩余空间时使用原生安全回退，不宣称完整 CSS 对齐。

absolute Grid item、named lines/areas、dense/column flow、auto-fit/fill、subgrid、
nested repeat、min/max-content/fit-content、百分比 gap、margin/order 和完整 CSS
Grid 算法均未实现。v347 的 JS 接线在现有 CSS projection owner 映射受支持字段，
不复制另一套布局算法或加入股票专用分支；原生 Scene 字段语义不因此改写。

## 资源与生命周期

- 保持 Scene 1 MiB / 2,048 nodes / 32 depth；显式列表单轴最多 64 轨道、最多 64
  列/2,048 隐式行，最终矩形最多 16,384 cells。
- 每次 placement 搜索按探测的 row span 计费，最多 262,144；每轴内容贡献的 span
  总和同样最多 262,144，先检查范围和预算，失败不返回部分几何。
- fr 冻结使用 active mask，至多轨道数加一轮；不按像素重试，也不在内循环线性
  contains 搜索活动轨道。尺寸有限，单轨道/盒最多 32,768。
- 仅包含 Grid 的场景启用 node ID + available bits 缓存，每反馈轮最多 8,192 次
  cache miss；样式属于同一测量 owner，反馈或重新 prepare 清空缓存。
- 不增加原八轮文字/图片收敛预算；新场景、reflow、快照恢复和绘制命中共享相同
  grid cell。Presentation 准备失败保持上一有效场景，窗口关闭沿原 owner 释放。

## 验证入口与边界

纯模型见 `surface_grid_tests.rs` / `surface_grid_validation_tests.rs` 和 placement
内部预算反例；真实 DirectWrite/图片反馈见 `surface_grid_native_tests.rs`；真实隐藏
HWND 的像素、文字、命中、原 click route、替换失败回滚、隐藏/resize/close 见
`desktop/surface_window_grid_tests.rs`。定向开发命令：

```powershell
cargo test --manifest-path native/Cargo.toml --locked --offline --lib grid -- --test-threads=1
```

`grid` 筛选也可能匹配其他历史测试，必须按测试名和日志核对，不拿总数当完整迁移。
开发和封存产品结果以 [迁移台账](WINDOWS_NATIVE_MIGRATION.md) v346 为准；实际
产品仍需重新构建、摘要绑定、封存后资源/startup 门。默认 JS、真实 Loom/股票、完整
DOM/CSS/Canvas、OS 权限隔离、资源长稳、用户手测和 N01–N12 验收仍开放。

最终本切片专属 22 项通过（`grid` 宽筛选为 25 项），封存测试程序按相同字节复验；
相关 Surface/Window/JS/授权回归、严格静态和 3,535 文件行数门通过。产品重建后的
资源/startup 和原 JS 显式门也通过，源码/六程序/旧封存产品摘要一致、相关进程残留
零。以上为 v346 核心结果；v347 JS 接线结果另验，默认能力及完整迁移边界不变；失败原件见
迁移台账 v346 和本轮 closeout。
