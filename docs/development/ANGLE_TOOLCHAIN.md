# ANGLE 固定源码暂存、显式工具链与隔离 DLL 构建

当前方向已由 [统一修订计划](WINDOWS_NATIVE_REVISED_PLAN.md) 调整为原生双后端与 ANGLE 退役。
下述是现有工具/历史证据，不再作为长期兼容路线扩建；替代功能验证前保留，不授权删除或默认加载 DLL。

本工具服务于独立 Windows 原生迁移的 Shader 构建供应链。入口可以生成可单独导入的
`setup_toolchain.py` 适配目录、固定 GN overlay，或从显式本地输入暂存构建树；独立的
显式构建入口才会运行已审阅的 GN/Ninja、编译器与 SDK 工具。
暂存入口仅运行只读 Git 对象消费，不获取依赖、不接受许可、不安装 SDK、不运行
编译器、SDK 包内工具、CIPD、vpython、hooks、GN 或 ninja，也不授权应用加载 DLL。
完整迁移仍以 [迁移总计划](WINDOWS_NATIVE_MIGRATION.md) 为准。

v334 的实际产品显式消费入口另见 [本地 Shader 候选](SHADER_LOCAL_CANDIDATE.md)。
它固定已审阅的两 DLL 身份并检查持续私有路径，不因材料 verify 成功自动授予执行
信任；本地开发运行证据不改本工具材料中的 false 法律/安装/发行批准。

## v333：本地材料候选与独立封存核验

`scripts/native-candidate/angle_materials.py` 消费已独立审阅的 stage/build，不成为
新的构建 owner。它复用 v332 的完整 stage、工具链 anchors、日志、PE 与源码身份门，
不执行构建工具，不下载、接受许可、安装或分发 SDK，不默认激活 DLL。

```powershell
rtk proxy python -I -B scripts/native-candidate/angle_materials.py create `
  --stage $ReviewedStage --expected-stage-sha256 $ReviewedStageSha256 `
  --build $ReviewedBuild --expected-build-sha256 $ReviewedBuildSha256 `
  --output $NewMaterialsDirectory

rtk proxy python -I -B scripts/native-candidate/angle_materials.py verify `
  --output $MaterialsDirectory --expected-materials-sha256 $IndependentlyReviewedMaterialsSha256
```

两个输入摘要须由调用者独立审核；不能从未知目录自行计算摘要后称输入已可信。
create 输出全新仓库外目录，和 stage、构建证据、VS/SDK、SystemRoot、TEMP 不重叠。
成功输出 JSON，退出 0；输入/身份/材料错误退出 1，用法错误退出 2。创建后的失败
保留 `FAILED.txt` 和部分材料，既有目录不覆盖；有效候选必须完整可解析、无失败标记
且通过独立核验，不能仅凭 materials.json 存在放行。

材料包含 `payload/` 两 DLL、`evidence/` 中绑定的 stage/profile/build、GN trace、
八份原始日志、构建与收集器源码及 Rust API 快照，还有 `notices/` 原字节载体和两种
SBOM。只复制明确白名单，不复制 SDK/MSVC 二进制、缓存 d3dcompiler 或完整源码树。
证据保留本机绝对路径和工具 anchors 元数据，是本地开发材料，不应不经审查公开。
源输入和收集器源码结束前再次复核；未改 native-angle owner，不因增加消费方伪造
新 stage、重标旧构建或声称本轮重新编译 DLL。

`angle_intake` 拥有只读构建准入，`angle_notices` 拥有声明选择与原字节预算，
`angle_sbom` 拥有材料清单；`angle_materials` 编排复制，`angle_materials_evidence`
关联封存证据，`angle_materials_verify` 拥有独立封存核验。复用既有路径/名称空间/
JSON/流式写入 owner，不执行复制的源码。verify 不需要原 stage、VS 或 SDK 目录，
只承诺外部预期 materials SHA 绑定的精确材料一致性；不重新证明构建命令发生、
PE 调用语义、目录 ACL、来源签名或法律批准，也不是对恶意并发替换的原子保护。

单文件 16 MiB、材料总量 256 MiB、4,096 文件，create 每次写入前和 verify 都检查；
manifest 另受 16 MiB JSON 上限。声明每文件 4 MiB、合计 64 MiB、2,048 文件，
严格 UTF-8 无 BOM，保留原字节而非改写许可。路径复用 Windows 别名/设备名/穿越/
ADS/链接拒绝，实际文件和目录必须精确相等，额外 DLL、空目录与失败标记也拒绝。

声明选择仅为已绑定源码中的 LICENSE/COPYING/COPYRIGHT/NOTICE/PATENTS 名称、
README.chromium 和四个已审阅内联 Chromium attribution 载体，不是完整许可审计。
真实计划确实编译 volk，不能以 D3D11 profile 排除其声明。Chromium base/zlib 对
Chromium root LICENSE 的引用、header-only/最终 archive members、静态 CRT 与
工具 runtime 权利仍未全部闭合；79 个 gitlink 仍有 66 个未提供。
`legalClearance`、`runtimeVerified`、`runtimeActivated`、`releaseApproved`、
`publishable`、三种 complete closure/coverage、`sdkInstalled`、`sdkRedistributed`
固定 false，独立 verifier 拒绝虚假批准。

