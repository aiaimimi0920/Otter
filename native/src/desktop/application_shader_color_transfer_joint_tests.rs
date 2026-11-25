//! Explicit hidden Application joint gate with the actual installed Loom color-transfer Art.
use super::*;
use std::{io::Read, path::PathBuf, sync::atomic::AtomicBool};

const ART_ID: &str = "neuro.official/custom-1770131241684";
// Graph PNG is exported from the editor's 8-bit premultiplied BGRA surface.
// Unpremultiplication quantizes the half-alpha pixel and clears invisible RGB.
const GRAPH_RGBA: [u8; 12] = [255, 127, 63, 128, 0, 0, 0, 0, 1, 2, 3, 255];

#[test]
#[ignore = "requires a dedicated Loom daemon with the actual color-transfer process Art"]
fn installed_loom_color_transfer_commits_native_png() {
    let input = PathBuf::from(std::env::var_os("OTTER_COLOR_TRANSFER_INPUT").unwrap());
    assert!(input.is_absolute());
    let mut bytes = Vec::new();
    std::fs::File::open(&input)
        .unwrap()
        .take(4097)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= 4096);
    let input: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let manifest = PathBuf::from(input["manifest"].as_str().unwrap());
    let image = PathBuf::from(input["image"].as_str().unwrap());
    assert!(manifest.is_absolute() && image.is_absolute());
    let identity = crate::loom_discovery::Config::from_lookup(|name| {
        (name == "LOOM_MANIFEST_PATH").then(|| manifest.clone().into_os_string())
    })
    .load()
    .unwrap();
    let handshake =
        crate::loom_handshake_exchange::fetch(&identity, &AtomicBool::new(false)).unwrap();
    assert_eq!(handshake.arts().len(), 1, "use an isolated one-Art catalog");
    let art = handshake.capability(0).unwrap();
    assert_eq!(art.id, ART_ID);
    assert!(art.runtime().shader);
    let mut parameters = crate::loom_parameter_state::Parameters::new(&art).unwrap();
    parameters.set("strength", Some(json!(0))).unwrap();
    let mut selection = Selection {
        execution: Default::default(),
        identity: new_unit_id().unwrap(),
        parameters,
        catalog: Rc::new(crate::desktop::capability_job::Loaded {
            identity,
            handshake,
        }),
    };
    selection.execution.notify_downstream = false;
    let _input = crate::desktop::loom_input_job::TEST_SERIAL.lock().unwrap();
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let instance = crate::desktop::workspace_window_fixture::register();
        let state = State::new(None, false).unwrap();
        *state.selected_art.borrow_mut() = Some(selection);
        let target = state.unit_id.clone();
        let document = DocumentWindow::create(instance, state, false).unwrap();
        let mut app = Application::new(instance, document);
        app.shader_installation = crate::desktop::startup_entry::shader::resolve(
            &crate::desktop::Startup::default(),
            false,
        )
        .unwrap()
        .installation;
        assert!(app.shader_installation.as_ref().unwrap().bundle.is_none());
        let source = State::new(
            Some(crate::image_decode::Image::open(&image).unwrap()),
            false,
        )
        .unwrap();
        let source_id = source.unit_id.clone();
        app.add_state(source, false).unwrap();
        // Both formal ports get a real graph image. Strength zero supplies an independent
        // pixel oracle, including alpha, without mirroring the sample's shader algorithm.
        for port in ["input", "reference"] {
            app.workspace
                .connect(crate::workspace::Link {
                    id: new_unit_id().unwrap(),
                    from_unit: source_id.clone(),
                    from_port: "output_image".into(),
                    to_unit: target.clone(),
                    to_port: port.into(),
                })
                .unwrap();
        }
        let doc = &app.documents[0];
        crate::desktop::context_menu::execute(
            doc.window.0,
            &doc.state,
            crate::desktop::execution_start::COMMAND,
        )
        .unwrap();
        app.start_requested_preparations(false);
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            app.pending_windows(false).unwrap();
            let state = &app.documents[0].state;
            let status = state.execution_results.borrow().status();
            assert_ne!(
                status,
                Status::Failed,
                "{:?}",
                state.execution_results.borrow().error()
            );
            assert!(
                state.preparation_error.borrow().is_none(),
                "{:?}",
                state.preparation_error
            );
            if status == Status::Completed && app.execution_jobs.is_empty() {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "color-transfer commit: {status:?}"
            );
            thread::sleep(Duration::from_millis(2));
        }
        assert!(app.executions.is_idle() && app.input_jobs.is_empty());
        let results = app.documents[0].state.execution_results.borrow();
        let output = results.image_source("output").expect("formal output").image;
        assert_eq!((output.width, output.height), (3, 1));
        let decoded =
            crate::image_decode::shader::png(output.encoded_png().unwrap(), 1024, &|| false)
                .unwrap();
        assert_eq!(&*decoded.rgba, &GRAPH_RGBA);
        assert_eq!(
            &*output.pixels,
            &[32, 64, 128, 128, 0, 0, 0, 0, 3, 2, 1, 255]
        );
    }
}
