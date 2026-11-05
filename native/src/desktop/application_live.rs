//! 应用持有 Live 会话：回调外创建、每次消息后结算、真实线程句柄与 UI 消息共同等待。
use super::*;
use crate::{desktop::composition::live::session::Session, live_job::MAX_SESSIONS};
use std::os::windows::io::AsRawHandle;
#[path = "application_live_freeze.rs"]
pub(super) mod freeze;
#[path = "application_live_groups.rs"]
mod groups;
#[path = "application_live_recycle.rs"]
mod recycle;
#[path = "application_live_shutdown.rs"]
mod shutdown;
#[path = "application_live_source.rs"]
mod source;
#[path = "application_surface_journal.rs"]
mod surface_journal;
#[path = "application_surface_shutdown.rs"]
mod surface_shutdown;
#[path = "application_live_wait.rs"]
mod wait;
#[path = "application_live_workspace.rs"]
mod workspace;
// 每个远程会话是 Ready+完成或帧+完成，最多两个句柄；消息等待必须保留一个槽。
const MAX_WAIT_HANDLES: usize =
    MAX_SESSIONS * 3 + 7 + crate::desktop::composition::relay::sessions::MAX_SESSIONS * 2;
const _: () = assert!(MAX_WAIT_HANDLES <= 63);
pub(super) fn fits_wait_budget(live: usize, surfaces: usize) -> bool {
    live.checked_mul(3)
        .and_then(|n| {
            n.checked_add(7 + crate::desktop::composition::relay::sessions::MAX_SESSIONS * 2)
        })
        .and_then(|n| n.checked_add(surfaces))
        .is_some_and(|n| n <= MAX_WAIT_HANDLES)
}

fn append_wait_handle(handles: &mut [HANDLE], count: &mut usize, handle: HANDLE) -> Result<()> {
    if handles[..*count].contains(&handle) {
        return Ok(());
    }
    let slot = handles
        .get_mut(*count)
        .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Live wait set exhausted"))?;
    *slot = handle;
    *count += 1;
    Ok(())
}

