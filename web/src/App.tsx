import type { ReactElement } from 'react';
import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom';
import Layout from './components/Layout';
import { getToken } from './api/client';
import { getUser } from './stores/auth';
import LoginPage from './pages/LoginPage';
import DashboardPage from './pages/DashboardPage';
import TokensPage from './pages/TokensPage';
import LogsPage from './pages/LogsPage';
import SandboxLogsPage from './pages/SandboxLogsPage';
import PasswordPage from './pages/PasswordPage';
import ProjectsPage from './pages/ProjectsPage';
import ModelsPage from './pages/admin/ModelsPage';
import ProvidersPage from './pages/admin/ProvidersPage';
import UsersPage from './pages/admin/UsersPage';
import PermissionsPage from './pages/admin/PermissionsPage';
import SettingsPage from './pages/admin/SettingsPage';

function RequireAuth({ children }: { children: ReactElement }) {
  return getToken() ? children : <Navigate to="/login" replace />;
}

function RequireAdmin({ children }: { children: ReactElement }) {
  return getUser()?.is_admin ? children : <Navigate to="/" replace />;
}

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route element={<RequireAuth><Layout /></RequireAuth>}>
          <Route path="/" element={<DashboardPage />} />
          <Route path="/tokens" element={<TokensPage />} />
          <Route path="/logs" element={<LogsPage />} />
          <Route path="/sandbox-logs" element={<SandboxLogsPage />} />
          <Route path="/password" element={<PasswordPage />} />
          <Route path="/projects" element={<ProjectsPage />} />
          <Route path="/admin/models" element={<RequireAdmin><ModelsPage /></RequireAdmin>} />
          <Route path="/admin/providers" element={<RequireAdmin><ProvidersPage /></RequireAdmin>} />
          <Route path="/admin/users" element={<RequireAdmin><UsersPage /></RequireAdmin>} />
          <Route path="/admin/permissions" element={<RequireAdmin><PermissionsPage /></RequireAdmin>} />
          <Route path="/admin/settings" element={<RequireAdmin><SettingsPage /></RequireAdmin>} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}
