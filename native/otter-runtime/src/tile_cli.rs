//! 独立 Tile CLI 在普通工作区/设置/全局输入初始化前分流；只接受明确 origin 和隔离 profile。
use std::{ffi::OsString, path::PathBuf};
#[derive(Debug, PartialEq)]
pub enum Command {
    Outputs,
    Run {
        output: Option<String>,
        origin: String,
        profile: Option<PathBuf>,
    },
}
pub fn parse(args: &[OsString]) -> Result<Option<Command>, &'static str> {
    if !args
        .iter()
        .any(|a| a == "--tile" || a == "--tile-output" || a == "--tile-outputs")
    {
        return Ok(None);
    }
    if args == [OsString::from("--tile-outputs")] {
        return Ok(Some(Command::Outputs));
    }
    let mut index = 1;
    let output = if args.first().is_some_and(|s| s == "--tile-output") {
        let id = args
            .get(1)
            .and_then(|s| s.to_str())
            .ok_or("tile_output_invalid")?;
        if id.len() != 72
            || !id.starts_with("monitor-")
            || !id.as_bytes()[8..].iter().all(|b| b.is_ascii_hexdigit())
        {
            return Err("tile_output_invalid");
        }
        index = 2;
        Some(id.to_owned())
    } else if args.first().is_some_and(|s| s == "--tile") {
        None
    } else {
        return Err("tile_arguments_invalid");
    };
    let mut origin = None;
    let mut profile = None;
    while index < args.len() {
        let key = &args[index];
        let value = args.get(index + 1).ok_or("tile_arguments_invalid")?;
        if key == "--loom" && origin.is_none() {
            origin = Some(
                crate::device_session_attempt::canonical_origin(
                    value.to_str().ok_or("tile_origin_invalid")?,
                )
                .map_err(|_| "tile_origin_invalid")?,
            );
        } else if key == "--profile" && profile.is_none() {
            let p = PathBuf::from(value);
            if !p.is_absolute() {
                return Err("tile_profile_invalid");
            }
            profile = Some(p);
        } else {
            return Err("tile_arguments_invalid");
        }
        index += 2;
    }
    Ok(Some(Command::Run {
        output,
        origin: origin.ok_or("tile_origin_required")?,
        profile,
    }))
}
