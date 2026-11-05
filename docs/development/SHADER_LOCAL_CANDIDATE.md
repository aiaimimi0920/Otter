# 显式本地 Shader 候选与产品启动

当前目标见 [统一修订计划](WINDOWS_NATIVE_REVISED_PLAN.md)：功能替代验证后退役 ANGLE。
下述候选是既有开发证据，不代表未来 Shader 路线；不再以旧 GLSL 不改动作为保留 ANGLE 的要求。

这是 Windows 原生内部开发入口，不是通用 DLL 安装器、来源签名验证器或公共发行许可。
默认仍为 `Installation=None`；不扫描 PATH、环境、Art 响应、settings 或 session 来寻找
ANGLE。完整迁移与材料供应链分别见 [迁移计划](WINDOWS_NATIVE_MIGRATION.md) 和
[ANGLE 工具链](ANGLE_TOOLCHAIN.md)。

## 执行信任来自哪里

`native/src/shader_candidate.rs` 固定 v332 构建、v333 材料中已复核的两 DLL 身份。
命令行只能选择目录，不能提供或自动导入新的 hash：

| 文件 | 字节数 | SHA-256 |
| --- | ---: | --- |
| libEGL.dll | 199680 | fb538bb38dae68c612cfc309881143cdbd3ace9498e207f4cb2f6d38c3721639 |
| libGLESv2.dll | 5295616 | beeb45725d5ef558eaaa1e3a222df31e9d53103c0ee154f6853380e2a362e5ff |

更新 pin 需要源码审阅、新构建/材料证据与开发侧运行验证；未知目录自算摘要、合法 PE、
材料自洽或能渲染像素，都不自动获得新的执行信任或法律批准。

候选入口不复制文件、不创建目录、不改 ACL、不保存启动选项，不系统安装或分发 SDK。
调用者另行准备只含上述两个普通文件的本地固定盘目录；大小写、额外文件/目录、硬链接、
reparse、ADS、设备/UNC/相对路径、`.`/`..` 和 Windows 歧义名称都会被拒绝。

固定卷根及 OS/卷命名空间是信任锚。卷根之后的每一级祖先目录都按句柄检查 owner/DACL，
只有当前用户、SYSTEM、Administrators 可改其属性/EA、ACL 或 owner。候选目录和两文件
进一步拒绝其他主体的任何写入、追加或删除权。未知复杂 ACE、null DACL 与越界 SID 拒绝；
inherit-only ACE 不当作当前对象授权，每个实际子级另检其生效 ACE。
不自动修复共享祖先权限；例如本机 Public/linshi 下的完整路径不符合此条件，必须拒绝。

逐级目录以读取访问和无 write/delete sharing 保活，拒绝已存在的链接；两 DLL 保持只读
共享、同句柄限量摘要校验。**目录共享锁不能单独挡 `FILE_WRITE_ATTRIBUTES` reparse
控制**，因此祖先 ACL 检查不可删除。属性读取句柄也不等于能保护 rename 的读取句柄。
两个反例均已在 linshi 的新建空目录实际验证，失败/反例日志保留。

所有租约绑定 `Arc<TrustedBundle>`，覆盖准备、执行、取消、超时和子进程等待。固定候选
在 worker 读取请求时重新取得同样的路径、ACL、摘要与文件租约，不依赖父进程跨进程
持有 Arc。其他既有可信清单 wire 调用方仍保留原合同，不冒称它们都经过新候选门。
父进程不加载 ANGLE；实际加载仍由原 worker/Library owner 执行，不扩大 DLL 搜索路径。
Job 提供资源/进程生命周期约束，不是 restricted token、AppContainer 或权限沙箱。
本入口信任当前用户、管理员、SYSTEM 和 OS 卷根，不防同用户恶意进程、内核/卷命名
空间管理员或已有恶意映射，不以 hash/ACL 快照声称覆盖所有 Windows 攻击面。
祖先必须持续处于可信权限下；ACL 快照不能撤销其他主体历史上已取得的属性或
WRITE_DAC 句柄，刚收紧一个曾公共可写的目录不能因此视为可信私有路径。

## 产品入口与兼容性

```powershell
rtk proxy $ReviewedProductExe --shader-candidate $PrivatePinnedDirectory
rtk proxy $ReviewedProductExe $Image --pin --shader-candidate $PrivatePinnedDirectory
rtk proxy $ReviewedProductExe --session $Workspace --session-check --shader-candidate $PrivatePinnedDirectory
```

选项在所有私有 worker 分流之后、session/普通/capture 模式分流之前提取。`Startup`
保留原公开 `run/run_capture/run_session` wrappers，并把可选 Installation 送到普通和
session Application；smoke 的独立 Application 也接同一安装方法。读取显式配置失败
必须在截图、建窗或注册热键之前终止，不静默禁用候选后继续。
候选只解除原有纯 Shader 本地准备门，不扩大 Surface、JavaScript、自动执行或正式
后端执行策略；关闭程序后不持久启用。`--settings` 与会话带值选项保持独立，不把值
重新解释为顶层开关。无选项的原 smoke 不触发 Shader 入口。

## 实际 EXE 开发侧验证

```powershell
$env:OTTER_SHADER_CANDIDATE = $ReviewedV333MaterialsPayload
$env:OTTER_SHADER_PRIVATE_TEST_ROOT = $NewDirectoryDirectlyUnderPrivateUserHome
rtk proxy cargo test --manifest-path native/Cargo.toml --locked --offline `
  --test shader_startup shader_product_exe_first_fetch_child_commit_and_cached_rerender `
  -- --ignored --exact --nocapture
```

