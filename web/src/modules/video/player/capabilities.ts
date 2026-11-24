/**
 * Browser media capability detection reported to `POST /items/{id}/video-playback`.
 * Accepts injectable probe functions so unit tests can exercise Chrome, Safari, Firefox and
 * restricted environments deterministically without relying on jsdom's empty `<video>` stub.
 */
import Hls from "hls.js";
import type { ClientCapabilities } from "../api";

export interface CapabilityProbeEnv {
  canPlayType: (mime: string) => string;
  mseSupported: boolean;
  isTypeSupported: (mime: string) => boolean;
}

const CONTAINER_PROBES: ReadonlyArray<[name: string, mimes: readonly string[]]> = [
  ["mp4", ['video/mp4; codecs="avc1.42E01E, mp4a.40.2"', "video/mp4"]],
  ["webm", ['video/webm; codecs="vp9, opus"', "video/webm"]],
  ["ogg", ['video/ogg; codecs="theora, vorbis"', "video/ogg"]],
  ["mov", ["video/quicktime"]],
];

const VIDEO_CODEC_PROBES: ReadonlyArray<[name: string, mimes: readonly string[]]> = [
  ["h264", ['video/mp4; codecs="avc1.640028"', 'video/mp4; codecs="avc1.42E01E"']],
  ["hevc", ['video/mp4; codecs="hvc1.1.6.L120.90"', 'video/mp4; codecs="hev1.1.6.L120.90"']],
  ["vp9", ['video/webm; codecs="vp09.00.10.08"', 'video/webm; codecs="vp9"']],
  ["av1", ['video/mp4; codecs="av01.0.08M.08"', 'video/webm; codecs="av01.0.04M.08"']],
];

const MSE_HEVC_MUXED_PROBES = [
  'video/mp4; codecs="hvc1.1.6.L120.90, mp4a.40.2"',
  'video/mp4; codecs="hvc1.2.4.L150.90, mp4a.40.2"',
] as const;

const AUDIO_CODEC_PROBES: ReadonlyArray<[name: string, mimes: readonly string[]]> = [
  ["aac", ['audio/mp4; codecs="mp4a.40.2"']],
  ["mp3", ["audio/mpeg"]],
  ["opus", ['audio/webm; codecs="opus"', 'audio/ogg; codecs="opus"']],
  ["flac", ["audio/flac", 'audio/mp4; codecs="flac"']],
  ["vorbis", ['audio/ogg; codecs="vorbis"']],
  ["ac3", ['audio/mp4; codecs="ac-3"']],
  ["eac3", ['audio/mp4; codecs="ec-3"']],
];

const HLS_NATIVE_MIMES = [
  "application/vnd.apple.mpegurl",
  "application/x-mpegURL",
] as const;

function defaultProbeEnv(): CapabilityProbeEnv {
  const video = typeof document !== "undefined" ? document.createElement("video") : null;
  const mediaSource = typeof globalThis !== "undefined" ? globalThis.MediaSource : undefined;
  return {
    canPlayType: (mime) => {
      try {
        return video?.canPlayType(mime) ?? "";
      } catch {
        return "";
      }
    },
    mseSupported: Boolean(
      (typeof Hls !== "undefined" && typeof Hls.isSupported === "function" && Hls.isSupported()) ||
        (mediaSource && typeof mediaSource.isTypeSupported === "function"),
    ),
    isTypeSupported: (mime) => {
      try {
        return Boolean(mediaSource && typeof mediaSource.isTypeSupported === "function" && mediaSource.isTypeSupported(mime));
      } catch {
        return false;
      }
    },
  };
}

function playable(result: string): boolean {
  return result === "probably" || result === "maybe";
}

export function detectClientCapabilities(env: CapabilityProbeEnv = defaultProbeEnv()): ClientCapabilities {
  const supportsAny = (mimes: readonly string[]) =>
    mimes.some((mime) => playable(env.canPlayType(mime)) || env.isTypeSupported(mime));

  const nativeHls = HLS_NATIVE_MIMES.some((mime) => playable(env.canPlayType(mime)));
  const mseHls =
    env.mseSupported &&
    (env.isTypeSupported('video/mp4; codecs="avc1.42E01E, mp4a.40.2"') ||
      env.isTypeSupported('video/mp2t; codecs="avc1.42E01E, mp4a.40.2"') ||
      !nativeHls);

  // Prefer MSE (`hls.js`) whenever available so browsers like desktop Chrome (which report
  // `"maybe"` for `application/vnd.apple.mpegurl` via an incomplete built-in HLS handler) use
  // `hls.js` reliably, while iOS Safari (no MSE) cleanly uses native HLS.
  let hls: ClientCapabilities["hls"] = "none";
  if (mseHls) hls = "mse";
  else if (nativeHls) hls = "native";

  const containers = CONTAINER_PROBES.filter(([, mimes]) =>
    mimes.some((mime) => playable(env.canPlayType(mime))),
  ).map(([name]) => name);

  const videoCodecs = VIDEO_CODEC_PROBES.filter(([name, mimes]) => {
    if (name === "hevc" && hls === "mse") {
      // Only advertise HEVC for MSE remuxing when MediaSource accepts both Main and Main 10
      // muxed with AAC; otherwise Chrome throws `bufferAddCodecError` on 10-bit HEVC MKVs.
      return MSE_HEVC_MUXED_PROBES.every((mime) => env.isTypeSupported(mime));
    }
    return supportsAny(mimes);
  }).map(([name]) => name);
  const audioCodecs = AUDIO_CODEC_PROBES.filter(([, mimes]) => supportsAny(mimes)).map(([name]) => name);

  return {
    containers: containers.length > 0 ? containers : ["mp4"],
    videoCodecs: videoCodecs.length > 0 ? videoCodecs : ["h264"],
    audioCodecs: audioCodecs.length > 0 ? audioCodecs : ["aac"],
    hls,
  };
}
