//! 签名 worker 复用原 LocalInput/视口/rAF/timer 协议，不授予远端 emit 或服务权限。
use super::*;
use crate::{
    surface_javascript_wire::{InputPhase, KeyPhase, KeyboardInput, TextInput},
    surface_viewport::Viewport,
};
const SOURCE: &str = r#"
let label,field;let raf=0,timer=0;
NeuroSurface.define({mount({root}){
label=document.createElement('span');label.textContent='ready';
field=document.createElement('input');field.value='';
field.addEventListener('input',()=>{label.textContent=field.value});
field.addEventListener('keydown',e=>{e.preventDefault();label.textContent='key'});
root.replaceChildren(label,field);
requestAnimationFrame(()=>{raf++;label.textContent='animation'});
setTimeout(()=>{timer++;label.textContent='timer'},0);
}});
"#;
#[test]
fn signed_worker_accepts_text_keyboard_animation_and_timer_through_original_slot() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let (mut runtime, _grant, probe) = start_source(SOURCE);
    let first = reply(&mut runtime);
    assert!(first.animation_pending);
    assert!(first.timer_delay_ms.is_some());
    let field = first.scene.root().children()[1].id().to_owned();
    let binding = runtime.binding.clone();
    let viewport = Viewport::new([640, 480], 96).unwrap();
    // AnimationFrame 也执行到期 timer；先单独验证 Timer，避免依赖回调覆盖顺序。
    runtime
        .submit(&binding, Operation::Timer(viewport))
        .unwrap();
    assert_eq!(
        crate::surface_javascript::fixture::text(&reply(&mut runtime).scene),
        "timer"
    );
    runtime
        .submit(&binding, Operation::Animation(viewport))
        .unwrap();
    assert_eq!(
        crate::surface_javascript::fixture::text(&reply(&mut runtime).scene),
        "animation"
    );
    runtime
        .submit(
            &binding,
            Operation::Input(
                TextInput {
                    target: field.clone(),
                    phase: InputPhase::Input,
                    value: "本地🙂".into(),
                }
                .into(),
                viewport,
            ),
        )
        .unwrap();
    let frame = reply(&mut runtime);
    assert_eq!(
        crate::surface_javascript::fixture::text(&frame.scene),
        "本地🙂"
    );
    assert!(frame.default_allowed.is_none());
    runtime
        .submit(
            &binding,
            Operation::Input(
                KeyboardInput {
                    target: field,
                    phase: KeyPhase::Keydown,
                    flags: 0,
                }
                .into(),
                viewport,
            ),
        )
        .unwrap();
    let frame = reply(&mut runtime);
    assert_eq!(
        crate::surface_javascript::fixture::text(&frame.scene),
        "key"
    );
    assert_eq!(frame.default_allowed, Some(false));
    runtime.request_stop();
    exit(&mut runtime);
    assert!(probe.recv_timeout(Duration::from_secs(5)).unwrap().exited());
}
