//! 退出前给已知附件一次停用机会；绝对截止后由原有撤权/drain 回收，不重放 unknown。
use super::*;
use std::time::{Duration, Instant};

const GRACE: Duration = Duration::from_secs(2);

impl Application {
    pub(in crate::desktop::application) fn finish_surface_transitions(&mut self) -> Result<()> {
        self.surface_windows.close_all()?;
        // 图片 GET 不需要远端停用；立即取消，给附件生命周期事务让出共享槽位。
        self.remote_sessions.cancel_surface_resources(|_, _| true);
        let deadline = Instant::now() + GRACE;
        pump_until(deadline, || {
            // 错误仍保存在 Outcome，交回窗口后由最终附件门禁报告；不在退出时弹窗。
            let _ = self.remote_sessions.poll_surfaces();
            // closed 准入只登记失败证据，不为退出创建隐藏窗口；迟到挂载也可停用。
            self.admit_surface_results()?;
            self.activate_surface_windows(false)?;
            let now = crate::desktop::surface_windows::now_ms()?;
            for failure in self.surface_windows.failures_mut() {
                let Some(owner) = failure.begin_inactive() else {
                    continue;
                };
                if let Err(error) = super::super::surface_admission::start_transition(
                    &mut self.authorization_owner,
                    &mut self.remote_sessions,
                    owner,
                    crate::surface_live_binding::Lifecycle::Inactive,
                    now,
                ) {
                    failure.failed_start(error);
                }
            }
            Ok(self
                .remote_sessions
                .surface_shutdown_handles()
                .map(|handle| HANDLE(handle.as_raw_handle()))
                .chain(
                    self.surface_windows
                        .javascript_wait_handles()
                        .map(|handle| HANDLE(handle.as_raw_handle())),
                )
                .collect())
        })
    }
}

struct Quit(Option<i32>);
impl Drop for Quit {
    fn drop(&mut self) {
        if let Some(code) = self.0 {
            unsafe { PostQuitMessage(code) };
        }
    }
}

/// settle 回调必须保持返回句柄存活至下一次回调；不得重复返回已回收任务的句柄。
/// 每轮先结算，再阻塞等待，不建定时器或辅助线程；截止不会因消息而重置。
fn pump_until(deadline: Instant, mut settle: impl FnMut() -> Result<Vec<HANDLE>>) -> Result<()> {
    let mut quit = Quit(None);
    loop {
        if Instant::now() >= deadline {
            return Err(Error::new(E_FAIL, "Surface shutdown grace period expired"));
        }
        let handles = settle()?;
        if handles.is_empty() {
            return Ok(());
        }
        let mut message = MSG::default();
        unsafe {
            if wait::next_until(&handles, &mut message, Some(deadline))? == 0 {
                // 先移出 WM_QUIT，结束时再交给原 drain/外层；不能重新入队造成忙循环。
                quit.0 = Some(message.wParam.0 as i32);
            } else {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
}

#[cfg(test)]
#[path = "application_surface_shutdown_tests.rs"]
mod tests;
