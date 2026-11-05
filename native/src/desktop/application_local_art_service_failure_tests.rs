//! 原OpenArt/输入队列/结果/Save：暂态服务失败不替换本地实例，用户显式再调用独立成功。
use super::*;
const SOURCE: &str = r#"
NeuroSurface.define({mount({root,snapshot}) {
 const picture=NeuroSurface.image(snapshot.state.package.picture.resourceId);
 const button=document.createElement('button');button.textContent='Run packaged Art';
 const canvas=document.createElement('canvas');canvas.id='formal';canvas.width=2;canvas.height=2;
 root.replaceChildren(button,canvas);canvas.getContext('2d').drawImage(picture,0,0);
 button.addEventListener('click',async()=>{
  let mode='online';
  try {const value=await NeuroSurface.service('capability:vendor:service.run',{answer:42},5000);
   if(value!==null)throw Error('wrong service value');}
  catch(error){if(error.message!=='service_unavailable')throw error;mode=error.message;}
  NeuroSurface.commitResult({output_image:{kind:'canvas',nodeId:'formal'},
   answer:{kind:'value',value:42},mode:{kind:'value',value:mode}});
 });
}});
"#;
#[test]
fn application_service_failure_keeps_instance_and_formal_save_then_explicit_retry() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    unsafe {
        let _environment = environment();
        let root = root();
        let mut peer = package_peer::Peer::with_source(&root, SOURCE);
        peer.state.lock().unwrap().submit_status = Some(503);
        let mut app = image_app();
        authorize(&mut app, &peer, &root);
        app.art_issuers = trust(&peer);
        let original = app.documents[0]
            .state
            .canvas
            .borrow()
            .image
            .as_ref()
            .unwrap()
            .pixels
            .clone();
        start(&mut app, &peer, &root);
        pump(&mut app, "call-local failure result", |app| {
            values(app).is_some()
        });
        let first = saved(&mut app, "service_unavailable");
        let hwnd = app
            .local_arts
            .test_window()
            .unwrap()
            .dialog_owner()
            .unwrap();
        let target = app.documents[1].state.unit_id.clone();
        assert_eq!(app.documents.len(), 2);
        {
            let state = peer.state.lock().unwrap();
            assert_eq!(
                state
                    .requests
                    .iter()
                    .filter(|s| s.ends_with("/submit"))
                    .count(),
                1
            );
            assert_eq!(
                state
                    .requests
                    .iter()
                    .filter(|s| s.ends_with("/cancel"))
                    .count(),
                1
            );
            assert!(!state.requests.iter().any(|s| s.ends_with("/poll")));
        }
        peer.state.lock().unwrap().release = true;
        // 正式结果由Application在poll后领取；下一轮才恢复输入，不能绕过单槽背压强行点击。
        pump(&mut app, "input resumed after formal result", |app| {
            app.local_arts
                .test_window()
                .is_some_and(|window| window.pointer_enabled())
        });
        let window = app.local_arts.test_window().unwrap();
        window.javascript_queue_click_for_test(&window.javascript_first_button_for_test());
        pump(&mut app, "explicit retry formal result", |app| {
            values(app).is_some_and(|v| v["mode"] == "online")
        });
        assert_eq!(
            app.local_arts
                .test_window()
                .unwrap()
                .dialog_owner()
                .unwrap(),
            hwnd
        );
        assert_eq!(app.documents[1].state.unit_id, target);
        assert!(original.same_storage(
            &app.documents[0]
                .state
                .canvas
                .borrow()
                .image
                .as_ref()
                .unwrap()
                .pixels
        ));
        let second = saved(&mut app, "online");
        assert_ne!(first, second);
        app.shutdown();
        assert!(!app.local_arts.busy());
        assert!(app.authorization_owner.finished());
        peer.stop();
        peer.assert_no_offline_connections();
        let state = peer.state.lock().unwrap();
        assert_eq!(
            state
                .requests
                .iter()
                .filter(|s| s.ends_with("/submit"))
                .count(),
            2
        );
        assert_eq!(
            state
                .requests
                .iter()
                .filter(|s| s.ends_with("/cancel"))
                .count(),
            1
        );
        assert_eq!(
            state
                .requests
                .iter()
                .filter(|s| s.as_str() == "/v1/device-sessions")
                .count(),
            1
        );
        std::fs::write(root.join("service-failure-validated.json"),br#"{"hiddenApplication":true,"sameInstance":true,"transientFailure":"503","oneCancel":true,"explicitRetryOnly":true,"formalPixels":true,"saveRestore":true,"sourceUnchanged":true,"shutdown":true}"#).unwrap();
    }
}
