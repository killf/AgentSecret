import { post } from './client';

export interface UserInfo {
  id: number;
  email: string;
  name: string;
  is_admin: boolean;
}

export interface LoginResult {
  token: string;
  expires_at: string;
  user: UserInfo;
}

export function login(email: string, passwordSha1: string) {
  return post<LoginResult>('/api/v1/auth/login', { email, password: passwordSha1 });
}

export function logout() {
  return post<null>('/api/v1/auth/logout');
}

export function me() {
  return post<UserInfo>('/api/v1/auth/me');
}

export function changePassword(oldPasswordSha1: string, newPasswordSha1: string) {
  return post<null>('/api/v1/auth/password', {
    old_password: oldPasswordSha1,
    new_password: newPasswordSha1,
  });
}
