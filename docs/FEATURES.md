# Otter Features and Manual Regression Matrix

## 空工作区托盘

桌面没有钉图时，点击或键盘激活托盘图标保持空工作区，不显示黑色空宿主。
已有内容的召回、托盘右键菜单和截图快捷键保留。回归覆盖空宿主、普通图片和无图 Art；
手测全新启动、删除最后一张钉图后的托盘点击，以及已有钉图隐藏后的恢复。

## 未发布 Windows-native 分支：v2 账号分享单入口（v664，验证中）

两端均使用最新 Otter、Loom，分别连接自己的本机 Loom 并登录同一账号。二维码和链接只使用账号 v2，
取消旧模式切换、旧邀请导入和协议回退。创建、导入、持续 PNG 更新、重开和解绑进入一个原生账号生命周期。
接收图只有真实检查点提交后才显示；暂停撤销在途处理，解除关联保留末图。

真实双 Loom Application 联合已验证创建/接受、两次 PNG 更新、双方重启和解绑。单入口调整后的二维码、窗口、
账号与恢复 34 项定向回归已通过。v664 完整内部包已完成来源、归档及解包 EXE 工作区回归；
普通分享菜单仍待验，双 Loom 局域网 Live 尚未实现。公网 Live 与外网验收按用户授权暂缓。
以下带旧版本号的记录是历史状态，不构成当前兼容或旧版回归要求。

## 未发布 Windows-native 分支：右键图库滚动、删除与预览（v653）

回收站、收藏夹和剪贴板图片按三列卡片展示，滚轮按鼠标所在面板整行滚动。每条右上角“×”直接删除对应条目，键盘选中后也可按 Delete；正文点击仍使用原库操作或剪贴板粘贴。两库删除复用原事务，剪贴板删除按 Windows 历史 ID 定位，系统拒绝时显示失败，不清空整个历史或当前剪贴板。

剪贴板文字与图片悬浮时在网格右侧显示较大的阅读预览，图片保持完整比例，右侧空间不足时放在左侧。预览不接收条目操作；跨卡片、滚动、刷新与在预览内松开鼠标均撤销旧点击。缩略图加载保留滚动位置，显式翻页从页首开始。两库缩略图只为当前页后台准备；剪贴板阅读图在既有 worker 中准备，长边最多360像素并计入原总预算，不在 UI 解码。

手测三处菜单的零项／一项／多行、文字与图片混排、滚轮与分页、卡片边缘点击、恢复／复制／删除和不同显示缩放。本轮只做针对性检查及手测整包，不把编译视为普通桌面视觉验收。

## 未发布 Windows-native 分支：主干菜单与端点光标（v651）

钉图右键依次为保存、关闭、回收站、清空回收站、收藏／取消收藏、收藏夹、清空收藏夹和剪贴板，保留主干分组；关闭沿用回收事务。收藏状态由原有界库索引 worker 读取，未知时禁用，不在窗口回调解析整库。清空只更新选定库的内存会话原文，沿原 Receipt／单 worker 结算，不直接删除图片文件。

托盘依次为截图、实时截图、长截图、编辑已有图片、接收投射、自启动、退出，不追加剪贴板或冻结库。接收投射复用现有接收策略与收件菜单，不绕过授权或自动接受。端点条默认箭头，不使用会在连接结束后残留的类十字光标。手测拖连成功／空白松手／Esc取消、右键关闭与回收、两库清空／收藏切换、剪贴板和托盘各入口；真实跨设备接受与桌面观感不由菜单树回归替代。

## 未发布 Windows-native 分支：普通截图交付延迟（v650）

普通Ctrl+1的采集结果与钉图准备不再等隐藏采集25ms／导入100ms轮询，完成后立即沿原交付链处理，不提前采样旧画面或降级HDR。手测不同大小的选区、连续多次截图、Esc取消及已有多钉图场景，对比松手后出现钉图的等待感；真实首帧延迟不能由隐藏HWND和合成像素回归推定为零。

## 未发布 Windows-native 分支：启动与粘贴位置（v615–v618）

v618 修正 Windows 关闭剪贴板时补充图片格式导致发布序列号变化、内部来源失效的问题。新增隔离 station 的真实系统发布/读取回归，检查六次读取仍保留来源与位置会话，并交付独立窗口；可见 A 内十二个不同位置的累计落点由原私有 desktop 回归独立检查。物理 Ctrl+C/Ctrl+V 及第三方剪贴板工具的交互仍需手测。

普通无参启动使用 GUI 子系统，不弹出控制台；全新及恢复的空工作区只保留隐藏命令宿主和托盘，已有钉图、显式图片启动仍按原流程显示。手测双击 EXE、恢复有图／空工作区、托盘和截图快捷键。隔离产品测试不使用 `CREATE_NO_WINDOW` 掩盖控制台，另核 PE 子系统。

复制时冻结鼠标相对图像的偏移；复制来源 A 的当前可见范围优先：在 A 内任意位置粘贴，均以 A 左上角累计偏移 20、40、60……DIP（按复制来源 DPI 转物理像素），不以鼠标静止或副本的遮挡层序作为堆叠条件。A 外的其它可见目标仍按原目标规则定位；鼠标移到图外则使用鼠标减复制偏移。该规则与可编辑性分离，普通图片、Art 和缩略图都保留位置；Art／缩略图仍是扁平图片，不恢复执行权限或可编辑 Art。外部 PNG／DIB／文件使用粘贴时鼠标作为左上角；取消、准备失败和新复制不会错误推进旧锚点。手测十次以上连续粘贴、另一张钉图内粘贴、来源移动、多屏、不同 DPI 和外部剪贴板覆盖。

Shift 拖出继续使用主干默认 `{label}_{shortId}_{date}_{time}`；普通图片为 `image_<ID 后四位>_<YYYYMMDD>_<HHmmsscc>.png`，自定义模板与同名递增保留。定向回归检查默认名到缓存发布及真实 Shell 文件夹的最终名字；该验证不替代物理拖动与用户旧配置验收。

## 未发布 Windows-native 分支：底栏层级、稳定三角与正确的互斥对象（v614）

底部黄色三角及设置条与所属钉图紧邻，并跟随其普通／置顶状态。展开前后保留相同宽度和水平坐标，三角直接切换到条下方；收起仍仅三角区域可见、可点，不增加两侧空白命中区。

同一钉图的端点内容与编辑工具栏首个按钮打开的左侧“钉图设置”双向互斥。点击左右端点、Tab 总展开和 F2 打开参数时收起左侧属性栏；打开左侧属性栏时立即收起该钉图的左右内容及候选。黄色三角打开的底部抽屉与两者都不互斥，仍支持独立点击和 Tab 总开关。其它钉图不受影响，未提交参数草稿和原提交事务保留，旧属性编辑不跨收起重开生效。手测两个方向快速往返、底部抽屉共存、跨应用层叠、置顶、字段草稿、候选、移动和混合 DPI；隔离 HWND 的中间位置与层序回归不替代普通桌面视觉验收。

## 未发布 Windows-native 分支：Tab 层序（v612）

左右端点展开内容、参数/候选控件和连续行底板紧邻各自钉图，随其进入或退出置顶层；不再取全工作区最低钉图作为共同锚点。普通应用切换、两张不同层位的钉图、参数输入焦点、Tab 展开/收起和移动需一起手测。底板间隙仍穿透，图片/边框遮挡和截图保护保持；隔离 HWND 回归不替代普通桌面观感。

## 未发布 Windows-native 分支：Art 菜单、Tab 与拖出（v611）

