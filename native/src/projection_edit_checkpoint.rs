//! 后台生成有界协作包含凭证；不持栅格/Document副本，也不授予磁盘已提交事实。
use super::{
    journal::Context,
    workspace::{Role, Snapshot},
    Document, Route,
};
use sha2::{Digest, Sha256};

pub(crate) struct Index(Vec<Entry>);
struct Entry {
    context: Context,
    projection: String,
    route: Route,
    role: Role,
    document: [u8; 32],
}
impl Index {
    pub(crate) fn capture(source: &crate::session_document::Document<'_>) -> Option<Self> {
        let mut entries = Vec::new();
        for window in &source.windows {
            let Some(snapshot) = Snapshot::restore(window, source.projection_scope).ok()? else {
                continue;
            };
            if entries.len() >= super::workspace::MAX_ENTRIES {
                return None;
            }
            if snapshot.suspended().is_none() {
                entries.push(Entry {
                    context: snapshot.context().clone(),
                    projection: snapshot.projection_id().into(),
                    route: snapshot.route(),
                    role: snapshot.role(),
                    document: digest(snapshot.document())?,
                });
            }
        }
        Some(Self(entries))
    }
    pub(crate) fn includes(&self, snapshot: &Snapshot) -> bool {
        snapshot.suspended().is_none()
            && digest(snapshot.document()).is_some_and(|document| {
                self.0.iter().any(|entry| {
                    entry.context == *snapshot.context()
                        && entry.projection == snapshot.projection_id()
                        && entry.route == snapshot.route()
                        && entry.role == snapshot.role()
                        && entry.document == document
                })
            })
    }
}
fn digest(document: &Document) -> Option<[u8; 32]> {
    struct Hash(Sha256);
    impl std::io::Write for Hash {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.update(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut hash = Hash(Sha256::new());
    serde_json::to_writer(&mut hash, document).ok()?;
    Some(hash.0.finalize().into())
}
