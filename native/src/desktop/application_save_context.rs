//! 新建工作区的显式保存上下文；与导入会话的历史 CSS/DPR 上下文分开建立。
use super::*;

pub(super) fn fresh(host: crate::workspace_id::UnitId) -> restore_batch::Record {
    restore_batch::Record {
        restored_projections: None,
        restored_senders: None,
        restored_projection_scope: None,
        projection_scope: None,
        disk_original: None,
        workflow_cycle: None,
        workflow_settings: None,
        workflow_sync: None,
        workflow_delivery: None,
        workflow: None,
        original: br#"{"documentSchemaVersion":1,"documentRevision":0,"stickers":[],"links":[],"groups":[]}"#.to_vec().into(),
        saved_path: None,
        recovery_path: None,
        // 新原生会话以桌面物理像素为保存坐标，不借用某一窗口所在显示器的 DPI。
        // 标注仍由保存映射按每个窗口实际 DPI 转换到此空间；负屏幕坐标合法。
        space: Some(crate::session_placement::Space { origin: (0, 0), scale: 1.0 }),
        annotation_ids: Vec::new(),
        transient_host: Some(host),
        pending_arts: Vec::new(),
        binding_catalog: None,
        attempted_arts: Vec::new(),
    }
}

/// 使用刚创建的隐藏 HWND 捕获完整候选，不写文件、不替换实际恢复上下文。
pub(super) unsafe fn verify(app: &mut Application) -> Result<()> {
    if app.save_status() != save_state::Status::Unsaved {
        return Err(Error::new(E_FAIL, "Fresh workspace was marked saved"));
    }
    if !app.save_requires_destination(false) || !app.save_requires_destination(true) {
        return Err(Error::new(
            E_FAIL,
            "Fresh workspace must request a destination",
        ));
    }
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let host = app.documents[0].state.unit_id.clone();
    let (receipt, empty) = app.capture_session_snapshot()?;
    let empty = empty
        .encode(&cancel)
        .map_err(|error| Error::new(E_FAIL, format!("Cannot encode fresh workspace: {error:?}")))?;
    let decoded = crate::session_document::decode(&empty.bytes)
        .map_err(|_| Error::new(E_FAIL, "Invalid fresh workspace candidate"))?;
    if empty.expected_revision != 0
        || empty.revision != 1
        || !decoded.workspace.units().is_empty()
        || !receipt.matches(app)
    {
        return Err(Error::new(E_FAIL, "Fresh workspace baseline mismatch"));
    }
    app.documents[0].state.canvas.borrow_mut().pinned = true;
    let result = (|| {
        let (_, active) = app.capture_session_snapshot()?;
        let active = active
            .encode(&cancel)
            .map_err(|error| Error::new(E_FAIL, format!("Cannot encode fresh pin: {error:?}")))?;
        let decoded = crate::session_document::decode(&active.bytes)
            .map_err(|_| Error::new(E_FAIL, "Invalid fresh pin candidate"))?;
        if !decoded.workspace.contains(&host) || receipt.matches(app) {
            return Err(Error::new(
                E_FAIL,
                "Fresh host pin was omitted or old receipt accepted",
            ));
        }
        Ok(())
    })();
    app.documents[0].state.canvas.borrow_mut().pinned = false;
    result?;
    println!(
        "native-fresh-save-smoke: empty-host-omitted=true active-host-preserved=true revision=1"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fresh_context_has_explicit_physical_space_and_no_import_bindings() {
        let host = crate::workspace_id::UnitId::parse("fresh-host").unwrap();
        let record = fresh(host.clone());
        let document = crate::session_document::decode(&record.original).unwrap();
        assert_eq!(document.revision, 0);
        assert!(document.workspace.units().is_empty());
        assert_eq!(record.transient_host, Some(host));
        assert!(record.saved_path.is_none());
        assert!(record.pending_arts.is_empty() && record.attempted_arts.is_empty());
        assert!(record.binding_catalog.is_none() && record.annotation_ids.is_empty());
        let space = record.space.unwrap();
        assert_eq!(space.origin, (0, 0));
        assert_eq!(space.scale, 1.0);
        let frame = crate::pin_geometry::PinRect {
            x: -1920,
            y: -100,
            width: 640,
            height: 480,
        };
        assert_eq!(
            space.physical(space.logical(frame, None).unwrap()).unwrap(),
            frame
        );
    }
}
