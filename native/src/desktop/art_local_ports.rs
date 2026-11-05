//! 签名端口接纳事务：同一宿主/请求复核后更新提示和旧默认边，再捕获新配置/输入凭证。
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
        // 旧直接 API 测试可在普通文档上启动默认端口；新证明不能把普通文档暗转 Local。
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
    // 后续只有内存发布；没有Win32、网络、脚本回调或其他可重入点。
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
#[cfg(test)]
#[path = "art_local_ports_tests.rs"]
mod tests;
