//! 同一 native profile 租约下的独立同意文件；不受普通命名导入影响，不另开 journal。
use crate::{
    pixels::Pixels,
    projection_receive_policy::{Change, Document, Policy, MAX_BYTES},
    session_file_replace::Replacement,
    session_file_write::Staged,
};
use std::{
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

#[derive(Clone)]
pub(super) struct File {
    path: PathBuf,
    baseline: Option<Pixels>,
    document: Option<Document>,
}
impl File {
    pub(super) fn load(root: &Path, cancel: &AtomicBool) -> Self {
        let path = root.join("projection-receive-policy.json");
        let result = crate::profile_directory::validate(root).and_then(|_| {
            crate::desktop::session_job::file::read_recovering_limited(&path, cancel, MAX_BYTES)
        });
        match result {
            Ok((bytes, None)) => {
                let document = Document::decode(&bytes).ok();
                match crate::pixels::reserve(bytes.capacity()) {
                    Some(lease) => Self {
                        document,
                        path,
                        baseline: Some(Pixels::from_reserved(bytes, lease)),
                    },
                    None => Self {
                        path,
                        baseline: None,
                        document: None,
                    },
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Self {
                path,
                baseline: None,
                document: Some(Document::default()),
            },
            _ => Self {
                path,
                baseline: None,
                document: None,
            },
        }
    }
    pub(super) fn policy(&self) -> Policy {
        self.document
            .as_ref()
            .map_or(Policy::Reject, Document::policy)
    }
    pub(super) fn writable(&self) -> bool {
        self.document.is_some()
    }
    pub(super) fn paused(&self) -> bool {
        self.document.as_ref().is_none_or(Document::paused)
    }
    pub(super) fn prepare(
        self,
        change: impl Into<Change>,
        cancel: &AtomicBool,
    ) -> Result<Prepared, String> {
        let mut lease =
            crate::pixels::reserve(MAX_BYTES).ok_or("Receive policy byte budget exhausted")?;
        let bytes = self
            .document
            .as_ref()
            .ok_or("Receive policy is unreadable")?
            .with_change(change.into())?;
        if !lease.grow_to(bytes.capacity()) {
            return Err("Receive policy byte budget exhausted".into());
        }
        let bytes = Pixels::from_reserved(bytes, lease);
        let document = Document::decode(&bytes)?;
        crate::profile_directory::validate(self.path.parent().ok_or("Missing profile")?)
            .map_err(|error| error.to_string())?;
        let transaction = match &self.baseline {
            Some(before) => Transaction::Update(
                Replacement::prepare_bytes(&self.path, before, bytes.clone(), cancel)
                    .map_err(|error| error.to_string())?,
            ),
            None => Transaction::New(
                Staged::prepare_bytes(&self.path, bytes.clone(), cancel)
                    .map_err(|error| error.to_string())?,
            ),
        };
        Ok(Prepared {
            transaction,
            next: Self {
                path: self.path,
                baseline: Some(bytes),
                document: Some(document),
            },
        })
    }
}
enum Transaction {
    New(Staged),
    Update(Replacement),
}
pub(super) struct Prepared {
    transaction: Transaction,
    next: File,
}
impl Prepared {
    pub(super) fn commit(self) -> Result<(File, bool), String> {
        let cancel = AtomicBool::new(false);
        let retained = match self.transaction {
            Transaction::New(staged) => {
                staged.commit_new(&cancel).map_err(|e| e.to_string())?;
                false
            }
            Transaction::Update(staged) => staged
                .commit(&cancel)
                .map_err(|_| {
                    "Receive policy commit failed; recovery retained if needed".to_owned()
                })?
                .retained_recovery
                .is_some(),
        };
        Ok((self.next, retained))
    }
}
