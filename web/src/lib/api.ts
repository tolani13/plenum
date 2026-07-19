// The one fetch wrapper. Same-origin `/api/*` (Vite proxies to the API on
// 127.0.0.1:5777), JSON in/out, credentials included so the session cookie
// rides along. Non-2xx bodies are the API's typed envelope
// { error: { code, message } } — surfaced as a typed ApiError, never swallowed.

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

interface ErrorEnvelope {
  error?: { code?: string; message?: string };
}

async function parse<T>(res: Response): Promise<T> {
  const text = await res.text();
  const body: unknown = text ? JSON.parse(text) : null;
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

export async function apiGet<T>(path: string): Promise<T> {
  const res = await fetch(path, {
    method: "GET",
    credentials: "same-origin",
    headers: { Accept: "application/json" },
  });
  return parse<T>(res);
}

export async function apiPost<T>(path: string, body?: unknown): Promise<T> {
  const res = await fetch(path, {
    method: "POST",
    credentials: "same-origin",
    headers: { "Content-Type": "application/json", Accept: "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  return parse<T>(res);
}
