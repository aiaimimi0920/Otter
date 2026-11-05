//! 真正 Job child 的私有 Promise/服务往返；不是已授权 Loom 联合测试。
use super::*;
use services::{Call, Dispatcher, Mount, ResultValue};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
const SOURCE: &str = r#"
let label;
NeuroSurface.define({
 async mount({root}) {
  label=document.createElement('span'); label.textContent='pending'; root.appendChild(label);
  const value=await NeuroSurface.service('capability:vendor:service.run',{n:1},5000);
  label.textContent=value === null ? 'null-complete' : 'bad';
  const b=document.createElement('button'); b.textContent='run'; root.appendChild(b);
  b.addEventListener('click',async()=>{
   try { await NeuroSurface.service('capability:vendor:service.run',{n:2},5000); }
   catch(e) { label.textContent=e.message; }
  });
 },
});
"#;
struct ReplyWith {
    count: Arc<AtomicUsize>,
    cancelled: Arc<AtomicBool>,
    cancel: bool,
}
impl Dispatcher for ReplyWith {
    fn invoke(
        &mut self,
        call: Call,
        deadline: Instant,
        stopped: &dyn Fn() -> bool,
    ) -> io::Result<ResultValue> {
        assert!(!stopped() && Instant::now() < deadline);
        assert_eq!(call.service, "capability:vendor:service.run");
        self.count.fetch_add(1, Ordering::SeqCst);
        // 超过原 JS 250ms；原执行预算必须暂停而非重置，最终帧不能先返回。
        std::thread::sleep(Duration::from_millis(350));
        if self.cancel {
            self.cancelled.store(true, Ordering::SeqCst);
        }
        if call.input["n"] == 1 {
            Ok(ResultValue::Succeeded {
                output: Value::Null,
            })
        } else {
            Ok(ResultValue::Failed {
                error: "service_denied".into(),
            })
        }
    }
}
fn input(source: &str, dispatcher: impl Dispatcher + 'static) -> Mount {
    Mount {
        images: Vec::new(),
        binding: fixture::binding(),
        source: source.into(),
        snapshot: fixture::snapshot(1, 1),
        resources: serde_json::json!({}),
        viewport: None,
        dispatcher: Some(Box::new(dispatcher)),
        allow_results: false,
    }
}
fn responder(cancel: bool) -> (ReplyWith, Arc<AtomicUsize>, Arc<AtomicBool>) {
    let count = Arc::new(AtomicUsize::new(0));
    let cancelled = Arc::new(AtomicBool::new(false));
    (
        ReplyWith {
            count: count.clone(),
            cancelled: cancelled.clone(),
            cancel,
        },
        count,
        cancelled,
    )
}
#[test]
fn javascript_service_real_child_mount_click_null_failure_and_cleanup() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (reply, count, _) = responder(false);
    let (mut session, frame) = Session::mount_serviced(input(SOURCE, reply), &|| false).unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert_eq!(fixture::text(&frame.scene), "null-complete");
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    let target = frame.scene.root().children()[1].id().to_owned();
    let frame = session.click(&target, &|| false).unwrap();
    assert_eq!(fixture::text(&frame.scene), "service_denied");
    assert_eq!(count.load(Ordering::SeqCst), 2);
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
fn javascript_service_default_session_cannot_dispatch() {
    let _serial = fixture::SERIAL.lock().unwrap();
    assert!(Session::mount(
        fixture::binding(),
        SOURCE.into(),
        fixture::snapshot(1, 1),
        serde_json::json!({}),
        &|| false
    )
    .is_err());
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
fn javascript_service_cancellation_discards_reply_and_reclaims_child() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (reply, count, cancelled) = responder(true);
    assert!(
        Session::mount_serviced(input(SOURCE, reply), &|| cancelled.load(Ordering::SeqCst))
            .is_err()
    );
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
fn javascript_service_cpu_after_wait_is_still_bounded() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (reply, count, _) = responder(false);
    let source = "NeuroSurface.define({async mount(){await NeuroSurface.service('capability:vendor:service.run',{n:1},5000);while(true){}}});";
    let start = Instant::now();
    assert!(Session::mount_serviced(input(source, reply), &|| false).is_err());
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert!(start.elapsed() < Duration::from_secs(4));
    assert_eq!(CAPACITY.used(), 0);
}

#[test]
fn javascript_service_real_child_preserves_canvas_pixels_across_wait_and_reaps() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (reply, count, _) = responder(false);
    let source = r#"NeuroSurface.define({async mount({root}) {
        const canvas=document.createElement('canvas'); canvas.width=2; canvas.height=1;
        root.appendChild(canvas); const context=canvas.getContext('2d');
        context.fillStyle='red'; context.fillRect(0,0,1,1);
        await NeuroSurface.service('capability:vendor:service.run',{n:1},5000);
        context.fillStyle='blue'; context.fillRect(1,0,1,1);
    }});"#;
    let (session, frame) = Session::mount_serviced(input(source, reply), &|| false).unwrap();
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert_eq!(frame.images.len(), 1);
    assert_eq!(
        frame.images.values().next().unwrap().pixels.as_ref(),
        &[0, 0, 255, 255, 255, 0, 0, 255]
    );
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    assert!(probe.peak_job_bytes().unwrap() <= CHILD_LIMIT);
    assert_eq!(CAPACITY.used(), 0);
}