`Shift+1` 使用居中的 250×300 DIP 无标题搜索菜单，名称/id 包含匹配忽略首尾空格，清空后回到搜索框，单击同一行添加；拖动、空白列表区域和不可用项不提交。保留原模态会话、IME 与授权，尚未改成主干随钉图移动的非模态窗，原生滚动条及部分图标细节不承诺像素一致。

下方收起标记仅保留三角区域，展开条与参数按钮在最终布局完成后发布完整帧。手测连续切换 Tab、单独展开/收起、隐藏收起端点以及混合 DPI，检查无黑色矩形、上传按钮闪底或位置漂移，原草稿与点击行为不变。

空闲 Pan／静默模式按住 Shift 拖出到文件夹，系统可按 MOVE 保存本次新生成的 PNG 副本，原图片及钉图保留；普通文件拖出接口仍只允许 COPY。导出期间缓存排他锁阻止应用清理，副本只读句柄禁止改写但允许 Shell 移动。松键早于准备完成仍取消；定向真实 Shell 目标落盘测试不等于物理鼠标 OLE 全流程或普通桌面观感通过。

## 未发布 Windows-native 分支：镜像不闪动编辑控件（v609）

水平／垂直镜像后台计算期间保留主编辑栏、属性栏和左侧子栏的原有可用外观，不再先整栏禁用再恢复。命令执行仍拒绝冲突操作，镜像完成后才更新图像、按下状态与撤销历史。真实 HWND 回归检查两个轴的控件启停消息、窗口复用、冲突拦截和撤销／重做；普通桌面的动态观感仍需手测。

## 未发布 Windows-native 分支：左侧内联子栏与分享（v608）

恢复原主干内联连接行、投射/被投射页签、方向、策略、设备/组和来源复制/退出；左侧子栏加入钉图/主编辑栏的同一位置批次，纯拖动保留宽度草稿。二维码导出只有二维码与关闭，链接导出有只读输入与复制；两者复用同一共享 Loom v1 签名邀请。导入提供二维码/链接两页，选择 PNG 或粘贴只本地解析，点击导入才连接已授权的同一 Loom，以当前版本接受并新建关联钉图。关闭窗口不停止已保存关联；失效/已连接/停止邀请不能继续复制。

手测：打开左侧子栏，连续/快速拖动钉图并对照主编辑栏；切换内联页签、输入宽度草稿、滚动和取消菜单。两台已授权同一 Loom 的设备之间分别测试链接和二维码 PNG 导入，确认源钉图编辑同步、入口钉图不被覆盖、重开工作区仍有关联，以及退出/停止后的状态。重复点击当前导入页签保留草稿，换页清输入；在解码、联网、保存等待阶段关闭浮层，检查无迟到自动显示。错误来源、自己的邀请、过期邀请、非 PNG、超过 4 MiB/4096 边长图片应拒绝，且不自动切换 Loom。此版本不包含账号联网 v2 分享；本地 TLS/自有 HWND 回归不代替真实跨机、混合 DPI 或普通桌面观感。

## 未发布 Windows-native 分支：编辑栏稳定性与左侧设置（v607）

主按钮不再在 owner-draw 前先擦成空底色；左侧字段一次更新最终可用性，空 OCR/译文等禁用字段不再使整个子栏持续启停和重绘。左侧新增主干原图标的 Loom、投射目标、接收来源/策略、数据流通/协作入口，复用原生业务菜单和原请求执行器，不在展开时发起网络请求。QR/链接导入导出、主干完整内联方向页签和来源 ID 复制仍未迁移。

手测：连续切换主编辑按钮，打开子栏并停留；在宽度输入草稿时刷新状态，检查不闪、不丢焦点/草稿；分别点击四入口，检查子栏获得焦点后仍能打开，取消后不禁用全栏，接收策略与已有投送/协作操作正常，长内容局部滚动且不缩放钉图。目录或文档换代不得执行旧菜单目标。真实自有 HWND/内存像素回归不代替普通桌面观感、混合 DPI 或真实 Loom/跨机验收。

## Unpublished Windows-native branch: editor presentation

The line group exposes only straight lines and Bézier curves. Arrowheads remain a property rather than a duplicate Arrow entry; standalone Arrow/Polyline tool commands (including A/U) are removed without changing saved annotation decoding. Manually check both remaining tools and arrow-property editing of existing strokes.

Open/close Ctrl+E repeatedly, use its triangle and long-press menus, and switch Rectangle/Ellipse/Line/Brush while the secondary row is open. Check for white or cleared intermediate frames, readable input drafts and selection, unchanged focus, overlapping corner buttons, and screenshot exclusion. Focused native HWND/pixel regressions do not establish ordinary desktop no-flicker acceptance.

## Unpublished Windows-native branch: right-side candidate results (v599)

Public Art multi-result cards now attach below the right output rows only while total Tab is expanded. The compact two-column grid shows prepared thumbnails, titles, indices and the current selection, with a separate immediate local scroll viewport. Candidate clicks use the existing typed result_index transaction and one manual execution; stale result/configuration clicks and uncommitted parameter drafts are rejected. Preview pixels never become formal output. Empty public parameter/output lists still use one document parameter owner.

Manually check a real multi-result Art, select another candidate, inspect pending/disabled states, scroll a long result list without moving the left parameters or zooming the pin, and compare Tab close/reopen, movement, overlapping pins and screenshot protection. Existing editor triangles and Select/closed Ctrl+E pin dragging remain regression targets. Local Art forms, Live auxiliary controls, Art VIEW, full node multiselection and connection/projection UI remain unfinished; isolated HWND and Gallery checks do not replace ordinary-desktop or real Loom acceptance.

## Unpublished Windows-native branch: unified local parameter rows (v598)

Opening one parameter anchor, total Tab expansion and F2 now use the same attached multi-row form. Several local rows can stay open together; a mouse expansion does not steal focus. F2 enters the matching control and F6 returns to its port without closing the form. Escape first restores a changed text draft, then closes the form without deleting the pin. Folding all local rows retains dirty text and its HWND until it is reopened or explicitly closed; these drafts still block the original save/exit idle gate. Valid blur commits and stale-configuration rejection keep the original transaction.

Manually compare local single/multiple rows with Tab, type then fold/reopen, use F2/F6 and two-stage Escape, try typed options and linked values, and check Ctrl+E empty-selection dragging and its closed pin-drag mode. Ordinary desktop/IME/mixed-DPI acceptance remains separate. Local Art forms, the right-side auxiliary area, Art VIEW and connection/projection UI are not completed by this change. Earlier transitional single-field notes below describe their historical iteration, not the current local disclosure path.

## Unpublished Windows-native branch: attached inline Tab parameters (v597)

The native Tab layout now shares a dark backing behind consecutive visible expanded rows, matching the baseline without adding an input surface to row gaps. Folding a row splits its group; scrolling clips the backing with the rows. Output eyes and 72px names use the original image-side padding rather than a mirrored input layout. Manually compare both sides, long lists, local folding, overlapping pins and mixed-DPI text; native pixel/window checks are not ordinary-desktop visual acceptance.

Total Tab expansion now mounts ordinary-image/public Art fields inside their own 28px capsule rows, not a separate captioned form. Text, number/slider, boolean, typed options, color and file actions consume the existing parameter snapshot and atomic transaction. Label/anchor context actions disable/restore a parameter; value context actions reset it or clear color/image. Multiple fields share one document owner. Explicit inputs remain fixed; derived parameters have direct local wheel scrolling, a bounded viewport and a proportional scrollbar. Scroll and local folding keep the existing EDIT controls and drafts. Tab round trips remember transient per-unit offsets; nothing adds a trailing scroll animation or pin zoom.

