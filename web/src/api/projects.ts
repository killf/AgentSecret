import { post } from './client';

export interface Project {
  id: number;
  name: string;
  description: string | null;
  guidance: string | null;
  repos: string[];
  created_at: string;
  disabled_at: string | null;
}

export interface Environment {
  id: number;
  project_id: number;
  name: string;
  description: string | null;
  created_at: string;
}

export const listProjects = () => post<Project[]>('/api/v1/projects/list');
export const addProject = (body: { name: string; description?: string; guidance?: string; repos?: string[] }) =>
  post<{ id: number }>('/api/v1/projects/add', body);
export const updateProject = (body: Record<string, unknown>) =>
  post<null>('/api/v1/projects/update', body);
export const deleteProject = (id: number) => post<null>('/api/v1/projects/delete', { id });
export const listEnvironments = (project_id: number) =>
  post<Environment[]>('/api/v1/environments/list', { project_id });
export const addEnvironment = (body: { project_id: number; name: string; description?: string }) =>
  post<{ id: number }>('/api/v1/environments/add', body);
export const updateEnvironment = (body: Record<string, unknown>) =>
  post<null>('/api/v1/environments/update', body);
export const deleteEnvironment = (id: number) => post<null>('/api/v1/environments/delete', { id });
