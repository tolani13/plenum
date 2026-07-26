// The one fetch wrapper. Same-origin `/api/*` (Vite proxies to the API on
// 127.0.0.1:5777), JSON in/out, credentials included so the session cookie
// rides along. Non-2xx bodies are the API's typed envelope
// { error: { code, message } } — surfaced as a typed ApiError, never swallowed.
//
// D-2 (2026-07-25): a failure now says WHICH KIND of failure it was. Two
// distinct outcomes leave this module, and the screens tell them apart:
//   · ApiError     — the API answered, and answered with its typed envelope.
//                    Carries the server's status, code and message verbatim.
//   · NetworkError — the request never got an answer: fetch itself rejected
//                    (DNS, TLS, offline, connection reset, aborted), or the
//                    body could not be read. This is the ONLY case in which
//                    a screen may claim the API could not be reached.
// Before this, fetch's raw TypeError escaped untyped and every screen
// reported every failure — 500s included — as a connectivity problem.

/** A typed API failure carrying the HTTP status and the server's error code. */
export class ApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly code: string,
    message: string,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

/** The request never reached the API, or its answer never arrived intact. */
export class NetworkError extends Error {
  constructor(public readonly cause: unknown) {
    super("Couldn’t reach the API.");
    this.name = "NetworkError";
  }
}

interface ErrorEnvelope {
  error?: { code?: string; message?: string };
}

async function parse<T>(res: Response): Promise<T> {
  let body: unknown;
  try {
    const text = await res.text();
    body = text ? JSON.parse(text) : null;
  } catch (cause) {
    // The status line arrived but the body did not (connection dropped
    // mid-response, or the response was not JSON). On a non-2xx we still know
    // the status, so that stays an ApiError; on a 2xx the payload is lost and
    // that IS a transport failure.
    if (!res.ok) {
      throw new ApiError(res.status, "error", `request failed (${res.status})`);
    }
    throw new NetworkError(cause);
  }
  if (!res.ok) {
    const env = (body ?? {}) as ErrorEnvelope;
    throw new ApiError(
      res.status,
      env.error?.code ?? "error",
      env.error?.message ?? `request failed (${res.status})`,
    );
  }
  return body as T;
}

/** fetch rejects only when the request never got an answer — type it as such. */
async function send(path: string, init: RequestInit): Promise<Response> {
  try {
    return await fetch(path, init);
  } catch (cause) {
    throw new NetworkError(cause);
  }
}

export async function apiGet<T>(path: string): Promise<T> {
  const res = await send(path, {
    method: "GET",
    credentials: "same-origin",
    headers: { Accept: "application/json" },
  });
  return parse<T>(res);
}

export async function apiPost<T>(path: string, body?: unknown): Promise<T> {
  const res = await send(path, {
    method: "POST",
    credentials: "same-origin",
    headers: { "Content-Type": "application/json", Accept: "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  return parse<T>(res);
}

export async function apiPatch<T>(path: string, body?: unknown): Promise<T> {
  const res = await send(path, {
    method: "PATCH",
    credentials: "same-origin",
    headers: { "Content-Type": "application/json", Accept: "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  return parse<T>(res);
}

export async function apiPut<T>(path: string, body?: unknown): Promise<T> {
  const res = await send(path, {
    method: "PUT",
    credentials: "same-origin",
    headers: { "Content-Type": "application/json", Accept: "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  return parse<T>(res);
}

export async function apiDelete<T>(path: string): Promise<T> {
  const res = await send(path, {
    method: "DELETE",
    credentials: "same-origin",
    headers: { Accept: "application/json" },
  });
  return parse<T>(res);
}

/** One true sentence about a failed request, for the screens to render. */
export function describeError(error: unknown): string {
  if (error instanceof ApiError) {
    const what =
      error.status >= 500
        ? "The API failed on this request"
        : "The API rejected this request";
    return `${what}: ${error.message} (${error.code}, HTTP ${error.status}).`;
  }
  if (error instanceof NetworkError) {
    return "Couldn’t reach the API — the request never got a response.";
  }
  if (error instanceof Error && error.message) {
    return `The request failed: ${error.message}`;
  }
  return "The request failed, and gave no reason.";
}
