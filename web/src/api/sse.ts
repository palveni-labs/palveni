/**
 * Server-sent events over `fetch`.
 *
 * The contract sends the bearer token in the Authorization header and never in the URL, which
 * rules out the browser's EventSource. This module implements the parts of the SSE wire format
 * (https://html.spec.whatwg.org/multipage/server-sent-events.html) the client needs, plus
 * reconnection with exponential backoff.
 */

export interface SseMessage {
  /** Defaults to "message" when the server sends no `event:` field. */
  event: string;
  data: string;
  id: string | null;
}

/** Incremental parser: feed decoded text chunks of any size, get complete messages back. */
export class SseParser {
  private buffer = "";
  private eventType = "";
  private dataLines: string[] = [];
  private lastEventId: string | null = null;
  private pendingCarriageReturn = false;
  private firstChunk = true;
  /** Last `retry:` value in milliseconds, if the server sent one. */
  retry: number | null = null;

  feed(chunk: string): SseMessage[] {
    let text = chunk;
    if (this.firstChunk && text.length > 0) {
      this.firstChunk = false;
      if (text.charCodeAt(0) === 0xfeff) text = text.slice(1);
    }
    // A CRLF split across chunks: the CR already ended the line, so drop the LF.
    if (this.pendingCarriageReturn && text.startsWith("\n")) text = text.slice(1);
    this.pendingCarriageReturn = false;

    this.buffer += text;
    const messages: SseMessage[] = [];
    let start = 0;
    for (let i = 0; i < this.buffer.length; i++) {
      const char = this.buffer[i];
      if (char !== "\n" && char !== "\r") continue;
      const line = this.buffer.slice(start, i);
      if (char === "\r") {
        if (i + 1 < this.buffer.length) {
          if (this.buffer[i + 1] === "\n") i++;
        } else {
          this.pendingCarriageReturn = true;
        }
      }
      start = i + 1;
      const message = this.processLine(line);
      if (message) messages.push(message);
    }
    this.buffer = this.buffer.slice(start);
    return messages;
  }

  private processLine(line: string): SseMessage | null {
    if (line === "") return this.dispatch();
    if (line.startsWith(":")) return null;

    const colon = line.indexOf(":");
    const field = colon === -1 ? line : line.slice(0, colon);
    let value = colon === -1 ? "" : line.slice(colon + 1);
    if (value.startsWith(" ")) value = value.slice(1);

    switch (field) {
      case "event":
        this.eventType = value;
        break;
      case "data":
        this.dataLines.push(value);
        break;
      case "id":
        if (!value.includes("\0")) this.lastEventId = value;
        break;
      case "retry":
        if (/^\d+$/.test(value)) this.retry = Number(value);
        break;
      default:
        // Unknown fields are ignored.
        break;
    }
    return null;
  }

  private dispatch(): SseMessage | null {
    const hasData = this.dataLines.length > 0;
    const message: SseMessage = {
      event: this.eventType || "message",
      data: this.dataLines.join("\n"),
      id: this.lastEventId,
    };
    this.eventType = "";
    this.dataLines = [];
    return hasData ? message : null;
  }
}

export type SseConnectionState = "connecting" | "open" | "reconnecting" | "closed";

export interface SseClientOptions {
  url: string;
  /** Read on every (re)connect so a refreshed token is picked up. */
  getToken: () => string | null;
  onMessage: (message: SseMessage) => void;
  onStateChange?: (state: SseConnectionState) => void;
  /** Called after a successful reconnect; events may have been missed while disconnected. */
  onReconnect?: (lastEventId: string | null) => void;
  /** The server rejected the token; the client stops. */
  onUnauthorized?: () => void;
  /** Reconnect when nothing (not even a ping) arrived for this long. Default 45 s (pings every 20 s). */
  idleTimeoutMs?: number;
  initialRetryMs?: number;
  maxRetryMs?: number;
  fetchImpl?: typeof fetch;
}

/** Delay before reconnect attempt `attempt` (0-based): exponential with ±20 % jitter. */
export function backoffDelay(attempt: number, initialMs: number, maxMs: number, random = Math.random): number {
  const base = Math.min(maxMs, initialMs * 2 ** attempt);
  return Math.round(base * (0.8 + random() * 0.4));
}

class UnauthorizedError extends Error {}

/** Opens the stream and keeps it open until the returned function is called. */
export function connectSse(options: SseClientOptions): () => void {
  const {
    url,
    getToken,
    onMessage,
    onStateChange,
    onReconnect,
    onUnauthorized,
    idleTimeoutMs = 45_000,
    initialRetryMs = 1_000,
    maxRetryMs = 30_000,
    fetchImpl = (...args) => fetch(...args),
  } = options;

  let stopped = false;
  let controller: AbortController | null = null;
  let retryTimer: ReturnType<typeof setTimeout> | undefined;
  let attempt = 0;
  let serverRetry: number | null = null;
  let everOpened = false;
  let lastEventId: string | null = null;

  const setState = (state: SseConnectionState) => onStateChange?.(state);

  const run = async () => {
    if (stopped) return;
    const token = getToken();
    if (!token) {
      setState("closed");
      return;
    }
    setState(everOpened ? "reconnecting" : "connecting");
    controller = new AbortController();
    const current = controller;
    let idleTimer: ReturnType<typeof setTimeout> | undefined;
    const armIdle = () => {
      clearTimeout(idleTimer);
      idleTimer = setTimeout(() => current.abort(), idleTimeoutMs);
    };

    try {
      const headers: Record<string, string> = {
        Accept: "text/event-stream",
        Authorization: `Bearer ${token}`,
      };
      if (lastEventId) headers["Last-Event-ID"] = lastEventId;
      armIdle();
      const response = await fetchImpl(url, { headers, signal: current.signal, cache: "no-store" });
      if (response.status === 401) throw new UnauthorizedError();
      if (!response.ok || !response.body) throw new Error(`SSE HTTP ${response.status}`);

      const reconnected = everOpened;
      everOpened = true;
      attempt = 0;
      setState("open");
      if (reconnected) onReconnect?.(lastEventId);

      const parser = new SseParser();
      const reader = response.body.pipeThrough(new TextDecoderStream()).getReader();
      for (;;) {
        const { value, done } = await reader.read();
        if (done) break;
        armIdle();
        for (const message of parser.feed(value)) {
          onMessage(message);
          if (message.id !== null) lastEventId = message.id;
        }
        if (parser.retry !== null) serverRetry = parser.retry;
      }
    } catch (error) {
      if (error instanceof UnauthorizedError) {
        stopped = true;
        setState("closed");
        onUnauthorized?.();
        return;
      }
      // Aborted by the idle watchdog or a network failure: fall through to reconnect.
    } finally {
      current.abort();
      clearTimeout(idleTimer);
    }

    if (stopped) return;
    setState("reconnecting");
    const delay = serverRetry ?? backoffDelay(attempt, initialRetryMs, maxRetryMs);
    attempt++;
    retryTimer = setTimeout(() => void run(), delay);
  };

  void run();

  return () => {
    stopped = true;
    clearTimeout(retryTimer);
    controller?.abort();
    setState("closed");
  };
}
