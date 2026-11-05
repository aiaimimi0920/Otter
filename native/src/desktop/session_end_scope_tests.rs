//! 回调词法生命周期、身份和重入；无需系统退出或全局输入。
use super::*;
#[test]
fn weak_owner_is_lexical_and_freezes_input_without_blocking_cleanup_or_reads() {
    let owner = Rc::new(Control::default());
    assert!(active().is_none());
    with_owner(&owner, || {
        assert!(Rc::ptr_eq(&active().unwrap(), &owner));
        assert!(!ready(&owner), "weak owner is not a persistence permission");
        owner.0.set(Phase::Queried);
        for message in [
            WM_CHAR,
            WM_SETTEXT,
            WM_COMMAND,
            WM_LBUTTONDOWN,
            WM_IME_COMPOSITION,
            windows::Win32::UI::Controls::EM_REPLACESEL,
            CB_SETCURSEL,
        ] {
            assert!(blocks_input(message));
        }
        for message in [
            WM_DESTROY,
            WM_NCDESTROY,
            WM_GETTEXT,
            WM_CANCELMODE,
            WM_CAPTURECHANGED,
            WM_GESTURE,
            WM_TOUCH,
        ] {
            assert!(!blocks_input(message));
        }
        let other = Rc::new(Control::default());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            with_owner(&other, || {
                assert!(!blocks_input(WM_CHAR));
                panic!("owner scope fixture");
            });
        }));
        assert!(result.is_err());
        assert!(blocks_input(WM_CHAR));
    });
    assert!(active().is_none());
    assert!(!blocks_input(WM_CHAR));
}
#[test]
fn query_is_scoped_to_owner_and_cannot_escape_or_reenter() {
    let owner = Rc::new(Control::default());
    let other = Rc::new(Control::default());
    assert!(!ready(&owner));
    with_query(
        &owner,
        || {
            assert!(!ready(&owner));
            true
        },
        || {
            assert!(ready(&owner));
            assert!(!ready(&other));
        },
    );
    assert!(!ready(&owner));
}
#[test]
fn nested_scope_restores_previous_callback_and_unwind_retires_stack_pointer() {
    let owner = Rc::new(Control::default());
    with_query(
        &owner,
        || true,
        || {
            with_query(&owner, || false, || assert!(!ready(&owner)));
            assert!(ready(&owner));
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                with_query(&owner, || false, || panic!("owned scope unwind fixture"));
            }));
            assert!(result.is_err());
            assert!(ready(&owner));
        },
    );
    assert!(!ready(&owner));
}
