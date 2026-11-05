//! 三种输入共用非关闭分页路径；替换后旧页失效，普通命令仍关闭并交付一次。
use super::*;
use windows::{
    core::w,
    Win32::UI::Input::KeyboardAndMouse::{GetCapture, SetCapture},
};
struct Owner(HWND);
impl Owner {
    fn new() -> Self {
        Self(unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("navigation"),
                WS_OVERLAPPED,
                0,
                0,
                100,
                100,
                None,
                None,
                None,
                None,
            )
            .unwrap()
        })
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}
fn row(command: usize) -> Item {
    Item {
        command,
        label: format!("item {command}"),
        enabled: true,
        checked: false,
        radio: false,
        default: false,
        separator: false,
        children: None,
        thumbnail: None,
    }
}
#[test]
fn keyboard_pointer_and_automation_navigation_keep_root_and_capture() {
    for mode in 0..3 {
        let owner = Owner::new();
        let mut parent = row(0);
        parent.children = Some(vec![row(10), row(42)]);
        let mut menu = Cascade::open(owner.0, vec![parent], POINT { x: 20, y: 20 }).unwrap();
        menu.navigation_commands(&[10]).unwrap();
        menu.expand(0, 0).unwrap();
        menu.focus(1, Some(0)).unwrap();
        let root = menu.pages[0].panel.handle().unwrap();
        let child = menu.pages[1].panel.handle().unwrap();
        unsafe {
            SetCapture(root);
        }
        match mode {
            0 => menu.key(0x0d).unwrap(),
            1 => {
                menu.event(1, Event::Down(4, 4)).unwrap();
                menu.event(1, Event::Up(4, 4)).unwrap();
            }
            _ => menu
                .event(
                    1,
                    Event::Automation(0, crate::native_menu::accessibility::Action::Invoke),
                )
                .unwrap(),
        }
        assert!(!menu.closed());
        assert!(menu.take_command().is_none());
        menu.activate(1, 1).unwrap(); // 未处理导航期间不能覆盖成业务命令。
        assert_eq!(menu.take_navigation(), Some(10));
        assert!(menu.take_navigation().is_none());
        menu.replace_submenu(0, vec![row(77)]).unwrap();
        assert_eq!(menu.pages[0].panel.handle().unwrap(), root);
        assert_eq!(unsafe { GetCapture() }, root);
        assert!(!unsafe { IsWindow(Some(child)) }.as_bool());
        menu.focus(1, Some(0)).unwrap();
        menu.key(0x0d).unwrap();
        assert!(menu.closed());
        assert_eq!(menu.take_command(), Some(77));
        assert!(menu.take_command().is_none());
    }
}
#[test]
fn navigation_mapping_is_bounded_unique_and_cannot_be_rebound() {
    let owner = Owner::new();
    let mut menu = Cascade::open(owner.0, vec![row(42)], POINT { x: 0, y: 0 }).unwrap();
    for ids in [vec![0], vec![1, 1], (1..=9).collect()] {
        assert!(menu.navigation_commands(&ids).is_err());
    }
    menu.navigation_commands(&[1, 2]).unwrap();
    menu.navigation_commands(&[1, 2]).unwrap();
    assert!(menu.navigation_commands(&[3]).is_err());
    menu.cancel();
    assert!(menu.take_navigation().is_none());
}
