//! 多选意图依据刷新后的稳定身份派发，不因不可用目标阻塞或重复发送。
use super::*;

fn row(id: &str, checked: bool, command: usize) -> Entry {
    Entry {
        id: id.into(),
        name: id.into(),
        checked,
        enabled: true,
        command,
    }
}

#[test]
fn directory_loading_empty_failure_and_unnamed_device_are_visible() {
    use crate::projection_delivery::targets::Directory;
    let state = State::new(None, false).unwrap();
    assert_eq!(directory_status(&state), "目录不可用，请刷新");
    state.authorization.borrow_mut().targets.busy = true;
    assert_eq!(directory_status(&state), "正在加载…");
    {
        let mut view = state.authorization.borrow_mut();
        view.targets.busy = false;
        view.targets.status = "目标目录查询失败".into();
    }
    assert_eq!(directory_status(&state), "目标目录查询失败");
    state.authorization.borrow_mut().targets.directory = Some(Directory::parse(
        br#"{"targets":[{"deviceId":"code-device","name":"","policy":"confirm","route":"shared_loom"}],"groups":[],"peerDirectory":{"status":"complete"},"friends":{"users":[]}}"#,
        "https://loom.example",
    ).unwrap().into());
    assert_eq!(entries(&state, DEVICES)[0].name, "code-device");
    assert!(entries(&state, GROUPS).is_empty());
    assert_eq!(directory_status(&state), "暂无目标");
}

#[test]
fn directory_refresh_does_not_hide_a_failed_projection() {
    use crate::projection_delivery::targets::Directory;
    unsafe {
        let owner = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            windows::core::w!("STATIC"),
            None,
            WS_POPUP,
            100,
            100,
            300,
            200,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let state = State::new(None, false).unwrap();
        {
            let mut view = state.authorization.borrow_mut();
            view.origin = Some("https://loom.example".into());
            view.connected = true;
            view.sender.ready = true;
            view.targets.directory = Some(Directory::parse(
                br#"{"targets":[],"groups":[],"peerDirectory":{"status":"complete"},"friends":{"users":[]}}"#,
                "https://loom.example",
            ).unwrap().into());
        }
        let mut popup = Popup::open(
            owner,
            &state,
            DEVICES,
            RECT {
                left: 100,
                top: 100,
                right: 150,
                bottom: 130,
            },
        )
        .unwrap();
        popup.queue.push_back(("removed-device".into(), true));
        popup.applying = true;
        assert!(popup.poll(owner, &state).unwrap());
        let label = GetDlgItem(Some(popup.menu.hwnd()), 11).unwrap();
        let mut text = [0u16; 256];
        let n = GetWindowTextW(label, &mut text) as usize;
        let failure = String::from_utf16_lossy(&text[..n]);
        assert!(failure.starts_with("未完成："));
        for _ in 0..8 {
            popup.sync_directory(&state);
        }
        let n = GetWindowTextW(label, &mut text) as usize;
        assert_eq!(String::from_utf16_lossy(&text[..n]), failure);
        drop(popup);
        DestroyWindow(owner).unwrap();
    }
}

#[test]
fn unavailable_target_does_not_block_other_selections() {
    let rows = [row("offline", false, 10), row("online", false, 11)];
    let mut queue = VecDeque::from([("offline".into(), true), ("online".into(), true)]);
    assert_eq!(
        next_choice(&mut queue, &rows, |command| command == 11),
        Some(("online".into(), true, 11))
    );
    assert_eq!(queue, VecDeque::from([("offline".into(), true)]));
    assert_eq!(next_choice(&mut queue, &rows, |_| false), None);
    assert_eq!(queue.len(), 1);
}

#[test]
fn refresh_and_completion_use_current_identity_and_command() {
    let mut queue = VecDeque::from([
        ("removed".into(), true),
        ("complete".into(), true),
        ("active".into(), false),
    ]);
    let rows = [row("active", true, 99), row("complete", true, 98)];
    assert_eq!(
        next_choice(&mut queue, &rows, |_| true),
        Some(("active".into(), false, 99))
    );
    assert_eq!(queue, VecDeque::from([("removed".into(), true)]));
    // 解绑完成后重新勾选同一设备，使用当前目录的发送命令。
    queue.push_back(("active".into(), true));
    assert_eq!(
        next_choice(&mut queue, &[row("active", false, 42)], |_| true),
        Some(("active".into(), true, 42))
    );
}

#[test]
fn draft_refresh_reorders_by_id_and_confirmation_contains_only_actual_changes() {
    let rows = vec![row("a", true, 10), row("b", false, 11)];
    let mut draft = BTreeMap::new();
    merge_draft(&mut draft, &rows);
    draft.insert("a".into(), false);
    draft.insert("b".into(), true);
    let refreshed = vec![row("b", false, 30), row("a", true, 31), row("c", false, 32)];
    merge_draft(&mut draft, &refreshed);
    assert_eq!(
        changes(&draft, &refreshed),
        vec![("b".into(), true), ("a".into(), false)]
    );
    let before = draft.clone();
    select_all(&mut draft, &refreshed);
    select_all(&mut draft, &refreshed);
    assert!(draft.values().all(|checked| *checked));
    assert_ne!(before, draft);
    assert_eq!(
        changes(&draft, &refreshed),
        vec![("b".into(), true), ("c".into(), true)]
    );
}

