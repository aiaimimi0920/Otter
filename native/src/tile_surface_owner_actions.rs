//! 原Views串行调度动作；每次先读新鲜state再至多提交一个意图，冻结/识别只允许取消已有执行。
use super::*;
use crate::tile_surface_action_dispatch as dispatch;
use crate::tile_surface_actions::{pending::Ticket, Command, Kind};

fn unstarted(
    command: Command,
    ticket: &Ticket,
    state: &State,
    job: &Job,
    shared: &Shared,
    defer: bool,
) {
    let now = Instant::now();
    let mut s = shared.lock();
    // 只在权限仍有效且纯prepare/paint空隙时归还；暂停/撤权后即使恢复也不能复活旧意图。
    if defer
        && s.actions.holds(ticket)
        && dispatch::current(&s, job)
        && !s.frozen
        && !s.black
        && !s.identification.blocked()
        && s.until.is_some_and(|v| now < v)
        && s.clock.permits(s.activate_at, now)
        && now < command.until
        && command.edit.as_ref().is_none_or(|e| e.active())
        && !matches!(command.kind, Kind::Cancel { .. })
        && !dispatch::ready(&s, state, job, now)
    {
        s.actions.defer(command, ticket);
    } else {
        s.actions.release(ticket);
        if let Some(edit) = &command.edit {
            s.actions.cancel_edit(edit);
        }
        s.actions.notice = "Art操作已失效";
    }
}
impl Views<'_> {
    pub(super) fn dispatch(&mut self, id: &str, shared: &Shared, job: &Job, cancel_only: bool) {
        let Some(entry) = self.entries.get(id) else {
            return;
        };
        let state = entry.state.clone();
        self.dispatch_state(state, entry.ready.clone(), shared, job, cancel_only);
    }
    fn dispatch_state(
        &self,
        state: Arc<State>,
        art: Option<Arc<Art>>,
        shared: &Shared,
        job: &Job,
        cancel_only: bool,
    ) {
        let command = {
            let mut s = shared.lock();
            if s.stopped
                || s.expired
                || s.generation != job.generation
                || s.desired.as_ref() != Some(&job.key)
            {
                return;
            }
            s.actions.observe(
                state.clone(),
                self.client.origin(),
                self.lease.device_id(),
                Instant::now(),
            );
            let ready = !cancel_only && dispatch::ready(&s, &state, job, Instant::now());
            s.actions.reserve(&state, Instant::now(), ready)
        };
        let Some((mut command, ticket)) = command else {
            return;
        };
        let _sending = command.edit.as_ref().map(|e| e.dispatch_guard());
        if !crate::tile_surface_action_dispatch::validate(
            &mut command,
            &state,
            art.as_ref(),
            job,
            shared,
        ) {
            #[cfg(test)]
            crate::tile_surface_action_dispatch::trace(
                &command,
                &state,
                art.as_ref(),
                shared,
                "validation_rejected",
            );
            unstarted(command, &ticket, &state, job, shared, true);
            return;
        }
        #[cfg(test)]
        tests::prepared(&mut command);
        let Ok(control) = self.lease.control(self.client) else {
            unstarted(command, &ticket, &state, job, shared, false);
            return;
        };
        {
            let s = shared.lock();
            if !s.actions.holds(&ticket) || !dispatch::eligible(&command, &state, job, &s) {
                drop(s);
                unstarted(command, &ticket, &state, job, shared, true);
                return;
            }
            // 此处提交一次HTTP尝试；后续暂停不能撤回已提交字节，但会拒绝迟到结果，绝不重排。
        }
        #[cfg(test)]
        crate::tile_surface_action_dispatch::trace(
            &command,
            &state,
            art.as_ref(),
            shared,
            "validated",
        );
        let result = control.wall_surface_command(&command, &state);
        #[cfg(test)]
        if let Err(error) = &result {
            eprintln!("tile-wall-command-error: {error:?}");
        }
        let result = result.map_err(|_| ());
        #[cfg(test)]
        crate::tile_surface_action_dispatch::trace(
            &command,
            &state,
            art.as_ref(),
            shared,
            if result.is_ok() {
                "http_ack"
            } else {
                "http_failed"
            },
        );
        let mut s = shared.lock();
        if !s.actions.release(&ticket)
            || s.stopped
            || s.expired
            || s.generation != job.generation
            || s.desired.as_ref() != Some(&job.key)
        {
            return;
        }
        match result {
            Ok(ack) => s.actions.attempted(&command, &ack, Instant::now()),
            Err(()) => {
                if let Some(edit) = &command.edit {
                    s.actions.cancel_edit(edit);
                }
                s.actions.failed(&state.view.instance_id);
            }
        }
    }
    pub(crate) fn poll_cancel(&self, shared: &Shared, job: &Job) {
        // 不因取消刷新冻结画布；仅更新纯控制状态。原view由本worker继续唯一拥有。
        for entry in self.entries.values() {
            if !shared
                .lock()
                .actions
                .cancel_for(&entry.state.view, Instant::now())
            {
                continue;
            }
            let Some(control) = self.lease.control(self.client).ok() else {
                return;
            };
            let result = control
                .wall_surface_state(&entry.state.view, entry.state.snapshot.revision)
                .map_err(|_| ())
                .and_then(|bytes| {
                    State::parse(
                        &bytes,
                        &entry.state.view.binding,
                        &entry.state.view.instance_id,
                        Some(&entry.state),
                    )
                    .map_err(|_| ())
                });
            match result {
                Ok(state) => self.dispatch_state(Arc::new(state), None, shared, job, true),
                Err(()) => shared.lock().actions.failed(&entry.state.view.instance_id),
            }
        }
    }
}
#[cfg(test)]
#[path = "tile_surface_dispatch_tests.rs"]
mod tests;
