//! Independent actual-message oracle for the joint gate; confirm release in both control rounds while Source still publishes, not through stop-publication cleanup.
use super::*;
#[derive(Default)]
pub(super) struct Progress {
    applied: [bool; 2],
    released: [bool; 2],
}
fn record(root: &std::path::Path, name: &str, bytes: &[u8]) {
    let temporary = root.join(format!("{name}.tmp"));
    std::fs::write(&temporary, bytes).unwrap();
    std::fs::rename(temporary, root.join(name)).unwrap();
}
impl Progress {
    pub(super) fn poll(
        &mut self,
        root: &std::path::Path,
        target: &super::super::interaction::oracle::Window,
        idle: bool,
    ) {
        let messages = target.messages();
        for (index, code) in [65, 66].into_iter().enumerate() {
            let (applied, released) = if index == 0 {
                (
                    "source-input-applied.json",
                    "source-input-released-live.json",
                )
            } else {
                (
                    "source-input-applied-again.json",
                    "source-input-released-live-again.json",
                )
            };
            if !self.applied[index]
                && messages
                    .iter()
                    .any(|&(m, k, _)| m == WM_KEYDOWN && k == code)
                && messages
                    .iter()
                    .filter(|&&(m, _, _)| m == WM_LBUTTONDOWN)
                    .count()
                    > index
            {
                record(root, applied, &serde_json::to_vec(&serde_json::json!({"actualKeyDown":code,"actualMouseDown":true,"originalWorker":true})).unwrap());
                self.applied[index] = true;
            }
            if self.applied[index]
                && !self.released[index]
                && idle
                && messages.iter().any(|&(m, k, _)| m == WM_KEYUP && k == code)
                && messages
                    .iter()
                    .filter(|&&(m, _, _)| m == WM_LBUTTONUP)
                    .count()
                    > index
            {
                record(root, released, &serde_json::to_vec(&serde_json::json!({"actualKeyUp":code,"actualMouseUp":true,"originalWorkerIdle":true,"sourceStillPublishing":true})).unwrap());
                self.released[index] = true;
            }
        }
    }
}
