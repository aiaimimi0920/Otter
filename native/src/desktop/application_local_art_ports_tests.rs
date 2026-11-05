//! 新签名端口经真实TLS→图→PNG→child；保存重建和离线仍用原owner，不冒充前台验收。
use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
#[path = "application_local_art_visibility_tests.rs"]
mod visibility;
const READ: &str = r#"
NeuroSurface.define({mount({snapshot}) {
 const inputs=snapshot.state.inputs;
 if(Object.keys(inputs).join(',')!=='原始图片')throw Error('wrong input identity');
 const input=inputs['原始图片'];
 if(!input||input.kind!=='image')throw Error('missing custom image');
 const im=NeuroSurface.image(input.resourceId);
 if(im.width!==2||im.height!==2)throw Error('image dimensions');
 NeuroSurface.commitResult({observed:{kind:'value',value:{name:'原始图片',width:im.width,height:im.height,uri:NeuroSurface.resource(input.resourceId)}}});
}});
"#;
unsafe fn red_app() -> Application {
    let instance: HINSTANCE = windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
        .unwrap()
        .into();
    let image = crate::desktop::image::Image {
        width: 2,
        height: 2,
        pixels: [0, 0, 255, 255].repeat(4).into(),
    };
    // 使用真实2×2钉图几何；未钉住的主窗是1000×720，恢复后本就按其逻辑框导出。
    let document =
        DocumentWindow::create(instance, State::new(Some(image), true).unwrap(), false).unwrap();
    Application::new(instance, document)
}
fn observed(app: &Application, index: usize) {
    let outputs = app.documents[index]
        .state
        .execution_results
        .borrow()
        .save_value_outputs()
        .unwrap();
    let value = &outputs["observed"];
    assert_eq!(value["name"], "原始图片");
    assert_eq!(value["width"], 2);
    assert_eq!(value["height"], 2);
    let bytes = STANDARD
        .decode(
            value["uri"]
                .as_str()
                .unwrap()
                .strip_prefix("data:image/png;base64,")
                .unwrap(),
        )
        .unwrap();
    let image = crate::desktop::image::Image::from_encoded(&bytes).unwrap();
    assert_eq!((image.width, image.height), (2, 2));
    assert_eq!(image.pixels.as_ref(), &[0, 0, 255, 255].repeat(4));
}
unsafe fn restored(app: &mut Application, bytes: Vec<u8>) {
    use crate::desktop::{
        session_binding::Bound,
        session_candidate::{self, VectorCandidate},
    };
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let candidate = session_candidate::prepare(bytes, None, &cancel).unwrap();
    let space = crate::session_coordinates::select(None, candidate.coordinate_space).unwrap();
    let bound = Bound::prepare_for(
        VectorCandidate::prepare(candidate, &cancel).unwrap(),
        None,
        &cancel,
    )
    .map_err(|failure| failure.1)
    .unwrap();
    let parts = bound
        .into_parts_for(None, &cancel)
        .unwrap_or_else(|_| panic!("restored catalog changed"));
    let prepared = parts.prepare_states(&cancel).unwrap();
    let batch = app
        .prepare_restored_batch(prepared, &cancel, |p| Ok(space.physical(p.rect).unwrap()))
        .unwrap();
    app.publish_restored_batch(batch, false).unwrap();
    // 与生产restore_job相同，窗口发布后保留已验证元数据，供下一次保存使用。
    app.restored_session.as_mut().unwrap().space = Some(space);
}
#[test]
fn custom_input_reaches_child_and_survives_saved_window_rebuild_and_offline_execution() {
    let _javascript = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    unsafe {
        let _environment = environment();
        let mut app = red_app();
        let peer = peer::Peer::with_input(READ, "原始图片");
        let config = peer.config();
        let origin = peer.origin.clone();
        let trust = peer.authority();
        peer.authorize(&mut app.authorization_owner);
        app.art_issuers = peer.authority();
        let from = app.documents[0].state.unit_id.to_string();
        app.open_local_art(&origin, "vendor/application".into(), &from, peer.config())
            .unwrap();
        let before = crate::desktop::link_catalog::Catalog::capture(
            &app.documents,
            &app.live_sessions,
            &app.workspace,
        );
        settle_local(&mut app, (1, 1));
        peer.finish();
        observed(&app, 1);
        assert_eq!(app.workspace.links()[0].to_port, "原始图片");
        assert!(!before.matches(&app.documents, &app.live_sessions, &app.workspace));
        let catalog = crate::desktop::link_catalog::Catalog::capture(
            &app.documents,
            &app.live_sessions,
            &app.workspace,
        );
        assert_eq!(catalog.endpoints[1].inputs, vec!["原始图片"]);
        assert!(app.documents[1].state.selected_art.borrow().is_none());
        let target = app.documents[1].state.unit_id.clone();
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let (_, snapshot) = app.capture_session_snapshot().unwrap();
        let encoded = snapshot.encode(&cancel).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&encoded.bytes).unwrap();
        assert_eq!(value["stickers"][0]["w"].as_f64(), Some(2.0));
        assert_eq!(value["stickers"][0]["h"].as_f64(), Some(2.0));
        assert_eq!(
            value["stickers"][1]["otterNativeLocalArtPorts"]["ports"]["inputs"][0]["name"],
            "原始图片"
        );
        app.shutdown();
        let mut reopened = red_app();
        restored(&mut reopened, encoded.bytes.to_vec());
        assert_eq!(reopened.documents[1].state.unit_id, target);
        assert_eq!(reopened.workspace.links()[0].to_port, "原始图片");
        observed(&reopened, 1);
        let old = reopened.documents[1]
            .state
            .execution_results
            .borrow()
            .output_stamp();
        reopened.art_issuers = trust;
        assert!(reopened
            .authorization_owner
            .lease(&origin, crate::desktop::surface_windows::now_ms().unwrap())
            .is_err());
        reopened
            .open_local_art(
                &origin,
                "vendor/application".into(),
                target.as_str(),
                config,
            )
            .unwrap();
        settle_local(&mut reopened, (1, 1));
        observed(&reopened, 1);
        assert!(!old.matches(&reopened.documents[1].state.execution_results.borrow()));
        assert!(reopened.documents[1].state.selected_art.borrow().is_none());
        reopened.shutdown();
        assert!(!reopened.local_arts.busy());
    }
}
#[test]
fn live_custom_input_uses_real_pixels_and_config_change_rejects_queued_result() {
    let _javascript = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    unsafe {
        let _environment = environment();
        for changed in [false, true] {
            let mut app = app();
            let (_, unit) = crate::desktop::application::live::freeze::tests::live(&mut app, true);
            let peer = peer::Peer::with_input(READ, "原始图片");
            peer.authorize(&mut app.authorization_owner);
            app.art_issuers = peer.authority();
            app.open_local_art_child(
                &peer.origin,
                "vendor/application".into(),
                unit.as_str(),
                peer.config(),
                |_| true,
            )
            .unwrap();
            let deadline = Instant::now() + Duration::from_secs(10);
            while app.local_arts.test_counts() != (1, 1) {
                app.local_arts.poll(
                    &app.documents,
                    &mut app.workspace,
                    &app.live_sessions,
                    false,
                );
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(1));
            }
            peer.finish();
            let catalog = crate::desktop::link_catalog::Catalog::capture(
                &app.documents,
                &app.live_sessions,
                &app.workspace,
            );
            if changed {
                let config = app.documents[1].state.local_art_id.as_ref().unwrap();
                let mut record = config.ports.borrow().as_deref().unwrap().clone();
                record.package_digest = "c".repeat(64);
                *config.ports.borrow_mut() = Some(std::sync::Arc::new(record));
                assert!(!catalog.matches(&app.documents, &app.live_sessions, &app.workspace));
            }
            app.process_local_arts(false);
            if changed {
                assert!(app.documents[1]
                    .state
                    .execution_results
                    .borrow()
                    .save_value_outputs()
                    .is_none());
                settle_local(&mut app, (0, 0));
            } else {
                observed(&app, 1);
            }
            assert_eq!(app.workspace.links()[0].to_port, "原始图片");
            assert!(app.live_sessions[0].is_alive());
            app.shutdown();
        }
    }
}
#[test]
fn signed_empty_inputs_deliver_no_pixels_and_expose_no_fabricated_outputs() {
    let _javascript = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    unsafe {
        let _environment = environment();
        let mut app = red_app();
        let source="NeuroSurface.define({mount({snapshot}){if(Object.keys(snapshot.state.inputs).length)throw Error('unexpected pixels');NeuroSurface.commitResult({empty:{kind:'value',value:true}})}});";
        let peer = peer::Peer::with_ports(
            source,
            crate::art_execution_grant::package::ports::Ports {
                inputs: vec![],
                outputs: vec![],
            },
        );
        peer.authorize(&mut app.authorization_owner);
        app.art_issuers = peer.authority();
        let from = app.documents[0].state.unit_id.to_string();
        app.open_local_art(
            &peer.origin,
            "vendor/application".into(),
            &from,
            peer.config(),
        )
        .unwrap();
        settle_local(&mut app, (1, 1));
        peer.finish();
        assert!(app.workspace.links().is_empty());
        let config = app.documents[1].state.local_art_id.as_ref().unwrap();
        assert!(config.inputs().is_empty());
        assert!(config.outputs().is_empty());
        assert_eq!(
            app.documents[1]
                .state
                .execution_results
                .borrow()
                .save_value_outputs()
                .unwrap()["empty"],
            true
        );
        let unit = app.documents[1].state.unit_id.clone();
        assert!(
            crate::desktop::application::add_art::source::Source::capture(&app, &unit).is_err()
        );
        app.shutdown();
    }
}
#[test]
fn custom_image_output_connects_next_local_child_using_declared_port() {
    use crate::art_execution_grant::package::ports::{Port, Ports};
    let _javascript = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    unsafe {
        let _environment = environment();
        let mut app = red_app();
        let source="NeuroSurface.define({mount({root,snapshot}){if(!snapshot.state.inputs['原始图片'])throw Error('missing input');const c=document.createElement('canvas');c.id='result';c.width=2;c.height=2;root.appendChild(c);const x=c.getContext('2d');x.fillStyle='red';x.fillRect(0,0,2,2);NeuroSurface.commitResult({rendered:{kind:'canvas',nodeId:'result'}})}});";
        let peer = peer::Peer::with_ports(
            source,
            Ports {
                inputs: vec![Port {
                    name: "原始图片".into(),
                    kind: "image".into(),
                }],
                outputs: vec![Port {
                    name: "rendered".into(),
                    kind: "image".into(),
                }],
            },
        );
        peer.authorize(&mut app.authorization_owner);
        app.art_issuers = peer.authority();
        let from = app.documents[0].state.unit_id.to_string();
        app.open_local_art(
            &peer.origin,
            "vendor/application".into(),
            &from,
            peer.config(),
        )
        .unwrap();
        settle_local(&mut app, (1, 1));
        let first_issuer = peer.issuer();
        peer.finish();
        let from = app.documents[1].state.unit_id.to_string();
        let next = peer::Peer::with_input(READ, "原始图片");
        next.authorize(&mut app.authorization_owner);
        // 保留第一 origin 的活跃租约；替换整个 Authority 会正确撤销它。
        app.art_issuers
            .replace(crate::art_issuer_trust::Document {
                issuers: vec![first_issuer, next.issuer()],
            })
            .unwrap();
        app.open_local_art_child(
            &next.origin,
            "vendor/application".into(),
            &from,
            next.config(),
            |_| true,
        )
        .unwrap();
        settle_local(&mut app, (2, 2));
        next.finish();
        assert_eq!(app.workspace.links()[1].from_port, "rendered");
        assert_eq!(app.workspace.links()[1].to_port, "原始图片");
        observed(&app, 2);
        app.shutdown();
    }
}
