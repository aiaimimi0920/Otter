//! Language refresh mutates HWND presentation only, including retained dynamic button states.
use super::*;
use crate::ui_text::{text_for, DisplayLanguage};

unsafe fn caption(hwnd: HWND) -> String {
    let mut text = [0; 128];
    let count = GetWindowTextW(hwnd, &mut text) as usize;
    String::from_utf16(&text[..count]).unwrap()
}

unsafe fn assert_labels(f: &Fixture, language: DisplayLanguage) {
    let hwnd = f.hwnd();
    assert_eq!(caption(hwnd), text_for(language, "share-title"));
    for (id, key) in layout::FIXED_LABELS.into_iter().chain([
        (1, f.popup.model.primary_label.get()),
        (108, f.popup.model.action_label.get()),
    ]) {
        let control = GetDlgItem(Some(hwnd), id).unwrap();
        assert_eq!(caption(control), text_for(language, key));
        ValidateRect(Some(control), None).ok().unwrap();
    }
    ValidateRect(Some(hwnd), None).ok().unwrap();
    refresh_language(&f.popup.model, language).unwrap();
    assert!(!GetUpdateRect(hwnd, None, false).as_bool());
    for (id, _) in layout::FIXED_LABELS.into_iter().chain([(1, ""), (108, "")]) {
        assert!(!GetUpdateRect(GetDlgItem(Some(hwnd), id).unwrap(), None, false).as_bool());
    }
}

#[test]
fn language_refresh_retains_qr_output_pending_creation_and_copy_state() {
    unsafe {
        let f = Fixture::new(false, true);
        f.view.borrow_mut().account.bindings.push(invitation());
        f.popup.refresh();
        let hwnd = f.hwnd();
        let m = &f.popup.model;
        let link = m.link.borrow().clone();
        let envelope = m.account_envelope.borrow().clone();
        let colors = m.code.borrow().as_ref().unwrap().colors.as_ptr();
        let epoch = m.input_epoch.get();
        let cancel = Arc::new(AtomicBool::new(false));
        *m.decode_cancel.borrow_mut() = Some(cancel.clone());
        m.create_pending.set(true);
        m.set_status("existing event snapshot");
        label(m, 1, "share-copied");
        label(m, 108, "share-unavailable");
        for language in [DisplayLanguage::SimplifiedChinese, DisplayLanguage::English] {
            refresh_language(m, language).unwrap();
            assert_labels(&f, language);
            assert_eq!(f.hwnd(), hwnd);
            assert_eq!(m.link.borrow().as_str(), link);
            assert!(m.account_envelope.borrow().as_ref() == envelope.as_ref());
            assert_eq!(m.code.borrow().as_ref().unwrap().colors.as_ptr(), colors);
            assert_eq!(m.input_epoch.get(), epoch);
            assert!(m.create_pending.get());
            assert!(!cancel.load(Ordering::Acquire));
            assert_eq!(&*m.status.borrow(), "existing event snapshot");
            assert!(f.view.borrow().request.is_none());
            assert_eq!(f.view.borrow().account.bindings.len(), 1);
        }
    }
}

#[test]
fn language_refresh_preserves_receiver_draft_and_import_enablement() {
    unsafe {
        let f = Fixture::new(true, false);
        let link = invitation().envelope().link().unwrap();
        f.popup.model.parsed(&link);
        let hwnd = f.hwnd();
        let edit = GetDlgItem(Some(hwnd), 101).unwrap();
        let m = &f.popup.model;
        let epoch = m.input_epoch.get();
        let envelope = m.account_envelope.borrow().clone();
        let inputs = m.inputs.borrow().clone();
        for language in [DisplayLanguage::SimplifiedChinese, DisplayLanguage::English] {
            refresh_language(m, language).unwrap();
            assert_labels(&f, language);
            assert_eq!(f.hwnd(), hwnd);
            assert_eq!(GetDlgItem(Some(hwnd), 101).unwrap(), edit);
            assert_eq!(f.text(), link);
            assert_eq!(*m.inputs.borrow(), inputs);
            assert!(m.account_envelope.borrow().as_ref() == envelope.as_ref());
            assert_eq!(m.input_epoch.get(), epoch);
            assert!(IsWindowEnabled(GetDlgItem(Some(hwnd), 1).unwrap()).as_bool());
            assert!(!m.account_import.get());
            assert!(f.view.borrow().request.is_none());
        }
    }
}
