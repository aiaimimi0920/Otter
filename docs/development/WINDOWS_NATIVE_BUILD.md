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

`npm run dev` 同时构建主程序和配套 Live DLL。定向回归示例：

```powershell
cargo test --locked --manifest-path native/Cargo.toml --lib loom_handshake:: -- --test-threads=1
node --test scripts/tests/effective-code-lines.test.mjs
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

原版包、来源摘要及证据互相核对。解包后从 ZIP 启动，以隔离 profile 执行 smoke 和参数拒绝；
不替换运行中的 Otter。真实桌面联测按 [Live 联合测试](../features/LIVE_JOINT_TESTING.md)
和[投射联合测试](../features/QR_PROJECTION.md#从仓库运行联合测试)进行。
完整包上手需要保留 EXE 和相邻 DLL；内部包不签名、不对外发布。
