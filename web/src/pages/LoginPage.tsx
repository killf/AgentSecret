import { App as AntApp, Button, Card, Form, Input, Typography } from 'antd';
import { SafetyCertificateOutlined, ThunderboltOutlined, LockOutlined } from '@ant-design/icons';
import { useNavigate } from 'react-router-dom';
import { login } from '../api/auth';
import { setToken } from '../api/client';
import { setUser } from '../stores/auth';
import { sha1 } from '../utils/sha1';

interface LoginForm {
  email: string;
  password: string;
}

const highlights = [
  { icon: <SafetyCertificateOutlined />, title: '密钥不进入沙盒', text: '脚本通过审核后的固定凭据动作访问上游。' },
  { icon: <ThunderboltOutlined />, title: '统一模型网关', text: 'OpenAI Responses 兼容入口，自动路由与额度控制。' },
  { icon: <LockOutlined />, title: '完整审计链路', text: '请求、执行、审核和令牌操作均可追踪。' },
];

export default function LoginPage() {
  const navigate = useNavigate();
  const { message } = AntApp.useApp();
  const [form] = Form.useForm<LoginForm>();

  async function handleSubmit(values: LoginForm) {
    try {
      const result = await login(values.email, await sha1(values.password));
      setToken(result.token);
      setUser(result.user);
      navigate('/');
    } catch (cause) {
      message.error(cause instanceof Error ? cause.message : '登录失败');
    }
  }

  return (
    <main className="login-shell">
      <section className="login-hero">
        <div>
          <div className="login-brand">AS</div>
          <Typography.Title level={1}>AgentSecret 控制台</Typography.Title>
          <Typography.Paragraph>
            面向团队密钥、模型调用和沙盒脚本的统一安全执行平台。
          </Typography.Paragraph>
        </div>

        <div className="login-points">
          {highlights.map(highlight => (
            <div className="login-point" key={highlight.title}>
              <span>{highlight.icon}</span>
              <div>
                <strong>{highlight.title}</strong>
                <span>{highlight.text}</span>
              </div>
            </div>
          ))}
        </div>
      </section>

      <section className="login-panel">
        <Card className="login-card" variant="borderless">
          <Typography.Title level={3}>欢迎回来</Typography.Title>
          <Typography.Paragraph type="secondary">登录后进入运行总览。</Typography.Paragraph>

          <Form<LoginForm> form={form} layout="vertical" onFinish={handleSubmit} requiredMark={false}>
            <Form.Item label="邮箱" name="email" rules={[{ required: true, message: '请输入邮箱' }]}>
              <Input placeholder="name@example.com" autoComplete="username" />
            </Form.Item>
            <Form.Item label="密码" name="password" rules={[{ required: true, message: '请输入密码' }]}>
              <Input.Password autoComplete="current-password" />
            </Form.Item>
            <Button type="primary" htmlType="submit" block>登录</Button>
          </Form>
        </Card>
      </section>
    </main>
  );
}