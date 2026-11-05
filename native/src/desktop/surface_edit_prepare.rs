//! 字段提交准备与 click 前缀截止；不拥有第二份队列，不执行发送或结算。
use super::*;
impl Mount {
    pub fn prepare(
        &mut self,
        presentation: &Presentation,
        binding: &Binding,
        revision: u64,
    ) -> Result<Vec<ClickEvent>> {
        self.prepare_at(presentation, binding, revision, std::time::Instant::now())
    }
    pub(in crate::desktop) fn prepare_at(
        &mut self,
        presentation: &Presentation,
        binding: &Binding,
        revision: u64,
        now: std::time::Instant,
    ) -> Result<Vec<ClickEvent>> {
        self.prepare_bound(presentation, binding, revision, now, None)
    }
    pub fn prepare_before(
        &mut self,
        presentation: &Presentation,
        binding: &Binding,
        revision: u64,
        fence: &Fence,
    ) -> Result<Vec<ClickEvent>> {
        self.prepare_bound(
            presentation,
            binding,
            revision,
            std::time::Instant::now(),
            Some(fence),
        )
    }
    fn prepare_bound(
        &mut self,
        presentation: &Presentation,
        binding: &Binding,
        revision: u64,
        now: std::time::Instant,
        until: Option<&Fence>,
    ) -> Result<Vec<ClickEvent>> {
        let nodes = self.preparation_nodes(presentation, binding, now)?;
        while let Some(intent) = self.intents.front().cloned() {
            let intents::Edge::Text(phase) = intent.edge else {
                return Err(Error::new(
                    E_FAIL,
                    "Keyboard input requires JavaScript owner",
                ));
            };
            // 无 handler 的前缀也可能被本次循环耗尽；不得越过 click 的封存边沿。
            if let Some(fence) = until {
                match self.intents.state(fence) {
                    FenceState::Ready | FenceState::InFlight => return Ok(Vec::new()),
                    FenceState::Invalid => {
                        return Err(Error::new(E_FAIL, "Surface edit prefix invalidated"))
                    }
                    FenceState::Pending => {}
                }
            }
            if let Some(field) = self.switches.get_mut(&intent.node) {
                let Some(node) = nodes.get(intent.node.as_str()).filter(|_| !field.disabled) else {
                    self.intents.prepared(None);
                    continue;
                };
                let event = field.prepare(node, binding, revision, &intent.value)?;
                self.intents.prepared(event.as_ref().map(ClickEvent::id));
                if self.intents.has_node(&intent.node) {
                    field.protect()?;
                }
                if let Some(event) = event {
                    return Ok(vec![event]);
                }
                continue;
            }
            if let Some(field) = self.selects.get(&intent.node) {
                let event = nodes
                    .get(intent.node.as_str())
                    .filter(|_| !field.disabled && field.visible)
                    .map(|node| {
                        crate::surface_input_event::encode(
                            node,
                            binding,
                            revision,
                            Phase::Change,
                            crate::surface_input_event::Value::Text(&intent.value),
                        )
                    })
                    .transpose()
                    .map_err(|_| Error::new(E_FAIL, "Select event encoding failed"))?
                    .flatten();
                self.intents.prepared(event.as_ref().map(ClickEvent::id));
                if let Some(event) = event {
                    return Ok(vec![event]);
                }
                continue;
            }
            if let Some(field) = self.sliders.get(&intent.node) {
                let value = intent
                    .value
                    .parse::<f64>()
                    .map_err(|_| Error::new(E_FAIL, "Invalid Slider snapshot"))?;
                let event = nodes
                    .get(intent.node.as_str())
                    .filter(|_| !field.disabled && field.visible)
                    .map(|node| {
                        crate::surface_input_event::encode(
                            node,
                            binding,
                            revision,
                            phase,
                            crate::surface_input_event::Value::Number(Some(value)),
                        )
                    })
                    .transpose()
                    .map_err(|_| Error::new(E_FAIL, "Slider event encoding failed"))?
                    .flatten();
                self.intents.prepared(event.as_ref().map(ClickEvent::id));
                if let Some(event) = event {
                    return Ok(vec![event]);
                }
                continue;
            }
            let Some(field) = self.fields.get_mut(&intent.node) else {
                self.intents.prepared(None);
                continue;
            };
            if field.session.composing() {
                return Ok(Vec::new());
            }
            let Some(node) = nodes.get(intent.node.as_str()).filter(|_| !field.disabled) else {
                self.intents.prepared(None);
                continue;
            };
            let event =
                field
                    .session
                    .prepare_value(node, binding, revision, phase, Some(&intent.value))?;
            self.intents.prepared(event.as_ref().map(ClickEvent::id));
            if self.intents.has_node(&intent.node) {
                field.session.protect_queued()?;
            }
            if let Some(event) = event {
                return Ok(vec![event]);
            }
        }
        Ok(Vec::new())
    }
    pub(super) fn preparation_nodes<'a>(
        &mut self,
        presentation: &'a Presentation,
        binding: &Binding,
        now: std::time::Instant,
    ) -> Result<BTreeMap<&'a str, &'a Node>> {
        if self.binding.as_ref() != Some(binding) || !self.editing {
            return Err(Error::new(E_FAIL, "Surface edit binding inactive"));
        }
        if self.intents.expire(now) {
            for field in self.fields.values_mut() {
                field.changed = false;
            }
            return Err(Error::new(E_FAIL, "Surface edit queue expired"));
        }
        let mut nodes = BTreeMap::new();
        collect(presentation.scene().root(), &mut nodes);
        Ok(nodes)
    }
}
