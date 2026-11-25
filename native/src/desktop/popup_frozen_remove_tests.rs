//! Two synthetic libraries exercise actual menu-update callbacks and the original Application worker; no user libraries are accessed.
use super::*;
use windows::Win32::Foundation::{HWND, POINT};

pub(in crate::desktop) fn exercise(
    hwnd: HWND,
    source: Source,
    library: usize,
    count: usize,
    remove: &mut super::super::Remove<'_>,
) {
    let mut tree = Vec::new();
    let frozen = LibraryMenu::attach(&mut tree, Some(source), Workers::default());
    let root = frozen.roots[library];
    let mut contents = super::super::Contents {
        tree: Vec::new(),
        history: None,
        frozen: Some(frozen),
    };
    let mut menu = Cascade::open(hwnd, tree, POINT { x: 30, y: 30 }).unwrap();
    for _ in 0..=library {
        menu.key(0x28).unwrap();
    }
    menu.key(0x0d).unwrap();
    let settle = |contents: &mut super::super::Contents,
                  menu: &mut Cascade,
                  remove: &mut super::super::Remove<'_>| {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            contents.update(menu, Some(remove)).unwrap();
            let frozen = contents.frozen.as_ref().unwrap();
            assert!(!menu.closed() && menu.submenu_open(root));
            assert!(menu.take_command().is_none());
            if frozen.data.is_some() && !frozen.removing {
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    };
    settle(&mut contents, &mut menu, remove);
    for remaining in (count.saturating_sub(3)..count).rev() {
        menu.key(0x26).unwrap();
        contents.update(&mut menu, Some(remove)).unwrap();
        let before = menu.gallery_view(root).unwrap();
        let previous = contents.frozen.as_ref().unwrap().source.clone().unwrap();
        menu.key(0x2e).unwrap();
        contents.update(&mut menu, Some(remove)).unwrap();
        assert!(contents.frozen.as_ref().unwrap().removing);
        assert!(contents
            .frozen
            .as_ref()
            .unwrap()
            .take(REMOVE + library * CAPACITY)
            .is_none());
        settle(&mut contents, &mut menu, remove);
        let model = contents.frozen.as_ref().unwrap();
        assert_eq!(model.entries(library).unwrap().len(), remaining);
        assert!(!model.source.as_ref().unwrap().same_storage(&previous));
        if remaining > 0 {
            let choice = model.take(REMOVE + library * CAPACITY).unwrap();
            assert!(choice.source.same_storage(model.source.as_ref().unwrap()));
            let after = menu.gallery_view(root).unwrap();
            assert!(after.start <= before.start);
            if before.start > 0 {
                assert!(after.start > 0);
            }
        }
    }
}
