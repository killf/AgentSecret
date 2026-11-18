import { useEffect, useState } from 'react';
import { App as AntApp, Button, Input, Popconfirm, Space, Typography } from 'antd';
import { PlusOutlined, ReloadOutlined } from '@ant-design/icons';
import { PageHeader, PanelCard } from '../components/Page';
import { Badge, DataTable } from '../components';
import { addToken, listTokens, resetToken, tokenAction, type TokenInfo } from '../api/tokens';
import { formatTime } from '../utils/format';

export default function TokensPage() {
  const [rows, setRows] = useState<TokenInfo[]>([]);
  const [name, setName] = useState('');
  const [message, setMessage] = useState('');
  const { message: apiMessage } = AntApp.useApp();

  async function refresh() {
    setRows(await listTokens());
  }

  async function create() {
    const result = await addToken(name || undefined);
    setMessage(result.token);
    apiMessage.success('访问令牌已创建');
    setName('');
    await refresh();
  }

  useEffect(() => { void refresh(); }, []);

  return (
    <div className="page-shell">
      <PageHeader title="访问令牌" description="管理模型网关与 MCP 使用的长期访问令牌。" extra={<Button icon={<ReloadOutlined />} onClick={() => void refresh()}>刷新</Button>} />

      <PanelCard title="新增令牌">
        <Space.Compact style={{ width: '100%', maxWidth: 520 }}>
          <Input value={name} onChange={event => setName(event.target.value)} placeholder="令牌备注，例如 CI 集成" />
          <Button type="primary" icon={<PlusOutlined />} onClick={() => void create()}>创建</Button>
        </Space.Compact>
      </PanelCard>

      {message && (
        <PanelCard title="新令牌明文">
          <Typography.Paragraph copyable={{ text: message }} style={{ margin: 0 }}>{message}</Typography.Paragraph>
          <Typography.Text type="warning">仅本次创建后完整显示，请立即保存。</Typography.Text>
        </PanelCard>
      )}

      <PanelCard title="令牌列表">
        <DataTable<TokenInfo> rows={rows} columns={[
          { key: 'name', title: '备注', render: row => row.name || '—' },
          { key: 'created_at', title: '创建时间', render: row => formatTime(row.created_at) },
          { key: 'expires_at', title: '过期时间', render: row => formatTime(row.expires_at) },
          { key: 'status', title: '状态', render: row => <Badge tone={row.disabled_at ? 'muted' : 'success'}>{row.disabled_at ? '禁用' : '启用'}</Badge> },
          { key: 'actions', title: '操作', render: row => (
            <Space size={4}>
              <Button size="small" type="text" onClick={() => void tokenAction(row.disabled_at ? 'enable' : 'disable', row.id).then(refresh)}>
                {row.disabled_at ? '启用' : '禁用'}
              </Button>
              <Button size="small" type="text" onClick={() => void resetToken(row.id).then(result => setMessage(result.token)).then(refresh)}>重置</Button>
              <Popconfirm title="删除令牌？" onConfirm={() => void tokenAction('delete', row.id).then(refresh)}>
                <Button size="small" type="text" danger>删除</Button>
              </Popconfirm>
            </Space>
          ) },
        ]} />
      </PanelCard>
    </div>
  );
}