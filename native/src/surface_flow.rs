//! 行/列容器主轴分配；纯数值计算，不创建窗口或分配像素，保留最小尺寸造成的溢出。
#[derive(Clone, Copy, Debug)]
pub struct Item {
    pub basis: f32,
    pub minimum: f32,
    pub maximum: f32,
    pub grow: f32,
    pub shrink: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Justify {
    Start,
    Center,
    End,
    Between,
    Around,
    Evenly,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub offset: f32,
    pub size: f32,
}
#[derive(Debug)]
pub struct Flow {
    pub items: Vec<Placement>,
    pub overflow: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    Input,
}
fn dimension(value: f32) -> bool {
    value.is_finite() && (0. ..=32768.).contains(&value)
}
pub fn arrange(items: &[Item], available: f32, gap: f32, justify: Justify) -> Result<Flow, Error> {
    if items.len() > crate::surface_scene::MAX_NODES {
        return Err(Error::Limit);
    }
    if !dimension(available)
        || !dimension(gap)
        || items.iter().any(|i| {
            !dimension(i.basis)
                || !dimension(i.minimum)
                || !dimension(i.maximum)
                || i.minimum > i.maximum
                || !i.grow.is_finite()
                || !(0. ..=100.).contains(&i.grow)
                || !i.shrink.is_finite()
                || !(0. ..=100.).contains(&i.shrink)
        })
    {
        return Err(Error::Input);
    }
    if items.is_empty() {
        return Ok(Flow {
            items: vec![],
            overflow: 0.,
        });
    }
    // f64 中间值避免大树累加使边界判断因 f32 舍入来回振荡。
    let gap_total = f64::from(gap) * (items.len() - 1) as f64;
    let target = f64::from(available) - gap_total;
    let mut sizes: Vec<f64> = items
        .iter()
        .map(|i| f64::from(i.basis.clamp(i.minimum, i.maximum)))
        .collect();
    let growing = target >= sizes.iter().sum::<f64>();
    let initial_free = (target - sizes.iter().sum::<f64>()).abs();
    let bases = sizes.clone();
    let mut active = vec![true; items.len()];
    let mut weights = vec![0.; items.len()];
    // 每轮至少冻结一个边界项，或者消耗全部可分配空间，循环有节点数上界。
    for _ in 0..=items.len() {
        let used: f64 = (0..items.len())
            .map(|i| if active[i] { bases[i] } else { sizes[i] })
            .sum();
        let remaining = (target - used).abs();
        if remaining <= 0.000001 {
            break;
        }
        for (index, weight) in weights.iter_mut().enumerate() {
            *weight = if !active[index] {
                0.
            } else if growing {
                f64::from(items[index].grow)
            } else {
                f64::from(items[index].shrink) * f64::from(items[index].basis)
            };
        }
        let total: f64 = weights.iter().sum();
        if total == 0. {
            break;
        }
        // grow 总和小于 1 时不强行填满；剩余空间留给 justify。
        let budget = if growing {
            remaining.min(initial_free * total.min(1.))
        } else {
            remaining
        };
        let mut frozen = false;
        for index in 0..items.len() {
            let fraction = weights[index] / total;
            if fraction == 0. {
                continue;
            }
            let proposed = bases[index]
                + if growing {
                    budget * fraction
                } else {
                    -budget * fraction
                };
            let bounded = proposed.clamp(
                f64::from(items[index].minimum),
                f64::from(items[index].maximum),
            );
            sizes[index] = bounded;
            if bounded != proposed {
                active[index] = false;
                frozen = true;
            }
        }
        if !frozen {
            break;
        }
    }
    let occupied = sizes.iter().sum::<f64>() + gap_total;
    let free = (f64::from(available) - occupied).max(0.);
    let n = items.len() as f64;
    let (mut offset, extra) = match justify {
        Justify::Start => (0., 0.),
        Justify::Center => (free / 2., 0.),
        Justify::End => (free, 0.),
        Justify::Between if items.len() > 1 => (0., free / (n - 1.)),
        Justify::Between => (0., 0.),
        Justify::Around => (free / (2. * n), free / n),
        Justify::Evenly => (free / (n + 1.), free / (n + 1.)),
    };
    let placements = sizes
        .into_iter()
        .map(|size| {
            let item = Placement {
                offset: offset as f32,
                size: size as f32,
            };
            offset += size + f64::from(gap) + extra;
            item
        })
        .collect();
    Ok(Flow {
        items: placements,
        overflow: (occupied - f64::from(available)).max(0.) as f32,
    })
}
#[cfg(test)]
#[path = "surface_flow_tests.rs"]
mod tests;
