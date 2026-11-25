## Native action feedback. Diagnostic details and user content are passed through unchanged.
action-new-window-failed = 无法新建工作窗口。
    { $error }
action-new-pin-failed = 无法创建新钉图，原文档保留。
    { $error }
action-hotkey-failed = 无法切换 Ctrl+1 全局截图快捷键。可能已被其他程序占用。
    { $error }
action-hotkey-degraded = 全局截图快捷键（{ $shortcuts } 不可用）{ "\u0009" }Ctrl+1 / Ctrl+2 / Ctrl+3
action-hotkey-menu = 全局截图 / Live / 长截图录制{ "\u0009" }Ctrl+1 / Ctrl+2 / Ctrl+3
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
