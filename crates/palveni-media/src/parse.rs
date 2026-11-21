//! Parsers for ffmpeg's human-readable listings.
//!
//! The formats are stable across ffmpeg 4–8 but not formally specified, so
//! the parsers are lenient: unknown lines are skipped rather than failing.

/// Extracts the version token from `ffmpeg -version` / `ffprobe -version`,
/// e.g. `7.1.1-Jellyfin`, `6.1.1-3ubuntu5`, `N-118000-g1234567890`.
pub fn parse_version(output: &str) -> Option<String> {
    let first = output.lines().find(|line| !line.trim().is_empty())?;
    let mut words = first.split_whitespace();
    let _tool = words.next()?;
    if words.next()? != "version" {
        return None;
    }
    words.next().map(str::to_owned)
}

/// Methods listed by `ffmpeg -hide_banner -hwaccels`.
pub fn parse_hwaccels(output: &str) -> Vec<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.ends_with(':'))
        .filter(|line| !line.contains(char::is_whitespace))
        .map(str::to_owned)
        .collect()
}

/// One line of `ffmpeg -encoders`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListedEncoder {
    pub name: String,
    /// `V`ideo, `A`udio or `S`ubtitle.
    pub media_type: char,
    /// Codec from the trailing `(codec xyz)`, or the encoder name when the
    /// encoder is the codec's native implementation (e.g. `aac`).
    pub codec: String,
    pub description: String,
}

/// Parses `ffmpeg -hide_banner -encoders`.
pub fn parse_encoders(output: &str) -> Vec<ListedEncoder> {
    let lines: Vec<&str> = output.lines().collect();
    // The legend ends with a `------` line; older builds may lack it.
    let start = lines
        .iter()
        .position(|line| line.trim() == "------")
        .map_or(0, |i| i + 1);
    lines[start..]
        .iter()
        .filter_map(|line| parse_encoder_line(line))
        .collect()
}

fn parse_encoder_line(line: &str) -> Option<ListedEncoder> {
    let rest = line.trim_start();
    let (flags, rest) = rest.split_once(char::is_whitespace)?;
    if flags.len() != 6 {
        return None;
    }
    let media_type = flags.chars().next()?;
    if !matches!(media_type, 'V' | 'A' | 'S') {
        return None;
    }
    let rest = rest.trim_start();
    let (name, description) = match rest.split_once(char::is_whitespace) {
        Some((name, description)) => (name, description.trim()),
        None => (rest, ""),
    };
    // Legend lines look like `V..... = Video`.
    if name.is_empty() || name == "=" {
        return None;
    }
    let codec = codec_from_description(description).unwrap_or_else(|| name.to_owned());
    Some(ListedEncoder {
        name: name.to_owned(),
        media_type,
        codec,
        description: description.to_owned(),
    })
}

fn codec_from_description(description: &str) -> Option<String> {
    let start = description.rfind("(codec ")? + "(codec ".len();
    let end = description[start..].find(')')? + start;
    let codec = description[start..end].trim();
    (!codec.is_empty()).then(|| codec.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VERSION_JELLYFIN: &str = include_str!("../fixtures/version_jellyfin.txt");
    const VERSION_UBUNTU_FFPROBE: &str = include_str!("../fixtures/version_ubuntu_ffprobe.txt");
    const HWACCELS: &str = include_str!("../fixtures/hwaccels.txt");
    const ENCODERS: &str = include_str!("../fixtures/encoders.txt");

    #[test]
    fn versions() {
        assert_eq!(
            parse_version(VERSION_JELLYFIN).as_deref(),
            Some("7.1.1-Jellyfin")
        );
        assert_eq!(
            parse_version(VERSION_UBUNTU_FFPROBE).as_deref(),
            Some("6.1.1-3ubuntu5")
        );
        assert_eq!(
            parse_version("ffmpeg version N-118000-g1234567890 Copyright (c) 2000-2025").as_deref(),
            Some("N-118000-g1234567890")
        );
        assert_eq!(parse_version("bash: ffmpeg: command not found"), None);
        assert_eq!(parse_version(""), None);
    }

    #[test]
    fn hwaccels() {
        assert_eq!(
            parse_hwaccels(HWACCELS),
            vec!["vdpau", "cuda", "vaapi", "qsv", "drm", "opencl", "vulkan"]
        );
        assert!(parse_hwaccels("Hardware acceleration methods:\n\n").is_empty());
    }

    #[test]
    fn encoders() {
        let encoders = parse_encoders(ENCODERS);
        // Legend lines are not encoders.
        assert!(encoders.iter().all(|e| e.name != "=" && !e.name.is_empty()));

        let find = |name: &str| encoders.iter().find(|e| e.name == name).unwrap();
        let vaapi = find("h264_vaapi");
        assert_eq!(vaapi.media_type, 'V');
        assert_eq!(vaapi.codec, "h264");
        assert_eq!(find("hevc_qsv").codec, "hevc");
        assert_eq!(find("av1_nvenc").codec, "av1");
        assert_eq!(find("libx264").codec, "h264");
        // The description itself contains parentheses before `(codec …)`.
        assert_eq!(find("h264_qsv").codec, "h264");
        // Native encoders have no `(codec …)` suffix.
        assert_eq!(find("mjpeg").codec, "mjpeg");
        assert_eq!(find("aac").media_type, 'A');
        assert_eq!(find("aac").codec, "aac");
        assert_eq!(find("webvtt").media_type, 'S');
        assert_eq!(encoders.len(), 34);
    }

    #[test]
    fn encoders_without_legend() {
        let encoders = parse_encoders(" V....D libx264  libx264 H.264 (codec h264)\n");
        assert_eq!(encoders.len(), 1);
        assert_eq!(encoders[0].codec, "h264");
    }
}
