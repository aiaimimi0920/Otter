//! 隐藏工作区链的真实菜单请求：过期拒绝、应用层断开和显示回退。
use super::Application;
use crate::desktop::link_menu;
use windows::Win32::UI::WindowsAndMessaging::*;
pub(super) fn verify(app: &mut Application) {
    let incoming = app.workspace.links()[0].clone();
    let outgoing = app.workspace.links()[1].clone();
    let doc = &app.documents[2];
    let old = doc.state.link_menu.borrow().as_ref().unwrap().clone();
    unsafe {
        let menu = CreatePopupMenu().unwrap();
        assert!(link_menu::append(menu, &doc.state).unwrap().is_some());
        let pages = GetSubMenu(menu, 0);
        assert_eq!(GetMenuItemCount(Some(GetSubMenu(pages, 0))), 2);
        DestroyMenu(menu).unwrap();
        doc.state.group_locked.set(true);
        assert!(!old
            .queue(doc.window.0, &doc.state, link_menu::FIRST)
            .unwrap());
        doc.state.group_locked.set(false);
        assert!(old
            .queue(doc.window.0, &doc.state, link_menu::FIRST)
            .unwrap());
        assert!(app.workspace.remove_link(&outgoing.id));
        app.pending_windows(false).unwrap();
        assert!(
            app.workspace.links().contains(&incoming),
            "stale topology must reject removal"
        );
        app.workspace.connect(outgoing).unwrap();
        app.sync_display_frames();
        let doc = &app.documents[2];
        assert!(!old
            .queue(doc.window.0, &doc.state, link_menu::FIRST)
            .unwrap());
        let current = doc.state.link_menu.borrow().as_ref().unwrap().clone();
        assert!(current
            .queue(doc.window.0, &doc.state, link_menu::FIRST)
            .unwrap());
        app.pending_windows(false).unwrap();
    }
    assert!(!app
        .workspace
        .links()
        .iter()
        .any(|link| link.id == incoming.id));
    assert!(app.documents[2].state.resolved_display.borrow().is_none());
    assert!(app.documents[2].state.link_request.borrow().is_none());
    let doc = &app.documents[2];
    let catalog = doc.state.link_catalog.borrow().as_ref().unwrap().clone();
    assert!(app.documents.iter().all(|doc| std::rc::Rc::ptr_eq(
        doc.state.link_catalog.borrow().as_ref().unwrap(),
        &catalog
    )));
    let source = catalog
        .endpoints
        .iter()
        .position(|e| e.unit == incoming.from_unit)
        .unwrap();
    let target = catalog
        .endpoints
        .iter()
        .position(|e| e.unit == incoming.to_unit)
        .unwrap();
    let output = catalog.endpoints[source]
        .outputs
        .iter()
        .position(|p| p == &incoming.from_port)
        .unwrap();
    let input = catalog.endpoints[target]
        .inputs
        .iter()
        .position(|p| p == &incoming.to_port)
        .unwrap();
    unsafe {
        crate::desktop::link_create::tests::verify(catalog.clone(), target);
        assert!(!catalog
            .queue(doc.window.0, &doc.state, usize::MAX, output, input)
            .unwrap());
        assert!(!catalog
            .queue(doc.window.0, &doc.state, source, usize::MAX, input)
            .unwrap());
        assert!(!catalog
            .queue(doc.window.0, &doc.state, target, 0, input)
            .unwrap());
        assert!(catalog
            .queue(doc.window.0, &doc.state, source, output, input)
            .unwrap());
        assert!(!catalog
            .queue(doc.window.0, &doc.state, source, output, input)
            .unwrap());
    }
    // 入队后改变拓扑：即使端点仍然存在，也不能把旧目录应用到新工作区。
    let outgoing = app.workspace.links()[0].clone();
    assert!(app.workspace.remove_link(&outgoing.id));
    app.workspace.connect(outgoing).unwrap();
    unsafe {
        app.pending_windows(false).unwrap();
    }
    assert!(!app
        .workspace
        .links()
        .iter()
        .any(|link| link.to_unit == incoming.to_unit));
    let doc = &app.documents[2];
    let current = doc.state.link_catalog.borrow().as_ref().unwrap().clone();
    unsafe {
        assert!(!catalog
            .queue(doc.window.0, &doc.state, source, output, input)
            .unwrap());
        assert!(current
            .queue(doc.window.0, &doc.state, source, output, input)
            .unwrap());
    }
    unsafe {
        app.pending_windows(false).unwrap();
    }
    assert!(app.documents[2].state.link_creation.borrow().is_none());
    assert!(app
        .workspace
        .links()
        .iter()
        .any(|link| link.from_unit == incoming.from_unit
            && link.from_port == incoming.from_port
            && link.to_unit == incoming.to_unit
            && link.to_port == incoming.to_port));
    app.sync_display_frames();
    assert!(app.documents[2].state.resolved_display.borrow().is_some());
    let before = app.workspace.links().to_vec();
    let doc = &app.documents[0];
    let catalog = doc.state.link_catalog.borrow().as_ref().unwrap().clone();
    let downstream = catalog
        .endpoints
        .iter()
        .position(|e| e.unit == app.documents[1].state.unit_id)
        .unwrap();
    unsafe {
        assert!(catalog
            .queue(doc.window.0, &doc.state, downstream, 0, 0)
            .unwrap());
        app.pending_windows(false).unwrap();
    }
    assert_eq!(
        app.workspace.links(),
        before,
        "cycle rejection must preserve every existing edge"
    );
    let doc = &app.documents[2];
    let catalog = doc.state.link_catalog.borrow().as_ref().unwrap().clone();
    unsafe {
        assert!(catalog
            .queue(doc.window.0, &doc.state, source, output, input)
            .unwrap());
        app.pending_windows(false).unwrap();
    }
    assert_eq!(
        app.workspace.links().len(),
        before.len(),
        "connecting an occupied input replaces its edge"
    );
    let replaced = app
        .workspace
        .links()
        .iter()
        .find(|link| link.to_unit == incoming.to_unit)
        .unwrap();
    assert!(!before.iter().any(|link| link.id == replaced.id));
    assert_eq!(replaced.from_unit, incoming.from_unit);
    assert_eq!(replaced.to_port, incoming.to_port);
    assert!(app.documents[2].state.resolved_display.borrow().is_some());
}
