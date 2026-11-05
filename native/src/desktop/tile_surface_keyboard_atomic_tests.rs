//! 原生销毁/滚动paint重入不得把旧Tab候选写回；由未切换的独立desktop运行。
use super::*;
use windows::Win32::UI::Shell::*;

fn controls(kind: &str) -> Value {
    json!({"id":"root","type":"column","layout":{"width":64,"height":64},"children":[
        {"id":"first","type":"input","layout":{"width":64,"height":20,"minHeight":20},"props":{"value":"source"}},
        {"id":"value","type":kind,"layout":{"width":64,"height":20,"minHeight":20},
        "props":{"value":if kind=="select" {json!("a")} else {json!(2)},"min":0,"max":10,"step":2,
        "options":[{"label":"A","value":"a"},{"label":"B","value":"b"}]},"events":{"change":"save"}},
        {"id":"send","type":"button","layout":{"width":64,"height":24,"minHeight":24},"events":{"click":"go"}}]})
}
struct Reentry<'a> {
    root: HWND,
    state: &'a State,
    successor: bool,
    trigger: u32,
    calls: std::cell::Cell<u32>,
}
unsafe extern "system" fn reenter(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    let entry = &*(data as *const Reentry<'_>);
    if msg == entry.trigger {
        let _ = RemoveWindowSubclass(hwnd, Some(reenter), 0x41544f4d);
        entry.calls.set(entry.calls.get() + 1);
        if entry.successor {
            navigate(entry.root, entry.state, true);
        } else {
            retire(entry.state);
        }
    }
    DefSubclassProc(hwnd, msg, wp, lp)
}
struct Hook<'a> {
    hwnd: HWND,
    _entry: std::marker::PhantomData<&'a Reentry<'a>>,
}
impl Drop for Hook<'_> {
    fn drop(&mut self) {
        unsafe {
            let _ = RemoveWindowSubclass(self.hwnd, Some(reenter), 0x41544f4d);
        }
    }
}
fn hook<'a>(hwnd: HWND, entry: &'a Reentry<'a>) -> Hook<'a> {
    unsafe {
        assert!(SetWindowSubclass(
            hwnd,
            Some(reenter),
            0x41544f4d,
            entry as *const Reentry<'_> as usize,
        )
        .as_bool());
    }
    Hook {
        hwnd,
        _entry: std::marker::PhantomData,
    }
}
#[test]
fn value_destruction_cancel_never_restores_the_old_tab_candidate() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for kind in ["select", "slider"] {
        let (f, w) = setup(controls(kind));
        navigate(w.hwnd, &w.state, false);
        navigate(w.hwnd, &w.state, false);
        assert_eq!(focus(&w), "value");
        let h = child(&w);
        let entry = Reentry {
            root: w.hwnd,
            state: &w.state,
            successor: false,
            trigger: WM_NCDESTROY,
            calls: 0.into(),
        };
        let _hook = hook(h, &entry);
        navigate(w.hwnd, &w.state, false);
        assert_eq!(entry.calls.get(), 1);
        assert!(w.state.keyboard.borrow().focus.is_none());
        assert!(w.state.keyboard_value.borrow().is_none());
        assert!(events(&f).is_empty());
        assert!(!unsafe { IsWindow(Some(h)) }.as_bool());
        assert_eq!(
            f.art.snapshot.scene,
            serde_json::to_vec(&controls(kind)).unwrap()
        );
    }
}
#[test]
fn value_destruction_successor_keeps_its_logical_focus_actual_edit_and_owner() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for kind in ["select", "slider"] {
        let (f, w) = setup(controls(kind));
        navigate(w.hwnd, &w.state, false);
        let first = text::active_child(&w.state);
        navigate(w.hwnd, &w.state, false);
        let h = child(&w);
        let entry = Reentry {
            root: w.hwnd,
            state: &w.state,
            successor: true,
            trigger: WM_NCDESTROY,
            calls: 0.into(),
        };
        let _hook = hook(h, &entry);
        navigate(w.hwnd, &w.state, false);
        assert_eq!(entry.calls.get(), 1);
        assert_eq!(focus(&w), "first");
        assert_eq!(text::active_child(&w.state), first);
        assert_eq!(unsafe { GetFocus() }, first);
        assert!(w.state.keyboard_value.borrow().is_none());
        assert!(unsafe { IsWindow(Some(first)) }.as_bool());
        assert!(events(&f).is_empty());
    }
}
#[test]
fn reveal_paint_cancel_does_not_republish_focus_after_a_successful_present() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (f, w) = setup(
        json!({"id":"root","type":"scroll","layout":{"width":64,"height":64},"children":[
        {"id":"first","type":"input","layout":{"width":64,"height":20,"minHeight":20},"props":{"value":"source"}},
        {"id":"gap","type":"view","layout":{"height":80,"minHeight":80}},
        {"id":"send","type":"button","layout":{"width":64,"height":20,"minHeight":20},"events":{"click":"go"}}]}),
    );
    navigate(w.hwnd, &w.state, false);
    let entry = Reentry {
        root: w.hwnd,
        state: &w.state,
        successor: false,
        trigger: WM_PAINT,
        calls: 0.into(),
    };
    let _hook = hook(w.hwnd, &entry);
    navigate(w.hwnd, &w.state, false);
    assert_eq!(entry.calls.get(), 1);
    assert!(actions::active(&w.state).is_some());
    assert!(w.state.keyboard.borrow().focus.is_none());
    assert!(w.state.keyboard_value.borrow().is_none());
    assert!(events(&f).is_empty());
}
