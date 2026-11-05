//! 真实 HMENU 的分页、命令索引与生命周期，不打开阻塞式可见菜单。
use super::*;
#[test]
fn disconnect_menu_pages_preserve_snapshot_indices_and_disabled_state() {
    unsafe {
        let unit = UnitId::parse("source").unwrap();
        let workspace = Workspace::with_initial(unit.clone());
        let snapshot = Snapshot {
            topology: workspace.topology_stamp(),
            links: (0..65)
                .map(|i| Link {
                    id: UnitId::parse(&format!("edge-{i}")).unwrap(),
                    from_unit: unit.clone(),
                    from_port: "output".into(),
                    to_unit: UnitId::parse(&format!("target-{i}")).unwrap(),
                    to_port: "image".into(),
                })
                .collect(),
        };
        for locked in [false, true] {
            let root = Menu(CreatePopupMenu().unwrap());
            snapshot.append(root.0, locked).unwrap();
            let pages = GetSubMenu(root.0, 0);
            assert_eq!(GetMenuItemCount(Some(pages)), 2);
            for (page, count) in [(0, 64), (1, 1)] {
                let child = GetSubMenu(pages, page);
                assert_eq!(GetMenuItemCount(Some(child)), count);
                for offset in 0..count {
                    let command = GetMenuItemID(child, offset) as usize;
                    assert_eq!(command, FIRST + page as usize * 64 + offset as usize);
                    let request = snapshot.request(command).unwrap();
                    assert_eq!(request.link, snapshot.links[command - FIRST]);
                    assert!(request.topology.matches(&workspace));
                    assert_eq!(
                        GetMenuState(child, offset as u32, MF_BYPOSITION) & MF_GRAYED.0 != 0,
                        locked
                    );
                }
            }
        }
        for command in [0, FIRST - 1, FIRST + 65, usize::MAX] {
            assert!(snapshot.request(command).is_none());
        }
        let empty = Snapshot::capture(&workspace, &unit);
        assert!(empty.select(HWND::default()).unwrap().is_none());
    }
}
