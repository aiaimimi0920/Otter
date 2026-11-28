## Native action feedback. Diagnostic details and user content are passed through unchanged.
action-new-window-failed = 无法新建工作窗口。
    { $error }
action-new-pin-failed = 无法创建新钉图，原文档保留。
    { $error }
action-hotkey-failed = 无法切换 Ctrl+1 全局截图快捷键。可能已被其他程序占用。
    { $error }
action-hotkey-degraded = 全局截图快捷键（{ $shortcuts } 不可用）
action-hotkey-menu = 全局截图 / Live / 长截图录制
action-hotkey-suspended = 截图快捷键已在全屏期间暂挂
capture-loupe-keys = Tab 换格式 · Z 显隐
capture-size-more = Z显示更多
action-shortcut-settings = 在 Loom 中配置快捷键…
action-shortcut-settings-guide = 请将本机 Otter 连接到 Loom，在 Loom 的「设置 → Otter → 快捷键」中选择本设备并应用配置。断开连接后仍可使用上次保存的键位。
action-shortcut-import-conflict = 导入文件的快捷键与本配置不同。请通过 Loom 修改快捷键，不要通过命名文件导入。
action-open-failed = 无法打开图片，原文档未被替换。
    { $error }
action-replace-one-image = 替换图片必须恰好包含一张图片，原文档保留。
action-replace-editable = 可编辑内容不能替换当前图片。
action-replace-stale = 加载期间文档已修改，已取消替换。请重新打开图片。
action-import-failed = 无法导入图片，原文档保留。
    { $error }
action-drop-count = 一次最多拖入 32 张图片，空批次不可导入。
action-drop-path = 拖入文件的路径为空或过长。
action-drop-incomplete = 无法读取完整的拖入文件路径。
action-image-loading = 图片仍在加载，请等待完成或按 Esc 取消。
action-open-paths-invalid = 文件选择结果不完整、路径非法或超出批次限制。
action-flip-failed = 翻转未提交，原图与标注保留。
    { $error }
action-erase-failed = 擦除未提交，原文档保留。
    { $error }
action-crop-failed = 裁剪未提交，原画框及效果保留。
    { $error }
action-resize-failed = 缩放未提交，原标注保留。
    { $error }
action-rasterize-failed = 栅格化未提交，原标注保留。
    { $error }
action-blur-failed = 模糊准备失败，原图和标注未修改。
    { $error }
action-text-failed = 无法开始文字输入。
action-text-draft-rejected = 文字未保存。草稿超出文本限制或可用内存预算。
action-text-draft-unavailable = 无法显示当前草稿。文字仍保留在编辑框中。
    { $error }
action-font-failed = 无法应用字体。
    { $error }
action-cache-failed = 缓存设置未修改。
    { $error }
action-copy-unavailable = 无法调度复制任务，剪贴板未修改。
action-copy-failed = 复制未完成。
    { $error }
action-paste-failed = 无法粘贴图片，原文档保留。
    { $error }
action-paste-no-image = 剪贴板中没有可用的图片。
action-paste-one-file = 替换图片只能粘贴一个文件。
action-clipboard-files-invalid = 剪贴板文件列表无效或超出 32 个文件的批次限制。
action-crop-selection = 裁剪选区
action-reset-crop = 重置裁剪
action-rasterize-selection = 栅格化选中标注
action-rasterize-all = 栅格化全部标注
action-rasterizing = 栅格化处理中…
action-receive-upstream = 接收上游图片
action-inline-text = 输入标注文本
action-zoom-preview = Otter · 缩放预览
action-serial-radius = 序号半径必须为 8–999 图像像素，原设置未修改。
action-session-unsaved = 工作区尚未安全保存，请完成操作或先退出 Otter。
action-configuration-changed = 节点配置已变化，旧准备已取消
action-hdr-unrestored = HDR（原图文件未恢复）
action-capture-unrecorded = 未记录
action-untitled-window = 无标题窗口
action-window-limit = 仅显示前 64 个窗口
action-settings-recovery-title = Otter 配置恢复
action-settings-recovery = 配置原文件缺失，已读取唯一恢复副本。
    原路径：{ $target }
    恢复副本：{ $recovery }
    文件未修改；请使用命名设置另存为，不能直接覆盖保存。
