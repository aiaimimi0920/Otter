//! 跨源线程 dialog 只从已核验焦点/既有成员补入；不枚举全进程或操作系统前台。
use super::*;
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError};

enum Action {
    Show,
    Hide,
    Owner(usize),
}
struct ThreadPopup {
    hwnd: HWND,
    commands: Option<SyncSender<(Action, SyncSender<()>)>>,
    thread: Option<std::thread::JoinHandle<()>>,
}
fn pump() {
    unsafe {
        let mut message = MSG::default();
        while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
            DispatchMessageW(&message);
        }
    }
}
fn receive<T>(reply: Receiver<T>) -> T {
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        match reply.try_recv() {
            Ok(value) => return value,
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => panic!("popup source thread disconnected"),
        }
        assert!(Instant::now() < until, "popup source thread timed out");
        // 创建/销毁跨线程 owned HWND 时允许源回调，不移动鼠标或切换前台。
        pump();
        std::thread::sleep(Duration::from_millis(1));
    }
}
impl ThreadPopup {
    fn new(owner: HWND) -> Self {
        let (commands, requests) = mpsc::sync_channel::<(Action, SyncSender<()>)>(1);
        let (ready, starting) = mpsc::sync_channel(1);
        let thread = std::thread::spawn(move || {
            let popup = Source::new(None, rect(-30000, -30000, 80, 60), true);
            ready.send(popup.0 .0 as usize).unwrap();
            while let Ok((action, reply)) = requests.recv_timeout(Duration::from_secs(20)) {
                unsafe {
                    match action {
                        Action::Show => popup.show(),
                        Action::Hide => {
                            let _ = ShowWindow(popup.0, SW_HIDE);
                        }
                        Action::Owner(owner) => {
                            SetWindowLongPtrW(popup.0, GWLP_HWNDPARENT, owner as isize);
                            assert_eq!(GetWindow(popup.0, GW_OWNER).unwrap().0 as usize, owner);
                        }
                    }
                }
                let _ = reply.send(());
            }
        });
        let popup = Self {
            hwnd: HWND(receive(starting) as *mut _),
            commands: Some(commands),
            thread: Some(thread),
        };
        popup.call(Action::Owner(owner.0 as usize));
        popup
    }
    fn call(&self, action: Action) {
        let (reply, completed) = mpsc::sync_channel(1);
        self.commands
            .as_ref()
            .unwrap()
            .send((action, reply))
            .unwrap();
        receive(completed);
    }
}
impl Drop for ThreadPopup {
    fn drop(&mut self) {
        self.commands.take();
        let thread = self.thread.take().unwrap();
        let until = Instant::now() + Duration::from_secs(10);
        while !thread.is_finished() {
            assert!(Instant::now() < until, "popup source cleanup timed out");
            pump();
            std::thread::sleep(Duration::from_millis(1));
        }
        thread.join().unwrap();
    }
}
fn contains(scopes: &[Scope], hwnd: HWND) -> bool {
    scopes
        .iter()
        .any(|scope| scope.target().is_ok_and(|t| t.hwnd() == hwnd))
}

#[test]
fn authenticated_cross_thread_focus_is_discovered_and_existing_member_survives_b_focus() {
    let root = Source::new(None, rect(-30000, -30000, 100, 100), true);
    let target = Target::bind(root.0).unwrap();
    let popup = ThreadPopup::new(root.0);
    popup.call(Action::Show);
    assert_ne!(unsafe { GetWindowThreadProcessId(root.0, None) }, unsafe {
        GetWindowThreadProcessId(popup.hwnd, None)
    });
    assert!(!contains(
        &discover(&target, None, &[], &[]).unwrap(),
        popup.hwnd
    ));
    let focus = Scope::bind(&target, None, popup.hwnd).unwrap();
    let discovered = discover(&target, None, &[], &[focus.clone(), focus.clone()]).unwrap();
    assert!(contains(&discovered, popup.hwnd));
    assert_eq!(
        discovered
            .iter()
            .filter(|s| s.target().unwrap().hwnd() == popup.hwnd)
            .count(),
        1
    );
    let member = discovered
        .into_iter()
        .find(|s| s.target().unwrap().hwnd() == popup.hwnd)
        .unwrap();
    // focus.active 会在回到 B 时清除；已认证成员必须继续进入 present，不能立刻 stop。
    assert!(contains(
        &discover(&target, None, &[(&member, None)], &[]).unwrap(),
        popup.hwnd
    ));
    popup.call(Action::Hide);
    assert!(!contains(
        &discover(&target, None, &[(&member, None)], &[focus]).unwrap(),
        popup.hwnd
    ));
}

#[test]
fn cross_thread_candidate_owner_change_and_foreign_scope_are_not_authorization() {
    let root = Source::new(None, rect(-30000, -30000, 100, 100), true);
    let other = Source::new(None, rect(-30000, -30000, 100, 100), true);
    let target = Target::bind(root.0).unwrap();
    let popup = ThreadPopup::new(root.0);
    popup.call(Action::Show);
    let old = Scope::bind(&target, None, popup.hwnd).unwrap();
    popup.call(Action::Owner(other.0 .0 as usize));
    assert!(old.target().is_err());
    let foreign = Scope::bind(&Target::bind(other.0).unwrap(), None, popup.hwnd).unwrap();
    assert!(!contains(
        &discover(&target, None, &[(&old, None)], &[foreign]).unwrap(),
        popup.hwnd
    ));
}

#[test]
fn cross_thread_nested_dialog_keeps_nearest_parent_mirror_and_epoch() {
    let root = Source::new(None, rect(-30000, -30000, 100, 100), true);
    let parent = Source::new(Some(root.0), rect(-30000, -30000, 80, 60), true);
    let mirror = Source::new(None, rect(-30000, -30000, 80, 60), true);
    let owner = crate::live_hover_owner::Owner::new(mirror.0).unwrap();
    owner.control(true).unwrap();
    let target = Target::bind(root.0).unwrap();
    let parent_scope = Scope::bind(&target, None, parent.0).unwrap();
    let popup = ThreadPopup::new(parent.0);
    popup.call(Action::Show);
    let focus = Scope::bind(&parent_scope.target().unwrap(), None, popup.hwnd).unwrap();
    let scopes = discover(
        &target,
        None,
        &[(&parent_scope, Some(owner.identity()))],
        &[focus],
    )
    .unwrap();
    let child = scopes
        .into_iter()
        .find(|s| s.target().unwrap().hwnd() == popup.hwnd)
        .unwrap();
    assert_eq!(child.mirror_owner(), Some(mirror.0));
    owner.control(false).unwrap();
    assert!(
        child.target().is_err(),
        "parent epoch still binds cross-thread dialog"
    );
}
