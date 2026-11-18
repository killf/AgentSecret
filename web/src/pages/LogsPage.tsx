import { useEffect, useState } from 'react';
import { Button, Drawer, Input, Select, Space } from 'antd';
import { SearchOutlined } from '@ant-design/icons';
import { PageHeader, PanelCard } from '../components/Page';
import { Badge, DataTable } from '../components';
import { getRequestLog, listRequestLogs, type RequestLogDetail, type RequestLogItem } from '../api/logs';
import { formatBoolean, formatTime } from '../utils/format';

export default function LogsPage() {
  const [rows, setRows] = useState<RequestLogItem[]>([]);
  const [status, setStatus] = useState<string | undefined>();
  const [detail, setDetail] = useState<RequestLogDetail | null>(null);

  async function refresh() {
    const result = await listRequestLogs(status ? { status } : {});
    setRows(result.items);
  }

  useEffect(() => { void refresh(); }, []);

  return (
    <div className="page-shell">
      <PageHeader title="请求日志" description="审计模型网关调用的用量、耗时、供应商与敏感信息命中情况。" />

      <PanelCard>
        <Space wrap>
          <Select
            allowClear
            value={status}
            onChange={setStatus}
            placeholder="请求状态"
            style={{ width: 180 }}
            options={[
              { value: 'success', label: '成功' },
              { value: 'failure', label: '失败' },
            ]}
          />
          <Button type="primary" icon={<SearchOutlined />} onClick={() => void refresh()}>查询</Button>
        </Space>
      </PanelCard>

      <PanelCard>
        <DataTable<RequestLogItem> rows={rows} columns={[
          { key: 'created_at', title: '时间', render: row => formatTime(row.created_at), width: 170 },
          { key: 'request_status', title: '状态', render: row => <Badge tone={row.request_status === 'success' ? 'success' : 'danger'}>{row.request_status === 'success' ? '成功' : '失败'}</Badge> },
          { key: 'user_email', title: '用户' },
          { key: 'model_name', title: '模型' },
          { key: 'provider_name', title: '供应商' },
          { key: 'input_tokens', title: '输入' },
          { key: 'output_tokens', title: '输出' },
          { key: 'duration_ms', title: '耗时(ms)' },
          { key: 'contains_sensitive', title: '敏感', render: row => <Badge tone={row.contains_sensitive ? 'danger' : 'muted'}>{formatBoolean(row.contains_sensitive)}</Badge> },
          { key: 'actions', title: '操作', render: row => <Button size="small" type="text" onClick={() => void getRequestLog(row.id).then(setDetail)}>详情</Button> },
        ]} />
      </PanelCard>

      <Drawer title={`请求详情 #${detail?.id ?? ''}`} open={detail !== null} onClose={() => setDetail(null)} width={720}>
        {detail && (
          <>
            <h3>请求</h3>
            <pre>{JSON.stringify(detail.request_detail, null, 2)}</pre>
            <h3>响应</h3>
            <pre>{JSON.stringify(detail.response_detail, null, 2)}</pre>
          </>
        )}
      </Drawer>
    </div>
  );
}