//! Select 挂载/离散选择接缝；不建 draft，不加入表单值，复用共享事件 FIFO。
use super::*;
use crate::desktop::surface_select_control::Control;
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
    source: String,
    accepted: Option<usize>,
    pub disabled: bool,
    pub visible: bool,
    placement: Option<Placement>,
}
impl Mount {
    fn retain_select_intents(&mut self, nodes: &BTreeMap<&str, &Node>) {
        self.intents.retain(|id| {
            if nodes
                .get(id)
                .is_some_and(|node| node.kind() == Kind::Select)
            {
                self.selects.contains_key(id)
            } else {
                self.fields.contains_key(id)
                    || self.switches.contains_key(id)
                    || self.sliders.contains_key(id)
            }
        });
    }
    pub(super) fn sync_selects(
        &mut self,
        parent: HWND,
        presentation: &Presentation,
        nodes: &BTreeMap<&str, &Node>,
        contracts: &BTreeMap<&str, [u8; 32]>,
        dpi: u32,
    ) -> Result<()> {
        let previous_count = self.selects.len();
        self.selects.retain(|id, field| {
            nodes.get(id.as_str()).is_some_and(|node| {
                node.kind() == Kind::Select && contracts.get(id.as_str()) == Some(&field.contract)
            })
        });
        self.fields_changed |= self.selects.len() != previous_count;
        // 先撤销旧身份队列，再创建同 ID 新控件，避免旧 value 改投新 action。
        self.retain_select_intents(nodes);
        let overlays = presentation.scrollbars();
        for (index, geometry) in presentation.geometry().nodes().iter().enumerate() {
            let Some(node) = nodes
                .get(geometry.id())
                .filter(|node| node.kind() == Kind::Select)
            else {
                continue;
            };
            if geometry.size().iter().any(|v| *v < 1.) {
                self.fields_changed |= self.selects.remove(node.id()).is_some();
                continue;
            }
            let source = node
                .props()
                .get("value")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            if !self.selects.contains_key(node.id()) {
                let options = crate::surface_select::Options::parse(node)
                    .map_err(|_| Error::new(E_INVALIDARG, "Invalid select options"))?;
                let accepted = options.source_index();
                let control = Control::new(
                    parent,
                    RECT {
                        left: 0,
                        top: 0,
                        right: 1,
                        bottom: 160,
                    },
                    options,
                )?;
                self.selects.insert(
                    node.id().to_owned(),
                    Field {
                        control,
                        contract: contracts[node.id()],
                        source: source.to_owned(),
                        accepted,
                        disabled: false,
                        visible: false,
                        placement: None,
                    },
                );
                self.fields_changed = true;
            }
            let field = self.selects.get_mut(node.id()).unwrap();
            if source != field.source {
                let options = crate::surface_select::Options::parse(node)
                    .map_err(|_| Error::new(E_INVALIDARG, "Invalid select source"))?;
                field.control.close_popup()?;
                field.control.select(options.source_index())?;
                field.source = source.to_owned();
                field.accepted = options.source_index();
            }
            field.disabled = node.props().get("disabled") == Some(&serde_json::Value::Bool(true));
            let placement = Placement {
                origin: geometry.origin(),
                size: geometry.size(),
                clip: geometry.clip(),
                dpi,
            };
            // 不依赖不同控件版本的 SetWindowPos 副作用；真实锚点/裁剪变化先收起。
            let changed = field.placement.as_ref() != Some(&placement);
            if changed {
                field.control.close_popup()?;
            }
            let font = presentation
                .control_font(node.id())
                .ok_or_else(|| Error::new(E_FAIL, "Missing Select resolved font"))?;
            field.control.set_font(dpi, font)?;
            field
                .control
                .set_color(presentation.select_ink(node.id()))?;
            field.visible = if changed {
                surface_edit_geometry::place(
                    field.control.hwnd()?,
                    geometry,
                    &presentation.geometry().nodes()[index + 1..],
                    &overlays,
                    dpi,
                    surface_edit_geometry::Part::Select,
                )?
            } else {
                surface_edit_geometry::refresh_select(
                    field.control.hwnd()?,
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
    pub(super) fn notify_select(&mut self, hwnd: HWND, message: u32, code: u32) -> Result<bool> {
        let Some((id, field)) = self
            .selects
            .iter_mut()
            .find(|(_, field)| field.control.hwnd().ok() == Some(hwnd))
        else {
            return Ok(false);
        };
        // v6 UIA SelectionItem 可改变真实索引而不发 CBN_SELCHANGE。
        // 收起时核对最终索引，仍由 accepted 去重；不合成系统通知或绕过 FIFO。
        if message != WM_COMMAND || !matches!(code, CBN_SELCHANGE | CBN_CLOSEUP) {
            return Ok(true);
        }
        if !self.editing || field.disabled || !field.visible {
            field.control.select(field.accepted)?;
            return Ok(true);
        }
        let selected = field.control.selected_index()?;
        if selected == field.accepted {
            return Ok(true);
        }
        let Some(value) = field.control.value()?.map(str::to_owned) else {
            return Ok(true);
        };
        if !self.intents.can_push_change() {
            field.control.select(field.accepted)?;
            return Err(Error::new(E_FAIL, "Surface select queue full"));
        }
        let pushed = self.intents.push(id, Phase::Change, &value);
        debug_assert!(pushed, "single UI owner reserved select change capacity");
        field.accepted = selected;
        Ok(true)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn surface_select_mount_kind_replacement_cannot_inherit_switch_intent() {
        let old = crate::surface_scene::Scene::parse(br#"{"id":"same","type":"switch"}"#).unwrap();
        let next = crate::surface_scene::Scene::parse(br#"{"id":"same","type":"select"}"#).unwrap();
        let mut mount = Mount::default();
        mount.switches.insert(
            "same".to_owned(),
            switch_session::Session::new(old.root(), [0; 32]).unwrap(),
        );
        assert!(mount.intents.push("same", Phase::Change, "true"));
        let mut nodes = BTreeMap::new();
        collect(next.root(), &mut nodes);
        mount.retain_select_intents(&nodes);
        assert!(mount.intents.front().is_none());
    }
}
