//! Actual captionless input-layer/document-transaction regressions; create only synthetic pins on an isolated desktop, without user instances.
use super::*;
use windows::Win32::{System::LibraryLoader::GetModuleHandleW, UI::Controls::*};
struct Fixture {
    hwnd: HWND,
    state: Box<State>,
}
impl Fixture {
    unsafe fn new() -> Self {
        let class = w!("Otter.Native.InlineTextFixture");
        let instance = HINSTANCE(GetModuleHandleW(None).unwrap().0);
        let registered = RegisterClassW(&WNDCLASSW {
            hInstance: instance,
            lpszClassName: class,
            lpfnWndProc: Some(desktop::procedure::window_proc),
            ..Default::default()
        });
        assert!(registered != 0 || GetLastError() == ERROR_CLASS_ALREADY_EXISTS);
        let state = Box::new(
            State::new(
                Some(crate::image_decode::Image {
                    width: 400,
                    height: 240,
                    pixels: [70, 80, 90, 255].repeat(400 * 240).into(),
                }),
                true,
            )
            .unwrap(),
        );
        let hwnd = CreateWindowExW(
            WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOOLWINDOW,
            class,
            w!("Inline text fixture"),
            WS_POPUP,
            150,
            180,
            400,
            240,
            None,
            None,
            Some(instance),
            Some((&*state as *const State).cast()),
        )
        .unwrap();
        state.canvas.borrow_mut().viewport = crate::viewport::Viewport {
            origin: Point::default(),
            scale: 1.0,
        };
        state
            .canvas
            .borrow_mut()
            .select_tool(crate::editor::Tool::Text);
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        Self { hwnd, state }
    }
    unsafe fn start(&self) -> (HWND, HWND) {
        super::super::insert(self.hwnd, &self.state, Point { x: 30.0, y: 40.0 });
        self.handles()
    }
    unsafe fn handles(&self) -> (HWND, HWND) {
        let popup = self
            .state
            .inline_text
            .borrow()
            .as_ref()
            .expect("inline session; no modal dialog")
            .ui
            .hwnd();
        (popup, GetWindow(popup, GW_CHILD).unwrap())
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe {
            if IsWindow(Some(self.hwnd)).as_bool() {
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}
unsafe fn enter(edit: HWND) {
    SendMessageW(edit, WM_KEYDOWN, Some(WPARAM(VK_RETURN.0 as usize)), None);
}
unsafe fn drain(f: &Fixture) {
    let mut message = MSG::default();
    while PeekMessageW(
        &mut message,
        Some(f.hwnd),
        messages::FINISH,
        messages::CANCEL,
        PM_REMOVE,
    )
    .as_bool()
    {
        DispatchMessageW(&message);
    }
}
#[test]
fn click_opens_empty_anchored_nonmodal_input_and_enter_commits_one_undo_step() {
    unsafe {
        let f = Fixture::new();
        let (popup, edit) = f.start();
        assert_eq!(GetWindowTextLengthW(edit), 0);
        assert_eq!(GetAncestor(popup, GA_ROOTOWNER), f.hwnd);
        assert_eq!(
            GetWindowLongW(popup, GWL_STYLE) as u32 & (WS_CAPTION | WS_DLGFRAME).0,
            0
        );
        assert_eq!(GetFocus(), edit);
        assert!(!f.state.file_dialog.get());
        assert!(desktop::group_lock::editing_busy(&f.state));
        let mut before = RECT::default();
        GetWindowRect(popup, &mut before).unwrap();
        let mut point = POINT { x: 30, y: 40 };
        ClientToScreen(f.hwnd, &mut point).ok().unwrap();
        assert_eq!((before.left, before.top), (point.x, point.y));
        // Multiline EDIT WM_SETTEXT sends no EN_CHANGE; use actual edit messages to verify input expansion.
        SendMessageW(
            edit,
            EM_REPLACESEL,
            Some(WPARAM(1)),
            Some(LPARAM(
                w!("直接输入中文，连续文本会自动扩展输入框，不需要额外弹窗").as_ptr() as isize,
            )),
        );
        let mut after = RECT::default();
        GetWindowRect(popup, &mut after).unwrap();
        assert!(after.right - after.left > before.right - before.left);
        assert!(
            f.state.canvas.borrow().editor.elements().is_empty(),
            "draft is not document history"
        );
        enter(edit);
        assert!(f.state.inline_text.borrow().is_none());
        assert!(!IsWindow(Some(popup)).as_bool());
        let mut canvas = f.state.canvas.borrow_mut();
        assert_eq!(canvas.editor.elements().len(), 1);
        assert!(canvas.editor.elements()[0]
            .text
            .as_ref()
            .unwrap()
            .content()
            .starts_with("直接输入中文"));
        assert!(canvas.editor.undo());
        assert!(canvas.editor.elements().is_empty());
        assert!(canvas.editor.redo());
        assert_eq!(canvas.editor.elements().len(), 1);
    }
}
#[test]
fn escape_and_empty_draft_leave_document_untouched() {
    unsafe {
        let f = Fixture::new();
        let (_, edit) = f.start();
        SetWindowTextW(edit, w!("取消的草稿")).unwrap();
        SendMessageW(edit, WM_KEYDOWN, Some(WPARAM(VK_ESCAPE.0 as usize)), None);
        assert!(f.state.inline_text.borrow().is_none());
        let (_, edit) = f.start();
        enter(edit);
        assert!(f.state.inline_text.borrow().is_none());
        assert!(f.state.canvas.borrow().editor.elements().is_empty());
        assert!(!f.state.canvas.borrow_mut().editor.undo());
    }
}
#[path = "text_inline_style_tests.rs"]
mod style;

#[test]
fn owner_escape_cancels_draft_without_closing_pin_or_committing_queued_focus_exit() {
    unsafe {
        let f = Fixture::new();
        let (_, edit) = f.start();
        SetWindowTextW(edit, w!("宿主取消草稿")).unwrap();
        let _ = SetFocus(Some(f.hwnd));
        SendMessageW(f.hwnd, WM_KEYDOWN, Some(WPARAM(VK_ESCAPE.0 as usize)), None);
        drain(&f);
        assert!(IsWindow(Some(f.hwnd)).as_bool());
        assert!(f.state.inline_text.borrow().is_none());
        assert!(f.state.canvas.borrow().editor.elements().is_empty());
        assert!(!f.state.canvas.borrow_mut().editor.undo());
    }
}
#[test]
fn displayed_image_adoption_waits_for_inline_commit_or_cancel() {
    unsafe {
        let f = Fixture::new();
        let original = f.state.canvas.borrow().editor.document_stamp();
        let frame = desktop::display_frame::Frame::capture(
            desktop::document_state::new_unit_id().unwrap(),
            crate::image_decode::Image {
                width: 400,
                height: 240,
                pixels: [20, 30, 40, 255].repeat(400 * 240).into(),
            },
            None,
        );
        let identity = frame.identity.clone();
        *f.state.resolved_display.borrow_mut() = Some(frame);
        let (_, edit) = f.start();
        SetWindowTextW(edit, w!("取消接管")).unwrap();
        assert!(f.state.pending_adoption.borrow().is_some());
        assert!(!desktop::display_edit::settle(&f.state));
        finish(f.hwnd, &f.state, false).unwrap();
        assert!(desktop::display_edit::settle(&f.state));
        assert!(original.matches(&f.state.canvas.borrow().editor));
        assert!(std::sync::Arc::ptr_eq(
            &identity,
            &f.state.resolved_display.borrow().as_ref().unwrap().identity
        ));
        let (_, edit) = f.start();
        SetWindowTextW(edit, w!("当前图像上的文字")).unwrap();
        assert!(!desktop::display_edit::settle(&f.state));
        enter(edit);
        assert!(desktop::display_edit::settle(&f.state));
        assert!(f.state.pending_adoption.borrow().is_none());
        assert!(f.state.resolved_display.borrow().is_none());
        assert_eq!(f.state.canvas.borrow().editor.elements().len(), 1);
    }
}
#[test]
fn composition_enter_is_not_commit_and_end_then_enter_commits() {
    unsafe {
        let f = Fixture::new();
        let (_, edit) = f.start();
        SendMessageW(edit, WM_IME_STARTCOMPOSITION, None, None);
        SetWindowTextW(edit, w!("中文输入")).unwrap();
        enter(edit);
        assert!(
            f.state.inline_text.borrow().is_some(),
            "candidate confirmation cannot finish annotation"
        );
        SendMessageW(edit, WM_IME_ENDCOMPOSITION, None, None);
        enter(edit);
        assert!(f.state.inline_text.borrow().is_none());
        assert_eq!(
            f.state.canvas.borrow().editor.elements()[0]
                .text
                .as_ref()
                .unwrap()
                .content(),
            "中文输入"
        );
    }
}
#[test]
fn outside_pointer_commits_before_tool_change_without_consuming_the_click() {
    unsafe {
        let f = Fixture::new();
        let (_, edit) = f.start();
        SetWindowTextW(edit, w!("外部点击提交")).unwrap();
        assert!(!before_message(
            f.hwnd,
            &f.state,
            &MSG {
                hwnd: f.hwnd,
                message: WM_LBUTTONDOWN,
                ..Default::default()
            }
        ));
        assert!(f.state.inline_text.borrow().is_none());
        f.state
            .canvas
            .borrow_mut()
            .select_tool(crate::editor::Tool::Rectangle);
        assert_eq!(f.state.canvas.borrow().editor.elements().len(), 1);
    }
}
#[test]
fn focus_exit_commits_but_stale_document_or_destroyed_owner_never_receives_draft() {
    unsafe {
        let f = Fixture::new();
        let (popup, edit) = f.start();
        SetWindowTextW(edit, w!("失焦提交")).unwrap();
        let _ = SetFocus(Some(f.hwnd));
        drain(&f);
        assert!(f.state.inline_text.borrow().is_none());
        assert!(!IsWindow(Some(popup)).as_bool());
        assert_eq!(f.state.canvas.borrow().editor.elements().len(), 1);
        let (_, edit) = f.start();
        SetWindowTextW(edit, w!("过期草稿")).unwrap();
        f.state
            .canvas
            .borrow_mut()
            .select_tool(crate::editor::Tool::Rectangle);
        enter(edit);
        assert_eq!(f.state.canvas.borrow().editor.elements().len(), 1);
        f.state
            .canvas
            .borrow_mut()
            .select_tool(crate::editor::Tool::Text);
        let (popup, edit) = f.start();
        SetWindowTextW(edit, w!("关窗取消")).unwrap();
        DestroyWindow(f.hwnd).unwrap();
        assert!(f.state.inline_text.borrow().is_none());
        assert!(!IsWindow(Some(popup)).as_bool());
        assert!(!IsWindow(Some(edit)).as_bool());
    }
}
#[test]
fn f2_preserves_unchanged_multiline_content_and_rotation_then_edits_in_place() {
    unsafe {
        let f = Fixture::new();
        let (_, edit) = f.start();
        SetWindowTextW(edit, w!("第一行\r\n第二行")).unwrap();
        enter(edit);
        {
            let mut canvas = f.state.canvas.borrow_mut();
            canvas.editor.rotate(0, 35.0).unwrap();
            canvas.select_tool(crate::editor::Tool::Select);
            canvas.selected = Some((0, canvas.editor.revision()));
        }
        let before = f.state.canvas.borrow().editor.elements()[0].clone();
        super::super::edit_selected(f.hwnd, &f.state);
        let (_, edit) = f.handles();
        enter(edit);
        assert_eq!(
            f.state.canvas.borrow().editor.elements()[0].text,
            before.text
        );
        assert_eq!(
            f.state.canvas.borrow().editor.elements()[0].points,
            before.points
        );
        super::super::edit_selected(f.hwnd, &f.state);
        let (_, edit) = f.handles();
        SetWindowTextW(edit, w!("新的多行\r\n内容")).unwrap();
        enter(edit);
        let mut canvas = f.state.canvas.borrow_mut();
        assert_eq!(canvas.editor.elements()[0].rotation, 35.0);
        assert_eq!(canvas.editor.elements()[0].style(), before.style());
        assert!(canvas.editor.undo());
        assert_eq!(canvas.editor.elements()[0].text, before.text);
    }
}
#[test]
fn capture_affinity_and_original_text_limits_are_kept() {
    unsafe {
        let f = Fixture::new();
        SetWindowDisplayAffinity(f.hwnd, WDA_EXCLUDEFROMCAPTURE).unwrap();
        let (popup, edit) = f.start();
        let mut affinity = 0;
        GetWindowDisplayAffinity(popup, &mut affinity).unwrap();
        assert_eq!(affinity, WDA_EXCLUDEFROMCAPTURE.0);
        assert_eq!(
            SendMessageW(edit, EM_GETLIMITTEXT, None, None).0 as usize,
            crate::text_annotation::MAX_TEXT_UNITS
        );
        assert_ne!(
            SendMessageW(
                edit,
                EM_GETIMESTATUS,
                Some(WPARAM(EMSIS_COMPOSITIONSTRING as usize)),
                None
            )
            .0 & EIMES_COMPLETECOMPSTRKILLFOCUS as isize,
            0
        );
        let content = "中".repeat(20_000); // Exceeds the Surface field's 48KB budget but remains valid for existing annotations.
        SetWindowTextW(edit, &HSTRING::from(content.clone())).unwrap();
        enter(edit);
        assert_eq!(
            f.state.canvas.borrow().editor.elements()[0]
                .text
                .as_ref()
                .unwrap()
                .content(),
            content
        );
    }
}

#[test]
fn queued_focus_exit_cannot_commit_a_newer_input_session() {
    unsafe {
        let f = Fixture::new();
        let (old_popup, edit) = f.start();
        SetWindowTextW(edit, w!("旧输入")).unwrap();
        let old_epoch = WPARAM(GetPropW(old_popup, window::EPOCH).0 as usize);
        let _ = SetFocus(Some(f.hwnd)); // Queue focus loss without destroying windows inside system callbacks.
        assert!(f.state.inline_text.borrow().is_some());
        finish(f.hwnd, &f.state, false).unwrap();
        let (new_popup, edit) = f.start();
        SetWindowTextW(edit, w!("新输入")).unwrap();
        // Even if the system reuses the same HWND, an old epoch does not own a new draft.
        SendMessageW(
            f.hwnd,
            messages::FINISH,
            Some(old_epoch),
            Some(LPARAM(new_popup.0 as isize)),
        );
        drain(&f);
        assert!(f.state.inline_text.borrow().is_some());
        assert!(f.state.canvas.borrow().editor.elements().is_empty());
        enter(edit);
        assert_eq!(
            f.state.canvas.borrow().editor.elements()[0]
                .text
                .as_ref()
                .unwrap()
                .content(),
            "新输入"
        );
    }
}

#[test]
fn save_shortcut_commits_then_reaches_owner_once_instead_of_retired_edit() {
    use windows::Win32::UI::Shell::*;
    unsafe extern "system" fn observe(
        hwnd: HWND,
        msg: u32,
        wp: WPARAM,
        lp: LPARAM,
        id: usize,
        data: usize,
    ) -> LRESULT {
        if msg == WM_KEYDOWN && wp.0 == 0x53 {
            let calls = &*(data as *const std::cell::Cell<u32>);
            calls.set(calls.get() + 1);
            return LRESULT(0); // Do not open file selectors during automation.
        }
        if msg == WM_NCDESTROY {
            let _ = RemoveWindowSubclass(hwnd, Some(observe), id);
        }
        DefSubclassProc(hwnd, msg, wp, lp)
    }
    unsafe {
        let f = Fixture::new();
        let (_, edit) = f.start();
        SetWindowTextW(edit, w!("保存前提交")).unwrap();
        let calls = std::cell::Cell::new(0_u32);
        SetWindowSubclass(f.hwnd, Some(observe), 1, &calls as *const _ as usize)
            .ok()
            .unwrap();
        let mut original = [0; 256];
        GetKeyboardState(&mut original).unwrap();
        let mut keys = [0; 256];
        keys[VK_CONTROL.0 as usize] = 0x80;
        SetKeyboardState(&keys).unwrap();
        let handled = before_message(
            f.hwnd,
            &f.state,
            &MSG {
                hwnd: edit,
                message: WM_KEYDOWN,
                wParam: WPARAM(0x53),
                ..Default::default()
            },
        );
        SetKeyboardState(&original).unwrap();
        RemoveWindowSubclass(f.hwnd, Some(observe), 1).ok().unwrap();
        assert!(handled);
        assert_eq!(calls.get(), 1);
        assert!(f.state.inline_text.borrow().is_none());
        assert_eq!(
            f.state.canvas.borrow().editor.elements()[0]
                .text
                .as_ref()
                .unwrap()
                .content(),
            "保存前提交"
        );
    }
}
