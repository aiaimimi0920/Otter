//! 真实隐藏批次的双侧变化检查，不显示窗口、不消费待处理命令。
use super::*;

pub(super) unsafe fn verify_requests(app: &mut Application, catalog: &Rc<Loaded>) -> Result<()> {
    let frame = crate::pin_geometry::PinRect {
        x: 0,
        y: 0,
        width: 100,
        height: 100,
    };
    let batch =
        app.prepare_restored_batch(prepared(catalog)?, &AtomicBool::new(false), |_| Ok(frame))?;
    let unit = crate::desktop::document_state::new_unit_id()?;
    let result = (|| -> Result<()> {
        let request = app
            .executions
            .begin(unit.clone(), unit.clone())
            .map_err(|_| Error::new(E_FAIL, "Cannot prepare synthetic active request"))?;
        app.executions
            .mark_preview(&unit, &request, "pending-preview".into())
            .map_err(|_| Error::new(E_FAIL, "Cannot prepare synthetic preview"))?;
        let mut converted = false;
        let attempt =
            app.prepare_restored_batch(prepared(catalog)?, &AtomicBool::new(false), |_| {
                converted = true;
                Ok(frame)
            });
        if batch.matches_application(app)
            || attempt.is_ok()
            || converted
            || app.executions.active(&unit) != Some(&request)
            || app.executions.preview(&unit, &request) != Some("pending-preview")
        {
            return Err(Error::new(
                E_FAIL,
                "Restoration ignored or consumed active graph request",
            ));
        }
        if !app.executions.finish(&unit, &request) || !batch.matches_application(app) {
            return Err(Error::new(
                E_FAIL,
                "Finished graph request kept restoration blocked",
            ));
        }
        Ok(())
    })();
    // 只清理本 smoke 的独立请求项，不重置其它节点的代际和执行记录。
    app.executions.retain_units(|id| id != &unit);
    result
}

pub(super) unsafe fn reject_inconsistent(app: &Application, catalog: &Rc<Loaded>) -> Result<()> {
    let before = windows()?;
    for case in 0..8 {
        let mut candidate = prepared(catalog)?;
        let first = candidate.states[0].unit_id.clone();
        match case {
            0 => {
                candidate.states.pop();
            }
            1 => candidate.candidate.placements[1].unit = first,
            2 => candidate.candidate.images[1].unit = first,
            3 => candidate.annotation_ids[1].0 = first,
            4 => candidate.annotation_ids[0].1.push("missing-element".into()),
            5 => candidate.states[0].group_locked.set(true),
            6 => {
                *candidate.states[1].execution_results.borrow_mut() =
                    crate::desktop::loom_result_state::Results::new(first);
            }
            _ => candidate.states[0].canvas.borrow_mut().pinned = false,
        }
        let mut converted = false;
        let result = app.prepare_restored_batch(candidate, &AtomicBool::new(false), |_| {
            converted = true;
            Err(Error::new(
                E_FAIL,
                "Inconsistent manifest reached window preparation",
            ))
        });
        if result.is_ok() || converted || windows()? != before {
            return Err(Error::new(
                E_FAIL,
                "Inconsistent restoration manifest was not rejected before HWND creation",
            ));
        }
    }
    Ok(())
}

pub(super) unsafe fn verify(app: &Application, batch: &Windows) -> Result<()> {
    for state in [&app.documents[0].state, &batch.documents[0].state] {
        for property in 0..3 {
            let previous = {
                let mut canvas = state.canvas.borrow_mut();
                let value = match property {
                    0 => &mut canvas.normal_opacity,
                    1 => &mut canvas.window_opacity,
                    _ => &mut canvas.minified_opacity,
                };
                let previous = *value;
                *value = 0.123;
                previous
            };
            let stale = !batch.matches_application(app);
            {
                let mut canvas = state.canvas.borrow_mut();
                match property {
                    0 => canvas.normal_opacity = previous,
                    1 => canvas.window_opacity = previous,
                    _ => canvas.minified_opacity = previous,
                }
            }
            if !stale || !batch.matches_application(app) {
                return Err(Error::new(
                    E_FAIL,
                    "Opacity mutation escaped restoration guard",
                ));
            }
        }
        for flag in [&state.group_locked, &state.group_hidden, &state.new_window] {
            let previous = flag.get();
            flag.set(!previous);
            let stale = !batch.matches_application(app);
            let retained = flag.get() != previous;
            flag.set(previous);
            if !stale || !retained || !batch.matches_application(app) {
                return Err(Error::new(
                    E_FAIL,
                    "Candidate state mutation escaped restoration guard",
                ));
            }
        }
    }
    let state = &batch.documents[0].state;
    state
        .canvas
        .borrow_mut()
        .editor
        .begin(
            crate::editor::Tool::Brush,
            crate::viewport::Point::default(),
        )
        .map_err(|_| Error::new(E_FAIL, "Cannot prepare candidate draft"))?;
    let stale = !batch.matches_application(app);
    state.canvas.borrow_mut().editor.cancel();
    if !stale || !batch.matches_application(app) {
        return Err(Error::new(
            E_FAIL,
            "Candidate draft escaped restoration guard",
        ));
    }
    Ok(())
}
