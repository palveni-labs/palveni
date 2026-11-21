"""Exercise the audio pipelines in palveni-media using the packaged command-line tools."""
import json
import math
import struct
import subprocess
import wave


def run(*args, source=None, check=True):
    result = subprocess.run(args, stdin=source, capture_output=True, timeout=30)
    if check and result.returncode:
        raise RuntimeError(f"{args}: {result.stderr.decode(errors='replace')}")
    return result


FORMATS = {
    "aac": ("aac", "mp4", ["-movflags", "+empty_moov+default_base_moof+frag_keyframe", "-frag_duration", "250000"]),
    "mp3": ("libmp3lame", "mp3", ["-write_xing", "0"]),
    "opus": ("libopus", "ogg", ["-page_duration", "200000"]),
}


def encode(ffmpeg, recording, fmt, window=None):
    # CUE windows use the 44.1 kHz sample grid, including for other source rates.
    pcm = fmt == "pcm"
    options = ["-nostdin", "-hide_banner", "-v", "error", "-threads", "1", "-filter_threads", "1",
               "-protocol_whitelist", "fd", "-readrate", "4" if pcm else "1.25"]
    anchor = max(0, window[0] // 44100 - 1) if window else 0
    seek_ms = anchor * 1000 if window else 4250
    if seek_ms:
        options += ["-ss", f"{seek_ms}ms"]
    if pcm or fmt == "aac":
        options += ["-readrate_initial_burst", "24" if pcm else "8"]
    options += ["-i", "fd:", "-map", "0:a:0", "-vn", "-sn", "-dn", "-map_metadata", "-1",
                "-map_chapters", "-1", "-threads", "1", "-ac", "2"]
    if window:
        start, end = (sample - anchor * 44100 for sample in window)
        options += ["-af", f"aresample=44100,apad,atrim=start_sample={start}:end_sample={end},asetpts=PTS-STARTPTS"]
    if pcm:
        options += ["-ar", "44100", "-c:a", "pcm_s16le", "-f", "s16le"]
    else:
        encoder, muxer, extra = FORMATS[fmt]
        options += ["-ar", "48000", "-c:a", encoder, "-f", muxer, *extra, "-b:a", "128k"]
    with recording.open("rb") as source:
        return run(ffmpeg, *options, "-flush_packets", "1", "pipe:1", source=source, check=pcm)


def loudness(ffmpeg, recording, segment=None):
    options = ["-hide_banner", "-nostats", "-nostdin", "-xerror", "-max_alloc", "67108864",
               "-threads", "1", "-protocol_whitelist", "fd"]
    filters = "loudnorm=I=-23:TP=-1:print_format=json"
    if segment:
        start, end = segment
        options += ["-ss", f"{start / 75:.6f}"]
        filters = f"atrim=duration={(end - start) / 75:.6f}," + filters
    with recording.open("rb") as source:
        log = run(ffmpeg, *options, "-i", "fd:", "-map", "0:a:0", "-map_metadata", "-1",
                  "-filter_threads", "1", "-af", filters, "-threads", "1", "-f", "null", "-",
                  source=source).stderr.decode()
    marker = log.rindex("[Parsed_loudnorm_")
    data, _ = json.JSONDecoder().raw_decode(log[log.index("{", marker):])
    values = [float(data[key]) for key in ("input_i", "input_tp", "input_lra")]
    assert all(math.isfinite(value) for value in values), values
    return values


def verify_audio(ffmpeg, ffprobe, temp):
    wav = temp / "levels.wav"
    flac = temp / "levels.flac"
    rate = 48000
    with wave.open(str(wav), "wb") as out:
        out.setparams((1, 2, rate, 0, "NONE", ""))
        for second, amplitude in enumerate([16000, 16000, 1600, 1600, 16000]):
            frequency = 997 + second
            out.writeframes(b"".join(struct.pack("<h", round(amplitude * math.sin(2 * math.pi * frequency * i / rate)))
                                     for i in range(rate)))
    run(ffmpeg, "-v", "error", "-i", wav, "-c:a", "flac", flac)
    for recording, codec in ((wav, "pcm_s16le"), (flac, "flac")):
        actual = run(ffprobe, "-v", "error", "-select_streams", "a:0", "-show_entries",
                     "stream=codec_name", "-of", "csv=p=0", recording).stdout.decode().strip()
        assert actual == codec, (recording.name, actual)

    # Check the exact byte range returned after reopening the file and seeking to a new anchor.
    first, last = 80 * 588, 330 * 588
    seek = 3 * 44100 + 131
    full = encode(ffmpeg, flac, "pcm", (first, last)).stdout
    tail = encode(ffmpeg, flac, "pcm", (seek, last)).stdout
    assert len(full) == (last - first) * 4 and any(full), "CUE PCM length or content"
    assert tail == full[(seek - first) * 4:], "CUE PCM seek changed the sample sequence"

    whole = loudness(ffmpeg, flac)
    quiet = loudness(ffmpeg, flac, (151, 299))
    assert 18 < whole[0] - quiet[0] < 21 and 19 < whole[1] - quiet[1] < 21, (whole, quiet)
    print(f"CUE: {len(full) // 4} stereo frames, {len(tail)} matching seek bytes. "
          f"Loudness: {whole[0]:.2f} LUFS full, {quiet[0]:.2f} LUFS segment.")

    encoder_rows = [line.split() for line in run(ffmpeg, "-hide_banner", "-encoders").stdout.decode().splitlines()]
    muxer_rows = [line.split() for line in run(ffmpeg, "-hide_banner", "-muxers").stdout.decode().splitlines()]
    encoders = {row[1] for row in encoder_rows if len(row) >= 2 and row[0].startswith("A")}
    muxers = {name for row in muxer_rows if len(row) >= 2 and row[0] == "E" for name in row[1].split(",")}
    assert "pcm_s16le" in encoders and "s16le" in muxers, "CUE PCM missing from capability lists"
    available = set()
    for fmt, (encoder, muxer, _) in FORMATS.items():
        result = encode(ffmpeg, flac, fmt)
        if result.returncode:
            error = result.stderr.decode(errors="replace")
            if fmt == "aac" or (encoder in encoders and muxer in muxers):
                raise RuntimeError(f"{fmt}: {error}")
            missing = []
            if encoder not in encoders:
                missing.append(f"{encoder} encoder")
            if muxer not in muxers:
                missing.append(f"{muxer} muxer")
            print(f"{fmt} unavailable: missing {', '.join(missing)}")
            continue
        assert encoder in encoders and muxer in muxers, f"{fmt} worked but was missing from capability lists"
        output = temp / f"stream.{muxer}"
        output.write_bytes(result.stdout)
        probe = json.loads(run(ffprobe, "-v", "error", "-count_frames", "-show_streams",
                               "-show_format", "-of", "json", output).stdout)
        stream, = probe["streams"]
        assert muxer in probe["format"]["format_name"].split(","), probe["format"]
        assert stream["codec_name"] == fmt and stream["channels"] == 2 and stream["sample_rate"] == "48000", stream
        assert int(stream["nb_read_frames"]) > 1 and abs(float(probe["format"]["duration"]) - .75) < .1, probe
        if fmt == "aac":
            assert b"moof" in result.stdout and b"mdat" in result.stdout, "missing MP4 audio fragments"
        available.add(fmt)
    print(f"Verified streaming formats: {', '.join(sorted(available))}")
    return available
