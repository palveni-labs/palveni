//! EBU R128 measurements from the original stream. The null muxer discards the filtered audio.
use palveni_core::{PerfTier, music::LoudnessValues};
use std::{fs::File, future::Future, io, path::Path, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
};

fn parse_summary(stderr: &[u8]) -> io::Result<LoudnessValues> {
    let invalid = || io::Error::other("FFmpeg did not produce a valid loudness summary");
    let log = String::from_utf8_lossy(stderr);
    let (_, suffix) = log.rsplit_once("[Parsed_loudnorm_").ok_or_else(invalid)?;
    let start = suffix.find('{').ok_or_else(invalid)?;
    let end = suffix[start..].find('}').ok_or_else(invalid)? + start;
    let json: serde_json::Value =
        serde_json::from_str(&suffix[start..=end]).map_err(|_| invalid())?;
    let number = |key| -> io::Result<Option<f64>> {
        let text = json
            .get(key)
            .and_then(serde_json::Value::as_str)
            .ok_or_else(invalid)?;
        if text == "-inf" {
            return Ok(None);
        }
        let value: f64 = text.parse().map_err(|_| invalid())?;
        if !value.is_finite() {
            return Err(invalid());
        }
        Ok(Some(value))
    };
    let integrated_lufs = number("input_i")?;
    let range = number("input_lra")?;
    Ok(LoudnessValues {
        integrated_lufs,
        range_lu: integrated_lufs.and(range),
        true_peak_dbtp: number("input_tp")?,
    })
}

async fn log_tail(mut stream: impl AsyncRead + Unpin) -> io::Result<Vec<u8>> {
    let mut tail = Vec::with_capacity(8192);
    let mut block = [0; 4096];
    loop {
        let read = stream.read(&mut block).await?;
        if read == 0 {
            return Ok(tail);
        }
        if tail.len() + read > 8192 {
            tail.drain(..tail.len() + read - 8192);
        }
        tail.extend_from_slice(&block[..read]);
    }
}

pub async fn measure(
    program: &Path,
    file: File,
    segment: Option<std::ops::Range<u64>>,
    tier: PerfTier,
    cancel: impl Future<Output = ()>,
) -> io::Result<LoudnessValues> {
    let seconds = match tier {
        PerfTier::Tiny => 60,
        PerfTier::Lite => 120,
        PerfTier::Standard => 300,
        PerfTier::Performance => 600,
    };
    let input = if cfg!(windows) { "handle" } else { "fd" };
    let mut command = Command::new(program);
    command.args([
        "-hide_banner",
        "-nostats",
        "-nostdin",
        "-xerror",
        "-max_alloc",
        "67108864",
        "-threads",
        "1",
        "-protocol_whitelist",
        input,
    ]);
    let meter = "loudnorm=I=-23:TP=-1:print_format=json";
    let filter = if let Some(frames) = segment {
        if frames.is_empty() {
            return Err(io::Error::other("Empty CUE track"));
        }
        let seconds =
            |frames: u64| format!("{}.{:06}", frames / 75, (frames % 75 * 1_000_000 + 37) / 75);
        command.args(["-ss", &seconds(frames.start)]);
        format!(
            "atrim=duration={},{meter}",
            seconds(frames.end - frames.start)
        )
    } else {
        meter.to_owned()
    };
    let mut child = command
        .args([
            "-i",
            &format!("{input}:"),
            "-map",
            "0:a:0",
            "-map_metadata",
            "-1",
            "-filter_threads",
            "1",
            "-af",
            &filter,
            "-threads",
            "1",
            "-f",
            "null",
            "-",
        ])
        .stdin(Stdio::from(file))
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;
    let pipe = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("FFmpeg diagnostic pipe unavailable"))?;
    let result = tokio::select! {
        biased;
        () = cancel => Err(io::Error::new(io::ErrorKind::Interrupted, "Loudness analysis cancelled")),
        () = tokio::time::sleep(Duration::from_secs(seconds)) => Err(io::Error::new(io::ErrorKind::TimedOut,"Loudness analysis timed out")),
        run = async { tokio::try_join!(child.wait(), log_tail(pipe)) } => run,
    };
    let (status, log) = match result {
        Ok(result) => result,
        Err(error) => {
            let _ = child.kill().await;
            return Err(error);
        }
    };
    if !status.success() {
        return Err(io::Error::other(
            "FFmpeg could not decode the complete audio stream",
        ));
    }
    parse_summary(&log)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_input_values_and_below_gate_results_only() {
        let data = br#"[Parsed_loudnorm_0 @ 0x0]
{"input_i":"-22.84","input_tp":"-19.01","input_lra":"1.4","output_i":"-23.0"}"#;
        assert_eq!(parse_summary(data).unwrap().integrated_lufs, Some(-22.84));
        let short = br#"[Parsed_loudnorm_0 @ 0x0] {"input_i":"-inf","input_tp":"-19.01","input_lra":"0.00"}"#;
        assert_eq!(
            parse_summary(short).unwrap(),
            LoudnessValues {
                integrated_lufs: None,
                range_lu: None,
                true_peak_dbtp: Some(-19.01)
            }
        );
        assert!(parse_summary(b"no summary").is_err());
        assert!(parse_summary(br#"[Parsed_loudnorm_0] {"input_i":"NaN"}"#).is_err());
    }
}
