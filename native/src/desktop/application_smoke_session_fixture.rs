//! Own the disposable session and asset directories used by the native smoke path.
use super::*;
use std::{
    fs,
    path::{Component, Path, PathBuf, Prefix},
};

pub(super) struct Directory {
    selected: PathBuf,
    resolved: PathBuf,
}

const SESSION_NAME: &str = "session.json";

impl Directory {
    pub(super) fn new() -> Result<Self> {
        // Simulate the file chooser, not the canonical path returned by saving.
        let parent = std::env::temp_dir();
        if !parent.is_absolute()
            || !matches!(parent.components().next(), Some(Component::Prefix(prefix))
                if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::UNC(_, _)))
        {
            return Err(Error::new(
                E_FAIL,
                "Session smoke requires an ordinary absolute TEMP directory",
            ));
        }
        let id = unsafe { windows::Win32::System::Com::CoCreateGuid()? };
        Self::create(parent.join(format!("otter-session-fixture-{id:?}")))
    }

    fn create(selected: PathBuf) -> Result<Self> {
        // A pre-existing directory never becomes a fixture or a cleanup target.
        fs::create_dir(&selected).map_err(io_error)?;
        match selected.canonicalize() {
            Ok(resolved) => Ok(Self { selected, resolved }),
            Err(error) => {
                let _ = fs::remove_dir(&selected);
                Err(io_error(error))
            }
        }
    }

    pub(super) fn child(&self, name: &'static str) -> Result<Self> {
        Self::create(self.selected.join(name))
    }

    pub(super) fn path(&self) -> &Path {
        &self.selected
    }

    pub(super) fn target(&self) -> PathBuf {
        self.selected.join(SESSION_NAME)
    }

    pub(super) fn require_saved(&self, app: &Application) -> Result<PathBuf> {
        self.require_saved_path(
            app.restored_session
                .as_ref()
                .and_then(|r| r.saved_path.as_deref()),
        )
    }

    fn require_saved_path(&self, saved: Option<&Path>) -> Result<PathBuf> {
        let target = self.target();
        let resolved = self.resolved.join(SESSION_NAME);
        if !saved.is_some_and(|path| path == target || path == resolved)
            || target.canonicalize().map_err(io_error)? != resolved
        {
            return Err(Error::new(
                E_ABORT,
                "Workflow smoke requires its own committed session",
            ));
        }
        Ok(target)
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        // Only reclaim direct files of the new directory; never walk nested
        // directories or follow a directory that has been redirected.
        if !self
            .selected
            .canonicalize()
            .is_ok_and(|path| path == self.resolved)
        {
            return;
        }
        if let Ok(entries) = fs::read_dir(&self.resolved) {
            for entry in entries.flatten() {
                if entry.file_type().is_ok_and(|kind| kind.is_file()) {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
        let _ = fs::remove_dir(&self.resolved);
    }
}

fn io_error(error: std::io::Error) -> Error {
    Error::new(E_FAIL, format!("Session smoke fixture: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn session_fixture_accepts_only_its_selected_path_or_save_receipt() {
        let first = Directory::new().unwrap();
        let second = Directory::new().unwrap();
        let selected = first.target();
        let cancel = AtomicBool::new(false);
        let staged = crate::session_file_write::Staged::prepare_bytes(
            &selected,
            b"fixture".to_vec().into(),
            &cancel,
        )
        .unwrap();
        let committed = staged.commit_new(&cancel).unwrap();
        assert_eq!(
            first.require_saved_path(Some(&committed)).unwrap(),
            selected
        );
        assert_eq!(first.require_saved_path(Some(&selected)).unwrap(), selected);
        assert_eq!(
            second
                .require_saved_path(Some(&committed))
                .unwrap_err()
                .code(),
            E_ABORT
        );
        assert_eq!(
            crate::desktop::session_job::file::read_limited(&selected, &cancel, 64).unwrap(),
            b"fixture"
        );
    }

    #[test]
    fn session_fixtures_are_exclusive_and_reclaim_their_files() {
        let first = Directory::new().unwrap();
        let second = Directory::new().unwrap();
        let first_path = first.path().to_owned();
        assert_ne!(first_path, second.path());
        assert!(Directory::create(first_path.clone()).is_err());
        assert!(!first.target().exists());
        fs::write(first.target(), b"fixture").unwrap();
        fs::write(second.target(), b"independent").unwrap();
        let child = first.child("assets").unwrap();
        fs::write(child.path().join("sample.png"), b"synthetic asset").unwrap();
        drop(child);
        drop(first);
        assert!(!first_path.exists());
        assert_eq!(fs::read(second.target()).unwrap(), b"independent");
        let second_path = second.path().to_owned();
        drop(second);
        assert!(!second_path.exists());
    }
}
