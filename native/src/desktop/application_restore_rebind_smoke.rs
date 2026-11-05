//! 在已离线恢复的独立隐藏工作区验证迟到目录；不联网、不执行节点。
use super::*;
use crate::desktop::capability_job::Loaded;
use std::rc::Rc;

pub(in crate::desktop::application) fn verify(app: &mut Application) -> Result<()> {
    let base = crate::desktop::application::restore_batch::smoke::catalog()?;
    // 缺失能力只记一次尝试；后续新的目录身份必须允许重试。
    app.catalog.borrow_mut().loaded = Some(base.clone());
    app.rebind_restored_arts()?;
    app.rebind_restored_arts()?;
    let record = app.restored_session.as_ref().unwrap();
    if record.attempted_arts.len() != 1 || record.pending_arts.len() != 1 {
        return Err(Error::new(
            E_FAIL,
            "Missing capability retried or lost pending Art",
        ));
    }
    let handshake = crate::loom_handshake::response(br#"{"protocolVersion":"loom.hook.v1","serverName":"fixture","serverVersion":"1","sessionId":"fixture","transport":"websocket","capabilities":{"artDefinitions":[{"id":"unavailable","parameters":[{"id":"opaque","secret":true}],"inputs":[],"outputs":[]}],"operations":[],"surface":{"apiVersion":"1","input":{"pointer":false,"hover":false,"touch":false,"keyboard":false},"runtimes":[],"nodes":[],"transports":[],"capabilities":[]}}}"#)
        .map_err(|_| Error::new(E_FAIL, "Invalid rebind fixture catalog"))?
        .ok_or_else(|| Error::new(E_FAIL, "Missing rebind fixture handshake"))?;
    let catalog = Rc::new(Loaded {
        identity: base.identity.clone(),
        handshake,
    });
    app.catalog.borrow_mut().loaded = Some(catalog.clone());
    let index = app
        .documents
        .iter()
        .position(|doc| doc.state.unit_id.as_str() == "offline-art")
        .ok_or_else(|| Error::new(E_FAIL, "Missing offline Art fixture"))?;
    let original = app.restored_session.as_ref().unwrap().original.clone();
    let stamp = app.documents[index]
        .state
        .canvas
        .borrow()
        .editor
        .document_stamp();
    app.documents[index].state.file_dialog.set(true);
    let blocked = app.rebind_restored_arts();
    app.documents[index].state.file_dialog.set(false);
    blocked?;
    if app.documents[index].state.selected_art.borrow().is_some() {
        return Err(Error::new(E_FAIL, "Rebinding changed modal workspace"));
    }
    // 使用常规目录处理入口，验证完成后的应用循环实际安装绑定。
    app.process_catalog();
    let identity = {
        let selected = app.documents[index].state.selected_art.borrow();
        let selected = selected
            .as_ref()
            .ok_or_else(|| Error::new(E_FAIL, "Late catalog did not bind restored Art"))?;
        if !Rc::ptr_eq(&selected.catalog, &catalog) || selected.parameters.get("opaque").is_some() {
            return Err(Error::new(
                E_FAIL,
                "Late binding bypassed catalog identity or secret filtering",
            ));
        }
        selected.identity.clone()
    };
    app.rebind_restored_arts()?;
    // 已选中的节点不被更新目录覆盖，即使新目录不再提供该能力。
    app.catalog.borrow_mut().loaded = Some(base);
    app.rebind_restored_arts()?;
    let state = &app.documents[index].state;
    if state.selected_art.borrow().as_ref().unwrap().identity != identity
        || state.pending_start.borrow().is_some()
        || state.execution_results.borrow().active().is_some()
        || !stamp.matches(&state.canvas.borrow().editor)
        || !app
            .restored_session
            .as_ref()
            .unwrap()
            .original
            .same_storage(&original)
    {
        return Err(Error::new(
            E_FAIL,
            "Late binding repeated, executed or changed preserved data",
        ));
    }
    println!(
        "native-restored-rebind-smoke: modal-deferred=true secret-filtered=true no-execution=true catalog-retry=true selection-preserved=true"
    );
    Ok(())
}