正向门显式复制两 DLL 至新的私有测试目录，只修改本次新副本的 DACL，不修改源材料
或现有祖先。linshi 外夹具保留，不自动删除；构建、日志、其余测试和证据仍放 linshi。
完整产品 smoke 的外部期限仍是 15 秒，不通过放宽期限掩盖旧间歇超时。

测试 peer 的 localhost 证书只在内存生成，不导入证书库。`--shader-smoke-peer <manifest>`
仅能与 `--shader-candidate`、`--smoke-test` 同用；复用现有发现清单格式，但不读取默认
发现路径。实际 TLS1.3、叶证书 pin 和 exporter proof 认证后先取能力目录，再由生产
菜单→输入准备→首次取码→真实 child→`commit_checked` 提交图像。再次改变 gain 走原
配方缓存和第二个真实 child，peer 断言无第三次连接；不预填缓存或直接调用 renderer。
正式图像及 PNG 解码像素都要匹配，父侧 ANGLE 模块必须始终不存在。

这会对**显式信任的 peer**请求固定 `shader-startup` Art 的真实准备材料；ID 不证明
对面绝非真实 Loom，不应对未知清单运行。测试自己只创建回环 fixture，不访问现有服务。
smoke 不读默认 profile，但显式 `--settings`/`OTTER_NATIVE_PROFILE_DIR` 仍是现有配置
入口；测试 runner 主动移除后者。不能把此门称为实际用户 Loom 或完整 UI 验收。

完整 notice/source/tool/静态 CRT 权利闭包、可信发行安装、完整桌面回归、资源长稳、
JavaScript 和用户最终手测仍是独立门；材料中所有 false 批准边界保持不变。

### v334 验证停点：partial，不是最终全绿

候选路径/ACL/SID 五项、启动参数三项、默认产品拒绝三项通过。最终 Shader 定向
65 passed / 15 ignored，严格 Clippy/all-targets check/fmt/版本和 3,459 文件行数门
通过；八个新文件 53–178 有效行，18 个修改 Rust 文件最大 498，无新增行数例外。
九文档与本轮源码的 UTF-8 无 BOM、空白和本地链接另检。

中间源码真实正向门首轮 15 秒超时、同源码复验通过。三处 Clippy 多余借用修正后，
最终 EXE SHA-256 为 `a5d38177f587268be93baa222067dd91afedb77eafcc9f4e8cc40bf226c65688`。
先绑定产品与七个测试程序摘要，再无编译直跑六套集成，26 项全部通过、摘要不变。
但最终默认计时的 Shader 正向门仍在 15 秒超时，两轮 Shader 提交标记都已出现，
最后输出到旧 application open smoke；不是 Shader 正向整门通过，也不能证明根因。
只做一轮现有计时开关诊断后在 14.23 秒通过，确认连接/缓存/child/正式像素和退出
锁释放断言；诊断不替代失败的正式门，未再重试凑绿、改期限或删覆盖。
原始失败、诊断、源码与实际 EXE 身份保存在 linshi/otter-native-v334-regression。
完整桌面库未重跑；旧 UIA/启动问题及本次最终超时都未结清。

### v335 验证停点：布局修复通过，完整 smoke 仍 partial

真实 HWND 回归先红后绿：消除不变布局重复定位，同时保留真实纠偏、全部 BUTTON、
resize/双轴滚动、隐藏状态和父子窗口释放。最终工具面板相关 22 项通过；Shader
定向 65 passed / 15 ignored，严格 Clippy/all-target check/fmt 通过。候选 pin、ACL、
租约、child 重验、默认关闭及材料 false 批准没有变化。

预先固定三轮，不设置 OTTER_NATIVE_SMOKE_TIMING、不经 Cargo，每轮使用新私有夹具，
原 15 秒 child 期限不变。第 1、3 轮通过（测试整体 14.86/14.61 秒，含夹具准备），
第 2 轮 child 在 15.027 秒超时；两轮 Shader commit 已输出，后续旧 smoke 最后到
execution-owner 标记，但不能据此断定阻塞点。原矩阵结果是 2 passed / 1 failed，
不是整体通过；私有夹具保留，计时诊断也不替代正式失败。

收尾额外 Cargo build 改写了 target/debug/otter-native.exe，摘要断言主动失败。
原矩阵绑定 `799c7edefdcc72f621faa0a5a4b9654fc14e387ca29c19393974f51cde6f12d4`，
后续字节为 `1b2d2834a92d93604ec3e582cab209eb3bf237f433df7b58cb2ec8af01a607e9`。
新 EXE 已先封存至本轮 final-artifact，再只做一次独立身份验证；13.21 秒通过
（测试整体时间），复核两认证连接/两 child/正式 PNG/缓存无第三次连接与锁释放。
这是新字节的运行证明，不覆盖原三轮失败，不把 v334 或旧 UIA/启动问题结为通过。
两套身份、构建变化、诊断及原始失败均保存在 linshi/otter-native-v335-regression。
最终封存 EXE 的拒绝三项和六套集成 26 项通过，产品/封存副本/七测试程序共九个
摘要前后相同。版本、3,461 文件行数门、六文档 UTF-8/本地链接与 diff check 通过；
两新文件 69/189 有效行、两修改 Rust 432/181 行，无新例外，最终 2,309 项输入
（原生 1,777）前后一致。这些定向证明不等于完整桌面或原三轮矩阵通过。
不再增加正向重试；完整桌面库、真实用户 Loom、JavaScript、完整供应链/长稳和最终
全面手测未完成，无产品组包、正式程序替换或提交推送。
