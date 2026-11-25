//! Real TLS accept -> PNG -> definite HWND append -> checkpoint -> restart restoration; hidden painting is not a displayed receipt.
use super::*;
use crate::{
    desktop::{session_binding::Bound, session_candidate},
    device_authorization_lease::Grant,
    device_session_client::tests::{accept, fixture_with_pem, reply, request, session},
    projection_delivery::accept::tests as png,
    projection_delivery_runtime::{Job, Operation, Output},
    workspace_id::UnitId,
};
use rustls::{ServerConnection, StreamOwned};
use std::os::windows::io::AsRawHandle;
use windows::Win32::{Foundation::WAIT_OBJECT_0, System::Threading::WaitForSingleObject};
#[path = "application_projection_checkpoint_tests.rs"]
mod checkpoint;

unsafe fn append_projection(
    app: &mut Application,
    value: crate::projection_delivery::accept::Verified,
    intent: &crate::projection_delivery::journal::Ticket,
) -> UnitId {
    let unit = UnitId::parse(value.unit()).unwrap();
    let space = app.restored_session.as_ref().unwrap().space.unwrap();
    let ticket = intent.retry();
    let expected_pixels = value.image().pixels.clone();
    app.capture_session_snapshot().unwrap();
    let scope = app
        .restored_session
        .as_ref()
        .unwrap()
        .projection_scope
        .unwrap();
    let source = app.restored_session.as_ref().unwrap().original.clone();
    let units = app.workspace.units().to_vec();
    let (restore, retained, expected) = std::thread::spawn(move || {
        let _com = crate::desktop::Apartment::new().unwrap();
        session_candidate::projection::prepare(
            value,
            &ticket,
            scope,
            space,
            crate::session_window::Rect {
                x: 20.0,
                y: 30.0,
                w: 160.0,
                h: 80.0,
            },
            crate::session_append::Target {
                original: &source,
                active: &units,
            },
            &AtomicBool::new(false),
        )
        .unwrap()
    })
    .join()
    .unwrap();
    assert!(restore.vector.candidate.images[0]
        .source
        .as_ref()
        .unwrap()
        .pixels
        .same_storage(&expected_pixels));
    let source = app.restored_session.as_ref().unwrap().original.clone();
    let cancel = AtomicBool::new(false);
    let index =
        crate::projection_delivery::binding::checkpoint::Index::capture(&retained.clone().into())
            .unwrap();
    assert!(index.includes(&expected));
    assert!(expected.matches_ticket(intent));
    let states = Bound::from_prepared(restore, None)
        .into_parts_for(None, &cancel)
        .ok()
        .unwrap()
        .prepare_states(&cancel)
        .unwrap();
    assert!(!states.states[0].is_art());
    assert!(states.states[0].selected_art.borrow().is_none());
    assert!(states.states[0]
        .execution_results
        .borrow()
        .ports()
        .is_empty());
    let windows = app
        .prepare_restored_batch(states, &cancel, |placement| {
            space
                .physical(placement.rect)
                .map_err(|_| Error::new(E_FAIL, "Projection frame"))
        })
        .unwrap();
    let append = app
        .prepare_append(windows, source, retained, space)
        .unwrap();
    app.publish_append(append, false).unwrap();
    unit
}

