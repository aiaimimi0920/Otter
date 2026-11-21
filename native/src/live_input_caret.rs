//! 已授权源的系统 caret 只读观测；不附加输入队列、不聚焦，也不读取控件文本。
use super::{Dpi, Target};
use crate::live_window_target::region::Crop;
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*,
        Graphics::Gdi::ClientToScreen,
        UI::{HiDpi::*, WindowsAndMessaging::*},
    },
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Observation {
    pub(crate) point: [f64; 2],
    pub(crate) size: [u32; 2],
}
/// 返回相对采集帧/crop 的连续位置，可能在 0..1 之外；仅供显示，不能作为输入授权。
pub(crate) fn sample(root: &Target, crop: Option<Crop>) -> Result<Option<Observation>> {
    sample_with(root, crop, || {
        let mut info = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        // 没有 GUI 队列或正在切换激活时可以没有系统 caret，保留点击后备定位。
        unsafe { GetGUIThreadInfo(root.identity.thread, &mut info) }
            .ok()
            .map(|_| info)
    })
}

fn sample_with(
    root: &Target,
    crop: Option<Crop>,
    mut read: impl FnMut() -> Option<GUITHREADINFO>,
) -> Result<Option<Observation>> {
    root.validate()?;
    if unsafe { IsIconic(root.hwnd()) }.as_bool() {
        return Ok(None);
    }
    let Some(before) = read().filter(|info| in_scope(root, info)) else {
        return Ok(None);
    };
    let frame = root.frame_bounds()?;
    let point = physical(&before)?;
    let after = read();
    root.validate()?;
    // 源可以在读坐标期间移动 caret、换焦点、销毁/复用控件或移动整个窗口。
    // 不使用不一致的两次观测，也不将无关线程的焦点/caret 绑定成新的源。
    let Some(after) = after.filter(|info| in_scope(root, info) && same(&before, info)) else {
        return Ok(None);
    };
    let current = physical(&after)?;
    if point.x != current.x || point.y != current.y || rect(frame) != rect(root.frame_bounds()?) {
        return Ok(None);
    }
    root.validate()?;
    Ok(Some(relative(frame, crop, point)?))
}

fn in_scope(root: &Target, info: &GUITHREADINFO) -> bool {
    [info.hwndFocus, info.hwndCaret].into_iter().all(|hwnd| {
        let mut pid = 0;
        let tid = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        pid == root.identity.process
            && tid == root.identity.thread
            && (hwnd == root.hwnd() || unsafe { IsChild(root.hwnd(), hwnd) }.as_bool())
    }) && info.rcCaret.right >= info.rcCaret.left
        && info.rcCaret.bottom > info.rcCaret.top
}

fn same(before: &GUITHREADINFO, after: &GUITHREADINFO) -> bool {
    before.hwndFocus == after.hwndFocus
        && before.hwndCaret == after.hwndCaret
        && rect(before.rcCaret) == rect(after.rcCaret)
}
fn rect(r: RECT) -> [i32; 4] {
    [r.left, r.top, r.right, r.bottom]
}

fn physical(info: &GUITHREADINFO) -> Result<POINT> {
    // rcCaret 始终属于 caret 窗口的逻辑客户坐标，不随调用方 DPI 虚拟化。
    let _dpi = Dpi::enter(info.hwndCaret)
        .map_err(|_| Error::new(E_FAIL, "Live caret DPI context unavailable"))?;
    let mut point = POINT {
        x: info.rcCaret.left,
        y: info.rcCaret.bottom - 1,
    };
    let context = unsafe { GetWindowDpiAwarenessContext(info.hwndCaret) };
    if !unsafe { ClientToScreen(info.hwndCaret, &mut point) }.as_bool()
        || (unsafe { GetAwarenessFromDpiAwarenessContext(context) }
            != DPI_AWARENESS_PER_MONITOR_AWARE
            && !unsafe { LogicalToPhysicalPointForPerMonitorDPI(Some(info.hwndCaret), &mut point) }
                .as_bool())
    {
        return Err(Error::new(E_FAIL, "Live caret physical transform failed"));
    }
    Ok(point)
}

fn relative(frame: RECT, crop: Option<Crop>, point: POINT) -> Result<Observation> {
    let bounds = if let Some(crop) = crop {
        crop.screen(frame)?
    } else {
        frame
    };
    let width = i64::from(bounds.right) - i64::from(bounds.left);
    let height = i64::from(bounds.bottom) - i64::from(bounds.top);
    if width <= 0 || height <= 0 {
        return Err(Error::new(E_FAIL, "Live caret frame bounds unavailable"));
    }
    // 不夹紧：已知 caret 在选区之外时应隐藏该定位，而不是使用旧点击假装仍在选区内。
    Ok(Observation {
        point: [
            (i64::from(point.x) - i64::from(bounds.left)) as f64 / width as f64,
            (i64::from(point.y) - i64::from(bounds.top)) as f64 / height as f64,
        ],
        size: [width as u32, height as u32],
    })
}

#[cfg(test)]
#[path = "live_input_caret_tests.rs"]
mod tests;
