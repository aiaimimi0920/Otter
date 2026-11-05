//! 创建失败的候选准入凭证；原始错误与 HWND 关联状态分离，不猜测 HRESULT 或错误字符串。
use windows::core::Error;

#[derive(Debug)]
pub(super) struct Failure {
    error: Error,
    fallback_allowed: bool,
}
pub(super) type Result<T> = std::result::Result<T, Failure>;

impl Failure {
    pub(super) fn with_context(self, context: impl std::fmt::Display) -> Self {
        Self {
            error: Error::new(self.error.code(), format!("{context}; {}", self.error)),
            fallback_allowed: self.fallback_allowed,
        }
    }
    /// 已尝试关联，即使异步退休已经开始也不能直接准许下一候选接管。
    pub(super) fn attached(error: Error) -> Self {
        Self {
            error,
            fallback_allowed: false,
        }
    }
    pub(super) fn fallback_allowed(&self) -> bool {
        self.fallback_allowed
    }
    pub(super) fn into_error(self) -> Error {
        self.error
    }
}
impl From<Error> for Failure {
    fn from(error: Error) -> Self {
        Self {
            error,
            fallback_allowed: true,
        }
    }
}
