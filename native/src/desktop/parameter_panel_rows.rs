//! 每页至多16行：文本/数值原位编辑，类型选项保持原 Value，特殊输入沿原显式对话框。
use super::*;
pub(super) unsafe fn build(
    b: &mut Builder<'_>,
    snapshot: &Rc<Snapshot>,
    row: &model::Row,
) -> Result<()> {
    b.label(&format!(
        "{}{}",
        row.label,
        if row.linked { " · 连线" } else { "" }
    ))?;
    b.y += 24;
    if let Some(&index) = row.primary.first() {
        let choice = &snapshot.choices[index];
        match &choice.action {
            Action::Edit(spec, initial) => {
                let mut spec = spec.clone();
                spec.multiline |= row.widget == "textarea";
                let height = if spec.multiline || row.widget == "textarea" {
                    80
                } else {
                    28
                };
                let style = WS_TABSTOP
                    | WS_BORDER
                    | WINDOW_STYLE(if height == 80 {
                        ES_MULTILINE as u32
                            | ES_AUTOVSCROLL as u32
                            | ES_WANTRETURN as u32
                            | WS_VSCROLL.0
                    } else {
                        ES_AUTOHSCROLL as u32
                    });
                let edit = b.child(
                    w!("EDIT"),
                    initial,
                    style,
                    12,
                    if spec.number { 304 } else { 416 },
                    height,
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
                        initial: text(edit).unwrap_or_else(|| initial.clone()),
                    },
                    enabled: choice.enabled,
                });
                if spec.number {
                    for (up, x, label) in [(false, 324, "−"), (true, 380, "+")] {
                        b.button(
                            &format!("{}:step:{up}", row.id),
                            label,
                            x,
                            48,
                            Kind::Step(edit, spec.clone(), up),
                            choice.enabled,
                        )?;
                    }
                }
                b.y += height + 4;
                if row.widget == "slider" {
                    if let Some(range) = spec.slider(initial) {
                        let rect = layout::field_rect(
                            b.ui,
                            RECT {
                                left: 12,
                                top: b.y,
                                right: 428,
                                bottom: b.y + 28,
                            },
                        );
                        let slider = Rc::new(desktop::parameter_slider::Control::create(
                            b.ui.hwnd.get(),
                            layout::scaled(rect, GetDpiForWindow(b.ui.hwnd.get())),
                            range,
                        )?);
                        // Control owns no HWND lifetime; parent destroys it. Its identity comes from child LPARAM.
                        let current = slider.handle();
                        b.placements.push((current, rect));
                        b.bindings.push(Binding {
                            hwnd: current,
                            key: format!("{}:slider", row.id),
                            kind: Kind::Slider(slider, edit, snapshot.clone(), index),
                            enabled: choice.enabled,
                        });
                        b.y += 32;
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
                    12,
                    416,
                    180,
                )?;
                theme::options::resize(combo)?;
                let mut indices = Vec::new();
                for &option in &row.primary {
                    let choice = &snapshot.choices[option];
                    if !matches!(choice.action, Action::Set(_)) {
                        continue;
                    }
                    let label = model::clean(
                        choice
                            .label
                            .split_once('：')
                            .map_or(choice.label.as_str(), |(_, label)| label),
                    );
                    if SendMessageW(
                        combo,
                        CB_ADDSTRING,
                        None,
                        Some(LPARAM(HSTRING::from(&label).as_ptr() as isize)),
                    )
                    .0 < 0
                    {
                        return Err(Error::new(
                            E_OUTOFMEMORY,
                            "Cannot populate parameter options",
                        ));
                    }
                    if choice.checked {
                        SendMessageW(combo, CB_SETCURSEL, Some(WPARAM(indices.len())), None);
                    }
                    indices.push(option);
                }
                b.bindings.push(Binding {
                    hwnd: combo,
                    key: format!("{}:options", row.id),
                    kind: Kind::Options(snapshot.clone(), indices),
                    enabled: choice.enabled,
                });
                b.y += 32;
            }
            Action::Set(_) => {
                b.button(
                    &format!("{}:boolean", row.id),
                    if choice.checked { "☑ 开" } else { "☐ 关" },
                    12,
                    416,
                    Kind::Choice(snapshot.clone(), index),
                    choice.enabled,
                )?;
                b.y += 32;
            }
            Action::Color(initial) => {
                b.button(
                    &format!("{}:color", row.id),
                    &format!("颜色 · {}", model::clean(initial)),
                    12,
                    416,
                    Kind::Choice(snapshot.clone(), index),
                    choice.enabled,
                )?;
                b.y += 32;
                if let Some(&clear) = row.primary.get(1) {
                    b.button(
                        &format!("{}:clear", row.id),
                        "清空颜色",
                        12,
                        202,
                        Kind::Choice(snapshot.clone(), clear),
                        snapshot.choices[clear].enabled,
                    )?;
                    b.y += 32;
                }
            }
            Action::File => {
                b.button(
                    &format!("{}:file", row.id),
                    "选择文件…",
                    12,
                    416,
                    Kind::Choice(snapshot.clone(), index),
                    choice.enabled,
                )?;
                b.y += 32;
            }
            _ => {}
        }
    }
    if let Some(index) = row.reset {
        b.button(
            &format!("{}:reset", row.id),
            if matches!(row.widget.as_str(), "file" | "image_link") {
                "清空图片"
            } else {
                "恢复默认值"
            },
            12,
            202,
            Kind::Choice(snapshot.clone(), index),
            snapshot.choices[index].enabled,
        )?;
    }
    if let Some(index) = row.disable {
        let choice = &snapshot.choices[index];
        b.button(
            &format!("{}:disable", row.id),
            if choice.checked {
                "启用参数"
            } else {
                "禁用参数"
            },
            226,
            202,
            Kind::Choice(snapshot.clone(), index),
            choice.enabled,
        )?;
    }
    b.y += 40;
    Ok(())
}