#[test]
fn confirmed_group_keeps_exact_targets_across_edits_refresh_and_popup_close() {
    use crate::projection_delivery::{sender::binding::tests as saved, targets::Directory};
    use serde_json::json;
    let origin = "https://loom.example";
    let catalog = |ids: &[&str]| {
        Directory::parse(
            &serde_json::to_vec(&json!({
                "targets": ids.iter().map(|id| json!({"deviceId":id,"name":"Same name",
                    "policy":"confirm","route":"shared_loom"})).collect::<Vec<_>>(),
                "groups":[{"groupId":"all","name":"All","targetIds":ids,"unavailableCount":0}],
                "peerDirectory":{"status":"complete"},"friends":{"users":[]}
            }))
            .unwrap(),
            origin,
        )
        .unwrap()
        .into()
    };
    unsafe {
        let owner = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            windows::core::w!("STATIC"),
            None,
            WS_POPUP,
            100,
            100,
            300,
            200,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let state = State::new(None, false).unwrap();
        {
            let mut view = state.authorization.borrow_mut();
            view.origin = Some(origin.into());
            view.connected = true;
            view.sender.ready = true;
            view.targets.directory = Some(catalog(&["target-1", "target-2"]));
        }
        let anchor = RECT {
            left: 100,
            top: 100,
            right: 150,
            bottom: 130,
        };
        let mut abandoned = Popup::open(owner, &state, GROUPS, anchor).unwrap();
        select_all(&mut abandoned.draft, &abandoned.rows);
        drop(abandoned);
        assert!(state.authorization.borrow().request.is_none());
        let mut popup = Popup::open(owner, &state, GROUPS, anchor).unwrap();
        select_all(&mut popup.draft, &popup.rows);
        assert!(state.authorization.borrow().request.is_none());
        popup.confirm(&state);
        // 已确认后目录加入第三个目标，同时重排；原队列不得扩大或按旧索引串目标。
        state.authorization.borrow_mut().targets.directory =
            Some(catalog(&["target-2", "target-3", "target-1"]));
        {
            let mut canvas = state.canvas.borrow_mut();
            canvas.tool = crate::editor::Tool::Brush;
            canvas
                .editor
                .begin(
                    crate::editor::Tool::Brush,
                    crate::viewport::Point { x: 1., y: 1. },
                )
                .unwrap();
            assert!(canvas.editor.commit());
            assert!(!popup.target.matches(&canvas.editor));
        }
        assert!(popup.poll(owner, &state).unwrap());
        let Some(auth::Request::SendProjection(first)) =
            state.authorization.borrow_mut().request.take()
        else {
            panic!("first confirmed target was not dispatched");
        };
        assert_eq!(first.directory.targets()[first.index].id(), "target-1");
        // 在途关闭窗口只隐藏呈现；原 owner 确认结果后仍发送下一项。
        state.authorization.borrow_mut().sender.busy = true;
        assert!(popup.menu.translate(&MSG {
            hwnd: popup.menu.hwnd(),
            message: WM_KEYDOWN,
            wParam: WPARAM(27),
            ..Default::default()
        }));
        let mut message = MSG::default();
        while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
            DispatchMessageW(&message);
        }
        assert!(popup.menu.closed());
        assert!(popup.poll(owner, &state).unwrap());
        assert!(state.authorization.borrow().request.is_none());
        let scope = "a".repeat(64);
        let source = saved::source(state.unit_id.as_str(), &scope);
        let (_directory, _store, intent) = saved::signed(state.unit_id.as_str(), &scope, origin);
        let candidate = saved::candidate(&source, &intent, 1, intent.digest());
        {
            let mut view = state.authorization.borrow_mut();
            view.sender.busy = false;
            view.sender.bindings = vec![candidate.binding().clone()];
        }
        assert!(popup.poll(owner, &state).unwrap());
        let Some(auth::Request::SendProjection(second)) =
            state.authorization.borrow_mut().request.take()
        else {
            panic!("second confirmed target was not dispatched");
        };
        assert_eq!(second.directory.targets()[second.index].id(), "target-2");
        assert!(popup.queue.is_empty());
        // 文档本身换代仍须废弃批次，不能将选择套到另一张图。
        state
            .canvas
            .borrow_mut()
            .replace_image(crate::desktop::image::Image {
                width: 1,
                height: 1,
                pixels: vec![0, 0, 0, 255].into(),
            })
            .unwrap();
        assert!(!popup.poll(owner, &state).unwrap());
        assert!(state.authorization.borrow().request.is_none());
        drop(popup);
        DestroyWindow(owner).unwrap();
    }
}

pub(in crate::desktop) struct Confirmed(Popup);
impl Confirmed {
    pub(in crate::desktop) unsafe fn all(owner: HWND, state: &State) -> Self {
        let mut popup = Popup::open(
            owner,
            state,
            DEVICES,
            RECT {
                left: 200,
                top: 200,
                right: 250,
                bottom: 230,
            },
        )
        .unwrap();
        select_all(&mut popup.draft, &popup.rows);
        popup.confirm(state);
        Self(popup)
    }
    pub(in crate::desktop) unsafe fn poll(&mut self, owner: HWND, state: &State) -> bool {
        self.0.poll(owner, state).unwrap()
    }
    pub(in crate::desktop) fn checked(state: &State) -> Vec<bool> {
        entries(state, DEVICES).iter().map(|r| r.checked).collect()
    }
}
