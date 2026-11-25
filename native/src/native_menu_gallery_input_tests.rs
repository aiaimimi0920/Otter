//! 真实 HWND 消息路由、删除目标隔离与悬浮窗回收；只使用合成条目。
use super::*;
fn wheel(menu: &Cascade, slot: usize, delta: i16) -> MSG {
    let (x, y) = center(menu, slot);
    let page = &menu.pages[1];
    MSG {
        hwnd: menu.pages[0].panel.handle().unwrap(),
        message: WM_MOUSEWHEEL,
        wParam: WPARAM((delta as u16 as usize) << 16),
        lParam: LPARAM(
            (((page.bounds.top + y) as u16 as u32) << 16 | (page.bounds.left + x) as u16 as u32)
                as isize,
        ),
        ..Default::default()
    }
}
#[test]
fn sent_root_owner_and_queued_wheels_target_the_gallery_under_the_cursor() {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetCapture, ReleaseCapture, SetCapture};
    let (owner, mut menu) = gallery(64);
    let root = menu.pages[0].panel.handle().unwrap();
    let guard = menu.pages[0].panel.forward_owner_wheel(owner.0).unwrap();
    unsafe {
        SetCapture(root);
        assert_eq!(GetCapture(), root);
    }
    for target in [root, owner.0] {
        let msg = wheel(&menu, 4, -120);
        let previous = menu.pages[1].first;
        unsafe {
            SendMessageW(target, msg.message, Some(msg.wParam), Some(msg.lParam));
        }
        menu.poll().unwrap();
        assert_eq!(menu.pages.len(), 2);
        assert_eq!(menu.pages[0].first, 0);
        assert!(menu.pages[1].first > previous);
        assert_eq!(menu.pages[1].first % 3, 0);
    }
    let msg = wheel(&menu, 4, -120);
    unsafe {
        PostMessageW(Some(root), msg.message, msg.wParam, msg.lParam).unwrap();
    }
    let mut queued = MSG::default();
    assert!(unsafe {
        PeekMessageW(
            &mut queued,
            Some(root),
            WM_MOUSEWHEEL,
            WM_MOUSEWHEEL,
            PM_REMOVE,
        )
    }
    .as_bool());
    let previous = menu.pages[1].first;
    menu.pointer(&queued).unwrap();
    assert!(menu.pages[1].first > previous);
    unsafe {
        ReleaseCapture().unwrap();
    }
    drop(guard);
}
#[test]
fn system_hit_wheels_keep_scrolled_content_after_hover_and_refresh() {
    crate::desktop::workspace_window_fixture::with_private_desktop(system_hit_wheels);
}

