import { post } from './client';

export interface ModelItem {
  id: number;
  name: string;
  disabled_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface ProviderInfo {
  id: number;
  name: string;
  base_url: string;
  provider_type: string;
  quota_5h_remaining: number | null;
  quota_5h_refreshed_at: string | null;
  quota_weekly_remaining: number | null;
  quota_weekly_refreshed_at: string | null;
  quota_monthly_remaining: number | null;
  quota_monthly_refreshed_at: string | null;
  disabled_at: string | null;
}

export interface AdminUser {
  id: number;
  email: string;
  name: string;
  is_admin: boolean;
  banned_at: string | null;
  unban_at: string | null;
  last_login_at: string | null;
  created_at: string;
}

export interface Permission {
  id: number;
  user_id: number;
  target_id: number | null;
  limits: { daily_token_limit: number | null; weekly_token_limit: number | null; monthly_token_limit: number | null } | null;
}

export interface Setting {
  id: number;
  key: string;
  value: string;
  description: string | null;
  updated_at: string;
}

export const listModels = () => post<ModelItem[]>('/api/v1/admin/models/list');
export const addModel = (name: string) => post<{ id: number }>('/api/v1/admin/models/add', { name });
export const updateModel = (body: Record<string, unknown>) =>
  post<null>('/api/v1/admin/models/update', body);
export const deleteModel = (id: number) => post<null>('/api/v1/admin/models/delete', { id });

export interface ModelProviderRelation {
  id: number;
  model_id: number;
  provider_id: number;
  model_name: string;
  max_output_tokens: number;
  max_context_tokens: number;
  enabled: boolean;
}

export const listModelProviders = (model_id?: number) =>
  post<ModelProviderRelation[]>('/api/v1/admin/model-providers/list', { model_id });
export const listProviders = () => post<ProviderInfo[]>('/api/v1/admin/providers/list');
export const updateProvider = (body: Record<string, unknown>) =>
  post<null>('/api/v1/admin/providers/update', body);
export const deleteProvider = (id: number) =>
  post<null>('/api/v1/admin/providers/delete', { id });
export const addProvider = (body: { name: string; base_url: string; access_token: string; provider_type: string }) =>
  post<{ id: number }>('/api/v1/admin/providers/add', body);
export const listUsers = (body: Record<string, unknown> = {}) =>
  post<{ total: number; items: AdminUser[] }>('/api/v1/admin/users/list', { page: 1, page_size: 20, ...body });
export const listPermissions = (kind: string, user_id?: number) =>
  post<Permission[]>('/api/v1/admin/permissions/list', { type: kind, user_id });
export const upsertPermission = (body: Record<string, unknown>) =>
  post<{ id: number }>('/api/v1/admin/permissions', body);
export const deletePermission = (type: string, id: number) =>
  post<null>('/api/v1/admin/permissions/delete', { type, id });
export const listSettings = () => post<Setting[]>('/api/v1/admin/settings/list');

export interface ConnectivityResult {
  ok: boolean;
  latency_ms: number | null;
  error: string | null;
}

export const addModelProvider = (body: Record<string, unknown>) =>
  post<{ id: number }>('/api/v1/admin/model-providers/add', body);
export const updateModelProvider = (body: Record<string, unknown>) =>
  post<null>('/api/v1/admin/model-providers/update', body);
export const deleteModelProvider = (id: number) =>
  post<null>('/api/v1/admin/model-providers/delete', { id });
export const testModelProvider = (id: number) =>
  post<ConnectivityResult>('/api/v1/admin/model-providers/test', { id });
export const refreshProviderQuota = (id: number) =>
  post<unknown>('/api/v1/admin/providers/refresh-quota', { id });
export const createUser = (body: { email: string; name: string; password: string }) =>
  post<{ id: number }>('/api/v1/admin/users/add', body);
export const userAction = (path: string, body: Record<string, unknown>) =>
  post<null>(`/api/v1/admin/users/${path}`, body);
