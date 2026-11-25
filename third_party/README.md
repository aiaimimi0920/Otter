# 原生 SVG 依赖补丁

`native/Cargo.toml` 通过 path patch 使用以下固定版本的 crate 副本。上游许可证随归档保留，
不要直接修改 Cargo registry，也不要从临时实验目录引用生产源码。

| 副本 | 本地补丁责任 |
| --- | --- |
| `resvg-0.48.1` | Gaussian/缩放、morphology、偏移与颜色运算的有限性和资源保护；`turbulence` 使用 checked 转换及运算，错误沿 `render_checked` 交回 worker。 |
| `usvg-0.48.1` | 保留零宽 `context-stroke` 的 paint/坐标上下文；路径文字使用精确直线弧长、位移跨度和原字符索引；outline glyph 保持独立绘制顺序。另有元素长度和滤镜参数边界保护。 |
| `tiny-skia-0.12.0` | 小画布的 AA hairline 保留实际 clip，以及曲线快拒绝。 |
| `roxmltree-0.21.1` | 迭代处理元素嵌套并维护 XmlEvents 校验，单独覆盖深层 XML；不是对任意输入深度的保证。 |

依赖能解析某种语法不等于 Otter 宿主准入。SVG 文字的 `textLength`、`lengthAdjust` 与
`direction=ltr/rtl` 有单独属性回归；不能因此推定任意 CSS 或滤镜可用。参见
[宿主规格](../docs/development/SURFACE.md)及[原生属性回归](../native/src/surface_svg_direction_tests.rs)。

## 更新与验证

- [`sources.json`](sources.json) 定义副本位置。`patches/<name-version>/source-manifest.json` 记录
  上游 archive、补丁及每个候选文件的 SHA-256；上游已有文件另有输入摘要，本地新增文件的
  `upstreamSha256` 为 `null`。`fix.patch` 是实际可回放的差异。
- 更新时从固定上游归档重放最小补丁，用 `git -c core.autocrlf=false apply` 保持来源字节；
  一起审阅源码、许可、补丁和完整清单，不通过手改预期摘要来掩盖失败。
- 运行 `node scripts/release/third-party-sources.cjs .`。它检查文件集合、链接、越界、字节及
  `cargo metadata --locked` 中实际解析的路径。依赖安全与候选打包复用此门；patched Cargo PURL
  带源码树校验限定值，不与官方同名同版本包合并。
- 按变化运行库和真实 child 回归、原生验证及[依赖安全门](../docs/security/DEPENDENCY_SECURITY.md)。
  固定上游副本的既有行数例外不适用于新增手写代码、测试或清单工具。构建产物不放在副本内。

副本内的 README、CHANGELOG 和测试说明同属固定来源字节，应随源码与许可证保留，
不作为 Otter 当前能力承诺或开发计划清理；Otter 自身的使用说明从[文档中心](../docs/README.md)进入。
