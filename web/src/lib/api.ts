export class ApiError extends Error {
  status: number;

  constructor(message: string, status: number) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
  }
}

export async function api<T>(path: string, init: RequestInit = {}): Promise<T> {
  const controller = new AbortController();
  const timeout = window.setTimeout(() => controller.abort(), 10_000);
  try {
    const response = await fetch(path, {
      credentials: 'same-origin',
      ...init,
      signal: init.signal || controller.signal,
      headers: { 'content-type': 'application/json', ...(init.headers || {}) },
    });
    if (!response.ok) {
      const body = await response.json().catch(() => ({ error: `HTTP ${response.status}` }));
      throw new ApiError(body.error || `HTTP ${response.status}`, response.status);
    }
    return response.status === 204 ? (undefined as T) : response.json();
  } catch (cause) {
    if (cause instanceof DOMException && cause.name === 'AbortError') {
      throw new Error(`Request timed out while contacting the server (${path})`);
    }
    throw cause;
  } finally {
    window.clearTimeout(timeout);
  }
}

export function formatFrequency(frequency: number | null): string {
  if (frequency === null) return 'frequency not shared';
  if (frequency >= 1_000_000) return `${(frequency / 1_000_000).toFixed(6)} MHz`;
  if (frequency >= 1_000) return `${(frequency / 1_000).toFixed(3)} kHz`;
  return `${frequency} Hz`;
}
