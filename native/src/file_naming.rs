//! 主干命名占位符的有界渲染；只返回安全文件名 stem，不读写文件或解析路径权限。
#[path = "file_naming_allocate.rs"]
pub mod allocate;
#[path = "file_naming_sanitize.rs"]
mod sanitize;
pub const DEFAULT_SAVE_PATTERN: &str = "Otter_{date}_{time}_{width}x{height}";
pub const MAX_PATTERN_BYTES: usize = 4096;
pub const MAX_CONTEXT_BYTES: usize = 65536;
const KEYS: &[&str] = &[
    "app",
    "kind",
    "label",
    "title",
    "process",
    "unitId",
    "shortId",
    "width",
    "height",
    "date",
    "time",
    "timestamp",
];
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Pattern,
    Capacity,
    TitleLimit,
}
#[derive(Default)]
pub struct Context<'a> {
    pub app: &'a str,
    pub kind: &'a str,
    pub label: &'a str,
    pub title: &'a str,
    pub process: &'a str,
    pub unit_id: &'a str,
    pub short_id: &'a str,
    pub width: Option<u32>,
    pub height: Option<u32>,
}
pub struct Moment<'a> {
    pub date: &'a str,
    pub time: &'a str,
    pub timestamp: &'a str,
}

pub fn validate(pattern: &str) -> Result<(), Error> {
    visit(pattern, |_, _| Ok(()))
}
fn visit(
    pattern: &str,
    mut emit: impl FnMut(&str, bool) -> Result<(), Error>,
) -> Result<(), Error> {
    if pattern.len() > MAX_PATTERN_BYTES {
        return Err(Error::Capacity);
    }
    if pattern.trim().is_empty() {
        return Err(Error::Pattern);
    }
    let mut rest = pattern;
    while let Some(index) = rest.find(['{', '}']) {
        emit(&rest[..index], false)?;
        rest = &rest[index..];
        if rest.starts_with('}') {
            return Err(Error::Pattern);
        }
        let end = rest.find('}').ok_or(Error::Pattern)?;
        let name = &rest[1..end];
        if !KEYS.contains(&name) {
            return Err(Error::Pattern);
        }
        emit(name, true)?;
        rest = &rest[end + 1..];
    }
    emit(rest, false)
}

pub fn render(
    pattern: &str,
    context: &Context<'_>,
    moment: &Moment<'_>,
    title_limit: usize,
) -> Result<String, Error> {
    if [
        context.app,
        context.kind,
        context.label,
        context.title,
        context.process,
        context.unit_id,
        context.short_id,
        moment.date,
        moment.time,
        moment.timestamp,
    ]
    .iter()
    .any(|value| value.len() > MAX_CONTEXT_BYTES)
    {
        return Err(Error::Capacity);
    }
    if !(1..=240).contains(&title_limit) {
        return Err(Error::TitleLimit);
    }
    let app = if context.app.trim().is_empty() {
        "Otter"
    } else {
        context.app
    };
    let kind = if context.kind.trim().is_empty() {
        "image"
    } else {
        context.kind
    };
    let label = if context.label.trim().is_empty() {
        kind
    } else {
        context.label
    };
    let title: String = context.title.chars().take(title_limit).collect();
    let short: String = if context.short_id.trim().is_empty() && !context.unit_id.trim().is_empty()
    {
        context
            .unit_id
            .chars()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    } else {
        String::new()
    };
    let width = context
        .width
        .map(|value| value.to_string())
        .unwrap_or_default();
    let height = context
        .height
        .map(|value| value.to_string())
        .unwrap_or_default();
    let mut result = sanitize::Stem::default();
    visit(pattern, |text, placeholder| {
        let value = if !placeholder {
            text
        } else {
            match text {
                "app" => app,
                "kind" => kind,
                "label" => label,
                "title" => &title,
                "process" => context.process,
                "unitId" => context.unit_id,
                "shortId" => {
                    if context.short_id.trim().is_empty() {
                        &short
                    } else {
                        context.short_id
                    }
                }
                "width" => &width,
                "height" => &height,
                "date" => moment.date,
                "time" => moment.time,
                "timestamp" => moment.timestamp,
                _ => return Err(Error::Pattern),
            }
        };
        result.push(value)
    })?;
    Ok(result.finish())
}

#[cfg(test)]
#[path = "file_naming_tests.rs"]
mod tests;
