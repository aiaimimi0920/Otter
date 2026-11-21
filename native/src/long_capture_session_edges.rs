//! 固定边界只收窄不扩张；释放误认的正文行须有严格运动证据，不能吞掉真实栏变化。
use super::*;

pub(super) fn match_next(
    frames: &[Positioned],
    current: &Frame,
    axis: Axis,
    options: MatchOptions,
    old: (u32, u32),
    cancel: &AtomicBool,
) -> Result<FixedMatch, Error> {
    check(cancel)?;
    let first = &frames[0].frame;
    let previous = &frames
        .last()
        .expect("session always has a first frame")
        .frame;
    if previous.bytes() == current.bytes() {
        return Ok(FixedMatch::Unchanged);
    }
    let length = axis_length(axis, current);
    let mut leading = 0;
    while leading < old.0 && fixed::equal_line(first, current, axis, leading) {
        check(cancel)?;
        leading += 1;
    }
    let mut trailing = 0;
    while trailing < old.1 && fixed::equal_line(first, current, axis, length - trailing - 1) {
        check(cancel)?;
        trailing += 1;
    }
    let found = approximate::match_region(
        previous,
        current,
        axis,
        options,
        cancel,
        (leading, length - trailing),
    )?;
    let changed = (leading, trailing) != old;
    let ApproximateMatch::Offset { delta, mean_error } = found else {
        return if changed {
            Err(Error::ChangedEdges)
        } else {
            Ok(FixedMatch::Unresolved(found))
        };
    };
    if delta == 0 {
        return if changed {
            Err(Error::ChangedEdges)
        } else {
            Ok(FixedMatch::Unresolved(found))
        };
    }
    if changed {
        let bounds = (old, (leading, trailing));
        // 边界会应用到全部已提交帧；历史也必须支持释放，否则真实栏消失会伪装成正文。
        for pair in frames.windows(2) {
            verify_release(
                &pair[0].frame,
                &pair[1].frame,
                axis,
                bounds,
                pair[1].origin - pair[0].origin,
                cancel,
            )?;
        }
        verify_release(previous, current, axis, bounds, delta, cancel)?;
    }
    Ok(FixedMatch::Motion {
        leading,
        trailing,
        delta,
        mean_error,
    })
}

fn verify_release(
    previous: &Frame,
    current: &Frame,
    axis: Axis,
    (old, new): ((u32, u32), (u32, u32)),
    delta: i64,
    cancel: &AtomicBool,
) -> Result<(), Error> {
    let length = axis_length(axis, current);
    // 新露出的像素没有对应帧；选择重叠一侧，逐像素证明原固定区域实际随正文移动。
    let released = (new.0..old.0)
        .map(|line| (line, true))
        .chain((length - old.1..length - new.1).map(|line| (line, false)));
    for (line, front) in released {
        check(cancel)?;
        let line = i64::from(line);
        let (a, b) = if (delta > 0) == front {
            (line + delta, line)
        } else {
            (line, line - delta)
        };
        if !(0..i64::from(length)).contains(&a)
            || !(0..i64::from(length)).contains(&b)
            || !fixed::equal_lines(previous, current, axis, a as u32, b as u32)
        {
            return Err(Error::ChangedEdges);
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "long_capture_session_edges_tests.rs"]
mod tests;
