import { useEffect, useState } from 'react';
import { App as AntApp, Button, Form, InputNumber, Popconfirm, Select, Space } from 'antd';
import { PlusOutlined } from '@ant-design/icons';
import { PageHeader, PanelCard } from '../../components/Page';
import { DataTable } from '../../components';
import { deletePermission, listPermissions, upsertPermission, type Permission } from '../../api/admin';

const limitedKinds = ['models', 'providers'];
const kindLabels: Record<string, string> = {
  projects: '项目',
  environments: '环境',
  models: '模型',
  providers: '供应商',
};

interface PermissionForm {
  user_id: number;
  target_id: number | null;
  daily_token_limit: number | null;
  weekly_token_limit: number | null;
  monthly_token_limit: number | null;
}

export default function PermissionsPage() {
  const [kind, setKind] = useState('projects');
  const [rows, setRows] = useState<Permission[]>([]);
  const [form] = Form.useForm<PermissionForm>();
  const { message } = AntApp.useApp();

  async function refresh() { setRows(await listPermissions(kind)); }

  async function save(values: PermissionForm) {
    await upsertPermission({
      type: kind,
      user_id: values.user_id,
      target_id: values.target_id ?? null,
      ...(limitedKinds.includes(kind)
        ? {
            limits: {
              daily_token_limit: values.daily_token_limit ?? null,
              weekly_token_limit: values.weekly_token_limit ?? null,
              monthly_token_limit: values.monthly_token_limit ?? null,
            },
          }
        : {}),
    });
    message.success('权限已保存');
    await refresh();
  }

  useEffect(() => {
    form.resetFields();
    void refresh();
  }, [kind, form]);

  return (
    <div className="page-shell">
      <PageHeader title="权限管理" description="按项目、环境、模型和供应商控制用户可见范围与调用限额。" />

      <PanelCard title="权限配置">
        <Space style={{ marginBottom: 18 }}>
          <Select
            value={kind}
            onChange={setKind}
            style={{ width: 180 }}
            options={Object.entries(kindLabels).map(([value, label]) => ({ value, label }))}
          />
        </Space>

        <Form<PermissionForm> form={form} layout="vertical" onFinish={save} requiredMark={false}>
          <div className="form-grid">
            <Form.Item label="用户 ID" name="user_id" rules={[{ required: true, message: '请输入用户 ID' }]}>
              <InputNumber min={1} style={{ width: '100%' }} />
            </Form.Item>
            <Form.Item label="目标 ID" name="target_id" extra="留空表示全部可见">
              <InputNumber min={1} style={{ width: '100%' }} />
            </Form.Item>
            {limitedKinds.includes(kind) && (
              <>
                <Form.Item label="日限额" name="daily_token_limit">
                  <InputNumber min={0} style={{ width: '100%' }} placeholder="不限" />
                </Form.Item>
                <Form.Item label="周限额" name="weekly_token_limit">
                  <InputNumber min={0} style={{ width: '100%' }} placeholder="不限" />
                </Form.Item>
                <Form.Item label="月限额" name="monthly_token_limit">
                  <InputNumber min={0} style={{ width: '100%' }} placeholder="不限" />
                </Form.Item>
              </>
            )}
            <Form.Item label=" ">
              <Button type="primary" htmlType="submit" icon={<PlusOutlined />}>保存</Button>
            </Form.Item>
          </div>
        </Form>
      </PanelCard>

      <PanelCard title={`${kindLabels[kind]}权限`}>
        <DataTable<Permission> rows={rows} columns={[
          { key: 'id', title: 'ID' },
          { key: 'user_id', title: '用户 ID' },
          { key: 'target_id', title: '目标', render: row => row.target_id ?? '全部' },
          { key: 'limits', title: '日 / 周 / 月限额', render: row => row.limits ? `${row.limits.daily_token_limit ?? '—'} / ${row.limits.weekly_token_limit ?? '—'} / ${row.limits.monthly_token_limit ?? '—'}` : '—' },
          { key: 'actions', title: '操作', render: row => (
            <Popconfirm title={`删除权限记录 #${row.id}？`} onConfirm={() => void deletePermission(kind, row.id).then(refresh)}>
              <Button size="small" type="text" danger>删除</Button>
            </Popconfirm>
          ) },
        ]} />
      </PanelCard>
    </div>
  );
}