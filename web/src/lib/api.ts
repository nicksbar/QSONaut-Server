export async function api<T>(path: string, init: RequestInit = {}): Promise<T> {
  const response = await fetch(path, {
    credentials: 'same-origin',
    ...init,
    headers: { 'content-type': 'application/json', ...(init.headers || {}) },
  });
  if (!response.ok) {
    const body = await response.json().catch(() => ({ error: `HTTP ${response.status}` }));
    throw new Error(body.error);
  }
  return response.status === 204 ? (undefined as T) : response.json();
}

export function formatFrequency(frequency: number | null): string {
  if (frequency === null) return 'frequency not shared';
  if (frequency >= 1_000_000) return `${(frequency / 1_000_000).toFixed(6)} MHz`;
  if (frequency >= 1_000) return `${(frequency / 1_000).toFixed(3)} kHz`;
  return `${frequency} Hz`;
}
