//! 仅测试跨child交付固定阶段/单调时间/预算状态；不接收脚本、异常文本或业务值。
use serde::{Deserialize, Serialize};
use std::{cell::Cell, io::Write, time::Instant};
use windows::Win32::{
    Foundation::FILETIME,
    System::Threading::{GetCurrentThread, GetThreadTimes},
};
pub(crate) const MAX_SAMPLES: usize = 64;
thread_local! { static REPORT_SUCCESS: Cell<bool> = const { Cell::new(false) }; }
pub(crate) fn capture(failed: bool) -> bool {
    failed || REPORT_SUCCESS.with(Cell::get)
}
// Only the explicit test-worker entry enables success reports; restore on unwind as well.
pub(crate) fn with_success_reports<T>(run: impl FnOnce() -> T) -> T {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            REPORT_SUCCESS.with(|enabled| enabled.set(self.0));
        }
    }
    let _reset = Reset(REPORT_SUCCESS.with(|enabled| enabled.replace(true)));
    run()
}
fn thread_cpu_us() -> Option<u64> {
    let [mut created, mut exited, mut kernel, mut user] = [FILETIME::default(); 4];
    unsafe {
        GetThreadTimes(
            GetCurrentThread(),
            &mut created,
            &mut exited,
            &mut kernel,
            &mut user,
        )
    }
    .ok()?;
    let ticks = |v: FILETIME| (u64::from(v.dwHighDateTime) << 32) | u64::from(v.dwLowDateTime);
    ticks(kernel)
        .checked_add(ticks(user))
        .map(|ticks| ticks / 10)
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub(crate) enum Stage {
    EngineBegin,
    GeometryBegin,
    BridgeBegin,
    Module,
    Apply,
    Microtasks,
    Timers,
    Animation,
    ResizeBegin,
    Resize,
    Collect,
    OutputParse,
    CanvasSnapshots,
    FinalValidation,
    Encode,
    Complete,
    CanvasBegin,
    CanvasAdmission,
    CanvasParse,
    CanvasRenderer,
    CanvasRendererReady,
    CanvasDraw,
    CanvasBacking,
    CanvasBitmapReady,
    CanvasTargetReady,
    CanvasBindReady,
    CanvasInitialEndDraw,
    CanvasBackingReady,
    CanvasImagesReady,
    CanvasCommandsBegin,
    CanvasCommandsReady,
    CanvasEndDrawReady,
    CanvasDrawReturned,
    CanvasSubmitComplete,
    CanvasSubmitFailed,
    CanvasSnapshotAdmission,
    CanvasSnapshot,
    CanvasSnapshotComplete,
    CanvasSnapshotFailed,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Sample {
    pub(crate) stage: Stage,
    pub(crate) elapsed_us: u64,
    // Thread CPU excludes blocking/descheduling, but not CPU on other D2D threads.
    pub(crate) thread_cpu_us: Option<u64>,
}
pub(crate) struct Trace {
    started: Instant,
    started_cpu_us: Option<u64>,
    samples: Vec<Sample>,
}
impl Default for Trace {
    fn default() -> Self {
        Self {
            started: Instant::now(),
            started_cpu_us: thread_cpu_us(),
            samples: Vec::with_capacity(MAX_SAMPLES),
        }
    }
}
impl Trace {
    pub(crate) fn reset(&mut self) {
        self.started = Instant::now();
        self.started_cpu_us = thread_cpu_us();
        self.samples.clear();
    }
    pub(crate) fn mark(&mut self, stage: Stage) {
        if self.samples.len() == MAX_SAMPLES {
            self.samples.remove(0);
        }
        self.samples.push(Sample {
            stage,
            elapsed_us: self.started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64,
            thread_cpu_us: thread_cpu_us()
                .zip(self.started_cpu_us)
                .and_then(|(now, start)| now.checked_sub(start)),
        });
    }
    pub(crate) fn samples(&self) -> Vec<Sample> {
        self.samples.clone()
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Report {
    pub(crate) engine: Vec<Sample>,
    pub(crate) canvas: Vec<Sample>,
    pub(crate) engine_expired: bool,
    pub(crate) geometry_valid: bool,
    pub(crate) canvas_valid: bool,
    pub(crate) canvas_failed: bool,
}
impl Report {
    fn valid(&self) -> bool {
        [&self.engine, &self.canvas].into_iter().all(|samples| {
            samples.len() <= MAX_SAMPLES
                && samples.windows(2).all(|pair| {
                    pair[0].elapsed_us <= pair[1].elapsed_us
                        && pair[0]
                            .thread_cpu_us
                            .zip(pair[1].thread_cpu_us)
                            .is_none_or(|(a, b)| a <= b)
                })
        })
    }
}
pub(crate) fn log(report: Option<&Report>) {
    log_to(report, &mut std::io::stderr().lock());
}
fn log_to(report: Option<&Report>, writer: &mut impl Write) {
    if let Some(report) = report.filter(|report| report.valid()) {
        // 诊断写入失败不得改变原错误/回收；Debug只含上述固定enum、整数和bool。
        let _ = writeln!(writer, "native-js-diagnostic: {report:?}");
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn javascript_diagnostic_trace_is_bounded_monotonic_and_resets() {
        let mut trace = Trace::default();
        for _ in 0..1000 {
            trace.mark(Stage::Apply);
        }
        trace.mark(Stage::Complete);
        let values = trace.samples();
        assert_eq!(values.len(), MAX_SAMPLES);
        assert!(matches!(values.last().unwrap().stage, Stage::Complete));
        assert!(values
            .windows(2)
            .all(|pair| pair[0].elapsed_us <= pair[1].elapsed_us));
        assert!(values.iter().all(|sample| sample.thread_cpu_us.is_some()));
        assert!(values
            .windows(2)
            .all(|pair| pair[0].thread_cpu_us <= pair[1].thread_cpu_us));
        trace.reset();
        assert!(trace.samples().is_empty());
    }
    #[test]
    fn javascript_diagnostic_wire_accepts_absence_but_not_arbitrary_phase_or_fields() {
        let response: super::super::Response =
            serde_json::from_str(r#"{"sequence":1,"output":null}"#).unwrap();
        assert!(response.diagnostic.is_none());
        assert!(serde_json::from_str::<Sample>(
            r#"{"stage":"secret-script","elapsed_us":0,"thread_cpu_us":null}"#
        )
        .is_err());
        assert!(serde_json::from_str::<Sample>(
            r#"{"stage":"Apply","elapsed_us":0,"thread_cpu_us":null,"source":"private"}"#
        )
        .is_err());
        let mut report = Report {
            engine: vec![],
            canvas: vec![],
            engine_expired: false,
            geometry_valid: true,
            canvas_valid: true,
            canvas_failed: false,
        };
        assert!(report.valid());
        report.engine = vec![
            Sample {
                stage: Stage::Apply,
                elapsed_us: 0,
                thread_cpu_us: None,
            };
            MAX_SAMPLES + 1
        ];
        assert!(!report.valid());
    }
    #[test]
    fn javascript_diagnostic_logging_rejects_invalid_reports_and_ignores_broken_pipe() {
        let mut report = Report {
            engine: vec![
                Sample {
                    stage: Stage::Apply,
                    elapsed_us: 2,
                    thread_cpu_us: Some(2),
                },
                Sample {
                    stage: Stage::Collect,
                    elapsed_us: 1,
                    thread_cpu_us: Some(1),
                },
            ],
            canvas: vec![],
            engine_expired: false,
            geometry_valid: true,
            canvas_valid: true,
            canvas_failed: false,
        };
        let mut bytes = Vec::new();
        log_to(Some(&report), &mut bytes);
        assert!(bytes.is_empty());
        report.engine[1].elapsed_us = 3;
        assert!(!report.valid());
        report.engine.clear();
        struct Broken;
        impl Write for Broken {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        log_to(Some(&report), &mut Broken);
        assert!(report.valid());
        log_to(Some(&report), &mut bytes);
        assert!(String::from_utf8(bytes)
            .unwrap()
            .contains("native-js-diagnostic"));
    }
    #[test]
    fn javascript_diagnostic_success_scope_is_nested_and_restored_after_unwind() {
        assert!(!capture(false) && capture(true));
        with_success_reports(|| {
            assert!(capture(false));
            with_success_reports(|| assert!(capture(false)));
            assert!(capture(false));
        });
        assert!(!capture(false));
        let result = std::panic::catch_unwind(|| {
            with_success_reports(|| panic!("expected diagnostic scope unwind"))
        });
        assert!(result.is_err() && !capture(false));
    }
}
