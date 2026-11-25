//! Separate immediate confirmation when imports change execution trust; deny by default and never elevate an ordinary naming import into authorization.
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
            rows.push(crate::ui_text::format(
                "app-issuer-remove-trust",
                &[
                    ("origin", old.origin.as_str().into()),
                    ("key", old.key_id.as_str().into()),
                ],
            ));
        }
    }
    for entry in &next.issuers {
        if before.issuers.iter().any(|e| e == entry) {
            continue;
        }
        let key = STANDARD
            .decode(&entry.public_key)
            .map_err(|_| Error::new(E_INVALIDARG, "Invalid issuer key"))?;
        rows.push(crate::ui_text::format(
            "app-issuer-trust-entry",
            &[
                ("origin", entry.origin.as_str().into()),
                ("key", entry.key_id.as_str().into()),
                ("fingerprint", format!("{:x}", Sha256::digest(key)).into()),
            ],
        ));
    }
    Ok(crate::ui_text::format(
        "app-issuer-change-confirmation",
        &[("entries", rows.join("\n\n").into())],
    ))
}
pub(super) unsafe fn confirm(hwnd: HWND, before: &Document, next: &Document) -> Result<bool> {
    if !IsWindow(Some(hwnd)).as_bool() {
        return Err(Error::new(E_ABORT, "Issuer requester closed"));
    }
    let message = describe(before, next)?;
    // 32 origins may exceed readable modal length; reject rather than truncate critical fingerprints.
    if message.encode_utf16().count() > 12000 {
        return Err(Error::new(
            E_INVALIDARG,
            "Too many issuer changes for one confirmation",
        ));
    }
    Ok(MessageBoxW(
        Some(hwnd),
        &HSTRING::from(message),
        &windows::core::HSTRING::from(crate::ui_text::text("app-art-issuer-title")),
        MB_YESNO | MB_DEFBUTTON2 | MB_ICONWARNING,
    ) == IDYES)
}
