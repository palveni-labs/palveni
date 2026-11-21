import { ApiError, errorFromResponse } from "./errors";
import { clearSession, getToken } from "./session";

export const API_BASE = "/api/v1";

type QueryValue = string | number | boolean | null | undefined;

export interface RequestOptions {
  method?: "GET" | "POST" | "PUT" | "PATCH" | "DELETE";
  query?: Record<string, QueryValue>;
  body?: unknown;
  signal?: AbortSignal;
  /** Attach the bearer token when one is stored. Defaults to true. */
  auth?: boolean;
}

let unauthorizedHandler: (() => void) | null = null;

/**
 * Called after a request that carried a token is rejected with 401: the token has already been
 * cleared, the handler only has to move the user to the login page.
 */
export function setUnauthorizedHandler(handler: (() => void) | null) {
  unauthorizedHandler = handler;
}

/** Invalidates the stored session and notifies the app; shared with the SSE client. */
export function handleUnauthorized() {
  clearSession();
  unauthorizedHandler?.();
}

export function buildUrl(path: string, query?: Record<string, QueryValue>): string {
  let url = `${API_BASE}${path}`;
  if (query) {
    const params = new URLSearchParams();
    for (const [key, value] of Object.entries(query)) {
      if (value === undefined || value === null) continue;
      params.set(key, String(value));
    }
    const search = params.toString();
    if (search) url += `?${search}`;
  }
  return url;
}

function isAbort(error: unknown): boolean {
  return error instanceof DOMException && error.name === "AbortError";
}

export async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const { method = "GET", query, body, signal, auth = true } = options;
  const headers = new Headers({ Accept: "application/json" });
  const token = auth ? getToken() : null;
  if (token) headers.set("Authorization", `Bearer ${token}`);
  if (body !== undefined) headers.set("Content-Type", body instanceof Blob ? "application/x-tar" : "application/json");

  let response: Response;
  try {
    response = await fetch(buildUrl(path, query), {
      method,
      headers,
      body: body instanceof Blob ? body : body === undefined ? undefined : JSON.stringify(body),
      signal,
      credentials: "same-origin",
    });
  } catch (error) {
    if (isAbort(error) || signal?.aborted) throw error;
    const message = error instanceof Error ? error.message : String(error);
    throw new ApiError(0, "network_error", message, { cause: error });
  }

  if (!response.ok) {
    const error = await errorFromResponse(response);
    // A 401 on a request that carried a token means the session is gone. Login failures
    // (`invalid_credentials`) are answered without a token and never end up here.
    if (response.status === 401 && token && error.code !== "invalid_credentials") {
      handleUnauthorized();
    }
    throw error;
  }

  if (response.status === 204 || response.headers.get("Content-Length") === "0") {
    return undefined as T;
  }
  const text = await response.text();
  if (!text) return undefined as T;
  try {
    return JSON.parse(text) as T;
  } catch (error) {
    throw new ApiError(response.status, "unexpected_response", "Response is not valid JSON", { cause: error });
  }
}

/** Binary responses (images). Same auth and error handling as `request`. */
export async function requestBlob(path: string, options: Pick<RequestOptions, "signal"> = {}): Promise<Blob> {
  const headers = new Headers();
  const token = getToken();
  if (token) headers.set("Authorization", `Bearer ${token}`);
  let response: Response;
  try {
    response = await fetch(buildUrl(path), { headers, signal: options.signal, credentials: "same-origin" });
  } catch (error) {
    if (isAbort(error) || options.signal?.aborted) throw error;
    throw new ApiError(0, "network_error", error instanceof Error ? error.message : String(error), { cause: error });
  }
  if (!response.ok) throw await errorFromResponse(response);
  return response.blob();
}
