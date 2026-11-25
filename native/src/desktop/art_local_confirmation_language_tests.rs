//! Hidden real HWND regressions; language refresh is not a new confirmation or authorization.
use super::*;
use crate::live_relay_http::art_prepare::service::confirmation::{Channel, Preview};
use std::time::{Duration, Instant};
use windows::Win32::UI::{
    Controls::{EM_GETSEL, EM_SETSEL},
    Input::KeyboardAndMouse::{GetActiveWindow, GetFocus},
    Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
};

struct LanguageReset(DisplayLanguage);
impl LanguageReset {
    fn english() -> Self {
        let reset = Self(ui_text::current_language());
        ui_text::set_language(DisplayLanguage::English);
        reset
    }
}
impl Drop for LanguageReset {
    fn drop(&mut self) {
        ui_text::set_language(self.0);
    }
}

fn ticket() -> Arc<Ticket> {
    Channel::new(Arc::new(|| {}))
        .publish(
            "Original 参数 🙂\r\n{\"input\":\"unchanged\"}".into(),
            Instant::now() + Duration::from_secs(30),
        )
        .unwrap()
}

fn window_text(hwnd: HWND) -> String {
    unsafe {
        let mut buffer = vec![0; GetWindowTextLengthW(hwnd) as usize + 1];
        let copied = GetWindowTextW(hwnd, &mut buffer) as usize;
        String::from_utf16(&buffer[..copied]).unwrap()
    }
}

fn assert_labels(hwnd: HWND) {
    assert_eq!(
        window_text(hwnd),
        ui_text::text("integration-art-confirm-title")
    );
    for (id, key) in [(1, "integration-allow-once"), (2, "integration-reject")] {
        let control = unsafe { GetDlgItem(Some(hwnd), id) }.unwrap();
        assert_eq!(window_text(control), ui_text::text(key));
    }
}

#[test]
fn language_refresh_preserves_prompt_ticket_controls_choice_and_focus() {
    let _reset = LanguageReset::english();
    let mut ticket = ticket();
    Arc::get_mut(&mut ticket).unwrap().images.push(Preview {
        label: "User preview 原图".into(),
        size: [1, 1],
        image: crate::image_thumbnail::Thumbnail::from_rgba(1, 1, &[0, 0, 0, 255]).unwrap(),
    });
    let prompt = Prompt::hidden(HWND::default(), ticket.clone()).unwrap();
    let hwnd = prompt.handle().unwrap();
    let controls = [101, 1, 2, 201, 301].map(|id| unsafe { GetDlgItem(Some(hwnd), id).unwrap() });
    let focus = unsafe { GetFocus() };
    let active = unsafe { GetActiveWindow() };
    unsafe {
        SendMessageW(controls[0], EM_SETSEL, Some(WPARAM(2)), Some(LPARAM(8)));
    }
    let selection = unsafe { SendMessageW(controls[0], EM_GETSEL, None, None) };
    assert_labels(hwnd);
    for language in [DisplayLanguage::SimplifiedChinese, DisplayLanguage::English] {
        ui_text::set_language(language);
        prompt.refresh_language().unwrap();
        assert_eq!(prompt.language.get(), Some(language));
        assert_eq!(prompt.handle(), Some(hwnd));
        assert!(Arc::ptr_eq(&prompt.state.ticket, &ticket));
        assert_eq!(
            [101, 1, 2, 201, 301].map(|id| unsafe { GetDlgItem(Some(hwnd), id).unwrap() }),
            controls
        );
        assert_labels(hwnd);
        assert_eq!(window_text(controls[0]), ticket.text);
        assert_eq!(window_text(controls[4]), ticket.images[0].label);
        assert_eq!(
            unsafe { SendMessageW(controls[0], EM_GETSEL, None, None) },
            selection
        );
        assert_eq!(unsafe { GetFocus() }, focus);
        assert_eq!(unsafe { GetActiveWindow() }, active);
        assert!(!unsafe { IsWindowVisible(hwnd) }.as_bool());
        assert_eq!(ticket.choice(), None);
        assert!(!prompt.state.failed.get());
        if language == DisplayLanguage::SimplifiedChinese {
            assert_eq!(prompt.state.choice.get(), None);
            unsafe {
                SendMessageW(hwnd, WM_COMMAND, Some(WPARAM(1)), Some(LPARAM(0)));
            }
        } else {
            assert_eq!(prompt.state.choice.get(), Some(true));
        }
    }
    assert_eq!(prompt.take_choice(), Some(true));
    assert_eq!(ticket.choice(), None);
}