action-naming-import-failed = 命名设置未导入。
    { $error }
action-naming-save-failed = 文件与缓存设置未保存。
    { $error }
action-naming-save-warning = 文件与缓存设置已保存，但旧配置恢复文件未清理：{ $path }
    { $error }
action-naming-autosave-failed = 自动保存已停止；当前设置仍保留，请解决错误后重新开启。
    { $error }
action-naming-load-first = 请先加载配置或将设置另存为，再开启自动保存。
action-naming-recovery-save = 恢复来源必须先另存为，不能开启自动覆盖保存。
action-naming-no-source = 尚未加载配置文件，请先导入或通过 --settings 启动。
action-naming-recovery-update = 当前配置来自恢复副本，请先另存为；恢复文件不会被覆盖。
action-naming-new-file = 请选择新文件；文件与缓存设置另存为不覆盖现有文件。
action-naming-import-title = 导入命名设置（仅本次运行，不修改源文件）
action-naming-save-title = 文件与缓存设置另存为（仅创建新文件）
action-live-control-failed = Live 源窗口控制不可用。采集画面仍保留。
    { $error }
action-live-stopped = Live 已停止。
    { $error }
action-live-retain-failed = Live 画面未保留。
    { $error }
action-live-start-failed = 无法开始 Live。
    { $error }
capture-loupe-hex = HEX · 8 位 sRGB
capture-loupe-rgb = RGB · 8 位 sRGB
capture-loupe-hsl = HSL · sRGB
tray-delay = 延迟截图
tray-delay-three = 3 秒后截图
tray-delay-five = 5 秒后截图
tray-delay-cancel = 取消延时截图
capture-countdown-badge = { $seconds } 秒后锁定画面
archive-menu = 图像自动归档
archive-copy = 归档已复制的图像
archive-pin = 归档新建的截图钉图
archive-directory = 选择归档目录…
archive-select-directory = 请先选择一个已存在的本机归档目录。
archive-config-failed = 归档设置未更改：{ $error }
archive-consent = 本次运行期间，Otter 将在 { $directory } 中保存 PNG 图像，不覆盖已有文件。这些是永久文件，不是剪贴板缓存。是否开启？
archive-queue-full = 图像归档队列已满。
archive-failed = 图像归档失败：{ $error }。原操作不受影响。
paste-card-invalid = 剪贴板文本须有效，最多 65,536 个 UTF-16 单元、400 行。
paste-card-budget = 此文本卡超出了排版尺寸或图像内存上限。
edit-rect-mask = 矩形打码
edit-text-wrap = 文本框折行
edit-box-width = 文本框宽度
edit-box-padding = 文本框内边距
export-webp-lossless = WebP（无损）
tray-token-elevated = 当前会话：已使用管理员权限
tray-token-standard = 当前会话：未提权
tray-token-unknown = 当前会话：无法读取提权状态
loom-needs-standard-session = 如需使用 Loom 或设备投射，请退出 Otter，再双击以普通权限打开。
export-webp-lossy = WebP（有损）
export-quality-webp = WebP 画质：{ $quality } · 透明度无损保留。
export-quality-lossless = 无损编码 · 保留原有透明度。
export-quality-white = JPEG 画质：{ $quality } · 透明区域使用白色背景。
export-size-pending = 正在编码预览…
export-size-exact = 编码体积：{ $bytes } 字节
export-size-failed = 预览不可用：{ $error }
export-extension-mismatch = 文件扩展名与选择的图像格式不一致。
tool-spotlight = 聚光灯
tool-shape = 形状
tool-shape-rect = 矩形
tool-shape-round = 圆角矩形
tool-shape-ellipse = 椭圆
tool-spot-rect = 矩形聚光灯
tool-spot-round = 圆角聚光灯
tool-spot-ellipse = 椭圆聚光灯
tool-spot-density = 聚光灯遮罩浓度
tool-magnifier = 局部放大镜
tool-lens-zoom = 放大倍率
