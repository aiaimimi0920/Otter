# 原生 SVG 依赖补丁

v225 增加 turbulence 实际坐标的 checked 转换/运算与 render_checked
失败通道，贯通嵌套子树，worker 丢弃错误结果。43 文件重放一致，保留
原始 Cargo/许可证。新增入口及测试独立测量 31/100 行；filter/mod.rs
必要接线增加 21 行至 892，未增加独立算法职责。宿主滤镜仍不开放。

这些目录是固定版本上游 crate 的受控副本，不是新的 Otter 业务模块。
`native/Cargo.toml` 仅对原生宿主应用 path patch，Tauri 的依赖解析不变。
上游许可证随完整归档保留。禁止直接修改 Cargo registry 或依赖临时目录。

v224 在 resvg turbulence 初始化中使用 unsigned_abs 后取模，修复
i32::MIN seed 的有符号取负溢出；取模后才转回 i32，正 seed 路径不变。
新增宽整数对照回归，完整 41 文件重新从原归档重放；Cargo/许可证不变。
这是库数值边界修复，不开放宿主滤镜，不代表全部 turbulence 参数安全。

| 副本 | 上游来源 | 本地补丁责任 |
| --- | --- | --- |
| resvg-0.48.1 | crates.io 的 resvg 0.48.1 归档 | 有限 Gaussian/缩放帧、morphology 队列、滤镜结果生命周期、颜色/分量转换与偏移边界；滤镜仍未由宿主准入 |
| usvg-0.48.1 | crates.io 的 usvg 0.48.1 归档 | context-stroke 保留零宽来源的 paint 与坐标上下文；路径文字精确直线弧长、位移跨度锚定与原字符索引回写；outline glyph 独立按序绘制 |
| tiny-skia-0.12.0 | crates.io 的 tiny-skia 0.12.0 归档 | 小画布 AA hairline 保留实际 clip；独立保留曲线快拒绝 |
| roxmltree-0.21.1 | crates.io 的 roxmltree 0.21.1 归档 | 元素嵌套 tokenization 改为迭代，保持 XmlEvents 校验；独立深层测试 |

v221 接入重新由官方归档重放的 resvg 41 文件与 usvg 45 文件。resvg 只改
源码，保留上游 Cargo.toml、Cargo.lock 和许可证；usvg 相对 v219 仅增加
filter parser 的 morphology 半径及 turbulence 频率有限性保护。原生锁文件
仅将 resvg 改为本地来源，版本/其他包不变；不得依赖外部实验目录。
九个 resvg 新增模块继续受独立有效行数合同约束。上游 filter/mod.rs 的
遗留调度职责为 871 有效行（原 898），本轮不扩大；新增算法已按责任分离，
由库测试及隔离像素/资源回归保护，后续开放滤镜前仍需完整准入预算与等价门。
这次源码接入不开放 filter、fe* 或 filter 属性；实际 child 的拒绝合同保留。

v219 将已重放的 usvg 45 文件和 roxmltree 210 文件接入固定目录；tiny-skia
200 文件不变。usvg 新增元素长度约束、片段分组、导出范围/路径 ID 与叶 span
模块，并保留方向及布局优化。生产 policy 暂未开放 textLength/lengthAdjust/
direction，接入与准入分别验证。roxmltree 与叶 span 提取组合修复已复现的
512 层 child 栈溢出，不声称任意嵌套深度安全。
新清单使用 schema 2 的 upstream/local-added 来源与精确 added 清单；XML
副本 Cargo.toml 仅额外注册深层测试，独立 lock 固定本机已有 memchr 2.8.3，
不替换原生应用的锁版本。新增手写模块和测试仍须单独运行有效行数检查。

v219 后续准入已接入 textLength、lengthAdjust 与 ltr/rtl direction 的属性路径，
新增真实 child 的浏览器像素和拒绝/资源测试；CSS style 不开放。
上段“暂未开放”描述依赖接入阶段，当前验证结果以 SURFACE_SVG.md 最新条目为准。

`sources.json` 枚举副本位置；`patches/<name-version>/source-manifest.json`
记录上游 archive SHA256、补丁 SHA256，以及每个上游/补丁后文件的 SHA256。
`fix.patch` 是可回放的差异。首次接入已从匹配 Cargo.lock 校验和的本地官方
crate 归档重建，应用补丁后逐字节校验全部 240 个文件；回放必须使用
`git -c core.autocrlf=false apply`，避免全局 Windows 换行设置改变来源字节。

维护流程：固定上游版本和 archive checksum，重新生成最小补丁及全部文件清单，
从独立归档重放验证，运行真实 Job child 回归、完整 native 测试和依赖安全门。
不能仅更新 manifest 中的哈希来绕过失败；来源更新必须一起评审补丁和测试。
构建产物不得放入副本目录。原型实验和日志放在工作区之外的 linshi。

v216 增补 `src/text/layout.rs`：普通文字 x=50 与直线路径 startOffset=40 原本
产生 50 与 49.951435 的不同 glyph 平移。直线/闭合线段现按端点距离直接求点和
切向，不借用曲线逆弧长的 0.5 精度；零长线段跳过，Q/C 曲线算法不变。
40 个 usvg 文件从同一固定官方归档重新应用累计补丁后逐字节一致。

v217 增补同一 layout 文件：textPath 锚定计入 dx 后的 advance 跨度；以局部
f64 坐标计算跨度，最后加公共路径起点。法线查询按路径距离排序，但结果始终
回写原 cluster 索引，隐藏字符不挤占后续槽位。普通水平文字和曲线求解不变。
40 个 usvg 文件再次从固定归档回放并逐字节验证；来源哈希与双 SBOM 已更新。

v218 增补 `src/text/flatten.rs`，累计五个 usvg 文件变化。每个 outline glyph
立即提交 Path，不合并相邻 glyph 的轮廓；保留重叠 AA、fill-opacity 与 stroke
合成顺序。外层 group opacity 和整段 text paint-server bbox 处理不变。
代价是每个 glyph 增加独立 Path/样式克隆/bbox/绘制调用；4096 glyph 真实 child
预算测试通过，不宣称常量开销或完整性能验收。40 个文件再次回放验证一致。
textLength 实验布局补丁没有接入本副本，该属性仍由 Otter policy 拒绝。

完整性门：`node scripts/release/third-party-sources.cjs .`。
许可证审计与双格式 SBOM 库存生成均调用此门，拒绝漏文件、额外文件、符号链接、
路径越界及字节不匹配。SBOM 的 patched Cargo PURL 带源码树 SHA256 checksum
qualifier，不与官方同名同版本包合并。该摘要由按路径排序的
`path + NUL + candidateSha256 + LF` UTF-8 记录拼接后计算 SHA256。
门还运行有超时的 `cargo metadata --locked`，逐包核对实际解析的 manifest_path
与已校验目录一致且在 resolve 图中，拒绝同名同版本但换成其他 path 的源码。
干净机器可按锁文件下载依赖，不要求预先存在离线缓存。

行数豁免仅限四个精确第三方副本目录的上游遗留源码；本目录中的补丁清单和检查脚本没有
通用豁免。不能将手写业务实现放入这些副本以逃避代码规模限制。
新增的本地模块及测试不享受上游规模豁免，接入时单独用仓库 lexer 测量并留证。
当前仅为开发接入，不代表正式发布、依赖安全放行或 Windows 迁移完成。
