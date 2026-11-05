//! 私有服务快照实际child/封存产品worker；无网络/用户配置，也不冒称已授权产品UI。
use super::*;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::sync::atomic::{AtomicUsize, Ordering};
const SOURCE: &str = r#"
NeuroSurface.define({async mount({root}) {
 const c=document.createElement('canvas');c.width=2;c.height=1;root.appendChild(c);
 const g=c.getContext('2d');g.fillStyle='red';g.fillRect(0,0,1,1);
 g.fillStyle='rgba(0,0,255,0.5)';g.fillRect(1,0,1,1);
 const p=NeuroSurface.serviceCanvases('s',null,[c],5000);
 g.clearRect(0,0,2,1);g.fillStyle='green';g.fillRect(0,0,2,1);
 await p;
}});
"#;
struct Oracle(Arc<AtomicUsize>);
impl services::Dispatcher for Oracle {
    fn invoke(
        &mut self,
        call: services::Call,
        deadline: Instant,
        stopped: &dyn Fn() -> bool,
    ) -> io::Result<services::ResultValue> {
        assert!(!stopped() && Instant::now() < deadline);
        assert!(call.images.is_empty());
        assert_eq!(call.canvases.len(), 1);
        let frame = &call.canvases[0];
        let bytes = STANDARD.decode(&frame.png).unwrap();
        let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
            .read_info()
            .unwrap();
        assert_eq!([reader.info().width, reader.info().height], [2, 1]);
        let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
        reader.next_frame(&mut pixels).unwrap();
        assert_eq!(pixels, [255, 0, 0, 255, 0, 0, 255, 128]);
        self.0.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(350));
        Ok(services::ResultValue::Succeeded {
            output: Value::Null,
        })
    }
}
fn exercise(product: Option<std::path::PathBuf>, diagnostic: bool) {
    let count = Arc::new(AtomicUsize::new(0));
    let (session,frame)=Session::mount_options(fixture::binding(),SOURCE.into(),fixture::snapshot(1,1),serde_json::json!({}),
        services::Options{images:vec![],viewport:None,dispatcher:Some(Box::new(Oracle(count.clone()))),allow_results:false},&||false,
        |endpoint,deadline|{
            let policy=worker::Policy{suffix:if product.is_some(){" --native-javascript-worker"}else if diagnostic{" --exact surface_javascript_worker::tests::javascript_diagnostic_worker_entry --ignored --nocapture"}else{" --exact surface_javascript_worker::tests::javascript_worker_entry --ignored --nocapture"},maximum:CHILD_LIMIT,pool:&CAPACITY,environment:worker::Environment::SystemRoot};
            match &product {Some(path)=>worker::start_at(endpoint,deadline,&||false,policy,path),None=>worker::start(endpoint,deadline,&||false,policy)}
        }).unwrap();
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert_eq!(frame.images.len(), 1);
    assert_eq!(
        frame.images.values().next().unwrap().pixels.as_ref(),
        [0, 128, 0, 255, 0, 128, 0, 255]
    );
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    assert!(probe.peak_job_bytes().unwrap() <= CHILD_LIMIT);
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
fn canvas_service_actual_child_freezes_before_await_and_reclaims_after_wait() {
    let _serial = fixture::SERIAL.lock().unwrap();
    exercise(None, false);
}
#[test]
#[ignore = "one-shot actual-child phase/CPU investigation; not a replacement for the original failure"]
fn canvas_service_actual_child_diagnostic_phases_and_cpu() {
    let _serial = fixture::SERIAL.lock().unwrap();
    exercise(None, true);
}
#[test]
#[ignore = "requires explicitly sealed product EXE and external SHA-256"]
fn canvas_service_actual_product_worker_freezes_before_await_and_reclaims() {
    use sha2::{Digest, Sha256};
    let _serial = fixture::SERIAL.lock().unwrap();
    let path = std::path::PathBuf::from(std::env::var_os("OTTER_JAVASCRIPT_PRODUCT_EXE").unwrap());
    let expected = std::env::var("OTTER_JAVASCRIPT_PRODUCT_SHA256").unwrap();
    assert!(path.is_absolute());
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
        expected
    );
    exercise(Some(path.clone()), false);
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(path).unwrap())),
        expected
    );
}
