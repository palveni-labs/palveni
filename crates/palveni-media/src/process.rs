//! External tools use argument vectors, captured output and hard deadlines.
//! Media probes read an already-open file through the child's standard input.

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use std::{ffi::OsString, io};

use tokio::{io::AsyncReadExt, process::Command};

pub(crate) const FILE_INPUT: &str = if cfg!(windows) { "handle" } else { "fd" };

pub(crate) fn configure_file_input(cmd: &mut Command, demuxers: &str) {
    cmd.args([
        "-v",
        "error",
        "-threads",
        "1",
        "-max_alloc",
        "67108864",
        "-max_streams",
        "256",
        "-probesize",
        "2097152",
        "-analyzeduration",
        "2000000",
        "-protocol_whitelist",
        FILE_INPUT,
        "-format_whitelist",
        demuxers,
    ]);
}

pub(crate) async fn bounded_output(
    mut cmd: Command,
    limit: usize,
    deadline: Duration,
) -> io::Result<Vec<u8>> {
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("missing tool output"))?;
    let run = async {
        let mut output = Vec::new();
        stdout
            .take(limit as u64 + 1)
            .read_to_end(&mut output)
            .await?;
        if output.len() > limit || !child.wait().await.is_ok_and(|status| status.success()) {
            return Err(io::Error::other("invalid or oversized tool output"));
        }
        Ok(output)
    };
    let result = tokio::time::timeout(deadline, run)
        .await
        .unwrap_or_else(|_| Err(io::Error::new(io::ErrorKind::TimedOut, "tool timed out")));
    if result.is_err() {
        let _ = child.kill().await;
    }
    result
}

#[derive(Clone, Debug)]
pub struct CommandOutput {
    pub success: bool,
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error("cannot start {program}: {source}")]
    Spawn {
        program: String,
        #[source]
        source: std::io::Error,
    },
    #[error("timed out after {} s", .0.as_secs())]
    TimedOut(Duration),
}

/// Converts string-like arguments into an argument vector.
pub fn os_args<I, S>(args: I) -> Vec<OsString>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    args.into_iter().map(Into::into).collect()
}

pub async fn run(
    program: &Path,
    args: &[OsString],
    timeout: Duration,
) -> Result<CommandOutput, RunError> {
    let mut command = tokio::process::Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    match tokio::time::timeout(timeout, command.output()).await {
        Ok(Ok(output)) => Ok(CommandOutput {
            success: output.status.success(),
            code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }),
        Ok(Err(source)) => Err(RunError::Spawn {
            program: program.display().to_string(),
            source,
        }),
        Err(_elapsed) => Err(RunError::TimedOut(timeout)),
    }
}

/// Last meaningful stderr line, shortened; used as a one-line error summary.
pub fn summarize_stderr(stderr: &str) -> Option<String> {
    const MAX_CHARS: usize = 300;
    let line = stderr
        .lines()
        .map(str::trim)
        .rev()
        .find(|line| !line.is_empty())?;
    let mut summary: String = line.chars().take(MAX_CHARS).collect();
    if line.chars().count() > MAX_CHARS {
        summary.push('…');
    }
    Some(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[tokio::test]
    async fn subprocess_output_and_time_are_bounded() {
        let echo = || {
            let mut cmd = Command::new("/bin/echo");
            cmd.arg("subtitle");
            cmd
        };
        assert_eq!(
            bounded_output(echo(), 9, Duration::from_secs(1))
                .await
                .unwrap(),
            b"subtitle\n"
        );
        assert_eq!(
            bounded_output(echo(), 8, Duration::from_secs(1))
                .await
                .unwrap_err()
                .kind(),
            io::ErrorKind::Other
        );
        let mut sleep = Command::new("/bin/sleep");
        sleep.arg("5");
        assert_eq!(
            bounded_output(sleep, 8, Duration::from_millis(30))
                .await
                .unwrap_err()
                .kind(),
            io::ErrorKind::TimedOut
        );
    }

    #[test]
    fn summary_takes_last_non_empty_line() {
        let stderr = "[h264_vaapi @ 0x55] No usable encoding profile found.\nError initializing output stream 0:0 -- Error while opening encoder\n\n";
        assert_eq!(
            summarize_stderr(stderr).as_deref(),
            Some("Error initializing output stream 0:0 -- Error while opening encoder")
        );
        assert_eq!(summarize_stderr("\n  \n"), None);
        let long = "x".repeat(400);
        assert_eq!(summarize_stderr(&long).unwrap().chars().count(), 301);
    }
}
