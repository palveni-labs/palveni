import type { ApiErrorBody, ApiErrorCode, ApiErrorDetails } from "./types";

/** Codes produced on the client when no server error body is available. */
export type ClientErrorCode = "network_error" | "unexpected_response";

export type ErrorCode = ApiErrorCode | ClientErrorCode;

const KNOWN_CODES: ReadonlySet<string> = new Set<ErrorCode>([
  "validation_failed",
  "path_not_found",
  "unauthorized",
  "invalid_credentials",
  "too_many_requests",
  "forbidden",
  "not_found",
  "setup_already_completed",
  "conflict",
  "method_not_allowed",
  "internal",
  "network_error",
  "unexpected_response",
]);

/** A failed API call: either a non-2xx response or a transport failure (`status` 0). */
export class ApiError extends Error {
  override readonly name = "ApiError";
  readonly status: number;
  /** Error code from the response body; unknown server codes are kept verbatim. */
  readonly code: ErrorCode | (string & {});
  /** Where the error happened, when the server says (may be absent: never rely on it). */
  readonly details: ApiErrorDetails | null;

  constructor(status: number, code: string, message: string, options?: { cause?: unknown; details?: ApiErrorDetails | null }) {
    super(message, options);
    this.status = status;
    this.code = code;
    this.details = options?.details ?? null;
  }

  /** True when the code is one the UI has a translated message for. */
  get isKnownCode(): boolean {
    return KNOWN_CODES.has(this.code);
  }
}

export function isApiError(error: unknown, code?: ErrorCode): error is ApiError {
  return error instanceof ApiError && (code === undefined || error.code === code);
}

function isErrorBody(value: unknown): value is ApiErrorBody {
  if (typeof value !== "object" || value === null || !("error" in value)) return false;
  const inner = value.error;
  return (
    typeof inner === "object" &&
    inner !== null &&
    typeof (inner as { code?: unknown }).code === "string" &&
    typeof (inner as { message?: unknown }).message === "string"
  );
}

/** Keeps only well-typed detail fields. */
function cleanDetails(value: unknown): ApiErrorDetails | null {
  if (typeof value !== "object" || value === null) return null;
  const raw = value as Record<string, unknown>;
  const details: ApiErrorDetails = {};
  if (typeof raw.field === "string") details.field = raw.field;
  if (typeof raw.index === "number" && Number.isInteger(raw.index) && raw.index >= 0) details.index = raw.index;
  if (typeof raw.path === "string") details.path = raw.path;
  return Object.keys(details).length > 0 ? details : null;
}

/** Fallback code for responses without a contract-shaped body (proxies, crashes). */
function codeForStatus(status: number): ErrorCode {
  switch (status) {
    case 400:
      return "validation_failed";
    case 401:
      return "unauthorized";
    case 403:
      return "forbidden";
    case 404:
      return "not_found";
    case 405:
      return "method_not_allowed";
    case 429:
      return "too_many_requests";
    case 409:
      return "conflict";
    default:
      return status >= 500 ? "internal" : "unexpected_response";
  }
}

/** Builds an ApiError from a non-2xx response, tolerating bodies that are not JSON. */
export async function errorFromResponse(response: Response): Promise<ApiError> {
  let text = "";
  try {
    text = await response.text();
  } catch {
    // Body unreadable; fall through to the status-based error.
  }
  if (text) {
    try {
      const parsed: unknown = JSON.parse(text);
      if (isErrorBody(parsed)) {
        return new ApiError(response.status, parsed.error.code, parsed.error.message, { details: cleanDetails(parsed.error.details) });
      }
    } catch {
      // Not JSON (e.g. an HTML error page from a reverse proxy).
    }
  }
  const message = response.statusText || `HTTP ${response.status}`;
  return new ApiError(response.status, codeForStatus(response.status), message);
}
