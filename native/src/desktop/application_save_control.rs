//! Application 单槽保存准备；后台编码/暂存后复核凭证，最终提交仍由 UI 明确执行。
use super::*;
pub(super) enum Poll {
    Pending,
    Stale,
    Ready(Box<save_publish::Prepared>),
}
impl Application {
    pub unsafe fn request_save_candidate(&mut self) -> Result<()> {
        self.request_save(None)
    }
    pub unsafe fn request_save_new(&mut self, destination: std::path::PathBuf) -> Result<()> {
        self.request_save(Some(save_disk::Destination::New(destination)))
    }
    pub unsafe fn request_save_existing(&mut self) -> Result<()> {
        let path = self
            .restored_session
            .as_ref()
            .and_then(|record| record.saved_path.clone())
            .ok_or_else(|| Error::new(E_ABORT, "No committed session destination"))?;
        let record = self.restored_session.as_ref().unwrap();
        let expected = record
            .disk_original
            .as_ref()
            .unwrap_or(&record.original)
            .clone();
        self.request_save(Some(save_disk::Destination::Replace(path, expected)))
    }
    unsafe fn request_save(&mut self, destination: Option<save_disk::Destination>) -> Result<()> {
        if self.save_job.is_some() {
            return Err(Error::new(
                E_ABORT,
                "A session save preparation is already running",
            ));
        }
        let (receipt, data) = self.capture_session_snapshot()?;
        let job = save_snapshot::job::Job::start(data, destination, self.execution_wake.callback())
            .map_err(|error| Error::new(E_FAIL, format!("Cannot start session save: {error}")))?;
        self.save_job = Some((receipt, job));
        Ok(())
    }
    pub fn cancel_save_candidate(&self) {
        if let Some((_, job)) = &self.save_job {
            job.cancel();
        }
    }
    pub unsafe fn take_save_candidate(&mut self) -> Result<Option<save_publish::Prepared>> {
        match self.poll_save_candidate()? {
            Poll::Pending => Ok(None),
            Poll::Ready(prepared) => Ok(Some(*prepared)),
            Poll::Stale => Err(Error::new(
                E_ABORT,
                "Workspace changed while preparing session save",
            )),
        }
    }
    pub(super) unsafe fn poll_save_candidate(&mut self) -> Result<Poll> {
        self.poll_disk_candidate(false)
    }
    pub(super) unsafe fn poll_checkpoint_candidate(&mut self) -> Result<Poll> {
        self.poll_disk_candidate(true)
    }
    unsafe fn poll_disk_candidate(&mut self, checkpoint: bool) -> Result<Poll> {
        if self.save_job.as_ref().is_some_and(|(_, job)| {
            if checkpoint {
                !job.is_checkpoint()
            } else {
                !job.is_save()
            }
        }) {
            return Err(Error::new(E_ABORT, "Disk save candidate purpose mismatch"));
        }
        let Some(result) = self.save_job.as_mut().and_then(|(_, job)| job.poll()) else {
            return Ok(Poll::Pending);
        };
        let (receipt, _) = self.save_job.take().unwrap();
        let output = result.map_err(|message| Error::new(E_ABORT, message))?;
        if !receipt.matches(self) {
            return Ok(Poll::Stale);
        }
        Ok(Poll::Ready(Box::new(save_publish::Prepared {
            checkpoint_senders: output.checkpoint_senders,
            checkpoint_scope: output.checkpoint_scope,
            checkpoint_projections: output.checkpoint_projections,
            candidate: output.candidate,
            staged: output.staged,
            receipt,
        })))
    }
}

pub(super) unsafe fn verify(app: &mut Application) -> Result<()> {
    save_ui::verify_requests(app)?;
    let source = &app.documents[0];
    save_ui::request(source.window.0, &source.state)?;
    save_ui::request(source.window.0, &source.state)?;
    if !source.state.save_session_requested.replace(false) {
        return Err(Error::new(E_FAIL, "Save UI request was not queued"));
    }
    for mode in 0..3 {
        if mode == 0 {
            app.request_save_new(save_publish::smoke_destination()?)?;
        } else {
            app.request_save_candidate()?;
        }
        if app.request_save_candidate().is_ok() {
            return Err(Error::new(
                E_FAIL,
                "Save coordinator admitted a second worker",
            ));
        }
        if app
            .request_restore(
                br#"{"stickers":[],"links":[]}"#.to_vec(),
                None,
                crate::session_placement::Space {
                    origin: (0, 0),
                    scale: 1.0,
                },
            )
            .is_ok()
        {
            app.cancel_restore();
            return Err(Error::new(
                E_FAIL,
                "Restore started during save preparation",
            ));
        }
        let opacity = app.documents[0].state.canvas.borrow().normal_opacity;
        if mode == 1 {
            app.cancel_save_candidate();
        }
        if mode == 2 {
            app.documents[0].state.canvas.borrow_mut().normal_opacity =
                if opacity == 0.5 { 0.6 } else { 0.5 };
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let result = loop {
            match app.take_save_candidate() {
                Ok(None) if std::time::Instant::now() < deadline => {
                    std::thread::sleep(std::time::Duration::from_millis(1))
                }
                Ok(None) => {
                    app.cancel_save_candidate();
                    app.save_job.take();
                    break Err(Error::new(E_FAIL, "Save coordinator timed out"));
                }
                result => break result,
            }
        };
        app.documents[0].state.canvas.borrow_mut().normal_opacity = opacity;
        match (mode, result) {
            (0, Ok(Some(prepared))) => {
                crate::session_document::decode(&prepared.candidate.bytes)
                    .map_err(|_| Error::new(E_FAIL, "Save worker returned invalid candidate"))?;
                save_publish::verify(app, prepared)?;
            }
            (1 | 2, Err(error)) if error.code() == E_ABORT && app.save_job.is_none() => {}
            (_, Err(error)) => return Err(error),
            _ => {
                return Err(Error::new(
                    E_FAIL,
                    "Save coordinator accepted cancelled or stale candidate",
                ))
            }
        }
        if app.take_save_candidate()?.is_some() {
            return Err(Error::new(E_FAIL, "Save coordinator delivered twice"));
        }
    }
    println!(
        "native-save-worker-smoke: single-slot=true cancelled-rejected=true stale-rejected=true"
    );
    Ok(())
}
