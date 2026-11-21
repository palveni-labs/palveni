import { describe, expect, it, vi } from "vitest";
import { backoffDelay, connectSse, SseParser, type SseConnectionState, type SseMessage } from "./sse";

function feedAll(chunks: string[]): SseMessage[] {
  const parser = new SseParser();
  return chunks.flatMap((chunk) => parser.feed(chunk));
}

describe("SseParser", () => {
  const stream = 'event: job.updated\ndata: {"id":"j1"}\n\n: keep-alive comment\n\nevent: ping\ndata: {}\n\n';

  it("parses named events and ignores comments", () => {
    expect(feedAll([stream])).toEqual([
      { event: "job.updated", data: '{"id":"j1"}', id: null },
      { event: "ping", data: "{}", id: null },
    ]);
  });

  it("gives the same result however the stream is chunked", () => {
    const expected = feedAll([stream]);
    expect(feedAll([...stream])).toEqual(expected);
    expect(feedAll([stream.slice(0, 7), stream.slice(7, 30), stream.slice(30)])).toEqual(expected);
  });

  it("accepts CRLF and CR line endings, including a CRLF split across chunks", () => {
    expect(feedAll(["event: a\r\ndata: 1\r", "\n\r\n", "event: b\rdata: 2\r\r"])).toEqual([
      { event: "a", data: "1", id: null },
      { event: "b", data: "2", id: null },
    ]);
  });

  it("joins multi-line data, defaults the event name and keeps the last id", () => {
    expect(feedAll(["id: 7\ndata: first\ndata:second\n\n", "data: x\n\n"])).toEqual([
      { event: "message", data: "first\nsecond", id: "7" },
      { event: "message", data: "x", id: "7" },
    ]);
  });

  it("drops events without data, strips a leading BOM and reads retry", () => {
    const parser = new SseParser();
    expect(parser.feed("﻿event: lonely\n\nretry: 2500\ndata: ok\n\n")).toEqual([{ event: "message", data: "ok", id: null }]);
    expect(parser.retry).toBe(2500);
  });

  it("holds back an incomplete event until its blank line arrives", () => {
    const parser = new SseParser();
    expect(parser.feed("event: library.deleted\ndata: {\"id\":")).toEqual([]);
    expect(parser.feed('"l1"}\n\n')).toEqual([{ event: "library.deleted", data: '{"id":"l1"}', id: null }]);
  });
});

describe("backoffDelay", () => {
  it("grows exponentially and is capped", () => {
    const mid = () => 0.5; // no jitter
    expect([0, 1, 2, 3, 10].map((attempt) => backoffDelay(attempt, 1000, 30_000, mid))).toEqual([1000, 2000, 4000, 8000, 30_000]);
  });
});

function streamResponse(chunks: string[]): Response {
  const encoder = new TextEncoder();
  return new Response(
    new ReadableStream<Uint8Array>({
      start(controller) {
        for (const chunk of chunks) controller.enqueue(encoder.encode(chunk));
        controller.close();
      },
    }),
    { status: 200, headers: { "Content-Type": "text/event-stream" } },
  );
}

describe("connectSse", () => {
  it("sends the token in the header, reconnects after the stream ends and stops on 401", async () => {
    const responses = [
      streamResponse(['event: ping\ndata: {}\n\nid: 37\nevent: library.deleted\ndata: {"id":"l1"}\n\nid: 38\ndata: partial']),
      streamResponse(["event: ping\ndata: {}\n\n"]),
      new Response(null, { status: 401 }),
    ];
    const fetchImpl = vi.fn(() => {
      const next = responses.shift();
      return next ? Promise.resolve(next) : Promise.reject(new Error("unexpected extra connect"));
    });
    const messages: string[] = [];
    const states: SseConnectionState[] = [];
    const onReconnect = vi.fn();
    const onUnauthorized = vi.fn();

    const stop = connectSse({
      url: "/api/v1/events",
      getToken: () => "secret",
      onMessage: (message) => messages.push(message.event),
      onStateChange: (state) => states.push(state),
      onReconnect,
      onUnauthorized,
      initialRetryMs: 1,
      maxRetryMs: 2,
      fetchImpl,
    });

    await vi.waitFor(() => expect(onUnauthorized).toHaveBeenCalledTimes(1));
    stop();

    expect(fetchImpl).toHaveBeenCalledTimes(3);
    const [url, init] = fetchImpl.mock.calls[0] as unknown as [string, RequestInit];
    expect(url).toBe("/api/v1/events");
    expect(url).not.toContain("secret");
    expect((init.headers as Record<string, string>).Authorization).toBe("Bearer secret");
    const [, reconnect] = fetchImpl.mock.calls[1] as unknown as [string, RequestInit];
    expect(new Headers(reconnect.headers).get("Last-Event-ID")).toBe("37");
    expect(onReconnect).toHaveBeenCalledWith("37");
    expect(messages).toEqual(["ping", "library.deleted", "ping"]);
    expect(onReconnect).toHaveBeenCalledTimes(1);
    expect(states).toContain("open");
    expect(states).toContain("reconnecting");
    expect(states.at(-1)).toBe("closed");
  });

  it("does not connect without a token", () => {
    const fetchImpl = vi.fn();
    const stop = connectSse({ url: "/api/v1/events", getToken: () => null, onMessage: () => {}, fetchImpl });
    stop();
    expect(fetchImpl).not.toHaveBeenCalled();
  });
});