fn system_hit_wheels() {
    let mut positions = Vec::new();
    for deltas in [vec![-120], vec![-30; 4]] {
        let (_owner, mut menu) = gallery(64);
        let (x, y) = center(&menu, 4);
        menu.event(1, Event::Move(x, y)).unwrap();
        for (step, delta) in deltas.iter().enumerate() {
            let message = wheel(&menu, 4, *delta);
            let page = &menu.pages[1];
            let target = unsafe {
                WindowFromPoint(POINT {
                    x: page.bounds.left + x,
                    y: page.bounds.top + y,
                })
            };
            assert_eq!(target, page.panel.handle().unwrap());
            unsafe {
                SendMessageW(
                    target,
                    message.message,
                    Some(message.wParam),
                    Some(message.lParam),
                );
            }
            menu.poll().unwrap();
            if step + 1 < deltas.len() {
                assert_eq!(menu.pages[1].first, 0);
            }
        }
        let first = menu.pages[1].first;
        assert!(first > 0);
        positions.push(first);
        menu.event(1, Event::Move(x, y)).unwrap();
        menu.replace_gallery(0, (0..64).map(|n| item(100 + n)).collect())
            .unwrap();
        menu.event(1, Event::Move(x + 1, y)).unwrap();
        assert_eq!(menu.pages[1].first, first);
        assert_eq!(menu.pages[1].focus, Some(first + 4));
        let visible = menu.pages[1].panel.accessibility_nodes();
        assert_eq!(
            visible[0].read().unwrap().name,
            format!("命令 {}", 100 + first)
        );
        unsafe {
            UpdateWindow(menu.pages[1].panel.handle().unwrap())
                .ok()
                .unwrap();
        }
        menu.event(1, Event::Down(x, y)).unwrap();
        menu.event(1, Event::Up(x, y)).unwrap();
        assert_eq!(menu.take_command(), Some(100 + first + 4));
    }
    assert_eq!(positions[0], positions[1]);
}
#[test]
fn delete_requires_the_same_corner_and_is_retired_by_scrolling_or_replacement() {
    let (_owner, mut menu) = gallery(64);
    let rows = || {
        (0..64)
            .map(|n| {
                let mut row = item(100 + n);
                row.delete = Some(200 + n);
                row
            })
            .collect()
    };
    menu.replace_gallery(0, rows()).unwrap();
    let corner = menu.pages[1].layout.delete_rect(0, menu.content_width(1));
    let (x, y) = (
        (corner.left + corner.right) / 2,
        (corner.top + corner.bottom) / 2,
    );
    let (body_x, body_y) = center(&menu, 0);
    for (down, up) in [((x, y), (body_x, body_y)), ((body_x, body_y), (x, y))] {
        menu.event(1, Event::Down(down.0, down.1)).unwrap();
        menu.event(1, Event::Up(up.0, up.1)).unwrap();
        assert!(menu.take_command().is_none());
        assert!(menu.take_delete().is_none());
    }
    menu.event(1, Event::Down(x, y)).unwrap();
    menu.event(1, Event::Up(x, y)).unwrap();
    assert_eq!(menu.take_delete(), Some(200));
    assert!(menu.take_delete().is_none());
    assert!(menu.take_command().is_none());
    menu.event(1, Event::Down(x, y)).unwrap();
    menu.event(1, Event::Wheel(-120)).unwrap();
    menu.event(1, Event::Up(x, y)).unwrap();
    assert!(menu.take_delete().is_none());
    let first = menu.pages[1].first;
    menu.event(1, Event::Down(x, y)).unwrap();
    menu.replace_gallery(0, rows()).unwrap();
    assert_eq!(menu.pages[1].first, first);
    menu.event(1, Event::Up(x, y)).unwrap();
    assert!(menu.take_delete().is_none());
}
#[test]
fn consecutive_corner_deletes_keep_the_same_panels_and_the_empty_submenu_open() {
    let (_owner, mut menu) = gallery(3);
    let root = menu.pages[0].panel.handle().unwrap();
    let child = menu.pages[1].panel.handle().unwrap();
    for first in 0..3 {
        menu.replace_gallery(
            0,
            (first..3)
                .map(|n| {
                    let mut row = item(100 + n);
                    row.delete = Some(200 + n);
                    row
                })
                .collect(),
        )
        .unwrap();
        let corner = menu.pages[1].layout.delete_rect(0, menu.content_width(1));
        let (x, y) = (
            (corner.left + corner.right) / 2,
            (corner.top + corner.bottom) / 2,
        );
        menu.event(1, Event::Down(x, y)).unwrap();
        menu.event(1, Event::Up(x, y)).unwrap();
        assert_eq!(menu.take_delete(), Some(200 + first));
        assert!(menu.take_command().is_none());
        assert!(menu.submenu_open(0));
        assert!(!menu.closed());
        assert_eq!(menu.pages[0].panel.handle().unwrap(), root);
        assert_eq!(menu.pages[1].panel.handle().unwrap(), child);
    }
    let mut empty = item(0);
    empty.enabled = false;
    empty.label = "没有内容".into();
    menu.replace_submenu(0, vec![empty]).unwrap();
    assert!(menu.submenu_open(0) && !menu.closed());
    assert_eq!(menu.pages[0].panel.handle().unwrap(), root);
}
#[test]
fn hovering_previews_both_types_reuses_window_and_closes_it_with_the_menu() {
    let (_owner, mut menu) = gallery(2);
    let mut text = item(101);
    text.preview = Some(crate::native_menu::panel::Preview::Text(
        "第一行\n第二行 & 内容".into(),
    ));
    let mut image = item(102);
    image.preview = Some(crate::native_menu::panel::Preview::Image(
        crate::image_decode::Image {
            width: 2,
            height: 1,
            pixels: vec![0, 0, 255, 255, 0, 255, 0, 255].into(),
        },
    ));
    menu.replace_gallery(0, vec![text, image]).unwrap();
    let (x, y) = center(&menu, 0);
    menu.event(1, Event::Move(x, y)).unwrap();
    let first = menu.preview.as_ref().unwrap().panel.handle().unwrap();
    assert_eq!(menu.pages.len(), 2);
    assert!(
        menu.preview.as_ref().unwrap().bounds.left >= menu.pages[1].bounds.right
            || menu.preview.as_ref().unwrap().bounds.right <= menu.pages[1].bounds.left
    );
    unsafe {
        UpdateWindow(first).ok().unwrap();
    }
    let (x, y) = center(&menu, 1);
    menu.event(1, Event::Move(x, y)).unwrap();
    let second = menu.preview.as_ref().unwrap().panel.handle().unwrap();
    assert_eq!(first, second);
    assert!(unsafe { IsWindow(Some(first)) }.as_bool());
    assert_eq!(menu.preview.as_ref().unwrap().index, 1);
    unsafe {
        UpdateWindow(second).ok().unwrap();
    }
    menu.event(1, Event::Down(x, y)).unwrap();
    menu.event(1, Event::Up(x, y)).unwrap();
    assert_eq!(menu.take_command(), Some(102));
    assert!(!unsafe { IsWindow(Some(second)) }.as_bool());
}
#[test]
fn preview_releases_and_direct_messages_cannot_reuse_a_gallery_delete_gesture() {
    let (_owner, mut menu) = gallery(1);
    let mut row = item(101);
    row.delete = Some(201);
    row.preview = Some(crate::native_menu::panel::Preview::Text("preview".into()));
    menu.replace_gallery(0, vec![row]).unwrap();
    let (body_x, body_y) = center(&menu, 0);
    menu.event(1, Event::Move(body_x, body_y)).unwrap();
    let preview = menu.preview.as_ref().unwrap().panel.handle().unwrap();
    let corner = menu.pages[1].layout.delete_rect(0, menu.content_width(1));
    let (x, y) = (corner.left + 2, corner.top + 2);
    menu.event(1, Event::Down(x, y)).unwrap();
    for message in [WM_LBUTTONUP, WM_LBUTTONDOWN] {
        menu.pointer(&MSG {
            hwnd: preview,
            message,
            lParam: LPARAM(10 | (10 << 16)),
            ..Default::default()
        })
        .unwrap();
    }
    menu.event(1, Event::Up(x, y)).unwrap();
    assert!(menu.take_delete().is_none());
    for message in [WM_LBUTTONDOWN, WM_LBUTTONUP] {
        unsafe {
            SendMessageW(preview, message, None, Some(LPARAM((y << 16 | x) as isize)));
        }
    }
    menu.poll().unwrap();
    assert!(menu.take_delete().is_none());
    assert!(menu.take_command().is_none());
    menu.event(1, Event::Key(0x2e)).unwrap();
    assert_eq!(menu.take_delete(), Some(201));
}

