import { describe, expect, it } from "vitest";
import { formatVttTimestamp, offsetWebVtt, parseVttTimestamp } from "./subtitles";

describe("WebVTT subtitle offset", () => {
  it("parses and formats timestamps accurately", () => {
    expect(parseVttTimestamp("01:02:03.450")).toBe(3_723_450);
    expect(parseVttTimestamp("02:10.500")).toBe(130_500);
    expect(formatVttTimestamp(3_723_450)).toBe("01:02:03.450");
  });

  it("shifts cues by HLS startMs, clamping partial cues and dropping expired cues", () => {
    const sample = [
      "WEBVTT",
      "",
      "00:00:02.000 --> 00:00:05.000",
      "Expired cue",
      "",
      "00:00:08.000 --> 00:00:14.000 align:center",
      "Overlapping cue",
      "",
      "00:00:15.500 --> 00:00:20.000",
      "Future cue",
    ].join("\n");

    const shifted = offsetWebVtt(sample, 10_000);
    expect(shifted).not.toContain("Expired cue");
    expect(shifted).toContain("00:00:00.000 --> 00:00:04.000 align:center");
    expect(shifted).toContain("Overlapping cue");
    expect(shifted).toContain("00:00:05.500 --> 00:00:10.000");
    expect(shifted).toContain("Future cue");
  });

  it("returns original text untouched when offset is 0", () => {
    const sample = "WEBVTT\n\n00:00:01.000 --> 00:00:03.000\nHello\n";
    expect(offsetWebVtt(sample, 0)).toBe(sample);
  });
});
