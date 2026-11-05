//! 隐藏 HWND 注入颜色结果，不调用桌面采样，也不改动真实鼠标捕获。
use super::*;
struct Fixture(HWND);
impl Fixture {
    unsafe fn new() -> Self {
        Self(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("color test"),
                WS_OVERLAPPED,
                0,
                0,
                20,
                20,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}
unsafe fn begin(hwnd: HWND, state: &State, fill: bool) {
    *state.screen_color.borrow_mut() = Some(prepare(hwnd, state, fill).unwrap());
}
#[test]
fn stroke_fill_and_cancel_restore_modal_and_destroy_preview() {
    unsafe {
        let fixture = Fixture::new();
        let state = State::new(None, false).unwrap();
        let initial = state.canvas.borrow().editor.style();
        begin(fixture.0, &state, false);
        let popup = state.screen_color.borrow().as_ref().unwrap().preview.0;
        assert!(state.file_dialog.get());
        assert!(prepare(fixture.0, &state, true).is_err());
        finish(fixture.0, &state, Some(Color(1, 2, 3)));
        assert_eq!(state.canvas.borrow().editor.style().color(), Color(1, 2, 3));
        assert_eq!(state.canvas.borrow().editor.style().fill(), initial.fill());
        assert!(!state.file_dialog.get());
        assert!(!IsWindow(Some(popup)).as_bool());
        begin(fixture.0, &state, true);
        finish(fixture.0, &state, Some(Color(4, 5, 6)));
        assert_eq!(
            state.canvas.borrow().editor.style().fill(),
            Some(Color(4, 5, 6))
        );
        let before = state.canvas.borrow().editor.style();
        begin(fixture.0, &state, false);
        dispatch(fixture.0, &state, WM_KEYDOWN, WPARAM(VK_ESCAPE.0 as usize));
        assert_eq!(state.canvas.borrow().editor.style(), before);
        assert!(state.screen_color.borrow().is_none());
        let history = state.capture_history.borrow();
        assert_eq!(history.data.colors.len(), 2);
        assert_eq!(history.data.colors[0].hex, "#040506");
        assert_eq!(history.data.colors[1].hex, "#010203");
    }
}
#[test]
fn stale_style_tool_or_closed_owner_cannot_apply_and_commands_do_not_reenter() {
    unsafe {
        for mode in 0..3 {
            let fixture = Fixture::new();
            let state = State::new(None, false).unwrap();
            begin(fixture.0, &state, false);
            assert!(dispatch(fixture.0, &state, WM_COMMAND, WPARAM(STROKE)).is_some());
            match mode {
                0 => {
                    let style = state.canvas.borrow().editor.style();
                    state
                        .canvas
                        .borrow_mut()
                        .editor
                        .set_style(style.with_color(Color(7, 8, 9)));
                }
                1 => {
                    state.canvas.borrow_mut().select_tool(Tool::Brush);
                }
                _ => DestroyWindow(fixture.0).unwrap(),
            }
            let before = state.canvas.borrow().editor.style();
            finish(fixture.0, &state, Some(Color(1, 2, 3)));
            assert_eq!(state.canvas.borrow().editor.style(), before);
            assert!(!state.file_dialog.get());
            assert!(state.capture_history.borrow().data.colors.is_empty());
        }
    }
}
