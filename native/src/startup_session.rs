//! 会话启动参数；显式坐标必须成对，否则从文件读取，不根据显示器猜历史 DPR。
use std::{ffi::OsString, path::PathBuf};
pub struct Session {
    pub path: PathBuf,
    pub assets: Option<PathBuf>,
    pub space: Option<otter_native::session_placement::Space>,
    pub check: bool,
}
pub fn parse(args: &[OsString]) -> Result<Option<Session>, &'static str> {
    if !args
        .iter()
        .any(|arg| arg.to_string_lossy().starts_with("--session"))
    {
        return Ok(None);
    }
    let (mut path, mut assets, mut origin, mut scale) = (None, None, None, None);
    let mut check = false;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if arg == "--session-check" {
            if check {
                return Err("Duplicate --session-check");
            }
            check = true;
            continue;
        }
        let value = args.next().ok_or("Session option requires a value")?;
        if arg == "--session" && path.is_none() {
            path = Some(PathBuf::from(value));
        } else if arg == "--session-assets" && assets.is_none() {
            assets = Some(PathBuf::from(value));
        } else if arg == "--session-origin" && origin.is_none() {
            let (x, y) = value
                .to_str()
                .and_then(|value| value.split_once(','))
                .ok_or("Expected --session-origin X,Y")?;
            origin = Some((
                x.parse().map_err(|_| "Invalid origin X")?,
                y.parse().map_err(|_| "Invalid origin Y")?,
            ));
        } else if arg == "--session-scale" && scale.is_none() {
            let value: f64 = value
                .to_str()
                .ok_or("Invalid session scale")?
                .parse()
                .map_err(|_| "Invalid session scale")?;
            if !value.is_finite() || value <= 0.0 {
                return Err("Session scale must be finite and positive");
            }
            scale = Some(value);
        } else {
            return Err("Unknown, duplicate or incompatible session option");
        }
    }
    let space = match (origin, scale) {
        (Some(origin), Some(scale)) => {
            Some(otter_native::session_placement::Space { origin, scale })
        }
        (None, None) => None,
        _ => return Err("--session-origin and --session-scale must be supplied together"),
    };
    Ok(Some(Session {
        path: path.ok_or("--session requires a file path")?,
        assets,
        space,
        check,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(value: &str) -> Vec<OsString> {
        value.split_whitespace().map(Into::into).collect()
    }
    #[test]
    fn explicit_context_and_unicode_paths_are_preserved() {
        assert!(parse(&args("--session native.json --session-check"))
            .unwrap()
            .unwrap()
            .space
            .is_none());
        let parsed = parse(&args(
            "--session 会话.json --session-origin -1920,-200 --session-scale 1.5 --session-check",
        ))
        .unwrap()
        .unwrap();
        assert_eq!(parsed.path, PathBuf::from("会话.json"));
        assert_eq!(parsed.space.unwrap().origin, (-1920, -200));
        assert_eq!(parsed.space.unwrap().scale, 1.5);
        assert!(parsed.check);
        assert!(parse(&args("image.png --pin")).unwrap().is_none());
    }
    #[test]
    fn missing_context_duplicate_and_invalid_options_fail() {
        for tail in [
            "--session-scale 1",
            "--session-origin 0,0",
            "--session-origin 0,0 --session-scale NaN",
            "--session-origin 0,0 --session-scale -1",
            "--session-origin 0,0 --session-scale 1 --pin",
            "--session-origin 0,0 --session-scale 1 --session-check --session-check",
        ] {
            assert!(
                parse(&args(&format!("--session x.json {tail}"))).is_err(),
                "{tail}"
            );
        }
    }
}
