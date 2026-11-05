//! Enter 边沿复用原 Session/ticket/FIFO；默认 change 只替换原键盘槽，不重放系统消息。
use super::*;
impl Mount {
    pub(crate) fn javascript_enter(
        &mut self,
        hwnd: HWND,
        flags: u32,
        focus_epoch: u64,
    ) -> Result<()> {
        if flags > 127 || flags & 3 == 3 {
            return Err(Error::new(E_INVALIDARG, "Invalid native Enter edge"));
        }
        let active = self.editing
            && self.fields.values().any(|field| {
                field.kind == Kind::Input
                    && !field.disabled
                    && !field.session.composing()
                    && field.session.control().hwnd().ok() == Some(hwnd)
            });
        if !active {
            return Ok(());
        }
        // 读取最终草稿仍由原通知入口拥有；通常已有 EDIT_END，此调用不会手造重复 input。
        self.notify(hwnd, WM_COMMAND, EN_CHANGE)?;
        let (node, field) = self
            .fields
            .iter()
            .find(|(_, field)| field.session.control().hwnd().ok() == Some(hwnd))
            .ok_or_else(|| Error::new(E_FAIL, "Native keyboard field became unavailable"))?;
        if !self.intents.push_keyboard(
            node,
            field.session.value(),
            flags,
            focus_epoch,
            if field.changed {
                field.change_epoch
            } else {
                field.committed_epoch
            },
        ) {
            return Err(Error::new(E_FAIL, "Surface keyboard queue full"));
        }
        Ok(())
    }
    pub(crate) fn complete_javascript(
        &mut self,
        id: &str,
        allowed: Option<bool>,
        focus: Option<(HWND, u64)>,
    ) -> Result<()> {
        let Some(intent) = self.intents.prepared_intent(id).cloned() else {
            return Ok(());
        };
        let mut change = None;
        match intent.edge {
            intents::Edge::Text(_) | intents::Edge::Pointer(_) | intents::Edge::Hover(_)
                if allowed.is_some() =>
            {
                return Err(Error::new(E_FAIL, "Unexpected keyboard result"))
            }
            intents::Edge::Keyboard {
                flags,
                focus_epoch,
                change_epoch,
            } => {
                let allowed =
                    allowed.ok_or_else(|| Error::new(E_FAIL, "Missing keyboard result"))?;
                if allowed
                    && flags & 3 == 0
                    && self.editing
                    && self.intents.default_current(&intent)
                {
                    if let Some(field) = self.fields.get_mut(&intent.node) {
                        if !field.disabled
                            && !field.session.composing()
                            && change_epoch != u64::MAX
                            && change_epoch > field.committed_epoch
                            && focus_epoch != u64::MAX
                            && focus.is_some_and(|(hwnd, epoch)| {
                                epoch == focus_epoch
                                    && field.session.control().hwnd().ok() == Some(hwnd)
                            })
                        {
                            field.committed_epoch = change_epoch;
                            field.changed = field.change_epoch > field.committed_epoch;
                            // 用已接纳 listener 帧的权威值，不能回写旧键盘快照覆盖程序改值。
                            change = Some(field.session.authoritative_value().to_owned());
                        }
                    }
                }
            }
            _ => {}
        }
        self.intents.complete_local(id, change.as_deref());
        Ok(())
    }
}
