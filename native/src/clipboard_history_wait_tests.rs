//! 延迟确认取消的模拟操作；不访问系统历史，验证读取许可不会提前释放。
use super::super::*;
use std::cell::Cell;
struct Delayed<'a> {
    polls: &'a Cell<usize>,
    cancellations: &'a Cell<usize>,
}
// 与 WinRT 生成接口同名，便于直接验证同一等待宏。
#[allow(non_snake_case)]
impl Delayed<'_> {
    fn Cancel(&self) -> Result<()> {
        self.cancellations.set(self.cancellations.get() + 1);
        Ok(())
    }
    fn Status(&self) -> Result<(i32,)> {
        assert_eq!(self.cancellations.get(), 1);
        assert!(ReadPermit::acquire().is_err());
        let count = self.polls.get() + 1;
        self.polls.set(count);
        Ok((if count < 3 { 0 } else { 2 },))
    }
    fn GetResults(&self) -> Result<()> {
        panic!("Cancelled operation must not publish results")
    }
}
#[test]
fn cancel_waits_for_terminal_status_and_keeps_exclusive_permit() {
    let polls = Cell::new(0);
    let cancellations = Cell::new(0);
    let permit = ReadPermit::acquire().unwrap();
    let run = || -> Result<()> {
        super::operation!(
            Delayed {
                polls: &polls,
                cancellations: &cancellations
            },
            &AtomicBool::new(true),
            Instant::now() + Duration::from_secs(5)
        )
    };
    assert_eq!(run().unwrap_err().code(), E_ABORT);
    assert_eq!(polls.get(), 3);
    assert_eq!(cancellations.get(), 1);
    drop(permit);
    let _released = ReadPermit::acquire().unwrap();
}