#[test]
fn projection_receive_appends_existing_workspace_and_restores_identical_pixels_from_checkpoint() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            let old = app.documents[0].window.0;
            let old_unit = app.documents[0].state.unit_id.clone();
            let (listener, client, tls, _) = fixture_with_pem();
            let origin = client.origin().to_owned();
            let expected = png::response(&origin);
            let response = serde_json::to_vec(&expected).unwrap();
            let server = std::thread::spawn(move || {
                let mut stream =
                    StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
                let (head, body) = request(&mut stream);
                assert!(head.starts_with("POST /v1/projections/accept "));
                assert_eq!(body["confirmed"], true);
                reply(&mut stream, 200, &response);
                drop(stream);
                listener
            });
            let mut grant = Grant::new(session(&origin, u64::MAX));
            let destination = grant.lease(&origin, 0).unwrap();
            let mut job = Job::start_prepared(
                client,
                grant.lease(&origin, 0).unwrap(),
                Operation::Accept {
                    invitation: Box::new(png::invitation(&origin)),
                    journal: crate::projection_delivery::journal::Store::open(&directory.0)
                        .unwrap(),
                },
            )
            .unwrap();
            assert_eq!(
                WaitForSingleObject(HANDLE(job.wait_handle().unwrap().as_raw_handle()), 5000),
                WAIT_OBJECT_0
            );
            let Output::Received(ready) = job.try_join().unwrap().unwrap() else {
                panic!("PNG not ready")
            };
            drop(job);
            let (value, held) = ready.transfer(&destination, 0).unwrap();
            let intent = held.ticket();
            let expected_pixels = value.image().pixels.clone();
            let expected_binding =
                crate::projection_delivery::binding::Binding::new(&value, intent).unwrap();
            let unit = append_projection(&mut app, value, intent);
            assert!(app.projection_checkpoint_inclusion(intent).is_none());
            let cancel = AtomicBool::new(false);
            assert_eq!(app.documents.len(), 2);
            assert_eq!(app.source_window(&old_unit), Some(old));
            assert!(app.workspace.contains(&unit));
            let hwnd = app.source_window(&unit).unwrap();
            assert!(!IsWindowVisible(hwnd).as_bool());
            settle(&mut app);
            assert!(app.projection_checkpoint_inclusion(intent) == Some(&expected_binding));
            let bytes = std::fs::read(directory.path()).unwrap();
            let saved: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            let sticker = saved["stickers"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["id"] == unit.as_str())
                .unwrap();
            assert_eq!(sticker["qrProjection"]["envelope"], expected["envelope"]);
            assert_eq!(sticker["qrProjection"]["digest"], expected["digest"]);
            assert_eq!(sticker["qrProjection"]["revision"], 5);
            assert_eq!(sticker["qrProjection"]["role"], "receiver");
            assert_eq!(sticker["qrProjection"]["localUnitId"], unit.as_str());
            assert_eq!(
                sticker["qrProjection"]["otterNativeReceive"]["intentId"],
                intent.intent_id()
            );
            let persisted =
                crate::desktop::session_image::inline(sticker["src"].as_str().unwrap(), &cancel)
                    .unwrap();
            assert_eq!((persisted.width, persisted.height), (2, 1));
            assert!(
                persisted.pixels == expected_pixels,
                "Persisted projection source pixels changed"
            );
            assert_eq!(app.workspace_checkpoint.as_ref().unwrap().revision, 1);
            // Workspace persistence does not imply displayed/unlinked; retain the intent owner for future consumers.
            assert_eq!(intent.unit(), unit.as_str());
            drop(app);
            let mut restored = new_app(instance);
            restored
                .install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            assert_eq!(restored.documents.len(), 2);
            assert!(restored.projection_checkpoint_inclusion(intent) == Some(&expected_binding));
            let doc = restored
                .documents
                .iter()
                .find(|doc| doc.state.unit_id == unit)
                .unwrap();
            {
                let canvas = doc.state.canvas.borrow();
                let source = canvas.minified_source.as_ref().unwrap().image();
                assert_eq!((source.width, source.height), (2, 1));
                assert!(
                    source.pixels == expected_pixels,
                    "Restored projection source pixels changed"
                );
                // Editor display is a projection into the logical frame, not the original 2x1 PNG.
                let display = canvas.image.as_ref().unwrap();
                assert_eq!((display.width, display.height), (160, 80));
                let projected = crate::raster_projection::project(
                    &expected_pixels,
                    (2, 1),
                    (160, 80),
                    crate::crop_geometry::Rect {
                        x: 0.0,
                        y: 0.0,
                        width: 160.0,
                        height: 80.0,
                    },
                    || false,
                )
                .unwrap();
                assert!(
                    display.pixels == projected,
                    "Restored display differs from source projection"
                );
            }
            assert!(!IsWindowVisible(doc.window.0).as_bool());
            // The TLS peer receives only accept; neither unpersisted nor hidden windows count as displayed, and no other sources are requested.
            let listener = server.join().unwrap();
            assert!(matches!(listener.accept(),Err(e) if e.kind()==std::io::ErrorKind::WouldBlock));
            restored.shutdown();
            drop(held);
        });
    }
}
