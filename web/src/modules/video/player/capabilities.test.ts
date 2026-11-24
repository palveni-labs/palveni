import { describe, expect, it } from "vitest";
import { detectClientCapabilities } from "./capabilities";

describe("detectClientCapabilities", () => {
  it("reports mse HLS and codecs for Chromium-like browsers", () => {
    const caps = detectClientCapabilities({
      canPlayType: (mime) => {
        if (mime.includes("video/mp4") || mime.includes("audio/mp4")) return "probably";
        if (mime.includes("video/webm") || mime.includes("audio/webm")) return "probably";
        if (mime.includes("audio/mpeg") || mime.includes("audio/flac")) return "probably";
        return "";
      },
      mseSupported: true,
      isTypeSupported: (mime) => mime.includes("avc1") || mime.includes("vp09") || mime.includes("av01"),
    });

    expect(caps.hls).toBe("mse");
    expect(caps.containers).toEqual(expect.arrayContaining(["mp4", "webm"]));
    expect(caps.videoCodecs).toEqual(expect.arrayContaining(["h264", "vp9", "av1"]));
    expect(caps.audioCodecs).toEqual(expect.arrayContaining(["aac", "mp3", "opus", "flac"]));
  });

  it("reports native HLS and HEVC for Safari-like browsers", () => {
    const caps = detectClientCapabilities({
      canPlayType: (mime) => {
        if (mime === "application/vnd.apple.mpegurl") return "maybe";
        if (mime.includes("video/mp4") || mime.includes("quicktime")) return "probably";
        if (mime.includes("hvc1") || mime.includes("mp4a.40.2")) return "probably";
        return "";
      },
      mseSupported: false,
      isTypeSupported: () => false,
    });

    expect(caps.hls).toBe("native");
    expect(caps.containers).toEqual(expect.arrayContaining(["mp4", "mov"]));
    expect(caps.videoCodecs).toEqual(expect.arrayContaining(["h264", "hevc"]));
    expect(caps.audioCodecs).toContain("aac");
  });

  it("prefers mse HLS over Chrome native HLS probes and gates MSE HEVC on Main 10 support", () => {
    const caps = detectClientCapabilities({
      canPlayType: (mime) => {
        if (mime === "application/vnd.apple.mpegurl") return "maybe";
        if (mime === "video/x-matroska") return "maybe";
        if (mime.includes("hvc1.1.6")) return "probably";
        if (mime.includes("video/mp4") || mime.includes("audio/mp4")) return "probably";
        return "";
      },
      mseSupported: true,
      isTypeSupported: (mime) =>
        mime.includes("avc1") || mime.includes("hvc1.1.6.L120.90"),
    });

    expect(caps.hls).toBe("mse");
    expect(caps.containers).not.toContain("mkv");
    expect(caps.videoCodecs).toContain("h264");
    expect(caps.videoCodecs).not.toContain("hevc");
  });

  it("falls back safely when browser probes return empty strings", () => {
    const caps = detectClientCapabilities({
      canPlayType: () => "",
      mseSupported: false,
      isTypeSupported: () => false,
    });

    expect(caps.hls).toBe("none");
    expect(caps.containers).toEqual(["mp4"]);
    expect(caps.videoCodecs).toEqual(["h264"]);
    expect(caps.audioCodecs).toEqual(["aac"]);
  });
});
