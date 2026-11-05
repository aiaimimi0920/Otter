//! 裁剪选区仅保存起止点；松手一次提交，失焦/捕获丢失取消，不生成像素预览缓冲。
use super::{canvas::Canvas, State};
use crate::{
    crop_geometry::Rect,
    editor::Tool,
    viewport::{Point, Viewport},
};
use windows::{
    core::{w, Result},
    Win32::{
        Foundation::HWND,
        Graphics::Direct2D::{Common::*, *},
        UI::WindowsAndMessaging::*,
    },
};

pub(super) const START: usize = 0x102E;
pub(super) const RESET: usize = 0x102F;
pub(super) struct Drag {
    start: Point,
    end: Point,
    revision: u64,
}

impl Canvas {
    pub(super) fn begin_crop(&mut self, screen: Point) -> bool {
        let Some(point) = self.image_point(screen, false) else {
            return false;
        };
        self.crop_drag = Some(Drag {
            start: point,
            end: point,
            revision: self.editor.revision(),
        });
        // 共用手势准入标记，使滚轮和异步交付不会在拖动中改变坐标系。
        self.selection_drag = Some((point, Point::default()));
        true
    }
    pub(super) fn move_crop(&mut self, screen: Point) -> bool {
        let Some(point) = self.image_point(screen, true) else {
            return false;
        };
        let Some(drag) = self.crop_drag.as_mut() else {
            return false;
        };
        if drag.revision != self.editor.revision() {
            self.cancel_gesture();
            return true;
        }
        let changed = point != drag.end;
        drag.end = point;
        changed
    }
    pub(super) fn finish_crop(&mut self) -> bool {
        let Some(drag) = self.crop_drag.take() else {
            return false;
        };
        self.selection_drag = None;
        if drag.revision != self.editor.revision() {
            return true;
        }
        let width = (drag.end.x - drag.start.x).abs();
        let height = (drag.end.y - drag.start.y).abs();
        if width < 4.0 || height < 4.0 {
            return true;
        }
        let offset = self
            .editor
            .crop_view()
            .map_or((0.0, 0.0), |view| (view.source.rect.x, view.source.rect.y));
        let selection = Rect {
            x: drag.start.x.min(drag.end.x) as f64 - offset.0,
            y: drag.start.y.min(drag.end.y) as f64 - offset.1,
            width: width as f64,
            height: height as f64,
        };
        super::crop_apply::prepare(self, Some(selection));
        true
    }
}

pub(super) unsafe fn append(menu: HMENU, state: &State) -> Result<()> {
    let canvas = state.canvas.borrow();
    let display = state.resolved_display.borrow();
    let crop = display.as_ref().map_or_else(
        || canvas.editor.crop_view(),
        |frame| {
            frame
                .editor
                .as_deref()
                .and_then(|editor| editor.crop_view())
        },
    );
    AppendMenuW(
        menu,
        MF_STRING
            | if canvas.crop_mode {
                MF_CHECKED
            } else {
                MF_UNCHECKED
            },
        START,
        w!("裁剪选区"),
    )?;
    AppendMenuW(
        menu,
        MF_STRING
            | if crop.is_some() {
                MF_ENABLED
            } else {
                MF_GRAYED
            },
        RESET,
        w!("重置裁剪"),
    )
}

pub(super) unsafe fn command(hwnd: HWND, state: &State, command: usize) {
    if !matches!(command, START | RESET) || state.group_locked.get() {
        return;
    }
    if state.erase_job.borrow().is_busy()
        || state.crop_job.borrow().is_some()
        || state.resize_job.borrow().is_busy()
        || state.flip_job.borrow().is_busy()
        || state.rasterize_job.borrow().is_some()
    {
        return;
    }
    if command == RESET && !super::display_edit::prepare_crop(state) {
        return;
    }
    let mut canvas = state.canvas.borrow_mut();
    let active = canvas.crop_mode;
    canvas.select_tool(Tool::Select);
    if command == START {
        canvas.crop_mode = !active;
    } else {
        super::crop_apply::prepare(&mut canvas, None);
    }
    drop(canvas);
    super::crop_apply::submit(hwnd, state);
    if let Err(error) = super::crop_window::sync(hwnd, state) {
        eprintln!("native-crop-window: {error}");
    }
    let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
    let _ = super::hit_region::update(hwnd, state);
    let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(hwnd), None, false);
}

pub(super) unsafe fn draw(
    target: &ID2D1RenderTarget,
    brush: &ID2D1SolidColorBrush,
    drag: Option<&Drag>,
    viewport: Viewport,
) {
    let Some(drag) = drag else {
        return;
    };
    let rect = D2D_RECT_F {
        left: viewport.origin.x + drag.start.x.min(drag.end.x) * viewport.scale,
        top: viewport.origin.y + drag.start.y.min(drag.end.y) * viewport.scale,
        right: viewport.origin.x + drag.start.x.max(drag.end.x) * viewport.scale,
        bottom: viewport.origin.y + drag.start.y.max(drag.end.y) * viewport.scale,
    };
    let previous = brush.GetColor();
    brush.SetColor(&D2D1_COLOR_F {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    });
    target.DrawRectangle(&rect, brush, 3.0, None);
    brush.SetColor(&D2D1_COLOR_F {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    });
    target.DrawRectangle(&rect, brush, 1.0, None);
    brush.SetColor(&previous);
}
