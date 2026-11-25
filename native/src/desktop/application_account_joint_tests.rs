//! Application driver for an explicit real dual-Loom fixture; use normal task queues, continuous scheduling, and checkpoints, not direct projection HTTP calls.
use super::*;
use crate::desktop::device_authorization_ui::{account::Request as AccountRequest, Request};
use crate::projection_account::scene::Index;
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Input {
    action: String,
    envelope: Option<crate::projection_account::Envelope>,
    pixel: Option<u8>,
    revision: Option<u64>,
}
#[test]
#[ignore = "explicit isolated real account service and two Looms"]
fn actual_account_application_create_import_update_reopen_and_stop() {
    unsafe {
        run(|instance| {
            let path =
                |name| PathBuf::from(std::env::var_os(name).expect("isolated path required"));
            let spec = path("OTTER_PROJECTION_ACCOUNT_SPEC");
            let output = path("OTTER_PROJECTION_ACCOUNT_RESULT");
            let profile = path("OTTER_PROJECTION_ACCOUNT_PROFILE");
            let input: Input = serde_json::from_slice(&std::fs::read(&spec).unwrap()).unwrap();
            assert!(profile.is_absolute() && spec.parent() == output.parent());
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&profile).unwrap()), true)
                .unwrap();
            let connected = crate::loom_local_connection::connect(
                crate::loom_discovery::Config::from_env(),
                &profile,
                &crate::device_request_cancel::Cancel::new(),
            )
            .unwrap();
            // As with the normal connection owner, retain verified current Loom trust before installing the session.
            crate::loom_local_connection::remember(&connected).unwrap();
            let origin = connected.origin.clone();
            app.authorization_owner
                .install(
                    &origin,
                    connected.session,
                    crate::desktop::surface_windows::now_ms().unwrap(),
                )
                .unwrap();
            app.authorization.borrow_mut().origin = Some(origin.clone());
            app.authorization
                .borrow_mut()
                .consent
                .install(crate::projection_receive_policy::Policy::Confirm);
            let previous = app
                .workspace_checkpoint
                .as_ref()
                .unwrap()
                .account_bindings(&app.workspace);
            if let Some(pixel) = input.pixel {
                let doc = if let Some(binding) = previous.iter().find(|b| b.is_source()) {
                    app.documents
                        .iter()
                        .find(|d| d.state.unit_id.as_str() == binding.unit())
                        .unwrap()
                } else {
                    &app.documents[0]
                };
                let mut canvas = doc.state.canvas.borrow_mut();
                let size = canvas.editor.layers().unwrap().size;
                let mut pixels = vec![0; size.0 as usize * size.1 as usize * 4];
                for p in pixels.chunks_exact_mut(4) {
                    p.copy_from_slice(&[pixel, 83, 211, 255]);
                }
                assert!(canvas.editor.replace_source(pixels.into()));
                canvas.sync_raster_source();
                canvas.minified_source = None;
                canvas.discard_device();
            }
            settle(&mut app);
            match input.action.as_str() {
                "create" => {
                    let doc = &app.documents[0];
                    app.authorization.borrow_mut().request = Some(Request::AccountProjection(
                        Box::new(AccountRequest::Create(
                            crate::desktop::device_authorization_ui::share::SourceSelection {
                                unit: doc.state.unit_id.clone(),
                                origin: origin.clone(),
                                target: crate::desktop::popup_menu::target::MenuTarget::capture(
                                    &doc.state.canvas.borrow(),
                                ),
                                cancel: std::sync::Arc::new(AtomicBool::new(false)),
                            },
                        )),
                    ));
                }
                "import" => {
                    app.authorization.borrow_mut().request = Some(Request::AccountProjection(
                        Box::new(AccountRequest::Import {
                            envelope: input.envelope.unwrap(),
                            cancel: std::sync::Arc::new(AtomicBool::new(false)),
                        }),
                    ))
                }
                "stop" => {
                    app.authorization.borrow_mut().request = Some(Request::AccountProjection(
                        Box::new(AccountRequest::Stop(previous[0].clone())),
                    ))
                }
                "read" | "update" => {}
                _ => panic!("unknown fixture action"),
            }
            let until = Instant::now() + Duration::from_secs(35);
            let mut last_status = String::new();
            let binding = loop {
                app.process_account_projection();
                let status = app.authorization.borrow().account.status.clone();
                if status != last_status {
                    eprintln!("account-joint-status: {status}");
                    last_status = status;
                }
                settle(&mut app);
                drain_messages();
                let bindings = app
                    .workspace_checkpoint
                    .as_ref()
                    .unwrap()
                    .account_bindings(&app.workspace);
                if let Some(binding) = bindings.first().filter(|b| {
                    if input.action == "stop" {
                        b.stopped()
                    } else {
                        !b.stopped() && b.revision() >= input.revision.unwrap_or(1)
                    }
                }) {
                    if !app.account_projection.busy() {
                        break binding.clone();
                    }
                }
                assert!(
                    Instant::now() < until,
                    "account application deadline: {}",
                    app.authorization.borrow().account.status
                );
                std::thread::sleep(Duration::from_millis(10));
            };
            app.account_projection.shutdown();
            settle(&mut app);
            let bytes = std::fs::read(profile.join("workspace.json")).unwrap();
            assert!(Index::capture(&bytes.clone().into())
                .unwrap()
                .includes(&binding));
            let doc = app
                .documents
                .iter()
                .find(|d| d.state.unit_id.as_str() == binding.unit())
                .unwrap();
            let pixels = sample(doc);
            let result = serde_json::json!({"envelope":binding.envelope(),"revision":binding.revision(),"digest":binding.digest(),"stopped":binding.stopped(),"unit":binding.unit(),"pixels":pixels,"documentCount":app.documents.len(),"status":app.authorization.borrow().account.status});
            app.shutdown();
            drop(app);
            let mut reopened = new_app(instance);
            reopened
                .install_workspace_checkpoint(Some(Profile::open(&profile).unwrap()), true)
                .unwrap();
            assert!(reopened
                .workspace_checkpoint
                .as_ref()
                .unwrap()
                .includes_account(&binding, &reopened.workspace));
            let doc = reopened
                .documents
                .iter()
                .find(|doc| doc.state.unit_id.as_str() == binding.unit())
                .unwrap();
            let restored = sample(doc);
            assert_eq!(
                restored["original"], pixels["original"],
                "reopened PNG must match committed pixels"
            );
            assert_eq!(
                restored["center"], pixels["center"],
                "reopened presentation must preserve center pixels"
            );
            use std::io::Write;
            std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(output)
                .unwrap()
                .write_all(&serde_json::to_vec(&result).unwrap())
                .unwrap();
        });
    }
}

fn sample(doc: &DocumentWindow) -> serde_json::Value {
    let canvas = doc.state.canvas.borrow();
    let image = canvas.image.as_ref().unwrap();
    let original = canvas
        .minified_source
        .as_ref()
        .map(|s| s.image())
        .unwrap_or(image);
    let center = ((image.height / 2 * image.width + image.width / 2) * 4) as usize;
    serde_json::json!({"corner":image.pixels[..4],"center":image.pixels[center..center+4],
        "original":original.pixels[..4],"size":[image.width,image.height],"originalSize":[original.width,original.height]})
}