Manually check several field types and rows at once, high-resolution wheels over values/anchors, the first/last clipped rows, fixed explicit inputs, thumb drag/track clicks, drafts through scrolling/folding, disabled stash/linked fallback, Enter/IME/Escape, settings/Run after drafts, Tab close/reopen, movement/overlap and screenshot protection. Bare F2/local-only field editors, local Art forms and the right-side candidate/Live auxiliary region remain transitional; this is not full Tab or overall migration completion. Owned HWND/DIB checks do not substitute for ordinary desktop, mixed-DPI or real Loom acceptance.

## Unpublished Windows-native branch: per-row Tab disclosure (v596)

Normal-size hidden ports retain their baseline row slots instead of moving later rows upward. Each row inherits the total Tab state until its own anchor explicitly folds or opens it; the next total-state edge resynchronizes that node only. Keyboard focus alone no longer expands rows. Labels do not start disclosure or connection gestures, and a release outside the pressed anchor does not click it. Link drops still use the existing graph transaction and now require the destination anchor, not its label. Eye actions retain their own hit slot.

Manually expand Tab, fold/open individual input/output rows, toggle Tab again, and compare an independently open row on another pin. Hide a middle port and confirm later anchors and lines stay on their original rows. Check F6 then Enter/Space, label clicks, short releases outside an anchor, real dragging/link drops, eyes, movement, minification, and Ctrl+E empty-selection dragging. These changes do not finish inline Tab parameter controls, rail scrolling, Art VIEW or connection/projection UI. The standalone parameter window/single-field editor remains transitional; isolated HWND tests do not replace ordinary desktop/mixed-DPI or real Loom acceptance.

## Unpublished Windows-native branch: attached bottom node drawer (v595)

An explicit document-node click or Tab expansion mounts the baseline's bottom drawer unless the pin is minified. Tab edges resynchronize disclosure; the bottom caret may independently expand/fold it. Original SVGs expose the node name, collapsed endpoint visibility, downstream notification, sticker edit inheritance and Art upstream/parameter automation plus manual execution. Unresolved Art actions are disabled rather than given a synthetic execution owner. Settings and Run use the existing parameter-draft transaction and execution owner. Escape folds only this drawer and retires queued clicks; Ctrl+E/F4 from its buttons still target the owning pin. The attached host moves in the existing frame/port batch and stays below all pins without raising them.

Manually check ordinary/sticker and Art drawers, independent caret versus Tab edges, small-image mode, real parameter drafts before settings/Run, lock/hide/capture/close, movement and overlapping pins, Ctrl+E return to pin dragging, and save/reopen. Focused owned-HWND and memory-DIB checks do not replace ordinary desktop/mixed-DPI or real Loom acceptance. This slice does not complete inline Tab forms, rail scrolling, full workspace multiselection/box selection, standalone Live settings, Art VIEW, or connection/projection UI. Older sections below describe their historical slice, not the current combined completion state.

## Unpublished Windows-native branch: node settings state

The existing parameter window exposes collapsed endpoint visibility, downstream notifications, sticker edit inheritance, and the baseline Art upstream/parameter automation switches. Art upstream automation toggles both trigger and listener together, without executing immediately. Hiding collapsed endpoints removes their paint, mouse and keyboard entry; Tab expansion reveals them again. Notification policy gates both a sticker source and sticker relays but does not disconnect ports or suppress image display. These settings persist without discarding unrelated execution metadata.

Manually check each switch, Tab expand/collapse, F6 eligibility, an image→sticker→Art chain with notifications disabled at each sticker, independent image inheritance, lock/hide/close, and save/reopen. This is the business/state seam with the existing parameter window, not the attached bottom drawer or full inline Tab migration. Focused owned-HWND and save/restore regressions do not replace ordinary desktop or real Loom acceptance.

## Unpublished Windows-native branch: pencil highlight property

The ordinary pencil's GLOW switch is a remembered pencil property, not a tool switch. Opacity and smoothing fields stay visible with their current values. Highlighted pencil drafts retain pencil sampling and smoothing, render in the existing highlighter layer with `0.35 × pencil opacity`, and use the existing annotation/session format and one history entry per stroke. The duplicate standalone highlighter entry and H command are removed; saved highlighter strokes keep their existing rendering.

Manually set pencil opacity to 50% and smoothing to 100%, enable GLOW, and draw a bent stroke: both fields should remain visible, and the stroke should be smoothed with approximately 17.5% opacity. Disable GLOW and check an ordinary 50% pencil stroke; switch away and back, undo/redo, and save/reopen both kinds of stroke. Focused model, owned-HWND button and D2D/PNG regressions do not replace physical input or ordinary desktop acceptance.

## Unpublished Windows-native branch: extension toolbar contributions

Ctrl+E displays up to eight trusted extension toolbar slots using baseline menu visibility, command availability, ordering and grouping. Direct text buttons invoke their declared command; a grouped button or its corner opens the same bounded, internally scrollable command list, with unavailable entries disabled. OCR retains its existing dedicated buttons instead of appearing twice. Static tools and property editors keep their HWNDs. Opening a contribution menu does not whiten or disable every tool label.

Check direct/grouped tools, body versus corner clicks, long lists, disabled entries, catalog/revocation changes during a menu, switching/closing the target, repeated clicks while busy, narrow-screen layout, and empty-selection pin dragging/Ctrl+E return to Pan. The existing command owner and authorization path execute the selected exact commandId; synthetic directory/HWND tests are not real Loom or ordinary desktop acceptance. Art VIEW selection, full Tab layout and connection settings remain separate open migration items.

## Unpublished Windows-native branch: inline port visibility

Expanded Tab rails reveal all declared non-secret ports, including ports hidden while collapsed. Each expanded capsule uses the baseline eye SVG to change its persisted collapsed visibility directly, without opening the separate port dialog. The eye is not a connection drag or disclosure toggle; releasing outside it or dragging cancels the click. A locally disclosed row stays open after a visibility change and is rebound by Unit/port name and host identity, never by a stale catalog index. Collapsed and compact nodes still obey the visibility mask. Graph edges and execution settings are unchanged.

Manually check input/output eyes, initially hidden/default-hidden ports, Tab expand/collapse, individually disclosed rows, link retention, save/reopen, zoom/move/occlusion, and lock/hide/close while a click is pending. The full Tab form, inline parameter values, rail scrolling, bottom settings drawer, Art and connection pages are not complete; owned-HWND/DIB tests do not replace ordinary desktop acceptance.

## Unpublished Windows-native branch: cached full-text copy

Pin settings expose the baseline's original SVG buttons for `原文` / `译文`. Each copies the complete cached OCR/translation text independently of overlay visibility and online renderer availability, without running a model. Current official attachments take precedence over legacy saved results; translations require their original source revision/dependency. Empty or stale results disable only the relevant button. The clicked button reports `已复制` or `复制失败` for about 1.6 seconds. Invalid/NUL/over-budget text is rejected before changing the clipboard, never silently truncated.

Manually check hidden results, multiline Chinese/emoji/long text pasted into an editor, old saved workspaces, stale translations after edits, clipboard contention, repeated clicks, closing Ctrl+E during feedback, and small-screen internal scrolling. Recheck triangle/long-press menus without all-button whitening, selecting/moving annotations, empty-selection pin dragging, and Ctrl+E restoring ordinary pin dragging. Synthetic owned-HWND tests inject only the clipboard publisher and do not replace ordinary desktop or real Loom acceptance. Full Tab/Art/connection settings remain separate migration items.

