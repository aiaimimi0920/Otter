//! 参数窗候选卡片复用同一Snapshot提交result_index，再排唯一显式执行；绘制无I/O。
use super::*;
use std::sync::Arc;
type Set = desktop::loom_output_batch::candidates::Set;
pub(in crate::desktop::document_window) struct Choice {
    set: Arc<Set>,
    item: usize,
    snapshot: Option<Rc<Snapshot>>,
    stamp: Stamp,
    selected: bool,
    parameter: Option<usize>,
}
fn parameter(snapshot: &Snapshot, index: u32) -> Option<usize> {
    snapshot.choices.iter().position(|c| {
        c.id == "result_index"
            && c.enabled
            && matches!(&c.action, Action::Edit(spec, _) if spec.number
                && spec.parse(&index.to_string()) == Some(serde_json::Value::from(index)))
    })
}
impl Choice {
    pub(in super::super) fn selected(&self) -> bool {
        self.selected
    }
    pub(in super::super) fn card(&self, state: &State) -> neuro_mini_ui::PreviewCard {
        let item = &self.set.items[self.item];
        neuro_mini_ui::PreviewCard {
            title: if item.title.is_empty() {
                format!("候选 {}", item.index + 1)
            } else {
                item.title.clone()
            },
            detail: format!("#{}", item.index + 1),
            enabled: self.available(state),
            preview: item
                .thumbnail
                .clone()
                .map(|t| Rc::new(t) as Rc<dyn neuro_mini_ui::CardPreview>),
        }
    }
    pub(in super::super) fn available(&self, state: &State) -> bool {
        self.stamp.matches(state)
            && self.snapshot.as_ref().is_some_and(|s| s.current(state))
            && self.parameter.is_some()
            && desktop::execution_start::available(state)
            && state
                .execution_results
                .borrow()
                .candidate_view(&self.stamp)
                .is_some_and(|set| Arc::ptr_eq(set, &self.set))
    }
    pub(in super::super) unsafe fn submit(
        &self,
        hwnd: HWND,
        state: &State,
        current: &dyn Fn() -> bool,
    ) -> Result<bool> {
        if !current()
            || !state.lifetime.owns(hwnd)
            || !super::super::available(state)
            || !self.available(state)
        {
            return Ok(false);
        }
        let Some(parameter) = self.parameter else {
            return Ok(false);
        };
        let Some(snapshot) = &self.snapshot else {
            return Ok(false);
        };
        let text = self.set.items[self.item].index.to_string();
        // 验证原Spec；相同index也允许显式重跑，但绝不发布缩略图为正式图像。
        let Action::Edit(spec, _) = &snapshot.choices[parameter].action else {
            return Ok(false);
        };
        let Some(value) = spec.parse(&text) else {
            return Ok(false);
        };
        // 数值控件允许clamp，但选择候选必须精确指向该index，不能悄悄选择另一项。
        if !spec.number || value != self.set.items[self.item].index {
            return Ok(false);
        }
        let unchanged = state
            .selected_art
            .borrow()
            .as_ref()
            .is_some_and(|selected| {
                selected.parameters.get("result_index").flatten() == Some(&value)
            });
        if !unchanged && !snapshot.submit_guarded(hwnd, state, parameter, Some(&text), current)? {
            return Ok(false);
        }
        desktop::execution_start::request_after_parameter(hwnd, state)?;
        Ok(true)
    }
}
pub(super) unsafe fn build(b: &mut Builder<'_>, model: &model::Model) -> Result<()> {
    for (position, choice) in choices(model, b.ui.state()).into_iter().enumerate() {
        if position == 0 {
            b.label("候选")?;
            b.y += 24;
        }
        let item = &choice.set.items[choice.item];
        let text = format!(
            "{}#{} {}",
            if choice.selected { "✓ " } else { "" },
            item.index + 1,
            item.title
        );
        let hwnd = b.child(
            w!("BUTTON"),
            &text,
            WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32),
            12 + (position as i32 % 2) * 214,
            202,
            80,
        )?;
        let enabled = b.ui.state().is_some_and(|s| choice.available(s));
        b.bindings.push(Binding {
            hwnd,
            key: format!("candidate:{}", item.index),
            kind: Kind::Candidate(choice),
            enabled,
        });
        if position % 2 == 1 || position + 1 == model.candidates.as_ref().unwrap().items.len() {
            b.y += 86;
        }
    }
    Ok(())
}
pub(in super::super) fn choices(model: &model::Model, state: Option<&State>) -> Vec<Rc<Choice>> {
    let Some(set) = &model.candidates else {
        return Vec::new();
    };
    let selected = set.selected.or_else(|| {
        state.and_then(|state| {
            state.selected_art.borrow().as_ref().and_then(|selection| {
                selection
                    .parameters
                    .get("result_index")
                    .flatten()
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|n| u32::try_from(n).ok())
            })
        })
    });
    set.items
        .iter()
        .enumerate()
        .map(|(position, item)| {
            let selected = selected == Some(item.index);
            Rc::new(Choice {
                set: set.clone(),
                item: position,
                snapshot: model.snapshot.clone(),
                stamp: model.stamp.clone(),
                selected,
                parameter: model
                    .snapshot
                    .as_ref()
                    .and_then(|s| parameter(s, item.index)),
            })
        })
        .collect()
}
impl neuro_mini_ui::CardPreview for crate::image_thumbnail::Thumbnail {
    unsafe fn paint(&self, dc: HDC, rect: RECT) {
        crate::image_thumbnail::draw::draw_cover(dc, rect, self, 0x110c09);
    }
}
pub(in super::super) unsafe fn draw(ui: &Ui, item: &DRAWITEMSTRUCT) -> bool {
    let binding = ui
        .bindings
        .borrow()
        .iter()
        .find(|b| b.hwnd == item.hwndItem)
        .cloned();
    let Some(Binding {
        kind: Kind::Candidate(choice),
        ..
    }) = binding
    else {
        return false;
    };
    let candidate = &choice.set.items[choice.item];
    let background = 0x201711;
    theme::fill(item.hDC, &item.rcItem, background);
    let scale = GetDpiForWindow(ui.hwnd.get()).max(96) as i32;
    let mut thumb = item.rcItem;
    thumb.left += 4 * scale / 96;
    thumb.right = thumb.left + 68 * scale / 96;
    thumb.top += 4 * scale / 96;
    thumb.bottom -= 4 * scale / 96;
    if let Some(thumbnail) = &candidate.thumbnail {
        crate::image_thumbnail::draw::draw(item.hDC, thumb, thumbnail, background);
    }
    let old = SelectObject(
        item.hDC,
        HGDIOBJ(SendMessageW(item.hwndItem, WM_GETFONT, None, None).0 as *mut _),
    );
    SetBkMode(item.hDC, TRANSPARENT);
    SetTextColor(
        item.hDC,
        COLORREF(if item.itemState.0 & ODS_DISABLED.0 != 0 {
            0x9f9a92
        } else {
            0xeff8f7
        }),
    );
    let mut rect = item.rcItem;
    rect.left = thumb.right + 4 * scale / 96;
    rect.right -= 4 * scale / 96;
    rect.top += 8 * scale / 96;
    let mut text = [0; 260];
    let count = GetWindowTextW(item.hwndItem, &mut text).max(0) as usize;
    DrawTextW(
        item.hDC,
        &mut text[..count],
        &mut rect,
        DT_WORDBREAK | DT_NOPREFIX | DT_END_ELLIPSIS,
    );
    if choice.selected || item.itemState.0 & ODS_FOCUS.0 != 0 {
        SetDCBrushColor(item.hDC, COLORREF(0x38ffd9));
        FrameRect(item.hDC, &item.rcItem, HBRUSH(GetStockObject(DC_BRUSH).0));
    }
    SelectObject(item.hDC, old);
    true
}
