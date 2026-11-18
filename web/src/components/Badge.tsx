import { Tag } from 'antd';
import type { ReactNode } from 'react';

export type BadgeTone = 'success' | 'muted' | 'danger' | 'running';

const toneColor: Record<BadgeTone, string> = {
  success: 'success',
  muted: 'default',
  danger: 'error',
  running: 'processing',
};

export function Badge({ tone, children }: { tone: BadgeTone; children: ReactNode }) {
  return <Tag color={toneColor[tone]}>{children}</Tag>;
}