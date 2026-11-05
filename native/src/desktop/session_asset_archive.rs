//! 工作流归档资源批次；逐张发布并持有文件句柄，成功后才交付完整引用和索引。
use super::{
    publish::{Published, Slot},
    Root,
};
use crate::session_document::workflow_snapshot::metadata::{archive, Plan};
use serde::Deserialize;
use std::{
    borrow::Cow,
    sync::atomic::{AtomicBool, Ordering},
};
use windows::{
    core::{Error, Result},
    Win32::Foundation::{E_ABORT, E_INVALIDARG},
};

pub struct Asset {
    pub sticker_id: String,
    pub src: String,
    pub preview_src: Option<String>,
}
pub struct Batch {
    assets: Vec<Asset>,
    index: serde_json::Value,
    adoptions: Vec<crate::session_document::workflow_snapshot::metadata::Adoption>,
    source_digest: [u8; 32],
    // 会话候选使用引用期间必须保持 Batch 存活；最后才释放资源文件句柄。
    _pins: Vec<Published>,
}
pub struct Prepared {
    candidate: crate::session_write::Candidate,
    _pins: Vec<Published>,
}
impl Prepared {
    /// 只借出会话候选，资源句柄随 Prepared 保留到调用方结束磁盘提交。
    pub fn candidate(&self) -> &crate::session_write::Candidate {
        &self.candidate
    }
}
impl Batch {
    pub fn into_candidate(self, saved: &[u8], cancel: &AtomicBool) -> Result<Prepared> {
        check(cancel)?;
        if !self.matches(saved) {
            return Err(invalid(
                "Archive batch source changed before session assembly",
            ));
        }
        let references: Vec<_> = self
            .assets
            .iter()
            .map(|asset| archive::Asset {
                sticker_id: &asset.sticker_id,
                src: &asset.src,
                preview_src: asset.preview_src.as_deref(),
            })
            .collect();
        let candidate = crate::session_write::archive::prepare(
            saved,
            self.index,
            &references,
            &self.adoptions,
            cancel,
        )
        .map_err(|_| invalid("Cannot assemble archived session candidate"))?;
        Ok(Prepared {
            candidate,
            _pins: self._pins,
        })
    }
    pub fn assets(&self) -> &[Asset] {
        &self.assets
    }
    pub fn index(&self) -> &serde_json::Value {
        &self.index
    }
    pub fn matches(&self, saved: &[u8]) -> bool {
        use sha2::Digest;
        saved.len() <= crate::session_document::MAX_SESSION_BYTES
            && <[u8; 32]>::from(sha2::Sha256::digest(saved)) == self.source_digest
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Sticker<'a> {
    #[serde(borrow)]
    id: Cow<'a, str>,
    #[serde(default, borrow)]
    src: Cow<'a, str>,
    #[serde(default, borrow)]
    preview_src: Option<Cow<'a, str>>,
}
#[derive(Deserialize)]
struct Source<'a> {
    #[serde(borrow)]
    stickers: Vec<Sticker<'a>>,
}
pub fn prepare(
    root: &Root,
    plan: &Plan,
    saved: &[u8],
    timestamp_ms: u64,
    cancel: &AtomicBool,
) -> Result<Batch> {
    check(cancel)?;
    if !plan.matches(saved) {
        return Err(invalid("Archive source no longer matches workflow plan"));
    }
    let source: Source<'_> =
        serde_json::from_slice(saved).map_err(|_| invalid("Invalid archive source"))?;
    if source.stickers.len() > crate::workspace::MAX_UNITS {
        return Err(invalid("Archive asset count exceeds budget"));
    }
    let mut assets = Vec::new();
    let mut pins = Vec::new();
    for sticker in source.stickers {
        check(cancel)?;
        let src = publish(
            root,
            &sticker.id,
            Slot::Source,
            &sticker.src,
            &mut pins,
            cancel,
        )?;
        let preview_src = sticker
            .preview_src
            .as_ref()
            .map(|reference| {
                publish(
                    root,
                    &sticker.id,
                    Slot::Preview,
                    reference,
                    &mut pins,
                    cancel,
                )
            })
            .transpose()?;
        assets.push(Asset {
            sticker_id: sticker.id.into_owned(),
            src,
            preview_src,
        });
    }
    let references: Vec<_> = assets
        .iter()
        .map(|asset| archive::Asset {
            sticker_id: &asset.sticker_id,
            src: &asset.src,
            preview_src: asset.preview_src.as_deref(),
        })
        .collect();
    let index = archive::prepare(plan, saved, &references, timestamp_ms, cancel)
        .map_err(|_| invalid("Cannot prepare workflow archive index"))?;
    check(cancel)?;
    use sha2::Digest;
    Ok(Batch {
        assets,
        index,
        adoptions: plan.adoptions().to_vec(),
        source_digest: sha2::Sha256::digest(saved).into(),
        _pins: pins,
    })
}
fn publish(
    root: &Root,
    id: &str,
    slot: Slot,
    reference: &str,
    pins: &mut Vec<Published>,
    cancel: &AtomicBool,
) -> Result<String> {
    if !reference.starts_with("data:image") {
        // 与主干一致保留普通引用；不隐式访问任意文件/URL，也不声称其内容已归档。
        if reference.len() > 32 * 1024 || reference.contains('\0') {
            return Err(invalid("Archive reference exceeds bounds"));
        }
        return Ok(reference.into());
    }
    let resource = root.publish_inline(id, slot, reference, cancel)?;
    let reference = resource.reference().to_owned();
    pins.push(resource);
    Ok(reference)
}
fn check(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::new(E_ABORT, "Archive batch cancelled"))
    } else {
        Ok(())
    }
}
fn invalid(message: &str) -> Error {
    Error::new(E_INVALIDARG, message)
}

#[cfg(test)]
#[path = "session_asset_archive_tests.rs"]
mod tests;
