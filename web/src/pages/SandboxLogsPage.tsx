import { useEffect, useState } from 'react';
import { Button, Drawer, Select, Space, Typography } from 'antd';
import { SearchOutlined } from '@ant-design/icons';
import { PageHeader, PanelCard } from '../components/Page';
import { Badge, DataTable } from '../components';
import { getExecutionLog, listExecutions, type ExecutionItem } from '../api/logs';
import { formatTime } from '../utils/format';

export default function SandboxLogsPage() {
  const [rows, setRows] = useState<ExecutionItem[]>([]);
  const [status, setStatus] = useState<string | undefined>();
  const [detail, setDetail] = useState<ExecutionItem | null>(null);

  async function refresh() {
    const result = await listExecutions(status ? { status } : {});
    setRows(result.items);
  }

  useEffect(() => { void refresh(); }, []);

  return (
    <div className="page-shell">
      <PageHeader title="沙盒日志" description="查看脚本执行状态、错误码和过滤后的标准输出 / 标准错误。" />

      <PanelCard>
        <Space wrap>
          <Select
            allowClear
            value={status}
            onChange={setStatus}
            placeholder="执行状态"
            style={{ width: 180 }}
            options={[
              { value: 'running', label: '运行中' },
              { value: 'success', label: '成功' },
              { value: 'failed', label: '失败' },
            ]}
          />
          <Button type="primary" icon={<SearchOutlined />} onClick={() => void refresh()}>查询</Button>
        </Space>
      </PanelCard>

      <PanelCard>
        <DataTable<ExecutionItem> rows={rows} columns={[
          { key: 'id', title: 'ID', width: 80 },
          { key: 'status', title: '状态', render: row => <Badge tone={row.status === 'success' ? 'success' : row.status === 'failed' ? 'danger' : 'running'}>{row.status}</Badge> },
          { key: 'started_at', title: '开始', render: row => formatTime(row.started_at) },
          { key: 'finished_at', title: '结束', render: row => formatTime(row.finished_at) },
          { key: 'error_code', title: '错误码', render: row => row.error_code || '—' },
          { key: 'actions', title: '操作', render: row => <Button size="small" type="text" onClick={() => void getExecutionLog(row.id).then(setDetail)}>详情</Button> },
        ]} />
      </PanelCard>

      <Drawer title={`执行详情 #${detail?.id ?? ''}`} open={detail !== null} onClose={() => setDetail(null)} width={680}>
        {detail && (
          <>
            <Typography.Title level={5}>标准输出</Typography.Title>
            <pre>{detail.stdout ?? '—'}</pre>
            <Typography.Title level={5}>标准错误</Typography.Title>
            <pre>{detail.stderr ?? '—'}</pre>
          </>
        )}
      </Drawer>
    </div>
  );
}