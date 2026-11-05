//! 真 HWND/图片快照到回收 worker 的隔离验证；候选只检查，不执行关闭提交。
use super::*;
use crate::desktop::application::save_snapshot::job::{Job, RecycleRequest};

pub(super) unsafe fn run(instance: HINSTANCE) -> Result<()> {
    let image = crate::desktop::image::Image::from_encoded(
        crate::desktop::application::smoke::image_fixture::PNG,
    )?;
    let first = DocumentWindow::create(instance, State::new(Some(image), false)?, false)?;
    let mut app = Application::new(instance, first);
    let source = app.restored_session.as_ref().unwrap().original.clone();
    let unit = app.documents[0].state.unit_id.clone();
    let hwnd = app.documents[0].window.0;
    for mode in 0..3 {
        app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.65;
        let (receipt, data) = app.capture_session_snapshot()?;
        let job = Job::start_recycle(
            data,
            RecycleRequest {
                unit: unit.clone(),
                entry_id: format!("recycle-worker-{mode}"),
                created_at: "2026-10-23T00:00:00.000Z".into(),
                policy: Default::default(),
                now_ms: 0,
            },
            app.execution_wake.callback(),
        )
        .map_err(|error| Error::new(E_FAIL, error.to_string()))?;
        if job.is_save() || job.is_library_update() || job.recycle_unit() != Some(&unit) {
            return Err(Error::new(E_FAIL, "Recycle purpose lost"));
        }
        app.save_job = Some((receipt, job));
        if app.request_save_candidate().is_ok() || app.take_save_candidate().is_ok() {
            return Err(Error::new(E_FAIL, "Recycle candidate admitted as save"));
        }
        // 收藏轮询必须不消费此任务，即使 worker 已经完成。
        app.poll_reference()?;
        if app.save_job.is_none() {
            return Err(Error::new(
                E_FAIL,
                "Reference consumer stole recycle candidate",
            ));
        }
        if mode == 1 {
            app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.4;
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !app.save_job.as_ref().unwrap().1.is_ready() {
            if std::time::Instant::now() >= deadline {
                return Err(Error::new(E_FAIL, "Recycle worker timeout"));
            }
            crate::desktop::smoke_timing::sleep(std::time::Duration::from_millis(1));
        }
        if mode == 0 {
            app.cancel_save_candidate(); // 已完成、未领取的结果也必须丢弃。
        }
        app.poll_reference()?;
        let (receipt, mut job) = app.save_job.take().unwrap();
        let result = job.poll().unwrap();
        if job.poll().is_some() || result.is_err() != (mode == 0) {
            return Err(Error::new(
                E_FAIL,
                "Recycle cancellation or single delivery failed",
            ));
        }
        if receipt.matches(&app) == (mode == 1) {
            return Err(Error::new(E_FAIL, "Recycle stale receipt was not detected"));
        }
        if let Ok(output) = result {
            let library = crate::session_library::decode(&output.candidate.bytes, Kind::RecycleBin)
                .map_err(|error| Error::new(E_FAIL, error))?;
            let value: serde_json::Value = serde_json::from_slice(&output.candidate.bytes)
                .map_err(|error| Error::new(E_FAIL, error.to_string()))?;
            if output.staged.is_some()
                || output.candidate.expected_revision != output.candidate.revision
                || library.entries().len() != 1
                || value["recycleBin"][0]["snapshot"]["opacityNormal"]
                    .as_f64()
                    .is_none_or(|value| (value - 0.65).abs() > 0.0001)
                || !value["recycleBin"][0]["snapshot"]["src"]
                    .as_str()
                    .is_some_and(|src| src.starts_with("data:image/png;base64,"))
            {
                return Err(Error::new(
                    E_FAIL,
                    "Recycle worker lost current image snapshot",
                ));
            }
        }
        if !IsWindow(Some(hwnd)).as_bool()
            || !app
                .restored_session
                .as_ref()
                .unwrap()
                .original
                .same_storage(&source)
        {
            return Err(Error::new(
                E_FAIL,
                "Recycle preparation published or closed early",
            ));
        }
    }
    println!("native-recycle-prepare-smoke: image=true typed=true single-slot=true late-cancel=true stale=true unpublished=true windows-preserved=true");
    Ok(())
}
