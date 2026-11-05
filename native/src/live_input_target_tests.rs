use super::*;
use windows::core::w;
struct Window(HWND);
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}
fn window(parent: Option<HWND>) -> Window {
    Window(
        unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!(""),
                if parent.is_some() { WS_CHILD } else { WS_POPUP },
                0,
                0,
                16,
                16,
                parent,
                None,
                None,
                None,
            )
        }
        .unwrap(),
    )
}
#[test]
#[ignore = "requires interactive default desktop; creates only owned hidden windows, no input"]
fn root_child_clone_reparent_destroy_and_foreign_root_are_checked() {
    let root = window(None);
    let other = window(None);
    let child = window(Some(root.0));
    let target = Target::bind(root.0).unwrap();
    assert_eq!(
        Destination::bind(&target, other.0).err(),
        Some(Error::NotDescendant)
    );
    let root_destination = Destination::bind(&target, root.0).unwrap();
    assert_eq!(root_destination.validate(), Ok(()));
    let destination = Destination::bind(&target, child.0).unwrap();
    let clone = destination.clone();
    assert_eq!(Arc::strong_count(&destination.process), 2);
    assert_eq!(destination.preflight(), Ok(()));
    unsafe {
        SetParent(child.0, Some(other.0)).unwrap();
    }
    assert_eq!(destination.validate(), Err(Error::NotDescendant));
    unsafe {
        SetParent(child.0, Some(root.0)).unwrap();
    }
    assert_eq!(clone.validate(), Ok(()));
    drop(child);
    assert_eq!(destination.validate(), Err(Error::TargetChanged));
    drop(destination);
    assert_eq!(Arc::strong_count(&clone.process), 1);
    drop(root);
    assert_eq!(root_destination.validate(), Err(Error::SourceChanged));
}