#[test]
fn preview_wheel_and_snapshot_replacement_keep_window_and_refresh_same_label_content() {
    let (_owner, mut menu) = gallery(64);
    let rows = |suffix: &str| {
        (0..64)
            .map(|n| {
                let mut row = item(100 + n);
                row.preview = Some(crate::native_menu::panel::Preview::Text(
                    format!("{n}-{suffix}").into(),
                ));
                row
            })
            .collect()
    };
    menu.replace_gallery(0, rows("before")).unwrap();
    let (x, y) = center(&menu, 4);
    menu.event(1, Event::Move(x, y)).unwrap();
    let window = menu.preview.as_ref().unwrap().panel.handle().unwrap();
    let bounds = menu.preview.as_ref().unwrap().bounds;
    let message = wheel(&menu, 4, -120);
    menu.pointer(&message).unwrap();
    let index = menu.pages[1].first + 4;
    assert!(index > 4);
    assert_eq!(menu.preview.as_ref().unwrap().index, index);
    menu.replace_gallery(0, rows("after")).unwrap();
    let view = menu.preview.as_ref().unwrap();
    assert_eq!(view.panel.handle().unwrap(), window);
    assert_eq!(view.bounds, bounds);
    let Some(crate::native_menu::panel::Preview::Text(text)) = view.panel.preview_content() else {
        panic!("text preview")
    };
    assert_eq!(&*text, format!("{index}-after"));
    unsafe {
        UpdateWindow(window).ok().unwrap();
    }
    for _ in 0..10 {
        menu.event(1, Event::Move(x, y)).unwrap();
    }
    assert!(!unsafe { GetUpdateRect(window, None, false) }.as_bool());
    assert_eq!(
        menu.preview.as_ref().unwrap().panel.handle().unwrap(),
        window
    );
    menu.cancel();
    assert!(!unsafe { IsWindow(Some(window)) }.as_bool());
}

