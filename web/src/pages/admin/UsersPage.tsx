import { useEffect, useState } from 'react';
import { App as AntApp, Button, Drawer, Form, Input, Popconfirm, Space, Switch } from 'antd';
import { PlusOutlined, SearchOutlined, TeamOutlined } from '@ant-design/icons';
import { PageHeader, PanelCard } from '../../components/Page';
import { Badge, DataTable } from '../../components';
import { createUser, listUsers, userAction, type AdminUser } from '../../api/admin';
import { formatTime } from '../../utils/format';
import { sha1 } from '../../utils/sha1';

interface UserForm {
  email: string;
  name: string;
  password: string;
}

export default function UsersPage() {
  const [rows, setRows] = useState<AdminUser[]>([]);
  const [keyword, setKeyword] = useState('');
  const [editing, setEditing] = useState<AdminUser | null>(null);
  const [passwordUser, setPasswordUser] = useState<AdminUser | null>(null);
  const [newPassword, setNewPassword] = useState('');
  const [form] = Form.useForm<UserForm>();
  const { message } = AntApp.useApp();

  async function refresh(body: Record<string, unknown> = {}) {
    const result = await listUsers({ page: 1, page_size: 20, ...body });
    setRows(result.items);
  }

  async function create(values: UserForm) {
    const result = await createUser({ ...values, password: await sha1(values.password) });
    message.success(`用户已创建：#${result.id}`);
    form.resetFields();
    await refresh(keyword ? { keyword } : {});
  }

  async function saveEdit() {
    if (!editing) return;

    await userAction('update', { id: editing.id, name: editing.name, is_admin: editing.is_admin });
    setEditing(null);
    await refresh(keyword ? { keyword } : {});
  }

  async function resetPassword() {
    if (!passwordUser || !newPassword) return;

    await userAction('password', { id: passwordUser.id, new_password: await sha1(newPassword) });
    setPasswordUser(null);
    setNewPassword('');
    message.success('密码已重置');
  }

  useEffect(() => { void refresh(); }, []);

  return (
    <div className="page-shell">
      <PageHeader title="用户管理" description="管理登录用户、管理员角色、封禁状态和初始密码。" />

      <PanelCard title="新增用户">
        <Form<UserForm> form={form} layout="vertical" onFinish={create} requiredMark={false}>
          <div className="form-grid">
            <Form.Item label="邮箱" name="email" rules={[{ required: true, message: '请输入邮箱' }]}>
              <Input />
            </Form.Item>
            <Form.Item label="显示名" name="name" rules={[{ required: true, message: '请输入显示名' }]}>
              <Input />
            </Form.Item>
            <Form.Item label="初始密码" name="password" rules={[{ required: true, message: '请输入初始密码' }]}>
              <Input.Password />
            </Form.Item>
            <Form.Item label=" ">
              <Button type="primary" htmlType="submit" icon={<PlusOutlined />}>创建</Button>
            </Form.Item>
          </div>
        </Form>
      </PanelCard>

      <PanelCard title="用户列表" extra={<Space>
        <Input value={keyword} onChange={event => setKeyword(event.target.value)} prefix={<SearchOutlined />} placeholder="搜索邮箱或显示名" style={{ width: 280 }} />
        <Button type="primary" onClick={() => void refresh({ keyword })}>搜索</Button>
      </Space>}>
        <DataTable<AdminUser> rows={rows} columns={[
          { key: 'email', title: '邮箱' },
          { key: 'name', title: '显示名' },
          { key: 'is_admin', title: '角色', render: row => <Badge tone={row.is_admin ? 'running' : 'muted'}>{row.is_admin ? '管理员' : '成员'}</Badge> },
          { key: 'banned_at', title: '状态', render: row => <Badge tone={row.banned_at ? 'danger' : 'success'}>{row.banned_at ? '封禁' : '正常'}</Badge> },
          { key: 'last_login_at', title: '最近登录', render: row => formatTime(row.last_login_at) },
          { key: 'created_at', title: '创建时间', render: row => formatTime(row.created_at) },
          { key: 'actions', title: '操作', render: row => (
            <Space size={4}>
              <Button size="small" type="text" icon={<TeamOutlined />} onClick={() => setEditing(structuredClone(row))}>编辑</Button>
              <Button size="small" type="text" onClick={() => setPasswordUser(row)}>设置密码</Button>
              <Popconfirm title={row.banned_at ? `解禁 ${row.email}？` : `封禁 ${row.email}？`} onConfirm={() => void userAction(row.banned_at ? 'unban' : 'ban', { id: row.id, permanent: false }).then(() => refresh(keyword ? { keyword } : {}))}>
                <Button size="small" type="text">{row.banned_at ? '解禁' : '封禁'}</Button>
              </Popconfirm>
              <Popconfirm title={`删除用户 ${row.email}？`} onConfirm={() => void userAction('delete', { id: row.id }).then(() => refresh(keyword ? { keyword } : {}))}>
                <Button size="small" type="text" danger>删除</Button>
              </Popconfirm>
            </Space>
          ) },
        ]} />
      </PanelCard>

      <Drawer title={`编辑用户：${editing?.email ?? ''}`} open={editing !== null} onClose={() => setEditing(null)} width={480}>
        {editing && (
          <Form layout="vertical">
            <Form.Item label="显示名">
              <Input value={editing.name} onChange={event => setEditing({ ...editing, name: event.target.value })} />
            </Form.Item>
            <Form.Item label="管理员">
              <Switch checked={editing.is_admin} onChange={checked => setEditing({ ...editing, is_admin: checked })} />
            </Form.Item>
            <Button type="primary" onClick={() => void saveEdit()}>保存</Button>
          </Form>
        )}
      </Drawer>

      <Drawer title={`设置密码：${passwordUser?.email ?? ''}`} open={passwordUser !== null} onClose={() => setPasswordUser(null)} width={420}>
        <Form layout="vertical">
          <Form.Item label="新密码">
            <Input.Password value={newPassword} onChange={event => setNewPassword(event.target.value)} />
          </Form.Item>
          <Button type="primary" onClick={() => void resetPassword()}>保存</Button>
        </Form>
      </Drawer>
    </div>
  );
}