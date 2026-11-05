//! 磁盘准备结果的排他类型；新文件和替换事务不能同时出现或互相冒充。
use crate::{
    session_file_replace::{Failure, Published, Replacement},
    session_file_write::Staged,
    session_write::Candidate,
};
use std::{io, path::PathBuf, sync::atomic::AtomicBool};
pub enum Destination {
    New(PathBuf),
    Replace(PathBuf, crate::session_source::Source),
}
pub enum Disk {
    New(Staged),
    Replace(Replacement),
}
impl Destination {
    pub fn prepare(self, candidate: &Candidate, cancel: &AtomicBool) -> io::Result<Disk> {
        match self {
            Self::New(path) => Staged::prepare(&path, candidate, cancel).map(Disk::New),
            Self::Replace(path, expected) => {
                Replacement::prepare(&path, &expected, candidate, cancel).map(Disk::Replace)
            }
        }
    }
}
impl Disk {
    pub fn is_for(&self, candidate: &Candidate) -> bool {
        match self {
            Self::New(staged) => staged.is_for(candidate),
            Self::Replace(staged) => staged.is_for(candidate),
        }
    }
    pub fn commit(self, cancel: &AtomicBool) -> Result<Published, Failure> {
        match self {
            Self::New(staged) => staged
                .commit_new(cancel)
                .map(|destination| Published {
                    destination,
                    retained_recovery: None,
                })
                .map_err(|error| Failure {
                    error,
                    recovery: None,
                    restore_error: None,
                }),
            Self::Replace(staged) => staged.commit(cancel),
        }
    }
}
