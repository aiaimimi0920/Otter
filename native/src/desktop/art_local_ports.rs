//! Signed-port admission transaction: recheck the same host/request, update hints and old default edges, then capture new configuration/input evidence.
use super::*;
use crate::{art_execution_grant::package::ports::Record, live_relay_http::art_prepare::Prepared};
use std::sync::Arc;
pub(super) fn accept(
    entry: &mut Entry,
    prepared: &Prepared,
    documents: &[DocumentWindow],
    workspace: &mut Workspace,
) -> Result<(), &'static str> {
    let document = documents
        .iter()
        .find(|d| d.state.unit_id == entry.unit)
        .ok_or("Art 目标已关闭")?;
    let Some(config) = document.state.local_art_id.as_ref() else {
        // Legacy direct-API tests may start default ports on ordinary documents; new evidence must not silently convert ordinary documents to Local.
        return if prepared.ports.is_none() {
            Ok(())
        } else {
            Err("Art 宿主类型已变化")
        };
    };
    if **config != prepared.binding().art_id {
        return Err("Art 宿主身份已变化");
    }
    let previous = config.ports.borrow();
    let Some(ports) = &prepared.ports else {
        return if previous.is_none() {
            Ok(())
        } else {
            Err("Art 服务缺少已保存端口的签名证明")
        };
    };
    let mut record = Record::new(prepared.binding(), ports.clone());
    record.visibility = prepared.visibility.clone();
    if previous
        .as_ref()
        .is_some_and(|old| old.visibility.is_some())
        && record.visibility.is_none()
    {
        return Err("Art 服务缺少已保存显隐的签名证明");
    }
    record.validate(config).map_err(|_| "Art 端口证明无效")?;
    if previous.as_deref() == Some(&record) {
        return Ok(());
    }
    let next = topology(
        workspace,
        &entry.unit,
        previous.is_none(),
        config.pending_input.borrow().as_ref(),
        ports,
    )?;
    let inputs = dependencies::Stamp::capture(&next, &entry.unit)?;
    drop(previous);
    // Only in-memory publication follows: no Win32, network, script callbacks, or other reentrancy points.
    *workspace = next;
    *config.ports.borrow_mut() = Some(Arc::new(record));
    config.pending_input.borrow_mut().take();
    entry.inputs = inputs;
    entry.stamp = Stamp::capture(&document.state);
    Ok(())
}
fn topology(
    workspace: &Workspace,
    unit: &UnitId,
    first: bool,
    pending: Option<&crate::workspace::Link>,
    ports: &crate::art_execution_grant::package::ports::Ports,
) -> Result<Workspace, &'static str> {
    let mut next = workspace.clone();
    if !first {
        return Ok(next);
    }
    if let Some(primary) = ports.primary_input() {
        if primary == "input_image" {
            return Ok(next);
        }
        let links: Vec<_> = next
            .links()
            .iter()
            .filter(|l| &l.to_unit == unit && l.to_port == "input_image")
            .cloned()
            .collect();
        if links.len() > 1
            || (!links.is_empty()
                && next
                    .links()
                    .iter()
                    .any(|l| &l.to_unit == unit && l.to_port == primary))
        {
            return Err("Art 旧默认输入与自定义输入冲突，原连接已保留");
        }
        for mut link in links {
            next.remove_link(&link.id);
            link.to_port = primary.into();
            next.add_link(link).map_err(|_| "Art 自定义输入连接无效")?;
        }
    } else if let Some(pending) = pending {
        if next.links().contains(pending) {
            next.remove_link(&pending.id);
        }
    }
    Ok(next)
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-loom")))]
#[path = "art_local_ports_tests.rs"]
mod tests;
