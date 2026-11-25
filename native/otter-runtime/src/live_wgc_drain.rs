//! Complete submitted GPU snapshot copies before terminating Live WGC so source destruction does not make Close wait for unfinished source-texture references.
use std::time::{Duration, Instant};
use windows::{
    core::{Error, Result, HRESULT},
    Win32::{Foundation::E_UNEXPECTED, Graphics::Direct3D11::*},
};

const TIMEOUT: HRESULT = HRESULT(0x800705b4u32 as i32);

pub(super) fn finish(context: &ID3D11DeviceContext) -> Result<()> {
    unsafe {
        let mut query = None;
        context.GetDevice()?.CreateQuery(
            &D3D11_QUERY_DESC {
                Query: D3D11_QUERY_EVENT,
                MiscFlags: 0,
            },
            Some(&mut query),
        )?;
        let query = query.ok_or_else(|| Error::from(E_UNEXPECTED))?;
        // The Frame guard has returned, but CopyResource/CopySubresourceRegion are asynchronous.
        // Flush only submits commands; observe the completion point on the same immediate context.
        // Do not ClearState: other consumers of the shared device may still use this context.
        context.End(&query);
        context.Flush();
        wait_until(Instant::now() + Duration::from_secs(2), || {
            let mut complete = 0u32;
            context.GetData(
                &query,
                Some((&mut complete as *mut u32).cast()),
                std::mem::size_of::<u32>() as u32,
                D3D11_ASYNC_GETDATA_DONOTFLUSH.0 as u32,
            )?;
            Ok(complete != 0)
        })
    }
}

fn wait_until(deadline: Instant, mut ready: impl FnMut() -> Result<bool>) -> Result<()> {
    while Instant::now() < deadline {
        if ready()? {
            return Ok(());
        }
        std::thread::sleep(
            Duration::from_millis(1).min(deadline.saturating_duration_since(Instant::now())),
        );
    }
    Err(Error::from(TIMEOUT))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completion_requires_positive_query_result() {
        let mut polls = 0;
        wait_until(Instant::now() + Duration::from_secs(1), || {
            polls += 1;
            Ok(polls == 2)
        })
        .unwrap();
        assert_eq!(polls, 2);
    }
    #[test]
    fn expired_budget_does_not_query_or_report_completion() {
        let error = wait_until(Instant::now(), || panic!("expired query")).unwrap_err();
        assert_eq!(error.code(), TIMEOUT);
    }
    #[test]
    fn query_error_is_preserved_without_retry() {
        let mut polls = 0;
        let error = wait_until(Instant::now() + Duration::from_secs(1), || {
            polls += 1;
            Err(Error::from(E_UNEXPECTED))
        })
        .unwrap_err();
        assert_eq!(error.code(), E_UNEXPECTED);
        assert_eq!(polls, 1);
    }
}
