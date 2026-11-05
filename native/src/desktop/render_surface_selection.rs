//! 只有完整 D3D12 呈现链创建成功才发布；失败后尝试原兼容链一次，不循环重试。
use windows::core::{Error, Result};

/// 普通窗口和钉图的 CPU 候选都只接受“尚未关联”的失败，不能绕过 DComp 退休。
pub(super) fn compatibility<T>(
    gpu: super::super::composition_creation::Result<T>,
    pinned: bool,
    software: impl FnOnce() -> Result<T>,
) -> super::super::composition_creation::Result<T> {
    match gpu {
        Ok(value) => Ok(value),
        Err(failure) if !failure.fallback_allowed() => Err(failure),
        Err(failure) => {
            let first = failure.into_error();
            let kind = if pinned { "pin" } else { "workspace" };
            software()
                .inspect(|_| {
                    eprintln!(
                        "native-render-backend: selected=software-{kind} stage=create-presentation"
                    )
                })
                .map_err(|last| {
                    Error::new(
                        last.code(),
                        format!("Compatibility {kind} unavailable: gpu={first}; software={last}"),
                    )
                    .into()
                })
        }
    }
}

pub(super) fn select<T>(
    primary: impl FnOnce() -> super::super::composition_creation::Result<T>,
    fallback: impl FnOnce() -> super::super::composition_creation::Result<T>,
    legacy_label: impl FnOnce(&T) -> &'static str,
) -> super::super::composition_creation::Result<T> {
    let first = match primary() {
        Ok(value) => {
            eprintln!("native-render-backend: selected=d3d12 stage=create-presentation");
            return Ok(value);
        }
        Err(failure) => {
            if !failure.fallback_allowed() {
                eprintln!("native-render-backend: rejected=d3d12 fallback=denied reason=unconfirmed-association");
                return Err(failure);
            }
            failure.into_error()
        }
    };
    eprintln!(
        "native-render-backend: rejected=d3d12 stage=create-presentation code={:?}",
        first.code()
    );
    match fallback() {
        Ok(value) => {
            let legacy_label = legacy_label(&value);
            eprintln!("native-render-backend: selected={legacy_label} stage=create-presentation");
            Ok(value)
        }
        Err(last) => Err(last.with_context(format!(
            "Native Canvas presentation unavailable: d3d12={first}; compatibility"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use windows::Win32::Foundation::{E_FAIL, E_INVALIDARG};

    #[test]
    fn compatibility_software_is_only_created_after_unassociated_gpu_failure() {
        for pinned in [false, true] {
            assert_eq!(
                compatibility(Ok(7), pinned, || panic!(
                    "GPU success must not allocate CPU"
                ))
                .unwrap(),
                7
            );
            assert_eq!(
                compatibility(Err(Error::new(E_FAIL, "no GPU").into()), pinned, || Ok(9)).unwrap(),
                9
            );
            let denied = super::super::super::composition_creation::Failure::attached(Error::new(
                E_FAIL,
                "association",
            ));
            let failure = compatibility::<()>(Err(denied), pinned, || {
                panic!("unconfirmed association must block CPU")
            })
            .unwrap_err();
            assert!(!failure.fallback_allowed());
            assert_eq!(failure.into_error().message(), "association");
            let failure = compatibility::<()>(
                Err(Error::new(E_FAIL, "gpu fixture").into()),
                pinned,
                || Err(Error::new(E_INVALIDARG, "cpu fixture")),
            )
            .unwrap_err()
            .into_error();
            assert_eq!(failure.code(), E_INVALIDARG);
            assert!(failure.message().contains("gpu fixture"));
            assert!(failure.message().contains("cpu fixture"));
        }
    }

    #[test]
    fn compatibility_association_denial_survives_outer_selection_context() {
        let failure = select::<()>(
            || Err(Error::new(E_FAIL, "primary preparation").into()),
            || {
                Err(
                    super::super::super::composition_creation::Failure::attached(Error::new(
                        E_INVALIDARG,
                        "compatibility association",
                    )),
                )
            },
            |_| "fixture",
        )
        .unwrap_err();
        assert!(!failure.fallback_allowed());
        let error = failure.into_error();
        assert_eq!(error.code(), E_INVALIDARG);
        assert!(error.message().contains("primary preparation"));
        assert!(error.message().contains("compatibility association"));
    }

    #[test]
    fn unconfirmed_primary_association_never_enters_compatibility() {
        let error = select::<()>(
            || {
                Err(
                    super::super::super::composition_creation::Failure::attached(Error::new(
                        E_INVALIDARG,
                        "partial primary association",
                    )),
                )
            },
            || panic!("unconfirmed graph must not hand HWND to fallback"),
            |_| "fixture",
        )
        .unwrap_err()
        .into_error();
        assert_eq!(error.code(), E_INVALIDARG);
        assert!(error.message().contains("partial primary association"));
    }

    #[test]
    fn workspace_cpu_is_lazy_once_and_preserves_both_failures() {
        assert_eq!(
            compatibility(Ok(1), false, || panic!("CPU must remain cold")).unwrap(),
            1
        );
        let calls = Cell::new(0);
        assert_eq!(
            compatibility(Err(Error::new(E_FAIL, "hardware").into()), false, || {
                calls.set(calls.get() + 1);
                Ok(2)
            })
            .unwrap(),
            2
        );
        assert_eq!(calls.get(), 1);
        let error = compatibility::<()>(Err(Error::new(E_FAIL, "hardware").into()), false, || {
            Err(Error::new(E_INVALIDARG, "software"))
        })
        .unwrap_err()
        .into_error();
        assert_eq!(error.code(), E_INVALIDARG);
        assert!(error.to_string().contains("hardware") && error.to_string().contains("software"));
    }

    #[test]
    fn primary_success_does_not_create_compatibility_resources() {
        assert_eq!(
            select(
                || Ok(12),
                || panic!("fallback must stay cold"),
                |_| "fixture"
            )
            .unwrap(),
            12
        );
    }
    #[test]
    fn failed_primary_falls_back_once_and_final_error_keeps_both_causes() {
        let calls = Cell::new(0);
        let value = select(
            || Err(Error::new(E_FAIL, "primary fixture").into()),
            || {
                calls.set(calls.get() + 1);
                Ok(11)
            },
            |_| "fixture",
        )
        .unwrap();
        assert_eq!(value, 11);
        assert_eq!(calls.get(), 1);
        let error = select::<()>(
            || Err(Error::new(E_FAIL, "primary fixture").into()),
            || Err(Error::new(E_INVALIDARG, "fallback fixture").into()),
            |_| "fixture",
        )
        .unwrap_err();
        let error = error.into_error();
        assert_eq!(error.code(), E_INVALIDARG);
        let message = error.to_string();
        assert!(message.contains("primary fixture") && message.contains("fallback fixture"));
    }
}
