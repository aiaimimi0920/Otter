# LiveRelay 协议

Otter 与 Loom 按当前版本配套。Loom 的 `protocol/LIVE_SCREENSHOT_PROTOCOL.md`
及 `protocol/schemas/live-control.v1.schema.json` 定义服务端合同；Otter 管理采集、
显示和本地窗口。不要把协议名里的 `v1` 当成旧客户端模式。

## 控制、媒体与性能

- `loom.live.v1` 控制消息使用 `protocolVersion`、`sessionId`、`epoch`、
  `sequence`、`messageType`、`payload`。会话、权限、按键和按钮边沿、观察及触发审计
  必须有序；只有可替换的鼠标移动样本允许合并。
- `/v1/live/media` 是独立 WebSocket 媒体通道。`NLLV` 64 字节公共头包含 epoch、
  frame ID、采集/编码时间、尺寸、关键帧、丢帧计数、色彩空间、codec 和正文长度。
  帧不写入工作区，也不进入 Surface 控制轮询。
- WebSocket 子协议确定媒体 profile：`loom.live.h264.v1`、`loom.live.jpeg.v1`
  或 `loom.live.v1`（raw BGRA）。H.264 依赖源端编码器、服务端视频策略和接收端
  解码器；当前版本在不支持视频或解码链失败时仍用图片，不需要旧客户端。
  JPEG 是独立关键帧，未经 JPEG 协商不能收；H.264 必须按序解码，不能当作独立图片。
- 帧头、尺寸、序列、codec 与分配预算在接收边界核对；损坏帧不替换可用画面，
  epoch 变化不会混用旧帧。运行时由 `native/src/live_relay_*` 管理；见
  [接收诊断](LIVE_RELAY_DIAGNOSTICS.md)及[联合测试](LIVE_JOINT_TESTING.md)。
  解码提交和收包计数不等于物理显示 FPS 或跨设备端到端延迟。

## 授权与恢复

选择会话或加入观看不授予输入权。设备、Surface attachment、controller lease、
epoch、授权期限和撤销状态须分别核对；Windows 输入仍受当前用户、会话、完整性级别
及安全桌面限制。公开发现不返回观察值，正常日志不保存标题、像素、键盘或 OCR 内容。

续签重用原签名设备身份与成员关系，不重新配对、不重放输入、不重获 controller lease。
接收端从认证快照的 `requesterControl` 确定游标，新连接从确认的 `viewer_joined`
事件开始；身份、epoch、快照或成员关系不匹配时停止。`requireExistingMembership`
由 Loom 原子校验。源端恢复先退役旧 worker，再从已确认的观察序列继续；旧缓存值不能
直接变为新的可信观察。

Device 媒体连接收到 `1008` 且原因为 `live_media_device_revoked` 时关闭本地 relay、
释放授权并清掉接收帧。普通断线不等于设备撤销，HTTP 401 的认证处理也不靠匹配正文。
失败不会触发未经确认的新加入或重放。远程媒体须使用当前授权的 HTTPS/WSS；本机
回环单独受限。没有公共 NAT/TURN 中继时不宣称具备该能力。

## 观察与触发

UI Automation 在专用线程读取当前允许的控件；稳定 locator 冲突、未知或过期值不会
推断成 `exact`。快照和事件有界，观察序列与媒体帧号独立。Otter 校验并转交绑定到
Surface 的触发请求；Loom 负责可信值、稳定时长、权限、幂等和实际 Art 执行。
视觉/OCR 等可选能力由验证后的 Loom Capability Plugin 提供，不由 Otter 核心暗中启用。

另见[实际使用与验证边界](LIVE_CAPTURE.md#compatibility-and-validation)。
