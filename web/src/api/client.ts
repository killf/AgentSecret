export interface ApiEnvelope<T> {
  code: number;
  message: string;
  data: T;
}

export class ApiError extends Error {
  constructor(public code: number, message: string) {
    super(message);
  }
}

export function getToken(): string | null {
  return localStorage.getItem('agentsecret-token');
}

export function setToken(token: string | null): void {
  if (token) localStorage.setItem('agentsecret-token', token);
  else localStorage.removeItem('agentsecret-token');
}

export async function post<T>(path: string, body: unknown = {}): Promise<T> {
  const response = await fetch(path, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...(getToken() ? { Authorization: `Bearer ${getToken()}` } : {}),
    },
    body: JSON.stringify(body),
  });
  const payload = (await response.json()) as ApiEnvelope<T>;
  if (payload.code !== 0) {
    if (payload.code === 2) {
      setToken(null);
      localStorage.removeItem('agentsecret-user');
      window.location.assign('/login');
    }
    throw new ApiError(payload.code, payload.message);
  }

  return payload.data;
}
