import { post } from './client';

export interface RequestLogItem {
  id: number;
  created_at: string;
  request_status: string;
  user_email: string;
  model_name: string;
  provider_name: string;
  input_tokens: number;
  output_tokens: number;
  duration_ms: number;
  contains_sensitive: boolean;
  failure_reason: string | null;
}

export interface ExecutionItem {
  id: number;
  status: string;
  stdout: string | null;
  stderr: string | null;
  started_at: string | null;
  finished_at: string | null;
  error_code: string | null;
}

export const listRequestLogs = (body: Record<string, unknown> = {}) =>
  post<{ total: number; items: RequestLogItem[] }>('/api/v1/logs/request/list', {
    page: 1,
    page_size: 20,
    ...body,
  });

export const listExecutions = (body: Record<string, unknown> = {}) =>
  post<{ total: number; items: ExecutionItem[] }>('/api/v1/logs/executions/list', {
    page: 1,
    page_size: 20,
    ...body,
  });

export interface RequestLogDetail extends RequestLogItem {
  request_detail: unknown;
  response_detail: unknown;
}

export const getRequestLog = (id: number) =>
  post<RequestLogDetail>('/api/v1/logs/request/detail', { id });
export const getExecutionLog = (id: number) =>
  post<ExecutionItem>('/api/v1/logs/executions/detail', { id });
