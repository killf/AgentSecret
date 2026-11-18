import { post } from './client';

export interface TokenInfo {
  id: number;
  name: string | null;
  expires_at: string | null;
  disabled_at: string | null;
  created_at: string;
}

export interface TokenDetail extends TokenInfo {
  token: string;
}

export const listTokens = () => post<TokenInfo[]>('/api/v1/tokens/list');
export const addToken = (name?: string) => post<TokenDetail>('/api/v1/tokens/add', { name });
export const tokenAction = (path: string, id: number) => post<null>(`/api/v1/tokens/${path}`, { id });
export const resetToken = (id: number) =>
  post<{ token: string; expires_at: string | null }>('/api/v1/tokens/reset', { id });
