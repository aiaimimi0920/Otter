//! 仅按自启PID/类/owner定位窗口，经真实菜单和UIA公开模式操作，不发送全局键鼠。
use super::fixture::Product;
use std::time::{Duration, Instant};
use windows::{
    core::{BOOL, BSTR},
    Win32::{
        Foundation::*,
        System::Com::*,
        UI::{Accessibility::*, WindowsAndMessaging::*},
    },
};
pub fn until<T>(product: &mut Product, stage: &str, mut check: impl FnMut(u32) -> Option<T>) -> T {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(value) = check(product.0.id()) {
            return value;
        }
        assert!(
            product.0.try_wait().unwrap().is_none(),
            "product exited at {stage}"
        );
        assert!(
            Instant::now() < deadline,
            "product witness timed out: {stage}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
struct Search<'a> {
    pid: u32,
    class: &'a str,
    title: Option<&'a str>,
    owner: Option<HWND>,
    found: Vec<HWND>,
}
unsafe extern "system" fn enumerate(hwnd: HWND, lp: LPARAM) -> BOOL {
    let search = &mut *(lp.0 as *mut Search<'_>);
    let mut pid = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid != search.pid || !IsWindowVisible(hwnd).as_bool() {
        return BOOL(1);
    }
    let (mut class, mut title) = ([0; 128], [0; 256]);
    let cn = GetClassNameW(hwnd, &mut class);
    let tn = GetWindowTextW(hwnd, &mut title);
    if String::from_utf16_lossy(&class[..cn as usize]).eq_ignore_ascii_case(search.class)
        && search
            .title
            .is_none_or(|expected| String::from_utf16_lossy(&title[..tn as usize]) == expected)
        && search
            .owner
            .is_none_or(|owner| GetWindow(hwnd, GW_OWNER).ok() == Some(owner))
    {
        search.found.push(hwnd);
    }
    BOOL(1)
}
pub fn windows(pid: u32, class: &str, title: Option<&str>, owner: Option<HWND>) -> Vec<HWND> {
    let mut search = Search {
        pid,
        class,
        title,
        owner,
        found: Vec::new(),
    };
    unsafe {
        EnumWindows(
            Some(enumerate),
            LPARAM(&mut search as *mut Search<'_> as isize),
        )
        .unwrap()
    };
    search.found
}
pub struct Ui {
    pub client: std::mem::ManuallyDrop<IUIAutomation2>,
}
impl Ui {
    pub fn new() -> Self {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok().unwrap();
            let client: IUIAutomation2 =
                CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER).unwrap();
            client.SetConnectionTimeout(3000).unwrap();
            client.SetTransactionTimeout(3000).unwrap();
            Self {
                client: std::mem::ManuallyDrop::new(client),
            }
        }
    }
    pub fn named(&self, hwnd: HWND, name: &str) -> Option<IUIAutomationElement> {
        unsafe {
            let condition = self
                .client
                .CreatePropertyCondition(UIA_NamePropertyId, &name.into())
                .unwrap();
            self.client
                .ElementFromHandle(hwnd)
                .ok()?
                .FindFirst(TreeScope_Descendants, &condition)
                .ok()
        }
    }
    pub fn invoke(&self, element: &IUIAutomationElement) {
        unsafe {
            element
                .GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId)
                .unwrap()
                .Invoke()
                .unwrap()
        };
    }
    pub fn menu(&self, product: &mut Product, hwnd: HWND) -> HWND {
        eprintln!(
            "art-product-stage: owned-foreground pid={} hwnd={hwnd:?}",
            product.0.id()
        );
        unsafe {
            let _ = SetForegroundWindow(hwnd);
        }
        if unsafe { GetForegroundWindow() } != hwnd {
            // 通过系统 HWND provider 请求焦点，仍核真实前台；不伪造 WM_ACTIVATE。
            let focused = unsafe {
                self.client
                    .ElementFromHandle(hwnd)
                    .and_then(|element| element.SetFocus())
            };
            eprintln!(
                "art-product-uia-focus: hwnd={} result={focused:?}",
                hwnd.0 as usize
            );
        }
        if unsafe { GetForegroundWindow() } != hwnd {
            // 父测试不能把给子PID的许可当成自己的许可；外部仅激活实际返回的自有窗口。
            eprintln!(
                "art-product-activate-needed: hwnd={} pid={}",
                hwnd.0 as usize,
                product.0.id()
            );
        }
        until(product, "owned menu foreground", |_| unsafe {
            (GetForegroundWindow() == hwnd).then_some(())
        });
        eprintln!("art-product-activate-satisfied: hwnd={}", hwnd.0 as usize);
        unsafe {
            PostMessageW(
                Some(hwnd),
                WM_CONTEXTMENU,
                WPARAM(hwnd.0 as usize),
                LPARAM(100 | (100 << 16)),
            )
            .unwrap()
        };
        until(product, "owned menu", |pid| {
            windows(pid, "STATIC", None, Some(hwnd)).first().copied()
        })
    }
    pub fn menu_row(&self, menu: HWND, name: &str) -> IUIAutomationElement {
        self.find_menu_row(menu, name)
            .unwrap_or_else(|| panic!("owned menu item not found within bounded pages: {name}"))
    }
    pub(super) fn find_menu_row(&self, menu: HWND, name: &str) -> Option<IUIAutomationElement> {
        unsafe {
            let root = self.client.ElementFromHandle(menu).unwrap();
            for _ in 0..16 {
                if let Some(row) = self.named(menu, name) {
                    return Some(row);
                }
                let scroll = root
                    .GetCurrentPatternAs::<IUIAutomationScrollPattern>(UIA_ScrollPatternId)
                    .unwrap();
                // 未就绪的短子菜单没有可滚动页面；返回缺项，由status关闭快照后再等owner。
                if !scroll.CurrentVerticallyScrollable().unwrap().as_bool()
                    || scroll.CurrentVerticalScrollPercent().unwrap() >= 100.0
                {
                    return None;
                }
                scroll
                    .Scroll(ScrollAmount_NoAmount, ScrollAmount_LargeIncrement)
                    .unwrap();
            }
        }
        None
    }
    pub fn status(&self, product: &mut Product, hwnd: HWND, name: &str) -> bool {
        self.status_at(product, hwnd, None, name)
    }
    pub fn status_in(&self, product: &mut Product, hwnd: HWND, submenu: &str, name: &str) -> bool {
        self.status_at(product, hwnd, Some(submenu), name)
    }
    fn status_at(
        &self,
        product: &mut Product,
        hwnd: HWND,
        submenu: Option<&str>,
        name: &str,
    ) -> bool {
        // 菜单是不可变快照，打开时可能尚未接纳后台完成；关闭后才允许owner继续消费。
        let deadline = Instant::now() + Duration::from_secs(15);
        for _ in 0..8 {
            let menu = self.menu(product, hwnd);
            let page = submenu.map_or(menu, |label| self.submenu(product, hwnd, menu, label));
            let row = self.find_menu_row(page, name);
            let enabled = row.map(|row| unsafe { row.CurrentIsEnabled().unwrap().as_bool() });
            self.dismiss(product, menu);
            if let Some(enabled) = enabled {
                return enabled;
            }
            assert!(
                Instant::now() < deadline,
                "product menu status deadline: {name}"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("product menu status missing: {name}")
    }
    pub fn command(&self, product: &mut Product, hwnd: HWND, name: &str) {
        let menu = self.menu(product, hwnd);
        self.invoke(&self.menu_row(menu, name));
        until(product, "menu command dispatch", |_| unsafe {
            (!IsWindow(Some(menu)).as_bool()).then_some(())
        });
    }
    pub fn command_in(&self, product: &mut Product, hwnd: HWND, submenu: &str, name: &str) {
        let menu = self.menu(product, hwnd);
        let child = self.submenu(product, hwnd, menu, submenu);
        self.invoke(&self.menu_row(child, name));
        until(product, "submenu command dispatch", |_| unsafe {
            (!IsWindow(Some(menu)).as_bool()).then_some(())
        });
    }
    fn submenu(&self, product: &mut Product, hwnd: HWND, menu: HWND, name: &str) -> HWND {
        // 原生菜单的子页通过ExpandCollapse而非Invoke展开；所有级联面板均归同一业务owner。
        unsafe {
            self.menu_row(menu, name)
                .GetCurrentPatternAs::<IUIAutomationExpandCollapsePattern>(
                    UIA_ExpandCollapsePatternId,
                )
                .unwrap()
                .Expand()
                .unwrap();
        }
        until(product, "owned submenu", |pid| {
            let children: Vec<_> = windows(pid, "STATIC", Some("Otter 菜单"), Some(hwnd))
                .into_iter()
                .filter(|candidate| *candidate != menu)
                .collect();
            assert!(children.len() <= 1, "ambiguous owned submenu");
            children.first().copied()
        })
    }
    pub fn input(&self, product: &mut Product, owner: HWND, title: &str, text: &str) {
        let dialog = until(product, "owned input dialog", |pid| {
            windows(pid, "#32770", Some(title), Some(owner))
                .first()
                .copied()
        });
        unsafe {
            let edit = GetDlgItem(Some(dialog), 101).unwrap();
            let field = self.client.ElementFromHandle(edit).unwrap();
            field
                .GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)
                .unwrap()
                .SetValue(&BSTR::from(text))
                .unwrap();
        }
        self.invoke(&self.named(dialog, "确定").unwrap());
        until(product, "input dialog closed", |_| unsafe {
            (!IsWindow(Some(dialog)).as_bool()).then_some(())
        });
    }
    pub fn dismiss(&self, product: &mut Product, menu: HWND) {
        unsafe { PostMessageW(Some(menu), WM_CLOSE, WPARAM(0), LPARAM(0)).unwrap() };
        until(product, "menu dismissed", |_| unsafe {
            (!IsWindow(Some(menu)).as_bool()).then_some(())
        });
    }
    pub fn exit(&self, product: &mut Product, hwnd: HWND) {
        self.command(product, hwnd, "退出 Otter");
        let dialog = until(product, "owned exit confirmation", |pid| {
            windows(pid, "#32770", Some("退出 Otter"), None)
                .first()
                .copied()
        });
        unsafe {
            PostMessageW(Some(dialog), WM_COMMAND, WPARAM(IDNO.0 as usize), LPARAM(0)).unwrap()
        };
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if let Some(status) = product.0.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            assert!(
                Instant::now() < deadline,
                "owned product exit drain timed out"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Ui {
    fn drop(&mut self) {
        // COM接口必须先于所属apartment释放；普通字段在Drop返回后才自动析构。
        unsafe {
            std::mem::ManuallyDrop::drop(&mut self.client);
            CoUninitialize()
        }
    }
}
