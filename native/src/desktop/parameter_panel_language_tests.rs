//! Locale refresh must retain real EDIT handles, draft selection, and the original transaction.
use super::*;
use crate::ui_text::{self, DisplayLanguage};

struct LanguageReset(DisplayLanguage);
impl Drop for LanguageReset {
    fn drop(&mut self) {
        ui_text::set_language(self.0);
    }
}

#[test]
fn language_refresh_preserves_draft_selection_and_snapshot_then_commits() {
    let _reset = LanguageReset(ui_text::current_language());
    ui_text::set_language(DisplayLanguage::English);
    unsafe {
        let f = Fixture::new();
        let edit = f.text("text:edit", "Draft 中文🙂");
        let binding = f.binding("text:edit");
        let controls::Kind::Text {
            snapshot, initial, ..
        } = binding.kind
        else {
            panic!("text binding");
        };
        SendMessageW(edit, EM_SETSEL, Some(WPARAM(2)), Some(LPARAM(6)));
        let selection = SendMessageW(edit, EM_GETSEL, None, None);
        let stamp = Stamp::capture(&f.state);
        f.panel.ui.localized_error("parameter-complete-input");
        ui_text::set_language(DisplayLanguage::SimplifiedChinese);
        f.tick();
        assert_eq!(f.binding("text:edit").hwnd, edit);
        assert_eq!(controls::text(edit).as_deref(), Some("Draft 中文🙂"));
        assert_eq!(SendMessageW(edit, EM_GETSEL, None, None), selection);
        assert!(Stamp::capture(&f.state) == stamp);
        let controls::Kind::Text {
            snapshot: current,
            initial: current_initial,
            ..
        } = f.binding("text:edit").kind
        else {
            panic!("text binding");
        };
        assert!(Rc::ptr_eq(&snapshot, &current));
        assert_eq!(initial, current_initial);
        assert_eq!(
            controls::text(f.binding("reload").hwnd).as_deref(),
            Some("重新加载")
        );
        assert_eq!(
            &*f.panel.ui.message.borrow(),
            ui_text::text("parameter-complete-input")
        );
        controls::commit(&f.panel.ui, &f.binding("text:edit"));
        f.tick();
        assert_eq!(f.value("text"), "Draft 中文🙂");
        assert!(f.panel.ui.message.borrow().is_empty());
    }
}

#[test]
fn locale_refresh_does_not_rebase_a_draft_after_real_external_change() {
    let _reset = LanguageReset(ui_text::current_language());
    ui_text::set_language(DisplayLanguage::English);
    unsafe {
        let f = Fixture::new();
        let edit = f.text("text:edit", "Uncommitted");
        f.state
            .selected_art
            .borrow_mut()
            .as_mut()
            .unwrap()
            .parameters
            .set("text", Some(serde_json::json!("External")))
            .unwrap();
        ui_text::set_language(DisplayLanguage::SimplifiedChinese);
        f.tick();
        assert_eq!(f.binding("text:edit").hwnd, edit);
        assert_eq!(controls::text(edit).as_deref(), Some("Uncommitted"));
        assert_eq!(f.value("text"), "External");
        assert_eq!(
            &*f.panel.ui.message.borrow(),
            ui_text::text("parameter-config-draft-conflict")
        );
        assert_eq!(
            controls::text(f.binding("reload").hwnd).as_deref(),
            Some("重新加载")
        );
        controls::commit(&f.panel.ui, &f.binding("text:edit"));
        f.tick();
        assert_eq!(f.value("text"), "External");
    }
}

#[test]
fn inline_locale_refresh_keeps_edit_and_slider_drafts() {
    let _reset = LanguageReset(ui_text::current_language());
    ui_text::set_language(DisplayLanguage::English);
    unsafe {
        let f = Fixture::new();
        let inline = Panel::inline(f.host.0, &f.state, f.panel.handle()).unwrap();
        let bindings = inline.ui.bindings.borrow().clone();
        let edit = bindings.iter().find(|b| b.key == "text:edit").unwrap().hwnd;
        let slider = bindings.iter().find(|b| b.key == "n:slider").unwrap().hwnd;
        SetWindowTextW(edit, &HSTRING::from("Inline draft")).unwrap();
        ui_text::set_language(DisplayLanguage::SimplifiedChinese);
        inline.update(&f.state).unwrap();
        let next = inline.ui.bindings.borrow();
        assert_eq!(
            next.iter().find(|b| b.key == "text:edit").unwrap().hwnd,
            edit
        );
        assert_eq!(
            next.iter().find(|b| b.key == "n:slider").unwrap().hwnd,
            slider
        );
        assert_eq!(controls::text(edit).as_deref(), Some("Inline draft"));
        assert!(inline.ui.message.borrow().is_empty());
    }
}
