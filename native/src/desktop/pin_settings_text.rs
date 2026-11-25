//! Original/translated-text buttons in pin settings; the host owns synchronous copying, target revalidation, and 1.6-second feedback without model execution.
use super::*;
use crate::sticker_extra_text::{self, Kind as TextKind};
use std::time::{Duration, Instant};
pub(in crate::desktop) const OCR: u16 = 109;
pub(in crate::desktop) const TRANSLATION: u16 = 110;
pub(in crate::desktop) const TIMER: usize = 0x6e12;
pub(super) struct Feedback {
    id: u16,
    copied: bool,
    until: Instant,
}
fn kind(id: u16) -> TextKind {
    if id == TRANSLATION {
        TextKind::Translation
    } else {
        TextKind::Ocr
    }
}
pub(super) fn available(state: &State, id: u16) -> bool {
    let d = state.extension_document.borrow();
    sticker_extra_text::text(kind(id), &d.store, d.revision(), &d.legacy_text).is_some()
}
pub(super) fn fields(feedback: Option<&Feedback>) -> Vec<Field> {
    [(OCR, "pin-copy-ocr"), (TRANSLATION, "pin-copy-translation")]
        .into_iter()
        .map(|(id, label)| {
            let feedback = feedback.filter(|f| f.id == id);
            Field {
                id,
                label: crate::ui_text::text(feedback.map_or(label, |f| {
                    if f.copied {
                        "pin-text-copied"
                    } else {
                        "pin-text-copy-failed"
                    }
                })),
                icon: feedback.is_none().then(|| {
                    neuro_mini_ui::Icon::svg(if id == OCR {
                        include_str!("../../assets/edit-strip/SettingsCopyOcr.svg")
                    } else {
                        include_str!("../../assets/edit-strip/SettingsCopyTranslation.svg")
                    })
                }),
                kind: Kind::Action {
                    // Icons preserve the compact paired row; feedback gets its whole text area.
                    value: feedback
                        .map_or("", |f| {
                            crate::ui_text::text(if f.copied {
                                "pin-text-copied"
                            } else {
                                "pin-text-copy-failed-short"
                            })
                        })
                        .into(),
                    active: false,
                    width: 58,
                },
            }
        })
        .collect()
}
pub(super) unsafe fn clear(owner: HWND, session: &mut Session) {
    if session.text_feedback.take().is_some() {
        let _ = KillTimer(Some(owner), TIMER);
    }
}
pub(in crate::desktop) unsafe fn timer(owner: HWND, state: &State) {
    let mut s = state.pin_settings.borrow_mut();
    if s.text_feedback
        .as_ref()
        .is_some_and(|f| Instant::now() < f.until)
    {
        return; // An old WM_TIMER must not prematurely clear feedback from a new copy.
    }
    clear(owner, &mut s);
    let _ = KillTimer(Some(owner), TIMER);
    drop(s);
    refresh(state);
}
pub(super) unsafe fn copy(owner: HWND, state: &State, id: u16) -> Result<()> {
    copy_with(owner, state, id, |text, current| {
        super::super::clipboard::copy_full_text_checked(owner, text, current)
    })
}
pub(in crate::desktop) unsafe fn copy_with(
    owner: HWND,
    state: &State,
    id: u16,
    publish: impl FnOnce(&str, &dyn Fn() -> bool) -> Result<()>,
) -> Result<()> {
    let (text, stamp) = {
        let d = state.extension_document.borrow();
        let Some(text) = sticker_extra_text::text(kind(id), &d.store, d.revision(), &d.legacy_text)
        else {
            return Ok(());
        };
        (text.to_owned(), d.stamp())
    };
    let context = state.pin_settings.borrow().context;
    let current = || {
        super::super::extension_document::observe(state);
        state.lifetime.owns(owner)
            && ready(state)
            && IsWindowVisible(state.tool_panel.get()).as_bool()
            && state.pin_settings.borrow().open
            && state.pin_settings.borrow().context == context
            && stamp.matches(&state.extension_document.borrow())
    };
    if !current() {
        return Ok(());
    }
    let result = publish(&text, &current);
    if !current() {
        return Ok(());
    }
    if let Err(e) = &result {
        eprintln!("native-pin-copy-text: {e}");
    }
    if SetTimer(Some(owner), TIMER, 1600, None) == 0 {
        return Err(Error::from_win32());
    }
    state.pin_settings.borrow_mut().text_feedback = Some(Feedback {
        id,
        copied: result.is_ok(),
        until: Instant::now() + Duration::from_millis(1600),
    });
    Ok(())
}
