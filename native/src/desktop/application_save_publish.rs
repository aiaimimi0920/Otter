//! Prepared save results retain the UI Receipt; recheck after the temporary file is written and before final commit.
use super::*;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
use crate::session_file_write::Staged;
use crate::session_write::Candidate;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

pub(in crate::desktop) struct Prepared {
    pub(super) checkpoint_accounts: Option<crate::projection_account::scene::Index>,
    pub(super) checkpoint_senders:
        Option<crate::projection_delivery::sender::binding::checkpoint::Index>,
    pub(super) checkpoint_scope: Option<crate::session_projection_scope::Proof>,
    pub(super) checkpoint_projections:
        Option<crate::projection_delivery::binding::checkpoint::Index>,
    pub(super) candidate: Candidate,
    pub(super) staged: Option<save_disk::Disk>,
    pub(super) promote_host: bool,
    pub(super) receipt: save_snapshot::Receipt,
}
impl Prepared {
    #[cfg(any(test, debug_assertions, feature = "smoke-test"))]
    unsafe fn current(&self, app: &Application, staged: &Staged) -> bool {
        staged.is_for(&self.candidate) && self.receipt.matches(app)
    }
    /// The UI thread must not pump messages between validation and rename; swap the baseline synchronously on success and preserve the old record on failure.
    #[cfg(any(test, debug_assertions, feature = "smoke-test"))]
    pub(super) unsafe fn commit_new(
        self,
        app: &mut Application,
        staged: Staged,
        cancel: &AtomicBool,
    ) -> Result<PathBuf> {
        self.commit_disk(app, save_disk::Disk::New(staged), cancel)
            .map(|published| published.destination)
    }
    pub(super) unsafe fn commit_ready(
        mut self,
        app: &mut Application,
        cancel: &AtomicBool,
    ) -> Result<crate::session_file_replace::Published> {
        let disk = self
            .staged
            .take()
            .ok_or_else(|| Error::new(E_FAIL, "Missing prepared session file"))?;
        self.commit_disk(app, disk, cancel)
    }
    unsafe fn commit_disk(
        self,
        app: &mut Application,
        disk: save_disk::Disk,
        cancel: &AtomicBool,
    ) -> Result<crate::session_file_replace::Published> {
        let source = crate::session_source::Source::from(self.candidate.bytes.clone());
        let stale = || Error::new(E_ABORT, "Save candidate or workspace changed before commit");
        if !disk.is_for(&self.candidate)
            || self
                .checkpoint_senders
                .as_ref()
                .is_some_and(|index| !index.is_for(&source))
            || self.checkpoint_scope.as_ref().is_none_or(|proof| {
                !proof.is_for(&source)
                    || app
                        .restored_session
                        .as_ref()
                        .and_then(|record| record.projection_scope)
                        != Some(proof.scope())
            })
            || self
                .checkpoint_projections
                .as_ref()
                .is_some_and(|index| !index.is_for(&source))
        {
            return Err(stale());
        }
        let saved_state = self.receipt.checked_saved_stamp(app).ok_or_else(stale)?;
        let published = disk.commit(cancel).map_err(|failure| {
            let mut message = format!("Cannot commit session: {}", failure.error);
            if let Some(path) = failure.recovery {
                message.push_str(&crate::ui_text::format(
                    "app-session-original-retained",
                    &[("path", path.display().to_string().into())],
                ));
            }
            if let Some(error) = failure.restore_error {
                message.push_str(&format!("\nCannot restore original destination: {error}"));
            }
            Error::new(E_FAIL, message)
        })?;
        app.projection_receive
            .committed(self.checkpoint_projections.as_ref(), &app.workspace);
        app.projection_sender
            .committed(self.checkpoint_senders.as_ref(), &app.workspace);
        let record = app
            .restored_session
            .as_mut()
            .expect("receipt verified retained session");
        if record.saved_path.as_ref() != Some(&published.destination) {
            app.workspace_autosave.disable();
        }
        record.original = source;
        // Advance read-only association identity only through committed candidates; manual files do not grant the collaboration ack eligibility of the default Checkpoint.
        record.restored_projections = self.checkpoint_projections;
        record.disk_original = None;
        record.saved_path = Some(published.destination.clone());
        // Receipt still names the same transient host the worker encoded.
        if self.promote_host {
            record.transient_host = None;
        }
        app.saved_state = Some(saved_state);
        Ok(published)
    }
}