## Unpublished Windows-native branch: developer-mode display parity

Active `unsigned_developer` scopes, like `trusted` scopes, may project saved renderer/overlay scenes, official OCR renderer fallback, cached visibility and retained notices. Revoked, untrusted and offline scopes do not display old scenes or retain scoped notices. Display metadata does not grant command execution, shortcut registration, resource access or attachment-effect writes; those retain their existing trusted authorization path.

Check saved developer-mode views and notice retention across catalog updates, missing/returning renderers, hide/show, zoom and revocation. A declared developer action may remain visible but must not invoke an untrusted command. Synthetic scope-matrix, real Canvas/zoom, HWND notice and click regressions do not substitute for actual Loom/developer-package acceptance. Full Tab/Art/connection settings and command parallel/cancellation semantics remain open.

## Unpublished Windows-native branch: extension notice stacks

Extension feedback keeps the newest eight notices at the pin's top-right inset. The stack has a local height boundary; long messages also scroll internally without discarding text. Wheel, track/thumb, and keyboard scrolling do not resize or change the pin's opacity. Click/Enter/Space/Esc or the original five-second timeout dismisses a notice and immediately reflows the stack; new/removed notices reset the outer scroll to the newest item. Pin move/resize/hide and capture exclusion apply to the attached stack, without activating the pin or adding animation/worker queues.

Manually check several simultaneous notices on a small pin, long titles/messages, nested scroll boundaries, clicking a partially visible card, timer versus explicit close, moving/hiding/closing the pin during a scrollbar drag, and revoking the originating capability. The Gallery and synthetic owned-HWND tests exercise the real component/consumer but do not replace ordinary desktop, mixed-DPI or real Loom acceptance. Full Tab/Art/connection settings remain open.

## Unpublished Windows-native branch: overlay text input

Official schema 1 OCR/translation result text supports local single-block click-to-copy, double-click word selection, drag selection within its initial text block, and Ctrl+C before image copying. The toolbar may be closed or in ordinary Select mode; painting/transform tools keep their input. Blank overlay space still moves the pin, and text gestures do not move or minify it. Declarative non-local click actions use the existing authorized extension command path.

Manually check multiline text versus ancestor action payloads, selection without automatic whole-block copying, Ctrl+C, blank-area dragging, Ctrl+E and Brush/shape tools, transparent PNGs, repeated zoom/opacity changes, and hide/close/revoke while selecting. Check selection highlights without frame/opacity flashing. Real Loom capabilities and ordinary desktop/DPI acceptance remain open.

The active official OCR data type retains its validated saved Surface scene when its renderer is temporarily missing. A valid descriptor with no usable scene displays `扩展视图不可用`; unavailable attachments show at most eight noninteractive `扩展数据不可用` badges. Offline/revoked catalogs never display saved attachment scenes, and visibility intent/payloads are not rewritten. Unit-relative fallback and badges reflow in the same Canvas and shared full-frame zoom presentation; fixed descriptor scenes do not remeasure on resize. Check missing/returning renderers, offline saved workspaces, hidden results, narrow images, zoom, and blank-area dragging. Developer-mode support remains outside this slice.

## Unpublished Windows-native branch: extension shortcuts and authorized overlay toggles

Trusted Loom capability declarations provide local/global shortcut keys, including `Ctrl+4/5` execution and `Alt+4/5` result visibility when declared by the installed packages. The native message path preserves standard text editing, exact modifiers, modal/capture ownership, and the last selected unit; a closed target does not redirect to another image. Overlay buttons and shortcuts check remote authorization before changing cached visibility, and fall back to real execution only when no usable cache exists.

Manually check declared keys with the toolbar closed, two different selected images, native text fields, denied/disconnected capabilities, repeated presses, and a deleted target. Physical keyboard and real Loom model acceptance remain open; synthetic TLS and owned HWND regressions do not replace them.

## Unpublished Windows-native branch: selection toggle and attached transform handles

Opening the native editor with `Ctrl+E` enables its second, non-dropdown Select button. Clicking it again or pressing `Q` switches to quiet mode and clears the current selection without editing the document. Existing annotations ignore ordinary, Ctrl/Alt, Shift-selection and double-click text picking. Newly created annotations remain selected for immediate move, rotate, scale or text adjustment while the toggle stays off; after deselecting them, re-enable Select to pick them again. Explicit selection with `S` also enables picking. The former Move/Rotate/Scale modes and their W/E/R overrides are removed; those keys retain their original Eraser/Ellipse/Rectangle tools.

The rotation circle and scale square use the current annotation preview during single/group dragging, including mosaic and blur, rather than waiting for mouse release. Check both drag directions, zoomed pins, cancel/undo, fresh text/serial/shape creation in quiet mode, turning Select back on, and closing/reopening the toolbar. Isolated model/HWND tests do not replace ordinary desktop or mixed-DPI visual acceptance.

## Unpublished Windows-native branch: annotation modifier-wheel scaling

In Select mode, `Ctrl+Alt+Wheel` scales selected annotations around the group center. `Ctrl+Alt+Shift+Wheel` scales each annotation around its own center. Both preserve the pinned window's geometry, viewport and opacity, the selection, and the toolbar's persistent transform preferences. Empty selections and other drawing tools do not fall through to whole-window scaling. A blur resize started by this gesture accepts subsequent wheel inputs in a bounded FIFO and commits each input through the existing single worker and undo history; other busy edits do not accept them. Cancellation, a changed target or unavailable editing stops the old batch.

Manually check single/multiple selections, fine wheel increments, both center modes, text/serial layout, continuous blur scaling with direction/center changes, one undo step per input, cancellation or closing the toolbar during computation, and ordinary `Ctrl+Wheel` window scaling / `Alt+Wheel` opacity. Isolated HWND regression coverage is not physical mouse, DPI or foreground visual acceptance.

## Unpublished Windows-native branch: single-field parameter capsules

`F2` enters the real native editor on a focused public Art parameter input; `F6` returns to the port. The workspace has one active field editor outside image pixels. It reuses the parameter transactions and widgets; linked values remain read-only and secrets are omitted. Ordinary sticker image upload is not part of this editor. These are development-branch controls, not completed ordinary desktop or IME acceptance. See [native parity](development/WINDOWS_NATIVE_PARITY.md).

## Unpublished Windows-native branch: port keyboard controls

`F6` enters visible ports on the focused document/Live host; `Shift+F6` starts at its last port. Within the rail, Tab/Shift+Tab and arrows navigate eligible workspace ports, Home/End select the first/last, and Enter/Space toggle name disclosure. Ctrl+Enter selects an output (or the existing source of a linked input), then commits to another node input. Escape first cancels the source, then returns focus; F6 leaves directly. Cancellation preserves the existing edge. These controls apply only to the native development branch.

Manual acceptance remains open for ordinary visible Art/Live, mouse-to-keyboard takeover, IME, DPI/multiple monitors and stale-owner transitions. Isolated HWND tests are not foreground acceptance; see [native parity](development/WINDOWS_NATIVE_PARITY.md).

[Documentation](README.md) · [Getting started](guide/getting-started.md) · [中文教程](guide/getting-started.zh-CN.md)

