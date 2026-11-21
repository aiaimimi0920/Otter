//! OCR→命名文本→翻译的单连接顺序；每次真正读取图片/附件前重新申请单次许可。
use super::{
    extension_command_bridge::{Port, Reply, Request},
    extension_command_route::{Context, OCR, OCR_OVERLAY, TRANSLATE, TRANSLATION_OVERLAY},
};
use crate::{
    loom_extension_session::{Attachment, Error, Image, Prepared, Session, Target},
    loom_extension_snapshot::Snapshot,
};
use serde_json::{json, Value};
use std::sync::Arc;
pub(super) struct Spec {
    pub(super) catalog: Arc<Snapshot>,
    pub(super) target: Target,
    pub(super) context: Context,
    pub(super) command: String,
    pub(super) input: Value,
}
pub(super) fn run(session: &mut Session, mut spec: Spec, port: &Port) -> Result<(), Error> {
    if !session
        .snapshot()
        .is_some_and(|s| s.same_catalog(&spec.catalog))
    {
        return Err(Error::AuthorityChanged);
    }
    if let Some(translation) = match spec.command.as_str() {
        OCR_OVERLAY => Some(false),
        TRANSLATION_OVERLAY => Some(true),
        _ => None,
    } {
        if toggle(session, &spec, translation, port)? {
            return Ok(());
        }
        spec.command = if translation { TRANSLATE } else { OCR }.into();
    }
    let context = spec.context.with(|context| {
        let command = spec
            .catalog
            .command(&spec.command, context)
            .ok_or(Error::Unavailable)?;
        Ok(
            match command.payload().and_then(|p| p.get("inputContext")) {
                None => false,
                Some(v)
                    if v == "ocr-text.v1"
                        && session.supports_ocr_text()
                        && command.permits("hook.unit.attachments.read") =>
                {
                    true
                }
                _ => return Err(Error::Unsupported),
            },
        )
    })?;
    let input = if context {
        // 命名输入有独立 checkOnly；不能凭翻译插件的 opaque 读取权泄露 OCR 附件。
        spec.context.with(|context| {
            session.authorize(&spec.command, context, &spec.target, &port.cancel)
        })?;
        let toggle_type = spec.context.with(|context| {
            spec.catalog.command(&spec.command, context).and_then(|c| {
                c.payload()
                    .and_then(|p| p.get("toggleAttachmentType"))
                    .cloned()
            })
        });
        if let Some(kind) = toggle_type {
            if kind != crate::extension_visual::TRANSLATION {
                return Err(Error::Unsupported);
            }
            permitted_toggle(&spec, true)?;
            if matches!(
                port.ask(Request::Toggle { translation: true })?,
                Reply::Toggled(true)
            ) {
                return Ok(());
            }
        }
        match input(port)? {
            Some(value) => value,
            None => {
                spec.context = invoke(session, &spec, OCR, json!({}), port)?;
                input(port)?.ok_or(Error::Unavailable)?
            }
        }
    } else {
        spec.input.clone()
    };
    invoke(session, &spec, &spec.command, input, port)?;
    Ok(())
}
fn permitted_toggle(spec: &Spec, translation: bool) -> Result<(), Error> {
    spec.context.with(|context| {
        let c = spec
            .catalog
            .command(&spec.command, context)
            .ok_or(Error::Unavailable)?;
        if c.plugin()
            != if translation {
                crate::extension_visual::TRANSLATE
            } else {
                crate::extension_visual::OCR
            }
            || !c.permits("hook.unit.attachments.read")
            || !c.permits("hook.unit.attachments.write")
        {
            return Err(Error::Denied);
        }
        Ok(())
    })
}
fn toggle(
    session: &mut Session,
    spec: &Spec,
    translation: bool,
    port: &Port,
) -> Result<bool, Error> {
    permitted_toggle(spec, translation)?;
    spec.context
        .with(|context| session.authorize(&spec.command, context, &spec.target, &port.cancel))?;
    match port.ask(Request::Toggle { translation })? {
        Reply::Toggled(changed) => Ok(changed),
        _ => Err(Error::Wire),
    }
}
fn input(port: &Port) -> Result<Option<Value>, Error> {
    match port.ask(Request::Input)? {
        Reply::Input(input) => Ok(input),
        _ => Err(Error::Wire),
    }
}
fn invoke(
    session: &mut Session,
    spec: &Spec,
    command: &str,
    input: Value,
    port: &Port,
) -> Result<Context, Error> {
    let outcome = spec.context.with(|context| {
        session.invoke(
            command,
            context,
            &spec.target,
            |access| {
                let Reply::Resources { image, store } = port.ask(Request::Resources {
                    image: access.image_allowed(),
                    attachments: access.attachments_allowed(),
                })?
                else {
                    return Err(Error::Wire);
                };
                port.check()?;
                let mut prepared = Prepared::new(input)?;
                if access.image_allowed() {
                    let image = image.ok_or(Error::Unavailable)?;
                    let png =
                        super::export::png_raw(&image, &port.cancel).map_err(|_| Error::Limit)?;
                    prepared = prepared.image(Image::new("image/png", &png)?);
                }
                if let Some(store) = store {
                    for item in store
                        .items()
                        .iter()
                        .filter(|i| i.attachment.plugin() == access.plugin())
                    {
                        prepared = prepared
                            .attachment(Attachment::from_validated(item.attachment.clone()))?;
                    }
                }
                port.check()?;
                Ok(prepared)
            },
            &port.cancel,
        )
    })?;
    match port.ask(Request::Apply {
        command: command.into(),
        outcome,
    })? {
        Reply::Applied(context) => Ok(context),
        _ => Err(Error::Wire),
    }
}
