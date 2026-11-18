import type { UserInfo } from '../types';

let currentUser: UserInfo | null = null;

export function setUser(user: UserInfo | null): void {
  currentUser = user;
  if (user) localStorage.setItem('agentsecret-user', JSON.stringify(user));
  else localStorage.removeItem('agentsecret-user');
}

export function getUser(): UserInfo | null {
  if (!currentUser) {
    const raw = localStorage.getItem('agentsecret-user');
    currentUser = raw ? (JSON.parse(raw) as UserInfo) : null;
  }

  return currentUser;
}
