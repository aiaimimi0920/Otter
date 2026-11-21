//! 原来源跨线程传递与清理权限：真实源 hook 建立 lease，不把控制撤销等同显示实例销毁。
use super::*;
use crate::live_window_target::recovery::popup_layer::fixture::Fixture;

struct Window(HWND);
impl Window {
    fn new(owner: Option<HWND>) -> Self {
        Self(unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                windows::core::w!("STATIC"),
                windows::core::w!(""),
                WS_POPUP,
                -30000,
                -30000,
                20,
                20,
                owner,
                None,
                None,
                None,
            )
            .unwrap()
        })
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.0).unwrap();
        }
    }
}

fn leased(fixture: &Fixture, hwnd: HWND) -> Scope {
    Scope::bind(
        &Target::bind(fixture.root.0).unwrap(),
        Some(fixture.owner.identity()),
        hwnd,
    )
    .unwrap()
}

#[test]
fn transferring_to_a_worker_cannot_rebind_a_new_display_on_the_same_hwnd() {
    let fixture = Fixture::new();
    let first = fixture.show();
    let transfer = leased(&fixture, fixture.popup.0).transfer();
    let second = fixture.show();
    assert_ne!(first.instance, second.instance);
    assert!(leased(&fixture, fixture.popup.0).target().is_ok());
    let instance = first.instance;
    std::thread::spawn(move || {
        let original = transfer.into_scope();
        assert!(original.target().is_err());
        assert!(original.cleanup_target().is_err());
        match original {
            Scope::Leased { lease, .. } => assert_eq!(lease.instance, instance),
            _ => panic!("original lease kind must not change"),
        }
    })
    .join()
    .unwrap();
}

#[test]
fn revoked_control_and_a_new_epoch_keep_only_the_original_instance_cleanup() {
    let fixture = Fixture::new();
    fixture.show();
    let original = leased(&fixture, fixture.popup.0).transfer().into_scope();
    fixture.owner.control(false).unwrap();
    assert!(original.target().is_err());
    assert_eq!(original.cleanup_target().unwrap().hwnd(), fixture.popup.0);
    fixture.owner.control(true).unwrap();
    assert!(original.target().is_err());
    assert!(original.cleanup_target().is_ok());
    fixture.show();
    assert!(original.cleanup_target().is_err());
}

#[test]
fn a_chained_parent_epoch_revokes_input_without_erasing_same_instance_cleanup() {
    let fixture = Fixture::new();
    fixture.show();
    let parent_b = Window::new(None);
    let child = Window::new(Some(fixture.popup.0));
    let owner = crate::live_hover_owner::Owner::new(parent_b.0).unwrap();
    owner.control(true).unwrap();
    let chained = leased(&fixture, fixture.popup.0)
        .bind_descendant(Some(owner.identity()), child.0)
        .unwrap()
        .transfer()
        .into_scope();
    assert!(chained.target().is_ok());
    owner.control(false).unwrap();
    assert!(chained.target().is_err());
    assert_eq!(chained.cleanup_target().unwrap().hwnd(), child.0);
    fixture.show();
    assert!(chained.cleanup_target().is_err());
}

#[test]
fn transferred_owned_scopes_still_require_the_original_owner_chain_for_cleanup() {
    let fixture = Fixture::new();
    let popup = Window::new(Some(fixture.root.0));
    let original = Scope::bind(&Target::bind(fixture.root.0).unwrap(), None, popup.0)
        .unwrap()
        .transfer();
    let original = std::thread::spawn(move || {
        let scope = original.into_scope();
        assert!(scope.target().is_ok());
        assert!(scope.cleanup_target().is_ok());
        scope.transfer()
    })
    .join()
    .unwrap()
    .into_scope();
    unsafe {
        SetWindowLongPtrW(popup.0, GWLP_HWNDPARENT, 0);
    }
    assert!(original.target().is_err());
    assert!(original.cleanup_target().is_err());
}