impl Application {
    pub(super) fn shutdown_live(&mut self) -> Result<()> {
        let result = shutdown::drain(
            &mut self.live_sessions,
            &mut self.live_visibility,
            &mut self.authorization_owner,
            &mut self.remote_sessions,
        );
        self.retain_workspace_owners();
        result
    }
    unsafe fn poll_live(&mut self, visible: bool) -> Result<()> {
        self.poll_remote_live(visible)?;
        self.live_visibility
            .poll()
            .map_err(|error| Error::new(E_FAIL, error.to_string()))?;
        let mut index = 0;
        while index < self.live_sessions.len() {
            match self.live_sessions[index].poll() {
                None => {
                    let notice = self.live_sessions[index].take_input_notice().map(|owner| {
                        (
                            owner,
                            self.live_sessions[index]
                                .take_input_error()
                                .unwrap_or_default(),
                        )
                    });
                    index += 1;
                    // 不持有 Session 借用跨模态消息泵；失败仅停止源控制，采集继续。
                    if let Some((owner, error)) = notice {
                        if visible && IsWindowVisible(owner).as_bool() {
                            MessageBoxW(
                                Some(owner),
                                &HSTRING::from(format!(
                                    "Live 源窗口控制不可用。采集画面仍保留。\n{error}"
                                )),
                                w!("Otter"),
                                MB_OK | MB_ICONWARNING,
                            );
                        }
                    }
                }
                Some(result) => {
                    self.live_sessions.swap_remove(index);
                    self.retain_workspace_owners();
                    if let Err(error) = result {
                        if !visible {
                            return Err(Error::new(E_FAIL, error));
                        }
                        MessageBoxW(
                            None,
                            &HSTRING::from(format!("Live 已停止。\n{error}")),
                            w!("Otter"),
                            MB_OK | MB_ICONWARNING,
                        );
                    }
                }
            }
        }
        self.live_visibility
            .poll()
            .map_err(|error| Error::new(E_FAIL, error.to_string()))
    }
    pub(super) unsafe fn process_live(&mut self, visible: bool) -> Result<()> {
        self.poll_live(visible)?;
        let recycle = self.process_live_recycle();
        self.reference_result(recycle, visible)?;
        if let Err(error) = self.process_live_freeze(visible) {
            if !visible {
                return Err(error);
            }
            MessageBoxW(
                None,
                &HSTRING::from(format!("Live 画面未保留。\n{error}")),
                w!("Otter"),
                MB_OK | MB_ICONWARNING,
            );
        }
        for index in 0..self.documents.len() {
            let document = &self.documents[index];
            let request = document.state.live_request.borrow_mut().take();
            let Some(request) = request else {
                continue;
            };
            if !document.is_alive() || self.exit_requested.get() {
                continue;
            }
            let source = document.state.unit_id.clone();
            let result = (|| -> Result<()> {
                if self.live_sessions.len() >= MAX_SESSIONS
                    || self.workspace.units().len() >= crate::workspace::MAX_UNITS
                    || !fits_wait_budget(
                        self.live_sessions.len() + 1,
                        self.surface_windows.wait_budget(),
                    )
                {
                    return Err(Error::new(E_OUTOFMEMORY, "Live session limit reached"));
                }
                // 先保留 owner 容量；显示失败会析构局部会话，停止采集并回收线程。
                self.live_sessions
                    .try_reserve(1)
                    .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot allocate Live owner"))?;
                use crate::desktop::capture_input::live::Request;
                let session = match request {
                    Request::Region(screen) => Session::start_region(screen),
                    Request::Window(target) => self.start_live_window(target, None),
                    Request::WindowRegion(target, crop) => {
                        self.start_live_window(target, Some(crop))
                    }
                }
                .map_err(|error| Error::new(E_FAIL, error.to_string()))?;
                self.admit_live(session, visible)?;
                Ok(())
            })();
            if let Err(error) = result {
                if !visible {
                    return Err(error);
                }
                MessageBoxW(
                    self.source_window(&source),
                    &HSTRING::from(format!("无法开始 Live。\n{error}")),
                    w!("Otter"),
                    MB_OK | MB_ICONWARNING,
                );
            }
        }
        Ok(())
    }
    unsafe fn start_live_window(
        &mut self,
        target: crate::live_window_target::Target,
        crop: Option<crate::live_window_target::region::Crop>,
    ) -> std::io::Result<Session> {
        // 在隐藏/恢复保护接管之前记录用户选中源的物理可见外框。
        let frame = target.frame_bounds().map_err(std::io::Error::other)?;
        let screen = crop
            .map(|crop| crop.screen(frame))
            .transpose()
            .map_err(std::io::Error::other)?
            .unwrap_or(frame);
        let protection = self
            .live_visibility
            .acquire(target.clone())
            .map_err(std::io::Error::other)?;
        Session::start_window_region(target, crop, screen).map(|mut session| {
            session.protect(protection);
            session
        })
    }
    pub(super) unsafe fn next_live_message(&mut self, message: &mut MSG) -> Result<i32> {
        if self.session_end.paused() {
            return wait::next(&[], message);
        }
        self.process_authorization();
        self.surface_windows
            .poll(crate::desktop::surface_windows::now_ms()?)?;
        self.poll_live(true)?;
        let mut handles = [HANDLE::default(); MAX_WAIT_HANDLES];
        let mut count = 0;
        for session in &self.live_sessions {
            for handle in [
                session.completion_handle(),
                session.source_process_handle(),
                session.input_completion_handle(),
            ]
            .into_iter()
            .flatten()
            {
                let handle = HANDLE(handle.as_raw_handle());
                append_wait_handle(&mut handles, &mut count, handle)?;
            }
        }
        for handle in [
            self.live_visibility.completion_handle(),
            self.live_visibility.child_handle(),
            self.live_visibility.input_handle(),
        ]
        .into_iter()
        .flatten()
        {
            append_wait_handle(&mut handles, &mut count, HANDLE(handle.as_raw_handle()))?;
        }
        for handle in self.authorization_owner.wait_handles() {
            append_wait_handle(&mut handles, &mut count, HANDLE(handle.as_raw_handle()))?;
        }
        for handle in self.remote_sessions.wait_handles() {
            append_wait_handle(&mut handles, &mut count, HANDLE(handle.as_raw_handle()))?;
        }
        if let Some(handle) = self.projection_receive.wait_handle() {
            // 原 accept 的 Slot 仍在 Hold；这里只借接续 Job 的原线程句柄，不创建新 event。
            append_wait_handle(&mut handles, &mut count, HANDLE(handle.as_raw_handle()))?;
        }
        if let Some(handle) = self.projection_edit.wait_handle() {
            // edit与receiver准入互斥，共用原接收等待名额，不扩Windows的63句柄预算。
            append_wait_handle(&mut handles, &mut count, HANDLE(handle.as_raw_handle()))?;
        }
        for handle in self.surface_windows.wait_handles() {
            append_wait_handle(&mut handles, &mut count, HANDLE(handle.as_raw_handle()))?;
        }
        let deadline = self
            .surface_windows
            .deadline(crate::desktop::surface_windows::now_ms()?)?;
        let deadline = deadline
            .into_iter()
            .chain(
                self.remote_sessions
                    .resource_deadline(crate::desktop::surface_windows::now_ms()?),
            )
            .chain(
                self.projection_receive
                    .deadline(crate::desktop::surface_windows::now_ms()?),
            )
            .chain(self.projection_cleanup.deadline())
            .chain(
                self.projection_edit
                    .deadline(crate::desktop::surface_windows::now_ms()?),
            )
            .chain(
                self.projection_sender
                    .deadline(crate::desktop::surface_windows::now_ms()?),
            )
            .chain(self.projection_policy_deadline())
            .chain(self.local_arts.deadline())
            .min();
        // 本调用返回前不修改会话集合或回收任务；句柄 owner 始终存活。
        super::super::session_end::with_query(
            &self.session_end,
            || self.session_end_ready(),
            || wait::next_until(&handles[..count], message, deadline),
        )
    }
}

#[cfg(test)]
#[path = "application_live_tests.rs"]
mod tests;
