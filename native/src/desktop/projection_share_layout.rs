//! 分享浮层独立于钉图尺寸；只布局当前视图，几何和可见性不变时不触发重绘。
use super::*;
pub(super) fn template() -> Vec<u32> {
    fn text(out: &mut Vec<u16>, value: &str) {
        out.extend(value.encode_utf16());
        out.push(0);
    }
    fn dword(out: &mut Vec<u16>, value: u32) {
        out.extend([value as u16, (value >> 16) as u16]);
    }
    let mut out = Vec::new();
    dword(&mut out, WS_POPUP.0 | WS_CLIPCHILDREN.0 | DS_SETFONT as u32);
    dword(&mut out, WS_EX_TOOLWINDOW.0);
    out.extend([11, 0, 0, 224, 240, 0, 0]);
    text(&mut out, "投射分享");
    out.push(9);
    text(&mut out, "Microsoft YaHei UI");
    for (id, class, style, label) in [
        (1, 0x80, WS_TABSTOP.0 | BS_OWNERDRAW as u32, "导入"),
        (2, 0x80, WS_TABSTOP.0 | BS_OWNERDRAW as u32, "×"),
        (
            101,
            0x81,
            WS_TABSTOP.0 | WS_BORDER.0 | ES_AUTOHSCROLL as u32,
            "",
        ),
        (102, 0x82, 0, ""),
        (103, 0x80, WS_TABSTOP.0 | BS_OWNERDRAW as u32, "选择图片"),
        (104, 0x80, WS_TABSTOP.0 | BS_OWNERDRAW as u32, "二维码"),
        (105, 0x80, WS_TABSTOP.0 | BS_OWNERDRAW as u32, "链接"),
        (108, 0x80, WS_TABSTOP.0 | BS_OWNERDRAW as u32, "解除关联"),
        (109, 0x80, WS_TABSTOP.0 | BS_OWNERDRAW as u32, "局域网"),
        (110, 0x80, WS_TABSTOP.0 | BS_OWNERDRAW as u32, "公网"),
        (
            106,
            0x82,
            windows::Win32::System::SystemServices::SS_OWNERDRAW.0,
            "投射邀请二维码",
        ),
    ] {
        if out.len() % 2 != 0 {
            out.push(0);
        }
        dword(&mut out, WS_CHILD.0 | WS_VISIBLE.0 | style);
        dword(&mut out, 0);
        out.extend([0, 0, 1, 1, id, 0xffff, class]);
        text(&mut out, label);
        out.push(0);
    }
    if out.len() % 2 != 0 {
        out.push(0);
    }
    out.chunks_exact(2)
        .map(|p| u32::from(p[0]) | (u32::from(p[1]) << 16))
        .collect()
}
pub(super) fn width(m: &Model) -> i32 {
    if !m.receive && m.qr.get() {
        m.code
            .borrow()
            .as_ref()
            .map_or(288, |c| ((c.size as i32 + 8) * 2 + 16).max(288))
    } else {
        288
    }
}
pub(super) fn height(m: &Model) -> i32 {
    if !m.receive && m.qr.get() {
        width(m) + 104
    } else {
        176
    }
}
pub(super) unsafe fn arrange(hwnd: HWND, m: &Model, dpi: u32) {
    let dpi = dpi.max(96);
    let px = |n: i32| (n * dpi as i32 + 48) / 96;
    let mut bounds = RECT::default();
    let _ = GetClientRect(hwnd, &mut bounds);
    let width = bounds.right * 96 / dpi as i32;
    let height = bounds.bottom * 96 / dpi as i32;
    let inner = (width - 16).max(1);
    let qr_side = inner.min((height - 120).max(1));
    let account_linked = m.public.get()
        && m.view.upgrade().is_some_and(|view| {
            view.borrow().account.bindings.iter().any(|binding| {
                binding.unit() == m.unit.as_str()
                    && binding.origin() == m.origin
                    && !binding.stopped()
            })
        });
    let items = [
        (2, width - 32, 4, 28, 24, true),
        (109, 8, 4, 72, 24, true),
        (110, 82, 4, 72, 24, true),
        (104, 8, 34, inner / 2 - 2, 28, true),
        (105, 10 + inner / 2, 34, inner / 2 - 2, 28, true),
        (103, 8, 70, inner, 32, m.receive && m.qr.get()),
        (101, 8, 70, inner, 28, !m.qr.get()),
        (
            102,
            8,
            height - 68,
            inner,
            28,
            !m.status.borrow().is_empty(),
        ),
        (108, 8, height - 36, 96, 28, !m.receive || account_linked),
        (1, width - 96, height - 36, 88, 28, true),
        (
            106,
            (width - qr_side) / 2,
            68,
            qr_side,
            qr_side,
            !m.receive && m.qr.get() && m.code.borrow().is_some() && m.status.borrow().is_empty(),
        ),
    ];
    for (id, x, y, w, h, visible) in items {
        if let Ok(control) = GetDlgItem(Some(hwnd), id) {
            let desired = RECT {
                left: px(x),
                top: px(y),
                right: px(x + w),
                bottom: px(y + h),
            };
            let mut old = RECT::default();
            let _ = GetWindowRect(control, &mut old);
            let _ = windows::Win32::Graphics::Gdi::MapWindowPoints(
                None,
                Some(hwnd),
                std::slice::from_raw_parts_mut((&mut old as *mut RECT).cast::<POINT>(), 2),
            );
            if old != desired {
                let _ = MoveWindow(
                    control,
                    desired.left,
                    desired.top,
                    desired.right - desired.left,
                    desired.bottom - desired.top,
                    true,
                );
            }
            // 父窗尚未显示时 IsWindowVisible 恒为 false；必须比较子控件自身的显示标记。
            if (GetWindowLongW(control, GWL_STYLE) as u32 & WS_VISIBLE.0 != 0) != visible {
                let _ = ShowWindow(control, if visible { SW_SHOWNA } else { SW_HIDE });
            }
        }
    }
}
