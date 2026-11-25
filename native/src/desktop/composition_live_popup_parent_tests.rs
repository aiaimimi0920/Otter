//! Reuse Family's owned-window fixtures to verify nearest-parent relationships between source menus and mirrors.
use super::*;

#[test]
fn owned_submenu_uses_nearest_parent_mirror_not_main_or_insertion_order() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let bounds = rect(100, 100, 80, 60);
    let root = Source::new(None, bounds, true);
    let first = Source::new(Some(root.0), bounds, true);
    let second = Source::new(Some(first.0), bounds, true);
    let third = Source::new(Some(second.0), bounds, true);
    let b1 = Source::new(None, bounds, true);
    let b2 = Source::new(Some(b1.0), bounds, true);
    let owner1 = crate::live_hover_owner::Owner::new(b1.0).unwrap();
    let owner2 = crate::live_hover_owner::Owner::new(b2.0).unwrap();
    owner1.control(true).unwrap();
    owner2.control(true).unwrap();
    let target = Target::bind(root.0).unwrap();
    let scope1 = Scope::bind(&target, None, first.0).unwrap();
    let scope2 = Scope::bind(&target, None, second.0).unwrap();
    let parents = [
        (&scope1, Some(owner1.identity())),
        (&scope2, Some(owner2.identity())),
    ];
    for popup in [&first, &second, &third] {
        popup.show();
    }
    let scopes = discover(&target, None, &parents, &[]).unwrap();
    let nested = scopes
        .iter()
        .find(|s| s.target().unwrap().hwnd() == third.0)
        .unwrap();
    assert_eq!(nested.mirror_owner(), Some(b2.0));
    let frames = crate::live_frames::Source::new();
    let mirror = LiveWindow::new(frames.subscribe().unwrap(), 40, 40).unwrap();
    mirror.configure_popup(b1.0, true).unwrap();
    mirror
        .reparent_popup(nested.mirror_owner().unwrap())
        .unwrap();
    assert_eq!(
        unsafe { GetWindow(mirror.popup_hwnd(), GW_OWNER).unwrap() },
        b2.0
    );
    owner2.control(false).unwrap();
    assert!(nested.target().is_err());
}