#[test]
fn delete_hover_only_tracks_corner_and_clears_on_scrollbar_and_keyboard() {
    let (_owner, mut menu) = gallery(64);
    let rows = (0..64)
        .map(|n| {
            let mut row = item(100 + n);
            row.delete = Some(200 + n);
            row
        })
        .collect();
    menu.replace_gallery(0, rows).unwrap();
    let corner = menu.pages[1].layout.delete_rect(0, menu.content_width(1));
    let (x, y) = (corner.left + 2, corner.top + 2);
    let (body_x, body_y) = center(&menu, 0);
    menu.event(1, Event::Move(body_x, body_y)).unwrap();
    assert_eq!(menu.pages[1].panel.delete_hover(), None);
    menu.event(1, Event::Move(x, y)).unwrap();
    assert_eq!(menu.pages[1].panel.delete_hover(), Some(0));
    assert!(menu.take_delete().is_none());
    let bar = menu.bar(1).unwrap();
    menu.event(1, Event::Move(bar.track.left + 1, bar.track.top + 1))
        .unwrap();
    assert_eq!(menu.pages[1].panel.delete_hover(), None);
    menu.event(1, Event::Move(x, y)).unwrap();
    menu.key(0x28).unwrap();
    assert_eq!(menu.pages[1].panel.delete_hover(), None);
}

#[test]
fn scrollbar_contact_retires_gallery_delete_gesture_without_prior_hover() {
    let (_owner, mut menu) = gallery(64);
    let rows = (0..64)
        .map(|n| {
            let mut row = item(100 + n);
            row.delete = Some(200 + n);
            row
        })
        .collect();
    menu.replace_gallery(0, rows).unwrap();
    let corner = menu.pages[1].layout.delete_rect(0, menu.content_width(1));
    let (x, y) = (corner.left + 2, corner.top + 2);
    let bar = menu.bar(1).unwrap();
    for event in [
        Event::Move(bar.track.left, bar.thumb.top),
        Event::Up(bar.track.left, bar.thumb.top),
        Event::Down(bar.track.left, bar.thumb.top),
    ] {
        menu.event(1, Event::Down(x, y)).unwrap();
        assert_eq!(menu.pages[1].delete_pressed, Some(0));
        menu.event(1, event).unwrap();
        assert_eq!(menu.pages[1].delete_pressed, None);
        menu.event(1, Event::Up(x, y)).unwrap();
        assert!(menu.take_delete().is_none());
    }
    menu.event(1, Event::Down(x, y)).unwrap();
    menu.event(1, Event::Up(x, y)).unwrap();
    assert_eq!(menu.take_delete(), Some(200));
}
