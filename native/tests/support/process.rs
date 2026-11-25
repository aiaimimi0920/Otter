//! Bounded child-process regressions: drain both pipes concurrently, cap retained diagnostics and reclaim only owned children on timeout.
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
        // Exceptional paths must not leave processes started by this test behind; do not search by name or clean other instances.
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
        // Keep reading and discarding after the diagnostic limit is reached, so a full pipe cannot block the writer again.
    }
}

pub fn run(command: &mut Command, timeout: Duration) -> io::Result<Output> {
    // Hidden tests must not inherit the developer's real profile; retain only isolated overrides explicitly supplied by the test.
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
    // This runner is only for test commands that do not spawn descendants inheriting the pipes; wait for both readers from this run to finish.
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
