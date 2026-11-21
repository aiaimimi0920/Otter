//! 在隐藏准备边界解析采样坐标下的自有钉图；整矩形命中不受透明像素影响。
use super::*;
use crate::{desktop::clipboard_placement, pin_geometry::PinRect};
use windows::core::BOOL;

unsafe fn visible_frame(doc: &DocumentWindow) -> Option<PinRect> {
    let hwnd = doc.window.0;
    if !doc.is_alive()
        || !doc.state.canvas.borrow().pinned
        || doc.state.group_hidden.get()
        || !IsWindowVisible(hwnd).as_bool()
        || IsIconic(hwnd).as_bool()
    {
        return None;
    }
    let mut rect = RECT::default();
    GetWindowRect(hwnd, &mut rect).ok()?;
    clipboard_placement::rectangle(rect)
}
struct Search<'a> {
    candidates: &'a [(HWND, PinRect)],
    selected: Option<PinRect>,
}
unsafe extern "system" fn visit(hwnd: HWND, data: LPARAM) -> BOOL {
    // EnumWindows 同步调用，只匹配已校验的自有 HWND，不读取其它程序的内容或几何。
    let search = &mut *(data.0 as *mut Search<'_>);
    search.selected = search
        .candidates
        .iter()
        .find(|(candidate, _)| *candidate == hwnd)
        .map(|(_, frame)| *frame);
    BOOL::from(search.selected.is_none())
}
impl Application {
    pub(super) unsafe fn paste_frame(
        &self,
        request: &clipboard_placement::Request,
    ) -> Result<PinRect> {
        let (source_id, saved_frame) = request.source();
        let source = self
            .documents
            .iter()
            .find(|doc| doc.state.unit_id == *source_id && self.workspace.contains(source_id));
        let source_frame = source.map_or(Some(saved_frame), |doc| visible_frame(doc));
        // 先识别最初复制的 A，不能让上层副本或鼠标在 A 内移动改变堆叠序列。
        if let Some(frame) = source_frame.filter(|frame| request.contains_cursor(*frame)) {
            return Ok(request.cascade_in_source(frame));
        }
        if request.continuing() {
            return Ok(request.frame(None));
        }
        let candidates: Vec<_> = self
            .documents
            .iter()
            .filter(|doc| self.workspace.contains(&doc.state.unit_id))
            .filter_map(|doc| visible_frame(doc).map(|frame| (doc.window.0, frame)))
            .filter(|(_, frame)| request.contains_cursor(*frame))
            .collect();
        let target = match candidates.as_slice() {
            [] => None,
            [(_, frame)] => Some(*frame),
            _ => {
                let mut search = Search {
                    candidates: &candidates,
                    selected: None,
                };
                let result =
                    EnumWindows(Some(visit), LPARAM(&mut search as *mut Search<'_> as isize));
                // 找到最上层的自有矩形后主动停止枚举，不把正常停止解释为 Win32 错误。
                if search.selected.is_none() {
                    result?;
                }
                search.selected
            }
        };
        if target.is_none() && self.source_window(request.unit()).is_some() {
            // 存活但不可命中的锚点（含隐藏/最小化）不能通过来源兜底重新参与图内定位。
            return Ok(request.relative_frame());
        }
        Ok(request.frame(target))
    }
}
