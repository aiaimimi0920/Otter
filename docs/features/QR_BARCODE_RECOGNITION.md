# OCR 扩展：二维码与条码

与[投射邀请二维码](QR_PROJECTION.md)不同，扫描钉图上的码属于 Loom 可选 OCR 能力。
Loom 负责能力包、安装、授权及解码结果；Otter 负责对所选钉图发命令、挂载附件和呈现授权的 Surface。
可用的码制、结果和操作由当前已启用的能力包提供，不是 Otter 内置的解码合同。

- 选中图像后，`Ctrl+4` 请求 OCR；`Alt+4` 切换 OCR 结果图层。宿主识别
  `neuro.official/ocr.codes.v1` 和文本 OCR 附件的可见性，具体覆盖场景以能力返回为准。
- 识别内容是数据，不会自动打开链接或执行程序。链接等命令须走受授权的宿主操作；
  识别、可信能力清单或一个 URL 本身都不赋予外部执行权限。
- 没有匹配能力时，本地截图和编辑仍可使用；不要把导入投射邀请当成通用扫码入口。

Otter 侧变更从 [`extension_command_route.rs`](../../native/src/desktop/extension_command_route.rs) 和
[`extension_visual.rs`](../../native/otter-media/src/extension_visual.rs) 核对，运行 `cargo test --locked --manifest-path native/Cargo.toml -p otter-media --lib extension_visual`。
能力包的码制和解码回归由 Loom 负责，Otter 的宿主测试不替代这些验证。
