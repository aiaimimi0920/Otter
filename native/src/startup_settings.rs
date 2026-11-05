//! 显式启动配置选项；不修改环境变量，不发现默认用户目录。
use std::{ffi::OsString, path::PathBuf};

#[derive(Default)]
pub struct Options {
    pub settings: Option<PathBuf>,
    pub candidate: Option<PathBuf>,
    pub peer: Option<PathBuf>,
}
pub fn extract(args: Vec<OsString>) -> Result<(Vec<OsString>, Options), &'static str> {
    let mut rest = Vec::with_capacity(args.len());
    let mut options = Options::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let field = match arg.to_str() {
            Some("--settings") => Some(&mut options.settings),
            Some("--shader-candidate") => Some(&mut options.candidate),
            Some("--shader-smoke-peer") => Some(&mut options.peer),
            _ => None,
        };
        if let Some(field) = field {
            if field.is_some() {
                return Err("Duplicate startup path option");
            }
            let path = args.next().ok_or("Startup option requires a path")?;
            if path.is_empty() || path.to_string_lossy().starts_with('-') {
                return Err("Startup option requires a path, not another option");
            }
            *field = Some(PathBuf::from(path));
        } else {
            // 会话选项的值不能再当成顶层选项处理。
            let takes_value = matches!(
                arg.to_str(),
                Some("--session" | "--session-assets" | "--session-origin" | "--session-scale")
            );
            rest.push(arg);
            if takes_value {
                rest.push(args.next().ok_or("Session option requires a value")?);
            }
        }
    }
    if options.peer.is_some() && !rest.iter().any(|arg| arg == "--smoke-test") {
        return Err("--shader-smoke-peer requires --smoke-test");
    }
    Ok((rest, options))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }
    #[test]
    fn settings_preserve_unicode_and_other_mode_arguments() {
        let (rest, settings) = extract(args(&[
            "--settings",
            "设置 文件.json",
            "image.png",
            "--pin",
        ]))
        .unwrap();
        assert_eq!(settings.settings.unwrap(), PathBuf::from("设置 文件.json"));
        assert_eq!(rest, args(&["image.png", "--pin"]));
        let (rest, settings) =
            extract(args(&["--session", "--settings", "--session-check"])).unwrap();
        assert!(settings.settings.is_none());
        assert_eq!(rest, args(&["--session", "--settings", "--session-check"]));
        let (rest, settings) =
            extract(args(&["--capture-region", "--settings", "a.json"])).unwrap();
        assert_eq!(rest, args(&["--capture-region"]));
        assert!(settings.settings.is_some());
    }
    #[test]
    fn missing_empty_duplicate_and_option_values_are_rejected() {
        for input in [
            vec!["--settings"],
            vec!["--settings", ""],
            vec!["--settings", "--pin"],
            vec!["--settings", "a", "--settings", "b"],
            vec!["--session"],
        ] {
            assert!(extract(args(&input)).is_err());
        }
    }
    #[test]
    fn shader_paths_preserve_modes_and_do_not_reparse_session_values() {
        for mode in ["--capture", "--capture-region", "--smoke-test"] {
            let (rest, options) = extract(args(&[
                mode,
                "--shader-candidate",
                "C:/候选 DLL",
                "--settings",
                "设置.json",
            ]))
            .unwrap();
            assert_eq!(rest, args(&[mode]));
            assert_eq!(options.candidate.unwrap(), PathBuf::from("C:/候选 DLL"));
            assert_eq!(options.settings.unwrap(), PathBuf::from("设置.json"));
            assert!(options.peer.is_none());
        }
        let (rest, options) = extract(args(&[
            "--session",
            "--shader-candidate",
            "--session-check",
        ]))
        .unwrap();
        assert!(options.candidate.is_none());
        assert_eq!(
            rest,
            args(&["--session", "--shader-candidate", "--session-check"])
        );
        let (rest, options) = extract(args(&[
            "--session",
            "workspace.json",
            "--shader-candidate",
            "C:/candidate",
            "--session-check",
        ]))
        .unwrap();
        assert!(options.candidate.is_some());
        assert_eq!(
            rest,
            args(&["--session", "workspace.json", "--session-check"])
        );
        let (_, options) = extract(args(&[
            "--smoke-test",
            "--shader-candidate",
            "C:/candidate",
            "--shader-smoke-peer",
            "peer.json",
        ]))
        .unwrap();
        assert!(options.peer.is_some());
        let (_, options) =
            extract(args(&["--smoke-test", "--shader-smoke-peer", "peer.json"])).unwrap();
        assert!(options.peer.is_some() && options.candidate.is_none());
        assert!(extract(args(&["--shader-smoke-peer", "peer.json"])).is_err());
    }
}
