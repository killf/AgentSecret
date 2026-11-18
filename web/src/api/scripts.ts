import { post } from './client';

export interface ScriptItem {
  id: number;
  project_id: number;
  environment_id: number | null;
  sandbox_id: number | null;
  name: string;
  description: string | null;
  type: 'bash' | 'python';
  review_status: string;
  revision: number;
  updated_at: string;
}

export interface ScriptDetail extends ScriptItem {
  params: unknown;
  content?: string;
  credential_actions?: unknown;
}

export const listScripts = (project_id: number) =>
  post<ScriptItem[]>('/api/v1/scripts/list', { project_id });
export const getScript = (id: number) => post<ScriptDetail>('/api/v1/scripts/detail', { id });
export const addScript = (body: Record<string, unknown>) =>
  post<{ id: number; revision: number }>('/api/v1/scripts/add', body);
export const updateScript = (body: Record<string, unknown>) =>
  post<{ id: number; revision: number }>('/api/v1/scripts/update', body);
export const reviewScript = (id: number, revision: number, decision: 'approve' | 'reject') =>
  post<null>('/api/v1/scripts/review', { id, revision, decision });
export const runScript = (id: number, params: Record<string, unknown> = {}) =>
  post<{ execution_id: number }>('/api/v1/scripts/run', { id, params });
export const deleteScript = (id: number) => post<null>('/api/v1/scripts/delete', { id });
