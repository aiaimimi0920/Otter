//! 只在已绑定来源的 UIA 子树查找指针下最深滚动容器；不按桌面命中，不读取文本。
use super::*;
use windows::Win32::UI::HiDpi::*;
use windows::{
    core::*,
    Win32::{Foundation::*, System::Com::*, UI::Accessibility::*},
};
struct PhysicalDpi(DPI_AWARENESS_CONTEXT);
impl Drop for PhysicalDpi {
    fn drop(&mut self) {
        unsafe {
            let _ = SetThreadDpiAwarenessContext(self.0);
        }
    }
}
struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
pub(super) struct Provider {
    client: IUIAutomation,
    cache: IUIAutomationCacheRequest,
    target: Target,
    _apartment: Apartment,
    _dpi: PhysicalDpi,
}
pub(super) struct Prepared {
    element: IUIAutomationElement,
    pattern: IUIAutomationScrollPattern,
    point: [i32; 2],
    ticks: i32,
    horizontal: bool,
}
impl Provider {
    pub(super) fn new(target: Target) -> Result<Self> {
        unsafe {
            // UIA Win32 provider 仍可能按客户端线程 DPI 虚拟化矩形；协议点始终为物理像素。
            let previous = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            if previous.0.is_null() {
                return Err(Error::from_win32());
            }
            let dpi = PhysicalDpi(previous);
            CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
            let apartment = Apartment;
            let client: IUIAutomation2 =
                CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
            // Live 控制后台 A，不能让 UIA pattern 的默认聚焦抢走 B 的输入焦点。
            client.SetAutoSetFocus(false)?;
            client.SetConnectionTimeout(200)?;
            client.SetTransactionTimeout(200)?;
            let client: IUIAutomation = client.cast()?;
            // 单节点缓存将跨进程的多次属性 RPC 合并到导航请求；不跨事件保留旧页面节点。
            let cache = client.CreateCacheRequest()?;
            for property in [
                UIA_ProcessIdPropertyId,
                UIA_IsPasswordPropertyId,
                UIA_BoundingRectanglePropertyId,
                UIA_ControlTypePropertyId,
                UIA_ScrollHorizontallyScrollablePropertyId,
                UIA_ScrollVerticallyScrollablePropertyId,
            ] {
                cache.AddProperty(property)?;
            }
            cache.AddPattern(UIA_ScrollPatternId)?;
            // 首次连接启动浏览器的可访问性树；后续滚动重新绑定，不能缓存旧页面节点。
            let root = client.ElementFromHandleBuildCache(target.hwnd(), &cache)?;
            warm_document(&client, &cache, &root, target.process_id())?;
            Ok(Self {
                client,
                cache,
                target,
                _apartment: apartment,
                _dpi: dpi,
            })
        }
    }
    pub(super) fn prepare(
        &self,
        point: [i32; 2],
        ticks: i32,
        horizontal: bool,
    ) -> Result<Option<Prepared>> {
        self.target.validate()?;
        crate::live_window_target::input_security::preflight(&self.target)
            .map_err(|_| Error::from(E_ACCESSDENIED))?;
        let deadline = Instant::now() + Duration::from_millis(450);
        // A 可被用户或页面脚本独立滚动。先刷新，再查询树和读取当前位置，
        // 否则 UIA 相对 Scroll 会按旧 AX 位置生成绝对目标，把页面拉回旧位置。
        super::refresh::position(&self.target, point)?;
        unsafe {
            let root = self
                .client
                .ElementFromHandleBuildCache(self.target.hwnd(), &self.cache)?;
            if root.CachedProcessId()? != self.target.process_id() as i32 {
                return Err(Error::from(E_ACCESSDENIED));
            }
            let walker = visible_walker(&self.client)?;
            let mut stack = vec![(root, 0usize)];
            let mut selected = None;
            let mut deepest = 0;
            let mut visited = 0;
            while let Some((element, depth)) = stack.pop() {
                if Instant::now() >= deadline {
                    return Err(Error::from(HRESULT(0x800705B4u32 as i32)));
                }
                visited += 1;
                if visited > 256 || depth > 32 {
                    return Err(Error::from(E_BOUNDS));
                }
                if element.CachedProcessId()? != self.target.process_id() as i32
                    || element.CachedIsPassword()?.as_bool()
                {
                    continue;
                }
                let bounds = element.CachedBoundingRectangle()?;
                // Chromium 的虚拟分组节点可以没有矩形，其后代仍有可见滚动容器。
                // 只裁掉确定不包含指针的非空分支，不能把无矩形的桥接节点当成叶子。
                if bounds.right > bounds.left
                    && bounds.bottom > bounds.top
                    && !contains(bounds, point)
                {
                    continue;
                }
                if let Some(pattern) = optional(
                    element.GetCachedPatternAs::<IUIAutomationScrollPattern>(UIA_ScrollPatternId),
                )? {
                    let available = if horizontal {
                        pattern.CachedHorizontallyScrollable()?
                    } else {
                        pattern.CachedVerticallyScrollable()?
                    };
                    if contains(bounds, point) && available.as_bool() && depth >= deepest {
                        deepest = depth;
                        selected = Some((element.clone(), pattern));
                    }
                }
                let mut child =
                    optional(walker.GetFirstChildElementBuildCache(&element, &self.cache))?;
                while let Some(next) = child {
                    if Instant::now() >= deadline {
                        return Err(Error::from(HRESULT(UIA_E_TIMEOUT as i32)));
                    }
                    if visited + stack.len() >= 256 {
                        return Err(Error::from(E_BOUNDS));
                    }
                    child = optional(walker.GetNextSiblingElementBuildCache(&next, &self.cache))?;
                    stack.push((next, depth + 1));
                }
            }
            Ok(selected.map(|(element, pattern)| Prepared {
                element,
                pattern,
                point,
                ticks,
                horizontal,
            }))
        }
    }
    pub(super) fn scroll(
        &self,
        prepared: Prepared,
        deadline: Instant,
        attempted: &mut bool,
    ) -> Result<bool> {
        crate::live_window_target::input_security::preflight(&self.target)
            .map_err(|_| Error::from(E_ACCESSDENIED))?;
        let Prepared {
            element,
            pattern,
            point,
            ticks,
            horizontal,
        } = prepared;
        unsafe {
            let increment = if horizontal { ticks > 0 } else { ticks < 0 };
            let amount = if increment {
                ScrollAmount_SmallIncrement
            } else {
                ScrollAmount_SmallDecrement
            };
            for _ in 0..ticks.unsigned_abs() {
                if Instant::now() >= deadline {
                    return Err(Error::from(HRESULT(0x800705B4u32 as i32)));
                }
                self.target.validate()?;
                if element.CurrentProcessId()? != self.target.process_id() as i32
                    || !contains(element.CurrentBoundingRectangle()?, point)
                {
                    return Err(Error::from(E_ABORT));
                }
                let percent = || {
                    if horizontal {
                        pattern.CurrentHorizontalScrollPercent()
                    } else {
                        pattern.CurrentVerticalScrollPercent()
                    }
                };
                let before = percent()?;
                // 属性 RPC 也可能耗尽预算；最后一次检查必须紧邻动作，不能越期启动。
                if Instant::now() >= deadline {
                    return Err(Error::from(HRESULT(0x800705B4u32 as i32)));
                }
                // 每格一次 provider 小步；部分完成/超时是未知结果，不能回退再发原滚轮。
                *attempted = true;
                pattern.Scroll(
                    if horizontal {
                        amount
                    } else {
                        ScrollAmount_NoAmount
                    },
                    if horizontal {
                        ScrollAmount_NoAmount
                    } else {
                        amount
                    },
                )?;
                // Scroll 成功不代表 AX 缓存已更新；只读命中请求促使源及时反馈，不能重发动作。
                if Instant::now() >= deadline {
                    return Err(Error::from(HRESULT(0x800705B4u32 as i32)));
                }
                super::refresh::position(&self.target, point)?;
                super::settle::observe(before, increment, deadline, percent)?;
            }
            Ok(true)
        }
    }
}
// Chromium 首次 UIA 连接会异步装载网页树。只在启动预算内等 Document 出现，
// 不执行或重试滚动动作；到期后仍可处理没有网页的浏览器窗口。
fn warm_document(
    client: &IUIAutomation,
    cache: &IUIAutomationCacheRequest,
    root: &IUIAutomationElement,
    pid: u32,
) -> Result<()> {
    let deadline = Instant::now() + Duration::from_millis(800);
    unsafe {
        let walker = visible_walker(client)?;
        loop {
            let mut stack = vec![root.clone()];
            let mut visited = 0;
            while let Some(element) = stack.pop() {
                if Instant::now() >= deadline {
                    return Ok(());
                }
                visited += 1;
                if visited > 256 {
                    return Err(Error::from(E_BOUNDS));
                }
                if element.CachedProcessId()? != pid as i32 || element.CachedIsPassword()?.as_bool()
                {
                    continue;
                }
                if element.CachedControlType()? == UIA_DocumentControlTypeId {
                    return Ok(());
                }
                let mut child = optional(walker.GetFirstChildElementBuildCache(&element, cache))?;
                while let Some(next) = child {
                    if Instant::now() >= deadline {
                        return Ok(());
                    }
                    if visited + stack.len() >= 256 {
                        return Err(Error::from(E_BOUNDS));
                    }
                    child = optional(walker.GetNextSiblingElementBuildCache(&next, cache))?;
                    stack.push(next);
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
fn contains(r: RECT, p: [i32; 2]) -> bool {
    p[0] >= r.left && p[0] < r.right && p[1] >= r.top && p[1] < r.bottom
}
// 普通 overflow 容器可能不属于 ControlView，却仍提供 ScrollPattern。
// 保留它们才能命中最深容器；仍排除屏外节点，不扩大为整棵 RawView。
unsafe fn visible_walker(client: &IUIAutomation) -> Result<IUIAutomationTreeWalker> {
    let visible = client.CreatePropertyCondition(UIA_IsOffscreenPropertyId, &false.into())?;
    let scrollable =
        client.CreatePropertyCondition(UIA_IsScrollPatternAvailablePropertyId, &true.into())?;
    let candidates = client.CreateOrCondition(&client.ControlViewCondition()?, &scrollable)?;
    let condition = client.CreateAndCondition(&candidates, &visible)?;
    client.CreateTreeWalker(&condition)
}
fn optional<T>(result: Result<T>) -> Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error)
            if error.code() == S_OK || error.code() == HRESULT(UIA_E_NOTSUPPORTED as i32) =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}