CycloneDX 1.6/SPDX 2.3 只列材料文件及摘要，明确排除自身、另一份 SBOM 和 manifest，
不把 checkout 存在推定为完整 DLL 依赖。SPDX 许可/版权为 NOASSERTION，唯一 namespace
按本次 UUID 构造。真实候选已通过联网取得的官方
[CycloneDX schema](https://cyclonedx.org/schema/bom-1.6.schema.json) 和
[SPDX schema](https://raw.githubusercontent.com/spdx/spdx-spec/v2.3/schemas/spdx-schema.json)
及全部使用的 format checker；schema/URL/摘要和隔离验证器版本均留回执。
首次旧 Ajv 缺 iri-reference、后续验证辅助脚本遍历错误原日志保留，未弱化格式门。

实际 create/独立 verify 通过：159 个清单文件、109 份声明载体（434,303 字节）。
新鲜外部公共库驱动直接消费材料 payload，硬件/WARP 六 contexts、18 次像素对照、
12 成功 child/两无效 GLSL 拒绝通过，零像素差异且 child 父进程不加载 ANGLE。
另用固定 Ninja 的只读 `-t compdb -x solink` 展开两条链接响应内容，没有重新链接；
显式参数可见 compiler-rt，不代表对象内 DEFAULTLIB 已解析或 archive 成员闭包已证明。
回执位于 linshi/otter-native-v333-regression；运行验证独立留在外层，不修改材料中
的 false 边界。不是实际产品 EXE/Loom、可信安装或正式发行验收，默认关闭能力、
原生输入/旧产品程序和旧 UIA/间歇启动失败边界不变；完整迁移仍开放。

最终相关 Python 合同 105 passed、原生候选 Node 合同 9 passed，Rust fmt、版本、
全库 3,451 文件行数 ratchet 与 diff check 通过。九个新增生产/夹具/测试文件为
30–120 有效行，没有新增行数例外；34 Python 源码 AST、七文档 UTF-8/本地链接
通过。原生 1,767 项输入和旧 EXE 未改，相关构建/外部驱动进程零残留；完整桌面库
未重跑，不能据定向材料门追溯修复旧间歇失败。

## v332：显式固定 DLL 构建入口

`scripts/native-angle/build_fixed.py` 消费 `stage_build.py` 的全新外部暂存树，执行
固定 x64 D3D11 Clang/lld + MSVC STL、Release 静态 CRT profile。它不是下载器、
SDK 安装器、许可证接受器、签名器或运行时安装入口。只允许在已有授权范围内使用
SDK；本次授权仅覆盖 linshi 内的 Otter 隔离构建，不包含 SDK 分发或默认 DLL 激活。

```powershell
$env:DEPOT_TOOLS_WIN_TOOLCHAIN = '0'
rtk proxy python -I -B scripts/native-angle/build_fixed.py `
  --stage $ReviewedStage `
  --expected-stage-sha256 $ReviewedStageSha256 `
  --output $NewEvidenceDirectory
```

父调用进程必须显式设置 `DEPOT_TOOLS_WIN_TOOLCHAIN=0`。`$ReviewedStageSha256` 必须是
调用者从已独立审阅的 `stage.json` 取得的预期 SHA-256；入口不会从未知目录自行推定
预期身份。摘要只绑定已审阅回执，不是签名、发布授权或对未知输入的自动信任。
stage 绑定固定源码 pins、全部构建输入、profile/adapter、Python EXE 及全部
native-angle Python 源码；修改构建 owner 后必须重新暂存，不能改写旧回执迁就新源码。

构建只接受 fresh stage：`src/out/otter/args.gn` 是清单内模板，但已有 Ninja 缓存或
其他未登记文件会拒绝。输出也必须是全新仓库外绝对目录，与 stage、VS/SDK、
SystemRoot 和 TEMP 不重叠。成功退出 0 并输出 JSON；输入/构建错误退出 1，CLI
用法错误退出 2。创建输出后的失败保留原始日志和 `FAILED.txt`，不得覆盖旧目录重试。
有效回执须完整可解析、摘要/文件匹配且没有失败标记；外部强制中止的现场也不能仅凭
局部日志或残留 DLL 当作成功，必须另记中止原因并换新 stage/output。

`build_inputs.py` 独占构建前后身份/集合校验，复用原 JSON/路径 owner，只有明确调用
才把 JSON 上限从默认 64 KiB 提升至最多 16 MiB。构建后只允许 `src/out/otter/`
增加缓存，原源码/工具/profile、VS/SDK anchors、自有源码和 Rust API 仍再次核验。
环境复用 `load_environment()`，不继承 ambient PATH/INCLUDE/LIB，不回退发现工具链。

`build_job.py` 独占 Win32 进程树：以 suspended 启动，成功挂入 kill-on-close Job 后
才 resume；最多 32 活动进程，STARTUPINFOEX 仅允许继承复制的 stdin/stdout/stderr，
stdin 为 NUL。超时、异常和 root 提前退出都会终止 Job 的剩余子进程，查询 active=0
并有界等待 root rundown；即使一项 cleanup 报错仍清理其余句柄。Job 是生命周期边界，
不是权限/网络/文件系统沙箱。同 session 的 `Local` 路径摘要 mutex 只协调遵循本入口
的调用，不是跨 session、对抗恶意写入者或验证目录 ACL 的全局安全锁。

GN 最长 180 秒，plan/no-work 各 60 秒，compile 最长 1800 秒。每个 stdout/stderr
轮询检查 16 MiB；轮询间可能 overshoot，不是严格磁盘硬配额。实际 GN trace 必须只
出现五种已审阅脚本：setup_toolchain、compute_build_timestamp、get_concurrent_links、
get_cpu_count 和 `build/win/use_ansi_codes.py`。真实 Ninja 计划要求 `/MT`、`/WX` 和
lld-link，拒绝动态 CRT flags、vs_toolchain/vcvarsall；编译日志 warning/error 诊断
和后续仍有工作的状态都会拒绝，不修改生成后的 Ninja 文件。

`build_binary.py` 只读 PE 字节，不加载 DLL。两 DLL 必须是 x64 PE32+，Rust API
要求的 12/40 个名称需绑定有效 ordinal、非零 EAT、唯一且有文件 backing 的 RVA；
拒绝 forwarder，必需函数的目标节必须可执行，其他合法数据导出不被误当作函数。
普通和 RVA-based delay imports 都检查动态 MSVC CRT 与 `api-ms-win-crt-*`，不支持的
VA-based delay imports 拒绝。地址/节检查仍不证明机器码或调用语义，必须另做运行门。

上游 Ninja 会把获准 SDK 的 d3dcompiler_47.dll 复制到隔离构建缓存；成功的交付目录
`payload/` 只含 libEGL.dll、libGLESv2.dll，不包含或分发该 SDK DLL。
`build.json` 绑定 stage、全部构建 owner、环境、Python、Rust API、GN trace、各阶段
命令/日志及 DLL 摘要。`buildToolsExecuted=true`，但 completeSourceClosure、
completeToolchainClosure、runtimeVerified、sdkInstalled、runtimeActivated 和
releaseApproved 保持 false。构建成功不是完整默认源码/工具/许可闭包、ACL 信任、
可信安装或实际产品验收；特别是 79 个 gitlink 仍有 66 个未提供，完整 Ninja 图也有
缺失 libc++/llvm-libc 的非本次目标边，不能从两 DLL 构建成功推导全图 closure。

本轮全部环境/adapter/profile/staging/build 合同为 81 passed / 0 failed / 0 skipped；
其中包含必需函数可执行地址与可选数据导出的正反分支，以及真实 Python Job 生命周期。
Rust fmt、版本一致性、全库 3,442 文件行数 ratchet 通过，八个新文件为 74–167
有效行；原 JSON owner 的必要窄接线为 94→96 行，没有新增行数例外。
发现最后 PE 缺口时主动停止了绑定旧源码的首轮构建，保留 ABORTED.json 和原始日志；
该现场不是最终构建通过证据，修复后必须另建 stage/output，不能沿用旧回执。

最终修复版已在全新目录实际消费仓库 staging/build CLI：暂存 12,569 个构建文件，
GN 生成 513 targets/135 files，504 步（489 条 Clang `/MT` `/WX` 命令）完整两 DLL
编译耗时 558.765 秒并通过无工作门，完整编译 stdout/stderr 没有 warning/error 诊断。
这次是最终源码绑定的新树真实重编译，不再仅靠另一棵树的内容等价关联。两 DLL 的
x64/52 必需可执行导出和普通/delay-import 静态 CRT 门通过；本次字节与 v331 两 DLL
相同，但不将这一重复结果泛化为跨环境的可复现构建证明。

新建显式夹具另外重放已绑定公共库外部驱动，硬件/WARP 六 context、18 次像素对照、
12 成功 child/两无效 GLSL 拒绝通过，像素零差异、child 父进程不加载 ANGLE。
回执在 linshi/otter-native-v332-regression，最终构建与重放在其 final-run 子目录。
原生 1,767 项输入和旧产品 EXE 保持不变；未构建/运行新产品 EXE，未重跑完整桌面库。
实际 Loom、可信安装、完整默认源码/工具/许可、JavaScript 呈现、资源长稳和最终手测
继续开放，v324 UIA/外部前台和 v325 间歇启动失败不追溯放行。默认 Installation=None/
关闭 capability 不变，没有组包、系统 SDK 安装、注册表修改、SDK 分发或提交推送。

## v331：固定本地源码与工具暂存入口

`scripts/native-angle/stage_build.py` 是固定 profile 的暂存编排 owner，复用
`prepare_profile()`；不是下载器、SDK 安装器、完整依赖闭包或编译执行入口。
固定源 commit/tree、挂载点和工具 SHA-256 只由 `stage_pins.py` 维护，不解析 HEAD、
自动升级或从调用者工作树复制源码。`stage_git.py` 使用显式 Git 的只读
`cat-file --batch`，独立校验 commit/tree/blob 原始对象身份；不会执行 filter、hooks、
gclient 或上游脚本，也不进行 CRLF 猜测转换。gitlink 仅登记；提供独立 pinned
checkout 的挂载项须与其对象一致，其他项不跟随或下载。

```powershell
rtk proxy python -I -B scripts/native-angle/stage_build.py `
  --config $ConfigurationFile --output $NewOutputDirectory
```

配置是 UTF-8 无 BOM JSON，严格拒绝重复、未知或缺失字段。以下为结构示例，
`sourceRoots` 必须实际列全 `stage_pins.SOURCES` 的 15 个 key；省略号不是合法配置：

```json
{
  "schemaVersion": 1,
  "sourceRoots": {
    "angle": "C:/reviewed-sources/angle",
    "build": "C:/reviewed-sources/build"
  },
  "artifacts": {
    "gn": "C:/reviewed-archives/gn.zip",
    "ninja": "C:/reviewed-archives/ninja.zip",
    "clang": "C:/reviewed-archives/clang.tar.xz",
    "chromium-rc": "C:/reviewed-archives/chromium-rc.exe"
  },
  "toolchainConfig": "C:/reviewed-config/toolchain.json",
  "gitExecutable": "C:/reviewed-git/cmd/git.exe"
}
```

上述两项 sourceRoots 仅演示格式，不可直接运行。完整 key 为 angle、build、buildtools、
jsoncpp、jsoncpp-source、testing、abseil、astc、rapidjson、zlib、clang-scripts、
vulkan-headers、spirv-tools、spirv-headers、tools-win；具体 pin 以源码为准。
工具归档须已取得并审核；摘要 pin 不等于数字签名、完整源码或再分发许可。
`toolchainConfig` 使用下述显式 VS/SDK schema，必须提供 x86/x64；SDK 授权仍仅限
已批准的 Otter 隔离构建，不由本命令推定新许可、安装或分发授权。

输出必须是全新仓库外绝对路径，与配置、Git、源树、归档、VS、SDK、SystemRoot
和临时根都不重叠。成功退出 0 并输出 JSON 回执；输入/身份/输出冲突退出 1，CLI
用法错误退出 2。创建输出后的失败留下 `FAILED.txt` 和现场，重试换新目录，不覆盖。
有效输出必须无失败标记、清单可解析且实际内容匹配，不能只检查 stage.json 存在。

- `src/`：原 Git blob、固定 build commit 导出的 LASTCHANGE，以及已审阅 profile
  overlay；`tools/`：精确 GN/Ninja EXE。Clang/Chromium rc 放在上游约定的源码内路径。
- `profile/`：复用 adapter 的环境清单、overlay 和完整 profile 回执，不复制 SDK/MSVC。
- `files.json`：最终 src/tools 文件大小、SHA-256 和来源；被 overlay 替换的文件另记
  原始摘要。`gitlinks.json` 记录未跟随对象及是否由独立挂载提供。
- `stage.json`：配置/归档/Git/自有源码、上述清单和 profile 摘要；关键输入、实际输出
  以及全部 native-angle Python 源码成功前再次校验。固定 `gitExecuted=true`、
  `buildToolsExecuted=false`，完整 source/toolchain closure、runtimeActivated 和
  releaseApproved 仍为 false。

`stage_inventory.py` 独占 Windows 名称空间、流式写入和容量；拒绝每级大小写别名、
file/dir 冲突、保留设备名（含 COM/LPT 上标数字和 CONIN$/CONOUT$）、点/尾空格、
路径穿越与链接。单文件 512 MiB、总写入 2 GiB、30,000 文件，Git 单对象元数据
4 MiB、每源 50,000 tree entries、深度 64。归档 owner 拒绝 symlink、加密 ZIP、
路径别名及预算扩张，不执行展开工具。Git 禁止 lazy fetch、对象替换、全局/系统配置、
外部 alternates 和网络 protocol；每个 reader 有 300 秒终止 timer，正常/异常及
timer 启动失败均回收本次进程和管道。这仍不是 Git/Python/SDK 全部实例内容闭包、
ACL 信任或抵抗并发恶意替换的原子保护。

本轮最终加固版实际暂存 12,569 个构建文件，79 个 gitlink 中 13 个由固定挂载提供，
66 个仍未提供，未完成全部不可达性审计，不能声明完整默认 source closure。最终树
与首轮树的清单和实际构建文件逐项相同；最终树真实 GN 生成 513 targets/135 files，
504 条命令计划与首轮原始日志逐字节一致。完整编译实际发生在首轮树，耗时
857.063 秒并通过无工作门；不是最终树重新编译。两 DLL 的 x64、52 必需导出、无
动态 MSVC CRT 检查通过，完整构建日志没有 warning/error 诊断。新 DLL 摘要不同于
v330，不声称可复现构建；本轮显式新夹具另行验证，不复用旧运行成绩。

硬件/WARP 公共库外部驱动六 context、18 次像素对照、12 成功 child/两无效 GLSL
拒绝通过，零像素差异且 child 模式父进程不加载 ANGLE。54 项合同、Rust fmt、
版本一致性和全库 3,434 文件行数 ratchet 通过，新八文件为 23–132 有效行。
证据在 linshi/otter-native-v331-regression；首次比较器误将 CRLF 归一化文本与原始
日志比较的断言失败保留，原始字节复核一致，没有修改命令计划来掩盖差异。
这是固定 profile 暂存边界，不是实际产品 EXE/Loom/完整桌面、许可闭包、可信安装
或完整 N01–N12 放行；默认 Installation=None/关闭 capability 和旧失败边界不变。

## v330：固定 Clang profile 与原生 GN 自动再生

`scripts/native-angle/prepare_profile.py` 复用既有 adapter，只准备固定
`windows-x64-d3d11-clang-release-staticcrt` profile 的外部 overlay，不下载、复制完整
源码树、执行 GN/编译器或安装 DLL。`gn_profile.py` 独占上游 `.gn`、显式 VS/SDK
断言、静态 CRT profile 断言、gclient overrides 与默认 `out/otter/args.gn` 模板；
三个被改写上游文件有固定 SHA-256，ANGLE revision 固定为
`ded782bca9d5f165d1c4a70124cdc5384043a8b3`，build revision 沿用下述 adapter pin。
不关闭 `/WX`，不修补生成后的 Ninja，也不把本 profile 冒充上游全部默认后端。

```powershell
rtk proxy python -I -B scripts/native-angle/prepare_profile.py `
  --angle-source $AngleSource --build-source $BuildSource `
  --config $ConfigurationFile --output $NewOutputDirectory
```

ANGLE/build/config 与输出都须为已审阅的显式路径，输出必须是全新仓库外目录，
不得与原始 ANGLE、build 或既有工具链根重叠。必须同时提供 x86/x64 环境；
实际目标仍为 Windows x64 desktop Release、MSVC STL、静态 CRT 与 D3D11。
固定 Clang/lld 开启，custom libcxx、Rust、Siso 与 Chrome clang plugins 关闭。
这些是构建选项，不改变 Otter 的 Win32/Shader/JavaScript 功能目标。

成功退出 0 并输出 `profile.json`，绑定配置、当前 Python EXE、上游 anchors、
本工具源码、`adapter.json` 及全部输出摘要；输入/输出冲突退出 1，用法错误退出 2。
创建输出后的失败保留现场与 `FAILED.txt`，重试使用新目录。有效输出必须没有失败
标记、回执完整可解析且清单匹配；仅见到文件名不算成功。关键输入前后复核，仍不是
完整源码/工具内容闭包、ACL 信任或并发恶意替换保护。准备器不决定 CRT 的整个上游
配置图，实际 `/MT` flags 和无动态 CRT 导入必须由独立 GN/PE 消费门另验。

真实消费只在新 linshi 实验树进行，原 checkout 与旧实验不变。初次 GN 缺少
tools/win DebugVisualizers 的失败保留；随后按 DEPS 从官方固定取得
`tools/win@d16e6b55b2bd699735919d8a13a55ff284086603`，核对 Git tree/blob 并暂存。
复用 v329 固定 Clang/Chromium rc 与获授权 SDK，不安装系统组件。GN 成功生成
513 targets / 135 files；504 条命令的实际两 DLL 构建成功，耗时 602.235 秒，
完整 stdout/stderr 扫描没有 warning/error 诊断，不将此有限日志检查称为安全证明。

仓库准备器生成的 overlay 已实际消费；仅将默认 args 模板明确搬到独占实验缓存
`out/clang/args.gn`，有消费记录。Ninja 真实触发 `Regenerating ninja files`，再生
前后 toolchain 字节相同、无 prefix overlay，完整 DLL 无工作门通过。再生后实际
C++/两资源依赖为 5/24/24，三项项目头 mtime 失效/恢复门通过。Clang 的
`/showIncludes:user` 排除 system headers；5 不能与旧 MSVC 的 121 直接比较，也
不是完整 SDK 内容变化会自动失效的证明，完整内容闭包仍是单独门。

新 libEGL/libGLESv2 为 199,680/5,295,616 字节，SHA-256 分别为
`5d530a79bccc19ca5aa8e5d069a2a14a1e0ba43293f347a8d4a73ec9def9c84e` 与
`92b412eaffe0cf943fbad4d79213387695db6c976c7c8be61a0edac701fbd19c`。
x64、12/40 个必需导出、无动态 MSVC CRT 检查通过。显式两 DLL 夹具复用已绑定
的公共库外部驱动，硬件/WARP 六 context、18 次像素对照及 12 成功 child/两次
无效 GLSL 拒绝通过，像素全为零差异、child 父进程不加载 ANGLE。不是新产品
EXE、实际 Loom 或完整桌面验收；直接 context 销毁不要求上游 lazy loader 立即卸载。

证据在 linshi/otter-native-v330-regression 的 generation、static-verification、
runtime 子目录，固定源构建在 linshi/otter-angle-fixed-source-20261024 的
clang-profile-20261024，profile 输出在同根 gn-profile-v330。固定 profile 的原生
再生边界已闭环，但完整源码/工具 staging owner、许可闭包、可信安装、实际产品
重放和完整迁移仍未完成；默认 Installation=None/关闭 capability 不变。

本轮全部 adapter/environment/profile 合同为 35 passed / 0 failed / 0 skipped；
Rust fmt、版本一致性、全库 3,426 文件的有效行数 ratchet 均通过，无超限或新增
例外。原生代码未改，本轮未重跑完整桌面库，不追溯放行旧 UIA/间歇启动失败。

## v329 实际 SDK 与 GN 消费证据

经用户明确授权，仅在 linshi 内解包并用于 Otter 隔离构建的 SDK common/x86/x64
10.0.26100.1 包已形成 10.0.26100.0 layout。6,887 个展开文件均记录大小和
SHA-256；没有系统安装、注册表修改、SDK 分发或运行时激活。已有 MSVC
14.37.32822 与此 SDK 经 adapter 的真实 x86/x64 NUL 环境完成 C++20、D3D
import library 链接、rc 与 mt 探针；产物架构正确、无动态 MSVC CRT 导入，探针
EXE 没有执行。这不是完整 ANGLE 构建或 SDK/MSVC 全部内容闭包证明。

固定源码另复制到隔离实验树，逐文件核对 Git blob；checkout 的显式 CRLF 属性
使用原 Git object materialize，不猜测转码或修改 checkout。实验生成 DEPS 导出的
gclient_args 和固定 build commit 的 LASTCHANGE 元数据，绑定实际 Python、GN、
Ninja。实验明确采用 x64 desktop Release、静态 CRT、symbol_level=0、MSVC/STL
与 D3D11 后端，不冒充上游全部默认后端；Vulkan system-info 保留并补齐真实
Vulkan-Headers/SPIRV 源码消费。Rust/Siso/Clang 插件关闭，不涉及应用 JavaScript
或 Shader 合同裁剪。暂存适配以精确根及 profile 断言取代 VS 发现/WDK 假路径，
只在此静态无符号 profile 中省去无关 CRT/debugger/DIA DLL staging。

真实 GN 暴露空位置参数被丢失的问题，v329 在下述 marker 接缝修复。新增回归
先红后绿，连同显式、不继承的 VSLANG 请求合同，完整 adapter 门为
25 passed / 0 failed / 0 skipped。最终
`gn gen --fail-on-unused-args` 生成 111 targets / 102 files，libEGL/libGLESv2
Ninja dry-run 的 402 条命令通过。GN trace 的实际脚本只有 setup_toolchain、
compute_build_timestamp、get_concurrent_links 和 get_cpu_count，没有 vs_toolchain
或 vcvarsall。angle_common、translator、libGLESv2_static 与 libEGL_static 目标另以
Ninja -j2 实际编译成功；静态 target 的通过不等于其单个 archive 已自包含所有依赖，
也不能据此宣称新 DLL 的硬件/WARP Shader 重放通过。MSVC 实验的 zlib 编译日志
含 D9002（忽略 /clang 选项），不称 warning-free 或最终发行 profile 已通过。

真实 MSVC 只有中文 clui.dll；请求 VSLANG=1033 仍输出中文 /showIncludes，初次
对象的 Ninja 头依赖计数为 0。单独的生成文件适配保留中文原文，将实际
`msvc_deps_prefix` 配为 `注意: 包含文件:` 后，同一对象记录 121 个头依赖。
临时变更实验树头文件 mtime 会触发重编译 dry-run，恢复原 mtime/内容后无工作，
完整 DLL dry-run 仍通过。VSLANG 只是显式语言请求，不保证本机有英文资源；
消费方必须验证真实前缀与依赖失效。这条 post-GN 适配尚未成为仓库可复用 owner，
不能让下一次 GN 覆盖后静默丢失，也不能用首次静态成功放行增量构建。

上述为首次阶段证据；后续 DLL 与显式运行结果见下节，旧回执不改写。这些仍是
linshi 内实验接线，不是仓库完整 GN staging/构建入口。完整工具/内容闭包、GN
再生安全、可信安装、实际产品 EXE 重放、长稳和最终手测仍是独立门。证据根分别为
linshi/otter-angle-fixed-source-20261024 与 linshi/otter-native-v329-regression。

## v329 后续：固定资源工具、完整 DLL 与显式离屏重放

未分配新的内部版本，原生及 adapter 源码未改。依据固定 tools/clang 脚本直接从
官方 HTTPS 获取 clang-llvmorg-23-init-10931-g20b6ec66-8.tar.xz，归档 SHA-256 为
`7fa1aa9bf477f565687a2f516bf75ff6bfdd13da9b36bf521a75ac8ae2761d6a`。
Chromium rc 按源码固定 SHA-1 `dea7da0d9bfcb8dad6cf53ca608dddac3459faaa` 获取，
另记 SHA-256 `903769d98df8a005c79d52d7089fe34f23c0b7ada4cd0c42b6063526a5593e6a`。
均绑定官方对象 generation、响应头、大小及服务器 MD5；这不是数字签名证明，
两个 EXE 实际无 Authenticode 签名。归档 350 项无链接，安全检查后展开 330 文件，
连同 rc 共 331 文件逐项散列；不执行更新脚本/hooks，不安装系统工具。
保留 LLVM 固定完整提交的官方身份/许可及 Chromium 来源许可材料，但不声称历史
rc 二进制完整源码闭包、工具重现构建或分发批准。

同一隔离 profile 下，libEGL/libGLESv2 实际 DLL 构建成功（401 个待执行步骤，
482.688 秒），后续 Ninja 无工作。两 DLL 分别为 190,464 与 5,052,928 字节，
SHA-256 分别为 `5b413d8ce9e489708f1a4a8ed7c41b3c0137bab051c70ab2782e3535a2b9309e`
与 `f4dc739f81c2c585986fd34ef70cbabe8aaa9f9af3f5af009161e1c361e5bdb8`。PE 为
x64，当前 Otter 所需 EGL/GL 的 12/40 个导出齐全，无动态 MSVC CRT；GLES 的
Windows API-set 是虚拟 loader contract，不伪称磁盘文件。44 条 D9002 仍保留，
不是 warning-free 最终发行 profile。上游资源 wrapper 实际运行 Clang、Chromium
rc 和 SDK rc 比对，没有用 SDK 探针替代。仅 C/C++ 规则配置中文依赖前缀，RC
保持英文；C++/两资源对象分别记录 121/24/24 头依赖，三个真实头文件 mtime
失效及恢复后无工作均通过。生成文件 overlay 仍不保护未来 GN 再生。

新建仅含两 DLL 的显式夹具，以新鲜编译、源码未改的 Otter 公共库链接外部 Rust
证据驱动，不修改或重标旧产品 EXE。硬件 NVIDIA RTX 2080 Ti 与 WARP 的六次
context 创建/销毁、18 次真实旧 ShaderRenderer 合同像素对照通过；隔离 child
另有 12 次成功渲染和两次无效 GLSL 拒绝，父进程每次均未加载 ANGLE。alpha/方向
不放宽，复杂 tone 仅沿原合同允许 RGB 一字节舍入；本次所有对照实际为零差异。
这不是实际产品 EXE、Loom 服务或完整 UI 验收。首次直接加载诊断误要求 context
销毁后立即卸载 DLL，失败已保留；上游 EGL lazy dispatch 保留 GLES 模块引用直到
进程退出，不能把 context 生命周期等同模块卸载。独立 child 门的父进程隔离不变。

下载、静态检查、增量与运行回执统一在
linshi/otter-angle-fixed-source-20261024/{resource-tools-20261024,dll-experiment-20261024}。
默认 Installation=None/关闭 capability、用户程序、未安装/未分发/未发布边界不变。

## 责任和接缝

`scripts/native-angle/prepare_toolchain.py` 是 Python 标准库 CLI，输入固定的 Chromium
build 源码、显式 JSON 配置与尚不存在的仓库外输出目录。成功退出 0；输入、身份或
输出冲突退出 1，参数用法错误退出 2。成功输出 UTF-8 JSON 回执；不会修改原始
checkout 或既有输出。创建目录后的失败保留 `FAILED.txt`，重试必须换新目录。

`toolchain_paths.py` 拥有路径/schema、明确版本/架构和关键文件摘要；
`otter_toolchain_env.py` 拥有清单消费与隔离环境。准备器只替换固定上游
`_LoadToolchainEnv` 函数体、拒绝外部 runtime_dirs、移除其 PATH 二次拼接，并明确
SDK Include 根，保留原有 GN flags 与 NUL 环境块输出逻辑。原上游即使收到 GN SDK
路径，仍可能经 vcvarsall/注册表寻找系统 SDK，因此只设置 windows_sdk_path 不够。

固定输入：

- Chromium build revision：`8a46ede8ea86151737d9c284f82d238b3c701fa7`。
- `toolchain/win/setup_toolchain.py` SHA-256：
  `f6ae24c99ae0bfc72b093da415615119652f25415155dffb4db22f49b78ca8cf`。
- `gn_helpers.py` SHA-256：
  `4c99d6d159f0faef3b83ac594ea0cf4d30973c029179721f3525cf87d765f082`。

固定摘要不自动升级。两个上游脚本保留其版权/许可头；本工具不声称已完成完整源码、
依赖或分发许可材料闭包，不能把暂存目录作为可分发的 SDK 或 ANGLE 发行件。

## 显式配置

配置必须是 UTF-8 无 BOM JSON，不能有重复或未知字段。以下路径和 MSVC 版本仅是
结构示例，必须改为已经独立审核的实际目录；工具不创建这些输入目录：

```json
{
  "schemaVersion": 1,
  "visualStudioRoot": "C:/toolchains/reviewed-vs",
  "msvcVersion": "14.44.35207",
  "sdkRoot": "C:/toolchains/reviewed-sdk",
  "sdkVersion": "10.0.26100.0",
  "architectures": ["x86", "x64"],
  "systemRoot": "C:/Windows",
  "tempRoot": "C:/toolchains/build-temp"
}
```

SDK 必须有固定版本的 `Include/{ucrt,shared,um,winrt}`、
`Lib/{ucrt,um}/<cpu>` 和 `bin/10.0.26100.0/x64`；版本目录均在根下的标准位置。
MSVC 必须有 `VC/Tools/MSVC/<msvcVersion>/{include,lib/<cpu>,bin/Hostx64/<cpu>}`。
清单绑定 vcruntime/stdio/Windows 头、vcruntime/libcmt/ucrt/kernel32 库，以及
cl/link/rc/mt 关键文件；这里只读取这些文件，不执行它们。

只准入显式 x86/x64 桌面架构，不支持 Store/ARM。可以生成仅含 x64 的数据预检清单，
但固定 GN 的 win_toolchain_data.gni 会读取 x86 和 x64；缺少架构就拒绝，不能把
x64 替代给 x86，也不能把已取得的 common/x64 SDK 包说成完整 GN 工具链。

## 运行与输出

在 PowerShell 中显式指定三个已准备好的绝对路径变量：

```powershell
rtk proxy python -X utf8 -B scripts/native-angle/prepare_toolchain.py `
  --build-source $BuildSource --config $ConfigurationFile --output $NewOutputDirectory
```

输出包括 `environment.json`、`adapter.json`、
`build/toolchain/win/{setup_toolchain.py,otter_toolchain_env.py,toolchain_paths.py}` 和
固定的 `build/gn_helpers.py`。回执绑定输入配置、适配源码与每个输出文件摘要。
关键输入在准备前后复核；失败不生成成功回执，不覆盖已有输出或清理失败现场。

消费时必须显式设置 `DEPOT_TOOLS_WIN_TOOLCHAIN=0` 与
`OTTER_ANGLE_TOOLCHAIN_MANIFEST=<output>/environment.json`。上游传入的 VS/SDK 根
必须与清单一致；清单缺失、版本/架构不符、关键字节变化都失败，不走发现或下载分支。
PATH 首项是获准 MSVC bin，随后是固定 SDK bin、当前 Python 实际目录和显式
System32；不继承环境中的 INCLUDE/LIB/LIBPATH/PATH，不含空段或当前目录。
VSLANG 显式请求 1033 而非继承调用者；该值不是英文资源存在或 Ninja 前缀正确的证明。

本输出的脚本可独立进行数据预检；实际调用仍使用上游六个位置参数，空 runtime_dirs
必须作为真正的空字符串保留。Windows PowerShell 5.1 原生命令参数传递可能丢弃空
参数；合同测试用 Python argv 列表调用，不能用缺失参数的命令当作通过证据。

**生成外部目录不会让 GN 自动改用这个目录。** v329 的 linshi 实验已接入真实 GN，
不代表仓库存在完整 staging owner。消费方仍须审核并绑定实际 GN/编译入口与输出，
不能覆盖原始 checkout 或把本准备器当作执行全部 GN 脚本的安全授权。实验的静态
profile 之外，vs_toolchain、runtime DLL、WDK、clang/Python/工具实例仍须独立收口。

Windows GN 的 exec_script 会丢失空字符串位置参数。接入时显式使用：

```gn
visual_studio_runtime_dirs = "otter-no-runtime-dirs"
```

adapter 只把此精确 marker 还原为空值，仍允许 Python argv 列表中的真正空字符串；
任何其他非空 runtime_dirs 均拒绝。marker 不进入返回 PATH、GN flags 或 NUL 环境块，
没有增加运行库搜索目录，也不允许前后空格/追加路径的近似值。不能传 GN 空列表，
也不能因上游显示有参数位置就认定实际 argv 已保留它。

## 安全与验证边界

路径拒绝 UNC/设备路径、相对或点分量、环境/命令注入字符、链接和 reparse 祖先；
可用已存在的规范映射盘，不创建新映射。JSON 上限 64 KiB，单个 anchor 上限
512 MiB，以 1 MiB 分块散列，无常驻进程或队列。不改变父进程环境。

anchor 不是完整 SDK/MSVC/Python 内容清单；Python 自身和非 anchor 文件仍需闭包。
路径检查与随后读取不是对并发恶意替换的原子保护，尚无 ACL/安装信任保证。
回执不是签名证明，`completeToolchainClosure`、`releaseApproved` 恒为 false，
准备器的 `sdkInstalled`、`toolsExecuted` 恒为 false。应用的默认
`Installation=None`/关闭 Shader capability 完全不变。

测试只在显式临时根生成文本形式的头/库/EXE，不运行这些合成工具。测试会执行已审阅
上游 Python 的数据消费逻辑和当前 Python 解释器，阻断工具链发现与 subprocess；
不是“从未执行任何上游代码”，也不证明真实 SDK、GN 或 DLL 构建可用：

```powershell
$env:OTTER_ANGLE_TEST_ROOT = $ExternalTestDirectory
$env:OTTER_ANGLE_BUILD_SOURCE = $BuildSource
$env:OTTER_ANGLE_SOURCE = $AngleSource
$env:PYTHONDONTWRITEBYTECODE = '1'
rtk proxy python -X utf8 -B -m unittest discover -s scripts/tests -p 'test_native_angle_*.py' -v
```

本门覆盖两种架构/中文路径、旧环境污染、当前目录 cl.exe 抢占、缺失清单、GN 根
冲突、关键文件变化、输入中途变化、输出不覆盖和真实 CLI/独立导入；v330 新增
profile guards、输出清单、双架构/上游 pin/插值拒绝与真实 `-I` CLI 回归。此前缺少
兄弟模块显式导入的 CLI 失败已先红后绿。合成合同不替代上述真实 SDK/GN/DLL 门；
完整依赖/工具实例、可信安装、Loom、资源长稳及最终手测仍是独立门。
