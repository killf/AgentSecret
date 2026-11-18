import { useEffect, useState } from 'react';
import { App as AntApp, Button, Drawer, Form, Input, Popconfirm, Select, Space } from 'antd';
import { PlusOutlined, ReloadOutlined, SyncOutlined } from '@ant-design/icons';
import { PageHeader, PanelCard } from '../../components/Page';
import { Badge, DataTable } from '../../components';
import {
  addProvider,
  deleteProvider,
  listProviders,
  refreshProviderQuota,
  updateProvider,
  type ProviderInfo,
} from '../../api/admin';
import { formatTime } from '../../utils/format';

interface ProviderForm {
  name: string;
  base_url: string;
  access_token: string;
  provider_type: string;
}

export default function ProvidersPage() {
  const [rows, setRows] = useState<ProviderInfo[]>([]);
  const [editing, setEditing] = useState<ProviderInfo | null>(null);
  const [editToken, setEditToken] = useState('');
  const [form] = Form.useForm<ProviderForm>();
  const { message } = AntApp.useApp();

  async function refresh() { setRows(await listProviders()); }

  async function create(values: ProviderForm) {
    await addProvider(values);
    message.success('供应商已创建');
    form.resetFields();
    await refresh();
  }

  async function saveEdit() {
    if (!editing) return;

    await updateProvider({
      id: editing.id,
      name: editing.name,
      base_url: editing.base_url,
      ...(editToken ? { access_token: editToken } : {}),
    });
    setEditing(null);
    setEditToken('');
    message.success('供应商已更新');
    await refresh();
  }

  useEffect(() => { void refresh(); }, []);

  return (
    <div className="page-shell">
      <PageHeader
        title="供应商管理"
        description="维护上游服务地址、访问令牌、启用状态和额度。"
        extra={<Button icon={<ReloadOutlined />} onClick={() => void refresh()}>刷新</Button>}
      />

      <PanelCard title="新增供应商">
        <Form<ProviderForm> form={form} layout="vertical" onFinish={create} requiredMark={false} initialValues={{ provider_type: 'codex' }}>
          <div className="form-grid">
            <Form.Item label="名称" name="name" rules={[{ required: true, message: '请输入名称' }]}>
              <Input />
            </Form.Item>
            <Form.Item label="类型" name="provider_type">
              <Select options={[
                { value: 'codex', label: 'Codex' },
                { value: 'glm', label: 'GLM' },
                { value: 'minimax', label: 'MiniMax' },
              ]} />
            </Form.Item>
            <Form.Item label="服务地址" name="base_url" rules={[{ required: true, message: '请输入服务地址' }]}>
              <Input placeholder="https://api.example.com" />
            </Form.Item>
            <Form.Item label="访问令牌" name="access_token" rules={[{ required: true, message: '请输入访问令牌' }]}>
              <Input.Password placeholder="仅提交时传输" />
            </Form.Item>
            <Form.Item label=" ">
              <Button type="primary" htmlType="submit" icon={<PlusOutlined />}>创建</Button>
            </Form.Item>
          </div>
        </Form>
      </PanelCard>

      <PanelCard title="供应商列表">
        <DataTable<ProviderInfo> rows={rows} columns={[
          { key: 'name', title: '名称' },
          { key: 'provider_type', title: '类型' },
          { key: 'base_url', title: '服务地址' },
          { key: 'quota_5h_remaining', title: '5h 剩余', render: row => row.quota_5h_remaining ?? '—' },
          { key: 'quota_weekly_remaining', title: '周剩余', render: row => row.quota_weekly_remaining ?? '—' },
          { key: 'quota_5h_refreshed_at', title: '刷新时间', render: row => formatTime(row.quota_5h_refreshed_at) },
          { key: 'status', title: '状态', render: row => <Badge tone={row.disabled_at ? 'muted' : 'success'}>{row.disabled_at ? '禁用' : '启用'}</Badge> },
          { key: 'actions', title: '操作', render: row => (
            <Space size={4}>
              <Button size="small" type="text" icon={<SyncOutlined />} onClick={() => void refreshProviderQuota(row.id).then(refresh)}>刷新额度</Button>
              <Button size="small" type="text" onClick={() => setEditing(structuredClone(row))}>编辑</Button>
              <Button size="small" type="text" onClick={() => void updateProvider({ id: row.id, disabled_at: row.disabled_at ? null : new Date().toISOString() }).then(refresh)}>{row.disabled_at ? '启用' : '禁用'}</Button>
              <Popconfirm title={`删除供应商 ${row.name}？`} onConfirm={() => void deleteProvider(row.id).then(refresh)}>
                <Button size="small" type="text" danger>删除</Button>
              </Popconfirm>
            </Space>
          ) },
        ]} />
      </PanelCard>

      <Drawer title={`编辑供应商：${editing?.name ?? ''}`} open={editing !== null} onClose={() => setEditing(null)} width={520}>
        {editing && (
          <Form layout="vertical">
            <Form.Item label="名称">
              <Input value={editing.name} onChange={event => setEditing({ ...editing, name: event.target.value })} />
            </Form.Item>
            <Form.Item label="服务地址">
              <Input value={editing.base_url} onChange={event => setEditing({ ...editing, base_url: event.target.value })} />
            </Form.Item>
            <Form.Item label="新访问令牌">
              <Input.Password value={editToken} onChange={event => setEditToken(event.target.value)} placeholder="留空表示不修改" />
            </Form.Item>
            <Button type="primary" onClick={() => void saveEdit()}>保存</Button>
          </Form>
        )}
      </Drawer>
    </div>
  );
}