#[test]
fn partial_language_refresh_retries_when_returning_to_the_original_language() {
    let _reset = LanguageReset::english();
    let ticket = ticket();
    let prompt = Prompt::hidden(HWND::default(), ticket.clone()).unwrap();
    let hwnd = prompt.handle().unwrap();
    let reject = unsafe { GetDlgItem(Some(hwnd), 2) }.unwrap();
    unsafe {
        SetWindowLongPtrW(reject, GWLP_ID, 3);
    }
    ui_text::set_language(DisplayLanguage::SimplifiedChinese);
    assert!(prompt.refresh_language().is_err());
    assert_eq!(prompt.language.get(), None);
    assert_eq!(
        window_text(hwnd),
        ui_text::text("integration-art-confirm-title")
    );
    assert_eq!(prompt.state.choice.get(), None);
    assert_eq!(ticket.choice(), None);
    assert!(!prompt.state.failed.get());
    unsafe {
        SetWindowLongPtrW(reject, GWLP_ID, 2);
    }
    ui_text::set_language(DisplayLanguage::English);
    prompt.refresh_language().unwrap();
    assert_eq!(prompt.language.get(), Some(DisplayLanguage::English));
    assert_labels(hwnd);
    ticket.close();
    unsafe {
        SendMessageW(hwnd, WM_TIMER, Some(WPARAM(TIMER)), Some(LPARAM(0)));
    }
    assert_eq!(prompt.take_choice(), Some(false));
}

#[test]
fn language_refresh_rejects_changed_or_destroyed_prompt_identity() {
    let _reset = LanguageReset::english();
    let ticket = ticket();
    let prompt = Prompt::hidden(HWND::default(), ticket.clone()).unwrap();
    let hwnd = prompt.handle().unwrap();
    let original_title = window_text(hwnd);
    let identity = unsafe { SetWindowLongPtrW(hwnd, USER, 0) };
    ui_text::set_language(DisplayLanguage::SimplifiedChinese);
    assert!(prompt.refresh_language().is_err());
    assert_eq!(window_text(hwnd), original_title);
    assert_eq!(ticket.choice(), None);
    unsafe {
        SetWindowLongPtrW(hwnd, USER, identity);
        DestroyWindow(hwnd).unwrap();
    }
    assert!(prompt.refresh_language().is_err());
    assert_eq!(ticket.choice(), None);
    assert_eq!(prompt.state.choice.get(), None);
}

unsafe extern "system" fn retire_after_caption(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    _: usize,
) -> LRESULT {
    let result = DefSubclassProc(hwnd, message, wp, lp);
    if message == WM_SETTEXT {
        SetWindowLongPtrW(hwnd, USER, 0);
    }
    result
}

#[test]
fn caption_refresh_reentry_cannot_write_buttons_after_prompt_identity_changes() {
    let _reset = LanguageReset::english();
    let ticket = ticket();
    let prompt = Prompt::hidden(HWND::default(), ticket.clone()).unwrap();
    let hwnd = prompt.handle().unwrap();
    let buttons = [1, 2].map(|id| unsafe { GetDlgItem(Some(hwnd), id).unwrap() });
    let labels = buttons.map(window_text);
    let identity = unsafe { GetWindowLongPtrW(hwnd, USER) };
    unsafe {
        assert!(SetWindowSubclass(hwnd, Some(retire_after_caption), 1, 0).as_bool());
    }
    ui_text::set_language(DisplayLanguage::SimplifiedChinese);
    let result = prompt.refresh_language();
    let valid = prompt.handle();
    unsafe {
        RemoveWindowSubclass(hwnd, Some(retire_after_caption), 1)
            .ok()
            .unwrap();
        SetWindowLongPtrW(hwnd, USER, identity);
    }
    assert!(result.is_err());
    assert_eq!(valid, None);
    assert_eq!(prompt.language.get(), None);
    assert_eq!(buttons.map(window_text), labels);
    assert_eq!(prompt.state.choice.get(), None);
    assert_eq!(ticket.choice(), None);
}

#[test]
fn confirmation_poll_keeps_runtime_and_ticket_after_label_refresh_failure() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let _reset = LanguageReset::english();
    let (runtime, _grant, probe) = crate::surface_javascript::signed::runtime::tests::start(None);
    let child = probe.recv_timeout(Duration::from_secs(5)).unwrap();
    let mut owner =
        super::super::Window::new(runtime, crate::surface_javascript::fixture::binding());
    let ticket = ticket();
    let prompt = Prompt::hidden(HWND::default(), ticket.clone()).unwrap();
    let hwnd = prompt.handle().unwrap();
    let reject = unsafe { GetDlgItem(Some(hwnd), 2) }.unwrap();
    unsafe {
        SetWindowLongPtrW(reject, GWLP_ID, 3);
    }
    owner.confirmation = Some(prompt);
    ui_text::set_language(DisplayLanguage::SimplifiedChinese);
    owner.poll_confirmation(&|| true, true).unwrap();
    assert_eq!(owner.confirmation_handle(), Some(hwnd));
    assert!(!owner.stopping);
    assert!(!owner.finished());
    assert_eq!(ticket.choice(), None);
    unsafe {
        SetWindowLongPtrW(reject, GWLP_ID, 2);
    }
    owner.poll_confirmation(&|| true, true).unwrap();
    assert_labels(hwnd);
    assert_eq!(ticket.choice(), None);
    drop(owner);
    assert!(child.exited());
}
