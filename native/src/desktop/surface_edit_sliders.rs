//! Slider 挂载与共享 FIFO 接缝；无文本 draft/表单值，键盘 input 前预留 commit 槽。
use super::*;
use crate::desktop::surface_slider_control::{Control, NOTIFY};
#[derive(PartialEq)]
struct Placement {
    origin: [f32; 2],
    size: [f32; 2],
    clip: crate::surface_clip::Rect,
    dpi: u32,
}
pub(super) struct Field {
    pub control: Control,
    contract: [u8; 32],
    source: f64,
    accepted: f64,
    committed: f64,
    reported: f64,
    reported_readonly: bool,
    pub disabled: bool,
    pub visible: bool,
    placement: Option<Placement>,
}
impl Mount {
    pub(in crate::desktop) fn slider_readonly(&self, id: &str) -> Option<bool> {
        self.sliders
            .get(id)
            .map(|field| !self.editing || field.disabled)
    }
    pub(in crate::desktop) fn take_slider_readonly_changes(&mut self) -> Vec<(String, bool, bool)> {
        self.sliders
            .iter_mut()
            .filter_map(|(id, field)| {
                let after = !self.editing || field.disabled;
                if field.reported_readonly == after {
                    return None;
                }
                let before = std::mem::replace(&mut field.reported_readonly, after);
                Some((id.clone(), before, after))
            })
            .collect()
    }
    pub(in crate::desktop) fn slider_value(&self, id: &str) -> Option<f64> {
        self.sliders.get(id).map(|field| field.accepted)
    }
    pub(in crate::desktop) fn take_slider_changes(&mut self) -> Vec<(String, f64, f64)> {
        // 最多 64 个字段；只读已接纳值，不扫描 HWND 或创建第二份待发送队列。
        self.sliders
            .iter_mut()
            .filter_map(|(id, field)| {
                if field.reported == field.accepted {
                    return None;
                }
                let before = std::mem::replace(&mut field.reported, field.accepted);
                Some((id.clone(), before, field.accepted))
            })
            .collect()
    }
    pub(in crate::desktop) fn slider_range(
        &self,
        id: &str,
    ) -> Result<(crate::surface_slider::Range, f64, bool)> {
        let field = self
            .sliders
            .get(id)
            .ok_or_else(|| Error::new(E_FAIL, "Slider unavailable"))?;
        Ok((
            field.control.range()?,
            field.control.value()?,
            !self.editing || field.disabled,
        ))
    }
    pub(in crate::desktop) fn commit_slider_range(&mut self, id: &str, value: f64) -> Result<()> {
        use windows::Win32::UI::{Accessibility::*, Input::KeyboardAndMouse::GetCapture};
        let field = self
            .sliders
            .get_mut(id)
            .ok_or_else(|| Error::new(E_FAIL, "Slider unavailable"))?;
        if !self.editing || field.disabled || !field.visible {
            return Err(Error::from_hresult(windows::core::HRESULT(
                UIA_E_ELEMENTNOTENABLED as i32,
            )));
        }
        let range = field.control.range()?;
        if !value.is_finite() || value < range.min() || value > range.max() {
            return Err(Error::from_hresult(E_INVALIDARG));
        }
        let next = range
            .normalize(value)
            .map_err(|_| Error::from_hresult(E_INVALIDARG))?;
        if next == field.control.value()? {
            return Ok(());
        }
        if unsafe { GetCapture() } == field.control.hwnd()? || !self.intents.can_push_step(id) {
            return Err(Error::from_hresult(windows::core::HRESULT(
                UIA_E_INVALIDOPERATION as i32,
            )));
        }
        // 同一 UI owner 内已排除本控件捕获，set_value 不会 ReleaseCapture/通知重入。
        field.control.set_value(next)?;
        let snapshot = next.to_string();
        let input = self.intents.push(id, Phase::Input, &snapshot);
        let change = self.intents.push(id, Phase::Change, &snapshot);
        debug_assert!(input && change, "range pair reserved by single UI owner");
        field.accepted = next;
        field.committed = next;
        Ok(())
    }
    pub(in crate::desktop) fn cancel_sliders(&self) {
        for field in self.sliders.values() {
            let _ = field.control.cancel();
        }
    }
    pub(super) fn sync_sliders(
        &mut self,
        parent: HWND,
        presentation: &Presentation,
        nodes: &BTreeMap<&str, &Node>,
        contracts: &BTreeMap<&str, [u8; 32]>,
        dpi: u32,
    ) -> Result<()> {
        let old = self.sliders.len();
        self.sliders.retain(|id, field| {
            nodes.get(id.as_str()).is_some_and(|node| {
                node.kind() == Kind::Slider && contracts.get(id.as_str()) == Some(&field.contract)
            })
        });
        self.fields_changed |= old != self.sliders.len();
        // 在创建同 ID 新字段前清旧合同，且不能继承别种字段的意图。
        self.intents.retain(|id| {
            if nodes
                .get(id)
                .is_some_and(|node| node.kind() == Kind::Slider)
            {
                self.sliders.contains_key(id)
            } else {
                self.fields.contains_key(id)
                    || self.selects.contains_key(id)
                    || self.switches.contains_key(id)
            }
        });
        let overlays = presentation.scrollbars();
        for (index, geometry) in presentation.geometry().nodes().iter().enumerate() {
            let Some(node) = nodes
                .get(geometry.id())
                .filter(|node| node.kind() == Kind::Slider)
            else {
                continue;
            };
            if geometry.size().iter().any(|v| *v < 1.) {
                self.fields_changed |= self.sliders.remove(node.id()).is_some();
                continue;
            }
            let range = crate::surface_slider::Range::parse(node)
                .map_err(|_| Error::new(E_INVALIDARG, "Invalid Slider range"))?;
            let source = range.source();
            if !self.sliders.contains_key(node.id()) {
                let control = Control::new(
                    parent,
                    RECT {
                        left: 0,
                        top: 0,
                        right: 1,
                        bottom: 1,
                    },
                    range,
                )?;
                self.sliders.insert(
                    node.id().to_owned(),
                    Field {
                        control,
                        contract: contracts[node.id()],
                        source,
                        accepted: source,
                        committed: source,
                        reported: source,
                        reported_readonly: !self.editing
                            || node.props().get("disabled") == Some(&serde_json::Value::Bool(true)),
                        disabled: false,
                        visible: false,
                        placement: None,
                    },
                );
                self.fields_changed = true;
            }
            let field = self.sliders.get_mut(node.id()).unwrap();
            if source != field.source {
                field.control.set_value(source)?;
                field.source = source;
                field.accepted = source;
                field.committed = source;
            }
            field.disabled = node.props().get("disabled") == Some(&serde_json::Value::Bool(true));
            let placement = Placement {
                origin: geometry.origin(),
                size: geometry.size(),
                clip: geometry.clip(),
                dpi,
            };
            let changed = field.placement.as_ref() != Some(&placement);
            if changed {
                field.control.cancel()?;
            }
            let hwnd = field.control.hwnd()?;
            field.visible = if changed {
                surface_edit_geometry::place(
                    hwnd,
                    geometry,
                    &presentation.geometry().nodes()[index + 1..],
                    &overlays,
                    dpi,
                    surface_edit_geometry::Part::Whole,
                )?
            } else {
                surface_edit_geometry::refresh_slider(
                    hwnd,
                    geometry,
                    &presentation.geometry().nodes()[index + 1..],
                    &overlays,
                    dpi,
                )?
            };
            field.placement = Some(placement);
            field
                .control
                .set_enabled(self.editing && !field.disabled && field.visible)?;
            field.control.show()?;
            if field.disabled || !field.visible {
                self.intents.retain(|id| id != node.id());
            }
        }
        Ok(())
    }
    pub(super) fn notify_slider(&mut self, hwnd: HWND, message: u32, code: u32) -> Result<bool> {
        let Some((id, field)) = self
            .sliders
            .iter_mut()
            .find(|(_, f)| f.control.hwnd().ok() == Some(hwnd))
        else {
            return Ok(false);
        };
        if message != NOTIFY || code > 1 {
            return Ok(true);
        }
        if !self.editing || field.disabled || !field.visible {
            field.control.set_value(field.accepted)?;
            return Ok(true);
        }
        let value = field.control.value()?;
        let phase = if code == 0 {
            Phase::Input
        } else {
            Phase::Change
        };
        if (code == 0 && value == field.accepted) || (code == 1 && value == field.committed) {
            return Ok(true);
        }
        if (code == 0 && !self.intents.can_push_step(id))
            || (code == 1 && !self.intents.can_push_change())
        {
            field.control.set_value(field.accepted)?;
            return Err(Error::new(E_FAIL, "Surface Slider queue full"));
        }
        let pushed = self.intents.push(id, phase, &value.to_string());
        debug_assert!(pushed);
        field.accepted = value;
        if code == 1 {
            field.committed = value;
        }
        Ok(true)
    }
}
