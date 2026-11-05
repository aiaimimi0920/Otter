//! JS 输入沿原 HWND 候选身份和 batch owner；一个 listener/emit 批次未消费前不接第二次点击。
use super::*;
impl AuthorizedWindow {
    pub(in crate::desktop::surface_authorized_window) fn validate_javascript_input_viewport(
        &mut self,
    ) -> Result<()> {
        if self
            .javascript
            .as_ref()
            .and_then(|js| js.events_viewport)
            .is_some_and(|view| {
                self.window.viewport().ok() != Some(view)
                    || self.javascript.as_ref().unwrap().events_viewport_epoch
                        != Some(self.window.viewport_epoch())
            })
        {
            self.fail_javascript("JS prepared input viewport became stale".into())?;
            return Err(Error::new(
                E_FAIL,
                "JS prepared input viewport became stale",
            ));
        }
        Ok(())
    }
    pub(in crate::desktop::surface_authorized_window) fn javascript_input_idle(&self) -> bool {
        self.javascript_ready()
            && self
                .javascript
                .as_ref()
                .is_none_or(|js| js.events.is_none() && js.presentation.is_none())
    }
    pub(in crate::desktop::surface_authorized_window) fn javascript_has_input(&self) -> bool {
        self.javascript
            .as_ref()
            .is_some_and(|js| js.events.is_some())
            || (self.javascript_input_idle()
                && (self.window.has_pending_click()
                    || self.window.has_pending_edit()
                    || self.deferred_click.is_some()))
    }
    pub(crate) fn prepared_input_fits(&self, limit: usize) -> bool {
        self.javascript
            .as_ref()
            .and_then(|js| js.events.as_ref())
            .is_none_or(|events| events.len() <= limit)
    }
    pub(in crate::desktop::surface_authorized_window) fn take_javascript_input(
        &mut self,
        now: u64,
    ) -> Result<Option<Vec<crate::surface_click_event::ClickEvent>>> {
        self.refresh(now)?;
        if self.stopped.is_some()
            || !self.is_shown()
            || !self.javascript_ready()
            || !self.lifecycle.owner().is_some_and(Owner::confirmed_active)
        {
            return Ok(None);
        }
        if self.javascript.as_ref().unwrap().events.is_some() {
            self.validate_javascript_input_viewport()?;
        }
        if let Some(events) = self.javascript.as_mut().unwrap().events.take() {
            return Ok(Some(events));
        }
        let current = self.javascript_mount()?;
        if self.deferred_click.is_none() {
            if let Some(click) = self.window.take_click() {
                self.deferred_click = Some(self.window.defer_click(
                    click,
                    &current.binding,
                    self.snapshot_revision,
                )?);
            }
        }
        if let Some(pending) = &self.deferred_click {
            use crate::desktop::surface_edit_mount::FenceState;
            match self.window.deferred_state(pending) {
                FenceState::Invalid => {
                    self.deferred_click.take();
                    return Err(Error::new(
                        E_FAIL,
                        "JS click dependency invalidated or expired",
                    ));
                }
                FenceState::InFlight => return Ok(None),
                FenceState::Pending | FenceState::Ready => {}
            }
        }
        if let Some(prepared) = self
            .window
            .prepare_javascript_edit(&current.binding, self.deferred_click.as_ref())?
        {
            let submitted = (|| {
                self.refresh(now)?;
                if !self.javascript_input_idle()
                    || !self.is_shown()
                    || self.binding() != &current.binding
                {
                    return Err(Error::new(E_FAIL, "JS text input became stale"));
                }
                self.window.set_submission_enabled(false)?;
                let viewport = self.capture_javascript_viewport()?;
                let js = self.javascript.as_mut().unwrap();
                js.local_pointer = matches!(
                    prepared.input,
                    crate::surface_javascript_wire::LocalInput::Pointer(_)
                        | crate::surface_javascript_wire::LocalInput::Hover(_)
                )
                .then_some(viewport);
                js.runtime
                    .as_mut()
                    .unwrap()
                    .local_input(current, prepared.input, viewport)
                    .map_err(|error| Error::new(E_FAIL, error.to_string()))?;
                js.local_input = Some(prepared.id);
                Ok(())
            })();
            if let Err(error) = submitted {
                // 邮箱唤醒失败可能已交付命令；销毁自有会话，不能把 ticket 归队后重放。
                self.fail_javascript("JS text input submission failed".into())?;
                return Err(error);
            }
            return Ok(None);
        }
        let Some(pending) = self.deferred_click.as_ref() else {
            return Ok(None);
        };
        if self.window.deferred_state(pending)
            != crate::desktop::surface_edit_mount::FenceState::Ready
        {
            return Ok(None);
        }
        let target = pending.target().to_owned();
        // 与声明式点击共用场景身份、命中和键盘焦点门；DOM 不得直接声明远端动作。
        if !self
            .window
            .prepare_deferred(pending, &current.binding, self.snapshot_revision)?
            .is_empty()
        {
            return Err(Error::new(
                E_FAIL,
                "JS DOM cannot directly submit declarative actions",
            ));
        }
        self.deferred_click.take();
        self.refresh(now)?;
        if !self.javascript_input_idle() || !self.is_shown() || self.binding() != &current.binding {
            return Err(Error::new(E_FAIL, "JS click became stale"));
        }
        self.window.set_submission_enabled(false)?;
        let viewport = self.capture_javascript_viewport()?;
        self.javascript
            .as_mut()
            .unwrap()
            .runtime
            .as_mut()
            .unwrap()
            .click(current, target, viewport)
            .map_err(|error| Error::new(E_FAIL, error.to_string()))?;
        Ok(None)
    }
    pub(super) fn accept_javascript_frame(
        &mut self,
        frame: crate::surface_javascript::Frame,
    ) -> Result<()> {
        // 后台只读 operation 开始的视口；resize/DPI 竞态不重跑 listener，也不提交过期尺寸。
        if self
            .javascript
            .as_ref()
            .unwrap()
            .local_pointer
            .or(frame.viewport)
            .is_some_and(|view| {
                self.window.viewport().ok() != Some(view)
                    || self.javascript.as_ref().unwrap().submitted_viewport_epoch
                        != self.window.viewport_epoch()
            })
        {
            return Err(Error::new(E_FAIL, "JS layout viewport became stale"));
        }
        let js = self.javascript.as_ref().unwrap();
        if !frame.events.is_empty()
            && (js.synced.is_none()
                || js.events.is_some()
                || frame.events.len() > 32
                || frame
                    .events
                    .iter()
                    .map(|event| event.bytes().len())
                    .sum::<usize>()
                    > crate::surface_click_event::MAX_EVENT_BYTES)
        {
            return Err(Error::new(E_FAIL, "JS emit batch cannot be admitted"));
        }
        let images = frame
            .images
            .into_iter()
            .map(|(id, image)| Ok((id, crate::surface_image_paint::Bitmap::new(image)?)))
            .collect::<Result<_>>()?;
        self.window.replace_javascript_frame(frame.scene, images)?;
        if self
            .javascript
            .as_ref()
            .unwrap()
            .local_pointer
            .or(frame.viewport)
            .is_some_and(|view| {
                self.window.viewport().ok() != Some(view)
                    || self.javascript.as_ref().unwrap().submitted_viewport_epoch
                        != self.window.viewport_epoch()
            })
        {
            return Err(Error::new(
                E_FAIL,
                "JS layout viewport changed during presentation",
            ));
        }
        let local_input = self.javascript.as_mut().unwrap().local_input.take();
        let pointer_viewport = self.javascript.as_mut().unwrap().local_pointer.take();
        if let Some(id) = &local_input {
            self.window.settle_javascript_ticket(id)?;
        }
        // 回复仅含意图；过期/失焦/后台/撤权不重试，也不能绕过真实 change/FIFO。
        let input_authorized = self.is_shown()
            && self.stopped.is_none()
            && self.lifecycle.owner().is_some_and(Owner::confirmed_active);
        if input_authorized {
            self.lease
                .validate(
                    self.binding().origin(),
                    crate::desktop::surface_windows::now_ms()?,
                )
                .map_err(|_| Error::new(E_FAIL, "JS focus authorization expired"))?;
            self.window.apply_javascript_focus(
                &frame.focus,
                self.javascript.as_ref().unwrap().submitted_focus_epoch,
            )?;
        }
        if let Some(id) = local_input {
            // blur 的真实失焦先进入原 FIFO，再核焦点/epoch；取消或失效默认不归队重放。
            self.window.complete_javascript_edit(
                &id,
                frame
                    .default_allowed
                    .map(|allowed| allowed && input_authorized),
            )?;
        } else if frame.default_allowed.is_some() {
            return Err(Error::new(E_FAIL, "Unsolicited JavaScript keyboard result"));
        }
        let js = self.javascript.as_mut().unwrap();
        if let Some((resources, revision)) = js.staged.take() {
            self.snapshot_revision = revision;
            *self.resources.get_mut() = Some(resources);
            js.runtime
                .as_ref()
                .unwrap()
                .hold_notification(true)
                .map_err(|error| Error::new(E_FAIL, error.to_string()))?;
        }
        if !frame.events.is_empty() {
            js.events = Some(frame.events);
            js.events_viewport = frame.viewport.or(pointer_viewport);
            js.events_viewport_epoch = js.events_viewport.map(|_| js.submitted_viewport_epoch);
        }
        self.accept_javascript_animation(
            frame.animation_pending,
            frame.resize_observed,
            frame.timer_delay_ms,
        )?;
        Ok(())
    }
}
