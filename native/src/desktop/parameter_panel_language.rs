//! Relabel native controls without recreating edit handles or changing parameter transactions.

use super::*;

#[derive(Clone)]
pub(super) struct Caption {
    id: &'static str,
    arguments: Vec<(&'static str, String)>,
    checked: Option<bool>,
}

impl Caption {
    pub(super) fn plain(id: &'static str) -> Self {
        Self {
            id,
            arguments: Vec::new(),
            checked: None,
        }
    }

    pub(super) fn message(id: &'static str, arguments: &[(&'static str, String)]) -> Self {
        Self {
            id,
            arguments: arguments.to_vec(),
            checked: None,
        }
    }

    pub(super) fn checked(id: &'static str, checked: bool) -> Self {
        Self {
            id,
            arguments: Vec::new(),
            checked: Some(checked),
        }
    }

    pub(super) fn text(&self) -> String {
        let text = if self.arguments.is_empty() {
            crate::ui_text::text(self.id).to_owned()
        } else {
            let arguments = self
                .arguments
                .iter()
                .map(|(id, value)| (*id, value.as_str().into()))
                .collect::<Vec<_>>();
            crate::ui_text::format(self.id, &arguments)
        };
        match self.checked {
            Some(checked) => format!("{} {text}", if checked { "☑" } else { "☐" }),
            None => text,
        }
    }
}

pub(super) unsafe fn refresh(ui: &Ui, state: &State) -> Result<()> {
    let model = ui.model.borrow().clone();
    if ui.language.get() == crate::ui_text::current_language() {
        return Ok(());
    }
    let _redraw = neuro_mini_ui::paint_batch::Hold::new(ui.hwnd.get());
    SetWindowTextW(
        ui.hwnd.get(),
        &HSTRING::from(crate::ui_text::text("parameter-panel-title")),
    )?;
    let captions = ui.captions.borrow().clone();
    for (hwnd, caption) in captions {
        if ui.state().is_none() || ui.closed.get() {
            return Ok(());
        }
        if GetParent(hwnd).ok() == Some(ui.hwnd.get()) {
            SetWindowTextW(hwnd, &HSTRING::from(caption.text()))?;
        }
    }
    if ui.state().is_none() || ui.closed.get() {
        return Ok(());
    }
    if let Some(caption) = ui.message_caption.borrow().clone() {
        *ui.message.borrow_mut() = caption.text();
        ui.status_dirty.set(!ui.inline);
    }
    // Labels in the side rail and candidate cards use this model. Existing controls
    // retain their snapshot, initial value, selection, IME state, and unsaved draft.
    if model.current_content(state) {
        *ui.model.borrow_mut() = Rc::new(ui.capture(state));
    }
    ui.language.set(crate::ui_text::current_language());
    Ok(())
}
