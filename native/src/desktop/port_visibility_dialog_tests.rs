//! 标准LISTBOX/COMBOBOX首显前真实通知；无全局输入、无用户窗口。
use super::*;
use crate::workspace::ports::Visibility;
use std::any::Any;
type Initialize = Box<dyn FnOnce(HWND)>;
thread_local! {
    static READY:RefCell<Option<Initialize>>=const{RefCell::new(None)};
    static PANIC:RefCell<Option<Box<dyn Any+Send>>>=const{RefCell::new(None)};
}
pub(super) unsafe fn initialized(hwnd: HWND) {
    if let Some(action) = READY.with(|s| s.borrow_mut().take()) {
        if let Err(error) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| action(hwnd)))
        {
            PANIC.with(|s| *s.borrow_mut() = Some(error));
        }
        let _ = EndDialog(hwnd, 0);
    }
}
unsafe fn run(action: impl FnOnce(HWND) + 'static) -> Option<Visibility> {
    let app = super::super::super::tests::app();
    let mut catalog = Catalog::capture(&app.documents, &app.live_sessions, &app.workspace);
    let endpoint = &mut Rc::get_mut(&mut catalog).unwrap().endpoints[0];
    endpoint.inputs = vec!["same".into(), "中文🙂".into()];
    endpoint.outputs = vec!["same".into()];
    endpoint.input_defaults = vec![false, true];
    endpoint.output_defaults = vec![true];
    let model = Model::new(endpoint, Visibility::default());
    READY.with(|s| *s.borrow_mut() = Some(Box::new(action)));
    let result = show(app.documents[0].window.0, model, true);
    assert!(READY.with(|s| s.borrow().is_none()));
    if let Some(error) = PANIC.with(|s| s.borrow_mut().take()) {
        std::panic::resume_unwind(error);
    }
    result.unwrap()
}
unsafe fn choose(hwnd: HWND, row: usize, mode: usize) {
    SendMessageW(child(hwnd, PORTS), LB_SETCURSEL, Some(WPARAM(row)), None);
    SendMessageW(
        hwnd,
        WM_COMMAND,
        Some(WPARAM(PORTS as usize | ((LBN_SELCHANGE as usize) << 16))),
        None,
    );
    SendMessageW(child(hwnd, MODE), CB_SETCURSEL, Some(WPARAM(mode)), None);
    SendMessageW(
        hwnd,
        WM_COMMAND,
        Some(WPARAM(MODE as usize | ((CBN_SELCHANGE as usize) << 16))),
        None,
    );
}
unsafe fn command(hwnd: HWND, id: i32) {
    SendMessageW(hwnd, WM_COMMAND, Some(WPARAM(id as usize)), None);
}
#[test]
fn actual_controls_edit_shared_name_both_directions_reset_and_accept() {
    unsafe {
        let result = run(|hwnd| {
            assert_eq!(
                SendMessageW(child(hwnd, PORTS), LB_GETCOUNT, None, None).0,
                3
            );
            assert_eq!(selected(hwnd, MODE, CB_GETCURSEL), Some(0));
            choose(hwnd, 0, 2);
            SendMessageW(child(hwnd, PORTS), LB_SETCURSEL, Some(WPARAM(2)), None);
            refresh(
                hwnd,
                &*(GetWindowLongPtrW(hwnd, DWLP_USER) as *const Dialog),
            );
            assert_eq!(selected(hwnd, MODE, CB_GETCURSEL), Some(2));
            choose(hwnd, 2, 0);
            choose(hwnd, 1, 2);
            command(hwnd, IDOK.0);
        })
        .unwrap();
        assert_eq!(result.get("same"), None);
        assert_eq!(result.get("中文🙂"), Some(false));
    }
}
#[test]
fn cancel_close_and_invalid_selection_never_publish_a_draft() {
    unsafe {
        for close in [false, true] {
            assert!(run(move |hwnd| {
                choose(hwnd, 0, 2);
                choose(hwnd, usize::MAX, usize::MAX);
                if close {
                    SendMessageW(hwnd, WM_CLOSE, None, None);
                } else {
                    command(hwnd, IDCANCEL.0);
                }
            })
            .is_none());
        }
    }
}
#[test]
fn capture_affinity_dpi_keeps_draft_and_destruction_releases_font_and_user_pointer() {
    unsafe {
        let handles = Rc::new(RefCell::new(None));
        let capture = handles.clone();
        run(move |hwnd| {
            assert!(!IsWindowVisible(hwnd).as_bool());
            let mut affinity = 0;
            GetWindowDisplayAffinity(hwnd, &mut affinity).unwrap();
            assert_eq!(affinity, WDA_EXCLUDEFROMCAPTURE.0);
            let list = child(hwnd, PORTS);
            choose(hwnd, 0, 2);
            let mut rect = RECT::default();
            GetWindowRect(hwnd, &mut rect).unwrap();
            SendMessageW(
                hwnd,
                WM_DPICHANGED,
                Some(WPARAM(144 | (144 << 16))),
                Some(LPARAM(&rect as *const _ as isize)),
            );
            assert_eq!(child(hwnd, PORTS), list);
            assert_eq!(selected(hwnd, MODE, CB_GETCURSEL), Some(2));
            *capture.borrow_mut() = Some((hwnd, crate::desktop::tool_panel::font::object(hwnd)));
        });
        let (hwnd, font) = handles.borrow().unwrap();
        assert!(!IsWindow(Some(hwnd)).as_bool());
        let mut descriptor = LOGFONTW::default();
        assert_eq!(
            GetObjectW(
                font,
                std::mem::size_of::<LOGFONTW>() as i32,
                Some((&mut descriptor as *mut LOGFONTW).cast())
            ),
            0
        );
    }
}
#[test]
fn model_overflow_invalid_mode_and_missing_row_preserve_the_previous_candidate() {
    let mut visibility = Visibility::default();
    for i in 0..crate::workspace::ports::MAX_PORTS {
        visibility = visibility.edited(&format!("p{i}"), Some(true)).unwrap();
    }
    let mut model = Model {
        rows: vec![model::Row {
            name: "extra".into(),
            label: "输入 · extra".into(),
            default_visible: false,
        }],
        visibility: visibility.clone(),
    };
    assert!(model.edit(0, 2).is_err());
    assert!(model.edit(0, 3).is_err());
    assert!(model.edit(1, 0).is_err());
    assert_eq!(model.visibility, visibility);
    model.edit(0, 0).unwrap();
    assert_eq!(model.visibility, visibility);
}
