//! 单槽后台减权/metadata patch/ack；不持有 HWND，结果安装前必须重核当前 canonical 与 Record allocation。
use super::*;
use crate::desktop::loom_execution_signal::{CompletionSignal, Notify};
use std::{
    sync::{atomic::AtomicBool, Arc},
    thread::JoinHandle,
};
pub(super) enum Work {
    Begin,
    Patch {
        context: Context,
        binding: Binding,
        remove: bool,
    },
    Ack(crate::projection_delivery::sender::binding::stop::Acknowledgement),
}
pub(super) enum Output {
    Begun,
    Patched(Context, Source),
    Acknowledged,
}
pub(super) struct Job {
    pub request: Request,
    pub begin: bool,
    pub ack: bool,
    thread: Option<JoinHandle<std::result::Result<Output, SenderError>>>,
}
impl Job {
    pub(super) fn start(request: Request, work: Work, notify: Notify) -> std::io::Result<Self> {
        let begin = matches!(&work, Work::Begin);
        let ack = matches!(&work, Work::Ack(_));
        let worker = request.clone();
        let thread = std::thread::Builder::new()
            .name("otter-sender-stop".into())
            .spawn(move || {
                let cancel = Arc::new(AtomicBool::new(false));
                let _done = CompletionSignal::new(cancel.clone(), notify);
                match work {
                    Work::Begin => {
                        worker.request_stop()?;
                        Ok(Output::Begun)
                    }
                    Work::Patch {
                        context,
                        binding,
                        remove,
                    } => {
                        if !worker.stop_requested() || (remove && !worker.published_unlinked()) {
                            return Err(SenderError::Changed);
                        }
                        let source = crate::projection_delivery::sender::binding::stop::prepare(
                            &context.canonical,
                            &binding,
                            remove,
                            &cancel,
                        )?;
                        Ok(Output::Patched(context, source))
                    }
                    Work::Ack(proof) => {
                        worker.acknowledge_stop(&proof)?;
                        Ok(Output::Acknowledged)
                    }
                }
            })?;
        Ok(Self {
            request,
            begin,
            ack,
            thread: Some(thread),
        })
    }
    pub(super) fn finished(&self) -> bool {
        self.thread.as_ref().unwrap().is_finished()
    }
    pub(super) fn join(&mut self) -> std::result::Result<Output, SenderError> {
        self.thread
            .take()
            .unwrap()
            .join()
            .unwrap_or(Err(SenderError::Write))
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
