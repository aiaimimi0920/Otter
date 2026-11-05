//! 有界子进程回归：运行期间同时排空两条管道，诊断保留有上限，超时仅回收自有 child。
use std::{
    io::{self, Read},
    process::{Child, Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};
const OUTPUT_LIMIT: usize = 256 * 1024;

struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        // 异常路径也不得留下本测试启动的进程；不按名称搜索或清理其他实例。
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn drain(mut pipe: impl Read) -> io::Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut buffer = [0; 8192];
    loop {
        let count = pipe.read(&mut buffer)?;
        if count == 0 {
            return Ok(output);
        }
        let keep = count.min(OUTPUT_LIMIT - output.len());
        output.extend_from_slice(&buffer[..keep]);
        // 达到诊断上限后继续读取并丢弃，不能再次让写端卡在已满的管道上。
    }
}

pub fn run(command: &mut Command, timeout: Duration) -> io::Result<Output> {
    // 隐藏测试不能继承开发者的真实 profile；只有测试显式传入的隔离覆盖才保留。
    if !command.get_envs().any(|(key, _)| {
        key.to_string_lossy()
            .eq_ignore_ascii_case("OTTER_NATIVE_PROFILE_DIR")
    }) {
        command.env_remove("OTTER_NATIVE_PROFILE_DIR");
    }
    let mut child = OwnedChild(
        command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?,
    );
    let stdout = child.0.stdout.take().unwrap();
    let stderr = child.0.stderr.take().unwrap();
    let stdout = thread::spawn(move || drain(stdout));
    let stderr = thread::spawn(move || drain(stderr));
    let started = Instant::now();
    let status = loop {
        match child.0.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Err(error) => break Err(error),
            Ok(None) if started.elapsed() >= timeout => {
                break Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "Native startup child timed out",
                ));
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
        }
    };
    if status.is_err() {
        let _ = child.0.kill();
        let _ = child.0.wait();
    }
    // 本 runner 仅用于不派生继承管道子孙的测试命令；等待本次两个 reader 回收。
    let stdout = stdout
        .join()
        .map_err(|_| io::Error::other("stdout reader panicked"))??;
    let stderr = stderr
        .join()
        .map_err(|_| io::Error::other("stderr reader panicked"))??;
    let status =
        status.map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
        "{error}; elapsed={:?}; stdout (first {} bytes):\n{}\nstderr (first {} bytes):\n{}",
        started.elapsed(), stdout.len(), String::from_utf8_lossy(&stdout),
        stderr.len(), String::from_utf8_lossy(&stderr),
    ),
            )
        })?;
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}
