import { useEffect, useMemo, useState } from 'react';
import { Link, Outlet, useLocation, useNavigate } from 'react-router-dom';
import {
  AppstoreOutlined,
  CloudServerOutlined,
  CodeSandboxOutlined,
  DashboardOutlined,
  FileTextOutlined,
  FolderOutlined,
  KeyOutlined,
  LockOutlined,
  LogoutOutlined,
  MenuOutlined,
  SafetyOutlined,
  SettingOutlined,
  TeamOutlined,
} from '@ant-design/icons';
import { Avatar, Button, Flex, Grid, Layout as AntLayout, Menu, Tag, Typography } from 'antd';
import { logout } from '../api/auth';
import { setToken } from '../api/client';
import { getUser } from '../stores/auth';

const basicLinks = [
  { to: '/', label: '总览', icon: <DashboardOutlined /> },
  { to: '/tokens', label: '访问令牌', icon: <KeyOutlined /> },
  { to: '/logs', label: '请求日志', icon: <FileTextOutlined /> },
  { to: '/sandbox-logs', label: '沙盒日志', icon: <CodeSandboxOutlined /> },
  { to: '/projects', label: '项目与脚本', icon: <FolderOutlined /> },
  { to: '/password', label: '修改密码', icon: <LockOutlined /> },
];

const adminLinks = [
  { to: '/admin/models', label: '模型', icon: <AppstoreOutlined /> },
  { to: '/admin/providers', label: '供应商', icon: <CloudServerOutlined /> },
  { to: '/admin/users', label: '用户', icon: <TeamOutlined /> },
  { to: '/admin/permissions', label: '权限', icon: <SafetyOutlined /> },
  { to: '/admin/settings', label: '设置', icon: <SettingOutlined /> },
];

const allLinks = [...basicLinks, ...adminLinks];

export default function Layout() {
  const navigate = useNavigate();
  const location = useLocation();
  const user = getUser();
  const breakpoints = Grid.useBreakpoint();
  const [menuOpen, setMenuOpen] = useState(false);

  useEffect(() => { setMenuOpen(false); }, [location.pathname]);

  async function handleLogout() {
    try {
      await logout();
    } finally {
      setToken(null);
      localStorage.removeItem('agentsecret-user');
      navigate('/login');
    }
  }

  const items = useMemo(() => [
    {
      key: 'workspace',
      label: '工作台',
      type: 'group' as const,
      children: basicLinks.map(link => ({
        key: link.to,
        icon: link.icon,
        label: <Link to={link.to}>{link.label}</Link>,
      })),
    },
    ...(user?.is_admin ? [{
      key: 'administration',
      label: '管理',
      type: 'group' as const,
      children: adminLinks.map(link => ({
        key: link.to,
        icon: link.icon,
        label: <Link to={link.to}>{link.label}</Link>,
      })),
    }] : []),
  ], [user?.is_admin]);

  const current = allLinks.find(link => link.to === location.pathname);
  const initial = (user?.name ?? user?.email ?? 'U').slice(0, 1).toUpperCase();
  const collapsed = breakpoints.lg ? false : !menuOpen;

  return (
    <AntLayout className="app-layout" hasSider>
      <AntLayout.Sider
        className="app-sider"
        width={228}
        collapsedWidth={0}
        collapsed={collapsed}
        collapsible
        trigger={null}
      >
        <Link className="app-brand" to="/">
          <span className="app-brand-mark">AS</span>
          <span>
            <strong>AgentSecret</strong>
            <small>控制台</small>
          </span>
        </Link>

        <Menu mode="inline" selectedKeys={[location.pathname]} items={items} />

        <div className="sidebar-footer">
          <Tag color="success">运行中</Tag>
          <span>v0.1</span>
        </div>
      </AntLayout.Sider>

      <AntLayout className="workspace-layout">
        <AntLayout.Header className="app-header">
          <Flex align="center" gap={10}>
            {breakpoints.lg ? null : (
              <Button aria-label="打开导航" type="text" icon={<MenuOutlined />} onClick={() => setMenuOpen(value => !value)} />
            )}
            <Typography.Text className="header-title">{current?.label ?? 'AgentSecret'}</Typography.Text>
          </Flex>

          <Flex align="center" gap={10}>
            <Flex align="center" gap={8} className="user-chip">
              <Avatar size={28} style={{ background: '#F0EFFE', color: '#4F46E5', fontWeight: 700 }}>{initial}</Avatar>
              <span>{user?.email ?? '未登录'}</span>
            </Flex>
            <Button icon={<LogoutOutlined />} onClick={handleLogout}>退出</Button>
          </Flex>
        </AntLayout.Header>

        <AntLayout.Content className="app-content">
          <Outlet />
        </AntLayout.Content>
      </AntLayout>
    </AntLayout>
  );
}