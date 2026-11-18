import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { BrowserRouter } from 'react-router-dom';
import { vi, expect, test } from 'vitest';
import LoginPage from './LoginPage';

vi.mock('../api/auth', () => ({
  login: vi.fn(async () => ({
    token: 'jwt-token',
    expires_at: '2026-11-24T00:00:00Z',
    user: { id: 1, email: 'admin@example.com', name: 'Admin', is_admin: true },
  })),
}));

test('login submits SHA-1 password', async () => {
  render(<BrowserRouter><LoginPage /></BrowserRouter>);
  await userEvent.type(screen.getByLabelText(/邮箱/), 'admin@example.com');
  await userEvent.type(screen.getByLabelText(/密码/), 'password');
  await userEvent.click(screen.getByRole('button', { name: /登\s*录/ }));
  expect(await vi.waitFor(() => localStorage.getItem('agentsecret-token'))).toBe('jwt-token');
});
