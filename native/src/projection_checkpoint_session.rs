//! Device-association indexes share original session-text traversal; readable workspaces do not imply every association category is recoverable.
use crate::{session_json_fields::fields, session_source::Parsed};
use serde::{Deserialize, Deserializer};
use serde_json::value::RawValue;
use std::{borrow::Cow, collections::BTreeMap};

pub(super) type Object<'a> = BTreeMap<String, Cow<'a, RawValue>>;

#[derive(Deserialize)]
struct Stickers<'a>(#[serde(borrow, deserialize_with = "stickers")] Vec<&'a RawValue>);
fn stickers<'de: 'a, 'a, D: Deserializer<'de>>(decoder: D) -> Result<Vec<&'a RawValue>, D::Error> {
    crate::session_document::bounded::<D, &'a RawValue, { crate::workspace::MAX_UNITS }>(decoder)
}

/// Hold only window slices/current-row field tables. If whole-row parsing fails, no index may use partial results.
pub(super) fn visit(
    parsed: &Parsed<'_>,
    mut visit: impl FnMut(&Object<'_>) -> Option<()>,
) -> Option<()> {
    let stickers: Stickers<'_> = {
        let mut root = fields(std::str::from_utf8(parsed.source()).ok()?).ok()?;
        let Cow::Borrowed(raw) = root.remove("stickers")? else {
            return None;
        };
        serde_json::from_str(raw.get()).ok()?
    };
    for raw in stickers.0 {
        visit(&fields(raw.get()).ok()?)?;
    }
    Some(())
}

#[cfg(windows)]
#[derive(Default)]
pub(crate) struct Indices {
    pub(crate) receiver: Option<super::binding::checkpoint::Index>,
    pub(crate) sender: Option<super::sender::binding::checkpoint::Index>,
}

#[cfg(windows)]
pub(crate) fn capture(parsed: &Parsed<'_>) -> Indices {
    let mut receiver = Some(super::binding::checkpoint::Index::new(parsed));
    let mut sender = Some(super::sender::binding::checkpoint::Index::new(parsed));
    let scope = parsed
        .document()
        .projection_scope
        .map(|scope| scope.to_string());
    if visit(parsed, |object| {
        // Discard only the index rejecting this row; the other side continues reading later windows.
        if receiver
            .as_mut()
            .is_some_and(|index| index.add(object).is_none())
        {
            receiver = None;
        }
        if sender
            .as_mut()
            .is_some_and(|index| index.add(object, scope.as_deref()).is_none())
        {
            sender = None;
        }
        (receiver.is_some() || sender.is_some()).then_some(())
    })
    .is_none()
    {
        return Indices::default();
    }
    Indices { receiver, sender }
}
