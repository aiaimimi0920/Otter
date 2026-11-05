//! 非模态可信确认：窗口过程只记录选择，原Window在重新核验宿主后才移交批准。
use crate::live_relay_http::art_prepare::service::confirmation::Ticket;
use std::{cell::Cell, sync::Arc};
use windows::{
    core::HSTRING,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::SystemServices::{SS_CENTER, SS_OWNERDRAW},
        UI::{Controls::DRAWITEMSTRUCT, WindowsAndMessaging::*},
    },
};
const USER: WINDOW_LONG_PTR_INDEX =
    WINDOW_LONG_PTR_INDEX((2 * std::mem::size_of::<isize>()) as i32);
const TIMER: usize = 0x4f74;
struct State {
    ticket: Arc<Ticket>,
    choice: Cell<Option<bool>>,
    failed: Cell<bool>,
}
pub(super) struct Prompt {
    hwnd: HWND,
    state: Box<State>,
}
impl Prompt {
    pub fn new(owner: HWND, ticket: Arc<Ticket>) -> windows::core::Result<Self> {
        Self::create(owner, ticket, true)
    }
    #[cfg(test)]
    pub(super) fn hidden(owner: HWND, ticket: Arc<Ticket>) -> windows::core::Result<Self> {
        Self::create(owner, ticket, false)
    }
    fn create(owner: HWND, ticket: Arc<Ticket>, visible: bool) -> windows::core::Result<Self> {
        let state = Box::new(State {
            ticket,
            choice: Cell::new(None),
            failed: Cell::new(false),
        });
        let template = template(&state.ticket.images);
        // Box地址稳定；DestroyWindow同步返回后才释放State，过程不借用Application/Document。
        let hwnd = unsafe {
            CreateDialogIndirectParamW(
                None,
                template.as_ptr().cast(),
                Some(owner),
                Some(procedure),
                LPARAM(&*state as *const State as isize),
            )?
        };
        let prompt = Self { hwnd, state };
        if prompt.state.failed.get() {
            return Err(windows::core::Error::from_win32());
        }
        if visible {
            unsafe {
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            }
        }
        Ok(prompt)
    }
    pub fn take_choice(&self) -> Option<bool> {
        if !self.state.ticket.live() {
            return Some(false);
        }
        self.state.choice.take()
    }
    pub fn resolve(&self, approve: bool) {
        self.state.ticket.resolve(approve);
    }
    pub(super) fn handle(&self) -> Option<HWND> {
        // owner关闭可同步销毁子对话；不得对已复用的HWND发送消息或再次DestroyWindow。
        unsafe {
            (IsWindow(Some(self.hwnd)).as_bool()
                && GetWindowLongPtrW(self.hwnd, USER) == &*self.state as *const State as isize)
                .then_some(self.hwnd)
        }
    }
}
impl Drop for Prompt {
    fn drop(&mut self) {
        self.state.ticket.resolve(false);
        unsafe {
            if self.handle().is_some() {
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}
/// # Safety
/// hwnd须为当前UI线程刚从owner核验并复制的提示；调用时不持Application/Document借用。
pub(in crate::desktop) unsafe fn translate_confirmation(hwnd: HWND, message: &MSG) -> bool {
    if !(WM_KEYFIRST..=WM_KEYLAST).contains(&message.message)
        || (message.hwnd != hwnd && !IsChild(hwnd, message.hwnd).as_bool())
    {
        return false;
    }
    // 多行只读Edit对Enter声明DLGC_WANTALLKEYS，不能只依赖IsDialogMessage的默认按钮。
    // 参数区Enter及任意自有控件Escape均只记录拒绝；批准仍需明确选择允许按钮。
    let deny = message.message == WM_KEYDOWN
        && (message.wParam.0 == 0x1b
            || (message.wParam.0 == 0x0d
                && GetDlgItem(Some(hwnd), 101).is_ok_and(|input| input == message.hwnd)));
    if deny {
        SendMessageW(hwnd, WM_COMMAND, Some(WPARAM(2)), Some(LPARAM(0)));
        true
    } else {
        IsDialogMessageW(hwnd, message).as_bool()
    }
}
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> isize {
    if message == WM_INITDIALOG {
        SetWindowLongPtrW(hwnd, USER, lp.0);
    }
    let Some(state) = (GetWindowLongPtrW(hwnd, USER) as *const State).as_ref() else {
        return 0;
    };
    let choose = |approved| {
        let _ = KillTimer(Some(hwnd), TIMER);
        if state.choice.get().is_none() {
            state.choice.set(Some(approved));
            state.ticket.wake();
        }
        let _ = ShowWindow(hwnd, SW_HIDE);
    };
    match message {
        WM_INITDIALOG => {
            if SetDlgItemTextW(hwnd, 101, &HSTRING::from(&state.ticket.text)).is_err()
                || SetTimer(Some(hwnd), TIMER, 50, None) == 0
            {
                state.failed.set(true);
                choose(false);
            }
            SendMessageW(hwnd, DM_SETDEFID, Some(WPARAM(2)), Some(LPARAM(0)));
            // 不让dialog manager在创建隐藏/NOACTIVATE提示时自动为首个控件设置焦点。
            0
        }
        WM_COMMAND if wp.0 & 0xffff == 1 => {
            choose(true);
            1
        }
        WM_COMMAND if wp.0 & 0xffff == 2 => {
            choose(false);
            1
        }
        WM_CLOSE => {
            choose(false);
            1
        }
        WM_DRAWITEM if lp.0 != 0 => {
            let item = &*(lp.0 as *const DRAWITEMSTRUCT);
            if let Some(image) = item
                .CtlID
                .checked_sub(201)
                .and_then(|i| state.ticket.images.get(i as usize))
            {
                FillRect(item.hDC, &item.rcItem, GetSysColorBrush(COLOR_WINDOW));
                if !crate::image_thumbnail::draw::draw(
                    item.hDC,
                    item.rcItem,
                    &image.image,
                    GetSysColor(COLOR_WINDOW),
                ) {
                    state.failed.set(true);
                    choose(false);
                }
                1
            } else {
                0
            }
        }
        WM_TIMER if wp.0 == TIMER => {
            if !state.ticket.live() {
                choose(false);
            }
            1
        }
        WM_NCDESTROY => {
            let _ = KillTimer(Some(hwnd), TIMER);
            SetWindowLongPtrW(hwnd, USER, 0);
            1
        }
        _ => 0,
    }
}
fn template(
    images: &[crate::live_relay_http::art_prepare::service::confirmation::Preview],
) -> Vec<u32> {
    fn text(data: &mut Vec<u16>, value: &str) {
        data.extend(value.encode_utf16());
        data.push(0);
    }
    fn word(data: &mut Vec<u16>, value: u32) {
        data.extend([value as u16, (value >> 16) as u16]);
    }
    let mut data = Vec::new();
    word(
        &mut data,
        WS_POPUP.0 | WS_CAPTION.0 | WS_SYSMENU.0 | DS_MODALFRAME as u32 | DS_SETFONT as u32,
    );
    word(&mut data, 0);
    let buttons_y = if images.is_empty() { 210 } else { 280 };
    data.extend([3 + 2 * images.len() as u16, 0, 0, 360, buttons_y + 30, 0, 0]);
    text(&mut data, "Otter · Art 服务确认");
    data.push(9);
    text(&mut data, "Microsoft YaHei UI");
    let mut controls = vec![
        (
            101,
            0x81,
            WS_BORDER.0
                | WS_VSCROLL.0
                | WS_TABSTOP.0
                | ES_MULTILINE as u32
                | ES_READONLY as u32
                | ES_AUTOVSCROLL as u32,
            10,
            10,
            340,
            190,
            "",
        ),
        (1, 0x80, WS_TABSTOP.0, 220, buttons_y, 65, 20, "允许本次"),
        (
            2,
            0x80,
            WS_TABSTOP.0 | BS_DEFPUSHBUTTON as u32,
            290,
            buttons_y,
            60,
            20,
            "拒绝",
        ),
    ];
    for (index, image) in images.iter().enumerate() {
        let x = 10 + index as u16 * 85;
        controls.push((201 + index as u16, 0x82, SS_OWNERDRAW.0, x, 207, 80, 57, ""));
        controls.push((
            301 + index as u16,
            0x82,
            SS_CENTER.0,
            x,
            265,
            80,
            12,
            image.label.as_str(),
        ));
    }
    for (id, class, style, x, y, width, height, label) in controls {
        if data.len() % 2 != 0 {
            data.push(0);
        }
        word(&mut data, WS_CHILD.0 | WS_VISIBLE.0 | style);
        word(&mut data, 0);
        data.extend([x, y, width, height, id, 0xffff, class]);
        text(&mut data, label);
        data.push(0);
    }
    if data.len() % 2 != 0 {
        data.push(0);
    }
    data.chunks_exact(2)
        .map(|value| u32::from(value[0]) | u32::from(value[1]) << 16)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live_relay_http::art_prepare::service::confirmation::Channel;
    use std::time::{Duration, Instant};
    #[test]
    fn native_confirmation_only_records_choice_and_drop_denies_unclaimed_approval() {
        let channel = Channel::new(Arc::new(|| {}));
        let ticket = channel
            .publish(
                "原生确认\r\n参数：42".into(),
                Instant::now() + Duration::from_secs(5),
            )
            .unwrap();
        let prompt = Prompt::hidden(HWND::default(), ticket.clone()).unwrap();
        assert!(!unsafe { IsWindowVisible(prompt.hwnd) }.as_bool());
        unsafe {
            SendMessageW(prompt.hwnd, WM_COMMAND, Some(WPARAM(1)), Some(LPARAM(0)));
        }
        assert_eq!(prompt.take_choice(), Some(true));
        assert_eq!(ticket.choice(), None);
        let hwnd = prompt.hwnd;
        drop(prompt);
        assert!(!unsafe { IsWindow(Some(hwnd)) }.as_bool());
        assert_eq!(ticket.choice(), Some(false));
    }
    #[test]
    fn native_confirmation_revocation_hides_timer_window_and_cannot_be_approved() {
        let channel = Channel::new(Arc::new(|| {}));
        let ticket = channel
            .publish("expired".into(), Instant::now() + Duration::from_secs(5))
            .unwrap();
        let prompt = Prompt::hidden(HWND::default(), ticket.clone()).unwrap();
        channel.close();
        unsafe {
            SendMessageW(prompt.hwnd, WM_TIMER, Some(WPARAM(TIMER)), Some(LPARAM(0)));
        }
        assert!(!unsafe { IsWindowVisible(prompt.hwnd) }.as_bool());
        assert_eq!(prompt.take_choice(), Some(false));
        assert!(!ticket.resolve(true));
        let hwnd = prompt.hwnd;
        drop(prompt);
        assert!(!unsafe { IsWindow(Some(hwnd)) }.as_bool());
    }
    #[test]
    fn native_confirmation_modeless_keyboard_defaults_to_denial() {
        let channel = Channel::new(Arc::new(|| {}));
        let ticket = channel
            .publish("keyboard".into(), Instant::now() + Duration::from_secs(5))
            .unwrap();
        let prompt = Prompt::hidden(HWND::default(), ticket.clone()).unwrap();
        let input = unsafe { GetDlgItem(Some(prompt.hwnd), 101) }.unwrap();
        let message = MSG {
            hwnd: input,
            message: WM_KEYDOWN,
            wParam: WPARAM(0x0d),
            ..Default::default()
        };
        assert_eq!(
            unsafe { SendMessageW(prompt.hwnd, DM_GETDEFID, None, None) }.0 & 0xffff,
            2
        );
        assert!(unsafe { translate_confirmation(prompt.hwnd, &message) });
        assert_eq!(prompt.take_choice(), Some(false));
        assert_eq!(ticket.choice(), None);
    }
    #[test]
    fn native_confirmation_keyboard_ignores_foreign_window_and_escape_denies() {
        let channel = Channel::new(Arc::new(|| {}));
        let ticket = channel
            .publish("keyboard".into(), Instant::now() + Duration::from_secs(5))
            .unwrap();
        let prompt = Prompt::hidden(HWND::default(), ticket.clone()).unwrap();
        let mut message = MSG {
            message: WM_KEYDOWN,
            wParam: WPARAM(0x1b),
            ..Default::default()
        };
        assert!(!unsafe { translate_confirmation(prompt.hwnd, &message) });
        assert_eq!(prompt.take_choice(), None);
        message.hwnd = unsafe { GetDlgItem(Some(prompt.hwnd), 1) }.unwrap();
        assert!(unsafe { translate_confirmation(prompt.hwnd, &message) });
        assert_eq!(prompt.take_choice(), Some(false));
        assert_eq!(ticket.choice(), None);
    }
    #[test]
    fn native_confirmation_keeps_complete_long_parameters_without_changing_thread_focus() {
        use windows::Win32::UI::Input::KeyboardAndMouse::{GetActiveWindow, GetFocus};
        let channel = Channel::new(Arc::new(|| {}));
        let text = "完整参数：42\r\n".repeat(8000);
        let ticket = channel
            .publish(text.clone(), Instant::now() + Duration::from_secs(5))
            .unwrap();
        let focus = unsafe { GetFocus() };
        let active = unsafe { GetActiveWindow() };
        let prompt = Prompt::hidden(HWND::default(), ticket).unwrap();
        assert_eq!(unsafe { GetFocus() }, focus);
        assert_eq!(unsafe { GetActiveWindow() }, active);
        let input = unsafe { GetDlgItem(Some(prompt.hwnd), 101) }.unwrap();
        let expected: Vec<_> = text.encode_utf16().collect();
        assert_eq!(
            unsafe { GetWindowTextLengthW(input) } as usize,
            expected.len()
        );
        let mut buffer = vec![0; expected.len() + 1];
        let copied = unsafe { GetWindowTextW(input, &mut buffer) } as usize;
        assert_eq!(&buffer[..copied], expected.as_slice());
        assert_eq!(unsafe { GetFocus() }, focus);
        assert_eq!(unsafe { GetActiveWindow() }, active);
    }
}
