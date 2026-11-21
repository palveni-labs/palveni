//! Running external tools safely: argument vectors only, no stdin, output
//! captured, hard timeout, child killed when the future is dropped.

use std::ffi::OsString;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

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
