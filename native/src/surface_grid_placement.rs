//! 稀疏 row 自动放置：显式占位先于自动项，保留 DOM 绘制顺序和允许的显式重叠。
use super::{
    tracks::{MAX_CELLS, MAX_COLUMNS},
    Error,
};
use crate::surface_scene::{Node, MAX_NODES};
use serde_json::Value;
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Line {
    pub start: Option<usize>,
    pub span: usize,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Area {
    pub start: [usize; 2],
    pub span: [usize; 2],
}
fn integer(value: &str) -> Result<i32, Error> {
    if value.len() > 6
        || value.is_empty()
        || !value.bytes().all(|c| c.is_ascii_digit() || c == b'-')
    {
        return Err(Error::Syntax);
    }
    let v = value.parse::<i32>().map_err(|_| Error::Syntax)?;
    if v == 0 || v.unsigned_abs() as usize > MAX_NODES + 1 {
        return Err(Error::Limit);
    }
    Ok(v)
}
fn line(value: i32, explicit: usize) -> Result<usize, Error> {
    let index = if value > 0 {
        value - 1
    } else {
        explicit as i32 + 1 + value
    };
    if index < 0 {
        return Err(Error::Unsupported);
    }
    Ok(index as usize)
}
pub(super) fn parse(value: Option<&Value>, explicit: usize) -> Result<Line, Error> {
    let Some(value) = value else {
        return Ok(Line {
            start: None,
            span: 1,
        });
    };
    let text = value.as_str().ok_or(Error::Syntax)?;
    if text.len() > 64 || !text.is_ascii() {
        return Err(Error::Syntax);
    }
    let parts = text.split('/').map(str::trim).collect::<Vec<_>>();
    if parts.is_empty() || parts.len() > 2 {
        return Err(Error::Syntax);
    }
    let read = |text: &str| -> Result<(Option<usize>, Option<usize>), Error> {
        if text == "auto" {
            return Ok((None, None));
        }
        if let Some(v) = text.strip_prefix("span ") {
            let n = integer(v.trim())?;
            if n <= 0 {
                return Err(Error::Syntax);
            }
            return Ok((None, Some(n as usize)));
        }
        Ok((Some(line(integer(text)?, explicit)?), None))
    };
    let (a, sa) = read(parts[0])?;
    let (b, sb) = if parts.len() == 2 {
        read(parts[1])?
    } else {
        (None, None)
    };
    if sa.is_some() && sb.is_some() {
        return Err(Error::Unsupported);
    }
    let (start, span) = match (a, b, sa, sb) {
        (Some(a), Some(b), _, _) if a != b => (Some(a.min(b)), a.abs_diff(b)),
        (Some(a), _, _, span) => (Some(a), span.unwrap_or(1)),
        (_, Some(b), span, _) => {
            let n = span.unwrap_or(1);
            (Some(b.checked_sub(n).ok_or(Error::Unsupported)?), n)
        }
        _ => (None, sa.or(sb).unwrap_or(1)),
    };
    Ok(Line { start, span })
}
fn mask(column: usize, span: usize) -> Result<u64, Error> {
    if span == 0 || column.checked_add(span).is_none_or(|v| v > MAX_COLUMNS) {
        return Err(Error::Limit);
    }
    Ok(if span == 64 {
        u64::MAX
    } else {
        ((1u64 << span) - 1) << column
    })
}
fn fits(rows: &[u64], area: Area, work: &mut usize) -> Result<bool, Error> {
    let bits = mask(area.start[0], area.span[0])?;
    let end = area.start[1]
        .checked_add(area.span[1])
        .ok_or(Error::Limit)?;
    if end > MAX_NODES {
        return Err(Error::Limit);
    }
    // 按可能访问的行收费，而非只约束最终 cell 数；失败搜索也必须有界。
    *work += area.span[1];
    if *work > MAX_CELLS * 16 {
        return Err(Error::Limit);
    }
    Ok((area.start[1]..end).all(|row| rows.get(row).copied().unwrap_or(0) & bits == 0))
}
fn occupy(rows: &mut Vec<u64>, area: Area) -> Result<(), Error> {
    let bits = mask(area.start[0], area.span[0])?;
    let end = area.start[1] + area.span[1];
    if end > MAX_NODES {
        return Err(Error::Limit);
    }
    rows.resize(rows.len().max(end), 0);
    for row in &mut rows[area.start[1]..end] {
        *row |= bits;
    }
    Ok(())
}
pub(super) fn place(
    children: &[&Node],
    explicit: [usize; 2],
) -> Result<(Vec<Area>, [usize; 2]), Error> {
    if children.is_empty() {
        return Ok((Vec::new(), explicit));
    }
    let lines = children
        .iter()
        .map(|n| {
            Ok([
                parse(n.layout().get("gridColumn"), explicit[0])?,
                parse(n.layout().get("gridRow"), explicit[1])?,
            ])
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let mut columns = explicit[0].max(1);
    for item in &lines {
        columns = columns.max(item[0].start.unwrap_or(0) + item[0].span);
    }
    if columns > MAX_COLUMNS {
        return Err(Error::Limit);
    }
    let mut occupied = Vec::new();
    let mut work = 0;
    let mut output = vec![
        Area {
            start: [0; 2],
            span: [1; 2]
        };
        children.len()
    ];
    for (index, item) in lines.iter().enumerate() {
        if let (Some(x), Some(y)) = (item[0].start, item[1].start) {
            output[index] = Area {
                start: [x, y],
                span: [item[0].span, item[1].span],
            };
            occupy(&mut occupied, output[index])?;
        }
    }
    let mut row_cursors = vec![0; MAX_NODES];
    for (index, item) in lines.iter().enumerate() {
        if item[0].start.is_some() || item[1].start.is_none() {
            continue;
        }
        let mut area = Area {
            start: [0, item[1].start.unwrap()],
            span: [item[0].span, item[1].span],
        };
        let end = area.start[1] + area.span[1];
        if end > MAX_NODES {
            return Err(Error::Limit);
        }
        area.start[0] = row_cursors[area.start[1]..end]
            .iter()
            .copied()
            .max()
            .unwrap_or(0);
        while !fits(&occupied, area, &mut work)? {
            area.start[0] += 1;
        }
        columns = columns.max(area.start[0] + area.span[0]);
        occupy(&mut occupied, area)?;
        row_cursors[area.start[1]..end].fill(area.start[0] + area.span[0]);
        output[index] = area;
    }
    let mut cursor = [0; 2];
    for (index, item) in lines.iter().enumerate() {
        if item[1].start.is_some() {
            continue;
        }
        let mut area = Area {
            start: cursor,
            span: [item[0].span, item[1].span],
        };
        if let Some(column) = item[0].start {
            if column < cursor[0] {
                area.start[1] += 1;
            }
            area.start[0] = column;
            while !fits(&occupied, area, &mut work)? {
                area.start[1] += 1;
            }
        } else {
            loop {
                if area.start[0] + area.span[0] > columns {
                    area.start[0] = 0;
                    area.start[1] += 1;
                }
                if fits(&occupied, area, &mut work)? {
                    break;
                }
                area.start[0] += 1;
            }
        }
        occupy(&mut occupied, area)?;
        cursor = area.start;
        output[index] = area;
    }
    let rows = explicit[1].max(occupied.len()).max(1);
    if columns * rows > MAX_CELLS {
        return Err(Error::Limit);
    }
    Ok((output, [columns, rows]))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grid_failed_placement_probes_charge_row_work_before_cell_limit() {
        let mut work = MAX_CELLS * 16 - 1;
        assert_eq!(
            fits(
                &[],
                Area {
                    start: [0, 0],
                    span: [1, 2]
                },
                &mut work
            ),
            Err(Error::Limit)
        );
    }
}
