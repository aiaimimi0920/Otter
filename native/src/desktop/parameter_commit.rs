//! 菜单与 modeless 面板共享同一候选提交事务；文本只允许原 Spec 解析，不能覆盖其它动作。
use super::*;
use crate::desktop;

impl Snapshot {
    /// 重捕获后按稳定参数/动作合同重绑定；JSON pretty 超限等情况可删除前序 choice。
    pub(in crate::desktop) fn corresponding(&self, old: &Self, index: usize) -> Option<usize> {
        let old = old.choices.get(index)?;
        let mut matches = self.choices.iter().enumerate().filter(|(_, new)| {
            new.id == old.id
                && match (&old.action, &new.action) {
                    (Action::Set(a), Action::Set(b))
                    | (Action::ToggleDisabled(a), Action::ToggleDisabled(b)) => a == b,
                    (Action::Reset(a), Action::Reset(b)) => a == b,
                    (Action::Edit(a, _), Action::Edit(b, _)) => a == b,
                    (Action::Color(a), Action::Color(b)) => a == b,
                    (Action::File, Action::File) => true,
                    _ => false,
                }
        });
        let (index, _) = matches.next()?;
        matches.next().is_none().then_some(index)
    }
    pub(in crate::desktop) fn current(&self, state: &State) -> bool {
        self.stamp.matches(state) && desktop::parameter_view::matches(&self.view, state)
    }
    /// 一页草稿先完整解析到候选，再取消旧执行并一次发布；失败不能留下半批参数。
    pub(in crate::desktop) unsafe fn submit_texts(
        &self,
        hwnd: HWND,
        state: &State,
        edits: &[(usize, String)],
    ) -> Result<bool> {
        if edits.is_empty()
            || edits.len() > 16
            || !self.current(state)
            || !IsWindow(Some(hwnd)).as_bool()
            || state.session_end.paused()
            || state.group_hidden.get()
            || state.group_locked.get()
            || !desktop::capture_input::exclusive_available(state)
        {
            return Ok(false);
        }
        let mut candidate = match state.selected_art.borrow().as_ref() {
            Some(selected) => selected.parameters.clone(),
            None => return Ok(false),
        };
        let mut changed = false;
        for (index, text) in edits {
            let Some(choice) = self.choices.get(*index).filter(|choice| choice.enabled) else {
                return Ok(false);
            };
            let Action::Edit(spec, _) = &choice.action else {
                return Ok(false);
            };
            let value = spec
                .parse(text)
                .ok_or_else(|| windows::core::Error::new(E_INVALIDARG, "参数值无效"))?;
            changed |= candidate
                .set(&choice.id, Some(value))
                .map_err(|_| windows::core::Error::new(E_INVALIDARG, "参数值无效"))?;
        }
        if changed {
            desktop::execution_cancel::request(hwnd, state)?;
            state.selected_art.borrow_mut().as_mut().unwrap().parameters = candidate;
            desktop::parameter_view::View::after_local_text(state, &self.stamp);
            desktop::execution_start::parameter_changed(state);
        }
        Ok(true)
    }
    pub(in crate::desktop) unsafe fn submit(
        &self,
        hwnd: HWND,
        state: &State,
        index: usize,
        text: Option<&str>,
    ) -> Result<bool> {
        self.submit_guarded(hwnd, state, index, text, &|| true)
    }
    /// Modeless/child的退休资格须跨同步模态保留；菜单沿原配置凭证路径。
    pub(in crate::desktop) unsafe fn submit_guarded(
        &self,
        hwnd: HWND,
        state: &State,
        index: usize,
        text: Option<&str>,
        owner_current: &dyn Fn() -> bool,
    ) -> Result<bool> {
        let valid = || {
            owner_current()
                && IsWindow(Some(hwnd)).as_bool()
                && !state.session_end.paused()
                && !state.group_hidden.get()
                && !state.group_locked.get()
                && desktop::capture_input::exclusive_available(state)
                && self.current(state)
        };
        if !valid() {
            return Ok(false);
        }
        let Some(choice) = self.choices.get(index).filter(|choice| choice.enabled) else {
            return Ok(false);
        };
        let parsed = match text {
            Some(text) => {
                let Action::Edit(spec, _) = &choice.action else {
                    return Ok(false);
                };
                Some(Some(spec.parse(text).ok_or_else(|| {
                    windows::core::Error::new(E_INVALIDARG, "参数值无效")
                })?))
            }
            None => None,
        };
        if matches!(choice.action, Action::File) {
            return desktop::parameter_file_input::choose(
                hwnd,
                state,
                &choice.id,
                self.stamp.clone(),
                self.view.clone(),
                &valid,
            );
        }
        let mut candidate = match state.selected_art.borrow().as_ref() {
            Some(selected) => selected.parameters.clone(),
            None => return Ok(false),
        };
        let changed = if let Action::ToggleDisabled(fallback) = &choice.action {
            candidate.toggle_disabled(&choice.id, fallback.clone())
        } else {
            let Some(value) = parsed.or_else(|| choice.action.choose(hwnd, state)) else {
                return Ok(false);
            };
            // 模态返回后不沿用旧宿主/配置/有效连线值；无副作用解析也共用此边界。
            if !valid() {
                return Ok(false);
            }
            candidate.set(&choice.id, value)
        }
        .map_err(|_| windows::core::Error::new(E_INVALIDARG, "参数值无效"))?;
        if !changed {
            return Ok(false);
        }
        desktop::execution_cancel::request(hwnd, state)?;
        state.selected_art.borrow_mut().as_mut().unwrap().parameters = candidate;
        desktop::execution_start::parameter_changed(state);
        Ok(true)
    }
}
