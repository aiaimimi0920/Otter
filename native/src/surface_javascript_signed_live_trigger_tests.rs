//! 固定回调走真实签名child；不伪造click/焦点，不恢复暂停timer，停止拒绝迟到结果。
use super::*;
use crate::art_live_trigger::{tests as f, ticket::Ticket};
const SOURCE: &str = r#"
let label, field, clicks=0, timers=0;
NeuroSurface.define({mount({root}) {
 label=document.createElement('span');label.textContent='initial';
 field=document.createElement('input');field.id='field';
 const button=document.createElement('button');button.textContent='click';button.addEventListener('click',()=>clicks++);
 root.replaceChildren(label,field,button);setTimeout(()=>timers++,1);
},onLiveTrigger(event) {
 field.focus();label.textContent=JSON.stringify([event.delivery.callback,event.delivery.observation.value.text,clicks,timers]);
 NeuroSurface.commitResult({data:{kind:'value',value:{count:1,text:event.delivery.observation.value.text}}});
}});
"#;
pub(crate) fn start(source: &'static str) -> (Runtime, crate::device_authorization_lease::Grant) {
    let (owner, lease) = fixture::lease();
    let mut grant = fixture::grant(source);
    grant
        .permissions
        .push(crate::art_execution_grant::wire::Permission::WorkspaceResult);
    let current = grant.binding.clone();
    let expiry = grant.expires_at_ms;
    let worker = lease.clone();
    let runtime = Runtime::spawn(
        current,
        lease,
        (expiry, Instant::now() + Duration::from_secs(60)),
        true,
        move || fixture::mount(&grant, source, worker, &|| false).map_err(Error::Local),
    )
    .unwrap();
    (runtime, owner)
}
fn submit(
    runtime: &mut Runtime,
) -> (
    std::sync::Arc<crate::art_live_trigger::ticket::Authorization>,
    Ticket,
) {
    let (gate, ticket) = f::ticket(runtime.binding.clone());
    runtime
        .submit(
            &runtime.binding.clone(),
            Operation::LiveTrigger(
                ticket.clone(),
                crate::surface_viewport::Viewport::new([640, 480], 96).unwrap(),
            ),
        )
        .unwrap();
    (gate, ticket)
}
#[test]
fn paused_callback_produces_formal_result_without_click_focus_or_timer_resume() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let (mut runtime, _owner) = start(SOURCE);
    reply(&mut runtime);
    runtime
        .submit(&runtime.binding.clone(), Operation::Suspend)
        .unwrap();
    reply(&mut runtime);
    let (_gate, ticket) = submit(&mut runtime);
    let frame = reply(&mut runtime);
    assert_eq!(
        crate::surface_javascript::fixture::text(&frame.scene),
        r#"["onLiveTrigger","eight",0,0]"#
    );
    assert!(frame.focus.is_empty());
    assert!(!frame.animation_pending);
    assert!(frame.timer_delay_ms.is_none());
    assert!(frame.events.is_empty());
    assert!(ticket.current(ticket.binding()));
    let result = frame.result.unwrap();
    assert!(
        matches!(&result.outputs["data"],crate::surface_javascript_wire::result::ResolvedOutput::Value(v) if v==&serde_json::json!({"count":1,"text":"eight"}))
    );
    assert!(
        ticket.receipt().is_none(),
        "child result is not an Applied ACK"
    );
    runtime.request_stop();
    exit(&mut runtime);
    assert_eq!(crate::surface_javascript::CAPACITY.used(), 0);
}
#[test]
fn missing_callback_fails_and_reclaims_original_child() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let (mut runtime, _owner) = start(fixture::SOURCE);
    reply(&mut runtime);
    let (_gate, _ticket) = submit(&mut runtime);
    exit(&mut runtime);
    assert!(runtime.take_reply(&runtime.binding.clone()).is_none());
    assert_eq!(crate::surface_javascript::CAPACITY.used(), 0);
}
#[test]
fn stopped_ticket_rejects_already_ready_callback_frame_and_result() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let (mut runtime, _owner) = start(SOURCE);
    reply(&mut runtime);
    let (gate, _ticket) = submit(&mut runtime);
    let handle = [HANDLE(runtime.mailbox.reply_handle().as_raw_handle())];
    assert_eq!(
        unsafe { WaitForMultipleObjects(&handle, false, 8000) },
        WAIT_OBJECT_0
    );
    gate.stop();
    assert!(matches!(
        runtime.take_reply(&runtime.binding.clone()),
        Some(Err(Error::Stopped))
    ));
    exit(&mut runtime);
    assert_eq!(crate::surface_javascript::CAPACITY.used(), 0);
}
