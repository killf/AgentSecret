import { App as AntApp, Button, Form, Input } from 'antd';
import { useNavigate } from 'react-router-dom';
import { changePassword } from '../api/auth';
import { setToken } from '../api/client';
import { PageHeader, PanelCard } from '../components/Page';
import { sha1 } from '../utils/sha1';

interface PasswordForm {
  oldPassword: string;
  newPassword: string;
}

export default function PasswordPage() {
  const navigate = useNavigate();
  const { message } = AntApp.useApp();

  async function submit(values: PasswordForm) {
    await changePassword(await sha1(values.oldPassword), await sha1(values.newPassword));
    setToken(null);
    message.success('密码已修改，请重新登录');
    navigate('/login');
  }

  return (
    <div className="page-shell">
      <PageHeader title="修改密码" description="修改后当前登录会话立即失效，需要重新登录。" />
      <PanelCard title="安全设置">
        <Form<PasswordForm> layout="vertical" onFinish={submit} requiredMark={false} style={{ maxWidth: 420 }}>
          <Form.Item label="旧密码" name="oldPassword" rules={[{ required: true, message: '请输入旧密码' }]}>
            <Input.Password autoComplete="current-password" />
          </Form.Item>
          <Form.Item label="新密码" name="newPassword" rules={[{ required: true, message: '请输入新密码' }, { min: 8, message: '至少 8 位' }]}>
            <Input.Password autoComplete="new-password" />
          </Form.Item>
          <Button type="primary" htmlType="submit">提交</Button>
        </Form>
      </PanelCard>
    </div>
  );
}