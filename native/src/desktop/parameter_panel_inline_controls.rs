//! 28 DIP 胶囊内的真实控件；类型选项/数值/多行对话框仍沿原 Snapshot 和提交事务。
use super::*;
pub(super) unsafe fn build(ui: &Ui) -> Result<()> {
    let model = ui.model.borrow().clone();
    let mut b = Builder {
        ui,
        bindings: Vec::new(),
        placements: Vec::new(),
        y: 4,
        next: 1000,
    };
    for (rank, row) in model.rows.iter().enumerate() {
        b.y = rank as i32 * 36 + 4;
        if row.widget == "sticker-image" {
            if let Some(config) = &model.sticker {
                b.button(
                    "image:upload",
                    config
                        .asset
                        .as_ref()
                        .map_or("上传图片…", |a| a.filename.as_str()),
                    104,
                    96,
                    Kind::Sticker(model.stamp.clone(), desktop::sticker_input::Action::Upload),
                    !config.disabled,
                )?;
            }
            continue;
        }
        let Some(snapshot) = &model.snapshot else {
            continue;
        };
        let Some(&index) = row.primary.first() else {
            continue;
        };
        let choice = &snapshot.choices[index];
        match &choice.action {
            Action::Edit(spec, initial) if !spec.multiline && row.widget != "textarea" => {
                let width = if spec.number { 40 } else { 96 };
                let edit = b.child(
                    w!("EDIT"),
                    initial,
                    WS_TABSTOP | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
                    if spec.number { 160 } else { 104 },
                    width,
                    20,
                )?;
                keyboard::install(edit)?;
                SendMessageW(
                    edit,
                    EM_SETLIMITTEXT,
                    Some(WPARAM(desktop::parameter_value::MAX_TEXT)),
                    None,
                );
                b.bindings.push(Binding {
                    hwnd: edit,
                    key: format!("{}:edit", row.id),
                    kind: Kind::Text {
                        snapshot: snapshot.clone(),
                        index,
                        spec: spec.clone(),
                        initial: initial.clone(),
                    },
                    enabled: choice.enabled,
                });
                if spec.number {
                    // number 和 slider 同样提供滑轨；范围和步长由已有 Spec 唯一解释。
                    if let Some(range) = spec.slider(initial) {
                        let rect = RECT {
                            left: 104,
                            top: b.y,
                            right: 156,
                            bottom: b.y + 20,
                        };
                        let slider = Rc::new(desktop::parameter_slider::Control::create(
                            ui.hwnd.get(),
                            layout::scaled(rect, GetDpiForWindow(ui.hwnd.get())),
                            range,
                        )?);
                        b.placements.push((slider.handle(), rect));
                        b.bindings.push(Binding {
                            hwnd: slider.handle(),
                            key: format!("{}:slider", row.id),
                            kind: Kind::Slider(slider, edit, snapshot.clone(), index),
                            enabled: choice.enabled,
                        });
                    }
                }
            }
            Action::Set(_) if matches!(row.widget.as_str(), "select" | "radio") => {
                let combo = b.child(
                    w!("COMBOBOX"),
                    "",
                    WS_TABSTOP
                        | WS_VSCROLL
                        | WINDOW_STYLE(
                            CBS_DROPDOWNLIST as u32
                                | CBS_OWNERDRAWFIXED as u32
                                | CBS_HASSTRINGS as u32,
                        ),
                    104,
                    96,
                    180,
                )?;
                theme::options::resize(combo)?;
                let mut indices = Vec::new();
                for &index in &row.primary {
                    let option = &snapshot.choices[index];
                    if !matches!(option.action, Action::Set(_)) {
                        continue;
                    }
                    let label = option
                        .label
                        .split_once('：')
                        .map_or(option.label.as_str(), |(_, label)| label);
                    if SendMessageW(
                        combo,
                        CB_ADDSTRING,
                        None,
                        Some(LPARAM(HSTRING::from(label).as_ptr() as isize)),
                    )
                    .0 < 0
                    {
                        return Err(Error::from(E_OUTOFMEMORY));
                    }
                    if option.checked {
                        SendMessageW(combo, CB_SETCURSEL, Some(WPARAM(indices.len())), None);
                    }
                    indices.push(index);
                }
                b.bindings.push(Binding {
                    hwnd: combo,
                    key: format!("{}:options", row.id),
                    kind: Kind::Options(snapshot.clone(), indices),
                    enabled: choice.enabled,
                });
            }
            _ => {
                let label = match &choice.action {
                    Action::Edit(_, value) | Action::Color(value) => model::clean(value),
                    Action::Set(_) => {
                        if choice.checked {
                            "☑".into()
                        } else {
                            "☐".into()
                        }
                    }
                    Action::File => "选择文件…".into(),
                    _ => continue,
                };
                b.button(
                    &format!("{}:value", row.id),
                    &label,
                    104,
                    96,
                    Kind::Choice(snapshot.clone(), index),
                    choice.enabled,
                )?;
            }
        }
    }
    ui.content_height.set(model.rows.len() as i32 * 36);
    for (child, _) in &b.placements {
        inline_input::install(*child)?;
    }
    *ui.placements.borrow_mut() = b.placements;
    *ui.bindings.borrow_mut() = b.bindings;
    Ok(())
}