#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
pub(super) unsafe fn verify(
    app: &mut Application,
    mut prepared: Prepared,
    fixture: &smoke_session_fixture::Directory,
) -> Result<()> {
    let cancel = AtomicBool::new(false);
    let failed = fixture.child("failed-commits")?;
    let path = failed.target();
    verify_failed_commits(app, &prepared.candidate, &path)?;
    let disk = prepared
        .staged
        .take()
        .ok_or_else(|| Error::new(E_FAIL, "Save worker omitted staged file"))?;
    let save_disk::Disk::New(staged) = disk else {
        return Err(Error::new(E_FAIL, "New save returned replacement"));
    };
    if !prepared.current(app, &staged) {
        return Err(Error::new(E_FAIL, "Fresh staged candidate rejected"));
    }
    let opacity = app.documents[0].state.canvas.borrow().normal_opacity;
    app.documents[0].state.canvas.borrow_mut().normal_opacity =
        if opacity == 0.5 { 0.6 } else { 0.5 };
    let stale_rejected = !prepared.current(app, &staged);
    app.documents[0].state.canvas.borrow_mut().normal_opacity = opacity;
    if !stale_rejected {
        return Err(Error::new(
            E_FAIL,
            "Staged save ignored late workspace change",
        ));
    }
    let expected = prepared.candidate.bytes.clone();
    let revision = prepared.candidate.revision;
    let (old_receipt, _) = app.capture_session_snapshot()?;
    let committed = prepared.commit_new(app, staged, &cancel)?;
    fixture.require_saved(app)?;
    if failed.require_saved(app).is_ok() {
        return Err(Error::new(
            E_FAIL,
            "Unrelated fixture admitted the saved workspace",
        ));
    }
    drop(failed);
    let result = std::fs::read(&committed);
    if result.map_err(|error| Error::new(E_FAIL, error.to_string()))? != expected.as_ref() {
        return Err(Error::new(
            E_FAIL,
            "Published session differs from staged candidate",
        ));
    }
    let record = app.restored_session.as_ref().unwrap();
    let decoded = crate::session_document::decode(&record.original)
        .map_err(|_| Error::new(E_FAIL, "Cannot decode saved coordinate metadata"))?;
    if !decoded
        .coordinate_space
        .zip(record.space)
        .is_some_and(|(actual, expected)| {
            actual.origin == expected.origin && actual.scale == expected.scale
        })
    {
        return Err(Error::new(
            E_FAIL,
            "Saved coordinate metadata differs from snapshot",
        ));
    }
    if old_receipt.matches(app)
        || !record
            .original
            .same_storage(&crate::session_source::Source::from(expected))
        || record.saved_path.as_ref() != Some(&committed)
    {
        return Err(Error::new(
            E_FAIL,
            "Published session did not adopt exact baseline",
        ));
    }
    let (_, next) = app.capture_session_snapshot()?;
    let next = next
        .encode(&cancel)
        .map_err(|_| Error::new(E_FAIL, "Cannot encode next saved revision"))?;
    if next.expected_revision != revision || next.revision != revision + 1 {
        return Err(Error::new(
            E_FAIL,
            "Next save did not advance committed baseline",
        ));
    }
    restore_batch::workflow::smoke::verify(app, fixture)?;
    save_replace_smoke::verify(app, &committed)?;
    open_session::smoke::verify(app.instance, &fixture.target())?;
    println!("native-save-publish-smoke: candidate-bound=true late-stale-rejected=true new-file-roundtrip=true failed-baseline-preserved=true");
    Ok(())
}

/// Exercise the actual commit-failure branch, not just current; the target is only the exclusive path generated above.
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
unsafe fn verify_failed_commits(
    app: &mut Application,
    candidate: &Candidate,
    path: &std::path::Path,
) -> Result<()> {
    let io_error = |error: std::io::Error| Error::new(E_FAIL, error.to_string());
    for mode in 0..4 {
        let (receipt, _) = app.capture_session_snapshot()?;
        let (unchanged, _) = app.capture_session_snapshot()?;
        let record = app.restored_session.as_ref().unwrap();
        let old_path = record.saved_path.clone();
        let old_host = record.transient_host.clone();
        let prepared = Prepared {
            checkpoint_accounts: None,
            checkpoint_senders: None,
            checkpoint_scope: crate::session_projection_scope::Proof::capture(
                &candidate.bytes.clone().into(),
            ),
            checkpoint_projections: None,
            staged: None,
            promote_host: false,
            candidate: Candidate {
                bytes: candidate.bytes.clone(),
                expected_revision: candidate.expected_revision,
                revision: candidate.revision,
            },
            receipt,
        };
        let cancel = AtomicBool::new(false);
        let staged = Staged::prepare(path, &prepared.candidate, &cancel).map_err(io_error)?;
        let opacity = app.documents[0].state.canvas.borrow().normal_opacity;
        if mode == 0 {
            cancel.store(true, std::sync::atomic::Ordering::Release);
        } else if mode == 1 {
            app.documents[0].state.canvas.borrow_mut().normal_opacity =
                if opacity == 0.5 { 0.6 } else { 0.5 };
        } else if mode == 2 {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map_err(io_error)?;
            std::io::Write::write_all(&mut file, b"existing session sentinel").map_err(io_error)?;
        }
        let moved = if mode == 3 {
            let hwnd = app.documents[0].window.0;
            let mut rect = RECT::default();
            GetWindowRect(hwnd, &mut rect)?;
            let reposition = move |x| {
                SetWindowPos(
                    hwnd,
                    None,
                    x,
                    rect.top,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                )
            };
            reposition(rect.left + 7)?;
            Some((reposition, rect.left))
        } else {
            None
        };
        let result = prepared.commit_new(app, staged, &cancel);
        if let Some((reposition, x)) = moved {
            reposition(x)?;
            if !result.as_ref().is_err_and(|error| error.code() == E_ABORT) {
                return Err(Error::new(
                    E_FAIL,
                    "Commit did not reject late window movement",
                ));
            }
        }
        app.documents[0].state.canvas.borrow_mut().normal_opacity = opacity;
        let target_preserved = if mode == 2 {
            let bytes = std::fs::read(path).map_err(io_error)?;
            std::fs::remove_file(path).map_err(io_error)?;
            bytes == b"existing session sentinel"
        } else {
            !path.try_exists().map_err(io_error)?
        };
        let record = app.restored_session.as_ref().unwrap();
        if result.is_ok()
            || !target_preserved
            || !unchanged.matches(app)
            || record.saved_path != old_path
            || record.transient_host != old_host
        {
            return Err(Error::new(E_FAIL, "Failed save changed target or baseline"));
        }
    }
    Ok(())
}
