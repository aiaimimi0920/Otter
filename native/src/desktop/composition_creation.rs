//! Admission evidence for candidates whose creation failed; separate original errors from HWND association state, without guessing from HRESULTs/error strings.
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
    /// Once association has been attempted, even starting asynchronous retirement cannot directly authorize the next candidate to take over.
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
