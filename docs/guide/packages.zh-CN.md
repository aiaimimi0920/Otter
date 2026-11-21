# 发布包、签名与本地数据

[English](packages.md) · [文档中心](../README.md)

当前开发包是未签名的 Windows x64 内部 ZIP，不是公开发行版。完整解压并运行 `otter-native.exe`，保留同目录 Live DLL。
GitHub 的历史公开下载仍按各自发布说明使用。主干没有安装器或公开签名入口。

可用 `OTTER_NATIVE_PROFILE_DIR` 为测试选取独立目录。默认工作区和旧版目录不是同一目录；
不要覆盖正在使用的 profile、移动凭据或把清理旧源码当作迁移本地用户数据。
见[构建与校验](../development/WINDOWS_NATIVE_BUILD.md)和[发布来源](../release/release-provenance.md)。
