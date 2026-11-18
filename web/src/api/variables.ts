import { post } from './client';

export interface VariableItem {
  id: number;
  key: string;
  source_env: string;
  description: string | null;
  disabled_at: string | null;
}

export const listVariables = (project_id: number, environment_id: number) =>
  post<VariableItem[]>('/api/v1/variables/list', { project_id, environment_id });
export const addVariable = (body: Record<string, unknown>) =>
  post<{ id: number }>('/api/v1/variables/add', body);
export const updateVariable = (body: Record<string, unknown>) =>
  post<null>('/api/v1/variables/update', body);
export const deleteVariable = (id: number) =>
  post<null>('/api/v1/variables/delete', { id });
