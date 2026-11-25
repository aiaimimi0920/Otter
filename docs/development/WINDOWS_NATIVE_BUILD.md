# Otter 开发构建

Windows x64：使用 Rust MSVC、Visual Studio C++ Build Tools 和 Windows SDK（也可使用
`llvm-rc.exe` 编译 Windows 资源）。Node.js 22 用于版本及打包辅助脚本，不需要安装 npm 依赖。
Python 3.11+ 用于归档启动探针和联合测试。首次联网拉取锁定的 crate 和 NeuroDesign Git 提交；
无需相邻 NeuroDesign checkout。仅在资源编译器无法从 PATH 找到时设置 `OTTER_NATIVE_RC`。

```powershell
cargo fetch --locked --manifest-path native/Cargo.toml
npm run dev
npm run verify:local
```

`npm run dev` 同时构建主程序和配套 Live DLL。快速并行检查、子进程隔离单测与定向回归示例：

```powershell
npm run check:fast
npm run test:nextest -- -p otter-core -p otter-media -p otter-runtime
cargo nextest run --locked --manifest-path native/Cargo.toml -p otter-native --lib --features test-live -E 'test(/live_hover/)'
cargo nextest run --locked --manifest-path native/Cargo.toml -p otter-native -E 'test(/loom_handshake/)'
node --test scripts/tests/effective-code-lines.test.mjs scripts/tests/review-batches.test.mjs scripts/tests/review-ledger-upsert.test.mjs
python -m unittest discover -s scripts/tests -p 'test_ci_*.py'
```

## 内部 ZIP 与实际启动

在 Otter 根目录运行，三个输出目录使用新路径；构建缓存可以重用，但不能同时写入。

```powershell
.\scripts\build-native-candidate.ps1 -OutputDirectory C:/build/otter-candidate -TargetDirectory C:/build/otter-target
.\scripts\package-native-candidate.ps1 -CandidateDirectory C:/build/otter-candidate -OutputDirectory C:/build/otter-package
$zip = Get-ChildItem C:/build/otter-package -Filter '*-internal.zip' -File
.\scripts\verify-native-package.ps1 -ZipPath $zip.FullName
python scripts/tests/native-package-runtime.py --package-dir C:/build/otter-package --evidence-dir C:/build/otter-runtime
```

原版包、来源摘要及证据互相核对。正式 Release 包默认剥离开发期 `--smoke-test` 自检模块（完整 smoke 由
Debug 集成测试覆盖；Release 集成 smoke 使用 `cargo test --release --features smoke-test`。
单测所需 helper 在 `cfg(test)` 下保留，默认 Release 集成测试另验 smoke 拒绝入口）。解包后从
ZIP 启动，以隔离 profile 执行 `--workspace-check`、带外部 PNG 资产的 `--session-check`（含隐藏建窗、
Direct2D 绘制与设备重建及输入 PNG 未修改核验），核对空 profile 不生成工作区文件，
并验证 `--smoke-test` 与非法参数入口按预期拒绝。Release 保留默认 unwind，避免绕过 worker panic
错误交付和文件替换的析构恢复；体积优化不能改变这些恢复契约。
两项隐藏启动检查的防卡死预算为 60 秒；三项参数拒绝入口为 15 秒。探针核对完成后没有遗留会话夹具目录。
回执同时记录实际耗时、预算、退出码与成功标记；这些预算不作为冷启动性能指标。
不替换运行中的 Otter。真实桌面联测按 [Live 联合测试](../features/LIVE_JOINT_TESTING.md)
和[投射联合测试](JOINT_TESTING.md)进行。
完整包上手需要保留 EXE 和相邻 DLL；内部包不签名、不对外发布。

## 意外退出排查

保留问题包、工作区与原始诊断，记录 EXE/DLL 摘要、启动参数、PID 与退出码，区分正常退出、
外部终止与程序错误。先按本页验证 ZIP 的实际启动，再用独立 profile、自有源窗口复现，不关闭
用户实例或发送全局输入。涉及 Live 时检查源窗口和恢复 journal；进程退出本身不证明输入、
悬浮窗口或源权限已归还。