This is a reference for current controls and manual regression scenarios, not a
record that every scenario passed on the current build. For a first session, use
the getting-started guide above. Jump to [global shortcuts](#1-native-global-shortcuts),
[frontend shortcuts](#2-frontend-shortcuts), [pointer controls](#3-pointer-and-wheel-controls),
or [sticker editing](#5-sticker-editing-regression-matrix).

## System dialog priority

On the ordinary Windows desktop, standard dialogs (`#32770`), modal-frame windows
and windows with a disabled owner take priority over existing stickers. Both the
visual overlay and native input shield go below the dialog; synthetic input excludes
its physical rectangle. Exposed stickers remain interactive. Explicit capture,
sticker interaction or focus can reclaim priority; later foreground changes or an
external click yield again. Background hit-map refreshes do not claim priority.
Maintenance uses the existing 250 ms timer and also respects visible foreign
foreground windows above the sticker host, regardless of dialog class. New button
and wheel events additionally check the actual Windows point target and relative
Z-order, so visible foreign windows cannot be accepted solely by a stale sticker
hit map. Underlying applications reached through click-through are not mistaken
for windows above the sticker. Capture and mouse-move streams retain their fast path.
Relative order is checked in both directions: Windows shell system-band dialogs
can be absent from the ordinary overlay's upward window chain. Their downward
chain must still prove that they are above the overlay; a point hit alone is not
enough to classify a click-through underlying application as an occluder.

Manual acceptance: existing sticker then file/confirmation dialog; dialog then
capture/new sticker; exposed sticker interaction; rapid alternating clicks; negative
monitor coordinates; Windows firewall permission prompts overlapping stickers;
closing the dialog; UAC/lock-desktop entry and return during
capture/drag/edit. Secure-desktop capture/overlays are unsupported: input is yielded,
transient gestures canceled, and ordinary-desktop input restored without changing UAC.
Policy/unit and hidden-HWND Z-order tests do not replace this interactive acceptance.

## Loom restart connection

Check successful connection followed by normal exit/restart, manual disconnect
followed by restart, cancellation during startup reconnect, an offline saved
target, and changed/revoked server identity. Only the saved successful target is
attempted; history alone does not authorize restart connection. Closing a dialog
does not disconnect. Device authentication and server approval remain required.

## Long-capture result preview

During native `Ctrl+3` recording, a click-through thumbnail beside the selected
region shows the current stitched result and its full output dimensions. It
prefers the right side, falls back to the left and stays within the viewport;
when neither side fits, inset placement requires successful capture exclusion.
The preview preserves the whole image's aspect ratio, updates at most twice per
second, and does not write capture files. Check first frame, continued scrolling,
reverse scrolling, pause-to-finish agreement, near-edge selection, cancellation,
restart, and exclusion failure.
Native desktop capture exclusion and input acceptance require manual verification.

The preview has up/down/left/right viewfinder buttons. Each click moves only the
sampling rectangle, by at most 64 logical pixels with overlap, clamped to the
viewport. The rectangle size, session and stitched content are preserved. The
first movement upgrades the existing aggregate to a two-dimensional covered canvas;
later samples are placed by verified overlap and viewfinder displacement (including
logical-to-physical pixel scaling). No wheel events are sent to the background.
Only these buttons intercept pointer input during recording. Controls wait for
the updated preview before accepting another move; stale-position samples are discarded.

L-shaped paths retain transparent, unsampled gaps in the preview. Finish crops to
the largest-area axis-aligned rectangle whose every pixel was sampled, discarding
the other parts of the L. Equal-area ties prefer topmost, then leftmost. Coverage
is explicit alpha, not a test for black/white pixels. Spatial capture is bounded
to 64 megapixels and 512 recorded frames. Unverifiable overlaps fail closed rather
than inventing image content. Test turns, negative-direction moves, revisits that
fill a gap, continued page scrolling after a turn, and final no-hole crop.

## System clipboard submenu

The sticker context menu ends with a compact separator and **剪贴板**. Hover or
focus it to open the scrollable Windows history list. Its custom scrollbar supports
both wheel and thumb dragging through native overlay input. All sticker submenus
align with the hovered row when space permits, shift vertically into the viewport,
and open to the left when the right side cannot fit them. Native hit rectangles
follow the relocated panels, including asynchronously loaded clipboard entries.
Only Bitmap and
Text formats are read; file lists, HTML-only data and other formats are excluded.
Images create independent stickers without replacing the target; text adds centered
annotations with undo and existing edit propagation. Multiline text becomes one
annotation per nonempty line. Check event-driven updates, empty/disabled/denied
states, selection after scrolling, menu dismissal during image decoding, and cleanup.
Windows history remains the source of truth; no separate archive or clipboard writes
are performed. Reads are limited to 25 items, 4 MiB per encoded image, 16 MiB total,
8192 pixels per image dimension, 64 MiB decode allocation, and 64 KiB/256 lines of text.
Oversized or unsupported items are skipped. Native Windows acceptance is separate
from mocked component tests and compilation.

## Text and numbering properties

With the text tool active, a single click on the topmost text annotation reopens its existing content instead of creating another annotation. The edit preserves its ID, position, font, size and style. Enter/blur commits, Escape cancels, unchanged text creates no history entry, and content changes support undo/redo. Empty space still starts a new text annotation; serial annotations keep their existing interaction.

Creation and selected-annotation properties use the compact drawing toolbar shell, aligned to the main toolbar with viewport avoidance. Text orders color, size, font; numbering orders fill, foreground, radius, font. Check alpha checkerboards and a diagonal mark at zero alpha, 8–999 integer inputs, increment/decrement and arrow keys, Enter/blur commit, Escape cancellation, font scrolling and viewport clipping. Selected edits preserve creation defaults and use the existing undo history. Numbering foreground still controls both the border and digit; this UI does not introduce separate colors or new typography options.

## Pencil and paintbrush

The pencil dropdown enables an independent natural paintbrush. Check compact color/size/opacity/smoothing fields, independent numeric profiles, equal-width pencil versus speed-sensitive, lightly softened brush rendering, a single-click dab, cancelled strokes, one history entry per stroke, undo/redo, and rasterization consistency. Legacy pencil and highlighter annotations retain their rendering. Native pressure and cross-Loom object editing are not claimed; rasterize natural brush strokes before using the existing remote edit protocol. See [freehand editing](features/FREEHAND_EDITING.md).

## Bézier path editing

Use the line-tool dropdown to create a Bézier path. Double-click a curve to insert an anchor without changing its shape; drag either endpoint's `+` handle to extend. Right-click a white anchor and choose “删除节点” to merge adjacent segments or trim the first/last segment. Deletion may change the local curve shape; at least one segment remains. Yellow tangent handles cannot be deleted separately. Check insertion, both-end extension/deletion, disabled deletion on the final segment, menu dismissal, undo/redo, and rasterization. See [curve editing](features/BEZIER_EDITING.md).

Local Loom Art/extension integration is native-only and enabled by default;
`OTTER_ENABLE_LOOM_OTTER=0` disables it. The legacy `HOOK_ENABLE_LOOM_HOOK=0`
applies only when the canonical variable is absent; see [environment precedence](development/RENAMING.md#环境变量).
In browser preview, verify that local UI/session work
still functions, no Loom request/push WebSocket is opened, and Art execution
finishes with a local unsupported error rather than uploading images or inventing
a result. A native paired-version runtime is required for real integration.

Projection target buttons show device/group/user icons and confirmed successful target counts,
and open the corresponding tab in the shared target dialog.
Receive placeholders use a gray background. The checkable tray “自启动” item controls current-user Windows login startup; re-enable it after moving the portable executable.
The tray's “接收投射” creates a blank sticker and opens the shared QR/link import panel inside it; acceptance fills the same sticker. Both receiver import buttons open this same tabbed panel with the requested method selected. Tray shortcut hints use the configured bindings and a separate aligned native menu column.
Available targets show one ID and checkbox per row; checking starts projection,
unchecking cancels it. Per-row markers distinguish pending, success and failure;
success requires receiver display or a confirmed remote unlink. Retry failed
operations inline. Group checkboxes aggregate member state and indicate partial
selection; stopping a group updates the same device rows, including overlaps.
Check long device/group and font lists with both the mouse wheel and scrollbar
drag, including the last row, Escape dismissal and restored trigger focus.
The user dropdown reports that user projection is not yet available.

The receive toolbar has a read-only flow indicator and one import button. Verify
both QR PNG and projection-link input in its compact sticker-anchored dialog, explicit Import confirmation,
error/retry feedback, Escape and focus restoration. The sender identifier remains;
the revision-number management button is removed.

This document records currently implemented user-facing controls. It is a manual
verification guide, not an implementation specification. When behavior changes,
verify the code first and update this file with the same change.

Self-hosted HTTPS Loom with a private CA supports [origin-scoped process trust](security/SECURITY_BOUNDARIES.md#origin-scoped-loom-https-trust).
Check async/blocking HTTP and LiveRelay/wall WSS together: the selected origin
accepts its valid certificate, while a wrong hostname, expired/untrusted
certificate, malformed configuration or redirect fails closed. Other origins
retain WebPKI trust. Restart Otter after a CA change; device pairing remains
mandatory. These TLS checks do not replace native two-machine projection proof.

## 1. Native global shortcuts

These shortcuts are registered by the Tauri host and can be triggered while
another application has focus.

| Shortcut | Current behavior | Manual check |
| --- | --- | --- |
| `Ctrl+1` | Native: select over the changing desktop without a frozen preview; drag and release to capture the current selected region. A click can choose the highlighted window bounds. | Keep an animation running while dragging, change its content before release, and verify the captured content is current and has no selection outline. Also test reverse drags, negative-coordinate monitors, Escape/right-click cancellation and existing visible pins. |
| `Ctrl+2` | Enter live region/window capture mode. Double-click chooses a revalidated full window. A drag fully contained by one valid program window binds a fixed window-local pixel region; other drags use a screen region. | Select a 50×50 area at offset (100,100), move the source window, and confirm the view still shows that same UI area. |
| `Ctrl+3` | Enter long-capture mode, with four-direction viewfinder movement during recording. | Capture a scrollable page, move the viewfinder horizontally and vertically, verify the final no-hole crop, then cancel and retry to confirm session cleanup. |
| `Ctrl+E` | Toggle sticker editing, including projection invitations, device/group multiselect and opt-in two-way v1 annotation editing. Enabled Capability Plugins may contribute namespaced menus and actions. | Select overlapping targets, retry partial sends, switch source-only direction, merge separate objects and explicitly resolve same-object conflicts. Check durable restart, Escape focus and native popup clicks. Shared Loom needs no official login; account mode keeps its login requirement and does not support two-way editing. |
| `Ctrl+Alt+Space` | Reserved Talk integration concept, not an implemented feature of the original Otter. Excluded from the current Windows-native migration. | No start/stop acceptance is required for this unimplemented concept; future implementation needs a separate scope. |
| Three `Esc+Delete` chords, with less than 400 ms between chords | Emergency exit. Both keys must overlap, then both must be released before the next chord counts. The main process and independent watchdog share this rule. | Verify three full press/release cycles exit; repeated Esc alone, Delete alone, autorepeat, and releasing only one key must not exit. |
| `Ctrl+Alt+Shift+F12` | Backup emergency-exit chord handled by the watchdog. | Verify it terminates Otter and restores cursor/input state. |

The Windows-native v524 candidate now implements both emergency channels; its isolated tests do not replace physical-keyboard acceptance. Emergency exit bypasses saving and destructors. Its watchdog terminates only the authenticated original parent, leaving the existing Live parent-death recovery child responsible for registered sources. The native host does not alter global cursor constraints, so it does not unconditionally reset another application's cursor state.

The tray currently exposes **Capture**, **Live capture**, **Long capture**,
**Open existing image**, **Receive projection**, checkable **Autostart**, and **Quit**.
Live capture enters the same window-aware
selection surface: double-click selects a revalidated full window, while a drag
inside one valid window creates a window-relative region. A drag not fully
contained by one valid window creates a display-region source. The settings dialog remains implemented, but
its tray entry is temporarily hidden.

Device delivery confirmation uses a bottom-right desktop prompt with accept,
reject and defer actions. Deferred invitations remain available in the receiver
panel; polling does not repeatedly reopen the same deferred prompt. Native hit
rectangles and direct synthetic input handling keep the prompt interactive.
The native receive tab offers Pause, Receive, Reject, and Auto. Receive (default)
and Auto accept new invitations without another approval prompt; Pause suspends
participation and Reject declines new invitations. All sticker panels share the
setting; existing received stickers remain linked. Server authorization and deny
rules still apply. The sender shows waiting and received counts separately.
Ctrl+C copies the owning pin when focus is on an attached button; text fields
retain text copying. A newer explicit copy replaces stale pending work without
overwriting later edits or newer clipboard contents.

## 2. Frontend shortcuts

### 实时投射（LiveRelay，不是二维码投射或屏幕墙）

- 源端：`Ctrl+2` 创建实时截图，在其 Unit 参数面板中“发布到 Loom”。
- 观看端：使用已配对的默认 Loom，打开支持 Surface 的 Art Unit 参数面板；等待 Surface
  挂载后，刷新列表、选择会话、“加入观看”。缺少绑定、源未连接或正在加入时禁用加入按钮。
- 服务端决定观看权限。加入不会请求 controller；观看窗口中的“请求控制”仍是独立操作。
- 被删除的设备可显式“重新配对”；只重新提交原 public key，不自动批准、加入或请求控制。
  新身份通过正常 Surface 重挂载获取绑定。批准后刷新、选择并加入，旧 closed viewer 不阻止
  新 viewer，但旧 relay 的身份、终态与空帧缓存不能复活。活动或 recovering viewer 仍去重。
- 检查空列表、长标题/设备 ID、键盘选择、错误/重试、连续点击、面板关闭及 Surface 换代。
  已打开窗口不因关闭参数面板而结束；未完成的加入被取消后不应留下新 viewer。
- 观看窗口的[只读诊断槽](features/LIVE_RELAY_DIAGNOSTICS.md)输出 source/epoch/frame、接收计数、
  codec/字节和解码提交耗时；epoch/停止使旧提交失效，无图像、标题或错误正文。
  原生基线必须另绑实际 EXE/进程 SHA；不把这些字段当物理 FPS 或跨机延迟。
- 原生两机、权限撤销和性能矩阵仍须按实际包验证，不把组件测试当作网络/物理显示验收。

These controls are context-sensitive. Most are ignored while the event target is
an input, textarea, or content-editable element; `Escape` remains active for
cancel/delete semantics.

| Shortcut | Context | Current behavior |
| --- | --- | --- |
| `Ctrl+C` / `Ctrl+V` | selected unit / canvas | Copy the selected unit or paste an internal unit, system image, or file payload. |
| `Ctrl+O` | canvas | Open an existing image for editing. |
| `Ctrl+H` | canvas | Toggle screenshot/color history. |
| `Ctrl+S` | selected unit | Save the selected image. |
| `Ctrl+Z` / `Ctrl+Y` | selected unit | Undo or redo sticker edits. |
| `Delete` / `Backspace` / `Escape` | selected unit | Delete the selected unit. |
| `Escape` | capture selection | Cancel the current screenshot selection. |
| `Escape` | sticker editing | Cancel the uncommitted sticker edit draft. |
| `Shift+1` (`Shift+!`) | selected unit | Toggle the compact Art actions menu. |
| `Tab` | selected unit | Toggle the parameter panel. |
| `Ctrl+E` | canvas/overlay | Toggle the selected unit's sticker editing toolbar. Generic plugin menus are rendered only from active contributions. |
| `Ctrl+4` | selected unit | Not registered by Otter core. The official OCR Capability Plugin contributes re-recognition and complete-result copying while enabled. |
| `Alt+4` | selected unit | The enabled official OCR plugin contributes this shortcut. Toggle cached OCR overlays; recognize and show them if no result exists. |
| `Ctrl+5` | selected image unit | The enabled translation plugin translates from cached OCR, recognizing first when needed. Repeated use refreshes translation while preserving OCR content. |
| `Alt+5` | selected image unit | Toggle cached translation without a model request; produce OCR and translation on first use. |
| `Ctrl+Shift+4` | canvas | Cycle sticker chrome: all controls → border only → hidden → all controls. |
| `Q` / `W` / `E` / `R` | selected unit or sticker editing | Select the annotation transform mode: select, move, rotate, or scale. |

Ordinary `Escape`/`Delete` context actions and triple-`Esc+Delete` emergency exit
are separate mechanisms. Either key may complete a chord; simultaneous means
overlapping down states, not identical event timestamps. Every cycle must release
both keys, and adjacent chord presses must be less than 400 ms apart. Holding one
key and repeatedly pressing the other does not advance the counter. Three Esc
presses alone no longer close Otter. Do not lengthen the emergency window to
compensate for missed input; fix the input path instead.

OCR text overlays preserve native substring selection. A plain block click copies
that block; `Shift+click` toggles bounded multi-block selection without moving the
sticker. The contributed OCR toolbar can copy cached full text, reconstruct relative
layout, or copy selected blocks in reading order. Re-running OCR clears the selection.

OCR and translation use separate attachments and mutually exclusive overlays.
Showing translation suppresses OCR without changing the OCR visibility preference.
Hiding translation restores OCR unless the user explicitly hid it. `Alt+4` while
translation is visible selects OCR and hides translation; a second `Alt+4` hides
OCR too. Visibility changes preserve attachment revisions, digests, text, and
geometry. Re-recognizing the image invalidates the older translation.

With the updated Loom OCR capability, each OCR block uses the dominant nearby
background color from its source region after estimated text-color exclusion.
Similar sampled colors are grouped; translation inherits the region colors rather
than a shared contrasting fill. Text contrast remains independently protected.
Legacy migration preserves available per-block background evidence. Cached results
are not silently rewritten: rerun `Ctrl+4`, then `Ctrl+5`, to refresh old colors.

The focused `ocrTranslationVisibility.test.tsx` regression drives command and
shortcut dispatch into the real attachment store and rendered overlay, including
first use, repeated toggles, persistence, and an in-flight translation.

QR/barcode recognition is a child capability of the installed official OCR
Capability Plugin. `Ctrl+4` performs text and code recognition together; the
unit toolbar intentionally exposes cached OCR actions instead of a redundant
manual code-recognition item. The package writes a generic
`neuro.official/ocr.codes.v1` attachment. `Alt+4`
toggles the text and code overlays together. Each green frame has a center
action circle that opens a package-declared panel for copying, closing, or
opening credential-free HTTPS results. Otter core does not register a decoder,
QR-specific component, or QR-specific graph port; it only provides the generic
permission-checked external URL broker.

## 3. Pointer and wheel controls

| Input | Current behavior | Manual check |
| --- | --- | --- |
| Left drag | Move a sticker/unit while preserving the initial cursor offset. | Move slowly and rapidly across monitors; the sticker must not detach or chase alternating pointer streams. |
| `Alt` + drag | Align/snap against nearby units. | Confirm Alt still reaches other applications when Otter is not handling the interaction. |
| `Ctrl` + drag | Cascade/stack placement. | Confirm final order and positions are committed once at release. |
| `Shift` + drag | Native file drag-out to Explorer or the desktop. | Confirm the exported PNG uses the shared visible filename rules. |
| `Ctrl` + wheel | Resize the hovered sticker around its intended anchor. | Check image, annotations, links, and crop geometry together. |
| `Alt` + wheel | Adjust sticker opacity. | Verify it works before and after entering edit mode. |
| Double-click sticker | Toggle minified/full view. | Test stickers with many annotations; live image and annotation layers must remain mounted and visible, without switching to a baked bitmap. |

## 4. Capture and image regression matrix

All applications, including browsers, use native Live capture. Moving a source
window retains its window-local crop. Scrolling its content or switching tabs
updates the crop to the newly displayed pixels. Browser document binding and
extension installation are no longer part of this feature.

Route B native GPU presentation is automatic for eligible Windows Live images,
with `OTTER_LIVE_GPU_PREVIEW=0` forcing JPEG compatibility. See [GPU presentation](features/LIVE_GPU_PRESENTATION.md). Its additional
gate must prove native pixels plus the existing Unit/editor/input behavior;
ordinary browser screenshots and submission counters alone do not prove native
compositor visibility or monitor FPS. Copy/save/native Shift-drag can request a
bounded lossless GPU snapshot, with stale-result rejection and exported-file cache
invalidation. Healthy native presentation skips continuous CPU readback/JPEG when
no relay publisher needs encoded frames; publishing keeps budgeted JPEG production
without disabling the native plane or multiplying CPU permits.
editing, Art, stop and unsupported composition retain ordinary Unit behavior.
The gate also covers static-source fallback, capture health without encoded frames,
and an animated-source CPU sample; submission counts are not monitor FPS.

- Region capture follows the physical cursor without losing button edges during
  fast movement.
- Window targeting excludes Otter's hidden/overlay host, desktop surfaces, taskbar,
  hidden windows, and unsuitable tool windows.
- Final window capture revalidates the target instead of trusting a stale hover
  rectangle.
- HDR capture is attempted only for an eligible Windows 11 HDR display; unsupported
  or SDR-only paths fall back to SDR without failing the capture.
- Long capture remains SDR by design.
- Capture payloads may be file-backed; callers must not assume every image arrives
  as a large Base64 string.
- Window Live crops share a WGC owner by HWND/process/size. Each Unit retains its
  delivery worker, three-frame queue and single-slot callback buffer. Slow local consumers discard obsolete
  video frames instead of blocking capture; lifecycle and errors are not placed
  in that lossy queue.
- Local video requests up to 60 FPS. A bounded frame-ready signal wakes encoding
  immediately instead of adding a second sampling interval after each JPEG.
  Full-source pixels and active consumer count reduce capture cadence under a
  shared budget, including for tiny crops of large windows. Interactive WGC pools
  use two full-source buffers. GPU contention retains/retries the newest owned
  texture, not synchronous CPU readback. Automatic fallback is globally serialized
  before staging/Map through JPEG and uses a pixel/rate budget plus measured cooldown.
  Hidden/offscreen consumers without relay publication keep 1 FPS capture health but no automatic JPEG work;
  showing them resumes budgeted work. Explicit user snapshots remain independently bounded.
  Windows WIC performs full-resolution quality-82 JPEG encoding with 4:4:4 chroma;
  a logged software fallback preserves capture if the system codec fails.
  The frontend decodes the next JPEG before replacing the displayed URL, reuses
  the IPC binary view, and subtracts poll/read/decode work from the frame budget.
  Actual throughput remains source-, hardware-, size-, and concurrency-dependent.
- Repeat `Ctrl+2` after completing a selection to create resource-admitted concurrent
  local Live stickers. Same-window regions and different windows coexist; each
  session owns its crop, frame queue, input sequence, Unit, and stop channel.
- Admission uses source/crop memory estimates, available RAM, DXGI budget and
  sampled CPU/allocation growth, bounded by 16 sources rather than a fixed four.
  Pressure reduces cadence, refuses new work, and preserves existing Units.
  See [resource admission](features/LIVE_RESOURCE_ADMISSION.md) for limits and diagnostics.
  Creating a new selection does not replace existing Live Units. The desktop
  does not mount the global LIVE relay-panel badge; relay transport remains intact.
- The local live view starts at the selected screen rectangle and renders only
  the current frame with a theme-green border and four theme-yellow corners; it
  has no header, status panel, or visible buttons. Drag any yellow corner (or
  use `Alt`+drag) to move it, use `Alt+Shift`+drag to resize it without changing
  aspect ratio, `Ctrl`+wheel to resize around the pointer, `Alt`+wheel to change
  opacity, and `Alt+Delete` to close it. Native mouse-up, blur, pointer cancel,
  visibility loss, and teardown all terminate a local move. A non-streaming
  state does not present the last frame as healthy.
- A live drag wholly inside a declared-supported same-integrity Win32/WinForms
  window stores the selected rectangle in fixed window-local physical pixels.
  Moving the program therefore changes the absolute screen origin, not the
  captured UI region. Direct mouse, wheel, drag, and keyboard input is enabled
  automatically and uses the same cropped region transform, including across
  different Windows DPI-awareness contexts. The visible Live rectangle remains
  in logical CSS pixels while WGC frame descriptors retain physical pixel sizes.
  Every displayed pixel maps to the captured source; the yellow corner hit area
  is clipped to its L-shaped strokes so nearby source controls remain reachable.
- Live input requires the same user and session on the active default desktop;
  the source's integrity must not exceed Otter's. An elevated Otter can therefore
  control an ordinary same-user source without weakening upward-input refusal.
  Refused input remains owned by the Live content and shows a deduplicated Unit
  notice instead of falling through to ordinary sticker dragging.
- Shared Otter shortcuts take precedence over source input, including Ctrl+E,
  Tab, and Shift+1. Window blur, pointer cancellation, ending
  control, reclaim, close, and native session teardown all release tracked input.
- Same-window sessions share one visibility snapshot, keyed by HWND/process/thread
  identity. Closing one releases only its input and visibility lease; the last
  session restores the recorded source placement. Explicit source unhide is a
  window-wide action, visible to every region, not a session close. Temporary
  transparency changes during hit testing share the same lock. The independent
  watchdog uses a content-free, atomically replaced recovery journal on abnormal exit.
- Closing the source must show a real failure rather than silently retaining the
  last frame as a healthy live view. Closing the live view must stop and join its
  owner thread and clear buffered media.
- Static region/window and long-capture paths remain separate from live sessions.

## 5. Sticker editing regression matrix

- Crop updates the sticker's effective content dimensions and propagates the new
  geometry to matched-size/centered views.
- Erase, brush, highlighter, mosaic, blur, line, arrow, text, number, and shape
  previews update interactively and commit history once when the gesture ends.
- Annotation selection uses geometry-aware hit testing after a bounds prefilter;
  blank space inside a freehand annotation's bounding box must not select it.
- Annotation movement previews with an imperative transform and commits actual
  coordinates once on release.
- Minify/restore keeps live content mounted and visible, clips the full-size
  content frame, and ignores baked-preview arrivals rather than replacing live layers.
- Finish a content-eraser stroke with existing blur annotations: retain the decoded
  blur until its replacement is ready, without a blank or gray flash. Increasing
  blur strength or changing geometry must not retain a weaker or misaligned frame.
- Save, clipboard image export, and drag-out use the same Unicode-safe visible
  naming templates. Internal caches and content-addressed assets do not.

## 6. Workflow and persistence regression matrix

- Art capabilities come from the current `loom.hook.v1` contract; Otter must not add
  per-Art executors or legacy compatibility branches without an active protocol
  requirement.
- Shader-backed Art parameters should update the preview immediately through the
  general preview pipeline.
- Session load preserves readable data and reports unknown fields without rejecting
  a session solely because of schema drift.
- Restart restores units, links, groups, recycle bin, reference library, history,
  tool settings, and application settings through their current persistence paths.
- Removing or replacing a workspace/unit invalidates its related bounded caches.

## 7. Visibility and safety regression matrix

- A sticker intercepts pointer input only when its overlay is actually visible and
  not occluded by an exclusive/fullscreen foreground surface.
- Ordinary Alt key use in another application is not cancelled by Otter.
- Normal quit, emergency quit, and watchdog termination restore system cursor and
  transient input state.
- Starting Otter after a force-killed instance restores the configured system cursor
  before capture mode can begin.
- Native minimize, higher-integrity targets, other Windows sessions, and secure
  desktop input remain fail-closed rather than reporting successful control.

## 8. Tile terminal regression matrix

The opt-in `--tile` / `--tile-output` entry is described in
[TILE_TERMINAL.md](features/TILE_TERMINAL.md). It has its own output mutex and device
identity path and does not start the ordinary capture workspace.

- Images and existing Live sessions follow one versioned crop/rotation mapping;
  creating a placement does not create a second source program or capture.
- Declarative Art reuses an existing instance and one ephemeral attachment per
  output. Repeated placements share that attachment; ordinary source state and
  formal results survive view cleanup and later edits.
- Art input uses applied-layout authorization, host confirmation and declared
  cancellation. Stable controls retain text drafts while accepted edits finish;
  slow execution does not erase a newer queued absolute value edit.
- An input grant belongs to one endpoint, presenter lease, layout revision and
  placement. A contending endpoint receives a refusal without stealing control.
- Authority feedback leaves the image visible and the canvas hit-testable.
- Click, drag, wheel and keyboard produce observable effects in the source;
  dragging a push button must not activate it as a click.
- Blur, invalid layout, source loss and output crash release held source input;
  delayed network results do not restore invalid authority or old frames.
- Output loss is detected while network work is pending; stale authorization
  clears static and dynamic presentations independently of control I/O.
- Closing the manager leaves the output running. Restarting that output keeps
  the same source session and application state; closing it releases only its
  own presentation resources.
- Freeze retains only an already complete frame for the current mapping and
  presenter lease. Art polling and media decoding stop; accepted source work
  continues. Black removes all content. Both modes immediately reject new wall
  input at the daemon, release Live control and report each output's application.
- Resume advances the layout revision and loads current source state. Output
  restart, authorization loss, geometry changes or black-to-freeze cannot replay
  old frozen pixels: the output is cleared and reports `frame_unavailable`.
- Physical identification shows the enumerated output name, device/output IDs,
  pixels and a countdown, even before assigning a wall. Its lease-owned request
  lasts at most 10 seconds; repeated commands do not extend it or revise geometry.
  Management distinguishes acceptance from the output's application report and
  preserves dirty layout drafts. Covered input is paused; Escape/close dismisses
  the marker without exiting the output. Freeze/black must resume first.

`scripts/tests/Invoke-TileWallNativeInputProbe.ps1` uses an isolated packaged
daemon and terminal with a native Rust WGC/input source harness and a WinForms
client-region fixture. Its output records executable paths, digests, source
effects, teardown and measured observation latency. This is one-physical-output
evidence. `scripts/tests/Invoke-TileWallArtProbe.ps1` adds installed PowerShell
Art execution, rapid cross-field input, bounded acknowledgement stress,
confirmation/cancellation, resources and daemon/output recovery. Physical
multi-machine seams/synchronization and longer soaks remain separate acceptance
requirements.
