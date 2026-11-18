import { useEffect, useState } from 'react';
import {
  CodeSandboxOutlined,
  FileTextOutlined,
  FolderOutlined,
  KeyOutlined,
} from '@ant-design/icons';
import { PageHeader, PanelCard } from '../components/Page';
import { Badge, DataTable } from '../components';
import { listRequestLogs, listExecutions, type ExecutionItem, type RequestLogItem } from '../api/logs';
import { listTokens, type TokenInfo } from '../api/tokens';
import { listProjects, type Project } from '../api/projects';
import { formatBoolean, formatTime } from '../utils/format';

interface Summary {
  requests: number;
  executions: number;
  tokens: number;
  projects: number;
}

const metrics = [
  { key: 'requests', label: '请求总数', icon: <FileTextOutlined /> },
  { key: 'executions', label: '沙盒执行', icon: <CodeSandboxOutlined /> },
  { key: 'tokens', label: '访问令牌', icon: <KeyOutlined /> },
  { key: 'projects', label: '项目数量', icon: <FolderOutlined /> },
] as const;

export default function DashboardPage() {
  const [summary, setSummary] = useState<Summary | null>(null);
  const [requests, setRequests] = useState<RequestLogItem[]>([]);
  const [executions, setExecutions] = useState<ExecutionItem[]>([]);
  const [tokens, setTokens] = useState<TokenInfo[]>([]);
  const [projects, setProjects] = useState<Project[]>([]);

  useEffect(() => {
    void (async () => {
      const [requestPage, executionPage, tokenRows, projectRows] = await Promise.all([
        listRequestLogs({ page: 1, page_size: 5 }),
        listExecutions({ page: 1, page_size: 5 }),
        listTokens(),
        listProjects(),
      ]);
      setSummary({
        requests: requestPage.total,
        executions: executionPage.total,
        tokens: tokenRows.length,
        projects: projectRows.length,
      });
      setRequests(requestPage.items);
      setExecutions(executionPage.items);
      setTokens(tokenRows);
      setProjects(projectRows);
    })();
  }, []);

  return (
    <div className="page-shell">
      <PageHeader
        title="总览"
        description="AgentSecret 的模型调用、沙盒执行和访问面状态。"
        extra={<div className="system-status"><span />服务运行中</div>}
      />

      <div className="metric-grid">
        {metrics.map(metric => (
          <div className="metric-card" key={metric.key}>
            <span className="metric-icon">{metric.icon}</span>
            <div>
              <strong>{summary ? summary[metric.key] : '—'}</strong>
              <span>{metric.label}</span>
            </div>
          </div>
        ))}
      </div>

      <div className="dashboard-grid">
        <PanelCard title="最近模型请求">
          <DataTable<RequestLogItem> rows={requests} pageSize={5} columns={[
            { key: 'created_at', title: '时间', render: row => formatTime(row.created_at) },
            { key: 'model_name', title: '模型' },
            { key: 'request_status', title: '状态', render: row => <Badge tone={row.request_status === 'success' ? 'success' : 'danger'}>{row.request_status}</Badge> },
            { key: 'contains_sensitive', title: '敏感', render: row => formatBoolean(row.contains_sensitive) },
          ]} />
        </PanelCard>

        <PanelCard title="最近沙盒执行">
          <DataTable<ExecutionItem> rows={executions} pageSize={5} columns={[
            { key: 'id', title: 'ID' },
            { key: 'status', title: '状态', render: row => <Badge tone={row.status === 'success' ? 'success' : row.status === 'failed' ? 'danger' : 'running'}>{row.status}</Badge> },
            { key: 'started_at', title: '开始时间', render: row => formatTime(row.started_at) },
            { key: 'error_code', title: '错误', render: row => row.error_code || '—' },
          ]} />
        </PanelCard>
      </div>

      <div className="dashboard-grid compact">
        <PanelCard title="访问令牌">
          <div className="entity-list">
            {tokens.slice(0, 5).map(token => (
              <div key={token.id}><span>{token.name || `Token #${token.id}`}</span><Badge tone={token.disabled_at ? 'muted' : 'success'}>{token.disabled_at ? '禁用' : '启用'}</Badge></div>
            ))}
            {tokens.length === 0 && <div className="empty-inline">暂无令牌</div>}
          </div>
        </PanelCard>

        <PanelCard title="项目">
          <div className="entity-list">
            {projects.slice(0, 5).map(project => (
              <div key={project.id}><span>{project.name}</span><span>{project.repos.length} 仓库</span></div>
            ))}
            {projects.length === 0 && <div className="empty-inline">暂无项目</div>}
          </div>
        </PanelCard>
      </div>
    </div>
  );
}