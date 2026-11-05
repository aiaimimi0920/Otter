//! 导入含执行信任变化时的独立即时确认；默认拒绝，不将普通命名导入提升为授权。
use super::*;
use crate::art_issuer_trust::Document;
use base64::{engine::general_purpose::STANDARD, Engine};
use sha2::{Digest, Sha256};
pub(super) fn describe(before: &Document, next: &Document) -> Result<String> {
    before.validate().map_err(|e| Error::new(E_INVALIDARG, e))?;
    next.validate().map_err(|e| Error::new(E_INVALIDARG, e))?;
    let mut rows = Vec::new();
    for old in &before.issuers {
        if !next.issuers.iter().any(|e| e == old) {
            rows.push(format!("移除旧信任：{} / {}", old.origin, old.key_id));
        }
    }
    for entry in &next.issuers {
        if before.issuers.iter().any(|e| e == entry) {
            continue;
        }
        let key = STANDARD
            .decode(&entry.public_key)
            .map_err(|_| Error::new(E_INVALIDARG, "Invalid issuer key"))?;
        rows.push(format!(
            "信任：{}\n{}\nSHA-256: {:x}",
            entry.origin,
            entry.key_id,
            Sha256::digest(key)
        ));
    }
    Ok(format!("此配置将更改 Art 执行许可签发者。请先通过独立可信渠道核对公钥指纹。\n移除或替换信任会停止相关 Art 请求。仅本次运行；保存配置后才持久化。\n\n{}\n\n是否接受这些信任变更？",rows.join("\n\n")))
}
pub(super) unsafe fn confirm(hwnd: HWND, before: &Document, next: &Document) -> Result<bool> {
    if !IsWindow(Some(hwnd)).as_bool() {
        return Err(Error::new(E_ABORT, "Issuer requester closed"));
    }
    let message = describe(before, next)?;
    // 32 个 origin 可能超过可读模态长度；拒绝而不是截断关键指纹。
    if message.encode_utf16().count() > 12000 {
        return Err(Error::new(
            E_INVALIDARG,
            "Too many issuer changes for one confirmation",
        ));
    }
    Ok(MessageBoxW(
        Some(hwnd),
        &HSTRING::from(message),
        w!("Art 签发者信任"),
        MB_YESNO | MB_DEFBUTTON2 | MB_ICONWARNING,
    ) == IDYES)
}
