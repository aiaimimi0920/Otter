//! Explicit deletion matches only the Windows history ID used for drawing; never delete by index or clear history/current clipboard.
use super::*;
use windows::Win32::Foundation::{E_ACCESSDENIED, E_FAIL};

pub(super) fn remove(id: &str, cancel: &AtomicBool, _permit: ReadPermit) -> Result<Snapshot> {
    let deadline = Instant::now() + Duration::from_secs(5);
    check(cancel, deadline)?;
    let _apartment = Apartment::new()?;
    let result = wait::operation!(Clipboard::GetHistoryItemsAsync()?, cancel, deadline)?;
    if result.Status()? != ClipboardHistoryItemsResultStatus::Success {
        return Err(Error::new(
            E_ACCESSDENIED,
            "Clipboard history deletion unavailable",
        ));
    }
    let items = result.Items()?;
    delete_matching(
        id,
        items.Size()?.min(MAX_ITEMS),
        |index| {
            check(cancel, deadline)?;
            Ok(items.GetAt(index)?.Id()?.to_string())
        },
        |index| {
            check(cancel, deadline)?;
            Clipboard::DeleteItemFromHistory(&items.GetAt(index)?)
        },
    )?;
    Ok(Snapshot {
        status: Status::Ready,
        entries: Vec::new(),
    })
}
fn delete_matching(
    id: &str,
    count: u32,
    mut identity: impl FnMut(u32) -> Result<String>,
    mut delete: impl FnMut(u32) -> Result<bool>,
) -> Result<()> {
    for index in 0..count {
        if identity(index)? == id {
            return if delete(index)? {
                Ok(())
            } else {
                Err(Error::new(E_FAIL, "Clipboard history item was not deleted"))
            };
        }
    }
    Err(Error::new(
        E_ABORT,
        "Clipboard history item is no longer available",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deletion_matches_identity_after_reordering_and_never_falls_back_to_a_slot() {
        let ids = ["new", "painted", "other"];
        let mut deleted = Vec::new();
        delete_matching(
            "painted",
            3,
            |i| Ok(ids[i as usize].into()),
            |i| {
                deleted.push(i);
                Ok(true)
            },
        )
        .unwrap();
        assert_eq!(deleted, [1]);
        assert!(delete_matching(
            "gone",
            3,
            |i| Ok(ids[i as usize].into()),
            |_| panic!("unrelated delete")
        )
        .is_err());
    }
    #[test]
    fn system_refusal_is_an_error_and_is_not_retried() {
        let mut calls = 0;
        assert!(delete_matching(
            "id",
            2,
            |_| Ok("id".into()),
            |_| {
                calls += 1;
                Ok(false)
            }
        )
        .is_err());
        assert_eq!(calls, 1);
    }
}
