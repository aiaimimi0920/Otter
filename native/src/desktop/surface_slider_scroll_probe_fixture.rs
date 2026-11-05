//! 系统触摸探针使用正式 Surface/挂载/滚动 owner；仅向测试闭包借出实际 HWND。
use super::*;

fn scene() -> Scene {
    Scene::parse(
        br#"{
        "id":"root","type":"scroll","layout":{"width":240,"height":180},
        "events":{"click":"outer"},"children":[{
            "id":"content","type":"column","layout":{"width":240,"height":600,"minHeight":600},
            "children":[
                {"id":"spacer","type":"view","layout":{"height":80,"minHeight":80}},
                {"id":"slider","type":"slider","layout":{"width":200,"height":30},
                 "props":{"value":50},"events":{"input":"input","change":"change"}},
                {"id":"tail","type":"view","layout":{"height":400,"minHeight":400}}
            ]
        }]
    }"#,
    )
    .unwrap()
}
fn fixture() -> SurfaceWindow {
    let window = SurfaceWindow::new(scene(), 360, 300).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    window.show().unwrap();
    window
}
fn binding() -> crate::surface_live_binding::Binding {
    crate::surface_live_binding::Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":1}}}}"#,
        "https://localhost:1234", "viewer:test", "node:test").unwrap()
}
fn settle_horizontal(window: &SurfaceWindow, value: f64) {
    // 走正式 prepare/settle，而不是只观察 HWND 通知；不发送网络请求。
    let mut wire: Vec<serde_json::Value> = Vec::new();
    let binding = binding();
    for _ in 0..32 {
        if !window.has_pending_edit() {
            break;
        }
        let events = window.prepare_edit(&binding, 1).unwrap();
        assert!(!events.is_empty());
        for event in &events {
            wire.push(serde_json::from_slice(event.bytes()).unwrap());
        }
        window.settle_edits(&events).unwrap();
    }
    assert!(!window.has_pending_edit(), "bounded queue did not settle");
    assert_eq!(wire.first().unwrap()["event"], "input");
    let last = wire.last().unwrap();
    assert_eq!(last["event"], "change");
    assert_eq!(last["class"], "commit");
    assert_eq!(last["payload"]["value"].as_f64(), Some(value));
    assert_eq!(
        wire.iter()
            .filter(|event| event["event"] == "change")
            .count(),
        1
    );
    for event in &wire {
        assert_eq!(event["nodeId"], "slider");
        assert!(event["payload"]["value"].is_number());
        if event["event"] == "input" {
            assert_eq!(event["class"], "continuous");
        }
    }
    println!(
        "SURFACE_TOUCH_WIRE events={} final={value} settled=true",
        wire.len()
    );
}

pub(in crate::desktop) fn cutoff(replace: bool, input: impl FnOnce(HWND, HWND, &dyn Fn())) {
    let window = fixture();
    let child = window
        .state
        .host
        .borrow()
        .as_ref()
        .unwrap()
        .accessibility_edit_hwnd("slider")
        .unwrap();
    let stopped = Cell::new(None);
    input(window.hwnd, child, &|| {
        println!(
            "CUTOFF_STATE replace={replace} pan={} origin={:?} foreground_own={}",
            window.state.pan.borrow().is_some(),
            origin(&window),
            unsafe { GetForegroundWindow() } == window.hwnd
        );
        assert!(stopped.get().is_none(), "cutoff must execute only once");
        assert!(
            window.state.pan.borrow().is_some(),
            "cutoff did not observe an active pan"
        );
        assert!(
            origin(&window)[1] < -1.,
            "cutoff before actual scrolling would be vacuous"
        );
        if replace {
            window.replace(scene()).unwrap();
        } else {
            window.set_input_policy(false, false).unwrap();
        }
        stopped.set(Some(origin(&window)));
        assert!(window.state.pan.borrow().is_none());
    });
    let stopped = stopped.get().expect("cutoff callback was not executed");
    println!(
        "SURFACE_CUTOFF replace={replace} stopped={stopped:?} final={:?}",
        origin(&window)
    );
    assert_eq!(
        origin(&window),
        stopped,
        "late touch or inertia mutated the cut-off Surface"
    );
    assert!(window.state.pan.borrow().is_none());
    assert_eq!(
        window
            .state
            .host
            .borrow()
            .as_ref()
            .unwrap()
            .slider_value("slider"),
        Some(50.)
    );
    assert!(!window.has_pending_edit());
    assert!(!window.has_pending_click());
    assert!(window.take_error().is_none());
}
fn origin(window: &SurfaceWindow) -> [f32; 2] {
    window
        .state
        .host
        .borrow()
        .as_ref()
        .unwrap()
        .presentation()
        .geometry()
        .nodes()[1]
        .origin()
}
pub(in crate::desktop) fn run(horizontal: bool, input: impl FnOnce(HWND, HWND)) {
    let window = fixture();
    let child = window
        .state
        .host
        .borrow()
        .as_ref()
        .unwrap()
        .accessibility_edit_hwnd("slider")
        .unwrap();
    let before = origin(&window);
    input(window.hwnd, child);
    let after = origin(&window);
    let value = window
        .state
        .host
        .borrow()
        .as_ref()
        .unwrap()
        .slider_value("slider")
        .unwrap();
    println!(
        "SURFACE_TOUCH horizontal={horizontal} before={before:?} after={after:?} value={value}"
    );
    if horizontal {
        assert_eq!(after, before, "horizontal Slider touch scrolled parent");
        assert!(value > 50.);
        assert!(window.has_pending_edit());
        settle_horizontal(&window, value);
    } else {
        assert!(
            after[1] < before[1] - 1.,
            "vertical touch did not scroll real Surface"
        );
        assert_eq!(value, 50.);
        assert!(
            !window.has_pending_edit(),
            "vertical scroll generated Slider events"
        );
    }
    assert!(!window.has_pending_click());
    assert!(window.take_error().is_none());
}

