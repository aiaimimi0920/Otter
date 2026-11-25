//! 应用级单任务恢复协调：启动即冻结来源凭证，完成后再绑定目录、建窗与发布。
use super::*;
use crate::desktop::{session_asset_root::Root, session_binding::Bound, session_job};
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "application_restore_context_smoke.rs"]
mod context_smoke;
#[path = "application_frozen_job.rs"]
mod frozen;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "application_restore_job_smoke.rs"]
pub(super) mod smoke;

pub(in crate::desktop::application) struct Job {
    worker: session_job::Job,
    origin: guard::Guard,
    catalog: Option<Rc<Loaded>>,
    space: Option<crate::session_placement::Space>,
    cancelled: bool,
}

impl Application {
    pub(in crate::desktop::application) fn restore_reap_deadline(
        &self,
    ) -> Option<std::time::Instant> {
        self.restore_job.as_ref()?.worker.deadline()
    }

    /// CLI 检查模式消费真实消息唤醒，但不进入注册热键/显示首窗口的 run()。
    pub(in crate::desktop) unsafe fn check_restored_startup(mut self) -> Result<()> {
        loop {
            self.execution_wake.acknowledge();
            self.process_restore(false)?;
            self.process_frozen_menu(false)?;
            if self.restore_job.is_none()
                && self.save_job.is_none()
                && !self.recycle_maintenance_pending
            {
                break;
            }
            if self.restore_job.is_none() && self.save_job.is_none() {
                return Err(Error::new(
                    E_ABORT,
                    "Session check workspace is busy before recycle maintenance",
                ));
            }
            let mut message = MSG::default();
            // 此循环只结算恢复与冻结库，不引入普通应用任务的截止。
            if self.next_session_check_message(&mut message)? == 0 {
                self.cancel_restore();
                return Err(Error::new(
                    E_FAIL,
                    "Session check message loop ended before completion",
                ));
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        let record = self
            .restored_session
            .as_ref()
            .ok_or_else(|| Error::new(E_FAIL, "Session check did not publish a workspace"))?;
        self.verify_startup_naming()?;
        let recycle = crate::session_library::decode(
            &record.original,
            crate::session_library::Kind::RecycleBin,
        )
        .map_err(|message| Error::new(E_FAIL, message))?;
        println!(
            "native-session-recycle: entries={}",
            recycle.entries().len()
        );
        let mut rendered = 0;
        for doc in &self.documents {
            let before = doc.state.canvas.borrow().paints;
            for rebuild in [false, true] {
                if rebuild {
                    doc.state.canvas.borrow_mut().discard_device();
                }
                SendMessageW(doc.window.0, WM_PAINT, None, None);
                if let Some(error) = doc.state.error.borrow_mut().take() {
                    return Err(error);
                }
            }
            if !doc.is_alive() || doc.state.canvas.borrow().paints != before + 2 {
                return Err(Error::new(
                    E_FAIL,
                    "Restored window failed paint/resource recreation check",
                ));
            }
            rendered += 2;
        }
        if self.documents.iter().any(|doc| {
            IsWindowVisible(doc.window.0).as_bool()
                || IsWindowVisible(doc.panel.0).as_bool()
                || doc.state.global_capture.get()
                || doc.state.global_long_capture.get()
                || doc.state.global_live_capture.get()
        }) {
            return Err(Error::new(
                E_FAIL,
                "Session check unexpectedly exposed a window or hotkey",
            ));
        }
        println!(
            "native-session-check: windows={} annotations={} bytes={} offline={} hidden=true",
            self.documents.len(),
            record
                .annotation_ids
                .iter()
                .map(|(_, ids)| ids.len())
                .sum::<usize>(),
            record.original.len(),
            self.catalog.borrow().loaded.is_none()
        );
        println!("native-session-render-check: paints={rendered} device-recreated=true");
        println!("native-session-save-state: {:?}", self.save_status());
        if let Some(document) = self.documents.first() {
            let mut rect = RECT::default();
            GetWindowRect(document.window.0, &mut rect)?;
            println!(
                "native-session-placement: left={} top={} width={} height={}",
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top
            );
        }
        Ok(())
    }

    unsafe fn next_session_check_message(&self, message: &mut MSG) -> Result<i32> {
        let deadline = self
            .restore_reap_deadline()
            .into_iter()
            .chain(self.save_reap_deadline())
            .min();
        live::wait::next_until(&[], message, deadline)
    }

    /// bytes/root 是调用方显式选择的数据；不读取默认用户配置，也不猜历史 DPR。
    pub unsafe fn request_restore(
        &mut self,
        bytes: Vec<u8>,
        root: Option<Root>,
        space: crate::session_placement::Space,
    ) -> Result<()> {
        self.request_restore_input(session_job::Input::Bytes(bytes), root, space)
    }
    pub unsafe fn request_restore_input(
        &mut self,
        input: session_job::Input,
        root: Option<Root>,
        space: impl Into<Option<crate::session_placement::Space>>,
    ) -> Result<()> {
        if matches!(&input, session_job::Input::Frozen(_)) {
            return Err(Error::new(
                E_INVALIDARG,
                "Frozen input requires append request ownership",
            ));
        }
        self.start_restore(input, root, space.into())
    }

    unsafe fn start_restore(
        &mut self,
        input: session_job::Input,
        root: Option<Root>,
        space: Option<crate::session_placement::Space>,
    ) -> Result<()> {
        if self.restore_job.is_some() || self.save_job.is_some() {
            return Err(Error::new(
                E_ABORT,
                "A session restoration or save preparation is already running",
            ));
        }
        if let Some(space) = space {
            crate::session_coordinates::select(Some(space), None)
                .map_err(|message| Error::new(E_INVALIDARG, message))?;
        }
        let origin = guard::Guard::capture(self)?;
        if !origin.matches(self) {
            return Err(Error::new(
                E_ABORT,
                "Workspace is busy during session restoration request",
            ));
        }
        let catalog = self.catalog.borrow().loaded.clone();
        let handshake = catalog.as_ref().map(|loaded| loaded.handshake.clone());
        let worker =
            session_job::Job::start_input(input, root, handshake, self.execution_wake.callback())
                .map_err(|error| {
                Error::new(E_FAIL, format!("Cannot start session preparation: {error}"))
            })?;
        self.restore_job = Some(Job {
            worker,
            origin,
            catalog,
            space,
            cancelled: false,
        });
        Ok(())
    }

    /// 只发取消信号；不在 UI 线程等待 WIC/文件调用，完成通知仍负责回收。
    pub fn cancel_restore(&mut self) {
        if let Some(job) = &mut self.restore_job {
            job.cancelled = true;
            job.worker.cancel();
        }
    }

    pub(super) unsafe fn poll_restore(&mut self, visible: bool) -> Result<()> {
        self.poll_restore_with(visible, |app| {
            if !visible {
                return Err(Error::new(E_INVALIDARG, "Session has no coordinate metadata; supply --session-origin X,Y and --session-scale"));
            }
            let doc = app.documents.iter().find(|doc| doc.is_alive())
                .ok_or_else(|| Error::new(E_ABORT, "Missing coordinate dialog owner"))?;
            let flag = doc.state.file_dialog.clone();
            let _dialog = crate::desktop::file_dialog::Guard::begin(&flag)
                .ok_or_else(|| Error::new(E_ABORT, "Another file dialog is active"))?;
            Ok(crate::desktop::session_coordinate_dialog::show(doc.window.0))
        })
    }

    pub(in crate::desktop::application) unsafe fn poll_restore_with(
        &mut self,
        visible: bool,
        choose: impl FnOnce(&Self) -> Result<Option<crate::session_placement::Space>>,
    ) -> Result<()> {
        let Some(result) = self.restore_job.as_mut().and_then(|job| job.worker.poll()) else {
            return Ok(());
        };
        let job = self.restore_job.take().unwrap();
        if job.cancelled {
            return Ok(());
        }
        let session_job::Ready {
            restore: prepared,
            frozen,
        } = result.map_err(|message| Error::new(E_FAIL, message))?;
        if !job.origin.matches(self)
            || !crate::desktop::session_binding::catalog_matches(
                &job.catalog,
                self.catalog.borrow().loaded.as_ref(),
            )
        {
            return Err(Error::new(
                E_ABORT,
                "Workspace or capability catalog changed while loading session",
            ));
        }
        let embedded = prepared.vector.candidate.coordinate_space;
        let supplied = if job.space.is_none() && embedded.is_none() {
            let Some(space) = choose(self)? else {
                return Ok(());
            };
            Some(space)
        } else {
            job.space
        };
        // 模态输入可能派发窗口/目录消息；确认后重新核对原来源，不能覆盖期间新增编辑。
        if !job.origin.matches(self)
            || !crate::desktop::session_binding::catalog_matches(
                &job.catalog,
                self.catalog.borrow().loaded.as_ref(),
            )
        {
            return Err(Error::new(
                E_ABORT,
                "Workspace changed during coordinate input",
            ));
        }
        let space = crate::session_coordinates::select(supplied, embedded)
            .map_err(|message| Error::new(E_INVALIDARG, message))?;
        let cancel = AtomicBool::new(false);
        let prepared = Bound::from_prepared(prepared, job.catalog)
            .into_parts_for(self.catalog.borrow().loaded.as_ref(), &cancel)
            .map_err(|_| Error::new(E_ABORT, "Session binding became stale"))?
            .prepare_states(&cancel)?;
        let batch = self.prepare_restored_batch(prepared, &cancel, |placement| {
            space.physical(placement.rect).map_err(|_| {
                Error::new(E_INVALIDARG, "Session window coordinate conversion failed")
            })
        })?;
        if let Some(frozen) = frozen {
            // 追加原文按任务开始时的活跃集合构造；建窗期间若有重入，仍须核对原任务凭证。
            if !job.origin.matches(self) {
                return Err(Error::new(
                    E_ABORT,
                    "Workspace changed before frozen append",
                ));
            }
            let append = self.prepare_append(batch, frozen.source, frozen.retained, space)?;
            return self.publish_append(append, visible);
        }
        self.publish_restored_batch(batch, visible)?;
        self.restored_session
            .as_mut()
            .expect("published session")
            .space = Some(space);
        self.record_loaded_save_state();
        self.schedule_recycle_maintenance();
        Ok(())
    }

    pub(in crate::desktop::application) unsafe fn process_restore(
        &mut self,
        visible: bool,
    ) -> Result<()> {
        if let Err(error) = self.poll_restore(visible) {
            if !visible {
                return Err(error);
            }
            // 加载失败不能终止应用或关闭原窗口；可见模式报告一次后清除本次任务。
            MessageBoxW(
                self.documents.first().map(|doc| doc.window.0),
                &HSTRING::from(format!("无法恢复会话，原工作区保留。\n{error}")),
                w!("Otter"),
                MB_OK | MB_ICONWARNING,
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::{
        loom_execution_tasks::{
            reap_fixture::{Gate, WAIT},
            shader_tests::SERIAL,
        },
        Apartment,
    };

    #[test]
    fn session_check_reaps_restore_after_its_only_notification() {
        let _serial = SERIAL.lock().unwrap();
        let _apartment = Apartment::new().unwrap();
        unsafe {
            let instance = crate::desktop::workspace_window_fixture::register();
            let first =
                DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
            let mut app = Application::new(instance, first);
            let _watchdog = execution_test_pump::Pump::new();
            let wake = app.execution_wake.callback();
            let gate = Gate::new(move || wake());
            let worker = session_job::Job::start(
                br#"{"stickers":[],"links":[]}"#.to_vec(),
                None,
                gate.notify,
            )
            .unwrap();
            app.restore_job = Some(Job {
                worker,
                origin: guard::Guard::capture(&app).unwrap(),
                catalog: None,
                space: Some(crate::session_placement::Space {
                    origin: (0, 0),
                    scale: 1.0,
                }),
                cancelled: false,
            });
            gate.noticed.recv_timeout(WAIT).unwrap();
            let mut message = MSG::default();
            assert!(
                PeekMessageW(&mut message, None, WM_APP + 92, WM_APP + 92, PM_REMOVE).as_bool()
            );
            app.execution_wake.acknowledge();
            app.process_restore(false).unwrap();
            assert!(app.restore_job.is_some(), "notifier has not exited");
            assert!(app.restore_reap_deadline().is_some());
            // 忽略可能仍排队的隐藏窗口消息；WM_NULL 必须来自 CLI 自己的截止。
            loop {
                assert_eq!(app.next_session_check_message(&mut message).unwrap(), 1);
                if message.message == WM_NULL && message.hwnd.is_invalid() {
                    break;
                }
                assert_ne!(
                    message.message,
                    WM_APP + 997,
                    "session check missed reap deadline"
                );
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            assert!(app.restore_job.is_some());
            gate.release.send(()).unwrap();
            app.check_restored_startup().unwrap();
            assert!(gate.noticed.try_recv().is_err());
        }
    }
}