#[test]
fn surface_gesture_accepts_only_root_and_current_mounted_slider() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = fixture();
    let child = window
        .state
        .host
        .borrow()
        .as_ref()
        .unwrap()
        .accessibility_edit_hwnd("slider")
        .unwrap();
    assert!(gesture::accepts_target(
        window.hwnd,
        &window.state,
        window.hwnd
    ));
    assert!(gesture::accepts_target(window.hwnd, &window.state, child));
    let foreign = fixture();
    assert!(!gesture::accepts_target(
        window.hwnd,
        &window.state,
        foreign.hwnd
    ));
    let foreign_child = foreign
        .state
        .host
        .borrow()
        .as_ref()
        .unwrap()
        .accessibility_edit_hwnd("slider")
        .unwrap();
    assert!(!gesture::accepts_target(
        window.hwnd,
        &window.state,
        foreign_child
    ));
    let occupied = window.state.host.borrow_mut();
    assert!(!gesture::accepts_target(window.hwnd, &window.state, child));
    drop(occupied);
    unsafe {
        let rogue = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("STATIC"),
            w!(""),
            WS_CHILD,
            0,
            0,
            10,
            10,
            Some(window.hwnd),
            None,
            None,
            None,
        )
        .unwrap();
        let accepted = gesture::accepts_target(window.hwnd, &window.state, rogue);
        DestroyWindow(rogue).unwrap();
        assert!(!accepted, "an unmounted child cannot cancel or pan Surface");
    }
    window
        .replace(
            Scene::parse(br#"{"id":"replacement","type":"text","props":{"text":"done"}}"#).unwrap(),
        )
        .unwrap();
    assert!(!gesture::accepts_target(window.hwnd, &window.state, child));
}

#[test]
fn surface_gesture_same_sequence_from_other_target_cannot_move_or_end_child_pan() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = fixture();
    let child = window
        .state
        .host
        .borrow()
        .as_ref()
        .unwrap()
        .accessibility_edit_hwnd("slider")
        .unwrap();
    let mut rect = RECT::default();
    let mut point;
    unsafe {
        GetWindowRect(child, &mut rect).unwrap();
        point = POINT {
            x: (rect.left + rect.right) / 2,
            y: (rect.top + rect.bottom) / 2,
        };
        assert!(ScreenToClient(window.hwnd, &mut point).as_bool());
    }
    let before = origin(&window);
    gesture::pan_from(
        window.hwnd,
        &window.state,
        child,
        7,
        GF_BEGIN,
        [point.x, point.y],
    );
    assert!(window.state.pan.borrow().is_some());
    gesture::pan_from(
        window.hwnd,
        &window.state,
        window.hwnd,
        7,
        GF_END,
        [point.x, point.y - 20],
    );
    assert_eq!(origin(&window), before);
    assert!(window.state.pan.borrow().is_some());
    gesture::pan_from(
        window.hwnd,
        &window.state,
        child,
        7,
        GF_END,
        [point.x, point.y - 20],
    );
    assert!(origin(&window)[1] < before[1]);
    assert!(window.state.pan.borrow().is_none());
}
